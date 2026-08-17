---
文档编号：AG-X-D-03
文档状态：A
负责模块：AG
文档职责：WP-AG10-01 独立治理说明
上游约束：A-AG-X-D-01、A-AG-X-L-04、A-AG-M-S-01、A-AG-M-T-01、A-AG-I-P-01、A-SYS-I-P-03、A-ENT-M-T-01
直接承接：Tool Capability、Editor/Diff、Session/UI、所有主回合终态消费者
使用边界：本文只描述主回合执行内核，不决定用户意图、上下文内容、Prompt 文本或 Diff 业务状态
变更要求：状态、终态、门禁、取消或网关轮次语义变化时同步状态机、接口、前端投影、持久化和相关测试
---

# WP-AG10-01 · Execution Kernel 情况说明

## 1. 模块定位与当前裁决

该模块负责把一个已经组装好的文档 Agent 回合安全地执行完：真实门禁、模型流、工具续轮、取消、错误和
唯一终态。它决定“是否能执行、执行到哪一步停止”，不决定“用户想做什么”。

当前等级：`Integrated`。本地自动化已覆盖单 session 单活、provider abort、工具前取消、续轮门禁和
首终态锁存；真实 AiGateway/provider 的 G5 故障注入仍未完成。

## 2. 职责、输入与输出

| 项目 | 内容 |
|---|---|
| 输入 | Runtime Profile、PromptPackage/messages、GateOutcome、工具请求、Cancellation signal |
| 输出 | 有序 Chunk/ToolRequest/ToolResult/DiffProposed 与一个用户可见终态 |
| 负责 | 单活注册、门禁、流式调用、续轮、取消传播、终态锁存、状态广播 |
| 不负责 | Intent 解析、上下文选择、工具授权策略、proposal 内容与 DE 应用 |

## 3. 当前实现地图

- `src-tauri/src/ag/turn.rs`：SM-AG-01、`TerminalKind`、`TerminalLatch`、Gate guard。
- `src-tauri/src/ent/gateway.rs`：`stream_round` 与 `RoundOutcome::{Done,ToolCalls,Failed}`；只发非终态 Chunk。
- `src-tauri/src/lib.rs`：活动回合注册、abort handle、取消命令、终态 emit、`ag_start_turn` 工具循环。
- `src/services/agService.ts`：Channel 消费、前端 `terminalSeen`、`cancelActiveTurn`。
- `src/store/agStore.ts`：activeTurn 与后端 SM-AG-01 状态投影。

## 4. 当前工作流程

注册同 session 单一活动回合
↓
Assembling → AwaitingGate
↓ 本次服务端 `check_gate`
GatePassed → Streaming；Denied/Error → 唯一终态
↓
Abortable provider stream
↓
Done / ToolCalls / Failed
↓ ToolCalls 时每次续调前重新 Gate
ToolValidating → ToolExecuting → ResultInjecting
↓
Completed / Cancelled / Denied / Error 中首个终态获胜

取消命令同时设置原子取消标志并 abort 当前 provider future；产生 proposal 前持有活动回合 effect guard，确保
“取消先发生则禁止 proposal，proposal 先开始则完成权威事件后再接受取消”。

## 5. 当前冻结项与允许变化项

必须冻结：

- GatePassed 只能来自本次服务端 GateOutcome，每一次模型续调都重新门禁。
- 同一 session 只能有一个活动 turn；重试创建新 turn，不复活 Error 终态。
- 网关层不发送 Done/Error，终态由 Runtime 单点锁存。
- Error/Denied/Cancelled 后不得再有 Done、token、工具或 proposal。
- 取消必须触达 provider 和工具边界，不能只是前端停止渲染。

允许变化：轮次上限、超时参数、用户错误文案、内部 trace 字段；变化不得削弱上述不变量。

## 6. 已验证事实

- 后端单 session 双发被 `AG_TURN_ACTIVE` 拒绝。
- 错误 turn 或不存在 turn 的取消被拒绝；正确取消设置 flag 并 abort。
- `TerminalLatch` 拒绝 Error 后的 Completed/Cancelled。
- 前端 Error 后迟到 Done 不会把状态覆盖成 completed。
- 非沙箱全量 Rust 295、Core chain 5、状态广播 4 通过。

独立验证条件：

1. Gate deny/offline 时 provider/tool 调用数为 0。
2. provider 流中、工具请求前、proposal 临界区三处取消均只有 Cancelled 终态。
3. Error→Done、Cancel→Chunk、Denied→Tool 等乱序事件被拒绝或忽略。
4. 第 N 次工具续轮 Gate 失败时立即停止，不使用首轮通过缓存。
5. 同 session 双发拒绝，不影响其他 session 的既有回合。

## 7. 已知风险与待治理点

- 尚无单一确定性网关 harness 覆盖真实 `ag_start_turn` 的全部并发时序。
- 活动回合按 session 单活，但前端 `sending` 是全局门控；这是 1.0 体验取舍，未来并发策略需单独裁决。
- `src-tauri/src/lib.rs` 同时承载多个工作包；Execution 修改容易误触 Prompt/Tool/Diff 编排。
- 终态日志是脱敏文本 trace，不是持久化的结构化 AgentTrace 存储。

## 8. 下一治理会话入口

必读顺序：A-AG-M-S-01 → A-AG-M-T-01 的 BR-AG-STATE-001/VERIFY-001 → A-SYS-I-P-03 →
`turn.rs` → `gateway.rs` → `lib.rs` 活动回合/终态/主循环 → `agService.ts`。

候选修改范围：上述执行文件与 `agRuntime.test.ts`、Rust execution tests；若改变 AgTurnEvent，必须扩展到
IPC 协议、前端 store、历史持久化和 MessageFlow 消费者。

建议会话任务描述：

> 治理 WP-AG10-01 Execution Kernel。核验每次模型调用的真实门禁、单 session 单活、取消传播和唯一终态，
> 重点检查并发竞态和终态后副作用。不要讨论 Intent/Prompt 内容或新增工具。若修改，先建立 CODE_CHANGE
> Issue Trace/Scope Lock，并用故障注入证明 Gate/Cancel/Error 路径没有 provider/tool/proposal 旁路。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-08 | v1.0 | 建立 Execution Kernel 当前实现、执行时序、不变量、验证矩阵、已知风险与独立治理入口 |
