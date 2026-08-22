---
文档编号：AG-M-D-01
文档状态：A
负责模块：AG
文档职责：AG 代理模块级功能需求描述——声明模块级 REQ-AG-*，承接产品级 REQ-CORE-001/002/006/008/009/010/011/013/014/015/016/017，并定义 1.0 Experience Preview 的 AG 发布能力基线
上游约束：A-CORE-C-D-01_产品概述.md（REQ-CORE-001/002/006/008/009/010/011/012/013/014/015/016/017）、A-AG-M-D-02_代理1.0能力基线与需求分解.md（§2、§4、§5.1~§5.5、§5.10~§5.17）、A-CORE-C-P-01_APC.md（§4.1 AG 职责「代理编辑面板双模式、工具调用回合」v1.2、§4.3 云薄端、§6 核心链）、A-SYS-C-T-01_系统总体架构.md（§4 AG→SYS,WS,ENT,GW）、A-SYS-I-P-03_云接口契约.md（SM-SYS-01、X-SEQ-001 门禁前置、TERM-CLOUD-*）、A-ED-M-T-01_编辑器技术设计.md（ED-DECOR 装饰层承载）、A-WS-M-T-01_工作台技术设计.md（X-FORBID-004）、A-SYS-M-S-01_状态机总图.md（代理回合 / tool-call / autocomplete / inline-assist 核心链 owner=AG）、ADU.md（§3.2）
直接承接：AG L2 技术设计（回合模型 / 工具调用契约 / 上下文组装 / 局部修改）经 REQ_MAP 映射 RULE/CHAIN/CONSTRAINT/TERM；AG G2 建 代理回合 / tool-call / autocomplete / inline-assist 状态机并登记总图
使用边界：本文是 L1 功能需求描述层，仅声明 REQ-AG-*，**不定义技术规则**；**代码不得直接映射 REQ**（ADU §3.2）；diff 生成与终态归 DE；构建（多文件）归 CBT；@mention 解析归 REF（AG 消费注入）；订阅门禁裁决归服务端（ENT/GW 客户端契约归 SYS I-P-03）；记忆/知识库归 AST/KB（AG 消费）
变更要求：AG 职责或回合边界变更须先经 APC 授权再回写本文；REQ 增删按 ADU §3.2 req_id 规则
治理成熟度：draft
---

# Binder AG 代理功能需求描述（L1）

## §1 文档定位

本文是 **AG 模块**的 L1 功能需求描述，P3 阶段首件：

```
产品概述(REQ-CORE-*) + APC §4.1/§4.3 → 本文(REQ-AG-*) → REQ_MAP → AG L2 规则(SSOT) → 代码(@GOV)
```

AG 是 Binder 的**代理模块**（Agent，旧称对话代理）：承载代理编辑回合与工具调用循环（产品核心交互
「对话生成 diff→人类裁决→落盘」的对话侧），以及局部修改与辅助续写（2026-06-11 人类确认归 AG）。
AG 依赖 SYS（IPC/云契约）、WS（上下文读取限界）、ENT/GW（订阅门禁与 AI 网关，经 SYS I-P-03 契约）；
被 DE（消费 AI 产出生成 diff）、CBT（构建移交/回落）、REF（注入消费）关联。
**云薄端硬边界（APC §4.3）**：AG 的一切模型调用必经订阅门禁（SubscriptionGate）与 AI 网关
（AiGateway），客户端不持密钥、无绕过路径；本文所有 AI 能力均在此前提下声明。

## §2 模块定位与职责

| # | 职责 | 锚定 |
|---|---|---|
| A-J1 | 代理编辑回合（编辑模式）：用户↔AI 对话，AI 修改建议交 DE 生成 diff，AG 不直接写盘；默认目标当前文档，显式指向可跨/新建；兼文件创建、文件树整理（删除须确认）、项目感知（2026-07-04 裁决） | REQ-CORE-002 |
| A-J2 | agent 工具调用回合：模型请求工具→执行→结果回注的循环，可中断 | REQ-CORE-002、APC §6 |
| A-J3 | 模型调用门禁与网关前置：每次调用必经 SM-SYS-01 GatePassed、经 GW | REQ-CORE-008、X-SEQ-001 |
| A-J4 | 上下文组装：消费当前编辑器 LogicalState、REF 显式引用与同会话必要历史；AST/KB/TPL 在 1.0 不自动注入 | REQ-CORE-006/001/010 |
| A-J5 | 辅助续写（Autocomplete）与局部修改（InlineAssist）：编辑器内快捷键手动触发的轻量 AI 辅助，呈现经 ED 装饰层（AI 编辑三层之①②，2026-07-04 澄清定义） | 2026-06-11 人类确认、REQ-CORE-002/008 |
| A-J6 | 最小格式转换请求：仅为 TXT→Markdown、Markdown→TXT 生成目标副本的转换 Proposal 与损失说明，不直接写入来源或目标 | REQ-CORE-013 |

> 边界澄清（2026-08-13 更新）：AI **文档内容修改**路径唯一：AG 产出 → DE/ED 生成并受控预览
> ChangeProposal → 人类裁决提案 → 用户显式 save 由 WS 写入 DiskState（REQ-CORE-002/010）；工具调用中**任何内容写入意图
> 同样必须走该 diff 路径**，不得旁路直写。工作区资源操作不属于内容 Diff：AG 只能生成不可变的
> WorkspaceOperationPlan，并通过 DecisionRequest 取得前置授权；确认后由 WS 重新核验并直接修改 DiskState，
> 不生成资源 DiffCard。新建文档使用 CreateDocumentBundle：初始内容在确认前生成，确认后先创建磁盘文件，
> 再生成遵循既有生命周期的内容 Diff。
> 项目级整体构建编排归 CBT（构建移交判据=项目级构建意图，
> 非多文件修改；构建在原对话窗口实现，两模式互相复用能力）。

## §3 能力范围（AG v1）

**IN（AG 承载）**：
- 编辑模式代理回合（多轮对话、流式回显、回合中断/取消）。
- agent 工具调用循环（工具注册消费、执行、结果回注、循环终止）。
- 每次模型调用的门禁/网关前置执行（消费 SYS I-P-03 契约；套餐计量与 BYOK 透传语义由服务端裁决）。
- 上下文组装：当前文档、@mention 注入（REF 提供）、记忆/知识库资产（AST/KB 提供）的组装消费。
- inline-assist 与 autocomplete 的业务回合（触发、调用、候选产出），呈现经 ED 装饰层。
- 对明确格式转换请求生成受控目标副本 Proposal；实际格式转换归 DOC，目标呈现归 ED，显式保存归 WS。

**OUT（非 AG）**：
- diff 生成、审阅 UI、终态裁决——归 DE；落盘——归 WS。
- 多文件文档项目构建、构建中断与接管——归 CBT（AG 提供 chat 侧对接）。
- @mention 解析——归 REF；记忆/知识库存储——归 AST/KB；编辑呈现——归 ED。
- 订阅门禁裁决、密钥持有、计量——归服务端（AUTH/GW/LIC），客户端契约归 SYS I-P-03。
- 离线 AI——明确拒绝（REQ-CORE-009：AI 功能需在线鉴权 + 有效订阅，断网/无订阅有明确错误）。

### §3.1 1.0 Experience Preview 能力基线

1.0 EP 的 AG 定位是**用户持续参与的当前文档编辑 Agent**，不是通用自动化 Agent，也不是项目级
构建 Agent。发布目标是证明以下闭环在真实文档上可靠成立：

