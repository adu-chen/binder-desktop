---
文档编号：WS-M-T-02
文档状态：A
负责模块：WS
文档职责：工作区资源操作与统一工作区时间轴的 L2 技术设计（规则 SSOT）
上游约束：REQ-CORE-010/012/014、REQ-AG-008/011/013/015、REQ-WS-006/008/010、BR-SYS-STATE-008、X-FORBID-004
直接承接：A-WS-M-S-02、A-WS-I-P-01、A-SYS-I-P-01、A-AG-M-T-01、A-VAL-X-V-04 及工作区资源操作实现
使用边界：本文只定义创建、重命名、移动、删除与恢复等工作区资源副作用；文档正文、样式、表格和图片修改仍归 DE 内容 Diff；TaskRun 只编排，不拥有资源状态
变更要求：操作种类、前置裁决、批量失败策略、创建文档绑定语义或时间轴还原语义变化，必须同步状态机、接口、共享结构和验证矩阵
治理成熟度：draft
---

# Binder 工作区资源操作技术设计

## 0. Terminology Registry

<!-- TERM
term_id: TERM-WS-003
owner_module: WS
zh: 工作区资源操作
en: WorkspaceOperation
definition: 对 workspace 内文件或文件夹的创建、重命名、移动、删除或恢复；操作在用户前置裁决后直接改变 DiskState，不使用内容 DiffCard
forbidden: 文件整理,文件操作diff,资源diff,结构diff,文件树diff
chains: WS-OPERATION, AG-TURN
-->

<!-- TERM
term_id: TERM-WS-004
owner_module: WS
zh: 工作区资源操作计划
en: WorkspaceOperationPlan
definition: 执行前冻结的资源操作清单，包含目标、顺序、影响范围、基线、确认文案和失败策略；DecisionRequest 只裁决该不可变计划
forbidden: 删除diff,操作草稿,文件修改建议,结构Proposal
chains: WS-OPERATION, AG-TURN
-->

<!-- TERM
term_id: TERM-WS-005
owner_module: WS
zh: 工作区时间轴
en: WorkspaceTimeline
definition: 统一呈现文档内容版本与工作区资源操作历史的时间轴；内容节点继续使用 DocumentTimeline 语义，资源节点使用 WorkspaceOperationEntry 及类型化还原执行器
forbidden: 回收站,撤销栈,文件历史栈,项目版本栈
chains: WS-TIMELINE, WS-OPERATION
-->

<!-- TERM
term_id: TERM-WS-006
owner_module: WS
zh: 工作区资源操作条目
en: WorkspaceOperationEntry
definition: WorkspaceTimeline 中记录一次已提交资源操作、执行回执、前后路径、资源清单、快照引用及可逆性的历史条目
forbidden: 删除节点,文件diff节点,操作日志行
chains: WS-TIMELINE, WS-OPERATION
-->

<!-- TERM
term_id: TERM-WS-007
owner_module: WS
zh: 新建文档绑定
en: CreateDocumentBundle
definition: 同一 operation_id 下绑定“前置裁决后创建磁盘空文件”与“创建后投影的初始内容 Diff”的组合；只有绑定初始 Diff 且资源基线未变化时，拒绝才可补偿删除本次创建文件
forbidden: 新建diff,空文件diff,创建提案
chains: WS-OPERATION, DE-DIFF, AG-TURN
-->

## 1. 架构边界

系统只保留两种写入模型：

1. **内容写入**：模型生成内容 Diff，DE/ED 预览先改变 LogicalState，用户后置裁决；磁盘写入仍遵守既有三态与保存规则。
2. **工作区资源写入**：系统先生成 WorkspaceOperationPlan，由 DecisionRequest 前置裁决；确认后 WS 重新核验并直接改变 DiskState，不生成资源 DiffCard，也不再二次接受。

TaskRun 只持有 `operation_id`、`request_id`、步骤状态和执行结果，不解析路径迁移、快照或时间轴还原细节。
DecisionRequest 是裁决身份和恢复游标；其 UI 可直接呈现 WorkspaceOperationPlan，但不得复制另一张“资源 Proposal 卡”。

