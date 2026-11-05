---
文档编号：AG-M-S-01
文档状态：A
负责模块：AG
文档职责：AG 代理模块级状态机设计——代理回合机（SM-AG-01）、工具调用循环机（SM-AG-02）、辅助回合机（SM-AG-03）与 TaskRun 投影机（SM-AG-04），含 APC §6 必备字段；登记 AG↔DE / AG↔ED / AG↔SYS 门禁 X-INTER-001 转移点
上游约束：A-AG-M-T-01_代理技术设计.md（AG-TURN/AG-TOOL/AG-ASSIST 链路、BR-AG-* 规则）、A-CORE-C-P-01_APC.md（§6）、A-SYS-M-S-01_状态机总图.md（X-SEQ-003/X-INTER-001）、A-SYS-I-P-03_云接口契约.md（SM-SYS-01 门禁段）、A-SYS-I-P-02_IPC协议.md（BR-SYS-STATE-002 流式终态）、A-ED-M-S-01_编辑器状态机设计.md（SM-ED-01 edit 入口）、ADU.md（§3.1）
直接承接：四机经 rules 字段承载 A-AG-M-T-01 与 SYS 共享规则；machine_id 登记总图；SM-AG-04 仅作事件派生投影；产出交 DE 转移点供 DE 对接
使用边界：本文定义 `agent-edit-1.0` 的 AG 状态机内部；不重定义 SM-SYS-01（门禁裁决归 SYS，本文以转移点对接）、不定义 DE diff 终态机、不定义 ED/WS 机内部；Build/CBT 为 P2 后续 Profile，不进入本文状态集合或转移
变更要求：transitions 变更须同步 A-AG-M-T-01 规则并重跑测试矩阵；跨模块转移点变更须与对侧 owner 共同复核；machine_id 一经登记不可更改
治理成熟度：draft
---

# Binder 代理状态机设计（AG · L2 技术设计）

## §1 文档定位

```
A-AG-M-T-01（规则 SSOT）→ 本文 SM-AG-01/02/03/04
本文 machine_id ──登记──→ A-SYS-M-S-01 §3/§3.1
转移点：AG↔SM-SYS-01（门禁）、AG 产出↔DE、SM-AG-03 采纳↔SM-ED-01
```

满足 X-SEQ-003：AgentTurn、ToolCall、Assist 与 TaskRun 的状态化核心链在此建机。
autocomplete 与 inline-assist 由 **SM-AG-03 同机承载**（二者生命周期同构：触发→节流→门禁→
轻量调用→装饰层呈现→采纳/弃，区别仅在触发源与候选形态，归 context 字段），总图两行回填同一
machine_id 并注明。

### 1.1 `STATE_MACHINE.rules` 字段语义

`rules` 只登记**直接约束本机转移、守卫、终态、等待语义、证据/观察**的 AG/SYS 规则。ED/DE/DOC/WS 等下游效果规则通过链路视图表达，不进入本机规则闭包；若未来需要引入外部规则，必须新增显式字段并经治理评审，不得混入 `rules`。该语义用于保证四台 AG 状态机的规则闭包可比较、可审计。

`@GOV chain` 字段采用有序约定：**首项为 primary_chain，其余项为 consumer_chains**。ADUS 链路视图只按 primary_chain 分组，consumer_chains 列展示跨链消费者；同一 code 不得在多个链视图中拥有主责。外部模块块（WS/DE/SYS 等）以自身主链为主，AG 只作为 consumer_chains 出现，不计入 AG owned 统计。

## §2 SM-AG-01 · 代理回合机

<!-- STATE_MACHINE
machine_id: SM-AG-01
owner_module: AG
status: active
summary: 一次代理回合的事件化生命周期（冻结事实→门禁→模型/工具自适应循环→产出处置；可澄清/裁决/中断，无半落盘）
source_type: design
chain: AG-TURN
rules: BR-AG-STATE-001, BR-AG-STATE-002, BR-AG-STATE-005, BR-AG-VERIFY-001, BR-AG-DATA-001, BR-AG-DATA-002, BR-AG-DATA-003, BR-AG-DATA-004, BR-AG-DATA-005, BR-AG-DATA-008, BR-AG-PERSIST-001, BR-AG-OBS-001, BR-SYS-STATE-007, BR-SYS-STATE-008
initial_state: Idle
terminal_states: Completed, Cancelled, Denied, TurnError
-->

- **machine_id**：SM-AG-01　**owner_module**：AG
- **scope**：一次 `agent-edit-1.0` 代理回合（用户发起→产出处置完成/中断/拒绝；目标边界按 BR-AG-DATA-003）
- **context**：回合输入（session_id + runtime_profile + prompt + 注入载荷 + open_doc_snapshots）、append-only TurnEvent、当前 IntentResult revision、AgentReadContext、PromptPackage、CapabilityGrant、真实 GateOutcome、CancellationToken、terminal_latch、流式缓冲、最终回复及已持久化 ProposalBatch 引用、历史持久化状态（completed/error/cancelled/gate_rejected/draft）
- **states**：Idle、Assembling、AwaitingClarification、AwaitingDecision、AwaitingGate、Streaming、Finalizing、Completed、Cancelled、Denied、TurnError
- **events**：send、assembled、assembleFail、gatePassed、gateDenied、gateFailed、modelClarification、clarificationProvided、decisionProposed、decisionResolved、operationFailed、toolRoundCompleted、modelProtocolRejected、modelFinal、streamFail、cancel、finalizeOk、finalizeFail
- **transitions（主表）**：2026-09-15 P00 将 v2 准入改为身份先行；下表为目标设计，生产实现待
  P01-P08 验证。v1 仍按 I-P-01 的完整快照准入，不能把其校验失败冒充 v2 unavailable。

