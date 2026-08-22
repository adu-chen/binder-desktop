# Issue Trace — AG-1.0-ORCHESTRATION-LIFECYCLE-01：任务级确认后的同一 TaskRun 续片段

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-20

## 1. 用户可见症状

复杂任务出现任务级裁决卡后，用户点击“确认并继续”会发起一条看似正常的续片段，但原 `TaskRun` 已被
错误标记为 `Interrupted`，随后又创建新的任务实例。结果是确认并未可靠延续原目标，计划、步骤和后续
确认点失去同一身份链路。

## 2. 入口点

- 前端裁决入口：`src/services/agService.ts` — `resolveDecisionAndContinue`。
- 后端回合入口：`src-tauri/src/lib.rs` — `ag_start_turn`。
- 任务状态入口：`src-tauri/src/ag/task.rs` — `resolve_decision`、`begin_turn`、`mark_stale_as_interrupted`。
- 应用启动入口：`src-tauri/src/lib.rs` — `run().setup`。

## 3. 相关规则

- `BR-SYS-STATE-008`：TaskRun 仅组织当前用户目标；任务级裁决后自动开始下一执行片段；不跨重启续跑。
- `BR-AG-STATE-001`：代理回合终态和可见状态必须真实、唯一。
- `BR-AG-PERSIST-001`：回合/任务持久化须可追踪，不能伪造新的任务身份。
- `X-SEQ-002`：前端经 Service → Tauri command → Rust 后端进入任务状态转换。

## 4. 相关链路

`AG-TURN`：`DecisionRequest resolved(confirm) → TaskRun Executing → continuation AgentTurn → TaskRun settle`。

应用重启清理是同一链路的恢复分支：`application setup → stale active TaskRun → Interrupted`，不得在每次
`ag_start_turn` 时误触发。

## 5. 生产者块

- `BR-SYS-STATE-008-RB-AG-AG-TASK-001` 位于 `src-tauri/src/ag/task.rs`：SM-AG-04 转移引擎。
- `src-tauri/src/ag/task.rs` — `resolve_decision`：产生 `Executing` 或 `Cancelled` 的任务级裁决结果。
- `src-tauri/src/lib.rs` — `ag_start_turn`：建立/沿用 TaskRun 并启动回合。
- `src/services/agService.ts` — `resolveDecisionAndContinue`：确认后请求续片段。

## 6. 消费者块

- `src-tauri/src/ag/task.rs` — `begin_turn`：消费会话内活动 TaskRun，决定沿用或新建。
- `src-tauri/src/lib.rs` — `settle_task_run`：消费回合结果，写回 TaskRun 状态并关联 AgentTurn。
- `src/services/agService.ts` — `startTurn`：消费裁决后的续片段意图，投递 `ag_start_turn`。
- `src/services/agRuntime.test.ts` 与 `src-tauri/src/ag/task.rs` 内联测试：验证前后端续片段契约。

## 7. 路径上的空规则块

- `src-tauri/src/ag/task.rs` 的状态转移块已有 `BR-SYS-STATE-008` / `BR-AG-STATE-001` 映射。
- 本批不新增业务 `rules: []` 块。若审计发现新增可执行块未映射，必须在本 Scope 内按真实职责补充，
  不以空规则规避。

## 8. boundary 假设

- `lifecycle_assumption=application-startup-only`：陈旧活动态清理只代表重启恢复边界，不能代表同一进程内的
  用户确认后续片段；当前实现在每次 `ag_start_turn` 执行，假设不成立，须修复。
- `document_snapshot=unchanged`：本批只修复任务身份与生命周期，不修改文档 LogicalState、DiffCard 或写盘路径。

## 9. 运行时假设

- `caller_context=async tauri command`：前端确认后仍经 `ag_start_turn` 进入新 AgentTurn，但必须携带并校验
  原 TaskRun 身份。
- `runtime_guard=pending-decision-gate`：存在未决 `DecisionRequest` 时，普通新消息仍被拒绝；只有已完成裁决
  的受控续片段可以沿用原 TaskRun。
