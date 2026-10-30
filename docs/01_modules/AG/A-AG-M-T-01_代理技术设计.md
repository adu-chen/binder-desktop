---
文档编号：AG-M-T-01
文档状态：A
负责模块：AG
文档职责：AG 代理模块级 L2 技术设计（规则 SSOT）——代理回合、工具调用、上下文/提示词、会话与辅助能力，并定义 agent-edit-1.0 Runtime Profile 的可靠执行边界
上游约束：A-AG-M-D-01_代理功能需求描述.md（REQ-AG-001~009/011/013/015）、A-CORE-X-D-02_Agent1.0冻结能力基线.md、A-CORE-C-P-01_APC.md（§4.1/§4.3 云薄端、§6、§13）、A-SYS-I-P-03_云接口契约.md（SM-SYS-01、X-SEQ-001、CLOUD-GATE/INVOKE、TERM-CLOUD-001~004）、A-SYS-I-P-02_IPC协议.md（BR-SYS-STATE-002 流式终态、统一错误形状）、A-SYS-I-P-01_数据结构定义.md（X-SHARED-001、BR-SYS-PERSIST-001）、A-WS-M-T-01_工作台技术设计.md（X-FORBID-004）、A-ED-M-T-01_编辑器技术设计.md（ED-DECOR、X-FORBID-005）、A-DE-M-D-01_DiffEdit功能需求描述.md（REQ-DE-001 产出消费）、ADU.md（§3）
直接承接：本文 RULE/CHAIN/CONSTRAINT/TERM 为 AG 规则 SSOT，代码经 @GOV 映射；SM-AG-01/02/03/04 在 A-AG-M-S-01 定义并登记总图；AG 产出契约供 DE 承接；注入消费契约对接 REF
使用边界：本文是 AG 规则 SSOT；不重定义门禁/网关契约（归 A-SYS-I-P-03，本文承接引用）、不定义 diff 生成与终态（归 DE）、不定义 @mention 解析（归 REF）、不定义装饰层机制（归 ED）、不锁定模型/提示词/工具具体清单（延实现期与运营配置）、状态机内部归 A-AG-M-S-01；BuildMode/CBT 属 P2 后续 Profile，不是 `agent-edit-1.0` 的能力或调用路径
变更要求：门禁前置语义变更须先经 APC 授权（§13 不得绕过订阅门禁）；工具权限边界变更须与 DE/WS 联合复核；回合历史持久化结构变更遵守 BR-SYS-PERSIST-001；TERM 一经 term_ref 引用其 en 不可更改
治理成熟度：draft
---

# Binder 代理技术设计（AG · L2 技术设计）

## 0. Terminology Registry

> AG 主责核心业务概念首批注册。订阅门禁/AI网关/会话令牌/BYOK透传已由 SYS 注册
> （TERM-CLOUD-001~004），本文引用不重注册。如人类对命名有异议，可调整后重跑 generate。

<!-- TERM
term_id: TERM-AG-001
chains: AG-TURN
zh: 代理回合
en: AgentTurn
forbidden: 对话轮, 轮次, 会话轮, round, 聊天回合
-->
一次用户发起到 AI 产出处置完成的完整回合：上下文组装→门禁→网关流式→（工具循环）→产出处置
（修改建议交 DE）；可中断/取消，终态明确（REQ-AG-001）。

<!-- TERM
term_id: TERM-AG-002
chains: AG-TOOL
zh: 工具调用
en: ToolCall
forbidden: 函数调用, function call, 工具执行, 能力调用
-->
代理回合内模型发起的工具请求-执行-结果回注循环单元；必须经注册与权限校验，循环有上限且可中断；
文件写入意图必须走 DE diff 路径（REQ-AG-002）。

<!-- TERM
term_id: TERM-AG-003
chains: AG-ASSIST
zh: 局部修改
en: InlineAssist
forbidden: 内联辅助, 行内辅助, 内联建议, inline建议, 行内AI
-->
AI 编辑三层之第二层（选区/光标级，2026-07-04 人类澄清定义；en 沿用 InlineAssist 不变）：
用户经快捷键在选中文本（或仅激活光标）处弹出简易编辑窗口，以 AI 交互对内容编辑/生成/美化；
选区态 Apply 替换选区，光标态为生成插入；弹窗内**可多轮修改**（上轮产出为下轮基底，每轮均过
门禁）；**辅助对话不持久化**（不入回合历史）；候选经 ED 装饰层呈现，采纳作为用户编辑操作进入
编辑模型；无订阅/断网静默不提供（REQ-AG-005）。

<!-- TERM
term_id: TERM-AG-004
chains: AG-ASSIST
zh: 辅助续写
en: Autocomplete
forbidden: 自动补全, 自动完成, 补全建议, 智能补全, 续写补全
-->
AI 编辑三层之第一层（光标级，2026-07-04 人类澄清定义；en 沿用 Autocomplete 不变）：光标激活
状态下经快捷键**手动触发**（非键入自动触发），基于上下文感知直接生成 3 个后续内容候选（每个
50–100 字量级，数值属运营配置）；仅候选选择交互（Tab 采纳第 1 个，1/2/3 采纳对应候选，Esc/继续
输入忽略）、无对话；同过门禁、同经装饰层/overlay 语义呈现、采纳同为用户编辑操作（REQ-AG-005）。

<!-- TERM
term_id: TERM-AG-006
chains: AG-TURN
zh: 编辑模式
en: EditMode
forbidden: 对话模式, chat模式, chat 模式, 对话编辑模式
-->
代理编辑（TERM-AG-008）面板在 Agent 1.0 的当前运行模式：默认面向当前文档的 AI 精确修改
（编辑器感知 + 块级锚定，内容产出一律经 DE diff 裁决、人最后裁定），兼具文件创建、工作区资源整理、
项目感知、对话与资源获取。资源操作统一生成 WorkspaceOperationPlan，并以 DecisionRequest 前置裁决；
确认后由 WS 重新核验并直接修改 DiskState，不生成资源 DiffCard。Agent 1.0 不存在可与之互相复用能力的 BuildMode 运行路径。注：编辑器/文档层的「编辑态 vs 预览态」（docx 打开
形态）与本模式无关，不得混用。

<!-- TERM
term_id: TERM-AG-007
chains: AG-TURN
zh: 代理构建
en: BuildMode
forbidden: 构建模式, agent模式, agent 模式, Chat Build, chat-build
-->
后续 Profile 的保留名称（2026-08-07 中文称呼更名为「代理构建」，旧称「构建模式」退役为禁用别名，en/term_id 不变）。
当前仅用于第四个模型偏好槽位与历史会话元数据兼容，不定义 RuntimeProfile、入口、回合、状态机、CBT 接管、工具或验收语义。
任何未来启用都必须由新的 REQUIREMENT/DESIGN Scope 重新定义，不得从本 TERM 推导旧工作流。

<!-- TERM
term_id: TERM-AG-008
chains: AG-TURN
zh: 代理编辑
en: AgentEdit
forbidden: 对话编辑, 对话编辑板块
-->
AI 编辑三层之第三层（面板级，2026-07-04 人类裁决定名）：Agent 1.0 面板承载 EditMode、会话持久化、
经 DE diff 裁决的产出与受限工具调用；BuildMode 是 P2 后续 Profile，不属于当前 AgentEdit 运行边界。
沿革：旧版曾以「对话编辑」总称整个 agent 能力，该词后又错位为 DE 模块中文名——三重身份
致混，2026-07-04 裁定：正式名「代理编辑」，「对话编辑」全面退役为禁用别名；DE 模块更名
**Diff Edit**（不再有中文名）。
AI 编辑三层：①辅助续写（TERM-AG-004，光标级）②局部修改（TERM-AG-003，选区/光标级）
③代理编辑（本条，面板级）；①②采纳即用户编辑直改不经 diff，③产出经 DE 裁决——轻重分界
（BR-AG-STATE-003 / X-FORBID-006）。

<!-- TERM
term_id: TERM-AG-009
chains: AG-TURN
zh: 代理编辑会话
en: AgentSession
forbidden: 聊天标签, 对话标签, 会话标签, agent标签, chat tab, session tab
-->
代理编辑（TERM-AG-008）面板中的可重开会话标签：承载一组按时间排序的代理回合、标题、模式、
归档/打开顺序与激活态。它是 workspace 控制态的一部分，重启后恢复标签存在状态与顺序；回合内容按
懒加载策略恢复。代理编辑会话不同于文档标签（SYS/ED 视图导航态）和账号会话（云认证），不得混用。

<!-- TERM
term_id: TERM-AG-010
chains: AG-MODEL-CATALOG, AG-ASSIST, AG-TURN
zh: 预设模型
en: PresetModel
forbidden: 内置模型, 托管模型, 官方模型
-->
云端配置下发、经订阅门禁可调用的模型（REQ-AG-006）：与 BYOK 模型（TERM-CLOUD-004，用户自带
密钥）分层呈现；桌面端仅消费清单（GET /models），不承载配置权威源；1.0 EP 主路径 DeepSeek 托管
模型，自部署预设模型演进边界见 A-GW-M-D-01 §3.5（GW-06）。

<!-- TERM
term_id: TERM-AG-011
chains: AG-TURN
zh: 消息片段
en: MessageSegment
forbidden: 消息块, 消息段, 文本片段, 卡片锚点, segment
-->
assistant 消息内部按事件到达顺序组织的最小呈现单元：`text`（流式累积正文）、`tool`（引用
ToolCall 卡片）、`diff`（引用 DiffCard）。回合内 text/tool/diff 片段按时间序交错排列，构成
"对话流与工具调用、修改建议的时序呈现"（BR-AG-UI-001）；持久化时随 assistantMessage 序列化，
旧记录无片段信息时前端降级为消息后沉底布局。

<!-- TERM
term_id: TERM-AG-012
chains: AG-TURN, AG-TOOL
zh: 本轮意图结果
en: IntentResult
forbidden: 意图布尔值, 编辑意图标记, intent flag, 意图关键词结果
-->
由模型可见输出与真实运行事件持续更新的当前目标语义投影：至少表达目标描述、目标候选、预期副作用、
所需上下文、歧义与确认点，并携版本/修订号。运行时可在首个模型结果前建立 `unresolved` 初始投影，
但不得以关键词或本地分类结果冒充模型已经完成的语义判断。IntentResult 不包含 `allowed_tools`，不授予
目标、工具或副作用权限，也不替代用户原文或保存模型隐藏推理；上下文与界面只消费其当前版本，执行
授权唯一归 CapabilityGrant（BR-AG-DATA-004 / BR-AG-STATE-002）。

<!-- TERM
term_id: TERM-AG-013
chains: AG-TURN
zh: 运行时档位
en: RuntimeProfile
forbidden: 运行时配置, 能力档位, runtime 配置, profile 档位, 代理档位, 运行配置
-->
Agent 交互的运行时能力边界声明。1.0 唯一 `agent-edit-1.0`：缺失或未知 profile 必须在 Gate、provider、
工具与 Proposal 前失败关闭；已注册工具、Prompt、前端参数或兼容默认值都不是 profile 的授权来源
（A-SYS-I-P-01 §3.1，BR-SYS-STATE-007）。

<!-- TERM
term_id: TERM-AG-014
chains: AG-TURN, AG-TOOL
zh: 当轮能力授权
en: CapabilityGrant
forbidden: 工具授权, 能力授权, 权限授予, grant, 当轮授权, 工具许可
-->
运行时针对下一次具体工具执行计算的短生命周期授权：由 RuntimeProfile、工具注册表、当前 workspace/
读取范围、ExplicitEditTargetSet、允许的副作用路径、格式与动态能力、已解决 DecisionRequest、Gate、预算、
取消和终态共同收窄。IntentResult、TaskPlan、模型文字和工具发现都不是授权源；注册表存在也不等于当前
可调用。每次工具请求均重新计算并校验，授权在该动作完成、事实变化、取消或终态后失效，不跨回合继承
（A-SYS-I-P-01 §3.1，BR-SYS-STATE-007）。

<!-- TERM
term_id: TERM-AG-015
chains: AG-TURN
zh: 受限任务实例
en: TaskRun
forbidden: 任务实例, 后台任务, 任务队列, 子任务集合, task 实例, 复合任务对象
-->
复杂目标或必须跨多个可见执行片段继续时，由 TurnEvent 派生的可观察任务投影，用于把多个 AgentTurn、
已发生动作、等待点和当前候选路线关联到同一用户目标；简单目标不建立 TaskRun。TaskRun 不调度工具、
不预授权未来动作，也不是后台队列：不自动重试、不跨重启续跑、不越过确认点；“工作中”只能由真实
provider、工具或副作用事件驱动（A-SYS-I-P-01 §3.1，BR-SYS-STATE-008）。

<!-- TERM
term_id: TERM-AG-016
chains: AG-TURN
zh: 任务步骤
en: TaskStep
forbidden: 任务步骤, 步骤, 子步骤, step 单元, 执行步
-->
TaskRun 中从 TurnEvent 归纳出的已发生或当前候选工作单元（读取/搜索/分析/生成/提案/等待确认等）。
TaskStep 只用于解释进展与依赖，不触发工具、不持有 CapabilityGrant、不承诺候选工作一定执行；进入
等待用户或裁决的工作单元不得在不可见后台继续（A-SYS-I-P-01 §3.1，BR-SYS-STATE-008）。

<!-- TERM
term_id: TERM-AG-017
chains: AG-TURN
zh: 任务级裁决请求
en: DecisionRequest
forbidden: 确认卡, 裁决项, decision 实例, 确认请求
-->
任务级通用裁决项（类型：权限 permission / 方案 plan_choice / 范围 scope / 人工确认 manual_confirm），与内容
裁决分离：生命周期 proposed → resolved（confirm / cancel）。裁决结果作为 TurnEvent 恢复或终结原
AgentTurn；存在 TaskRun 时同步更新其投影。内容层 DiffCard 的接受/拒绝不驱动 Agent 的未来路线
（本文 §7.2、A-AG-M-S-01 §2.2，REQ-AG-015）。

<!-- TERM
term_id: TERM-AG-018
chains: AG-TURN
zh: 任务计划
en: TaskPlan
forbidden: 计划, 任务计划清单, plan 单元, 执行计划
-->
复杂目标中由模型当前可见路线说明与 TurnEvent 派生的可展示计划投影（非模型思维链）：只表达已完成、
正在发生、等待中和可调整的候选工作及其依赖/结果。简单目标单 AgentTurn 直行不建 TaskPlan；TaskPlan
不保存或发放授权、不充当执行游标，模型可根据真实 ToolResult 更新路线，运行时仍对每次调用重新 Gate/
授权。确认只解除对应等待条件，不自动启动预排片段（本文 §7.2、A-AG-M-S-01 §2.2，REQ-AG-015）。

