//! 独立图片段落对齐的 OOXML 适配与保存验证。
//! 仅处理段落中唯一图片的 left/center/right；不扩展到通用段落对齐或浮动布局。

use crate::ipc::IpcError;
use std::io::{Cursor, Read, Write};
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const MARKER_PREFIX: &str = "BINDER_IMAGE_LAYOUT_";

fn fail(detail: impl AsRef<str>) -> IpcError {
    IpcError::new(
        "DOC_IMAGE_LAYOUT_INVALID",
        &format!("DOCX 图片段落对齐处理失败: {}", detail.as_ref()),
        true,
        false,
    )
}

fn attribute(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let needle = format!("{}=", name.to_ascii_lowercase());
    let start = lower.find(&needle)? + needle.len();
    let quote = *tag.as_bytes().get(start)?;
    if quote != b'\'' && quote != b'"' {
        return None;
    }
    let value_start = start + 1;
    let value_end = value_start + tag[value_start..].find(quote as char)?;
    Some(tag[value_start..value_end].to_string())
}

fn alignment_from_tag(tag: &str) -> &'static str {
    let Some(style) = attribute(tag, "style") else {
        return "left";
    };
    style
        .split(';')
        .find_map(|property| {
            let (name, value) = property.split_once(':')?;
            if !name.trim().eq_ignore_ascii_case("text-align") {
                return None;
            }
            match value.trim().to_ascii_lowercase().as_str() {
                "center" => Some("center"),
                "right" => Some("right"),
                _ => Some("left"),
            }
        })
        .unwrap_or("left")
}

fn image_paragraphs(html: &str) -> Vec<(usize, usize, usize, Option<&'static str>)> {
    let lower = html.to_ascii_lowercase();
    let mut found = Vec::new();
    let mut cursor = 0;
    while let Some(relative) = lower[cursor..].find("<p") {
        let start = cursor + relative;
        let Some(open_relative) = lower[start..].find('>') else {
            break;
        };
        let open_end = start + open_relative + 1;
        let Some(close_relative) = lower[open_end..].find("</p>") else {
            break;
        };
        let close = open_end + close_relative;
        let body = &html[open_end..close];
        let body_lower = body.to_ascii_lowercase();
        let image_count = body_lower.matches("<img").count();
        let mut text = String::new();
        let mut in_tag = false;
        for ch in body.chars() {
            match ch {
                '<' => in_tag = true,
                '>' => in_tag = false,
                _ if !in_tag => text.push(ch),
                _ => {}
            }
        }
        if image_count > 0 {
            let standalone = image_count == 1 && text.replace("&nbsp;", " ").trim().is_empty();
            found.push((
                start,
                open_end,
                close + "</p>".len(),
                standalone.then(|| alignment_from_tag(&html[start..open_end])),
            ));
        }
        cursor = close + "</p>".len();
    }
    found
}

pub fn mark_html_image_layouts(html: &str) -> String {
    let paragraphs = image_paragraphs(html)
        .into_iter()
        .filter_map(|(start, open_end, end, alignment)| {
            alignment.map(|value| (start, open_end, end, value))
        })
        .collect::<Vec<_>>();
    if paragraphs.is_empty() {
        return html.to_string();
    }
    let mut out = String::with_capacity(html.len() + paragraphs.len() * 96);
    let mut cursor = 0;
    for (_, open_end, _, alignment) in paragraphs {
        out.push_str(&html[cursor..open_end]);
        out.push_str(&format!(
            r#"<span custom-style="BinderImageLayoutMarker">{MARKER_PREFIX}{alignment}</span>"#
        ));
        cursor = open_end;
    }
    out.push_str(&html[cursor..]);
    out
}

