//! 模型可见语义控制封套解析。封套只承载当前意图、候选路线与下一动作，不承载授权或隐藏推理。

use super::context::{
    AmbiguityLevelV1, IntentResultV2, IntentTargetCandidateV2, IntentTypeV2, SideEffectIntentV2,
};
use crate::ipc::IpcError;

pub const CONTROL_OPEN: &str = "<binder-turn-control>";
pub const CONTROL_CLOSE: &str = "</binder-turn-control>";
const CONTROL_MAX_CHARS: usize = 16_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelNextAction {
    Tool,
    Clarify,
    Final,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelIntentProjection {
    pub goal: String,
    pub intent_type: IntentTypeV2,
    #[serde(default)]
    pub target_candidates: Vec<IntentTargetCandidateV2>,
    pub side_effect_intent: SideEffectIntentV2,
    pub ambiguity: AmbiguityLevelV1,
    #[serde(default)]
    pub required_context: Vec<String>,
    #[serde(default)]
    pub confirmation_points: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelRouteObservation {
    pub summary: String,
    #[serde(default)]
    pub steps: Vec<String>,
}

/// 模型只能把结论关联到运行时生成的 evidence_id；是否成立由 Finalizing 校验。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelFinalFindingV2 {
    pub statement: String,
    #[serde(default)]
    pub evidence_ids: Vec<String>,
    pub uncertainty: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelTurnControl {
    pub version: String,
    pub next_action: ModelNextAction,
    pub intent: ModelIntentProjection,
    pub route: ModelRouteObservation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clarification_question: Option<String>,
    #[serde(default)]
    pub findings: Vec<ModelFinalFindingV2>,
    /// A stage outcome, never an authorization or proof of a resource effect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion: Option<ModelCompletion>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelCompletion {
    pub status: String,
    pub reason: String,
}

/// Fixed diagnostic vocabulary only; never retain provider values or serde's raw error text.
#[derive(Debug)]
pub(super) struct ControlProtocolRejection {
    pub error: IpcError,
    reason_code: &'static str,
    field_path: String,
    control_source: Option<String>,
    tool_index: Option<usize>,
}

impl ControlProtocolRejection {
    fn invalid(reason: &'static str, path: &str, message: &str) -> Self {
        Self {
            error: IpcError::new(
                "AG_MODEL_CONTROL_INVALID",
                &format!("[{reason}] {message}（字段：{path}）"),
                true,
                false,
            ),
            reason_code: reason,
            field_path: path.into(),
            control_source: None,
            tool_index: None,
        }
    }
    pub(super) fn diagnostic(&self) -> serde_json::Value {
        serde_json::json!({"reasonCode":self.reason_code,"fieldPath":self.field_path,
            "controlSource":self.control_source,"toolIndex":self.tool_index})
    }
    fn at(mut self, source: &str, index: usize) -> Self {
        self.control_source = Some(source.into());
        self.tool_index = Some(index);
        self
    }
}

impl ModelTurnControl {
    pub fn validate(&self) -> Result<(), IpcError> {
        self.validate_detailed()
            .map_err(|rejection| rejection.error)
    }

    fn validate_detailed(&self) -> Result<(), ControlProtocolRejection> {
        let invalid = ControlProtocolRejection::invalid;
        if self.version != "binder-turn-control-v1" {
            return Err(invalid(
                "version_unsupported",
                "version",
                "模型控制封套版本不受支持",
            ));
        }
        if let Some(completion) = &self.completion {
            if self.next_action != ModelNextAction::Final
                || !matches!(
                    completion.status.as_str(),
                    "report_only" | "blocked" | "no_op" | "partial"
                )
                || completion.reason.trim().is_empty()
                || completion.reason.chars().count() > 2000
            {
                return Err(invalid("completion_invalid", "completion", "completion仅final可选，status须为report_only/blocked/no_op/partial并有非空reason；不能以此制造等待或执行证据"));
            }
        }
        if self.intent.intent_type == IntentTypeV2::Unresolved {
            return Err(invalid(
                "intent_unresolved",
                "intent.intentType",
                "模型输出不得把 unresolved 作为已解析意图",
            ));
        }
        if self.intent.goal.trim().is_empty() || self.route.summary.trim().is_empty() {
            return Err(invalid(
                "description_empty",
                if self.intent.goal.trim().is_empty() {
                    "intent.goal"
                } else {
                    "route.summary"
                },
                "模型控制封套缺少 goal 或 route.summary",
            ));
        }
        if self.intent.target_candidates.iter().any(|candidate| {
            !matches!(
                candidate.kind.as_str(),
                "current_document"
                    | "selection"
                    | "explicit_document"
                    | "new_document"
                    | "workspace"
            ) || candidate
                .rel_path
                .as_deref()
                .is_some_and(|path| path.trim().is_empty())
        }) {
            return Err(invalid(
                "target_candidate_invalid",
                "intent.targetCandidates",
                "模型控制封套包含无效 target candidate",
            ));
        }
        if let Some(field) = [
            (&self.intent.required_context, "intent.requiredContext"),
            (
                &self.intent.confirmation_points,
                "intent.confirmationPoints",
            ),
            (&self.route.steps, "route.steps"),
        ]
        .into_iter()
        .find_map(|(items, path)| {
            items
                .iter()
                .any(|item| item.trim().is_empty())
                .then_some(path)
        }) {
            return Err(invalid(
                "control_array_item_empty",
                field,
                "模型控制封套数组不得包含空项",
            ));
        }
        if self.findings.iter().any(|finding| {
            finding.statement.trim().is_empty()
                || !matches!(
                    finding.uncertainty.as_str(),
                    "none" | "low" | "medium" | "high"
                )
                || finding
                    .evidence_ids
                    .iter()
                    .any(|evidence_id| evidence_id.trim().is_empty())
        }) {
            return Err(invalid(
                "finding_invalid",
                "findings",
                "模型 findings 包含空结论、空证据 ID 或无效 uncertainty",
            ));
        }
        let blocking = self.intent.ambiguity == AmbiguityLevelV1::Blocking;
        match self.next_action {
            ModelNextAction::Clarify => {
                if !blocking
                    || !self
                        .clarification_question
                        .as_deref()
                        .is_some_and(|question| !question.trim().is_empty())
                {
                    return Err(invalid(
                        "clarification_contract_invalid",
                        "clarificationQuestion",
                        "clarify 必须对应 blocking ambiguity 和非空问题",
                    ));
                }
                if !self.findings.is_empty() {
                    return Err(invalid(
                        "clarification_has_findings",
                        "findings",
                        "clarify 不得携带 final findings",
                    ));
                }
            }
            ModelNextAction::Tool | ModelNextAction::Final => {
                if blocking || self.clarification_question.is_some() {
                    return Err(invalid(
                        "action_ambiguity_conflict",
                        "intent.ambiguity",
                        "blocking ambiguity 只能进入 clarify",
                    ));
                }
                if self.next_action == ModelNextAction::Tool && !self.findings.is_empty() {
                    return Err(invalid(
                        "tool_has_findings",
                        "findings",
                        "tool 动作不得携带 final findings",
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn intent_result(&self, revision: u64, source_event_seq: u64) -> IntentResultV2 {
        IntentResultV2 {
            revision,
            source_event_seq,
            goal: self.intent.goal.clone(),
            intent_type: self.intent.intent_type,
            target_candidates: self.intent.target_candidates.clone(),
            side_effect_intent: self.intent.side_effect_intent,
            ambiguity: self.intent.ambiguity,
            required_context: self.intent.required_context.clone(),
            confirmation_points: self.intent.confirmation_points.clone(),
        }
    }
}

/// 仅在封套完成解析后才释放正文分片，防止内部控制 JSON 投影到用户消息。
#[derive(Debug, Default, Clone)]
pub struct ModelOutputDecoder {
    pending: String,
    control: Option<ModelTurnControl>,
    rejection: Option<IpcError>,
    plain_prefix: bool,
}

fn control_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "version": { "type": "string", "enum": ["binder-turn-control-v1"] },
            "nextAction": { "type": "string", "enum": ["tool"] },
            "intent": {
                "type": "object",
                "properties": {
                    "goal": { "type": "string" },
                    "intentType": { "type": "string", "enum": ["answer", "inspect", "search", "analyze", "edit", "create_document", "organize_workspace"] },
                    "targetCandidates": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "kind": { "type": "string", "enum": ["current_document", "selection", "explicit_document", "new_document", "workspace"] },
                                "relPath": { "type": "string" }
                            },
                            "required": ["kind"],
                            "additionalProperties": false
                        }
                    },
                    "sideEffectIntent": { "type": "string", "enum": ["none", "content_proposal", "structure_plan"] },
                    "ambiguity": { "type": "string", "enum": ["none", "non_blocking"] },
                    "requiredContext": { "type": "array", "items": { "type": "string" } },
                    "confirmationPoints": { "type": "array", "items": { "type": "string" } }
                },
                "required": ["goal", "intentType", "targetCandidates", "sideEffectIntent", "ambiguity", "requiredContext", "confirmationPoints"],
                "additionalProperties": false
            },
            "route": {
                "type": "object",
                "properties": {
                    "summary": { "type": "string" },
                    "steps": { "type": "array", "items": { "type": "string" } }
                },
                "required": ["summary", "steps"],
                "additionalProperties": false
            }
        },
        "required": ["version", "nextAction", "intent", "route"],
        "additionalProperties": false
    })
}

/// Every direct business call carries the response control at its arguments top level. A JSON
/// Schema describes one call and cannot express "first call of this response", so an optional
/// control would be accepted by the schema and then rejected by the runtime whenever the call
/// happened to be the response's only one. The runtime still keeps the first control as
/// canonical and tolerates an identical redundant copy on later calls.
/**
 * @GOV
 * codes:    BR-AG-DATA-004-DATA-AG-AG-TOOL-061
 * type:     DATA
 * chain:    AG-TOOL, AG-TURN
 * rules:    BR-AG-DATA-004
 * boundary: in=ToolDefinitionsAndReadOnlyPermissions | out=ModelVisibleToolSchemaWithResponseControl | does_not=executeToolsOrGrantCapabilities
 */
pub fn decorate_tool_definitions(tools: &mut serde_json::Value) -> Result<(), IpcError> {
    let definitions = tools.as_array_mut().ok_or_else(|| {
        IpcError::new(
            "AG_TOOL_MANIFEST_INVALID",
            "工具清单必须是数组",
            false,
            false,
        )
    })?;
    for definition in definitions {
        let read_only = definition
            .pointer("/function/name")
            .and_then(serde_json::Value::as_str)
            .and_then(super::tool_registry::lookup)
            .is_some_and(|tool| tool.permission == super::tool_registry::ToolPermission::ReadOnly);
        let parameters = definition
            .pointer_mut("/function/parameters")
            .and_then(serde_json::Value::as_object_mut)
            .ok_or_else(|| {
                IpcError::new(
                    "AG_TOOL_MANIFEST_INVALID",
                    "工具定义缺少 function.parameters",
                    false,
                    false,
                )
            })?;
        let properties = parameters
            .entry("properties")
            .or_insert_with(|| serde_json::json!({}))
            .as_object_mut()
            .ok_or_else(|| {
                IpcError::new(
                    "AG_TOOL_MANIFEST_INVALID",
                    "工具 parameters.properties 必须是对象",
                    false,
                    false,
                )
            })?;
        let mut response_control_schema = control_schema();
        response_control_schema["description"] = serde_json::Value::String(if read_only {
            "响应级语义控制：每次只读工具调用的 arguments 顶层都必须提供完整 _binderTurn；同一响应内各调用语义一致，运行时保留首个为主控制。不得嵌套进 params、calls 或任何传输封套。".to_string()
        } else {
            "响应级语义控制：有副作用工具须单独提交，并在 arguments 顶层提供完整 _binderTurn；不得嵌套进 params、calls 或任何传输封套。".to_string()
        });
        properties.insert("_binderTurn".to_string(), response_control_schema);
        let required = parameters
            .entry("required")
            .or_insert_with(|| serde_json::json!([]))
            .as_array_mut()
            .ok_or_else(|| {
                IpcError::new(
                    "AG_TOOL_MANIFEST_INVALID",
                    "工具 parameters.required 必须是数组",
                    false,
                    false,
                )
            })?;
        // JSON Schema describes one call, not its position in a response-level group. The
        // model-visible contract must therefore be position-independent: every business call
        // carries the control, and the runtime keeps the first one as canonical. Keeping the
        // field optional for read-only tools is what let a lone read-only call be accepted by
        // the schema and rejected by the runtime, so it is no longer optional.
        if !required.iter().any(|field| field == "_binderTurn") {
            required.push(serde_json::Value::String("_binderTurn".to_string()));
        }
    }
    Ok(())
}

pub fn take_tool_control(params: &mut serde_json::Value) -> Result<ModelTurnControl, IpcError> {
    take_tool_control_detailed(params).map_err(|rejection| rejection.error)
}

pub const TOOL_BATCH_NAME: &str = "binder_tool_batch";
pub const DIRECT_TOOL_PROTOCOL: &str = "native-business-tool-v1";
pub const BATCH_TOOL_PROTOCOL: &str = "binder-tool-batch-v1";
pub const LEGACY_TOOL_PROTOCOL: &str = "legacy-tool-control-v1";

/// Check the frozen model-facing transport before expanding or executing any business call.
/// Historical batch and legacy turns retain their original wire contracts.
pub(super) fn validate_transport_call_group(
    protocol: &str,
    calls: &[crate::ent::gateway::GwToolCall],
) -> Result<(), ControlProtocolRejection> {
    let has_batch = calls.iter().any(|call| call.name == TOOL_BATCH_NAME);
    if protocol == BATCH_TOOL_PROTOCOL {
        if !has_batch {
            return Err(ControlProtocolRejection::invalid(
                "transport_protocol_mismatch",
                "$batch",
                "本原回合冻结为批次工具协议，不能改用独立工具",
            ));
        }
    } else if has_batch {
        return Err(ControlProtocolRejection::invalid(
            "transport_protocol_mismatch",
            "$batch",
            "本原回合冻结为独立业务工具协议，不能提交批次工具",
        ));
    }
    if protocol == DIRECT_TOOL_PROTOCOL
        && calls.len() > 1
        && calls.iter().any(|call| {
            !super::tool_registry::lookup(&call.name).is_some_and(|tool| {
                tool.permission == super::tool_registry::ToolPermission::ReadOnly
            })
        })
    {
        return Err(ControlProtocolRejection::invalid(
            "direct_dependency_unsupported",
            "tool_calls",
            "同一响应的多个独立工具只能是只读调用；计划、提案或裁决须单独提交",
        ));
    }
    Ok(())
}

/// One provider call carries one response control and a typed batch from the same registry.
pub(super) fn batch_tool_definitions(definitions: &serde_json::Value) -> serde_json::Value {
    let variants: Vec<_> = definitions.as_array().into_iter().flatten().map(|definition| {
        serde_json::json!({"type":"object","description":definition["function"]["description"],"properties":{
            "toolName":{"type":"string","enum":[definition["function"]["name"]]},
            "params":definition["function"]["parameters"]
        },"required":["toolName","params"],"additionalProperties":false})
    }).collect();
    if variants.is_empty() {
        return serde_json::json!([]);
    }
    serde_json::json!([{"type":"function","function":{
        "name":TOOL_BATCH_NAME,
        "description":"Transport envelope only, not an extra capability: submit ONE control and business calls limited to CapabilityGrant.allowedTools. Multiple calls must be independent read-only; proposals/decisions/resource plans alone. No embedded _binderTurn in params.",
        "parameters":{"type":"object","properties":{
            "control":control_schema(),"calls":{"type":"array","minItems":1,"maxItems":16,"items":{"oneOf":variants}}
        },"required":["control","calls"],"additionalProperties":false}
    }}])
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeBatch {
    control: ModelTurnControl,
    calls: Vec<NativeBusinessCall>,
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NativeBusinessCall {
    tool_name: String,
    params: serde_json::Value,
}

// Diagnostic inspection only. Paths come exclusively from this fixed schema, never from
// provider keys or values. The original typed deserializer remains the acceptance boundary.
fn batch_schema_issue(
    value: &serde_json::Value,
    schema: &serde_json::Value,
    path: &str,
) -> Option<(&'static str, String)> {
    let matches_type = |kind: &str| match kind {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "null" => value.is_null(),
        _ => false,
    };
    let correct_type = match &schema["type"] {
        serde_json::Value::String(kind) => matches_type(kind),
        serde_json::Value::Array(kinds) => kinds
            .iter()
            .filter_map(|kind| kind.as_str())
            .any(matches_type),
        _ => true,
    };
    if !correct_type {
        return Some(("schema_type_invalid", path.into()));
    }
    if let Some(options) = schema["enum"].as_array() {
        if !options.contains(value) {
            return Some(("schema_enum_invalid", path.into()));
        }
    }
    if let Some(object) = value.as_object() {
        let properties = schema["properties"].as_object();
        let child_path = |key: &str| {
            if path == "$batch" {
                key.into()
            } else {
                format!("{path}.{key}")
            }
        };
        for required in schema["required"].as_array().into_iter().flatten() {
            if let Some(key) = required.as_str() {
                if !object.contains_key(key) {
                    return Some(("schema_missing_field", child_path(key)));
                }
            }
        }
        if schema["additionalProperties"] == false
            && object
                .keys()
                .any(|key| !properties.is_some_and(|p| p.contains_key(key)))
        {
            return Some(("schema_unknown_field", path.into()));
        }
        for (key, child_schema) in properties.into_iter().flatten() {
            if let Some(child) = object.get(key) {
                if let Some(issue) = batch_schema_issue(child, child_schema, &child_path(key)) {
                    return Some(issue);
                }
            }
        }
    }
    if let Some(array) = value.as_array() {
        for child in array {
            if let Some(issue) = batch_schema_issue(child, &schema["items"], &format!("{path}[]")) {
                return Some(issue);
            }
        }
    }
    None
}

fn native_batch_rejection(argument: &str, error: &serde_json::Error) -> ControlProtocolRejection {
    let (reason, path) = match serde_json::from_str::<serde_json::Value>(argument) {
        Err(json_error) => (
            if json_error.is_eof() {
                "batch_json_incomplete"
            } else {
                "batch_json_invalid"
            },
            "$batch".into(),
        ),
        Ok(value) => {
            let mut diagnostic_control = control_schema();
            // Typed controls also accept these known optional fields. Do not blame an
            // accepted empty findings/default field for a different deserialization error.
            diagnostic_control["properties"]["findings"] = serde_json::json!({"type":"array","items":{"type":"object","properties":{
                "statement":{"type":"string"},"evidenceIds":{"type":"array","items":{"type":"string"}},"uncertainty":{"type":"string"}
            },"required":["statement","uncertainty"],"additionalProperties":false}});
            diagnostic_control["properties"]["clarificationQuestion"] =
                serde_json::json!({"type":["string","null"]});
            diagnostic_control["properties"]["completion"] = serde_json::json!({"type":["object","null"],"properties":{"status":{"type":"string"},"reason":{"type":"string"}},"required":["status","reason"],"additionalProperties":false});
            let schema = serde_json::json!({"type":"object","properties":{
                "control":diagnostic_control,"calls":{"type":"array","items":{
                    "type":"object","properties":{"toolName":{"type":"string"},"params":{"type":"object"}},
                    "required":["toolName","params"],"additionalProperties":false
                }}},"required":["control","calls"],"additionalProperties":false});
            batch_schema_issue(&value, &schema, "$batch").unwrap_or_else(|| {
                // Never return serde's raw message: unknown fields/variants may contain secrets.
                let reason = if error.to_string().starts_with("duplicate field") {
                    "schema_duplicate_field"
                } else {
                    "schema_invalid"
                };
                (reason, "$batch".into())
            })
        }
    };
    ControlProtocolRejection::invalid(
        reason,
        &path,
        "批次参数不符合当前schema；请修正该位置，原值不回显",
    )
    .at("tool[0]", 0)
}

pub(super) fn expand_native_batch(
    calls: &[crate::ent::gateway::GwToolCall],
) -> Result<Option<Vec<crate::ent::gateway::GwToolCall>>, ControlProtocolRejection> {
    if !calls.iter().any(|call| call.name == TOOL_BATCH_NAME) {
        return Ok(None);
    }
    let invalid = |reason, path, message| {
        ControlProtocolRejection::invalid(reason, path, message).at("tool[0]", 0)
    };
    if calls.len() != 1 || calls[0].name != TOOL_BATCH_NAME || calls[0].id.trim().is_empty() {
        return Err(invalid(
            "batch_transport_conflict",
            "$batch",
            "一次响应只能提交一个批次",
        ));
    }
    let batch: NativeBatch = serde_json::from_str(&calls[0].arguments)
        .map_err(|error| native_batch_rejection(&calls[0].arguments, &error))?;
    batch.control.validate_detailed()?;
    if batch.control.next_action != ModelNextAction::Tool
        || batch.calls.is_empty()
        || batch.calls.len() > 16
    {
        return Err(invalid(
            "batch_action_invalid",
            "calls",
            "批次须为tool动作且包含1至16个调用",
        ));
    }
    let mut expanded = Vec::with_capacity(batch.calls.len());
    for (index, call) in batch.calls.into_iter().enumerate() {
        let Some(object) = call.params.as_object() else {
            return Err(invalid(
                "arguments_not_object",
                "params",
                "业务参数必须是对象",
            ));
        };
        if object.contains_key("_binderTurn") || call.tool_name == TOOL_BATCH_NAME {
            return Err(invalid(
                "batch_nested_control",
                "params",
                "业务参数不得重复控制或嵌套批次",
            ));
        }
        expanded.push(crate::ent::gateway::GwToolCall {
            id: format!("{}:{index}", calls[0].id),
            name: call.tool_name,
            arguments: call.params.to_string(),
        });
    }
    if expanded.len() > 1
        && expanded.iter().any(|call| {
            !super::tool_registry::lookup(&call.name).is_some_and(|tool| {
                tool.permission == super::tool_registry::ToolPermission::ReadOnly
            })
        })
    {
        return Err(invalid(
            "batch_dependency_unsupported",
            "calls",
            "非只读操作须单独提交，不能跨越确认或预览点",
        ));
    }
    let mut first: serde_json::Value =
        serde_json::from_str(&expanded[0].arguments).expect("serialized object");
    first["_binderTurn"] = serde_json::to_value(batch.control).expect("typed control");
    expanded[0].arguments = first.to_string();
    Ok(Some(expanded))
}

/// Internal virtual calls retain per-tool identity/evidence; provider history retains its one
/// native call and exactly one aggregated reply. This is a transport adapter, not an executor.
pub(super) fn project_native_batch_history(
    messages: &[serde_json::Value],
) -> Vec<(serde_json::Value, Option<String>)> {
    let mut projected = Vec::new();
    let mut pending: Option<(String, Vec<serde_json::Value>)> = None;
    for message in messages {
        if message["role"] == "tool" {
            if let Some((_, results)) = pending.as_mut() {
                results.push(serde_json::json!({"toolCallId":message["tool_call_id"],"content":message["content"]}));
                continue;
            }
        }
        if let Some((parent, results)) = pending.take() {
            projected.push((serde_json::json!({"role":"tool","tool_call_id":parent,"content":serde_json::json!({"results":results}).to_string()}),None));
        }
        let mut sent = message.clone();
        let layer = sent
            .as_object_mut()
            .and_then(|o| o.remove("layer"))
            .and_then(|v| v.as_str().map(str::to_owned));
        if let Some(batch) = sent.as_object_mut().and_then(|o| o.remove("nativeBatch")) {
            let id = batch["id"].as_str().unwrap_or("").to_string();
            sent["tool_calls"] = serde_json::json!([batch]);
            pending = Some((id, Vec::new()));
        }
        projected.push((sent, layer));
    }
    if let Some((parent, results)) = pending {
        projected.push((serde_json::json!({"role":"tool","tool_call_id":parent,"content":serde_json::json!({"results":results}).to_string()}),None));
    }
    projected
}

fn take_tool_control_detailed(
    params: &mut serde_json::Value,
) -> Result<ModelTurnControl, ControlProtocolRejection> {
    let control = params
        .as_object_mut()
        .and_then(|object| object.remove("_binderTurn"))
        .ok_or_else(|| {
            let mut rejection = ControlProtocolRejection::invalid(
                "control_missing",
                "_binderTurn",
                "ToolCall 缺少 _binderTurn 语义封套",
            );
            rejection.error = IpcError::new(
                "AG_MODEL_CONTROL_MISSING",
                "ToolCall 缺少 _binderTurn 语义封套",
                true,
                false,
            );
            rejection
        })?;
    let control: ModelTurnControl = serde_json::from_value(control).map_err(|error| {
        let text = error.to_string();
        let reason = if text.starts_with("missing field") {
            "schema_missing_field"
        } else if text.starts_with("unknown field") {
            "schema_unknown_field"
        } else if text.starts_with("unknown variant") {
            "schema_enum_invalid"
        } else if text.starts_with("invalid type") {
            "schema_type_invalid"
        } else {
            "schema_invalid"
        };
        // serde_json::from_value has no field path. Do not invent one or expose unknown keys/values.
        ControlProtocolRejection::invalid(
            reason,
            "$control",
            "ToolCall 的 _binderTurn 结构无效；请按工具schema提交，原值不回显",
        )
    })?;
    control.validate_detailed()?;
    if control.next_action != ModelNextAction::Tool {
        return Err(ControlProtocolRejection::invalid(
            "tool_action_required",
            "nextAction",
            "ToolCall 的 nextAction 必须为 tool",
        ));
    }
    Ok(control)
}

// Only field names leave this boundary. The compared values may include user paths or text and
// must never be copied into the diagnostic event or model-visible validation response.
fn changed_control_fields(left: &ModelTurnControl, right: &ModelTurnControl) -> Vec<&'static str> {
    let mut fields = Vec::new();
    if left.version != right.version {
        fields.push("version");
    }
    if left.next_action != right.next_action {
        fields.push("nextAction");
    }
    if left.intent.goal != right.intent.goal {
        fields.push("intent.goal");
    }
    if left.intent.intent_type != right.intent.intent_type {
        fields.push("intent.intentType");
    }
    if left.intent.target_candidates != right.intent.target_candidates {
        fields.push("intent.targetCandidates");
    }
    if left.intent.side_effect_intent != right.intent.side_effect_intent {
        fields.push("intent.sideEffectIntent");
    }
    if left.intent.ambiguity != right.intent.ambiguity {
        fields.push("intent.ambiguity");
    }
    if left.intent.required_context != right.intent.required_context {
        fields.push("intent.requiredContext");
    }
    if left.intent.confirmation_points != right.intent.confirmation_points {
        fields.push("intent.confirmationPoints");
    }
    if left.route.summary != right.route.summary {
        fields.push("route.summary");
    }
    if left.route.steps != right.route.steps {
        fields.push("route.steps");
    }
    if left.clarification_question != right.clarification_question {
        fields.push("clarificationQuestion");
    }
    if left.findings != right.findings {
        fields.push("findings");
    }
    fields
}

/** 一次响应只需要一份模型语义。主控制固定为文本或首个工具提供者。
 * 后续副本独立合法，只有描述字段可不同；实质冲突或损坏仍整批拒绝。
 */
pub fn parse_tool_batch(
    arguments: &[&str],
    output_control: Option<ModelTurnControl>,
) -> Result<(ModelTurnControl, Vec<serde_json::Value>), IpcError> {
    parse_tool_batch_detailed(arguments, output_control)
        .map(|batch| (batch.control, batch.params))
        .map_err(|rejection| rejection.error)
}

#[derive(Debug)]
pub(super) struct ParsedToolBatch {
    pub control: ModelTurnControl,
    pub params: Vec<serde_json::Value>,
    pub observations: Vec<serde_json::Value>,
}

pub(super) fn parse_tool_batch_detailed(
    arguments: &[&str],
    output_control: Option<ModelTurnControl>,
) -> Result<ParsedToolBatch, ControlProtocolRejection> {
    parse_tool_batch_detailed_with_policy(arguments, output_control, false)
}

/// Only a prevalidated group of independent read-only direct tools may treat further
/// complete controls as redundant observations. Tool arguments and the canonical
/// response control are never rewritten; effectful and legacy groups stay strict.
pub(super) fn parse_tool_batch_detailed_with_policy(
    arguments: &[&str],
    output_control: Option<ModelTurnControl>,
    independent_read_only_direct_group: bool,
) -> Result<ParsedToolBatch, ControlProtocolRejection> {
    let mut batch_control = output_control;
    let mut control_source = batch_control.as_ref().map(|_| "text".to_string());
    if let Some(control) = batch_control.as_ref() {
        control.validate_detailed().map_err(|mut rejection| {
            rejection.control_source = Some("text".into());
            rejection
        })?;
    }
    let mut parsed = Vec::with_capacity(arguments.len());
    let mut observations = Vec::new();
    for (index, argument) in arguments.iter().enumerate() {
        let source = format!("tool[{index}]");
        let arguments_invalid = || {
            let mut rejection = ControlProtocolRejection::invalid(
                "arguments_not_object",
                "$arguments",
                "工具参数必须是合法 JSON 对象",
            )
            .at(&source, index);
            rejection.error.code = "AG_TOOL_ARGUMENTS_INVALID".into();
            rejection
        };
        let mut params: serde_json::Value =
            serde_json::from_str(argument).map_err(|_| arguments_invalid())?;
        if !params.is_object() {
            return Err(arguments_invalid());
        }
        if params.get("_binderTurn").is_some() {
            let control = take_tool_control_detailed(&mut params)
                .map_err(|rejection| rejection.at(&source, index))?;
            if control.next_action != ModelNextAction::Tool {
                return Err(ControlProtocolRejection::invalid(
                    "tool_action_required",
                    "nextAction",
                    "工具响应的 nextAction 必须为 tool",
                )
                .at(&source, index));
            }
            if let Some(expected) = batch_control.as_ref() {
                let changed = changed_control_fields(expected, &control);
                let conflicts: Vec<_> = changed
                    .iter()
                    .copied()
                    .filter(|field| {
                        !matches!(*field, "intent.goal" | "route.summary" | "route.steps")
                    })
                    .collect();
                // Normalize only the three explicitly compatible descriptions, then compare
                // the whole object. Future fields stay strict even before diagnostics names them.
                let mut comparable = control.clone();
                comparable.intent.goal = expected.intent.goal.clone();
                comparable.route.summary = expected.route.summary.clone();
                comparable.route.steps = expected.route.steps.clone();
                if expected != &comparable && !independent_read_only_direct_group {
                    let path = conflicts.first().copied().unwrap_or("$control");
                    let fields = if conflicts.is_empty() {
                        "$control".into()
                    } else {
                        conflicts.join(",")
                    };
                    let mut rejection = ControlProtocolRejection::invalid(
                        "control_decision_conflict",
                        path,
                        "同一响应的动作或范围结论冲突",
                    )
                    .at(&source, index);
                    rejection.error = IpcError::new(
                        "AG_MODEL_CONTROL_CONFLICT",
                        &format!(
                            "同一响应的语义结论冲突（{} 与 tool[{index}]；差异字段：{fields}）",
                            control_source.as_deref().unwrap_or("unknown")
                        ),
                        true,
                        false,
                    );
                    return Err(rejection);
                }
                if !changed.is_empty() {
                    if independent_read_only_direct_group {
                        observations.push(serde_json::json!({"canonicalSource":control_source,"controlSource":source,"toolIndex":index,"controlFields":changed}));
                    } else {
                        observations.push(serde_json::json!({"canonicalSource":control_source,"controlSource":source,"toolIndex":index,"descriptionFields":changed}));
                    }
                }
            } else {
                batch_control = Some(control);
                control_source = Some(source);
            }
        }
        parsed.push(params);
    }
    let control = batch_control.ok_or_else(|| {
        let mut rejection = ControlProtocolRejection::invalid(
            "control_missing",
            "_binderTurn",
            "缺少响应级语义，请在工具 arguments 顶层提交控制封套",
        );
        rejection.error=IpcError::new(
        "AG_MODEL_CONTROL_MISSING",
        "本次请求未执行：工具 arguments 顶层缺少 _binderTurn。请在每次业务工具调用的 arguments 顶层补全完整 _binderTurn（version、nextAction=tool、intent、route），并保留原有业务参数；不要使用 calls/control 传输封套，也不要把它嵌套进 params。",
        true, false,
    );
        rejection
    })?;
    control.validate_detailed()?;
    if control.next_action != ModelNextAction::Tool {
        let mut rejection = ControlProtocolRejection::invalid(
            "tool_action_required",
            "nextAction",
            "工具响应的 nextAction 必须为 tool",
        );
        rejection.control_source = control_source;
        return Err(rejection);
    }
    Ok(ParsedToolBatch {
        control,
        params: parsed,
        observations,
    })
}

impl ModelOutputDecoder {
    pub(super) fn parsed_control(&self) -> Option<&ModelTurnControl> {
        self.control.as_ref()
    }
    pub fn push(&mut self, chunk: &str) -> Result<Vec<String>, IpcError> {
        if self.control.is_some() {
            return Ok((!chunk.is_empty())
                .then(|| chunk.to_string())
                .into_iter()
                .collect());
        }
        self.pending.push_str(chunk);
        let trimmed = self.pending.trim_start();
        // Native tool responses may have a harmless prose preamble. Quarantine it until the
        // complete response establishes whether a tool supplies the control. Never infer control
        // from prose or scan for a later tag and promote it to the response's authority.
        if self.plain_prefix || self.rejection.is_some() {
            return self.check_quarantine_limit();
        }
        if trimmed.len() < CONTROL_OPEN.len() && CONTROL_OPEN.starts_with(trimmed) {
            return self.check_quarantine_limit();
        }
        if !trimmed.starts_with(CONTROL_OPEN) {
            self.plain_prefix = true;
            return self.check_quarantine_limit();
        }
        let Some(close_offset) = trimmed.find(CONTROL_CLOSE) else {
            return self.check_quarantine_limit();
        };
        if trimmed[..close_offset].chars().count() > CONTROL_MAX_CHARS {
            return Err(IpcError::new(
                "AG_MODEL_CONTROL_TOO_LARGE",
                "模型控制封套超过大小上限",
                true,
                false,
            ));
        }
        let json_start = CONTROL_OPEN.len();
        let json = &trimmed[json_start..close_offset];
        let control = serde_json::from_str::<ModelTurnControl>(json)
            .map_err(|_| {
                IpcError::new(
                    "AG_MODEL_CONTROL_INVALID",
                    "模型控制封套不是合法结构",
                    true,
                    false,
                )
            })
            .and_then(|control| {
                control.validate()?;
                Ok(control)
            });
        let control = match control {
            Ok(control) => control,
            Err(error) => {
                self.rejection = Some(error);
                return self.check_quarantine_limit();
            }
        };
        let body_start = close_offset + CONTROL_CLOSE.len();
        let body = trimmed[body_start..].to_string();
        self.control = Some(control);
        self.pending.clear();
        Ok((!body.is_empty()).then_some(body).into_iter().collect())
    }

    /**
     * @GOV
     * codes:    BR-AG-DATA-004-GUARD-AG-AG-TURN-047, BR-AG-STATE-001-GUARD-AG-AG-TURN-047
     * type:     GUARD
     * chain:    AG-TURN
     * rules:    BR-AG-DATA-004, BR-AG-STATE-001
     * term_ref: TERM-AG-012
     * boundary: in=ProviderVisibleTextPrefix | out=ValidatedModelIntentRouteNextActionAndOptionalEvidenceReferencesWithHiddenControlRemoved | does_not=acceptAuthorizationHiddenReasoningUnknownFieldsUnstructuredFirstIntentOrModelDefinedEvidence | consumer=BR-AG-STATE-001
     */
    pub fn finish(self) -> Result<ModelTurnControl, IpcError> {
        if let Some(error) = self.rejection {
            return Err(error);
        }
        self.control.ok_or_else(|| {
            IpcError::new(
                if self.plain_prefix {
                    "AG_MODEL_CONTROL_MISSING"
                } else {
                    "AG_MODEL_CONTROL_INCOMPLETE"
                },
                "模型响应没有完整合法的结构化回合控制封套",
                true,
                false,
            )
        })
    }

    pub fn finish_optional(self) -> Result<Option<ModelTurnControl>, IpcError> {
        if let Some(error) = self.rejection {
            return Err(error);
        }
        if self.control.is_some() {
            return Ok(self.control);
        }
        if self.pending.trim().is_empty() {
            return Ok(None);
        }
        if self.plain_prefix
            && !self.pending.contains("<binder-turn-control")
            && !self.pending.contains(CONTROL_CLOSE)
        {
            return Ok(None);
        }
        Err(IpcError::new(
            "AG_MODEL_CONTROL_INCOMPLETE",
            "模型工具响应包含不完整的结构化回合控制封套",
            true,
            false,
        ))
    }

    fn check_quarantine_limit(&self) -> Result<Vec<String>, IpcError> {
        if self.pending.chars().count() > CONTROL_MAX_CHARS {
            return Err(IpcError::new(
                "AG_MODEL_CONTROL_TOO_LARGE",
                "未验证的模型响应超过隔离上限，已停止接收；本批未执行",
                true,
                false,
            ));
        }
        Ok(Vec::new())
    }

    pub fn prefix_kind(&self) -> &'static str {
        let prefix = self.pending.trim_start();
        if self.control.is_some() || prefix.starts_with(CONTROL_OPEN) {
            "control_tag"
        } else if prefix.is_empty() {
            "empty"
        } else if prefix.starts_with("```") {
            "markdown_fence"
        } else if prefix.starts_with('<') {
            "other_tag"
        } else {
            "plain_text"
        }
    }

    /// A fixed diagnostic for correction prompts, never an alternate control source. In
    /// particular, a complete JSON object followed by a non-Binder closing marker is still
    /// rejected; its action only helps explain why no native tool call was executed.
    pub(super) fn response_rejection_kind(&self) -> &'static str {
        let text_tool_action = self
            .control
            .as_ref()
            .is_some_and(|control| control.next_action == ModelNextAction::Tool)
            || self
                .pending
                .trim_start()
                .strip_prefix(CONTROL_OPEN)
                .and_then(|body| {
                    serde_json::Deserializer::from_str(body)
                        .into_iter::<ModelTurnControl>()
                        .next()
                        .and_then(Result::ok)
                })
                .is_some_and(|control| control.next_action == ModelNextAction::Tool);
        if text_tool_action {
            "text_tool_action"
        } else if self.rejection.is_some() {
            "control_invalid"
        } else if self.pending.trim_start().starts_with(CONTROL_OPEN) {
            "control_incomplete"
        } else {
            "control_missing"
        }
    }
}

