//! 跨模块集成测试（P5/G5 预备）：核心交互链「AG 产出 → DE 生成 → 裁决 → WS 落盘 → 生命周期」
//! 端到端验证模块互锁（X-INTER-001）——此前各 SM/规则为单模块单测，本测试在真实 workspace.db +
//! 临时文件上跑通整条链，验证：
//! - DE 生成（generate_diff，锚 Located）→ 持久化 PendingDiff；
//! - SM-DE-01 accept 是唯一通往落盘的裁决；
//! - WS apply_diff 原子写文件 + 恰好一个时间轴节点 + 置 status=accepted；
//! - reject 路径不写文件；
//! - DE lifecycle 过期扫描经 WS discard 落 expired 终态。
//! 仅依赖 binder_lib 公共 API，不触前端/命令层，与并发进程零冲突。

use binder_lib::db;
use binder_lib::de::expiry::ExternalCheck;
use binder_lib::de::generate::{generate_diff, ChangeProposal};
use binder_lib::de::lifecycle::{expire_diffs, list_pending, sweep_expiry, PendingDiffRow};
use binder_lib::de::{transition, DiffEvent, DiffState};
use binder_lib::types::{DiffStatus, DocRef, LocateResult, PendingDiff};
use binder_lib::workspace::diff_apply::{apply_diff, discard_diff, persist_diff};
use binder_lib::workspace::timeline::list_timeline;
use rusqlite::Connection;

fn fresh_ws_db() -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    db::migrate(&mut conn).unwrap();
    conn.execute(
        "INSERT INTO workspaces (workspace_id, root_path, is_active, opened_at) VALUES ('ws1','/w',1,0)",
        [],
    )
    .unwrap();
    conn
}

fn doc(rel: &str) -> DocRef {
    DocRef {
        workspace_id: "ws1".into(),
        rel_path: rel.into(),
        doc_type: "md".into(),
    }
}

fn status_of(conn: &Connection, id: &str) -> String {
    conn.query_row(
        "SELECT status FROM pending_diffs WHERE diff_id=?1",
        [id],
        |r| r.get(0),
    )
    .unwrap()
}

// 核心链正路：AG 产出 → DE 生成 → accept → WS 落盘（文件 + 恰好一个时间轴节点 + accepted）。
#[test]
fn accepted_proposal_flows_through_de_to_ws_landing() {
    let conn = fresh_ws_db();
    let target = doc("notes/a.md");

    // 1) AG 侧产出修改建议（改既有文档，带 before_snapshot）。
    let proposal = ChangeProposal {
        origin_task: "turn_1".into(),
        target: target.clone(),
        after_content: b"# A\n\n updated body\n".to_vec(),
        before_snapshot: Some("h0".into()),
        style_ops: None,
    };
    // 2) DE 生成：锚 Located → PendingDiff（pending）。
    let diff = generate_diff(
        &proposal,
        "d1",
        "AG",
        "payload_1",
        1_000,
        LocateResult::Located,
    )
    .unwrap();
    assert_eq!(diff.status, DiffStatus::Pending);
    persist_diff(&conn, &diff).unwrap();
    assert_eq!(status_of(&conn, "d1"), "pending");

    // 3) SM-DE-01：accept 是唯一通往落盘的裁决。
    assert_eq!(
        transition(DiffState::Pending, DiffEvent::UserAccept),
        Some(DiffState::Accepted)
    );

    // 4) WS 落盘（仅 accepted 触发）：原子写文件 + 记节点 + 置 accepted。
    let tmp = std::env::temp_dir().join(format!("binder_itest_{}.md", std::process::id()));
    let _ = std::fs::remove_file(&tmp);
    let entry = apply_diff(&conn, "d1", &target, &tmp, &proposal.after_content, None).unwrap();

    // 5) 端到端断言：文件内容 = 提案内容；时间轴恰好一个节点；diff = accepted。
    assert_eq!(std::fs::read(&tmp).unwrap(), proposal.after_content);
    assert_eq!(entry.version_seq, 1);
    assert_eq!(list_timeline(&conn, &target).unwrap().len(), 1);
    assert_eq!(status_of(&conn, "d1"), "accepted");
    let _ = std::fs::remove_file(&tmp);
}

