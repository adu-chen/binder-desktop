//! Agent block 查询与显式引用上下文的运行时适配。

use crate::ipc::{CommandRequest, IpcError};
use crate::types::ReferencePayload;
#[cfg(test)]
use crate::types::ReferenceSessionMessage;
use crate::{active_ws, ag, str_param, AppState};

/**
 * @GOV
 * codes:    BR-AG-DATA-001-QUERY-AG-AG-TURN-029
 * type:     QUERY
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-001
 * boundary: in=WorkspaceDocumentIdentityAndOpenDocSnapshots | out=AgentEditableBlockSnapshotFromLogicalStateOrDiskState
 */
pub(crate) fn route_list_blocks(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let ws = active_ws(state)?;
    let open_doc_snapshots: Vec<ag::blocks::OpenDocSnapshot> = request
        .params
        .get("openDocSnapshots")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| {
            IpcError::new(
                "AG_OPEN_DOC_SNAPSHOT_INVALID",
                &format!("openDocSnapshots 解析失败: {e}"),
                false,
                false,
            )
        })?
        .unwrap_or_default();
    let read_ctx =
        ag::blocks::AgentReadContext::new(std::path::Path::new(&ws.root_path), open_doc_snapshots)?;
    let snapshot = read_ctx.snapshot(&rel_path)?;
    Ok(serde_json::json!({
        "rel_path": snapshot.rel_path,
        "editable_kind": snapshot.editable_kind,
        "content_format": snapshot.content_format,
        "source": snapshot.source,
        "blocks": snapshot.blocks,
    }))
}

/// 块级引用（选区精确锚，PI-AG-09F）：用户引用某文档某块的选中内容。serde camelCase 对接前端。
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BlockRef {
    pub(crate) rel_path: String,
    pub(crate) block_id: String,
    pub(crate) start: Option<usize>,
    pub(crate) end: Option<usize>,
    pub(crate) text: String,
}

pub(super) fn reference_target_path(payload: &ReferencePayload) -> Option<&str> {
    match payload {
        ReferencePayload::Doc { rel_path, .. } | ReferencePayload::Selection { rel_path, .. } => {
            Some(rel_path.as_str())
        }
        _ => None,
    }
}

/**
 * @GOV
 * codes:    BR-AG-DATA-001-RB-AG-AG-TURN-017
 * type:     RB
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-001
 * term_ref: TERM-REF-003
 * boundary: in=ReferencePayloadAndAgentReadContext | out=explicitReferenceContextBlock(validatedReadSnapshot) | delegate=reference::context::format_payload_context
 */
/// 引用上下文读取+分派（AG 组装职责，REF-REFERENCE-REPAIR-01 F3）：doc/selection 经 AgentReadContext
/// 读取（已打开文档优先 LogicalState），格式化与锚校验委托 reference::context（BR-REF-DATA-003 /
/// BR-REF-VERIFY-001）；REF 模块不再持有 AG 类型，规则逻辑集中于 REF。
pub(super) fn format_reference_payload_context(
    payload: &ReferencePayload,
    read_ctx: Option<&ag::blocks::AgentReadContext>,
    cap: usize,
) -> String {
    use crate::reference::context::DocRead;
    match payload {
        ReferencePayload::Doc { rel_path, .. } => {
            let Some(ctx) = read_ctx else {
                return crate::reference::context::format_payload_context(
                    payload,
                    Some(DocRead::NoWorkspace),
                    None,
                    cap,
                );
            };
            match ctx.snapshot(rel_path) {
                Ok(snapshot) => crate::reference::context::format_payload_context(
                    payload,
                    Some(DocRead::Loaded {
                        content: &snapshot.content,
                    }),
                    None,
                    cap,
                ),
                Err(e) => crate::reference::context::format_payload_context(
                    payload,
                    Some(DocRead::Failed {
                        code: &e.code,
                        message: &e.message,
                    }),
                    None,
                    cap,
                ),
            }
        }
        ReferencePayload::Selection { .. } => {
            let Some(ctx) = read_ctx else {
                return crate::reference::context::format_payload_context(payload, None, None, cap);
            };
            let check = crate::reference::context::validate_selection(
                payload,
                selection_block_text(ctx, payload).as_deref(),
            );
            crate::reference::context::format_payload_context(payload, None, Some(check), cap)
        }
        _ => crate::reference::context::format_payload_context(payload, None, None, cap),
    }
}

/// 经 AgentReadContext 取选区所在块的当前文本（LogicalState 优先；读取失败/块缺失 → None → 保守 Stale）。
pub(super) fn selection_block_text(
    read_ctx: &ag::blocks::AgentReadContext,
    payload: &ReferencePayload,
) -> Option<String> {
    let ReferencePayload::Selection {
        rel_path, block_id, ..
    } = payload
    else {
        return None;
    };
    let snapshot = read_ctx.snapshot(rel_path).ok()?;
    snapshot
        .blocks
        .iter()
        .find(|b| b.block_id.as_deref() == Some(block_id.as_str()))
        .map(|b| b.text.clone())
}

