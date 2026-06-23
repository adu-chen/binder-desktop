//! PI-BE-04 · workspace 内文件 CRUD（create/rename/delete）· WS-FS / BR-WS-DATA-001 / X-FORBID-004
//!
//! 承接 A-WS-I-P-01 §3（ws.create_doc）与 §8（move/copy 签名随实现补充）：workspace 内文档新建/重命名/
//! 删除。所有目标路径由调用方 resolve_in_workspace 守界（越界拒绝，X-FORBID-004）；本模块只做边界内
//! 文件系统操作，已存在/不存在等冲突回统一错误形状（不静默覆盖、不静默成功）。

use super::save::content_hash;
use crate::ipc::IpcError;
use std::path::Path;

fn io_err(action: &str, e: std::io::Error) -> IpcError {
    IpcError::new("WS_IO_ERROR", &format!("{action}失败: {e}"), true, false)
}

/**
 * @GOV
 * codes:    BR-WS-DATA-001-EFFECT-WS-WS-FS-001
 * type:     EFFECT
 * chain:    WS-FS
 * rules:    BR-WS-DATA-001
 * term_ref: TERM-WS-001
 * boundary: in=docAbsolutePathAndInitialContent | out=createdDocBaseline(rejectedWhenExists)
 */
/// 新建文档（BR-WS-DATA-001）：目标已存在则拒（WS_EXISTS，不覆盖）；按需建父目录后写入，回加载基线哈希。
/// 路径守界由调用方保证（X-FORBID-004）。
pub fn create_doc(abs_path: &Path, content: &[u8]) -> Result<u64, IpcError> {
    if abs_path.exists() {
        return Err(IpcError::new("WS_EXISTS", "目标已存在，不覆盖", false, false));
    }
    if let Some(parent) = abs_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| io_err("建目录", e))?;
    }
    std::fs::write(abs_path, content).map_err(|e| io_err("新建", e))?;
    Ok(content_hash(content))
}

/**
 * @GOV
 * codes:    BR-WS-DATA-001-EFFECT-WS-WS-FS-002
 * type:     EFFECT
 * chain:    WS-FS
 * rules:    BR-WS-DATA-001
 * term_ref: TERM-WS-001
 * boundary: in=fromAndToAbsolutePath | out=renamedDoc(rejectedWhenTargetExistsOrSourceMissing)
 */
/// 重命名/移动文档（BR-WS-DATA-001，承 §8 move）：源不存在或目标已存在则拒（不覆盖）；同 workspace 内
/// rename。两端路径均由调用方守界（X-FORBID-004）。按需建目标父目录。
pub fn rename_doc(from_abs: &Path, to_abs: &Path) -> Result<(), IpcError> {
    if !from_abs.exists() {
        return Err(IpcError::new("WS_NOT_FOUND", "源文件不存在", false, false));
    }
    if to_abs.exists() {
        return Err(IpcError::new("WS_EXISTS", "目标已存在，不覆盖", false, false));
    }
    if let Some(parent) = to_abs.parent() {
        std::fs::create_dir_all(parent).map_err(|e| io_err("建目录", e))?;
    }
    std::fs::rename(from_abs, to_abs).map_err(|e| io_err("重命名", e))?;
    Ok(())
}

/**
 * @GOV
 * codes:    BR-WS-DATA-001-EFFECT-WS-WS-FS-003
 * type:     EFFECT
 * chain:    WS-FS
 * rules:    BR-WS-DATA-001
 * term_ref: TERM-WS-001
 * boundary: in=docAbsolutePath | out=deletedDoc(rejectedWhenMissing)
 */
/// 删除文档（BR-WS-DATA-001）：不存在则拒（WS_NOT_FOUND，不静默成功）；路径守界由调用方保证。
pub fn delete_doc(abs_path: &Path) -> Result<(), IpcError> {
    if !abs_path.exists() {
        return Err(IpcError::new("WS_NOT_FOUND", "文件不存在", false, false));
    }
    std::fs::remove_file(abs_path).map_err(|e| io_err("删除", e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir(tag: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("binder_fileops_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    // covers: BR-WS-DATA-001
    // 新建写入并回基线；已存在则拒（不覆盖）；建父目录。
    #[test]
    fn create_rejects_existing_and_makes_parent() {
        let d = tmp_dir("create");
        let f = d.join("sub/new.md");
        let base = create_doc(&f, b"hello").unwrap();
        assert_eq!(std::fs::read(&f).unwrap(), b"hello");
        assert_eq!(base, content_hash(b"hello"));
        // 已存在 → WS_EXISTS
        let err = create_doc(&f, b"x").unwrap_err();
        assert_eq!(err.code, "WS_EXISTS");
        let _ = std::fs::remove_dir_all(&d);
    }

    // covers: BR-WS-DATA-001
    // 重命名移动文件；源缺失/目标已存在均拒。
    #[test]
    fn rename_moves_and_rejects_conflicts() {
        let d = tmp_dir("rename");
        let from = d.join("a.md");
        let to = d.join("b.md");
        std::fs::write(&from, b"body").unwrap();
        rename_doc(&from, &to).unwrap();
        assert!(!from.exists() && std::fs::read(&to).unwrap() == b"body");
        // 源缺失
        assert_eq!(rename_doc(&from, &d.join("c.md")).unwrap_err().code, "WS_NOT_FOUND");
        // 目标已存在
        std::fs::write(&from, b"x").unwrap();
        assert_eq!(rename_doc(&from, &to).unwrap_err().code, "WS_EXISTS");
        let _ = std::fs::remove_dir_all(&d);
    }

    // covers: BR-WS-DATA-001
    // 删除存在文件；不存在则拒（不静默成功）。
    #[test]
    fn delete_removes_and_rejects_missing() {
        let d = tmp_dir("delete");
        let f = d.join("x.md");
        std::fs::write(&f, b"x").unwrap();
        delete_doc(&f).unwrap();
        assert!(!f.exists());
        assert_eq!(delete_doc(&f).unwrap_err().code, "WS_NOT_FOUND");
        let _ = std::fs::remove_dir_all(&d);
    }
}