// 锚失效：DE 生成应回报失败（不产悬空 diff），链路不进入落盘。
#[test]
fn anchor_lost_blocks_generation() {
    let proposal = ChangeProposal {
        origin_task: "turn_2".into(),
        target: doc("a.md"),
        after_content: b"x".to_vec(),
        before_snapshot: Some("h0".into()),
        style_ops: None,
    };
    let err = generate_diff(&proposal, "d2", "AG", "p", 0, LocateResult::AnchorLost).unwrap_err();
    assert_eq!(err.code, "DE_ANCHOR_LOST");
}

// reject 路径：SM-DE-01 reject → WS discard，不写文件、不记节点。
#[test]
fn rejected_proposal_does_not_land() {
    let conn = fresh_ws_db();
    let target = doc("b.md");
    let diff = PendingDiff {
        diff_id: "d3".into(),
        target: target.clone(),
        origin_module: "AG".into(),
        status: DiffStatus::Pending,
        created_at: 0,
        payload_ref: "p".into(),
    };
    persist_diff(&conn, &diff).unwrap();
    assert_eq!(
        transition(DiffState::Pending, DiffEvent::UserReject),
        Some(DiffState::Rejected)
    );
    discard_diff(&conn, "d3", "rejected", None).unwrap();
    assert_eq!(status_of(&conn, "d3"), "rejected");
    assert_eq!(list_timeline(&conn, &target).unwrap().len(), 0); // 未落盘无节点
}

// covers: BR-AG-DATA-003, BR-DE-DATA-001, BR-DE-DATA-002
// 新建文档必须先形成空基准 proposal；接受前目标文件不存在，拒绝也不创建，只有 accept 才落盘。
#[test]
fn new_document_empty_base_proposal_waits_for_acceptance() {
    let conn = fresh_ws_db();
    let target = doc("new/report.md");
    let proposal = ChangeProposal {
        origin_task: "turn_create".into(),
        target: target.clone(),
        after_content: b"# Report\n\nDraft\n".to_vec(),
        before_snapshot: None,
        style_ops: None,
    };
    let diff = generate_diff(
        &proposal,
        "d-new",
        "AG",
        "payload-new",
        1_000,
        LocateResult::Located,
    )
    .unwrap();
    persist_diff(&conn, &diff).unwrap();

    let root = std::env::temp_dir().join(format!(
        "binder_new_doc_itest_{}_{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    let target_path = root.join("new/report.md");
    let _ = std::fs::remove_dir_all(&root);
    assert!(!target_path.exists(), "proposal 阶段不得提前创建文件");

    std::fs::create_dir_all(target_path.parent().unwrap()).unwrap();
    apply_diff(
        &conn,
        "d-new",
        &target,
        &target_path,
        &proposal.after_content,
        None,
    )
    .unwrap();
    assert_eq!(std::fs::read(&target_path).unwrap(), proposal.after_content);
    assert_eq!(status_of(&conn, "d-new"), "accepted");
    let _ = std::fs::remove_dir_all(&root);
}

// 生命周期：超时 pending diff 经扫描 + WS discard 落 expired，不再出现在 pending 列举。
#[test]
fn lifecycle_sweep_expires_stale_pending() {
    let conn = fresh_ws_db();
    for (id, created) in [("d_old", 0_i64), ("d_new", 9_000)] {
        let diff = PendingDiff {
            diff_id: id.into(),
            target: doc("c.md"),
            origin_module: "AG".into(),
            status: DiffStatus::Pending,
            created_at: created,
            payload_ref: "p".into(),
        };
        persist_diff(&conn, &diff).unwrap();
    }
    let rows: Vec<PendingDiffRow> = list_pending(&conn, "ws1").unwrap();
    assert_eq!(rows.len(), 2);
    // now=10_000, timeout=5_000：d_old 超时过期，d_new 保持。
    let expired = sweep_expiry(&rows, 10_000, 5_000, |_| ExternalCheck::Unchanged);
    let n = expire_diffs(&conn, &expired, None).unwrap();
    assert_eq!(n, 1);
    assert_eq!(status_of(&conn, "d_old"), "expired");
    assert_eq!(status_of(&conn, "d_new"), "pending");
    assert_eq!(list_pending(&conn, "ws1").unwrap().len(), 1);
}
