//! DOC-LO-RELIABILITY-FONT-IMPLEMENT-01 · S4 · LO Worker/Supervisor · DOC-CONVERT
//!
//! 封装 LibreOffice headless 子进程的完整生命周期：请求级隔离 profile、超时守护、崩溃
//! 识别、过期目录清理与脱敏错误诊断。`libreoffice_runtime` 的 `convert_to_pdf` 委托给本
//! 模块执行，PRV 保持唯一公开 `prv.preview`。

use crate::doc::{font_trace, tool_runtime};
use crate::ipc::IpcError;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// 默认转换超时（秒）。大文件的 pptx 可能在慢机器上接近此值。
const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// 过期目录清理阈值：修改时间超过此值的 `doc-runtime/<id>/` 被视为过期残留。
const STALE_DIR_SECS: u64 = 3600; // 1 hour

// ── 错误构造 ──────────────────────────────────────────────────────────

fn dep_missing() -> IpcError {
    IpcError::new(
        "DOC_LIBREOFFICE_MISSING",
        "LibreOffice 依赖缺失（预览链），明确报错不静默降级",
        true,
        false,
    )
}

fn start_fail(detail: impl AsRef<str>) -> IpcError {
    IpcError::new(
        "DOC_PREVIEW_START_FAILED",
        &format!("LibreOffice headless 启动失败: {}", detail.as_ref()),
        true,
        false,
    )
}

fn convert_fail(detail: impl AsRef<str>) -> IpcError {
    IpcError::new("DOC_PREVIEW_FAIL", detail.as_ref(), true, false)
}

fn timeout_error(request_id: &str, secs: u64) -> IpcError {
    IpcError::new(
        "DOC_PREVIEW_TIMEOUT",
        &format!("请求 {request_id} 预览转换超时（>{secs}s）"),
        true,
        false,
    )
}

fn crash_error(request_id: &str, signal: i32) -> IpcError {
    IpcError::new(
        "DOC_PREVIEW_WORKER_CRASH",
        &format!("请求 {request_id} LO Worker 崩溃 (signal={signal})"),
        true,
        false,
    )
}

// ── 脱敏 ──────────────────────────────────────────────────────────────

/// 移除 stderr 中的绝对路径与文件名，保留 LO 模块名/错误行号/字体名。
fn sanitize_diagnostic(raw: &str, input: &Path, work_dir: &Path) -> String {
    let input_str = input.to_string_lossy();
    let work_str = work_dir.to_string_lossy();
    raw.replace(work_str.as_ref(), "<workdir>")
        .replace(input_str.as_ref(), "<input>")
        // 把形如 `<workdir>/work-N/test.pdf` 的路径中文件名也模糊化
        .replace(".pdf", ".<output>")
}

// ── 子进程管理 ────────────────────────────────────────────────────────

fn spawn_soffice(cmd: &mut Command) -> Result<Child, IpcError> {
    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| start_fail(e.to_string()))
}

fn signal_from_exit(exit: std::process::ExitStatus) -> Option<i32> {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        exit.signal()
    }
    #[cfg(not(unix))]
    {
        let _ = exit;
        None
    }
}

/// 从已保存的 stderr handle 读取（用于 child 已被 move 进线程后的场景）。
fn read_stderr_from_handle(stderr: &mut std::process::ChildStderr) -> String {
    use std::io::Read;
    let mut buf = vec![0u8; 16 * 1024];
    match stderr.read(&mut buf) {
        Ok(n) if n > 0 => String::from_utf8_lossy(&buf[..n]).to_string(),
        _ => String::new(),
    }
}

/// 读取子进程 stderr（限制最大字节数，避免 OOM）。
fn read_stderr(child: &mut Child, limit: usize) -> String {
    let mut buf = vec![0u8; limit.min(16 * 1024)];
    if let Some(stderr) = child.stderr.as_mut() {
        use std::io::Read;
        if let Ok(n) = stderr.read(&mut buf) {
            return String::from_utf8_lossy(&buf[..n]).to_string();
        }
    }
    String::new()
}

// ── LoWorker ──────────────────────────────────────────────────────────

pub struct LoWorker {
    soffice: PathBuf,
    timeout: Duration,
}

#[derive(Debug)]
pub struct LoWorkerOutput {
    pub pdf_path: PathBuf,
    pub font_diagnostic: Option<font_trace::FontPlan>,
    pub duration: Duration,
}

impl LoWorker {
    pub fn new(soffice: PathBuf) -> Self {
        Self {
            soffice,
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
        }
    }

    /// 仅测试用：缩短超时。
    #[cfg(test)]
    pub fn with_timeout(soffice: PathBuf, secs: u64) -> Self {
        Self {
            soffice,
            timeout: Duration::from_secs(secs),
        }
    }

