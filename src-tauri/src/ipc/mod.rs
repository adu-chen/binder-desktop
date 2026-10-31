//! PI-SYS-01 · IPC 信封 / 统一错误形状 / command 注册表 · BR-SYS-DATA-003 / OBS-002 / STATE-002
//!
//! 承接 A-SYS-I-P-02：前后端跨进程调用的协议骨架（命名/信封/错误/流式逻辑契约）。
//! 物理传输机制（Channel<T> vs event，SY-02）与序列化标量为实现选型（A 类债务）。

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod system_tests;
use crate::types::ProjectReportV2;
use serde_json::Value;
use std::collections::HashSet;

/**
 * @GOV
 * codes:    BR-SYS-OBS-002-DATA-SYS-IPC-CALL-002
 * type:     DATA
 * chain:    IPC-CALL
 * rules:    BR-SYS-OBS-002
 * boundary: in=commandFailure | out=IpcError(code,message,retriable,reauthRequired)
 */
/// 统一错误形状（BR-SYS-OBS-002）：所有 command 失败以此回传；
/// 字段集合与云调用错误（BR-SYS-OBS-001）一致，是其规范父形状。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IpcError {
    /// 机器可判错误码。
    pub code: String,
    /// 用户可读消息。
    pub message: String,
    /// 是否可重试。
    pub retriable: bool,
    /// 是否需重新认证（对齐 reauth_required 语义）。
    pub reauth_required: bool,
}

impl IpcError {
    pub fn new(code: &str, message: &str, retriable: bool, reauth_required: bool) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            retriable,
            reauth_required,
        }
    }

    /// 未注册 command 的统一拒绝错误（不可重试、无需重认证）。
    pub fn command_unregistered(name: &str) -> Self {
        Self::new(
            "IPC_COMMAND_UNREGISTERED",
            &format!("未注册的 command: {name}"),
            false,
            false,
        )
    }
}

/// 请求信封（A-SYS-I-P-02 §3，语义字段）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRequest {
    pub command: String,
    /// 调用关联标识，关联响应与流式分片。
    pub correlation_id: String,
    pub params: Value,
}

/// 响应信封：result 与 error 互斥。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResponse {
    pub correlation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<IpcError>,
}

/**
 * @GOV
 * codes:    BR-SYS-STATE-002-DATA-SYS-IPC-CALL-003
 * type:     DATA
 * chain:    IPC-CALL
 * rules:    BR-SYS-STATE-002
 * boundary: in=streamFrame | out=StreamEvent(chunk,completed,interrupted,error)
 */
/// 流式回传逻辑契约（BR-SYS-STATE-002）：分片经 correlation_id 关联到发起 command，
/// 具备明确的完成 / 中断 / 错误三类终态。物理传输不在此锁定（SY-02）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum StreamEvent {
    /// 有序分片（seq 单调）。
    Chunk {
        correlation_id: String,
        seq: u64,
        data: Value,
    },
    /// 完成终态。
    Completed { correlation_id: String },
    /// 中断/取消终态（不再投递后续分片）。
    Interrupted { correlation_id: String },
    /// 错误终态（统一错误形状）。
    Error {
        correlation_id: String,
        error: IpcError,
    },
}

/// AG 回合流式领域事件（A-AG-I-P-01 §2.1）。
///
/// **与上面 `StreamEvent` 分层不同、非重复**：
/// - `StreamEvent` 是通用 IPC 传输契约（dispatch 信封 + correlation_id，BR-SYS-STATE-002）。
/// - `AgTurnEvent` 是 append-only TurnEvent 的安全 wire 投影，走 Tauri 原生 per-call
///   `Channel<AgTurnEvent>`（SY-02 选型，不经 dispatch 信封）。
///
/// envelope 字段使用 camelCase；flatten 的 payload 以 `type` + snake_case 区分分支。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgTurnEvent {
    pub event_id: String,
    pub turn_id: String,
    pub seq: u64,
    pub occurred_at: i64,
    pub payload_version: u32,
    #[serde(flatten)]
    pub event: AgTurnEventPayload,
}

