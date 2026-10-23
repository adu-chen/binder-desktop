//! PI-ENT-01 · 门禁前置与网关调用通道 · CLOUD-INVOKE / CLOUD-GATE
//! 规则：BR-ENT-STATE-001（调用通道/流式三终态）/ BR-ENT-VERIFY-001（门禁前置无绕过）
//!
//! 承接 A-ENT-M-T-01：模型调用**仅在 GatePassed 后发起**，全部流量经 AiGateway——客户端不内置 provider
//! SDK、无直连路径（X-FORBID-001）；流式三终态（完成/中断/错误，可取消，承 BR-SYS-STATE-002）；失败统一
//! 错误形状（含 retriable/reauth_required）。ENT 是模型调用**唯一客户端门禁通道**：每次调用提交令牌经服务端
//! 裁决（X-SEQ-001），**客户端不缓存「通过」、不本地假通过**（X-FORBID-002）；GateDenied 终态不可绕过；
//! **断网失败关闭**（返回调用错误而非授权拒绝，不静默排队重试）。计量/BYOK 区分由服务端裁决。

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

/// 调用通道事件。只有 gatePassed/gateDenied 代表有效服务端裁决。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvokeEvent {
    Submit,
    GatePassed,
    GateDenied,
    GateFailed,
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
/// gateDenied→GateDenied，gateFailed/offline→Errored；流式三终态——streamDone→Completed、cancel→
/// Interrupted、streamError→Errored。返回 `None` = 无合法转移（含终态冻结、无门禁不得 Streaming）。
pub fn transition(state: InvokeState, event: InvokeEvent) -> Option<InvokeState> {
    use InvokeEvent as Ev;
    use InvokeState::*;
    let next = match (state, event) {
        (Idle, Ev::Submit) => GateSubmitting,
        // 门禁前置：唯一通往 Streaming 的路径。
        (GateSubmitting, Ev::GatePassed) => Streaming,
        (GateSubmitting, Ev::GateDenied) => GateDenied,
        (GateSubmitting, Ev::Offline | Ev::GateFailed) => Errored,
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
 * boundary: in=tokenAndServerVerdict | out=GateOutcomeOrIpcError(serverDecidedNeverLocalPass)
 */
/// 门禁裁决守卫（BR-ENT-VERIFY-001 / X-FORBID-002）：每次调用须提交令牌经**服务端**裁决——本函数
/// **无产生 Pass 的本地路径**（Pass 仅源于 ServerVerdict::Passed，不缓存复用、不本地假通过）；无令牌
/// 返回认证错误；Denied 返回 Deny，Unreachable 返回调用错误，均不静默重试或放行。
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
        ServerVerdict::Unreachable => Err(cloud_error(
            CloudFailure::Gate,
            "ENT_GATE_UNREACHABLE",
            "门禁通道不可达，请显式重试",
            true,
            false,
        )),
    }
}

// ── PI-AG-WIRE-01：dev AI 网关 HTTP 流式调用（接 cloud/dev-gateway） ──────────────────
//
// 客户端只连 localhost dev GW（X-FORBID-001：不内置 provider SDK、无直连 provider）。GW 归一化 SSE
// （event: chunk/done/error）→ 解析后回调 AG TurnEvent owner（承 BR-ENT-STATE-001 流式三终态）。
// dev 门禁/密钥沿用 GW（真实服务端裁决 X-FORBID-002、BYOK 加密 GW-02 归正式版）。

const DEFAULT_GW_URL: &str = "http://localhost:8787";
// A request deadline covers response headers and the entire streamed body. The values are
// provisional dev-gateway limits; provider-specific calibration remains a G5 system check.
const GATE_REQUEST_DEADLINE: std::time::Duration = std::time::Duration::from_secs(15);
const ASSIST_DEADLINE: std::time::Duration = std::time::Duration::from_secs(180);
// A long but active reasoning stream must not be confused with a stalled connection. The
// idle deadline is refreshed by content-free progress frames; the whole round remains bounded.
const MODEL_ROUND_DEADLINE: std::time::Duration = std::time::Duration::from_secs(300);
const MODEL_STREAM_IDLE_DEADLINE: std::time::Duration = std::time::Duration::from_secs(45);

#[cfg(any(test, debug_assertions))]
thread_local! {
    static TEST_GATEWAY_DEADLINE: std::cell::Cell<Option<std::time::Duration>> = const { std::cell::Cell::new(None) };
    static TEST_GATEWAY_IDLE_DEADLINE: std::cell::Cell<Option<std::time::Duration>> = const { std::cell::Cell::new(None) };
}

fn gateway_deadline(default: std::time::Duration) -> std::time::Duration {
    #[cfg(any(test, debug_assertions))]
    if let Some(deadline) = TEST_GATEWAY_DEADLINE.with(std::cell::Cell::get) {
        return deadline;
    }
    default
}

fn gateway_idle_deadline() -> std::time::Duration {
    #[cfg(any(test, debug_assertions))]
    if let Some(deadline) = TEST_GATEWAY_IDLE_DEADLINE.with(std::cell::Cell::get) {
        return deadline;
    }
    gateway_deadline(MODEL_STREAM_IDLE_DEADLINE)
}

#[cfg(test)]
pub(crate) async fn with_test_gateway_deadline<F: std::future::Future>(
    deadline: std::time::Duration,
    future: F,
) -> F::Output {
    struct Restore(Option<std::time::Duration>);
    impl Drop for Restore {
        fn drop(&mut self) {
            TEST_GATEWAY_DEADLINE.with(|slot| slot.set(self.0));
        }
    }
    futures_util::pin_mut!(future);
    futures_util::future::poll_fn(|cx| {
        let previous = TEST_GATEWAY_DEADLINE.with(|slot| slot.replace(Some(deadline)));
        let _restore = Restore(previous);
        future.as_mut().poll(cx)
    })
    .await
}

#[cfg(test)]
async fn with_test_gateway_idle_deadline<F: std::future::Future>(
    deadline: std::time::Duration,
    future: F,
) -> F::Output {
    struct Restore(Option<std::time::Duration>);
    impl Drop for Restore {
        fn drop(&mut self) {
            TEST_GATEWAY_IDLE_DEADLINE.with(|slot| slot.set(self.0));
        }
    }
    futures_util::pin_mut!(future);
    futures_util::future::poll_fn(|cx| {
        let previous = TEST_GATEWAY_IDLE_DEADLINE.with(|slot| slot.replace(Some(deadline)));
        let _restore = Restore(previous);
        future.as_mut().poll(cx)
    })
    .await
}

pub(crate) fn gw_base_url() -> String {
    #[cfg(any(test, debug_assertions))]
    if let Some(url) = scoped_test_gateway_url() {
        return url;
    }
    std::env::var("BINDER_GW_URL").unwrap_or_else(|_| DEFAULT_GW_URL.to_string())
}

