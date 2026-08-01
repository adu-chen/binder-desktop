//! DOCX 文本颜色的 OOXML 往返适配层。
//!
//! Pandoc 能承担 HTML/DOCX 结构转换，却不会稳定把 HTML CSS `color` 写为 `w:color`，也不会稳定把
//! `w:color` 读回 HTML。因此颜色在 Pandoc 两端以短生命周期标记传递，最终由本模块直接写入/读取
//! `word/document.xml`。标记不会落入最终 DOCX；无法按文本顺序恢复的来源颜色会返回统计，交由样式报告
//! 明示，而不静默宣称保真。

use crate::ipc::IpcError;
use std::io::{Cursor, Read, Write};
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const START_PREFIX: &str = "BINDER_COLOR_START_";
const END_PREFIX: &str = "BINDER_COLOR_END_";

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ColorRecovery {
    pub detected_colored_runs: usize,
    pub restored_runs: usize,
    pub unrecovered_runs: usize,
}

#[derive(Debug, Clone)]
struct ColoredRun {
    text: String,
    color: Option<String>,
}

fn fail(detail: impl AsRef<str>) -> IpcError {
    IpcError::new(
        "DOC_EDIT_COLOR_PROCESS_FAIL",
        &format!("DOCX 文字颜色处理失败: {}", detail.as_ref()),
        true,
        false,
    )
}

/// 规范化编辑器可生成的 CSS 颜色；未知形式不进入颜色写回，避免生成非法 OOXML。
fn normalize_color(raw: &str) -> Option<String> {
    let value = raw.trim().to_ascii_lowercase();
    if let Some(hex) = value.strip_prefix('#') {
        return match hex.len() {
            3 if hex.bytes().all(|b| b.is_ascii_hexdigit()) => Some(
                hex.chars()
                    .flat_map(|c| [c, c])
                    .collect::<String>()
                    .to_ascii_uppercase(),
            ),
            6 if hex.bytes().all(|b| b.is_ascii_hexdigit()) => Some(hex.to_ascii_uppercase()),
            _ => None,
        };
    }
    if value.len() == 6 && value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Some(value.to_ascii_uppercase());
    }
    let body = value.strip_prefix("rgb(")?.strip_suffix(')')?;
    let channels = body
        .split(',')
        .map(|v| v.trim().parse::<u8>().ok())
        .collect::<Option<Vec<_>>>()?;
    (channels.len() == 3)
        .then(|| format!("{:02X}{:02X}{:02X}", channels[0], channels[1], channels[2]))
}

fn attribute(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let needle = format!("{name}=");
    let start = lower.find(&needle)? + needle.len();
    let quote = tag.as_bytes().get(start).copied()?;
    if quote != b'\'' && quote != b'"' {
        return None;
    }
    let rest = &tag[start + 1..];
    let end = rest.find(quote as char)?;
    Some(rest[..end].to_string())
}

fn color_from_tag(tag: &str) -> Option<String> {
    let style = attribute(tag, "style")?;
    style.split(';').find_map(|property| {
        let (name, value) = property.split_once(':')?;
        (name.trim().eq_ignore_ascii_case("color")).then(|| normalize_color(value))?
    })
}

fn is_span_tag(tag: &str) -> (bool, bool) {
    let trimmed = tag.trim_start_matches('<').trim_start();
    let closing = trimmed.starts_with('/');
    let name = trimmed
        .trim_start_matches('/')
        .trim_start()
        .split(|c: char| c.is_ascii_whitespace() || c == '>' || c == '/')
        .next()
        .unwrap_or_default();
    (name.eq_ignore_ascii_case("span"), closing)
}

