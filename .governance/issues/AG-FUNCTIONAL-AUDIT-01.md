---
文档编号：AG-FUNCTIONAL-AUDIT-01
文档状态：A
负责模块：AG
文档职责：AG 代理功能全链路审计报告——对需求文档、设计文档、代码实现进行逐功能模块对照审计；目标是识别可独立推进、独立审计、独立验证、独立固定的模块，实现 agent 功能的结构化拆分，支撑阶段化开发
审计范围：AG 模块十大功能审计面（工具调用/内容生成/编辑能力/引用协同/编辑器协同/提示词工程/工作策略/场景能力/上下文处理/意图识别）；代理构建（BuildMode/CBT）不在本阶段（用户明确为后续版本功能）
审计结论：本轮只审计，不修改代码
上游约束：A-AG-M-D-01（REQ-AG-001~006）、A-AG-M-T-01（BR-AG-* 规则 SSOT）、A-AG-M-T-02（提示词架构）、A-AG-M-S-01（SM-AG-01/02/03）、A-AG-I-P-01（接口协议）、A-AG-X-L-01/02（开发计划）、A-REF-M-T-01、A-SYS-I-P-03/04、A-SYS-M-S-01
治理成熟度：draft
---

# Binder AG 代理功能全链路审计报告

## §0 审计概览

### 0.1 审计目的与方法

按人类指示，对 AG（代理）功能进行全链路审计：**明确需求 → 对照设计 → 分析代码实现**。
审计面按高交叉的 agent 能力拆分为 10 个功能审计维度，逐一核对「需求文档锚点 / 设计规则锚点 /
实现位置 / 实现程度 / 审计发现 / 独立性评估」。最终产出可独立推进的模块化拆分方案。

审计方法：文档层完整读取（AG 六份 L1–L3 文档）+ 代码层逐文件读取（后端 `src-tauri/src/ag/*`、
`lib.rs` 接线、`ent/gateway.rs`；前端 `agService/agStore/assistStore` 与 `components/ag/*`；
REF 模块 `reference/*`）+ 测试覆盖核对（`agRuntime.test.ts` / `assistRuntime.test.ts` /
`AgentUI.test.tsx` / 各 Rust 单测 / `core_chain.rs`）+ 治理状态核对（`active_scope.json`、
Issue Trace）。

### 0.2 总体结论（摘要）

1. **文档-实现整体高度对齐**：AG 是项目治理成熟度最高的模块之一。L1–L3 六份文档已回写至
   2026-08-07（最新一轮 AG-MSG-RENDER-01），规则链 REQ→REQ_MAP→RULE/CHAIN→@GOV→测试 基本闭合，
   `governance:audit` 为硬门且当前 Scope（AG-MSG-RENDER-01，状态 COMPLETE）已过。
2. **实现已越过"骨架"到达"真实"阶段**：PI-AG-07~12 大体完成——四类提示词正文、12 工具注册与真实执行、
   上下文组装（历史分流/意图沿用/分层截断）、`ag_start_turn` 真实流式（dev GW）、辅助续写/局部修改
   完整链路、会话历史持久化（三表 + 懒加载 + 软归档）均已接线；前端消息流按回合内时序渲染。
3. **十大审计面中 8 项可独立推进**（详见 §5 拆分方案）；「编辑器协同」与「工作策略」因跨模块最深
   建议作为契约锚点/编排层审计，而非独立开发模块。
4. **主要未闭合点**（详见 §4 审计发现清单）：
   - **D1（需人类裁决）**：`create_doc` 新建文档"直接创建不经过 DiffCard"与
     BR-AG-DATA-003「新建以空基准表达、经 DE diff 裁决闸门」的语义张力；
   - **D2（需人类裁决）**：open_session 草稿"自动应用但不自动确认"（D1 审计修复痕迹）与
     I-P-01「立即执行自动应用事务并禁用卡片裁决」的语义对齐；
   - **D3（需复核）**：提示词分层实现为 M-T-02 13 层的缩减版（任务状态层/模板层/工作区依赖层未接入），
     ContextSource 优先级阶梯与 BR-AG-DATA-005 P0–P3 需建立显式映射表防漂移；
   - **D4（需复核）**：`turn.rs` 旧表持久化（`persist_turn`/`recover_turns`）与 `history.rs` 新表
     是否存在双轨/遗留代码（仅审计记录，未确认调用方）；
   - **D5（低风险）**：`verify_anchor`（BR-REF-VERIFY-001）实现位置与 AG 消费路径的显式调用关系
     需核对（selection 失效不作为当前事实已有测试覆盖）。

### 0.3 本报告范围外

- 代理构建（BuildMode / CBT）：用户明确不在本阶段，本报告仅在涉及移交/回落边界时引用。
- DE/ED/WS/REF 模块内部实现细节：仅审计与 AG 的接口契约侧。
- 云 GW（A-GW-M-D-01）内部实现：仅审计客户端消费契约（门禁/流式/错误形状）。

## §1 现状总览

### 1.1 文档层成熟度（L1–L3）

