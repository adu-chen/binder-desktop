# Issue Trace — AG-1.0-R5B-TASK-ORCHESTRATION-REQUIREMENTS-01：受限复合任务、真实进度与 Diff 接力

文档状态：A
任务类型：DOC_ONLY
治理阶段：REQUIREMENT
创建时间：2026-08-14

## 1. 用户可见问题

只把意图识别当作“选一个工具”时，Agent 无法可靠完成“先创建、再编辑、再排版”或“研究资料、起草大纲、分步生成内容”等复合目标；用户也无法知道它究竟正在工作、等待自己还是已经结束。

同一文档的连续编辑还必须处理 Diff 接力：若 Diff2 修改 Diff1 预览结果中的一句话，系统不能把两个卡片当作可任意叠加或回滚的补丁栈，否则会违背 LogicalState、基准推进和卡片失效规则。

## 2. 已核实的实现与设计漂移

- 现有 AG L2 把 `clarificationRequired` 转为 `Completed`，与 R5-A 的“澄清等待”冲突。
- 现有 AG L2 设计 `TurnError → retry → Assembling`，而 R5-B 要求失败事实保留、用户显式重试创建新 AgentTurn。
- 现有 L2 只把取消定义在 Streaming；R5-A 已确认取消必须覆盖所有活动阶段。
- 当前流中断可保存为 `draft`，但尚无 R5-B 所需的“重启显示中断可重试、绝不自动续跑”的完整任务语义。
- 较新的 history 实现已按 workspace + AgentSession 查询回合；旧 workspace-only 恢复辅助路径仍存在，后续 DESIGN 必须统一。

这些均为后续设计/实现迁移债务；本轮不改 L2、代码、接口、Prompt 或测试。

## 3. 本轮人类确认

1. 意图识别是任务编排入口：复杂目标必须形成受限 TaskRun，而不是只选择单一工具。
2. 一个用户目标可组织多个 AgentTurn 和步骤；TaskRun 不等于后台队列，不允许脱离用户持续自主执行。
3. 明确的同文档复合目标允许连续产生 Diff1、Diff2、Diff3，直到既有确认点、澄清、失败或取消才暂停。
4. Diff2 必须读取 Diff1 成功预览后的最新 LogicalState；Diff2 成功预览推进基准，使 Diff1 失效、保留历史且不可再裁决。
5. 连续 Diff 不形成补丁栈或任务级事务/回滚。拒绝最新 Diff 只能回到其自身有效基准，不能回滚已失效的先前卡。
6. 无 LogicalState 的背景 pending Diff 不得作为后续编辑输入；必须先打开并迁移到 LogicalState，或按背景分流裁决后读取真实当前状态。
7. 用户必须能看见真实的当前状态：任务判断/步骤计划、读取或搜索、工具执行、生成 Proposal、等待补充、等待卡片裁决、完成、失败、取消或中断可重试；不得以聊天文本或模型内部推理伪装工作中。
8. 其余 R5-B 边界保持确认：一个 AgentSession 最多一个执行中 AgentTurn；无任务队列、无任务级自动重试、无跨重启自动续跑；取消停止后续步骤但不回滚已成功预览的 LogicalState/Diff 生命周期。

## 4. 需求边界

- TaskRun 只组织已授权的最小步骤。每一步重新受 Runtime Profile、IntentResult、CapabilityGrant、当前目标、当前上下文和既有确认点限制。
- 只读/研究步骤可以受限连续执行；多既有文档、删除、结构保存及其他已确认确认点仍按 R4 规则暂停，TaskRun 不可越过。
- Proposal 预览不是接受或保存。继续同文档步骤可推进 LogicalState，但不会自动接受、保存或绕过卡片裁决。
- 用户取消后，已成功的预览仍在当前 LogicalState 和最新卡片中如实存在；系统只停止尚未开始的后续步骤。
- 重试只能由用户显式发起，并创建新的 AgentTurn；必须重新校验 profile、Gate、意图、上下文、基准和授权。已取消/已完成/已拒绝回合不复活。

## 5. 直接消费者

- CORE：新增产品级复合任务、真实进度和连续 Diff 的统一边界。
- AG：TaskRun、复合意图、步骤推进、最小授权、澄清/中断/重试和真实工作状态。
- DE：连续 Proposal 的 TaskRun 关联、最新 LogicalState 基准、前卡失效和不可叠加/回滚边界。
- SYS：TaskRun/步骤/AgentTurn 的共享身份、事件、恢复和隔离契约。

ED、WS、REF 的现有 L1 已分别承接 LogicalState、卡片失效、保存分流和会话事实；本轮不重复新增其 REQ。

## 6. 验收计划

