//! PI-DOC-01/03 · 转换生命周期 SM-DOC-01 + 转换管线边界 · DOC-CONVERT
//! 规则：BR-DOC-STATE-001（转换生命周期）/ BR-DOC-DATA-001（管线必经/双链分离/依赖缺失明确报错）
//!
//! 承接 A-DOC-M-S-01 SM-DOC-01：打开转换失败→OpenError **不建立编辑会话**（对接 SM-ED-01 loadFail/
//! parseFail）；保存转换/落盘失败→SaveError **原 docx 保持不变**（原子经 WS BR-WS-PERSIST-001）、无半成品；
//! 失败可重试；Idle 为起止态（循环生命周期）。BR-DOC-DATA-001：docx 解析/生成必经 DOC 管线——**编辑链
//! （Pandoc）与预览链（LibreOffice）完全分离**、不共享中间态；转换全程本地不上行；**依赖缺失回统一错误
//! 形状、不静默降级**。Pandoc/LibreOffice 实际子进程调用与 Lua 过滤器为实现期（DOC-01/05，A 类）。

use crate::ipc::IpcError;

/// SM-DOC-01 状态（A-DOC-M-S-01）。Idle 为起止态（循环，无破坏性残留）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocConvertState {
    Idle,
    OpenConverting,
    Ready,
    OpenError,
    SaveConverting,
    SaveError,
}

/// SM-DOC-01 事件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocConvertEvent {
    OpenDocx,
    ConvertOk,
    ConvertFail,
    SaveDocx,
    SaveConvertOk,
    SaveConvertFail,
    Retry,
    Close,
}

/// 转换链（双链分离：编辑链 Pandoc / 预览链 LibreOffice，不共享中间态）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvertChain {
    Edit,    // 编辑链 → Pandoc
    Preview, // 预览链 → LibreOffice
}

/// 转换工具。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvertTool {
    Pandoc,
    LibreOffice,
}

/// 是否回到起止态（Idle）。
pub fn is_idle(state: DocConvertState) -> bool {
    matches!(state, DocConvertState::Idle)
}

/**
 * @GOV
 * codes:    BR-DOC-STATE-001-RB-DOC-DOC-CONVERT-001
 * type:     RB
 * chain:    DOC-CONVERT
 * rules:    BR-DOC-STATE-001
 * boundary: in=DocConvertEvent | out=DocConvertState(convertFailNoSessionSaveFailOriginalIntactRetryable)
 */
/// SM-DOC-01 转移引擎（BR-DOC-STATE-001）：openDocx→OpenConverting；convertFail→OpenError（**不建编辑
/// 会话**）；convertOk→Ready；saveDocx→SaveConverting；saveConvertFail→SaveError（**原 docx 不变、无半
/// 成品**）；OpenError/SaveError 可 retry 或 close 回 Idle。返回 `None` = 无合法转移（含：未经转换不得
/// Ready/落盘成功，失败态不直接成功）。
pub fn transition(state: DocConvertState, event: DocConvertEvent) -> Option<DocConvertState> {
    use DocConvertEvent::*;
    use DocConvertState::*;
    let next = match (state, event) {
        (Idle, OpenDocx) => OpenConverting,
        (OpenConverting, ConvertOk) => Ready,
        (OpenConverting, ConvertFail) => OpenError, // 不建立编辑会话
        (OpenError, Retry) => OpenConverting,
        (OpenError, Close) => Idle,
        (Ready, SaveDocx) => SaveConverting,
        (SaveConverting, SaveConvertOk) => Ready, // 原子替换完成
        (SaveConverting, SaveConvertFail) => SaveError, // 原 docx 保持不变
        (SaveError, Retry) => SaveConverting,
        (SaveError, Close) => Idle, // 经用户确认（脏态处置归 SM-ED-01）
        (Ready, Close) => Idle,
        // 其余无合法转移（未经 OpenConverting+convertOk 不得 Ready；失败态不直接成功；Idle 不直接 Ready）。
        _ => return None,
    };
    Some(next)
}

/**
 * @GOV
 * codes:    BR-DOC-DATA-001-GUARD-DOC-DOC-CONVERT-001
 * type:     GUARD
 * chain:    DOC-CONVERT
 * rules:    BR-DOC-DATA-001
 * boundary: in=convertChainDependency | out=toolPermitOrUnifiedError(missingDepNoSilentDegrade)
 */
