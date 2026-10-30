//! PI-AG-02 · 工具调用循环机 SM-AG-02 · AG-TOOL / BR-AG-STATE-002 / X-FORBID-006
//!
//! 承接 A-AG-M-S-01 SM-AG-02：嵌套于 SM-AG-01 Streaming 段的工具循环——模型请求工具须**在注册清单内
//! 且通过权限边界校验**方可执行（未注册/越权一律拒绝并将拒绝原因回注模型）；执行结果（含失败）回注后
//! 续调；**循环有上限**，达上限中止；用户可随时中断（终态无半落盘）；**任何文件写入意图不得由工具直接
//! 落盘**，须产出修改建议走 DE diff（X-FORBID-006）；只读工具经 WS 限 workspace 内（X-FORBID-004）。
//! 注册清单/权限分级内容延实现期运营配置，本机锁校验机制与写盘禁令。

/// SM-AG-02 状态（A-AG-M-S-01）。LoopCompleted/LoopAborted 为终态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolLoopState {
    ModelStreaming,
    ToolValidating,
    ToolExecuting,
    ResultRecording,
    LoopCompleted,
    LoopAborted,
}

/// SM-AG-02 事件。injected 携带循环计数是否仍在上限内。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolLoopEvent {
    ToolRequest,
    FinalAnswer,
    ProtocolRejected,
    ValidateOk,
    ValidateFail,
    ExecOk,
    ExecFail,
    Recorded,
    NextToolCall,
    ProposalRecorded,
    DecisionHandoff,
    LoopLimit,
    LoopError,
    Cancel,
}

/// 工具校验裁决。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolDecision {
    Allow,
    Reject(String), // 拒绝原因（回注模型）
    InvalidArguments {
        field: &'static str,
        constraint: &'static str,
        message: String,
    },
}

/// 终态判定。
pub fn is_terminal(state: ToolLoopState) -> bool {
    matches!(
        state,
        ToolLoopState::LoopCompleted | ToolLoopState::LoopAborted
    )
}

/**
 * @GOV
 * codes:    BR-AG-STATE-002-RB-AG-AG-TOOL-001
 * type:     RB
 * chain:    AG-TOOL
 * rules:    BR-AG-STATE-002
 * term_ref: TERM-AG-002
 * boundary: in=ToolLoopEvent | out=ToolLoopState(validateGatedResultRecordedProposalDecisionLoopLimitedErrorCancelTerminal)
 */
/// SM-AG-02 转移引擎（BR-AG-STATE-002）：toolRequest→校验；validateFail/execOk/execFail 均进入
/// ResultRecording；同一响应仍有未处理 ToolCall 时经 nextToolCall 保持激活回到 ModelStreaming；
/// 记录完成、Proposal 或 DecisionRequest 后结束本次嵌套循环并回归 SM-AG-01；
/// 达上限/循环级错误→LoopAborted；
/// 任意非终态 cancel→LoopAborted（无半落盘）。返回 `None` = 无合法转移。
pub fn transition(state: ToolLoopState, event: ToolLoopEvent) -> Option<ToolLoopState> {
    use ToolLoopEvent::*;
    use ToolLoopState::*;
    // 任意非终态用户中断 → LoopAborted（无半落盘）。
    if matches!(event, Cancel) && !is_terminal(state) {
        return Some(LoopAborted);
    }
    if matches!(event, LoopError) && !is_terminal(state) {
        return Some(LoopAborted);
    }
    let next = match (state, event) {
        (ModelStreaming, ToolRequest) => ToolValidating,
        (ModelStreaming, FinalAnswer) => LoopCompleted,
        (ModelStreaming, ProtocolRejected) => LoopCompleted,
        (ToolValidating, ValidateOk) => ToolExecuting,
        (ToolValidating, ValidateFail) => ResultRecording,
        (ToolExecuting, ExecOk) => ResultRecording,
        (ToolExecuting, ExecFail) => ResultRecording,
        (ResultRecording, Recorded | ProposalRecorded | DecisionHandoff) => LoopCompleted,
        // 同一响应内的下一 ToolCall 仍在当前 SM-AG-02 激活内，必须经转移引擎而非直接赋值状态。
        (ResultRecording, NextToolCall) => ModelStreaming,
        (ResultRecording, LoopLimit) => LoopAborted,
        // 其余无合法转移（终态冻结；未校验不得执行；未注入不得续调）。
        _ => return None,
    };
    Some(next)
}

/**
 * @GOV
 * codes:    BR-AG-STATE-002-GUARD-AG-AG-TOOL-001
 * type:     GUARD
 * chain:    AG-TOOL
 * rules:    BR-AG-STATE-002
 * boundary: in=toolRegistrationPermissionAndWriteIntent | out=ToolDecision(rejectUnregisteredOverreachNoDirectWrite)
 */