pub fn protocol_prompt() -> &'static str {
    "[BINDER TURN OUTPUT PROTOCOL]\n\
每个不含原生 tool_calls 的响应（包括工具失败、受阻说明、结束或取消说明），以一个 <binder-turn-control>合法 JSON 对象</binder-turn-control> 开始，再输出用户正文；不能直接以Markdown标题或解释开头。所有属性名和字符串必须使用双引号；不能输出 JSON 占位符、单引号或枚举选项串。\n\
属性及枚举以工具 schema 和下列合法示例为准，不得把多个枚举值拼为字符串。findings 仅 final 可选，包含 statement、evidenceIds、uncertainty（none、low、medium、high 中一个）；不得自造 evidence ID。final/tool 不得携带 clarificationQuestion。\n\
运行时状态中的 revision/sourceEventSeq 由系统维护，不属于模型输出的 control.intent；control.intent 只填写工具 schema 列出的字段。调用 binder_tool_batch 时参数必须同时包含 control 和 calls，不能把 control 只写在正文封套中。\n\
澄清示例：<binder-turn-control>{\"version\":\"binder-turn-control-v1\",\"nextAction\":\"clarify\",\"intent\":{\"goal\":\"确定目标\",\"intentType\":\"organize_workspace\",\"targetCandidates\":[],\"sideEffectIntent\":\"structure_plan\",\"ambiguity\":\"blocking\",\"requiredContext\":[],\"confirmationPoints\":[]},\"route\":{\"summary\":\"等待目标补充\",\"steps\":[]},\"clarificationQuestion\":\"要处理哪份文件？\"}</binder-turn-control>要处理哪份文件？\n\
原生工具只调用binder_tool_batch一次，参数包含control和calls。它只是传输封套，不是新增业务能力；calls[].toolName必须属于本轮CapabilityGrant.allowedTools。control是本响应唯一语义（version=binder-turn-control-v1，nextAction=tool，intent和route按schema填写）；calls示例：[{\"toolName\":\"workspace_glob\",\"params\":{\"pattern\":\"*.txt\"}}]。业务params中不写_binderTurn。多个独立只读调用可以同批；计划、提案或裁决须单独一项，不跨等待点批量提交。\n\
需要工具时只能通过API原生tool_calls提交binder_tool_batch；绝不能把nextAction=tool的控制对象、函数调用JSON或供应商内部工具标记打印到content。文本中的工具意图不会执行；若本轮不能提交原生工具调用，只能用final或clarify完整封套如实说明尚未执行。\n\
最小问候示例：<binder-turn-control>{\"version\":\"binder-turn-control-v1\",\"nextAction\":\"final\",\"intent\":{\"goal\":\"回应问候\",\"intentType\":\"answer\",\"targetCandidates\":[],\"sideEffectIntent\":\"none\",\"ambiguity\":\"none\",\"requiredContext\":[],\"confirmationPoints\":[]},\"route\":{\"summary\":\"直接回应\",\"steps\":[]}}</binder-turn-control>你好，有什么需要我帮忙？\n这不是隐藏推理：只写简短结论和候选步骤，不写思维链。它不授权工具。\n\
若可直接回答，nextAction=final 并在封套后输出正文；若缺少必须信息，nextAction=clarify。需要工具时仅提交一个binder_tool_batch，不同时输出文本控制或多份控制。最终答复必须nextAction=final；阻断澄清必须nextAction=clarify且ambiguity=blocking。control和候选路线均不是授权，实际调用仍由运行时逐项核验。\n\
final可选阶段说明：completion={\"status\":\"report_only\",\"reason\":\"本次仅列候选清单，未创建文件\"}。status仅report_only/blocked/no_op/partial四选一且reason非空；不是执行凭证。待裁决必须调用实际工具，缺信息用clarify，不能用此字段假造等待。\n\
[/BINDER TURN OUTPUT PROTOCOL]"
}