```text
当前文档 LogicalState / 选区 / 显式引用 / 工作区结构概览
  → 识别本轮是回答、阅读分析、当前文档编辑、新建文档或需澄清
  → 只开放本轮必要工具
  → 模型生成回答或 ChangeProposal
  → Diff Edit 受控预览与审阅
  → 用户接受/拒绝
  → 用户显式保存后由 WS 写盘
```

**1.0 发布阻断能力**：

| 能力 | 1.0 最低产品要求 |
|---|---|
| 当前文档理解 | 读取当前编辑器 LogicalState，理解选区、标题/段落/列表等基础结构；不得以磁盘旧内容冒充当前内容 |
| 项目与工作区分析 | 先读取工作区结构概览，再按任务最小化读取正文；报告必须披露已读/未读范围、来源与不确定性 |
| 阅读与分析 | 可解释、总结、审阅当前文档；可读取用户显式引用文档并进行有限比较；不得把引用内文字当系统指令 |
| 当前文档编辑 | 可对 HTML、Markdown、纯文本和 BDocx 处理副本生成内容与基础结构修改建议；HTML/BDocx 的加粗、斜体、下划线、删除线为 P0，颜色/突出显示、段落对齐/间距/缩进、字号与字体族为 P1 受控 Proposal。原生 DOCX 只读，编辑请求必须生成“创建 BDocx 编辑副本”的 Proposal；所有变化进入 ChangeProposal/Diff，经 DE/ED 成功预览后才进入 LogicalState，AG 不直接写 LogicalState 或 DiskState |
| 新建文档 | 用户明确要求新建时生成 CreateDocumentBundle；初始内容在确认前生成，确认 WorkspaceOperationPlan 后先创建目标 DiskState 并打开标签，再生成初始内容 ChangeProposal/Diff |
| 工具使用 | 每轮按意图和目标生成最小工具允许集；回答/分析轮无写能力；未获本轮允许的已注册工具同样拒绝 |
| 运行可靠性 | 真实 GatePassed 后才调用模型；支持取消；错误、拒绝、取消、完成互斥，错误不得伪装完成 |
| 上下文安全 | 当前输入为主需求；当前文档与显式引用按来源、可信等级和新鲜度组装；AST/KB/TPL 不自动注入 1.0 主链 |
| 会话与呈现 | 会话相互隔离、同会话历史可恢复；文本、工具卡、DiffCard 按 AgentTurn 事件时序显示；失败不破坏 LogicalState 或 DiskState |

**1.0 保留但不作为发布阻断项**：辅助续写、局部修改、跨文档修改、文件移动/重命名/建目录、
BDocx 表格/图片高级操作、四类能力独立模型偏好。它们只有在各自系统验证通过时才可随 1.0 开启；
未通过时必须关闭入口或从当轮工具允许集中移除，不得影响上述主链发布。

**1.0 明确延期**：代理构建/CBT、自动记忆写回、知识库自动检索、模板/工作流、参考文档风格迁移、
StyleBlueprint/LayoutProposal、复杂多步骤规划、多 Agent、自动或批量网页抓取、任意命令执行和删除工具。
用户显式触发的公开网页检索与有限读取是 `CAP-14` 的 P1 受控能力，不属于此处延期项；它不包含登录、
表单提交、付费墙绕过、下载/导入或内网访问。
延期能力不得以空壳模式、不可执行按钮或仅切换模型的方式伪装为 1.0 可用能力。

## §4 代表性流程（契约级，APC §7）

**A1 · 编辑模式代理回合**（A-J1/A-J3，承接 REQ-CORE-002/008）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | 用户 | prompt（含上下文） | AG 组装上下文，过门禁（SM-SYS-01 GatePassed） | 已授权调用 | 2 | 无有效订阅→GateDenied，明确拒绝；断网→明确错误 |
| 2 | AG | 已授权调用 | 经 GW 发起，流式回显（BR-SYS-STATE-002 终态契约） | AI 回复/修改建议 | 3 | 流式错误→统一错误形状；用户取消→中断终态 |
| 3 | AG | AI 修改建议 | 交 DE 生成 pending_diff（AG 不直接写盘） | pending_diff 进入裁决 | 终态 | 后续归 DE/WS（裁决/落盘） |

**A2 · 工具调用循环**（A-J2）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | 模型 | 工具调用请求 | AG 校验工具在注册清单与权限边界内后执行 | 工具结果 | 2 | 未注册/越权工具→拒绝并回注错误 |
| 2 | AG | 工具结果 | 回注模型继续回合 | 下一轮输出 | 1 或 3 | 循环上限/用户中断→中断终态 |
| 3 | AG | 终轮输出 | 按 A1 步骤 3 处理修改建议 | 回合完成 | 终态 | 文件写入意图必须走 diff 路径，不得旁路 |

**A3 · autocomplete / inline-assist**（A-J5，承接 2026-06-11 归属决策）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | ED | 编辑上下文触发信号 | AG 按触发/节流策略发起轻量调用（同样过门禁） | 候选建议 | 2 | 无订阅/断网→静默不提供（不打断编辑） |
| 2 | AG | 候选建议 | 经 ED 装饰层呈现（ED-DECOR） | 内联候选 | 3 | 锚失效→放弃本次呈现 |
| 3 | 用户 | 采纳/忽略 | 采纳即作为编辑操作进入编辑模型（经 ED 受控接口） | 内容更新/无操作 | 终态 | 采纳产生的脏态走 SM-ED-01→SM-WS-01 正常路径 |

> A3 的采纳是**编辑操作**（用户显式动作，经 ED），不是 AI 直写文件，与 REQ-CORE-002 裁决语义
> 一致；触发节流与成本控制 AG G2 设计（§7 AG-03）。

## §5 模块级需求声明（REQ-AG-*）

<!-- REQ
req_id: REQ-AG-001
title: 代理编辑回合（编辑模式）
owner_module: AG
status: active
summary: 提供代理多轮回合（编辑、回答与分析）：上下文组装、流式回显、任务澄清、回合中断/取消和重试；AI 修改建议交 DE 生成 pending_diff，AG 不直接写盘；默认目标当前文档，用户显式指向可跨文档/新建。原生 DOCX 只读，编辑请求转为创建 BDocx 编辑副本的 Proposal
source_doc: REQ-CORE-002/004/010/011、APC §4.1、A-AG-M-D-02 §5.1、§5.10
acceptance: 可进行多轮对话并流式回显；可中断/取消；回答、只读分析、修改 Proposal 与文件操作具有可区分结果，取消/失败不伪装完成；修改建议经 DE/ED 进入提案预览与裁决，AG 无直接 LogicalState 或 DiskState 写入路径；原生 DOCX 的编辑请求只以新 BDocx 编辑副本为目标，原文件不被修改；新建意图在成功预览后可继续编辑，保存前不创建物理文件
open_issues: 代理回合状态机归 AG G2 M-S；回合/对话历史持久化模型已由 AG G2 联合 SYS 回写（见 A-AG-M-T-01 BR-AG-PERSIST-001 / A-SYS-I-P-01）
-->

