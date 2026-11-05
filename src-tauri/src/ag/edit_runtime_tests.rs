//! Agent 编辑解析、TableOperation 与 ImageOperation 的回归测试。
#![cfg(test)]

use super::edit_image_runtime::*;
use super::edit_support::*;
use super::edit_table_runtime::*;
use super::edit_types::*;
use super::task_runtime::edit_intent_has_terminal_tool_result;
use crate::{ag, doc};
use serde_json::json;

fn table_editable_snapshot() -> ag::blocks::AgentEditableSnapshot {
    ag::blocks::AgentEditableSnapshot {
        rel_path: "notes/a.docx".to_string(),
        editable_kind: "bdocx".to_string(),
        source: "active_editor".to_string(),
        content_format: "html".to_string(),
        content: "<table></table>".to_string(),
        baseline: None,
        session_version: Some(7),
        content_hash: Some("h7".to_string()),
        blocks: Vec::new(),
    }
}

fn table_snapshot(content_kind: &str) -> serde_json::Value {
    json!({
        "tableId": "table_1",
        "fingerprint": "fp_1",
        "rows": 2,
        "columns": 2,
        "moveDirections": ["up", "down"],
        "capabilities": ["insert_row", "delete_row", "insert_column", "delete_column", "set_header_row", "merge_cells", "split_cell", "set_cell_text", "set_cell_background", "move_table"],
        "cells": [{ "row": 0, "column": 0, "rowspan": 1, "colspan": 1, "backgroundColor": null, "contentKind": content_kind }]
    })
}

fn image_editable_snapshot(baseline: u64) -> ag::blocks::AgentEditableSnapshot {
    ag::blocks::AgentEditableSnapshot {
        rel_path: "notes/a.docx".to_string(),
        editable_kind: "bdocx".to_string(),
        source: "active_editor".to_string(),
        content_format: "html".to_string(),
        content: r#"<p data-block-id="b1">正文</p><p data-block-id="img1"><img src="media/image1.png"></p>"#.to_string(),
        baseline: Some(baseline),
        session_version: Some(8),
        content_hash: Some("h8".to_string()),
        blocks: vec![
            ag::blocks::AgentEditableBlock {
                block_id: Some("b1".to_string()),
                block_index: 0,
                block_type: Some("p".to_string()),
                text: "正文".to_string(),
                marks: Vec::new(),
                block_attrs: json!({}),
                ..Default::default()
            },
            ag::blocks::AgentEditableBlock {
                block_id: Some("img1".to_string()),
                block_index: 1,
                block_type: Some("p".to_string()),
                text: "\u{fffc}".to_string(),
                marks: Vec::new(),
                block_attrs: json!({}),
                ..Default::default()
            },
        ],
    }
}

fn bdocx_route_context(
    table_snapshots: Vec<serde_json::Value>,
    image_snapshots: Vec<serde_json::Value>,
) -> ag::blocks::AgentReadContext {
    ag::blocks::AgentReadContext::new(
        &std::env::temp_dir(),
        vec![ag::blocks::OpenDocSnapshot {
            rel_path: "notes/a.docx".to_string(),
            content: r#"<p data-block-id="b1">正文</p>"#.to_string(),
            baseline: None,
            edit_format: "html".to_string(),
            content_format: "html".to_string(),
            source: "active_editor".to_string(),
            dirty: false,
            session_version: 1,
            content_hash: "h1".to_string(),
            captured_at: 1,
            block_snapshots: Vec::new(),
            table_snapshots,
            image_snapshots,
        }],
    )
    .unwrap()
}

#[test]
fn structured_edit_blocked_satisfies_edit_tool_terminal_without_a_diff() {
    assert!(edit_intent_has_terminal_tool_result(true, false));
    assert!(edit_intent_has_terminal_tool_result(false, true));
    assert!(!edit_intent_has_terminal_tool_result(false, false));
}

