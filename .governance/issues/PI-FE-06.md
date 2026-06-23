# Issue Trace · PI-FE-06 Chat 面板骨架（AG 前端）

承接 A-CORE-X-L-03 §2 PI-FE-06 / AG L2（A-AG-M-T-01）/ SM-AG-01 / X-SEQ-001（门禁前置）/ BR-AG-VERIFY-001 / BR-SYS-STATE-002（流式契约）/ X-FORBID-007 / TERM-AG-001（AgentTurn）/ TERM-AG-002（ToolCall）/ TERM-CLOUD-001（SubscriptionGate 桩）。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - src-tauri/src/lib.rs                        # dispatch 增 ag.start_turn 桩路由（走 ensure_gate_passed 门禁前置）+ 注册
  - src/services/agService.ts                   # startTurn + 流式回显模拟
  - src/store/agStore.ts                         # AgentTurn 消息流 + ToolCall 投影
  - src/components/ag/ChatPanel.tsx             # chat/agent 模式 + 会话
  - src/components/ag/MessageList.tsx           # 消息流（流式回显）
  - src/components/ag/ToolCallCard.tsx          # ToolCall 6 状态
  - src/components/ag/Composer.tsx              # 输入 + 发送
  - src/shell/Workbench.tsx                       # chat 区挂载 ChatPanel
  - src/styles.css                              # 对话区样式
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md
  - .governance/active_scope.json
  - .governance/issues/PI-FE-06.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/src/ipc/mod.rs
  - src-tauri/src/ag/turn.rs
  - src-tauri/src/workspace/lifecycle.rs
confirmation: YES
scope_reason: PI-FE-06 复用 AG 既有 SM-AG-01 + ensure_gate_passed（仅 lib.rs 加桩路由）+ 前端对话 UI；模型输出桩化（云不可用），门禁前置结构保留；人类已授权连续推进。
```

## Expected Behavior

- 后端 ag.start_turn 桩路由：驱动 SM-AG-01（Idle→Assembling→AwaitingGate），经 ensure_gate_passed
  （SubscriptionGate 桩=Passed，X-SEQ-001 门禁前置结构保留，真实门禁服务端权威 A-SYS-I-P-03）→Streaming，
  回桩化助手回复。无有效订阅时门禁拒绝路径结构可达（Denied）。
- 前端 Chat 面板：chat/agent 模式切换 + 会话标签 + 消息流；发送 → 流式逐字回显（模拟 turn_stream，
  桩化标注）；composer 输入发送。
- ToolCallCard 6 状态视觉互斥：pending（排队）/ executing（旋转器+进度）/ done（OK+耗时）/
  awaiting_confirmation（危险授权门，锈红边）/ failed（可重试/跳过）/ skipped。本 PI 以演示态呈现全 6 态。
- 三种 AI 辅助入口边界注释（⌘J/⌘K 不产 Diff，归 PI-FE-08）；本 PI 仅 Chat 面板骨架。
- 会话/回合态前端投影（X-FORBID-007）；密钥不落客户端（X-FORBID-001，桩不持密钥）。

## Validation Commands

```
npx tsc --noEmit
cargo test
npm run build
npm run governance:audit -- --issue .governance/issues/PI-FE-06.md
```

验收：tsc/cargo/build 绿；ag.start_turn @GOV 追溯 BR-AG-VERIFY-001 + term_ref TERM-AG-001/TERM-CLOUD-001；
ToolCallCard 6 状态互斥；scoped 门禁无越界。
