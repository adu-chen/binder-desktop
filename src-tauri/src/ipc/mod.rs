//! PI-SYS-01 · IPC 信封 / 统一错误形状 / command 注册表 · BR-SYS-DATA-003 / OBS-002 / STATE-002
//!
//! 承接 A-SYS-I-P-02：前后端跨进程调用的协议骨架（命名/信封/错误/流式逻辑契约）。
//! 物理传输机制（Channel<T> vs event，SY-02）与序列化标量为实现选型（A 类债务）。

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod system_tests;
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

/// AG 回合流式领域事件（A-AG-I-P-01 §2.1 · PI-AG-10C 契约冻结点）。
///
/// **与上面 `StreamEvent` 分层不同、非重复**：
/// - `StreamEvent` 是通用 IPC 传输契约（dispatch 信封 + correlation_id，BR-SYS-STATE-002）。
/// - `AgTurnEvent` 是 AG 回合**领域**事件，走 Tauri 原生 per-call `Channel<AgTurnEvent>`
///   （SY-02 选型，不经 dispatch 信封；channel 本身即回合关联，故用 turn_id 而非 correlation_id）。
///
/// 三终态（Done / Error / Done{stop_reason:"cancelled"}）语义承 BR-SYS-STATE-002 的
/// 完成/错误/中断，但因传输层不同（dedicated channel vs dispatch envelope）用独立 wire 类型，
/// 不复用上面的 `StreamEvent`，避免把领域事件埋进通用 `data`。
///
/// 前端按 `type` 标签分发（PI-AG-10F）：chunk→appendToMessage、tool_request/tool_result→工具卡片、
/// diff_proposed→deStore.stageDiff、done→finishMessage、error→pushError。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgTurnEvent {
    /// 模型输出文本分片。
    Chunk { turn_id: String, text: String },
    /// 模型请求工具调用（SM-AG-02 激活）。
    ToolRequest {
        turn_id: String,
        tool_name: String,
        params: Value,
    },
    /// 工具执行结果回注（供前端展示工具状态）。
    ToolResult {
        turn_id: String,
        tool_name: String,
        result: String,
        success: bool,
    },
    /// diff 已生成待用户裁决（ChangeProposal→DE 产出，X-FORBID-006）。
    DiffProposed {
        turn_id: String,
        diff_id: String,
        rel_path: String,
        /// 受限任务实例关联（TERM-AG-015 · BR-SYS-STATE-008，UNC-09）：DiffCard 可经此关联 TaskRun。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        task_run_id: Option<String>,
    },
    /// AG-1.0-TASKRUN-REWORK-IMP-RUNTIME-01 · 实时当前动作（A-AG-M-T-01 §7.2.2）：
    /// 信息流底部状态条数据源，不伪造（BR-SYS-STATE-008）。
    StepProgress {
        turn_id: String,
        /// action：reading / tool / generating / proposing / deciding。
        action: String,
        label: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    /// 任务级裁决请求（TERM-AG-017）：proposed → 等待确认（确认点唯一来源，三层分离）。
    DecisionRequestProposed {
        turn_id: String,
        request_id: String,
        kind: String,
        payload: Value,
    },
    /// 任务级裁决结果（confirm / cancel → TaskRun UserConfirm/UserCancel）。
    DecisionRequestResolved {
        turn_id: String,
        request_id: String,
        outcome: String,
    },
    /// 澄清等待：IntentResult.ambiguity=blocking 时等待用户输入的非终态事件（BR-SYS-STATE-007）。
    /// 回合不终结、不锁存终态；用户补充输入后以同 turn_id 受控重入，或在此阶段取消。
    ClarificationRequired { turn_id: String, text: String },
    /// 回合正常完成（含 stop_reason:"cancelled" 中断终态）。
    Done {
        turn_id: String,
        stop_reason: String,
    },
    /// 回合错误（含可重试标记）。
    Error {
        turn_id: String,
        code: String,
        message: String,
        retriable: bool,
    },
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
#[derive(Debug, Default)]
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
    // AgTurnEvent（A-AG-I-P-01 §2.1）：六分支 serde 形态 tag=type + snake_case；
    // 三终态语义（done / done{cancelled} / error）承 BR-SYS-STATE-002，与通用 StreamEvent 分层不同。
    #[test]
    fn ag_turn_event_serializes_with_type_tag_and_has_terminal_states() {
        let chunk = AgTurnEvent::Chunk {
            turn_id: "t1".into(),
            text: "tok".into(),
        };
        let j = serde_json::to_string(&chunk).unwrap();
        assert!(j.contains("\"type\":\"chunk\""), "tag=type/snake_case: {j}");
        assert!(j.contains("turn_id"));

        let diff = AgTurnEvent::DiffProposed {
            turn_id: "t1".into(),
            diff_id: "d1".into(),
            rel_path: "a.md".into(),
            task_run_id: None,
        };
        assert!(serde_json::to_string(&diff)
            .unwrap()
            .contains("\"type\":\"diff_proposed\""));

        // 三终态：完成 / 中断（done+cancelled）/ 错误
        let terminals = [
            AgTurnEvent::Done {
                turn_id: "t1".into(),
                stop_reason: "stop".into(),
            },
            AgTurnEvent::Done {
                turn_id: "t1".into(),
                stop_reason: "cancelled".into(),
            },
            AgTurnEvent::Error {
                turn_id: "t1".into(),
                code: "E".into(),
                message: "m".into(),
                retriable: false,
            },
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