// covers: BR-AG-DATA-003
// 当前编辑器实时快照只允许作为当前目标文档的 diff 基准；非当前目标必须另读自身内容，避免锚错位。
#[test]
fn live_editor_base_only_applies_to_current_target() {
    assert_eq!(
        live_agent_edit_base("notes/a.docx", Some("notes/a.docx"), Some("<p>live</p>")),
        Some("<p>live</p>")
    );
    assert_eq!(
        live_agent_edit_base(
            "notes/other.docx",
            Some("notes/a.docx"),
            Some("<p>live</p>")
        ),
        None
    );
    assert_eq!(
        live_agent_edit_base("notes/a.docx", None, Some("<p>live</p>")),
        None
    );
}

#[test]
fn existing_document_rejects_legacy_bare_content() {
    let err = resolve_agent_edit_operation(
        &json!({ "path": "notes/a.docx", "content": "<p>全文</p>" }),
        true,
    )
    .unwrap_err();

    assert_eq!(err.code, "AG_PRECISE_TARGET_REQUIRED");
    assert!(err.message.contains("replace_block"));
    assert!(err.message.contains("replace_document"));
}

#[test]
fn new_document_allows_legacy_content_creation() {
    let op = resolve_agent_edit_operation(
        &json!({ "path": "notes/new.md", "content": "新文档" }),
        false,
    )
    .unwrap();

    assert_eq!(op, AgentEditOperation::ReplaceDocument);
}

#[test]
fn explicit_replace_document_requires_content() {
    let op = resolve_agent_edit_operation(
        &json!({ "path": "notes/a.docx", "operation": "replace_document", "content": "<p>全文</p>" }),
        true,
    )
    .unwrap();
    assert_eq!(op, AgentEditOperation::ReplaceDocument);

    let err = resolve_agent_edit_operation(
        &json!({ "path": "notes/a.docx", "operation": "replace_document" }),
        true,
    )
    .unwrap_err();
    assert_eq!(err.code, "AG_TOOL_ARG");

    let err = resolve_agent_edit_operation(
        &json!({
            "path": "notes/a.docx",
            "operation": "block_ops",
            "operations": [{
                "op": "replace_node",
                "blockId": "p-1",
                "fingerprint": "fp-1",
                "expectedNodeType": "paragraph",
                "afterNode": { "type": "paragraph", "attrs": { "blockId": "p-1" } }
            }]
        }),
        true,
    )
    .unwrap_err();
    assert_eq!(err.code, "AG_TOOL_ARG");
}

#[test]
fn explicit_replace_block_requires_selector_and_new_text() {
    let op = resolve_agent_edit_operation(
        &json!({ "path": "notes/a.docx", "operation": "replace_block", "block_id": "b1", "new_text": "新块" }),
        true,
    )
    .unwrap();
    assert_eq!(op, AgentEditOperation::ReplaceBlock);

    let err = resolve_agent_edit_operation(
        &json!({ "path": "notes/a.docx", "operation": "replace_block", "new_text": "新块" }),
        true,
    )
    .unwrap_err();
    assert_eq!(err.code, "AG_TOOL_ARG");
}

#[test]
fn explicit_block_ops_requires_base_and_non_empty_operations() {
    let op = resolve_agent_edit_operation(
        &json!({
            "path": "notes/a.docx",
            "operation": "block_ops",
            "base": { "sessionVersion": 1, "contentHash": "h1" },
            "operations": [{
                "op": "replace_node",
                "blockId": "p-1",
                "fingerprint": "fp-1",
                "expectedNodeType": "paragraph",
                "afterNode": { "type": "paragraph", "attrs": { "blockId": "p-1" } }
            }]
        }),
        true,
    )
    .unwrap();
    assert_eq!(op, AgentEditOperation::BlockOps);

    let err = resolve_agent_edit_operation(
        &json!({
            "path": "notes/a.docx",
            "operation": "block_ops",
            "base": { "sessionVersion": 1, "contentHash": "h1" },
            "operations": []
        }),
        true,
    )
    .unwrap_err();
    assert_eq!(err.code, "AG_TOOL_ARG");
}

