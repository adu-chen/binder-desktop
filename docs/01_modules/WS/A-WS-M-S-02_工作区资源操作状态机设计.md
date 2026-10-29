---
文档编号：WS-M-S-02
文档状态：A
负责模块：WS
文档职责：WorkspaceOperation 与 WorkspaceTimeline 资源还原状态机设计
上游约束：A-WS-M-T-02（WS-OPERATION、BR-WS-STATE-003/BR-WS-DATA-003/BR-WS-OBS-002/BR-WS-PERSIST-003）、A-AG-M-S-01（SM-AG-01、SM-AG-04 投影）、A-SYS-M-S-01
直接承接：WS 资源操作执行器、DecisionRequest 接线、统一时间轴还原器及状态转移测试
使用边界：不定义内容 Diff 状态机；不允许模型、AgentTurn 外层调用方、TaskRun 投影或前端绕过本机直接执行资源副作用
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
- **states**：Planning、WaitingConfirm、Revalidating、Executing、AwaitingContent、Compensating、Completed、Rejected、Expired、FailedCompensated、PartialFailed、Interrupted。
- **events**：planReady、planFail、userConfirm、userReject、baselineValid、baselineDrift、itemOk、itemFail、contentRequired、contentSaved、allItemsOk、compensationOk、compensationFail、interrupt、explicitRecover。

| from | event | guard | to | action |
|---|---|---|---|---|
| Planning | planReady | 计划完整、路径守界、影响可展示 | WaitingConfirm | 持久化计划并提出 DecisionRequest |
| WaitingConfirm | userReject | request_id/plan_id 匹配 | Rejected | 不写盘，记录拒绝 |
| WaitingConfirm | userConfirm | request_id/plan_id 匹配且未重复裁决 | Revalidating | 冻结授权并重新读取磁盘基线 |
| Revalidating | baselineDrift | 任一目标变化 | Expired | 不写盘，要求重新生成计划 |
| Revalidating | baselineValid | 全部基线一致 | Executing | 按计划顺序执行 |
| Executing | itemOk | — | Executing | 写逐项回执并压入补偿栈 |
| Executing | contentRequired | 绑定一份初始内容；创建阶段成功（包括单独创建） | AwaitingContent | 持久化当前回执，等待真实保存并暂停剩余项；不得把空文件创建解释为内容已完成 |
| AwaitingContent | contentSaved | 真实保存事实、原确认/回合仍有效、单次领取 | Executing | 在同一事务领取阶段；重验保存后新文件及原有源资源基线，仅执行剩余项 |
| AwaitingContent | itemFail | 打开/投影/预览失败 | Compensating | 基线匹配才补偿；旧组同胞先全量预检，逐项记录；新目录最后且仅空目录清理 |
| Executing | allItemsOk | 所有项完成 | Completed | 写 WorkspaceOperationEntry、刷新消费者 |
| Executing | itemFail | — | Compensating | 停止后续项，逆序补偿 |
| Compensating | compensationOk | 全部已完成项补偿成功 | FailedCompensated | 写失败与补偿回执 |
| Compensating | compensationFail | 任一项无法补偿 | PartialFailed | 保留真实磁盘状态、快照和逐项回执 |
| Revalidating/Executing/AwaitingContent/Compensating | interrupt | — | Interrupted | 不自动续跑，持久化当前回执；已有创建历史不覆盖，缓存回执同步中断 |
| Interrupted | explicitRecover | 用户显式选择恢复 | Planning | 基于真实磁盘与既有回执生成新的修复/重试计划、新 plan_id 和新 DecisionRequest；不复用旧确认 |

- **forbidden_transitions**：WaitingConfirm→Executing 无 userConfirm；userConfirm 后跳过 Revalidating；Rejected/Expired→Executing；失败后继续后续项；补偿失败却标 Completed；重复确认再次执行；应用重启自动从 Interrupted 进入 Executing。
- **recovery_paths**：Expired→重新规划产生新 plan_id；FailedCompensated→用户可重新发起；PartialFailed→WorkspaceTimeline 人工恢复；Interrupted→显式恢复→Planning→新 DecisionRequest。
- **acceptance_criteria**：未确认和基线漂移零副作用；确认仅执行一次；批量失败真实补偿；所有终态都有可见事件与历史回执。

