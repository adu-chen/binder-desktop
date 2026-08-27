//! REF-INJECT 注入上下文格式化与选区锚校验（BR-REF-DATA-003 / BR-REF-VERIFY-001）
//!
//! 承接 A-REF-M-T-01：REF 负责解析与供给，AG 负责读取与预算组装（BR-AG-DATA-001）。
//! 本模块为**纯函数层**：不持有 AG 类型（AgentReadContext / OpenDocSnapshot），doc/selection 的
//! 读取由调用方（AG 回合组装）完成后以参数传入；本模块只做校验与格式化，保证 REF 规则逻辑
//! 集中于 REF 模块、可独立测试、不产生 REF→AG 反向依赖（REF-REFERENCE-REPAIR-01 F3/F4/F6）。
//!
//! 锚失效语义（F6）：回合内统一采用**标注注入**——失效不中断回合，输出统一「引用锚已失效」
//! 前缀 + 原始快照，不得静默注入当前内容；verify_anchor 的 Err 契约保留供 ref.* command 路径。

use crate::types::ReferencePayload;

/// 引用目标读取结果（AG 经 AgentReadContext/WS 读取后传入）。
pub enum DocRead<'a> {
    /// 读取时点内容（LogicalState 优先）。
    Loaded { content: &'a str },
    /// 读取失败（code/message 可溯源）。
    Failed { code: &'a str, message: &'a str },
    /// 无 active workspace 等不可读场景。
    NoWorkspace,
}

