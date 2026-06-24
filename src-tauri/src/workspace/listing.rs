//! PI-FE-04 · workspace 内文件枚举（WS-FS / BR-WS-DATA-001 / X-FORBID-004）
//!
//! 承接 A-WS-I-P-01 §3 `ws.list_docs`：递归枚举活动 workspace root 下文件，回 DocRef 列表。
//! 边界守护：仅枚举 root 下子树，跳过隐藏项（.binder/.git 等点开头条目），不跟随符号链接逃逸
//! （symlink canonicalize 深化为实现期 A 类债务，与 workspace/mod.rs 既有注记一致）。

use crate::ipc::IpcError;
use std::path::Path;

/// 文件树列举条目（WS 本地结构，非 SYS 共享 DocRef）：文件或目录。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FsEntry {
    /// 相对 root、`/` 分隔的路径。
    pub rel_path: String,
    /// 文件扩展名推断的类型（目录为空串）。
    pub doc_type: String,
    /// 是否目录（true 时即便为空也在文件树可见）。
    pub is_dir: bool,
}

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
 * boundary: in=workspaceRoot | out=FsEntryList(filesAndDirsWithinRootBoundaryHiddenSkipped)
 */
/// 递归枚举 workspace root 下文件与目录为 FsEntry 列表（BR-WS-DATA-001）：含**空目录**（is_dir=true），
/// 使目录结构在文件树可见；隐藏项整树跳过；读目录失败回统一错误形状。每个 rel_path 以 `/` 分隔、相对
/// root，天然不越界（X-FORBID-004 由"只下行不上溯"结构保证）。
pub fn list_docs(root: &Path) -> Result<Vec<FsEntry>, IpcError> {
    let mut out = Vec::new();
    walk(root, root, &mut out)?;
    out.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    Ok(out)
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<FsEntry>) -> Result<(), IpcError> {
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
        // rel_path 相对 root，统一 `/` 分隔（跨平台前端建树键）。
        let rel = path
            .strip_prefix(root)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| name.to_string());
        if file_type.is_dir() {
            // 目录条目（含空目录可见）+ 递归子树。
            out.push(FsEntry { rel_path: rel, doc_type: String::new(), is_dir: true });
            walk(root, &path, out)?;
        } else if file_type.is_file() {
            out.push(FsEntry { rel_path: rel, doc_type: doc_type_of(&path), is_dir: false });
        }
    }
    Ok(())
}

/// 全文搜索命中（行级定位）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SearchHit {
    pub rel_path: String,
    /// 1-based 行号。
    pub line: u32,
    /// 命中行预览（截断）。
    pub preview: String,
}

const SEARCH_MAX_HITS: usize = 200;
const SEARCH_MAX_FILE_BYTES: u64 = 2 * 1024 * 1024; // 跳过 >2MB 文件
const PREVIEW_MAX: usize = 160;

fn search_walk(root: &Path, dir: &Path, needle: &str, out: &mut Vec<SearchHit>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return }; // 单目录不可读则跳过，不中断整次搜索
    for entry in entries.flatten() {
        if out.len() >= SEARCH_MAX_HITS {
            return;
        }
        let name = entry.file_name();
        if is_hidden(&name.to_string_lossy()) {
            continue;
        }
        let path = entry.path();
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_dir() {
            search_walk(root, &path, needle, out);
        } else if ft.is_file() {
            if entry.metadata().map(|m| m.len() > SEARCH_MAX_FILE_BYTES).unwrap_or(true) {
                continue; // 超大/不可读跳过
            }
            // 仅文本文件（非 UTF-8 读取失败则跳过）。
            let Ok(content) = std::fs::read_to_string(&path) else { continue };
            let rel = path
                .strip_prefix(root)
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|_| name.to_string_lossy().to_string());
            for (i, raw) in content.lines().enumerate() {
                if raw.to_lowercase().contains(needle) {
                    let preview: String = raw.trim().chars().take(PREVIEW_MAX).collect();
                    out.push(SearchHit { rel_path: rel.clone(), line: (i + 1) as u32, preview });
                    if out.len() >= SEARCH_MAX_HITS {
                        return;
                    }
                }
            }
        }
    }
}

