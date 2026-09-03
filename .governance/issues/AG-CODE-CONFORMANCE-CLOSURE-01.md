# Issue Trace — AG-CODE-CONFORMANCE-CLOSURE-01：Agent 1.0 规则映射代码一致性收敛

文档状态：A
任务类型：CODE_CHANGE
治理阶段：VALIDATION
任务状态：READY
创建时间：2026-09-01
上游基线：APC §4/§6/§8、A-AG-M-D-01、A-AG-M-T-01、A-AG-M-S-01、A-AG-M-T-02、A-AG-I-P-01。

## 1. 用户可见症状

Agent 功能经过多轮治理后仍保留多套互相矛盾的实现事实：1.0 已移除的 Build/CBT 接管状态仍存在于回合机；明确关闭的自动记忆仍保留完整可执行实现；Prompt 对工具授权、编辑目标与多 Proposal 执行片段的说明和真实规则/运行时相反。除此之外，旧上下文骨架、旧回合持久化路径及若干 helper 仅由自身测试引用，继续扩大代码与映射噪声。

## 2. 入口点

- `src-tauri/src/ag/turn.rs::transition`
- `src-tauri/src/ag/turn_runtime.rs::ag_start_turn`
- `src-tauri/src/ag/turn_control.rs::ag_cancel_turn`
- `src-tauri/src/ag/turn_events.rs::emit_ag_turn_done`
- `src-tauri/src/ag/turn_tool_loop.rs::run_turn_tool_loop`
- `src-tauri/src/ag/prompts.rs::SYSTEM_PROMPT_CHAT`
- `src-tauri/src/ag/context.rs` 的 Prompt/Context 结构
- `src-tauri/src/ag/memory_link.rs`
- `src/store/agStore.ts::RustTurnState`
- `src/services/agService.ts` 的 Agent UI 服务门面

## 3. 相关规则

- BR-AG-STATE-001：SM-AG-01 生命周期、真实状态转移与唯一终态。
- BR-AG-STATE-002：注册清单与本轮 `IntentResult.allowed_tools` 双重授权。
- BR-AG-STATE-004：`agent-edit-1.0` 不得自动提议、写入、检索或注入项目记忆。
- BR-AG-DATA-001/002/003/005：当前 Prompt Runtime、两类回合骨架、显式编辑目标集与截断真源。
- BR-AG-DATA-007：保留四槽模型偏好数据，但 `build` 不授予 1.0 运行能力。
- BR-AG-PERSIST-001：当前会话/回合历史持久化唯一实现。
- BR-SYS-STATE-007/008：澄清与 TaskRun/执行片段边界。

## 4. 相关链路

`REQ-AG-* → REQ_MAP → Active RULE/STATE_MACHINE → @GOV owner/consumer → 生产调用 → 覆盖测试 → ADUS`

涉及业务链：AG-TURN、AG-TOOL、AG-ASSIST、AG-MODEL-CATALOG；本轮不修改 CBT-BUILD。

## 5. 生产者块

- `ag::turn::transition` 生产 SM-AG-01 合法状态。
- `resolve_intent_result_with_targets` 与 `resolve_capability_grant` 生产本轮授权边界。
- `ExplicitEditTargetSet` 生产内容 Proposal 的目标授权集合。
- 当前 Prompt Runtime 的 `LayerKind`/PromptPackage 生产模型请求。
- `ag::history` 生产当前 AgentSession/AgentTurn 持久化事实。

## 6. 消费者块

- `turn_runtime`、`turn_tool_loop` 与 `turn_events` 消费回合状态和授权结果。
- 前端 `agStore`/`agService` 消费 Rust 状态、流式事件和 PreviewAck。
- DE/ED 消费 Proposal 并形成预览/裁决；WS 消费确认后的资源计划。
- ADUS 与治理审计消费剩余 @GOV 映射和测试覆盖。

## 7. 路径上的空规则块

本轮不新增 `rules: []`。删除的 @GOV 块均属于已经与 Active 规则相反或被唯一生产实现替代、且全仓无生产消费者的旧代码；删除后由 `governance:generate` 更新 ADUS，不留下双 owner。

## 8. boundary 假设

