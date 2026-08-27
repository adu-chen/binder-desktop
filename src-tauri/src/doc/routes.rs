//! 从应用组合根迁入的模块运行时/路由；保持原 command 与状态机行为等价。

use crate::*;

fn doc_type_of(rel_path: &str) -> String {
    std::path::Path::new(rel_path)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .unwrap_or_else(|| "bin".to_string())
}

fn is_docx_rel_path(rel_path: &str) -> bool {
    matches!(doc_type_of(rel_path).as_str(), "docx" | "doc")
}

/// 路由 `doc.create_draft`（旧版成熟流程）：原生 docx → 同目录草稿副本 `{stem}.draft.{ext}`（守界
/// X-FORBID-004）。原生 docx 点击为只读预览，"创建草稿进行编辑"后草稿直接进编辑模式（doc.open_for_edit）。
/// 返回草稿 rel_path。已存在草稿先覆盖。
pub(crate) fn route_create_draft(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let p = std::path::Path::new(&rel_path);
    let stem = p
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| IpcError::new("DOC_PATH_INVALID", "无法解析文件名", false, false))?;
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("docx");
    let parent = p
        .parent()
        .map(|x| x.to_string_lossy().to_string())
        .filter(|s| !s.is_empty());
    let draft_rel = match parent {
        Some(d) => format!("{d}/{stem}.draft.{ext}"),
        None => format!("{stem}.draft.{ext}"),
    };
    let (ws, abs_src) = resolve_active(state, &rel_path)?;
    let (_, abs_draft) = resolve_active(state, &draft_rel)?;
    if !abs_src.exists() {
        return Err(IpcError::new("DOC_NOT_FOUND", "原文档不存在", false, false));
    }
    let _ = std::fs::remove_file(&abs_draft); // 已存在草稿先覆盖
    std::fs::copy(&abs_src, &abs_draft).map_err(|e| {
        IpcError::new(
            "DOC_DRAFT_FAILED",
            &format!("创建草稿失败: {e}"),
            true,
            false,
        )
    })?;
    workspace::listing::mark_bdocx(std::path::Path::new(&ws.root_path), &draft_rel)?;
    Ok(serde_json::json!({ "rel_path": draft_rel }))
}

/// 广播 SM-DOC-01 docx 开/存机转移（rel_path 为实例，A-CORE-X-L-04 Task 2-DOC）。循环机无终态。
pub(crate) fn emit_doc01(
    app: &tauri::AppHandle,
    rel_path: &str,
    st: doc::convert::DocConvertState,
) {
    sm_events::emit_sm_state(
        app,
        sm_events::SmStatePayload::new("SM-DOC-01", rel_path, &format!("{st:?}"), false),
    );
}

