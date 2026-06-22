---
文档编号：CBT-M-S-01
文档状态：A
负责模块：CBT
文档职责：CBT 构建模块级状态机设计——构建回合机（SM-CBT-01），含 APC §6 必备字段；回填 A-SYS-M-S-01 总图 CBT 构建行（planned→active），闭合 Chat↔Build 接管 X-INTER-001 转移点（双侧）
上游约束：A-CBT-M-T-01_构建技术设计.md（CBT-BUILD、BR-CBT-* 规则）、A-CORE-C-P-01_APC.md（§6）、A-SYS-M-S-01_状态机总图.md（X-SEQ-003/X-INTER-001、Chat↔Build 转移点）、A-AG-M-S-01_对话代理状态机设计.md（SM-AG-01/02）、A-DE-M-S-01_对话编辑状态机设计.md（SM-DE-01 裁决）、ADU.md（§3.1）
直接承接：SM-CBT-01 经 rules 字段承载 A-CBT-M-T-01 规则；machine_id 登记总图；接管事件与 SM-AG-01 双向对接
使用边界：本文定义 CBT 状态机内部；不重定义 SM-AG-01/02、SM-DE-01 内部；任务内执行细节委托 AG 基建（嵌套语义同 SM-AG-02 模式）
变更要求：transitions 变更须同步 A-CBT-M-T-01 并重跑测试矩阵；接管转移点变更须与 AG 共同复核；machine_id 一经登记不可更改
治理成熟度：draft
---

# Binder 构建状态机设计（CBT · L2 技术设计）

## §1 文档定位

```
A-CBT-M-T-01（规则 SSOT）→ 本文 SM-CBT-01
本文 machine_id ──登记──→ 总图 §3（CBT 构建行 planned→active）
接管转移点 ──双向──→ SM-AG-01（takeover/fallback + 回执，X-INTER-001 双侧闭合）
```

满足 X-SEQ-003。任务执行段嵌套委托 SM-AG-01/02（同 SM-AG-02 嵌套于回合的模式）。

## §2 SM-CBT-01 · 构建回合机

<!-- STATE_MACHINE
machine_id: SM-CBT-01
owner_module: CBT
status: active
summary: 多文件构建回合生命周期（计划→逐任务执行→产物裁决；可中断无半成品；与 chat 双向接管）
source_type: design
chain: CBT-BUILD
rules: BR-CBT-STATE-001, BR-CBT-STATE-002, BR-CBT-DATA-001, BR-CBT-PERSIST-001
initial_state: Idle
terminal_states: Completed, Interrupted
-->

- **machine_id**：SM-CBT-01　**owner_module**：CBT
- **scope**：一次构建回合（接管/发起→任务计划→逐任务→产物裁决→完成/中断/回落）
- **context**：构建意图、任务计划（BuildTask 序列）、当前任务索引、各任务产物与裁决进度、
  接管来源（chat/直接发起）、中断点
- **states**：Idle、Planning、PlanReview、TaskRunning、TaskPaused、Adjudicating、Completed、Interrupted
- **events**：startBuild、takeoverIn、planReady、planFail、planConfirmed、planAdjusted、taskOk、
  taskFail、resume、allTasksDone、adjudicationDone、interrupt、fallbackOut
- **transitions（主表）**：

| from | event | guard | to | action |
|---|---|---|---|---|
| Idle | startBuild / takeoverIn | takeoverIn 须 SM-AG-01 移交 + 用户确认（X-INTER-001） | Planning | 意图分解（经 AG 基建，过门禁）；回 takeoverAck |
| Planning | planReady | — | PlanReview | 任务计划呈现用户 |
| Planning | planFail | 意图不明 | Idle | 回落 chat 澄清（fallbackOut + fallbackAck） |
| PlanReview | planConfirmed / planAdjusted | 用户确认（可调整后确认） | TaskRunning | 自首任务开始（嵌套 SM-AG-01/02） |
| TaskRunning | taskOk | 有后续任务 | TaskRunning | 产物生成 ChangeProposal 交 DE（SM-DE-01）；下一任务 |
| TaskRunning | allTasksDone | 全任务产出完毕 | Adjudicating | 产物按任务序待裁决（按条基线） |
| TaskRunning | taskFail | — | TaskPaused | 暂停待处置（不静默跳步） |
| TaskPaused | resume | 用户处置（重试/跳过须显式） | TaskRunning | 续行 |
| Adjudicating | adjudicationDone | 用户裁决完毕（含部分接受） | Completed | accepted 任务已经 DE→WS 落盘；控制态闭合 |
| 任意非终态 | interrupt | 用户中断 | Interrupted | 无半成品落盘；未裁决产物按用户选择留存/丢弃；可 fallbackOut 回落 chat |

