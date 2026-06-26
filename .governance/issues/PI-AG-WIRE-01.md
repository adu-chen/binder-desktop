# Issue Trace · PI-AG-WIRE-01 客户端接 dev GW（PI-AG-10B/10F/11 dev 接线）

承接 A-AG-X-L-02 PI-AG-10B/10F/11 / A-AG-I-P-01 §2.1（AgTurnEvent 契约）/ A-AG-M-T-01 BR-AG-STATE-001
（回合生命周期）/ BR-AG-VERIFY-001（门禁前置）/ A-ENT-M-T-01 BR-ENT-STATE-001（流式三终态）/
X-SEQ-001（门禁前置）/ X-FORBID-001（客户端不持密钥、不直连 provider）。把 `route_start_turn` 桩
替换为经本地 dev GW（cloud/dev-gateway，PI-GW-DEV-01）的真实流式 AI。

## ⚠️ 留痕（unresolved_items / CLAUDE.md §9）

- **RISK-CARGO-LOCK-01**：人类 2026-06-26 明确授权加 `reqwest` 并**接受 Cargo.lock 与并发进程改动交错**
  （并发进程曾有未提交 Cargo.lock 改动）。本提交的 Cargo.lock 可能含并发进程的改动。
- dev 门禁桩=Passed；真实服务端订阅裁决（X-FORBID-002）、BYOK 加密（GW-02）、真实 AUTH/LIC 归正式 GW。
- 旧 `dispatch` 内 `ag.start_turn` 桩**保留**为兼容入口（前端已切独立命令；桩可后续移除）。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: PI-AG-WIRE-01
allowed_files:
  - src-tauri/Cargo.toml / Cargo.lock      # +reqwest +futures-util（http-only，无 TLS）
  - src-tauri/src/lib.rs                    # async 命令 ag_start_turn + 注册 generate_handler
  - src-tauri/src/ent/gateway.rs            # stream_invoke：调 dev GW /invoke/stream，SSE→AgTurnEvent
  - src/services/agService.ts               # Channel<AgTurnEvent> 真实流式（替换桩+逐字模拟）
  - src/store/agStore.ts                    # （未改，仅核对 API）
  - .governance/active_scope.json
  - .governance/issues/PI-AG-WIRE-01.md
forbidden_files: ADU/ADP/CLAUDE/APC / ipc/mod.rs / EditorTab / edStore
confirmation: YES
```

## Expected Behavior

- 后端 `#[tauri::command] async fn ag_start_turn(turn_id, prompt, provider?, model?, on_event: Channel<AgTurnEvent>)`：
  驱动 SM-AG-01（Idle→Assembling→AwaitingGate→ensure_gate_passed(Passed)→Streaming，X-SEQ-001）→ 组装
  system(SYSTEM_PROMPT_CHAT,PI-AG-07)+user → `ent::gateway::stream_invoke`。注册进 `generate_handler![dispatch, ag_start_turn]`。
- `ent::gateway::stream_invoke`：reqwest POST `localhost:8787/invoke/stream`（BINDER_GW_URL 可覆盖），
  解析归一化 SSE（chunk/done/error）→ emit `AgTurnEvent`；网关不可达→ AG_GW_UNREACHABLE 错误终态；
  流自然结束补 done（BR-SYS-STATE-002 三终态闭合）。**客户端不持密钥**（X-FORBID-001）。
- 前端 `agService.startTurn`：`new Channel<AgTurnEvent>()` + `invoke("ag_start_turn",…)`；onmessage 投影
  chunk→appendToMessage、done→finishMessage、error→错误消息；删除桩 callCommand/逐字模拟。

## Validation Commands

```
cargo build --lib && cargo test --lib 'ent::gateway::tests'
npx tsc --noEmit            # agService/agStore 无 TS 错误
# 端到端（需 dev GW + 真实 key）：启 cloud/dev-gateway（设 GW_DEEPSEEK_KEY），app 内发消息见真实流式
```

验收：cargo build 干净 + gateway 测试过；tsc agService/agStore 无错误；ag_start_turn 注册；
stream_invoke 无密钥硬编码；端到端真实 AI 由人类用自有 key 自验。

## 后续

- 工具卡片（tool_request/tool_result）、DE diff 卡片（diff_proposed→deStore.stageDiff）接入。
- BYOK 客户端入口 → `ent::byok::upload_byok` 真实 deliver。
- 正式化：迁独立云仓 + 真实 AUTH/LIC/门禁 + GW-02 加密（RISK-CARGO-LOCK-01 / dev 桩门禁解除）。