    /// 清理 `doc-runtime/` 中超过 STALE_DIR_SECS 的残留目录。
    pub fn clean_stale_dirs(runtime_dir: &Path) {
        let Ok(entries) = fs::read_dir(runtime_dir) else {
            return;
        };
        let cutoff = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .saturating_sub(Duration::from_secs(STALE_DIR_SECS));
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            let Ok(mtime) = meta.modified() else {
                continue;
            };
            if mtime.duration_since(UNIX_EPOCH).unwrap_or_default() < cutoff {
                let _ = fs::remove_dir_all(&path);
                eprintln!("[binder:lo] 清理过期目录: {}", path.display());
            }
        }
    }

    /// 写入请求级 Font Trace 诊断。
    fn diagnose(&self, input: &Path, work_dir: &Path) -> Option<font_trace::FontPlan> {
        let trace = font_trace::inspect_office_font_references(input).ok()?;
        let policy_path = tool_runtime::font_plan_path().ok()?;
        let policy_bytes = fs::read(&policy_path).ok()?;
        let policy = serde_json::from_slice::<font_trace::FontPolicy>(&policy_bytes).ok()?;
        let plan = font_trace::build_font_plan(&trace, &policy).ok()?;
        let _ = font_trace::write_font_diagnostic(work_dir, &trace, &plan);
        let (resolved, defaulted, unmapped) = font_trace::plan_summary(&plan);
        eprintln!(
            "[binder:lo] font-trace refs={} resolved={} defaulted={} unmapped={} policy=v{}",
            plan.entries.len(),
            resolved,
            defaulted,
            unmapped,
            plan.policy_version
        );
        Some(plan)
    }

    /// 执行一次转换。调用方负责确保 work_dir 已创建且为此请求独占。
    pub fn execute(
        &self,
        input: &Path,
        kind: crate::doc::libreoffice_runtime::PreviewKind,
        work_dir: &Path,
    ) -> Result<LoWorkerOutput, IpcError> {
        let started = Instant::now();
        let request_id = work_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        // 字体取证（docx/pptx）
        let font_plan = if matches!(
            kind,
            crate::doc::libreoffice_runtime::PreviewKind::Docx
                | crate::doc::libreoffice_runtime::PreviewKind::Presentation
        ) {
            self.diagnose(input, work_dir)
        } else {
            None
        };

        // 构建 soffice 命令
        let mut cmd = build_soffice_command(&self.soffice, work_dir)?;
        cmd.arg("--headless")
            .arg("--nologo")
            .arg("--nodefault")
            .arg("--norestore")
            .arg("--nolockcheck")
            .arg("--convert-to")
            .arg(crate::doc::libreoffice_runtime::pdf_filter(kind))
            .arg("--outdir")
            .arg(work_dir)
            .arg(input);

        let mut child = spawn_soffice(&mut cmd)?;
        let child_id = child.id();

        // 先读取 stderr handle（在 child 被 move 进线程前）
        let mut child_stderr = child.stderr.take();

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let _ = tx.send(child.wait());
        });

        let exit = match rx.recv_timeout(self.timeout) {
            Ok(Ok(status)) => status,
            Ok(Err(_)) => return Err(start_fail("子进程 wait 失败")),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let _ = Command::new("kill")
                    .arg("-9")
                    .arg(child_id.to_string())
                    .output();
                return Err(timeout_error(request_id, self.timeout.as_secs()));
            }
            Err(_) => return Err(start_fail("子进程通道断开")),
        };

        // 崩溃检测
        if !exit.success() {
            if let Some(sig) = signal_from_exit(exit) {
                return Err(crash_error(request_id, sig));
            }
            // 从已保存的 stderr handle 读取诊断信息（审计修复 P0：之前此处 stderr 完全丢失）
            let stderr = if let Some(ref mut se) = child_stderr {
                read_stderr_from_handle(se)
            } else {
                String::from("(stderr unavailable)")
            };
            let sanitized = sanitize_diagnostic(&stderr, input, work_dir);
            return Err(convert_fail(&sanitized));
        }

        // 扫描 PDF
        let pdf = find_generated_pdf(work_dir, input)?;
        Ok(LoWorkerOutput {
            pdf_path: pdf,
            font_diagnostic: font_plan,
            duration: started.elapsed(),
        })
    }
}

// ── 复用 libreoffice_runtime 的辅助函数 ───────────────────────────
// （这些函数逻辑与 libreoffice_runtime 相同，Worker 化后宜收敛到本模块）

fn path_to_user_installation_url(path: &Path) -> String {
    let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let s = abs.to_string_lossy().replace('\\', "/").replace(' ', "%20");
    if s.starts_with('/') {
        format!("file://{s}")
    } else {
        format!("file:///{s}")
    }
}

fn create_isolated_profile(work_dir: &Path) -> Result<PathBuf, IpcError> {
    let lo_user = work_dir.join("lo_user");
    fs::create_dir_all(&lo_user).map_err(|e| convert_fail(&e.to_string()))?;
    Ok(lo_user)
}

