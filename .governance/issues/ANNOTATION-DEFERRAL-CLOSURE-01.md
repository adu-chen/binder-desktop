# Issue Trace · ANNOTATION-DEFERRAL-CLOSURE-01

## 1. 用户可见症状

`npm run governance:audit` 对 5 条已到复核期的脚手架标注延期记录报
`ANNOTATION_DEFERRAL_STALE`。产品运行不受影响，但治理审计不能通过。

## 2. 入口点

- `src/shell/App.tsx`：活动 workspace 恢复后的外壳切换。
- `src/shell/Welcome.tsx`：打开、切换和最近 workspace 的入口编排。
- `src/shell/Workbench.tsx`：活动 workspace 绑定的主工作台投影。
- `src/store/workspaceShell.ts`：后端 workspace 控制态的前端投影。
- `src/services/workspaceService.ts`：打开 workspace 后的 IPC 与下游会话重置/恢复。

## 3. 相关规则与链路

- `BR-WS-PERSIST-002`：workspace 控制态持久化、单活动态与重启恢复。
- `X-FORBID-007`：前端只投影后端 workspace 控制态，不另立权威业务状态。
- 链路：`DATA-MIGRATE`。

## 4. 处置

五个符号现已具备稳定的控制态投影或协调职责；以既有 `BR-WS-PERSIST-002` 补充精确 `@GOV`，并关闭
对应延期记录。不得新建业务规则、改变运行时逻辑、或仅延后复核日期。

## Scope Lock

```yaml
task_type: CODE_CHANGE
governance_phase: VALIDATION
allowed_files:
  - .governance/issues/ANNOTATION-DEFERRAL-CLOSURE-01.md
  - docs/00_core/A-CORE-X-R-02_脚手架占位_标注延期清单.md
  - docs/00_core/A-CORE-C-R-01_ADUS.md
  - src/shell/App.tsx
  - src/shell/Welcome.tsx
  - src/shell/Workbench.tsx
  - src/store/workspaceShell.ts
  - src/services/workspaceService.ts
forbidden_files:
  - .governance/active_scope.json
  - ADU.md
  - ADP.md
  - AGENTS.md
  - CLAUDE.md
  - governance.config.json
  - package.json
  - src-tauri/**
confirmation: YES
scope_reason: 清退到期的脚手架标注延期并补既有 workspace 控制态规则的代码映射；不改变功能。
```

## 验证计划

- `npm run governance:generate`
- `npm run test:plan -- --files [changed_files]`
- `npm run test:related -- --files [changed_files]`
- `npm run governance:audit`
- `git diff --check`

## 实施结果

- App、Welcome、Workbench 分别补充活动 workspace 恢复、入口协调和工作台绑定的 `RB` 边界。
- workspaceShell 补充后端 workspace 控制态的内存投影 `EFFECT`；openWorkspace 补充目录选择到
  `ws.open_workspace` IPC 及下游会话恢复的 `EFFECT`。
- 五条延期记录均改为已解决；没有仅推迟复核日期，没有新增静态规则，也没有改变运行时逻辑。
- ADUS 已由 `npm run governance:generate` 刷新；当前登记的标注延期均为已解决记录。

## Task Runtime Report

```yaml
task_id: ANNOTATION-DEFERRAL-CLOSURE-01
task_type: CODE_CHANGE
governance_phase: VALIDATION
final_state: READY
scope:
  allowed_files: 8
  outside_scope_changes: 0
validation:
  governance_generate: PASS (220 @GOV blocks; 5 resolved annotation-deferral records)
  test_plan: PASS (CORE/SYS/WS; conservative full frontend and Rust selection)
  related_frontend: PASS (21 files; 460 tests)
  typescript: PASS (npx tsc --noEmit)
  related_rust: ACCEPTED_RISK (262/263; DOC LibreOffice sandbox E2E signal=6, recorded in DOC-LO-SANDBOX-VALIDATION-01)
  governance_audit: PASS (0 governance findings)
  diff_check: PASS
unresolved_items:
  - DOC LibreOffice release-state validation remains governed by DOC-LO-SANDBOX-VALIDATION-01 and BLOCK-REL-LO-001; it was not changed by this task.
```

人类于 2026-08-01 确认闭合本次治理：五条 `ANNOTATION_DEFERRAL_STALE` 已清退，以上运行报告为本任务的最终记录。
