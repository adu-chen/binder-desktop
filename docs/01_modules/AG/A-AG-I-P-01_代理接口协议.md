---
文档编号：AG-I-P-01
文档状态：A
负责模块：AG
文档职责：AG 代理模块级接口协议（L2 接口层）——按 A-SYS-I-P-02 骨架声明 代理回合 command、工具调用回注接口、局部修改/辅助续写接口、上下文组装与门禁前置对接、流式回传事件；AI 调用面（一切模型调用经门禁→网关）
上游约束：A-SYS-I-P-02_IPC协议.md（骨架、流式逻辑契约）、A-SYS-I-P-03_云接口契约.md（CLOUD-GATE/INVOKE、SM-SYS-01、X-SEQ-001）、A-AG-M-T-01_代理技术设计.md（BR-AG-* 规则、AG-TURN/TOOL/ASSIST、X-FORBID-006）、A-AG-M-S-01_代理状态机设计.md（SM-AG-01/02/03）、A-DE-I-P-01_DiffEdit接口协议.md（generate_diff 承接产出）、A-ED-I-P-01_编辑器接口协议.md（apply_edit/装饰层）、A-REF-M-T-01（注入载荷）、ADU.md
直接承接：实现期 AG command 注册；门禁/网关经 ENT 客户端通道（A-SYS-I-P-03 契约）；产出交 DE（de.generate_diff）；辅助采纳经 ED（ed.apply_edit）；记忆/知识/引用注入经 AST/KB/REF 供给
使用边界：本文声明 AG 接口语义级签名；门禁/网关协议字段归 A-SYS-I-P-03（不重定义）；落盘经 DE→WS（不重复）；呈现经 ED（不重复）；不新增 RULE/CHAIN/CONSTRAINT/TERM；工具清单/提示词/模型档延实现期运营配置
变更要求：门禁前置语义变更涉 APC §13 须先授权；产出接口变更复核 DE；辅助采纳接口变更复核 ED；工具回注接口变更复核 SM-AG-02
治理成熟度：draft
---

# Binder 代理接口协议（AG · L2 接口层）

## §1 文档定位

```
A-SYS-I-P-02（骨架）+ A-SYS-I-P-03（门禁/网关契约）→ 本文（AG 回合/工具/辅助接口）→ 实现 + @GOV(追溯 A-AG-M-T-01)
门禁前置：每个发起 command 内部必经 ENT 门禁通道（SM-SYS-01 GatePassed）方达网关——X-SEQ-001 在接口层的硬约束
产出：HandingOff → de.generate_diff；辅助采纳 → ed.apply_edit
```

**接口层门禁不变量（X-SEQ-001 / BR-AG-VERIFY-001）**：下列任一发起 command 的实现路径中，
模型调用前必经门禁；无有效订阅/断网时——代理/工具回合 command 返回 GateDenied 类错误，
辅助类 command 静默返回空候选（不报错、不打断编辑）。接口签名不暴露任何绕过门禁的入口。

## §2 代理回合接口（AG-TURN / SM-AG-01）

| command/事件 | 参数（语义） | 结果 | 特有错误/状态 | 归属 |
|---|---|---|---|---|
| `ag.start_turn` | DocRef、prompt、注入引用集（`references` 主通道；旧 `mentions/block_refs` 兼容，可空）、`Channel<AgTurnEvent>` 句柄 | turn_id（SM-AG-01 启动）；流式经 channel 回推（§2.1） | **gate_denied**（无订阅/断网，不可绕过）；组装失败（注入锚失效） | 经门禁→网关；BR-AG-STATE-001/VERIFY-001 |
| `ag.cancel_turn` | turn_id | 中断确认（Cancelled 终态，无半落盘）；前端关闭 channel | — | SM-AG-01 cancel |
| `ag.turn_stream`（`Channel<AgTurnEvent>`） | turn_id、channel（随 start_turn 传入） | AgTurnEvent 六分支回推（§2.1） | 流式错误→Error 分支（统一错误形状） | BR-SYS-STATE-002 三类终态 |
| `ag.list_turns` | DocRef（可空） | 回合历史列表（agent_turns，A-SYS-I-P-01） | — | BR-AG-PERSIST-001（重启可恢复、不上云） |

> 含修改建议的回合产出：回合内部经 `de.generate_diff` 交 DE（AG 无直接写盘，X-FORBID-006）；
> 流式仅回显，落盘必经 diff 裁决链。