<!-- TERM
term_id: TERM-AG-019
chains: AG-TURN, AG-TOOL
zh: 显式编辑目标集
en: ExplicitEditTargetSet
forbidden: 编辑白名单, 目标白名单, 跨文档白名单, 隐式目标集, 模型目标集
-->
当前 AgentTurn 被明确授权生成 ChangeProposal 的 active workspace 既有文档集合。初始成员仅来自当前
文档和用户输入中指向文档的 ReferencePayload（兼容输入须先归一为同一语义）；同一 AgentTurn 的
AwaitingClarification→Assembling 补充可单调追加新的显式引用。模型执行 read/search/list 或在正文中提到
路径只产生读取事实，不产生修改授权。工具发现的新目标必须冻结为 DecisionRequest（kind=`scope`）路径清单，
经用户确认后才追加到绑定 TaskRun 的权威集合；显式 TaskRun continuation 继承该冻结集合，不从模型输出或
历史文本重建。经确认完成的 CreateDocumentBundle 只在真实资源建立后追加其目标。简单 AgentTurn 的集合仅在
回合内持有，不因本术语创建 TaskRun。集合外既有文档在 ChangeProposal 生成前失败关闭；`cross_doc` 只能表示
集合内非当前文档的展示属性，不是授权来源（BR-AG-DATA-003 / BR-SYS-STATE-007/008）。

<!-- TERM
term_id: TERM-AG-020
chains: AG-TURN
zh: 代理上下文事实集
en: AgentContextFacts
forbidden: 上下文状态, 编辑器上下文, 项目上下文, AgentContext, context facts
-->
AgentTurn 每次进入 Prompt 装配前冻结的版本化只读事实载荷，旧 revision 不可覆盖。v1 保留完整快照
事实；v2 先冻结当前 workspace 与活动/已打开资源身份和可用性，按需取得后才包含
来源/格式/脏态/保存基准/LogicalState 版本。另含任务显式需要的 SelectionRef，以及关联
ChangeProposal 的身份与状态；文档正文继续由 OpenDocSnapshot/ReferencePayload 承载。该事实集是
ED/DE/WS→AG 的跨模块投影，不授予读取集合之外的访问权，也不扩大 CapabilityGrant 或
ExplicitEditTargetSet（BR-AG-DATA-008 / BR-SYS-DATA-008）。

<!-- TERM
term_id: TERM-AG-021
chains: AG-TURN, AG-TOOL
zh: 项目报告
en: ProjectReport
forbidden: 项目总结, 项目扫描报告, workspace 报告, 分析总结, 普通回答
-->
面向用户的结构化只读分析投影，声明本轮请求范围、由 EvidenceLedger 证明的已读项及来源、未读范围、
结论证据和不确定性。ProjectReport 由运行时把真实上下文/ToolResult 证据与模型最终回答关联后派生，
不要求模型调用完成工具；模型自述、目录列表或搜索命中不能提高实际读取覆盖（BR-AG-OBS-001）。

<!-- TERM
term_id: TERM-AG-022
chains: AG-TURN, AG-TOOL
zh: 回合事件
en: TurnEvent
forbidden: 运行日志, 状态日志, 事件记录, event log, 回合日志
-->
AgentTurn 内按单调序号追加的不可变事实事件，覆盖用户输入接收、上下文冻结、Gate 结果、provider 调用、
模型输出、工具请求/结果、裁决、Proposal 与首个终态。TurnEvent 是回合运行事实的唯一来源；UI 状态、
IntentResult、TaskRun/TaskPlan 与恢复视图均从事件及被引用的持久化业务事实重建，不允许覆盖历史事件。

<!-- TERM
term_id: TERM-AG-023
chains: AG-TURN, AG-TOOL
zh: 证据账本
en: EvidenceLedger
forbidden: 读取日志, 证据列表, 阅读记录, evidence log, 工具证据
-->
由上下文冻结事件和成功 ToolResult 事件确定性派生的只读证据投影，记录来源、目标、覆盖范围、版本/hash
与新鲜度。它不接受模型自报写入，不授予访问或编辑权限，是 ProjectReport 已读/未读与不确定性计算的
唯一证据来源。

## §1 文档定位

```
CORE 冻结基线 → REQ-AG-*（AG L1）→ 本文（已映射需求的规则 SSOT）→ A-AG-M-S-01 / A-AG-I-P-01 → 代码(@GOV)
本文 ←承接← A-SYS-I-P-03（门禁/网关契约，不重定义）、A-ED-M-T-01（ED-DECOR）、REF（注入载荷）
本文 →驱动→ A-AG-M-S-01（SM-AG-01/02/03/04）→登记→ A-SYS-M-S-01 总图
本文产出契约 →供→ DE G2（AI 产出→diff 生成）承接
```

**云薄端前提重申（APC §4.3/§13）**：AG 不持密钥、不内置 provider SDK、不绕过订阅门禁；
本文一切规则在 SYS I-P-03 契约（CLOUD-AUTH/GATE/INVOKE、SM-SYS-01）之上承接，不重定义云协议。

## §2 技术分层与边界

```
AG 代理（本文 SSOT）——Agent 1.0 代理编辑（TERM-AG-008）面板：EditMode；BuildMode/CBT 为 P2 后续 Profile
  ├─ 代理回合（§3，AG-TURN）        组装→门禁→网关流式→产出处置；历史持久化原则
  ├─ 工具调用循环（§4，AG-TOOL）    注册/权限校验→执行→回注→循环控制
  ├─ 上下文组装（§5）               文档/注入载荷/本地资产，workspace 限界
  └─ 辅助续写与局部修改（§6，AG-ASSIST） 快捷键触发→防抖→轻量调用→装饰层呈现→采纳即编辑
```

- 模型调用唯一路径：`AG → SM-SYS-01 GatePassed → AiGateway`（X-SEQ-001）；失败/拒绝走统一
  错误形状；流式遵守 BR-SYS-STATE-002（完成/中断/错误三类终态，中断无半落盘副作用）。
- AI 内容修改唯一路径：`AG 产出 → DE 生成 pending_diff → ED/DE 受控转正`；激活文档由人类裁决转入
  LogicalState，已打开非激活文档由基线校验通过后的自动应用事务转入 LogicalState，未打开文档才在
  人类接受后经 WS 落盘。AG 不得直接写 LogicalState 或 DiskState（X-FORBID-006 / X-FORBID-009）。

### §2.1 agent-edit-1.0 Runtime Profile

1.0 运行时固定使用 `runtime_profile=agent-edit-1.0`。该 profile 是发布能力边界，不取代完整产品规则：

| 维度 | 1.0 激活策略 |
|---|---|
| 目标 | 当前活动文档与用户输入中指向 active workspace 既有文档的显式 ReferencePayload 身份可初始化 ExplicitEditTargetSet；引用内容保持 data-only，成员资格不单独授予写工具；工具发现目标另经 scope DecisionRequest 确认；有初始内容的新建文档经 `plan_workspace_operation`（CreateDocumentBundle）前置裁决，确认建盘后才追加目标并投影初始内容 Diff；明确零字节 TXT 仍经同一资源计划与裁决链，但不绑定初始内容 Diff |
| 工作策略 | 模型在运行时硬边界内理解目标、选择工具并根据 ToolResult 调整路线；有初始内容的新建文档固定走 CreateDocumentBundle，不产出空基准 ChangeProposal；明确零字节 TXT 只允许单文件无初始内容计划，确认后直接创建零字节文件；简单目标只使用单个 AgentTurn，复杂或跨执行片段目标才派生 TaskRun/TaskPlan 投影，不启用后台队列或多 Agent |
| 工具 | Prompt 只暴露运行时按当前事实计算的 CapabilityGrant；模型从中选择工具，执行器对每次请求重新计算并校验；文件移动/建目录、表格/图片高级操作仅在既有专项能力与动态条件同时满足时启用 |
| 上下文 | 当前输入 + 资源身份/可用性 + 显式引用 + 必要历史；v2 按需取得专业 LogicalState，取得后仍为文档主事实；v1 兼容边界见 I-P-01；AST/KB/TPL 自动供给关闭 |
| Prompt | 统一 Prompt Runtime 装配；系统约束与 data-only 文档/引用分离；携 runtime version |
| 产出 | 回答或 ChangeProposal；任何内容创建/修改均经 DE Diff 裁决 |
| 会话 | workspace 隔离、本地恢复、单会话同一时刻一个活动回合 |
| 延期 | BuildMode/CBT、自动记忆/知识、模板工作流、参考风格迁移、开放式通用规划、多 Agent |

前端可继续保存完整路线所需的 additive 字段，但 1.0 不得展示没有独立 Runtime Profile 的代理构建
入口；延期能力未通过专项系统验证时必须 feature-gate，而不是依靠提示词声明“不要使用”。

### §2.2 独立验证的运行边界（P00，2026-09-15）

本节是已确认设计，不是实现或 G5 通过登记；承接 BR-AG-STATE-001/002、BR-AG-DATA-001/002/003/008、
BR-AG-UI-001、BR-SYS-STATE-007/008、BR-SYS-DATA-008，不新增产品 Profile 或平行授权规则。

Binder Agent 是嵌入本地文档工作台、围绕用户当前任务进行有限自主执行的文档协作 Agent。运行核心是
Binder 任务间可复用的运行机制，不是脱离写作场景的通用聊天产品，也不是覆盖所有 Agent 类型的框架。

| 决策 | 本阶段边界 | 实现责任 |
|---|---|---|
| 任务/自主范围 | 对话、读取分析、授权内 ChangeProposal 与既有 WorkspaceOperation；模型选择步骤，遇规定确认点等待 | AG 保持有限循环，不新增 Shell、后台队列或自动重试 |
| 状态所有权 | AG 拥有运行身份/事件；ED/DE/DOC/WS 拥有专业状态；UI 只投影 | 核心不直接维护 ProseMirror、Block、LogicalState 或 DiskState |
| 生效语义 | Agent 结束、专业操作成功、接受、保存分别证明 | 保留 DE 三分支，不把 ToolResult 成功推导为已保存 |
| 用户并发操作 | 用户可编辑/切换文档；单执行槽，同会话最多一个活动 AgentTurn | 冻结请求目标；等待归属会话；版本漂移不得静默换目标 |
| 权限 | RuntimeProfile、可信宿主事实、用户授权与逐调用 CapabilityGrant 共同收窄 | 不以模型、历史、读取结果或能力注册扩大范围 |
| 上下文 | v2 通用准入不要求无关文档正文；按实际需求取得有效专业快照 | 已打开但未取得快照不等于未打开；禁止 DiskState 兜底 |
| 记忆/恢复 | 有限会话历史、显式 MemorySelectionRef；恢复事实，不自动恢复执行 | AST 自动供给关闭；新请求重新核验；历史证据不复活授权 |
| 外发与验证 | 本地优先不等于不外发；只向真实网关发送当前任务允许的必要上下文 | 不旁路 Gate；模型、核心、专业能力、UI、真实集成五线分证据 |

**责任端口**：模型适配提供响应流；核心负责解析/预算/授权/调度/等待/终态；专业适配提供上下文、
可用条件、参数校验、执行和专业回执；UI 展示安全事件并转达用户动作。模型传输测试替身只能替换该外部
边界，必须经过生产解析、状态机与授权路径，不能直接制造 ToolResult 或 UI 成功。

**最小能力契约要求**：声明能力/工具身份与版本、参数/结果 schema、上下文需求、可用条件、权限/效果类型、
期限及结果规模、重试/回执查询策略、可信专业指令片段。调用关联 workspace/session/turn/task/tool/attempt、
CapabilityGrant、目标版本、取消和期限；返回执行状态、效果状态、专业对象引用、错误/下一步及原始执行事实。
共享字段归 SYS I-P-01，具体 wire DTO 在对应实施工作包前完成，不可由 AG/UI 自建近义真值。

**等待与效果**：核心须从第一条基线支持带版本资源、异步回执、用户裁决、过期拒绝和效果分层；
通用参数入口使用已发布 common 工具 schema 的 object/required/type/enum/items/数值边界校验，
单次序列化参数上限 1 MiB、结构深度上限 32；专业 propose_edit 的结构规则继续归专业适配。
只读执行等待期限 30 秒，取消或超时后丢弃迟到结果，不登记成功证据、不自动重试。
底层同步读取可能继续收尾，因此不得把该包装用于写操作。受控复制、专业 Proposal 和任务控制能力
经独立 owned 执行端口：原数据库和回合注册表共享句柄，不复制业务真值；短时注册锁只授予效果 lease，
不覆盖整个耗时操作。核心等待上限 30 秒，取消可及时结束等待；未结算效果保留原身份并禁止接管或重放。
端口先提交 started 身份，再执行能力，再提交 settled 安全回执，结算持久化成功后才能返回成功；
未知/迟到结果归原调用的独立效果事实，既不回注新模型片段，也不改写原回合终态。查询契约见 SYS §3.1.0。
同 AgentTurn 的历史 ToolCall 标识（含裁决恢复检查点）不得重复；整批身份非法时整批未执行。
同一回合使用可信运行策略冻结模型/工具预算（开发默认模型12次、工具16次、控制封套与参数错误分别最多纠错2次、模型预留2次收尾；数值不是供应商容量保证），裁决检查点恢复扣除先前已提交工具批次/调用数，
不得通过反复等待和确认重置执行额度。用户等待不计模型/只读工具等待期限，可显式取消；
网关和只读执行分别保留独立期限，而非宣称可以强制终止任意专业同步写入。
无需理解回执背后的具体编辑器。回执只消费一次，旧操作只能更新/清理自己的身份；专业效果不确定时保留
阻断与核对动作，不得自动重放。此处不新增跨进程自动续跑、普遍 exactly-once 或任务级回滚承诺。
DecisionRequest 的等待恢复沿用同 AgentTurn；专业预览续片段沿用既有 TaskRun/checkpoint 路径，不混成同一语义。

**失败分类**：模型传输、模型协议、核心上下文、核心控制、授权、专业能力、UI 投影、环境分别记录。
每条失败区分“最先观察的边界”和“已确认根因”；模型结构错误被安全拒绝可证明核心错误路径，不证明业务成功。
纯问候不因无关文档元数据缺少正文证据而生成 ProjectReport 噪声；请求相关报告仍受 BR-AG-OBS-001 约束。

## §3 代理回合（AG-TURN）

<!-- CHAIN
chain_id: AG-TURN
主责模块: AG
status: active
risk_tier: P1
integration_test: deferred: phase-5
-->