/// 工具校验守卫（BR-AG-STATE-002 / X-FORBID-006）：未注册 → 拒绝；越权（不在权限边界）→ 拒绝；
/// **写入意图工具不得直接落盘** → 拒绝直写（须产出修改建议走 DE diff）。拒绝均带原因（回注模型）。
/// 注册清单与权限分级具体内容为实现期运营配置，本守卫锁校验机制与写盘禁令。
pub fn validate_tool(
    registered: bool,
    within_permission: bool,
    direct_write_intent: bool,
) -> ToolDecision {
    if !registered {
        return ToolDecision::Reject("工具未在注册清单内".into());
    }
    if !within_permission {
        return ToolDecision::Reject("工具越出权限边界".into());
    }
    if direct_write_intent {
        // X-FORBID-006：写入意图须走 DE diff，不得工具直写落盘。
        return ToolDecision::Reject("写入意图不得直接落盘，须产出修改建议走 DE diff".into());
    }
    ToolDecision::Allow
}

#[cfg(test)]
mod tests {
    use super::ToolLoopEvent::*;
    use super::ToolLoopState::*;
    use super::*;

    // covers: BR-AG-STATE-002
    // 循环：请求→校验→执行→回注→续调；校验失败回注不中断；上限/错误中止；终答/移交完成；终态冻结。
    #[test]
    fn loop_validate_inject_limit_and_terminals() {
        assert_eq!(
            transition(ModelStreaming, ToolRequest),
            Some(ToolValidating)
        );
        assert_eq!(transition(ToolValidating, ValidateOk), Some(ToolExecuting));
        assert_eq!(transition(ToolExecuting, ExecOk), Some(ResultRecording));
        assert_eq!(transition(ResultRecording, Recorded), Some(LoopCompleted));
        // 校验失败/执行失败均回注，不中断循环
        assert_eq!(
            transition(ToolValidating, ValidateFail),
            Some(ResultRecording)
        );
        assert_eq!(transition(ToolExecuting, ExecFail), Some(ResultRecording));
        // 终答 / 上限
        assert_eq!(transition(ModelStreaming, FinalAnswer), Some(LoopCompleted));
        assert_eq!(
            transition(ModelStreaming, ProtocolRejected),
            Some(LoopCompleted)
        );
        assert_eq!(transition(ToolExecuting, ProtocolRejected), None);
        assert_eq!(
            transition(ResultRecording, ProposalRecorded),
            Some(LoopCompleted)
        );
        assert_eq!(
            transition(ResultRecording, DecisionHandoff),
            Some(LoopCompleted)
        );
        // 同批下一 ToolCall 保持同一激活，不进入终态。
        assert_eq!(
            transition(ResultRecording, NextToolCall),
            Some(ModelStreaming)
        );
        assert_eq!(transition(ResultRecording, LoopLimit), Some(LoopAborted));
        // 未校验不得执行
        assert_eq!(transition(ModelStreaming, ValidateOk), None);
        // 终态冻结
        for ev in [
            ToolRequest,
            ValidateOk,
            Recorded,
            ProposalRecorded,
            LoopError,
            Cancel,
        ] {
            assert_eq!(transition(LoopCompleted, ev), None);
            assert_eq!(transition(LoopAborted, ev), None);
        }
    }

    // covers: BR-AG-STATE-002
    // 任意非终态可中断 → LoopAborted（无半落盘）。
    #[test]
    fn cancel_aborts_from_any_active_state() {
        for s in [
            ModelStreaming,
            ToolValidating,
            ToolExecuting,
            ResultRecording,
        ] {
            assert_eq!(transition(s, Cancel), Some(LoopAborted));
        }
    }

    // covers: BR-AG-STATE-002
    // provider/Gate/TaskPlan 等循环级错误从任意活动态统一进入 LoopAborted。
    #[test]
    fn loop_error_aborts_from_any_active_state() {
        for s in [
            ModelStreaming,
            ToolValidating,
            ToolExecuting,
            ResultRecording,
        ] {
            assert_eq!(transition(s, LoopError), Some(LoopAborted));
        }
    }

    // covers: BR-AG-STATE-002
    // 工具校验：未注册/越权拒绝；写入意图禁直写（须走 DE diff）；合规放行。
    #[test]
    fn validate_rejects_unregistered_overreach_and_direct_write() {
        assert_eq!(validate_tool(true, true, false), ToolDecision::Allow);
        assert!(matches!(
            validate_tool(false, true, false),
            ToolDecision::Reject(_)
        ));
        assert!(matches!(
            validate_tool(true, false, false),
            ToolDecision::Reject(_)
        ));
        // 写入意图不得直写
        match validate_tool(true, true, true) {
            ToolDecision::Reject(r) => assert!(r.contains("DE diff")),
            _ => panic!("写入意图应被拒绝直写"),
        }
    }
}
