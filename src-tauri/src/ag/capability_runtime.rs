//! Owned execution port. Waiting can end independently; effect facts cannot be discarded.
use crate::{
    ipc::IpcError,
    types::{
        CapabilityExecutionFactV1, CapabilityExecutionPhase, ToolCallReceiptIdentityV1,
        ToolCallReceiptReferenceV1,
    },
    AppState,
};
use rusqlite::{params, Connection};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

fn fact_key(identity: &ToolCallReceiptIdentityV1, phase: &str) -> String {
    format!(
        "ag-effect:{}:{phase}",
        serde_json::to_string(&(
            &identity.workspace_id,
            &identity.session_id,
            &identity.turn_id,
            &identity.tool_call_id
        ))
        .expect("string identity")
    )
}

fn persist_fact(conn: &Connection, fact: &CapabilityExecutionFactV1) -> Result<(), IpcError> {
    let phase = match fact.phase {
        CapabilityExecutionPhase::Started => "started",
        CapabilityExecutionPhase::Settled => "settled",
    };
    let payload = serde_json::to_string(fact).map_err(db_error)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0);
    conn.execute("INSERT INTO agent_turn_payloads (payload_ref,workspace_id,payload_json,payload_size,created_at) VALUES (?1,?2,?3,?4,?5)", params![fact_key(&fact.identity, phase), fact.identity.workspace_id, payload, payload.len(), now]).map_err(db_error)?;
    if cfg!(debug_assertions) {
        eprintln!(
            "[AG-AG-TOOL] effect_recorded: {}",
            serde_json::json!({
                "workspaceId": fact.identity.workspace_id, "sessionId": fact.identity.session_id,
                "turnId": fact.identity.turn_id, "toolCallId": fact.identity.tool_call_id,
                "occurredAt": now, "phase": phase,
                "executionStatus": fact.receipt.as_ref().map(|receipt| &receipt.execution_status),
                "effectStatus": fact.receipt.as_ref().map(|receipt| &receipt.effect_status),
            })
        );
    }
    Ok(())
}

fn db_error(error: impl std::fmt::Display) -> IpcError {
    IpcError::new(
        "AG_EFFECT_FACT_UNAVAILABLE",
        &format!("执行事实不可核对：{error}；禁止自动重放"),
        false,
        false,
    )
}

/**
 * @GOV
 * codes: BR-AG-PERSIST-001-QUERY-AG-AG-TOOL-046, BR-SYS-STATE-007-QUERY-AG-AG-TOOL-046
 * type: QUERY
 * chain: AG-TOOL, AG-TURN
 * rules: BR-AG-PERSIST-001, BR-SYS-STATE-007
 * boundary: in=WorkspaceIdentityAndOptionalSessionTurnIdentity | out=PersistedCapabilityExecutionFacts | does_not=RestoreExecutionAuthorityOrAppendAfterTerminalTurnEvent | consumer=BR-AG-PERSIST-001,BR-SYS-STATE-007
 */
pub(super) fn facts(
    conn: &Connection,
    workspace: &str,
    session_turn: Option<(&str, &str)>,
) -> rusqlite::Result<Vec<CapabilityExecutionFactV1>> {
    let mut stmt = conn.prepare("SELECT payload_json FROM agent_turn_payloads WHERE workspace_id=?1 AND payload_ref LIKE 'ag-effect:%' ORDER BY created_at,payload_ref")?;
    let rows = stmt.query_map([workspace], |row| row.get::<_, String>(0))?;
    let mut facts = Vec::new();
    for row in rows {
        let fact: CapabilityExecutionFactV1 = serde_json::from_str(&row?).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?;
        if fact.identity.workspace_id != workspace {
            return Err(rusqlite::Error::InvalidQuery);
        }
        if session_turn.is_none_or(|(session, turn)| {
            fact.identity.session_id == session && fact.identity.turn_id == turn
        }) {
            facts.push(fact);
        }
    }
    Ok(facts)
}

fn start_fact(conn: &Connection, identity: &ToolCallReceiptIdentityV1) -> Result<(), IpcError> {
    let owns: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM agent_turns WHERE workspace_id=?1 AND session_id=?2 AND turn_id=?3 AND status NOT IN ('completed','error','cancelled','gate_rejected'))", params![identity.workspace_id,identity.session_id,identity.turn_id], |row| row.get(0)).map_err(db_error)?;
    if !owns {
        return Err(IpcError::new(
            "AG_TURN_CANCELLED",
            "回合已结束，能力未启动",
            false,
            false,
        ));
    }
    let previous = facts(conn, &identity.workspace_id, None).map_err(db_error)?;
    for fact in previous
        .iter()
        .filter(|fact| fact.phase == CapabilityExecutionPhase::Started)
    {
        let settlement = previous.iter().find(|item| {
            item.phase == CapabilityExecutionPhase::Settled && item.identity == fact.identity
        });
        let unresolved = match settlement {
            None => true,
            Some(item) => match item.receipt.as_ref() {
                None => true,
                Some(receipt) => {
                    receipt.effect_status == crate::types::ToolCallEffectStatus::Unknown
                        && !legacy_proposal_precommit_failure(conn, item).map_err(db_error)?
                }
            },
        };
        if unresolved {
            return Err(IpcError::new(
                "AG_EFFECT_PENDING",
                "工作区存在未结算或效果未知的能力；请核对原执行事实，禁止再次写入",
                false,
                false,
            ));
        }
        if fact.identity.turn_id == identity.turn_id
            && fact.identity.tool_call_id == identity.tool_call_id
        {
            return Err(IpcError::new(
                "AG_MODEL_TOOL_ID_INVALID",
                "相同操作身份不得重复执行",
                false,
                false,
            ));
        }
    }
    persist_fact(
        conn,
        &CapabilityExecutionFactV1 {
            identity: identity.clone(),
            phase: CapabilityExecutionPhase::Started,
            receipt: None,
        },
    )
}