| from | event | guard | to | action |
|---|---|---|---|---|
| Idle | send | workspace/session/turn/Profile 合法；v2 activeTarget/openResources 身份与可用性有效（v1 仍要求完整快照）；执行槽可用且同 session 无其他活动回合 | Assembling | 持久化 AgentSession/AgentTurn draft 与 UserInputReceived；冻结 AgentContextFacts 身份及 revision；不因无关资源未就绪拒绝；IntentResult 初始 unresolved |
| Assembling | assembled | 本次实际注入的内容/锚均有有效版本；未就绪资源仅作为明确元数据；PromptPackage 角色/预算/作用域有效；terminal_latch 未锁定 | AwaitingGate | 追加 ContextFrozen/PromptPrepared；提交 SM-SYS-01 门禁（X-INTER-001）；不得由可用性替代 Gate |
| Assembling | assembleFail | 载荷版本/身份非法，或本次必需内容采集失败、过期、超预算；无关资源 unavailable 本身不触发 | TurnError | 统一错误形状、装配阶段及下一步；不得用 DiskState 或 loadedContent 旧快照兜底；保留此前专业效果 |
| AwaitingGate | gatePassed | 本次服务端 GateOutcome=GatePassed 且 verdict_id 可追踪 | Streaming | 追加 GatePassed/ProviderStarted；经 AiGateway 发起一次流式调用；本地常量/缓存/桩值不得触发 |
| AwaitingGate | gateDenied | SM-SYS-01 GateDenied：合法 denied 或明确授权 HTTP 拒绝 | Denied | 明确拒绝（终态，不可绕过）；AgentTurn status=`gate_rejected` |
| AwaitingGate | gateFailed | 认证/门禁不可达、超时、非授权 HTTP 异常、非法或未知响应 | TurnError | 保留原错误码、可重试标记和 gate 观察边界；不调用模型、不自动重试；此前工具证据保留，AgentTurn status=`error` |
| Streaming | modelClarification | 模型可见输出明确缺少继续所需信息，且未产生副作用 | AwaitingClarification | 更新 IntentResult revision，追加 ClarificationRequested；停止 provider/工具，等待用户 |
| AwaitingClarification | clarificationProvided | 用户向原 AgentTurn 补充所需信息；同 session 无其他活动回合 | Assembling | 追加 ClarificationProvided；重新冻结事实并装配；不创建新 AgentTurn、不沿用旧授权 |
| Streaming | decisionProposed | 已形成不可变 DecisionRequest | AwaitingDecision | 追加 DecisionRequestProposed；停止 provider/工具，等待裁决 |
| AwaitingDecision | decisionResolved | 对应请求已 confirm/cancel；confirm 涉及 WS 操作时，运行时基于当前终态/取消状态、确认回执与冻结 plan 重新计算 permitted_effect_paths，通过后才可请求 WS 重验执行 | Assembling | 追加 DecisionRequestResolved 与可选领域执行结果；重新装配，TaskRun 仅同步投影；不把 WS 执行入口暴露给模型 |
| AwaitingDecision | operationFailed | 已确认领域操作或后处理失败且仍属原draft回合 | TurnError | 冻结后续执行；结算领域效果、TaskRun投影，核心输出结果说明并提交唯一error；不追加模型请求 |
| Streaming | toolRoundCompleted | SM-AG-02 已追加全部 ToolResult，且未产生等待/终态 | Assembling | 根据证据更新 IntentResult/EvidenceLedger 投影；下一次 provider 调用重新装配和 Gate |
| Streaming | modelProtocolRejected | 完整响应控制校验未通过，仍有纠错/模型预算且未终结 | Assembling | 记录脱敏拒绝与纠错指令；不推断意图、不执行本批；重新装配、预算检查和 Gate |
| Streaming | modelFinal | 模型已给出最终可见回答，且无未决 ToolCall/DecisionRequest；任何修改建议已通过工具形成 ProposalBatch | Finalizing | 追加 ModelFinal；核验最终回复、ProjectReport 投影与已持久化 ProposalBatch 引用，不接受绕过工具的直接修改载荷 |
| Streaming | streamFail | — | TurnError | 统一错误形状（含可重试标记）；AgentTurn status=`error` |
| Assembling / AwaitingClarification / AwaitingDecision / AwaitingGate / Streaming / Finalizing | cancel | terminal_latch 尚未锁定 | Cancelled | 触发 CancellationToken，终止已启动的流式请求、阻止新模型/工具/Proposal 副作用并追加唯一终态；无半落盘副作用；AgentTurn status=`cancelled` |
| Finalizing | finalizeOk | 最终回复已持久化；若有修改建议，ProposalBatch 已由 DE 生成一个或多个 pending_diff | Completed | 追加唯一 Completed；AgentTurn status=`completed`；不得自动触发项目记忆副作用（BR-AG-STATE-004） |
| Finalizing | finalizeFail | 最终回复/投影持久化或一致性校验失败 | TurnError | 保留真实 Proposal 事实但不伪报回合完成；AgentTurn status=`error` |
- **error_states**：TurnError（原 AgentTurn 终态）
- **recovery_paths**：用户对 TurnError 显式重试时创建新 AgentTurn，由新状态机的 Idle→send→Assembling 启动；原 AgentTurn 保持 TurnError/error 用于审计。Denied 不可重试绕过（须恢复订阅/网络后新建 AgentTurn 并 send）
- **forbidden_transitions**：
  - Idle / Assembling → Streaming（未过门禁不得调用，对齐 SM-SYS-01 forbidden）
  - Denied → Streaming（被拒不得绕过门禁，终态）
  - Streaming → Completed（必须经 Finalizing 核验回复、报告投影与既有 Proposal 引用）
  - Assembling → AwaitingGate 当实际作为正文/有效编辑锚注入的打开文档缺少有效 OpenDocSnapshot（v2 可显式仅保留未就绪元数据，不伪称已读；v1 仍按完整准入校验）
  - 任意态 → 直接写盘（本机无写盘动作，X-FORBID-006）
  - Completed / Cancelled / Denied / TurnError → 任意其他状态或终态（terminal_latch 只允许首次锁定）
  - AwaitingClarification / AwaitingDecision → AwaitingGate / Streaming / Completed（未收到对应用户事件并重新完成 Assembling，不得推进或伪装完成）
- **persistence_projection**：`send` 后先持久化 `draft`；`finalizeOk`→`completed`；
  `modelClarification/decisionProposed` 保持 `draft` 非终态；`gateFailed/streamFail/finalizeFail/命令级错误`→`error`；`cancel`→`cancelled`；`gateDenied`→`gate_rejected`；
  应用崩溃/关闭/流式中断未收到终态时保持 `draft`，重启后只能显示未完成/可重试。
- **acceptance_criteria**：不存在不经真实服务端 gatePassed 通往 Streaming 的路径；Denied 为终态且不可达
  Streaming；修改建议只能由 SM-AG-02 的 `propose_edit` 工具交 DE，Finalizing 不再生成修改；cancel 后无半落盘副作用；
  completed/error/cancelled/gate_rejected/draft 五类历史状态均可重启恢复；每个 turn 最多锁定一个
  用户可见终态，网关 Error/Denied/Cancelled 后不得再出现 Done/Completed；语义歧义由模型输出触发澄清；
  澄清/裁决等待期间零 provider/工具活动，补充或裁决只使原 AgentTurn 回到 Assembling，不创建新 AgentTurn、不产生取消终态；
  任一工具结果后的 provider continuation 必须走 Assembling→AwaitingGate；事件流可确定性重建状态；
  状态集合中不存在 Build takeover/HandedOver/fallbackIn。
- **test_matrix**：见 §5

**P00 交接补充（沿用现有状态，不新建后台等待机）**：

- 通用准入与专业内容就绪分开；专业采集只在当前 Assembling 或工具 Executing 内有界等待，超时/取消
  必须收敛。按需追加 contextRevision 后重新装配，旧 ContextFrozen 仍不可变。
- AwaitingDecision 的 requestId 是恢复身份，不能仅按“当前会话有等待”继续；A 消费后产生 B，
  A 的 UI 收尾或迟到事件不得清理 B。澄清/裁决等待不占正在执行的槽，但归属仍在原 session。
- 预览 checkpoint 继续保留专业 diffId/version/hash 校验；ack、预览、裁决和保存是不同事实。
- 带初始内容的单独创建与组合创建共用原AwaitingDecision保存等待，不因空文件已创建提前结束。预览回执只记录编辑器已应用的技术事实，不启动/恢复 TaskRun；接受/保存只改变卡片生命周期与磁盘状态。TaskRun 在片段 `partial` 且仍有剩余工作时由调度器自动开下一片段，上下文使用当前 LogicalState；无进展或真实权限/范围歧义才暂停。
  重复回执不得重复继续，过期需明确失败/下一步；回执成功而继续失败不得伪称未发生预览。
- 流缺少有效结束信号或模型控制非法必须按错误处理，不能默认 Completed；预算超限不是 Provider
  拒绝的同义词。诊断区分失败边界和根因，生产错误处理不自动重试整个请求。
- 取消后禁止启动新效果；已有专业效果按真实回执保留。无法核实效果时显示未核实和阻断重试，不把
  cancelled 解释为“所有操作已回滚”；本节不放宽已有原子性/无半落盘约束。
- 验收至少含无编辑器真实枚举、非法响应、连续确认 A→B、等待期间版本变化、重复回执、工具成功后
  模型失败、唯一终态及会话切换；只通过问候不能证明 Binder 运行语义闭合。