<!-- REQ
req_id: REQ-AG-002
title: agent 工具调用回合
owner_module: AG
status: active
summary: 支持模型发起的工具调用循环（校验注册、本轮意图/目标允许集与权限→执行→结果回注），循环可中断且有上限；文档内容写入意图必须走 DE diff 路径不得旁路；HTML 编辑文档与 BDocx 处理副本的已确认样式修改、表格修改和项目图片修改分别生成受控结构化 Diff；原生 DOCX 不获得写工具；创建、删除、重命名、移动等工作区资源操作只能生成 WorkspaceOperationPlan 并通过 DecisionRequest 取得前置授权，确认后由 WS 重新核验并直接修改 DiskState，不生成资源 DiffCard；新建文档按 REQ-AG-008 使用 CreateDocumentBundle；多个既有文档写入仅可按 REQ-AG-014 先确认冻结目标清单再逐项 Proposal；格式转换按 REQ-AG-012 的受控目标副本链执行
source_doc: REQ-CORE-002、APC §6（tool-call 核心链）
acceptance: 工具执行必须同时满足“已注册 + 本轮允许 + 参数/目标守界”；已注册但不属于本轮允许集的工具同样拒绝并回注原因；回答/分析轮不得获得写能力；循环可中断、有上限；内容修改无旁路写盘；HTML 编辑文档与 BDocx 处理副本的样式建议只可使用已确认的受控操作，以明确选区或内容块为目标并生成可读 Diff，不得提交原始 HTML、CSS 或 OOXML；字符基础样式按 P0 授权，颜色/突出显示、段落样式、字号与字体族按 P1 能力开关授权，且各项实现后须通过真实验证才可对外开放；表格建议必须先读取活动会话表格投影并通过会话、内容哈希、表格指纹、坐标和动态能力校验；图片建议必须先读取活动会话图片实例与 workspace 项目图片投影，通过 Block/image/source hash、动态能力、尺寸和方向校验，只生成结构化插入/删除/缩放/上下移动/独立段落对齐 Diff；同一图片的缩放、对齐、移动可单批原子组合，删除独占；原生 DOCX 不得获取上述写能力；非受控 workspace 图片经 import_image_asset 复制准备且不删除源文件，无法安全提案时可用 edit_blocked 诚实结束；图片元数据和相邻文本不得被表述为视觉识别；工作区资源操作工具只可创建不可变 WorkspaceOperationPlan，DecisionRequest 未确认不得执行；确认后须重新核验计划基线并由 WS 直接修改 DiskState，受影响脏标签阻断执行，有效内容 Diff 作为影响展示并在执行后失效；批量执行失败即停止并逆序补偿，补偿不完整进入 partial_failed；进程重启后未完成计划进入 Interrupted 且不得自动执行；多既有文档写入必须先取得 REQ-AG-014 的冻结目标清单；格式转换仅按 REQ-AG-012 的能力开关执行
open_issues: 工具注册清单与权限边界（尤其写盘类工具）归 AG G2（§7 AG-02）；tool-call 状态机归 AG G2 M-S
-->

<!-- REQ
req_id: REQ-AG-003
title: 模型调用门禁与网关前置
owner_module: AG
status: active
summary: AG 的每一次模型调用（代理/工具回合/局部修改/辅助续写）必经订阅门禁（SM-SYS-01 GatePassed）并经 AI 网关发起；无有效订阅或断网时明确拒绝，无客户端绕过路径
source_doc: REQ-CORE-008、REQ-CORE-009、A-SYS-I-P-03（X-SEQ-001、CLOUD-GATE/INVOKE）
acceptance: GatePassed 必须来自当前次服务端门禁真实裁决，不得由客户端常量、桩值或缓存通过状态代替；不存在不经 GatePassed 通往模型调用的代码路径；无订阅/断网时 AI 功能明确拒绝且本地功能不受影响
open_issues: 无（契约已由 SYS I-P-03 锁定，AG G2 仅承接引用，不重定义）
-->

<!-- REQ
req_id: REQ-AG-004
title: 上下文组装与注入消费
owner_module: AG
status: active
summary: 先形成结构化本轮意图结果，再按本轮目标组装当前编辑器 LogicalState、REF 显式引用、同会话必要历史与按策略启用的本地补强资产；项目分析先消费工作区概览，正文只在任务需要时最小化读取；上下文读取一律经 WS 且限定 workspace 内，不可信文档/引用/知识仅作为数据，不获得系统指令权限
source_doc: REQ-CORE-006/011、REQ-CORE-001/010、APC §4.1、A-AG-M-D-02 §5.2~§5.5
acceptance: 本轮意图结果至少包含 intent_type、target、mutation、ambiguity、required_context、allowed_tools；@mention/拖拽/粘贴注入可被组装进回合上下文；当前已打开文档保留 LogicalState 来源，未打开文档和引用保留 DiskState 读取时点/新鲜度；项目报告披露已读/未读范围、来源与不确定性；上下文读取不越出 workspace（X-FORBID-004）；1.0 主链不自动注入 AST/KB/TPL
open_issues: 上下文长度预算与裁剪策略归 AG G2；REF/AST/KB 的消费契约随其 G2 联合
-->

<!-- REQ
req_id: REQ-AG-005
title: 辅助续写与局部修改
owner_module: AG
status: active
summary: 承载辅助续写（Autocomplete）与局部修改（InlineAssist）业务回合（2026-06-11 归 AG；2026-07-04 澄清定义；2026-07-16 人类确认细化）：快捷键手动触发（防抖/防重）发起轻量模型调用（同过门禁）；新请求覆盖旧请求；辅助续写一次严格返回 3 个 50–100 字候选；局部修改支持选区/光标两态（无选区为光标插入）、弹窗内多轮、对话不持久化，多轮只传上一轮候选结果与新指令；候选经 ED 装饰层/overlay 语义呈现，视觉浮层必须贴近光标/选区视觉锚点并支持上下翻转，不得仅钳制到编辑器窗口边缘；浮层放置采用稳定静态尺寸模型，尺寸应贴近初始输入态/候选态的真实控件大小，不引入运行时测量反向驱动状态；编辑器缩放只影响视觉锚点坐标，不得压缩浮层控件尺寸，滚动/缩放/窗口尺寸变化时应重算浮层位置；采纳作为用户编辑操作进入编辑模型；采纳位置固定为触发瞬间捕获的文档/block/字符锚，不跟随采纳时光标；用户点击浮层外、移动光标或改变选区即视为放弃本次辅助回合，关闭浮层并丢弃已返回或后续返回内容
source_doc: 2026-06-11 人类归属决策（A-SYS-M-S-01 §3）、REQ-CORE-002/008、A-ED-M-T-01（ED-DECOR）
acceptance: 辅助续写可呈现 3 个候选且 Tab/1/2/3 采纳正确；候选不足 3 条时不展示半候选、不允许采纳，并保留轻量空态而非瞬间关闭；模型输出允许经解析容错收敛为 3 候选（纯 JSON 数组、fenced/多行 JSON、candidates 对象包装、编号/项目符号文本），但收敛后仍不足 3 条必须进入空态；gate/网关/空输出/解析不足/旧请求丢弃需记录内部 failure_reason 与隐私安全诊断，failure_reason 必须从后端结果穿透到前端 assist 状态或等价诊断点，不得在 service/editor 层被吞掉；普通用户界面仍不弹错误；局部修改选区态替换、光标态插入，多轮仅以上轮候选为基底；候选可经 ED 装饰层/overlay 语义呈现且采纳/忽略/继续输入复位正确；辅助续写词条优先贴近光标下方、空间不足翻到上方；局部修改弹窗有选区时按选区矩形居中优先上方、无选区时按光标插入点放置，空间不足再翻转；局部修改初始输入态不应因过大预估高度远离锚点；辅助续写触发后应立即显示生成中浮层；编辑器 50%–200% 缩放下浮层仍保持应用 UI 可用尺寸，位置跟随缩放后的光标/选区视觉锚点；滚动、缩放或窗口 resize 后浮层重新定位，目标结构失效才关闭；旧请求返回不展示不应用；用户点击浮层外、移动光标或改变选区后，辅助续写与局部修改均立即关闭，已返回内容丢弃，未返回请求的后续结果不得展示、不应用、不入历史；采纳前校验 DocRef/blockId/字符锚与触发前后文指纹；每次调用过门禁；无订阅/断网时静默不提供且不打断编辑
open_issues: 防抖具体数值属实现参数，默认 450ms；成本策略与延迟指标延 G5 标定
-->

