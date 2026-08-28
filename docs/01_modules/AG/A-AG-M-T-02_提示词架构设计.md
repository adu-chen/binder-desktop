---
文档编号：AG-M-T-02
文档状态：A
负责模块：AG
文档职责：AG 提示词架构 L3 专项设计——定义分层装配、当前轮意图、数据可信边界、上下文截断与 1.0 Prompt Runtime 最小契约
上游约束：A-AG-M-T-01_代理技术设计.md（§5/§5.1 BR-AG-DATA-001/002 组装骨架与来源裁剪、AG-TURN 链路、TERM-AG-001~004）、A-CORE-C-P-01_APC.md（云薄端、§13 不绕门禁）、A-SYS-I-P-03_云接口契约.md（X-SEQ-001 门禁前置）
直接承接：本文 RULE/TERM 为提示词架构 SSOT，PI-AG-07~09（A-AG-X-L-02）承接实现；调研依据见 R-AG-X-R-01（仅参考，不约束）
使用边界：本文是 M-T-01 §5.1 的 L3 细化，不重定义组装骨架（归 BR-AG-DATA-002）与来源裁剪语义（归 BR-AG-DATA-001）；不锁定提示词文本/措辞/模型档（实现期运营配置）；不定义门禁/网关契约（归 SYS I-P-03）、diff 生成（归 DE）、@mention 解析（归 REF）；§6 结构示例为非规范说明，不构成被锁文本
变更要求：新增/调整提示词层或截断序须同步 BR-AG-DATA-004/005 并回写 M-T-01 §5.1 交叉引用；TERM-AG-005 一经 term_ref 引用其 en 不可更改；落地后须 npm run governance:generate 注册
治理成熟度：draft
---

# Binder 提示词架构设计（AG · L3 专项设计）

## 0. Terminology Registry

> 仅注册本文新增的核心概念。提示词分层/装配包沿用 M-T-01 §5.1「组装骨架」既有表述，
> 不另立 TERM 以免与已锁措辞冲突。

<!-- TERM
term_id: TERM-AG-005
chains: AG-TURN
zh: 意图沿用判定
en: IntentCarryover
forbidden: 历史复用, 意图继承, 上下文继承, 意图沿袭, carryover
-->
在代理回合上下文组装前，对当前轮用户输入与历史轮意图关系做出的结构化判定：识别本轮是否显式
沿用/调用/选择了历史轮意图（指代回指 / 显式选择 / 任务延续 / 修正追加），输出沿用目标，决定
历史意图是否提权进入当前轮上下文。默认判定为「无沿用」，历史不进当前轮事实层（BR-AG-DATA-004）。

## §1 文档定位

```
M-T-01 §5.1（组装骨架 SSOT：BR-AG-DATA-002 分层/顺序、BR-AG-DATA-001 来源/裁剪）
   │ 深度拆解（L3）
   ▼
本文（提示词架构）：多层清单 + 装配顺序/缓存边界 + 当前轮次原则 + 补强层隔离/截断序 + 结构示例
   │ 新增锁：BR-AG-DATA-004（当前轮次原则与意图沿用）、BR-AG-DATA-005（补强层隔离与分层截断序）
   ▼
PI-AG-07~09（A-AG-X-L-02）承接实现；R-AG-X-R-01 提供调研依据（仅参考）
```

**与 M-T-01 的边界（职责单一化，去重）**：M-T-01 §5.1 锁「四层组装骨架（分层 + 注入顺序 + 三类
回合差异）」（BR-AG-DATA-002）与「来源限定 + 确定性裁剪 + 显式注入优先保留」原则（BR-AG-DATA-001）。
本文**不重定义骨架与来源原则**，只做两件 M-T-01 未覆盖的事：
1. 把四层骨架细化为面向 binder 文档场景的**完整分层清单**（含引用/记忆/知识/模板/展示等特有层），
   每层标注来源模块与已有规则映射（§2）；
2. 补 M-T-01 缺失的两个机制：**当前轮次原则 / 意图沿用判定**（§4，BR-AG-DATA-004）与
   **补强层污染隔离 / 分层截断序**（§5，BR-AG-DATA-005）。

**版本优先（迭代声明）**：本文是提示词设计的迭代版本。**分层截断序以本文 BR-AG-DATA-005 为唯一
权威**——M-T-01 §5.1 早期把当前文档排在裁剪优先级最低位的旧排序「显式 @mention > 记忆/知识 >
当前文档」，与当前轮次原则（当前文档为当前轮主事实，§4）冲突，**已废止**，统一以本文「当前文档 P0
永不截、显式 @mention/引用 P1、记忆/知识/模板 P2 先截」为准。M-T-01 v1.2 已同步移除该旧排序并委派
本文，**不存在两处规则各持一套截断序的逻辑重复**。

**云薄端前提重申**：提示词在客户端装配，但模型调用经门禁→网关（X-SEQ-001）；本文不涉调用链。

## §2 提示词分层总览（M-T-01 §5.1 骨架细化）

> M-T-01 四层骨架（系统提示词层→工具/能力定义层→上下文注入层→用户输入层）在 binder 文档场景下
> 细化为下表。**本表为设计映射，各层文本属实现期运营配置（不入规则）**；分层与顺序承 BR-AG-DATA-002，
> 来源与裁剪承 BR-AG-DATA-001。

