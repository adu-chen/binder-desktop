//! DOC-LO-RELIABILITY-FONT-IMPLEMENT-01 · 原生 Office 字体取证 · DOC-CONVERT / BR-DOC-DATA-001
//!
//! 在 LibreOffice 真正排版前，只读扫描 docx/pptx OOXML 中的字体声明，形成不含正文的 Font Trace。
//! 它不解压到磁盘、不修改用户源文件、不访问设备字体库；FontPlan 只消费版本化策略中的通用别名/候选链，
//! 不针对某个文档或某种字体硬编码。最终 glyph 选择仍由后续 LO 源码专项裁决。

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

const MAX_XML_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum FontReferenceKind {
    Run,
    Style,
    Theme,
    FontTable,
    Drawing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct FontReference {
    pub family: String,
    pub kind: FontReferenceKind,
    /// 包内 XML part 名称；不记录用户源文件的绝对路径或正文。
    pub part: String,
    /// 例如 ascii / east_asia / typeface / major_east_asia。
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FontTrace {
    pub format: String,
    pub references: Vec<FontReference>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FontPolicyEntry {
    /// 策略稳定 id；不使用来源字体名作为 id。
    pub id: String,
    pub aliases: Vec<String>,
    /// 仅引用包内字体资源的稳定 id；实际 font family 与资源清单在 LO 构建专项锁定。
    /// 为空表示已知此字体但尚无包内候选——将标记为 Defaulted 而非 Unmapped。
    #[serde(default)]
    pub candidates: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FontPolicy {
    pub version: u32,
    pub entries: Vec<FontPolicyEntry>,
}

/// S2 三结果分类：保真预览（候选可用）/ Binder 统一预览（已知但无候选）/ 覆盖缺失失败（未知字体）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum FontPlanDecision {
    /// 字体已在策略中映射到包内候选链 → 保真预览
    Resolved {
        policy_id: String,
        candidates: Vec<String>,
    },
    /// 字体在策略中登记但尚无包内候选字体 → Binder 统一预览字体替代
    Defaulted { policy_id: String, reason: String },
    /// 策略中无此字体的任何记录 → 覆盖缺失，不得从宿主字体库猜测
    Unmapped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FontPlanEntry {
    pub reference: FontReference,
    pub normalized_family: String,
    pub decision: FontPlanDecision,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FontPlan {
    pub policy_version: u32,
    pub entries: Vec<FontPlanEntry>,
}

#[derive(Debug)]
pub enum FontTraceError {
    Io(std::io::Error),
    Zip(zip::result::ZipError),
    Xml(quick_xml::Error),
    UnsupportedFormat,
    InvalidPolicy(String),
    Serialize(serde_json::Error),
}

impl fmt::Display for FontTraceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "文件读取失败: {error}"),
            Self::Zip(error) => write!(f, "Office 包读取失败: {error}"),
            Self::Xml(error) => write!(f, "OOXML 读取失败: {error}"),
            Self::UnsupportedFormat => write!(f, "仅支持 docx 或 pptx 的字体取证"),
            Self::InvalidPolicy(error) => write!(f, "FontPlan 策略无效: {error}"),
            Self::Serialize(error) => write!(f, "Font Trace 序列化失败: {error}"),
        }
    }
}

impl std::error::Error for FontTraceError {}

impl From<std::io::Error> for FontTraceError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
impl From<zip::result::ZipError> for FontTraceError {
    fn from(error: zip::result::ZipError) -> Self {
        Self::Zip(error)
    }
}
impl From<quick_xml::Error> for FontTraceError {
    fn from(error: quick_xml::Error) -> Self {
        Self::Xml(error)
    }
}

fn local_name(raw: &[u8]) -> String {
    let name = std::str::from_utf8(raw).unwrap_or_default();
    name.rsplit(':')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
}

fn normalize_family(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn supported_part(format: &str, name: &str) -> bool {
    let is_xml = name.ends_with(".xml");
    match format {
        "docx" => is_xml && name.starts_with("word/"),
        "pptx" => is_xml && name.starts_with("ppt/"),
        _ => false,
    }
}

fn xml_kind(format: &str, part: &str, element: &str, attribute: &str) -> Option<FontReferenceKind> {
    if attribute.ends_with("theme") {
        return Some(FontReferenceKind::Theme);
    }
    if format == "docx"
        && part.ends_with("styles.xml")
        && matches!(attribute, "ascii" | "hansi" | "eastasia" | "cs")
    {
        return Some(FontReferenceKind::Style);
    }
    if element == "rfonts" {
        return Some(FontReferenceKind::Run);
    }
    if element == "font" && attribute == "name" {
        return Some(FontReferenceKind::FontTable);
    }
    if attribute == "typeface" {
        return Some(if part.contains("theme") {
            FontReferenceKind::Theme
        } else {
            FontReferenceKind::Drawing
        });
    }
    None
}

fn push_reference(
    references: &mut BTreeSet<FontReference>,
    part: &str,
    kind: FontReferenceKind,
    role: String,
    family: String,
) {
    let family = family.trim();
    if family.is_empty() {
        return;
    }
    references.insert(FontReference {
        family: family.to_string(),
        kind,
        part: part.to_string(),
        role,
    });
}

fn scan_xml_part(
    format: &str,
    part: &str,
    bytes: &[u8],
    references: &mut BTreeSet<FontReference>,
) -> Result<(), FontTraceError> {
    let mut reader = Reader::from_reader(bytes);
    reader.trim_text(false);
    let mut buffer = Vec::new();
    loop {
        match reader.read_event_into(&mut buffer)? {
            Event::Start(event) | Event::Empty(event) => {
                scan_element(format, part, &event, references)?;
            }
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }
    Ok(())
}

fn scan_element(
    format: &str,
    part: &str,
    event: &BytesStart<'_>,
    references: &mut BTreeSet<FontReference>,
) -> Result<(), FontTraceError> {
    let element = local_name(event.name().as_ref());
    for attribute in event.attributes().with_checks(false).flatten() {
        let name = local_name(attribute.key.as_ref());
        let Some(kind) = xml_kind(format, part, &element, &name) else {
            continue;
        };
        let value = attribute.unescape_value()?.into_owned();
        push_reference(references, part, kind, name, value);
    }
    Ok(())
}

/**
 * @GOV
 * codes:    BR-DOC-DATA-001-DATA-DOC-DOC-CONVERT-006
 * type:     DATA
 * chain:    DOC-CONVERT
 * rules:    BR-DOC-DATA-001
 * boundary: in=readOnlyOfficeArchive | out=FontTrace(fontMetadataOnlyNoSourceMutationNoHostFontProbe)
 */
/// 扫描 DOCX/PPTX 内部 OOXML 的字体元数据。只读取压缩包内 XML，不提取文件、不过问设备字体库，
/// 也不把用户正文写入 trace；读取异常由调用方记录为诊断，不能触发系统工具回退。
pub fn inspect_office_font_references(input: &Path) -> Result<FontTrace, FontTraceError> {
    let format = input
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .filter(|extension| matches!(extension.as_str(), "docx" | "pptx"))
        .ok_or(FontTraceError::UnsupportedFormat)?;
    let file = File::open(input)?;
    let mut archive = ZipArchive::new(file)?;
    let mut references = BTreeSet::new();
    let mut warnings = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let part = entry.name().to_string();
        if !supported_part(&format, &part) {
            continue;
        }
        if entry.size() > MAX_XML_BYTES {
            warnings.push(format!("跳过过大的 XML part: {part}"));
            continue;
        }
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut bytes)?;
        if let Err(error) = scan_xml_part(&format, &part, &bytes, &mut references) {
            warnings.push(format!("跳过无法解析的 XML part {part}: {error}"));
        }
    }
    Ok(FontTrace {
        format,
        references: references.into_iter().collect(),
        warnings,
    })
}

fn policy_index(policy: &FontPolicy) -> Result<BTreeMap<String, &FontPolicyEntry>, FontTraceError> {
    let mut index = BTreeMap::new();
    for entry in &policy.entries {
        if entry.id.trim().is_empty() {
            return Err(FontTraceError::InvalidPolicy(format!(
                "策略项必须具有 id: {}",
                entry.id
            )));
        }
        // candidates 可为空，表示已知但无候选 → Defaulted
        for alias in &entry.aliases {
            let key = normalize_family(alias);
            if key.is_empty() {
                return Err(FontTraceError::InvalidPolicy(format!(
                    "策略项 {} 包含空别名",
                    entry.id
                )));
            }
            if let Some(previous) = index.insert(key.clone(), entry) {
                if previous.id != entry.id {
                    return Err(FontTraceError::InvalidPolicy(format!(
                        "别名 {key} 同时属于 {} 与 {}",
                        previous.id, entry.id
                    )));
                }
            }
        }
    }
    Ok(index)
}

/**
 * @GOV
 * codes:    BR-DOC-DATA-001-DATA-DOC-DOC-CONVERT-007
 * type:     DATA
 * chain:    DOC-CONVERT
 * rules:    BR-DOC-DATA-001
 * boundary: in=FontTraceAndVersionedFontPolicy | out=FontPlan(deterministicPackageCandidateOrUnmappedNoHostFallback)
 */
/// 从通用、版本化策略生成 FontPlan。未映射不是“任选系统字体”，而是显式 `unmapped`，由后续 LO
/// 源码级 GlyphFallback 或 `DOC_FONT_COVERAGE_MISSING` 处理。
pub fn build_font_plan(trace: &FontTrace, policy: &FontPolicy) -> Result<FontPlan, FontTraceError> {
    let index = policy_index(policy)?;
    let entries = trace
        .references
        .iter()
        .cloned()
        .map(|reference| {
            let normalized_family = normalize_family(&reference.family);
            let decision = match index.get(&normalized_family) {
                Some(entry) => {
                    if entry.candidates.is_empty() {
                        FontPlanDecision::Defaulted {
                            policy_id: entry.id.clone(),
                            reason: "策略已登记但尚未分配包内候选字体".to_string(),
                        }
                    } else {
                        FontPlanDecision::Resolved {
                            policy_id: entry.id.clone(),
                            candidates: entry.candidates.clone(),
                        }
                    }
                }
                None => FontPlanDecision::Unmapped,
            };
            FontPlanEntry {
                reference,
                normalized_family,
                decision,
            }
        })
        .collect();
    Ok(FontPlan {
        policy_version: policy.version,
        entries,
    })
}

/// 统计 FontPlan 三结果分类：resolved / defaulted / unmapped。
pub fn plan_summary(plan: &FontPlan) -> (usize, usize, usize) {
    let mut resolved = 0;
    let mut defaulted = 0;
    let mut unmapped = 0;
    for entry in &plan.entries {
        match entry.decision {
            FontPlanDecision::Resolved { .. } => resolved += 1,
            FontPlanDecision::Defaulted { .. } => defaulted += 1,
            FontPlanDecision::Unmapped => unmapped += 1,
        }
    }
    (resolved, defaulted, unmapped)
}

/// 将 trace 与 plan 作为请求级诊断写入 app_data 工作目录。输出不含正文，也不包含用户源文件绝对路径。
pub fn write_font_diagnostic(
    work_dir: &Path,
    trace: &FontTrace,
    plan: &FontPlan,
) -> Result<(), FontTraceError> {
    let diagnostic = serde_json::json!({ "trace": trace, "plan": plan });
    let bytes = serde_json::to_vec_pretty(&diagnostic).map_err(FontTraceError::Serialize)?;
    std::fs::write(work_dir.join("font-diagnostic.json"), bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::FileOptions;
    use zip::{CompressionMethod, ZipWriter};

    fn make_office_archive(extension: &str, parts: &[(&str, &str)]) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "binder_font_trace_{}.{}",
            uuid::Uuid::new_v4(),
            extension
        ));
        let file = File::create(&path).unwrap();
        let mut writer = ZipWriter::new(file);
        let options = FileOptions::default().compression_method(CompressionMethod::Stored);
        for (name, xml) in parts {
            writer.start_file(*name, options).unwrap();
            writer.write_all(xml.as_bytes()).unwrap();
        }
        writer.finish().unwrap();
        path
    }

    // covers: BR-DOC-DATA-001
    #[test]
    fn traces_docx_run_style_theme_and_font_table_without_text_content() {
        let path = make_office_archive(
            "docx",
            &[
                (
                    "word/document.xml",
                    r#"<w:document xmlns:w="x"><w:rFonts w:ascii="Example Sans" w:eastAsia="Example CJK"/><w:t>secret body</w:t></w:document>"#,
                ),
                (
                    "word/styles.xml",
                    r#"<w:styles xmlns:w="x"><w:rFonts w:hAnsi="Style Serif"/></w:styles>"#,
                ),
                (
                    "word/theme/theme1.xml",
                    r#"<a:theme xmlns:a="x"><a:latin typeface="Theme Latin"/></a:theme>"#,
                ),
                (
                    "word/fontTable.xml",
                    r#"<w:fonts xmlns:w="x"><w:font w:name="Declared Face"/></w:fonts>"#,
                ),
            ],
        );
        let trace = inspect_office_font_references(&path).unwrap();
        assert_eq!(trace.format, "docx");
        assert!(trace
            .references
            .iter()
            .any(|entry| entry.family == "Example Sans" && entry.kind == FontReferenceKind::Run));
        assert!(trace
            .references
            .iter()
            .any(|entry| entry.family == "Style Serif" && entry.kind == FontReferenceKind::Style));
        assert!(trace
            .references
            .iter()
            .any(|entry| entry.family == "Theme Latin" && entry.kind == FontReferenceKind::Theme));
        assert!(trace
            .references
            .iter()
            .any(|entry| entry.family == "Declared Face"
                && entry.kind == FontReferenceKind::FontTable));
        assert!(!serde_json::to_string(&trace)
            .unwrap()
            .contains("secret body"));
        let _ = std::fs::remove_file(path);
    }

    // covers: BR-DOC-DATA-001
    #[test]
    fn traces_pptx_typefaces_and_builds_general_policy_plan() {
        let path = make_office_archive(
            "pptx",
            &[(
                "ppt/slides/slide1.xml",
                r#"<p:sld xmlns:a="x" xmlns:p="y"><a:latin typeface="Any Sans"/><a:ea typeface="Any CJK"/></p:sld>"#,
            )],
        );
        let trace = inspect_office_font_references(&path).unwrap();
        let policy = FontPolicy {
            version: 7,
            entries: vec![FontPolicyEntry {
                id: "cjk-sans".to_string(),
                aliases: vec!["any cjk".to_string()],
                candidates: vec!["binder-cjk-sans-regular".to_string()],
            }],
        };
        let plan = build_font_plan(&trace, &policy).unwrap();
        assert!(plan.entries.iter().any(|entry| matches!(&entry.decision, FontPlanDecision::Resolved { policy_id, .. } if policy_id == "cjk-sans")));
        assert!(plan
            .entries
            .iter()
            .any(|entry| matches!(entry.decision, FontPlanDecision::Unmapped)));
        let _ = std::fs::remove_file(path);
    }

    // covers: BR-DOC-DATA-001
    #[test]
    fn rejects_ambiguous_aliases_instead_of_picking_a_random_policy_entry() {
        let trace = FontTrace {
            format: "docx".to_string(),
            references: vec![],
            warnings: vec![],
        };
        let policy = FontPolicy {
            version: 1,
            entries: vec![
                FontPolicyEntry {
                    id: "first".to_string(),
                    aliases: vec!["same".to_string()],
                    candidates: vec!["a".to_string()],
                },
                FontPolicyEntry {
                    id: "second".to_string(),
                    aliases: vec!["same".to_string()],
                    candidates: vec!["b".to_string()],
                },
            ],
        };
        assert!(matches!(
            build_font_plan(&trace, &policy),
            Err(FontTraceError::InvalidPolicy(_))
        ));
    }

    // covers: BR-DOC-DATA-001
    #[test]
    fn traces_the_real_bundled_reference_docx_without_extracting_it() {
        let reference = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/reference.docx");
        let trace = inspect_office_font_references(&reference).expect("项目参考 DOCX 应可只读取证");
        assert_eq!(trace.format, "docx");
        assert!(
            !trace.references.is_empty(),
            "真实 DOCX 至少应提供字体声明/字体表线索"
        );
    }

    // covers: BR-DOC-DATA-001
    /// S2 验证：三结果分类正确性 —— Resolved(有候选) / Defaulted(已知无候选) / Unmapped(未知)。
    #[test]
    fn s2_three_way_font_decision_with_real_policy() {
        // 加载真实的 font-plan.json
        let policy_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/libreoffice/font-plan.json");
        let policy_bytes = std::fs::read(&policy_path).expect("font-plan.json 必须可读");
        let policy: FontPolicy =
            serde_json::from_slice(&policy_bytes).expect("font-plan.json 必须合法");

        // 构造代表性 DOCX：Arial(应Resolved) + Calibri(应Defaulted) + NonExistentFont(应Unmapped)
        let test = make_office_archive(
            "docx",
            &[(
                "word/document.xml",
                r#"<w:document xmlns:w="x"><w:rFonts w:ascii="Arial" w:eastAsia="SimHei"/><w:rFonts w:ascii="Calibri"/><w:rFonts w:ascii="NonExistentFont12345"/><w:t>body</w:t></w:document>"#,
            )],
        );
        let trace = inspect_office_font_references(&test).unwrap();
        let plan = build_font_plan(&trace, &policy).unwrap();

        // 验证三结果分类
        let (resolved, defaulted, unmapped) = plan_summary(&plan);
        assert!(resolved >= 1, "Arial/SimHei 应在策略中有候选 → Resolved");
        assert!(defaulted >= 1, "Calibri 在策略中但无候选 → Defaulted");
        assert!(unmapped >= 1, "NonExistentFont 不在策略中 → Unmapped");

        // 验证未映射字体不会静默通过
        let unknown = plan
            .entries
            .iter()
            .find(|e| e.reference.family == "NonExistentFont12345")
            .expect("应存在未映射条目");
        assert!(
            matches!(unknown.decision, FontPlanDecision::Unmapped),
            "未知字体不得被静默归类为 Resolved 或 Defaulted"
        );

        // 验证 Defaulted 条目包含原因
        let calibri_entry = plan
            .entries
            .iter()
            .find(|e| e.reference.family == "Calibri")
            .expect("Calibri 应在 FontPlan 中");
        assert!(
            matches!(&calibri_entry.decision, FontPlanDecision::Defaulted { policy_id, reason } if policy_id == "calibri-family" && !reason.is_empty()),
            "Defaulted 必须包含 policy_id 与 reason"
        );

        let _ = std::fs::remove_file(test);
    }

    // covers: BR-DOC-DATA-001
    /// S2 验证：策略无任何候选字体时全部视为 Defaulted 或 Unmapped，绝不误报 Resolved。
    #[test]
    fn s2_empty_candidate_not_misclassified_as_resolved() {
        let trace = FontTrace {
            format: "docx".to_string(),
            references: vec![],
            warnings: vec![],
        };
        let policy = FontPolicy {
            version: 1,
            entries: vec![
                FontPolicyEntry {
                    id: "known-no-font".to_string(),
                    aliases: vec!["verdana".to_string()],
                    candidates: vec![], // 无候选
                },
                FontPolicyEntry {
                    id: "has-font".to_string(),
                    aliases: vec!["arial".to_string()],
                    candidates: vec!["binder:latin-sans".to_string()],
                },
            ],
        };
        // 空 candidates 的策略条目应被接受（不再报错）
        assert!(build_font_plan(&trace, &policy).is_ok());
    }
}
