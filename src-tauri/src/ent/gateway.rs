//! PI-ENT-01 · 门禁前置与网关调用通道 · CLOUD-INVOKE / CLOUD-GATE
//! 规则：BR-ENT-STATE-001（调用通道/流式三终态）/ BR-ENT-VERIFY-001（门禁前置无绕过）
//!
//! 承接 A-ENT-M-T-01：模型调用**仅在 GatePassed 后发起**，全部流量经 AiGateway——客户端不内置 provider
//! SDK、无直连路径（X-FORBID-001）；流式三终态（完成/中断/错误，可取消，承 BR-SYS-STATE-002）；失败统一
//! 错误形状（含 retriable/reauth_required）。ENT 是模型调用**唯一客户端门禁通道**：每次调用提交令牌经服务端
//! 裁决（X-SEQ-001），**客户端不缓存「通过」、不本地假通过**（X-FORBID-002）；GateDenied 终态不可绕过；
//! **断网等同拒绝**（明确错误，不静默排队重试）。计量/BYOK 区分由服务端裁决，客户端不据本地状态切换。

use crate::ipc::IpcError;

/// Provider 默认模型。显式模型参数始终优先；该表只服务未指定模型的运行时请求。
pub(crate) fn default_model(provider: &str) -> String {
    match provider {
        "openai" => "gpt-4o-mini",
        "anthropic" => "claude-sonnet-4-6",
        _ => "deepseek-chat",
    }
    .to_string()
}

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
        InvokeState::GateDenied
            | InvokeState::Completed
            | InvokeState::Interrupted
            | InvokeState::Errored
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
 * codes:    BR-ENT-VERIFY-001-GUARD-ENT-CLOUD-GATE-002
 * type:     GUARD
 * chain:    CLOUD-GATE
 * rules:    BR-ENT-VERIFY-001
 * boundary: in=tokenAndServerVerdict | out=GateOutcome(serverDecidedNeverLocalPassOfflineDeny)
 */
