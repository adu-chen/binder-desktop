//! PI-ENT-01 · 门禁前置与网关调用通道 · CLOUD-INVOKE / CLOUD-GATE
//! 规则：BR-ENT-STATE-001（调用通道/流式三终态）/ BR-ENT-VERIFY-001（门禁前置无绕过）
//!
//! 承接 A-ENT-M-T-01：模型调用**仅在 GatePassed 后发起**，全部流量经 AiGateway——客户端不内置 provider
//! SDK、无直连路径（X-FORBID-001）；流式三终态（完成/中断/错误，可取消，承 BR-SYS-STATE-002）；失败统一
//! 错误形状（含 retriable/reauth_required）。ENT 是模型调用**唯一客户端门禁通道**：每次调用提交令牌经服务端
//! 裁决（X-SEQ-001），**客户端不缓存「通过」、不本地假通过**（X-FORBID-002）；GateDenied 终态不可绕过；
//! **断网等同拒绝**（明确错误，不静默排队重试）。计量/BYOK 区分由服务端裁决，客户端不据本地状态切换。

use crate::ipc::IpcError;

/// 调用通道状态。GateDenied/Completed/Interrupted/Errored 为终态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvokeState {
    Idle,
    GateSubmitting,
    Streaming,
    GateDenied,
    Completed,
    Interrupted,
    Errored,
}

/// 调用通道事件。gatePassed/gateDenied/offline 来自服务端裁决（客户端不自决）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvokeEvent {
    Submit,
    GatePassed,
    GateDenied,
    Offline,
    StreamDone,
    Cancel,
    StreamError,
}

/// 服务端门禁裁决（客户端无权自行产生 Passed）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerVerdict {
    Passed,
    Denied,
    Unreachable, // 断网/网关不可达
}

/// 门禁结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateOutcome {
    Pass,
    Deny,
}

/// 云调用失败的三类来源（认证/门禁/网关），用于统一错误形状归类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudFailure {
    Auth,
    Gate,
    Gateway,
}

/**
 * @GOV
 * codes:    BR-SYS-OBS-001-DATA-ENT-CLOUD-INVOKE-001
 * type:     DATA
 * chain:    CLOUD-INVOKE
 * rules:    BR-SYS-OBS-001
 * boundary: in=CloudFailureClassAndFields | out=unifiedIpcError(codeMessageRetriableReauth)
 */
/// 云调用统一错误形状（BR-SYS-OBS-001）：认证/门禁/网关三类失败均产出**统一 IpcError 形状**——错误码
/// + 用户可读消息 + 是否可重试 + 是否需重认证，与 IPC 统一错误形状（REQ-SYS-001）对齐，供上层一致处理
/// 与埋点。ENT 是云调用错误的客户端产出点（承接 SYS 契约）。
pub fn cloud_error(
    _kind: CloudFailure,
    code: &str,
    message: &str,
    retriable: bool,
    reauth_required: bool,
) -> IpcError {
    IpcError::new(code, message, retriable, reauth_required)
}

/// 终态判定。
pub fn is_terminal(state: InvokeState) -> bool {
    matches!(
        state,
        InvokeState::GateDenied | InvokeState::Completed | InvokeState::Interrupted | InvokeState::Errored
    )
}

/**
 * @GOV
 * codes:    BR-ENT-STATE-001-RB-ENT-CLOUD-INVOKE-001
 * type:     RB
 * chain:    CLOUD-INVOKE
 * rules:    BR-ENT-STATE-001
 * boundary: in=InvokeEvent | out=InvokeState(streamingOnlyAfterGatePassedThreeStreamTerminals)
 */
