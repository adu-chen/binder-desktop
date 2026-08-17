# Issue Trace — AG-1.0-IMP-CONSECUTIVE-PROPOSAL-01：连续 Proposal 前卡失效（R5-B）实现

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-14

## 1. 用户可见症状

冻结基线 R5-B（REQ-DE-009 / BR-SYS-STATE-008）要求：同一 TaskRun 的连续 Proposal 必须以最新
LogicalState 接力；后续 Proposal 成功预览推进基准后，所有基于旧基准的前卡失效（document_revision_advanced）、
保留历史且不可再裁决；不构成补丁栈、无任务级回滚。当前实现：保存时（`acceptAppliedPendingDiffsForSave`）
与 open_session 自身 base 校验失败时已实现失效，但**后续 Proposal 生成/预览成功时不会主动失效同文档
旧 pending 卡**——旧卡仍可裁决（可能基于已推进的旧基准，导致错误内容或覆盖新基准改动）。

## 2. 入口点

- `src/services/agService.ts` `diff_proposed` 事件处理（:1167-1222）：新 Proposal 加入 deStore 后，未失效
  同文档旧 pending 卡。
- `src/services/deService.ts` `applyOpenSessionDiff`（:165-208）：open_session 预览成功推进 session 版本后，
  未失效同文档旧 pending 卡。
- 既有失效原语：`deService.expireDiff`（:96-103，`de.expire_diff` + `deStore.expireLocal`，
  `ExpiredReason.document_revision_advanced`）。

## 3. 相关规则

- `BR-SYS-STATE-008`（TaskRun 编排与 Proposal 接力：连续 Proposal 以最新 LogicalState 为基准、前卡基准
  推进失效、非补丁栈/任务级回滚；D1 注册）
- 间接：BR-CORE-STATE-001（已有 LogicalState 预览/保存裁决）、BR-DE-STATE-002（过期语义）

## 4. 相关链路

DE-DIFF（Proposal 链）、ED-EDIT（LogicalState 承接）、AG-TURN（回合产出）。

## 5. 生产者块

- `deService.ts` 既有 @GOV：BR-CORE-STATE-001-EFFECT-DE-ED-EDIT-001（applyOpenSessionDiff）、
  BR-CORE-STATE-003-EFFECT-DE-ED-EDIT-001（promotePendingDiffOnOpen）等。
- 本实现新增 @GOV：BR-SYS-STATE-008-EFFECT-DE-DE-DIFF-001（expireStalePendingForPath）。

## 6. 消费者块

- DiffCard 呈现（旧卡显示"原文已被手动编辑"失效态，接受/拒绝禁用）。
- 保存链（失效卡不被误裁决）。
- 连续 Proposal 接力（新卡以最新 LogicalState 生成）。

## 7. 路径上的空规则块

deService.ts / agService.ts 已有 @GOV 标注；新增函数补 @GOV 引用 BR-SYS-STATE-008。

## 8. boundary 假设

- 失效条件：同文档（relPath 相同）、state==="pending"、diffId != 新卡、base 版本落后或内容哈希不同。
- 并行同基准卡（同一 base 版本 + 哈希）不受影响（不构成基准推进）。
- background pending 卡无 LogicalState 基准，不参与失效（也不能作为续编输入）。
- 前卡失效保留历史（不可恢复裁决），不产生任务级回滚、不触碰文件内容。

## 9. 运行时假设

- `diff_proposed`（新卡生成）与 `applyOpenSessionDiff`（open_session 预览成功）为两个触发点。
- 失效调用异步、失败隔离（沿用 expireDiff 的 catch 语义），不影响新卡流程。

## 10. 疑似不变量破坏

- BR-CORE-STATE-001~003 三分流语义不变（失效只在基准推进时触发，不改变裁决路径）。
- 保存链"保存有效 pending 先裁决"不变（已失效卡不再被保存裁决）。
- 不触碰磁盘内容（失效不落盘）。

## 11. 验证计划

- `deRuntime.test.ts` 补连续 Proposal 前卡失效测试（新卡生成/预览推进后旧卡 expired + 不可裁决）。
- 运行 `npm run test:plan -- --files [changed_files]` 审阅影响，再 `npm run test:related` 执行；
  `npx tsc --noEmit`；`cargo test`；`npm run governance:audit`。
