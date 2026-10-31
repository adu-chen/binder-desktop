//! PI-AG-01（B-1/B-4）+ PI-AG-SEM-01 · 上下文组装 + 提示词骨架 + 目标边界 · AG-TURN
//! 规则：BR-AG-DATA-001（来源限定/裁剪优先级）/ BR-AG-DATA-002（提示词组装骨架）/ BR-AG-DATA-003（目标集边界）
//!
//! 承接 A-AG-M-T-01：上下文来源限定为 当前文档 / REF @mention / AST-KB 本地资产，限 workspace 内；超预算
//! 裁剪确定性且**显式 @mention 优先保留、裁剪可追溯**（不静默丢弃显式注入，B-1）；提示词按统一分层骨架
//! 系统→工具→上下文→用户组装，三类回合骨架差异（编辑/构建/辅助）；ChangeProposal target 边界按
//! BR-AG-DATA-003 2026-07-04 版：默认当前文档，显式声明集内可跨，新建经 diff 裁决闸门，未声明既有文档拒绝
//! （原「单文档铁律/跨文档升级 CBT」已废止）。预算数值/裁剪算法/提示词文本延实现期。

use sha2::{Digest, Sha256};

pub const PROMPT_RUNTIME_VERSION_V2: &str = "prompt-runtime-v2";