/// 门禁裁决守卫（BR-ENT-VERIFY-001 / X-FORBID-002）：每次调用须提交令牌经**服务端**裁决——本函数
/// **无产生 Pass 的本地路径**（Pass 仅源于 ServerVerdict::Passed，不缓存复用、不本地假通过）；无令牌
/// 或 Denied/Unreachable（断网）一律 Deny（明确拒绝，不静默重试）。返回 GateOutcome 供调用通道发起。
pub fn submit_for_verdict(
    token: Option<&str>,
    verdict: ServerVerdict,
) -> Result<GateOutcome, IpcError> {
    // 无令牌不得提交（无本地假通过路径）；错误经统一云错误形状（BR-SYS-OBS-001）。
    let token = token.filter(|t| !t.trim().is_empty()).ok_or_else(|| {
        cloud_error(
            CloudFailure::Auth,
            "ENT_NO_TOKEN",
            "缺令牌：模型调用须提交令牌经服务端裁决",
            false,
            true,
        )
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

// PI-AG-GATE-01 · 会话令牌缓存（auth 凭据，非门禁裁决缓存——每次调用仍经服务端 checkGate，X-FORBID-002）。
static SESSION_TOKEN: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// 取会话令牌：已有缓存即用，否则 dev auto-login（POST /auth/login）。失败回 None（GW 将以无令牌拒绝）。
async fn ensure_session_token() -> Option<String> {
    if let Some(tok) = SESSION_TOKEN.lock().ok().and_then(|g| g.as_ref().cloned()) {
        return Some(tok);
    }
    let url = format!("{}/auth/login", gw_base_url());
    let resp = match reqwest::Client::new()
        .post(&url)
        .json(&serde_json::json!({ "email": "dev@binder.local" }))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[ag-gw] ✗ auth/login 不可达 {url}: {e}");
            return None;
        }
    };
    if !resp.status().is_success() {
        eprintln!(
            "[ag-gw] ✗ auth/login HTTP {} from {url}",
            resp.status().as_u16()
        );
        return None;
    }
    let v: serde_json::Value = resp.json().await.ok()?;
    let token = v.get("token").and_then(|t| t.as_str())?.to_string();
    if let Ok(mut g) = SESSION_TOKEN.lock() {
        *g = Some(token.clone());
    }
    Some(token)
}

/// 清会话令牌（门禁拒绝/令牌过期 → 下轮重登录）。
fn clear_session_token() {
    if let Ok(mut g) = SESSION_TOKEN.lock() {
        *g = None;
    }
}

#[derive(Debug, serde::Deserialize)]
struct GateCheckBody {
    verdict: String,
    #[allow(dead_code)]
    reason: Option<String>,
    reauth_required: Option<bool>,
}

fn verdict_from_gate_body(body: &GateCheckBody) -> ServerVerdict {
    if body.verdict == "passed" {
        ServerVerdict::Passed
    } else {
        ServerVerdict::Denied
    }
}

/// 单次模型调用前提交令牌到 dev GW `/gate/check` 获取服务端裁决。
/// 这是 auth token 复用，不是 gate pass 缓存；每次调用仍由服务端判定。
pub async fn check_gate() -> Result<GateOutcome, IpcError> {
    let url = format!("{}/gate/check", gw_base_url());
    let client = reqwest::Client::new();
    let mut retried_reauth = false;
    loop {
        let token = ensure_session_token().await;
        let token_ref = token.as_deref();
        if token_ref.is_none() {
            return submit_for_verdict(None, ServerVerdict::Unreachable);
        }
        let resp = match client
            .post(&url)
            .json(&serde_json::json!({ "token": token_ref }))
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[ag-gw] ✗ gate/check 不可达 {url}: {e}");
                return submit_for_verdict(token_ref, ServerVerdict::Unreachable);
            }
        };
        if !resp.status().is_success() {
            if resp.status().as_u16() == 401 && !retried_reauth {
                clear_session_token();
                retried_reauth = true;
                eprintln!("[ag-gw] gate/check 401，已清令牌并重试一次");
                continue;
            }
            if resp.status().as_u16() == 401 {
                clear_session_token();
            }
            return submit_for_verdict(token_ref, ServerVerdict::Denied);
        }
        let body = resp.json::<GateCheckBody>().await.map_err(|e| {
            cloud_error(
                CloudFailure::Gate,
                "ENT_GATE_BAD_RESPONSE",
                &format!("门禁响应无法解析：{e}"),
                true,
                false,
            )
        })?;
        if body.reauth_required.unwrap_or(false) && !retried_reauth {
            clear_session_token();
            retried_reauth = true;
            eprintln!("[ag-gw] gate/check 要求重新认证，已清令牌并重试一次");
            continue;
        }
        if body.reauth_required.unwrap_or(false) {
            clear_session_token();
        }
        return submit_for_verdict(token_ref, verdict_from_gate_body(&body));
    }
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
    Done { stop_reason: String },
    ToolCalls(Vec<GwToolCall>),
    Failed(IpcError),
}

