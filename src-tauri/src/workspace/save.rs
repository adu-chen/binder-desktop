//! PI-WS-02 · save 保存机与外部变更冲突保护 · WS-SAVE / SM-WS-01 / BR-WS-PERSIST-001 / BR-WS-OBS-001
//!
//! 承接 A-WS-M-T-01 §10 / §4（外部变更感知）与 A-WS-M-S-01 SM-WS-01（含 saveConflict/override）：
//! - 原子落盘（写临时文件 + rename）：写入中断不产生半落盘副作用，失败原文件保持（BR-WS-PERSIST-001）。
//! - 加载基线（内容哈希）：save 前校验磁盘内容与基线一致；不一致即冲突，不静默覆盖外部修改
//!   （BR-WS-OBS-001）。文件监视机制（FSEvents/轮询）为实现期 A 类债务；本模块以保存前校验为下限。

use crate::ipc::IpcError;
use std::path::Path;

/// 内容指纹（基线哈希）。算法为实现期选型（此处 FNV-1a 64 占位，A 类债务）。
pub fn content_hash(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// 读取文档内容 + 加载基线（BR-WS-OBS-001）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DocRead {
    pub content: String,
    /// 读取时点内容基线哈希——供 save 校验与外部变更检测。
    pub baseline: u64,
}

/**
 * @GOV
 * codes:    BR-WS-OBS-001-QUERY-WS-WS-FS-003
 * type:     QUERY
 * chain:    WS-FS
 * rules:    BR-WS-OBS-001
 * boundary: in=docAbsolutePath | out=DocRead(contentWithLoadedBaseline)
 */
/// 读文档内容并附加载基线（BR-WS-OBS-001）：基线随读取时点内容计算，供后续 save 一致性校验与外部
/// 变更检测；非 UTF-8（二进制）按错误回报（二进制读写归 DOC/媒体，不在文本读路）。路径守界由调用方
/// resolve_in_workspace 保证（X-FORBID-004）。
pub fn read_doc(abs_path: &Path) -> Result<DocRead, IpcError> {
    let bytes = std::fs::read(abs_path)
        .map_err(|e| IpcError::new("WS_READ_FAILED", &format!("读取文档失败: {e}"), true, false))?;
    let baseline = content_hash(&bytes);
    let content = String::from_utf8(bytes)
        .map_err(|_| IpcError::new("WS_NOT_UTF8", "文档非 UTF-8 文本，无法以文本打开", false, false))?;
    Ok(DocRead { content, baseline })
}

/// 读取文件当前内容哈希；文件不存在记为 None（新建场景）。
fn disk_hash(path: &Path) -> std::io::Result<Option<u64>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(content_hash(&bytes))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// save 结果：成功返回新基线哈希。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Saved {
    pub baseline: u64,
}

/**
 * @GOV
 * codes:    BR-WS-OBS-001-GUARD-WS-WS-SAVE-001
 * type:     GUARD
 * chain:    WS-SAVE
 * rules:    BR-WS-OBS-001
 * boundary: in=loadedBaselineVsDisk | out=conflictVerdict(saveConflictWhenDiverged)
 */
/// 保存前外部变更校验（BR-WS-OBS-001 / SM-WS-01 saveConflict guard）：
/// 比对磁盘当前哈希与加载基线——不一致（外部已改）且非显式覆盖时返回冲突错误，不写盘。
/// `override_external=true` 即 SM-WS-01 overrideConfirmed，须源自用户显式确认。
fn check_baseline(
    path: &Path,
    loaded_baseline: Option<u64>,
    override_external: bool,
) -> Result<(), IpcError> {
    if override_external {
        return Ok(());
    }
    let current = disk_hash(path).map_err(|e| {
        IpcError::new("WS_IO_ERROR", &format!("读取磁盘失败: {e}"), true, false)
    })?;
    if current != loaded_baseline {
        return Err(IpcError::new(
            "WS_SAVE_CONFLICT",
            "目标文件已被外部修改，需用户裁决（覆盖或重载）",
            false,
            false,
        ));
    }
    Ok(())
}

