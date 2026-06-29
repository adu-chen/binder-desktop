//! PI-AG-01（B-1/B-4）· 上下文组装 + 提示词骨架 + 单文档边界 · AG-TURN
//! 规则：BR-AG-DATA-001（来源限定/裁剪优先级）/ BR-AG-DATA-002（提示词组装骨架）/ BR-AG-DATA-003（单文档边界）
//!
//! 承接 A-AG-M-T-01：上下文来源限定为 当前文档 / REF @mention / AST-KB 本地资产，限 workspace 内；超预算
//! 裁剪确定性且**显式 @mention 优先保留、裁剪可追溯**（不静默丢弃显式注入，B-1）；提示词按统一分层骨架
//! 系统→工具→上下文→用户组装，三类回合骨架差异（chat/构建/内联）；单文档回合 ChangeProposal target 限
//! 当前文档，跨文档意图**升级 CBT**（B-4，2026-06-12 解除）。预算数值/裁剪算法/提示词文本延实现期。

use crate::types::DocRef;

/// 上下文来源（裁剪保留优先级：ExplicitMention > Memory ≈ Knowledge > CurrentDoc）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextSource {
    ExplicitMention, // 用户显式 @mention（最高，永不静默丢弃）
    Memory,
    Knowledge,
    CurrentDoc,
}

fn priority(s: ContextSource) -> u8 {
    match s {
        ContextSource::ExplicitMention => 0,
        ContextSource::Memory => 1,
        ContextSource::Knowledge => 1,
        ContextSource::CurrentDoc => 2,
    }
}

/// 待组装上下文项（cost = 预算占用单位）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextItem {
    pub source: ContextSource,
    pub content: String,
    pub cost: u32,
}

/// 组装结果：保留项 + 被裁剪项（可追溯，不静默丢弃）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssembledContext {
    pub kept: Vec<ContextItem>,
    pub trimmed: Vec<ContextItem>,
}

/**
 * @GOV
 * codes:    BR-AG-DATA-001-DATA-AG-AG-TURN-001
 * type:     DATA
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-001
 * boundary: in=contextSourcesAndBudget | out=AssembledContext(explicitMentionAlwaysKeptTrimTraceable)
 */
/// 上下文组装与确定性裁剪（BR-AG-DATA-001）：按保留优先级（显式 @mention > 记忆/知识 > 当前文档）
/// 在预算内贪心保留；**显式 @mention 永不裁剪**（即使超预算也保留）；被裁剪项进入 trimmed（可追溯，
/// 不静默丢弃）。同优先级按输入顺序确定性处理。
pub fn assemble_context(mut items: Vec<ContextItem>, budget: u32) -> AssembledContext {
    // 稳定排序保证确定性：优先级升序（0 最先保留），同级保持输入序。
    items.sort_by_key(|it| priority(it.source));
    let mut kept = Vec::new();
    let mut trimmed = Vec::new();
    let mut used = 0u32;
    for it in items {
        let is_explicit = it.source == ContextSource::ExplicitMention;
        if is_explicit || used.saturating_add(it.cost) <= budget {
            used = used.saturating_add(it.cost);
            kept.push(it); // 显式注入永不裁剪（即便超预算）
        } else {
            trimmed.push(it); // 裁剪可追溯
        }
    }
    AssembledContext { kept, trimmed }
}

/// 回合类型（骨架差异）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnKind {
    Chat,        // 对话历史 + 修改建议导向 diff
    BuildTask,   // 任务计划上下文 + 单文件产出导向
    InlineAssist, // 轻量、无系统对话历史、续写/局部改写
}

/// 提示词分层（固定顺序：系统→工具→上下文→用户）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptLayer {
    System,
    Tools,
    Context,
    User,
}

/// 组装出的提示词骨架（layers 恒为固定顺序；include_history 体现 chat 差异）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptSkeleton {
    pub kind: TurnKind,
    pub layers: Vec<PromptLayer>,
    pub include_history: bool,
}

/**
 * @GOV
 * codes:    BR-AG-DATA-002-DATA-AG-AG-TURN-002
 * type:     DATA
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-002
 * boundary: in=turnKindAndContext | out=PromptSkeleton(systemToolsContextUserOrderKindVariants)
 */
/// 提示词骨架组装（BR-AG-DATA-002）：分层恒为 系统→工具→上下文→用户；chat 含对话历史，内联辅助
/// 无系统对话历史（轻量），构建任务为单文件产出导向。提示词文本/措辞属实现期，本函数锁分层与差异。
pub fn assemble_prompt(kind: TurnKind) -> PromptSkeleton {
    PromptSkeleton {
        kind,
        layers: vec![
            PromptLayer::System,
            PromptLayer::Tools,
            PromptLayer::Context,
            PromptLayer::User,
        ],
        include_history: matches!(kind, TurnKind::Chat),
    }
}

