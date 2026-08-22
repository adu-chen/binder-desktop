---
文档编号：AG-M-S-01
文档状态：A
负责模块：AG
文档职责：AG 代理模块级状态机设计——代理回合机（SM-AG-01）、工具调用循环机（SM-AG-02）、辅助回合机（SM-AG-03，承载 autocomplete 与 inline-assist），含 APC §6 必备字段；回填 A-SYS-M-S-01 总图四条 planned 行，登记 AG↔DE / AG↔ED / AG↔SYS 门禁 X-INTER-001 转移点
上游约束：A-AG-M-T-01_代理技术设计.md（AG-TURN/AG-TOOL/AG-ASSIST 链路、BR-AG-* 规则）、A-CORE-C-P-01_APC.md（§6）、A-SYS-M-S-01_状态机总图.md（X-SEQ-003/X-INTER-001）、A-SYS-I-P-03_云接口契约.md（SM-SYS-01 门禁段）、A-SYS-I-P-02_IPC协议.md（BR-SYS-STATE-002 流式终态）、A-ED-M-S-01_编辑器状态机设计.md（SM-ED-01 edit 入口）、ADU.md（§3.1）
直接承接：三机经 rules 字段承载 A-AG-M-T-01 规则；machine_id 登记总图；产出交 DE 转移点供 DE G2 对接
使用边界：本文定义 AG 状态机内部；不重定义 SM-SYS-01（门禁裁决归 SYS，本文以转移点对接）、不定义 DE diff 终态机（待 DE G2）、不定义 ED/WS 机内部；构建移交/回落状态机对接已双侧闭合（§2 takeover/fallbackIn + A-CBT-M-S-01 §3）
变更要求：transitions 变更须同步 A-AG-M-T-01 规则并重跑测试矩阵；跨模块转移点变更须与对侧 owner 共同复核；machine_id 一经登记不可更改
治理成熟度：draft
---

# Binder 代理状态机设计（AG · L2 技术设计）

## §1 文档定位

```
A-AG-M-T-01（规则 SSOT）→ 本文 SM-AG-01/02/03
本文 machine_id ──登记──→ A-SYS-M-S-01 §3（代理回合 / tool-call / autocomplete / inline-assist 四行转 active）
转移点：AG↔SM-SYS-01（门禁）、AG 产出↔DE（待 DE G2）、SM-AG-03 采纳↔SM-ED-01（双侧就绪）
```

满足 X-SEQ-003：四条状态化核心链（APC §6 列举 + 2026-06-11 归属决策）在此建机。
autocomplete 与 inline-assist 由 **SM-AG-03 同机承载**（二者生命周期同构：触发→节流→门禁→
轻量调用→装饰层呈现→采纳/弃，区别仅在触发源与候选形态，归 context 字段），总图两行回填同一
machine_id 并注明。

## §2 SM-AG-01 · 代理回合机

<!-- STATE_MACHINE
machine_id: SM-AG-01
owner_module: AG
status: active
summary: 一次代理回合的完整生命周期（组装→门禁→流式→产出交 DE；可中断、无半落盘）
source_type: design
chain: AG-TURN
rules: BR-AG-STATE-001, BR-AG-VERIFY-001, BR-AG-DATA-001, BR-AG-DATA-002, BR-AG-DATA-004, BR-AG-DATA-005, BR-AG-PERSIST-001
initial_state: Idle
terminal_states: Completed, Cancelled, Denied, HandedOver
-->

- **machine_id**：SM-AG-01　**owner_module**：AG
- **scope**：一次代理回合（用户发起→产出处置完成/中断/拒绝/移交构建；目标边界按 BR-AG-DATA-003 2026-07-04 版）
- **context**：回合输入（session_id + runtime_profile + prompt + 注入载荷 + open_doc_snapshots）、IntentResult、AgentReadContext、PromptPackage、真实 GateOutcome、CancellationToken、terminal_latch、流式缓冲、产出（回复/修改建议）、移交上下文、历史持久化状态（completed/error/cancelled/gate_rejected/draft）
- **states**：Idle、Assembling、AwaitingGate、Streaming、HandingOff、Completed、Cancelled、Denied、TurnError、**HandedOver**
- **events**：send、assembled、clarificationRequired、assembleFail、gatePassed、gateDenied、streamEnd、streamFail、cancel、handoffOk、handoffFail、retry、**takeover、fallbackIn**
- **transitions（主表）**：

