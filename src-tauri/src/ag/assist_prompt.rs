//! AG-ASSIST 的轻量请求消息构造；不执行门禁、网络调用或状态转移。

pub(super) struct AssistPromptInput<'a> {
    pub(super) kind: &'a str,
    pub(super) doc_path: Option<&'a str>,
    pub(super) load_seq: Option<i64>,
    pub(super) block_id: Option<&'a str>,
    pub(super) start: Option<i64>,
    pub(super) end: Option<i64>,
    pub(super) instruction: Option<&'a str>,
    pub(super) selection: Option<&'a str>,
    pub(super) prefix: Option<&'a str>,
    pub(super) before: Option<&'a str>,
    pub(super) after: Option<&'a str>,
    pub(super) base_text: Option<&'a str>,
}

/**
 * @GOV
 * codes:    BR-AG-DATA-002-DATA-AG-AG-ASSIST-011
 * type:     DATA
 * chain:    AG-ASSIST
 * rules:    BR-AG-DATA-002
 * boundary: in=AssistPromptInput | out=KindSpecificAssistGatewayMessages(noConversationHistory) | consumer=BR-AG-DATA-002
 */
pub(super) fn build_assist_messages(input: AssistPromptInput<'_>) -> serde_json::Value {
    let system_prompt = if input.kind == "autocomplete" {
        super::prompts::SYSTEM_PROMPT_AUTOCOMPLETE
    } else {
        super::prompts::SYSTEM_PROMPT_INLINE_ASSIST
    };
    let context = format!(
        "【定位上下文】doc={} load_seq={} block_id={} range=[{},{}]\n【锚点前文指纹】{}\n【锚点后文指纹】{}",
        input.doc_path.unwrap_or(""),
        input.load_seq.unwrap_or_default(),
        input.block_id.unwrap_or(""),
        input.start.unwrap_or_default(),
        input.end.unwrap_or_default(),
        input.before.unwrap_or(""),
        input.after.unwrap_or("")
    );
    let user = if input.kind == "inline" {
        let base = input
            .base_text
            .filter(|text| !text.trim().is_empty())
            .unwrap_or_else(|| input.selection.unwrap_or_default());
        if input.selection.unwrap_or_default().trim().is_empty() && base.trim().is_empty() {
            format!(
                "{context}\n【插入指令】{}\n【光标前文本】{}\n请只返回要插入到光标处的正文。",
                input.instruction.unwrap_or_default(),
                input.prefix.unwrap_or_default()
            )
        } else {
            format!(
                "{context}\n【修改指令】{}\n【基底文本】{}\n请只返回修改后的正文。",
                input.instruction.unwrap_or_default(),
                base
            )
        }
    } else {
        format!(
            "{context}\n【续写以下文本，只给续写部分】\n{}\n请生成 3 个不同候选，每个 50-100 字。只返回 JSON 字符串数组，不要解释。",
            input.prefix.unwrap_or_default()
        )
    };
    serde_json::json!([
        { "role": "system", "content": system_prompt },
        { "role": "user", "content": user },
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: BR-AG-DATA-002
    #[test]
    fn inline_message_prefers_refinement_base_text() {
        let messages = build_assist_messages(AssistPromptInput {
            kind: "inline",
            doc_path: Some("a.md"),
            load_seq: Some(2),
            block_id: Some("b1"),
            start: Some(1),
            end: Some(3),
            instruction: Some("改写"),
            selection: Some("原选区"),
            prefix: None,
            before: None,
            after: None,
            base_text: Some("上一轮候选"),
        });
        assert!(messages[1]["content"]
            .as_str()
            .unwrap()
            .contains("上一轮候选"));
    }

    // covers: BR-AG-STATE-003
    #[test]
    fn autocomplete_message_keeps_json_candidate_contract() {
        let messages = build_assist_messages(AssistPromptInput {
            kind: "autocomplete",
            doc_path: Some("a.md"),
            load_seq: None,
            block_id: None,
            start: None,
            end: None,
            instruction: None,
            selection: None,
            prefix: Some("开头"),
            before: None,
            after: None,
            base_text: None,
        });
        let user = messages[1]["content"].as_str().unwrap();
        assert!(user.contains("3 个不同候选"));
        assert!(user.contains("JSON 字符串数组"));
    }
}
