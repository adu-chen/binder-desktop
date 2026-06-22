---
文档编号：TPL-M-S-01
文档状态：A
负责模块：TPL
文档职责：TPL 模板/工作流模块级状态机设计——工作流执行机（SM-TPL-01），含 APC §6 必备字段；登记 A-SYS-M-S-01 总图（新增工作流执行行）
上游约束：A-TPL-M-T-01_模板工作流技术设计.md（TPL-FLOW、BR-TPL-STATE-001）、A-CORE-C-P-01_APC.md（§6）、A-SYS-M-S-01_状态机总图.md（X-SEQ-003/X-INTER-001）、A-AG-M-S-01_对话代理状态机设计.md（SM-AG-01 委托）、A-CBT-M-S-01_构建状态机设计.md（SM-CBT-01 委托）、ADU.md（§3.1）
直接承接：SM-TPL-01 经 rules 字段承载 A-TPL-M-T-01 规则；machine_id 登记总图；步骤委托对接 SM-AG-01 / SM-CBT-01
使用边界：本文定义 TPL 状态机内部；步骤内执行归 SM-AG-01/SM-CBT-01（委托不重定义）；模板单次实例化豁免不建机（A-TPL-M-T-01 §1）
变更要求：transitions 变更须同步 A-TPL-M-T-01 并重跑测试矩阵；委托转移点变更须与 AG/CBT 复核；machine_id 一经登记不可更改
治理成熟度：draft
---

# Binder 模板/工作流状态机设计（TPL · L2 技术设计）

## §1 文档定位

```
A-TPL-M-T-01（规则 SSOT：TPL-FLOW）→ 本文 SM-TPL-01 →登记→ 总图（新增「工作流执行」行）
步骤委托 ──→ SM-AG-01（单文档回合）/ SM-CBT-01（多文件构建）（X-INTER-001 委托转移点）
```

满足 X-SEQ-003（A-TPL-M-T-01 §1 评估结论：建机）。

## §2 SM-TPL-01 · 工作流执行机

<!-- STATE_MACHINE
machine_id: SM-TPL-01
owner_module: TPL
status: active
summary: 工作流执行生命周期（按序委托步骤、失败暂停不跳步、确认点强制暂停、中止无半成品、无悬挂）
source_type: design
chain: TPL-FLOW
rules: BR-TPL-STATE-001, BR-TPL-DATA-001
initial_state: Idle
terminal_states: Completed, Aborted
-->

- **machine_id**：SM-TPL-01　**owner_module**：TPL
- **scope**：一次工作流执行（启动→逐步委托→完成/中止）
- **context**：WritingWorkflow 定义、当前步索引、各步进度与产物裁决状态、确认点标记、中断点
- **states**：Idle、AwaitingConfirm、StepRunning、Paused、Completed、Aborted
- **events**：start、confirmStep、stepOk、stepFail、pause、resume、skipConfirmed、abort、allDone
- **transitions（主表）**：

| from | event | guard | to | action |
|---|---|---|---|---|
| Idle | start | 工作流定义有效 | AwaitingConfirm 或 StepRunning | 首步有确认点→AwaitingConfirm；否则直接执行 |
| AwaitingConfirm | confirmStep | 用户确认 | StepRunning | 执行当前步（委托 SM-AG-01/SM-CBT-01） |
| StepRunning | stepOk | 有后续步 | AwaitingConfirm 或 StepRunning | 后续步有确认点→AwaitingConfirm；否则续行 |
| StepRunning | allDone | 末步完成 | Completed | 执行控制态闭合 |
| StepRunning | stepFail | — | Paused | 暂停待处置（不静默跳步） |
| StepRunning | pause | 用户暂停 | Paused | 保存中断点 |
| Paused | resume | 用户选择重试/继续 | StepRunning | 当前步重试或续行 |
| Paused | skipConfirmed | 用户**显式**跳过失败步 | StepRunning 或 Completed | 跳至下一步（显式跳过留痕）；无后续→Completed |
| Paused / AwaitingConfirm / StepRunning | abort | 用户中止 | Aborted | 无半成品落盘（产物均在 diff 路径未裁决即不落） |

- **error_states**：Paused（兼失败暂停与用户暂停，recovery：resume/skipConfirmed/abort）
- **recovery_paths**：Paused→resume；重启后经控制态恢复至中断点（Paused）或用户明确关闭→Aborted
  （无悬挂执行态）
- **forbidden_transitions**：
  - Idle → StepRunning 当首步声明确认点（确认点强制暂停，不得绕过）
  - StepRunning（stepFail 后）→ 下一步 无 skipConfirmed（失败不得静默跳步）
  - AwaitingConfirm → StepRunning 无 confirmStep（确认点不得自动通过）
  - 任意态 → 直接写盘（步骤产物必经 DE diff 路径，X-FORBID-006）
- **acceptance_criteria**：确认点步骤必经用户确认；失败步必经显式处置（重试/显式跳过/中止）；
  中止无半成品落盘；重启可恢复或明确关闭；每步委托通道过对应门禁
- **test_matrix**：见 §3

## §3 测试与验收（强制转移测试矩阵，APC §6/§10）

- **正路**：start→（AwaitingConfirm→confirmStep→）StepRunning×N→allDone→Completed。
- **失败处置路**：stepFail→Paused→resume / skipConfirmed（断言显式跳过留痕）/ abort。
- **确认点**：声明确认点的步骤不可自动通过（forbidden 核验）。
- **中止与恢复**：abort 各态可达且无半成品落盘；执行中重启→恢复 Paused 或明确关闭。
- **委托联调（G5）**：步骤委托 SM-AG-01/SM-CBT-01 的发起与回执、产物经 SM-DE-01 裁决。

## §4 总图登记

| 核心链 | machine_id | 总图回填动作 |
|---|---|---|
| 工作流执行 | SM-TPL-01 | §3 **新增**「工作流执行｜TPL｜SM-TPL-01｜active」行（总图清单非穷举，按 owner G2 暴露登记） |
| 步骤委托 | — | §5 登记 TPL→AG/CBT 委托转移点（发起+回执） |

## §5 术语归属与债务

Template/WritingWorkflow 已在 A-TPL-M-T-01 §0 注册。债务：条件分支扩展（v1 顺序基线）若引入须
扩展本机并回写；步骤委托的取消传播（abort 时通知 SM-AG-01/SM-CBT-01 中断）细节延实现期。

## §6 后续动作

1. 同步回填 A-SYS-M-S-01（§3 新增行 + §5 委托转移点）；generate 复跑核对。
2. 待人类宣布 TPL G2 闭合。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-11 | v1.0 | 初版（TPL G2 状态机件）：定义 SM-TPL-01 工作流执行机（Idle/AwaitingConfirm/StepRunning/Paused，确认点强制暂停、失败显式处置不静默跳步、中止无半成品、重启无悬挂），含 APC §6 全部必备字段；步骤委托 SM-AG-01/SM-CBT-01 转移点登记；满足 X-SEQ-003 |