<!-- REQ
req_id: REQ-AG-006
title: 能力模型独立配置
owner_module: AG
status: active
summary: 四类 agent 能力（辅助续写 Autocomplete / 局部修改 InlineAssist / 代理编辑 AgentEdit / 代理构建 BuildMode）各自可独立配置调用模型（provider/model），默认跟随全局默认模型、可显式指定覆盖；模型分预设（云端下发、订阅可调用）与 BYOK（用户自带密钥、网关代持）两类来源分层呈现与选择；模型配置的密钥语义仍受 X-FORBID-001 约束（客户端不持密钥）
source_doc: MODEL-CONFIG-FIX-PLAN-01（2026-08-06 人类四项确认）、A-GW-M-D-01（预设模型演进边界 GW-06）
acceptance: 四类能力在模型配置界面各自可选择「跟随全局」或指定独立模型；辅助续写/局部修改/代理编辑/代理构建四类调用按其能力偏好解析出 provider/model 生效；预设模型与 BYOK 模型在界面分层展示且来源可辨；密钥不落客户端、不因模型配置引入新密钥路径
open_issues: 预设模型清单云端下发契约（GET /models）与订阅档位映射归云 GW/LIC；能力偏好持久化并入统一设置体系后立项
-->

<!-- REQ
req_id: REQ-AG-007
title: Agent 运行时、意图与会话基础能力
owner_module: AG
status: active
summary: 提供可恢复、可观察的 Agent 运行时：仅 `agent-edit-1.0` 可启动，未知或缺失 profile 在 Gate、provider、工具和 Proposal 前失败关闭；每次用户发送形成独立 AgentTurn，澄清是等待用户输入的非终态，取消可在任一活动阶段生效且首个终态后迟到事件/工具无副作用。IntentResult 固定任务类型、目标、预期副作用、所需上下文、允许工具和确认点，并据此调用最小工具集合；AgentSession 是消息、任务事件、工具/DiffCard、引用与未保存上下文的展示、持久化和恢复隔离边界
source_doc: REQ-CORE-002/006/008/016、A-AG-M-D-02 §4.2（CAP-00）、§5.15~§5.17
acceptance: 缺失/未知 profile 不到达 Gate、provider、工具或 Proposal；每个 AgentTurn 的澄清等待、完成/取消/失败/拒绝互斥且不伪装完成，任一活动阶段取消和首终态后的迟到事件/工具均不破坏文档或卡片状态；IntentResult 驱动上下文和工具最小集，回答/分析回合不得获得写能力，写入目标或范围不明时只能澄清或生成目标计划；用户只见意图结论、读取/工具、Proposal/操作卡、等待确认和终态原因等结构化事件，不见模型内部推理；不同 workspace/AgentSession 不混入消息、活动回合、上下文、未保存引用或卡片事件，重启只恢复所属本地事实而不伪造成功；本要求不自动写入或注入长期项目记忆
open_issues: Runtime Profile 兼容迁移、AgentTurn 等待澄清/取消/终态状态机、IntentResult/CapabilityGrant/AgentSession/结构化事件/Trace 的具体字段、本地历史恢复与会话管理归 AG/SYS DESIGN Scope；本 L1 不定义接口、数据字段、Prompt、UI 或实现算法
-->

<!-- REQ
req_id: REQ-AG-008
title: 新建文档意图与 CreateDocumentBundle 协同
owner_module: AG
status: active
summary: 对明确的新建文档意图，AG 在确认前完成目标路径、资源计划和初始内容生成，形成 CreateDocumentBundle；DecisionRequest 确认后由 WS 重新核验并创建空的目标 DiskState，ED 打开标签，随后 DE/ED 以该磁盘基线生成并预览初始内容 Diff。资源创建授权与内容 Diff 裁决相互独立
source_doc: REQ-CORE-010、A-AG-M-D-02 §5.10、A-WS-M-T-02 §6
acceptance: 确认前不得创建文件；确认后先创建 DiskState 并自动打开标签，再生成初始内容 Diff；初始 Diff 接受只改变卡片生命周期，保存把 LogicalState 写入既有 DiskState；初始 Diff 拒绝仅在资源基线、DiskState 与 LogicalState 自创建以来均未变化时删除刚创建的文件并关闭标签，任一基线变化即使卡片失效且文件继续存在；创建或 Diff 投影失败不得伪报成功
open_issues: 无；对象、状态机和补偿条件由 A-WS-M-T-02 / A-WS-M-S-02 统一定义
-->

<!-- REQ
req_id: REQ-AG-009
title: 项目感知、编辑器事实与受限文档分析
owner_module: AG
status: active
summary: Agent 1.0 提供 P0 项目感知和阅读分析：先获取 workspace 结构概览，再按用户任务最小化读取正文，形成可复核的项目报告；同时消费当前 workspace、活动文档、打开标签、脏态、保存基准、任务所需选区和关联 Proposal。当前/已打开文档读取以 LogicalState 为准，未打开文档才按受限 workspace 读取 DiskState
source_doc: REQ-CORE-011、A-AG-M-D-02 §5.2~§5.5、CAP-02~04
acceptance: 项目报告列出已读/未读范围、来源和不确定性，不把目录列表当正文；文件定位返回候选而非编造存在；当前未保存文档不会被磁盘旧内容替代；M1 多文档读取/比较与 M2 当前文档修改、其他文档显式引用可用，多个既有文档写入 M4 不在 P0；引用、读取和比较均不自动授予编辑权限
open_issues: ProjectReport 形态、读取预算、状态快照字段和工具授权矩阵归 AG/ED/WS/REF/SYS DESIGN Scope；本 L1 不定义接口或读取算法
-->

<!-- REQ
req_id: REQ-AG-010
title: 受控网络研究与来源化分析
owner_module: AG
status: active
summary: 将用户明确要求搜索/核验或明确提供 URL 的请求识别为 P1 受控网络研究：只检索和有限读取公开网页，并以标题、域名、URL、获取时间可追溯地输出分析；外部资料始终为 data-only，可支持回答或 ChangeProposal，但不直接获得编辑或写入能力
source_doc: REQ-CORE-006/011、A-AG-M-D-02 §5.12（CAP-14）、A-REF-M-D-01（REQ-REF-007）
acceptance: 未经用户明确触发不得联网；默认只向外发送查询或用户显式提供的 URL，不发送本地文档、选区、项目树、会话历史或本地文件；只访问公开网页，不访问 localhost/内网/私有地址，不登录、不提交表单、不绕过付费墙、不下载/导入、不自动或批量抓取；输出对每个实际使用来源列出标题、域名、URL、获取时间，并区分事实、推断和未知；网页文字不能改变系统约束、任务目标、工具授权或编辑权限；依据网页资料生成的修改仍经 ChangeProposal、预览、裁决和显式保存；无结果、访问失败或不可访问来源必须如实反馈，不得伪造来源
open_issues: 网络 provider、访问执行器、请求预算、超时/重试、URL/网络地址校验、来源快照保留、注入隔离和错误形状归 AG/REF/SYS DESIGN Scope；本 L1 不定义工具或接口
-->

