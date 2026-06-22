//! PI-REF-02 · 锚校验与注入载荷组装 · REF-INJECT / BR-REF-VERIFY-001 / BR-REF-DATA-002
//!
//! 承接 A-REF-M-T-01：
//! - 锚校验（VERIFY-001）：注入前校验锚有效性——路径级校验目标存在；块级经 ed.locate_block（LocateResult）；
//!   **锚失效发明确信号，不静默注入空/过期/错位内容**；检测不可得（Undetectable）**按失效保守处理（不注入）**。
//! - 载荷组装（DATA-002）：内容经 WS 读取目标在**读取时点**当前内容（不缓存陈旧跨回合复用）；载荷含来源标识
//!   可溯源；结构用 A-SYS-I-P-01 RefInjection（不私自扩展跨模块字段）；裁剪归 AG，REF 不裁剪。

use crate::ipc::IpcError;
use crate::reference::resolve::ReferenceAnchor;
use crate::types::{LocateResult, RefInjection};

/**
 * @GOV
 * codes:    BR-REF-VERIFY-001-GUARD-REF-REF-INJECT-001
 * type:     GUARD
 * chain:    REF-INJECT
 * rules:    BR-REF-VERIFY-001
 * boundary: in=anchorAndLocateResult | out=verifiedOrFailSignal(noStaleEmptyMisplacedInjection)
 */
/// 注入前锚校验（BR-REF-VERIFY-001）：路径级锚校验目标存在（path_exists）；块级锚经 blockId 定位——
/// Located 通过，**AnchorLost / Undetectable（检测不可得保守）→ Err 明确失效信号**（消费方可示用户重选），
/// 不静默注入空/过期/错位内容。
pub fn verify_anchor(
    anchor: &ReferenceAnchor,
    path_exists: bool,
    block_locate: LocateResult,
) -> Result<(), IpcError> {
    let fail = || IpcError::new("REF_ANCHOR_INVALID", "引用锚失效，须重选（不静默注入）", false, false);
    match anchor {
        ReferenceAnchor::Path(_) => {
            if path_exists {
                Ok(())
            } else {
                Err(fail())
            }
        }
        ReferenceAnchor::Block { .. } => match block_locate {
            LocateResult::Located => Ok(()),
            // 失效 / 检测不可得（保守）→ 不注入。
            LocateResult::AnchorLost | LocateResult::Undetectable => Err(fail()),
        },
    }
}

/**
 * @GOV
 * codes:    BR-REF-DATA-002-DATA-REF-REF-INJECT-001
 * type:     DATA
 * chain:    REF-INJECT
 * rules:    BR-REF-DATA-002
 * boundary: in=verifiedAnchorAndReadTimeContent | out=RefInjection(freshContentTraceableNoSelfExtend)
 */
/// 注入载荷组装（BR-REF-DATA-002）：以**读取时点**当前内容（调用方经 WS 即时读取，不缓存陈旧）构建
/// RefInjection（A-SYS-I-P-01 结构，不私自扩展字段）；含 origin_label 可溯源。应在 verify_anchor 通过后调用。
/// 不做裁剪（裁剪归 AG BR-AG-DATA-001）。
pub fn build_injection(
    anchor: &ReferenceAnchor,
    content_at_read_time: String,
    origin_label: String,
) -> RefInjection {
    match anchor {
        ReferenceAnchor::Path(target) => RefInjection {
            target: target.clone(),
            block_ids: None,
            content: content_at_read_time,
            origin_label,
        },
        ReferenceAnchor::Block { target, block_ids } => RefInjection {
            target: target.clone(),
            block_ids: Some(block_ids.clone()),
            content: content_at_read_time,
            origin_label,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DocRef;

    fn path_anchor() -> ReferenceAnchor {
        ReferenceAnchor::Path(DocRef { workspace_id: "ws1".into(), rel_path: "a.md".into(), doc_type: "md".into() })
    }
    fn block_anchor() -> ReferenceAnchor {
        ReferenceAnchor::Block {
            target: DocRef { workspace_id: "ws1".into(), rel_path: "a.md".into(), doc_type: "md".into() },
            block_ids: vec!["b1".into()],
        }
    }

    // covers: BR-REF-VERIFY-001
    // 路径级存在通过/不存在失败；块级 Located 通过，AnchorLost/Undetectable 保守失效（不静默注入）。
    #[test]
    fn verify_fails_closed_on_invalid_anchor() {
        assert!(verify_anchor(&path_anchor(), true, LocateResult::Located).is_ok());
        assert_eq!(
            verify_anchor(&path_anchor(), false, LocateResult::Located).unwrap_err().code,
            "REF_ANCHOR_INVALID"
        );
        assert!(verify_anchor(&block_anchor(), true, LocateResult::Located).is_ok());
        for lost in [LocateResult::AnchorLost, LocateResult::Undetectable] {
            assert!(verify_anchor(&block_anchor(), true, lost).is_err(), "块级失效须不注入");
        }
    }

    // covers: BR-REF-DATA-002
    // 载荷用读取时点内容 + origin_label 可溯源；块级带 block_ids，结构为 RefInjection（不扩展字段）。
    #[test]
    fn build_injection_uses_fresh_content_and_is_traceable() {
        let inj = build_injection(&block_anchor(), "读取时点内容".into(), "@a.md#b1".into());
        assert_eq!(inj.content, "读取时点内容");
        assert_eq!(inj.origin_label, "@a.md#b1"); // 可溯源
        assert_eq!(inj.block_ids.as_deref(), Some(&["b1".to_string()][..]));
        // 路径级 block_ids 为 None
        assert!(build_injection(&path_anchor(), "x".into(), "@a.md".into()).block_ids.is_none());
    }
}