/// 调用通道转移引擎（BR-ENT-STATE-001）：**Streaming 仅经 GateSubmitting+gatePassed 可达**（门禁前置）；
/// gateDenied/offline→GateDenied 终态（断网等同拒绝）；流式三终态——streamDone→Completed、cancel→
/// Interrupted、streamError→Errored。返回 `None` = 无合法转移（含终态冻结、无门禁不得 Streaming）。
pub fn transition(state: InvokeState, event: InvokeEvent) -> Option<InvokeState> {
    use InvokeEvent as Ev;
    use InvokeState::*;
    let next = match (state, event) {
        (Idle, Ev::Submit) => GateSubmitting,
        // 门禁前置：唯一通往 Streaming 的路径。
        (GateSubmitting, Ev::GatePassed) => Streaming,
        (GateSubmitting, Ev::GateDenied) => GateDenied,
        (GateSubmitting, Ev::Offline) => GateDenied, // 断网等同拒绝
        // 流式三终态（可取消）。
        (Streaming, Ev::StreamDone) => Completed,
        (Streaming, Ev::Cancel) => Interrupted,
        (Streaming, Ev::StreamError) => Errored,
        // 其余无合法转移（终态冻结；无 gatePassed 不得 Streaming）。
        _ => return None,
    };
    Some(next)
}

/**
 * @GOV
 * codes:    BR-ENT-VERIFY-001-GUARD-ENT-CLOUD-GATE-001
 * type:     GUARD
 * chain:    CLOUD-GATE
 * rules:    BR-ENT-VERIFY-001
 * boundary: in=tokenAndServerVerdict | out=GateOutcome(serverDecidedNeverLocalPassOfflineDeny)
 */
/// 门禁裁决守卫（BR-ENT-VERIFY-001 / X-FORBID-002）：每次调用须提交令牌经**服务端**裁决——本函数
/// **无产生 Pass 的本地路径**（Pass 仅源于 ServerVerdict::Passed，不缓存复用、不本地假通过）；无令牌
/// 或 Denied/Unreachable（断网）一律 Deny（明确拒绝，不静默重试）。返回 GateOutcome 供调用通道发起。
pub fn submit_for_verdict(token: Option<&str>, verdict: ServerVerdict) -> Result<GateOutcome, IpcError> {
    // 无令牌不得提交（无本地假通过路径）；错误经统一云错误形状（BR-SYS-OBS-001）。
    let token = token.filter(|t| !t.trim().is_empty()).ok_or_else(|| {
        cloud_error(CloudFailure::Auth, "ENT_NO_TOKEN", "缺令牌：模型调用须提交令牌经服务端裁决", false, true)
    })?;
    let _ = token; // 令牌仅用于上送服务端裁决，本函数不据其本地判断
    match verdict {
        // Pass 仅源于服务端裁决——无任何本地缓存/假通过分支。
        ServerVerdict::Passed => Ok(GateOutcome::Pass),
        ServerVerdict::Denied => Ok(GateOutcome::Deny),
        ServerVerdict::Unreachable => Ok(GateOutcome::Deny), // 断网等同拒绝
    }
}

// ── PI-AG-WIRE-01：dev AI 网关 HTTP 流式调用（接 cloud/dev-gateway） ──────────────────
//
// 客户端只连 localhost dev GW（X-FORBID-001：不内置 provider SDK、无直连 provider）。GW 归一化 SSE
// （event: chunk/done/error）→ 解析为 AgTurnEvent emit 到 Channel（承 BR-ENT-STATE-001 流式三终态）。
// dev 门禁/密钥沿用 GW（真实服务端裁决 X-FORBID-002、BYOK 加密 GW-02 归正式版）。

use crate::ipc::AgTurnEvent;

const DEFAULT_GW_URL: &str = "http://localhost:8787";

pub(crate) fn gw_base_url() -> String {
    std::env::var("BINDER_GW_URL").unwrap_or_else(|_| DEFAULT_GW_URL.to_string())
}

/// GW 回传的一次工具调用（OpenAI function-calling 累积结果）。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct GwToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String, // JSON 字符串（工具参数）
}

/// 一轮 GW 调用的结果：模型给出最终内容（Done）或请求执行工具（ToolCalls，需客户端执行后再续轮）。
pub enum RoundOutcome {
    Done,
    ToolCalls(Vec<GwToolCall>),
}

