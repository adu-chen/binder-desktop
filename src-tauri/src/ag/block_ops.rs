//! 富文档结构化块操作校验。
//!
//! BlockOperation 只允许替换当前 ProseMirror LogicalState 中已声明 `replace_node`
//! 能力的 textblock。后端补齐 beforeNode 并把整批操作固化进 Diff；前端再以同一批锚、
//! 节点类型和 fingerprint 原子应用或精确回滚。

use crate::ag::blocks::{AgentEditableBlock, AgentEditableSnapshot};
use crate::ipc::IpcError;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeSet;

const MAX_OPERATIONS: usize = 32;
const MAX_TEXT_UTF16: usize = 200_000;
const WRITABLE_NODE_TYPES: &[&str] = &["paragraph", "heading", "codeBlock"];
const INLINE_NODE_TYPES: &[&str] = &["text", "hardBreak"];
const MARK_TYPES: &[&str] = &[
    "bold",
    "italic",
    "underline",
    "strike",
    "subscript",
    "superscript",
    "code",
    "link",
    "highlight",
    "textStyle",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StructuredBlockBase {
    pub session_version: u64,
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RequestedBlockOperation {
    op: String,
    #[serde(default)]
    block_id: String,
    #[serde(default)]
    fingerprint: String,
    #[serde(default)]
    expected_node_type: String,
    // AG-TASK-01 F2：delete_node 无 afterNode（删除标记 null）。
    #[serde(default)]
    after_node: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuredBlockOperation {
    pub op: String,
    pub block_id: String,
    pub fingerprint: String,
    pub expected_node_type: String,
    pub before_node: Value,
    pub after_node: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuredBlockEditPayload {
    pub payload_version: u8,
    pub base: StructuredBlockBase,
    pub operations: Vec<StructuredBlockOperation>,
}

fn invalid(message: impl AsRef<str>) -> IpcError {
    IpcError::new("AG_BLOCK_OPERATION_INVALID", message.as_ref(), false, false)
}

fn stale(message: impl AsRef<str>) -> IpcError {
    IpcError::new(
        "AG_BLOCK_OPERATION_BASE_STALE",
        message.as_ref(),
        true,
        false,
    )
}

fn object<'a>(value: &'a Value, label: &str) -> Result<&'a Map<String, Value>, IpcError> {
    value
        .as_object()
        .ok_or_else(|| invalid(format!("{label} 必须是对象")))
}

fn exact_keys(object: &Map<String, Value>, allowed: &[&str], label: &str) -> Result<(), IpcError> {
    if let Some(key) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(invalid(format!("{label} 包含未允许字段 {key}")));
    }
    Ok(())
}

fn validate_mark(value: &Value) -> Result<(), IpcError> {
    let mark = object(value, "mark")?;
    exact_keys(mark, &["type", "attrs"], "mark")?;
    let mark_type = mark
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("mark.type 必须是字符串"))?;
    if !MARK_TYPES.contains(&mark_type) {
        return Err(invalid(format!("不支持 mark 类型 {mark_type}")));
    }
    if let Some(attrs) = mark.get("attrs") {
        let attrs = object(attrs, "mark.attrs")?;
        match mark_type {
            "link" => {
                exact_keys(attrs, &["href", "target", "rel", "class"], "link.attrs")?;
                let href = attrs
                    .get("href")
                    .and_then(Value::as_str)
                    .ok_or_else(|| invalid("link mark 必须提供 href"))?;
                if !crate::ag::bdocx_capabilities::valid_safe_href(href) {
                    return Err(invalid("link href 使用了不安全协议"));
                }
                for key in ["target", "rel", "class"] {
                    if attrs
                        .get(key)
                        .is_some_and(|value| !value.is_null() && !value.is_string())
                    {
                        return Err(invalid(format!("link.attrs.{key} 必须是字符串或 null")));
                    }
                }
            }
            "highlight" => {
                exact_keys(attrs, &["color"], "highlight.attrs")?;
                if attrs.get("color").is_some_and(|value| {
                    !value.is_null()
                        && value
                            .as_str()
                            .map(crate::ag::bdocx_capabilities::valid_hex_color)
                            != Some(true)
                }) {
                    return Err(invalid("highlight.color 必须是 #RRGGBB 或 null"));
                }
            }
            "textStyle" => {
                exact_keys(
                    attrs,
                    &["color", "fontFamily", "fontSize"],
                    "textStyle.attrs",
                )?;
                for (key, validator) in [
                    (
                        "color",
                        crate::ag::bdocx_capabilities::valid_hex_color as fn(&str) -> bool,
                    ),
                    (
                        "fontFamily",
                        crate::ag::bdocx_capabilities::valid_font_family as fn(&str) -> bool,
                    ),
                    (
                        "fontSize",
                        crate::ag::bdocx_capabilities::valid_font_size as fn(&str) -> bool,
                    ),
                ] {
                    if attrs.get(key).is_some_and(|value| {
                        !value.is_null() && value.as_str().map(validator) != Some(true)
                    }) {
                        return Err(invalid(format!("textStyle.attrs.{key} 类型或格式非法")));
                    }
                }
            }
            _ if !attrs.is_empty() => {
                return Err(invalid(format!("{mark_type} mark 不接受 attrs")));
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_inline_node(value: &Value, text_utf16: &mut usize) -> Result<(), IpcError> {
    let node = object(value, "afterNode.content[]")?;
    let node_type = node
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("inline node 缺 type"))?;
    if !INLINE_NODE_TYPES.contains(&node_type) {
        return Err(invalid(format!(
            "结构化块内容不允许 inline node 类型 {node_type}"
        )));
    }
    match node_type {
        "text" => {
            exact_keys(node, &["type", "text", "marks"], "text node")?;
            let text = node
                .get("text")
                .and_then(Value::as_str)
                .filter(|text| !text.is_empty())
                .ok_or_else(|| invalid("text node 必须提供非空 text"))?;
            *text_utf16 = text_utf16.saturating_add(text.encode_utf16().count());
            if let Some(marks) = node.get("marks") {
                let marks = marks
                    .as_array()
                    .ok_or_else(|| invalid("text.marks 必须是数组"))?;
                for mark in marks {
                    validate_mark(mark)?;
                }
            }
        }
        "hardBreak" => exact_keys(node, &["type"], "hardBreak node")?,
        _ => unreachable!(),
    }
    Ok(())
}

fn validate_after_node(
    operation: &RequestedBlockOperation,
    target: &AgentEditableBlock,
) -> Result<(), IpcError> {
    let after_node = operation.after_node.as_ref().ok_or_else(|| {
        invalid("replace_node 必须提供 afterNode")
    })?;
    let node = object(after_node, "afterNode")?;
    exact_keys(node, &["type", "attrs", "content"], "afterNode")?;
    let node_type = node
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("afterNode.type 必须是字符串"))?;
    if node_type != operation.expected_node_type
        || !WRITABLE_NODE_TYPES.contains(&node_type)
        || target.node_type.as_deref() != Some(node_type)
    {
        return Err(invalid("afterNode.type 与目标节点类型不一致或不在可写子集"));
    }
    let attrs = node
        .get("attrs")
        .cloned()
        .unwrap_or_else(|| Value::Object(Map::new()));
    if attrs != target.block_attrs {
        return Err(invalid(
            "内容块操作不得修改 attrs；样式或布局必须使用对应结构化操作",
        ));
    }
    if attrs.get("blockId").and_then(Value::as_str) != target.block_id.as_deref() {
        return Err(invalid("afterNode 必须保留目标 blockId"));
    }
    let mut text_utf16 = 0usize;
    if let Some(content) = node.get("content") {
        let content = content
            .as_array()
            .ok_or_else(|| invalid("afterNode.content 必须是数组"))?;
        for child in content {
            validate_inline_node(child, &mut text_utf16)?;
        }
    }
    if text_utf16 > MAX_TEXT_UTF16 {
        return Err(invalid("结构化块操作文本超过大小限制"));
    }
    Ok(())
}

fn target_for<'a>(
    snapshot: &'a AgentEditableSnapshot,
    block_id: &str,
) -> Result<&'a AgentEditableBlock, IpcError> {
    snapshot
        .blocks
        .iter()
        .find(|block| block.block_id.as_deref() == Some(block_id))
        .ok_or_else(|| {
            IpcError::new(
                "AG_ANCHOR_LOST",
                &format!("未找到 block_id={block_id} 的结构化块"),
                true,
                false,
            )
        })
}