| from | event | guard | to | action |
|---|---|---|---|---|
| Idle | send | open_doc_snapshots 覆盖全部已打开文档且每项含 session_version/content_hash；active live 序列化成功；同 session 无其他活动回合 | Assembling | 写入/更新 AgentSession；创建 AgentTurn `draft` payload，记录 userMessage（BR-AG-PERSIST-001）；形成 IntentResult；构建 AgentReadContext、ContextBundle 与 PromptPackage |
| Idle | fallbackIn | CBT 构建完成/中断回落（cbt.fallback，X-INTER-001） | Assembling | 以回落上下文起新 代理回合；回 fallbackAck（无双死） |
| Assembling | clarificationRequired | IntentResult.ambiguity=blocking | Completed | 输出结构化澄清问题并锁定 completed；不调用门禁、网关或工具，不产生修改建议 |
| Assembling | assembled | IntentResult 非阻断；已打开文档引用和 selection 均基于 AgentReadContext 校验通过；PromptPackage 角色/预算/作用域校验通过 | AwaitingGate | 提交 SM-SYS-01 门禁（X-INTER-001 转移点） |
| Assembling | assembleFail | OpenDocSnapshot 缺失/版本缺失/active live 序列化失败/注入锚失效 | TurnError | 统一错误形状；不得用 DiskState 或 loadedContent 旧快照兜底 |
| AwaitingGate | gatePassed | 本次服务端 GateOutcome=GatePassed 且 verdict_id 可追踪 | Streaming | 经 AiGateway 发起流式调用；本地常量/缓存/桩值不得触发本转移 |
| AwaitingGate | gateDenied | SM-SYS-01 GateDenied / 断网 | Denied | 明确拒绝（终态，不可绕过）；AgentTurn status=`gate_rejected` |
| Streaming | streamEnd | — | HandingOff | 产出处置：修改建议交 DE，纯回复直接完成 |
| Streaming | takeover | 构建触发（2026-07-04 判据换轴，BR-CBT-STATE-002）：用户显式触发（明确指令/构建按钮，触发即视为确认）或 AI 识别**项目级构建意图**提议经用户确认；多文件修改本身不再是判据 | HandedOver | 移交 CBT（cbt.takeover，上下文含项目蓝图随移交）；回 takeoverAck；本回合移交终结（无双活）；构建在原对话窗口呈现 |
| Streaming | streamFail | — | TurnError | 统一错误形状（含可重试标记）；AgentTurn status=`error` |
| Streaming | cancel | terminal_latch 尚未锁定 | Cancelled | 触发 CancellationToken，终止流式请求、阻止新工具启动并锁定唯一终态；无半落盘副作用；AgentTurn status=`cancelled` |
| HandingOff | handoffOk | 修改建议已交 DE 生成 pending_diff / 无修改建议 | Completed | AgentTurn status=`completed`；异步触发记忆写入（BR-AG-STATE-004） |
| HandingOff | handoffFail | — | TurnError | DE 生成失败（如锚失效）回报，可重试；AgentTurn status=`error` |
| TurnError | retry | — | Assembling | 重试回合 |

- **error_states**：TurnError（recovery：retry→Assembling）
- **recovery_paths**：TurnError→retry；Denied 不可重试绕过（须恢复订阅/网络后重新 send）
- **forbidden_transitions**：
  - Idle / Assembling → Streaming（未过门禁不得调用，对齐 SM-SYS-01 forbidden）
  - Denied → Streaming（被拒不得绕过门禁，终态）
  - Streaming → Completed（产出含修改建议时必经 HandingOff 交 DE，不得跳过）
  - Assembling → AwaitingGate 当已打开文档缺少 OpenDocSnapshot 或引用锚无法校验（不得以 DiskState 兜底进入模型）
  - 任意态 → 直接写盘（本机无写盘动作，X-FORBID-006）
  - takeover 无显式触发/确认（构建移交须用户显式触发或确认提议，BR-CBT-STATE-002）
  - HandedOver → 任意活动态（移交终结后本回合不得继续，无双活）
  - Completed / Cancelled / Denied / HandedOver / 已持久化 error → 任意其他终态（terminal_latch 只允许首次锁定）
  - `runtime_profile=agent-edit-1.0` → takeover / HandedOver（1.0 不启用代理构建）