#[cfg(test)]
mod ag_reference_context_tests {
    use super::*;

    fn block_ref(start: Option<usize>, end: Option<usize>) -> BlockRef {
        BlockRef {
            rel_path: "chapter.md".to_string(),
            block_id: "block_1".to_string(),
            start,
            end,
            text: "引用文本".to_string(),
        }
    }

    // covers: BR-REF-DATA-003
    // 块引用上下文（REF-REFERENCE-REPAIR-01 F3：委托 reference::context 纯函数）。
    #[test]
    fn block_ref_context_preserves_selection_range() {
        let br = block_ref(Some(2), Some(8));
        let out = crate::reference::context::format_block_ref_context(
            &br.rel_path,
            &br.block_id,
            br.start,
            br.end,
            "引用文本",
        );
        assert!(out.contains("chapter.md"));
        assert!(out.contains("block_1"));
        assert!(out.contains("字符区间 [2..8]"));
        assert!(out.ends_with("引用文本"));
    }

    #[test]
    fn block_ref_context_degrades_invalid_range_without_dropping_text() {
        for br in [
            block_ref(None, None),
            block_ref(Some(3), None),
            block_ref(Some(8), Some(2)),
        ] {
            let out = crate::reference::context::format_block_ref_context(
                &br.rel_path,
                &br.block_id,
                br.start,
                br.end,
                "引用文本",
            );
            assert!(out.contains("字符区间未提供"));
            assert!(out.ends_with("引用文本"));
        }
    }

    #[test]
    fn reference_payload_context_formats_url_and_long_text() {
        let url = ReferencePayload::Url {
            url: "https://example.com".to_string(),
            title: Some("Example".to_string()),
            label: None,
        };
        assert_eq!(
            format_reference_payload_context(&url, None, 1000),
            "引用外部链接「Example」：https://example.com"
        );

        let long = ReferencePayload::LongText {
            text: "abcdef".repeat(10),
            char_count: 60,
            label: None,
        };
        let out = format_reference_payload_context(&long, None, 20);
        assert!(out.contains("引用长文本（60 字）"));
        assert!(out.contains("[引用长文本较长"));
    }

    #[test]
    fn reference_payload_context_formats_session_snapshot() {
        let payload = ReferencePayload::Session {
            session_id: "s1".to_string(),
            title: "需求讨论".to_string(),
            messages: vec![
                ReferenceSessionMessage {
                    role: "user".to_string(),
                    content: "先做引用".to_string(),
                },
                ReferenceSessionMessage {
                    role: "assistant".to_string(),
                    content: "收到".to_string(),
                },
            ],
            label: None,
        };
        let out = format_reference_payload_context(&payload, None, 1000);
        assert!(out.contains("引用会话「需求讨论」"));
        assert!(out.contains("user: 先做引用"));
        assert!(out.contains("assistant: 收到"));
    }

