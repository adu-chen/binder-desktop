//! Agent 编辑操作的纯逻辑解析、结构保护和共享校验。

use super::edit_types::AgentEditOperation;
use crate::ag;
use crate::ipc::IpcError;

pub(super) fn now_millis() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 不透明 id（diff_id / payload_ref）：前缀 + UUIDv4（G-4 定档，A-CORE-X-R-03 §7，与 lifecycle 同策略）。
pub(super) fn gen_id(prefix: &str) -> String {
    format!("{prefix}_{}", uuid::Uuid::new_v4())
}

pub(super) fn doc_type_of(rel_path: &str) -> String {
    std::path::Path::new(rel_path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_else(|| "bin".to_string())
}

pub(super) fn is_docx_rel_path(rel_path: &str) -> bool {
    matches!(doc_type_of(rel_path).as_str(), "docx" | "doc")
}

#[cfg(test)]
pub(super) fn live_agent_edit_base<'a>(
    target_rel: &str,
    current_rel: Option<&str>,
    base_content: Option<&'a str>,
) -> Option<&'a str> {
    if current_rel == Some(target_rel) {
        base_content
    } else {
        None
    }
}

pub(super) fn style_ops_param(params: &serde_json::Value) -> Option<&serde_json::Value> {
    params
        .get("style_ops")
        .or_else(|| params.get("styleOps"))
        .filter(|v| match v {
            serde_json::Value::Array(items) => !items.is_empty(),
            serde_json::Value::Null => false,
            _ => true,
        })
}

pub(super) fn capability_error_to_ipc(
    error: ag::bdocx_capabilities::CapabilityValidationError,
) -> IpcError {
    IpcError::new(error.code, &error.message, error.retriable, false)
}

pub(super) fn parse_block_index_param(value: Option<&serde_json::Value>) -> Option<usize> {
    match value {
        Some(serde_json::Value::Number(n)) => n.as_u64().map(|v| v as usize),
        Some(serde_json::Value::String(s)) if s.trim().chars().all(|c| c.is_ascii_digit()) => {
            s.trim().parse::<usize>().ok()
        }
        _ => None,
    }
}

pub(super) fn is_active_editor_snapshot_source(source: &str) -> bool {
    matches!(source, "active_editor" | "active_editor_session")
}

/// WS-RESOURCE-WRITE-GUARD-01（F1，X-FORBID-006）：内容 Diff 不得创建资源。
/// propose_edit 目标路径不存在即拒绝（与意图分类无关）；新建文档唯一入口是
/// plan_workspace_operation（确认后建盘并投影初始内容 Diff）。
pub(super) fn guard_propose_edit_target(target_exists: bool) -> Result<(), IpcError> {
    if !target_exists {
        return Err(IpcError::new(
            "AG_TARGET_NOT_EXISTS",
            "propose_edit 目标路径不存在；新建文档请使用 plan_workspace_operation（确认后创建真实资源并投影初始内容 Diff），编辑请先确认目标路径。",
            true,
            false,
        ));
    }
    Ok(())
}

