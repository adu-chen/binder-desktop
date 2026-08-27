//! PI-DE-01 · diff 生成与载荷 · DE-DIFF / BR-DE-DATA-001 / BR-DE-DATA-002
//!
//! 承接 A-DE-M-T-01：消费 ChangeProposal 生成待裁决 PendingDiff——修改既有文档须经 blockId 锚定
//! （ed.locate_block，LocateResult），**锚失效（AnchorLost / 检测不可得 Undetectable 保守）→ 生成失败明确
//! 回报**（不产出错位/悬空 diff，BR-DE-DATA-001）；载荷形状统一（DiffPayload：target + after_content +
//! before_snapshot，**新建文档空基准 None**，BR-DE-DATA-002）。持久化经 ws.persist_diff、终态机归 PI-DE-02、
//! 过期/冲突归 PI-DE-03。ChangeProposal 由 DE 单一拥有（AG/CBT 产出侧引用本类型）。

use crate::ipc::IpcError;
use crate::types::{DiffStatus, DocRef, LocateResult, PendingDiff};

/// 修改建议载荷（AG/CBT 产出 → DE 消费）。DE 为 diff 链路与载荷形状的单一拥有方（BR-DE-DATA-002）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeProposal {
    /// 产出该提案的来源（AG 回合 / CBT 任务），按条裁决/留痕。
    pub origin_task: String,
    /// 目标文档（修改既有或计划新建的 DocRef）。
    pub target: DocRef,
    /// 提案后内容。
    pub after_content: Vec<u8>,
    /// 修改既有文档的内容基准；**None = 新建文档（空基准）**。
    pub before_snapshot: Option<String>,
    /// 样式操作序列（StyleOp JSON，形状归 ED A-ED-M-T-01 §5.1；PI-ED-11 / 裁定 D-3 方案A）。
    /// None = 纯文本 diff。样式修改的审阅呈现语义载体，与 after_content 并存。
    pub style_ops: Option<String>,
}

/// 统一 diff 载荷形状（BR-DE-DATA-002）。is_new_doc 由 before_snapshot 缺省派生。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffPayload {
    pub target: DocRef,
    pub after_content: Vec<u8>,
    pub before_snapshot: Option<String>,
    pub is_new_doc: bool,
    /// 样式操作序列（可空，A-DE-M-T-01 v1.1：语义归 ED BR-ED-DATA-005，载荷承载归本结构）。
    pub style_ops: Option<String>,
}

/**
 * @GOV
 * codes:    BR-DE-DATA-002-DATA-DE-DE-DIFF-001
 * type:     DATA
 * chain:    DE-DIFF
 * rules:    BR-DE-DATA-002
 * boundary: in=ChangeProposal | out=DiffPayload(unifiedShapeNewDocEmptyBaseline)
 */
/// 载荷组装（BR-DE-DATA-002）：ChangeProposal → 统一 DiffPayload；新建文档 before_snapshot=None 即空基准
/// （is_new_doc=true），修改既有携带 before_snapshot 基准。载荷形状单一（消费方按统一结构理解）。
pub fn build_payload(proposal: &ChangeProposal) -> DiffPayload {
    DiffPayload {
        target: proposal.target.clone(),
        after_content: proposal.after_content.clone(),
        before_snapshot: proposal.before_snapshot.clone(),
        is_new_doc: proposal.before_snapshot.is_none(),
        style_ops: proposal.style_ops.clone(),
    }
}

/**
 * @GOV
 * codes:    BR-AG-DATA-003-GUARD-DE-DE-DIFF-004
 * type:     GUARD
 * chain:    DE-DIFF
 * rules:    BR-AG-DATA-003
 * boundary: in=proposalTargetAndDeclaredTargetSet | out=declaredTargetVerdict(rejectUndeclaredExistingDocBeforeDiffGen)
 */
/// DE 侧目标声明判定（BR-AG-DATA-003「DE 侧协同复核」，2026-07-04 版；AG-DELETE-01 放宽）：
/// 返回目标是否属回合声明的目标集（当前文档 ∪ 用户显式指向集）；不存在的路径视为新建放行。
/// **不再硬拒未声明的既有文档**——由 AG 层标注 cross_doc + DiffCard「跨文档目标」确认承担安全，
/// 多文档联动（参考 A 改 B 等）不再被锁死。CBT 构建路径不经本函数（BR-CBT-DATA-001 绑定）。
pub fn target_declared(
    target_rel: &str,
    current_rel: Option<&str>,
    declared_rels: &[String],
    target_exists: bool,
) -> bool {
    !target_exists
        || current_rel == Some(target_rel)
        || declared_rels.iter().any(|d| d == target_rel)
}

/**
 * @GOV
 * codes:    BR-DE-DATA-001-GUARD-DE-DE-DIFF-005
 * type:     GUARD
 * chain:    DE-DIFF
 * rules:    BR-DE-DATA-001
 * boundary: in=proposalAndAnchorLocate | out=PendingDiff(genFailedWhenAnchorLostNoStaleDiff)
 */
