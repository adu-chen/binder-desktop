---
文档编号：AG-M-T-02
文档状态：A
负责模块：AG
文档职责：AG 提示词架构 L3 专项设计——定义分层装配、当前轮意图、数据可信边界、上下文截断与 1.0 Prompt Runtime 最小契约
上游约束：A-AG-M-D-01_代理功能需求描述.md（REQ-AG-004/007/017）、A-CORE-X-D-02_Agent1.0冻结能力基线.md、A-AG-M-T-01_代理技术设计.md（§2.1/§5/§5.1 BR-AG-DATA-001/002、BR-AG-STATE-002）、A-CORE-C-P-01_APC.md（云薄端、§13 不绕门禁）、A-SYS-I-P-03_云接口契约.md（X-SEQ-001 门禁前置）
直接承接：本文唯一持有 IntentCarryover（TERM-AG-005 / BR-AG-DATA-004）与数据隔离/截断序（BR-AG-DATA-005）的专项真源，组装骨架仍归 A-AG-M-T-01 BR-AG-DATA-002；A-AG-X-L-04 承接实现与验证，R-AG-X-R-01 仅作调研参考
使用边界：本文是 M-T-01 §5.1 的 L3 细化，不重定义组装骨架（归 BR-AG-DATA-002）与来源裁剪语义（归 BR-AG-DATA-001）；不锁定提示词文本/措辞/模型档或预算数值（实现期运营配置）；不定义门禁/网关契约（归 SYS I-P-03）、diff 生成（归 DE）、@mention 解析（归 REF）；Build/CBT、自动记忆/知识/模板不属于 `agent-edit-1.0`；§6 结构示例为非规范说明，不构成被锁文本
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
IntentResult 中对当前轮与历史目标关系的可见语义投影：模型可表达本轮是否沿用/调用/选择了历史对象
（指代回指 / 显式选择 / 任务延续 / 修正追加）。运行时只提供有来源、受预算约束的历史候选，不以本地
关键词预判沿用；沿用也不授予目标或工具权限（BR-AG-DATA-004）。

## §1 文档定位

```
M-T-01 §5.1（组装骨架 SSOT：BR-AG-DATA-002 分层/顺序、BR-AG-DATA-001 来源/裁剪）
   │ 深度拆解（L3）
   ▼
本文（提示词架构）：多层清单 + 装配顺序/缓存边界 + 当前轮次原则 + 补强层隔离/截断序 + 结构示例
   │ 新增锁：BR-AG-DATA-004（当前轮次原则与意图沿用）、BR-AG-DATA-005（补强层隔离与分层截断序）
   ▼
A-AG-X-L-04 承接当前实现与验证；R-AG-X-R-01 提供调研依据（仅参考）
```

**与 M-T-01 的边界（职责单一化，去重）**：M-T-01 §5.1 锁「四层组装骨架（分层 + 注入顺序 + 当前两类
回合差异）」（BR-AG-DATA-002）与「来源限定 + 确定性裁剪 + 显式注入优先保留」原则（BR-AG-DATA-001）。
本文**不重定义骨架与来源原则**，只做两件 M-T-01 未覆盖的事：
1. 把四层骨架细化为面向 binder 文档场景的**当前分层清单**，并标出 P2 保留层不得在当前 Profile 自动激活，
   每层标注来源模块与已有规则映射（§2）；
2. 补 M-T-01 缺失的两个机制：**当前轮次原则 / 意图沿用判定**（§4，BR-AG-DATA-004）与
   **补强层污染隔离 / 分层截断序**（§5，BR-AG-DATA-005）。

**版本优先（迭代声明）**：本文是提示词设计的迭代版本。**分层截断序以本文 BR-AG-DATA-005 为唯一
权威**——M-T-01 §5.1 早期把当前文档排在裁剪优先级最低位的旧排序「显式 @mention > 记忆/知识 >
当前文档」，与当前轮次原则（当前文档为当前轮主事实，§4）冲突，**已废止**，统一以本文「当前文档 P0
不得静默截断、显式引用与用户明确选择的项目记忆 P1、P2 自动补强关闭」为准；2026-09-15 P00 明确
v2 的 P0 是已激活必需内容的保留义务，不是所有请求的全量采集前提。M-T-01 v1.2 已同步移除该旧排序并委派
本文，**不存在两处规则各持一套截断序的逻辑重复**。

**云薄端前提重申**：提示词在客户端装配，但模型调用经门禁→网关（X-SEQ-001）；本文不涉调用链。

## §2 提示词分层总览（M-T-01 §5.1 骨架细化）

