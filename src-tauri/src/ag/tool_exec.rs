//! PI-AG-08B · 工具执行（只读：list_docs / read_doc / search_text / list_blocks / list_tables /
//! list_images；受控复制：import_image_asset）。
//!
//! 经 WS 在 **active workspace** 内执行（X-FORBID-004 边界守卫），结果回注模型（SM-AG-02 ResultInjecting）。
//! 文档内容读取统一走 AgentEditableSnapshot；权限/注册校验归 tool_registry::validate_requested_tool
//! （BR-AG-STATE-002）。
//! 写意图工具 propose_edit 由 AG 回合专用路径生成 ChangeProposal→DE diff（X-FORBID-006），不进入本只读执行器。

use crate::ipc::IpcError;
use std::path::Path;

fn path_arg(args: &serde_json::Value, tool: &str) -> Result<String, IpcError> {
    args.get("path")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or_else(|| IpcError::new("AG_TOOL_ARG", &format!("{tool} 缺 path 参数"), false, false))
}

fn string_arg(args: &serde_json::Value, key: &str, tool: &str) -> Result<String, IpcError> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            IpcError::new(
                "AG_TOOL_ARG",
                &format!("{tool} 缺 {key} 参数"),
                false,
                false,
            )
        })
}

/**
 * @GOV
 * codes:    BR-AG-STATE-002-QUERY-AG-AG-TOOL-012, BR-AG-DATA-001-QUERY-AG-AG-TOOL-012, BR-WS-DATA-001-QUERY-AG-AG-TOOL-012
 * type:     QUERY
 * chain:    AG-TOOL
 * rules:    BR-AG-STATE-002, BR-AG-DATA-001, BR-WS-DATA-001
 * term_ref: TERM-ED-008, TERM-WS-001
 * boundary: in=ActiveBDocxLogicalStateAndWorkspaceProjectImages | out=ListImagesProjection(currentInstancesAndValidatedAssets)
 */
fn exec_list_images(
    ctx: &crate::ag::blocks::AgentReadContext,
    rel: &str,
) -> Result<String, IpcError> {
    let snapshot = ctx.snapshot(rel)?;
    let images = ctx
        .image_snapshots(rel)?
        .iter()
        .cloned()
        .map(|mut image| {
            let block_id = image.get("blockId").and_then(|value| value.as_str());
            let block_index = block_id.and_then(|id| {
                snapshot
                    .blocks
                    .iter()
                    .position(|block| block.block_id.as_deref() == Some(id))
            });
            let preview = |index: usize| -> Option<String> {
                snapshot
                    .blocks
                    .get(index)
                    .map(|block| block.text.trim().chars().take(160).collect::<String>())
                    .filter(|text| !text.is_empty())
            };
            let before_text = block_index.and_then(|index| (0..index).rev().find_map(preview));
            let after_text = block_index
                .and_then(|index| ((index + 1)..snapshot.blocks.len()).find_map(preview));
            if let Some(object) = image.as_object_mut() {
                object.insert(
                    "positionContext".to_string(),
                    serde_json::json!({
                        "blockIndex": block_index,
                        "precedingText": before_text,
                        "followingText": after_text,
                    }),
                );
            }
            image
        })
        .collect::<Vec<_>>();
    let mut assets = Vec::new();
    let mut rejected_candidates = 0usize;
    let catalog = crate::workspace::listing::load_project_image_catalog(ctx.root())?;
    for candidate in catalog.candidates {
        match crate::doc::image_media::inspect_project_image(ctx.root(), &candidate) {
            Ok(asset) => assets.push(serde_json::json!({
                "assetId": asset.asset_id,
                "sourcePath": asset.rel_path,
                "sourceHash": asset.content_hash,
                "mediaType": asset.media_type,
                "byteSize": asset.byte_size,
            })),
            Err(_) => rejected_candidates += 1,
        }
    }
    Ok(serde_json::to_string(&serde_json::json!({
        "relPath": snapshot.rel_path,
        "baseline": snapshot.baseline,
        "sessionVersion": snapshot.session_version,
        "contentHash": snapshot.content_hash,
        "images": images,
        "assets": assets,
        "imageRoots": catalog.image_roots,
        "catalogSource": catalog.config_source,
        "rejectedCandidates": rejected_candidates,
        "visualContentAvailable": false,
        "contextNotice": "图片内容未被视觉识别；precedingText/followingText 仅用于文档流定位，不代表图片画面。",
    }))
    .unwrap_or_else(|_| "{}".into()))
}

