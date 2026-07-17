//! PI-AG-08F · 文档分块（块列表注入 + 块级 propose_edit 重建）。
//!
//! 两种文档形态：
//! - **text/markdown**：按行（`\n`）切块；与 ED textToDoc 的“每行一个 Block”一致，重建 = 替块后 join。
//! - **docx/html**（editFormat=html）：按 `data-block-id` 块级元素切块（与 ED blockId / DOC 兜底同源思路，
//!   A-DOC-M-T-01 BR-DOC-DATA-003），展示 strip 后文本；重建 = **原串 in-place 替换该块 inner（非破坏，
//!   保留其余 HTML 与未捕获内容）**。
//!
//! dev 级轻量字符串扫描（无 HTML parser 依赖），适配 TipTap 平铺块元素（p/h/li/blockquote 等带 block-id）；
//! 嵌套同名块、畸形 HTML 不保证——真实 ED blockId 锚定位（对齐 LocateResult）后续。

use crate::ipc::IpcError;
use std::path::Path;

/// 一个文档块。`block_id` = HTML 块的 data-block-id（稳定锚；text 块 None）；
/// `inner_span` = HTML 块 inner 在原串中的字节区间（in-place 重建用；text 块 None）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocBlock {
    pub block_id: Option<String>,
    pub text: String,
    pub inner_span: Option<(usize, usize)>,
}

/// AgentEdit 统一可消费块投影：`block_index` 是当前快照内 0 起块号；`block_id` 是 HTML/BDocx
/// 的稳定锚，text 文档可为空。marks/block_attrs 仅 HTML 快照有语义。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct AgentEditableBlock {
    pub block_id: Option<String>,
    pub block_index: usize,
    pub text: String,
    pub marks: Vec<serde_json::Value>,
    pub block_attrs: serde_json::Value,
}

/// AgentEdit 统一内容快照。BDocx 下游一律消费此编辑模型快照，不直接读取 docx 二进制。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct AgentEditableSnapshot {
    pub rel_path: String,
    pub editable_kind: String,
    pub source: String,
    pub content_format: String,
    pub content: String,
    pub baseline: Option<u64>,
    pub blocks: Vec<AgentEditableBlock>,
}

