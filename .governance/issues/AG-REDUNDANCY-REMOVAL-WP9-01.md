# Issue Trace — AG-REDUNDANCY-REMOVAL-WP9-01

## 1. 用户可见症状

Agent 当前路径已迁移到模型语义投影、调用时 CapabilityGrant、append-only TurnEvent、动态 TaskRun 投影和顶层 ProposalBatch，但仓库仍保留无生产消费者的旧关键词意图栈、前端 AG 状态广播空入口及重复批次写入，增加第二真源复活和后续维护误用风险。

## 2. 入口点

- `src-tauri/src/ag/context.rs`：旧 IntentResultV1 resolver 与 PromptPackageV1。
- `src/store/agStore.ts` / `src/types/smEvents.ts`：已退出 UI 投影的 SM-AG-01 广播类型和空入口。
- `src-tauri/src/ag/edit_runtime.rs`：当前 Proposal 仍向 `style_ops` 写 `_proposalBatch` 副本。
- `src-tauri/src/ag/task.rs`：当前事件投影测试仍操作旧 `task_plans` 缓存表。

## 3. 相关规则

`BR-AG-DATA-002`、`BR-AG-DATA-004`、`BR-AG-STATE-001`、`BR-AG-STATE-002`、
`BR-AG-PERSIST-001`、`BR-SYS-STATE-007`、`BR-SYS-STATE-008`、`BR-DE-DATA-001`。

## 4. 相关链路

`AG-TURN → AG-TOOL → DE-DIFF`。本轮只删除已被当前链路替代且无生产消费者的实现，不改变链路语义。

## 5. 生产者块

- 当前语义：`model_output.rs` 产生可 revision 的 IntentResultV2；`turn_prompt.rs` 每次调用构建 PromptPackageV2。
- 当前授权：`tool_registry.rs` 只由运行时硬事实计算一次性 CapabilityGrantV2。
- 当前状态：`turn_events.rs` 向 UI 发送安全 TurnEvent；SM 广播保留后端可观测性，不再驱动 Agent UI。
- 当前批次：`edit_runtime.rs` / `de::store` 原子持久化 ProposalBatch 与 PendingDiff 顶层批次字段。

## 6. 消费者块

- `turn_tool_loop.rs` 消费 IntentResultV2、PromptPackageV2 与 CapabilityGrantV2。
- `agService.ts` / `agStore.ts` 只按 TurnEvent seq 投影 UI。
- `de/routes.rs` 从 PendingDiff 顶层字段返回 ProposalBatch；无 `_proposalBatch` 读取者。

## 7. 路径上的空规则块

无。删除旧 `BR-AG-DATA-004-RB-AG-AG-TURN-015` 实现块后，该规则仍由 IntentResultV2 当前生产块和测试覆盖。

## 8. boundary 假设

- 全库引用搜索显示 IntentResultV1/resolver/PromptPackageV1 只在 `context.rs` 自身旧测试中使用。
- `RustTurnState`、`SmAg01Payload`、`applyTurnState` 无生产消费者；SM-AG-01 后端广播仍可独立存在。
- `_proposalBatch` 仅有新写入点和旧测试 fixture，无读取点；顶层字段是唯一批次真源。

## 9. 运行时假设

- 删除旧关键词 resolver 不影响 provider 调用，生产路径仅调用 `build_prompt_package_v2`。
- 删除前端 AG 状态广播类型不注销其他 DE/WS/DOC 状态监听。
- 删除嵌套批次副本不影响操作 payload 本体和 DE 顶层批次原子提交。

## 10. 疑似不变量破坏

- 无调用代码仍可表达完整旧意图和 PromptPackage，可能被误接回生产形成第二语义真源。
- `applyTurnState` 明示 deprecated 但仍暴露为空操作，旧测试还主动调用。
- `_proposalBatch` 与 PendingDiff 顶层批次字段重复，允许二者漂移。
- 当前投影测试通过删除旧缓存表来证明重建，继续给已退出路径制造活跃感。

## 11. 验证计划

- 编译证明删除后无隐藏消费者。
- 当前 IntentResultV2/CapabilityGrantV2/PromptPackageV2、TaskRun 投影、ProposalBatch 与 TurnEvent 测试必须保留并通过。
- 全库扫描禁用标识并逐项分类。
- 运行同输入 `test:plan` / `test:related`、治理审计和 diff check。

## Validation Commands

- `rg` 全库禁用标识扫描
- `npx tsc --noEmit`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `npm run governance:generate`
- `npm run governance:audit -- --issue .governance/issues/AG-REDUNDANCY-REMOVAL-WP9-01.md`
- `npm run test:plan -- --files [changed_files]`
- `npm run test:related -- --files [same_changed_files]`
- `git diff --check`

## Expected Behavior

Agent 生产面只保留当前语义、授权、状态、任务投影和批次真源；已替代实现无法再被误用，兼容保留项都有明确边界和删除条件。

## Scope Lock

