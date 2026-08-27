---
文档编号：AG-M-T-01
文档状态：A
负责模块：AG
文档职责：AG 代理模块级 L2 技术设计（规则 SSOT）——代理回合、工具调用、上下文/提示词、会话与辅助能力，并定义 agent-edit-1.0 Runtime Profile 的可靠执行边界
上游约束：A-AG-M-D-01_代理功能需求描述.md（REQ-AG-001~006、AG 1.0 EP 能力基线）、A-CORE-C-P-01_APC.md（§4.1/§4.3 云薄端、§6、§13）、A-SYS-I-P-03_云接口契约.md（SM-SYS-01、X-SEQ-001、CLOUD-GATE/INVOKE、TERM-CLOUD-001~004）、A-SYS-I-P-02_IPC协议.md（BR-SYS-STATE-002 流式终态、统一错误形状）、A-SYS-I-P-01_数据结构定义.md（X-SHARED-001、BR-SYS-PERSIST-001）、A-WS-M-T-01_工作台技术设计.md（X-FORBID-004）、A-ED-M-T-01_编辑器技术设计.md（ED-DECOR、X-FORBID-005）、A-DE-M-D-01_DiffEdit功能需求描述.md（REQ-DE-001 产出消费）、ADU.md（§3）
直接承接：本文 RULE/CHAIN/CONSTRAINT/TERM 为 AG 规则 SSOT，代码经 @GOV 映射；SM-AG-01/02/03 在 A-AG-M-S-01 定义并登记总图；AG 产出契约供 DE G2 承接；注入消费契约对接 REF
使用边界：本文是 AG 规则 SSOT；不重定义门禁/网关契约（归 A-SYS-I-P-03，本文承接引用）、不定义 diff 生成与终态（归 DE）、不定义 @mention 解析（归 REF）、不定义装饰层机制（归 ED）、不锁定模型/提示词/工具具体清单（延实现期与运营配置）、状态机内部归 A-AG-M-S-01；项目级构建编排归 CBT（构建移交/回落契约见 BR-CBT-STATE-002，双侧已闭合）
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
代理编辑（TERM-AG-008）面板双模式之一（2026-07-04 人类裁决）：默认面向当前文档的 AI 精确修改
（编辑器感知 + 块级锚定，内容产出一律经 DE diff 裁决、人最后裁定），兼具文件创建、工作区资源整理、
项目感知、对话与资源获取。资源操作统一生成 WorkspaceOperationPlan，并以 DecisionRequest 前置裁决；
确认后由 WS 重新核验并直接修改 DiskState，不生成资源 DiffCard。模式是默认
姿态非能力墙——与代理构建互相复用彼此能力。注：编辑器/文档层的「编辑态 vs 预览态」（docx 打开
形态）与本模式无关，不得混用。

<!-- TERM
term_id: TERM-AG-007
chains: AG-TURN
zh: 代理构建
en: BuildMode
forbidden: 构建模式, agent模式, agent 模式, Chat Build, chat-build
-->
代理编辑（TERM-AG-008）面板双模式之一（2026-07-04 人类裁决；2026-08-07 中文称呼更名
「代理构建」，旧称「构建模式」退役为禁用别名，en/term_id 不变）：对话阶段（多轮需求搜集→项目蓝图
BuildBlueprint，属本模式下的普通代理回合）→ 显式触发构建（明确指令或构建按钮）→ 计划确认 →
执行构建（期间不可发起对话，仅中断）→ 完成后回落对话继续编辑调整。构建执行编排归 CBT（SM-CBT-01）。

<!-- TERM
term_id: TERM-AG-008
chains: AG-TURN
zh: 代理编辑
en: AgentEdit
forbidden: 对话编辑, 对话编辑板块
-->
AI 编辑三层之第三层（面板级，2026-07-04 人类裁决定名）：agent 面板承载的完整 AI 能力域
（编辑模式 EditMode + 代理构建 BuildMode），会话持久化、产出经 DE diff 裁决、工具调用、项目
构建。沿革：旧版曾以「对话编辑」总称整个 agent 能力，该词后又错位为 DE 模块中文名——三重身份
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
代理回合进入上下文选择和工具开放前形成的结构化当前轮决策：至少包含 `intent_type`、目标
`target`、是否产生内容变化 `mutation`、歧义 `ambiguity`、必需上下文 `required_context` 与本轮允许
工具 `allowed_tools`。它是 Context、Prompt 与工具权限的共同输入，不替代用户原始输入，也不保存
模型隐藏推理（BR-AG-DATA-004 / BR-AG-STATE-002）。

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
本轮允许的工具/操作集合，逐层收窄（RuntimeProfile ∩ 任务类型 ∩ 目标范围 ∩ 副作用等级 ∩ 格式/动态能力）。
注册表存在 ≠ 本轮可调用；grant 只在当前回合有效，跨回合不隐式继承（A-SYS-I-P-01 §3.1，BR-SYS-STATE-007）。

<!-- TERM
term_id: TERM-AG-015
chains: AG-TURN
zh: 受限任务实例
en: TaskRun
forbidden: 任务实例, 后台任务, 任务队列, 子任务集合, task 实例, 复合任务对象
-->
同一用户目标内组织多个受限步骤/AgentTurn 的编排实例，不是后台队列：不自动重试、不跨重启续跑、不越过
确认点；“工作中”只能由真实回合、步骤或工具事件驱动（A-SYS-I-P-01 §3.1，BR-SYS-STATE-008）。

<!-- TERM
term_id: TERM-AG-016
chains: AG-TURN
zh: 任务步骤
en: TaskStep
forbidden: 任务步骤, 步骤, 子步骤, step 单元, 执行步
-->
TaskRun 内有依赖的受限步骤（读取/搜索/分析/生成/提案/等待确认等）；每步重新取得当前最小授权，进入
W1/W2/W3 的步骤不得在不可见后台继续（A-SYS-I-P-01 §3.1，BR-SYS-STATE-008）。