/// 路由 `doc.open_for_edit`（A-DOC-I-P-01 §2 编辑链）：docx → 可编辑 HTML（DOC 编辑链 docx+styles +
/// preserve-styles.lua，保留样式子集供 TipTap）；回 baseline（docx 字节内容哈希）供保存冲突保护
/// （SM-WS-01）。转换失败回 convert_failed 不建会话（对接 SM-DOC-01 OpenError / ed loadFail）。
pub(crate) fn route_open_for_edit(
    request: &CommandRequest,
    state: &AppState,
    app: &tauri::AppHandle,
) -> Result<serde_json::Value, IpcError> {
    use doc::convert::{transition, DocConvertEvent, DocConvertState};
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let (ws, abs) = resolve_active(state, &rel_path)?;
    if is_docx_rel_path(&rel_path)
        && !workspace::listing::is_bdocx(std::path::Path::new(&ws.root_path), &rel_path)
    {
        return Err(IpcError::new(
            "DOC_NOT_BDOCX",
            "原生 docx 不能直接编辑，请先创建 b-docx",
            false,
            false,
        ));
    }
    // 加载基线：docx 原始字节内容哈希（供 save_from_edit 校验外部变更，BR-WS-OBS-001）。
    let baseline = std::fs::read(&abs)
        .map(|b| workspace::save::content_hash(&b))
        .map_err(|e| {
            IpcError::new(
                "DOC_READ_FAILED",
                &format!("读 docx 失败: {e}"),
                true,
                false,
            )
        })?;
    // ① 集成：开始打开转换 → OpenConverting（经 SM-DOC-01）。
    let opening = transition(DocConvertState::Idle, DocConvertEvent::OpenDocx)
        .expect("SM-DOC-01: Idle+OpenDocx→OpenConverting");
    emit_doc01(app, &rel_path, opening);
    // 编辑链：docx → HTML；extract-media 工作目录按文档隔离（图片解出供编辑器；缓存策略实现期）。
    let work_dir = doc::image_media::edit_media_dir(baseline);
    match doc::edit_chain::docx_to_edit_html(&abs, &work_dir) {
        Ok(edit_model) => {
            if let Err(error) = doc::image_media::authorize_media_files(app, &work_dir) {
                let st = transition(opening, DocConvertEvent::ConvertFail)
                    .expect("OpenConverting+ConvertFail→OpenError");
                emit_doc01(app, &rel_path, st);
                return Err(error);
            }
            let st = transition(opening, DocConvertEvent::ConvertOk)
                .expect("OpenConverting+ConvertOk→Ready");
            emit_doc01(app, &rel_path, st);
            // 图文呈现（BR-DOC-DATA-002 嵌入图片必需子集）：extract-media 把图片解到 work_dir（按 baseline
            // 隔离），HTML 内为相对 `media/…` 路径。逐文件授权入 asset 协议 scope（最小暴露，同 prv.media_src，
            // X-FORBID-004 守界由后端把关）；回 media_base 供前端将相对 <img src> 改写为 asset URL（只读呈现）。
            Ok(serde_json::json!({
                "format": "html",
                "html": edit_model.html,
                "baseline": baseline.to_string(),
                "media_base": work_dir.to_string_lossy(),
                "style_report": edit_model.style_report,
            }))
        }
        Err(e) => {
            // 转换失败：不建编辑会话，OpenError（原文件不受影响）。
            let st = transition(opening, DocConvertEvent::ConvertFail)
                .expect("OpenConverting+ConvertFail→OpenError");
            emit_doc01(app, &rel_path, st);
            Err(e)
        }
    }
}