- 同步 `scripts/test_governance.test.mjs` 前端测试计数基线（新增用例导致 DE/AG 计数变化）。

## 12. 文件范围结论（Scope Lock 输入）

允许修改：
- `src/services/deService.ts`（expireStalePendingForPath + applyOpenSessionDiff 触发）
- `src/services/agService.ts`（diff_proposed 触发 + import）
- `src/services/deRuntime.test.ts`（补测试）
- `src/services/agRuntime.test.ts`（如需）
- `scripts/test_governance.test.mjs`（测试计数基线调整）
- `docs/01_modules/DE/A-DE-M-S-01_DiffEdit状态机设计.md`（变更记录注明落地）
- `.governance/issues/AG-1.0-IMP-CONSECUTIVE-PROPOSAL-01.md`、`.governance/active_scope.json`

禁止修改：工作区其他未提交改动；其余 src/src-tauri 文件；其他模块文档；治理控制文档；governance.config.json。

## 13. 人类确认记录

2026-08-14：人类经 AskUserQuestion 确认本 Scope（连续 Proposal 前卡失效）作为下一个实现 Scope；
基于侦察事实（AST 自动写入无生产调用点）排除 R5-C 项。confirmation: YES。
同类测试计数基线调整（DE 69→73）沿用本会话已批准模式（APC §10 基线维护留痕）。

## 14. Task Runtime Report

- task_type：`CODE_CHANGE`
- governance_phase：`IMPLEMENTATION`
- final_state：`READY`
- issue_trace：本文件
- changed_files：
  - `src/services/deService.ts`（新增 @GOV BR-SYS-STATE-008-EFFECT-DE-DE-DIFF-001 的
    `expireStalePendingForPath`：同文档旧 pending 卡按 base 版本/哈希比较失效；applyOpenSessionDiff
    成功预览后触发）
  - `src/services/agService.ts`（diff_proposed 新卡加入后触发 expireStalePendingForPath）
  - `src/services/deRuntime.test.ts`（连续 Proposal 前卡失效 4 用例）
  - `scripts/test_governance.test.mjs`（DE 前端测试计数基线 69→73，留痕）
  - `docs/01_modules/DE/A-DE-M-S-01_DiffEdit状态机设计.md`（§2.1 契约落地变更记录 v1.12）
  - `docs/00_core/A-CORE-C-R-01_ADUS.md`（generate 更新：@GOV 253→254、测试用例 898→902）
  - 本 Issue Trace、`.governance/active_scope.json`
- mapping_chain：BR-SYS-STATE-008（D1 注册）→ deService.ts @GOV + deRuntime.test.ts covers 引用，
  覆盖缺口从 2 减到 1（仅剩 BR-SYS-DATA-007 未实现部分）。
- test_impact：
  - changed_files：deService.ts、agService.ts、deRuntime.test.ts
  - direct_modules：AG、DE、ED、WS、SYS、CORE
  - selection：test:plan 判定前端全量 + tsc + cargo 全量 + audit（升级全量）
- validation：
  - `npm run test:related`（npm test 前端全量） # passed：26 files / 572 tests（含连续 Proposal 4 用例；
    test_governance 19 过；deRuntime 41 过）
  - `npx tsc --noEmit` # passed
  - `cargo test`（Rust 全量） # passed：330 tests
  - `npm run governance:generate` # passed（@GOV 253→254；测试用例 898→902；BR-SYS-STATE-008 进入 covered）
  - `npm run governance:audit` # failed（2 项，较澄清 Scope 的 2 项收敛，仅剩 BR-SYS-DATA-007 一条规则
    无代码 @GOV 引用/测试 covers——未实现部分，预期）；其余维度全部 passed
  - `git diff --check` # passed
- automated_tests：前端 572 + Rust 330 全过。
- stop_conditions：无。
- unresolved_items：BR-SYS-DATA-007（外部内容/记忆隔离）为下一个实现 Scope 的债务；跨 TaskRun/回合的
  关联字段（task_run_id）由 D3 后续实现补充；diff_proposed 立即失效 vs 预览成功失效的时序按保守语义取
  立即失效，后续可按产品体验调整。
