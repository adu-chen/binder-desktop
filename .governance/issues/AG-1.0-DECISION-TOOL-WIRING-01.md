# Issue Trace — AG-1.0-DECISION-TOOL-WIRING-01：模型发起真实任务级裁决

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-20
前置：AG-1.0-ORCHESTRATION-LIFECYCLE-01 已完成；范围外 LibreOffice 回归失败已由人类接受风险。

## 1. 用户可见症状

界面能够展示任务级裁决区，但模型在真实回合中没有已注册、已授权、可执行的 `propose_decision` 工具。
因此任务遇到权限、方案、范围或人工确认点时，无法稳定生成 `DecisionRequest`，确认卡只能由测试或 IPC 手工构造。

## 2. 入口点

- 模型工具声明：`src-tauri/src/ag/tool_registry.rs` — `TOOLS`、`resolve_capability_grant`、`tool_definitions_json`。
- 模型工具循环：`src-tauri/src/lib.rs` — `ag_start_turn` 内 `RoundOutcome::ToolCalls` 分派。
- 任务状态：`src-tauri/src/ag/task.rs` — `propose_decision`、`settle_turn`。
- 模型行为约束：`src-tauri/src/ag/prompts.rs` — `SYSTEM_PROMPT_CHAT`。

## 3. 相关规则

- `BR-SYS-STATE-008`：TaskRun 的确认点只由 DecisionRequest 驱动；等待确认时不可继续执行。
- `BR-AG-STATE-002`：模型工具必须完成注册、本轮 CapabilityGrant 校验与执行回注。
- `BR-AG-STATE-001`：回合可见终态真实且唯一；任务确认后才进入下一片段。
- `BR-AG-DATA-004`：当前轮意图优先，确认工具不得借历史扩张范围。
- `X-FORBID-006`：内容修改仍只经 Diff 路径；DecisionRequest 不得伪装为文档写入。

## 4. 相关链路

`AG-TURN → AG-TOOL → DecisionRequest(proposed) → TaskRun(WaitingConfirm) → structured event → DecisionBar → resolve_decision → next continuation segment`。

## 5. 生产者块

- `BR-SYS-STATE-008-RB-AG-AG-TASK-001` 位于 `src-tauri/src/ag/task.rs`：TaskRun 转移引擎。
- `src-tauri/src/ag/task.rs` — `propose_decision`：持久化 DecisionRequest 并置 TaskRun 为 `WaitingConfirm`。
- `src-tauri/src/ag/tool_registry.rs` — 工具注册、能力授权和模型 JSON Schema。
- `src-tauri/src/lib.rs` — AG 工具调用循环、工具事件和任务结算。

## 6. 消费者块

- `src-tauri/src/lib.rs` — 发送 `DecisionRequestProposed`，结束当前片段并让 `settle_turn` 投影 `WaitingConfirm`。
- `src/services/agService.ts` / `DecisionBar`（既有消费方，不修改）：接收既有事件、展示裁决并发起现有 `ag.resolve_decision`。
- `src-tauri/src/ag/task.rs` — 已有 `resolve_decision` / `begin_confirmed_continuation` 消费裁决结果。

## 7. 路径上的空规则块

- 本批新增的模型工具和执行分派属于 AG-TURN / AG-TOOL 核心路径，必须映射既有规则，不使用 `rules: []`。
- 测试以 `// covers: BR-SYS-STATE-008` / `BR-AG-STATE-002` 连接，不新增 triage 例外。

## 8. boundary 假设

- `lifecycle_assumption=decision-ends-segment`：成功生成 DecisionRequest 后必须结束当前执行片段；模型不得在同一工具循环继续读、提案或修改。
- `document_snapshot=unchanged`：DecisionRequest 只改变 TaskRun/裁决持久态，不修改 LogicalState、PendingDiff 或 DiskState。

## 9. 运行时假设

- `caller_context=model tool loop`：工具参数必须是受限 schema，kind 仅为 permission / plan_choice / scope / manual_confirm。
- `runtime_guard=CapabilityGrant`：未注册、未授权或无效参数的 `propose_decision` 均回注错误，不产生 DecisionRequest 或状态变化。
- `runtime_guard=single active segment`：成功裁决请求后不允许模型继续该片段；TaskRun 只在用户裁决后由受控续片段重新进入模型。

## 10. 疑似不变量破坏

- 若只添加前端卡片或 IPC 路由，模型仍无法发起真实裁决，违反 `BR-SYS-STATE-008` 的确认点来源。
- 若裁决工具成功后仍继续模型工具循环，可能绕过等待确认并产生额外 Diff，违反确认点边界。
- 若将裁决误标为 WriteIntent 或直接写入文档，会破坏 `X-FORBID-006` 三层分离。
- `DecisionRequest`、`TaskRun`、`TaskStep` 已有 TERM；本批不新增业务术语。

## 11. 验证计划

- Rust 单元测试：注册表 schema / grant；有效裁决请求进入 `WaitingConfirm`；非法 kind、未授权与重复请求不改变任务状态；成功请求结束片段而不产生 Diff。
- 前端回归：既有 `agRuntime` 消费 `decision_request_proposed` / `resolveDecisionAndContinue` 测试保持通过。
- 影响测试：按实际 changed_files 运行 `npm run test:plan` 和 `npm run test:related`；若全量 Rust再被已接受的 LibreOffice 外部失败阻断，如实登记。
- 链路日志：真实模型调用中核对 `ToolRequest(propose_decision) → DecisionRequestProposed → Done`，且无后续同片段工具；用户确认后才出现下一 `ag_start_turn`。
- 治理：新增 @GOV 后运行 `npm run governance:generate` 和 `npm run governance:audit`。

## 文件范围（Scope Lock 输入）