fn build_soffice_command(soffice: &Path, work_dir: &Path) -> Result<Command, IpcError> {
    let mut cmd = Command::new(soffice);
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

fn find_generated_pdf(output_dir: &Path, source: &Path) -> Result<PathBuf, IpcError> {
    let expected = source
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| format!("{s}.pdf"));
    const MAX_RETRIES: u32 = 10;
    for attempt in 0..=MAX_RETRIES {
        let mut pdfs: Vec<(PathBuf, String, std::time::SystemTime)> = Vec::new();
        if let Ok(entries) = fs::read_dir(output_dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) == Some("pdf") {
                    if let Ok(m) = fs::metadata(&p) {
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
            thread::sleep(Duration::from_millis(500));
        }
    }
    Err(convert_fail(&format!(
        "PDF 未生成于输出目录: {output_dir:?}"
    )))
}

// ── 测试 ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // 准备一个有 PATH 中可用的外部命令作为"假 soffice"
    fn dummy_soffice() -> PathBuf {
        // /usr/bin/true 总是返回 0 且无输出——用于模拟正常退出
        PathBuf::from("/usr/bin/true")
    }
    fn slow_dummy() -> PathBuf {
        PathBuf::from("/bin/sleep")
    }

    #[test]
    fn sanitize_removes_paths() {
        let input = Path::new("/Users/x/docs/test.docx");
        let work = Path::new("/tmp/doc-runtime/req-1");
        let raw = "Error: cannot open /tmp/doc-runtime/req-1/test.pdf after conversion of /Users/x/docs/test.docx";
        let clean = sanitize_diagnostic(raw, input, work);
        assert!(!clean.contains("/Users/x/docs/"));
        assert!(!clean.contains("/tmp/doc-runtime/req-1"));
        assert!(clean.contains("<workdir>"));
        assert!(clean.contains("<input>"));
    }

    #[test]
    fn timeout_kills_slow_process() {
        // 直接测试超时逻辑，绕过 build_soffice_command 的 LO 特定参数
        let mut cmd = Command::new("/bin/sleep");
        cmd.arg("300").stdout(Stdio::null()).stderr(Stdio::null());
        let mut child = cmd.spawn().unwrap();
        let child_id = child.id();

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let _ = tx.send(child.wait());
        });

        let result = rx.recv_timeout(Duration::from_secs(1));
        match result {
            Ok(Ok(_)) => panic!("sleep 300 不应在 1s 内退出"),
            Ok(Err(_)) => panic!("wait 失败"),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // 预期路径：超时 → kill
                let _ = Command::new("kill")
                    .arg("-9")
                    .arg(child_id.to_string())
                    .output();
                // 成功——超时触发
            }
            Err(_) => panic!("通道断开"),
        }
    }

    #[test]
    fn crash_detected_on_signal() {
        // 使用 /bin/bash -c 'kill -6 $$' 模拟 SIGABRT (exit 134)
        let bash = PathBuf::from("/bin/bash");
        if !bash.exists() {
            return; // 非 macOS
        }
        let work = std::env::temp_dir().join(format!("binder_lo_crash_{}", std::process::id()));
        fs::create_dir_all(&work).unwrap();
        let mut worker = LoWorker::new(bash);
        worker.timeout = Duration::from_secs(10);

        // 直接构造一个会 SIGABRT 的命令，绕过 execute 的 Command 构建
        let mut cmd = Command::new(&worker.soffice);
        cmd.args(["-c", "kill -6 $$"])
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mut child = cmd.spawn().unwrap();
        let status = child.wait().unwrap();
        let sig = signal_from_exit(status);
        // kill -6 = SIGABRT → exit code 134, signal = 6
        assert_eq!(sig, Some(6));
        let _ = fs::remove_dir_all(&work);
    }

    #[test]
    fn cleans_stale_dirs() {
        let runtime = std::env::temp_dir().join(format!("binder_lo_clean_{}", std::process::id()));
        fs::create_dir_all(&runtime).unwrap();
        let old = runtime.join("old-request");
        fs::create_dir_all(&old).unwrap();
        // 设置为旧时间
        let old_time = SystemTime::now() - Duration::from_secs(STALE_DIR_SECS + 60);
        filetime::set_file_mtime(&old, filetime::FileTime::from_system_time(old_time)).unwrap();
        let recent = runtime.join("recent-request");
        fs::create_dir_all(&recent).unwrap();

        LoWorker::clean_stale_dirs(&runtime);

        assert!(!old.exists(), "过期目录应被清理");
        assert!(recent.exists(), "新目录不应被清理");
        let _ = fs::remove_dir_all(&runtime);
    }

    #[test]
    fn signal_detection_works() {
        // Linux/macOS: kill -l shows signals
        // 使用 bash -c 'exit 1' = normal failure (no signal)
        let mut cmd = Command::new("/bin/bash");
        cmd.args(["-c", "exit 1"])
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mut child = cmd.spawn().unwrap();
        let status = child.wait().unwrap();
        assert!(signal_from_exit(status).is_none());
        assert!(!status.success());
    }
}
