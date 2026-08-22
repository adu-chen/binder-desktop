---
文档编号：WS-M-S-02
文档状态：A
负责模块：WS
文档职责：WorkspaceOperation 与 WorkspaceTimeline 资源还原状态机设计
上游约束：A-WS-M-T-02（WS-OPERATION、BR-WS-STATE-003/BR-WS-DATA-003/BR-WS-OBS-002/BR-WS-PERSIST-003）、A-AG-M-S-01（SM-AG-04）、A-SYS-M-S-01
直接承接：WS 资源操作执行器、DecisionRequest 接线、统一时间轴还原器及状态转移测试
使用边界：不定义内容 Diff 状态机；不允许 TaskRun 或前端绕过本机直接执行资源副作用
变更要求：状态、事件、guard、补偿或终态变化须同步 A-WS-M-T-02、A-SYS-M-S-01 和验证矩阵
治理成熟度：draft
---

# 工作区资源操作状态机设计

## 1. SM-WS-04 · WorkspaceOperation

<!-- STATE_MACHINE
machine_id: SM-WS-04
owner_module: WS
status: active
summary: WorkspaceOperationPlan 从预检、前置裁决、确认时再核验、直接磁盘提交到补偿和时间轴记录的生命周期
source_type: design
chain: WS-OPERATION
rules: BR-WS-STATE-003, BR-WS-DATA-003, BR-WS-OBS-002, BR-WS-PERSIST-003
initial_state: Planning
terminal_states: Completed, Rejected, Expired, FailedCompensated, PartialFailed
-->

- **context**：plan_id、operation_id（开始执行后生成）、idempotency_key、WorkspaceOperationPlan、DecisionRequest 身份、逐项执行回执、补偿栈、WorkspaceOperationEntry。
- **states**：Planning、WaitingConfirm、Revalidating、Executing、Compensating、Completed、Rejected、Expired、FailedCompensated、PartialFailed、Interrupted。
- **events**：planReady、planFail、userConfirm、userReject、baselineValid、baselineDrift、itemOk、itemFail、allItemsOk、compensationOk、compensationFail、interrupt、explicitRecover。

| from | event | guard | to | action |
|---|---|---|---|---|
| Planning | planReady | 计划完整、路径守界、影响可展示 | WaitingConfirm | 持久化计划并提出 DecisionRequest |
| WaitingConfirm | userReject | request_id/plan_id 匹配 | Rejected | 不写盘，记录拒绝 |
| WaitingConfirm | userConfirm | request_id/plan_id 匹配且未重复裁决 | Revalidating | 冻结授权并重新读取磁盘基线 |
| Revalidating | baselineDrift | 任一目标变化 | Expired | 不写盘，要求重新生成计划 |
| Revalidating | baselineValid | 全部基线一致 | Executing | 按计划顺序执行 |
| Executing | itemOk | — | Executing | 写逐项回执并压入补偿栈 |
| Executing | allItemsOk | 所有项完成 | Completed | 写 WorkspaceOperationEntry、刷新消费者 |
| Executing | itemFail | — | Compensating | 停止后续项，逆序补偿 |
| Compensating | compensationOk | 全部已完成项补偿成功 | FailedCompensated | 写失败与补偿回执 |
| Compensating | compensationFail | 任一项无法补偿 | PartialFailed | 保留真实磁盘状态、快照和逐项回执 |
| Revalidating/Executing/Compensating | interrupt | — | Interrupted | 不自动续跑，持久化当前回执 |
| Interrupted | explicitRecover | 用户显式选择恢复 | Planning | 基于真实磁盘与既有回执生成新的修复/重试计划、新 plan_id 和新 DecisionRequest；不复用旧确认 |

- **forbidden_transitions**：WaitingConfirm→Executing 无 userConfirm；userConfirm 后跳过 Revalidating；Rejected/Expired→Executing；失败后继续后续项；补偿失败却标 Completed；重复确认再次执行；应用重启自动从 Interrupted 进入 Executing。
- **recovery_paths**：Expired→重新规划产生新 plan_id；FailedCompensated→用户可重新发起；PartialFailed→WorkspaceTimeline 人工恢复；Interrupted→显式恢复→Planning→新 DecisionRequest。
- **acceptance_criteria**：未确认和基线漂移零副作用；确认仅执行一次；批量失败真实补偿；所有终态都有可见事件与历史回执。

## 2. CreateDocumentBundle 子链

绑定初始 Diff 不另建资源状态机，而作为 SM-WS-04 `create_document` 成功后的受控后置动作：

```text
Completed(create empty file)
→ ED 建立 LogicalState
→ DE 投影 initial_content_ref
→ InitialDiff pending
```

InitialDiff 正常接受归 SM-DE-01；拒绝只有在 `creationCompensationEligible` guard 成立时，向 SM-WS-04 发起同
operation_id 的补偿删除事件。guard 不成立时 Diff 失效，不得删除磁盘资源。投影失败也走相同补偿 guard。

## 3. WorkspaceTimeline 资源还原

资源还原复用 SM-WS-04：读取 WorkspaceOperationEntry 后构造反向 WorkspaceOperationPlan，仍须产生新的
DecisionRequest，经确认和再核验后执行，并追加新 WorkspaceOperationEntry。现有 SM-WS-03 继续只处理
DocumentTimeline 内容快照；统一 UI 根据 entry_type 路由 SM-WS-03 或 SM-WS-04。

## 4. 与 SM-AG-04 的跨模块转移

```text
SM-WS-04.WaitingConfirm
→ DecisionRequest proposed
→ SM-AG-04.WaitingConfirm
→ confirm
→ SM-AG-04.Executing + SM-WS-04.Revalidating
→ OperationOutcome
→ SM-AG-04 Completed/Failed/Interrupted
```

资源操作不产生 DiffCard 事件；只有 CreateDocumentBundle 的绑定初始内容在资源创建成功后进入 SM-DE-01。

## 5. 测试矩阵

- 状态主路：Planning→WaitingConfirm→Revalidating→Executing→Completed。
- 拒绝/漂移：WaitingConfirm→Rejected；Revalidating→Expired，断言零磁盘副作用。
- 补偿：第二项失败→Compensating→FailedCompensated；补偿失败→PartialFailed。
- 幂等：重复 confirm/operation_id 不重复执行。
- 中断：三个活动状态→Interrupted，重启不续跑；显式恢复必经再核验。
- CreateDocumentBundle：初始 Diff 拒绝 guard 成功删除；任一基线推进 guard 失败并使卡失效。
- 时间轴：内容节点路由 SM-WS-03，资源节点路由 SM-WS-04；资源还原追加新节点不改旧历史。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-21 | v1.0 | 建立 SM-WS-04，固定前置裁决、确认时再核验、直接写盘、fail-fast 补偿、CreateDocumentBundle 和统一时间轴资源还原。 |
