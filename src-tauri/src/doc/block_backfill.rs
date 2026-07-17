//! PI-DOC-03 · 转换层 blockId 兜底 · DOC-CONVERT / BR-DOC-DATA-003
//!
//! 承接 A-DOC-M-T-01：docx→编辑模型转换须为**缺失 data-block-id 的块级元素注入 blockId 兜底**（生成规则
//! 与 ED blockId 机制一致：唯一、不复用），保障上层锚定不因转换缺锚；编辑模型→docx **尽力保留**
//! data-block-id；**往返保留度不达标时显式采用「重建锚」降级**（回写本文与 A-ED-M-T-01），不得静默让锚漂移。
//! id 生成算法与 ED 对齐（占位实现，实现期换 ULID；A 类）。

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static SEQ: AtomicU64 = AtomicU64::new(0);

fn gen_block_id() -> String {
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("b_{:x}_{}", t, SEQ.fetch_add(1, Ordering::Relaxed))
}

/**
 * @GOV
 * codes:    BR-DOC-DATA-003-DATA-DOC-DOC-CONVERT-001
 * type:     DATA
 * chain:    DOC-CONVERT
 * rules:    BR-DOC-DATA-003
 * boundary: in=convertedBlockIds | out=completeBlockIds(missingInjectedExistingKeptUniqueNoReuse)
 */
/// blockId 兜底（BR-DOC-DATA-003）：为每个块产出确定 blockId——既有且唯一者保留；缺失（None）或
/// 与前序重复者注入新唯一 id（与 ED 机制一致：不复用、全文唯一）。保障转换后上层锚定不缺锚/不冲突。
pub fn backfill_block_ids(ids: &[Option<String>], gen: impl Fn() -> String) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        match id {
            Some(existing) if !seen.contains(existing) => {
                seen.insert(existing.clone());
                out.push(existing.clone()); // 既有且唯一 → 保留
            }
            _ => {
                let mut nid = gen();
                while seen.contains(&nid) {
                    nid = gen(); // 不复用/不冲突
                }
                seen.insert(nid.clone());
                out.push(nid); // 缺失或重复 → 注入新 id
            }
        }
    }
    out
}

/// 默认 id 生成器的兜底（生产路径）。
pub fn backfill_default(ids: &[Option<String>]) -> Vec<String> {
    backfill_block_ids(ids, gen_block_id)
}

const BLOCK_TAGS: &[&str] = &[
    "p",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "blockquote",
    "li",
    "pre",
    "div",
    "table",
];

#[derive(Debug)]
struct HtmlBlockSlot {
    insert_pos: usize,
    value_range: Option<std::ops::Range<usize>>,
    id: Option<String>,
}

fn find_tag_end(html: &str, open_pos: usize) -> Option<usize> {
    let mut quote: Option<char> = None;
    for (offset, ch) in html[open_pos..].char_indices() {
        match quote {
            Some(q) if ch == q => quote = None,
            Some(_) => {}
            None if ch == '"' || ch == '\'' => quote = Some(ch),
            None if ch == '>' => return Some(open_pos + offset),
            None => {}
        }
    }
    None
}

fn find_name_end(html: &str, name_start: usize, tag_end: usize) -> usize {
    html[name_start..tag_end]
        .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
        .map(|p| name_start + p)
        .unwrap_or(tag_end)
}

