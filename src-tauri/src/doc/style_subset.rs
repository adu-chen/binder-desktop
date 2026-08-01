//! PI-DOC-02 · 样式能力五档报告模型 · DOC-CONVERT / BR-DOC-DATA-002
//!
//! 承接 A-DOC-M-T-01 §4：docx 进入编辑模式即按三级样式子集契约子集化——
//! - 必需子集（Required）：承诺往返保真，回归测试强制覆盖；
//! - 部分支持（Partial）：尽力保留，损失对用户**明示**（不静默丢失）；
//! - 明确不处理（Unsupported）：不承诺，由预览链（PRV）全保真补偿。
//! 三级清单是 A-DOC-M-T-01 §4 的 SSOT，**不得在转换代码中私自增减**；未知特性按 Unsupported 保守处理
//! （不静默当作保真）。Lua 过滤器与往返实证率为实现期（DOC-02，A 类）。

use crate::doc::docx_cell_shading::CellShadingRecovery;
use crate::doc::docx_color::ColorRecovery;
use crate::doc::image_media::ImageRecovery;

/// 样式特性的五档归属。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StyleTier {
    Verified,          // 已实测保证
    RequiredPending,   // 必需待扩展
    BestEffort,        // 尽力保留
    TemplateInherited, // 仅模板继承
    Unsupported,       // 明确不处理
}

// 三级清单（A-DOC-M-T-01 §4 SSOT，与文档逐项对应）。
const VERIFIED: &[&str] = &[
    "heading",
    "bold",
    "text_color",
    "table_cell_shading",
    "image",
];
const REQUIRED_PENDING: &[&str] = &[
    "paragraph_align",
    "italic",
    "underline",
    "ordered_list",
    "unordered_list",
    "basic_table",
];
const BEST_EFFORT: &[&str] = &["font_family", "font_size", "custom_style"];
const TEMPLATE_INHERITED: &[&str] = &["page_margin", "page_size", "output_header_footer"];
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
    if VERIFIED.contains(&feature) {
        StyleTier::Verified
    } else if REQUIRED_PENDING.contains(&feature) {
        StyleTier::RequiredPending
    } else if BEST_EFFORT.contains(&feature) {
        StyleTier::BestEffort
    } else if TEMPLATE_INHERITED.contains(&feature) {
        StyleTier::TemplateInherited
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
        .filter(|(_, tier)| *tier != StyleTier::Verified)
        .collect()
}

/// 对当前已转换的编辑 HTML 做实际样式检测，并把颜色恢复结果一起暴露给调用方。
/// 仅报告文档中真实命中的特性；无法由 HTML 检测的 OOXML 高级对象仍由后续专项扩展。
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct StyleReport {
    pub detected_features: Vec<String>,
    pub verified: Vec<String>,
    pub required_pending: Vec<String>,
    pub best_effort: Vec<String>,
    pub template_inherited: Vec<String>,
    pub unsupported: Vec<String>,
    pub notices: Vec<String>,
    pub color_recovery: ColorRecovery,
    pub cell_shading_recovery: CellShadingRecovery,
    pub image_recovery: ImageRecovery,
}

fn has_style_property(html: &str, property: &str) -> bool {
    html.to_ascii_lowercase().contains(&format!("{property}:"))
}

fn detected_features(html: &str) -> Vec<&'static str> {
    let lower = html.to_ascii_lowercase();
    let mut found = Vec::new();
    if (1..=6).any(|level| lower.contains(&format!("<h{level}"))) {
        found.push("heading");
    }
    if lower.contains("<strong") || lower.contains("<b>") {
        found.push("bold");
    }
    if has_style_property(html, "color") {
        found.push("text_color");
    }
    if has_style_property(html, "font-family") {
        found.push("font_family");
    }
    if has_style_property(html, "font-size") {
        found.push("font_size");
    }
    if lower.contains("data-custom-style=") {
        found.push("custom_style");
    }
    if lower.contains("<img") {
        found.push("image");
    }
    if lower.contains("<table") {
        found.push("basic_table");
    }
    if lower.contains("data-binder-cell-shading=") {
        found.push("table_cell_shading");
    }
    found
}

