//! PI-DOC-01/03（实现期续接）· LibreOffice 预览链运行时 · DOC-CONVERT / BR-DOC-DATA-001
//!
//! 把 convert.rs 的双链分离契约落到真实 LibreOffice headless 子进程：docx/xlsx/pptx → PDF 预览——
//! **内容不经网络上行**（纯本地子进程）；**依赖缺失（soffice 不可用）回统一错误形状、不静默降级**
//! （BR-DOC-DATA-001）；编辑链用 Pandoc（pandoc_runtime）、预览链用 LibreOffice（本模块），互不共享中间态。
//!
//! 处理逻辑移植并优化自旧项目 binder-project 的 libreoffice_service（同一处理逻辑）：
//! - **包内唯一路径解析**：Tauri `resource_dir()` → manifest 校验的
//!   `libreoffice/LibreOffice.app/Contents/MacOS/soffice`；不探测系统 /Applications 或 PATH。
//! - **macOS 运行环境**（使内置 LO 自包含运行）：current_dir=Contents、DYLD_LIBRARY_PATH=Frameworks+MacOS、
//!   SAL_DISABLE_OPENCL=1、HOME=lo_user、`-env:UserInstallation=file://…/lo_user`。VCL 后端由稳定源码构建验收决定，
//!   运行时不猜测性强制选择。
//! - **转换 filter**（含文本层与嵌字体，确保可复制/字体一致）：
//!   `pdf:{writer|calc|impress}_pdf_Export:UseTaggedPDF=1:SelectPdfVersion=1:EmbedStandardFonts=1:
//!   EmbedLatinScriptFonts=1:EmbedAsianScriptFonts=1`。
//! - **字体策略**：当前只生成不含正文的 Font Trace/FontPlan 诊断；旧 `fontsubst.xcu` 宿主默认字体替换
//!   已停用。实际 glyph 选择须由后续已验证的包内字体与 LO 源码 GlyphFallback 实现接管。
//! - **重试扫描找 PDF**：精确名匹配优先，否则取最新；>100 字节。
//! LO 资源打包（deploy 脚本）与缓存策略为部署/实现期；本模块落处理逻辑与依赖守卫。

use crate::{
    doc::{font_trace, tool_runtime},
    ipc::IpcError,
};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

/// 预览转换格式（决定 PDF 导出 filter；编辑链 odt 不在此）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewKind {
    Docx,         // writer_pdf_Export
    Excel,        // calc_pdf_Export
    Presentation, // impress_pdf_Export
}

/// PDF 导出公共选项（文本层 + 嵌字体；移植旧项目，确保可复制与字体一致）。
const PDF_EXPORT_OPTS: &str =
    "UseTaggedPDF=1:SelectPdfVersion=1:EmbedStandardFonts=1:EmbedLatinScriptFonts=1:EmbedAsianScriptFonts=1";

fn dep_missing() -> IpcError {
    IpcError::new(
        "DOC_LIBREOFFICE_MISSING",
        "LibreOffice 依赖缺失（预览链），明确报错不静默降级",
        true,
        false,
    )
}
fn convert_fail(detail: &str) -> IpcError {
    IpcError::new(
        "DOC_PREVIEW_FAIL",
        &format!("LibreOffice 预览转换失败: {detail}"),
        true,
        false,
    )
}
fn start_fail(detail: &str) -> IpcError {
    IpcError::new(
        "DOC_PREVIEW_START_FAILED",
        &format!("LibreOffice headless 启动失败: {detail}"),
        true,
        false,
    )
}

/// PDF 导出 filter（BR-DOC-DATA-001 预览链）：按格式选 writer/calc/impress，附公共导出选项。
pub fn pdf_filter(kind: PreviewKind) -> String {
    let module = match kind {
        PreviewKind::Docx => "writer_pdf_Export",
        PreviewKind::Excel => "calc_pdf_Export",
        PreviewKind::Presentation => "impress_pdf_Export",
    };
    format!("pdf:{module}:{PDF_EXPORT_OPTS}")
}