> M-T-01 四层骨架（系统提示词层→工具/能力定义层→上下文注入层→用户输入层）在 binder 文档场景下
> 细化为下表。**本表为设计映射，各层文本属实现期运营配置（不入规则）**；分层与顺序承 BR-AG-DATA-002，
> 来源与裁剪承 BR-AG-DATA-001。

| 层 | 对应 M-T-01 骨架层 | 内容 | 来源模块 | 截断优先级 | 已有规则映射 |
|---|---|---|---|---|---|
| 治理/系统层 | 系统提示词层 | 角色定位 + 硬约束边界（修改经 diff 裁决、不越权落盘、workspace 限界） | AG | P0 永不截 | BR-AG-STATE-001 / X-FORBID-006 |
| 工具/能力层 | 工具/能力定义层 | 当轮 CapabilityGrant.allowed_tools 的精确契约投影；不暴露完整注册表 | AG | P0 | BR-AG-STATE-002 |
| 格式层 | 系统提示词层（子项） | 输出格式约束（**GFM Markdown 子集**：段落/标题/列表/加粗/围栏代码块标注语言/表格；**声明当前无图表/流程图（Mermaid）渲染能力**；HTML/docx 仅面向 BDocx 能力投影） | DOC/DE | P0 | BR-AG-DATA-002（骨架内）、BR-AG-UI-001 |
| 任务状态层 | 上下文注入层 | 当前受限任务 + 阶段 + artifacts（仅存在 TaskRun 时） | AG | P1 | BR-SYS-STATE-008 / SM-AG-04 |
| 代理上下文事实集层 | 上下文注入层 | 版本化 AgentContextFacts：workspace、活动/打开会话、格式、脏态、保存基准、LogicalState 版本、显式 SelectionRef、关联 Proposal 状态；不含正文 | SYS/ED/DE/WS | P0 当前轮事实元数据 | BR-AG-DATA-008 / BR-SYS-DATA-008 |
| 当前文档/编辑器层 | 上下文注入层 | v2 按需取得的文件/块/选区/基线；未激活时只在事实层声明身份与可用性，不携正文 | WS/ED/DE | **激活后 P0，不静默截断** | BR-AG-DATA-001/008 |
| 引用层 | 上下文注入层 | 显式 @mention（精确锚点可编辑 / 阅读上下文仅理解） | REF | P1 最高保留 | BR-AG-DATA-001（@mention 优先） |
| 当前对话层 | 用户输入层 | 当前轮 user 输入 + IntentResult（结构化决策不替代原文） | AG | P0 信号源 | BR-AG-DATA-004（本文） |
| 历史对话层 | （由 messages[] 承载） | 既往轮次，默认弱化 | AG | P1/弱 | BR-AG-DATA-004（本文） |
| 显式项目记忆层 | 上下文注入层 | 用户在当前 workspace 明确选择的既有记忆条目；按显式引用归一，data-only | AST/REF | P1 | BR-AG-STATE-004 / REQ-AG-017 |
| 知识补强层 | 上下文注入层 | P2 保留层；`agent-edit-1.0` 不自动检索或注入 | KB | inactive | A-CORE-X-D-02 §3 |
| 模板补强层 | 上下文注入层 | P2 保留层；`agent-edit-1.0` 不自动执行或注入 | TPL | inactive | A-CORE-X-D-02 §3 |
| 工作区依赖层 | 上下文注入层 | 文件依赖图、pending diff 状态 | WS | P3 按需 | BR-AG-DATA-005（本文） |
| 展示层（输出侧） | —（非输入层） | 流式 chunk / 工具卡片 / diff 卡片，与模型输入解耦 | AG/前端 | — | 归 DE/前端，见 R-AG-X-R-01 §6 |

**分层准则**：当前输入始终保留；已激活的专业主事实与显式引用为 P0/P1 高保留。v2 不因当前标签存在
就必然采集正文或格式操作清单，不因无关编辑器未就绪阻断纯对话。用户明确选择的项目记忆
按 P1 data-only 引用处理；知识/模板自动补强在当前 Profile 不装配；工作区依赖为 P3 按需。展示层是输出侧，不进模型输入（隔离）。

## §3 装配顺序与缓存边界布局

> 承 BR-AG-DATA-002 注入顺序，本节补**物理布局**设计（缓存/recency 优化），不改顺序语义。

```
[ 稳定区·可进 prompt cache ]
  治理/系统不变量 → 格式层
        ──────── 缓存边界标记 ────────
[会话特定区·每轮易变]
  RuntimeControl → 本轮 CapabilityGrant.allowed_tools 精确投影 → 任务状态层（如有）
  → 代理上下文事实集层 → 当前文档/编辑器层 → 显式引用层（含用户明确选择的项目记忆）
  ───────────────────────────────────────────────
[messages[] 数组·与 system 分离]
  历史对话层(默认弱化) … 当前对话层(末条，含意图沿用结果)
```