/// 恢复持久化 BDocx LogicalState 的显示媒体：前端只回稳定 HTML 相对引用和原加载 baseline，
/// 后端据此授权同一媒体目录。缓存缺失时，只有磁盘仍与该 baseline 相同才允许重新抽取；磁盘已变化
/// 则保留保存冲突，不用新文档媒体替换旧草稿媒体。
pub(crate) fn route_restore_edit_media(
    request: &CommandRequest,
    state: &AppState,
    app: &tauri::AppHandle,
) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let html = request
        .params
        .get("html")
        .and_then(|value| value.as_str())
        .ok_or_else(|| IpcError::new("DOC_HTML_REQUIRED", "缺少 html 参数", false, false))?;
    let requested_baseline = optional_u64_param(request, "baseline", "DOC_BASELINE_INVALID")?;
    let (ws, abs) = resolve_active(state, &rel_path)?;
    if !is_docx_rel_path(&rel_path)
        || !workspace::listing::is_bdocx(std::path::Path::new(&ws.root_path), &rel_path)
    {
        return Err(IpcError::new(
            "DOC_NOT_BDOCX",
            "只有 BDocx 编辑会话可以恢复编辑媒体",
            false,
            false,
        ));
    }
    let current = std::fs::read(&abs).map_err(|error| {
        IpcError::new(
            "DOC_READ_FAILED",
            &format!("读 BDocx 失败: {error}"),
            true,
            false,
        )
    })?;
    let current_baseline = workspace::save::content_hash(&current);
    let baseline = requested_baseline.unwrap_or(current_baseline);
    let work_dir = doc::image_media::edit_media_dir(baseline);
    doc::image_media::materialize_project_images_for_html(
        std::path::Path::new(&ws.root_path),
        baseline,
        html,
    )?;
    let initial = doc::image_media::inspect_html_resources(html, Some(&work_dir))?;
    if !initial.missing_resources.is_empty() {
        if current_baseline != baseline {
            return Err(IpcError::new(
                "DOC_IMAGE_MEDIA_RESTORE_CONFLICT",
                &format!(
                    "旧编辑媒体已不存在，且磁盘 BDocx 已偏离会话基线；缺失资源：{}",
                    initial.missing_resources.join("、")
                ),
                false,
                false,
            ));
        }
        // 只借同基线源文档重建媒体目录；持久化草稿 HTML 仍是 LogicalState，不被转换结果替换。
        let _ = doc::edit_chain::docx_to_edit_html(&abs, &work_dir)?;
        // docx 重抽取可能重建 media 子目录；项目来源图片按持久化 path/hash 再物化一次。
        doc::image_media::materialize_project_images_for_html(
            std::path::Path::new(&ws.root_path),
            baseline,
            html,
        )?;
        let rebuilt = doc::image_media::inspect_html_resources(html, Some(&work_dir))?;
        if !rebuilt.missing_resources.is_empty() {
            return Err(IpcError::new(
                "DOC_IMAGE_RESOURCE_MISSING",
                &format!(
                    "重建同基线编辑媒体后仍缺失资源：{}",
                    rebuilt.missing_resources.join("、")
                ),
                true,
                false,
            ));
        }
    } else {
        std::fs::create_dir_all(&work_dir).map_err(|error| {
            IpcError::new(
                "DOC_IMAGE_MEDIA_RESTORE_FAILED",
                &format!("创建编辑媒体目录失败: {error}"),
                true,
                false,
            )
        })?;
    }
    doc::image_media::authorize_media_files(app, &work_dir)?;
    Ok(serde_json::json!({
        "baseline": baseline.to_string(),
        "media_base": work_dir.to_string_lossy(),
    }))
}

pub(crate) fn route_import_project_image(
    request: &CommandRequest,
    state: &AppState,
    app: &tauri::AppHandle,
) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let baseline =
        optional_u64_param(request, "baseline", "DOC_BASELINE_INVALID")?.ok_or_else(|| {
            IpcError::new(
                "DOC_BASELINE_REQUIRED",
                "导入 BDocx 图片必须携带当前编辑会话 baseline",
                false,
                false,
            )
        })?;
    let (workspace, _) = resolve_active(state, &rel_path)?;
    let workspace_root = std::path::Path::new(&workspace.root_path);
    if !is_docx_rel_path(&rel_path) || !workspace::listing::is_bdocx(workspace_root, &rel_path) {
        return Err(IpcError::new(
            "DOC_NOT_BDOCX",
            "只有 BDocx 编辑会话可以受控导入本地图片",
            false,
            false,
        ));
    }
    let source_path = request
        .params
        .get("sourcePath")
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty());
    let source_data_url = request
        .params
        .get("sourceDataUrl")
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty());
    let imported = match (source_path, source_data_url) {
        (Some(source_path), None) => {
            let source = std::path::Path::new(source_path);
            let resolved_source = if source.is_absolute() {
                source.to_path_buf()
            } else {
                if !source
                    .components()
                    .all(|component| matches!(component, std::path::Component::Normal(_)))
                {
                    return Err(IpcError::new(
                        "DOC_PROJECT_IMAGE_IMPORT_SOURCE_INVALID",
                        "项目文件树图片必须使用规范 workspace 相对路径",
                        false,
                        false,
                    ));
                }
                workspace_root.join(source)
            };
            doc::image_media::import_project_image(workspace_root, &resolved_source)?
        }
        (None, Some(source_data_url)) => {
            let source_name = request
                .params
                .get("sourceName")
                .and_then(|value| value.as_str())
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| {
                    IpcError::new(
                        "DOC_PROJECT_IMAGE_IMPORT_SOURCE_REQUIRED",
                        "剪贴板图片缺少文件名",
                        false,
                        false,
                    )
                })?;
            doc::image_media::import_project_image_data_url(
                workspace_root,
                source_name,
                source_data_url,
            )?
        }
        _ => {
            return Err(IpcError::new(
                "DOC_PROJECT_IMAGE_IMPORT_SOURCE_REQUIRED",
                "必须且只能提供 sourcePath 或 sourceDataUrl",
                false,
                false,
            ))
        }
    };
    let materialized = doc::image_media::materialize_project_image(
        workspace_root,
        &imported.rel_path,
        &imported.content_hash,
        baseline,
    )?;
    let media_base = doc::image_media::edit_media_dir(baseline);
    doc::image_media::authorize_media_files(app, &media_base)?;
    Ok(serde_json::json!({
        "assetId": imported.asset_id,
        "sourcePath": imported.rel_path,
        "sourceHash": imported.content_hash,
        "mediaType": imported.media_type,
        "byteSize": imported.byte_size,
        "mediaRef": materialized.media_ref,
        "mediaBase": media_base.to_string_lossy(),
        "baseline": baseline.to_string(),
    }))
}