- `agent-edit-1.0` 是唯一可进入的 Agent 主回合 Profile。
- SM-AG-01 终态只有 Completed、Cancelled、Denied、TurnError；没有 HandedOver、takeover、fallbackIn。
- 工具注册不等于授权；模型只能从本轮精确 CapabilityGrant 选择工具。
- 读取结果、模型正文与历史路径都不能扩大 ExplicitEditTargetSet；新目标必须经 scope DecisionRequest 前置确认。
- 每张 Proposal 都终止当前模型执行片段；后续编辑只能在 PreviewAck 后以最新 LogicalState 开新片段。
- `build` 偏好槽是兼容数据，不是 Build 运行入口。

## 9. 运行时假设

- 全仓引用检索显示 `memory_link`、`SYSTEM_PROMPT_BUILD`、旧 `persist_turn/recover_turns`、旧 `assemble_context`/`TurnKind` 与 `setAgentMode` 无生产消费者。
- 当前回合历史由 `ag::history` 与 `agService` 持久化，不依赖 `turn.rs` 的旧 workspace-only payload_ref 路径。
- `turn_tool_loop` 在每张成功 Proposal 后设置 handoff 并返回；PreviewAck 由前端显式开启下一片段。
- 当前工作树包含用户既有文档/治理改动，本轮不覆盖 Scope 外变更。

## 10. 疑似不变量破坏

- 旧 HandedOver/takeover/fallbackIn 使代码状态集合大于 Active 状态机，并可能被未来调用误激活。
- 自动记忆模块的代码与测试把“关闭规则”实现成“终态自动写入”，规则语义完全反向。
- Prompt 的“全量授予”和“跨文档后置确认”会诱导模型请求未授权工具/目标，制造运行时拒绝与空转。
- Prompt 声称同片段连续 Proposal，但工具循环在首张 Proposal 后立即 handoff；模型行为与执行内核不一致。
- 完成态 helper 以 `unwrap_or(Completed)` 在转移失败时伪造 Completed，且遗漏 HandingOff 广播。
- 旧上下文/持久化骨架仅靠自测维持，形成虚假规则映射与维护负担。

## 11. 验证计划

- 删除前后执行全仓引用检索，确认不存在生产消费者与残留禁用状态/Prompt。
- 运行 `npm run governance:generate` 刷新 @GOV 派生索引。
- 对全部 changed_files 运行 `npm run test:plan` 并按同一输入运行 `npm run test:related`。
- 运行 Rust check、`npm run governance:audit` 与 `git diff --check`。
- 在 Runtime Report 中逐条记录 BR-AG-STATE-001/002/004、BR-AG-DATA-002/003/PERSIST 的语义比对结论。

## Expected Behavior

Agent 1.0 代码只表达 Active 真源中的回合、工具、Prompt、目标与持久化语义；已退出 Profile 的实现和被替代的旧骨架不再占据生产代码或测试映射；前后端状态集合一致，Prompt 与运行时拒绝边界一致，ADUS 只登记仍真实承担规则的代码块。

## Validation Commands

- `npm run governance:generate`
- `npm run test:plan -- --files [changed_files]`
- `npm run test:related -- --files [changed_files]`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `npm run governance:audit`
- `git diff --check`

## Scope Lock

```yaml
task_type: CODE_CHANGE
allowed_files:
  - .governance/active_scope.json
  - .governance/issues/AG-CODE-CONFORMANCE-CLOSURE-01.md
  - src-tauri/src/ag/mod.rs
  - src-tauri/src/ag/memory_link.rs
  - src-tauri/src/ag/turn.rs
  - src-tauri/src/ag/turn_events.rs
  - src-tauri/src/ag/turn_tool_loop.rs
  - src-tauri/src/ag/turn_runtime.rs
  - src-tauri/src/ag/turn_control.rs
  - src-tauri/src/ag/prompts.rs
  - src-tauri/src/ag/context.rs
  - src-tauri/src/ag/task.rs
  - src-tauri/src/ag/blocks.rs
  - src/store/agStore.ts
  - src/services/agService.ts
  - src/services/agRuntime.test.ts
  - src/components/ag/AgentUI.test.tsx
  - docs/00_core/A-CORE-C-R-01_ADUS.md
forbidden_files:
  - docs/01_modules/AG/**
  - src-tauri/src/cbt/**
  - src/store/modelStore.ts
  - src/components/ag/ModelConfig.tsx
confirmation: YES
scope_reason: 人类已明确要求按当前规则映射修复 Agent 实现偏差并删除冗余；范围来自 Active 规则与全仓生产调用核验。
```

## 人类决策

