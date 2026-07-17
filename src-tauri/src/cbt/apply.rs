//! PI-CBT-02 · 产物 diff 化落盘 · CBT-BUILD / BR-CBT-DATA-001 / X-FORBID-006
//!
//! 承接 A-CBT-M-T-01 §3：构建产物**逐任务生成 ChangeProposal 交 DE**（de.generate_diff）生成待裁决修改，
//! **不存在旁路写盘**（X-FORBID-006 唯一落盘路径经 WS）。修改既有文档带 before_snapshot；**新建文档以空
//! 基准表达**（before_snapshot=None，accepted 后由 WS 创建写入）；多任务产物按任务序提交，支持按条裁决。
//! 本模块只构造提案载荷——**无任何文件系统写入**（旁路写盘在结构上不可达）；DE 载荷形状归 PI-DE-01。

use crate::de::generate::ChangeProposal; // DE 单一拥有 diff 载荷形状（BR-DE-DATA-002）
use crate::types::DocRef;

/**
 * @GOV
 * codes:    BR-CBT-DATA-001-DATA-CBT-CBT-BUILD-001
 * type:     DATA
 * chain:    CBT-BUILD
 * rules:    BR-CBT-DATA-001
 * boundary: in=BuildTaskArtifact | out=ChangeProposalForDE(noBypassWrite,newDocEmptyBaseline)
 */
/// 构造单任务 ChangeProposal（BR-CBT-DATA-001 / X-FORBID-006）：产物只表达为交 DE 的修改建议，
/// **不写文件**（无旁路落盘）；新建文档 before_snapshot=None（空基准），修改既有文档带 Some(基准)。
pub fn build_proposal(
    origin_task: &str,
    target: DocRef,
    after_content: Vec<u8>,
    before_snapshot: Option<String>,
) -> ChangeProposal {
    ChangeProposal {
        origin_task: origin_task.to_string(),
        target,
        after_content,
        before_snapshot,
        style_ops: None, // CBT 构建产出暂无样式操作（PI-ED-11 载荷字段，AG/CBT 真实产出接线时填）
    }
}

/**
 * @GOV
 * codes:    BR-CBT-DATA-001-DATA-CBT-CBT-BUILD-002
 * type:     DATA
 * chain:    CBT-BUILD
 * rules:    BR-CBT-DATA-001
 * boundary: in=orderedBuildTasks | out=changeProposalSequence(perTaskAdjudicationOrderPreserved)
 */
/// 按任务序批量构造提案（BR-CBT-DATA-001）：保持任务顺序，使产物可**按条裁决**（部分接受时仅
/// accepted 任务落盘，其余不影响——落盘逻辑归 DE/WS，本处只保序成列）。
pub fn build_proposal_sequence(
    tasks: Vec<(String, DocRef, Vec<u8>, Option<String>)>,
) -> Vec<ChangeProposal> {
    tasks
        .into_iter()
        .map(|(t, target, after, before)| build_proposal(&t, target, after, before))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(rel: &str) -> DocRef {
        DocRef {
            workspace_id: "ws1".into(),
            rel_path: rel.into(),
            doc_type: "md".into(),
        }
    }

    // covers: BR-CBT-DATA-001
    // 新建文档空基准（None）；修改既有带基准（Some）；构造提案不写任何文件（无旁路写盘）。
    #[test]
    fn new_doc_empty_baseline_modify_has_baseline_no_file_write() {
        // 新建文档：before_snapshot = None（空基准）
        let new_path = std::env::temp_dir().join("binder_cbt_should_not_exist.md");
        let _ = std::fs::remove_file(&new_path);
        let p_new = build_proposal("task1", doc("new.md"), b"hello".to_vec(), None);
        assert!(p_new.before_snapshot.is_none(), "新建文档应空基准");
        // 修改既有：带 before_snapshot
        let p_mod = build_proposal(
            "task2",
            doc("exist.md"),
            b"edited".to_vec(),
            Some("h0".into()),
        );
        assert_eq!(p_mod.before_snapshot.as_deref(), Some("h0"));
        // 构造提案不产生任何文件（旁路写盘结构不可达）
        assert!(!new_path.exists(), "build_proposal 不得写盘");
    }

    // covers: BR-CBT-DATA-001
    // 多任务产物按任务序成列，支持按条裁决（顺序保持、各任务可溯）。
    #[test]
    fn proposals_preserve_task_order_for_per_item_adjudication() {
        let seq = build_proposal_sequence(vec![
            ("t1".into(), doc("a.md"), b"A".to_vec(), None),
            ("t2".into(), doc("b.md"), b"B".to_vec(), Some("hb".into())),
            ("t3".into(), doc("c.md"), b"C".to_vec(), None),
        ]);
        let order: Vec<&str> = seq.iter().map(|p| p.origin_task.as_str()).collect();
        assert_eq!(order, vec!["t1", "t2", "t3"]); // 按任务序
        assert!(seq[0].before_snapshot.is_none() && seq[1].before_snapshot.is_some());
    }
}
