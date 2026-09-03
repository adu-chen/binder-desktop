---
文档编号：AG-X-R-01
文档状态：R
负责模块：AG
文档职责：Agent 提示词工程调研与参考（L4 只读参考）
上游约束：A-AG-M-T-01、A-AG-M-T-02、A-AG-X-L-04
直接承接：PI-AG-07~11 开发前调研依据（仅供参考，不约束实现）
使用边界：reference-only（ADU §2.1 L4），不定义 RULE/CHAIN/CONSTRAINT/TERM、不作为权威设计源；落地以 AG/DE Active 技术设计与 A-AG-X-L-04 为准
变更要求：调研增补可直接追加；若结论被采纳进设计，须在对应 M-T 文档落地并反向引用本文
信息来源：旧项目 /Users/chenzhenqiang/Desktop/test/binder-project（context_manager.rs、ai_providers/、tool_*、diffCardMachine.ts、diffStore.ts）+ 公开网络调研（见文末「信息源」）
---

# Binder Agent 提示词工程调研与参考（AG · 调研 / X-R）

> 阅读顺序建议：先看 §1 结论摘要 → 按三大关注点定位 §4（当前轮次原则）/§5（分层）/§6（diff 信息流）→ §8 落到 PI-AG-* 的具体建议。

---

## §1 调研结论摘要（Executive Summary）

| 关注点 | 一句话结论 | 详见 |
|---|---|---|
| 当前轮次对话原则 | 旧项目已用「Only act on the LAST user message」+ 知识检索门控落地此原则；研究文献（loss-in-middle-turns，多轮平均掉点 39%）为其提供强证据。binder 应把"当前轮为主需求、历史/参考/记忆默认为弱上下文、仅在显式判定沿用时才提权"做成**结构化的意图沿用判定**，而非靠一句提示词 | §4 |
| 多层提示词分层 | Claude Code 公开版本的系统提示词是 **23 段动态装配 + 缓存边界标记**；旧项目是 **7 层 PromptPackage + ContextLayer P0–P3 优先级**。binder 因引用/记忆/知识/模板/展示/格式等特有层，分层确实多于二者，需要一张**层级清单 + 每层的来源/优先级/截断序/污染隔离规则** | §5 |
| diff 交互与信息流 | 旧项目已有成熟的 **diffCardMachine（pending→working→applied/expired/rejected/applyRetryable）+ executionAnchor + revision/baseline 门禁 + 块列表(block_index)定位**；公开项目里 Aider 的编辑格式研究（diff 比 whole 显著降低"偷懒")与 Cline 的 DiffViewProvider（流式 diff 视图 + 逐行覆盖动画 + 审批工作流）是两条关键参考线 | §6 |
| 其他重要参考 | Aider（编辑格式可靠性基准）、Cline/Roo-Code（编辑器内 diff 流式呈现）、Continue（IDE 上下文供给）、void（旧项目已参考其简洁性）、"LLMs Get Lost in Multi-Turn Conversation" 与 "Lost in the Middle" 两篇文献 | §3 / §7 |

**与现状的衔接**：旧项目的 prompt 装配逻辑是单进程本地直连 provider（含 provider key）的产物；binder 受 X-FORBID-001（客户端永不持密钥）+ X-FORBID-006（产出唯经 AG→DE→WS 落盘）约束，调用链必须走云 GW（见 A-AG-X-L-04 / A-GW-M-D-01）。本文只保存调研背景，不参与当前装配裁决。

---

## §2 旧项目 binder-project 实现剖析

> 来源文件均已实读，行号引用基于旧仓库 `src-tauri/src/services/context_manager.rs`（1723 行）等。

### 2.1 七层语义 PromptPackage（核心可复用资产）

`context_manager.rs` 的 `build_prompt_package()` 把 prompt 装配为七个语义层（注释原文：`governance → task → conversation → fact → constraint → augmentation → tool_and_output`）：