/// diff 生成与锚校验（BR-DE-DATA-001）：修改既有文档（before_snapshot=Some）须 blockId 锚 **Located**
/// 方生成；**AnchorLost / Undetectable（保守）→ Err（GenFailed 明确回报）**，不产出错位/悬空 diff。
/// 新建文档（None）无既有锚，跳过定位。成功返回 status=Pending 的 PendingDiff（待持久化经 ws.persist_diff）。
pub fn generate_diff(
    proposal: &ChangeProposal,
    diff_id: &str,
    origin_module: &str,
    payload_ref: &str,
    created_at: i64,
    anchor_locate: LocateResult,
) -> Result<PendingDiff, IpcError> {
    // 修改既有文档须锚定位成功；新建文档无既有锚。
    if proposal.before_snapshot.is_some() {
        match anchor_locate {
            LocateResult::Located => {}
            LocateResult::AnchorLost | LocateResult::Undetectable => {
                return Err(IpcError::new(
                    "DE_ANCHOR_LOST",
                    "blockId 锚失效，diff 生成失败（不产出错位/悬空 diff）",
                    true, // 可重试（重新定位/重选）
                    false,
                ));
            }
        }
    }
    Ok(PendingDiff {
        diff_id: diff_id.to_string(),
        target: proposal.target.clone(),
        origin_module: origin_module.to_string(),
        status: DiffStatus::Pending,
        created_at,
        payload_ref: payload_ref.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DocRef, LocateResult};

    fn doc(rel: &str) -> DocRef {
        DocRef {
            workspace_id: "ws1".into(),
            rel_path: rel.into(),
            doc_type: "md".into(),
        }
    }
    fn modify_proposal() -> ChangeProposal {
        ChangeProposal {
            origin_task: "turn1".into(),
            target: doc("a.md"),
            after_content: b"edited".to_vec(),
            before_snapshot: Some("h0".into()),
            style_ops: None,
        }
    }
    fn new_proposal() -> ChangeProposal {
        ChangeProposal {
            origin_task: "turn2".into(),
            target: doc("new.md"),
            after_content: b"created".to_vec(),
            before_snapshot: None,
            style_ops: None,
        }
    }

    // covers: BR-DE-DATA-002
    // 载荷统一形状：新建空基准 is_new_doc=true；修改既有携带 before_snapshot。
    #[test]
    fn payload_shape_new_vs_modify() {
        let np = build_payload(&new_proposal());
        assert!(np.is_new_doc && np.before_snapshot.is_none());
        let mp = build_payload(&modify_proposal());
        assert!(!mp.is_new_doc && mp.before_snapshot.as_deref() == Some("h0"));
    }

    // covers: BR-DE-DATA-001
    // 修改既有：锚 Located 生成 Pending；AnchorLost/Undetectable → GenFailed 回报（不产出悬空 diff）。
    #[test]
    fn modify_requires_located_anchor_else_genfailed() {
        let p = modify_proposal();
        let ok = generate_diff(&p, "d1", "AG", "pl1", 0, LocateResult::Located).unwrap();
        assert_eq!(ok.status, DiffStatus::Pending);
        // 锚失效 / 检测不可得（保守）→ 明确失败
        for lost in [LocateResult::AnchorLost, LocateResult::Undetectable] {
            let err = generate_diff(&p, "d1", "AG", "pl1", 0, lost).unwrap_err();
            assert_eq!(err.code, "DE_ANCHOR_LOST");
            assert!(err.retriable);
        }
    }

    // covers: BR-DE-DATA-001
    // 新建文档无既有锚：即便 locate 非 Located 也照常生成（不因不存在的锚失败）。
    #[test]
    fn new_doc_skips_anchor_locate() {
        let p = new_proposal();
        let d = generate_diff(&p, "d2", "CBT", "pl2", 0, LocateResult::AnchorLost).unwrap();
        assert_eq!(d.status, DiffStatus::Pending);
        assert_eq!(d.target.rel_path, "new.md");
    }

    // covers: BR-AG-DATA-003
    // DE 侧协同判定（AG-DELETE-01 放宽）：当前文档/声明集内/新建放行；未声明既有 → false
    //（由 AG 层标注 cross_doc + DiffCard「跨文档目标」确认承担安全，不再硬拒）。
    #[test]
    fn de_side_flags_undeclared_existing_target() {
        let declared = vec!["ref.md".to_string()];
        assert!(target_declared("a.md", Some("a.md"), &declared, true));
        assert!(target_declared("ref.md", Some("a.md"), &declared, true));
        assert!(target_declared("new.md", Some("a.md"), &declared, false)); // 新建放行
        assert!(!target_declared("other.md", Some("a.md"), &declared, true)); // 未声明既有 → 跨文档标注
        assert!(!target_declared("other.md", None, &[], true));
    }
}