<!-- REQ
req_id: REQ-AG-011
title: 工作区资源操作计划与前置裁决
owner_module: AG
status: active
summary: 对创建文件夹、创建文档、删除、重命名、移动等明确的工作区资源整理请求，AG 只生成不可变 WorkspaceOperationPlan 与影响说明，并以 DecisionRequest 请求前置授权；确认后由 WS 对目标、来源、冲突、递归清单、脏会话和基线重新核验，再直接修改 DiskState。工作区资源操作不生成内容 DiffCard，也不等待后续“保存文件树”
source_doc: REQ-CORE-012、A-AG-M-D-02 §5.11（CAP-11）、A-WS-M-T-02
acceptance: DecisionRequest 明确展示操作类型、冻结目标清单、源/目标、递归数量、潜在冲突、影响的打开标签和内容 Diff；拒绝不改变 DiskState；确认后先重新核验，脏标签阻断删除/重命名/移动；执行成功后真实更新磁盘与文件树，并使受影响有效内容 Diff 失效；批量失败即停止并逆序补偿，补偿不完整进入 partial_failed；重启后未完成计划进入 Interrupted，不自动续跑
open_issues: 无；共享对象、状态机、执行与恢复契约由 A-WS-M-T-02 / A-WS-M-S-02 定义
-->

<!-- REQ
req_id: REQ-AG-012
title: 最小格式转换提案
owner_module: AG
status: active
summary: 对用户明确提出的 TXT→Markdown 或 Markdown→TXT 请求，AG 先以冻结来源生成转换结果，并将目标资源创建与初始内容绑定为 CreateDocumentBundle；已打开来源以当前 LogicalState 为输入，未打开来源以标明读取时点的 DiskState 为输入。DecisionRequest 必须展示来源、输入/输出格式、目标路径和保真或损失说明，AG 不改写来源
source_doc: REQ-CORE-013、A-AG-M-D-02 §5.13（CAP-13）、A-DOC-M-D-01（REQ-DOC-006）
acceptance: TXT→Markdown 不被表述为自动语义结构化；Markdown→TXT 在裁决信息中明示有损且不承诺样式、结构、表格、图片或可逆往返；确认后先创建真实目标 DiskState，再打开标签并投影转换结果初始内容 Diff；内容卡接受不重复创建文件，保存写入已存目标，目标继续编辑使旧卡失效且文件保留；生成/资源创建失败不伪称目标可用；HTML、通用 DOCX、批量或原地转换不产生工具授权，BDocx 不被作为通用转换入口
open_issues: 转换意图、格式能力开关、Proposal 载荷、来源快照、目标命名、错误形状、Trace 和工具/运行时授权归 AG/DE/DOC/ED/WS/SYS DESIGN Scope；本 L1 不定义工具或接口
-->

<!-- REQ
req_id: REQ-AG-013
title: 删除与恢复的前置裁决
owner_module: AG
status: active
summary: 对用户明确提出的已保存文件或文件夹删除/恢复请求，AG 生成包含冻结影响清单、递归数量、来源快照和恢复信息的 WorkspaceOperationPlan，并通过 DecisionRequest 取得前置授权；确认后 WS 重新核验并直接修改 DiskState，同时写入统一 WorkspaceTimeline。永久清理不属于 Agent 1.0
source_doc: REQ-CORE-014、A-AG-M-D-02 §5.11.1（CAP-12）、A-WS-M-T-02
acceptance: 决策卡明确显示目标、影响范围、递归数量、不可逆风险与可恢复来源；拒绝不改盘；确认后若范围、来源、冲突或脏会话不满足条件则阻断且不伪报成功；成功删除后相关有效内容 Diff 失效、标签关闭或按真实状态处理、后续引用明确失效；WorkspaceTimeline 可按类型恢复资源且恢复冲突不自动覆盖；CreateDocumentBundle 初始 Diff 的安全拒绝只按 REQ-AG-008 的专用补偿条件执行
open_issues: 无；删除、恢复与 WorkspaceTimeline 由 A-WS-M-T-02 / A-WS-M-S-02 统一定义
-->

<!-- REQ
req_id: REQ-AG-014
title: 多个既有文档的目标计划与逐项 Proposal
owner_module: AG
status: active
summary: 对多个既有文档的修改请求，AG 必须先呈现候选或用户显式选择形成的目标计划，待用户确认后冻结目标成员、路径和身份；随后只对 TXT、Markdown、HTML 与 BDocx 编辑副本逐目标生成独立 Proposal。每个目标的读取状态、基准、预览、裁决、保存和失败独立，AG 不把未声明文档或后续搜索结果加入目标集
source_doc: REQ-CORE-015、A-AG-M-D-02 §5.14（CAP-15）、A-DE-M-D-01（REQ-DE-008）
acceptance: 目标计划在任何既有文档写入前可见且需确认；原生 DOCX、删除/文件树整理/格式转换混合操作不生成本能力的写入授权；每个目标遵循其自身 LogicalState/DiskState 分流并具独立卡片；某一目标冲突、外部变化、结构变化或删除时只报告并失效该目标，不静默跳过或影响其他目标；没有批量接受、批量保存、跨文档全有或全无承诺
open_issues: 目标计划卡、用户选择/候选确认交互、逐目标工具轮次、并发/顺序、独立基准和 Trace/恢复契约归 AG/DE/ED/WS/REF/SYS DESIGN Scope；本 L1 不定义跨文档 API 或事务
-->

<!-- REQ
req_id: REQ-AG-015
title: 受限复合任务编排与真实工作状态
owner_module: AG
status: active
summary: AG 将 IntentResult 作为任务编排入口：简单目标可由单个 AgentTurn 完成，复杂目标形成受限任务实例（TaskRun），在同一用户目标内组织多个 AgentTurn 和有依赖的读取、研究、分析、生成或 Proposal 步骤。每一步按当前 profile、意图、目标、上下文、允许工具和确认点重新收窄；TaskRun 不是后台队列，不预授权后续工具、不自动重试、不跨重启续跑。明确的同文档复合目标可连续生成 Proposal，但每张后续卡只以最新 LogicalState 为基准，前卡因基准推进失效；用户只见真实的当前任务/步骤状态，不见内部推理
source_doc: REQ-CORE-017、REQ-CORE-016、A-AG-M-D-02 §5.15（CAP-16）、A-DE-M-D-01（REQ-DE-009）
acceptance: 创建→内容→排版、研究→大纲→内容等复合任务有可见目标、当前步骤、实际读取/工具、等待点和终态；同一 AgentSession 至多一个执行中 AgentTurn，等待补充或卡片裁决不伪装为工作中且不持有模型/工具执行；每一步仅获当前最小授权，多既有文档目标计划、删除、结构保存和其他既有确认点照常阻断；同文档 Diff2/3 读取成功预览后的最新 LogicalState，成功预览后前卡失效且不可再接受/拒绝，拒绝最新卡仅按其自身基准处理；背景 pending 卡不以 payload 续编，必须先迁移为 LogicalState 或完成背景分流；取消只停止未开始步骤并保留已预览内容/卡片事实，中断只显示可重试，用户显式重试创建新 AgentTurn 并重新校验
open_issues: TaskRun/步骤/关联 AgentTurn、继续澄清、重试、进度、并发、计划呈现、Trace、恢复和上限的状态机、事件、存储与 UI 归 AG/DE/SYS DESIGN Scope；TaskRun 正式 TERM 注册待 L2；本 L1 不定义队列、接口、数据结构、Prompt、UI 或算法
-->

