//! PI-WS-05 · workspace 注册与生命周期持久化 · DATA-MIGRATE / BR-WS-PERSIST-002
//!
//! 承接 A-WS-M-T-01 §7：workspace 控制态持久化于 workspace.db（承 BR-SYS-PERSIST-001 迁移）。
//! 不变量：同一时刻至多一个 is_active；应用重启后活动 workspace 可恢复（active_workspace）。
//! workspace 级隔离与多 workspace 缓存策略为实现期债务（WS-04）；ID 生成算法实现期选型（A 类）。

use crate::types::WorkspaceRef;
use rusqlite::{Connection, OptionalExtension};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 不透明 workspace ID：`ws_` + UUIDv4（G-4 定档，A-CORE-X-R-03 §7；全局唯一、跨进程无碰撞）。
fn gen_workspace_id() -> String {
    format!("ws_{}", uuid::Uuid::new_v4())
}

/// 置某 workspace 为唯一活动态（清除其余 is_active）——单活动不变量的执行点。
fn set_unique_active(tx: &rusqlite::Transaction, workspace_id: &str) -> rusqlite::Result<()> {
    tx.execute("UPDATE workspaces SET is_active = 0", [])?;
    tx.execute(
        "UPDATE workspaces SET is_active = 1, last_active_at = ?2 WHERE workspace_id = ?1",
        (workspace_id, now_millis()),
    )?;
    Ok(())
}

/**
 * @GOV
 * codes:    BR-WS-PERSIST-002-EFFECT-WS-DATA-MIGRATE-001
 * type:     EFFECT
 * chain:    DATA-MIGRATE
 * rules:    BR-WS-PERSIST-002
 * boundary: in=rootPath | out=activeWorkspaceRef(persistedSingleActive)
 */
/// 打开 workspace（注册或复用 root_path 已有项）并置为唯一活动态（BR-WS-PERSIST-002）。
pub fn open_workspace(conn: &mut Connection, root_path: &str) -> rusqlite::Result<WorkspaceRef> {
    let existing: Option<String> = conn
        .query_row(
            "SELECT workspace_id FROM workspaces WHERE root_path = ?1",
            [root_path],
            |r| r.get(0),
        )
        .optional()?;
    let id = existing.clone().unwrap_or_else(gen_workspace_id);

    let tx = conn.transaction()?;
    if existing.is_none() {
        tx.execute(
            "INSERT INTO workspaces (workspace_id, root_path, is_active, opened_at) \
             VALUES (?1, ?2, 0, ?3)",
            (&id, root_path, now_millis()),
        )?;
    }
    set_unique_active(&tx, &id)?;
    tx.commit()?;

    Ok(WorkspaceRef {
        workspace_id: id,
        root_path: root_path.to_string(),
        is_active: true,
    })
}

/**
 * @GOV
 * codes:    BR-WS-PERSIST-002-EFFECT-WS-DATA-MIGRATE-002
 * type:     EFFECT
 * chain:    DATA-MIGRATE
 * rules:    BR-WS-PERSIST-002
 * boundary: in=workspaceId | out=activeWorkspaceRef(enforcerSingleActive)
 */
/// 切换活动 workspace（BR-WS-PERSIST-002：维持单活动不变量）。
pub fn switch_workspace(
    conn: &mut Connection,
    workspace_id: &str,
) -> rusqlite::Result<WorkspaceRef> {
    let root: String = conn.query_row(
        "SELECT root_path FROM workspaces WHERE workspace_id = ?1",
        [workspace_id],
        |r| r.get(0),
    )?;
    let tx = conn.transaction()?;
    set_unique_active(&tx, workspace_id)?;
    tx.commit()?;
    Ok(WorkspaceRef {
        workspace_id: workspace_id.to_string(),
        root_path: root,
        is_active: true,
    })
}

/**
 * @GOV
 * codes:    BR-WS-PERSIST-002-EFFECT-WS-DATA-MIGRATE-003
 * type:     EFFECT
 * chain:    DATA-MIGRATE
 * rules:    BR-WS-PERSIST-002
 * boundary: in=workspaceId | out=deactivated(controlStateRetained)
 */