| 层 | 对应 M-T-01 骨架层 | 内容 | 来源模块 | 截断优先级 | 已有规则映射 |
|---|---|---|---|---|---|
| 治理/系统层 | 系统提示词层 | 角色定位 + 硬约束边界（修改经 diff 裁决、不越权落盘、workspace 限界） | AG | P0 永不截 | BR-AG-STATE-001 / X-FORBID-006 |
| 工具/能力层 | 工具/能力定义层 | 工具清单与契约（AG-TOOL 注册项投影），按模式启停 | AG | P0 | BR-AG-STATE-002 |
| 格式层 | 系统提示词层（子项） | 输出格式约束（**GFM Markdown 子集**：段落/标题/列表/加粗/围栏代码块标注语言/表格；**声明当前无图表/流程图（Mermaid）渲染能力**；HTML/docx 仅面向 BDocx 能力投影） | DOC/DE | P0 | BR-AG-DATA-002（骨架内）、BR-AG-UI-001 |
| 任务状态层 | 上下文注入层 | 当前 Agent 任务 + 阶段 + artifacts（仅 active） | AG | P1 | BR-AG-STATE-004 |
| 当前文档/编辑器层 | 上下文注入层 | 当前文件 + 分级块列表 + 选区/光标 + revision/baseline + pending diffs | WS/ED/DE | **P0 当前轮主事实** | BR-AG-DATA-001 |
| 引用层 | 上下文注入层 | 显式 @mention（精确锚点可编辑 / 阅读上下文仅理解） | REF | P1 最高保留 | BR-AG-DATA-001（@mention 优先） |
| 当前对话层 | 用户输入层 | 当前轮 user 输入 + IntentResult（结构化决策不替代原文） | AG | P0 信号源 | BR-AG-DATA-004（本文） |
| 历史对话层 | （由 messages[] 承载） | 既往轮次，默认弱化 | AG | P1/弱 | BR-AG-DATA-004（本文） |
| 记忆补强层 | 上下文注入层 | 长期记忆检索结果，带权重声明 | AST | P2 先截 | BR-AG-DATA-005（本文） |
| 知识补强层 | 上下文注入层 | 知识库自动检索切片（带 citation/provenance），经触发门控 | KB | P2 先截 | BR-AG-DATA-005（本文） |
| 模板补强层 | 上下文注入层 | 编排/工作流约束引用（作为约束而非内容） | TPL | P2 先截 | BR-AG-DATA-005（本文） |
| 工作区依赖层 | 上下文注入层 | 文件依赖图、pending diff 状态 | WS | P3 按需 | BR-AG-DATA-005（本文） |
| 展示层（输出侧） | —（非输入层） | 流式 chunk / 工具卡片 / diff 卡片，与模型输入解耦 | AG/前端 | — | 归 DE/前端，见 R-AG-X-R-01 §6 |

**分层准则**：当前轮主事实（当前文档/当前对话）与显式引用为 P0/P1 高保留；记忆/知识/模板为 P2
补强层，整体低于当前轮事实；工作区依赖为 P3 按需。展示层是输出侧，不进模型输入（隔离）。

## §3 装配顺序与缓存边界布局

> 承 BR-AG-DATA-002 注入顺序，本节补**物理布局**设计（缓存/recency 优化），不改顺序语义。

```
[稳定区·可进 prompt cache]
  治理/系统层 → 工具/能力层 → 格式层
        ──────── 缓存边界标记 ────────
[会话特定区·每轮易变]
  任务状态层 → 补强层(记忆/知识/模板，带权重声明) → 当前文档/编辑器层 → 引用层
  ───────────────────────────────────────────────
[messages[] 数组·与 system 分离]
  历史对话层(默认弱化) … 当前对话层(末条，含意图沿用结果)
```

1.0 必须落地统一 Prompt Runtime 和 `runtime_version`，但 provider prompt cache 命中率不是 1.0 发布
阻断项：没有可靠缓存时允许每轮完整装配，不能为了缓存复用而混淆系统规则、文档数据和用户输入。
稳定区、动态区、工具 manifest hash、IntentResult id 与 Context trace id 必须可观测，以支持同一回合复现。

**两条布局设计依据**（调研支撑见 R-AG-X-R-01 §3.1/§3.4）：
1. **稳定层在前并进缓存边界**：角色/工具/格式跨轮不变，置于缓存边界前以复用 prompt cache。
2. **当前轮易变事实靠近末尾**：当前文档、当前对话置于 prompt 末尾吃 recency；补强层虽在中部
   （接受其低优先级代价），但以显式权重声明（§5）抵消「中间迷失」的语义稀释。

## §4 当前轮次原则与意图沿用判定

> M-T-01 §5.1 只锁注入顺序，未规定历史轮的处置。本节补此机制。

**原则**：当前轮用户输入是本轮主需求（信号源）。历史轮、记忆、知识、模板默认为弱上下文，不得
自动凌驾当前轮事实判断。只有意图沿用判定（IntentCarryover）识别出本轮显式沿用历史意图时，才把
对应历史意图提权重述到当前轮邻近位置——而非整段堆叠历史。

**意图沿用判定（设计层，维度示意，阈值实现期）**：

| 维度 | 触发示意 | 沿用输出 |
|---|---|---|
| 指代回指 | 「继续」「接着上面」「它/那个」、无主语祈使 | 解析上一轮目标 |
| 显式选择 | 「用第二个方案」「按刚才说的」 | 沿用上一轮产出 |
| 任务延续 | 当前轮是 active Agent 任务的下一步 | 续 task 上下文 |
| 修正追加 | 「再短一点」「把刚才那段改成…」 | 操作对象=上一轮 diff/选区 |

