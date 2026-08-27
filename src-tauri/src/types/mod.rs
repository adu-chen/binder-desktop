//! PI-SYS-02 · 跨模块共享数据结构 · BR-SYS-DATA-002 / X-SHARED-001
//!
//! 承接 A-SYS-I-P-01：跨模块传递的实体/类型/枚举契约的单一来源（SSOT）。
//! 字段语义对齐 A-SYS-I-P-01 §3；具体序列化与时间标量为实现选型（A 类债务）。

use serde::{Deserialize, Serialize};

/**
 * @GOV
 * codes:    BR-SYS-DATA-002-DATA-SYS-DATA-MIGRATE-001
 * type:     DATA
 * chain:    DATA-MIGRATE
 * rules:    BR-SYS-DATA-002
 * boundary: in=structDefinitions | out=WorkspaceRef,DocRef,PendingDiff,DiffProjection,TimelineEntry,WorkspaceTimelineEntry,WorkspaceOperationPlan,WorkspaceOperationEntry,CreateDocumentBundle,RefInjection,ReferencePayload,MemoryInjection,KnowledgeInjection
 */
// 跨模块共享实体与枚举集合的单一契约来源（BR-SYS-DATA-002）：任何模块不得在本模块之外
// 私自重定义这些结构或枚举集合；状态转移语义不属本块，归各 owner 状态机。

/// workspace 引用（边界标识，A-SYS-I-P-01 §3 / REQ-CORE-001）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceRef {
    pub workspace_id: String,
    /// 用户选定的本地根目录绝对路径——文件 I/O / 搜索 / diff / 上下文的唯一边界。
    pub root_path: String,
    pub is_active: bool,
}

/// workspace 内文档定位（A-SYS-I-P-01 §3）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocRef {
    pub workspace_id: String,
    /// workspace 内相对路径；不得越出 root_path（X-FORBID-004）。
    pub rel_path: String,
    /// 文档类型（md / docx 等）。
    pub doc_type: String,
}

/// diff 状态枚举集合（单一来源，BR-SYS-DATA-002）。
/// 转移语义（pending→accepted/rejected/expired）归 DE 状态机，不在此定义。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffStatus {
    Pending,
    Accepted,
    Rejected,
    Expired,
}

/// 待裁决 diff（跨模块持久化共享结构，REQ-CORE-002）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingDiff {
    pub diff_id: String,
    pub target: DocRef,
    /// 生成来源模块编码（DE / AG / CBT 等）。
    pub origin_module: String,
    pub status: DiffStatus,
    /// 创建时间（epoch 毫秒；标量为实现选型）。
    pub created_at: i64,
    /// diff 内容载荷引用；内容形状归 DE，本结构仅占位。
    pub payload_ref: String,
}

/// AG 生产、DE 持有、WS/ED/前端消费的统一待裁决投影（A-SYS-I-P-01 §3）。
/// 跨模块字段只允许在本共享契约中定义；各模块可持有或转换，不得复制结构体定义。
#[derive(Debug, Clone, Serialize)]
pub(crate) struct DiffProjection {
    pub(crate) diff_id: String,
    pub(crate) rel_path: String,
    pub(crate) patch_kind: String,
    pub(crate) block_id: Option<String>,
    pub(crate) block_index: Option<usize>,
    pub(crate) before_text: String,
    pub(crate) after_text: String,
    pub(crate) location_label: String,
    pub(crate) draft_mode: String,
    pub(crate) snapshot_source: String,
    pub(crate) base_session_version: Option<u64>,
    pub(crate) base_content_hash: Option<String>,
    pub(crate) style_ops: Option<serde_json::Value>,
    pub(crate) block_ops: Option<serde_json::Value>,
    pub(crate) table_ops: Option<serde_json::Value>,
    pub(crate) image_ops: Option<serde_json::Value>,
    pub(crate) batch: Option<serde_json::Value>,
    pub(crate) preview_checkpoint_required: bool,
    pub(crate) cross_doc: Option<bool>,
}

/// 文档级时间轴条目基线（REQ-CORE-007，主责 WS）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineEntry {
    pub entry_id: String,
    pub target: DocRef,
    /// 文档版本序，单调递增。
    pub version_seq: u64,
    pub label: Option<String>,
    pub created_at: i64,
}

