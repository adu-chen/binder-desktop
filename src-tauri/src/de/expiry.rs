//! PI-DE-03 · 过期保守判定与多 diff 冲突重检 · DE-DIFF / BR-DE-STATE-002 / BR-DE-DATA-003
//!
//! 承接 A-DE-M-T-01 §6/§7：
//! - 过期（BR-DE-STATE-002）：超时或目标内容偏离 before_snapshot 转 Expired；外部变更**检测不可得
//!   时保守保持 Pending**（不误杀待裁决 diff）。
//! - 冲突重检（BR-DE-DATA-003）：先序 diff 落盘后，与其目标重叠的 pending diff 须重检；内容已变
//!   （before_snapshot 不匹配当前）转 Expired，不静默用陈旧 after_content 覆盖新内容。
//! 超时时长与检测触发时机为实现期调度（DE-03，A 类）。

use std::collections::HashSet;

/// 外部变更检测结果。`Undetectable` = 检测不可得（如磁盘读失败），触发保守语义。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalCheck {
    Unchanged,
    Changed,
    Undetectable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpiryReason {
    Timeout,
    ExternalChange,
}

/// 过期判定结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpiryDecision {
    Expire(ExpiryReason),
    KeepPending,
}

/**
 * @GOV
 * codes:    BR-DE-STATE-002-GUARD-DE-DE-DIFF-001
 * type:     GUARD
 * chain:    DE-DIFF
 * rules:    BR-DE-STATE-002
 * boundary: in=elapsedAndExternalCheck | out=ExpiryDecision(conservativeWhenUndetectable)
 */
/// 过期判定（BR-DE-STATE-002）：外部已变 → Expire(ExternalChange)；否则超时 → Expire(Timeout)；
/// 外部变更**检测不可得（Undetectable）不因此过期**（保守保持 Pending），但超时仍独立生效。
pub fn check_expiry(
    elapsed_ms: i64,
    timeout_ms: i64,
    external: ExternalCheck,
) -> ExpiryDecision {
    match external {
        ExternalCheck::Changed => ExpiryDecision::Expire(ExpiryReason::ExternalChange),
        ExternalCheck::Unchanged | ExternalCheck::Undetectable => {
            if elapsed_ms >= timeout_ms {
                ExpiryDecision::Expire(ExpiryReason::Timeout)
            } else {
                ExpiryDecision::KeepPending // Undetectable 不触发外部变更过期（不误杀）
            }
        }
    }
}

fn blocks_overlap(a: &[String], b: &[String]) -> bool {
    let set: HashSet<&String> = a.iter().collect();
    b.iter().any(|x| set.contains(x))
}

/**
 * @GOV
 * codes:    BR-DE-DATA-003-GUARD-DE-DE-DIFF-001
 * type:     GUARD
 * chain:    DE-DIFF
 * rules:    BR-DE-DATA-003
 * boundary: in=appliedVsPendingBlocks | out=ExpiryDecision(noStaleOverwrite)
 */
/// 先序 diff 落盘后的重叠重检（BR-DE-DATA-003）：与已落盘 diff 目标块重叠的 pending diff，
/// 若其 before_snapshot 哈希不再匹配目标当前哈希（内容已被先序落盘改变）→ Expire，不以陈旧
/// after_content 覆盖新内容；不重叠或内容未变则保持 Pending。
pub fn recheck_overlap(
    applied_blocks: &[String],
    pending_blocks: &[String],
    pending_before_hash: u64,
    current_target_hash: u64,
) -> ExpiryDecision {
    if blocks_overlap(applied_blocks, pending_blocks) && pending_before_hash != current_target_hash {
        ExpiryDecision::Expire(ExpiryReason::ExternalChange)
    } else {
        ExpiryDecision::KeepPending
    }
}

#[cfg(test)]
mod tests {
    use super::ExpiryDecision::*;
    use super::ExpiryReason::*;
    use super::*;

    // covers: BR-DE-STATE-002
    // 超时/外部变更转 Expired；检测不可得不误杀（保持 Pending），但超时独立生效。
    #[test]
    fn expiry_is_conservative_when_undetectable() {
        // 外部已变 → 过期
        assert_eq!(check_expiry(10, 1000, ExternalCheck::Changed), Expire(ExternalChange));
        // 超时 → 过期
        assert_eq!(check_expiry(2000, 1000, ExternalCheck::Unchanged), Expire(Timeout));
        // 未变且未超时 → 保持
        assert_eq!(check_expiry(10, 1000, ExternalCheck::Unchanged), KeepPending);
        // 检测不可得 + 未超时 → 保守保持（不误杀）
        assert_eq!(check_expiry(10, 1000, ExternalCheck::Undetectable), KeepPending);
        // 检测不可得 + 已超时 → 超时仍独立过期
        assert_eq!(check_expiry(2000, 1000, ExternalCheck::Undetectable), Expire(Timeout));
    }

    // covers: BR-DE-DATA-003
    // 先序落盘后：重叠且内容已变 → Expire（不覆盖新内容）；不重叠或内容未变 → 保持。
    #[test]
    fn overlap_recheck_prevents_stale_overwrite() {
        let applied = vec!["b1".to_string(), "b2".to_string()];
        // 重叠（b2）且内容已变（hash 不同）→ 过期，不覆盖
        assert_eq!(
            recheck_overlap(&applied, &["b2".into(), "b3".into()], 100, 200),
            Expire(ExternalChange)
        );
        // 不重叠 → 保持（互不影响可独立落盘）
        assert_eq!(
            recheck_overlap(&applied, &["b9".into()], 100, 200),
            KeepPending
        );
        // 重叠但内容未变（hash 相同）→ 保持（仍可应用）
        assert_eq!(
            recheck_overlap(&applied, &["b2".into()], 100, 100),
            KeepPending
        );
    }
}