- 判定=无沿用 → 历史不进事实层，仅保留极简近因摘要（防失忆，不污染）。
- 判定≠无沿用 → 被沿用的历史意图重述为一条当前轮上下文行（贴近末尾），不原样堆历史。

<!-- RULE
rule_id: BR-AG-DATA-004
主链路: AG-TURN
域: DATA
-->
代理回合上下文组装前必须执行意图沿用判定（IntentCarryover）：当前轮用户输入为本轮主需求，历史
对话、记忆、知识、模板默认为弱上下文，不得自动凌驾当前轮事实判断；仅当判定识别出本轮显式沿用/
调用/选择历史轮意图（指代回指 / 显式选择 / 任务延续 / 修正追加）时，方可将对应历史意图提权并重述
至当前轮上下文邻近位置；判定为「无沿用」时历史不进当前轮事实层，仅保留极简近因摘要，不得整段堆叠
历史污染当前轮。判定维度的具体识别规则与阈值属实现期，本规则锁「当前轮为主、历史默认弱化、显式
沿用才提权、不污染当前轮事实」的机制语义。本规则承接 BR-AG-DATA-001/002 的来源与注入顺序，不重
定义组装骨架。

在选择上下文和开放工具前，同一步骤必须形成版本化 IntentResult（TERM-AG-012），至少包含
`intent_type(answer/inspect/search/analyze/edit/create_document/organize_workspace/clarify)`、`target`、
`mutation`、`ambiguity`、`required_context`、`allowed_tools`。显式目标、否定约束（如“不要修改”）、
ReferencePayload 与当前编辑器状态优先由确定性规则解析；低置信或存在多个高风险目标时输出
`ambiguity=blocking` 并进入澄清，不得凭模型猜测执行。IntentResult 必须同时约束 Context 选择、
Prompt 任务声明和工具允许集；原始用户输入仍作为末条 user 内容保留，IntentResult 不替代原文、不保存
模型隐藏推理。1.0 不要求通用规划器或小模型分类器，允许规则优先实现，但不得退化为单一“是否编辑”布尔值。
当 `ambiguity=blocking` 时，原 AgentTurn 进入 AwaitingClarification，不调用门禁、模型或工具；
用户补充所需信息后，补充内容并入原 AgentTurn，由 `clarificationProvided` 返回 Assembling 并
重新形成 IntentResult。该继续语义不创建新 AgentTurn、不伪造取消，也不替代 DecisionRequest
裁决、错误/中断重试或 Assist 的独立状态语义。

`IntentResult.allowed_tools` 是当前 AgentTurn 的精确允许集，不是 Prompt 建议：

- `answer` 在不需项目事实时为空集，需要事实时只给必要的只读工具；`inspect/search/analyze` 只给当前任务必要的只读工具。
- `edit` 只给目标读取及当前格式/动态能力允许的内容 Proposal 工具；不得因为工具已注册而获得资源操作能力。
- `create_document` / `organize_workspace` 只给当前任务必要的只读工具与 `plan_workspace_operation`；只读结果只能用于形成当前计划，资源副作用唯一仍是 CreateDocumentBundle/WorkspaceOperationPlan 前置裁决链。
- `create_document` 的目标资源建立前不给 `propose_edit`；`clarify` 不开放模型、工具或 Proposal 副作用。
- RuntimeProfile、工具注册、Prompt 文本、前端参数和历史回合都不得扩大该允许集；执行器仍须按 CapabilityGrant 二次校验。

**实现登记（2026-08-12，AG-STRATEGY-05 提示词按改动范围分流编辑操作）**：编辑操作选择由「文件类型一刀切」改为「改动范围」分流——整篇重写/大幅重排 → 纯文本可用 `replace_document`；当前 BDocx 仅当同轮 LogicalState 明确**无表格且无图片**时可用 `replace_document` 提交完整 HTML，后端同时拒绝既有或新引入的表格/图片；局部修改 → `replace_block`（纯文本）/ `block_ops`（富文档）；表格/图片 → `table_ops`/`image_ops`。结构状态不明时先读取运行时声明或 `list_tables`/`list_images`，不得从段落列表猜测。这样既不把无受保护结构的整篇改写错误拆为易残留的块操作，也不允许整篇替换绕过受保护结构。

**实现登记（2026-08-12，AG-STRATEGY-04 放开整篇替换 + insert 新块唯一 id）**：当前 BDocx 在动态声明确认无表格、无图片时才放开 `replace_document` 整篇替换（identity 漂移校验 + 服务端表格/图片守卫兜底；既有块局部改动仍须 `block_ops`）；insert/append 新块 afterNode 结构改为 type/attrs/content（attrs 含 blockId，对齐 replace_node），后端兜底生成唯一 attrs.blockId（new_block_{uuid}），前端 nodeFromJSON 生成的新块可跨批锚定，根治多块重写"新块无 id/标题同 id"错乱。