/// AG-STRATEGY-04：为 insert/append 新块兜底生成唯一 attrs.blockId（模型未显式给时）。
/// 使前端 nodeFromJSON 生成的新块自带稳定 id，可被跨批/后续操作锚定，根治"新块无 id 错乱"。
fn ensure_new_block_id(after_node: Value) -> Value {
    let mut node = after_node;
    if let Some(obj) = node.as_object_mut() {
        let block_id = obj
            .get("attrs")
            .and_then(|attrs| attrs.get("blockId"))
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| format!("new_block_{}", uuid::Uuid::new_v4()));
        let attrs = obj
            .entry("attrs")
            .or_insert_with(|| Value::Object(Map::new()));
        if let Some(attrs_obj) = attrs.as_object_mut() {
            attrs_obj.insert("blockId".to_string(), Value::String(block_id));
        }
    }
    node
}

/**
 * @GOV
 * codes:    BR-AG-DATA-003-RB-AG-AG-TOOL-016, BR-ED-DATA-001-RB-AG-AG-TOOL-016, BR-DE-DATA-001-RB-AG-AG-TOOL-016
 * type:     RB
 * chain:    AG-TOOL
 * rules:    BR-AG-DATA-003, BR-ED-DATA-001, BR-DE-DATA-001
 * term_ref: TERM-ED-001
 * boundary: in=RequestedStructuredBlockOperationsAndCurrentAgentEditableSnapshot | out=CanonicalStructuredBlockEditPayloadWithExactBeforeNode | effect=none
 */
