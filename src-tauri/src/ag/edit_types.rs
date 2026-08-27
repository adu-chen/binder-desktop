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
    pub(super) diff_id: String,
    pub(super) rel_path: String,
    pub(super) projection: DiffProjection,
    pub(super) batch: Option<BatchMeta>,
}

/// AG-TASK-01 F1b：批次元数据——batch_id 标识任务批次组，index/total 标识第几批。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub(super) struct BatchMeta {
    pub(super) batch_id: String,
    pub(super) index: usize,
    pub(super) total: usize,
}

pub(super) struct EditProjectionDraft {
    pub(super) after_content: String,
    pub(super) block_id: Option<String>,
    pub(super) block_index: Option<usize>,
    pub(super) before_text: String,
    pub(super) after_text: String,
    pub(super) location_label: String,
}