<!-- RULE
rule_id: BR-AG-STATE-001
主链路: AG-TURN
域: STATE
-->
代理回合必须经完整生命周期：上下文组装→门禁裁决→经网关流式调用→产出处置；回合可中断/取消，
中断必须实际终止流式请求并阻止后续工具启动，中断为终态且不产生半落盘副作用（对齐
BR-SYS-STATE-002）；同一回合只能锁定一个用户可见终态，错误/拒绝/取消后不得再广播完成；状态广播
必须来自真实状态转移并追加为 TurnEvent；当模型输出需要补充的 blocking ambiguity 时，原 AgentTurn 进入
AwaitingClarification 非终态；用户补充澄清信息时，原 AgentTurn 回到 Assembling，重新冻结事实并调用
模型更新 IntentResult，不新建 AgentTurn、不写入取消终态。该语义不代替 DecisionRequest 裁决、错误/中断重试或
Assist 状态机的各自转移。状态广播不得在副作用完成后补发装饰性状态。含修改建议的产出唯一出口是交 DE 生成
pending_diff（AG 无直接写盘路径，X-FORBID-006）；交 DE 生成待裁决修改时必须保留会话、回合和
本轮 assistant 消息锚点，使 DiffCard 回到对应 AgentTurn 信息流内呈现，而不是作为会话底部全局列表
沉底汇总；**回合内呈现时序（2026-08-07 细化，承接 BR-AG-UI-001）**：DiffCard 与 ToolCallCard 按
事件到达顺序内嵌于 assistant 消息流（消息片段 TERM-AG-011），不整体沉到正文底部——模型正文、
工具调用、修改建议按时间序交错呈现；回合失败回统一错误形状并允许用户显式重试，但重试必须创建新 AgentTurn，原 error AgentTurn 保留审计。状态转移归 SM-AG-01（A-AG-M-S-01）。

<!-- RULE
rule_id: BR-AG-DATA-003
主链路: AG-TURN
域: DATA
-->
代理回合修改建议产出（ChangeProposal）的目标边界（2026-07-04 人类裁决，**取代原「单文档铁律」**；
2026-08-28 HD-S3D-01 补齐追加语义）：target 一律限 active workspace 内（X-FORBID-004），并且必须属于
显式编辑目标集（TERM-AG-019）。集合初始成员仅为当前文档与本次用户输入中指向既有文档的显式
ReferencePayload（旧 mentions / blockRefs 须先归一到相同语义）；同一 AgentTurn 从 AwaitingClarification
恢复 Assembling 时，可把补充输入中的显式文档引用单调追加到原集合，不创建新 AgentTurn。

read_doc、search_docs、list_docs 等读取成功、模型正文提及路径或历史中出现路径都不得自行扩大集合。若模型根据
工具结果发现新的既有目标，必须先生成 kind=`scope` 的 DecisionRequest，冻结待追加路径清单；只有用户确认后才
把这些路径追加到原 AgentTurn 的权威集合，存在 TaskRun 时同步更新其投影。后续 provider 调用沿用该冻结集合，
不重新从模型输出、历史或当前已打开标签推断；cancel、拒绝、失效裁决均不追加。简单 AgentTurn 的集合只在回合内持有，不得仅为保存集合
创建 TaskRun。集合外既有文档必须在 ChangeProposal / PendingDiff 生成前失败关闭；`cross_doc` 仅可标识集合内
非当前文档的展示位置，不能作为越权放行或人后裁决替代前置授权。

有初始内容的新建文档不以空基准 ChangeProposal 进入集合，必须先经 `plan_workspace_operation` 生成 CreateDocumentBundle 与
DecisionRequest，确认后由 WS 创建真实资源并打开编辑标签；只有真实资源建立成功后，目标才追加到原 AgentTurn 的
显式编辑目标集（存在 TaskRun 时同步投影），再以该 DiskState 为基线投影初始内容 Diff。用户明确要求零字节 TXT 时，`plan_workspace_operation` 只接受一个 `.txt` 创建项与显式 `createEmpty=true` 标记，不携带 `initialContent`；仍由相同 DecisionRequest 前置裁决、WS 重验创建，完成后无初始 Diff 或内容保存屏障（承 A-WS-M-T-02 §6；内容 Diff 不得创建资源，
X-FORBID-006——WS-RESOURCE-WRITE-GUARD-01）。一切内容产出仍经 DE/ED 受控转正，不得由 AG 直写
LogicalState 或 DiskState（X-FORBID-006 / X-FORBID-009）。已打开目标必须以 OpenDocSnapshot 中的
LogicalState 为 ChangeProposal 基线；未打开目标才允许读取 DiskState。Agent 1.0 不识别或触发 Build/CBT 接管；遇到超出当前冻结能力的请求时只能明确拒绝或收敛到已授权的 Agent 1.0 能力，不得复用 BR-CBT-STATE-002。**DE 侧协同复核**同步调整：`de.generate_diff` 校验 target 在 workspace 内且属回合
显式编辑目标集（当前文档 + 显式指向的既有文档 + 经确认追加的既有文档；CreateDocumentBundle 建盘并追加后
目标才可进入内容链路），越出者拒绝。
（沿革：原文本限单文档、跨文档一律升级 CBT——2026-06-12 B-4 裁定；2026-07-04 人类澄清编辑模式
含文件创建/整理/项目感知，原文本废止。）

<!-- RULE
rule_id: BR-AG-VERIFY-001
主链路: AG-TURN
域: VERIFY
-->
AG 发起的每一次模型调用——代理回合、工具循环内续调、局部修改、辅助续写——必须先经
SM-SYS-01 达 GatePassed 并经 AiGateway 发起（X-SEQ-001 承接）；代码中不存在不经门禁通往模型
调用的路径；GatePassed 必须来自本次服务端 GateOutcome，客户端预设常量、开发桩值或历史通过缓存
不得驱动 AwaitingGate→Streaming；无有效订阅时代理/工具回合明确拒绝；认证/门禁不可达、超时、非法响应或非授权 HTTP 异常返回调用错误，不冒充订阅拒绝。两类失败均不调用模型、不自动重试，显式新请求重新裁决，保留此前执行证据；局部修改/辅助续写静默
不提供且不打断编辑。本规则是 SYS 门禁契约在 AG 的承接执行，不重定义裁决语义（裁决归服务端）。

<!-- RULE
rule_id: BR-AG-STATE-005
主链路: AG-TURN
域: STATE
-->
回合额度收尾必须与真实终态结算一致，不得把提前收工或额度耗尽伪装成完成。输入为原回合冻结的执行策略
（模型/工具上限与收尾预留）、已按续调与实际执行扣减后的剩余工具调用与剩余模型轮次、本轮模型
completion（status/reason）、side_effect_intent、结构告警和原始 stop_reason；输出为收尾判定（是否进入
completion_stage）、收尾记录（completionStatus、budgetClosing、remainingToolCalls、remainingRounds）、
向用户的一条收尾说明、TaskRun 终态结算和 done 停止原因。禁止：额度耗尽或 completion=partial/
blocked、或 report_only 且存在副作用意图时把 TaskRun 结算为 Completed；用冻结前的旧计数冒充收尾时
剩余额度；在收尾记录与 done 落库前提前返回成功。失败信号：剩余额度与 completion 只允许结算
Completed/Failed 二选一；收尾记录或广播失败必须向上返回 IpcError，不得吞掉后按成功处理；
额度耗尽且模型未给出可验证收尾时，done 的 stopReason 必须为 budget_exhausted，既有工具回执与效果
证据保留、未完成范围不得标记为完成。

<!-- RULE
rule_id: BR-AG-PERSIST-001
主链路: AG-TURN
域: PERSIST
-->
回合与对话历史属 workspace 控制态：本地持久化（workspace.db 扩展表，遵守 BR-SYS-PERSIST-001
版本化迁移），重启后会话历史可恢复；对话内容不因历史持久化上行云端（本地优先，REQ-CORE-001）。
恢复范围覆盖 `completed / error / cancelled / gate_rejected / draft` 五类状态；不得只保存成功回合。
流式中断、应用关闭或崩溃留下的半成品必须保持 `draft`，重启后展示为未完成/可重试，不得伪装为
正常完成态。代理编辑会话（TERM-AG-009）的元数据（存在状态、顺序、标题、模式、归档/打开态、
激活态）全部恢复；仅 eager-load 当前激活会话的回合内容，其他会话点击时懒加载。工具卡只保存
结构化摘要（工具名、入参摘要、结果状态/摘要），历史卡片为折叠只读，不重新绑定实时状态、不得从
	历史里重新触发工具；工具摘要必须带回合内消息锚点，恢复后插入对应 assistant 消息后方，不能堆到会话底部。
	DiffCard 只保存引用（`diff_id`、关联文件路径、锚点消息），新写入的 `diffRefs[]` 必须能定位到本轮
	assistant 消息；legacy 记录缺锚点时可按同 turn 的 assistant message 派生，仍无法派生才进入“未归属修改建议”
	fallback 区。真实内容与当前状态仍通过 DE/WS 的 diff 列表恢复合并；**不得把 diff 内容快照写入 AG payload**。
	存储位置固定为 app data
下的 `workspace.db`，不随 workspace 目录移动；迁移/共享字段回写 A-SYS-I-P-01（X-SHARED-001）。
保留策略按 workspace 软上限 200MB：超限时先把最老代理编辑会话标记 `archived=true`，不做无限保留。
历史列表每个代理编辑会话行提供删除按钮；删除只作用于该会话对应的 `agent_sessions`、`agent_turns`
与 `agent_turn_payloads` 记录；不得删除 DE `pending_diffs` / `diff_payloads` 等待裁决修改本体。
删除按钮在代理回合进行中禁用，避免流式回合的异步保存把旧记录写回。

### §3.1 历史持久化数据模型（AG-TURN / AgentSession）

- **`agent_sessions`**：`session_id(PK)`、`workspace_id`、`title`、`mode(edit/build)`、
  `archived`、`open_index`、`is_active`、`created_at`、`updated_at`、`last_active_at`。
  该表恢复代理编辑会话标签存在状态、顺序、标题、模式与激活态。
- **`agent_turns`**：`turn_id(PK)`、`session_id`、`workspace_id`、`rel_path(可空)`、
  `status(completed/error/cancelled/gate_rejected/draft)`、`payload_ref`、`created_at`、`updated_at`。
  同一用户消息重试时必须新建 `turn_id`，旧 draft/error 记录保留审计。
- **`agent_turn_payloads`**：`payload_ref(PK)`、`workspace_id`、`payload_json`、`payload_size`、
  `created_at`。`payload_json` 为 AG 私有 v1 载荷，不上传云端。

	`payload_json` v1 至少包含：`userMessage`、可空 `assistantMessage`、`toolSummaries[]`、
	`diffRefs[]`、可空 `failure`、可空 `incompleteReason`。`toolSummaries[]` 只保存名称、入参摘要、
	结果摘要、状态和 `anchorMessageId`；`diffRefs[]` 只保存 `diffId`、`relPath`、`anchorMessageId`，
	不保存 before/after/styleOps 等 diff 快照。
	**回合内时序（2026-08-07 增补，承接 BR-AG-UI-001 / TERM-AG-011）**：`assistantMessage` 可含
	可选 `segments[]`（`{kind:"text",text} | {kind:"tool",toolCallId} | {kind:"diff",diffId}` 按事件
	到达顺序），用于恢复时重建"正文↔工具卡↔Diff 卡"的时序交错；无 `segments` 的旧记录降级为
	按 `anchorMessageId` 沉底布局，不得丢失或篡改数据。前端恢复时先拉 `agent_sessions` 元数据，再拉当前激活会话的
	turns；其他会话点击后按 `session_id` 懒加载。恢复 `diffRefs[]` 时必须按 `diffId` 查询 DE 当前状态：
	`pending` 可继续裁决，`accepted/rejected/expired` 只读展示，DE 找不到的 diff 只读标记为不可用，不得默认
	复活为 `pending`。
	`ag.delete_session(session_id)` 以当前 workspace + 指定会话为边界删除该会话历史记录；成功后前端从
	历史/标签列表移除该会话，若删除的是活动会话则切换到剩余会话，没有剩余会话时新建空会话并持久化。
	该命令不触碰 DE/WS 的修改建议状态。

## §3.2 消息流渲染与时序（AG-TURN 展示子域）

<!-- RULE
rule_id: BR-AG-UI-001
主链路: AG-TURN
域: UI
-->
代理编辑面板的消息流呈现必须满足：assistant 消息正文按 GFM Markdown 子集渲染（段落、标题、
加粗/斜体/删除线、行内代码、围栏代码块、有序/无序/任务列表、引用、分隔线、表格、链接），
用户消息与错误消息保持纯文本；渲染前必须经 sanitize 白名单过滤（禁 script/iframe/事件属性，
react-markdown 默认不渲染原始 HTML 作第一道安全底）；**回合内时序**：文本段、ToolCallCard 段、
DiffCard 段按事件到达顺序交错排列（TERM-AG-011 消息片段），不整体沉到消息正文底部；
Mermaid/流程图语法**当前不渲染**——mermaid 围栏块降级为普通代码块展示，提示词同步声明无图表能力；
链接仅允许 http/https 且默认阻止 WebView 内导航（防应用被导航出界）。渲染为纯展示层，不改变
回合/工具/持久化语义；历史回合无片段信息时降级为消息后沉底布局且不丢失数据（承接 BR-AG-PERSIST-001）。

## §4 工具调用循环（AG-TOOL）

<!-- CHAIN
chain_id: AG-TOOL
主责模块: AG
status: active
risk_tier: P1
integration_test: deferred: phase-5
-->

<!-- RULE
rule_id: BR-AG-STATE-002
主链路: AG-TOOL
域: STATE
-->
工具调用循环必须满足：模型只能从当前 Prompt 暴露的工具中自主选择；每个工具请求执行前，运行时都按
最新事实重新计算 CapabilityGrant，并同时校验注册、workspace/目标、参数、副作用、格式/动态能力、
DecisionRequest、预算、取消与终态。IntentResult/TaskPlan、模型文字、历史授权和注册表存在性均不得
放行工具；未注册、当前未授权或越权请求一律拒绝并以 ToolResult 回注模型。执行结果（含失败）追加为
TurnEvent 与 EvidenceLedger 证据后，模型可调整 IntentResult 与后续路线；任何 provider 续调仍重新经过
Gate。同一响应内的多个 ToolCall 按响应顺序逐个重新计算授权并执行；中间项经 nextToolCall 保持同一
SM-AG-02 激活，最后一项 recorded 后才回归 Assembling；不得直接改写状态或绕过转移引擎。循环有运行时上限，
达上限中止；用户可随时中断，中断为终态。**任何文件内容写入意图不得由工具
直接落盘，必须产出修改建议走 DE diff 路径**（X-FORBID-006）；只读类工具（读文档/搜索/列块）必须经
AgentReadContext 并限定 workspace 内（X-FORBID-004）：已打开文档优先读取 OpenDocSnapshot 的 LogicalState，
未打开文档才经 WS/DOC 读取 DiskState；已打开文档快照缺失、版本缺失或锚校验失败时必须拒绝工具结果
或阻断回合，不得用 DiskState 兜底；
**工作区资源工具**（创建文档/文件夹、删除、重命名、移动、恢复）不属内容写入，但也不得从模型工具调用
直接执行：工具只能生成不可变 WorkspaceOperationPlan 与 DecisionRequest；用户确认后由 WS 按
BR-WS-STATE-003 重新核验并执行。模型参数、工具返回或 Prompt 文案均不能伪造用户确认。批量执行按
BR-WS-PERSIST-003 失败即停并逆序补偿，重启后的未完成计划进入 Interrupted 且不自动执行。
用户明确要求新建且有初始内容时使用 CreateDocumentBundle：初始内容在确认前生成，确认后先创建目标 DiskState，
再进入普通内容 Diff；明确零字节 TXT 使用同一 `plan_workspace_operation` 的单文件无初始内容计划，确认后直接创建空文件，不生成无变化的 Diff；不得使用 `create_doc` 或 `propose_edit(delete_*)` 绕过资源计划。状态转移归
SM-AG-02（工具循环）与 SM-WS-04（资源执行）。
当前工具集合按硬能力分层计算：workspace 内只读工具仅在其读取范围和数据来源可校验时开放；内容 Proposal
工具还必须具备 ExplicitEditTargetSet 成员、可用基线、格式/动态能力和无终态/取消；资源计划工具只生成
WorkspaceOperationPlan 与 DecisionRequest，不产生副作用；WS 执行入口不暴露给模型，只消费真实确认回执。
`propose_edit` 目标必须已存在（内容 Diff 不得创建资源，WS-RESOURCE-WRITE-GUARD-01）；结构操作或富文档
高级工具只有目标、格式能力、动态快照与 RuntimeProfile feature gate 同时允许时才可进入清单。模型的
语义路线影响它选择哪个已开放工具，但不参与 CapabilityGrant 的授权交集，也不得通过伪造工具名、参数或
提示内容扩权。