/**
 * @GOV
 * codes:    BR-AG-OBS-001-DATA-AG-AG-TURN-043, BR-AG-STATE-001-DATA-AG-AG-TURN-043, BR-AG-STATE-002-DATA-AG-AG-TURN-043, BR-AG-PERSIST-001-DATA-AG-AG-TURN-043, BR-SYS-STATE-007-DATA-AG-AG-TURN-043
 * type:     DATA
 * chain:    AG-TURN, AG-TOOL
 * rules:    BR-AG-OBS-001, BR-AG-STATE-001, BR-AG-STATE-002, BR-AG-PERSIST-001, BR-SYS-STATE-007
 * term_ref: TERM-AG-001, TERM-AG-002, TERM-AG-021, TERM-AG-022
 * boundary: in=SafePersistedTurnEventPayload | out=VersionedAgTurnEventPayload(chunkIntentRouteToolEvidenceReportProposalDecisionWaitOrTerminal) | does_not=exposeGateContextProviderSecretsCapabilityGrantOrHiddenReasoning | consumer=BR-AG-STATE-001,BR-AG-STATE-002,BR-AG-PERSIST-001,BR-AG-OBS-001,BR-SYS-STATE-007
 */
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgTurnEventPayload {
    /// 模型输出文本分片。
    Chunk { text: String },
    /// 模型可见意图语义修订；projection 不包含授权对象。
    IntentUpdated { revision: u64, projection: Value },
    /// 当前可见候选路线；不是工具调度命令。
    RouteUpdated {
        revision: u64,
        summary: String,
        steps: Vec<Value>,
    },
    /// 模型请求工具调用（SM-AG-02 激活）。
    ToolRequest {
        #[serde(rename = "toolCallId")]
        tool_call_id: String,
        #[serde(rename = "toolName")]
        tool_name: String,
        params: Value,
    },
    /// 工具执行结果回注（供前端展示工具状态）。
    ToolResult {
        #[serde(rename = "toolCallId")]
        tool_call_id: String,
        #[serde(rename = "toolName")]
        tool_name: String,
        result: String,
        success: bool,
        #[serde(rename = "evidenceIds")]
        evidence_ids: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        diagnostic: Option<AgentFailureDiagnostic>,
        #[serde(
            rename = "executionReceipt",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        execution_receipt: Option<crate::types::ToolCallReceiptV1>,
    },
    /// Finalizing 从同回合 EvidenceLedger 派生的只读报告；不构成授权或完成工具。
    ProjectReport { report: ProjectReportV2 },
    /// diff 已生成待用户裁决（ChangeProposal→DE 产出，X-FORBID-006）。
    DiffProposed {
        #[serde(rename = "diffId")]
        diff_id: String,
        #[serde(rename = "proposalBatchId")]
        proposal_batch_id: String,
        #[serde(rename = "groupIndex")]
        group_index: u32,
        #[serde(rename = "groupCount")]
        group_count: u32,
        #[serde(rename = "relPath")]
        rel_path: String,
        /// 受限任务实例关联（TERM-AG-015 · BR-SYS-STATE-008，UNC-09）：DiffCard 可经此关联 TaskRun。
        #[serde(rename = "taskRunId")]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        task_run_id: Option<String>,
    },
    /// AG-1.0-TASKRUN-REWORK-IMP-RUNTIME-01 · 实时当前动作（A-AG-M-T-01 §7.2.2）：
    /// 信息流底部状态条数据源，不伪造（BR-SYS-STATE-008）。
    StepProgress {
        /// action：reading / tool / generating / proposing / deciding。
        action: String,
        label: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    /// 任务级裁决请求（TERM-AG-017）：proposed → 等待确认（确认点唯一来源，三层分离）。
    DecisionRequestProposed {
        #[serde(rename = "requestId")]
        request_id: String,
        kind: String,
        payload: Value,
    },
    /// 任务级裁决结果（confirm / cancel → TaskRun UserConfirm/UserCancel）。
    DecisionRequestResolved {
        #[serde(rename = "requestId")]
        request_id: String,
        outcome: String,
    },
    /// 澄清等待：IntentResult.ambiguity=blocking 时等待用户输入的非终态事件（BR-SYS-STATE-007）。
    /// 回合不终结、不锁存终态；用户补充输入后以同 turn_id 受控重入，或在此阶段取消。
    ClarificationRequired { text: String },
    /// 回合正常完成（含 stop_reason:"cancelled" 中断终态）。
    Done {
        #[serde(rename = "stopReason")]
        stop_reason: String,
    },
    /// 回合错误（含可重试标记）。
    Error {
        code: String,
        message: String,
        retriable: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        diagnostic: Option<AgentFailureDiagnostic>,
    },
}

/// Safe technical metadata, not execution evidence or an inferred root cause.
/// Shared contract: SYS I-P-01 §3.1.1 (P01). No message/parameter content is copied.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentFailureDiagnostic {
    pub version: u8,
    pub observed_at: String,
    pub category: String,
    pub root_cause_status: String,
}

