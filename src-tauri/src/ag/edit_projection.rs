//! 已校验 AgentEditOperation 到内容与 DiffCard 展示投影的构造。

use super::edit_support::{
    guard_bdocx_raw_image_edit, guard_bdocx_replace_document, parse_block_index_param,
};
use super::edit_types::{AgentEditOperation, EditProjectionDraft};
use crate::ag;
use crate::ipc::IpcError;

#[allow(clippy::too_many_arguments)]
/**
 * @GOV
 * codes:    BR-DE-DATA-002-DATA-AG-AG-TOOL-060, BR-DE-DATA-001-DATA-AG-AG-TOOL-060
 * type:     DATA
 * chain:    AG-TOOL, DE-DIFF
 * rules:    BR-DE-DATA-002, BR-DE-DATA-001
 * boundary: in=ProposeEditParamsAndDocumentSnapshot | out=EditProjectionDraftWithBeforeAfterPayload | does_not=generatePendingDiffPersistProposalOrWriteFile | consumer=BR-DE-DATA-002,BR-DE-DATA-001
 */
pub(super) fn build_edit_projection(
    params: &serde_json::Value,
    edit_operation: AgentEditOperation,
    snapshot: Option<&ag::blocks::AgentEditableSnapshot>,
    base_content_for_diff: &str,
    base_is_html: bool,
    rel_path: &str,
    target_exists: bool,
    read_ctx: &ag::blocks::AgentReadContext,
    style_ops_json: Option<&serde_json::Value>,
    block_ops_json: Option<&serde_json::Value>,
    table_ops_json: Option<&serde_json::Value>,
    image_ops_json: Option<&serde_json::Value>,
) -> Result<EditProjectionDraft, IpcError> {
    // replace_block：基准用当前编辑器快照或 BDocx 转换 HTML 切块，替目标块并重建文档；
    // replace_document：仅显式整篇重写可用；style_ops：文本基准保持不变，样式语义由载荷进入 DE 裁决。
    let projection_block_id: Option<String>;
    let projection_block_index: Option<usize>;
    let projection_before: String;
    let projection_after: String;
    let projection_location: String;

    let after_content = match edit_operation {
        AgentEditOperation::ReplaceBlock => {
            let new_text = params
                .get("new_text")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    IpcError::new(
                        "AG_TOOL_ARG",
                        "propose_edit 块模式缺 new_text 参数",
                        false,
                        false,
                    )
                })?;
            let snapshot = snapshot.as_ref().ok_or_else(|| {
                IpcError::new(
                    "AG_EDIT_BASE_REQUIRED",
                    "块级修改必须基于既有文档快照",
                    false,
                    false,
                )
            })?;
            let raw_block_id = params
                .get("block_id")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty());
            let numeric_block_id_as_index = raw_block_id.and_then(|s| s.parse::<usize>().ok());
            let selected_block = if let Some(block_id) =
                raw_block_id.filter(|_| numeric_block_id_as_index.is_none())
            {
                snapshot
                    .blocks
                    .iter()
                    .find(|b| b.block_id.as_deref() == Some(block_id))
                    .ok_or_else(|| {
                        IpcError::new(
                            "AG_ANCHOR_LOST",
                            &format!("锚失效：未找到 block_id={block_id} 的块"),
                            true,
                            false,
                        )
                    })?
            } else {
                let bi = numeric_block_id_as_index
                    .or_else(|| parse_block_index_param(params.get("block_index")))
                    .ok_or_else(|| {
                        IpcError::new(
                            "AG_TOOL_ARG",
                            "propose_edit 缺 block_id/block_index",
                            false,
                            false,
                        )
                    })?;
                snapshot.blocks.get(bi).ok_or_else(|| {
                    IpcError::new(
                        "AG_TOOL_ARG",
                        &format!(
                            "block_index {bi} 越界（当前共 {} 块）",
                            snapshot.blocks.len()
                        ),
                        false,
                        false,
                    )
                })?
            };
            projection_block_id = selected_block.block_id.clone();
            projection_block_index = Some(selected_block.block_index);
            projection_before = selected_block.text.clone();
            projection_after = new_text.to_string();
            projection_location = format!("Block {}", selected_block.block_index);
            let replaced = if let Some(block_id) = selected_block.block_id.as_deref() {
                // 真实 blockId 锚（HTML data-block-id，PI-AG-08G）：按 id 定位；锚失效报错不误改。
                crate::ag::blocks::replace_block_by_id(base_content_for_diff, block_id, new_text)
                    .map_err(|m| IpcError::new("AG_ANCHOR_LOST", &m, true, false))?
            } else {
                // 位置块号（text 或回退）。
                crate::ag::blocks::replace_block(
                    base_content_for_diff,
                    base_is_html,
                    selected_block.block_index,
                    new_text,
                )
                .map_err(|m| IpcError::new("AG_TOOL_ARG", &m, false, false))?
            };
            guard_bdocx_raw_image_edit(&rel_path, base_content_for_diff, &replaced, false)?;
            replaced
        }
        AgentEditOperation::BlockOps => {
            let payload = block_ops_json
                .as_ref()
                .expect("block_ops validated before match");
            let operations = payload["operations"]
                .as_array()
                .expect("validated block operations");
            let current = snapshot
                .as_ref()
                .expect("block_ops requires existing snapshot");
            // append 不带锚点，不能被要求解析"首个锚"：只有真正携带 blockId 的操作才参与
            // 锚定解析。整批 append 时用文末最后一块做投影定位（仅展示用，不参与校验）。
            let first_block = operations
                .iter()
                .find_map(|operation| {
                    operation
                        .get("blockId")
                        .and_then(serde_json::Value::as_str)
                        .filter(|id| !id.trim().is_empty())
                })
                .map(|id| {
                    current
                        .blocks
                        .iter()
                        .find(|block| block.block_id.as_deref() == Some(id))
                        .ok_or_else(|| {
                            IpcError::new("AG_ANCHOR_LOST", "结构化块首个锚已失效", true, false)
                        })
                })
                .transpose()?
                .or_else(|| current.blocks.last());
            projection_block_id = first_block.and_then(|block| block.block_id.clone());
            projection_block_index = first_block.map(|block| block.block_index);
            projection_before = operations
                .iter()
                .map(|operation| {
                    let block_id = operation["blockId"].as_str().unwrap_or("");
                    let text = ag::block_ops::node_text(&operation["beforeNode"]);
                    format!("{block_id}: {text}")
                })
                .collect::<Vec<_>>()
                .join("\n");
            projection_after = operations
                .iter()
                .enumerate()
                .map(|(idx, operation)| {
                    let op = operation["op"].as_str().unwrap_or("");
                    // AG-STRATEGY-04：insert/append 新块不再复用锚点 blockId 标注——新块取
                    // afterNode.attrs.blockId（后端已兜底生成 new_block_{uuid}）或 fallback new_N，
                    // 避免标题块与正文块同 id 导致 DiffCard 渲染/应用错乱；replace/delete 仍用目标块 id。
                    let block_id = match op {
                        "insert_before" | "insert_after" | "append" => operation["afterNode"]
                            ["attrs"]["blockId"]
                            .as_str()
                            .filter(|s| !s.is_empty())
                            .map(str::to_string)
                            .unwrap_or_else(|| format!("new_{}", idx + 1)),
                        _ => operation["blockId"].as_str().unwrap_or("").to_string(),
                    };
                    let text = ag::block_ops::node_text(&operation["afterNode"]);
                    format!("{block_id}: {text}")
                })
                .collect::<Vec<_>>()
                .join("\n");
            projection_location = if operations.len() == 1 {
                match first_block {
                    Some(block) => format!("Block {}", block.block_index),
                    None => "文末追加".to_string(),
                }
            } else {
                format!("{} 个独立内容块", operations.len())
            };
            base_content_for_diff.to_string()
        }
        AgentEditOperation::ReplaceDocument => {
            let content = params
                .get("content")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    IpcError::new(
                        "AG_TOOL_ARG",
                        "operation=replace_document 必须提供 content",
                        false,
                        false,
                    )
                })?;
            guard_bdocx_replace_document(read_ctx, rel_path, snapshot, content)?;
            if target_exists
                && !crate::ag::blocks::rewrite_preserves_document_identity(
                    base_content_for_diff,
                    content,
                )
            {
                return Err(IpcError::new(
                    "AG_REPLACE_DOCUMENT_IDENTITY_DRIFT",
                    "整篇替换内容与原文主题/实体重合过低，疑似错文档或历史上下文污染，已拒绝生成修改建议。请先重新读取当前文档并优先按块修改；若用户明确要求改成全新主题，需要重新确认后再执行。",
                    true,
                    false,
                ));
            }
            projection_block_id = None;
            projection_block_index = None;
            projection_before = base_content_for_diff.to_string();
            projection_after = content.to_string();
            projection_location = "整篇".to_string();
            content.to_string()
        }
        AgentEditOperation::StyleOps => {
            let snapshot = snapshot.as_ref().ok_or_else(|| {
                IpcError::new(
                    "AG_EDIT_BASE_REQUIRED",
                    "样式修改必须基于既有文档快照",
                    false,
                    false,
                )
            })?;
            let first_block_id =
                style_ops_json
                    .as_ref()
                    .and_then(|v| v.as_array())
                    .and_then(|items| {
                        items
                            .iter()
                            .find_map(|item| item.get("blockId").and_then(|v| v.as_str()))
                    });
            let selected_block = first_block_id.and_then(|id| {
                snapshot
                    .blocks
                    .iter()
                    .find(|b| b.block_id.as_deref() == Some(id))
            });
            if let Some(block) = selected_block {
                projection_block_id = block.block_id.clone();
                projection_block_index = Some(block.block_index);
                projection_before = block.text.clone();
            } else {
                projection_block_id = None;
                projection_block_index = None;
                projection_before = base_content_for_diff.to_string();
            }
            projection_after = "（样式修改）".to_string();
            projection_location = "样式修改".to_string();
            base_content_for_diff.to_string()
        }
        AgentEditOperation::TableOps => {
            let payload = table_ops_json
                .as_ref()
                .expect("table_ops validated before match");
            projection_block_id = None;
            projection_block_index = None;
            projection_before = format!(
                "表格 {} 当前结构",
                payload["tableId"].as_str().unwrap_or("")
            );
            projection_after = format!(
                "表格结构修改（{} 项）",
                payload["operations"]
                    .as_array()
                    .map(|v| v.len())
                    .unwrap_or(0)
            );
            projection_location = "表格结构".to_string();
            base_content_for_diff.to_string()
        }
        AgentEditOperation::ImageOps => {
            let payload = image_ops_json
                .as_ref()
                .expect("image_ops validated before match");
            let operations = payload["operations"]
                .as_array()
                .expect("validated image operations");
            let inserted = operations
                .iter()
                .filter(|operation| operation["op"] == "insert_image")
                .count();
            let deleted = operations
                .iter()
                .filter(|operation| operation["op"] == "delete_image")
                .count();
            let resized = operations
                .iter()
                .filter(|operation| operation["op"] == "resize_image")
                .count();
            let moved = operations
                .iter()
                .filter(|operation| operation["op"] == "move_image")
                .count();
            let aligned = operations
                .iter()
                .filter(|operation| operation["op"] == "set_image_layout")
                .count();
            projection_block_id = operations
                .iter()
                .find_map(|operation| {
                    operation
                        .get("anchorBlockId")
                        .and_then(|value| value.as_str())
                        .map(str::to_string)
                })
                .or_else(|| {
                    let images = read_ctx.image_snapshots(&rel_path).ok()?;
                    operations.iter().find_map(|operation| {
                        let image_id = operation.get("imageId")?.as_str()?;
                        let fingerprint = operation.get("fingerprint")?.as_str()?;
                        images.iter().find_map(|image| {
                            (image.get("imageId").and_then(|value| value.as_str())
                                == Some(image_id)
                                && image.get("fingerprint").and_then(|value| value.as_str())
                                    == Some(fingerprint))
                            .then(|| {
                                image
                                    .get("blockId")
                                    .and_then(|value| value.as_str())
                                    .map(str::to_string)
                            })
                            .flatten()
                        })
                    })
                });
            projection_block_index = projection_block_id.as_ref().and_then(|block_id| {
                snapshot
                    .as_ref()
                    .and_then(|current| {
                        current
                            .blocks
                            .iter()
                            .find(|block| block.block_id.as_ref() == Some(block_id))
                    })
                    .map(|block| block.block_index)
            });
            projection_before =
                format!("当前图片 {} 张", read_ctx.image_snapshots(&rel_path)?.len());
            projection_after = format!(
                "图片操作：插入 {inserted}，删除 {deleted}，缩放 {resized}，移动 {moved}，对齐 {aligned}"
            );
            projection_location = "图片结构".to_string();
            base_content_for_diff.to_string()
        }
    };

    Ok(EditProjectionDraft {
        after_content,
        block_id: projection_block_id,
        block_index: projection_block_index,
        before_text: projection_before,
        after_text: projection_after,
        location_label: projection_location,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ag::blocks::{AgentEditableBlock, AgentEditableSnapshot};

    fn snapshot() -> AgentEditableSnapshot {
        AgentEditableSnapshot {
            rel_path: "a.html".into(),
            editable_kind: "bdocx".into(),
            source: "active_editor".into(),
            content_format: "html".into(),
            content: "<p data-block-id=\"last\">末块</p>".into(),
            baseline: Some(1),
            session_version: Some(1),
            content_hash: Some("h".into()),
            blocks: vec![AgentEditableBlock {
                block_id: Some("last".into()),
                block_index: 0,
                node_type: Some("paragraph".into()),
                block_type: Some("paragraph".into()),
                role: "content".into(),
                parent_block_id: None,
                depth: 0,
                ordinal: 0,
                text: "末块".into(),
                marks: vec![],
                block_attrs: serde_json::json!({}),
                node_json: serde_json::json!({}),
                fingerprint: Some("fp".into()),
                capabilities: vec!["replace_node".into()],
            }],
        }
    }

    // append 没有锚点，不能要求解析"首个锚"：整批 append 必须能生成投影。
    #[test]
    fn append_only_block_ops_does_not_require_anchor() {
        let payload = serde_json::json!({
            "base": {"sessionVersion": 1, "contentHash": "h"},
            "operations": [
                {"op": "append", "blockId": "", "afterNode": {"type":"paragraph","content":[{"type":"text","text":"新增一"}]}},
                {"op": "append", "blockId": "", "afterNode": {"type":"paragraph","content":[{"type":"text","text":"新增二"}]}}
            ]
        });
        let snap = snapshot();
        let read_ctx =
            ag::blocks::AgentReadContext::new(std::path::Path::new("."), vec![]).unwrap();
        let draft = build_edit_projection(
            &serde_json::json!({"path":"a.html","operation":"block_ops"}),
            AgentEditOperation::BlockOps,
            Some(&snap),
            &snap.content,
            true,
            "a.html",
            true,
            &read_ctx,
            None,
            Some(&payload),
            None,
            None,
        )
        .unwrap();
        assert_eq!(draft.block_id.as_deref(), Some("last"));
        assert_eq!(draft.block_index, Some(0));
        assert!(draft.after_text.contains("新增二"));
    }

    // 带 blockId 但锚已失效时仍必须报 AG_ANCHOR_LOST，不能被 append 的宽松逻辑放行。
    #[test]
    fn stale_insert_anchor_still_fails_closed() {
        let payload = serde_json::json!({
            "base": {"sessionVersion": 1, "contentHash": "h"},
            "operations": [
                {"op": "insert_after", "blockId": "gone", "afterNode": {"type":"paragraph","content":[{"type":"text","text":"新"}]}}
            ]
        });
        let snap = snapshot();
        let read_ctx =
            ag::blocks::AgentReadContext::new(std::path::Path::new("."), vec![]).unwrap();
        let error = build_edit_projection(
            &serde_json::json!({"path":"a.html","operation":"block_ops"}),
            AgentEditOperation::BlockOps,
            Some(&snap),
            &snap.content,
            true,
            "a.html",
            true,
            &read_ctx,
            None,
            Some(&payload),
            None,
            None,
        )
        .unwrap_err();
        assert_eq!(error.code, "AG_ANCHOR_LOST");
    }
}