## 2. WorkspaceOperationPlan

共享语义字段：

| 字段 | 语义 |
|---|---|
| plan_id | 不可变计划身份；DecisionRequest 的裁决对象 |
| operation_id | 确认并开始执行后生成的一次执行身份 |
| idempotency_key | 重复 confirm/execute 的幂等键 |
| workspace_id | 唯一工作区边界 |
| operations | 有序 create/rename/move/delete/restore 项 |
| resource_manifest | 文件类型、源/目标路径、递归成员及数量 |
| baseline | 确认卡生成时的存在性、类型、大小、修改时点与内容哈希/目录清单哈希 |
| consequences | 标签、内容 Diff、引用与 BDocx 身份的影响摘要 |
| failure_policy | 固定为 fail_fast_compensate |
| initial_content_ref | 仅 CreateDocumentBundle：确认前已生成的初始内容载荷引用 |
| status | planning/waiting_confirm/revalidating/executing/compensating/completed/rejected/expired/failed_compensated/partial_failed/interrupted |

计划生成后不可原地修改；用户确认前目标或基线发生变化时，旧计划和 DecisionRequest 一并过期，重新生成新 plan_id。

## 3. 前置裁决与执行链

<!-- CHAIN
chain_id: WS-OPERATION
主责模块: WS
status: active
risk_tier: P1
integration_test: src-tauri/src/workspace/operation.rs::plan_freezes_real_baseline_and_execute_requires_bound_resolved_decision
-->

```text
IntentResult（Agent 意图）
→ 只读预检并构造 WorkspaceOperationPlan
→ DecisionRequest(proposed)
→ 用户 reject：计划 rejected，不写盘
→ 用户 confirm：确认时重新核验完整 baseline
→ 不一致：计划 expired，不写盘
→ 一致：WS 按顺序执行 DiskState 副作用
→ 每项写执行回执；失败即停并反向补偿已完成项
→ 成功或真实 partial_failed 写 WorkspaceOperationEntry
→ 更新文件树、标签、引用、BDocx 身份和 TaskRun 可见事件
```

> 用户文件树动作不进入 WS-OPERATION 链（2026-08-22，WS-RESOURCE-WRITE-GUARD-01）：人工直调
> `ws.create_doc/create_folder/rename_doc/delete_doc/delete_folder` 保持直写，但删除/重命名必须遵守
> BR-WS-STATE-009 安全底线（dirty 阻断、快照/时间轴可恢复、内容 Diff 失效）。

<!-- RULE
rule_id: BR-WS-STATE-003
主链路: WS-OPERATION
域: STATE
-->
WorkspaceOperation 必须先形成不可变 WorkspaceOperationPlan，并只由绑定该 plan_id 的
DecisionRequest 前置裁决；未确认、拒绝、过期或确认时基线再核验失败均不得产生磁盘副作用。确认且再核验成功后
WS 直接提交 DiskState，不生成或等待资源 DiffCard。重复确认和重复执行以 idempotency_key 返回同一结果，不重复提交。

<!-- RULE
rule_id: BR-WS-DATA-003
主链路: WS-OPERATION, WS-TIMELINE
域: DATA
-->
WorkspaceOperationPlan 必须冻结有序操作、资源清单、前置基线、影响摘要、失败策略和必要快照引用；执行结果必须逐项
记录 attempted/completed/compensated/failed。成功、补偿成功或补偿不完整都写 WorkspaceOperationEntry，历史不得
因失败或恢复被覆盖。资源操作历史与文档内容节点统一呈现在 WorkspaceTimeline，但二者使用类型化还原器。

<!-- RULE
rule_id: BR-WS-OBS-002
主链路: WS-OPERATION, AG-TURN
域: OBS
-->
确认卡必须展示实际目标、操作种类、递归数量、覆盖/冲突边界、脏会话或内容 Diff 影响及可恢复性；执行中、已拒绝、
已过期、已补偿、部分失败和完成必须由真实事件投影，不得把等待裁决或补偿过程表述为已完成。