| 文档 | 版本 | 状态 | 内容 |
|---|---|---|---|
| A-AG-M-D-01 代理功能需求描述 | v2.15 | A | REQ-AG-001~006；模块边界（IN/OUT）；代表性流程 A1–A3 |
| A-AG-M-T-01 代理技术设计 | v2.26 | A | 规则 SSOT：AG-TURN/AG-TOOL/AG-ASSIST/AG-MODEL-CATALOG 链路，BR-AG-* 规则，X-FORBID-006，TERM-AG-001~011 |
| A-AG-M-T-02 提示词架构设计 | v1.13 | A | 13 层分层总览、装配顺序与缓存边界、BR-AG-DATA-004/005、TERM-AG-005 |
| A-AG-M-S-01 代理状态机设计 | v2.10 | A | SM-AG-01（回合机）/ SM-AG-02（工具循环机）/ SM-AG-03（辅助回合机），APC §6 必备字段齐备 |
| A-AG-I-P-01 代理接口协议 | v1.31 | A | AgTurnEvent 六分支、ag_start_turn 签名、辅助接口、BDocx 语义工具、历史恢复接口 |
| A-AG-X-L-01/02 开发计划 | — | A | PI-AG-01~06（骨架，G4）/ PI-AG-07~12（集成） |

### 1.2 代码层实现状态

- **后端 Rust**：`ag/{turn,context,tool,tool_registry,tool_exec,assist,prompts,history,memory_link,
  blocks,block_ops,bdocx_capabilities}.rs` 全量实现；`lib.rs` 提供 `ag_start_turn`（async + Channel）、
  `ag_request_assist`、`ag_finish_assist` 三独立命令 + 会话/历史 dispatch 命令；
  `ent/gateway.rs` 提供门禁/流式/辅助调用（本地 dev GW）。
- **前端 TS**：`agService.ts`（Channel 订阅、OpenDocSnapshot 采集、历史恢复、回合发起）、
  `agStore.ts`（会话/消息/工具卡投影）、`assistStore.ts`（辅助视图态 + 防抖）、
  `components/ag/*`（AgentPanel/Composer/MessageList/MessageFlow/MessageMarkdown/ToolCallCard/
  AutocompleteOverlay/InlineAssistPopover/ModelConfig）。
- **引用侧**：`reference/{resolve,inject,context}.rs` + 前端 `referencePayload.ts`（8 种 payload 类型
  归一化）+ `Composer.tsx` referenceToken 内联原子 + 拖拽/粘贴。

### 1.3 测试覆盖

- Rust：各 ag 模块内嵌单测（状态机转移矩阵、门禁无绕过、上下文裁剪确定性、意图沿用、历史持久化、
  payload additive 兼容、块解析/样式投影/身份守卫），`core_chain.rs` 集成。
- 前端：`agRuntime.test.ts`（20+ 用例：OpenDocSnapshot 采集/历史恢复/ReferencePayload/assist 契约）、
  `assistRuntime.test.ts`、`AgentUI.test.tsx`（MessageMarkdown/MessageFlow 渲染 8 例）。
- 测试治理：AG 登记为 critical 模块，`test:related` 联动 SYS/Rust（`requires_rust_on_related`）。

### 1.4 治理状态

- 当前活跃 Scope：AG-MSG-RENDER-01（状态 COMPLETE，confirmation YES）。
- 历史 Issue Trace：PI-AG-WIRE-01 等留痕完整；dev 门禁桩=Passed（真实裁决归正式 GW，X-FORBID-002 延后）。

## §2 十大功能审计面逐一审计

### 2.1 工具调用模块（agent 具备哪些能力、调用哪些工具）

| 维度 | 内容 |
|---|---|
| 需求锚点 | REQ-AG-002（agent 工具调用回合：注册/权限/上限/回注；内容写入走 DE diff 不得旁路） |
| 设计锚点 | AG-TOOL 链路（P1）、BR-AG-STATE-002（校验机制+写盘禁令）、SM-AG-02（工具循环机）、X-FORBID-006、BDocx 受控链（list_tables→table_ops / list_images→image_ops / block_ops） |
| 实现位置 | 后端：`tool.rs`（SM-AG-02 转移+validate_tool）、`tool_registry.rs`（12 工具注册+权限分级+OpenAI schema 生成）、`tool_exec.rs`（只读/结构工具执行）、`blocks.rs`（AgentReadContext）、`block_ops.rs`（结构化块校验）、`bdocx_capabilities.rs`（manifest 投影）；接线：`lib.rs` ag_start_turn 工具循环（MAX_ROUNDS=6）；前端：`agStore` 工具卡投影、`agService` tool_request/tool_result 事件 |
| 实现程度 | **完整**。12 工具：只读 6（read_doc/list_docs/search_text/list_blocks/list_tables/list_images）、结构 4（move_doc/create_folder/import_image_asset/create_doc）、写意图 2（propose_edit/edit_blocked）；删除类工具按要求未注册（等确认 UI）。校验链 validate_requested_tool→validate_tool 完整；schema 从 manifest 动态投影（不手写第二套） |
| 审计发现 | ①工具集较设计 v1 的 4 工具已按 2026-07 系列扩展至 12，文档 v2.26 已同步，无漂移；②循环上限 6（dev 值，延 G5 标定，设计已声明）；③`create_doc` 新建直写与「新建经 diff 裁决」的张力见 D1 |
| 独立性评估 | **A 级可独立**。后端注册表/执行器/状态机与 lib.rs 循环接线之间只有工具名+参数契约，schema 由 manifest 单一来源，可独立审计/验证/固定 |