### §2.1 流式回传契约（AgTurnEvent · 前后端双轨同步冻结点）

> 本节是 A-AG-X-L-02 PI-AG-10C 的契约落点——前端轨（PI-AG-10F）与后端轨（PI-AG-10B）共同承接的
> **唯一真源**。**物理传输** Tauri 2 `Channel<AgTurnEvent>`（SY-02 选型，per-call channel，不经
> dispatch 信封）；**事件语义**承 BR-SYS-STATE-002 三终态。本节为语义级接口签名，不新增注册类治理块。
>
> **与 SYS `ipc::StreamEvent` 的关系（分层不同·非重复）**：`ipc::StreamEvent` 是通用 IPC 传输契约
> （dispatch 信封 + correlation_id，BR-SYS-STATE-002）；`AgTurnEvent` 是 AG 回合**领域**事件，走
> 独立 per-call `Channel<T>`（channel 即回合关联，故用 turn_id 而非 correlation_id）。终态语义相通，
> 但传输层不同故用独立 wire 类型，**不复用** `StreamEvent`（避免把领域事件埋进通用 `data`），
> 也不与之重名。

**AgTurnEvent 六分支**（`src-tauri/src/ipc/mod.rs`，serde `tag="type"` snake_case）：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgTurnEvent {
    Chunk        { turn_id: String, text: String },                                  // 模型输出文本分片
    ToolRequest  { turn_id: String, tool_name: String, params: serde_json::Value },  // 模型请求工具（SM-AG-02 激活）
    ToolResult   { turn_id: String, tool_name: String, result: String, success: bool }, // 工具结果回注（展示）
    DiffProposed { turn_id: String, diff_id: String, rel_path: String },             // ChangeProposal→DE 产出待裁决
    Done         { turn_id: String, stop_reason: String },                           // 正常完成 / cancelled
    Error        { turn_id: String, code: String, message: String, retriable: bool },// 错误（含可重试）
}
```

**ag_start_turn 命令签名**（独立 `tauri::command`，不走 dispatch 信封）：

```rust
#[tauri::command]
async fn ag_start_turn(
    turn_id: String,
    prompt: String,
    provider: Option<String>,
    model: Option<String>,
    active_doc_path: Option<String>,
    active_doc_content: Option<String>,
    active_doc_format: Option<String>,
    history: Vec<HistTurn>,
    mentions: Vec<String>,
    block_refs: Vec<BlockRef>,
    references: Vec<ReferencePayload>,
    on_event: Channel<AgTurnEvent>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), IpcError>