/// 执行白名单资源准备工具。普通创建、删除、重命名与移动必须走 WorkspaceOperationPlan，不能进入此执行器。
pub fn exec_structure_tool(
    root: &Path,
    name: &str,
    args: &serde_json::Value,
) -> Result<String, IpcError> {
    match name {
        "import_image_asset" => {
            let source_path = string_arg(args, "source_path", "import_image_asset")?;
            let source = crate::workspace::resolve_in_workspace(root, &source_path)?;
            let asset = crate::doc::image_media::import_project_image(root, &source)?;
            Ok(serde_json::to_string(&serde_json::json!({
                "operation": "import_image_asset",
                "copiedOrReused": true,
                "sourceFilePreserved": true,
                "assetId": asset.asset_id,
                "sourcePath": asset.rel_path,
                "sourceHash": asset.content_hash,
                "mediaType": asset.media_type,
                "byteSize": asset.byte_size,
                "summary": "图片已复制或复用到受控项目图片目录；源文件保持不变。请重新调用 list_images 后提出 image_ops 修改建议。"
            }))
            .unwrap_or_else(|_| "图片已准备".into()))
        }
        other => Err(IpcError::new(
            "AG_TOOL_UNKNOWN",
            &format!("结构工具执行器不支持: {other}"),
            false,
            false,
        )),
    }
}

pub fn exec_edit_blocked(args: &serde_json::Value) -> Result<String, IpcError> {
    let code = string_arg(args, "code", "edit_blocked")?;
    let reason = string_arg(args, "reason", "edit_blocked")?;
    if code.len() > 80 || reason.len() > 1000 {
        return Err(IpcError::new(
            "AG_TOOL_ARG",
            "edit_blocked 的 code/reason 过长",
            false,
            false,
        ));
    }
    Ok(serde_json::json!({
        "status": "edit_blocked",
        "code": code,
        "reason": reason,
        "documentModified": false,
    })
    .to_string())
}