/// 构建可通过 IPC 发送的五档样式报告。颜色找回失败会形成明确用户告知。
pub fn inspect_html(
    html: &str,
    color_recovery: ColorRecovery,
    cell_shading_recovery: CellShadingRecovery,
    image_recovery: ImageRecovery,
) -> StyleReport {
    let mut report = StyleReport {
        color_recovery,
        cell_shading_recovery,
        image_recovery,
        ..Default::default()
    };
    for feature in detected_features(html) {
        report.detected_features.push(feature.to_string());
        match classify_style(feature) {
            StyleTier::Verified => report.verified.push(feature.to_string()),
            StyleTier::RequiredPending => report.required_pending.push(feature.to_string()),
            StyleTier::BestEffort => report.best_effort.push(feature.to_string()),
            StyleTier::TemplateInherited => report.template_inherited.push(feature.to_string()),
            StyleTier::Unsupported => report.unsupported.push(feature.to_string()),
        }
    }
    if report.color_recovery.unrecovered_runs > 0 {
        report.notices.push(format!(
            "检测到 {} 个来源文字颜色片段，其中 {} 个未能映射到可编辑 HTML；这些片段可能以默认颜色显示。",
            report.color_recovery.detected_colored_runs, report.color_recovery.unrecovered_runs
        ));
    }
    if report.cell_shading_recovery.unrecovered_cells > 0 {
        report.notices.push(format!(
            "检测到 {} 个来源单元格底纹，其中 {} 个不属于纯色支持子集或未能映射到可编辑 HTML；这些单元格可能以默认底纹显示。",
            report.cell_shading_recovery.detected_shaded_cells,
            report.cell_shading_recovery.unrecovered_cells
        ));
    }
    if report.image_recovery.unrecovered_images > 0
        || report.image_recovery.unrecovered_sized_images > 0
        || !report.image_recovery.missing_resources.is_empty()
    {
        report.notices.push(format!(
            "检测到 {} 个来源图片实例，其中 {} 个图片或 {} 个尺寸未能恢复；缺失资源：{}。",
            report.image_recovery.detected_images,
            report.image_recovery.unrecovered_images,
            report.image_recovery.unrecovered_sized_images,
            if report.image_recovery.missing_resources.is_empty() {
                "无".to_string()
            } else {
                report.image_recovery.missing_resources.join("、")
            }
        ));
    }
    if !report.best_effort.is_empty()
        || !report.required_pending.is_empty()
        || !report.unsupported.is_empty()
    {
        report
            .notices
            .push("此文档含非已验证样式；保存前请以预览结果复核版式。".to_string());
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: BR-DOC-DATA-002
    // 三级归类正确；未知特性保守归 Unsupported（不静默当作保真）。
    #[test]
    fn classify_three_tiers_unknown_conservative() {
        assert_eq!(classify_style("heading"), StyleTier::Verified);
        assert_eq!(classify_style("basic_table"), StyleTier::RequiredPending);
        assert_eq!(classify_style("font_size"), StyleTier::BestEffort);
        assert_eq!(classify_style("text_color"), StyleTier::Verified);
        assert_eq!(classify_style("table_cell_shading"), StyleTier::Verified);
        assert_eq!(classify_style("image"), StyleTier::Verified);
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
        // Partial（font_size）+ Unsupported（smartart/header_footer）全部告知；颜色已具 OOXML 往返实证。
        assert_eq!(report.len(), 3);
        assert!(report
            .iter()
            .any(|(f, t)| f == "font_size" && *t == StyleTier::BestEffort));
        assert!(report
            .iter()
            .any(|(f, t)| f == "smartart" && *t == StyleTier::Unsupported));
        // 仅已实测保证项 → 无风险告知；图片尚属待扩展。
        assert!(loss_report(&["heading", "bold", "text_color"]).is_empty());
    }

    #[test]
    fn inspect_html_reports_detected_color_and_recovery_failure() {
        let report = inspect_html(
            r##"<p><span style="color:#FF0000">红字</span><span style="font-size:12px">小字</span></p><table><tr><td data-binder-cell-shading="#FFE699">内容</td></tr></table>"##,
            ColorRecovery {
                detected_colored_runs: 2,
                restored_runs: 1,
                unrecovered_runs: 1,
            },
            CellShadingRecovery {
                detected_shaded_cells: 2,
                restored_cells: 1,
                unrecovered_cells: 1,
            },
            ImageRecovery::default(),
        );
        assert!(report.verified.contains(&"text_color".to_string()));
        assert!(report.verified.contains(&"table_cell_shading".to_string()));
        assert!(report.best_effort.contains(&"font_size".to_string()));
        assert!(report
            .notices
            .iter()
            .any(|notice| notice.contains("未能映射")));
        assert!(report
            .notices
            .iter()
            .any(|notice| notice.contains("来源单元格底纹")));
    }
}