/**
 * @GOV
 * codes:    BR-WS-PERSIST-001-EFFECT-WS-WS-SAVE-001
 * type:     EFFECT
 * chain:    WS-SAVE
 * rules:    BR-WS-PERSIST-001
 * boundary: in=absolutePathAndContent | out=Saved(atomicNoHalfWrite)
 */
/// 原子保存（BR-WS-PERSIST-001）：先经基线校验（BR-WS-OBS-001），再写临时文件 + 持久化重命名；
/// 写入中断不产生半落盘副作用，失败时原文件保持 save 前内容。返回新基线哈希。
pub fn save_atomic(
    path: &Path,
    content: &[u8],
    loaded_baseline: Option<u64>,
    override_external: bool,
) -> Result<Saved, IpcError> {
    check_baseline(path, loaded_baseline, override_external)?;

    let dir = path.parent().ok_or_else(|| {
        IpcError::new("WS_PATH_INVALID", "目标路径无父目录", false, false)
    })?;
    let tmp = dir.join(format!(
        ".{}.tmp",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("save")
    ));

    let io_err = |e: std::io::Error| IpcError::new("WS_IO_ERROR", &format!("写入失败: {e}"), true, false);
    // 写临时文件（失败：原文件未动，无半落盘）。
    std::fs::write(&tmp, content).map_err(io_err)?;
    // 原子重命名替换（同目录 rename 原子）。
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp); // 清理临时文件
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

/// SM-WS-01 事件。`SaveOk`（磁盘==基线落盘成功）/ `SaveConflict`（磁盘≠基线）/ `SaveFail`（IO 失败）；
/// `Retry`（非冲突失败重试）/ `OverrideConfirmed`（冲突场景用户显式覆盖）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveEvent {
    Edit,
    Save,
    SaveOk,
    SaveConflict,
    SaveFail,
    Retry,
    OverrideConfirmed,
}

/**
 * @GOV
 * codes:    BR-WS-PERSIST-001-RB-WS-WS-SAVE-002
 * type:     RB
 * chain:    WS-SAVE
 * rules:    BR-WS-PERSIST-001
 * boundary: in=SaveEvent | out=SaveState(noneWhenForbidden,enforcerNoSilentOverwrite)
 */
