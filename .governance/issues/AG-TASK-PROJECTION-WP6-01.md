# Issue Trace — AG-TASK-PROJECTION-WP6-01

## 1. 用户可见症状

复杂任务的进度呈现仍由旧持久化计划模型驱动，包含游标、预授权工具和静态步骤；简单目标也可能被关键词
升级为 TaskRun。用户看到的是一个可执行计划器的影子，而不是模型当前路线与真实事件的轻量投影。

## 2. 入口点

- `src-tauri/src/ag/turn_tool_loop.rs::apply_model_control` 接收模型 Intent/route revision。
- `src-tauri/src/ag/turn_tool_loop.rs::run_turn_tool_loop` 执行真实 provider/tool/Decision/Proposal 事件。
- `src-tauri/src/ag/task_runtime.rs::route_resolve_decision` 裁决后恢复原 AgentTurn。
- `src/services/agService.ts::refreshSessionTaskRuns` 与 `PlanCard.tsx` 查询并展示任务投影。

## 3. 相关规则

`BR-SYS-STATE-007`、`BR-SYS-STATE-008`、`BR-AG-STATE-001`、`BR-AG-STATE-002`、
`BR-AG-DATA-003`、`BR-AG-PERSIST-001`、`X-SHARED-002`、`X-INTER-001`。

## 4. 相关链路

`AG-TURN → AG-TOOL`；DecisionRequest 确认后的资源操作继续交 `WS-OPERATION`，投影本身不进入该链路。

## 5. 生产者块

- `GUARD-AG-AG-TURN-047`：模型可见 Intent/route 封套校验。
- `RB-AG-AG-TOOL-018`：模型—工具—事件循环与 RouteUpdated/StepProgress/Decision 事实。
- `EFFECT-AG-AG-DECISION-001/002`：DecisionRequest proposed/resolved 真实事实。
- `API-AG-AG-TURN-018`：AgentTurn 入口、恢复身份与事件序列。

## 6. 消费者块

- `RB-AG-AG-TASK-001` 与 `EFFECT-AG-AG-TASK-002`：当前旧 TaskRun 生命周期/持久化。
- `GUARD/EFFECT-AG-AG-TASKPLAN-001`：当前旧步骤调度、游标推进。
- `QUERY-AG-AG-TURN-027/028`：当前旧计划读写 IPC。
- `agService.ts`、`agStore.ts`、`PlanCard.tsx`：前端任务投影消费者。

## 7. 路径上的空规则块

本路径未发现需要新增业务 owner 的 `rules: []` 块；序列化与映射辅助沿用现有主责块，不新增游离规则。

## 8. boundary 假设

- TurnEvent 以同 turn 单调 seq 追加且不可覆盖，成立。
- DecisionRequest 与 TaskRun/AgentTurn 复合身份一致后才能恢复，继续成立。
- 旧 `TaskPlanCurrentCursor...`、`PersistedTaskPlanStepStatusAndCursor` boundary 与当前设计冲突，必须删除。
- TaskPlan 只可作为事件派生投影，不携带执行边界，修改后需由重建测试证明。

## 9. 运行时假设

- provider/tool 活动只由 SM-AG-01/02 真实事件驱动，TaskRun 不得反向调用，继续成立。
- 当前 `task_runs`/`task_plans` 可作为兼容缓存读取，但投影丢失不得损坏 Decision/Proposal/ToolResult。
- confirm/cancel 只解除原 AgentTurn 的对应等待；恢复后重新组装、Gate 与授权，继续成立。
- 应用启动只能把未终态投影解释为 Interrupted，不得启动 provider/tool。

## 10. 疑似不变量破坏

- `TaskPlanV1` 持有 `required_grant` 与 `cursor`，违反 BR-SYS-STATE-008。
- `ag.create_plan` 允许外部写入计划，形成 TurnEvent 之外的第二真源。
- `start_next_plan_step_for_tool`/`finish_plan_step_for_tool` 具有静态调度和推进语义。
- `is_complex_target` 依赖中文关键词，既漏判通用复杂目标，也可能把简单回合误建 TaskRun。
- PlanCard 展示“游标”和授权字段，使展示投影看起来像执行器。
- TaskRun 的步骤由旁路 `steps_json` 写入，删除后不能仅凭 TurnEvent 确定性重建。

## 11. 验证计划