// covers: BR-AG-STATE-002
// covers: BR-AG-DATA-010
// P5（AG-R2-01）：同一锚点只允许一个插入方向；同向多插入是合法能力，不得误判为锚点矛盾。
#[test]
fn block_ops_rejects_conflicting_anchor_usage() {
    let ok = resolve_agent_edit_operation(
        &json!({
            "path": "notes/a.docx",
            "operation": "block_ops",
            "base": { "sessionVersion": 1, "contentHash": "h1" },
            "operations": [
                { "op": "insert_before", "blockId": "anchor-a", "afterNode": { "type": "paragraph" } },
                { "op": "insert_after", "blockId": "anchor-b", "afterNode": { "type": "paragraph" } }
            ]
        }),
        true,
    )
    .unwrap();
    assert_eq!(ok, AgentEditOperation::BlockOps);

    // 同一锚点的同向连续插入必须放行（顺序由 operations 数组保证，分组由 split 合并）。
    let same_direction = resolve_agent_edit_operation(
        &json!({
            "path": "notes/a.docx",
            "operation": "block_ops",
            "base": { "sessionVersion": 1, "contentHash": "h1" },
            "operations": [
                { "op": "insert_after", "blockId": "anchor-y", "afterNode": { "type": "paragraph" } },
                { "op": "insert_after", "blockId": "anchor-y", "afterNode": { "type": "paragraph" } }
            ]
        }),
        true,
    )
    .unwrap();
    assert_eq!(same_direction, AgentEditOperation::BlockOps);

    let err = resolve_agent_edit_operation(
        &json!({
            "path": "notes/a.docx",
            "operation": "block_ops",
            "base": { "sessionVersion": 1, "contentHash": "h1" },
            "operations": [
                { "op": "insert_before", "blockId": "anchor-x", "afterNode": { "type": "paragraph" } },
                { "op": "insert_after", "blockId": "anchor-x", "afterNode": { "type": "paragraph" } }
            ]
        }),
        true,
    )
    .unwrap_err();
    assert_eq!(err.code, "AG_TOOL_ARG");
    assert!(err.message.contains("锚点矛盾"));
}

// covers: BR-AG-DATA-009
// 目标存在性区分：整篇重写只作为目标不存在时的防御分支，已有文本目标必须走显式按块/结构化操作。
#[test]
fn block_ops_target_existence_distinguishes_replace_document() {
    let params = json!({ "content": "# 新正文" });
    assert_eq!(
        resolve_agent_edit_operation(&params, false).unwrap(),
        AgentEditOperation::ReplaceDocument
    );
    let existing = resolve_agent_edit_operation(&params, true).unwrap_err();
    assert_eq!(existing.code, "AG_PRECISE_TARGET_REQUIRED");
    let unknown =
        resolve_agent_edit_operation(&json!({ "operation": "unknown" }), true).unwrap_err();
    assert_eq!(unknown.code, "AG_TOOL_ARG");
    let missing = resolve_agent_edit_operation(&json!({}), true).unwrap_err();
    assert_eq!(missing.code, "AG_TOOL_ARG");
}

// covers: BR-AG-DATA-005
// 一章 90 块不再被退回：单次调用不设操作总数上限，落卡时按每 64 个自动拆卡。
#[test]
fn block_ops_accepts_long_chapter_within_single_call_cap() {
    let mut operations = Vec::new();
    for i in 0..90 {
        operations.push(serde_json::json!({
            "op": "append",
            "afterNode": { "type": "paragraph", "text": format!("段落 {i}") }
        }));
    }
    let resolved = resolve_agent_edit_operation(
        &json!({
            "path": "notes/a.docx",
            "operation": "block_ops",
            "base": { "sessionVersion": 1, "contentHash": "h1" },
            "operations": operations
        }),
        true,
    )
    .unwrap();
    assert_eq!(resolved, AgentEditOperation::BlockOps);
}