### 2.2 内容生成模块（生成内容、生成工具、生成命令、diff 卡的生成）

| 维度 | 内容 |
|---|---|
| 需求锚点 | REQ-AG-001（修改建议产出）、REQ-AG-002（工具产出）、REQ-AG-005（辅助续写 3 候选 / 局部修改单候选） |
| 设计锚点 | BR-AG-STATE-001（产出唯一出口交 DE）、AG-TURN HandingOff、DiffProjection 契约（I-P-01）、autocomplete JSON 三候选契约（M-T-02 §6） |
| 实现位置 | 后端：`assist.rs`（parse_autocomplete_candidates 容错解析：纯 JSON 数组/fenced/对象包装/编号行）、`prompts.rs`（四类 SYSTEM_PROMPT 约束输出形态）、`lib.rs` ag_propose_edit（DiffProjection 生成）、`de/generate.rs`；前端：`agService` parseDiffProjectionResult、`assistStore`、AutocompleteOverlay/InlineAssistPopover |
| 实现程度 | **完整**。正文生成（GFM Markdown 子集+无 Mermaid 声明）、工具产出（propose_edit→DiffProjection）、辅助候选（严格 3 候选 + parse_underflow 空态 + failure_reason 分型穿透）三形态均实现 |
| 审计发现 | ①候选解析容错（JSON/fenced/对象/编号行 4 类偏差）与 v2.9 契约逐条对齐；②failure_reason 从后端→service→assistStore→overlay 链路无吞点（invoke_error 映射明确）；③DiffProjection 生成后由 tool_result 权威返回、前端禁止从 tool_request 重建 before/after——符合 v1.11 契约 |
| 独立性评估 | **B 级**。内容生成强依赖 DiffProjection 契约（与 DE/ED 冻结）与模型调用；但 assist 候选解析为纯函数可独立测试。建议与 2.3 编辑能力合并为一个「产出面」 |

### 2.3 编辑能力（如何编辑编辑器文档）

| 维度 | 内容 |
|---|---|
| 需求锚点 | REQ-AG-002（写意图走 DE diff；block_ops/table_ops/image_ops 受控链） |
| 设计锚点 | X-FORBID-006（唯一落盘路径）、BR-AG-STATE-002（只读经 AgentReadContext）、BR-AG-DATA-003（目标集守卫）、AG-RICH-BLOCK-STRUCTURED-EDIT-01 |
| 实现位置 | 后端：`lib.rs` ag_propose_edit（propose_edit 专用路径→de.generate_diff→persist_diff）、`blocks.rs`（replace_block/replace_block_by_id/rewrite_preserves_document_identity 身份守卫）、`block_ops.rs`（validate_and_build 批校验 + beforeNode 补齐）；前端：`deService`（acceptDiff/rejectDiff/applyOpenSessionDiff）、`deStore`、ED 草稿链（accept_draft_diff） |
| 实现程度 | **完整**。四种 patch_kind 全支持：replace_block（纯文本）、block_ops（富文档结构化，container/atom 只读、capabilities=replace_node 门）、style_ops（真实 mark/attrs 校验）、table_ops/image_ops（活动会话五重校验）。富文档拒绝 replace_document（AG_RICH_BLOCK_OPERATION_REQUIRED）、整篇替换身份守卫实现 |
| 审计发现 | ①编辑能力实现已远超骨架，结构化校验链（fingerprint/expectedNodeType/node_json 受控 Schema）完整；②create_doc 直写新建与 diff 裁决闸门张力见 D1；③open_session 应用语义见 D2 |
| 独立性评估 | **A 级可独立**（与 DE 以 DiffProjection 契约冻结为锚）。编辑产出链 = AG 提案 → DE 裁决 → ED/WS 落盘，契约边界清晰 |

### 2.4 引用协同（引用内容如何注入、如何准确获取引用内容）

| 维度 | 内容 |
|---|---|
| 需求锚点 | REQ-AG-004（REF 注入消费）、REQ-CORE-006 |
| 设计锚点 | BR-REF-DATA-001/002/003（解析/锚校验/载荷组装）、ReferencePayload 主通道（I-P-01 v1.6，8 种 kind）、BR-AG-DATA-001（显式引用优先保留不静默丢弃） |
| 实现位置 | 后端：`reference/{resolve,inject,context}.rs`（resolve_mention/verify_anchor/build_injection）、`lib.rs` ag_start_turn 内组装（mentions/block_refs/references 三通道，MENTION_CAP=4000 截断但保留）；前端：`referencePayload.ts`（归一化+校验）、`Composer.tsx`（referenceToken 内联原子、@mention 触发、拖拽/粘贴）、`referenceDrag.ts` |
| 实现程度 | **完整**。8 种 payload（doc/selection/session/memory/knowledge/template/url/long_text）+ file 兼容；整篇/块级/选区引用注入；「发送时按 AgentReadContext 校验 selection，失效不作当前事实」有实现与测试（agRuntime.test.ts） |
| 审计发现 | ①`verify_anchor`（BR-REF-VERIFY-001）函数存在但 AG 消费路径（format_reference_payload_context）是否显式复用需核对（D5，低风险）；②引用层 P1 语义在 lib.rs 体现为「截断（cap）但不裁剪」，符合「显式引用优先保留」；③url 引用明确声明"不代表已抓取网页内容"——无 WebFetch/抓取能力，与设计一致（写作域不承诺网页抓取） |
| 独立性评估 | **A 级可独立**。REF 模块本身独立 owner；AG 消费路径窄（ag_start_turn 注入段 + Composer），前端 referencePayload 为纯 TS 可独立测试 |