| 层 key | 标题 | 内容 | 注入位置 |
|---|---|---|---|
| L1 governance | System Governance | 角色 + 行为准则 + 编辑规则 + 事实优先级 + 任务边界 + 响应风格 | system prompt |
| L2 task | Agent Task State | 当前 Agent 任务摘要 + 最近 artifacts（verification/confirmation） | system prompt |
| L3 conversation | （占位） | **不注入 system，由 messages 数组承载多轮历史** | messages[] |
| L4 fact | Document & Editor Context | 当前文档（分级注入）+ 块列表 + 选区/光标 + 工作区路径 + revision/baseline + pending diffs + 文件依赖 | system prompt |
| L5 constraint | Reference Materials | 用户显式引用（文件/选区/精确锚点四元组），带预算截断 | system prompt |
| L6 augmentation | Memory / Knowledge Augmentation | 记忆库 + 知识库自动检索切片（明确标注"仅补强，不覆盖当前文档/显式引用"） | system prompt |
| L7 tool_and_output | （占位） | 工具经 provider function-calling 注入，与 L1 工具指导互补 | provider tools[] |

**关键设计点（值得 binder 直接继承）**：
- L3（历史）刻意**不进 system prompt**，而是放进 `messages[]` 数组——把"事实/约束/补强"与"对话历史"在结构上分开，天然降低历史对当前事实判断的污染。
- L6 注入串里硬编码了优先级声明：`"以下内容来自知识库自动检索，仅作 augmentation 补强，不覆盖当前文档、显式引用或当前轮 artifact"`——这是**用提示词文字显式声明层级权重**的做法。

### 2.2 ContextLayer 优先级（P0–P3）与截断序

`context_manager.rs:421` 定义了一个独立的截断优先级枚举（数字越小越先保留）：

```
P0 永远保留 : SystemPrompt, UserMessage, EditTarget
P1 尽量保留 : CurrentDocument, DirectReferences, RecentHistory
P2 按预算   : MemoryLibrary, KnowledgeBase, TemplateLibrary
P3 按需注入 : WorkspaceDependencies, PendingDiffStatus
```

配套有 `RelevanceScorer` trait（当前是 `PassthroughScorer` 取前 5 条占位）和 `TruncationStrategy`（KeepRecent / SummarizeMiddle / KeepTaskGoal / LayeredPriority）。**这是一个已经留好接口、但相关性打分尚未实现的半成品**——binder 接入时这是一个明确的增强点（见 §8）。

### 2.3 分级文档内容注入（ContentInjectionStrategy）

`determine_injection_strategy()`（:390）按"是否有编辑目标 / 是否聚焦当前文档 / 内容字数 / 是否文件操作意图"在五种策略间选择：

- `SelectionContext`：有编辑锚点 → 注入选区 ±2 块的子列表
- `FocusedCurrentDocument`：聚焦当前文档且 ≥800 字 → 光标 ±3 块 + 选区 ±2 块 + **按用户消息 query terms 命中的块 ±1** 的并集
- `Full`：<800 字 → 全块列表
- `MetaOnly`：文件操作意图 → 只给文件名+字数，让模型按需 read_file
- `Summary`：其余 → 前 10 块 + 全部标题块（全文扫描意图则给全块）

**要点**：注入的不是裸 HTML，而是**带 `block_index`（0 起）的"文档块列表"**（`format_blocks_as_list`），并标注光标位置。这套块编号体系是后续 diff 精确定位（edit_mode + block_index + target text）的基础，直接服务于 §6 的 diff 生成。

### 2.4 知识检索门控（当前文档优先的核心机制）

`should_trigger_knowledge_retrieval()`（:682）用一串短路判定决定**是否允许把知识库混入当前轮**，每条都带可观测的 reason 枚举：

```
automatic_policy_blocked / no_automatic_candidates / query_too_short(<5字)
/ editing_context(有选区或锚点) / file_operation_intent
/ explicit_references_sufficient(已有显式引用)
/ current_scope_only("只基于当前文档") / current_document_priority(当前文档被显式引用/提及)
/ workspace_project_scope("项目里…") / current_document_sufficient
→ 否则 Triggered
```

