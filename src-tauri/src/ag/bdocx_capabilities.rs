//! BDocxCapabilityManifest 运行时：版本化机器清单加载、提示词/工具 Schema 投影与 Diff 前校验。
//!
//! 语义 SSOT 见 A-SYS-I-P-04（BR-SYS-DATA-006）。本模块不新增编辑能力，也不取代
//! AgentEditableSnapshot / list_tables / list_images 的当前锚、范围、fingerprint 与动态 capabilities。

use crate::doc::style_subset::{classify_style, StyleTier};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

const RAW_MANIFEST: &str = include_str!("../../resources/bdocx-agent-capabilities.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentAvailability {
    Available,
    Dynamic,
    ManualOnly,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffFidelity {
    Block,
    Summary,
    Structured,
    None,
}

impl DiffFidelity {
    fn as_str(self) -> &'static str {
        match self {
            Self::Block => "block",
            Self::Summary => "summary",
            Self::Structured => "structured",
            Self::None => "none",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoundtripTier {
    Verified,
    RequiredPending,
    BestEffort,
    TemplateInherited,
    Unsupported,
}

impl RoundtripTier {
    fn as_str(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::RequiredPending => "required_pending",
            Self::BestEffort => "best_effort",
            Self::TemplateInherited => "template_inherited",
            Self::Unsupported => "unsupported",
        }
    }
}

impl From<StyleTier> for RoundtripTier {
    fn from(value: StyleTier) -> Self {
        match value {
            StyleTier::Verified => Self::Verified,
            StyleTier::RequiredPending => Self::RequiredPending,
            StyleTier::BestEffort => Self::BestEffort,
            StyleTier::TemplateInherited => Self::TemplateInherited,
            StyleTier::Unsupported => Self::Unsupported,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentOperationCapability {
    pub capability_id: String,
    pub operation: String,
    pub agent_availability: AgentAvailability,
    pub diff_fidelity: DiffFidelity,
    pub constraint: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleAttributeCapability {
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub required: bool,
    pub roundtrip_feature: Option<String>,
    pub roundtrip_tier: Option<RoundtripTier>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleMarkCapability {
    pub capability_id: String,
    pub mark_type: String,
    pub agent_availability: AgentAvailability,
    pub diff_fidelity: DiffFidelity,
    pub roundtrip_feature: Option<String>,
    pub roundtrip_tier: Option<RoundtripTier>,
    #[serde(default)]
    pub require_any_attribute: bool,
    #[serde(default)]
    pub attributes: Vec<StyleAttributeCapability>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockAttributeCapability {
    pub capability_id: String,
    pub name: String,
    pub kind: String,
    pub values: Vec<String>,
    pub supported_block_types: Vec<String>,
    pub agent_availability: AgentAvailability,
    pub diff_fidelity: DiffFidelity,
    pub roundtrip_feature: String,
    pub roundtrip_tier: RoundtripTier,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleOpsCapability {
    pub operation: String,
    pub max_items: usize,
    pub range_unit: String,
    pub marks: Vec<StyleMarkCapability>,
    pub block_attributes: Vec<BlockAttributeCapability>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableFieldCapability {
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableOperationCapability {
    pub capability_id: String,
    pub op: String,
    pub agent_availability: AgentAvailability,
    pub diff_fidelity: DiffFidelity,
    pub roundtrip_feature: String,
    pub roundtrip_tier: RoundtripTier,
    pub fields: Vec<TableFieldCapability>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableOpsCapability {
    pub operation: String,
    pub operations: Vec<TableOperationCapability>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageOperationCapability {
    pub capability_id: String,
    pub op: String,
    pub agent_availability: AgentAvailability,
    pub diff_fidelity: DiffFidelity,
    pub roundtrip_feature: String,
    pub roundtrip_tier: RoundtripTier,
    pub fields: Vec<TableFieldCapability>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageOpsCapability {
    pub operation: String,
    pub max_items: usize,
    pub batch_constraints: ImageBatchConstraints,
    pub operations: Vec<ImageOperationCapability>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageBatchConstraints {
    pub compatible_per_image: Vec<String>,
    pub exclusive: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualCapability {
    pub capability_id: String,
    pub label: String,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffFidelitySet {
    pub replace_block: DiffFidelity,
    pub block_ops: DiffFidelity,
    pub replace_document: DiffFidelity,
    pub style_ops: DiffFidelity,
    pub table_ops: DiffFidelity,
    pub image_ops: DiffFidelity,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoundtripFeature {
    pub feature: String,
    pub tier: RoundtripTier,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BDocxCapabilityManifest {
    pub manifest_id: String,
    pub version: String,
    pub editable_kind: String,
    pub content_format: String,
    pub document_operations: Vec<DocumentOperationCapability>,
    pub style_ops: StyleOpsCapability,
    pub table_ops: TableOpsCapability,
    pub image_ops: ImageOpsCapability,
    pub manual_only: Vec<ManualCapability>,
    pub diff_fidelity: DiffFidelitySet,
    pub roundtrip: Vec<RoundtripFeature>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityValidationError {
    pub code: &'static str,
    pub message: String,
    pub retriable: bool,
}

impl CapabilityValidationError {
    fn manifest(message: impl Into<String>) -> Self {
        Self {
            code: "AG_CAPABILITY_MANIFEST_INVALID",
            message: message.into(),
            retriable: false,
        }
    }

    fn style(message: impl Into<String>) -> Self {
        Self {
            code: "AG_STYLE_OPERATION_INVALID",
            message: message.into(),
            retriable: false,
        }
    }

    fn anchor(message: impl Into<String>) -> Self {
        Self {
            code: "AG_STYLE_ANCHOR_LOST",
            message: message.into(),
            retriable: true,
        }
    }

    fn table(message: impl Into<String>) -> Self {
        Self {
            code: "AG_TABLE_OPERATION_INVALID",
            message: message.into(),
            retriable: false,
        }
    }

    fn image(message: impl Into<String>) -> Self {
        Self {
            code: "AG_IMAGE_OPERATION_INVALID",
            message: message.into(),
            retriable: false,
        }
    }
}

static MANIFEST: OnceLock<Result<BDocxCapabilityManifest, String>> = OnceLock::new();
static MANIFEST_HASH: OnceLock<String> = OnceLock::new();

fn active_availability(value: AgentAvailability) -> bool {
    matches!(
        value,
        AgentAvailability::Available | AgentAvailability::Dynamic
    )
}

fn validate_tier_reference(
    catalog: &BTreeMap<&str, RoundtripTier>,
    feature: &str,
    tier: RoundtripTier,
) -> Result<(), String> {
    let Some(catalog_tier) = catalog.get(feature).copied() else {
        return Err(format!("roundtrip 缺少被能力引用的 feature={feature}"));
    };
    if catalog_tier != tier {
        return Err(format!(
            "feature={feature} 的能力 tier={} 与 roundtrip tier={} 不一致",
            tier.as_str(),
            catalog_tier.as_str()
        ));
    }
    let doc_tier = RoundtripTier::from(classify_style(feature));
    if doc_tier != tier {
        return Err(format!(
            "feature={feature} 的 manifest tier={} 与 DOC classify_style={} 不一致",
            tier.as_str(),
            doc_tier.as_str()
        ));
    }
    Ok(())
}

fn validate_manifest(value: &BDocxCapabilityManifest) -> Result<(), String> {
    if value.manifest_id != "binder.bdocx.agent-edit" {
        return Err("manifestId 必须为 binder.bdocx.agent-edit".to_string());
    }
    let version_parts = value.version.split('.').collect::<Vec<_>>();
    if version_parts.len() != 3
        || version_parts
            .iter()
            .any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return Err("version 必须是 x.y.z 形式的语义版本".to_string());
    }
    if value.editable_kind != "bdocx" || value.content_format != "html" {
        return Err("BDocx manifest 必须适用于 editableKind=bdocx/contentFormat=html".to_string());
    }
    if value.style_ops.operation != "style_ops"
        || value.table_ops.operation != "table_ops"
        || value.image_ops.operation != "image_ops"
    {
        return Err("StyleOp/TableOperation/ImageOperation 顶层 operation 名不正确".to_string());
    }
    if value.style_ops.max_items == 0 || value.style_ops.max_items > 256 {
        return Err("styleOps.maxItems 必须在 1..=256".to_string());
    }
    if value.style_ops.range_unit != "utf16_code_unit" {
        return Err("styleOps.rangeUnit 必须为 utf16_code_unit".to_string());
    }
    if value.image_ops.max_items == 0 || value.image_ops.max_items > 64 {
        return Err("imageOps.maxItems 必须在 1..=64".to_string());
    }
    if value.manual_only.is_empty() {
        return Err("manualOnly 不能为空，必须显式声明人工专属边界".to_string());
    }

    let mut capability_ids = BTreeSet::new();
    let mut insert_capability = |id: &str| -> Result<(), String> {
        if id.trim().is_empty() {
            return Err("capabilityId 不能为空".to_string());
        }
        if !capability_ids.insert(id.to_string()) {
            return Err(format!("capabilityId 重复: {id}"));
        }
        Ok(())
    };
    for capability in &value.document_operations {
        insert_capability(&capability.capability_id)?;
        if !active_availability(capability.agent_availability)
            || capability.diff_fidelity == DiffFidelity::None
            || capability.operation.trim().is_empty()
            || capability.constraint.trim().is_empty()
        {
            return Err(format!(
                "文档能力 {} 的 availability/diff/operation/constraint 非法",
                capability.capability_id
            ));
        }
        let declared = match capability.operation.as_str() {
            "replace_block" => value.diff_fidelity.replace_block,
            "block_ops" => value.diff_fidelity.block_ops,
            "replace_document" => value.diff_fidelity.replace_document,
            _ => {
                return Err(format!(
                    "文档能力 {} 的 operation 不支持",
                    capability.capability_id
                ))
            }
        };
        if declared != capability.diff_fidelity {
            return Err(format!(
                "文档能力 {} 的 Diff 等级与 diffFidelity 汇总不一致",
                capability.capability_id
            ));
        }
    }

    let mut roundtrip = BTreeMap::new();
    for feature in &value.roundtrip {
        if feature.feature.trim().is_empty() {
            return Err("roundtrip feature 不能为空".to_string());
        }
        if roundtrip
            .insert(feature.feature.as_str(), feature.tier)
            .is_some()
        {
            return Err(format!("roundtrip feature 重复: {}", feature.feature));
        }
        let doc_tier = RoundtripTier::from(classify_style(&feature.feature));
        if doc_tier != feature.tier {
            return Err(format!(
                "roundtrip feature={} 声明 {}，DOC classify_style 为 {}",
                feature.feature,
                feature.tier.as_str(),
                doc_tier.as_str()
            ));
        }
    }

    let mut mark_types = BTreeSet::new();
    for mark in &value.style_ops.marks {
        insert_capability(&mark.capability_id)?;
        if !mark_types.insert(mark.mark_type.as_str()) {
            return Err(format!("markType 重复: {}", mark.mark_type));
        }
        if !active_availability(mark.agent_availability) || mark.diff_fidelity == DiffFidelity::None
        {
            return Err(format!("mark {} 未声明为代理可用或无 Diff", mark.mark_type));
        }
        if mark.diff_fidelity != value.diff_fidelity.style_ops {
            return Err(format!(
                "mark {} 的 Diff 等级与 styleOps 汇总不一致",
                mark.mark_type
            ));
        }
        if mark.require_any_attribute && mark.attributes.is_empty() {
            return Err(format!(
                "mark {} 要求 attrs 但 attributes 为空",
                mark.mark_type
            ));
        }
        match (&mark.roundtrip_feature, mark.roundtrip_tier) {
            (Some(feature), Some(tier)) => validate_tier_reference(&roundtrip, feature, tier)?,
            (None, None) => {}
            _ => {
                return Err(format!(
                    "mark {} 的 roundtripFeature/roundtripTier 必须同时存在或同时为空",
                    mark.mark_type
                ))
            }
        }
        let mut attr_names = BTreeSet::new();
        for attr in &mark.attributes {
            if !attr_names.insert(attr.name.as_str()) {
                return Err(format!("mark {} attrs 重复: {}", mark.mark_type, attr.name));
            }
            if !matches!(
                attr.kind.as_str(),
                "hex_color" | "font_size" | "font_family" | "safe_href"
            ) {
                return Err(format!(
                    "mark {} attr kind 不支持: {}",
                    mark.mark_type, attr.kind
                ));
            }
            match (&attr.roundtrip_feature, attr.roundtrip_tier) {
                (Some(feature), Some(tier)) => validate_tier_reference(&roundtrip, feature, tier)?,
                (None, None) => {}
                _ => {
                    return Err(format!(
                        "mark {} attr {} 的 roundtrip 引用不完整",
                        mark.mark_type, attr.name
                    ))
                }
            }
        }
    }
    if mark_types.contains("sub") || mark_types.contains("sup") {
        return Err("markType 必须使用 subscript/superscript，不能使用 sub/sup".to_string());
    }

    let mut block_attrs = BTreeSet::new();
    for attr in &value.style_ops.block_attributes {
        insert_capability(&attr.capability_id)?;
        if !block_attrs.insert(attr.name.as_str())
            || attr.kind != "enum"
            || attr.values.is_empty()
            || attr.supported_block_types.is_empty()
            || !active_availability(attr.agent_availability)
            || attr.diff_fidelity == DiffFidelity::None
        {
            return Err(format!("块属性 {} 声明非法", attr.name));
        }
        if attr.diff_fidelity != value.diff_fidelity.style_ops {
            return Err(format!(
                "块属性 {} 的 Diff 等级与 styleOps 汇总不一致",
                attr.name
            ));
        }
        validate_tier_reference(&roundtrip, &attr.roundtrip_feature, attr.roundtrip_tier)?;
    }

    let mut table_ops = BTreeSet::new();
    for operation in &value.table_ops.operations {
        insert_capability(&operation.capability_id)?;
        if !table_ops.insert(operation.op.as_str())
            || !active_availability(operation.agent_availability)
            || operation.diff_fidelity == DiffFidelity::None
        {
            return Err(format!("TableOperation {} 声明非法或重复", operation.op));
        }
        if operation.diff_fidelity != value.diff_fidelity.table_ops {
            return Err(format!(
                "TableOperation {} 的 Diff 等级与 tableOps 汇总不一致",
                operation.op
            ));
        }
        validate_tier_reference(
            &roundtrip,
            &operation.roundtrip_feature,
            operation.roundtrip_tier,
        )?;
        let mut fields = BTreeSet::new();
        for field in &operation.fields {
            if !fields.insert(field.name.as_str())
                || !matches!(
                    field.kind.as_str(),
                    "non_negative_integer"
                        | "boolean"
                        | "coordinate"
                        | "string"
                        | "hex_color_or_null"
                        | "enum"
                )
            {
                return Err(format!(
                    "TableOperation {} 字段 {} 声明非法或重复",
                    operation.op, field.name
                ));
            }
            if field.kind == "enum" && field.values.is_empty() {
                return Err(format!(
                    "TableOperation {} 字段 {} 的 enum 为空",
                    operation.op, field.name
                ));
            }
        }
    }
    let mut image_ops = BTreeSet::new();
    for operation in &value.image_ops.operations {
        insert_capability(&operation.capability_id)?;
        if !image_ops.insert(operation.op.as_str())
            || !active_availability(operation.agent_availability)
            || operation.diff_fidelity == DiffFidelity::None
        {
            return Err(format!("ImageOperation {} 声明非法或重复", operation.op));
        }
        if operation.diff_fidelity != value.diff_fidelity.image_ops {
            return Err(format!(
                "ImageOperation {} 的 Diff 等级与 imageOps 汇总不一致",
                operation.op
            ));
        }
        validate_tier_reference(
            &roundtrip,
            &operation.roundtrip_feature,
            operation.roundtrip_tier,
        )?;
        let mut fields = BTreeSet::new();
        for field in &operation.fields {
            if !fields.insert(field.name.as_str())
                || !matches!(
                    field.kind.as_str(),
                    "string" | "positive_integer" | "relative_path" | "sha256" | "enum"
                )
            {
                return Err(format!(
                    "ImageOperation {} 字段 {} 声明非法或重复",
                    operation.op, field.name
                ));
            }
            if field.kind == "enum" && field.values.is_empty() {
                return Err(format!(
                    "ImageOperation {} 字段 {} 的 enum 为空",
                    operation.op, field.name
                ));
            }
        }
    }
    if value.image_ops.batch_constraints.compatible_per_image
        != ["resize_image", "set_image_layout", "move_image"]
        || value.image_ops.batch_constraints.exclusive != ["delete_image"]
    {
        return Err(
            "imageOps.batchConstraints 必须声明 resize/layout/move 可组合且 delete 独占"
                .to_string(),
        );
    }
    for capability in &value.manual_only {
        insert_capability(&capability.capability_id)?;
        if capability.label.trim().is_empty() || capability.reason.trim().is_empty() {
            return Err(format!(
                "manualOnly {} 缺 label/reason",
                capability.capability_id
            ));
        }
    }
    Ok(())
}

/**
 * @GOV
 * codes:    BR-SYS-DATA-006-DATA-AG-AG-TOOL-001
 * type:     DATA
 * chain:    AG-TOOL
 * rules:    BR-SYS-DATA-006
 * term_ref: TERM-SYS-002
 * boundary: in=bundledBDocxCapabilityManifestJson | out=BDocxCapabilityManifest(validatedVersionedSingleRuntimeSource)
 */
pub fn manifest() -> Result<&'static BDocxCapabilityManifest, CapabilityValidationError> {
    let result = MANIFEST.get_or_init(|| {
        let parsed = serde_json::from_str::<BDocxCapabilityManifest>(RAW_MANIFEST)
            .map_err(|error| format!("解析 BDocxCapabilityManifest 失败: {error}"))?;
        validate_manifest(&parsed)?;
        Ok(parsed)
    });
    result
        .as_ref()
        .map_err(|message| CapabilityValidationError::manifest(message.clone()))
}

pub fn manifest_sha256() -> &'static str {
    MANIFEST_HASH
        .get_or_init(|| format!("{:x}", Sha256::digest(RAW_MANIFEST.as_bytes())))
        .as_str()
}

fn string_schema(kind: &str, values: &[String]) -> Value {
    match kind {
        "hex_color" => json!({ "type": "string", "pattern": "^#[0-9A-Fa-f]{6}$" }),
        "font_size" => json!({
            "type": "string",
            "pattern": "^[1-9][0-9]{0,2}(?:\\.[0-9]+)?(?:pt|px)$"
        }),
        "font_family" => json!({
            "type": "string",
            "minLength": 1,
            "maxLength": 128,
            "pattern": "^[^;\\u0000-\\u001F\\u007F]+$"
        }),
        "safe_href" => json!({
            "type": "string",
            "minLength": 1,
            "maxLength": 2048,
            "pattern": "^[^\\u0000-\\u001F\\u007F]+$",
            "allOf": [
                { "not": { "pattern": "^[Jj][Aa][Vv][Aa][Ss][Cc][Rr][Ii][Pp][Tt]:" } },
                { "not": { "pattern": "^[Dd][Aa][Tt][Aa]:" } }
            ]
        }),
        "enum" => json!({ "type": "string", "enum": values }),
        _ => json!({ "type": "string" }),
    }
}

fn style_mark_schema(mark: &StyleMarkCapability) -> Value {
    let mut mark_properties = Map::new();
    mark_properties.insert("type".to_string(), json!({ "const": mark.mark_type }));
    let mut required = vec![json!("type")];
    if !mark.attributes.is_empty() {
        let mut attrs_properties = Map::new();
        let mut attrs_required = Vec::new();
        for attr in &mark.attributes {
            attrs_properties.insert(attr.name.clone(), string_schema(&attr.kind, &[]));
            if attr.required {
                attrs_required.push(json!(attr.name));
            }
        }
        let mut attrs_schema = json!({
            "type": "object",
            "properties": attrs_properties,
            "additionalProperties": false
        });
        if mark.require_any_attribute {
            attrs_schema["minProperties"] = json!(1);
            required.push(json!("attrs"));
        }
        if !attrs_required.is_empty() {
            attrs_schema["required"] = Value::Array(attrs_required);
            if !required.contains(&json!("attrs")) {
                required.push(json!("attrs"));
            }
        }
        mark_properties.insert("attrs".to_string(), attrs_schema);
    }
    json!({
        "type": "object",
        "properties": mark_properties,
        "required": required,
        "additionalProperties": false
    })
}

fn style_operation_schema(value: &BDocxCapabilityManifest) -> Value {
    let set_mark = value
        .style_ops
        .marks
        .iter()
        .map(|mark| {
            json!({
                "type": "object",
                "properties": {
                    "op": { "const": "set_mark" },
                    "blockId": { "type": "string", "minLength": 1 },
                    "start": {
                        "type": "integer",
                        "minimum": 0,
                        "description": format!("块内 {} 半开区间起点", value.style_ops.range_unit)
                    },
                    "end": {
                        "type": "integer",
                        "minimum": 1,
                        "description": format!("块内 {} 半开区间终点", value.style_ops.range_unit)
                    },
                    "mark": style_mark_schema(mark)
                },
                "required": ["op", "blockId", "start", "end", "mark"],
                "additionalProperties": false
            })
        })
        .collect::<Vec<_>>();
    let mark_types = value
        .style_ops
        .marks
        .iter()
        .map(|mark| Value::String(mark.mark_type.clone()))
        .collect::<Vec<_>>();
    let unset_mark = json!({
        "type": "object",
        "properties": {
            "op": { "const": "unset_mark" },
            "blockId": { "type": "string", "minLength": 1 },
            "start": {
                "type": "integer",
                "minimum": 0,
                "description": format!("块内 {} 半开区间起点", value.style_ops.range_unit)
            },
            "end": {
                "type": "integer",
                "minimum": 1,
                "description": format!("块内 {} 半开区间终点", value.style_ops.range_unit)
            },
            "markType": { "type": "string", "enum": mark_types }
        },
        "required": ["op", "blockId", "start", "end", "markType"],
        "additionalProperties": false
    });
    let block_attrs = value
        .style_ops
        .block_attributes
        .iter()
        .map(|attr| {
            let mut attrs_properties = Map::new();
            attrs_properties.insert(attr.name.clone(), string_schema(&attr.kind, &attr.values));
            json!({
                "type": "object",
                "properties": {
                    "op": { "const": "set_block_attr" },
                    "blockId": { "type": "string", "minLength": 1 },
                    "attrs": {
                        "type": "object",
                        "properties": attrs_properties,
                        "required": [attr.name.clone()],
                        "additionalProperties": false
                    }
                },
                "required": ["op", "blockId", "attrs"],
                "additionalProperties": false
            })
        })
        .collect::<Vec<_>>();
    let mut variants = set_mark;
    variants.push(unset_mark);
    variants.extend(block_attrs);
    json!({
        "type": "array",
        "minItems": 1,
        "maxItems": value.style_ops.max_items,
        "items": { "oneOf": variants }
    })
}

fn table_field_schema(field: &TableFieldCapability) -> Value {
    match field.kind.as_str() {
        "non_negative_integer" => json!({ "type": "integer", "minimum": 0 }),
        "boolean" => json!({ "type": "boolean" }),
        "coordinate" => json!({
            "type": "object",
            "properties": {
                "row": { "type": "integer", "minimum": 0 },
                "column": { "type": "integer", "minimum": 0 }
            },
            "required": ["row", "column"],
            "additionalProperties": false
        }),
        "hex_color_or_null" => json!({
            "anyOf": [
                { "type": "string", "pattern": "^#[0-9A-Fa-f]{6}$" },
                { "type": "null" }
            ]
        }),
        "enum" => json!({ "type": "string", "enum": field.values }),
        _ => json!({ "type": "string" }),
    }
}

fn table_operations_schema(value: &BDocxCapabilityManifest) -> Value {
    let variants = value
        .table_ops
        .operations
        .iter()
        .map(|operation| {
            let mut properties = Map::new();
            properties.insert("op".to_string(), json!({ "const": operation.op }));
            let mut required = vec![json!("op")];
            for field in &operation.fields {
                properties.insert(field.name.clone(), table_field_schema(field));
                if field.required {
                    required.push(json!(field.name));
                }
            }
            json!({
                "type": "object",
                "properties": properties,
                "required": required,
                "additionalProperties": false
            })
        })
        .collect::<Vec<_>>();
    json!({
        "type": "array",
        "minItems": 1,
        "items": { "oneOf": variants }
    })
}

fn image_field_schema(field: &TableFieldCapability) -> Value {
    match field.kind.as_str() {
        "positive_integer" => json!({ "type": "integer", "minimum": 1, "maximum": 4096 }),
        "relative_path" => json!({
            "type": "string",
            "minLength": 1,
            "maxLength": 1024,
            "pattern": "^[^/\\\\\\u0000][^\\u0000]*$",
            "description": "list_images 返回的 workspace 相对图片路径；动态校验拒绝绝对、越界和符号链接"
        }),
        "sha256" => json!({ "type": "string", "pattern": "^[0-9a-f]{64}$" }),
        "enum" => json!({ "type": "string", "enum": field.values }),
        _ if field.name == "alt" => {
            json!({ "type": "string", "maxLength": 512, "pattern": "^[^\\u0000]*$" })
        }
        _ => json!({ "type": "string", "minLength": 1, "maxLength": 1024 }),
    }
}

fn image_operations_schema(value: &BDocxCapabilityManifest) -> Value {
    let variants = value
        .image_ops
        .operations
        .iter()
        .map(|operation| {
            let mut properties = Map::new();
            properties.insert("op".to_string(), json!({ "const": operation.op }));
            let mut required = vec![json!("op")];
            for field in &operation.fields {
                properties.insert(field.name.clone(), image_field_schema(field));
                if field.required {
                    required.push(json!(field.name));
                }
            }
            json!({
                "type": "object",
                "properties": properties,
                "required": required,
                "additionalProperties": false
            })
        })
        .collect::<Vec<_>>();
    json!({
        "type": "array",
        "minItems": 1,
        "maxItems": value.image_ops.max_items,
        "items": { "oneOf": variants }
    })
}

/**
 * @GOV
 * codes:    BR-SYS-DATA-006-DATA-AG-AG-TOOL-002
 * type:     DATA
 * chain:    AG-TOOL
 * rules:    BR-SYS-DATA-006
 * term_ref: TERM-SYS-002, TERM-ED-008
 * boundary: in=BDocxCapabilityManifest | out=ProposeEditJsonSchema(conditionallyRequiredBlockStyleTableImageOperations)
 */
pub fn propose_edit_parameters_schema() -> Result<Value, CapabilityValidationError> {
    let value = manifest()?;
    // propose_edit 是跨格式工具。replace_document 在 BDocx 中也可由 manifest 动态开放，
    // 但其是否可用由当前快照的表格/图片保护状态和后端守卫决定；enum 只声明工具形状，
    // 不替代运行时路由校验。
    let mut operations = vec![
        Value::String("replace_block".to_string()),
        Value::String("replace_document".to_string()),
    ];
    operations.extend(
        value
            .document_operations
            .iter()
            .filter(|capability| {
                active_availability(capability.agent_availability)
                    && capability.operation != "replace_document"
            })
            .map(|capability| Value::String(capability.operation.clone())),
    );
    operations.push(Value::String(value.style_ops.operation.clone()));
    operations.push(Value::String(value.table_ops.operation.clone()));
    operations.push(Value::String(value.image_ops.operation.clone()));
    let table_operations = table_operations_schema(value);
    let image_operations = image_operations_schema(value);
    let block_operations = json!({
        "type": "array",
        "minItems": 1,
        // Projected from the runtime guard so the schema can never declare a batch legal that
        // the executor then rejects. Do not hardcode a number here.
        "maxItems": crate::ag::edit_support::MAX_BLOCK_OPS_PER_CALL,
        "description": format!(
            "单次最多 {} 个操作；同一章连续追加用相同 adjudicationGroup 合成一张卡，写完一章直接提交，不需要自己分批",
            crate::ag::edit_support::MAX_BLOCK_OPS_PER_CALL
        ),
        "items": {
            "oneOf": [
                {
                    "type": "object",
                    "properties": {
                        "op": { "const": "replace_node" },
                        "adjudicationGroup": { "type": "string", "minLength": 1, "description": "仅当相邻块必须共同接受或拒绝时使用相同值；省略则每块独立成卡" },
                        "blockId": { "type": "string", "minLength": 1 },
                        "fingerprint": { "type": "string", "minLength": 1 },
                        "expectedNodeType": {
                            "type": "string",
                            "enum": ["paragraph", "heading", "codeBlock"]
                        },
                        "afterNode": {
                            "type": "object",
                            "properties": {
                                "type": {
                                    "type": "string",
                                    "enum": ["paragraph", "heading", "codeBlock"]
                                },
                                "attrs": {
                                    "type": "object",
                                    "description": "必须从目标 nodeJson 原样复制，保留 blockId"
                                },
                                "content": {
                                    "type": "array",
                                    "items": {
                                        "oneOf": [
                                            {
                                                "type": "object",
                                                "properties": {
                                                    "type": { "const": "text" },
                                                    "text": { "type": "string", "minLength": 1 },
                                                    "marks": {
                                                        "type": "array",
                                                        "items": {
                                                            "type": "object",
                                                            "properties": {
                                                                "type": {
                                                                    "type": "string",
                                                                    "enum": ["bold", "italic", "underline", "strike", "subscript", "superscript", "code", "link", "highlight", "textStyle"]
                                                                },
                                                                "attrs": { "type": "object" }
                                                            },
                                                            "required": ["type"],
                                                            "additionalProperties": false
                                                        }
                                                    }
                                                },
                                                "required": ["type", "text"],
                                                "additionalProperties": false
                                            },
                                            {
                                                "type": "object",
                                                "properties": { "type": { "const": "hardBreak" } },
                                                "required": ["type"],
                                                "additionalProperties": false
                                            }
                                        ]
                                    }
                                }
                            },
                            "required": ["type", "attrs"],
                            "additionalProperties": false
                        }
                    },
                    "required": ["op", "blockId", "fingerprint", "expectedNodeType", "afterNode"],
                    "additionalProperties": false
                },
                // AG-TASK-01 F2：delete_node——删除 content 原子块（afterNode 为 null 删除标记）。
                {
                    "type": "object",
                    "properties": {
                        "op": { "const": "delete_node" },
                        "adjudicationGroup": { "type": "string", "minLength": 1, "description": "仅当相邻块必须共同接受或拒绝时使用相同值" },
                        "blockId": { "type": "string", "minLength": 1, "description": "要删除的块ID（role=content 且 capabilities 含 delete_node 的可删；容器/原子块不可删）" },
                        "expectedNodeType": {
                            "type": "string",
                            "enum": ["paragraph", "heading", "codeBlock"]
                        }
                    },
                    "required": ["op", "blockId", "expectedNodeType"],
                    "additionalProperties": false
                },
                {
                    "type": "object",
                    "properties": {
                        "op": { "const": "insert_before" },
                        "adjudicationGroup": { "type": "string", "minLength": 1, "description": "仅当相邻块必须共同接受或拒绝时使用相同值" },
                        "blockId": { "type": "string", "minLength": 1, "description": "锚点块ID，新块将插入到此块之前" },
                        "afterNode": {
                            "type": "object",
                            "properties": {
                                "type": {
                                    "type": "string",
                                    "enum": ["paragraph", "heading", "codeBlock"]
                                },
                                // AG-STRATEGY-04：新块 attrs（含可选 blockId）——缺省由后端兜底生成唯一 id；
                                // 结构对齐 replace_node 的 afterNode.attrs，使前端 nodeFromJSON 生成带 id 的新块。
                                "attrs": {
                                    "type": "object",
                                    "description": "新块属性（可选 blockId；缺省由后端生成 new_block_{uuid}）"
                                },
                                "content": {
                                    "type": "array",
                                    "items": {
                                        "oneOf": [
                                            {
                                                "type": "object",
                                                "properties": {
                                                    "type": { "const": "text" },
                                                    "text": { "type": "string", "minLength": 1 }
                                                },
                                                "required": ["type", "text"],
                                                "additionalProperties": false
                                            },
                                            {
                                                "type": "object",
                                                "properties": { "type": { "const": "hardBreak" } },
                                                "required": ["type"],
                                                "additionalProperties": false
                                            }
                                        ]
                                    }
                                }
                            },
                            "required": ["type", "content"],
                            "additionalProperties": false
                        }
                    },
                    "required": ["op", "blockId", "afterNode"],
                    "additionalProperties": false
                },
                {
                    "type": "object",
                    "properties": {
                        "op": { "const": "insert_after" },
                        "adjudicationGroup": { "type": "string", "minLength": 1, "description": "仅当相邻块必须共同接受或拒绝时使用相同值" },
                        "blockId": { "type": "string", "minLength": 1, "description": "锚点块ID，新块将插入到此块之后" },
                        "afterNode": {
                            "type": "object",
                            "properties": {
                                "type": {
                                    "type": "string",
                                    "enum": ["paragraph", "heading", "codeBlock"]
                                },
                                "attrs": {
                                    "type": "object",
                                    "description": "新块属性（可选 blockId；缺省由后端生成 new_block_{uuid}）"
                                },
                                "content": {
                                    "type": "array",
                                    "items": {
                                        "oneOf": [
                                            {
                                                "type": "object",
                                                "properties": {
                                                    "type": { "const": "text" },
                                                    "text": { "type": "string", "minLength": 1 }
                                                },
                                                "required": ["type", "text"],
                                                "additionalProperties": false
                                            },
                                            {
                                                "type": "object",
                                                "properties": { "type": { "const": "hardBreak" } },
                                                "required": ["type"],
                                                "additionalProperties": false
                                            }
                                        ]
                                    }
                                }
                            },
                            "required": ["type", "content"],
                            "additionalProperties": false
                        }
                    },
                    "required": ["op", "blockId", "afterNode"],
                    "additionalProperties": false
                },
                {
                    "type": "object",
                    "properties": {
                        "op": { "const": "append" },
                        "adjudicationGroup": { "type": "string", "minLength": 1, "description": "仅当相邻块必须共同接受或拒绝时使用相同值" },
                        "afterNode": {
                            "type": "object",
                            "properties": {
                                "type": {
                                    "type": "string",
                                    "enum": ["paragraph", "heading", "codeBlock"]
                                },
                                "attrs": {
                                    "type": "object",
                                    "description": "新块属性（可选 blockId；缺省由后端生成 new_block_{uuid}）"
                                },
                                "content": {
                                    "type": "array",
                                    "items": {
                                        "oneOf": [
                                            {
                                                "type": "object",
                                                "properties": {
                                                    "type": { "const": "text" },
                                                    "text": { "type": "string", "minLength": 1 }
                                                },
                                                "required": ["type", "text"],
                                                "additionalProperties": false
                                            },
                                            {
                                                "type": "object",
                                                "properties": { "type": { "const": "hardBreak" } },
                                                "required": ["type"],
                                                "additionalProperties": false
                                            }
                                        ]
                                    }
                                }
                            },
                            "required": ["type", "content"],
                            "additionalProperties": false
                        }
                    },
                    "required": ["op", "afterNode"],
                    "additionalProperties": false
                }
            ]
        }
    });
    Ok(json!({
        "type": "object",
        "properties": {
            "path": { "type": "string", "minLength": 1, "description": "目标文档相对路径。对已存在文档生成修改建议；填写不存在的路径则视为新建文档（经 DiffCard 审阅后创建）。" },
            "operation": {
                "type": "string",
                "enum": operations,
                "description": "replace_block=纯文本块替换；block_ops=富文档受控内容节点替换；replace_document=完整文档重写（BDocx 仅当前快照无表格且无图片时可用，后端强制校验）；style_ops=文本/块样式；table_ops=受限表格结构与单元格属性；image_ops=受限图片操作"
            },
            "block_id": { "type": "string", "minLength": 1, "description": "块列表 [id=…] 的完整稳定锚" },
            "block_index": { "type": "integer", "minimum": 0, "description": "没有 block_id 时使用的 0 起块号" },
            "new_text": { "type": "string", "description": "replace_block 的块内新内容" },
            "content": { "type": "string", "description": "replace_document 的完整文档内容；获准的 BDocx 整篇重写必须提供完整 HTML，且不得引入表格或图片" },
            "style_ops": style_operation_schema(value),
            "base": {
                "type": "object",
                "properties": {
                    "sessionVersion": { "type": "integer", "minimum": 0 },
                    "contentHash": { "type": "string", "minLength": 1 }
                },
                "required": ["sessionVersion", "contentHash"],
                "additionalProperties": false
            },
            "target": {
                "type": "object",
                "properties": {
                    "tableId": { "type": "string", "minLength": 1 },
                    "fingerprint": { "type": "string", "minLength": 1 }
                },
                "required": ["tableId", "fingerprint"],
                "additionalProperties": false
            },
            "operations": {
                "oneOf": [block_operations.clone(), table_operations.clone(), image_operations.clone()]
            },
        },
        "required": ["path", "operation"],
        "allOf": [
            {
                "if": { "properties": { "operation": { "const": "replace_block" } } },
                "then": {
                    "required": ["new_text"],
                    "anyOf": [
                        { "required": ["block_id"] },
                        { "required": ["block_index"] }
                    ]
                }
            },
            {
                "if": { "properties": { "operation": { "const": "block_ops" } } },
                "then": {
                    "required": ["base", "operations"],
                    "properties": { "operations": block_operations }
                }
            },
            {
                "if": { "properties": { "operation": { "const": "replace_document" } } },
                "then": { "required": ["content"] }
            },
            {
                "if": { "properties": { "operation": { "const": "style_ops" } } },
                "then": { "required": ["style_ops"] }
            },
            {
                "if": { "properties": { "operation": { "const": "table_ops" } } },
                "then": {
                    "required": ["base", "target", "operations"],
                    "properties": { "operations": table_operations }
                }
            },
            {
                "if": { "properties": { "operation": { "const": "image_ops" } } },
                "then": {
                    "required": ["base", "operations"],
                    "properties": { "operations": image_operations }
                }
            }
        ],
        "additionalProperties": false
    }))
}

fn roundtrip_summary(value: &BDocxCapabilityManifest) -> String {
    let mut groups: BTreeMap<RoundtripTier, Vec<&str>> = BTreeMap::new();
    for feature in &value.roundtrip {
        groups
            .entry(feature.tier)
            .or_default()
            .push(feature.feature.as_str());
    }
    [
        RoundtripTier::Verified,
        RoundtripTier::RequiredPending,
        RoundtripTier::BestEffort,
        RoundtripTier::TemplateInherited,
        RoundtripTier::Unsupported,
    ]
    .iter()
    .map(|tier| {
        format!(
            "{}={}",
            tier.as_str(),
            groups.get(tier).cloned().unwrap_or_default().join(",")
        )
    })
    .collect::<Vec<_>>()
    .join("；")
}

/// 从当前 BDocx 快照的结构保护状态投影整篇改写路由。
///
/// 清单说明可用操作的静态形状；表格和图片的存在性属于同轮 LogicalState 的动态事实，
/// 因此必须由调用方传入，且后端会用同一条件再次拒绝不安全的 `replace_document`。
pub fn bdocx_rewrite_route_instruction(has_tables: bool, has_images: bool) -> String {
    let protected = match (has_tables, has_images) {
        (true, true) => "表格和图片",
        (true, false) => "表格",
        (false, true) => "图片",
        (false, false) => "",
    };
    if protected.is_empty() {
        "[当前 BDocx 重写路由] 当前快照未检测到表格或图片。用户要求整篇重写或大幅重排时，可用 propose_edit operation=replace_document 并提交完整 HTML；不得提交片段，也不得在新内容中引入 table 或 img。局部文字修改仍使用 block_ops。新增小标题可用 block_ops 的 insert_before/insert_after/append 插入 afterNode.type=heading；正文末尾追加多段必须使用多个 append（不需要锚点），禁止对同一末块重复 insert_after；replace_node 必须保留既有块的 node type，不能把既有段落转换成标题，既有块的类型转换为人工专属。".to_string()
    } else {
        format!(
            "[当前 BDocx 重写路由] 当前快照检测到{protected}。replace_document 已被禁止，后端会拒绝该操作；包括大幅改写在内的既有文字内容只能用 block_ops，表格只能用 table_ops，图片只能用 image_ops。新增小标题仍可用 block_ops 的 insert_before/insert_after/append 插入 afterNode.type=heading；replace_node 必须保留既有块的 node type，不能把既有段落转换成标题，既有块的类型转换为人工专属。"
        )
    }
}

/**
 * @GOV
 * codes:    BR-SYS-DATA-006-DATA-AG-AG-TOOL-032
 * type:     DATA
 * chain:    AG-TURN
 * rules:    BR-SYS-DATA-006
 * term_ref: TERM-SYS-002
 * boundary: in=BDocxCapabilityManifest | out=AgentPromptCapabilityProjection(versionHashAgentManualDiffRoundtrip)
 */
pub fn prompt_projection() -> Result<String, CapabilityValidationError> {
    let value = manifest()?;
    let document_ops = value
        .document_operations
        .iter()
        .map(|capability| {
            format!(
                "{}({},约束={})",
                capability.operation,
                capability.diff_fidelity.as_str(),
                capability.constraint
            )
        })
        .collect::<Vec<_>>()
        .join("；");
    let style_marks = value
        .style_ops
        .marks
        .iter()
        .map(|mark| {
            if mark.attributes.is_empty() {
                format!(
                    "{}[{}]",
                    mark.mark_type,
                    mark.roundtrip_tier
                        .map(RoundtripTier::as_str)
                        .unwrap_or("unsupported")
                )
            } else {
                let attrs = mark
                    .attributes
                    .iter()
                    .map(|attr| {
                        format!(
                            "{}[{}]",
                            attr.name,
                            attr.roundtrip_tier
                                .map(RoundtripTier::as_str)
                                .or_else(|| mark.roundtrip_tier.map(RoundtripTier::as_str))
                                .unwrap_or("unsupported")
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                format!("{}{{{attrs}}}", mark.mark_type)
            }
        })
        .collect::<Vec<_>>()
        .join("；");
    let block_attrs = value
        .style_ops
        .block_attributes
        .iter()
        .map(|attr| {
            format!(
                "{}={} [{}]",
                attr.name,
                attr.values.join("|"),
                attr.roundtrip_tier.as_str()
            )
        })
        .collect::<Vec<_>>()
        .join("；");
    let table_ops = value
        .table_ops
        .operations
        .iter()
        .map(|operation| format!("{}[{}]", operation.op, operation.roundtrip_tier.as_str()))
        .collect::<Vec<_>>()
        .join("；");
    let image_ops = value
        .image_ops
        .operations
        .iter()
        .map(|operation| format!("{}[{}]", operation.op, operation.roundtrip_tier.as_str()))
        .collect::<Vec<_>>()
        .join("；");
    let compatible_image_ops = value
        .image_ops
        .batch_constraints
        .compatible_per_image
        .join("、");
    let exclusive_image_ops = value.image_ops.batch_constraints.exclusive.join("、");
    let manual_only = value
        .manual_only
        .iter()
        .map(|capability| capability.label.as_str())
        .collect::<Vec<_>>()
        .join("；");
    Ok(format!(
        "[当前 BDocx 编辑能力声明]\nmanifestId={} version={} sha256={}\n格式：editable_kind={} content_format={}。\n文档操作：{}。整篇改写路由以同一条当前文档运行时声明中的表格/图片状态为准：两者都不存在时，才可用 replace_document 提交完整 HTML；任一存在时，replace_document 被禁止，既有文字内容使用 block_ops，表格/图片分别使用 table_ops/image_ops。block_ops 必须基于活动 ProseMirror LogicalState，只命中 capabilities=replace_node 的内容块，原样带回 sessionVersion/contentHash、blockId、fingerprint、expectedNodeType 与 node attrs；容器/原子块只读，多块同批原子提交。新增 heading 块可用 insert_before/insert_after/append；正文末尾追加多段必须使用多个 append（append 不需要锚点），禁止对同一末块重复 insert_after，否则整批按锚点冲突拒绝；同一章连续追加使用相同 adjudicationGroup，使整章合成一张卡裁决，不同章使用不同组名；replace_node 必须保留既有块类型，既有块的类型转换是人工专属。不得使用 replace_block 或原始 HTML 绕过受保护结构。\nStyleOp（range 为块内纯文本半开区间，单位={}，批内原子）：marks={}；blockAttrs={}；最多 {} 项。\nTableOperation（必须先 list_tables，并原样带回 sessionVersion/contentHash/tableId/fingerprint；还须服从当前 capabilities 与 moveDirections）：{}。\nImageOperation（必须先 list_images，并原样带回 sessionVersion/contentHash、稳定 imageId/当前 fingerprint、动态 capabilities/resizeBounds/moveDirections；插入还须带回 assetId/sourcePath/sourceHash；允许插入、删除、按宽度缩放、独立图片段落上下移动与 left/center/right 对齐；同一图片单批可各含至多一个 {}，{} 必须独占；禁止远程/绝对/越界来源且不得删除项目源文件）：{}。\n图片理解边界：list_images 只提供图片元数据与相邻文本语境，不提供图片像素、OCR 或视觉识别；不得把语境推断表述为看懂图片画面。workspace 内非受控图片先调用 import_image_asset 复制准备，不得移动或删除源引用；确实无法安全提案时调用 edit_blocked 说明原因。\n人工专属、代理不可调用：{}。\nDiff 等级：replace_block={}；block_ops={}；replace_document={}；style_ops={}；table_ops={}；image_ops={}。\nDOCX 往返等级：{}。非 verified 项保存重开后可能变化；不得向用户声称完全保真。",
        value.manifest_id,
        value.version,
        manifest_sha256(),
        value.editable_kind,
        value.content_format,
        document_ops,
        value.style_ops.range_unit,
        style_marks,
        block_attrs,
        value.style_ops.max_items,
        table_ops,
        compatible_image_ops,
        exclusive_image_ops,
        image_ops,
        manual_only,
        value.diff_fidelity.replace_block.as_str(),
        value.diff_fidelity.block_ops.as_str(),
        value.diff_fidelity.replace_document.as_str(),
        value.diff_fidelity.style_ops.as_str(),
        value.diff_fidelity.table_ops.as_str(),
        value.diff_fidelity.image_ops.as_str(),
        roundtrip_summary(value)
    ))
}

fn object<'a>(
    value: &'a Value,
    label: &str,
) -> Result<&'a Map<String, Value>, CapabilityValidationError> {
    value
        .as_object()
        .ok_or_else(|| CapabilityValidationError::style(format!("{label} 必须是 object")))
}

fn exact_keys(
    object: &Map<String, Value>,
    allowed: &[&str],
    label: &str,
    error: fn(String) -> CapabilityValidationError,
) -> Result<(), CapabilityValidationError> {
    if let Some(unknown) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(error(format!("{label} 包含未知字段 {unknown}")));
    }
    Ok(())
}

pub(crate) fn valid_hex_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(crate) fn valid_font_size(value: &str) -> bool {
    let Some(number) = value
        .strip_suffix("pt")
        .or_else(|| value.strip_suffix("px"))
    else {
        return false;
    };
    let integer = number.split('.').next().unwrap_or("");
    if integer.is_empty()
        || !integer.bytes().all(|byte| byte.is_ascii_digit())
        || (integer.len() > 1 && integer.starts_with('0'))
    {
        return false;
    }
    if let Some(fraction) = number.split_once('.').map(|(_, fraction)| fraction) {
        if fraction.is_empty() || !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
            return false;
        }
    }
    number
        .parse::<f64>()
        .map(|number| number.is_finite() && number > 0.0 && number <= 999.0)
        .unwrap_or(false)
}

pub(crate) fn valid_font_family(value: &str) -> bool {
    !value.trim().is_empty()
        && value.chars().count() <= 128
        && !value
            .chars()
            .any(|character| character.is_control() || character == ';')
}

pub(crate) fn valid_safe_href(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty()
        && trimmed.chars().count() <= 2048
        && !trimmed.chars().any(char::is_control)
        && !trimmed.to_ascii_lowercase().starts_with("javascript:")
        && !trimmed.to_ascii_lowercase().starts_with("data:")
}

fn validate_style_attrs(
    mark: &StyleMarkCapability,
    attrs: Option<&Value>,
) -> Result<(), CapabilityValidationError> {
    if mark.attributes.is_empty() {
        if attrs.is_some() {
            return Err(CapabilityValidationError::style(format!(
                "mark {} 不接受 attrs",
                mark.mark_type
            )));
        }
        return Ok(());
    }
    let attrs = match attrs {
        Some(Value::Object(attrs)) => attrs,
        Some(_) => {
            return Err(CapabilityValidationError::style(format!(
                "mark {}.attrs 必须是 object",
                mark.mark_type
            )))
        }
        None if mark.require_any_attribute || mark.attributes.iter().any(|attr| attr.required) => {
            return Err(CapabilityValidationError::style(format!(
                "mark {} 必须提供 attrs",
                mark.mark_type
            )))
        }
        None => return Ok(()),
    };
    let allowed = mark
        .attributes
        .iter()
        .map(|attr| attr.name.as_str())
        .collect::<Vec<_>>();
    exact_keys(
        attrs,
        &allowed,
        &format!("mark {}.attrs", mark.mark_type),
        CapabilityValidationError::style,
    )?;
    if mark.require_any_attribute && attrs.is_empty() {
        return Err(CapabilityValidationError::style(format!(
            "mark {}.attrs 至少包含一个支持属性",
            mark.mark_type
        )));
    }
    for attr in &mark.attributes {
        let value = attrs.get(&attr.name);
        if attr.required && value.is_none() {
            return Err(CapabilityValidationError::style(format!(
                "mark {}.attrs 缺 {}",
                mark.mark_type, attr.name
            )));
        }
        let Some(value) = value else {
            continue;
        };
        let valid = value.as_str().map(|value| match attr.kind.as_str() {
            "hex_color" => valid_hex_color(value),
            "font_size" => valid_font_size(value),
            "font_family" => valid_font_family(value),
            "safe_href" => valid_safe_href(value),
            _ => false,
        });
        if valid != Some(true) {
            return Err(CapabilityValidationError::style(format!(
                "mark {}.attrs.{} 格式非法",
                mark.mark_type, attr.name
            )));
        }
    }
    Ok(())
}

/**
 * @GOV
 * codes:    BR-SYS-DATA-006-GUARD-AG-AG-TOOL-021, BR-ED-DATA-005-GUARD-AG-AG-TOOL-021
 * type:     GUARD
 * chain:    AG-TOOL
 * rules:    BR-SYS-DATA-006, BR-ED-DATA-005
 * term_ref: TERM-SYS-002, TERM-ED-006, TERM-ED-004
 * boundary: in=UntrustedStyleOpBatchAndAgentEditableSnapshot | out=ValidatedStyleOpBatchOrPreDiffRejection
 */
pub fn validate_style_ops(
    ops: &Value,
    snapshot: &super::blocks::AgentEditableSnapshot,
) -> Result<(), CapabilityValidationError> {
    let value = manifest()?;
    let ops = ops
        .as_array()
        .filter(|ops| !ops.is_empty())
        .ok_or_else(|| CapabilityValidationError::style("style_ops 必须是非空数组"))?;
    if ops.len() > value.style_ops.max_items {
        return Err(CapabilityValidationError::style(format!(
            "style_ops 超过单批上限 {}",
            value.style_ops.max_items
        )));
    }
    for (index, operation) in ops.iter().enumerate() {
        let operation = object(operation, &format!("style_ops[{index}]"))?;
        let op = operation.get("op").and_then(Value::as_str).ok_or_else(|| {
            CapabilityValidationError::style(format!("style_ops[{index}].op 缺失"))
        })?;
        let block_id = operation
            .get("blockId")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                CapabilityValidationError::style(format!("style_ops[{index}].blockId 缺失"))
            })?;
        let block = snapshot
            .blocks
            .iter()
            .find(|block| block.block_id.as_deref() == Some(block_id))
            .ok_or_else(|| {
                CapabilityValidationError::anchor(format!(
                    "style_ops[{index}] 未找到 blockId={block_id}；请重新 list_blocks"
                ))
            })?;
        if snapshot.content_format == "html"
            && snapshot.source == "active_editor"
            && !block
                .capabilities
                .iter()
                .any(|capability| capability == "style_ops")
        {
            return Err(CapabilityValidationError::style(format!(
                "style_ops[{index}] 目标块 {block_id} 是容器、原子块或受保护块"
            )));
        }
        match op {
            "set_mark" => {
                exact_keys(
                    operation,
                    &["op", "blockId", "start", "end", "mark"],
                    &format!("style_ops[{index}]"),
                    CapabilityValidationError::style,
                )?;
                let start = operation
                    .get("start")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| {
                        CapabilityValidationError::style(format!(
                            "style_ops[{index}].start 必须是非负整数"
                        ))
                    })? as usize;
                let end = operation
                    .get("end")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| {
                        CapabilityValidationError::style(format!(
                            "style_ops[{index}].end 必须是非负整数"
                        ))
                    })? as usize;
                let text_len = block.text.encode_utf16().count();
                if start >= end || end > text_len {
                    return Err(CapabilityValidationError::style(format!(
                        "style_ops[{index}] range [{start},{end}) 非法，block chars={text_len}"
                    )));
                }
                let mark = operation.get("mark").ok_or_else(|| {
                    CapabilityValidationError::style(format!("style_ops[{index}].mark 缺失"))
                })?;
                let mark = object(mark, &format!("style_ops[{index}].mark"))?;
                exact_keys(
                    mark,
                    &["type", "attrs"],
                    &format!("style_ops[{index}].mark"),
                    CapabilityValidationError::style,
                )?;
                let mark_type = mark.get("type").and_then(Value::as_str).ok_or_else(|| {
                    CapabilityValidationError::style(format!("style_ops[{index}].mark.type 缺失"))
                })?;
                let capability = value
                    .style_ops
                    .marks
                    .iter()
                    .find(|capability| capability.mark_type == mark_type)
                    .ok_or_else(|| {
                        CapabilityValidationError::style(format!(
                            "style_ops[{index}] 不支持 mark {mark_type}"
                        ))
                    })?;
                validate_style_attrs(capability, mark.get("attrs"))?;
            }
            "unset_mark" => {
                exact_keys(
                    operation,
                    &["op", "blockId", "start", "end", "markType"],
                    &format!("style_ops[{index}]"),
                    CapabilityValidationError::style,
                )?;
                let start = operation
                    .get("start")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| {
                        CapabilityValidationError::style(format!(
                            "style_ops[{index}].start 必须是非负整数"
                        ))
                    })? as usize;
                let end = operation
                    .get("end")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| {
                        CapabilityValidationError::style(format!(
                            "style_ops[{index}].end 必须是非负整数"
                        ))
                    })? as usize;
                let text_len = block.text.encode_utf16().count();
                if start >= end || end > text_len {
                    return Err(CapabilityValidationError::style(format!(
                        "style_ops[{index}] range [{start},{end}) 非法，block chars={text_len}"
                    )));
                }
                let mark_type = operation
                    .get("markType")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        CapabilityValidationError::style(format!(
                            "style_ops[{index}].markType 缺失"
                        ))
                    })?;
                if !value
                    .style_ops
                    .marks
                    .iter()
                    .any(|capability| capability.mark_type == mark_type)
                {
                    return Err(CapabilityValidationError::style(format!(
                        "style_ops[{index}] 不支持 markType {mark_type}"
                    )));
                }
            }
            "set_block_attr" => {
                exact_keys(
                    operation,
                    &["op", "blockId", "attrs"],
                    &format!("style_ops[{index}]"),
                    CapabilityValidationError::style,
                )?;
                let attrs = operation
                    .get("attrs")
                    .and_then(Value::as_object)
                    .filter(|attrs| !attrs.is_empty())
                    .ok_or_else(|| {
                        CapabilityValidationError::style(format!(
                            "style_ops[{index}].attrs 必须是非空 object"
                        ))
                    })?;
                let allowed = value
                    .style_ops
                    .block_attributes
                    .iter()
                    .map(|attr| attr.name.as_str())
                    .collect::<Vec<_>>();
                exact_keys(
                    attrs,
                    &allowed,
                    &format!("style_ops[{index}].attrs"),
                    CapabilityValidationError::style,
                )?;
                for (name, attr_value) in attrs {
                    let capability = value
                        .style_ops
                        .block_attributes
                        .iter()
                        .find(|capability| capability.name == *name)
                        .expect("exact_keys guaranteed capability");
                    if !block
                        .block_type
                        .as_deref()
                        .map(|block_type| {
                            capability
                                .supported_block_types
                                .iter()
                                .any(|supported| supported == block_type)
                        })
                        .unwrap_or(false)
                    {
                        return Err(CapabilityValidationError::style(format!(
                            "style_ops[{index}] 的 blockId={block_id} 不支持块属性 {name}"
                        )));
                    }
                    if attr_value
                        .as_str()
                        .map(|value| capability.values.iter().any(|allowed| allowed == value))
                        != Some(true)
                    {
                        return Err(CapabilityValidationError::style(format!(
                            "style_ops[{index}].attrs.{name} 值非法"
                        )));
                    }
                }
            }
            _ => {
                return Err(CapabilityValidationError::style(format!(
                    "style_ops[{index}] 不支持 op {op}"
                )))
            }
        }
    }
    Ok(())
}

fn validate_field_kind(field: &TableFieldCapability, value: &Value) -> bool {
    match field.kind.as_str() {
        "non_negative_integer" => value.as_u64().is_some(),
        "boolean" => value.as_bool().is_some(),
        "coordinate" => {
            value.as_object().map(|coordinate| {
                coordinate.len() == 2
                    && coordinate.get("row").and_then(Value::as_u64).is_some()
                    && coordinate.get("column").and_then(Value::as_u64).is_some()
            }) == Some(true)
        }
        "string" => value.as_str().is_some(),
        "hex_color_or_null" => value.is_null() || value.as_str().map(valid_hex_color) == Some(true),
        "enum" => {
            value
                .as_str()
                .map(|value| field.values.iter().any(|allowed| allowed == value))
                == Some(true)
        }
        _ => false,
    }
}

pub fn validate_table_operation_shapes(
    operations: &[Value],
) -> Result<(), CapabilityValidationError> {
    let value = manifest()?;
    if operations.is_empty() {
        return Err(CapabilityValidationError::table(
            "table_ops.operations 必须是非空数组",
        ));
    }
    for (index, operation) in operations.iter().enumerate() {
        let operation = operation.as_object().ok_or_else(|| {
            CapabilityValidationError::table(format!("operations[{index}] 必须是 object"))
        })?;
        let op = operation.get("op").and_then(Value::as_str).ok_or_else(|| {
            CapabilityValidationError::table(format!("operations[{index}].op 缺失"))
        })?;
        let capability = value
            .table_ops
            .operations
            .iter()
            .find(|capability| capability.op == op)
            .ok_or_else(|| {
                CapabilityValidationError::table(format!(
                    "operations[{index}] 不支持 TableOperation {op}"
                ))
            })?;
        let mut allowed = vec!["op"];
        allowed.extend(capability.fields.iter().map(|field| field.name.as_str()));
        exact_keys(
            operation,
            &allowed,
            &format!("operations[{index}]"),
            CapabilityValidationError::table,
        )?;
        for field in &capability.fields {
            let field_value = operation.get(&field.name);
            if field.required && field_value.is_none() {
                return Err(CapabilityValidationError::table(format!(
                    "operations[{index}] 缺字段 {}",
                    field.name
                )));
            }
            if let Some(field_value) = field_value {
                if !validate_field_kind(field, field_value) {
                    return Err(CapabilityValidationError::table(format!(
                        "operations[{index}].{} 类型或格式非法",
                        field.name
                    )));
                }
            }
        }
    }
    Ok(())
}

fn valid_relative_project_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 1024
        && !value.starts_with('/')
        && !value.starts_with('\\')
        && !value.contains('\\')
        && !value.contains('\0')
        && !value.contains("://")
        && !value
            .split('/')
            .any(|component| component.is_empty() || component == "." || component == "..")
        && !value
            .as_bytes()
            .get(1)
            .is_some_and(|byte| *byte == b':' && value.as_bytes()[0].is_ascii_alphabetic())
}

fn validate_image_field(field: &TableFieldCapability, value: &Value) -> bool {
    match field.kind.as_str() {
        "positive_integer" => value
            .as_u64()
            .is_some_and(|number| (1..=4096).contains(&number)),
        "relative_path" => value.as_str().is_some_and(valid_relative_project_path),
        "sha256" => value.as_str().is_some_and(|hash| {
            hash.len() == 64
                && hash
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        }),
        "enum" => value
            .as_str()
            .is_some_and(|value| field.values.iter().any(|allowed| allowed == value)),
        "string" if field.name == "alt" => value
            .as_str()
            .is_some_and(|text| text.chars().count() <= 512 && !text.chars().any(char::is_control)),
        "string" => value.as_str().is_some_and(|text| {
            !text.trim().is_empty()
                && text.chars().count() <= 1024
                && !text.chars().any(char::is_control)
        }),
        _ => false,
    }
}

/**
 * @GOV
 * codes:    BR-SYS-DATA-006-GUARD-AG-AG-TOOL-022, BR-ED-DATA-003-GUARD-AG-AG-TOOL-022
 * type:     GUARD
 * chain:    AG-TOOL
 * rules:    BR-SYS-DATA-006, BR-ED-DATA-003
 * term_ref: TERM-SYS-002, TERM-ED-008
 * boundary: in=UntrustedImageOperationBatch | out=ManifestValidatedImageOperationShapesOrPreDiffRejection
 */
pub fn validate_image_operation_shapes(
    operations: &[Value],
) -> Result<(), CapabilityValidationError> {
    let value = manifest()?;
    if operations.is_empty() {
        return Err(CapabilityValidationError::image(
            "image_ops.operations 必须是非空数组",
        ));
    }
    if operations.len() > value.image_ops.max_items {
        return Err(CapabilityValidationError::image(format!(
            "image_ops.operations 超过单批上限 {}",
            value.image_ops.max_items
        )));
    }
    for (index, operation) in operations.iter().enumerate() {
        let operation = operation.as_object().ok_or_else(|| {
            CapabilityValidationError::image(format!("operations[{index}] 必须是 object"))
        })?;
        let op = operation.get("op").and_then(Value::as_str).ok_or_else(|| {
            CapabilityValidationError::image(format!("operations[{index}].op 缺失"))
        })?;
        let capability = value
            .image_ops
            .operations
            .iter()
            .find(|capability| capability.op == op)
            .ok_or_else(|| {
                CapabilityValidationError::image(format!(
                    "operations[{index}] 不支持 ImageOperation {op}"
                ))
            })?;
        let mut allowed = vec!["op"];
        allowed.extend(capability.fields.iter().map(|field| field.name.as_str()));
        exact_keys(
            operation,
            &allowed,
            &format!("operations[{index}]"),
            CapabilityValidationError::image,
        )?;
        for field in &capability.fields {
            let field_value = operation.get(&field.name);
            if field.required && field_value.is_none() {
                return Err(CapabilityValidationError::image(format!(
                    "operations[{index}] 缺字段 {}",
                    field.name
                )));
            }
            if let Some(field_value) = field_value {
                if !validate_image_field(field, field_value) {
                    return Err(CapabilityValidationError::image(format!(
                        "operations[{index}].{} 类型或格式非法",
                        field.name
                    )));
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ag::blocks::{AgentEditableBlock, AgentEditableSnapshot};

    fn snapshot() -> AgentEditableSnapshot {
        AgentEditableSnapshot {
            rel_path: "notes/a.docx".to_string(),
            editable_kind: "bdocx".to_string(),
            source: "active_editor".to_string(),
            content_format: "html".to_string(),
            content: r#"<p data-block-id="b1">正文内容</p>"#.to_string(),
            baseline: None,
            session_version: Some(1),
            content_hash: Some("h1".to_string()),
            blocks: vec![AgentEditableBlock {
                block_id: Some("b1".to_string()),
                block_index: 0,
                block_type: Some("p".to_string()),
                text: "正文内容".to_string(),
                marks: Vec::new(),
                block_attrs: json!({}),
                role: "content".to_string(),
                capabilities: vec!["replace_node".to_string(), "style_ops".to_string()],
                ..Default::default()
            }],
        }
    }

    // covers: BR-SYS-DATA-006
    #[test]
    fn bundled_manifest_is_valid_versioned_and_matches_doc_tiers() {
        let value = manifest().unwrap();
        assert_eq!(value.manifest_id, "binder.bdocx.agent-edit");
        assert_eq!(value.version, "1.6.0");
        assert_eq!(value.editable_kind, "bdocx");
        assert_eq!(value.content_format, "html");
        assert!(value.document_operations.iter().any(|capability| {
            capability.operation == "replace_document"
                && capability.agent_availability == AgentAvailability::Dynamic
                && capability.diff_fidelity == DiffFidelity::Block
        }));
        assert_eq!(manifest_sha256().len(), 64);
        assert!(value
            .roundtrip
            .iter()
            .all(|feature| RoundtripTier::from(classify_style(&feature.feature)) == feature.tier));
    }

    // covers: BR-SYS-DATA-006
    #[test]
    fn prompt_projection_uses_real_mark_names_and_discloses_limits() {
        let prompt = prompt_projection().unwrap();
        assert!(prompt.contains("version=1.6.0"));
        assert!(prompt.contains("block_ops"));
        assert!(prompt.contains("两者都不存在时，才可用 replace_document"));
        assert!(prompt.contains("新增 heading 块"));
        assert!(prompt.contains("既有块的类型转换是人工专属"));
        assert!(prompt.contains("utf16_code_unit"));
        assert!(prompt.contains("subscript"));
        assert!(prompt.contains("superscript"));
        assert!(!prompt.contains("sub/sup"));
        assert!(prompt.contains("人工专属"));
        assert!(prompt.contains("list_images"));
        assert!(prompt.contains("insert_image"));
        assert!(prompt.contains("resize_image"));
        assert!(prompt.contains("set_image_layout"));
        assert!(prompt.contains("delete_image 必须独占"));
        assert!(prompt.contains("不提供图片像素"));
        assert!(prompt.contains("import_image_asset"));
        assert!(prompt.contains("edit_blocked"));
        assert!(prompt.contains("best_effort"));
        assert!(prompt.contains("unsupported"));
    }

    // covers: BR-SYS-DATA-006
    #[test]
    fn propose_edit_schema_has_conditional_block_style_table_and_image_shapes() {
        let schema = propose_edit_parameters_schema().unwrap();
        assert!(schema["allOf"].as_array().unwrap().len() >= 5);
        let style_items = &schema["properties"]["style_ops"]["items"]["oneOf"];
        assert!(style_items.as_array().unwrap().len() > 3);
        let table_items = &schema["allOf"][4]["then"]["properties"]["operations"]["items"]["oneOf"];
        assert_eq!(
            table_items.as_array().unwrap().len(),
            manifest().unwrap().table_ops.operations.len()
        );
        // 模型可见上限必须与运行时判定同源：任何一方另写数字都会让"schema 合法却被拒"复发。
        assert_eq!(
            schema["allOf"][1]["then"]["properties"]["operations"]["maxItems"].as_u64(),
            Some(crate::ag::edit_support::MAX_BLOCK_OPS_PER_CALL as u64),
            "block_ops schema maxItems 必须引用运行时单次上限"
        );
        // 生成粒度：schema 必须说明系统会自动拆卡，模型写完一章直接提交即可。
        let ops_description = schema["allOf"][1]["then"]["properties"]["operations"]["description"]
            .as_str()
            .unwrap_or_default();
        assert!(ops_description.contains("写完一章直接提交"));
        assert!(ops_description.contains("adjudicationGroup"));
        let serialized = schema.to_string();
        assert!(serialized.contains("subscript"));
        assert!(serialized.contains("replace_node"));
        assert!(serialized.contains("expectedNodeType"));
        assert!(serialized.contains("adjudicationGroup"));
        assert!(schema["properties"].get("batch").is_none());
        assert!(serialized.contains("set_cell_background"));
        assert!(serialized.contains("insert_image"));
        assert!(serialized.contains("resize_image"));
        assert!(serialized.contains("set_image_layout"));
        assert!(serialized.contains("assetId"));
        assert!(serialized.contains("sourceHash"));
        assert!(serialized.contains("additionalProperties"));
        assert!(serialized.contains("当前快照无表格且无图片时可用"));
    }

    #[test]
    fn bdocx_rewrite_route_is_explicit_about_snapshot_protection_and_heading_insertion() {
        let unprotected = bdocx_rewrite_route_instruction(false, false);
        assert!(unprotected.contains("未检测到表格或图片"));
        assert!(unprotected.contains("replace_document"));
        assert!(unprotected.contains("新增小标题"));
        assert!(unprotected.contains("不能把既有段落转换成标题"));

        let protected = bdocx_rewrite_route_instruction(true, true);
        assert!(protected.contains("检测到表格和图片"));
        assert!(protected.contains("replace_document 已被禁止"));
        assert!(protected.contains("文字内容只能用 block_ops"));
        assert!(protected.contains("表格只能用 table_ops"));
        assert!(protected.contains("图片只能用 image_ops"));
    }

    // covers: BR-SYS-DATA-006, BR-ED-DATA-005
    #[test]
    fn style_validation_accepts_supported_shapes_and_rejects_alias_attrs_anchor_range() {
        let snapshot = snapshot();
        let valid = json!([
            { "op": "set_mark", "blockId": "b1", "start": 0, "end": 2, "mark": { "type": "bold" } },
            { "op": "set_mark", "blockId": "b1", "start": 0, "end": 2, "mark": { "type": "textStyle", "attrs": { "color": "#FF0000", "fontSize": "12pt" } } },
            { "op": "set_mark", "blockId": "b1", "start": 0, "end": 2, "mark": { "type": "link", "attrs": { "href": "https://example.com" } } },
            { "op": "set_block_attr", "blockId": "b1", "attrs": { "textAlign": "center" } }
        ]);
        assert!(validate_style_ops(&valid, &snapshot).is_ok());

        for invalid in [
            json!([{ "op": "set_mark", "blockId": "b1", "start": 0, "end": 2, "mark": { "type": "sub" } }]),
            json!([{ "op": "set_mark", "blockId": "b1", "start": 0, "end": 2, "mark": { "type": "bold", "attrs": {} } }]),
            json!([{ "op": "set_mark", "blockId": "b1", "start": 0, "end": 2, "mark": { "type": "bold", "attrs": { "color": "#FF0000" } } }]),
            json!([{ "op": "set_mark", "blockId": "b1", "start": 0, "end": 2, "mark": { "type": "textStyle", "attrs": { "fontSize": "01pt" } } }]),
            json!([{ "op": "set_mark", "blockId": "missing", "start": 0, "end": 2, "mark": { "type": "bold" } }]),
            json!([{ "op": "set_mark", "blockId": "b1", "start": 2, "end": 99, "mark": { "type": "bold" } }]),
            json!([{ "op": "set_mark", "blockId": "b1", "start": 1, "end": 1, "mark": { "type": "bold" } }]),
        ] {
            assert!(validate_style_ops(&invalid, &snapshot).is_err());
        }
    }

    // covers: BR-SYS-DATA-006, BR-ED-DATA-005
    #[test]
    fn style_range_uses_editor_utf16_plain_text_units() {
        let mut snapshot = snapshot();
        snapshot.blocks[0].text = "A😀B".to_string();
        let valid = json!([
            { "op": "set_mark", "blockId": "b1", "start": 1, "end": 3, "mark": { "type": "bold" } }
        ]);
        assert!(validate_style_ops(&valid, &snapshot).is_ok());
        let invalid = json!([
            { "op": "set_mark", "blockId": "b1", "start": 3, "end": 5, "mark": { "type": "bold" } }
        ]);
        assert!(validate_style_ops(&invalid, &snapshot).is_err());
    }

    // covers: BR-SYS-DATA-006
    #[test]
    fn table_shape_validation_is_manifest_driven_and_strict() {
        assert!(validate_table_operation_shapes(&[
            json!({ "op": "set_cell_background", "at": { "row": 0, "column": 1 }, "color": "#FFE699" }),
            json!({ "op": "move_table", "direction": "down" }),
        ])
        .is_ok());
        assert!(validate_table_operation_shapes(&[
            json!({ "op": "set_cell_background", "at": { "row": 0, "column": 1 } })
        ])
        .is_err());
        assert!(validate_table_operation_shapes(&[
            json!({ "op": "move_table", "direction": "left" })
        ])
        .is_err());
        assert!(validate_table_operation_shapes(&[json!({ "op": "delete_table" })]).is_err());
    }

    // covers: BR-SYS-DATA-006, BR-ED-DATA-003
    #[test]
    fn image_shape_validation_is_manifest_driven_and_rejects_unsafe_sources() {
        assert!(validate_image_operation_shapes(&[
            json!({
                "op": "insert_image",
                "anchorBlockId": "b1",
                "position": "after",
                "assetId": "asset_cover",
                "sourcePath": "images/cover.png",
                "sourceHash": "a".repeat(64),
                "alt": "封面",
                "width": 640
            }),
            json!({ "op": "delete_image", "imageId": "image_0", "fingerprint": "fp" }),
            json!({ "op": "resize_image", "imageId": "image_0", "fingerprint": "fp", "width": 320 }),
            json!({ "op": "move_image", "imageId": "image_0", "fingerprint": "fp", "direction": "up" }),
            json!({ "op": "set_image_layout", "imageId": "image_0", "fingerprint": "fp", "alignment": "center" }),
        ])
        .is_ok());
        for invalid in [
            json!([{ "op": "insert_image", "anchorBlockId": "b1", "position": "after", "assetId": "asset_cover", "sourcePath": "../cover.png", "sourceHash": "a".repeat(64) }]),
            json!([{ "op": "insert_image", "anchorBlockId": "b1", "position": "middle", "assetId": "asset_cover", "sourcePath": "images/cover.png", "sourceHash": "a".repeat(64) }]),
            json!([{ "op": "insert_image", "anchorBlockId": "b1", "position": "after", "assetId": "asset_cover", "sourcePath": "images/cover.png", "sourceHash": "ABC" }]),
            json!([{ "op": "insert_image", "anchorBlockId": "b1", "position": "after", "assetId": "asset_cover", "sourcePath": "images/cover.png", "sourceHash": "a".repeat(64), "width": 0 }]),
            json!([{ "op": "delete_image", "imageId": "image_0" }]),
            json!([{ "op": "move_image", "imageId": "image_0", "fingerprint": "fp", "direction": "left" }]),
            json!([{ "op": "set_image_layout", "imageId": "image_0", "fingerprint": "fp", "alignment": "justify" }]),
        ] {
            let operations = invalid.as_array().unwrap();
            assert!(validate_image_operation_shapes(operations).is_err());
        }
    }
}
