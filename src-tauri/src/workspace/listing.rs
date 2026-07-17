//! PI-FE-04 · workspace 内文件枚举（WS-FS / BR-WS-DATA-001 / X-FORBID-004）
//!
//! 承接 A-WS-I-P-01 §3 `ws.list_docs`：递归枚举活动 workspace root 下文件，回 DocRef 列表。
//! 边界守护：仅枚举 root 下子树，跳过隐藏项（.binder/.git 等点开头条目），不跟随符号链接逃逸
//! （symlink canonicalize 深化为实现期 A 类债务，与 workspace/mod.rs 既有注记一致）。

use crate::ipc::IpcError;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;

/// 文件树列举条目（WS 本地结构，非 SYS 共享 DocRef）：文件或目录。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FsEntry {
    /// 相对 root、`/` 分隔的路径。
    pub rel_path: String,
    /// 文件扩展名推断的类型（目录为空串）。
    pub doc_type: String,
    /// 是否目录（true 时即便为空也在文件树可见）。
    pub is_dir: bool,
    /// 是否 Binder 可编辑的 docx 类文档（BDocx）；原生 docx 保持 false，只走预览链。
    pub is_bdocx: bool,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct BDocxManifest {
    paths: Vec<String>,
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

fn bdocx_manifest_path(root: &Path) -> PathBuf {
    root.join(".binder").join("bdocx_manifest.json")
}

fn normalize_rel_path(rel_path: &str) -> String {
    rel_path.replace('\\', "/")
}

fn load_bdocx_manifest(root: &Path) -> BDocxManifest {
    let path = bdocx_manifest_path(root);
    let Ok(bytes) = std::fs::read(path) else {
        return BDocxManifest::default();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

fn write_bdocx_manifest(root: &Path, manifest: &BDocxManifest) -> Result<(), IpcError> {
    let dir = root.join(".binder");
    std::fs::create_dir_all(&dir).map_err(|e| {
        IpcError::new(
            "WS_BDOCX_MANIFEST_WRITE_FAILED",
            &format!("创建 b-docx 身份目录失败: {e}"),
            true,
            false,
        )
    })?;
    let bytes = serde_json::to_vec_pretty(manifest).map_err(|e| {
        IpcError::new(
            "WS_BDOCX_MANIFEST_WRITE_FAILED",
            &format!("序列化 b-docx 身份清单失败: {e}"),
            true,
            false,
        )
    })?;
    std::fs::write(bdocx_manifest_path(root), bytes).map_err(|e| {
        IpcError::new(
            "WS_BDOCX_MANIFEST_WRITE_FAILED",
            &format!("写入 b-docx 身份清单失败: {e}"),
            true,
            false,
        )
    })
}

fn is_docx_doc_type(doc_type: &str) -> bool {
    matches!(doc_type, "docx" | "doc")
}

fn is_draft_bdocx_rel(rel_path: &str, doc_type: &str) -> bool {
    is_docx_doc_type(doc_type) && rel_path.to_ascii_lowercase().contains(".draft.")
}

fn is_bdocx_rel(rel_path: &str, doc_type: &str, registered: &HashSet<String>) -> bool {
    is_draft_bdocx_rel(rel_path, doc_type) || registered.contains(&normalize_rel_path(rel_path))
}

/// 注册 workspace 内某个 doc/docx 文件为 BDocx（文件树新建、Agent 创建、草稿转换后调用）。
pub fn mark_bdocx(root: &Path, rel_path: &str) -> Result<(), IpcError> {
    let rel_path = normalize_rel_path(rel_path);
    let doc_type = doc_type_of(Path::new(&rel_path));
    if !is_docx_doc_type(&doc_type) {
        return Ok(());
    }
    let mut paths = load_bdocx_manifest(root).paths;
    if !paths.iter().any(|p| p == &rel_path) {
        paths.push(rel_path);
    }
    paths.sort();
    paths.dedup();
    write_bdocx_manifest(root, &BDocxManifest { paths })
}

/// 查询某个相对路径是否为 BDocx。`*.draft.docx` 是兼容识别；其余 docx 依赖清单身份。
pub fn is_bdocx(root: &Path, rel_path: &str) -> bool {
    let rel_path = normalize_rel_path(rel_path);
    let doc_type = doc_type_of(Path::new(&rel_path));
    let registered: HashSet<String> = load_bdocx_manifest(root).paths.into_iter().collect();
    is_bdocx_rel(&rel_path, &doc_type, &registered)
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
    let bdocx_paths: HashSet<String> = load_bdocx_manifest(root).paths.into_iter().collect();
    walk(root, root, &bdocx_paths, &mut out)?;
    out.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    Ok(out)
}

fn walk(
    root: &Path,
    dir: &Path,
    bdocx_paths: &HashSet<String>,
    out: &mut Vec<FsEntry>,
) -> Result<(), IpcError> {
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
            out.push(FsEntry {
                rel_path: rel,
                doc_type: String::new(),
                is_dir: true,
                is_bdocx: false,
            });
            walk(root, &path, bdocx_paths, out)?;
        } else if file_type.is_file() {
            let doc_type = doc_type_of(&path);
            let is_bdocx = is_bdocx_rel(&rel, &doc_type, bdocx_paths);
            out.push(FsEntry {
                rel_path: rel,
                doc_type,
                is_dir: false,
                is_bdocx,
            });
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
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    }; // 单目录不可读则跳过，不中断整次搜索
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
            if entry
                .metadata()
                .map(|m| m.len() > SEARCH_MAX_FILE_BYTES)
                .unwrap_or(true)
            {
                continue; // 超大/不可读跳过
            }
            // 仅文本文件（非 UTF-8 读取失败则跳过）。
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue;
            };
            let rel = path
                .strip_prefix(root)
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|_| name.to_string_lossy().to_string());
            for (i, raw) in content.lines().enumerate() {
                if raw.to_lowercase().contains(needle) {
                    let preview: String = raw.trim().chars().take(PREVIEW_MAX).collect();
                    out.push(SearchHit {
                        rel_path: rel.clone(),
                        line: (i + 1) as u32,
                        preview,
                    });
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
        assert!(
            !hits.iter().any(|h| h.rel_path.contains(".binder")),
            "隐藏目录不应命中"
        );
        assert_eq!(
            hits.iter().find(|h| h.rel_path == "a.md").unwrap().preview,
            "Hello World"
        );
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
        assert_eq!(
            paths,
            vec!["a.md", "empty_dir", "notes", "notes/b.txt"],
            "含空目录、跳隐藏、有序"
        );
        let by = |p: &str| entries.iter().find(|e| e.rel_path == p).unwrap();
        assert!(by("empty_dir").is_dir && by("notes").is_dir);
        assert!(!by("a.md").is_dir && by("a.md").doc_type == "md");
        assert!(!by("notes/b.txt").is_dir && by("notes/b.txt").doc_type == "txt");
        assert!(!by("a.md").is_bdocx);
        assert!(
            !entries.iter().any(|e| e.rel_path.contains(".binder")),
            "隐藏目录须跳过"
        );

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

    // covers: BR-WS-DATA-001
    // BDocx 身份：*.draft.docx 自动识别；文件树/Agent 创建的 docx 由隐藏清单标记；原生 docx 不误判。
    #[test]
    fn lists_bdocx_identity_from_draft_name_or_manifest() {
        let tmp = std::env::temp_dir().join(format!("binder_bdocx_list_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join("native.docx"), b"PK-native").unwrap();
        fs::write(tmp.join("draft.draft.docx"), b"PK-draft").unwrap();
        fs::write(tmp.join("created.docx"), b"PK-created").unwrap();

        mark_bdocx(&tmp, "created.docx").unwrap();
        assert!(is_bdocx(&tmp, "created.docx"));
        assert!(is_bdocx(&tmp, "draft.draft.docx"));
        assert!(!is_bdocx(&tmp, "native.docx"));

        let entries = list_docs(&tmp).unwrap();
        let by = |p: &str| entries.iter().find(|e| e.rel_path == p).unwrap();
        assert!(!by("native.docx").is_bdocx);
        assert!(by("draft.draft.docx").is_bdocx);
        assert!(by("created.docx").is_bdocx);
        assert!(!entries.iter().any(|e| e.rel_path.contains(".binder")));

        let _ = fs::remove_dir_all(&tmp);
    }
}