**这正是用户关注点 #1 的工程化身**：默认不让外部知识污染当前轮，只有在"当前文档不足且用户表达了扩展意图"时才放行。binder 应把这套门控原则**从知识库一项推广到所有非当前轮来源（历史/记忆/模板/引用）**。

### 2.5 引用：精确锚点 vs 阅读上下文

`build_reference_prompt()`（:1239）对每条引用区分两种精度：
- **有四元组**（start_block_id/offset → end_block_id/offset）→ `[precise reference anchor]`，可作精确编辑目标；
- **无四元组的文本引用** → 明确标注 `reading context only`，提示模型"仅供理解，编辑须经块列表重新定位，不要把此引用当精确目标"。

这避免了"引用内容被误当成可直接落盘的编辑目标"的污染，是 binder REF→DE 衔接的重要原则。

### 2.6 Provider 抽象与工具层

- `ai_providers/mod.rs`：`AIProvider` trait（chat_stream 返回 `ChatChunk::{Text, ToolCall}` 流）+ OpenAI 兼容 `ChatMessage`（assistant 带 tool_calls 时 content 必须为 None）。只实现了 openai/deepseek，**anthropic 被注释掉**（`// pub mod anthropic`）——印证了 A-GW-M-D-01 里"Anthropic 需独立适配器"的判断。
- 工具层：`tool_matrix.rs`（主源）+ `tool_definitions.rs`（兼容入口）+ `tool_policy.rs`（`ToolCallBudget` 默认 max_tool_rounds=20 / max_force_continues=5；`AllowedDelegationScene` 多 agent 预留但未开放）。

### 2.7 Diff 卡片状态机（见 §6 详述）

`src/machines/diffCardMachine.ts`：`pending --ACCEPT--> working --> applied|expired|apply_retryable`，`CONTENT_CHANGED→expired`、`REJECT→rejected`。`diffStore.ts`（2122 行）承载 `DiffEntry`、`ExecutionAnchor`、revision/baseline 门禁与多种 `DiffExpireReason`。

---

## §3 公开项目调研

### 3.1 Claude Code 公开版本——动态装配的 23 段系统提示词

公开拆解（dbreunig，基于 2026-03 泄漏源）显示 Claude Code 的 system prompt **不是静态字符串，而是按条件动态装配的有序段落**，约 23 段，关键顺序与"常含/条件含"标注：

```
1 Intro(常)  2 System Rules(常)  3 Doing Tasks(条件)  4 Executing Actions with Care(常)
5 Using Your Tools(条件)  6 Tone and Style(常)  7 Output Efficiency(条件)
8 ★Cache Boundary Marker(条件——分隔"全局可缓存"与"会话特定"内容)
9 Session Guidance(多个条件块：agent 工具/技能发现/验证要求/shell 快捷)
10 Memory Prompt(条件，auto-memory 开启时)  11 Ant Override(条件)
12 Environment Info(常：工作目录/平台/模型/知识截止)  13 Language(条件)
14 Output Style(条件)  15 MCP Server Instructions(条件，每轮重算)
16 Scratchpad  17 Function Result Clearing  18 Summarize Tool Results(常)
19 Numeric Length Anchors  20 Token Budget  21 Brief  22 Git Status Snapshot(条件)
23 Append System Prompt(条件，用户 flag)
```

**对 binder 的三条直接启发**：
1. **缓存边界标记（#8）**：把"全局稳定内容（角色/规则/工具）"放在缓存边界之前，"会话特定内容（当前文档/选区/git 状态）"放在之后——这与 binder 想做的"当前轮为主"在**物理布局**上一致：稳定层在前、易变的当前轮事实在后（贴近 prompt 末尾，吃 recency bias 的正向收益，见 §4）。
2. **环境信息段（#12）+ git 状态段（#22）**：binder 的对偶是"当前文档/工作区/pending diffs/revision"——旧项目 L4 fact 层已覆盖。
3. **工具描述也是条件装配的**（约 50 个工具按 repl_mode/embedded_search/task_tool_enabled 等条件增减）——binder 工具远少（4 个起步），但"按模式（Chat/Build/Assist）启停工具集"的思路应保留（旧项目 enable_tools 已是雏形）。

