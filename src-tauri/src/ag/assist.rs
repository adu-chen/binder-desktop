//! PI-AG-04 · 局部修改/辅助续写机 SM-AG-03 · AG-ASSIST / BR-AG-STATE-003
//!
//! 承接 A-AG-M-S-01 SM-AG-03（autocomplete 与 inline-assist 同机承载，context 区分）：按**节流策略**触发
//! （不得每次按键直发调用）；每次调用同样**经门禁与网关**（BR-AG-VERIFY-001，无例外路径）；候选经 ED
//! 装饰层呈现，**锚失效放弃**本次呈现；**采纳是用户编辑操作**——经 ED 受控接口进入编辑模型走 SM-ED-01→
//! SM-WS-01 正常脏态/保存路径，不构成 AI 直写文件；无订阅/断网/调用失败时**静默不提供**，不打断编辑、
//! 不弹错误。节流参数/成本策略延实现期标定（AG-03 数值部分），本机锁机制语义。

/// SM-AG-03 状态。Accepted/Dismissed 为终态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssistState {
    Idle,
    Throttling,
    Requesting,
    Presenting,
    Accepted,
    Dismissed,
}

/// SM-AG-03 事件。throttlePass 携带门禁是否可用（GatePassed 且有订阅/在线）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssistEvent {
    Trigger,
    ThrottleDrop,
    ThrottlePass { gate_available: bool },
    CandidateOk,
    CandidateUnderflow,
    RequestFail,
    Accept,
    Dismiss,
    AnchorLost,
}

/// 终态判定。
pub fn is_terminal(state: AssistState) -> bool {
    matches!(state, AssistState::Accepted | AssistState::Dismissed)
}

/**
 * @GOV
 * codes:    BR-AG-STATE-003-RB-AG-AG-ASSIST-001
 * type:     RB
 * chain:    AG-ASSIST
 * rules:    BR-AG-STATE-003
 * boundary: in=AssistEvent | out=AssistState(throttledGateGatedSilentOnFailAcceptIsEditAnchorLostDismiss)
 */
/// SM-AG-03 转移引擎（BR-AG-STATE-003）：trigger→节流；throttleDrop 静默回 Idle；throttlePass **仅在
/// 门禁可用**时→Requesting，否则静默回 Idle（无订阅/断网不提供、不打断）；requestFail 静默回 Idle（不弹
/// 错误）；candidateOk→Presenting；candidateUnderflow→Presenting 空态；accept→Accepted（采纳=用户编辑，驱动 SM-ED-01）；dismiss/anchorLost→
/// Dismissed（装饰复位不残留）。返回 `None` = 无合法转移。
pub fn transition(state: AssistState, event: AssistEvent) -> Option<AssistState> {
    use AssistEvent::*;
    use AssistState::*;
    let next = match (state, event) {
        (Idle, Trigger) => Throttling,
        (Throttling, Trigger) => Throttling,
        (Requesting, Trigger) => Throttling,
        (Presenting, Trigger) => Throttling,
        (Throttling, ThrottleDrop) => Idle, // 节流内/输入活跃，静默丢弃
        (
            Throttling,
            ThrottlePass {
                gate_available: true,
            },
        ) => Requesting,
        (
            Throttling,
            ThrottlePass {
                gate_available: false,
            },
        ) => Idle, // 无订阅/断网静默不提供
        (Requesting, CandidateOk) => Presenting,
        (Requesting, CandidateUnderflow) => Presenting,
        (Requesting, RequestFail) => Idle, // 静默（不弹错误、不打断编辑）
        (Presenting, Accept) => Accepted,  // 采纳=用户编辑操作
        (Presenting, Dismiss) => Dismissed,
        (Presenting, AnchorLost) => Dismissed, // 锚失效放弃呈现
        // 其余无合法转移（终态冻结；未过节流不得请求；未呈现不得采纳）。
        _ => return None,
    };
    Some(next)
}