BDocx 表格工具是该循环的受控特例：模型必须先调用 `list_tables` 读取活动编辑会话的
TableSnapshot，再以 `propose_edit(operation=table_ops)` 提交 TableOperation。底纹建议只允许
`set_cell_background(at,color)`，`color` 为 `#RRGGBB` 或 `null`；工具执行层必须校验会话版本、
内容哈希、表格指纹、坐标、动态能力与合并单元格左上锚。校验成功只生成 DE Diff，不直接改写
LogicalState 或 DiskState。

BDocx 图片工具同样是受控特例：模型必须先调用 `list_images` 读取同一次活动编辑会话的图片实例、
稳定 Block 和 workspace 内项目图片资源，再以 `propose_edit(operation=image_ops)` 提交
ImageOperation。执行层必须校验 sessionVersion/contentHash、Block 或稳定 imageId/当前 fingerprint、
项目相对路径与内容哈希，并服从每张图片的 capabilities、resizeBounds 与 moveDirections；插入仅允许
稳定 Block 前后的独立图片段落，删除仅删除文档图片节点，缩放只设宽度，上下移动只交换相邻顶层 Block，
布局只允许独立图片段落 left/center/right。同一图片单批可各含一次缩放、布局和移动，执行层基于同一
原始 fingerprint 预校验并以缩放→布局→移动的固定顺序生成原子载荷；删除必须独占。workspace 内非受控
PNG/JPEG 先经 `import_image_asset` 复制到受控图片根并重新 `list_images`，不得用 `move_doc` 搬走源图；
没有安全资产、锚或动态能力时调用 `edit_blocked` 结构化结束，不能伪造修改建议。`list_images` 的相邻
文本只用于文档流定位，不构成图片像素、OCR 或视觉识别。成功只生成 DE Diff，不得直接修改
LogicalState/DiskState，也不得删除或改写项目源文件。非活动 BDocx 会话必须明确拒绝。

BDocx 的静态工具能力不得继续由提示词、function schema 与执行器分别手写。三者统一消费
A-SYS-I-P-04 定义的 BDocxCapabilityManifest（TERM-SYS-002 / BR-SYS-DATA-006）：提示词负责如实
说明格式、代理可用/人工专属能力、Diff 等级和 DOCX 往返风险；工具 Schema 负责条件字段与枚举；
执行器在生成 PendingDiff 前负责同版本静态校验，并叠加当前
AgentEditableSnapshot/TableSnapshot/ImageSnapshot 的锚、范围、来源哈希和动态能力校验。manifest
无效或任一校验失败时必须回注错误，不得退回手写默认清单或先生成
不可执行 Diff。

富文档普通内容块采用与表格/图片相同的结构化原则：活动编辑器把同次 ProseMirror LogicalState
直接投影为层级 AgentEditableBlockSnapshot；container/atom 只供模型理解父子关系，只有声明
`replace_node` 的 content 块可写。模型提交 `block_ops` 时，工具层以 sessionVersion/contentHash、
blockId、fingerprint、expectedNodeType 和受控 node JSON 完成整批预检，并从可信快照补齐 beforeNode。
ED 在单一 ProseMirror transaction 中先处理同锚点的 replace/delete，再按提案顺序插入新块；forward
必须校验最终节点、稳定 blockId 与数量，失败不得 dispatch；reverse 必须按插入 afterNode 的稳定 blockId
删除实际插入节点，不得解析插入操作的空 beforeNode。BDocx 的 `replace_block` 始终拒绝；`
replace_document` 仅当当前 LogicalState 没有表格且没有图片时可用于完整 HTML 整篇改写，执行器同时拒绝
既有或新引入的表格/图片，其余 BDocx 内容分别走 block_ops/table_ops/image_ops。HTML 字符串扫描只保留
未打开文档只读兼容，不再承担受保护结构的编辑或重建职责。

<!-- RULE
rule_id: BR-AG-DATA-011
主链路: AG-TOOL
域: DATA
-->
ToolCall 回执必须由同一次执行授权身份构造，并与模型回注、事件投影保持同一事实。输入为已校验并通过
执行边界的 ToolCall（workspace/session/turn/tool_call_id/tool_name）、本次重新计算的 CapabilityGrant、
执行结果或失败、本次调用实际产生的 PendingDiff/DecisionRequest 与成功只读执行的 evidenceIds；输出为唯一
ToolCallReceiptV1（attempt=1，身份与 grantId 来自同一授权计算）、只含本次实际产物的 references、
模型可见的 tool 消息（完整 toolResult + evidenceIds + executionReceipt）；公开事件复用同一回执与
evidenceIds。字段形状与枚举
归 BR-SYS-DATA-008，本规则只约束 AG 的构造与投影。禁止：成功标志与 validation_allows_execution 不一致；
把未创建产物写成 references；为失败调用伪造 evidenceIds；把原始错误正文、文档正文或凭据写入回执；
以另一个 grant 或另一身份重建回执。失败信号：执行前拒绝为 rejected/blocked/not_started；
执行器失败保留真实 IpcError code 且不得写成 succeeded；有副作用的失败保持 effect=unknown 与
inspect_effect，不自动重放。

<!-- RULE
rule_id: BR-AG-DATA-012
主链路: AG-TOOL
域: DATA
-->
propose_edit 成功生成建议卡后，给模型的工具结果必须如实区分「已生成待裁决卡」与「已预览/已接受/已保存」。
输入为本次 execute_proposal 的 proposal_batch_id、rel_path、projections，以及预览 checkpoint 是否真实建立；
输出为一个结果投影：summary 的卡数由 projections 长度派生，diff_ids 按 projections 顺序逐项对应，
proposal_status=pending_user_review、editor_preview_status=not_reported、disk_persistence_status=not_saved，
preview_checkpoint_required 反映真实 checkpoint 结果。禁止：声称已应用、已保存或已收到编辑器预览；
卡数与 diff_ids 数量不一致；丢弃、重排或截断 projections；checkpoint 建立失败时仍标记为可续写成功。
失败信号：execute_proposal 失败沿用其错误 code 并回注「生成修改建议失败」，不得为该结果生成
pending_user_review 投影；checkpoint 建立失败只清空
preview_checkpoint_required，Diff 仍作为当前回合待裁决提案可见，不得伪造成功续写。

<!-- RULE
rule_id: BR-AG-DATA-013
主链路: AG-TOOL
域: DATA
-->
本轮模型响应的控制封套必须在产生任何 Intent/TaskRun 投影或工具执行前完成终态前置校验。输入为网关
RoundOutcome、已流式解析的 ModelOutputDecoder、budget_closing、workspace_plan_completed、可见正文字符数、
any_tool_called 与 continuation_messages；输出为 Some(可用的 ModelTurnControl)、失败轮次的 None，或明确
IpcError。禁止：收尾态接受
非 final 控制；把缺少原生 ToolCall 的 nextAction=tool 当工具请求；接受无可见正文的 final；在结构计划缺失
且未完成时接受无 completion 的 final；在本轮已进入 final-only 纠正时重新接受工具批次。失败信号：
AG_MODEL_CONTROL_INVALID（收尾非 final、tool 无原生调用、final-only 下工具批次）、AG_MODEL_FINAL_EMPTY
（无可见正文）、AG_WORKSPACE_PLAN_TOOL_REQUIRED（结构计划缺失）。校验失败不得写入 IntentResult、
TaskRun 或执行任何工具。

## §5 上下文组装（AG-TURN）

<!-- RULE
rule_id: BR-AG-DATA-001
主链路: AG-TURN
域: DATA
-->
回合上下文的来源限定为：已打开文档 OpenDocSnapshot（LogicalState）、未打开文档 DiskState 读取结果、
REF 注入载荷（@mention 与 ReferencePayload，含来源标识）、用户在当前 workspace 当轮明确选择的项目记忆快照（经 REF 作 data-only 引用进入；其管理归 AST）；
一切 workspace 内容读取限定 workspace 内（X-FORBID-004）；超出上下文预算时裁剪策略必须确定性且用户显式选定的注入优先保留，裁剪发生时对调用
可追溯（不静默丢弃显式注入）。预算数值与裁剪算法延实现期标定，本规则锁来源限定与裁剪优先级语义。
已打开文档读取存在硬优先级：`source=active_editor/active_editor_session/open_editor_session` 的 LogicalState > 未打开文档
DiskState；任何已打开文档不得从 DiskState 或 loadedContent 旧快照取上下文主事实。ReferencePayload.selection
必须在作为当前文档事实或编辑锚使用前按当前 AgentReadContext 校验；无法校验的 selection 只能作为用户原始选区快照呈现，不得作为当前
文档事实。
`runtime_profile=agent-edit-1.0` 下 AST/KB/TPL 不做自动写入、检索或注入；只有用户当轮明确选择的项目记忆且来源可校验时，才可作为 data-only 引用进入本轮。未来 Profile 不得从 BR-AG-DATA-005 推导自动补强语义，须重新进入 REQUIREMENT/DESIGN Scope。

<!-- RULE
rule_id: BR-AG-DATA-008
主链路: AG-TURN
域: DATA
-->
每次 PromptPackage 装配前必须冻结版本化 AgentContextFacts（TERM-AG-020），其共享字段、版本及
revision 归 BR-SYS-DATA-008。v1 保持完整快照准入；v2 先冻结 workspace/session/turn、活动目标及
打开资源身份和可用性，不因无关资源正文未就绪阻断通用准入。前端只从 ED/DE/WS 真值及用户显式
SelectionRef 组装；后端校验身份及可用性。只有声明 ready 的文档才可作为 LogicalState 内容事实，
其 source/sessionVersion/contentHash 必须与实际 OpenDocSnapshot 一致；loading/unavailable 只能表示
已知资源身份及阻塞原因，不得伪装已读。所需文档按需采集；已打开但快照缺失时禁止回退 DiskState。
同一 AgentTurn 每次事实更新追加 revision 并重新装配、重新授权，不覆盖旧 ContextFrozen 证据。
选区只在当前任务显式携带 SelectionRef 时
进入，且必须带 relPath/blockId/range；兼容 `blockRefs` 必须先等价归一为本轮显式 SelectionFact。
Proposal 事实只能投影当前 AgentSession，携带 diffId/sessionId/relPath/state/effectStatus/saved，不携带或改变内容。

AgentContextFacts 作为独立 `AgentContextFacts` data-only Prompt 层注入，位于当前 IntentResult 投影之后、
CurrentDocument 之前；不得并入 RuntimeControl、OpenDocSnapshot 或自然语言提示词。载荷缺失、版本未知、
跨 workspace 或冻结活动身份不一致必须失败关闭；v2 的显式 unavailable 不等于坏载荷。需要消费的文档
来源或版本/hash 不一致时，在该内容注入或 ToolCall 执行前拒绝/重新获取；必需上下文仍不可用时明确失败，
不以旧快照继续。绑定目标不得因用户切换活动标签改变。
该载荷只供模型认识会话事实并由 ContextFrozen TurnEvent 初始化 EvidenceLedger，不授予额外读取、编辑、Proposal 或磁盘权限，
也不得把打开标签、读取结果或 Proposal 路径加入 ExplicitEditTargetSet。

<!-- RULE
rule_id: BR-AG-OBS-001
主链路: AG-TURN, AG-TOOL
域: OBS
-->
运行时必须从 ContextFrozen 与成功只读 ToolResult 的 TurnEvent 确定性派生 EvidenceLedger；模型输出、失败
ToolCall、目录枚举和搜索命中只能登记它们实际证明的范围，不能自报为完整正文读取。Inspect/Search/Analyze
类回合完成时，运行时把当前 EvidenceLedger 与模型最终回答关联，派生版本化 ProjectReport（TERM-AG-021），
至少包含 requestedScope、readItems、unreadItems、findings 与 uncertainties；每个 readItem 必须声明 relPath、
source、coverage、version/hash 并能反查证据事件。未被证明的路径只能进入 unreadItems 或 uncertainties。
成功读取结果必须把运行时生成的 evidence_id 连同内容交给模型；模型最终输出可为结论声明 evidence_ids，
运行时只接受当前回合、截至当前 seq 且确实存在于 EvidenceLedger 的引用。未知、跨回合、覆盖等级不足或
未声明证据的结论不得写成已证实 finding，只能进入 uncertainties；该结构化关联不是模型完成工具。

ProjectReport 是只读呈现投影，不是模型工具、完成门或新的事实输入：不创建 TaskRun/DecisionRequest/
ChangeProposal/Diff，不写 LogicalState/DiskState，不扩大 CapabilityGrant 或 ExplicitEditTargetSet。若模型回答
声称的覆盖高于 EvidenceLedger，运行时必须按真实覆盖降级报告或标记不确定性，不得伪造成功读取。

## §5.1 提示词 / 上下文组装架构（AG-TURN，2026-06-12 补 B-1）

> 解除 A-CORE-X-R-01 B-1：提示词**文本/措辞/模型档**属实现期运营配置（不入规则）；本节锁设计层
> 的**组装骨架**——分层、注入顺序、当前两类回合的结构差异。
>
> **L3 细化与版本优先（2026-06-24，迭代版）**：组装骨架的完整分层（含引用/记忆/知识/模板等
> binder 特有层）、当前轮次原则与意图沿用判定、补强层污染隔离与**分层截断序**，由 L3 专项
> A-AG-M-T-02_提示词架构设计.md 细化（BR-AG-DATA-004/005）。**职责单一化（去重）**：本节（BR-AG-DATA-002）
> 自此只锁**组装骨架**（分层/注入顺序/当前两类回合差异），**截断序统一归 M-T-02 BR-AG-DATA-005**，
> 不再在本节内嵌裁剪优先级排序。冲突裁决按职责分开：组装骨架与层序以本文 BR-AG-DATA-002 为准；
> 当前轮意图沿用以 BR-AG-DATA-004 为准；数据隔离与截断序以 M-T-02 BR-AG-DATA-005 为准。

