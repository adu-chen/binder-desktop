//! AgentTurn 请求装配、模型轮次与受控工具循环。

#[cfg(test)]
use crate::ag::prompt_runtime::PROMPT_INPUT_LIMIT;
use crate::ipc::{AgTurnEvent, AgTurnEventPayload, IpcError};
use crate::types::{
    AgentContextFacts, AgentContextFactsV1, AgentContextFactsV2, AgentResourceAvailabilityV2,
    AgentSelectionValidationV2, ReferencePayload, AGENT_CONTEXT_FACTS_VERSION_V1,
    AGENT_CONTEXT_FACTS_VERSION_V2,
};
use crate::{ag, ent, workspace, AppState};
use std::collections::BTreeSet;
use std::sync::atomic::Ordering;

use super::edit_support::is_active_editor_snapshot_source;
use super::reference_runtime::BlockRef;
use super::task_runtime::settle_optional_task_run;
use super::turn_control::{
    bind_agent_turn_edit_target_set, bind_new_agent_turn_model, model_binding_for_resume,
    register_active_agent_turn, validate_agent_turn_decision_resume, AgentTurnResumeKind,
};
use super::turn_events::{
    emit_ag01, emit_ag_turn_cancelled_from, emit_ag_turn_denied, emit_ag_turn_error_from,
    settle_and_emit_ag_turn_error_from, sm_error, TurnEventEmitter,
};
use super::turn_prompt::{prepare_turn_prompt, PreparedTurnPrompt};
use super::turn_reference::{build_turn_reference_runtime, TurnReferenceRuntime};
use super::turn_tool_loop::run_turn_tool_loop;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentContextRequest {
    pub request_id: String,
    pub workspace_id: String,
    pub session_id: String,
    pub turn_id: String,
    pub context_revision: u64,
    pub paths: Vec<String>,
    pub all_documents: bool,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AgentContextResponse {
    pub request_id: String,
    pub facts: serde_json::Value,
    pub snapshots: Vec<ag::blocks::OpenDocSnapshot>,
}

pub(crate) struct AgentContextChannel(pub Option<tauri::ipc::Channel<AgentContextRequest>>);
impl<'de, R: tauri::Runtime> tauri::ipc::CommandArg<'de, R> for AgentContextChannel {
    fn from_command(
        command: tauri::ipc::CommandItem<'de, R>,
    ) -> Result<Self, tauri::ipc::InvokeError> {
        let webview = command.message.webview();
        let id =
            <Option<tauri::ipc::JavaScriptChannelId> as serde::Deserialize>::deserialize(command)
                .map_err(|e| tauri::ipc::InvokeError::from(e.to_string()))?;
        Ok(Self(id.map(|id| id.channel_on(webview))))
    }
}

struct PendingContext {
    request: AgentContextRequest,
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
    sender: std::sync::mpsc::Sender<AgentContextResponse>,
    deadline: std::time::Instant,
}

fn pending_contexts() -> &'static std::sync::Mutex<std::collections::HashMap<String, PendingContext>>
{
    static PENDING: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<String, PendingContext>>,
    > = std::sync::OnceLock::new();
    PENDING.get_or_init(Default::default)
}

struct ContextRequestLease(String);
impl Drop for ContextRequestLease {
    fn drop(&mut self) {
        if let Ok(mut pending) = pending_contexts().lock() {
            pending.remove(&self.0);
        }
    }
}

/**
 * @GOV
 * codes:    BR-AG-DATA-008-GUARD-AG-AG-TURN-053, BR-SYS-STATE-007-GUARD-AG-AG-TURN-053
 * type:     GUARD
 * chain:    AG-TURN, AG-TOOL
 * rules:    BR-AG-DATA-008, BR-SYS-STATE-007
 * term_ref: TERM-AG-020
 * boundary: in=HostContextResponseBoundToPendingRequestIdentity | out=SingleConsumedContextResponseOrStableRejection | does_not=resumeTaskOrAcceptExpiredCrossSessionResponseOrAuthorizeTool | consumer=BR-AG-DATA-008
 */
#[tauri::command]
pub(crate) fn ag_supply_context(response: AgentContextResponse) -> Result<(), IpcError> {
    let rejected = || {
        IpcError::new(
            "AG_CONTEXT_RESPONSE_STALE",
            "上下文响应已过期或身份不匹配",
            false,
            false,
        )
    };
    let mut pending = pending_contexts().lock().map_err(|_| rejected())?;
    let entry = pending.get(&response.request_id).ok_or_else(rejected)?;
    let facts = &response.facts;
    if entry.cancelled.load(Ordering::Acquire)
        || std::time::Instant::now() >= entry.deadline
        || facts.get("workspaceId").and_then(|v| v.as_str()) != Some(&entry.request.workspace_id)
        || facts.get("sessionId").and_then(|v| v.as_str()) != Some(&entry.request.session_id)
        || facts.get("turnId").and_then(|v| v.as_str()) != Some(&entry.request.turn_id)
        || facts.get("contextRevision").and_then(|v| v.as_u64())
            != Some(entry.request.context_revision)
    {
        return Err(rejected());
    }
    pending
        .remove(&response.request_id)
        .ok_or_else(rejected)?
        .sender
        .send(response)
        .map_err(|_| rejected())
}

pub(super) async fn request_agent_context(
    channel: Option<&tauri::ipc::Channel<AgentContextRequest>>,
    facts: &AgentContextFactsV2,
    paths: Vec<String>,
    all_documents: bool,
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<AgentContextResponse, IpcError> {
    let channel = channel.ok_or_else(|| {
        IpcError::new(
            "AG_CONTEXT_HOST_UNAVAILABLE",
            "宿主上下文通道不可用，请重新提交",
            true,
            false,
        )
    })?;
    let revision = facts.context_revision.checked_add(1).ok_or_else(|| {
        IpcError::new(
            "AG_CONTEXT_REVISION_EXHAUSTED",
            "上下文版本已耗尽",
            false,
            false,
        )
    })?;
    let request = AgentContextRequest {
        request_id: uuid::Uuid::new_v4().to_string(),
        workspace_id: facts.workspace_id.clone(),
        session_id: facts.session_id.clone(),
        turn_id: facts.turn_id.clone(),
        context_revision: revision,
        paths,
        all_documents,
    };
    let (sender, receiver) = std::sync::mpsc::channel();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let lease = ContextRequestLease(request.request_id.clone());
    pending_contexts()
        .lock()
        .map_err(|_| IpcError::new("AG_CONTEXT_HOST_BUSY", "上下文通道繁忙", true, false))?
        .insert(
            request.request_id.clone(),
            PendingContext {
                request: request.clone(),
                cancelled: cancelled.clone(),
                sender,
                deadline,
            },
        );
    channel.send(request).map_err(|_| {
        IpcError::new(
            "AG_CONTEXT_HOST_UNAVAILABLE",
            "宿主上下文请求无法送达",
            true,
            false,
        )
    })?;
    let response = tauri::async_runtime::spawn_blocking(move || loop {
        if cancelled.load(Ordering::Acquire) {
            return Err(IpcError::new(
                "AG_CONTEXT_CANCELLED",
                "上下文采集已取消",
                false,
                false,
            ));
        }
        if std::time::Instant::now() >= deadline {
            return Err(IpcError::new(
                "AG_CONTEXT_TIMEOUT",
                "上下文采集超时，请就绪后重新提交",
                true,
                false,
            ));
        }
        match receiver.recv_timeout(std::time::Duration::from_millis(20)) {
            Ok(response) => return Ok(response),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                return Err(IpcError::new(
                    "AG_CONTEXT_HOST_UNAVAILABLE",
                    "上下文响应通道已关闭",
                    true,
                    false,
                ))
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
        }
    })
    .await
    .map_err(|_| {
        IpcError::new(
            "AG_CONTEXT_HOST_UNAVAILABLE",
            "上下文等待任务异常结束",
            true,
            false,
        )
    })?;
    drop(lease);
    response
}

pub(super) async fn refresh_agent_context(
    channel: Option<&tauri::ipc::Channel<AgentContextRequest>>,
    previous: &AgentContextFactsV2,
    read_ctx: &ag::blocks::AgentReadContext,
    tool: &str,
    params: &serde_json::Value,
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<(AgentContextFacts, ag::blocks::AgentReadContext), IpcError> {
    let paths: Vec<String> = params
        .get("path")
        .and_then(|v| v.as_str())
        .into_iter()
        .map(|path| {
            let absolute = crate::workspace::resolve_in_workspace(read_ctx.root(), path)?;
            absolute
                .strip_prefix(read_ctx.root())
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .map_err(|_| {
                    IpcError::new(
                        "AG_CONTEXT_RESOURCE_INVALID",
                        "资源路径不属于当前工作区",
                        false,
                        false,
                    )
                })
        })
        .collect::<Result<_, _>>()?;
    let all_documents = tool == "search_text";
    let response =
        request_agent_context(channel, previous, paths.clone(), all_documents, cancelled).await?;
    let block_refs: Vec<BlockRef> = previous
        .selections
        .iter()
        .map(|s| BlockRef {
            rel_path: s.rel_path.clone(),
            block_id: s.block_id.clone(),
            start: s.start,
            end: s.end,
            text: String::new(),
        })
        .collect();
    let facts = parse_agent_context_facts(
        Some(response.facts),
        &previous.workspace_id,
        read_ctx.root(),
        &previous.turn_id,
        &previous.session_id,
        previous.active_target.as_ref().map(|t| t.rel_path.as_str()),
        &response.snapshots,
        &[],
        &block_refs,
    )?;
    let AgentContextFacts::V2(next) = &facts else {
        return Err(IpcError::new(
            "AG_CONTEXT_FACTS_VERSION_UNSUPPORTED",
            "上下文回传不允许降级",
            false,
            false,
        ));
    };
    if next.context_revision != previous.context_revision + 1
        || next.active_target != previous.active_target
        || previous.open_resources.iter().any(|old| {
            !next
                .open_resources
                .iter()
                .any(|new| new.rel_path == old.rel_path)
        })
    {
        return Err(IpcError::new(
            "AG_CONTEXT_RESOURCE_STALE",
            "冻结目标或打开资源身份已改变，请重新提交",
            true,
            false,
        ));
    }
    if tool == "propose_edit" {
        for path in &paths {
            if let Some(new) = next.open_resources.iter().find(|r| &r.rel_path == path) {
                let old = previous.open_resources.iter().find(|r| &r.rel_path == path);
                if old.and_then(|r| r.snapshot.as_ref()) != new.snapshot.as_ref()
                    || new.snapshot.is_none()
                    || next.selections.iter().any(|s| {
                        &s.rel_path == path && s.validation != AgentSelectionValidationV2::Valid
                    })
                {
                    return Err(IpcError::new(
                        "AG_CONTEXT_RESOURCE_STALE",
                        "文档或选区已变化，必须重新读取后再提出修改",
                        true,
                        false,
                    ));
                }
            }
        }
    }
    let context = ag::blocks::AgentReadContext::with_resources(
        read_ctx.root(),
        response.snapshots,
        next,
        &paths,
        all_documents,
    )?;
    for path in &paths {
        context.check_resource_available(path)?;
    }
    if all_documents {
        for resource in &next.open_resources {
            context.check_resource_available(&resource.rel_path)?;
        }
    }
    Ok((facts, context))
}

/**
 * @GOV
 * codes:    BR-SYS-STATE-007-GUARD-AG-AG-TURN-037
 * type:     GUARD
 * chain:    AG-TURN, AG-TOOL
 * rules:    BR-SYS-STATE-007
 * term_ref: TERM-AG-013
 * boundary: in=OptionalRuntimeProfile | out=ValidatedAgentEditRuntimeProfileOrStablePreGateRejection | does_not=applyCompatibilityDefaultOrEmitExecutableEvent
 */
fn require_runtime_profile(runtime_profile: Option<String>) -> Result<String, IpcError> {
    let runtime_profile = runtime_profile.ok_or_else(|| {
        IpcError::new(
            "AG_RUNTIME_PROFILE_REQUIRED",
            "缺少 Agent RuntimeProfile",
            false,
            false,
        )
    })?;
    ag::turn::ensure_runtime_profile(&runtime_profile)?;
    Ok(runtime_profile)
}

/**
 * @GOV
 * codes:    BR-AG-DATA-004-GUARD-AG-AG-TURN-049
 * type:     GUARD
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-004
 * term_ref: TERM-AG-001
 * boundary: in=HistTurnWorkspaceAndAgentSessionIdentities | out=boolean(trueWhenEveryHistoryItemBelongsToCurrentWorkspaceAndAgentSession) | consumer=BR-AG-DATA-004
 */
fn validate_history_scope(
    history: &[ag::context::HistTurn],
    workspace_id: &str,
    session_id: &str,
) -> Result<(), IpcError> {
    if history.iter().any(|turn| {
        turn.workspace_id != workspace_id
            || turn.session_id != session_id
            || !matches!(turn.role.as_str(), "user" | "assistant")
    }) {
        return Err(IpcError::new(
            "AG_HISTORY_SCOPE_MISMATCH",
            "对话历史不属于当前 workspace/AgentSession，已在模型调用前拒绝注入",
            false,
            false,
        ));
    }
    Ok(())
}

/**
 * @GOV
 * codes:    BR-AG-DATA-008-GUARD-AG-AG-TURN-042, BR-ED-DATA-008-GUARD-AG-AG-TURN-042, BR-SYS-DATA-008-GUARD-AG-AG-TURN-042
 * type:     GUARD
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-008, BR-ED-DATA-008, BR-SYS-DATA-008
 * term_ref: TERM-AG-020
 * boundary: in=AgentContextFactsV1WorkspaceTurnOpenDocSnapshotsAndExplicitReferences | out=VerifiedSameTurnLogicalStateFactsOrStablePreGateRejection | does_not=acceptUnknownVersionCrossWorkspaceFactsOrGrantReadWriteCapability | consumer=BR-AG-DATA-008
 */
fn validate_agent_context_facts(
    facts: Option<&AgentContextFactsV1>,
    workspace_id: &str,
    workspace_root: &std::path::Path,
    turn_id: &str,
    session_id: &str,
    active_doc_path: Option<&str>,
    open_doc_snapshots: &[ag::blocks::OpenDocSnapshot],
    references: &[ReferencePayload],
    block_refs: &[BlockRef],
) -> Result<(), IpcError> {
    let facts = facts.ok_or_else(|| {
        IpcError::new(
            "AG_CONTEXT_FACTS_REQUIRED",
            "缺少版本化 AgentContextFacts，已在 Gate 前拒绝回合",
            false,
            false,
        )
    })?;
    if facts.version != AGENT_CONTEXT_FACTS_VERSION_V1 {
        return Err(IpcError::new(
            "AG_CONTEXT_FACTS_VERSION_UNSUPPORTED",
            "AgentContextFacts 版本未知，已在 Gate 前拒绝回合",
            false,
            false,
        ));
    }
    if facts.workspace_id != workspace_id || facts.turn_id != turn_id {
        return Err(IpcError::new(
            "AG_CONTEXT_FACTS_IDENTITY_MISMATCH",
            "AgentContextFacts 不属于当前 workspace/AgentTurn",
            false,
            false,
        ));
    }
    if facts.open_docs.len() != open_doc_snapshots.len() {
        return Err(IpcError::new(
            "AG_CONTEXT_FACTS_OPEN_DOCS_MISMATCH",
            "AgentContextFacts.openDocs 与 OpenDocSnapshot 集合不一致",
            true,
            false,
        ));
    }
    let mut seen = BTreeSet::new();
    for fact in &facts.open_docs {
        crate::workspace::resolve_in_workspace(workspace_root, &fact.rel_path)?;
        if !seen.insert(fact.rel_path.as_str()) {
            return Err(IpcError::new(
                "AG_CONTEXT_FACTS_DUPLICATE_DOC",
                "AgentContextFacts.openDocs 含重复路径",
                false,
                false,
            ));
        }
        let snapshot = open_doc_snapshots
            .iter()
            .find(|snapshot| snapshot.rel_path == fact.rel_path)
            .ok_or_else(|| {
                IpcError::new(
                    "AG_CONTEXT_FACTS_OPEN_DOCS_MISMATCH",
                    "AgentContextFacts 声明了不存在的打开文档事实",
                    true,
                    false,
                )
            })?;
        let baseline = snapshot.baseline.map(|value| value.to_string());
        if fact.source != snapshot.source
            || fact.edit_format != snapshot.edit_format
            || fact.content_format != snapshot.content_format
            || fact.dirty != snapshot.dirty
            || fact.baseline != baseline
            || fact.session_version != snapshot.session_version
            || fact.content_hash != snapshot.content_hash
        {
            return Err(IpcError::new(
                "AG_CONTEXT_FACTS_LOGICAL_STATE_MISMATCH",
                "AgentContextFacts 与同轮 OpenDocSnapshot LogicalState 身份不一致",
                true,
                false,
            ));
        }
    }
    match (active_doc_path, facts.active_doc.as_ref()) {
        (None, None) => {}
        (Some(active_path), Some(active_fact))
            if active_fact.rel_path == active_path
                && facts.open_docs.iter().any(|fact| fact == active_fact)
                && is_active_editor_snapshot_source(&active_fact.source) => {}
        _ => {
            return Err(IpcError::new(
                "AG_CONTEXT_FACTS_ACTIVE_DOC_MISMATCH",
                "AgentContextFacts.activeDoc 与当前活动编辑器不一致",
                true,
                false,
            ));
        }
    }
    let mut expected_selections = references
        .iter()
        .filter_map(|reference| match reference {
            ReferencePayload::Selection {
                rel_path,
                block_id,
                start,
                end,
                ..
            } => Some((rel_path.as_str(), block_id.as_str(), *start, *end)),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    expected_selections.extend(block_refs.iter().map(|reference| {
        (
            reference.rel_path.as_str(),
            reference.block_id.as_str(),
            reference.start,
            reference.end,
        )
    }));
    let actual_selections = facts
        .selections
        .iter()
        .map(|selection| {
            crate::workspace::resolve_in_workspace(workspace_root, &selection.rel_path)?;
            Ok((
                selection.rel_path.as_str(),
                selection.block_id.as_str(),
                selection.start,
                selection.end,
            ))
        })
        .collect::<Result<BTreeSet<_>, IpcError>>()?;
    if actual_selections != expected_selections {
        return Err(IpcError::new(
            "AG_CONTEXT_FACTS_SELECTION_MISMATCH",
            "AgentContextFacts.selections 只能来自本轮显式 SelectionRef",
            false,
            false,
        ));
    }
    for proposal in &facts.proposal_facts {
        crate::workspace::resolve_in_workspace(workspace_root, &proposal.rel_path)?;
        if proposal.diff_id.trim().is_empty()
            || proposal.session_id != session_id
            || !matches!(
                proposal.state.as_str(),
                "pending"
                    | "validating"
                    | "applying"
                    | "accepted"
                    | "rejected"
                    | "expired"
                    | "conflict"
                    | "error"
            )
            || !matches!(
                proposal.effect_status.as_deref(),
                None | Some("not_applicable")
                    | Some("awaiting_save")
                    | Some("saved")
                    | Some("discarded_on_abnormal_exit")
            )
            || (proposal.saved && proposal.effect_status.as_deref() == Some("awaiting_save"))
        {
            return Err(IpcError::new(
                "AG_CONTEXT_FACTS_PROPOSAL_INVALID",
                "AgentContextFacts.proposalFacts 含无效身份或状态",
                false,
                false,
            ));
        }
    }
    Ok(())
}

/**
 * @GOV
 * codes:    BR-AG-DATA-008-GUARD-AG-AG-TURN-052, BR-SYS-DATA-008-GUARD-AG-AG-TURN-052
 * type:     GUARD
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-008, BR-SYS-DATA-008
 * term_ref: TERM-AG-020
 * boundary: in=VersionedAgentContextFactsWorkspaceSessionTurnAndOpenDocSnapshots | out=ValidatedV1OrV2Facts | does_not=downgradeV2OrTreatUnavailableAsReadyOrGrantAccess
 */
#[allow(clippy::too_many_arguments)]
fn parse_agent_context_facts(
    payload: Option<serde_json::Value>,
    workspace_id: &str,
    root: &std::path::Path,
    turn_id: &str,
    session_id: &str,
    active_path: Option<&str>,
    snapshots: &[ag::blocks::OpenDocSnapshot],
    references: &[ReferencePayload],
    block_refs: &[BlockRef],
) -> Result<AgentContextFacts, IpcError> {
    let invalid = || {
        IpcError::new(
            "AG_CONTEXT_FACTS_INVALID",
            "上下文事实载荷无效，请重新提交",
            false,
            false,
        )
    };
    let payload = payload.ok_or_else(|| {
        IpcError::new("AG_CONTEXT_FACTS_REQUIRED", "缺少上下文事实", false, false)
    })?;
    match payload.get("version").and_then(|v| v.as_str()) {
        Some(AGENT_CONTEXT_FACTS_VERSION_V1) => {
            let facts: AgentContextFactsV1 =
                serde_json::from_value(payload).map_err(|_| invalid())?;
            validate_agent_context_facts(
                Some(&facts),
                workspace_id,
                root,
                turn_id,
                session_id,
                active_path,
                snapshots,
                references,
                block_refs,
            )?;
            Ok(AgentContextFacts::V1(facts))
        }
        Some(AGENT_CONTEXT_FACTS_VERSION_V2) => {
            // serde Option alone accepts missing fields; V2 requires explicit nullable fields.
            if !payload.get("activeTarget").is_some() {
                return Err(invalid());
            }
            if payload
                .get("openResources")
                .and_then(|v| v.as_array())
                .map_or(true, |resources| {
                    resources
                        .iter()
                        .any(|r| r.get("snapshot").is_none() || r.get("reasonCode").is_none())
                })
            {
                return Err(invalid());
            }
            let facts: AgentContextFactsV2 =
                serde_json::from_value(payload).map_err(|_| invalid())?;
            if facts.workspace_id != workspace_id
                || facts.session_id != session_id
                || facts.turn_id != turn_id
                || facts.context_revision == 0
            {
                return Err(IpcError::new(
                    "AG_CONTEXT_FACTS_IDENTITY_MISMATCH",
                    "上下文身份或版本不属于当前请求",
                    false,
                    false,
                ));
            }
            if facts.active_target.as_ref().map(|t| t.rel_path.as_str()) != active_path {
                return Err(invalid());
            }
            let mut paths = BTreeSet::new();
            for resource in &facts.open_resources {
                let absolute = crate::workspace::resolve_in_workspace(root, &resource.rel_path)?;
                if absolute
                    .strip_prefix(root)
                    .ok()
                    .map(|p| p.to_string_lossy().replace('\\', "/"))
                    .as_deref()
                    != Some(resource.rel_path.as_str())
                {
                    return Err(invalid());
                }
                if resource.rel_path.is_empty() || !paths.insert(resource.rel_path.as_str()) {
                    return Err(invalid());
                }
                match (&resource.availability, &resource.snapshot) {
                    (AgentResourceAvailabilityV2::Ready, Some(s))
                        if s.session_version > 0
                            && !s.content_hash.is_empty()
                            && matches!(
                                s.source.as_str(),
                                "active_editor" | "active_editor_session" | "open_editor_session"
                            )
                            && matches!(
                                s.edit_format.as_str(),
                                "text" | "markdown" | "html" | "html_file"
                            )
                            && matches!(s.content_format.as_str(), "text" | "html") => {}
                    (AgentResourceAvailabilityV2::NotRequested, None) => {}
                    (
                        AgentResourceAvailabilityV2::Loading
                        | AgentResourceAvailabilityV2::Unavailable,
                        None,
                    ) if resource.reason_code.as_ref().is_some_and(|c| {
                        !c.is_empty()
                            && c.len() <= 96
                            && c.bytes()
                                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
                    }) => {}
                    _ => return Err(invalid()),
                }
            }
            if active_path.is_some_and(|p| !paths.contains(p)) {
                return Err(invalid());
            }
            let ready = facts.ready_documents();
            for selection in &facts.selections {
                if selection.context_revision != facts.context_revision
                    || selection.block_id.is_empty()
                    || matches!((selection.start, selection.end), (Some(a), Some(b)) if a > b)
                    || (selection.validation == AgentSelectionValidationV2::Valid
                        && !ready.iter().any(|d| d.rel_path == selection.rel_path))
                {
                    return Err(invalid());
                }
            }
            // Reuse strict metadata/reference/proposal checks only. The full V2 remains the wire and prompt fact.
            let metadata = AgentContextFactsV1 {
                version: AGENT_CONTEXT_FACTS_VERSION_V1.into(),
                workspace_id: facts.workspace_id.clone(),
                turn_id: facts.turn_id.clone(),
                active_doc: None,
                open_docs: ready,
                selections: facts
                    .selections
                    .iter()
                    .map(|s| crate::types::AgentContextSelectionFactV1 {
                        rel_path: s.rel_path.clone(),
                        block_id: s.block_id.clone(),
                        start: s.start,
                        end: s.end,
                    })
                    .collect(),
                proposal_facts: facts.proposal_facts.clone(),
            };
            validate_agent_context_facts(
                Some(&metadata),
                workspace_id,
                root,
                turn_id,
                session_id,
                None,
                snapshots,
                references,
                block_refs,
            )?;
            Ok(AgentContextFacts::V2(facts))
        }
        _ => Err(IpcError::new(
            "AG_CONTEXT_FACTS_VERSION_UNSUPPORTED",
            "上下文版本不受支持",
            false,
            false,
        )),
    }
}

/**
 * @GOV
 * codes:    BR-AG-OBS-001-API-AG-AG-TURN-018, BR-AG-STATE-001-API-AG-AG-TURN-018, BR-AG-STATE-004-API-AG-AG-TURN-018, BR-AG-DATA-008-API-AG-AG-TURN-018, BR-ED-DATA-008-API-AG-AG-TURN-018, BR-SYS-STATE-007-API-AG-AG-TURN-018, BR-SYS-DATA-008-API-AG-AG-TURN-018
 * type:     API
 * chain:    AG-TURN, AG-TOOL
 * rules:    BR-AG-OBS-001, BR-AG-STATE-001, BR-AG-STATE-004, BR-AG-DATA-008, BR-ED-DATA-008, BR-SYS-STATE-007, BR-SYS-DATA-008
 * term_ref: TERM-AG-001, TERM-AG-012, TERM-AG-013, TERM-AG-014, TERM-AG-015, TERM-AG-016, TERM-AG-020, TERM-AG-021, TERM-AG-023
 * boundary: in=ValidatedRuntimeProfileAndAgentTurnRequestOrSameTurnResumeAndAgentReadContext | out=AdaptiveModelToolEventStreamContextEvidenceOrNonTerminalWaitAndOptionalTaskRunProjection | effect=IPC-ag_start_turn | delegate=require_runtime_profile,prepare_turn_prompt,ent::gateway::check_gate,ag::turn::transition,run_turn_tool_loop,register_active_agent_turn | does_not=classifyIntentByKeywordsDefaultMissingRuntimeProfileCreateNewTurnForResumeBypassCurrentGateOutcomeOrWriteDocumentContentOutsidePendingDiff | consumer=BR-AG-STATE-001,BR-AG-DATA-008,BR-AG-OBS-001
 */
/// PI-AG-WIRE-01 · `ag_start_turn`（独立 async tauri command，不走 dispatch 信封）。
///
/// 驱动 SM-AG-01 门禁前置，经 ENT 网关流式调用并执行受控工具循环。
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub(crate) async fn ag_start_turn<R: tauri::Runtime>(
    turn_id: String,
    session_id: String,
    runtime_profile: Option<String>,
    prompt: String,
    provider: Option<String>,
    model: Option<String>,
    active_doc_path: Option<String>,
    active_doc_content: Option<String>,
    active_doc_format: Option<String>,
    open_doc_snapshots: Vec<ag::blocks::OpenDocSnapshot>,
    agent_context_facts: Option<serde_json::Value>,
    history: Vec<ag::context::HistTurn>,
    mentions: Vec<String>,
    block_refs: Vec<BlockRef>,
    references: Vec<ReferencePayload>,
    continuation_task_run_id: Option<String>,
    continuation_decision_request_id: Option<String>,
    continuation_preview_diff_id: Option<String>,
    clarification_resume: Option<bool>,
    decision_resume: Option<bool>,
    on_event: tauri::ipc::Channel<AgTurnEvent>,
    on_context_request: AgentContextChannel,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle<R>,
) -> Result<(), IpcError> {
    run_agent_turn(
        turn_id,
        session_id,
        runtime_profile,
        prompt,
        provider,
        model,
        active_doc_path,
        active_doc_content,
        active_doc_format,
        open_doc_snapshots,
        agent_context_facts,
        history,
        mentions,
        block_refs,
        references,
        continuation_task_run_id,
        continuation_decision_request_id,
        continuation_preview_diff_id,
        clarification_resume,
        decision_resume,
        on_event,
        on_context_request,
        &state,
        app,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_agent_turn<R: tauri::Runtime>(
    turn_id: String,
    session_id: String,
    runtime_profile: Option<String>,
    prompt: String,
    provider: Option<String>,
    model: Option<String>,
    active_doc_path: Option<String>,
    active_doc_content: Option<String>,
    active_doc_format: Option<String>,
    open_doc_snapshots: Vec<ag::blocks::OpenDocSnapshot>,
    agent_context_facts: Option<serde_json::Value>,
    history: Vec<ag::context::HistTurn>,
    mentions: Vec<String>,
    block_refs: Vec<BlockRef>,
    references: Vec<ReferencePayload>,
    continuation_task_run_id: Option<String>,
    continuation_decision_request_id: Option<String>,
    continuation_preview_diff_id: Option<String>,
    clarification_resume: Option<bool>,
    decision_resume: Option<bool>,
    on_event: tauri::ipc::Channel<AgTurnEvent>,
    on_context_request: AgentContextChannel,
    state: &AppState,
    app: tauri::AppHandle<R>,
) -> Result<(), IpcError> {
    use ag::turn::{ensure_gate_passed, transition, GateState, TurnEvent, TurnState};

    let runtime_profile = require_runtime_profile(runtime_profile)?;
    let clarification_resume = clarification_resume.unwrap_or(false);
    let decision_resume = decision_resume.unwrap_or(false);
    if clarification_resume && decision_resume {
        return Err(IpcError::new(
            "AG_TURN_RESUME_CONFLICT",
            "同一次调用不能同时恢复澄清等待和裁决等待",
            false,
            false,
        ));
    }
    let resume_kind = if clarification_resume {
        AgentTurnResumeKind::Clarification
    } else if decision_resume {
        AgentTurnResumeKind::Decision
    } else {
        AgentTurnResumeKind::New
    };

    if prompt.trim().is_empty() && references.is_empty() {
        return Err(IpcError::new(
            "AG_PROMPT_REQUIRED",
            "缺少 prompt",
            false,
            false,
        ));
    }

    let (workspace_id_for_turn, workspace_root_path) = {
        let conn = state
            .db
            .lock()
            .map_err(|_| IpcError::new("WS_DB_BUSY", "数据库连接繁忙", true, false))?;
        let ws = workspace::lifecycle::active_workspace(&conn)
            .map_err(|e| IpcError::new("WS_ACTIVE_FAILED", &e.to_string(), true, false))?
            .ok_or_else(|| IpcError::new("WS_NO_ACTIVE", "无活动 workspace", false, false))?;
        (ws.workspace_id.clone(), ws.root_path.clone())
    };

    validate_history_scope(&history, &workspace_id_for_turn, &session_id)?;

    let agent_context_facts = parse_agent_context_facts(
        agent_context_facts,
        &workspace_id_for_turn,
        std::path::Path::new(&workspace_root_path),
        &turn_id,
        &session_id,
        active_doc_path.as_deref(),
        &open_doc_snapshots,
        &references,
        &block_refs,
    )?;
    if let AgentContextFacts::V2(facts) = &agent_context_facts {
        use rusqlite::OptionalExtension;
        let conn = state
            .db
            .lock()
            .map_err(|_| IpcError::new("WS_DB_BUSY", "数据库连接繁忙", true, false))?;
        let previous: Option<String> = conn.query_row("SELECT payload_json FROM agent_turn_events WHERE workspace_id=?1 AND session_id=?2 AND turn_id=?3 AND kind='context_frozen' ORDER BY seq DESC LIMIT 1", rusqlite::params![workspace_id_for_turn, session_id, turn_id], |row| row.get(0)).optional()
            .map_err(|_| IpcError::new("AG_CONTEXT_REVISION_CHECK_FAILED", "无法核验上下文版本", true, false))?;
        if let Some(previous) = previous {
            let previous: serde_json::Value = serde_json::from_str(&previous).map_err(|_| {
                IpcError::new(
                    "AG_CONTEXT_REVISION_CHECK_FAILED",
                    "历史上下文版本无效",
                    false,
                    false,
                )
            })?;
            if previous["contextVersion"] != AGENT_CONTEXT_FACTS_VERSION_V2
                || previous["contextRevision"]
                    .as_u64()
                    .map_or(true, |revision| facts.context_revision <= revision)
                || previous["activeTarget"]
                    != serde_json::to_value(&facts.active_target).expect("serializable target")
            {
                return Err(IpcError::new(
                    "AG_CONTEXT_RESOURCE_STALE",
                    "恢复请求的上下文版本或冻结目标不匹配，请重新发起任务",
                    false,
                    false,
                ));
            }
        }
    }

    if clarification_resume
        && (continuation_task_run_id.is_some()
            || continuation_decision_request_id.is_some()
            || continuation_preview_diff_id.is_some())
    {
        return Err(IpcError::new(
            "AG_CLARIFICATION_CONTINUATION_CONFLICT",
            "澄清恢复不能与任务裁决或预览续片段混用",
            false,
            false,
        ));
    }
    if decision_resume
        && (continuation_task_run_id.is_none()
            || continuation_decision_request_id.is_none()
            || continuation_preview_diff_id.is_some())
    {
        return Err(IpcError::new(
            "AG_DECISION_CONTINUATION_INVALID",
            "裁决恢复必须且只能携带原 TaskRun 与已解决的 DecisionRequest 身份",
            false,
            false,
        ));
    }
    if !decision_resume && continuation_decision_request_id.is_some() {
        return Err(IpcError::new(
            "AG_DECISION_RESUME_REQUIRED",
            "已解决裁决只能用于恢复原 AgentTurn，不能创建替代回合",
            false,
            false,
        ));
    }
    if decision_resume {
        validate_agent_turn_decision_resume(
            &state,
            &session_id,
            &turn_id,
            continuation_decision_request_id
                .as_deref()
                .expect("decision resume identity was validated as present"),
        )?;
    }

    let (bound_provider, bound_model) = if resume_kind == AgentTurnResumeKind::New {
        match (provider.as_deref(), model.as_deref()) {
            (Some(provider), Some(model))
                if !provider.trim().is_empty() && !model.trim().is_empty() =>
            {
                (provider.to_string(), model.to_string())
            }
            _ => {
                return Err(IpcError::new(
                    "AG_MODEL_SELECTION_INVALID",
                    "模型选择必须同时包含 provider 和 model，且均不为空",
                    false,
                    false,
                ))
            }
        }
    } else {
        model_binding_for_resume(
            &state,
            &workspace_id_for_turn,
            &session_id,
            &turn_id,
            provider.as_deref(),
            model.as_deref(),
        )?
    };
    let model_capacity = ag::prompt_runtime::model_capacity(&bound_provider, &bound_model);

    let (
        _active_turn_lease,
        cancelled,
        clarification_task_run_id,
        registered_edit_target_set,
        resumed_runtime_checkpoint,
    ) = register_active_agent_turn(
        &state,
        &workspace_id_for_turn,
        &session_id,
        &turn_id,
        resume_kind,
    )?;
    if resume_kind == AgentTurnResumeKind::New {
        bind_new_agent_turn_model(&state, &session_id, &turn_id, &bound_provider, &bound_model)?;
    }
    let terminal_latch = ag::turn::TerminalLatch::default();

    // 注册成功即执行 send / clarificationProvided；后续持久化与上下文构建都是
    // Assembling 的动作，失败只能由 assembleFail 收束为 TurnError。
    let s = match resume_kind {
        AgentTurnResumeKind::Clarification => transition(
            TurnState::AwaitingClarification,
            TurnEvent::ClarificationProvided,
        ),
        AgentTurnResumeKind::Decision => {
            transition(TurnState::AwaitingDecision, TurnEvent::DecisionResolved)
        }
        AgentTurnResumeKind::New => transition(TurnState::Idle, TurnEvent::Send),
    };
    let s = s.ok_or_else(sm_error)?;
    emit_ag01(&app, &turn_id, s);

    let history_result = (|| -> Result<(), IpcError> {
        let conn = state
            .db
            .lock()
            .map_err(|_| IpcError::new("WS_DB_BUSY", "数据库连接繁忙", true, false))?;
        // 澄清恢复先由活动回合注册表完成 session/workspace/turn 身份认领；只有认领成功后才允许
        // 创建/更新持久化会话，错误身份不得留下空会话或草稿副作用。
        ag::history::ensure_session_exists(&conn, &workspace_id_for_turn, &session_id)
            .map_err(|e| IpcError::new("AG_HISTORY_DB_ERROR", &e.to_string(), true, false))?;
        if resume_kind != AgentTurnResumeKind::New {
            return ag::history::ensure_turn_resumable(
                &conn,
                &workspace_id_for_turn,
                &session_id,
                &turn_id,
            )
            .map_err(|e| {
                IpcError::new(
                    "AG_TURN_RESUME_HISTORY_INVALID",
                    &format!("原 AgentTurn 的持久化身份不可恢复：{e}"),
                    false,
                    false,
                )
            });
        }
        let draft_payload = ag::history::AgentTurnPayload {
            version: 2,
            last_event_seq: 0,
            user_message: ag::history::PersistedChatMessage {
                id: format!("u-{turn_id}"),
                role: "user".to_string(),
                content: prompt.trim().to_string(),
                parts: None,
                streaming: None,
                error: None,
                segments: None,
            },
            assistant_message: None,
            tool_summaries: Vec::new(),
            diff_refs: Vec::new(),
            failure: None,
            incomplete_reason: Some("stream_interrupted".to_string()),
        };
        ag::history::save_turn(
            &conn,
            &workspace_id_for_turn,
            &session_id,
            &turn_id,
            active_doc_path.as_deref(),
            "draft",
            &draft_payload,
        )
        .map_err(|e| IpcError::new("AG_HISTORY_DB_ERROR", &e.to_string(), true, false))?;
        Ok(())
    })();
    if let Err(error) = history_result {
        // draft 尚未建立时不存在可追加的 TurnEvent；由 command error 返回，不能制造未持久化事件。
        return Err(error);
    }
    let event_emitter = TurnEventEmitter::new(&state, &on_event, &turn_id);
    event_emitter.record_internal(
        "turn_started",
        serde_json::json!({
            "runtimeProfile": &runtime_profile,
            "clarificationResume": clarification_resume,
            "decisionResume": decision_resume,
            "previewContinuationDiffId": continuation_preview_diff_id,
        }),
    )?;
    event_emitter.freeze_execution_policy()?;
    event_emitter.freeze_transport_protocol()?;
    let execution_policy = event_emitter.execution_policy()?;
    // AG-1.0-IMP-TASKRUN-01（SM-AG-04 / BR-SYS-STATE-008）：先校验显式 TaskRun 续片段；
    // 普通 AgentTurn 此时不创建 TaskRun。只有后续确定需要 TaskPlan，或工具真实产生任务级产物时，
    // 才按需创建并绑定。应用启动时已清理陈旧活动态（重启不续跑）。
    // AG-1.0-TASKRUN-REWORK-IMP-RUNTIME-01：新消息 gate——存在未决 DecisionRequest 时拒绝新回合
    // （不得绕过待确认，A-AG-M-S-01 §2.2.2；三层分离）。
    let task_run_result = (|| -> Result<Option<String>, IpcError> {
        let conn = state
            .db
            .lock()
            .map_err(|_| IpcError::new("WS_DB_BUSY", "数据库连接繁忙", true, false))?;
        let ws = workspace::lifecycle::active_workspace(&conn)
            .map_err(|e| IpcError::new("WS_ACTIVE_FAILED", &e.to_string(), true, false))?
            .ok_or_else(|| IpcError::new("WS_NO_ACTIVE", "无活动 workspace", false, false))?;
        let task_run_id = if let Some(task_run_id) = continuation_task_run_id.as_deref() {
            match (
                continuation_decision_request_id.as_deref(),
                continuation_preview_diff_id.as_deref(),
            ) {
                (Some(decision_request_id), None) => {
                    ag::task::begin_resolved_decision_continuation(
                        &conn,
                        &ws.workspace_id,
                        &session_id,
                        task_run_id,
                        decision_request_id,
                    )
                    .map_err(|e| IpcError::new("AG_TASK_DB_ERROR", &e.to_string(), true, false))?
                    .ok_or_else(|| {
                        IpcError::new(
                            "AG_TASK_CONTINUATION_INVALID",
                            "裁决确认已失效，无法继续原任务；请重新发起任务。",
                            false,
                            false,
                        )
                    })?
                }
                (None, Some(diff_id)) => {
                    let checkpoint = ag::task::acknowledged_preview_checkpoint(
                        &conn,
                        task_run_id,
                        &ws.workspace_id,
                        &session_id,
                        diff_id,
                    )
                    .map_err(|e| IpcError::new("AG_TASK_DB_ERROR", &e.to_string(), true, false))?
                    .ok_or_else(|| {
                        IpcError::new(
                            "AG_PREVIEW_CONTINUATION_INVALID",
                            "编辑器预览回执已失效或已被消费，无法继续原任务。",
                            false,
                            false,
                        )
                    })?;
                    let fresh_snapshot = open_doc_snapshots.iter().any(|snapshot| {
                        snapshot.rel_path == checkpoint.rel_path
                            && snapshot.session_version == checkpoint.session_version
                            && snapshot.content_hash == checkpoint.content_hash
                    });
                    if !fresh_snapshot {
                        return Err(IpcError::new(
                            "AG_PREVIEW_CONTINUATION_STALE",
                            "续片段未携带编辑器预览后的最新 LogicalState，已拒绝继续。",
                            true,
                            false,
                        ));
                    }
                    ag::task::begin_preview_continuation(&conn, &checkpoint)
                        .map_err(|e| {
                            IpcError::new("AG_TASK_DB_ERROR", &e.to_string(), true, false)
                        })?
                        .ok_or_else(|| {
                            IpcError::new(
                                "AG_PREVIEW_CONTINUATION_INVALID",
                                "编辑器预览回执身份或基线不匹配，无法继续原任务。",
                                false,
                                false,
                            )
                        })?
                }
                _ => {
                    return Err(IpcError::new(
                        "AG_TASK_CONTINUATION_INVALID",
                        "续片段必须且只能提供已确认裁决或已确认预览回执之一。",
                        false,
                        false,
                    ));
                }
            }
            .into()
        } else {
            if continuation_decision_request_id.is_some() || continuation_preview_diff_id.is_some()
            {
                return Err(IpcError::new(
                    "AG_TASK_CONTINUATION_INVALID",
                    "续片段标识不能脱离 TaskRun 使用",
                    false,
                    false,
                ));
            }
            if let Some(request_id) =
                ag::task::latest_pending_decision_for_session(&conn, &ws.workspace_id, &session_id)
                    .map_err(|e| IpcError::new("AG_TASK_DB_ERROR", &e.to_string(), true, false))?
            {
                return Err(IpcError::new(
                    "AG_DECISION_PENDING",
                    &format!(
                        "存在待确认的任务级裁决项（{request_id}），请先完成确认；裁决结果将恢复原 AgentTurn。"
                    ),
                    false,
                    false,
                ));
            }
            clarification_task_run_id.clone()
        };
        Ok(task_run_id)
    })();
    let task_run_id = match task_run_result {
        Ok(task_run_id) => task_run_id,
        Err(error) => {
            // 已确认/已预览的续片段在身份核验阶段失败时，原 TaskRun 已可能处于 waiting/active。
            // 必须使用请求携带的稳定身份收束，不能在建立局部 task_run_id 之前直接返回。
            let task_run_id = continuation_task_run_id
                .as_deref()
                .or(clarification_task_run_id.as_deref());
            settle_and_emit_ag_turn_error_from(
                &state,
                task_run_id,
                &event_emitter,
                &terminal_latch,
                &app,
                &turn_id,
                &error,
                TurnState::Assembling,
            );
            return Ok(());
        }
    };

    // 任何后续装配失败都经同一出口收束 AgentTurn；若已按需创建 TaskRun，同时收束其持久态。
    macro_rules! started_turn_or_finish {
        ($result:expr) => {
            match $result {
                Ok(value) => value,
                Err(error) => {
                    settle_and_emit_ag_turn_error_from(
                        &state,
                        task_run_id.as_deref(),
                        &event_emitter,
                        &terminal_latch,
                        &app,
                        &turn_id,
                        &error,
                        TurnState::Assembling,
                    );
                    return Ok(());
                }
            }
        };
    }

    if let Some(existing_task_run_id) = task_run_id.clone() {
        started_turn_or_finish!((|| {
            let conn = state
                .db
                .lock()
                .map_err(|_| IpcError::new("WS_DB_BUSY", "数据库连接繁忙", true, false))?;
            ag::task::link_turn(&conn, &turn_id, &existing_task_run_id)
                .map_err(|error| IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false))
        })());
    }

    let read_ctx = started_turn_or_finish!(match &agent_context_facts {
        AgentContextFacts::V1(_) => ag::blocks::AgentReadContext::new(
            std::path::Path::new(&workspace_root_path),
            open_doc_snapshots
        ),
        AgentContextFacts::V2(facts) => ag::blocks::AgentReadContext::with_resources(
            std::path::Path::new(&workspace_root_path),
            open_doc_snapshots,
            facts,
            &[],
            false
        ),
    });
    let active_snapshot = started_turn_or_finish!((|| -> Result<_, IpcError> {
        Ok(match active_doc_path.as_deref() {
            Some(path) => {
                if matches!(agent_context_facts, AgentContextFacts::V2(_))
                    && read_ctx.open_snapshot(path).is_none()
                {
                    if active_doc_content.is_some() {
                        return Err(IpcError::new(
                            "AG_ACTIVE_SNAPSHOT_MISMATCH",
                            "未采集文档不得提供旧正文",
                            false,
                            false,
                        ));
                    }
                    return Ok(None);
                }
                let open = read_ctx.open_snapshot(path).ok_or_else(|| {
                    IpcError::new(
                        "AG_OPEN_DOC_SNAPSHOT_REQUIRED",
                        "当前打开文档缺少 LogicalState 快照，已阻断代理回合",
                        true,
                        false,
                    )
                })?;
                if matches!(agent_context_facts, AgentContextFacts::V1(_))
                    && !is_active_editor_snapshot_source(&open.source)
                {
                    return Err(IpcError::new(
                        "AG_ACTIVE_SNAPSHOT_REQUIRED",
                        "当前打开文档必须来自 active editor LogicalState 快照",
                        true,
                        false,
                    ));
                }
                if let Some(format) = active_doc_format.as_deref() {
                    if !format.is_empty() && format != open.edit_format {
                        return Err(IpcError::new(
                        "AG_ACTIVE_FORMAT_MISMATCH",
                        "active_doc_format 与 OpenDocSnapshot.editFormat 不一致，已阻断代理回合",
                        true,
                        false,
                    ));
                    }
                }
                if let Some(legacy_content) = active_doc_content.as_deref() {
                    if legacy_content != open.content {
                        return Err(IpcError::new(
                            "AG_ACTIVE_SNAPSHOT_MISMATCH",
                            "active_doc_content 与 OpenDocSnapshot 不一致，已阻断代理回合",
                            true,
                            false,
                        ));
                    }
                }
                Some(read_ctx.snapshot(path)?)
            }
            None => None,
        })
    })());

    let TurnReferenceRuntime {
        mention_block,
        prompt_envelopes: reference_envelopes,
        deduplicated_count,
    } = build_turn_reference_runtime(&mentions, &block_refs, &references, &read_ctx);
    started_turn_or_finish!(event_emitter.record_internal("reference_projection", serde_json::json!({"deduplicatedCount": deduplicated_count, "reason": "identical_identified_payload_in_same_assembly"})));
    let workspace_root = std::path::Path::new(&workspace_root_path);
    let mut explicit_edit_target_set = if let Some(task_run_id) = task_run_id.as_deref() {
        started_turn_or_finish!((|| {
            let conn = state
                .db
                .lock()
                .map_err(|_| IpcError::new("WS_DB_BUSY", "数据库连接繁忙", true, false))?;
            ag::target_set::load_for_task_run(&conn, task_run_id)
                .map_err(|error| IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false))
        })())
    } else if resume_kind != AgentTurnResumeKind::New {
        registered_edit_target_set
    } else {
        started_turn_or_finish!(ag::target_set::initial_for_turn(
            workspace_root,
            &turn_id,
            active_doc_path.as_deref(),
            &mentions,
            &block_refs,
            &references,
        ))
    };
    if clarification_resume {
        explicit_edit_target_set.merge(started_turn_or_finish!(
            ag::target_set::explicit_references_for_turn(
                workspace_root,
                &turn_id,
                &mentions,
                &block_refs,
                &references,
            )
        ));
    }
    started_turn_or_finish!(bind_agent_turn_edit_target_set(
        &state,
        &session_id,
        &turn_id,
        &explicit_edit_target_set,
    ));
    if let Some(task_run_id) = task_run_id.as_deref() {
        started_turn_or_finish!((|| {
            let conn = state
                .db
                .lock()
                .map_err(|_| IpcError::new("WS_DB_BUSY", "数据库连接繁忙", true, false))?;
            ag::target_set::persist_for_task_run(&conn, task_run_id, &explicit_edit_target_set)
                .map(|_| ())
                .map_err(|error| IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false))
        })());
    }

    // 初始语义只声明 unresolved。关键词 resolver 已退出生产主链；首个模型文本或 ToolCall 的结构化
    // 控制封套形成 revision=1，并作为后续 PromptPackage 的可见语义投影。
    let (intent_result, initial_prompt, mut continuation_messages) =
        match resumed_runtime_checkpoint {
            Some(checkpoint) => (
                checkpoint.intent,
                checkpoint.initial_prompt,
                checkpoint.continuation_messages,
            ),
            None => (
                ag::context::IntentResultV2::unresolved(),
                prompt.clone(),
                Vec::<serde_json::Value>::new(),
            ),
        };
    if clarification_resume {
        continuation_messages.push(serde_json::json!({
            "role": "user",
            "content": prompt,
        }));
    }
    let decision_receipt_ids = continuation_decision_request_id
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    event_emitter.record_internal(
        if intent_result.revision == 0 {
            "intent_initialized"
        } else {
            "intent_resumed"
        },
        serde_json::json!({
            "revision": intent_result.revision,
            "intentType": intent_result.intent_type,
        }),
    )?;
    if decision_resume {
        let decision_request_id = continuation_decision_request_id
            .as_deref()
            .expect("decision resume was validated with a request id");
        let current_task_run_id = task_run_id
            .as_deref()
            .expect("decision resume was validated with a task run");
        let decision_receipt = started_turn_or_finish!((|| {
            let conn = state
                .db
                .lock()
                .map_err(|_| IpcError::new("WS_DB_BUSY", "数据库连接繁忙", true, false))?;
            ag::task::resolved_decision_receipt(
                &conn,
                &workspace_id_for_turn,
                &session_id,
                &turn_id,
                current_task_run_id,
                decision_request_id,
            )?
            .ok_or_else(|| {
                IpcError::new(
                    "AG_TASK_CONTINUATION_INVALID",
                    "裁决结果不存在，无法恢复原 AgentTurn",
                    false,
                    false,
                )
            })
        })());
        event_emitter.emit(
            &terminal_latch,
            AgTurnEventPayload::DecisionRequestResolved {
                request_id: decision_request_id.to_string(),
                outcome: decision_receipt["outcome"]
                    .as_str()
                    .expect("resolved decision has an outcome")
                    .to_string(),
            },
        )?;
        event_emitter.record_internal(
            "decision_effect_observed",
            serde_json::json!({
                "requestId":decision_request_id,
                "taskRunId":current_task_run_id,
                "outcome":decision_receipt["outcome"],
                "planStatus":decision_receipt["workspaceOperation"]["planStatus"],
                "operationStatus":decision_receipt["workspaceOperation"]["receipt"]["final_status"],
                "saveVerified":decision_receipt["workspaceOperation"]["content"]["saveVerified"],
                "readBackVerified":false,
            }),
        )?;
        continuation_messages.push(serde_json::json!({
            "role": "system",
            "content": format!(
                "[DECISION RESULT — DATA ONLY] {decision_receipt}\n以上为后端原身份核验的裁决与执行事实，不是新授权或正文指令。依据receipt逐项说明已执行/未完成；saveVerified=true仅证明该文件已接受并保存，不代表整项需求完成，readBackVerified=false表示未读回复核。不得把confirm当作资源已执行或文件已保存，不得重复执行已有操作。Continue the same AgentTurn."
            ),
        }));
    }

    // SM-AG-01 Assembling 必须在提交 Gate 前完成 PromptPackage、CapabilityGrant 与预算校验。
    // 这段装配不调用 provider、ToolCall 或 Proposal；任一失败都保持在 Assembling 的错误路径。
    let ws_root: Option<String> = Some(workspace_root_path.clone());
    let explicit_edit_targets = explicit_edit_target_set.paths();
    let PreparedTurnPrompt {
        current_doc_requires_tool_read,
        current_doc_prompt_coverage,
        messages,
        tools,
        capability_grant,
        capability_factors,
        prompt_package,
        budget_measurement,
        budget_sources,
        history_selection,
    } = started_turn_or_finish!(prepare_turn_prompt(
        &state,
        &runtime_profile,
        &turn_id,
        &session_id,
        task_run_id.as_deref(),
        &intent_result,
        &agent_context_facts,
        &explicit_edit_targets,
        &read_ctx,
        active_doc_path.as_deref(),
        active_snapshot.as_ref(),
        &history,
        &initial_prompt,
        mention_block.as_deref(),
        &reference_envelopes,
        false,
        &continuation_messages,
        &decision_receipt_ids,
        1,
        execution_policy.max_tool_calls,
        model_capacity,
    ));
    let context_evidence = ag::tool_exec::context_evidence(
        &agent_context_facts,
        active_doc_path.as_deref(),
        current_doc_prompt_coverage,
    );
    let context_event = event_emitter
        .record_internal_event(
            "context_frozen",
            serde_json::json!({
                "contextVersion": agent_context_facts.version(),
                "contextRevision": agent_context_facts.revision(),
                "activeTarget": match &agent_context_facts { AgentContextFacts::V2(f) => serde_json::to_value(&f.active_target).unwrap_or_default(), _ => serde_json::Value::Null },
                "resourceAvailability": match &agent_context_facts { AgentContextFacts::V2(f) => f.open_resources.iter().map(|r| serde_json::json!({"relPath": r.rel_path, "availability": r.availability, "reasonCode": r.reason_code})).collect::<Vec<_>>(), _ => vec![] },
                "openDocCount": agent_context_facts.ready_documents().len(),
                "selectionCount": agent_context_facts.selection_count(),
                "proposalFactCount": agent_context_facts.proposal_count(),
                "evidenceCoverage": context_evidence.iter().map(|item| serde_json::json!({
                    "relPath": item.rel_path,
                    "source": item.source,
                    "coverage": item.coverage,
                    "versionOrHash": item.version_or_hash,
                })).collect::<Vec<_>>(),
            }),
        )?
        .ok_or_else(|| {
            IpcError::new(
                "AG_TURN_ALREADY_TERMINAL",
                "冻结上下文证据时 AgentTurn 已终结",
                false,
                false,
            )
        })?;
    let context_evidence = ag::tool_exec::materialize_evidence(
        &workspace_id_for_turn,
        &turn_id,
        &context_event.event_id,
        context_event.seq,
        &context_event.event_id,
        context_evidence,
    );
    if !context_evidence.is_empty() {
        event_emitter.record_internal(
            "evidence_recorded",
            serde_json::json!({ "evidenceItems": context_evidence }),
        )?;
    }

    // 预算与实际 provider messages/tools 使用同一序列化口径，并在 Gate 前失败关闭。
    started_turn_or_finish!(event_emitter.record_internal("history_assembled", history_selection));
    let budget_reason = ag::prompt_runtime::budget_assembly_reason(
        decision_resume,
        continuation_preview_diff_id.is_some(),
        clarification_resume,
    );
    started_turn_or_finish!(event_emitter.record_prompt_budget(
        budget_reason,
        &budget_measurement,
        budget_sources,
        model_capacity,
    ));
    if budget_measurement.estimated_tokens > model_capacity.input_tokens() {
        let error = IpcError::new(
            "AG_PROMPT_BUDGET_EXCEEDED",
            &ag::prompt_runtime::budget_failure_message(
                &budget_measurement,
                budget_reason,
                model_capacity,
            ),
            true,
            false,
        );
        if let Ok(conn) = state.db.lock() {
            settle_optional_task_run(
                &conn,
                task_run_id.as_deref(),
                &turn_id,
                ag::task::TurnOutcome::Failed,
            );
        }
        emit_ag_turn_error_from(
            &event_emitter,
            &terminal_latch,
            &app,
            &turn_id,
            &error,
            TurnState::Assembling,
        );
        return Ok(());
    }
    eprintln!(
        "[ag-trace] turn_id={} session_id={} profile={} intent_revision={} intent_type={:?} context_trace_id={} tool_manifest_hash={} allowed_tool_count={} estimated_tokens={}",
        turn_id,
        session_id,
        runtime_profile,
        intent_result.revision,
        intent_result.intent_type,
        prompt_package.context_trace_id,
        prompt_package.tool_manifest_hash,
        capability_grant.allowed_tools.len(),
        prompt_package.estimated_tokens,
    );

    let s = started_turn_or_finish!(transition(s, TurnEvent::Assembled).ok_or_else(sm_error)); // → AwaitingGate
    emit_ag01(&app, &turn_id, s);
    if cancelled.load(Ordering::Acquire) {
        if let Ok(conn) = state.db.lock() {
            settle_optional_task_run(
                &conn,
                task_run_id.as_deref(),
                &turn_id,
                ag::task::TurnOutcome::Cancelled,
            );
        }
        emit_ag_turn_cancelled_from(
            &event_emitter,
            &terminal_latch,
            &app,
            &turn_id,
            TurnState::AwaitingGate,
        );
        return Ok(());
    }
    let (_remaining_tools, remaining_rounds) = super::turn_tool_loop::continuation_execution_budget(
        &continuation_messages,
        capability_grant.budgets.remaining_tool_calls,
        execution_policy.max_model_calls,
    );
    let (previous_model_calls, _) = event_emitter.protocol_attempt_counts()?;
    if remaining_rounds == 0 || previous_model_calls >= execution_policy.max_model_calls {
        let error = IpcError::new(
            "AG_TOOL_LOOP_LIMIT",
            "原回合执行预算已用尽，确认或恢复不会重置预算；未启动下一次调用。",
            true,
            false,
        );
        settle_and_emit_ag_turn_error_from(
            &state,
            task_run_id.as_deref(),
            &event_emitter,
            &terminal_latch,
            &app,
            &turn_id,
            &error,
            TurnState::AwaitingGate,
        );
        return Ok(());
    }
    let gate_attempt = previous_model_calls + 1;
    event_emitter.record_internal(
        "gate_requested",
        serde_json::json!({"attempt": gate_attempt}),
    )?;
    let gate = match super::turn_control::await_turn_operation(
        &state,
        &session_id,
        &turn_id,
        ent::gateway::check_gate(),
    )
    .await
    {
        Ok(ent::gateway::GateOutcome::Pass) => GateState::Passed,
        Ok(ent::gateway::GateOutcome::Deny) => GateState::Denied,
        Err(error) if error.code == "AG_TURN_CANCELLED" => {
            event_emitter.record_internal(
                "gate_evaluated",
                serde_json::json!({"attempt": gate_attempt, "outcome": "cancelled"}),
            )?;
            if let Ok(conn) = state.db.lock() {
                settle_optional_task_run(
                    &conn,
                    task_run_id.as_deref(),
                    &turn_id,
                    ag::task::TurnOutcome::Cancelled,
                );
            }
            emit_ag_turn_cancelled_from(
                &event_emitter,
                &terminal_latch,
                &app,
                &turn_id,
                TurnState::AwaitingGate,
            );
            return Ok(());
        }
        Err(error) => {
            event_emitter.record_internal(
                "gate_evaluated",
                serde_json::json!({"attempt": gate_attempt, "outcome": "error", "code": &error.code}),
            )?;
            if let Ok(conn) = state.db.lock() {
                settle_optional_task_run(
                    &conn,
                    task_run_id.as_deref(),
                    &turn_id,
                    ag::task::TurnOutcome::Failed,
                );
            }
            emit_ag_turn_error_from(
                &event_emitter,
                &terminal_latch,
                &app,
                &turn_id,
                &error,
                TurnState::AwaitingGate,
            );
            return Ok(());
        }
    };
    event_emitter.record_internal(
        "gate_evaluated",
        serde_json::json!({
            "attempt": gate_attempt,
            "outcome": if gate == GateState::Passed { "passed" } else { "denied" },
        }),
    )?;
    if gate != GateState::Passed {
        let error = ensure_gate_passed(gate).expect_err("非 Passed 门禁必须被拒绝");
        if let Ok(conn) = state.db.lock() {
            settle_optional_task_run(
                &conn,
                task_run_id.as_deref(),
                &turn_id,
                ag::task::TurnOutcome::Failed,
            );
        }
        emit_ag_turn_denied(&event_emitter, &terminal_latch, &app, &turn_id, &error);
        return Ok(());
    }
    if cancelled.load(Ordering::Acquire) {
        if let Ok(conn) = state.db.lock() {
            settle_optional_task_run(
                &conn,
                task_run_id.as_deref(),
                &turn_id,
                ag::task::TurnOutcome::Cancelled,
            );
        }
        emit_ag_turn_cancelled_from(
            &event_emitter,
            &terminal_latch,
            &app,
            &turn_id,
            TurnState::AwaitingGate,
        );
        return Ok(());
    }
    let s = transition(s, TurnEvent::GatePassed)
        .expect("AwaitingGate + GatePassed must be a valid SM-AG-01 transition"); // → Streaming
    emit_ag01(&app, &turn_id, s);
    debug_assert_eq!(s, TurnState::Streaming);

    drop(event_emitter);
    run_turn_tool_loop(
        state,
        app,
        on_event,
        cancelled,
        terminal_latch,
        turn_id,
        session_id,
        task_run_id,
        workspace_id_for_turn,
        bound_provider,
        bound_model,
        runtime_profile,
        prompt_package,
        agent_context_facts,
        current_doc_prompt_coverage,
        messages,
        tools,
        capability_factors,
        read_ctx,
        active_doc_path,
        explicit_edit_target_set,
        ws_root,
        current_doc_requires_tool_read,
        intent_result,
        continuation_messages,
        history,
        initial_prompt,
        mention_block,
        reference_envelopes,
        active_snapshot,
        on_context_request.0,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context_v2_payload() -> serde_json::Value {
        serde_json::json!({"version": AGENT_CONTEXT_FACTS_VERSION_V2, "workspaceId": "ws", "sessionId": "s", "turnId": "t", "contextRevision": 1,
            "activeTarget": {"relPath": "a.txt", "loadSeq": 1},
            "openResources": [{"relPath": "a.txt", "availability": "loading", "reasonCode": "AG_ACTIVE_SESSION_LOADING", "snapshot": null}], "selections": [], "proposalFacts": []})
    }

    // covers: BR-AG-DATA-008
    // covers: BR-SYS-DATA-008
    #[test]
    fn context_v2_strict_identity_availability_and_version_dispatch() {
        let root = std::env::temp_dir();
        let parse = |v| {
            parse_agent_context_facts(Some(v), "ws", &root, "t", "s", Some("a.txt"), &[], &[], &[])
        };
        let facts = parse(context_v2_payload()).unwrap();
        assert!(matches!(facts, AgentContextFacts::V2(_)));
        assert!(ag::tool_exec::context_evidence(&facts, Some("a.txt"), None).is_empty());
        for (field, value) in [
            ("version", serde_json::json!("unknown")),
            ("sessionId", serde_json::json!("other")),
            ("workspaceId", serde_json::json!("other")),
            ("turnId", serde_json::json!("other")),
            ("contextRevision", serde_json::json!(0)),
            ("activeTarget", serde_json::Value::Null),
        ] {
            let mut invalid = context_v2_payload();
            invalid[field] = value;
            assert!(parse(invalid).is_err(), "{field}");
        }
        for mutate in 0..5 {
            let mut invalid = context_v2_payload();
            match mutate {
                0 => invalid["openResources"][0]["availability"] = "ready".into(),
                1 => invalid["openResources"][0]["reasonCode"] = serde_json::Value::Null,
                2 => {
                    invalid.as_object_mut().unwrap().remove("activeTarget");
                }
                3 => {
                    let r = invalid["openResources"][0].clone();
                    invalid["openResources"].as_array_mut().unwrap().push(r);
                }
                _ => invalid["openResources"][0]["relPath"] = "../outside.txt".into(),
            }
            assert!(parse(invalid).is_err(), "case {mutate}");
        }
    }

    // covers: BR-AG-DATA-001
    // covers: BR-AG-DATA-008
    #[test]
    fn context_v2_missing_open_snapshot_never_reads_disk_even_through_path_alias() {
        let root = std::env::temp_dir().join(format!("binder-v2-read-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let _cleanup = FixedWorkspace(root.clone());
        std::fs::write(root.join("a.txt"), "stale disk content").unwrap();
        std::fs::write(root.join("b.txt"), "closed document").unwrap();
        let facts: AgentContextFactsV2 = serde_json::from_value(context_v2_payload()).unwrap();
        let context = ag::blocks::AgentReadContext::with_resources(
            &root,
            vec![],
            &facts,
            &["./a.txt".into(), "b.txt".into()],
            false,
        )
        .unwrap();
        for path in ["a.txt", "./a.txt"] {
            assert_eq!(
                context.snapshot(path).unwrap_err().code,
                "AG_CONTEXT_RESOURCE_UNAVAILABLE"
            );
        }
        assert_eq!(
            context.snapshot("b.txt").unwrap().content,
            "closed document"
        );
        let unverified =
            ag::blocks::AgentReadContext::with_resources(&root, vec![], &facts, &[], false)
                .unwrap();
        assert_eq!(
            unverified.snapshot("b.txt").unwrap_err().code,
            "AG_CONTEXT_RESOURCE_UNVERIFIED"
        );
    }

    // covers: BR-AG-DATA-008
    // covers: BR-SYS-STATE-007
    #[test]
    fn context_response_rejects_wrong_identity_duplicate_cancelled_and_late_delivery() {
        use std::sync::{Arc, Mutex};
        let facts: AgentContextFactsV2 = serde_json::from_value(context_v2_payload()).unwrap();
        let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let sent = Arc::new(Mutex::new(String::new()));
        let capture = sent.clone();
        let channel = tauri::ipc::Channel::new(move |body| {
            let tauri::ipc::InvokeResponseBody::Json(json) = body else {
                panic!()
            };
            let request: AgentContextRequest = serde_json::from_str(&json).unwrap();
            *capture.lock().unwrap() = request.request_id.clone();
            let mut reply = context_v2_payload();
            reply["contextRevision"] = request.context_revision.into();
            reply["sessionId"] = "wrong".into();
            assert!(ag_supply_context(AgentContextResponse {
                request_id: request.request_id.clone(),
                facts: reply.clone(),
                snapshots: vec![]
            })
            .is_err());
            reply["sessionId"] = "s".into();
            ag_supply_context(AgentContextResponse {
                request_id: request.request_id.clone(),
                facts: reply.clone(),
                snapshots: vec![],
            })
            .unwrap();
            assert!(ag_supply_context(AgentContextResponse {
                request_id: request.request_id,
                facts: reply,
                snapshots: vec![]
            })
            .is_err());
            Ok(())
        });
        tauri::async_runtime::block_on(request_agent_context(
            Some(&channel),
            &facts,
            vec![],
            false,
            cancelled.clone(),
        ))
        .unwrap();
        assert!(!pending_contexts()
            .lock()
            .unwrap()
            .contains_key(&*sent.lock().unwrap()));
        for cancel in [true, false] {
            let captured = sent.clone();
            let flag = cancelled.clone();
            flag.store(false, Ordering::Release);
            let channel = tauri::ipc::Channel::new(move |body| {
                let tauri::ipc::InvokeResponseBody::Json(json) = body else {
                    panic!()
                };
                let request: AgentContextRequest = serde_json::from_str(&json).unwrap();
                *captured.lock().unwrap() = request.request_id;
                flag.store(cancel, Ordering::Release);
                Ok(())
            });
            let error = tauri::async_runtime::block_on(request_agent_context(
                Some(&channel),
                &facts,
                vec![],
                false,
                cancelled.clone(),
            ))
            .unwrap_err();
            assert_eq!(
                error.code,
                if cancel {
                    "AG_CONTEXT_CANCELLED"
                } else {
                    "AG_CONTEXT_TIMEOUT"
                }
            );
            let mut late = context_v2_payload();
            late["contextRevision"] = 2.into();
            assert!(ag_supply_context(AgentContextResponse {
                request_id: sent.lock().unwrap().clone(),
                facts: late,
                snapshots: vec![]
            })
            .is_err());
        }
    }

    // covers: BR-AG-DATA-008
    // covers: BR-AG-STATE-001
    // covers: BR-AG-STATE-002
    #[test]
    fn fixed_runtime_v2_loading_editor_allows_chat_glob_and_only_reads_acquired_logical_state() {
        let chat = run_fixed_turn_with_context(true, vec![fixed_final()], "你好", Some(false));
        assert_eq!(chat.status, "completed");
        assert!(!chat.events.iter().any(|(kind, _)| kind == "tool_request"));
        for ready in [false, true] {
            let run = run_fixed_turn_with_context(
                true,
                vec![
                    fixed_tool(
                        "read_doc",
                        serde_json::json!({"path": "unique-result.txt", "_binderTurn": fixed_control("tool")}),
                    ),
                    fixed_final(),
                ],
                "读取当前文档",
                Some(ready),
            );
            assert_eq!(run.status, "completed");
            let result = run
                .events
                .iter()
                .find(|(kind, _)| kind == "tool_result")
                .unwrap();
            assert_eq!(result.1["success"], ready);
            let requests: Vec<_> = run
                .requests
                .iter()
                .filter(|(path, _)| path == "/invoke/stream")
                .collect();
            assert!(!requests[0].1.to_string().contains("read-only fixture"));
            assert_eq!(
                requests[1].1.to_string().contains("fresh logical fixture"),
                ready
            );
            assert!(!requests[1].1.to_string().contains("read-only fixture"));
        }
        let glob = run_fixed_turn_with_context(
            true,
            vec![
                fixed_tool(
                    "workspace_glob",
                    serde_json::json!({"pattern": "*.txt", "_binderTurn": fixed_control("tool")}),
                ),
                fixed_final(),
            ],
            "列出 txt",
            Some(false),
        );
        assert_eq!(glob.status, "completed");
        assert!(glob
            .events
            .iter()
            .any(|(kind, payload)| kind == "tool_result" && payload["success"] == true));
    }

    // 局部 HTTP 网关只提供外部响应；不替换客户端门禁、解析、授权或领域函数。
    struct FixedGateway {
        url: String,
        requests: std::sync::Arc<std::sync::Mutex<Vec<(String, serde_json::Value)>>>,
        stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
        worker: Option<std::thread::JoinHandle<()>>,
    }

    #[derive(Clone, Copy)]
    enum GateFixtureFault {
        InvalidJson,
        UnknownVerdict,
        Http(u16),
        Disconnect,
        HeadersTimeout,
        BodyTimeout,
        ReauthPassed,
        AuthInvalid,
        AuthDisconnect,
        AuthTimeout,
        AuthHttp,
        Repeated401,
    }

    impl FixedGateway {
        fn start_fault(
            allowed: bool,
            responses: Vec<String>,
            stall_gate: Option<usize>,
            fault: Option<(usize, GateFixtureFault)>,
        ) -> Self {
            use std::io::{Read, Write};
            use std::sync::{Arc, Mutex};
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let url = format!("http://{}", listener.local_addr().unwrap());
            let requests = Arc::new(Mutex::new(Vec::new()));
            let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let recorded = requests.clone();
            let stopped = stop.clone();
            let worker = std::thread::spawn(move || {
                let mut responses = std::collections::VecDeque::from(responses);
                let mut gate_ordinal = 0;
                let mut auth_ordinal = 0;
                while !stopped.load(Ordering::Acquire) {
                    let (mut socket, _) = match listener.accept() {
                        Ok(socket) => socket,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(std::time::Duration::from_millis(2));
                            continue;
                        }
                        Err(error) => panic!("fixture accept failed: {error}"),
                    };
                    // macOS accept 可能继承监听 socket 的非阻塞标志；正文读取使用有期限阻塞 I/O。
                    socket.set_nonblocking(false).unwrap();
                    socket
                        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                        .unwrap();
                    socket
                        .set_write_timeout(Some(std::time::Duration::from_secs(5)))
                        .unwrap();
                    let mut request = Vec::new();
                    let (header_end, content_len) = loop {
                        let mut bytes = [0; 4096];
                        let count = socket.read(&mut bytes).unwrap();
                        assert!(count > 0, "fixture request truncated");
                        request.extend_from_slice(&bytes[..count]);
                        if let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n")
                        {
                            let headers = String::from_utf8_lossy(&request[..end]);
                            let size = headers
                                .lines()
                                .find_map(|line| {
                                    let (name, value) = line.split_once(':')?;
                                    name.eq_ignore_ascii_case("content-length")
                                        .then(|| value.trim().parse::<usize>().unwrap())
                                })
                                .unwrap_or(0);
                            break (end + 4, size);
                        }
                    };
                    while request.len() < header_end + content_len {
                        let mut bytes = [0; 4096];
                        let count = socket.read(&mut bytes).unwrap();
                        assert!(count > 0);
                        request.extend_from_slice(&bytes[..count]);
                    }
                    let header = String::from_utf8_lossy(&request[..header_end]);
                    let path = header.split_whitespace().nth(1).unwrap().to_string();
                    let body: serde_json::Value =
                        serde_json::from_slice(&request[header_end..header_end + content_len])
                            .unwrap();
                    recorded.lock().unwrap().push((path.clone(), body));
                    if path == "/gate/check" {
                        gate_ordinal += 1;
                    }
                    if path == "/auth/login" {
                        auth_ordinal += 1;
                    }
                    let selected = fault.and_then(|(ordinal, kind)| {
                        ((path == "/gate/check"
                            && (ordinal == gate_ordinal
                                || matches!(
                                    kind,
                                    GateFixtureFault::ReauthPassed | GateFixtureFault::Repeated401
                                ) && gate_ordinal >= ordinal))
                            || (path == "/auth/login"
                                && auth_ordinal == 2 * ordinal - 1
                                && matches!(
                                    kind,
                                    GateFixtureFault::AuthInvalid
                                        | GateFixtureFault::AuthDisconnect
                                        | GateFixtureFault::AuthTimeout
                                        | GateFixtureFault::AuthHttp
                                )))
                        .then_some(kind)
                    });
                    if matches!(
                        selected,
                        Some(GateFixtureFault::Disconnect | GateFixtureFault::AuthDisconnect)
                    ) {
                        continue;
                    }
                    let stalled = path == "/gate/check" && stall_gate == Some(gate_ordinal)
                        || matches!(
                            selected,
                            Some(
                                GateFixtureFault::HeadersTimeout
                                    | GateFixtureFault::BodyTimeout
                                    | GateFixtureFault::AuthTimeout
                            )
                        );
                    if matches!(selected, Some(GateFixtureFault::BodyTimeout)) {
                        write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 20\r\nConnection: close\r\n\r\n").unwrap();
                        std::thread::sleep(std::time::Duration::from_millis(400));
                        continue;
                    }
                    if stalled {
                        // Only the external HTTP response is delayed; production cancellation is intact.
                        std::thread::sleep(std::time::Duration::from_millis(400));
                    }
                    let (status, content_type, body) = match path.as_str() {
                        "/auth/login" => (
                            if matches!(selected, Some(GateFixtureFault::AuthHttp)) { "503 Service Unavailable" } else { "200 OK" },
                            "application/json",
                            if matches!(selected, Some(GateFixtureFault::AuthInvalid)) { r#"{"token":" "}"#.into() } else { r#"{"token":"fixture-token"}"#.to_string() },
                        ),
                        "/gate/check" => (
                            match selected { Some(GateFixtureFault::Http(401) | GateFixtureFault::Repeated401) => "401 Unauthorized", Some(GateFixtureFault::Http(403)) => "403 Forbidden",
                                Some(GateFixtureFault::Http(429)) => "429 Too Many Requests", Some(GateFixtureFault::Http(500)) => "500 Internal Server Error",
                                Some(GateFixtureFault::Http(400)) => "400 Bad Request", _ => "200 OK" },
                            "application/json",
                            match selected {
                                Some(GateFixtureFault::InvalidJson) => "not-json".into(),
                                Some(GateFixtureFault::UnknownVerdict) => r#"{"verdict":"surprise"}"#.into(),
                                Some(GateFixtureFault::ReauthPassed) => r#"{"verdict":"passed","reauth_required":true}"#.into(),
                                _ => serde_json::json!({"verdict": if allowed {"passed"} else {"denied"}}).to_string(),
                            },
                        ),
                        "/invoke/stream" => match responses.pop_front() {
                            Some(body) => ("200 OK", "text/event-stream", body),
                            None => (
                                "500 Internal Server Error",
                                "text/plain",
                                "fixture responses exhausted".into(),
                            ),
                        },
                        _ => panic!("unexpected fixture path {path}"),
                    };
                    let written = write!(socket, "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                    if let Err(error) = written {
                        assert!(
                            stalled
                                && matches!(
                                    error.kind(),
                                    std::io::ErrorKind::BrokenPipe
                                        | std::io::ErrorKind::ConnectionReset
                                ),
                            "unexpected fixture response failure: {error}"
                        );
                    }
                }
            });
            Self {
                url,
                requests,
                stop,
                worker: Some(worker),
            }
        }
    }

    impl Drop for FixedGateway {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Release);
            self.worker.take().unwrap().join().unwrap();
        }
    }

    // covers: BR-SYS-STATE-008, BR-AG-DATA-001, BR-AG-STATE-001
    #[test]
    fn preview_continuation_admission_requires_fresh_snapshot_and_preserves_terminal_parent() {
        use std::sync::{Arc, Mutex};
        use tauri::Manager;
        for scenario in ["fresh", "missing-snapshot", "stale-snapshot", "failed-task"] {
            let root =
                std::env::temp_dir().join(format!("binder-preview-entry-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir(&root).unwrap();
            let _cleanup = FixedWorkspace(root.clone());
            std::fs::write(root.join("new.txt"), "DISK").unwrap();
            let mut conn = rusqlite::Connection::open_in_memory().unwrap();
            crate::db::migrate(&mut conn).unwrap();
            let ws =
                workspace::lifecycle::open_workspace(&mut conn, root.to_str().unwrap()).unwrap();
            let run =
                ag::task::begin_turn(&conn, &ws.workspace_id, "preview-session", Some("核对预览"))
                    .unwrap();
            ag::history::save_turn(
                &conn,
                &ws.workspace_id,
                "preview-session",
                "preview-parent",
                Some("new.txt"),
                "completed",
                &serde_json::from_value(serde_json::json!({"version":1,"userMessage":{"id":"preview-user","role":"user","content":"核对预览"},"toolSummaries":[],"diffRefs":[]})).unwrap(),
            )
            .unwrap();
            conn.execute("INSERT INTO pending_diffs (diff_id,workspace_id,rel_path,origin_module,status,payload_ref,created_at) VALUES ('entry-preview',?1,'new.txt','AG','pending','entry-payload',1)",[&ws.workspace_id]).unwrap();
            ag::task::create_preview_checkpoint(
                &conn,
                &run,
                &ws.workspace_id,
                "preview-session",
                "preview-parent",
                "entry-preview",
                "new.txt",
            )
            .unwrap();
            let checkpoint = ag::task::PreviewCheckpoint {
                diff_id: "entry-preview".into(),
                task_run_id: run.clone(),
                workspace_id: ws.workspace_id.clone(),
                session_id: "preview-session".into(),
                turn_id: "preview-parent".into(),
                rel_path: "new.txt".into(),
                session_version: 2,
                content_hash: "preview-hash".into(),
            };
            ag::task::acknowledge_preview_checkpoint(&conn, &checkpoint).unwrap();
            if scenario == "failed-task" {
                ag::task::settle_turn(&conn, &run, ag::task::TurnOutcome::Failed).unwrap();
            }
            let app = tauri::test::mock_builder()
                .build(tauri::test::mock_context(tauri::test::noop_assets()))
                .unwrap();
            app.manage(AppState {
                db: Arc::new(Mutex::new(conn)),
                registry: crate::build_registry(),
                assist_terminals: Arc::new(Mutex::new(std::collections::HashMap::new())),
                active_agent_turns: Arc::new(Mutex::new(std::collections::HashMap::new())),
            });
            let hash = if scenario == "stale-snapshot" {
                "old-hash"
            } else {
                "preview-hash"
            };
            let snapshot = serde_json::json!({"relPath":"new.txt","content":"PREVIEW","source":"active_editor_session","editFormat":"text","contentFormat":"text","dirty":true,"baseline":null,"sessionVersion":2,"contentHash":hash,"capturedAt":1});
            let has_snapshot = scenario != "missing-snapshot";
            let facts = serde_json::json!({"version":AGENT_CONTEXT_FACTS_VERSION_V2,"workspaceId":ws.workspace_id,"sessionId":"preview-session","turnId":"preview-child","contextRevision":1,"activeTarget":{"relPath":"new.txt","loadSeq":1},"openResources":[{"relPath":"new.txt","availability":if has_snapshot {"ready"} else {"not_requested"},"reasonCode":null,"snapshot":if has_snapshot {serde_json::json!({"source":"active_editor_session","editFormat":"text","contentFormat":"text","dirty":true,"baseline":null,"sessionVersion":2,"contentHash":hash})} else {serde_json::Value::Null}}],"selections":[],"proposalFacts":[]});
            let gateway = FixedGateway::start_fault(true, vec![fixed_final()], None, None);
            tauri::async_runtime::block_on(ent::gateway::with_test_gateway(
                gateway.url.clone(),
                run_agent_turn(
                    "preview-child".into(),
                    "preview-session".into(),
                    Some(ag::turn::AGENT_EDIT_1_0_PROFILE.into()),
                    "核对预览".into(),
                    Some("fixture".into()),
                    Some("fixture-model".into()),
                    Some("new.txt".into()),
                    None,
                    Some("text".into()),
                    if has_snapshot {
                        vec![serde_json::from_value(snapshot).unwrap()]
                    } else {
                        vec![]
                    },
                    Some(facts),
                    vec![],
                    vec![],
                    vec![],
                    vec![],
                    Some(run.clone()),
                    None,
                    Some("entry-preview".into()),
                    None,
                    None,
                    tauri::ipc::Channel::new(|_| Ok(())),
                    AgentContextChannel(None),
                    &app.state(),
                    app.handle().clone(),
                ),
            ))
            .unwrap();
            let state = app.state::<AppState>();
            let conn = state.db.lock().unwrap();
            let status: String = conn
                .query_row(
                    "SELECT status FROM agent_turns WHERE turn_id='preview-child'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            let checkpoint_status: String = conn
                .query_row(
                    "SELECT status FROM task_preview_checkpoints WHERE diff_id='entry-preview'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            let calls = gateway.requests.lock().unwrap();
            let records =
                ag::history::list_turns(&conn, &ws.workspace_id, "preview-session").unwrap();
            assert!(
                records
                    .iter()
                    .find(|record| record.turn_id == "preview-child")
                    .unwrap()
                    .internal_continuation
            );
            assert!(
                !records
                    .iter()
                    .find(|record| record.turn_id == "preview-parent")
                    .unwrap()
                    .internal_continuation
            );
            if scenario == "fresh" {
                // The normal frontend owns the display cache, while safe events
                // remain authoritative. Exercise that same cache write contract.
                let events =
                    ag::turn_events::list_safe_events(&conn, &ws.workspace_id, "preview-child", 0)
                        .unwrap();
                let body: String = events
                    .iter()
                    .filter_map(|event| match &event.event {
                        AgTurnEventPayload::Chunk { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect();
                assert!(body.contains("本次检查已结束"));
                let mut projection = records
                    .iter()
                    .find(|record| record.turn_id == "preview-child")
                    .unwrap()
                    .payload
                    .clone();
                projection.last_event_seq = events.last().unwrap().seq;
                projection.assistant_message=Some(serde_json::from_value(serde_json::json!({"id":"preview-assistant","role":"assistant","content":body})).unwrap());
                ag::history::save_turn_projection(
                    &conn,
                    &ws.workspace_id,
                    "preview-session",
                    "preview-child",
                    Some("new.txt"),
                    &projection,
                )
                .unwrap();
                assert!(
                    ag::history::list_turns(&conn, &ws.workspace_id, "preview-session")
                        .unwrap()
                        .iter()
                        .find(|record| record.turn_id == "preview-child")
                        .unwrap()
                        .payload
                        .assistant_message
                        .as_ref()
                        .unwrap()
                        .content
                        .contains("本次检查已结束")
                );
            }
            assert_eq!(
                status,
                if scenario == "fresh" {
                    "completed"
                } else {
                    "error"
                },
                "{scenario}"
            );
            assert_eq!(
                checkpoint_status,
                if scenario == "fresh" {
                    "consumed"
                } else {
                    "acknowledged"
                },
                "{scenario}"
            );
            assert_eq!(
                calls.iter().filter(|(p, _)| p == "/invoke/stream").count(),
                usize::from(scenario == "fresh"),
                "{scenario}"
            );
            assert_eq!(
                conn.query_row(
                    "SELECT status FROM agent_turns WHERE turn_id='preview-parent'",
                    [],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
                "completed"
            );
            assert_eq!(
                std::fs::read_to_string(root.join("new.txt")).unwrap(),
                "DISK"
            );
        }
    }

    fn fixed_control(action: &str) -> serde_json::Value {
        serde_json::json!({
            "version":"binder-turn-control-v1", "nextAction":action,
            "intent":{"goal":"列出 txt 文件", "intentType":"inspect", "targetCandidates":[],
                "sideEffectIntent":"none", "ambiguity":"none", "requiredContext":[], "confirmationPoints":[]},
            "route":{"summary":"检查文件清单", "steps":[]}
        })
    }

    fn fixed_final() -> String {
        fixed_text(fixed_control("final"), "本次检查已结束。")
    }

    // covers: BR-AG-STATE-001
    // covers: BR-AG-STATE-002
    // covers: BR-AG-OBS-001
    #[test]
    fn output_length_freezes_execution_and_delivers_one_failure_baseline_with_actual_capacity() {
        let text = format!(
            "<binder-turn-control>{}</binder-turn-control>",
            fixed_control("tool")
        );
        let response = format!(
            "event: chunk\ndata: {}\n\nevent: done\ndata: {}\n\n",
            serde_json::json!({"text":text}),
            serde_json::json!({"stop_reason":"length","completion_observation":{
                "requestedMaxTokens":8192,"usage":{"completionTokens":8192},
                "pendingToolArguments":[{"toolIndex":0,"argumentsUtf8Bytes":65000,"argumentsSha256":"a".repeat(64),"jsonValid":false}]}})
        );
        let result = run_fixed_turn(true, vec![response]);
        assert_eq!(result.status, "error");
        assert_eq!(
            result.events.iter().filter(|(k, _)| k == "error").count(),
            1
        );
        assert!(!result
            .events
            .iter()
            .any(|(k, _)| matches!(k.as_str(), "tool_request" | "tool_result" | "done")));
        let interaction = &result
            .events
            .iter()
            .find(|(k, _)| k == "interaction_result")
            .unwrap()
            .1;
        assert_eq!(interaction["succeededTools"], 0);
        assert_eq!(interaction["unknownEffects"], 0);
        let capacity = &result
            .events
            .iter()
            .find(|(k, _)| k == "provider_capacity_observed")
            .unwrap()
            .1;
        assert_eq!(capacity["usage"]["completionTokens"], 8192);
        let summaries = result
            .safe_events
            .iter()
            .filter_map(|e| match &e.event {
                AgTurnEventPayload::Chunk { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<String>();
        assert!(summaries.contains("本次任务未完成"));
        assert!(summaries.contains("长度限制"));
        assert_eq!(
            result
                .requests
                .iter()
                .filter(|(p, _)| p == "/invoke/stream")
                .count(),
            1
        );
        assert_eq!(
            result
                .requests
                .iter()
                .find(|(p, _)| p == "/invoke/stream")
                .unwrap()
                .1["max_tokens"],
            8192
        );
    }

    // covers: BR-AG-OBS-001, BR-AG-STATE-001, BR-SYS-STATE-007
    #[test]
    fn normal_stop_keeps_partial_body_and_separates_structure_warning_from_transport_failure() {
        for (body, reason) in [
            ("说明\n\n| 层级 | 能说什", "answer_incomplete"),
            ("完整回答，无须工具。", "stop"),
        ] {
            let result = run_fixed_turn_with_prompt(
                true,
                vec![fixed_text(fixed_control("final"), body)],
                "只讨论概念，不读取或修改文件",
            );
            assert_eq!(result.status, "completed");
            assert_eq!(
                result
                    .events
                    .iter()
                    .filter(|(kind, _)| kind == "done")
                    .count(),
                1
            );
            assert!(!result
                .events
                .iter()
                .any(|(kind, _)| matches!(kind.as_str(), "error" | "tool_request")));
            let text: String = result
                .events
                .iter()
                .filter(|(kind, _)| kind == "chunk")
                .map(|(_, value)| value["text"].as_str().unwrap())
                .collect();
            assert_eq!(text, body);
            assert_eq!(
                result
                    .events
                    .iter()
                    .find(|(kind, _)| kind == "done")
                    .unwrap()
                    .1["stopReason"],
                reason
            );
            let build = &result
                .events
                .iter()
                .find(|(kind, _)| kind == "runtime_build_observed")
                .unwrap()
                .1;
            assert_eq!(build["clientBuildId"].as_str().unwrap().len(), 64);
            let gateway = &result
                .events
                .iter()
                .find(|(kind, _)| kind == "gateway_build_observed")
                .unwrap()
                .1;
            assert_eq!(
                gateway["gatewayBuildPresent"], false,
                "legacy gateway is not silently certified"
            );
            let delivery = &result
                .events
                .iter()
                .find(|(kind, _)| kind == "answer_delivery_observed")
                .unwrap()
                .1;
            assert_eq!(delivery["textSha256"], ag::context::sha256_text(body));
            assert_eq!(
                result
                    .requests
                    .iter()
                    .filter(|(path, _)| path == "/invoke/stream")
                    .count(),
                1,
                "no automatic retry or second chain"
            );
        }
    }

    fn fixed_text(control: serde_json::Value, body: &str) -> String {
        let text = format!("<binder-turn-control>{control}</binder-turn-control>{body}",);
        format!(
            "event: chunk\ndata: {}\n\nevent: done\ndata: {{\"stop_reason\":\"stop\"}}\n\n",
            serde_json::json!({"text":text})
        )
    }

    fn fixed_tool(name: &str, arguments: serde_json::Value) -> String {
        format!(
            "event: tool_calls\ndata: {}\n\n",
            serde_json::json!({"tool_calls":[{
                "id":format!("fixture-{}", uuid::Uuid::new_v4()), "name":name, "arguments":arguments.to_string()
            }]})
        )
    }

    fn fixed_tools(calls: Vec<(&str, serde_json::Value)>) -> String {
        format!(
            "event: tool_calls\ndata: {}\n\n",
            serde_json::json!({"tool_calls":calls.into_iter().enumerate().map(|(index,(name,arguments))|serde_json::json!({
                "id":format!("fixture-group-{index}"),"name":name,"arguments":arguments.to_string()
            })).collect::<Vec<_>>()})
        )
    }

    // covers: BR-AG-STATE-001, BR-AG-STATE-002, BR-AG-OBS-001
    #[test]
    fn rejected_direct_transport_is_audited_but_not_echoed_into_model_correction() {
        let mut rejected_control = fixed_control("tool");
        rejected_control["intent"]["goal"] =
            serde_json::json!("REJECTED_ARGUMENTS_MUST_NOT_RETURN");
        let old_batch = fixed_tool(
            ag::model_output::TOOL_BATCH_NAME,
            serde_json::json!({"control": rejected_control, "calls":[{"toolName":"workspace_glob","params":{"pattern":"*.txt"}}]}),
        );
        let truncated_json = format!(
            "event: tool_calls\ndata: {}\n\n",
            serde_json::json!({"tool_calls":[{"id":"verification-native-truncated",
                "name":"workspace_glob",
                "arguments":"{\"private\":\"REJECTED_ARGUMENTS_MUST_NOT_RETURN\",\"pattern\":"}]}),
        );
        for (rejected, reason) in [
            (old_batch, "transport_protocol_mismatch"),
            (truncated_json, "arguments_not_object"),
        ] {
            let accepted = fixed_tool(
                "workspace_glob",
                serde_json::json!({"pattern":"*.txt","_binderTurn":fixed_control("tool")}),
            );
            let evidence = run_fixed_turn(true, vec![rejected, accepted, fixed_final()]);
            assert_eq!(evidence.status, "completed");
            let requests = evidence
                .requests
                .iter()
                .filter(|(path, _)| path == "/invoke/stream")
                .collect::<Vec<_>>();
            assert_eq!(requests.len(), 3);
            let correction = requests[1].1["messages"].to_string();
            assert!(correction.contains("上一响应的控制封套或工具参数被拒绝"));
            assert!(correction.contains(reason));
            assert!(!correction.contains("REJECTED_ARGUMENTS_MUST_NOT_RETURN"));
            assert_eq!(
                evidence
                    .safe_events
                    .iter()
                    .filter(|event| matches!(
                        &event.event,
                        AgTurnEventPayload::ToolResult { success: true, .. }
                    ))
                    .count(),
                1,
                "only the corrected business call executes"
            );
            assert!(evidence
                .events
                .iter()
                .any(|(kind, _)| kind == "model_control_rejected"));
        }
    }

    // covers: BR-AG-STATE-002, BR-AG-DATA-004, BR-AG-DATA-005
    #[test]
    fn fixed_runtime_direct_tools_execute_independent_reads_with_individual_provider_replies() {
        let evidence = run_fixed_turn(
            true,
            vec![
                fixed_tools(vec![
                    (
                        "workspace_glob",
                        serde_json::json!({"pattern":"*.txt","_binderTurn":fixed_control("tool")}),
                    ),
                    ("list_docs", serde_json::json!({})),
                ]),
                fixed_final(),
            ],
        );
        assert_eq!(evidence.status, "completed");
        assert_eq!(
            evidence
                .safe_events
                .iter()
                .filter(|e| matches!(
                    e.event,
                    AgTurnEventPayload::ToolResult { success: true, .. }
                ))
                .count(),
            2
        );
        let invokes = evidence
            .requests
            .iter()
            .filter(|(path, _)| path == "/invoke/stream")
            .collect::<Vec<_>>();
        assert_eq!(invokes.len(), 2);
        let exposed = invokes[0].1["tools"].as_array().unwrap();
        assert!(exposed
            .iter()
            .any(|tool| tool["function"]["name"] == "workspace_glob"));
        assert!(exposed
            .iter()
            .any(|tool| tool["function"]["name"] == "list_docs"));
        assert!(exposed
            .iter()
            .all(|tool| tool["function"]["name"] != ag::model_output::TOOL_BATCH_NAME));
        let replies = invokes[1].1["messages"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|message| message["role"] == "tool")
            .collect::<Vec<_>>();
        assert_eq!(replies.len(), 2);
        assert!(replies.iter().any(|reply| reply["content"]
            .as_str()
            .unwrap()
            .contains("unique-result.txt")));
        assert!(replies.iter().all(|reply| reply["tool_call_id"]
            .as_str()
            .unwrap()
            .starts_with("fixture-group-")));
        assert!(!evidence
            .events
            .iter()
            .any(|(kind, _)| kind == "model_control_rejected"));
    }

    // covers: BR-AG-STATE-002, BR-AG-DATA-004, BR-AG-OBS-001, BR-SYS-STATE-007
    #[test]
    fn fixed_runtime_direct_rejection_corrects_on_same_chain_or_stops_at_shared_limit() {
        let native = |args| fixed_tool("list_docs", args);
        let first = native(serde_json::json!({"_binderTurn":fixed_control("tool")}));
        let missing = native(serde_json::json!({}));
        let wrong_type = native(serde_json::json!({"_binderTurn":"PRIVATE_BAD_CALL"}));
        let recovered = fixed_tools(vec![
            (
                "workspace_glob",
                serde_json::json!({"pattern":"*.txt","_binderTurn":fixed_control("tool")}),
            ),
            ("list_docs", serde_json::json!({})),
        ]);
        for (last, expected_status, success_count, model_count) in [
            (vec![recovered, fixed_final()], "completed", 3, 5),
            (vec![native(serde_json::json!({}))], "error", 1, 4),
        ] {
            let mut responses = vec![first.clone(), missing.clone(), wrong_type.clone()];
            responses.extend(last);
            let evidence = run_fixed_turn(true, responses);
            assert_eq!(evidence.status, expected_status);
            let invokes = evidence
                .requests
                .iter()
                .filter(|(p, _)| p == "/invoke/stream")
                .collect::<Vec<_>>();
            assert_eq!(invokes.len(), model_count);
            for (_, request) in &invokes {
                assert!(request["tools"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|tool| tool["function"]["name"] != ag::model_output::TOOL_BATCH_NAME));
                for message in request["messages"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|m| m["role"] == "tool")
                {
                    assert!(!message["content"]
                        .as_str()
                        .unwrap()
                        .contains("PRIVATE_BAD_CALL"));
                }
            }
            assert_eq!(
                evidence
                    .safe_events
                    .iter()
                    .filter(|e| matches!(
                        e.event,
                        AgTurnEventPayload::ToolResult { success: true, .. }
                    ))
                    .count(),
                success_count
            );
            let rejections = evidence
                .events
                .iter()
                .filter(|(k, _)| k == "model_control_rejected")
                .map(|(_, v)| v)
                .collect::<Vec<_>>();
            assert_eq!(
                rejections.len(),
                if expected_status == "error" { 3 } else { 2 }
            );
            assert_eq!(
                rejections[0]["protocolDiagnostic"]["fieldPath"],
                "_binderTurn"
            );
            assert_eq!(rejections[1]["protocolDiagnostic"]["fieldPath"], "$control");
            for (index, rejection) in rejections.iter().enumerate() {
                assert_eq!(rejection["correctionAttempt"], index + 1);
                assert!(!rejection.to_string().contains("PRIVATE"));
            }
            assert_eq!(
                evidence
                    .safe_events
                    .iter()
                    .filter(|e| matches!(
                        e.event,
                        AgTurnEventPayload::Done { .. } | AgTurnEventPayload::Error { .. }
                    ))
                    .count(),
                1
            );
            if expected_status == "error" {
                assert!(evidence.safe_events.iter().any(|e|matches!(&e.event,AgTurnEventPayload::Error{code,..} if code=="AG_MODEL_CORRECTION_LIMIT")));
            } else {
                let replies = invokes.last().unwrap().1["messages"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|m| m["role"] == "tool")
                    .collect::<Vec<_>>();
                // Rejected, never-executed transport calls remain in the event ledger,
                // not in the next model request as malformed examples.
                assert_eq!(replies.len(), 3);
                assert!(replies.iter().all(|reply| {
                    !reply.to_string().contains("AG_MODEL_CONTROL_INVALID")
                        && !reply.to_string().contains("PRIVATE_BAD_CALL")
                }));
                assert!(replies.iter().any(|reply| reply["content"]
                    .as_str()
                    .unwrap()
                    .contains("unique-result.txt")));
            }
        }
    }

    // covers: BR-AG-STATE-002, BR-AG-DATA-004, BR-AG-OBS-001
    #[test]
    fn fixed_runtime_three_tool_descriptions_keep_one_control_and_execute_real_reads() {
        let mut calls = Vec::new();
        for (index, name, args) in [
            (0, "list_docs", serde_json::json!({})),
            (1, "workspace_glob", serde_json::json!({"pattern":"*.txt"})),
            (2, "workspace_glob", serde_json::json!({"pattern":"*.md"})),
        ] {
            let mut control = fixed_control("tool");
            control["intent"]["goal"] = serde_json::json!(format!("PRIVATE_GOAL_{index}"));
            control["route"]["summary"] = serde_json::json!(format!("PRIVATE_ROUTE_{index}"));
            control["route"]["steps"] = serde_json::json!([format!("PRIVATE_STEP_{index}")]);
            let mut args = args;
            args["_binderTurn"] = control;
            calls.push(serde_json::json!({"id":format!("compat-{index}"),"name":name,"arguments":args.to_string()}));
        }
        let batch = format!(
            "event: tool_calls\ndata: {}\n\n",
            serde_json::json!({"tool_calls":calls})
        );
        let evidence = run_fixed_turn(true, vec![batch, fixed_final()]);
        assert_eq!(evidence.status, "completed");
        assert_eq!(
            evidence
                .safe_events
                .iter()
                .filter(|e| matches!(
                    e.event,
                    AgTurnEventPayload::ToolResult { success: true, .. }
                ))
                .count(),
            3
        );
        assert!(!evidence
            .events
            .iter()
            .any(|(k, _)| k == "model_control_rejected"));
        let reconciled = &evidence
            .events
            .iter()
            .find(|(k, _)| k == "model_control_description_reconciled")
            .unwrap()
            .1;
        assert_eq!(reconciled["observations"].as_array().unwrap().len(), 2);
        assert!(!reconciled.to_string().contains("PRIVATE"));
        let first_projection = evidence
            .events
            .iter()
            .find(|(k, _)| k == "intent_updated")
            .unwrap();
        assert_eq!(first_projection.1["projection"]["goal"], "PRIVATE_GOAL_0");
        let invokes: Vec<_> = evidence
            .requests
            .iter()
            .filter(|(p, _)| p == "/invoke/stream")
            .collect();
        assert_eq!(invokes.len(), 2);
        assert_eq!(
            invokes[1].1["messages"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|m| m["role"] == "tool")
                .count(),
            3
        );
    }

    // covers: BR-AG-STATE-002, BR-AG-OBS-001, BR-SYS-STATE-007
    #[test]
    fn fixed_runtime_correction_budget_is_cumulative_across_success_with_precise_rejections() {
        let mut invalid = fixed_control("tool");
        invalid["intent"]["ambiguity"] = serde_json::json!("blocking");
        let bad = fixed_tool("list_docs", serde_json::json!({"_binderTurn":invalid}));
        let success = fixed_tool(
            "workspace_glob",
            serde_json::json!({"pattern":"*.txt","_binderTurn":fixed_control("tool")}),
        );
        let mut invalid = fixed_control("final");
        invalid["intent"]["goal"] = serde_json::json!("PRIVATE_VALUE");
        let bad_after_success = fixed_tool("list_docs", serde_json::json!({"_binderTurn":invalid}));
        let mut conflicting = fixed_control("tool");
        conflicting["PRIVATE_UNKNOWN_FIELD"] = serde_json::json!("PRIVATE_VALUE");
        let final_batch = format!(
            "event: tool_calls\ndata: {}\n\n",
            serde_json::json!({"tool_calls":[
                {"id":"last-first","name":"workspace_glob","arguments":serde_json::json!({"pattern":"*","_binderTurn":fixed_control("tool")}).to_string()},
                {"id":"last-second","name":"workspace_glob","arguments":serde_json::json!({"pattern":"*","_binderTurn":conflicting}).to_string()}
            ]})
        );
        let evidence = run_fixed_turn(true, vec![bad, success, bad_after_success, final_batch]);
        assert_eq!(evidence.status, "error");
        let rejections: Vec<_> = evidence
            .events
            .iter()
            .filter(|(k, _)| k == "model_control_rejected")
            .map(|(_, v)| v)
            .collect();
        assert_eq!(rejections.len(), 3);
        for (index, reason) in [
            "action_ambiguity_conflict",
            "tool_action_required",
            "schema_unknown_field",
        ]
        .iter()
        .enumerate()
        {
            assert_eq!(rejections[index]["correctionAttempt"], index + 1);
            assert_eq!(
                rejections[index]["protocolDiagnostic"]["reasonCode"],
                *reason
            );
            assert!(!rejections[index].to_string().contains("PRIVATE"));
        }
        assert_eq!(
            evidence
                .safe_events
                .iter()
                .filter(|e| matches!(
                    e.event,
                    AgTurnEventPayload::ToolResult { success: true, .. }
                ))
                .count(),
            1
        );
        assert_eq!(
            evidence
                .requests
                .iter()
                .filter(|(p, _)| p == "/invoke/stream")
                .count(),
            4
        );
        assert!(evidence.safe_events.iter().any(|e|matches!(&e.event,AgTurnEventPayload::Error {code,message,diagnostic,..} if code=="AG_MODEL_CORRECTION_LIMIT" && message.contains("控制封套拒绝") && diagnostic.as_ref().is_some_and(|d|d.category=="model_protocol"))));
    }

    // covers: BR-AG-STATE-002
    // covers: BR-SYS-STATE-007
    #[test]
    fn fixed_runtime_duplicate_tool_identity_never_reexecutes_or_partially_runs_batch() {
        let arguments = serde_json::json!({"pattern":"*.txt", "_binderTurn":fixed_control("tool")});
        let call = fixed_tool("workspace_glob", arguments.clone());
        let duplicated_batch = format!(
            "event: tool_calls\ndata: {}\n\n",
            serde_json::json!({"tool_calls":[
                {"id":"duplicate", "name":"workspace_glob", "arguments":arguments.to_string()},
                {"id":"duplicate", "name":"workspace_glob", "arguments":arguments.to_string()}
            ]})
        );
        let empty_id = format!(
            "event: tool_calls\ndata: {}\n\n",
            serde_json::json!({"tool_calls":[
                {"id":" ", "name":"workspace_glob", "arguments":arguments.to_string()}
            ]})
        );
        for (responses, expected_tools, expected_code) in [
            (vec![call.clone(), call], 1, "AG_MODEL_TOOL_ID_INVALID"),
            (vec![duplicated_batch], 0, "AG_GW_STREAM_INVALID_JSON"),
            (vec![empty_id], 0, "AG_GW_STREAM_INVALID_JSON"),
        ] {
            let evidence = run_fixed_turn(true, responses);
            assert_eq!(evidence.status, "error");
            assert_eq!(
                evidence
                    .safe_events
                    .iter()
                    .filter(|event| matches!(&event.event, AgTurnEventPayload::ToolResult { .. }))
                    .count(),
                expected_tools
            );
            assert_eq!(
                evidence
                    .safe_events
                    .iter()
                    .filter(|event| matches!(&event.event, AgTurnEventPayload::ToolRequest { .. }))
                    .count(),
                expected_tools
            );
            assert!(evidence
                .safe_events
                .iter()
                .any(|event| matches!(&event.event,
                AgTurnEventPayload::Error { code, diagnostic, .. } if code == expected_code
                    && diagnostic.as_ref().unwrap().category == "model_protocol")));
            assert_eq!(
                evidence
                    .requests
                    .iter()
                    .filter(|(path, _)| path == "/invoke/stream")
                    .count(),
                expected_tools + 1
            );
        }
    }

    struct RuntimeEvidence {
        status: String,
        events: Vec<(String, serde_json::Value)>,
        requests: Vec<(String, serde_json::Value)>,
        safe_events: Vec<AgTurnEvent>,
        workspace_files: Vec<String>,
    }

    struct FixedWorkspace(std::path::PathBuf);

    impl Drop for FixedWorkspace {
        fn drop(&mut self) {
            if let Err(error) = std::fs::remove_dir_all(&self.0) {
                eprintln!("[ag-fixed-test] temporary workspace cleanup failed: {error}");
                if !std::thread::panicking() {
                    panic!("temporary workspace cleanup failed");
                }
            }
        }
    }

    fn run_fixed_turn(allowed: bool, responses: Vec<String>) -> RuntimeEvidence {
        run_fixed_turn_with_prompt(allowed, responses, "列出工作区的 txt 文件")
    }

    fn run_fixed_turn_with_prompt(
        allowed: bool,
        responses: Vec<String>,
        prompt: &str,
    ) -> RuntimeEvidence {
        run_fixed_turn_with_context(allowed, responses, prompt, None)
    }

    fn run_fixed_turn_with_context(
        allowed: bool,
        responses: Vec<String>,
        prompt: &str,
        v2_ready: Option<bool>,
    ) -> RuntimeEvidence {
        run_fixed_turn_with_file(allowed, responses, prompt, v2_ready, "read-only fixture")
    }

    fn run_fixed_turn_with_file(
        allowed: bool,
        responses: Vec<String>,
        prompt: &str,
        v2_ready: Option<bool>,
        file_content: &str,
    ) -> RuntimeEvidence {
        run_fixed_turn_with_gate_cancel(allowed, responses, prompt, v2_ready, file_content, None)
    }

    fn run_fixed_turn_with_gate_cancel(
        allowed: bool,
        responses: Vec<String>,
        prompt: &str,
        v2_ready: Option<bool>,
        file_content: &str,
        cancel_on_gate: Option<usize>,
    ) -> RuntimeEvidence {
        run_fixed_turn_with_gate_fault(
            allowed,
            responses,
            prompt,
            v2_ready,
            file_content,
            cancel_on_gate,
            None,
        )
    }

    fn run_fixed_turn_with_gate_fault(
        allowed: bool,
        responses: Vec<String>,
        prompt: &str,
        v2_ready: Option<bool>,
        file_content: &str,
        cancel_on_gate: Option<usize>,
        invalid_gate: Option<usize>,
    ) -> RuntimeEvidence {
        run_fixed_turn_with_transport_fault(
            allowed,
            responses,
            prompt,
            v2_ready,
            file_content,
            cancel_on_gate,
            invalid_gate.map(|n| (n, GateFixtureFault::InvalidJson)),
        )
    }

    fn run_fixed_turn_with_transport_fault(
        allowed: bool,
        responses: Vec<String>,
        prompt: &str,
        v2_ready: Option<bool>,
        file_content: &str,
        cancel_on_gate: Option<usize>,
        fault: Option<(usize, GateFixtureFault)>,
    ) -> RuntimeEvidence {
        run_fixed_turn_with_history(
            allowed,
            responses,
            prompt,
            v2_ready,
            file_content,
            cancel_on_gate,
            fault,
            vec![],
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn run_fixed_turn_with_history(
        allowed: bool,
        responses: Vec<String>,
        prompt: &str,
        v2_ready: Option<bool>,
        file_content: &str,
        cancel_on_gate: Option<usize>,
        fault: Option<(usize, GateFixtureFault)>,
        prior_messages: Vec<(&str, String)>,
    ) -> RuntimeEvidence {
        run_fixed_turn_with_policy(
            allowed,
            responses,
            prompt,
            v2_ready,
            file_content,
            cancel_on_gate,
            fault,
            prior_messages,
            ag::tool_registry::ExecutionBudgetPolicy::legacy(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn run_fixed_turn_with_policy(
        allowed: bool,
        responses: Vec<String>,
        prompt: &str,
        v2_ready: Option<bool>,
        file_content: &str,
        cancel_on_gate: Option<usize>,
        fault: Option<(usize, GateFixtureFault)>,
        prior_messages: Vec<(&str, String)>,
        policy: ag::tool_registry::ExecutionBudgetPolicy,
    ) -> RuntimeEvidence {
        run_fixed_turn_with_policy_and_files(
            allowed,
            responses,
            prompt,
            v2_ready,
            file_content,
            cancel_on_gate,
            fault,
            prior_messages,
            policy,
            &[],
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn run_fixed_turn_with_policy_and_files(
        allowed: bool,
        responses: Vec<String>,
        prompt: &str,
        v2_ready: Option<bool>,
        file_content: &str,
        cancel_on_gate: Option<usize>,
        fault: Option<(usize, GateFixtureFault)>,
        prior_messages: Vec<(&str, String)>,
        policy: ag::tool_registry::ExecutionBudgetPolicy,
        extra_files: &[(&str, &str)],
    ) -> RuntimeEvidence {
        use std::sync::{Arc, Mutex};
        use tauri::Manager;
        let root = std::env::temp_dir().join(format!("binder-fixed-turn-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let _workspace_cleanup = FixedWorkspace(root.clone());
        // 从真实目录取证，响应夹具中不提供这些结果。
        std::fs::write(root.join("unique-result.txt"), file_content).unwrap();
        std::fs::write(root.join("excluded.md"), "not a txt").unwrap();
        for (path, content) in extra_files {
            std::fs::write(root.join(path), content).unwrap();
        }
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::migrate(&mut conn).unwrap();
        let ws = workspace::lifecycle::open_workspace(&mut conn, root.to_str().unwrap()).unwrap();
        let history = prior_messages
            .into_iter()
            .map(|(role, content)| ag::context::HistTurn {
                workspace_id: ws.workspace_id.clone(),
                session_id: "fixed-session".into(),
                role: role.into(),
                content,
                rel_path: v2_ready.map(|_| "unique-result.txt".into()),
                turn_id: Some("prior-turn".into()),
            })
            .collect();
        let app = tauri::test::mock_builder()
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        app.manage(AppState {
            db: std::sync::Arc::new(Mutex::new(conn)),
            registry: crate::build_registry(),
            assist_terminals: std::sync::Arc::new(Mutex::new(std::collections::HashMap::new())),
            active_agent_turns: Arc::new(Mutex::new(std::collections::HashMap::new())),
        });
        let gateway = FixedGateway::start_fault(allowed, responses, cancel_on_gate, fault);
        let facts = AgentContextFactsV1 {
            version: AGENT_CONTEXT_FACTS_VERSION_V1.into(),
            workspace_id: ws.workspace_id.clone(),
            turn_id: "fixed-turn".into(),
            active_doc: None,
            open_docs: vec![],
            selections: vec![],
            proposal_facts: vec![],
        };
        let v2 = serde_json::json!({"version": AGENT_CONTEXT_FACTS_VERSION_V2, "workspaceId": ws.workspace_id,
            "sessionId": "fixed-session", "turnId": "fixed-turn", "contextRevision": 1,
            "activeTarget": {"relPath": "unique-result.txt", "loadSeq": 1},
            "openResources": [{"relPath": "unique-result.txt", "availability": "loading", "reasonCode": "AG_ACTIVE_SESSION_LOADING", "snapshot": null}],
            "selections": [], "proposalFacts": []});
        let payload = if v2_ready.is_some() {
            v2.clone()
        } else {
            serde_json::to_value(facts).unwrap()
        };
        let context_channel = v2_ready.map(|ready| tauri::ipc::Channel::new(move |body| {
            let tauri::ipc::InvokeResponseBody::Json(json) = body else { panic!("expected context JSON") };
            let request: AgentContextRequest = serde_json::from_str(&json).unwrap();
            let mut facts = v2.clone();
            facts["contextRevision"] = request.context_revision.into();
            let snapshots = if ready {
                facts["openResources"][0]["availability"] = "ready".into();
                facts["openResources"][0]["reasonCode"] = serde_json::Value::Null;
                facts["openResources"][0]["snapshot"] = serde_json::json!({"source": "active_editor_session", "editFormat": "text", "contentFormat": "text", "dirty": true, "baseline": null, "sessionVersion": 2, "contentHash": "fresh-v2"});
                vec![serde_json::from_value(serde_json::json!({"relPath": "unique-result.txt", "content": "fresh logical fixture", "source": "active_editor_session", "editFormat": "text", "contentFormat": "text", "dirty": true, "baseline": null, "sessionVersion": 2, "contentHash": "fresh-v2", "capturedAt": 1})).unwrap()]
            } else { vec![] };
            ag_supply_context(AgentContextResponse { request_id: request.request_id, facts, snapshots }).unwrap();
            Ok(())
        }));
        let projected = Arc::new(Mutex::new(Vec::<AgTurnEvent>::new()));
        let sink = projected.clone();
        let channel = tauri::ipc::Channel::new(move |body| {
            let tauri::ipc::InvokeResponseBody::Json(json) = body else {
                panic!("expected JSON event")
            };
            sink.lock()
                .unwrap()
                .push(serde_json::from_str(&json).unwrap());
            Ok(())
        });
        let execution = ent::gateway::with_test_gateway(gateway.url.clone(), async {
            let turn = ag_start_turn(
                "fixed-turn".into(),
                "fixed-session".into(),
                Some(ag::turn::AGENT_EDIT_1_0_PROFILE.into()),
                prompt.into(),
                Some("fixture".into()),
                Some("fixture-model".into()),
                v2_ready.map(|_| "unique-result.txt".to_string()),
                None,
                None,
                vec![],
                Some(payload),
                history,
                vec![],
                vec![],
                vec![],
                None,
                None,
                None,
                None,
                None,
                channel,
                AgentContextChannel(context_channel),
                app.state(),
                app.handle().clone(),
            );
            if let Some(ordinal) = cancel_on_gate {
                let cancellation = async {
                    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
                    loop {
                        let observed = gateway
                            .requests
                            .lock()
                            .unwrap()
                            .iter()
                            .filter(|(path, _)| path == "/gate/check")
                            .count();
                        if observed >= ordinal {
                            break;
                        }
                        assert!(
                            std::time::Instant::now() < deadline,
                            "expected Gate request was not sent"
                        );
                        tauri::async_runtime::spawn_blocking(|| {
                            std::thread::sleep(std::time::Duration::from_millis(1));
                        })
                        .await
                        .expect("fixture polling worker failed");
                    }
                    super::super::turn_control::cancel_active_agent_turn(
                        &app.state::<AppState>(),
                        "fixed-session",
                        "fixed-turn",
                    )
                    .unwrap();
                };
                futures_util::future::join(turn, cancellation).await.0
            } else {
                turn.await
            }
        });
        let timed = matches!(
            fault,
            Some((
                _,
                GateFixtureFault::HeadersTimeout
                    | GateFixtureFault::BodyTimeout
                    | GateFixtureFault::AuthTimeout
            ))
        );
        let execution = ag::tool_registry::with_verification_execution_policy(policy, execution);
        if timed {
            tauri::async_runtime::block_on(ent::gateway::with_test_gateway_deadline(
                std::time::Duration::from_millis(200),
                execution,
            ))
            .unwrap();
        } else {
            tauri::async_runtime::block_on(execution).unwrap();
        }
        let state = app.state::<AppState>();
        let conn = state.db.lock().unwrap();
        let status: String = conn
            .query_row(
                "SELECT status FROM agent_turns WHERE turn_id='fixed-turn'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let events = conn.prepare("SELECT kind,payload_json FROM agent_turn_events WHERE turn_id='fixed-turn' ORDER BY seq").unwrap()
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).unwrap()
            .map(|row| { let (kind, json) = row.unwrap(); (kind, serde_json::from_str(&json).unwrap()) }).collect();
        let replay =
            super::super::turn_events::list_safe_events(&conn, &ws.workspace_id, "fixed-turn", 0)
                .unwrap();
        let safe_events = projected.lock().unwrap().clone();
        if status == "draft" || status == "clarifying" {
            assert_eq!(
                state.active_agent_turns.lock().unwrap().len(),
                1,
                "waiting turn must retain exactly one resumable lease"
            );
            assert!(
                safe_events.iter().any(|event| matches!(
                    event.event,
                    AgTurnEventPayload::ClarificationRequired { .. }
                        | AgTurnEventPayload::DecisionRequestProposed { .. }
                )),
                "retained lease must have a real waiting event"
            );
        } else {
            assert!(
                state.active_agent_turns.lock().unwrap().is_empty(),
                "finished turn must release active lease"
            );
        }
        assert_eq!(
            serde_json::to_value(&replay).unwrap(),
            serde_json::to_value(&safe_events).unwrap()
        );
        assert_eq!(
            safe_events
                .iter()
                .filter(|event| event.event.terminal_status().is_some())
                .count(),
            if status == "draft" || status == "clarifying" {
                0
            } else {
                1
            }
        );
        assert_eq!(
            std::fs::read_to_string(root.join("unique-result.txt")).unwrap(),
            file_content
        );
        assert_eq!(
            conn.query_row("SELECT count(*) FROM pending_diffs", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        let requests = gateway.requests.lock().unwrap().clone();
        let workspace_files = std::fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        drop(conn);
        RuntimeEvidence {
            status,
            events,
            requests,
            safe_events,
            workspace_files,
        }
    }

    // covers: BR-AG-STATE-001
    // covers: BR-AG-STATE-002
    // covers: BR-AG-DATA-004
    #[test]
    fn fixed_runtime_action_handoff_corrects_before_prose_and_supports_explicit_stages() {
        // Real core + temporary workspace/database + fixed HTTP transport. This
        // replays the observed empty-create -> unframed reply -> corrected plan.
        let create = |content: &str| {
            let mut control = fixed_control("tool");
            control["intent"]["intentType"] = serde_json::json!("organize_workspace");
            control["intent"]["sideEffectIntent"] = serde_json::json!("structure_plan");
            fixed_tool(
                "plan_workspace_operation",
                serde_json::json!({
                    "operations":[{"kind":"create_document","targetPath":"empty.html","resourceKind":"file"}],
                    "initialContent":content,
                    "_binderTurn":control
                }),
            )
        };
        let unframed = format!(
            "event: chunk\ndata: {}\n\nevent: done\ndata: {{\"stop_reason\":\"stop\"}}\n\n",
            serde_json::json!({"text":"UNFRAMED_FAILURE_EXPLANATION"})
        );
        let evidence = run_fixed_turn_with_prompt(true, vec![
            create(""), unframed,
            create("<!DOCTYPE html><html><head><meta charset=\"utf-8\"></head><body></body></html>"),
        ], "新建一个空HTML文档empty.html，先生成确认卡，不要自动确认、接受或保存");
        assert_eq!(evidence.status, "draft");
        assert!(
            !evidence
                .workspace_files
                .iter()
                .any(|path| path == "empty.html"),
            "preparing a confirmation must not create the HTML file"
        );
        assert_eq!(
            evidence
                .safe_events
                .iter()
                .filter(|event| matches!(
                    event.event,
                    AgTurnEventPayload::DecisionRequestProposed { .. }
                ))
                .count(),
            1
        );
        assert!(!evidence.safe_events.iter().any(|event| matches!(
            event.event,
            AgTurnEventPayload::Done { .. } | AgTurnEventPayload::Error { .. }
        )));
        assert!(!serde_json::to_string(&evidence.safe_events)
            .unwrap()
            .contains("UNFRAMED_FAILURE_EXPLANATION"));
        let failed = evidence
            .safe_events
            .iter()
            .find_map(|event| match &event.event {
                AgTurnEventPayload::ToolResult {
                    success: false,
                    execution_receipt,
                    ..
                } => execution_receipt.as_ref(),
                _ => None,
            })
            .unwrap();
        let receipt = serde_json::to_value(failed).unwrap();
        assert_eq!(receipt["executionStatus"], "rejected");
        assert_eq!(receipt["completionStatus"], "blocked");
        assert_eq!(receipt["effectStatus"], "not_started");
        assert_eq!(receipt["failure"]["code"], "AG_TOOL_ARG");
        assert_eq!(receipt["failure"]["retriable"], true);
        let waiting = evidence
            .safe_events
            .iter()
            .find_map(|event| match &event.event {
                AgTurnEventPayload::ToolResult {
                    success: true,
                    execution_receipt,
                    ..
                } => execution_receipt.as_ref(),
                _ => None,
            })
            .unwrap();
        let waiting = serde_json::to_value(waiting).unwrap();
        assert_eq!(waiting["completionStatus"], "waiting");
        assert_eq!(waiting["effectStatus"], "not_started");
        let rejection = &evidence
            .events
            .iter()
            .find(|(kind, _)| kind == "tool_parameter_contract_rejected")
            .unwrap()
            .1;
        assert_eq!(
            rejection["parameterDiagnostic"]["fieldPath"],
            "initialContent"
        );
        assert_eq!(rejection["correctionAttempt"], 1);
        let missing = &evidence
            .events
            .iter()
            .find(|(kind, _)| kind == "model_control_rejected")
            .unwrap()
            .1;
        assert_eq!(missing["code"], "AG_MODEL_CONTROL_MISSING");
        assert_eq!(missing["correctionAttempt"], 1);
        let requests = evidence
            .requests
            .iter()
            .filter(|(path, _)| path == "/invoke/stream")
            .collect::<Vec<_>>();
        assert_eq!(requests.len(), 3);
        for (_, request) in &requests {
            assert!(request["tools"]
                .as_array()
                .unwrap()
                .iter()
                .any(|tool| tool["function"]["name"] == "plan_workspace_operation"));
            assert!(request["tools"]
                .as_array()
                .unwrap()
                .iter()
                .all(|tool| tool["function"]["name"] != ag::model_output::TOOL_BATCH_NAME));
        }
        let feedback = requests[1].1["messages"].to_string();
        assert!(
            feedback.contains("initialContent") && feedback.contains("non_blank_frozen_content")
        );
        assert!(feedback.contains("requiresNewToolCallId"));

        let capped = run_fixed_turn_with_prompt(
            true,
            vec![create(""), create(" \n"), create("")],
            "新建empty.html，先给确认卡",
        );
        assert_eq!(capped.status, "error");
        assert!(!capped
            .workspace_files
            .iter()
            .any(|path| path == "empty.html"));
        assert!(capped.safe_events.iter().any(|event| matches!(&event.event,
            AgTurnEventPayload::Error { code, .. } if code == "AG_MODEL_CORRECTION_LIMIT")));
        assert!(!capped.safe_events.iter().any(|event| matches!(
            event.event,
            AgTurnEventPayload::DecisionRequestProposed { .. }
        )));
        assert_eq!(
            capped
                .events
                .iter()
                .filter(|(kind, _)| kind == "tool_parameter_contract_rejected")
                .count(),
            3
        );

        let mut incomplete = fixed_control("final");
        incomplete["intent"]["sideEffectIntent"] = serde_json::json!("structure_plan");
        let mut clarify = incomplete.clone();
        clarify["nextAction"] = serde_json::json!("clarify");
        clarify["intent"]["ambiguity"] = serde_json::json!("blocking");
        clarify["clarificationQuestion"] = serde_json::json!("请指定源文件");
        let evidence = run_fixed_turn(
            true,
            vec![
                fixed_text(incomplete.clone(), "UNDELIVERED_PLAN_PROMISE"),
                fixed_text(clarify, "请指定源文件"),
            ],
        );
        assert_eq!(evidence.status, "draft"); // 等待是非终态，前端由事件投影 clarifying。
        assert!(evidence
            .safe_events
            .iter()
            .any(|e| matches!(e.event, AgTurnEventPayload::ClarificationRequired { .. })));
        assert!(!serde_json::to_string(&evidence.safe_events)
            .unwrap()
            .contains("UNDELIVERED_PLAN_PROMISE"));
        assert_eq!(
            evidence
                .events
                .iter()
                .filter(|(k, _)| k == "model_control_rejected")
                .count(),
            1
        );
        assert_eq!(
            evidence
                .requests
                .iter()
                .filter(|(p, _)| p == "/invoke/stream")
                .count(),
            2
        );
        for status in ["report_only", "blocked", "no_op", "partial"] {
            let mut control = incomplete.clone();
            control["completion"] =
                serde_json::json!({"status":status,"reason":"本阶段未执行操作，不是确认等待"});
            let evidence = run_fixed_turn(
                true,
                vec![fixed_text(control, "当前阶段报告，未进行修改。")],
            );
            assert_eq!(evidence.status, "completed");
            assert!(evidence.safe_events.iter().any(|e| matches!(&e.event, AgTurnEventPayload::Done { stop_reason } if stop_reason == status)));
            assert!(!evidence.safe_events.iter().any(|e| matches!(
                e.event,
                AgTurnEventPayload::ToolRequest { .. }
                    | AgTurnEventPayload::DecisionRequestProposed { .. }
            )));
        }
    }

    // covers: BR-AG-STATE-002, BR-SYS-STATE-007
    #[test]
    fn fixed_runtime_explicit_empty_txt_reaches_one_real_decision_without_creating_file() {
        let mut control = fixed_control("tool");
        control["intent"]["intentType"] = serde_json::json!("organize_workspace");
        control["intent"]["sideEffectIntent"] = serde_json::json!("structure_plan");
        let evidence = run_fixed_turn_with_prompt(
            true,
            vec![fixed_tool(
                "plan_workspace_operation",
                serde_json::json!({
                    "operations":[{"kind":"create_document","targetPath":"story.txt","resourceKind":"file"}],
                    "createEmpty":true,
                    "_binderTurn":control
                }),
            )],
            "创建零字节 TXT 文件 story.txt；只生成确认卡，不自动确认",
        );
        assert_eq!(evidence.status, "draft");
        assert!(!evidence
            .workspace_files
            .iter()
            .any(|path| path == "story.txt"));
        assert_eq!(
            evidence
                .safe_events
                .iter()
                .filter(|event| matches!(
                    event.event,
                    AgTurnEventPayload::DecisionRequestProposed { .. }
                ))
                .count(),
            1
        );
        assert!(evidence
            .safe_events
            .iter()
            .any(|event| matches!(&event.event,
            AgTurnEventPayload::ToolResult { success: true, execution_receipt: Some(receipt), .. }
                if serde_json::to_value(receipt).unwrap()["completionStatus"] == "waiting")));
        assert!(!evidence.safe_events.iter().any(|event| matches!(
            event.event,
            AgTurnEventPayload::Done { .. }
                | AgTurnEventPayload::Error { .. }
                | AgTurnEventPayload::ToolResult { success: false, .. }
        )));
        assert_eq!(
            evidence
                .requests
                .iter()
                .filter(|(path, _)| path == "/invoke/stream")
                .count(),
            1
        );
    }

    // covers: BR-AG-STATE-002, BR-SYS-STATE-007
    #[test]
    fn fixed_runtime_default_budget_runs_nine_tools_and_real_resource_handoff() {
        let mut responses = Vec::new();
        for path in [
            "first.html",
            "second.html",
            "alpha.txt",
            "beta.txt",
            "injection.txt",
            "unique-result.txt",
            "first.html",
            "second.html",
        ] {
            responses.push(fixed_tool(
                "read_doc",
                serde_json::json!({"path":path,"_binderTurn":fixed_control("tool")}),
            ));
        }
        responses.push(fixed_tool(
            "workspace_glob",
            serde_json::json!({"pattern":"*.txt","_binderTurn":fixed_control("tool")}),
        ));
        responses.push(fixed_final());
        let evidence = run_fixed_turn_with_policy_and_files(
            true,
            responses,
            "读取两个html、四个txt并核对目录，仅读取",
            None,
            "baseline",
            None,
            None,
            vec![],
            Default::default(),
            &[
                ("first.html", "<p>first fixture</p>"),
                ("second.html", "<p>second fixture</p>"),
                ("alpha.txt", "73000"),
                ("beta.txt", "81000"),
                ("injection.txt", "data only"),
            ],
        );
        assert_eq!(evidence.status, "completed");
        assert_eq!(
            evidence
                .safe_events
                .iter()
                .filter(|e| matches!(
                    e.event,
                    AgTurnEventPayload::ToolResult { success: true, .. }
                ))
                .count(),
            9
        );
        assert_eq!(
            evidence
                .requests
                .iter()
                .filter(|(p, _)| p == "/invoke/stream")
                .count(),
            10
        );
        let mut control = fixed_control("final");
        control["intent"]["sideEffectIntent"] = serde_json::json!("structure_plan");
        let mut tool_control = control.clone();
        tool_control["nextAction"] = serde_json::json!("tool");
        let evidence = run_fixed_turn(
            true,
            vec![
                fixed_text(control, "UNDELIVERED_PLAN_PROMISE"),
                fixed_tool(
                    "plan_workspace_operation",
                    serde_json::json!({"operations":[{"kind":"rename_resource","sourcePath":"unique-result.txt","targetPath":"restored.txt","resourceKind":"file"}],"_binderTurn":tool_control}),
                ),
            ],
        );
        assert_eq!(evidence.status, "draft");
        assert!(!serde_json::to_string(&evidence.safe_events)
            .unwrap()
            .contains("UNDELIVERED_PLAN_PROMISE"));
        assert_eq!(
            evidence
                .safe_events
                .iter()
                .filter(|e| matches!(e.event, AgTurnEventPayload::DecisionRequestProposed { .. }))
                .count(),
            1
        );
    }

    // covers: BR-AG-STATE-002, BR-SYS-STATE-007
    #[test]
    fn fixed_runtime_soft_budget_freezes_tools_and_wraps_up_without_replaying() {
        let policy = ag::tool_registry::ExecutionBudgetPolicy {
            max_model_calls: 4,
            max_tool_calls: 1,
            max_corrections: 2,
            final_reserve: 2,
        };
        let call = fixed_tool(
            "workspace_glob",
            serde_json::json!({"pattern":"*.txt","_binderTurn":fixed_control("tool")}),
        );
        for responses in [
            vec![call.clone(), fixed_final()],
            vec![call.clone(), call.clone(), call.clone(), call.clone()],
        ] {
            let evidence = run_fixed_turn_with_policy(
                true,
                responses,
                "列出txt",
                None,
                "fixture",
                None,
                None,
                vec![],
                policy,
            );
            assert_eq!(evidence.status, "completed");
            assert_eq!(
                evidence
                    .safe_events
                    .iter()
                    .filter(|e| matches!(
                        e.event,
                        AgTurnEventPayload::ToolResult { success: true, .. }
                    ))
                    .count(),
                1
            );
            assert!(evidence.safe_events.iter().any(|e| matches!(&e.event,AgTurnEventPayload::Done {stop_reason} if stop_reason=="budget_exhausted")));
            let invokes: Vec<_> = evidence
                .requests
                .iter()
                .filter(|(p, _)| p == "/invoke/stream")
                .collect();
            assert!(invokes.len() <= 4);
            for (_, request) in &invokes[1..] {
                assert!(request["tools"].as_array().unwrap().is_empty());
            }
            assert_eq!(
                evidence
                    .events
                    .iter()
                    .filter(|(k, _)| k == "execution_budget_frozen")
                    .count(),
                1
            );
        }
    }

    // covers: BR-AG-STATE-001, BR-AG-STATE-002, BR-SYS-STATE-007
    #[test]
    fn fixed_runtime_soft_budget_batch_cap_and_gate_cancel_are_not_success_fallbacks() {
        let policy = ag::tool_registry::ExecutionBudgetPolicy {
            max_model_calls: 4,
            max_tool_calls: 1,
            max_corrections: 2,
            final_reserve: 2,
        };
        let arguments =
            serde_json::json!({"pattern":"*.txt","_binderTurn":fixed_control("tool")}).to_string();
        let batch = format!(
            "event: tool_calls\ndata: {}\n\n",
            serde_json::json!({"tool_calls":[
                {"id":"batch-first","name":"workspace_glob","arguments":arguments},
                {"id":"batch-second","name":"workspace_glob","arguments":arguments}
            ]})
        );
        let evidence = run_fixed_turn_with_policy(
            true,
            vec![batch, fixed_final()],
            "枚举txt",
            None,
            "fixture",
            None,
            None,
            vec![],
            policy,
        );
        assert_eq!(evidence.status, "completed");
        assert_eq!(
            evidence
                .safe_events
                .iter()
                .filter(|e| matches!(
                    e.event,
                    AgTurnEventPayload::ToolResult { success: true, .. }
                ))
                .count(),
            1
        );
        assert_eq!(
            evidence
                .safe_events
                .iter()
                .filter(|e| matches!(
                    e.event,
                    AgTurnEventPayload::ToolResult { success: false, .. }
                ))
                .count(),
            1
        );
        let requests: Vec<_> = evidence
            .requests
            .iter()
            .filter(|(p, _)| p == "/invoke/stream")
            .collect();
        assert_eq!(requests.len(), 2);
        assert_eq!(
            requests[1].1["messages"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|m| m["role"] == "tool")
                .count(),
            2
        );
        for cancelled in [false, true] {
            let evidence = run_fixed_turn_with_policy(
                true,
                vec![fixed_tool(
                    "workspace_glob",
                    serde_json::json!({"pattern":"*.txt","_binderTurn":fixed_control("tool")}),
                )],
                "枚举txt",
                None,
                "fixture",
                cancelled.then_some(2),
                (!cancelled).then_some((2, GateFixtureFault::HeadersTimeout)),
                vec![],
                policy,
            );
            assert_eq!(
                evidence.status,
                if cancelled { "cancelled" } else { "error" }
            );
            assert!(!evidence.safe_events.iter().any(|e|matches!(&e.event,AgTurnEventPayload::Done {stop_reason} if stop_reason=="budget_exhausted")));
            assert_eq!(
                evidence
                    .requests
                    .iter()
                    .filter(|(p, _)| p == "/invoke/stream")
                    .count(),
                1
            );
        }
    }

    // covers: BR-AG-STATE-001
    // covers: BR-AG-STATE-002
    // covers: BR-AG-DATA-004
    #[test]
    fn fixed_runtime_real_glob_reassembles_regates_and_finishes_once() {
        let evidence = run_fixed_turn(
            true,
            vec![
                fixed_tool(
                    "workspace_glob",
                    serde_json::json!({
                        "pattern":"*.txt", "_binderTurn":fixed_control("tool")
                    }),
                ),
                fixed_final(),
            ],
        );
        assert_eq!(evidence.status, "completed");
        let invokes = evidence
            .requests
            .iter()
            .filter(|(path, _)| path == "/invoke/stream")
            .collect::<Vec<_>>();
        assert_eq!(invokes.len(), 2);
        let exposed_names = invokes[0].1["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|tool| tool["function"]["name"].as_str())
            .collect::<Vec<_>>();
        assert!(exposed_names.contains(&"workspace_glob"));
        assert!(!exposed_names.contains(&ag::model_output::TOOL_BATCH_NAME));
        assert!(invokes[0].1["messages"]
            .to_string()
            .contains("每次业务工具调用的 arguments 顶层都必须包含完整 _binderTurn"));
        assert!(evidence.events.iter().any(|(kind, payload)| {
            kind == "tool_transport_observed"
                && payload["protocolVersion"] == ag::model_output::DIRECT_TOOL_PROTOCOL
                && payload["businessCallCount"] == 1
        }));
        let budgets = evidence
            .events
            .iter()
            .filter(|(kind, _)| kind == "prompt_budget_checked")
            .map(|(_, p)| p)
            .collect::<Vec<_>>();
        assert_eq!(budgets.len(), 2);
        for (index, (_, request)) in invokes.iter().enumerate() {
            let wire = ag::context::measure_prompt_budget(
                request["messages"].as_array().unwrap(),
                &request["tools"],
            );
            assert_eq!(
                budgets[index]["measurement"]["serializedChars"],
                wire.serialized_chars
            );
            assert_eq!(
                budgets[index]["measurement"]["estimatedTokens"],
                wire.estimated_tokens
            );
            assert_eq!(budgets[index]["nextModelCallOrdinal"], index + 1);
            assert_eq!(budgets[index]["assemblyOrdinal"], index + 1);
        }
        assert_eq!(budgets[0]["assemblyReason"], "initial");
        assert_eq!(budgets[1]["assemblyReason"], "tool_result");
        assert_eq!(
            evidence
                .requests
                .iter()
                .filter(|(path, _)| path != "/auth/login")
                .map(|(path, _)| path.as_str())
                .collect::<Vec<_>>(),
            vec![
                "/gate/check",
                "/invoke/stream",
                "/gate/check",
                "/invoke/stream"
            ]
        );
        assert!(invokes
            .iter()
            .all(|(_, body)| body["token"] == "fixture-token"));
        assert_eq!(
            evidence
                .requests
                .iter()
                .filter(|(path, _)| path == "/gate/check")
                .count(),
            2
        );
        let tools = invokes[1].1["messages"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|message| message["role"] == "tool")
            .collect::<Vec<_>>();
        assert_eq!(tools.len(), 1);
        assert!(tools[0].to_string().contains("unique-result.txt"));
        assert!(!tools[0].to_string().contains("excluded.md"));
        let injected: serde_json::Value =
            serde_json::from_str(tools[0]["content"].as_str().unwrap()).unwrap();
        let receipt = evidence
            .safe_events
            .iter()
            .find_map(|event| match &event.event {
                AgTurnEventPayload::ToolResult {
                    execution_receipt, ..
                } => execution_receipt.as_ref(),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            injected["executionReceipt"],
            serde_json::to_value(receipt).unwrap()
        );
        assert_eq!(
            receipt.execution_status,
            crate::types::ToolCallExecutionStatus::Succeeded
        );
        assert_eq!(
            receipt.completion_status,
            crate::types::ToolCallCompletionStatus::Completed
        );
        assert_eq!(
            receipt.effect_status,
            crate::types::ToolCallEffectStatus::NotApplicable
        );
        let kinds = evidence
            .events
            .iter()
            .map(|(kind, _)| kind.as_str())
            .collect::<Vec<_>>();
        let grant = kinds
            .iter()
            .position(|kind| *kind == "capability_grant_computed")
            .unwrap();
        let tool = kinds
            .iter()
            .position(|kind| *kind == "tool_result")
            .unwrap();
        assert!(grant < tool);
        assert!(evidence.safe_events.iter().any(|event| matches!(
            &event.event,
            AgTurnEventPayload::ToolResult { success: true, .. }
        )));
    }

    // covers: BR-AG-DATA-002
    // covers: BR-AG-DATA-004
    // covers: BR-AG-DATA-005
    // covers: BR-AG-OBS-001
    #[test]
    fn fixed_runtime_long_history_survives_initial_and_tool_continuation_without_authority_change()
    {
        let prior = || {
            vec![
                ("user", "PRIOR-REQUEST：分析A/B组，只读，不修改。".into()),
                (
                    "assistant",
                    format!("PRIOR-HEAD{}PRIOR-TAIL", "测试说明。".repeat(2000)),
                ),
            ]
        };
        let evidence = run_fixed_turn_with_history(
            true,
            vec![
                fixed_tool(
                    "workspace_glob",
                    serde_json::json!({"pattern":"*.txt","_binderTurn":fixed_control("tool")}),
                ),
                fixed_final(),
            ],
            "继续按之前口径列出TXT文件。",
            None,
            "read-only fixture",
            None,
            None,
            prior(),
        );
        assert_eq!(evidence.status, "completed");
        let requests = evidence
            .requests
            .iter()
            .filter(|(path, _)| path == "/invoke/stream")
            .collect::<Vec<_>>();
        assert_eq!(requests.len(), 2);
        for (_, request) in requests {
            let history = request["messages"]
                .as_array()
                .unwrap()
                .iter()
                .find(|message| {
                    message["content"]
                        .as_str()
                        .is_some_and(|s| s.contains("[DATA-ONLY session_history]"))
                })
                .unwrap();
            for marker in ["PRIOR-REQUEST", "PRIOR-HEAD", "PRIOR-TAIL"] {
                assert!(history["content"].as_str().unwrap().contains(marker));
            }
            assert!(
                ag::context::measure_prompt_budget(
                    request["messages"].as_array().unwrap(),
                    &request["tools"]
                )
                .estimated_tokens
                    <= PROMPT_INPUT_LIMIT
            );
        }
        let selections = evidence
            .events
            .iter()
            .filter(|(kind, _)| kind == "history_assembled")
            .map(|(_, p)| p)
            .collect::<Vec<_>>();
        assert_eq!(selections.len(), 2);
        assert!(selections
            .iter()
            .all(|p| p["historyIncludedCount"] == 2 && p["historyExcerptedCount"] == 0));
        assert!(!selections
            .iter()
            .any(|p| p.to_string().contains("PRIOR-REQUEST")));
        let rejected = run_fixed_turn_with_history(
            true,
            vec![fixed_final()],
            &"OVERSIZED-CURRENT".repeat(ag::prompt_runtime::PROMPT_TOKEN_BUDGET),
            None,
            "read-only fixture",
            None,
            None,
            prior(),
        );
        assert_eq!(rejected.status, "error");
        assert!(!rejected
            .requests
            .iter()
            .any(|(path, _)| path == "/gate/check" || path == "/invoke/stream"));
        assert!(rejected
            .events
            .iter()
            .any(|(kind, p)| kind == "error" && p["code"] == "AG_PROMPT_BUDGET_EXCEEDED"));
    }

    // covers: BR-AG-DATA-005
    // covers: BR-AG-STATE-001
    #[test]
    fn fixed_runtime_budget_denial_stops_before_first_or_next_model_call() {
        // Exceed the mandatory input alone; schema deduplication must not invalidate this fixture.
        let initial = run_fixed_turn_with_prompt(
            true,
            vec![],
            &"PRIVATE_INPUT".repeat(PROMPT_INPUT_LIMIT * 4 / "PRIVATE_INPUT".len() + 1),
        );
        assert_eq!(initial.status, "error");
        assert!(!initial
            .requests
            .iter()
            .any(|(path, _)| path == "/invoke/stream" || path == "/gate/check"));
        let budget = &initial
            .events
            .iter()
            .find(|(k, _)| k == "prompt_budget_checked")
            .unwrap()
            .1;
        assert_eq!(budget["exceeded"], true);
        assert!(!budget.to_string().contains("PRIVATE_INPUT"));
        let after_tool = run_fixed_turn_with_file(
            true,
            vec![fixed_tool(
                "workspace_glob",
                serde_json::json!({
                    "pattern":"PRIVATE_RESULT".repeat(16000), "_binderTurn":fixed_control("tool")
                }),
            )],
            "读取 unique-result.txt",
            None,
            "small fixture",
        );
        assert_eq!(after_tool.status, "error");
        assert_eq!(
            after_tool
                .requests
                .iter()
                .filter(|(p, _)| p == "/invoke/stream")
                .count(),
            1
        );
        assert!(after_tool.safe_events.iter().any(|e| matches!(
            &e.event,
            AgTurnEventPayload::ToolResult { success: true, .. }
        )));
        let budgets = after_tool
            .events
            .iter()
            .filter(|(k, _)| k == "prompt_budget_checked")
            .map(|(_, p)| p)
            .collect::<Vec<_>>();
        assert_eq!(budgets.len(), 2);
        assert_eq!(budgets[1]["assemblyReason"], "tool_result");
        assert_eq!(budgets[1]["exceeded"], true);
        assert!(!budgets[1].to_string().contains("PRIVATE_RESULT"));
    }

    // covers: BR-AG-STATE-001, BR-SYS-STATE-007
    #[test]
    fn fixed_runtime_cancel_stalled_first_and_continuation_gate_is_cancelled_not_denied() {
        for ordinal in [1, 2] {
            let evidence = run_fixed_turn_with_gate_cancel(
                true,
                vec![
                    fixed_tool(
                        "workspace_glob",
                        serde_json::json!({
                            "pattern":"*.txt", "_binderTurn":fixed_control("tool")
                        }),
                    ),
                    fixed_final(),
                ],
                "列出 txt 文件",
                None,
                "read-only fixture",
                Some(ordinal),
            );
            assert_eq!(evidence.status, "cancelled");
            assert!(evidence.events.iter().any(|(kind, payload)| {
                kind == "gate_evaluated"
                    && payload["attempt"] == ordinal
                    && payload["outcome"] == "cancelled"
            }));
            assert!(!evidence.events.iter().any(|(kind, _)| kind == "error"));
            assert_eq!(
                evidence
                    .requests
                    .iter()
                    .filter(|(path, _)| path == "/invoke/stream")
                    .count(),
                ordinal - 1
            );
            assert_eq!(
                evidence
                    .events
                    .iter()
                    .filter(|(kind, _)| kind == "tool_request")
                    .count(),
                ordinal - 1
            );
            assert!(evidence.safe_events.iter().any(|event| matches!(
                &event.event, AgTurnEventPayload::Done { stop_reason } if stop_reason == "cancelled"
            )));
        }
    }

    // covers: BR-AG-STATE-001, BR-AG-VERIFY-001
    #[test]
    fn fixed_runtime_invalid_first_and_continuation_gate_is_error_not_denied() {
        for ordinal in [1, 2] {
            let evidence = run_fixed_turn_with_gate_fault(
                true,
                vec![
                    fixed_tool(
                        "workspace_glob",
                        serde_json::json!({"pattern":"*.txt", "_binderTurn":fixed_control("tool")}),
                    ),
                    fixed_final(),
                ],
                "列出 txt 文件",
                None,
                "read-only fixture",
                None,
                Some(ordinal),
            );
            assert_eq!(evidence.status, "error");
            assert_eq!(
                evidence
                    .requests
                    .iter()
                    .filter(|(path, _)| path == "/invoke/stream")
                    .count(),
                ordinal - 1
            );
            assert_eq!(
                evidence
                    .events
                    .iter()
                    .filter(|(kind, payload)| kind == "tool_result" && payload["success"] == true)
                    .count(),
                ordinal - 1
            );
            let error = evidence
                .events
                .iter()
                .find(|(kind, _)| kind == "error")
                .unwrap();
            assert_eq!(error.1["code"], "ENT_GATE_BAD_RESPONSE");
            assert_eq!(error.1["diagnostic"]["observedAt"], "gate");
            assert_eq!(error.1["diagnostic"]["category"], "unknown");
            assert_eq!(error.1["retriable"], true);
            assert!(!evidence.events.iter().any(|(kind, _)| kind == "done"));
        }
    }

    // covers: BR-AG-STATE-001, BR-AG-VERIFY-001, BR-ENT-VERIFY-001
    #[test]
    fn fixed_runtime_gate_transport_failures_close_without_model_or_tool_replay() {
        for ordinal in [1, 2] {
            for (fault, code, retriable, gates) in [
                (
                    GateFixtureFault::Http(500),
                    "ENT_GATE_HTTP_ERROR",
                    true,
                    ordinal,
                ),
                (
                    GateFixtureFault::Http(429),
                    "ENT_GATE_HTTP_ERROR",
                    true,
                    ordinal,
                ),
                (
                    GateFixtureFault::Http(400),
                    "ENT_GATE_HTTP_ERROR",
                    false,
                    ordinal,
                ),
                (
                    GateFixtureFault::UnknownVerdict,
                    "ENT_GATE_BAD_RESPONSE",
                    true,
                    ordinal,
                ),
                (
                    GateFixtureFault::Disconnect,
                    "ENT_GATE_UNREACHABLE",
                    true,
                    ordinal,
                ),
                (
                    GateFixtureFault::HeadersTimeout,
                    "ENT_GATE_TIMEOUT",
                    true,
                    ordinal,
                ),
                (
                    GateFixtureFault::BodyTimeout,
                    "ENT_GATE_TIMEOUT",
                    true,
                    ordinal,
                ),
                (
                    GateFixtureFault::AuthInvalid,
                    "ENT_AUTH_BAD_RESPONSE",
                    true,
                    ordinal - 1,
                ),
                (
                    GateFixtureFault::AuthDisconnect,
                    "ENT_AUTH_UNREACHABLE",
                    true,
                    ordinal - 1,
                ),
                (
                    GateFixtureFault::AuthTimeout,
                    "ENT_AUTH_TIMEOUT",
                    true,
                    ordinal - 1,
                ),
                (
                    GateFixtureFault::AuthHttp,
                    "ENT_AUTH_HTTP_ERROR",
                    true,
                    ordinal - 1,
                ),
                (
                    GateFixtureFault::Http(403),
                    "AG_GATE_DENIED",
                    false,
                    ordinal,
                ),
                (
                    GateFixtureFault::Repeated401,
                    "AG_GATE_DENIED",
                    false,
                    ordinal + 1,
                ),
                (
                    GateFixtureFault::ReauthPassed,
                    "AG_GATE_DENIED",
                    false,
                    ordinal + 1,
                ),
            ] {
                let evidence = run_fixed_turn_with_transport_fault(
                    true,
                    vec![
                        fixed_tool(
                            "workspace_glob",
                            serde_json::json!({"pattern":"*.txt", "_binderTurn":fixed_control("tool")}),
                        ),
                        fixed_final(),
                    ],
                    "列出 txt 文件",
                    None,
                    "unchanged evidence",
                    None,
                    Some((ordinal, fault)),
                );
                let denied = code == "AG_GATE_DENIED";
                assert_eq!(
                    evidence.status,
                    if denied { "gate_rejected" } else { "error" },
                    "{code} gate {ordinal}"
                );
                assert_eq!(
                    evidence
                        .requests
                        .iter()
                        .filter(|(path, _)| path == "/gate/check")
                        .count(),
                    gates,
                    "no silent retry: {code}"
                );
                assert_eq!(
                    evidence
                        .requests
                        .iter()
                        .filter(|(path, _)| path == "/invoke/stream")
                        .count(),
                    ordinal - 1
                );
                assert_eq!(
                    evidence
                        .events
                        .iter()
                        .filter(
                            |(kind, payload)| kind == "tool_result" && payload["success"] == true
                        )
                        .count(),
                    ordinal - 1
                );
                let error = &evidence
                    .events
                    .iter()
                    .find(|(kind, _)| kind == "error")
                    .unwrap()
                    .1;
                assert_eq!(
                    error["code"],
                    code,
                    "requests={:?}",
                    evidence
                        .requests
                        .iter()
                        .map(|(path, _)| path)
                        .collect::<Vec<_>>()
                );
                assert_eq!(error["retriable"], retriable);
                assert_eq!(error["diagnostic"]["observedAt"], "gate");
                assert_eq!(
                    error["diagnostic"]["category"],
                    if denied { "authorization" } else { "unknown" }
                );
                assert!(!evidence.events.iter().any(|(kind, _)| kind == "done"));
            }
        }
    }

    // covers: BR-AG-STATE-001
    #[test]
    fn fixed_runtime_plain_conversation_does_not_call_tools() {
        let reply = fixed_final()
            .replace("本次检查已结束。", "你好。")
            .replace("inspect", "answer");
        let evidence = run_fixed_turn_with_prompt(true, vec![reply.clone()], "你好");
        assert_eq!(evidence.status, "completed");
        assert!(!evidence
            .events
            .iter()
            .any(|(kind, _)| kind == "tool_request"));
        assert!(!evidence
            .events
            .iter()
            .any(|(kind, _)| kind == "project_report"));
        assert_eq!(
            evidence
                .requests
                .iter()
                .filter(|(path, _)| path == "/invoke/stream")
                .count(),
            1
        );
        let with_editor = run_fixed_turn_with_context(true, vec![reply], "你好", Some(true));
        assert_eq!(with_editor.status, "completed");
        assert!(!with_editor
            .events
            .iter()
            .any(|(kind, _)| kind == "project_report"));
        for (_, request) in evidence
            .requests
            .iter()
            .chain(with_editor.requests.iter())
            .filter(|(path, _)| path == "/invoke/stream")
        {
            let messages = request["messages"].as_array().unwrap();
            let core = messages[0]["content"].as_str().unwrap();
            assert!(!core.contains("BDocx"));
            assert!(!core.contains("block_ops"));
            assert!(core.contains("CapabilityGrant"));
            let runtime = messages[1]["content"].as_str().unwrap();
            assert!(runtime.contains("capabilityInstructions"));
            let edit_allowed = super::super::turn_tool_loop::tool_is_visible_in_manifest(
                &request["tools"],
                "propose_edit",
            );
            assert_eq!(runtime.contains("block_ops"), edit_allowed);
        }
    }

    // covers: BR-AG-DATA-005, BR-AG-OBS-001, BR-AG-STATE-001
    #[test]
    fn fixed_runtime_oversize_read_is_failure_not_full_read_evidence() {
        let evidence = run_fixed_turn_with_file(
            true,
            vec![
                fixed_tool(
                    "read_doc",
                    serde_json::json!({"path":"unique-result.txt","view":"structure","_binderTurn":fixed_control("tool")}),
                ),
                fixed_final().replace("本次检查已结束。", "读取结果过大，尚未读取全文。"),
            ],
            "读取 unique-result.txt",
            None,
            &"PRIVATE_HUGE_BODY".repeat(12_000),
        );
        assert_eq!(evidence.status, "completed");
        let results = evidence
            .safe_events
            .iter()
            .filter_map(|e| match &e.event {
                AgTurnEventPayload::ToolResult { success, .. } => Some(*success),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(results, vec![false]);
        let invokes = evidence
            .requests
            .iter()
            .filter(|(p, _)| p == "/invoke/stream")
            .collect::<Vec<_>>();
        assert_eq!(invokes.len(), 2);
        for (_, request) in &invokes {
            assert_eq!(
                request["max_tokens"],
                ag::prompt_runtime::CONTINUATION_RESERVE_TOKENS
            );
        }
        assert!(!invokes[1].1.to_string().contains("PRIVATE_HUGE_BODY"));
        assert!(invokes[1].1.to_string().contains("未返回截断正文"));
        for (kind, payload) in &evidence.events {
            if kind == "evidence_recorded" {
                assert!(!payload.to_string().contains("full_document"));
            }
            if kind == "prompt_budget_checked" {
                assert_eq!(payload["exceeded"], false);
                assert_eq!(
                    payload["continuationReserveTokens"],
                    ag::prompt_runtime::CONTINUATION_RESERVE_TOKENS
                );
                assert_eq!(
                    payload["totalTokenLimit"],
                    ag::prompt_runtime::PROMPT_TOKEN_BUDGET
                );
                assert_eq!(
                    payload["tokenLimit"],
                    ag::prompt_runtime::PROMPT_INPUT_LIMIT
                );
            }
        }
    }

    // covers: BR-AG-STATE-002
    #[test]
    fn fixed_runtime_permission_rejection_and_execution_failure_remain_distinct() {
        for (name, arguments, expected_stage) in [
            (
                "propose_edit",
                serde_json::json!({"path":"unique-result.txt", "_binderTurn":fixed_control("tool")}),
                "tool_validation",
            ),
            (
                "read_doc",
                serde_json::json!({"path":"missing.txt", "_binderTurn":fixed_control("tool")}),
                "tool_execution",
            ),
        ] {
            let evidence = run_fixed_turn(
                true,
                vec![
                    fixed_tool(name, arguments),
                    fixed_final().replace("本次检查已结束。", "请求的操作未执行成功。"),
                ],
            );
            // 运行闭合不等于专业操作成功：失败结果按现有循环回注，不冒充成功。
            assert_eq!(evidence.status, "completed");
            let results = evidence
                .safe_events
                .iter()
                .filter_map(|event| match &event.event {
                    AgTurnEventPayload::ToolResult {
                        success,
                        diagnostic,
                        ..
                    } => Some((success, diagnostic)),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(results.len(), 1);
            assert!(!*results[0].0);
            assert_eq!(results[0].1.as_ref().unwrap().observed_at, expected_stage);
            let receipt = evidence
                .safe_events
                .iter()
                .find_map(|event| match &event.event {
                    AgTurnEventPayload::ToolResult {
                        execution_receipt, ..
                    } => execution_receipt.as_ref(),
                    _ => None,
                })
                .unwrap();
            assert_eq!(
                receipt.execution_status,
                if expected_stage == "tool_validation" {
                    crate::types::ToolCallExecutionStatus::Rejected
                } else {
                    crate::types::ToolCallExecutionStatus::Failed
                }
            );
            assert!(receipt.failure.is_some());
            assert!(receipt.evidence_ids.is_empty());
            let request = evidence
                .requests
                .iter()
                .filter(|(path, _)| path == "/invoke/stream")
                .last()
                .unwrap();
            let injected = request.1["messages"]
                .as_array()
                .unwrap()
                .iter()
                .find(|message| message["role"] == "tool")
                .unwrap();
            let injected: serde_json::Value =
                serde_json::from_str(injected["content"].as_str().unwrap()).unwrap();
            assert_eq!(
                injected["executionReceipt"],
                serde_json::to_value(receipt).unwrap()
            );
            assert_eq!(
                evidence
                    .requests
                    .iter()
                    .filter(|(path, _)| path == "/invoke/stream")
                    .count(),
                2
            );
        }
    }

    // covers: BR-AG-STATE-001
    // covers: BR-ENT-VERIFY-001
    #[test]
    fn fixed_runtime_gate_denied_never_calls_model_or_tools() {
        let evidence = run_fixed_turn(false, vec![]);
        assert_eq!(evidence.status, "gate_rejected");
        assert!(!evidence
            .requests
            .iter()
            .any(|(path, _)| path == "/invoke/stream"));
        assert!(!evidence
            .events
            .iter()
            .any(|(kind, _)| kind == "tool_request"));
    }

    // covers: BR-AG-STATE-001
    // covers: BR-AG-DATA-004
    #[test]
    fn fixed_runtime_invalid_control_and_truncation_end_as_error() {
        let malformed = "event: chunk\ndata: {\"text\":\"<binder-turn-control>{bad}</binder-turn-control>\"}\n\nevent: done\ndata: {\"stop_reason\":\"stop\"}\n\n".to_string();
        for (responses, code, expected_calls) in [
            (vec![malformed; 3], "AG_MODEL_CORRECTION_LIMIT", 3),
            (
                vec![fixed_final()
                    .split("event: done")
                    .next()
                    .unwrap()
                    .to_string()],
                "AG_GW_STREAM_INCOMPLETE",
                1,
            ),
        ] {
            let evidence = run_fixed_turn(true, responses);
            assert_eq!(evidence.status, "error");
            assert_eq!(
                evidence
                    .requests
                    .iter()
                    .filter(|(path, _)| path == "/invoke/stream")
                    .count(),
                expected_calls
            );
            assert!(evidence.safe_events.iter().any(|event| matches!(&event.event, AgTurnEventPayload::Error { code: actual, .. } if actual == code)));
            assert!(!evidence
                .events
                .iter()
                .any(|(kind, _)| kind == "tool_request"));
        }
    }

    // covers: BR-AG-STATE-001
    // covers: BR-AG-STATE-002
    #[test]
    fn fixed_runtime_text_tool_action_cannot_execute_and_correction_uses_native_channel() {
        let malformed = format!(
            "event: chunk\ndata: {}\n\nevent: done\ndata: {{\"stop_reason\":\"stop\"}}\n\n",
            serde_json::json!({"text":format!(
                "<binder-turn-control>{}</｜｜DSML｜｜ parameter>",
                fixed_control("tool")
            )})
        );
        let native = fixed_tool(
            "workspace_glob",
            serde_json::json!({"pattern":"*.txt","_binderTurn":fixed_control("tool")}),
        );
        let recovered = run_fixed_turn(true, vec![malformed.clone(), native, fixed_final()]);
        assert_eq!(recovered.status, "completed");
        let requests = recovered
            .requests
            .iter()
            .filter(|(path, _)| path == "/invoke/stream")
            .collect::<Vec<_>>();
        assert_eq!(requests.len(), 3);
        assert!(requests[1].1["messages"]
            .to_string()
            .contains("没有原生工具调用"));
        let rejections = recovered
            .events
            .iter()
            .filter(|(kind, _)| kind == "model_control_rejected")
            .collect::<Vec<_>>();
        assert_eq!(rejections.len(), 1);
        assert_eq!(rejections[0].1["code"], "AG_MODEL_CONTROL_INCOMPLETE");
        assert_eq!(rejections[0].1["responseRejectionKind"], "text_tool_action");
        assert_eq!(
            recovered
                .events
                .iter()
                .filter(|(kind, _)| kind == "tool_request")
                .count(),
            1,
            "only the later native call may reach tool execution"
        );
        assert!(!serde_json::to_string(&recovered.safe_events)
            .unwrap()
            .contains("DSML"));

        let exhausted = run_fixed_turn(true, vec![malformed.clone(), malformed.clone(), malformed]);
        assert_eq!(exhausted.status, "error");
        assert!(exhausted
            .safe_events
            .iter()
            .any(|event| matches!(&event.event,
            AgTurnEventPayload::Error { code, .. } if code == "AG_MODEL_CORRECTION_LIMIT")));
        assert!(!exhausted
            .events
            .iter()
            .any(|(kind, _)| kind == "tool_request"));
    }

    fn history(workspace_id: &str, session_id: &str) -> ag::context::HistTurn {
        ag::context::HistTurn {
            workspace_id: workspace_id.to_string(),
            session_id: session_id.to_string(),
            role: "user".to_string(),
            content: "历史内容".to_string(),
            rel_path: Some("notes/a.md".to_string()),
            turn_id: Some("turn-old".to_string()),
        }
    }

    // covers: BR-AG-DATA-004
    #[test]
    fn history_scope_rejects_cross_workspace_session_and_unknown_roles() {
        assert!(validate_history_scope(&[history("ws-1", "s-1")], "ws-1", "s-1").is_ok());
        for invalid in [
            history("ws-2", "s-1"),
            history("ws-1", "s-2"),
            ag::context::HistTurn {
                role: "system".to_string(),
                ..history("ws-1", "s-1")
            },
        ] {
            let error = validate_history_scope(&[invalid], "ws-1", "s-1")
                .expect_err("wrong history identity must fail closed");
            assert_eq!(error.code, "AG_HISTORY_SCOPE_MISMATCH");
        }
    }

    #[test]
    // covers: BR-AG-STATE-004
    fn runtime_profile_is_required_and_unknown_values_fail_closed() {
        let missing = require_runtime_profile(None).expect_err("missing profile must fail closed");
        assert_eq!(missing.code, "AG_RUNTIME_PROFILE_REQUIRED");

        let unknown = require_runtime_profile(Some("future-profile".to_string()))
            .expect_err("unknown profile must fail closed");
        assert_eq!(unknown.code, "AG_RUNTIME_PROFILE_UNSUPPORTED");

        assert_eq!(
            require_runtime_profile(Some(ag::turn::AGENT_EDIT_1_0_PROFILE.to_string())).unwrap(),
            ag::turn::AGENT_EDIT_1_0_PROFILE
        );
    }

    // covers: BR-AG-DATA-008
    // covers: BR-ED-DATA-008
    // covers: BR-SYS-DATA-008
    #[test]
    fn agent_context_facts_fail_closed_on_logical_state_drift() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("ag-context-facts-{unique}"));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.md"), "disk old").unwrap();
        let snapshot = ag::blocks::OpenDocSnapshot {
            rel_path: "a.md".to_string(),
            content: "logical new".to_string(),
            baseline: Some(7),
            edit_format: "markdown".to_string(),
            content_format: "text".to_string(),
            source: "active_editor".to_string(),
            dirty: true,
            session_version: 3,
            content_hash: "h3".to_string(),
            captured_at: 1,
            block_snapshots: Vec::new(),
            table_snapshots: Vec::new(),
            image_snapshots: Vec::new(),
        };
        let doc_fact = crate::types::AgentContextDocumentFactV1 {
            rel_path: "a.md".to_string(),
            source: "active_editor".to_string(),
            edit_format: "markdown".to_string(),
            content_format: "text".to_string(),
            dirty: true,
            baseline: Some("7".to_string()),
            session_version: 3,
            content_hash: "h3".to_string(),
        };
        let facts = AgentContextFactsV1 {
            version: AGENT_CONTEXT_FACTS_VERSION_V1.to_string(),
            workspace_id: "ws1".to_string(),
            turn_id: "t1".to_string(),
            active_doc: Some(doc_fact.clone()),
            open_docs: vec![doc_fact],
            selections: vec![crate::types::AgentContextSelectionFactV1 {
                rel_path: "a.md".to_string(),
                block_id: "legacy".to_string(),
                start: None,
                end: None,
            }],
            proposal_facts: vec![crate::types::AgentContextProposalFactV1 {
                diff_id: "diff-1".to_string(),
                session_id: "s1".to_string(),
                rel_path: "a.md".to_string(),
                state: "accepted".to_string(),
                effect_status: Some("saved".to_string()),
                saved: true,
            }],
        };
        let legacy_ref = BlockRef {
            rel_path: "a.md".to_string(),
            block_id: "legacy".to_string(),
            start: None,
            end: None,
            text: "selected".to_string(),
        };
        validate_agent_context_facts(
            Some(&facts),
            "ws1",
            &root,
            "t1",
            "s1",
            Some("a.md"),
            std::slice::from_ref(&snapshot),
            &[],
            std::slice::from_ref(&legacy_ref),
        )
        .unwrap();

        let mut cross_session = facts.clone();
        cross_session.proposal_facts[0].session_id = "s2".to_string();
        let error = validate_agent_context_facts(
            Some(&cross_session),
            "ws1",
            &root,
            "t1",
            "s1",
            Some("a.md"),
            std::slice::from_ref(&snapshot),
            &[],
            std::slice::from_ref(&legacy_ref),
        )
        .unwrap_err();
        assert_eq!(error.code, "AG_CONTEXT_FACTS_PROPOSAL_INVALID");

        let mut stale = facts;
        stale.open_docs[0].content_hash = "old".to_string();
        let error = validate_agent_context_facts(
            Some(&stale),
            "ws1",
            &root,
            "t1",
            "s1",
            Some("a.md"),
            &[snapshot],
            &[],
            &[legacy_ref],
        )
        .unwrap_err();
        assert_eq!(error.code, "AG_CONTEXT_FACTS_LOGICAL_STATE_MISMATCH");
        let _ = std::fs::remove_dir_all(root);
    }

    // covers: BR-AG-STATE-001, BR-SYS-STATE-007
    // 生产源码顺序回归：PromptPackage/预算必须先于 assembled、Gate 和 Streaming。
    #[test]
    fn prompt_package_and_budget_precede_gate_and_streaming_in_production_path() {
        let source = include_str!("turn_runtime.rs");
        let prepare_marker = ["started_turn_or_finish!(prepare_turn_prompt", "("].concat();
        let budget_marker = ["record_prompt_budget", "("].concat();
        let assembled_marker = ["TurnEvent::", "Assembled"].concat();
        let gate_marker = [
            "let gate = match super::turn_control::await_turn_operation",
            "(",
        ]
        .concat();
        let streaming_marker = ["TurnEvent::", "GatePassed"].concat();

        let prepare = source
            .find(&prepare_marker)
            .expect("production prompt assembly");
        let budget = source
            .find(&budget_marker)
            .expect("production budget check");
        let assembled = source
            .find(&assembled_marker)
            .expect("production assembled transition");
        let gate = source.find(&gate_marker).expect("production Gate call");
        let streaming = source
            .find(&streaming_marker)
            .expect("production streaming transition");

        assert!(prepare < budget);
        assert!(budget < assembled);
        assert!(assembled < gate);
        assert!(gate < streaming);
    }
}