- `HD-AG-CONFORMANCE-01`（CONFIRMED）：2026-09-01 人类明确要求继续按规则映射审计、修复代码实现不符并删除多余冗余。
- `HD-AG-CONFORMANCE-02`（INHERITED）：本轮锚定当前已归一化的 1.0 需求/设计，不新增需求、功能或逻辑。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-09-01 | v1.0 | 建立 Agent 1.0 规则映射代码一致性收敛 Scope。 |
| 2026-09-01 | v1.1 | 完成实现收敛、ADUS 重建、规则闭环审计与全量影响验证，进入 READY。 |

## Task Runtime Report

### 规则一致性结论

| 规则 | 审计结论 | 实现处置 |
|---|---|---|
| BR-AG-STATE-001 | 原实现状态集合大于 Active SM-AG-01，终态 helper 可绕过转移引擎；取消转移覆盖不全，Gate 调用异常投影错误 | 删除 HandedOver/takeover/fallbackIn/Retry；补齐五个活动态 cancel；所有完成、取消、拒绝、错误终态均由合法来源状态和事件推导；真实广播 HandingOff→Completed；GateDenied/断网统一到 Denied |
| BR-AG-STATE-002 | Runtime 已按 IntentResult 最小授权，但 Prompt 声称“全量授予”；同片段多 Proposal 计数分支不可达 | Prompt 改为精确 CapabilityGrant；删除不可达 proposal_count/五提案分支，保留每个 Proposal 后 handoff 的唯一执行事实 |
| BR-AG-STATE-004 | 无生产调用的 memory_link 仍完整实现与测试“终态自动写记忆”，规则语义反向 | 删除 memory_link 模块及注册；在 ag_start_turn 边界登记“不触发自动项目记忆” |
| BR-AG-DATA-001/002/005 | 旧 ContextSource/assemble_context/TurnKind 仅由自身测试维持，且与当前 LayerKind/PromptPackage 真源重复 | 删除旧骨架和自证测试；保留当前 PromptPackage、LayerKind、来源校验与可追溯预算实现为唯一代码事实 |
| BR-AG-DATA-003 | Prompt 仍允许未声明既有文档后置确认，与 ExplicitEditTargetSet 前置授权相反 | 明确读取/模型/历史不能扩权；新目标必须先 scope DecisionRequest，确认后才进入后续片段 |
| BR-AG-DATA-007 | build 四槽偏好是规则要求的数据兼容，不是运行入口 | 保留模型偏好槽和兼容数据；删除 Agent Build Prompt，不增加 Build Runtime/Profile/状态 |
| BR-AG-PERSIST-001 | turn.rs 存在无生产消费者的 workspace-only payload_ref 持久化副本，当前历史 owner 已是 ag::history | 删除旧 persist_turn/recover_turns 及自证测试，保留 ag::history 唯一生产路径 |

### 删除与简化

- 15 个生产/测试文件合计新增 237 行、删除 514 行，代码净减少 277 行。
- 整体删除 `memory_link.rs` 128 行；删除旧 Context 结构/算法、旧回合持久化路径、Build Prompt、不可达多 Proposal 计数、未使用 helper 和前端无调用服务门面。
- 保留 CBT 模块和 build 模型偏好数据，原因是它们分别属于本 Scope 外的独立模块和 BR-AG-DATA-007 明确的数据兼容要求；Agent 1.0 没有到 CBT/Build 的 Active 转移或运行入口。

### 验证证据

- `npm run governance:generate`：通过；ADUS 生成 313 个解析 @GOV 块、100 条规则、27 条链路、961 个测试用例索引。
- `npm run governance:audit`：通过；310 个唯一代码块，99 条被引用规则全部有测试覆盖，未覆盖规则为 0。
- `npm run test:plan -- --files [changed_files]`：通过；选择前端全量、TypeScript、Rust 全量与治理审计。
- 前端：26 个测试文件、594 个测试全部通过；`npx tsc --noEmit` 通过。
- Rust：受限沙箱首轮仅 DOC LibreOffice Worker 因 signal 6 失败；随后在正常进程权限下重跑同一完整 `cargo test`，374 个库测试、5 个 core_chain 测试、4 个 sm_broadcast 测试全部通过。
- `cargo check`：通过；Agent 无编译警告。仍有 8 个既存 DOC dead_code 警告，属于 Scope 外未改。
- `npm run build`：通过；bundled tools、TypeScript 与 Vite 生产构建完成。
- `git diff --check`：通过。

### 未解决项

- DOC LibreOffice runtime 的 8 个既存 dead_code 警告不属于 Agent 规则链，本轮按 Scope Lock 未修改；不阻断 Agent 收敛结果。