/// Older builds classified every failed write adapter as an unknown effect. A propose_edit
/// AG_TOOL_ARG is emitted before persist_proposal_batch, and that store commits its batch and
/// pending diffs in one transaction. Require the absence of that exact call's batch as a second
/// independent proof; never infer no effect for any other error or tool.
fn legacy_proposal_precommit_failure(
    conn: &Connection,
    fact: &CapabilityExecutionFactV1,
) -> rusqlite::Result<bool> {
    let Some(receipt) = fact.receipt.as_ref() else {
        return Ok(false);
    };
    if fact.identity.tool_name != "propose_edit"
        || receipt.effect_status != crate::types::ToolCallEffectStatus::Unknown
        || receipt
            .failure
            .as_ref()
            .map(|failure| failure.code.as_str())
            != Some("AG_TOOL_ARG")
        || !receipt.references.is_empty()
    {
        return Ok(false);
    }
    let batch_exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM proposal_batches WHERE workspace_id=?1 AND turn_id=?2 AND tool_call_id=?3)",
        params![
            fact.identity.workspace_id,
            fact.identity.turn_id,
            fact.identity.tool_call_id
        ],
        |row| row.get(0),
    )?;
    Ok(!batch_exists)
}

pub(super) fn delete_settled_session_facts(
    conn: &Connection,
    workspace: &str,
    session: &str,
) -> rusqlite::Result<usize> {
    let records = facts(conn, workspace, None)?
        .into_iter()
        .filter(|fact| fact.identity.session_id == session)
        .collect::<Vec<_>>();
    for started in records
        .iter()
        .filter(|fact| fact.phase == CapabilityExecutionPhase::Started)
    {
        let receipt = records
            .iter()
            .find(|fact| {
                fact.phase == CapabilityExecutionPhase::Settled && fact.identity == started.identity
            })
            .and_then(|fact| fact.receipt.as_ref());
        if receipt.is_none_or(|receipt| {
            receipt.effect_status == crate::types::ToolCallEffectStatus::Unknown
        }) {
            return Err(rusqlite::Error::InvalidParameterName(
                "AG_EFFECT_PENDING: cannot delete unresolved effect identity".into(),
            ));
        }
    }
    let mut deleted = 0;
    for fact in records {
        deleted += conn.execute(
            "DELETE FROM agent_turn_payloads WHERE workspace_id=?1 AND payload_ref=?2",
            params![
                workspace,
                fact_key(
                    &fact.identity,
                    match fact.phase {
                        CapabilityExecutionPhase::Started => "started",
                        CapabilityExecutionPhase::Settled => "settled",
                    }
                )
            ],
        )?;
    }
    Ok(deleted)
}

/**
 * @GOV
 * codes: BR-AG-STATE-002-EFFECT-AG-AG-TOOL-048, BR-SYS-STATE-007-EFFECT-AG-AG-TOOL-048, BR-AG-PERSIST-001-EFFECT-AG-AG-TOOL-048
 * type: EFFECT
 * chain: AG-TOOL, AG-TURN
 * rules: BR-AG-STATE-002, BR-SYS-STATE-007, BR-AG-PERSIST-001
 * boundary: in=ConfirmedWorkspaceOperationClosureAndFrozenToolCallIdentityAndCallerOwnedEffectLease | out=WorkspaceOperationReceiptAndPersistentEffectSettlement | effect=workspaceDb,DelegatedCapability | does_not=InferDiskSuccessFromModelTextOrRewriteAgentTurnTerminal | consumer=BR-AG-STATE-002,BR-AG-PERSIST-001,BR-SYS-STATE-007
 */
