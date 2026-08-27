//! PI-DOC-01（实现期续接）· docx 编辑链 docx↔HTML · DOC-CONVERT / BR-DOC-DATA-001 / BR-DOC-DATA-002
//!
//! 移植旧项目 binder-project 的 DOC 编辑处理逻辑（同一处理逻辑）：编辑链产出**HTML**（非 markdown），
//! 因 TipTap/ProseMirror 是 HTML 模型、且 markdown 丢失样式子集的「部分支持」层（颜色/字体/字号）。
//! 处理逻辑：
//! - **打开（docx→可编辑 HTML）**：`pandoc <docx> --from docx+styles --to
//!   html+raw_html+native_divs+native_spans --wrap=none --extract-media=. --preserve-tabs
//!   （**不加 --standalone**：只输出 <body> 正文片段，不含 <head>/<style> CSS——否则前端会把那坨 CSS
//!    当文本/代码块渲染。前端再以 generateJSON 按 HTML 解析，绕过 markdown 扩展，渲染为成熟文本块）
//!   --lua-filter preserve-styles.lua`（`docx+styles` 读取扩展保留样式；`native_divs/spans` 把样式落为
//!   内联 `style=`/`data-custom-style`；Lua 过滤器强制保留对齐/颜色/字号；`--extract-media` 解出图片）。
//!   工作目录设为文档父目录使图片可被编辑器找到；HTML 超 15MB 拒绝（编辑模式上限，防崩溃）。
//! - **保存（HTML→docx）**：`pandoc --from html --to docx --reference-doc reference.docx`（参考文档统一
//!   样式，往返保真）。
//! 必需子集往返保真 + 部分支持保留并明示由此达成（BR-DOC-DATA-002）；本地子进程内容不上行
//! （BR-DOC-DATA-001）；依赖（pandoc / 资源）缺失回统一错误不静默降级。资源 reference.docx /
//! preserve-styles.lua 经 bundle.resources 打包，路径解析内置优先。

#[cfg(test)]
use crate::doc::pandoc_runtime::pandoc_available;
use crate::doc::{
    docx_cell_shading, docx_color, docx_image_layout, image_media, style_subset, tool_runtime,
};
use crate::ipc::IpcError;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

/// 编辑模式 HTML 上限（移植旧项目：超限拒绝，防大文档崩溃）。
const MAX_EDIT_HTML_BYTES: usize = 15 * 1024 * 1024;

/// 打开编辑链的完整产物：HTML 与同一份实际转换结果的样式报告必须一起返回，防止报告和内容脱节。
#[derive(Debug, Clone)]
pub struct EditHtml {
    pub html: String,
    pub style_report: style_subset::StyleReport,
}

fn dep_missing(detail: &str) -> IpcError {
    IpcError::new(
        "DOC_EDIT_DEP_MISSING",
        &format!("编辑链依赖缺失: {detail}（不静默降级）"),
        true,
        false,
    )
}
fn convert_fail(detail: &str) -> IpcError {
    IpcError::new(
        "DOC_EDIT_CONVERT_FAIL",
        &format!("编辑链转换失败: {detail}"),
        true,
        false,
    )
}

