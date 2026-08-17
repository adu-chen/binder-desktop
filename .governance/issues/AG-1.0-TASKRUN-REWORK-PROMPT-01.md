# Issue Trace — AG-1.0-TASKRUN-REWORK-PROMPT-01：Prompt 修正（三层分离 / 片段内多卡 / Diff 失效）

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-15
前置：AG-1.0-TASKRUN-REWORK-IMP-RUNTIME-01 / IMP-UI-01（阶段 2/3 已落地）

## 1. 用户可见问题

提示词仍把"Diff 接受/拒绝即人类确认"总括化、批次表述诱导跨轮分批（prompts.rs:51、:62-63），与三层分离
契约（A-AG-M-D-01 §5b）不符。修正：DiffCard 只确认内容；任务级裁决走 DecisionRequest；同一执行片段内按
计划连续多 Proposal；同文档 Diff2 基于 Diff1 成功预览后 LogicalState、Diff1 失效。

## 2. 入口点

- `src-tauri/src/ag/prompts.rs`（仅提示词文案）
- `docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md`（变更记录 v2.43）
- 本 Issue Trace、`.governance/active_scope.json`

## 3. 相关规则

- REQ-AG-015、A-AG-M-D-01 §5b（三层分离/片段内连续 Diff/前卡失效）

## 4. 人类确认记录

2026-08-15：人类批准分阶段实现；阶段 4 仅 Prompt 文案，不改运行时。confirmation: YES。

## 5. Task Runtime Report（完成后填写）

- task_type：`CODE_CHANGE`
- final_state：`READY`
- changed_files：`src-tauri/src/ag/prompts.rs`（文案修正）、`A-AG-M-T-01`（v2.43）、本 Issue Trace、active_scope
- validation：`cargo test --lib` # passed（335）；`governance:audit` # passed（exit 0）；`git diff --check` # passed
- unresolved_items：阶段 5（验证）V-S7/手动卡修订 + 真实环境登记。