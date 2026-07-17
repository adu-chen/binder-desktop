//! PI-DOC-01/03（实现期续接）· LibreOffice 预览链运行时 · DOC-CONVERT / BR-DOC-DATA-001
//!
//! 把 convert.rs 的双链分离契约落到真实 LibreOffice headless 子进程：docx/xlsx/pptx → PDF 预览——
//! **内容不经网络上行**（纯本地子进程）；**依赖缺失（soffice 不可用）回统一错误形状、不静默降级**
//! （BR-DOC-DATA-001）；编辑链用 Pandoc（pandoc_runtime）、预览链用 LibreOffice（本模块），互不共享中间态。
//!
//! 处理逻辑移植并优化自旧项目 binder-project 的 libreoffice_service（同一处理逻辑）：
//! - **内置优先路径解析**：TAURI_RESOURCE_DIR → 打包 .app/Contents/Resources → 开发 src-tauri/resources
//!   → 系统 /Applications / PATH，内置形如 `libreoffice/LibreOffice.app/Contents/MacOS/soffice`。
//! - **macOS 运行环境**（使内置 LO 自包含运行）：current_dir=Contents、DYLD_LIBRARY_PATH=Frameworks+MacOS、
//!   SAL_USE_VCLPLUGIN=gen、SAL_DISABLE_OPENCL=1、HOME=lo_user、`-env:UserInstallation=file://…/lo_user`。
//! - **转换 filter**（含文本层与嵌字体，确保可复制/字体一致）：
//!   `pdf:{writer|calc|impress}_pdf_Export:UseTaggedPDF=1:SelectPdfVersion=1:EmbedStandardFonts=1:
//!   EmbedLatinScriptFonts=1:EmbedAsianScriptFonts=1`。
//! - **字体替换** fontsubst.xcu：文档常见 CJK/Latin 字体缺失时统一替换为默认（macOS: PingFang SC / Arial），
//!   避免随机回退。
//! - **重试扫描找 PDF**：精确名匹配优先，否则取最新；>100 字节。
//! LO 资源打包（deploy 脚本）与缓存策略为部署/实现期；本模块落处理逻辑与依赖守卫。

use crate::ipc::IpcError;
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

/// macOS 内置 LibreOffice 相对可执行路径。
const MACOS_SOFFICE_REL: &str = "LibreOffice.app/Contents/MacOS/soffice";

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

/// PDF 导出 filter（BR-DOC-DATA-001 预览链）：按格式选 writer/calc/impress，附公共导出选项。
pub fn pdf_filter(kind: PreviewKind) -> String {
    let module = match kind {
        PreviewKind::Docx => "writer_pdf_Export",
        PreviewKind::Excel => "calc_pdf_Export",
        PreviewKind::Presentation => "impress_pdf_Export",
    };
    format!("pdf:{module}:{PDF_EXPORT_OPTS}")
}