/// New turns expose the authorized business tools directly. Tool semantics are still model
/// supplied, but no transport function or calls[] wrapper is model-visible.
pub fn direct_protocol_prompt() -> &'static str {
    "[BINDER TURN OUTPUT PROTOCOL — DIRECT BUSINESS TOOLS]\n\
无原生 tool_calls 的回复必须先输出一个 <binder-turn-control>合法 JSON 对象</binder-turn-control>，再输出正文。final/clarify 的控制仍包含 version、nextAction、intent、route；所有属性和字符串使用双引号，不输出占位符或 Markdown 围栏。\n\
需要工具时只调用本轮 API 原生工具清单中的业务函数（如 workspace_glob、read_doc、plan_workspace_operation），不要构造额外的传输函数、calls 数组或 control 包裹层。每次业务工具调用的 arguments 顶层都必须包含完整 _binderTurn，值为 version=binder-turn-control-v1、nextAction=tool、intent 和 route；同一响应内多个调用的 _binderTurn 必须语义一致，运行时保留首个为主控制，逐项授权只看真实参数和 CapabilityGrant。控制必须直接位于 arguments 顶层，不能嵌套进 params 或任何子对象；纠错重发、重试和后续新响应同样必须重新携带。有副作用工具必须单独提交且保持严格控制校验。多个独立只读工具可在同一响应提交，但不得跨依赖或等待点；提案、计划或裁决必须单独提交。_binderTurn 是当前响应的语义结论，不是授权。\n\
工具示例：调用原生 workspace_glob，arguments={\"pattern\":\"*.txt\",\"_binderTurn\":{\"version\":\"binder-turn-control-v1\",\"nextAction\":\"tool\",\"intent\":{\"goal\":\"定位 TXT 文件\",\"intentType\":\"inspect\",\"targetCandidates\":[],\"sideEffectIntent\":\"none\",\"ambiguity\":\"none\",\"requiredContext\":[],\"confirmationPoints\":[]},\"route\":{\"summary\":\"列出候选文件\",\"steps\":[]}}}。不要在 content 中打印工具 JSON、伪工具标记或 nextAction=tool 的文本封套；只有实际原生工具调用才会执行。\n\
澄清示例：<binder-turn-control>{\"version\":\"binder-turn-control-v1\",\"nextAction\":\"clarify\",\"intent\":{\"goal\":\"确定目标\",\"intentType\":\"organize_workspace\",\"targetCandidates\":[],\"sideEffectIntent\":\"structure_plan\",\"ambiguity\":\"blocking\",\"requiredContext\":[],\"confirmationPoints\":[]},\"route\":{\"summary\":\"等待目标补充\",\"steps\":[]},\"clarificationQuestion\":\"要处理哪份文件？\"}</binder-turn-control>要处理哪份文件？\n\
直接回答示例：<binder-turn-control>{\"version\":\"binder-turn-control-v1\",\"nextAction\":\"final\",\"intent\":{\"goal\":\"回应问候\",\"intentType\":\"answer\",\"targetCandidates\":[],\"sideEffectIntent\":\"none\",\"ambiguity\":\"none\",\"requiredContext\":[],\"confirmationPoints\":[]},\"route\":{\"summary\":\"直接回应\",\"steps\":[]}}</binder-turn-control>你好，有什么需要我帮忙？\n\
final 可选 completion={\"status\":\"report_only\",\"reason\":\"本轮仅列出清单，未创建文件\"}；它不是工具回执。findings 仅 final 可选，证据 ID 只能引用真实运行时证据。revision/sourceEventSeq 由系统维护，不能放入模型 intent。缺少必要信息用 clarify 且 ambiguity=blocking；实际执行、等待和效果只凭运行时回执，不由 control 授权或证明。\n\
[/BINDER TURN OUTPUT PROTOCOL]"
}