### 3.2 Aider——编辑格式可靠性（diff 信息流的实证基准）

Aider 系统比较了多种"让模型输出编辑"的格式：

| 格式 | 模型输出什么 | 取舍 |
|---|---|---|
| whole | 整文件重写 | 最简单，但慢且贵，大文件浪费 token，易"偷懒"省略 |
| diff | `<<<<<<< SEARCH / ======= / >>>>>>> REPLACE` 块，路径在 fence 外 | 主流，只回改动段 |
| diff-fenced | 同 diff，但路径在 fence 内 | 为 Gemini 系（不守 fence 规范）准备 |
| udiff | 简化版 unified diff（-/+ 前缀） | 对 GPT-4 Turbo **降低"偷懒"3 倍**（26%→59%）；但对行号高度敏感 |
| editor-diff/whole | architect 模式下编辑模型的精简版 | 双模型协作：规划模型出意图，编辑模型只管语法正确 |

**对 binder 的启发**：binder **不应让模型直接产 unified diff**（行号敏感、易错），而应延续旧项目的 **"block_index + edit_mode + target text" 结构化编辑指令**——它本质是 SEARCH/REPLACE 的强约束变体（SEARCH=block 内 target 精确文本，REPLACE=content），由后端 DE 解析为 ChangeProposal，避免模型直接操纵行号。这与 Aider 结论一致：**结构化、锚定文本的编辑比裸 diff 可靠**。

### 3.3 Cline / Roo-Code——编辑器内 diff 的流式呈现

- **DiffViewProvider**：负责 VS Code 编辑器里的"流式 diff 效果"与一次编辑会话生命周期；`update()` 用 `WorkspaceEdit` 增量更新 diff 视图右侧，制造"AI 正在打字"的实时反馈。
- 两类工具：`write_to_file`（整文件创建/覆盖）vs `replace_in_file`（外科式 diff 编辑）；后者更高效、更不易误删、更易 review。
- **视觉反馈**：未处理的行用半透明覆盖层盖住，当前处理行高亮——明确的"信息流进度"可视化。
- 审批工作流：变更入 diff 视图 → 用户确认后才落盘。

**对 binder 的启发**（关注点 #3）：binder 的"含修改建议产出唯经 DE→WS、accepted 唯一触发落盘"（X-FORBID-006）天然就是审批工作流。可借鉴 Cline 的**流式 diff 呈现 + 逐块进度反馈**：模型 streaming 阶段产出 ChangeProposal 时，前端按块**逐步揭示 diff 卡片**（而非等整轮结束），并用"待确认/应用中/已应用/已过期"四态映射 diffCardMachine。

### 3.4 研究文献——多轮对话与"中间迷失"

- **Lost in the Middle**：模型对上下文呈 U 形利用曲线（首因+近因强，中间弱）；相关信息从首/尾移到中间，性能掉 30%+。
- **LLMs Get Lost in Multi-Turn Conversation**：把同一基准题分散到多轮，所有模型平均掉点 **39%**；"loss-in-middle-turns"——多轮里中间轮次同样被忽视。
- **上下文污染**：模型在信息不全时的早期错误回答会留在上下文里**毒化后续推理**；历史/工具输出/检索数据堆积会把系统提示词挤出有效注意力，造成行为漂移。

**这是关注点 #1 的学术支撑**：历史轮次默认是"噪声源"，当前轮才是信号。工程上应（a）把当前轮事实放在 prompt 末尾吃 recency；（b）历史默认弱化/摘要化；（c）只在显式判定"需沿用历史意图"时才把那部分历史**提权重述**到当前轮附近，而非整段堆叠。

---

## §4 当前轮次对话原则（关注点 #1，深入）

### 4.1 原则陈述

> **当前轮用户消息 = 本轮主需求（信号源）。历史轮、参考、记忆、知识、模板默认是弱上下文（潜在噪声），不得自动凌驾当前轮事实判断。只有当分析识别出"用户在本轮显式沿用/调用/选择历史意图"时，才把对应历史意图提权并重述到当前轮上下文。**