/// 在有 CSS color 的 span 内插入 Pandoc 会保留为独立 run 的短标记。
pub fn mark_html_colors(html: &str) -> String {
    let mut out = String::with_capacity(html.len() + 128);
    let mut cursor = 0;
    let mut index = 0usize;
    let mut span_stack: Vec<Option<(usize, String)>> = Vec::new();
    while let Some(relative) = html[cursor..].find('<') {
        let start = cursor + relative;
        let Some(end_relative) = html[start..].find('>') else {
            break;
        };
        let end = start + end_relative + 1;
        let tag = &html[start..end];
        let (is_span, closing) = is_span_tag(tag);
        out.push_str(&html[cursor..start]);
        if is_span && closing {
            if let Some(Some((marker_id, _))) = span_stack.pop() {
                out.push_str(&format!(
                    r#"<span custom-style="BinderColorMarker">{END_PREFIX}{marker_id}</span>"#
                ));
            }
            out.push_str(tag);
        } else {
            out.push_str(tag);
            if is_span {
                let marker = color_from_tag(tag).map(|color| {
                    let current = index;
                    index += 1;
                    out.push_str(&format!(
                        r#"<span custom-style="BinderColorMarker">{START_PREFIX}{current}_{color}</span>"#
                    ));
                    (current, color)
                });
                span_stack.push(marker);
            }
        }
        cursor = end;
    }
    out.push_str(&html[cursor..]);
    out
}

fn run_text(run: &str) -> String {
    let mut text = String::new();
    let mut cursor = 0;
    while let Some(relative) = run[cursor..].find("<w:t") {
        let start = cursor + relative;
        let Some(content_relative) = run[start..].find('>') else {
            break;
        };
        let content_start = start + content_relative + 1;
        let Some(end_relative) = run[content_start..].find("</w:t>") else {
            break;
        };
        let content_end = content_start + end_relative;
        text.push_str(&xml_unescape(&run[content_start..content_end]));
        cursor = content_end + "</w:t>".len();
    }
    text
}