pub(super) fn resolve_agent_edit_operation(
    params: &serde_json::Value,
    target_exists: bool,
) -> Result<AgentEditOperation, IpcError> {
    // P1 预算修复（AG-R2-01）：单批 block_ops 操作数上限（实现期配置）。
    const MAX_BLOCK_OPS_PER_BATCH: usize = 24;
    let operation = params
        .get("operation")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let has_style_ops = style_ops_param(params).is_some();
    let has_style_payload = params.get("style_ops").is_some() || params.get("styleOps").is_some();
    let has_block_selector =
        params.get("block_id").is_some() || params.get("block_index").is_some();
    let has_new_text_payload = params.get("new_text").is_some();
    let has_new_text = params.get("new_text").and_then(|v| v.as_str()).is_some();
    let has_content_payload = params.get("content").is_some();
    let has_content = params.get("content").and_then(|v| v.as_str()).is_some();
    // table_ops 专属载荷字段：target/operations。base 是所有结构化 op（block_ops/style_ops/table_ops/
    // image_ops）的公共快照字段，由 has_structured_base 单独校验，不参与"混合载荷"判定
    // （修复：style_ops + base 曾被误判为混合 table_ops 载荷而拒绝）。
    let has_table_payload = params.get("target").is_some() || params.get("operations").is_some();
    let has_table_target = params.get("target").is_some();
    let has_table_ops = params
        .get("operations")
        .and_then(|v| v.as_array())
        .map(|v| !v.is_empty())
        .unwrap_or(false);
    let block_ops_len = params
        .get("operations")
        .and_then(|v| v.as_array())
        .map(|v| v.len())
        .unwrap_or(0);
    let has_structured_base = params.get("base").is_some();

    let op = match operation {
        Some("replace_block") => AgentEditOperation::ReplaceBlock,
        Some("block_ops") => AgentEditOperation::BlockOps,
        Some("replace_document") => AgentEditOperation::ReplaceDocument,
        Some("style_ops") => AgentEditOperation::StyleOps,
        Some("table_ops") => AgentEditOperation::TableOps,
        Some("image_ops") => AgentEditOperation::ImageOps,
        Some(other) => {
            return Err(IpcError::new(
                "AG_TOOL_ARG",
                &format!("propose_edit operation 不支持: {other}"),
                false,
                false,
            ));
        }
        None if has_block_selector && has_new_text => AgentEditOperation::ReplaceBlock,
        None if has_style_ops && !has_content => AgentEditOperation::StyleOps,
        // 生产不可达（WS-RESOURCE-WRITE-GUARD-01/F1）：ag_propose_edit 已在入口拒绝不存在路径；
        // 保留分支供既有定向测试与后续调用方防御，不承接"内容 Diff 创建资源"语义。
        None if has_content && !target_exists => AgentEditOperation::ReplaceDocument,
        None if has_content => {
            return Err(IpcError::new(
                "AG_PRECISE_TARGET_REQUIRED",
                "已有文本文件按块修改使用 replace_block；富文档必须使用 block_ops；只有用户明确要求整篇重写时才可使用 replace_document。",
                true,
                false,
            ));
        }
        None => {
            return Err(IpcError::new(
                "AG_TOOL_ARG",
                "propose_edit 缺 operation/content/new_text/style_ops/operations 参数",
                false,
                false,
            ));
        }
    };

    let mixed_payload = match op {
        AgentEditOperation::ReplaceBlock => {
            has_content_payload || has_style_payload || has_table_payload
        }
        AgentEditOperation::BlockOps => {
            has_block_selector
                || has_new_text_payload
                || has_content_payload
                || has_style_payload
                || has_table_target
        }
        AgentEditOperation::ReplaceDocument => {
            has_block_selector || has_new_text_payload || has_style_payload || has_table_payload
        }
        AgentEditOperation::StyleOps => {
            has_block_selector || has_new_text_payload || has_content_payload || has_table_payload
        }
        AgentEditOperation::TableOps => {
            has_block_selector || has_new_text_payload || has_content_payload || has_style_payload
        }
        AgentEditOperation::ImageOps => {
            has_block_selector
                || has_new_text_payload
                || has_content_payload
                || has_style_payload
                || has_table_target
        }
    };
    if mixed_payload {
        return Err(IpcError::new(
            "AG_TOOL_ARG",
            "propose_edit 不允许混合不同 operation 的载荷字段",
            false,
            false,
        ));
    }

    match op {
        AgentEditOperation::ReplaceBlock if !has_block_selector || !has_new_text => {
            Err(IpcError::new(
                "AG_TOOL_ARG",
                "operation=replace_block 必须提供 block_id 或 block_index，并提供 new_text",
                false,
                false,
            ))
        }
        AgentEditOperation::ReplaceDocument if !has_content => Err(IpcError::new(
            "AG_TOOL_ARG",
            "operation=replace_document 必须提供 content",
            false,
            false,
        )),
        AgentEditOperation::BlockOps if !has_structured_base || !has_table_ops => {
            Err(IpcError::new(
                "AG_TOOL_ARG",
                "operation=block_ops 必须提供 base 与非空 operations 数组",
                false,
                false,
            ))
        }
        // P1 预算修复（AG-R2-01）：单批 block_ops 操作数上限，防止模型一次提交超大载荷
        // 撑爆上下文；超限要求分批提交（1.0 分批原子提交，等价于多次 propose_edit）。
        AgentEditOperation::BlockOps if has_table_ops && block_ops_len > MAX_BLOCK_OPS_PER_BATCH => {
            Err(IpcError::new(
                "AG_TOOL_ARG",
                &format!(
                    "operation=block_ops 单批操作数 {block_ops_len} 超过上限 {MAX_BLOCK_OPS_PER_BATCH}；请分批提交，每批最多 {MAX_BLOCK_OPS_PER_BATCH} 个操作"
                ),
                false,
                false,
            ))
        }
        AgentEditOperation::StyleOps if !has_style_ops => Err(IpcError::new(
            "AG_TOOL_ARG",
            "operation=style_ops 必须提供 style_ops",
            false,
            false,
        )),
        AgentEditOperation::TableOps if !has_table_ops => Err(IpcError::new(
            "AG_TOOL_ARG",
            "operation=table_ops 必须提供非空 operations 数组",
            false,
            false,
        )),
        AgentEditOperation::ImageOps if !has_structured_base || !has_table_ops => {
            Err(IpcError::new(
                "AG_TOOL_ARG",
                "operation=image_ops 必须提供 base 与非空 operations 数组",
                false,
                false,
            ))
        }
        _ => {
            // P5 修复（AG-R2-01）：block_ops 锚点矛盾校验——同一锚点块被 insert_before
            // 又被 insert_after 引用，或同一块被多个锚点引用，属于自相矛盾，拒绝并回注。
            if op == AgentEditOperation::BlockOps {
                if let Some(anchor_conflict) =
                    find_block_ops_anchor_conflict(params.get("operations"))
                {
                    return Err(IpcError::new(
                        "AG_TOOL_ARG",
                        &format!("operation=block_ops 锚点矛盾：{anchor_conflict}；insert_before/insert_after 的锚点必须唯一且与 base 相邻关系一致"),
                        false,
                        false,
                    ));
                }
            }
            Ok(op)
        }
    }
}

