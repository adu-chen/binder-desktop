//! PI-AG-08A · 工具注册表（只读工具 + 结构操作 + WriteIntent 权限分级）。
//!
//! 为 SM-AG-02 `tool::validate_tool` 提供「注册清单 + 权限边界」数据（BR-AG-STATE-002）：模型请求的工具
//! 必须在注册清单内且通过权限校验方可执行。清单 v1 覆盖文档列表/读取/搜索/块读取 + 非破坏性结构操作 + 修改建议。
//!
//! **执行实现**（read_doc/list_docs/search_text/list_blocks 调统一 AgentEditableSnapshot；
//! plan_workspace_operation 只生成 WorkspaceOperationPlan + DecisionRequest；import_image_asset 经 DOC/WS 受控复制；
//! edit_blocked 结构化结束不可安全执行的编辑请求；propose_edit 产出 ChangeProposal→DE）归
//! PI-AG-08B（与模型循环 PI-AG-11 配套，避免重复 lib.rs 既有 route_*）。本模块只持有注册与权限数据。

use super::tool::{validate_tool, ToolDecision};
use super::turn::AGENT_EDIT_1_0_PROFILE;
use std::collections::BTreeSet;

/// Prompt 装配时已核验的当前格式/动态事实；只用于继续收窄，不作为独立授权来源。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct CapabilityFactorsV2 {
    pub(super) has_task_run: bool,
    pub(super) active_bdocx: bool,
    pub(super) table_ops_available: bool,
    pub(super) image_ops_available: bool,
    pub(super) tools_frozen: bool,
    pub(super) decision_receipt_ids: Vec<String>,
    pub(super) gate_attempt: u32,
    pub(super) gate_passed: bool,
    pub(super) remaining_tool_calls: u32,
    pub(super) cancelled: bool,
    pub(super) terminal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityBudgetsV2 {
    pub max_tool_calls: u32,
    pub remaining_tool_calls: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum CapabilityGrantPurposeV2 {
    PromptManifest,
    ToolCall {
        tool_call_id: String,
        computed_at_seq: u64,
    },
    RuntimeWorkspaceEffect {
        plan_id: String,
        decision_request_id: String,
        computed_at_seq: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityGrantV2 {
    pub version: String,
    pub grant_id: String,
    pub profile: String,
    pub turn_id: String,
    pub computed_at_seq: u64,
    pub allowed_tools: Vec<String>,
    pub target_scope: Vec<String>,
    pub readable_scope: Vec<String>,
    pub explicit_edit_targets_hash: String,
    pub permitted_effect_paths: Vec<String>,
    pub format_capability_hash: String,
    pub dynamic_capability_hash: String,
    pub decision_receipt_ids: Vec<String>,
    pub budgets: CapabilityBudgetsV2,
    pub expires_after_tool_call: Option<String>,
    pub expires_after_runtime_effect: Option<String>,
    pub grant_kind: String,
    pub gate_attempt: u32,
    pub gate_passed: bool,
    pub parameter_constraints: Vec<String>,
    pub authorization_facts_hash: String,
}

impl CapabilityGrantV2 {
    pub fn allows(&self, tool_name: &str) -> bool {
        self.allowed_tools
            .iter()
            .any(|allowed| allowed == tool_name)
    }

    pub fn is_prompt_manifest(&self) -> bool {
        self.grant_kind == "prompt_manifest"
    }
}

#[derive(Debug, Default)]
pub struct CapabilityGrantUseTrackerV2 {
    consumed_grant_ids: BTreeSet<String>,
    consumed_tool_call_ids: BTreeSet<String>,
}

/**
 * @GOV
 * codes:    BR-AG-STATE-002-DATA-AG-AG-TOOL-039, BR-AG-DATA-004-DATA-AG-AG-TOOL-039
 * type:     DATA
 * chain:    AG-TURN, AG-TOOL
 * rules:    BR-AG-STATE-002, BR-AG-DATA-004
 * term_ref: TERM-AG-013, TERM-AG-014, TERM-AG-019
 * boundary: in=RuntimeProfileExplicitEditTargetSetGateDecisionFormatDynamicBudgetCancellationTerminalAndToolCallFacts | out=PromptManifestProjectionOrSingleUseCapabilityGrantV2IndependentOfIntentText | does_not=useIntentTaskPlanPromptHistoryToolResultContentOrRegistryPresenceAloneAsAuthorization
 */
/// Prompt Manifest 与 ToolCall 执行授权共用同一硬事实算法。Prompt 投影不绑定动作，因此不可用于执行；
/// 执行 grant 必须绑定真实 Gate 后的唯一 ToolCall 与 append-only 事件序号，校验尝试后立即失效。
pub(super) fn resolve_capability_grant_v2(
    profile: &str,
    turn_id: &str,
    explicit_edit_targets: &[String],
    mut factors: CapabilityFactorsV2,
    purpose: CapabilityGrantPurposeV2,
) -> CapabilityGrantV2 {
    let execution_is_valid = match &purpose {
        CapabilityGrantPurposeV2::PromptManifest => true,
        CapabilityGrantPurposeV2::ToolCall {
            tool_call_id,
            computed_at_seq,
        } => {
            !tool_call_id.trim().is_empty()
                && *computed_at_seq > 0
                && factors.gate_passed
                && factors.remaining_tool_calls > 0
        }
        CapabilityGrantPurposeV2::RuntimeWorkspaceEffect {
            plan_id,
            decision_request_id,
            computed_at_seq,
        } => {
            !plan_id.trim().is_empty()
                && !decision_request_id.trim().is_empty()
                && *computed_at_seq > 0
                && factors
                    .decision_receipt_ids
                    .iter()
                    .any(|receipt| receipt == decision_request_id)
        }
    };
    let is_runtime_workspace_effect = matches!(
        &purpose,
        CapabilityGrantPurposeV2::RuntimeWorkspaceEffect { .. }
    );
    let runtime_is_active = profile == AGENT_EDIT_1_0_PROFILE
        && !factors.tools_frozen
        && !factors.cancelled
        && !factors.terminal
        && factors.remaining_tool_calls > 0
        && execution_is_valid
        && !is_runtime_workspace_effect;
    let mut allowed_tools = if runtime_is_active {
        vec![
            "list_docs",
            "workspace_glob",
            "search_text",
            "read_doc",
            "list_blocks",
            "report_project",
            "plan_workspace_operation",
            "propose_decision",
        ]
    } else {
        Vec::new()
    };
    if !explicit_edit_targets.is_empty() && runtime_is_active {
        allowed_tools.extend(["propose_edit", "edit_blocked"]);
    }
    if runtime_is_active && factors.active_bdocx && factors.table_ops_available {
        allowed_tools.push("list_tables");
    }
    if runtime_is_active && factors.active_bdocx && factors.image_ops_available {
        allowed_tools.extend(["list_images", "import_image_asset"]);
    }
    allowed_tools.retain(|tool| is_registered(tool));
    allowed_tools.sort_unstable();
    allowed_tools.dedup();
    let allowed_tools = allowed_tools
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut target_scope = explicit_edit_targets.to_vec();
    target_scope.sort();
    target_scope.dedup();
    factors.decision_receipt_ids.sort();
    factors.decision_receipt_ids.dedup();
    let mut readable_scope = target_scope.clone();
    if runtime_is_active {
        readable_scope.extend([
            "workspace_search".to_string(),
            "workspace_read_all".to_string(),
        ]);
    }
    readable_scope.sort();
    readable_scope.dedup();
    let explicit_edit_targets_hash = super::context::sha256_text(&target_scope.join("\n"));
    let format_capability_hash = super::context::sha256_text(&format!(
        "active_bdocx={};table_ops={};image_ops={}",
        factors.active_bdocx, factors.table_ops_available, factors.image_ops_available
    ));
    let dynamic_capability_hash = super::context::sha256_text(&format!(
        "has_task_run={};decision_receipts={}",
        factors.has_task_run,
        factors.decision_receipt_ids.join(",")
    ));
    let mut permitted_effect_paths = Vec::new();
    if allowed_tools.iter().any(|tool| {
        matches!(
            tool.as_str(),
            "list_docs"
                | "workspace_glob"
                | "search_text"
                | "read_doc"
                | "list_blocks"
                | "list_tables"
                | "list_images"
        )
    }) {
        permitted_effect_paths.push("workspace.read".to_string());
    }
    if allowed_tools.iter().any(|tool| tool == "report_project") {
        permitted_effect_paths.push("ag.project_report.compatibility".to_string());
    }
    if allowed_tools
        .iter()
        .any(|tool| tool == "plan_workspace_operation")
    {
        permitted_effect_paths.push("workspace.plan_operation".to_string());
    }
    if allowed_tools.iter().any(|tool| tool == "propose_decision") {
        permitted_effect_paths.push("ag.decision.propose".to_string());
    }
    if allowed_tools.iter().any(|tool| tool == "propose_edit") {
        permitted_effect_paths.push("de.generate_diff".to_string());
    }
    if allowed_tools.iter().any(|tool| tool == "edit_blocked") {
        permitted_effect_paths.push("ag.edit_blocked".to_string());
    }
    if allowed_tools
        .iter()
        .any(|tool| tool == "import_image_asset")
    {
        permitted_effect_paths.push("doc.import_image_asset".to_string());
    }
    if is_runtime_workspace_effect
        && execution_is_valid
        && profile == AGENT_EDIT_1_0_PROFILE
        && !factors.cancelled
        && !factors.terminal
    {
        permitted_effect_paths.push("workspace.execute_operation".to_string());
    }
    permitted_effect_paths.sort();
    let (computed_at_seq, expires_after_tool_call, expires_after_runtime_effect, grant_kind) =
        match purpose {
            CapabilityGrantPurposeV2::PromptManifest => {
                (0, None, None, "prompt_manifest".to_string())
            }
            CapabilityGrantPurposeV2::ToolCall {
                tool_call_id,
                computed_at_seq,
            } => (
                computed_at_seq,
                Some(tool_call_id),
                None,
                "tool_call".to_string(),
            ),
            CapabilityGrantPurposeV2::RuntimeWorkspaceEffect {
                plan_id,
                decision_request_id,
                computed_at_seq,
            } => (
                computed_at_seq,
                None,
                Some(format!(
                    "workspace.execute_operation:{plan_id}:{decision_request_id}"
                )),
                "runtime_effect".to_string(),
            ),
        };
    let mut parameter_constraints = vec![
        "active_workspace_only".to_string(),
        "no_direct_document_write".to_string(),
        "proposal_applies_on_user_confirm".to_string(),
        "propose_edit_requires_explicit_edit_target_set".to_string(),
        "workspace_execute_requires_decision_receipt".to_string(),
    ];
    if factors.active_bdocx && factors.table_ops_available {
        parameter_constraints.push("bdocx_table_ops_allowed".to_string());
    }
    if factors.active_bdocx && factors.image_ops_available {
        parameter_constraints.push("bdocx_image_ops_allowed".to_string());
    }
    let authorization_facts_hash = super::context::sha256_text(
        &serde_json::to_string(&serde_json::json!({
            "profile": profile,
            "turnId": turn_id,
            "computedAtSeq": computed_at_seq,
            "allowedTools": &allowed_tools,
            "readableScope": &readable_scope,
            "explicitEditTargetsHash": &explicit_edit_targets_hash,
            "permittedEffectPaths": &permitted_effect_paths,
            "formatCapabilityHash": &format_capability_hash,
            "dynamicCapabilityHash": &dynamic_capability_hash,
            "decisionReceiptIds": &factors.decision_receipt_ids,
            "remainingToolCalls": factors.remaining_tool_calls,
            "expiresAfterToolCall": &expires_after_tool_call,
            "expiresAfterRuntimeEffect": &expires_after_runtime_effect,
            "grantKind": &grant_kind,
            "gateAttempt": factors.gate_attempt,
            "gatePassed": factors.gate_passed,
            "cancelled": factors.cancelled,
            "terminal": factors.terminal,
        }))
        .unwrap_or_default(),
    );
    let grant_id = format!("capability-grant:{}", authorization_facts_hash);
    CapabilityGrantV2 {
        version: "capability-grant-v2".to_string(),
        grant_id,
        profile: profile.to_string(),
        turn_id: turn_id.to_string(),
        computed_at_seq,
        allowed_tools,
        target_scope,
        readable_scope,
        explicit_edit_targets_hash,
        permitted_effect_paths,
        format_capability_hash,
        dynamic_capability_hash,
        decision_receipt_ids: factors.decision_receipt_ids,
        budgets: CapabilityBudgetsV2 {
            max_tool_calls: 6,
            remaining_tool_calls: factors.remaining_tool_calls,
        },
        expires_after_tool_call,
        expires_after_runtime_effect,
        grant_kind,
        gate_attempt: factors.gate_attempt,
        gate_passed: factors.gate_passed,
        parameter_constraints,
        authorization_facts_hash,
    }
}

/// 模型不可见的 WorkspaceOperation 执行守卫。它只消费绑定到同一 source turn、plan、
/// DecisionRequest 与当前重算序号的 runtime effect grant；真实资源身份和基线仍由 WS 再校验。
/**
 * @GOV
 * codes:    BR-SYS-STATE-008-GUARD-AG-AG-TOOL-040, BR-WS-STATE-003-GUARD-AG-AG-TOOL-040
 * type:     GUARD
 * chain:    AG-TOOL, WS-OPERATION
 * rules:    BR-SYS-STATE-008, BR-WS-STATE-003
 * term_ref: TERM-AG-014, TERM-AG-015, TERM-WS-003
 * boundary: in=RuntimeEffectCapabilityGrantSourceTurnWorkspaceOperationPlanAndResolvedDecisionRequest | out=ExactInternalWorkspaceExecutionAuthorizationOrStableRejection | does_not=registerModelVisibleExecuteToolOrReplaceWorkspaceRevalidation
 */
pub(super) fn validate_workspace_operation_effect_for_grant(
    grant: &CapabilityGrantV2,
    turn_id: &str,
    plan_id: &str,
    decision_request_id: &str,
    computed_at_seq: u64,
) -> ToolDecision {
    let expected_effect = format!("workspace.execute_operation:{plan_id}:{decision_request_id}");
    if grant.profile != AGENT_EDIT_1_0_PROFILE
        || grant.grant_kind != "runtime_effect"
        || grant.turn_id != turn_id
        || grant.computed_at_seq == 0
        || grant.computed_at_seq != computed_at_seq
        || grant.expires_after_tool_call.is_some()
        || grant.expires_after_runtime_effect.as_deref() != Some(expected_effect.as_str())
        || !grant
            .decision_receipt_ids
            .iter()
            .any(|receipt| receipt == decision_request_id)
        || !grant
            .permitted_effect_paths
            .iter()
            .any(|effect| effect == "workspace.execute_operation")
        || !grant.allowed_tools.is_empty()
    {
        return ToolDecision::Reject(
            "WorkspaceOperation 运行时授权已过期或与当前确认事实不匹配".to_string(),
        );
    }
    ToolDecision::Allow
}

/**
 * @GOV
 * codes:    BR-AG-STATE-002-GUARD-AG-AG-TOOL-035, BR-SYS-STATE-007-GUARD-AG-AG-TOOL-035, BR-AG-DATA-003-GUARD-AG-AG-TOOL-035, BR-WS-DATA-001-GUARD-AG-AG-TOOL-035
 * type:     GUARD
 * chain:    AG-TOOL
 * rules:    BR-AG-STATE-002, BR-SYS-STATE-007, BR-AG-DATA-003, BR-WS-DATA-001
 * term_ref: TERM-AG-014, TERM-AG-017, TERM-AG-019, TERM-WS-001
 * boundary: in=ToolCallParametersAndCurrentCapabilityGrant | out=RegisteredAllowedTargetedAndParameterValidToolCallOrStableRejection | does_not=upgradeGrantFromRepeatedFailureOrToolParameters
 */
pub fn validate_requested_tool_for_grant(
    name: &str,
    params: &serde_json::Value,
    grant: &CapabilityGrantV2,
    turn_id: &str,
    tool_call_id: &str,
    computed_at_seq: u64,
    workspace_root: Option<&std::path::Path>,
    use_tracker: &mut CapabilityGrantUseTrackerV2,
) -> ToolDecision {
    if grant.profile != AGENT_EDIT_1_0_PROFILE
        || grant.turn_id != turn_id
        || grant.computed_at_seq == 0
        || grant.computed_at_seq != computed_at_seq
        || grant.expires_after_tool_call.as_deref() != Some(tool_call_id)
        || !grant.gate_passed
        || grant.budgets.remaining_tool_calls == 0
    {
        return ToolDecision::Reject("工具授权已过期或不属于当前回合".to_string());
    }
    if use_tracker.consumed_grant_ids.contains(&grant.grant_id)
        || use_tracker.consumed_tool_call_ids.contains(tool_call_id)
    {
        return ToolDecision::Reject("工具授权或 ToolCall 已使用，不能重复执行".to_string());
    }
    // 授权在第一次校验尝试即消费；参数失败、未知工具或执行失败都不能重用或扩权。
    use_tracker
        .consumed_grant_ids
        .insert(grant.grant_id.clone());
    use_tracker
        .consumed_tool_call_ids
        .insert(tool_call_id.to_string());
    let registered = is_registered(name);
    if !registered {
        return validate_tool(false, false, false);
    }
    if !grant.allows(name) {
        return ToolDecision::Reject(format!("工具 {name} 未获本轮 CapabilityGrant 授权"));
    }
    if !params.is_object() {
        return ToolDecision::Reject(format!("工具 {name} 参数必须是 JSON 对象"));
    }
    let normalize_workspace_path = |raw: &str| -> Result<String, String> {
        let root =
            workspace_root.ok_or_else(|| "缺少 active workspace，工具目标无法校验".to_string())?;
        let absolute = crate::workspace::resolve_in_workspace(root, raw)
            .map_err(|error| format!("工具目标越出 active workspace：{}", error.message))?;
        let relative = absolute
            .strip_prefix(root)
            .map_err(|_| "工具目标无法归一到 active workspace".to_string())?;
        let normalized = relative.to_string_lossy().replace('\\', "/");
        if normalized.is_empty() {
            return Err("workspace 根目录不能作为文档工具目标".to_string());
        }
        Ok(normalized)
    };
    if name == "propose_decision" {
        let kind = params.get("kind").and_then(|value| value.as_str());
        let summary = params
            .get("summary")
            .and_then(|value| value.as_str())
            .filter(|value| !value.trim().is_empty());
        if !matches!(
            kind,
            Some("permission" | "plan_choice" | "scope" | "manual_confirm")
        ) || summary.is_none()
            || params.get("detail").is_some_and(|value| !value.is_string())
            || params.get("options").is_some_and(|value| {
                !value.as_array().is_some_and(|options| {
                    !options.is_empty()
                        && options.iter().all(|option| {
                            option
                                .as_str()
                                .is_some_and(|option| !option.trim().is_empty())
                        })
                })
            })
        {
            return ToolDecision::Reject(
                "propose_decision 的 kind、summary、detail 或 options 参数无效".to_string(),
            );
        }
        if grant
            .parameter_constraints
            .iter()
            .any(|constraint| constraint == "propose_decision_scope_only")
            && params.get("kind").and_then(|value| value.as_str()) != Some("scope")
        {
            return ToolDecision::Reject(
                "本轮 CapabilityGrant 只允许 scope 类型 DecisionRequest".to_string(),
            );
        }
        if let Err(reason) = validate_scope_decision_parameters(params) {
            return ToolDecision::Reject(reason);
        }
    }
    if matches!(
        name,
        "read_doc" | "list_blocks" | "list_tables" | "list_images"
    ) {
        let raw_target = params
            .get("path")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let target = match normalize_workspace_path(raw_target) {
            Ok(target) => target,
            Err(reason) => return ToolDecision::Reject(reason),
        };
        if !grant.readable_scope.iter().any(|allowed| allowed == &target)
            && !grant
                .readable_scope
                .iter()
                .any(|allowed| allowed == "workspace_search")
            // P4 修复（AG-R2-01）：workspace 内只读放行（X-FORBID-004 限定 workspace 内，
            // 仅影响读取，不授写；exec 层仍校验实际路径在 workspace 内）。
            && !grant.readable_scope.iter().any(|allowed| allowed == "workspace_read_all")
        {
            return ToolDecision::Reject("只读工具目标未在本轮可读范围内".to_string());
        }
    }
    if name == "propose_edit" {
        if grant
            .parameter_constraints
            .iter()
            .any(|constraint| constraint == "new_document_requires_workspace_operation_plan")
        {
            return ToolDecision::Reject(
                "新建文档必须使用 plan_workspace_operation；确认后才建立真实资源并投影初始内容 Diff"
                    .to_string(),
            );
        }
        let raw_target = params
            .get("path")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        if raw_target.trim().is_empty() {
            return ToolDecision::Reject("propose_edit 缺少目标 path".to_string());
        }
        let target = match normalize_workspace_path(raw_target) {
            Ok(target) => target,
            Err(reason) => return ToolDecision::Reject(reason),
        };
        if !grant.target_scope.iter().any(|allowed| allowed == &target) {
            return ToolDecision::Reject(format!(
                "既有文档 {target} 不在当前 ExplicitEditTargetSet"
            ));
        }
        let operation = params.get("operation").and_then(|value| value.as_str());
        if !matches!(
            operation,
            Some(
                "replace_block"
                    | "replace_document"
                    | "block_ops"
                    | "style_ops"
                    | "table_ops"
                    | "image_ops"
            )
        ) {
            return ToolDecision::Reject("propose_edit.operation 未注册或缺失".to_string());
        }
        match operation {
            Some("table_ops")
                if !grant
                    .parameter_constraints
                    .iter()
                    .any(|constraint| constraint == "bdocx_table_ops_allowed") =>
            {
                return ToolDecision::Reject("本轮未获 BDocx 表格动态能力".to_string());
            }
            Some("image_ops")
                if !grant
                    .parameter_constraints
                    .iter()
                    .any(|constraint| constraint == "bdocx_image_ops_allowed") =>
            {
                return ToolDecision::Reject("本轮未获 BDocx 图片动态能力".to_string());
            }
            _ => {}
        }
    }
    if name == "list_docs" {
        if params.get("path").is_some_and(|value| !value.is_string()) {
            return ToolDecision::Reject("list_docs.path 必须是字符串".to_string());
        }
        if let Some(path) = params.get("path").and_then(|value| value.as_str()) {
            if let Err(reason) = normalize_workspace_path(path) {
                return ToolDecision::Reject(reason);
            }
        }
    }
    if name == "workspace_glob"
        && !params
            .get("pattern")
            .and_then(|value| value.as_str())
            .is_some_and(|value| !value.trim().is_empty())
    {
        return ToolDecision::Reject("workspace_glob.pattern 必须是非空字符串".to_string());
    }
    if name == "search_text"
        && params
            .get("query")
            .and_then(|value| value.as_str())
            .is_none()
    {
        return ToolDecision::Reject("search_text.query 必须是字符串".to_string());
    }
    if name == "edit_blocked" {
        let code = params
            .get("code")
            .and_then(|value| value.as_str())
            .filter(|value| !value.trim().is_empty() && value.len() <= 80);
        let reason = params
            .get("reason")
            .and_then(|value| value.as_str())
            .filter(|value| !value.trim().is_empty() && value.len() <= 1000);
        if code.is_none() || reason.is_none() {
            return ToolDecision::Reject("edit_blocked 的 code/reason 参数无效".to_string());
        }
    }
    if name == "import_image_asset" {
        let source_path = params
            .get("source_path")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        if let Err(reason) = normalize_workspace_path(source_path) {
            return ToolDecision::Reject(reason);
        }
    }
    if name == "plan_workspace_operation" {
        let operations = params
            .get("operations")
            .and_then(|value| value.as_array())
            .filter(|items| !items.is_empty());
        let Some(operations) = operations else {
            return ToolDecision::Reject(
                "plan_workspace_operation.operations 必须是非空数组".to_string(),
            );
        };
        let create_count = operations
            .iter()
            .filter(|item| {
                item.get("kind").and_then(|value| value.as_str()) == Some("create_document")
            })
            .count();
        let initial_content = params
            .get("initialContent")
            .and_then(|value| value.as_str())
            .filter(|value| !value.trim().is_empty());
        if create_count > 0 && (create_count != 1 || initial_content.is_none()) {
            return ToolDecision::Reject(
                "新建文档计划必须且只能包含一个 create_document，并提供确认前冻结的完整 initialContent"
                    .to_string(),
            );
        }
        if create_count == 0 && params.get("initialContent").is_some() {
            return ToolDecision::Reject(
                "initialContent 只能与单个 create_document 操作绑定".to_string(),
            );
        }
        for operation in operations {
            let kind = operation.get("kind").and_then(|value| value.as_str());
            let resource_kind = operation
                .get("resourceKind")
                .and_then(|value| value.as_str());
            if !matches!(
                kind,
                Some(
                    "create_document"
                        | "create_folder"
                        | "rename_resource"
                        | "move_resource"
                        | "delete_resource"
                )
            ) || !matches!(resource_kind, Some("file" | "folder"))
            {
                return ToolDecision::Reject(
                    "WorkspaceOperation 的 kind 或 resourceKind 无效".to_string(),
                );
            }
            let requires_source = matches!(
                kind,
                Some("rename_resource" | "move_resource" | "delete_resource")
            );
            let requires_target = matches!(
                kind,
                Some("create_document" | "create_folder" | "rename_resource" | "move_resource")
            );
            if requires_source
                && !operation
                    .get("sourcePath")
                    .and_then(|value| value.as_str())
                    .is_some_and(|value| !value.trim().is_empty())
            {
                return ToolDecision::Reject(
                    "WorkspaceOperation 缺少必需的 sourcePath".to_string(),
                );
            }
            if requires_target
                && !operation
                    .get("targetPath")
                    .and_then(|value| value.as_str())
                    .is_some_and(|value| !value.trim().is_empty())
            {
                return ToolDecision::Reject(
                    "WorkspaceOperation 缺少必需的 targetPath".to_string(),
                );
            }
            for field in ["sourcePath", "targetPath"] {
                if let Some(path) = operation.get(field).and_then(|value| value.as_str()) {
                    if let Err(reason) = normalize_workspace_path(path) {
                        return ToolDecision::Reject(reason);
                    }
                }
            }
        }
    }
    validate_tool(true, true, false)
}

/**
 * @GOV
 * codes:    BR-AG-DATA-003-GUARD-AG-AG-TOOL-033, BR-AG-STATE-002-GUARD-AG-AG-TOOL-033
 * type:     GUARD
 * chain:    AG-TOOL
 * rules:    BR-AG-DATA-003, BR-AG-STATE-002
 * term_ref: TERM-AG-019, TERM-AG-017
 * boundary: in=DecisionRequestToolParameters | out=ScopeDecisionWithFrozenNonEmptyTargetPathsOrRejection | does_not=appendTargetsOrTreatReadResultsAsAuthorization
 */
fn validate_scope_decision_parameters(params: &serde_json::Value) -> Result<(), String> {
    let kind = params.get("kind").and_then(|value| value.as_str());
    let target_paths = params.get("targetPaths");
    if kind == Some("scope") {
        let valid = target_paths
            .and_then(|value| value.as_array())
            .is_some_and(|paths| {
                !paths.is_empty()
                    && paths
                        .iter()
                        .all(|path| path.as_str().is_some_and(|path| !path.trim().is_empty()))
            });
        if !valid {
            return Err(
                "scope DecisionRequest 必须提供非空 targetPaths 既有文档路径清单".to_string(),
            );
        }
    } else if target_paths.is_some() {
        return Err("targetPaths 只能用于 kind=scope 的 DecisionRequest".to_string());
    }
    Ok(())
}

/// 工具权限分级。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolPermission {
    /// 只读：读文档/列文档/搜索，经 WS/KB 且限 active workspace（X-FORBID-004）。
    ReadOnly,
    /// 结构操作：移动/重命名/建目录，经 WS 边界执行；不修改文档内容，不走 DE。
    StructureWrite,
    /// 写意图：产出修改建议走 DE diff（X-FORBID-006）；**非直接写盘**。
    WriteIntent,
    /// 任务控制意图：只创建任务级确认点，不改变文档、Diff 或文件树。
    TaskControl,
}

/// 工具定义（注册项）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolDef {
    pub name: &'static str,
    pub permission: ToolPermission,
}

/// v1 工具集（A-AG-X-L-02 PI-AG-08）。
///
/// propose_edit 标记 WriteIntent 表示其**执行**走 DE 修改建议路径，但它本身不直写盘
/// （`direct_write_intent=false`）；plan_workspace_operation 只生成前置裁决计划，不直写磁盘；
/// import_image_asset 是受控复制白名单，不直接修改文档内容。
pub const TOOLS: &[ToolDef] = &[
    ToolDef {
        name: "read_doc",
        permission: ToolPermission::ReadOnly,
    },
    ToolDef {
        name: "list_docs",
        permission: ToolPermission::ReadOnly,
    },
    ToolDef {
        name: "search_text",
        permission: ToolPermission::ReadOnly,
    },
    ToolDef {
        name: "list_blocks",
        permission: ToolPermission::ReadOnly,
    },
    ToolDef {
        name: "workspace_glob",
        permission: ToolPermission::ReadOnly,
    },
    ToolDef {
        name: "list_tables",
        permission: ToolPermission::ReadOnly,
    },
    ToolDef {
        name: "list_images",
        permission: ToolPermission::ReadOnly,
    },
    ToolDef {
        name: "report_project",
        permission: ToolPermission::ReadOnly,
    },
    ToolDef {
        name: "plan_workspace_operation",
        permission: ToolPermission::TaskControl,
    },
    ToolDef {
        name: "import_image_asset",
        permission: ToolPermission::StructureWrite,
    },
    ToolDef {
        name: "edit_blocked",
        permission: ToolPermission::WriteIntent,
    },
    ToolDef {
        name: "propose_edit",
        permission: ToolPermission::WriteIntent,
    },
    ToolDef {
        name: "propose_decision",
        permission: ToolPermission::TaskControl,
    },
];

/// 按名查注册项。
pub fn lookup(name: &str) -> Option<ToolDef> {
    TOOLS.iter().copied().find(|t| t.name == name)
}

/// 是否在注册清单内。
pub fn is_registered(name: &str) -> bool {
    lookup(name).is_some()
}

/// 发送给模型的工具定义（OpenAI function-calling 格式）。
/// dev：只读工具 + 结构工具 + propose_edit 写意图工具；propose_edit 走 ChangeProposal→DE，不直接写文档内容。
pub fn tool_definitions_json(
) -> Result<serde_json::Value, super::bdocx_capabilities::CapabilityValidationError> {
    let propose_edit_parameters = super::bdocx_capabilities::propose_edit_parameters_schema()?;
    Ok(serde_json::json!([
        {
            "type": "function",
            "function": {
                "name": "workspace_glob",
                "description": "按文件名模式在工作区内定位文档（通配：* 匹配任意字符序列，? 匹配单个字符）。如 workspace_glob(pattern=\"**/寓言*.txt\") 返回匹配的文件路径列表（不含正文）。用于替代逐目录展开 list_docs，快速定位目标文件。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "文件名通配模式（相对 workspace 根，支持 * 与 ?；如 \"*寓言*.txt\"、\"文档/*.docx\"）。"
                        }
                    },
                    "required": ["pattern"],
                    "additionalProperties": false
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_docs",
                "description": "感知当前工作区（workspace）结构。不传 path：返回工作区概览——顶层目录及各自文件数、根级文件列表、总文件数（工作区较小时直接返回全部文件，属豁免）；传 path（目录相对路径）：返回该目录的直接子项（一层，不递归）。文件内容一律用 read_doc 按需读取（本工具永不返回正文）。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "可选。目录相对路径（如 \"文学作品\"）；传此参数时返回该目录的直接子项。"
                        }
                    },
                    "additionalProperties": false
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "report_project",
                "description": "以结构化 ProjectReport 结束只读检查/搜索/分析。必须如实列出 requestedScope、可证明的 readItems、unreadItems、findings 和 uncertainties。readItems 必须逐字使用本轮证据：初始 AgentContextFacts 为 coverage=session_metadata/evidence=agent_context_facts；提示词完整当前文档为 full_document/prompt_current_document，长文档目录为 structure_index/prompt_current_document；read_doc 全文为 full_document/read_doc，blocks 参数为 selected_blocks/read_doc；list_blocks/list_tables/list_images 为 structure_index/同名工具；list_docs/workspace_glob 为 workspace_structure/同名工具；search_text 命中为 search_excerpt/search_text。目录和搜索片段不得声称读完正文。该工具只生成只读报告证据，不修改任何内容或授权。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "version": { "type": "string", "enum": ["project-report-v1"] },
                        "requestedScope": { "type": "array", "items": { "type": "string" } },
                        "readItems": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "relPath": { "type": "string" },
                                    "source": { "type": "string" },
                                    "coverage": { "type": "string", "enum": ["session_metadata", "workspace_structure", "structure_index", "search_excerpt", "selected_blocks", "full_document"] },
                                    "evidence": { "type": "string" }
                                },
                                "required": ["relPath", "source", "coverage", "evidence"],
                                "additionalProperties": false
                            }
                        },
                        "unreadItems": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "relPath": { "type": "string" },
                                    "reason": { "type": "string" }
                                },
                                "required": ["relPath", "reason"],
                                "additionalProperties": false
                            }
                        },
                        "findings": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "statement": { "type": "string" },
                                    "sourcePaths": { "type": "array", "items": { "type": "string" } },
                                    "uncertainty": { "type": "string", "enum": ["none", "low", "medium", "high"] }
                                },
                                "required": ["statement", "sourcePaths", "uncertainty"],
                                "additionalProperties": false
                            }
                        },
                        "uncertainties": { "type": "array", "minItems": 1, "items": { "type": "string" } }
                    },
                    "required": ["version", "requestedScope", "readItems", "unreadItems", "findings", "uncertainties"],
                    "additionalProperties": false
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "plan_workspace_operation",
                "description": "为工作区资源创建、建目录、重命名、移动或删除生成不可变操作计划和独立确认卡。工具成功只表示正在等待用户确认；确认前不修改磁盘，也不生成 DiffCard。确认后系统重新核验目标并直接执行。新建文档时必须同时提供 initialContent；系统确认后先创建真实文件，再把初始内容作为普通内容 Diff 投影。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "operations": {
                            "type": "array",
                            "minItems": 1,
                            "items": {
                                "type": "object",
                                "properties": {
                                    "kind": { "type": "string", "enum": ["create_document", "create_folder", "rename_resource", "move_resource", "delete_resource"] },
                                    "sourcePath": { "type": "string", "description": "删除/重命名/移动的源资源相对路径" },
                                    "targetPath": { "type": "string", "description": "创建/重命名/移动的目标资源相对路径" },
                                    "resourceKind": { "type": "string", "enum": ["file", "folder"] },
                                    "recursive": { "type": "boolean", "description": "删除目录时是否递归" }
                                },
                                "required": ["kind", "resourceKind"],
                                "additionalProperties": false
                            }
                        },
                        "initialContent": { "type": "string", "description": "仅 create_document：确认前生成并冻结的新文档完整初始内容" }
                    },
                    "required": ["operations"],
                    "additionalProperties": false
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "import_image_asset",
                "description": "把 workspace 内尚未位于受控 imageRoots 的 PNG/JPEG 复制到项目图片导入目录，返回 assetId/sourcePath/sourceHash。源文件保持不变。需要插入普通项目图片但 list_images 未暴露该文件时调用，随后重新调用 list_images 并用 image_ops 提案。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "source_path": { "type": "string", "description": "待复制图片的 workspace 相对路径；不得传绝对或越界路径" }
                    },
                    "required": ["source_path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "edit_blocked",
                "description": "编辑请求确实无法形成安全修改建议时，结构化报告阻断原因。仅用于缺少资产、锚点或动态能力等真实阻断；调用后可诚实结束回合，不得伪称文档已修改。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "code": { "type": "string", "description": "稳定、简短的阻断代码" },
                        "reason": { "type": "string", "description": "面向用户的具体阻断原因与下一步" }
                    },
                    "required": ["code", "reason"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "propose_decision",
                "description": "当且仅当任务无法在当前执行片段继续、且确实需要用户作权限授予、方案选择、范围确认或人工确认时，提出任务级确认卡。成功后当前执行片段立即结束并等待用户 confirm/cancel；它不修改文档、Diff 或文件树。不要用它替代普通编辑 DiffCard，也不要在调用成功后继续调用其他工具。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "kind": {
                            "type": "string",
                            "enum": ["permission", "plan_choice", "scope", "manual_confirm"],
                            "description": "需要用户裁决的类别"
                        },
                        "summary": {
                            "type": "string",
                            "description": "面向用户的简短裁决标题或问题"
                        },
                        "detail": {
                            "type": "string",
                            "description": "可选：作出裁决所需的具体上下文"
                        },
                        "options": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "可选：用户可选方案；未提供时使用确认/取消"
                        },
                        "targetPaths": {
                            "type": "array",
                            "minItems": 1,
                            "items": { "type": "string" },
                            "description": "仅 kind=scope：冻结的 active workspace 既有文档相对路径清单；read/search 结果不会自动获得修改授权"
                        }
                    },
                    "required": ["kind", "summary"],
                    "additionalProperties": false
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "read_doc",
                "description": "读取当前工作区内指定相对路径文档的内容。省略 blocks 时返回完整内容；传入 blocks（块 id 或块索引数组）时只返回指定块的完整投影（含 node_json，供 propose_edit block_ops 从 node_json 复制节点构造 afterNode）。注意：当前编辑器打开的文档已在上下文提供，无需用本工具重复读取。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "文档相对路径（相对 workspace 根）" },
                        "blocks": { "type": "array", "items": { "type": "string" }, "description": "可选：需要读取的块 id 或块索引数组（如 [\"b_xxx\", \"0\"]），只返回这些块的完整投影" }
                    },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "search_text",
                "description": "在当前工作区内按关键词搜索文档内容，返回命中的文件与片段。b-docx 会按 DOC 编辑模型转 HTML 后搜索块文本。",
                "parameters": {
                    "type": "object",
                    "properties": { "query": { "type": "string", "description": "搜索关键词" } },
                    "required": ["query"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_blocks",
                "description": "读取指定文档的块目录（轻量投影，不含 node_json/完整文本）。返回 block_id/block_index/node_type/role/parent_block_id/depth/text(前40字)/fingerprint/capabilities；需要某块完整投影（node_json）时调用 read_doc 并传入 blocks 参数。当前富文档直接返回 ProseMirror 层级投影；只有 capabilities 含 replace_node 的 content 块可用于 block_ops，容器与原子块只读。",
                "parameters": {
                    "type": "object",
                    "properties": { "path": { "type": "string", "description": "文档相对路径（相对 workspace 根）" } },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_tables",
                "description": "读取当前活动 BDocx 编辑器会话的表格结构：tableId、行列、单元格跨度、文本、backgroundColor、fingerprint、sessionVersion 与可执行能力。修改表格结构或单元格底纹前必须调用；不可用时应请用户打开目标 BDocx。",
                "parameters": {
                    "type": "object",
                    "properties": { "path": { "type": "string", "description": "当前活动 BDocx 的相对路径" } },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_images",
                "description": "读取当前活动 BDocx 的图片实例、相邻文本位置摘要与 workspace 内可插入的 PNG/JPEG 项目图片。返回同次 sessionVersion/contentHash、imageId/fingerprint、动态 capabilities/resizeBounds/moveDirections、稳定 Block、项目相对 sourcePath/sourceHash。这里只提供元数据和文档语境，不提供图片像素或视觉识别结果；不得把语境推断表述成看懂了图片。插入、删除、缩放、上下移动或设置独立图片段落对齐前必须调用；不得构造远程、绝对或 workspace 外来源。",
                "parameters": {
                    "type": "object",
                    "properties": { "path": { "type": "string", "description": "当前活动 BDocx 的相对路径" } },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "propose_edit",
                "description": "对文档提出待用户裁决的修改建议，不直接写盘。工具成功只表示 pending 建议卡已生成，不表示编辑器预览已应用、用户已接受或文件已保存；没有客户端状态回执时，回复只能说「已提出建议，待确认」。纯文本局部编辑使用 replace_block。当前 BDocx 的整篇重写只有运行时声明确认无表格且无图片时才可用 replace_document + 完整 HTML；否则 replace_document 会被后端拒绝，既有文字内容用 block_ops，表格/图片分别用 table_ops/image_ops。block_ops 只能命中 list_blocks 返回 capabilities=replace_node 的内容块，并带回稳定 blockId/fingerprint/expectedNodeType 和受控 afterNode；容器/原子块不可通用替换，相关多块须同批提交。可用 insert_before/insert_after/append 插入一个新的 heading 块；replace_node 必须保留既有块类型，不能把既有段落转换为标题。只改样式使用 style_ops。表格先 list_tables 再用 table_ops；图片先 list_images 再用 image_ops，支持受控插入、删除、缩放、上下移动、独立段落对齐及 resize/layout/move。不得用原始 HTML 绕过富文档受保护结构。",
                "parameters": propose_edit_parameters
            }
        }
    ]))
}

