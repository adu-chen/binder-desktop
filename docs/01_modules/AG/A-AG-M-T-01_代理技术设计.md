---
文档编号：AG-M-T-01
文档状态：A
负责模块：AG
文档职责：AG 代理模块级 L2 技术设计（规则 SSOT）——代理回合（代理编辑：编辑/构建双模式）生命周期、模型调用门禁前置、工具调用循环与权限边界、上下文组装、回合历史持久化原则、辅助续写与局部修改回合；承接 REQ-AG-001~005（模块旧称「对话代理」，2026-07-04 起正式名「代理（Agent）」）
上游约束：A-AG-M-D-01_代理功能需求描述.md（REQ-AG-001~005、AG-01~05）、A-CORE-C-P-01_APC.md（§4.1/§4.3 云薄端、§6、§13）、A-SYS-I-P-03_云接口契约.md（SM-SYS-01、X-SEQ-001、CLOUD-GATE/INVOKE、TERM-CLOUD-001~004）、A-SYS-I-P-02_IPC协议.md（BR-SYS-STATE-002 流式终态、统一错误形状）、A-SYS-I-P-01_数据结构定义.md（X-SHARED-001、BR-SYS-PERSIST-001）、A-WS-M-T-01_工作台技术设计.md（X-FORBID-004）、A-ED-M-T-01_编辑器技术设计.md（ED-DECOR、X-FORBID-005）、A-DE-M-D-01_DiffEdit功能需求描述.md（REQ-DE-001 产出消费）、ADU.md（§3）
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
状态下经快捷键**手动触发**（非键入自动触发），基于上下文感知直接生成一段后续内容（50–100 字
量级，数值属运营配置）；单候选、零交互、无对话；同过门禁、同经装饰层、采纳同为用户编辑操作
（REQ-AG-005）。

<!-- TERM
term_id: TERM-AG-006
chains: AG-TURN
zh: 编辑模式
en: EditMode
forbidden: 对话模式, chat模式, chat 模式, 对话编辑模式
-->
代理编辑（TERM-AG-008）面板双模式之一（2026-07-04 人类裁决）：默认面向当前文档的 AI 精确修改
（编辑器感知 + 块级锚定，产出一律经 DE diff 裁决、人最后裁定），兼具文件创建、文件树整理
（移动/重命名/新建文件夹直接执行；**删除须用户确认**）、项目感知、对话与资源获取。模式是默认
姿态非能力墙——与构建模式互相复用彼此能力。注：编辑器/文档层的「编辑态 vs 预览态」（docx 打开
形态）与本模式无关，不得混用。

<!-- TERM
term_id: TERM-AG-007
chains: AG-TURN
zh: 构建模式
en: BuildMode
forbidden: agent模式, agent 模式, Chat Build, chat-build
-->
代理编辑（TERM-AG-008）面板双模式之一（2026-07-04 人类裁决）：对话阶段（多轮需求搜集→项目蓝图
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
（编辑模式 EditMode + 构建模式 BuildMode），会话持久化、产出经 DE diff 裁决、工具调用、项目
构建。沿革：旧版曾以「对话编辑」总称整个 agent 能力，该词后又错位为 DE 模块中文名——三重身份
致混，2026-07-04 裁定：正式名「代理编辑」，「对话编辑」全面退役为禁用别名；DE 模块更名
**Diff Edit**（不再有中文名）。
AI 编辑三层：①辅助续写（TERM-AG-004，光标级）②局部修改（TERM-AG-003，选区/光标级）
③代理编辑（本条，面板级）；①②采纳即用户编辑直改不经 diff，③产出经 DE 裁决——轻重分界
（BR-AG-STATE-003 / X-FORBID-006）。

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
AG 代理（本文 SSOT）——代理编辑（TERM-AG-008）面板：编辑模式 + 构建模式（构建编排归 CBT）
  ├─ 代理回合（§3，AG-TURN）        组装→门禁→网关流式→产出处置；历史持久化原则
  ├─ 工具调用循环（§4，AG-TOOL）    注册/权限校验→执行→回注→循环控制
  ├─ 上下文组装（§5）               文档/注入载荷/本地资产，workspace 限界
  └─ 辅助续写与局部修改（§6，AG-ASSIST） 快捷键触发→防抖→轻量调用→装饰层呈现→采纳即编辑
