# Issue Trace — AG-ADAPTIVE-RUNTIME-REQ-01

## 1. 问题描述

Agent 1.0 的能力边界已冻结，但 L1 需求把若干具体治理手段误写成产品事实：关键词式 IntentResult 固定 `allowed_tools`、TaskPlan 预排执行、模型必须调用 `report_project`、同文档多块修改缺少独立裁决粒度。这些表述共同收窄了模型泛化空间，并把运行时观察对象变成执行权威。

## 2. 影响范围

- 产品根的 REQ-CORE-016/017。
- Agent 冻结能力基线与导航索引。
- AG 的工具回合、上下文、运行时、项目分析和复合任务需求。
- SYS 的共享运行时与任务投影需求。
- DE 的 Diff 生成、呈现、载荷与连续 Proposal 需求。

## 3. 相关规则

本 Scope 不新增或改写 RULE；只修改现有 REQ 正文。既有 REQ_MAP 在下一 DESIGN Scope 内按同一 REQ ID 细化规则语义。

## 4. 相关链路

`用户输入 → 模型语义与路线 → 运行时硬边界/动态 CapabilityGrant → 工具/证据 → ProposalBatch/DiffCard → 人类裁决`

## 5. 生产者块

- 模型产出的意图/路线/工具调用请求。
- 运行时采集的 workspace、文档、引用、工具结果和门禁事实。

## 6. 消费者块

- AgentTurn 可见事件、TaskRun 投影、CapabilityGrant、ProjectReport、ProposalBatch 与 DiffCard。

## 7. 路径上的空规则块

无。本阶段不改 L2 映射；现有规则语义迁移由随后 DESIGN Scope 承接。

## 8. boundary 假设

- 模型可以理解、选择和调整路线，但不能授予自身工具、目标或副作用权限。
- 运行时只依据 profile、注册能力、目标/格式、动态能力、DecisionRequest、门禁和预算决定可执行集合。
- 读取结果不自动授予编辑权限；内容修改仍只生成 Proposal。
- 不连续或语义独立的块应可独立裁决；连续且共同表达一个不可分修改的块可以合并。

## 9. 运行时假设

- `agent-edit-1.0` 仍是唯一 1.0 Profile。
- 同一 AgentSession 同时至多一个活动 AgentTurn；等待用户或裁决时不持有模型/工具执行。
- 每次 provider 调用继续经过真实 Gate，取消和首终态锁存继续失败关闭。

## 10. 疑似不变量破坏

- 将语义分类器的输出误当授权，造成泛化不足或错误拒绝。
- 静态计划先于证据冻结路线，导致模型无法根据工具结果调整。
- 要求模型“自报”读取覆盖，报告与真实工具证据可能分叉。
- 多个不连续块合为一张卡，迫使用户一次接受或拒绝互不相干的修改。

## 11. 验证计划

- 全文检索旧权威表述并逐项消解。
- 运行 governance:generate 和带本 Issue 的 governance:audit。
- 运行 test:plan/test:related。
- 运行 git diff --check。

## Expected Behavior

Agent 1.0 在不扩大产品能力和副作用边界的前提下，允许模型基于真实证据自适应理解与调整工作路线；运行时持续实施硬授权与状态约束；用户获得真实进度、证据化报告和可独立裁决的 DiffCard。

## Validation Commands

- `npm run governance:generate`
- `npm run governance:audit -- --issue .governance/issues/AG-ADAPTIVE-RUNTIME-REQ-01.md`
- `npm run test:plan -- --files [...]`
- `npm run test:related -- --files [...]`
- `git diff --check`

## Scope Lock

```yaml
task_type: GOVERNANCE_CHANGE
governance_phase: REQUIREMENT
allowed_files:
  - .governance/active_scope.json
  - .governance/issues/AG-ADAPTIVE-RUNTIME-REQ-01.md
  - docs/00_core/A-CORE-C-D-01_产品概述.md
  - docs/00_core/A-CORE-X-D-01_1.0需求基线与文档体系索引.md
  - docs/00_core/A-CORE-X-D-02_Agent1.0冻结能力基线.md
  - docs/01_modules/AG/A-AG-M-D-01_代理功能需求描述.md
  - docs/01_modules/SYS/A-SYS-M-D-01_系统底座功能需求描述.md
  - docs/01_modules/DE/A-DE-M-D-01_DiffEdit功能需求描述.md
  - docs/00_core/A-CORE-C-R-01_ADUS.md
forbidden_files:
  - 所有 L2 设计、接口、状态机、计划、验证与生产代码
confirmation: YES
scope_reason: 人类已确认推荐方案 C1-C5；本 Scope 仅归一化既有 1.0 需求事实。
```

## 人类决策

- C1：模型主导语义理解、工具选择和路线调整；运行时主导事实、授权、预算、门禁与副作用。
- C2：IntentResult 是可更新语义投影，不是 `allowed_tools` 真源。
- C3：TaskRun/TaskPlan 是复杂任务可观察投影，不是静态调度器。
- C4：ProjectReport 从真实读取证据派生，不依赖模型调用报告工具。
- C5：DiffCard 按可独立裁决的连续块组拆分，同批保留关联。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-09-03 | v1.0 | 建立 Agent 1.0 自适应运行时需求收敛 Scope。 |

## Task Runtime Report

- task_type: GOVERNANCE_CHANGE
- governance_phase: REQUIREMENT
- final_state: READY
- issue_trace: AG-ADAPTIVE-RUNTIME-REQ-01
- scope: explicit / confirmed；9 个 changed_files 均在 Scope Lock 内。
- phase_gate: passed；人类已确认 C1-C5，且本轮只归一既有 1.0 需求，不进入设计或实现。
- mapping_chain: complete；受影响文件中的 REQ 数量修改前后均为 57，未新增 RULE/CHAIN；全库生成仍为 142 REQ、195 REQ_MAP、100 RULE、27 CHAIN。
- test_impact: direct=AG/CBT/CORE/DE/ED/SYS/WS，affected=全模块；选择 frontend-full、rust-full、governance-audit。
- validation:
  - `npm run governance:generate`: passed，ADUS 已按当前需求真源刷新。
  - `npm run governance:audit -- --issue ...`: passed，无治理 finding，Scope Lock 通过。
  - `npm test`: passed，26 files / 595 tests。
  - `cargo test --manifest-path src-tauri/Cargo.toml`（沙箱内）: 373/374，通过项之外仅 LibreOffice 子进程 signal 6。
  - 同一 LibreOffice 测试（沙箱外）: passed，确认失败由 macOS 沙箱执行环境引起。
  - `cargo test --manifest-path src-tauri/Cargo.toml`（沙箱外完整复跑）: passed，374 unit + 5 core_chain + 4 sm_broadcast，0 failed。
  - `git diff --check`: passed。
- stop_conditions:
  - SC-04 曾由沙箱内 LibreOffice worker 崩溃触发；在不修改代码或测试的前提下用相同测试命令于沙箱外完整通过，已解除。
- recovery: 未弱化、跳过或删除测试；通过执行环境对照定位并完成全量复跑。
- unresolved_items: []