/// 工作区资源操作状态（TERM-WS-003 · SM-WS-04）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceOperationStatus {
    Planning,
    WaitingConfirm,
    Revalidating,
    Executing,
    Compensating,
    Completed,
    Rejected,
    Expired,
    FailedCompensated,
    PartialFailed,
    Interrupted,
}

impl WorkspaceOperationStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed
                | Self::Rejected
                | Self::Expired
                | Self::FailedCompensated
                | Self::PartialFailed
        )
    }
}

/// WorkspaceOperation 支持的资源副作用集合。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceOperationKind {
    CreateDocument,
    CreateFolder,
    RenameResource,
    MoveResource,
    DeleteResource,
    RestoreResource,
    DiscardCreatedResource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceResourceKind {
    File,
    Folder,
}

/// 冻结在 WorkspaceOperationPlan 内的单项资源操作。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceOperationItem {
    pub item_id: String,
    pub seq: u32,
    pub kind: WorkspaceOperationKind,
    pub source_rel_path: Option<String>,
    pub target_rel_path: Option<String>,
    pub resource_kind: WorkspaceResourceKind,
    pub recursive: bool,
    pub restore_payload_ref: Option<String>,
}

/// 计划生成时冻结、确认时重新采集并逐字段比对的资源基线。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceResourceBaseline {
    pub rel_path: String,
    pub exists: bool,
    pub resource_kind: Option<WorkspaceResourceKind>,
    pub size: Option<u64>,
    pub modified_at: Option<i64>,
    pub content_hash: Option<String>,
    pub manifest_hash: Option<String>,
}

/// 前置确认卡与执行消费者需要显示的真实影响清单。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WorkspaceOperationConsequences {
    pub affected_sessions: Vec<String>,
    pub affected_diffs: Vec<String>,
    pub affected_references: Vec<String>,
    pub recursive_count: u64,
    pub recoverable: bool,
}

/// TERM-WS-004：落库后操作项、清单、基线与影响不可原地替换；仅生命周期字段推进。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceOperationPlan {
    pub plan_id: String,
    pub operation_id: Option<String>,
    pub idempotency_key: String,
    pub workspace_id: String,
    pub task_run_id: Option<String>,
    pub source_turn_id: Option<String>,
    pub operation_items: Vec<WorkspaceOperationItem>,
    pub frozen_manifest: Vec<WorkspaceResourceBaseline>,
    pub baseline_set: Vec<WorkspaceResourceBaseline>,
    pub consequences: WorkspaceOperationConsequences,
    pub compensation_policy: String,
    pub initial_content_ref: Option<String>,
    pub decision_request_id: Option<String>,
    pub status: WorkspaceOperationStatus,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceOperationItemStatus {
    Attempted,
    Completed,
    Compensated,
    Failed,
}

/// WorkspaceOperation 的逐项真实执行/补偿回执。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceOperationItemReceipt {
    pub item_id: String,
    pub seq: u32,
    pub status: WorkspaceOperationItemStatus,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub snapshot_ref: Option<String>,
    pub completed_at: i64,
}

/// TERM-WS-006：资源操作历史载荷；历史条目只追加、不覆盖。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceOperationEntry {
    pub entry_id: String,
    pub operation_id: String,
    pub plan_id: String,
    pub workspace_id: String,
    pub before_manifest: Vec<WorkspaceResourceBaseline>,
    pub after_manifest: Vec<WorkspaceResourceBaseline>,
    pub item_receipts: Vec<WorkspaceOperationItemReceipt>,
    pub compensation_receipts: Vec<WorkspaceOperationItemReceipt>,
    pub restore_kind: String,
    pub restore_payload_ref: Option<String>,
    pub final_status: WorkspaceOperationStatus,
    pub created_at: i64,
}

/// 一次 execute 的幂等结果；重复执行返回同一回执而不重复写 DiskState。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceOperationReceipt {
    pub operation_id: String,
    pub plan_id: String,
    pub final_status: WorkspaceOperationStatus,
    pub item_receipts: Vec<WorkspaceOperationItemReceipt>,
    pub compensation_receipts: Vec<WorkspaceOperationItemReceipt>,
    pub entry_id: Option<String>,
    #[serde(default)]
    pub invalidated_diff_ids: Vec<String>,
}

