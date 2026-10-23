//! 从应用组合根迁入的模块运行时/路由；保持原 command 与状态机行为等价。

use crate::*;

fn now_millis() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

pub(crate) fn route_persist_session_snapshot(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let snapshot_value = request.params.get("snapshot").cloned().ok_or_else(|| {
        IpcError::new(
            "ED_SESSION_SNAPSHOT_REQUIRED",
            "缺少 snapshot 参数",
            false,
            false,
        )
    })?;
    let snapshot_baseline = match snapshot_value.get("baseline") {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::Number(value)) => Some(value.as_u64().ok_or_else(|| {
            IpcError::new(
                "ED_SESSION_SNAPSHOT_INVALID",
                "BDocx snapshot baseline 必须是 u64",
                false,
                false,
            )
        })?),
        Some(serde_json::Value::String(value)) => Some(value.parse::<u64>().map_err(|_| {
            IpcError::new(
                "ED_SESSION_SNAPSHOT_INVALID",
                "BDocx snapshot baseline 必须是 u64 字符串",
                false,
                false,
            )
        })?),
        Some(_) => {
            return Err(IpcError::new(
                "ED_SESSION_SNAPSHOT_INVALID",
                "BDocx snapshot baseline 类型无效",
                false,
                false,
            ))
        }
    };
    let snapshot: ag::blocks::OpenDocSnapshot =
        serde_json::from_value(snapshot_value).map_err(|e| {
            IpcError::new(
                "ED_SESSION_SNAPSHOT_INVALID",
                &format!("OpenDocSnapshot 解析失败: {e}"),
                false,
                false,
            )
        })?;
    if snapshot.content_hash.trim().is_empty() {
        return Err(IpcError::new(
            "ED_SESSION_SNAPSHOT_INVALID",
            "OpenDocSnapshot 缺少 content_hash",
            false,
            false,
        ));
    }
    workspace::resolve_in_workspace(std::path::Path::new(&ws.root_path), &snapshot.rel_path)?;
    let session_version = i64::try_from(snapshot.session_version).map_err(|_| {
        IpcError::new(
            "ED_SESSION_SNAPSHOT_INVALID",
            "session_version 超出持久化范围",
            false,
            false,
        )
    })?;
    // 所有编辑格式均须保留加载基准；否则恢复后的既有文本文件会被误判为新建。
    // 复用内部 envelope，读取时解包，不让存储元数据进入 LogicalState。
    let persisted_content =
        doc::image_media::encode_snapshot_content(&snapshot.content, snapshot_baseline);
    let conn = lock_db(state)?;
    conn.execute(
        "INSERT OR REPLACE INTO ed_session_snapshots \
         (workspace_id, rel_path, content, edit_format, content_format, dirty, session_version, content_hash, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            ws.workspace_id,
            snapshot.rel_path,
            persisted_content,
            snapshot.edit_format,
            snapshot.content_format,
            if snapshot.dirty { 1 } else { 0 },
            session_version,
            snapshot.content_hash,
            now_millis(),
        ],
    )
    .map_err(|e| IpcError::new("ED_SESSION_DB_ERROR", &e.to_string(), true, false))?;
    Ok(serde_json::json!({ "ok": true }))
}

pub(crate) fn route_list_session_snapshots(
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let conn = lock_db(state)?;
    let mut stmt = conn
        .prepare(
            "SELECT rel_path, content, edit_format, content_format, dirty, session_version, content_hash, updated_at \
             FROM ed_session_snapshots WHERE workspace_id = ?1 ORDER BY updated_at ASC",
        )
        .map_err(|e| IpcError::new("ED_SESSION_DB_ERROR", &e.to_string(), true, false))?;
    let rows = stmt
        .query_map([ws.workspace_id], |r| {
            let stored_content: String = r.get(1)?;
            let (content, baseline) = doc::image_media::decode_snapshot_content(&stored_content);
            let dirty: i64 = r.get(4)?;
            let session_version: i64 = r.get(5)?;
            let updated_at: i64 = r.get(7)?;
            Ok(serde_json::json!({
                "relPath": r.get::<_, String>(0)?,
                "content": content,
                "baseline": baseline.map(|value| value.to_string()),
                "editFormat": r.get::<_, String>(2)?,
                "contentFormat": r.get::<_, String>(3)?,
                "source": "open_editor_session",
                "dirty": dirty != 0,
                "sessionVersion": session_version.max(0),
                "contentHash": r.get::<_, String>(6)?,
                "capturedAt": updated_at,
            }))
        })
        .map_err(|e| IpcError::new("ED_SESSION_DB_ERROR", &e.to_string(), true, false))?;
    let snapshots = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| IpcError::new("ED_SESSION_DB_ERROR", &e.to_string(), true, false))?;
    Ok(serde_json::json!({ "snapshots": snapshots }))
}