// ── PI-AG-09B：对话历史 + P0–P3 预算截断 ───────────────────────────────────────────
//
// 承 BR-AG-DATA-005（M-T-02 分层截断序）：当前文档/当前轮输入 P0（层保留、超长内容可缩）；历史 P1
// （按预算保留最近、drop 最旧，确定性可追溯）。历史是 role 结构化层，独立于上面 assemble_context 的
// 内容源预算（assemble_context 处理 REF/AST/KB 补强源，③）。

/// 对话历史轮（前端 agStore 消息投影）。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct HistTurn {
    pub role: String,
    pub content: String,
}

/// P0 文档层内容封顶（层保留、内容可缩）：超 `max_chars` 截断 + 显式注记（不静默丢弃，可追溯）。
pub fn cap_text(s: &str, max_chars: usize, note: &str) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let head: String = s.chars().take(max_chars).collect();
    format!("{head}\n{note}")
}

/**
 * @GOV
 * codes:    BR-AG-DATA-005-DATA-AG-AG-TURN-004
 * type:     DATA
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-005
 * boundary: in=HistTurnsAndCharBudget | out=RecentHistory(keepNewestDropOldestDeterministicWithDropCount)
 */
/// P1 历史层按预算保留**最近**若干轮（drop 最旧，确定性）。返回 (保留<按时序>, 丢弃数<可追溯>)。
pub fn select_recent_history(turns: &[HistTurn], budget_chars: usize) -> (Vec<HistTurn>, usize) {
    let mut kept_rev = Vec::new();
    let mut used = 0usize;
    for t in turns.iter().rev() {
        let c = t.content.chars().count();
        if used.saturating_add(c) > budget_chars {
            break;
        }
        used += c;
        kept_rev.push(t.clone());
    }
    let dropped = turns.len() - kept_rev.len();
    kept_rev.reverse();
    (kept_rev, dropped)
}

/// 意图沿用判定结果（TERM-AG-005 IntentCarryover，BR-AG-DATA-004）。
/// None = 本轮为新请求，历史仅作背景；其余 = 本轮显式沿用历史意图，应提权参考。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Carryover {
    None,
    Refine,          // 修正追加（再短一点 / 改成… / 把刚才那段…）
    Select,          // 显式选择（用第二个 / 按刚才说的）
    ReferenceTarget, // 指代回指 / 续写（继续 / 接着 / 它 / 那个 / 上面）
}

/**
 * @GOV
 * codes:    BR-AG-DATA-004-DATA-AG-AG-TURN-003
 * type:     DATA
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-004
 * term_ref: TERM-AG-005
 * boundary: in=currentPromptAndHistoryPresence | out=Carryover(noneUnlessExplicitThenRefineSelectReferenceTarget)
 */
/// 意图沿用判定（BR-AG-DATA-004）：默认弱化历史，仅当本轮显式沿用历史意图时提权。
/// **dev 启发式**（识别策略延实现期：可换小模型/更细规则）；本函数锁「默认 None、显式沿用才非 None」机制。
pub fn decide_intent_carryover(prompt: &str, has_history: bool) -> Carryover {
    if !has_history {
        return Carryover::None;
    }
    let p = prompt.trim();
    let any = |ks: &[&str]| ks.iter().any(|k| p.contains(k));
    // 修正追加（操作对象=上一轮产出）。
    if any(&["改成", "换成", "短一点", "长一点", "再写", "再短", "再长", "正式一点", "把刚才", "把上面", "把上一", "重写"]) {
        return Carryover::Refine;
    }
    // 显式选择（沿用上一轮某个产出/方案）。
    if any(&["第一个", "第二个", "第三个", "按刚才", "按上面", "用刚才", "用上面", "就那个", "就用"]) {
        return Carryover::Select;
    }
    // 指代回指 / 续写（延续上一轮目标）。
    if any(&["继续", "接着", "接下来", "然后呢", "上面", "刚才", "那个", "这个", "它"]) {
        return Carryover::ReferenceTarget;
    }
    Carryover::None
}

/// 单文档边界裁决结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetDecision {
    Accept,        // target 即当前文档，单文档回合内产出
    EscalateCbt,   // 跨文档/多文件意图，升级 CBT 接管（不旁路产出跨文档 diff）
}

/**
 * @GOV
 * codes:    BR-AG-DATA-003-GUARD-AG-AG-TURN-001
 * type:     GUARD
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-003
 * boundary: in=currentDocAndProposalTarget | out=TargetDecision(acceptCurrentOrEscalateCbtNoCrossDoc)
 */