- 自动化覆盖简单回答/简单编辑零 TaskRun、复杂 route projection、route revision、真实步骤归约与等待无活动。
- 删除缓存后从相同 TurnEvent 重建相同 TaskRun/TaskPlan；验证跨 session/workspace 隔离。
- DecisionRequest confirm/cancel 保持原 turn_id，错误身份/重复恢复失败关闭，并验证恢复重新 Gate 的事件顺序。
- 扫描生产代码不存在 `ag.create_plan`、`required_grant`、TaskPlan cursor 和步骤调度函数。
- 运行实际 changed_files 的 `test:plan`、同输入 `test:related`、generate、Issue Trace audit、格式与 diff check。

## Expected Behavior

简单 AgentTurn 直接完成；复杂路线或跨执行片段事实出现后，运行时仅从 TurnEvent 派生 TaskRun/TaskPlan/TaskStep
只读投影。投影展示当前候选路线、已发生动作、等待点和结果，不执行动作、不授权、不跨重启续跑。
DecisionRequest 仍是独立真实事实，裁决后恢复原 AgentTurn 并重新校验。

## Validation Commands

- `npm run governance:generate`
- `npm run governance:audit -- --issue .governance/issues/AG-TASK-PROJECTION-WP6-01.md`
- `npm run test:plan -- --files [...]`
- `npm run test:related -- --files [...]`
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
- `cargo check --manifest-path src-tauri/Cargo.toml --lib --tests`
- `git diff --check`

## Scope Lock

```yaml
task_type: CODE_CHANGE
governance_phase: IMPLEMENTATION
allowed_files:
  - .governance/active_scope.json
  - .governance/issues/AG-TASK-PROJECTION-WP6-01.md
  - docs/00_core/A-CORE-C-R-01_ADUS.md
  - src-tauri/src/types/mod.rs
  - src-tauri/src/ipc/mod.rs
  - src-tauri/src/lib.rs
  - src-tauri/src/ag/runtime.rs
  - src-tauri/src/ag/task.rs
  - src-tauri/src/ag/task_runtime.rs
  - src-tauri/src/ag/turn_control.rs
  - src-tauri/src/ag/turn_events.rs
  - src-tauri/src/ag/turn_prompt.rs
  - src-tauri/src/ag/turn_runtime.rs
  - src-tauri/src/ag/turn_tool_loop.rs
  - src/services/agService.ts
  - src/services/agRuntime.test.ts
  - src/store/agStore.ts
  - src/components/ag/PlanCard.tsx
forbidden_files:
  - 控制文档、APC、需求/设计/规则/协议/验证卡/计划、数据库迁移、云端与 WP7-WP9 文件
confirmation: YES
scope_reason: 只实现冻结 WP6 的事件派生任务投影与 DecisionRequest 同回合恢复，删除旧计划执行语义。
```

## 人类决策

- 2026-09-04 已确认按推荐方案完整执行；2026-09-05 明确要求继续下一项，`confirmation: YES`。

## Task Runtime Report

```yaml
status: READY
changed_scope:
  - TaskRunProjectionV2 / TaskPlanProjectionV2 / TaskStepProjectionV2 共享数据契约
  - 复杂模型路线或真实任务级等待点才建立 TaskRun 稳定身份
  - append-only TurnEvent 确定性重建已发生步骤、等待、结果和当前 route revision
  - DecisionRequest confirm/cancel 继续绑定原 AgentTurn 并重新过 Gate
  - 启动恢复只追加 Interrupted 观察，不续跑 provider/tool
  - ag.create_plan、计划游标、required_grant、步骤调度器与 steps_json 旁路写入删除
validation:
  governance_generate: passed
  governance_issue_audit: passed
  production_legacy_scheduler_scan: passed; zero matches
  git_diff_check: passed
  cargo_fmt_check: passed
  cargo_check_lib_tests: passed
  typescript: passed
  frontend_task_projection: 60/60 passed
  rust_task_projection: 12/12 passed
  frontend_full: 596/596 passed
  rust_full_sandbox: lib 400/401 passed; only LibreOffice Worker signal=6（环境限制，非实现断言失败）
  rust_full_host: lib 401/401 passed; integration core_chain 5/5; sm_broadcast 4/4; doc tests 0/0
  test_plan_same_input: passed; selected frontend full + TypeScript + Rust full + governance audit
  test_related_same_input: passed outside sandbox with approved escalation; exit code 0
unresolved_items: []
```
