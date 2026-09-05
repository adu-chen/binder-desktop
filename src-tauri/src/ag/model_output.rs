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

/**
 * @GOV
 * codes:    BR-AG-DATA-004-GUARD-AG-AG-TURN-047, BR-AG-STATE-001-GUARD-AG-AG-TURN-047
 * type:     GUARD
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-004, BR-AG-STATE-001
 * term_ref: TERM-AG-012
 * boundary: in=ProviderVisibleTextPrefix | out=ValidatedModelIntentRouteAndNextActionWithHiddenControlRemoved | does_not=acceptAuthorizationHiddenReasoningUnknownFieldsOrUnstructuredFirstIntent
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
            }
            ModelNextAction::Tool | ModelNextAction::Final => {
                if blocking || self.clarification_question.is_some() {
                    return Err(invalid("blocking ambiguity 只能进入 clarify"));
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
#[derive(Debug, Default)]
pub struct ModelOutputDecoder {
    pending: String,
    control: Option<ModelTurnControl>,
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

/// 把语义结论作为每个 provider 工具调用的必填内部元数据；它不会进入领域工具校验或执行。
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
        if !required
            .iter()
            .any(|item| item.as_str() == Some("_binderTurn"))
        {
            required.push(serde_json::Value::String("_binderTurn".to_string()));
        }
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
        if trimmed.len() < CONTROL_OPEN.len() && CONTROL_OPEN.starts_with(trimmed) {
            return Ok(Vec::new());
        }
        if !trimmed.starts_with(CONTROL_OPEN) {
            return Err(IpcError::new(
                "AG_MODEL_CONTROL_MISSING",
                "模型响应缺少结构化回合控制封套",
                true,
                false,
            ));
        }
        let Some(close_offset) = trimmed.find(CONTROL_CLOSE) else {
            if trimmed.chars().count() > CONTROL_MAX_CHARS {
                return Err(IpcError::new(
                    "AG_MODEL_CONTROL_TOO_LARGE",
                    "模型控制封套超过大小上限",
                    true,
                    false,
                ));
            }
            return Ok(Vec::new());
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
        let control: ModelTurnControl = serde_json::from_str(json).map_err(|error| {
            IpcError::new(
                "AG_MODEL_CONTROL_INVALID",
                &format!("模型控制封套不是合法结构：{error}"),
                true,
                false,
            )
        })?;
        control.validate()?;
        let body_start = close_offset + CONTROL_CLOSE.len();
        let body = trimmed[body_start..].to_string();
        self.control = Some(control);
        self.pending.clear();
        Ok((!body.is_empty()).then_some(body).into_iter().collect())
    }

    pub fn finish(self) -> Result<ModelTurnControl, IpcError> {
        self.control.ok_or_else(|| {
            IpcError::new(
                "AG_MODEL_CONTROL_INCOMPLETE",
                "模型响应结束时结构化回合控制封套仍不完整",
                true,
                false,
            )
        })
    }

    pub fn finish_optional(self) -> Result<Option<ModelTurnControl>, IpcError> {
        if self.control.is_some() {
            return Ok(self.control);
        }
        if self.pending.trim().is_empty() {
            return Ok(None);
        }
        Err(IpcError::new(
            "AG_MODEL_CONTROL_INCOMPLETE",
            "模型工具响应包含不完整的结构化回合控制封套",
            true,
            false,
        ))
    }
}

pub fn protocol_prompt() -> &'static str {
    "[BINDER TURN OUTPUT PROTOCOL]\n\
每个不含原生 tool_calls 的响应，必须以且只以一个 <binder-turn-control>{JSON}</binder-turn-control> 开始。\n\
JSON 严格形状：{\"version\":\"binder-turn-control-v1\",\"nextAction\":\"clarify|final\",\"intent\":{\"goal\":\"当前目标\",\"intentType\":\"answer|inspect|search|analyze|edit|create_document|organize_workspace\",\"targetCandidates\":[{\"kind\":\"current_document|selection|explicit_document|new_document|workspace\",\"relPath\":\"可选路径\"}],\"sideEffectIntent\":\"none|content_proposal|structure_plan\",\"ambiguity\":\"none|non_blocking|blocking\",\"requiredContext\":[],\"confirmationPoints\":[]},\"route\":{\"summary\":\"可见的简短路线\",\"steps\":[]},\"clarificationQuestion\":\"仅 clarify 时提供\"}。\n\
这不是隐藏推理：只写简短结论和候选步骤，不写思维链。它不授权工具。\n\
若可直接回答，nextAction=final 并在封套后输出正文；若缺少必须信息，nextAction=clarify。需要工具时直接返回原生 tool_calls，并在每个工具参数的 _binderTurn 中提交同一份语义结论（nextAction=tool）；不要把 _binderTurn 当作领域参数。最终答复必须 nextAction=final；阻断澄清必须 nextAction=clarify 且 ambiguity=blocking。\n\
[/BINDER TURN OUTPUT PROTOCOL]"
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let error = invalid
            .push(&control("clarify", "none", Some("要处理哪一份文档？")))
            .unwrap_err();
        assert_eq!(error.code, "AG_MODEL_CONTROL_INVALID");
    }

    #[test]
    fn unstructured_or_unresolved_control_fails_closed() {
        let mut unstructured = ModelOutputDecoder::default();
        assert_eq!(
            unstructured.push("直接回答").unwrap_err().code,
            "AG_MODEL_CONTROL_MISSING"
        );
        let unresolved = control("final", "none", None).replace("\"inspect\"", "\"unresolved\"");
        let mut decoder = ModelOutputDecoder::default();
        assert_eq!(
            decoder.push(&unresolved).unwrap_err().code,
            "AG_MODEL_CONTROL_INVALID"
        );
    }

    #[test]
    fn tool_manifest_requires_control_and_executor_strips_it_from_domain_params() {
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
        assert!(tools[0]["function"]["parameters"]["required"]
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
