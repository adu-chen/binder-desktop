//! Agent 编辑运行时内部类型与跨 DE/WS 的结果投影。

use crate::types::DiffProjection;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AgentEditOperation {
    ReplaceBlock,
    BlockOps,
    ReplaceDocument,
    StyleOps,
    TableOps,
    ImageOps,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(super) struct AgentProposeEditResult {
    pub(super) proposal_batch_id: String,
    pub(super) rel_path: String,
    pub(super) projections: Vec<DiffProjection>,
}

pub(super) struct EditProjectionDraft {
    pub(super) after_content: String,
    pub(super) block_id: Option<String>,
    pub(super) block_index: Option<usize>,
    pub(super) before_text: String,
    pub(super) after_text: String,
    pub(super) location_label: String,
}
