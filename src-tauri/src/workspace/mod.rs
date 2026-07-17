//! PI-WS-01 · workspace 生命周期与文件 I/O 边界 · WS-FS / BR-WS-DATA-001 / X-FORBID-004
//!
//! 承接 A-WS-M-T-01：WS 是文件/搜索/diff/上下文操作的唯一物理边界。本 PI 落实路径边界守护
//! （X-FORBID-004：任一操作路径不得越出活动 workspace root）与 DocRef 定位（BR-WS-DATA-001）。
//! workspace 生命周期 CRUD（open/switch/close）依赖 db workspaces 表，落 PI-WS-05；本模块聚焦
//! 文件 I/O 物理边界守护。符号链接层逃逸防护（canonicalize）为实现期深化（A 类）。

use crate::ipc::IpcError;
use crate::types::{DocRef, WorkspaceRef};
use std::path::{Component, Path, PathBuf};

/// PI-WS-05 · workspace 注册与生命周期持久化（控制态，BR-WS-PERSIST-002）。
pub mod lifecycle;

/// PI-WS-02 · save 保存机与外部变更冲突保护（BR-WS-PERSIST-001 / BR-WS-OBS-001 / SM-WS-01）。
pub mod save;

/// PI-WS-04 · 文档级时间轴存储与还原（BR-WS-DATA-002 / BR-WS-STATE-002 / SM-WS-03）。
pub mod timeline;

/// PI-WS-03 · pending_diffs 持久化与裁决后落盘（BR-WS-STATE-001 / SM-WS-02）。
pub mod diff_apply;

/// PI-FE-04 · workspace 内文件枚举（ws.list_docs，BR-WS-DATA-001 / X-FORBID-004）。
pub mod listing;

/// PI-BE-04 · workspace 内文件 CRUD（create/rename/delete，BR-WS-DATA-001 / X-FORBID-004）。
pub mod fileops;

fn escape_error(rel_path: &str) -> IpcError {
    IpcError::new(
        "WS_PATH_ESCAPE",
        &format!("路径越出 workspace 边界: {rel_path}"),
        false,
        false,
    )
}

/**
 * @GOV
 * codes:    BR-WS-DATA-001-GUARD-WS-WS-FS-001
 * type:     GUARD
 * chain:    WS-FS
 * rules:    BR-WS-DATA-001
 * boundary: in=workspaceRootAndRelPath | out=safeAbsolutePath(rejectedWhenEscaping)
 */
/// 将 workspace 内相对路径安全解析为绝对路径（X-FORBID-004 守护）：
/// 逐路径组件解析——普通组件入栈，`.` 跳过，`..` 出栈（栈空即越界）；拒绝绝对路径与盘符前缀。
/// 任一越界一律拒绝并回统一错误形状（不静默截断、不回退到 workspace 外）。
pub fn resolve_in_workspace(root: &Path, rel_path: &str) -> Result<PathBuf, IpcError> {
    let rel = Path::new(rel_path);
    let mut stack: Vec<&std::ffi::OsStr> = Vec::new();
    for comp in rel.components() {
        match comp {
            Component::Normal(c) => stack.push(c),
            Component::CurDir => {}
            Component::ParentDir => {
                if stack.pop().is_none() {
                    return Err(escape_error(rel_path)); // .. 越出 root
                }
            }
            // 绝对路径 / 盘符前缀：不接受（必须是 workspace 内相对路径）。
            Component::RootDir | Component::Prefix(_) => {
                return Err(escape_error(rel_path));
            }
        }
    }
    let mut result = root.to_path_buf();
    for c in stack {
        result.push(c);
    }
    Ok(result)
}

/// 以 DocRef 定位 workspace 内文档的绝对路径（BR-WS-DATA-001：统一以 DocRef 定位）。
pub fn resolve_doc(ws: &WorkspaceRef, doc: &DocRef) -> Result<PathBuf, IpcError> {
    resolve_in_workspace(Path::new(&ws.root_path), &doc.rel_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ws() -> WorkspaceRef {
        WorkspaceRef {
            workspace_id: "ws1".into(),
            root_path: "/home/u/workspace".into(),
            is_active: true,
        }
    }

    // covers: BR-WS-DATA-001
    // workspace 内合法相对路径（含内部 .. 不逃逸）解析到 root 下。
    #[test]
    fn in_bounds_paths_resolve_under_root() {
        let root = Path::new("/home/u/workspace");
        assert_eq!(
            resolve_in_workspace(root, "a/b.md").unwrap(),
            PathBuf::from("/home/u/workspace/a/b.md")
        );
        // 内部回溯但不逃逸：a/x/../b → a/b
        assert_eq!(
            resolve_in_workspace(root, "a/x/../b.md").unwrap(),
            PathBuf::from("/home/u/workspace/a/b.md")
        );
        // ./ 当前目录
        assert_eq!(
            resolve_in_workspace(root, "./a.md").unwrap(),
            PathBuf::from("/home/u/workspace/a.md")
        );
    }

    // covers: BR-WS-DATA-001
    // X-FORBID-004：越界路径（.. 逃逸 / 绝对路径）一律拒绝并回统一错误形状。
    #[test]
    fn escaping_paths_rejected_with_unified_error() {
        let root = Path::new("/home/u/workspace");
        for bad in [
            "../secret",
            "../../etc/passwd",
            "a/../../out",
            "/etc/passwd",
        ] {
            let err = resolve_in_workspace(root, bad).unwrap_err();
            assert_eq!(err.code, "WS_PATH_ESCAPE", "应拒绝越界路径: {bad}");
            assert!(!err.retriable);
        }
    }

    // covers: BR-WS-DATA-001
    // 经 DocRef 定位等价于 root + rel_path 守护；越界 DocRef 同样被拒。
    #[test]
    fn resolve_doc_via_docref_enforces_boundary() {
        let ws = ws();
        let ok = DocRef {
            workspace_id: "ws1".into(),
            rel_path: "notes/a.md".into(),
            doc_type: "md".into(),
        };
        assert_eq!(
            resolve_doc(&ws, &ok).unwrap(),
            PathBuf::from("/home/u/workspace/notes/a.md")
        );
        let escaping = DocRef {
            workspace_id: "ws1".into(),
            rel_path: "../../../etc/x".into(),
            doc_type: "md".into(),
        };
        assert!(resolve_doc(&ws, &escaping).is_err());
    }
}