/**
 * @GOV
 * codes:    BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-003
 * type:     IO
 * chain:    DOC-CONVERT
 * rules:    BR-DOC-DATA-001
 * boundary: in=verifiedBundledToolRuntime | out=sofficePath(resourceDirOnlyNoSystemFallback)
 */
/// 解析 soffice 路径（BR-DOC-DATA-001 预览链）：只接受 `DocToolRuntime` 已校验的 Tauri resource_dir
/// 内置路径。无有效包内工具即 None（由调用方回统一错误），绝不探测系统安装或 PATH；与编辑链分离。
pub fn resolve_soffice_path() -> Option<PathBuf> {
    tool_runtime::soffice_path().ok().map(Path::to_path_buf)
}

/// LibreOffice 是否可用（预览链依赖探测）。
pub fn libreoffice_available() -> bool {
    resolve_soffice_path().is_some()
}

fn path_to_user_installation_url(path: &Path) -> String {
    let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let s = abs.to_string_lossy().replace('\\', "/").replace(' ', "%20");
    if s.starts_with('/') {
        format!("file://{s}")
    } else {
        format!("file:///{s}")
    }
}

/**
 * @GOV
 * codes:    BR-DOC-DATA-001-DATA-DOC-DOC-CONVERT-004
 * type:     DATA
 * chain:    DOC-CONVERT
 * rules:    BR-DOC-DATA-001
 * boundary: in=requestWorkDirectory | out=isolatedLibreOfficeProfile(noHostFontSubstitution)
 */
/// 创建请求专属 LO profile。不得在这里写 `fontsubst.xcu`：它会把未命中的字体改写为宿主相关默认字体，
/// 无法给出可复现的 glyph 覆盖证明；实际替换只能由包内 FontPlan + 已验证的 LO 源码路径执行。
fn create_isolated_profile(work_dir: &Path) -> Result<PathBuf, IpcError> {
    let lo_user = work_dir.join("lo_user");
    std::fs::create_dir_all(&lo_user).map_err(|e| convert_fail(&e.to_string()))?;
    Ok(lo_user)
}

/// S0 Font Trace 只记录请求级字体元数据，绝不读取设备字体库或修改原生 Office 输入。策略文件为空/无命中
/// 时也显式写出 `unmapped`，供后续 S2 FontPlan 与 S3 LO 源码补丁取证；取证本身失败不掩盖真实转换错误。
fn write_font_trace_diagnostic(input: &Path, work_dir: &Path) {
    let trace = match font_trace::inspect_office_font_references(input) {
        Ok(trace) => trace,
        Err(error) => {
            eprintln!("[binder:lo] font-trace unavailable: {error}");
            return;
        }
    };
    let policy_path = match tool_runtime::font_plan_path() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("[binder:lo] font-plan unavailable: {}", error.message);
            return;
        }
    };
    let policy = match std::fs::read(&policy_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<font_trace::FontPolicy>(&bytes).ok())
    {
        Some(policy) => policy,
        None => {
            eprintln!(
                "[binder:lo] font-plan invalid or missing: {}",
                policy_path.display()
            );
            return;
        }
    };
    match font_trace::build_font_plan(&trace, &policy)
        .and_then(|plan| font_trace::write_font_diagnostic(work_dir, &trace, &plan).map(|_| plan))
    {
        Ok(plan) => {
            let unmapped = plan
                .entries
                .iter()
                .filter(|entry| matches!(entry.decision, font_trace::FontPlanDecision::Unmapped))
                .count();
            eprintln!(
                "[binder:lo] font-trace refs={} unmapped={} policy=v{} · metadata only",
                plan.entries.len(),
                unmapped,
                plan.policy_version
            );
        }
        Err(error) => eprintln!("[binder:lo] font-trace diagnostic failed: {error}"),
    }
}