/// WorkspaceTimeline 的统一跨模块投影；payload_ref 决定类型化预览/恢复器。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceTimelineEntry {
    pub entry_id: String,
    pub workspace_id: String,
    pub entry_kind: String,
    pub target_refs: Vec<DocRef>,
    pub version_seq: u64,
    pub label: Option<String>,
    pub payload_ref: String,
    pub created_at: i64,
}

/// TERM-WS-007：资源创建与后置普通内容 Diff 的持久绑定。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateDocumentBundle {
    pub plan_id: String,
    pub workspace_id: String,
    pub target: DocRef,
    pub initial_content_ref: String,
    pub operation_id: Option<String>,
    pub initial_diff_id: Option<String>,
    pub rejection_guard_ref: Option<String>,
    pub status: String,
}

/// CreateDocumentBundle 初始内容 Diff 的拒绝补偿基线。三层身份必须同时保持不变，
/// 否则只允许把 Diff 置为过期，不得删除已经继续编辑或被外部改动的真实文件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateDocumentRejectionGuard {
    pub plan_id: String,
    pub operation_id: String,
    pub target: DocRef,
    pub disk_baseline: WorkspaceResourceBaseline,
    pub logical_session_version: u64,
    pub logical_content_hash: String,
    pub created_at: i64,
}

/// 引用注入载荷基线（REQ-CORE-006，主责 REF；REF→AG/CBT 传递）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefInjection {
    pub target: DocRef,
    /// 块级锚集合；路径级引用时为空。
    pub block_ids: Option<Vec<String>>,
    /// 读取时点的目标内容（不缓存陈旧内容跨回合复用）。
    pub content: String,
    /// 来源标识，消费方可溯源到引用目标。
    pub origin_label: String,
}

/// 引用输入载荷基线（REQ-CORE-006，主责 REF；Composer→AG 显式引用主通道）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReferencePayload {
    Doc {
        #[serde(rename = "relPath")]
        rel_path: String,
        label: Option<String>,
    },
    Selection {
        #[serde(rename = "relPath")]
        rel_path: String,
        #[serde(rename = "blockId")]
        block_id: String,
        start: Option<usize>,
        end: Option<usize>,
        text: String,
        label: Option<String>,
    },
    Session {
        #[serde(rename = "sessionId")]
        session_id: String,
        title: String,
        messages: Vec<ReferenceSessionMessage>,
        label: Option<String>,
    },
    Memory {
        #[serde(rename = "entryId")]
        entry_id: Option<String>,
        title: Option<String>,
        content: String,
        label: Option<String>,
    },
    Knowledge {
        #[serde(rename = "entryId")]
        entry_id: Option<String>,
        title: Option<String>,
        content: String,
        #[serde(rename = "sourceRef")]
        source_ref: Option<String>,
        label: Option<String>,
    },
    Template {
        #[serde(rename = "templateId")]
        template_id: Option<String>,
        name: String,
        content: String,
        label: Option<String>,
    },
    Url {
        url: String,
        title: Option<String>,
        label: Option<String>,
    },
    LongText {
        text: String,
        #[serde(rename = "charCount")]
        char_count: usize,
        label: Option<String>,
    },
    /// 外部文件引用（F5，纳入 v1）：仅元数据（name/path/size），不读内容，守 X-FORBID-004。
    File {
        name: String,
        #[serde(default)]
        path: Option<String>,
        #[serde(default)]
        size: Option<u64>,
        label: Option<String>,
        /// ExternalContentRef（TERM-REF-005）：可读扩展名的外部文件内容，仅当前回合 data-only 传递
        /// （mention_block 一次性，不落库/不转存）；None = 仅元数据、未读取内容（不伪称已读）。
        #[serde(default, rename = "readContent")]
        read_content: Option<String>,
    },
}

/// 代理会话引用中的消息快照。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceSessionMessage {
    pub role: String,
    pub content: String,
}

/// 记忆注入载荷基线（主责 AST；AST→AG 传递）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryInjection {
    /// 记忆类型（分级依据，归 A-AST-M-T-01 §3）。
    pub memory_kind: String,
    pub content: String,
    /// 来源回合标识，消费方可溯源（AgentTurn）。
    pub origin_turn: String,
}