pub(super) fn record_owned_execution(
    state: &AppState,
    identity: ToolCallReceiptIdentityV1,
    operation: impl FnOnce() -> Result<crate::types::WorkspaceOperationReceipt, IpcError>,
) -> Result<crate::types::WorkspaceOperationReceipt, IpcError> {
    // The caller owns an EffectLease before resolving the decision.
    {
        let conn = state
            .db
            .lock()
            .map_err(|_| db_error("database lock poisoned"))?;
        start_fact(&conn, &identity)?;
    }
    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation)).unwrap_or_else(|_| {
            Err(IpcError::new(
                "AG_CAPABILITY_WORKER_FAILED",
                "执行异常，效果待核对",
                false,
                false,
            ))
        });
    let mut receipt = super::tool_exec::tool_call_receipt(
        identity.clone(),
        true,
        result.is_ok(),
        result.as_ref().err(),
        vec![],
        vec![],
    );
    if let Ok(effect) = &result {
        receipt
            .references
            .push(crate::types::ToolCallReceiptReferenceV1 {
                kind: crate::types::ToolCallReceiptReferenceKind::WorkspaceOperationPlan,
                id: effect.plan_id.clone(),
            });
        if effect.final_status == crate::types::WorkspaceOperationStatus::Completed {
            receipt.effect_status = crate::types::ToolCallEffectStatus::ResourceExecuted;
            receipt.completion_status = crate::types::ToolCallCompletionStatus::Completed;
            receipt.next_action = crate::types::ToolCallNextAction::None;
        } else if effect.final_status == crate::types::WorkspaceOperationStatus::AwaitingContent {
            receipt.effect_status = crate::types::ToolCallEffectStatus::ResourceExecuted;
            receipt.completion_status = crate::types::ToolCallCompletionStatus::Waiting;
            receipt.next_action = crate::types::ToolCallNextAction::AwaitUser;
        }
    }
    let conn = state
        .db
        .lock()
        .map_err(|_| db_error("database lock poisoned"))?;
    persist_fact(
        &conn,
        &CapabilityExecutionFactV1 {
            identity,
            phase: CapabilityExecutionPhase::Settled,
            receipt: Some(receipt),
        },
    )?;
    result
}

/** @GOV
 * codes: BR-AG-STATE-002-EFFECT-AG-AG-TOOL-046, BR-SYS-STATE-007-EFFECT-AG-AG-TOOL-046, BR-AG-PERSIST-001-EFFECT-AG-AG-TOOL-046
 * type: EFFECT
 * chain: AG-TOOL, AG-TURN
 * rules: BR-AG-STATE-002, BR-SYS-STATE-007, BR-AG-PERSIST-001
 * boundary: in=AuthorizedCapabilityClosureAndFrozenToolCallIdentityAndEffectLease | out=CapabilityResultOrBoundedWaitFailureAndPersistentEffectSettlement | effect=workspaceDb,DelegatedCapability | does_not=ReplayWritesOrInjectLateResultIntoModelOrRewriteAgentTurnTerminal | consumer=BR-AG-STATE-002,BR-AG-PERSIST-001,BR-SYS-STATE-007
 */
pub(super) async fn execute<T: Send + 'static>(
    state: AppState,
    identity: ToolCallReceiptIdentityV1,
    cancelled: Arc<AtomicBool>,
    timeout: Duration,
    operation: impl FnOnce(&AppState) -> Result<T, IpcError> + Send + 'static,
    references: impl FnOnce(&T) -> Vec<ToolCallReceiptReferenceV1> + Send + 'static,
) -> Result<T, IpcError> {
    execute_with_effect_boundary(
        state, identity, cancelled, timeout, None, operation, references,
    )
    .await
}