- **persistence_projection**：`send` 后先持久化 `draft`；`handoffOk`→`completed`；
  `streamFail/handoffFail/命令级错误`→`error`；`cancel`→`cancelled`；`gateDenied`→`gate_rejected`；
  应用崩溃/关闭/流式中断未收到终态时保持 `draft`，重启后只能显示未完成/可重试。
- **acceptance_criteria**：不存在不经真实服务端 gatePassed 通往 Streaming 的路径；Denied 为终态且不可达
  Streaming；修改建议唯一出口经 HandingOff 交 DE；cancel 后无半落盘副作用；takeover 须显式触发
  或经确认且移交后本回合终结（HandedOver 终态，无双活）；fallbackIn 起新回合时回 fallbackAck（无双死）
  ；completed/error/cancelled/gate_rejected/draft 五类历史状态均可重启恢复；每个 turn 最多锁定一个
  用户可见终态，网关 Error/Denied/Cancelled 后不得再出现 Done/Completed；blocking ambiguity 不发起模型调用。
- **test_matrix**：见 §5
- **构建移交/回落（旧称 Chat↔Build 接管；B-3 补齐 2026-06-12，判据 2026-07-04 换轴）**：takeover
  （显式触发或项目级构建意图经确认→HandedOver 终态，移交 CBT，蓝图随移交）/ fallbackIn（CBT 构建
  完成/中断回落起新回合）与 SM-CBT-01（A-CBT-M-S-01 §3）双向回执对接（takeoverAck/fallbackAck），
  登记 A-SYS-M-S-01 §5。构建执行期间（SM-CBT-01 活动态）不得发起新代理回合（仅中断），由前端
  按 X-FORBID-007 投影门控。

### 2.1 澄清非终态与首终态契约（REQ-AG-007 / BR-SYS-STATE-007，2026-08-14 D3 登记）

承接冻结基线 R5-A（A-CORE-X-D-02 §4.2）与 BR-SYS-STATE-007（D1，A-SYS-I-P-01 §3.1）：

| 场景 | SM-AG-01 语义 | 说明 |
|---|---|---|
| 澄清等待 | `IntentResult.ambiguity=blocking` 时回合进入**等待用户输入的非终态**（AwaitingClarification），**不是 Completed** | 澄清期间不调用门禁/网关/工具、不产生修改建议、不锁定终态事件（BR-SYS-STATE-007） |
| 澄清后继续 | 用户补充输入后重新组装（回 Assembling）并再次收敛 IntentResult；或按新目标生成新回合 | 不把澄清伪装为完成（BR-AG-STATE-001） |
| 任一活动阶段取消 | 取消可在澄清等待、组装、门禁、流式等任一活动阶段生效 | 无半落盘副作用 |
| 首终态锁存 | 完成/失败/拒绝/取消互斥，`terminal_latch` 首个终态获胜 | 首终态后到达的迟到事件/工具不得产生文档、卡片或落盘副作用 |
| 结构化可见事件 | 用户只见意图结论、读取/工具、提案卡、等待确认与终态原因等结构化事件 | 不展示模型内部推理（StructuredVisibleEvent，A-SYS-I-P-01 §3.1） |

> **迁移债务（已解决 · AG-1.0-IMP-CLARIFICATION-01 落地）**：既有 SM-AG-01 主表曾将
> `clarificationRequired → Completed`（澄清被终结为终态），与冻结基线「澄清是等待用户输入的非终态」
> 不一致；AgentTurnStatus 亦无澄清等待非终态值。实现 Scope（turn.rs `AwaitingClarification` 非终态 +
> 前端 `AgentTurnStatus.clarifying` + agRuntime 澄清非终态测试）已将该债务关闭：澄清建模为
> AwaitingClarification 非终态，澄清期间不调用门禁/网关/工具、不锁定终态事件（BR-SYS-STATE-007）。