- `runtime_guard=no-cross-restart-resume`：启动时清理保持；进程内确认续片段不得被当作重启恢复。

## 10. 疑似不变量破坏

- 当前 `ag_start_turn` 无条件调用 `mark_stale_as_interrupted`，在确认已把原 run 置为 `Executing` 后立即中断它，
  违反 `BR-SYS-STATE-008` 的“确认后自动下一执行片段”。
- 若只删除清理调用而不把它移至应用启动，异常重启后的非终态 run 可能被错误沿用，违反“不跨重启续跑”。
- 若续片段未明确绑定已裁决的 TaskRun，普通新消息可能错误附着于旧目标，或新建任务而丢失步骤身份。
- 本批不引入新业务术语；`continuation_task_run_id` 是既有 `TaskRun` 的技术参数，不新增 TERM。

## 11. 验证计划

- Rust：在 `ag::task` 转移测试中覆盖“决策 confirm 后续片段沿用同一 TaskRun”“普通新消息不越过未决裁决”
  以及“启动清理只中断陈旧 run”。
- 前端：在 `agRuntime` 覆盖确认续片段向 `ag_start_turn` 传递原 TaskRun 身份，且普通手动发送不携带该身份。
- 影响测试：先运行 `npm run test:plan -- --files [changed_files]`，再执行选择器给出的
  `npm run test:related -- --files [changed_files]`；按 AG critical 风险补充 Rust 定向/全量测试。
- 链路日志：真实回合中核对 `decision_request_resolved(confirm)` 后，续片段的 `task_run_id` 与裁决前相同；
  重启后旧 run 才显示 `Interrupted`。
- 治理：运行 `npm run governance:audit`、`git diff --check`；若新增或变更 @GOV 块，再先运行
  `npm run governance:generate`。本批不计划改变静态设计标注。

## 文件范围（Scope Lock 输入）

允许修改：

- `src-tauri/src/ag/task.rs`：TaskRun 沿用/恢复守卫与状态转移测试。
- `src-tauri/src/lib.rs`：应用启动清理、`ag_start_turn` 的受控续片段绑定与后端集成测试。
- `src/services/agService.ts`：确认续片段传递原 TaskRun 身份。
- `src/services/agRuntime.test.ts`：前端续片段 IPC 契约测试。
- `.governance/issues/AG-1.0-ORCHESTRATION-LIFECYCLE-01.md`、`.governance/active_scope.json`：本任务运行记录与范围锁。

禁止修改：

- `AGENTS.md`、`ADU.md`、`ADP.md`、`governance.config.json`、APC、ADUS 自动生成区。
- `src-tauri/src/ag/tool_registry.rs`、`src-tauri/src/ag/tool_exec.rs`、`src-tauri/src/ag/context.rs`：工具授权/意图收窄留给后续批次。
- `src-tauri/src/db/mod.rs`、`src-tauri/src/ipc/mod.rs`：本批不需 schema 或事件形状变更。
- `src/services/deService.ts`、`src/components/ed/**`、`src/components/de/**`：连续 Diff 与预览确认留给后续批次。
- 不在允许列表中的文件默认禁止；发现必须扩展时按 ADP §3.3 停止并请求确认。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|----------|
| 2026-08-20 | v1.0 | 初始 Issue Trace：锁定任务级确认后的同一 TaskRun 续片段与启动清理边界。 |

## 12. Task Runtime Report — AG-1.0-ORCHESTRATION-LIFECYCLE-01

task_type: `CODE_CHANGE`
governance_phase: `IMPLEMENTATION`
final_state: `READY`（范围外验证失败已由人类接受风险）
issue_trace: `.governance/issues/AG-1.0-ORCHESTRATION-LIFECYCLE-01.md`

changed_files:

- `src-tauri/src/ag/task.rs`：启动时全 workspace 清理、确认来源绑定的受控续片段入口；补同身份与跨 workspace
  恢复测试。
