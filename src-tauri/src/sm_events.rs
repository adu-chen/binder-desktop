//! 状态机运行时广播（A-CORE-X-L-04 §5.2/§5.3 · X-FORBID-007）。
//!
//! 后端各核心链状态机转移成功后，经 `sm:{machine_id}:state_changed` 广播权威态到前端，
//! 前端 store 据此投影（不另立可漂移的并行业务态）。本模块提供统一载荷结构与广播助手，
//! 各模块命令处理器在 transition 成功并落库后调用 `emit_sm_state`。

use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// 统一状态机广播载荷。`machine_id` 如 "SM-DE-01"；`segment` 区分同机分层承接段
/// （如 SM-SYS-01 的 "session" / "ent_invoke"，无分层时为 None）；`instance_id` 区分实例
/// （diff_id / turn_id / run_id …）；`state` 为状态枚举 Debug 名；`terminal` 为是否终态。
#[derive(Debug, Clone, Serialize)]
pub struct SmStatePayload {
    pub machine_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segment: Option<String>,
    pub instance_id: String,
    pub state: String,
    pub terminal: bool,
}

impl SmStatePayload {
    /// 构造一条无分层段的载荷。
    pub fn new(
        machine_id: &str,
        instance_id: impl Into<String>,
        state: &str,
        terminal: bool,
    ) -> Self {
        SmStatePayload {
            machine_id: machine_id.to_string(),
            segment: None,
            instance_id: instance_id.into(),
            state: state.to_string(),
            terminal,
        }
    }

    /// 标注分层段（如 SM-SYS-01 的 "session" / "ent_invoke"）。
    pub fn with_segment(mut self, segment: &str) -> Self {
        self.segment = Some(segment.to_string());
        self
    }
}

/**
 * @GOV
 * codes:    BR-SYS-STATE-003-EFFECT-SYS-IPC-CALL-001
 * type:     EFFECT
 * chain:    IPC-CALL
 * rules:    BR-SYS-STATE-003
 * boundary: in=SmStatePayload | out=stateChangedBroadcast(smIdEventFailSilentBusinessUnblocked)
 */
/// 广播状态机转移到前端（BR-SYS-STATE-003 / A-SYS-I-P-02 §5.1，承接 X-FORBID-007）：事件名
/// `sm:{machine_id}:state_changed`，载荷 SmStatePayload；广播失败吞掉（不得因广播失败中断业务流程，
/// 落库为权威态已先行完成）。泛型 `R: Runtime`：运行期为 Wry；测试期 MockRuntime 验证 emit→listen 回路。
pub fn emit_sm_state<R: tauri::Runtime>(app: &AppHandle<R>, payload: SmStatePayload) {
    let event = format!("sm:{}:state_changed", payload.machine_id);
    let _ = app.emit(&event, payload);
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: BR-SYS-STATE-003
    // 广播信封契约（A-SYS-I-P-02 §5.1）：machine_id/instance_id/state/terminal 必备；无段省略 segment、有段携带。
    // emit→listen 真实 Tauri 事件回路另由 tests/sm_broadcast.rs（MockRuntime）实跑验证。
    #[test]
    fn payload_contract_fields_and_segment_skip() {
        // 无分层段：segment 省略（skip_serializing_if），其余字段齐备。
        let v =
            serde_json::to_value(SmStatePayload::new("SM-DE-01", "d1", "Accepted", true)).unwrap();
        assert_eq!(v["machine_id"], "SM-DE-01");
        assert_eq!(v["instance_id"], "d1");
        assert_eq!(v["state"], "Accepted");
        assert_eq!(v["terminal"], true);
        assert!(v.get("segment").is_none(), "无分层段应省略 segment");
        // 有分层段（SM-SYS-01 ENT 承接段）：segment 携带。
        let v2 = serde_json::to_value(
            SmStatePayload::new("SM-SYS-01", "sess-1", "Streaming", false)
                .with_segment("ent_invoke"),
        )
        .unwrap();
        assert_eq!(v2["segment"], "ent_invoke");
    }
}