- **error_states**：TaskPaused（recovery：resume；持续失败可 interrupt）；Planning 失败回 Idle
  （回落 chat）
- **recovery_paths**：TaskPaused→resume；重启后经 BR-CBT-PERSIST-001 恢复至中断点（context
  持久化）或用户明确关闭→Interrupted
- **forbidden_transitions**：
  - Idle → TaskRunning（未经计划与用户确认不得执行）
  - Planning / TaskRunning → Completed（未经裁决不得完成）
  - TaskPaused → TaskRunning 无 resume（失败不得静默续行）
  - 任意态 → 直接写盘（产物必经 SM-DE-01 裁决，X-FORBID-006）
  - 接管中 chat 与构建双活（takeover 后 SM-AG-01 必达移交终结，回执确认前不得双方持有回合）
- **acceptance_criteria**：执行必经用户确认的计划；产物落盘唯一经 DE accepted；中断无半成品；
  重启可恢复或明确关闭（无悬挂）；接管/回落双向回执、无双活双死
- **test_matrix**：见 §4

## §3 跨模块转移契约（X-INTER-001，本文闭合 Chat↔Build 双侧）

| 转移点 | 方向 | 契约 |
|---|---|---|
| 接管 | SM-AG-01 →（takeover + 用户确认）→ SM-CBT-01 Idle→Planning | 上下文随移交传递；CBT 回 takeoverAck 后 AG 侧回合移交终结；无双活 |
| 回落 | SM-CBT-01（planFail/Interrupted/Completed 后）→（fallback）→ SM-AG-01 | chat 续接构建上下文；AG 回 fallbackAck；无双死 |
| 产物裁决 | SM-CBT-01 TaskRunning → SM-DE-01 Generating（逐任务 ChangeProposal） | 复用 AG 产出→DE 交接契约（A-DE-M-S-01 §3），CBT 为另一来源方（origin_module=CBT） |

## §4 测试与验收（强制转移测试矩阵，APC §6/§10）

- **正路**：startBuild→Planning→PlanReview→planConfirmed→TaskRunning（×N）→allTasksDone→
  Adjudicating→adjudicationDone→Completed（部分接受场景断言仅 accepted 落盘）。
- **接管路**：SM-AG-01 takeover（用户确认）→takeoverAck→Planning；planFail→fallback→fallbackAck
  回 chat；断言无双活/双死。
- **失败与中断**：taskFail→TaskPaused→resume；interrupt 在各非终态可达且无半成品落盘；
  重启恢复至中断点/明确关闭。
- **禁止转移**：未确认计划不可执行；未裁决不可 Completed；失败不静默续行；无直接写盘路径。
- 跨模块（G5 联调）：与 AG（接管回执）、DE（逐任务交接）、WS（accepted 落盘）三链。

## §5 总图登记

| 核心链 | machine_id | 总图回填动作 |
|---|---|---|
| CBT 构建 | SM-CBT-01 | §3 行 planned→active |
| Chat↔Build 接管 | — | §5 转移点更新为双侧就绪（事件+回执契约） |

## §6 术语归属说明

BuildRun/BuildTask 已在 A-CBT-M-T-01 §0 注册；ChangeProposal 归 DE、AgentTurn 归 AG，引用不抢注。

## §7 技术债务与盲区（诚实声明）

- 任务并行执行（v1 按序基线）与嵌套取消传播细节延实现期。
- Adjudicating 的批量裁决形态联动 DE-04（待人类）；本机按条裁决语义为基线，批量编排若引入
  须复核本机或 DE 侧扩展。
- TPL 工作流委托进入本机的入口（startBuild 的 TPL 来源）随 TPL G2 对接。

## §8 后续动作

1. 同步回填 A-SYS-M-S-01（§3 CBT 行、§5 Chat↔Build 双侧）。
2. generate 复跑核对；待人类宣布 CBT G2 闭合。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-11 | v1.0 | 初版（CBT G2 状态机件）：定义 SM-CBT-01 构建回合机（Planning/PlanReview/TaskRunning/TaskPaused/Adjudicating，计划须用户确认、失败不静默续行、中断无半成品、重启可恢复无悬挂），含 APC §6 全部必备字段；闭合 Chat↔Build 接管 X-INTER-001 双侧（takeover/fallback + 回执、无双活双死）；产物裁决复用 AG→DE 交接契约（origin=CBT）；满足 X-SEQ-003 |