fn xml_unescape(value: &str) -> String {
    value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

fn run_color(run: &str) -> Option<String> {
    let start = run.find("<w:color")?;
    let end = start + run[start..].find('>')? + 1;
    attribute(&run[start..end], "w:val").and_then(|value| normalize_color(&value))
}

fn marker_start(text: &str) -> Option<(usize, String)> {
    let rest = text.strip_prefix(START_PREFIX)?;
    let (id, color) = rest.rsplit_once('_')?;
    Some((id.parse().ok()?, normalize_color(color)?))
}

fn marker_end(text: &str) -> Option<usize> {
    text.strip_prefix(END_PREFIX)?.parse().ok()
}

fn set_run_color(run: &str, color: &str) -> String {
    if !run.contains("<w:t") {
        return run.to_string();
    }
    if let Some(props_start) = run.find("<w:rPr") {
        let Some(open_end_relative) = run[props_start..].find('>') else {
            return run.to_string();
        };
        let props_content_start = props_start + open_end_relative + 1;
        let Some(props_end_relative) = run[props_content_start..].find("</w:rPr>") else {
            return run.to_string();
        };
        let props_end = props_content_start + props_end_relative;
        let mut props = run[props_content_start..props_end].to_string();
        while let Some(color_start) = props.find("<w:color") {
            let Some(color_end_relative) = props[color_start..].find('>') else {
                break;
            };
            props.replace_range(color_start..color_start + color_end_relative + 1, "");
        }
        props.push_str(&format!(r#"<w:color w:val="{color}"/>"#));
        format!(
            "{}{}{}",
            &run[..props_content_start],
            props,
            &run[props_end..]
        )
    } else if let Some(run_end) = run.find('>') {
        format!(
            "{}<w:rPr><w:color w:val=\"{}\"/></w:rPr>{}",
            &run[..run_end + 1],
            color,
            &run[run_end + 1..]
        )
    } else {
        run.to_string()
    }
}

fn transform_document_colors(xml: &str) -> String {
    let mut out = String::with_capacity(xml.len());
    let mut cursor = 0;
    let mut active: Vec<(usize, String)> = Vec::new();
    while let Some(relative) = xml[cursor..].find("<w:r") {
        let start = cursor + relative;
        let after_name = xml.as_bytes().get(start + 4).copied();
        if !matches!(
            after_name,
            Some(b'>') | Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r')
        ) {
            out.push_str(&xml[cursor..start + 4]);
            cursor = start + 4;
            continue;
        }
        let Some(end_relative) = xml[start..].find("</w:r>") else {
            break;
        };
        let end = start + end_relative + "</w:r>".len();
        let run = &xml[start..end];
        let text = run_text(run);
        out.push_str(&xml[cursor..start]);
        if let Some((id, color)) = marker_start(&text) {
            active.push((id, color));
        } else if let Some(id) = marker_end(&text) {
            if let Some(pos) = active.iter().rposition(|(active_id, _)| *active_id == id) {
                active.remove(pos);
            }
        } else if let Some((_, color)) = active.last() {
            out.push_str(&set_run_color(run, color));
        } else {
            out.push_str(run);
        }
        cursor = end;
    }
    out.push_str(&xml[cursor..]);
    out
}

fn document_xml(bytes: &[u8]) -> Result<String, IpcError> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|e| fail(e.to_string()))?;
    let mut file = archive
        .by_name("word/document.xml")
        .map_err(|e| fail(format!("缺少 word/document.xml: {e}")))?;
    let mut xml = String::new();
    file.read_to_string(&mut xml)
        .map_err(|e| fail(e.to_string()))?;
    Ok(xml)
}

fn rewrite_document_xml(bytes: &[u8], next_xml: &str) -> Result<Vec<u8>, IpcError> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|e| fail(e.to_string()))?;
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|e| fail(e.to_string()))?;
        let name = entry.name().to_string();
        let options = FileOptions::default().compression_method(match entry.compression() {
            CompressionMethod::Stored => CompressionMethod::Stored,
            _ => CompressionMethod::Deflated,
        });
        if entry.is_dir() {
            writer
                .add_directory(name, options)
                .map_err(|e| fail(e.to_string()))?;
            continue;
        }
        writer
            .start_file(&name, options)
            .map_err(|e| fail(e.to_string()))?;
        if name == "word/document.xml" {
            writer
                .write_all(next_xml.as_bytes())
                .map_err(|e| fail(e.to_string()))?;
        } else {
            std::io::copy(&mut entry, &mut writer).map_err(|e| fail(e.to_string()))?;
        }
    }
    let output = writer.finish().map_err(|e| fail(e.to_string()))?;
    Ok(output.into_inner())
}

/// 移除 Pandoc 中转标记，并在所有颜色区间的文本 run 上写入显式 `w:color`。
pub fn materialize_marked_colors(docx: &[u8]) -> Result<Vec<u8>, IpcError> {
    let xml = document_xml(docx)?;
    rewrite_document_xml(docx, &transform_document_colors(&xml))
}

fn colored_runs(docx: &[u8]) -> Result<Vec<ColoredRun>, IpcError> {
    let xml = document_xml(docx)?;
    let mut runs = Vec::new();
    let mut cursor = 0;
    while let Some(relative) = xml[cursor..].find("<w:r") {
        let start = cursor + relative;
        let after_name = xml.as_bytes().get(start + 4).copied();
        if !matches!(
            after_name,
            Some(b'>') | Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r')
        ) {
            cursor = start + 4;
            continue;
        }
        let Some(end_relative) = xml[start..].find("</w:r>") else {
            break;
        };
        let end = start + end_relative + "</w:r>".len();
        let run = &xml[start..end];
        let text = run_text(run);
        if !text.is_empty() {
            runs.push(ColoredRun {
                text,
                color: run_color(run),
            });
        }
        cursor = end;
    }
    Ok(runs)
}

fn visible_text_match(html: &str, needle: &str, from: usize) -> Option<(usize, usize)> {
    let mut offset = from;
    while let Some(relative) = html[offset..].find(needle) {
        let start = offset + relative;
        let end = start + needle.len();
        let before = &html[..start];
        let open = before.rfind('<');
        let close = before.rfind('>');
        if close.map_or(true, |i| open.map_or(true, |j| i > j)) && !html[start..end].contains('<') {
            return Some((start, end));
        }
        offset = end;
    }
    None
}