/// 选区锚校验结果（UTF-16 偏移语义，BR-ED-DATA-004）。
pub enum SelectionCheck {
    /// 锚有效：块存在且字符区间内当前内容。
    Located {
        /// 当前 LogicalState 下区间内容。
        current: String,
        /// 当前内容与发送时原始选区快照是否不一致（不一致时注入当前内容 + 原始快照背景）。
        changed: bool,
    },
    /// 锚失效（块缺失/区间无效/越界/检测不可得），保守不注入当前内容。
    Stale { reason: &'static str },
}

/** @GOV
 * codes:    BR-REF-DATA-003-DATA-REF-REF-INJECT-003
 * type:     DATA
 * chain:    REF-INJECT
 * rules:    BR-REF-DATA-003
 * term_ref: TERM-REF-003
 * boundary: in=referenceSnapshotPayloads | out=injectionContextText(traceableSnapshotNotCurrentDocFactWhenStale)
 */
/// 文本截断（cap 字符 + 后缀标注），供各引用分支共用。
pub fn cap_text(input: &str, cap: usize, note: &str) -> String {
    if input.chars().count() <= cap {
        return input.to_string();
    }
    let head: String = input.chars().take(cap).collect();
    format!("{head}\n{note}")
}

/**
 * @GOV
 * codes:    BR-REF-VERIFY-001-GUARD-REF-REF-INJECT-002
 * type:     GUARD
 * chain:    REF-INJECT
 * rules:    BR-REF-VERIFY-001
 * term_ref: TERM-REF-003
 * boundary: in=selectionPayloadAndCurrentBlockText | out=locatedOrStale(conservativeNoStaleInjection)
 */
/// 选区锚校验（BR-REF-VERIFY-001 / BR-ED-DATA-004）：start/end 按 **UTF-16 code unit** 偏移
/// （与前端 `textBetween` / `String.length` 语义一致），块缺失/区间无效/越界 → Stale 保守处理。
pub fn validate_selection(payload: &ReferencePayload, block_text: Option<&str>) -> SelectionCheck {
    let ReferencePayload::Selection {
        start, end, text, ..
    } = payload
    else {
        return SelectionCheck::Stale {
            reason: "非选区载荷",
        };
    };
    let Some(current_block) = block_text else {
        return SelectionCheck::Stale {
            reason: "当前文档块读取不可得",
        };
    };
    let (Some(start), Some(end)) = (*start, *end) else {
        return SelectionCheck::Stale {
            reason: "选区未提供字符区间",
        };
    };
    if end <= start {
        return SelectionCheck::Stale {
            reason: "选区字符区间无效",
        };
    }
    let utf16: Vec<u16> = current_block.encode_utf16().collect();
    if end > utf16.len() {
        return SelectionCheck::Stale {
            reason: "选区字符区间越界",
        };
    }
    let current = String::from_utf16_lossy(&utf16[start..end]);
    SelectionCheck::Located {
        changed: current != *text,
        current,
    }
}

/**
 * @GOV
 * codes:    BR-REF-DATA-003-DATA-REF-REF-INJECT-002, BR-REF-VERIFY-001-DATA-REF-REF-INJECT-002, BR-SYS-DATA-007-DATA-REF-REF-INJECT-002
 * type:     DATA
 * chain:    REF-INJECT
 * rules:    BR-REF-DATA-003, BR-REF-VERIFY-001, BR-SYS-DATA-007
 * term_ref: TERM-REF-003, TERM-REF-005
 * boundary: in=referencePayloadAndReadOutcome | out=injectionContextBlock(traceableExplicitReferenceHighPriorityExternalContentDataOnly) | consumer=ReferencePayload
 */
/// 注入上下文格式化（BR-REF-DATA-003）：doc/selection 内容与校验结果由调用方（AG）传入；
/// session/memory/knowledge/template/url/long_text 快照直接格式化；失效选区输出统一「引用锚已
/// 失效」前缀（F6），不静默注入当前内容。File 分支按 ExternalContentRef（BR-SYS-DATA-007 /
/// TERM-REF-005）：有只读解析内容 → 一次性 data-only 注入；无 → 保留"未读取内容"（不伪称已读）。
pub fn format_payload_context(
    payload: &ReferencePayload,
    doc: Option<DocRead<'_>>,
    selection: Option<SelectionCheck>,
    cap: usize,
) -> String {
    match payload {
        ReferencePayload::Doc { rel_path, .. } => match doc {
            Some(DocRead::Loaded { content }) => {
                let capped = cap_text(
                    content,
                    cap,
                    "[引用文档较长，已截取前部；需要完整内容可用 read_doc]",
                );
                format!("引用文档「{rel_path}」：\n{capped}")
            }
            Some(DocRead::Failed { code, message }) => {
                format!("引用文档「{rel_path}」读取失败（{code}）：{message}")
            }
            Some(DocRead::NoWorkspace) | None => {
                format!("引用文档「{rel_path}」未读取：当前没有 active workspace")
            }
        },
        ReferencePayload::Selection { rel_path, text, .. } => {
            let stale = |reason: &'static str| {
                let capped = cap_text(text, cap, "[引用选区较长，已截取前部]");
                format!(
                    "[引用锚已失效（{reason}），以下为发送时原始选区快照，不得作为当前文档事实]「{rel_path}」：\n{capped}"
                )
            };
            match selection {
                Some(SelectionCheck::Located {
                    current,
                    changed: false,
                }) => {
                    let capped = cap_text(&current, cap, "[引用选区较长，已截取前部]");
                    format!("引用选区（已校验）「{rel_path}」：\n{capped}")
                }
                Some(SelectionCheck::Located {
                    current,
                    changed: true,
                }) => {
                    let capped = cap_text(&current, cap, "[引用选区较长，已截取前部]");
                    let original = cap_text(text, cap, "[原始选区较长，已截取前部]");
                    format!(
                        "引用选区「{rel_path}」已按当前 LogicalState 重新校验，当前内容：\n{capped}\n\n原始选区快照仅作背景（不得作为当前文档事实）：\n{original}"
                    )
                }
                Some(SelectionCheck::Stale { reason }) => stale(reason),
                None => stale("读取不可得"),
            }
        }
        ReferencePayload::Session {
            title, messages, ..
        } => {
            let body = messages
                .iter()
                .filter(|m| !m.content.trim().is_empty())
                .map(|m| format!("{}: {}", m.role, m.content))
                .collect::<Vec<_>>()
                .join("\n");
            let capped = cap_text(&body, cap, "[引用会话较长，已截取前部]");
            format!("引用会话「{title}」：\n{capped}")
        }
        ReferencePayload::Memory { title, content, .. } => {
            let capped = cap_text(content, cap, "[引用记忆较长，已截取前部]");
            format!(
                "引用记忆「{}」：\n{}",
                title.as_deref().unwrap_or("未命名"),
                capped
            )
        }
        ReferencePayload::Knowledge {
            title,
            content,
            source_ref,
            ..
        } => {
            let capped = cap_text(content, cap, "[引用知识较长，已截取前部]");
            format!(
                "引用知识「{}」{}：\n{}",
                title.as_deref().unwrap_or("未命名"),
                source_ref
                    .as_deref()
                    .map(|s| format!("（来源：{s}）"))
                    .unwrap_or_default(),
                capped
            )
        }
        ReferencePayload::Template { name, content, .. } => {
            let capped = cap_text(content, cap, "[引用模板较长，已截取前部]");
            format!("引用模板「{name}」：\n{capped}")
        }
        ReferencePayload::Url { url, title, .. } => {
            format!("引用外部链接「{}」：{url}", title.as_deref().unwrap_or(url))
        }
        ReferencePayload::LongText {
            text, char_count, ..
        } => {
            let capped = cap_text(text, cap, "[引用长文本较长，已截取前部]");
            format!("引用长文本（{char_count} 字）：\n{capped}")
        }
        ReferencePayload::File {
            name,
            path,
            size,
            read_content,
            ..
        } => {
            let origin = path
                .as_deref()
                .map(|p| format!("（来源：{p}）"))
                .unwrap_or_default();
            let size_label = size.map(|s| format!("，大小 {s} 字节")).unwrap_or_default();
            // ExternalContentRef（BR-SYS-DATA-007 / TERM-REF-005）：有可用只读解析路径的内容进当前回合
            // （一次性 data-only）；None = 仅元数据，不伪称已读。
            match read_content {
                Some(content) => {
                    let capped = cap_text(content, cap, "[引用文件较长，已截取前部]");
                    format!("引用文件「{name}」{origin}{size_label}：\n{capped}")
                }
                None => format!("引用文件「{name}」{origin}{size_label}：仅文件引用，未读取内容"),
            }
        }
    }
}