pub fn validate_and_build(
    params: &Value,
    snapshot: &AgentEditableSnapshot,
) -> Result<StructuredBlockEditPayload, IpcError> {
    if snapshot.content_format != "html" || snapshot.source != "active_editor" {
        return Err(IpcError::new(
            "AG_BLOCK_OPERATION_REQUIRES_ACTIVE_EDITOR",
            "富文档结构化块操作只支持当前活动编辑器 LogicalState",
            false,
            false,
        ));
    }
    let base: StructuredBlockBase = serde_json::from_value(
        params
            .get("base")
            .cloned()
            .ok_or_else(|| invalid("operation=block_ops 必须提供 base"))?,
    )
    .map_err(|error| invalid(format!("base 无效: {error}")))?;
    if Some(base.session_version) != snapshot.session_version
        || Some(base.content_hash.as_str()) != snapshot.content_hash.as_deref()
    {
        return Err(stale(
            "BlockOperation base 与当前 sessionVersion/contentHash 不一致，请重新读取文档",
        ));
    }
    let requested: Vec<RequestedBlockOperation> = serde_json::from_value(
        params
            .get("operations")
            .cloned()
            .ok_or_else(|| invalid("operation=block_ops 必须提供 operations"))?,
    )
    .map_err(|error| invalid(format!("operations 无效: {error}")))?;
    if requested.is_empty() || requested.len() > MAX_OPERATIONS {
        return Err(invalid(format!(
            "operations 数量必须为 1..={MAX_OPERATIONS}"
        )));
    }
    let mut seen = BTreeSet::new();
    let mut canonical = Vec::with_capacity(requested.len());
    for operation in requested {
        match operation.op.as_str() {
            "replace_node" => {
                if operation.block_id.trim().is_empty()
                    || operation.fingerprint.trim().is_empty()
                    || operation.expected_node_type.trim().is_empty()
                {
                    return Err(invalid(
                        "replace_node 必须提供 blockId/fingerprint/expectedNodeType",
                    ));
                }
                if !seen.insert(operation.block_id.clone()) {
                    return Err(invalid(format!(
                        "同一批操作重复命中 blockId {}",
                        operation.block_id
                    )));
                }
                let target = target_for(snapshot, &operation.block_id)?;
                if target.role != "content"
                    || !target
                        .capabilities
                        .iter()
                        .any(|capability| capability == "replace_node")
                    || target.node_json.is_null()
                {
                    return Err(IpcError::new(
                        "AG_BLOCK_OPERATION_TARGET_READONLY",
                        &format!(
                            "块 {} 是容器、原子块或受保护块，不能通用替换",
                            operation.block_id
                        ),
                        false,
                        false,
                    ));
                }
                if target.fingerprint.as_deref() != Some(operation.fingerprint.as_str())
                    || target.node_type.as_deref() != Some(operation.expected_node_type.as_str())
                {
                    return Err(stale(format!(
                        "块 {} 的类型或 fingerprint 已变化，请重新读取文档",
                        operation.block_id
                    )));
                }
                validate_after_node(&operation, target)?;
                canonical.push(StructuredBlockOperation {
                    op: operation.op,
                    block_id: operation.block_id,
                    fingerprint: operation.fingerprint,
                    expected_node_type: operation.expected_node_type,
                    before_node: target.node_json.clone(),
                    after_node: operation.after_node.unwrap_or(Value::Null),
                });
            }
            "insert_before" | "insert_after" => {
                if operation.block_id.trim().is_empty() {
                    return Err(invalid(format!(
                        "{} 必须提供 blockId 作为锚点块",
                        operation.op
                    )));
                }
                // 校验锚点块存在
                let anchor = target_for(snapshot, &operation.block_id)?;
                if anchor.role != "content" {
                    return Err(IpcError::new(
                        "AG_BLOCK_OPERATION_TARGET_READONLY",
                        &format!(
                            "锚点块 {} 不是 content 块，不能在附近插入",
                            operation.block_id
                        ),
                        false,
                        false,
                    ));
                }
                if anchor.node_json.is_null() {
                    return Err(IpcError::new(
                        "AG_BLOCK_OPERATION_TARGET_READONLY",
                        &format!("锚点块 {} 缺少 node_json", operation.block_id),
                        false,
                        false,
                    ));
                }
                // 校验 afterNode 结构（节点类型和内容）
                let after_node = operation.after_node.as_ref().ok_or_else(|| {
                    invalid("insert_before/insert_after 必须提供 afterNode")
                })?;
                let node = object(after_node, "afterNode")?;
                let node_type = node
                    .get("type")
                    .and_then(Value::as_str)
                    .ok_or_else(|| invalid("afterNode.type 必须是字符串"))?;
                if !WRITABLE_NODE_TYPES.contains(&node_type) {
                    return Err(invalid(format!(
                        "afterNode.type {} 不在可写子集",
                        node_type
                    )));
                }
                // 新块 attrs 可含 blockId；缺省由后端兜底生成唯一 id（AG-STRATEGY-04）。
                let mut text_utf16 = 0usize;
                if let Some(content) = node.get("content") {
                    let content = content
                        .as_array()
                        .ok_or_else(|| invalid("afterNode.content 必须是数组"))?;
                    for child in content {
                        validate_inline_node(child, &mut text_utf16)?;
                    }
                }
                if text_utf16 > MAX_TEXT_UTF16 {
                    return Err(invalid("结构化块操作文本超过大小限制"));
                }
                let empty_before = Value::Object(Map::new());
                canonical.push(StructuredBlockOperation {
                    op: operation.op.clone(),
                    block_id: operation.block_id.clone(),
                    fingerprint: String::new(),
                    expected_node_type: node_type.to_string(),
                    before_node: empty_before,
                    after_node: ensure_new_block_id(operation.after_node.unwrap_or(Value::Null)),
                });
            }
            "append" => {
                // 校验 afterNode 结构
                let after_node = operation.after_node.as_ref().ok_or_else(|| {
                    invalid("append 必须提供 afterNode")
                })?;
                let node = object(after_node, "afterNode")?;
                let node_type = node
                    .get("type")
                    .and_then(Value::as_str)
                    .ok_or_else(|| invalid("afterNode.type 必须是字符串"))?;
                if !WRITABLE_NODE_TYPES.contains(&node_type) {
                    return Err(invalid(format!(
                        "afterNode.type {} 不在可写子集",
                        node_type
                    )));
                }
                let mut text_utf16 = 0usize;
                if let Some(content) = node.get("content") {
                    let content = content
                        .as_array()
                        .ok_or_else(|| invalid("afterNode.content 必须是数组"))?;
                    for child in content {
                        validate_inline_node(child, &mut text_utf16)?;
                    }
                }
                if text_utf16 > MAX_TEXT_UTF16 {
                    return Err(invalid("结构化块操作文本超过大小限制"));
                }
                let empty_before = Value::Object(Map::new());
                canonical.push(StructuredBlockOperation {
                    op: operation.op,
                    block_id: String::new(),
                    fingerprint: String::new(),
                    expected_node_type: node_type.to_string(),
                    before_node: empty_before,
                    after_node: ensure_new_block_id(operation.after_node.unwrap_or(Value::Null)),
                });
            }
            // AG-TASK-01 F2：删除块。仅允许 role=content 且 capabilities 含 delete_node 的原子块；
            // 容器/原子块（含子块）拒绝级联删除。删除不需要 afterNode/fingerprint（无 after 节点）。
            "delete_node" => {
                if operation.block_id.trim().is_empty() {
                    return Err(invalid("delete_node 必须提供 blockId"));
                }
                if !seen.insert(operation.block_id.clone()) {
                    return Err(invalid(format!(
                        "同一批操作重复命中 blockId {}",
                        operation.block_id
                    )));
                }
                let target = target_for(snapshot, &operation.block_id)?;
                if target.role != "content"
                    || !target
                        .capabilities
                        .iter()
                        .any(|capability| capability == "delete_node")
                {
                    return Err(IpcError::new(
                        "AG_BLOCK_OPERATION_TARGET_READONLY",
                        &format!(
                            "块 {} 是容器、原子块或受保护块，不能删除（1.0 不级联删除子树）",
                            operation.block_id
                        ),
                        false,
                        false,
                    ));
                }
                if target.node_json.is_null() {
                    return Err(IpcError::new(
                        "AG_BLOCK_OPERATION_TARGET_READONLY",
                        &format!("块 {} 缺少 node_json", operation.block_id),
                        false,
                        false,
                    ));
                }
                // 删除块：beforeNode 为被删块完整节点，afterNode 置 null 标记删除（前端渲染删除线）。
                canonical.push(StructuredBlockOperation {
                    op: operation.op,
                    block_id: operation.block_id,
                    fingerprint: operation.fingerprint,
                    expected_node_type: operation.expected_node_type,
                    before_node: target.node_json.clone(),
                    after_node: Value::Null,
                });
            }
            _ => {
                return Err(invalid(format!(
                    "不支持的 BlockOperation 类型: {}",
                    operation.op
                )));
            }
        }
    }
    Ok(StructuredBlockEditPayload {
        payload_version: 1,
        base,
        operations: canonical,
    })
}