/// 路由样式报告：复用与打开链完全相同的 DOCX→HTML 颜色恢复与实际检测，不返回正文或授权媒体。
pub(crate) fn route_get_style_report(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let (ws, abs) = resolve_active(state, &rel_path)?;
    if is_docx_rel_path(&rel_path)
        && !workspace::listing::is_bdocx(std::path::Path::new(&ws.root_path), &rel_path)
    {
        return Err(IpcError::new(
            "DOC_NOT_BDOCX",
            "原生 docx 不能进入可编辑样式报告，请先创建 b-docx",
            false,
            false,
        ));
    }
    let source = std::fs::read(&abs).map_err(|e| {
        IpcError::new(
            "DOC_READ_FAILED",
            &format!("读 docx 失败: {e}"),
            true,
            false,
        )
    })?;
    let work_dir = std::env::temp_dir()
        .join("binder_style_report")
        .join(format!("{:x}", workspace::save::content_hash(&source)));
    let edit_model = doc::edit_chain::docx_to_edit_html(&abs, &work_dir)?;
    Ok(serde_json::json!({ "style_report": edit_model.style_report }))
}

/// 路由 `doc.save_from_edit`（A-DOC-I-P-01 §2 编辑链）：编辑 HTML → docx（--reference-doc 统一样式）→
/// 交 WS 原子落盘（SM-WS-01，baseline 校验外部变更、原文件不变保护）。转换失败回 convert_failed
/// （SM-DOC-01 SaveError，原 docx 不变）。
pub(crate) fn route_save_from_edit(
    request: &CommandRequest,
    state: &AppState,
    app: &tauri::AppHandle,
) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let html = request
        .params
        .get("html")
        .and_then(|v| v.as_str())
        .ok_or_else(|| IpcError::new("DOC_HTML_REQUIRED", "缺少 html 参数", false, false))?;
    let baseline =
        optional_u64_param(request, "baseline", "DOC_BASELINE_INVALID")?.ok_or_else(|| {
            IpcError::new(
                "DOC_BASELINE_REQUIRED",
                "保存 BDocx 必须携带打开或持久恢复时的加载基线",
                false,
                false,
            )
        })?;
    let override_confirmed = bool_param(request, "override", false);
    let (ws, abs) = resolve_active(state, &rel_path)?;
    let saving = doc::convert::transition(
        doc::convert::DocConvertState::Ready,
        doc::convert::DocConvertEvent::SaveDocx,
    )
    .expect("SM-DOC-01: Ready+SaveDocx→SaveConverting");
    emit_doc01(app, &rel_path, saving);
    let media_dir = doc::image_media::edit_media_dir(baseline);
    if let Err(error) = doc::image_media::materialize_project_images_for_html(
        std::path::Path::new(&ws.root_path),
        baseline,
        html,
    ) {
        let st = doc::convert::transition(saving, doc::convert::DocConvertEvent::SaveConvertFail)
            .expect("SaveConverting+SaveConvertFail→SaveError");
        emit_doc01(app, &rel_path, st);
        return Err(error);
    }
    let docx_bytes = match doc::edit_chain::html_to_docx(html, Some(&media_dir)) {
        Ok(b) => b,
        Err(e) => {
            let st =
                doc::convert::transition(saving, doc::convert::DocConvertEvent::SaveConvertFail)
                    .expect("SaveConverting+SaveConvertFail→SaveError");
            emit_doc01(app, &rel_path, st);
            return Err(e);
        }
    };
    let next_baseline = workspace::save::content_hash(&docx_bytes);
    if let Err(error) = doc::image_media::carry_html_media_forward(html, baseline, next_baseline) {
        let st = doc::convert::transition(saving, doc::convert::DocConvertEvent::SaveConvertFail)
            .expect("SaveConverting+SaveConvertFail→SaveError");
        emit_doc01(app, &rel_path, st);
        return Err(error);
    }
    let next_media_dir = doc::image_media::edit_media_dir(next_baseline);
    if let Err(error) = std::fs::create_dir_all(&next_media_dir)
        .map_err(|cause| {
            IpcError::new(
                "DOC_IMAGE_MEDIA_CARRY_FAILED",
                &format!("创建新 baseline 编辑媒体目录失败: {cause}"),
                true,
                false,
            )
        })
        .and_then(|_| doc::image_media::authorize_media_files(app, &next_media_dir))
    {
        let st = doc::convert::transition(saving, doc::convert::DocConvertEvent::SaveConvertFail)
            .expect("SaveConverting+SaveConvertFail→SaveError");
        emit_doc01(app, &rel_path, st);
        return Err(error);
    }
    match save_atomic_sm(
        app,
        &rel_path,
        &abs,
        &docx_bytes,
        Some(baseline),
        override_confirmed,
    ) {
        Ok(saved) => {
            let st = doc::convert::transition(saving, doc::convert::DocConvertEvent::SaveConvertOk)
                .expect("SaveConverting+SaveConvertOk→Ready");
            emit_doc01(app, &rel_path, st);
            // 与文本保存路径相同：文件成功转换并写入后，效果态同步失败必须向调用方暴露，不能
            // 静默把历史留在 awaiting_save。先记时间轴 save 节点（BR-WS-DATA-002，内容变化才记、
            // 内容寻址幂等），再翻效果态。
            let conn = lock_db(state)?;
            let doc = crate::types::DocRef {
                workspace_id: ws.workspace_id,
                doc_type: doc_type_of(&rel_path),
                rel_path,
            };
            workspace::timeline::record_save_node(&conn, &doc, &docx_bytes).map_err(|e| {
                IpcError::new(
                    "WS_DB_ERROR",
                    &format!("文件已保存，但时间轴节点记录失败: {e}"),
                    true,
                    false,
                )
            })?;
            de::lifecycle::flip_awaiting_save_to_saved(&conn, &doc.workspace_id, &doc.rel_path)
                .map_err(|e| {
                    IpcError::new(
                        "DE_EFFECT_STATUS_SYNC_FAILED",
                        &format!("文件已保存，但 diff 效果状态同步失败: {e}"),
                        true,
                        false,
                    )
                })?;
            workspace::operation::mark_create_document_bundles_saved_for_path(
                &conn,
                &doc.workspace_id,
                &doc.rel_path,
            )
            .map_err(|error| {
                IpcError::new("WS_OPERATION_DB_ERROR", &error.to_string(), true, false)
            })?;
            Ok(serde_json::json!({
                "baseline": saved.baseline.to_string(),
                "media_base": next_media_dir.to_string_lossy(),
            }))
        }
        Err(e) => {
            let st =
                doc::convert::transition(saving, doc::convert::DocConvertEvent::SaveConvertFail)
                    .expect("SaveConverting+SaveConvertFail→SaveError");
            emit_doc01(app, &rel_path, st);
            Err(e)
        }
    }
}
