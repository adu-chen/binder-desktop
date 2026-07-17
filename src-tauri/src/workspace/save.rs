//! PI-WS-02 · save 原子落盘 · WS-SAVE / BR-WS-PERSIST-001
//!
//! - 原子落盘（写临时文件 + rename）：写入中断不产生半落盘副作用，失败原文件保持（BR-WS-PERSIST-001）。

use crate::ipc::IpcError;
use std::path::Path;

/// 内容指纹（FNV-1a 64，**已定档**，G-5）。用途限非对抗性内容寻址与去重；
/// 不承诺抗对抗碰撞，不得挪用为完整性/安全校验。
pub fn content_hash(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// 读取文档内容 + 内容哈希。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DocRead {
    pub content: String,
    /// 读取时点的内容哈希。
    pub baseline: u64,
}

/// 读文档内容并附加内容哈希；非 UTF-8（二进制）按错误回报。路径守界由调用方
/// resolve_in_workspace 保证（X-FORBID-004）。
pub fn read_doc(abs_path: &Path) -> Result<DocRead, IpcError> {
    let bytes = std::fs::read(abs_path)
        .map_err(|e| IpcError::new("WS_READ_FAILED", &format!("读取文档失败: {e}"), true, false))?;
    let baseline = content_hash(&bytes);
    let content = String::from_utf8(bytes).map_err(|_| {
        IpcError::new(
            "WS_NOT_UTF8",
            "文档非 UTF-8 文本，无法以文本打开",
            false,
            false,
        )
    })?;
    Ok(DocRead { content, baseline })
}

/// save 结果：成功返回新内容哈希。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Saved {
    pub baseline: u64,
}

/**
 * @GOV
 * codes:    BR-WS-PERSIST-001-EFFECT-WS-WS-SAVE-001
 * type:     EFFECT
 * chain:    WS-SAVE
 * rules:    BR-WS-PERSIST-001
 * boundary: in=absolutePathAndContent | out=Saved(atomicNoHalfWrite)
 */
/// 原子保存（BR-WS-PERSIST-001）：写临时文件 + 持久化重命名；
/// 写入中断不产生半落盘副作用，失败时原文件保持 save 前内容。返回新内容哈希。
pub fn save_atomic(path: &Path, content: &[u8]) -> Result<Saved, IpcError> {
    let dir = path
        .parent()
        .ok_or_else(|| IpcError::new("WS_PATH_INVALID", "目标路径无父目录", false, false))?;
    let tmp = dir.join(format!(
        ".{}.tmp",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("save")
    ));

    let io_err =
        |e: std::io::Error| IpcError::new("WS_IO_ERROR", &format!("写入失败: {e}"), true, false);
    std::fs::write(&tmp, content).map_err(io_err)?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        io_err(e)
    })?;

    Ok(Saved {
        baseline: content_hash(content),
    })
}

/// SM-WS-01 状态（A-WS-M-S-01 §2 save 保存机）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveState {
    Idle,
    Dirty,
    Saving,
    Saved,
    SaveError,
}

/// SM-WS-01 事件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveEvent {
    Edit,
    Save,
    SaveOk,
    SaveFail,
    Retry,
}

/**
 * @GOV
 * codes:    BR-WS-PERSIST-001-RB-WS-WS-SAVE-002
 * type:     RB
 * chain:    WS-SAVE
 * rules:    BR-WS-PERSIST-001
 * boundary: in=SaveEvent | out=SaveState(noneWhenForbidden)
 */
/// SM-WS-01 转移引擎（enforcer of BR-WS-PERSIST-001）：
/// 返回 `None` = 无合法转移（含 forbidden：未经 Saving 不得直接 Saved）。
pub fn transition(state: SaveState, event: SaveEvent) -> Option<SaveState> {
    use SaveEvent::*;
    use SaveState::*;
    let next = match (state, event) {
        (Idle, Edit) => Dirty,
        (Dirty, Save) => Saving,
        (Saving, SaveOk) => Saved,
        (Saving, SaveFail) => SaveError,
        (SaveError, Retry) => Saving,
        (Saved, Edit) => Dirty,
        _ => return None,
    };
    Some(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_path(tag: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "binder_save_{}_{}.txt",
            tag,
            content_hash(tag.as_bytes())
        ));
        let _ = std::fs::remove_file(&p);
        p
    }

    // read_doc 回内容 + 哈希；非 UTF-8 报错。
    #[test]
    fn read_doc_returns_content_and_matching_baseline() {
        let path = tmp_path("readdoc");
        std::fs::write(&path, "标题\n正文").unwrap();
        let r = read_doc(&path).unwrap();
        assert_eq!(r.content, "标题\n正文");
        assert_eq!(r.baseline, content_hash("标题\n正文".as_bytes()));
        assert!(save_atomic(&path, "标题\n正文".as_bytes()).is_ok());
        std::fs::write(&path, [0xff, 0xfe, 0x00]).unwrap();
        let err = read_doc(&path).unwrap_err();
        assert_eq!(err.code, "WS_NOT_UTF8");
        let _ = std::fs::remove_file(&path);
    }

    // covers: BR-WS-PERSIST-001
    // 原子保存成功写入并返回内容哈希。
    #[test]
    fn atomic_save_writes_and_returns_baseline() {
        let path = tmp_path("atomic");
        let saved = save_atomic(&path, b"hello").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"hello");
        assert_eq!(saved.baseline, content_hash(b"hello"));
        let saved2 = save_atomic(&path, b"hello world").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"hello world");
        assert_eq!(saved2.baseline, content_hash(b"hello world"));
        let _ = std::fs::remove_file(&path);
    }

    // covers: BR-WS-PERSIST-001
    // SM-WS-01 强制转移矩阵：正路 + forbidden。
    #[test]
    fn sm_ws01_transition_matrix() {
        use SaveEvent::*;
        use SaveState::*;
        assert_eq!(transition(Idle, Edit), Some(Dirty));
        assert_eq!(transition(Dirty, Save), Some(Saving));
        assert_eq!(transition(Saving, SaveOk), Some(Saved));
        assert_eq!(transition(Saved, Edit), Some(Dirty));
        assert_eq!(transition(Saving, SaveFail), Some(SaveError));
        assert_eq!(transition(SaveError, Retry), Some(Saving));
        assert_eq!(
            transition(Dirty, SaveOk),
            None,
            "未经 Saving 不得直接 Saved"
        );
        assert_eq!(transition(Saving, Edit), None, "落盘中不得静默丢弃");
        assert_eq!(transition(Saved, SaveOk), None);
        assert_eq!(transition(Idle, Save), None);
    }
}
