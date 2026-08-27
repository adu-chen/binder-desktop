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

/// 文件重命名时同步更新 bdocx_manifest：若旧路径已注册为 BDocx，移除旧路径；
/// 若新路径是 docx 类型，注册为新 BDocx。
/// 审计修复（2026-07-24）：之前 rename 不更新清单，导致旧 BDocx 路径残留、新路径未注册。
pub fn update_bdocx_for_rename(root: &Path, old_rel: &str, new_rel: &str) -> Result<(), IpcError> {
    let old = normalize_rel_path(old_rel);
    let new = normalize_rel_path(new_rel);
    let mut paths: Vec<String> = load_bdocx_manifest(root).paths;
    let had_old = paths.iter().any(|p| p == &old);
    // 移除旧路径
    paths.retain(|p| p != &old);
    // 若旧路径是 BDocx 或新路径是 docx 类型，注册新路径
    let new_doc_type = doc_type_of(Path::new(&new));
    if (had_old || is_docx_doc_type(&new_doc_type)) && !paths.iter().any(|p| p == &new) {
        paths.push(new);
    }
    paths.sort();
    paths.dedup();
    if had_old || is_docx_doc_type(&new_doc_type) {
        write_bdocx_manifest(root, &BDocxManifest { paths })
    } else {
        Ok(())
    }
}

/**
 * @GOV
 * codes:    BR-WS-DATA-001-EFFECT-WS-WS-FS-008
 * type:     EFFECT
 * chain:    WS-FS
 * rules:    BR-WS-DATA-001
 * boundary: in=workspaceRootAndDeletedDirRelPath | out=bdocxManifestWithoutDeletedPrefixPaths(unchangedWhenNoHit)
 */
