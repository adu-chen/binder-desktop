# Issue Trace — AG-1.0-TASKPLAN-EXECUTOR-01：TaskPlan 受控步骤执行器

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-20
前置：AG-1.0-ORCHESTRATION-LIFECYCLE-01 与 AG-1.0-DECISION-TOOL-WIRING-01 已 READY；LibreOffice DOCX→PDF Worker 的稳定崩溃仍是人类已接受的范围外验证风险。

## 1. 用户可见症状

复杂编辑目标会生成并展示 TaskPlan，但该计划没有执行职责：工具调用不会推进步骤状态或 cursor，模型也不知道当前计划的可执行步骤。因此 UI 上的计划不能证明任务确实按依赖和确认边界推进，多步任务仍退化为无计划约束的工具循环。

## 2. 入口点

- `src-tauri/src/lib.rs` — `build_task_plan`、复杂目标创建计划、`ag_start_turn` 的 RuntimeControl 与工具循环。
- `src-tauri/src/ag/task.rs` — TaskPlan 持久化、TaskStep 状态、TaskRun 状态机。
- `src/services/agRuntime.test.ts` — AgentTurn 事件与 TaskRun/TaskPlan 前端投影的既有回归入口（仅在实际契约变化时修改）。

## 3. 相关规则

- `BR-SYS-STATE-008`：TaskRun 仅以真实步骤/回合/工具事件体现进度；不后台续跑、不越过确认点。
- `BR-AG-STATE-001`：回合终态真实且唯一，计划未完成不得伪装任务完成。
- `BR-AG-STATE-002`：模型工具经注册、授权、调用时校验与结果回注；计划仅在当前就绪步骤边界内约束工具顺序。
- `BR-AG-DATA-002`：运行时控制以结构化 RuntimeControl 注入模型，不由数据内容扩权。

## 4. 相关链路

`IntentResult(complex) → TaskPlan(create/load) → RuntimeControl(plan projection) → AG-TOOL(plan-ready guard) → TaskStep(started/ok/failed + cursor) → TaskRun / DecisionRequest segment boundary`。

## 5. 生产者块

- `src-tauri/src/lib.rs` — `build_task_plan` 创建“读取 → 提案”有序步骤；当前只保存，不参与执行。
- `src-tauri/src/ag/task.rs` — `task_plans` / `task_plan_steps` 与 `TaskPlanV1` JSON 是计划与执行投影的双持久表示。
- `src-tauri/src/lib.rs` — 工具结果是唯一可将计划步骤标为完成或失败的真实事件来源。

## 6. 消费者块

- `src-tauri/src/lib.rs` — RuntimeControl 向模型公开当前计划、可执行步骤与不可越过的依赖边界；工具循环只允许推进就绪计划步骤。
- `src-tauri/src/ag/task.rs` — 原子更新计划 JSON、明细步骤、cursor，并在依赖、重复推进或终态时失败关闭。
- 既有 TaskRun/TaskPlan UI 投影（不在本批改变 UI）— 回合结束或历史恢复时读取更新后的计划状态。

## 7. 路径上的空规则块

- 新增执行器逻辑属于 `AG-TURN / AG-TOOL` 核心路径，必须映射既有 `BR-SYS-STATE-008`、`BR-AG-STATE-001`、`BR-AG-STATE-002`，不得使用 `rules: []`。
- 不新增业务术语；`TaskPlan`、`TaskStep`、`TaskRun`、`CapabilityGrant` 均已有 TERM。

## 8. boundary 假设

- `plan_scope=complex_only`：仅复杂目标自动生成的 TaskPlan 进入执行器；简单目标保持单 AgentTurn 直行。
- `plan_control=ordering_not_document_mutation`：执行器不修改 LogicalState、PendingDiff 或 DiskState；内容变化继续只经 `propose_edit → DiffCard`。
- `decision_boundary=unchanged`：DecisionRequest 仍是 TaskRun WaitingConfirm 的唯一来源；DiffCard 接受/拒绝不驱动 TaskRun。

## 9. 运行时假设