**组装分层**（自上而下）：

| 层 | 内容 | 性质 |
|---|---|---|
| 系统提示词层 | 产品角色定位 + 硬约束边界声明（修改须经 diff 裁决、不得越权落盘、workspace 限界） | 设计锁边界，文本实现期 |
| 工具/能力定义层 | 当轮 CapabilityGrant.allowed_tools 的精确契约投影；注册表/Profile 不得扩大允许集 | 允许集为当轮动态真值；具体工具实现归运行时 |
| 上下文注入层 | 来源限定承 BR-AG-DATA-001；v2 按需取得专业上下文，已激活文档内容必须为有效 OpenDocSnapshot LogicalState；显式引用不被静默丢弃，显式记忆保持 data-only，自动补强关闭。P0 保留义务不等于所有请求必须注入当前文档；详见 M-T-02 §5 | 顺序设计锁；激活与截断细化归 M-T-02 |
| 用户输入层 | 本轮 prompt / 编辑触发上下文 | — |

**实现装配登记（2026-08-09，AG-PROMPT-STRUCTURE-FIX-01）**：生产装配以 `LayerKind` 枚举作为 BR-AG-DATA-002 的唯一实现投影
（装配顺序、审计层名、版本登记共用），取代早期仅供测试的 `assemble_prompt`/`PromptSkeleton` 四层骨架。
固定层序为 `System → RuntimeControl → IntentHint → AgentContextFacts → CurrentDocument → Reference → History →
CrossDocHistory → User`（CrossDocHistory 仅显式沿用时注入）；`RuntimeControl` 独立为结构化 system
消息（与自然语言提示词分离，P2 修复）；历史聚合为单块 DATA-ONLY 消息并保留 role 序列（P5/P7 修复）；
`system_prompt_version` 由提示词内容 hash 派生（P3 修复）；预算 gate 使用序列化 messages+tools 统一口径
（P4 修复）；审计直接读取 `message.layer` 字段（P6 修复）。规范真源仍是 BR-AG-DATA-002；`LayerKind` 必须投影它，不得反向修改文档语义。

**实现登记（2026-08-11，AG-WS-LAYERED-01 模型分层感知工作区）**：`list_docs` 分层——无参数 → 概览（mode=overview：顶层目录+文件计数+总文件数；豁免 ≤30 文件且 ≤4KB → mode=full 全量）；`path` → 该目录一层直接子项（mode=children）。树枚举只含索引（路径/类型/计数），正文唯一入口 read_doc/search_text；前端 ws.list_docs 与 search_text 内部遍历保持全量语义。prompt 分层引导 + CreateDocument 专项。语义详见 A-AG-M-T-02 §4。

**历史实现登记（2026-08-11，AG-TASK-01）**：当时登记了多提案批次、`block_ops.delete_node` 与提示词单批上限从 24 调到 32。该数值不再构成设计常量；当前批次/操作上限只由运行时 capability/config 持有，Prompt 必须投影同一值，不得在文档或提示词另设数字。多提案与删除的有效语义仍以 Active 规则、DE/ED 契约和运行时能力为准。

**实现登记（2026-08-13，AG-STRATEGY-05 文档改写安全闭合）**：BDocx capability manifest、工具 schema、运行时声明和服务端 guard 统一为「无表格且无图片才允许整篇 `replace_document`」；表格或图片存在、或新 HTML 引入其中任一结构时，返回 `AG_BDOCX_REPLACE_DOCUMENT_REQUIRES_STRUCTURED_OPS`，不产生 PendingDiff。`block_ops` 修复同锚点插入/替换互相覆盖：replace/delete 先写、插入后写，同坐标多插入按原 proposal 顺序展示；forward→reverse 保证恢复原 LogicalState。`propose_edit` 工具结果只确认 pending proposal，前端以 `queued/applied/failed` 单独投影草稿预览并输出不含正文的 `[de-draft-preview]` 运行事件；接受仅确认、保存才写盘，卡片不得提前宣称完成。

**实现登记（2026-08-11，AG-CTX-01 上下文管理重构）**：上下文控制从"事后截断"改为"源头轻量投影+按需读取"——`list_blocks` 轻量投影（块目录，无 node_json/全文）、`read_doc` 支持 `blocks` 范围参数（只返回目标块完整投影含 node_json，向后兼容）、移除工具结果事后截断；序列化省略空 marks/attrs。历史关键词意图与工具分组不再构成现行设计，统一由模型语义投影与 CapabilityGrant 硬边界取代。

<!-- RULE
rule_id: BR-AG-DATA-002
主链路: AG-TURN
域: DATA
-->
回合的提示词与上下文必须按统一组装骨架构建：系统提示词层（声明产品角色与硬约束边界——修改经
diff 裁决、不越权落盘、workspace 限界）→ 工具/能力定义层 → 上下文注入层（来源与注入顺序承
BR-AG-DATA-001；完整分层、当前轮次优先与**截断序**归 A-AG-M-T-02 BR-AG-DATA-004/005，冲突以
M-T-02 为准）→ 用户输入层；`agent-edit-1.0` 的两类回合采用结构骨架差异——**编辑回合**（代码名 TurnKind::Chat 沿用；
含对话历史 + 修改建议导向 diff 产出）、**辅助回合**（辅助续写/局部修改；轻量、无系统对话历史；辅助续写与局部修改必须
按 assist_kind 使用不同 system prompt，避免 3 候选 JSON 契约与单正文改写契约混用）。提示词文本/措辞/模型档属实现期运营配置不入规则；
本规则同时锁可信边界：只有产品不变量、运行策略和经本轮允许的工具契约可进入 system instruction；
当前文档、显式引用、历史、用户明确选择的项目记忆和工具结果必须作为 data-only PromptDataEnvelope 注入，并保留
来源、freshness、hash、trust 与 token 元数据，不得因数据正文含命令文字而获得指令权限；`agent-edit-1.0`
不得自动检索或注入记忆、知识、模板。每次调用形成
带 runtime/version、当前 intent revision、context trace、tool manifest hash 和预算结果的 PromptPackage；调试追踪
默认不保存用户正文、密钥或会话令牌。专业指令由可信能力实现按当前激活条件提供，不永久并入全局系统
常量；文档或历史中出现的专业指令文字仍为 data-only，不重新获得权限。本规则锁组装分层、注入顺序、可信边界与当前两类回合骨架差异
（**截断序不在本规则，归 BR-AG-DATA-005，去重单一化**）。Build/CBT 的 Prompt 骨架不属于
`agent-edit-1.0`，由未来独立 Profile 的需求与设计定义。

## §5.2 Agent 1.0 记忆关闭边界（AG-TURN）

<!-- RULE
rule_id: BR-AG-STATE-004
主链路: AG-TURN
域: STATE
-->
`runtime_profile=agent-edit-1.0` 的回合终态不得触发项目记忆提议、写入、检索或注入；会话历史、编辑历史、
外部文件与工具结果也不得自动转存为项目记忆。只有用户在当前 workspace 明确选择的既有项目记忆条目，
才可作为当轮 data-only 引用进入上下文。代码中保留的 `ag/memory_link.rs`、`ast/capture.rs` 或工具注册不构成
激活依据，不得存在生产调用方。未来 Profile 若启用自动记忆，必须新开 REQUIREMENT/DESIGN Scope 建立
独立规则、状态机、接口和验证，不得从本规则推导启用语义。

## §6 辅助续写与局部修改（AG-ASSIST）

<!-- CHAIN
chain_id: AG-ASSIST
主责模块: AG
status: active
risk_tier: P1
integration_test: deferred: phase-5
-->

<!-- RULE
rule_id: BR-AG-STATE-003
主链路: AG-ASSIST
域: STATE
-->
局部修改与辅助续写回合必须满足：经用户快捷键**手动触发**（2026-07-04 澄清：非键入自动触发），
触发须防抖/防重并采用 latest-wins（默认 450ms 内重复快捷键静默忽略，不发起模型调用；防抖窗口后
再次触发视为刷新候选，重新捕获当前锚点并覆盖旧 requestId，旧响应不得展示或应用）；辅助续写一次
**严格返回 3 个候选**（每个 50–100 字量级，Tab/1/2/3 采纳；不足 3 个视为失败，不展示半候选、
不允许采纳，但前端保留轻量空态，避免触发后浮层瞬间消失）；局部修改弹窗内**可多轮**（无选区时为光标插入；
上轮候选产出为下轮基底，只传上一轮结果与新指令，每轮逐次过门禁）；**辅助对话不持久化**（不入
回合历史，与代理编辑面板层的会话持久化形成注册分界）；每次调用同样经门禁与网关
（BR-AG-VERIFY-001，无例外路径，客户端不得缓存 gate pass）；候选经 ED 装饰层/overlay 语义呈现
（ED-DECOR），采纳位置固定为触发瞬间捕获的 DocRef/blockId/字符锚，不跟随采纳时光标；结果返回和
采纳前均校验文档、blockId、字符锚与触发时前后文指纹仍有效，锚失效放弃本次呈现；文字位置未变但
样式变化时按当前最新样式上下文采纳；**采纳是用户编辑操作**——经 ED 受控接口进入编辑模型并走
SM-ED-01→SM-WS-01 正常脏态/保存路径，不构成 AI 直写文件；无订阅/断网/调用失败时静默不提供，
不打断编辑、不弹错误。防抖参数与成本策略延实现期标定（AG-03 的数值部分），本规则锁机制语义。
前端触发层不得只依赖编辑器内部冒泡事件：`Cmd/Ctrl+J`、`Cmd/Ctrl+K` 需在编辑器宿主/窗口 capture
阶段兜底拦截，并在触发瞬间读取 ED 当前文档上下文；候选 overlay 的**逻辑锚点**为触发瞬间的
DocRef/blockId/字符锚，**视觉锚点**为当前光标矩形或选区 DOM 矩形。视觉放置必须以编辑纸张可视内容区
为边界，且必须叠加编辑器宿主/center 可见区右边界，右侧 gutter 与代理编辑面板不得遮挡浮层；编辑器
缩放只影响视觉锚点坐标，不得以缩放后的 ProseMirror 正文矩形压缩浮层控件尺寸；浮层布局边界采用
editor-paper 可视区与 editor-host 的交集，浮层保持应用 UI 尺寸，只有宿主真实可用宽度不足时才降级压缩。
浮层定位输入中的宽高采用稳定静态尺寸模型，尺寸应接近局部修改初始输入态与辅助续写候选态，禁止通过
ResizeObserver/useLayoutEffect 等运行时测量链反向驱动 assist 状态，避免首帧渲染、测量、重定位循环导致浮层消失。
滚动、缩放或窗口 resize 后，若逻辑锚仍有效，前端必须按当前视觉锚点重算浮层位置；若目标结构失效则
关闭并回写 `anchor_lost`。按目标
相邻优先而非窗口边缘钳制：辅助续写词条优先 `bottom-start`，当右侧可用空间不足或已接近右边界时
提前切换为 `bottom-end` / `top-end`；局部修改弹窗有选区时优先 `top-center` 贴近选区，无选区时按
光标点优先上方，空间不足翻到下方；横向溢出时在内容区内最小偏移，不得把浮层无关联地推到编辑器窗口边缘。
overlay 键盘确认仅在焦点仍处于
编辑器宿主或 overlay 内时生效，输入法 composition
期间不得采纳，避免工具栏、滚动容器、旧闭包或窗口级快捷键导致误确认/错位。局部修改弹窗已激活时，
再次触发 `Cmd/Ctrl+K` 视为放弃本次局部修改，必须关闭弹窗并丢弃未采纳预览，不得重开目标或发起新请求。
辅助续写和局部修改共享交互锚生命周期：浮层打开后，用户点击浮层外任意区域、通过鼠标/键盘移动光标，
或改变当前选区，均表示当前交互目标已离开触发瞬间捕获的锚点，前端必须立即关闭对应浮层并丢弃已返回
候选/预览；若模型请求仍在返回中，后续响应只能按旧 requestId 丢弃，不得再次展示、应用或写入历史。
该类用户放弃事件回写 `ag.finish_assist(dismiss)`；只有 blockId/range/前后文指纹等结构定位失效才回写
`anchor_lost`，避免把用户主动放弃与定位系统故障混为一类。
AG-ASSIST 的实现映射必须覆盖完整唯一链：编辑器快捷键捕获与锚点校验（Editor）→ 前端辅助状态机
（assistStore）→ IPC 服务封装（requestAssist/finishAssist）→ 后端 `ag_request_assist` 门禁与网关调用 →
候选解析和 `failure_reason` → Autocomplete/InlineAssist overlay 呈现与采纳。链路上的任一实现点不得只以
普通注释或测试说明承接规则；`failure_reason` 不得在 service/editor/store 层被吞掉。普通用户界面可继续
显示轻量空态，但内部状态、诊断日志或测试可观察点必须保留 `gate_denied`、`gate_offline`、`gateway_error`、
`empty_output`、`parse_underflow`、`stale_request`、`invoke_error` 等原因，避免所有失败被压缩为同一 UI 文案。
前端采纳、忽略/关闭、锚失效必须回写 SM-AG-03 终态（Accepted/Dismissed/AnchorLost）；终态回写只记录
辅助回合生命周期，不直接修改文档。
状态转移归 SM-AG-03。
**样式操作同语义（2026-07-03 裁定 D-3）**：用户显式触发的局部修改样式操作（StyleOp，
A-ED-M-T-01 §5.1）经 ed.apply_edit 视为用户编辑直改；chat/agent **回合产出**的样式修改不走
本路径，一律经 DE diff 裁决落盘（X-FORBID-006 / BR-ED-DATA-005），不得借内联语义绕过裁决。

### §6.1 模型注册表与能力偏好（AG-MODEL-CATALOG）

模型配置体系的客户端承载（REQ-AG-006）：模型注册表（来源分层）与四个能力偏好槽位
均由客户端维护（localStorage 偏好，P0-P1 沿用；统一设置体系并入后立项）；预设模型清单由云端
下发（GET /models，桌面端仅消费，TERM-AG-010）；调用时按能力偏好解析 provider/model 传入
既有 AG-ASSIST / AG-TURN 链路，门禁与密钥语义不变（X-FORBID-001/002、X-SEQ-001）。

<!-- CHAIN
chain_id: AG-MODEL-CATALOG
主责模块: AG
status: active
integration_test: deferred: phase-5
-->
模型注册表维护与预设清单拉取、能力偏好配置与解析、调用前按能力偏好解析 provider/model。

