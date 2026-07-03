//! PI-SYS-03 · workspace.db schema 与版本化迁移 · DATA-MIGRATE / BR-SYS-PERSIST-001
//!
//! 承接 A-SYS-I-P-01 §4：持久化控制态（非用户正文）。schema_meta.schema_version 单调递增
//! 为迁移唯一驱动；迁移不破坏既有持久化态；应用重启后控制态可恢复。
//! 完整列/索引调优为实现期 A 类债务（本框架落基线表 + 七张控制态扩展表）。

use rusqlite::Connection;
use std::time::{SystemTime, UNIX_EPOCH};

/// 一步迁移：目标版本号 + DDL（追加式，既有条目不可修改）。
struct Migration {
    version: i64,
    sql: &'static str,
}

/// 迁移序列。每次 schema 变更**追加**一条（version 单调递增），不改既有。
const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        sql: r#"
            CREATE TABLE IF NOT EXISTS schema_meta (
                schema_version INTEGER NOT NULL,
                applied_at     INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS workspaces (
                workspace_id   TEXT PRIMARY KEY,
                root_path      TEXT NOT NULL,
                is_active      INTEGER NOT NULL DEFAULT 0,
                opened_at      INTEGER NOT NULL,
                last_active_at INTEGER
            );
            CREATE TABLE IF NOT EXISTS pending_diffs (
                diff_id       TEXT PRIMARY KEY,
                workspace_id  TEXT NOT NULL REFERENCES workspaces(workspace_id),
                rel_path      TEXT NOT NULL,
                origin_module TEXT NOT NULL,
                status        TEXT NOT NULL,
                payload_ref   TEXT NOT NULL,
                created_at    INTEGER NOT NULL,
                decided_at    INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_pending_diffs_ws_status
                ON pending_diffs(workspace_id, status);
            CREATE TABLE IF NOT EXISTS timeline (
                entry_id     TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL REFERENCES workspaces(workspace_id),
                rel_path     TEXT NOT NULL,
                version_seq  INTEGER NOT NULL,
                entry_kind   TEXT NOT NULL,
                label        TEXT,
                snapshot_ref TEXT NOT NULL,
                created_at   INTEGER NOT NULL,
                UNIQUE(workspace_id, rel_path, version_seq)
            );
            CREATE INDEX IF NOT EXISTS idx_timeline_ws_path
                ON timeline(workspace_id, rel_path);
        "#,
    },
    Migration {
        version: 2,
        sql: r#"
            CREATE TABLE IF NOT EXISTS agent_turns (
                turn_id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL, rel_path TEXT,
                payload_ref TEXT NOT NULL, created_at INTEGER NOT NULL );
            CREATE TABLE IF NOT EXISTS build_runs (
                run_id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL, status TEXT NOT NULL,
                plan_ref TEXT NOT NULL, interrupted_at INTEGER,
                created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL );
            CREATE TABLE IF NOT EXISTS workflow_runs (
                run_id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL, workflow_id TEXT NOT NULL,
                current_step INTEGER NOT NULL, status TEXT NOT NULL,
                created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL );
            CREATE TABLE IF NOT EXISTS templates (
                template_id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL, template_kind TEXT NOT NULL,
                body TEXT NOT NULL, organization TEXT,
                created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL );
            CREATE TABLE IF NOT EXISTS workflows (
                workflow_id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL, definition_ref TEXT NOT NULL,
                created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL );
            CREATE TABLE IF NOT EXISTS memory_entries (
                entry_id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL, memory_kind TEXT NOT NULL,
                content TEXT NOT NULL, origin_turn TEXT NOT NULL, status TEXT NOT NULL,
                created_at INTEGER NOT NULL, revoked_at INTEGER );
            CREATE TABLE IF NOT EXISTS knowledge_entries (
                entry_id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL, content TEXT NOT NULL,
                organization TEXT, source_ref TEXT NOT NULL,
                created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL );
        "#,
    },
    Migration {
        version: 3,
        // DiffPayload（TERM-DE-004）持久化：accepted 落盘的内容来源，经 PendingDiff.payload_ref 关联
        // （PI-BE-02 解除 A-DE-I-P-01 §7 存储形态延期项）。after_content 为提案后内容字节。
        sql: r#"
            CREATE TABLE IF NOT EXISTS diff_payloads (
                payload_ref     TEXT PRIMARY KEY,
                workspace_id    TEXT NOT NULL,
                rel_path        TEXT NOT NULL,
                after_content   BLOB NOT NULL,
                before_snapshot TEXT,
                is_new_doc      INTEGER NOT NULL,
                created_at      INTEGER NOT NULL
            );
        "#,
    },
    Migration {
        version: 4,
        // 时间轴快照内容存储（解除 WS-03 快照存储形态债务，PI-BE-05）：全量内容、内容寻址哈希
        // （snapshot_ref=blob_{hash}）、去重；timeline.snapshot_ref 指向之，供 preview/restore 取回。
        sql: r#"
            CREATE TABLE IF NOT EXISTS timeline_snapshots (
                snapshot_ref TEXT PRIMARY KEY,
                content      BLOB NOT NULL,
                created_at   INTEGER NOT NULL
            );
        "#,
    },
    Migration {
        version: 5,
        // 样式操作载荷（PI-ED-11 后端件，A-DE-M-T-01 v1.1 / 裁定 D-3 方案A）：回合产出的样式修改
        // （StyleOp 序列 JSON，形状归 ED A-ED-M-T-01 §5.1）随 DiffPayload 持久化，可空（纯文本 diff 无）。
        sql: r#"
            ALTER TABLE diff_payloads ADD COLUMN style_ops TEXT;
        "#,
    },
];

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 当前 schema 版本：schema_meta 不存在或为空记为 0。
fn current_version(conn: &Connection) -> i64 {
    conn.query_row(
        "SELECT MAX(schema_version) FROM schema_meta",
        [],
        |r| r.get::<_, Option<i64>>(0),
    )
    .map(|v| v.unwrap_or(0))
    .unwrap_or(0)
}

