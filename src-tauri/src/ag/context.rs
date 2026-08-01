//! PI-AG-01（B-1/B-4）+ PI-AG-SEM-01 · 上下文组装 + 提示词骨架 + 目标边界 · AG-TURN
//! 规则：BR-AG-DATA-001（来源限定/裁剪优先级）/ BR-AG-DATA-002（提示词组装骨架）/ BR-AG-DATA-003（目标集边界）
//!
//! 承接 A-AG-M-T-01：上下文来源限定为 当前文档 / REF @mention / AST-KB 本地资产，限 workspace 内；超预算
//! 裁剪确定性且**显式 @mention 优先保留、裁剪可追溯**（不静默丢弃显式注入，B-1）；提示词按统一分层骨架
//! 系统→工具→上下文→用户组装，三类回合骨架差异（编辑/构建/辅助）；ChangeProposal target 边界按
//! BR-AG-DATA-003 2026-07-04 版：默认当前文档，显式声明集内可跨，新建经 diff 裁决闸门，未声明既有文档拒绝
//! （原「单文档铁律/跨文档升级 CBT」已废止）。预算数值/裁剪算法/提示词文本延实现期。

/// 上下文来源（裁剪保留优先级：BR-AG-DATA-005 分层截断序）。
/// P0 永不截：ExplicitMention、CurrentDoc（当前文档层，与显式引用同级不可裁）。
/// P1 可裁：Memory、Knowledge（补强层，仅作补强不覆盖当前文档与显式引用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextSource {
    ExplicitMention, // 用户显式 @mention（P0 永不裁剪）
    CurrentDoc,      // 当前文档（P0 永不裁剪，与 BR-AG-DATA-005 一致）
    Memory,          // 记忆补强层（P1 可裁）
    Knowledge,       // 知识补强层（P1 可裁）
}

fn priority(s: ContextSource) -> u8 {
    match s {
        ContextSource::ExplicitMention => 0,
        ContextSource::CurrentDoc => 0, // P0 永不截，与 ExplicitMention 同级
        ContextSource::Memory => 1,     // P1 补强层
        ContextSource::Knowledge => 1,  // P1 补强层
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
        // P0 永不截：ExplicitMention 与 CurrentDoc 优先于预算
        let is_p0 =
            it.source == ContextSource::ExplicitMention || it.source == ContextSource::CurrentDoc;
        if is_p0 || used.saturating_add(it.cost) <= budget {
            used = used.saturating_add(it.cost);
            kept.push(it);
        } else {
            trimmed.push(it);
        }
    }
    AssembledContext { kept, trimmed }
}

/// 回合类型（骨架差异）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnKind {
    Chat,         // 对话历史 + 修改建议导向 diff
    BuildTask,    // 任务计划上下文 + 单文件产出导向
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
/// 提示词骨架组装（BR-AG-DATA-002）：分层恒为 系统→工具→上下文→用户；编辑回合（TurnKind::Chat）含对话历史，辅助回合
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
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistTurn {
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
    Refine,          // 修正追加（把刚才那段改成…）
    Select,          // 显式选择（用第二个 / 按刚才说的）
    ReferenceTarget, // 指代回指 / 续写（继续 / 接着 / 它 / 那个 / 上面）
}