/// 调 dev GW `/invoke/stream` **一轮**：content 分片 emit `Chunk`；收到 `tool_calls` → 返回 ToolCalls
/// （**不** emit Done，交客户端执行工具后续轮）。本边界只发送非终态 Chunk；完成或错误作为
/// `RoundOutcome` 返回，由 AG Runtime 的 terminal latch 统一发送唯一终态。密钥不经客户端
/// （GW 用 env/BYOK，X-FORBID-001）。
fn request_trace_enabled() -> bool {
    std::env::var("BINDER_PROMPT_AUDIT")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn serialized_chars(value: &serde_json::Value) -> usize {
    serde_json::to_string(value)
        .map(|serialized| serialized.chars().count())
        .unwrap_or(0)
}

fn request_shape(messages: &serde_json::Value, tools: &serde_json::Value) -> serde_json::Value {
    let messages_chars = serialized_chars(messages);
    let tools_chars = serialized_chars(tools);
    serde_json::json!({
        "message_count": messages.as_array().map_or(0, Vec::len),
        "tool_count": tools.as_array().map_or(0, Vec::len),
        "serialized_messages_chars": messages_chars,
        "serialized_tools_chars": tools_chars,
        "estimated_input_tokens": (messages_chars + tools_chars + 3) / 4,
    })
}

struct RequestLifecycleTrace {
    enabled: bool,
    turn_id: String,
    round_index: usize,
    started_at: std::time::Instant,
    terminal_emitted: bool,
}

impl RequestLifecycleTrace {
    fn new(turn_id: &str, round_index: usize) -> Self {
        Self {
            enabled: request_trace_enabled(),
            turn_id: turn_id.to_string(),
            round_index,
            started_at: std::time::Instant::now(),
            terminal_emitted: false,
        }
    }

    fn elapsed_ms(&self) -> u64 {
        self.started_at.elapsed().as_millis().min(u64::MAX as u128) as u64
    }

    fn record(&self, phase: &str, details: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "trace_marker": "[REQUEST-TRACE]",
            "turn_id": self.turn_id,
            "model_call_round": self.round_index,
            "phase": phase,
            "elapsed_ms": self.elapsed_ms(),
            "details": details,
        })
    }

    fn emit(&self, phase: &str, details: serde_json::Value) {
        if !self.enabled {
            return;
        }
        let record = self.record(phase, details);
        let output = serde_json::to_string(&record)
            .unwrap_or_else(|error| format!("{{\"serialization_error\":\"{error}\"}}"));
        eprintln!("[REQUEST-TRACE] {output}");
    }

    fn finish(&mut self, phase: &str, details: serde_json::Value) {
        self.emit(phase, details);
        self.terminal_emitted = true;
    }
}

impl Drop for RequestLifecycleTrace {
    fn drop(&mut self) {
        if self.enabled && !self.terminal_emitted {
            self.emit(
                "request_future_dropped",
                serde_json::json!({
                    "outcome": "cancelled_or_aborted",
                }),
            );
        }
    }
}

#[derive(Default)]
struct StreamTraceCounters {
    network_chunk_count: usize,
    received_bytes: usize,
    sse_event_count: usize,
    content_chunk_count: usize,
    content_chars: usize,
    tool_call_event_count: usize,
    malformed_event_count: usize,
    unknown_event_count: usize,
}

impl StreamTraceCounters {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "network_chunk_count": self.network_chunk_count,
            "received_bytes": self.received_bytes,
            "sse_event_count": self.sse_event_count,
            "content_chunk_count": self.content_chunk_count,
            "content_chars": self.content_chars,
            "tool_call_event_count": self.tool_call_event_count,
            "malformed_event_count": self.malformed_event_count,
            "unknown_event_count": self.unknown_event_count,
        })
    }
}