    #[test]
    fn reference_payload_context_reads_doc_from_workspace() {
        let root = std::env::temp_dir().join(format!(
            "binder-ref-doc-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("notes")).unwrap();
        std::fs::write(root.join("notes/a.md"), "文档引用内容").unwrap();
        let payload = ReferencePayload::Doc {
            rel_path: "notes/a.md".to_string(),
            label: None,
        };

        let ctx = ag::blocks::AgentReadContext::new(&root, Vec::new()).unwrap();
        let out = format_reference_payload_context(&payload, Some(&ctx), 1000);

        assert!(out.contains("引用文档「notes/a.md」"));
        assert!(out.contains("文档引用内容"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn reference_payload_context_reads_doc_from_open_logical_state() {
        let root = std::env::temp_dir().join(format!(
            "binder-ref-open-doc-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("notes")).unwrap();
        std::fs::write(root.join("notes/a.md"), "磁盘旧内容").unwrap();
        let ctx = ag::blocks::AgentReadContext::new(
            &root,
            vec![ag::blocks::OpenDocSnapshot {
                rel_path: "notes/a.md".to_string(),
                content: "打开后的最新内容".to_string(),
                baseline: None,
                edit_format: "text".to_string(),
                content_format: "text".to_string(),
                source: "open_editor_session".to_string(),
                dirty: true,
                session_version: 2,
                content_hash: "h2".to_string(),
                captured_at: 1,
                block_snapshots: Vec::new(),
                table_snapshots: Vec::new(),
                image_snapshots: Vec::new(),
            }],
        )
        .unwrap();
        let payload = ReferencePayload::Doc {
            rel_path: "notes/a.md".to_string(),
            label: None,
        };

        let out = format_reference_payload_context(&payload, Some(&ctx), 1000);

        assert!(out.contains("打开后的最新内容"));
        assert!(!out.contains("磁盘旧内容"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn reference_payload_context_formats_selection_snapshot() {
        let payload = ReferencePayload::Selection {
            rel_path: "notes/a.md".to_string(),
            block_id: "block-1".to_string(),
            start: Some(2),
            end: Some(6),
            text: "选区引用内容".to_string(),
            label: None,
        };

        let out = format_reference_payload_context(&payload, None, 1000);

        assert!(out.contains("引用锚已失效"));
        assert!(out.contains("不得作为当前文档事实"));
        assert!(out.ends_with("选区引用内容"));
    }

    #[test]
    fn reference_payload_context_validates_selection_against_open_snapshot() {
        let root = std::env::temp_dir().join(format!(
            "binder-ref-selection-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("a.html"),
            "<p data-block-id=\"b1\">0123456789</p>",
        )
        .unwrap();
        let ctx = ag::blocks::AgentReadContext::new(
            &root,
            vec![ag::blocks::OpenDocSnapshot {
                rel_path: "a.html".to_string(),
                content: "<p data-block-id=\"b1\">0123456789</p>".to_string(),
                baseline: None,
                edit_format: "html".to_string(),
                content_format: "html".to_string(),
                source: "active_editor".to_string(),
                dirty: true,
                session_version: 4,
                content_hash: "h4".to_string(),
                captured_at: 1,
                block_snapshots: vec![ag::blocks::AgentEditableBlockSnapshot {
                    block_id: "b1".to_string(),
                    block_index: 0,
                    block_type: "p".to_string(),
                    node_type: "paragraph".to_string(),
                    role: "content".to_string(),
                    parent_block_id: None,
                    depth: 0,
                    ordinal: 0,
                    text: "0123456789".to_string(),
                    attrs: serde_json::json!({"blockId": "b1"}),
                    marks: Vec::new(),
                    node_json: serde_json::json!({
                        "type": "paragraph",
                        "attrs": {"blockId": "b1"},
                        "content": [{"type": "text", "text": "0123456789"}]
                    }),
                    fingerprint: "fp-b1".to_string(),
                    capabilities: vec!["replace_node".to_string()],
                }],
                table_snapshots: Vec::new(),
                image_snapshots: Vec::new(),
            }],
        )
        .unwrap();
        let payload = ReferencePayload::Selection {
            rel_path: "a.html".to_string(),
            block_id: "b1".to_string(),
            start: Some(2),
            end: Some(6),
            text: "2345".to_string(),
            label: None,
        };

        let out = format_reference_payload_context(&payload, Some(&ctx), 1000);

        assert!(out.contains("引用选区（已校验）"));
        assert!(out.contains("2345"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn reference_payload_context_formats_asset_snapshots() {
        let memory = ReferencePayload::Memory {
            entry_id: Some("m1".to_string()),
            title: Some("偏好".to_string()),
            content: "用户偏好简洁回答".to_string(),
            label: None,
        };
        let knowledge = ReferencePayload::Knowledge {
            entry_id: Some("k1".to_string()),
            title: Some("术语表".to_string()),
            content: "ReferencePayload 是输入侧引用载荷".to_string(),
            source_ref: Some("kb/ref.md".to_string()),
            label: None,
        };
        let template = ReferencePayload::Template {
            template_id: Some("tpl1".to_string()),
            name: "周报模板".to_string(),
            content: "本周完成：\n下周计划：".to_string(),
            label: None,
        };

        let memory_out = format_reference_payload_context(&memory, None, 1000);
        let knowledge_out = format_reference_payload_context(&knowledge, None, 1000);
        let template_out = format_reference_payload_context(&template, None, 1000);

        assert!(memory_out.contains("引用记忆「偏好」"));
        assert!(memory_out.contains("用户偏好简洁回答"));
        assert!(knowledge_out.contains("引用知识「术语表」（来源：kb/ref.md）"));
        assert!(knowledge_out.contains("ReferencePayload 是输入侧引用载荷"));
        assert!(template_out.contains("引用模板「周报模板」"));
        assert!(template_out.contains("本周完成"));
    }

    #[test]
    fn reference_payload_declares_doc_targets() {
        let doc = ReferencePayload::Doc {
            rel_path: "a.md".to_string(),
            label: None,
        };
        let selection = ReferencePayload::Selection {
            rel_path: "b.md".to_string(),
            block_id: "block".to_string(),
            start: Some(1),
            end: Some(3),
            text: "bc".to_string(),
            label: None,
        };
        let memory = ReferencePayload::Memory {
            entry_id: None,
            title: None,
            content: "偏好".to_string(),
            label: None,
        };
        assert_eq!(reference_target_path(&doc), Some("a.md"));
        assert_eq!(reference_target_path(&selection), Some("b.md"));
        assert_eq!(reference_target_path(&memory), None);
    }
}
