//! ImageOperation 基线、能力、锚点和项目图片物化校验。

use super::edit_support::{capability_error_to_ipc, top_level_html_block_ids};
use crate::ipc::IpcError;
use crate::{ag, doc, workspace};

/**
 * @GOV
 * codes:    BR-SYS-DATA-006-GUARD-AG-AG-TOOL-004, BR-ED-DATA-001-GUARD-AG-AG-TOOL-004, BR-ED-DATA-003-GUARD-AG-AG-TOOL-004
 * type:     GUARD
 * chain:    AG-TOOL
 * rules:    BR-SYS-DATA-006, BR-ED-DATA-001, BR-ED-DATA-003
 * term_ref: TERM-SYS-002, TERM-ED-008
 * boundary: in=UntrustedImageOperationAndActiveBDocxSnapshot | out=ValidatedMaterializedImageOperationPayloadOrPreDiffRejection | consumer=BR-SYS-DATA-006,BR-ED-DATA-001,BR-ED-DATA-003
 */
pub(super) fn image_ops_payload(
    params: &serde_json::Value,
    snapshot: &ag::blocks::AgentEditableSnapshot,
    images: &[serde_json::Value],
    workspace_root: &std::path::Path,
) -> Result<serde_json::Value, IpcError> {
    let base = params
        .get("base")
        .and_then(|value| value.as_object())
        .ok_or_else(|| {
            IpcError::new(
                "AG_TOOL_ARG",
                "image_ops 缺 base（list_images 的 sessionVersion/contentHash）",
                false,
                false,
            )
        })?;
    let base_version = base
        .get("sessionVersion")
        .or_else(|| base.get("session_version"))
        .and_then(|value| value.as_u64());
    let base_hash = base
        .get("contentHash")
        .or_else(|| base.get("content_hash"))
        .and_then(|value| value.as_str());
    if base_version != snapshot.session_version || base_hash != snapshot.content_hash.as_deref() {
        return Err(IpcError::new(
            "AG_IMAGE_STALE_SNAPSHOT",
            "图片快照已过期；请重新调用 list_images 后再提出修改建议。",
            true,
            false,
        ));
    }
    let baseline = snapshot.baseline.ok_or_else(|| {
        IpcError::new(
            "AG_IMAGE_BASELINE_REQUIRED",
            "当前 BDocx 图片操作缺少媒体 baseline；请重新打开文档后重试。",
            true,
            false,
        )
    })?;
    let operations = params
        .get("operations")
        .and_then(|value| value.as_array())
        .filter(|operations| !operations.is_empty())
        .ok_or_else(|| IpcError::new("AG_TOOL_ARG", "image_ops 缺非空 operations", false, false))?;
    ag::bdocx_capabilities::validate_image_operation_shapes(operations)
        .map_err(capability_error_to_ipc)?;

    let top_level_block_ids = top_level_html_block_ids(&snapshot.content);
    let catalog = workspace::listing::load_project_image_catalog(workspace_root)?;
    let catalog_candidates = catalog
        .candidates
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    let mut image_targets =
        std::collections::BTreeMap::<String, std::collections::BTreeSet<String>>::new();
    let mut insert_anchors = std::collections::BTreeSet::new();
    let mut removed_block_anchors = std::collections::BTreeSet::new();
    for operation in operations {
        match operation.get("op").and_then(|value| value.as_str()) {
            Some("insert_image") => {
                let anchor_block_id = operation["anchorBlockId"].as_str().unwrap_or_default();
                if !top_level_block_ids.contains(anchor_block_id) {
                    return Err(IpcError::new(
                        "AG_IMAGE_ANCHOR_LOST",
                        &format!(
                            "未找到 anchorBlockId={anchor_block_id} 的当前 Block；请重新调用 list_blocks/list_images。"
                        ),
                        true,
                        false,
                    ));
                }
                insert_anchors.insert(anchor_block_id.to_string());
                let source_path = operation["sourcePath"].as_str().unwrap_or_default();
                let source_hash = operation["sourceHash"].as_str().unwrap_or_default();
                let asset_id = operation["assetId"].as_str().unwrap_or_default();
                if !catalog_candidates.contains(source_path) {
                    return Err(IpcError::new(
                        "AG_IMAGE_SOURCE_UNAVAILABLE",
                        &format!(
                            "项目图片不在当前配置 imageRoots 中：{source_path}；请重新调用 list_images。"
                        ),
                        true,
                        false,
                    ));
                }
                let asset = doc::image_media::inspect_project_image(workspace_root, source_path)?;
                if asset.content_hash != source_hash || asset.asset_id != asset_id {
                    return Err(IpcError::new(
                        "AG_IMAGE_SOURCE_CHANGED",
                        &format!(
                            "项目图片身份或内容已变化：{source_path}；请重新调用 list_images 获取最新 assetId/sourceHash。"
                        ),
                        true,
                        false,
                    ));
                }
            }
            Some(op @ ("delete_image" | "resize_image" | "move_image" | "set_image_layout")) => {
                let image_id = operation["imageId"].as_str().unwrap_or_default();
                let fingerprint = operation["fingerprint"].as_str().unwrap_or_default();
                let planned = image_targets.entry(image_id.to_string()).or_default();
                if planned.contains(op) {
                    return Err(IpcError::new(
                        "AG_IMAGE_OPERATION_INVALID",
                        &format!("同一图片 {image_id} 在单批中不能重复执行 {op}"),
                        false,
                        false,
                    ));
                }
                if planned.contains("delete_image") || (op == "delete_image" && !planned.is_empty())
                {
                    return Err(IpcError::new(
                        "AG_IMAGE_OPERATION_INVALID",
                        &format!(
                            "同一图片 {image_id} 的 delete_image 必须独占，不能与其他 ImageOperation 组合"
                        ),
                        false,
                        false,
                    ));
                }
                planned.insert(op.to_string());
                let matched = images.iter().find(|image| {
                    image.get("imageId").and_then(|value| value.as_str()) == Some(image_id)
                        && image.get("fingerprint").and_then(|value| value.as_str())
                            == Some(fingerprint)
                });
                let Some(matched) = matched else {
                    return Err(IpcError::new(
                        "AG_IMAGE_ANCHOR_LOST",
                        &format!(
                            "未找到 imageId={image_id} 且 fingerprint 匹配的当前图片；请重新调用 list_images。"
                        ),
                        true,
                        false,
                    ));
                };
                let capability_available = matched
                    .get("capabilities")
                    .and_then(|value| value.as_array())
                    .is_some_and(|capabilities| {
                        capabilities.iter().any(|value| value.as_str() == Some(op))
                    });
                if !capability_available {
                    return Err(IpcError::new(
                        "AG_IMAGE_OPERATION_UNAVAILABLE",
                        &format!("当前图片 {image_id} 不允许操作 {op}；请重新调用 list_images。"),
                        true,
                        false,
                    ));
                }
                if op == "resize_image" {
                    let width = operation["width"].as_u64().unwrap_or_default();
                    let bounds = matched
                        .get("resizeBounds")
                        .and_then(|value| value.as_object());
                    let min = bounds
                        .and_then(|value| value.get("min"))
                        .and_then(|value| value.as_u64())
                        .unwrap_or(1);
                    let max = bounds
                        .and_then(|value| value.get("max"))
                        .and_then(|value| value.as_u64())
                        .unwrap_or(0);
                    if width < min || width > max {
                        return Err(IpcError::new(
                            "AG_IMAGE_OPERATION_UNAVAILABLE",
                            &format!("图片宽度 {width} 超出当前 resizeBounds {min}..={max}"),
                            false,
                            false,
                        ));
                    }
                }
                if op == "move_image" {
                    let direction = operation["direction"].as_str().unwrap_or_default();
                    let allowed = matched
                        .get("moveDirections")
                        .and_then(|value| value.as_array())
                        .is_some_and(|directions| {
                            directions
                                .iter()
                                .any(|value| value.as_str() == Some(direction))
                        });
                    if !allowed {
                        return Err(IpcError::new(
                            "AG_IMAGE_OPERATION_UNAVAILABLE",
                            &format!("当前图片 {image_id} 不能向 {direction} 移动"),
                            true,
                            false,
                        ));
                    }
                }
                if op == "set_image_layout"
                    && matched.get("standalone").and_then(|value| value.as_bool()) != Some(true)
                {
                    return Err(IpcError::new(
                        "AG_IMAGE_OPERATION_UNAVAILABLE",
                        "只有独立顶层图片段落可设置 left/center/right 对齐",
                        false,
                        false,
                    ));
                }
                if op == "delete_image"
                    && top_level_block_ids.len() > 1
                    && matched.get("standalone").and_then(|value| value.as_bool()) == Some(true)
                {
                    if let Some(block_id) = matched.get("blockId").and_then(|value| value.as_str())
                    {
                        removed_block_anchors.insert(block_id.to_string());
                    }
                }
            }
            _ => unreachable!("static ImageOperation validation rejects unknown operations"),
        }
    }
    if let Some(anchor) = insert_anchors.intersection(&removed_block_anchors).next() {
        return Err(IpcError::new(
            "AG_IMAGE_OPERATION_INVALID",
            &format!(
                "同一批 ImageOperation 不能一边以 Block {anchor} 为插入锚，一边删除会移除该 Block 的独立图片。"
            ),
            false,
            false,
        ));
    }

    let mut materialized_operations = Vec::with_capacity(operations.len());
    for operation in operations {
        let mut materialized = operation.clone();
        if operation.get("op").and_then(|value| value.as_str()) == Some("insert_image") {
            let source_path = operation["sourcePath"].as_str().unwrap_or_default();
            let source_hash = operation["sourceHash"].as_str().unwrap_or_default();
            let image = doc::image_media::materialize_project_image(
                workspace_root,
                source_path,
                source_hash,
                baseline,
            )?;
            materialized
                .as_object_mut()
                .expect("validated image operation object")
                .insert(
                    "mediaRef".to_string(),
                    serde_json::Value::String(image.media_ref),
                );
        }
        materialized_operations.push(materialized);
    }
    Ok(serde_json::json!({
        "base": {
            "sessionVersion": base_version,
            "contentHash": base_hash,
        },
        "operations": materialized_operations,
    }))
}
