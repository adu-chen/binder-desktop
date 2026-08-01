//! DOCX TableCellShading 的 OOXML 往返适配层。
//!
//! Pandoc 会保留 HTML 表格结构，却不会把 `td/th` 的 CSS 底纹稳定写为 `w:tcPr/w:shd`，也不会把
//! 来源 `w:shd` 稳定恢复为 HTML 属性。本模块在保存侧用短生命周期标记定位单元格并直接物化 OOXML，
//! 打开侧按单元格文档序恢复 `data-binder-cell-shading` 与 `background-color`。只保证显式不透明
//! `#RRGGBB`；主题色、透明度、渐变和图案计入未恢复报告。

use crate::ipc::IpcError;
use std::io::{Cursor, Read, Write};
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const MARKER_PREFIX: &str = "BINDER_CELL_SHADING_";

/**
 * @GOV
 * codes:    BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-008
 * type:     DATA
 * chain:    DOC-CONVERT
 * rules:    BR-DOC-DATA-002
 * term_ref: TERM-DOC-005
 * boundary: in=DocxTableCellShadingScan | out=CellShadingRecoveryCounts
 */
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct CellShadingRecovery {
    pub detected_shaded_cells: usize,
    pub restored_cells: usize,
    pub unrecovered_cells: usize,
}

#[derive(Debug, Clone, Default)]
struct CellShading {
    detected: bool,
    color: Option<String>,
}

fn fail(detail: impl AsRef<str>) -> IpcError {
    IpcError::new(
        "DOC_EDIT_CELL_SHADING_PROCESS_FAIL",
        &format!("DOCX 单元格底纹处理失败: {}", detail.as_ref()),
        true,
        false,
    )
}

fn normalize_color(raw: &str) -> Option<String> {
    let value = raw.trim().to_ascii_lowercase();
    if let Some(hex) = value.strip_prefix('#') {
        return (hex.len() == 6 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
            .then(|| hex.to_ascii_uppercase());
    }
    if value.len() == 6 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Some(value.to_ascii_uppercase());
    }
    let body = value.strip_prefix("rgb(")?.strip_suffix(')')?;
    let channels = body
        .split(',')
        .map(|part| part.trim().parse::<u8>().ok())
        .collect::<Option<Vec<_>>>()?;
    (channels.len() == 3)
        .then(|| format!("{:02X}{:02X}{:02X}", channels[0], channels[1], channels[2]))
}

fn attribute_range(tag: &str, name: &str) -> Option<(usize, usize)> {
    let lower = tag.to_ascii_lowercase();
    let needle = format!("{}=", name.to_ascii_lowercase());
    let name_start = lower.find(&needle)?;
    let quote_pos = name_start + needle.len();
    let quote = *tag.as_bytes().get(quote_pos)?;
    if quote != b'\'' && quote != b'"' {
        return None;
    }
    let value_start = quote_pos + 1;
    let value_end = value_start + tag[value_start..].find(quote as char)?;
    Some((value_start, value_end))
}

fn attribute(tag: &str, name: &str) -> Option<String> {
    let (start, end) = attribute_range(tag, name)?;
    Some(tag[start..end].to_string())
}