- `src-tauri/src/lib.rs`：启动阶段执行陈旧 TaskRun 清理；`ag.resolve_decision` 返回 TaskRun 身份；
  `ag_start_turn` 校验确认来源与 TaskRun 后才续片段。
- `src/services/agService.ts`：确认结果映射为 TaskRun 身份，自动续片段携带
  `continuationTaskRunId` 与 `continuationDecisionRequestId`。
- `src/services/agRuntime.test.ts`：断言确认续片段把原任务身份和裁决请求传给后端。
- `docs/00_core/A-CORE-C-R-01_ADUS.md`：由 `governance:generate` 刷新，登记新增 `@GOV` 映射。

scope:

- mode: `explicit`
- status: `confirmed`

phase_gate:

- status: `passed`
- evidence: 冻结能力基线、AG 技术设计 §7.2、SM-AG-04 §2.2.2 已定义本批所需的状态/事件/规则映射。

mapping_chain:

- status: `complete`
- evidence: `REQ-AG-015 → BR-SYS-STATE-008 → AG-TURN / SM-AG-04 → @GOV → ag::task`；
  `governance:audit` 根状态为 `passed`，`findings=[]`。

test_impact:

- changed_files: `src-tauri/src/ag/task.rs`、`src-tauri/src/lib.rs`、`src/services/agService.ts`、
  `src/services/agRuntime.test.ts`
- direct_modules: `AG, AST, CORE, DE, DOC, ED, PRV, REF, SYS, WS`
- affected_modules: `ACCT, AG, AST, CBT, CORE, DE, DOC, ED, ENT, KB, PRV, REF, SYS, TPL, WS`
- selection: `frontend-full + rust-full + governance:audit`

validation:

- `cargo test --manifest-path src-tauri/Cargo.toml ag::task::tests -- --test-threads=1 --nocapture` # passed，9/9。
- `npm test -- src/services/agRuntime.test.ts` # passed，53/53；测试夹具仍输出既有的
  `ag.save_session failed {}` / `ag.save_turn failed {}` stderr。
- `npm run test:related -- --files ...` # failed；前端全量 26 files / 584 tests 通过、TypeScript 通过；
  Rust 全量 336 passed / 1 failed。
- `cargo test --manifest-path src-tauri/Cargo.toml doc::libreoffice_runtime::tests::docx_to_pdf_end_to_end_when_libreoffice_available -- --nocapture` # failed，二次复现
  `DOC_PREVIEW_WORKER_CRASH`，LibreOffice Worker `signal=6`。
- `npm run governance:generate` # passed，ADUS 生成：258 个 `@GOV` 块、930 个测试用例。
- `npm run governance:audit` # passed，根状态 `passed`，`findings=[]`。
- `git diff --check` # passed。
- `cargo fmt --check --manifest-path src-tauri/Cargo.toml` # blocked；输出显示大量范围外既有格式差异，未为取得绿灯改动无关文件。

stop_conditions:

- 无活动 Stop Condition。原 `SC-04 VALIDATION_BLOCKED` 已于 2026-08-20 由人类明确接受范围外风险解除；
  该失败仍保留为未解决的验证证据，不能表述为完整 Rust 回归通过。

recovery:

- 人类于 2026-08-20 明确接受范围外失败风险；如需关闭该技术债务，扩展到 DOC/LibreOffice 失败根因与其测试，
  再重跑 `npm run test:related -- --files src-tauri/src/ag/task.rs src-tauri/src/lib.rs src/services/agService.ts src/services/agRuntime.test.ts`。

unresolved_items:

- 真实环境 V-S7 尚未执行。
- 下一能力批次仍依序处理 DecisionRequest 模型接线、TaskPlan 执行器、预览确认回执/连续 Diff、按步授权与可观测性。

## 13. 人类风险接受

2026-08-20：人类明确决定“接受范围外失败，继续”。因此 DOCX→PDF LibreOffice Worker 的稳定失败不再阻断
本 Scope 的交付；该失败不属于本批代码通过证据，仍作为范围外技术债务保留。
