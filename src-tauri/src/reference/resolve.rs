//! PI-REF-01 · @mention 触发与解析 · REF-INJECT / BR-REF-DATA-001
//!
//! 承接 A-REF-M-T-01：@mention 候选列举**限 workspace 内**（经 WS，X-FORBID-004）；解析产物为引用锚
//! ReferenceAnchor（路径级 DocRef 或 块级 DocRef+blockId）；**目标不可达 → 解析失败并明确回报，不产生悬空
//! 引用**。触发输入面/候选面板/引用标记渲染必经 ED 扩展点（X-FORBID-005），REF 不直接操纵编辑器内部状态
//! （前端呈现归 ED，本后端模块只做解析与锚构造）。

use crate::ipc::IpcError;
use crate::types::{DocRef, WorkspaceRef};
use crate::workspace::resolve_in_workspace;
use std::path::Path;

/// 引用锚：路径级（整文档）或块级（文档内指定 block）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceAnchor {
    Path(DocRef),
    Block { target: DocRef, block_ids: Vec<String> },
}

/**
 * @GOV
 * codes:    BR-REF-DATA-001-GUARD-REF-REF-INJECT-001
 * type:     GUARD
 * chain:    REF-INJECT
 * rules:    BR-REF-DATA-001
 * boundary: in=workspaceMentionTarget | out=ReferenceAnchor(scopedReachableElseFailNoDangling)
 */
/// @mention 解析为引用锚（BR-REF-DATA-001）：经 WS 路径守护限 workspace 内（越界 → 拒绝，X-FORBID-004）；
/// **目标不可达（target_exists=false）→ 明确失败**（不产生悬空引用）；block_ids 非空 → 块级锚，否则路径级。
pub fn resolve_mention(
    ws: &WorkspaceRef,
    rel_path: &str,
    block_ids: Option<Vec<String>>,
    target_exists: bool,
) -> Result<ReferenceAnchor, IpcError> {
    // 限 workspace 内（越界路径由 WS 守护拒绝）。
    resolve_in_workspace(Path::new(&ws.root_path), rel_path)?;
    // 目标不可达 → 解析失败（不悬空）。
    if !target_exists {
        return Err(IpcError::new(
            "REF_UNREACHABLE",
            "引用目标不可达，解析失败（不产生悬空引用）",
            false,
            false,
        ));
    }
    let target = DocRef {
        workspace_id: ws.workspace_id.clone(),
        rel_path: rel_path.to_string(),
        doc_type: "md".into(),
    };
    match block_ids {
        Some(ids) if !ids.is_empty() => Ok(ReferenceAnchor::Block { target, block_ids: ids }),
        _ => Ok(ReferenceAnchor::Path(target)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ws() -> WorkspaceRef {
        WorkspaceRef { workspace_id: "ws1".into(), root_path: "/home/u/ws".into(), is_active: true }
    }

    // covers: BR-REF-DATA-001
    // 解析：可达路径级/块级锚正确；越界路径拒绝（X-FORBID-004）；不可达失败不悬空。
    #[test]
    fn resolves_anchors_rejects_escape_and_unreachable() {
        let ws = ws();
        // 路径级
        assert_eq!(
            resolve_mention(&ws, "notes/a.md", None, true).unwrap(),
            ReferenceAnchor::Path(DocRef {
                workspace_id: "ws1".into(),
                rel_path: "notes/a.md".into(),
                doc_type: "md".into()
            })
        );
        // 块级
        match resolve_mention(&ws, "a.md", Some(vec!["b1".into()]), true).unwrap() {
            ReferenceAnchor::Block { block_ids, .. } => assert_eq!(block_ids, vec!["b1"]),
            _ => panic!("应为块级锚"),
        }
        // 越界拒绝
        assert_eq!(
            resolve_mention(&ws, "../../etc/passwd", None, true).unwrap_err().code,
            "WS_PATH_ESCAPE"
        );
        // 不可达失败（不悬空）
        assert_eq!(resolve_mention(&ws, "gone.md", None, false).unwrap_err().code, "REF_UNREACHABLE");
    }
}