1.0 必须落地统一 Prompt Runtime 和 `runtime_version`，但 provider prompt cache 命中率不是 1.0 发布
阻断项：没有可靠缓存时允许每轮完整装配，不能为了缓存复用而混淆系统规则、文档数据和用户输入。
稳定区、动态区、工具 manifest hash、IntentResult revision 与 Context trace id 必须可观测，以支持同一回合复现。

**两条布局设计依据**（调研支撑见 R-AG-X-R-01 §3.1/§3.4）：
1. **稳定层在前并进缓存边界**：仅角色/系统不变量/格式可视为跨轮稳定；当轮 CapabilityGrant 及其工具契约投影必须放在动态区，不得因缓存沿用旧轮允许集。
2. **当前轮易变事实靠近末尾**：当前文档、显式引用与当前对话置于 prompt 末尾；未获本轮选择或授权的
   记忆、知识、模板不得为缓存或“补强”而注入。

## §4 当前轮次原则与意图沿用判定

> M-T-01 §5.1 只锁注入顺序，未规定历史轮的处置。本节补此机制。

**原则**：当前轮用户输入是本轮主需求。历史轮、记忆、知识、模板默认为弱上下文，不得自动凌驾当前
轮事实。运行时提供带来源的有限历史候选，模型在当前 IntentResult 中表达是否沿用；只有明确沿用的
对象才进入当前路线，而不是整段堆叠历史。

**模型可判断的沿用关系（设计示意，不是关键词路由表）**：

| 维度 | 输入示意 | 语义投影 |
|---|---|---|
| 指代回指 | 「继续」「接着上面」「它/那个」、无主语祈使 | 解析上一轮目标 |
| 显式选择 | 「用第二个方案」「按刚才说的」 | 沿用上一轮产出 |
| 任务延续 | 当前轮是 active Agent 任务的下一步 | 续 task 上下文 |
| 修正追加 | 「再短一点」「把刚才那段改成…」 | 操作对象=上一轮 diff/选区 |

- 当前 IntentResult 未表达沿用 → 历史不提升为当前事实，仅保留运行时允许的极简近因摘要。
- 当前 IntentResult 明确沿用 → 被引用历史作为 data-only 语义背景，仍不授予目标或工具权限。

<!-- RULE
rule_id: BR-AG-DATA-004
主链路: AG-TURN
域: DATA
-->
代理回合装配必须坚持当前轮优先：当前用户输入完整保留为末条 user 内容；历史对话、记忆、知识与模板
默认为 data-only 弱上下文，不得自动凌驾当前轮事实。运行时只根据显式回指身份、同一未终结回合和预算
选择可供模型判断的历史摘要/引用，不以关键词预判任务类型、编辑意图或工具集合；模型基于当前输入与这些
可见事实形成或更新 IntentResult（TERM-AG-012），其中可表达 IntentCarryover。未被模型当前语义投影沿用的
历史不得提升为当前目标，整段历史不得无界堆叠。

IntentResult 至少包含 `revision/goal/intent_type/target_candidates/side_effect_intent/ambiguity/
required_context/confirmation_points`；初始可为 `unresolved`，每次模型输出或 ToolResult 后可修订。它不包含
`allowed_tools`，不控制 Context 的事实来源，也不授予目标或副作用。运行时可用 ExplicitEditTargetSet、
ReferencePayload 身份和编辑器状态校验模型目标是否可执行，但不得把确定性关键词分类结果伪装成模型语义。
原始输入始终保留，IntentResult 不替代原文、不保存隐藏推理。

当模型指出关键信息缺失且继续会造成目标或高风险副作用不明确时，运行时追加 clarification 事件并让原
AgentTurn 进入 AwaitingClarification；用户补充后，补充内容进入原回合，重新冻结 AgentContextFacts、重新
Gate 并调用模型更新 IntentResult。该继续语义不创建新 AgentTurn、不伪造取消，也不替代 DecisionRequest、
错误/中断重试或 Assist 的独立状态语义。CapabilityGrant 只按 M-T-01 BR-AG-STATE-002 的硬事实计算，
不从 IntentResult 的 intent_type 或 side_effect_intent 派生。

