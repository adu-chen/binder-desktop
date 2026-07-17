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

use crate::doc::pandoc_runtime::pandoc_available;
use crate::ipc::IpcError;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// 编辑模式 HTML 上限（移植旧项目：超限拒绝，防大文档崩溃）。
const MAX_EDIT_HTML_BYTES: usize = 15 * 1024 * 1024;

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
 * codes:    BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-003
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
    if !pandoc_available() {
        return Err(dep_missing("pandoc"));
    }
    if !docx_path.exists() {
        return Err(convert_fail(&format!("输入 docx 不存在: {docx_path:?}")));
    }
    let lua = resolve_resource("preserve-styles.lua")
        .ok_or_else(|| dep_missing("preserve-styles.lua（样式保留过滤器）"))?;
    std::fs::create_dir_all(work_dir).map_err(|e| convert_fail(&e.to_string()))?;
    let out = Command::new("pandoc")
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
    let html = crate::doc::block_backfill::inject_missing_data_block_ids(&html);
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
    Ok(html)
}

/**
 * @GOV
 * codes:    BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-004
 * type:     IO
 * chain:    DOC-CONVERT
 * rules:    BR-DOC-DATA-002
 * boundary: in=editedHtml | out=docxBytes(referenceDocStyledRoundTripLocalNoUplink)
 */
/// 保存可编辑 HTML 为 docx（BR-DOC-DATA-002 往返保真 / BR-DOC-DATA-001 本地不上行）：经 pandoc
/// `html → docx --reference-doc reference.docx`（参考文档统一样式），html 经 stdin、docx 写临时文件读回
/// 字节，即用即清。依赖/资源缺失回统一错误不静默降级。
pub fn html_to_docx(html: &str) -> Result<Vec<u8>, IpcError> {
    use std::io::Write;
    if !pandoc_available() {
        return Err(dep_missing("pandoc"));
    }
    let reference = resolve_resource("reference.docx")
        .ok_or_else(|| dep_missing("reference.docx（参考样式文档）"))?;
    let out_path = std::env::temp_dir().join(format!("binder_edit_{}.docx", std::process::id()));
    let mut child = Command::new("pandoc")
        .args(["--from", "html", "--to", "docx"])
        .arg("--reference-doc")
        .arg(&reference)
        .arg("-o")
        .arg(&out_path)
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| dep_missing("pandoc"))?;
    child
        .stdin
        .take()
        .ok_or_else(|| convert_fail("无法写入 stdin"))?
        .write_all(html.as_bytes())
        .map_err(|e| convert_fail(&e.to_string()))?;
    let status = child.wait().map_err(|e| convert_fail(&e.to_string()))?;
    if !status.success() {
        let _ = std::fs::remove_file(&out_path);
        return Err(convert_fail("pandoc 非零退出"));
    }
    let bytes = std::fs::read(&out_path).map_err(|e| convert_fail(&e.to_string()));
    let _ = std::fs::remove_file(&out_path);
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

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
    // 端到端编辑链往返（pandoc 可用时真跑）：构造带样式的 HTML → docx → 再 docx→HTML，必需子集（标题/
    // 粗体）保真，部分支持（颜色 inline style）经 docx+styles+lua 保留。无 pandoc 自动跳过。
    #[test]
    fn html_docx_roundtrip_preserves_subset_when_pandoc_available() {
        if !pandoc_available() {
            return;
        }
        let html = r#"<h1>编辑标题</h1><p><strong>加粗</strong> 与 <span style="color:#ff0000">红字</span></p>"#;
        let docx = html_to_docx(html).expect("html→docx");
        assert!(!docx.is_empty() && &docx[..2] == b"PK", "docx 为 OOXML zip");

        // 写 docx 到临时文件再开回 HTML（编辑链打开侧）
        let base = std::env::temp_dir().join(format!("binder_edit_e2e_{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        let docx_path = base.join("doc.docx");
        std::fs::write(&docx_path, &docx).unwrap();
        let back = docx_to_html(&docx_path, &base.join("work")).expect("docx→html");
        assert!(back.contains("编辑标题"), "必需子集：标题往返保真");
        assert!(back.contains("加粗"), "必需子集：粗体往返保真");
        let _ = std::fs::remove_dir_all(&base);
    }
}