- `runtime_guard=next_ready_step`：模型只能以已满足依赖、状态为 pending 的当前计划步骤推进 cursor；重复、越序或终态步骤不得产生步骤状态变化。
- `runtime_guard=tool_result_truth`：步骤成功/失败只由实际 ToolResult 写入；不得在模型文本、计划创建或 UI 渲染时预先标记完成。
- `runtime_guard=segment_reentry`：任务级裁决 confirm 后的受控续片段继续同一 TaskRun 和同一 TaskPlan，从持久 cursor 重新计算就绪步骤；不后台续跑。
- `completion_guard=plan_not_complete`：当前计划仍有未完成步骤时，普通模型完成不得把 TaskRun 伪装为完成；应给模型/用户明确的受控失败或继续信号，具体实现以不自动后台续跑为界。

## 10. 疑似不变量破坏

- 现有 `build_task_plan` 的注释明确写着“计划不驱动执行”，与冻结基线的 TaskPlan 执行语义不一致。
- `task_plan_steps` 虽已落库，`record_tool_step` 只附加 TaskRun 展示步骤，未更新计划步骤或 cursor；因此计划可显示而不真实。
- 若只在前端模拟 cursor 或将 `propose_edit` 直接视为 TaskRun 确认点，会违反真实进度与三层分离。
- 若让未确认的 DecisionRequest 后续步骤在后台继续，会违反确认边界。

## 11. 验证计划

- Rust 状态测试：依赖顺序、pending→running→ok/failed、cursor 单调、重复/越序拒绝、DecisionRequest 确认后从持久 cursor 恢复、简单任务不建计划。
- Rust/集成测试：复杂目标的 RuntimeControl 包含计划投影；工具成功才推进相应步骤；计划未完成时不能将 TaskRun 标记完成；成功裁决后不产生后台执行。
- 前端回归：既有 `agRuntime` 的确认续片段和计划读取测试保持通过；仅在事件/DTO 实际变化时扩展。
- 影响验证：对最终 changed_files 运行 `npm run test:plan`、同输入的 `npm run test:related`、定向 Rust/前端测试、`governance:generate`、`governance:audit` 与 `git diff --check`。
- 系统验证：真实模型环境发起“读取两份文档→分别生成建议”的任务，核对计划 cursor 与步骤状态只在对应 ToolResult 后推进，且用户任务级确认前不存在下一片段。

## 文件范围（Scope Lock 输入）

允许修改：

- `src-tauri/src/ag/task.rs`：TaskPlan 查询、就绪判定、原子步骤转移/cursor 与状态测试。
- `src-tauri/src/lib.rs`：复杂任务计划装配、RuntimeControl 计划投影、工具循环的计划顺序守卫和终态收束。
- `src/services/agRuntime.test.ts`：仅在既有前端回归契约需要调整时补强。
- `docs/00_core/A-CORE-C-R-01_ADUS.md`：仅由 `governance:generate` 自动生成。
- `.governance/issues/AG-1.0-TASKPLAN-EXECUTOR-01.md`、`.governance/active_scope.json`：本批运行记录与范围锁。

禁止修改：

- `AGENTS.md`、`ADU.md`、`ADP.md`、`governance.config.json`、APC、其他设计文档。
- `src-tauri/src/db/mod.rs`、`src-tauri/src/ipc/mod.rs`：既有 TaskPlan 表与 StepProgress 事件形状足够，本批不做迁移或 wire 协议扩张。
- `src-tauri/src/ag/tool_registry.rs`、`src-tauri/src/ag/context.rs`：按步骤缩窄 CapabilityGrant 属后续“按步授权”批次；本批只将计划投影加入既有 RuntimeControl。
- `src/services/agService.ts`、`src/store/agStore.ts`、`src/components/ag/**`：本批不新增 UI/DTO；既有回合结束/恢复刷新承担展示。
- `src/services/deService.ts`、`src/components/de/**`、`src/components/ed/**`：连续 Diff 的预览确认回执属后续批次。
- 不在允许列表中的文件默认禁止；若需要扩展，按 ADP §3.3 停止并请求确认。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|----------|
| 2026-08-20 | v1.0 | 初始 Issue Trace：TaskPlan 从展示数据变为受控步骤执行器，锁定真实工具结果、确认边界与非后台执行。 |