/// 单文档回合 target 守卫（BR-AG-DATA-003，解除 B-4）：ChangeProposal target 须为当前文档；指向其他
/// 文档 → EscalateCbt（升级 CBT 接管，不在单文档回合内旁路产出跨文档 diff）。与 DE 侧 generate_diff
/// 越界拒绝双侧把守。以 (workspace_id, rel_path) 判定同一文档。
pub fn guard_single_doc_target(current: &DocRef, proposal_target: &DocRef) -> TargetDecision {
    if current.workspace_id == proposal_target.workspace_id
        && current.rel_path == proposal_target.rel_path
    {
        TargetDecision::Accept
    } else {
        TargetDecision::EscalateCbt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(s: ContextSource, c: &str, cost: u32) -> ContextItem {
        ContextItem { source: s, content: c.into(), cost }
    }
    fn doc(ws: &str, rel: &str) -> DocRef {
        DocRef { workspace_id: ws.into(), rel_path: rel.into(), doc_type: "md".into() }
    }

    // covers: BR-AG-DATA-001
    // 显式 @mention 永不裁剪（即便超预算）；低优先级超预算被裁剪且可追溯（trimmed 非空）。
    #[test]
    fn explicit_mention_always_kept_trim_is_traceable() {
        use ContextSource::*;
        let items = vec![
            item(CurrentDoc, "doc", 100),
            item(Memory, "mem", 50),
            item(ExplicitMention, "@ref", 80),
        ];
        let r = assemble_context(items, 60); // 预算 60
        // @mention 必在 kept（即便 cost 80 > 60）
        assert!(r.kept.iter().any(|i| i.source == ExplicitMention));
        // CurrentDoc(100) 超预算被裁剪，进入 trimmed（可追溯）
        assert!(r.trimmed.iter().any(|i| i.source == CurrentDoc));
        assert!(!r.trimmed.is_empty(), "裁剪须可追溯");
    }

    // covers: BR-AG-DATA-002
    // 提示词分层恒为 系统→工具→上下文→用户；chat 含历史，内联辅助不含历史（骨架差异）。
    #[test]
    fn prompt_skeleton_layer_order_and_kind_variants() {
        let chat = assemble_prompt(TurnKind::Chat);
        assert_eq!(
            chat.layers,
            vec![PromptLayer::System, PromptLayer::Tools, PromptLayer::Context, PromptLayer::User]
        );
        assert!(chat.include_history);
        assert!(!assemble_prompt(TurnKind::InlineAssist).include_history); // 轻量无历史
        assert!(!assemble_prompt(TurnKind::BuildTask).include_history);
        // 所有回合分层顺序一致
        assert_eq!(assemble_prompt(TurnKind::BuildTask).layers[0], PromptLayer::System);
    }

    // covers: BR-AG-DATA-003
    // 单文档回合：target=当前文档 Accept；指向他文档 EscalateCbt（不旁路跨文档产出）。
    #[test]
    fn single_doc_target_accepts_current_escalates_cross_doc() {
        let cur = doc("ws1", "a.md");
        assert_eq!(guard_single_doc_target(&cur, &doc("ws1", "a.md")), TargetDecision::Accept);
        assert_eq!(guard_single_doc_target(&cur, &doc("ws1", "b.md")), TargetDecision::EscalateCbt);
        assert_eq!(guard_single_doc_target(&cur, &doc("ws2", "a.md")), TargetDecision::EscalateCbt);
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

    // PI-AG-09B：历史层按预算保留最近、drop 最旧（确定性、可追溯）。
    #[test]
    fn select_recent_history_keeps_newest_drops_oldest() {
        let h = |r: &str, c: &str| HistTurn { role: r.into(), content: c.into() };
        let turns = vec![h("user", "aaaa"), h("assistant", "bbbb"), h("user", "cc")];
        // 预算 6 字：从最近往前 "cc"(2)+"bbbb"(4)=6 放下，"aaaa" 丢弃。
        let (kept, dropped) = select_recent_history(&turns, 6);
        assert_eq!(dropped, 1);
        assert_eq!(kept.len(), 2);
        assert_eq!(kept[0].content, "bbbb"); // 时序：旧在前
        assert_eq!(kept[1].content, "cc");
        // 预算 0：全丢弃。
        assert_eq!(select_recent_history(&turns, 0).1, 3);
    }

    // PI-AG-09C：意图沿用判定——无历史/新话题=None；修正/选择/回指=对应类型（默认弱化、显式沿用提权）。
    #[test]
    fn intent_carryover_defaults_none_and_detects_carryover() {
        // 无历史 → None（即便有沿用词）。
        assert_eq!(decide_intent_carryover("继续写", false), Carryover::None);
        // 新话题 → None。
        assert_eq!(decide_intent_carryover("帮我写一首关于春天的诗", true), Carryover::None);
        // 修正追加 / 显式选择 / 指代回指。
        assert_eq!(decide_intent_carryover("再短一点", true), Carryover::Refine);
        assert_eq!(decide_intent_carryover("把刚才那段改成正式语气", true), Carryover::Refine);
        assert_eq!(decide_intent_carryover("用第二个方案", true), Carryover::Select);
        assert_eq!(decide_intent_carryover("继续", true), Carryover::ReferenceTarget);
        assert_eq!(decide_intent_carryover("接着上面写", true), Carryover::ReferenceTarget);
    }
}
