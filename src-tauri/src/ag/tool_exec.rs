//! PI-AG-08B · 工具执行（只读：list_docs / read_doc / search_text）。
//!
//! 经 WS 在 **active workspace** 内执行（X-FORBID-004 边界守卫），结果回注模型（SM-AG-02 ResultInjecting）。
//! 复用 lib.rs route_* 同款 workspace 访问；权限/注册校验归 tool_registry::validate_requested_tool（BR-AG-STATE-002）。
//! 写意图工具 propose_edit 由 AG 回合专用路径生成 ChangeProposal→DE diff（X-FORBID-006），不进入本只读执行器。

use crate::ipc::IpcError;
use std::path::Path;

/// 执行只读工具，返回回注模型的结果文本（JSON 序列化）。`root` 为 active workspace 根（X-FORBID-004）。
pub fn exec_read_only(
    root: &Path,
    name: &str,
    args: &serde_json::Value,
) -> Result<String, IpcError> {
    match name {
        "list_docs" => {
            let docs = crate::workspace::listing::list_docs(root)?;
            Ok(serde_json::to_string(&docs).unwrap_or_else(|_| "[]".into()))
        }
        "search_text" => {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let hits = crate::workspace::listing::search(root, query);
            Ok(serde_json::to_string(&hits).unwrap_or_else(|_| "[]".into()))
        }
        "read_doc" => {
            let rel = args.get("path").and_then(|v| v.as_str()).ok_or_else(|| {
                IpcError::new("AG_TOOL_ARG", "read_doc 缺 path 参数", false, false)
            })?;
            // 路径限 workspace 内（X-FORBID-004）。
            let abs = crate::workspace::resolve_in_workspace(root, rel)?;
            let read = crate::workspace::save::read_doc(&abs)?;
            Ok(serde_json::to_string(&read).unwrap_or_else(|_| "{}".into()))
        }
        other => Err(IpcError::new(
            "AG_TOOL_UNKNOWN",
            &format!("只读工具执行器不支持: {other}（propose_edit 由 ChangeProposal→DE 专用路径处理）"),
            false,
            false,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_tool_errors() {
        let r = exec_read_only(
            Path::new("/tmp"),
            "delete_everything",
            &serde_json::Value::Null,
        );
        assert_eq!(r.unwrap_err().code, "AG_TOOL_UNKNOWN");
    }

    #[test]
    fn read_doc_missing_path_errors() {
        let r = exec_read_only(Path::new("/tmp"), "read_doc", &serde_json::json!({}));
        assert_eq!(r.unwrap_err().code, "AG_TOOL_ARG");
    }

    #[test]
    fn list_docs_on_temp_dir_returns_entries() {
        let dir = std::env::temp_dir().join(format!("ag-toolexec-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.md"), "# hi").unwrap();
        let out = exec_read_only(&dir, "list_docs", &serde_json::Value::Null).unwrap();
        assert!(out.contains("a.md"), "list_docs 应含 a.md: {out}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