<!-- RULE
rule_id: BR-AG-DATA-006
主链路: AG-MODEL-CATALOG
域: DATA
-->
模型注册表条目必须携带来源分层：`source` 仅允许 `preset`（预设模型，云端下发、订阅可调用，
TERM-AG-010）或 `byok`（BYOK 模型，用户自带密钥、网关代持，TERM-CLOUD-004）；同一
`provider/model` 键在注册表中唯一；注册表只保存 provider/model/label/source 标识符与可用性标记，
**不保存任何密钥**（X-FORBID-001）；来源为 `byok` 的条目仅在对应 provider 密钥已上送网关时可用。

<!-- RULE
rule_id: BR-AG-DATA-007
主链路: AG-MODEL-CATALOG
域: DATA
-->
能力模型偏好解析必须满足：四个槽位（autocomplete / inline-assist / agent-edit / build）各自维护
一项偏好，值为「跟随全局默认」或「显式指定 modelKey」二者之一；解析结果必须是注册表内存在的
modelKey。仅首次安装、尚无选择记录时可以初始化默认模型；已保存的全局或能力显式选择即使因删除或清单同步而不可用，也必须保留原键并显式阻断调用，要求用户重新选择，禁止静默改为其他模型。
解析为**纯同步逻辑**；发起 AgentTurn 时冻结解析所得 provider/model，同一回合的澄清、裁决、预览及工具结果续行均使用该绑定，不按界面当前偏好或网关默认值重新解析。解析出的 provider/model 仅作为调用参数传入既有链路，
不改变门禁前置与服务端裁决路径（X-FORBID-002）。`build` 只保存后续 Profile 的偏好数据，不授予
Agent 1.0 入口、调用、工具或 RuntimeProfile 能力。

## §7 跨模块约束（CONSTRAINT）

<!-- CONSTRAINT
constraint_id: X-FORBID-006
摘要: AG 及其工具调用不得绕过两条唯一写路径：文档内容修改必须经 SM-AG-02 propose_edit→DE 内容 Diff 裁决→WS 保存/落盘；工作区资源操作必须经 WorkspaceOperationPlan→DecisionRequest 前置裁决→WS 重验并直接修改 DiskState。资源操作不得生成 DiffCard，内容 Diff 不得创建、删除、重命名或移动资源；CreateDocumentBundle 只在资源创建完成后把初始内容接入普通 Diff
涉及模块: AG, DE, ED, WS, REF, SYS, CBT
链路影响: AG-TURN, AG-TOOL, WS-DIFF-APPLY, WS-OPERATION
-->

### 7.1 Agent 1.0 运行时契约落地（REQ-AG-007/015）

承接 A-SYS-I-P-01 §3.1 共享对象契约与 SM-AG-01/04，AG 侧落地口径：

| 对象 | AG 侧落地 | 承接 |
|---|---|---|
| RuntimeProfile | `ag_start_turn` 入口显式校验 `runtime_profile`（1.0 仅 `agent-edit-1.0`）；缺失/未知在 Gate、provider、工具与 Proposal 前失败关闭，不产生可执行事件 | BR-SYS-STATE-007、A-AG-I-P-01 §2.2 |
| AgentTurn | 所有运行事实按序追加 TurnEvent；模型请求澄清时进入 AwaitingClarification，用户补充后原 AgentTurn 重新冻结事实并继续；DecisionRequest 进入独立等待，错误/中断重试仍创建新回合 | BR-AG-STATE-001、SM-AG-01 |
| IntentResult | 模型驱动、可随模型输出和 ToolResult 更新的语义投影；表达目标/候选/副作用意图/上下文/歧义/确认点，不含工具授权 | BR-SYS-STATE-007、TERM-AG-012 |
| CapabilityGrant | Prompt 暴露工具与每个工具执行前均由运行时按当前硬事实计算；注册表存在 ≠ 当前可调用；事实变化或动作完成即失效 | BR-SYS-STATE-007、BR-AG-STATE-002 |
| TaskRun/TaskPlan/TaskStep | 简单目标不创建；复杂目标从 TurnEvent 派生可观察投影，只描述真实进度、等待与模型当前候选路线，不调度工具、不持有授权；连续 Proposal 以最新 LogicalState 接力 | BR-SYS-STATE-008、TERM-AG-015/016/018 |
| TurnEvent | 用户可见事件只是 append-only 回合事件的安全投影；不展示模型内部推理，状态、恢复、EvidenceLedger 与任务投影均可由事件重建 | BR-SYS-STATE-007、TERM-AG-022 |
| EvidenceLedger/ProjectReport | 上下文和成功读取工具事件派生证据账本；运行时把证据与最终回答关联生成 ProjectReport，不调用 `report_project` | BR-AG-OBS-001、TERM-AG-021/023 |

> 授权非来源声明：已注册工具、Prompt、前端参数、兼容默认值、现有代码与测试绿灯均不构成当轮授权
> （X-SHARED-002，A-SYS-I-P-01 §5）。

**复用既有约束**：X-SEQ-001（门禁前置，A-SYS-I-P-03）、X-FORBID-004（workspace 限界）、
X-FORBID-005（不绕过 ED 接口）、X-SHARED-001（共享结构经 SYS）、X-SEQ-003/X-INTER-001
（建机前置与跨模块转移登记）。

### 7.2 TaskRun 可观察投影契约（承接 REQ-AG-015 / A-AG-M-S-01 §2.2）

本节只适用于模型当前路线与真实事件已经表明目标复杂、跨多个可见执行片段或存在等待点的回合。简单
目标只创建单个 AgentTurn，不创建 `task_runs`、TaskPlan、TaskStep 或空投影；不得由关键词分类预先建计划。

#### 7.2.1 数据与重建契约

- `agent_turn_events(event_id TEXT PK, workspace_id TEXT NOT NULL, session_id TEXT NOT NULL,
  turn_id TEXT NOT NULL, seq INTEGER NOT NULL, kind TEXT NOT NULL, payload_version INTEGER NOT NULL,
  payload_json TEXT NOT NULL, occurred_at INTEGER NOT NULL)` 是追加式运行事实表；同 turn 的 seq 唯一递增，
  已提交事件不可覆盖或单项删除，删除整个 AgentSession 时随历史一起删除；
- `task_runs`、`task_plans`、`task_plan_steps` 为可丢弃并重建的查询投影：只保存/缓存目标、当前状态、已发生
  工作、等待点、模型当前候选路线与结果引用，不保存 `required_grant` 或执行游标；
- `decision_requests` 是独立业务事实，必须绑定 `turn_id`，可选绑定 `task_run_id`；投影丢失不影响裁决恢复；
- IntentResult、EvidenceLedger 与 ProjectReport 同样从 TurnEvent 及其引用的上下文/ToolResult 业务事实重建。

#### 7.2.2 事件契约

- 每个 TurnEvent 至少携 `event_id/turn_id/seq/kind/occurred_at/payload_version/payload`；事件 kind 覆盖输入、
  context、Gate、provider、intent revision、工具、DecisionRequest、Proposal 与 terminal；
- `step_progress` 是已发生事件的 UI 投影，不是可执行命令；`route_updated` 只记录模型当前候选路线，不能
  创建授权或自动启动下一步；
- `decision_request_proposed/resolved` 保留真实裁决；confirm 只使原回合具备重新 Gate/计算授权的条件；
- `diff_proposed` 只驱动 Diff 生命周期，不驱动 TaskRun 或 provider 自动续调。

#### 7.2.3 UI 契约

- 不展示独立顶部 TaskRun 控制卡；任务投影只在复杂任务中以轻量进度/等待状态呈现；
- ToolCallCard / DiffCard 与当前动作均从同一事件序列按时序投影，恢复后顺序一致；
- 等待确认 / 等待补充 / 完成 / 失败 / 中断必须来自真实事件，等待期间无 provider 或工具在后台运行；
- DecisionRequest 卡裁决原业务事实；TaskRun/TaskPlan 仅随事件更新，不接收“执行下一步”命令。

### 7.3 WorkspaceOperation 与 TaskRun 边界（2026-08-21）

- 资源整理步骤由 AG 生成 WorkspaceOperationPlan，DecisionRequest.payload 只引用冻结的 plan_id、摘要、影响与基线；不得把可变工具参数当执行计划。
- AgentTurn 在模型提出资源计划后创建 DecisionRequest 并进入等待；TaskRun 若存在只投影这一事实。资源操作的核验、磁盘执行、补偿与 WorkspaceTimeline 归 WS。
- `userConfirm` 恢复原 AgentTurn；运行时以当前终态/取消状态、真实确认回执和冻结 plan 重新计算允许的副作用路径，通过后才进入 WS Revalidating/Executing。该内部执行入口不暴露给模型，不从 TaskPlan 自动调度，也不增加 `WaitingUserCommit`、资源 DiffCard 或第二次保存确认。
- `userCancel` 终结该资源请求且 DiskState 不变；是否继续其他工作由模型在新证据下重新表达路线，不能由旧 TaskPlan 静默跳过。
- 应用重启后，未完成 WorkspaceOperation 进入 Interrupted；TaskRun 可显示中断和提供“重新计划”，但不得自动重放旧确认或执行旧计划。
- CreateDocumentBundle 的资源创建完成后，初始内容 Diff 只是普通内容提案事件；它不把 TaskRun 再次置为 WaitingConfirm，也不因计划中存在后续候选工作而自动继续。

## §8 需求到规则映射（REQ_MAP）

当前映射边界：REQ-AG-001～009、011、013、015、017 已由本文、A-AG-M-S-01 或已列明的跨模块 Active 规则承接。
REQ-AG-010（网络研究）、012（格式转换）、014（多个既有文档）、016（外部文件内容）仍为
`deferred / DESIGN_PENDING`，不得从代码、工具注册、Prompt 或历史情况说明推导设计完成；补齐时只能在独立 DESIGN Scope
新增或映射规则。本节只登记真实存在的映射，不为追求全量映射伪造 RULE。

<!-- RULE
rule_id: BR-AG-DATA-009
主链路: AG-TOOL
域: DATA
-->
propose_edit 的操作载荷必须完整且互斥。输入为一次 propose_edit 的 params；输出为唯一一个
AgentEditOperation。每种操作必须满足：block_ops 提供 base 与非空 operations；replace_block 提供
block_id 或 block_index，并提供 new_text；replace_document 提供 content；style_ops、table_ops、image_ops
分别提供各自载荷。禁止未知 operation、禁止把不同 operation 的载荷字段混在同一请求（例如同时带
content 与 operations）；字段可推导时按固定优先级推导，推导不出即失败。失败信号统一为 AG_TOOL_ARG，
消息必须指出缺失或冲突的字段名，且不得回显载荷内容。

<!-- RULE
rule_id: BR-AG-DATA-010
主链路: AG-TOOL
域: DATA
-->
block_ops 单次提交必须受数量与字节双重预算约束，且预算只有一处来源。输入为 operations 数组；
输出为通过校验的待投影操作序列。数量上限为单次调用上限（默认 320，按每卡 64 拆卡），序列化字节
上限为 64 KiB；模型可见 schema 的 maxItems 与运行时判定必须引用同一常量，不得各自写死数字。
超过任一上限时整批拒绝，失败信号为 AG_TOOL_ARG，消息必须包含实际数量或字节、上限值，以及可执行的
拆分区间；禁止部分接受、禁止静默截断。正文尾部追加必须使用 append（无锚点），不得以同一末块重复
insert_after 表达顺序追加。

<!-- REQ_MAP
req_id: REQ-AG-001
target_type: CHAIN
target_id: AG-TURN
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AG-001
target_type: RULE
target_id: BR-AG-STATE-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AG-001
target_type: RULE
target_id: BR-AG-PERSIST-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AG-002
target_type: CHAIN
target_id: AG-TOOL
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AG-002
target_type: RULE
target_id: BR-AG-STATE-002
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AG-002
target_type: CONSTRAINT
target_id: X-FORBID-006
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AG-003
target_type: RULE
target_id: BR-AG-VERIFY-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AG-004
target_type: RULE
target_id: BR-AG-DATA-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AG-005
target_type: CHAIN
target_id: AG-ASSIST
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AG-005
target_type: RULE
target_id: BR-AG-STATE-003
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AG-004
target_type: RULE
target_id: BR-AG-DATA-002
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AG-017
target_type: RULE
target_id: BR-AG-STATE-004
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AG-001
target_type: RULE
target_id: BR-AG-DATA-003
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AG-001
target_type: RULE
target_id: BR-AG-UI-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AG-006
target_type: CHAIN
target_id: AG-MODEL-CATALOG
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AG-006
target_type: RULE
target_id: BR-AG-DATA-006
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AG-006
target_type: RULE
target_id: BR-AG-DATA-007
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AG-008
target_type: CHAIN
target_id: WS-OPERATION
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AG-008
target_type: RULE
target_id: BR-WS-STATE-003
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AG-011
target_type: CHAIN
target_id: WS-OPERATION
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AG-011
target_type: RULE
target_id: BR-WS-PERSIST-003
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AG-013
target_type: RULE
target_id: BR-WS-DATA-003
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AG-009
target_type: CHAIN
target_id: AG-TURN
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AG-009
target_type: RULE
target_id: BR-AG-DATA-008
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AG-009
target_type: RULE
target_id: BR-AG-OBS-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AG-009
target_type: CONSTRAINT
target_id: X-SHARED-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AG-009
target_type: CONSTRAINT
target_id: X-SHARED-002
status: mapped
-->

## §9 测试与验收

- **门禁前置无绕过**（BR-AG-VERIFY-001，REQ-AG-003 验收）：静态可查不存在不经 GatePassed 的
  模型调用路径；无订阅时 chat 明确拒绝、assist 静默；断网时本地功能不受影响。
- **回合生命周期**（BR-AG-STATE-001 / SM-AG-01）：TurnEvent 序号单调、只追加且可重建状态；回合可中断且无半落盘；修改建议唯一出口交 DE；
  DiffCard 带会话/回合/消息锚点并显示在对应 assistant 消息后；失败可重试，首终态后事件不得复活执行。
- **工具循环**（BR-AG-STATE-002 / SM-AG-02）：模型能按 ToolResult 调整路线；每次工具请求重新计算 CapabilityGrant，IntentResult/TaskPlan 不扩权；未注册/越权工具被拒并回注；循环上限生效；注入写盘
  意图的工具调用不产生旁路写入；内容走 DE Diff，资源只生成 WorkspaceOperationPlan 并等待 DecisionRequest。
- **资源操作编排**（WS-OPERATION / SM-WS-04）：确认前零副作用，确认后原 AgentTurn 调用 WS 重验执行；TaskRun 只投影事件；拒绝、过期、
  脏标签阻断、fail-fast 补偿、partial_failed、重启 Interrupted 与 CreateDocumentBundle 安全拒绝均有跨模块验证。
- **上下文限界与裁剪**（BR-AG-DATA-001）：越界读取被拒；裁剪确定性且显式注入优先保留、可追溯。
  已打开文档覆盖测试必须包含：active live 内容覆盖 `loadedContent`、非激活 open session 内容覆盖 DiskState、
  标签切换过渡期 active binding 未对齐时读取目标 session LogicalState 且回写旧绑定文档、session 尚未载入时短暂等待、
  `search_text` 命中未保存 LogicalState、ReferencePayload.selection 失效时不作为当前事实。
