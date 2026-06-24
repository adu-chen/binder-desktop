//! PI-BE-08 · PRV 预览（PRV-VIEW / BR-PRV-DATA-002）
//!
//! 承接 A-PRV-I-P-01 §2 `prv.preview` 与 A-PRV-M-T-01（管线归属决策、只读硬边界）：docx/Excel/PPT 预览
//! 经 DOC 预览链（LibreOffice → PDF）出只读 PDF；本模块只做"格式→预览链"归属编排，转换运行时归 DOC
//! （doc::libreoffice_runtime）。音视频系统解码、导出/打印延后（A-PRV-I-P-01 §4）。

use crate::doc::libreoffice_runtime::{convert_to_pdf, PreviewKind};
use crate::doc::pandoc_runtime::{docx_to_markdown, markdown_to_docx};
use crate::ipc::IpcError;
use std::path::Path;

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
 * boundary: in=docAbsolutePathAndExt | out=previewPdfBytes(readOnlyOrUnsupportedExplicit)
 */
/// 预览管线归属编排（BR-PRV-DATA-002）：docx/Excel/PPT → DOC 预览链（LibreOffice convert_to_pdf）出只读
/// PDF 字节；不支持格式 → 明确错误（PRV_UNSUPPORTED，不静默空白）；渲染失败由 DOC 运行时回明确失败态。
/// 只读硬边界——本模块不产生编辑入口（A-PRV-M-T-01）。
pub fn render_preview(abs_path: &Path, ext: &str) -> Result<Vec<u8>, IpcError> {
    let kind = preview_kind(ext).ok_or_else(|| {
        IpcError::new("PRV_UNSUPPORTED", &format!("不支持预览的格式: {ext}"), false, false)
    })?;
    let out_dir = std::env::temp_dir().join(format!("binder_prv_{}", std::process::id()));
    std::fs::create_dir_all(&out_dir)
        .map_err(|e| IpcError::new("PRV_IO_ERROR", &format!("建预览目录失败: {e}"), true, false))?;
    let pdf = convert_to_pdf(abs_path, kind, &out_dir)?; // 失败态（含 LibreOffice 缺失）由 DOC 运行时回报
    let bytes = std::fs::read(&pdf)
        .map_err(|e| IpcError::new("PRV_IO_ERROR", &format!("读取预览 PDF 失败: {e}"), true, false))?;
    let _ = std::fs::remove_file(&pdf); // 预览产物即用即清（字节已回传）
    Ok(bytes)
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
pub fn export(abs_src: &Path, src_ext: &str, target_format: &str, abs_dest: &Path) -> Result<(), IpcError> {
    let read_err = |e: std::io::Error| IpcError::new("PRV_IO_ERROR", &format!("读源失败: {e}"), true, false);
    let write_err = |e: std::io::Error| IpcError::new("PRV_IO_ERROR", &format!("写导出失败: {e}"), true, false);
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
        // office（docx/xls/ppt）→ pdf（复用预览链字节）
        "pdf" if matches!(src.as_str(), "docx" | "doc" | "xlsx" | "xls" | "pptx" | "ppt") => {
            let bytes = render_preview(abs_src, src_ext)?;
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