/// SM-WS-01 转移引擎（enforcer of BR-WS-PERSIST-001 / BR-WS-OBS-001）：
/// 返回 `None` = 无合法转移（含 forbidden：未经 Saving 不得直接 Saved；磁盘≠基线必经
/// saveConflict→SaveError，不静默覆盖；冲突态须 OverrideConfirmed 方可重写）。
pub fn transition(state: SaveState, event: SaveEvent) -> Option<SaveState> {
    use SaveEvent::*;
    use SaveState::*;
    let next = match (state, event) {
        (Idle, Edit) => Dirty,
        (Dirty, Save) => Saving,
        (Saving, SaveOk) => Saved,
        (Saving, SaveConflict) => SaveError, // 外部变更：不覆盖，交用户裁决
        (Saving, SaveFail) => SaveError,
        (SaveError, Retry) => Saving,             // 非冲突失败重试
        (SaveError, OverrideConfirmed) => Saving, // 冲突：用户显式覆盖后重写
        (Saved, Edit) => Dirty,
        // 其余：无合法转移（含全部 forbidden_transitions）。
        _ => return None,
    };
    Some(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_path(tag: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("binder_save_{}_{}.txt", tag, content_hash(tag.as_bytes())));
        let _ = std::fs::remove_file(&p);
        p
    }

    // covers: BR-WS-OBS-001
    // read_doc 回内容 + 加载基线，基线与同内容 save 校验一致（读后即可无冲突 save）；非 UTF-8 报错。
    #[test]
    fn read_doc_returns_content_and_matching_baseline() {
        let path = tmp_path("readdoc");
        std::fs::write(&path, "标题\n正文").unwrap();
        let r = read_doc(&path).unwrap();
        assert_eq!(r.content, "标题\n正文");
        assert_eq!(r.baseline, content_hash("标题\n正文".as_bytes()));
        // 以读得基线原样 save 不冲突。
        assert!(save_atomic(&path, "标题\n正文".as_bytes(), Some(r.baseline), false).is_ok());
        // 非 UTF-8 字节按错误回报。
        std::fs::write(&path, [0xff, 0xfe, 0x00]).unwrap();
        let err = read_doc(&path).unwrap_err();
        assert_eq!(err.code, "WS_NOT_UTF8");
        let _ = std::fs::remove_file(&path);
    }

    // covers: BR-WS-PERSIST-001
    // 原子保存成功写入并返回新基线；再次以新基线保存可继续（往返）。
    #[test]
    fn atomic_save_writes_and_returns_baseline() {
        let path = tmp_path("atomic");
        // 新建（基线 None）
        let saved = save_atomic(&path, b"hello", None, false).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"hello");
        assert_eq!(saved.baseline, content_hash(b"hello"));
        // 以当前基线再次保存
        let saved2 = save_atomic(&path, b"hello world", Some(saved.baseline), false).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"hello world");
        assert_eq!(saved2.baseline, content_hash(b"hello world"));
        let _ = std::fs::remove_file(&path);
    }

    // covers: BR-WS-OBS-001
    // 外部修改后以陈旧基线保存 → saveConflict，不覆盖外部内容；用户显式 override 后方可覆盖。
    #[test]
    fn external_change_triggers_conflict_then_override() {
        let path = tmp_path("conflict");
        let saved = save_atomic(&path, b"v1", None, false).unwrap();
        let loaded = saved.baseline;

        // 模拟外部程序改写文件
        std::fs::write(&path, b"external-edit").unwrap();

        // 以加载基线保存：检测到外部变更 → 冲突，不覆盖
        let err = save_atomic(&path, b"my-edit", Some(loaded), false).unwrap_err();
        assert_eq!(err.code, "WS_SAVE_CONFLICT");
        assert!(!err.retriable);
        assert_eq!(std::fs::read(&path).unwrap(), b"external-edit", "冲突时不得覆盖外部修改");

        // 用户显式覆盖确认（override）后落盘
        let ok = save_atomic(&path, b"my-edit", Some(loaded), true).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"my-edit");
        assert_eq!(ok.baseline, content_hash(b"my-edit"));
        let _ = std::fs::remove_file(&path);
    }

    // covers: BR-WS-PERSIST-001
    // SM-WS-01 强制转移矩阵：正路 + 冲突路 + forbidden（不静默覆盖、覆盖须显式确认）。
    #[test]
    fn sm_ws01_transition_matrix() {
        use SaveEvent::*;
        use SaveState::*;
        // 正路
        assert_eq!(transition(Idle, Edit), Some(Dirty));
        assert_eq!(transition(Dirty, Save), Some(Saving));
        assert_eq!(transition(Saving, SaveOk), Some(Saved));
        assert_eq!(transition(Saved, Edit), Some(Dirty));
        // 冲突路：Saving→saveConflict→SaveError→overrideConfirmed→Saving
        assert_eq!(transition(Saving, SaveConflict), Some(SaveError));
        assert_eq!(transition(SaveError, OverrideConfirmed), Some(Saving));
        assert_eq!(transition(SaveError, Retry), Some(Saving));
        // forbidden_transitions
        assert_eq!(transition(Dirty, SaveOk), None, "未经 Saving 不得直接 Saved");
        assert_eq!(transition(Saving, Edit), None, "落盘中不得静默丢弃");
        assert_eq!(transition(Saved, SaveOk), None);
        assert_eq!(transition(Idle, Save), None);
    }
}