/**
 * @GOV
 * codes:    BR-WS-DATA-001-QUERY-WS-WS-FS-005
 * type:     QUERY
 * chain:    WS-FS
 * rules:    BR-WS-DATA-001
 * term_ref: TERM-WS-001
 * boundary: in=workspaceRootAndQuery | out=SearchHitList(withinRootBoundaryHiddenSkippedCapped)
 */
/// workspace 内全文搜索（BR-WS-DATA-001 / X-FORBID-004）：递归仅在 root 子树内、行级大小写不敏感子串匹配；
/// 跳隐藏项/非 UTF-8/超大文件；命中数封顶。空查询回空（不全表扫描）。
pub fn search(root: &Path, query: &str) -> Vec<SearchHit> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    search_walk(root, root, &needle, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // covers: BR-WS-DATA-001
    // 搜索：行级大小写不敏感子串匹配，回 rel_path+行号+预览；跳隐藏目录；空查询回空。
    #[test]
    fn search_matches_lines_case_insensitive_skipping_hidden() {
        let tmp = std::env::temp_dir().join(format!("binder_search_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("notes")).unwrap();
        fs::create_dir_all(tmp.join(".binder")).unwrap();
        fs::write(tmp.join("a.md"), "Hello World\nsecond line").unwrap();
        fs::write(tmp.join("notes/b.txt"), "another HELLO here").unwrap();
        fs::write(tmp.join(".binder/secret.md"), "hello hidden").unwrap();

        let hits = search(&tmp, "hello");
        // 命中 a.md:1 与 notes/b.txt:1（大小写不敏感），隐藏目录跳过
        let paths: Vec<(&str, u32)> = hits.iter().map(|h| (h.rel_path.as_str(), h.line)).collect();
        assert!(paths.contains(&("a.md", 1)));
        assert!(paths.contains(&("notes/b.txt", 1)));
        assert!(!hits.iter().any(|h| h.rel_path.contains(".binder")), "隐藏目录不应命中");
        assert_eq!(hits.iter().find(|h| h.rel_path == "a.md").unwrap().preview, "Hello World");
        // 空查询回空（不全表扫描）
        assert!(search(&tmp, "   ").is_empty());

        let _ = fs::remove_dir_all(&tmp);
    }

    // covers: BR-WS-DATA-001
    // 枚举返回 root 下文件与目录的 FsEntry（含**空目录**、统一 / 分隔、按 rel_path 有序）；
    // 隐藏项（.binder）整树跳过；文件 doc_type 由扩展名推断、目录为空串。
    #[test]
    fn lists_files_and_dirs_including_empty_skipping_hidden() {
        let tmp = std::env::temp_dir().join(format!("binder_list_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("notes")).unwrap();
        fs::create_dir_all(tmp.join("empty_dir")).unwrap(); // 空目录：应仍可见
        fs::create_dir_all(tmp.join(".binder")).unwrap();
        fs::write(tmp.join("a.md"), b"a").unwrap();
        fs::write(tmp.join("notes/b.txt"), b"b").unwrap();
        fs::write(tmp.join(".binder/secret.db"), b"x").unwrap();

        let entries = list_docs(&tmp).unwrap();
        let paths: Vec<&str> = entries.iter().map(|d| d.rel_path.as_str()).collect();
        assert_eq!(paths, vec!["a.md", "empty_dir", "notes", "notes/b.txt"], "含空目录、跳隐藏、有序");
        let by = |p: &str| entries.iter().find(|e| e.rel_path == p).unwrap();
        assert!(by("empty_dir").is_dir && by("notes").is_dir);
        assert!(!by("a.md").is_dir && by("a.md").doc_type == "md");
        assert!(!by("notes/b.txt").is_dir && by("notes/b.txt").doc_type == "txt");
        assert!(!entries.iter().any(|e| e.rel_path.contains(".binder")), "隐藏目录须跳过");

        let _ = fs::remove_dir_all(&tmp);
    }

    // covers: BR-WS-DATA-001
    // 不存在的目录回统一错误形状（不 panic、不静默成功）。
    #[test]
    fn missing_root_returns_unified_error() {
        let missing = std::env::temp_dir().join("binder_list_missing_xyz_404");
        let _ = fs::remove_dir_all(&missing);
        let err = list_docs(&missing).unwrap_err();
        assert_eq!(err.code, "WS_LIST_READ_FAILED");
        assert!(err.retriable);
    }
}