**完整工作流增量（2026-10-24人类批准）**：已确认资源的初始内容失败归同一AgentTurn，领域效果先结算、失败说明后提交唯一error；不得因前端投影抛错遗留draft。等待保存是原裁决续行的非终态，预览ack不越过保存依赖；真实保存及领域重验完成后才恢复。TaskRun只是这些事件的投影，不授予检查点或执行权限。错误终态前允许核心生成无模型/无工具的确定性结果说明，任务失败与交互已交付可以并存；首终态后仍禁止新效果。

### 2.1 澄清非终态与首终态契约（REQ-AG-007 / BR-SYS-STATE-007）

承接冻结基线 R5-A（A-CORE-X-D-02 §4.2）与 BR-SYS-STATE-007（A-SYS-I-P-01 §3.1）：

| 场景 | SM-AG-01 语义 | 说明 |
|---|---|---|
| 澄清等待 | 模型可见输出表达 blocking ambiguity 后，回合进入**等待用户输入的非终态**（AwaitingClarification），**不是 Completed** | 澄清期间不调用门禁/网关/工具、不产生修改建议、不锁定终态事件（BR-SYS-STATE-007） |
| 澄清后继续 | 用户补充输入后，原 AgentTurn 经 clarificationProvided 回到 Assembling，重新冻结事实并由模型修订 IntentResult | 不新建 AgentTurn，不把正常补充伪装为完成或取消（BR-AG-STATE-001） |
| 裁决等待 | DecisionRequestProposed 后进入 AwaitingDecision；确认/取消前不执行资源副作用、不继续 provider | confirm 只授权对应冻结计划进入领域重验，cancel 不取消整个回合；二者均不从 TaskPlan 自动调度 |
| 任一活动阶段取消 | 取消可在澄清等待、组装、门禁、流式等任一活动阶段生效 | 无半落盘副作用 |
| 首终态锁存 | 完成/失败/拒绝/取消互斥，`terminal_latch` 首个终态获胜 | 首终态后到达的迟到事件/工具不得产生文档、卡片或落盘副作用 |
| 结构化可见事件 | 用户只见意图结论、读取/工具、提案卡、等待确认与终态原因等 TurnEvent 安全投影 | 不展示模型内部推理；事件追加后不可覆盖（A-SYS-I-P-01 §3.1） |

### 2.2 SM-AG-04 · TaskRun 投影机（REQ-AG-015 / BR-SYS-STATE-008）

<!-- STATE_MACHINE
machine_id: SM-AG-04
owner_module: AG
status: active
summary: 复杂或跨执行片段目标的 TaskRun 可观察投影——由 TurnEvent 归纳目标、真实进度、等待点与模型当前候选路线；不调度、不授权、非后台队列；简单目标不创建
source_type: design
chain: AG-TURN
rules: BR-SYS-STATE-008, BR-AG-STATE-001, BR-AG-STATE-002, BR-CORE-STATE-001, BR-AG-DATA-003, BR-AG-DATA-008
initial_state: Projected
terminal_states: Completed, Failed, Cancelled, Interrupted
-->

- **machine_id**：SM-AG-04　**owner_module**：AG
- **scope**：复杂目标、多个真实工作阶段或等待点出现后，从一个或多个 AgentTurn 事件派生 TaskRun；简单目标仅使用 AgentTurn，不进入本状态机
- **context**：task_run_id、AgentSession、用户目标、关联 turn_ids、当前 IntentResult/route revision、已发生
  TaskStep 投影、DecisionRequest/Proposal 引用与最后事件序号；不含 CapabilityGrant、执行游标或可执行命令
- **states**：Projected、Active、Waiting、Completed、Failed、Cancelled、Interrupted
- **steps**（展示投影）：candidate、active、waiting、completed、failed、cancelled；candidate 不承诺执行
- **events**：taskProjected、activityObserved、routeObserved、waitObserved、waitResolved、terminalObserved、recoveryObserved
- **transitions（主表）**：

| from | event | guard | to | action |
|---|---|---|---|---|
| — | taskProjected | 已有事件足以证明复杂/跨阶段目标；简单回合不得触发 | Projected | 从事件建立最小目标投影，不启动任何工作 |
| Projected / Active / Waiting | routeObserved | 模型产生新的可见候选路线或 IntentResult revision | Active | 只更新候选 TaskPlan/TaskStep 投影；不得执行 candidate |
| Projected / Active / Waiting | activityObserved | 对应 AgentTurn 已真实进入 provider/tool/proposal 事件 | Active | 按事实更新当前动作和已完成结果 |
| Projected / Active | waitObserved | 对应 AgentTurn 已进入 clarification/DecisionRequest 等等待 | Waiting | 投影等待原因；不持有 provider/工具活动 |
| Waiting | waitResolved | 对应用户事件已发生 | Waiting | 记录等待已解除；只有后续 activityObserved 才回到 Active |
| Projected / Active / Waiting | terminalObserved | AgentTurn/任务聚合事实已终结 | Completed / Failed / Cancelled | 按真实首终态投影，不运行收尾副作用 |
| Projected / Active / Waiting | recoveryObserved | 重启时存在未终结投影 | Interrupted | 只读显示中断；不得自动续跑 |
- **forbidden_transitions**：简单目标 → taskProjected；任一投影事件 → 调用 provider/工具/WS；候选 TaskStep → 自动执行；TaskPlan/TaskStep → 产生 CapabilityGrant；waitResolved → Active 无真实 activityObserved；DiffCard accept/reject → Agent 路线；Interrupted → Active 自动续跑；任一态 → 后台队列。
- **acceptance_criteria**：简单目标不创建 TaskRun/TaskPlan；相同 TurnEvent 序列重建相同投影；TaskRun/TaskPlan 删除后可重建且不影响真实裁决；候选路线可被后续模型事件替换；等待期间无活动执行；连续 Proposal 以最新 LogicalState 接力（A-DE-M-S-01 §2.1）；取消不回滚已成功预览内容
- **test_matrix**：见 §5 增补
- **总图登记**：machine_id 已在 A-SYS-M-S-01 §3.1 登记为 active

#### 2.2.1 WorkspaceOperation 确认后的执行语义（2026-08-21）

- WorkspaceOperationPlan 通过 SM-AG-01 的 DecisionRequest 进入 AwaitingDecision；SM-AG-04 只投影 Waiting，资源完整状态由 SM-WS-04 承载。
- `userConfirm` 由 SM-AG-01 校验回执并调用 WS 重验冻结 plan_id；SM-AG-04 只在观察到领域事件后更新，不存在 `WaitingUserCommit`、资源 DiffCard 或后置保存裁决。
- WS 返回 Completed / FailedCompensated / PartialFailed 后追加领域结果，模型在下一次 Gate 后决定后续路线；TaskPlan 不自动推进依赖步骤。
- CreateDocumentBundle 的资源创建成功后，初始内容 Diff 只投影 Proposal 事实；DiffCard 仅管理内容卡生命周期。
- 应用重启使未完成资源计划和 TaskRun 投影进入 Interrupted；用户显式重试创建新 AgentTurn，重新计划、核验并产生新的 DecisionRequest，旧确认不得复用。

<!-- REQ_MAP
req_id: REQ-AG-007
target_type: RULE
target_id: BR-SYS-STATE-007
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AG-015
target_type: RULE
target_id: BR-SYS-STATE-008
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AG-015
target_type: RULE
target_id: BR-CORE-STATE-001
status: mapped
-->

## §3 SM-AG-02 · 工具调用循环机

> 本机嵌套运行于 SM-AG-01 的 Streaming 段（模型输出含工具请求时激活），共享其回合上下文与
> 取消信号；工具结果记录后回归 SM-AG-01 Assembling，下一次 provider 调用统一重新装配并过 Gate。