### 1.1 保存依赖的组合执行（2026-10-24人类批准）

含绑定初始内容的计划（包括单独创建），在创建资源成功后进入非终态AwaitingContent，保留相同operation_id和逐项进度；后续非创建项尚未执行。初始Diff生成、ack、接受均不解除依赖；用户真实保存后，以原冻结决策/目标/基线和仍未取消的原AgentTurn重新核验，只执行未执行项。重复确认返回原等待/终态回执；重启仅Interrupted，不自动执行；保存失败或外部变化不得删除旧资源。

组合投影/打开/预览失败，结算AwaitingContent及原AgentTurn。补偿前按创建后基线检查全部相关资源；旧版本已执行的同组兄弟操作也须反向恢复，不能只删除新空文件后宣称整组回滚。基线漂移不覆盖用户更改，结果为PartialFailed且列明剩余效果。Completed资源历史保留；后续补偿是新条目，整组当前结果由最新结算回执表示。

## 2. CreateDocumentBundle 子链

绑定初始 Diff 不另建资源状态机，而作为 SM-WS-04 `create_document` 成功后的受控后置动作：

```text
Completed(无绑定初始内容) / AwaitingContent(含初始内容，含单独创建)
→ ED 建立 LogicalState
→ DE 投影 initial_content_ref
→ InitialDiff pending
→ 真实预览ack → 用户接受 → 用户保存
→ 保存事实与原冻结计划重验 → 仅执行尚未执行的依赖项
```

InitialDiff 正常接受归 SM-DE-01；拒绝只有在 `creationCompensationEligible` guard 成立时，向 SM-WS-04 发起同
operation_id 的补偿删除事件。guard 不成立时 Diff 失效，不得删除磁盘资源。投影失败也走相同补偿 guard。

## 3. WorkspaceTimeline 资源还原

资源还原复用 SM-WS-04：读取 WorkspaceOperationEntry 后构造反向 WorkspaceOperationPlan，仍须产生新的
DecisionRequest，经确认和再核验后执行，并追加新 WorkspaceOperationEntry。现有 SM-WS-03 继续只处理
DocumentTimeline 内容快照；统一 UI 根据 entry_type 路由 SM-WS-03 或 SM-WS-04。

## 4. 与 SM-AG-01 / SM-AG-04 的跨模块转移

```text
SM-WS-04.WaitingConfirm
→ DecisionRequest proposed
→ SM-AG-01.AwaitingDecision；SM-AG-04（若存在）从事件投影 Waiting
→ confirm
→ SM-AG-01 以当前终态/取消状态、确认回执和冻结 plan 重算 permitted_effect_paths，由运行时调用 SM-WS-04.Revalidating
→ OperationOutcome
→ SM-AG-01 追加 ToolResult/终态事件；SM-AG-04 仅据事件投影 Active/Completed/Failed/Interrupted
```

SM-AG-04 不触发状态转移、调用工具或恢复授权。资源操作不产生 DiffCard 事件；只有 CreateDocumentBundle 的绑定
初始内容在资源创建成功后进入 SM-DE-01。

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
| 2026-10-24 | v1.4 | 带初始内容的单独创建同样进入AwaitingContent，预览/接受不解锁；保存后只经原裁决重验，不新增预览执行分支。 |
| 2026-10-24 | v1.3 | AG-WORKFLOW-COMPLETION-01：保存依赖、原回合身份检查点、旧组合整组补偿及失败/拒绝交互收尾；不自动接受、保存或重放。 |
| 2026-08-21 | v1.0 | 建立 SM-WS-04，固定前置裁决、确认时再核验、直接写盘、fail-fast 补偿、CreateDocumentBundle 和统一时间轴资源还原。 |
| 2026-09-03 | v1.1 | AG-ADAPTIVE-RUNTIME-DESIGN-01：DecisionRequest 等待与恢复归 SM-AG-01；SM-AG-04 仅从 TurnEvent 派生任务可观察状态，不再与 WS 执行器联动调度。 |