### 4.2 旧项目已有的落地（可继承）

1. system prompt 末句：`Only act on the LAST user message. Previous messages are completed history.`（Chat 与 Agent 两模式都有）。
2. 历史进 `messages[]` 而非 system prompt（结构隔离，§2.1 L3）。
3. 知识检索门控（§2.4）：当前文档优先、显式引用充分则不触发外部检索。
4. 记忆/知识注入串显式声明"仅补强，不覆盖当前文档/显式引用/当前轮 artifact"（§2.1 L6）。

### 4.3 缺口与 binder 增强建议

旧项目的"当前轮原则"主要靠**提示词文字 + 知识门控**，对"历史意图沿用"缺少**结构化判定**。建议 binder 补一层 **意图沿用判定（intent-carryover decision）**，置于 build_turn_context 内、装配 prompt 之前：

```
输入：当前轮 user_message + 最近 N 轮历史摘要 + 当前选区/锚点/引用
判定维度（任一命中才允许引入对应历史意图，否则历史仅作弱背景）：
  1. 指代回指    —— "继续"/"接着上面"/"还是那个"/"它"/无主语祈使 等 → 需要解析上一轮目标
  2. 显式选择    —— "用第二个方案"/"按刚才说的" → 沿用上一轮产出
  3. 任务延续    —— 当前轮是上一轮 Agent 任务（agent_task_summary active）的下一步
  4. 修正追加    —— "再短一点"/"把刚才那段改成…" → 操作对象=上一轮 diff/选区
输出：carryover = { none | reference_target(turn_id, anchor) | continue_task(task_id) | refine(diff_id) }
```

- carryover=none 时：历史不进 prompt 事实层，仅保留极简近因摘要（防止完全失忆但不污染）。
- carryover≠none 时：把被沿用的那一段历史意图**重述为一条当前轮上下文行**（贴近 prompt 末尾），而非把整段历史原样堆进去——既满足 recency，又避免中间迷失。

> 与治理的关系：此判定输出可作为 SM-AG-01 turn 装配阶段的输入，落地时建议在 A-AG-M-T-01 增设一条 assemble 前置规则（本文不注册规则，仅建议）。

### 4.4 反面清单（binder 不应做的）

- ❌ 把全部历史轮原样拼进 system prompt（中间迷失 + 污染）。
- ❌ 让记忆/知识检索在无扩展意图时默认注入并与当前文档同权。
- ❌ 把"无四元组的阅读上下文引用"当作精确编辑目标。
- ❌ 历史轮里模型的早期错误结论不清理就继续（污染累积）。

---

## §5 Binder 多层提示词工程分层（关注点 #2）

用户判断"binder 分层可能多于原版 Claude Code"——**成立**。Claude Code 23 段主要是"行为/环境/工具/会话"四类；binder 因文档化场景额外引入引用/记忆/知识/模板/展示/格式等**内容供给层**。下表是面向 binder 的完整分层清单（合并旧项目七层 + Claude Code 段落 + binder 特有层），每层标注 **来源模块 / 优先级 / 截断序 / 污染隔离规则**。