/// 受限任务实例状态（TERM-AG-015 · SM-AG-04，BR-SYS-STATE-008）。
/// 非后台队列：不自动重试、不跨重启续跑、不越过确认点。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskRunStatus {
    Planned,
    Executing,
    WaitingConfirm,
    WaitingClarification,
    Completed,
    Failed,
    Cancelled,
    Interrupted,
}

/// 任务步骤状态（TERM-AG-016 · SM-AG-04 steps）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStepStatus {
    Pending,
    Running,
    WaitingConfirm,
    Ok,
    Failed,
    Cancelled,
}

/// 任务步骤（TERM-AG-016）：TaskRun 内有依赖的受限步骤。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskStepV1 {
    pub step_id: String,
    /// 步骤类别：tool（工具调用）/ propose（提案＝W1/W2/W3 确认点）/ read / confirm。
    pub kind: String,
    pub label: String,
    pub status: TaskStepStatus,
}

/// 受限任务实例（TERM-AG-015 · SM-AG-04）：同一用户目标内组织步骤/回合的编排实例。
/// 真实工作状态只由回合/步骤/工具事件驱动；DiffCard 可经 task_run_id 关联（UNC-09）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskRunV1 {
    pub task_run_id: String,
    pub workspace_id: String,
    pub session_id: String,
    pub status: TaskRunStatus,
    pub intent_summary: Option<String>,
    pub steps: Vec<TaskStepV1>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 任务级裁决请求状态（TERM-AG-017 · DecisionRequest）：proposed → resolved。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionRequestStatus {
    Proposed,
    Resolved,
}

/// 任务级裁决请求（TERM-AG-017 · A-AG-M-D-01 §5b）：与内容裁决分离；
/// confirm/cancel 驱动 TaskRun 的 UserConfirm/UserCancel 事件，Diff 接受/拒绝不驱动 TaskRun。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionRequestV1 {
    pub request_id: String,
    pub task_run_id: String,
    /// 裁决类型：permission（权限）/ plan_choice（方案）/ scope（范围）/ manual_confirm（人工确认）。
    pub kind: String,
    pub payload: serde_json::Value,
    pub status: DecisionRequestStatus,
    /// 裁决结果：confirm / cancel（resolved 时有值）。
    pub outcome: Option<String>,
    pub proposed_at: i64,
    pub resolved_at: Option<i64>,
}

/// 任务计划步骤（TERM-AG-018 · TaskPlan）：可展示操作计划的组成步骤。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPlanStepV1 {
    pub step_id: String,
    pub seq: i32,
    pub kind: String,
    pub label: String,
    /// 依赖的前置步骤 id 列表。
    pub deps: Vec<String>,
    /// 本步所需授权（capability 声明）；confirm_point 标记确认点。
    pub required_grant: Option<String>,
    pub confirm_point: bool,
    pub status: TaskStepStatus,
}

/// 任务计划（TERM-AG-018 · TaskPlan）：IntentResult 判定复杂目标后创建的可展示操作计划（非模型思维链）；
/// 保存步骤/依赖/当前游标/所需授权/确认点/阶段结果；执行按片段推进，每片段重新授权。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPlanV1 {
    pub plan_id: String,
    pub task_run_id: String,
    pub steps: Vec<TaskPlanStepV1>,
    /// 当前游标（下一步 seq；0 起）。
    pub cursor: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 知识召回载荷基线（主责 KB；KB→AG 传递）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeInjection {
    pub content: String,
    /// 入库来源标识，可溯源到源文档/外部文本。
    pub source_ref: String,
    pub organization: Option<String>,
}