fn set_paragraph_alignment(paragraph: &str, alignment: &str) -> String {
    let mut clean = paragraph.to_string();
    while let Some(start) = clean.find("<w:jc") {
        let Some(end_relative) = clean[start..].find('>') else {
            break;
        };
        clean.replace_range(start..start + end_relative + 1, "");
    }
    let jc = format!(r#"<w:jc w:val="{alignment}"/>"#);
    if let Some(end) = clean.find("</w:pPr>") {
        clean.insert_str(end, &jc);
        return clean;
    }
    let Some(open_end) = clean.find('>') else {
        return clean;
    };
    clean.insert_str(open_end + 1, &format!("<w:pPr>{jc}</w:pPr>"));
    clean
}

fn remove_marker_runs(paragraph: &str) -> (String, Option<&'static str>) {
    let mut out = String::with_capacity(paragraph.len());
    let mut cursor = 0;
    let mut alignment = None;
    while let Some(relative) = paragraph[cursor..].find("<w:r") {
        let start = cursor + relative;
        let Some(end_relative) = paragraph[start..].find("</w:r>") else {
            break;
        };
        let end = start + end_relative + "</w:r>".len();
        let run = &paragraph[start..end];
        let detected = ["left", "center", "right"]
            .into_iter()
            .find(|value| run.contains(&format!("{MARKER_PREFIX}{value}")));
        out.push_str(&paragraph[cursor..start]);
        if let Some(value) = detected {
            alignment = Some(value);
        } else {
            out.push_str(run);
        }
        cursor = end;
    }
    out.push_str(&paragraph[cursor..]);
    (out, alignment)
}

fn transform_document(xml: &str) -> String {
    let mut out = String::with_capacity(xml.len());
    let mut cursor = 0;
    while let Some(relative) = xml[cursor..].find("<w:p") {
        let start = cursor + relative;
        let after = xml.as_bytes().get(start + 4).copied();
        if !matches!(
            after,
            Some(b'>') | Some(b' ') | Some(b'\t') | Some(b'\r') | Some(b'\n')
        ) {
            out.push_str(&xml[cursor..start + 4]);
            cursor = start + 4;
            continue;
        }
        let Some(end_relative) = xml[start..].find("</w:p>") else {
            break;
        };
        let end = start + end_relative + "</w:p>".len();
        let paragraph = &xml[start..end];
        let (clean, alignment) = remove_marker_runs(paragraph);
        out.push_str(&xml[cursor..start]);
        out.push_str(&alignment.map_or(clean.clone(), |value| {
            set_paragraph_alignment(&clean, value)
        }));
        cursor = end;
    }
    out.push_str(&xml[cursor..]);
    out
}

fn rewrite_document(
    docx: &[u8],
    transform: impl FnOnce(&str) -> String,
) -> Result<Vec<u8>, IpcError> {
    let mut archive =
        ZipArchive::new(Cursor::new(docx)).map_err(|error| fail(error.to_string()))?;
    let mut output = Cursor::new(Vec::new());
    {
        let mut writer = ZipWriter::new(&mut output);
        let mut transform = Some(transform);
        for index in 0..archive.len() {
            let mut entry = archive
                .by_index(index)
                .map_err(|error| fail(error.to_string()))?;
            let mut bytes = Vec::new();
            entry
                .read_to_end(&mut bytes)
                .map_err(|error| fail(error.to_string()))?;
            let options = FileOptions::default().compression_method(
                if entry.compression() == CompressionMethod::Stored {
                    CompressionMethod::Stored
                } else {
                    CompressionMethod::Deflated
                },
            );
            writer
                .start_file(entry.name(), options)
                .map_err(|error| fail(error.to_string()))?;
            if entry.name() == "word/document.xml" {
                let xml = std::str::from_utf8(&bytes).map_err(|error| fail(error.to_string()))?;
                let next = transform.take().expect("document transform used once")(xml);
                writer
                    .write_all(next.as_bytes())
                    .map_err(|error| fail(error.to_string()))?;
            } else {
                writer
                    .write_all(&bytes)
                    .map_err(|error| fail(error.to_string()))?;
            }
        }
        writer.finish().map_err(|error| fail(error.to_string()))?;
    }
    Ok(output.into_inner())
}

pub fn materialize_marked_image_layouts(docx: &[u8]) -> Result<Vec<u8>, IpcError> {
    rewrite_document(docx, transform_document)
}

fn document_xml(docx: &[u8]) -> Result<String, IpcError> {
    let mut archive =
        ZipArchive::new(Cursor::new(docx)).map_err(|error| fail(error.to_string()))?;
    let mut xml = String::new();
    archive
        .by_name("word/document.xml")
        .map_err(|error| fail(error.to_string()))?
        .read_to_string(&mut xml)
        .map_err(|error| fail(error.to_string()))?;
    Ok(xml)
}

fn drawing_alignments(xml: &str) -> Vec<&'static str> {
    let mut result = Vec::new();
    let mut cursor = 0;
    while let Some(relative) = xml[cursor..].find("<w:p") {
        let start = cursor + relative;
        let Some(end_relative) = xml[start..].find("</w:p>") else {
            break;
        };
        let end = start + end_relative + "</w:p>".len();
        let paragraph = &xml[start..end];
        if paragraph.contains("<w:drawing") {
            let alignment = ["center", "right"]
                .into_iter()
                .find(|value| paragraph.contains(&format!(r#"<w:jc w:val="{value}""#)))
                .unwrap_or("left");
            result.push(alignment);
        }
        cursor = end;
    }
    result
}

pub fn verify_docx_image_layouts(html: &str, docx: &[u8]) -> Result<(), IpcError> {
    let expected = image_paragraphs(html);
    let actual = drawing_alignments(&document_xml(docx)?);
    if expected.len() != actual.len()
        || expected
            .iter()
            .zip(&actual)
            .any(|((_, _, _, alignment), actual)| alignment.is_some_and(|value| value != *actual))
    {
        return Err(fail(format!(
            "独立图片段落对齐不一致（期望 {:?}，实际 {:?}）",
            expected
                .iter()
                .map(|(_, _, _, alignment)| *alignment)
                .collect::<Vec<_>>(),
            actual
        )));
    }
    Ok(())
}

fn set_html_alignment(tag: &str, alignment: &str) -> String {
    let existing = attribute(tag, "style").unwrap_or_default();
    let mut properties = existing
        .split(';')
        .filter_map(|property| {
            let trimmed = property.trim();
            if trimmed.is_empty() {
                return None;
            }
            let name = trimmed
                .split_once(':')
                .map(|(name, _)| name.trim())
                .unwrap_or_default();
            (!name.eq_ignore_ascii_case("text-align")).then_some(trimmed.to_string())
        })
        .collect::<Vec<_>>();
    properties.push(format!("text-align: {alignment}"));
    let value = properties.join("; ");
    let lower = tag.to_ascii_lowercase();
    if let Some(style_start) = lower.find("style=") {
        let quote_pos = style_start + "style=".len();
        if let Some(quote) = tag.as_bytes().get(quote_pos).copied() {
            if quote == b'\'' || quote == b'"' {
                let value_start = quote_pos + 1;
                if let Some(relative) = tag[value_start..].find(quote as char) {
                    let mut out = tag.to_string();
                    out.replace_range(value_start..value_start + relative, &value);
                    return out;
                }
            }
        }
    }
    let Some(close) = tag.rfind('>') else {
        return tag.to_string();
    };
    let mut out = tag.to_string();
    out.insert_str(close, &format!(r#" style="{value}""#));
    out
}

pub fn restore_html_image_layouts(docx: &[u8], html: &str) -> Result<String, IpcError> {
    let alignments = drawing_alignments(&document_xml(docx)?);
    let paragraphs = image_paragraphs(html);
    if alignments.len() != paragraphs.len() {
        return Err(fail(format!(
            "图片段落数量不一致（DOCX {}，HTML {}）",
            alignments.len(),
            paragraphs.len()
        )));
    }
    let mut out = String::with_capacity(html.len() + alignments.len() * 28);
    let mut cursor = 0;
    for ((start, open_end, _, _), alignment) in paragraphs.into_iter().zip(alignments) {
        out.push_str(&html[cursor..start]);
        out.push_str(&set_html_alignment(&html[start..open_end], alignment));
        cursor = open_end;
    }
    out.push_str(&html[cursor..]);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zip_with_document(xml: &str) -> Vec<u8> {
        let mut output = Cursor::new(Vec::new());
        {
            let mut writer = ZipWriter::new(&mut output);
            writer
                .start_file("word/document.xml", FileOptions::default())
                .unwrap();
            writer.write_all(xml.as_bytes()).unwrap();
            writer.finish().unwrap();
        }
        output.into_inner()
    }

    #[test]
    fn marks_materializes_and_verifies_standalone_image_alignment() {
        let html = r#"<p style="text-align:center"><img src="media/a.png"></p>"#;
        let marked = mark_html_image_layouts(html);
        assert!(marked.contains("BINDER_IMAGE_LAYOUT_center"));
        let xml = r#"<w:document><w:body><w:p><w:r><w:t>BINDER_IMAGE_LAYOUT_center</w:t></w:r><w:r><w:drawing/></w:r></w:p></w:body></w:document>"#;
        let docx = materialize_marked_image_layouts(&zip_with_document(xml)).unwrap();
        let xml = document_xml(&docx).unwrap();
        assert!(xml.contains(r#"<w:jc w:val="center"/>"#));
        assert!(!xml.contains(MARKER_PREFIX));
        verify_docx_image_layouts(html, &docx).unwrap();
        let restored =
            restore_html_image_layouts(&docx, r#"<p><img src="media/a.png"></p>"#).unwrap();
        assert!(restored.contains("text-align: center"));
    }
}