**实现登记（2026-08-12，AG-STRATEGY-05 提示词按改动范围分流编辑操作）**：编辑操作选择由「文件类型一刀切」改为「改动范围」分流——整篇重写/大幅重排 → 纯文本可用 `replace_document`；当前 BDocx 仅当同轮 LogicalState 明确**无表格且无图片**时可用 `replace_document` 提交完整 HTML，后端同时拒绝既有或新引入的表格/图片；局部修改 → `replace_block`（纯文本）/ `block_ops`（富文档）；表格/图片 → `table_ops`/`image_ops`。结构状态不明时先读取运行时声明或 `list_tables`/`list_images`，不得从段落列表猜测。这样既不把无受保护结构的整篇改写错误拆为易残留的块操作，也不允许整篇替换绕过受保护结构。

**实现登记（2026-08-12，AG-STRATEGY-04 放开整篇替换 + insert 新块唯一 id）**：当前 BDocx 在动态声明确认无表格、无图片时才放开 `replace_document` 整篇替换（identity 漂移校验 + 服务端表格/图片守卫兜底；既有块局部改动仍须 `block_ops`）；insert/append 新块 afterNode 结构改为 type/attrs/content（attrs 含 blockId，对齐 replace_node），后端兜底生成唯一 attrs.blockId（new_block_{uuid}），前端 nodeFromJSON 生成的新块可跨批锚定，根治多块重写"新块无 id/标题同 id"错乱。

**实现登记（2026-08-13，AG-STRATEGY-05 事务安全与状态语义闭合）**：同锚点的 `insert_before`/`insert_after` 与 `replace_node`/`delete_node` 改为单事务内先处理既有锚点、再插入新块；同坐标多插入按提案阅读顺序稳定落位。forward 先作最终节点、节点 id 与数量后置校验，失败不 dispatch；reverse 按 `afterNode.attrs.blockId` 定位并删除实际插入节点，绝不把插入操作的空 `beforeNode` 当作可解析节点。提示词、能力声明和工具结果统一为三段事实：`propose_edit` 只确认待审建议生成；客户端预览另有 applied/failed 回执；用户接受只确认，保存成功才落盘。前端日志以 `[de-draft-preview]` 输出 `diff_id`、`preview_state` 与可选错误码，不记录正文；卡片在 queued/applied/failed 状态分别显示真实边界并阻止 queued 直接接受。

**结构化编辑登记（2026-08-11）**：insert/append 新块 afterNode 允许可选 blockId，服务端负责生成/校验
唯一 id；block_ops 投影的新增块不复用锚点 id；Prompt 可说明 style_ops/heading 与生成后核对工具。历史
Edit 动词表、寒暄/任务分类和确定性工具分组已退出现行设计，不得作为 IntentResult 或 CapabilityGrant 真源。

**当前 Prompt 约束（2026-08-21，WorkspaceOperation 基线）**：

- `organize_workspace` 及创建/重命名/移动/删除/恢复意图只能产生 `WorkspaceOperationPlan`，并由独立 `DecisionRequest` 前置裁决；不得调用 `propose_edit(operation=...)`、生成资源 DiffCard 或在确认后等待第二次保存。
- 新建文档必须在确认前同时准备目标路径、初始内容和 `CreateDocumentBundle`；确认后由 WS 创建真实磁盘目标，系统再打开标签并以普通内容 Diff 投影已冻结内容。
- Prompt 只描述当前 IntentResult 投影、CapabilityGrant 工具和上述边界；确认身份、幂等、路径/基线核验、dirty 阻断、补偿和重启 Interrupted 由运行时强制，不以模型文字作为授权证据。
- `userConfirm` 恢复原 AgentTurn；运行时以当前终态/取消状态、真实确认回执与冻结 plan 重新计算 permitted_effect_paths，通过后才调用 WS 重验执行；该内部入口不暴露给模型。TaskRun 若存在只投影事件，不新增 `WaitingUserCommit`。重启后旧运行进入 Interrupted，再次执行必须重新生成 plan 和 DecisionRequest。

**实现登记（2026-08-11，AG-WS-LAYERED-01 模型分层感知工作区）**：`list_docs` 不再全量返回文件树——无参数返回工作区概览（顶层目录+文件计数+总文件数，mode=overview；工作区 ≤30 文件且 ≤4KB 时豁免返回全量 mode=full）；`path` 参数返回该目录直接子项（一层，mode=children）。树枚举只含索引（路径/类型/计数），**永不携带正文**（正文唯一入口 read_doc / search_text）。该登记只证明分层读取现状，不授予新建或其他资源副作用；新建文档必须遵守本节 CreateDocumentBundle 约束。

