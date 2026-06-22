//! PI-TPL-01 · 模板管理与原子实例化 · TPL-FLOW / BR-TPL-DATA-001
//!
//! 承接 A-TPL-M-T-01：模板 workspace 级隔离（templates 表，PI-SYS-03；跨 workspace 不可见）；
//! 文档模板**原子实例化**为新文档——渲染失败或写盘失败**无半成品**（复用 WS save_atomic 的临时文件 +
//! rename 原子语义，失败原位置不残留部分内容）。变量插值为 `{{key}}` 占位替换；模板组织层级与
//! 工作流模板复合为实现期细化（TPL-01，A 类）。

use crate::ipc::IpcError;
use crate::workspace::save;
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

static SEQ: AtomicU64 = AtomicU64::new(0);

fn gen_template_id() -> String {
    format!("tpl_{}_{}", now_millis(), SEQ.fetch_add(1, Ordering::Relaxed))
}

/// `{{key}}` 占位替换（未提供的占位保持原样，由调用方校验完整性）。
fn render(body: &str, vars: &HashMap<String, String>) -> String {
    let mut out = body.to_string();
    for (k, v) in vars {
        out = out.replace(&format!("{{{{{k}}}}}"), v);
    }
    out
}

/**
 * @GOV
 * codes:    BR-TPL-DATA-001-EFFECT-TPL-TPL-FLOW-001
 * type:     EFFECT
 * chain:    TPL-FLOW
 * rules:    BR-TPL-DATA-001
 * boundary: in=TemplateDefinition | out=persistedTemplate(workspaceScoped)
 */
/// 新建模板（BR-TPL-DATA-001）：落 templates 表于 workspace 控制态（隔离）。返回 template_id。
pub fn create_template(
    conn: &Connection,
    workspace_id: &str,
    template_kind: &str,
    body: &str,
    organization: Option<&str>,
) -> Result<String, IpcError> {
    let id = gen_template_id();
    let now = now_millis();
    conn.execute(
        "INSERT INTO templates \
         (template_id, workspace_id, template_kind, body, organization, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        (&id, workspace_id, template_kind, body, organization, now),
    )
    .map_err(|e| IpcError::new("TPL_DB_ERROR", &format!("模板落库失败: {e}"), true, false))?;
    Ok(id)
}

/**
 * @GOV
 * codes:    BR-TPL-DATA-001-QUERY-TPL-TPL-FLOW-001
 * type:     QUERY
 * chain:    TPL-FLOW
 * rules:    BR-TPL-DATA-001
 * boundary: in=workspaceId | out=workspaceTemplates(scopedNeverCrossWorkspace)
 */
/// 列举当前 workspace 模板（BR-TPL-DATA-001）：恒以 workspace_id 作用域，跨 workspace 不可见。
pub fn list_templates(
    conn: &Connection,
    workspace_id: &str,
) -> rusqlite::Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT template_id, template_kind FROM templates WHERE workspace_id=?1 ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map([workspace_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
    rows.collect()
}

/**
 * @GOV
 * codes:    BR-TPL-DATA-001-EFFECT-TPL-TPL-FLOW-002
 * type:     EFFECT
 * chain:    TPL-FLOW
 * rules:    BR-TPL-DATA-001
 * boundary: in=scopedTemplateAndVars | out=newDocumentAtomic(noHalfProductOnFailure)
 */
/// 原子实例化（BR-TPL-DATA-001）：以 (template_id, workspace_id) 作用域加载模板（跨 workspace 拒绝=
/// 隔离），渲染 `{{key}}` 后经 WS `save_atomic`（临时文件 + rename）**原子**写入新文档；模板缺失或写盘
/// 失败 → Err 且**无半成品**（原子写不留部分内容）。abs_path 须由调用方经 WS 路径守护解析。
pub fn instantiate_to_path(
    conn: &Connection,
    workspace_id: &str,
    template_id: &str,
    vars: &HashMap<String, String>,
    abs_path: &Path,
) -> Result<(), IpcError> {
    let body: String = conn
        .query_row(
            "SELECT body FROM templates WHERE template_id=?1 AND workspace_id=?2",
            (template_id, workspace_id),
            |r| r.get(0),
        )
        .map_err(|_| {
            IpcError::new(
                "TPL_NOT_FOUND",
                "模板不存在或不属当前 workspace（跨 workspace 隔离）",
                false,
                false,
            )
        })?;
    let rendered = render(&body, vars);
    // 原子写入：失败（如父目录不存在）经 save_atomic 返回 Err，目标不留半成品。
    save::save_atomic(abs_path, rendered.as_bytes(), None, true)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn fresh() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        db::migrate(&mut conn).unwrap();
        conn
    }

    fn vars(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    // covers: BR-TPL-DATA-001
    // 模板 workspace 隔离：列举/实例化恒以 workspace_id 作用域，ws2 不可用 ws1 的模板。
    #[test]
    fn templates_are_workspace_scoped() {
        let conn = fresh();
        let t1 = create_template(&conn, "ws1", "doc", "# {{title}}", None).unwrap();
        create_template(&conn, "ws2", "doc", "other", None).unwrap();
        assert_eq!(list_templates(&conn, "ws1").unwrap().len(), 1);
        assert_eq!(list_templates(&conn, "ws2").unwrap().len(), 1);

        // 跨 workspace 实例化被拒（隔离）：ws2 用 ws1 的模板 id
        let mut path = std::env::temp_dir();
        path.push(format!("binder_tpl_iso_{}.md", now_millis()));
        let err = instantiate_to_path(&conn, "ws2", &t1, &vars(&[]), &path).unwrap_err();
        assert_eq!(err.code, "TPL_NOT_FOUND");
        assert!(!path.exists(), "跨 workspace 拒绝不得产出文件");
    }

    // covers: BR-TPL-DATA-001
    // 原子实例化：渲染占位并原子写入新文档；成功后内容完整。
    #[test]
    fn instantiation_renders_and_writes_atomically() {
        let conn = fresh();
        let t = create_template(&conn, "ws1", "doc", "# {{title}}\n作者：{{author}}", None).unwrap();
        let mut path = std::env::temp_dir();
        path.push(format!("binder_tpl_ok_{}.md", now_millis()));
        let _ = std::fs::remove_file(&path);

        instantiate_to_path(&conn, "ws1", &t, &vars(&[("title", "周报"), ("author", "Z")]), &path)
            .unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "# 周报\n作者：Z");
        let _ = std::fs::remove_file(&path);
    }

    // covers: BR-TPL-DATA-001
    // 失败无半成品：模板缺失 → Err 且不产出文件；写盘失败（父目录不存在）→ Err 且无残留。
    #[test]
    fn failure_leaves_no_half_product() {
        let conn = fresh();
        // 模板不存在
        let mut p1 = std::env::temp_dir();
        p1.push(format!("binder_tpl_missing_{}.md", now_millis()));
        assert!(instantiate_to_path(&conn, "ws1", "nope", &vars(&[]), &p1).is_err());
        assert!(!p1.exists());

        // 写盘失败：目标父目录不存在 → save_atomic Err，无半成品
        let t = create_template(&conn, "ws1", "doc", "x", None).unwrap();
        let bad = Path::new("/nonexistent_dir_binder/sub/out.md");
        assert!(instantiate_to_path(&conn, "ws1", &t, &vars(&[]), bad).is_err());
        assert!(!bad.exists());
    }
}