### 2.2 SM-AG-04 · TaskRun 编排机（REQ-AG-015 / BR-SYS-STATE-008，2026-08-14 D3 登记）

<!-- STATE_MACHINE
machine_id: SM-AG-04
owner_module: AG
status: active
summary: 受限任务实例（TaskRun）生命周期——同一用户目标内组织有依赖步骤/回合，非后台队列；真实工作状态由步骤/回合/工具事件驱动
source_type: design
chain: AG-TURN
rules: BR-SYS-STATE-008, BR-AG-STATE-001, BR-AG-STATE-002
initial_state: Planned
terminal_states: Completed, Failed, Cancelled, Interrupted
-->

- **machine_id**：SM-AG-04　**owner_module**：AG
- **scope**：一个复杂用户目标被组织为 TaskRun 后的受限编排生命周期（目标计划→逐步执行→确认点→终态）
- **context**：task_run_id、AgentSession、当前用户目标、计划步骤集合、每步 CapabilityGrant、确认点、
  关联 AgentTurn/Proposal、中断与重试状态
- **states**：Planned、Executing、WaitingConfirm、WaitingClarification、Completed、Failed、Cancelled、Interrupted
- **steps**（步骤态，单步内）：pending、running、waiting_confirm、ok、failed、cancelled
- **events**：planApproved、stepStarted、stepOk、stepFail、waitingConfirm、userConfirm、clarificationNeeded、
  userCancel、interrupt、retry、finish
- **transitions（主表）**：

| from | event | guard | to | action |
|---|---|---|---|---|
| Planned | planApproved | 用户确认目标计划；目标/范围/确认点明确 | Executing | 逐步骤调度；每步取得当前最小授权（CapabilityGrant） |
| Executing | stepStarted | 步骤属于当前目标且已授权 | Executing | 真实工作状态由回合/步骤/工具事件驱动（BR-SYS-STATE-008） |
| Executing | stepOk | 步骤只读/研究完成 | Executing | 可连续推进只读步骤 |
| Executing | waitingConfirm | 步骤进入 W1/W2/W3（提案/写入）需用户裁决 | WaitingConfirm | 等待卡片裁决；不伪装为执行 |
| Executing | clarificationNeeded | 目标/范围/副作用不明 | WaitingClarification | 等待用户补充（非终态，复用 SM-AG-01 澄清语义） |
| WaitingConfirm | userConfirm | 用户裁决继续 | Executing | 继续后续步骤 |
| WaitingConfirm | userCancel | 任一活动阶段取消 | Cancelled | 停止未开始步骤；保留已成功预览内容，不产生任务级回滚 |
| Executing | stepFail | 步骤失败 | Failed | 不自动重试；如实报告失败原因 |
| Executing | interrupt | 应用中断/重启 | Interrupted | 显示中断可重试；**不跨重启续跑** |
| Interrupted | retry | 用户显式重试 | Planned | 创建新回合并重新校验（不沿用旧上下文） |
| Executing | finish | 所有步骤完成 | Completed | 终态 |
- **forbidden_transitions**：Executing → Completed 未经 finish；WaitingConfirm/WaitingClarification → 执行（无用户裁决/补充不得推进 W1/W2/W3 步骤）；Interrupted → Executing 无用户重试（不自动续跑）；任一态 → 后台队列（TaskRun 非后台队列，BR-SYS-STATE-008）
- **acceptance_criteria**：TaskRun 只组织当前用户目标内的已授权步骤；不自动重试、不跨重启续跑、不越确认点；
  连续 Proposal 以最新 LogicalState 接力（A-DE-M-S-01 §2.1）；取消不回滚已成功预览内容；重试创建新回合
- **test_matrix**：见 §5 增补
- **总图登记**：machine_id 登记 A-SYS-M-S-01 §3.1（D1 已将 TaskRun 登记为 planned，D3 回填 active）