/// 转换依赖守卫（BR-DOC-DATA-001）：编辑链必经 Pandoc、预览链必经 LibreOffice（**双链分离**，工具唯一
/// 对应）；依赖不可用时回**统一错误形状、不静默降级**（不回退到其他链或纯文本）。available 由调用方探测。
pub fn ensure_chain_dependency(chain: ConvertChain, available: bool) -> Result<ConvertTool, IpcError> {
    let tool = tool_for(chain); // 双链分离：编辑→Pandoc，预览→LibreOffice
    if !available {
        return Err(IpcError::new(
            "DOC_DEP_MISSING",
            &format!("转换依赖缺失（{tool:?}），明确报错不静默降级"),
            true, // 安装后可重试
            false,
        ));
    }
    Ok(tool)
}

/// 双链分离的工具映射（编辑链 Pandoc / 预览链 LibreOffice，互不共享）。
pub fn tool_for(chain: ConvertChain) -> ConvertTool {
    match chain {
        ConvertChain::Edit => ConvertTool::Pandoc,
        ConvertChain::Preview => ConvertTool::LibreOffice,
    }
}

#[cfg(test)]
mod tests {
    use super::DocConvertEvent::*;
    use super::DocConvertState::*;
    use super::*;

    // covers: BR-DOC-STATE-001
    // 打开转换失败不建会话；保存转换失败原文件不变；失败可重试/关闭；正路往返。
    #[test]
    fn lifecycle_fail_isolation_and_retry() {
        assert_eq!(transition(Idle, OpenDocx), Some(OpenConverting));
        // 打开失败 → OpenError（不建编辑会话），可重试/放弃
        assert_eq!(transition(OpenConverting, ConvertFail), Some(OpenError));
        assert_eq!(transition(OpenError, Retry), Some(OpenConverting));
        assert_eq!(transition(OpenError, Close), Some(Idle));
        // 正路：打开成功 → Ready → 保存
        assert_eq!(transition(OpenConverting, ConvertOk), Some(Ready));
        assert_eq!(transition(Ready, SaveDocx), Some(SaveConverting));
        assert_eq!(transition(SaveConverting, SaveConvertOk), Some(Ready));
        // 保存失败 → SaveError（原 docx 不变），可重试
        assert_eq!(transition(SaveConverting, SaveConvertFail), Some(SaveError));
        assert_eq!(transition(SaveError, Retry), Some(SaveConverting));
        assert_eq!(transition(Ready, Close), Some(Idle));
        assert!(is_idle(Idle) && !is_idle(Ready));
    }

    // covers: BR-DOC-STATE-001
    // forbidden：未经 OpenConverting+convertOk 不得 Ready；失败态不直接成功；Idle 不直达 Ready。
    #[test]
    fn forbidden_transitions_return_none() {
        assert_eq!(transition(Idle, ConvertOk), None, "Idle 不得直达 Ready");
        assert_eq!(transition(OpenError, ConvertOk), None, "失败态不直接成功");
        assert_eq!(transition(SaveError, SaveConvertOk), None);
        assert_eq!(transition(Ready, ConvertOk), None);
        assert_eq!(transition(Idle, SaveDocx), None);
    }

    // covers: BR-DOC-DATA-001
    // 双链分离：编辑链→Pandoc、预览链→LibreOffice；依赖缺失统一错误不静默降级。
    #[test]
    fn chain_separation_and_missing_dep_errors() {
        assert_eq!(tool_for(ConvertChain::Edit), ConvertTool::Pandoc);
        assert_eq!(tool_for(ConvertChain::Preview), ConvertTool::LibreOffice);
        // 双链工具互不相同（分离）
        assert_ne!(tool_for(ConvertChain::Edit), tool_for(ConvertChain::Preview));
        // 可用 → 放行对应工具
        assert_eq!(ensure_chain_dependency(ConvertChain::Edit, true).unwrap(), ConvertTool::Pandoc);
        // 缺失 → 统一错误形状，不静默降级
        let err = ensure_chain_dependency(ConvertChain::Preview, false).unwrap_err();
        assert_eq!(err.code, "DOC_DEP_MISSING");
        assert!(err.retriable);
    }
}