/// Active 1.0 运行时意图语义。它只描述模型对当前目标的可见结论，不包含工具授权。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentTypeV2 {
    Unresolved,
    Answer,
    Inspect,
    Search,
    Analyze,
    Edit,
    CreateDocument,
    OrganizeWorkspace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SideEffectIntentV2 {
    None,
    ContentProposal,
    StructurePlan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmbiguityLevelV1 {
    None,
    NonBlocking,
    Blocking,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntentTargetCandidateV2 {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rel_path: Option<String>,
}

/**
 * @GOV
 * codes:    BR-AG-DATA-004-DATA-AG-AG-TURN-046, BR-AG-STATE-001-DATA-AG-AG-TURN-046
 * type:     DATA
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-004, BR-AG-STATE-001
 * term_ref: TERM-AG-012
 * boundary: in=ModelVisibleCurrentTurnSemanticProjection | out=VersionedIntentResultWithoutAuthorizationOrHiddenReasoning | does_not=containAllowedToolsOrOverrideExplicitEditTargetSet | consumer=BR-AG-STATE-001
 */
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntentResultV2 {
    pub revision: u64,
    pub source_event_seq: u64,
    pub goal: String,
    pub intent_type: IntentTypeV2,
    pub target_candidates: Vec<IntentTargetCandidateV2>,
    pub side_effect_intent: SideEffectIntentV2,
    pub ambiguity: AmbiguityLevelV1,
    pub required_context: Vec<String>,
    pub confirmation_points: Vec<String>,
}

impl IntentResultV2 {
    pub fn unresolved() -> Self {
        Self {
            revision: 0,
            source_event_seq: 0,
            goal: "unresolved".to_string(),
            intent_type: IntentTypeV2::Unresolved,
            target_candidates: Vec::new(),
            side_effect_intent: SideEffectIntentV2::None,
            ambiguity: AmbiguityLevelV1::None,
            required_context: vec!["current_user_input".to_string()],
            confirmation_points: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptSourceTypeV1 {
    AgentContextFacts,
    CurrentDocument,
    Selection,
    ExplicitReference,
    SessionHistory,
    CurrentUserInput,
    UserSupplement,
    ToolResult,
    DecisionResult,
    ModelContinuation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptTrustLevelV1 {
    CurrentFact,
    RuntimeObservation,
    UserProvidedData,
    HistoricalData,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptDataEnvelopeV1 {
    pub source_type: PromptSourceTypeV1,
    pub source_id: String,
    pub provenance: String,
    pub freshness: String,
    pub content_hash: String,
    pub trust_level: PromptTrustLevelV1,
    pub token_count: usize,
    pub original_char_count: usize,
    pub included_char_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub truncation_reason: Option<String>,
    pub payload: String,
}

pub fn sha256_text(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

/// 统一预算口径（BR-AG-DATA-002 预算结果）：以实际序列化 messages + tools 的字符数估算输入 token，
/// 供 gate 与 audit 使用同一口径，避免仅信封之和导致的低估。估算为 ceil(chars / 4)（dev 近似）。
pub fn estimate_serialized_input_tokens(
    messages: &[serde_json::Value],
    tools: &serde_json::Value,
) -> usize {
    measure_prompt_budget(messages, tools).estimated_tokens
}

/// The provider projection is shared by sending, the budget gate and optional request audits.
pub fn provider_message_projection(messages: &[serde_json::Value]) -> Vec<serde_json::Value> {
    super::model_output::project_native_batch_history(messages)
        .into_iter()
        .map(|(message, _)| message)
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptBudgetMeasurement {
    pub estimated_tokens: usize,
    pub serialized_chars: usize,
    pub message_framing_chars: usize,
    pub tool_schema_chars: usize,
    pub message_layers: std::collections::BTreeMap<String, usize>,
}

/**
 * @GOV
 * codes: BR-AG-DATA-002-DATA-AG-AG-TURN-055, BR-AG-DATA-005-DATA-AG-AG-TURN-055
 * type: DATA
 * chain: AG-TURN
 * rules: BR-AG-DATA-002, BR-AG-DATA-005
 * boundary: in=PromptMessagesAndToolSchema | out=ProviderProjectedSerializedCharacterLedgerAndApproximateTokenTotal | does_not=retainContentAlterAuthorizationOrTruncateRequiredSources | consumer=BR-AG-DATA-002
 */
pub fn measure_prompt_budget(
    messages: &[serde_json::Value],
    tools: &serde_json::Value,
) -> PromptBudgetMeasurement {
    // Value serialization to String is infallible; no zero-on-error budget fallback.
    let projected = provider_message_projection(messages);
    let chars = |value: &serde_json::Value| value.to_string().chars().count();
    let mut message_layers = std::collections::BTreeMap::<String, usize>::new();
    for (sent, source_layer) in super::model_output::project_native_batch_history(messages) {
        let layer = source_layer.as_deref();
        let known_layer = layer_order()
            .into_iter()
            .map(|k| k.as_str())
            .find(|k| Some(*k) == layer);
        let category =
            known_layer.unwrap_or_else(|| match sent.get("role").and_then(|v| v.as_str()) {
                Some("tool") => "tool_result",
                Some("assistant") => "model_continuation",
                Some("user") => "user_supplement",
                Some("system") => "continuation_receipt",
                _ => "unclassified",
            });
        *message_layers.entry(category.to_string()).or_default() += chars(&sent);
    }
    let message_framing_chars = 2 + projected.len().saturating_sub(1);
    let tool_schema_chars = chars(tools);
    let serialized_chars =
        message_layers.values().sum::<usize>() + message_framing_chars + tool_schema_chars;
    PromptBudgetMeasurement {
        estimated_tokens: serialized_chars.div_ceil(4),
        serialized_chars,
        message_framing_chars,
        tool_schema_chars,
        message_layers,
    }
}

pub fn prompt_data_envelope(
    source_type: PromptSourceTypeV1,
    source_id: &str,
    provenance: &str,
    freshness: &str,
    trust_level: PromptTrustLevelV1,
    payload: &str,
) -> PromptDataEnvelopeV1 {
    prompt_data_envelope_with_budget(
        source_type,
        source_id,
        provenance,
        freshness,
        trust_level,
        payload,
        payload.chars().count(),
        None,
    )
}

pub fn prompt_data_envelope_with_budget(
    source_type: PromptSourceTypeV1,
    source_id: &str,
    provenance: &str,
    freshness: &str,
    trust_level: PromptTrustLevelV1,
    payload: &str,
    original_char_count: usize,
    truncation_reason: Option<&str>,
) -> PromptDataEnvelopeV1 {
    let included_char_count = payload.chars().count();
    PromptDataEnvelopeV1 {
        source_type,
        source_id: source_id.to_string(),
        provenance: provenance.to_string(),
        freshness: freshness.to_string(),
        content_hash: sha256_text(payload),
        trust_level,
        token_count: included_char_count.div_ceil(4),
        original_char_count,
        included_char_count,
        truncation_reason: truncation_reason.map(str::to_string),
        payload: payload.to_string(),
    }
}

/// Active 1.0 每次 provider 调用前重新构建的内部 PromptPackage。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptPackageV2 {
    pub runtime_version: String,
    pub intent_revision: u64,
    pub context_trace_id: String,
    pub capability_grant_id: String,
    pub tool_manifest_hash: String,
    pub estimated_tokens: usize,
    pub messages: Vec<serde_json::Value>,
}

pub fn build_prompt_package_v2(
    intent: &IntentResultV2,
    envelopes: &[PromptDataEnvelopeV1],
    capability_grant_id: &str,
    tool_manifest_hash: &str,
    messages: Vec<serde_json::Value>,
    tools: &serde_json::Value,
) -> PromptPackageV2 {
    let trace_material = serde_json::to_string(&serde_json::json!({
        "intentRevision": intent.revision,
        "capabilityGrantId": capability_grant_id,
        "toolManifestHash": tool_manifest_hash,
        "sources": envelopes.iter().map(|envelope| serde_json::json!({
            "sourceType": envelope.source_type,
            "sourceId": envelope.source_id,
            "provenance": envelope.provenance,
            "freshness": envelope.freshness,
            "contentHash": envelope.content_hash,
            "trustLevel": envelope.trust_level,
            "tokenCount": envelope.token_count,
            "originalCharCount": envelope.original_char_count,
            "includedCharCount": envelope.included_char_count,
            "truncationReason": envelope.truncation_reason,
        })).collect::<Vec<_>>(),
    }))
    .unwrap_or_else(|_| {
        format!(
            "{capability_grant_id}:{tool_manifest_hash}:{}",
            intent.revision
        )
    });
    let estimated_tokens = estimate_serialized_input_tokens(&messages, tools);
    PromptPackageV2 {
        runtime_version: PROMPT_RUNTIME_VERSION_V2.to_string(),
        intent_revision: intent.revision,
        context_trace_id: format!("ctx-{}", &sha256_text(&trace_material)[..16]),
        capability_grant_id: capability_grant_id.to_string(),
        tool_manifest_hash: tool_manifest_hash.to_string(),
        estimated_tokens,
        messages,
    }
}

/// 提示词分层唯一真源（BR-AG-DATA-002 实现映射）：装配顺序、审计层名与版本登记共用同一枚举，
/// 取代早期仅供测试的四层骨架。层序固定，不另起顺序。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerKind {
    System,            // 静态提示词文本（P0 · system）
    RuntimeControl,    // 结构化运行时控制 JSON（P0 · system）
    IntentHint,        // 本轮意图提示（P0 · system）
    AgentContextFacts, // 版本化 ED/DE/WS 当前轮事实（P0 · data-only system）
    CurrentDocument,   // 当前文档块视图（P0 · system）
    Reference,         // 显式引用（P0 · system）
    History,           // 会话历史聚合单块（P1 · system）
    CrossDocHistory,   // 跨文档历史弱注入（P1 · system）
    User,              // 当前输入（P0 · user）
}

impl LayerKind {
    /// audit 层名（与装配 layer 元数据一致，审计直接读取不做字符串推断）。
    pub fn as_str(&self) -> &'static str {
        match self {
            LayerKind::System => "system_prompt_final",
            LayerKind::RuntimeControl => "runtime_control",
            LayerKind::IntentHint => "intent_hint",
            LayerKind::AgentContextFacts => "agent_context_facts",
            LayerKind::CurrentDocument => "current_document",
            LayerKind::Reference => "explicit_reference",
            LayerKind::History => "session_history",
            LayerKind::CrossDocHistory => "cross_document_history",
            LayerKind::User => "current_user_input",
        }
    }
}

/**
 * @GOV
 * codes:    BR-AG-DATA-002-DATA-AG-AG-TURN-016
 * type:     DATA
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-002
 * boundary: in=PromptLayerKinds | out=LayerOrder(systemRuntimeControlIntentFactsDocRefHistoryUser) | consumer=BR-AG-DATA-002
 */
/// 提示词装配层序（BR-AG-DATA-002）：恒为 System→RuntimeControl→IntentHint→CurrentDocument→Reference→
/// History→CrossDocHistory→User；LayerKind 是装配顺序、审计层名与版本登记的唯一真源，取代旧四层骨架。
/// 提示词文本/措辞属实现期，本函数锁分层顺序与差异。
pub fn layer_order() -> [LayerKind; 9] {
    [
        LayerKind::System,
        LayerKind::RuntimeControl,
        LayerKind::IntentHint,
        LayerKind::AgentContextFacts,
        LayerKind::CurrentDocument,
        LayerKind::Reference,
        LayerKind::History,
        LayerKind::CrossDocHistory,
        LayerKind::User,
    ]
}

// ── PI-AG-09B：对话历史 + P0–P3 预算截断 ───────────────────────────────────────────
//
// 承 BR-AG-DATA-005（M-T-02 分层截断序）：当前文档/当前轮输入 P0（层保留、超长内容可缩）；历史 P1
// （按预算保留最近、drop 最旧，确定性可追溯）。历史保持 role 结构化，不降级为普通文本项。

/// 对话历史轮（前端 agStore 消息投影）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistTurn {
    pub workspace_id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    #[serde(default)]
    pub rel_path: Option<String>,
    #[serde(default)]
    pub turn_id: Option<String>,
}

/// P0 文档层内容封顶（层保留、内容可缩）：超 `max_chars` 截断 + 显式注记（不静默丢弃，可追溯）。
pub fn cap_text(s: &str, max_chars: usize, note: &str) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let head: String = s.chars().take(max_chars).collect();
    format!("{head}\n{note}")
}

#[derive(Debug)]
pub(super) struct HistorySelection {
    pub turns: Vec<HistTurn>,
    pub dropped: usize,
    pub excerpted: usize,
}

// Verbatim head/tail only, not a semantic summary. The omission is visible to the
// model and the original message remains unchanged in the store/database.
fn history_excerpt(content: &str, allowance: usize) -> Option<String> {
    let count = content.chars().count();
    if count <= allowance {
        return Some(content.to_string());
    }
    let marker = format!(
        "\n[历史原文摘录；中间已省略，非完整消息；originalChars={count}; sha256={}]\n",
        sha256_text(content)
    );
    let visible = allowance.checked_sub(marker.chars().count())?;
    if visible < 2 {
        return None;
    }
    let head = visible.div_ceil(2);
    let tail = content
        .chars()
        .rev()
        .take(visible - head)
        .collect::<Vec<_>>();
    Some(
        content.chars().take(head).collect::<String>()
            + &marker
            + &tail.into_iter().rev().collect::<String>(),
    )
}

/// Keep recent complete conversation units. Only the newest oversized unit is
/// excerpted; its user request gets first claim on capacity, never a lone large
/// assistant answer that displaces every user constraint. Wire overhead is fitted
/// by prepare_turn_prompt against the single complete-request budget afterwards.
/**
 * @GOV
 * codes:    BR-AG-DATA-005-DATA-AG-AG-TURN-004
 * type:     DATA
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-005
 * boundary: in=IdentityScopedHistTurnsAndDerivedCharAllowance | out=RecentHistorySelectionWithTurnGroupsMarkedVerbatimExcerptsAndDropCounts | does_not=mutatePersistedHistoryOrGrantAuthority
 */
pub(super) fn select_recent_history(turns: &[HistTurn], budget_chars: usize) -> HistorySelection {
    if budget_chars == 0 {
        return HistorySelection {
            turns: Vec::new(),
            dropped: turns.len(),
            excerpted: 0,
        };
    }
    let mut groups: Vec<&[HistTurn]> = Vec::new();
    let mut start = 0;
    for index in 1..turns.len() {
        let previous = &turns[index - 1];
        let current = &turns[index];
        let new_group = match (&previous.turn_id, &current.turn_id) {
            (Some(left), Some(right)) => left != right,
            (None, None) => current.role == "user",
            _ => true,
        };
        if new_group {
            groups.push(&turns[start..index]);
            start = index;
        }
    }
    if start < turns.len() {
        groups.push(&turns[start..]);
    }
    let mut kept_rev = Vec::new();
    let mut used = 0usize;
    let mut excerpted = 0;
    for group in groups.into_iter().rev() {
        let remaining = budget_chars.saturating_sub(used);
        let count = group
            .iter()
            .map(|turn| turn.content.chars().count())
            .sum::<usize>();
        if count <= remaining {
            kept_rev.extend(group.iter().rev().cloned());
            used += count;
            continue;
        }
        if kept_rev.is_empty() && remaining > 0 {
            let users = group
                .iter()
                .enumerate()
                .filter(|(_, t)| t.role == "user")
                .map(|(i, _)| i)
                .collect::<Vec<_>>();
            let others = group
                .iter()
                .enumerate()
                .filter(|(_, t)| t.role != "user")
                .map(|(i, _)| i)
                .collect::<Vec<_>>();
            let mut selected = vec![None; group.len()];
            let mut remaining = remaining;
            for (phase, (indices, reserve_answer)) in
                [(&users, !others.is_empty()), (&others, false)]
                    .into_iter()
                    .enumerate()
            {
                if phase == 1
                    && !users.is_empty()
                    && users.iter().all(|&index| selected[index].is_none())
                {
                    // Never inject an answer from this unit after dropping every
                    // user request that gives the answer its meaning.
                    break;
                }
                let role_total = indices
                    .iter()
                    .map(|&index| group[index].content.chars().count())
                    .sum::<usize>();
                let mut role_allowance = if reserve_answer && role_total > remaining {
                    remaining.div_ceil(2)
                } else if reserve_answer {
                    // Preserve all user constraints if they fit. Do not excerpt
                    // a fitting request merely to give its answer half the room.
                    role_total
                } else {
                    remaining
                };
                for (position, &index) in indices.iter().rev().enumerate() {
                    let unprocessed = &indices[..indices.len() - position];
                    let unprocessed_chars = unprocessed
                        .iter()
                        .map(|&index| group[index].content.chars().count())
                        .sum::<usize>();
                    let allowance = if unprocessed_chars <= role_allowance {
                        group[index].content.chars().count()
                    } else {
                        role_allowance / unprocessed.len()
                    };
                    if let Some(content) = history_excerpt(&group[index].content, allowance) {
                        let included = content.chars().count();
                        let mut turn = group[index].clone();
                        excerpted += usize::from(content != turn.content);
                        turn.content = content;
                        selected[index] = Some(turn);
                        role_allowance -= included;
                        remaining -= included;
                    }
                }
            }
            kept_rev.extend(selected.into_iter().rev().flatten());
        }
        break;
    }
    let dropped = turns.len() - kept_rev.len();
    kept_rev.reverse();
    HistorySelection {
        turns: kept_rev,
        dropped,
        excerpted,
    }
}

/// 按当前文档归属分流后的历史。same_doc 可作为同文档 P1 背景；other_doc 始终只能弱注入。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedHistory {
    pub same_doc: Vec<HistTurn>,
    pub other_doc: Vec<HistTurn>,
}

/**
 * @GOV
 * codes:    BR-AG-DATA-004-DATA-AG-AG-TURN-005
 * type:     DATA
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-004
 * term_ref: TERM-AG-005
 * boundary: in=currentDocPathAndIdentityValidatedHistTurns | out=ScopedHistory(sameDocHistoryAndCrossDocWeakCandidatesWithoutKeywordJudgment)
 */
/// 历史归属分流（BR-AG-DATA-004）：有当前文档时，只有同 relPath 历史可作为同文档 P1 背景；
/// 跨文档或无 relPath 的历史只进入有界弱候选。是否沿用由模型 IntentResult 表达，本层不解析关键词。
pub fn scope_history_for_current_doc(
    turns: &[HistTurn],
    current_rel: Option<&str>,
) -> ScopedHistory {
    let Some(current) = current_rel.filter(|p| !p.trim().is_empty()) else {
        return ScopedHistory {
            same_doc: turns.to_vec(),
            other_doc: Vec::new(),
        };
    };
    let mut same_doc = Vec::new();
    let mut other_doc_all = Vec::new();
    for turn in turns {
        if turn.rel_path.as_deref() == Some(current) {
            same_doc.push(turn.clone());
        } else {
            other_doc_all.push(turn.clone());
        }
    }
    ScopedHistory {
        same_doc,
        other_doc: other_doc_all,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: BR-AG-DATA-002
    // covers: BR-AG-DATA-005
    #[test]
    fn budget_ledger_matches_wire_unicode_escaping_and_unknown_layers() {
        let messages = vec![
            serde_json::json!({"role":"user", "layer":"user_input", "content":"预算\n\"\\😀"}),
            serde_json::json!({"role":"assistant", "layer":"PRIVATE_LAYER", "tool_calls":[{"id":"x", "arguments":"{\"a\":1}"}]}),
            serde_json::json!({"role":"tool", "content":"PRIVATE_RESULT"}),
        ];
        let tools = serde_json::json!([{"name":"read_doc","description":"读取"}]);
        let m = measure_prompt_budget(&messages, &tools);
        let sent = provider_message_projection(&messages);
        let actual = serde_json::to_string(&sent).unwrap().chars().count()
            + tools.to_string().chars().count();
        assert_eq!(m.serialized_chars, actual);
        assert_eq!(m.estimated_tokens, actual.div_ceil(4));
        assert_eq!(
            m.message_layers.values().sum::<usize>()
                + m.message_framing_chars
                + m.tool_schema_chars,
            actual
        );
        assert_eq!(m, measure_prompt_budget(&messages, &tools));
        assert_eq!(
            estimate_serialized_input_tokens(&sent, &tools),
            m.estimated_tokens
        );
        let diagnostic = serde_json::to_string(&m).unwrap();
        assert!(!diagnostic.contains("PRIVATE"));
        assert!(!diagnostic.contains("预算"));
        assert!(messages[0].get("layer").is_some());
        assert!(sent.iter().all(|v| v.get("layer").is_none()));
        let empty = measure_prompt_budget(&[], &serde_json::json!([]));
        assert_eq!(empty.serialized_chars, 4);
    }

    // covers: BR-AG-DATA-005
    #[test]
    fn budget_exact_threshold_uses_one_global_rounding_step() {
        let limit = super::super::prompt_runtime::PROMPT_TOKEN_BUDGET;
        let tools = serde_json::json!([]);
        let base =
            measure_prompt_budget(&[serde_json::json!({"role":"user","content":""})], &tools)
                .serialized_chars;
        for (delta, expected) in [(-1_i64, limit), (0, limit), (1, limit + 1)] {
            let length = (limit * 4 - base) as i64 + delta;
            let m = measure_prompt_budget(
                &[serde_json::json!({"role":"user","content":"中".repeat(length as usize)})],
                &tools,
            );
            assert_eq!(m.estimated_tokens, expected);
        }
    }

    // covers: BR-AG-DATA-002
    // 层序唯一真源：LayerKind 固定为 System→RuntimeControl→IntentHint→AgentContextFacts→CurrentDocument→Reference→History→CrossDocHistory→User。
    #[test]
    fn layer_order_is_fixed_and_matches_enum() {
        let order = layer_order();
        assert_eq!(order.len(), 9);
        assert_eq!(order[0], LayerKind::System);
        assert_eq!(order[1], LayerKind::RuntimeControl);
        assert_eq!(order[2], LayerKind::IntentHint);
        assert_eq!(order[3], LayerKind::AgentContextFacts);
        assert_eq!(order[4], LayerKind::CurrentDocument);
        assert_eq!(order[5], LayerKind::Reference);
        assert_eq!(order[6], LayerKind::History);
        assert_eq!(order[7], LayerKind::CrossDocHistory);
        assert_eq!(order[8], LayerKind::User);
        // 层名是审计与装配共用的唯一标识，不允许空或重复。
        let names: Vec<&str> = order.iter().map(|kind| kind.as_str()).collect();
        assert!(names.iter().all(|name| !name.is_empty()));
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), names.len(), "layer 名不得重复");
    }

    // PI-AG-09B：文档层封顶（短不动、长截断 + 注记，不静默）。
    #[test]
    fn cap_text_truncates_long_with_note() {
        assert_eq!(cap_text("短", 10, "[截断]"), "短");
        let out = cap_text("一二三四五六七八九十", 3, "[已截断]");
        assert!(out.starts_with("一二三"));
        assert!(out.contains("[已截断]"));
        assert!(!out.contains("四"));
    }

    // covers: BR-AG-DATA-005
    // PI-AG-09B：分层截断序——历史层（P1）按预算保留最近、drop 最旧（确定性、丢弃数可追溯）。
    #[test]
    fn select_recent_history_keeps_newest_drops_oldest() {
        let h = |r: &str, c: &str| HistTurn {
            workspace_id: "ws-1".into(),
            session_id: "session-1".into(),
            role: r.into(),
            content: c.into(),
            rel_path: None,
            turn_id: None,
        };
        let turns = vec![
            h("user", "aaaa"),
            h("assistant", "bbbb"),
            h("user", "cc"),
            h("assistant", "dd"),
        ];
        // Keep the newest complete pair, not an orphaned answer from an older pair.
        let selected = select_recent_history(&turns, 6);
        assert_eq!(selected.dropped, 2);
        assert_eq!(selected.excerpted, 0);
        assert_eq!(selected.turns.len(), 2);
        assert_eq!(selected.turns[0].content, "cc");
        assert_eq!(selected.turns[1].content, "dd");
        // 预算 0：全丢弃。
        assert_eq!(select_recent_history(&turns, 0).dropped, 4);
    }

    // covers: BR-AG-DATA-004
    // covers: BR-AG-DATA-005
    #[test]
    fn oversized_latest_history_keeps_user_and_marks_unicode_answer_excerpt() {
        let message = |role: &str, content: String| HistTurn {
            workspace_id: "ws".into(),
            session_id: "session".into(),
            role: role.into(),
            content,
            rel_path: Some("a.txt".into()),
            turn_id: Some("prior".into()),
        };
        let answer = format!("ANSWER-HEAD{}ANSWER-TAIL", "中😀\\\"".repeat(3000));
        let history = vec![
            message("user", "分析A/B组；只读，不修改。".into()),
            message("assistant", answer.clone()),
        ];
        let selected = select_recent_history(&history, 900);
        assert_eq!(selected.dropped, 0);
        assert_eq!(selected.excerpted, 1);
        assert_eq!(selected.turns[0], history[0]);
        let excerpt = &selected.turns[1].content;
        assert!(excerpt.starts_with("ANSWER-HEAD") && excerpt.ends_with("ANSWER-TAIL"));
        assert!(excerpt.contains("非完整消息") && excerpt.contains(&sha256_text(&answer)));
        assert!(
            selected
                .turns
                .iter()
                .map(|t| t.content.chars().count())
                .sum::<usize>()
                <= 900
        );
        assert_eq!(history[1].content, answer);
        assert_eq!(select_recent_history(&history, 900).turns, selected.turns);
        assert_eq!(select_recent_history(&history, 40).turns[0], history[0]);
        let no_orphan = vec![
            message("user", "用户约束".repeat(1000)),
            message("assistant", "已确认".into()),
        ];
        assert!(select_recent_history(&no_orphan, 40).turns.is_empty());
        let request_first = vec![
            message("user", "请求约束".repeat(750)),
            message("assistant", "回答内容".repeat(500)),
        ];
        let selected = select_recent_history(&request_first, 4000);
        assert_eq!(selected.turns[0], request_first[0]);
        assert_eq!(selected.excerpted, 1);
    }

    // covers: BR-AG-DATA-004
    // 历史归属：同 relPath 与跨文档候选按身份分层，Prompt 层再对跨文档内容弱注入。
    #[test]
    fn scope_history_separates_same_doc_from_cross_doc() {
        let h = |rel: Option<&str>, c: &str| HistTurn {
            workspace_id: "ws-1".into(),
            session_id: "session-1".into(),
            role: "user".into(),
            content: c.into(),
            rel_path: rel.map(str::to_string),
            turn_id: None,
        };
        let turns = vec![
            h(Some("a.md"), "A 历史"),
            h(Some("b.md"), "B 历史"),
            h(None, "旧版本无归属"),
        ];

        let scoped = scope_history_for_current_doc(&turns, Some("b.md"));
        assert_eq!(scoped.same_doc.len(), 1);
        assert_eq!(scoped.same_doc[0].content, "B 历史");
        assert_eq!(scoped.other_doc.len(), 2);

        let no_current = scope_history_for_current_doc(&turns, None);
        assert_eq!(no_current.same_doc.len(), 3);
        assert!(no_current.other_doc.is_empty());
    }

    // covers: BR-AG-DATA-001
    // covers: BR-AG-DATA-002
    // covers: BR-AG-DATA-004
    #[test]
    fn prompt_package_v2_trace_changes_with_dynamic_source_and_records_truncation() {
        let intent = IntentResultV2::unresolved();
        let base = prompt_data_envelope(
            PromptSourceTypeV1::CurrentUserInput,
            "user-input:t1",
            "current_agent_turn_input",
            "current_turn",
            PromptTrustLevelV1::UserProvidedData,
            "检查当前文档",
        );
        let truncated = prompt_data_envelope_with_budget(
            PromptSourceTypeV1::ExplicitReference,
            "doc:large.md",
            "explicit_document_reference",
            "disk_state_read_at=1;baseline=7",
            PromptTrustLevelV1::CurrentFact,
            "引用摘要",
            50_000,
            Some("source_exceeded_per_reference_cap"),
        );
        let messages = vec![serde_json::json!({"role":"user","content":"检查当前文档"})];
        let tools = serde_json::json!([]);
        let first = build_prompt_package_v2(
            &intent,
            std::slice::from_ref(&base),
            "grant-1",
            "manifest-1",
            messages.clone(),
            &tools,
        );
        assert_eq!(base.provenance, "current_agent_turn_input");
        assert_eq!(base.content_hash, sha256_text(&base.payload));
        let second = build_prompt_package_v2(
            &intent,
            &[base, truncated.clone()],
            "grant-1",
            "manifest-1",
            messages,
            &tools,
        );

        assert_ne!(first.context_trace_id, second.context_trace_id);
        assert_eq!(truncated.original_char_count, 50_000);
        assert_eq!(truncated.included_char_count, "引用摘要".chars().count());
        assert_eq!(
            truncated.truncation_reason.as_deref(),
            Some("source_exceeded_per_reference_cap")
        );
    }
}
