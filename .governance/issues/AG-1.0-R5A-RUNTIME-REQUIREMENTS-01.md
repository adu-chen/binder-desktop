# Issue Trace — AG-1.0-R5A-RUNTIME-REQUIREMENTS-01：运行时、意图授权、可见事件与会话隔离

文档状态：A
任务类型：DOC_ONLY
治理阶段：REQUIREMENT
创建时间：2026-08-14

## 1. 用户可见问题

若 Agent 的运行配置、澄清、取消、工具授权和会话历史没有共同边界，模型会在不同入口或重启后得到不一致能力：
缺失 profile 可能意外启动、澄清被表现为完成、取消后的迟到事件继续改变界面、不同会话混入消息或未保存上下文。
这些不是单个 Prompt 能解决的问题，而是 Agent 1.0 能否可信运行的 P0 前提。

## 2. 已核实的实现漂移

- `turn.rs` 的 `ensure_runtime_profile` 会拒绝未知值，但 Runtime Profile 说明记录：缺失 profile 目前为兼容旧调用而默认成 `agent-edit-1.0`。
- `ClarificationRequired` 当前从 `Assembling` 进入 `Completed`，不是等待用户补充的状态。
- `Cancel` 当前只定义自 `Streaming` 进入 `Cancelled`，不能覆盖全部活动阶段。
- `agent_turns` 持久化和恢复仅以 `workspace_id` 写入/查询，没有 AgentSession 维度。

这些均为后续设计/实现迁移债务；本轮不改代码、接口、Prompt、状态机或测试。

## 3. 本轮人类确认

1. 只有 `agent-edit-1.0` 可以启动；未知或缺失 profile 都必须失败关闭。Build/CBT 保持 P2。
2. 一个用户发送对应一个 AgentTurn；澄清为等待用户输入的非终态。
3. 取消可在任何活动阶段生效；首个终态获胜，迟到事件和工具不得再产生副作用。
4. IntentResult 必须同时给出任务类型、目标、预期副作用、所需上下文、允许工具与确认点。
5. 涉及写入但目标或范围不清时，只能要求澄清或生成目标计划，不能猜测、生成 Proposal 或执行写入。
6. 用户只看到结构化任务事件：意图结论、读取/工具、Proposal/操作卡、等待确认、完成/取消/失败原因；不展示模型内部推理。
7. workspace / AgentSession / AgentTurn 是三层隔离。AgentSession 是持久化、恢复和展示隔离边界；消息、工具卡、Diff 卡、引用和未保存上下文不串会话。
8. 历史与编辑事实均本地保存；长期项目记忆保持后续 P1 决策。

## 4. 需求边界

- Runtime Profile 只给能力上限，不取代 IntentResult 或 CapabilityGrant 的逐回合收窄。
- 已注册工具、前端参数、提示词声明或旧兼容路径都不能扩大本轮授权。
- 结构化可见事件不是模型思维过程；进度和终态必须可核对、可取消、不可伪成功。
- 会话历史和编辑历史是事实记录，不能被模型摘要替代；workspace 不能替代 AgentSession 隔离键。

## 5. 直接消费者

- CORE：新增产品级运行底座边界，作为 Agent 1.0 P0 的可见可信运行要求。
- AG：扩展 Runtime、IntentResult、CapabilityGrant、AgentTurn、事件呈现和 AgentSession 需求。
- SYS：承接 Profile/Turn/Intent/Grant/Session 的共享身份、事件、持久化与恢复隔离契约。

ED 与 REF 已通过会话事实和引用隔离需求消费该边界，本轮不重复新增其 L1 REQ。

## 6. 验收计划

- profile：已知、未知和缺失值都验证在 Gate、provider、工具和 Proposal 前得到正确结果。
- 回合：澄清等待、各活动阶段取消、首终态锁存、迟到事件/工具抑制、失败与拒绝不伪装完成。
- 意图/授权：只读分析不产生 Proposal；写入歧义只能澄清/给计划；工具必须同时满足 profile、意图、目标、副作用、格式/动态能力与参数守卫。
- 可见事件：用户可看到结构化步骤和终态原因，不见内部推理。
- 隔离与恢复：多 workspace、多 AgentSession 交错，消息、事件、工具卡、Diff、引用和未保存上下文不串线；恢复只显示所属本地会话事实。

## 7. 待设计项

- Runtime Profile 的强制入参迁移、AgentTurn 的等待澄清/取消状态、终态锁存和迟到事件处理。
- IntentResult、CapabilityGrant、AgentSession、结构化事件和脱敏 Trace 的字段、传输与持久化契约。
- AgentSession 本地存储/恢复、会话标签创建删除归档、历史容量和异常退出后的清算。

## 8. 风险与迁移债务

- 缺失 profile 默认 `agent-edit-1.0` 与“缺失也拒绝”的新基线冲突。
- `ClarificationRequired → Completed`、取消只覆盖 Streaming、按 workspace 恢复回合历史均与新基线冲突。
- 不得通过修改 Prompt、隐藏事件、吞掉取消错误或仅调整 UI 来伪造符合；须在后续 DESIGN Scope 完成状态机、接口和验证后迁移。

## 9. 文件范围结论

允许修改：CORE 产品根/需求索引、AG 能力基线与 L1、SYS L1、本 Issue Trace 与 active scope。

禁止修改：ED、REF、WS、DE、DOC 等现有消费需求；L2 技术设计/状态机/接口、代码、测试、脚本、APC、ADUS 和治理配置。

## 10. Scope 外事项

- R5-B 任务编排的具体步骤图、恢复/重试与任务队列；R5-C 长期项目记忆、外部文件内容读取。
- Build/CBT 授权、模型参数、复杂自主规划、多 Agent、数据库/事件字段、Prompt、UI、工具执行器和测试实现。
- 会话删除/归档、容量、异常恢复的具体产品交互和技术策略。

## 11. Task Runtime Report

- task_type：`DOC_ONLY`
- governance_phase：`REQUIREMENT`
- final_state：`READY`
- changed_files：CORE 产品根、AG 能力基线与 L1、SYS L1、需求索引、本 Issue Trace 与 active scope。
- mapping_chain：不适用；本轮不修改 REQ_MAP、RULE、CHAIN、CONSTRAINT、TERM、状态机或接口。
- test_impact：不适用；本轮无代码或测试改动。
- validation：已核对 R5-A 关键语义在 CORE/AG/SYS/索引的一致性；CORE 16、AG 14、SYS 10 条 REQ 均无重复；REQ-CORE-016、REQ-AG-007、REQ-SYS-010 的必填字段完整；范围内文档尾随空白扫描与 `git diff --check` 均通过。
- automated_tests：不适用。
- unresolved_items：见 §7；R5-B/R5-C 与 DESIGN Scope 完成前不得进入实现。