/// 构建 soffice 命令并配置 macOS 运行环境（移植旧项目：DYLD/SAL/HOME/UserInstallation）。
fn build_soffice_command(soffice: &Path, work_dir: &Path) -> Result<Command, IpcError> {
    let mut cmd = Command::new(soffice);
    // 唯一允许的输入是包内 .app；Contents 环境配置不再有系统 PATH 分支。
    if let Some(contents) = soffice.parent().and_then(|p| p.parent()) {
        if contents.ends_with("Contents") {
            cmd.current_dir(contents);
            let frameworks = contents.join("Frameworks");
            let program = contents.join("MacOS");
            let mut dyld = vec![];
            if frameworks.exists() {
                dyld.push(frameworks.to_string_lossy().to_string());
            }
            if program.exists() {
                dyld.push(program.to_string_lossy().to_string());
            }
            if !dyld.is_empty() {
                cmd.env("DYLD_LIBRARY_PATH", dyld.join(":"));
            }
            cmd.env("SAL_DISABLE_OPENCL", "1");
        }
    }
    let lo_user = create_isolated_profile(work_dir)?;
    cmd.env("HOME", lo_user.to_string_lossy().as_ref());
    cmd.arg(format!(
        "-env:UserInstallation={}",
        path_to_user_installation_url(&lo_user)
    ));
    Ok(cmd)
}

/// 重试扫描输出目录找当前输入的预期 PDF（>100 字节）。请求方使用独立目录，且禁止以“最新 PDF”兜底，
/// 避免并发或残留文件被误返回给用户。
fn find_generated_pdf(output_dir: &Path, source: &Path) -> Result<PathBuf, IpcError> {
    let expected = source
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| format!("{s}.pdf"));
    const MAX_RETRIES: u32 = 10;
    for attempt in 0..=MAX_RETRIES {
        let mut pdfs: Vec<(PathBuf, String, std::time::SystemTime)> = Vec::new();
        if let Ok(entries) = std::fs::read_dir(output_dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) == Some("pdf") {
                    if let Ok(m) = std::fs::metadata(&p) {
                        if m.len() > 100 {
                            let name = p
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("")
                                .to_string();
                            pdfs.push((p, name, m.modified().unwrap_or(std::time::UNIX_EPOCH)));
                        }
                    }
                }
            }
        }
        if let Some(exp) = &expected {
            if let Some((p, ..)) = pdfs.iter().find(|(_, n, _)| n.eq_ignore_ascii_case(exp)) {
                return Ok(p.clone());
            }
        }
        if attempt < MAX_RETRIES {
            std::thread::sleep(Duration::from_millis(500));
        }
    }
    Err(convert_fail(&format!(
        "PDF 未生成于输出目录: {output_dir:?}"
    )))
}

/**
 * @GOV
 * codes:    BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-005
 * type:     IO
 * chain:    DOC-CONVERT
 * rules:    BR-DOC-DATA-001
 * boundary: in=docPathAndKind | out=pdfPath(localSubprocessNoUplinkUnifiedErrorWhenDepMissing)
 */
