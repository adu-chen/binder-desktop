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

/**
 * @GOV
 * codes:    BR-AG-DATA-004-GUARD-AG-AG-TURN-047, BR-AG-STATE-001-GUARD-AG-AG-TURN-047
 * type:     GUARD
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-004, BR-AG-STATE-001
 * term_ref: TERM-AG-012
 * boundary: in=ProviderVisibleTextPrefix | out=ValidatedModelIntentRouteNextActionAndOptionalEvidenceReferencesWithHiddenControlRemoved | does_not=acceptAuthorizationHiddenReasoningUnknownFieldsUnstructuredFirstIntentOrModelDefinedEvidence
 */
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
}

impl ModelTurnControl {
    pub fn validate(&self) -> Result<(), IpcError> {
        let invalid =
            |message: &str| IpcError::new("AG_MODEL_CONTROL_INVALID", message, true, false);
        if self.version != "binder-turn-control-v1" {
            return Err(invalid("模型控制封套版本不受支持"));
        }
        if self.intent.intent_type == IntentTypeV2::Unresolved {
            return Err(invalid("模型输出不得把 unresolved 作为已解析意图"));
        }
        if self.intent.goal.trim().is_empty() || self.route.summary.trim().is_empty() {
            return Err(invalid("模型控制封套缺少 goal 或 route.summary"));
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
            return Err(invalid("模型控制封套包含无效 target candidate"));
        }
        if self
            .intent
            .required_context
            .iter()
            .chain(self.intent.confirmation_points.iter())
            .chain(self.route.steps.iter())
            .any(|item| item.trim().is_empty())
        {
            return Err(invalid("模型控制封套数组不得包含空项"));
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
                    return Err(invalid("clarify 必须对应 blocking ambiguity 和非空问题"));
                }
                if !self.findings.is_empty() {
                    return Err(invalid("clarify 不得携带 final findings"));
                }
            }
            ModelNextAction::Tool | ModelNextAction::Final => {
                if blocking || self.clarification_question.is_some() {
                    return Err(invalid("blocking ambiguity 只能进入 clarify"));
                }
                if self.next_action == ModelNextAction::Tool && !self.findings.is_empty() {
                    return Err(invalid("tool 动作不得携带 final findings"));
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

/// 工具可承载一份响应级语义；不要求每个领域工具重复同一控制结构。
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
        properties.insert("_binderTurn".to_string(), control_schema());
    }
    Ok(())
}

pub fn take_tool_control(params: &mut serde_json::Value) -> Result<ModelTurnControl, IpcError> {
    let control = params
        .as_object_mut()
        .and_then(|object| object.remove("_binderTurn"))
        .ok_or_else(|| {
            IpcError::new(
                "AG_MODEL_CONTROL_MISSING",
                "ToolCall 缺少 _binderTurn 语义封套",
                true,
                false,
            )
        })?;
    let control: ModelTurnControl = serde_json::from_value(control).map_err(|error| {
        IpcError::new(
            "AG_MODEL_CONTROL_INVALID",
            &format!("ToolCall 的 _binderTurn 不是合法结构：{error}"),
            true,
            false,
        )
    })?;
    control.validate()?;
    if control.next_action != ModelNextAction::Tool {
        return Err(IpcError::new(
            "AG_MODEL_CONTROL_INVALID",
            "ToolCall 的 nextAction 必须为 tool",
            true,
            false,
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

/** 一次响应只需要一份模型语义，来自文本封套或任意一个工具。
 * 所有出现的副本必须相同；缺失、冲突或损坏返回校验失败，调用方回注而非执行。
 */
pub fn parse_tool_batch(
    arguments: &[&str],
    output_control: Option<ModelTurnControl>,
) -> Result<(ModelTurnControl, Vec<serde_json::Value>), IpcError> {
    let mut batch_control = output_control;
    let mut control_source = batch_control.as_ref().map(|_| "text".to_string());
    let mut parsed = Vec::with_capacity(arguments.len());
    for (index, argument) in arguments.iter().enumerate() {
        let mut params: serde_json::Value = serde_json::from_str(argument).map_err(|_| {
            IpcError::new(
                "AG_TOOL_ARGUMENTS_INVALID",
                "工具参数不是合法 JSON 对象",
                true,
                false,
            )
        })?;
        if !params.is_object() {
            return Err(IpcError::new(
                "AG_TOOL_ARGUMENTS_INVALID",
                "工具参数必须为对象",
                true,
                false,
            ));
        }
        if params.get("_binderTurn").is_some() {
            let control = take_tool_control(&mut params)?;
            if let Some(expected) = batch_control.as_ref() {
                if expected != &control {
                    let fields = changed_control_fields(expected, &control).join(",");
                    return Err(IpcError::new(
                        "AG_MODEL_CONTROL_CONFLICT",
                        &format!(
                            "同一响应的语义结论冲突（{} 与 tool[{index}]；差异字段：{fields}）",
                            control_source.as_deref().unwrap_or("unknown")
                        ),
                        true,
                        false,
                    ));
                }
            }
            batch_control = Some(control);
            control_source = Some(format!("tool[{index}]"));
        }
        parsed.push(params);
    }
    let control = batch_control.ok_or_else(|| IpcError::new(
        "AG_MODEL_CONTROL_MISSING",
        "本次请求未执行：缺少响应级语义。请在首个工具参数的 _binderTurn 提交一份完整语义结论，再提交需要的工具。",
        true, false,
    ))?;
    control.validate()?;
    if control.next_action != ModelNextAction::Tool {
        return Err(IpcError::new(
            "AG_MODEL_CONTROL_INVALID",
            "工具响应的 nextAction 必须为 tool",
            true,
            false,
        ));
    }
    Ok((control, parsed))
}

impl ModelOutputDecoder {
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
}

pub fn protocol_prompt() -> &'static str {
    "[BINDER TURN OUTPUT PROTOCOL]\n\
每个不含原生 tool_calls 的响应，以一个 <binder-turn-control>合法 JSON 对象</binder-turn-control> 开始。所有属性名和字符串必须使用双引号；不能输出 JSON 占位符、单引号或枚举选项串。\n\
属性及枚举以工具 schema 和下列合法示例为准，不得把多个枚举值拼为字符串。findings 仅 final 可选，包含 statement、evidenceIds、uncertainty（none、low、medium、high 中一个）；不得自造 evidence ID。final/tool 不得携带 clarificationQuestion。\n\
澄清示例：<binder-turn-control>{\"version\":\"binder-turn-control-v1\",\"nextAction\":\"clarify\",\"intent\":{\"goal\":\"确定目标\",\"intentType\":\"organize_workspace\",\"targetCandidates\":[],\"sideEffectIntent\":\"structure_plan\",\"ambiguity\":\"blocking\",\"requiredContext\":[],\"confirmationPoints\":[]},\"route\":{\"summary\":\"等待目标补充\",\"steps\":[]},\"clarificationQuestion\":\"要处理哪份文件？\"}</binder-turn-control>要处理哪份文件？\n\
原生 workspace_glob 的参数示例：{\"pattern\":\"*.txt\",\"_binderTurn\":{\"version\":\"binder-turn-control-v1\",\"nextAction\":\"tool\",\"intent\":{\"goal\":\"定位文本文件\",\"intentType\":\"inspect\",\"targetCandidates\":[],\"sideEffectIntent\":\"none\",\"ambiguity\":\"none\",\"requiredContext\":[],\"confirmationPoints\":[]},\"route\":{\"summary\":\"只读列出候选\",\"steps\":[]}}}\n\
最小问候示例：<binder-turn-control>{\"version\":\"binder-turn-control-v1\",\"nextAction\":\"final\",\"intent\":{\"goal\":\"回应问候\",\"intentType\":\"answer\",\"targetCandidates\":[],\"sideEffectIntent\":\"none\",\"ambiguity\":\"none\",\"requiredContext\":[],\"confirmationPoints\":[]},\"route\":{\"summary\":\"直接回应\",\"steps\":[]}}</binder-turn-control>你好，有什么需要我帮忙？\n这不是隐藏推理：只写简短结论和候选步骤，不写思维链。它不授权工具。\n\
若可直接回答，nextAction=final 并在封套后输出正文；若缺少必须信息，nextAction=clarify。需要工具时直接返回原生 tool_calls，并在首个工具参数的 _binderTurn 中提交一份响应级语义结论（nextAction=tool）；后续工具不重复。也可在文本封套中提交这份结论，二者择一；不要把 _binderTurn 当作领域参数。最终答复必须 nextAction=final；阻断澄清必须 nextAction=clarify 且 ambiguity=blocking。\n\
[/BINDER TURN OUTPUT PROTOCOL]"
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn tool_manifest_allows_batch_control_and_executor_strips_it_from_domain_params() {
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
        assert!(!tools[0]["function"]["parameters"]["required"]
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

        assert_eq!(
            parse_tool_batch(&[second], None).unwrap_err().code,
            "AG_MODEL_CONTROL_MISSING"
        );
        let mut different = expected.clone();
        different.intent.goal = "private goal must not enter diagnostics".into();
        let conflict = parse_tool_batch(&[&first], Some(different)).unwrap_err();
        assert_eq!(conflict.code, "AG_MODEL_CONTROL_CONFLICT");
        assert!(conflict.message.contains("text 与 tool[0]"));
        assert!(conflict.message.contains("intent.goal"));
        assert!(!conflict.message.contains("private goal"));
        let mut changed = expected.clone();
        changed.route.summary = "private route must not enter diagnostics".into();
        let conflicting_second =
            serde_json::json!({"path": "b.md", "_binderTurn": changed}).to_string();
        let conflict = parse_tool_batch(&[&first, &conflicting_second], None).unwrap_err();
        assert!(conflict.message.contains("tool[0] 与 tool[1]"));
        assert!(conflict.message.contains("route.summary"));
        assert!(!conflict.message.contains("private route"));
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
        let native = prompt
            .split("原生 workspace_glob 的参数示例：")
            .nth(1)
            .unwrap()
            .split('\n')
            .next()
            .unwrap();
        assert_eq!(
            parse_tool_batch(&[native], None).unwrap().0.next_action,
            ModelNextAction::Tool
        );
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