/// 调 dev GW `/invoke/stream` **一轮**：content 分片 emit `Chunk`；收到 `tool_calls` → 返回 ToolCalls
/// （**不** emit Done，交客户端执行工具后续轮）；`done` 且无工具 → emit Done。密钥不经客户端
/// （GW 用 env/BYOK，X-FORBID-001）；错误以 AgTurnEvent::Error 回传并按 Done 收尾（不 panic）。
pub async fn stream_round(
    turn_id: &str,
    provider: &str,
    model: &str,
    messages: &serde_json::Value,
    tools: &serde_json::Value,
    channel: &tauri::ipc::Channel<AgTurnEvent>,
) -> Result<RoundOutcome, IpcError> {
    use futures_util::StreamExt;

    let url = format!("{}/invoke/stream", gw_base_url());
    let body = serde_json::json!({
        "provider": provider, "model": model, "messages": messages, "tools": tools
    });

    let resp = match reqwest::Client::new().post(&url).json(&body).send().await {
        Ok(r) => r,
        Err(e) => {
            let _ = channel.send(AgTurnEvent::Error {
                turn_id: turn_id.to_string(),
                code: "AG_GW_UNREACHABLE".into(),
                message: format!("dev 网关不可达（{url}）：{e}。请先启动 cloud/dev-gateway。"),
                retriable: true,
            });
            return Ok(RoundOutcome::Done);
        }
    };

    if !resp.status().is_success() {
        let code = resp.status().as_u16();
        let msg = resp.text().await.unwrap_or_default();
        let _ = channel.send(AgTurnEvent::Error {
            turn_id: turn_id.to_string(),
            code: format!("AG_GW_HTTP_{code}"),
            message: msg,
            retriable: code >= 500,
        });
        return Ok(RoundOutcome::Done);
    }

    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    let mut tool_calls: Vec<GwToolCall> = Vec::new();
    let mut done_sent = false;

    while let Some(item) = stream.next().await {
        let bytes = match item {
            Ok(b) => b,
            Err(e) => {
                let _ = channel.send(AgTurnEvent::Error {
                    turn_id: turn_id.to_string(),
                    code: "AG_GW_STREAM".into(),
                    message: format!("流式中断：{e}"),
                    retriable: true,
                });
                return Ok(RoundOutcome::Done);
            }
        };
        buf.push_str(&String::from_utf8_lossy(&bytes));

        // 按 SSE 事件块（以空行分隔）解析。
        while let Some(pos) = buf.find("\n\n") {
            let raw: String = buf[..pos].to_string();
            buf = buf[pos + 2..].to_string();
            let (mut event, mut data) = (String::new(), String::new());
            for line in raw.lines() {
                if let Some(r) = line.strip_prefix("event:") {
                    event = r.trim().to_string();
                } else if let Some(r) = line.strip_prefix("data:") {
                    data = r.trim().to_string();
                }
            }
            let json: serde_json::Value = serde_json::from_str(&data).unwrap_or(serde_json::Value::Null);
            match event.as_str() {
                "chunk" => {
                    if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
                        let _ = channel.send(AgTurnEvent::Chunk {
                            turn_id: turn_id.to_string(),
                            text: text.to_string(),
                        });
                    }
                }
                "tool_calls" => {
                    if let Some(arr) = json.get("tool_calls").and_then(|t| t.as_array()) {
                        for tc in arr {
                            if let Ok(call) = serde_json::from_value::<GwToolCall>(tc.clone()) {
                                if !call.name.is_empty() {
                                    tool_calls.push(call);
                                }
                            }
                        }
                    }
                }
                "done" => {
                    // 仅当本轮无工具请求时，done 才是真正完成终态。
                    if tool_calls.is_empty() {
                        let stop = json.get("stop_reason").and_then(|s| s.as_str()).unwrap_or("stop").to_string();
                        let _ = channel.send(AgTurnEvent::Done {
                            turn_id: turn_id.to_string(),
                            stop_reason: stop,
                        });
                        done_sent = true;
                    }
                }
                "error" => {
                    let _ = channel.send(AgTurnEvent::Error {
                        turn_id: turn_id.to_string(),
                        code: json.get("code").and_then(|c| c.as_str()).unwrap_or("AG_GW_ERR").to_string(),
                        message: json.get("message").and_then(|m| m.as_str()).unwrap_or("").to_string(),
                        retriable: json.get("retriable").and_then(|r| r.as_bool()).unwrap_or(false),
                    });
                    return Ok(RoundOutcome::Done);
                }
                _ => {}
            }
        }
    }

    if !tool_calls.is_empty() {
        return Ok(RoundOutcome::ToolCalls(tool_calls));
    }
    // 流自然结束但未收到 done：补完成终态（BR-SYS-STATE-002 三终态闭合）。
    if !done_sent {
        let _ = channel.send(AgTurnEvent::Done {
            turn_id: turn_id.to_string(),
            stop_reason: "stop".into(),
        });
    }
    Ok(RoundOutcome::Done)
}