<!-- STATE_MACHINE
machine_id: SM-AG-02
owner_module: AG
status: active
summary: 回合内逐 ToolCall 事实循环（按当前硬事实计算 CapabilityGrant→校验→执行→记录结果→回上层重新装配；写盘禁令、上限、可中断）
source_type: design
chain: AG-TOOL
rules: BR-AG-STATE-002, BR-AG-VERIFY-001, BR-AG-DATA-003, BR-AG-DATA-008, BR-AG-DATA-009, BR-AG-DATA-010, BR-AG-DATA-011, BR-AG-DATA-012, BR-AG-DATA-013, BR-AG-OBS-001, BR-AG-PERSIST-001, BR-SYS-STATE-008
initial_state: ModelStreaming
terminal_states: LoopCompleted, LoopAborted
-->

- **machine_id**：SM-AG-02　**owner_module**：AG
- **scope**：一次代理回合内的工具调用循环（首个工具请求出现→终轮答复/中止）
- **context**：工具注册清单、当前运行事实、短生命周期 CapabilityGrant、AgentReadContext、循环计数与上限、当前工具请求/结果、CancellationToken、terminal_latch
- **states**：ModelStreaming、ToolValidating、ToolExecuting、ResultRecording、LoopCompleted、LoopAborted
- **events**：toolRequest、validateOk、validateFail、execOk、execFail、recorded、nextToolCall、finalAnswer、protocolRejected、proposalRecorded、decisionHandoff、loopLimit、loopError、cancel
- **transitions（主表）**：

| from | event | guard | to | action |
|---|---|---|---|---|
| ModelStreaming | toolRequest | terminal_latch 未锁定 | ToolValidating | 按 RuntimeProfile、注册表、当前范围/目标/格式/动态能力、裁决、预算和取消状态计算 CapabilityGrant，再校验请求 |
| ModelStreaming | finalAnswer | — | LoopCompleted | 终轮答复，回归 SM-AG-01 modelFinal |
| ModelStreaming | protocolRejected | 本响应拒绝事实已记录，未执行本批工具 | LoopCompleted | 结束当前解析片段，回归 SM-AG-01 modelProtocolRejected；不伪造工具请求或工具执行 |
| ToolValidating | validateOk | 工具已注册、属于刚计算的 CapabilityGrant 且目标/参数/权限有效 | ToolExecuting | 先追加/广播真实 ToolExecuting，再执行；只读经 AgentReadContext，写意图只生成 diff 建议 |
| ToolValidating | validateFail | 未注册/当前未授权/越权/目标或参数失效 | ResultRecording | 记录结构化拒绝，不执行副作用 |
| ToolExecuting | execOk | terminal_latch 未锁定 | ResultRecording | 追加成功 ToolResult；由事件重建 EvidenceLedger 的可证明覆盖 |
| ToolExecuting | execFail | — | ResultRecording | 追加失败 ToolResult；不增加证据覆盖 |
| ResultRecording | nextToolCall | 当前响应仍有未处理 ToolCall；未达上限；未形成等待 | ModelStreaming | 保持同一 SM-AG-02 激活，按响应顺序处理下一 ToolCall；不回归 SM-AG-01，不重新 Gate |
| ResultRecording | recorded | 当前响应最后一项 ToolCall 已记录；循环计数 < 上限；未形成等待 | LoopCompleted | 回归 SM-AG-01 toolRoundCompleted→Assembling；不直接调用 provider |
| ResultRecording | proposalRecorded | `propose_edit` 已真实形成 ProposalBatch | LoopCompleted | 追加逐卡 DiffProposed，当前响应剩余工具不再执行；回归 SM-AG-01 toolRoundCompleted→Assembling，后续 provider 调用重过 Gate |
| ResultRecording | decisionHandoff | 已真实形成不可变 DecisionRequest | LoopCompleted | 回归 SM-AG-01 decisionProposed→AwaitingDecision；当前响应剩余工具不再执行 |
| ResultRecording | loopLimit | 达循环上限 | LoopAborted | 中止并回报（统一错误形状） |
| 任意非终态 | loopError | provider/Gate/运行时错误使当前循环无法继续 | LoopAborted | 统一错误形状回归 SM-AG-01 TurnError；不得结算 Completed/Done |
| 任意非终态 | cancel | 用户中断 | LoopAborted | 中断终态，无半落盘 |

- **多 ToolCall 顺序**：一个模型轮次返回多个 ToolCall 时，按响应顺序为每个 ToolCall 独立执行
  `toolRequest→validateOk/validateFail→可选执行→结果记录`；中间项完成 `nextToolCall` 保持同一激活后处理
  下一项，最后一项完成 `recorded` 后才回归 SM-AG-01。校验失败项只回注拒绝原因，不进入 ToolExecuting，也不阻止
  后续独立 ToolCall 重新接受校验；任一项形成 ProposalBatch/DecisionRequest 并触发 proposalRecorded/
  decisionHandoff 后，当前批次剩余 ToolCall 不再执行。
  每次 provider continuation 经 SM-AG-01 Assembling→AwaitingGate→GatePassed 后开启新的 SM-AG-02 激活，
  初始态为 ModelStreaming；上一激活的 LoopCompleted 不复用为同一激活内的内部转移。激活边界由 SM-AG-01
  的 toolRoundCompleted→Assembling→AwaitingGate→GatePassed 事件标识；跨激活的状态变化不套用本表转移。
- **error_states**：无独立错误状态——单个工具执行失败经 execFail→ResultRecording 记录，后续由新 provider 轮次决定路线；
  循环级错误以 loopError 进入 LoopAborted，回归 SM-AG-01 TurnError，不得映射为 Completed/Done
- **recovery_paths**：LoopAborted→SM-AG-01 TurnError/Cancelled 路径处置
- **重复只读失败**：相同只读请求与错误重复出现时，不再继续工具尝试；保留失败回执及已有成功证据，在 Gate/期限内回装配，只允许 final/clarify 说明未完成范围。后续原生工具批次执行前拒绝，不重放副作用。模型持续不合约、网络失败、取消或预算耗尽仍保留真实 error/cancelled，不能伪造业务成功。参数的跨字段条件在上下文采集前与执行器共用校验，快照版本/页边界在取快照后再次核验。
- **收尾能力一致性**：可信 system 续行指令的 final-only 模式同时冻结 Prompt Manifest 和原生工具清单，不仅靠文字禁止。适用于重复只读失败及已有非只读操作后的协议修正；普通纠错和正常任务工具能力不变。用户/模型/工具消息不得通过同名字段伪造可信收尾指令。
- **只读失败后澄清**：纯只读任务的收尾若选择 clarify，等待期间不执行；checkpoint 可释放该次只读失败的临时收尾限制，使真实用户补充范围后的受校验续行重新取得能力。原累计 Gate/模型/工具/协议预算与失败证据不重置。有任何既有非只读调用（含拒绝/未知回执）时，不通过此路径解除防重放保护。
- **响应协议与动作交接纠错**：正文/工具的缺失、损坏、冲突控制以及缺少必要动作产物共享 AgentTurn 级最多两次纠错和冻结模型调用预算（开发默认12次）；从已提交 TurnEvent 重建，等待/确认续行不得重置。无封套的普通前置说明有界隔离，只有原生工具携合法响应级控制时可舍弃该说明并继续；显式损坏控制不能被工具控制覆盖。资源请求缺少计划时在正文释放前拒绝并有界纠正，不从文字制造确认；最终分析/阻断/无需操作须显式说明阶段和原因，不宣称已修改。正文纠错在唯一终态之前发生，保留原执行游标和证据；已有非只读调用时，正文纠错仅允许 final/clarify，不得重新提交工具。网络截断、供应商拒绝和取消不进入本协议纠错。
- **无执行额度**：不再有模型轮次/只读/写入/纠错/收尾预留计数；原生工具不会因“次数用完”冻结。回合只由模型 final/clarify、真实错误或取消结束；持续不合约由模型自行纠正，超时/取消仍走真实出口。done 不再有 budget_exhausted，TaskRun 按真实 completion 结算。新用户续办使用新身份、重新核验效果和授权，不能自动重放非只读动作。
- **forbidden_transitions**：
  - ToolValidating → ToolExecuting 无 validateOk（未注册/越权不得执行）
  - 已注册工具 → ToolExecuting 当其不在当前 CapabilityGrant（注册不等于当前获权）
  - ToolExecuting → 直接文件写入（写意图必须产出 diff 建议，X-FORBID-006）
  - ToolExecuting → DiskState 读取，当目标文档已打开且 AgentReadContext 中应存在 OpenDocSnapshot
  - ResultRecording → 直接调用 provider（必须回 SM-AG-01 Assembling→AwaitingGate）
  - 任意生产状态广播未经过 SM-AG-02 转移引擎
  - ResultRecording → ModelStreaming 无 nextToolCall（同批下一 ToolCall 必须经转移引擎，不得直接赋值状态）
  - LoopAborted → TaskRun Completed 或 AgentTurn Done