<!-- RULE
rule_id: BR-WS-PERSIST-003
主链路: WS-OPERATION, WS-TIMELINE
域: PERSIST
-->
一批操作采用 fail_fast_compensate：任一项失败立即停止，已完成项按反向顺序执行补偿；不得承诺文件系统级严格事务。
补偿全部成功则终态 failed_compensated；任一补偿失败则终态 partial_failed，并保留逐项回执和可人工恢复的快照。
应用重启不得自动继续旧授权；任何未完成状态统一进入 interrupted。用户显式恢复时必须基于真实磁盘与既有回执生成新的修复/重试计划和新的 DecisionRequest。

<!-- RULE
rule_id: BR-WS-STATE-009
主链路: WS-FS, WS-TIMELINE
域: STATE
-->
人工文件树（用户 UI 直调 ws.create_doc/create_folder/rename_doc/delete_doc/delete_folder）的删除与重命名
必须复用与 Agent 资源操作一致的安全底线：命中 dirty 编辑会话阻断（WS_DELETE_DIRTY_TARGET /
WS_RENAME_DIRTY_TARGET），不得静默丢弃未保存编辑；删除前保存可恢复快照并写 WorkspaceTimeline 条目
（restore_kind=manual_delete/manual_rename），删除后可经时间轴生成新计划与 DecisionRequest 恢复；
成功后使受影响的有效内容 Diff 失效。人工创建保持直写（低风险、无破坏性）；人工动作不进入
DecisionRequest 前置裁决——WS-OPERATION 链仅限 Agent 意图（2026-08-22，WS-RESOURCE-WRITE-GUARD-01）。

## 4. 操作策略

| 操作 | 确认后磁盘效果 | 关键 guard | 还原器 |
|---|---|---|---|
| create_document | 创建空文件；初始内容另走 CreateDocumentBundle | 目标不存在；父路径合法 | 删除本次创建且身份未漂移的文件 |
| create_folder | 创建目录 | 目标不存在或明确幂等空目录 | 删除本次创建且仍为空/成员均属本次操作的目录 |
| rename_resource | 原路径重命名为目标路径 | 源存在、目标不存在、无脏会话 | 反向重命名 |
| move_resource | 同 workspace 内移动 | 源存在、目标不存在、无脏会话 | 反向移动 |
| delete_resource | 删除前保存完整资源快照，然后移除 DiskState | 递归清单一致、无脏会话 | 从快照重建到安全目标 |
| restore_resource | 从 WorkspaceTimeline 快照恢复 | 目标无冲突 | 删除恢复产物或还原恢复前状态 |

目录移动、跨 workspace 移动、覆盖既有目标和永久清理历史快照不进入 Agent 1.0。

## 5. 打开文档、脏会话与内容 Diff

- 删除、重命名或移动命中 dirty LogicalState 时，计划和 DecisionRequest 可展示完整影响，但确认后的 Revalidating 必须阻断执行；用户先保存或明确放弃后需重新核验或重新生成计划。
- 命中仍可裁决的内容 Diff 时，确认卡必须展示“资源操作成功后这些 Diff 将失效”；用户确认后以资源身份/路径变化原因使其失效。
- 操作成功后由同一提交协调更新 DocRef、编辑器标签、活动文档、BDocx 清单与后续引用；历史引用保留发送时快照。
- 外部修改发生在卡片生成与确认之间时，计划 expired，不能借旧确认覆盖或删除变化后的资源。

## 6. CreateDocumentBundle

初始内容必须在确认前生成并冻结，但确认前不创建文件、不显示可裁决 DiffCard：

```text
生成目标路径 + initial_content_ref
→ 创建 WorkspaceOperationPlan(create_document)
→ 用户确认
→ 创建空文件并记 WorkspaceOperationEntry(create)
→ 打开标签、建立 LogicalState
→ 将 initial_content_ref 投影为绑定初始 DiffCard
```

绑定初始 Diff 接受时完全遵守普通内容 Diff：只裁决，不重复应用；保存才把 LogicalState 写入磁盘并记录内容节点。

