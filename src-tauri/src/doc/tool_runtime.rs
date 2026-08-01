//! DOC-BUNDLED-RUNTIME-01 · DOC 转换工具的包内唯一解析与完整性校验。
//!
//! Pandoc、LibreOffice 只能从 Tauri resource_dir 中由 manifest 锁定的路径执行。这里刻意没有 PATH、
//! /Applications、当前工作目录或系统字体的回退逻辑：资源失效必须让转换明确失败，不能改变输出来源。

use crate::ipc::IpcError;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{BufReader, Read},
    path::{Path, PathBuf},
    sync::OnceLock,
};
use tauri::{AppHandle, Manager};

static TOOL_RUNTIME: OnceLock<Result<DocToolRuntime, String>> = OnceLock::new();
static DOC_RUNTIME_DIR: OnceLock<PathBuf> = OnceLock::new();

#[cfg(test)]
static TEST_TOOL_RUNTIME: OnceLock<DocToolRuntime> = OnceLock::new();

#[derive(Debug, Deserialize)]
struct ToolManifest {
    pandoc: ToolSpec,
    libreoffice: ToolSpec,
    font_plan: ToolSpec,
}

#[derive(Debug, Deserialize)]
struct ToolSpec {
    version: String,
    path: String,
    sha256: String,
}

#[derive(Debug, Clone)]
pub struct DocToolRuntime {
    resource_dir: PathBuf,
    pandoc: PathBuf,
    soffice: PathBuf,
    font_plan: PathBuf,
}

fn invalid(detail: impl AsRef<str>) -> IpcError {
    IpcError::new("DOC_TOOL_INVALID", detail.as_ref(), false, false)
}

fn sha256_file(path: &Path) -> Result<String, IpcError> {
    let file = File::open(path).map_err(|e| invalid(format!("无法读取包内工具 {path:?}: {e}")))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|e| invalid(format!("无法校验包内工具 {path:?}: {e}")))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn validate_tool(resource_dir: &Path, label: &str, spec: &ToolSpec) -> Result<PathBuf, IpcError> {
    let root = resource_dir
        .canonicalize()
        .map_err(|e| invalid(format!("无法解析 Binder resource_dir: {e}")))?;
    let path = resource_dir.join(&spec.path);
    let canonical = path
        .canonicalize()
        .map_err(|_| invalid(format!("包内 {label} 缺失: {path:?}")))?;
    if !canonical.starts_with(&root) {
        return Err(invalid(format!("包内 {label} 路径越出 resource_dir")));
    }
    if !canonical.is_file() {
        return Err(invalid(format!(
            "包内 {label} 不是可执行文件: {canonical:?}"
        )));
    }
    let actual_hash = sha256_file(&canonical)?;
    if actual_hash != spec.sha256 {
        return Err(invalid(format!("包内 {label} SHA-256 不匹配")));
    }
    if spec.version.trim().is_empty() {
        return Err(invalid(format!("包内 {label} 清单版本为空")));
    }
    Ok(canonical)
}

fn validate_resource_file(
    resource_dir: &Path,
    label: &str,
    spec: &ToolSpec,
) -> Result<PathBuf, IpcError> {
    let root = resource_dir
        .canonicalize()
        .map_err(|e| invalid(format!("无法解析 Binder resource_dir: {e}")))?;
    let path = resource_dir.join(&spec.path);
    let canonical = path
        .canonicalize()
        .map_err(|_| invalid(format!("包内 {label} 缺失: {path:?}")))?;
    if !canonical.starts_with(&root) || !canonical.is_file() {
        return Err(invalid(format!("包内 {label} 路径无效")));
    }
    if sha256_file(&canonical)? != spec.sha256 {
        return Err(invalid(format!("包内 {label} SHA-256 不匹配")));
    }
    if spec.version.trim().is_empty() {
        return Err(invalid(format!("包内 {label} 清单版本为空")));
    }
    Ok(canonical)
}