```

`block_refs` 与前端 `SelectionRef` 对齐，serde camelCase：

```rust
struct BlockRef {
    rel_path: String,
    block_id: String,
    start: Option<usize>,
    end: Option<usize>,
    text: String,
}
```

`start/end` 为块内纯文本偏移区间（CharAnchor，A-ED-I-P-01 §4 / A-SYS-I-P-01 RefInjection
`block_ranges`）；缺省时降级为块级选中文本引用，不得静默丢弃 `text`。

`references` 为新前端路径主通道，serde `tag="kind"` snake_case，字段 camelCase；覆盖
`doc` / `selection` / `session` / `memory` / `knowledge` / `template` / `url` / `long_text`。
当 `references` 非空时，前端不得再把同一引用重复塞入 `mentions/block_refs`；后端保留旧参数仅为兼容
历史调用。

`active_doc_content` 必须优先取当前 ED 编辑器实时序列化内容；不得用打开时的 `loadedContent` 旧快照作为
代理回合主事实。该实时快照只可作为 `path == active_doc_path` 的编辑基准；`propose_edit` 指向其他文档时
必须读取该目标文档自身内容，不得复用当前编辑器快照。当前打开文档缺少实时快照或当前编辑器锚无法定位时，
不得发出正常 pending `DiffProposed`；前端应投影为 error 卡并阻止进入 ED 草稿链。

AgentEdit 对文档内容的读取只有一个运行时投影：`AgentEditableSnapshot`。该投影由 AG 读取层统一产出，
字段包括 `{rel_path, editable_kind, source, content_format, content, baseline?, blocks[]}`；`blocks[]`
统一为 `{block_id, block_index, text, marks, block_attrs}`。当前编辑器实时内容、`read_doc`、`search_text`、
`ag.list_blocks`、`propose_edit`、ReferencePayload.doc 消费 BDocx 时必须复用该投影。BDocx 必须先经
DOC 编辑链转为 canonical HTML 后投影；原生 `.docx` 不得进入该投影，应返回 `DOC_NOT_BDOCX`。
`propose_edit` 的 `block_id` 只能传块列表中 `[id=...]` 的完整不透明 id；纯数字块号必须
传 `block_index`。后端对数字型 `block_id` 做 `block_index` 兜底，但该兜底只用于防错，不作为推荐协议。
`propose_edit` 必须声明 `operation`：已有文档局部编辑使用 `replace_block`，并提供
`block_id/block_index` 与 `new_text`；只有用户明确要求整篇重写时才允许 `replace_document` + `content`；新建文档可兼容旧式
`content` 创建路径。已有文档若只传裸 `content`，后端必须拒绝并返回 `AG_PRECISE_TARGET_REQUIRED`，
不得静默生成整篇替换 diff。
`propose_edit` 可携带 `style_ops`（StyleOp 数组，schema 见 A-ED-M-T-01 §5.1）：样式类修改必须经该字段
进入 DE 裁决，不得只用自然语言声明“已加粗/已改样式”。若本轮是编辑意图且当前文档存在，后端必须在
回合结束前收到成功的 `propose_edit` 并发出 `DiffProposed`；否则回 `Error { code: AG_EDIT_TOOL_REQUIRED }`，
前端不得将该回合投影为正常完成或“已修改”。

成功的 `propose_edit` 必须先通过 `ToolResult { tool_name: "propose_edit" }` 返回后端权威
`DiffProjection` JSON，再发送 `DiffProposed` 通知。`DiffProjection` 至少包含
`{diff_id, rel_path, patch_kind, block_id?, block_index?, before_text, after_text, location_label,
draft_mode, style_ops?}`，其中 before/after 必须来自同一 `AgentEditableSnapshot` 的目标块或显式
`replace_document` 全文基准。前端不得再从 `tool_request.params` 或当前编辑器尾部内容推测 before/after；
缺少 `DiffProjection` 时必须投 error 卡（`DE_DIFF_PROJECTION_MISSING`），不进入 ED 草稿链。

**三终态映射**（BR-SYS-STATE-002）：完成 → `Done`；用户中断 → `Done { stop_reason: "cancelled" }`（无半落盘）；错误 → `Error`。

**前端消费映射**（PI-AG-10F，`agStore`/`deStore`）：`Chunk`→`appendToMessage`、`ToolRequest`/`ToolResult`→工具卡片；
成功 `ToolResult(propose_edit).projection`→缓存权威 DiffProjection，随后 `DiffProposed`→`deStore.addDiff`
（按 projection 建卡并进入 ED 草稿链或 error 卡）；`Done`→`finishMessage`、`Error`→`pushError`。

**关联命令形状**（同属契约，前后端对齐）：`ag.cancel_turn(turn_id)`（前端关闭 channel）；产出落
`de.generate_diff`→`DiffProposed`；当前编辑器草稿路径裁决经 `de.accept_draft_diff`/`de.reject_diff`，
后台文本/.html 路径仍可经 `de.accept_diff`/`de.reject_diff`（归 A-DE-I-P-01，本文不重定义）。既有
BDocx 修改的磁盘内容是 docx 字节，未应用到当前编辑器 HTML 草稿的 BDocx diff 不得走后台 raw apply；前端应投
`DE_BDOCX_DRAFT_REQUIRED`，避免把 HTML 当 docx 字节写入。

**冻结约定**：本 schema 一经冻结，字段增删须前后端**双侧同步**并回写本节；桩阶段（GW 未就绪）后端经 `route_start_turn` 桩从 channel 推 `Chunk×N + Done`，前端零改动切换真实。

## §3 工具调用回注接口（AG-TOOL / SM-AG-02）

| 接口 | 参数（语义） | 结果 | 特有错误/状态 | 归属 |
|---|---|---|---|---|
| `ag.tool_result`（回注） | turn_id、tool_call_id、执行结果/失败 | 续行回合 | 工具未注册/越权→拒绝并回注拒绝原因（不执行） | SM-AG-02 ToolValidating；BR-AG-STATE-002 |

> 工具执行在回合内编排：只读类工具（读/搜/块读取）经 workspace 限界后统一产出
> `AgentEditableSnapshot`，BDocx 走 DOC 编辑模型而非 WS UTF-8 文本读取；
> 非破坏性结构工具（`move_doc` / `create_folder`）经 WS 边界直接执行，只改变文件树结构，不修改文档
> 内容、不生成 DiffCard；目标冲突拒绝，不覆盖；
> **写意图工具不直写**——产出修改建议走 `de.generate_diff`（X-FORBID-006）；循环上限/中断由
> SM-AG-02 管理，达上限 LoopAborted。删除类工具需独立用户确认回路，未落地前不得注册给模型。

| 工具 | 参数（语义） | 结果 | 权限 | 约束 |
|---|---|---|---|---|
| `move_doc` | from_path、to_path | operation/from_path/to_path/summary | StructureWrite | workspace 内移动/重命名；目标已存在拒绝；不覆盖 |
| `create_folder` | path | operation/path/summary | StructureWrite | workspace 内建目录；目标已是目录幂等成功；目标为文件拒绝 |

## §4 局部修改/辅助续写接口（AG-ASSIST / SM-AG-03）

| command | 参数（语义） | 结果 | 特有错误/状态 | 归属 |
|---|---|---|---|---|
| `ag.request_assist` | request_id、assist_kind（autocomplete/inline_assist）、编辑上下文（DocRef、blockId、字符锚、位置/选区/光标、触发前后文指纹）、instruction、prefix、base_text（局部修改 refine=上一轮候选） | autocomplete 严格返回 3 candidates；inline_assist 返回 candidate；失败为空并可携内部 `failure_reason` | 新请求覆盖旧请求（旧 request_id 返回后由前端丢弃）；候选不足 3 条/无订阅/断网/失败→**静默空候选**（不报错）；`failure_reason` 仅供诊断/测试，不直接暴露给普通用户 | SM-AG-03；BR-AG-STATE-003（每次过门禁，无例外） |
| `ag.finish_assist` | request_id、event（accept/dismiss/anchor_lost） | ok | 仅广播 SM-AG-03 终态；不修改文档、不触发模型调用 | SM-AG-03；BR-AG-STATE-003 |

> 候选呈现经 `ed.mount_decoration`（装饰层）；**采纳经 `ed.apply_edit`**（作为用户编辑操作进
> 编辑模型，驱动 SM-ED-01 edit→脏态，非 AI 直写）；忽略/继续输入经 `ed.dismiss_decoration` 复位。
> 结果返回和采纳前均校验 request_id 与 DocRef/blockId/字符锚/触发前后文指纹；锚失效或旧请求返回时丢弃候选。
> 前端默认 450ms 防抖后再调 request_assist；采纳、忽略/关闭、锚失效必须调用 finish_assist 回写终态。
> autocomplete 的候选解析允许兼容纯 JSON 数组、fenced/多行 JSON、`{ candidates: [...] }` 包装和编号/项目符号文本；
> 解析后仍不足 3 条时返回空候选 + `failure_reason=parse_underflow`，前端保留轻量空态但不展示半候选。
> `failure_reason` 是内部诊断字段，不弹普通错误；但 requestAssist service、assistStore 与 overlay 诊断点必须保留该值，
> 不得在 Editor 层统一吞成无原因空态。invoke 异常由 service 映射为 `failure_reason=invoke_error`。

### §4.1 文档块读取接口（agent 编辑定位 / 样式投影，PI-ED-10/11 联合承接）

| command | 参数（语义） | 结果 | 特有错误/状态 | 归属 |
|---|---|---|---|---|
| `ag.list_blocks` | DocRef | 块列表：`{block_id, block_index, text, marks, block_attrs}`；前端边界可映射为 camelCase。`marks` 为 `{type, range:[start,end], attrs}`，range 在块内纯文本平面，与字符锚同基准 | 文档不存在/越界 → WS/DOC 错误透传；原生 docx → `DOC_NOT_BDOCX` | agent 块级引用选择 + 字符级取址基准 + 样式读取（BR-ED-DATA-005 读侧） |

> 投影 schema 的单一来源在 A-ED-M-T-01 §5.1（ED 定义、AG 承载读取面）；写侧（StyleOp）经
> `ed.apply_edit`（A-ED-I-P-01 §4.1），落盘分流按裁定 D-3（回合产出→DE diff / 显式内联→用户编辑）。
> BDocx 的非当前文档读取必须先经 DOC 编辑链转为 HTML 后再分块；原生 `.docx` 不支持 `ag.list_blocks`，
> 应回 `DOC_NOT_BDOCX`，避免把二进制 docx 当 UTF-8 文本或可编辑块列表处理。
> 锚有效期 = 编辑会话内（BR-ED-DATA-004）：文档重开后 blockId 重建，agent 须重新 list_blocks。

## §5 上下文组装对接（AG-TURN / BR-AG-DATA-001，仅索引）

回合上下文来源（组装在 ag.start_turn 内部完成，限界 X-FORBID-004，显式注入优先保留）：

| 来源 | 供给 | 文档 |
|---|---|---|
| 当前文档 | `active_doc_content` / `AgentEditableSnapshot` | 本文 §2.1、A-WS-I-P-01 §3、A-DOC-M-T-01 §2 |
| 输入引用 | REF ReferencePayload / 注入载荷 | A-REF-M-T-01 §4、A-REF-I-P-01 §2.1 |
| 记忆 | AST 供给（MemoryInjection） | A-AST-M-T-01 §4 |
| 知识库 | KB 召回（KnowledgeInjection） | A-KB-M-T-01 §3 |

> REF/AST/KB 的供给接口随其各自 I-P（待建）声明；本文只定 AG 消费侧的组装入口。

## §6 测试与验收

- **门禁不变量**（BR-AG-VERIFY-001，强制）：静态可查 start_turn/tool 执行/request_assist 三条
  发起路径均无不经门禁的模型调用入口；无订阅时 chat 返回 gate_denied、assist 静默空候选。
- **产出唯一出口**：含修改建议回合经 `de.generate_diff`，无直接写盘（X-FORBID-006 静态+联调）。
- **工具循环**：未注册/越权工具拒绝回注；上限生效；写意图工具不直写。
- **辅助采纳路径**：request_assist→mount_decoration→apply_edit 走正常脏态（非 diff），与 ED G5 联调。
- 流式三终态、回合历史恢复、与 DE/ED/WS/REF/AST/KB G5 全链联调。

## §7 债务与盲区

- 工具注册清单/权限分级表、提示词与模型档延实现期运营配置（接口不锁）。
- request_assist 的节流（前端侧 vs 接口侧）实现期定；候选呈现 schema 联合 ED/DE。
- 构建移交/回落接口（takeover/fallback）随 CBT I-P 联合声明（本文不预定义，AG-04）。
- REF/AST/KB 供给接口待其 I-P；本文 §5 仅占消费入口。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（实现前置件，AI 调用面）：AG 接口协议——chat 回合 4（start/cancel/stream/list，门禁不变量内建）、工具回注 1（tool_result 校验回注）、内联辅助 1（request_assist 静默失败、采纳经 ed.apply_edit）、上下文组装对接索引（WS/REF/AST/KB）；门禁前置 X-SEQ-001 固化为接口不变量、产出唯一经 de.generate_diff；零新增注册项；债务（工具清单/节流/接管接口/供给接口延后） |
| 2026-06-24 | v1.1 | 新增 §2.1 流式回传契约（AgTurnEvent · PI-AG-10C 冻结点）：AgTurnEvent 六分支（Chunk/ToolRequest/ToolResult/DiffProposed/Done/Error）、ag_start_turn 命令签名（Channel<AgTurnEvent> 传输，承 SY-02 选型）、三终态映射（BR-SYS-STATE-002）、前端消费映射（agStore/deStore）、关联命令形状与冻结约定；§2 表格 start_turn/turn_stream 行改为 Channel<AgTurnEvent> 模型并指向 §2.1。前后端双轨同步真源，零新增注册类治理块 |
| 2026-06-26 | v1.2 | 命名落地修正：AG 回合流式事件定名 **AgTurnEvent**（避开 src-tauri/src/ipc/mod.rs 既有 SYS `StreamEvent` 重名），§2.1 补「与 SYS StreamEvent 分层不同·非重复」说明（通用 dispatch 传输契约 vs AG 领域 per-call Channel）；PI-AG-10C 落地同步 |
| 2026-07-03 | v1.3 | 批B 契约扩展（A-ED-X-L-02 PI-ED-10/11 文档先行，ED 窗口执笔·追加型最小修订）：新增 §4.1 文档块读取接口——补登记既有 ag.list_blocks + 样式投影扩展（marks/blockAttrs，schema 单一来源 A-ED-M-T-01 §5.1）；写侧 StyleOp 经 ed.apply_edit、落盘分流按裁定 D-3；锚有效期=编辑会话内（BR-ED-DATA-004） |
| 2026-07-04 | v1.4 | 术语统一清扫（承 2026-07-04 人类三轮裁决）：正文禁用别名替换——内联辅助→局部修改、自动补全→辅助续写、对话编辑→Diff Edit、Chat↔Build 接管→构建移交/回落、chat/chat-agent 回合→代理回合；文件名与变更记录历史行不改写 |
| 2026-07-08 | v1.5 | 同步当前 `ag_start_turn` 真实命令签名；补 `block_refs` 的 SelectionRef 字段 `{rel_path, block_id, start, end, text}`，明确 start/end 为 CharAnchor 块内纯文本偏移，缺省时降级为块级选中文本引用且不得丢弃 text |
| 2026-07-08 | v1.6 | `ag_start_turn` 新增 `references: Vec<ReferencePayload>` 主通道，覆盖 doc/selection/session/memory/knowledge/template/url/long_text；旧 `mentions/block_refs` 保留兼容但不得与 `references` 重复注入同一引用 |
| 2026-07-08 | v1.7 | 修正代理编辑 diff 契约：active_doc_content 优先实时编辑器内容；propose_edit 明确 block_id 与 block_index，不允许数字块号误填 block_id；当前编辑器草稿路径接受经 `de.accept_draft_diff` 确认，保存仍走 ED/WS save |
| 2026-07-09 | v1.8 | 补齐代理编辑协议门：`propose_edit` 支持 `style_ops`，样式修改经 DE 裁决进入 ED 草稿；编辑意图未产生 `DiffProposed` 时返回 `AG_EDIT_TOOL_REQUIRED`；`ag.list_blocks` 对 BDocx 走 DOC→HTML 分块，原生 docx 明确拒绝 |
| 2026-07-09 | v1.9 | 收紧 AgentEdit 基础锚链：当前编辑器实时快照只可服务当前目标文档；当前打开文档缺快照或锚失效时不得投正常 pending DiffProposed，前端以 error 卡阻止进入 ED 草稿链 |
| 2026-07-09 | v1.10 | 收紧 `propose_edit.operation` 协议：已有文档默认必须走 `replace_block` 精准块编辑；`replace_document` 仅用于用户明确整篇重写；已有文档裸 `content` 拒绝为 `AG_PRECISE_TARGET_REQUIRED`，避免 BDocx 局部编辑误投影为整篇替换 |
| 2026-07-09 | v1.11 | 补齐 AgentEdit 唯一读写投影契约：`AgentEditableSnapshot` 统一当前文档、read_doc/search_text/list_blocks/propose_edit/ReferencePayload.doc 的 BDocx 读取；`propose_edit` 先回 `DiffProjection` 后发 `DiffProposed`，前端禁止从 tool_request 重建 before/after；既有 BDocx 修改未进当前编辑器草稿时禁止后台 raw apply |
| 2026-07-16 | v1.12 | 细化 `ag.request_assist`：请求显式携 request_id、DocRef、blockId、字符锚与 base_text；辅助续写返回 3 candidates，局部修改返回 candidate；新请求覆盖旧请求，旧 request_id 返回后丢弃；结果返回和采纳前均校验锚有效 |
| 2026-07-16 | v1.13 | 补齐 AG 结构工具协议：`move_doc`/`create_folder` 作为 StructureWrite 工具经 WS 边界直接执行并回注；内容修改仍走 `propose_edit`；删除工具等待确认回路，未注册给模型 |
| 2026-07-16 | v1.14 | 续修 `ag.request_assist`/`ag.finish_assist`：严格 3 候选、触发前后文指纹、默认 450ms 前端防抖，以及采纳/忽略/锚失效终态回写 |
| 2026-07-17 | v1.15 | 加固 `ag.request_assist` 失败诊断：返回内部 `failure_reason`，autocomplete 解析兼容常见模型格式偏差但仍以 3 candidates 为展示硬门 |
| 2026-07-17 | v1.16 | 收敛前端诊断传递契约：failure_reason 必须经 requestAssist、assistStore、overlay 诊断点保留，invoke 异常映射为 invoke_error |