### 2.5 编辑器协同（编辑器缓存、如何配合编辑、如何展示 diff、后续业务链）

| 维度 | 内容 |
|---|---|
| 需求锚点 | REQ-AG-001（DiffCard 时序内嵌）、REQ-AG-005（装饰层呈现）、REQ-AG-002（草稿链） |
| 设计锚点 | BR-AG-STATE-001（DiffCard/ToolCallCard 按事件时序内嵌）、draft_mode 三态（current_editor/open_session/background，I-P-01）、ED-DECOR（装饰层）、TERM-AG-011 消息片段 |
| 实现位置 | 后端：DiffProjection.draft_mode 字段、`de/generate.rs`；前端：`agService`（open_doc_snapshots 采集、diff_proposed 投影）、`deStore.addDiff`、`deService.applyOpenSessionDiff`（ED session 草稿应用）、`edStore`（EditSession/LogicalState/版本哈希缓存）、`MessageFlow.tsx`（segments 时序交错渲染 DiffCard） |
| 实现程度 | **完整但语义需复核**。编辑器缓存（LogicalState + sessionVersion/contentHash + binding 对齐 + active_editor_session 降级）实现完整；消息片段时序（text/tool/diff 交错）已落地（AG-MSG-RENDER-01）；draft_mode 三态分流实现 |
| 审计发现 | ①open_session 的「自动应用但不自动确认」与 I-P-01「立即自动应用事务并禁用卡片裁决」存在语义差（D2，D1 审计修复痕迹，需人类确认后回写文档）；②编辑器协同跨 AG/DE/ED/WS 四模块，是耦合最深的面，建议以 draft_mode 契约 + DiffProjection 字段为冻结锚点做集成验证，而非独立开发 |
| 独立性评估 | **B 级（集成验证面）**。不拆独立开发模块，作为跨模块联调与契约冻结锚点 |

### 2.6 提示词工程（★重点：提示词如何分层、构建策略）

| 维度 | 内容 |
|---|---|
| 需求锚点 | REQ-AG-004（上下文组装）、REQ-AG-005（三类回合提示词分离） |
| 设计锚点 | BR-AG-DATA-002（四层骨架 SSOT）、M-T-02（13 层细化 + 装配顺序 + 缓存边界 + BR-AG-DATA-004/005 截断序 + 三类回合结构示例 + SYSTEM_PROMPT 分型） |
| 实现位置 | 后端：`prompts.rs`（SYSTEM_PROMPT_CHAT/BUILD/AUTOCOMPLETE/INLINE_ASSIST + bdocx_capability_prompt manifest 投影）、`context.rs`（assemble_prompt 骨架/assemble_context 裁剪/cap_text/select_recent_history）、`lib.rs` assemble_agent_turn_messages（真实组装：系统→补强→同文档历史→跨文档弱历史→显式引用→当前文档 P0→意图提示→当前输入） |
| 实现程度 | **骨架完整、分层为缩减版**。四类提示词正文齐备且测试校验关键声明（硬约束/当前轮次/JSON 契约/Markdown+无 Mermaid）；组装顺序符合 BR-AG-DATA-002；三类回合差异实现（Chat 含历史，Assist 无历史） |
| 审计发现 | **D3**：M-T-02 13 层中 3 层未接入实现——任务状态层（P1，active 任务+阶段+artifacts）、模板补强层（P2，TPL 未消费）、工作区依赖层（P3，文件依赖图）；ContextSource 仅 4 源（ExplicitMention/CurrentDoc/Memory/Knowledge），且 Memory/Knowledge 标为 P1（设计为 P2 先截）。语义上「可裁」等价，但与 BR-AG-DATA-005 阶梯无显式映射表，TPL 接入时易漂移。②prompt cache（M-T-02 §3 稳定区进缓存）未实现——属实现期优化不违规则；③预算值（CTX_BUDGET=16000/AUG_BUDGET=2500/DOC_CTX_CAP=9000/MENTION_CAP=4000）为 dev 值，G5 标定，设计已声明 |
| 独立性评估 | **A 级可独立**。提示词文本/分层/截断集中在 prompts.rs + context.rs + assemble_agent_turn_messages 三处，测试完备，可独立演进；建议与 2.9 上下文处理合并为一个「组装面」 |

### 2.7 agent 工作策略（整体工作策略、AI 如何稳定执行、理解/阅读/抓取/分析/调研/编辑/生成如何协同）

