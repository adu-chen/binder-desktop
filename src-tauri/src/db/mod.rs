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

/// agent_turn_events 结构 DDL（与已执行的 v17 同构；v19 前向补建漂移库缺失的结构）。
/// 索引/触发器名在 SQLite 中全局唯一：旧构建若已把同名对象挂在旧表上，
/// `IF NOT EXISTS` 会静默跳过，故先 DROP 再 CREATE，保证新表真正获得索引与 append-only 触发器。
const AGENT_TURN_EVENTS_TABLE_SQL: &str = r#"
    CREATE TABLE IF NOT EXISTS agent_turn_events (
        event_id       TEXT PRIMARY KEY,
        workspace_id   TEXT NOT NULL,
        session_id     TEXT NOT NULL,
        turn_id        TEXT NOT NULL,
        seq            INTEGER NOT NULL,
        kind           TEXT NOT NULL,
        payload_version INTEGER NOT NULL,
        payload_json   TEXT NOT NULL,
        occurred_at    INTEGER NOT NULL,
        UNIQUE(turn_id, seq)
    );
    DROP INDEX IF EXISTS idx_agent_turn_events_workspace_turn_seq;
    CREATE INDEX IF NOT EXISTS idx_agent_turn_events_workspace_turn_seq
        ON agent_turn_events(workspace_id, turn_id, seq);
    DROP TRIGGER IF EXISTS trg_agent_turn_events_no_update;
    CREATE TRIGGER IF NOT EXISTS trg_agent_turn_events_no_update
        BEFORE UPDATE ON agent_turn_events
        BEGIN
            SELECT RAISE(ABORT, 'agent_turn_events are append-only');
        END;
"#;

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
    Migration {
        version: 17,
        // Agent 1.0 append-only TurnEvent（AG-ADAPTIVE-RUNTIME-WP1-EVENTS-01）：运行事实按
        // turn_id 共用单调 seq；安全 Channel 投影与内部 context/gate/provider 事件共享顺序。
        // payload_json 不保存密钥、额外用户正文副本或模型隐藏推理。
        sql: r#"
            CREATE TABLE IF NOT EXISTS agent_turn_events (
                event_id       TEXT PRIMARY KEY,
                workspace_id   TEXT NOT NULL,
                session_id     TEXT NOT NULL,
                turn_id        TEXT NOT NULL,
                seq            INTEGER NOT NULL,
                kind           TEXT NOT NULL,
                payload_version INTEGER NOT NULL,
                payload_json   TEXT NOT NULL,
                occurred_at    INTEGER NOT NULL,
                UNIQUE(turn_id, seq)
            );
            CREATE INDEX IF NOT EXISTS idx_agent_turn_events_workspace_turn_seq
                ON agent_turn_events(workspace_id, turn_id, seq);
            CREATE TRIGGER IF NOT EXISTS trg_agent_turn_events_no_update
                BEFORE UPDATE ON agent_turn_events
                BEGIN
                    SELECT RAISE(ABORT, 'agent_turn_events are append-only');
                END;
        "#,
    },
    Migration {
        version: 18,
        // ProposalBatch（TERM-DE-005 / BR-DE-DATA-001）：批次只持久化来源与分组版本，
        // PendingDiff 直接保存批次身份和稳定组序；既有单卡数据无损迁移为一行一批。
        sql: r#"
            CREATE TABLE IF NOT EXISTS proposal_batches (
                proposal_batch_id TEXT PRIMARY KEY,
                workspace_id      TEXT NOT NULL,
                turn_id           TEXT,
                tool_call_id      TEXT,
                grouping_version  INTEGER NOT NULL,
                created_at        INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_proposal_batches_workspace_time
                ON proposal_batches(workspace_id, created_at);
            ALTER TABLE pending_diffs ADD COLUMN proposal_batch_id TEXT NOT NULL DEFAULT '';
            ALTER TABLE pending_diffs ADD COLUMN group_index INTEGER NOT NULL DEFAULT 1;
            ALTER TABLE pending_diffs ADD COLUMN group_count INTEGER NOT NULL DEFAULT 1;
            UPDATE pending_diffs SET proposal_batch_id = diff_id WHERE proposal_batch_id = '';
            INSERT OR IGNORE INTO proposal_batches
                (proposal_batch_id, workspace_id, turn_id, tool_call_id, grouping_version, created_at)
                SELECT proposal_batch_id, workspace_id, NULL, NULL, 1, created_at FROM pending_diffs;
            CREATE INDEX IF NOT EXISTS idx_pending_diffs_proposal_batch_group
                ON pending_diffs(proposal_batch_id, group_index);
        "#,
    },
    Migration {
        version: 19,
        // 迁移漂移修复（AG-ADAPTIVE-RUNTIME-WP1-EVENTS-01 升级路径）：早期构建在 schema_meta
        // 推进后仍把回合事件写入旧表名 turn_events，使创建 agent_turn_events 的 v17 不会重跑。
        // v19 前向补建同构结构（幂等），并在 migrate() 中同事务调用 reconcile_legacy_turn_events
        // 归一旧行；不回改已执行的 v17。健康库为 no-op。
        sql: AGENT_TURN_EVENTS_TABLE_SQL,
    },
    Migration {
        version: 20,
        // v19 已在真实库执行；以新版本修复七列旧表，不能依赖重跑 v19。
        sql: "",
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
        if m.version == 19 {
            reconcile_legacy_turn_events(&tx)?;
        }
        if m.version == 20 {
            migrate_legacy_turn_event_rows(&tx)?;
        }
        tx.execute(
            "INSERT INTO schema_meta (schema_version, applied_at) VALUES (?1, ?2)",
            (m.version, now_millis()),
        )?;
        tx.commit()?;
    }
    Ok(current_version(conn))
}