**实现登记（2026-08-13，AG-STRATEGY-05 事务安全与状态语义闭合）**：同锚点的 `insert_before`/`insert_after` 与 `replace_node`/`delete_node` 改为单事务内先处理既有锚点、再插入新块；同坐标多插入按提案阅读顺序稳定落位。forward 先作最终节点、节点 id 与数量后置校验，失败不 dispatch；reverse 按 `afterNode.attrs.blockId` 定位并删除实际插入节点，绝不把插入操作的空 `beforeNode` 当作可解析节点。提示词、能力声明和工具结果统一为三段事实：`propose_edit` 只确认待审建议生成；客户端预览另有 applied/failed 回执；用户接受只确认，保存成功才落盘。前端日志以 `[de-draft-preview]` 输出 `diff_id`、`preview_state` 与可选错误码，不记录正文；卡片在 queued/applied/failed 状态分别显示真实边界并阻止 queued 直接接受。

**实现登记（2026-08-11，AG-STRATEGY-03 文档感知 + 块 id + 排版意识 + 确认去重）**：Edit 动词表补「丰富/扩写/充实/重排/排版/美化」、文档指涉词补「故事/寓言/童话/小说」；非寒暄 Answer/Inspect/Analyze 意图且打开文档时兜底注入当前文档（只读，纯寒暄不注入）；insert/append 新块 afterNode 允许可选 blockId（模型显式给新块 id）；block_ops 投影 after 侧对 insert/append 新块用独立 id 标注（不再复用锚点 id，修复标题块与正文块同 id 错乱）；prompt 补排版映射（style_ops + heading）、确认去重（默认直接 propose_edit）、生成后自检（list_blocks 核对 + 删残句）。修复「丰富故事」被判零工具 Answer 导致模型看不到内容、insert 新块无 id、无排版意识、反复文字确认 4 类问题。

**当前 Prompt 约束（2026-08-21，WorkspaceOperation 基线）**：

- `organize_workspace` 及创建/重命名/移动/删除/恢复意图只能产生 `WorkspaceOperationPlan`，并由独立 `DecisionRequest` 前置裁决；不得调用 `propose_edit(operation=...)`、生成资源 DiffCard 或在确认后等待第二次保存。
- 新建文档必须在确认前同时准备目标路径、初始内容和 `CreateDocumentBundle`；确认后由 WS 创建真实磁盘目标，系统再打开标签并以普通内容 Diff 投影已冻结内容。
- Prompt 只描述意图、允许工具和上述边界；确认身份、幂等、路径/基线核验、dirty 阻断、补偿和重启 Interrupted 由运行时强制，不以模型文字作为授权证据。
- `userConfirm` 恢复原 TaskRun 并立即调用 WS 执行冻结 plan；不新增 `WaitingUserCommit`。重启后旧运行进入 Interrupted，再次执行必须重新生成 plan 和 DecisionRequest。

**实现登记（2026-08-11，AG-INTENT-COMPOSITE-01 复合请求意图优先级）**：`resolve_intent_result` 新增 `create_requested` 前置判定——非否定 ∧ 文本含 Create 强词（创建/新建/生成/建立/制作）∧ 目标为 Document/Workspace 时，CreateDocument 优先于 `verb == Edit` 分支。修复真实案例「新建一个word文档，…并优化排版」被判 Edit 当前文档而无法新建（detect_verb 按 Edit→Create 首命中，「优化」掩盖「新建」）。detect_verb 首命中机制与 V2/V3 语义不变；补复合意图回归测试（复合/存量编辑/单动作/否定）。

**实现登记（2026-08-11，AG-WS-LAYERED-01 模型分层感知工作区）**：`list_docs` 不再全量返回文件树——无参数返回工作区概览（顶层目录+文件计数+总文件数，mode=overview；工作区 ≤30 文件且 ≤4KB 时豁免返回全量 mode=full）；`path` 参数返回该目录直接子项（一层，mode=children）。树枚举只含索引（路径/类型/计数），**永不携带正文**（正文唯一入口 read_doc / search_text）。该登记只证明分层读取现状，不授予新建或其他资源副作用；新建文档必须遵守本节 CreateDocumentBundle 约束。

**实现登记（2026-08-11，AG-TASK-01 多提案批次衔接 + block_ops 删除）**：①`block_ops` 新增 `delete_node` 操作——仅删除 capabilities 含 delete_node 的 content 原子块（删除后块整体移除不留占位），容器/原子块拒绝 AG_BLOCK_OPERATION_TARGET_READONLY（不级联子树）；删除后同批后续操作引用已删块 → stale 回注（F4）。②放开同回合多提案——AG_PROPOSAL_ALREADY_CREATED 改为按批放行，单回合 ≤5 批（AG_TOO_MANY_PROPOSALS 超限），同轮可并行多 tool_calls，每批 base 递增。③批次元数据——propose_edit 支持可选 `batch={id,index,total}` 参数（1≤index≤total），随 style_ops `_batch` 键落库；接受路由返回 batch_remaining 衔接提示（F1c）。④前端——DiffCard 批次徽标（第 x/y 批）、删除块渲染（before=内容/after=【已删除】删除样式）、edBlockOps 支持 delete_node 原子应用与 reverse 精确恢复（记录删除位置重建）。⑤提示词——block_ops 描述补 delete_node、单批上限 24→32、批次衔接引导。语义详见 A-AG-M-T-01 §5.1。