/// P5 锚点校验（AG-R2-01）：返回首个自相矛盾的锚点描述；无冲突返回 None。
/// 检查 insert_before/insert_after 是否引用同一 blockId 作为多个操作的锚点。
pub(super) fn find_block_ops_anchor_conflict(
    operations: Option<&serde_json::Value>,
) -> Option<String> {
    let operations = operations?.as_array()?;
    let mut anchor_refs: Vec<(String, &str)> = Vec::new();
    for operation in operations {
        let op_type = operation
            .get("op")
            .and_then(|v| v.as_str())
            .unwrap_or("replace_node");
        let block_id = operation
            .get("blockId")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        match op_type {
            "insert_before" | "insert_after" if !block_id.is_empty() => {
                for (existing, existing_type) in &anchor_refs {
                    if existing == block_id {
                        return Some(format!(
                            "块 {block_id} 被 {existing_type} 与 {op_type} 同时作为锚点"
                        ));
                    }
                }
                anchor_refs.push((block_id.to_string(), op_type));
            }
            _ => {}
        }
    }
    None
}

pub(super) fn html_image_tags(html: &str) -> Vec<String> {
    let lower = html.to_ascii_lowercase();
    let mut tags = Vec::new();
    let mut cursor = 0usize;
    while let Some(relative) = lower[cursor..].find("<img") {
        let start = cursor + relative;
        let name_end = start + 4;
        if lower
            .as_bytes()
            .get(name_end)
            .is_some_and(|byte| !byte.is_ascii_whitespace() && *byte != b'>' && *byte != b'/')
        {
            cursor = name_end;
            continue;
        }
        let Some(end_relative) = lower[start..].find('>') else {
            break;
        };
        let end = start + end_relative + 1;
        // 标签名匹配大小写不敏感，但防绕过比较必须保留属性和值的原始字节；
        // 否则 case-only 的 src/hash/alt 变化会被误认为“图片未变”。
        tags.push(html[start..end].to_string());
        cursor = end;
    }
    tags
}

pub(super) fn html_opening_tag_count(html: &str, tag_name: &str) -> usize {
    let lower = html.to_ascii_lowercase();
    let needle = format!("<{}", tag_name.to_ascii_lowercase());
    let mut count = 0usize;
    let mut cursor = 0usize;
    while let Some(relative) = lower[cursor..].find(&needle) {
        let start = cursor + relative;
        let name_end = start + needle.len();
        if lower
            .as_bytes()
            .get(name_end)
            .is_some_and(|byte| !byte.is_ascii_whitespace() && *byte != b'>' && *byte != b'/')
        {
            cursor = name_end;
            continue;
        }
        count += 1;
        cursor = name_end;
    }
    count
}

pub(super) fn bdocx_structure_presence(
    snapshot: &ag::blocks::AgentEditableSnapshot,
    table_snapshot_count: usize,
    image_snapshot_count: usize,
) -> (bool, bool) {
    if snapshot.editable_kind != "bdocx" || snapshot.content_format != "html" {
        return (false, false);
    }
    (
        table_snapshot_count > 0 || html_opening_tag_count(&snapshot.content, "table") > 0,
        image_snapshot_count > 0 || !html_image_tags(&snapshot.content).is_empty(),
    )
}

pub(super) fn bdocx_structure_presence_from_context(
    read_ctx: &ag::blocks::AgentReadContext,
    rel_path: &str,
    snapshot: &ag::blocks::AgentEditableSnapshot,
) -> (bool, bool) {
    let (table_snapshot_count, image_snapshot_count) = read_ctx
        .open_snapshot(rel_path)
        .map(|open| (open.table_snapshots.len(), open.image_snapshots.len()))
        .unwrap_or_default();
    bdocx_structure_presence(snapshot, table_snapshot_count, image_snapshot_count)
}

