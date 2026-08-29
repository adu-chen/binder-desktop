//! PI-SYS-03 · workspace.db schema 与版本化迁移 · DATA-MIGRATE / BR-SYS-PERSIST-001
//!
//! 承接 A-SYS-I-P-01 §4：持久化控制态（非用户正文）。schema_meta.schema_version 单调递增
//! 为迁移唯一驱动；迁移不破坏既有持久化态；应用重启后控制态可恢复。
//! 完整列/索引调优为实现期 A 类债务（本框架落基线表 + 八张控制态扩展表）。

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
    Migration {
        version: 6,
        // AG 历史持久化（PI-AG-12）：代理编辑会话标签元数据、全状态回合、payload JSON。
        // 兼容 v2 已存在的薄 agent_turns 表，按追加列迁移；旧记录 session_id 可空，不参与新恢复查询。
        sql: r#"
            CREATE TABLE IF NOT EXISTS agent_sessions (
                session_id     TEXT PRIMARY KEY,
                workspace_id   TEXT NOT NULL,
                title          TEXT NOT NULL,
                mode           TEXT NOT NULL,
                archived       INTEGER NOT NULL,
                open_index     INTEGER NOT NULL,
                is_active      INTEGER NOT NULL,
                created_at     INTEGER NOT NULL,
                updated_at     INTEGER NOT NULL,
                last_active_at INTEGER
            );
            ALTER TABLE agent_turns ADD COLUMN session_id TEXT;
            ALTER TABLE agent_turns ADD COLUMN status TEXT NOT NULL DEFAULT 'completed';
            ALTER TABLE agent_turns ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
            UPDATE agent_turns SET updated_at = created_at WHERE updated_at = 0;
            CREATE TABLE IF NOT EXISTS agent_turn_payloads (
                payload_ref  TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                payload_size INTEGER NOT NULL,
                created_at   INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_agent_sessions_ws
                ON agent_sessions(workspace_id, archived, open_index);
            CREATE INDEX IF NOT EXISTS idx_agent_turns_session
                ON agent_turns(workspace_id, session_id, created_at);
            CREATE INDEX IF NOT EXISTS idx_agent_turn_payloads_ws
                ON agent_turn_payloads(workspace_id);
        "#,
    },
    Migration {
        version: 7,
        // 打开文档 LogicalState 持久镜像：只保存编辑会话内容与基线元数据，不能视为用户文档落盘。
        sql: r#"
            CREATE TABLE IF NOT EXISTS ed_session_snapshots (
                workspace_id     TEXT NOT NULL,
                rel_path         TEXT NOT NULL,
                content          TEXT NOT NULL,
                edit_format      TEXT NOT NULL,
                dirty            INTEGER NOT NULL,
                session_version  INTEGER NOT NULL,
                content_hash     TEXT NOT NULL,
                updated_at       INTEGER NOT NULL,
                PRIMARY KEY(workspace_id, rel_path)
            );
            CREATE INDEX IF NOT EXISTS idx_ed_session_snapshots_ws
                ON ed_session_snapshots(workspace_id, updated_at);
        "#,
    },
    Migration {
        version: 8,
        // 编辑器打开标签持久态：只保存“哪些文档打开、顺序、当前文档”，不保存正文内容。
        sql: r#"
            CREATE TABLE IF NOT EXISTS ed_open_tabs (
                workspace_id TEXT NOT NULL,
                rel_path     TEXT NOT NULL,
                kind         TEXT NOT NULL,
                open_index   INTEGER NOT NULL,
                is_active    INTEGER NOT NULL,
                updated_at   INTEGER NOT NULL,
                PRIMARY KEY(workspace_id, rel_path)
            );
            CREATE UNIQUE INDEX IF NOT EXISTS idx_ed_open_tabs_ws_index
                ON ed_open_tabs(workspace_id, open_index);
            CREATE UNIQUE INDEX IF NOT EXISTS idx_ed_open_tabs_ws_active
                ON ed_open_tabs(workspace_id) WHERE is_active = 1;
            CREATE INDEX IF NOT EXISTS idx_ed_open_tabs_ws_order
                ON ed_open_tabs(workspace_id, open_index);
        "#,
    },
    Migration {
        version: 9,
        // Markdown 终态编辑：会话 LogicalState 的编码与磁盘 edit_format 分离，旧快照默认为 text。
        sql: r#"
            ALTER TABLE ed_session_snapshots
                ADD COLUMN content_format TEXT NOT NULL DEFAULT 'text';
        "#,
    },
    Migration {
        version: 10,
        // 应用关闭生命周期（APP-CLOSE-DIFF-LIFECYCLE-01 / BR-SYS-DATA-005）：diff 效果态与终态原因，
        // 区分"裁决结果"（status）与"内容是否已生效"（effect_status）两个维度；异常退出清算据此
        // 识别 accepted 但未落盘的行。存量数据一次性回填（2026-07-20 人类决策 4）：accepted 行保守
        // 回填为 saved（历史数据无法逐行区分是否仍 awaiting_save）；其余状态回填为 not_applicable。
        sql: r#"
            ALTER TABLE pending_diffs ADD COLUMN effect_status TEXT NOT NULL DEFAULT 'not_applicable';
            ALTER TABLE pending_diffs ADD COLUMN terminal_reason TEXT;
            UPDATE pending_diffs SET effect_status = 'saved' WHERE status = 'accepted';
        "#,
    },
    Migration {
        version: 11,
        // 受限 TaskRun 持久化（AG-1.0-IMP-TASKRUN-01 / BR-SYS-STATE-008）：task_runs 表承载
        // SM-AG-04 生命周期（Planned/Executing/WaitingConfirm/WaitingClarification/Completed/Failed/
        // Cancelled/Interrupted）与步骤态（pending/running/waiting_confirm/ok/failed/cancelled）；
        // agent_turns 加 task_run_id 关联（回合→TaskRun，DiffCard 可经此关联，UNC-09）。
        // 不跨重启续跑：恢复仅为展示 Interrupted，retry 创建新回合（BR-SYS-STATE-008）。
        sql: r#"
            CREATE TABLE IF NOT EXISTS task_runs (
                task_run_id  TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                session_id   TEXT NOT NULL,
                status       TEXT NOT NULL,
                intent_summary TEXT,
                steps_json   TEXT NOT NULL DEFAULT '[]',
                created_at   INTEGER NOT NULL,
                updated_at   INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_task_runs_session
                ON task_runs(workspace_id, session_id, updated_at);
            ALTER TABLE agent_turns ADD COLUMN task_run_id TEXT;
        "#,
    },
    Migration {
        version: 12,
        // TaskRun 编排重构（AG-1.0-TASKRUN-REWORK-IMP-RUNTIME-01 · A-AG-M-T-01 §7.2.1）：
        // 三层分离数据层——task_plans（可展示操作计划，非思维链）、task_plan_steps（步骤/依赖/授权/
        // 确认点）、decision_requests（任务级裁决项 proposed→resolved，confirm/cancel 驱动 TaskRun）。
        // 语义修正：waiting_confirm 仅由 DecisionRequest 产生；新消息不绕过未决确认。
        sql: r#"
            CREATE TABLE IF NOT EXISTS task_plans (
                plan_id     TEXT PRIMARY KEY,
                task_run_id TEXT NOT NULL,
                plan_json   TEXT NOT NULL,
                created_at  INTEGER NOT NULL,
                updated_at  INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_task_plans_run ON task_plans(task_run_id);

            CREATE TABLE IF NOT EXISTS task_plan_steps (
                step_id        TEXT PRIMARY KEY,
                plan_id        TEXT NOT NULL,
                seq            INTEGER NOT NULL,
                kind           TEXT NOT NULL,
                label          TEXT NOT NULL,
                deps_json      TEXT NOT NULL DEFAULT '[]',
                required_grant TEXT,
                confirm_point  INTEGER NOT NULL DEFAULT 0,
                status         TEXT NOT NULL DEFAULT 'pending',
                updated_at     INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_plan_steps_plan ON task_plan_steps(plan_id, seq);

            CREATE TABLE IF NOT EXISTS decision_requests (
                request_id   TEXT PRIMARY KEY,
                task_run_id  TEXT NOT NULL,
                kind         TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                status       TEXT NOT NULL DEFAULT 'proposed',
                outcome      TEXT,
                proposed_at  INTEGER NOT NULL,
                resolved_at  INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_decision_run
                ON decision_requests(task_run_id, status);
        "#,
    },
    Migration {
        version: 13,
        // 连续 Diff 预览回执（AG-1.0-DIFF-PREVIEW-ACK-CHAIN-01）：LogicalState Diff 的模型
        // 片段在卡片生成后停止；编辑器只有在真实受控预览成功后才记录一次性 checkpoint，随后前端
        // 明确启动下一片段。该表只保存控制态/基线标识，不保存用户正文，也不构成后台队列。
        sql: r#"
            CREATE TABLE IF NOT EXISTS task_preview_checkpoints (
                diff_id         TEXT PRIMARY KEY,
                task_run_id     TEXT NOT NULL,
                workspace_id    TEXT NOT NULL,
                session_id      TEXT NOT NULL,
                turn_id         TEXT NOT NULL,
                rel_path        TEXT NOT NULL,
                status          TEXT NOT NULL DEFAULT 'awaiting_ack',
                session_version INTEGER,
                content_hash    TEXT,
                created_at      INTEGER NOT NULL,
                acknowledged_at INTEGER,
                consumed_at     INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_preview_checkpoint_run_status
                ON task_preview_checkpoints(task_run_id, status);
            CREATE INDEX IF NOT EXISTS idx_preview_checkpoint_session_status
                ON task_preview_checkpoints(workspace_id, session_id, status);
        "#,
    },
    Migration {
        version: 14,
        // WorkspaceOperation / WorkspaceTimeline 资源历史 / CreateDocumentBundle（WS-OPERATION-IMPLEMENTATION-01）。
        // 计划载荷与恢复载荷分离；计划状态可推进，但冻结的 operation_items/baseline 不得原地替换。
        sql: r#"
            CREATE TABLE IF NOT EXISTS workspace_operation_plans (
                plan_id             TEXT PRIMARY KEY,
                workspace_id        TEXT NOT NULL,
                task_run_id         TEXT,
                source_turn_id      TEXT,
                plan_json           TEXT NOT NULL,
                decision_request_id TEXT,
                idempotency_key     TEXT NOT NULL UNIQUE,
                status              TEXT NOT NULL,
                operation_id        TEXT,
                outcome_json        TEXT,
                created_at          INTEGER NOT NULL,
                updated_at          INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_workspace_operation_plans_ws_status
                ON workspace_operation_plans(workspace_id, status, updated_at);
            CREATE INDEX IF NOT EXISTS idx_workspace_operation_plans_task_run
                ON workspace_operation_plans(task_run_id, updated_at);

            CREATE TABLE IF NOT EXISTS workspace_operation_payloads (
                payload_ref TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                payload_kind TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS workspace_operation_snapshots (
                snapshot_ref TEXT NOT NULL,
                rel_path TEXT NOT NULL,
                resource_kind TEXT NOT NULL,
                content BLOB,
                created_at INTEGER NOT NULL,
                PRIMARY KEY(snapshot_ref, rel_path)
            );

            CREATE TABLE IF NOT EXISTS workspace_operation_entries (
                operation_id TEXT PRIMARY KEY,
                entry_id TEXT NOT NULL UNIQUE,
                plan_id TEXT NOT NULL,
                workspace_id TEXT NOT NULL,
                before_ref TEXT NOT NULL,
                after_ref TEXT NOT NULL,
                receipts_ref TEXT NOT NULL,
                restore_kind TEXT NOT NULL,
                restore_payload_ref TEXT,
                final_status TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_workspace_operation_entries_ws_time
                ON workspace_operation_entries(workspace_id, created_at);

            CREATE TABLE IF NOT EXISTS create_document_bundles (
                plan_id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                target_rel_path TEXT NOT NULL,
                initial_content_ref TEXT NOT NULL,
                operation_id TEXT,
                initial_diff_id TEXT,
                guard_ref TEXT,
                status TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_create_document_bundles_operation
                ON create_document_bundles(operation_id);
        "#,
    },
    Migration {
        version: 15,
        // WorkspaceOperation crash journal：逐项执行/补偿回执在文件系统副作用前后持续落库，
        // 使异常退出后的 Interrupted 计划可以依据真实进度生成新的恢复计划。
        sql: r#"
            ALTER TABLE workspace_operation_plans ADD COLUMN progress_json TEXT;
        "#,
    },
    Migration {
        version: 16,
        // 显式编辑目标集（AG-CONFORMANCE-CODE-S3D · TERM-AG-019 / BR-AG-DATA-003）：
        // 简单 AgentTurn 只在内存持有；需要跨执行片段的 TaskRun 逐目标持久化授权来源，
        // scope DecisionRequest confirm 与 CreateDocumentBundle 建盘后只做幂等追加。
        sql: r#"
            CREATE TABLE IF NOT EXISTS task_run_edit_targets (
                task_run_id  TEXT NOT NULL,
                rel_path     TEXT NOT NULL,
                source_kind  TEXT NOT NULL,
                source_ref   TEXT,
                authorized_at INTEGER NOT NULL,
                PRIMARY KEY(task_run_id, rel_path)
            );
            CREATE INDEX IF NOT EXISTS idx_task_run_edit_targets_run
                ON task_run_edit_targets(task_run_id, authorized_at);
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
    conn.query_row("SELECT MAX(schema_version) FROM schema_meta", [], |r| {
        r.get::<_, Option<i64>>(0)
    })
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
            conn.execute(
                "INSERT INTO ed_session_snapshots \
                 (workspace_id, rel_path, content, edit_format, dirty, session_version, content_hash, updated_at) \
                 VALUES ('ws1','a.md','逻辑镜像','text',1,2,'h2',?1)",
                (now_millis(),),
            )
            .unwrap();
            conn.execute(
                "INSERT INTO ed_open_tabs \
                 (workspace_id, rel_path, kind, open_index, is_active, updated_at) \
                 VALUES ('ws1','a.md','doc',0,1,?1)",
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
        let session_content: String = conn
            .query_row(
                "SELECT content FROM ed_session_snapshots WHERE workspace_id='ws1' AND rel_path='a.md'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(session_content, "逻辑镜像");
        let active_open_tab: i64 = conn
            .query_row(
                "SELECT is_active FROM ed_open_tabs WHERE workspace_id='ws1' AND rel_path='a.md'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(active_open_tab, 1);

        drop(conn);
        let _ = std::fs::remove_file(&path);
    }

    /// 测试专用：只迁移到指定版本（用于构造"迁移前"状态）。
    fn migrate_to(conn: &mut Connection, target: i64) {
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        let from = current_version(conn);
        for m in MIGRATIONS
            .iter()
            .filter(|m| m.version > from && m.version <= target)
        {
            let tx = conn.transaction().unwrap();
            tx.execute_batch(m.sql).unwrap();
            tx.execute(
                "INSERT INTO schema_meta (schema_version, applied_at) VALUES (?1, ?2)",
                (m.version, now_millis()),
            )
            .unwrap();
            tx.commit().unwrap();
        }
    }

    // covers: BR-SYS-DATA-005
    // v10 存量回填（2026-07-20 人类决策 4）：迁移前 accepted 行回填 saved，其余状态回填 not_applicable；
    // 新增两列在迁移前不存在，验证回填基于迁移前的 status 快照而非默认值巧合。
    #[test]
    fn migration_v10_backfills_effect_status_from_legacy_rows() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate_to(&mut conn, 9);
        conn.execute(
            "INSERT INTO workspaces (workspace_id, root_path, is_active, opened_at) VALUES ('ws1','/w',1,0)",
            [],
        )
        .unwrap();
        for (id, status) in [
            ("d_pending", "pending"),
            ("d_accepted", "accepted"),
            ("d_rejected", "rejected"),
            ("d_expired", "expired"),
        ] {
            conn.execute(
                "INSERT INTO pending_diffs \
                 (diff_id, workspace_id, rel_path, origin_module, status, payload_ref, created_at) \
                 VALUES (?1,'ws1','a.md','AG',?2,'p1',0)",
                rusqlite::params![id, status],
            )
            .unwrap();
        }

        migrate(&mut conn).unwrap(); // 应用 v10

        let effect_status_of = |conn: &Connection, id: &str| -> String {
            conn.query_row(
                "SELECT effect_status FROM pending_diffs WHERE diff_id=?1",
                [id],
                |r| r.get(0),
            )
            .unwrap()
        };
        assert_eq!(effect_status_of(&conn, "d_accepted"), "saved");
        assert_eq!(effect_status_of(&conn, "d_pending"), "not_applicable");
        assert_eq!(effect_status_of(&conn, "d_rejected"), "not_applicable");
        assert_eq!(effect_status_of(&conn, "d_expired"), "not_applicable");

        let terminal_reason: Option<String> = conn
            .query_row(
                "SELECT terminal_reason FROM pending_diffs WHERE diff_id='d_pending'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(terminal_reason, None);
    }
}
