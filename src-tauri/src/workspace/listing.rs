//! PI-FE-04 · workspace 内文件枚举（WS-FS / BR-WS-DATA-001 / X-FORBID-004）
//!
//! 承接 A-WS-I-P-01 §3 `ws.list_docs`：递归枚举活动 workspace root 下文件，回 DocRef 列表。
//! 边界守护：仅枚举 root 下子树，跳过隐藏项（.binder/.git 等点开头条目），不跟随符号链接逃逸
//! （symlink canonicalize 深化为实现期 A 类债务，与 workspace/mod.rs 既有注记一致）。

use crate::ipc::IpcError;
use crate::types::DocRef;
use std::path::Path;

/// 由扩展名推断文档类型（小写无点）；无扩展名归 `bin`（前端按支持集决定可编辑性）。
fn doc_type_of(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_else(|| "bin".to_string())
}

/// 是否隐藏条目（点开头）——枚举时整体跳过（含其子树）。
fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

/**
 * @GOV
 * codes:    BR-WS-DATA-001-QUERY-WS-WS-FS-002
 * type:     QUERY
 * chain:    WS-FS
 * rules:    BR-WS-DATA-001
 * term_ref: TERM-WS-001
 * boundary: in=workspaceRootAndId | out=DocRefList(withinRootBoundaryHiddenSkipped)
 */
/// 递归枚举 workspace root 下文件为 DocRef 列表（BR-WS-DATA-001：统一以 DocRef 定位）。
/// 仅返回常规文件（目录结构由 rel_path 隐含，前端建树）；隐藏项整树跳过；读目录失败回统一错误形状。
/// 每个 rel_path 以 `/` 分隔、相对 root，天然不越界（X-FORBID-004 由"只下行不上溯"结构保证）。
pub fn list_docs(root: &Path, workspace_id: &str) -> Result<Vec<DocRef>, IpcError> {
    let mut out = Vec::new();
    walk(root, root, workspace_id, &mut out)?;
    out.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    Ok(out)
}

fn walk(root: &Path, dir: &Path, workspace_id: &str, out: &mut Vec<DocRef>) -> Result<(), IpcError> {
    let entries = std::fs::read_dir(dir).map_err(|e| {
        IpcError::new(
            "WS_LIST_READ_FAILED",
            &format!("读取目录失败: {e}"),
            true,
            false,
        )
    })?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if is_hidden(&name) {
            continue; // 跳过隐藏条目及其子树（.binder/.git 等）
        }
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue, // 元信息不可读则跳过该条目，不中断整次枚举
        };
        if file_type.is_dir() {
            walk(root, &path, workspace_id, out)?;
        } else if file_type.is_file() {
            // rel_path 相对 root，统一 `/` 分隔（跨平台前端建树键）。
            let rel = path
                .strip_prefix(root)
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|_| name.to_string());
            out.push(DocRef {
                workspace_id: workspace_id.to_string(),
                rel_path: rel,
                doc_type: doc_type_of(&path),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // covers: BR-WS-DATA-001
    // 枚举返回 root 下常规文件的 DocRef（含子目录、统一 / 分隔、按 rel_path 有序）；
    // 隐藏项（.binder）整树跳过；doc_type 由扩展名推断。
    #[test]
    fn lists_files_within_root_skipping_hidden() {
        let tmp = std::env::temp_dir().join(format!("binder_list_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("notes")).unwrap();
        fs::create_dir_all(tmp.join(".binder")).unwrap();
        fs::write(tmp.join("a.md"), b"a").unwrap();
        fs::write(tmp.join("notes/b.txt"), b"b").unwrap();
        fs::write(tmp.join(".binder/secret.db"), b"x").unwrap();

        let docs = list_docs(&tmp, "ws1").unwrap();
        let paths: Vec<&str> = docs.iter().map(|d| d.rel_path.as_str()).collect();
        assert_eq!(paths, vec!["a.md", "notes/b.txt"], "隐藏目录须跳过且有序");
        assert_eq!(docs[0].doc_type, "md");
        assert_eq!(docs[1].doc_type, "txt");
        assert!(docs.iter().all(|d| d.workspace_id == "ws1"));

        let _ = fs::remove_dir_all(&tmp);
    }

    // covers: BR-WS-DATA-001
    // 不存在的目录回统一错误形状（不 panic、不静默成功）。
    #[test]
    fn missing_root_returns_unified_error() {
        let missing = std::env::temp_dir().join("binder_list_missing_xyz_404");
        let _ = fs::remove_dir_all(&missing);
        let err = list_docs(&missing, "ws1").unwrap_err();
        assert_eq!(err.code, "WS_LIST_READ_FAILED");
        assert!(err.retriable);
    }
}