/// 删除目录后清洗 bdocx_manifest（WS-DIR-DELETE-01）：移除 rel_path 等于 dir_rel 或以其 `dir_rel/` 为前缀的
/// 已注册路径，防陈旧清单。无命中不写文件（避免无谓 IO）。文件级删除无需清洗（清单只登记 docx 文件路径，
/// 文件删除不产生子路径残留）。
pub fn cleanup_bdocx_under(root: &Path, dir_rel: &str) -> Result<(), IpcError> {
    let dir_rel = normalize_rel_path(dir_rel);
    let prefix = format!("{dir_rel}/");
    let paths: Vec<String> = load_bdocx_manifest(root).paths;
    let retained: Vec<String> = paths
        .iter()
        .filter(|p| *p != &dir_rel && !p.starts_with(&prefix))
        .cloned()
        .collect();
    if retained.len() == paths.len() {
        return Ok(()); // 无命中不写
    }
    write_bdocx_manifest(root, &BDocxManifest { paths: retained })
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

// AG-WS-LAYERED-01：分层感知豁免阈值——工作区文件数 ≤30 且全量序列化 ≤4KB 时，
// list_docs() 返回全量扁平列表（mode=full），否则返回概览（mode=overview）。
const OVERVIEW_FULL_FILE_LIMIT: usize = 30;
const OVERVIEW_FULL_BYTES_LIMIT: usize = 4096;

/// 工作区概览（AG-WS-LAYERED-01 L0）：顶层目录 + 文件计数 + 总文件数；不含正文。
/// 豁免：文件数 ≤30 且全量序列化 ≤4KB → 返回全量扁平列表（mode=full）。
/// 返回 JSON 字符串（AG list_docs 工具无 path 参数时直接注入）。
pub fn list_docs_overview_json(root: &Path) -> Result<String, IpcError> {
    let docs = list_docs(root)?;
    let files: Vec<&FsEntry> = docs.iter().filter(|e| !e.is_dir).collect();
    let total_files = files.len();
    let full_json = serde_json::to_string(&docs).unwrap_or_default();
    if total_files <= OVERVIEW_FULL_FILE_LIMIT && full_json.len() <= OVERVIEW_FULL_BYTES_LIMIT {
        return Ok(serde_json::json!({
            "mode": "full",
            "total_files": total_files,
            "entries": docs,
        })
        .to_string());
    }
    // 根级文件（顶层，非目录）+ 顶层目录（各自子树文件计数）。
    let root_files: Vec<&FsEntry> = files
        .iter()
        .filter(|e| !e.rel_path.contains('/'))
        .copied()
        .collect();
    let top_level_dirs: Vec<serde_json::Value> = docs
        .iter()
        .filter(|e| e.is_dir && !e.rel_path.contains('/'))
        .map(|dir| {
            let prefix = format!("{}/", dir.rel_path);
            let sub_files: Vec<&FsEntry> = files
                .iter()
                .filter(|e| e.rel_path.starts_with(&prefix))
                .copied()
                .collect();
            serde_json::json!({
                "rel_path": dir.rel_path,
                "file_count": sub_files.len(),
                "docx_count": sub_files.iter().filter(|e| e.doc_type == "docx").count(),
            })
        })
        .collect();
    Ok(serde_json::json!({
        "mode": "overview",
        "total_files": total_files,
        "root_files": root_files,
        "top_level_dirs": top_level_dirs,
    })
    .to_string())
}

/// 目录一层展开（AG-WS-LAYERED-01 L1）：返回 rel_dir 的直接子项（不递归，不含正文）。
/// rel_dir 为空 = 根级直接子项；不存在或为文件 → 明确 IpcError 回报。
pub fn list_docs_children_json(root: &Path, rel_dir: &str) -> Result<String, IpcError> {
    let rel_dir = rel_dir.trim().trim_matches('/');
    let dir_path = root.join(rel_dir);
    let meta = std::fs::metadata(&dir_path).map_err(|_| {
        IpcError::new(
            "WS_LIST_DIR_MISSING",
            &format!(
                "目录不存在：{rel_dir}；请先用 list_docs() 查看概览或 list_docs(path=...) 逐层展开"
            ),
            false,
            false,
        )
    })?;
    if !meta.is_dir() {
        return Err(IpcError::new(
            "WS_LIST_DIR_NOT_DIR",
            &format!("{rel_dir} 不是目录，不能展开；内容请用 read_doc 读取"),
            false,
            false,
        ));
    }
    let docs = list_docs(root)?;
    let prefix = if rel_dir.is_empty() {
        String::new()
    } else {
        format!("{}/", rel_dir)
    };
    let is_direct_child = |rel: &str| -> bool {
        if prefix.is_empty() {
            !rel.contains('/')
        } else {
            rel.strip_prefix(&prefix)
                .map(|rest| !rest.contains('/'))
                .unwrap_or(false)
        }
    };
    let items: Vec<serde_json::Value> = docs
        .iter()
        .filter(|e| is_direct_child(&e.rel_path))
        .map(|e| {
            if e.is_dir {
                let sub = format!("{}/", e.rel_path);
                let file_count = docs
                    .iter()
                    .filter(|x| !x.is_dir && x.rel_path.starts_with(&sub))
                    .count();
                serde_json::json!({
                    "rel_path": e.rel_path,
                    "is_dir": true,
                    "file_count": file_count,
                })
            } else {
                serde_json::json!({
                    "rel_path": e.rel_path,
                    "doc_type": e.doc_type,
                    "is_bdocx": e.is_bdocx,
                })
            }
        })
        .collect();
    Ok(serde_json::json!({
        "mode": "children",
        "path": rel_dir,
        "total_items": items.len(),
        "items": items,
    })
    .to_string())
}

/// AG-STRATEGY-01：按文件名通配模式定位文档（* 匹配任意字符序列含 `/`，? 匹配单个字符）。
/// 只返回文件（非目录），限 workspace 内（复用 list_docs 全量枚举，隐藏项已跳过）。
pub fn glob_files(root: &Path, pattern: &str) -> Result<Vec<FsEntry>, IpcError> {
    let docs = list_docs(root)?;
    Ok(docs
        .into_iter()
        .filter(|e| !e.is_dir && wildcard_match(pattern, &e.rel_path))
        .collect())
}

/// 简单通配匹配（DP）：`*` 匹配任意字符序列（含空与 `/`），`?` 匹配单个字符。
fn wildcard_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let mut dp = vec![vec![false; t.len() + 1]; p.len() + 1];
    dp[0][0] = true;
    for i in 1..=p.len() {
        if p[i - 1] == '*' {
            dp[i][0] = dp[i - 1][0];
        }
    }
    for i in 1..=p.len() {
        for j in 1..=t.len() {
            dp[i][j] = match p[i - 1] {
                '*' => dp[i - 1][j] || dp[i][j - 1],
                '?' => dp[i - 1][j - 1],
                c => c == t[j - 1] && dp[i - 1][j - 1],
            };
        }
    }
    dp[p.len()][t.len()]
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

const PROJECT_IMAGE_MAX_ITEMS: usize = 500;
const PROJECT_IMAGE_CATALOG_PATH: &str = ".binder/project_assets.json";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectImageCatalog {
    pub image_roots: Vec<String>,
    pub candidates: Vec<String>,
    pub config_source: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProjectImageCatalogConfig {
    version: u8,
    image_roots: Vec<String>,
}

fn safe_catalog_root(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('/')
        && !value.starts_with('\\')
        && !value.contains('\\')
        && !value.contains("://")
        && !value
            .split('/')
            .any(|component| component.is_empty() || component == "." || component == "..")
}

fn configured_project_image_roots(root: &Path) -> Result<(Vec<String>, String), IpcError> {
    let config_path = root.join(PROJECT_IMAGE_CATALOG_PATH);
    if !config_path.exists() {
        return Ok((vec!["images".to_string()], "default".to_string()));
    }
    let binder_dir = root.join(".binder");
    if std::fs::symlink_metadata(&binder_dir)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err(IpcError::new(
            "WS_PROJECT_IMAGE_CONFIG_INVALID",
            "项目图片配置目录不能是符号链接",
            false,
            false,
        ));
    }
    let metadata = std::fs::symlink_metadata(&config_path).map_err(|error| {
        IpcError::new(
            "WS_PROJECT_IMAGE_CONFIG_INVALID",
            &format!("读取项目图片配置失败: {error}"),
            true,
            false,
        )
    })?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(IpcError::new(
            "WS_PROJECT_IMAGE_CONFIG_INVALID",
            "项目图片配置必须是 workspace 内普通文件",
            false,
            false,
        ));
    }
    let bytes = std::fs::read(&config_path).map_err(|error| {
        IpcError::new(
            "WS_PROJECT_IMAGE_CONFIG_INVALID",
            &format!("读取项目图片配置失败: {error}"),
            true,
            false,
        )
    })?;
    let config: ProjectImageCatalogConfig = serde_json::from_slice(&bytes).map_err(|error| {
        IpcError::new(
            "WS_PROJECT_IMAGE_CONFIG_INVALID",
            &format!("项目图片配置 JSON 无效: {error}"),
            false,
            false,
        )
    })?;
    if config.version != 1
        || config.image_roots.is_empty()
        || config.image_roots.len() > 16
        || config
            .image_roots
            .iter()
            .any(|value| !safe_catalog_root(value))
    {
        return Err(IpcError::new(
            "WS_PROJECT_IMAGE_CONFIG_INVALID",
            "项目图片配置要求 version=1 且 imageRoots 为 1..=16 个安全相对目录",
            false,
            false,
        ));
    }
    let mut roots = config.image_roots;
    roots.sort();
    roots.dedup();
    Ok((roots, PROJECT_IMAGE_CATALOG_PATH.to_string()))
}