```

- 模型调用唯一路径：`AG → SM-SYS-01 GatePassed → AiGateway`（X-SEQ-001）；失败/拒绝走统一
  错误形状；流式遵守 BR-SYS-STATE-002（完成/中断/错误三类终态，中断无半落盘副作用）。
- AI 修改落盘唯一路径：`AG 产出 → DE 生成 pending_diff → 人类裁决 → WS 落盘`（X-FORBID-006）。

## §3 代理回合（AG-TURN）

<!-- CHAIN
chain_id: AG-TURN
主责模块: AG
status: active
risk_tier: P1
-->

<!-- RULE
rule_id: BR-AG-STATE-001
主链路: AG-TURN
域: STATE
-->
代理回合必须经完整生命周期：上下文组装→门禁裁决→经网关流式调用→产出处置；回合可中断/取消，
中断为终态且不产生半落盘副作用（对齐 BR-SYS-STATE-002）；含修改建议的产出唯一出口是交 DE 生成
pending_diff（AG 无直接写盘路径，X-FORBID-006）；回合失败回统一错误形状且可重试。状态转移归
SM-AG-01（A-AG-M-S-01）。

<!-- RULE
rule_id: BR-AG-DATA-003
主链路: AG-TURN
域: DATA
-->
代理回合修改建议产出（ChangeProposal）的目标边界（2026-07-04 人类裁决，**取代原「单文档铁律」**）：
target 一律限 active workspace 内（X-FORBID-004）；**默认目标为当前文档**；用户**显式指向**其他
文档（@mention 引用/明示路径）或明确要求创建新文档时，可产出指向该文档/新文档的 ChangeProposal
（新建以空基准表达，承 BR-DE-DATA-002）；一切内容产出仍经 DE 裁决落盘（X-FORBID-006）。识别到
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
调用的路径；无有效订阅或断网时：代理/工具回合明确拒绝（统一错误形状），局部修改/辅助续写静默
不提供且不打断编辑。本规则是 SYS 门禁契约在 AG 的承接执行，不重定义裁决语义（裁决归服务端）。

<!-- RULE
rule_id: BR-AG-PERSIST-001
主链路: AG-TURN
域: PERSIST
-->
回合与对话历史属 workspace 控制态：本地持久化（workspace.db 扩展表，遵守 BR-SYS-PERSIST-001
版本化迁移），重启后会话历史可恢复；对话内容不因历史持久化上行云端（本地优先，REQ-CORE-001）。
扩展表具体列定义在进入实现前与 WS/SYS 联合敲定并回写 A-SYS-I-P-01（X-SHARED-001），本规则先锁
「本地持久化 + 可恢复 + 不上云」原则。

## §4 工具调用循环（AG-TOOL）

<!-- CHAIN
chain_id: AG-TOOL
主责模块: AG
status: active
risk_tier: P1
-->

<!-- RULE
rule_id: BR-AG-STATE-002
主链路: AG-TOOL
域: STATE
-->
工具调用循环必须满足：模型请求的工具必须在注册清单内且通过权限边界校验方可执行（未注册/越权
一律拒绝并将拒绝原因回注模型）；执行结果（含失败）回注后继续回合；循环有上限，达上限中止；
用户可随时中断，中断为终态；**任何文件内容写入意图不得由工具直接落盘，必须产出修改建议走 DE
diff 路径**（X-FORBID-006）；只读类工具（读文档/搜索）经 WS 且限定 workspace 内（X-FORBID-004）；
**文件树结构类工具**（移动/重命名/新建文件夹）不属内容写入，经 WS 命令直接执行、不经 diff，
**删除类工具须经用户确认后执行**（2026-07-04 人类裁决：整理直接执行，删除有风险须确认）。
工具注册清单与权限分级的具体内容延实现期运营配置，本规则锁校验机制与写盘禁令。状态转移归
SM-AG-02。

## §5 上下文组装（AG-TURN）

<!-- RULE
rule_id: BR-AG-DATA-001
主链路: AG-TURN
域: DATA
-->
回合上下文的来源限定为：当前文档内容（经 WS 读取）、REF 注入载荷（@mention，含来源标识）、
AST/KB 本地资产（其供给契约随各自 G2 承接）；一切上下文读取限定 workspace 内（X-FORBID-004）；
超出上下文预算时裁剪策略必须确定性且用户显式选定的注入（@mention）优先保留，裁剪发生时对调用
可追溯（不静默丢弃显式注入）。预算数值与裁剪算法延实现期标定，本规则锁来源限定与裁剪优先级语义。

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
| 上下文注入层 | 来源限定承 BR-AG-DATA-001（显式 @mention 不被静默丢弃）；当前文档为当前轮主事实、记忆/知识为补强层——完整分层与**截断序**见 M-T-02 §2/§5（BR-AG-DATA-005，**当前文档 P0 永不截**） | 顺序设计锁；截断序细化归 M-T-02 |
| 用户输入层 | 本轮 prompt / 编辑触发上下文 | — |

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
A-CBT-M-T-01）、**辅助回合**（辅助续写/局部修改；轻量、无系统对话历史、续写/局部改写导向）。提示词文本/措辞/模型档属实现期运营配置不入规则；
本规则只锁组装分层、注入顺序与三类回合骨架差异（**截断序不在本规则，归 BR-AG-DATA-005，去重单一化**）。

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

## §6 辅助续写与局部修改（AG-ASSIST）

<!-- CHAIN
chain_id: AG-ASSIST
主责模块: AG
status: active
risk_tier: P1
-->

<!-- RULE
rule_id: BR-AG-STATE-003
主链路: AG-ASSIST
域: STATE
-->
局部修改与辅助续写回合必须满足：经用户快捷键**手动触发**（2026-07-04 澄清：非键入自动触发），
触发须防抖/防重（单次操作不产生多次模型调用）；局部修改弹窗内**可多轮**（上轮产出为下轮基底，
每轮逐次过门禁）；**辅助对话不持久化**（不入回合历史，与代理编辑面板层的会话持久化形成注册
分界）；每次调用同样经门禁与网关（BR-AG-VERIFY-001，无例外路径）；候选经 ED 装饰层呈现
（ED-DECOR），锚失效放弃本次呈现；**采纳是用户编辑操作**——经 ED 受控接口进入编辑模型并走
SM-ED-01→SM-WS-01 正常脏态/保存路径，不构成 AI 直写文件；无订阅/断网/调用失败时静默不提供，
不打断编辑、不弹错误。防抖参数与成本策略延实现期标定（AG-03 的数值部分），本规则锁机制语义。
状态转移归 SM-AG-03。
**样式操作同语义（2026-07-03 裁定 D-3）**：用户显式触发的局部修改样式操作（StyleOp，
A-ED-M-T-01 §5.1）经 ed.apply_edit 视为用户编辑直改；chat/agent **回合产出**的样式修改不走
本路径，一律经 DE diff 裁决落盘（X-FORBID-006 / BR-ED-DATA-005），不得借内联语义绕过裁决。

## §7 跨模块约束（CONSTRAINT）

<!-- CONSTRAINT
constraint_id: X-FORBID-006
摘要: AG 及其工具调用不得绕过 DE diff 路径直接写入 workspace 文件内容；AI 的文档内容修改落盘唯一路径为 AG 产出→DE 裁决→WS 落盘。文件树结构操作（移动/重命名/新建文件夹）不属内容写入、经 WS 命令直接执行（限 workspace，X-FORBID-004）；删除文件须用户确认后执行（2026-07-04 人类裁决）
涉及模块: AG, DE, WS, CBT
链路影响: AG-TURN, AG-TOOL, WS-DIFF-APPLY
-->

**复用既有约束**：X-SEQ-001（门禁前置，A-SYS-I-P-03）、X-FORBID-004（workspace 限界）、
X-FORBID-005（不绕过 ED 接口）、X-SHARED-001（共享结构经 SYS）、X-SEQ-003/X-INTER-001
（建机前置与跨模块转移登记）。

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

## §9 测试与验收

- **门禁前置无绕过**（BR-AG-VERIFY-001，REQ-AG-003 验收）：静态可查不存在不经 GatePassed 的
  模型调用路径；无订阅时 chat 明确拒绝、assist 静默；断网时本地功能不受影响。
- **回合生命周期**（BR-AG-STATE-001 / SM-AG-01）：回合可中断且无半落盘；修改建议唯一出口交 DE；
  失败可重试。
- **工具循环**（BR-AG-STATE-002 / SM-AG-02）：未注册/越权工具被拒并回注；循环上限生效；注入写盘
  意图的工具调用不产生直接文件写入（产出 diff 经 DE，G4/G5 与 DE/WS 联合验证）。
- **上下文限界与裁剪**（BR-AG-DATA-001）：越界读取被拒；裁剪确定性且显式注入优先保留、可追溯。
- **辅助回合**（BR-AG-STATE-003 / SM-AG-03）：节流生效（高频键入不逐次调用）；采纳走 ED 受控
  接口产生正常脏态；无订阅/断网静默。
- **历史持久化**（BR-AG-PERSIST-001）：重启后会话历史可恢复；无历史内容网络上行（可经网络核验）。
- 量化 NFR（首 token 延迟、补全延迟、节流参数）延 G5 标定（AG-05）。

## §10 技术债务与盲区（诚实声明）

- **工具注册清单与权限分级的具体内容**（AG-02 余项）：校验机制与写盘禁令已锁，清单/分级延实现期
  运营配置，写盘类工具的 diff 化适配与 DE G2 联合。
- **回合历史扩展表列定义**（AG-01 余项）：原则已锁（本地/可恢复/不上云），列定义进入实现前与
  WS/SYS 联合敲定并回写 A-SYS-I-P-01。
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