**历史实现登记（2026-08-11，AG-TASK-01）**：当时登记了 `block_ops.delete_node`、多提案批次、批次元数据与 Prompt 数字调整。此处不再持有批次数或单批操作数；运行时 capability/config 是数值唯一源，Prompt 只投影同一值。删除、多提案和 DiffCard 的有效语义以 Active AG/DE/ED 规则与运行时能力为准。

**实现登记（2026-08-11，AG-CTX-01 上下文管理重构）**：上下文控制从"事后截断"改为"源头轻量投影+按需读取"——`list_blocks` 返回不含全文的块目录，`read_doc` 可按 blocks 读取完整目标投影，工具结果不再事后截掉关键提案数据；查询意图与整篇对齐任务使用确定性边界提示，批次/操作上限从运行时 capability/config 动态投影；`AG_EDIT_TOOL_REQUIRED` 区分已读未提案与零工具调用，快照省略空 marks/attrs。本登记不持有预算或操作数值。

实现约束补充（2026-07-18，现行语义已收敛）：历史消息必须携带所属 `relPath/turnId`。运行时按
AgentSession、近因和预算选择有限历史候选：同文档历史可作为 P1 背景，跨文档或无归属历史只能以明确
来源标签的弱摘要进入，均不得作为当前文档事实。不得用“刚才/继续/重写/优化”等关键词决定沿用、目标
或工具；是否沿用由模型在当前 IntentResult revision 中表达，运行时再以身份、范围与当前事实校验。

实现约束补充（2026-10-23，AG-HISTORY-BUDGET-01）：历史按可验证的 AgentTurn 身份形成连续对话单元，
旧版无身份消息只按 user 开始分组；按同文档优先、近因优先装配。最新单元过长时优先保留其用户需求，
并对回答提供带原始长度、哈希和省略标记的首尾原文摘录，不能让一条长回答导致全部近因历史消失。
摘录不是模型生成的语义摘要，不证明被省略内容；原历史不修改，不得据此重建当前授权。
历史额度由完整 provider 投影的剩余预算计算，移除独立字符总盘子、固定 RuntimeControl 字符预留和
固定历史字符硬上限；能力说明只在最终请求测量一次。装配后含历史标签、来源信封、JSON转义、tools及
同回合回执的整个请求必须再次测量；优先收缩历史，必需来源本身超限仍在 Gate 前失败关闭。
每次初始与续行装配均记录历史候选/纳入/丢弃/摘录数、原始/纳入字符数及派生额度，不输出历史正文。

## §5 显式补充数据隔离与分层截断序

> 承 BR-AG-DATA-001「确定性裁剪 + 显式注入优先保留」，本节细化补强层隔离与完整截断阶梯。

**补充数据隔离**：只有用户明确选择的项目记忆可按显式引用进入 `agent-edit-1.0`，注入串必须声明
data-only，且不得覆盖当前文档、其他显式引用、当前用户输入、IntentResult 或 CapabilityGrant。
知识库和模板的自动检索/注入在当前 Profile 关闭，不存在当前门控、相关性打分或 TopK 选择语义。

**分层截断序（确定性阶梯，超预算时从高数字层先截）**：

```
P0 不静默截 : 治理/系统层、当轮允许工具契约、已激活格式/文档层、当前对话层
P1 尽量留 : 任务状态层、引用层（含用户明确选择的项目记忆）、历史对话层(可先摘要化)
P2 不装配 : 自动记忆/知识/模板（当前 Profile 关闭）
P3 按需   : 工作区依赖层
```

<!-- RULE
rule_id: BR-AG-DATA-005
主链路: AG-TURN
域: DATA
-->
用户明确选择的项目记忆必须按显式 data-only 引用注入，保留 workspace、来源与选择事实，不得覆盖当前文档、
其他显式引用、当前输入、IntentResult、CapabilityGrant 或当前轮产出；未选择的项目记忆与知识/模板不得自动
检索或注入。v2 先按真实需求激活专业上下文，再进行预算裁剪；激活不是授权，不得以关键词分类
决定目标权限。超上下文预算时按确定性优先级阶梯截断：已激活的必需文档、当前对话、系统/当轮授权工具/格式层不得
静默截断（P0）；任务状态、显式引用（含明确选择的项目记忆）、历史（可先摘要）尽量保留（P1）；
工作区依赖按需注入（P3）。预算数值属运行时配置，不在本文硬编码。本规则细化 BR-AG-DATA-001 的
数据隔离与裁剪优先级，不构成任何自动补强能力的启用依据。