pub(crate) fn route_persist_open_tabs(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let tabs = request
        .params
        .get("tabs")
        .and_then(|v| v.as_array())
        .ok_or_else(|| IpcError::new("ED_OPEN_TABS_REQUIRED", "tabs 必须是数组", false, false))?;
    let mut rows: Vec<(String, String, i64, bool)> = Vec::with_capacity(tabs.len());
    let mut active_count = 0usize;
    for (idx, tab) in tabs.iter().enumerate() {
        let rel_path = tab
            .get("relPath")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| {
                IpcError::new("ED_OPEN_TAB_INVALID", "打开标签缺少 relPath", false, false)
            })?
            .to_string();
        workspace::resolve_in_workspace(std::path::Path::new(&ws.root_path), &rel_path)?;
        let kind = tab
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or("doc")
            .to_string();
        if kind != "doc" {
            return Err(IpcError::new(
                "ED_OPEN_TAB_INVALID",
                "当前仅支持持久化 doc 类型编辑标签",
                false,
                false,
            ));
        }
        let open_index = tab
            .get("openIndex")
            .and_then(|v| v.as_i64())
            .unwrap_or(idx as i64);
        if open_index < 0 {
            return Err(IpcError::new(
                "ED_OPEN_TAB_INVALID",
                "openIndex 不能为负数",
                false,
                false,
            ));
        }
        let is_active = tab
            .get("isActive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if is_active {
            active_count += 1;
        }
        rows.push((rel_path, kind, open_index, is_active));
    }
    if rows.len() > 1 && active_count != 1 {
        return Err(IpcError::new(
            "ED_OPEN_TABS_ACTIVE_INVALID",
            "多个打开文档必须且只能有一个当前文档",
            false,
            false,
        ));
    }
    if rows.len() == 1 && active_count == 0 {
        rows[0].3 = true;
    }

    let mut conn = lock_db(state)?;
    let tx = conn
        .transaction()
        .map_err(|e| IpcError::new("ED_OPEN_TABS_DB_ERROR", &e.to_string(), true, false))?;
    tx.execute(
        "DELETE FROM ed_open_tabs WHERE workspace_id = ?1",
        [ws.workspace_id.as_str()],
    )
    .map_err(|e| IpcError::new("ED_OPEN_TABS_DB_ERROR", &e.to_string(), true, false))?;
    let updated_at = now_millis();
    for (rel_path, kind, open_index, is_active) in rows {
        tx.execute(
            "INSERT INTO ed_open_tabs \
             (workspace_id, rel_path, kind, open_index, is_active, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                ws.workspace_id.as_str(),
                rel_path,
                kind,
                open_index,
                if is_active { 1 } else { 0 },
                updated_at,
            ],
        )
        .map_err(|e| IpcError::new("ED_OPEN_TABS_DB_ERROR", &e.to_string(), true, false))?;
    }
    tx.commit()
        .map_err(|e| IpcError::new("ED_OPEN_TABS_DB_ERROR", &e.to_string(), true, false))?;
    Ok(serde_json::json!({ "ok": true }))
}

pub(crate) fn route_list_open_tabs(state: &AppState) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let conn = lock_db(state)?;
    let mut stmt = conn
        .prepare(
            "SELECT rel_path, kind, open_index, is_active, updated_at \
             FROM ed_open_tabs WHERE workspace_id = ?1 ORDER BY open_index ASC",
        )
        .map_err(|e| IpcError::new("ED_OPEN_TABS_DB_ERROR", &e.to_string(), true, false))?;
    let rows = stmt
        .query_map([ws.workspace_id], |r| {
            let is_active: i64 = r.get(3)?;
            Ok(serde_json::json!({
                "relPath": r.get::<_, String>(0)?,
                "kind": r.get::<_, String>(1)?,
                "openIndex": r.get::<_, i64>(2)?,
                "isActive": is_active != 0,
                "updatedAt": r.get::<_, i64>(4)?,
            }))
        })
        .map_err(|e| IpcError::new("ED_OPEN_TABS_DB_ERROR", &e.to_string(), true, false))?;
    let tabs = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| IpcError::new("ED_OPEN_TABS_DB_ERROR", &e.to_string(), true, false))?;
    Ok(serde_json::json!({ "tabs": tabs }))
}