/// 按当前文档归属分流后的历史。same_doc 可作为同文档 P1 背景；other_doc 只能在显式沿用时弱注入。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedHistory {
    pub same_doc: Vec<HistTurn>,
    pub other_doc: Vec<HistTurn>,
    pub dropped_cross_doc: usize,
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn mentions_ordinal_choice(text: &str) -> bool {
    [
        "第一个方案",
        "第二个方案",
        "第三个方案",
        "第一个版本",
        "第二个版本",
        "第三个版本",
        "第一个回答",
        "第二个回答",
        "第三个回答",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

pub fn prompt_has_explicit_history_reference(prompt: &str) -> bool {
    let p = prompt.trim();
    contains_any(
        p,
        &[
            "刚才",
            "刚刚",
            "上面",
            "上一轮",
            "上轮",
            "前面",
            "之前那",
            "你刚写",
            "你刚才",
            "你上面",
            "按刚才",
            "按上面",
            "用刚才",
            "用上面",
            "继续刚才",
            "接着上面",
        ],
    ) || mentions_ordinal_choice(p)
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
    if !prompt_has_explicit_history_reference(p) {
        return Carryover::None;
    }
    // 显式选择（沿用上一轮某个产出/方案）。
    if contains_any(p, &["按刚才", "按上面", "用刚才", "用上面", "就用"])
        || mentions_ordinal_choice(p)
    {
        return Carryover::Select;
    }
    // 修正追加（操作对象必须被显式锚定为历史产出）。
    if contains_any(
        p,
        &[
            "把刚才",
            "把刚刚",
            "把上面",
            "把上一",
            "刚才那",
            "刚刚那",
            "上面那",
            "上一轮",
        ],
    ) && contains_any(
        p,
        &[
            "改成",
            "换成",
            "短一点",
            "长一点",
            "再写",
            "再短",
            "再长",
            "正式一点",
            "重写",
            "润色",
            "优化",
        ],
    ) {
        return Carryover::Refine;
    }
    // 指代回指 / 续写（延续上一轮目标）。
    Carryover::ReferenceTarget
}

/**
 * @GOV
 * codes:    BR-AG-DATA-004-DATA-AG-AG-TURN-005
 * type:     DATA
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-004
 * term_ref: TERM-AG-005
 * boundary: in=currentDocPathAndHistTurns | out=ScopedHistory(sameDocHistorySeparateCrossDocWeakOnlyWhenExplicit)
 */
/// 历史归属分流（BR-AG-DATA-004）：有当前文档时，只有同 relPath 历史可作为同文档 P1 背景；
/// 跨文档或无 relPath 的历史默认丢弃，只有显式沿用历史时才进入 other_doc，且调用方必须弱注入。
pub fn scope_history_for_current_doc(
    turns: &[HistTurn],
    current_rel: Option<&str>,
    carryover: Carryover,
) -> ScopedHistory {
    let Some(current) = current_rel.filter(|p| !p.trim().is_empty()) else {
        return ScopedHistory {
            same_doc: turns.to_vec(),
            other_doc: Vec::new(),
            dropped_cross_doc: 0,
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
    let dropped_cross_doc = if carryover == Carryover::None {
        other_doc_all.len()
    } else {
        0
    };
    let other_doc = if carryover == Carryover::None {
        Vec::new()
    } else {
        other_doc_all
    };
    ScopedHistory {
        same_doc,
        other_doc,
        dropped_cross_doc,
    }
}

/// 目标边界裁决结果（BR-AG-DATA-003，2026-07-04 版）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetDecision {
    /// 当前文档 / 显式声明集内 / 新建文档（新建的人类闸门=diff 裁决卡，接受方落盘）。
    Accept,
    /// 未声明的**既有**文档：拒绝（原因回注模型，提示请用户显式 @引用该文档）。
    RejectUndeclared,
}

/**
 * @GOV
 * codes:    BR-AG-DATA-003-GUARD-AG-AG-TURN-001
 * type:     GUARD
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-003
 * boundary: in=declaredTargetSetAndProposalTarget | out=TargetDecision(acceptCurrentDeclaredOrNewDocRejectUndeclaredExisting)
 */
/// 回合目标集守卫（BR-AG-DATA-003，2026-07-04 人类裁决版，取代单文档铁律）：ChangeProposal target
/// 须属回合声明的目标集——当前文档 ∪ 用户显式指向集（@mention / 块引用所在文档）；target 为**不存在
/// 的路径**视为新建文档放行（仍经 DE diff 裁决、人类接受方落盘，X-FORBID-006 闸门即「明确要求」的
/// 确认点）；指向**未声明的既有文档**一律拒绝，防模型游走改无关文件。workspace 限界（X-FORBID-004）
/// 由调用方 resolve_in_workspace 前置保证。与 DE 侧 `ensure_target_declared` 双侧把守。
pub fn guard_target_set(
    current_rel: Option<&str>,
    declared_rels: &[String],
    target_rel: &str,
    target_exists: bool,
) -> TargetDecision {
    if current_rel == Some(target_rel) {
        return TargetDecision::Accept;
    }
    if declared_rels.iter().any(|d| d == target_rel) {
        return TargetDecision::Accept;
    }
    if !target_exists {
        return TargetDecision::Accept; // 新建：diff 裁决卡为人类确认闸门
    }
    TargetDecision::RejectUndeclared
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(s: ContextSource, c: &str, cost: u32) -> ContextItem {
        ContextItem {
            source: s,
            content: c.into(),
            cost,
        }
    }

    // covers: BR-AG-DATA-001
    // P0 永不截：ExplicitMention 与 CurrentDoc 在预算不足时仍保留；P1 补强层可裁且可追溯。
    #[test]
    fn explicit_mention_and_current_doc_always_kept_trim_is_traceable() {
        use ContextSource::*;
        let items = vec![
            item(CurrentDoc, "doc", 100),
            item(Memory, "mem", 50),
            item(ExplicitMention, "@ref", 80),
        ];
        let r = assemble_context(items, 60); // 预算 60
                                             // @mention 与 CurrentDoc 必在 kept（P0 永不截）
        assert!(r.kept.iter().any(|i| i.source == ExplicitMention));
        assert!(r.kept.iter().any(|i| i.source == CurrentDoc));
        // Memory(50) 超预算被裁剪，进入 trimmed（可追溯）
        assert!(r.trimmed.iter().any(|i| i.source == Memory));
        assert!(!r.trimmed.is_empty(), "裁剪须可追溯");
    }

    // covers: BR-AG-DATA-002
    // 提示词分层恒为 系统→工具→上下文→用户；编辑回合含历史，辅助回合不含历史（骨架差异）。
    #[test]
    fn prompt_skeleton_layer_order_and_kind_variants() {
        let chat = assemble_prompt(TurnKind::Chat);
        assert_eq!(
            chat.layers,
            vec![
                PromptLayer::System,
                PromptLayer::Tools,
                PromptLayer::Context,
                PromptLayer::User
            ]
        );
        assert!(chat.include_history);
        assert!(!assemble_prompt(TurnKind::InlineAssist).include_history); // 轻量无历史
        assert!(!assemble_prompt(TurnKind::BuildTask).include_history);
        // 所有回合分层顺序一致
        assert_eq!(
            assemble_prompt(TurnKind::BuildTask).layers[0],
            PromptLayer::System
        );
    }

    // covers: BR-AG-DATA-003
    // 目标集边界（2026-07-04 版）：当前文档/声明集内/新建 → Accept；未声明既有文档 → RejectUndeclared。
    #[test]
    fn target_set_accepts_current_declared_new_rejects_undeclared_existing() {
        let declared = vec!["ref.md".to_string()];
        // ① 当前文档。
        assert_eq!(
            guard_target_set(Some("a.md"), &declared, "a.md", true),
            TargetDecision::Accept
        );
        // ② 用户显式声明集内（@mention / 块引用）。
        assert_eq!(
            guard_target_set(Some("a.md"), &declared, "ref.md", true),
            TargetDecision::Accept
        );
        // ③ 新建文档（不存在的路径，diff 裁决卡为人类闸门）。
        assert_eq!(
            guard_target_set(Some("a.md"), &declared, "new.md", false),
            TargetDecision::Accept
        );
        // ④ 未声明的既有文档：拒绝（防模型游走改无关文件）。
        assert_eq!(
            guard_target_set(Some("a.md"), &declared, "other.md", true),
            TargetDecision::RejectUndeclared
        );
        // ⑤ 无当前文档（未打开编辑器）时，声明集与新建仍可用，未声明既有仍拒。
        assert_eq!(
            guard_target_set(None, &declared, "ref.md", true),
            TargetDecision::Accept
        );
        assert_eq!(
            guard_target_set(None, &[], "other.md", true),
            TargetDecision::RejectUndeclared
        );
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
            role: r.into(),
            content: c.into(),
            rel_path: None,
            turn_id: None,
        };
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

    // covers: BR-AG-DATA-004
    // PI-AG-09C：意图沿用判定——无历史/新话题=None；修正/选择/回指=对应类型（默认弱化、显式沿用提权）。
    #[test]
    fn intent_carryover_defaults_none_and_detects_carryover() {
        // 无历史 → None（即便有沿用词）。
        assert_eq!(decide_intent_carryover("继续写", false), Carryover::None);
        // 新话题 → None。
        assert_eq!(
            decide_intent_carryover("帮我写一首关于春天的诗", true),
            Carryover::None
        );
        // 泛编辑词没有显式历史回指时，不提权历史。
        assert_eq!(decide_intent_carryover("再短一点", true), Carryover::None);
        assert_eq!(
            decide_intent_carryover("重写当前文档", true),
            Carryover::None
        );
        assert_eq!(
            decide_intent_carryover("把这篇优化一下", true),
            Carryover::None
        );
        // 修正追加 / 显式选择 / 指代回指。
        assert_eq!(
            decide_intent_carryover("把刚才那段改成正式语气", true),
            Carryover::Refine
        );
        assert_eq!(
            decide_intent_carryover("用第二个方案", true),
            Carryover::Select
        );
        assert_eq!(
            decide_intent_carryover("继续刚才的方向", true),
            Carryover::ReferenceTarget
        );
        assert_eq!(
            decide_intent_carryover("接着上面写", true),
            Carryover::ReferenceTarget
        );
        // 泛指当前任务/当前文档的词不应单独提权历史意图。
        assert_eq!(
            decide_intent_carryover("接下来优化这个文档", true),
            Carryover::None
        );
        assert_eq!(
            decide_intent_carryover("把它整理成三段", true),
            Carryover::None
        );
    }

    // covers: BR-AG-DATA-004
    // 历史归属：当前文档只接收同 relPath 历史；跨文档历史默认丢弃，显式沿用时仅弱注入。
    #[test]
    fn scope_history_separates_same_doc_from_cross_doc() {
        let h = |rel: Option<&str>, c: &str| HistTurn {
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

        let none = scope_history_for_current_doc(&turns, Some("b.md"), Carryover::None);
        assert_eq!(none.same_doc.len(), 1);
        assert_eq!(none.same_doc[0].content, "B 历史");
        assert!(none.other_doc.is_empty());
        assert_eq!(none.dropped_cross_doc, 2);

        let explicit = scope_history_for_current_doc(&turns, Some("b.md"), Carryover::Refine);
        assert_eq!(explicit.same_doc.len(), 1);
        assert_eq!(explicit.other_doc.len(), 2);
        assert_eq!(explicit.dropped_cross_doc, 0);

        let no_current = scope_history_for_current_doc(&turns, None, Carryover::None);
        assert_eq!(no_current.same_doc.len(), 3);
        assert!(no_current.other_doc.is_empty());
    }
}