- **acceptance_criteria**：IntentResult/TaskPlan 内容改变不能单独改变 validate 结果；未注册、当前未授权或越权工具不可达 ToolExecuting；循环上限强制生效；任何工具执行
  不产生直接文件写入；read_doc/search_text/list_blocks 对已打开文档命中 OpenDocSnapshot，缺失时拒绝；
  多 ToolCall 逐项重新计算授权且失败项不执行；每次工具结果后的 provider continuation 都经 SM-AG-01 重新装配并取得新的 GatePassed，
  GateDenied/门禁错误只进入 LoopAborted/TurnError；ProposalBatch 记录后回 Assembling，DecisionRequest 进入 AwaitingDecision；loopError/loopLimit 进入
  LoopAborted 后只允许 TurnError，cancel 随时可达 LoopAborted；生产广播均由转移引擎输出
- **test_matrix**：见 §5

## §4 SM-AG-03 · 辅助回合机（autocomplete + inline-assist 同机承载）

<!-- STATE_MACHINE
machine_id: SM-AG-03
owner_module: AG
status: active
summary: 辅助续写/局部修改回合（快捷键手动触发→防抖/latest-wins→门禁→轻量调用→装饰层呈现→采纳即编辑；辅助续写 3 候选；局部修改可多轮；失败静默）
source_type: design
chain: AG-ASSIST
rules: BR-AG-STATE-003, BR-AG-VERIFY-001, BR-AG-DATA-002, BR-AG-DATA-007
initial_state: Idle
terminal_states: Accepted, Dismissed
-->

- **machine_id**：SM-AG-03　**owner_module**：AG
- **scope**：一次局部修改或辅助续写回合（触发→候选处置）；触发源与候选形态归 context
  （assist_kind: autocomplete / inline-assist），生命周期同构
- **context**：assist_kind、requestId（latest-wins）、触发上下文（DocRef、编辑位置/选区、目标
  blockId、字符锚、原文本/前缀文本、前后文指纹）、防抖窗口（默认 450ms）、门禁结果、候选（autocomplete=3 candidates；
  inline-assist=preview）、多轮基底（局部修改：上轮候选产出）
- **states**：Idle、Throttling、Requesting、Presenting、Accepted、Dismissed
- **events**：trigger、throttlePass、throttleDrop、candidateOk、candidateUnderflow、requestFail、accept、dismiss、anchorLost、reposition、**refine**
- **transitions（主表）**：

| from | event | guard | to | action |
|---|---|---|---|---|
| Idle | trigger | 用户快捷键手动触发（2026-07-04 澄清：非键入自动触发） | Throttling | 进入防抖/防重判定（BR-AG-STATE-003），捕获 DocRef/blockId/字符锚/前后文指纹 |
| Throttling | trigger | 新请求到达 | Throttling | 更新 requestId 与锚点，旧请求标记 superseded |
| Requesting | trigger | 新请求到达 | Throttling | 更新 requestId 与锚点，旧响应返回后丢弃 |
| Presenting | trigger | 新请求到达/继续输入 | Throttling | 复位旧装饰，更新 requestId 与锚点 |
| Throttling | throttleDrop | 防抖窗口内重复触发且不替换最新请求 | Idle | 丢弃本次触发（静默） |
| Throttling | throttlePass | 防抖通过 且 门禁可用（SM-SYS-01 GatePassed） | Requesting | 经网关轻量调用 |
| Throttling | throttlePass | 防抖通过 但 无订阅/断网 | Idle | 静默不提供，不打断编辑 |
| Requesting | candidateOk | requestId 为最新 且 文档/blockId/字符锚/前后文指纹仍有效；autocomplete 候选数=3 | Presenting | 候选经 ED 装饰层/overlay 语义呈现（ED-DECOR） |
| Requesting | candidateOk | requestId 已过期 或 锚失效 | Dismissed | 丢弃候选，不展示不应用 |
| Requesting | dismiss | 用户点击浮层外、移动光标或改变选区 | Dismissed | 关闭浮层；后续响应按旧 requestId 丢弃，不展示不应用 |
| Requesting | candidateUnderflow | autocomplete 经容错解析后候选不足 3 条（failure_reason=parse_underflow） | Presenting | 不展示半候选、不允许采纳；保留轻量空态，等待用户 dismiss 或移动锚点关闭 |
| Requesting | requestFail | — | Idle | 静默（不弹错误、不打断编辑） |
| Presenting | **refine** | assist_kind=inline-assist（局部修改多轮，2026-07-04 澄清） | Requesting | 以上轮候选产出为基底携新指令续调（仅传上一轮结果；每轮逐次过门禁；对话不持久化） |
| Presenting | reposition | 滚动/缩放/窗口 resize 且逻辑锚仍有效 | Presenting | 基于当前缩放后的视觉锚点与静态浮层尺寸重算浮层位置；浮层 UI 尺寸不随正文缩放 |
| Presenting | reposition | 逻辑锚结构失效 | Dismissed | 关闭浮层并回写 anchorLost |
| Presenting | accept | requestId 为最新、逻辑锚有效，且 autocomplete 恰有 3 个有效候选或 inline-assist 有有效 preview | Accepted | 采纳=用户编辑操作：经 ED 受控接口进入编辑模型，驱动 SM-ED-01 edit（X-INTER-001） |
| Presenting | dismiss | 用户忽略/关闭弹窗、点击浮层外、移动光标或改变选区 | Dismissed | 装饰复位不残留；已返回内容丢弃，不入历史 |
| Presenting | anchorLost | blockId 失效 | Dismissed | 放弃呈现，装饰复位 |

- **error_states**：无显式错误终态——辅助回合失败不弹错、不打断；gate_denied/gate_offline/gateway_error/empty_output 静默回 Idle，parse_underflow 经 candidateUnderflow 进入轻量空态 Presenting 以避免触发后瞬间消失
- **recovery_paths**：失败静默回 Idle，下次触发自然重试
- **forbidden_transitions**：
  - Idle / Throttling → Requesting 无门禁可用（不经门禁不得调用，BR-AG-VERIFY-001）
  - Requesting / Presenting → 直接修改文档模型（必经 accept 走 ED 受控接口，X-FORBID-005）
  - Idle → Presenting（未经调用不得呈现）
  - 过期 requestId 的 candidateOk → Presenting（旧请求不得覆盖新请求）
  - candidateUnderflow 形成的轻量空态 → Accepted（无有效候选不得采纳）