**实现登记（2026-08-11，AG-CTX-01 上下文管理重构）**：上下文控制从"事后截断"改为"源头轻量投影+按需读取"（Claude Code Tool Result Budget 同思路）——①`list_blocks` 轻量投影：只返回块目录（block_id/block_index/node_type/role/parent/depth/text 前40字/fingerprint/capabilities），省略 node_json/全文/marks/attrs；②`read_doc` 支持 `blocks` 范围参数：只返回指定块完整投影（含 node_json 供 block_ops 提案），省略整篇 content，向后兼容（不传则全量）；③移除工具结果截断（trim_tool_results_to_budget 删除，曾导致提案关键数据丢失）；④查询动词补齐（列出/列举/显示/展示/有哪些 → Search 只读组），修复「列出文件树」被误判 Edit；⑤整篇对齐任务检测（`detect_sync_from_reference`）：「改成和 X 一样」注入能力边界提示（block_ops 单批 ≤24 块，引导分批/关键章节），不阻断；⑥AG_EDIT_TOOL_REQUIRED 文案区分「已读未提案」与「零工具调用」；⑦快照序列化空 marks/attrs 省略（`skip_serializing_if`）。

实现约束补充（2026-07-18）：历史消息必须携带所属 `relPath/turnId`。存在当前文档时，AG
装配必须先按当前 `active_doc_path` 分流历史：同文档历史可进入同文档 P1 背景；跨文档或无归属历史
默认不得注入事实层。只有当本轮输入包含明确历史回指（如“刚才/上面/上一轮/按刚才/第二个方案”等）
时，跨文档历史才可被弱标签化注入，且必须声明“不得作为当前文档事实”。“重写/改写/优化/再短一点”
等泛编辑词本身不构成历史沿用。

## §5 补强层污染隔离与分层截断序

> 承 BR-AG-DATA-001「确定性裁剪 + 显式注入优先保留」，本节细化补强层隔离与完整截断阶梯。

**补强层隔离**：记忆/知识/模板为补强层（augmentation），其注入串必须自带权重声明——明示
「仅作补强，不覆盖当前文档、显式引用与当前轮产出」；知识库自动检索须经触发门控（当前文档优先：
有编辑锚点/显式引用充分/纯当前文档意图/文件操作意图时不触发，详尽门控示意见 R-AG-X-R-01 §2.4）。

**分层截断序（确定性阶梯，超预算时从高数字层先截）**：

```
P0 永不截 : 治理/系统层、工具/能力层、格式层、当前文档/编辑器层、当前对话层
P1 尽量留 : 任务状态层、引用层(显式@mention)、历史对话层(可先摘要化)
P2 先截   : 记忆补强层、知识补强层、模板补强层
P3 按需   : 工作区依赖层
```

<!-- RULE
rule_id: BR-AG-DATA-005
主链路: AG-TURN
域: DATA
-->
提示词补强层（记忆 / 知识 / 模板）的注入必须满足污染隔离：注入串自带权重声明，明示仅作补强、不
覆盖当前文档与显式引用与当前轮产出；知识库自动检索须经确定性触发门控（当前文档优先，编辑上下文/
显式引用充分/纯当前文档意图时不触发）。超上下文预算时按确定性优先级阶梯截断：当前文档、当前对话、
系统/工具/格式层永不截（P0）；任务状态、显式引用、历史（可先摘要）尽量保留（P1）；记忆/知识/模板
补强层先截（P2）；工作区依赖按需注入（P3）。预算数值与门控阈值属实现期标定，本规则锁「补强层低权 +
权重声明 + 确定性分层截断序」。本规则细化 BR-AG-DATA-001 裁剪优先级语义，不重定义来源限定。

实现约束补充（2026-07-18）：当前文档 P0 不得以“截取前部后继续编辑”的方式静默降级。当前文档
超过内联预算时，AG 必须注入可追踪块目录/manifest，并要求模型先调用 `read_doc` 或 `list_blocks`
读取当前文档目标内容；若模型在未读取前直接对当前文档 `propose_edit`，工具层返回可恢复错误，不生成
diff。未打开文档仍按 DiskState 读取规则处理。

### §5.1 Prompt 数据可信边界与最小运行包

> 本节细化 BR-AG-DATA-002 的 Prompt 可信边界，并由 BR-AG-DATA-004 的 IntentResult 与
> BR-AG-DATA-005 的分层预算共同约束，不新增平行规则。

Prompt Runtime 必须把系统约束与不可信数据物理分层：只有产品不变量、运行策略和经本轮允许的工具契约
可进入 system instruction；当前文档、显式引用、历史、记忆、知识和工具结果必须封装为 data-only
PromptDataEnvelope，携 `source_kind/provenance/freshness/content_hash/trust_level/token_estimate`，不得因其
内容包含“忽略规则/调用工具/修改文件”等文字而获得指令权限。每次调用产出版本化 PromptPackage，
至少记录 `runtime_version/intent_id/context_trace_id/tool_manifest_hash/estimated_tokens`；发送前校验总预算、
角色边界与 workspace 作用域。调试追踪默认只保存层元数据、哈希、选择/裁剪原因，不保存用户正文、密钥
或会话令牌。

`agent-edit-1.0` 的装配顺序固定为：系统不变量与输出格式 → 本轮工具允许集 → IntentResult → 当前文档
data-only envelope → 显式引用 data-only envelope → 同文档必要历史 → 当前 user 原文。AST/KB/TPL 自动
补强关闭；后续 profile 启用时按 BR-AG-DATA-005 的 P2 门控插入，不得改变系统/数据可信边界。