/// 关闭活动 workspace：解除活动态但保留控制态（BR-WS-PERSIST-002：关闭不丢控制态）。
pub fn close_workspace(conn: &Connection, workspace_id: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE workspaces SET is_active = 0 WHERE workspace_id = ?1",
        [workspace_id],
    )?;
    Ok(())
}

/**
 * @GOV
 * codes:    BR-WS-PERSIST-002-QUERY-WS-DATA-MIGRATE-001
 * type:     QUERY
 * chain:    DATA-MIGRATE
 * rules:    BR-WS-PERSIST-002
 * boundary: in=workspaceDb | out=activeWorkspaceRef(restartRecovery)
 */
/// 重启恢复入口：读取持久化的活动 workspace（BR-WS-PERSIST-002：重启可恢复）。
pub fn active_workspace(conn: &Connection) -> rusqlite::Result<Option<WorkspaceRef>> {
    conn.query_row(
        "SELECT workspace_id, root_path FROM workspaces WHERE is_active = 1 LIMIT 1",
        [],
        |r| {
            Ok(WorkspaceRef {
                workspace_id: r.get(0)?,
                root_path: r.get(1)?,
                is_active: true,
            })
        },
    )
    .optional()
}

/**
 * @GOV
 * codes:    BR-WS-PERSIST-002-QUERY-WS-DATA-MIGRATE-002
 * type:     QUERY
 * chain:    DATA-MIGRATE
 * rules:    BR-WS-PERSIST-002
 * boundary: in=workspaceDb | out=workspaceRefList(registry)
 */
/// 列出已注册 workspace（BR-WS-PERSIST-002：控制态可见可恢复）。
pub fn list_workspaces(conn: &Connection) -> rusqlite::Result<Vec<WorkspaceRef>> {
    let mut stmt = conn
        .prepare("SELECT workspace_id, root_path, is_active FROM workspaces ORDER BY opened_at")?;
    let rows = stmt.query_map([], |r| {
        Ok(WorkspaceRef {
            workspace_id: r.get(0)?,
            root_path: r.get(1)?,
            is_active: r.get::<_, i64>(2)? != 0,
        })
    })?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn fresh_db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        db::migrate(&mut conn).unwrap();
        conn
    }

    // covers: BR-WS-PERSIST-002
    // 打开多个 workspace 维持单活动不变量；切换/关闭后活动态正确。
    #[test]
    fn single_active_invariant() {
        let mut conn = fresh_db();
        let a = open_workspace(&mut conn, "/ws/a").unwrap();
        assert!(a.is_active);
        let b = open_workspace(&mut conn, "/ws/b").unwrap();
        // 打开 B 后仅 B 活动
        let active = active_workspace(&conn).unwrap().unwrap();
        assert_eq!(active.workspace_id, b.workspace_id);
        let actives: usize = list_workspaces(&conn)
            .unwrap()
            .iter()
            .filter(|w| w.is_active)
            .count();
        assert_eq!(actives, 1);

        // 切回 A
        switch_workspace(&mut conn, &a.workspace_id).unwrap();
        assert_eq!(
            active_workspace(&conn).unwrap().unwrap().workspace_id,
            a.workspace_id
        );

        // 关闭活动 → 无活动，但两个 workspace 仍在册（控制态保留）
        close_workspace(&conn, &a.workspace_id).unwrap();
        assert!(active_workspace(&conn).unwrap().is_none());
        assert_eq!(list_workspaces(&conn).unwrap().len(), 2);
    }

    // covers: BR-WS-PERSIST-002
    // 重启可恢复：打开 workspace → 关闭连接 → 重开 db → 活动 workspace 完整恢复。
    #[test]
    fn active_workspace_survives_restart() {
        let mut path = std::env::temp_dir();
        path.push(format!("binder_ws_lifecycle_{}.db", now_millis()));

        let opened_id;
        {
            let mut conn = db::open_and_migrate(&path).unwrap();
            let w = open_workspace(&mut conn, "/ws/main").unwrap();
            opened_id = w.workspace_id;
        } // 关闭（模拟退出）

        let conn = db::open_and_migrate(&path).unwrap();
        let recovered = active_workspace(&conn).unwrap().unwrap();
        assert_eq!(recovered.workspace_id, opened_id);
        assert_eq!(recovered.root_path, "/ws/main");

        drop(conn);
        let _ = std::fs::remove_file(&path);
    }
}