- **acceptance_criteria**：重复触发被防抖（默认 450ms 内不产生多次调用）；无订阅/断网/失败均静默且
  编辑不受影响；辅助续写严格呈现 3 候选并按 Tab/1/2/3 采纳，候选不足不展示半候选且保留空态；新请求覆盖旧请求，旧响应不得展示或应用；
  局部修改选区态替换、光标态插入；refine 每轮均过门禁且只传上一轮候选结果，回合不入历史（不持久化）；
  编辑器缩放、滚动、窗口 resize 不改变逻辑锚，只重算浮层视觉锚；浮层 UI 尺寸不随正文缩放；
  浮层使用静态尺寸模型，避免运行时测量反向驱动状态；局部修改初始态尺寸不得过大导致远离锚点；
  用户点击浮层外、移动光标或改变选区时，Requesting/Presenting 均关闭并丢弃已返回/后续返回内容，不展示、不应用、不入历史；
  accept 产生的变更走 SM-ED-01→SM-WS-01 正常脏态/保存路径；accept/dismiss/anchorLost 均回写终态；
  终态后装饰复位不残留
- **test_matrix**：见 §5

## §5 测试与验收（强制转移测试矩阵，APC §6/§10）

- **SM-AG-01**：send→Assembling→AwaitingGate→gatePassed→Streaming→modelFinal→Finalizing→Completed
  正路；gateDenied→Denied 终态且不可达 Streaming；gateFailed→TurnError，首轮/工具续轮均不发送下一模型请求、保留既有证据、只写一个 error；cancel→Cancelled 无半落盘；finalizeFail/
  streamFail→TurnError 后原 AgentTurn 保持 error，用户显式重试须新建 AgentTurn 并从 Idle/send 启动；禁止 Idle/Assembling→Streaming 旁路；持久化投影覆盖
  completed/error/cancelled/gate_rejected/draft，draft 重启后不得显示为完成态；已打开文档缺
  OpenDocSnapshot 或 selection 校验失败时 assembleFail，不得进入 AwaitingGate。
- **SM-AG-01 1.0 加固**：本地 Passed 常量不得触发 Streaming；模型 blocking ambiguity 走
  `modelClarification→AwaitingClarification`、等待期后续网关调用为零，用户补充后须由原 AgentTurn 走
  `clarificationProvided→Assembling`；Gateway Error 后不得出现 Completed；cancel 必须触发
  CancellationToken 且后续工具调用数为零；terminal_latch 拒绝第二终态。
- **SM-AG-02**：toolRequest→当前授权计算/校验→执行→结果记录→回 SM-AG-01 重新装配/门禁；validateFail 记录拒绝；loopLimit 强制
  中止；cancel 随时中止；断言无任何工具执行产生直接文件写入（与 DE/WS G5 联合验证）；
  read_doc/search_text/list_blocks 已打开文档覆盖 LogicalState，未打开文档读取 DiskState。
  已注册但不在当前 CapabilityGrant 的工具必须 validateFail；状态事件顺序必须先 ToolExecuting、后真实执行、
  再 ResultRecording，不得在工具完成后补发状态；同批多 ToolCall 中间项必须经 nextToolCall 回到 ModelStreaming，
  最后一项才 recorded 完成，不得直接赋值状态；IntentResult/TaskPlan 不能使工具从拒绝变为允许；每次
  ToolResult 后的 provider continuation 必须重新取得真实 GatePassed，不能复用首次或上一轮门禁结果。
- **SM-AG-03**：重复 trigger 下 throttleDrop 或 latest-wins 覆盖生效；无订阅/断网 throttlePass 后静默回
  Idle；过期 requestId 的 candidateOk 被丢弃；候选不足 3 条 candidateUnderflow 且记录 parse_underflow；candidateOk→Presenting→accept→Accepted
  驱动 SM-ED-01 edit；辅助续写 3 候选采纳；局部修改光标插入；Presenting→refine→Requesting 多轮循环（只传上一轮
  候选结果，每轮过门禁、不持久化）；anchorLost/dismiss 回写终态且复位不残留；禁止未经门禁到 Requesting。
- **跨模块（G5 联调）**：AG↔SM-SYS-01 门禁转移、SM-AG-02 propose_edit↔DE、SM-AG-03 accept↔SM-ED-01 edit；
  完成等级只由验证文档与真实证据判定，不在状态机正文宣称。
- 量化 NFR（节流参数、延迟）延 G5 标定。

## §6 总图登记（A-SYS-M-S-01）

| 核心链 | machine_id | 总图回填动作 |
|---|---|---|
| 代理回合 | SM-AG-01 | §3 已登记 active |
| tool-call | SM-AG-02 | §3 已登记 active |
| autocomplete | SM-AG-03 | §3 已登记 active（与 inline-assist 同机承载，context 区分） |
| inline-assist | SM-AG-03 | §3 已登记 active（同上） |
| TaskRun 可观察投影 | SM-AG-04 | §3.1 登记 active；简单 AgentTurn 不进入；不得承载调度/授权 |
| 转移点 | — | §5 X-INTER-001 登记：AG↔门禁（SM-SYS-01）、SM-AG-02 Proposal 工具→DE、SM-AG-03 采纳→SM-ED-01 |

## §7 术语归属说明

代理回合/工具调用/局部修改/辅助续写 TERM 已在 A-AG-M-T-01 §0 注册；订阅门禁等云术语归 SYS
（TERM-CLOUD-001~004）；diff 主责归 DE。状态机框架词不注册。

## §8 技术债务与盲区（诚实声明）

- **AG Proposal→DE 系统验证**：SM-AG-02 `propose_edit` 契约已定（修改建议+定位锚交 DE，回执 ToolResult + DiffProposed/失败）；
  是否完成双侧真实联调只由 A-VAL-X-V-04 与实际证据判定，本文不持有完成等级。
- **P2 Build/CBT**：不属于本状态机设计；未来独立 Profile 不得复用已从 SM-AG-01 移除的历史 takeover/
  HandedOver/fallbackIn 语义，必须重新经过需求、设计与验证。
- **SM-AG-02 嵌套语义**：与 SM-AG-01 Streaming 段的嵌套激活/取消传播在实现期细化（单实例语义
  已锁，嵌套调度属实现）。
- 节流窗口、循环上限数值延 G5/运营标定（机制已锁）。

## §9 后续设计入口

1. 总图只登记 SM-AG-01/02/03/04 及当前跨模块转移；不得保留 `agent-edit-1.0` 到 CBT 的 active 转移。
2. REQ-AG-010/012/014/016 若形成新的状态化链路，须在各自 DESIGN Scope 建机并登记；当前不预建状态。
3. P2 Build/CBT 只有在人类确认新的 RuntimeProfile 需求后，才可设计新的跨模块状态机。

### 正常停止与交付完整性说明（2026-10-25）

`done.stopReason=answer_incomplete`仍走已有Finalizing→Completed，表示该AgentTurn已结束且存在正文结构未闭合风险，不是网络失败、业务成功或可执行恢复凭证。保留原供应商stop原因和正文摘要的内部TurnEvent；前端历史恢复展示相同风险，不自动调用模型或工具。句末没有标点不构成该风险规则，结构检查不保证任意语义残缺均可检测。

裁决恢复的confirm/cancel仍只解除原等待条件。恢复模型必须消费同身份后端资源回执及接受/保存事实；资源步骤成功不强制TaskRun整体Completed，真正partial/blocked及未知效果继续保持原不变量。

## §6 链路边表（内部转移与跨模块 handoff）

本表是规则链审计的边清单，不替代 §2/§3/§4 主转移表；`rules` 列给出该边的最小规则证据。跨模块边以 `consumer=` 规则表达，不并入 AG 状态机 `rules` 闭包。