<!-- TERM
term_id: TERM-AG-017
chains: AG-TURN
zh: 任务级裁决请求
en: DecisionRequest
forbidden: 确认卡, 裁决项, decision 实例, 确认请求
-->
任务级通用裁决项（类型：权限 permission / 方案 plan_choice / 范围 scope / 人工确认 manual_confirm），与内容
裁决分离：生命周期 proposed → resolved（confirm / cancel），confirm/cancel 驱动 TaskRun 的 UserConfirm /
UserCancel 事件；内容层 DiffCard 的接受/拒绝不驱动 TaskRun（A-AG-M-D-01 §5b，REQ-AG-015）。

<!-- TERM
term_id: TERM-AG-018
chains: AG-TURN
zh: 任务计划
en: TaskPlan
forbidden: 计划, 任务计划清单, plan 单元, 执行计划
-->
IntentResult 判定复杂目标后创建的可展示操作计划（非模型思维链）：保存步骤列表、步骤依赖、当前游标、所需
授权、确认点标记与阶段结果；简单目标单 AgentTurn 直行不建 TaskPlan；执行按片段推进，每片段重新授权，
任务级确认后自动开始下一片段且不后台续跑（A-AG-M-D-01 §5b，REQ-AG-015）。

## §1 文档定位

```
REQ-CORE-001/002/006/008/009 → REQ-AG-001~005（AG L1）→ 本文（SSOT）→ 代码(@GOV)
本文 ←承接← A-SYS-I-P-03（门禁/网关契约，不重定义）、A-ED-M-T-01（ED-DECOR）、REF（注入载荷）
本文 →驱动→ A-AG-M-S-01（SM-AG-01/02/03）→登记→ A-SYS-M-S-01 总图（四条 planned 行转 active）
本文产出契约 →供→ DE G2（AI 产出→diff 生成）承接
```

**云薄端前提重申（APC §4.3/§13）**：AG 不持密钥、不内置 provider SDK、不绕过订阅门禁；
本文一切规则在 SYS I-P-03 契约（CLOUD-AUTH/GATE/INVOKE、SM-SYS-01）之上承接，不重定义云协议。

## §2 技术分层与边界