| 维度 | 内容 |
|---|---|
| 需求锚点 | REQ-AG-001/002/003/004（回合/工具/门禁/上下文四能力协同） |
| 设计锚点 | SM-AG-01/02/03 三机、BR-AG-STATE-001/002/003、M-T-02 任务状态层（规划概念，未实现）、工作流体现为工具链（read/list/search/propose） |
| 实现位置 | 后端：`lib.rs` ag_start_turn 整体编排（门禁→快照校验→补强检索→引用注入→文档上下文→历史分流→工具循环→编辑产出→终态）、smListeners 状态广播（SM-AG-01/02/03 → 前端 sending 门控）；前端：agService/agStore 事件投影 |
| 实现程度 | **机制层完整、规划层未建**。稳定执行由五重机制保障：①门禁硬前置（结构上无绕过）；②工具注册/权限/上限校验；③写盘禁令（propose_edit 唯一内容落盘路径）；④编辑意图门（AG_EDIT_TOOL_REQUIRED：编辑请求必须产生 DiffProposed 或 edit_blocked）；⑤当前文档读前置门（AG_CURRENT_DOC_READ_REQUIRED：超长文档须先 read_doc/list_blocks 再编辑） |
| 审计发现 | ①「理解→阅读→抓取→分析→调研→编辑→生成」的协同当前由模型在提示词+工具约束下自由发挥（单回合 ≤6 轮工具循环），**无显式子任务规划/反思/多步骤工作流层**——与编辑模式产品形态匹配（轻量），1.0 EP 可接受；②设计中的「任务状态层」（M-T-02 P1）未实现，任务化编排语义归 CBT（本阶段外）；③策略稳定性不依赖规划层而依赖机制约束，是当前实现的明确取向，审计认为与产品定位一致 |
| 独立性评估 | **B 级（编排层）**。横切所有子模块，不建议拆独立开发模块；作为「回合编排」审计面（lib.rs + SM-AG-01 + smListeners）整体验证 |

### 2.8 场景能力（不同会话标签如何隔离，包括上下文隔离与前端物理隔离、聊天历史持久化）

| 维度 | 内容 |
|---|---|
| 需求锚点 | REQ-AG-001（会话）、REQ-AG-002（历史持久化） |
| 设计锚点 | TERM-AG-009 代理编辑会话、BR-AG-PERSIST-001（agent_sessions/agent_turns/agent_turn_payloads 三表、五态保留、懒加载、200MB 软归档、删除保留 DE diff）、I-P-01 历史恢复接口 |
| 实现位置 | 后端：`history.rs`（upsert_session/save_turn/list_sessions/list_turns/delete_session/soft_archive_over_limit/is_valid_status 五态校验）、`lib.rs`（ag.list_sessions/save_session/save_turn/list_turns/delete_session dispatch 命令）；前端：`agStore`（sessions/activeSessionId/标签操作/归档）、`AgentPanel.tsx`（标签栏+历史菜单+删除确认）、`agService`（hydrateAgentHistory/ensureAgentSessionTurns/persistAgentSessionsMetadata/turnSaveChain 串行化） |
| 实现程度 | **完整（十面中实现度最高）**。多标签会话隔离（前端内存态分组 + 后端 workspace_id/session_id 作用域持久化）；流式期间切标签不串会话（回合归属发起时 session 闭包冻结 relPath）；五态全保留（completed/error/cancelled/gate_rejected/draft）；激活会话 eager-load 其余懒加载；draft 重试新建 turn 不覆盖；软归档（200MB）；删除会话保留 DE pending diff（有专项测试） |
| 审计发现 | ①`turn.rs` 旧表持久化（persist_turn/recover_turns，无 session_id/status）与新三表是否双轨需确认（D4）；②删除按钮在回合进行中禁用（sending 门控）实现；③D1 审计修复「open_session 不再 auto-accept」影响历史恢复的 readonly 语义（D2 关联） |
| 独立性评估 | **S 级独立（优先固定）**。后端 history.rs + 前端 agStore/AgentPanel 自成闭环，命令契约（ag.list_sessions/save_session/save_turn/list_turns/delete_session）已冻结，可独立推进/审计/验证/固定 |

### 2.9 上下文处理（如何读取并理解历史信息、如何对当前/历史信息定位）

| 维度 | 内容 |
|---|---|
| 需求锚点 | REQ-AG-004（来源限定+确定性裁剪+显式注入优先） |
| 设计锚点 | BR-AG-DATA-001（来源限定/裁剪）、BR-AG-DATA-005（分层截断序 P0–P3）、AgentReadContext（已打开 LogicalState > 未打开 DiskState）、I-P-01（open_doc_snapshots 采集协议） |
| 实现位置 | 后端：`blocks.rs` AgentReadContext（active_editor 结构化快照强制/active_editor_session/open_editor_session/DiskState 兜底 + AG_STRUCTURED_SNAPSHOT_REQUIRED 等错误）、`context.rs`（assemble_context/cap_text/select_recent_history）、`lib.rs`（open_doc_snapshots 校验、历史按 relPath 分流、hist_budget 预算）；前端：`agService.collectOpenDocSnapshots`（binding 对齐/LogicalState 等待轮询 800ms/active_editor_session 降级） |
| 实现程度 | **完整**。已打开文档读取硬优先级完整实现（active live 序列化失败阻断、session 版本/哈希校验、active_doc_content 与快照一致性校验）；历史分流（same_doc/other_doc/dropped_cross_doc 可观测）；P0 永不截（当前文档超预算降级为块目录 + read_doc/list_blocks 前置门）；裁剪确定性 + trimmed 可追溯 |
| 审计发现 | ①前端采集（collectOpenDocSnapshots）与后端 AgentReadContext 双层把守形成完整「已打开文档优先」链，测试覆盖充分（agRuntime.test.ts 5 例）；②上下文处理与提示词组装（2.6）耦合紧密，建议合并审计；③DOC_CTX_CAP=9000 超限降级块目录机制与 BR-AG-DATA-005「当前文档超预算须可追踪块目录」一致 |
| 独立性评估 | **A 级可独立**（与提示词工程合并为「组装面」）。context.rs 纯函数 + AgentReadContext 边界清晰 |