## 12. Task Runtime Report — AG-1.0-TASKPLAN-EXECUTOR-01

task_type: `CODE_CHANGE`
governance_phase: `IMPLEMENTATION`
final_state: `READY`（完整 Rust 回归的范围外 LibreOffice 失败延续既有人类风险接受）
issue_trace: `.governance/issues/AG-1.0-TASKPLAN-EXECUTOR-01.md`

changed_files:

- `src-tauri/src/ag/task.rs`：新增当前计划查询、就绪步骤守卫、`pending → running → ok/failed` 原子持久化、cursor 推进与未完成计划检查；补依赖/重复/失败状态测试。
- `src-tauri/src/lib.rs`：注入 TaskPlan RuntimeControl 投影；在工具执行前启动计划步骤、由实际 ToolResult 收束；计划未完成时拒绝把 TaskRun 标记完成；补 RuntimeControl 投影测试。
- `docs/00_core/A-CORE-C-R-01_ADUS.md`：由 `governance:generate` 自动刷新。

scope:

- mode: `explicit`
- status: `confirmed`

phase_gate:

- status: `passed`
- evidence: 冻结能力基线、SM-AG-04 §2.2.2、TaskPlan TERM 与 TaskRun/DecisionRequest 设计已定义本批状态、边界和验证要求。

mapping_chain:

- status: `complete`
- evidence: `BR-SYS-STATE-008 / BR-AG-STATE-001 / BR-AG-STATE-002 / BR-AG-DATA-002 → AG-TURN → TaskPlan executor + RuntimeControl → task/lib 测试`；`governance:audit` 根状态为 `passed`，`findings=[]`。

test_impact:

- changed_files: `src-tauri/src/ag/task.rs`、`src-tauri/src/ag/tool_registry.rs`、`src-tauri/src/ag/prompts.rs`、`src-tauri/src/lib.rs`、`src/services/agRuntime.test.ts`、`src/services/agService.ts`
- direct_modules: `AG, AST, CORE, DE, DOC, ED, PRV, REF, SYS, WS`
- affected_modules: `ACCT, AG, AST, CBT, CORE, DE, DOC, ED, ENT, KB, PRV, REF, SYS, TPL, WS`
- selection: `frontend-full + rust-full + governance:audit`

validation:

- `cargo test --manifest-path src-tauri/Cargo.toml ag::task::tests -- --test-threads=1 --nocapture` # passed，11/11。
- `cargo test --manifest-path src-tauri/Cargo.toml task_plan_runtime_projection_exposes_only_current_ready_step -- --test-threads=1 --nocapture` # passed，1/1。
- `npm run test:plan -- --files ...` # passed；选择器要求前端全量、TypeScript、完整 Rust 和治理审计。
- `npm run test:related -- --files ...` # failed；前端全量 26 files / 584 tests 通过、TypeScript 通过；Rust 全量 340 passed / 1 failed。
- `npm run governance:generate` # passed，ADUS 生成：261 个 `@GOV` 块、934 个测试用例。
- `npm run governance:audit` # passed，根状态 `passed`，`findings=[]`。
- `rustfmt --check src-tauri/src/ag/task.rs src-tauri/src/lib.rs` # passed。
- `git diff --check` # passed。

stop_conditions:

- 无活动 Stop Condition。Rust 全量失败是此前已由人类接受的范围外 LibreOffice Worker 风险，不得表述为完整 Rust 回归通过。

recovery:

- 若需关闭该技术债务，另行扩展到 DOC/LibreOffice Worker 的崩溃根因与其测试，再重跑本 Scope 的 `test:related` 输入。

unresolved_items:

- 尚未完成真实模型环境系统验证：复杂目标中核对 RuntimeControl 的 `currentReadyStep`、每个 ToolResult 后的 cursor 与步骤状态、以及未确认 DecisionRequest 前没有下一片段。
- 当前自动计划生成仍覆盖“已知目标文档”的读取→提案序列；新建后再编辑、研究后起草等更丰富的计划生成策略属于后续能力深化，不在本批修改。
- 后续批次仍依序处理预览确认/连续 Diff、按步授权与可观测性。
