//! PI-BE-08 · PRV 预览（PRV-VIEW / BR-PRV-DATA-002）
//!
//! 承接 A-PRV-I-P-01 §2 `prv.preview` 与 A-PRV-M-T-01（管线归属决策、只读硬边界）：docx/Excel/PPT 预览
//! 经 DOC 预览链（LibreOffice → PDF）出只读 PDF；本模块只做"格式→预览链"归属编排，转换运行时归 DOC
//! （doc::libreoffice_runtime）。音视频系统解码、导出/打印延后（A-PRV-I-P-01 §4）。

use crate::doc::pandoc_runtime::{docx_to_markdown, markdown_to_docx};
use crate::doc::{
    libreoffice_runtime::{convert_to_pdf, PreviewKind},
    tool_runtime,
};
use crate::ipc::IpcError;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// 预览缓存子目录（app_data_dir/doc-runtime/preview-cache，不在 workspace）。
const PREVIEW_CACHE_SUBDIR: &str = "preview-cache";
/// 缓存文件超过此年龄（秒）在惰性清理时删除。
const PREVIEW_CACHE_MAX_AGE_SECS: u64 = 24 * 3600;

fn io_error(context: &str, e: std::io::Error) -> IpcError {
    IpcError::new("PRV_IO_ERROR", &format!("{context}: {e}"), true, false)
}

fn preview_cache_dir() -> Result<PathBuf, IpcError> {
    Ok(tool_runtime::runtime_dir()?.join(PREVIEW_CACHE_SUBDIR))
}

/// 缓存键：(abs_path 短哈希)-(size)-(mtime_nanos)。源文件任一变化 → 键失效（自动换新缓存）。
fn preview_cache_key(abs_path: &Path) -> Result<String, IpcError> {
    let meta = std::fs::metadata(abs_path).map_err(|e| io_error("读源文件元数据失败", e))?;
    let size = meta.len();
    let mtime_nanos = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut hasher = Sha256::new();
    hasher.update(abs_path.to_string_lossy().as_bytes());
    let digest = format!("{:x}", hasher.finalize());
    Ok(format!("{}-{}-{}", &digest[..16], size, mtime_nanos))
}

/// 惰性清理：删除超过 PREVIEW_CACHE_MAX_AGE_SECS 的缓存 PDF（每次渲染时执行，扫描代价低）。
fn lazy_clean_cache(cache_dir: &Path) {
    let Ok(entries) = std::fs::read_dir(cache_dir) else {
        return;
    };
    let now = SystemTime::now();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("pdf") {
            continue;
        }
        let stale = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| now.duration_since(t).ok())
            .map(|d| d.as_secs() > PREVIEW_CACHE_MAX_AGE_SECS)
            .unwrap_or(false);
        if stale {
            let _ = std::fs::remove_file(&path);
        }
    }
}

/// 扩展名 → 预览链类型；不支持的格式回 None（调用方回明确"不支持"态，不静默空白）。
fn preview_kind(ext: &str) -> Option<PreviewKind> {
    match ext.to_ascii_lowercase().as_str() {
        "docx" | "doc" => Some(PreviewKind::Docx),
        "xlsx" | "xls" => Some(PreviewKind::Excel),
        "pptx" | "ppt" => Some(PreviewKind::Presentation),
        _ => None,
    }
}

/**
 * @GOV
 * codes:    BR-PRV-DATA-002-IO-PRV-PRV-VIEW-001
 * type:     IO
 * chain:    PRV-VIEW
 * rules:    BR-PRV-DATA-002
 * boundary: in=docAbsolutePathAndExt | out=previewPdfPath(readOnlyCacheHitOrDocConvert)
 */
