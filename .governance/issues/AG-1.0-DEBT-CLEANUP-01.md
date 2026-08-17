# Issue Trace — AG-1.0-DEBT-CLEANUP-01：实现债务批量清理（UNC-22/13）

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-15

## 1. 用户可见症状

未解决项逐步实现第一批：实现债务类中可安全清理项。
- UNC-22：旧 dispatch 内 `ag.start_turn` 桩保留为兼容入口（PI-AG-WIRE-01 留痕）——真实入口是独立
  async `ag_start_turn` + Channel，前端无 dispatch 调用，桩可移除。
- UNC-13：回合历史恢复存在 workspace-only 旧路径与 AgentSession 新路径并存（AG-FUNCTIONAL-AUDIT-01
  留痕，需确认旧函数调用方后退役）。

## 2. 入口点

- `src-tauri/src/lib.rs`（注册行 445、分发 arm 558、route_start_turn 4454-4460）
- `src-tauri/src/ag/history.rs`（旧 persist_turn/recover_turns 与 list_turns 并存——先确认调用方）

## 3. 相关规则

- 无新规则；清理不改变行为（前端唯一入口 ag_start_turn 不变）。

## 4. 相关链路

AG-TURN（回合）；AG-HISTORY（历史恢复）。

## 5. 生产者块

- lib.rs dispatch 信封（ag.start_turn 桩）。

## 6. 消费者块

- 前端 agService.startTurn（invoke ag_start_turn，不依赖 dispatch 桩）。

## 7. 路径上的空规则块

清理项无 @GOV 新增；不触碰行为链路。

## 8. boundary 假设

- UNC-22：前端确认无 dispatch ag.start_turn 调用 → 注册/分发/route 三处移除。
- UNC-13：仅当确认旧路径（persist_turn/recover_turns workspace-only）无生产调用方时退役；否则仅登记。

## 9. 运行时假设

- 移除桩后：dispatch ag.start_turn 返回命令不存在（前端不调用，无影响）。

## 10. 疑似不变量破坏

- 无：前端真实入口不变；历史恢复走 AgentSession 新路径。

## 11. 验证计划

- grep 确认前端/后端无 dispatch ag.start_turn 调用、旧恢复路径无调用方。
- `npm run test:related`（前端全量）+ `cargo test` + `npx tsc --noEmit` + `governance:audit`（全绿）。

## 12. 文件范围结论（Scope Lock 输入）

允许修改：
- `src-tauri/src/lib.rs`
- `src-tauri/src/ag/history.rs`（仅确认后退役旧路径）
- `.governance/issues/AG-1.0-DEBT-CLEANUP-01.md`、`.governance/active_scope.json`
- `docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md`（变更记录，如需）

禁止修改：前端（src/**）；其他后端模块；其他文档；治理控制文档。

## 13. 人类确认记录

2026-08-15：人类"未解决项逐步实现"——本批实现债务清理 UNC-22/13（确认安全后）。confirmation: YES。

## 14. Task Runtime Report

- task_type：`CODE_CHANGE`
- governance_phase：`IMPLEMENTATION`
- final_state：`READY`
- issue_trace：本文件
- changed_files：
  - `src-tauri/src/lib.rs`（UNC-22：移除旧 dispatch `ag.start_turn` 桩——注册行、分发 arm、route_start_turn
    函数（含 BR-SYS-DATA-003-GUARD-SYS-IPC-CALL-004 注释块）；前端唯一入口 `ag_start_turn` + Channel 不变）
  - `docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md`（变更记录 v2.38：UNC-22 移除 + UNC-13 退役结论）
  - `docs/00_core/A-CORE-C-R-01_ADUS.md`（generate 更新：@GOV 258→257，guard -1）
  - 本 Issue Trace、`.governance/active_scope.json`
- mapping_chain：UNC-22 → dispatch 桩移除（前端无调用确认：agService 仅 invoke `ag_start_turn`）；UNC-13
  → 确认 persist_turn/recover_turns 无生产调用方（仅自身测试）→ 文档登记退役（不删代码）。**audit 全绿**。
- test_impact：
  - changed_files：lib.rs、A-AG-M-T-01
  - direct_modules：AG、SYS
  - selection：前端全量（回合测试依赖真实入口）+ tsc + cargo 全量 + audit
- validation：
  - `cargo check` # passed
  - `npm run test:related` # passed（26 files / 576 tests；cargo 333 tests）
  - `npm run governance:generate` # passed（@GOV 258→257）
  - `npm run governance:audit` # **passed（exit 0）**
  - `git diff --check` # passed
- automated_tests：前端 576 + Rust 333 全过。
- stop_conditions：无。
- unresolved_items：UNC-13 退役代码保留（persist_turn/recover_turns 无生产调用方，防误用）；UNC-15
  AgentTrace、UNC-45 cargo fmt 等另批处理。