#[test]
fn propose_edit_rejects_mixed_operation_payloads_before_execution() {
    let err = resolve_agent_edit_operation(
        &json!({
            "path": "notes/a.docx",
            "operation": "replace_block",
            "block_id": "b1",
            "new_text": "新块",
            "style_ops": [
                { "op": "set_mark", "blockId": "b1", "start": 0, "end": 1, "mark": { "type": "bold" } }
            ]
        }),
        true,
    )
    .unwrap_err();
    assert_eq!(err.code, "AG_TOOL_ARG");
    assert!(err.message.contains("不允许混合"));

    // base 是所有结构化 op 的公共快照字段，不属于 table_ops 专属载荷；
    // replace_block + base 通过混合检测（base 由调用方在富文档路径另行校验，AG_PRECISE_TARGET_REQUIRED 拦截富文档误用）。
    let op = resolve_agent_edit_operation(
        &json!({
            "path": "notes/a.docx",
            "operation": "replace_block",
            "block_id": "b1",
            "new_text": "新块",
            "base": { "sessionVersion": 1, "contentHash": "h1" }
        }),
        true,
    )
    .unwrap();
    assert_eq!(op, AgentEditOperation::ReplaceBlock);
}

// covers: BR-AG-DATA-003
// 修复回归：style_ops + base 是合法富文档样式载荷，不得被误判为混合 table_ops。
#[test]
fn style_ops_with_structured_base_is_not_mixed_payload() {
    let op = resolve_agent_edit_operation(
        &json!({
            "path": "notes/a.docx",
            "operation": "style_ops",
            "style_ops": [
                { "blockId": "b_0", "start": 0, "end": 11, "op": "set_mark", "mark": { "type": "textStyle", "attrs": { "color": "#0000FF" } } }
            ],
            "base": { "sessionVersion": 5, "contentHash": "h5" }
        }),
        true,
    )
    .unwrap();
    assert_eq!(op, AgentEditOperation::StyleOps);

    // table_ops 专属载荷（operations/target）仍构成混合判定。
    let err = resolve_agent_edit_operation(
        &json!({
            "path": "notes/a.docx",
            "operation": "style_ops",
            "style_ops": [
                { "blockId": "b_0", "start": 0, "end": 1, "op": "set_mark", "mark": { "type": "bold" } }
            ],
            "operations": [{ "op": "set_cell_background", "tableId": "t1", "fingerprint": "fp" }]
        }),
        true,
    )
    .unwrap_err();
    assert_eq!(err.code, "AG_TOOL_ARG");
    assert!(err.message.contains("不允许混合"));
}