### 2.10 意图识别（如何准确识别用户意图，尤其是当前轮次意图，在庞大提示词中如何准确分辨当前意图）

| 维度 | 内容 |
|---|---|
| 需求锚点 | REQ-AG-004（上下文组装）、BR-AG-DATA-004（当前轮次原则 + IntentCarryover） |
| 设计锚点 | TERM-AG-005 意图沿用判定（四维度：指代回指/显式选择/任务延续/修正追加）、当前轮次原则（历史默认弱化、显式沿用才提权、不污染当前轮） |
| 实现位置 | 后端：`context.rs` decide_intent_carryover（dev 启发式：prompt_has_explicit_history_reference + mentions_ordinal_choice + Refine/Select/ReferenceTarget 分组匹配）+ scope_history_for_current_doc（relPath 归属分流）、`lib.rs` assemble_agent_turn_messages（carryover → 跨文档弱注入门 + intent_hint_for_prompt 意图提示行贴近末尾）；提示词层：SYSTEM_PROMPT_CHAT 当前轮次原则声明 |
| 实现程度 | **机制完整、识别为 dev 启发式（设计允许）**。carryover 四维中实现三（Refine/Select/ReferenceTarget）；「任务延续」维度无独立实现（编辑模式无任务概念，归 CBT）；当前轮次原则双保险（提示词声明 + 意图提示行）；跨文档历史仅在显式沿用（carryover≠None）时弱标签注入且声明不得覆盖当前文档事实 |
| 审计发现 | ①**carryover 判定结果已实际接线**（此前标记核对点已确认）：assemble_agent_turn_messages 内决定 other_doc 弱注入并产出「【本轮意图】」提示行；ag_start_turn 处 `let _carryover` 仅是丢弃已内嵌到消息的返回值，非未接线；②实现为关键词启发式（「继续/刚才/第二个方案/改成」等），阈值与识别策略延实现期——设计债务已登记，升级路径（小模型/规则细化）不影响其他模块；③「修正追加」要求对象被显式锚定（「把刚才…改成」），泛编辑词（重写/优化）不误触发，与 v1.7 实现约束一致 |
| 独立性评估 | **S 级独立（最容易固定）**。两个纯函数（decide_intent_carryover + scope_history_for_current_doc）零外部依赖、测试完备（context.rs 内 8 例），可独立升级（启发式→小模型）且不影响提示词/回合/持久化任何模块 |

## §3 跨模块关键契约与耦合点

| 契约 | 形态 | 冻结锚点 | 消费方 |
|---|---|---|---|
| AgTurnEvent 六分支 | Channel<AgTurnEvent> wire 类型 | I-P-01 §2.1（PI-AG-10C） | agService/agStore ↔ lib.rs/ipc |
| DiffProjection | JSON（diff_id/patch_kind/before/after/draft_mode/snapshot_source/基线） | I-P-01 v1.11 起 | AG→DE→ED/WS 双侧 |
| draft_mode 三态 | current_editor/open_session/background | I-P-01 v1.19 | DE/ED 草稿链分流 |
| OpenDocSnapshot | LogicalState + session_version/content_hash + block/table/image snapshots | I-P-01 §2.1 / A-SYS-I-P-01 | AG 读取层 ← ED 采集 |
| HistTurn（relPath/turnId） | 历史消息文档归属 | I-P-01 v1.21 | AG 上下文分流 ← 前端投影 |
| ReferencePayload 8 kind | 归一化引用载荷 | BR-REF-DATA-003 / I-P-01 v1.6 | AG 注入 ← REF/Composer |
| BDocxCapabilityManifest | manifestId/version/sha256 单一来源 | A-SYS-I-P-04 | 提示词投影 + 工具 Schema + 执行器校验 |
| 门禁/网关契约 | GateOutcome/统一错误形状/三终态 | A-SYS-I-P-03（X-SEQ-001） | AG↔ENT↔GW |
| 记忆/知识供给 | MemoryInjection/KnowledgeInjection | AST/KB I-P | AG 补强层（lib.rs） |
| 构建移交/回落 | takeover/fallbackIn 双向回执 | SM-AG-01 ↔ SM-CBT-01 | AG↔CBT（本阶段外） |

## §4 审计发现清单

### 4.1 需人类裁决（设计-实现语义张力）