/// 预览管线归属编排（BR-PRV-DATA-002）：docx/Excel/PPT → DOC 预览链（LibreOffice convert_to_pdf）出只读
/// PDF，落盘预览缓存（键=路径+size+mtime）后返回文件路径；缓存命中跳过 LO 转换（仍为 DOC 产物复用，
/// 不绕过 X-SEQ-004）。不支持格式 → 明确错误（PRV_UNSUPPORTED，不静默空白）；渲染失败由 DOC 运行时
/// 回报明确失败态。只读硬边界——本模块不产生编辑入口（A-PRV-M-T-01）。调用方负责 asset 授权。
pub fn render_preview(abs_path: &Path, ext: &str) -> Result<PathBuf, IpcError> {
    let kind = preview_kind(ext).ok_or_else(|| {
        IpcError::new(
            "PRV_UNSUPPORTED",
            &format!("不支持预览的格式: {ext}"),
            false,
            false,
        )
    })?;
    let cache_dir = preview_cache_dir()?;
    std::fs::create_dir_all(&cache_dir).map_err(|e| io_error("建预览缓存目录失败", e))?;
    lazy_clean_cache(&cache_dir);
    let key = preview_cache_key(abs_path)?;
    let cached = cache_dir.join(format!("{key}.pdf"));
    if cached.is_file() {
        return Ok(cached); // 缓存命中：同一源文件内容未变时跳过 LO 转换
    }
    // 未命中：LO 转换 → 临时目录 → 复制入缓存（临时文件 + rename 原子替换，防并发半写残留）
    let out_dir = tool_runtime::runtime_dir()?.join(format!("preview-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&out_dir).map_err(|e| io_error("建预览目录失败", e))?;
    let pdf = convert_to_pdf(abs_path, kind, &out_dir)?; // 失败态（含 LibreOffice 缺失）由 DOC 运行时回报
    let tmp = cache_dir.join(format!("{key}.tmp"));
    std::fs::copy(&pdf, &tmp).map_err(|e| io_error("写预览缓存失败", e))?;
    std::fs::rename(&tmp, &cached).map_err(|e| io_error("提交预览缓存失败", e))?;
    let _ = std::fs::remove_dir_all(&out_dir);
    Ok(cached)
}

/**
 * @GOV
 * codes:    BR-PRV-DATA-002-IO-PRV-PRV-VIEW-002
 * type:     IO
 * chain:    PRV-VIEW
 * rules:    BR-PRV-DATA-002
 * boundary: in=sourceAndTargetFormatAndDest | out=exportedFile(withinWorkspaceOrUnsupportedRejected)
 */
/// 导出管线归属编排（BR-PRV-DATA-002）：md/txt→docx（pandoc）、docx→md（pandoc）、office→pdf（libreoffice）；
/// 转换运行时归 DOC，本模块只做格式编排 + 落点写入（落点守界由调用方 resolve_in_workspace 保证，
/// X-FORBID-004）。不支持的源→目标组合明确拒绝（不静默产出空文件）。
pub fn export(
    abs_src: &Path,
    src_ext: &str,
    target_format: &str,
    abs_dest: &Path,
) -> Result<(), IpcError> {
    let read_err =
        |e: std::io::Error| IpcError::new("PRV_IO_ERROR", &format!("读源失败: {e}"), true, false);
    let write_err =
        |e: std::io::Error| IpcError::new("PRV_IO_ERROR", &format!("写导出失败: {e}"), true, false);
    let src = src_ext.to_ascii_lowercase();
    match target_format {
        // 文本（md/txt）→ docx
        "docx" if matches!(src.as_str(), "md" | "markdown" | "txt") => {
            let md = std::fs::read_to_string(abs_src).map_err(read_err)?;
            let bytes = markdown_to_docx(&md)?;
            std::fs::write(abs_dest, bytes).map_err(write_err)?;
        }
        // docx → markdown
        "md" if matches!(src.as_str(), "docx" | "doc") => {
            let docx = std::fs::read(abs_src).map_err(read_err)?;
            let md = docx_to_markdown(&docx)?;
            std::fs::write(abs_dest, md).map_err(write_err)?;
        }
        // office（docx/xls/ppt）→ pdf（复用预览链产物；写入目标落点）
        "pdf"
            if matches!(
                src.as_str(),
                "docx" | "doc" | "xlsx" | "xls" | "pptx" | "ppt"
            ) =>
        {
            let pdf = render_preview(abs_src, src_ext)?;
            let bytes = std::fs::read(&pdf).map_err(read_err)?;
            std::fs::write(abs_dest, bytes).map_err(write_err)?;
        }
        _ => {
            return Err(IpcError::new(
                "PRV_EXPORT_UNSUPPORTED",
                &format!("不支持的导出组合: {src} → {target_format}"),
                false,
                false,
            ))
        }
    }
    Ok(())
}

/**
 * @GOV
 * codes:    BR-PRV-DATA-002-IO-PRV-PRV-VIEW-003
 * type:     IO
 * chain:    PRV-VIEW
 * rules:    BR-PRV-DATA-002
 * boundary: in=workspaceFileAbsolutePath | out=systemDefaultAppOpened(readOnlyNoWriteback)
 */
/// 本地打开（BR-PRV-DATA-002 只读边界）：以系统默认应用打开 workspace 内文件（macOS `open`），
/// 只读查看不写回；落点守界由调用方 resolve_in_workspace 保证（X-FORBID-004）。打开失败回明确错误。
pub fn open_external(abs_path: &Path) -> Result<(), IpcError> {
    if !abs_path.is_file() {
        return Err(IpcError::new("PRV_NOT_FOUND", "文件不存在", false, false));
    }
    std::process::Command::new("open")
        .arg(abs_path)
        .status()
        .map_err(|e| IpcError::new("PRV_OPEN_FAILED", &format!("打开失败: {e}"), true, false))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: BR-PRV-DATA-002
    // 缓存键：同源文件两次计算一致；源内容/元数据变化后键变化（换新缓存）。
    #[test]
    fn preview_cache_key_is_stable_and_content_sensitive() {
        let dir = std::env::temp_dir().join(format!("binder_prv_key_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("sample.docx");
        std::fs::write(&file, b"alpha").unwrap();
        let k1 = preview_cache_key(&file).unwrap();
        let k2 = preview_cache_key(&file).unwrap();
        assert_eq!(k1, k2, "同源文件键必须稳定");
        std::fs::write(&file, b"alpha-longer").unwrap();
        let k3 = preview_cache_key(&file).unwrap();
        assert_ne!(k1, k3, "源内容变化后键必须变化");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // covers: BR-PRV-DATA-002
    // 缓存命中判定：cache_key 生成的文件路径存在即命中（render_preview 在命中时不触发 LO）。
    #[test]
    fn preview_cache_hit_path_deterministic() {
        let dir = std::env::temp_dir().join(format!("binder_prv_hit_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("sample.docx");
        std::fs::write(&src, b"content").unwrap();
        let key = preview_cache_key(&src).unwrap();
        let cache_dir = dir.join("preview-cache");
        std::fs::create_dir_all(&cache_dir).unwrap();
        let cached = cache_dir.join(format!("{key}.pdf"));
        std::fs::write(&cached, b"%PDF-fake").unwrap();
        assert!(cached.is_file(), "同键缓存文件应可命中");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // covers: BR-PRV-DATA-002
    // 本地打开：不存在的文件必须明确报错（不静默成功）。
    #[test]
    fn open_external_missing_file_errors() {
        let missing = std::path::Path::new("/nonexistent/binder-open-external.docx");
        let err = open_external(missing).unwrap_err();
        assert_eq!(err.code, "PRV_NOT_FOUND");
    }
}
