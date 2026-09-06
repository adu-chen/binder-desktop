//! 状态机广播 emit→listen 回路实跑验证（A-CORE-X-L-04 §5.3 · X-FORBID-007）。
//!
//! 用 tauri MockRuntime 实际经 Tauri 事件系统跑 `sm_events::emit_sm_state`，验证：
//! - 事件名格式 `sm:{machine_id}:state_changed` 正确，listen 真实收到；
//! - 载荷 JSON 字段与前端 `SmStatePayload`（smEvents.ts）对齐：machine_id/instance_id/state/terminal；
//! - 无分层段时 `segment` 省略（serde skip_serializing_if），有段时携带。
//! 这把「应该能工作」的广播原语变为「已观察到 emit→listen 实跑投递」。所有机器广播均经此单一原语。

use binder_lib::sm_events::{emit_sm_state, SmStatePayload};
use std::sync::{Arc, Mutex};
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::Listener;

fn mock_app() -> tauri::App<tauri::test::MockRuntime> {
    mock_builder().build(mock_context(noop_assets())).unwrap()
}

/// 在 mock app 上监听某事件，发一条载荷，回收 listen 实际收到的 JSON。
fn roundtrip(event: &str, payload: SmStatePayload) -> serde_json::Value {
    let app = mock_app();
    let got: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let sink = got.clone();
    app.handle().listen(event.to_string(), move |ev| {
        *sink.lock().unwrap() = Some(ev.payload().to_string());
    });
    emit_sm_state(app.handle(), payload);
    let raw = got
        .lock()
        .unwrap()
        .clone()
        .unwrap_or_else(|| panic!("未收到事件: {event}"));
    serde_json::from_str(&raw).unwrap()
}

// covers: BR-SYS-STATE-003
// SM-DE-01 终态广播：事件名 + 字段 + terminal=true；无段省略 segment（前端 deStore.applyBackendState 据此投影）。
#[test]
fn de01_terminal_broadcast_roundtrip() {
    let v = roundtrip(
        "sm:SM-DE-01:state_changed",
        SmStatePayload::new("SM-DE-01", "d1", "Accepted", true),
    );
    assert_eq!(v["machine_id"], "SM-DE-01");
    assert_eq!(v["instance_id"], "d1");
    assert_eq!(v["state"], "Accepted");
    assert_eq!(v["terminal"], true);
    assert!(v.get("segment").is_none(), "无分层段应省略 segment");
}

// SM-WS-01 保存机：rel_path 作实例、非终态 Saving（前端 edStore.applyWsSaveState 据此投影标签态）。
#[test]
fn ws01_saving_broadcast_roundtrip() {
    let v = roundtrip(
        "sm:SM-WS-01:state_changed",
        SmStatePayload::new("SM-WS-01", "doc/a.md", "Saving", false),
    );
    assert_eq!(v["machine_id"], "SM-WS-01");
    assert_eq!(v["instance_id"], "doc/a.md");
    assert_eq!(v["state"], "Saving");
    assert_eq!(v["terminal"], false);
}

// SM-AG-01 回合机：turn_id 实例 + Streaming 活动态，保留后端可观测性与集成契约；Agent UI 由 TurnEvent 驱动。
#[test]
fn ag01_streaming_broadcast_roundtrip() {
    let v = roundtrip(
        "sm:SM-AG-01:state_changed",
        SmStatePayload::new("SM-AG-01", "t-1", "Streaming", false),
    );
    assert_eq!(v["machine_id"], "SM-AG-01");
    assert_eq!(v["state"], "Streaming");
}

// 分层段（SM-SYS-01 ENT 承接段）：with_segment 携带 segment 字段（A-CORE-X-L-04 §4.2/§5.4）。
#[test]
fn segment_is_carried_when_present() {
    let v = roundtrip(
        "sm:SM-SYS-01:state_changed",
        SmStatePayload::new("SM-SYS-01", "sess-1", "Streaming", false).with_segment("ent_invoke"),
    );
    assert_eq!(v["segment"], "ent_invoke");
    assert_eq!(v["machine_id"], "SM-SYS-01");
}