实现约束补充（2026-07-18）：当前文档 P0 不得以“截取前部后继续编辑”的方式静默降级。当前文档
超过内联预算时，AG 必须注入可追踪块目录/manifest，并要求模型先调用 `read_doc` 或 `list_blocks`
读取当前文档目标内容；若模型在未读取前直接对当前文档 `propose_edit`，工具层返回可恢复错误，不生成
diff。未打开文档仍按 DiskState 读取规则处理。v2 未激活内容不承担上述内联义务，但仍保留资源身份及
可用性；模型必须请求真实读取，不能把身份元数据当正文。完整的激活和缺失处置见 I-P-01 §2.3。

### §5.0 按需激活与预算证据（P00，设计待实现）

1. 通用准入只需身份、当前输入和合法 AgentContextFacts v2；冻结活动资源但不强制收集全部正文。
2. 用户显式引用保留选择事实；只有已核验内容才进入专业事实。未就绪引用明确标记，不静默丢弃或当作已读。
3. 模型可由当前输入、轻量资源信息及工具说明决定读取；具体读取/编辑前由专业适配器检查目标与快照。
   模型的 required_context 仅是需求，不能改变事实来源、扩大 ExplicitEditTargetSet 或绕过授权。
4. 需要专业内容时先取得有效版本，追加 ContextFrozen revision；工具后及继续时重新装配，不复用旧 grant。
5. 预算按实际序列化 messages+tools 统一核算；记录估算方法和配置版本，不把字符近似当 Provider 精确 tokenizer。
6. 初始、工具后、裁决后、预览后分别登记：调用序号、装配原因、各层/工具 schema 占用、总量、上限、
   来源 ID/hash、输入与保留量、裁剪或重复消除原因。分项及序列化开销须能对账，不只计算 envelope 正文。
7. 预留 ToolResult/继续消息空间；结果在专业能力侧有界读取/分块/分页，不事后截断到参数或证据失真。
   只按同一来源/版本/用途去重，不能因文本相同丢失不同引用的来源。
8. 必需内容仍超限时在调用前拒绝，记录阶段和主要占用及下一步；提高上限不是默认修复。
   调试元数据不保存正文/密钥/token。预算失败不得抹掉此前已发生的专业效果。

### §5.1 Prompt 数据可信边界与最小运行包

2026-10-23 AG-BOUNDED-READING-01 设计补充：阅读任务默认使用 I-P-01 §4.1.1 的正文分页，不为阅读携带完整HTML/编辑AST；结构节点与 capability=readonly 只约束编辑，不禁止读取表格文本。已明确授权的只读分页可在剩余预算内继续，不要求每页重新征求同意；需要结束时说明实际覆盖和未读范围。允许从同一权威 schema 去除当前 CapabilityGrant 已明确不开放的操作分支及等价重复定义，不得降低运行时授权/控制协议/参数检查；提示词去重不得改变数据可信层级、写入确认或版本边界。预算仍逐次核算，不因阅读分页放宽总额或隐式刷新执行次数。

P04 实现登记（2026-09-21，验证状态见开发计划 §14.14）：预算门禁、PromptPackage 和发送路径共用剥离内部 layer 的消息投影；按每条完整消息的 JSON 字符数分层，再加数组分隔开销及完整 tools JSON，汇总后统一 ceil(chars/4)。这不是 Provider 精确 tokenizer，也不覆盖响应输出额度。`prompt_budget_checked` 默认只存装配序号、下一模型调用序号、原因、配置版本、分项、上限与来源哈希/输入保留量；来源投影与消息分项有重叠，不重复相加。初始/工具结果/裁决/预览及澄清原因由宿主已校验入口确定。错误保持原代码，增加阶段和前三项占用；超限不发送下一模型请求。旧显式开启的完整请求审计不替代此安全明细。P05 的结果有界读取、去重和继续空间预留，以及真实 Provider 标定尚未由本次实现关闭。

P05 实现登记（2026-09-21，验证状态见计划 §14.16）：同次装配中仅对有身份且完整序列化相同的 ReferencePayload 去重，跨来源、用途、不同内容或标签不合并；匿名 LongText 不去重，不建立跨轮缓存。`reference_projection` 只记录数量/原因。长文档默认目录至多 20 项/4,000 字符（整体条目），明确标注非全文；读取规模及版本分页见 I-P-01 §4.1.1。统一发送门禁保留原总额 24,000，将可用输入限制为 19,904，并记录 4,096 继续预留；不从旧历史字符预算重复扣减。此预留为估算容量裕量，不保证任意下一次结果都放得下；所有续轮仍重新核算。当前文档与 ToolResult、不同用途上下文不盲目合并，ToolCall/ToolResult 原始配对不裁掉。引用原有显式裁剪、累积结果仍可能触发预算失败；不承诺任意大块、无限历史或 Provider 精确 tokenizer。