| # | 层 | 内容 | 来源模块 | 优先级 | 注入位置 | 污染隔离规则 |
|---|---|---|---|---|---|---|
| 1 | 治理/系统层 | 角色、行为准则、安全与边界、模式（Chat/Build/Assist）规则 | AG | P0 永远保留 | system 头，缓存边界**前** | 稳定不变，进 prompt cache |
| 2 | 工具层 | 工具定义 + 何时调用 vs 直接回复 | AG/各工具 | P0 | provider tools[] + system 指导 | 按模式启停工具集 |
| 3 | 格式层 | 输出格式约束（纯文本/HTML/docx 规则、禁 markdown 符号、块编号约定） | DOC/DE | P0 | system 头 | 与正文分离，防止格式说明被当内容 |
| 4 | 任务状态层 | 当前 Agent 任务 + 阶段 + artifacts（verification/confirmation） | AG | P1 | system，缓存边界后 | 只注 active 任务 |
| 5 | 当前文档/编辑器交互层 | 当前文件 + 分级块列表 + 选区/光标 + revision/baseline + pending diffs + 文件依赖 | WS/ED/DE | **P0 当前轮主事实** | system 尾，**贴近末尾吃 recency** | 当前轮唯一高权事实源 |
| 6 | 引用层 | 用户显式引用（文件/选区/精确四元组锚点 vs 阅读上下文） | REF | P1 | system | 精确锚点可编辑/阅读上下文仅理解；预算截断 |
| 7 | 当前对话层 | 当前轮 user_message（+ 意图沿用判定结果） | AG | P0 | messages[] 末条 | 主需求，信号源 |
| 8 | 历史对话层 | 既往轮次 | AG | P1/弱 | messages[]（**不进 system**） | 默认弱化；仅 carryover≠none 时提权重述（§4.3） |
| 9 | 记忆库层 | 长期记忆检索结果 | AST/记忆 | P2 按预算 | system，标注"仅补强" | 不覆盖当前文档/显式引用/当前轮 artifact |
| 10 | 知识库层 | 知识库自动检索切片（带 citation/provenance/risk_flags） | KB | P2 | system，标注"仅补强" | 门控触发（§2.4）；结构化保真到最终消费 |
| 11 | 模板库层 | 编排/工作流约束引用（compiled workflow constraint） | TPL | P2 | system/引用 | 作为约束而非内容；与事实分离 |
| 12 | 增强/补强层（聚合） | L9–L11 的统一"augmentation"语义包 | — | P2 | system | 整体低于 P0/P1，超预算先截 |
| 13 | 工作区依赖层 | 文件依赖图、pending diff 状态 | WS | P3 按需 | system 尾 | 仅在相关时注入 |
| 14 | 展示层 | 面向用户的呈现（流式 chunk / 工具卡片 / diff 卡片）——**输出侧而非输入侧** | AG/前端 | — | StreamEvent → 前端 | 与模型输入解耦；见 §6 |

**分层使用准则（综合 Claude Code 缓存边界 + 旧项目 P0–P3 + 文献 recency）**：
- **物理顺序**：稳定层（1/2/3）在前并进 prompt cache；当前轮易变事实（5/7）尽量靠 prompt 末尾（recency 正向）；补强层（9–11）夹在中间但显式声明低权（接受"中间位置"代价，因为它们本就低优先级）。
- **截断序**：超预算时从 P3→P2→P1 逐层截，P0 永不截（旧项目 ContextLayer 已建模，缺相关性打分，见 §8）。
- **每层一句"权重声明"**：补强层注入串须自带"不覆盖当前文档/显式引用"的文字声明（旧项目已做，保留）。

---

## §6 Diff 交互生成与信息流（关注点 #3）

### 6.1 端到端信息流（模型产出 → 用户落盘）

```
模型 streaming
  │  产出结构化编辑指令（非裸 diff）：
  │  { edit_mode: replace|delete|insert|rewrite_block|rewrite_document,
  │    block_index:N, target:<块内精确明文>, content:<新文本>, occurrence_index? }
  ▼
DE 解析 → ChangeProposal → 定位（block_index/锚点四元组）→ PendingDiff(payload)
  │  绑定 executionAnchor { startBlockId, startOffset, endBlockId, endOffset, baselineId, revision }
  ▼
前端 diff 卡片（流式逐块揭示，参考 Cline DiffViewProvider 的渐进呈现）
  │  状态：pending
  ▼
用户 ACCEPT ──► working（buildAcceptReadRow 校验 revision/anchor）
  │                ├─ 校验失败 → expired（reason: revision_mismatch/advanced/anchor_lost…）
  │                ├─ 应用失败 → applyRetryable（DiffRetryController）
  │                └─ 成功 → applied（accept → ws.apply_diff 真落盘，X-FORBID-006 唯一触发）
  ├─ 文档被改 → CONTENT_CHANGED → expired
  └─ REJECT → rejected
```