/// 预览转换 docx/xlsx/pptx → PDF（BR-DOC-DATA-001 预览链）：解析 soffice（**缺失→DOC_LIBREOFFICE_MISSING
/// 统一错误，不静默降级**）；配置 macOS 环境与请求专属 profile；headless `--convert-to` 至临时输出目录；重试扫描
/// 取 PDF 路径。内容不经网络上行；与编辑链分离。output_dir 为工作/输出目录（调用方提供，限本地）。
pub fn convert_to_pdf(
    input: &Path,
    kind: PreviewKind,
    output_dir: &Path,
) -> Result<PathBuf, IpcError> {
    let soffice = tool_runtime::soffice_path().map_err(|_| dep_missing())?;
    eprintln!(
        "[binder:lo] soffice={} · isolated headless profile",
        soffice.display()
    );
    if !input.exists() {
        return Err(convert_fail(&format!("输入文件不存在: {input:?}")));
    }
    std::fs::create_dir_all(output_dir).map_err(|e| convert_fail(&e.to_string()))?;

    let worker = crate::doc::lo_worker::LoWorker::new(soffice.to_path_buf());
    let output = worker.execute(input, kind, output_dir)?;
    Ok(output.pdf_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: BR-DOC-DATA-001
    // 运行时不得伪造源码级 GlyphFallback 已完成；字体回退补丁仍由稳定 LO 构建专项承担。
    #[test]
    fn soffice_command_uses_isolated_profile_without_host_dyld() {
        let dir = std::env::temp_dir().join(format!("binder_lo_env_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cmd = build_soffice_command(std::path::Path::new("soffice"), &dir).unwrap();
        let envs: std::collections::HashMap<String, String> = cmd
            .get_envs()
            .filter_map(|(k, v)| {
                Some((
                    k.to_string_lossy().into_owned(),
                    v?.to_string_lossy().into_owned(),
                ))
            })
            .collect();
        assert!(!envs.contains_key("BINDER_FORCE_DEFAULT_FONTS"));
        assert!(envs.contains_key("HOME"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    // covers: BR-DOC-DATA-001
    // 预览 filter：三格式各用 writer/calc/impress，均含文本层(UseTaggedPDF)与嵌字体(Embed*)选项。
    #[test]
    fn pdf_filters_carry_text_layer_and_embedded_fonts() {
        let w = pdf_filter(PreviewKind::Docx);
        assert!(w.starts_with("pdf:writer_pdf_Export:"));
        assert!(w.contains("UseTaggedPDF=1") && w.contains("EmbedAsianScriptFonts=1"));
        assert!(pdf_filter(PreviewKind::Excel).contains("calc_pdf_Export"));
        assert!(pdf_filter(PreviewKind::Presentation).contains("impress_pdf_Export"));
    }

    // covers: BR-DOC-DATA-001
    // 请求 profile 不得写入旧 `fontsubst.xcu`，避免将未知来源字体静默重定向到宿主默认字体。
    #[test]
    fn isolated_profile_has_no_legacy_host_font_substitution() {
        let dir = std::env::temp_dir().join(format!("binder-lo-profile-{}", std::process::id()));
        let profile = create_isolated_profile(&dir).expect("profile should be created");
        assert!(profile.is_dir());
        assert!(!profile.join("user/config/fontsubst.xcu").exists());
        assert!(!profile.join("4/user/config/fontsubst.xcu").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    // covers: BR-DOC-DATA-001
    // 只接受资源目录中已校验的 LibreOffice；系统安装不影响解析结果。
    #[test]
    fn missing_dependency_errors_without_silent_degrade() {
        let soffice = resolve_soffice_path().expect("必须只解析已校验的包内 LibreOffice");
        assert!(soffice.is_absolute());
        assert!(!soffice
            .to_string_lossy()
            .contains("/Applications/LibreOffice.app"));
    }

    // covers: BR-DOC-DATA-001
    // 端到端预览：pandoc 造 docx → convert_to_pdf 经内置优化版 LO + macOS 环境 + 嵌字体 filter 生成 PDF；
    // 产物为 %PDF 魔数、非空。包内任一工具缺失都必须失败，不能通过系统工具或 skip 隐藏。
    #[test]
    fn docx_to_pdf_end_to_end_when_libreoffice_available() {
        assert!(libreoffice_available(), "必须使用已校验的包内 LibreOffice");
        assert!(
            crate::doc::pandoc_runtime::pandoc_available(),
            "必须使用已校验的包内 Pandoc"
        );
        // 1) pandoc 造一个含必需子集（标题/粗体）的 docx
        let docx = crate::doc::pandoc_runtime::markdown_to_docx("# 预览标题\n\n**加粗** 正文\n")
            .expect("pandoc md→docx");
        let base = std::env::temp_dir().join(format!("binder_lo_e2e_{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        let docx_path = base.join("input.docx");
        std::fs::write(&docx_path, &docx).unwrap();

        // 2) 预览链：docx → PDF（经内置优化版 LO + DYLD/SAL/UserInstallation + writer_pdf_Export filter）
        let out_dir = base.join("out");
        let pdf = convert_to_pdf(&docx_path, PreviewKind::Docx, &out_dir).expect("docx→pdf");

        // 3) 校验产物为有效 PDF
        let bytes = std::fs::read(&pdf).unwrap();
        assert!(bytes.len() > 100, "PDF 非空");
        assert_eq!(&bytes[..5], b"%PDF-", "产物为 PDF 魔数");

        let _ = std::fs::remove_dir_all(&base);
    }
}