**实现装配登记（2026-08-09，AG-PROMPT-STRUCTURE-FIX-01）**：生产装配以 `LayerKind` 枚举为唯一真源，
固定层序 `System → RuntimeControl → IntentHint → CurrentDocument → Reference → History →
CrossDocHistory → User`（CrossDocHistory 仅显式沿用时注入）。其中 `RuntimeControl`（AG RUNTIME CONTROL
结构化 system 消息）与自然语言提示词分离为两条独立消息；`History` 聚合为单块 DATA-ONLY 消息并保留
role 序列；`system_prompt_version` 由提示词内容 hash 派生；预算 gate 使用序列化 messages+tools 统一口径；
审计直接读取 `message.layer` 字段。本登记为 §2/§5.1 分层与截断序的实现映射，不改变 BR-AG-DATA-004/005
与 M-T-01 §5.1 骨架语义。

## §6 三类回合结构示例（非规范说明）

> ⚠️ 本节为**设计示例**，帮助理解装配产物形态；其中文本为占位示意，**不构成被锁提示词文本**
> （文本归实现期运营配置，M-T-01 §5.1 已声明）。三类回合骨架差异权威源为 BR-AG-DATA-002。

**Chat 回合**（含历史 + 修改建议导向 diff）：
```
[系统层] 角色 + 硬约束(改经diff裁决/不越权落盘/workspace限界) + 格式约束
[工具层] read_doc, list_docs, search_text, propose_edit
──缓存边界──
[当前文档] file + 文档块列表；富文档含 node_type/role/parent/depth/node_json/fingerprint/capabilities，
容器/原子块只作语境，内容块按稳定 blockId 独立寻址 + revision/baseline + pending diffs
[引用层] @mention: ref#1 精确锚点 block[..]→block[..] / ref#2 阅读上下文(仅理解)
[补强层] (经门控,带权重声明:不覆盖当前文档/显式引用)
messages[]: …历史(弱化/摘要)… → [当前对话] user 输入 (+ carryover 结果)
```

**Build 回合（CBT 构建任务）**（任务计划上下文 + 单文件产出导向）：
```
[系统层] 构建角色 + 单文件产出边界(目标=本任务计划文件，BR-CBT-DATA-001)
[工具层] 构建相关工具子集
──缓存边界──
[任务状态层] active 任务 + 阶段 + artifacts
[当前文档] 目标文件块列表
messages[]: 任务计划上下文 → [当前对话] 本步指令
```

**Assist 回合（辅助续写/局部修改，2026-07-04 澄清定义）**（轻量、无系统对话历史）：
```
[系统层] 按 assist_kind 选择专用系统提示词：Autocomplete / InlineAssist
（无工具层 / 无历史对话层）
[当前文档] 光标上下文 ±N 块 + 选区
[当前对话] 触发上下文(无显式 user 消息或极简指令)
```
两形态差异（BR-AG-STATE-003）：**辅助续写**（Autocomplete）一次产出严格 3 个 50–100 字量级续写候选
（数值属运营配置），请求提示要求只返回 JSON 字符串数组，不解释、不包裹 markdown；运行时解析器需兼容
纯 JSON 数组、fenced/多行 JSON、`{ candidates: [...] }` 包装和编号/项目符号文本，并在收敛后仍不足 3 条时
返回空候选 + 内部 `failure_reason=parse_underflow`，前端保留轻量空态但不展示半候选，且 service/store/overlay
诊断点必须保留该 failure_reason；**局部修改**（InlineAssist）支持弹窗内多轮——refine 轮仅携上轮
候选产出为基底 + 新指令续调（仍无系统对话历史层，不持久化，每轮过门禁）。

实现承接（提示词入口不得混用）：

- `SYSTEM_PROMPT_CHAT`：Agent 编辑回合，允许工具调用，文档内容修改必须经 `propose_edit` → DiffCard；
  系统常量只保留稳定行为边界。当前文档为 BDocx 时，格式、manifestId/version/sha256、StyleOp/
  TableOperation/ImageOperation、人工专属能力、Diff 等级、DOCX 往返风险与当前表格/图片存在性必须由
  A-SYS-I-P-04 BDocxCapabilityManifest 与同轮运行时声明动态投影；后端以同一结构状态守卫
  `replace_document`，禁止在系统常量中维护第二套 mark/op 清单。
- `SYSTEM_PROMPT_AUTOCOMPLETE`：辅助续写，禁止改写既有内容，禁止解释，唯一合法输出为正好 3 项 JSON 字符串数组；提示词需明确不要 markdown fenced block、不要对象包装、不要解释文字，降低模型偏航概率。
- `SYSTEM_PROMPT_INLINE_ASSIST`：局部修改，禁止 JSON 数组，唯一合法输出为单一候选正文。

## §7 需求到规则映射（REQ_MAP）

<!-- REQ_MAP
req_id: REQ-AG-004
target_type: RULE
target_id: BR-AG-DATA-004
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AG-004
target_type: RULE
target_id: BR-AG-DATA-005
status: mapped
-->
## §8 测试与验收

- **当前轮次原则**（BR-AG-DATA-004）：构造「无沿用」轮 → 历史不进事实层（可查装配产物不含历史正文，
  仅近因摘要）；构造「指代回指/修正追加」轮 → 被沿用历史意图提权重述且贴近末尾；判定可观测（输出
  carryover 类型）。
- **补强层隔离**（BR-AG-DATA-005）：记忆/知识/模板注入串含权重声明字样；知识门控在编辑上下文/显式
  引用充分时不触发（reason 可观测）；超预算时 P2 补强层先于 P0/P1 被截，P0 永不截（确定性可复现）。