/// A proposal adapter may prove that failure happened before its only persistence boundary.
/// All other adapters keep the conservative unknown-effect receipt.
pub(super) async fn execute_with_effect_boundary<T: Send + 'static>(
    state: AppState,
    identity: ToolCallReceiptIdentityV1,
    cancelled: Arc<AtomicBool>,
    timeout: Duration,
    effect_boundary: Option<Arc<AtomicBool>>,
    operation: impl FnOnce(&AppState) -> Result<T, IpcError> + Send + 'static,
    references: impl FnOnce(&T) -> Vec<ToolCallReceiptReferenceV1> + Send + 'static,
) -> Result<T, IpcError> {
    let lease = super::turn_control::lock_agent_turn_effect(
        &state,
        &identity.session_id,
        &identity.turn_id,
    )?;
    let deadline = Instant::now() + timeout;
    let worker_cancelled = cancelled.clone();
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        let result = (|| {
            if worker_cancelled.load(Ordering::Acquire) {
                return Err(IpcError::new(
                    "AG_TURN_CANCELLED",
                    "能力尚未启动即取消",
                    false,
                    false,
                ));
            }
            if Instant::now() >= deadline {
                return Err(IpcError::new(
                    "AG_CAPABILITY_TIMEOUT",
                    "能力排队期间已超时，未启动操作",
                    false,
                    false,
                ));
            }
            {
                let conn = state
                    .db
                    .lock()
                    .map_err(|_| db_error("database lock poisoned"))?;
                start_fact(&conn, &identity)?;
            }
            let result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| operation(&state)))
                    .unwrap_or_else(|_| {
                        Err(IpcError::new(
                            "AG_CAPABILITY_WORKER_FAILED",
                            "能力执行异常，效果待核对",
                            false,
                            false,
                        ))
                    });
            let refs = result.as_ref().map(references).unwrap_or_default();
            let mut receipt = super::tool_exec::tool_call_receipt(
                identity.clone(),
                true,
                result.is_ok(),
                result.as_ref().err(),
                refs,
                vec![],
            );
            if result.is_err()
                && identity.tool_name == "propose_edit"
                && effect_boundary
                    .as_ref()
                    .is_some_and(|entered| !entered.load(Ordering::Acquire))
            {
                receipt.effect_status = crate::types::ToolCallEffectStatus::NotStarted;
                receipt.next_action = crate::types::ToolCallNextAction::RespondBlocked;
            }
            let conn = state
                .db
                .lock()
                .map_err(|_| db_error("database lock poisoned"))?;
            persist_fact(
                &conn,
                &CapabilityExecutionFactV1 {
                    identity,
                    phase: CapabilityExecutionPhase::Settled,
                    receipt: Some(receipt),
                },
            )?;
            result
        })();
        // Dropped receiver only ends waiting. The settlement above remains queryable.
        let _ = sender.send(result);
    });
    tauri::async_runtime::spawn_blocking(move || loop {
        if cancelled.load(Ordering::Acquire) {
            return Err(IpcError::new(
                "AG_TURN_CANCELLED",
                "等待已取消；已启动能力可能仍在结算，请查询原回合执行事实，勿重放",
                false,
                false,
            ));
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(IpcError::new(
                "AG_CAPABILITY_TIMEOUT",
                "能力等待超时；不保证回滚已开始操作，请查询执行事实，勿重放",
                false,
                false,
            ));
        }
        match receiver.recv_timeout(remaining.min(Duration::from_millis(20))) {
            Ok(result) => return result,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => (),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                return Err(IpcError::new(
                    "AG_CAPABILITY_WORKER_FAILED",
                    "执行端口异常，效果待核对",
                    false,
                    false,
                ))
            }
        }
    })
    .await
    .map_err(|_| {
        IpcError::new(
            "AG_CAPABILITY_WORKER_FAILED",
            "等待线程异常，效果待核对",
            false,
            false,
        )
    })?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ag::{
        history,
        turn_control::{cancel_active_agent_turn, register_active_agent_turn, AgentTurnResumeKind},
    };
    use std::sync::Mutex;

    struct TemporaryWorkspace(std::path::PathBuf);
    impl Drop for TemporaryWorkspace {
        fn drop(&mut self) {
            for name in ["notes.txt", "runtime.sqlite"] {
                let path = self.0.join(name);
                if path.exists() {
                    std::fs::remove_file(path).expect("owned test file cleanup");
                }
            }
            std::fs::remove_dir(&self.0).expect("owned test workspace cleanup");
        }
    }
    fn state() -> (TemporaryWorkspace, AppState) {
        let root = std::env::temp_dir().join(format!("binder-capability-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("notes.txt"), "Original content").unwrap();
        let mut conn = Connection::open(root.join("runtime.sqlite")).unwrap();
        crate::db::migrate(&mut conn).unwrap();
        conn.execute("INSERT INTO workspaces(workspace_id,root_path,is_active,opened_at) VALUES ('ws',?1,1,0)", [root.to_string_lossy().as_ref()]).unwrap();
        history::ensure_session(&conn, "ws", "session", "isolated", "edit", 0, true).unwrap();
        let state = AppState {
            db: Arc::new(Mutex::new(conn)),
            registry: crate::build_registry(),
            assist_terminals: Arc::new(Mutex::new(Default::default())),
            active_agent_turns: Arc::new(Mutex::new(Default::default())),
        };
        add_turn(&state, "session", "turn");
        (TemporaryWorkspace(root), state)
    }
    fn add_turn(state: &AppState, session: &str, turn: &str) {
        let conn = state.db.lock().unwrap();
        history::ensure_session(&conn, "ws", session, "isolated", "edit", 0, true).unwrap();
        history::save_turn(
            &conn,
            "ws",
            session,
            turn,
            None,
            "draft",
            &history::AgentTurnPayload {
                version: 1,
                last_event_seq: 0,
                user_message: history::PersistedChatMessage {
                    id: format!("u-{turn}"),
                    role: "user".into(),
                    content: "isolated execution".into(),
                    parts: None,
                    streaming: None,
                    error: None,
                    segments: None,
                },
                assistant_message: None,
                tool_summaries: vec![],
                diff_refs: vec![],
                failure: None,
                incomplete_reason: None,
            },
        )
        .unwrap();
    }
    fn identity() -> ToolCallReceiptIdentityV1 {
        ToolCallReceiptIdentityV1 {
            workspace_id: "ws".into(),
            session_id: "session".into(),
            turn_id: "turn".into(),
            tool_call_id: "operation".into(),
            tool_name: "propose_edit".into(),
            capability_id: "document_edit".into(),
            grant_id: "owned-grant".into(),
            attempt: 1,
        }
    }

    // covers: BR-AG-STATE-002
    // covers: BR-AG-PERSIST-001
    // covers: BR-DE-DATA-001
    #[test]
    fn proposal_parameter_failure_before_commit_is_not_an_unknown_effect() {
        let (root, state) = state();
        let app = tauri::test::mock_builder()
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let (_lease, cancelled, _, _, _) =
            register_active_agent_turn(&state, "ws", "session", "turn", AgentTurnResumeKind::New)
                .unwrap();
        let read_ctx = crate::ag::blocks::AgentReadContext::new(&root.0, vec![]).unwrap();
        let mut targets = crate::ag::target_set::ExplicitEditTargetSet::default();
        targets.insert_authorized(
            "notes.txt".into(),
            crate::ag::target_set::SOURCE_EXPLICIT_REFERENCE,
            None,
        );
        let error = tauri::async_runtime::block_on(crate::ag::edit_runtime::execute_proposal(
            state.clone(),
            app.handle().clone(),
            serde_json::json!({"path":"notes.txt","operation":"replace_block"}),
            read_ctx,
            None,
            targets,
            identity(),
            cancelled,
        ))
        .unwrap_err();
        assert_eq!(error.code, "AG_TOOL_ARG");
        let conn = state.db.lock().unwrap();
        let records = facts(&conn, "ws", Some(("session", "turn"))).unwrap();
        assert_eq!(records.len(), 2);
        let receipt = records
            .iter()
            .find(|fact| fact.phase == CapabilityExecutionPhase::Settled)
            .and_then(|fact| fact.receipt.as_ref())
            .unwrap();
        assert_eq!(
            receipt.effect_status,
            crate::types::ToolCallEffectStatus::NotStarted
        );
        assert_eq!(receipt.failure.as_ref().unwrap().code, "AG_TOOL_ARG");
        let batches: i64 = conn
            .query_row("SELECT COUNT(*) FROM proposal_batches", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(batches, 0);
        drop(conn);
        add_turn(&state, "other", "new");
        let mut next = identity();
        next.session_id = "other".into();
        next.turn_id = "new".into();
        next.tool_call_id = "new-operation".into();
        start_fact(&state.db.lock().unwrap(), &next).unwrap();
    }

    // covers: BR-AG-STATE-002
    // covers: BR-AG-PERSIST-001
    #[test]
    fn legacy_proposal_argument_error_requires_no_batch_proof_before_new_effect() {
        for batch_exists in [false, true] {
            let (_root, state) = state();
            let original = identity();
            let receipt = super::super::tool_exec::tool_call_receipt(
                original.clone(),
                true,
                false,
                Some(&IpcError::new(
                    "AG_TOOL_ARG",
                    "invalid edit arguments",
                    false,
                    false,
                )),
                vec![],
                vec![],
            );
            assert_eq!(
                receipt.effect_status,
                crate::types::ToolCallEffectStatus::Unknown
            );
            let conn = state.db.lock().unwrap();
            persist_fact(
                &conn,
                &CapabilityExecutionFactV1 {
                    identity: original.clone(),
                    phase: CapabilityExecutionPhase::Started,
                    receipt: None,
                },
            )
            .unwrap();
            persist_fact(
                &conn,
                &CapabilityExecutionFactV1 {
                    identity: original.clone(),
                    phase: CapabilityExecutionPhase::Settled,
                    receipt: Some(receipt.clone()),
                },
            )
            .unwrap();
            if batch_exists {
                conn.execute(
                    "INSERT INTO proposal_batches (proposal_batch_id,workspace_id,turn_id,tool_call_id,grouping_version,created_at) VALUES ('old-batch','ws','turn','operation',1,0)",
                    [],
                )
                .unwrap();
            }
            drop(conn);
            add_turn(&state, "other", "new");
            let mut next = original;
            next.session_id = "other".into();
            next.turn_id = "new".into();
            next.tool_call_id = "new-operation".into();
            let result = start_fact(&state.db.lock().unwrap(), &next);
            if batch_exists {
                assert_eq!(result.unwrap_err().code, "AG_EFFECT_PENDING");
            } else {
                result.unwrap();
                let records =
                    facts(&state.db.lock().unwrap(), "ws", Some(("session", "turn"))).unwrap();
                let settled = records
                    .iter()
                    .find(|fact| fact.phase == CapabilityExecutionPhase::Settled)
                    .unwrap();
                assert_eq!(
                    settled.receipt.as_ref().unwrap().effect_status,
                    crate::types::ToolCallEffectStatus::Unknown,
                    "historical receipt must remain unchanged"
                );
            }
        }
    }

    // covers: BR-AG-STATE-002
    // covers: BR-AG-PERSIST-001
    #[test]
    fn failure_after_proposal_commit_boundary_remains_unknown() {
        let (_root, state) = state();
        let (_lease, cancelled, _, _, _) =
            register_active_agent_turn(&state, "ws", "session", "turn", AgentTurnResumeKind::New)
                .unwrap();
        let boundary = Arc::new(AtomicBool::new(false));
        let worker_boundary = boundary.clone();
        let result = tauri::async_runtime::block_on(execute_with_effect_boundary::<()>(
            state.clone(),
            identity(),
            cancelled,
            Duration::from_secs(1),
            Some(boundary),
            move |_| {
                worker_boundary.store(true, Ordering::Release);
                Err(IpcError::new(
                    "DE_DB_ERROR",
                    "commit outcome unavailable",
                    true,
                    false,
                ))
            },
            |_| vec![],
        ));
        assert_eq!(result.unwrap_err().code, "DE_DB_ERROR");
        let records = facts(&state.db.lock().unwrap(), "ws", Some(("session", "turn"))).unwrap();
        let settled = records
            .iter()
            .find(|fact| fact.phase == CapabilityExecutionPhase::Settled)
            .unwrap();
        assert_eq!(
            settled.receipt.as_ref().unwrap().effect_status,
            crate::types::ToolCallEffectStatus::Unknown
        );
    }

    // covers: BR-AG-STATE-002
    // covers: BR-SYS-STATE-007
    // covers: BR-AG-PERSIST-001
    #[test]
    fn owned_port_cancel_and_timeout_preserve_late_real_proposal_without_replay_or_terminal_rewrite(
    ) {
        for cancel in [true, false] {
            let (root, state) = state();
            let app = tauri::test::mock_builder()
                .build(tauri::test::mock_context(tauri::test::noop_assets()))
                .unwrap();
            let handle = app.handle().clone();
            let (lease, cancelled, _, _, _) = register_active_agent_turn(
                &state,
                "ws",
                "session",
                "turn",
                AgentTurnResumeKind::New,
            )
            .unwrap();
            let read_ctx = crate::ag::blocks::AgentReadContext::new(&root.0, vec![]).unwrap();
            let mut targets = crate::ag::target_set::ExplicitEditTargetSet::default();
            targets.insert_authorized(
                "notes.txt".into(),
                crate::ag::target_set::SOURCE_EXPLICIT_REFERENCE,
                None,
            );
            let (started_tx, started_rx) = std::sync::mpsc::channel();
            let (release_tx, release_rx) = std::sync::mpsc::channel();
            let work = tauri::async_runtime::spawn(execute(
                state.clone(),
                identity(),
                cancelled.clone(),
                Duration::from_millis(if cancel { 2000 } else { 80 }),
                move |state| {
                    started_tx.send(()).unwrap();
                    release_rx.recv_timeout(Duration::from_secs(5)).unwrap();
                    crate::ag::edit_runtime::ag_propose_edit(
                        state,
                        &handle,
                        &serde_json::json!({"path":"notes.txt", "operation":"replace_document", "content":"Updated content"}),
                        &read_ctx,
                        None,
                        &targets,
                        "turn",
                        "operation",
                    )
                },
                |result| {
                    result
                        .projections
                        .iter()
                        .map(|projection| ToolCallReceiptReferenceV1 {
                            kind: crate::types::ToolCallReceiptReferenceKind::PendingDiff,
                            id: projection.diff_id.clone(),
                        })
                        .collect()
                },
            ));
            started_rx.recv_timeout(Duration::from_secs(3)).unwrap();
            let before = Instant::now();
            if cancel {
                cancel_active_agent_turn(&state, "session", "turn").unwrap();
            }
            let error = tauri::async_runtime::block_on(work).unwrap().unwrap_err();
            assert_eq!(
                error.code,
                if cancel {
                    "AG_TURN_CANCELLED"
                } else {
                    "AG_CAPABILITY_TIMEOUT"
                }
            );
            assert!(
                before.elapsed() < Duration::from_millis(500),
                "core must not wait for release"
            );
            let emitter = crate::ag::turn_events::TurnEventEmitter::without_channel(&state, "turn");
            let latch = crate::ag::turn::TerminalLatch::default();
            if cancel {
                crate::ag::turn_events::emit_ag_turn_cancelled(
                    &emitter,
                    &latch,
                    app.handle(),
                    "turn",
                );
            } else {
                crate::ag::turn_events::emit_ag_turn_error(
                    &emitter,
                    &latch,
                    app.handle(),
                    "turn",
                    &error,
                );
            }
            let events_before = crate::ag::turn_events::list_safe_events(
                &state.db.lock().unwrap(),
                "ws",
                "turn",
                0,
            )
            .unwrap();
            assert_eq!(events_before.len(), if cancel { 1 } else { 2 });
            assert_eq!(
                events_before
                    .iter()
                    .filter(|e| matches!(
                        e.event,
                        crate::ipc::AgTurnEventPayload::Done { .. }
                            | crate::ipc::AgTurnEventPayload::Error { .. }
                    ))
                    .count(),
                1
            );
            if !cancel {
                assert!(
                    matches!(&events_before[0].event, crate::ipc::AgTurnEventPayload::Chunk {text} if text.contains("已停止启动后续操作") && text.contains("1 项执行未结算"))
                );
            }
            drop(lease);
            assert_eq!(
                facts(&state.db.lock().unwrap(), "ws", Some(("session", "turn")))
                    .unwrap()
                    .len(),
                1
            );
            assert_eq!(
                register_active_agent_turn(
                    &state,
                    "ws",
                    "session",
                    "new",
                    AgentTurnResumeKind::New
                )
                .err()
                .unwrap()
                .code,
                "AG_EFFECT_PENDING"
            );
            add_turn(&state, "other", "other-turn");
            let (other_lease, other_cancel, _, _, _) = register_active_agent_turn(
                &state,
                "ws",
                "other",
                "other-turn",
                AgentTurnResumeKind::New,
            )
            .unwrap();
            let mut other_identity = identity();
            other_identity.session_id = "other".into();
            other_identity.turn_id = "other-turn".into();
            let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let copy = counter.clone();
            let refused = tauri::async_runtime::block_on(execute(
                state.clone(),
                other_identity,
                other_cancel,
                Duration::from_secs(1),
                move |_| {
                    copy.fetch_add(1, Ordering::AcqRel);
                    Ok(())
                },
                |_| vec![],
            ))
            .unwrap_err();
            assert_eq!(refused.code, "AG_EFFECT_PENDING");
            assert_eq!(counter.load(Ordering::Acquire), 0);
            drop(other_lease);
            release_tx.send(()).unwrap();
            let deadline = Instant::now() + Duration::from_secs(3);
            let completed = loop {
                let facts =
                    facts(&state.db.lock().unwrap(), "ws", Some(("session", "turn"))).unwrap();
                if facts.len() == 2 {
                    break facts;
                }
                assert!(Instant::now() < deadline, "late settlement missing");
                std::thread::sleep(Duration::from_millis(10));
            };
            let receipt = completed
                .iter()
                .find_map(|fact| fact.receipt.as_ref())
                .unwrap();
            assert_eq!(
                receipt.effect_status,
                crate::types::ToolCallEffectStatus::ProposalCreated
            );
            assert_eq!(receipt.references.len(), 1);
            assert_eq!(
                std::fs::read_to_string(root.0.join("notes.txt")).unwrap(),
                "Original content"
            );
            assert_eq!(
                state
                    .db
                    .lock()
                    .unwrap()
                    .query_row("SELECT COUNT(*) FROM pending_diffs", [], |row| row
                        .get::<_, i64>(0))
                    .unwrap(),
                1
            );
            let events_after = crate::ag::turn_events::list_safe_events(
                &state.db.lock().unwrap(),
                "ws",
                "turn",
                0,
            )
            .unwrap();
            assert_eq!(
                serde_json::to_value(events_before).unwrap(),
                serde_json::to_value(events_after).unwrap()
            );
            let mut reopened = Connection::open(root.0.join("runtime.sqlite")).unwrap();
            crate::db::migrate(&mut reopened).unwrap();
            assert_eq!(
                facts(&reopened, "ws", Some(("session", "turn"))).unwrap(),
                completed
            );
            assert!(
                facts(&reopened, "wrong-workspace", Some(("session", "turn")))
                    .unwrap()
                    .is_empty()
            );
            assert!(facts(&reopened, "ws", Some(("wrong-session", "turn")))
                .unwrap()
                .is_empty());
            assert_eq!(
                history::list_turns(&reopened, "ws", "session").unwrap()[0].execution_facts,
                completed
            );
            drop(reopened);
            drop(emitter);
            drop(state);
            drop(app);
            drop(root);
        }
    }

    // covers: BR-AG-STATE-002
    // covers: BR-AG-PERSIST-001
    #[test]
    fn owned_port_duplicate_failure_and_pre_cancel_never_replay_an_effect() {
        let (root, state) = state();
        let (lease, cancel, _, _, _) =
            register_active_agent_turn(&state, "ws", "session", "turn", AgentTurnResumeKind::New)
                .unwrap();
        let error = tauri::async_runtime::block_on(execute::<()>(
            state.clone(),
            identity(),
            cancel.clone(),
            Duration::from_secs(1),
            |_| panic!("explicit test capability panic"),
            |_| vec![],
        ))
        .unwrap_err();
        assert_eq!(error.code, "AG_CAPABILITY_WORKER_FAILED");
        let saved = facts(&state.db.lock().unwrap(), "ws", None).unwrap();
        assert_eq!(saved.len(), 2);
        assert!(delete_settled_session_facts(&state.db.lock().unwrap(), "ws", "session").is_err());
        assert_eq!(facts(&state.db.lock().unwrap(), "ws", None).unwrap(), saved);
        assert_eq!(
            saved
                .iter()
                .find_map(|fact| fact.receipt.as_ref())
                .unwrap()
                .effect_status,
            crate::types::ToolCallEffectStatus::Unknown
        );
        let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let copy = count.clone();
        let duplicate = tauri::async_runtime::block_on(execute(
            state.clone(),
            identity(),
            cancel.clone(),
            Duration::from_secs(1),
            move |_| {
                copy.fetch_add(1, Ordering::AcqRel);
                Ok(())
            },
            |_| vec![],
        ))
        .unwrap_err();
        assert_eq!(duplicate.code, "AG_EFFECT_PENDING");
        assert_eq!(count.load(Ordering::Acquire), 0);
        cancel_active_agent_turn(&state, "session", "turn").unwrap();
        let refused = tauri::async_runtime::block_on(execute(
            state.clone(),
            identity(),
            cancel,
            Duration::from_secs(1),
            |_| Ok(()),
            |_| vec![],
        ))
        .unwrap_err();
        assert_eq!(refused.code, "AG_TURN_CANCELLED");
        assert_eq!(facts(&state.db.lock().unwrap(), "ws", None).unwrap(), saved);
        drop(lease);
        drop(state);
        drop(root);
    }

    // covers: BR-WS-STATE-003
    // covers: BR-AG-STATE-002
    // covers: BR-AG-PERSIST-001
    #[test]
    fn owned_port_resource_scan_does_not_block_cancel_or_timeout_terminal_persistence() {
        for cancel in [true, false] {
            let (root, state) = state();
            let task =
                crate::ag::task::begin_turn(&state.db.lock().unwrap(), "ws", "session", None)
                    .unwrap();
            let app = tauri::test::mock_builder()
                .build(tauri::test::mock_context(tauri::test::noop_assets()))
                .unwrap();
            let (lease, cancelled, _, _, _) = register_active_agent_turn(
                &state,
                "ws",
                "session",
                "turn",
                AgentTurnResumeKind::New,
            )
            .unwrap();
            let (started_tx, started_rx) = std::sync::mpsc::channel();
            let (release_tx, release_rx) = std::sync::mpsc::channel();
            let mut call_identity = identity();
            call_identity.tool_name = "plan_workspace_operation".into();
            call_identity.capability_id = "workspace_operation".into();
            let request = crate::ipc::CommandRequest {
                command: "ws.plan_operation".into(),
                correlation_id: "isolated-slow-scan".into(),
                params: serde_json::json!({"taskRunId":task,"sourceTurnId":"turn","operations":[{"kind":"rename_resource","sourcePath":"notes.txt","targetPath":"renamed.txt","resourceKind":"file"}]}),
            };
            let work = tauri::async_runtime::spawn(execute(
                state.clone(),
                call_identity,
                cancelled,
                Duration::from_millis(if cancel { 3000 } else { 80 }),
                move |state| {
                    crate::workspace::routes::plan_workspace_operation_with_prepare(
                        &request,
                        state,
                        |ws, intents, task, turn, content| {
                            let prepared = crate::workspace::operation::prepare_operation(
                                ws, intents, task, turn, content,
                            )?;
                            started_tx.send(()).unwrap();
                            release_rx.recv_timeout(Duration::from_secs(5)).unwrap();
                            Ok(prepared)
                        },
                    )
                },
                |_| vec![],
            ));
            started_rx.recv_timeout(Duration::from_secs(3)).unwrap();
            let before = Instant::now();
            if cancel {
                cancel_active_agent_turn(&state, "session", "turn").unwrap();
            }
            let error = tauri::async_runtime::block_on(work).unwrap().unwrap_err();
            assert_eq!(
                error.code,
                if cancel {
                    "AG_TURN_CANCELLED"
                } else {
                    "AG_CAPABILITY_TIMEOUT"
                }
            );
            // This is the real terminal emitter while the real prepared scan is still withheld.
            assert!(
                state.db.try_lock().is_ok(),
                "scan must not own shared database lock"
            );
            let emitter = crate::ag::turn_events::TurnEventEmitter::without_channel(&state, "turn");
            let latch = crate::ag::turn::TerminalLatch::default();
            if cancel {
                crate::ag::turn_events::emit_ag_turn_cancelled(
                    &emitter,
                    &latch,
                    app.handle(),
                    "turn",
                );
            } else {
                crate::ag::turn_events::emit_ag_turn_error(
                    &emitter,
                    &latch,
                    app.handle(),
                    "turn",
                    &error,
                );
            }
            assert!(before.elapsed() < Duration::from_millis(500));
            let events = crate::ag::turn_events::list_safe_events(
                &state.db.lock().unwrap(),
                "ws",
                "turn",
                0,
            )
            .unwrap();
            assert_eq!(events.len(), if cancel { 1 } else { 2 });
            assert_eq!(
                events
                    .iter()
                    .filter(|e| matches!(
                        e.event,
                        crate::ipc::AgTurnEventPayload::Done { .. }
                            | crate::ipc::AgTurnEventPayload::Error { .. }
                    ))
                    .count(),
                1
            );
            release_tx.send(()).unwrap();
            let deadline = Instant::now() + Duration::from_secs(3);
            loop {
                let records = facts(&state.db.lock().unwrap(), "ws", None).unwrap();
                if let Some(receipt) = records.iter().find_map(|fact| fact.receipt.as_ref()) {
                    assert_eq!(
                        receipt.execution_status,
                        crate::types::ToolCallExecutionStatus::Failed
                    );
                    assert_eq!(receipt.failure.as_ref().unwrap().code, "AG_TURN_CANCELLED");
                    break;
                }
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(10));
            }
            let conn = state.db.lock().unwrap();
            assert_eq!(
                conn.query_row(
                    "SELECT COUNT(*) FROM workspace_operation_plans",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
                0
            );
            assert_eq!(
                conn.query_row("SELECT COUNT(*) FROM decision_requests", [], |row| row
                    .get::<_, i64>(0))
                    .unwrap(),
                0
            );
            assert_eq!(
                serde_json::to_value(
                    crate::ag::turn_events::list_safe_events(&conn, "ws", "turn", 0).unwrap()
                )
                .unwrap(),
                serde_json::to_value(events).unwrap()
            );
            assert_eq!(
                std::fs::read_to_string(root.0.join("notes.txt")).unwrap(),
                "Original content"
            );
            assert!(!root.0.join("renamed.txt").exists());
            drop(conn);
            drop(emitter);
            drop(lease);
            drop(state);
            drop(app);
            drop(root);
        }
    }
}