fn doc_type_of_rel(rel_path: &str) -> String {
    Path::new(rel_path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_else(|| "bin".to_string())
}

fn is_docx_rel_path(rel_path: &str) -> bool {
    matches!(doc_type_of_rel(rel_path).as_str(), "docx" | "doc")
}

fn editable_kind(rel_path: &str, content_is_html: bool) -> String {
    if is_docx_rel_path(rel_path) {
        "bdocx".to_string()
    } else if content_is_html {
        "html".to_string()
    } else {
        "text".to_string()
    }
}

/// 是否按 HTML 处理（editFormat=html 或内容以 `<` 开头）。
pub fn is_html(content: &str, edit_format: Option<&str>) -> bool {
    edit_format == Some("html") || content.trim_start().starts_with('<')
}

/// 解析文档块。
pub fn parse_blocks(content: &str, is_html: bool) -> Vec<DocBlock> {
    if is_html {
        parse_html_blocks(content)
    } else {
        content
            .split('\n')
            .map(|s| DocBlock {
                block_id: None,
                text: s.to_string(),
                inner_span: None,
            })
            .collect()
    }
}

/// 构建统一块投影。当前编辑器上下文、`ag.list_blocks`、只读 tool 与 `propose_edit`
/// 必须复用该函数，避免同一 BDocx 在不同入口形成不同块列表。
pub fn project_blocks(content: &str, is_html: bool) -> Vec<AgentEditableBlock> {
    parse_blocks(content, is_html)
        .into_iter()
        .enumerate()
        .map(|(block_index, b)| {
            let (marks, block_attrs) = match b.inner_span {
                Some((s, e)) => {
                    let (_, spans) = extract_marks(&content[s..e]);
                    let marks = spans
                        .into_iter()
                        .map(|m| {
                            let mut o = serde_json::json!({
                                "type": m.mark_type,
                                "range": [m.range.0, m.range.1],
                            });
                            if let Some(attrs) = m.attrs {
                                o["attrs"] = attrs;
                            }
                            o
                        })
                        .collect();
                    let open = content[..s]
                        .rfind('<')
                        .map(|lt| &content[lt..s])
                        .unwrap_or("");
                    (marks, extract_block_attrs(open))
                }
                None => (Vec::new(), serde_json::json!({})),
            };
            AgentEditableBlock {
                block_id: b.block_id,
                block_index,
                text: b.text,
                marks,
                block_attrs,
            }
        })
        .collect()
}

/**
 * @GOV
 * codes:    BR-AG-DATA-001-QUERY-AG-AG-TURN-010, BR-DOC-DATA-003-QUERY-AG-AG-TURN-010
 * type:     QUERY
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-001, BR-DOC-DATA-003
 * term_ref: TERM-DOC-003, TERM-ED-001
 * boundary: in=workspaceDocOrActiveEditorSnapshot | out=AgentEditableSnapshot(singleBDocxReadModelWithBlocks)
 */
/// AgentEdit 统一快照读取：当前编辑器实时内容优先；非当前 BDocx 只经 DOC 转换链读取 HTML 编辑模型；
/// 普通文本文件走 WS 文本读取。AG/REF/DE 下游不得绕过该快照另读 BDocx 二进制。
pub fn read_editable_snapshot(
    root: &Path,
    rel_path: &str,
    abs: &Path,
    active_content: Option<&str>,
    active_format: Option<&str>,
) -> Result<AgentEditableSnapshot, IpcError> {
    let (content, source, baseline, format_hint) = if let Some(content) = active_content {
        (
            content.to_string(),
            "active_editor".to_string(),
            None,
            active_format.map(str::to_string),
        )
    } else if is_docx_rel_path(rel_path) {
        if !crate::workspace::listing::is_bdocx(root, rel_path) {
            return Err(IpcError::new(
                "DOC_NOT_BDOCX",
                "原生 docx 不能作为 AgentEdit 编辑模型读取，请先创建 b-docx",
                false,
                false,
            ));
        }
        let bytes = std::fs::read(abs).map_err(|e| {
            IpcError::new(
                "DOC_READ_FAILED",
                &format!("读 b-docx 失败: {e}"),
                true,
                false,
            )
        })?;
        let baseline = crate::workspace::save::content_hash(&bytes);
        let work_dir = std::env::temp_dir()
            .join("binder_ag_snapshot")
            .join(format!("{baseline:x}"));
        (
            crate::doc::edit_chain::docx_to_html(abs, &work_dir)?,
            "doc_convert".to_string(),
            Some(baseline),
            Some("html".to_string()),
        )
    } else {
        let read = crate::workspace::save::read_doc(abs)?;
        (
            read.content,
            "text_file".to_string(),
            Some(read.baseline),
            None,
        )
    };
    let content_is_html = is_html(&content, format_hint.as_deref());
    let content_format = if content_is_html { "html" } else { "text" }.to_string();
    let blocks = project_blocks(&content, content_is_html);
    Ok(AgentEditableSnapshot {
        rel_path: rel_path.to_string(),
        editable_kind: editable_kind(rel_path, content_is_html),
        source,
        content_format,
        content,
        baseline,
        blocks,
    })
}

/// 仅取各块展示文本（块列表注入用）。
pub fn block_texts(content: &str, is_html: bool) -> Vec<String> {
    parse_blocks(content, is_html)
        .into_iter()
        .map(|b| b.text)
        .collect()
}

/// 把第 `index` 块替换为 `new_text`，返回整篇新内容。
/// HTML：原串 in-place 替换该块 inner（非破坏）；text：替块后 join("\n")。越界返回 Err。
pub fn replace_block(
    content: &str,
    is_html: bool,
    index: usize,
    new_text: &str,
) -> Result<String, String> {
    let blocks = parse_blocks(content, is_html);
    if index >= blocks.len() {
        return Err(format!(
            "block_index {index} 越界（当前共 {} 块）",
            blocks.len()
        ));
    }
    if is_html {
        let (s, e) = blocks[index]
            .inner_span
            .ok_or_else(|| "HTML 块缺 inner 区间".to_string())?;
        Ok(format!(
            "{}{}{}",
            &content[..s],
            html_escape(new_text),
            &content[e..]
        ))
    } else {
        let mut texts: Vec<String> = blocks.into_iter().map(|b| b.text).collect();
        texts[index] = new_text.to_string();
        Ok(texts.join("\n"))
    }
}

fn strip_tags(s: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// 扫描带 data-block-id 的块级元素，记录 inner 字节区间 + strip 文本。
fn parse_html_blocks(html: &str) -> Vec<DocBlock> {
    let mut blocks = Vec::new();
    let mut search = 0usize;
    while let Some(rel) = html[search..].find("data-block-id") {
        let attr_pos = search + rel;
        // 元素 open tag 起点 = attr 前最近的 '<'。
        let Some(open_lt) = html[..attr_pos].rfind('<') else {
            search = attr_pos + "data-block-id".len();
            continue;
        };
        // tag 名（'<' 后到空白/'>'/'/' 之前）。
        let name_start = open_lt + 1;
        let name_end = html[name_start..]
            .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
            .map(|p| name_start + p)
            .unwrap_or(name_start);
        let tag = &html[name_start..name_end];
        if tag.is_empty() {
            search = attr_pos + "data-block-id".len();
            continue;
        }
        // open tag 结束 '>'。
        let Some(rel_gt) = html[attr_pos..].find('>') else {
            break;
        };
        let inner_start = attr_pos + rel_gt + 1;
        // 匹配最近的 </tag>。
        let close = format!("</{tag}>");
        let Some(rel_close) = html[inner_start..].find(&close) else {
            search = inner_start;
            continue;
        };
        let inner_end = inner_start + rel_close;
        let inner = &html[inner_start..inner_end];
        // 提取 data-block-id 的值（attr 后第一对引号内）。
        let block_id = {
            let after = attr_pos + "data-block-id".len();
            html[after..inner_start].find('"').and_then(|q1| {
                let vstart = after + q1 + 1;
                html[vstart..inner_start]
                    .find('"')
                    .map(|q2| html[vstart..vstart + q2].to_string())
            })
        };
        blocks.push(DocBlock {
            block_id,
            text: strip_tags(inner).trim().to_string(),
            inner_span: Some((inner_start, inner_end)),
        });
        search = inner_end + close.len();
    }
    blocks
}

/// 按 **data-block-id 稳定锚** 替换块 inner（HTML）。id 未找到 → Err（锚失效，文档已变化），
/// 不产错位 diff（对齐 A-DOC-M-T-01 blockId 锚 / BR-DE-DATA-001 锚失效不误改）。
pub fn replace_block_by_id(
    content: &str,
    block_id: &str,
    new_text: &str,
) -> Result<String, String> {
    let blk = parse_html_blocks(content)
        .into_iter()
        .find(|b| b.block_id.as_deref() == Some(block_id))
        .ok_or_else(|| {
            format!(
                "锚失效：未找到 data-block-id={block_id} 的块（文档可能已变化，请重新获取块列表）"
            )
        })?;
    let (s, e) = blk
        .inner_span
        .ok_or_else(|| "块缺 inner 区间".to_string())?;
    Ok(format!(
        "{}{}{}",
        &content[..s],
        html_escape(new_text),
        &content[e..]
    ))
}

// —— 样式投影（PI-ED-11 后端件 · A-AG-I-P-01 §4.1 / schema 单一来源 A-ED-M-T-01 §5.1） ——

/// 字符 mark 段：range 在**块内纯文本平面**（与 ED 字符锚同基准，BR-ED-DATA-004）。
#[derive(Debug, Clone, PartialEq)]
pub struct MarkSpan {
    pub mark_type: String,
    pub range: (usize, usize),
    pub attrs: Option<serde_json::Value>,
}

/// 内联标签 → mark 类型映射（TipTap 序列化 HTML 的规整子集；span 依 style 判 textStyle）。
fn mark_of_tag(tag: &str, attrs_str: &str) -> Option<(String, Option<serde_json::Value>)> {
    match tag {
        "strong" | "b" => Some(("bold".into(), None)),
        "em" | "i" => Some(("italic".into(), None)),
        "u" => Some(("underline".into(), None)),
        "s" | "del" | "strike" => Some(("strike".into(), None)),
        "mark" => Some(("highlight".into(), None)),
        "sub" => Some(("subscript".into(), None)),
        "sup" => Some(("superscript".into(), None)),
        "code" => Some(("code".into(), None)),
        "a" => {
            let href = attr_value(attrs_str, "href");
            Some((
                "link".into(),
                href.map(|h| serde_json::json!({ "href": h })),
            ))
        }
        "span" => {
            let style = attr_value(attrs_str, "style")?;
            let mut o = serde_json::Map::new();
            for kv in style.split(';') {
                let (k, v) = kv.split_once(':')?;
                match k.trim() {
                    "color" => {
                        o.insert("color".into(), v.trim().into());
                    }
                    "font-size" => {
                        o.insert("fontSize".into(), v.trim().into());
                    }
                    "font-family" => {
                        o.insert("fontFamily".into(), v.trim().into());
                    }
                    _ => {}
                }
            }
            if o.is_empty() {
                None
            } else {
                Some(("textStyle".into(), Some(serde_json::Value::Object(o))))
            }
        }
        _ => None,
    }
}

/// 取属性串中 `name="…"` 的值（dev 级扫描）。
fn attr_value(attrs_str: &str, name: &str) -> Option<String> {
    let pat = format!("{name}=\"");
    let i = attrs_str.find(&pat)? + pat.len();
    let j = attrs_str[i..].find('"')?;
    Some(attrs_str[i..i + j].to_string())
}

/// 有限实体解码（与 html_escape 逆配 + 常用集）。
fn decode_entity(ent: &str) -> Option<char> {
    match ent {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "#39" | "apos" => Some('\''),
        "nbsp" => Some('\u{a0}'),
        _ => None,
    }
}

/**
 * @GOV
 * codes:    BR-ED-DATA-005-QUERY-ED-ED-EDIT-009
 * type:     QUERY
 * chain:    ED-EDIT
 * rules:    BR-ED-DATA-005
 * term_ref: TERM-ED-006, TERM-ED-004
 * boundary: in=blockInnerHtml | out=plainTextWithMarkSpans(offsetsOnInBlockPlainTextPlane)
 */
/// 样式投影提取（BR-ED-DATA-005 读侧后端承载）：块 inner HTML → 纯文本 + mark 段（range 在块内
/// 纯文本平面，与 ED 字符锚同基准）。栈式扫描 TipTap 规整 HTML（嵌套标签配对、有限实体解码、
/// void 标签 1 字符占位对齐叶子 pos）；本模块 dev 级定位（畸形 HTML 不保证）已在模块头声明。
pub fn extract_marks(inner: &str) -> (String, Vec<MarkSpan>) {
    let mut text = String::new();
    let mut offset = 0usize; // 字符偏移（非字节）
    let mut spans: Vec<MarkSpan> = Vec::new();
    // 开标签栈：(tag, mark_type/attrs 可空, 起始偏移)
    let mut stack: Vec<(String, Option<(String, Option<serde_json::Value>)>, usize)> = Vec::new();
    let bytes: Vec<char> = inner.chars().collect();
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i];
        if c == '<' {
            // 收集到 '>'。
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() && bytes[j] != '>' {
                j += 1;
            }
            let tag_body: String = bytes[start..j.min(bytes.len())].iter().collect();
            i = (j + 1).min(bytes.len());
            let closing = tag_body.starts_with('/');
            let self_closing = tag_body.ends_with('/');
            let name_src = tag_body.trim_start_matches('/');
            let name_end = name_src
                .find(|ch: char| ch.is_whitespace() || ch == '/')
                .unwrap_or(name_src.len());
            let name = name_src[..name_end].to_ascii_lowercase();
            if closing {
                // 弹出最近同名开标签，产出 mark 段（空区间不产）。
                if let Some(pos) = stack.iter().rposition(|(t, _, _)| *t == name) {
                    let (_, mark, from) = stack.remove(pos);
                    if let Some((mt, attrs)) = mark {
                        if offset > from {
                            spans.push(MarkSpan {
                                mark_type: mt,
                                range: (from, offset),
                                attrs,
                            });
                        }
                    }
                }
            } else if matches!(name.as_str(), "br" | "img" | "hr") || self_closing {
                // void/自闭合 → 1 字符占位（对齐 ED 叶子内联 pos 语义）。
                text.push('\u{fffc}');
                offset += 1;
            } else {
                let attrs_str = &name_src[name_end..];
                stack.push((name.clone(), mark_of_tag(&name, attrs_str), offset));
            }
        } else if c == '&' {
            // 实体：& 到 ';'（最长 6 字符），解码失败原样输出 '&'。
            let mut j = i + 1;
            while j < bytes.len() && j - i <= 6 && bytes[j] != ';' {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == ';' {
                let ent: String = bytes[i + 1..j].iter().collect();
                if let Some(ch) = decode_entity(&ent) {
                    text.push(ch);
                    offset += 1;
                    i = j + 1;
                    continue;
                }
            }
            text.push('&');
            offset += 1;
            i += 1;
        } else {
            text.push(c);
            offset += 1;
            i += 1;
        }
    }
    (text, spans)
}

/// 块开标签的呈现属性投影（blockAttrs）：style 中 text-align（TipTap TextAlign 序列化形态）。
pub fn extract_block_attrs(open_tag: &str) -> serde_json::Value {
    let mut o = serde_json::Map::new();
    if let Some(style) = attr_value(open_tag, "style") {
        for kv in style.split(';') {
            if let Some((k, v)) = kv.split_once(':') {
                if k.trim() == "text-align" {
                    o.insert("textAlign".into(), v.trim().into());
                }
            }
        }
    }
    serde_json::Value::Object(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_blocks_split_by_editor_lines_including_empty_blocks() {
        let b = block_texts("第一行\n\n第三行", false);
        assert_eq!(b, vec!["第一行", "", "第三行"]);
    }

    #[test]
    fn text_replace_block_preserves_editor_line_granularity() {
        let out = replace_block("a\n\nc", false, 2, "C").unwrap();
        assert_eq!(out, "a\n\nC");
    }

    #[test]
    fn html_blocks_parsed_by_block_id_with_stripped_text() {
        let html = r#"<p data-block-id="b1">你好<strong>世界</strong></p><h1 data-block-id="b2">标题</h1>"#;
        assert!(is_html(html, Some("html")));
        let texts = block_texts(html, true);
        assert_eq!(texts, vec!["你好世界", "标题"]);
    }

    #[test]
    fn project_blocks_uses_single_block_shape_with_marks() {
        let html =
            r#"<p data-block-id="b1" style="text-align: center">你好<strong>世界</strong></p>"#;
        let blocks = project_blocks(html, true);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_id.as_deref(), Some("b1"));
        assert_eq!(blocks[0].block_index, 0);
        assert_eq!(blocks[0].text, "你好世界");
        assert_eq!(blocks[0].marks[0]["type"], "bold");
        assert_eq!(blocks[0].block_attrs["textAlign"], "center");
    }

    #[test]
    fn html_replace_block_is_inplace_nonlossy() {
        let html = r#"<p data-block-id="b1">旧</p><h1 data-block-id="b2">标题</h1>"#;
        let out = replace_block(html, true, 0, "新内容").unwrap();
        // 仅 b1 inner 被替换，b2 与标签结构原样保留。
        assert_eq!(
            out,
            r#"<p data-block-id="b1">新内容</p><h1 data-block-id="b2">标题</h1>"#
        );
    }

    #[test]
    fn html_replace_escapes_special_chars() {
        let html = r#"<p data-block-id="b1">x</p>"#;
        let out = replace_block(html, true, 0, "a<b>&c").unwrap();
        assert_eq!(out, r#"<p data-block-id="b1">a&lt;b&gt;&amp;c</p>"#);
    }

    // covers: BR-ED-DATA-005
    // 样式投影提取：range 在块内纯文本平面（与字符锚同基准）；嵌套配对；span style→textStyle attrs；
    // 实体解码偏移正确；void 标签 1 字符占位。
    #[test]
    fn extract_marks_offsets_on_plain_text_plane() {
        let (text, spans) =
            extract_marks(r#"普通<strong>加粗</strong><span style="color: #ff0000">红字</span>"#);
        assert_eq!(text, "普通加粗红字");
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].mark_type, "bold");
        assert_eq!(spans[0].range, (2, 4));
        assert_eq!(spans[1].mark_type, "textStyle");
        assert_eq!(spans[1].range, (4, 6));
        assert_eq!(spans[1].attrs.as_ref().unwrap()["color"], "#ff0000");
    }

    // covers: BR-ED-DATA-005
    #[test]
    fn extract_marks_nested_entities_and_void() {
        // 嵌套 <strong><em>…</em></strong> + 实体 &amp;（1 字符）+ <br/>（占位 1 字符）
        let (text, spans) = extract_marks("a<strong>b<em>c</em></strong>&amp;<br/>d");
        assert_eq!(text, "abc&\u{fffc}d");
        // em 段 (2,3)、strong 段 (1,3)
        assert!(spans.contains(&MarkSpan {
            mark_type: "italic".into(),
            range: (2, 3),
            attrs: None
        }));
        assert!(spans.contains(&MarkSpan {
            mark_type: "bold".into(),
            range: (1, 3),
            attrs: None
        }));
    }

    // covers: BR-ED-DATA-005
    #[test]
    fn extract_block_attrs_text_align() {
        let attrs = extract_block_attrs(r#"<p data-block-id="b1" style="text-align: center">"#);
        assert_eq!(attrs["textAlign"], "center");
        assert_eq!(extract_block_attrs("<p>"), serde_json::json!({}));
    }

    #[test]
    fn out_of_range_errors() {
        assert!(replace_block("a\n\nb", false, 5, "x").is_err());
        assert!(replace_block(r#"<p data-block-id="b1">a</p>"#, true, 3, "x").is_err());
    }

    #[test]
    fn block_id_captured_and_anchored_replace() {
        let html = r#"<p data-block-id="b1">旧</p><h1 data-block-id="b2">标题</h1>"#;
        let blocks = parse_blocks(html, true);
        assert_eq!(blocks[0].block_id.as_deref(), Some("b1"));
        assert_eq!(blocks[1].block_id.as_deref(), Some("b2"));
        // 按 id 锚定替换（与位置无关）。
        let out = replace_block_by_id(html, "b2", "新标题").unwrap();
        assert_eq!(
            out,
            r#"<p data-block-id="b1">旧</p><h1 data-block-id="b2">新标题</h1>"#
        );
    }

    #[test]
    fn replace_by_unknown_id_is_anchor_lost() {
        let r = replace_block_by_id(r#"<p data-block-id="b1">x</p>"#, "gone", "y");
        assert!(r.unwrap_err().contains("锚失效"));
    }
}