impl AgentFailureDiagnostic {
    pub fn observed(observed_at: &str, code: &str) -> Self {
        let category = match code {
            "AG_PROMPT_BUDGET_EXCEEDED" => "core_context",
            "AG_TOOL_TIMEOUT"
            | "AG_TOOL_WORKER_FAILED"
            | "AG_TOOL_DEADLINE_UNSAFE"
            | "AG_TOOL_REPEATED_FAILURE"
            | "AG_CAPABILITY_TIMEOUT"
            | "AG_CAPABILITY_WORKER_FAILED"
            | "AG_EFFECT_PENDING"
            | "AG_EFFECT_FACT_UNAVAILABLE" => "core_control",
            // Transport/schema failures do not establish an authorization denial or model fault.
            "ENT_GATE_BAD_RESPONSE"
            | "ENT_GATE_TIMEOUT"
            | "ENT_GATE_UNREACHABLE"
            | "ENT_GATE_HTTP_ERROR"
            | "ENT_AUTH_TIMEOUT"
            | "ENT_AUTH_UNREACHABLE"
            | "ENT_AUTH_HTTP_ERROR"
            | "ENT_CLIENT_SETUP"
            | "ENT_AUTH_BAD_RESPONSE"
            | "AG_GW_TIMEOUT"
            | "AG_GW_STREAM_TIMEOUT" => "unknown",
            "AG_OPERATION_INTERACTION_INTERRUPTED" => "ui_projection",
            "AG_MODEL_CONTROL_INVALID"
            | "AG_TOOL_ARG"
            | "AG_READ_CURSOR_INVALID"
            | "AG_READ_PAGE_VERSION_REQUIRED"
            | "AG_MODEL_CONTROL_MISSING"
            | "AG_MODEL_CONTROL_CONFLICT"
            | "AG_MODEL_CORRECTION_LIMIT"
            | "AG_TOOL_ARGUMENTS_INVALID"
            | "AG_GW_STREAM_INVALID_JSON"
            | "AG_GW_STREAM_INCOMPLETE"
            | "AG_MODEL_TOOL_ID_INVALID" => "model_protocol",
            "AG_READ_PAGE_OUT_OF_RANGE" | "AG_READ_VERSION_STALE" | "AG_TOOL_RESULT_TOO_LARGE" => {
                "core_context"
            }
            _ => match observed_at {
                "context_assembly" => "core_context",
                "gate" => "authorization",
                "tool_execution" | "operation_post_processing" => "professional_capability",
                _ => "unknown",
            },
        };
        Self {
            version: 1,
            observed_at: observed_at.into(),
            category: category.into(),
            root_cause_status: "unknown".into(),
        }
    }
}

impl AgTurnEventPayload {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Chunk { .. } => "chunk",
            Self::IntentUpdated { .. } => "intent_updated",
            Self::RouteUpdated { .. } => "route_updated",
            Self::ToolRequest { .. } => "tool_request",
            Self::ToolResult { .. } => "tool_result",
            Self::ProjectReport { .. } => "project_report",
            Self::DiffProposed { .. } => "diff_proposed",
            Self::StepProgress { .. } => "step_progress",
            Self::DecisionRequestProposed { .. } => "decision_request_proposed",
            Self::DecisionRequestResolved { .. } => "decision_request_resolved",
            Self::ClarificationRequired { .. } => "clarification_required",
            Self::Done { .. } => "done",
            Self::Error { .. } => "error",
        }
    }

    pub fn terminal_status(&self) -> Option<&'static str> {
        match self {
            Self::Done { stop_reason } if stop_reason == "cancelled" => Some("cancelled"),
            Self::Done { .. } => Some("completed"),
            Self::Error { code, .. }
                if matches!(code.as_str(), "AG_GATE_DENIED" | "AG_GATE_OFFLINE") =>
            {
                Some("gate_rejected")
            }
            Self::Error { .. } => Some("error"),
            _ => None,
        }
    }
}

/**
 * @GOV
 * codes:    BR-SYS-DATA-003-API-SYS-IPC-CALL-001
 * type:     API
 * chain:    IPC-CALL
 * rules:    BR-SYS-DATA-003
 * boundary: in=CommandRequest | out=registrationVerdict(okOrIpcError)
 */