/// A structural warning, never proof of a transport error or semantic completeness.
/// In particular, a missing period or a short answer is not a failure condition.
pub(super) fn answer_structure_warning(body: &str) -> Option<&'static str> {
    let mut fence: Option<(char, usize)> = None;
    for line in body.lines() {
        let trimmed = line.trim_start();
        if line.len() - trimmed.len() > 3 {
            continue;
        }
        let first = trimmed.chars().next().unwrap_or(' ');
        if !matches!(first, '`' | '~') {
            continue;
        }
        let count = trimmed.chars().take_while(|c| *c == first).count();
        if count < 3 {
            continue;
        }
        match fence {
            None => fence = Some((first, count)),
            Some((mark, width))
                if mark == first && count >= width && trimmed[count..].trim().is_empty() =>
            {
                fence = None
            }
            _ => {}
        }
    }
    if fence.is_some() {
        return Some("unclosed_code_fence");
    }
    let last_paragraph = body.trim_end().rsplit("\n\n").next().unwrap_or("");
    let lines: Vec<_> = last_paragraph
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    if lines.len() == 1
        && lines[0].trim_start().starts_with('|')
        && lines[0].chars().filter(|c| *c == '|').count() >= 2
    {
        return Some("dangling_table_header");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: BR-AG-OBS-001, BR-AG-STATE-001
    #[test]
    fn incomplete_structure_is_a_warning_not_a_punctuation_gate() {
        assert_eq!(
            answer_structure_warning("说明\n\n| 层级 | 你实际拥有的 | 能说什"),
            Some("dangling_table_header")
        );
        assert_eq!(
            answer_structure_warning("```text\n中文🙂"),
            Some("unclosed_code_fence")
        );
        for complete in [
            "你好",
            "plain answer without punctuation",
            "```text\n中文🙂\n```",
            "| A | B |\n|---|---|\n| 1 | 2 |",
            "x | y",
            "",
        ] {
            assert_eq!(answer_structure_warning(complete), None, "{complete}");
        }
    }

    // covers: BR-AG-DATA-004, BR-AG-STATE-002, BR-AG-DATA-005
    #[test]
    fn response_batch_has_one_control_and_preserves_provider_reply_identity() {
        let mut decoder = ModelOutputDecoder::default();
        decoder.push(&control("tool", "none", None)).unwrap();
        let c = decoder.finish().unwrap();
        let native=crate::ent::gateway::GwToolCall {id:"parent".into(),name:TOOL_BATCH_NAME.into(),arguments:serde_json::json!({"control":c,"calls":[
            {"toolName":"list_docs","params":{}},{"toolName":"workspace_glob","params":{"pattern":"*.txt"}}
        ]}).to_string()};
        let calls = expand_native_batch(&[native.clone()]).unwrap().unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].id, "parent:0");
        assert_eq!(calls[1].id, "parent:1");
        assert!(!calls[1].arguments.contains("_binderTurn"));
        let history = vec![
            serde_json::json!({"role":"assistant","content":null,"tool_calls":calls.iter().map(|call|serde_json::json!({"id":call.id,"type":"function","function":{"name":call.name,"arguments":call.arguments}})).collect::<Vec<_>>(),
            "nativeBatch":{"id":native.id,"type":"function","function":{"name":native.name,"arguments":native.arguments}}}),
            serde_json::json!({"role":"tool","tool_call_id":"parent:0","content":"RESULT_A"}),
            serde_json::json!({"role":"tool","tool_call_id":"parent:1","content":"RESULT_B"}),
        ];
        let projected = super::super::context::provider_message_projection(&history);
        assert_eq!(projected.len(), 2);
        assert_eq!(projected[0]["tool_calls"].as_array().unwrap().len(), 1);
        assert_eq!(projected[1]["tool_call_id"], "parent");
        assert!(projected[1]["content"]
            .as_str()
            .unwrap()
            .contains("RESULT_A"));
        assert!(projected[1]["content"]
            .as_str()
            .unwrap()
            .contains("RESULT_B"));
        assert!(!projected[0].to_string().contains("nativeBatch"));
        let measurement =
            super::super::context::measure_prompt_budget(&history, &serde_json::json!([]));
        assert_eq!(
            measurement.serialized_chars,
            serde_json::to_string(&projected).unwrap().chars().count() + 2
        );
        for bad in [
            serde_json::json!({"control":c,"calls":[{"toolName":"list_docs","params":{"_binderTurn":c}}]}),
            serde_json::json!({"control":c,"calls":[{"toolName":"list_docs","params":{}},{"toolName":"plan_workspace_operation","params":{}}]}),
            serde_json::json!({"control":c,"calls":[]}),
            serde_json::json!({"control":c,"calls":[],"extra":"SECRET"}),
        ] {
            let rejected = expand_native_batch(&[crate::ent::gateway::GwToolCall {
                arguments: bad.to_string(),
                ..native.clone()
            }])
            .unwrap_err();
            assert!(!rejected.error.message.contains("SECRET"));
        }
        let mut bad_enum =
            serde_json::json!({"control":c,"calls":[{"toolName":"list_docs","params":{}}]});
        bad_enum["control"]["intent"]["intentType"] = serde_json::json!("PRIVATE_ENUM");
        let mut bad_array = serde_json::json!({"control":c,"calls":[]});
        bad_array["control"]["route"]["steps"] = serde_json::json!([5]);
        for (argument, reason, path) in [
            ("{".into(), "batch_json_incomplete", "$batch"),
            // A typed enum error before a truncated tail must not hide JSON EOF.
            ("{\"control\":{\"version\":\"PRIVATE_ENUM\"},\"calls\":[".into(), "batch_json_incomplete", "$batch"),
            ("{PRIVATE}".into(), "batch_json_invalid", "$batch"),
            (serde_json::json!({"control":c}).to_string(), "schema_missing_field", "calls"),
            (serde_json::json!({"control":c,"calls":[{"toolName":5,"params":{}}]}).to_string(), "schema_type_invalid", "calls[].toolName"),
            (serde_json::json!({"control":c,"calls":[{"toolName":"list_docs","params":{},"PRIVATE_KEY":"SECRET"}]}).to_string(), "schema_unknown_field", "calls[]"),
            (bad_enum.to_string(), "schema_enum_invalid", "control.intent.intentType"),
            (bad_array.to_string(), "schema_type_invalid", "control.route.steps[]"),
            (format!("{{\"control\":{c},\"control\":{c},\"calls\":[]}}",c=serde_json::to_string(&c).unwrap()), "schema_duplicate_field", "$batch"),
        ] {
            let rejected = expand_native_batch(&[crate::ent::gateway::GwToolCall { arguments:argument, ..native.clone() }]).unwrap_err();
            let diagnostic = rejected.diagnostic();
            assert_eq!(diagnostic["reasonCode"], reason);
            assert_eq!(diagnostic["fieldPath"], path);
            assert_eq!(diagnostic["controlSource"], "tool[0]");
            assert!(!diagnostic.to_string().contains("PRIVATE"));
            assert!(!rejected.error.message.contains("SECRET"));
        }
    }

    // covers: BR-AG-DATA-004, BR-AG-STATE-002
    #[test]
    fn direct_transport_accepts_independent_reads_and_rejects_mixed_or_effect_groups() {
        let direct = |name: &str| crate::ent::gateway::GwToolCall {
            id: format!("id-{name}"),
            name: name.into(),
            arguments: "{}".into(),
        };
        let reads = vec![direct("workspace_glob"), direct("list_docs")];
        assert!(validate_transport_call_group(DIRECT_TOOL_PROTOCOL, &reads).is_ok());
        assert_eq!(
            validate_transport_call_group(DIRECT_TOOL_PROTOCOL, &[direct(TOOL_BATCH_NAME)])
                .unwrap_err()
                .diagnostic()["reasonCode"],
            "transport_protocol_mismatch"
        );
        assert_eq!(
            validate_transport_call_group(BATCH_TOOL_PROTOCOL, &[direct("list_docs")])
                .unwrap_err()
                .diagnostic()["reasonCode"],
            "transport_protocol_mismatch"
        );
        let mixed = vec![direct("workspace_glob"), direct("plan_workspace_operation")];
        assert_eq!(
            validate_transport_call_group(DIRECT_TOOL_PROTOCOL, &mixed)
                .unwrap_err()
                .diagnostic()["reasonCode"],
            "direct_dependency_unsupported"
        );
        assert!(validate_transport_call_group(
            DIRECT_TOOL_PROTOCOL,
            &[direct("plan_workspace_operation")]
        )
        .is_ok());
        assert!(direct_protocol_prompt()
            .contains("每次业务工具调用的 arguments 顶层都必须包含完整 _binderTurn"));
        assert!(!direct_protocol_prompt().contains(TOOL_BATCH_NAME));
    }

    // covers: BR-AG-DATA-004
    // covers: BR-AG-STATE-001
    #[test]
    fn fixed_transport_runs_production_control_decoder_without_model_or_editor() {
        tauri::async_runtime::block_on(async {
            use crate::ent::gateway::{fixed_round_stream, RoundOutcome};
            for (text, expected_error) in [
                (format!("{}最终答复", control("final", "none", None)), None),
                (
                    "<binder-turn-control>{bad}</binder-turn-control>".into(),
                    Some("AG_MODEL_CONTROL_INVALID"),
                ),
                ("没有控制封套".into(), Some("AG_MODEL_CONTROL_MISSING")),
                (
                    "<binder-turn-control>{".into(),
                    Some("AG_MODEL_CONTROL_INCOMPLETE"),
                ),
            ] {
                let source = format!(
                    "event: chunk\ndata: {}\n\nevent: done\ndata: {{\"stop_reason\":\"stop\"}}\n\n",
                    serde_json::json!({"text": text}),
                );
                let stream = futures_util::stream::iter(vec![Ok::<_, String>(source.into_bytes())]);
                let mut decoder = ModelOutputDecoder::default();
                let mut visible = String::new();
                let outcome = fixed_round_stream(stream, |text| {
                    for fragment in decoder.push(&text)? {
                        visible.push_str(&fragment);
                    }
                    Ok(())
                })
                .await;
                let parsed = match outcome {
                    Ok(RoundOutcome::Done { .. }) => decoder.finish(),
                    Err(error) => Err(error),
                    _ => panic!("unexpected transport outcome"),
                };
                match expected_error {
                    Some(code) => {
                        assert_eq!(parsed.unwrap_err().code, code);
                        assert!(visible.is_empty());
                    }
                    None => {
                        assert_eq!(parsed.unwrap().next_action, ModelNextAction::Final);
                        assert_eq!(visible, "最终答复");
                    }
                }
            }
        });
    }

    // covers: BR-AG-DATA-004
    // covers: BR-AG-STATE-002
    #[test]
    fn fixed_transport_native_tools_run_real_argument_and_control_validation() {
        tauri::async_runtime::block_on(async {
            use crate::ent::gateway::{fixed_round_stream, RoundOutcome};
            let mut decoder = ModelOutputDecoder::default();
            decoder.push(&control("tool", "none", None)).unwrap();
            let semantic = decoder.finish().unwrap();
            for (arguments, expected_error) in [
                (
                    serde_json::json!({"pattern":"*.txt", "_binderTurn":semantic}).to_string(),
                    None,
                ),
                ("{bad".into(), Some("AG_TOOL_ARGUMENTS_INVALID")),
                ("{}".into(), Some("AG_MODEL_CONTROL_MISSING")),
            ] {
                let source = format!(
                    "event: tool_calls\ndata: {}\n\n",
                    serde_json::json!({
                        "tool_calls":[{"id":"call-fixed", "name":"workspace_glob", "arguments":arguments}]
                    })
                );
                let stream = futures_util::stream::iter(vec![Ok::<_, String>(source.into_bytes())]);
                let outcome = fixed_round_stream(stream, |_| panic!("no text expected"))
                    .await
                    .unwrap();
                let RoundOutcome::ToolCalls(calls) = outcome else {
                    panic!("expected native tools")
                };
                let arguments = calls
                    .iter()
                    .map(|call| call.arguments.as_str())
                    .collect::<Vec<_>>();
                let parsed = parse_tool_batch(&arguments, None);
                match expected_error {
                    Some(code) => assert_eq!(parsed.unwrap_err().code, code),
                    None => {
                        let (control, parameters) = parsed.unwrap();
                        assert_eq!(control.next_action, ModelNextAction::Tool);
                        assert_eq!(parameters, vec![serde_json::json!({"pattern":"*.txt"})]);
                    }
                }
            }
        });
    }

    fn control(action: &str, ambiguity: &str, question: Option<&str>) -> String {
        let question = question
            .map(|value| format!(",\"clarificationQuestion\":\"{value}\""))
            .unwrap_or_default();
        format!(
            "{CONTROL_OPEN}{{\"version\":\"binder-turn-control-v1\",\"nextAction\":\"{action}\",\"intent\":{{\"goal\":\"检查当前文档\",\"intentType\":\"inspect\",\"targetCandidates\":[{{\"kind\":\"current_document\",\"relPath\":\"a.md\"}}],\"sideEffectIntent\":\"none\",\"ambiguity\":\"{ambiguity}\",\"requiredContext\":[\"current_document_logical_state\"],\"confirmationPoints\":[]}},\"route\":{{\"summary\":\"先读取再回答\",\"steps\":[\"读取当前文档\"]}}{question}}}{CONTROL_CLOSE}"
        )
    }

    // covers: BR-AG-DATA-004, BR-AG-STATE-001
    #[test]
    fn split_control_is_hidden_and_visible_body_streams_after_validation() {
        let source = format!("{}最终答复", control("final", "none", None));
        let split = CONTROL_OPEN.len() + 20;
        let mut decoder = ModelOutputDecoder::default();
        assert!(decoder.push(&source[..split]).unwrap().is_empty());
        assert_eq!(decoder.push(&source[split..]).unwrap(), vec!["最终答复"]);
        let parsed = decoder.finish().unwrap();
        assert_eq!(parsed.next_action, ModelNextAction::Final);
        assert_eq!(parsed.intent.intent_type, IntentTypeV2::Inspect);

        let large_body = "正文".repeat(CONTROL_MAX_CHARS);
        let mut single_chunk = ModelOutputDecoder::default();
        assert_eq!(
            single_chunk
                .push(&format!("{}{}", control("final", "none", None), large_body))
                .unwrap(),
            vec![large_body]
        );
        assert!(single_chunk.finish().is_ok());
    }

    // covers: BR-AG-DATA-004, BR-SYS-STATE-007
    #[test]
    fn clarification_requires_blocking_ambiguity_and_question() {
        let mut valid = ModelOutputDecoder::default();
        valid
            .push(&control("clarify", "blocking", Some("要处理哪一份文档？")))
            .unwrap();
        assert_eq!(
            valid.finish().unwrap().next_action,
            ModelNextAction::Clarify
        );

        let mut invalid = ModelOutputDecoder::default();
        assert!(invalid
            .push(&control("clarify", "none", Some("要处理哪一份文档？")))
            .unwrap()
            .is_empty());
        let error = invalid.finish().unwrap_err();
        assert_eq!(error.code, "AG_MODEL_CONTROL_INVALID");
    }

    #[test]
    fn unstructured_or_unresolved_control_fails_closed() {
        let mut unstructured = ModelOutputDecoder::default();
        assert!(unstructured.push("直接回答").unwrap().is_empty());
        assert_eq!(
            unstructured.finish().unwrap_err().code,
            "AG_MODEL_CONTROL_MISSING"
        );
        let unresolved = control("final", "none", None).replace("\"inspect\"", "\"unresolved\"");
        let mut decoder = ModelOutputDecoder::default();
        assert!(decoder.push(&unresolved).unwrap().is_empty());
        assert_eq!(
            decoder.finish().unwrap_err().code,
            "AG_MODEL_CONTROL_INVALID"
        );
        for prefix in ["先检查文件。", "```text\n说明\n```", "<note>说明</note>"] {
            let mut decoder = ModelOutputDecoder::default();
            for fragment in prefix.chars() {
                assert!(decoder.push(&fragment.to_string()).unwrap().is_empty());
            }
            assert_eq!(
                decoder.clone().finish().unwrap_err().code,
                "AG_MODEL_CONTROL_MISSING"
            );
            assert!(decoder.finish_optional().unwrap().is_none());
        }
        for malformed in [
            "<binder-turn-control>{bad}</binder-turn-control>",
            "说明<制度><binder-turn-control>{}",
        ] {
            let mut decoder = ModelOutputDecoder::default();
            assert!(decoder.push(malformed).unwrap().is_empty());
            assert!(
                decoder.finish_optional().is_err(),
                "explicit malformed control is not a harmless preamble"
            );
        }
        let mut oversized = ModelOutputDecoder::default();
        assert_eq!(
            oversized
                .push(&"x".repeat(CONTROL_MAX_CHARS + 1))
                .unwrap_err()
                .code,
            "AG_MODEL_CONTROL_TOO_LARGE"
        );
        let mut whitespace = ModelOutputDecoder::default();
        assert_eq!(
            whitespace
                .push(&" ".repeat(CONTROL_MAX_CHARS + 1))
                .unwrap_err()
                .code,
            "AG_MODEL_CONTROL_TOO_LARGE"
        );
    }

    // covers: BR-AG-STATE-001, BR-AG-STATE-002
    #[test]
    fn text_tool_intent_with_wrong_closer_is_diagnostic_only() {
        let mut decoder = ModelOutputDecoder::default();
        let malformed =
            control("tool", "none", None).replace(CONTROL_CLOSE, "</｜｜DSML｜｜ parameter>");
        assert!(decoder.push(&malformed).unwrap().is_empty());
        assert_eq!(decoder.response_rejection_kind(), "text_tool_action");
        assert_eq!(
            decoder.finish().unwrap_err().code,
            "AG_MODEL_CONTROL_INCOMPLETE"
        );

        let mut plain = ModelOutputDecoder::default();
        plain
            .push("正文里提到 nextAction=tool 也不是工具调用")
            .unwrap();
        assert_eq!(plain.response_rejection_kind(), "control_missing");
        let mut final_control = ModelOutputDecoder::default();
        final_control
            .push(&control("final", "none", None).replace(CONTROL_CLOSE, "</wrong>"))
            .unwrap();
        assert_eq!(
            final_control.response_rejection_kind(),
            "control_incomplete"
        );
    }

    #[test]
    fn tool_manifest_allows_one_response_control_and_executor_strips_it_from_domain_params() {
        let mut tools = serde_json::json!([{
            "type": "function",
            "function": {
                "name": "read_doc",
                "parameters": {
                    "type": "object",
                    "properties": { "path": { "type": "string" } },
                    "required": ["path"],
                    "additionalProperties": false
                }
            }
        }]);
        decorate_tool_definitions(&mut tools).unwrap();
        // Read-only tools must also declare the control as required: the schema describes a
        // single call and cannot mark it "only the response's first call needs this".
        assert!(tools[0]["function"]["parameters"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "_binderTurn"));
        assert!(tools[0]["function"]["parameters"]["properties"]["_binderTurn"].is_object());
        let mut effectful = serde_json::json!([{
            "type":"function",
            "function":{"name":"plan_workspace_operation","parameters":{
                "type":"object","properties":{},"required":[],"additionalProperties":false
            }}
        }]);
        decorate_tool_definitions(&mut effectful).unwrap();
        assert!(effectful[0]["function"]["parameters"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "_binderTurn"));

        let mut decoder = ModelOutputDecoder::default();
        decoder.push(&control("tool", "none", None)).unwrap();
        let expected = decoder.finish().unwrap();
        let mut params = serde_json::json!({
            "path": "a.md",
            "_binderTurn": expected,
        });
        let parsed = take_tool_control(&mut params).unwrap();
        assert_eq!(parsed.next_action, ModelNextAction::Tool);
        assert_eq!(params, serde_json::json!({"path": "a.md"}));
    }

    // covers: BR-AG-DATA-004
    // covers: BR-AG-STATE-002
    #[test]
    fn response_control_is_shared_without_requiring_domain_argument_duplicates() {
        let mut decoder = ModelOutputDecoder::default();
        decoder.push(&control("tool", "none", None)).unwrap();
        let expected = decoder.finish().unwrap();
        let first = serde_json::json!({"path":"a.md","_binderTurn": &expected}).to_string();
        let second = r#"{"path":"b.md"}"#;
        let (parsed, params) = parse_tool_batch(&[&first, second], None).unwrap();
        assert_eq!(parsed, expected);
        assert_eq!(
            params,
            vec![
                serde_json::json!({"path":"a.md"}),
                serde_json::json!({"path":"b.md"})
            ]
        );
        let (_, params) = parse_tool_batch(&[second], Some(expected.clone())).unwrap();
        assert_eq!(params, vec![serde_json::json!({"path":"b.md"})]);

        // A lone read-only call without the control is exactly the shape that used to loop to
        // AG_MODEL_CORRECTION_LIMIT: the schema accepted it, the runtime rejected it. The
        // rejection must name the required position and forbid the old transport envelope.
        let missing = parse_tool_batch(&[second], None).unwrap_err();
        assert_eq!(missing.code, "AG_MODEL_CONTROL_MISSING");
        assert!(missing.message.contains("arguments 顶层"));
        assert!(missing.message.contains("calls/control"));
        let mut different = expected.clone();
        different.intent.goal = "private goal must not enter diagnostics".into();
        let accepted = parse_tool_batch_detailed(&[&first], Some(different.clone())).unwrap();
        assert_eq!(accepted.control, different); // 文本主控制不被工具副本替换。
        assert_eq!(accepted.observations[0]["canonicalSource"], "text");
        assert_eq!(
            accepted.observations[0]["descriptionFields"],
            serde_json::json!(["intent.goal"])
        );
        assert!(!serde_json::to_string(&accepted.observations)
            .unwrap()
            .contains("private goal"));
        let mut changed = expected.clone();
        changed.route.summary = "private route must not enter diagnostics".into();
        let conflicting_second =
            serde_json::json!({"path": "b.md", "_binderTurn": changed}).to_string();
        let accepted = parse_tool_batch_detailed(&[&first, &conflicting_second], None).unwrap();
        assert_eq!(accepted.control, expected); // 首工具主控制不被后续子路线覆盖。
        assert_eq!(accepted.params[1], serde_json::json!({"path":"b.md"}));
        assert_eq!(
            accepted.observations[0]["descriptionFields"],
            serde_json::json!(["route.summary"])
        );
        assert!(!serde_json::to_string(&accepted.observations)
            .unwrap()
            .contains("private route"));
        assert_eq!(
            parse_tool_batch(&["[]"], Some(expected.clone()))
                .unwrap_err()
                .code,
            "AG_TOOL_ARGUMENTS_INVALID"
        );
        let unsafe_control = first.replace(
            r#""nextAction":"tool""#,
            r#""allowed_tools":["propose_edit"],"nextAction":"tool""#,
        );
        assert_eq!(
            parse_tool_batch(&[&unsafe_control], None).unwrap_err().code,
            "AG_MODEL_CONTROL_INVALID"
        );
        let mut final_control = expected;
        final_control.next_action = ModelNextAction::Final;
        assert_eq!(
            parse_tool_batch(&[second], Some(final_control))
                .unwrap_err()
                .code,
            "AG_MODEL_CONTROL_INVALID"
        );
    }

    // covers: BR-AG-DATA-004, BR-AG-STATE-002
    #[test]
    fn independent_direct_reads_keep_one_control_without_accepting_invalid_copies() {
        let mut decoder = ModelOutputDecoder::default();
        decoder.push(&control("tool", "none", None)).unwrap();
        let canonical = decoder.finish().unwrap();
        let mut changed = serde_json::to_value(&canonical).unwrap();
        changed["intent"]["requiredContext"] = serde_json::json!(["target block"]);
        let first = serde_json::json!({"path":"a.md","_binderTurn":canonical}).to_string();
        let second = serde_json::json!({"path":"b.md","_binderTurn":changed}).to_string();
        let accepted =
            parse_tool_batch_detailed_with_policy(&[&first, &second], None, true).unwrap();
        assert_eq!(accepted.control, canonical);
        assert_eq!(accepted.params[0], serde_json::json!({"path":"a.md"}));
        assert_eq!(accepted.params[1], serde_json::json!({"path":"b.md"}));
        assert_eq!(
            accepted.observations[0]["controlFields"],
            serde_json::json!(["intent.requiredContext"])
        );
        assert_eq!(
            parse_tool_batch_detailed(&[&first, &second], None)
                .unwrap_err()
                .error
                .code,
            "AG_MODEL_CONTROL_CONFLICT"
        );

        let tool_only = serde_json::json!({"path":"b.md"}).to_string();
        assert!(parse_tool_batch_detailed_with_policy(&[&first, &tool_only], None, true).is_ok());
        assert_eq!(
            parse_tool_batch_detailed_with_policy(&[&tool_only], None, true)
                .unwrap_err()
                .error
                .code,
            "AG_MODEL_CONTROL_MISSING"
        );

        let mut stale_copy = serde_json::to_value(&canonical).unwrap();
        stale_copy["nextAction"] = serde_json::json!("final");
        let invalid = serde_json::json!({"path":"b.md","_binderTurn":stale_copy}).to_string();
        assert_eq!(
            parse_tool_batch_detailed_with_policy(&[&first, &invalid], None, true)
                .unwrap_err()
                .diagnostic()["reasonCode"],
            "tool_action_required"
        );
        let text_canonical =
            parse_tool_batch_detailed_with_policy(&[&second], Some(canonical.clone()), true)
                .unwrap();
        assert_eq!(text_canonical.control, canonical);
    }

    // covers: BR-AG-DATA-004, BR-AG-STATE-002
    #[test]
    fn duplicate_description_compatibility_never_accepts_changed_decision_fields() {
        let mut decoder = ModelOutputDecoder::default();
        decoder.push(&control("tool", "none", None)).unwrap();
        let canonical = decoder.finish().unwrap();
        let original = serde_json::to_value(&canonical).unwrap();
        for (path, value) in [
            ("/intent/intentType", serde_json::json!("search")),
            (
                "/intent/sideEffectIntent",
                serde_json::json!("structure_plan"),
            ),
            ("/intent/ambiguity", serde_json::json!("non_blocking")),
            (
                "/intent/targetCandidates",
                serde_json::json!([{"kind":"explicit_document","relPath":"PRIVATE_TARGET"}]),
            ),
            (
                "/intent/requiredContext",
                serde_json::json!(["PRIVATE_CONTEXT"]),
            ),
            (
                "/intent/confirmationPoints",
                serde_json::json!(["PRIVATE_CONFIRM"]),
            ),
        ] {
            let mut changed = original.clone();
            *changed.pointer_mut(path).unwrap() = value;
            let first = serde_json::json!({"path":"a.md","_binderTurn":original}).to_string();
            let second = serde_json::json!({"path":"b.md","_binderTurn":changed}).to_string();
            let rejection = parse_tool_batch_detailed(&[&first, &second], None).unwrap_err();
            assert_eq!(rejection.error.code, "AG_MODEL_CONTROL_CONFLICT");
            assert!(rejection.error.message.contains("tool[0] 与 tool[1]"));
            assert_eq!(rejection.diagnostic()["toolIndex"], 1);
            assert_eq!(
                rejection.diagnostic()["reasonCode"],
                "control_decision_conflict"
            );
            assert!(
                !format!("{}{}", rejection.error.message, rejection.diagnostic())
                    .contains("PRIVATE")
            );
        }
    }

    // covers: BR-AG-DATA-004, BR-AG-STATE-002
    #[test]
    fn invalid_duplicate_has_typed_location_without_provider_values() {
        let mut decoder = ModelOutputDecoder::default();
        decoder.push(&control("tool", "none", None)).unwrap();
        let canonical = serde_json::to_value(decoder.finish().unwrap()).unwrap();
        let first = serde_json::json!({"pattern":"*","_binderTurn":canonical}).to_string();
        for (path, value, reason, field) in [
            (
                "/nextAction",
                serde_json::json!("PRIVATE_ENUM"),
                "schema_enum_invalid",
                "$control",
            ),
            (
                "/intent/goal",
                serde_json::json!(123),
                "schema_type_invalid",
                "$control",
            ),
            (
                "/intent/goal",
                serde_json::json!(" "),
                "description_empty",
                "intent.goal",
            ),
            (
                "/intent/ambiguity",
                serde_json::json!("blocking"),
                "action_ambiguity_conflict",
                "intent.ambiguity",
            ),
            (
                "/nextAction",
                serde_json::json!("final"),
                "tool_action_required",
                "nextAction",
            ),
        ] {
            let mut changed = canonical.clone();
            *changed.pointer_mut(path).unwrap() = value;
            let second = serde_json::json!({"pattern":"*","_binderTurn":changed}).to_string();
            let rejection = parse_tool_batch_detailed(&[&first, &second], None).unwrap_err();
            assert_eq!(rejection.error.code, "AG_MODEL_CONTROL_INVALID");
            assert_eq!(rejection.diagnostic()["reasonCode"], reason);
            assert_eq!(rejection.diagnostic()["fieldPath"], field);
            assert_eq!(rejection.diagnostic()["controlSource"], "tool[1]");
            assert!(
                !format!("{}{}", rejection.error.message, rejection.diagnostic())
                    .contains("PRIVATE")
            );
        }
        let mut changed = canonical;
        changed["PRIVATE_UNKNOWN_KEY"] = serde_json::json!("PRIVATE_BODY");
        let rejected = take_tool_control_detailed(&mut serde_json::json!({"_binderTurn":changed}))
            .unwrap_err();
        assert_eq!(rejected.diagnostic()["reasonCode"], "schema_unknown_field");
        assert!(
            !format!("{}{}", rejected.error.message, rejected.diagnostic()).contains("PRIVATE")
        );
    }

    // covers: BR-AG-DATA-004
    #[test]
    fn prompt_greeting_example_is_valid_control_not_placeholder_json() {
        let prompt = protocol_prompt();
        let example = prompt
            .split("最小问候示例：")
            .nth(1)
            .unwrap()
            .split('\n')
            .next()
            .unwrap();
        let mut decoder = ModelOutputDecoder::default();
        assert!(!decoder.push(example).unwrap().is_empty());
        assert_eq!(
            decoder.finish().unwrap().next_action,
            ModelNextAction::Final
        );
        let clarification = prompt
            .split("澄清示例：")
            .nth(1)
            .unwrap()
            .split('\n')
            .next()
            .unwrap();
        let mut decoder = ModelOutputDecoder::default();
        decoder.push(clarification).unwrap();
        assert_eq!(
            decoder.finish().unwrap().next_action,
            ModelNextAction::Clarify
        );
        assert!(prompt.contains("原生工具只调用binder_tool_batch一次"));
        assert!(prompt.contains("revision/sourceEventSeq 由系统维护"));
        assert!(prompt.contains("参数必须同时包含 control 和 calls"));
        let mut decoder = ModelOutputDecoder::default();
        decoder.push(&control("tool", "none", None)).unwrap();
        let tool_control = decoder.finish().unwrap();
        let native = crate::ent::gateway::GwToolCall {id:"native-example".into(),name:TOOL_BATCH_NAME.into(),arguments:serde_json::json!({"control":tool_control,"calls":[{"toolName":"workspace_glob","params":{"pattern":"*.txt"}}]}).to_string()};
        let expanded = expand_native_batch(&[native]).unwrap().unwrap();
        assert_eq!(
            parse_tool_batch(&[&expanded[0].arguments], None)
                .unwrap()
                .0
                .next_action,
            ModelNextAction::Tool
        );
    }

    // covers: BR-AG-STATE-001, BR-AG-DATA-004
    #[test]
    fn stage_completion_is_not_authorization_waiting_or_tool_control() {
        let mut decoder = ModelOutputDecoder::default();
        decoder.push(&control("final", "none", None)).unwrap();
        let original = decoder.finish().unwrap();
        for status in ["report_only", "blocked", "no_op", "partial"] {
            let mut current = original.clone();
            current.completion = Some(ModelCompletion {
                status: status.into(),
                reason: "当前仅报告，未操作".into(),
            });
            current.validate().unwrap();
            current.next_action = ModelNextAction::Tool;
            assert!(current.validate().is_err());
            current.next_action = ModelNextAction::Final;
            current.completion.as_mut().unwrap().reason = " ".into();
            assert!(current.validate().is_err());
        }
        let mut current = original;
        current.completion = Some(ModelCompletion {
            status: "awaiting_confirm".into(),
            reason: "等待".into(),
        });
        assert!(current.validate().is_err());
    }

    #[test]
    fn later_model_semantics_can_revise_intent_and_route() {
        let mut decoder = ModelOutputDecoder::default();
        decoder.push(&control("tool", "none", None)).unwrap();
        let first = decoder.finish().unwrap();
        let first_intent = first.intent_result(1, 10);

        let mut later = first.clone();
        later.intent.goal = "根据检索结果修改目标文档".to_string();
        later.intent.intent_type = IntentTypeV2::Edit;
        later.intent.side_effect_intent = SideEffectIntentV2::ContentProposal;
        later.route.summary = "新事实表明需要先定位块再提出修改".to_string();
        later.route.steps = vec!["定位目标块".to_string(), "提出修改建议".to_string()];
        later.validate().unwrap();
        let later_intent = later.intent_result(first_intent.revision + 1, 20);

        assert_eq!(later_intent.revision, 2);
        assert_eq!(later_intent.source_event_seq, 20);
        assert_eq!(later_intent.intent_type, IntentTypeV2::Edit);
        assert_ne!(later.route, first.route);
    }
}