#### 2.2.1 语义纠偏（2026-08-15 · AG-1.0-TASKRUN-REWORK-REQ-01，替代上文中的实现性表述）

> 落地审计确认：SM-AG-04 被实现收窄为"每回合事后事件记录"（确认点=提案卡、无任务级裁决入口、新消息绕回
> executing）。本节冻结纠偏语义，设计/实现以此为准；与 2.2 表冲突处以本节优先。

- **WaitingConfirm 唯一来源 = DecisionRequest（任务级裁决项）**：不再由 propose_edit 直接产生；
  propose_edit 产生的 Diff 仅属内容层（DiffCard 裁决不驱动 TaskRun 状态）。
- **userConfirm / userCancel 真实入口 = DecisionRequest 裁决**（permission / plan_choice / scope / manual_confirm）：
  由前端裁决动作发 TaskRun 事件，不再是"下一条任意用户消息"。
- **新消息 gate**：存在未决 DecisionRequest 时，新回合入口不得绕过确认（拒绝或仅接受带明确裁决语义的输入）；
  澄清走 SM-AG-01 澄清路径不受此 gate 影响。
- **TaskPlan**：IntentResult 判定复杂目标后创建（可展示操作计划，非模型思维链），保存步骤/依赖/当前游标/
  所需授权/确认点/阶段结果；简单目标单 AgentTurn 直行不建 TaskPlan。
- **执行片段**：TaskRun 跨多执行片段/AgentTurn；每片段按 profile+intent+capability+目标+上下文+既有确认点
  重新收窄；任务级确认后自动开始下一片段，不后台续跑。
- **同文档连续 Diff**：片段内可连续多 Proposal；Diff2 基于 Diff1 成功预览后 LogicalState；Diff1 因基准推进失效。

#### 2.2.2 SM-AG-04 重构设计（2026-08-15 · AG-1.0-TASKRUN-REWORK-DESIGN-01，承接 §2.2.1）

**状态**（保留 8 态；WaitingConfirm 唯一来源 = DecisionRequest）：
`Planned`（可携带 TaskPlan）→ `Executing`（执行片段内推进）→ 确认/完成/失败/中断。

**事件**（增补/语义修正）：

| 事件 | 语义 |
|---|---|
| planApproved | TaskPlan 确认，进入首片段（计划确认可选：复杂目标首片段前可发 DecisionRequest(plan_choice)） |
| segmentStarted | 片段开始；**每片段重新授权**（profile+intent+capability+目标+上下文+既有确认点收窄） |
| decisionProposed | DecisionRequest proposed → WaitingConfirm（唯一进入确认等待的路径） |
| userConfirm / userCancel | DecisionRequest 裁决（真实入口，前端裁决动作发 TaskRun 事件） |
| stepStarted / stepOk / stepFail | 步骤真实事件（保留，驱动真实进度） |
| interrupt / retry / finish | 保留；finish=阶段收敛（有阶段结果）→ Completed |

**转移（重构主表）**：

| from | event | guard | to | action |
|---|---|---|---|---|
| Planned | planApproved | 计划（若有）已确认；目标/范围/授权明确 | Executing | 首片段开始；逐步骤当前最小授权 |
| Executing | segmentStarted | 片段内步骤属于当前目标且已重新授权 | Executing | 真实事件驱动进度（BR-SYS-STATE-008） |
| Executing | decisionProposed | 任务级裁决项（权限/方案/范围/人工确认） | WaitingConfirm | 等待 DecisionRequest 裁决；不伪装工作中 |
| WaitingConfirm | userConfirm | 裁决确认（confirm） | Executing | 自动开始下一执行片段（无需新聊天消息；重新授权；不后台续跑） |
| WaitingConfirm | userCancel | 裁决取消（cancel） | Cancelled | 停止未开始步骤；保留已成功预览内容 |
| Executing | stepFail | 步骤失败 | Failed | 不自动重试；如实报告 |
| Executing | interrupt | 应用中断/重启 | Interrupted | 显示可重试；不跨重启续跑 |
| Interrupted | retry | 用户显式重试 | Planned | 新回合重新校验 |
| Executing | finish | 片段完成/无更多步骤 | Completed | 阶段收敛终态（有阶段结果） |