/// command 注册表（BR-SYS-DATA-003）：跨进程调用必经已注册 command；
/// 未注册的 command 一律按统一错误形状拒绝，不进入路由。
#[derive(Debug, Default, Clone)]
pub struct CommandRegistry {
    registered: HashSet<String>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, name: &str) {
        self.registered.insert(name.to_string());
    }

    pub fn is_registered(&self, name: &str) -> bool {
        self.registered.contains(name)
    }

    /// 校验请求的 command 是否注册；未注册返回统一错误形状（BR-SYS-DATA-003 / BR-SYS-OBS-002）。
    pub fn validate(&self, req: &CommandRequest) -> Result<(), IpcError> {
        if self.is_registered(&req.command) {
            Ok(())
        } else {
            Err(IpcError::command_unregistered(&req.command))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: BR-SYS-STATE-007, BR-AG-PERSIST-001
    #[test]
    fn failure_diagnostic_is_additive_and_does_not_claim_root_cause() {
        let legacy = r#"{"type":"error","code":"E","message":"m","retriable":false}"#;
        let event: AgTurnEventPayload = serde_json::from_str(legacy).unwrap();
        assert!(matches!(
            event,
            AgTurnEventPayload::Error {
                diagnostic: None,
                ..
            }
        ));
        let diagnostic = AgentFailureDiagnostic::observed("runtime", "AG_PROMPT_BUDGET_EXCEEDED");
        assert_eq!(diagnostic.category, "core_context");
        assert_eq!(diagnostic.root_cause_status, "unknown");
        assert_eq!(
            AgentFailureDiagnostic::observed("runtime", "AG_MODEL_CONTROL_INVALID").category,
            "model_protocol"
        );
        for code in [
            "AG_MODEL_CONTROL_CONFLICT",
            "AG_TOOL_ARGUMENTS_INVALID",
            "AG_READ_CURSOR_INVALID",
            "AG_MODEL_CORRECTION_LIMIT",
        ] {
            assert_eq!(
                AgentFailureDiagnostic::observed("tool_validation", code).category,
                "model_protocol"
            );
        }
        for code in ["AG_GW_STREAM_INVALID_JSON", "AG_GW_STREAM_INCOMPLETE"] {
            let diagnostic = AgentFailureDiagnostic::observed("runtime", code);
            assert_eq!(diagnostic.observed_at, "runtime");
            assert_eq!(diagnostic.category, "model_protocol");
            assert_eq!(diagnostic.root_cause_status, "unknown");
        }
        assert_eq!(
            AgentFailureDiagnostic::observed("runtime", "OTHER_MODEL_ERROR").category,
            "unknown"
        );
        assert_eq!(
            AgentFailureDiagnostic::observed("gate", "ENT_GATE_BAD_RESPONSE").category,
            "unknown"
        );
        assert_eq!(
            AgentFailureDiagnostic::observed("gate", "AG_GATE_DENIED").category,
            "authorization"
        );
        for code in ["AG_GW_TIMEOUT", "AG_GW_STREAM_TIMEOUT"] {
            assert_eq!(
                AgentFailureDiagnostic::observed("runtime", code).category,
                "unknown"
            );
        }
        for code in [
            "ENT_GATE_BAD_RESPONSE",
            "ENT_GATE_TIMEOUT",
            "ENT_GATE_UNREACHABLE",
            "ENT_GATE_HTTP_ERROR",
            "ENT_AUTH_TIMEOUT",
            "ENT_AUTH_UNREACHABLE",
            "ENT_AUTH_HTTP_ERROR",
            "ENT_AUTH_BAD_RESPONSE",
            "ENT_CLIENT_SETUP",
            "AG_GW_TIMEOUT",
            "UNRELATED_401",
            "UNKNOWN_GATE_ERROR",
        ] {
            let event = AgTurnEventPayload::Error {
                code: code.into(),
                message: "failure".into(),
                retriable: true,
                diagnostic: None,
            };
            assert_eq!(event.terminal_status(), Some("error"));
        }
        for code in ["AG_GATE_DENIED", "AG_GATE_OFFLINE"] {
            let event = AgTurnEventPayload::Error {
                code: code.into(),
                message: "denied".into(),
                retriable: false,
                diagnostic: None,
            };
            assert_eq!(event.terminal_status(), Some("gate_rejected"));
        }
        let failure = crate::ag::history::TurnFailure {
            code: "E".into(),
            message: "m".into(),
            retriable: false,
            diagnostic: Some(diagnostic.clone()),
        };
        let restored: crate::ag::history::TurnFailure =
            serde_json::from_value(serde_json::to_value(failure).unwrap()).unwrap();
        assert_eq!(restored.diagnostic, Some(diagnostic));
        let old: crate::ag::history::TurnFailure =
            serde_json::from_str(r#"{"code":"E","message":"m","retriable":false}"#).unwrap();
        assert!(old.diagnostic.is_none());
    }

    // covers: BR-SYS-OBS-002
    // 统一错误形状含四要素（code/message/retriable/reauth_required）且 serde 往返稳定。
    #[test]
    fn ipc_error_has_four_fields_and_roundtrips() {
        let e = IpcError::new("X_FAIL", "失败", true, false);
        let j = serde_json::to_string(&e).unwrap();
        for f in ["code", "message", "retriable", "reauth_required"] {
            assert!(j.contains(f), "缺字段 {f}");
        }
        let back: IpcError = serde_json::from_str(&j).unwrap();
        assert_eq!(back, e);
    }

    // covers: BR-SYS-STATE-002
    // 流式具备完成/中断/错误三终态，分片经 correlation_id 关联，serde 往返稳定。
    #[test]
    fn stream_event_has_three_terminal_states_and_correlated_chunks() {
        let terminals = [
            StreamEvent::Completed {
                correlation_id: "c".into(),
            },
            StreamEvent::Interrupted {
                correlation_id: "c".into(),
            },
            StreamEvent::Error {
                correlation_id: "c".into(),
                error: IpcError::new("E", "m", false, false),
            },
        ];
        for ev in terminals {
            let j = serde_json::to_string(&ev).unwrap();
            let _back: StreamEvent = serde_json::from_str(&j).unwrap();
        }
        let chunk = StreamEvent::Chunk {
            correlation_id: "corr-1".into(),
            seq: 0,
            data: serde_json::json!("tok"),
        };
        assert!(serde_json::to_string(&chunk).unwrap().contains("corr-1"));
    }

    // covers: BR-SYS-STATE-002
    // AgTurnEvent（A-AG-I-P-01 §2.1）：envelope camelCase + payload tag=type/snake_case；
    // event_id/seq/time/version 与安全 payload 同时往返。
    #[test]
    fn ag_turn_event_serializes_with_type_tag_and_has_terminal_states() {
        let event = |payload| AgTurnEvent {
            event_id: "evt:t1:1".into(),
            turn_id: "t1".into(),
            seq: 1,
            occurred_at: 42,
            payload_version: 1,
            event: payload,
        };
        let chunk = event(AgTurnEventPayload::Chunk { text: "tok".into() });
        let j = serde_json::to_string(&chunk).unwrap();
        assert!(j.contains("\"type\":\"chunk\""), "tag=type/snake_case: {j}");
        assert!(j.contains("\"eventId\":\"evt:t1:1\""));
        assert!(j.contains("\"turnId\":\"t1\""));
        assert!(j.contains("\"payloadVersion\":1"));

        let diff = event(AgTurnEventPayload::DiffProposed {
            diff_id: "d1".into(),
            proposal_batch_id: "pb1".into(),
            group_index: 1,
            group_count: 1,
            rel_path: "a.md".into(),
            task_run_id: None,
        });
        let diff_json = serde_json::to_string(&diff).unwrap();
        assert!(diff_json.contains("\"type\":\"diff_proposed\""));
        assert!(diff_json.contains("\"proposalBatchId\":\"pb1\""));

        // 三终态：完成 / 中断（done+cancelled）/ 错误
        let terminals = [
            event(AgTurnEventPayload::Done {
                stop_reason: "stop".into(),
            }),
            event(AgTurnEventPayload::Done {
                stop_reason: "cancelled".into(),
            }),
            event(AgTurnEventPayload::Error {
                code: "E".into(),
                message: "m".into(),
                retriable: false,
                diagnostic: None,
            }),
        ];
        for ev in terminals {
            let s = serde_json::to_string(&ev).unwrap();
            let _back: AgTurnEvent = serde_json::from_str(&s).unwrap();
        }
    }

    // covers: BR-SYS-DATA-003
    // 未注册 command 经统一错误形状拒绝、不进入路由；已注册放行。
    #[test]
    fn unregistered_command_rejected_with_unified_error() {
        let mut reg = CommandRegistry::new();
        reg.register("ws.open_workspace");

        let ok = CommandRequest {
            command: "ws.open_workspace".into(),
            correlation_id: "1".into(),
            params: Value::Null,
        };
        assert!(reg.validate(&ok).is_ok());

        let bad = CommandRequest {
            command: "ws.unknown".into(),
            correlation_id: "2".into(),
            params: Value::Null,
        };
        let err = reg.validate(&bad).unwrap_err();
        assert_eq!(err.code, "IPC_COMMAND_UNREGISTERED");
        assert!(!err.retriable);
    }
}