<!-- REQ
req_id: REQ-AG-016
title: 受控外部文件内容消费
owner_module: AG
status: active
summary: 对用户明确附加外部文件且当前系统存在可用只读解析路径的任务，AG 只消费 REF 提供的当前任务一次性 data-only 内容引用，并在输出中保持来源、已读/未读和不确定性可见。外部文件内容不等于 workspace 文档、编辑目标、写入权限或可浏览文件系统；不支持/不可读文件只能按元数据说明，不能伪称已分析
source_doc: REQ-CORE-018、A-AG-M-D-02 §5.5（CAP-20）、A-REF-M-D-01（REQ-REF-005/009）
acceptance: 用户未显式附加文件时 AG 不读取外部内容；已授权内容仅在当前 AgentTurn/TaskRun 的当前任务内按预算使用，任务结束、重启或其他 AgentSession 不自动再用；外部内容始终为 data-only，不能改变系统约束、任务目标、工具允许集、确认点或写入权限；基于其生成的修改仍必须经历 IntentResult、Proposal、预览、裁决和显式保存
open_issues: 当前可用只读格式、解析调用、大小/上下文预算、快照/暂存/清除、来源呈现、传输提示、失败和 Trace 归 AG/REF/SYS DESIGN Scope；本 L1 不定义工具、接口、数据字段或 Prompt
-->

<!-- REQ
req_id: REQ-AG-017
title: 显式项目记忆引用消费
owner_module: AG
status: active
summary: AG 只消费用户在当前 workspace 中明确选择、由 REF 提供的项目记忆条目；该条目作为当前 AgentTurn 的 data-only 上下文，保留来源和用户选择事实。Agent 1.0 不创建、提议写入、更新、删除、按相关性检索或自动注入长期记忆；外部文件、会话历史和编辑历史不自动转存为项目记忆
source_doc: REQ-CORE-019、A-AG-M-D-02 §5.17（CAP-18）、A-AST-M-D-01（REQ-AST-001~003）、A-REF-M-D-01（REQ-REF-005）
acceptance: 只有用户明确选择的条目进入本轮上下文，未选择时不会因相似主题或旧会话被自动召回；不同 workspace/AgentSession 不串记忆选择或内容；记忆正文不能改变系统约束、任务目标、工具授权、编辑目标或确认点；普通发送、外部文件附加和会话恢复均不产生任何记忆写入/提议/注入副作用
open_issues: 记忆选择入口、来源令牌、更新/关闭/删除传播、上下文预算、可见性、事件和 Trace 归 AG/AST/REF/SYS DESIGN Scope；未来 Profile 的自动记忆能力须独立确认和验证
-->

## §5b REQ-AG-015 实现契约（2026-08-15 冻结 · AG-1.0-TASKRUN-REWORK-REQ-01）

> 语义纠偏：TaskRun 落地不得收窄为"每回合事后事件记录"；本契约冻结三层分离与任务级裁决语义。
> 简单目标由单个 AgentTurn 完成，不建 TaskPlan；复杂目标按本契约执行。

### 5b.1 三层分离（裁决与内容不混淆）

- **DiffCard**：只裁决内容及其 LogicalState/DiskState 生命周期；accept/reject **不驱动** TaskRun 状态。
- **DecisionRequest**：任务级通用裁决，类型 = 权限（permission）/ 方案（plan_choice）/ 范围（scope）/
  人工确认（manual_confirm）；生命周期 = proposed → resolved（confirm / cancel）；裁决结果 → TaskRun 事件
  （UserConfirm / UserCancel）。
- **TaskRun**：只响应明确任务裁决；不因 Diff 裁决或任意新消息隐式推进。

### 5b.2 TaskPlan（可展示操作计划，非模型思维链）

- IntentResult 判定复杂目标后创建；保存：步骤列表、步骤依赖、当前游标、所需授权、确认点标记、阶段结果；
- 简单目标单 AgentTurn 直行。

### 5b.3 执行语义

- 一个 TaskRun 可跨多个执行片段/AgentTurn；每个片段/步骤按
  RuntimeProfile + IntentResult + CapabilityGrant + 当前目标 + 当前上下文 + 既有确认点重新收窄（每步重新授权）。
- 任务级确认（DecisionRequest confirm）后自动开始下一执行片段：无需用户再输入聊天消息；绝不后台续跑。
- 确认点不伪装为工作中、不持有模型/工具执行（REQ-AG-015 acceptance）。

### 5b.4 新消息 gate

- 存在未决 DecisionRequest 时，任意新聊天消息不得绕过待确认状态（新回合入口拒绝或仅接受带明确裁决语义的输入）；
  澄清（clarification）走既有路径不受此 gate 影响。

### 5b.5 同文档连续 Diff

- 同一执行片段内按计划可连续生成多张 Proposal；Diff2 必须基于 Diff1 成功预览后的最新 LogicalState；
  Diff1 因基准推进失效，不可再接受/拒绝，不形成补丁栈。

## §6 模块边界与约束（边界闭合）

- **云薄端硬边界**（APC §4.3/§13）：AG 不持密钥、不内置 provider SDK、不绕过订阅门禁；一切模型
  调用经 SYS I-P-03 契约（CLOUD-AUTH/GATE/INVOKE）。
- **唯一落盘路径**：AG 产出的文档内容修改必经 DE/ED 提案预览与人类裁决 → 用户显式 save → WS 落盘；工具调用不得旁路直写内容。
  工作区资源整理不是内容写入：AG 只生成 WorkspaceOperationPlan 和 DecisionRequest，确认后由 WS 重新核验并直接修改 DiskState，不生成资源 DiffCard，也不存在后置“保存文件树”。新建文档使用 CreateDocumentBundle，资源创建完成后初始内容仍走普通 Diff 生命周期。多既有文档编辑仍先确认冻结目标清单并逐目标执行。
- **职责边界**：代理回合归 AG，项目级构建编排归 CBT（构建移交/回落，BR-CBT-STATE-002）；解析归 REF、存储归
  AST/KB、呈现归 ED。
- **状态机边界**（APC §6）：代理回合、tool-call、autocomplete、inline-assist 四条状态化
  核心链均 owner=AG（总图 §3），AG G2 在其 M-S 定义并回填。
- **运行时授权与可见性边界**：只有 `agent-edit-1.0` 可进入 1.0 回合；profile、IntentResult、CapabilityGrant 与执行校验逐层收窄，缺失/未知 profile 不能以兼容默认值、前端参数、已注册工具或 Prompt 扩权。用户只见结构化任务事件，不见模型内部推理；项目记忆仅在用户明确选择时作为 data-only 引用进入，绝不自动写入、提议、检索或注入。
- **复合任务边界**：TaskRun 只组织当前用户目标内的受限步骤，不是后台队列或任务级事务。后续同文档 Proposal 必须以当前 LogicalState 接力，前卡基准推进失效；不得以旧卡 payload 叠加、跨确认点续跑或在取消后自动回滚已预览内容。
- **L4 原料边界**：取材台账 §4 相关 AG/对话原料（P1/P2），原料不为权威。
- **外部研究边界**：只有 REQ-AG-010 的显式用户触发可产生公开网页读取；AG 消费外部资料时保持 data-only，来源令牌由 REF 供给，REF 不发起网络请求。
- **外部文件边界**：REQ-AG-016 的外部内容只来自用户显式附加且可用只读解析的当前任务引用；AG 不扫描目录、不自动复读、不导入/编辑外部文件，也不把它默认沉淀到会话历史或项目记忆。

## §7 G1 问题暴露清单