fn attr_value_range(
    html: &str,
    attrs_start: usize,
    attrs_end: usize,
    attr_name: &str,
) -> Option<(std::ops::Range<usize>, String)> {
    let attrs = &html[attrs_start..attrs_end];
    let lower = attrs.to_ascii_lowercase();
    let mut search = 0usize;
    while let Some(rel) = lower[search..].find(attr_name) {
        let attr_pos = search + rel;
        let before_ok = attr_pos == 0
            || !lower.as_bytes()[attr_pos - 1].is_ascii_alphanumeric()
                && lower.as_bytes()[attr_pos - 1] != b'-';
        let after_name = attr_pos + attr_name.len();
        let after_ok = after_name >= lower.len()
            || lower.as_bytes()[after_name].is_ascii_whitespace()
            || lower.as_bytes()[after_name] == b'=';
        if !before_ok || !after_ok {
            search = after_name;
            continue;
        }
        let mut i = attrs_start + after_name;
        while i < attrs_end && html.as_bytes()[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= attrs_end || html.as_bytes()[i] != b'=' {
            return None;
        }
        i += 1;
        while i < attrs_end && html.as_bytes()[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= attrs_end {
            return None;
        }
        let quote = html.as_bytes()[i];
        if quote != b'"' && quote != b'\'' {
            return None;
        }
        let value_start = i + 1;
        let value_end = html[value_start..attrs_end]
            .find(quote as char)
            .map(|p| value_start + p)?;
        return Some((
            value_start..value_end,
            html[value_start..value_end].to_string(),
        ));
    }
    None
}

fn insertion_pos(html: &str, tag_end: usize) -> usize {
    let mut i = tag_end;
    while i > 0 && html.as_bytes()[i - 1].is_ascii_whitespace() {
        i -= 1;
    }
    if i > 0 && html.as_bytes()[i - 1] == b'/' {
        i - 1
    } else {
        tag_end
    }
}

fn collect_html_block_slots(html: &str) -> Vec<HtmlBlockSlot> {
    let mut slots = Vec::new();
    let mut search = 0usize;
    while let Some(rel) = html[search..].find('<') {
        let open = search + rel;
        let name_start = open + 1;
        let Some(first) = html[name_start..].chars().next() else {
            break;
        };
        if first == '/' || first == '!' || first == '?' {
            search = name_start + first.len_utf8();
            continue;
        }
        let Some(tag_end) = find_tag_end(html, open) else {
            break;
        };
        let tag_name_end = find_name_end(html, name_start, tag_end);
        let tag = html[name_start..tag_name_end].to_ascii_lowercase();
        if !BLOCK_TAGS.contains(&tag.as_str()) {
            search = tag_end + 1;
            continue;
        }
        let attr = attr_value_range(html, tag_name_end, tag_end, "data-block-id");
        let (value_range, id) = match attr {
            Some((range, value)) => (Some(range), Some(value)),
            None => (None, None),
        };
        slots.push(HtmlBlockSlot {
            insert_pos: insertion_pos(html, tag_end),
            value_range,
            id,
        });
        search = tag_end + 1;
    }
    slots
}

/**
 * @GOV
 * codes:    BR-DOC-DATA-003-DATA-DOC-DOC-CONVERT-003
 * type:     DATA
 * chain:    DOC-CONVERT
 * rules:    BR-DOC-DATA-003
 * boundary: in=editableHtmlBlockTags | out=editableHtmlWithCompleteStableBlockIds
 */
/// HTML blockId 注入守则（BR-DOC-DATA-003）：docx→HTML 后进入 ED/AG/DE 前，所有块级元素必须拥有
/// 唯一 data-block-id；已有唯一 id 保留，缺失或重复者补新 id，避免 BDocx diff 锚链缺锚/漂移。
pub fn inject_missing_data_block_ids(html: &str) -> String {
    let slots = collect_html_block_slots(html);
    if slots.is_empty() {
        return html.to_string();
    }
    let ids: Vec<Option<String>> = slots.iter().map(|slot| slot.id.clone()).collect();
    let resolved = backfill_default(&ids);
    let mut out = String::with_capacity(html.len() + slots.len() * 32);
    let mut last = 0usize;
    for (slot, wanted) in slots.iter().zip(resolved.iter()) {
        match &slot.value_range {
            Some(range) if slot.id.as_deref() != Some(wanted.as_str()) => {
                out.push_str(&html[last..range.start]);
                out.push_str(wanted);
                last = range.end;
            }
            Some(_) => {}
            None => {
                out.push_str(&html[last..slot.insert_pos]);
                out.push_str(" data-block-id=\"");
                out.push_str(wanted);
                out.push('"');
                last = slot.insert_pos;
            }
        }
    }
    out.push_str(&html[last..]);
    out
}

/// 往返锚保留决策。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorDecision {
    Preserve,       // 保留度达标，沿用既有锚
    RebuildAnchors, // 不达标 → 显式重建锚降级（不静默漂移）
}

/**
 * @GOV
 * codes:    BR-DOC-DATA-003-GUARD-DOC-DOC-CONVERT-002
 * type:     GUARD
 * chain:    DOC-CONVERT
 * rules:    BR-DOC-DATA-003
 * boundary: in=roundtripRetention | out=AnchorDecision(rebuildWhenBelowThresholdNoSilentDrift)
 */
/// 往返保留度评估（BR-DOC-DATA-003）：保留率（preserved/total）≥ 阈值 → Preserve；**低于阈值 →
/// RebuildAnchors（显式重建锚降级，不静默让锚漂移）**；total=0 → Preserve（无锚可漂移）。
pub fn assess_roundtrip(total: usize, preserved: usize, min_retention: f64) -> AnchorDecision {
    if total == 0 {
        return AnchorDecision::Preserve;
    }
    let rate = preserved as f64 / total as f64;
    if rate >= min_retention {
        AnchorDecision::Preserve
    } else {
        AnchorDecision::RebuildAnchors // 显式降级
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seq_gen() -> impl Fn() -> String {
        let n = std::cell::Cell::new(0);
        move || {
            n.set(n.get() + 1);
            format!("n{}", n.get())
        }
    }

    // covers: BR-DOC-DATA-003
    // 兜底：缺失注入、既有保留、重复改取；全唯一不复用。
    #[test]
    fn backfill_injects_missing_keeps_existing_dedups() {
        let ids = vec![Some("a".to_string()), None, Some("a".to_string()), None];
        let out = backfill_block_ids(&ids, seq_gen());
        assert_eq!(out[0], "a"); // 既有保留
        assert_eq!(out[1], "n1"); // 缺失注入
        assert_eq!(out[2], "n2"); // 重复的 "a" → 改取新
        assert_eq!(out[3], "n3"); // 缺失注入
                                  // 全唯一
        let uniq: std::collections::HashSet<_> = out.iter().collect();
        assert_eq!(uniq.len(), out.len());
    }

    // covers: BR-DOC-DATA-003
    // 生成 id 与既有冲突时跳过重取（不复用）。
    #[test]
    fn backfill_avoids_collision_with_existing() {
        // gen 先吐 "x"（与既有冲突）应跳过
        let calls = std::cell::Cell::new(0);
        let gen = || {
            calls.set(calls.get() + 1);
            if calls.get() == 1 {
                "x".to_string()
            } else {
                "y".to_string()
            }
        };
        let out = backfill_block_ids(&[Some("x".to_string()), None], gen);
        assert_eq!(out[0], "x");
        assert_eq!(out[1], "y"); // 跳过冲突的 "x"
    }

    // covers: BR-DOC-DATA-003
    // HTML 注入：块级标签补齐 data-block-id，已有唯一 id 保留，重复 id 改取新 id。
    #[test]
    fn inject_missing_data_block_ids_adds_and_dedups_block_tags() {
        let html = r#"<p>甲</p><span>内联</span><h1 data-block-id="keep">题</h1><p data-block-id="keep">乙</p>"#;
        let out = inject_missing_data_block_ids(html);
        assert!(out.contains("<p data-block-id=\"b_"));
        assert!(out.contains("<span>内联</span>"));
        assert!(out.contains("<h1 data-block-id=\"keep\">题</h1>"));
        assert!(out.contains(">乙</p>"));
        assert_eq!(out.matches("data-block-id=\"keep\"").count(), 1);
    }

    // covers: BR-DOC-DATA-003
    // HTML 注入：已有唯一 id 不改，空 HTML/无块级标签保持稳定。
    #[test]
    fn inject_missing_data_block_ids_keeps_unique_existing_ids() {
        let html = r#"<p data-block-id="a">甲</p><p data-block-id="b">乙</p>"#;
        assert_eq!(inject_missing_data_block_ids(html), html);
        assert_eq!(
            inject_missing_data_block_ids("<span>内联</span>"),
            "<span>内联</span>"
        );
    }

    // covers: BR-DOC-DATA-003
    // 往返保留度：达标 Preserve；不达标 RebuildAnchors（显式降级）；空集 Preserve。
    #[test]
    fn roundtrip_below_threshold_rebuilds() {
        assert_eq!(assess_roundtrip(10, 9, 0.8), AnchorDecision::Preserve); // 90% ≥ 80%
        assert_eq!(assess_roundtrip(10, 5, 0.8), AnchorDecision::RebuildAnchors); // 50% < 80%
        assert_eq!(assess_roundtrip(0, 0, 0.8), AnchorDecision::Preserve); // 无锚
    }
}