#[cfg(test)]
mod tests {
    use super::InvokeEvent::*;
    use super::InvokeState::*;
    use super::*;

    // covers: BR-ENT-STATE-001
    // 门禁前置：Streaming 仅经 GateSubmitting+gatePassed 可达；流式三终态；断网→GateDenied；终态冻结。
    #[test]
    fn streaming_only_after_gate_and_three_stream_terminals() {
        assert_eq!(transition(Idle, Submit), Some(GateSubmitting));
        assert_eq!(transition(GateSubmitting, GatePassed), Some(Streaming));
        // 流式三终态
        assert_eq!(transition(Streaming, StreamDone), Some(Completed));
        assert_eq!(transition(Streaming, Cancel), Some(Interrupted));
        assert_eq!(transition(Streaming, StreamError), Some(Errored));
        // 门禁拒绝 / 断网 → GateDenied 终态
        assert_eq!(transition(GateSubmitting, InvokeEvent::GateDenied), Some(InvokeState::GateDenied));
        assert_eq!(transition(GateSubmitting, Offline), Some(InvokeState::GateDenied), "断网等同拒绝");
        // 结构性门禁前置：除 GateSubmitting+gatePassed 外无路径达 Streaming
        for from in [Idle, Streaming, Completed, InvokeState::GateDenied, Interrupted, Errored] {
            assert_eq!(transition(from, GatePassed), None, "{from:?} 不应可达 Streaming");
        }
        // 终态冻结
        for ev in [Submit, GatePassed, StreamDone, Cancel] {
            assert_eq!(transition(Completed, ev), None);
            assert_eq!(transition(InvokeState::GateDenied, ev), None);
        }
    }

    // covers: BR-ENT-VERIFY-001
    // 裁决唯一源服务端：Pass 仅来自 Passed；Denied/断网→Deny；无令牌拒绝（无本地假通过路径）。
    #[test]
    fn verdict_is_server_decided_never_local_pass() {
        assert_eq!(submit_for_verdict(Some("tok"), ServerVerdict::Passed).unwrap(), GateOutcome::Pass);
        assert_eq!(submit_for_verdict(Some("tok"), ServerVerdict::Denied).unwrap(), GateOutcome::Deny);
        // 断网等同拒绝（不静默重试、不本地放行）
        assert_eq!(submit_for_verdict(Some("tok"), ServerVerdict::Unreachable).unwrap(), GateOutcome::Deny);
        // 无令牌拒绝（须提交令牌经服务端裁决）
        let err = submit_for_verdict(None, ServerVerdict::Passed).unwrap_err();
        assert_eq!(err.code, "ENT_NO_TOKEN");
        assert!(submit_for_verdict(Some("  "), ServerVerdict::Passed).is_err());
    }

    // covers: BR-SYS-OBS-001
    // 云调用三类失败统一错误形状：错误码 + 消息 + 可重试 + 需重认证 四字段齐备且一致可读。
    #[test]
    fn cloud_errors_share_unified_shape() {
        for kind in [CloudFailure::Auth, CloudFailure::Gate, CloudFailure::Gateway] {
            let e = cloud_error(kind, "CLOUD_X", "失败", true, true);
            assert_eq!(e.code, "CLOUD_X");
            assert!(!e.message.is_empty());
            assert!(e.retriable && e.reauth_required); // 四字段齐备
        }
    }
}