/// 解析打包资源（reference.docx / preserve-styles.lua）路径：内置优先（TAURI_RESOURCE_DIR → 打包
/// .app/Contents/Resources → 开发 src-tauri/resources / resources）。移植旧项目资源解析顺序。
pub fn resolve_resource(filename: &str) -> Option<PathBuf> {
    if let Ok(res) = std::env::var("TAURI_RESOURCE_DIR") {
        let p = PathBuf::from(res).join(filename);
        if p.exists() {
            return Some(p);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(contents) = exe.parent().and_then(|p| p.parent()) {
            if contents.ends_with("Contents") {
                let p = contents.join("Resources").join(filename);
                if p.exists() {
                    return Some(p);
                }
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        for d in [cwd.join("src-tauri/resources"), cwd.join("resources")] {
            let p = d.join(filename);
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}

/**
 * @GOV
 * codes:    BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-016, BR-DOC-DATA-003-IO-DOC-DOC-CONVERT-016
 * type:     IO
 * chain:    DOC-CONVERT
 * rules:    BR-DOC-DATA-002, BR-DOC-DATA-003
 * boundary: in=docxFileAndWorkdir | out=editableHtml(stylesPreservedBlockIdBackfilledViaStylesExtAndLuaFilterCapped)
 */
/// 打开 docx 为可编辑 HTML（BR-DOC-DATA-002 样式子集保留 / BR-DOC-DATA-001 本地不上行）：经 pandoc
/// `docx+styles → html+raw_html+native_divs+native_spans` + preserve-styles.lua 过滤器，必需子集往返、
/// 部分支持（颜色/字体/字号）保留为内联样式供 TipTap 编辑；`--extract-media` 至 work_dir 使图片可见；
/// 转换后补齐块级 data-block-id（BR-DOC-DATA-003）；HTML 超 15MB 拒绝。依赖/资源缺失回统一错误不静默降级。
pub fn docx_to_html(docx_path: &Path, work_dir: &Path) -> Result<String, IpcError> {
    Ok(docx_to_edit_html(docx_path, work_dir)?.html)
}

/// 打开 docx 为编辑 HTML，并对 OOXML 直接颜色执行恢复、生成本次转换的样式报告。
pub fn docx_to_edit_html(docx_path: &Path, work_dir: &Path) -> Result<EditHtml, IpcError> {
    let pandoc = tool_runtime::pandoc_path().map_err(|_| dep_missing("包内 pandoc"))?;
    if !docx_path.exists() {
        return Err(convert_fail(&format!("输入 docx 不存在: {docx_path:?}")));
    }
    let lua = resolve_resource("preserve-styles.lua")
        .ok_or_else(|| dep_missing("preserve-styles.lua（样式保留过滤器）"))?;
    std::fs::create_dir_all(work_dir).map_err(|e| convert_fail(&e.to_string()))?;
    let out = Command::new(pandoc)
        .arg(docx_path)
        .args(["--from", "docx+styles"])
        .args(["--to", "html+raw_html+native_divs+native_spans"])
        .args(["--wrap=none", "--extract-media=.", "--preserve-tabs"])
        .arg("--lua-filter")
        .arg(&lua)
        .current_dir(work_dir) // --extract-media=. 解压到此，图片可被编辑器找到
        .output()
        .map_err(|_| dep_missing("pandoc"))?;
    if !out.status.success() {
        return Err(convert_fail(&String::from_utf8_lossy(&out.stderr)));
    }
    let html = String::from_utf8(out.stdout).map_err(|e| convert_fail(&e.to_string()))?;
    // Pandoc 不保证把 w:color 读成 HTML CSS；从同一份 OOXML 恢复颜色，并记录不能匹配的片段。
    let source_docx = std::fs::read(docx_path).map_err(|e| convert_fail(&e.to_string()))?;
    let (html, color_recovery) = docx_color::restore_html_colors(&source_docx, &html)?;
    let (html, cell_shading_recovery) =
        docx_cell_shading::restore_html_cell_shading(&source_docx, &html)?;
    let html = docx_image_layout::restore_html_image_layouts(&source_docx, &html)?;
    let html = crate::doc::block_backfill::inject_missing_data_block_ids(&html);
    let image_recovery = image_media::inspect_recovery(&source_docx, &html, work_dir)?;
    if html.len() > MAX_EDIT_HTML_BYTES {
        return Err(IpcError::new(
            "DOC_EDIT_TOO_LARGE",
            &format!(
                "文档转换后 HTML 过大（{} MB），超编辑模式上限 15MB",
                html.len() / 1024 / 1024
            ),
            false,
            false,
        ));
    }
    Ok(EditHtml {
        style_report: style_subset::inspect_html(
            &html,
            color_recovery,
            cell_shading_recovery,
            image_recovery,
        ),
        html,
    })
}

/**
 * @GOV
 * codes:    BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-004
 * type:     IO
 * chain:    DOC-CONVERT
 * rules:    BR-DOC-DATA-002
 * boundary: in=editedHtml | out=docxBytes(referenceDocStyledRoundTripLocalNoUplink)
 */
/// Pandoc 子进程超时（秒）。大文档 HTML→docx 通常 < 30s；预留 60s 上限。
const PANDOC_TIMEOUT_SECS: u64 = 60;
static OUTPUT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn has_resource_loss_warning(stderr: &str) -> bool {
    let lower = stderr.to_ascii_lowercase();
    lower.contains("could not fetch resource")
        || lower.contains("replacing image with description")
        || lower.contains("could not find image")
}

/// 保存可编辑 HTML 为 docx（BR-DOC-DATA-002 往返保真 / BR-DOC-DATA-001 本地不上行）：经 pandoc
/// `html → docx --reference-doc reference.docx`（参考文档统一样式），html 经 stdin、docx 写临时文件读回
/// 字节，即用即清。依赖/资源缺失回统一错误不静默降级。
///
/// **审计修复（2026-07-24）**：
/// - 显式 drop stdin 确保 Pandoc 收到 EOF，避免死锁（P0）
/// - 新增 wait_timeout 超时控制，防止 Pandoc 挂死（P1）
/// - 设置 current_dir 为文档父目录，使 <img src> 相对路径可解析（P1，图片保存路径断裂）
pub fn html_to_docx(html: &str, media_dir: Option<&Path>) -> Result<Vec<u8>, IpcError> {
    use std::io::Write;
    let pandoc = tool_runtime::pandoc_path().map_err(|_| dep_missing("包内 pandoc"))?;
    let reference = resolve_resource("reference.docx")
        .ok_or_else(|| dep_missing("reference.docx（参考样式文档）"))?;
    let resources = image_media::inspect_html_resources(html, media_dir)?;
    if !resources.missing_resources.is_empty() {
        return Err(IpcError::new(
            "DOC_IMAGE_RESOURCE_MISSING",
            &format!(
                "BDocx 图片资源缺失：{}；已拒绝覆盖原文档",
                resources.missing_resources.join("、")
            ),
            true,
            false,
        ));
    }
    let output_id = OUTPUT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let out_path = std::env::temp_dir().join(format!(
        "binder_edit_{}_{}.docx",
        std::process::id(),
        output_id
    ));
    let mut cmd = Command::new(pandoc);
    cmd.args(["--from", "html", "--to", "docx"])
        .arg("--reference-doc")
        .arg(&reference)
        .arg("-o")
        .arg(&out_path)
        .stdin(Stdio::piped())
        .stderr(Stdio::piped());
    // 相对图片只在来源 baseline 派生的媒体目录解析；禁止退回 DOCX 父目录猜测资源位置。
    if let Some(dir) = media_dir {
        cmd.current_dir(dir);
    }
    let mut child = cmd.spawn().map_err(|_| dep_missing("pandoc"))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| convert_fail("无法读取 pandoc stderr"))?;
    let stderr_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = stderr.read_to_end(&mut bytes);
        (result, bytes)
    });

    // 写入 stdin 后显式 drop，确保 Pandoc 收到 EOF
    let write_result = (|| -> Result<(), IpcError> {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| convert_fail("无法写入 stdin"))?;
        // Pandoc 本身不可靠地将 CSS color 写为 w:color：先写入仅本次转换使用的标记，生成 DOCX 后
        // 再由 docx_color 直接物化为 OOXML 字体色，最终包中不保留标记文本。
        let marked_html = docx_image_layout::mark_html_image_layouts(html);
        let marked_html = docx_cell_shading::mark_html_cell_shading(&marked_html);
        let marked_html = docx_color::mark_html_colors(&marked_html);
        stdin
            .write_all(marked_html.as_bytes())
            .map_err(|e| convert_fail(&e.to_string()))?;
        // drop(stdin) 关闭管道，Pandoc 收到 EOF
        Ok(())
    })();
    if let Err(error) = write_result {
        let _ = child.kill();
        let _ = child.wait();
        let _ = stderr_reader.join();
        let _ = std::fs::remove_file(&out_path);
        return Err(error);
    }

    // 子进程保留在当前线程轮询，超时时可以真实 kill；stderr 并发排空，避免大量警告填满管道死锁。
    let started_at = std::time::Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None)
                if started_at.elapsed() < std::time::Duration::from_secs(PANDOC_TIMEOUT_SECS) =>
            {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stderr_reader.join();
                let _ = std::fs::remove_file(&out_path);
                return Err(convert_fail(&format!(
                    "pandoc 转换超时（>{PANDOC_TIMEOUT_SECS}s）"
                )));
            }
            Err(error) => {
                let _ = stderr_reader.join();
                let _ = std::fs::remove_file(&out_path);
                return Err(convert_fail(&error.to_string()));
            }
        }
    };
    let stderr_bytes = match stderr_reader.join() {
        Ok((Ok(_), bytes)) => bytes,
        Ok((Err(error), _)) => {
            let _ = std::fs::remove_file(&out_path);
            return Err(convert_fail(&format!("读取 pandoc stderr 失败: {error}")));
        }
        Err(_) => {
            let _ = std::fs::remove_file(&out_path);
            return Err(convert_fail("读取 pandoc stderr 的线程异常退出"));
        }
    };
    let stderr_text = String::from_utf8_lossy(&stderr_bytes);

    if !status.success() {
        let _ = std::fs::remove_file(&out_path);
        return Err(convert_fail(&format!(
            "pandoc 非零退出: {}",
            stderr_text.trim()
        )));
    }
    if has_resource_loss_warning(&stderr_text) {
        let _ = std::fs::remove_file(&out_path);
        return Err(IpcError::new(
            "DOC_IMAGE_RESOURCE_MISSING",
            &format!(
                "Pandoc 未能读取图片资源，已拒绝覆盖原文档: {}",
                stderr_text.trim()
            ),
            true,
            false,
        ));
    }
    let bytes = std::fs::read(&out_path).map_err(|e| convert_fail(&e.to_string()));
    let _ = std::fs::remove_file(&out_path);
    let bytes = docx_color::materialize_marked_colors(&bytes?)?;
    let bytes = docx_cell_shading::materialize_marked_cell_shading(&bytes)?;
    let bytes = docx_image_layout::materialize_marked_image_layouts(&bytes)?;
    image_media::verify_docx_images(html, &bytes, media_dir)?;
    docx_image_layout::verify_docx_image_layouts(html, &bytes)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG_1X1: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 8, 215, 99, 248, 207, 192, 240, 31,
        0, 5, 0, 1, 255, 137, 153, 61, 29, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];

    // covers: BR-DOC-DATA-002
    // 编辑链资源解析：开发模式从 src-tauri/resources 找到 reference.docx / preserve-styles.lua。
    #[test]
    fn resolves_edit_chain_resources() {
        // 在仓库根运行 cargo test 时，cwd=src-tauri，resources/ 下应有这两个资源
        assert!(
            resolve_resource("preserve-styles.lua").is_some(),
            "preserve-styles.lua 应可解析（已拷入 resources）"
        );
        assert!(
            resolve_resource("reference.docx").is_some(),
            "reference.docx 应可解析"
        );
        assert!(resolve_resource("不存在的资源.xyz").is_none());
    }

    // covers: BR-DOC-DATA-002
    // 端到端编辑链往返：颜色必须在保存侧写为 `w:color`，并在打开侧恢复为编辑器 HTML CSS。
    #[test]
    fn html_docx_roundtrip_preserves_subset_when_pandoc_available() {
        assert!(pandoc_available(), "必须使用已校验的包内 Pandoc");
        let html = r##"<h1>编辑标题</h1><p><strong>加粗</strong> 与 <span style="color:#ff0000">红字</span></p><table><tbody><tr><td data-binder-cell-shading="#FFE699" style="background-color:#FFE699"><p>底纹单元格</p></td></tr></tbody></table>"##;
        let docx = html_to_docx(html, None).expect("html→docx");
        assert!(!docx.is_empty() && &docx[..2] == b"PK", "docx 为 OOXML zip");
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&docx)).expect("读取 docx zip");
        let mut xml = String::new();
        use std::io::Read;
        archive
            .by_name("word/document.xml")
            .expect("document.xml")
            .read_to_string(&mut xml)
            .expect("读取 document.xml");
        assert!(
            xml.contains(r#"<w:color w:val="FF0000"/>"#),
            "颜色必须写入 DOCX OOXML"
        );
        assert!(
            xml.contains(r#"<w:shd w:val="clear" w:fill="FFE699"/>"#),
            "TableCellShading 必须写入 DOCX OOXML"
        );
        assert!(!xml.contains("BINDER_COLOR_"), "中转标记不得落入最终 DOCX");
        assert!(
            !xml.contains("BINDER_CELL_SHADING_"),
            "底纹中转标记不得落入最终 DOCX"
        );

        // 写 docx 到临时文件再开回 HTML（编辑链打开侧）
        let base = std::env::temp_dir().join(format!("binder_edit_e2e_{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        let docx_path = base.join("doc.docx");
        std::fs::write(&docx_path, &docx).unwrap();
        let back = docx_to_html(&docx_path, &base.join("work")).expect("docx→html");
        assert!(back.contains("编辑标题"), "必需子集：标题往返保真");
        assert!(back.contains("加粗"), "必需子集：粗体往返保真");
        assert!(
            back.contains(r#"style="color:#FF0000""#),
            "文字颜色重开后必须恢复为 HTML 样式"
        );
        assert!(
            back.contains(r##"data-binder-cell-shading="#FFE699""##)
                && back.contains("background-color:#FFE699"),
            "TableCellShading 重开后必须恢复为 HTML 属性"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    // covers: BR-DOC-DATA-002, BR-DOC-DATA-003
    // 图片必须从 baseline 媒体目录嵌入，数量/资源/尺寸经最终 OOXML 和重开报告双重验证。
    #[test]
    fn image_roundtrip_preserves_resource_alt_and_dimensions_when_pandoc_available() {
        assert!(pandoc_available(), "必须使用已校验的包内 Pandoc");
        let root =
            std::env::temp_dir().join(format!("binder_image_roundtrip_{}", std::process::id()));
        let media_dir = root.join("source");
        let reopen_dir = root.join("reopen");
        std::fs::create_dir_all(media_dir.join("media")).expect("创建图片媒体目录");
        std::fs::write(media_dir.join("media/image1.png"), PNG_1X1).expect("写入测试图片");
        let html = r#"<p><img src="media/image1.png" alt="项目插图" width="120" height="72"></p>"#;
        let docx = html_to_docx(html, Some(&media_dir)).expect("图片 HTML→DOCX");

        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&docx)).expect("读取图片 DOCX");
        assert!(
            (0..archive.len()).any(|index| {
                archive
                    .by_index(index)
                    .map(|entry| entry.name().starts_with("word/media/"))
                    .unwrap_or(false)
            }),
            "输出包必须含 word/media 图片资源"
        );
        let mut document_xml = String::new();
        archive
            .by_name("word/document.xml")
            .expect("document.xml")
            .read_to_string(&mut document_xml)
            .expect("读取 document.xml");
        assert!(
            document_xml.contains("<w:drawing"),
            "输出 OOXML 必须含图片对象"
        );
        assert!(
            document_xml.contains(r#"cx="1143000""#) && document_xml.contains(r#"cy="685800""#),
            "120×72 CSS px 必须转换为 1.25×0.75in OOXML extent"
        );
        drop(archive);

        let docx_path = root.join("roundtrip.docx");
        std::fs::write(&docx_path, &docx).expect("写入重开 DOCX");
        let reopened = docx_to_edit_html(&docx_path, &reopen_dir).expect("重开图片 DOCX");
        assert!(reopened.html.contains("项目插图"), "图片 alt 文本必须保留");
        assert!(reopened.html.to_ascii_lowercase().contains("<img"));
        assert_eq!(reopened.style_report.image_recovery.detected_images, 1);
        assert_eq!(reopened.style_report.image_recovery.restored_images, 1);
        assert_eq!(reopened.style_report.image_recovery.unrecovered_images, 0);
        assert_eq!(
            reopened
                .style_report
                .image_recovery
                .unrecovered_sized_images,
            0
        );
        let _ = std::fs::remove_dir_all(root);
    }

    // covers: BR-DOC-DATA-002
    // 图片人工操作落入 HTML 后，DOC 转换必须保持次序、尺寸与删除结果；自定义 data-display 不属于保证集。
    #[test]
    fn image_interaction_results_roundtrip_in_document_flow_when_pandoc_available() {
        assert!(pandoc_available(), "必须使用已校验的包内 Pandoc");
        let root = std::env::temp_dir().join(format!(
            "binder_image_interaction_roundtrip_{}",
            std::process::id()
        ));
        let media_dir = root.join("source");
        std::fs::create_dir_all(media_dir.join("media")).expect("创建图片媒体目录");
        std::fs::write(media_dir.join("media/first.png"), PNG_1X1).expect("写入第一张图片");
        std::fs::write(media_dir.join("media/second.png"), PNG_1X1).expect("写入第二张图片");

        // 模拟 ED 已把独立图片段落移动到“中段”之后，并把第二张图拖宽到 144px。
        let moved_html = r#"<p>前段</p><p>中段</p><p><img src="media/second.png" alt="第二张图" width="144" data-display="block"></p><p><img src="media/first.png" alt="第一张图" width="96"></p><p>后段</p>"#;
        let moved_docx = html_to_docx(moved_html, Some(&media_dir)).expect("移动后 HTML→DOCX");
        let moved_path = root.join("moved.docx");
        std::fs::write(&moved_path, moved_docx).expect("写入移动后 DOCX");
        let moved =
            docx_to_edit_html(&moved_path, &root.join("moved-reopen")).expect("重开移动后 DOCX");
        let middle = moved.html.find("中段").expect("中段存在");
        let second = moved.html.find("第二张图").expect("第二张图存在");
        let first = moved.html.find("第一张图").expect("第一张图存在");
        let after = moved.html.find("后段").expect("后段存在");
        assert!(
            middle < second && second < first && first < after,
            "图片段落的文档流顺序必须保持"
        );
        assert!(
            moved.html.contains("width:1.5in"),
            "144px 图片宽度必须重开为等价 1.5in"
        );
        assert!(
            !moved.html.contains("data-display"),
            "Pandoc 不保留 data-display，不能把它声明为 BDocx 持久能力"
        );
        assert_eq!(moved.style_report.image_recovery.detected_images, 2);
        assert_eq!(moved.style_report.image_recovery.restored_images, 2);

        // 模拟 ED 删除第一张图片及其独立段落；保存和重开后实例数必须真实下降。
        let deleted_html = r#"<p>前段</p><p>中段</p><p><img src="media/second.png" alt="第二张图" width="144"></p><p>后段</p>"#;
        let deleted_docx = html_to_docx(deleted_html, Some(&media_dir)).expect("删除后 HTML→DOCX");
        let deleted_path = root.join("deleted.docx");
        std::fs::write(&deleted_path, deleted_docx).expect("写入删除后 DOCX");
        let deleted = docx_to_edit_html(&deleted_path, &root.join("deleted-reopen"))
            .expect("重开删除后 DOCX");
        assert!(deleted.html.contains("第二张图"));
        assert!(!deleted.html.contains("第一张图"));
        assert_eq!(deleted.style_report.image_recovery.detected_images, 1);
        assert_eq!(deleted.style_report.image_recovery.restored_images, 1);
        assert_eq!(deleted.style_report.image_recovery.unrecovered_images, 0);
        let _ = std::fs::remove_dir_all(root);
    }

    // covers: BR-DOC-DATA-002, BR-DOC-DATA-003, BR-WS-DATA-001
    #[test]
    fn agent_project_image_insert_roundtrips_resource_alt_width_and_block_position() {
        assert!(pandoc_available(), "必须使用已校验的包内 Pandoc");
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let baseline = unique as u64;
        let root = std::env::temp_dir().join(format!("binder_agent_project_image_{unique}"));
        std::fs::create_dir_all(root.join("images")).expect("创建项目图片目录");
        std::fs::write(root.join("images/cover.png"), PNG_1X1).expect("写入项目图片");
        let asset =
            image_media::inspect_project_image(&root, "images/cover.png").expect("校验项目图片");
        let materialized = image_media::materialize_project_image(
            &root,
            "images/cover.png",
            &asset.content_hash,
            baseline,
        )
        .expect("物化项目图片");
        let html = format!(
            r#"<p data-block-id="before">前段</p><p data-block-id="inserted" style="text-align:center"><img src="{media}" data-binder-media-src="{media}" data-binder-project-src="images/cover.png" data-binder-project-hash="{hash}" alt="代理封面" width="320"></p><p data-block-id="after">后段</p>"#,
            media = materialized.media_ref,
            hash = asset.content_hash,
        );
        let docx = html_to_docx(&html, Some(&image_media::edit_media_dir(baseline)))
            .expect("代理项目图片 HTML→DOCX");
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&docx)).expect("读取图片 DOCX");
        let mut xml = String::new();
        use std::io::Read;
        archive
            .by_name("word/document.xml")
            .expect("document.xml")
            .read_to_string(&mut xml)
            .expect("读取 document.xml");
        assert!(
            xml.contains(r#"<w:jc w:val="center"/>"#),
            "独立图片段落居中必须写入 w:jc"
        );
        let path = root.join("agent-image.docx");
        std::fs::write(&path, docx).expect("写入代理图片 DOCX");
        let reopened = docx_to_edit_html(&path, &root.join("reopen")).expect("重开代理图片 DOCX");
        let before = reopened.html.find("前段").expect("前段");
        let image = reopened.html.find("代理封面").expect("代理图片 alt");
        let after = reopened.html.find("后段").expect("后段");
        assert!(
            before < image && image < after,
            "独立图片 Block 位置必须保持"
        );
        assert!(
            reopened.html.contains("width:3.33333in"),
            "320px 图片宽度必须重开为等价物理长度"
        );
        assert!(
            reopened.html.contains("text-align: center")
                || reopened.html.contains("text-align:center"),
            "独立图片段落居中必须在 DOCX 重开后恢复"
        );
        assert_eq!(reopened.style_report.image_recovery.detected_images, 1);
        assert_eq!(reopened.style_report.image_recovery.restored_images, 1);
        assert_eq!(reopened.style_report.image_recovery.unrecovered_images, 0);
        assert_eq!(
            std::fs::read(root.join("images/cover.png")).unwrap(),
            PNG_1X1,
            "保存链不得改写项目源图片"
        );
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(image_media::edit_media_dir(baseline));
    }

    // covers: BR-DOC-DATA-002, BR-DOC-DATA-003, BR-WS-PERSIST-001
    // 代理多图片操作的最终 LogicalState 必须经真实 Pandoc/OOXML 保存重开；转换失败发生在原子写盘前。
    #[test]
    fn agent_image_operation_batch_saves_reopens_and_failed_conversion_keeps_original() {
        assert!(pandoc_available(), "必须使用已校验的包内 Pandoc");
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("binder_agent_image_e2e_{unique}"));
        let media_dir = root.join("edit");
        std::fs::create_dir_all(media_dir.join("media")).expect("创建编辑媒体目录");
        std::fs::write(media_dir.join("media/first.png"), PNG_1X1).expect("写入第一张图片");
        let mut second_png = PNG_1X1.to_vec();
        second_png.push(0);
        std::fs::write(media_dir.join("media/second.png"), &second_png).expect("写入第二张图片");
        std::fs::write(media_dir.join("media/deleted.png"), PNG_1X1).expect("写入待删除图片资源");

        let target = root.join("current.draft.docx");
        let original = html_to_docx("<p>原始文档</p>", None).expect("生成原始 DOCX");
        crate::workspace::save::save_atomic(&target, &original).expect("写入原始 DOCX");

        // 模拟 Editor 已完成：第二张图上移并右对齐、第一张图缩放并居中、第三张图删除。
        let edited_html = r#"<p>前段</p><p style="text-align:right"><img src="media/second.png" alt="第二张图" width="240"></p><p>中段</p><p style="text-align:center"><img src="media/first.png" alt="第一张图" width="120"></p><p>后段</p>"#;
        let edited = html_to_docx(edited_html, Some(&media_dir)).expect("组合图片操作 HTML→DOCX");
        crate::workspace::save::save_atomic(&target, &edited).expect("原子保存组合图片结果");
        let saved_bytes = std::fs::read(&target).expect("读取已保存 DOCX");
        let reopened = docx_to_edit_html(&target, &root.join("reopen")).expect("重开组合图片 DOCX");

        let before = reopened.html.find("前段").expect("前段");
        let second = reopened.html.find("第二张图").expect("第二张图");
        let middle = reopened.html.find("中段").expect("中段");
        let first = reopened.html.find("第一张图").expect("第一张图");
        let after = reopened.html.find("后段").expect("后段");
        assert!(
            before < second && second < middle && middle < first && first < after,
            "移动后的图片与正文顺序必须保存重开一致"
        );
        assert!(!reopened.html.contains("待删除图片"));
        assert!(
            reopened.html.contains("width:2.5in") && reopened.html.contains("width:1.25in"),
            "240px 与 120px 图片宽度必须保存为等价物理尺寸"
        );
        assert!(
            reopened.html.contains("text-align: right")
                || reopened.html.contains("text-align:right"),
            "第二张图右对齐必须恢复"
        );
        assert!(
            reopened.html.contains("text-align: center")
                || reopened.html.contains("text-align:center"),
            "第一张图居中必须恢复"
        );
        assert_eq!(reopened.style_report.image_recovery.detected_images, 2);
        assert_eq!(reopened.style_report.image_recovery.restored_images, 2);
        assert_eq!(reopened.style_report.image_recovery.unrecovered_images, 0);
        assert_eq!(
            std::fs::read(media_dir.join("media/first.png")).unwrap(),
            PNG_1X1,
            "保存不得改写媒体源"
        );
        assert_eq!(
            std::fs::read(media_dir.join("media/second.png")).unwrap(),
            second_png,
            "保存不得改写媒体源"
        );

        let conversion_error = html_to_docx(
            r#"<p><img src="media/missing.png" alt="缺失图片"></p>"#,
            Some(&media_dir),
        )
        .expect_err("缺失资源必须在原子写盘前阻断");
        assert_eq!(conversion_error.code, "DOC_IMAGE_RESOURCE_MISSING");
        assert_eq!(
            std::fs::read(&target).unwrap(),
            saved_bytes,
            "转换失败不得覆盖当前 DOCX"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn missing_image_resource_is_an_explicit_conversion_error() {
        let dir = std::env::temp_dir().join(format!("binder_missing_image_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("创建空媒体目录");
        let error = html_to_docx(
            r#"<p><img src="media/missing.png" alt="不可静默丢失"></p>"#,
            Some(&dir),
        )
        .expect_err("缺失图片必须失败");
        assert_eq!(error.code, "DOC_IMAGE_RESOURCE_MISSING");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn pandoc_resource_loss_warning_is_never_treated_as_success() {
        assert!(has_resource_loss_warning(
            "[WARNING] Could not fetch resource media/image.png: replacing image with description"
        ));
        assert!(!has_resource_loss_warning(
            "[WARNING] This is an unrelated formatting warning"
        ));
    }

    #[test]
    fn output_image_count_drop_is_rejected() {
        assert!(pandoc_available(), "必须使用已校验的包内 Pandoc");
        let image_less = html_to_docx("<p>只有文字</p>", None).expect("生成无图 DOCX");
        let error = image_media::verify_docx_images(
            r#"<p><img src="data:image/png;base64,AAAA"></p>"#,
            &image_less,
            None,
        )
        .expect_err("图片数量下降必须失败");
        assert_eq!(error.code, "DOC_IMAGE_ROUNDTRIP_LOSS");
    }

    #[test]
    fn output_image_media_order_and_extent_mismatch_are_rejected() {
        assert!(pandoc_available(), "必须使用已校验的包内 Pandoc");
        let root = std::env::temp_dir().join(format!("binder_image_verify_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("media")).unwrap();
        std::fs::write(root.join("media/first.png"), PNG_1X1).unwrap();
        let mut second = PNG_1X1.to_vec();
        second.push(0);
        std::fs::write(root.join("media/second.png"), &second).unwrap();
        let html = r#"<p><img src="media/first.png" width="120"></p><p><img src="media/second.png" width="160"></p>"#;
        let docx = html_to_docx(html, Some(&root)).expect("生成双图 DOCX");

        let swapped = r#"<p><img src="media/second.png" width="120"></p><p><img src="media/first.png" width="160"></p>"#;
        assert_eq!(
            image_media::verify_docx_images(swapped, &docx, Some(&root))
                .unwrap_err()
                .code,
            "DOC_IMAGE_ROUNDTRIP_LOSS"
        );
        let wrong_width = html.replace(r#"width="120""#, r#"width="200""#);
        assert_eq!(
            image_media::verify_docx_images(&wrong_width, &docx, Some(&root))
                .unwrap_err()
                .code,
            "DOC_IMAGE_ROUNDTRIP_LOSS"
        );
        std::fs::write(root.join("media/first.png"), &second).unwrap();
        assert_eq!(
            image_media::verify_docx_images(html, &docx, Some(&root))
                .unwrap_err()
                .code,
            "DOC_IMAGE_ROUNDTRIP_LOSS"
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