// 只在测试 future 的单次 poll 内绑定 HTTP 目标，跨 await/并行测试不修改进程环境。
#[cfg(any(test, debug_assertions))]
thread_local! {
    static TEST_GATEWAY_URL: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

#[cfg(any(test, debug_assertions))]
fn scoped_test_gateway_url() -> Option<String> {
    TEST_GATEWAY_URL.with(|url| url.borrow().clone())
}

#[cfg(any(test, debug_assertions))]
pub(crate) async fn with_test_gateway<F: std::future::Future>(url: String, future: F) -> F::Output {
    struct Restore(Option<String>);
    impl Drop for Restore {
        fn drop(&mut self) {
            TEST_GATEWAY_URL.with(|slot| *slot.borrow_mut() = self.0.take());
        }
    }
    futures_util::pin_mut!(future);
    futures_util::future::poll_fn(|cx| {
        let previous = TEST_GATEWAY_URL.with(|slot| slot.replace(Some(url.clone())));
        let _restore = Restore(previous);
        future.as_mut().poll(cx)
    })
    .await
}

fn shared_session_cache_enabled() -> bool {
    #[cfg(any(test, debug_assertions))]
    if scoped_test_gateway_url().is_some() {
        return false;
    }
    true
}

// PI-AG-GATE-01 · 会话令牌缓存（auth 凭据，非门禁裁决缓存——每次调用仍经服务端 checkGate，X-FORBID-002）。
static SESSION_TOKEN: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

fn invocation_client_with_read_timeout(
    read_timeout: Option<std::time::Duration>,
) -> Result<reqwest::Client, IpcError> {
    // Task/model requests are never replayed implicitly by the transport. Credential renewal
    // is the separate, explicitly bounded check_gate branch below.
    #[allow(unused_mut)]
    let mut builder = reqwest::Client::builder().retry(reqwest::retry::never());
    if let Some(timeout) = read_timeout {
        builder = builder.read_timeout(timeout);
    }
    #[cfg(any(test, debug_assertions))]
    if scoped_test_gateway_url().is_some() {
        // An isolated local fixture must receive the bytes, not a system proxy's response.
        builder = builder.no_proxy();
    }
    builder.build().map_err(|_| {
        cloud_error(
            CloudFailure::Gateway,
            "ENT_CLIENT_SETUP",
            "调用通道初始化失败",
            false,
            false,
        )
    })
}

fn invocation_client() -> Result<reqwest::Client, IpcError> {
    invocation_client_with_read_timeout(None)
}

/// 取认证凭据而非授权结论；失败保留认证边界，不伪装成缺令牌或门禁拒绝。
async fn ensure_session_token() -> Result<String, IpcError> {
    if let Some(tok) = shared_session_cache_enabled()
        .then(|| SESSION_TOKEN.lock().ok().and_then(|g| g.as_ref().cloned()))
        .flatten()
    {
        return Ok(tok);
    }
    let url = format!("{}/auth/login", gw_base_url());
    let resp = match invocation_client()?
        .post(&url)
        .json(&serde_json::json!({ "email": "dev@binder.local" }))
        .timeout(gateway_deadline(GATE_REQUEST_DEADLINE))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return Err(request_failure(CloudFailure::Auth, &e)),
    };
    if !resp.status().is_success() {
        let status = resp.status();
        let denied = matches!(status.as_u16(), 401 | 403);
        return Err(cloud_error(
            CloudFailure::Auth,
            if denied {
                "ENT_AUTH_DENIED"
            } else {
                "ENT_AUTH_HTTP_ERROR"
            },
            "认证请求未成功，未发送模型调用",
            status.as_u16() == 429 || status.is_server_error(),
            denied,
        ));
    }
    let v: serde_json::Value = resp.json().await.map_err(|e| {
        if e.is_timeout() {
            request_failure(CloudFailure::Auth, &e)
        } else {
            cloud_error(
                CloudFailure::Auth,
                "ENT_AUTH_BAD_RESPONSE",
                "认证响应无法解析",
                true,
                false,
            )
        }
    })?;
    let token = v
        .get("token")
        .and_then(|t| t.as_str())
        .filter(|t| !t.trim().is_empty())
        .ok_or_else(|| {
            cloud_error(
                CloudFailure::Auth,
                "ENT_AUTH_BAD_RESPONSE",
                "认证响应缺少有效令牌",
                true,
                false,
            )
        })?
        .to_string();
    if shared_session_cache_enabled() {
        if let Ok(mut g) = SESSION_TOKEN.lock() {
            *g = Some(token.clone());
        }
    }
    Ok(token)
}

fn request_failure(kind: CloudFailure, error: &reqwest::Error) -> IpcError {
    let code = match (kind, error.is_timeout()) {
        (CloudFailure::Auth, true) => "ENT_AUTH_TIMEOUT",
        (CloudFailure::Auth, false) => "ENT_AUTH_UNREACHABLE",
        (_, true) => "ENT_GATE_TIMEOUT",
        (_, false) => "ENT_GATE_UNREACHABLE",
    };
    cloud_error(
        kind,
        code,
        "认证或门禁通道请求失败，未发送模型调用；请显式重试",
        true,
        false,
    )
}

