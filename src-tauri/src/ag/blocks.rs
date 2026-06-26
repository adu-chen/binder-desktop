//! PI-AG-08F · 文档分块（块列表注入 + 块级 propose_edit 重建）。
//!
//! 两种文档形态：
//! - **text/markdown**：按段（`\n\n`）切块；重建 = 替块后 join。
//! - **docx/html**（editFormat=html）：按 `data-block-id` 块级元素切块（与 ED blockId / DOC 兜底同源思路，
//!   A-DOC-M-T-01 BR-DOC-DATA-003），展示 strip 后文本；重建 = **原串 in-place 替换该块 inner（非破坏，
//!   保留其余 HTML 与未捕获内容）**。
//!
//! dev 级轻量字符串扫描（无 HTML parser 依赖），适配 TipTap 平铺块元素（p/h/li/blockquote 等带 block-id）；
//! 嵌套同名块、畸形 HTML 不保证——真实 ED blockId 锚定位（对齐 LocateResult）后续。

/// 一个文档块。`inner_span` = HTML 块 inner 在原串中的字节区间（in-place 重建用）；text 块为 None。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocBlock {
    pub text: String,
    pub inner_span: Option<(usize, usize)>,
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
            .split("\n\n")
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| DocBlock { text: s.to_string(), inner_span: None })
            .collect()
    }
}

/// 仅取各块展示文本（块列表注入用）。
pub fn block_texts(content: &str, is_html: bool) -> Vec<String> {
    parse_blocks(content, is_html).into_iter().map(|b| b.text).collect()
}

/// 把第 `index` 块替换为 `new_text`，返回整篇新内容。
/// HTML：原串 in-place 替换该块 inner（非破坏）；text：替块后 join("\n\n")。越界返回 Err。
pub fn replace_block(content: &str, is_html: bool, index: usize, new_text: &str) -> Result<String, String> {
    let blocks = parse_blocks(content, is_html);
    if index >= blocks.len() {
        return Err(format!("block_index {index} 越界（当前共 {} 块）", blocks.len()));
    }
    if is_html {
        let (s, e) = blocks[index]
            .inner_span
            .ok_or_else(|| "HTML 块缺 inner 区间".to_string())?;
        Ok(format!("{}{}{}", &content[..s], html_escape(new_text), &content[e..]))
    } else {
        let mut texts: Vec<String> = blocks.into_iter().map(|b| b.text).collect();
        texts[index] = new_text.to_string();
        Ok(texts.join("\n\n"))
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
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
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
        let Some(rel_gt) = html[attr_pos..].find('>') else { break };
        let inner_start = attr_pos + rel_gt + 1;
        // 匹配最近的 </tag>。
        let close = format!("</{tag}>");
        let Some(rel_close) = html[inner_start..].find(&close) else {
            search = inner_start;
            continue;
        };
        let inner_end = inner_start + rel_close;
        let inner = &html[inner_start..inner_end];
        blocks.push(DocBlock {
            text: strip_tags(inner).trim().to_string(),
            inner_span: Some((inner_start, inner_end)),
        });
        search = inner_end + close.len();
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_blocks_split_by_paragraph() {
        let b = block_texts("第一段\n\n第二段\n\n第三段", false);
        assert_eq!(b, vec!["第一段", "第二段", "第三段"]);
    }

    #[test]
    fn text_replace_block_joins() {
        let out = replace_block("a\n\nb\n\nc", false, 1, "B").unwrap();
        assert_eq!(out, "a\n\nB\n\nc");
    }

    #[test]
    fn html_blocks_parsed_by_block_id_with_stripped_text() {
        let html = r#"<p data-block-id="b1">你好<strong>世界</strong></p><h1 data-block-id="b2">标题</h1>"#;
        assert!(is_html(html, Some("html")));
        let texts = block_texts(html, true);
        assert_eq!(texts, vec!["你好世界", "标题"]);
    }

    #[test]
    fn html_replace_block_is_inplace_nonlossy() {
        let html = r#"<p data-block-id="b1">旧</p><h1 data-block-id="b2">标题</h1>"#;
        let out = replace_block(html, true, 0, "新内容").unwrap();
        // 仅 b1 inner 被替换，b2 与标签结构原样保留。
        assert_eq!(out, r#"<p data-block-id="b1">新内容</p><h1 data-block-id="b2">标题</h1>"#);
    }

    #[test]
    fn html_replace_escapes_special_chars() {
        let html = r#"<p data-block-id="b1">x</p>"#;
        let out = replace_block(html, true, 0, "a<b>&c").unwrap();
        assert_eq!(out, r#"<p data-block-id="b1">a&lt;b&gt;&amp;c</p>"#);
    }

    #[test]
    fn out_of_range_errors() {
        assert!(replace_block("a\n\nb", false, 5, "x").is_err());
        assert!(replace_block(r#"<p data-block-id="b1">a</p>"#, true, 3, "x").is_err());
    }
}
