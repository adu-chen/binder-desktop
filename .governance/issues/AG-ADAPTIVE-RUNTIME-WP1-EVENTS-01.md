# Issue Trace — AG-ADAPTIVE-RUNTIME-WP1-EVENTS-01

## 1. 问题描述

当前 AgentTurn 的实时 Channel 事件没有稳定事件身份和序号，也不持久化；`agent_turns.status` 由前端 `ag.save_turn` 回写，前端 `sending` 又同时受本地 Promise、Channel 终态和独立 SM 广播修改。断线、重放、乱序或终态竞态下无法证明同一事实序列得到同一投影，亦不能从后端事实恢复。

## 2. 影响范围

- Rust AgentTurn 事件类型、事件写入/终态出口、provider 分片与工具事件产生点。
- workspace.db 迁移、Agent 历史兼容缓存和删除语义。
- IPC 注册/路由与前端事件消费、历史缓存版本、回合输入门控投影。

## 3. 相关规则

`BR-AG-STATE-001/002`、`BR-AG-PERSIST-001`、`BR-AG-UI-001`、`BR-SYS-STATE-002/007/008`、`BR-SYS-PERSIST-001`；不新增或修改静态规则。

## 4. 相关链路

`AG-TURN`、`AG-TOOL`、`IPC-CALL`、`DATA-MIGRATE`。

## 5. 生产者块

- `ag_start_turn` 创建 draft 并冻结本轮事实。
- provider 流适配产生 Chunk，工具循环产生 ToolRequest/ToolResult/Proposal/Decision，统一终态出口产生 Done/Error。
- workspace.db `agent_turn_events` 追加边界分配 seq 并形成安全 `AgTurnEvent`。

## 6. 消费者块

- Tauri `Channel<AgTurnEvent>` 与 `ag.list_turn_events`。
- `agService` 事件 reducer、`agStore` 活动态/消息/工具投影。
- `ag.list_turns` 兼容展示缓存与 session 删除。
- 后续 WP2～WP6 对 Intent、Route、Evidence 与 TaskRun 的事件化实现。

## 7. 路径上的空规则块

无。`TERM-AG-022` 已在 Active 技术设计登记但尚无代码引用，本 Scope 将其挂到真实 TurnEvent owner 块。

## 8. boundary 假设

- 持久化事件是运行事实，Channel 只是同一事件的安全投影；内部事件可被安全查询过滤，但与可见事件共用 seq。
- `agent_turns.status` 只能随后端状态机终态/等待事实更新；前端缓存不能更改。
- `ag.save_turn` 只更新可丢弃展示 payload；`lastEventSeq` 只能指向已经提交的事件。
- 安全事件不携带密钥、重复用户正文或隐藏推理。

## 9. 运行时假设

- AppState 单一 SQLite 连接由 mutex 串行化；同 turn 的 `MAX(seq)+1` 与 insert 必须仍在同一数据库事务完成。
- 一个 turn_id 全局唯一并已存在 `agent_turns` draft 后才可追加事件。
- 内部事件会造成安全投影 seq 间隙；前端只要求严格递增，不要求连续。
- Channel 发送失败不回滚已提交事件，客户端可用 `afterSeq` 补流。

## 10. 疑似不变量破坏

- 无 seq 的事件无法去重，同名工具调用靠队列而非 tool_call_id 配对。
- terminal_latch 锁存与历史 status 更新分离，迟到事件仍可能投影卡片或动作。
- SM 广播与 Channel/Promise 同时写 `sending`，到达顺序不同会使输入门控回退。
- 删除 session 不删除未来新增的事件会留下孤儿历史。

## 11. 验证计划

- Rust：迁移、append-only/单调 seq、终态冻结、safe projection/afterSeq、workspace 隔离、session 删除事件计数、wire serde。
- 前端：严格递增、重复/倒序忽略、终态冻结、澄清等待与同 turn 恢复、toolCallId 精确配对。
- 运行 changed_files 的 `test:plan` / `test:related`，再执行 governance 生成/审计和完整 Rust 回归。