- **分层映射一致性**：§2 各层来源与 M-T-01 §5.1 骨架层一一对应，无层越权直写文件（X-FORBID-006）。
- **本轮意图结果**（BR-AG-DATA-004）：否定编辑、显式编辑、新建、分析、目标不明等样本产生稳定
  IntentResult；blocking ambiguity 不开放副作用工具；Context 与工具允许集消费同一 intent_id。
- **数据可信边界**（BR-AG-DATA-002/005）：含提示注入文字的文档/引用仍为 data-only；PromptPackage 可查
  runtime/version/hash/token/trace，超预算或 envelope 越出 workspace 时在网关调用前拒绝。
- **BDocx 改写路由与状态诚实性**：无表格、无图片的当前 BDocx 可提交完整 HTML `replace_document`；已有或
  新引入表格/图片时服务端返回结构化操作拒绝。`propose_edit` 工具结果必须只称建议已生成，并显式区分
  `pending_user_review`、未回报的编辑器预览和未保存的文件状态；前端预览 applied/failed 事件不记录正文。
- 提示词文本/措辞/预算数值不在验收范围（实现期运营配置）。

## §9 技术债务与盲区（诚实声明）

- **意图沿用判定的识别实现**（BR-AG-DATA-004 余项）：四维度的具体识别（规则/小模型/启发式）与阈值
  延实现期；本文仅锁机制语义，PI-AG-09 落地时需补识别策略并回写。
- **相关性打分**：分层截断在预算内的「选哪些补强条目」需相关性打分；旧项目（R-AG-X-R-01 §2.2）为
  Passthrough 占位，binder 实现期需补，否则 P2 层「按预算选 TopK」形同虚设（列 R-01 风险 R-01）。
- **预算数值与门控阈值**：机制已锁，数值延 G5 标定。
- **展示层细节**（流式 diff 呈现）归 DE/前端，本文不展开（见 R-AG-X-R-01 §6 / A-AG-X-L-02 PI-AG-10）。

## §10 后续动作