| 编号 | 严重度 | 内容 | 依据 |
|---|---|---|---|
| D1 | 高 | **create_doc 新建文档「直接创建不经过 DiffCard 审阅」，与 BR-AG-DATA-003「新建以空基准表达、经 DE diff 裁决闸门」语义张力**。`tool_registry.rs` create_doc 描述与 `SYSTEM_PROMPT_CHAT` 均声明新建直写；但 BR-AG-DATA-003（2026-07-04 版）规定新建文档以空基准经 diff 裁决卡为人类确认闸门。需人类确认：新建文档是否豁免 diff 裁决（若豁免，BR-AG-DATA-003 需补豁免条款；若不免，create_doc 执行路径需改） | BR-AG-DATA-003 / tool_registry.rs / prompts.rs |
| D2 | 中 | **open_session 草稿「自动应用到 ED session 但不自动确认」与 I-P-01「立即执行自动应用事务并禁用卡片裁决」语义对齐**。deService.applyOpenSessionDiff 实现"apply 只改 session 内容，不调用 de.accept_draft_diff，accept 由后续用户操作驱动"（2026-07-26 D1 审计修复）；卡片对 open_session 标记 readonly。需确认「禁用卡片裁决 = 用户仍须显式确认？」并回写文档 | deService.ts / I-P-01 v1.31 / agService.ts |

### 4.2 需复核/留痕

| 编号 | 严重度 | 内容 | 依据 |
|---|---|---|---|
| D3 | 中 | 提示词分层实现为 M-T-02 13 层缩减版：任务状态层（P1）、模板补强层（P2）、工作区依赖层（P3）未接入；ContextSource 仅 4 源，Memory/Knowledge 标 P1（设计 P2）。需建立「代码 ContextSource ↔ BR-AG-DATA-005 P0–P3 阶梯」显式映射表，防 TPL 接入时漂移 | M-T-02 §2/§5 / context.rs / lib.rs |
| D4 | 低 | `turn.rs` 旧表持久化（persist_turn/recover_turns，无 session_id/status 列）与 `history.rs` 新三表并存；新链路（ag_start_turn/save_turn）只用 history.rs。需确认旧函数是否仍有调用方，无则标记退役（本次不修改，仅留痕） | turn.rs / history.rs / lib.rs |
| D5 | 低 | `reference/inject.rs` verify_anchor（BR-REF-VERIFY-001）与 AG 消费路径（lib.rs format_reference_payload_context）的显式调用关系未在本次审计中逐行确认；selection 失效不作当前事实已有测试覆盖，风险低 | reference/inject.rs / lib.rs |
| D6 | 信息 | 预算常量（CTX_BUDGET=16000/AUG_BUDGET=2500/DOC_CTX_CAP=9000/MENTION_CAP=4000/MAX_ROUNDS=6/450ms 防抖）均为 dev 值，G5 标定（设计已声明延后） | lib.rs / assistStore.ts |

### 4.3 已确认无问题（免留痕）

- 门禁无绕过：SM-AG-01 结构性门禁（仅 AwaitingGate+GatePassed 达 Streaming）+ ent/gateway 门禁通道双把守，静态可查。
- carryover 接线：intent 判定结果实际影响跨文档弱注入与意图提示行（非死代码）。
- failure_reason 穿透链：后端→service→assistStore→overlay 无吞点。
- 会话隔离与历史持久化：多标签/五态/懒加载/软归档/删除保留 DE diff 全部实现且有测试。
- 已打开文档读取硬优先级：前端采集 + 后端 AgentReadContext 双层守门，覆盖 active live/非激活/切换过渡期。

## §5 模块化拆分方案（结构化拆分，支撑阶段化开发）

### 5.1 拆分原则

- 以**代码边界 + 契约冻结点**为拆，不以文档章节为拆（文档已按规则分层，代码模块以"可独立跑测试/独立固定契约"为准）。
- 每个拆分模块给出：职责、代码面、独立验证方式、耦合契约、固定方式。
- 代理构建（CBT）本阶段外，不拆。

### 5.2 八个可独立推进模块