## Expected Behavior

每个当前可见 AgentTurn 事件先在后端成为不可变 TurnEvent，再以同一 `eventId/turnId/seq` 投影到 Channel；前端只消费更大的 seq，首终态后拒绝所有迟到事件。重启或 Channel 中断后可从 `ag.list_turn_events(afterSeq)` 获得相同安全事件序列；展示缓存滞后不改变运行事实。

## Validation Commands

- `npm run governance:generate`
- `npm run governance:audit -- --issue .governance/issues/AG-ADAPTIVE-RUNTIME-WP1-EVENTS-01.md`
- `npm run test:plan -- --files [...]`
- `npm run test:related -- --files [...]`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `git diff --check`

## Scope Lock

```yaml
task_type: CODE_CHANGE
governance_phase: IMPLEMENTATION
allowed_files:
  - .governance/active_scope.json
  - .governance/issues/AG-ADAPTIVE-RUNTIME-WP1-EVENTS-01.md
  - docs/00_core/A-CORE-C-R-01_ADUS.md
  - src-tauri/src/db/mod.rs
  - src-tauri/src/ipc/mod.rs
  - src-tauri/src/ag/turn_events.rs
  - src-tauri/src/ag/turn_control.rs
  - src-tauri/src/ag/turn_runtime.rs
  - src-tauri/src/ag/turn_tool_loop.rs
  - src-tauri/src/ag/history.rs
  - src-tauri/src/ag/history_routes.rs
  - src-tauri/src/ent/gateway.rs
  - src-tauri/src/lib.rs
  - src/services/agService.ts
  - src/store/agStore.ts
  - src/services/smListeners.ts
  - src/services/agRuntime.test.ts
  - src/services/systemRuntime.test.ts
forbidden_files:
  - Agent 需求/设计/计划真源、CapabilityGrant/TaskRun/ProposalBatch 实现、上述清单外生产与测试文件
confirmation: YES
scope_reason: 执行已确认 WP1，只收敛 TurnEvent 事实、持久化、安全投影与直接消费者。
```

## 人类决策

- 按推荐方案完整执行；每个阶段进行进度与完成度自检。

## 当前阻断（SC-01）

代码审计确认两个直接依赖未包含在初始 Scope：

- `src-tauri/src/ag/turn_control.rs`：等待澄清时的取消命令仍只广播 SM 状态；简单回合因无 task_run_id 甚至不会移除等待身份，也没有 append-only terminal TurnEvent。
- `src/services/systemRuntime.test.ts`：SM-AG-01 监听删除后，监听组数量断言仍固定为 5；只读运行已证实 15 项中仅该项失败（实际应为 4）。

按 ADP §3.3，修改前需人类明确确认将这两个文件加入 allowed_files。确认前 WP1 不得标记 READY。

2026-09-04 人类已明确确认扩展；两文件已加入 Scope，SC-01 解除并恢复 PATCHING。

## Task Runtime Report — Agent 1.0 TurnEvent 事实源收敛

task_type: CODE_CHANGE
governance_phase: IMPLEMENTATION
final_state: READY
issue_trace: AG-ADAPTIVE-RUNTIME-WP1-EVENTS-01