> 本节细化 BR-AG-DATA-002 的 Prompt 可信边界，并由 BR-AG-DATA-004 的 IntentResult 与
> BR-AG-DATA-005 的分层预算共同约束，不新增平行规则。

Prompt Runtime 必须把系统约束与不可信数据物理分层：只有产品不变量、运行策略和经本轮 CapabilityGrant
允许的工具契约可进入 system instruction；当前文档、显式引用、历史、用户明确选择的项目记忆和工具结果必须封装为 data-only
PromptDataEnvelope，携 `source_kind/provenance/freshness/content_hash/trust_level/token_estimate`，不得因其
内容包含“忽略规则/调用工具/修改文件”等文字而获得指令权限。每次调用产出版本化 PromptPackage，
至少记录 `runtime_version/intent_revision/context_trace_id/tool_manifest_hash/estimated_tokens`；工具 manifest 必须是
当轮 `CapabilityGrant.allowed_tools` 的精确投影，不能暴露完整注册表或完整 RuntimeProfile；发送前校验总预算、
角色边界与 workspace 作用域。调试追踪默认只保存层元数据、哈希、选择/裁剪原因，不保存用户正文、密钥
或会话令牌。

成功只读 ToolResult 注入模型时必须携运行时生成的 evidence_id、source、coverage 和 version/hash；模型
不得自定义或改写这些身份。需要形成 ProjectReport 的最终输出使用 A-AG-I-P-01 定义的
ModelFinalEnvelopeV2，把 finding 关联到已见 evidence_ids；普通回答正文仍由 answer_markdown 承载。
运行时会验证并降级无效引用，该 envelope 不替代 EvidenceLedger，也不产生工具授权。

`agent-edit-1.0` 每次 provider 调用的装配顺序固定为：系统不变量与输出格式 → RuntimeControl → 当前
CapabilityGrant 工具投影 → 当前 IntentResult 投影（首次可为 `unresolved`）→ AgentContextFacts → 已激活的当前文档 data-only envelope（v2 可省略正文层）→ 显式引用 data-only envelope（可含用户明确
选择的项目记忆）→ 同文档必要历史 → 当前 user 原文。Prompt 对可编辑目标的声明只能来自当轮
ExplicitEditTargetSet，不能把已读路径、打开标签、搜索结果或完整 workspace 描述成可写目标。AST/KB/TPL
自动补强关闭；未来 Profile 启用必须新开需求/设计，不从本文推导。

**实现装配登记（2026-08-09，AG-PROMPT-STRUCTURE-FIX-01）**：生产 `LayerKind` 是 M-T-01 §5.1
BR-AG-DATA-002 的实现投影，具体层序只在该骨架真源中维护。本文只补充实现必须同时投影 BR-AG-DATA-004 的当前轮/历史处置与 BR-AG-DATA-005 的数据隔离/截断结果；
`RuntimeControl` 与自然语言提示词物理分层，`History` 保留 role 序列，预算 gate 按序列化 messages+tools 统一核算。任何实现枚举都不得反向成为新的文档真源。

## §6 当前两类回合结构示例（非规范说明）

> ⚠️ 本节为**设计示例**，帮助理解装配产物形态；其中文本为占位示意，**不构成被锁提示词文本**
> （文本归实现期运营配置，M-T-01 §5.1 已声明）。当前回合骨架差异权威源为 BR-AG-DATA-002。

