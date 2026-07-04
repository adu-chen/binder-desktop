/**
 * 后端状态机广播载荷类型（A-CORE-X-L-04 §5.2 · X-FORBID-007）。
 * 与 `src-tauri/src/sm_events.rs` 的 `SmStatePayload` 逐字段对齐：后端各核心链状态机转移成功后经
 * `sm:{machine_id}:state_changed` 广播权威态，前端 store 据此投影（不另立可漂移的并行业务态）。
 *
 * 随各机集成（Phase 2 逐台）补充对应 machine 的 state 联合类型。
 */

import type { RustDiffState, RustDiffApplyState } from "../store/deStore";
import type { RustSaveState } from "../store/edStore";
import type { RustTurnState } from "../store/agStore";

/** 统一广播载荷（泛型 S 收窄各机的 state 枚举）。 */
export interface SmStatePayload<S extends string = string> {
  machine_id: string;
  /** 同机分层承接段（如 SM-SYS-01 的 "session" / "ent_invoke"）；无分层时缺省。 */
  segment?: string;
  /** 实例标识（diff_id / turn_id / run_id …）。 */
  instance_id: string;
  /** 状态枚举名（Rust 枚举 Debug 名）。 */
  state: S;
  terminal: boolean;
}

/** SM-DE-01 · diff 终态机（Task 2-B 已集成）。 */
export type SmDe01Payload = SmStatePayload<RustDiffState> & { machine_id: "SM-DE-01" };

/** SM-WS-02 · diff 落盘机（Task 2-D 已集成）。 */
export type SmWs02Payload = SmStatePayload<RustDiffApplyState> & { machine_id: "SM-WS-02" };

/** SM-WS-01 · 保存机（Task 2-C 已集成）；instance_id 为 rel_path。 */
export type SmWs01Payload = SmStatePayload<RustSaveState> & { machine_id: "SM-WS-01" };

/** SM-AG-01 · 代理回合机（Task 2-E 已集成）；instance_id 为 turn_id。 */
export type SmAg01Payload = SmStatePayload<RustTurnState> & { machine_id: "SM-AG-01" };

/** SM-DOC-01 · docx 开/存转换机（PI-ED-07 #7 加载反馈）；instance_id 为 rel_path。 */
export type SmDoc01Payload = SmStatePayload<string> & { machine_id: "SM-DOC-01" };