/// 执行只读工具，返回回注模型的结果文本（JSON 序列化）。`ctx` 持有 active workspace 根与打开文档
/// LogicalState 覆盖层（X-FORBID-004）。
pub fn exec_read_only(
    ctx: &crate::ag::blocks::AgentReadContext,
    name: &str,
    args: &serde_json::Value,
) -> Result<String, IpcError> {
    match name {
        "list_docs" => {
            // AG-WS-LAYERED-01：无 path → L0 概览（豁免时全量）；path → L1 一层展开。
            // 树枚举只含索引（路径/类型/计数），永不携带正文；正文仅经 read_doc。
            let rel_dir = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            if rel_dir.trim().is_empty() {
                crate::workspace::listing::list_docs_overview_json(ctx.root())
            } else {
                crate::workspace::listing::list_docs_children_json(ctx.root(), rel_dir)
            }
        }
        "workspace_glob" => {
            // AG-STRATEGY-01：按文件名通配模式定位文档（只读，不含正文）。
            let pattern = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
            if pattern.trim().is_empty() {
                return Err(IpcError::new(
                    "WS_GLOB_PATTERN_MISSING",
                    "workspace_glob 缺少 pattern 参数（如 pattern=\"*寓言*.txt\"）",
                    false,
                    false,
                ));
            }
            let hits = crate::workspace::listing::glob_files(ctx.root(), pattern)?;
            Ok(serde_json::to_string(&hits).unwrap_or_else(|_| "[]".into()))
        }
        "search_text" => {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let needle = query.trim().to_lowercase();
            let mut hits: Vec<serde_json::Value> = Vec::new();
            if !needle.is_empty() {
                for entry in crate::workspace::listing::list_docs(ctx.root())? {
                    if entry.is_dir {
                        continue;
                    }
                    let Ok(snapshot) = ctx.snapshot(&entry.rel_path) else {
                        continue;
                    };
                    for block in snapshot.blocks {
                        if !block.text.to_lowercase().contains(&needle) {
                            continue;
                        }
                        let preview: String = block.text.trim().chars().take(160).collect();
                        hits.push(serde_json::json!({
                            "rel_path": entry.rel_path.clone(),
                            "block_index": block.block_index,
                            "block_id": block.block_id,
                            "preview": preview,
                        }));
                        if hits.len() >= 200 {
                            break;
                        }
                    }
                    if hits.len() >= 200 {
                        break;
                    }
                }
            }
            Ok(serde_json::to_string(&hits).unwrap_or_else(|_| "[]".into()))
        }
        "read_doc" => {
            let rel = path_arg(args, "read_doc")?;
            let snapshot = ctx.snapshot(&rel)?;
            // AG-CTX-01：read_doc 支持 blocks 范围参数（可选）——只返回指定块的完整投影
            // （含 node_json，供 block_ops 提案从 node_json 复制节点构造 afterNode）；
            // 省略 blocks 参数时返回完整内容（向后兼容）。轻量索引走 list_blocks。
            if let Some(blocks) = args
                .get("blocks")
                .and_then(|value| value.as_array())
            {
                let wanted: Vec<&str> = blocks
                    .iter()
                    .filter_map(|value| value.as_str())
                    .collect();
                if wanted.is_empty() {
                    return Err(IpcError::new(
                        "AG_TOOL_ARG",
                        "read_doc blocks 参数必须是块 id/索引数组且不能为空",
                        false,
                        false,
                    ));
                }
                let filtered = snapshot
                    .blocks
                    .iter()
                    .filter(|block| {
                        let id = block.block_id.as_deref().unwrap_or("");
                        wanted.contains(&id)
                            || wanted.contains(&block.block_index.to_string().as_str())
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let mut slim = serde_json::to_value(&snapshot).unwrap_or(serde_json::Value::Null);
                if let Some(object) = slim.as_object_mut() {
                    object["blocks"] = serde_json::to_value(&filtered)
                        .unwrap_or(serde_json::Value::Array(Vec::new()));
                    // 只返回目标块时省略整篇 content（块投影已含 text/node_json）。
                    object.remove("content");
                }
                return Ok(slim.to_string());
            }
            Ok(serde_json::to_string(&snapshot).unwrap_or_else(|_| "{}".into()))
        }
        "list_blocks" => {
            let rel = path_arg(args, "list_blocks")?;
            let snapshot = ctx.snapshot(&rel)?;
            // AG-CTX-01：list_blocks 轻量投影——只返回块目录（无 node_json/全文/marks/attrs），
            // 结构性压缩（Claude Code Tool Result Budget 同思路），模型提案时经 read_doc(blocks) 按需取完整块。
            let directory = snapshot
                .blocks
                .iter()
                .map(|block| {
                    let mut entry = serde_json::Map::new();
                    entry.insert(
                        "block_id".to_string(),
                        serde_json::Value::String(
                            block.block_id.clone().unwrap_or_default(),
                        ),
                    );
                    entry.insert(
                        "block_index".to_string(),
                        serde_json::json!(block.block_index),
                    );
                    if let Some(node_type) = &block.node_type {
                        entry.insert(
                            "node_type".to_string(),
                            serde_json::Value::String(node_type.clone()),
                        );
                    }
                    entry.insert(
                        "role".to_string(),
                        serde_json::Value::String(block.role.clone()),
                    );
                    if let Some(parent) = &block.parent_block_id {
                        entry.insert(
                            "parent_block_id".to_string(),
                            serde_json::Value::String(parent.clone()),
                        );
                    }
                    entry.insert("depth".to_string(), serde_json::json!(block.depth));
                    let text: String = block.text.chars().take(40).collect();
                    entry.insert(
                        "text".to_string(),
                        serde_json::Value::String(text),
                    );
                    if let Some(fingerprint) = &block.fingerprint {
                        entry.insert(
                            "fingerprint".to_string(),
                            serde_json::Value::String(fingerprint.clone()),
                        );
                    }
                    entry.insert(
                        "capabilities".to_string(),
                        serde_json::Value::Array(
                            block
                                .capabilities
                                .iter()
                                .map(|c| serde_json::Value::String(c.clone()))
                                .collect(),
                        ),
                    );
                    serde_json::Value::Object(entry)
                })
                .collect::<Vec<_>>();
            Ok(serde_json::to_string(&serde_json::json!({
                "rel_path": snapshot.rel_path,
                "editable_kind": snapshot.editable_kind,
                "content_format": snapshot.content_format,
                "source": snapshot.source,
                "blocks": directory,
            }))
            .unwrap_or_else(|_| "{}".into()))
        }
        "list_tables" => {
            let rel = path_arg(args, "list_tables")?;
            let snapshot = ctx.snapshot(&rel)?;
            if !matches!(
                snapshot.source.as_str(),
                "active_editor" | "active_editor_session"
            ) || snapshot.content_format != "html"
            {
                return Err(IpcError::new(
                    "AG_TABLE_EDIT_REQUIRES_ACTIVE_EDITOR",
                    "表格结构读取必须针对当前活动的 BDocx HTML 编辑器会话；原生预览或未打开文档不可编辑。",
                    false,
                    false,
                ));
            }
            let tables = ctx.table_snapshots(&rel)?;
            Ok(serde_json::to_string(&serde_json::json!({
                "rel_path": snapshot.rel_path,
                "session_version": snapshot.session_version,
                "content_hash": snapshot.content_hash,
                "tables": tables,
            }))
            .unwrap_or_else(|_| "{}".into()))
        }
        "list_images" => {
            let rel = path_arg(args, "list_images")?;
            exec_list_images(ctx, &rel)
        }
        other => Err(IpcError::new(
            "AG_TOOL_UNKNOWN",
            &format!(
                "只读工具执行器不支持: {other}（propose_edit 由 ChangeProposal→DE 专用路径处理）"
            ),
            false,
            false,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(root: &Path) -> crate::ag::blocks::AgentReadContext {
        crate::ag::blocks::AgentReadContext::new(root, Vec::new()).unwrap()
    }

    fn active_block(
        block_id: &str,
        block_index: usize,
        text: &str,
        role: &str,
    ) -> crate::ag::blocks::AgentEditableBlockSnapshot {
        crate::ag::blocks::AgentEditableBlockSnapshot {
            block_id: block_id.to_string(),
            block_index,
            block_type: "p".to_string(),
            node_type: "paragraph".to_string(),
            role: role.to_string(),
            parent_block_id: None,
            depth: 0,
            ordinal: block_index,
            text: text.to_string(),
            attrs: serde_json::json!({ "blockId": block_id }),
            marks: Vec::new(),
            node_json: serde_json::json!({
                "type": "paragraph",
                "attrs": { "blockId": block_id },
                "content": if text.is_empty() {
                    Vec::<serde_json::Value>::new()
                } else {
                    vec![serde_json::json!({ "type": "text", "text": text })]
                },
            }),
            fingerprint: format!("fp-{block_id}"),
            capabilities: if role == "content" {
                vec!["replace_node".to_string(), "style_ops".to_string()]
            } else {
                Vec::new()
            },
        }
    }

    #[test]
    fn unknown_tool_errors() {
        let root = Path::new("/tmp");
        let r = exec_read_only(&ctx(root), "delete_everything", &serde_json::Value::Null);
        assert_eq!(r.unwrap_err().code, "AG_TOOL_UNKNOWN");
    }

    #[test]
    fn read_doc_missing_path_errors() {
        let root = Path::new("/tmp");
        let r = exec_read_only(&ctx(root), "read_doc", &serde_json::json!({}));
        assert_eq!(r.unwrap_err().code, "AG_TOOL_ARG");
    }

    #[test]
    fn list_blocks_returns_unified_block_projection_for_text_file() {
        let dir = std::env::temp_dir().join(format!("ag-toolexec-blocks-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.md"), "一\n二").unwrap();
        let out = exec_read_only(
            &ctx(&dir),
            "list_blocks",
            &serde_json::json!({ "path": "a.md" }),
        )
        .unwrap();
        assert!(out.contains("\"block_index\":0"));
        assert!(out.contains("\"text\":\"一\""));
        // AG-CTX-01 F1：list_blocks 轻量投影不得包含 node_json/完整 marks。
        assert!(!out.contains("node_json"), "轻量投影应省略 node_json");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // covers: BR-AG-DATA-005
    // AG-CTX-01 F1：read_doc 支持 blocks 范围参数——只返回指定块完整投影，省略整篇 content。
    #[test]
    fn read_doc_blocks_scope_returns_only_selected_blocks_without_full_content() {
        let dir = std::env::temp_dir().join(format!("ag-toolexec-scope-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // 无 data-block-id 的文本文件：块索引 0/1。
        std::fs::write(dir.join("a.md"), "第一行\n第二行").unwrap();
        let out = exec_read_only(
            &ctx(&dir),
            "read_doc",
            &serde_json::json!({ "path": "a.md", "blocks": ["1"] }),
        )
        .unwrap();
        assert!(out.contains("第二行"), "应返回索引 1 的块: {out}");
        assert!(!out.contains("第一行"), "不应包含未选中的块 0");
        assert!(!out.contains("\"content\":"), "范围读取应省略整篇 content: {out}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_docs_on_temp_dir_returns_entries() {
        let dir = std::env::temp_dir().join(format!("ag-toolexec-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.md"), "# hi").unwrap();
        let out = exec_read_only(&ctx(&dir), "list_docs", &serde_json::Value::Null).unwrap();
        assert!(out.contains("a.md"), "list_docs 应含 a.md: {out}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // AG-WS-LAYERED-01：list_docs 无参数 → 大工作区返回概览（mode=overview，不含 entries/正文）；
    // path 参数 → 该目录一层直接子项（mode=children）。
    #[test]
    fn list_docs_layered_overview_and_children() {
        let dir = std::env::temp_dir().join(format!("ag-toolexec-layer-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("docs/sub")).unwrap();
        std::fs::write(dir.join("root.md"), "r").unwrap();
        for i in 0..35 {
            std::fs::write(dir.join(format!("docs/f{i}.txt")), "content").unwrap();
        }
        // 无 path：36 文件 > 30 豁免 → overview。
        let overview =
            exec_read_only(&ctx(&dir), "list_docs", &serde_json::json!({})).unwrap();
        let value: serde_json::Value = serde_json::from_str(&overview).unwrap();
        assert_eq!(value["mode"], "overview");
        assert_eq!(value["total_files"], 36);
        assert!(value.get("entries").is_none(), "概览不得携带全量 entries");
        assert!(
            value["top_level_dirs"]
                .as_array()
                .unwrap()
                .iter()
                .any(|d| d["rel_path"] == "docs" && d["file_count"] == 35)
        );
        // path="docs"：一层直接子项（35 文件 + sub 目录，不递归）。
        let children =
            exec_read_only(&ctx(&dir), "list_docs", &serde_json::json!({ "path": "docs" }))
                .unwrap();
        let child_value: serde_json::Value = serde_json::from_str(&children).unwrap();
        assert_eq!(child_value["mode"], "children");
        assert_eq!(child_value["total_items"], 36); // docs/f0..34 + docs/sub
        assert!(
            child_value["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|i| i["rel_path"] == "docs/sub" && i["file_count"] == 0)
        );
        // 不存在目录 → 明确错误码。
        let err = exec_read_only(&ctx(&dir), "list_docs", &serde_json::json!({ "path": "nope" }))
            .unwrap_err();
        assert_eq!(err.code, "WS_LIST_DIR_MISSING");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // AG-STRATEGY-01：workspace_glob 按模式返回匹配文件；缺 pattern → 明确错误。
    #[test]
    fn workspace_glob_lists_matching_files() {
        let dir = std::env::temp_dir().join(format!("ag-toolexec-glob-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("寓言")).unwrap();
        std::fs::write(dir.join("寓言/小马过河.txt"), "x").unwrap();
        std::fs::write(dir.join("readme.md"), "x").unwrap();
        let out = exec_read_only(
            &ctx(&dir),
            "workspace_glob",
            &serde_json::json!({ "pattern": "*寓言*" }),
        )
        .unwrap();
        assert!(
            out.contains("寓言/小马过河.txt"),
            "glob 应命中寓言文件: {out}"
        );
        assert!(!out.contains("readme.md"), "不应命中无关文件: {out}");
        let err = exec_read_only(&ctx(&dir), "workspace_glob", &serde_json::json!({}))
            .unwrap_err();
        assert_eq!(err.code, "WS_GLOB_PATTERN_MISSING");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn search_text_prefers_open_logical_state() {
        let dir = std::env::temp_dir().join(format!("ag-toolexec-open-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.md"), "旧内容").unwrap();
        let ctx = crate::ag::blocks::AgentReadContext::new(
            &dir,
            vec![crate::ag::blocks::OpenDocSnapshot {
                rel_path: "a.md".to_string(),
                content: "最新 LogicalState".to_string(),
                baseline: None,
                edit_format: "text".to_string(),
                content_format: "text".to_string(),
                source: "open_editor_session".to_string(),
                dirty: true,
                session_version: 3,
                content_hash: "h3".to_string(),
                captured_at: 1,
                block_snapshots: Vec::new(),
                table_snapshots: Vec::new(),
                image_snapshots: Vec::new(),
            }],
        )
        .unwrap();
        let out =
            exec_read_only(&ctx, "search_text", &serde_json::json!({ "query": "最新" })).unwrap();
        assert!(out.contains("最新 LogicalState"));
        assert!(!out.contains("旧内容"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_tables_only_returns_active_html_bdocx_snapshot() {
        let dir = std::env::temp_dir().join(format!("ag-toolexec-tables-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let ctx = crate::ag::blocks::AgentReadContext::new(
            &dir,
            vec![crate::ag::blocks::OpenDocSnapshot {
                rel_path: "a.docx".to_string(),
                content: "<table data-table-id=\"table_1\"></table>".to_string(),
                baseline: Some(17),
                edit_format: "html".to_string(),
                content_format: "html".to_string(),
                source: "active_editor".to_string(),
                dirty: true,
                session_version: 7,
                content_hash: "h7".to_string(),
                captured_at: 1,
                block_snapshots: vec![active_block("table-context", 0, "", "container")],
                table_snapshots: vec![
                    serde_json::json!({ "tableId": "table_1", "rows": 1, "columns": 1 }),
                ],
                image_snapshots: Vec::new(),
            }],
        )
        .unwrap();
        let out = exec_read_only(
            &ctx,
            "list_tables",
            &serde_json::json!({ "path": "a.docx" }),
        )
        .unwrap();
        assert!(out.contains("table_1"));
        assert!(out.contains("session_version"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    // covers: BR-AG-STATE-002, BR-AG-DATA-001, BR-WS-DATA-001
    #[test]
    fn list_images_combines_active_image_snapshots_and_valid_project_assets() {
        let dir = std::env::temp_dir().join(format!("ag-toolexec-images-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("images")).unwrap();
        std::fs::write(dir.join("a.docx"), b"docx").unwrap();
        std::fs::write(
            dir.join("images/cover.png"),
            [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 1],
        )
        .unwrap();
        std::fs::write(dir.join("images/bad.png"), b"bad").unwrap();
        crate::workspace::listing::mark_bdocx(&dir, "a.docx").unwrap();
        let ctx = crate::ag::blocks::AgentReadContext::new(
            &dir,
            vec![crate::ag::blocks::OpenDocSnapshot {
                rel_path: "a.docx".to_string(),
                content: "<p data-block-id=\"before\">第一条说明</p><p data-block-id=\"b1\"><img src=\"media/a.png\"></p><p data-block-id=\"after\">第二条说明</p>".to_string(),
                baseline: Some(17),
                edit_format: "html".to_string(),
                content_format: "html".to_string(),
                source: "active_editor".to_string(),
                dirty: true,
                session_version: 8,
                content_hash: "h8".to_string(),
                captured_at: 1,
                block_snapshots: vec![
                    active_block("before", 0, "第一条说明", "content"),
                    active_block("b1", 1, "", "atom"),
                    active_block("after", 2, "第二条说明", "content"),
                ],
                table_snapshots: Vec::new(),
                image_snapshots: vec![serde_json::json!({
                    "imageId": "image_0",
                    "fingerprint": "fp_0",
                    "blockId": "b1",
                    "ordinal": 0,
                    "standalone": true
                })],
            }],
        )
        .unwrap();
        let output = exec_read_only(
            &ctx,
            "list_images",
            &serde_json::json!({ "path": "a.docx" }),
        )
        .unwrap();
        let value: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(value["sessionVersion"], 8);
        assert_eq!(value["images"][0]["imageId"], "image_0");
        assert_eq!(
            value["images"][0]["positionContext"]["precedingText"],
            "第一条说明"
        );
        assert_eq!(
            value["images"][0]["positionContext"]["followingText"],
            "第二条说明"
        );
        assert_eq!(value["visualContentAvailable"], false);
        assert_eq!(value["assets"].as_array().unwrap().len(), 1);
        assert!(value["assets"][0]["assetId"]
            .as_str()
            .is_some_and(|value| value.starts_with("asset_")));
        assert_eq!(value["assets"][0]["sourcePath"], "images/cover.png");
        assert_eq!(value["assets"][0]["sourceHash"].as_str().unwrap().len(), 64);
        assert_eq!(value["assets"][0]["mediaType"], "image/png");
        assert_eq!(value["imageRoots"], serde_json::json!(["images"]));
        assert_eq!(value["catalogSource"], "default");
        assert_eq!(value["rejectedCandidates"], 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn import_image_asset_copies_workspace_source_and_edit_blocked_is_structured() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ag-import-image-{unique}"));
        std::fs::create_dir_all(dir.join("文档/多媒体")).unwrap();
        let source = dir.join("文档/多媒体/a.png");
        let png = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 1];
        std::fs::write(&source, png).unwrap();

        let result = exec_structure_tool(
            &dir,
            "import_image_asset",
            &serde_json::json!({ "source_path": "文档/多媒体/a.png" }),
        )
        .unwrap();
        let value: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(value["sourceFilePreserved"], true);
        assert!(value["sourcePath"]
            .as_str()
            .is_some_and(|path| path.starts_with("images/imported/")));
        assert_eq!(std::fs::read(&source).unwrap(), png);

        let blocked = exec_edit_blocked(&serde_json::json!({
            "code": "IMAGE_ASSET_UNAVAILABLE",
            "reason": "没有可用图片，请用户提供项目内来源。"
        }))
        .unwrap();
        let blocked: serde_json::Value = serde_json::from_str(&blocked).unwrap();
        assert_eq!(blocked["status"], "edit_blocked");
        assert_eq!(blocked["documentModified"], false);
        let _ = std::fs::remove_dir_all(dir);
    }

}
