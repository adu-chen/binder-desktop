//! AgentTurn 当前文档、历史、引用与 CapabilityGrant 的 PromptPackage 装配。

use super::edit_support::{bdocx_structure_presence_from_context, capability_error_to_ipc};
use super::prompt_runtime::{
    assemble_agent_turn_messages, build_current_doc_prompt_context,
    empty_current_doc_prompt_context, insert_runtime_control_message, DOC_CTX_CAP,
    PROMPT_TOKEN_BUDGET,
};
use super::task_runtime::task_plan_runtime_projection;
use crate::ag;
use crate::ipc::IpcError;
use crate::AppState;

pub(super) struct PreparedTurnPrompt {
    pub(super) current_doc_requires_tool_read: bool,
    pub(super) messages: Vec<serde_json::Value>,
    pub(super) tools: serde_json::Value,
    pub(super) capability_grant: ag::tool_registry::CapabilityGrantV1,
    pub(super) prompt_package: ag::context::PromptPackageV1,
}

/**
 * @GOV
 * codes:    BR-AG-DATA-002-DATA-AG-AG-TURN-031
 * type:     DATA
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-002
 * term_ref: TERM-AG-005
 * boundary: in=IntentCurrentDocumentReferenceHistoryAndToolManifest | out=BudgetedPromptPackageMessagesAndCapabilityGrant | delegate=assemble_agent_turn_messages | consumer=PromptPackageV1
 */
#[allow(clippy::too_many_arguments)]
pub(super) fn prepare_turn_prompt(
    state: &AppState,
    runtime_profile: &str,
    turn_id: &str,
    session_id: &str,
    task_run_id: &str,
    intent_result: &ag::context::IntentResultV1,
    declared_targets: &[String],
    read_ctx: &ag::blocks::AgentReadContext,
    active_doc_path: Option<&str>,
    active_snapshot: Option<&ag::blocks::AgentEditableSnapshot>,
    history: &[ag::context::HistTurn],
    prompt: &str,
    mention_block: Option<&str>,
    requires_edit_tool: bool,
) -> Result<PreparedTurnPrompt, IpcError> {
    // P0 当前文档块视图（PI-AG-08E/08F/08G）。短文档注入完整块内容；超长文档注入块目录并要求工具读取。
    // 2026-08-09：DOC_CTX_CAP 9000→16000（AG-LIMIT-01 扩大内联注入上限，长文档测试可直接编辑；G5 标定）。
    let needs_current_document = intent_result
        .required_context
        .iter()
        .any(|required| required == "current_document_logical_state");
    let current_doc_prompt_context = match (
        needs_current_document,
        active_doc_path.as_deref(),
        active_snapshot.as_ref(),
    ) {
        (true, Some(path), Some(snapshot)) => {
            let (has_bdocx_tables, has_bdocx_images) =
                bdocx_structure_presence_from_context(&read_ctx, path, snapshot);
            build_current_doc_prompt_context(
                path,
                snapshot,
                DOC_CTX_CAP,
                has_bdocx_tables,
                has_bdocx_images,
            )
            .map_err(capability_error_to_ipc)?
        }
        _ => empty_current_doc_prompt_context(),
    };
    let current_doc_requires_tool_read =
        requires_edit_tool && current_doc_prompt_context.requires_read_before_edit;
    let doc_ctx = current_doc_prompt_context.text;

    // 历史选择必须先于 PromptPackage 构建，确保 trace 中的历史 envelope 与实际模型输入完全一致。
    // 为 Runtime 控制层预留固定空间后，按既有同文档/显式沿用规则确定性保留最近历史。
    // 2026-08-09：CTX_BUDGET_CHARS 16000→24000、RUNTIME_CONTROL_RESERVE_CHARS 4000→6000
    // （AG-LIMIT-01 与 DOC_CTX_CAP 扩大联动，避免长文档内联后历史预算归零）。
    let history_budget = ag::prompt_runtime::history_budget_chars(
        ag::prompts::SYSTEM_PROMPT_CHAT,
        doc_ctx.as_deref(),
        &prompt,
        mention_block.as_deref(),
    );
    let mut assembled_messages = assemble_agent_turn_messages(
        ag::prompts::SYSTEM_PROMPT_CHAT,
        None,
        &history,
        active_doc_path.as_deref(),
        history_budget,
        mention_block.as_deref(),
        doc_ctx.as_deref(),
        &prompt,
    );

    let capability_grant = ag::tool_registry::resolve_capability_grant(
        &runtime_profile,
        &turn_id,
        &intent_result,
        &declared_targets,
    );
    let tools = ag::tool_registry::tool_definitions_json_for_grant(&capability_grant)
        .map_err(capability_error_to_ipc)?;
    let task_plan_runtime = state.db.lock().ok().and_then(|conn| {
        ag::task::active_plan_by_run(&conn, &task_run_id)
            .ok()
            .flatten()
            .map(|plan| task_plan_runtime_projection(&plan))
    });
    let tool_manifest_hash =
        ag::context::sha256_text(&serde_json::to_string(&tools).map_err(|error| {
            IpcError::new(
                "AG_TOOL_MANIFEST_INVALID",
                &format!("本轮工具清单无法序列化: {error}"),
                false,
                false,
            )
        })?);
    let mut prompt_envelopes = Vec::new();
    if let (Some(path), Some(context)) = (active_doc_path.as_deref(), doc_ctx.as_deref()) {
        let freshness = active_snapshot
            .as_ref()
            .map(|snapshot| {
                format!(
                    "session_version={};content_hash={}",
                    snapshot.session_version.unwrap_or_default(),
                    snapshot.content_hash.as_deref().unwrap_or("unknown")
                )
            })
            .unwrap_or_else(|| "unknown".to_string());
        prompt_envelopes.push(ag::context::prompt_data_envelope(
            ag::context::PromptSourceTypeV1::CurrentDocument,
            path,
            "active_editor_logical_state",
            &freshness,
            ag::context::PromptTrustLevelV1::CurrentFact,
            context,
        ));
    }
    if let Some(reference_context) = mention_block.as_deref() {
        prompt_envelopes.push(ag::context::prompt_data_envelope(
            ag::context::PromptSourceTypeV1::ExplicitReference,
            &format!("references-{turn_id}"),
            "user_explicit_reference",
            "validated_at_turn_start",
            ag::context::PromptTrustLevelV1::UserProvidedData,
            reference_context,
        ));
    }
    if let Some(history_payload) = assembled_messages.history_payload.as_deref() {
        prompt_envelopes.push(ag::context::prompt_data_envelope(
            ag::context::PromptSourceTypeV1::SessionHistory,
            &session_id,
            "same_session_persisted_history",
            "before_current_turn",
            ag::context::PromptTrustLevelV1::HistoricalData,
            history_payload,
        ));
    }
    let prompt_package = ag::context::build_prompt_package(
        intent_result.clone(),
        prompt_envelopes,
        &prompt,
        PROMPT_TOKEN_BUDGET,
        &tool_manifest_hash,
    );
    insert_runtime_control_message(
        &mut assembled_messages,
        &runtime_profile,
        &prompt_package,
        &capability_grant,
        task_plan_runtime.as_ref(),
    );
    let messages = assembled_messages.messages;
    let _carryover = assembled_messages.carryover;
    let _dropped_cross_doc_history = assembled_messages.dropped_cross_doc_history;

    Ok(PreparedTurnPrompt {
        current_doc_requires_tool_read,
        messages,
        tools,
        capability_grant,
        prompt_package,
    })
}