/// 只把逐回合授权的工具暴露给模型。注册表可以包含未来能力，但 1.0 模型不可枚举未授权项。
pub fn tool_definitions_json_for_grant(
    grant: &CapabilityGrantV2,
) -> Result<serde_json::Value, super::bdocx_capabilities::CapabilityValidationError> {
    let all = tool_definitions_json()?;
    let filtered = all
        .as_array()
        .into_iter()
        .flatten()
        .filter(|definition| {
            definition["function"]["name"]
                .as_str()
                .map(|name| grant.allows(name))
                .unwrap_or(false)
        })
        .cloned()
        .collect::<Vec<_>>();
    Ok(serde_json::Value::Array(filtered))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_workspace_root() -> &'static std::path::Path {
        std::path::Path::new("/tmp/binder-capability-grant-tests")
    }

    #[test]
    fn tools_registered_with_expected_permissions() {
        assert_eq!(TOOLS.len(), 13);
        assert!(is_registered("read_doc"));
        assert!(is_registered("list_docs"));
        assert!(is_registered("search_text"));
        assert!(is_registered("list_blocks"));
        assert!(is_registered("list_tables"));
        assert!(is_registered("list_images"));
        assert!(is_registered("workspace_glob"));
        assert!(is_registered("report_project"));
        assert!(!is_registered("move_doc"));
        assert!(!is_registered("create_folder"));
        assert!(is_registered("import_image_asset"));
        assert!(is_registered("edit_blocked"));
        assert!(is_registered("propose_edit"));
        assert!(is_registered("propose_decision"));
        assert!(is_registered("plan_workspace_operation"));
        assert!(!is_registered("delete_file"));
        assert_eq!(
            lookup("read_doc").unwrap().permission,
            ToolPermission::ReadOnly
        );
        assert_eq!(
            lookup("search_text").unwrap().permission,
            ToolPermission::ReadOnly
        );
        assert_eq!(
            lookup("list_blocks").unwrap().permission,
            ToolPermission::ReadOnly
        );
        assert_eq!(
            lookup("report_project").unwrap().permission,
            ToolPermission::ReadOnly
        );
        assert_eq!(
            lookup("plan_workspace_operation").unwrap().permission,
            ToolPermission::TaskControl
        );
        assert_eq!(
            lookup("propose_edit").unwrap().permission,
            ToolPermission::WriteIntent
        );
        assert_eq!(
            lookup("propose_decision").unwrap().permission,
            ToolPermission::TaskControl
        );
    }

    fn prompt_grant(
        turn_id: &str,
        targets: &[String],
        factors: CapabilityFactorsV2,
    ) -> CapabilityGrantV2 {
        resolve_capability_grant_v2(
            AGENT_EDIT_1_0_PROFILE,
            turn_id,
            targets,
            CapabilityFactorsV2 {
                remaining_tool_calls: 6,
                ..factors
            },
            CapabilityGrantPurposeV2::PromptManifest,
        )
    }

    fn execution_grant(
        turn_id: &str,
        tool_call_id: &str,
        seq: u64,
        targets: &[String],
        factors: CapabilityFactorsV2,
    ) -> CapabilityGrantV2 {
        resolve_capability_grant_v2(
            AGENT_EDIT_1_0_PROFILE,
            turn_id,
            targets,
            CapabilityFactorsV2 {
                gate_attempt: 1,
                gate_passed: true,
                remaining_tool_calls: 6,
                ..factors
            },
            CapabilityGrantPurposeV2::ToolCall {
                tool_call_id: tool_call_id.to_string(),
                computed_at_seq: seq,
            },
        )
    }

    #[test]
    fn prompt_manifest_is_hard_fact_projection_and_never_execution_authority() {
        let without_target = prompt_grant("turn-1", &[], CapabilityFactorsV2::default());
        let with_target = prompt_grant(
            "turn-1",
            &["a.md".to_string()],
            CapabilityFactorsV2::default(),
        );
        assert!(without_target.is_prompt_manifest());
        assert!(without_target.allows("read_doc"));
        assert!(!without_target.allows("propose_edit"));
        assert!(with_target.allows("propose_edit"));
        assert_ne!(
            without_target.explicit_edit_targets_hash,
            with_target.explicit_edit_targets_hash
        );
        assert_ne!(without_target.grant_id, with_target.grant_id);
        assert!(!with_target
            .permitted_effect_paths
            .iter()
            .any(|path| path.contains("execute_operation")));

        let mut tracker = CapabilityGrantUseTrackerV2::default();
        assert!(matches!(
            validate_requested_tool_for_grant(
                "read_doc",
                &serde_json::json!({"path":"a.md"}),
                &with_target,
                "turn-1",
                "call-1",
                1,
                Some(test_workspace_root()),
                &mut tracker,
            ),
            ToolDecision::Reject(_)
        ));
    }

    #[test]
    fn execution_grant_binds_turn_call_seq_gate_budget_and_is_single_use() {
        let grant = execution_grant(
            "turn-1",
            "call-1",
            7,
            &["a.md".to_string()],
            CapabilityFactorsV2::default(),
        );
        let mut tracker = CapabilityGrantUseTrackerV2::default();
        assert_eq!(
            validate_requested_tool_for_grant(
                "read_doc",
                &serde_json::json!({"path":"a.md"}),
                &grant,
                "turn-1",
                "call-1",
                7,
                Some(test_workspace_root()),
                &mut tracker,
            ),
            ToolDecision::Allow
        );
        assert!(matches!(
            validate_requested_tool_for_grant(
                "read_doc",
                &serde_json::json!({"path":"a.md"}),
                &grant,
                "turn-1",
                "call-1",
                7,
                Some(test_workspace_root()),
                &mut tracker,
            ),
            ToolDecision::Reject(reason) if reason.contains("已使用")
        ));

        for (turn_id, call_id, seq) in [
            ("other-turn", "call-1", 7),
            ("turn-1", "other-call", 7),
            ("turn-1", "call-1", 8),
        ] {
            assert!(matches!(
                validate_requested_tool_for_grant(
                    "read_doc",
                    &serde_json::json!({"path":"a.md"}),
                    &grant,
                    turn_id,
                    call_id,
                    seq,
                    Some(test_workspace_root()),
                    &mut CapabilityGrantUseTrackerV2::default(),
                ),
                ToolDecision::Reject(_)
            ));
        }

        for factors in [
            CapabilityFactorsV2 {
                gate_passed: false,
                remaining_tool_calls: 6,
                ..Default::default()
            },
            CapabilityFactorsV2 {
                gate_passed: true,
                remaining_tool_calls: 0,
                ..Default::default()
            },
            CapabilityFactorsV2 {
                gate_passed: true,
                remaining_tool_calls: 6,
                cancelled: true,
                ..Default::default()
            },
            CapabilityFactorsV2 {
                gate_passed: true,
                remaining_tool_calls: 6,
                terminal: true,
                ..Default::default()
            },
        ] {
            let denied = resolve_capability_grant_v2(
                AGENT_EDIT_1_0_PROFILE,
                "turn-1",
                &["a.md".to_string()],
                factors,
                CapabilityGrantPurposeV2::ToolCall {
                    tool_call_id: "call-denied".to_string(),
                    computed_at_seq: 9,
                },
            );
            assert!(denied.allowed_tools.is_empty());
        }
    }

    #[test]
    fn registered_unknown_parameter_and_target_boundaries_fail_closed() {
        let grant = execution_grant(
            "turn-2",
            "call-plan",
            10,
            &[],
            CapabilityFactorsV2::default(),
        );
        let invalid_plan = serde_json::json!({
            "operations":[{"kind":"create_document","targetPath":"meeting.md","resourceKind":"file"}]
        });
        assert!(matches!(
            validate_requested_tool_for_grant(
                "plan_workspace_operation",
                &invalid_plan,
                &grant,
                "turn-2",
                "call-plan",
                10,
                Some(test_workspace_root()),
                &mut CapabilityGrantUseTrackerV2::default(),
            ),
            ToolDecision::Reject(_)
        ));

        let unknown = execution_grant(
            "turn-2",
            "call-unknown",
            11,
            &[],
            CapabilityFactorsV2::default(),
        );
        assert!(matches!(
            validate_requested_tool_for_grant(
                "execute_workspace_operation",
                &serde_json::json!({}),
                &unknown,
                "turn-2",
                "call-unknown",
                11,
                Some(test_workspace_root()),
                &mut CapabilityGrantUseTrackerV2::default(),
            ),
            ToolDecision::Reject(_)
        ));
        assert!(!is_registered("execute_workspace_operation"));
    }

    #[test]
    fn target_and_workspace_paths_are_normalized_before_tool_executing() {
        let target_grant = execution_grant(
            "turn-target",
            "call-target-ok",
            20,
            &["docs/a.md".to_string()],
            CapabilityFactorsV2::default(),
        );
        assert_eq!(
            validate_requested_tool_for_grant(
                "propose_edit",
                &serde_json::json!({"path":"docs/./a.md","operation":"replace_block"}),
                &target_grant,
                "turn-target",
                "call-target-ok",
                20,
                Some(test_workspace_root()),
                &mut CapabilityGrantUseTrackerV2::default(),
            ),
            ToolDecision::Allow
        );

        let outside_target = execution_grant(
            "turn-target",
            "call-target-outside",
            21,
            &["docs/a.md".to_string()],
            CapabilityFactorsV2::default(),
        );
        assert!(matches!(
            validate_requested_tool_for_grant(
                "propose_edit",
                &serde_json::json!({"path":"docs/b.md","operation":"replace_block"}),
                &outside_target,
                "turn-target",
                "call-target-outside",
                21,
                Some(test_workspace_root()),
                &mut CapabilityGrantUseTrackerV2::default(),
            ),
            ToolDecision::Reject(reason) if reason.contains("ExplicitEditTargetSet")
        ));

        let escaped_read = execution_grant(
            "turn-target",
            "call-read-escape",
            22,
            &[],
            CapabilityFactorsV2::default(),
        );
        assert!(matches!(
            validate_requested_tool_for_grant(
                "read_doc",
                &serde_json::json!({"path":"../outside.md"}),
                &escaped_read,
                "turn-target",
                "call-read-escape",
                22,
                Some(test_workspace_root()),
                &mut CapabilityGrantUseTrackerV2::default(),
            ),
            ToolDecision::Reject(reason) if reason.contains("workspace")
        ));

        let escaped_plan = execution_grant(
            "turn-target",
            "call-plan-escape",
            23,
            &[],
            CapabilityFactorsV2::default(),
        );
        assert!(matches!(
            validate_requested_tool_for_grant(
                "plan_workspace_operation",
                &serde_json::json!({
                    "operations":[{
                        "kind":"create_document",
                        "targetPath":"../outside.md",
                        "resourceKind":"file"
                    }],
                    "initialContent":"body"
                }),
                &escaped_plan,
                "turn-target",
                "call-plan-escape",
                23,
                Some(test_workspace_root()),
                &mut CapabilityGrantUseTrackerV2::default(),
            ),
            ToolDecision::Reject(reason) if reason.contains("workspace")
        ));
    }

    #[test]
    fn malformed_registered_tool_parameters_fail_at_the_grant_boundary() {
        for (seq, call_id, tool, params) in [
            (
                24,
                "call-glob-invalid",
                "workspace_glob",
                serde_json::json!({}),
            ),
            (
                25,
                "call-decision-invalid",
                "propose_decision",
                serde_json::json!({"kind":"scope","summary":"","targetPaths":[]}),
            ),
            (
                26,
                "call-edit-invalid",
                "propose_edit",
                serde_json::json!({"path":"docs/a.md","operation":"direct_write"}),
            ),
        ] {
            let targets = if tool == "propose_edit" {
                vec!["docs/a.md".to_string()]
            } else {
                Vec::new()
            };
            let grant = execution_grant(
                "turn-invalid-params",
                call_id,
                seq,
                &targets,
                CapabilityFactorsV2::default(),
            );
            assert!(matches!(
                validate_requested_tool_for_grant(
                    tool,
                    &params,
                    &grant,
                    "turn-invalid-params",
                    call_id,
                    seq,
                    Some(test_workspace_root()),
                    &mut CapabilityGrantUseTrackerV2::default(),
                ),
                ToolDecision::Reject(_)
            ));
        }
    }

    #[test]
    fn confirmed_workspace_effect_uses_runtime_only_grant_and_exact_identity() {
        let grant = resolve_capability_grant_v2(
            AGENT_EDIT_1_0_PROFILE,
            "turn-effect",
            &[],
            CapabilityFactorsV2 {
                has_task_run: true,
                decision_receipt_ids: vec!["decision-1".to_string()],
                ..Default::default()
            },
            CapabilityGrantPurposeV2::RuntimeWorkspaceEffect {
                plan_id: "plan-1".to_string(),
                decision_request_id: "decision-1".to_string(),
                computed_at_seq: 30,
            },
        );
        assert!(grant.allowed_tools.is_empty());
        assert!(!grant.is_prompt_manifest());
        assert_eq!(grant.grant_kind, "runtime_effect");
        assert_eq!(
            validate_workspace_operation_effect_for_grant(
                &grant,
                "turn-effect",
                "plan-1",
                "decision-1",
                30,
            ),
            ToolDecision::Allow
        );
        assert!(matches!(
            validate_workspace_operation_effect_for_grant(
                &grant,
                "turn-effect",
                "plan-1",
                "decision-other",
                30,
            ),
            ToolDecision::Reject(_)
        ));

        let no_receipt = resolve_capability_grant_v2(
            AGENT_EDIT_1_0_PROFILE,
            "turn-effect",
            &[],
            CapabilityFactorsV2::default(),
            CapabilityGrantPurposeV2::RuntimeWorkspaceEffect {
                plan_id: "plan-1".to_string(),
                decision_request_id: "decision-1".to_string(),
                computed_at_seq: 31,
            },
        );
        assert!(!no_receipt
            .permitted_effect_paths
            .iter()
            .any(|effect| effect == "workspace.execute_operation"));
    }

    #[test]
    fn format_and_dynamic_facts_narrow_advanced_bdocx_tools() {
        let targets = vec!["a.docx".to_string()];
        let plain = prompt_grant("turn-format", &targets, CapabilityFactorsV2::default());
        assert!(!plain.allows("list_tables"));
        assert!(!plain.allows("list_images"));
        assert!(!plain.allows("import_image_asset"));

        let table = prompt_grant(
            "turn-format",
            &targets,
            CapabilityFactorsV2 {
                active_bdocx: true,
                table_ops_available: true,
                ..Default::default()
            },
        );
        assert!(table.allows("list_tables"));
        assert!(!table.allows("list_images"));

        let image = execution_grant(
            "turn-format",
            "call-image",
            12,
            &targets,
            CapabilityFactorsV2 {
                active_bdocx: true,
                image_ops_available: true,
                ..Default::default()
            },
        );
        assert!(image.allows("list_images"));
        assert!(image.allows("import_image_asset"));
        assert!(matches!(
            validate_requested_tool_for_grant(
                "propose_edit",
                &serde_json::json!({"path":"a.docx","operation":"image_ops"}),
                &image,
                "turn-format",
                "call-image",
                12,
                Some(test_workspace_root()),
                &mut CapabilityGrantUseTrackerV2::default(),
            ),
            ToolDecision::Allow
        ));
        assert_ne!(plain.format_capability_hash, image.format_capability_hash);
        assert_ne!(plain.grant_id, image.grant_id);
    }

    #[test]
    fn decision_and_task_facts_change_grant_without_authorizing_ws_execute() {
        let base = prompt_grant("turn-decision", &[], CapabilityFactorsV2::default());
        let decided = prompt_grant(
            "turn-decision",
            &[],
            CapabilityFactorsV2 {
                has_task_run: true,
                decision_receipt_ids: vec!["decision-1".to_string()],
                ..Default::default()
            },
        );
        assert_ne!(
            base.dynamic_capability_hash,
            decided.dynamic_capability_hash
        );
        assert_ne!(base.grant_id, decided.grant_id);
        assert_eq!(decided.decision_receipt_ids, vec!["decision-1"]);
        assert!(decided.allows("plan_workspace_operation"));
        assert!(!decided.allows("execute_workspace_operation"));
        assert!(!decided
            .permitted_effect_paths
            .iter()
            .any(|path| path.contains("execute_operation")));
    }

    #[test]
    fn propose_edit_schema_exposes_style_ops() {
        let defs = tool_definitions_json().unwrap();
        let propose = defs
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["function"]["name"] == "propose_edit")
            .unwrap();
        assert!(propose["function"]["parameters"]["properties"]["operation"].is_object());
        assert!(propose["function"]["parameters"]["required"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("operation")));
        assert!(propose["function"]["parameters"]["properties"]["style_ops"].is_object());
        assert!(propose["function"]["description"]
            .as_str()
            .unwrap()
            .contains("style_ops"));
        let schema = propose["function"]["parameters"].to_string();
        assert!(schema.contains("set_cell_background"));
        assert!(schema.contains("insert_image"));
        assert!(schema.contains("subscript"));
        assert!(schema.contains("allOf"));
        assert!(schema.contains("oneOf"));
        let description = propose["function"]["description"].as_str().unwrap();
        assert!(description.contains("pending 建议卡已生成"));
        assert!(description.contains("不表示编辑器预览已应用"));
        assert!(description.contains("无表格且无图片"));
        assert!(description.contains("replace_document 会被后端拒绝"));
        assert!(description.contains("插入一个新的 heading 块"));
        assert!(description.contains("不能把既有段落转换为标题"));
        assert!(!description.contains("草稿可先应用到 LogicalState"));
        assert!(description.contains("缩放"));
        assert!(description.contains("上下移动"));
        assert!(description.contains("独立段落对齐"));
        assert!(description.contains("resize/layout/move"));
        let list_images = defs
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["function"]["name"] == "list_images")
            .unwrap();
        let list_images_description = list_images["function"]["description"].as_str().unwrap();
        assert!(list_images_description.contains("resizeBounds"));
        assert!(list_images_description.contains("moveDirections"));
        assert!(list_images_description.contains("视觉识别"));
    }

    #[test]
    fn propose_decision_schema_is_task_control_not_document_write() {
        let defs = tool_definitions_json().unwrap();
        let decision = defs
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["function"]["name"] == "propose_decision")
            .unwrap();
        let parameters = &decision["function"]["parameters"];
        assert_eq!(
            parameters["required"],
            serde_json::json!(["kind", "summary"])
        );
        assert!(parameters["properties"]["kind"]["enum"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("manual_confirm")));
        assert!(decision["function"]["description"]
            .as_str()
            .unwrap()
            .contains("不修改文档、Diff 或文件树"));
        assert_eq!(
            parameters["properties"]["targetPaths"]["minItems"],
            serde_json::json!(1)
        );
        assert!(validate_scope_decision_parameters(&serde_json::json!({
            "kind": "scope",
            "summary": "允许修改",
            "targetPaths": ["a.md"]
        }))
        .is_ok());
        assert!(validate_scope_decision_parameters(&serde_json::json!({
            "kind": "scope",
            "summary": "允许修改"
        }))
        .is_err());
        assert!(validate_scope_decision_parameters(&serde_json::json!({
            "kind": "manual_confirm",
            "summary": "确认",
            "targetPaths": ["a.md"]
        }))
        .is_err());
    }

    #[test]
    fn workspace_operation_schema_replaces_direct_structure_tools() {
        let defs = tool_definitions_json().unwrap();
        let arr = defs.as_array().unwrap();
        let operation = arr
            .iter()
            .find(|d| d["function"]["name"] == "plan_workspace_operation")
            .unwrap();
        assert!(operation["function"]["parameters"]["properties"]["operations"].is_object());
        assert!(operation["function"]["parameters"]["properties"]["initialContent"].is_object());
        assert!(!arr.iter().any(|d| d["function"]["name"] == "move_doc"));
        assert!(!arr.iter().any(|d| d["function"]["name"] == "create_folder"));
        assert!(!arr.iter().any(|d| d["function"]["name"] == "create_doc"));
        let import_image = arr
            .iter()
            .find(|d| d["function"]["name"] == "import_image_asset")
            .unwrap();
        assert!(import_image["function"]["parameters"]["properties"]["source_path"].is_object());
        let edit_blocked = arr
            .iter()
            .find(|d| d["function"]["name"] == "edit_blocked")
            .unwrap();
        assert!(edit_blocked["function"]["parameters"]["properties"]["reason"].is_object());
        assert!(!arr.iter().any(|d| d["function"]["name"] == "delete_doc"));
    }
}