允许修改：

- `src-tauri/src/ag/task.rs`：DecisionRequest 参数/状态守卫及状态测试。
- `src-tauri/src/ag/tool_registry.rs`：工具注册、CapabilityGrant、模型 schema 与单元测试。
- `src-tauri/src/ag/prompts.rs`：模型何时可请求任务级裁决的明确约束与测试。
- `src-tauri/src/lib.rs`：模型工具循环分派、结构化事件、片段停止与集成测试。
- `src/services/agRuntime.test.ts`：既有事件投影/自动续片段的回归测试补强（如实际需要）。
- `docs/00_core/A-CORE-C-R-01_ADUS.md`：仅由 `governance:generate` 自动生成。
- `.governance/issues/AG-1.0-DECISION-TOOL-WIRING-01.md`、`.governance/active_scope.json`：运行记录与范围锁。

禁止修改：

- `AGENTS.md`、`ADU.md`、`ADP.md`、`governance.config.json`、APC、其他设计文档。
- `src-tauri/src/ipc/mod.rs`：既有 DecisionRequest 事件形状足够，本批不得扩张 wire 协议。
- `src/services/deService.ts`、`src/components/de/**`、`src/components/ed/**`：Diff/编辑器状态不属本批。
- `src-tauri/src/db/mod.rs`：既有 decision_requests 表足够，本批不迁移 schema。
- 不在允许列表中的文件默认禁止；若需要扩展，按 ADP §3.3 停止。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|----------|
| 2026-08-20 | v1.0 | 初始 Issue Trace：模型工具到真实 DecisionRequest、片段停止与确认续片段闭环。 |

## 12. Task Runtime Report — AG-1.0-DECISION-TOOL-WIRING-01

task_type: `CODE_CHANGE`
governance_phase: `IMPLEMENTATION`
final_state: `READY`（完整 Rust 回归的范围外 LibreOffice 失败延续既有人类风险接受）
issue_trace: `.governance/issues/AG-1.0-DECISION-TOOL-WIRING-01.md`

changed_files:

- `src-tauri/src/ag/tool_registry.rs`：注册 `propose_decision`、增加任务控制权限类别、按回合授权与模型 JSON Schema；补注册、授权和 schema 测试。
- `src-tauri/src/ag/task.rs`：限制裁决类别、仅允许执行中 TaskRun 创建裁决、拒绝重复 pending 请求；补状态不变测试。
- `src-tauri/src/ag/prompts.rs`：明确何时使用任务级确认及其成功后的片段停止约束。
- `src-tauri/src/lib.rs`：分派模型 `propose_decision`、验证 payload、广播已有 `DecisionRequestProposed`，并在成功后结束当前片段。
- `docs/00_core/A-CORE-C-R-01_ADUS.md`：由 `governance:generate` 自动刷新。

scope:

- mode: `explicit`
- status: `confirmed`

phase_gate:

- status: `passed`
- evidence: 冻结能力基线、AG 技术设计 §7.2 与 SM-AG-04 §2.2.2 已定义本批所需的裁决状态、事件和续片段边界。

mapping_chain:

- status: `complete`
- evidence: `BR-SYS-STATE-008 / BR-AG-STATE-002 → AG-TURN / AG-TOOL → task::propose_decision + model tool loop → task/registry tests`；`governance:audit` 根状态为 `passed`，`findings=[]`。

test_impact:

- changed_files: `src-tauri/src/ag/task.rs`、`src-tauri/src/ag/tool_registry.rs`、`src-tauri/src/ag/prompts.rs`、`src-tauri/src/lib.rs`、`src/services/agRuntime.test.ts`、`src/services/agService.ts`
- direct_modules: `AG, AST, CORE, DE, DOC, ED, PRV, REF, SYS, WS`
- affected_modules: `ACCT, AG, AST, CBT, CORE, DE, DOC, ED, ENT, KB, PRV, REF, SYS, TPL, WS`
- selection: `frontend-full + rust-full + governance:audit`

validation:

- `cargo test --manifest-path src-tauri/Cargo.toml ag::tool_registry::tests -- --test-threads=1 --nocapture` # passed，8/8。
- `cargo test --manifest-path src-tauri/Cargo.toml ag::task::tests -- --test-threads=1 --nocapture` # passed，10/10。
- `npm test -- src/services/agRuntime.test.ts` # passed，53/53；夹具仍输出既有 `ag.save_session failed {}` / `ag.save_turn failed {}` stderr，但没有测试失败。
- `npm run test:plan -- --files ...` # passed；选择器要求前端全量、TypeScript、完整 Rust 和治理审计。
- `npm run test:related -- --files ...` # failed；前端全量 26 files / 584 tests 通过、TypeScript 通过；Rust 全量 338 passed / 1 failed。
- `npm run governance:generate` # passed，ADUS 生成：258 个 `@GOV` 块、932 个测试用例。
- `npm run governance:audit` # passed，根状态 `passed`，`findings=[]`。
- `git diff --check` # passed。

stop_conditions:

- 无活动 Stop Condition。Rust 全量失败是此前已由人类接受的范围外 LibreOffice Worker 风险，不得表述为完整 Rust 回归通过。

recovery:

- 若需关闭该技术债务，另行扩展到 DOC/LibreOffice Worker 的崩溃根因与其测试，再重跑本 Scope 的 `test:related` 输入。

unresolved_items:

- 尚未在接入真实模型的交互环境完成 V-S7 链路验证：`ToolRequest(propose_decision) → DecisionRequestProposed → Done(waiting_confirm)`，且确认后才出现受控下一片段。
- 后续批次仍依序处理 TaskPlan 执行器、预览确认回执/连续 Diff、按步授权与可观测性。
