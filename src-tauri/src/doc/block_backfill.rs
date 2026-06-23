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
    let t = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
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
            if calls.get() == 1 { "x".to_string() } else { "y".to_string() }
        };
        let out = backfill_block_ids(&[Some("x".to_string()), None], gen);
        assert_eq!(out[0], "x");
        assert_eq!(out[1], "y"); // 跳过冲突的 "x"
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