/// 默认替换字体（移植旧项目；macOS: PingFang SC / Arial）。
pub fn default_fonts() -> (&'static str, &'static str) {
    #[cfg(target_os = "macos")]
    {
        ("PingFang SC", "Arial")
    }
    #[cfg(target_os = "windows")]
    {
        ("Microsoft YaHei", "Arial")
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        ("WenQuanYi Micro Hei", "Arial")
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// 从指定目录取内置 soffice 路径（macOS：libreoffice/LibreOffice.app/Contents/MacOS/soffice）。
fn soffice_in_dir(libreoffice_dir: &Path) -> Option<PathBuf> {
    let p = libreoffice_dir.join(MACOS_SOFFICE_REL);
    if p.exists() {
        Some(p)
    } else {
        None
    }
}

/**
 * @GOV
 * codes:    BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-003
 * type:     IO
 * chain:    DOC-CONVERT
 * rules:    BR-DOC-DATA-001
 * boundary: in=environmentProbe | out=sofficePath(bundledFirstThenSystemNoneWhenAbsent)
 */
/// 解析 soffice 路径（BR-DOC-DATA-001 预览链）：**内置优先**（TAURI_RESOURCE_DIR → 打包 .app/Contents/
/// Resources → 开发 src-tauri/resources）→ 系统（/Applications / PATH）。无则 None（由调用方回统一错误，
/// 不静默降级）。与编辑链（Pandoc）分离。
pub fn resolve_soffice_path() -> Option<PathBuf> {
    // 1) 内置：TAURI_RESOURCE_DIR/libreoffice
    if let Ok(res) = std::env::var("TAURI_RESOURCE_DIR") {
        if let Some(p) = soffice_in_dir(&PathBuf::from(res).join("libreoffice")) {
            return Some(p);
        }
    }
    // 2) 内置：打包 .app/Contents/Resources/libreoffice（current_exe → Contents → Resources）
    if let Ok(exe) = std::env::current_exe() {
        if let Some(contents) = exe.parent().and_then(|p| p.parent()) {
            if contents.ends_with("Contents") {
                if let Some(p) = soffice_in_dir(&contents.join("Resources").join("libreoffice")) {
                    return Some(p);
                }
            }
        }
    }
    // 3) 内置：开发模式 src-tauri/resources/libreoffice 或 resources/libreoffice
    if let Ok(cwd) = std::env::current_dir() {
        for d in [
            cwd.join("src-tauri/resources/libreoffice"),
            cwd.join("resources/libreoffice"),
        ] {
            if let Some(p) = soffice_in_dir(&d) {
                return Some(p);
            }
        }
    }
    // 4) 系统：/Applications/LibreOffice.app
    for sys in [
        "/Applications/LibreOffice.app/Contents/MacOS/soffice",
        "/Applications/LibreOffice.app/Contents/MacOS/soffice.bin",
    ] {
        let p = PathBuf::from(sys);
        if p.exists() {
            return Some(p);
        }
    }
    // 5) 系统：PATH 中的 soffice（--version 探测）
    if Command::new("soffice")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return Some(PathBuf::from("soffice"));
    }
    None
}

/// LibreOffice 是否可用（预览链依赖探测）。
pub fn libreoffice_available() -> bool {
    resolve_soffice_path().is_some()
}

/**
 * @GOV
 * codes:    BR-DOC-DATA-001-DATA-DOC-DOC-CONVERT-004
 * type:     DATA
 * chain:    DOC-CONVERT
 * rules:    BR-DOC-DATA-001
 * boundary: in=defaultFonts | out=fontSubstXcu(missingCjkLatinReplacedNoRandomFallback)
 */
/// 生成 fontsubst.xcu（BR-DOC-DATA-001 预览链一致性）：文档常见 CJK/Latin 字体缺失时统一替换为默认字体，
/// 避免 LO 对缺失字体随机回退（移植旧项目字体列表）。纯函数，可测。
pub fn generate_fontsubst_content(cjk_font: &str, latin_font: &str) -> String {
    // 机制（旧项目注释语义）：仅对「可能缺失」的字体配置替换——LO 在字体缺失时用 ReplaceWith，
    // 已安装字体仍用原字体渲染；**未列出的缺失字体会被 LO 随机回退**（pptx 字体随机匹配根因），
    // 替换表须尽量覆盖文档常见字体（2026-07-03 #2 修复：补齐旧项目全集，CJK 24→36 / Latin +1）。
    const CJK: &[&str] = &[
        "宋体",
        "SimSun",
        "黑体",
        "SimHei",
        "楷体",
        "KaiTi",
        "微软雅黑",
        "Microsoft YaHei",
        "等线",
        "DengXian",
        "仿宋",
        "FangSong",
        "仿宋_GB2312",
        "楷体_GB2312",
        "华文黑体",
        "华文楷体",
        "华文宋体",
        "华文仿宋",
        "华文中宋",
        "STHeiti",
        "STKaiti",
        "STSong",
        "STFangsong",
        "STXihei",
        "Hiragino Sans GB",
        "冬青黑体",
        "苹方",
        "PingFang TC",
        "PingFang HK",
        "隶书",
        "幼圆",
        "新宋体",
        "NSimSun",
        "方正兰亭黑",
        "思源黑体",
        "Source Han Sans SC",
    ];
    const LATIN: &[&str] = &[
        "Times New Roman",
        "Calibri",
        "Helvetica",
        "Arial Narrow",
        "Cambria",
        "Georgia",
        "Verdana",
        "Tahoma",
        "Segoe UI",
    ];
    let mut items = String::new();
    for (list, target) in [(CJK, cjk_font), (LATIN, latin_font)] {
        for name in list {
            items.push_str(&format!(
                r#"      <item oor:path="/org.openoffice.Office.Common/Font/Substitution">
        <prop oor:name="FontName" oor:type="xs:string"><value>{}</value></prop>
        <prop oor:name="ReplaceWith" oor:type="xs:string"><value>{}</value></prop>
      </item>
"#,
                escape_xml(name),
                escape_xml(target)
            ));
        }
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<oor:component-data xmlns:oor="http://openoffice.org/2001/registry" xmlns:xs="http://www.w3.org/2001/XMLSchema" oor:name="Common" oor:package="org.openoffice.Office">
  <node oor:name="Font">
    <node oor:name="Substitution">
{items}    </node>
  </node>
</oor:component-data>"#
    )
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

/// 写字体替换配置到 lo_user profile（移植旧项目：同写 user/config 与 4/user/config）。
fn write_font_substitution(work_dir: &Path) -> Result<PathBuf, IpcError> {
    let lo_user = work_dir.join("lo_user");
    let (cjk, latin) = default_fonts();
    let content = generate_fontsubst_content(cjk, latin);
    for sub in ["user/config", "4/user/config"] {
        let dir = lo_user.join(sub);
        std::fs::create_dir_all(&dir).map_err(|e| convert_fail(&e.to_string()))?;
        std::fs::write(dir.join("fontsubst.xcu"), &content)
            .map_err(|e| convert_fail(&e.to_string()))?;
    }
    Ok(lo_user)
}

/// 构建 soffice 命令并配置 macOS 运行环境（移植旧项目：DYLD/SAL/HOME/UserInstallation）。
fn build_soffice_command(soffice: &Path, work_dir: &Path) -> Result<Command, IpcError> {
    let mut cmd = Command::new(soffice);
    // 仅内置 .app 形态需要 Contents 环境配置；系统 PATH 的 soffice（裸名）跳过。
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
            if let Ok(existing) = std::env::var("DYLD_LIBRARY_PATH") {
                if !existing.is_empty() {
                    dyld.push(existing);
                }
            }
            if !dyld.is_empty() {
                cmd.env("DYLD_LIBRARY_PATH", dyld.join(":"));
            }
            cmd.env("SAL_USE_VCLPLUGIN", "gen");
            cmd.env("SAL_DISABLE_OPENCL", "1");
        }
    }
    let lo_user = write_font_substitution(work_dir)?;
    cmd.env("HOME", lo_user.to_string_lossy().as_ref());
    cmd.arg(format!(
        "-env:UserInstallation={}",
        path_to_user_installation_url(&lo_user)
    ));
    // 激活 Binder VCL 字体回退补丁（2026-07-03 #2 根治）：内置优化版 LO 的 vcl/PhysicalFontCollection.cxx
    // 含源码级补丁——**匹配失败的字体统一回退到指定默认字体**（拦截 LO 原生"随缘 fallback"，字体随机
    // 匹配的根因）。补丁由 BINDER_FORCE_DEFAULT_FONTS 门控（不设则行为同上游 LO，此前一直休眠）。
    // fontsubst.xcu 替换表保留为第一道防线（已知字体名显式替换）；本补丁兜住未列出的一切缺失字体。
    // 注意：系统 PATH 的 stock LO 无此补丁（开发期系统兜底时环境变量被忽略，行为退化为 stock 随机回退）。
    let (cjk, latin) = default_fonts();
    cmd.env("BINDER_FORCE_DEFAULT_FONTS", "1");
    cmd.env("BINDER_DEFAULT_CJK_FONT", cjk);
    cmd.env("BINDER_DEFAULT_LATIN_FONT", latin);
    Ok(cmd)
}