1. 落地后随 `npm run governance:generate` 注册：BR-AG-DATA-004/005 规则、TERM-AG-005、REQ_MAP 进 ADUS。
2. M-T-01 §5.1 增一句交叉引用指向本文（本文为其 L3 细化）——属 M-T-01 变更，待人类确认后由 AI 执行。
3. PI-AG-09（A-AG-X-L-02）承接 BR-AG-DATA-004/005 实现意图沿用判定与分层截断。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-24 | v1.0 | 初版（M-T-01 §5.1 的 L3 专项细化）：提示词分层总览（13 层映射至来源模块与已有规则）、装配顺序与缓存边界布局、当前轮次原则与意图沿用判定、补强层污染隔离与分层截断序、三类回合结构示例（非规范）；新增规则 BR-AG-DATA-004（当前轮次原则/意图沿用）、BR-AG-DATA-005（补强层隔离/分层截断序）；新增 TERM-AG-005（IntentCarryover）；REQ_MAP（REQ-AG-004）；承接 BR-AG-DATA-001/002 不重定义骨架；调研依据 R-AG-X-R-01；作为提示词设计迭代版本，明确 BR-AG-DATA-005 为分层截断序唯一权威，废止 M-T-01 早期「@mention > 记忆/知识 > 当前文档」裁剪排序（同步 M-T-01 v1.2 去重，冲突以本文为准） |
| 2026-07-04 | v1.1 | Assist 回合承接术语澄清（人类裁决）：更名「辅助续写/局部修改」，补两形态提示词差异——续写单候选 50–100 字量级；局部修改多轮 refine（上轮产出为基底，无历史层、不持久化、每轮过门禁） |
| 2026-07-16 | v1.2 | 细化 Assist 提示词形态：辅助续写一次产出 3 个候选；局部修改 refine 仅携上轮候选结果与新指令，不保留完整对话历史 |
| 2026-07-16 | v1.3 | 续修辅助续写提示词契约：autocomplete 必须返回 JSON 字符串数组且严格 3 项，候选不足由 AG-ASSIST 静默失败 |
| 2026-07-16 | v1.4 | 拆分 Assist system prompt：辅助续写使用 `SYSTEM_PROMPT_AUTOCOMPLETE`，局部修改使用 `SYSTEM_PROMPT_INLINE_ASSIST`，Agent 编辑继续使用 `SYSTEM_PROMPT_CHAT` |
| 2026-07-17 | v1.5 | 加固辅助续写提示词与解析契约：提示词继续要求严格 JSON 三候选，运行时兼容常见模型格式偏差并通过 failure_reason 诊断解析不足 |
| 2026-07-17 | v1.6 | 收敛 Assist failure_reason 承接：解析不足等内部原因必须传递到前端诊断点，不得被统一空态吞掉 |
| 2026-07-18 | v1.7 | 加固 Agent 编辑 Prompt 装配：历史按 `relPath` 分流，跨文档历史仅显式回指时弱注入；泛编辑词不触发 IntentCarryover；当前文档超预算改为块目录 + read_doc/list_blocks 门禁，禁止前部截断后继续编辑 |
| 2026-07-28 | v1.8 | BDocx TableCellShading：`SYSTEM_PROMPT_CHAT` 能力说明纳入 `list_tables→table_ops`、`set_cell_background` 与禁止原始 HTML 的编辑协议。 |
| 2026-07-28 | v1.9 | BDocx 能力说明改由版本化 BDocxCapabilityManifest 投影；SYSTEM_PROMPT_CHAT 仅保留稳定行为边界，当前文档上下文注入格式、版本/hash、代理/人工能力、Diff 与往返等级。 |
| 2026-07-29 | v1.10 | BDOCX-IMAGE-AGENT-EDIT-01：BDocx 能力投影增加 list_images→image_ops、项目图片来源、动态锚/哈希和图片操作排除边界；提示词仍只消费 manifest，不新增手写枚举。 |
| 2026-07-30 | v1.11 | 图片代理编辑闭合审计：明确 SYSTEM_PROMPT_CHAT 的 BDocx 动态投影同时包含 ImageOperation；组合操作、受控导入与非视觉识别继续由 manifest 单一来源注入。 |
| 2026-07-30 | v1.12 | AG-RICH-BLOCK-STRUCTURED-EDIT-01：当前富文档层注入 ProseMirror 层级块、动态写能力和 node_json；提示词只允许 capability 驱动的 block_ops，多块同批，禁止用 HTML replace_block/replace_document。 |
| 2026-08-07 | v1.13 | AG-MSG-RENDER-01：格式层输出格式约束由「纯文本」改为「GFM Markdown 子集」，声明当前无图表/流程图（Mermaid）渲染能力；展示层承接 BR-AG-UI-001（消息流渲染与时序契约） |
| 2026-08-08 | v1.14 | 定义 1.0 Prompt Runtime 最小闭环：将版本化 IntentResult 收敛进 BR-AG-DATA-004，将 PromptDataEnvelope/PromptPackage 可信边界收敛进 BR-AG-DATA-002/005；锁定 agent-edit-1.0 装配顺序、版本/trace 字段，明确 provider cache 非发布阻断项、AST/KB/TPL 自动补强关闭 |
| 2026-08-09 | v1.15 | AG-PROMPT-STRUCTURE-FIX-01：§5.1 登记生产装配以 LayerKind 枚举为唯一真源（System→RuntimeControl→IntentHint→CurrentDocument→Reference→History→CrossDocHistory→User），RuntimeControl 独立消息、历史聚合单块、版本 hash 派生、预算统一口径、审计读 layer；不改变 BR-AG-DATA-004/005 语义 |
| 2026-08-11 | v1.21 | AG-TASK-01：§4 登记多提案批次衔接 + block_ops 删除——delete_node（content 原子块）、单回合 ≤5 批、batch 元数据落库/接受衔接、DiffCard 批次徽标/删除渲染、单批上限 24→32 |
| 2026-08-13 | v1.22 | AG-STRATEGY-05：纠正 BDocx 整篇改写边界为「同轮无表格且无图片」并由服务端拒绝受保护/新引入结构；登记同锚点 block_ops 原子排序、后置校验、稳定 id reverse，以及 proposal→预览→接受→保存的可观察状态和无正文前端 trace。 |
| 2026-08-21 | v1.23 | WorkspaceOperation 基线同步：增加当前 Prompt 约束，资源意图只生成 WorkspaceOperationPlan + DecisionRequest，新建经 CreateDocumentBundle 先创建真实目标再投影内容 Diff；明确 2026-08-11 资源 Diff/空基准新建段落只作历史实现记录。 |
| 2026-08-27 | v1.24 | AG-CONFORMANCE-GOVERNANCE-S1：明确 `allowed_tools` 为当前 AgentTurn 精确最小允许集，注册/Profile/Prompt 不构成授权；资源意图可获得任务必要只读工具 + plan_workspace_operation，副作用路径不变；澄清补充回到原 AgentTurn 的 Assembling，与 DecisionRequest、重试和 Assist 分界；移除 active 正文中的全量工具、资源 Diff/跨文档宽放和空基准新建旧登记，唯一化 CreateDocumentBundle 路径。 |
| 2026-08-11 | v1.20 | AG-CTX-01：§4 登记上下文管理重构——list_blocks 轻量投影、read_doc blocks 范围参数、移除工具结果截断、查询动词→Search、整篇对齐检测提示、AG_EDIT_TOOL_REQUIRED 文案区分、序列化瘦身 |
| 2026-08-11 | v1.19 | AG-DOCX-RO-01：§4 登记原生 docx 只读——复用 pandoc 转换链，source=doc_convert_readonly、editable_kind 覆盖 html（≠bdocx），结构性只读；propose_edit 仍拒绝，编辑器路径保持 DOC_NOT_BDOCX；提示词补充只读边界 |
| 2026-08-11 | v1.18 | AG-R2-01：§4 登记整理复原综合修复——预算（工具结果历史压缩 + block_ops 单批上限 24）、意图（纠错语义词 + 疑问文档指涉→只读）、提示词（编辑当轮提案 + 顺序自查）、范围（workspace 内只读放宽 + 锚点矛盾校验） |
| 2026-08-10 | v1.17 | AG-INTENT-V3-01：§4 登记乐观授权——工具授予三分类固定（只读组无条件 + 提案组不落盘），意图识别退化为上下文注入器，识别失败不再零工具瘫痪；寒暄零工具、只读只读、疑问无动作 Answer；BR-AG-STATE-002 allowed_tools 语义逐字满足 |
| 2026-08-09 | v1.16 | AG-INTENT-V2-01：§4 登记 resolve_intent_result 内部升级为 Parsing→Adjudication 语义化判定（动词×目标×修饰语 + 确定性优先级链），修复「改为红色」「创建md文档」被误判 Answer；IntentResultV1 契约不变，IntentSignal 为私有结构，沿用复用 carryover |