- **编辑器事实与项目报告**（BR-AG-DATA-008 / BR-AG-OBS-001，REQ-AG-009 验收）：AgentContextFacts
  缺失、版本未知、workspace/活动路径/LogicalState 版本不一致均在 Gate 前拒绝；EvidenceLedger 只能从初始
  注入或成功读取事件派生，ProjectReport 无模型完成工具也能确定性重建；目录/搜索片段不得伪装成完整正文，
  模型过度声称时必须降级覆盖或标记不确定性；读取与报告不增加任何编辑目标或写工具。
- **辅助回合**（BR-AG-STATE-003 / SM-AG-03）：latest-wins 生效（新请求覆盖旧请求）；辅助续写
  返回 3 候选且 Tab/1/2/3 采纳正确；局部修改支持选区替换与光标插入，多轮仅以上轮候选为基底；
  采纳走 ED 受控接口产生正常脏态；无订阅/断网静默。
- **历史持久化**（BR-AG-PERSIST-001）：重启后会话历史可恢复；无历史内容网络上行（可经网络核验）。
  错误、取消、门禁拒绝和 draft 均可见；draft 显示“未完成，可重试”，重试复用同一 user message 新建回合。
- 量化 NFR（首 token 延迟、补全延迟、节流参数）延 G5 标定（AG-05）。

## §10 技术债务与盲区（诚实声明）

- **工具注册清单的具体内容**（AG-02 余项）：动态授权机制与写盘禁令已锁，清单仍由运行时注册表与
  format capability 持有；新增工具必须另行进入需求/设计，不得从当前注册表反推产品能力。
- **历史后台压缩策略**：200MB 软上限、软归档与单会话删除已锁；后台压缩策略后续按数据库体积表现再定。
- **已打开非激活文档样式类自动应用**：文本/块替换由 ED session 事务承接；StyleOp 若缺少无 ProseMirror
  runtime 的可逆事务实现，必须让 DiffCard 失效并记录债务，不能降级为 DiskState 应用。
- **节流与成本参数**（AG-03 余项）：机制已锁，数值延 G5 标定。
- **P2 Build/CBT**：不属于 `agent-edit-1.0`；本文不持有接管/回落协议。未来启用须独立 Profile 与新的需求、设计和验证。
- **多个既有文档**：P1 需求已在 REQ-AG-014 冻结，当前 L2 仅有 ExplicitEditTargetSet/DecisionRequest 的部分公共边界，完整规则仍为 `DESIGN_PENDING`；不得退回已被取代的“跨文档一律升级 CBT”口径。
- **网络研究、格式转换、外部文件内容**：REQ-AG-010/012/016 已冻结但尚未完成 AG L2 全量映射，当前不得以工具存在或实现片段宣称设计完成。
- 模型选择、提示词、上下文预算数值属实现期/运营配置，不入规则。

## §11 后续设计入口