pub async fn stream_round(
    turn_id: &str,
    round_index: usize,
    provider: &str,
    model: &str,
    messages: &serde_json::Value,
    tools: &serde_json::Value,
    channel: &tauri::ipc::Channel<AgTurnEvent>,
) -> Result<RoundOutcome, IpcError> {
    use futures_util::StreamExt;

    let url = format!("{}/invoke/stream", gw_base_url());
    let mut trace = RequestLifecycleTrace::new(turn_id, round_index);
    trace.emit(
        "request_started",
        serde_json::json!({
            "url": &url,
            "provider": provider,
            "model": model,
            "request": request_shape(messages, tools),
        }),
    );
    // 门禁前置（X-SEQ-001）：随调用上送会话令牌（dev auto-login）；GW 必裁决（PI-AG-GATE-01）。
    let token = ensure_session_token().await;
    trace.emit(
        "auth_resolved",
        serde_json::json!({
            "token_present": token.is_some(),
        }),
    );
    let body = serde_json::json!({
        "provider": provider, "model": model, "messages": messages, "tools": tools, "token": token
    });

    let resp = match reqwest::Client::new().post(&url).json(&body).send().await {
        Ok(r) => r,
        Err(e) => {
            trace.finish(
                "http_send_failed",
                serde_json::json!({
                    "error": e.to_string(),
                    "retriable": true,
                }),
            );
            if !trace.enabled {
                eprintln!("[ag-gw] ✗ unreachable {url}: {e}");
            }
            return Ok(RoundOutcome::Failed(IpcError::new(
                "AG_GW_UNREACHABLE",
                &format!("dev 网关不可达（{url}）：{e}。请先启动 cloud/dev-gateway。"),
                true,
                false,
            )));
        }
    };

    let status = resp.status();
    let response_content_type = resp
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();
    trace.emit(
        "http_response_received",
        serde_json::json!({
            "status": status.as_u16(),
            "content_type": &response_content_type,
        }),
    );

    if !status.is_success() {
        let code = status.as_u16();
        // 401 门禁拒绝/令牌失效 → 清令牌缓存，下轮自动重登录（X-FORBID-002：不缓存通过）。
        if code == 401 {
            clear_session_token();
        }
        let body = resp.text().await.unwrap_or_default();
        // 诊断日志：状态 + content-type + 响应体片段（截 300 字符）——定位是谁回的（我方网关回 JSON，反代常回空/HTML）。
        let snippet: String = body.chars().take(300).collect();
        trace.finish(
            "http_error_response",
            serde_json::json!({
                "status": code,
                "content_type": response_content_type,
                "response_body_chars": body.chars().count(),
                "response_body_preview": &snippet,
            }),
        );
        if !trace.enabled {
            eprintln!(
                "[ag-gw] ✗ HTTP {code} from {url} content-type=\"{response_content_type}\" body=\"{snippet}\""
            );
        }
        // 空体（多为中间反代/上游 502）→ 给可操作提示，避免 UI 只见一个空白错误码。
        let message = if body.trim().is_empty() {
            format!(
                "网关返回 HTTP {code}（空响应体）：{url}。本项目 dev-gateway 自身不产生 502，多为中间反向代理或上游未接通——请确认 BINDER_GW_URL 指向正确、且 dev-gateway 正在运行。"
            )
        } else {
            body
        };
        return Ok(RoundOutcome::Failed(IpcError::new(
            &format!("AG_GW_HTTP_{code}"),
            &message,
            code >= 500 || code == 401,
            code == 401,
        )));
    }

    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    let mut tool_calls: Vec<GwToolCall> = Vec::new();
    let mut final_stop_reason: Option<String> = None;
    let mut counters = StreamTraceCounters::default();

    while let Some(item) = stream.next().await {
        let bytes = match item {
            Ok(b) => b,
            Err(e) => {
                trace.finish(
                    "stream_transport_failed",
                    serde_json::json!({
                        "error": e.to_string(),
                        "counters": counters.to_json(),
                    }),
                );
                return Ok(RoundOutcome::Failed(IpcError::new(
                    "AG_GW_STREAM",
                    &format!("流式中断：{e}"),
                    true,
                    false,
                )));
            }
        };
        counters.network_chunk_count += 1;
        counters.received_bytes += bytes.len();
        if counters.network_chunk_count == 1 {
            trace.emit(
                "first_network_chunk",
                serde_json::json!({
                    "bytes": bytes.len(),
                }),
            );
        }
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
            counters.sse_event_count += 1;
            if counters.sse_event_count == 1 {
                trace.emit(
                    "first_sse_event",
                    serde_json::json!({
                        "event": &event,
                        "data_chars": data.chars().count(),
                    }),
                );
            }
            let json: serde_json::Value = match serde_json::from_str(&data) {
                Ok(json) => json,
                Err(error) => {
                    counters.malformed_event_count += 1;
                    trace.emit(
                        "malformed_sse_event",
                        serde_json::json!({
                            "event": &event,
                            "data_chars": data.chars().count(),
                            "error": error.to_string(),
                        }),
                    );
                    serde_json::Value::Null
                }
            };
            match event.as_str() {
                "chunk" => {
                    if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
                        counters.content_chunk_count += 1;
                        counters.content_chars += text.chars().count();
                        if counters.content_chunk_count == 1 {
                            trace.emit(
                                "first_content_chunk",
                                serde_json::json!({
                                    "text_chars": text.chars().count(),
                                }),
                            );
                        }
                        let _ = channel.send(AgTurnEvent::Chunk {
                            turn_id: turn_id.to_string(),
                            text: text.to_string(),
                        });
                    }
                }
                "tool_calls" => {
                    counters.tool_call_event_count += 1;
                    if let Some(arr) = json.get("tool_calls").and_then(|t| t.as_array()) {
                        let mut accepted_names = Vec::new();
                        for tc in arr {
                            if let Ok(call) = serde_json::from_value::<GwToolCall>(tc.clone()) {
                                if !call.name.is_empty() {
                                    accepted_names.push(call.name.clone());
                                    tool_calls.push(call);
                                }
                            }
                        }
                        trace.emit(
                            "tool_calls_received",
                            serde_json::json!({
                                "event_tool_call_count": arr.len(),
                                "accepted_tool_call_count": accepted_names.len(),
                                "accepted_tool_names": accepted_names,
                            }),
                        );
                    }
                }
                "done" => {
                    // 仅记录 provider stop reason；唯一 wire 终态由上层 Runtime 发射。
                    if tool_calls.is_empty() {
                        final_stop_reason = Some(
                            json.get("stop_reason")
                                .and_then(|s| s.as_str())
                                .unwrap_or("stop")
                                .to_string(),
                        );
                    }
                    trace.emit(
                        "provider_done",
                        serde_json::json!({
                            "stop_reason": json
                                .get("stop_reason")
                                .and_then(|reason| reason.as_str())
                                .unwrap_or("stop"),
                            "pending_tool_call_count": tool_calls.len(),
                        }),
                    );
                }
                "error" => {
                    let code = json
                        .get("code")
                        .and_then(|value| value.as_str())
                        .unwrap_or("AG_GW_ERR");
                    let message = json
                        .get("message")
                        .and_then(|value| value.as_str())
                        .unwrap_or("");
                    let retriable = json
                        .get("retriable")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false);
                    trace.finish(
                        "provider_error",
                        serde_json::json!({
                            "code": code,
                            "message": message,
                            "retriable": retriable,
                            "counters": counters.to_json(),
                        }),
                    );
                    return Ok(RoundOutcome::Failed(IpcError::new(
                        code, message, retriable, false,
                    )));
                }
                _ => {
                    counters.unknown_event_count += 1;
                    trace.emit(
                        "unknown_sse_event",
                        serde_json::json!({
                            "event": &event,
                            "data_chars": data.chars().count(),
                        }),
                    );
                }
            }
        }
    }

    if !buf.trim().is_empty() {
        trace.emit(
            "trailing_stream_buffer",
            serde_json::json!({
                "buffer_chars": buf.chars().count(),
            }),
        );
    }
    if !tool_calls.is_empty() {
        trace.finish(
            "request_completed",
            serde_json::json!({
                "outcome": "tool_calls",
                "tool_call_count": tool_calls.len(),
                "counters": counters.to_json(),
            }),
        );
        return Ok(RoundOutcome::ToolCalls(tool_calls));
    }
    let stop_reason = final_stop_reason.unwrap_or_else(|| "stop".to_string());
    trace.finish(
        "request_completed",
        serde_json::json!({
            "outcome": "done",
            "stop_reason": &stop_reason,
            "counters": counters.to_json(),
        }),
    );
    Ok(RoundOutcome::Done { stop_reason })
}