#[test]
fn image_ops_is_a_distinct_typed_operation_and_html_tag_projection_is_stable() {
    let op = resolve_agent_edit_operation(
        &json!({
            "path": "notes/a.docx",
            "operation": "image_ops",
            "base": { "sessionVersion": 8, "contentHash": "h8" },
            "operations": [{ "op": "delete_image", "imageId": "image_0", "fingerprint": "fp0" }]
        }),
        true,
    )
    .unwrap();
    assert_eq!(op, AgentEditOperation::ImageOps);
    assert_eq!(
        html_image_tags(r#"<p>x</p><IMG src="media/a.png"><p>y</p>"#),
        vec![r#"<IMG src="media/a.png">"#]
    );
    assert_eq!(
        top_level_html_block_ids(
            r#"<p data-block-id="top">正文</p><ul data-block-id="list"><li data-block-id="nested">项</li></ul>"#
        ),
        std::collections::BTreeSet::from(["list".to_string(), "top".to_string()])
    );
    assert!(guard_bdocx_raw_image_edit(
        "notes/a.docx",
        r#"<p><img src="media/a.png"></p><p>旧</p>"#,
        r#"<p><img src="media/a.png"></p><p>新</p>"#,
        false,
    )
    .is_ok());
    assert_eq!(
        guard_bdocx_raw_image_edit(
            "notes/a.docx",
            r#"<p><img src="media/a.png"></p>"#,
            r#"<p><img src="MEDIA/A.PNG"></p>"#,
            false,
        )
        .unwrap_err()
        .code,
        "AG_IMAGE_OPERATION_REQUIRED"
    );
    assert_eq!(
        guard_bdocx_raw_image_edit(
            "notes/a.docx",
            r#"<p><img src="media/a.png"></p>"#,
            "<p>删除图片</p>",
            false,
        )
        .unwrap_err()
        .code,
        "AG_IMAGE_OPERATION_REQUIRED"
    );
    assert_eq!(
        guard_bdocx_raw_image_edit(
            "notes/a.docx",
            r#"<p><img src="media/a.png"></p>"#,
            r#"<p><img src="media/a.png"></p>"#,
            true,
        )
        .unwrap_err()
        .code,
        "AG_IMAGE_OPERATION_REQUIRED"
    );
}

#[test]
fn bdocx_replace_document_guard_requires_structured_ops_for_tables_and_images() {
    let snapshot = ag::blocks::AgentEditableSnapshot {
        rel_path: "notes/a.docx".to_string(),
        editable_kind: "bdocx".to_string(),
        source: "active_editor".to_string(),
        content_format: "html".to_string(),
        content: r#"<p data-block-id="b1">正文</p>"#.to_string(),
        baseline: None,
        session_version: Some(1),
        content_hash: Some("h1".to_string()),
        blocks: Vec::new(),
    };
    let no_protected_structure = bdocx_route_context(Vec::new(), Vec::new());
    assert!(guard_bdocx_replace_document(
        &no_protected_structure,
        "notes/a.docx",
        Some(&snapshot),
        r#"<p data-block-id="b1">改写后的正文</p>"#,
    )
    .is_ok());

    let table_context = bdocx_route_context(vec![json!({ "tableId": "t1" })], Vec::new());
    let table_error = guard_bdocx_replace_document(
        &table_context,
        "notes/a.docx",
        Some(&snapshot),
        r#"<p data-block-id="b1">改写后的正文</p>"#,
    )
    .unwrap_err();
    assert_eq!(
        table_error.code,
        "AG_BDOCX_REPLACE_DOCUMENT_REQUIRES_STRUCTURED_OPS"
    );
    assert!(table_error.message.contains("表格"));

    let image_context = bdocx_route_context(Vec::new(), vec![json!({ "imageId": "i1" })]);
    let image_error = guard_bdocx_replace_document(
        &image_context,
        "notes/a.docx",
        Some(&snapshot),
        r#"<p data-block-id="b1">改写后的正文</p>"#,
    )
    .unwrap_err();
    assert_eq!(
        image_error.code,
        "AG_BDOCX_REPLACE_DOCUMENT_REQUIRES_STRUCTURED_OPS"
    );
    assert!(image_error.message.contains("图片"));

    // 动态结构投影缺失时仍需对当前 HTML fail-closed，不能把表格/图片误判成可整篇重写。
    let snapshot_with_table = ag::blocks::AgentEditableSnapshot {
        content: r#"<table data-block-id="table_1"><tr><td>既有表格</td></tr></table>"#.to_string(),
        ..snapshot.clone()
    };
    assert_eq!(
        guard_bdocx_replace_document(
            &no_protected_structure,
            "notes/a.docx",
            Some(&snapshot_with_table),
            r#"<p data-block-id="b1">改写后的正文</p>"#,
        )
        .unwrap_err()
        .code,
        "AG_BDOCX_REPLACE_DOCUMENT_REQUIRES_STRUCTURED_OPS"
    );
    let snapshot_with_image = ag::blocks::AgentEditableSnapshot {
        content: r#"<p data-block-id="image_1"><img src="media/image.png"></p>"#.to_string(),
        ..snapshot.clone()
    };
    assert_eq!(
        guard_bdocx_replace_document(
            &no_protected_structure,
            "notes/a.docx",
            Some(&snapshot_with_image),
            r#"<p data-block-id="b1">改写后的正文</p>"#,
        )
        .unwrap_err()
        .code,
        "AG_BDOCX_REPLACE_DOCUMENT_REQUIRES_STRUCTURED_OPS"
    );

    let introduced_structure_error = guard_bdocx_replace_document(
        &no_protected_structure,
        "notes/a.docx",
        Some(&snapshot),
        "<table><tr><td>新表格</td></tr></table>",
    )
    .unwrap_err();
    assert_eq!(
        introduced_structure_error.code,
        "AG_BDOCX_REPLACE_DOCUMENT_REQUIRES_STRUCTURED_OPS"
    );
}

#[test]
fn table_ops_requires_complete_typed_operations_and_plain_text_cells() {
    let snapshot = table_editable_snapshot();
    let base = json!({ "sessionVersion": 7, "contentHash": "h7" });
    let target = json!({ "tableId": "table_1", "fingerprint": "fp_1" });
    let malformed = json!({ "base": base, "target": target, "operations": [{ "op": "merge_cells", "from": { "row": 0, "column": 0 } }] });
    assert_eq!(
        table_ops_payload(&malformed, &snapshot, &[table_snapshot("plain_text")])
            .unwrap_err()
            .code,
        "AG_TABLE_OPERATION_INVALID"
    );

    let rich = json!({
        "base": { "sessionVersion": 7, "contentHash": "h7" },
        "target": { "tableId": "table_1", "fingerprint": "fp_1" },
        "operations": [{ "op": "set_cell_text", "at": { "row": 0, "column": 0 }, "text": "覆盖" }]
    });
    assert_eq!(
        table_ops_payload(&rich, &snapshot, &[table_snapshot("rich_content")])
            .unwrap_err()
            .code,
        "AG_TABLE_RICH_CELL_UNSUPPORTED"
    );

    let valid = json!({
        "base": { "sessionVersion": 7, "contentHash": "h7" },
        "target": { "tableId": "table_1", "fingerprint": "fp_1" },
        "operations": [{ "op": "set_cell_text", "at": { "row": 0, "column": 0 }, "text": "安全" }]
    });
    assert!(table_ops_payload(&valid, &snapshot, &[table_snapshot("plain_text")]).is_ok());

    let rich_background = json!({
        "base": { "sessionVersion": 7, "contentHash": "h7" },
        "target": { "tableId": "table_1", "fingerprint": "fp_1" },
        "operations": [{ "op": "set_cell_background", "at": { "row": 0, "column": 0 }, "color": "#FFE699" }]
    });
    assert!(table_ops_payload(
        &rich_background,
        &snapshot,
        &[table_snapshot("rich_content")]
    )
    .is_ok());

    let invalid_color = json!({
        "base": { "sessionVersion": 7, "contentHash": "h7" },
        "target": { "tableId": "table_1", "fingerprint": "fp_1" },
        "operations": [{ "op": "set_cell_background", "at": { "row": 0, "column": 0 }, "color": "yellow" }]
    });
    assert_eq!(
        table_ops_payload(&invalid_color, &snapshot, &[table_snapshot("plain_text")])
            .unwrap_err()
            .code,
        "AG_TABLE_OPERATION_INVALID"
    );
}

// covers: BR-SYS-DATA-006, BR-ED-DATA-001, BR-ED-DATA-003, BR-WS-DATA-001
#[test]
fn image_ops_validates_base_anchors_hash_and_materializes_only_after_full_batch_validation() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let baseline = unique as u64;
    let root = std::env::temp_dir().join(format!("binder-agent-image-{}", unique));
    std::fs::create_dir_all(root.join("images")).unwrap();
    let source = root.join("images/new.png");
    let bytes = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 1, 2, 3];
    std::fs::write(&source, bytes).unwrap();
    let asset = doc::image_media::inspect_project_image(&root, "images/new.png").unwrap();
    let snapshot = image_editable_snapshot(baseline);
    let images = vec![json!({
        "imageId": "image_stable",
        "fingerprint": "fp0",
        "blockId": "img1",
        "standalone": true,
        "capabilities": ["delete_image", "resize_image", "move_image", "set_image_layout"],
        "resizeBounds": { "min": 40, "max": 800 },
        "moveDirections": ["up", "down"]
    })];

    let invalid_batch = json!({
        "base": { "sessionVersion": 8, "contentHash": "h8" },
        "operations": [
            {
                "op": "insert_image",
                "anchorBlockId": "b1",
                "position": "after",
                "assetId": asset.asset_id,
                "sourcePath": "images/new.png",
                "sourceHash": asset.content_hash
            },
            { "op": "delete_image", "imageId": "missing", "fingerprint": "missing" }
        ]
    });
    assert_eq!(
        image_ops_payload(&invalid_batch, &snapshot, &images, &root)
            .unwrap_err()
            .code,
        "AG_IMAGE_ANCHOR_LOST"
    );
    assert!(!doc::image_media::edit_media_dir(baseline)
        .join(format!("media/binder-project-{}.png", asset.content_hash))
        .exists());

    let valid = json!({
        "base": { "sessionVersion": 8, "contentHash": "h8" },
        "operations": [
            {
                "op": "insert_image",
                "anchorBlockId": "b1",
                "position": "after",
                "assetId": asset.asset_id,
                "sourcePath": "images/new.png",
                "sourceHash": asset.content_hash,
                "alt": "项目图",
                "width": 240
            },
            { "op": "delete_image", "imageId": "image_stable", "fingerprint": "fp0" }
        ]
    });
    let payload = image_ops_payload(&valid, &snapshot, &images, &root).unwrap();
    let media_ref = payload["operations"][0]["mediaRef"].as_str().unwrap();
    assert!(doc::image_media::edit_media_dir(baseline)
        .join(media_ref)
        .is_file());
    assert_eq!(std::fs::read(&source).unwrap(), bytes);

    let stale = json!({
        "base": { "sessionVersion": 7, "contentHash": "h8" },
        "operations": [{ "op": "delete_image", "imageId": "image_stable", "fingerprint": "fp0" }]
    });
    assert_eq!(
        image_ops_payload(&stale, &snapshot, &images, &root)
            .unwrap_err()
            .code,
        "AG_IMAGE_STALE_SNAPSHOT"
    );
    for operation in [
        json!({ "op": "resize_image", "imageId": "image_stable", "fingerprint": "fp0", "width": 320 }),
        json!({ "op": "move_image", "imageId": "image_stable", "fingerprint": "fp0", "direction": "up" }),
        json!({ "op": "set_image_layout", "imageId": "image_stable", "fingerprint": "fp0", "alignment": "right" }),
    ] {
        let payload = json!({
            "base": { "sessionVersion": 8, "contentHash": "h8" },
            "operations": [operation]
        });
        assert!(image_ops_payload(&payload, &snapshot, &images, &root).is_ok());
    }
    let composite = json!({
        "base": { "sessionVersion": 8, "contentHash": "h8" },
        "operations": [
            { "op": "move_image", "imageId": "image_stable", "fingerprint": "fp0", "direction": "up" },
            { "op": "set_image_layout", "imageId": "image_stable", "fingerprint": "fp0", "alignment": "center" },
            { "op": "resize_image", "imageId": "image_stable", "fingerprint": "fp0", "width": 320 }
        ]
    });
    assert!(image_ops_payload(&composite, &snapshot, &images, &root).is_ok());

    for invalid in [
        json!({
            "base": { "sessionVersion": 8, "contentHash": "h8" },
            "operations": [
                { "op": "resize_image", "imageId": "image_stable", "fingerprint": "fp0", "width": 320 },
                { "op": "resize_image", "imageId": "image_stable", "fingerprint": "fp0", "width": 360 }
            ]
        }),
        json!({
            "base": { "sessionVersion": 8, "contentHash": "h8" },
            "operations": [
                { "op": "delete_image", "imageId": "image_stable", "fingerprint": "fp0" },
                { "op": "set_image_layout", "imageId": "image_stable", "fingerprint": "fp0", "alignment": "center" }
            ]
        }),
    ] {
        assert_eq!(
            image_ops_payload(&invalid, &snapshot, &images, &root)
                .unwrap_err()
                .code,
            "AG_IMAGE_OPERATION_INVALID"
        );
    }
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(doc::image_media::edit_media_dir(baseline));
}

// covers: BR-AG-STATE-002
// WS-RESOURCE-WRITE-GUARD-01（F1，X-FORBID-006）：propose_edit 目标路径不存在必须拒绝
// （AG_TARGET_NOT_EXISTS，与意图分类无关），存在路径放行——内容 Diff 不得创建资源。
#[test]
fn guard_propose_edit_target_rejects_non_existent_path() {
    let err = guard_propose_edit_target(false).unwrap_err();
    assert_eq!(err.code, "AG_TARGET_NOT_EXISTS");
    assert!(err.message.contains("plan_workspace_operation"));
    assert!(guard_propose_edit_target(true).is_ok());
}