| edge_id | from | event | to | rules | owner |
|---|---|---|---|---|---|
| EDGE-AG-001 | SM-AG-01 Idle | send | Assembling | BR-AG-STATE-001 | turn_runtime |
| EDGE-AG-002 | SM-AG-01 Assembling | assembled | AwaitingGate | BR-AG-STATE-001 | turn_runtime |
| EDGE-AG-003 | SM-AG-01 AwaitingGate | gatePassed | Streaming | BR-AG-STATE-001, BR-SYS-VERIFY-001 | await_round_gate |
| EDGE-AG-004 | SM-AG-01 Streaming | toolRequest | SM-AG-02 ToolValidating | BR-AG-STATE-002 | run_one_tool_call |
| EDGE-AG-005 | SM-AG-02 ResultRecording | recorded | SM-AG-01 Assembling | BR-AG-STATE-001, BR-AG-STATE-002 | run_round_loop |
| EDGE-AG-006 | SM-AG-01 Streaming | modelFinal | Finalizing | BR-AG-STATE-001 | handle_done_round |
| EDGE-AG-007 | SM-AG-01 Streaming | modelClarification | AwaitingClarification | BR-AG-STATE-001, BR-SYS-STATE-007 | park_clarification |
| EDGE-AG-008 | SM-AG-01 Streaming | decisionProposed | AwaitingDecision | BR-AG-STATE-001, BR-SYS-STATE-008 | park_decision_handoff |
| EDGE-AG-009 | SM-AG-01 active | cancel | Cancelled | BR-AG-STATE-001, BR-SYS-STATE-007 | cancel_between_rounds |
| EDGE-AG-010 | AG-ASSIST accept | accept | ED-EDIT apply | BR-AG-STATE-003, BR-ED-STATE-001 | applyInline |
| EDGE-AG-011 | AG-TURN AwaitingGate | gate verdict | SYS CLOUD-GATE | BR-AG-STATE-001, BR-SYS-VERIFY-001 | await_round_gate |
| EDGE-AG-012 | AG AwaitingDecision | decisionResolved(confirm) | WS-OPERATION revalidate | BR-SYS-STATE-008, BR-WS-STATE-003 | task_runtime |
| EDGE-AG-013 | AG ToolResult(proposal) | proposalRecorded | DE-DIFF pending_diff | BR-AG-STATE-002, BR-DE-DATA-001 | edit_runtime |
| EDGE-AG-014 | SM-AG-04 Projected/Active | terminalObserved | TaskRun terminal | BR-SYS-STATE-008, BR-AG-STATE-001 | task_runtime |

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-10-30 | v2.37 | 补齐 SM-AG-02 同批多 ToolCall 转移：新增 nextToolCall（ResultRecording→ModelStreaming），中间项保持同一激活，最后一项 recorded 完成；明确每次 provider continuation 经 SM-AG-01 Gate 后开启新激活；禁止直接赋值状态绕过转移引擎。 |
| 2026-10-25 | v2.36 | 正常停止与正文结构风险分离，保留既有Completed转移及TaskRun真实未完成语义；裁决恢复消费真实效果。 |
| 2026-10-24 | v2.35 | 单独创建与组合操作共用原保存等待；预览续片段不生成用户消息，绑定原请求/模型和入场新快照，不复活旧终态。 |
| 2026-10-24 | v2.34 | 完整工作流：保存等待不以TaskRun状态授予权限；operationFailed收尾、拒绝取消及核心失败说明与唯一终态分开。 |
| 2026-11-01 | v2.39 | PI-AG-TASKRUN-24：内容提案前建 TaskRun；partial 保持 active；checkpoint 与提案同段建立。 |
| 2026-10-31 | v2.38 | PI-AG-LIMIT-REMOVE-14：删除执行额度与 budget_exhausted；回合仅由 final/clarify/error/cancel 结束，保留副作用与时间边界。 |
| 2026-10-23 | v2.33 | 资源交接前置校验、额度冻结和工具清空收尾；budget_exhausted仅结束执行，不提升TaskRun业务完成。 |
| 2026-10-22 | v2.32 | 参数跨字段预检、重复只读失败受约束收尾、native清单冻结及纯只读用户澄清恢复；保留原累计预算和非只读防重放。 |
| 2026-10-22 | v2.31 | 响应协议拒绝在终态前回装配/Gate；登记 SM-AG-01 modelProtocolRejected、SM-AG-02 protocolRejected、同回合纠错与模型预算、防重放边界。 |
| 2026-10-06 | v2.30 | 人类确认上游门禁契约收敛：离线/超时/非法响应进入 gateFailed→TurnError，不冒充服务端拒绝；合法拒绝保持 Denied。 |
| 2026-10-06 | v2.29 | 区分门禁调用 Err 与既有 GateOutcome::Deny，新增 gateFailed→TurnError；不修改 ENT 将离线转为 Deny 的现有契约，该差异单列待收敛。 |
| 2026-09-15 | v2.28 | P00：定义 v2 身份先行、实际内容使用前校验的准入转移；保留 v1 严格路径，明确裁决身份、异步回执、取消/断流及效果未知的边界，待代码接入验证。 |
| 2026-09-03 | v2.27 | AG-ADAPTIVE-RUNTIME-DESIGN-01：SM-AG-01 改为 append-only TurnEvent 驱动的模型/工具自适应循环，新增 AwaitingDecision 与 Finalizing；修改建议只由 SM-AG-02 `propose_edit` 交 DE；SM-AG-02 每次按硬事实计算 CapabilityGrant 并回上层统一重新 Gate；SM-AG-04 改为纯 TaskRun 投影机。 |
| 2026-06-11 | v1.0 | 初版（AG G2 状态机件）：定义 SM-AG-01 代理回合机（组装/门禁/流式/HandingOff 交 DE，Denied 终态不可绕过）、SM-AG-02 工具调用循环机（校验/执行/回注/上限/写盘禁令，嵌套于回合 Streaming 段）、SM-AG-03 辅助回合机（autocomplete 与 inline-assist 同机承载：节流/门禁/装饰层/采纳即编辑/失败静默），均含 APC §6 必备字段；满足 X-SEQ-003；转移点登记（门禁/交 DE 单侧/采纳→SM-ED-01 双侧）；债务（DE 侧待 G2、CBT 接管未建模、嵌套调度与参数延实现期） |
| 2026-07-04 | v2.0 | 双模式定义回写（人类裁决）：①SM-AG-01 takeover guard 判据换轴——「识别多文件构建意图」废止，改「构建触发：显式触发（指令/构建按钮，触发即确认）或项目级构建意图提议经确认」，移交携项目蓝图、构建在原对话窗口呈现；「Chat↔Build 接管」更名「构建移交/回落」；新增构建执行期禁发起新回合（前端 X-FORBID-007 投影门控）；②SM-AG-03：触发语义改快捷键手动+防抖/防重（原「节流」为自动触发遗留），新增 Presenting→refine→Requesting 多轮转移（局部修改，每轮过门禁、不持久化），test_matrix 同步 |
| 2026-07-16 | v2.1 | 细化 SM-AG-03：引入 requestId latest-wins、新请求覆盖旧请求、过期响应丢弃；辅助续写 3 候选；局部修改光标态插入，多轮只传上一轮候选结果；candidateOk 与 accept 前均以文档/block/字符锚有效为守卫 |
| 2026-07-16 | v2.2 | 续修 SM-AG-03：补默认 450ms 防抖、触发锚前后文指纹、候选不足 3 条静默失败、accept/dismiss/anchorLost 前端终态回写 |
| 2026-07-16 | v2.3 | 补充 SM-AG-03 交互锚失效投影：Requesting/Presenting 状态下点击浮层外、移动光标或改变选区均走 dismiss 终态，关闭浮层并丢弃已返回/后续返回内容 |
| 2026-07-16 | v2.4 | 补充 SM-AG-03 reposition 转移：滚动、缩放、窗口 resize 时逻辑锚不变，按当前视觉锚重算浮层位置，结构失效才关闭 |
| 2026-07-16 | v2.5 | 修正 SM-AG-03 稳定定位：撤销浮层 DOM 尺寸变化触发 reposition，改为静态尺寸模型，避免测量链影响浮层显示 |
| 2026-07-16 | v2.6 | 修正 SM-AG-03 辅助续写失败态：候选不足 3 条转为空态 Presenting，不展示半候选、不允许采纳，用户 dismiss 后关闭 |
| 2026-07-17 | v2.7 | 加固 SM-AG-03 失败分型：gate/网关/空输出静默回 Idle，parse_underflow 进入轻量空态；failure_reason 仅用于内部诊断 |
| 2026-07-17 | v2.8 | 清理 SM-AG-03 状态分叉：候选不足改用 candidateUnderflow 独立事件进入 Presenting，普通 requestFail 仅表示静默回 Idle |
| 2026-07-17 | v2.9 | 补齐 SM-AG-01 历史持久化投影：send 先写 draft，handoffOk→completed，streamFail/handoffFail→error，cancel→cancelled，gateDenied→gate_rejected；崩溃/关闭遗留 draft 重启后不得伪装完成 |
| 2026-07-18 | v2.10 | 加固 SM-AG-01/02 已打开文档门禁：send 阶段必须携 open_doc_snapshots 并构建 AgentReadContext；只读工具对已打开文档禁止 DiskState 兜底 |
| 2026-08-08 | v2.11 | 对齐 agent-edit-1.0：SM-AG-01 context 增 IntentResult/PromptPackage/CancellationToken/terminal_latch，GatePassed 限本次服务端 GateOutcome，新增 blocking ambiguity 无模型澄清路径与唯一终态；SM-AG-02 按 BR-AG-STATE-002 增本轮 allowed_tools 守卫和真实执行时序；1.0 profile 禁止 takeover |
| 2026-08-14 | v2.12 | AG-1.0-D3-AG-REF-AST-RUNTIME-CONTRACT-01：新增 §2.1 澄清非终态与首终态契约（REQ-AG-007 / BR-SYS-STATE-007——澄清为等待输入的非终态 AwaitingClarification、任一活动阶段取消、terminal_latch 首终态、迟到事件无副作用），登记 clarificationRequired→Completed 迁移债务；新增 §2.2 SM-AG-04 TaskRun 编排机（REQ-AG-015 / BR-SYS-STATE-008，Planned/Executing/WaitingConfirm/WaitingClarification/Completed/Failed/Cancelled/Interrupted，非后台队列、不自动重试/续跑/越确认点、中断可重试）；补 REQ-AG-007/015 的 REQ_MAP。 |
| 2026-08-14 | v2.13 | AG-1.0-DOC-CLOSURE-DCLOSURE-01：§2.1 迁移债务标记关闭（已解决 · IMP-CLARIFICATION-01 落地——turn.rs AwaitingClarification 非终态 + 前端 clarifying + agRuntime 澄清非终态测试）。 |
| 2026-08-14 | v2.14 | AG-1.0-IMP-TASKRUN-01：SM-AG-04 落地（运行时/数据层）——ag/task.rs 转移引擎（Planned/Executing/WaitingConfirm/WaitingClarification/Completed/Failed/Cancelled/Interrupted + steps 六级态，主表正路 + forbidden）+ TaskRun 生命周期（begin_turn 每回合建/沿用、提案卡=W1/W2/W3 确认点、settle 回合终态映射、mark_stale 重启置 Interrupted 不续跑）+ task_runs 表持久化（db v11）+ agent_turns.task_run_id 关联 + DiffProposed 事件带 task_run_id（UNC-09 数据层）；前端 DiffView.taskRunId 透传无 UI；5 个测试。 |
| 2026-08-15 | v2.15 | AG-1.0-IMP-TASKRUN-UI-01：SM-AG-04 前端呈现——ag.list_task_runs 查询 IPC（按会话倒序含步骤）+ agService listSessionTaskRuns/refreshSessionTaskRuns（回合 done/error 后 + 历史恢复刷新）+ agStore.taskRuns 投影 + TaskRunCard（状态徽标/步骤/确认点高亮/进度，只读不伪造，BR-SYS-STATE-008）挂 MessageList 顶部；3 测试。 |
| 2026-08-15 | v2.16 | AG-1.0-TASKRUN-REWORK-REQ-01：§2.2.1 语义纠偏——三层分离（DiffCard 内容层 / DecisionRequest 任务级裁决 / TaskRun 只响应明确任务裁决）、WaitingConfirm 唯一来源=DecisionRequest、userConfirm/userCancel 真实裁决入口、新消息不绕过确认 gate、TaskPlan（复杂目标，可展示计划）、执行片段每步重新授权 + 确认后自动下一片段、同文档连续 Diff 前卡失效。 |
| 2026-08-15 | v2.17 | AG-1.0-TASKRUN-REWORK-DESIGN-01：§2.2.2 SM-AG-04 重构设计——转移主表重构（planApproved/segmentStarted/decisionProposed/userConfirm/userCancel/finish）、forbidden 重构（新消息 gate、WaitingConfirm 仅 DecisionRequest 裁决推进、propose_edit 不再置 waiting_confirm）、每片段重新授权 guard。 |
| 2026-08-15 | v2.18 | AG-1.0-TASKRUN-REWORK-IMP-RUNTIME-01：SM-AG-04 重构落地（运行时/数据层）——settle 按未决 DecisionRequest 判定 WaitingConfirm（去除 has_pending_proposal）、begin_turn 不绕回 waiting_confirm、回合入口 gate（AG_DECISION_PENDING）、propose_decision/resolve_decision 生命周期与 IPC、TaskPlan 持久化、StepProgress 实时事件。 |
| 2026-08-21 | v2.19 | 工作区资源操作基线重构：SM-AG-04 增加 WorkspaceOperation 集成语义；DecisionRequest 确认后立即调用 SM-WS-04 执行，不增加 WaitingUserCommit 或资源 DiffCard；CreateDocumentBundle 初始内容 Diff 不驱动 TaskRun 等待；重启中断不得复用旧确认。 |
| 2026-08-27 | v2.20 | AG-CONFORMANCE-GOVERNANCE-S1：将 SM-AG-01 澄清唯一化为 AwaitingClarification 非终态，用户补充后原 AgentTurn 回到 Assembling；TurnError 保持原 AgentTurn 终态，显式重试创建新 AgentTurn；同步测试矩阵。将 SM-AG-04 限定为复杂或跨执行片段目标，WaitingConfirm 唯一来源为 DecisionRequest；删除 active 正文中重复的旧语义修正段。 |
| 2026-08-29 | v2.21 | AG-CONFORMANCE-CODE-S3E / HD-S3E-02：补齐 SM-AG-02 生产事件语义——新增 loopError→LoopAborted 与 ResultInjecting handoff→LoopCompleted；冻结同轮多 ToolCall 按单调用顺序驱动，校验失败不进入 ToolExecuting；禁止绕过转移引擎广播以及 LoopAborted 映射 Completed/Done。 |
| 2026-08-31 | v2.22 | AG-DOC-CONSOLIDATION-01：当前状态机清单唯一化为 SM-AG-01～04；移除 Agent 1.0 中 Build 接管/回落转移；SM-AG-02 每次 provider continuation 必须重新 Gate，SM-AG-03 空候选不可采纳，完成回合不产生自动记忆副作用。 |