/// 单次辅助产出（SM-AG-03 Requesting · 无工具 · 对外非流式）：POST dev GW `/invoke/stream`，累积 chunk
/// 文本，流结束返回全文；GW 不可达 / HTTP 错误 / 流错误 / error 事件 → Err（调用方按 SM-AG-03 RequestFail
/// 静默处理，不打断编辑）。门禁前置同 stream_round（上送会话令牌，GW 裁决，X-SEQ-001 / X-FORBID-001）。
pub async fn assist_complete(
    provider: &str,
    model: &str,
    messages: &serde_json::Value,
) -> Result<String, IpcError> {
    use futures_util::StreamExt;

    let url = format!("{}/invoke/stream", gw_base_url());
    let token = ensure_session_token().await;
    let body = serde_json::json!({
        "provider": provider, "model": model, "messages": messages, "tools": [], "token": token
    });
    let resp = reqwest::Client::new()
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            IpcError::new(
                "AG_GW_UNREACHABLE",
                &format!("dev 网关不可达（{url}）：{e}"),
                true,
                false,
            )
        })?;
    if !resp.status().is_success() {
        let code = resp.status().as_u16();
        if code == 401 {
            clear_session_token();
        }
        let msg = resp.text().await.unwrap_or_default();
        return Err(IpcError::new(
            &format!("AG_GW_HTTP_{code}"),
            &msg,
            code >= 500 || code == 401,
            code == 401,
        ));
    }

    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    let mut out = String::new();
    while let Some(item) = stream.next().await {
        let bytes = item
            .map_err(|e| IpcError::new("AG_GW_STREAM", &format!("流式中断：{e}"), true, false))?;
        buf.push_str(&String::from_utf8_lossy(&bytes));
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
            let json: serde_json::Value =
                serde_json::from_str(&data).unwrap_or(serde_json::Value::Null);
            match event.as_str() {
                "chunk" => {
                    if let Some(t) = json.get("text").and_then(|t| t.as_str()) {
                        out.push_str(t);
                    }
                }
                "error" => {
                    return Err(IpcError::new(
                        json.get("code")
                            .and_then(|c| c.as_str())
                            .unwrap_or("AG_GW_ERR"),
                        json.get("message").and_then(|m| m.as_str()).unwrap_or(""),
                        json.get("retriable")
                            .and_then(|r| r.as_bool())
                            .unwrap_or(false),
                        false,
                    ));
                }
                _ => {} // done / tool_calls（辅助不用工具）→ 收尾
            }
        }
    }
    Ok(out)
}