impl DocToolRuntime {
    /**
     * @GOV
     * codes:    BR-DOC-DATA-001-GUARD-DOC-DOC-CONVERT-002
     * type:     GUARD
     * chain:    DOC-CONVERT
     * rules:    BR-DOC-DATA-001
     * boundary: in=binderResourceDirAndToolManifest | out=verifiedBundledPandocAndLibreOffice(noPathOrSystemFallback)
     */
    /// 从 Tauri 的资源目录读取 manifest 并一次性校验两个工具；成功后才允许转换调用取得路径。
    pub fn from_resource_dir(resource_dir: &Path) -> Result<Self, IpcError> {
        let manifest_path = resource_dir.join("tools/manifest.json");
        let manifest_bytes = std::fs::read(&manifest_path)
            .map_err(|_| invalid(format!("包内工具清单缺失: {manifest_path:?}")))?;
        let manifest: ToolManifest = serde_json::from_slice(&manifest_bytes)
            .map_err(|e| invalid(format!("包内工具清单无效: {e}")))?;
        let pandoc = validate_tool(resource_dir, "Pandoc", &manifest.pandoc)?;
        let soffice = validate_tool(resource_dir, "LibreOffice", &manifest.libreoffice)?;
        let font_plan = validate_resource_file(resource_dir, "FontPlan", &manifest.font_plan)?;
        Ok(Self {
            resource_dir: resource_dir.to_path_buf(),
            pandoc,
            soffice,
            font_plan,
        })
    }

    pub fn pandoc_path(&self) -> &Path {
        &self.pandoc
    }

    pub fn soffice_path(&self) -> &Path {
        &self.soffice
    }

    pub fn font_plan_path(&self) -> &Path {
        &self.font_plan
    }

    pub fn resource_dir(&self) -> &Path {
        &self.resource_dir
    }
}

/// 在 Tauri setup 阶段静态校验包内工具。失败只禁用 DOC command，绝不阻断 Binder 启动，也不启动 soffice。
pub fn initialize(app: &AppHandle) {
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("无法解析 Tauri resource_dir: {e}"));
    let checked = resource_dir
        .and_then(|dir| DocToolRuntime::from_resource_dir(&dir).map_err(|error| error.message));
    let _ = TOOL_RUNTIME.set(checked);
    if let Ok(dir) = app.path().app_data_dir() {
        let runtime_dir = dir.join("doc-runtime");
        if std::fs::create_dir_all(&runtime_dir).is_ok() {
            let _ = DOC_RUNTIME_DIR.set(runtime_dir);
        }
    }
}

fn runtime() -> Result<&'static DocToolRuntime, IpcError> {
    if let Some(runtime) = TOOL_RUNTIME.get() {
        return runtime.as_ref().map_err(|detail| invalid(detail));
    }
    #[cfg(test)]
    {
        if let Some(runtime) = TEST_TOOL_RUNTIME.get() {
            return Ok(runtime);
        }
        let test_resource_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources");
        let candidate = DocToolRuntime::from_resource_dir(&test_resource_dir)?;
        let _ = TEST_TOOL_RUNTIME.set(candidate);
        return TEST_TOOL_RUNTIME
            .get()
            .ok_or_else(|| invalid("测试 DOC 工具运行时初始化失败"));
    }
    #[cfg(not(test))]
    Err(IpcError::new(
        "DOC_TOOL_RUNTIME_UNINITIALIZED",
        "包内 DOC 工具运行时尚未初始化",
        false,
        false,
    ))
}

pub fn pandoc_path() -> Result<&'static Path, IpcError> {
    Ok(runtime()?.pandoc_path())
}

pub fn soffice_path() -> Result<&'static Path, IpcError> {
    Ok(runtime()?.soffice_path())
}

pub fn resource_dir() -> Result<&'static Path, IpcError> {
    Ok(runtime()?.resource_dir())
}

pub fn font_plan_path() -> Result<&'static Path, IpcError> {
    Ok(runtime()?.font_plan_path())
}

pub fn runtime_dir() -> Result<&'static Path, IpcError> {
    DOC_RUNTIME_DIR
        .get()
        .map(PathBuf::as_path)
        .ok_or_else(|| invalid("DOC 运行时目录尚未初始化"))
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: BR-DOC-DATA-001
    #[test]
    fn project_resources_resolve_only_verified_bundled_tools() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources");
        let runtime = DocToolRuntime::from_resource_dir(&root).expect("包内工具必须完整可校验");
        assert!(runtime.pandoc_path().starts_with(&root));
        assert!(runtime.soffice_path().starts_with(&root));
        assert!(runtime.font_plan_path().starts_with(&root));
    }

    // covers: BR-DOC-DATA-001
    #[test]
    fn missing_manifest_is_explicit_not_a_system_fallback() {
        let missing =
            std::env::temp_dir().join(format!("binder_missing_tools_{}", std::process::id()));
        std::fs::create_dir_all(&missing).unwrap();
        let err = DocToolRuntime::from_resource_dir(&missing).unwrap_err();
        assert_eq!(err.code, "DOC_TOOL_INVALID");
        let _ = std::fs::remove_dir_all(missing);
    }
}
