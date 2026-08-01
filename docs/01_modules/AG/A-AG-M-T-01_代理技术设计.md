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
- AI 内容修改唯一路径：`AG 产出 → DE 生成 pending_diff → ED/DE 受控转正`；激活文档由人类裁决转入
  LogicalState，已打开非激活文档由基线校验通过后的自动应用事务转入 LogicalState，未打开文档才在
  人类接受后经 WS 落盘。AG 不得直接写 LogicalState 或 DiskState（X-FORBID-006 / X-FORBID-009）。

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
pending_diff（AG 无直接写盘路径，X-FORBID-006）；交 DE 生成待裁决修改时必须保留会话、回合和
本轮 assistant 消息锚点，使 DiffCard 回到对应 AgentTurn 信息流内呈现，而不是作为会话底部全局列表
沉底汇总；回合失败回统一错误形状且可重试。状态转移归 SM-AG-01（A-AG-M-S-01）。

<!-- RULE
rule_id: BR-AG-DATA-003
主链路: AG-TURN
域: DATA
-->
代理回合修改建议产出（ChangeProposal）的目标边界（2026-07-04 人类裁决，**取代原「单文档铁律」**）：
target 一律限 active workspace 内（X-FORBID-004）；**默认目标为当前文档**；用户**显式指向**其他
文档（@mention 引用/明示路径）或明确要求创建新文档时，可产出指向该文档/新文档的 ChangeProposal
（新建以空基准表达，承 BR-DE-DATA-002）；一切内容产出仍经 DE/ED 受控转正，不得由 AG 直写
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
调用的路径；无有效订阅或断网时：代理/工具回合明确拒绝（统一错误形状），局部修改/辅助续写静默
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
	不保存 before/after/styleOps 等 diff 快照。前端恢复时先拉 `agent_sessions` 元数据，再拉当前激活会话的
	turns；其他会话点击后按 `session_id` 懒加载。恢复 `diffRefs[]` 时必须按 `diffId` 查询 DE 当前状态：
	`pending` 可继续裁决，`accepted/rejected/expired` 只读展示，DE 找不到的 diff 只读标记为不可用，不得默认
	复活为 `pending`。
	`ag.delete_session(session_id)` 以当前 workspace + 指定会话为边界删除该会话历史记录；成功后前端从
	历史/标签列表移除该会话，若删除的是活动会话则切换到剩余会话，没有剩余会话时新建空会话并持久化。
	该命令不触碰 DE/WS 的修改建议状态。

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
diff 路径**（X-FORBID-006）；只读类工具（读文档/搜索/列块）必须经 AgentReadContext 并限定
workspace 内（X-FORBID-004）：已打开文档优先读取 OpenDocSnapshot 的 LogicalState，未打开文档才经
WS/DOC 读取 DiskState；已打开文档快照缺失、版本缺失或锚校验失败时必须拒绝工具结果或阻断回合，
不得用 DiskState 兜底；
**文件树结构类工具**（移动/重命名/新建文件夹）不属内容写入，经 WS 命令直接执行、不经 diff，
**删除类工具须经用户确认后执行**（2026-07-04 人类裁决：整理直接执行，删除有风险须确认）。
已接线非破坏性结构工具为 `move_doc`（移动/重命名文件）与 `create_folder`（建目录）；二者均限
workspace 内、目标冲突拒绝或幂等，不可覆盖。删除类工具在确认回路未落地前不得注册给模型，不得
用模型参数伪造用户确认。工具注册清单与权限分级的具体内容延实现期运营配置，本规则锁校验机制与
写盘禁令。状态转移归 SM-AG-02。

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
成功后 after_content 仍保持生成基线，实际草稿由 ED 单一 ProseMirror transaction 应用；富文档
`replace_block/replace_document` 必须在 PendingDiff 前返回 `AG_RICH_BLOCK_OPERATION_REQUIRED`。
HTML 字符串扫描只保留未打开文档只读兼容，不再承担结构编辑或重建职责。

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
  DiffCard 带会话/回合/消息锚点并显示在对应 assistant 消息后；失败可重试。
- **工具循环**（BR-AG-STATE-002 / SM-AG-02）：未注册/越权工具被拒并回注；循环上限生效；注入写盘
  意图的工具调用不产生直接文件写入（产出 diff 经 DE，G4/G5 与 DE/WS 联合验证）。
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