绑定初始 Diff 拒绝时，先核验：文件仍是该 operation_id 创建的资源、DiskState 仍等于空文件基线、LogicalState
无人工/后续 Agent 版本推进、路径和外部基线未变化。全部满足才补偿删除文件、关闭标签，并追加
`discard_created_resource` 资源操作条目；任一不满足则初始 Diff 进入基线失效态，禁止接受/拒绝，文件和当前内容保留。
普通内容 Diff 永远不得触发删除文件。

若初始内容在确认前生成失败，不产生 DecisionRequest、不创建文件。确认后投影失败时，按同一 operation_id 自动补偿
删除仍未漂移的空文件；补偿失败进入 partial_failed 并显示真实空文件状态。

## 7. WorkspaceTimeline

现有 DocumentTimeline 继续记录文档落盘内容版本；WorkspaceTimeline 是统一查询和 UI 入口，增加
WorkspaceOperationEntry 分支。资源条目必须保存足以执行类型化反向操作的信息，而不是写入不可取回的
`snapshot_ref="deleted"` 占位值。

还原动作本身也先生成 WorkspaceOperationPlan 并经 DecisionRequest 前置裁决；确认后执行还原并追加新条目，
不删除、改写或倒退既有历史。被拒绝且从未进入 DiskState 的初始内容只保留在 Diff 审计历史，不进入时间轴快照。

## 8. TaskRun 接口边界

TaskRun 的 WaitingConfirm 唯一由 DecisionRequest 产生。确认后恢复原 TaskRun，WS 执行完成即返回
OperationOutcome；不增加 WaitingUserCommit。失败不自动重试，重启进入 Interrupted，用户显式恢复时生成新执行片段
并生成新的计划/DecisionRequest。TaskRun 不解析 WorkspaceOperationPlan 的文件系统字段，也不执行补偿。

## 9. 验收矩阵

1. 未确认、拒绝、过期和基线漂移均零磁盘副作用。
2. 创建/重命名/移动/删除分别覆盖成功、冲突、重复确认和幂等。
3. 批量第二项失败时第一项反向补偿；补偿失败形成 partial_failed 与逐项回执。
4. dirty 目标不能进入执行；内容 Diff 影响在卡片可见且确认后准确失效。
5. 文件、空目录、非空目录删除均保存可取回快照并可经 WorkspaceTimeline 前进式恢复。
6. CreateDocumentBundle 接受、拒绝、基线推进、外部修改、投影失败和补偿失败全部覆盖。
7. TaskRun 确认后继续原 run；拒绝终止未执行步骤；重启不自动使用旧授权。

## 10. 需求到规则映射

<!-- REQ_MAP
req_id: REQ-WS-006
target_type: RULE
target_id: BR-WS-STATE-003
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-WS-006
target_type: RULE
target_id: BR-WS-DATA-003
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-WS-008
target_type: CHAIN
target_id: WS-OPERATION
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-WS-008
target_type: RULE
target_id: BR-WS-PERSIST-003
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-WS-009
target_type: RULE
target_id: BR-WS-STATE-003
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-WS-010
target_type: RULE
target_id: BR-WS-DATA-003
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-WS-010
target_type: RULE
target_id: BR-WS-OBS-002
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-WS-008
target_type: RULE
target_id: BR-WS-STATE-009
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-WS-010
target_type: RULE
target_id: BR-WS-STATE-009
status: mapped
-->

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-21 | v1.0 | 根据人类确认的八项决策建立 WorkspaceOperation/WorkspaceOperationPlan、前置裁决直接写盘、统一 WorkspaceTimeline、CreateDocumentBundle、fail-fast 补偿、dirty 阻断与 TaskRun 边界。 |
| 2026-08-22 | v1.1 | WS-RESOURCE-WRITE-GUARD-01：WS-OPERATION 链推进 active（integration_test=deferred: G5）并收窄为 Agent 意图；新增 BR-WS-STATE-009（人工文件树删除/重命名安全底线：dirty 阻断、快照/时间轴可恢复、内容 Diff 失效）；REQ-MAP REQ-WS-008/010 → BR-WS-STATE-009。 |
