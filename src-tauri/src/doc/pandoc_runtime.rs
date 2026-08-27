//! PI-DOC-01（实现期续接）· Pandoc 编辑链运行时 · DOC-CONVERT / BR-DOC-DATA-001
//!
//! 把 convert.rs 的双链分离契约落到真实 Pandoc 子进程：docx↔编辑模型（markdown）本地转换——
//! **内容不经网络上行**（纯本地子进程）；**依赖缺失（包内 Pandoc 不可用）回统一错误形状、不静默降级**
//! （BR-DOC-DATA-001）；转换失败回统一错误、不产半成品（临时文件即用即清）。编辑链用 Pandoc，预览链
//! 用 LibreOffice（未安装，预览链 runtime 待环境就绪）。blockId 兜底归 block_backfill、样式分级归
//! style_subset；本模块只负责真实字节流转换。docx 为二进制经临时文件，markdown 经 stdin/stdout。

use crate::{doc::tool_runtime, ipc::IpcError};
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static SEQ: AtomicU64 = AtomicU64::new(0);

fn temp_path(ext: &str) -> std::path::PathBuf {
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("binder_pandoc_{t}_{n}.{ext}"))
}

/// Pandoc 是否作为已校验的 Binder 包内工具可用；不探测 PATH 或设备安装版本。
pub fn pandoc_available() -> bool {
    tool_runtime::pandoc_path().is_ok()
}

/**
 * @GOV
 * codes:    BR-DOC-DATA-001-DATA-DOC-DOC-CONVERT-012
 * type:     DATA
 * chain:    DOC-CONVERT
 * rules:    BR-DOC-DATA-001
 * boundary: in=conversionFromTo | out=pandocArgs(localPipelineNoNetwork)
 */
/// pandoc 转换参数构造（BR-DOC-DATA-001）：本地管线 `-f from -t to`，无任何网络/上行参数。纯函数，可测。
pub fn pandoc_args(from: &str, to: &str) -> Vec<String> {
    vec!["-f".into(), from.into(), "-t".into(), to.into()]
}

fn dep_missing() -> IpcError {
    IpcError::new(
        "DOC_PANDOC_MISSING",
        "Pandoc 依赖缺失，明确报错不静默降级",
        true,
        false,
    )
}
fn convert_fail(detail: &str) -> IpcError {
    IpcError::new(
        "DOC_CONVERT_FAIL",
        &format!("Pandoc 转换失败: {detail}"),
        true,
        false,
    )
}

/**
 * @GOV
 * codes:    BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-001
 * type:     IO
 * chain:    DOC-CONVERT
 * rules:    BR-DOC-DATA-001
 * boundary: in=docxBytes | out=editableMarkdown(localSubprocessNoUplinkUnifiedErrorOnFail)
 */
/// docx → 编辑模型 markdown（BR-DOC-DATA-001 编辑链）：docx 二进制写临时文件，pandoc 本地转 markdown；
/// 依赖缺失/转换失败回统一错误形状（不静默降级、不产半成品）；临时文件即用即清。内容不经网络上行。
pub fn docx_to_markdown(docx: &[u8]) -> Result<String, IpcError> {
    let pandoc = tool_runtime::pandoc_path().map_err(|_| dep_missing())?;
    let input = temp_path("docx");
    std::fs::write(&input, docx).map_err(|e| convert_fail(&e.to_string()))?;
    let out = Command::new(pandoc)
        .arg(&input)
        .args(["-t", "markdown"])
        .output();
    let _ = std::fs::remove_file(&input); // 即用即清
    let out = out.map_err(|_| dep_missing())?;
    if !out.status.success() {
        return Err(convert_fail(&String::from_utf8_lossy(&out.stderr)));
    }
    String::from_utf8(out.stdout).map_err(|e| convert_fail(&e.to_string()))
}

/**
 * @GOV
 * codes:    BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-002
 * type:     IO
 * chain:    DOC-CONVERT
 * rules:    BR-DOC-DATA-001
 * boundary: in=editableMarkdown | out=docxBytes(localSubprocessNoUplinkUnifiedErrorOnFail)
 */
/// 编辑模型 markdown → docx（BR-DOC-DATA-001 编辑链）：markdown 经 stdin，pandoc 本地生成 docx 临时文件，
/// 读回字节；依赖缺失/失败回统一错误（不静默降级）；临时文件即用即清。内容不经网络上行。
pub fn markdown_to_docx(markdown: &str) -> Result<Vec<u8>, IpcError> {
    let pandoc = tool_runtime::pandoc_path().map_err(|_| dep_missing())?;
    let output = temp_path("docx");
    let mut child = Command::new(pandoc)
        .args(["-f", "markdown", "-t", "docx", "-o"])
        .arg(&output)
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| dep_missing())?;
    child
        .stdin
        .take()
        .ok_or_else(|| convert_fail("无法写入 stdin"))?
        .write_all(markdown.as_bytes())
        .map_err(|e| convert_fail(&e.to_string()))?;
    let status = child.wait().map_err(|e| convert_fail(&e.to_string()))?;
    if !status.success() {
        let _ = std::fs::remove_file(&output);
        return Err(convert_fail("pandoc 非零退出"));
    }
    let bytes = std::fs::read(&output).map_err(|e| convert_fail(&e.to_string()));
    let _ = std::fs::remove_file(&output); // 即用即清
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: BR-DOC-DATA-001
    // pandoc 参数为本地管线（-f/-t），无网络/上行参数。
    #[test]
    fn pandoc_args_are_local_pipeline() {
        assert_eq!(
            pandoc_args("docx", "markdown"),
            vec!["-f", "docx", "-t", "markdown"]
        );
        let a = pandoc_args("markdown", "docx");
        assert!(
            !a.iter().any(|x| x.contains("http") || x.contains("://")),
            "无网络参数"
        );
    }

    // covers: BR-DOC-DATA-002
    // 必需子集往返保真：标题/粗体经 markdown→docx→markdown 保留。包内 Pandoc 缺失即测试失败。
    #[test]
    fn required_subset_roundtrip_when_pandoc_available() {
        assert!(pandoc_available(), "必须使用已校验的包内 Pandoc");
        let md = "# 标题\n\n**加粗** 文本\n";
        let docx = markdown_to_docx(md).expect("md→docx");
        assert!(!docx.is_empty());
        // docx 是 zip（OOXML），魔数 PK
        assert_eq!(&docx[..2], b"PK", "docx 应为 OOXML zip");
        let back = docx_to_markdown(&docx).expect("docx→md");
        assert!(back.contains("标题"), "必需子集：标题往返保真");
        assert!(back.contains("加粗"), "必需子集：粗体内容往返保真");
    }
}