/// dev GW `/models` 返回的预设模型条目（TERM-AG-010，桌面端仅消费标识符；不承载配置权威源）。
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PresetModel {
    pub provider: String,
    pub model: String,
    pub label: String,
    pub source: String,
}

/// 拉取预设模型清单（GET /models，TERM-AG-010）：只读清单，不鉴权（dev）；订阅档位映射归正式
/// 云 GW/LIC。供 AG-MODEL-CATALOG 注册表合并（BR-AG-DATA-006）。失败回统一错误，由调用方决定是否缓存。
pub async fn fetch_models() -> Result<Vec<PresetModel>, IpcError> {
    let url = format!("{}/models", gw_base_url());
    let resp = reqwest::Client::new().get(&url).send().await.map_err(|e| {
        IpcError::new(
            "AG_GW_UNREACHABLE",
            &format!("dev 网关不可达（{url}）：{e}。请先启动 cloud/dev-gateway。"),
            true,
            false,
        )
    })?;
    if !resp.status().is_success() {
        return Err(IpcError::new(
            "AG_GW_HTTP",
            &format!("预设清单拉取失败：网关 HTTP {}", resp.status()),
            true,
            false,
        ));
    }
    let body: serde_json::Value = resp.json().await.map_err(|e| {
        IpcError::new(
            "AG_GW_BAD_RESPONSE",
            &format!("预设清单响应无法解析：{e}"),
            true,
            false,
        )
    })?;
    let arr = body
        .get("models")
        .and_then(|m| m.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(arr
        .iter()
        .filter_map(|m| serde_json::from_value::<PresetModel>(m.clone()).ok())
        .collect())
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
        assert_eq!(
            transition(GateSubmitting, InvokeEvent::GateDenied),
            Some(InvokeState::GateDenied)
        );
        assert_eq!(
            transition(GateSubmitting, Offline),
            Some(InvokeState::GateDenied),
            "断网等同拒绝"
        );
        // 结构性门禁前置：除 GateSubmitting+gatePassed 外无路径达 Streaming
        for from in [
            Idle,
            Streaming,
            Completed,
            InvokeState::GateDenied,
            Interrupted,
            Errored,
        ] {
            assert_eq!(
                transition(from, GatePassed),
                None,
                "{from:?} 不应可达 Streaming"
            );
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
        assert_eq!(
            submit_for_verdict(Some("tok"), ServerVerdict::Passed).unwrap(),
            GateOutcome::Pass
        );
        assert_eq!(
            submit_for_verdict(Some("tok"), ServerVerdict::Denied).unwrap(),
            GateOutcome::Deny
        );
        // 断网等同拒绝（不静默重试、不本地放行）
        assert_eq!(
            submit_for_verdict(Some("tok"), ServerVerdict::Unreachable).unwrap(),
            GateOutcome::Deny
        );
        // 无令牌拒绝（须提交令牌经服务端裁决）
        let err = submit_for_verdict(None, ServerVerdict::Passed).unwrap_err();
        assert_eq!(err.code, "ENT_NO_TOKEN");
        assert!(submit_for_verdict(Some("  "), ServerVerdict::Passed).is_err());
    }

    // covers: BR-ENT-VERIFY-001
    // `/gate/check` JSON verdict 只把服务端 passed 映射为 Passed，其余均 Denied。
    #[test]
    fn gate_check_body_maps_only_passed_to_server_passed() {
        assert_eq!(
            verdict_from_gate_body(&GateCheckBody {
                verdict: "passed".into(),
                reason: None,
                reauth_required: None,
            }),
            ServerVerdict::Passed
        );
        assert_eq!(
            verdict_from_gate_body(&GateCheckBody {
                verdict: "denied".into(),
                reason: Some("quota".into()),
                reauth_required: None,
            }),
            ServerVerdict::Denied
        );
    }

    // covers: BR-SYS-OBS-001
    // 云调用三类失败统一错误形状：错误码 + 消息 + 可重试 + 需重认证 四字段齐备且一致可读。
    #[test]
    fn cloud_errors_share_unified_shape() {
        for kind in [
            CloudFailure::Auth,
            CloudFailure::Gate,
            CloudFailure::Gateway,
        ] {
            let e = cloud_error(kind, "CLOUD_X", "失败", true, true);
            assert_eq!(e.code, "CLOUD_X");
            assert!(!e.message.is_empty());
            assert!(e.retriable && e.reauth_required); // 四字段齐备
        }
    }

    // covers: BR-SYS-OBS-001
    // 请求过程日志只记录可关联阶段与体量，不重复打印 prompt、工具参数或令牌。
    #[test]
    fn request_trace_records_shape_without_repeating_payload() {
        let messages = serde_json::json!([
            {"role": "system", "content": "SECRET_SYSTEM_PROMPT"},
            {"role": "user", "content": "SECRET_USER_DOCUMENT"}
        ]);
        let tools = serde_json::json!([{
            "type": "function",
            "function": {
                "name": "read_doc",
                "description": "SECRET_TOOL_DESCRIPTION"
            }
        }]);
        let trace = RequestLifecycleTrace {
            enabled: false,
            turn_id: "turn-trace".to_string(),
            round_index: 2,
            started_at: std::time::Instant::now(),
            terminal_emitted: true,
        };
        let record = trace.record(
            "request_started",
            serde_json::json!({
                "request": request_shape(&messages, &tools),
                "token_present": true,
            }),
        );
        let serialized = serde_json::to_string(&record).unwrap();

        assert_eq!(record["trace_marker"], "[REQUEST-TRACE]");
        assert_eq!(record["turn_id"], "turn-trace");
        assert_eq!(record["model_call_round"], 2);
        assert_eq!(record["phase"], "request_started");
        assert_eq!(record["details"]["request"]["message_count"], 2);
        assert_eq!(record["details"]["request"]["tool_count"], 1);
        assert!(!serialized.contains("SECRET_SYSTEM_PROMPT"));
        assert!(!serialized.contains("SECRET_USER_DOCUMENT"));
        assert!(!serialized.contains("SECRET_TOOL_DESCRIPTION"));
        assert_eq!(serialized.lines().count(), 1);
    }
}