changed_files:
  - `.governance/active_scope.json` # 建立并关闭已确认 Scope
  - `.governance/issues/AG-ADAPTIVE-RUNTIME-WP1-EVENTS-01.md` # Issue Trace 与运行报告
  - `docs/00_core/A-CORE-C-R-01_ADUS.md` # 机器生成索引刷新
  - `src-tauri/src/db/mod.rs` # `agent_turn_events` 迁移、索引与 append-only trigger
  - `src-tauri/src/ipc/mod.rs` # 版本化 TurnEvent 安全 wire
  - `src-tauri/src/ag/turn_events.rs` # append-and-project 唯一边界、终态事务、补流
  - `src-tauri/src/ag/turn_control.rs` # 等待澄清取消持久化与身份收束
  - `src-tauri/src/ag/turn_runtime.rs` # draft V2 与内部生命周期事件
  - `src-tauri/src/ag/turn_tool_loop.rs` # 可见事件统一出口、精确 ToolCall 身份与安全摘要
  - `src-tauri/src/ag/history.rs` # 展示缓存 V2、游标校验、事件删除语义
  - `src-tauri/src/ag/history_routes.rs` # 缓存只写路由与 afterSeq 查询
  - `src-tauri/src/ent/gateway.rs` # provider 分片改由回调进入统一事件边界
  - `src-tauri/src/lib.rs` # 注册 `ag.list_turn_events`
  - `src/services/agService.ts` # 顺序消费、补流、精确配对与缓存降权
  - `src/store/agStore.ts` # 单调 seq reducer 与终态冻结
  - `src/services/smListeners.ts` # 移除 SM-AG-01 并列门控写入
  - `src/services/agRuntime.test.ts` # TurnEvent、补流、取消与安全投影回归
  - `src/services/systemRuntime.test.ts` # 监听唯一性回归

scope:
  mode: explicit
  status: confirmed

phase_gate:
  status: passed
  evidence: G1/G2/G3 已完成并提交；WP1 未改变 Intent、CapabilityGrant、TaskRun 或 ProposalBatch 语义。

mapping_chain:
  status: complete
  evidence: `BR-AG-STATE-001/002`、`BR-AG-PERSIST-001`、`BR-SYS-STATE-007/008` 经 `AG-TURN`/`AG-TOOL` 映射到新增 `@GOV` 块与覆盖测试；`TERM-AG-022` 已被真实 owner 引用。语义复核确认 DATA/EFFECT/QUERY/API 边界与实现的读写、副作用和不暴露约束一致。

test_impact:
  changed_files: 本报告 changed_files 全集
  direct_modules: AG, CORE, ED, ENT, REF, SYS, WS
  affected_modules: ACCT, AG, AST, CBT, CORE, DE, DOC, ED, ENT, KB, PRV, REF, SYS, TPL, WS
  selection: frontend-full + typescript + rust-full + governance-audit

validation:
  - `npm run test:plan -- --files [...]` # passed；选择完整前端、TypeScript、Rust 与治理审计
  - `npm run test:related -- --files [...]` # passed；前端 595/595、Rust lib 380/380、Rust integration 9/9、doc tests 通过
  - `npx tsc --noEmit` # passed
  - `cargo check --manifest-path src-tauri/Cargo.toml` # passed
  - `cargo test --manifest-path src-tauri/Cargo.toml` # passed；需允许内置 LibreOffice 本地子进程在工作区沙箱外运行
  - `npm run governance:generate` # passed；ADUS 已刷新
  - `npm run governance:audit -- --issue .governance/issues/AG-ADAPTIVE-RUNTIME-WP1-EVENTS-01.md` # passed；无 governance finding
  - `git diff --check` # passed
  - 结构审计 # passed；生产代码无 TurnEvent 直发旁路、无前端 status 回写、无按 tool name 配对、无 SM-AG-01 门控监听

stop_conditions:
  - 无

recovery:
  - 初次 Rust 全量在受限沙箱内仅 LibreOffice 端到端测试以 signal 6 失败；同一测试及完整 Rust 回归在允许本地子进程后通过，确认不是本次 Agent 代码回归。

unresolved_items:
  - `A-AG-M-T-01 §7.2.1` 的 `turn_events` 简写与共享数据真源 `agent_turn_events` 命名不一致；实现已服从共享真源，下一独立 DOC_ONLY Scope 修正。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-09-04 | v1.1 | 完成 TurnEvent 事实源实现、Scope 扩展、验证与 READY 运行报告。 |
| 2026-09-03 | v1.0 | 建立 WP1 TurnEvent 事实源实现 Scope。 |