/// 清会话令牌（门禁拒绝/令牌过期 → 下轮重登录）。
fn clear_session_token() {
    if !shared_session_cache_enabled() {
        return;
    }
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

fn verdict_from_gate_body(body: &GateCheckBody) -> Result<ServerVerdict, IpcError> {
    match body.verdict.as_str() {
        "passed" => Ok(ServerVerdict::Passed),
        "denied" => Ok(ServerVerdict::Denied),
        _ => Err(cloud_error(
            CloudFailure::Gate,
            "ENT_GATE_BAD_RESPONSE",
            "门禁响应含未知裁决",
            true,
            false,
        )),
    }
}

/// 单次模型调用前提交令牌到 dev GW `/gate/check` 获取服务端裁决。
/// 这是 auth token 复用，不是 gate pass 缓存；每次调用仍由服务端判定。
pub async fn check_gate() -> Result<GateOutcome, IpcError> {
    let url = format!("{}/gate/check", gw_base_url());
    let client = invocation_client()?;
    let mut retried_reauth = false;
    loop {
        let token = ensure_session_token().await?;
        let token_ref = Some(token.as_str());
        let resp = match client
            .post(&url)
            .json(&serde_json::json!({ "token": token_ref }))
            .timeout(gateway_deadline(GATE_REQUEST_DEADLINE))
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => return Err(request_failure(CloudFailure::Gate, &e)),
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
            let status = resp.status();
            if matches!(status.as_u16(), 401 | 403) {
                return submit_for_verdict(token_ref, ServerVerdict::Denied);
            }
            return Err(cloud_error(
                CloudFailure::Gate,
                "ENT_GATE_HTTP_ERROR",
                "门禁服务返回非授权类 HTTP 错误，未发送模型调用",
                status.as_u16() == 429 || status.is_server_error(),
                false,
            ));
        }
        let body = resp.json::<GateCheckBody>().await.map_err(|e| {
            if e.is_timeout() {
                return request_failure(CloudFailure::Gate, &e);
            }
            cloud_error(
                CloudFailure::Gate,
                "ENT_GATE_BAD_RESPONSE",
                "门禁响应无法解析",
                true,
                false,
            )
        })?;
        let verdict = verdict_from_gate_body(&body)?;
        if body.reauth_required.unwrap_or(false) && !retried_reauth {
            clear_session_token();
            retried_reauth = true;
            eprintln!("[ag-gw] gate/check 要求重新认证，已清令牌并重试一次");
            continue;
        }
        if body.reauth_required.unwrap_or(false) {
            clear_session_token();
            return submit_for_verdict(token_ref, ServerVerdict::Denied);
        }
        return submit_for_verdict(token_ref, verdict);
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

enum GatewaySseEvent {
    Chunk(String),
    Progress(&'static str),
    ToolCalls(Vec<GwToolCall>),
    Done(String),
    Error(IpcError),
}

impl GatewaySseEvent {
    fn kind(&self) -> &'static str {
        match self {
            Self::Chunk(_) => "chunk",
            Self::Progress(_) => "progress",
            Self::ToolCalls(_) => "tool_calls",
            Self::Done(_) => "done",
            Self::Error(_) => "error",
        }
    }
}

fn invalid_sse_event(message: &str) -> IpcError {
    IpcError::new("AG_GW_STREAM_INVALID_JSON", message, true, false)
}

fn incomplete_sse_event(message: &str) -> IpcError {
    IpcError::new("AG_GW_STREAM_INCOMPLETE", message, true, false)
}

fn parse_gateway_sse_frame(raw: &[u8]) -> Result<(GatewaySseEvent, usize), IpcError> {
    let raw = std::str::from_utf8(raw)
        .map_err(|_| invalid_sse_event("模型流事件包含非法 UTF-8，不能继续使用"))?;
    let normalized = raw.replace("\r\n", "\n");
    let (mut event, mut data) = (None, None);
    for line in normalized.lines() {
        if let Some(value) = line.strip_prefix("event:") {
            if event.replace(value.trim()).is_some() {
                return Err(invalid_sse_event("模型流事件重复声明 event 字段"));
            }
        } else if let Some(value) = line.strip_prefix("data:") {
            if data.replace(value.trim()).is_some() {
                return Err(invalid_sse_event("模型流事件重复声明 data 字段"));
            }
        } else {
            return Err(invalid_sse_event("模型流包含无法识别的 SSE 字段"));
        }
    }
    let event = event
        .filter(|value| !value.is_empty())
        .ok_or_else(|| invalid_sse_event("模型流事件缺少 event 字段"))?;
    let data = data
        .filter(|value| !value.is_empty())
        .ok_or_else(|| invalid_sse_event("模型流事件缺少 data 字段"))?;
    let data_chars = data.chars().count();
    let json: serde_json::Value = serde_json::from_str(data).map_err(|_| {
        invalid_sse_event("模型流事件包含非法 JSON，本次响应不可继续使用，请显式重试。")
    })?;
    let invalid_shape = || invalid_sse_event("模型流事件 JSON 结构非法，本次响应不可继续使用");
    let parsed = match event {
        "chunk" => GatewaySseEvent::Chunk(
            json.get("text")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(invalid_shape)?
                .to_string(),
        ),
        "progress" => {
            let Some(fields) = json.as_object() else {
                return Err(invalid_shape());
            };
            if fields.len() != 1 {
                return Err(invalid_shape());
            }
            let phase = match fields.get("phase").and_then(serde_json::Value::as_str) {
                Some("reasoning") => "reasoning",
                Some("upstream") => "upstream",
                _ => return Err(invalid_shape()),
            };
            GatewaySseEvent::Progress(phase)
        }
        "tool_calls" => {
            let items = json
                .get("tool_calls")
                .and_then(serde_json::Value::as_array)
                .filter(|items| !items.is_empty())
                .ok_or_else(invalid_shape)?;
            let mut calls = Vec::with_capacity(items.len());
            let mut ids = std::collections::HashSet::new();
            for item in items {
                let call: GwToolCall =
                    serde_json::from_value(item.clone()).map_err(|_| invalid_shape())?;
                if call.id.trim().is_empty()
                    || call.name.trim().is_empty()
                    || !ids.insert(call.id.clone())
                {
                    return Err(invalid_shape());
                }
                calls.push(call);
            }
            GatewaySseEvent::ToolCalls(calls)
        }
        "done" => {
            let reason = json
                .get("stop_reason")
                .and_then(serde_json::Value::as_str)
                .filter(|reason| !reason.is_empty())
                .ok_or_else(invalid_shape)?;
            if matches!(reason, "length" | "max_tokens" | "content_filter") {
                return Err(incomplete_sse_event("模型在完整答复前停止，不能标记为完成"));
            }
            if !matches!(reason, "stop" | "end_turn" | "stop_sequence") {
                return Err(invalid_shape());
            }
            GatewaySseEvent::Done(reason.to_string())
        }
        "error" => {
            let code = json
                .get("code")
                .and_then(serde_json::Value::as_str)
                .filter(|code| !code.is_empty())
                .ok_or_else(invalid_shape)?;
            let message = json
                .get("message")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(invalid_shape)?;
            let retriable = json
                .get("retriable")
                .and_then(serde_json::Value::as_bool)
                .ok_or_else(invalid_shape)?;
            GatewaySseEvent::Error(IpcError::new(code, message, retriable, false))
        }
        _ => return Err(invalid_sse_event("模型流包含未知 SSE 事件")),
    };
    Ok((parsed, data_chars))
}

#[derive(Default)]
struct GatewaySseDecoder {
    pending: Vec<u8>,
    completion_seen: bool,
}

impl GatewaySseDecoder {
    fn push(&mut self, bytes: &[u8]) {
        self.pending.extend_from_slice(bytes);
    }

    fn next_event(&mut self) -> Option<Result<(GatewaySseEvent, usize), IpcError>> {
        let lf = self
            .pending
            .windows(2)
            .position(|bytes| bytes == b"\n\n")
            .map(|at| (at, 2));
        let crlf = self
            .pending
            .windows(4)
            .position(|bytes| bytes == b"\r\n\r\n")
            .map(|at| (at, 4));
        let (at, delimiter_len) = match (lf, crlf) {
            (Some(left), Some(right)) => {
                if left.0 < right.0 {
                    left
                } else {
                    right
                }
            }
            (Some(found), None) | (None, Some(found)) => found,
            (None, None) => return None,
        };
        let raw: Vec<u8> = self.pending.drain(..at).collect();
        self.pending.drain(..delimiter_len);
        if self.completion_seen {
            return Some(Err(incomplete_sse_event(
                "模型流在完成事件之后仍包含事件，本次响应不可继续使用",
            )));
        }
        let parsed = parse_gateway_sse_frame(&raw);
        if let Ok((GatewaySseEvent::Done(_) | GatewaySseEvent::ToolCalls(_), _)) = &parsed {
            self.completion_seen = true;
        }
        Some(parsed)
    }

    fn finish(&self) -> Result<(), IpcError> {
        if !self.pending.is_empty() {
            return Err(incomplete_sse_event(
                "模型响应在完整事件结束前中断，请显式重试。",
            ));
        }
        if !self.completion_seen {
            return Err(incomplete_sse_event(
                "模型响应缺少完成事件，不能确认本次响应完整，请显式重试。",
            ));
        }
        Ok(())
    }
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
    progress_event_count: usize,
    tool_call_event_count: usize,
    malformed_event_count: usize,
    unknown_event_count: usize,
}

pub(crate) struct GatewayStreamFailure {
    message: String,
    timed_out: bool,
}

impl From<reqwest::Error> for GatewayStreamFailure {
    fn from(error: reqwest::Error) -> Self {
        Self {
            message: error.to_string(),
            timed_out: error.is_timeout(),
        }
    }
}

#[cfg(test)]
impl From<String> for GatewayStreamFailure {
    fn from(message: String) -> Self {
        Self {
            message,
            timed_out: false,
        }
    }
}

#[cfg(test)]
impl From<&str> for GatewayStreamFailure {
    fn from(message: &str) -> Self {
        Self::from(message.to_string())
    }
}

fn gateway_stream_error(error: GatewayStreamFailure) -> IpcError {
    if error.timed_out {
        IpcError::new(
            "AG_GW_STREAM_TIMEOUT",
            "模型流在本轮期限内未完整结束；此前操作结果保留，请核对实际效果后显式重试。",
            true,
            false,
        )
    } else {
        IpcError::new(
            "AG_GW_STREAM",
            &format!("流式中断：{}", error.message),
            true,
            false,
        )
    }
}

impl StreamTraceCounters {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "network_chunk_count": self.network_chunk_count,
            "received_bytes": self.received_bytes,
            "sse_event_count": self.sse_event_count,
            "content_chunk_count": self.content_chunk_count,
            "content_chars": self.content_chars,
            "progress_event_count": self.progress_event_count,
            "tool_call_event_count": self.tool_call_event_count,
            "malformed_event_count": self.malformed_event_count,
            "unknown_event_count": self.unknown_event_count,
        })
    }
}