/// 重试扫描输出目录找生成的 PDF（移植旧项目：精确名优先，否则最新；>100 字节）。
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
        if !pdfs.is_empty() {
            pdfs.sort_by(|a, b| b.2.cmp(&a.2));
            return Ok(pdfs[0].0.clone());
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
/// 统一错误，不静默降级**）；配置 macOS 环境 + 字体替换；headless `--convert-to` 至临时输出目录；重试扫描
/// 取 PDF 路径。内容不经网络上行；与编辑链分离。output_dir 为工作/输出目录（调用方提供，限本地）。
pub fn convert_to_pdf(
    input: &Path,
    kind: PreviewKind,
    output_dir: &Path,
) -> Result<PathBuf, IpcError> {
    let soffice = resolve_soffice_path().ok_or_else(dep_missing)?;
    // 可观测性（2026-07-03 #2 复测辅助）：预览转换低频，明示实际命中的 soffice 与字体补丁激活状态
    // ——复测字体问题时据此分流「未走内置 LO」vs「字形级回退（补丁未覆盖，需 LO 二次改造）」。
    eprintln!(
        "[binder:lo] soffice={} · BINDER_FORCE_DEFAULT_FONTS=1 已激活（VCL family 级回退补丁；字形级 GlyphFallback 不在补丁范围）",
        soffice.display()
    );
    if !input.exists() {
        return Err(convert_fail(&format!("输入文件不存在: {input:?}")));
    }
    std::fs::create_dir_all(output_dir).map_err(|e| convert_fail(&e.to_string()))?;
    let mut cmd = build_soffice_command(&soffice, output_dir)?;
    cmd.arg("--headless")
        .arg("--convert-to")
        .arg(pdf_filter(kind))
        .arg("--outdir")
        .arg(output_dir)
        .arg(input);
    let out = cmd.output().map_err(|_| dep_missing())?;
    if !out.status.success() {
        return Err(convert_fail(&String::from_utf8_lossy(&out.stderr)));
    }
    find_generated_pdf(output_dir, input)
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: BR-DOC-DATA-001
    // Binder VCL 字体回退补丁激活（2026-07-03 #2 根治）：soffice 命令必须携带门控环境变量——
    // 缺失字体统一回退默认字体（补丁在内置 LO 二进制内，不设变量即休眠=随机回退回归）。
    #[test]
    fn soffice_command_activates_binder_font_fallback() {
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
        assert_eq!(
            envs.get("BINDER_FORCE_DEFAULT_FONTS").map(String::as_str),
            Some("1")
        );
        let (cjk, latin) = default_fonts();
        assert_eq!(
            envs.get("BINDER_DEFAULT_CJK_FONT").map(String::as_str),
            Some(cjk)
        );
        assert_eq!(
            envs.get("BINDER_DEFAULT_LATIN_FONT").map(String::as_str),
            Some(latin)
        );
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
    // 字体替换配置：常见缺失 CJK/Latin → 默认字体（避免随机回退），XML 结构完整。
    #[test]
    fn fontsubst_maps_missing_fonts_to_default() {
        let xcu = generate_fontsubst_content("PingFang SC", "Arial");
        assert!(xcu.contains("<value>宋体</value>"));
        assert!(xcu.contains("<value>PingFang SC</value>")); // CJK 缺失替换目标
        assert!(xcu.contains("<value>Times New Roman</value>"));
        assert!(xcu.contains("<value>Arial</value>")); // Latin 缺失替换目标
        assert!(xcu.contains("Font/Substitution"));
    }

    // covers: BR-DOC-DATA-001
    // 依赖缺失明确报错不静默降级：无 LibreOffice 时 convert_to_pdf 回 DOC_LIBREOFFICE_MISSING 统一错误。
    #[test]
    fn missing_dependency_errors_without_silent_degrade() {
        if libreoffice_available() {
            return; // 环境装有 LO：跳过缺失分支断言（缺失路径在无 LO 环境验证）
        }
        let out = std::env::temp_dir().join("binder_lo_test_out");
        let err =
            convert_to_pdf(Path::new("/nonexistent/x.docx"), PreviewKind::Docx, &out).unwrap_err();
        assert_eq!(err.code, "DOC_LIBREOFFICE_MISSING");
        assert!(err.retriable); // 安装后可重试
    }

    // covers: BR-DOC-DATA-001
    // 端到端预览（LibreOffice 部署后真跑）：pandoc 造 docx → convert_to_pdf 经内置优化版 LO + macOS 环境
    // + 嵌字体 filter 生成 PDF；产物为 %PDF 魔数、非空。无 LO 环境自动跳过（不误判）。
    #[test]
    fn docx_to_pdf_end_to_end_when_libreoffice_available() {
        if !libreoffice_available() || !crate::doc::pandoc_runtime::pandoc_available() {
            return; // 缺 LO 或 pandoc：跳过端到端实测
        }
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
