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
fn model_deadlines(provider: &str, model: &str) -> (std::time::Duration, std::time::Duration) {
    let capacity = crate::ag::prompt_runtime::model_capacity(provider, model);
    // One frozen model policy drives both output allowance and bounded deadlines.
    (
        std::time::Duration::from_secs(capacity.round_deadline_secs),
        std::time::Duration::from_secs(capacity.stream_idle_deadline_secs),
    )
}

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

fn gateway_idle_deadline(default: std::time::Duration) -> std::time::Duration {
    #[cfg(any(test, debug_assertions))]
    if let Some(deadline) = TEST_GATEWAY_IDLE_DEADLINE.with(std::cell::Cell::get) {
        return deadline;
    }
    gateway_deadline(default)
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
    ToolCalls(Vec<GwToolCall>, Option<serde_json::Value>),
    Done(String, Option<serde_json::Value>, Option<serde_json::Value>),
    Error(IpcError),
}

impl GatewaySseEvent {
    fn kind(&self) -> &'static str {
        match self {
            Self::Chunk(_) => "chunk",
            Self::Progress(_) => "progress",
            Self::ToolCalls(..) => "tool_calls",
            Self::Done(..) => "done",
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
                Some("tool_arguments") => "tool_arguments",
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
            GatewaySseEvent::ToolCalls(calls, json.get("argument_proofs").cloned())
        }
        "done" => {
            let reason = json
                .get("stop_reason")
                .and_then(serde_json::Value::as_str)
                .filter(|reason| !reason.is_empty())
                .ok_or_else(invalid_shape)?;
            if !matches!(
                reason,
                "stop" | "end_turn" | "stop_sequence" | "length" | "max_tokens" | "content_filter"
            ) {
                return Err(invalid_shape());
            }
            GatewaySseEvent::Done(
                reason.to_string(),
                json.get("text_proof").cloned(),
                json.get("completion_observation")
                    .filter(|value| value.is_object())
                    .map(completion_accounting),
            )
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
        if let Ok((GatewaySseEvent::Done(..) | GatewaySseEvent::ToolCalls(..), _)) = &parsed {
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

// Explicit local-development opt-in: these records contain document/history text.
// Never enable this from a model instruction, IPC request, or a release build.
fn model_io_audit_allowed(development_build: bool, configured: Option<&str>) -> bool {
    development_build
        && configured.is_some_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
}

// Some terminal/CLI collectors truncate a physical line at 64 KiB. Bound the
// serialized line, not character count: JSON escaping and CJK change byte size.
fn model_io_audit_lines(record: &serde_json::Value) -> Vec<String> {
    use sha2::{Digest, Sha256};
    const PREFIX: &str = "[MODEL-IO-AUDIT] ";
    const MAX_LINE_BYTES: usize = 8192;
    let serialized = record.to_string();
    if PREFIX.len() + serialized.len() + 1 <= MAX_LINE_BYTES {
        return vec![format!("{PREFIX}{serialized}")];
    }
    let mut fragments = Vec::new();
    let mut start = 0;
    while start < serialized.len() {
        let mut end = (start + 1024).min(serialized.len());
        while !serialized.is_char_boundary(end) {
            end -= 1;
        }
        fragments.push(&serialized[start..end]);
        start = end;
    }
    let id = uuid::Uuid::new_v4().to_string();
    let hash = format!("{:x}", Sha256::digest(serialized.as_bytes()));
    fragments.iter().enumerate().map(|(index, fragment)| {
        let part = serde_json::json!({
            "audit_marker":"[MODEL-IO-AUDIT]", "version":2, "phase":"record_fragment",
            // Full identity remains in the reconstructed record; bound repeated metadata too.
            "turn_id":record["turn_id"].as_str().map(|value| value.chars().take(128).collect::<String>()),
            "model_call_round":record["model_call_round"].as_u64(),
            "record_phase":record["phase"].as_str().map(|value| value.chars().take(64).collect::<String>()), "record_id":id,
            "part_index":index+1, "total_parts":fragments.len(),
            "total_utf8_bytes":serialized.len(), "record_sha256":hash,
            "fragment":fragment, "warning":"LOCAL DEBUG PLAINTEXT; do not publish or commit",
        });
        format!("{PREFIX}{part}")
    }).collect()
}

fn write_model_io_record(
    writer: &mut impl std::io::Write,
    record: &serde_json::Value,
) -> std::io::Result<()> {
    for line in model_io_audit_lines(record) {
        writeln!(writer, "{line}")?;
    }
    writer.flush()
}

fn open_model_io_file(
    directory: &std::path::Path,
) -> std::io::Result<(std::path::PathBuf, std::fs::File)> {
    let path = directory.join(format!(
        "binder-model-io-{}-{}.jsonl",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(&path)?;
    Ok((path, file))
}

fn emit_model_io_record(record: &serde_json::Value) {
    // The dev CLI can truncate a whole burst despite short physical lines. Bypass it for
    // the authoritative plaintext copy, only after the explicit development opt-in.
    static FILE: std::sync::OnceLock<Option<std::sync::Mutex<std::fs::File>>> =
        std::sync::OnceLock::new();
    let file = FILE.get_or_init(|| match open_model_io_file(&std::env::temp_dir()) {
        Ok((path, file)) => {
            eprintln!(
                "[MODEL-IO-AUDIT] plaintext_file={} warning=LOCAL_PRIVATE_DO_NOT_PUBLISH",
                path.display()
            );
            Some(std::sync::Mutex::new(file))
        }
        Err(error) => {
            eprintln!(
                "[MODEL-IO-AUDIT] plaintext_file_unavailable kind={:?}",
                error.kind()
            );
            None
        }
    });
    if let Some(file) = file {
        match file.lock() {
            Ok(mut file) => {
                if let Err(error) = write_model_io_record(&mut *file, record) {
                    eprintln!(
                        "[MODEL-IO-AUDIT] plaintext_write_failed kind={:?}",
                        error.kind()
                    );
                }
            }
            Err(_) => eprintln!("[MODEL-IO-AUDIT] plaintext_write_failed kind=lock_poisoned"),
        }
    }
    for line in model_io_audit_lines(record) {
        eprintln!("{line}");
    }
}

// Rebuild a fixed numeric/hash vocabulary; never persist arbitrary provider objects.
fn completion_accounting(raw: &serde_json::Value) -> serde_json::Value {
    let number = |value: &serde_json::Value| value.as_u64().filter(|n| *n <= 9_007_199_254_740_991);
    let usage = raw.get("usage").filter(|u| u.is_object()).map(|usage| {
        serde_json::json!({"promptTokens":number(&usage["promptTokens"]),
            "completionTokens":number(&usage["completionTokens"]),"totalTokens":number(&usage["totalTokens"]),
            "reasoningTokens":number(&usage["reasoningTokens"])})
    });
    let pending = raw["pendingToolArguments"].as_array().map(|calls| calls.iter().take(16).map(|call| {
        let hash = call["argumentsSha256"].as_str().filter(|s| s.len()==64 && s.bytes().all(|b| b.is_ascii_hexdigit()));
        serde_json::json!({"toolIndex":number(&call["toolIndex"]),"argumentsUtf8Bytes":number(&call["argumentsUtf8Bytes"]),
            "argumentsSha256":hash,"jsonValid":call["jsonValid"].as_bool()})
    }).collect::<Vec<_>>());
    serde_json::json!({"requestedMaxTokens":number(&raw["requestedMaxTokens"]),"usage":usage,"pendingToolArguments":pending})
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
    model_io_enabled: bool,
    turn_id: String,
    round_index: usize,
    started_at: std::time::Instant,
    terminal_emitted: bool,
}

impl RequestLifecycleTrace {
    fn new(turn_id: &str, round_index: usize) -> Self {
        Self {
            enabled: request_trace_enabled(),
            model_io_enabled: model_io_audit_allowed(
                cfg!(debug_assertions),
                std::env::var("BINDER_MODEL_IO_AUDIT").ok().as_deref(),
            ),
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

    fn model_io_record(
        &self,
        phase: &str,
        details: serde_json::Value,
    ) -> Option<serde_json::Value> {
        self.model_io_enabled.then(|| {
            serde_json::json!({
                "audit_marker": "[MODEL-IO-AUDIT]",
                "version": 1,
                "warning": "LOCAL DEBUG ONLY: contains plaintext prompts, document/history text and model output; do not publish or commit",
                "boundary": "client_gateway_normalized_not_provider_raw_sse",
                "turn_id": self.turn_id,
                "model_call_round": self.round_index,
                "phase": phase,
                "elapsed_ms": self.elapsed_ms(),
                "details": details,
            })
        })
    }

    fn emit_model_io(&self, phase: &str, details: serde_json::Value) {
        if let Some(record) = self.model_io_record(phase, details) {
            emit_model_io_record(&record);
        }
    }

    fn model_io_response(
        &self,
        event: &GatewaySseEvent,
        event_seq: usize,
    ) -> Option<serde_json::Value> {
        if !self.model_io_enabled {
            return None;
        }
        let response = match event {
            GatewaySseEvent::Chunk(text) => serde_json::json!({"text": text}),
            GatewaySseEvent::ToolCalls(calls, _) => {
                use sha2::{Digest, Sha256};
                serde_json::json!({"tool_calls": calls.iter().map(|call| {
                    let json_error = serde_json::from_str::<serde_json::Value>(&call.arguments)
                        .err().map(|error| serde_json::json!({
                            "category": format!("{:?}", error.classify()),
                            "line": error.line(), "column": error.column(),
                        }));
                    serde_json::json!({
                        "id": call.id, "name": call.name,
                        "arguments": call.arguments,
                        "arguments_utf8_bytes": call.arguments.len(),
                        "arguments_sha256": format!("{:x}", Sha256::digest(call.arguments.as_bytes())),
                        "arguments_json_error": json_error,
                    })
                }).collect::<Vec<_>>()})
            }
            GatewaySseEvent::Done(reason, proof, accounting) => {
                serde_json::json!({"stop_reason": reason,"text_proof":proof,"completion_observation":accounting})
            }
            GatewaySseEvent::Progress(phase) => serde_json::json!({"phase": phase}),
            // Error messages can echo upstream credentials; keep fixed error metadata only.
            GatewaySseEvent::Error(error) => {
                serde_json::json!({"code": error.code, "retriable": error.retriable})
            }
        };
        self.model_io_record(
            "response_event",
            serde_json::json!({
                "event_seq": event_seq, "event": event.kind(), "response": response,
            }),
        )
    }

    fn finish(&mut self, phase: &str, details: serde_json::Value) {
        self.emit(phase, details);
        self.emit_model_io(
            "request_finished",
            serde_json::json!({"terminal_phase": phase}),
        );
        self.terminal_emitted = true;
    }
}

impl Drop for RequestLifecycleTrace {
    fn drop(&mut self) {
        if !self.terminal_emitted {
            self.emit_model_io(
                "request_future_dropped",
                serde_json::json!({
                    "outcome": "not_confirmed_complete",
                }),
            );
        }
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

pub async fn stream_round<F, O>(
    turn_id: &str,
    round_index: usize,
    provider: &str,
    model: &str,
    messages: &serde_json::Value,
    tools: &serde_json::Value,
    on_chunk: F,
    mut on_observation: O,
) -> Result<RoundOutcome, IpcError>
where
    F: FnMut(String) -> Result<(), IpcError>,
    O: FnMut(&str, serde_json::Value) -> Result<(), IpcError>,
{
    let url = format!("{}/invoke/stream", gw_base_url());
    let mut trace = RequestLifecycleTrace::new(turn_id, round_index);
    on_observation(
        "runtime_build_observed",
        serde_json::json!({
            "roundIndex": round_index, "clientBuildId": runtime_build_id(),
        }),
    )?;
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
    let output_limit = crate::ag::prompt_runtime::model_capacity(provider, model).output_tokens;
    let (round_deadline, idle_deadline) = model_deadlines(provider, model);
    on_observation(
        "model_request_capacity",
        serde_json::json!({"roundIndex":round_index,"requestedMaxTokens":output_limit,
            "roundDeadlineMs":round_deadline.as_millis(),"streamIdleDeadlineMs":idle_deadline.as_millis()}),
    )?;
    let body = serde_json::json!({
        "provider": provider, "model": model, "messages": messages, "tools": tools, "token": token,
        "max_tokens": output_limit,
    });

    if trace.model_io_enabled {
        // Whitelist only model-bound fields, never the authenticated HTTP body or headers.
        trace.emit_model_io(
            "request_sending",
            serde_json::json!({
                "provider": provider, "model": model, "messages": messages, "tools": tools,
                "stream": true, "max_tokens": output_limit,
            }),
        );
    }
    let resp =
        match invocation_client_with_read_timeout(Some(gateway_idle_deadline(idle_deadline)))?
            .post(&url)
            .header("X-Binder-Turn-Id", turn_id)
            .header("X-Binder-Round-Index", round_index.to_string())
            .json(&body)
            .timeout(gateway_deadline(round_deadline))
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
    let gateway_build = resp
        .headers()
        .get("x-binder-gateway-build")
        .and_then(|v| v.to_str().ok())
        .filter(|v| v.len() == 64 && v.bytes().all(|b| b.is_ascii_hexdigit()));
    on_observation(
        "gateway_build_observed",
        serde_json::json!({
            "roundIndex": round_index, "gatewayBuildId":gateway_build,
            "gatewayBuildPresent":gateway_build.is_some(),
        }),
    )?;
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

    consume_round_stream(resp.bytes_stream(), &mut trace, on_chunk, on_observation).await
}

// Compile-embedded source identity, not the current files on disk or a permission token.
fn runtime_build_id() -> &'static str {
    static ID: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    ID.get_or_init(|| {
        use sha2::{Digest, Sha256};
        let mut hash = Sha256::new();
        for source in [
            include_str!("gateway.rs"),
            include_str!("../ag/model_output.rs"),
            include_str!("../ag/turn_tool_loop.rs"),
            include_str!("../ag/turn_runtime.rs"),
            include_str!("../ag/task.rs"),
            include_str!("../ag/prompt_runtime.rs"),
            include_str!("../ag/turn_prompt.rs"),
            include_str!("../ag/edit_projection.rs"),
            include_str!("../ag/edit_types.rs"),
            // 编辑门禁与提案构造也属于运行时契约：纳入构建号，避免"代码变了、构建号没变"
            // 让真实窗口无法证明跑的是哪一版。
            include_str!("../ag/edit_support.rs"),
            include_str!("../ag/block_ops.rs"),
            include_str!("../ag/bdocx_capabilities.rs"),
        ] {
            hash.update(source.as_bytes());
        }
        format!("{:x}", hash.finalize())
    })
}

/**
 * @GOV
 * codes: BR-ENT-STATE-001-RB-ENT-CLOUD-INVOKE-002
 * type: RB
 * chain: CLOUD-INVOKE
 * rules: BR-ENT-STATE-001
 * boundary: in=GatewayByteStreamChunkAndEvidenceCallbacks | out=RoundOutcomeAndSafeCompletionEvidenceOrCallbackError | does_not=grantCapabilityOrEmitAgentTurnTerminal
 */
// HTTP 与测试字节流共享此解析路径；本函数不产生授权或 AgentTurn 终态。
async fn consume_round_stream<S, B, E, F, O>(
    stream: S,
    trace: &mut RequestLifecycleTrace,
    mut on_chunk: F,
    mut on_observation: O,
) -> Result<RoundOutcome, IpcError>
where
    S: futures_util::Stream<Item = Result<B, E>>,
    B: AsRef<[u8]>,
    E: Into<GatewayStreamFailure>,
    F: FnMut(String) -> Result<(), IpcError>,
    O: FnMut(&str, serde_json::Value) -> Result<(), IpcError>,
{
    use futures_util::StreamExt;
    futures_util::pin_mut!(stream);
    let mut decoder = GatewaySseDecoder::default();
    let mut tool_calls: Vec<GwToolCall> = Vec::new();
    let mut final_stop_reason: Option<String> = None;
    let mut counters = StreamTraceCounters::default();
    use sha2::{Digest, Sha256};
    let mut response_text_hash = Sha256::new();
    let mut response_text_bytes = 0usize;

    while let Some(item) = stream.next().await {
        let bytes = match item {
            Ok(b) => b,
            Err(e) => {
                let error = gateway_stream_error(e.into());
                on_observation(
                    "stream_failure_observed",
                    serde_json::json!({
                        "roundIndex":trace.round_index,"code":error.code,"textUtf8Bytes":response_text_bytes,
                        "textSha256":format!("{:x}",response_text_hash.clone().finalize()),
                        "contentChunkCount":counters.content_chunk_count,"providerCompletionSeen":final_stop_reason.is_some(),
                    }),
                )?;
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
                    on_observation(
                        "stream_failure_observed",
                        serde_json::json!({
                            "roundIndex":trace.round_index,"code":error.code,"textUtf8Bytes":response_text_bytes,
                            "textSha256":format!("{:x}",response_text_hash.clone().finalize()),
                            "contentChunkCount":counters.content_chunk_count,"providerCompletionSeen":final_stop_reason.is_some(),
                        }),
                    )?;
                    trace.emit(
                        "malformed_sse_event",
                        serde_json::json!({ "code": &error.code }),
                    );
                    trace.finish("invalid_sse_event", counters.to_json());
                    return Ok(RoundOutcome::Failed(error));
                }
            };
            counters.sse_event_count += 1;
            if let Some(record) = trace.model_io_response(&event, counters.sse_event_count) {
                emit_model_io_record(&record);
            }
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
                    response_text_hash.update(text.as_bytes());
                    response_text_bytes += text.len();
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
                GatewaySseEvent::ToolCalls(calls, proofs) => {
                    let mut batch_proof_matches = true;
                    for (index, call) in calls.iter().enumerate() {
                        let argument_hash =
                            format!("{:x}", Sha256::digest(call.arguments.as_bytes()));
                        let proof_matches = proofs.as_ref().map(|value| {
                            value.as_array().is_some_and(|items| {
                                items.len() == calls.len()
                                    && items.get(index).is_some_and(|p| {
                                        p["toolIndex"].as_u64() == Some(index as u64)
                                            && p["toolCallId"].as_str() == Some(call.id.as_str())
                                            && p["toolName"].as_str() == Some(call.name.as_str())
                                            && p["argumentsUtf8Bytes"].as_u64()
                                                == Some(call.arguments.len() as u64)
                                            && p["argumentsSha256"].as_str()
                                                == Some(argument_hash.as_str())
                                    })
                            })
                        });
                        batch_proof_matches &= proof_matches != Some(false);
                        let json_error =
                            serde_json::from_str::<serde_json::Value>(&call.arguments).err();
                        on_observation(
                            "tool_arguments_observed",
                            serde_json::json!({
                                "roundIndex":trace.round_index,"toolIndex":index,
                                "toolCallId":call.id,"toolName":call.name,
                                "argumentsUtf8Bytes":call.arguments.len(),
                                "argumentsSha256":argument_hash,
                                "upstreamProofPresent":proofs.is_some(),"upstreamProofMatches":proof_matches,
                                "jsonErrorLine":json_error.as_ref().map(|e|e.line()),
                                "jsonErrorColumn":json_error.as_ref().map(|e|e.column()),
                            }),
                        )?;
                    }
                    if !batch_proof_matches {
                        return Ok(RoundOutcome::Failed(incomplete_sse_event(
                            "上下游工具参数摘要或身份不一致；本批未执行，不能继续使用",
                        )));
                    }
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
                GatewaySseEvent::Done(reason, proof, accounting) => {
                    if let Some(mut accounting) = accounting {
                        accounting["roundIndex"] = trace.round_index.into();
                        on_observation("provider_capacity_observed", accounting)?;
                    }
                    let client_hash = format!("{:x}", response_text_hash.clone().finalize());
                    let proof_matches = proof.as_ref().map(|proof| {
                        proof["textUtf8Bytes"].as_u64() == Some(response_text_bytes as u64)
                            && proof["textSha256"].as_str() == Some(client_hash.as_str())
                            && proof["contentDeltaCount"].as_u64()
                                == Some(counters.content_chunk_count as u64)
                    });
                    on_observation(
                        "provider_completion_observed",
                        serde_json::json!({
                            "roundIndex":trace.round_index,"stopReason":reason,
                            "upstreamProofPresent":proof.is_some(),"upstreamProofMatches":proof_matches,
                            "textUtf8Bytes":response_text_bytes,"textSha256":client_hash,
                            "contentChunkCount":counters.content_chunk_count,
                        }),
                    )?;
                    if proof_matches == Some(false) {
                        return Ok(RoundOutcome::Failed(incomplete_sse_event(
                            "上下游正文摘要不一致；已保留接收片段，不能确认响应完整",
                        )));
                    }
                    if matches!(reason.as_str(), "length" | "max_tokens") {
                        return Ok(RoundOutcome::Failed(incomplete_sse_event(
                            &format!("模型输出因长度限制提前结束（{reason}）；回答或工具参数不完整，不能标记为完成。已有执行结果保留，请核对后明确继续。"),
                        )));
                    }
                    if reason == "content_filter" {
                        return Ok(RoundOutcome::Failed(incomplete_sse_event(
                            "模型在完整答复前停止，不能标记为完成",
                        )));
                    }
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
        on_observation(
            "stream_failure_observed",
            serde_json::json!({
                "roundIndex":trace.round_index,"code":error.code,"textUtf8Bytes":response_text_bytes,
                "textSha256":format!("{:x}",response_text_hash.clone().finalize()),
                "contentChunkCount":counters.content_chunk_count,"providerCompletionSeen":final_stop_reason.is_some(),
            }),
        )?;
        trace.finish("stream_completion_invalid", counters.to_json());
        return Ok(RoundOutcome::Failed(error));
    }
    let text_hash = format!("{:x}", response_text_hash.finalize());
    on_observation(
        "stream_completion_observed",
        serde_json::json!({
            "roundIndex":trace.round_index,"stopReason":final_stop_reason,
            "textUtf8Bytes":response_text_bytes,"textSha256":text_hash,
            "contentChunkCount":counters.content_chunk_count,
            "providerCompletionSeen":true,"toolCallCount":tool_calls.len(),
            "contentBasis":"normalized_content_including_control",
        }),
    )?;
    trace.emit_model_io(
        "response_text_fingerprint",
        serde_json::json!({
            "text_utf8_bytes":response_text_bytes,
            "text_sha256":text_hash,
            "complete_stream":true,
        }),
    );
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
        model_io_enabled: false,
        turn_id: "fixed-stream-test".into(),
        round_index: 1,
        started_at: std::time::Instant::now(),
        terminal_emitted: false,
    };
    consume_round_stream(stream, &mut trace, on_chunk, |_, _| Ok(())).await
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
                GatewaySseEvent::Done(reason, _, _) => {
                    if matches!(reason.as_str(), "length" | "max_tokens" | "content_filter") {
                        return Err(incomplete_sse_event("模型在完整答复前停止，不能标记为完成"));
                    }
                }
                GatewaySseEvent::ToolCalls(..) => {
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

    // covers: BR-ENT-STATE-001
    // covers: BR-SYS-OBS-001
    #[test]
    fn truncated_capacity_is_observed_without_promoting_pending_arguments_to_tools() {
        let done = serde_json::json!({"stop_reason":"length","completion_observation":{
            "requestedMaxTokens":8192,"usage":{"promptTokens":12000,"completionTokens":8192,
                "totalTokens":20192,"reasoningTokens":0,"private":"SECRET"},
            "pendingToolArguments":[{"toolIndex":0,"argumentsUtf8Bytes":65000,
                "argumentsSha256":"a".repeat(64),"jsonValid":false,"arguments":"SECRET"}],"secret":"SECRET"}});
        let wire = format!("event: done\ndata: {done}\n\n");
        let mut trace = RequestLifecycleTrace {
            enabled: false,
            model_io_enabled: false,
            turn_id: "capacity".into(),
            round_index: 2,
            started_at: std::time::Instant::now(),
            terminal_emitted: false,
        };
        let mut observations = Vec::new();
        let outcome = tauri::async_runtime::block_on(consume_round_stream(
            fixed_bytes(wire.as_bytes()),
            &mut trace,
            |_| panic!("partial arguments are metadata only"),
            |kind, value| {
                observations.push((kind.to_owned(), value));
                Ok(())
            },
        ))
        .unwrap();
        let RoundOutcome::Failed(error) = outcome else {
            panic!("length cannot complete or execute")
        };
        assert_eq!(error.code, "AG_GW_STREAM_INCOMPLETE");
        assert!(error.message.contains("长度限制"));
        let capacity = &observations
            .iter()
            .find(|(kind, _)| kind == "provider_capacity_observed")
            .unwrap()
            .1;
        assert_eq!(capacity["roundIndex"], 2);
        assert_eq!(capacity["requestedMaxTokens"], 8192);
        assert_eq!(capacity["usage"]["completionTokens"], 8192);
        assert_eq!(
            capacity["pendingToolArguments"][0]["argumentsUtf8Bytes"],
            65000
        );
        assert!(!serde_json::to_string(&observations)
            .unwrap()
            .contains("SECRET"));
        let empty = completion_accounting(
            &serde_json::json!({"requestedMaxTokens":-1,"usage":{"completionTokens":"SECRET"}}),
        );
        assert!(empty["requestedMaxTokens"].is_null());
        assert!(empty["usage"]["completionTokens"].is_null());
    }

    // covers: BR-SYS-OBS-001
    #[test]
    fn plaintext_audit_file_is_private_and_preserves_every_large_record_fragment() {
        use sha2::{Digest, Sha256};
        let (path, mut file) = open_model_io_file(&std::env::temp_dir()).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(file.metadata().unwrap().permissions().mode() & 0o777, 0o600);
        }
        let record = serde_json::json!({"turn_id":"synthetic","model_call_round":1,"phase":"request_sending",
            "details":{"messages":[{"content":"中文🙂PRIVATE_SYNTHETIC".repeat(10000)}]}});
        write_model_io_record(&mut file, &record).unwrap();
        drop(file);
        let stored = std::fs::read_to_string(&path).unwrap();
        let parts = stored
            .lines()
            .map(|l| {
                serde_json::from_str::<serde_json::Value>(
                    l.strip_prefix("[MODEL-IO-AUDIT] ").unwrap(),
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        assert!(parts.len() > 100);
        assert_eq!(
            parts.len() as u64,
            parts[0]["total_parts"].as_u64().unwrap()
        );
        for (i, p) in parts.iter().enumerate() {
            assert_eq!(p["part_index"], i + 1);
            assert_eq!(p["record_id"], parts[0]["record_id"]);
        }
        let rebuilt = parts
            .iter()
            .map(|p| p["fragment"].as_str().unwrap())
            .collect::<String>();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&rebuilt).unwrap(),
            record
        );
        assert_eq!(
            rebuilt.len() as u64,
            parts[0]["total_utf8_bytes"].as_u64().unwrap()
        );
        assert_eq!(
            format!("{:x}", Sha256::digest(rebuilt.as_bytes())),
            parts[0]["record_sha256"]
        );
        std::fs::remove_file(path).unwrap();
    }

    // covers: BR-ENT-STATE-001, BR-SYS-OBS-001
    #[test]
    fn completion_evidence_preserves_stop_and_checks_upstream_text_before_accepting_done() {
        use sha2::{Digest, Sha256};
        for (reason, proof_kind, expected_error) in [
            ("stop", "match", None),
            ("stop", "missing", None),
            ("stop", "wrong", Some("AG_GW_STREAM_INCOMPLETE")),
            ("length", "match", Some("AG_GW_STREAM_INCOMPLETE")),
        ] {
            let body = "中文🙂\n\n| A | 未写完";
            let mut done = serde_json::json!({"stop_reason":reason});
            if proof_kind != "missing" {
                done["text_proof"] = serde_json::json!({"textUtf8Bytes":body.len(),
                "textSha256":if proof_kind=="wrong" {"0".repeat(64)} else {format!("{:x}",Sha256::digest(body.as_bytes()))},"contentDeltaCount":1});
            }
            let wire = format!(
                "event: chunk\ndata: {}\n\nevent: done\ndata: {done}\n\n",
                serde_json::json!({"text":body})
            );
            let mut trace = RequestLifecycleTrace {
                enabled: false,
                model_io_enabled: false,
                turn_id: "observed".into(),
                round_index: 1,
                started_at: std::time::Instant::now(),
                terminal_emitted: false,
            };
            let mut observed = Vec::new();
            let mut visible = String::new();
            let outcome = tauri::async_runtime::block_on(consume_round_stream(
                fixed_bytes(wire.as_bytes()),
                &mut trace,
                |text| {
                    visible.push_str(&text);
                    Ok(())
                },
                |kind, value| {
                    observed.push((kind.to_owned(), value));
                    Ok(())
                },
            ))
            .unwrap();
            assert_eq!(visible, body);
            match (outcome, expected_error) {
                (RoundOutcome::Done { stop_reason }, None) => assert_eq!(stop_reason, reason),
                (RoundOutcome::Failed(error), Some(code)) => assert_eq!(error.code, code),
                _ => panic!("completion proof mismatch must not be accepted as success"),
            }
            let completion = &observed
                .iter()
                .find(|(kind, _)| kind == "provider_completion_observed")
                .unwrap()
                .1;
            assert_eq!(completion["stopReason"], reason);
            assert_eq!(completion["upstreamProofPresent"], proof_kind != "missing");
            assert_eq!(
                completion["upstreamProofMatches"],
                if proof_kind == "missing" {
                    serde_json::Value::Null
                } else {
                    serde_json::json!(proof_kind == "match")
                }
            );
        }
    }

    // covers: BR-ENT-STATE-001, BR-SYS-OBS-001
    #[test]
    fn tool_argument_proof_distinguishes_transport_change_from_invalid_original_json() {
        use sha2::{Digest, Sha256};
        for proof_kind in ["match", "missing", "wrong_hash", "wrong_id", "missing_item"] {
            let arguments = "{\"broken\":\"中文🙂";
            let mut data = serde_json::json!({"tool_calls":[{"id":"original-call","name":"binder_tool_batch","arguments":arguments}]});
            if proof_kind != "missing" {
                let mut proof = serde_json::json!({"toolIndex":0,"toolCallId":"original-call","toolName":"binder_tool_batch",
                    "argumentsUtf8Bytes":arguments.len(),"argumentsSha256":format!("{:x}",Sha256::digest(arguments.as_bytes()))});
                if proof_kind == "wrong_hash" {
                    proof["argumentsSha256"] = serde_json::json!("0".repeat(64));
                }
                if proof_kind == "wrong_id" {
                    proof["toolCallId"] = serde_json::json!("another-call");
                }
                data["argument_proofs"] = if proof_kind == "missing_item" {
                    serde_json::json!([])
                } else {
                    serde_json::json!([proof])
                };
            }
            let wire = format!("event: tool_calls\ndata: {data}\n\n");
            let mut trace = RequestLifecycleTrace {
                enabled: false,
                model_io_enabled: false,
                turn_id: "argument-proof".into(),
                round_index: 1,
                started_at: std::time::Instant::now(),
                terminal_emitted: false,
            };
            let mut observed = Vec::new();
            let result = tauri::async_runtime::block_on(consume_round_stream(
                fixed_bytes(wire.as_bytes()),
                &mut trace,
                |_| Ok(()),
                |kind, value| {
                    observed.push((kind.to_owned(), value));
                    Ok(())
                },
            ))
            .unwrap();
            match result {
                RoundOutcome::ToolCalls(calls) if matches!(proof_kind, "match" | "missing") => {
                    assert_eq!(calls[0].arguments, arguments, "never repair invalid JSON")
                }
                RoundOutcome::Failed(error) if !matches!(proof_kind, "match" | "missing") => {
                    assert_eq!(error.code, "AG_GW_STREAM_INCOMPLETE")
                }
                _ => panic!("mismatched argument proof cannot reach the tool validator"),
            }
            let fact = &observed
                .iter()
                .find(|(kind, _)| kind == "tool_arguments_observed")
                .unwrap()
                .1;
            assert!(fact["jsonErrorLine"].as_u64().unwrap() > 0);
            assert_eq!(fact["upstreamProofPresent"], proof_kind != "missing");
            assert_eq!(
                fact["upstreamProofMatches"],
                if proof_kind == "missing" {
                    serde_json::Value::Null
                } else {
                    serde_json::json!(proof_kind == "match")
                }
            );
            assert!(
                !fact.to_string().contains("broken"),
                "safe evidence must not include original arguments"
            );
        }
    }

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
                    |_, _| Ok(()),
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
    fn long_output_deadlines_are_model_scoped_and_still_bounded() {
        assert_eq!(
            model_deadlines("deepseek", "deepseek-flash"),
            (
                std::time::Duration::from_secs(1_200),
                std::time::Duration::from_secs(90)
            )
        );
        assert_eq!(
            model_deadlines("deepseek", "deepseek-chat"),
            (
                std::time::Duration::from_secs(300),
                std::time::Duration::from_secs(45)
            )
        );
    }

    // covers: BR-AG-DATA-005, BR-ENT-STATE-001
    #[test]
    fn flash_capacity_reaches_the_actual_gateway_request() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let worker = std::thread::spawn(move || {
            let (mut auth, _) = listener.accept().unwrap();
            let mut buf = [0_u8; 4096];
            auth.read(&mut buf).unwrap();
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
            let (header_end, content_length) = loop {
                let count = invoke.read(&mut buf).unwrap();
                assert!(count > 0, "gateway request ended before headers");
                received.extend_from_slice(&buf[..count]);
                if let Some(end) = received.windows(4).position(|part| part == b"\r\n\r\n") {
                    let header = String::from_utf8_lossy(&received[..end]).to_ascii_lowercase();
                    let length = header
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length: "))
                        .unwrap()
                        .parse::<usize>()
                        .unwrap();
                    break (end + 4, length);
                }
            };
            while received.len() - header_end < content_length {
                let count = invoke.read(&mut buf).unwrap();
                assert!(count > 0, "gateway request body ended early");
                received.extend_from_slice(&buf[..count]);
            }
            let request: serde_json::Value =
                serde_json::from_slice(&received[header_end..header_end + content_length]).unwrap();
            write!(invoke, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\nevent: done\ndata: {{\"stop_reason\":\"stop\"}}\n\n").unwrap();
            request
        });
        let mut capacity = None;
        let _outcome = tauri::async_runtime::block_on(with_test_gateway(url, async {
            stream_round(
                "flash-capacity-turn",
                1,
                "deepseek",
                "deepseek-flash",
                &serde_json::json!([]),
                &serde_json::json!([]),
                |_| Ok(()),
                |kind, value| {
                    if kind == "model_request_capacity" {
                        capacity = Some(value);
                    }
                    Ok(())
                },
            )
            .await
            .unwrap()
        }));
        let request = worker.join().unwrap();
        assert_eq!(request["model"], "deepseek-flash");
        assert_eq!(request["max_tokens"], 16_384);
        let observed = capacity.unwrap();
        assert_eq!(observed["requestedMaxTokens"], 16_384);
        assert_eq!(observed["roundDeadlineMs"], 1_200_000);
        assert_eq!(observed["streamIdleDeadlineMs"], 90_000);
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
                        |_, _| Ok(()),
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
    fn fixed_stream_accepts_gateway_tool_argument_progress_without_executing_it() {
        tauri::async_runtime::block_on(async {
            let progress = "event: progress\ndata: {\"phase\":\"tool_arguments\"}\n\n";
            assert_eq!(progress.len(), 50);
            let source = format!(
                "{progress}event: tool_calls\ndata: {{\"tool_calls\":[{{\"id\":\"call-1\",\"name\":\"binder_tool_batch\",\"arguments\":\"{{}}\"}}]}}\n\n"
            );
            let outcome = fixed_round_stream(fixed_bytes(source.as_bytes()), |_| {
                panic!("tool argument progress must not become visible content")
            })
            .await
            .unwrap();
            match outcome {
                RoundOutcome::ToolCalls(calls) => {
                    assert_eq!(calls.len(), 1);
                    assert_eq!(calls[0].name, "binder_tool_batch");
                    assert_eq!(calls[0].arguments, "{}");
                }
                _ => panic!("a complete tool_calls event must still be required"),
            }

            let outcome = fixed_round_stream(fixed_bytes(progress.as_bytes()), |_| {
                panic!("tool argument progress must not become visible content")
            })
            .await
            .unwrap();
            assert!(matches!(outcome, RoundOutcome::Failed(error)
                if error.code == "AG_GW_STREAM_INCOMPLETE"));
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
            model_io_enabled: false,
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

        // Full payloads require a separate, development-only opt-in. The ordinary
        // request trace above remains metadata-only even when audit helpers exist.
        for configured in [None, Some(""), Some("0"), Some("false")] {
            assert!(!model_io_audit_allowed(true, configured));
        }
        for configured in ["1", "true", "YES", " on "] {
            assert!(model_io_audit_allowed(true, Some(configured)));
            assert!(!model_io_audit_allowed(false, Some(configured)));
        }
        assert!(trace
            .model_io_record("request_sending", messages.clone())
            .is_none());
        let invalid_arguments = "{\"control\":,\"calls\":[]}";
        let response = GatewaySseEvent::ToolCalls(
            vec![GwToolCall {
                id: "synthetic-call".into(),
                name: "binder_tool_batch".into(),
                arguments: invalid_arguments.into(),
            }],
            None,
        );
        assert!(trace.model_io_response(&response, 1).is_none());
        let full_trace = RequestLifecycleTrace {
            enabled: false,
            model_io_enabled: true,
            turn_id: trace.turn_id.clone(),
            round_index: trace.round_index,
            started_at: std::time::Instant::now(),
            terminal_emitted: true,
        };
        let full_request = full_trace
            .model_io_record(
                "request_sending",
                serde_json::json!({
                    "provider": "synthetic", "model": "synthetic",
                    "messages": messages, "tools": tools, "stream": true,
                }),
            )
            .unwrap();
        assert_eq!(full_request["turn_id"], "turn-trace");
        assert_eq!(full_request["model_call_round"], 2);
        assert_eq!(
            full_request["details"]["messages"][0]["content"],
            "SECRET_SYSTEM_PROMPT"
        );
        assert!(full_request["details"].get("token").is_none());
        assert!(full_request["details"].get("headers").is_none());
        let full_response = full_trace.model_io_response(&response, 3).unwrap();
        let call = &full_response["details"]["response"]["tool_calls"][0];
        assert_eq!(call["arguments"], invalid_arguments);
        assert_eq!(call["arguments_utf8_bytes"], invalid_arguments.len());
        assert_eq!(call["arguments_sha256"].as_str().unwrap().len(), 64);
        assert_eq!(call["arguments_json_error"]["category"], "Syntax");
        assert_eq!(call["arguments_json_error"]["line"], 1);
        assert!(call["arguments_json_error"]["column"].as_u64().unwrap() > 0);
        assert_eq!(full_response["details"]["event_seq"], 3);

        let valid_arguments =
            serde_json::json!({"html":"<p class=\"test\">中文\n第二行</p>"}).to_string();
        let valid_response = full_trace
            .model_io_response(
                &GatewaySseEvent::ToolCalls(
                    vec![GwToolCall {
                        id: "synthetic-valid".into(),
                        name: "binder_tool_batch".into(),
                        arguments: valid_arguments.clone(),
                    }],
                    None,
                ),
                4,
            )
            .unwrap();
        let valid_call = &valid_response["details"]["response"]["tool_calls"][0];
        assert_eq!(valid_call["arguments"], valid_arguments);
        assert!(valid_call["arguments_json_error"].is_null());
        assert_ne!(valid_call["arguments_sha256"], call["arguments_sha256"]);

        let raw_text =
            "<binder-turn-control>{\"nextAction\":\"final\"}</binder-turn-control>\n中文";
        let text_response = full_trace
            .model_io_response(&GatewaySseEvent::Chunk(raw_text.into()), 5)
            .unwrap();
        assert_eq!(text_response["details"]["response"]["text"], raw_text);
        assert_eq!(text_response.to_string().lines().count(), 1);
        let error_response = full_trace
            .model_io_response(
                &GatewaySseEvent::Error(IpcError::new(
                    "SYNTHETIC_FAILURE",
                    "SECRET_UPSTREAM_CREDENTIAL",
                    true,
                    false,
                )),
                6,
            )
            .unwrap();
        assert!(!error_response
            .to_string()
            .contains("SECRET_UPSTREAM_CREDENTIAL"));

        use sha2::{Digest, Sha256};
        // Exercise the actual formatter with >64KiB, multibyte text, escapes and
        // worst-case JSON escaping, not a mocked/truncated record.
        for repeated in ["中文🙂\n\"\\", "\u{0001}\t\r"] {
            let record = full_trace
                .model_io_record(
                    "request_sending",
                    serde_json::json!({"synthetic":repeated.repeat(40_000)}),
                )
                .unwrap();
            let original = record.to_string();
            assert!(original.len() > 65_536);
            let lines = model_io_audit_lines(&record);
            let parts = lines
                .iter()
                .map(|line| {
                    assert!(
                        line.len() + 1 <= 8192,
                        "physical log line exceeded collector-safe bound"
                    );
                    assert_eq!(line.lines().count(), 1);
                    serde_json::from_str::<serde_json::Value>(
                        line.strip_prefix("[MODEL-IO-AUDIT] ").unwrap(),
                    )
                    .unwrap()
                })
                .collect::<Vec<_>>();
            assert!(parts.len() > 1);
            for (index, part) in parts.iter().enumerate() {
                assert_eq!(part["phase"], "record_fragment");
                assert_eq!(part["part_index"], index + 1);
                assert_eq!(part["total_parts"], parts.len());
                assert_eq!(part["record_id"], parts[0]["record_id"]);
                assert_eq!(part["record_sha256"], parts[0]["record_sha256"]);
                assert_eq!(part["total_utf8_bytes"], original.len());
            }
            let reconstructed = parts
                .iter()
                .map(|part| part["fragment"].as_str().unwrap())
                .collect::<String>();
            assert_eq!(reconstructed, original);
            assert_eq!(
                format!("{:x}", Sha256::digest(reconstructed.as_bytes())),
                parts[0]["record_sha256"]
            );
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&reconstructed).unwrap(),
                record
            );
            let missing = parts[1..]
                .iter()
                .map(|part| part["fragment"].as_str().unwrap())
                .collect::<String>();
            assert_ne!(
                missing.len(),
                parts[0]["total_utf8_bytes"].as_u64().unwrap() as usize
            );
            assert_ne!(
                format!("{:x}", Sha256::digest(missing.as_bytes())),
                parts[0]["record_sha256"]
            );
            let duplicated = format!(
                "{}{}",
                reconstructed,
                parts[0]["fragment"].as_str().unwrap()
            );
            assert_ne!(
                format!("{:x}", Sha256::digest(duplicated.as_bytes())),
                parts[0]["record_sha256"]
            );
        }
        let small = model_io_audit_lines(&text_response);
        assert_eq!(small, vec![format!("[MODEL-IO-AUDIT] {text_response}")]);
        let oversized_identity =
            serde_json::json!({"turn_id":"中".repeat(10_000),"phase":"p".repeat(10_000)});
        assert!(model_io_audit_lines(&oversized_identity)
            .iter()
            .all(|line| line.len() + 1 <= 8192));
    }
}