fn insert_attribute(tag: &str, name: &str, value: &str) -> String {
    if let Some((start, end)) = attribute_range(tag, name) {
        let mut out = tag.to_string();
        out.replace_range(start..end, value);
        return out;
    }
    let Some(close) = tag.rfind('>') else {
        return tag.to_string();
    };
    let slash = tag[..close]
        .trim_end()
        .ends_with('/')
        .then(|| tag[..close].rfind('/').unwrap_or(close))
        .unwrap_or(close);
    let mut out = tag.to_string();
    out.insert_str(slash, &format!(r#" {name}="{value}""#));
    out
}

fn cell_tag_name(tag: &str) -> Option<&str> {
    let trimmed = tag.trim_start_matches('<').trim_start();
    if trimmed.starts_with('/') || trimmed.starts_with('!') || trimmed.starts_with('?') {
        return None;
    }
    let name = trimmed
        .split(|ch: char| ch.is_ascii_whitespace() || ch == '>' || ch == '/')
        .next()
        .unwrap_or_default();
    (name.eq_ignore_ascii_case("td") || name.eq_ignore_ascii_case("th")).then_some(name)
}

fn shading_from_html_tag(tag: &str) -> Option<String> {
    if let Some(color) =
        attribute(tag, "data-binder-cell-shading").and_then(|value| normalize_color(&value))
    {
        return Some(color);
    }
    let style = attribute(tag, "style")?;
    style.split(';').find_map(|property| {
        let (name, value) = property.split_once(':')?;
        name.trim()
            .eq_ignore_ascii_case("background-color")
            .then(|| normalize_color(value))?
    })
}

fn set_html_tag_shading(tag: &str, color: &str) -> String {
    let mut out = insert_attribute(tag, "data-binder-cell-shading", &format!("#{color}"));
    let style = attribute(&out, "style").unwrap_or_default();
    let mut properties = style
        .split(';')
        .filter_map(|property| {
            let trimmed = property.trim();
            if trimmed.is_empty() {
                return None;
            }
            let name = trimmed
                .split_once(':')
                .map(|(name, _)| name.trim())
                .unwrap_or("");
            (!name.eq_ignore_ascii_case("background-color")).then_some(trimmed.to_string())
        })
        .collect::<Vec<_>>();
    properties.push(format!("background-color:#{color}"));
    out = insert_attribute(&out, "style", &properties.join(";"));
    out
}

/// 在有显式 TableCellShading 的 td/th 开头插入 Pandoc 会保留的短标记。
pub fn mark_html_cell_shading(html: &str) -> String {
    let mut out = String::with_capacity(html.len() + 128);
    let mut cursor = 0;
    let mut index = 0usize;
    while let Some(relative) = html[cursor..].find('<') {
        let start = cursor + relative;
        let Some(end_relative) = html[start..].find('>') else {
            break;
        };
        let end = start + end_relative + 1;
        let tag = &html[start..end];
        out.push_str(&html[cursor..end]);
        if cell_tag_name(tag).is_some() {
            if let Some(color) = shading_from_html_tag(tag) {
                out.push_str(&format!(
                    r#"<span custom-style="BinderCellShadingMarker">{MARKER_PREFIX}{index}_{color}</span>"#
                ));
                index += 1;
            }
        }
        cursor = end;
    }
    out.push_str(&html[cursor..]);
    out
}

fn xml_unescape(value: &str) -> String {
    value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
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

fn marker_color(text: &str) -> Option<String> {
    let value = text.strip_prefix(MARKER_PREFIX)?;
    let (_, color) = value.rsplit_once('_')?;
    normalize_color(color)
}

fn find_element_start(xml: &str, from: usize, name: &str) -> Option<usize> {
    let mut cursor = from;
    while let Some(relative) = xml[cursor..].find(name) {
        let start = cursor + relative;
        let after = xml.as_bytes().get(start + name.len()).copied();
        if matches!(
            after,
            Some(b'>') | Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r')
        ) {
            return Some(start);
        }
        cursor = start + name.len();
    }
    None
}

fn cell_marker_color(cell: &str) -> Option<String> {
    let mut cursor = 0;
    while let Some(start) = find_element_start(cell, cursor, "<w:r") {
        let end = start + cell[start..].find("</w:r>")? + "</w:r>".len();
        if let Some(color) = marker_color(&run_text(&cell[start..end])) {
            return Some(color);
        }
        cursor = end;
    }
    None
}

fn remove_marker_runs(cell: &str) -> String {
    // 标记通常是独立首段；先只移除“含标记且无其他正文/对象”的段，保留用户原有空段。
    let mut without_marker_paragraph = String::with_capacity(cell.len());
    let mut paragraph_cursor = 0;
    while let Some(start) = find_element_start(cell, paragraph_cursor, "<w:p") {
        let Some(end_relative) = cell[start..].find("</w:p>") else {
            break;
        };
        let end = start + end_relative + "</w:p>".len();
        without_marker_paragraph.push_str(&cell[paragraph_cursor..start]);
        let paragraph = &cell[start..end];
        let mut run_cursor = 0;
        let mut has_marker = false;
        let mut has_other_text = false;
        while let Some(run_start) = find_element_start(paragraph, run_cursor, "<w:r") {
            let Some(run_end_relative) = paragraph[run_start..].find("</w:r>") else {
                break;
            };
            let run_end = run_start + run_end_relative + "</w:r>".len();
            let text = run_text(&paragraph[run_start..run_end]);
            if marker_color(&text).is_some() {
                has_marker = true;
            } else if !text.is_empty() {
                has_other_text = true;
            }
            run_cursor = run_end;
        }
        if !has_marker
            || has_other_text
            || paragraph.contains("<w:drawing")
            || paragraph.contains("<w:br")
        {
            without_marker_paragraph.push_str(paragraph);
        }
        paragraph_cursor = end;
    }
    without_marker_paragraph.push_str(&cell[paragraph_cursor..]);

    // 防御 Pandoc 把标记与正文放入同段的情况：只移除文本恰为标记的 run，不碰相邻正文 run。
    let mut out = String::with_capacity(without_marker_paragraph.len());
    let mut cursor = 0;
    while let Some(start) = find_element_start(&without_marker_paragraph, cursor, "<w:r") {
        let Some(end_relative) = without_marker_paragraph[start..].find("</w:r>") else {
            break;
        };
        let end = start + end_relative + "</w:r>".len();
        out.push_str(&without_marker_paragraph[cursor..start]);
        let run = &without_marker_paragraph[start..end];
        if marker_color(&run_text(run)).is_none() {
            out.push_str(run);
        }
        cursor = end;
    }
    out.push_str(&without_marker_paragraph[cursor..]);
    out
}

fn strip_existing_shading(props: &str) -> String {
    let mut out = String::with_capacity(props.len());
    let mut cursor = 0;
    while let Some(relative) = props[cursor..].find("<w:shd") {
        let start = cursor + relative;
        out.push_str(&props[cursor..start]);
        let Some(open_end_relative) = props[start..].find('>') else {
            cursor = start;
            break;
        };
        let open_end = start + open_end_relative + 1;
        if props[start..open_end].trim_end().ends_with("/>") {
            cursor = open_end;
        } else if let Some(close_relative) = props[open_end..].find("</w:shd>") {
            cursor = open_end + close_relative + "</w:shd>".len();
        } else {
            cursor = open_end;
        }
    }
    out.push_str(&props[cursor..]);
    out
}

fn set_cell_shading(cell: &str, color: &str) -> String {
    let cleaned = remove_marker_runs(cell);
    let shading = format!(r#"<w:shd w:val="clear" w:fill="{color}"/>"#);
    if let Some(props_start) = find_element_start(&cleaned, 0, "<w:tcPr") {
        let Some(open_end_relative) = cleaned[props_start..].find('>') else {
            return cleaned;
        };
        let open_end = props_start + open_end_relative + 1;
        if cleaned[props_start..open_end].trim_end().ends_with("/>") {
            let mut out = cleaned;
            out.replace_range(
                props_start..open_end,
                &format!("<w:tcPr>{shading}</w:tcPr>"),
            );
            return out;
        }
        let Some(close_relative) = cleaned[open_end..].find("</w:tcPr>") else {
            return cleaned;
        };
        let close = open_end + close_relative;
        let props = strip_existing_shading(&cleaned[open_end..close]);
        return format!(
            "{}{}{}{}",
            &cleaned[..open_end],
            props,
            shading,
            &cleaned[close..]
        );
    }
    let Some(open_end) = cleaned.find('>').map(|index| index + 1) else {
        return cleaned;
    };
    format!(
        "{}<w:tcPr>{shading}</w:tcPr>{}",
        &cleaned[..open_end],
        &cleaned[open_end..]
    )
}

fn transform_document_shading(xml: &str) -> String {
    let mut out = String::with_capacity(xml.len());
    let mut cursor = 0;
    while let Some(start) = find_element_start(xml, cursor, "<w:tc") {
        let Some(end_relative) = xml[start..].find("</w:tc>") else {
            break;
        };
        let end = start + end_relative + "</w:tc>".len();
        let cell = &xml[start..end];
        out.push_str(&xml[cursor..start]);
        if let Some(color) = cell_marker_color(cell) {
            out.push_str(&set_cell_shading(cell, &color));
        } else {
            out.push_str(cell);
        }
        cursor = end;
    }
    out.push_str(&xml[cursor..]);
    out
}

fn document_xml(bytes: &[u8]) -> Result<String, IpcError> {
    let mut archive =
        ZipArchive::new(Cursor::new(bytes)).map_err(|error| fail(error.to_string()))?;
    let mut file = archive
        .by_name("word/document.xml")
        .map_err(|error| fail(format!("缺少 word/document.xml: {error}")))?;
    let mut xml = String::new();
    file.read_to_string(&mut xml)
        .map_err(|error| fail(error.to_string()))?;
    Ok(xml)
}

fn rewrite_document_xml(bytes: &[u8], next_xml: &str) -> Result<Vec<u8>, IpcError> {
    let mut archive =
        ZipArchive::new(Cursor::new(bytes)).map_err(|error| fail(error.to_string()))?;
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| fail(error.to_string()))?;
        let name = entry.name().to_string();
        let options = FileOptions::default().compression_method(match entry.compression() {
            CompressionMethod::Stored => CompressionMethod::Stored,
            _ => CompressionMethod::Deflated,
        });
        if entry.is_dir() {
            writer
                .add_directory(name, options)
                .map_err(|error| fail(error.to_string()))?;
            continue;
        }
        writer
            .start_file(&name, options)
            .map_err(|error| fail(error.to_string()))?;
        if name == "word/document.xml" {
            writer
                .write_all(next_xml.as_bytes())
                .map_err(|error| fail(error.to_string()))?;
        } else {
            std::io::copy(&mut entry, &mut writer).map_err(|error| fail(error.to_string()))?;
        }
    }
    let output = writer.finish().map_err(|error| fail(error.to_string()))?;
    Ok(output.into_inner())
}

/**
 * @GOV
 * codes:    BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-006
 * type:     IO
 * chain:    DOC-CONVERT
 * rules:    BR-DOC-DATA-002
 * term_ref: TERM-DOC-005
 * boundary: in=PandocDocxBytesWithCellShadingMarkers | out=DocxBytesWithExplicitWShd | does_not=retain_marker_text
 */
pub fn materialize_marked_cell_shading(docx: &[u8]) -> Result<Vec<u8>, IpcError> {
    let xml = document_xml(docx)?;
    rewrite_document_xml(docx, &transform_document_shading(&xml))
}

fn cell_shading(cell: &str) -> CellShading {
    let Some(props_start) = find_element_start(cell, 0, "<w:tcPr") else {
        return CellShading::default();
    };
    let Some(props_end_relative) = cell[props_start..].find("</w:tcPr>") else {
        return CellShading::default();
    };
    let props_end = props_start + props_end_relative;
    let props = &cell[props_start..props_end];
    let Some(shading_start) = find_element_start(props, 0, "<w:shd") else {
        return CellShading::default();
    };
    let Some(shading_end_relative) = props[shading_start..].find('>') else {
        return CellShading {
            detected: true,
            color: None,
        };
    };
    let tag = &props[shading_start..shading_start + shading_end_relative + 1];
    let pattern_supported = attribute(tag, "w:val")
        .map(|value| matches!(value.to_ascii_lowercase().as_str(), "clear" | "solid"))
        .unwrap_or(true);
    let color = pattern_supported
        .then(|| attribute(tag, "w:fill").and_then(|value| normalize_color(&value)))
        .flatten();
    CellShading {
        detected: true,
        color,
    }
}

fn document_cell_shadings(docx: &[u8]) -> Result<Vec<CellShading>, IpcError> {
    let xml = document_xml(docx)?;
    let mut cells = Vec::new();
    let mut cursor = 0;
    while let Some(start) = find_element_start(&xml, cursor, "<w:tc") {
        let Some(end_relative) = xml[start..].find("</w:tc>") else {
            break;
        };
        let end = start + end_relative + "</w:tc>".len();
        cells.push(cell_shading(&xml[start..end]));
        cursor = end;
    }
    Ok(cells)
}

/**
 * @GOV
 * codes:    BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-007
 * type:     IO
 * chain:    DOC-CONVERT
 * rules:    BR-DOC-DATA-002
 * term_ref: TERM-DOC-005
 * boundary: in=SourceDocxBytesAndPandocHtml | out=EditableHtmlAndCellShadingRecovery | does_not=map_theme_or_pattern_fill
 */
pub fn restore_html_cell_shading(
    docx: &[u8],
    html: &str,
) -> Result<(String, CellShadingRecovery), IpcError> {
    let cells = document_cell_shadings(docx)?;
    let mut report = CellShadingRecovery {
        detected_shaded_cells: cells.iter().filter(|cell| cell.detected).count(),
        ..Default::default()
    };
    let mut out = String::with_capacity(html.len() + report.detected_shaded_cells * 64);
    let mut cursor = 0;
    let mut cell_index = 0usize;
    while let Some(relative) = html[cursor..].find('<') {
        let start = cursor + relative;
        let Some(end_relative) = html[start..].find('>') else {
            break;
        };
        let end = start + end_relative + 1;
        let tag = &html[start..end];
        out.push_str(&html[cursor..start]);
        if cell_tag_name(tag).is_some() {
            if let Some(shading) = cells.get(cell_index) {
                if let Some(color) = shading.color.as_deref() {
                    out.push_str(&set_html_tag_shading(tag, color));
                    report.restored_cells += 1;
                } else {
                    out.push_str(tag);
                    if shading.detected {
                        report.unrecovered_cells += 1;
                    }
                }
            } else {
                out.push_str(tag);
            }
            cell_index += 1;
        } else {
            out.push_str(tag);
        }
        cursor = end;
    }
    out.push_str(&html[cursor..]);
    report.unrecovered_cells += cells[cell_index.min(cells.len())..]
        .iter()
        .filter(|cell| cell.detected)
        .count();
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

    // covers: BR-DOC-DATA-002
    #[test]
    fn marks_only_supported_cell_colors() {
        let html = r##"<table><tr><td data-binder-cell-shading="#ffe699"><p>A</p></td><th style="background-color:rgb(255, 0, 0)"><p>B</p></th><td style="background-color:transparent">C</td></tr></table>"##;
        let marked = mark_html_cell_shading(html);
        assert!(marked.contains("BINDER_CELL_SHADING_0_FFE699"));
        assert!(marked.contains("BINDER_CELL_SHADING_1_FF0000"));
        assert_eq!(marked.matches(MARKER_PREFIX).count(), 2);
    }

    // covers: BR-DOC-DATA-002
    #[test]
    fn materializes_marker_as_w_shd_and_removes_marker_text() {
        let xml = r#"<w:document xmlns:w="w"><w:body><w:tbl><w:tr><w:tc><w:tcPr><w:tcW w:w="1"/></w:tcPr><w:p><w:r><w:t>BINDER_CELL_SHADING_0_FFE699</w:t></w:r></w:p><w:p><w:r><w:t>内容</w:t></w:r></w:p></w:tc></w:tr></w:tbl></w:body></w:document>"#;
        let docx = materialize_marked_cell_shading(&zip_with_document(xml)).unwrap();
        let transformed = document_xml(&docx).unwrap();
        assert!(transformed.contains(r#"<w:shd w:val="clear" w:fill="FFE699"/>"#));
        assert!(transformed.contains("内容"));
        assert!(!transformed.contains(MARKER_PREFIX));
    }

    // covers: BR-DOC-DATA-002
    #[test]
    fn restores_explicit_fill_and_reports_pattern_as_unrecovered() {
        let xml = r#"<w:document xmlns:w="w"><w:body><w:tbl><w:tr><w:tc><w:tcPr><w:shd w:val="clear" w:fill="FFE699"/></w:tcPr><w:p><w:r><w:t>A</w:t></w:r></w:p></w:tc><w:tc><w:tcPr><w:shd w:val="pct20" w:fill="FF0000"/></w:tcPr><w:p><w:r><w:t>B</w:t></w:r></w:p></w:tc></w:tr></w:tbl></w:body></w:document>"#;
        let html =
            r#"<table><tr><td class="a">A</td><th style="text-align:center">B</th></tr></table>"#;
        let (restored, report) = restore_html_cell_shading(&zip_with_document(xml), html).unwrap();
        assert!(restored.contains(r##"data-binder-cell-shading="#FFE699""##));
        assert!(restored.contains("background-color:#FFE699"));
        assert!(restored.contains("text-align:center"));
        assert_eq!(
            report,
            CellShadingRecovery {
                detected_shaded_cells: 2,
                restored_cells: 1,
                unrecovered_cells: 1,
            }
        );
    }
}