| 模块 | 职责 | 代码面 | 独立验证方式 | 耦合契约（冻结锚点） | 固定方式 |
|---|---|---|---|---|---|
| **M1 回合编排与工作策略** | 回合生命周期、门禁前置、产出路由、工具循环编排、状态广播 | `ag/turn.rs`、`lib.rs`（ag_start_turn/ag_finish_assist/ag_propose_edit 编排）、`smListeners.ts` | cargo 单测（SM-AG-01 矩阵）+ agRuntime.test.ts + core_chain.rs | AgTurnEvent、GateOutcome、DiffProjection、SM-AG-01 广播 | 契约冻结：AgTurnEvent 六分支 + 终态映射 |
| **M2 工具调用能力** | 工具注册表、权限分级、schema 生成、只读/结构工具执行、BDocx 受控链 | `ag/tool.rs`、`ag/tool_registry.rs`、`ag/tool_exec.rs`、`ag/block_ops.rs`、`ag/bdocx_capabilities.rs`、`ag/blocks.rs`（AgentReadContext） | cargo 单测（校验/执行/块投影）+ manifest schema 测试 | 工具名+参数 JSON、BDocxCapabilityManifest（version/sha256） | 清单与 schema 由 manifest 单一来源；新增工具走注册表审批 |
| **M3 上下文与提示词组装**（含 2.6+2.9+2.10） | 四类提示词正文、组装骨架、历史分流、意图沿用、分层截断、AgentReadContext | `ag/prompts.rs`、`ag/context.rs`、`lib.rs` assemble_agent_turn_messages、`blocks.rs`（读取层） | cargo 单测（carryover/分流/截断确定性/提示词关键声明）+ agRuntime.test.ts | HistTurn（relPath/turnId）、OpenDocSnapshot、budget 常量表 | 截断序映射表（代码↔BR-AG-DATA-005）；budget 集中常量待 G5 |
| **M4 辅助能力**（autocomplete/inline-assist） | 触发防抖/latest-wins、门禁静默、候选解析、浮层交互与锚生命周期 | `ag/assist.rs`、`lib.rs` ag_request_assist、`assistStore.ts`、`AutocompleteOverlay.tsx`、`InlineAssistPopover.tsx`、`ed/assistPlacement.ts` | cargo 单测（SM-AG-03+解析容错）+ assistRuntime.test.ts + AgentUI.test.tsx | AssistReq/AssistResponse、failure_reason 枚举 | 3 候选硬门 + 解析容错集（纯函数）可独立固定 |
| **M5 会话与历史持久化** | 多标签会话、五态回合、懒加载、软归档、删除 | `ag/history.rs`、`lib.rs`（ag.*_session/turn 命令）、`agStore.ts`、`AgentPanel.tsx`、`agService.ts`（历史段） | cargo 单测（三表/五态/软归档/删除）+ agRuntime.test.ts（恢复） | agent_sessions/agent_turns/agent_turn_payloads schema、AgentTurnPayloadV1 | **S 级独立，可最先固定** |
| **M6 消息流呈现** | GFM Markdown 子集渲染、sanitize、回合内时序（segments）、Diff/Tool 卡内嵌 | `MessageFlow.tsx`、`MessageList.tsx`、`MessageMarkdown.tsx`、`ToolCallCard.tsx`、`agStore.ts`（segments） | AgentUI.test.tsx（渲染/sanitize/时序）+ agRuntime.test.ts | TurnSegment（text/tool/diff）、legacy 降级语义 | 纯展示层，不改变回合/工具/持久化语义 |
| **M7 引用注入消费** | ReferencePayload 归一化、Composer 引用原子、注入组装 | `referencePayload.ts`、`Composer.tsx`、`referenceDrag.ts`、`lib.rs`（注入段）、`reference/*` | agRuntime.test.ts（ReferencePayload 契约）+ referencePayload 单测 | ReferencePayload 8 kind、REFERENCE_MIME | REF 模块独立 owner；AG 消费侧窄 |
| **M8 编辑产出与 Diff 协同** | propose_edit 提案、DiffProjection、draft_mode 分流、ED 草稿链 | `lib.rs` ag_propose_edit、`deService.ts`、`deStore.ts`、`edStore.ts`（草稿/缓存侧） | deRuntime.test.ts + edRuntime.test.ts + 跨模块集成（core_chain.rs） | DiffProjection、draft_mode 三态、OpenDocSnapshot | 契约冻结：DiffProjection 字段 + 裁决路径（accept/reject/draft） |

### 5.3 拆分后的建议开发次序（阶段化）

- **阶段 1（已基本完成，建议先固定）**：M5 会话与历史持久化（S 级）→ 契约全部落地，补 D4 双轨清理核对即可固定。
- **阶段 2（当前主线）**：M4 辅助能力 + M6 消息流呈现（已随 AG-MSG-RENDER-01 大部分完成）→ 补测试收敛与 failure_reason 诊断链路复核。
- **阶段 3（提示词与上下文深化）**：M3 上下文与提示词组装 → 补任务状态层/模板层/工作区依赖层（D3 映射表先行），G5 标定预算。
- **阶段 4（编辑与工具结构化收尾）**：M2 + M8 → 新建文档裁决语义（D1）、open_session 应用语义（D2）人类裁决后收口。
- **阶段 5（集成验证）**：M1 回合编排 + 跨模块 G5 联调（门禁→网关→流式→diff→落盘全链）。
- **后续版本（本阶段外）**：代理构建（CBT）经 takeover/fallbackIn 契约接入 M1。

## §6 建议后续动作（不包含本次执行）

1. 人类裁决 D1（新建文档是否豁免 diff 裁决）与 D2（open_session 草稿确认语义），裁决后回写 BR-AG-DATA-003 / I-P-01。
2. 复核 D4（turn.rs 旧表调用方）与 D5（verify_anchor 消费路径），无调用方则按治理流程标记退役/补显式调用。
3. 建立 ContextSource ↔ BR-AG-DATA-005 截断序映射表（D3 前置），为 TPL/任务状态层接入铺路。
4. 按 §5.2 八模块建立各模块独立测试入口与契约清单，作为后续阶段化开发的基线。
5. G5 标定 budget 常量与循环上限、防抖数值（D6）。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-08-07 | v1.0 | 初版：AG 代理功能全链路审计（只审计不修改）——十大功能审计面逐一对照（需求/设计/实现/差距/独立性）；审计发现清单 D1–D6；八个可独立推进模块的结构化拆分方案（M1–M8）与阶段化开发建议 |