pub(super) fn html_tag_attribute(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let needle = format!("{}=", name.to_ascii_lowercase());
    let mut cursor = 0usize;
    while let Some(relative) = lower[cursor..].find(&needle) {
        let start = cursor + relative;
        if start > 0 && !lower.as_bytes()[start - 1].is_ascii_whitespace() {
            cursor = start + needle.len();
            continue;
        }
        let quote_pos = start + needle.len();
        let quote = *tag.as_bytes().get(quote_pos)?;
        if quote != b'\'' && quote != b'"' {
            cursor = quote_pos + 1;
            continue;
        }
        let value_start = quote_pos + 1;
        let value_end = value_start + tag[value_start..].find(quote as char)?;
        return Some(tag[value_start..value_end].to_string());
    }
    None
}

pub(super) fn top_level_html_block_ids(html: &str) -> std::collections::BTreeSet<String> {
    let void_tags = [
        "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param",
        "source", "track", "wbr",
    ];
    let mut ids = std::collections::BTreeSet::new();
    let mut depth = 0usize;
    let mut cursor = 0usize;
    while let Some(relative) = html[cursor..].find('<') {
        let start = cursor + relative;
        let Some(end_relative) = html[start..].find('>') else {
            break;
        };
        let end = start + end_relative + 1;
        let tag = &html[start..end];
        let inner = tag[1..tag.len() - 1].trim();
        if inner.starts_with('!') || inner.starts_with('?') {
            cursor = end;
            continue;
        }
        if inner.starts_with('/') {
            depth = depth.saturating_sub(1);
            cursor = end;
            continue;
        }
        let name = inner
            .trim_start_matches('/')
            .split(|character: char| character.is_ascii_whitespace() || character == '/')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        if depth == 0 {
            if let Some(block_id) = html_tag_attribute(tag, "data-block-id") {
                ids.insert(block_id);
            }
        }
        if !inner.ends_with('/') && !void_tags.contains(&name.as_str()) {
            depth = depth.saturating_add(1);
        }
        cursor = end;
    }
    ids
}

pub(super) fn guard_bdocx_raw_image_edit(
    rel_path: &str,
    before: &str,
    after: &str,
    replace_document: bool,
) -> Result<(), IpcError> {
    if !is_docx_rel_path(rel_path) {
        return Ok(());
    }
    let before_images = html_image_tags(before);
    let after_images = html_image_tags(after);
    if (replace_document && (!before_images.is_empty() || !after_images.is_empty()))
        || (!replace_document && before_images != after_images)
    {
        return Err(IpcError::new(
            "AG_IMAGE_OPERATION_REQUIRED",
            if replace_document {
                "含图片的 BDocx 不允许通过 replace_document 原始 HTML 重写；图片增删必须使用 image_ops。"
            } else {
                "BDocx 块替换不得改变图片节点；请先调用 list_images，再使用 image_ops。"
            },
            false,
            false,
        ));
    }
    Ok(())
}

pub(super) fn guard_bdocx_replace_document(
    read_ctx: &ag::blocks::AgentReadContext,
    rel_path: &str,
    snapshot: Option<&ag::blocks::AgentEditableSnapshot>,
    content: &str,
) -> Result<(), IpcError> {
    let Some(snapshot) = snapshot else {
        return Ok(());
    };
    if !is_docx_rel_path(rel_path)
        || snapshot.editable_kind != "bdocx"
        || snapshot.content_format != "html"
    {
        return Ok(());
    }

    let (has_tables, has_images) =
        bdocx_structure_presence_from_context(read_ctx, rel_path, snapshot);
    let introduces_tables = html_opening_tag_count(content, "table") > 0;
    let introduces_images = !html_image_tags(content).is_empty();
    if !(has_tables || has_images || introduces_tables || introduces_images) {
        return Ok(());
    }

    let mut protected = Vec::new();
    if has_tables || introduces_tables {
        protected.push("表格");
    }
    if has_images || introduces_images {
        protected.push("图片");
    }
    Err(IpcError::new(
        "AG_BDOCX_REPLACE_DOCUMENT_REQUIRES_STRUCTURED_OPS",
        &format!(
            "当前或拟议的 BDocx 含{}，不允许通过 replace_document 整篇重写；既有文字内容请使用 block_ops，表格请使用 table_ops，图片请使用 image_ops。",
            protected.join("和")
        ),
        false,
        false,
    ))
}