fn clean_candidate_line(line: &str) -> String {
    let trimmed = line
        .trim()
        .trim_matches('"')
        .trim_matches('“')
        .trim_matches('”')
        .trim();
    let without_bullet = trimmed
        .trim_start_matches(|c: char| c == '-' || c == '*' || c == '•' || c.is_whitespace())
        .trim();
    let mut chars = without_bullet.char_indices();
    let mut cut = 0;
    while let Some((idx, ch)) = chars.next() {
        if ch.is_ascii_digit()
            || ch == '.'
            || ch == ')'
            || ch == '）'
            || ch == '、'
            || ch.is_whitespace()
        {
            cut = idx + ch.len_utf8();
            continue;
        }
        break;
    }
    without_bullet[cut..]
        .trim_start_matches(|c: char| c == ':' || c == '：' || c.is_whitespace())
        .trim()
        .to_string()
}

fn normalize_candidates(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        let candidate = value.trim().to_string();
        if candidate.is_empty() || out.iter().any(|seen| seen == &candidate) {
            continue;
        }
        out.push(candidate);
        if out.len() == 3 {
            break;
        }
    }
    out
}

fn candidates_from_json_value(value: &serde_json::Value) -> Option<Vec<String>> {
    let array = if let Some(array) = value.as_array() {
        array
    } else {
        value.get("candidates")?.as_array()?
    };
    let values = array
        .iter()
        .filter_map(|item| item.as_str().map(ToString::to_string));
    Some(normalize_candidates(values))
}

fn parse_candidates_json(text: &str) -> Option<Vec<String>> {
    let value = serde_json::from_str::<serde_json::Value>(text.trim()).ok()?;
    candidates_from_json_value(&value)
}