| 编号 | 分类 | 状态 | 处置 |
|---|---|---|---|
| AG-01 | BOUNDARY_OPEN | 已关闭 | 对话/回合历史持久化模型已回写 AG G2 与 SYS：app data `workspace.db`、全状态保留、标签元数据全恢复、内容懒加载、200MB 软归档 |
| AG-02 | DESIGN_RISK | 显式延期 | 工具注册清单与权限边界（写盘类工具必须走 diff 路径的强制机制）归 AG G2，与 DE/WS 联合 |
| AG-03 | DESIGN_RISK | 显式延期 | autocomplete 触发节流与调用成本（高频触发 × 门禁/网关延迟与计量）归 AG G2 设计 |
| AG-04 | BOUNDARY_OPEN | 显式延期 | 构建移交/回落边界（与 CBT 的状态机对接 X-INTER-001）联合 CBT G2 |
| AG-05 | ACCEPTANCE_MISSING | 部分关闭 | 定性验收已入 §5；量化指标（首 token 延迟、补全延迟）延 G5 NFR 标定 |
| AG-06 | FLOW_INCOMPLETE | 已识别，待 G2 | `agent-edit-1.0` 强制入参迁移、AgentTurn 的等待澄清/全活动态取消/首终态锁存、IntentResult/CapabilityGrant 最小授权、非推理可见事件、AgentSession 持久化隔离与恢复需按 REQ-AG-007 建模，并与 SYS 事件/持久化契约对齐；当前缺 profile 默认、澄清终结、受限取消和 workspace 级恢复均为迁移债务。 |
| AG-07 | FLOW_INCOMPLETE | 已识别，待实现 | CreateDocumentBundle 的确认、先建盘、初始内容 Diff 与安全拒绝补偿需按 REQ-AG-008 和 A-WS-M-T-02 / M-S-02 实现，不得复用旧“未保存逻辑新文档”链 |
| AG-08 | DESIGN_RISK | 已识别，待 G2 | 受控网络研究的 provider、请求预算、公开地址校验、来源快照/失败形状与注入隔离需按 REQ-AG-010 联合 REF/SYS 设计；不得用通用浏览器或爬虫绕过此边界 |
| AG-09 | IMPLEMENTATION_GAP | 设计已闭合，待实现 | WorkspaceOperationPlan、DecisionRequest 前置裁决、确认时核验、直接 DiskState 执行、脏标签阻断和内容 Diff 失效须按 A-WS-M-T-02 / M-S-02 迁移；不得保留资源 DiffCard 或后置保存链 |
| AG-10 | DESIGN_RISK | 已识别，待 G2 | TXT↔Markdown 转换的意图授权、来源快照、目标副本 Proposal、保真/损失告知和现有转换工具的迁移，须按 REQ-AG-012 联合 DE/DOC/ED/WS/SYS 设计；不得由通用文件工具或 BDocx 路径绕过 |
| AG-11 | IMPLEMENTATION_GAP | 设计已闭合，待实现 | 删除/恢复须迁移到 WorkspaceOperationPlan 前置裁决与 WorkspaceTimeline 类型化恢复；现有资源 DiffCard 和不完整删除快照均不得作为最终实现。 |
| AG-12 | DESIGN_RISK | 已识别，待 G2 | 多既有文档的目标计划确认/冻结、逐目标授权、独立 Proposal/基准/失效/保存与现有宽松 `cross_doc` 的迁移，须按 REQ-AG-014 联合 DE/ED/WS/REF/SYS 设计；不得动态扩展目标或引入批量裁决/事务。 |
| AG-13 | FLOW_INCOMPLETE | 已识别，待 G2 | REQ-AG-015 的 TaskRun/步骤、同文档连续 Proposal、真实工作状态、继续澄清、取消后保留预览、中断/新回合重试、并发与现有 L2 的澄清终结/原回合 Retry 迁移，须联合 DE/SYS 设计；不得用后台队列、自动重试或聊天文案替代。 |
| AG-14 | DESIGN_RISK | 已识别，待 G2 | REQ-AG-016/017 的外部内容一次性消费、可读格式判定、快照/清除、来源/失败呈现和用户选择记忆的消费/隔离，须联合 REF/AST/SYS 设计；不得从附件、历史或相似主题自动读取、写入或注入项目记忆。 |

> 术语：回合、工具调用、局部修改等 AG 主责核心概念在 AG L2 评估注册 TERM；订阅门禁/AI网关/会话
> 令牌/BYOK透传已由 SYS 注册（TERM-CLOUD-001~004），AG 引用不重注册。

## §8 后续动作

1. 待人类宣布 AG 模块级 G1 闭合后进入 AG G2：工具调用契约
   与权限边界（解除 AG-02）、局部修改触发与成本策略（解除 AG-03）、四条核心链状态机（M-S，回填
   总图 planned 行）、与 DE/CBT/REF 的对接契约。