- 复合任务：创建文档→生成内容→排版；研究→大纲→内容；每一步可见、可取消、受最小授权约束，不产生后台续跑。
- 连续 Diff：Diff1 预览进入 LogicalState 后 Diff2 修改其中一处；Diff1 失效且不可裁决，Diff2 为当前可审阅卡；拒绝 Diff2 只回到其自身基准。
- 背景 Diff：未打开目标不得直接在 pending payload 上生成连续编辑；打开迁移或背景卡裁决后才可继续。
- 状态展示：活跃工作、等待补充、等待卡片、完成、失败、取消与中断可重试可区分，且活跃状态有实际回合/步骤/工具事件证据。
- 恢复与重试：中断回合重启后不自动继续；用户重试产生新回合并重新校验；取消后不回滚已有预览。

## 7. 待设计项

- TaskRun、步骤、关联 AgentTurn、确认点、Diff 链、实时进度、恢复/重试和 Trace 的字段、状态机、IPC、持久化和投影。
- 一会话与跨会话的并发策略、工具循环/时长/工作量上限、计划/进度的具体 UI 和可访问性。
- L2/实现迁移：澄清等待、全活动态取消、终态锁存、旧原回合 Retry、`draft` 中断投影及历史恢复路径统一。

## 8. 风险与迁移债务

- 若把 Diff2 建立在 Diff1 payload 而非当前 LogicalState，会造成卡片可错误叠加、拒绝错位或回滚后续内容。
- 若将 TaskRun 实现为后台队列、自动重试或跨重启续跑，会越过用户持续参与和确认边界。
- 若“工作中”由模型文本或前端静态 spinner 驱动，会掩盖等待、失败和中断；必须以实际运行事件为真值。
- 不得用 Prompt、隐藏旧卡或吞掉中断错误来伪造满足；须在后续 DESIGN Scope 完成状态机、接口和验证后迁移。

## 9. 文件范围结论

允许修改：CORE 产品根/需求索引、AG 能力基线与 L1、DE L1、SYS L1、本 Issue Trace 与 active scope。

禁止修改：ED、REF、WS、DOC 等既有消费需求；L2 技术设计/状态机/接口、代码、测试、脚本、APC、ADUS 和治理配置。

## 10. Scope 外事项

- R5-C 长期项目记忆和外部文件内容读取。
- 任务队列、任务级事务/回滚、自动重试、自动跨重启续跑、开放式自主规划、多 Agent、Build/CBT。
- TaskRun 和进度的具体数据模型、UI、事件字段、存储、IPC、工具执行器和测试实现。

## 11. Task Runtime Report

- task_type：`DOC_ONLY`
- governance_phase：`REQUIREMENT`
- final_state：`READY`
- changed_files：`docs/00_core/A-CORE-C-D-01_产品概述.md`、`docs/00_core/A-CORE-X-D-01_1.0需求基线与文档体系索引.md`、`docs/01_modules/AG/A-AG-M-D-01_代理功能需求描述.md`、`docs/01_modules/AG/A-AG-M-D-02_代理1.0能力基线与需求分解.md`、`docs/01_modules/DE/A-DE-M-D-01_DiffEdit功能需求描述.md`、`docs/01_modules/SYS/A-SYS-M-D-01_系统底座功能需求描述.md`、本 Issue Trace 与 active scope。
- delivered_requirements：`REQ-CORE-017`、`REQ-AG-015`、`REQ-DE-009`、`REQ-SYS-011`。
- mapping_chain：不适用；本轮不修改 REQ_MAP、RULE、CHAIN、CONSTRAINT、TERM、状态机或接口。
- test_impact：不适用；本轮无代码或测试改动。
- validation：REQ 编号唯一性与必填字段复核通过（CORE 17、AG 15、DE 9、SYS 11）；上游引用与 R5-B 语义检索复核通过；`git diff --check` 与尾随空白复核通过；`npm run governance:audit` 的聚焦原始字段复核通过（总体、governance、TERM、test governance、product capability 均为 `passed`，`governance_findings` 为空）。
- automated_tests：不适用。
- unresolved_items：见 §7；R5-C 与 DESIGN Scope 完成前不得进入实现。

## 12. 语义纠偏契约登记（2026-08-15 · AG-1.0-TASKRUN-REWORK-REQ-01）

落地审计确认本冻结需求被实现收窄（TaskRun=每回合事后事件记录）。纠偏契约已冻结于：
`A-AG-M-D-01 §5b`（三层分离 / TaskPlan / DecisionRequest / 确认 gate / 同文档连续 Diff）、
`A-AG-M-S-01 §2.2.1`（SM-AG-04 语义纠偏，冲突处以纠偏优先）、`A-AG-M-T-01`（TERM-AG-017/018）。
本 issue 维持 `DOC_ONLY`/`REQUIREMENT` 状态；实现须符合纠偏契约。