/// 块级锚定位结果（`ed.locate_block` 回执；真实 TipTap 定位归 ED 前端，后端消费此结果）。
/// 被 REF（BR-REF-VERIFY-001）与 DE（BR-DE-DATA-001）共用：`Undetectable`（检测不可得）按失效保守处理。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocateResult {
    Located,
    AnchorLost,
    Undetectable,
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: BR-SYS-DATA-002
    // diff 状态枚举集合 pending/accepted/rejected/expired 为单一来源、序列化稳定。
    #[test]
    fn diff_status_enum_set_is_single_source() {
        let cases = [
            (DiffStatus::Pending, "\"pending\""),
            (DiffStatus::Accepted, "\"accepted\""),
            (DiffStatus::Rejected, "\"rejected\""),
            (DiffStatus::Expired, "\"expired\""),
        ];
        for (variant, json) in cases {
            assert_eq!(serde_json::to_string(&variant).unwrap(), json);
            let back: DiffStatus = serde_json::from_str(json).unwrap();
            assert_eq!(back, variant);
        }
    }

    // covers: BR-SYS-DATA-002
    // 共享结构 serde 往返保持字段语义不丢失（单一契约来源可跨进程传递）。
    #[test]
    fn shared_structs_serde_roundtrip() {
        let doc = DocRef {
            workspace_id: "ws1".into(),
            rel_path: "a/b.md".into(),
            doc_type: "md".into(),
        };
        let diff = PendingDiff {
            diff_id: "d1".into(),
            target: doc.clone(),
            origin_module: "DE".into(),
            status: DiffStatus::Pending,
            created_at: 1_700_000_000_000,
            payload_ref: "p1".into(),
        };
        let json = serde_json::to_string(&diff).unwrap();
        let back: PendingDiff = serde_json::from_str(&json).unwrap();
        assert_eq!(back, diff);
        assert_eq!(back.target, doc);
    }

    // covers: BR-SYS-DATA-002
    // DiffProjection 的跨模块字段由共享类型一次性序列化，避免 AG/DE/WS 各自漂移。
    #[test]
    fn diff_projection_serializes_complete_shared_contract() {
        let projection = DiffProjection {
            diff_id: "diff-1".into(),
            rel_path: "notes/a.md".into(),
            patch_kind: "replace_document".into(),
            block_id: None,
            block_index: None,
            before_text: String::new(),
            after_text: "after".into(),
            location_label: "整篇".into(),
            draft_mode: "current_editor".into(),
            snapshot_source: "active_editor_session".into(),
            base_session_version: Some(2),
            base_content_hash: Some("hash-2".into()),
            style_ops: None,
            block_ops: None,
            table_ops: None,
            image_ops: None,
            batch: Some(serde_json::json!({ "id": "batch-1", "index": 1, "total": 2 })),
            preview_checkpoint_required: true,
            cross_doc: Some(false),
        };
        let json = serde_json::to_value(projection).unwrap();
        assert_eq!(json["preview_checkpoint_required"], true);
        assert_eq!(json["batch"]["id"], "batch-1");
        assert_eq!(json["cross_doc"], false);
    }

    // covers: BR-SYS-DATA-002
    // ReferencePayload 字段集合按 A-SYS-I-P-01 v1.4 使用 camelCase 与 tag=kind 跨端稳定。
    #[test]
    fn reference_payload_serde_roundtrip() {
        let payload = ReferencePayload::Selection {
            rel_path: "notes/a.md".into(),
            block_id: "block_1".into(),
            start: Some(2),
            end: Some(8),
            text: "引用文本".into(),
            label: None,
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"kind\":\"selection\""));
        assert!(json.contains("\"relPath\":\"notes/a.md\""));
        assert!(json.contains("\"blockId\":\"block_1\""));
        let back: ReferencePayload = serde_json::from_str(&json).unwrap();
        assert_eq!(back, payload);
    }

    // covers: BR-SYS-DATA-002
    // file 变体（F5）：tag=kind=file，可选 path/size 缺省可解析（前端 file 拖入只给 name/size）。
    #[test]
    fn reference_payload_file_serde_roundtrip() {
        let payload = ReferencePayload::File {
            name: "report.pdf".into(),
            path: None,
            size: Some(2048),
            label: None,
            read_content: None,
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"kind\":\"file\""));
        let back: ReferencePayload = serde_json::from_str(&json).unwrap();
        assert_eq!(back, payload);
        // 缺省字段（size/path/readContent）兼容前端拖入。
        let minimal: ReferencePayload =
            serde_json::from_str(r#"{"kind":"file","name":"a.pdf"}"#).unwrap();
        assert!(matches!(minimal, ReferencePayload::File { size: None, .. }));
    }
}