**Chat 回合**（含历史 + 修改建议导向 diff）：
```
[系统层] 角色 + 硬约束(改经diff裁决/不越权落盘/workspace限界) + 格式约束
[工具层] 仅投影本次调用前由运行时计算的 CapabilityGrant.allowed_tools；示例不列完整注册工具
──缓存边界──
[代理上下文事实集] workspace + active/open docs + dirty/baseline/version + explicit selections + proposal states（data-only）
[当前文档] file + 文档块列表；富文档含 node_type/role/parent/depth/node_json/fingerprint/capabilities，
容器/原子块只作语境，内容块按稳定 blockId 独立寻址 + revision/baseline + pending diffs
[引用层] @mention: ref#1 精确锚点 block[..]→block[..] / ref#2 阅读上下文(仅理解)
[显式补充数据] 仅用户明确选择的项目记忆，可选；data-only，不覆盖当前文档/显式引用
messages[]: …历史(弱化/摘要)… → [当前对话] user 输入 (+ carryover 结果)
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

- **当前轮次原则**（BR-AG-DATA-004）：模型在无沿用样本中不把历史提升为当前目标；指代回指/修正追加
  样本可在 IntentResult revision 中表达明确沿用；运行时不通过关键词预先固定路线或工具集合。
- **补充数据隔离**（BR-AG-DATA-005）：仅用户当轮明确选择的项目记忆作 data-only 显式引用；未选记忆、知识库与模板的自动写入/检索/注入调用数为 0；超预算时按 P3→P1 的确定性阶梯处理，P0 不得静默截断。
- **分层映射一致性**：§2 各层来源与 M-T-01 §5.1 骨架层一一对应，无层越权直写文件（X-FORBID-006）。
- **本轮意图结果**（BR-AG-DATA-004）：复合、否定、目标不明与工具结果改变判断等样本由模型产生可修订
  IntentResult；首次可 unresolved；它不含 allowed_tools，CapabilityGrant 在相同语义下仍只由硬事实决定。
- **数据可信边界**（BR-AG-DATA-002/005）：含提示注入文字的文档/引用仍为 data-only；PromptPackage 可查
  runtime/version/hash/token/trace，超预算或 envelope 越出 workspace 时在网关调用前拒绝。
- **BDocx 改写路由与状态诚实性**：无表格、无图片的当前 BDocx 可提交完整 HTML `replace_document`；已有或
  新引入表格/图片时服务端返回结构化操作拒绝。`propose_edit` 工具结果必须只称建议已生成，并显式区分
  `pending_user_review`、未回报的编辑器预览和未保存的文件状态；前端预览 applied/failed 事件不记录正文。
- 提示词具体措辞不作为固定验收文本；预算数值属于运行时配置，但机制、明细对账、边界样本与真实 Provider 标定必须验证。
- P00 新增验证：未就绪无关文档不阻断问候；显式资源缺失不伪称已读；四种装配阶段预算可重放；工具结果
  占用计入同一口径；改变层序/数据可信边界或删除必需引用不得作为预算通过手段。

## §9 技术债务与盲区（诚实声明）

- **模型语义投影格式**（BR-AG-DATA-004 余项）：provider 间如何承载 IntentResult revision 可在实现期选择
  结构化响应或等价事件投影，但不得回退为关键词授权器，也不得暴露隐藏推理。
- **预算数值与门控阈值**：机制已锁，数值延 G5 标定。
- **展示层细节**（流式 diff 呈现）归 A-AG-I-P-01、DE 与前端，本文不展开。

## §10 后续设计入口

1. Prompt Runtime 的实现/验证只承接 A-AG-X-L-04；历史 PI-AG-07～09 不再作为当前计划源。
2. 预算和批次数字由运行时 capability/config 持有，验证 Prompt、工具 schema 与执行 guard 是否投影同一值。
3. 自动记忆/知识/模板与 Build 不在本文待办中；未来启用必须先进入新的 REQUIREMENT/DESIGN Scope。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-10-23 | v1.31 | AG-HISTORY-BUDGET-01：按连续对话单元保留近因、带标记原文摘录、按完整请求余量装配及实际测量，取消旧独立历史硬额度。 |
| 2026-10-23 | v1.30 | AG-BOUNDED-READING-01：正文分页与只读续行、按动态能力裁剪契约及重复提示说明；保留控制协议和实际授权。 |
| 2026-09-21 | v1.29 | P05 保守去重、有界读取及继续预留，明确残余边界。 |
| 2026-09-21 | v1.28 | P04 实现登记：统一发送投影与预算统计，追加安全分项证据；P05 与真实 Provider 标定保持待完成。 |
| 2026-09-15 | v1.27 | P00：区分 v2 按需激活与 P0 保留义务；定义四种装配阶段的预算证据、结果预留与失败关闭；专业指令不污染全局；生产接入待验证。 |
| 2026-09-03 | v1.26 | AG-ADAPTIVE-RUNTIME-DESIGN-01：当前轮语义改由模型在真实证据上形成和修订；IntentResult 移除 allowed_tools 与授权职责；Prompt 每次只投影当前 CapabilityGrant，澄清改为模型输出驱动。 |
| 2026-08-29 | v1.16 | AG-CONFORMANCE-CODE-S4A：新增独立 AgentContextFacts data-only 层，生产 LayerKind 顺序调整为 IntentHint→AgentContextFacts→CurrentDocument，并同步缓存边界与结构示例。 |
| 2026-08-31 | v1.25 | AG-DOC-CONSOLIDATION-01：提示词装配收敛到 Agent 1.0 两类当前调用，排除 Build 和自动 AST/KB/TPL 增强；与 M-T-01 按“骨架/当前轮/截断”分开唯一权威；tool manifest 仅投影当轮 CapabilityGrant.allowed_tools 且不得进入跨轮稳定缓存，预算/工具上限仅由运行时配置作数值真源。 |
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