pub async fn stream_round<F>(
    turn_id: &str,
    round_index: usize,
    provider: &str,
    model: &str,
    messages: &serde_json::Value,
    tools: &serde_json::Value,
    on_chunk: F,
) -> Result<RoundOutcome, IpcError>
where
    F: FnMut(String) -> Result<(), IpcError>,
{
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
    let token = ensure_session_token().await?;
    trace.emit(
        "auth_resolved",
        serde_json::json!({
            "token_present": true,
        }),
    );
    let body = serde_json::json!({
        "provider": provider, "model": model, "messages": messages, "tools": tools, "token": token
    });

    let resp = match invocation_client_with_read_timeout(Some(gateway_idle_deadline()))?
        .post(&url)
        .header("X-Binder-Turn-Id", turn_id)
        .header("X-Binder-Round-Index", round_index.to_string())
        .json(&body)
        .timeout(gateway_deadline(MODEL_ROUND_DEADLINE))
        .send()
        .await
    {
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
            return Ok(RoundOutcome::Failed(if e.is_timeout() {
                IpcError::new(
                    "AG_GW_TIMEOUT",
                    "模型网关在本轮期限内未返回响应；本次调用已停止，请核对实际效果后显式重试。",
                    true,
                    false,
                )
            } else {
                IpcError::new(
                    "AG_GW_UNREACHABLE",
                    &format!("dev 网关不可达（{url}）：{e}。请先启动 cloud/dev-gateway。"),
                    true,
                    false,
                )
            }));
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

    consume_round_stream(resp.bytes_stream(), &mut trace, on_chunk).await
}

/**
 * @GOV
 * codes: BR-ENT-STATE-001-RB-ENT-CLOUD-INVOKE-002
 * type: RB
 * chain: CLOUD-INVOKE
 * rules: BR-ENT-STATE-001
 * boundary: in=GatewayByteStreamAndChunkCallback | out=RoundOutcomeOrCallbackError | does_not=grantCapabilityOrEmitAgentTurnTerminal
 */
// HTTP 与测试字节流共享此解析路径；本函数不产生授权或 AgentTurn 终态。
async fn consume_round_stream<S, B, E, F>(
    stream: S,
    trace: &mut RequestLifecycleTrace,
    mut on_chunk: F,
) -> Result<RoundOutcome, IpcError>
where
    S: futures_util::Stream<Item = Result<B, E>>,
    B: AsRef<[u8]>,
    E: Into<GatewayStreamFailure>,
    F: FnMut(String) -> Result<(), IpcError>,
{
    use futures_util::StreamExt;
    futures_util::pin_mut!(stream);
    let mut decoder = GatewaySseDecoder::default();
    let mut tool_calls: Vec<GwToolCall> = Vec::new();
    let mut final_stop_reason: Option<String> = None;
    let mut counters = StreamTraceCounters::default();

    while let Some(item) = stream.next().await {
        let bytes = match item {
            Ok(b) => b,
            Err(e) => {
                let error = gateway_stream_error(e.into());
                trace.finish(
                    "stream_transport_failed",
                    serde_json::json!({
                        "code": &error.code,
                        "counters": counters.to_json(),
                    }),
                );
                return Ok(RoundOutcome::Failed(error));
            }
        };
        let bytes = bytes.as_ref();
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
        decoder.push(bytes);
        while let Some(parsed) = decoder.next_event() {
            let (event, data_chars) = match parsed {
                Ok(parsed) => parsed,
                Err(error) => {
                    counters.malformed_event_count += 1;
                    trace.emit(
                        "malformed_sse_event",
                        serde_json::json!({ "code": &error.code }),
                    );
                    trace.finish("invalid_sse_event", counters.to_json());
                    return Ok(RoundOutcome::Failed(error));
                }
            };
            counters.sse_event_count += 1;
            if counters.sse_event_count == 1 {
                trace.emit(
                    "first_sse_event",
                    serde_json::json!({
                        "event": event.kind(),
                        "data_chars": data_chars,
                    }),
                );
            }
            match event {
                GatewaySseEvent::Chunk(text) => {
                    counters.content_chunk_count += 1;
                    counters.content_chars += text.chars().count();
                    if counters.content_chunk_count == 1 {
                        trace.emit(
                            "first_content_chunk",
                            serde_json::json!({ "text_chars": text.chars().count() }),
                        );
                    }
                    on_chunk(text)?;
                }
                GatewaySseEvent::Progress(phase) => {
                    counters.progress_event_count += 1;
                    if counters.progress_event_count == 1 {
                        trace.emit(
                            "first_model_progress",
                            serde_json::json!({ "phase": phase }),
                        );
                    }
                }
                GatewaySseEvent::ToolCalls(calls) => {
                    counters.tool_call_event_count += 1;
                    trace.emit(
                        "tool_calls_received",
                        serde_json::json!({
                            "event_tool_call_count": calls.len(),
                            "accepted_tool_call_count": calls.len(),
                            "accepted_tool_names": calls.iter().map(|call| call.name.as_str()).collect::<Vec<_>>(),
                        }),
                    );
                    tool_calls = calls;
                }
                GatewaySseEvent::Done(reason) => {
                    // 仅记录 provider stop reason；唯一 wire 终态由上层 Runtime 发射。
                    final_stop_reason = Some(reason.clone());
                    trace.emit(
                        "provider_done",
                        serde_json::json!({
                            "stop_reason": reason,
                            "pending_tool_call_count": 0,
                        }),
                    );
                }
                GatewaySseEvent::Error(error) => {
                    trace.finish(
                        "provider_error",
                        serde_json::json!({
                            "code": &error.code,
                            "retriable": error.retriable,
                            "counters": counters.to_json(),
                        }),
                    );
                    return Ok(RoundOutcome::Failed(error));
                }
            }
        }
    }

    if let Err(error) = decoder.finish() {
        trace.finish("stream_completion_invalid", counters.to_json());
        return Ok(RoundOutcome::Failed(error));
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
    let Some(stop_reason) = final_stop_reason else {
        return Ok(RoundOutcome::Failed(incomplete_sse_event(
            "模型响应缺少完成事件",
        )));
    };
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

/// 测试仅替换字节来源，不替换生产 SSE 解析或调用方回调；无发布构建入口。
#[cfg(test)]
pub(crate) async fn fixed_round_stream<S, B, E, F>(
    stream: S,
    on_chunk: F,
) -> Result<RoundOutcome, IpcError>
where
    S: futures_util::Stream<Item = Result<B, E>>,
    B: AsRef<[u8]>,
    E: Into<GatewayStreamFailure>,
    F: FnMut(String) -> Result<(), IpcError>,
{
    let mut trace = RequestLifecycleTrace {
        enabled: false,
        turn_id: "fixed-stream-test".into(),
        round_index: 1,
        started_at: std::time::Instant::now(),
        terminal_emitted: false,
    };
    consume_round_stream(stream, &mut trace, on_chunk).await
}

async fn consume_assist_stream<S, B, E>(stream: S) -> Result<String, IpcError>
where
    S: futures_util::Stream<Item = Result<B, E>>,
    B: AsRef<[u8]>,
    E: Into<GatewayStreamFailure>,
{
    use futures_util::StreamExt;
    futures_util::pin_mut!(stream);
    let mut decoder = GatewaySseDecoder::default();
    let mut out = String::new();
    while let Some(item) = stream.next().await {
        let bytes = item.map_err(|error| gateway_stream_error(error.into()))?;
        decoder.push(bytes.as_ref());
        while let Some(parsed) = decoder.next_event() {
            match parsed?.0 {
                GatewaySseEvent::Chunk(text) => out.push_str(&text),
                GatewaySseEvent::Progress(_) => {}
                GatewaySseEvent::Done(_) => {}
                GatewaySseEvent::ToolCalls(_) => {
                    return Err(invalid_sse_event("辅助产出不接受工具请求"));
                }
                GatewaySseEvent::Error(error) => return Err(error),
            }
        }
    }
    decoder.finish()?;
    Ok(out)
}

#[cfg(test)]
async fn fixed_assist_stream<S, B, E>(stream: S) -> Result<String, IpcError>
where
    S: futures_util::Stream<Item = Result<B, E>>,
    B: AsRef<[u8]>,
    E: Into<GatewayStreamFailure>,
{
    consume_assist_stream(stream).await
}

/// 单次辅助产出（SM-AG-03 Requesting · 无工具 · 对外非流式）：POST dev GW `/invoke/stream`，累积 chunk
/// 文本，流结束返回全文；GW 不可达 / HTTP 错误 / 流错误 / error 事件 → Err（调用方按 SM-AG-03 RequestFail
/// 静默处理，不打断编辑）。门禁前置同 stream_round（上送会话令牌，GW 裁决，X-SEQ-001 / X-FORBID-001）。
pub async fn assist_complete(
    provider: &str,
    model: &str,
    messages: &serde_json::Value,
) -> Result<String, IpcError> {
    let url = format!("{}/invoke/stream", gw_base_url());
    let token = ensure_session_token().await?;
    let body = serde_json::json!({
        "provider": provider, "model": model, "messages": messages, "tools": [], "token": token
    });
    let resp = invocation_client()?
        .post(&url)
        .json(&body)
        .timeout(gateway_deadline(ASSIST_DEADLINE))
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

    consume_assist_stream(resp.bytes_stream()).await
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

    fn stalled_round(send_headers: bool) -> RoundOutcome {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let worker = std::thread::spawn(move || {
            let (mut auth, _) = listener.accept().unwrap();
            let mut request = [0_u8; 4096];
            auth.read(&mut request).unwrap();
            let token = r#"{"token":"fixture-token"}"#;
            write!(
                auth,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{token}",
                token.len()
            )
            .unwrap();
            drop(auth);

            let (mut invoke, _) = listener.accept().unwrap();
            let mut received = Vec::new();
            while !received.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                let bytes_read = invoke.read(&mut request).unwrap();
                assert!(
                    bytes_read > 0,
                    "gateway fixture request headers ended early"
                );
                received.extend_from_slice(&request[..bytes_read]);
            }
            let request_headers = String::from_utf8_lossy(&received).to_ascii_lowercase();
            assert!(request_headers.contains("x-binder-turn-id: deadline-fixture-turn"));
            assert!(request_headers.contains("x-binder-round-index: 1"));
            if send_headers {
                write!(invoke, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n").unwrap();
            }
            std::thread::sleep(std::time::Duration::from_millis(180));
        });
        let outcome = tauri::async_runtime::block_on(with_test_gateway(
            url,
            with_test_gateway_deadline(std::time::Duration::from_millis(60), async {
                stream_round(
                    "deadline-fixture-turn",
                    1,
                    "fixed",
                    "fixed",
                    &serde_json::json!([]),
                    &serde_json::json!([]),
                    |_| Ok(()),
                )
                .await
                .unwrap()
            }),
        ));
        worker.join().unwrap();
        outcome
    }

    // covers: BR-ENT-STATE-001
    #[test]
    fn model_round_deadline_covers_headers_and_incomplete_stream() {
        assert!(matches!(stalled_round(false), RoundOutcome::Failed(error)
            if error.code == "AG_GW_TIMEOUT" && error.retriable));
        assert!(matches!(stalled_round(true), RoundOutcome::Failed(error)
            if error.code == "AG_GW_STREAM_TIMEOUT" && error.retriable));
    }

    fn streamed_progress_round(stall_after_progress: bool) -> (RoundOutcome, String) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let worker = std::thread::spawn(move || {
            let (mut auth, _) = listener.accept().unwrap();
            let mut request = [0_u8; 4096];
            auth.read(&mut request).unwrap();
            let token = r#"{"token":"fixture-token"}"#;
            write!(
                auth,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{token}",
                token.len()
            )
            .unwrap();
            drop(auth);

            let (mut invoke, _) = listener.accept().unwrap();
            invoke.read(&mut request).unwrap();
            write!(
                invoke,
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
            for _ in 0..3 {
                write!(
                    invoke,
                    "event: progress\ndata: {{\"phase\":\"reasoning\"}}\n\n"
                )
                .unwrap();
                invoke.flush().unwrap();
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            if stall_after_progress {
                std::thread::sleep(std::time::Duration::from_millis(100));
            } else {
                write!(invoke, "event: chunk\ndata: {{\"text\":\"answer\"}}\n\nevent: done\ndata: {{\"stop_reason\":\"stop\"}}\n\n").unwrap();
            }
        });
        let mut visible = String::new();
        let outcome = tauri::async_runtime::block_on(with_test_gateway(
            url,
            with_test_gateway_deadline(
                std::time::Duration::from_millis(250),
                with_test_gateway_idle_deadline(std::time::Duration::from_millis(45), async {
                    stream_round(
                        "progress-fixture-turn",
                        1,
                        "fixed",
                        "fixed",
                        &serde_json::json!([]),
                        &serde_json::json!([]),
                        |text| {
                            visible.push_str(&text);
                            Ok(())
                        },
                    )
                    .await
                    .unwrap()
                }),
            ),
        ));
        worker.join().unwrap();
        (outcome, visible)
    }

    // covers: BR-ENT-STATE-001, BR-SYS-OBS-001
    #[test]
    fn active_reasoning_progress_resets_idle_deadline_but_does_not_complete_round() {
        let (complete, visible) = streamed_progress_round(false);
        assert!(matches!(complete, RoundOutcome::Done { .. }));
        assert_eq!(visible, "answer");
        let (stalled, visible) = streamed_progress_round(true);
        assert!(matches!(stalled, RoundOutcome::Failed(error)
            if error.code == "AG_GW_STREAM_TIMEOUT"));
        assert!(visible.is_empty());
    }

    #[test]
    fn fixed_gateway_scope_does_not_leak_between_future_polls() {
        tauri::async_runtime::block_on(async {
            let original = gw_base_url();
            let future = with_test_gateway("http://127.0.0.1:1".into(), async {
                assert_eq!(gw_base_url(), "http://127.0.0.1:1");
                assert!(!shared_session_cache_enabled());
                with_test_gateway("http://127.0.0.1:2".into(), async {
                    assert_eq!(gw_base_url(), "http://127.0.0.1:2");
                })
                .await;
                assert_eq!(gw_base_url(), "http://127.0.0.1:1");
                futures_util::future::pending::<()>().await;
            });
            futures_util::pin_mut!(future);
            assert!(futures_util::poll!(&mut future).is_pending());
            assert_eq!(gw_base_url(), original);
            assert!(shared_session_cache_enabled());
        });
        assert!(scoped_test_gateway_url().is_none());
    }

    fn fixed_bytes(bytes: &[u8]) -> impl futures_util::Stream<Item = Result<Vec<u8>, String>> {
        // 在每个 ASCII 字节边界分片，覆盖 SSE 字段名、JSON 和空行分隔符跨包。
        futures_util::stream::iter(bytes.iter().map(|byte| Ok(vec![*byte])).collect::<Vec<_>>())
    }

    // covers: BR-ENT-STATE-001
    #[test]
    fn fixed_stream_preserves_text_and_tool_completion_contracts() {
        tauri::async_runtime::block_on(async {
            let mut visible = String::new();
            let outcome = fixed_round_stream(
            fixed_bytes(b"event: chunk\ndata: {\"text\":\"hello\"}\n\nevent: done\ndata: {\"stop_reason\":\"stop\"}\n\n"),
            |text| { visible.push_str(&text); Ok(()) },
        ).await.unwrap();
            assert_eq!(visible, "hello");
            assert!(matches!(outcome, RoundOutcome::Done { stop_reason } if stop_reason == "stop"));

            // 真实网关在工具完成时仅发 tool_calls，不要求附加 done。
            let outcome = fixed_round_stream(
            fixed_bytes(b"event: tool_calls\ndata: {\"tool_calls\":[{\"id\":\"call-1\",\"name\":\"workspace_glob\",\"arguments\":\"{}\"}]}\n\n"),
            |_| panic!("tool-only response must not emit visible text"),
        ).await.unwrap();
            match outcome {
                RoundOutcome::ToolCalls(calls) => {
                    assert_eq!(calls.len(), 1);
                    assert_eq!(calls[0].id, "call-1");
                    assert_eq!(calls[0].name, "workspace_glob");
                    assert_eq!(calls[0].arguments, "{}");
                }
                _ => panic!("expected native tool response"),
            }
        });
    }

    // covers: BR-ENT-STATE-001
    // covers: BR-SYS-OBS-001
    #[test]
    fn fixed_stream_treats_reasoning_progress_as_non_terminal_and_non_visible() {
        tauri::async_runtime::block_on(async {
            let mut visible = String::new();
            let complete = b"event: progress\ndata: {\"phase\":\"reasoning\"}\n\nevent: progress\ndata: {\"phase\":\"upstream\"}\n\nevent: chunk\ndata: {\"text\":\"answer\"}\n\nevent: done\ndata: {\"stop_reason\":\"stop\"}\n\n";
            let outcome = fixed_round_stream(fixed_bytes(complete), |text| {
                visible.push_str(&text);
                Ok(())
            })
            .await
            .unwrap();
            assert!(matches!(outcome, RoundOutcome::Done { .. }));
            assert_eq!(visible, "answer");

            let progress_only = b"event: progress\ndata: {\"phase\":\"reasoning\"}\n\n";
            let outcome = fixed_round_stream(fixed_bytes(progress_only), |_| {
                panic!("progress must not become visible content")
            })
            .await
            .unwrap();
            assert!(matches!(outcome, RoundOutcome::Failed(error)
                if error.code == "AG_GW_STREAM_INCOMPLETE"));

            let invalid =
                b"event: progress\ndata: {\"phase\":\"reasoning\",\"text\":\"hidden\"}\n\n";
            let outcome = fixed_round_stream(fixed_bytes(invalid), |_| Ok(()))
                .await
                .unwrap();
            assert!(matches!(outcome, RoundOutcome::Failed(error)
                if error.code == "AG_GW_STREAM_INVALID_JSON"));

            let unknown_phase = b"event: progress\ndata: {\"phase\":\"unverified\"}\n\n";
            let outcome = fixed_round_stream(fixed_bytes(unknown_phase), |_| Ok(()))
                .await
                .unwrap();
            assert!(matches!(outcome, RoundOutcome::Failed(error)
                if error.code == "AG_GW_STREAM_INVALID_JSON"));

            let assist = b"event: progress\ndata: {\"phase\":\"reasoning\"}\n\nevent: chunk\ndata: {\"text\":\"answer\"}\n\nevent: done\ndata: {\"stop_reason\":\"stop\"}\n\n";
            assert_eq!(
                fixed_assist_stream(fixed_bytes(assist)).await.unwrap(),
                "answer"
            );
        });
    }

    // covers: BR-ENT-STATE-001
    #[test]
    fn fixed_stream_eof_never_manufactures_completion() {
        tauri::async_runtime::block_on(async {
            for source in [
            "",
            "event: chunk\ndata: {\"text\":\"partial\"}\n\n",
            "event: done\ndata: {\"stop_reason\":\"stop\"}",
            "event: done\ndata: {\"stop_reason\":\"stop\"}\n\nevent: chunk\ndata:",
            "event: tool_calls\ndata: {\"tool_calls\":[{\"id\":\"c\",\"name\":\"workspace_glob\",\"arguments\":\"{}\"}]}\n\nevent:",
        ] {
            let outcome = fixed_round_stream(fixed_bytes(source.as_bytes()), |_| Ok(())).await.unwrap();
            match outcome {
                RoundOutcome::Failed(error) => {
                    assert_eq!(error.code, "AG_GW_STREAM_INCOMPLETE", "{source}");
                    assert!(error.retriable);
                    assert!(!error.reauth_required);
                }
                _ => panic!("EOF must not become a completed response: {source}"),
            }
        }
        });
    }

    // covers: BR-ENT-STATE-001
    #[test]
    fn malformed_sse_json_fails_before_a_later_completion_can_mask_it() {
        tauri::async_runtime::block_on(async {
            let source = b"event: tool_calls\ndata: {invalid\n\nevent: done\ndata: {\"stop_reason\":\"stop\"}\n\n";
            let outcome = fixed_round_stream(fixed_bytes(source), |_| Ok(()))
                .await
                .unwrap();
            assert!(matches!(outcome, RoundOutcome::Failed(error)
                if error.code == "AG_GW_STREAM_INVALID_JSON" && error.retriable));
        });
    }

    // covers: BR-ENT-STATE-001
    #[test]
    fn fixed_stream_preserves_multibyte_text_split_at_every_network_byte() {
        tauri::async_runtime::block_on(async {
            let source = "event: chunk\ndata: {\"text\":\"中文🙂\"}\n\nevent: done\ndata: {\"stop_reason\":\"stop\"}\n\n";
            let mut visible = String::new();
            let outcome = fixed_round_stream(fixed_bytes(source.as_bytes()), |text| {
                visible.push_str(&text);
                Ok(())
            })
            .await
            .unwrap();
            assert!(matches!(outcome, RoundOutcome::Done { .. }));
            assert_eq!(visible, "中文🙂");
            let crlf = b"event: chunk\r\ndata: {\"text\":\"ok\"}\r\n\r\nevent: done\r\ndata: {\"stop_reason\":\"end_turn\"}\r\n\r\n";
            let mut visible = String::new();
            let outcome = fixed_round_stream(fixed_bytes(crlf), |text| {
                visible.push_str(&text);
                Ok(())
            })
            .await
            .unwrap();
            assert_eq!(visible, "ok");
            assert!(
                matches!(outcome, RoundOutcome::Done { stop_reason } if stop_reason == "end_turn")
            );
        });
    }

    // covers: BR-ENT-STATE-001
    #[test]
    fn fixed_stream_rejects_bad_event_shapes_and_conflicting_completion() {
        tauri::async_runtime::block_on(async {
            for (source, expected_code) in [
                ("event: chunk\ndata: {\"text\":42}\n\nevent: done\ndata: {\"stop_reason\":\"stop\"}\n\n", "AG_GW_STREAM_INVALID_JSON"),
                ("event: done\ndata: {}\n\n", "AG_GW_STREAM_INVALID_JSON"),
                ("event: unknown\ndata: {}\n\nevent: done\ndata: {\"stop_reason\":\"stop\"}\n\n", "AG_GW_STREAM_INVALID_JSON"),
                ("event: tool_calls\ndata: {\"tool_calls\":[]}\n\n", "AG_GW_STREAM_INVALID_JSON"),
                ("event: tool_calls\ndata: {\"tool_calls\":[{\"id\":\"valid\",\"name\":\"workspace_glob\",\"arguments\":\"{}\"},{}]}\n\n", "AG_GW_STREAM_INVALID_JSON"),
                ("event: done\ndata: {\"stop_reason\":\"stop\"}\n\nevent: done\ndata: {\"stop_reason\":\"stop\"}\n\n", "AG_GW_STREAM_INCOMPLETE"),
                ("event: tool_calls\ndata: {\"tool_calls\":[{\"id\":\"valid\",\"name\":\"workspace_glob\",\"arguments\":\"{}\"}]}\n\nevent: done\ndata: {\"stop_reason\":\"stop\"}\n\n", "AG_GW_STREAM_INCOMPLETE"),
                ("event: done\ndata: {\"stop_reason\":\"stop\"}\n\nevent: chunk\ndata: {\"text\":\"late\"}\n\n", "AG_GW_STREAM_INCOMPLETE"),
                ("event: done\ndata: {\"stop_reason\":\"length\"}\n\n", "AG_GW_STREAM_INCOMPLETE"),
                ("event: tool_calls\ndata: {\"tool_calls\":[{\"id\":\"same\",\"name\":\"workspace_glob\",\"arguments\":\"{}\"},{\"id\":\"same\",\"name\":\"read_doc\",\"arguments\":\"{}\"}]}\n\n", "AG_GW_STREAM_INVALID_JSON"),
            ] {
                let outcome = fixed_round_stream(fixed_bytes(source.as_bytes()), |_| Ok(()))
                    .await
                    .unwrap();
                assert!(matches!(outcome, RoundOutcome::Failed(error) if error.code == expected_code), "{source}");
            }
            let invalid_utf8 = b"event: chunk\ndata: {\"text\":\"\xff\"}\n\nevent: done\ndata: {\"stop_reason\":\"stop\"}\n\n";
            let outcome = fixed_round_stream(fixed_bytes(invalid_utf8), |_| Ok(()))
                .await
                .unwrap();
            assert!(
                matches!(outcome, RoundOutcome::Failed(error) if error.code == "AG_GW_STREAM_INVALID_JSON")
            );
        });
    }

    // covers: BR-ENT-STATE-001
    // covers: BR-AG-STATE-003
    #[test]
    fn fixed_assist_stream_requires_valid_utf8_and_one_complete_text_response() {
        tauri::async_runtime::block_on(async {
            let complete = "event: chunk\ndata: {\"text\":\"你好🙂\"}\n\nevent: done\ndata: {\"stop_reason\":\"stop\"}\n\n";
            assert_eq!(
                fixed_assist_stream(fixed_bytes(complete.as_bytes()))
                    .await
                    .unwrap(),
                "你好🙂"
            );
            for (source, expected_code) in [
                ("event: chunk\ndata: {\"text\":\"partial\"}\n\n", "AG_GW_STREAM_INCOMPLETE"),
                ("event: chunk\ndata: {bad}\n\nevent: done\ndata: {\"stop_reason\":\"stop\"}\n\n", "AG_GW_STREAM_INVALID_JSON"),
                ("event: chunk\ndata: {\"text\":\"partial\"}\n\nevent: done\ndata: {\"stop_reason\":\"stop\"}", "AG_GW_STREAM_INCOMPLETE"),
                ("event: tool_calls\ndata: {\"tool_calls\":[{\"id\":\"c\",\"name\":\"workspace_glob\",\"arguments\":\"{}\"}]}\n\n", "AG_GW_STREAM_INVALID_JSON"),
            ] {
                let error = fixed_assist_stream(fixed_bytes(source.as_bytes())).await.unwrap_err();
                assert_eq!(error.code, expected_code, "{source}");
            }
            let invalid_utf8 = b"event: chunk\ndata: {\"text\":\"\xff\"}\n\nevent: done\ndata: {\"stop_reason\":\"stop\"}\n\n";
            let error = fixed_assist_stream(fixed_bytes(invalid_utf8))
                .await
                .unwrap_err();
            assert_eq!(error.code, "AG_GW_STREAM_INVALID_JSON");
            let provider_error = b"event: chunk\ndata: {\"text\":\"partial\"}\n\nevent: error\ndata: {\"code\":\"PROVIDER_BUSY\",\"message\":\"busy\",\"retriable\":true}\n\n";
            let error = fixed_assist_stream(fixed_bytes(provider_error))
                .await
                .unwrap_err();
            assert_eq!(error.code, "PROVIDER_BUSY");
        });
    }

    // covers: BR-ENT-STATE-001
    // covers: BR-SYS-OBS-001
    #[test]
    fn fixed_stream_preserves_transport_provider_and_callback_failures() {
        tauri::async_runtime::block_on(async {
            let stream = futures_util::stream::iter(vec![Err::<Vec<u8>, _>("connection lost")]);
            let outcome = fixed_round_stream(stream, |_| Ok(())).await.unwrap();
            assert!(matches!(outcome, RoundOutcome::Failed(error) if error.code == "AG_GW_STREAM"));
            let outcome = fixed_round_stream(
            fixed_bytes(b"event: error\ndata: {\"code\":\"PROVIDER_BUSY\",\"message\":\"busy\",\"retriable\":true}\n\n"),
            |_| Ok(()),
        ).await.unwrap();
            assert!(
                matches!(outcome, RoundOutcome::Failed(error) if error.code == "PROVIDER_BUSY" && error.retriable)
            );
            let error = fixed_round_stream(
                fixed_bytes(b"event: chunk\ndata: {\"text\":\"bad\"}\n\nevent: done\ndata: {}\n\n"),
                |_| {
                    Err(IpcError::new(
                        "AG_MODEL_CONTROL_MISSING",
                        "missing control",
                        true,
                        false,
                    ))
                },
            )
            .await
            .err()
            .expect("callback failure must propagate");
            assert_eq!(error.code, "AG_MODEL_CONTROL_MISSING");
        });
    }

    // covers: BR-ENT-STATE-001
    #[test]
    fn fixed_stream_can_be_cancelled_while_waiting_for_bytes() {
        tauri::async_runtime::block_on(async {
            use futures_util::future::{AbortHandle, Abortable};
            let (handle, registration) = AbortHandle::new_pair();
            let stream = futures_util::stream::pending::<Result<Vec<u8>, String>>();
            let future = Abortable::new(fixed_round_stream(stream, |_| Ok(())), registration);
            futures_util::pin_mut!(future);
            assert!(futures_util::poll!(&mut future).is_pending());
            handle.abort();
            assert!(future.await.is_err());
            // 此断言仅证明传输 future 可取消；不冒充 AgentTurn DB/UI 的取消闭合证据。
        });
    }

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
            Some(Errored),
            "断网失败关闭但不是授权拒绝"
        );
        assert_eq!(transition(GateSubmitting, GateFailed), Some(Errored));
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
            assert_eq!(transition(Errored, ev), None);
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
        // 不可达不构成服务端裁决，仍失败关闭。
        let offline = submit_for_verdict(Some("tok"), ServerVerdict::Unreachable).unwrap_err();
        assert_eq!(offline.code, "ENT_GATE_UNREACHABLE");
        assert!(offline.retriable && !offline.reauth_required);
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
            })
            .unwrap(),
            ServerVerdict::Passed
        );
        assert_eq!(
            verdict_from_gate_body(&GateCheckBody {
                verdict: "denied".into(),
                reason: Some("quota".into()),
                reauth_required: None,
            })
            .unwrap(),
            ServerVerdict::Denied
        );
        for verdict in ["", "allow", "PASSED", "unknown"] {
            assert_eq!(
                verdict_from_gate_body(&GateCheckBody {
                    verdict: verdict.into(),
                    reason: None,
                    reauth_required: None
                })
                .unwrap_err()
                .code,
                "ENT_GATE_BAD_RESPONSE"
            );
        }
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