1. Active 规则、状态机与接口的现有差异在对应 DESIGN Scope 内修正，并以本文件、A-AG-M-S-01、A-AG-I-P-01 分层持有真值。
2. REQ-AG-010/012/014/016 只能在各自独立 DESIGN Scope 补齐 REQ_MAP、状态/接口（如需要）和验证卡；在此之前保持 `DESIGN_PENDING`。
3. Build/CBT、自动记忆/知识、模板工作流等 P2 路线不得复用本节作为启用依据，必须先回到 REQUIREMENT Scope。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-10-30 | v2.65 | 补齐 BR-AG-STATE-002 的同批多 ToolCall 语义：中间项经 nextToolCall 保持同一 SM-AG-02 激活，最后一项 recorded 后回归 Assembling，禁止直接改写状态绕过转移引擎；与 A-AG-M-S-01 v2.37 的 nextToolCall 转移同步。 |
| 2026-10-30 | v2.64 | 继续细化 turn_tool_loop：新增 BR-AG-DATA-013（控制封套终态前置校验）并抽出 validate_round_response；把重复的 Intent/TaskRun 投影收敛为 apply_control_and_observe（BR-SYS-STATE-008），把 ProjectReport 派生收敛为 emit_project_report（BR-AG-OBS-001）；修正裁决停放失败取消事件使用真实 turn_state；不改变授权、工具执行、裁决与保存语义。 |
| 2026-10-30 | v2.63 | AG-TOOL/AG-TURN 规则细化：新增 BR-AG-DATA-011（ToolCall 回执身份与工具结果回注）、BR-AG-DATA-012（修改建议结果投影）、BR-AG-STATE-005（额度收尾与终态结算），对应 turn_tool_loop 三个独立生产块；不改变执行、授权、确认与保存语义。 |
| 2026-10-28 | v2.62 | 模型可恢复错误按控制封套和参数独立有限计数，仍受总调用预算；只读续页改用绑定路径/版本的单游标。 |
| 2026-10-26 | v2.61 | 零字节 TXT 的显式创建例外：仅单文件 createEmpty=true 且省略初始内容，沿原资源计划/确认/重验链创建，不产生初始内容 Diff；其余新文档仍需冻结非空内容。 |
| 2026-10-23 | v2.60 | 统一回合冻结执行额度12/16/2及2次收尾预留；动作交接前置纠错与显式阶段结果，保留授权和未知效果边界。 |
| 2026-10-07 | v2.59 | 通用写能力接拥有生命周期的执行端口；30 秒等待期限、非阻塞取消、迟到效果独立核对及防重放；同步专业操作不承诺强杀/回滚，不改变编辑器或 DOCX 路线。 |
| 2026-10-06 | v2.58 | 经人类确认同步 ENT/SYS：明确授权拒绝与认证/门禁调用失败分离，均禁止后续模型调用，保留执行证据且新请求重新裁决。 |
| 2026-09-15 | v2.57 | P00：确认 Binder 独立验证运行边界；定义 v2 资源身份先行、按需专业快照及 revision，保留 v1 兼容、LogicalState 真值、授权/裁决/保存边界；仅设计，尚未证明生产实现。 |
| 2026-09-04 | v2.56 | AG-ADAPTIVE-RUNTIME-DOC-NAME-01：仅将 §7.2.1 的 TurnEvent 持久化表名、基线列和删除语义对齐至共享数据结构唯一真源 `agent_turn_events`；不新增需求、规则、功能或逻辑。 |
| 2026-09-03 | v2.55 | AG-ADAPTIVE-RUNTIME-DESIGN-01：IntentResult 改为模型驱动可修订语义投影并移除授权；CapabilityGrant 改为逐工具按硬事实计算；TaskRun/TaskPlan/TaskStep 投影化；注册 TurnEvent/EvidenceLedger；ProjectReport 改由真实证据派生并移除 report_project 完成门。 |
| 2026-09-01 | v2.54 | AG-MAPPING-REGISTRY-CLOSURE-01：同步四条 DESIGN_PENDING 需求的 deferred 状态；REQ-AG-009 补充映射到 Agent 1.0 共享运行时对象专门约束 X-SHARED-002，不新增业务规则。 |
| 2026-08-29 | v2.52 | AG-CONFORMANCE-CODE-S4A：注册 TERM-AG-020 AgentContextFacts / TERM-AG-021 ProjectReport、BR-AG-DATA-008 / BR-AG-OBS-001 与 REQ-AG-009 完整映射；冻结版本化编辑器事实、独立 Prompt 层、只读读取证据账本及 report_project 失败关闭契约。 |
| 2026-08-31 | v2.53 | AG-DOC-CONSOLIDATION-01：收敛 Agent 1.0 设计边界；Build/CBT 只作 P2 未来 Profile，禁止自动记忆/知识库/模板写入、检索和注入；按四台状态机建立当前映射，REQ-AG-010/012/014/016 明确保持 DESIGN_PENDING，不伪造设计闭环。 |
| 2026-06-11 | v1.0 | 初版（AG G2 首件，P3 G2 起步）：AG 规则 SSOT——代理回合生命周期与产出唯一出口、门禁前置承接（无绕过路径）、回合历史本地持久化原则、工具调用循环（注册/权限/上限/写盘禁令）、上下文组装（来源限定与裁剪优先级）、内联辅助与自动补全（节流/门禁/装饰层/采纳即编辑）；注册链路 AG-TURN/AG-TOOL/AG-ASSIST（均 P1）、规则 6 条、约束 X-FORBID-006、术语 TERM-AG-001~004（AgentTurn/ToolCall/InlineAssist/Autocomplete）；REQ_MAP（REQ-AG-001~005）；债务（工具清单/历史表列/节流参数/CBT 接管延后记明） |
| 2026-06-12 | v1.1 | Q5 澄清项留痕（人类指示后续细化）：§10 债务新增「单文档回合的跨文档产出边界待裁定」（倾向限定当前文档、跨文档升级 CBT，G3 前与 DE 联合裁定） |
| 2026-06-24 | v1.2 | 接入 L3 细化 A-AG-M-T-02（提示词架构迭代版）并做职责去重：§5.1 增「L3 细化与版本优先」声明；上下文注入层表格行去除旧裁剪排序「显式 @mention > 记忆/知识 > 当前文档」（将当前文档排最低，与当前轮次原则冲突），改委派 M-T-02 BR-AG-DATA-005（当前文档 P0 永不截）；BR-AG-DATA-002 移除内嵌裁剪优先级排序、收敛为只锁组装骨架（分层/顺序/三类回合），截断序统一归 BR-AG-DATA-005，消除「功能重合·术语不同」的逻辑重复；冲突一律以 M-T-02 为准 |
| 2026-07-03 | v1.3 | 批B 样式操作语义承接（A-ED-X-L-02 PI-ED-11 文档先行，人类裁定 D-3，ED 窗口执笔·追加型最小修订）：BR-AG-STATE-003 规则文本增补——用户显式触发的内联样式操作（StyleOp）视为用户编辑直改；回合产出的样式修改一律经 DE diff 裁决（X-FORBID-006/BR-ED-DATA-005），不得借内联语义绕过裁决 |
| 2026-07-04 | v2.0 | 术语沿革澄清与双模式定义回写（人类三轮陈述+裁决）：①TERM-AG-003 zh 内联辅助→**局部修改**（选区/光标两态、弹窗多轮、不持久化）、TERM-AG-004 zh 自动补全→**辅助续写**（快捷键手动触发、单候选 50–100 字），en 均不变，旧 zh 入 forbidden；②新增 TERM-AG-006 编辑模式 EditMode / 007 构建模式 BuildMode / 008 代理编辑 AgentEdit（AI 编辑三层第三层，「对话编辑」退役为禁用别名）；③BR-AG-DATA-003 重写：废止单文档铁律，默认当前文档+显式指向可跨/新建，升级判据换轴「项目级构建意图」（同窗口实现、两模式互相复用能力）；④BR-AG-STATE-003：手动触发+防抖/防重、局部修改多轮、辅助对话不持久化；⑤BR-AG-STATE-002/X-FORBID-006：范围限定文档内容写入，文件树操作（移/重命名/建夹）直接执行、删除须确认；⑥模块正式名「代理（Agent）」 |
| 2026-07-08 | v2.1 | 引用令牌消费侧扩展：BR-AG-DATA-001 明确 AG 上下文组装接收 REF ReferencePayload，覆盖 @mention、拖拽/粘贴与显式资产快照；AST/KB/TPL 仍负责生成/筛选/管理，AG 只消费显式引用 |
| 2026-07-09 | v2.2 | 明确代理回合产出的 DiffCard 必须携带 sessionId/turnId/assistant 消息锚点，渲染在对应 AgentTurn 信息流内，不再作为会话底部全局沉底列表 |
| 2026-07-16 | v2.3 | 细化 AG-ASSIST：辅助续写改为 3 候选，latest-wins 覆盖旧请求；局部修改光标态为插入，多轮只传上一轮候选结果与新指令；候选呈现归 ED 装饰层/overlay 语义，结果返回与采纳前校验文档/block/字符锚，每次真实模型调用前均经服务端或网关 gate |
| 2026-07-16 | v2.4 | 补充 AG-ASSIST 前端触发与定位约束：快捷键需在编辑器宿主/窗口 capture 阶段兜底拦截并读取当前 ED 上下文；候选 overlay 锚点按编辑纸张可视区钳制，避免快捷键被外层事件路径吞掉或弹窗错位 |
| 2026-07-16 | v2.5 | 修正局部修改弹窗激活态快捷键语义：再次触发 `Cmd/Ctrl+K` 为放弃本次修改并关闭弹窗，不重开目标、不发起新请求 |
| 2026-07-18 | v2.17 | 加固已打开文档读取门禁：AG 回合与只读工具统一使用 AgentReadContext；OpenDocSnapshot LogicalState 覆盖 active/open session，未打开文档才读取 DiskState；ReferencePayload.selection 发送时校验 |
| 2026-07-16 | v2.6 | 补齐 Agent 文件整理结构工具契约：`move_doc`/`create_folder` 作为 StructureWrite 工具经 WS 边界直接执行并回注；内容修改继续唯一走 `propose_edit`→DE；删除类工具等待用户确认链路，不注册给模型 |
| 2026-07-16 | v2.7 | 续修 AG-ASSIST 技术约束：默认 450ms 防抖、严格 3 候选、触发锚前后文指纹校验、overlay 键盘焦点/输入法保护，以及前端终态回写 SM-AG-03 |
| 2026-07-16 | v2.8 | 明确 Assist 提示词分离：辅助续写与局部修改必须按 assist_kind 使用不同 system prompt，Agent 编辑继续使用 `SYSTEM_PROMPT_CHAT` |
| 2026-07-16 | v2.9 | 修正 AG-ASSIST 视觉定位策略：区分逻辑锚点与视觉锚点，辅助续写/局部修改分别按光标或选区矩形相邻放置并支持上下翻转，避免浮层被纯钳制到编辑器边缘 |
| 2026-07-16 | v2.10 | 修正 AG-ASSIST 右侧边界：视觉边界叠加 editor-host/center 可见右边界，辅助续写靠右时提前切换 end 对齐，避免被右侧代理编辑面板遮挡 |
| 2026-07-16 | v2.11 | 补充 AG-ASSIST 交互锚生命周期：浮层打开后点击外部、移动光标或改变选区即视为用户放弃，关闭浮层并丢弃已返回/后续返回内容；用户放弃回写 dismiss，结构定位失效回写 anchor_lost |
| 2026-07-16 | v2.12 | 修正 AG-ASSIST 缩放定位模型：视觉锚跟随缩放后的光标/选区，浮层布局边界改为未缩放的 editor-paper/editor-host 可视区，滚动/缩放/resize 后重算位置 |
| 2026-07-17 | v2.13 | 闭合 BR-AG-PERSIST-001 具体模型：新增 TERM-AG-009 代理编辑会话；锁定 completed/error/cancelled/gate_rejected/draft 全状态保留、激活会话 eager-load 其余懒加载、工具摘要只读、Diff 引用不存快照、app data workspace.db 存储与 200MB 软归档 |
| 2026-07-18 | v2.16 | 补齐用户删除历史会话约束：历史列表每个会话行提供删除按钮，`ag.delete_session(session_id)` 仅删除当前 workspace 指定会话的 AG 会话/回合/payload 持久化记录，不删除 DE pending diff 本体；代理回合进行中禁用删除以避免异步保存回写 |
| 2026-07-16 | v2.13 | 修正 AG-ASSIST 稳定定位模型：撤销运行时 DOM 测量反向驱动状态，改为静态尺寸估算 + 滚动/缩放/resize 简单重定位，优先保证浮层稳定显示 |
| 2026-07-16 | v2.14 | 修正 AG-ASSIST 辅助续写失败态：候选不足 3 条不展示半候选、不允许采纳，但保留轻量空态，避免生成失败被误认为快捷键无效 |
| 2026-07-17 | v2.15 | 收敛 AG-ASSIST 唯一实现映射链：Editor、assistStore、agService、后端 request_assist、overlay 均须承接规则；failure_reason 不得在前端链路被吞掉 |
| 2026-07-28 | v2.18 | BDocx TableCellShading：工具循环补充 `list_tables→table_ops` 受控链，锁定 `set_cell_background` 的基准、能力、坐标、合并锚校验及只生成 Diff 的边界。 |
| 2026-07-28 | v2.19 | 接入 A-SYS-I-P-04 BDocxCapabilityManifest：提示词、工具 Schema 与 PendingDiff 前校验使用同一版本/hash，明确静态能力与当前表格动态能力分层。 |
| 2026-07-29 | v2.20 | BDOCX-IMAGE-AGENT-EDIT-01：工具循环新增 `list_images→image_ops`，锁定活动会话、Block/图片实例、workspace 来源哈希、只生成 Diff 与项目源文件保护。 |
| 2026-07-29 | v2.21 | BDOCX-IMAGE-ADVANCED-AGENT-OPS-01：list_images 增加稳定图片身份和动态操作边界，image_ops 开放受控缩放、上下移动和独立段落对齐。 |
| 2026-07-30 | v2.22 | 图片代理编辑闭合审计：同步同图 resize/layout/move 原子组合与 delete 独占，补齐 import_image_asset、edit_blocked、非视觉识别和源文件不搬移边界。 |
| 2026-07-30 | v2.23 | AG-RICH-BLOCK-STRUCTURED-EDIT-01：富文档普通内容写入改为活动 ProseMirror 层级快照 → block_ops → 后端批校验/补齐 beforeNode → ED 单事务；container/atom 只读，HTML 字符串替换与整篇替换在富文档生成前拒绝。 |
| 2026-08-06 | v2.24 | MODEL-CONFIG-01：新增 §6.1 模型注册表与能力偏好（AG-MODEL-CATALOG 链路、BR-AG-DATA-006 来源分层、BR-AG-DATA-007 能力偏好解析）；TERM-AG-010 预设模型；REQ-MAP REQ-AG-006 映射；客户端仅保存标识符不保存密钥（X-FORBID-001）不变 |
| 2026-08-07 | v2.25 | MODEL-CONFIG-TERM-01：TERM-AG-007 中文称呼「构建模式」更名「代理构建」（en BuildMode / term_id 不变），旧称退役为禁用别名；正文 BuildMode 指代同步 |
| 2026-08-07 | v2.26 | AG-MSG-RENDER-01：新增 §3.2 消息流渲染与时序（BR-AG-UI-001，AG-TURN 展示子域）与 TERM-AG-011 消息片段（MessageSegment）；BR-AG-STATE-001 增补 DiffCard/ToolCallCard 按事件时序内嵌语义；BR-AG-PERSIST-001 增补 `assistantMessage.segments[]` additive 结构（旧记录降级沉底）；REQ_MAP REQ-AG-001→BR-AG-UI-001 |
| 2026-08-08 | v2.27 | 定义 agent-edit-1.0 Runtime Profile；新增 TERM-AG-012 IntentResult，并将逐回合工具允许集/注册不等于授权收敛进 BR-AG-STATE-002、Prompt 可信边界收敛进 BR-AG-DATA-002；加固 BR-AG-STATE-001 唯一终态/真实取消、BR-AG-VERIFY-001 真实服务端 GateOutcome和新建文档空基准提案；1.0 关闭 AST/KB/TPL 自动供给与自动记忆写回 |
| 2026-08-09 | v2.28 | AG-PROMPT-STRUCTURE-FIX-01：§5.1 登记实现装配以 LayerKind 枚举为唯一真源（固定层序 System→RuntimeControl→IntentHint→CurrentDocument→Reference→History→CrossDocHistory→User），RuntimeControl 独立结构化消息、历史聚合单块、版本 hash 派生、预算统一口径、审计读 layer；不改变 BR-AG-DATA-002 骨架语义 |
| 2026-08-11 | v2.34 | AG-TASK-01：§5.1 登记多提案批次衔接 + block_ops 删除——delete_node（content 原子块/reverse 恢复）、单回合 ≤5 批、batch 元数据落库与接受衔接、DiffCard 批次徽标/删除渲染、单批上限 24→32；语义详见 A-AG-M-T-02 §4 |
| 2026-08-14 | v2.37 | AG-1.0-DOC-CLOSURE-DCLOSURE-01：§5.2（BR-AG-STATE-004）补 1.0 关闭处置登记——memory_link.rs / ast capture 自动写入实体保留但未接回合链（仅测试引用），与 A-AST-M-T-01 §4.1 关闭语义一致，后续 profile 启用时须按本规则异步接线。 |
| 2026-08-15 | v2.38 | AG-1.0-DEBT-CLEANUP-01：UNC-22 移除旧 dispatch `ag.start_turn` 桩（注册/分发/route 三处；前端唯一入口独立 async `ag_start_turn` + Channel 不变）；UNC-13 确认 workspace-only 旧路径 persist_turn/recover_turns（turn.rs）**无生产调用方**（仅自身测试），标记退役保留代码（新路径 ag.history list_turns/save_turn + AgentSession 为唯一恢复路径）。 |
| 2026-08-15 | v2.39 | AG-1.0-TASKRUN-REWORK-REQ-01：登记 TERM-AG-017（DecisionRequest 任务级裁决请求）/ TERM-AG-018（TaskPlan 任务计划）；三层分离、确认 gate、片段执行现由本文 §7.2 与 A-AG-M-S-01 §2.2 持有。 |
| 2026-08-15 | v2.40 | AG-1.0-TASKRUN-REWORK-DESIGN-01：新增 §7.2 TaskRun 编排重构契约——数据契约（迁移 v12：task_plans / task_plan_steps / decision_requests；task_runs 语义修正）、事件契约（step_progress / decision_request_proposed / decision_request_resolved；diff_proposed 不驱动 TaskRun）、UI 契约（移除顶部卡、底部当前动作状态条、DecisionRequest 卡）。 |
| 2026-08-15 | v2.41 | AG-1.0-TASKRUN-REWORK-IMP-RUNTIME-01：实现 §7.2 运行时/数据层——迁移 v12 三表（task_plans/task_plan_steps/decision_requests）；types 增 DecisionRequestV1/TaskPlanV1/TaskPlanStepV1；task.rs 增 propose_decision/resolve_decision/has_pending_decision/pending_decision/latest_pending_decision_for_session/create_plan/list_plans_by_run；begin_turn 不绕回 waiting_confirm；settle_turn 按未决 DecisionRequest 判定 WaitingConfirm（去除 has_pending_proposal）；lib.rs 回合入口 gate（AG_DECISION_PENDING）+ ag.propose_decision/ag.resolve_decision IPC + StepProgress 事件；ipc AgTurnEvent 增 StepProgress/DecisionRequestProposed/DecisionRequestResolved。 |
| 2026-08-15 | v2.42 | AG-1.0-TASKRUN-REWORK-IMP-UI-01：实现 §7.2.3 UI——移除顶部 TaskRun 卡（TaskRunCard 删除）；信息流底部实时当前动作状态条（step_progress→agStore.currentAction）+ DecisionRequest 卡（裁决 confirm/cancel→ag.resolve_decision）；回合终态清 currentAction、待裁决跨回合保留（三层分离）；agRuntime +2。 |
| 2026-08-15 | v2.43 | AG-1.0-TASKRUN-REWORK-PROMPT-01：prompts.rs 文案修正——DiffCard 只确认修改内容、任务级确认走 DecisionRequest（删"接受/拒绝即人类确认"总括）；同一执行片段内连续多张 Proposal（去跨轮分批诱导）；同文档 Diff2 基于前卡成功预览后 LogicalState、前卡失效不形成补丁栈。 |
| 2026-08-15 | v2.44 | AG-1.0-TASKRUN-REWORK-CONTINUE-01：auto-continue（确认后自动续片段）——resolveDecisionAndContinue（confirm→自动 startTurn 续片段、MAX=3 防死循环、手动发送重置计数；cancel 终止）；DecisionRequest 卡"确认并继续"；TaskPlan 呈现——ag.list_plans/ag.create_plan IPC + TaskPlanView/AgSession.taskPlans + PlanCard（步骤/依赖/游标/确认点/授权）。 |
| 2026-08-15 | v2.45 | AG-1.0-TASKRUN-REWORK-INTENT-01：IntentResult 复杂目标 → 自动 TaskPlan——is_complex_target 判定（写意图 + 多文档 或 单文档整篇信号）；回合内 intent_result 后接线（build_task_plan：每文档读取→生成建议/确认点，read→propose 依赖；同 run 已有计划不重复建）；前端 listPlansByRun snake→camel 映射 + refreshSessionPlans（done/error 后刷新）。 |
| 2026-08-15 | v2.46 | AG-1.0-DECISION-CARD-01：任务裁决卡重构——独立固定裁决区（DecisionBar，Composer 上方，任务暂停等决策；类型化选项：permission 允许本次/拒绝、plan_choice 等确认并继续/取消）；输入门控（pendingDecision 时手动发送拦截 + startTurn 前端 gate，auto-continue 除外）；纯确认/拒绝短词 intent 降权 Answer（不落入 general 档 Edit 强制工具）；提示词强化删除走 delete_doc/delete_folder（禁止 replace_document 清空代替删除）。 |
| 2026-08-21 | v2.47 | 工作区资源操作基线重构：BR-AG-STATE-002、X-FORBID-006 与 TaskRun 契约改为 WorkspaceOperationPlan + DecisionRequest 前置裁决、确认后 WS 直接执行；资源操作不生成 DiffCard；CreateDocumentBundle 在资源创建后衔接普通内容 Diff；旧 AG_DELETE 链降为迁移债务。 |
| 2026-08-22 | v2.48 | WS-RESOURCE-WRITE-GUARD-01：移除「新文档以空基准 ChangeProposal 创建」旧表述——新建文档唯一入口为 plan_workspace_operation（CreateDocumentBundle 确认后建盘并投影初始内容 Diff）；propose_edit 目标必须已存在（内容 Diff 不得创建资源，X-FORBID-006，工具层路径级拒绝）。 |
| 2026-08-27 | v2.49 | AG-CONFORMANCE-GOVERNANCE-S1：收敛 AgentTurn 澄清为原回合 AwaitingClarification→Assembling，与 DecisionRequest、重试和 Assist 分界；明确新建文档唯一 CreateDocumentBundle 路径、简单目标不建 TaskRun/TaskPlan；资源意图可获得任务必要只读工具 + plan_workspace_operation，副作用路径不变；冲突旧实现登记移出 active 规范正文。 |
| 2026-08-28 | v2.50 | AG-CONFORMANCE-GOVERNANCE-S3D：注册 TERM-AG-019 显式编辑目标集；按 HD-S3D-01 锁定初始来源、同 AgentTurn 澄清追加、工具发现目标经 scope DecisionRequest 确认追加、TaskRun continuation 继承、CreateDocumentBundle 建盘后追加及集合外硬拒；read/search 与 cross_doc 均不是授权来源。 |
| 2026-08-28 | v2.51 | AG-CONFORMANCE-DESIGN-S3D-COMPANION：承接 HD-S3D-03 与需求提交 fed343a，修正 Runtime Profile 的显式引用只读旧概括；区分引用内容 data-only、用户引用身份目标授权元数据与写工具授权。 |
| 2026-08-14 | v2.36 | AG-1.0-D3-AG-REF-AST-RUNTIME-CONTRACT-01：注册 TERM-AG-013~016（RuntimeProfile/CapabilityGrant/TaskRun/TaskStep）；新增 §7.1 Agent 1.0 运行时契约落地（REQ-AG-007/015）——RuntimeProfile 入口校验、CapabilityGrant 五因素收窄、IntentResult 收敛、TaskRun 接力、StructuredVisibleEvent 不含内部推理、授权非来源声明。 |
| 2026-08-13 | v2.35 | AG-STRATEGY-05：BDocx 整篇改写收敛为无表格且无图片的动态条件并由服务端拒绝受保护/新引入结构；block_ops 同锚点顺序、后置校验与稳定 id reverse 闭合；proposal、编辑器预览、接受和保存的真实状态与最小运行事件对齐。 |
| 2026-08-11 | v2.33 | AG-CTX-01：§5.1 登记上下文管理重构——list_blocks 轻量投影、read_doc blocks 范围参数、移除工具结果截断、AG_EDIT_TOOL_REQUIRED 文案区分、查询动词→Search、整篇对齐检测、序列化瘦身；语义详见 A-AG-M-T-02 §4 |
| 2026-08-11 | v2.32 | AG-DOCX-RO-01：§5.1 登记原生 docx 只读——read_editable_snapshot 放行原生 docx 走 pandoc 转换链，source=doc_convert_readonly、editable_kind 覆盖 html（≠bdocx），结构性只读；propose_edit 仍拒绝、编辑器路径保持 DOC_NOT_BDOCX；语义详见 A-AG-M-T-02 §4 |
| 2026-08-11 | v2.31 | AG-R2-01：§5.1 登记整理复原综合修复——预算（工具结果历史压缩 + block_ops 单批上限 24）、意图（纠错语义词 + 疑问文档指涉→只读）、提示词（编辑当轮提案 + 顺序自查）、范围（workspace 内只读放宽 + 锚点矛盾校验）；语义详见 A-AG-M-T-02 §4 |
| 2026-08-10 | v2.30 | AG-INTENT-V3-01：§5.1 登记乐观授权——工具授予三分类固定（只读组无条件 + 提案组不落盘），意图识别退化为上下文注入器，识别失败不再零工具瘫痪；resolve_capability_grant 同步固定工具集，BR-AG-STATE-002 allowed_tools 逐字满足；语义详见 A-AG-M-T-02 §4 |
| 2026-08-09 | v2.29 | AG-INTENT-V2-01：§5.1 登记 resolve_intent_result（BR-AG-DATA-004）内部升级 Parsing→Adjudication 语义化判定；修复「改为红色」「创建md文档」误判 Answer；IntentResultV1 契约不变，IntentSignal 私有，语义详见 A-AG-M-T-02 §4 |