```yaml
task_type: CODE_CHANGE
scope_id: AG-REDUNDANCY-REMOVAL-WP9-01
allowed_files:
  - .governance/active_scope.json
  - .governance/issues/AG-REDUNDANCY-REMOVAL-WP9-01.md
  - docs/00_core/A-CORE-C-R-01_ADUS.md
  - src-tauri/src/ag/context.rs
  - src-tauri/src/ag/task.rs
  - src-tauri/src/ag/edit_runtime.rs
  - src-tauri/src/de/store.rs
  - src-tauri/tests/sm_broadcast.rs
  - src/store/agStore.ts
  - src/types/smEvents.ts
  - src/components/ag/AgentUI.test.tsx
forbidden_files:
  - ADU.md
  - ADP.md
  - AGENTS.md
  - governance.config.json
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/src/db/mod.rs
  - cloud/dev-gateway/**
  - 需求、设计、规则、协议、状态机、验证卡和冻结计划文档
confirmation: YES
scope_reason: 删除已由当前 Agent 1.0 链路替代且无生产消费者的实现和夹具，不新增功能或替代抽象。
```

## 兼容读取白名单与删除条件

- 数据库 migration v12 的 `task_plans/task_plan_steps` DDL：历史迁移不可改写；当最低支持数据库版本高于 v12 且通过新 migration 清理遗留表时删除。
- `ag.list_plans` 查询名：当前返回 TaskPlanProjectionV2，供现有前端协议使用；仅在下一次明确的 IPC 破坏性版本迁移并完成消费者切换后删除或改名。
- AgentTurnPayloadV1 读取转换：仅在前端历史读取边界存在；当持久化保留窗口内不存在 V1 数据且产品版本明确停止兼容时删除。
- SM-AG-01 后端广播：保留状态机可观测性与集成契约；Agent UI 不消费，若未来统一后端诊断事件协议再单独迁移。

## 禁用标识残留分类

- IntentResultV1 / PromptPackageV1 / 关键词 resolver / `_proposalBatch` / 前端 AG 广播投影：生产源码零命中。
- IntentResultV1 / PromptPackageV1 的其余命中位于 R/X 历史文档、A 文档变更记录或迁移兼容声明，不构成运行入口；仅在产品明确关闭对应历史迁移窗口后另行做文档收口。
- `report_project`、`move_doc`、`create_doc`、`delete_doc` 及工具名 `create_folder` 只在工具注册/manifest 负向回归断言中出现，必须保留以防旧工具复活。
- `plan_workspace_operation.kind=create_folder` 是当前统一 WorkspaceOperationPlan 的参数枚举，不是旧模型工具。
- `task_plans/task_plan_steps` 只剩历史数据库 migration；Agent 运行目录没有表读写，`ag.list_plans` 仅从当前 TurnEvent route revision 投影。
- SM-AG-01 其余命中均属后端状态机、可观测广播或集成断言；Agent UI 的唯一运行投影仍是按 seq 消费 TurnEvent。

## 人类决策

- 2026-09-04 已确认按推荐方案完整执行，WP9 为冻结计划最后实现工作包。

## Task Runtime Report

```yaml
status: READY
changed_scope:
  - src-tauri/src/ag/context.rs: 删除 IntentResultV1、关键词 resolver、PromptPackageV1/build_prompt_package 及仅服务旧对象的测试；V2 测试承接来源/provenance/hash 断言
  - src-tauri/src/ag/task.rs: 删除旧 is_complex_target 兼容 helper；当前事件投影测试不再操作 task_plans 缓存表
  - src-tauri/src/ag/edit_runtime.rs: 停止向 style_ops 写入 _proposalBatch 副本
  - src-tauri/src/de/store.rs: ProposalBatch 原子持久化测试只使用顶层批次字段
  - src/store/agStore.ts / src/types/smEvents.ts / src/components/ag/AgentUI.test.tsx: 删除无消费者的 RustTurnState、SmAg01Payload、applyTurnState 及旧测试调用
  - src-tauri/tests/sm_broadcast.rs: 明确广播仅为后端可观测性，UI 由 TurnEvent 驱动
  - docs/00_core/A-CORE-C-R-01_ADUS.md: 机器重新生成，移除已删除 @GOV 块并保持规则覆盖闭合
validation:
  reference_scan: passed; 禁用生产标识归零，剩余项已按负向测试、历史/迁移文档和兼容白名单分类
  focused_frontend: passed; AgentUI 34/34
  focused_rust: passed; context 5/5、task 12/12、DE store 2/2、SM broadcast 4/4；全量回归覆盖 edit_runtime 当前测试
  typescript: passed; npx tsc --noEmit
  test_plan: passed; 精确 changed_files 选择前端全量、TypeScript、Rust 全量和治理审计
  test_related_sandbox: environment-limited; 前端 608/608，Rust 393/394；唯一失败为 LibreOffice worker signal=6
  test_related_same_input_outside_sandbox: passed; 前端 608/608；Rust lib 394/394、main 0、core_chain 5/5、sm_broadcast 4/4、doc tests 0；治理审计通过
  governance_generate: passed; 324 @GOV、142 REQ、195 REQ_MAP、100 RULE、995 tests
  governance_audit_issue: passed; 无失败检查、无 governance finding、99/99 规则有测试覆盖
  diff_check: passed
unresolved_items: []
```