pub fn node_text(value: &Value) -> String {
    match value {
        Value::Object(object) => {
            if object.get("type").and_then(Value::as_str) == Some("hardBreak") {
                return "\n".to_string();
            }
            if let Some(text) = object.get("text").and_then(Value::as_str) {
                return text.to_string();
            }
            object
                .get("content")
                .and_then(Value::as_array)
                .map(|content| content.iter().map(node_text).collect::<String>())
                .unwrap_or_default()
        }
        Value::Array(items) => items.iter().map(node_text).collect(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> AgentEditableSnapshot {
        AgentEditableSnapshot {
            rel_path: "a.draft.docx".into(),
            editable_kind: "bdocx".into(),
            source: "active_editor".into(),
            content_format: "html".into(),
            content: "<ul><li><p>第一条</p></li></ul>".into(),
            baseline: Some(1),
            session_version: Some(7),
            content_hash: Some("base-hash".into()),
            blocks: vec![AgentEditableBlock {
                block_id: Some("p-1".into()),
                block_index: 2,
                block_type: Some("p".into()),
                node_type: Some("paragraph".into()),
                role: "content".into(),
                parent_block_id: Some("item-1".into()),
                depth: 2,
                ordinal: 0,
                text: "第一条".into(),
                marks: vec![],
                block_attrs: serde_json::json!({"blockId": "p-1", "textAlign": null}),
                node_json: serde_json::json!({
                    "type": "paragraph",
                    "attrs": {"blockId": "p-1", "textAlign": null},
                    "content": [{"type": "text", "text": "第一条"}]
                }),
                fingerprint: Some("block-v1-abc".into()),
                capabilities: vec!["replace_node".into(), "style_ops".into()],
            }],
        }
    }

    #[test]
    fn canonicalizes_before_node_and_preserves_block_identity() {
        let params = serde_json::json!({
            "base": {"sessionVersion": 7, "contentHash": "base-hash"},
            "operations": [{
                "op": "replace_node",
                "blockId": "p-1",
                "fingerprint": "block-v1-abc",
                "expectedNodeType": "paragraph",
                "afterNode": {
                    "type": "paragraph",
                    "attrs": {"blockId": "p-1", "textAlign": null},
                    "content": [{"type": "text", "text": "新第一条", "marks": [{"type": "bold"}]}]
                }
            }]
        });
        let payload = validate_and_build(&params, &snapshot()).unwrap();
        assert_eq!(payload.operations.len(), 1);
        assert_eq!(node_text(&payload.operations[0].before_node), "第一条");
        assert_eq!(node_text(&payload.operations[0].after_node), "新第一条");
    }

    // covers: BR-AG-DATA-002
    // AG-STRATEGY-04：insert/append 新块缺省时后端兜底生成唯一 attrs.blockId（跨批可锚定）。
    #[test]
    fn insert_new_block_gets_fallback_unique_block_id() {
        let params = serde_json::json!({
            "base": {"sessionVersion": 7, "contentHash": "base-hash"},
            "operations": [{
                "op": "insert_before",
                "blockId": "p-1",
                "afterNode": {
                    "type": "heading",
                    "content": [{"type": "text", "text": "寒冬路上"}]
                }
            }]
        });
        let payload = validate_and_build(&params, &snapshot()).unwrap();
        assert_eq!(payload.operations.len(), 1);
        let block_id = payload.operations[0].after_node["attrs"]["blockId"]
            .as_str()
            .expect("insert 新块应有兜底 attrs.blockId")
            .to_string();
        assert!(
            block_id.starts_with("new_block_"),
            "兜底 id 应为 new_block_ 前缀，实际 {block_id}"
        );
        // 显式给 blockId 时保留模型值，不覆盖。
        let params_explicit = serde_json::json!({
            "base": {"sessionVersion": 7, "contentHash": "base-hash"},
            "operations": [{
                "op": "append",
                "afterNode": {
                    "type": "paragraph",
                    "attrs": {"blockId": "my-title"},
                    "content": [{"type": "text", "text": "教训"}]
                }
            }]
        });
        let explicit = validate_and_build(&params_explicit, &snapshot()).unwrap();
        assert_eq!(
            explicit.operations[0].after_node["attrs"]["blockId"],
            "my-title"
        );
    }

    // covers: BR-AG-DATA-002
    // AG-TASK-01 F2：delete_node——content 块可删（beforeNode 为被删块，afterNode=null 标记）；
    // 容器/受保护块拒绝（不级联删除子树）。
    #[test]
    fn delete_node_allows_content_block_and_rejects_container() {
        let mut current = snapshot();
        current.blocks[0].capabilities = vec!["delete_node".into()];
        let ok = validate_and_build(
            &serde_json::json!({
                "base": {"sessionVersion": 7, "contentHash": "base-hash"},
                "operations": [{
                    "op": "delete_node",
                    "blockId": "p-1",
                    "expectedNodeType": "paragraph"
                }]
            }),
            &current,
        )
        .unwrap();
        assert_eq!(ok.operations.len(), 1);
        assert_eq!(ok.operations[0].op, "delete_node");
        assert_eq!(ok.operations[0].after_node, serde_json::Value::Null);
        assert_eq!(node_text(&ok.operations[0].before_node), "第一条");

        // 容器块（无 delete_node 能力）→ 拒绝。
        let mut container = snapshot();
        container.blocks[0].role = "container".into();
        container.blocks[0].capabilities.clear();
        let err = validate_and_build(
            &serde_json::json!({
                "base": {"sessionVersion": 7, "contentHash": "base-hash"},
                "operations": [{
                    "op": "delete_node",
                    "blockId": "p-1",
                    "expectedNodeType": "paragraph"
                }]
            }),
            &container,
        )
        .unwrap_err();
        assert_eq!(err.code, "AG_BLOCK_OPERATION_TARGET_READONLY");
    }

    #[test]
    fn rejects_container_or_stale_or_structural_descendant() {
        let mut current = snapshot();
        current.blocks[0].role = "container".into();
        current.blocks[0].capabilities.clear();
        let params = serde_json::json!({
            "base": {"sessionVersion": 7, "contentHash": "base-hash"},
            "operations": [{
                "op": "replace_node",
                "blockId": "p-1",
                "fingerprint": "block-v1-abc",
                "expectedNodeType": "paragraph",
                "afterNode": {
                    "type": "paragraph",
                    "attrs": {"blockId": "p-1", "textAlign": null},
                    "content": [{"type": "image", "attrs": {"src": "x"}}]
                }
            }]
        });
        assert_eq!(
            validate_and_build(&params, &current).unwrap_err().code,
            "AG_BLOCK_OPERATION_TARGET_READONLY"
        );
        current.blocks[0].role = "content".into();
        current.blocks[0].capabilities = vec!["replace_node".into(), "style_ops".into()];
        current.content_hash = Some("changed".into());
        assert_eq!(
            validate_and_build(&params, &current).unwrap_err().code,
            "AG_BLOCK_OPERATION_BASE_STALE"
        );
        current.content_hash = Some("base-hash".into());
        assert_eq!(
            validate_and_build(&params, &current).unwrap_err().code,
            "AG_BLOCK_OPERATION_INVALID"
        );
    }
}