### 6.2 关键设计点（来自旧项目，建议 binder 继承）

1. **不让模型产裸 diff**：用 block_index + target 明文做"锚定式 SEARCH/REPLACE"（§3.2 Aider 结论支撑），由 DE 负责生成真正的 diff，模型只表达"在哪个块、改哪段文字、改成什么"。
2. **executionAnchor 是唯一执行真源**：accept / 红字删除 / 重定位都从 anchor 出发（diffStore.ts 注释原文："当前唯一执行真源"）。
3. **revision/baseline 门禁**：accept 前校验 tab 文档版本，版本前移/不匹配 → expired，防止"基于旧快照的 diff 落到新文档"——这是信息流一致性的核心闸门。
4. **过期原因可观测**：`DiffExpireReason`（document_revision_mismatch / advanced / anchor_lost …）+ `ExecutionExposure`（phase: route/resolve/validate/apply/refresh）让 diff 失败可定位、可向用户解释，而非静默失败。
5. **多块编辑 = 多次单块调用**：system prompt 明确"每块调用一次工具，绝不用 rewrite_document 做多块任务"——降低单次大改的错配风险（与 Aider"高层 hunk 更易误匹配"的取舍呼应，binder 选了更细粒度）。

### 6.3 流式呈现建议（结合 Cline）

- 历史建议曾讨论 ChangeProposal 的流式揭示；当前事件与 Diff 投影只以 A-AG-I-P-01 / DE Active 契约为准。
- 卡片四态 UI 直接映射 diffCardMachine：pending（待确认）/ working（应用中）/ applied / expired|rejected。
- 可选：借鉴 Cline 的"未处理行半透明覆盖 + 当前行高亮"，但 binder 是块级而非行级，建议做**块级高亮 + 进度计数**（第 k/n 块）。

---

## §7 其他重要参考项目（关注点 #4）

| 项目 | 为什么值得参考 | binder 可借鉴点 |
|---|---|---|
| **Aider** | 编辑格式可靠性的实证基准（whole/diff/udiff 对比） | 锚定式结构化编辑优于裸 diff；编辑格式按模型适配 |
| **Cline / Roo-Code** | 编辑器内 diff 的流式呈现 + 审批工作流 + DiffViewProvider | 流式逐块揭示、四态卡片、确认后落盘 |
| **Claude Code 公开版** | 23 段动态装配 + 缓存边界 + 条件化工具 | 缓存边界布局、按模式启停工具、环境/git 状态段 |
| **Continue.dev** | IDE 上下文供给（@file/@code/@docs 等 context provider） | 引用层的 context provider 抽象（binder REF 已有雏形） |
| **void editor** | 旧项目已显式参考其"简洁性"（context_manager 注释） | system prompt 简洁化、突出文档助手定位 |
| **OpenHands / SWE-agent** | agentic loop 与工具预算/续轮策略 | tool_policy 的 max_tool_rounds/force_continue 预算 |
| 文献：Lost in the Middle | U 形注意力曲线 | 当前轮事实贴末尾、补强层接受中间低权 |
| 文献：LLMs Get Lost in Multi-Turn | 多轮掉点 39%、loss-in-middle-turns | 历史默认弱化、意图沿用才提权（§4） |

---

## §8 对 binder PI-AG-07~11 的具体参考建议

> 历史“调研 → 计划”落点已过期；当前计划只见 A-AG-X-L-04。