fn strip_markdown_fences(text: &str) -> String {
    text.lines()
        .filter(|line| !line.trim_start().starts_with("```"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn extract_json_candidate(text: &str) -> Option<&str> {
    let array_start = text.find('[');
    let array_end = text.rfind(']');
    let object_start = text.find('{');
    let object_end = text.rfind('}');
    match (array_start, array_end, object_start, object_end) {
        (Some(s), Some(e), _, _) if s < e => Some(&text[s..=e]),
        (_, _, Some(s), Some(e)) if s < e => Some(&text[s..=e]),
        _ => None,
    }
}

/// 解析辅助续写模型输出为最多 3 个候选。优先接受 JSON 数组/对象，兼容 fenced、多行 JSON 与编号文本。
pub fn parse_autocomplete_candidates(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    if let Some(candidates) = parse_candidates_json(trimmed) {
        return candidates;
    }
    let unfenced = strip_markdown_fences(trimmed);
    if let Some(candidates) = parse_candidates_json(&unfenced) {
        return candidates;
    }
    if let Some(json_slice) = extract_json_candidate(&unfenced) {
        if let Some(candidates) = parse_candidates_json(json_slice) {
            return candidates;
        }
    }

    let mut out = Vec::new();
    for line in unfenced.lines() {
        let trimmed_line = line.trim();
        if trimmed_line.is_empty()
            || trimmed_line == "["
            || trimmed_line == "]"
            || trimmed_line == "{"
            || trimmed_line == "}"
        {
            continue;
        }
        let candidate = clean_candidate_line(trimmed_line.trim_end_matches(','));
        if candidate.is_empty() || out.iter().any(|seen| seen == &candidate) {
            continue;
        }
        out.push(candidate);
        if out.len() == 3 {
            break;
        }
    }
    if out.is_empty() {
        out.push(unfenced.trim().to_string());
    }
    out.truncate(3);
    out
}

#[cfg(test)]
mod tests {
    use super::AssistEvent::*;
    use super::AssistState::*;
    use super::*;

    // covers: BR-AG-STATE-003
    // 节流→门禁可用→请求→呈现→采纳；节流丢弃/门禁不可用/请求失败均静默回 Idle（不打断）。
    #[test]
    fn throttle_gate_request_present_accept_and_silent_paths() {
        assert_eq!(transition(Idle, Trigger), Some(Throttling));
        assert_eq!(
            transition(
                Throttling,
                ThrottlePass {
                    gate_available: true
                }
            ),
            Some(Requesting)
        );
        assert_eq!(transition(Requesting, CandidateOk), Some(Presenting));
        assert_eq!(transition(Presenting, Accept), Some(Accepted));
        // 静默路径：节流丢弃 / 门禁不可用 / 请求失败 → 回 Idle（不打断、不弹错误）
        assert_eq!(transition(Throttling, ThrottleDrop), Some(Idle));
        assert_eq!(
            transition(
                Throttling,
                ThrottlePass {
                    gate_available: false
                }
            ),
            Some(Idle)
        );
        assert_eq!(transition(Requesting, RequestFail), Some(Idle));
        assert_eq!(transition(Requesting, CandidateUnderflow), Some(Presenting));
        // 门禁前置：未过节流不得直接请求
        assert_eq!(transition(Idle, CandidateOk), None);
        // 新请求覆盖旧请求：进入新一轮防抖，旧响应由 requestId/latest-wins 丢弃。
        assert_eq!(transition(Requesting, Trigger), Some(Throttling));
        assert_eq!(transition(Presenting, Trigger), Some(Throttling));
    }

    // covers: BR-AG-STATE-003
    // 呈现期忽略/锚失效 → Dismissed（装饰复位）；终态冻结。
    #[test]
    fn dismiss_anchor_lost_and_terminals_frozen() {
        assert_eq!(transition(Presenting, Dismiss), Some(Dismissed));
        assert_eq!(transition(Presenting, AnchorLost), Some(Dismissed));
        for ev in [Trigger, Accept, Dismiss, CandidateOk] {
            assert_eq!(transition(Accepted, ev), None);
            assert_eq!(transition(Dismissed, ev), None);
        }
        assert!(is_terminal(Accepted) && is_terminal(Dismissed) && !is_terminal(Presenting));
    }

    // covers: BR-AG-STATE-003
    // 辅助续写模型输出收敛为最多 3 个候选，优先 JSON 数组，兼容编号行文本。
    #[test]
    fn parses_autocomplete_candidates_from_json_or_numbered_lines() {
        assert_eq!(
            parse_autocomplete_candidates(r#"["候选一", "候选二", "候选三", "候选四"]"#),
            vec!["候选一", "候选二", "候选三"]
        );
        assert_eq!(
            parse_autocomplete_candidates(
                "1. 继续写第一段\n2）继续写第二段\n- 继续写第三段\n4. 忽略"
            ),
            vec!["继续写第一段", "继续写第二段", "继续写第三段"]
        );
    }

    // covers: BR-AG-STATE-003
    // 真实模型常见格式偏差：fenced / 多行 JSON / 对象包装均应收敛为 3 候选。
    #[test]
    fn parses_autocomplete_candidates_from_fenced_multiline_or_object_json() {
        assert_eq!(
            parse_autocomplete_candidates(
                "```json\n[\n  \"候选一\",\n  \"候选二\",\n  \"候选三\"\n]\n```"
            ),
            vec!["候选一", "候选二", "候选三"]
        );
        assert_eq!(
            parse_autocomplete_candidates(
                "以下是候选：\n{\"candidates\":[\"候选一\",\"候选二\",\"候选三\"]}"
            ),
            vec!["候选一", "候选二", "候选三"]
        );
        assert_eq!(
            parse_autocomplete_candidates(
                "```json\n{\"candidates\":[\"候选一\",\"候选二\",\"候选三\",\"候选四\"]}\n```"
            ),
            vec!["候选一", "候选二", "候选三"]
        );
    }

    // covers: BR-AG-STATE-003
    // 解析器可返回不足 3 条；是否展示由调用层按严格三候选契约判定。
    #[test]
    fn parse_underflow_remains_visible_to_caller() {
        assert_eq!(
            parse_autocomplete_candidates(r#"["候选一", "候选二"]"#),
            vec!["候选一", "候选二"]
        );
    }
}