/// v19 归一：健康库（无 turn_events）为 no-op；若早期构建遗留同名事件表，将其仍符合
/// agent_turn_events 列形状的行幂等并入（INSERT OR IGNORE 依赖 UNIQUE(turn_id, seq)），
/// 保留原始 event_id/seq，使 afterSeq 补流与证据账本不受影响。列形状不兼容时仅告警不阻塞启动。
fn reconcile_legacy_turn_events(conn: &Connection) -> rusqlite::Result<()> {
    let legacy_present: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'turn_events'",
        [],
        |row| row.get(0),
    )?;
    if legacy_present == 0 {
        return Ok(());
    }
    const EVENT_COLUMNS: [&str; 9] = [
        "event_id",
        "workspace_id",
        "session_id",
        "turn_id",
        "seq",
        "kind",
        "payload_version",
        "payload_json",
        "occurred_at",
    ];
    let mut present = std::collections::HashSet::<String>::new();
    {
        let mut statement = conn.prepare("SELECT name FROM pragma_table_info('turn_events')")?;
        let names = statement.query_map([], |row| row.get::<_, String>(0))?;
        for name in names {
            present.insert(name?);
        }
    }
    if !EVENT_COLUMNS.iter().all(|column| present.contains(*column)) {
        eprintln!("[db] 检测到旧表 turn_events，但列与 agent_turn_events 不兼容，跳过事件归一");
        return Ok(());
    }
    let column_list = EVENT_COLUMNS.join(", ");
    let copied = conn.execute(
        &format!(
            "INSERT OR IGNORE INTO agent_turn_events ({column_list}) \
             SELECT {column_list} FROM turn_events"
        ),
        [],
    )?;
    if copied > 0 {
        eprintln!("[db] 已将 turn_events 的 {copied} 行回合事件归一至 agent_turn_events");
    }
    Ok(())
}

