//! 从应用组合根迁入的模块运行时/路由；保持原 command 与状态机行为等价。

use crate::*;

/// 路由 `prv.preview`：docx/Excel/PPT 经 DOC 预览链出只读 PDF（缓存落盘，X-SEQ-004 产物复用），
/// 返回缓存文件绝对路径并逐文件授权 asset 协议（最小暴露，同 prv.media_src 纪律；守界 X-FORBID-004）。
/// 前端以 convertFileSrc 生成 asset URL 交由 iframe 加载，不再回传 PDF 字节（PRV-PREVIEW-OPT-01）。
pub(crate) fn route_preview(
    request: &CommandRequest,
    state: &AppState,
    app: &tauri::AppHandle,
) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let (_, abs) = resolve_active(state, &rel_path)?;
    let ext = std::path::Path::new(&rel_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let pdf_path = prv::render_preview(&abs, ext)?;
    app.asset_protocol_scope()
        .allow_file(&pdf_path)
        .map_err(|e| {
            IpcError::new(
                "PRV_SCOPE_FAILED",
                &format!("授权预览访问失败: {e}"),
                false,
                false,
            )
        })?;
    Ok(serde_json::json!({ "format": "pdf", "abs_path": pdf_path.to_string_lossy() }))
}

/// 路由 `prv.open_external`：以系统默认应用打开 workspace 内文件（只读查看，X-FORBID-004 守界）。
pub(crate) fn route_open_external(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let (_, abs) = resolve_active(state, &rel_path)?;
    prv::open_external(&abs)?;
    Ok(serde_json::json!({ "opened": true }))
}

/// 路由 `prv.media_src`：守界解析媒体文件绝对路径（X-FORBID-004）+ 仅把**该单个文件**加入 asset 协议
/// scope（最小暴露，不开放整目录）；前端 convertFileSrc(abs) 流式播放音视频（不进内存，只读）。
pub(crate) fn route_media_src(
    request: &CommandRequest,
    state: &AppState,
    app: &tauri::AppHandle,
) -> Result<serde_json::Value, IpcError> {
    use tauri::Manager;
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let (_, abs) = resolve_active(state, &rel_path)?;
    if !abs.is_file() {
        return Err(IpcError::new(
            "PRV_NOT_FOUND",
            "媒体文件不存在",
            false,
            false,
        ));
    }
    app.asset_protocol_scope().allow_file(&abs).map_err(|e| {
        IpcError::new(
            "PRV_SCOPE_FAILED",
            &format!("授权媒体访问失败: {e}"),
            false,
            false,
        )
    })?;
    Ok(serde_json::json!({ "abs_path": abs.to_string_lossy() }))
}

/// 路由 `prv.table_data`（2026-07-03 #1 表格式预览）：xlsx/xls → 结构化表格数据（DOC calamine 管线，
/// X-SEQ-004；只读值投影，BR-PRV-DATA-001）。守界（X-FORBID-004）；csv/tsv 为文本格式由前端解析不经此。
pub(crate) fn route_table_data(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let (_, abs) = resolve_active(state, &rel_path)?;
    if !abs.is_file() {
        return Err(IpcError::new(
            "PRV_NOT_FOUND",
            "表格文件不存在",
            false,
            false,
        ));
    }
    let (sheets, truncated) = doc::table_preview::xlsx_to_sheets(&abs)?;
    Ok(serde_json::json!({ "sheets": sheets, "truncated": truncated }))
}

/// 路由 `prv.export`：源文档导出为目标格式，落点 = 同目录同名换扩展（workspace 内，守界 X-FORBID-004）。
pub(crate) fn route_export(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let format = str_param(request, "format", "PRV_FORMAT_REQUIRED")?;
    let src_ext = std::path::Path::new(&rel_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string();
    // 落点：源 rel_path 去扩展 + 目标扩展（同目录）。
    let stem = rel_path
        .rsplit_once('.')
        .map(|(s, _)| s)
        .unwrap_or(&rel_path);
    let dest_rel = format!("{stem}.{format}");
    let (_, abs_src) = resolve_active(state, &rel_path)?;
    let (_, abs_dest) = resolve_active(state, &dest_rel)?;
    prv::export(&abs_src, &src_ext, &format, &abs_dest)?;
    Ok(serde_json::json!({ "rel_path": dest_rel }))
}
