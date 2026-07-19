//! PI-AG-08B · 工具执行（只读：list_docs / read_doc / search_text / list_blocks；结构：move_doc / create_folder）。
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

/// 执行非破坏性文件树结构工具。此类工具只改变 workspace 内文件树结构，不修改文档内容，不走 DE diff。
pub fn exec_structure_tool(
    root: &Path,
    name: &str,
    args: &serde_json::Value,
) -> Result<String, IpcError> {
    match name {
        "move_doc" => {
            let from_path = string_arg(args, "from_path", "move_doc")?;
            let to_path = string_arg(args, "to_path", "move_doc")?;
            let from_abs = crate::workspace::resolve_in_workspace(root, &from_path)?;
            let to_abs = crate::workspace::resolve_in_workspace(root, &to_path)?;
            crate::workspace::fileops::rename_doc(&from_abs, &to_abs)?;
            Ok(serde_json::to_string(&serde_json::json!({
                "operation": "move_doc",
                "from_path": from_path,
                "to_path": to_path,
                "summary": format!("已移动 {from_path} -> {to_path}")
            }))
            .unwrap_or_else(|_| "已移动文件".into()))
        }
        "create_folder" => {
            let path = path_arg(args, "create_folder")?;
            let abs = crate::workspace::resolve_in_workspace(root, &path)?;
            let existed = abs.is_dir();
            crate::workspace::fileops::create_folder(&abs)?;
            Ok(serde_json::to_string(&serde_json::json!({
                "operation": "create_folder",
                "path": path,
                "created": !existed,
                "existed": existed,
                "summary": if existed { "目录已存在" } else { "目录已创建" }
            }))
            .unwrap_or_else(|_| "目录已存在或已创建".into()))
        }
        other => Err(IpcError::new(
            "AG_TOOL_UNKNOWN",
            &format!("结构工具执行器不支持: {other}"),
            false,
            false,
        )),
    }
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
            let docs = crate::workspace::listing::list_docs(ctx.root())?;
            Ok(serde_json::to_string(&docs).unwrap_or_else(|_| "[]".into()))
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
            Ok(serde_json::to_string(&snapshot).unwrap_or_else(|_| "{}".into()))
        }
        "list_blocks" => {
            let rel = path_arg(args, "list_blocks")?;
            let snapshot = ctx.snapshot(&rel)?;
            Ok(serde_json::to_string(&serde_json::json!({
                "rel_path": snapshot.rel_path,
                "editable_kind": snapshot.editable_kind,
                "content_format": snapshot.content_format,
                "source": snapshot.source,
                "blocks": snapshot.blocks,
            }))
            .unwrap_or_else(|_| "{}".into()))
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
                edit_format: "text".to_string(),
                source: "open_editor_session".to_string(),
                dirty: true,
                session_version: 3,
                content_hash: "h3".to_string(),
                captured_at: 1,
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
    fn create_folder_structure_tool_creates_directory() {
        let dir = std::env::temp_dir().join(format!("ag-structure-folder-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let out = exec_structure_tool(
            &dir,
            "create_folder",
            &serde_json::json!({ "path": "分类/文学作品" }),
        )
        .unwrap();
        assert!(dir.join("分类/文学作品").is_dir());
        assert!(out.contains("create_folder"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn move_doc_structure_tool_moves_without_overwrite() {
        let dir = std::env::temp_dir().join(format!("ag-structure-move-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.md"), "hello").unwrap();
        let out = exec_structure_tool(
            &dir,
            "move_doc",
            &serde_json::json!({ "from_path": "a.md", "to_path": "分类/a.md" }),
        )
        .unwrap();
        assert!(!dir.join("a.md").exists());
        assert_eq!(
            std::fs::read_to_string(dir.join("分类/a.md")).unwrap(),
            "hello"
        );
        assert!(out.contains("move_doc"));
        std::fs::write(dir.join("b.md"), "b").unwrap();
        assert_eq!(
            exec_structure_tool(
                &dir,
                "move_doc",
                &serde_json::json!({ "from_path": "b.md", "to_path": "分类/a.md" }),
            )
            .unwrap_err()
            .code,
            "WS_EXISTS"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
