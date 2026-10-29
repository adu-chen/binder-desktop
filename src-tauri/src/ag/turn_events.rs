//! AgentTurn append-only TurnEvent、Channel 安全投影与唯一终态输出。

use super::task_runtime::settle_task_run;
use crate::ipc::{AgTurnEvent, AgTurnEventPayload, AgentFailureDiagnostic, IpcError};
use crate::types::{EvidenceItemV1, EvidenceLedgerV1};
use crate::{ag, sm_events, AppState};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

const TURN_EVENT_PAYLOAD_VERSION: u32 = 1;

// Diagnostic projection only: never print event bodies, paths, arguments, or model text.
// Keep this allowlist independent of safe Channel payloads, which may contain user content.
fn audit_metadata(payload: &Value) -> Value {
    audit_metadata_level(payload, true)
}

fn audit_metadata_level(payload: &Value, include_containers: bool) -> Value {
    let mut metadata = serde_json::Map::new();
    for key in [
        "intentType",
        "ambiguity",
        "sideEffectIntent",
        "nextAction",
        "revision",
        "sourceEventSeq",
        "clarificationResume",
        "decisionResume",
        "roundIndex",
        "provider",
        "model",
        "contextVersion",
        "contextRevision",
        "deduplicatedCount",
        "gateAttempt",
        "remainingToolCalls",
        "maxModelCalls",
        "maxToolCalls",
        "maxCorrections",
        "finalReserve",
        "remainingModelCalls",
        "budgetClosing",
        "protocolDiagnostic",
        "reasonCode",
        "fieldPath",
        "controlSource",
        "canonicalSource",
        "toolIndex",
        "descriptionFields",
        "controlFields",
        "observations",
        "grantId",
        "computedAtSeq",
        "allowedToolCount",
        "toolCallCount",
        "nextRound",
        "intentRevision",
        "toolCallId",
        "toolName",
        "workspaceId",
        "sessionId",
        "turnId",
        "capabilityId",
        "taskRunId",
        "requestId",
        "diffId",
        "status",
        "success",
        "code",
        "retriable",
        "stopReason",
        "attempt",
        "outcome",
        "category",
        "observedAt",
        "rootCauseStatus",
        "executionStatus",
        "completionStatus",
        "effectStatus",
        "assemblyOrdinal",
        "assemblyReason",
        "nextModelCallOrdinal",
        "estimatedTokens",
        "serializedChars",
        "toolSchemaChars",
        "messageFramingChars",
        "totalTokenLimit",
        "continuationReserveTokens",
        "tokenLimit",
        "remainingInputTokens",
        "exceeded",
        "configVersion",
        "estimator",
        "correctionAttempt",
        "prefixKind",
        "rejectionStage",
        "protocolVersion",
        "businessCallCount",
        "taskOutcome",
        "executionFrozen",
        "succeededTools",
        "unsuccessfulTools",
        "unsettledEffects",
        "unknownEffects",
        "replySource",
        "failureCode",
        "finalOnly",
        "providerCompletionSeen",
        "hasExpectedHash",
        "nonInitialPage",
        "repeatedFailure",
        "requestFingerprint",
        "resultChars",
        "resultLimitChars",
        "readView",
        "requestedBlockCount",
        "requestedReadOffset",
        "requestedLimitChars",
        "requestedMaxTokens",
        "roundDeadlineMs",
        "streamIdleDeadlineMs",
        "promptTokens",
        "completionTokens",
        "totalTokens",
        "reasoningTokens",
        "jsonValid",
        "requestedDirectoryLimit",
        "returnedBlockCount",
        "readOffset",
        "readEndOffset",
        "readTotalChars",
        "readHasMore",
        "readComplete",
        "readVersion",
        "historySubmittedCount",
        "historyIncludedCount",
        "historyDroppedCount",
        "historyExcerptedCount",
        "historyOriginalChars",
        "historyIncludedChars",
        "historyAllowanceChars",
        "clientBuildId",
        "gatewayBuildId",
        "gatewayBuildPresent",
        "textUtf8Bytes",
        "textSha256",
        "contentChunkCount",
        "contentBasis",
        "upstreamProofPresent",
        "upstreamProofMatches",
        "argumentsUtf8Bytes",
        "argumentsSha256",
        "jsonErrorLine",
        "jsonErrorColumn",
        "answerStructureWarning",
        "planStatus",
        "operationStatus",
        "saveVerified",
        "readBackVerified",
    ] {
        if let Some(value) = payload.get(key) {
            match value {
                Value::Bool(_) | Value::Number(_) => {
                    metadata.insert(key.into(), value.clone());
                }
                Value::String(text)
                    if text.len() <= 160
                        && (text
                            .bytes()
                            .all(|ch| ch.is_ascii_alphanumeric() || b"_:-.".contains(&ch))
                            || (key == "fieldPath"
                                && matches!(
                                    text.as_str(),
                                    "$control"
                                        | "$arguments"
                                        | "$control.arrays"
                                        | "$batch"
                                        | "calls[]"
                                        | "calls[].toolName"
                                        | "calls[].params"
                                        | "control.intent.targetCandidates[]"
                                        | "control.intent.targetCandidates[].kind"
                                        | "control.intent.targetCandidates[].relPath"
                                        | "control.intent.requiredContext[]"
                                        | "control.intent.confirmationPoints[]"
                                        | "control.route.steps[]"
                                        | "control.findings[]"
                                        | "control.findings[].statement"
                                        | "control.findings[].evidenceIds"
                                        | "control.findings[].evidenceIds[]"
                                        | "control.findings[].uncertainty"
                                ))
                            || (matches!(key, "controlSource" | "canonicalSource")
                                && text
                                    .strip_prefix("tool[")
                                    .and_then(|s| s.strip_suffix(']'))
                                    .is_some_and(|s| {
                                        !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
                                    }))) =>
                {
                    metadata.insert(key.into(), value.clone());
                }
                _ => {}
            }
        }
    }
    if include_containers {
        if let Some(diagnostic) = payload.get("parameterDiagnostic") {
            let field = diagnostic["fieldPath"].as_str();
            let constraint = diagnostic["constraint"].as_str();
            if matches!(
                (field, constraint),
                (Some("initialContent"), Some("non_blank_frozen_content"))
                    | (Some("operations"), Some("single_create_document"))
            ) {
                metadata.insert(
                    "parameterDiagnostic".into(),
                    serde_json::json!({
                        "fieldPath":field,"constraint":constraint
                    }),
                );
            }
        }
        for key in [
            "projection",
            "diagnostic",
            "executionReceipt",
            "identity",
            "measurement",
            "protocolDiagnostic",
            "usage",
        ] {
            if let Some(value) = payload.get(key).filter(|value| value.is_object()) {
                // Read only fixed-depth metadata; never clone or traverse content-bearing objects.
                let mut projected = audit_metadata_level(value, false);
                if key == "executionReceipt" {
                    if let Some(failure) = value.get("failure").filter(|value| value.is_object()) {
                        projected
                            .as_object_mut()
                            .expect("metadata object")
                            .insert("failure".into(), audit_metadata_level(failure, false));
                    }
                }
                metadata.insert(key.into(), projected);
            }
        }
        for key in ["observations", "pendingToolArguments"] {
            if let Some(items) = payload.get(key).and_then(Value::as_array) {
                metadata.insert(
                    key.into(),
                    Value::Array(
                        items
                            .iter()
                            .take(16)
                            .map(|item| audit_metadata_level(item, false))
                            .collect(),
                    ),
                );
                metadata.insert(format!("{key}Count"), Value::from(items.len()));
            }
        }
    }
    for key in [
        "evidenceIds",
        "steps",
        "confirmationPoints",
        "requiredContext",
        "targetCandidates",
        "evidenceItems",
        "readyDocuments",
    ] {
        if let Some(items) = payload.get(key).and_then(Value::as_array) {
            metadata.insert(format!("{key}Count"), Value::from(items.len()));
        }
    }
    if let Some(layers) = payload.get("messageLayers").and_then(Value::as_object) {
        let counts = layers
            .iter()
            .filter(|(_, value)| value.is_u64())
            .filter(|(key, _)| {
                matches!(
                    key.as_str(),
                    "agent_context_facts"
                        | "current_user_input"
                        | "current_document"
                        | "explicit_reference"
                        | "session_history"
                        | "cross_document_history"
                        | "intent_hint"
                        | "runtime_control"
                        | "system_prompt_final"
                        | "model_continuation"
                        | "tool_result"
                        | "continuation_receipt"
                        | "protocol_correction"
                        | "protocol_correction_final_only"
                        | "read_failure_final_only"
                )
            })
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        metadata.insert("messageLayers".into(), Value::Object(counts));
    }
    // Observation entries use the same fixed field-name allowlist as top-level records.
    if let Some(fields) = payload.get("descriptionFields").and_then(Value::as_array) {
        metadata.insert(
            "descriptionFields".into(),
            Value::Array(
                fields
                    .iter()
                    .filter(|v| {
                        matches!(
                            v.as_str(),
                            Some("intent.goal" | "route.summary" | "route.steps")
                        )
                    })
                    .cloned()
                    .collect(),
            ),
        );
    }
    if let Some(fields) = payload.get("controlFields").and_then(Value::as_array) {
        metadata.insert(
            "controlFields".into(),
            Value::Array(
                fields
                    .iter()
                    .filter(|v| {
                        matches!(
                            v.as_str(),
                            Some(
                                "version"
                                    | "nextAction"
                                    | "intent.goal"
                                    | "intent.intentType"
                                    | "intent.targetCandidates"
                                    | "intent.sideEffectIntent"
                                    | "intent.ambiguity"
                                    | "intent.requiredContext"
                                    | "intent.confirmationPoints"
                                    | "route.summary"
                                    | "route.steps"
                                    | "clarificationQuestion"
                                    | "findings"
                            )
                        )
                    })
                    .cloned()
                    .collect(),
            ),
        );
    }
    Value::Object(metadata)
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

fn is_terminal_status(status: &str) -> bool {
    matches!(
        status,
        "completed" | "error" | "cancelled" | "gate_rejected"
    )
}

fn event_db_error(error: impl std::fmt::Display) -> IpcError {
    IpcError::new(
        "AG_TURN_EVENT_DB_ERROR",
        &format!("回合事件持久化失败：{error}"),
        true,
        false,
    )
}

#[derive(Debug)]
pub(super) struct AppendedEvent {
    pub(super) event_id: String,
    pub(super) seq: u64,
    pub(super) occurred_at: i64,
}

fn append_event_record(
    conn: &mut Connection,
    turn_id: &str,
    kind: &str,
    payload_json: &str,
    terminal_status: Option<&str>,
) -> rusqlite::Result<Option<AppendedEvent>> {
    let tx = conn.transaction()?;
    let (workspace_id, session_id, current_status) = tx
        .query_row(
            "SELECT workspace_id, session_id, status FROM agent_turns WHERE turn_id = ?1",
            [turn_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?
        .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    if is_terminal_status(&current_status) {
        if cfg!(debug_assertions) && kind != "chunk" {
            eprintln!(
                "[AG-AG-TURN] append_rejected: {}",
                serde_json::json!({
                    "workspaceId": workspace_id, "sessionId": session_id, "turnId": turn_id,
                    "eventType": kind, "terminalStatus": current_status,
                })
            );
        }
        return Ok(None);
    }
    let next_seq = tx.query_row(
        "SELECT COALESCE(MAX(seq), 0) + 1 FROM agent_turn_events WHERE turn_id = ?1",
        [turn_id],
        |row| row.get::<_, u64>(0),
    )?;
    let occurred_at = now_millis();
    let event_id = format!("turn-event:{turn_id}:{next_seq}");
    tx.execute(
        "INSERT INTO agent_turn_events \
         (event_id, workspace_id, session_id, turn_id, seq, kind, payload_version, payload_json, occurred_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            event_id,
            workspace_id,
            session_id,
            turn_id,
            next_seq,
            kind,
            TURN_EVENT_PAYLOAD_VERSION,
            payload_json,
            occurred_at
        ],
    )?;
    if let Some(status) = terminal_status {
        tx.execute(
            "UPDATE agent_turns SET status = ?4, updated_at = ?5 \
             WHERE turn_id = ?1 AND workspace_id = ?2 AND session_id = ?3",
            params![turn_id, workspace_id, session_id, status, occurred_at],
        )?;
    }
    tx.commit()?;
    // Emit after commit: this line witnesses persistence, not UI rendering or business success.
    // Streaming chunks remain persisted but are excluded from high-frequency console output.
    if cfg!(debug_assertions) && kind != "chunk" {
        let metadata = serde_json::from_str::<Value>(payload_json)
            .map(|payload| audit_metadata(&payload))
            .unwrap_or_else(|_| serde_json::json!({"metadataUnavailable": true}));
        eprintln!(
            "[AG-AG-TURN] committed: {}",
            serde_json::json!({
                "workspaceId": workspace_id, "sessionId": session_id, "turnId": turn_id,
                "eventId": event_id, "seq": next_seq, "occurredAt": occurred_at,
                "eventType": kind, "terminalStatus": terminal_status, "metadata": metadata,
            })
        );
    }
    Ok(Some(AppendedEvent {
        event_id,
        seq: next_seq,
        occurred_at,
    }))
}

/**
 * @GOV
 * codes:    BR-AG-OBS-001-EFFECT-AG-AG-TURN-044, BR-AG-STATE-001-EFFECT-AG-AG-TURN-044, BR-AG-STATE-002-EFFECT-AG-AG-TURN-044, BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-044, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-044
 * type:     EFFECT
 * chain:    AG-TURN, AG-TOOL
 * rules:    BR-AG-OBS-001, BR-AG-STATE-001, BR-AG-STATE-002, BR-AG-PERSIST-001, BR-SYS-STATE-007
 * term_ref: TERM-AG-001, TERM-AG-002, TERM-AG-022, TERM-AG-023
 * boundary: in=CurrentAgentTurnAndSafeOrInternalTurnEventIncludingRuntimeEvidence | out=AppendOnlyMonotonicTurnEventSameIdentityChannelOrAfterSeqProjectionAndEvidenceSourceIdentity | effect=workspaceDb,IPC-ag_turn_stream | does_not=overwriteCommittedEventExposeInternalPayloadAcceptModelEvidenceOrAppendAfterFirstTerminal
 */
pub(super) struct TurnEventEmitter<'a> {
    state: &'a AppState,
    channel: Option<&'a tauri::ipc::Channel<AgTurnEvent>>,
    turn_id: &'a str,
}

impl<'a> TurnEventEmitter<'a> {
    fn failure_summary(
        &self,
        latch: &ag::turn::TerminalLatch,
        error: &IpcError,
    ) -> Result<(), IpcError> {
        if latch.is_set() {
            return Ok(());
        }
        let baseline = {
            let conn = self
                .state
                .db
                .lock()
                .map_err(|_| event_db_error("database lock poisoned"))?;
            let live: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM agent_turns WHERE turn_id=?1 AND status='draft')",
                    [self.turn_id],
                    |row| row.get(0),
                )
                .map_err(event_db_error)?;
            if !live {
                return Ok(());
            }
            let (succeeded, rejected): (u64,u64) = conn.query_row(
                "SELECT COALESCE(SUM(json_extract(payload_json,'$.success')=1),0),COALESCE(SUM(json_extract(payload_json,'$.success')=0),0) FROM agent_turn_events WHERE turn_id=?1 AND kind='tool_result'",
                [self.turn_id], |row| Ok((row.get(0)?,row.get(1)?))).map_err(event_db_error)?;
            let mut stmt = conn.prepare("SELECT plan_id,COALESCE(json_extract(outcome_json,'$.final_status'),status) FROM workspace_operation_plans WHERE source_turn_id=?1 ORDER BY created_at LIMIT 32").map_err(event_db_error)?;
            let operations = stmt.query_map([self.turn_id], |row| Ok(serde_json::json!({"planId":row.get::<_,String>(0)?,"status":row.get::<_,String>(1)?}))).map_err(event_db_error)?
                .collect::<rusqlite::Result<Vec<_>>>().map_err(event_db_error)?;
            let (workspace, session): (String, String) = conn
                .query_row(
                    "SELECT workspace_id,session_id FROM agent_turns WHERE turn_id=?1",
                    [self.turn_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .map_err(event_db_error)?;
            let facts =
                super::capability_runtime::facts(&conn, &workspace, Some((&session, self.turn_id)))
                    .map_err(event_db_error)?;
            let unsettled = facts
                .iter()
                .filter(|fact| {
                    fact.phase == crate::types::CapabilityExecutionPhase::Started
                        && !facts.iter().any(|settled| {
                            settled.phase == crate::types::CapabilityExecutionPhase::Settled
                                && settled.identity == fact.identity
                        })
                })
                .count();
            let unknown = facts
                .iter()
                .filter(|fact| {
                    fact.receipt.as_ref().is_some_and(|receipt| {
                        receipt.effect_status == crate::types::ToolCallEffectStatus::Unknown
                    })
                })
                .count();
            serde_json::json!({"version":"interaction-result-v1","turnId":self.turn_id,"taskOutcome":"failed","executionFrozen":true,
                "succeededTools":succeeded,"unsuccessfulTools":rejected,"operations":operations,"unsettledEffects":unsettled,"unknownEffects":unknown,"replySource":"core","failureCode":error.code})
        };
        self.record_internal("interaction_result", baseline.clone())?;
        let operations = baseline["operations"]
            .as_array()
            .expect("constructed array");
        let operation_text = if operations.is_empty() {
            "未登记资源操作计划；这不证明所有在途操作均无效果。".to_string()
        } else {
            format!(
                "资源操作当前结果：{}。",
                operations
                    .iter()
                    .map(|p| format!(
                        "{}：{}",
                        p["planId"].as_str().unwrap_or("?"),
                        p["status"].as_str().unwrap_or("unknown")
                    ))
                    .collect::<Vec<_>>()
                    .join("；")
            )
        };
        self.emit(latch, AgTurnEventPayload::Chunk { text: format!("\n\n本次任务未完成，已停止启动后续操作。原因：[{}] {}\n已有 {} 项工具成功回执，{} 项失败或受阻回执；成功调用不等于业务目标完成。{}尚有 {} 项执行未结算、{} 项效果未知，不能据此认定没有修改；迟到结果以执行记录为准。系统没有自动重放操作。请先核对或恢复受影响对象，再明确提出后续请求。",
            error.code,error.message,baseline["succeededTools"],baseline["unsuccessfulTools"],operation_text,baseline["unsettledEffects"],baseline["unknownEffects"]) })?;
        Ok(())
    }
    pub(super) fn has_workspace_plan_receipt(&self) -> Result<bool, IpcError> {
        let conn = self
            .state
            .db
            .lock()
            .map_err(|_| IpcError::new("WS_DB_BUSY", "无法核对计划回执", true, false))?;
        conn.query_row("SELECT EXISTS(SELECT 1 FROM agent_turn_events WHERE turn_id=?1 AND kind='tool_result' AND json_extract(payload_json,'$.toolName')='plan_workspace_operation' AND json_extract(payload_json,'$.success')=1)", [self.turn_id], |row| row.get(0)).map_err(event_db_error)
    }
    pub(super) fn execution_policy(
        &self,
    ) -> Result<ag::tool_registry::ExecutionBudgetPolicy, IpcError> {
        let conn = self
            .state
            .db
            .lock()
            .map_err(|_| IpcError::new("WS_DB_BUSY", "无法读取冻结执行策略", true, false))?;
        let saved: Option<String> = conn.query_row(
            "SELECT payload_json FROM agent_turn_events WHERE turn_id=?1 AND kind='execution_budget_frozen' ORDER BY seq LIMIT 1",
            [self.turn_id], |row| row.get(0),
        ).optional().map_err(event_db_error)?;
        let policy = match saved {
            Some(saved) => serde_json::from_str(&saved).map_err(event_db_error)?,
            None => ag::tool_registry::configured_execution_policy(),
        };
        if !policy.valid() {
            return Err(IpcError::new(
                "AG_EXECUTION_POLICY_INVALID",
                "冻结执行策略无效，未执行",
                false,
                false,
            ));
        }
        Ok(policy)
    }

    pub(super) fn freeze_execution_policy(&self) -> Result<(), IpcError> {
        let policy = self.execution_policy()?;
        let exists = {
            let conn = self
                .state
                .db
                .lock()
                .map_err(|_| IpcError::new("WS_DB_BUSY", "无法核验执行策略", true, false))?;
            conn.query_row("SELECT COUNT(*) FROM agent_turn_events WHERE turn_id=?1 AND kind='execution_budget_frozen'", [self.turn_id], |row| row.get::<_, i64>(0)).map_err(event_db_error)? > 0
        };
        if !exists {
            let policy = if self.protocol_attempt_counts()?.0 > 0 {
                ag::tool_registry::ExecutionBudgetPolicy::legacy()
            } else {
                policy
            };
            self.record_internal(
                "execution_budget_frozen",
                serde_json::to_value(policy).map_err(event_db_error)?,
            )?;
        }
        Ok(())
    }

    pub(super) fn frozen_transport_protocol(&self) -> Result<Option<String>, IpcError> {
        let conn = self
            .state
            .db
            .lock()
            .map_err(|_| event_db_error("database lock poisoned"))?;
        let version: Option<String>=conn.query_row("SELECT json_extract(payload_json,'$.protocolVersion') FROM agent_turn_events WHERE turn_id=?1 AND kind='transport_protocol_frozen' ORDER BY seq LIMIT 1",[self.turn_id],|row|row.get(0)).optional().map_err(event_db_error)?;
        if version.as_deref().is_some_and(|version| {
            !matches!(
                version,
                "native-business-tool-v1" | "binder-tool-batch-v1" | "legacy-tool-control-v1"
            )
        }) {
            return Err(IpcError::new(
                "AG_MODEL_PROTOCOL_VERSION_UNSUPPORTED",
                "原回合协议版本不受支持；未自动迁移或重放",
                false,
                false,
            ));
        }
        Ok(version)
    }

    pub(super) fn freeze_transport_protocol(&self) -> Result<(), IpcError> {
        if self.frozen_transport_protocol()?.is_none() {
            let version = if self.protocol_attempt_counts()?.0 == 0 {
                "native-business-tool-v1"
            } else {
                "legacy-tool-control-v1"
            };
            self.record_internal(
                "transport_protocol_frozen",
                serde_json::json!({"protocolVersion":version}),
            )?;
        }
        Ok(())
    }
    /// Recover budgets from actual invocation/rejection facts, including text-only rounds and
    /// attempts before a clarification/decision checkpoint. No prompt or response text is read.
    pub(super) fn protocol_attempt_counts(&self) -> Result<(usize, usize), IpcError> {
        let conn = self
            .state
            .db
            .lock()
            .map_err(|_| IpcError::new("WS_DB_BUSY", "无法读取回合调用预算", true, false))?;
        conn.query_row(
            "SELECT COALESCE(SUM(kind='provider_started'),0), \
             COALESCE(SUM(kind IN ('model_control_rejected','tool_parameter_contract_rejected')),0) \
             FROM agent_turn_events WHERE turn_id=?1",
            [self.turn_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(event_db_error)
    }

    /// Persisted per-category correction counts survive decision and clarification resumes.
    /// A recoverable parameter mistake must not consume the separate control-envelope budget.
    pub(super) fn correction_attempt_breakdown(&self) -> Result<(usize, usize), IpcError> {
        let conn = self
            .state
            .db
            .lock()
            .map_err(|_| IpcError::new("WS_DB_BUSY", "无法读取回合纠错预算", true, false))?;
        conn.query_row(
            "SELECT COALESCE(SUM(kind='model_control_rejected'),0), \
             COALESCE(SUM(kind='tool_parameter_contract_rejected'),0) \
             FROM agent_turn_events WHERE turn_id=?1",
            [self.turn_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(event_db_error)
    }

    pub(super) fn new(
        state: &'a AppState,
        channel: &'a tauri::ipc::Channel<AgTurnEvent>,
        turn_id: &'a str,
    ) -> Self {
        Self {
            state,
            channel: Some(channel),
            turn_id,
        }
    }

    /// 等待态的原 Channel 已关闭；取消仍写同一事件事实，随后由 afterSeq 补流。
    pub(super) fn without_channel(state: &'a AppState, turn_id: &'a str) -> Self {
        Self {
            state,
            channel: None,
            turn_id,
        }
    }

    /**
     * @GOV
     * codes: BR-AG-DATA-005-IO-AG-AG-TURN-056, BR-AG-OBS-001-IO-AG-AG-TURN-056
     * type: IO
     * chain: AG-TURN
     * rules: BR-AG-DATA-005, BR-AG-OBS-001
     * boundary: in=PromptBudgetMeasurementAndHashedSourceProjections | out=OrderedPromptBudgetCheckpoint | effect=DB-agent_turn_events | does_not=storePromptContentOrAuthorizeModelInvocation
     */
    pub(super) fn record_prompt_budget(
        &self,
        reason: &str,
        measurement: &ag::context::PromptBudgetMeasurement,
        sources: Value,
        capacity: ag::prompt_runtime::ModelCapacity,
    ) -> Result<(), IpcError> {
        let mut conn = self
            .state
            .db
            .lock()
            .map_err(|_| IpcError::new("WS_DB_BUSY", "无法记录预算证据", true, false))?;
        let ordinal: i64 = conn.query_row(
            "SELECT COUNT(*) + 1 FROM agent_turn_events WHERE turn_id=?1 AND kind='prompt_budget_checked'",
            [self.turn_id], |row| row.get(0),
        ).map_err(event_db_error)?;
        let next_model_call: i64 = conn.query_row(
            "SELECT COUNT(*) + 1 FROM agent_turn_events WHERE turn_id=?1 AND kind='provider_started'",
            [self.turn_id], |row| row.get(0),
        ).map_err(event_db_error)?;
        let limit = capacity.input_tokens();
        let payload = serde_json::json!({
            "assemblyOrdinal": ordinal, "nextModelCallOrdinal": next_model_call, "assemblyReason": reason,
            "estimator": "ceil_serialized_unicode_chars_div_4",
            "configVersion": format!("prompt-budget-v5-total{}-reserve{}", capacity.total_tokens, capacity.output_tokens),
            "totalTokenLimit": capacity.total_tokens,
            "continuationReserveTokens": capacity.output_tokens,
            "tokenLimit": limit, "measurement": measurement,
            "remainingInputTokens": limit.saturating_sub(measurement.estimated_tokens),
            "exceeded": measurement.estimated_tokens > limit,
            "sourcesNonAdditive": sources,
        });
        let appended = append_event_record(
            &mut conn,
            self.turn_id,
            "prompt_budget_checked",
            &payload.to_string(),
            None,
        )
        .map_err(event_db_error)?;
        if appended.is_none() {
            return Err(IpcError::new(
                "AG_PROMPT_BUDGET_TERMINAL",
                "AgentTurn 已结束，不能继续预算检查",
                false,
                false,
            ));
        }
        Ok(())
    }

    /// 追加后端内部事件。payload 只允许安全元数据，不会投影到 Channel。
    pub(super) fn record_internal(
        &self,
        kind: &str,
        payload: Value,
    ) -> Result<Option<u64>, IpcError> {
        let payload_json = serde_json::to_string(&payload).map_err(event_db_error)?;
        let mut conn = self.state.db.lock().map_err(|_| {
            IpcError::new(
                "WS_DB_BUSY",
                "数据库连接繁忙，无法记录回合事件",
                true,
                false,
            )
        })?;
        append_event_record(&mut conn, self.turn_id, kind, &payload_json, None)
            .map(|appended| appended.map(|event| event.seq))
            .map_err(event_db_error)
    }

    pub(super) fn record_internal_event(
        &self,
        kind: &str,
        payload: Value,
    ) -> Result<Option<AppendedEvent>, IpcError> {
        let payload_json = serde_json::to_string(&payload).map_err(event_db_error)?;
        let mut conn = self.state.db.lock().map_err(|_| {
            IpcError::new(
                "WS_DB_BUSY",
                "数据库连接繁忙，无法记录回合事件",
                true,
                false,
            )
        })?;
        append_event_record(&mut conn, self.turn_id, kind, &payload_json, None)
            .map_err(event_db_error)
    }

    /// 安全事件必须先提交持久化，再使用完全相同的身份与 seq 投影到 Channel。
    pub(super) fn emit(
        &self,
        latch: &ag::turn::TerminalLatch,
        payload: AgTurnEventPayload,
    ) -> Result<bool, IpcError> {
        Ok(self.emit_recorded(latch, payload)?.is_some())
    }

    pub(super) fn emit_recorded(
        &self,
        latch: &ag::turn::TerminalLatch,
        payload: AgTurnEventPayload,
    ) -> Result<Option<AppendedEvent>, IpcError> {
        if latch.is_set() || payload.terminal_status().is_some() {
            return Ok(None);
        }
        let payload_json = serde_json::to_string(&payload).map_err(event_db_error)?;
        let appended = {
            let mut conn = self.state.db.lock().map_err(|_| {
                IpcError::new(
                    "WS_DB_BUSY",
                    "数据库连接繁忙，无法记录回合事件",
                    true,
                    false,
                )
            })?;
            append_event_record(&mut conn, self.turn_id, payload.kind(), &payload_json, None)
                .map_err(event_db_error)?
        };
        let Some(appended) = appended else {
            return Ok(None);
        };
        // Parked callbacks have no live Channel; the committed event is delivered by afterSeq.
        let Some(channel) = self.channel else {
            return Ok(Some(appended));
        };
        if let Err(error) = channel.send(AgTurnEvent {
            event_id: appended.event_id.clone(),
            turn_id: self.turn_id.to_string(),
            seq: appended.seq,
            occurred_at: appended.occurred_at,
            payload_version: TURN_EVENT_PAYLOAD_VERSION,
            event: payload,
        }) {
            // Channel 不是事实源。断线后继续产生真实终态，客户端按 afterSeq 补流。
            eprintln!(
                "[ag-trace] turn_id={} seq={} channel_projection_failed={error}",
                self.turn_id, appended.seq
            );
        }
        Ok(Some(appended))
    }

    fn emit_terminal(
        &self,
        latch: &ag::turn::TerminalLatch,
        terminal_kind: ag::turn::TerminalKind,
        payload: AgTurnEventPayload,
    ) -> Result<bool, IpcError> {
        if latch.is_set() {
            return Ok(false);
        }
        let Some(status) = payload.terminal_status() else {
            return Ok(false);
        };
        let payload_json = serde_json::to_string(&payload).map_err(event_db_error)?;
        let appended = {
            let mut conn = self.state.db.lock().map_err(|_| {
                IpcError::new(
                    "WS_DB_BUSY",
                    "数据库连接繁忙，无法记录回合终态",
                    true,
                    false,
                )
            })?;
            append_event_record(
                &mut conn,
                self.turn_id,
                payload.kind(),
                &payload_json,
                Some(status),
            )
            .map_err(event_db_error)?
        };
        let Some(appended) = appended else {
            return Ok(false);
        };
        if !latch.try_set(terminal_kind) {
            return Ok(false);
        }
        // Channel 可能已断开；事件与 status 已原子提交，可由 afterSeq 补流，因此不回滚事实。
        if let Some(channel) = self.channel {
            let _ = channel.send(AgTurnEvent {
                event_id: appended.event_id,
                turn_id: self.turn_id.to_string(),
                seq: appended.seq,
                occurred_at: appended.occurred_at,
                payload_version: TURN_EVENT_PAYLOAD_VERSION,
                event: payload,
            });
        }
        Ok(true)
    }
}

/**
 * @GOV
 * codes:    BR-AG-PERSIST-001-QUERY-AG-AG-TURN-045, BR-SYS-STATE-007-QUERY-AG-AG-TURN-045
 * type:     QUERY
 * chain:    AG-TURN
 * rules:    BR-AG-PERSIST-001, BR-SYS-STATE-007
 * term_ref: TERM-AG-001, TERM-AG-022
 * boundary: in=WorkspaceScopedAgentTurnIdentityAndAfterSeq | out=AscendingSafeAgTurnEventsWithOriginalSeqGaps | does_not=exposeInternalContextGateProviderEventsOrCrossWorkspaceTurn
 */
pub fn list_safe_events(
    conn: &Connection,
    workspace_id: &str,
    turn_id: &str,
    after_seq: u64,
) -> rusqlite::Result<Vec<AgTurnEvent>> {
    let owns_turn = conn
        .query_row(
            "SELECT 1 FROM agent_turns WHERE workspace_id = ?1 AND turn_id = ?2",
            params![workspace_id, turn_id],
            |_| Ok(()),
        )
        .optional()?;
    if owns_turn.is_none() {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    let mut statement = conn.prepare(
        "SELECT event_id, turn_id, seq, occurred_at, payload_version, payload_json \
         FROM agent_turn_events \
         WHERE workspace_id = ?1 AND turn_id = ?2 AND seq > ?3 AND kind IN ( \
           'chunk','intent_updated','route_updated','tool_request','tool_result','project_report','diff_proposed', \
           'step_progress','decision_request_proposed','decision_request_resolved', \
           'clarification_required','done','error' \
         ) ORDER BY seq ASC",
    )?;
    let rows = statement.query_map(params![workspace_id, turn_id, after_seq], |row| {
        let payload_json: String = row.get(5)?;
        let event = serde_json::from_str::<AgTurnEventPayload>(&payload_json).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                5,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?;
        Ok(AgTurnEvent {
            event_id: row.get(0)?,
            turn_id: row.get(1)?,
            seq: row.get(2)?,
            occurred_at: row.get(3)?,
            payload_version: row.get(4)?,
            event,
        })
    })?;
    rows.collect()
}

/**
 * @GOV
 * codes:    BR-AG-OBS-001-QUERY-AG-AG-TURN-048, BR-AG-PERSIST-001-QUERY-AG-AG-TURN-048, BR-SYS-DATA-008-QUERY-AG-AG-TURN-048
 * type:     QUERY
 * chain:    AG-TURN, AG-TOOL
 * rules:    BR-AG-OBS-001, BR-AG-PERSIST-001, BR-SYS-DATA-008
 * term_ref: TERM-AG-021, TERM-AG-023
 * boundary: in=WorkspaceScopedTurnAndAppendOnlyEvidenceEvents | out=EvidenceLedgerV1(validSameTurnSourceEventOrder) | does_not=acceptCrossWorkspaceFailedReadOrModelSuppliedEvidence
 */
pub(super) fn rebuild_evidence_ledger(
    conn: &Connection,
    workspace_id: &str,
    turn_id: &str,
) -> Result<EvidenceLedgerV1, IpcError> {
    let owns_turn = conn
        .query_row(
            "SELECT 1 FROM agent_turns WHERE workspace_id = ?1 AND turn_id = ?2",
            params![workspace_id, turn_id],
            |_| Ok(()),
        )
        .optional()
        .map_err(event_db_error)?;
    if owns_turn.is_none() {
        return Err(IpcError::new(
            "AG_EVIDENCE_TURN_SCOPE_INVALID",
            "证据账本不属于当前 workspace/turn",
            false,
            false,
        ));
    }
    let mut statement = conn
        .prepare(
            "SELECT payload_json FROM agent_turn_events \
             WHERE workspace_id = ?1 AND turn_id = ?2 AND kind = 'evidence_recorded' \
             ORDER BY seq ASC",
        )
        .map_err(event_db_error)?;
    let rows = statement
        .query_map(params![workspace_id, turn_id], |row| {
            row.get::<_, String>(0)
        })
        .map_err(event_db_error)?;
    let mut items = BTreeMap::<String, EvidenceItemV1>::new();
    for row in rows {
        let payload_json = row.map_err(event_db_error)?;
        let payload: Value = serde_json::from_str(&payload_json).map_err(event_db_error)?;
        let recorded = serde_json::from_value::<Vec<EvidenceItemV1>>(
            payload
                .get("evidenceItems")
                .cloned()
                .unwrap_or(Value::Array(Vec::new())),
        )
        .map_err(event_db_error)?;
        for item in recorded {
            if item.workspace_id != workspace_id || item.turn_id != turn_id || item.seq == 0 {
                continue;
            }
            let source_valid = conn
                .query_row(
                    "SELECT 1 FROM agent_turn_events \
                     WHERE workspace_id = ?1 AND turn_id = ?2 AND event_id = ?3 AND seq = ?4",
                    params![workspace_id, turn_id, item.source_event_id, item.seq],
                    |_| Ok(()),
                )
                .optional()
                .map_err(event_db_error)?
                .is_some();
            if source_valid {
                items.entry(item.evidence_id.clone()).or_insert(item);
            }
        }
    }
    let mut evidence_items = items.into_values().collect::<Vec<_>>();
    evidence_items.sort_by(|left, right| {
        (left.seq, left.evidence_id.as_str()).cmp(&(right.seq, right.evidence_id.as_str()))
    });
    let through_seq = evidence_items
        .iter()
        .map(|item| item.seq)
        .max()
        .unwrap_or(0);
    Ok(EvidenceLedgerV1 {
        turn_id: turn_id.to_string(),
        through_seq,
        evidence_items,
    })
}

pub(super) fn sm_error() -> IpcError {
    IpcError::new("AG_SM_INVALID", "SM-AG-01 非法转移", false, false)
}

/// 广播 SM-AG-01 回合机转移（turn_id 为实例，A-CORE-X-L-04 Task 2-E）。
pub(super) fn emit_ag01<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    turn_id: &str,
    st: ag::turn::TurnState,
) {
    sm_events::emit_sm_state(
        app,
        sm_events::SmStatePayload::new(
            "SM-AG-01",
            turn_id,
            &format!("{st:?}"),
            ag::turn::is_terminal(st),
        ),
    );
}

/// 广播 SM-AG-02 工具循环机转移（turn_id 为实例，A-CORE-X-L-04 Task 2-AG2）。
pub(super) fn emit_ag02<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    turn_id: &str,
    st: ag::tool::ToolLoopState,
) {
    sm_events::emit_sm_state(
        app,
        sm_events::SmStatePayload::new(
            "SM-AG-02",
            turn_id,
            &format!("{st:?}"),
            ag::tool::is_terminal(st),
        ),
    );
}

fn terminal_transition(
    source: ag::turn::TurnState,
    event: ag::turn::TurnEvent,
    expected: ag::turn::TurnState,
) -> Option<ag::turn::TurnState> {
    let next = ag::turn::transition(source, event)?;
    (next == expected && ag::turn::is_terminal(next)).then_some(next)
}

pub(super) fn emit_ag_turn_done<R: tauri::Runtime>(
    emitter: &TurnEventEmitter<'_>,
    latch: &ag::turn::TerminalLatch,
    app: &tauri::AppHandle<R>,
    turn_id: &str,
    stop_reason: &str,
    source: ag::turn::TurnState,
) -> bool {
    use ag::turn::{transition, TurnEvent, TurnState};
    let finalizing = match source {
        TurnState::Streaming => {
            let Some(finalizing) = transition(TurnState::Streaming, TurnEvent::ModelFinal) else {
                return false;
            };
            finalizing
        }
        TurnState::Finalizing => TurnState::Finalizing,
        _ => return false,
    };
    let Some(completed) = transition(finalizing, TurnEvent::FinalizeOk) else {
        return false;
    };
    let emitted = emitter
        .emit_terminal(
            latch,
            ag::turn::TerminalKind::Completed,
            AgTurnEventPayload::Done {
                stop_reason: stop_reason.to_string(),
            },
        )
        .unwrap_or_else(|error| {
            eprintln!(
                "[ag-trace] turn_id={turn_id} terminal_persist_failed={}",
                error.message
            );
            false
        });
    if !emitted {
        return false;
    }
    eprintln!("[ag-trace] turn_id={turn_id} terminal=completed stop_reason={stop_reason}");
    if source == TurnState::Streaming {
        emit_ag01(app, turn_id, finalizing);
    }
    emit_ag01(app, turn_id, completed);
    true
}

pub(super) fn emit_ag_turn_cancelled<R: tauri::Runtime>(
    emitter: &TurnEventEmitter<'_>,
    latch: &ag::turn::TerminalLatch,
    app: &tauri::AppHandle<R>,
    turn_id: &str,
) -> bool {
    emit_ag_turn_cancelled_from(emitter, latch, app, turn_id, ag::turn::TurnState::Streaming)
}

pub(super) fn emit_ag_turn_cancelled_from<R: tauri::Runtime>(
    emitter: &TurnEventEmitter<'_>,
    latch: &ag::turn::TerminalLatch,
    app: &tauri::AppHandle<R>,
    turn_id: &str,
    source: ag::turn::TurnState,
) -> bool {
    let Some(cancelled) = terminal_transition(
        source,
        ag::turn::TurnEvent::Cancel,
        ag::turn::TurnState::Cancelled,
    ) else {
        return false;
    };
    let emitted = emitter
        .emit_terminal(
            latch,
            ag::turn::TerminalKind::Cancelled,
            AgTurnEventPayload::Done {
                stop_reason: "cancelled".to_string(),
            },
        )
        .unwrap_or_else(|error| {
            eprintln!(
                "[ag-trace] turn_id={turn_id} terminal_persist_failed={}",
                error.message
            );
            false
        });
    if !emitted {
        return false;
    }
    eprintln!("[ag-trace] turn_id={turn_id} terminal=cancelled");
    emit_ag01(app, turn_id, cancelled);
    true
}

/// 非终态等待已无活动 Channel；取消在数据库中原子锁存，再由安全补流返回同一事件。
pub(super) fn persist_parked_ag_turn_cancelled<R: tauri::Runtime>(
    state: &AppState,
    app: &tauri::AppHandle<R>,
    turn_id: &str,
    source: ag::turn::TurnState,
) -> Result<bool, IpcError> {
    let Some(cancelled) = terminal_transition(
        source,
        ag::turn::TurnEvent::Cancel,
        ag::turn::TurnState::Cancelled,
    ) else {
        return Err(sm_error());
    };
    let emitter = TurnEventEmitter::without_channel(state, turn_id);
    let latch = ag::turn::TerminalLatch::default();
    let emitted = emitter.emit_terminal(
        &latch,
        ag::turn::TerminalKind::Cancelled,
        AgTurnEventPayload::Done {
            stop_reason: "cancelled".to_string(),
        },
    )?;
    if emitted {
        emit_ag01(app, turn_id, cancelled);
    }
    Ok(emitted)
}

/** @GOV
 * codes: BR-AG-STATE-001-EFFECT-AG-AG-TURN-069, BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-069
 * type: EFFECT
 * chain: AG-TURN, WS-OPERATION
 * rules: BR-AG-STATE-001, BR-AG-PERSIST-001
 * term_ref: TERM-AG-001, TERM-WS-004
 * boundary: in=FailedWorkspaceOperationPlanAndOriginalAgentTurnIdentity | out=PersistedFailureSummaryAndSingleErrorTerminal | does_not=ReplayResourceOperationOrOverwriteExistingTerminal
 */
pub(crate) fn finish_failed_operation(
    state: &AppState,
    plan_id: &str,
    error: &IpcError,
) -> Result<(), IpcError> {
    finish_parked_operation(state, plan_id, error, false)
}

pub(crate) fn finish_rejected_operation(state: &AppState, plan_id: &str) -> Result<(), IpcError> {
    finish_parked_operation(
        state,
        plan_id,
        &IpcError::new(
            "WS_INITIAL_CONTENT_REJECTED",
            "初始内容已拒绝；同组后续操作不会执行，资源效果请核对逐项回执",
            false,
            false,
        ),
        true,
    )
}

fn finish_parked_operation(
    state: &AppState,
    plan_id: &str,
    error: &IpcError,
    rejected: bool,
) -> Result<(), IpcError> {
    let owner = {
        let conn = state
            .db
            .lock()
            .map_err(|_| event_db_error("database lock poisoned"))?;
        conn.query_row("SELECT t.turn_id,t.session_id,p.task_run_id FROM workspace_operation_plans p JOIN agent_turns t ON t.turn_id=p.source_turn_id AND t.task_run_id=p.task_run_id AND t.workspace_id=p.workspace_id WHERE p.plan_id=?1 AND t.status='draft'", [plan_id],
            |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?))).optional().map_err(event_db_error)?
    };
    let Some((turn, session, run)) = owner else {
        return Ok(());
    };
    terminal_transition(
        ag::turn::TurnState::AwaitingDecision,
        if rejected {
            ag::turn::TurnEvent::Cancel
        } else {
            ag::turn::TurnEvent::OperationFailed
        },
        if rejected {
            ag::turn::TurnState::Cancelled
        } else {
            ag::turn::TurnState::TurnError
        },
    )
    .ok_or_else(sm_error)?;
    let emitter = TurnEventEmitter::without_channel(state, &turn);
    let latch = ag::turn::TerminalLatch::default();
    emitter.record_internal("operation_post_processing_failed", serde_json::json!({"planId":plan_id,"code":error.code,"observedAt":"operation_post_processing"}))?;
    if rejected {
        emitter.emit(&latch,AgTurnEventPayload::Chunk{text:"\n\n已拒绝初始内容，当前任务已取消，同组后续资源操作不会执行。只补偿基线仍匹配的本次创建；已有文件及实际效果请以逐项执行回执为准。系统没有自动重放。".into()})?;
    } else {
        emitter.failure_summary(&latch, error)?;
    }
    {
        let conn = state
            .db
            .lock()
            .map_err(|_| event_db_error("database lock poisoned"))?;
        let settled_status = ag::task::settle_turn(
            &conn,
            &run,
            if rejected {
                ag::task::TurnOutcome::Cancelled
            } else {
                ag::task::TurnOutcome::Failed
            },
        )
        .map_err(event_db_error)?;
        ag::task::record_projection_fact(
            &conn,
            &turn,
            "task_status_observed",
            serde_json::json!({"taskRunId":run,"status":settled_status}),
        )
        .map_err(event_db_error)?;
    }
    let terminal = if rejected {
        AgTurnEventPayload::Done {
            stop_reason: "cancelled".into(),
        }
    } else {
        AgTurnEventPayload::Error {
            code: error.code.clone(),
            message: error.message.clone(),
            retriable: false,
            diagnostic: Some(AgentFailureDiagnostic::observed(
                "operation_post_processing",
                &error.code,
            )),
        }
    };
    emitter.emit_terminal(
        &latch,
        if rejected {
            ag::turn::TerminalKind::Cancelled
        } else {
            ag::turn::TerminalKind::Error
        },
        terminal,
    )?;
    super::turn_control::release_failed_parked_turn(state, &session, &turn)
}

pub(super) fn emit_ag_turn_denied<R: tauri::Runtime>(
    emitter: &TurnEventEmitter<'_>,
    latch: &ag::turn::TerminalLatch,
    app: &tauri::AppHandle<R>,
    turn_id: &str,
    error: &IpcError,
) -> bool {
    let Some(denied) = terminal_transition(
        ag::turn::TurnState::AwaitingGate,
        ag::turn::TurnEvent::GateDenied,
        ag::turn::TurnState::Denied,
    ) else {
        return false;
    };
    let emitted = emitter
        .emit_terminal(
            latch,
            ag::turn::TerminalKind::Denied,
            AgTurnEventPayload::Error {
                code: error.code.clone(),
                message: error.message.clone(),
                retriable: error.retriable,
                diagnostic: Some(AgentFailureDiagnostic::observed("gate", &error.code)),
            },
        )
        .unwrap_or_else(|persist_error| {
            eprintln!(
                "[ag-trace] turn_id={turn_id} terminal_persist_failed={}",
                persist_error.message
            );
            false
        });
    if !emitted {
        return false;
    }
    eprintln!(
        "[ag-trace] turn_id={} terminal=denied code={} retriable={}",
        turn_id, error.code, error.retriable
    );
    emit_ag01(app, turn_id, denied);
    true
}

pub(super) fn emit_ag_turn_error<R: tauri::Runtime>(
    emitter: &TurnEventEmitter<'_>,
    latch: &ag::turn::TerminalLatch,
    app: &tauri::AppHandle<R>,
    turn_id: &str,
    error: &IpcError,
) -> bool {
    emit_ag_turn_error_from(
        emitter,
        latch,
        app,
        turn_id,
        error,
        ag::turn::TurnState::Streaming,
    )
}

pub(super) fn emit_ag_turn_error_from<R: tauri::Runtime>(
    emitter: &TurnEventEmitter<'_>,
    latch: &ag::turn::TerminalLatch,
    app: &tauri::AppHandle<R>,
    turn_id: &str,
    error: &IpcError,
    source: ag::turn::TurnState,
) -> bool {
    let event = match source {
        ag::turn::TurnState::Assembling => ag::turn::TurnEvent::AssembleFail,
        ag::turn::TurnState::AwaitingGate => ag::turn::TurnEvent::GateFailed,
        ag::turn::TurnState::Streaming => ag::turn::TurnEvent::StreamFail,
        ag::turn::TurnState::Finalizing => ag::turn::TurnEvent::FinalizeFail,
        _ => return false,
    };
    let Some(turn_error) = terminal_transition(source, event, ag::turn::TurnState::TurnError)
    else {
        return false;
    };
    if let Err(summary_error) = emitter.failure_summary(latch, error) {
        eprintln!(
            "[ag-trace] turn_id={turn_id} failure_summary_unavailable={}",
            summary_error.code
        );
    }
    let emitted = emitter
        .emit_terminal(
            latch,
            ag::turn::TerminalKind::Error,
            AgTurnEventPayload::Error {
                code: error.code.clone(),
                message: error.message.clone(),
                retriable: error.retriable,
                diagnostic: Some(AgentFailureDiagnostic::observed(
                    match source {
                        ag::turn::TurnState::Assembling => "context_assembly",
                        ag::turn::TurnState::AwaitingGate => "gate",
                        ag::turn::TurnState::Finalizing => "finalizing",
                        _ => "runtime",
                    },
                    &error.code,
                )),
            },
        )
        .unwrap_or_else(|persist_error| {
            eprintln!(
                "[ag-trace] turn_id={turn_id} terminal_persist_failed={}",
                persist_error.message
            );
            false
        });
    if !emitted {
        return false;
    }
    eprintln!(
        "[ag-trace] turn_id={} terminal=error code={} retriable={}",
        turn_id, error.code, error.retriable
    );
    emit_ag01(app, turn_id, turn_error);
    true
}

/// AgentTurn 的统一失败出口：存在 TaskRun 时先尽力收束持久态，再发送唯一回合终态。
pub(super) fn settle_and_emit_ag_turn_error<R: tauri::Runtime>(
    state: &AppState,
    task_run_id: Option<&str>,
    emitter: &TurnEventEmitter<'_>,
    latch: &ag::turn::TerminalLatch,
    app: &tauri::AppHandle<R>,
    turn_id: &str,
    error: &IpcError,
) {
    settle_and_emit_ag_turn_error_from(
        state,
        task_run_id,
        emitter,
        latch,
        app,
        turn_id,
        error,
        ag::turn::TurnState::Streaming,
    );
}

pub(super) fn settle_and_emit_ag_turn_error_from<R: tauri::Runtime>(
    state: &AppState,
    task_run_id: Option<&str>,
    emitter: &TurnEventEmitter<'_>,
    latch: &ag::turn::TerminalLatch,
    app: &tauri::AppHandle<R>,
    turn_id: &str,
    error: &IpcError,
    source: ag::turn::TurnState,
) {
    if let (Some(task_run_id), Ok(conn)) = (task_run_id, state.db.lock()) {
        settle_task_run(&conn, task_run_id, turn_id, ag::task::TurnOutcome::Failed);
    }
    emit_ag_turn_error_from(emitter, latch, app, turn_id, error, source);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_registry, db};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    fn fresh_turn() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        db::migrate(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO workspaces (workspace_id, root_path, is_active, opened_at) \
             VALUES ('ws1','/w',1,0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO agent_sessions \
             (session_id,workspace_id,title,mode,archived,open_index,is_active,created_at,updated_at) \
             VALUES ('s1','ws1','s','edit',0,0,1,0,0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO agent_turn_payloads \
             (payload_ref,workspace_id,payload_json,payload_size,created_at) \
             VALUES ('p1','ws1','{}',2,0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO agent_turns \
             (turn_id,workspace_id,payload_ref,created_at,session_id,status,updated_at) \
             VALUES ('t1','ws1','p1',0,'s1','draft',0)",
            [],
        )
        .unwrap();
        conn
    }

    // covers: BR-AG-OBS-001
    #[test]
    fn protocol_diagnostics_and_reconciliation_log_only_fixed_fields() {
        let payload = serde_json::json!({"protocolDiagnostic":{"reasonCode":"schema_enum_invalid","fieldPath":"$control","controlSource":"tool[1]","toolIndex":1,"raw":"PRIVATE_VALUE"},
            "observations":[{"canonicalSource":"text","controlSource":"tool[2]","toolIndex":2,"descriptionFields":["intent.goal","route.summary","PRIVATE_KEY"],"controlFields":["intent.requiredContext","intent.sideEffectIntent","PRIVATE_KEY"],"content":"PRIVATE_BODY"}]});
        let safe = audit_metadata(&payload);
        assert_eq!(safe["protocolDiagnostic"]["fieldPath"], "$control");
        assert_eq!(safe["protocolDiagnostic"]["controlSource"], "tool[1]");
        for path in [
            "$batch",
            "calls[]",
            "calls[].params",
            "control.intent.targetCandidates[].kind",
            "control.route.steps[]",
        ] {
            let safe = audit_metadata(
                &serde_json::json!({"protocolDiagnostic":{"fieldPath":path,"reasonCode":"schema_type_invalid","raw":"PRIVATE"}}),
            );
            assert_eq!(safe["protocolDiagnostic"]["fieldPath"], path);
            assert!(!safe.to_string().contains("PRIVATE"));
        }
        let unsafe_path = audit_metadata(
            &serde_json::json!({"protocolDiagnostic":{"fieldPath":"calls[].PRIVATE_KEY"}}),
        );
        assert!(unsafe_path["protocolDiagnostic"].get("fieldPath").is_none());
        assert_eq!(
            safe["observations"][0]["descriptionFields"],
            serde_json::json!(["intent.goal", "route.summary"])
        );
        assert_eq!(
            safe["observations"][0]["controlFields"],
            serde_json::json!(["intent.requiredContext", "intent.sideEffectIntent"])
        );
        assert!(!safe.to_string().contains("PRIVATE"));
        let parameter = audit_metadata(&serde_json::json!({"parameterDiagnostic":{
            "fieldPath":"initialContent","constraint":"non_blank_frozen_content","raw":"PRIVATE_BODY"
        }}));
        assert_eq!(
            parameter["parameterDiagnostic"]["fieldPath"],
            "initialContent"
        );
        assert_eq!(
            parameter["parameterDiagnostic"]["constraint"],
            "non_blank_frozen_content"
        );
        assert!(!parameter.to_string().contains("PRIVATE"));
        assert!(audit_metadata(&serde_json::json!({"parameterDiagnostic":{
            "fieldPath":"initialContent","constraint":"PRIVATE"
        }}))
        .get("parameterDiagnostic")
        .is_none());
    }

    // covers: BR-AG-STATE-001, BR-AG-STATE-002, BR-SYS-STATE-007
    #[test]
    fn execution_policy_and_attempts_survive_recreation_and_configuration_change() {
        let state = AppState {
            db: Arc::new(Mutex::new(fresh_turn())),
            registry: build_registry(),
            assist_terminals: Arc::new(Mutex::new(HashMap::new())),
            active_agent_turns: Arc::new(Mutex::new(HashMap::new())),
        };
        let policy = ag::tool_registry::ExecutionBudgetPolicy {
            max_model_calls: 4,
            max_tool_calls: 3,
            max_corrections: 1,
            final_reserve: 1,
        };
        tauri::async_runtime::block_on(ag::tool_registry::with_verification_execution_policy(
            policy,
            async {
                let emitter = TurnEventEmitter::without_channel(&state, "t1");
                emitter.freeze_execution_policy().unwrap();
                emitter.freeze_transport_protocol().unwrap();
                emitter
                    .record_internal("provider_started", serde_json::json!({"roundIndex":1}))
                    .unwrap();
                emitter
                    .record_internal(
                        "model_control_rejected",
                        serde_json::json!({"code":"AG_MODEL_CONTROL_INVALID"}),
                    )
                    .unwrap();
            },
        ));
        let emitter = TurnEventEmitter::without_channel(&state, "t1");
        assert_eq!(
            ag::tool_registry::configured_execution_policy(),
            Default::default()
        );
        emitter.freeze_execution_policy().unwrap();
        emitter.freeze_transport_protocol().unwrap();
        assert_eq!(
            emitter.frozen_transport_protocol().unwrap().as_deref(),
            Some("native-business-tool-v1")
        );
        assert_eq!(emitter.execution_policy().unwrap(), policy);
        assert_eq!(emitter.protocol_attempt_counts().unwrap(), (1, 1));
        assert_eq!(
            state
                .db
                .lock()
                .unwrap()
                .query_row(
                    "SELECT COUNT(*) FROM agent_turn_events WHERE kind='transport_protocol_frozen'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );
        assert_eq!(
            state
                .db
                .lock()
                .unwrap()
                .query_row(
                    "SELECT COUNT(*) FROM agent_turn_events WHERE kind='execution_budget_frozen'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );
        let legacy = AppState {
            db: Arc::new(Mutex::new(fresh_turn())),
            registry: build_registry(),
            assist_terminals: Arc::new(Mutex::new(HashMap::new())),
            active_agent_turns: Arc::new(Mutex::new(HashMap::new())),
        };
        let old = TurnEventEmitter::without_channel(&legacy, "t1");
        old.record_internal("provider_started", serde_json::json!({"roundIndex":1}))
            .unwrap();
        old.freeze_transport_protocol().unwrap();
        assert_eq!(
            old.frozen_transport_protocol().unwrap().as_deref(),
            Some("legacy-tool-control-v1")
        );
        let unsupported = AppState {
            db: Arc::new(Mutex::new(fresh_turn())),
            registry: build_registry(),
            assist_terminals: Arc::new(Mutex::new(HashMap::new())),
            active_agent_turns: Arc::new(Mutex::new(HashMap::new())),
        };
        let unknown = TurnEventEmitter::without_channel(&unsupported, "t1");
        unknown
            .record_internal(
                "transport_protocol_frozen",
                serde_json::json!({"protocolVersion":"unsupported"}),
            )
            .unwrap();
        assert_eq!(
            unknown.freeze_transport_protocol().unwrap_err().code,
            "AG_MODEL_PROTOCOL_VERSION_UNSUPPORTED"
        );
    }

    // covers: BR-AG-STATE-001, BR-AG-DATA-004
    #[test]
    fn historical_batch_transport_is_not_migrated_when_turn_resumes() {
        let state = AppState {
            db: Arc::new(Mutex::new(fresh_turn())),
            registry: build_registry(),
            assist_terminals: Arc::new(Mutex::new(HashMap::new())),
            active_agent_turns: Arc::new(Mutex::new(HashMap::new())),
        };
        let emitter = TurnEventEmitter::without_channel(&state, "t1");
        emitter
            .record_internal(
                "transport_protocol_frozen",
                serde_json::json!({"protocolVersion": ag::model_output::BATCH_TOOL_PROTOCOL}),
            )
            .unwrap();
        emitter.freeze_transport_protocol().unwrap();
        assert_eq!(
            emitter.frozen_transport_protocol().unwrap().as_deref(),
            Some(ag::model_output::BATCH_TOOL_PROTOCOL)
        );
        let frozen_count: i64 = state
            .db
            .lock()
            .unwrap()
            .query_row(
                "SELECT count(*) FROM agent_turn_events WHERE turn_id='t1' AND kind='transport_protocol_frozen'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(frozen_count, 1);
    }

    // covers: BR-AG-STATE-001, BR-SYS-STATE-007, BR-AG-PERSIST-001
    // covers: BR-AG-DATA-005
    #[test]
    fn budget_checkpoints_keep_stage_and_ordinals_across_emitter_recreation() {
        let state = AppState {
            db: std::sync::Arc::new(Mutex::new(fresh_turn())),
            registry: build_registry(),
            assist_terminals: std::sync::Arc::new(Mutex::new(HashMap::new())),
            active_agent_turns: Arc::new(Mutex::new(HashMap::new())),
        };
        let measurement = ag::context::measure_prompt_budget(
            &[serde_json::json!({"role":"user","content":"PRIVATE"})],
            &serde_json::json!([]),
        );
        for (index, reason) in [
            "initial",
            "tool_result",
            "decision_resume",
            "preview_resume",
        ]
        .into_iter()
        .enumerate()
        {
            let emitter = TurnEventEmitter::without_channel(&state, "t1");
            emitter
                .record_prompt_budget(
                    reason,
                    &measurement,
                    serde_json::json!([]),
                    ag::prompt_runtime::model_capacity("deepseek", "deepseek-chat"),
                )
                .unwrap();
            let payload: String = state.db.lock().unwrap().query_row(
                "SELECT payload_json FROM agent_turn_events WHERE kind='prompt_budget_checked' ORDER BY seq DESC LIMIT 1", [], |r| r.get(0)).unwrap();
            assert!(!payload.contains("PRIVATE"));
            let payload: Value = serde_json::from_str(&payload).unwrap();
            assert_eq!(payload["assemblyReason"], reason);
            assert_eq!(payload["assemblyOrdinal"], index + 1);
            assert_eq!(payload["nextModelCallOrdinal"], index + 1);
            let audit = audit_metadata(&payload);
            assert_eq!(audit["configVersion"], payload["configVersion"]);
            assert_eq!(
                audit["measurement"]["estimatedTokens"],
                measurement.estimated_tokens
            );
            assert_eq!(audit["assemblyReason"], reason);
            let service_limits = audit_metadata(&serde_json::json!({
                "requestedMaxTokens": 16384,
                "roundDeadlineMs": 1200000,
                "streamIdleDeadlineMs": 90000,
                "rawPrompt": "PRIVATE",
            }));
            assert_eq!(service_limits["requestedMaxTokens"], 16384);
            assert_eq!(service_limits["roundDeadlineMs"], 1200000);
            assert_eq!(service_limits["streamIdleDeadlineMs"], 90000);
            assert!(!service_limits.to_string().contains("PRIVATE"));
            let sensitive = serde_json::json!({
                "text":"PRIVATE", "message":"PRIVATE", "content":"PRIVATE", "params":{"path":"PRIVATE"},
                "projection":{"goal":"PRIVATE", "intentType":"inspect", "targetCandidates":["PRIVATE"]},
                "diagnostic":{"category":"model_protocol", "rawResponse":"PRIVATE"},
                "executionReceipt":{"effectStatus":"unknown", "summary":"PRIVATE", "failure":{"code":"AG_EFFECT_PENDING", "message":"PRIVATE"}},
                "toolName":"injected\nPRIVATE", "evidenceIds":["PRIVATE"]
            });
            let audit = audit_metadata(&sensitive);
            assert!(!audit.to_string().contains("PRIVATE"));
            assert_eq!(audit["projection"]["intentType"], "inspect");
            assert_eq!(audit["executionReceipt"]["effectStatus"], "unknown");
            assert_eq!(
                audit["executionReceipt"]["failure"]["code"],
                "AG_EFFECT_PENDING"
            );
            assert_eq!(audit["evidenceIdsCount"], 1);
            let history = audit_metadata(&serde_json::json!({
                "historySubmittedCount":4,"historyIncludedCount":2,"historyDroppedCount":2,
                "historyExcerptedCount":1,"historyOriginalChars":10000,"historyIncludedChars":1000,
                "historyAllowanceChars":1100,"historyText":"PRIVATE",
            }));
            assert_eq!(history["historyIncludedCount"], 2);
            assert_eq!(history["historyExcerptedCount"], 1);
            assert!(!history.to_string().contains("PRIVATE"));
            assert!(audit.get("toolName").is_none());
            emitter
                .record_internal("provider_started", serde_json::json!({}))
                .unwrap();
            let rejection_kind = if index % 2 == 0 {
                "model_control_rejected"
            } else {
                "tool_parameter_contract_rejected"
            };
            emitter
                .record_internal(
                    rejection_kind,
                    serde_json::json!({"code":"AG_MODEL_CONTROL_INVALID"}),
                )
                .unwrap();
            assert_eq!(
                TurnEventEmitter::without_channel(&state, "t1")
                    .protocol_attempt_counts()
                    .unwrap(),
                (index + 1, index + 1)
            );
            let audit = audit_metadata(&serde_json::json!({
                "prefixKind":"plain_text","rejectionStage":"response_completion","correctionAttempt":index+1,
                "finalOnly":true,"protocolVersion":"binder-response-v2","rawResponse":"PRIVATE"
            }));
            assert_eq!(audit["prefixKind"], "plain_text");
            assert_eq!(audit["correctionAttempt"], index + 1);
            assert!(!audit.to_string().contains("PRIVATE"));
        }
        let conn = state.db.lock().unwrap();
        assert!(list_safe_events(&conn, "ws1", "t1", 0).unwrap().is_empty());
        conn.execute(
            "UPDATE agent_turns SET status='completed' WHERE turn_id='t1'",
            [],
        )
        .unwrap();
        drop(conn);
        assert!(TurnEventEmitter::without_channel(&state, "t1")
            .record_prompt_budget(
                "initial",
                &measurement,
                serde_json::json!([]),
                ag::prompt_runtime::model_capacity("deepseek", "deepseek-chat")
            )
            .is_err());
    }

    // covers: BR-AG-STATE-001, BR-SYS-STATE-007, BR-AG-PERSIST-001
    #[test]
    fn failure_diagnostic_survives_safe_replay_without_changing_terminal_identity() {
        let state = AppState {
            db: Arc::new(Mutex::new(fresh_turn())),
            registry: build_registry(),
            assist_terminals: Arc::new(Mutex::new(HashMap::new())),
            active_agent_turns: Arc::new(Mutex::new(HashMap::new())),
        };
        let latch = ag::turn::TerminalLatch::default();
        let emitter = TurnEventEmitter::without_channel(&state, "t1");
        let diagnostic =
            AgentFailureDiagnostic::observed("context_assembly", "AG_PROMPT_BUDGET_EXCEEDED");
        assert!(emitter
            .emit_terminal(
                &latch,
                ag::turn::TerminalKind::Error,
                AgTurnEventPayload::Error {
                    code: "AG_PROMPT_BUDGET_EXCEEDED".into(),
                    message: "budget".into(),
                    retriable: true,
                    diagnostic: Some(diagnostic.clone()),
                }
            )
            .unwrap());
        assert!(!emitter
            .emit_terminal(
                &latch,
                ag::turn::TerminalKind::Completed,
                AgTurnEventPayload::Done {
                    stop_reason: "stop".into()
                }
            )
            .unwrap());
        let conn = state.db.lock().unwrap();
        let events = list_safe_events(&conn, "ws1", "t1", 0).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].turn_id, "t1");
        assert_eq!(events[0].seq, 1);
        assert!(
            matches!(&events[0].event, AgTurnEventPayload::Error { diagnostic: Some(d), .. } if *d == diagnostic)
        );
        assert!(list_safe_events(&conn, "ws2", "t1", 0).is_err());
        assert!(list_safe_events(&conn, "ws1", "t1", 1).unwrap().is_empty());
    }

    // covers: BR-AG-STATE-001, BR-SYS-STATE-007, BR-AG-PERSIST-001
    #[test]
    fn parked_turn_can_persist_cancelled_terminal_without_live_channel() {
        let state = AppState {
            db: Arc::new(Mutex::new(fresh_turn())),
            registry: build_registry(),
            assist_terminals: Arc::new(Mutex::new(HashMap::new())),
            active_agent_turns: Arc::new(Mutex::new(HashMap::new())),
        };
        let latch = ag::turn::TerminalLatch::default();
        let emitter = TurnEventEmitter::without_channel(&state, "t1");
        assert!(emitter
            .emit_terminal(
                &latch,
                ag::turn::TerminalKind::Cancelled,
                AgTurnEventPayload::Done {
                    stop_reason: "cancelled".into(),
                },
            )
            .unwrap());

        let conn = state.db.lock().unwrap();
        let events = list_safe_events(&conn, "ws1", "t1", 0).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event.terminal_status(), Some("cancelled"));
        let status: String = conn
            .query_row(
                "SELECT status FROM agent_turns WHERE turn_id='t1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, "cancelled");
    }

    // covers: BR-AG-PERSIST-001
    // covers: BR-SYS-STATE-007
    #[test]
    fn append_only_sequence_filters_internal_events_and_freezes_after_terminal() {
        let mut conn = fresh_turn();
        let internal = append_event_record(
            &mut conn,
            "t1",
            "context_frozen",
            &serde_json::json!({"openDocCount": 1}).to_string(),
            None,
        )
        .unwrap()
        .unwrap();
        assert_eq!(internal.seq, 1);

        let chunk = AgTurnEventPayload::Chunk { text: "A".into() };
        let chunk_record = append_event_record(
            &mut conn,
            "t1",
            chunk.kind(),
            &serde_json::to_string(&chunk).unwrap(),
            None,
        )
        .unwrap()
        .unwrap();
        assert_eq!(chunk_record.seq, 2);

        let done = AgTurnEventPayload::Done {
            stop_reason: "stop".into(),
        };
        let done_record = append_event_record(
            &mut conn,
            "t1",
            done.kind(),
            &serde_json::to_string(&done).unwrap(),
            done.terminal_status(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(done_record.seq, 3);

        assert!(append_event_record(
            &mut conn,
            "t1",
            chunk.kind(),
            &serde_json::to_string(&chunk).unwrap(),
            None,
        )
        .unwrap()
        .is_none());
        let status: String = conn
            .query_row(
                "SELECT status FROM agent_turns WHERE turn_id='t1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, "completed");

        let safe = list_safe_events(&conn, "ws1", "t1", 0).unwrap();
        assert_eq!(
            safe.iter().map(|event| event.seq).collect::<Vec<_>>(),
            vec![2, 3]
        );
        assert!(matches!(safe[0].event, AgTurnEventPayload::Chunk { .. }));
        let after = list_safe_events(&conn, "ws1", "t1", 2).unwrap();
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].seq, 3);
    }

    // covers: BR-AG-OBS-001, BR-AG-PERSIST-001, BR-SYS-DATA-008
    #[test]
    fn evidence_ledger_rebuilds_only_same_workspace_turn_source_events_in_order() {
        let mut conn = fresh_turn();
        let source = append_event_record(
            &mut conn,
            "t1",
            "context_frozen",
            &serde_json::json!({"contextVersion": "agent-context-facts-v1"}).to_string(),
            None,
        )
        .unwrap()
        .unwrap();
        let valid = crate::ag::tool_exec::materialize_evidence(
            "ws1",
            "t1",
            &source.event_id,
            source.seq,
            &source.event_id,
            vec![crate::ag::tool_exec::PendingEvidenceItem {
                rel_path: "a.md".to_string(),
                source: "active_editor".to_string(),
                coverage: "full_document".to_string(),
                version_or_hash: "h1".to_string(),
            }],
        );
        let mut forged = valid[0].clone();
        forged.workspace_id = "other".to_string();
        forged.evidence_id = "evidence:forged".to_string();
        append_event_record(
            &mut conn,
            "t1",
            "evidence_recorded",
            &serde_json::json!({"evidenceItems": [valid[0].clone(), forged]}).to_string(),
            None,
        )
        .unwrap();

        let ledger = rebuild_evidence_ledger(&conn, "ws1", "t1").unwrap();
        assert_eq!(ledger.turn_id, "t1");
        assert_eq!(ledger.through_seq, source.seq);
        assert_eq!(ledger.evidence_items.len(), 1);
        assert_eq!(ledger.evidence_items[0].source_event_id, source.event_id);
        assert!(rebuild_evidence_ledger(&conn, "other", "t1").is_err());
    }

    // covers: BR-AG-PERSIST-001
    #[test]
    fn safe_event_query_is_workspace_scoped_and_committed_rows_cannot_update() {
        let mut conn = fresh_turn();
        let chunk = AgTurnEventPayload::Chunk { text: "A".into() };
        append_event_record(
            &mut conn,
            "t1",
            chunk.kind(),
            &serde_json::to_string(&chunk).unwrap(),
            None,
        )
        .unwrap();
        assert!(matches!(
            list_safe_events(&conn, "other", "t1", 0),
            Err(rusqlite::Error::QueryReturnedNoRows)
        ));
        let update = conn.execute(
            "UPDATE agent_turn_events SET payload_json='{}' WHERE turn_id='t1'",
            [],
        );
        assert!(update.is_err(), "已提交 TurnEvent 不得原位修改");
    }
}