/// 将来源 DOCX 直接颜色恢复成编辑模型 HTML 的 `style="color:#RRGGBB"`。
pub fn restore_html_colors(docx: &[u8], html: &str) -> Result<(String, ColorRecovery), IpcError> {
    let runs = colored_runs(docx)?;
    let mut report = ColorRecovery {
        detected_colored_runs: runs.iter().filter(|run| run.color.is_some()).count(),
        ..Default::default()
    };
    let mut out = String::with_capacity(html.len() + report.detected_colored_runs * 32);
    let mut source_cursor = 0;
    for run in runs {
        let Some((start, end)) = visible_text_match(html, &run.text, source_cursor) else {
            if run.color.is_some() {
                report.unrecovered_runs += 1;
            }
            continue;
        };
        out.push_str(&html[source_cursor..start]);
        if let Some(color) = run.color {
            out.push_str(&format!(
                r#"<span style="color:#{color}">{}"#,
                &html[start..end]
            ));
            out.push_str("</span>");
            report.restored_runs += 1;
        } else {
            out.push_str(&html[start..end]);
        }
        source_cursor = end;
    }
    out.push_str(&html[source_cursor..]);
    Ok((out, report))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zip_with_document(xml: &str) -> Vec<u8> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file(
                "word/document.xml",
                FileOptions::default().compression_method(CompressionMethod::Deflated),
            )
            .unwrap();
        writer.write_all(xml.as_bytes()).unwrap();
        writer.finish().unwrap().into_inner()
    }

    #[test]
    fn marks_hex_and_rgb_colors_only() {
        let marked = mark_html_colors(
            r#"<p><span style="color:#f00">红</span><span style="font-weight:bold">黑</span><span style="color:rgb(0, 128, 255)">蓝</span></p>"#,
        );
        assert!(marked.contains("BINDER_COLOR_START_0_FF0000"));
        assert!(marked.contains("BINDER_COLOR_START_1_0080FF"));
        assert_eq!(marked.matches("BINDER_COLOR_START_").count(), 2);
    }

    #[test]
    fn materializes_markers_as_ooxml_color_and_removes_them() {
        let xml = r#"<w:document xmlns:w="w"><w:body><w:p><w:r><w:rPr><w:rStyle w:val="BinderColorMarker"/></w:rPr><w:t>BINDER_COLOR_START_0_FF0000</w:t></w:r><w:r><w:t>红字</w:t></w:r><w:r><w:t>BINDER_COLOR_END_0</w:t></w:r></w:p></w:body></w:document>"#;
        let docx = materialize_marked_colors(&zip_with_document(xml)).unwrap();
        let transformed = document_xml(&docx).unwrap();
        assert!(transformed.contains(r#"<w:color w:val="FF0000"/>"#));
        assert!(transformed.contains("红字"));
        assert!(!transformed.contains("BINDER_COLOR_"));
    }

    #[test]
    fn restores_document_color_to_html_and_reports_unmatched_runs() {
        let xml = r#"<w:document xmlns:w="w"><w:body><w:p><w:r><w:rPr><w:color w:val="FF0000"/></w:rPr><w:t>红字</w:t></w:r><w:r><w:t>黑字</w:t></w:r></w:p></w:body></w:document>"#;
        let (html, report) =
            restore_html_colors(&zip_with_document(xml), "<p>红字黑字</p>").unwrap();
        assert!(html.contains(r#"<span style="color:#FF0000">红字</span>"#));
        assert_eq!(report.detected_colored_runs, 1);
        assert_eq!(report.restored_runs, 1);
        assert_eq!(report.unrecovered_runs, 0);

        let (_, report) = restore_html_colors(&zip_with_document(xml), "<p>黑字</p>").unwrap();
        assert_eq!(report.unrecovered_runs, 1);
    }
}