/// 真实七列旧表从 AgentTurn 补回 workspace/session，保留事件身份、seq 与 created_at。
/// 无法确定归属或已有同身份不同事实时回滚本次迁移；不伪造归属、不覆盖当前事件。
fn migrate_legacy_turn_event_rows(conn: &Connection) -> rusqlite::Result<()> {
    let columns = conn
        .prepare("SELECT name FROM pragma_table_info('turn_events')")?
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<std::collections::HashSet<_>, _>>()?;
    if columns.is_empty() {
        return Ok(());
    }
    let common = [
        "event_id",
        "turn_id",
        "seq",
        "kind",
        "payload_version",
        "payload_json",
    ];
    let current_shape = ["workspace_id", "session_id", "occurred_at"]
        .iter()
        .all(|column| columns.contains(*column));
    if !common.iter().all(|column| columns.contains(*column))
        || (!current_shape && !columns.contains("created_at"))
    {
        return Err(rusqlite::Error::InvalidParameterName(
            "AG_EVENT_MIGRATION_SCHEMA: unsupported legacy turn_events columns".into(),
        ));
    }
    let projection = if current_shape {
        "SELECT e.event_id,e.workspace_id,e.session_id,e.turn_id,e.seq,e.kind,e.payload_version,e.payload_json,e.occurred_at FROM turn_events e"
    } else {
        "SELECT e.event_id,t.workspace_id,t.session_id,e.turn_id,e.seq,e.kind,e.payload_version,e.payload_json,e.created_at AS occurred_at FROM turn_events e LEFT JOIN agent_turns t ON t.turn_id=e.turn_id"
    };
    // 校验整个来源再插入，允许完全相同的重复行，禁止 INSERT OR IGNORE 静默丢弃冲突事实。
    let invalid: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM ({projection}) WHERE workspace_id IS NULL OR trim(workspace_id)='' OR session_id IS NULL OR trim(session_id)=''"),
        [], |row| row.get(0),
    )?;
    if invalid != 0 {
        return Err(rusqlite::Error::InvalidParameterName(
            "AG_EVENT_MIGRATION_IDENTITY: legacy events have no recoverable workspace/session"
                .into(),
        ));
    }
    let conflicts: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM ({projection}) old JOIN agent_turn_events current
            ON current.event_id=old.event_id OR (current.turn_id=old.turn_id AND current.seq=old.seq)
            WHERE current.event_id IS NOT old.event_id OR current.workspace_id IS NOT old.workspace_id
            OR current.session_id IS NOT old.session_id OR current.turn_id IS NOT old.turn_id
            OR current.seq IS NOT old.seq OR current.kind IS NOT old.kind
            OR current.payload_version IS NOT old.payload_version OR current.payload_json IS NOT old.payload_json
            OR current.occurred_at IS NOT old.occurred_at"),
        [], |row| row.get(0),
    )?;
    if conflicts != 0 {
        return Err(rusqlite::Error::InvalidParameterName(
            "AG_EVENT_MIGRATION_CONFLICT: legacy and current event facts disagree".into(),
        ));
    }
    conn.execute(
        &format!("INSERT INTO agent_turn_events (event_id,workspace_id,session_id,turn_id,seq,kind,payload_version,payload_json,occurred_at)
            SELECT * FROM ({projection}) old WHERE NOT EXISTS (SELECT 1 FROM agent_turn_events current WHERE current.event_id=old.event_id)"),
        [],
    )?;
    // 已迁移行仍保留在旧表作为升级证据；生产读取仅使用 agent_turn_events。
    Ok(())
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
        let turn_event_columns: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('agent_turn_events')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(turn_event_columns, 9);
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

    // covers: BR-SYS-PERSIST-001, BR-DE-DATA-001
    // v18 将既有单卡无损登记为一行一批，避免重启恢复出现无批次身份的第二种活动结构。
    #[test]
    fn migration_v18_backfills_legacy_diff_batch_identity() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate_to(&mut conn, 17);
        conn.execute(
            "INSERT INTO workspaces (workspace_id, root_path, is_active, opened_at) VALUES ('ws1','/w',1,0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO pending_diffs \
             (diff_id, workspace_id, rel_path, origin_module, status, payload_ref, created_at) \
             VALUES ('legacy-diff','ws1','a.md','AG','pending','p1',42)",
            [],
        )
        .unwrap();

        migrate(&mut conn).unwrap();

        let identity: (String, u32, u32) = conn
            .query_row(
                "SELECT proposal_batch_id, group_index, group_count FROM pending_diffs WHERE diff_id='legacy-diff'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(identity, ("legacy-diff".into(), 1, 1));
        let batch: (String, u32) = conn
            .query_row(
                "SELECT workspace_id, grouping_version FROM proposal_batches WHERE proposal_batch_id='legacy-diff'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(batch, ("ws1".into(), 1));
    }

    fn seed_real_legacy_events(conn: &Connection) {
        conn.execute_batch(
            r#"
            CREATE TABLE turn_events (
                event_id TEXT PRIMARY KEY, turn_id TEXT NOT NULL, seq INTEGER NOT NULL,
                kind TEXT NOT NULL, payload_version INTEGER NOT NULL,
                payload_json TEXT NOT NULL, created_at INTEGER NOT NULL, UNIQUE(turn_id,seq)
            );
            INSERT INTO agent_turns (turn_id,workspace_id,session_id,payload_ref,created_at)
                VALUES ('old-turn','ws-old','session-old','payload-old',40);
            INSERT INTO turn_events VALUES ('old-event','old-turn',1,'turn_started',1,'{}',42);
        "#,
        )
        .unwrap();
    }

    // covers: BR-SYS-PERSIST-001
    // covers: BR-AG-PERSIST-001
    #[test]
    fn migration_v20_preserves_real_seven_column_events_from_v18_and_v19() {
        for version in [18, 19] {
            let mut conn = Connection::open_in_memory().unwrap();
            migrate_to(&mut conn, version);
            seed_real_legacy_events(&conn);
            if version == 18 {
                conn.execute_batch("DROP TABLE agent_turn_events;").unwrap();
            }
            assert_eq!(migrate(&mut conn).unwrap(), latest());
            let identity: (String,String,i64,String) = conn.query_row(
                "SELECT workspace_id,session_id,occurred_at,payload_json FROM agent_turn_events WHERE event_id='old-event'",
                [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)),
            ).unwrap();
            assert_eq!(
                identity,
                ("ws-old".into(), "session-old".into(), 42, "{}".into())
            );
            assert_eq!(
                agent_turn_event_rows(&conn),
                vec![("old-event".into(), 1, "turn_started".into())]
            );
            assert_eq!(migrate(&mut conn).unwrap(), latest());
            assert_eq!(agent_turn_event_rows(&conn).len(), 1);
            assert!(has_agent_turn_event_structure(&conn));
        }
    }

    // covers: BR-SYS-PERSIST-001
    // covers: BR-AG-PERSIST-001
    #[test]
    fn migration_v20_rolls_back_missing_identity_and_conflicting_facts() {
        for defect in ["orphan", "session", "event_conflict", "seq_conflict"] {
            let mut conn = Connection::open_in_memory().unwrap();
            migrate_to(&mut conn, 19);
            seed_real_legacy_events(&conn);
            match defect {
                "orphan" => {
                    conn.execute("DELETE FROM agent_turns", []).unwrap();
                }
                "session" => {
                    conn.execute("UPDATE agent_turns SET session_id=NULL", [])
                        .unwrap();
                }
                _ => {
                    let event_id = if defect == "event_conflict" {
                        "old-event"
                    } else {
                        "different-event"
                    };
                    conn.execute("INSERT INTO agent_turn_events VALUES (?1,'ws-old','session-old','old-turn',1,'chunk',1,'{}',42)", [event_id]).unwrap();
                }
            }
            let before = agent_turn_event_rows(&conn);
            let error = migrate(&mut conn).unwrap_err().to_string();
            let expected = if defect == "orphan" || defect == "session" {
                "AG_EVENT_MIGRATION_IDENTITY"
            } else {
                "AG_EVENT_MIGRATION_CONFLICT"
            };
            assert!(error.contains(expected), "{defect}: {error}");
            assert_eq!(current_version(&conn), 19);
            assert_eq!(agent_turn_event_rows(&conn), before);
            assert_eq!(
                conn.query_row("SELECT COUNT(*) FROM turn_events", [], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                1
            );
        }
    }

    // covers: BR-SYS-PERSIST-001
    #[test]
    fn migration_v20_accepts_identical_rows() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate_to(&mut conn, 19);
        seed_real_legacy_events(&conn);
        conn.execute_batch("INSERT INTO agent_turn_events VALUES ('old-event','ws-old','session-old','old-turn',1,'turn_started',1,'{}',42)").unwrap();
        assert_eq!(migrate(&mut conn).unwrap(), latest());
        assert_eq!(agent_turn_event_rows(&conn).len(), 1);
    }

    fn has_agent_turn_event_structure(conn: &Connection) -> bool {
        let table: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='agent_turn_events'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let index: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' \
                 AND name='idx_agent_turn_events_workspace_turn_seq' AND tbl_name='agent_turn_events'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let trigger: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='trigger' \
                 AND name='trg_agent_turn_events_no_update' AND tbl_name='agent_turn_events'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        table == 1 && index == 1 && trigger == 1
    }

    fn agent_turn_event_rows(conn: &Connection) -> Vec<(String, i64, String)> {
        conn.prepare("SELECT event_id, seq, kind FROM agent_turn_events ORDER BY seq")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    // 同构改名夹具；真实七列旧表另由 migration_v20 测试覆盖。
    #[test]
    fn migration_v19_normalizes_legacy_turn_events_table() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate_to(&mut conn, 18);
        // 模拟早期构建形态：表改名后触发器/索引仍占同名（全局唯一），v19 必须先 DROP 再建。
        conn.execute_batch("ALTER TABLE agent_turn_events RENAME TO turn_events;")
            .unwrap();
        conn.execute(
            "INSERT INTO turn_events \
             (event_id,workspace_id,session_id,turn_id,seq,kind,payload_version,payload_json,occurred_at) \
             VALUES \
             ('turn-event:t1:1','ws1','s1','t1',1,'turn_started',1,'{\"runtimeProfile\":\"edit\"}',100), \
             ('turn-event:t1:2','ws1','s1','t1',2,'context_frozen',1,'{\"openDocCount\":1}',101)",
            [],
        )
        .unwrap();

        assert_eq!(migrate(&mut conn).unwrap(), latest());
        assert!(has_agent_turn_event_structure(&conn));
        assert_eq!(
            agent_turn_event_rows(&conn),
            vec![
                ("turn-event:t1:1".to_string(), 1, "turn_started".to_string()),
                (
                    "turn-event:t1:2".to_string(),
                    2,
                    "context_frozen".to_string()
                ),
            ]
        );
        // 幂等：再次迁移版本不涨、行不重复。
        assert_eq!(migrate(&mut conn).unwrap(), latest());
        assert_eq!(agent_turn_event_rows(&conn).len(), 2);
        let version_rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_meta", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version_rows, latest());
    }

    // 健康库（无 turn_events）：v19 为 no-op，不丢既有行、不新建旧表。
    #[test]
    fn migration_v19_is_noop_without_legacy_table() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate_to(&mut conn, 18);
        conn.execute(
            "INSERT INTO agent_turn_events \
             (event_id,workspace_id,session_id,turn_id,seq,kind,payload_version,payload_json,occurred_at) \
             VALUES ('turn-event:t1:1','ws1','s1','t1',1,'chunk',1,'{\"text\":\"A\"}',100)",
            [],
        )
        .unwrap();

        assert_eq!(migrate(&mut conn).unwrap(), latest());
        assert!(has_agent_turn_event_structure(&conn));
        assert_eq!(agent_turn_event_rows(&conn).len(), 1);
        let legacy: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='turn_events'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(legacy, 0);
    }

    // 文件级升级夹具：覆盖“从旧迁移形态升级后的真实持久数据库”跨重启场景。
    #[test]
    fn file_db_upgrade_from_legacy_turn_events_survives_restart() {
        let mut path = std::env::temp_dir();
        path.push(format!("binder_upgrade_legacy_{}.db", now_millis()));
        {
            let mut conn = Connection::open(&path).unwrap();
            migrate_to(&mut conn, 19);
            seed_real_legacy_events(&conn);
        }
        {
            // 升级后首次启动：漂移被修复并归一。
            let conn = open_and_migrate(&path).unwrap();
            assert!(has_agent_turn_event_structure(&conn));
            assert_eq!(agent_turn_event_rows(&conn).len(), 1);
        }
        {
            // 日常再次启动：结构仍在、迁移幂等。
            let conn = open_and_migrate(&path).unwrap();
            assert!(has_agent_turn_event_structure(&conn));
            assert_eq!(agent_turn_event_rows(&conn).len(), 1);
            let version_rows: i64 = conn
                .query_row("SELECT COUNT(*) FROM schema_meta", [], |r| r.get(0))
                .unwrap();
            assert_eq!(version_rows, latest());
        }
        let _ = std::fs::remove_file(&path);
    }
}