2. G2 经 REQ_MAP 把 REQ-AG-001~017 映射为 RULE/CHAIN/CONSTRAINT/TERM。
3. DE G1/G2 启动时与 AG 对接「AI 产出→diff 生成」契约（P3 内联动）。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-11 | v1.0 | 初版（P3 首件）：AG 对话代理模块级 L1；声明 REQ-AG-001~005（单文档 chat 回合 / 工具调用回合 / 门禁网关前置 / 上下文组装 / 内联辅助与自动补全），承接 REQ-CORE-001/002/006/008/009 与 2026-06-11 人类归属决策（inline-assist/autocomplete 归 AG）；代表性流程 A1–A3；云薄端硬边界与唯一落盘路径锁定；G1 暴露清单 AG-01~05 |
| 2026-07-04 | v2.0 | 术语沿革澄清回写（人类裁决）：REQ-AG-001 更名「代理编辑回合（编辑模式）」、去单文档限定（默认当前文档+显式指向可跨/新建）；REQ-AG-005 更名「辅助续写与局部修改」（手动触发/防抖、局部修改多轮不持久化、续写单候选 50–100 字）；A-J1/A-J5 职责与边界澄清同步（文件树整理直接执行·删除须确认；构建移交判据=项目级构建意图，原对话窗口实现）；模块正式名「代理（Agent）」 |
| 2026-07-16 | v2.1 | 细化 REQ-AG-005：辅助续写改为 3 候选；新请求覆盖旧请求；局部修改无选区为光标插入，多轮只传上一轮候选结果与新指令；候选呈现按 ED 装饰层/overlay 语义归口，采纳经用户编辑路径 |
| 2026-07-16 | v2.2 | 补齐 REQ-AG-002 文件整理验收：移动/重命名/建目录属文件树结构操作，经 WS 边界工具直接执行；文档内容修改仍经 DE diff；删除须等待独立用户确认链路，不得由模型参数伪确认 |
| 2026-07-16 | v2.3 | 续修 REQ-AG-005 验收：辅助续写严格 3 候选、采纳位置固定为触发锚、采纳前校验前后文指纹，防抖默认 450ms，候选不足静默关闭 |
| 2026-07-16 | v2.4 | 补充 REQ-AG-005 浮层定位验收：视觉浮层须贴近光标/选区视觉锚点，辅助续写优先下方、局部修改优先选区上方并支持空间不足翻转 |
| 2026-07-16 | v2.5 | 补充 REQ-AG-005 交互锚生命周期：用户点击浮层外、移动光标或改变选区即关闭辅助续写/局部修改浮层，丢弃已返回或后续返回内容，不展示、不应用、不入历史 |
| 2026-07-17 | v2.6 | 关闭 AG-01：代理编辑历史持久化模型已在 AG G2/SYS 回写，锁定 app data workspace.db、全状态保留、标签元数据全恢复、内容懒加载、200MB 软归档 |
| 2026-07-16 | v2.6 | 补充 REQ-AG-005 缩放兼容验收：编辑器缩放只影响视觉锚点坐标，不压缩浮层 UI 尺寸；滚动、缩放、窗口 resize 后重算浮层位置 |
| 2026-07-16 | v2.7 | 修正 REQ-AG-005 稳定定位验收：浮层放置采用静态尺寸模型，不使用运行时 DOM 测量反向驱动状态；局部修改初始态避免过大高度预估，辅助续写触发后必须稳定显示生成中浮层 |
| 2026-07-16 | v2.8 | 修正 REQ-AG-005 辅助续写失败态：候选不足 3 条时不展示半候选、不允许采纳，但保留轻量空态，避免触发后浮层瞬间消失 |
| 2026-07-17 | v2.9 | 加固 REQ-AG-005 辅助续写输出契约：保持 3 候选产品要求，补充 JSON/编号文本解析容错、failure_reason 内部诊断与隐私安全日志要求 |
| 2026-07-17 | v2.10 | 收敛 AG-ASSIST 诊断链：failure_reason 必须穿透后端、service、assist 状态与空态诊断点，不得被统一“暂无可用续写”吞没 |
| 2026-07-28 | v2.11 | BDocx TableCellShading：REQ-AG-002 纳入 `list_tables→table_ops` 表格语义链，以及单元格底纹的严格颜色、合并锚、动态能力与基准校验验收。 |
| 2026-07-29 | v2.12 | BDOCX-IMAGE-AGENT-EDIT-01：REQ-AG-002 纳入 `list_images→image_ops`、活动 BDocx 基线、Block/image/source hash 校验、结构化 Diff 和项目源文件保护。 |
| 2026-07-30 | v2.13 | 图片代理编辑闭合审计：REQ-AG-002 同步 manifest 1.4.0，补齐缩放/移动/对齐组合、import_image_asset、edit_blocked 和非视觉识别边界。 |
| 2026-08-06 | v2.14 | MODEL-CONFIG-01：新增 REQ-AG-006 能力模型独立配置（四类 agent 能力各自可独立配置模型；预设/BYOK 来源分层；密钥语义不变） |
| 2026-08-07 | v2.15 | MODEL-CONFIG-TERM-01：REQ-AG-006 中「构建 BuildMode」中文称呼同步为「代理构建 BuildMode」（TERM-AG-007 zh 更名） |
| 2026-08-08 | v2.16 | 定义 AG 1.0 EP 能力基线：收敛为当前文档优先、用户持续裁决的文档编辑 Agent；明确发布阻断/可选/延期能力；REQ-AG-002 增本轮最小工具允许集，REQ-AG-003 锁真实服务端 GatePassed，REQ-AG-004 增结构化本轮意图与 data-only 上下文边界 |
| 2026-08-13 | v2.17 | 依据 1.0 能力基线与 REQ-CORE-010，同步为“受控预览进入 LogicalState→裁决提案→显式保存”链路；新增 REQ-AG-007（Agent 运行时/意图/会话隔离）和 REQ-AG-008（新建文档意图与未保存逻辑状态协同），并把 AST/KB/TPL 自动注入排除在 1.0 主链外。 |
| 2026-08-13 | v2.18 | AG-1.0-R2-REQUIREMENTS-01：CAP-01~04 进入 P0；扩展 REQ-AG-001/004 的回答、分析、终态与两阶段读取边界，并新增 REQ-AG-009 承接项目报告、编辑器事实、LogicalState/DiskState 新鲜度及 M1/M2/M4 多文档范围。 |
| 2026-08-13 | v2.19 | AG-1.0-R3-REQUIREMENTS-01：原生 DOCX 限制为只读分析，编辑意图转为创建 BDocx 编辑副本 Proposal；表格/图片工具限定 HTML 编辑文档与 BDocx 处理副本，原生 DOCX 不获得写工具。 |
| 2026-08-13 | v2.20 | R3 自检收紧：当前文档编辑仅承诺内容与基础结构；HTML/BDocx 的具体样式修改必须等待样式矩阵确认。 |
| 2026-08-14 | v2.21 | AG-1.0-R3-STYLE-REQUIREMENTS-01：HTML/BDocx 样式矩阵进入 AG L1；字符基础样式 P0，颜色/突出显示、段落样式、字号与字体族 P1，字体族含 BDocx；验证为实现后开放门槛。 |
| 2026-08-14 | v2.22 | AG-1.0-R4-NETWORK-REQUIREMENTS-01：新增 REQ-AG-010，固定 CAP-14 为用户显式触发的 P1 受控公开网页研究；默认仅发送查询或用户提供的 URL，外部资料为 data-only 且来源可追溯，爬虫、认证/交互、下载/导入和内网访问继续排除。 |
| 2026-08-14 | v2.23 | AG-1.0-R4-STRUCTURE-REQUIREMENTS-01：新增 REQ-AG-011；建目录、单文件重命名/移动改为结构 Proposal→逻辑文件树→逐项显式保存，接受不再直写；现有直接结构工具列为后续迁移债务。 |
| 2026-08-14 | v2.24 | AG-1.0-R4-CONVERSION-REQUIREMENTS-01：新增 REQ-AG-012；仅 TXT→Markdown、Markdown→TXT 可生成目标副本转换 Proposal，来源不改写、目标预览后走新建逻辑文档与显式保存链；Markdown→TXT 明示有损，HTML 与通用 DOCX 转换不授予工具。 |
| 2026-08-14 | v2.25 | AG-1.0-R4-DELETE-MULTIDOC-REQUIREMENTS-01：新增 REQ-AG-013/014；删除/恢复只生成红色逻辑暂存与可恢复 Proposal，接受不删除、显式保存才执行且无永久删除；多个既有文档先确认冻结目标清单，再逐项 Proposal/裁决/保存，不提供批量接受、批量保存或跨文档事务。 |
| 2026-08-14 | v2.26 | AG-1.0-R5A-RUNTIME-REQUIREMENTS-01：扩展 REQ-AG-007，承接 REQ-CORE-016 的唯一 profile、澄清等待、全活动态取消/首终态锁存、IntentResult 最小授权、非推理可见事件、AgentSession 隔离和本地事实历史；当前运行时差异只登记为 DESIGN/实现迁移债务。 |
| 2026-08-14 | v2.27 | AG-1.0-R5B-TASK-ORCHESTRATION-REQUIREMENTS-01：新增 REQ-AG-015，承接受限 TaskRun、复杂目标多步骤/多 AgentTurn、真实工作状态、同文档连续 Proposal 的 LogicalState 接力、前卡失效、背景 pending 限制、取消与中断新回合重试；队列/自动续跑/事务均明确排除。 |
| 2026-08-14 | v2.28 | AG-1.0-R5C-MEMORY-EXTERNAL-READ-REQUIREMENTS-01：新增 REQ-AG-016/017；外部文件仅在用户显式附加且有可用只读解析路径时作为当前任务 data-only 内容消费，项目记忆仅由用户显式选择引用；Agent 1.0 不自动读盘、复读、写入、提议、检索或注入长期记忆。 |
| 2026-08-21 | v2.29 | 工作区资源操作基线重构：REQ-AG-002/008/011/013 改为 WorkspaceOperationPlan + DecisionRequest 前置裁决、确认后 WS 直接修改 DiskState；新建文档采用 CreateDocumentBundle，资源创建后初始内容继续遵循 Diff 生命周期；统一恢复入口为 WorkspaceTimeline。 |