/**
 * @GOV
 * codes:    BR-REF-DATA-003-DATA-REF-REF-INJECT-004
 * type:     DATA
 * chain:    REF-INJECT
 * rules:    BR-REF-DATA-003
 * term_ref: TERM-REF-003
 * boundary: in=blockRefSelectionAndCappedText | out=blockRefContextText(rangeLabeledSelectionText)
 */
/// 块级引用上下文（AG 兼容路径，PI-AG-09F）：字符区间标注 + 选中内容。
pub fn format_block_ref_context(
    rel_path: &str,
    block_id: &str,
    start: Option<usize>,
    end: Option<usize>,
    capped_text: &str,
) -> String {
    let range_label = match (start, end) {
        (Some(start), Some(end)) if end > start => format!("字符区间 [{start}..{end}]"),
        _ => "字符区间未提供（按该块选中内容理解）".to_string(),
    };
    format!("引用文档「{rel_path}」块 [{block_id}] 的选中内容，{range_label}：\n{capped_text}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selection_payload(start: Option<usize>, end: Option<usize>, text: &str) -> ReferencePayload {
        ReferencePayload::Selection {
            rel_path: "notes/a.md".to_string(),
            block_id: "block-1".to_string(),
            start,
            end,
            text: text.to_string(),
            label: None,
        }
    }

    // covers: BR-REF-DATA-003
    // 块引用：字符区间标注 + 文本保留；无效区间降级标注但不丢文本。
    #[test]
    fn block_ref_context_preserves_selection_range() {
        let out = format_block_ref_context("chapter.md", "block_1", Some(2), Some(8), "引用文本");
        assert!(out.contains("chapter.md"));
        assert!(out.contains("block_1"));
        assert!(out.contains("字符区间 [2..8]"));
        assert!(out.ends_with("引用文本"));
    }

    #[test]
    fn block_ref_context_degrades_invalid_range_without_dropping_text() {
        for (start, end) in [(None, None), (Some(3), None), (Some(8), Some(2))] {
            let out = format_block_ref_context("chapter.md", "block_1", start, end, "引用文本");
            assert!(out.contains("字符区间未提供"));
            assert!(out.ends_with("引用文本"));
        }
    }

    // covers: BR-REF-DATA-003
    // url / long_text / session / memory / knowledge / template 快照格式化。
    #[test]
    fn formats_url_and_long_text_snapshots() {
        let url = ReferencePayload::Url {
            url: "https://example.com".to_string(),
            title: Some("Example".to_string()),
            label: None,
        };
        assert_eq!(
            format_payload_context(&url, None, None, 1000),
            "引用外部链接「Example」：https://example.com"
        );

        let long = ReferencePayload::LongText {
            text: "abcdef".repeat(10),
            char_count: 60,
            label: None,
        };
        let out = format_payload_context(&long, None, None, 20);
        assert!(out.contains("引用长文本（60 字）"));
        assert!(out.contains("[引用长文本较长"));
    }

    #[test]
    fn formats_session_snapshot() {
        let payload = ReferencePayload::Session {
            session_id: "s1".to_string(),
            title: "需求讨论".to_string(),
            messages: vec![
                crate::types::ReferenceSessionMessage {
                    role: "user".to_string(),
                    content: "先做引用".to_string(),
                },
                crate::types::ReferenceSessionMessage {
                    role: "assistant".to_string(),
                    content: "收到".to_string(),
                },
            ],
            label: None,
        };
        let out = format_payload_context(&payload, None, None, 1000);
        assert!(out.contains("引用会话「需求讨论」"));
        assert!(out.contains("user: 先做引用"));
        assert!(out.contains("assistant: 收到"));
    }

    #[test]
    fn formats_asset_snapshots() {
        let memory = ReferencePayload::Memory {
            entry_id: Some("m1".to_string()),
            title: Some("偏好".to_string()),
            content: "用户偏好简洁回答".to_string(),
            label: None,
        };
        let knowledge = ReferencePayload::Knowledge {
            entry_id: Some("k1".to_string()),
            title: Some("术语表".to_string()),
            content: "ReferencePayload 是输入侧引用载荷".to_string(),
            source_ref: Some("kb/ref.md".to_string()),
            label: None,
        };
        let template = ReferencePayload::Template {
            template_id: Some("tpl1".to_string()),
            name: "周报模板".to_string(),
            content: "本周完成：\n下周计划：".to_string(),
            label: None,
        };

        let memory_out = format_payload_context(&memory, None, None, 1000);
        let knowledge_out = format_payload_context(&knowledge, None, None, 1000);
        let template_out = format_payload_context(&template, None, None, 1000);

        assert!(memory_out.contains("引用记忆「偏好」"));
        assert!(memory_out.contains("用户偏好简洁回答"));
        assert!(knowledge_out.contains("引用知识「术语表」（来源：kb/ref.md）"));
        assert!(knowledge_out.contains("ReferencePayload 是输入侧引用载荷"));
        assert!(template_out.contains("引用模板「周报模板」"));
        assert!(template_out.contains("本周完成"));
    }

    // covers: BR-REF-DATA-003
    // file 快照（F5）：仅元数据标注，明确未读取内容。
    #[test]
    fn formats_file_snapshot() {
        let file = ReferencePayload::File {
            name: "report.pdf".to_string(),
            path: Some("Downloads/report.pdf".to_string()),
            size: Some(2048),
            label: None,
            read_content: None,
        };
        let out = format_payload_context(&file, None, None, 1000);
        assert!(out.contains("引用文件「report.pdf」"));
        assert!(out.contains("来源：Downloads/report.pdf"));
        assert!(out.contains("大小 2048 字节"));
        assert!(out.contains("仅文件引用，未读取内容"));
    }

    // covers: BR-SYS-DATA-007
    // ExternalContentRef（TERM-REF-005）：可读只读解析内容 → 一次性 data-only 注入当前回合。
    #[test]
    fn file_payload_injects_external_content_when_read() {
        let read = ReferencePayload::File {
            name: "notes.txt".to_string(),
            path: Some("/tmp/notes.txt".to_string()),
            size: Some(9),
            label: None,
            read_content: Some("外部内容".to_string()),
        };
        let out = format_payload_context(&read, None, None, 1000);
        assert!(out.contains("引用文件「notes.txt」"));
        assert!(out.contains("外部内容"));
        assert!(!out.contains("未读取内容"));
    }

    // covers: BR-SYS-DATA-007
    // 不可读（无 read_content）：保留元数据，不伪称已读。
    #[test]
    fn file_payload_without_content_stays_metadata_only() {
        let meta = ReferencePayload::File {
            name: "photo.png".to_string(),
            path: None,
            size: Some(1024),
            label: None,
            read_content: None,
        };
        let out = format_payload_context(&meta, None, None, 1000);
        assert!(out.contains("仅文件引用，未读取内容"));
        assert!(!out.contains("外部内容"));
    }

    // covers: BR-SYS-DATA-007
    // 内容容量受控（cap 截断），不注入无界文本。
    #[test]
    fn file_external_content_is_capped() {
        let read = ReferencePayload::File {
            name: "big.md".to_string(),
            path: None,
            size: None,
            label: None,
            read_content: Some("x".repeat(5000)),
        };
        let out = format_payload_context(&read, None, None, 100);
        assert!(out.contains("[引用文件较长，已截取前部]"));
        assert!(out.len() < 500);
    }

    // covers: BR-REF-DATA-003
    // doc 引用：Loaded / Failed / 不可读 三形态。
    #[test]
    fn formats_doc_read_outcomes() {
        let doc = ReferencePayload::Doc {
            rel_path: "notes/a.md".to_string(),
            label: None,
        };
        let loaded = format_payload_context(
            &doc,
            Some(DocRead::Loaded {
                content: "文档引用内容",
            }),
            None,
            1000,
        );
        assert!(loaded.contains("引用文档「notes/a.md」"));
        assert!(loaded.contains("文档引用内容"));

        let failed = format_payload_context(
            &doc,
            Some(DocRead::Failed {
                code: "WS_READ_FAILED",
                message: "目标不可达",
            }),
            None,
            1000,
        );
        assert!(failed.contains("读取失败（WS_READ_FAILED）"));

        let none = format_payload_context(&doc, None, None, 1000);
        assert!(none.contains("未读取"));
    }

    // covers: BR-REF-VERIFY-001
    // 失效：无块/无区间/区间无效/越界 → Stale + 统一「引用锚已失效」前缀，不注入当前内容。
    #[test]
    fn stale_selection_emits_explicit_invalid_prefix_without_current_content() {
        let payload = selection_payload(Some(2), Some(6), "选区引用内容");
        for check in [
            validate_selection(&payload, None),
            validate_selection(&payload, Some("短文本")),
            validate_selection(&selection_payload(None, None, "x"), Some("0123456789")),
            validate_selection(
                &selection_payload(Some(8), Some(2), "x"),
                Some("0123456789"),
            ),
            validate_selection(
                &selection_payload(Some(2), Some(99), "x"),
                Some("0123456789"),
            ),
        ] {
            let out = format_payload_context(&payload, None, Some(check), 1000);
            assert!(out.contains("[引用锚已失效"));
            assert!(out.contains("不得作为当前文档事实"));
        }
    }

    // covers: BR-REF-VERIFY-001
    // 有效：区间内当前内容与原始快照一致 → 已校验；不一致 → 当前内容 + 原始快照背景。
    #[test]
    fn located_selection_emits_current_content() {
        let payload = selection_payload(Some(2), Some(6), "2345");
        let out = format_payload_context(
            &payload,
            None,
            Some(validate_selection(&payload, Some("0123456789"))),
            1000,
        );
        assert!(out.contains("引用选区（已校验）"));
        assert!(out.contains("2345"));

        let drifted = selection_payload(Some(2), Some(6), "旧内容");
        let out = format_payload_context(
            &payload,
            None,
            Some(validate_selection(&drifted, Some("0123456789"))),
            1000,
        );
        assert!(out.contains("已按当前 LogicalState 重新校验"));
        assert!(out.contains("原始选区快照仅作背景"));
    }

    // covers: BR-REF-VERIFY-001
    // UTF-16 偏移语义（BR-ED-DATA-004）：emoji 占 2 个 UTF-16 code unit，与前端 String.length 一致。
    #[test]
    fn selection_offsets_follow_utf16_semantics_for_non_bmp() {
        // "a😀b" 的 UTF-16 长度 = 4（a=1, emoji=2, b=1）；按 UTF-16 [1..3] 取到 emoji。
        let block = "a😀b";
        assert_eq!(block.chars().count(), 3); // Unicode scalar 计 3
        let utf16_len = block.encode_utf16().count(); // UTF-16 计 4
        assert_eq!(utf16_len, 4);

        let payload = selection_payload(Some(1), Some(3), "😀");
        let out = format_payload_context(
            &payload,
            None,
            Some(validate_selection(&payload, Some(block))),
            1000,
        );
        assert!(
            out.contains("引用选区（已校验）"),
            "UTF-16 偏移应命中 emoji，不退化 stale"
        );

        // 若误用 Unicode scalar 计数，[1..3] 会取到 "😀b" 之外的内容 → 校验失败，此处验证不退化。
        let stale = format_payload_context(
            &payload,
            None,
            Some(validate_selection(
                &selection_payload(Some(1), Some(4), "😀b"),
                Some(block),
            )),
            1000,
        );
        assert!(!stale.contains("[引用锚已失效"));
    }
}