**forbidden_transitions（重构）**：Executing → Completed 未经 finish；WaitingConfirm → 执行（无 DecisionRequest
裁决不得推进——**任意新聊天消息不得绕过待确认**，澄清除外）；Interrupted → Executing 无显式重试；任一态 → 后台队列。

**guard 重点**：
1. **新消息 gate**：存在未决 DecisionRequest 时，新回合入口拒绝（或仅接受带明确裁决语义的输入）；
2. **每片段重新授权**：capability 收窄在 segmentStarted 前执行；
3. **三层分离**：DiffCard 接受/拒绝只走内容三态链路，不产生 TaskRun 事件；propose_edit 不再直接置
   waiting_confirm（确认点只由 decisionProposed 产生）。

#### 2.2.3 WorkspaceOperation 确认后的执行语义（2026-08-21）

- WorkspaceOperationPlan 通过 DecisionRequest 进入 WaitingConfirm；资源操作的完整状态由 SM-WS-04 承载，SM-AG-04 不复制其 Planning/Revalidating/Executing/Compensating 状态。
- `userConfirm` 后 TaskRun 回到 Executing，并立即调用 WS 执行冻结 plan_id；这不是后台续跑，而是本次明确裁决触发的同一任务片段。不存在 `WaitingUserCommit`、资源 DiffCard 或后置保存裁决。
- WS 返回 Completed 后，TaskRun 才可进入下一片段；返回 FailedCompensated / PartialFailed 时该步骤失败并如实收敛，不能继续依赖步骤。
- CreateDocumentBundle 的资源创建成功后，初始内容 Diff 投影不驱动 WaitingConfirm；DiffCard 仅管理内容卡生命周期。
- 应用重启使未完成资源计划和 TaskRun 均进入 Interrupted；用户显式重试必须重新计划、重新核验并产生新的 DecisionRequest，旧确认不得复用。

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
> 取消信号；循环结束回归 SM-AG-01 流式终态路径。

<!-- STATE_MACHINE
machine_id: SM-AG-02
owner_module: AG
status: active
summary: 回合内工具调用循环（注册/权限校验→执行→回注→循环控制；写盘禁令、上限、可中断）
source_type: design
chain: AG-TOOL
rules: BR-AG-STATE-002, BR-AG-VERIFY-001
initial_state: ModelStreaming
terminal_states: LoopCompleted, LoopAborted
-->

- **machine_id**：SM-AG-02　**owner_module**：AG
- **scope**：一次代理回合内的工具调用循环（首个工具请求出现→终轮答复/中止）
- **context**：工具注册清单、本轮 IntentResult.allowed_tools、权限边界、AgentReadContext、循环计数与上限、当前工具请求/结果、CancellationToken
- **states**：ModelStreaming、ToolValidating、ToolExecuting、ResultInjecting、LoopCompleted、LoopAborted
- **events**：toolRequest、validateOk、validateFail、execOk、execFail、injected、finalAnswer、loopLimit、cancel
- **transitions（主表）**：

| from | event | guard | to | action |
|---|---|---|---|---|
| ModelStreaming | toolRequest | — | ToolValidating | 校验注册清单与权限边界 |
| ModelStreaming | finalAnswer | — | LoopCompleted | 终轮答复，回归 SM-AG-01 streamEnd |
| ToolValidating | validateOk | 工具已注册、属于本轮 allowed_tools 且目标/参数/权限有效 | ToolExecuting | 先广播真实 ToolExecuting，再执行；只读经 AgentReadContext，写意图只生成 diff 建议 |
| ToolValidating | validateFail | 未注册/未获本轮允许/越权/目标或参数失效 | ResultInjecting | 拒绝原因回注模型，不执行副作用 |
| ToolExecuting | execOk | — | ResultInjecting | 结果回注 |
| ToolExecuting | execFail | — | ResultInjecting | 失败信息回注（不中断循环） |
| ResultInjecting | injected | 循环计数 < 上限 | ModelStreaming | 续调模型（仍在已过门禁的回合内） |
| ResultInjecting | loopLimit | 达循环上限 | LoopAborted | 中止并回报（统一错误形状） |
| 任意非终态 | cancel | 用户中断 | LoopAborted | 中断终态，无半落盘 |