/**
 * @GOV
 * codes:    BR-SYS-PERSIST-001-EFFECT-SYS-DATA-MIGRATE-001
 * type:     EFFECT
 * chain:    DATA-MIGRATE
 * rules:    BR-SYS-PERSIST-001
 * boundary: in=workspaceDbConnection | out=schemaVersionAdvanced(persistedStatePreserved)
 */
/// 版本化迁移（BR-SYS-PERSIST-001）：以 schema_meta.schema_version 为唯一驱动，
/// 仅应用 version > 当前版本的迁移；每步在单事务内执行 DDL 并写入新版本，迁移不破坏既有
/// pending_diffs / timeline 持久化态（向前迁移不丢数据）。重复调用幂等。返回应用后的版本。
pub fn migrate(conn: &mut Connection) -> rusqlite::Result<i64> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    let from = current_version(conn);
    for m in MIGRATIONS.iter().filter(|m| m.version > from) {
        let tx = conn.transaction()?;
        tx.execute_batch(m.sql)?;
        tx.execute(
            "INSERT INTO schema_meta (schema_version, applied_at) VALUES (?1, ?2)",
            (m.version, now_millis()),
        )?;
        tx.commit()?;
    }
    Ok(current_version(conn))
}

/// 打开（或创建）workspace.db 并迁移到最新版本——重启恢复入口。
pub fn open_and_migrate(path: &std::path::Path) -> rusqlite::Result<Connection> {
    let mut conn = Connection::open(path)?;
    migrate(&mut conn)?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn latest() -> i64 {
        MIGRATIONS.iter().map(|m| m.version).max().unwrap()
    }

    // covers: BR-SYS-PERSIST-001
    // schema_version 单调递增驱动迁移；迁移幂等（重复调用版本不变、不重复应用）。
    #[test]
    fn migration_is_monotonic_and_idempotent() {
        let mut conn = Connection::open_in_memory().unwrap();
        assert_eq!(current_version(&conn), 0);
        let v1 = migrate(&mut conn).unwrap();
        assert_eq!(v1, latest());
        let v2 = migrate(&mut conn).unwrap();
        assert_eq!(v2, latest());
        let rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_meta", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, latest()); // 每个 version 恰一条
    }

    // covers: BR-SYS-PERSIST-001
    // 重启可恢复：写入未裁决 pending_diff + 时间轴 → 关闭 → 重新打开迁移 → 控制态完整无丢失。
    #[test]
    fn control_state_survives_restart() {
        let mut path = std::env::temp_dir();
        path.push(format!("binder_persist_test_{}.db", now_millis()));

        {
            let conn = open_and_migrate(&path).unwrap();
            conn.execute(
                "INSERT INTO workspaces (workspace_id, root_path, is_active, opened_at) \
                 VALUES ('ws1', '/tmp/ws', 1, ?1)",
                (now_millis(),),
            )
            .unwrap();
            conn.execute(
                "INSERT INTO pending_diffs \
                 (diff_id, workspace_id, rel_path, origin_module, status, payload_ref, created_at) \
                 VALUES ('d1','ws1','a.md','DE','pending','p1',?1)",
                (now_millis(),),
            )
            .unwrap();
            conn.execute(
                "INSERT INTO timeline \
                 (entry_id, workspace_id, rel_path, version_seq, entry_kind, snapshot_ref, created_at) \
                 VALUES ('e1','ws1','a.md',1,'apply','snap1',?1)",
                (now_millis(),),
            )
            .unwrap();
        } // 关闭连接（模拟应用退出）

        let conn = open_and_migrate(&path).unwrap();
        let diff_status: String = conn
            .query_row(
                "SELECT status FROM pending_diffs WHERE diff_id='d1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(diff_status, "pending");
        let seq: i64 = conn
            .query_row(
                "SELECT version_seq FROM timeline WHERE entry_id='e1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(seq, 1);

        drop(conn);
        let _ = std::fs::remove_file(&path);
    }
}
