//! Agent block 查询与显式引用上下文的运行时适配。

use crate::ipc::{CommandRequest, IpcError};
use crate::types::ReferencePayload;
#[cfg(test)]
use crate::types::ReferenceSessionMessage;
use crate::{active_ws, ag, str_param, AppState};

pub(super) struct ReferenceContextProjection {
    pub(super) text: String,
    pub(super) source_type: ag::context::PromptSourceTypeV1,
    pub(super) source_id: String,
    pub(super) provenance: String,
    pub(super) freshness: String,
    pub(super) trust_level: ag::context::PromptTrustLevelV1,
    pub(super) original_char_count: usize,
    pub(super) truncation_reason: Option<String>,
}

fn current_unix_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn snapshot_freshness(
    read_ctx: &ag::blocks::AgentReadContext,
    rel_path: &str,
    snapshot: &ag::blocks::AgentEditableSnapshot,
) -> String {
    if let Some(open) = read_ctx.open_snapshot(rel_path) {
        format!(
            "logical_state_captured_at={};session_version={};content_hash={}",
            open.captured_at, open.session_version, open.content_hash
        )
    } else {
        format!(
            "disk_state_read_at={};baseline={}",
            current_unix_ms(),
            snapshot
                .baseline
                .map(|baseline| baseline.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        )
    }
}

fn payload_original_chars(payload: &ReferencePayload) -> usize {
    match payload {
        ReferencePayload::Doc { .. } => 0,
        ReferencePayload::Url { url, .. } => url.chars().count(),
        ReferencePayload::Selection { text, .. } | ReferencePayload::LongText { text, .. } => {
            text.chars().count()
        }
        ReferencePayload::Session { messages, .. } => messages
            .iter()
            .map(|message| message.content.chars().count())
            .sum(),
        ReferencePayload::Memory { content, .. }
        | ReferencePayload::Knowledge { content, .. }
        | ReferencePayload::Template { content, .. } => content.chars().count(),
        ReferencePayload::File { read_content, .. } => read_content
            .as_deref()
            .map(|content| content.chars().count())
            .unwrap_or(0),
    }
}

fn payload_identity(
    payload: &ReferencePayload,
) -> (ag::context::PromptSourceTypeV1, String, String) {
    match payload {
        ReferencePayload::Doc { rel_path, .. } => (
            ag::context::PromptSourceTypeV1::ExplicitReference,
            format!("doc:{rel_path}"),
            "explicit_document_reference".to_string(),
        ),
        ReferencePayload::Selection {
            rel_path,
            block_id,
            start,
            end,
            ..
        } => (
            ag::context::PromptSourceTypeV1::Selection,
            format!(
                "selection:{rel_path}#{block_id}:{}-{}",
                start
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "?".to_string()),
                end.map(|value| value.to_string())
                    .unwrap_or_else(|| "?".to_string())
            ),
            "explicit_selection_reference".to_string(),
        ),
        ReferencePayload::Session { session_id, .. } => (
            ag::context::PromptSourceTypeV1::ExplicitReference,
            format!("session:{session_id}"),
            "explicit_session_snapshot".to_string(),
        ),
        ReferencePayload::Memory {
            entry_id, title, ..
        } => (
            ag::context::PromptSourceTypeV1::ExplicitReference,
            format!(
                "memory:{}",
                entry_id
                    .as_deref()
                    .or(title.as_deref())
                    .unwrap_or("unidentified")
            ),
            "explicit_memory_snapshot".to_string(),
        ),
        ReferencePayload::Knowledge {
            entry_id, title, ..
        } => (
            ag::context::PromptSourceTypeV1::ExplicitReference,
            format!(
                "knowledge:{}",
                entry_id
                    .as_deref()
                    .or(title.as_deref())
                    .unwrap_or("unidentified")
            ),
            "explicit_knowledge_snapshot".to_string(),
        ),
        ReferencePayload::Template {
            template_id, name, ..
        } => (
            ag::context::PromptSourceTypeV1::ExplicitReference,
            format!(
                "template:{}",
                template_id.as_deref().unwrap_or(name.as_str())
            ),
            "explicit_template_snapshot".to_string(),
        ),
        ReferencePayload::Url { url, .. } => (
            ag::context::PromptSourceTypeV1::ExplicitReference,
            format!("url:{}", ag::context::sha256_text(url)),
            "explicit_url_metadata".to_string(),
        ),
        ReferencePayload::LongText { text, .. } => (
            ag::context::PromptSourceTypeV1::ExplicitReference,
            format!("long_text:{}", ag::context::sha256_text(text)),
            "explicit_text_snapshot".to_string(),
        ),
        ReferencePayload::File {
            name,
            path,
            read_content,
            ..
        } => (
            ag::context::PromptSourceTypeV1::ExplicitReference,
            format!("file:{}", path.as_deref().unwrap_or(name)),
            if read_content.is_some() {
                "explicit_external_file_snapshot"
            } else {
                "explicit_external_file_metadata"
            }
            .to_string(),
        ),
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
pub(super) fn build_reference_context_projection(
    payload: &ReferencePayload,
    read_ctx: Option<&ag::blocks::AgentReadContext>,
    cap: usize,
) -> ReferenceContextProjection {
    use crate::reference::context::{DocRead, SelectionCheck};

    let (source_type, source_id, mut provenance) = payload_identity(payload);
    let mut original_char_count = payload_original_chars(payload);
    let mut freshness = "submitted_for_current_turn".to_string();
    let mut trust_level = ag::context::PromptTrustLevelV1::UserProvidedData;
    let text = match payload {
        ReferencePayload::Doc { rel_path, .. } => match read_ctx {
            Some(ctx) => match ctx.snapshot(rel_path) {
                Ok(snapshot) => {
                    original_char_count = snapshot.content.chars().count();
                    freshness = snapshot_freshness(ctx, rel_path, &snapshot);
                    provenance = format!("{provenance}:{}", snapshot.source);
                    trust_level = ag::context::PromptTrustLevelV1::CurrentFact;
                    crate::reference::context::format_payload_context(
                        payload,
                        Some(DocRead::Loaded {
                            content: &snapshot.content,
                        }),
                        None,
                        cap,
                    )
                }
                Err(error) => {
                    freshness = format!("read_failed_at={}", current_unix_ms());
                    provenance = format!("{provenance}:read_failed");
                    crate::reference::context::format_payload_context(
                        payload,
                        Some(DocRead::Failed {
                            code: &error.code,
                            message: &error.message,
                        }),
                        None,
                        cap,
                    )
                }
            },
            None => crate::reference::context::format_payload_context(
                payload,
                Some(DocRead::NoWorkspace),
                None,
                cap,
            ),
        },
        ReferencePayload::Selection { rel_path, .. } => match read_ctx {
            Some(ctx) => match ctx.snapshot(rel_path) {
                Ok(snapshot) => {
                    freshness = snapshot_freshness(ctx, rel_path, &snapshot);
                    provenance = format!("{provenance}:{}", snapshot.source);
                    let current_block = match payload {
                        ReferencePayload::Selection { block_id, .. } => snapshot
                            .blocks
                            .iter()
                            .find(|block| block.block_id.as_deref() == Some(block_id.as_str()))
                            .map(|block| block.text.as_str()),
                        _ => None,
                    };
                    let check =
                        crate::reference::context::validate_selection(payload, current_block);
                    if matches!(check, SelectionCheck::Located { .. }) {
                        trust_level = ag::context::PromptTrustLevelV1::CurrentFact;
                    }
                    crate::reference::context::format_payload_context(
                        payload,
                        None,
                        Some(check),
                        cap,
                    )
                }
                Err(_) => {
                    freshness = format!("selection_stale_at={}", current_unix_ms());
                    provenance = format!("{provenance}:read_failed");
                    let check = SelectionCheck::Stale {
                        reason: "当前文档块读取不可得",
                    };
                    crate::reference::context::format_payload_context(
                        payload,
                        None,
                        Some(check),
                        cap,
                    )
                }
            },
            None => crate::reference::context::format_payload_context(payload, None, None, cap),
        },
        _ => crate::reference::context::format_payload_context(payload, None, None, cap),
    };
    let truncation_reason = (original_char_count > cap)
        .then(|| format!("source_exceeded_per_reference_cap:{original_char_count}>{cap}"));
    ReferenceContextProjection {
        text,
        source_type,
        source_id,
        provenance,
        freshness,
        trust_level,
        original_char_count,
        truncation_reason,
    }
}

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

/// 引用上下文读取+分派（AG 组装职责，REF-REFERENCE-REPAIR-01 F3）：doc/selection 经 AgentReadContext
/// 读取（已打开文档优先 LogicalState），格式化与锚校验委托 reference::context（BR-REF-DATA-003 /
/// BR-REF-VERIFY-001）；REF 模块不再持有 AG 类型，规则逻辑集中于 REF。
#[cfg(test)]
pub(super) fn format_reference_payload_context(
    payload: &ReferencePayload,
    read_ctx: Option<&ag::blocks::AgentReadContext>,
    cap: usize,
) -> String {
    build_reference_context_projection(payload, read_ctx, cap).text
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

    // covers: BR-AG-DATA-001, BR-AG-DATA-002, BR-AG-DATA-005
    #[test]
    fn reference_projection_records_identity_hash_budget_and_truncation() {
        let payload = ReferencePayload::LongText {
            text: "abcdef".repeat(1_000),
            char_count: 6_000,
            label: None,
        };
        let projection = build_reference_context_projection(&payload, None, 20);
        assert!(projection.source_id.starts_with("long_text:"));
        assert_eq!(projection.provenance, "explicit_text_snapshot");
        assert_eq!(projection.freshness, "submitted_for_current_turn");
        assert_eq!(projection.original_char_count, 6_000);
        assert!(projection.truncation_reason.is_some());

        let envelope = ag::context::prompt_data_envelope_with_budget(
            projection.source_type,
            &projection.source_id,
            &projection.provenance,
            &projection.freshness,
            projection.trust_level,
            &projection.text,
            projection.original_char_count,
            projection.truncation_reason.as_deref(),
        );
        assert_eq!(
            envelope.content_hash,
            ag::context::sha256_text(&projection.text)
        );
        assert_eq!(envelope.original_char_count, 6_000);
        assert!(envelope.included_char_count < envelope.original_char_count);
        assert!(envelope.truncation_reason.is_some());
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