| PI | 调研给出的具体建议 |
|---|---|
| **PI-AG-07 系统提示词** | 三模式常量直接参考旧项目 `build_base_system_prompt` 的 Chat/Agent 文案（含"Only act on the LAST user message""事实优先级三级""纯文本/禁 markdown""多块=多次调用"）。按 §3.1 把稳定内容放缓存边界前。 |
| **PI-AG-08 工具注册/执行** | 工具描述参考 tool_matrix（read_doc/list_docs/search_text/propose_edit）。propose_edit 的参数采用 §6.1 的 block_index+edit_mode+target 结构（而非裸 diff）。保留 ToolCallBudget（max_tool_rounds 限额）防失控续轮。 |
| **PI-AG-09 上下文供给** | `build_turn_context` 直接移植旧项目分级注入（ContentInjectionStrategy）+ 块列表(format_blocks_as_list) + 知识门控(should_trigger_knowledge_retrieval)。**新增**：§4.3 的意图沿用判定（旧项目缺）。**补全**：ContextLayer 的 RelevanceScorer（旧项目是 PassthroughScorer 占位）——这是明确的半成品增强点。 |
| **PI-AG-10 流式 IPC** | StreamEvent 增加 DiffProposed 的逐块揭示语义（§6.3）。展示层与输入层解耦（§5 L14）。 |
| **PI-AG-11 真实模型调用** | prompt 装配逻辑可复用旧项目七层，但调用链改走云 GW（X-FORBID-001/006）。provider 侧 anthropic 适配器是旧项目所缺（被注释），由 A-GW-M-D-01 PI-GW-01 补。 |

### 8.1 调研暴露的待决/风险项

| 编号 | 类型 | 说明 |
|---|---|---|
| R-01 | 半成品继承 | 旧项目 RelevanceScorer 仅 Passthrough；binder 若直接移植需补相关性打分，否则"按预算选 TopK"形同虚设 |
| R-02 | 设计待定 | 意图沿用判定（§4.3）是新增机制，需在 A-AG-M-T-01 落地为 assemble 前置规则后再实现 |
| R-03 | 格式一致性 | docx 编辑需 HTML 与 file_cache/TipTap 一致（旧项目踩过，见 context_manager:1213 的 DOCX 注释）——binder DE 落盘须沿用 |
| R-04 | 边界 | 多 agent delegation（tool_policy AllowedDelegationScene）旧项目预留未开放；binder 起步同样不开放，仅留接口 |

---

## 信息源（Sources）

**旧项目（实读）**：`/Users/chenzhenqiang/Desktop/test/binder-project`
- `src-tauri/src/services/context_manager.rs`、`ai_providers/{mod,openai,deepseek}.rs`、`tool_{matrix,policy,definitions}.rs`、`conversation_manager.rs`
- `src/machines/diffCardMachine.ts`、`src/stores/diffStore.ts`、`src/services/{DiffActionService,DiffRetryController}.ts`、`src/types/agent.ts`

**公开网络调研**：
- [How Claude Code Builds a System Prompt — dbreunig](https://www.dbreunig.com/2026/04/04/how-claude-code-builds-a-system-prompt.html)
- [system_prompts_leaks (asgeirtj) — GitHub](https://github.com/asgeirtj/system_prompts_leaks)
- [Aider — Edit formats](https://aider.chat/docs/more/edit-formats.html)、[Aider — Unified diffs make GPT-4 Turbo 3X less lazy](https://aider.chat/docs/unified-diffs.html)
- [Roo-Code File Operations and Diff System — DeepWiki](https://deepwiki.com/RooVetGit/Roo-Code/4.3-file-operations-and-diff-system)、[Cline — Improving Diff Edits by 10%](https://cline.bot/blog/improving-diff-edits-by-10)、[Cline — System Prompt Advanced](https://cline.bot/blog/system-prompt-advanced)
- [Lost in the Middle: How Language Models Use Long Contexts — TACL](https://direct.mit.edu/tacl/article/doi/10.1162/tacl_a_00638/119630/)、[LLMs Get Lost In Multi-Turn Conversation — arXiv 2505.06120](https://arxiv.org/pdf/2505.06120)

---

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-24 | v1.0 | 初版调研：旧项目 binder-project 提示词工程剖析（七层 PromptPackage / P0–P3 / 分级注入 / 知识门控 / 引用精度 / diff 状态机）；公开项目调研（Claude Code 23 段装配 + 缓存边界、Aider 编辑格式、Cline DiffViewProvider、多轮对话文献）；三大关注点专章（§4 当前轮次原则 / §5 多层分层清单 / §6 diff 信息流）；§8 落到 PI-AG-07~11 的参考建议与风险项 |
