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
 * boundary: in=structDefinitions | out=WorkspaceRef,DocRef,PendingDiff,TimelineEntry,RefInjection,MemoryInjection,KnowledgeInjection
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

/// 引用注入载荷基线（REQ-CORE-006，主责 REF；REF→AG/CBT 传递）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefInjection {
    pub target: DocRef,
    /// 块级锚集合；路径级引用时为空。
    pub block_ids: Option<Vec<String>>,
    /// 读取时点的目标内容（不缓存陈旧内容跨回合复用）。
    pub content: String,
    /// 来源标识，消费方可溯源到引用目标。
    pub origin_label: String,
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
}