```
AG 代理（本文 SSOT）——代理编辑（TERM-AG-008）面板：编辑模式 + 代理构建（构建编排归 CBT）
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
| 目标 | 当前活动文档/选区可读写；显式引用文档默认只读；新建文档经 `plan_workspace_operation`（CreateDocumentBundle）前置裁决，确认后建盘并投影初始内容 Diff |
| 工作策略 | `direct_response / read_only / edit_proposal / create_proposal / clarify` 五类确定性路由；不生成通用多步骤 Plan |
| 工具 | 按 IntentResult 生成本轮最小允许集；文件移动/建目录、表格/图片高级操作仅在专项 G5 通过后启用 |
| 上下文 | 当前 LogicalState + 当前输入 + 显式引用 + 同文档必要历史；AST/KB/TPL 自动供给关闭 |
| Prompt | 统一 Prompt Runtime 装配；系统约束与 data-only 文档/引用分离；携 runtime version |
| 产出 | 回答或 ChangeProposal；任何内容创建/修改均经 DE Diff 裁决 |
| 会话 | workspace 隔离、本地恢复、单会话同一时刻一个活动回合 |
| 延期 | BuildMode/CBT、自动记忆/知识、模板工作流、参考风格迁移、复杂规划、多 Agent |

前端可继续保存完整路线所需的 additive 字段，但 1.0 不得展示没有独立 Runtime Profile 的代理构建
入口；延期能力未通过专项系统验证时必须 feature-gate，而不是依靠提示词声明“不要使用”。

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
必须来自真实状态转移，不得在副作用完成后补发装饰性状态。含修改建议的产出唯一出口是交 DE 生成
pending_diff（AG 无直接写盘路径，X-FORBID-006）；交 DE 生成待裁决修改时必须保留会话、回合和
本轮 assistant 消息锚点，使 DiffCard 回到对应 AgentTurn 信息流内呈现，而不是作为会话底部全局列表
沉底汇总；**回合内呈现时序（2026-08-07 细化，承接 BR-AG-UI-001）**：DiffCard 与 ToolCallCard 按
事件到达顺序内嵌于 assistant 消息流（消息片段 TERM-AG-011），不整体沉到正文底部——模型正文、
工具调用、修改建议按时间序交错呈现；回合失败回统一错误形状且可重试。状态转移归 SM-AG-01（A-AG-M-S-01）。

<!-- RULE
rule_id: BR-AG-DATA-003
主链路: AG-TURN
域: DATA
-->
代理回合修改建议产出（ChangeProposal）的目标边界（2026-07-04 人类裁决，**取代原「单文档铁律」**）：
target 一律限 active workspace 内（X-FORBID-004）；**默认目标为当前文档**；用户**显式指向**其他
文档（@mention 引用/明示路径）或明确要求创建新文档时，可产出指向该文档的 ChangeProposal
（新建文档必须经 `plan_workspace_operation` 前置裁决，确认后创建真实资源并投影初始内容 Diff，
承 A-WS-M-T-02 §6；内容 Diff 不得创建资源，X-FORBID-006——WS-RESOURCE-WRITE-GUARD-01）；一切内容产出仍经 DE/ED 受控转正，不得由 AG 直写
LogicalState 或 DiskState（X-FORBID-006 / X-FORBID-009）。已打开目标必须以 OpenDocSnapshot
中的 LogicalState 为 ChangeProposal 基线；未打开目标才允许读取 DiskState。识别到
**项目级整体构建意图**（从蓝图构建完整项目——多文件修改本身不再是升级判据）时提议触发构建
（判据归 BR-CBT-STATE-002），构建在**原对话窗口**内实现、不强制切换模式；编辑/构建两模式互相
复用彼此能力。**DE 侧协同复核**同步调整：`de.generate_diff` 校验 target 在 workspace 内且属回合
声明的目标集（当前文档 + 显式指向/新建集），越出者拒绝。
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
不得驱动 AwaitingGate→Streaming；无有效订阅或断网时：代理/工具回合明确拒绝（统一错误形状），局部修改/辅助续写静默
不提供且不打断编辑。本规则是 SYS 门禁契约在 AG 的承接执行，不重定义裁决语义（裁决归服务端）。

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
工具调用循环必须满足：模型请求的工具必须同时在注册清单和本轮 IntentResult.allowed_tools 内，且
通过权限、目标与参数边界校验方可执行（未注册/未获本轮允许/越权
一律拒绝并将拒绝原因回注模型）；执行结果（含失败）回注后继续回合；循环有上限，达上限中止；
用户可随时中断，中断为终态；**任何文件内容写入意图不得由工具直接落盘，必须产出修改建议走 DE
diff 路径**（X-FORBID-006）；只读类工具（读文档/搜索/列块）必须经 AgentReadContext 并限定
workspace 内（X-FORBID-004）：已打开文档优先读取 OpenDocSnapshot 的 LogicalState，未打开文档才经
WS/DOC 读取 DiskState；已打开文档快照缺失、版本缺失或锚校验失败时必须拒绝工具结果或阻断回合，
不得用 DiskState 兜底；
**工作区资源工具**（创建文档/文件夹、删除、重命名、移动、恢复）不属内容写入，但也不得从模型工具调用
直接执行：工具只能生成不可变 WorkspaceOperationPlan 与 DecisionRequest；用户确认后由 WS 按
BR-WS-STATE-003 重新核验并执行。模型参数、工具返回或 Prompt 文案均不能伪造用户确认。批量执行按
BR-WS-PERSIST-003 失败即停并逆序补偿，重启后的未完成计划进入 Interrupted 且不自动执行。
用户明确要求新建文档时使用 CreateDocumentBundle：初始内容在确认前生成，确认后先创建目标 DiskState，
再进入普通内容 Diff；不得使用 `create_doc` 或 `propose_edit(delete_*)` 绕过资源计划。状态转移归
SM-AG-02（工具循环）与 SM-WS-04（资源执行）。
本轮允许集必须从 IntentResult 派生：`answer/analyze` 只允许必要只读工具，`edit` 允许目标读取与
`propose_edit/edit_blocked`，`create_document` 只允许 `plan_workspace_operation`（确认后建盘并投影初始内容
Diff），`propose_edit` 目标必须已存在（内容 Diff 不得创建资源，WS-RESOURCE-WRITE-GUARD-01），`clarify` 不开放产生副作用的工具；
结构操作或富文档高级工具只有在意图、目标、格式能力和 Runtime Profile feature gate 同时允许时才可
进入清单。工具注册只表示系统拥有该能力，不等于当前回合获权；执行器必须再次校验本轮允许集，模型
不得通过伪造工具名、参数或提示内容扩权。

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

## §5 上下文组装（AG-TURN）

<!-- RULE
rule_id: BR-AG-DATA-001
主链路: AG-TURN
域: DATA
-->
回合上下文的来源限定为：已打开文档 OpenDocSnapshot（LogicalState）、未打开文档 DiskState 读取结果、
REF 注入载荷（@mention 与 ReferencePayload，含来源标识）、AST/KB/TPL 本地资产快照（经 REF 显式引用进入；其生成/筛选/管理归各 owner）；
一切 workspace 内容读取限定 workspace 内（X-FORBID-004）；超出上下文预算时裁剪策略必须确定性且用户显式选定的注入优先保留，裁剪发生时对调用
可追溯（不静默丢弃显式注入）。预算数值与裁剪算法延实现期标定，本规则锁来源限定与裁剪优先级语义。
已打开文档读取存在硬优先级：`source=active_editor/active_editor_session/open_editor_session` 的 LogicalState > 未打开文档
DiskState；任何已打开文档不得从 DiskState 或 loadedContent 旧快照取上下文主事实。ReferencePayload.selection
必须在发送时按当前 AgentReadContext 校验；无法校验的 selection 只能作为用户原始选区快照呈现，不得作为当前
文档事实。
`runtime_profile=agent-edit-1.0` 下 AST/KB/TPL 不做自动检索或自动注入；只有用户显式引用且来源可校验时
才可作为 data-only 引用进入本轮。该限制是 1.0 发布收敛，不取消后续 profile 按 BR-AG-DATA-005
确定性门控启用补强层的能力。

## §5.1 提示词 / 上下文组装架构（AG-TURN，2026-06-12 补 B-1）

> 解除 A-CORE-X-R-01 B-1：提示词**文本/措辞/模型档**属实现期运营配置（不入规则）；本节锁设计层
> 的**组装骨架**——分层、注入顺序、三类回合的结构差异。
>
> **L3 细化与版本优先（2026-06-24，迭代版）**：组装骨架的完整分层（含引用/记忆/知识/模板等
> binder 特有层）、当前轮次原则与意图沿用判定、补强层污染隔离与**分层截断序**，由 L3 专项
> A-AG-M-T-02_提示词架构设计.md 细化（BR-AG-DATA-004/005）。**职责单一化（去重）**：本节（BR-AG-DATA-002）
> 自此只锁**组装骨架**（分层/注入顺序/三类回合差异），**截断序统一归 M-T-02 BR-AG-DATA-005**，
> 不再在本节内嵌裁剪优先级排序；与 M-T-02 表述冲突时**以 M-T-02 为准**。

**组装分层**（自上而下）：

| 层 | 内容 | 性质 |
|---|---|---|
| 系统提示词层 | 产品角色定位 + 硬约束边界声明（修改须经 diff 裁决、不得越权落盘、workspace 限界） | 设计锁边界，文本实现期 |
| 工具/能力定义层 | 可用工具清单与契约（AG-TOOL 注册项投影） | 清单实现期运营 |
| 上下文注入层 | 来源限定承 BR-AG-DATA-001（显式 @mention 不被静默丢弃）；已打开文档以 OpenDocSnapshot LogicalState 为当前轮主事实，记忆/知识为补强层——完整分层与**截断序**见 M-T-02 §2/§5（BR-AG-DATA-005，**当前文档 P0 永不截**） | 顺序设计锁；截断序细化归 M-T-02 |
| 用户输入层 | 本轮 prompt / 编辑触发上下文 | — |

**实现装配登记（2026-08-09，AG-PROMPT-STRUCTURE-FIX-01）**：生产装配以 `LayerKind` 枚举为唯一真源
（装配顺序、审计层名、版本登记共用），取代早期仅供测试的 `assemble_prompt`/`PromptSkeleton` 四层骨架。
固定层序为 `System → RuntimeControl → IntentHint → CurrentDocument → Reference → History →
CrossDocHistory → User`（CrossDocHistory 仅显式沿用时注入）；`RuntimeControl` 独立为结构化 system
消息（与自然语言提示词分离，P2 修复）；历史聚合为单块 DATA-ONLY 消息并保留 role 序列（P5/P7 修复）；
`system_prompt_version` 由提示词内容 hash 派生（P3 修复）；预算 gate 使用序列化 messages+tools 统一口径
（P4 修复）；审计直接读取 `message.layer` 字段（P6 修复）。本登记不改变 BR-AG-DATA-002 组装骨架语义。

**迁移债务登记（2026-08-21）**：2026-08-11 的 `propose_edit(delete_doc/delete_folder) → AG_DELETE DiffCard → ws.apply_delete_diff`
实现与当前权威基线冲突，必须在独立实现 Scope 中迁移为 `WorkspaceOperationPlan → DecisionRequest → WS execute`。
该旧链只作为现状证据，不再是设计依据；资源删除不得进入 `propose_edit`、DiffCard 或内容 Proposal 生命周期。

**实现登记（2026-08-11，AG-STRATEGY-01 意图边界重构 + 工具全量 + 调用时刻校验）**：意图判定降级为 4 档强信号边界（chat/read-only/create/general）；allowed_tools 按 profile 全量（不再按意图收窄）；propose_edit 取消 targetScope 硬锁、安全下沉调用时刻（workspace 边界 + base 校验 + 新建检测 + DiffCard 确认）；新增 workspace_glob 只读工具；arguments JSON 解析失败回传明确错误。延续词/复合意图/多文档联动结构性解决。语义详见 A-AG-M-T-02 §4。

**实现登记（2026-08-11，AG-INTENT-COMPOSITE-01 复合请求意图优先级）**：`resolve_intent_result` 新增 `create_requested` 前置判定（非否定 ∧ Create 强词 ∧ Document/Workspace 目标 → CreateDocument，先于 `verb==Edit` 分支）；修复「新建…文档，并优化排版」被判 Edit 的复合意图缺陷；detect_verb 首命中与 V2/V3 语义不变；补复合意图回归测试。语义详见 A-AG-M-T-02 §4。

**实现登记（2026-08-11，AG-WS-LAYERED-01 模型分层感知工作区）**：`list_docs` 分层——无参数 → 概览（mode=overview：顶层目录+文件计数+总文件数；豁免 ≤30 文件且 ≤4KB → mode=full 全量）；`path` → 该目录一层直接子项（mode=children）。树枚举只含索引（路径/类型/计数），正文唯一入口 read_doc/search_text；前端 ws.list_docs 与 search_text 内部遍历保持全量语义。prompt 分层引导 + CreateDocument 专项。语义详见 A-AG-M-T-02 §4。

**实现登记（2026-08-11，AG-TASK-01 多提案批次衔接 + block_ops 删除）**：`block_ops` 新增 `delete_node`（仅 content 原子块，afterNode=null 删除标记，容器拒绝不级联；前后端原子应用 + reverse 精确恢复记录位置重建）；放开同回合多提案（≤5 批、同轮并行、每批 base 递增）；批次元数据 `batch={id,index,total}` 随 style_ops `_batch` 键落库，接受路由返回 batch_remaining 衔接提示；DiffCard 批次徽标 + 删除渲染；提示词单批上限 24→32。语义详见 A-AG-M-T-02 §4。

**实现登记（2026-08-13，AG-STRATEGY-05 文档改写安全闭合）**：BDocx capability manifest、工具 schema、运行时声明和服务端 guard 统一为「无表格且无图片才允许整篇 `replace_document`」；表格或图片存在、或新 HTML 引入其中任一结构时，返回 `AG_BDOCX_REPLACE_DOCUMENT_REQUIRES_STRUCTURED_OPS`，不产生 PendingDiff。`block_ops` 修复同锚点插入/替换互相覆盖：replace/delete 先写、插入后写，同坐标多插入按原 proposal 顺序展示；forward→reverse 保证恢复原 LogicalState。`propose_edit` 工具结果只确认 pending proposal，前端以 `queued/applied/failed` 单独投影草稿预览并输出不含正文的 `[de-draft-preview]` 运行事件；接受仅确认、保存才写盘，卡片不得提前宣称完成。

**实现登记（2026-08-11，AG-CTX-01 上下文管理重构）**：上下文控制从"事后截断"改为"源头轻量投影+按需读取"——`list_blocks` 轻量投影（块目录，无 node_json/全文）、`read_doc` 支持 `blocks` 范围参数（只返回目标块完整投影含 node_json，向后兼容）、移除工具结果截断（trim_tool_results_to_budget 删除）；`AG_EDIT_TOOL_REQUIRED` 文案区分「已读未提案」与「零工具调用」。意图层：查询动词补齐（列出/列举/显示/展示/有哪些 → Search 只读组）、整篇对齐检测（`detect_sync_from_reference` → 注入能力边界提示不阻断）。序列化：空 marks/attrs 省略。语义详见 A-AG-M-T-02 §4 实现登记。

<!-- RULE
rule_id: BR-AG-DATA-002
主链路: AG-TURN
域: DATA
-->
回合的提示词与上下文必须按统一组装骨架构建：系统提示词层（声明产品角色与硬约束边界——修改经
diff 裁决、不越权落盘、workspace 限界）→ 工具/能力定义层 → 上下文注入层（来源与注入顺序承
BR-AG-DATA-001；完整分层、当前轮次优先与**截断序**归 A-AG-M-T-02 BR-AG-DATA-004/005，冲突以
M-T-02 为准）→ 用户输入层；三类回合采用结构骨架差异——**编辑回合**（代码名 TurnKind::Chat 沿用；
含对话历史 + 修改建议导向 diff 产出）、**CBT 构建任务**（任务计划上下文 + 单文件产出导向，承
A-CBT-M-T-01）、**辅助回合**（辅助续写/局部修改；轻量、无系统对话历史；辅助续写与局部修改必须
按 assist_kind 使用不同 system prompt，避免 3 候选 JSON 契约与单正文改写契约混用）。提示词文本/措辞/模型档属实现期运营配置不入规则；
本规则同时锁可信边界：只有产品不变量、运行策略和经本轮允许的工具契约可进入 system instruction；
当前文档、显式引用、历史、记忆、知识和工具结果必须作为 data-only PromptDataEnvelope 注入，并保留
来源、freshness、hash、trust 与 token 元数据，不得因数据正文含命令文字而获得指令权限。每次调用形成
带 runtime/version、intent_id、context trace、tool manifest hash 和预算结果的 PromptPackage；调试追踪
默认不保存用户正文、密钥或会话令牌。本规则锁组装分层、注入顺序、可信边界与三类回合骨架差异
（**截断序不在本规则，归 BR-AG-DATA-005，去重单一化**）。

## §5.2 记忆写入触发对接（AG-TURN，2026-06-12 补 B-2）

> 解除 A-CORE-X-R-01 B-2：AST 侧写入已就绪（ast.capture + SM-AST-01），本节补 AG 侧**何时触发**
> 写入的对接契约。

<!-- RULE
rule_id: BR-AG-STATE-004
主链路: AG-TURN
域: STATE
-->
回合产出处置完成后（SM-AG-01 HandingOff/Completed 终态），AG 须**异步触发记忆写入**：将回合中
浮现的可沉淀事实候选交 AST（`ast.capture`，传 origin_turn 来源回合标识），由 AST 自动落库
（A-AST-M-T-01 BR-AST-STATE-001）。记忆写入**异步进行、不阻塞回合终态、失败不影响回合**（记忆为
增益非关键路径，不进 X-FORBID-006 落盘路径）；可沉淀事实的判定机制（模型/规则）延实现期。本规则
锁「回合终态后异步触发、传 origin_turn、不阻塞」的对接语义，写入落库语义归 AST。该 AG→AST 转移
登记 A-SYS-M-S-01 §5（X-INTER-001）。
`runtime_profile=agent-edit-1.0` 不激活自动记忆写回，因此不要求为 1.0 发布主链调用 AST；后续 profile
启用该能力时仍必须完整遵守本规则，不得以同步写入或失败传播方式简化。
> **1.0 关闭处置登记（2026-08-14 · AG-1.0-DOC-CLOSURE-DCLOSURE-01）**：代码实体 `ag/memory_link.rs`
> （on_turn_terminal / try_capture）与 `ast/capture.rs`（SM-AST-01 自动写入机）在 1.0 **保留但未接回合链**
> （全库无生产调用方，仅自身测试引用），与 A-AST-M-T-01 §4.1「BR-AST-STATE-001 自动写入在 1.0 关闭」
> 一致。后续 profile 启用本能力时须接线并按本规则异步触发；在关闭期间不得被误用为同步写入路径。

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

模型配置体系的客户端承载（REQ-AG-006）：模型注册表（来源分层）与能力偏好（四类能力独立配模型）
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
能力模型偏好解析必须满足：四类能力（autocomplete / inline-assist / agent-edit / build）各自维护
一项偏好，值为「跟随全局默认」或「显式指定 modelKey」二者之一；解析结果必须是注册表内存在的
modelKey，注册表条目被删除后引用它的显式偏好回落为「跟随全局默认」，不得产出悬空 modelKey；
解析为**纯同步逻辑**，不引入跨请求状态；解析出的 provider/model 仅作为调用参数传入既有链路，
不改变门禁前置与服务端裁决路径（X-FORBID-002）。

## §7 跨模块约束（CONSTRAINT）

<!-- CONSTRAINT
constraint_id: X-FORBID-006
摘要: AG 及其工具调用不得绕过两条唯一写路径：文档内容修改必须经 AG 产出→DE 内容 Diff 裁决→WS 保存/落盘；工作区资源操作必须经 WorkspaceOperationPlan→DecisionRequest 前置裁决→WS 重验并直接修改 DiskState。资源操作不得生成 DiffCard，内容 Diff 不得创建、删除、重命名或移动资源；CreateDocumentBundle 只在资源创建完成后把初始内容接入普通 Diff
涉及模块: AG, DE, ED, WS, REF, SYS, CBT
链路影响: AG-TURN, AG-TOOL, WS-DIFF-APPLY, WS-OPERATION
-->

### 7.1 Agent 1.0 运行时契约落地（REQ-AG-007/015，2026-08-14 D3 登记）

承接 A-SYS-I-P-01 §3.1 共享对象契约（D1）与 SM-AG-01/04（D3），AG 侧落地口径：

| 对象 | AG 侧落地 | 承接 |
|---|---|---|
| RuntimeProfile | `ag_start_turn` 入口显式校验 `runtime_profile`（1.0 仅 `agent-edit-1.0`）；缺失/未知在 Gate、provider、工具与 Proposal 前失败关闭，不产生可执行事件 | BR-SYS-STATE-007、A-AG-I-P-01 §2.2 |
| IntentResult | IntentResultV1 收敛任务类型、目标、预期副作用、所需上下文、允许工具与确认点；“只分析不修改”不获得 Proposal；写入歧义只澄清或生成目标计划 | BR-SYS-STATE-007、TERM-AG-012 |
| CapabilityGrant | 工具执行前经 SM-AG-02 按五因素收窄（RuntimeProfile ∩ 任务类型 ∩ 目标 ∩ 副作用等级 ∩ 格式/动态能力）校验；注册表存在 ≠ 本轮可调用；grant 仅当前回合有效 | BR-SYS-STATE-007、BR-AG-STATE-002 |
| TaskRun/TaskStep | 复杂目标由 SM-AG-04 组织步骤/回合；每步重新取得最小授权；连续 Proposal 以最新 LogicalState 接力（A-DE-M-S-01 §2.1） | BR-SYS-STATE-008、TERM-AG-015/016 |
| StructuredVisibleEvent | 用户只见意图结论、读取/工具、提案卡、等待确认与终态原因等事件；不展示模型内部推理 | BR-SYS-STATE-007、SM-AG-01 §2.1 |

> 授权非来源声明：已注册工具、Prompt、前端参数、兼容默认值、现有代码与测试绿灯均不构成当轮授权
> （X-SHARED-002，A-SYS-I-P-01 §5）。

**复用既有约束**：X-SEQ-001（门禁前置，A-SYS-I-P-03）、X-FORBID-004（workspace 限界）、
X-FORBID-005（不绕过 ED 接口）、X-SHARED-001（共享结构经 SYS）、X-SEQ-003/X-INTER-001
（建机前置与跨模块转移登记）。

### 7.2 TaskRun 编排重构契约（2026-08-15 · AG-1.0-TASKRUN-REWORK-DESIGN-01，承接 A-AG-M-D-01 §5b / A-AG-M-S-01 §2.2.2）

#### 7.2.1 数据契约（迁移 v12）

- `task_plans(plan_id TEXT PK, task_run_id TEXT NOT NULL, plan_json TEXT NOT NULL, created_at INTEGER, updated_at INTEGER)`
  — plan_json：步骤列表、依赖、当前游标、所需授权、确认点标记、阶段结果（可展示操作计划，非思维链）；
- `task_plan_steps(step_id TEXT PK, plan_id TEXT NOT NULL, seq INTEGER, kind TEXT, label TEXT,
  deps_json TEXT, required_grant TEXT, confirm_point INTEGER, status TEXT, updated_at INTEGER)`;
- `decision_requests(request_id TEXT PK, task_run_id TEXT NOT NULL, kind TEXT, payload_json TEXT,
  status TEXT(check proposed/resolved), outcome TEXT, proposed_at INTEGER, resolved_at INTEGER)`;
- `task_runs` 保留：`status` 语义修正（waiting_confirm 仅由 DecisionRequest 产生；executing 不再由任意新消息
  从 waiting_confirm 绕回）；索引沿用 idx_task_runs_session。

#### 7.2.2 事件契约（AgTurnEvent 扩展）

- `step_progress`：`{ turn_id, action: "reading"|"tool"|"generating"|"proposing"|"deciding", label, detail? }`
  ——实时当前动作（底部状态条数据源）；
- `decision_request_proposed`：`{ turn_id, request_id, kind, payload }`（权限/方案/范围/人工确认）；
- `decision_request_resolved`：`{ turn_id, request_id, outcome }`（confirm / cancel）；
- `diff_proposed` 保留（内容层：只驱动 Diff 生命周期，**不驱动 TaskRun**）。

#### 7.2.3 UI 契约

- **移除**顶部 TaskRun 卡（TaskRunCard 废弃；agStore.taskRuns / ag.list_task_runs 保留为任务记录查询）；
- 信息流保留真实步骤事件（ToolCallCard / DiffCard 不动）；
- 信息流底部实时"当前动作"状态条（step_progress 驱动，如"正在读取 a.md"）；回合结束后收敛为状态结论
  （等待确认 / 等待补充 / 完成 / 失败 / 中断），全部来自运行事件；
- DecisionRequest 卡（kind 区分权限/方案/范围/人工确认；裁决动作绑定 confirm/cancel → TaskRun 事件）。

### 7.3 WorkspaceOperation 与 TaskRun 边界（2026-08-21）

- 资源整理步骤由 AG 生成 WorkspaceOperationPlan，DecisionRequest.payload 只引用冻结的 plan_id、摘要、影响与基线；不得把可变工具参数当执行计划。
- TaskRun 只负责“识别该步骤需要裁决、等待 DecisionRequest、在确认后调用 WS 执行器、记录结果并继续/终止”；资源操作的核验、磁盘执行、补偿与 WorkspaceTimeline 归 WS。
- `userConfirm` 立即恢复原 TaskRun 并进入 WS Revalidating/Executing，不增加 `WaitingUserCommit`、资源 DiffCard 或第二次保存确认。
- `userCancel` 终结该资源步骤且 DiskState 不变；若用户选择继续其他不依赖步骤，必须由明确 TaskPlan 分支表达，不能静默跳过。
- 应用重启后，未完成 WorkspaceOperation 进入 Interrupted；TaskRun 可显示中断和提供“重新计划”，但不得自动重放旧确认或执行旧计划。
- CreateDocumentBundle 的资源创建完成后，初始内容 Diff 只是普通内容提案事件；它不把 TaskRun 再次置为 WaitingConfirm。TaskRun 可在提案成功投影后继续执行后续无需额外权限的步骤。

## §8 需求到规则映射（REQ_MAP）

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
req_id: REQ-AG-001
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

## §9 测试与验收

- **门禁前置无绕过**（BR-AG-VERIFY-001，REQ-AG-003 验收）：静态可查不存在不经 GatePassed 的
  模型调用路径；无订阅时 chat 明确拒绝、assist 静默；断网时本地功能不受影响。
- **回合生命周期**（BR-AG-STATE-001 / SM-AG-01）：回合可中断且无半落盘；修改建议唯一出口交 DE；
  DiffCard 带会话/回合/消息锚点并显示在对应 assistant 消息后；失败可重试。
- **工具循环**（BR-AG-STATE-002 / SM-AG-02）：未注册/越权工具被拒并回注；循环上限生效；注入写盘
  意图的工具调用不产生旁路写入；内容走 DE Diff，资源只生成 WorkspaceOperationPlan 并等待 DecisionRequest。
- **资源操作编排**（WS-OPERATION / SM-WS-04）：确认前零副作用，确认后 TaskRun 调用 WS 重验执行；拒绝、过期、
  脏标签阻断、fail-fast 补偿、partial_failed、重启 Interrupted 与 CreateDocumentBundle 安全拒绝均有跨模块验证。
- **上下文限界与裁剪**（BR-AG-DATA-001）：越界读取被拒；裁剪确定性且显式注入优先保留、可追溯。
  已打开文档覆盖测试必须包含：active live 内容覆盖 `loadedContent`、非激活 open session 内容覆盖 DiskState、
  标签切换过渡期 active binding 未对齐时读取目标 session LogicalState 且回写旧绑定文档、session 尚未载入时短暂等待、
  `search_text` 命中未保存 LogicalState、ReferencePayload.selection 失效时不作为当前事实。
- **辅助回合**（BR-AG-STATE-003 / SM-AG-03）：latest-wins 生效（新请求覆盖旧请求）；辅助续写
  返回 3 候选且 Tab/1/2/3 采纳正确；局部修改支持选区替换与光标插入，多轮仅以上轮候选为基底；
  采纳走 ED 受控接口产生正常脏态；无订阅/断网静默。
- **历史持久化**（BR-AG-PERSIST-001）：重启后会话历史可恢复；无历史内容网络上行（可经网络核验）。
  错误、取消、门禁拒绝和 draft 均可见；draft 显示“未完成，可重试”，重试复用同一 user message 新建回合。
- 量化 NFR（首 token 延迟、补全延迟、节流参数）延 G5 标定（AG-05）。

## §10 技术债务与盲区（诚实声明）

- **工具注册清单与权限分级的具体内容**（AG-02 余项）：校验机制与写盘禁令已锁，清单/分级延实现期
  运营配置，写盘类工具的 diff 化适配与 DE G2 联合。
- **历史后台压缩策略**：200MB 软上限、软归档与单会话删除已锁；后台压缩策略后续按数据库体积表现再定。
- **已打开非激活文档样式类自动应用**：文本/块替换由 ED session 事务承接；StyleOp 若缺少无 ProseMirror
  runtime 的可逆事务实现，必须让 DiffCard 失效并记录债务，不能降级为 DiskState 应用。
- **节流与成本参数**（AG-03 余项）：机制已锁，数值延 G5 标定。
- **构建移交/回落**（AG-04）：与 CBT 的状态机对接联合 CBT G2，本文不预定义接管协议。
- ~~单文档回合的跨文档产出边界待裁定~~ **已补（B-4，2026-06-12）→ 2026-07-04 再裁：单文档铁律废止，现行边界见 BR-AG-DATA-003**。原 B-4 结论（限当前文档、
  跨文档升级 CBT、DE 侧协同校验）已被取代。以下原始记录留痕——
- **单文档回合的跨文档产出边界待裁定**（2026-06-12 人类指示后续细化）：编辑模式代理回合能否产出
  指向 @mention 引用文档（非当前文档）的 ChangeProposal 未显式裁定——倾向限定当前文档、跨文档
  修改一律升级 CBT 构建路径；G3 前与 DE 联合补一句边界裁定。
- 模型选择、提示词、上下文预算数值属实现期/运营配置，不入规则。

## §11 后续动作

1. 落地后随 `npm run governance:generate` 注册：AG-TURN/AG-TOOL/AG-ASSIST 链路、
   BR-AG-STATE-001/002/003、BR-AG-VERIFY-001、BR-AG-DATA-001、BR-AG-PERSIST-001 规则、
   X-FORBID-006 约束、TERM-AG-001~004、REQ_MAP 进 ADUS。
2. 续 A-AG-M-S-01（SM-AG-01 回合机 / SM-AG-02 工具循环机 / SM-AG-03 辅助回合机），回填总图
   四条 planned 行（代理回合 / tool-call / autocomplete / inline-assist）。
3. 待人类宣布 AG G2 闭合；DE G2 承接产出契约；CBT G2 联合接管协议。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
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
| 2026-08-15 | v2.39 | AG-1.0-TASKRUN-REWORK-REQ-01：登记 TERM-AG-017（DecisionRequest 任务级裁决请求）/ TERM-AG-018（TaskPlan 任务计划）；§7.1 语义纠偏契约承接（三层分离、确认 gate、片段执行）以 A-AG-M-D-01 §5b 与 A-AG-M-S-01 §2.2.1 为准。 |
| 2026-08-15 | v2.40 | AG-1.0-TASKRUN-REWORK-DESIGN-01：新增 §7.2 TaskRun 编排重构契约——数据契约（迁移 v12：task_plans / task_plan_steps / decision_requests；task_runs 语义修正）、事件契约（step_progress / decision_request_proposed / decision_request_resolved；diff_proposed 不驱动 TaskRun）、UI 契约（移除顶部卡、底部当前动作状态条、DecisionRequest 卡）。 |
| 2026-08-15 | v2.41 | AG-1.0-TASKRUN-REWORK-IMP-RUNTIME-01：实现 §7.2 运行时/数据层——迁移 v12 三表（task_plans/task_plan_steps/decision_requests）；types 增 DecisionRequestV1/TaskPlanV1/TaskPlanStepV1；task.rs 增 propose_decision/resolve_decision/has_pending_decision/pending_decision/latest_pending_decision_for_session/create_plan/list_plans_by_run；begin_turn 不绕回 waiting_confirm；settle_turn 按未决 DecisionRequest 判定 WaitingConfirm（去除 has_pending_proposal）；lib.rs 回合入口 gate（AG_DECISION_PENDING）+ ag.propose_decision/ag.resolve_decision IPC + StepProgress 事件；ipc AgTurnEvent 增 StepProgress/DecisionRequestProposed/DecisionRequestResolved。 |
| 2026-08-15 | v2.42 | AG-1.0-TASKRUN-REWORK-IMP-UI-01：实现 §7.2.3 UI——移除顶部 TaskRun 卡（TaskRunCard 删除）；信息流底部实时当前动作状态条（step_progress→agStore.currentAction）+ DecisionRequest 卡（裁决 confirm/cancel→ag.resolve_decision）；回合终态清 currentAction、待裁决跨回合保留（三层分离）；agRuntime +2。 |
| 2026-08-15 | v2.43 | AG-1.0-TASKRUN-REWORK-PROMPT-01：prompts.rs 文案修正——DiffCard 只确认修改内容、任务级确认走 DecisionRequest（删"接受/拒绝即人类确认"总括）；同一执行片段内连续多张 Proposal（去跨轮分批诱导）；同文档 Diff2 基于前卡成功预览后 LogicalState、前卡失效不形成补丁栈。 |
| 2026-08-15 | v2.44 | AG-1.0-TASKRUN-REWORK-CONTINUE-01：auto-continue（确认后自动续片段）——resolveDecisionAndContinue（confirm→自动 startTurn 续片段、MAX=3 防死循环、手动发送重置计数；cancel 终止）；DecisionRequest 卡"确认并继续"；TaskPlan 呈现——ag.list_plans/ag.create_plan IPC + TaskPlanView/AgSession.taskPlans + PlanCard（步骤/依赖/游标/确认点/授权）。 |
| 2026-08-15 | v2.45 | AG-1.0-TASKRUN-REWORK-INTENT-01：IntentResult 复杂目标 → 自动 TaskPlan——is_complex_target 判定（写意图 + 多文档 或 单文档整篇信号）；回合内 intent_result 后接线（build_task_plan：每文档读取→生成建议/确认点，read→propose 依赖；同 run 已有计划不重复建）；前端 listPlansByRun snake→camel 映射 + refreshSessionPlans（done/error 后刷新）。 |
| 2026-08-15 | v2.46 | AG-1.0-DECISION-CARD-01：任务裁决卡重构——独立固定裁决区（DecisionBar，Composer 上方，任务暂停等决策；类型化选项：permission 允许本次/拒绝、plan_choice 等确认并继续/取消）；输入门控（pendingDecision 时手动发送拦截 + startTurn 前端 gate，auto-continue 除外）；纯确认/拒绝短词 intent 降权 Answer（不落入 general 档 Edit 强制工具）；提示词强化删除走 delete_doc/delete_folder（禁止 replace_document 清空代替删除）。 |
| 2026-08-21 | v2.47 | 工作区资源操作基线重构：BR-AG-STATE-002、X-FORBID-006 与 TaskRun 契约改为 WorkspaceOperationPlan + DecisionRequest 前置裁决、确认后 WS 直接执行；资源操作不生成 DiffCard；CreateDocumentBundle 在资源创建后衔接普通内容 Diff；旧 AG_DELETE 链降为迁移债务。 |
| 2026-08-22 | v2.48 | WS-RESOURCE-WRITE-GUARD-01：移除「新文档以空基准 ChangeProposal 创建」旧表述——新建文档唯一入口为 plan_workspace_operation（CreateDocumentBundle 确认后建盘并投影初始内容 Diff）；propose_edit 目标必须已存在（内容 Diff 不得创建资源，X-FORBID-006，工具层路径级拒绝）。 |
| 2026-08-14 | v2.36 | AG-1.0-D3-AG-REF-AST-RUNTIME-CONTRACT-01：注册 TERM-AG-013~016（RuntimeProfile/CapabilityGrant/TaskRun/TaskStep）；新增 §7.1 Agent 1.0 运行时契约落地（REQ-AG-007/015）——RuntimeProfile 入口校验、CapabilityGrant 五因素收窄、IntentResult 收敛、TaskRun 接力、StructuredVisibleEvent 不含内部推理、授权非来源声明。 |
| 2026-08-13 | v2.35 | AG-STRATEGY-05：BDocx 整篇改写收敛为无表格且无图片的动态条件并由服务端拒绝受保护/新引入结构；block_ops 同锚点顺序、后置校验与稳定 id reverse 闭合；proposal、编辑器预览、接受和保存的真实状态与最小运行事件对齐。 |
| 2026-08-11 | v2.33 | AG-CTX-01：§5.1 登记上下文管理重构——list_blocks 轻量投影、read_doc blocks 范围参数、移除工具结果截断、AG_EDIT_TOOL_REQUIRED 文案区分、查询动词→Search、整篇对齐检测、序列化瘦身；语义详见 A-AG-M-T-02 §4 |
| 2026-08-11 | v2.32 | AG-DOCX-RO-01：§5.1 登记原生 docx 只读——read_editable_snapshot 放行原生 docx 走 pandoc 转换链，source=doc_convert_readonly、editable_kind 覆盖 html（≠bdocx），结构性只读；propose_edit 仍拒绝、编辑器路径保持 DOC_NOT_BDOCX；语义详见 A-AG-M-T-02 §4 |
| 2026-08-11 | v2.31 | AG-R2-01：§5.1 登记整理复原综合修复——预算（工具结果历史压缩 + block_ops 单批上限 24）、意图（纠错语义词 + 疑问文档指涉→只读）、提示词（编辑当轮提案 + 顺序自查）、范围（workspace 内只读放宽 + 锚点矛盾校验）；语义详见 A-AG-M-T-02 §4 |
| 2026-08-10 | v2.30 | AG-INTENT-V3-01：§5.1 登记乐观授权——工具授予三分类固定（只读组无条件 + 提案组不落盘），意图识别退化为上下文注入器，识别失败不再零工具瘫痪；resolve_capability_grant 同步固定工具集，BR-AG-STATE-002 allowed_tools 逐字满足；语义详见 A-AG-M-T-02 §4 |
| 2026-08-09 | v2.29 | AG-INTENT-V2-01：§5.1 登记 resolve_intent_result（BR-AG-DATA-004）内部升级 Parsing→Adjudication 语义化判定；修复「改为红色」「创建md文档」误判 Answer；IntentResultV1 契约不变，IntentSignal 私有，语义详见 A-AG-M-T-02 §4 |