- **error_states**：无独立错误终态——工具失败回注模型继续循环（execFail→ResultInjecting）；
  循环级失败统一走 LoopAborted 回归 SM-AG-01 错误/中断路径
- **recovery_paths**：LoopAborted→SM-AG-01 TurnError/Cancelled 路径处置
- **forbidden_transitions**：
  - ToolValidating → ToolExecuting 无 validateOk（未注册/越权不得执行）
  - 已注册工具 → ToolExecuting 当其不在 IntentResult.allowed_tools（注册不等于本轮获权）
  - ToolExecuting → 直接文件写入（写意图必须产出 diff 建议，X-FORBID-006）
  - ToolExecuting → DiskState 读取，当目标文档已打开且 AgentReadContext 中应存在 OpenDocSnapshot
  - ResultInjecting → ModelStreaming 当循环已达上限（不得无限循环）
- **acceptance_criteria**：未注册、未获本轮允许或越权工具不可达 ToolExecuting；循环上限强制生效；任何工具执行
  不产生直接文件写入；read_doc/search_text/list_blocks 对已打开文档命中 OpenDocSnapshot，缺失时拒绝；
  cancel 随时可达 LoopAborted
- **test_matrix**：见 §5

## §4 SM-AG-03 · 辅助回合机（autocomplete + inline-assist 同机承载）

<!-- STATE_MACHINE
machine_id: SM-AG-03
owner_module: AG
status: active
summary: 辅助续写/局部修改回合（快捷键手动触发→防抖/latest-wins→门禁→轻量调用→装饰层呈现→采纳即编辑；辅助续写 3 候选；局部修改可多轮；失败静默）
source_type: design
chain: AG-ASSIST
rules: BR-AG-STATE-003, BR-AG-VERIFY-001
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
| Presenting | accept | — | Accepted | 采纳=用户编辑操作：经 ED 受控接口进入编辑模型，驱动 SM-ED-01 edit（X-INTER-001） |
| Presenting | dismiss | 用户忽略/关闭弹窗、点击浮层外、移动光标或改变选区 | Dismissed | 装饰复位不残留；已返回内容丢弃，不入历史 |
| Presenting | anchorLost | blockId 失效 | Dismissed | 放弃呈现，装饰复位 |

- **error_states**：无显式错误终态——辅助回合失败不弹错、不打断；gate_denied/gate_offline/gateway_error/empty_output 静默回 Idle，parse_underflow 经 candidateUnderflow 进入轻量空态 Presenting 以避免触发后瞬间消失
- **recovery_paths**：失败静默回 Idle，下次触发自然重试
- **forbidden_transitions**：
  - Idle / Throttling → Requesting 无门禁可用（不经门禁不得调用，BR-AG-VERIFY-001）
  - Requesting / Presenting → 直接修改文档模型（必经 accept 走 ED 受控接口，X-FORBID-005）
  - Idle → Presenting（未经调用不得呈现）
  - 过期 requestId 的 candidateOk → Presenting（旧请求不得覆盖新请求）
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

- **SM-AG-01**：send→Assembling→AwaitingGate→gatePassed→Streaming→streamEnd→HandingOff→Completed
  正路；gateDenied→Denied 终态且不可达 Streaming；cancel→Cancelled 无半落盘；handoffFail/
  streamFail→TurnError→retry；禁止 Idle/Assembling→Streaming 旁路；持久化投影覆盖
  completed/error/cancelled/gate_rejected/draft，draft 重启后不得显示为完成态；已打开文档缺
  OpenDocSnapshot 或 selection 校验失败时 assembleFail，不得进入 AwaitingGate。
- **SM-AG-01 1.0 加固**：本地 Passed 常量不得触发 Streaming；blocking ambiguity 走
  clarificationRequired→Completed 且网关调用为零；Gateway Error 后不得出现 Completed；cancel 必须触发
  CancellationToken 且后续工具调用数为零；terminal_latch 拒绝第二终态；agent-edit-1.0 禁止 takeover。
