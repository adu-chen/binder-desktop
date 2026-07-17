//! PI-DOC-02 · 样式子集三级执行契约 · DOC-CONVERT / BR-DOC-DATA-002
//!
//! 承接 A-DOC-M-T-01 §4：docx 进入编辑模式即按三级样式子集契约子集化——
//! - 必需子集（Required）：承诺往返保真，回归测试强制覆盖；
//! - 部分支持（Partial）：尽力保留，损失对用户**明示**（不静默丢失）；
//! - 明确不处理（Unsupported）：不承诺，由预览链（PRV）全保真补偿。
//! 三级清单是 A-DOC-M-T-01 §4 的 SSOT，**不得在转换代码中私自增减**；未知特性按 Unsupported 保守处理
//! （不静默当作保真）。Lua 过滤器与往返实证率为实现期（DOC-02，A 类）。

/// 样式特性的三级归属。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleTier {
    Required,    // 往返保真
    Partial,     // 尽力保留，损失明示
    Unsupported, // 不处理，预览链补偿
}

// 三级清单（A-DOC-M-T-01 §4 SSOT，与文档逐项对应）。
const REQUIRED: &[&str] = &[
    "heading",
    "paragraph_align",
    "bold",
    "italic",
    "underline",
    "ordered_list",
    "unordered_list",
    "basic_table",
    "image",
];
const PARTIAL: &[&str] = &["text_color", "font_family", "font_size"];
const UNSUPPORTED: &[&str] = &[
    "header_footer",
    "text_box",
    "smartart",
    "vba_macro",
    "complex_style_inheritance",
    "merged_cells",
];

/**
 * @GOV
 * codes:    BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-001
 * type:     DATA
 * chain:    DOC-CONVERT
 * rules:    BR-DOC-DATA-002
 * boundary: in=styleFeature | out=StyleTier(requiredFaithfulPartialLossyUnsupportedUnknownConservative)
 */
/// 样式特性归级（BR-DOC-DATA-002）：按 §4 SSOT 三级清单归类；**未注册特性保守归 Unsupported**
/// （不静默当作往返保真）。清单为 SSOT，本函数不私自增减。
pub fn classify_style(feature: &str) -> StyleTier {
    if REQUIRED.contains(&feature) {
        StyleTier::Required
    } else if PARTIAL.contains(&feature) {
        StyleTier::Partial
    } else if UNSUPPORTED.contains(&feature) {
        StyleTier::Unsupported
    } else {
        StyleTier::Unsupported // 未知 → 保守（不静默保真）
    }
}

/**
 * @GOV
 * codes:    BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-002
 * type:     DATA
 * chain:    DOC-CONVERT
 * rules:    BR-DOC-DATA-002
 * boundary: in=presentFeatures | out=lossNotice(nonRequiredSurfacedNeverSilent)
 */
/// 损失告知（BR-DOC-DATA-002）：对文档中出现的特性，凡非 Required（即 Partial/Unsupported）一律
/// 进入告知清单（用户可知，**不静默丢失**）；Required 不入清单（往返保真无须告知）。返回 (特性, 级别)。
pub fn loss_report(present_features: &[&str]) -> Vec<(String, StyleTier)> {
    present_features
        .iter()
        .map(|f| (f.to_string(), classify_style(f)))
        .filter(|(_, tier)| *tier != StyleTier::Required)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: BR-DOC-DATA-002
    // 三级归类正确；未知特性保守归 Unsupported（不静默当作保真）。
    #[test]
    fn classify_three_tiers_unknown_conservative() {
        assert_eq!(classify_style("heading"), StyleTier::Required);
        assert_eq!(classify_style("basic_table"), StyleTier::Required);
        assert_eq!(classify_style("font_size"), StyleTier::Partial);
        assert_eq!(classify_style("vba_macro"), StyleTier::Unsupported);
        assert_eq!(classify_style("merged_cells"), StyleTier::Unsupported);
        // 未知特性 → 保守 Unsupported
        assert_eq!(classify_style("某种未登记特效"), StyleTier::Unsupported);
    }

    // covers: BR-DOC-DATA-002
    // 损失告知：Partial/Unsupported 必入清单（不静默丢失）；Required 不入（往返保真）。
    #[test]
    fn loss_report_surfaces_all_non_required() {
        let present = [
            "heading",
            "bold",
            "font_size",
            "text_color",
            "smartart",
            "header_footer",
        ];
        let report = loss_report(&present);
        // Required（heading/bold）不入清单
        assert!(!report.iter().any(|(f, _)| f == "heading" || f == "bold"));
        // Partial（font_size/text_color）+ Unsupported（smartart/header_footer）全部告知
        assert_eq!(report.len(), 4);
        assert!(report
            .iter()
            .any(|(f, t)| f == "font_size" && *t == StyleTier::Partial));
        assert!(report
            .iter()
            .any(|(f, t)| f == "smartart" && *t == StyleTier::Unsupported));
        // 纯必需子集 → 无损失告知（往返保真）
        assert!(loss_report(&["heading", "bold", "image"]).is_empty());
    }
}
