/**
 * 状态机广播监听基础层（A-CORE-X-L-04 §5.5 Task 2-N · X-FORBID-007）。
 *
 * 监听后端 `sm:{machine_id}:state_changed` 事件，把权威态转发到对应 store 的投影入口
 * （`applyBackendState` 等）。store 只呈现与转达意图，业务态权威归后端状态机。
 * 随各机集成（Phase 2 逐台）在此注册新的监听。
 */

import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { deStore } from "../store/deStore";
import { edStore } from "../store/edStore";
import { agStore } from "../store/agStore";
import type { SmDe01Payload, SmWs02Payload, SmWs01Payload, SmAg01Payload, SmDoc01Payload } from "../types/smEvents";

const unlisteners: UnlistenFn[] = [];
let startPromise: Promise<void> | null = null;
let listenerGeneration = 0;

/** 启动全部状态机监听（幂等：重复调用不重复注册）。在 Workbench 挂载时调用。 */
export async function startSmListeners(): Promise<void> {
  if (unlisteners.length > 0) return;
  if (startPromise) return startPromise;
  const generation = ++listenerGeneration;
  const task = (async () => {
    const listeners: UnlistenFn[] = [];
    const register = async <Payload>(event: string, handler: (event: { payload: Payload }) => void) => {
      const unlisten = await listen<Payload>(event, handler);
      // StrictMode 卸载可能发生在 await listen 期间。每获得一个监听即检查，避免后续注册期间
      // 留下短暂的旧回调，亦无需等整组监听全部完成才清理。
      if (generation !== listenerGeneration) {
        unlisten();
        for (const registered of listeners) registered();
        return false;
      }
      listeners.push(unlisten);
      return true;
    };

    // SM-DE-01：diff 终态机（Task 2-B）。
    if (!await register<SmDe01Payload>("sm:SM-DE-01:state_changed", ({ payload }) => {
      deStore.applyBackendState(payload.instance_id, payload.state, payload.terminal);
    })) return;
    // SM-WS-02：diff 落盘机（Task 2-D）——applying 过渡 / Applied / ApplyError。
    if (!await register<SmWs02Payload>("sm:SM-WS-02:state_changed", ({ payload }) => {
      deStore.applyWsApplyState(payload.instance_id, payload.state);
    })) return;
    // SM-WS-01：保存机（Task 2-C）——instance_id 为 rel_path；Saving/Saved 投影到标签态。
    if (!await register<SmWs01Payload>("sm:SM-WS-01:state_changed", ({ payload }) => {
      edStore.applyWsSaveState(payload.instance_id, payload.state);
    })) return;
    // SM-AG-01：代理回合机（Task 2-E）——instance_id 为 turn_id；驱动 sending 输入门控。
    if (!await register<SmAg01Payload>("sm:SM-AG-01:state_changed", ({ payload }) => {
      agStore.applyTurnState(payload.instance_id, payload.state);
    })) return;
    // SM-DOC-01：docx 开/存转换机（PI-ED-07 #7）——OpenConverting 投影为会话加载反馈。
    if (!await register<SmDoc01Payload>("sm:SM-DOC-01:state_changed", ({ payload }) => {
      edStore.applyDocConvertState(payload.instance_id, payload.state);
    })) return;
    unlisteners.push(...listeners);
  })();
  startPromise = task;
  try {
    await task;
  } finally {
    if (startPromise === task) startPromise = null;
  }
}

/** 注销全部监听（Workbench 卸载 / 应用关闭时调用）。 */
export function stopSmListeners(): void {
  // 使尚在 await listen 的启动任务失效；其完成时会自行注销获得的监听。
  listenerGeneration += 1;
  for (const un of unlisteners) un();
  unlisteners.length = 0;
  startPromise = null;
}