pub fn project_image_roots(root: &Path) -> Result<Vec<String>, IpcError> {
    configured_project_image_roots(root).map(|(roots, _)| roots)
}

fn is_project_image_candidate(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg"
            )
        })
}

fn catalog_root_contains_symlink(root: &Path, relative: &str) -> Result<bool, IpcError> {
    let mut current = root.to_path_buf();
    for component in relative.split('/') {
        current.push(component);
        if !current.exists() {
            return Ok(false);
        }
        let metadata = std::fs::symlink_metadata(&current).map_err(|error| {
            IpcError::new(
                "WS_PROJECT_IMAGE_ROOT_INVALID",
                &format!("读取项目图片根失败 {relative}: {error}"),
                true,
                false,
            )
        })?;
        if metadata.file_type().is_symlink() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn walk_project_images(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<(), IpcError> {
    if out.len() >= PROJECT_IMAGE_MAX_ITEMS {
        return Ok(());
    }
    let entries = std::fs::read_dir(dir).map_err(|error| {
        IpcError::new(
            "WS_LIST_READ_FAILED",
            &format!("读取项目图片目录失败: {error}"),
            true,
            false,
        )
    })?;
    // read_dir 顺序不稳定；本查询有 500 项上限，必须先排序再截断，避免同一 workspace
    // 在不同回合向模型暴露不同的项目图片子集。
    let mut entries = entries.flatten().collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        if out.len() >= PROJECT_IMAGE_MAX_ITEMS {
            break;
        }
        let name = entry.file_name();
        if is_hidden(&name.to_string_lossy()) {
            continue;
        }
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            walk_project_images(root, &path, out)?;
        } else if file_type.is_file() && is_project_image_candidate(&path) {
            if let Ok(relative) = path.strip_prefix(root) {
                out.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    Ok(())
}

/**
 * @GOV
 * codes:    BR-WS-DATA-001-QUERY-WS-WS-FS-006
 * type:     QUERY
 * chain:    WS-FS
 * rules:    BR-WS-DATA-001
 * term_ref: TERM-WS-001
 * boundary: in=workspaceRoot | out=ProjectImageCandidateRelativePaths(hiddenAndSymlinksSkippedCapped)
 */
/// 只读枚举 workspace 内 PNG/JPEG 候选相对路径。WS 只负责物理边界；文件 magic 与 SHA-256
/// 由 DOC 图片资源层校验。隐藏目录、符号链接与非普通文件不会进入结果。
pub fn load_project_image_catalog(root: &Path) -> Result<ProjectImageCatalog, IpcError> {
    let (image_roots, config_source) = configured_project_image_roots(root)?;
    let mut out = Vec::new();
    for image_root in &image_roots {
        let directory = root.join(image_root);
        if catalog_root_contains_symlink(root, image_root)? {
            return Err(IpcError::new(
                "WS_PROJECT_IMAGE_ROOT_INVALID",
                &format!("项目图片根路径不能经过符号链接: {image_root}"),
                false,
                false,
            ));
        }
        if !directory.exists() {
            continue;
        }
        let metadata = std::fs::symlink_metadata(&directory).map_err(|error| {
            IpcError::new(
                "WS_PROJECT_IMAGE_ROOT_INVALID",
                &format!("读取项目图片根失败 {image_root}: {error}"),
                true,
                false,
            )
        })?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(IpcError::new(
                "WS_PROJECT_IMAGE_ROOT_INVALID",
                &format!("项目图片根必须是普通目录且不能是符号链接: {image_root}"),
                false,
                false,
            ));
        }
        walk_project_images(root, &directory, &mut out)?;
    }
    out.sort();
    out.dedup();
    Ok(ProjectImageCatalog {
        image_roots,
        candidates: out,
        config_source,
    })
}

pub fn list_project_image_candidates(root: &Path) -> Result<Vec<String>, IpcError> {
    Ok(load_project_image_catalog(root)?.candidates)
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

    // AG-WS-LAYERED-01：小工作区（≤30 文件且 ≤4KB）豁免——list_docs() 返回全量扁平列表（mode=full）。
    #[test]
    fn overview_small_workspace_returns_full() {
        let tmp = std::env::temp_dir().join(format!("binder_ov_small_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("a_dir")).unwrap();
        fs::write(tmp.join("a.md"), b"x").unwrap();
        fs::write(tmp.join("a_dir/b.txt"), b"y").unwrap();
        let json = list_docs_overview_json(&tmp).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["mode"], "full");
        assert_eq!(value["total_files"], 2);
        assert_eq!(value["entries"].as_array().unwrap().len(), 3); // a.md + a_dir + a_dir/b.txt
        let _ = fs::remove_dir_all(&tmp);
    }

    // AG-WS-LAYERED-01：大工作区（>30 文件）list_docs() 返回概览——顶层+计数，不含正文。
    #[test]
    fn overview_large_workspace_returns_summary() {
        let tmp = std::env::temp_dir().join(format!("binder_ov_large_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        // 35 个文件（>30 豁免阈值）触发 overview。
        fs::create_dir_all(tmp.join("docs")).unwrap();
        for i in 0..30 {
            fs::write(tmp.join(format!("docs/f{i}.txt")), b"content").unwrap();
        }
        fs::write(tmp.join("root.md"), b"root").unwrap();
        for i in 0..5 {
            fs::write(tmp.join(format!("r{i}.txt")), b"x").unwrap();
        }
        let json = list_docs_overview_json(&tmp).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["mode"], "overview");
        assert_eq!(value["total_files"], 36);
        // 顶层目录 docs 带 file_count=30；根级文件包含 root.md 与 r*.txt。
        let dirs = value["top_level_dirs"].as_array().unwrap();
        let docs_dir = dirs.iter().find(|d| d["rel_path"] == "docs").unwrap();
        assert_eq!(docs_dir["file_count"], 30);
        let roots = value["root_files"].as_array().unwrap();
        assert!(roots.iter().any(|f| f["rel_path"] == "root.md"));
        // 概览不含正文内容（无 entries 数组）。
        assert!(value.get("entries").is_none());
        let _ = fs::remove_dir_all(&tmp);
    }

    // AG-WS-LAYERED-01：list_docs(path=...) 返回一层直接子项（不递归）；目录带 file_count。
    #[test]
    fn children_returns_direct_children_only() {
        let tmp = std::env::temp_dir().join(format!("binder_ch_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("top/sub/deep")).unwrap();
        fs::write(tmp.join("top/a.md"), b"a").unwrap();
        fs::write(tmp.join("top/sub/b.md"), b"b").unwrap();
        fs::write(tmp.join("top/sub/deep/c.md"), b"c").unwrap();
        let json = list_docs_children_json(&tmp, "top").unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["mode"], "children");
        assert_eq!(value["total_items"], 2); // top/a.md + top/sub（一层，不含 deep）
        let items = value["items"].as_array().unwrap();
        assert!(items.iter().any(|i| i["rel_path"] == "top/a.md"));
        let sub = items.iter().find(|i| i["rel_path"] == "top/sub").unwrap();
        assert_eq!(sub["is_dir"], true);
        assert_eq!(sub["file_count"], 2); // b.md + c.md 子树
                                          // 根级（空 path 语义在 tool_exec 处理；此处验证目录一层不递归）不包含 deep。
        assert!(!items.iter().any(|i| i["rel_path"] == "top/sub/deep"));
        let _ = fs::remove_dir_all(&tmp);
    }

    // AG-WS-LAYERED-01：path 不存在或为文件 → 明确错误回报。
    #[test]
    fn children_rejects_missing_or_file_path() {
        let tmp = std::env::temp_dir().join(format!("binder_ch_err_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join("a.md"), b"a").unwrap();
        let missing = list_docs_children_json(&tmp, "no_such_dir").unwrap_err();
        assert_eq!(missing.code, "WS_LIST_DIR_MISSING");
        let not_dir = list_docs_children_json(&tmp, "a.md").unwrap_err();
        assert_eq!(not_dir.code, "WS_LIST_DIR_NOT_DIR");
        let _ = fs::remove_dir_all(&tmp);
    }

    // AG-STRATEGY-01：workspace_glob 按通配匹配文件名（* 含 `/`，? 单字符），只返回文件。
    #[test]
    fn glob_files_matches_wildcard_patterns() {
        let tmp = std::env::temp_dir().join(format!("binder_glob_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("寓言/寓言故事")).unwrap();
        fs::create_dir_all(tmp.join(".binder")).unwrap();
        fs::write(tmp.join("a.md"), b"x").unwrap();
        fs::write(tmp.join("寓言/小马.txt"), b"x").unwrap();
        fs::write(tmp.join("寓言/寓言故事/井底之光.txt"), b"x").unwrap();
        fs::write(tmp.join(".binder/secret.md"), b"x").unwrap();

        // 通配语义：* 匹配任意序列（含 /），? 匹配单字符。
        assert!(wildcard_match("*.md", "a.md"));
        assert!(wildcard_match("*寓言*", "文学作品/寓言故事/小马过河.txt"));
        assert!(wildcard_match("?.md", "a.md"));
        assert!(!wildcard_match("?.md", "ab.md"));

        let all_txt = glob_files(&tmp, "*.txt").unwrap();
        assert_eq!(
            all_txt.len(),
            2,
            "txt 文件应匹配 2 个（跳隐藏）: {all_txt:?}"
        );
        assert!(all_txt.iter().all(|e| !e.is_dir));
        let fable = glob_files(&tmp, "*寓言*").unwrap();
        assert!(fable.iter().any(|e| e.rel_path == "寓言/小马.txt"));
        assert!(fable
            .iter()
            .any(|e| e.rel_path == "寓言/寓言故事/井底之光.txt"));
        let _ = fs::remove_dir_all(&tmp);
    }

    // covers: BR-WS-DATA-001
    #[test]
    fn lists_project_image_candidates_without_hidden_or_symlink_entries() {
        let tmp =
            std::env::temp_dir().join(format!("binder_project_images_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("images")).unwrap();
        fs::create_dir_all(tmp.join("private")).unwrap();
        fs::create_dir_all(tmp.join(".hidden")).unwrap();
        fs::write(tmp.join("images/a.PNG"), b"png").unwrap();
        fs::write(tmp.join("images/b.jpeg"), b"jpeg").unwrap();
        fs::write(tmp.join("images/no.svg"), b"svg").unwrap();
        fs::write(tmp.join("private/not-exposed.png"), b"png").unwrap();
        fs::write(tmp.join(".hidden/secret.png"), b"png").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(tmp.join("images/a.PNG"), tmp.join("linked.png")).unwrap();

        assert_eq!(
            list_project_image_candidates(&tmp).unwrap(),
            vec!["images/a.PNG".to_string(), "images/b.jpeg".to_string()]
        );
        fs::create_dir_all(tmp.join(".binder")).unwrap();
        fs::write(
            tmp.join(".binder/project_assets.json"),
            br#"{"version":1,"imageRoots":["private"]}"#,
        )
        .unwrap();
        let catalog = load_project_image_catalog(&tmp).unwrap();
        assert_eq!(catalog.image_roots, vec!["private"]);
        assert_eq!(
            catalog.candidates,
            vec!["private/not-exposed.png".to_string()]
        );
        assert_eq!(catalog.config_source, ".binder/project_assets.json");
        fs::write(
            tmp.join(".binder/project_assets.json"),
            br#"{"version":1,"imageRoots":["../outside"]}"#,
        )
        .unwrap();
        assert_eq!(
            load_project_image_catalog(&tmp).unwrap_err().code,
            "WS_PROJECT_IMAGE_CONFIG_INVALID"
        );
        #[cfg(unix)]
        {
            fs::create_dir_all(tmp.join("outside")).unwrap();
            std::os::unix::fs::symlink(tmp.join("outside"), tmp.join("linked-root")).unwrap();
            fs::write(
                tmp.join(".binder/project_assets.json"),
                br#"{"version":1,"imageRoots":["linked-root/images"]}"#,
            )
            .unwrap();
            assert_eq!(
                load_project_image_catalog(&tmp).unwrap_err().code,
                "WS_PROJECT_IMAGE_ROOT_INVALID"
            );
        }
        let _ = fs::remove_dir_all(tmp);
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

        let _ = std::fs::remove_dir_all(&tmp);
    }

    // covers: BR-WS-DATA-001
    // 目录删除后清洗 bdocx_manifest：移除目录前缀与子目录路径，保留无关路径；无命中不写。
    #[test]
    fn cleanup_bdocx_under_removes_deleted_prefix_paths() {
        let tmp = std::env::temp_dir().join(format!("binder_bdocx_clean_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        mark_bdocx(&tmp, "notes/a.docx").unwrap();
        mark_bdocx(&tmp, "notes/sub/b.docx").unwrap();
        mark_bdocx(&tmp, "keep.docx").unwrap();
        cleanup_bdocx_under(&tmp, "notes").unwrap();
        assert!(!is_bdocx(&tmp, "notes/a.docx"));
        assert!(!is_bdocx(&tmp, "notes/sub/b.docx"));
        assert!(is_bdocx(&tmp, "keep.docx"));
        // 无命中：清单不变（cleanup 无关目录后 keep 仍注册）
        cleanup_bdocx_under(&tmp, "other").unwrap();
        assert!(is_bdocx(&tmp, "keep.docx"));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