- **SM-AG-02**：toolRequest→校验→执行→回注→续调循环；validateFail 回注拒绝；loopLimit 强制
  中止；cancel 随时中止；断言无任何工具执行产生直接文件写入（与 DE/WS G5 联合验证）；
  read_doc/search_text/list_blocks 已打开文档覆盖 LogicalState，未打开文档读取 DiskState。
  已注册但不在本轮 allowed_tools 的工具必须 validateFail；状态事件顺序必须先 ToolExecuting、后真实执行、
  再 ResultInjecting，不得在工具完成后补发状态。
- **SM-AG-03**：重复 trigger 下 throttleDrop 或 latest-wins 覆盖生效；无订阅/断网 throttlePass 后静默回
  Idle；过期 requestId 的 candidateOk 被丢弃；候选不足 3 条 candidateUnderflow 且记录 parse_underflow；candidateOk→Presenting→accept→Accepted
  驱动 SM-ED-01 edit；辅助续写 3 候选采纳；局部修改光标插入；Presenting→refine→Requesting 多轮循环（只传上一轮
  候选结果，每轮过门禁、不持久化）；anchorLost/dismiss 回写终态且复位不残留；禁止未经门禁到 Requesting。
- **跨模块（G5 联调）**：AG↔SM-SYS-01 门禁转移、HandingOff↔DE（待 DE G2 双侧）、
  SM-AG-03 accept↔SM-ED-01 edit（双侧就绪）。
- 量化 NFR（节流参数、延迟）延 G5 标定。

## §6 总图登记（A-SYS-M-S-01）

| 核心链 | machine_id | 总图回填动作 |
|---|---|---|
| 代理回合 | SM-AG-01 | §3 行 planned→active |
| tool-call | SM-AG-02 | §3 行 planned→active |
| autocomplete | SM-AG-03 | §3 行 planned→active（与 inline-assist 同机承载，context 区分） |
| inline-assist | SM-AG-03 | §3 行 planned→active（同上） |
| 转移点 | — | §5 X-INTER-001 登记：AG↔门禁（SM-SYS-01）、AG 产出→DE（DE 侧待 DE G2）、SM-AG-03 采纳→SM-ED-01（双侧就绪） |

## §7 术语归属说明

代理回合/工具调用/局部修改/辅助续写 TERM 已在 A-AG-M-T-01 §0 注册；订阅门禁等云术语归 SYS
（TERM-CLOUD-001~004）；diff 主责归 DE。状态机框架词不注册。

## §8 技术债务与盲区（诚实声明）

- **AG 产出→DE 转移仅单侧**：HandingOff 的 AG 侧契约已定（修改建议+定位锚交 DE，回执 handoffOk/
  Fail）；DE 侧 diff 终态机待 DE G2，双侧完整对接与 G5 联调在 DE G2 后闭合。
- ~~构建移交/回落未建模~~ **已建模（B-3，2026-06-12）**：SM-AG-01 增 takeover/fallbackIn 转移与
  HandedOver 终态，与 SM-CBT-01 双向回执对接，登记总图 §5。原始记录留痕——
- **构建移交/回落（历史债务，已解）**：SM-AG-01 与 CBT 构建机的接管/中断对接（总图 §5 既有转移点）联合
  CBT G2 补登，本文不预定义接管协议。
- **SM-AG-02 嵌套语义**：与 SM-AG-01 Streaming 段的嵌套激活/取消传播在实现期细化（单实例语义
  已锁，嵌套调度属实现）。
- 节流窗口、循环上限数值延 G5/运营标定（机制已锁）。

## §9 后续动作

1. 落地后同步编辑 A-SYS-M-S-01：§3 四行 planned→active（SM-AG-01/02/03），§5 登记转移点。
2. 随 `npm run governance:generate` 复跑核对。
3. 待人类宣布 AG G2 闭合；DE G2 完成 AG 产出→DE 双侧对接；CBT G2 补登接管转移点。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
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
