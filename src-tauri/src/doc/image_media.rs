//! BDocx 嵌入图片的编辑媒体身份、资源校验与往返恢复报告。
//!
//! Pandoc 打开 DOCX 时把嵌入图片抽取为相对 `media/...` 引用。该相对引用才是编辑模型中的稳定
//! 身份；`asset://` 只是 WebView 的显示投影。保存时必须在同一 DOCX baseline 派生的目录解析这些
//! 引用，并在写盘前核对输出 OOXML 中的图片实例数，避免 Pandoc 以 alt 文本替代缺失资源却返回成功。

use crate::ipc::IpcError;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};
use zip::ZipArchive;

const EDIT_MEDIA_DIR_NAME: &str = "binder_edit_media";
const SNAPSHOT_BASELINE_PREFIX: &str = "\u{001e}binder-docx-snapshot-v1:";
const SNAPSHOT_BASELINE_END: char = '\u{001f}';
const PROJECT_IMAGE_MAX_BYTES: u64 = 50 * 1024 * 1024;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectImageAsset {
    pub asset_id: String,
    pub rel_path: String,
    pub content_hash: String,
    pub media_type: String,
    pub byte_size: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectImageMaterialization {
    pub media_ref: String,
    pub source: ProjectImageAsset,
}

/**
 * @GOV
 * codes:    BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-009, BR-DOC-DATA-003-DATA-DOC-DOC-CONVERT-009
 * type:     DATA
 * chain:    DOC-CONVERT
 * rules:    BR-DOC-DATA-002, BR-DOC-DATA-003
 * boundary: in=DocxImageOccurrencesResourcesAndDimensions | out=ImageRecoveryCountsAndMissingRelativeResources
 */
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ImageRecovery {
    pub detected_images: usize,
    pub restored_images: usize,
    pub unrecovered_images: usize,
    pub detected_resources: usize,
    pub restored_resources: usize,
    pub detected_sized_images: usize,
    pub restored_sized_images: usize,
    pub unrecovered_sized_images: usize,
    pub missing_resources: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HtmlImageResources {
    pub image_instances: usize,
    pub sized_images: usize,
    pub missing_image_instances: usize,
    pub relative_resources: Vec<String>,
    pub missing_resources: Vec<String>,
}

fn fail(code: &str, detail: impl AsRef<str>) -> IpcError {
    IpcError::new(
        code,
        &format!("BDocx 图片资源处理失败: {}", detail.as_ref()),
        true,
        false,
    )
}

/// 编辑媒体缓存只由来源 DOCX baseline 派生；前端不得传入绝对缓存路径。
pub fn edit_media_dir(baseline: u64) -> PathBuf {
    std::env::temp_dir()
        .join(EDIT_MEDIA_DIR_NAME)
        .join(format!("{baseline:x}"))
}

/// `ed_session_snapshots` 现有 schema 没有独立 baseline 列。仅在数据库 content 列内部使用短 envelope
/// 保存 BDocx baseline；IPC 返回前必须由 `decode_snapshot_content` 去除，不能进入编辑 LogicalState。
pub fn encode_snapshot_content(content: &str, baseline: Option<u64>) -> String {
    match baseline {
        Some(value) => {
            format!("{SNAPSHOT_BASELINE_PREFIX}{value}{SNAPSHOT_BASELINE_END}{content}")
        }
        None => content.to_string(),
    }
}

pub fn decode_snapshot_content(content: &str) -> (String, Option<u64>) {
    let Some(rest) = content.strip_prefix(SNAPSHOT_BASELINE_PREFIX) else {
        return (content.to_string(), None);
    };
    let Some(end) = rest.find(SNAPSHOT_BASELINE_END) else {
        return (content.to_string(), None);
    };
    let Some(baseline) = rest[..end].parse::<u64>().ok() else {
        return (content.to_string(), None);
    };
    (
        rest[end + SNAPSHOT_BASELINE_END.len_utf8()..].to_string(),
        Some(baseline),
    )
}

fn attribute(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let needle = format!("{}=", name.to_ascii_lowercase());
    let mut cursor = 0usize;
    while let Some(relative) = lower[cursor..].find(&needle) {
        let name_start = cursor + relative;
        if name_start > 0 {
            let before = lower.as_bytes()[name_start - 1];
            if !before.is_ascii_whitespace() {
                cursor = name_start + needle.len();
                continue;
            }
        }
        let quote_pos = name_start + needle.len();
        let quote = *tag.as_bytes().get(quote_pos)?;
        if quote != b'\'' && quote != b'"' {
            cursor = quote_pos + 1;
            continue;
        }
        let value_start = quote_pos + 1;
        let value_end = value_start + tag[value_start..].find(quote as char)?;
        return Some(tag[value_start..value_end].to_string());
    }
    None
}

fn image_tags(html: &str) -> Vec<&str> {
    let lower = html.to_ascii_lowercase();
    let mut tags = Vec::new();
    let mut cursor = 0usize;
    while let Some(relative) = lower[cursor..].find("<img") {
        let start = cursor + relative;
        let name_end = start + 4;
        if lower
            .as_bytes()
            .get(name_end)
            .is_some_and(|byte| !byte.is_ascii_whitespace() && *byte != b'>' && *byte != b'/')
        {
            cursor = name_end;
            continue;
        }
        let Some(end_relative) = lower[start..].find('>') else {
            break;
        };
        let end = start + end_relative + 1;
        tags.push(&html[start..end]);
        cursor = end;
    }
    tags
}

fn style_has_dimension(style: &str) -> bool {
    style.split(';').any(|property| {
        property.split_once(':').is_some_and(|(name, value)| {
            (name.trim().eq_ignore_ascii_case("width")
                || name.trim().eq_ignore_ascii_case("height"))
                && !value.trim().is_empty()
        })
    })
}

fn tag_has_dimension(tag: &str) -> bool {
    attribute(tag, "width")
        .or_else(|| attribute(tag, "height"))
        .is_some_and(|value| !value.trim().is_empty())
        || attribute(tag, "style").is_some_and(|style| style_has_dimension(&style))
}

fn is_remote_or_inline(src: &str) -> bool {
    let value = src.trim();
    value
        .get(..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("data:"))
        || value
            .get(..5)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http:"))
        || value
            .get(..6)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https:"))
}

fn safe_relative_path(src: &str) -> Result<PathBuf, IpcError> {
    let normalized = src.trim().replace('\\', "/");
    if normalized.is_empty()
        || normalized.starts_with('/')
        || normalized.starts_with("//")
        || normalized.contains('\0')
    {
        return Err(fail(
            "DOC_IMAGE_RESOURCE_INVALID",
            format!("图片引用不是安全相对路径: {src}"),
        ));
    }
    let path = PathBuf::from(&normalized);
    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
    {
        return Err(fail(
            "DOC_IMAGE_RESOURCE_INVALID",
            format!("图片引用越出编辑媒体目录: {src}"),
        ));
    }
    if normalized
        .split('/')
        .next()
        .is_some_and(|segment| segment.contains(':'))
    {
        return Err(fail(
            "DOC_IMAGE_RESOURCE_INVALID",
            format!("图片引用包含不允许的本地协议: {src}"),
        ));
    }
    Ok(path)
}

fn strict_project_relative_path(rel_path: &str) -> Result<PathBuf, IpcError> {
    let normalized = rel_path.trim().replace('\\', "/");
    if normalized != rel_path
        || normalized.is_empty()
        || normalized.starts_with('/')
        || normalized.contains('\0')
        || normalized.contains("://")
        || normalized
            .split('/')
            .any(|component| component.is_empty() || component == "." || component == "..")
        || normalized
            .as_bytes()
            .get(1)
            .is_some_and(|byte| *byte == b':' && normalized.as_bytes()[0].is_ascii_alphabetic())
    {
        return Err(fail(
            "DOC_PROJECT_IMAGE_INVALID",
            format!("项目图片路径必须是规范 workspace 相对路径: {rel_path}"),
        ));
    }
    Ok(PathBuf::from(normalized))
}

fn project_image_type(rel_path: &str, bytes: &[u8]) -> Option<(&'static str, &'static str)> {
    let extension = Path::new(rel_path)
        .extension()
        .and_then(|extension| extension.to_str())?
        .to_ascii_lowercase();
    match extension.as_str() {
        "png" if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) => {
            Some(("image/png", "png"))
        }
        "jpg" | "jpeg" if bytes.starts_with(&[0xff, 0xd8, 0xff]) => Some(("image/jpeg", "jpg")),
        _ => None,
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn read_project_image(
    root: &Path,
    rel_path: &str,
) -> Result<(ProjectImageAsset, Vec<u8>, &'static str), IpcError> {
    let relative = strict_project_relative_path(rel_path)?;
    let candidate = root.join(&relative);
    let metadata = std::fs::symlink_metadata(&candidate).map_err(|error| {
        fail(
            "DOC_PROJECT_IMAGE_INVALID",
            format!("无法读取项目图片 {rel_path}: {error}"),
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(fail(
            "DOC_PROJECT_IMAGE_INVALID",
            format!("项目图片必须是非符号链接普通文件: {rel_path}"),
        ));
    }
    if metadata.len() == 0 || metadata.len() > PROJECT_IMAGE_MAX_BYTES {
        return Err(fail(
            "DOC_PROJECT_IMAGE_INVALID",
            format!("项目图片大小不在 1..={PROJECT_IMAGE_MAX_BYTES} 字节: {rel_path}"),
        ));
    }
    let canonical_root = root.canonicalize().map_err(|error| {
        fail(
            "DOC_PROJECT_IMAGE_INVALID",
            format!("无法解析 workspace root: {error}"),
        )
    })?;
    let canonical_candidate = candidate.canonicalize().map_err(|error| {
        fail(
            "DOC_PROJECT_IMAGE_INVALID",
            format!("无法解析项目图片 {rel_path}: {error}"),
        )
    })?;
    if !canonical_candidate.starts_with(&canonical_root) {
        return Err(fail(
            "DOC_PROJECT_IMAGE_INVALID",
            format!("项目图片越出 workspace: {rel_path}"),
        ));
    }
    let bytes = std::fs::read(&canonical_candidate).map_err(|error| {
        fail(
            "DOC_PROJECT_IMAGE_INVALID",
            format!("读取项目图片 {rel_path} 失败: {error}"),
        )
    })?;
    let Some((media_type, canonical_extension)) = project_image_type(rel_path, &bytes) else {
        return Err(fail(
            "DOC_PROJECT_IMAGE_INVALID",
            format!("项目图片扩展名与 PNG/JPEG 文件 magic 不匹配: {rel_path}"),
        ));
    };
    Ok((
        ProjectImageAsset {
            asset_id: format!("asset_{}", &sha256_hex(rel_path.as_bytes())[..16]),
            rel_path: rel_path.to_string(),
            content_hash: sha256_hex(&bytes),
            media_type: media_type.to_string(),
            byte_size: bytes.len() as u64,
        },
        bytes,
        canonical_extension,
    ))
}

/**
 * @GOV
 * codes:    BR-DOC-DATA-002-GUARD-DOC-DOC-CONVERT-011, BR-WS-DATA-001-GUARD-DOC-DOC-CONVERT-011
 * type:     GUARD
 * chain:    DOC-CONVERT
 * rules:    BR-DOC-DATA-002, BR-WS-DATA-001
 * term_ref: TERM-ED-008, TERM-WS-001
 * boundary: in=WorkspaceRootAndProjectImageRelativePath | out=ValidatedProjectImageAsset(pathHashMagicWithinWorkspace)
 */
pub fn inspect_project_image(root: &Path, rel_path: &str) -> Result<ProjectImageAsset, IpcError> {
    read_project_image(root, rel_path).map(|(asset, _, _)| asset)
}

fn path_is_inside_catalog_root(rel_path: &Path, image_root: &str) -> bool {
    let image_root = Path::new(image_root);
    rel_path == image_root || rel_path.starts_with(image_root)
}

fn imported_image_name(source: &Path, content_hash: &str, extension: &str) -> String {
    let stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("imported");
    let safe_stem = stem
        .chars()
        .map(|character| {
            if character.is_control() || matches!(character, '/' | '\\' | ':') {
                '_'
            } else {
                character
            }
        })
        .collect::<String>();
    format!("{safe_stem}-{content_hash}.{extension}")
}

fn decode_base64_image_payload(encoded: &str) -> Result<Vec<u8>, IpcError> {
    let encoded = encoded.trim();
    let max_encoded_len = ((PROJECT_IMAGE_MAX_BYTES as usize + 2) / 3) * 4;
    if encoded.is_empty() || encoded.len() > max_encoded_len {
        return Err(fail(
            "DOC_PROJECT_IMAGE_IMPORT_INVALID",
            "剪贴板图片编码为空或超过大小上限",
        ));
    }
    if encoded.len() % 4 != 0 {
        return Err(fail(
            "DOC_PROJECT_IMAGE_IMPORT_INVALID",
            "剪贴板图片 base64 长度无效",
        ));
    }
    let mut decoded = Vec::with_capacity((encoded.len() / 4) * 3);
    let mut quartet = [0u8; 4];
    for (index, chunk) in encoded.as_bytes().chunks_exact(4).enumerate() {
        let is_last = index + 1 == encoded.len() / 4;
        let mut padding = 0usize;
        for (slot, byte) in chunk.iter().copied().enumerate() {
            quartet[slot] = match byte {
                b'A'..=b'Z' => byte - b'A',
                b'a'..=b'z' => byte - b'a' + 26,
                b'0'..=b'9' => byte - b'0' + 52,
                b'+' => 62,
                b'/' => 63,
                b'=' if is_last && slot >= 2 => {
                    padding += 1;
                    0
                }
                _ => {
                    return Err(fail(
                        "DOC_PROJECT_IMAGE_IMPORT_INVALID",
                        "剪贴板图片包含无效 base64 字符",
                    ))
                }
            };
        }
        if padding > 2
            || (padding == 1 && chunk[3] != b'=')
            || (padding == 2 && &chunk[2..] != b"==")
        {
            return Err(fail(
                "DOC_PROJECT_IMAGE_IMPORT_INVALID",
                "剪贴板图片 base64 padding 无效",
            ));
        }
        decoded.push((quartet[0] << 2) | (quartet[1] >> 4));
        if padding < 2 {
            decoded.push((quartet[1] << 4) | (quartet[2] >> 2));
        }
        if padding == 0 {
            decoded.push((quartet[2] << 6) | quartet[3]);
        }
        if decoded.len() > PROJECT_IMAGE_MAX_BYTES as usize {
            return Err(fail(
                "DOC_PROJECT_IMAGE_IMPORT_INVALID",
                "剪贴板图片超过大小上限",
            ));
        }
    }
    Ok(decoded)
}

fn import_external_project_image(
    root: &Path,
    source_name: &str,
    bytes: &[u8],
    image_roots: &[String],
) -> Result<ProjectImageAsset, IpcError> {
    if bytes.is_empty() || bytes.len() as u64 > PROJECT_IMAGE_MAX_BYTES {
        return Err(fail(
            "DOC_PROJECT_IMAGE_IMPORT_INVALID",
            format!("待导入图片大小不在 1..={PROJECT_IMAGE_MAX_BYTES} 字节"),
        ));
    }
    let Some((_media_type, extension)) = project_image_type(source_name, bytes) else {
        return Err(fail(
            "DOC_PROJECT_IMAGE_IMPORT_INVALID",
            "仅支持扩展名与文件 magic 匹配的 PNG/JPEG 图片",
        ));
    };
    let content_hash = sha256_hex(bytes);
    let managed_root = image_roots
        .first()
        .ok_or_else(|| fail("DOC_PROJECT_IMAGE_IMPORT_INVALID", "项目图片根配置为空"))?;
    let import_rel_dir = format!("{managed_root}/imported");
    let import_dir = root.join(&import_rel_dir);
    std::fs::create_dir_all(&import_dir).map_err(|error| {
        fail(
            "DOC_PROJECT_IMAGE_IMPORT_FAILED",
            format!("创建项目图片导入目录失败: {error}"),
        )
    })?;
    let import_metadata = std::fs::symlink_metadata(&import_dir).map_err(|error| {
        fail(
            "DOC_PROJECT_IMAGE_IMPORT_FAILED",
            format!("读取项目图片导入目录失败: {error}"),
        )
    })?;
    if import_metadata.file_type().is_symlink() || !import_metadata.is_dir() {
        return Err(fail(
            "DOC_PROJECT_IMAGE_IMPORT_INVALID",
            "项目图片导入目录必须是非符号链接普通目录",
        ));
    }
    let canonical_root = root.canonicalize().map_err(|error| {
        fail(
            "DOC_PROJECT_IMAGE_IMPORT_INVALID",
            format!("无法解析 workspace root: {error}"),
        )
    })?;
    let canonical_import_dir = import_dir.canonicalize().map_err(|error| {
        fail(
            "DOC_PROJECT_IMAGE_IMPORT_FAILED",
            format!("无法解析项目图片导入目录: {error}"),
        )
    })?;
    if !canonical_import_dir.starts_with(&canonical_root) {
        return Err(fail(
            "DOC_PROJECT_IMAGE_IMPORT_INVALID",
            "项目图片导入目录越出 workspace",
        ));
    }

    for entry in std::fs::read_dir(&import_dir)
        .map_err(|error| fail("DOC_PROJECT_IMAGE_IMPORT_FAILED", error.to_string()))?
        .flatten()
    {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if !file_type.is_file() || file_type.is_symlink() || metadata.len() != bytes.len() as u64 {
            continue;
        }
        let Ok(existing) = std::fs::read(entry.path()) else {
            continue;
        };
        if sha256_hex(&existing) == content_hash {
            let existing_path = entry.path();
            let relative = existing_path.strip_prefix(root).map_err(|_| {
                fail(
                    "DOC_PROJECT_IMAGE_IMPORT_INVALID",
                    "既有导入图片不在 workspace 内",
                )
            })?;
            return inspect_project_image(root, &relative.to_string_lossy().replace('\\', "/"));
        }
    }

    let file_name = imported_image_name(Path::new(source_name), &content_hash, extension);
    let destination = import_dir.join(file_name);
    if !destination.exists() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temporary =
            import_dir.join(format!(".binder-import-{}-{nonce}.tmp", std::process::id()));
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| fail("DOC_PROJECT_IMAGE_IMPORT_FAILED", error.to_string()))?;
        if let Err(error) = file.write_all(bytes).and_then(|_| file.sync_all()) {
            let _ = std::fs::remove_file(&temporary);
            return Err(fail("DOC_PROJECT_IMAGE_IMPORT_FAILED", error.to_string()));
        }
        if let Err(error) = std::fs::rename(&temporary, &destination) {
            let _ = std::fs::remove_file(&temporary);
            if !destination.exists() {
                return Err(fail("DOC_PROJECT_IMAGE_IMPORT_FAILED", error.to_string()));
            }
        }
    }
    let relative = destination.strip_prefix(root).map_err(|_| {
        fail(
            "DOC_PROJECT_IMAGE_IMPORT_INVALID",
            "导入图片目标不在 workspace 内",
        )
    })?;
    let asset = inspect_project_image(root, &relative.to_string_lossy().replace('\\', "/"))?;
    if asset.content_hash != content_hash {
        return Err(fail(
            "DOC_PROJECT_IMAGE_IMPORT_COLLISION",
            "导入目标已存在但内容身份不一致",
        ));
    }
    Ok(asset)
}

/**
 * @GOV
 * codes:    BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-015, BR-WS-DATA-001-IO-DOC-DOC-CONVERT-015
 * type:     IO
 * chain:    DOC-CONVERT
 * rules:    BR-DOC-DATA-002, BR-WS-DATA-001
 * term_ref: TERM-ED-008, TERM-WS-001
 * boundary: in=UserAuthorizedLocalImagePathAndWorkspaceImageRoots | out=ValidatedProjectImageAsset(reusedOrAtomicCopyUnderManagedImageRoot) | effect=filesystem-copy | does_not=move_delete_or_modify_source
 */
pub fn import_project_image(root: &Path, source: &Path) -> Result<ProjectImageAsset, IpcError> {
    let source_metadata = std::fs::symlink_metadata(source).map_err(|error| {
        fail(
            "DOC_PROJECT_IMAGE_IMPORT_INVALID",
            format!("无法读取待导入图片: {error}"),
        )
    })?;
    if source_metadata.file_type().is_symlink() || !source_metadata.is_file() {
        return Err(fail(
            "DOC_PROJECT_IMAGE_IMPORT_INVALID",
            "待导入图片必须是非符号链接普通文件",
        ));
    }
    if source_metadata.len() == 0 || source_metadata.len() > PROJECT_IMAGE_MAX_BYTES {
        return Err(fail(
            "DOC_PROJECT_IMAGE_IMPORT_INVALID",
            format!("待导入图片大小不在 1..={PROJECT_IMAGE_MAX_BYTES} 字节"),
        ));
    }

    let canonical_root = root.canonicalize().map_err(|error| {
        fail(
            "DOC_PROJECT_IMAGE_IMPORT_INVALID",
            format!("无法解析 workspace root: {error}"),
        )
    })?;
    let canonical_source = source.canonicalize().map_err(|error| {
        fail(
            "DOC_PROJECT_IMAGE_IMPORT_INVALID",
            format!("无法解析待导入图片: {error}"),
        )
    })?;
    let catalog = crate::workspace::listing::load_project_image_catalog(root)?;
    let image_roots = catalog.image_roots;
    if let Ok(relative) = canonical_source.strip_prefix(&canonical_root) {
        if image_roots
            .iter()
            .any(|image_root| path_is_inside_catalog_root(relative, image_root))
        {
            let rel_path = relative.to_string_lossy().replace('\\', "/");
            return inspect_project_image(root, &rel_path);
        }
    }

    let bytes = std::fs::read(&canonical_source).map_err(|error| {
        fail(
            "DOC_PROJECT_IMAGE_IMPORT_INVALID",
            format!("读取待导入图片失败: {error}"),
        )
    })?;
    let source_name = source
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("imported.png");
    import_external_project_image(root, source_name, &bytes, &image_roots)
}

/// 剪贴板图片没有可信绝对路径；只接受带明确 PNG/JPEG 媒体类型的 data URL，解码后仍复用
/// 与系统文件拖入完全相同的 magic、大小、哈希、去重和原子复制边界。
pub fn import_project_image_data_url(
    root: &Path,
    source_name: &str,
    data_url: &str,
) -> Result<ProjectImageAsset, IpcError> {
    let (header, encoded) = data_url.split_once(',').ok_or_else(|| {
        fail(
            "DOC_PROJECT_IMAGE_IMPORT_INVALID",
            "剪贴板图片 data URL 缺少数据分隔符",
        )
    })?;
    let media_type = header
        .strip_prefix("data:")
        .and_then(|value| value.strip_suffix(";base64"))
        .ok_or_else(|| {
            fail(
                "DOC_PROJECT_IMAGE_IMPORT_INVALID",
                "剪贴板图片必须使用 base64 data URL",
            )
        })?;
    if !matches!(
        media_type.to_ascii_lowercase().as_str(),
        "image/png" | "image/jpeg"
    ) {
        return Err(fail(
            "DOC_PROJECT_IMAGE_IMPORT_INVALID",
            "剪贴板仅支持 PNG/JPEG 图片",
        ));
    }
    let bytes = decode_base64_image_payload(encoded)?;
    let catalog = crate::workspace::listing::load_project_image_catalog(root)?;
    import_external_project_image(root, source_name, &bytes, &catalog.image_roots)
}

/**
 * @GOV
 * codes:    BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-012
 * type:     IO
 * chain:    DOC-CONVERT
 * rules:    BR-DOC-DATA-002
 * term_ref: TERM-ED-008
 * boundary: in=ValidatedWorkspaceProjectImageAndBDocxBaseline | out=StableBaselineMediaRef(sourceFileUnmodified)
 */
pub fn materialize_project_image(
    root: &Path,
    rel_path: &str,
    expected_hash: &str,
    baseline: u64,
) -> Result<ProjectImageMaterialization, IpcError> {
    let (source, bytes, canonical_extension) = read_project_image(root, rel_path)?;
    if source.content_hash != expected_hash {
        return Err(fail(
            "DOC_PROJECT_IMAGE_CHANGED",
            format!(
                "项目图片内容哈希已变化: {rel_path}，expected={expected_hash} actual={}",
                source.content_hash
            ),
        ));
    }
    let media_ref = format!(
        "media/binder-project-{}.{}",
        source.content_hash, canonical_extension
    );
    let destination = edit_media_dir(baseline).join(&media_ref);
    let parent = destination
        .parent()
        .ok_or_else(|| fail("DOC_PROJECT_IMAGE_INVALID", "无法解析项目图片媒体目标目录"))?;
    std::fs::create_dir_all(parent).map_err(|error| {
        fail(
            "DOC_PROJECT_IMAGE_MATERIALIZE_FAILED",
            format!("创建图片媒体目录失败: {error}"),
        )
    })?;
    let needs_write = std::fs::read(&destination)
        .map(|existing| sha256_hex(&existing) != source.content_hash)
        .unwrap_or(true);
    if needs_write {
        std::fs::write(&destination, &bytes).map_err(|error| {
            fail(
                "DOC_PROJECT_IMAGE_MATERIALIZE_FAILED",
                format!("物化项目图片失败: {error}"),
            )
        })?;
    }
    Ok(ProjectImageMaterialization { media_ref, source })
}

/// 对持久草稿 HTML 中带 workspace 来源/hash 的图片重新物化。若媒体缓存仍在，该操作为幂等校验；
/// 来源缺失、变化或稳定 mediaRef 漂移时明确失败。
pub fn materialize_project_images_for_html(
    root: &Path,
    baseline: u64,
    html: &str,
) -> Result<Vec<ProjectImageMaterialization>, IpcError> {
    let mut out = Vec::new();
    for tag in image_tags(html) {
        let source_path = attribute(tag, "data-binder-project-src");
        let source_hash = attribute(tag, "data-binder-project-hash");
        match (source_path, source_hash) {
            (None, None) => continue,
            (Some(source_path), Some(source_hash)) => {
                let materialized =
                    materialize_project_image(root, &source_path, &source_hash, baseline)?;
                let media_ref = attribute(tag, "data-binder-media-src")
                    .or_else(|| attribute(tag, "src"))
                    .ok_or_else(|| {
                        fail("DOC_PROJECT_IMAGE_INVALID", "项目图片节点缺稳定 mediaRef")
                    })?;
                if media_ref != materialized.media_ref {
                    return Err(fail(
                        "DOC_PROJECT_IMAGE_INVALID",
                        format!(
                            "项目图片稳定媒体引用漂移: expected={} actual={media_ref}",
                            materialized.media_ref
                        ),
                    ));
                }
                out.push(materialized);
            }
            _ => {
                return Err(fail(
                    "DOC_PROJECT_IMAGE_INVALID",
                    "项目图片来源路径与内容哈希必须同时存在",
                ))
            }
        }
    }
    Ok(out)
}

/**
 * @GOV
 * codes:    BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-013, BR-DOC-DATA-003-IO-DOC-DOC-CONVERT-013
 * type:     IO
 * chain:    DOC-CONVERT
 * rules:    BR-DOC-DATA-002, BR-DOC-DATA-003
 * term_ref: TERM-ED-008
 * boundary: in=SavedBDocxHtmlAndOldNewBaselines | out=ReferencedMediaCopiedToNewBaselineCache(noSourceMutation)
 */
pub fn carry_html_media_forward(
    html: &str,
    old_baseline: u64,
    new_baseline: u64,
) -> Result<(), IpcError> {
    if old_baseline == new_baseline {
        return Ok(());
    }
    let old_dir = edit_media_dir(old_baseline);
    let resources = inspect_html_resources(html, Some(&old_dir))?;
    if !resources.missing_resources.is_empty() {
        return Err(fail(
            "DOC_IMAGE_RESOURCE_MISSING",
            format!(
                "保存前媒体缓存缺失: {}",
                resources.missing_resources.join("、")
            ),
        ));
    }
    let new_dir = edit_media_dir(new_baseline);
    for resource in resources.relative_resources {
        let relative = safe_relative_path(&resource)?;
        let source = old_dir.join(&relative);
        let metadata = std::fs::symlink_metadata(&source).map_err(|error| {
            fail(
                "DOC_IMAGE_MEDIA_CARRY_FAILED",
                format!("读取旧 baseline 媒体 {resource} 失败: {error}"),
            )
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(fail(
                "DOC_IMAGE_MEDIA_CARRY_FAILED",
                format!("旧 baseline 媒体不是普通文件: {resource}"),
            ));
        }
        let destination = new_dir.join(&relative);
        let parent = destination.parent().ok_or_else(|| {
            fail(
                "DOC_IMAGE_MEDIA_CARRY_FAILED",
                format!("无法解析新 baseline 媒体目录: {resource}"),
            )
        })?;
        std::fs::create_dir_all(parent).map_err(|error| {
            fail(
                "DOC_IMAGE_MEDIA_CARRY_FAILED",
                format!("创建新 baseline 媒体目录失败: {error}"),
            )
        })?;
        std::fs::copy(&source, &destination).map_err(|error| {
            fail(
                "DOC_IMAGE_MEDIA_CARRY_FAILED",
                format!("复制媒体 {resource} 到新 baseline 失败: {error}"),
            )
        })?;
    }
    Ok(())
}

/// 检查编辑 HTML 的全部图片引用。远程/内联图片由 Pandoc 转换与最终 OOXML 计数继续校验；
/// 本地引用必须是受控媒体目录内存在的普通文件。
pub fn inspect_html_resources(
    html: &str,
    media_dir: Option<&Path>,
) -> Result<HtmlImageResources, IpcError> {
    let tags = image_tags(html);
    let mut relative_resources = BTreeSet::new();
    let mut missing_resources = BTreeSet::new();
    let mut missing_image_instances = 0usize;
    let sized_images = tags.iter().filter(|tag| tag_has_dimension(tag)).count();

    for tag in &tags {
        let src = attribute(tag, "src").ok_or_else(|| {
            fail(
                "DOC_IMAGE_RESOURCE_INVALID",
                "检测到缺少 src 的图片节点，不能安全保存",
            )
        })?;
        if is_remote_or_inline(&src) {
            continue;
        }
        let relative = safe_relative_path(&src)?;
        let relative_string = relative.to_string_lossy().replace('\\', "/");
        relative_resources.insert(relative_string.clone());
        let Some(base) = media_dir else {
            missing_resources.insert(relative_string);
            missing_image_instances += 1;
            continue;
        };
        let candidate = base.join(&relative);
        if !candidate.is_file() {
            missing_resources.insert(relative_string);
            missing_image_instances += 1;
            continue;
        }
        let canonical_base = base.canonicalize().map_err(|error| {
            fail(
                "DOC_IMAGE_RESOURCE_MISSING",
                format!("无法解析编辑媒体目录: {error}"),
            )
        })?;
        let canonical_candidate = candidate.canonicalize().map_err(|error| {
            fail(
                "DOC_IMAGE_RESOURCE_MISSING",
                format!("无法解析图片资源 {relative_string}: {error}"),
            )
        })?;
        if !canonical_candidate.starts_with(&canonical_base) {
            return Err(fail(
                "DOC_IMAGE_RESOURCE_INVALID",
                format!("图片资源越出编辑媒体目录: {relative_string}"),
            ));
        }
    }

    Ok(HtmlImageResources {
        image_instances: tags.len(),
        sized_images,
        missing_image_instances,
        relative_resources: relative_resources.into_iter().collect(),
        missing_resources: missing_resources.into_iter().collect(),
    })
}

fn read_document_xml(docx: &[u8]) -> Result<(String, usize), IpcError> {
    let mut archive = ZipArchive::new(Cursor::new(docx))
        .map_err(|error| fail("DOC_IMAGE_OOXML_INVALID", error.to_string()))?;
    let media_resources = (0..archive.len())
        .filter_map(|index| {
            archive
                .by_index(index)
                .ok()
                .map(|entry| entry.name().to_string())
        })
        .filter(|name| name.starts_with("word/media/") && !name.ends_with('/'))
        .count();
    let mut document_xml = String::new();
    archive
        .by_name("word/document.xml")
        .map_err(|error| fail("DOC_IMAGE_OOXML_INVALID", error.to_string()))?
        .read_to_string(&mut document_xml)
        .map_err(|error| fail("DOC_IMAGE_OOXML_INVALID", error.to_string()))?;
    Ok((document_xml, media_resources))
}

fn occurrences(haystack: &str, needle: &str) -> usize {
    haystack.match_indices(needle).count()
}

#[derive(Debug)]
struct DocxImageOccurrence {
    content_hash: String,
    width_emu: Option<u64>,
    height_emu: Option<u64>,
}

fn xml_elements<'a>(xml: &'a str, name: &str) -> Vec<&'a str> {
    let mut elements = Vec::new();
    let needle = format!("<{name}");
    let mut cursor = 0usize;
    while let Some(relative) = xml[cursor..].find(&needle) {
        let start = cursor + relative;
        let Some(end_relative) = xml[start..].find('>') else {
            break;
        };
        let end = start + end_relative + 1;
        elements.push(&xml[start..end]);
        cursor = end;
    }
    elements
}

fn docx_image_occurrences(docx: &[u8]) -> Result<Vec<DocxImageOccurrence>, IpcError> {
    let mut archive = ZipArchive::new(Cursor::new(docx))
        .map_err(|error| fail("DOC_IMAGE_OOXML_INVALID", error.to_string()))?;
    let mut document_xml = String::new();
    archive
        .by_name("word/document.xml")
        .map_err(|error| fail("DOC_IMAGE_OOXML_INVALID", error.to_string()))?
        .read_to_string(&mut document_xml)
        .map_err(|error| fail("DOC_IMAGE_OOXML_INVALID", error.to_string()))?;
    let mut rels_xml = String::new();
    archive
        .by_name("word/_rels/document.xml.rels")
        .map_err(|error| fail("DOC_IMAGE_OOXML_INVALID", error.to_string()))?
        .read_to_string(&mut rels_xml)
        .map_err(|error| fail("DOC_IMAGE_OOXML_INVALID", error.to_string()))?;
    let relationships = xml_elements(&rels_xml, "Relationship")
        .into_iter()
        .filter_map(|element| Some((attribute(element, "Id")?, attribute(element, "Target")?)))
        .collect::<BTreeMap<_, _>>();

    let mut result = Vec::new();
    let mut cursor = 0usize;
    while let Some(relative) = document_xml[cursor..].find("<w:drawing") {
        let start = cursor + relative;
        let Some(end_relative) = document_xml[start..].find("</w:drawing>") else {
            return Err(fail("DOC_IMAGE_OOXML_INVALID", "图片 drawing 节点未闭合"));
        };
        let end = start + end_relative + "</w:drawing>".len();
        let drawing = &document_xml[start..end];
        let relationship_id = attribute(drawing, "r:embed")
            .ok_or_else(|| fail("DOC_IMAGE_OOXML_INVALID", "图片 drawing 缺少 r:embed 关系"))?;
        let target = relationships.get(&relationship_id).ok_or_else(|| {
            fail(
                "DOC_IMAGE_OOXML_INVALID",
                format!("图片关系不存在: {relationship_id}"),
            )
        })?;
        let normalized_target = target.trim_start_matches("../");
        let media_name = format!("word/{normalized_target}");
        let mut media = Vec::new();
        archive
            .by_name(&media_name)
            .map_err(|error| {
                fail(
                    "DOC_IMAGE_OOXML_INVALID",
                    format!("图片关系目标不存在 {media_name}: {error}"),
                )
            })?
            .read_to_end(&mut media)
            .map_err(|error| fail("DOC_IMAGE_OOXML_INVALID", error.to_string()))?;
        let extent = xml_elements(drawing, "wp:extent").into_iter().next();
        result.push(DocxImageOccurrence {
            content_hash: sha256_hex(&media),
            width_emu: extent
                .and_then(|element| attribute(element, "cx"))
                .and_then(|value| value.parse().ok()),
            height_emu: extent
                .and_then(|element| attribute(element, "cy"))
                .and_then(|value| value.parse().ok()),
        });
        cursor = end;
    }
    Ok(result)
}

fn css_length_to_emu(value: &str, unitless_is_px: bool) -> Option<u64> {
    let value = value.trim().to_ascii_lowercase();
    let split = value
        .find(|character: char| !character.is_ascii_digit() && character != '.')
        .unwrap_or(value.len());
    let number = value[..split].parse::<f64>().ok()?;
    if !number.is_finite() || number <= 0.0 {
        return None;
    }
    let unit = &value[split..];
    let factor = match unit {
        "" if unitless_is_px => 9_525.0,
        "px" => 9_525.0,
        "in" => 914_400.0,
        "cm" => 360_000.0,
        "mm" => 36_000.0,
        "pt" => 12_700.0,
        "pc" => 152_400.0,
        _ => return None,
    };
    Some((number * factor).round() as u64)
}

fn style_dimension(tag: &str, name: &str) -> Option<u64> {
    let style = attribute(tag, "style")?;
    style.split(';').find_map(|property| {
        let (property_name, value) = property.split_once(':')?;
        property_name
            .trim()
            .eq_ignore_ascii_case(name)
            .then(|| css_length_to_emu(value, false))
            .flatten()
    })
}

fn expected_dimension(tag: &str, name: &str) -> Option<u64> {
    attribute(tag, name)
        .and_then(|value| css_length_to_emu(&value, true))
        .or_else(|| style_dimension(tag, name))
}

/// 从来源 OOXML 与同次 Pandoc HTML 生成图片实例、资源、尺寸三组恢复统计。
pub fn inspect_recovery(
    source_docx: &[u8],
    html: &str,
    media_dir: &Path,
) -> Result<ImageRecovery, IpcError> {
    let (document_xml, detected_resources) = read_document_xml(source_docx)?;
    let detected_images =
        occurrences(&document_xml, "<w:drawing") + occurrences(&document_xml, "<w:pict");
    let detected_sized_images = occurrences(&document_xml, "<wp:extent");
    let resources = inspect_html_resources(html, Some(media_dir))?;
    let restored_resources = resources
        .relative_resources
        .len()
        .saturating_sub(resources.missing_resources.len());
    let restored_images = resources
        .image_instances
        .saturating_sub(resources.missing_image_instances);
    Ok(ImageRecovery {
        detected_images,
        restored_images,
        unrecovered_images: detected_images.saturating_sub(restored_images),
        detected_resources,
        restored_resources,
        detected_sized_images,
        restored_sized_images: resources.sized_images,
        unrecovered_sized_images: detected_sized_images.saturating_sub(resources.sized_images),
        missing_resources: resources.missing_resources,
    })
}

/**
 * @GOV
 * codes:    BR-DOC-DATA-002-GUARD-DOC-DOC-CONVERT-014
 * type:     GUARD
 * chain:    DOC-CONVERT
 * rules:    BR-DOC-DATA-002
 * term_ref: TERM-ED-008
 * boundary: in=CanonicalBDocxHtmlBaselineMediaAndConvertedDocxBytes | out=ImageCountMediaOrderExtentVerifiedOrSaveRejection
 */
/// 保存转换完成后的最终校验：图片实例、媒体内容、顺序及显式尺寸必须与编辑 HTML 一致。
pub fn verify_docx_images(
    html: &str,
    output_docx: &[u8],
    media_dir: Option<&Path>,
) -> Result<(), IpcError> {
    let tags = image_tags(html);
    let expected = tags.len();
    if expected == 0 {
        return Ok(());
    }
    let (document_xml, media_resources) = read_document_xml(output_docx)?;
    let actual = occurrences(&document_xml, "<w:drawing") + occurrences(&document_xml, "<w:pict");
    if actual != expected || media_resources == 0 {
        return Err(fail(
            "DOC_IMAGE_ROUNDTRIP_LOSS",
            format!(
                "转换前有 {expected} 个图片实例，输出 DOCX 有 {actual} 个图片对象、{media_resources} 个嵌入资源；已拒绝覆盖原文档"
            ),
        ));
    }
    let output = docx_image_occurrences(output_docx)?;
    if output.len() != tags.len() {
        return Err(fail(
            "DOC_IMAGE_ROUNDTRIP_LOSS",
            "输出 DOCX 图片关系数量与编辑 HTML 不一致",
        ));
    }
    for (index, (tag, occurrence)) in tags.iter().zip(&output).enumerate() {
        let src = attribute(tag, "src").ok_or_else(|| {
            fail(
                "DOC_IMAGE_RESOURCE_INVALID",
                "检测到缺少 src 的图片节点，不能安全保存",
            )
        })?;
        let expected_hash = if let Some(hash) = attribute(tag, "data-binder-project-hash") {
            Some(hash)
        } else if is_remote_or_inline(&src) {
            None
        } else {
            let base = media_dir.ok_or_else(|| {
                fail(
                    "DOC_IMAGE_RESOURCE_MISSING",
                    format!("第 {} 张图片缺少媒体基准目录", index + 1),
                )
            })?;
            let relative = safe_relative_path(&src)?;
            let bytes = std::fs::read(base.join(relative)).map_err(|error| {
                fail(
                    "DOC_IMAGE_RESOURCE_MISSING",
                    format!("第 {} 张图片在最终校验时不可读: {error}", index + 1),
                )
            })?;
            Some(sha256_hex(&bytes))
        };
        if expected_hash.is_some_and(|hash| hash != occurrence.content_hash) {
            return Err(fail(
                "DOC_IMAGE_ROUNDTRIP_LOSS",
                format!("第 {} 张图片的嵌入媒体内容或文档流顺序不一致", index + 1),
            ));
        }
        for (name, expected_emu, actual_emu) in [
            (
                "宽度",
                expected_dimension(tag, "width"),
                occurrence.width_emu,
            ),
            (
                "高度",
                expected_dimension(tag, "height"),
                occurrence.height_emu,
            ),
        ] {
            if let Some(expected_emu) = expected_emu {
                let Some(actual_emu) = actual_emu else {
                    return Err(fail(
                        "DOC_IMAGE_ROUNDTRIP_LOSS",
                        format!("第 {} 张图片缺少 wp:extent {name}", index + 1),
                    ));
                };
                if expected_emu.abs_diff(actual_emu) > 100 {
                    return Err(fail(
                        "DOC_IMAGE_ROUNDTRIP_LOSS",
                        format!(
                            "第 {} 张图片{name}不一致: expected={expected_emu} actual={actual_emu}",
                            index + 1
                        ),
                    ));
                }
            }
        }
    }
    Ok(())
}

/// 递归授权编辑媒体缓存中的文件进入 asset 协议 scope。只开放文件，不开放目录。
pub(crate) fn authorize_media_files(
    app: &tauri::AppHandle,
    dir: &std::path::Path,
) -> Result<(), IpcError> {
    use tauri::Manager;
    let entries = std::fs::read_dir(dir).map_err(|error| {
        IpcError::new(
            "DOC_IMAGE_MEDIA_AUTHORIZE_FAILED",
            &format!("读取 BDocx 编辑媒体目录失败: {error}"),
            true,
            false,
        )
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            IpcError::new(
                "DOC_IMAGE_MEDIA_AUTHORIZE_FAILED",
                &format!("枚举 BDocx 编辑媒体失败: {error}"),
                true,
                false,
            )
        })?;
        let path = entry.path();
        if path.is_dir() {
            authorize_media_files(app, &path)?;
        } else if path.is_file() {
            app.asset_protocol_scope()
                .allow_file(&path)
                .map_err(|error| {
                    IpcError::new(
                        "DOC_IMAGE_MEDIA_AUTHORIZE_FAILED",
                        &format!("授权 BDocx 编辑媒体失败: {error}"),
                        true,
                        false,
                    )
                })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_dir_is_stable_for_baseline() {
        assert_eq!(edit_media_dir(0x1234), edit_media_dir(0x1234));
        assert!(edit_media_dir(0x1234).ends_with("binder_edit_media/1234"));
    }

    #[test]
    fn snapshot_baseline_envelope_is_internal_and_reversible() {
        let encoded = encode_snapshot_content("<p>draft</p>", Some(81));
        assert_eq!(
            decode_snapshot_content(&encoded),
            ("<p>draft</p>".to_string(), Some(81))
        );
        assert_eq!(
            decode_snapshot_content(&encode_snapshot_content(&encoded, Some(82))),
            (encoded, Some(82))
        );
    }

    #[test]
    fn rejects_absolute_parent_and_asset_references() {
        for src in [
            "/tmp/a.png",
            "../a.png",
            "media/../../a.png",
            "asset://a.png",
        ] {
            let html = format!(r#"<img src="{src}">"#);
            assert!(
                inspect_html_resources(&html, None).is_err(),
                "{src} 必须被拒绝"
            );
        }
    }

    #[test]
    fn reports_missing_relative_resource_and_dimensions() {
        let dir = std::env::temp_dir().join(format!(
            "binder_image_media_test_{}_{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("unnamed")
        ));
        std::fs::create_dir_all(dir.join("media")).expect("创建媒体测试目录");
        std::fs::write(dir.join("media/ok.png"), b"png").expect("写测试资源");
        let report = inspect_html_resources(
            r#"<img src="media/ok.png" style="width:1.25in"><img src="media/missing.png">"#,
            Some(&dir),
        )
        .expect("检查资源");
        assert_eq!(report.image_instances, 2);
        assert_eq!(report.sized_images, 1);
        assert_eq!(report.missing_image_instances, 1);
        assert_eq!(report.relative_resources.len(), 2);
        assert_eq!(report.missing_resources, vec!["media/missing.png"]);
        let _ = std::fs::remove_dir_all(dir);
    }

    // covers: BR-DOC-DATA-002, BR-WS-DATA-001
    #[test]
    fn validates_and_materializes_workspace_project_images_without_changing_source() {
        let root = std::env::temp_dir().join(format!(
            "binder_project_image_materialize_{}_{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("unnamed")
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("images")).unwrap();
        let png = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 1, 2, 3, 4];
        std::fs::write(root.join("images/cover.png"), png).unwrap();
        let before = std::fs::read(root.join("images/cover.png")).unwrap();
        let asset = inspect_project_image(&root, "images/cover.png").unwrap();
        assert_eq!(asset.media_type, "image/png");
        assert_eq!(asset.content_hash.len(), 64);

        let materialized =
            materialize_project_image(&root, "images/cover.png", &asset.content_hash, 0xa11)
                .unwrap();
        assert_eq!(
            materialized.media_ref,
            format!("media/binder-project-{}.png", asset.content_hash)
        );
        assert_eq!(
            std::fs::read(edit_media_dir(0xa11).join(&materialized.media_ref)).unwrap(),
            before
        );
        assert_eq!(
            std::fs::read(root.join("images/cover.png")).unwrap(),
            before,
            "物化不得改写项目源文件"
        );

        std::fs::remove_file(edit_media_dir(0xa11).join(&materialized.media_ref)).unwrap();
        let html = format!(
            r#"<p><img src="{0}" data-binder-media-src="{0}" data-binder-project-src="images/cover.png" data-binder-project-hash="{1}"></p>"#,
            materialized.media_ref, asset.content_hash
        );
        assert_eq!(
            materialize_project_images_for_html(&root, 0xa11, &html)
                .unwrap()
                .len(),
            1
        );
        assert!(edit_media_dir(0xa11)
            .join(&materialized.media_ref)
            .is_file());
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(edit_media_dir(0xa11));
    }

    // covers: BR-DOC-DATA-002, BR-WS-DATA-001
    #[test]
    fn rejects_project_image_escape_magic_mismatch_and_hash_change() {
        let root = std::env::temp_dir().join(format!(
            "binder_project_image_reject_{}_{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("unnamed")
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("images")).unwrap();
        std::fs::write(root.join("images/not.png"), b"not-png").unwrap();
        assert!(inspect_project_image(&root, "../outside.png").is_err());
        assert!(inspect_project_image(&root, "images/not.png").is_err());

        let jpeg = [0xff, 0xd8, 0xff, 1, 2, 3];
        std::fs::write(root.join("images/photo.jpg"), jpeg).unwrap();
        let asset = inspect_project_image(&root, "images/photo.jpg").unwrap();
        assert_eq!(asset.media_type, "image/jpeg");
        let error = materialize_project_image(&root, "images/photo.jpg", &"0".repeat(64), 0xa12)
            .unwrap_err();
        assert_eq!(error.code, "DOC_PROJECT_IMAGE_CHANGED");
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(edit_media_dir(0xa12));
    }

    // covers: BR-DOC-DATA-002, BR-WS-DATA-001
    #[test]
    fn imports_local_image_by_reuse_or_copy_without_changing_source() {
        assert_eq!(decode_base64_image_payload("AQ==").unwrap(), vec![1]);
        assert_eq!(decode_base64_image_payload("AQI=").unwrap(), vec![1, 2]);
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("binder_image_import_root_{unique}"));
        let external = std::env::temp_dir().join(format!("binder_image_import_external_{unique}"));
        std::fs::create_dir_all(root.join("images")).unwrap();
        std::fs::create_dir_all(&external).unwrap();
        let png = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 4, 3, 2, 1];
        let source = external.join("源图.png");
        std::fs::write(&source, png).unwrap();

        let imported = import_project_image(&root, &source).unwrap();
        assert!(imported.rel_path.starts_with("images/imported/"));
        assert_eq!(std::fs::read(&source).unwrap(), png);
        assert_eq!(std::fs::read(root.join(&imported.rel_path)).unwrap(), png);
        assert_eq!(
            import_project_image(&root, &source).unwrap().rel_path,
            imported.rel_path,
            "同内容重复导入必须复用稳定项目资产"
        );

        let controlled = root.join("images/already.jpg");
        let jpeg = [0xff, 0xd8, 0xff, 1, 2, 3];
        std::fs::write(&controlled, jpeg).unwrap();
        assert_eq!(
            import_project_image(&root, &controlled).unwrap().rel_path,
            "images/already.jpg"
        );

        let mismatch = external.join("fake.png");
        std::fs::write(&mismatch, b"not-an-image").unwrap();
        assert_eq!(
            import_project_image(&root, &mismatch).unwrap_err().code,
            "DOC_PROJECT_IMAGE_IMPORT_INVALID"
        );
        let clipboard = import_project_image_data_url(
            &root,
            "clipboard.png",
            "data:image/png;base64,iVBORw0KGgoEAwIB",
        )
        .unwrap();
        assert_eq!(
            clipboard.rel_path, imported.rel_path,
            "相同剪贴板内容必须复用既有受控资产"
        );
        assert_eq!(
            import_project_image_data_url(
                &root,
                "clipboard.png",
                "data:image/png;base64,not-base64",
            )
            .unwrap_err()
            .code,
            "DOC_PROJECT_IMAGE_IMPORT_INVALID"
        );
        assert_eq!(
            import_project_image_data_url(
                &root,
                "clipboard.png",
                "data:image/webp;base64,iVBORw0KGgoEAwIB",
            )
            .unwrap_err()
            .code,
            "DOC_PROJECT_IMAGE_IMPORT_INVALID"
        );
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(external);
    }

    // covers: BR-DOC-DATA-002, BR-DOC-DATA-003
    #[test]
    fn carries_only_referenced_media_to_the_new_saved_baseline() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        let old_baseline = unique;
        let new_baseline = unique.wrapping_add(1);
        let old_dir = edit_media_dir(old_baseline);
        std::fs::create_dir_all(old_dir.join("media")).unwrap();
        std::fs::write(old_dir.join("media/used.png"), b"used").unwrap();
        std::fs::write(old_dir.join("media/unused.png"), b"unused").unwrap();

        carry_html_media_forward(
            r#"<p><img src="media/used.png" data-binder-media-src="media/used.png"></p>"#,
            old_baseline,
            new_baseline,
        )
        .unwrap();

        assert_eq!(
            std::fs::read(edit_media_dir(new_baseline).join("media/used.png")).unwrap(),
            b"used"
        );
        assert!(!edit_media_dir(new_baseline)
            .join("media/unused.png")
            .exists());
        let _ = std::fs::remove_dir_all(old_dir);
        let _ = std::fs::remove_dir_all(edit_media_dir(new_baseline));
    }
}
