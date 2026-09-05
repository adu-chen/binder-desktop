# Issue Trace — AG-EVIDENCE-REPORT-WP5-01

## 1. 问题描述

当前 `report_project` 仍是模型可调用工具，只读/分析回合可被强制要求调用它才能完成；模型提交 readItems、
findings 和 uncertainties，运行时以内存 `ProjectReadEvidenceLedger` 做相等校验。该路径没有把证据绑定到
TurnEvent seq/source_event_id，ToolResult.evidenceIds 始终为空，ProjectReport 仍是模型完成动作而非运行时投影。

## 2. 影响范围

- 成功只读工具执行、ToolResult 事件和 ContextFrozen 事件的证据投影。
- EvidenceLedgerV1 / ModelFinalEnvelopeV2 / ProjectReportV2 共享数据形状。
- 模型 final 校验、ProjectReport 安全事件及前端只读接收投影。
- `report_project` 注册、grant、schema、执行和完成门的删除。

## 3. 相关规则

`BR-AG-OBS-001`、`BR-AG-DATA-008`、`BR-AG-STATE-001/002`、`BR-AG-PERSIST-001`、
`BR-SYS-DATA-008`、`BR-SYS-STATE-007`、`X-SHARED-002`、`X-FORBID-004`。

## 4. 相关链路

`AG-TURN`、`AG-TOOL`、`IPC-CALL`；不新增链路。

## 5. 生产者块

- `turn_runtime.rs` 的 ContextFrozen 事实。
- `tool_exec.rs` 的受限只读工具结果与覆盖投影。
- `turn_events.rs` 的 append-only 事件身份、seq 和安全投影。
- `model_output.rs` 的最终回答与可选 finding→evidenceIds 声明。

## 6. 消费者块

- `turn_tool_loop.rs` 消费成功读取证据并在 Finalizing 派生 ProjectReportV2。
- `ipc/mod.rs` 提供版本化 ToolResult/ProjectReport TurnEvent payload。
- `agService.ts` / `agStore.ts` 只按 seq 接收 ProjectReport，不反向驱动执行。

## 7. 路径上的空规则块

共享结构、hash/去重和 serde 辅助沿用主责块规则；不新增业务规则。旧 ProjectReportV1/report_project 相关块
由 BR-AG-OBS-001 的运行时派生 owner 替代，不保留并列完成真源。

## 8. boundary 假设

- 证据 ID 只能由运行时根据 turn、工具调用、来源和覆盖生成。
- ToolResult 先成为 append-only 事件，证据项再绑定该事件身份；事件回放得到相同 ledger。
- 目录、glob、结构、搜索、选中块与全文只是不同覆盖事实，不代表结论本身。
- ProjectReport 只描述证据覆盖和模型显式关联，不修改最终回答正文。

## 9. 运行时假设

- 同一 turn 的事件 seq 由 SQLite 事务单调分配；安全 ToolResult 和内部 evidence 事件处于同一事实序列。
- AgentReadContext 已保证 workspace 限界和 LogicalState/DiskState 选择；失败结果不进入 ledger。
- 模型在后续 Prompt 的 ToolResult 中可看到运行时生成的 evidenceIds，但不能自定义或提升覆盖。
- Finalizing 可在 Done 前发出 ProjectReport 安全事件；任一事件持久化失败不得伪造 Completed。

## 10. 疑似不变量破坏

- `report_project` 同时存在于注册表、grant、Prompt schema、ToolLoop 分支与终态缺失错误中。
- `ProjectReadEvidenceLedger` 仅在进程内，条目缺 turn_id、through_seq、version_or_hash、source_event_id。
- `ToolResult.evidenceIds` 固定空数组，事件回放无法证明读取覆盖。
- 当前 ProjectReportV1 由模型完整提交，版本和字段均落后于共享 `project-report-v2` 契约。
- 报告失败会触发 `AG_PROJECT_REPORT_REQUIRED`，使已有最终回答被伪装成工具未完成。

## 11. 验证计划

- ContextFrozen、全文、局部块、结构、目录/glob、搜索分别生成正确 coverage 和版本/hash。
- 失败工具、跨 workspace、未知/伪造 evidence ID 不进入 ledger 或 finding。
- 从持久化事件重建 ledger，核对顺序、去重、through_seq、source_event_id。
- final 派生 ProjectReportV2；有效 finding 保留，无效/覆盖不足声明转 uncertainty；普通 final 不需要报告工具。
- 注册表、grant、Prompt、生产 ToolLoop 禁用标识扫描为零。
- 运行实际 changed_files 的 test:plan/test:related、generate、Issue Trace audit 与 diff check。

## Expected Behavior

成功的当前回合读取事件产生可重建 EvidenceLedger；模型最终回答到达时，运行时只使用该账本内截至当前 seq
的证据派生 ProjectReportV2，列出已读、未读、来源和不确定性。模型无需、也不能调用报告完成工具。

## Validation Commands

- `npm run governance:generate`
- `npm run governance:audit -- --issue .governance/issues/AG-EVIDENCE-REPORT-WP5-01.md`
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
  - .governance/issues/AG-EVIDENCE-REPORT-WP5-01.md
  - docs/00_core/A-CORE-C-R-01_ADUS.md
  - src-tauri/src/types/mod.rs
  - src-tauri/src/ipc/mod.rs
  - src-tauri/src/ag/model_output.rs
  - src-tauri/src/ag/prompts.rs
  - src-tauri/src/ag/tool_exec.rs
  - src-tauri/src/ag/tool_registry.rs
  - src-tauri/src/ag/turn_events.rs
  - src-tauri/src/ag/turn_runtime.rs
  - src-tauri/src/ag/turn_tool_loop.rs
  - src/services/agService.ts
  - src/services/agRuntime.test.ts
  - src/store/agStore.ts
forbidden_files:
  - ADU/ADP/AGENTS/APC、需求/设计/规则/协议/验证卡/计划、数据库迁移、cloud/dev-gateway、WP6-WP9 非必要文件
confirmation: YES
scope_reason: 只实现冻结 WP5 的事件证据账本与运行时 ProjectReport，并删除报告完成工具。
```

## 人类决策

- 2026-09-04 已确认按推荐方案完整执行；2026-09-05 继续执行 WP5，`confirmation: YES`。

## Task Runtime Report

```yaml
status: READY
changed_scope:
  - EvidenceLedgerV1 / ProjectReportV2 共享结构与安全事件投影
  - ContextFrozen、成功只读 ToolResult 的持久化证据链及回放
  - Finalizing 运行时报告派生与模型 evidence_id 引用校验
  - report_project 注册、grant、schema、执行与完成门删除
  - 前端按 TurnEvent seq 接收只读 ProjectReport
validation:
  governance_generate: passed
  governance_issue_audit: passed
  git_diff_check: passed
  cargo_fmt_check: passed
  cargo_check_lib_tests: passed
  typescript: passed
  frontend_full: 596/596 passed
  rust_agent: 192/192 passed
  rust_full_sandbox: 404/405 passed; only LibreOffice Worker signal=6（环境限制，非实现断言失败）
  rust_full_host: 405/405 passed; integration core_chain 5/5; sm_broadcast 4/4; doc tests 0/0
  test_plan_same_input: passed; selected frontend full + TypeScript + Rust full + governance audit
  test_related_same_input: passed outside sandbox with explicit human authorization; exit code 0
unresolved_items: []
```
