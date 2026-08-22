---
文档编号：AG-I-P-01
文档状态：A
负责模块：AG
文档职责：AG 代理模块级接口协议（L2 接口层）——声明代理回合、工具、辅助、流式事件与 agent-edit-1.0 Runtime Profile 的冻结接口
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
| `ag.start_turn` | session_id、`runtime_profile=agent-edit-1.0`、DocRef、prompt、已打开文档 `open_doc_snapshots`、注入引用集（`references` 主通道；旧 `mentions/block_refs` 兼容，可空）、`Channel<AgTurnEvent>` 句柄 | turn_id（SM-AG-01 启动）；流式经 channel 回推（§2.1）；后端先落 `draft`，终态更新 status | **gate_denied**（无订阅/断网，不可绕过）；`AG_RUNTIME_PROFILE_UNSUPPORTED`；组装失败；阻断歧义返回澄清且不调用模型 | 经门禁→网关；BR-AG-STATE-001/VERIFY-001/PERSIST-001/DATA-002/004/005 |
| `ag.cancel_turn` | turn_id | 触发后端 CancellationToken；中断流和后续工具调度；返回 Cancelled 或 already_terminal | 未知 turn_id→not_found | SM-AG-01 cancel；幂等 |
| `ag.turn_stream`（`Channel<AgTurnEvent>`） | turn_id、channel（随 start_turn 传入） | AgTurnEvent 六分支回推（§2.1） | 流式错误→Error 分支（统一错误形状） | BR-SYS-STATE-002 三类终态 |
| `ag.list_sessions` | — | 当前 workspace 的代理编辑会话元数据列表；含 activeSessionId | 无 active workspace → 空列表 | BR-AG-PERSIST-001（标签元数据全恢复） |
| `ag.save_session` | session_id、title、mode、archived、open_index、is_active | ok | 无 active workspace → 错误 | BR-AG-PERSIST-001（标签元数据持久化） |
| `ag.save_turn` | session_id、turn_id、status、rel_path、AgentTurnPayloadV1 | ok；upsert payload 与 turn 记录 | status 非五枚举值 → 拒绝 | BR-AG-PERSIST-001（完整 payload 由前端 UI 投影回写） |
| `ag.list_turns` | session_id | 指定代理编辑会话的回合历史列表（按 created_at 升序） | session 不属当前 workspace → 拒绝 | BR-AG-PERSIST-001（内容懒加载） |
| `ag.delete_session` | session_id | 删除当前 workspace 指定代理编辑会话及其回合 payload 记录；返回删除计数 | 无 active workspace → 错误；代理回合进行中前端禁用 | BR-AG-PERSIST-001（用户删除历史会话） |

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

**AgTurnEvent 七分支**（`src-tauri/src/ipc/mod.rs`，serde `tag="type"` snake_case）：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgTurnEvent {
    Chunk        { turn_id: String, text: String },                                  // 模型输出文本分片
    ToolRequest  { turn_id: String, tool_name: String, params: serde_json::Value },  // 模型请求工具（SM-AG-02 激活）
    ToolResult   { turn_id: String, tool_name: String, result: String, success: bool }, // 工具结果回注（展示）
    DiffProposed { turn_id: String, diff_id: String, rel_path: String },             // ChangeProposal→DE 产出待裁决
    ClarificationRequired { turn_id: String, text: String },                         // 澄清等待（非终态，BR-SYS-STATE-007）
    Done         { turn_id: String, stop_reason: String },                           // 正常完成 / cancelled
    Error        { turn_id: String, code: String, message: String, retriable: bool },// 错误（含可重试）
}
```

> **澄清等待事件（2026-08-14，AG-1.0-IMP-CLARIFICATION-01 / BR-SYS-STATE-007）**：
> `ClarificationRequired` 是**非终态**事件——回合不终结、不锁存终态、不发 Done；前端投影为
> `clarifying` 等待态（AgentTurnStatus），用户补充输入时先取消澄清回合再发起新回合，或在此阶段取消。
> 澄清期间不调用门禁/网关/工具（BR-AG-STATE-001 blocking ambiguity 语义）。

> **事件顺序即时序契约（2026-08-07，承接 BR-AG-UI-001 / TERM-AG-011）**：同一回合内
> `Chunk → ToolRequest → ToolResult → (DiffProposed) → 下一轮 Chunk → …` 按发射顺序经 Channel
> FIFO 到达前端；前端据此重建"正文↔工具卡↔Diff 卡"的交错时序（消息片段），接口分支本身不变。

**ag_start_turn 命令签名**（独立 `tauri::command`，不走 dispatch 信封）：

```rust
#[tauri::command]
async fn ag_start_turn(
    turn_id: String,
    session_id: String,
    runtime_profile: String, // 1.0 仅允许 "agent-edit-1.0"
    prompt: String,
    provider: Option<String>,
    model: Option<String>,
    active_doc_path: Option<String>,
    active_doc_content: Option<String>,
    active_doc_format: Option<String>,
    open_doc_snapshots: Vec<OpenDocSnapshot>,
    history: Vec<HistTurn>,
    mentions: Vec<String>,
    block_refs: Vec<BlockRef>,
    references: Vec<ReferencePayload>,
    on_event: Channel<AgTurnEvent>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), IpcError>
```

1.0 中 `runtime_profile` 不得由前端仅用 edit/build 展示态隐式推断；后端必须显式校验。`BuildMode` 在
独立 Runtime Profile 与 CBT 系统验证完成前返回 `AG_RUNTIME_PROFILE_UNSUPPORTED`，不得仅切换模型后
复用 agent-edit Prompt/Context/Tools。

`OpenDocSnapshot` 语义与字段以 A-SYS-I-P-01 §3 为唯一来源，serde 使用 camelCase：

```rust
struct OpenDocSnapshot {
    rel_path: String,
    content: String,
    edit_format: String,
    source: String, // active_editor / active_editor_session / open_editor_session
    dirty: bool,
    session_version: u64,
    content_hash: String,
    captured_at: i64,
}
```

`HistTurn` 为历史对话投影，serde 使用 camelCase：

```rust
struct HistTurn {
    role: String,
    content: String,
    rel_path: Option<String>, // 本历史消息所属文档；缺失时不得当作当前文档事实
    turn_id: Option<String>,
}
```

边界：`history` 不再允许作为裸 `role/content` 事实层输入。AG 后端必须按当前 `active_doc_path`
分流历史：同 `rel_path` 历史可作为同文档 P1 背景；不同 `rel_path` 或缺失 `rel_path` 的历史默认丢弃，
仅当本轮用户显式回指历史时，以“其他文档历史”弱提示注入，且不得覆盖当前 `OpenDocSnapshot`
形成的当前文档事实。

### §2.2 历史恢复接口（BR-AG-PERSIST-001）

历史恢复走 dispatch 信封，属于 workspace 控制态查询/更新：

```typescript
type AgentTurnStatus = "completed" | "error" | "cancelled" | "gate_rejected" | "draft" | "clarifying";

interface AgentSessionDto {
  session_id: string;
  title: string;
  mode: "edit" | "build";
  archived: boolean;
  open_index: number;
  is_active: boolean;
  created_at: number;
  updated_at: number;
  last_active_at?: number | null;
}

interface AgentTurnDto {
  turn_id: string;
  session_id: string;
  status: AgentTurnStatus;
  rel_path?: string | null;
  payload: AgentTurnPayloadV1;
  created_at: number;
  updated_at: number;
}

interface AgentSessionDeleteResult {
  deleted_sessions: number;
  deleted_turns: number;
  deleted_payloads: number;
}
```

`ag.list_sessions` 必须返回全部代理编辑会话元数据；前端打开/切换 workspace 后仅 eager-load
`is_active=true` 的会话 turns，其余会话点击时调用 `ag.list_turns(session_id)` 懒加载。
`ag.save_session` 用于新建、切换、关闭/归档、标题或模式变更后的元数据持久化。
`ag.save_turn` 是前端 UI 投影到后端历史 payload 的唯一写入口；原因是完整助手文本、工具摘要与
Diff 引用在前端流式消费后最完整。`ag_start_turn` 可先创建保底 `draft`，随后前端用同一 `turn_id`
多次 upsert，终态时写最终 status。
`ag.delete_session` 只删除当前 workspace 内指定代理编辑会话的 AG 历史记录，不删除 DE `pending_diffs` /
`diff_payloads`；前端必须在无活动代理回合时调用，成功后从历史/标签列表移除该会话。若删除的是活动
会话则切换到剩余会话，没有剩余会话时新建空会话并重新持久化元数据。

`AgentTurnPayloadV1` 至少包含：

```typescript
{
  version: 1;
  userMessage: { id: string; role: "user"; content: string; parts?: unknown[] };
  assistantMessage?: { id: string; role: "assistant"; content: string; error?: boolean; streaming: false };
  toolSummaries: { id: string; name: string; inputSummary?: string; resultSummary?: string; state: string; anchorMessageId?: string }[];
  diffRefs: { diffId: string; relPath: string; anchorMessageId?: string }[];
  failure?: { code: string; message: string; retriable: boolean };
  incompleteReason?: "app_closed" | "stream_interrupted";
}
```

	工具摘要恢复为折叠只读卡，不绑定实时执行态、不允许从历史重新触发；新写入摘要必须携带
	`anchorMessageId`，恢复时渲染到对应 assistant 消息后方，legacy 缺锚点才进入 fallback。
	DiffCard 只以 `diffRefs` 与 DE/WS 当前 diff 状态/投影合并展示；新写入 `diffRefs.anchorMessageId`
	必须指向本轮 assistant 消息，legacy 缺锚点可按 turn 的 assistant message 派生。payload 不得保存
	diff before/after/styleOps 快照；DE 找不到或已清理的 diff 不得默认恢复为 `pending`，只能只读展示为
	不可用。
`draft` 只表示未完成回合；重试时前端复用同一 `userMessage` 内容新建 `turn_id`，不得覆盖旧记录。

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

`start/end` 为块内纯文本 UTF-16 code unit 偏移区间（CharAnchor，A-ED-I-P-01 §4 /
A-SYS-I-P-01 RefInjection
`block_ranges`）；缺省时降级为块级选中文本引用，不得静默丢弃 `text`。

`references` 为新前端路径主通道，serde `tag="kind"` snake_case，字段 camelCase；覆盖
`doc` / `selection` / `session` / `memory` / `knowledge` / `template` / `url` / `long_text`。
当 `references` 非空时，前端不得再把同一引用重复塞入 `mentions/block_refs`；后端保留旧参数仅为兼容
历史调用。

`active_doc_content` 是兼容字段，必须与 `open_doc_snapshots` 中 `source=active_editor` 的同一路径内容一致。
前端发起代理回合前必须采集全部已打开文档的 OpenDocSnapshot：激活文档取当前 ED 编辑器实时序列化内容；
已打开非激活文档取 `EditSession.content`、`sessionVersion` 与 `contentHash`。不得用打开时的
`loadedContent` 旧快照作为代理回合主事实。active 文档读取有两种合法 source：`active_editor`
表示 ProseMirror 当前绑定已与 `relPath/loadSeq/editFormat` 对齐并完成实时序列化；`active_editor_session`
表示当前文档的 ED session LogicalState 已存在，但 ProseMirror binding 尚未就绪。后者仍是 LogicalState，
不是 DisplayState 反向定义。若已打开文档缺少 LogicalState、active live 序列化失败、
session 版本缺失、内容哈希缺失或当前编辑器锚无法定位，必须阻断本轮代理回合或投 error 卡；
active binding 尚未完成但目标 session LogicalState 已存在时，不得因此回退 DiskState，也不得阻断本轮；
应使用 `active_editor_session` 快照进入 AG 读取链。
不得退回 DiskState 读取。

AG 内部必须建立 `AgentReadContext`：先按 `open_doc_snapshots.rel_path` 命中已打开文档 LogicalState，
未命中时才读取 DiskState。`ReferencePayload.doc`、`ReferencePayload.selection`、`read_doc`、
`search_text`、`ag.list_blocks`、`propose_edit` 的目标内容读取全部使用该上下文。`propose_edit`
指向其他已打开文档时，必须读取该目标文档自身的 OpenDocSnapshot，不得复用当前激活文档快照；指向未打开文档时
才允许读取目标文档 DiskState。

AgentEdit 对文档内容的读取只有一个运行时投影：`AgentEditableSnapshot`。该投影由 AG 读取层统一产出，
字段包括 `{rel_path, editable_kind, source, content_format, content, baseline?, session_version?,
content_hash?, blocks[]}`。活动富文档的 `blocks[]` 必须直接来自同次 ProseMirror
`OpenDocSnapshot.block_snapshots`，字段为 `{block_id,block_index,block_type,node_type,role,
parent_block_id,depth,ordinal,text,marks,block_attrs,node_json,fingerprint,capabilities}`；列表等容器
不得被展平为一个可写文本块。纯文本块保留旧的行级投影。当前编辑器实时内容、`read_doc`、`search_text`、
`ag.list_blocks`、`propose_edit`、ReferencePayload.doc 消费 BDocx 时必须复用该投影。BDocx 必须先经
DOC 编辑链转为 canonical HTML 后投影；原生 `.docx` 不得进入该投影，应返回 `DOC_NOT_BDOCX`。
未打开 HTML 的字符串扫描只作只读兼容；活动富文档缺 ProseMirror 层级块快照时返回
`AG_STRUCTURED_SNAPSHOT_REQUIRED`，不得从序列化 HTML 重新猜测写目标。
`propose_edit` 的 `block_id` 只能传块列表中 `[id=...]` 的完整不透明 id；纯数字块号必须
传 `block_index`。后端对数字型 `block_id` 做 `block_index` 兜底，但该兜底只用于防错，不作为推荐协议。
`propose_edit` 必须声明 `operation`：纯文本已有文档局部编辑使用 `replace_block`；活动富文档内容
编辑只允许 `block_ops + base + operations`，只可命中 `role=content` 且 capabilities 含
`replace_node` 的块。每项必须原样带回 blockId/fingerprint/expectedNodeType，并从 node_json
构造保持类型、blockId、attrs 的 afterNode。BDocx/富文档使用 replace_block 或 replace_document
返回 `AG_RICH_BLOCK_OPERATION_REQUIRED`。新建文档仍可用 `content` 创建路径。
既有文档的 `replace_document` 还必须通过主题/实体身份守卫；若新旧全文重合度极低，返回
`AG_REPLACE_DOCUMENT_IDENTITY_DRIFT`，视为疑似错文档或历史上下文污染，不生成 diff。
`propose_edit` 可携带 `style_ops`（StyleOp 数组，schema 见 A-ED-M-T-01 §5.1）：样式类修改必须经该字段
进入 DE 裁决，不得只用自然语言声明“已加粗/已改样式”。若本轮是编辑意图且当前文档存在，后端必须在
回合结束前收到成功的 `propose_edit` 并发出 `DiffProposed`；否则回 `Error { code: AG_EDIT_TOOL_REQUIRED }`，
前端不得将该回合投影为正常完成或“已修改”。
`style_ops` 在生成 PendingDiff 前必须按 A-SYS-I-P-04 BDocxCapabilityManifest 校验真实 mark 名、
attrs 与操作字段，并按同一 AgentEditableSnapshot 校验 blockId 和块内纯文本半开区间。未知 mark、
错误别名（如 `sub/sup`）、非法 attrs、错误 blockId 或越界/空 range 必须返回
`AG_STYLE_OPERATION_INVALID` / `AG_STYLE_ANCHOR_LOST`，不得持久化 Diff。

成功的 `propose_edit` 必须先通过 `ToolResult { tool_name: "propose_edit" }` 返回后端权威
`DiffProjection` JSON，再发送 `DiffProposed` 通知。`DiffProjection` 至少包含
`{diff_id, rel_path, patch_kind, block_id?, block_index?, before_text, after_text, location_label,
draft_mode, snapshot_source, base_session_version?, base_content_hash?, style_ops?, block_ops?}`，其中 before/after
必须来自同一 `AgentEditableSnapshot` 的目标块或显式 `replace_document` 全文基准。`draft_mode`
枚举为 `current_editor / open_session / background`：目标是激活文档时为 `current_editor`；目标是已打开非激活文档时为
`open_session` 且必须携带 session 基线；目标未打开时为 `background`。前端不得再从 `tool_request.params`
或当前编辑器尾部内容推测 before/after；
缺少 `DiffProjection` 时必须投 error 卡（`DE_DIFF_PROJECTION_MISSING`），不进入 ED 草稿链。

**三终态映射**（BR-SYS-STATE-002）：完成 → `Done`；用户中断 → `Done { stop_reason: "cancelled" }`（无半落盘）；错误 → `Error`。
AG 历史持久化投影：完成→`completed`；`Done(cancelled)`→`cancelled`；门禁拒绝→`gate_rejected`；
`Error`/模型或工具链错误→`error`；命令发起后未到任一终态即退出/崩溃→保留 `draft`。
同一 `turn_id` 的 Done/Error/cancelled/gate_rejected 由 terminal_latch 首次写入即锁定；后续任何终态事件
必须被拒绝和记录诊断，尤其网关 Error 不得被转换为 `RoundOutcome::Done` 后再投 Completed。

**前端消费映射**（PI-AG-10F，`agStore`/`deStore`）：`Chunk`→`appendToMessage`、`ToolRequest`/`ToolResult`→工具卡片；
成功 `ToolResult(propose_edit).projection`→缓存权威 DiffProjection，随后 `DiffProposed`→`deStore.addDiff`
（按 projection 建卡并分流：`current_editor` 进入 ED 草稿链，`open_session` 立即执行 ED session
自动应用事务并禁用卡片裁决，`background` 等待人类接受后落盘，异常则投 error/expired 卡）；`Done`→`finishMessage`、`Error`→`pushError`。

**关联命令形状**（同属契约，前后端对齐）：`ag.cancel_turn(turn_id)`（前端关闭 channel）；产出落
`de.generate_diff`→`DiffProposed`；当前编辑器草稿路径裁决经 `de.accept_draft_diff`/`de.reject_diff`，
后台文本/.html 路径仍可经 `de.accept_diff`/`de.reject_diff`（归 A-DE-I-P-01，本文不重定义）。既有
BDocx 修改的磁盘内容是 docx 字节，未应用到当前编辑器 HTML 草稿的 BDocx diff 不得走后台 raw apply；前端应投
`DE_BDOCX_DRAFT_REQUIRED`，避免把 HTML 当 docx 字节写入。

**冻结约定**：本 schema 一经冻结，字段增删须前后端**双侧同步**并回写本节；桩阶段（GW 未就绪）后端经 `route_start_turn` 桩从 channel 推 `Chunk×N + Done`，前端零改动切换真实。

### §2.1.1 1.0 内部运行契约

以下对象不作为前端必须解释的 UI wire，但属于后端模块间冻结契约，必须版本化并可由 Trace 观察：

```text
IntentResultV1
  intent_type: answer | inspect | search | analyze | edit | create_document | organize_workspace | clarify
  target: active_document | selection | explicit_document | workspace | none
  mutation: none | content_proposal | structure_operation
  ambiguity: none | non_blocking | blocking
  required_context[]
  allowed_tools[]

PromptDataEnvelopeV1
  source_kind / provenance / freshness / trust_level / content_hash / token_estimate / content(data-only)

PromptPackageV1
  runtime_version / intent_id / context_trace_id / tool_manifest_hash / estimated_tokens / messages

CapabilityGrantV1
  turn_id / intent_id / allowed_tools / target_scope / expires_at_turn_terminal
```

`CapabilityGrantV1` 只在当前 turn 有效；注册表中存在但 grant 未包含的工具必须拒绝。上述对象默认只持久化
版本、摘要、哈希和选择原因，不持久化用户正文副本、密钥或模型隐藏推理。

## §3 工具调用回注接口（AG-TOOL / SM-AG-02）

| 接口 | 参数（语义） | 结果 | 特有错误/状态 | 归属 |
|---|---|---|---|---|
| `ag.tool_result`（回注） | turn_id、tool_call_id、执行结果/失败 | 续行回合 | 工具未注册/越权→拒绝并回注拒绝原因（不执行） | SM-AG-02 ToolValidating；BR-AG-STATE-002 |

> 工具执行在回合内编排：只读类工具（读/搜/块读取）经 workspace 限界后统一产出
> `AgentEditableSnapshot`，BDocx 走 DOC 编辑模型而非 WS UTF-8 文本读取；
> 工作区资源工具（创建文档/文件夹、删除、重命名、移动、恢复）只调用 `ws.plan_operation` 生成
> WorkspaceOperationPlan，并通过 DecisionRequest 请求前置授权；确认后 TaskRun 调用 `ws.execute_operation`。
> 它们不生成 DiffCard，也不得直接调用低层 `ws.create_* / delete_* / rename_doc`；
> **写意图工具不直写**——产出修改建议走 `de.generate_diff`（X-FORBID-006）；循环上限/中断由
> SM-AG-02 管理，达上限 LoopAborted。确有资产/锚/动态能力阻断时，`edit_blocked` 返回
> `{status,code,reason,documentModified:false}`，可诚实结束编辑回合而不追加 `AG_EDIT_TOOL_REQUIRED`。
> DecisionRequest 必须引用 plan_id、冻结摘要、影响和基线，模型参数不能伪造确认。
> 用户明确新建文档时生成 CreateDocumentBundle：初始内容在确认前生成；确认后先执行资源创建，再以
> operation_id/origin 调用 `de.generate_diff` 投影初始内容。

| 工具 | 参数（语义） | 结果 | 权限 | 约束 |
|---|---|---|---|---|
| `plan_workspace_operation` | operation intents（create/delete/rename/move/restore）、目标与来源 | plan_id/summary/impact/baselines | StructureWrite | 只生成不可变计划与 DecisionRequest，不执行磁盘操作 |
| `execute_workspace_operation` | plan_id、resolved DecisionRequest identity | receipt/final_state/timeline_entry_id | StructureWrite | 仅 TaskRun 在真实 confirm 后调用；WS 重验、幂等、fail-fast 补偿；重启不自动执行 |
| `import_image_asset` | source_path | assetId/sourcePath/sourceHash/mediaType/byteSize/sourceFilePreserved | StructureWrite | 仅 workspace 相对 PNG/JPEG；受控根内复用，否则复制到图片根 imported 子目录；不得移动或删除源文件 |
| `edit_blocked` | code、reason | 结构化阻断结果 | WriteIntent | 仅真实无法安全提案时使用；不修改文档、不生成 Diff |

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
| `ag.list_blocks` | DocRef、open_doc_snapshots? | 纯文本返回行块；活动富文档返回 `{block_id,block_index,block_type,node_type,role,parent_block_id,depth,ordinal,text,marks,block_attrs,node_json,fingerprint,capabilities}` | 文档不存在/越界；活动富文档缺结构快照 → `AG_STRUCTURED_SNAPSHOT_REQUIRED`；原生 docx → `DOC_NOT_BDOCX` | agent 层级语境、结构化内容/样式动态能力与精确节点基线 |

> 投影 schema 的单一来源在 A-ED-M-T-01 §5.1（ED 定义、AG 承载读取面）；写侧（StyleOp）经
> `ed.apply_edit`（A-ED-I-P-01 §4.1），落盘分流按裁定 D-3（回合产出→DE diff / 显式内联→用户编辑）。
> BDocx 的非当前文档读取必须先经 AgentReadContext：已打开时使用 OpenDocSnapshot，未打开时经 DOC 编辑链转为 HTML 后再分块；原生 `.docx` 不支持 `ag.list_blocks`，
> 应回 `DOC_NOT_BDOCX`，避免把二进制 docx 当 UTF-8 文本或可编辑块列表处理。
> 锚有效期 = 编辑会话内（BR-ED-DATA-004）：文档重开后 blockId 重建，agent 须重新 list_blocks。

### §4.2 富文档 BlockOperation

`propose_edit(operation=block_ops)` 仅接受当前活动 HTML LogicalState。`base` 必须精确匹配当前
`sessionVersion/contentHash`；operations 为 1..32 个不重复 blockId 的 `replace_node`。后端先校验
动态能力、fingerprint、expectedNodeType、blockId/attrs 保持和受控 inline/mark Schema，再从可信
快照补齐 beforeNode。任一项失败整批不生成 PendingDiff；成功后以 `patch_kind=block_ops` 和完整
beforeNode/afterNode 投影给 DE/ED，不生成替换后的 HTML 字符串。

## §5 上下文组装对接（AG-TURN / BR-AG-DATA-001，仅索引）

回合上下文来源（组装在 ag.start_turn 内部完成，限界 X-FORBID-004，显式注入优先保留）：

| 来源 | 供给 | 文档 |
|---|---|---|
| 已打开文档 | `open_doc_snapshots` / `AgentReadContext` / `AgentEditableSnapshot` | 本文 §2.1、A-SYS-I-P-01 §3、A-ED-I-P-01 §4 |
| 未打开文档 | DiskState 读取结果 / `AgentEditableSnapshot` | 本文 §2.1、A-WS-I-P-01 §3、A-DOC-M-T-01 §2 |
| 输入引用 | REF ReferencePayload / 注入载荷 | A-REF-M-T-01 §4、A-REF-I-P-01 §2.1 |
| 记忆 | AST 供给（MemoryInjection） | A-AST-M-T-01 §4 |
| 知识库 | KB 召回（KnowledgeInjection） | A-KB-M-T-01 §3 |

> REF/AST/KB 的供给接口随其各自 I-P（待建）声明；本文只定 AG 消费侧的组装入口。

## §5.1 BDocx 表格语义工具（AG-TURN）

`list_tables(path)` 仅接受当前活动 BDocx HTML 编辑器会话，返回 `sessionVersion`、`contentHash` 与每张表的
`tableId`、`fingerprint`、行列/跨度、`columnWidths`、每个逻辑单元格的 `backgroundColor`、可执行能力及可移动方向。代理修改表格必须先读该投影，
再调用 `propose_edit(operation=table_ops)`，原样带回基准版本与目标锚；不得传原始 HTML。

`operations[]` 仅允许增删行列、首行表头、合并/拆分、纯文本单元格写入、文档流上/下移动和
`set_cell_background`。该底纹操作的 `color` 只接受不透明 `#RRGGBB` 或 `null`，合并单元格只接受
左上锚；它只改节点属性，富内容子树必须保持。后端以会话、内容哈希、表格指纹、坐标范围、动态能力
五重验证；富内容单元格拒绝 `set_cell_text`，不得降级为纯文本。
代理拿到 `AG_TABLE_OPERATION_UNAVAILABLE` 或 `AG_TABLE_RICH_CELL_UNSUPPORTED` 时必须重新读取表格或请用户手工处理。
列宽仅允许人工在编辑器中直接拖拽，当前不作为代理操作，也不承诺 DOCX 往返保真。

## §5.2 BDocx 图片语义工具（AG-TURN）

`list_images(path)` 仅接受当前活动 BDocx HTML 编辑器会话，返回同次 `sessionVersion/contentHash`、
当前图片实例的 `imageId/fingerprint/blockId/ordinal/alt/width/height/mediaRef/standalone/
alignment/capabilities/resizeBounds/moveDirections/positionContext`；`positionContext` 只给出相邻文本
摘要用于文档流定位，不是图片像素、OCR 或视觉识别结果。工具同时返回受配置图片根约束、通过扩展名、文件 magic
与内容哈希校验的项目图片资源
`assets[{assetId,sourcePath,sourceHash,mediaType,byteSize}]`。工具不接受目录、绝对路径或远程
来源参数，不跟随越界符号链接。

代理随后调用 `propose_edit(operation=image_ops)`：插入带
`anchorBlockId/position/assetId/sourcePath/sourceHash` 与可选 alt/width；删除、缩放、移动和对齐带
`imageId/fingerprint` 及对应参数，并服从动态能力。同一图片单批允许各一个 resize/layout/move，
按固定顺序原子执行；同类重复拒绝，delete 独占。任一基线、锚、动态能力
或来源失效时返回明确错误，不生成 Diff；删除只作用于文档图片节点，不删除项目源文件。
workspace 内不在 imageRoots 的普通图片先用 `import_image_asset` 复制准备，再重新 `list_images`；
不得用 `move_doc` 把源图搬走。确实没有安全来源或锚时用 `edit_blocked` 结束回合。

## §5.3 BDocx 能力运行时声明

当前 `AgentEditableSnapshot.editable_kind=bdocx` 时，模型上下文必须附带 A-SYS-I-P-04 定义的
BDocxCapabilityManifest 投影，至少包含：

- `editable_kind=bdocx`、`content_format=html`、manifestId/version/sha256，以及唯一文档内容操作 `block_ops`。
- 可用 StyleOp 的真实 mark/attrs、块属性和各自 DOCX 往返等级。
- 可用 TableOperation、`list_tables` 动态前置条件及表格往返风险。
- 可用 ImageOperation、`list_images` 动态前置条件、项目来源限制及图片往返边界。
- 人工专属能力和当前 Diff 呈现等级。

`propose_edit` function schema 中 BlockOperation/StyleOp/TableOperation/ImageOperation 的 enum、条件必填字段与字段
类型也必须由同一 manifest 生成。静态 Schema 只描述可表达形状；TableOperation 的当前可用性仍以
list_tables 返回值为准，ImageOperation 仍以 list_images 的当前基线、锚和来源哈希为准。manifest
加载或一致性校验失败时返回
`AG_CAPABILITY_MANIFEST_INVALID`，不得以宽泛 object 继续调用模型。

## §6 测试与验收

- **门禁不变量**（BR-AG-VERIFY-001，强制）：静态可查 start_turn/tool 执行/request_assist 三条
  发起路径均无不经门禁的模型调用入口；无订阅时 chat 返回 gate_denied、assist 静默空候选。
- **产出唯一出口**：含修改建议回合经 `de.generate_diff`，无直接写盘（X-FORBID-006 静态+联调）。
- **已打开文档读取门禁**：打开文档的 read_doc/search_text/list_blocks/propose_edit/ReferencePayload 均命中
  OpenDocSnapshot；active live 序列化失败或非激活 session 基线缺失时阻断，不允许 DiskState 兜底。
- **工具循环**：未注册/越权工具拒绝回注；上限生效；写意图工具不直写。
- **辅助采纳路径**：request_assist→mount_decoration→apply_edit 走正常脏态（非 diff），与 ED G5 联调。
- 流式三终态、回合历史恢复、与 DE/ED/WS/REF/AST/KB G5 全链联调。
  历史恢复须覆盖 completed/error/cancelled/gate_rejected/draft；只 eager-load 激活会话，其余懒加载。

## §7 债务与盲区

- 工具注册清单/权限分级表、提示词与模型档延实现期运营配置（接口不锁）。
- request_assist 的节流（前端侧 vs 接口侧）实现期定；候选呈现 schema 联合 ED/DE。
- 构建移交/回落接口（takeover/fallback）随 CBT I-P 联合声明（本文不预定义，AG-04）。
- REF/AST/KB 供给接口待其 I-P；本文 §5 仅占消费入口。
- 历史保留策略覆盖 200MB 软上限、软归档与用户删除单个历史会话。

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
| 2026-07-17 | v1.17 | 补齐 AG 历史恢复接口：`ag_start_turn` 增 `session_id`，新增 `ag.list_sessions` / `ag.save_session` / `ag.save_turn` / `ag.list_turns`；锁定 AgentTurnPayloadV1、completed/error/cancelled/gate_rejected/draft 状态映射、激活会话 eager-load 其余懒加载、工具摘要只读与 Diff 仅引用 |
| 2026-07-18 | v1.18 | 补齐 `ag.delete_session`：当前 workspace 级删除指定 AG 会话/回合/payload 历史记录，保留 DE 修改建议本体；前端无活动代理回合时从历史会话行删除指定会话 |
| 2026-07-18 | v1.19 | `ag_start_turn` 增 `open_doc_snapshots` 与 AgentReadContext 契约：已打开文档 read_doc/search_text/list_blocks/propose_edit/ReferencePayload 必须读取 LogicalState；DiffProjection 增 source 与基线元数据，支持 `draft_mode=open_session` 自动应用 |
| 2026-07-18 | v1.20 | 细化 active 文档 source：新增 `active_editor_session`，补 binding 过渡期以 session LogicalState 读取、HTML 无 block id 投影和整篇替换身份漂移守卫 |
| 2026-07-18 | v1.21 | 补齐 `HistTurn.rel_path/turn_id` 协议：历史消息必须带文档归属；AG 后端按当前文档分流历史，同文档可作背景，跨文档/无归属历史默认不得进入当前文档事实层 |
| 2026-07-18 | v1.22 | 同步启动恢复决议：恢复后的当前文档若已有 ED session LogicalState，即使 ProseMirror binding 尚未完成，也必须进入 AG 工作序列 |
| 2026-07-28 | v1.23 | 增加 BDocx 表格语义工具契约：`list_tables → table_ops`、会话/指纹校验、动态能力、富内容单元格保护与列宽边界。 |
| 2026-07-28 | v1.24 | BDocx TableCellShading：`list_tables` 投影单元格 `backgroundColor`，`table_ops` 增 `set_cell_background`，锁定 #RRGGBB/null、合并左上锚与富内容保持。 |
| 2026-07-28 | v1.25 | 新增 §5.2 BDocxCapabilityManifest 运行时声明；StyleOp 在 PendingDiff 前校验真实 mark/attrs/blockId/range，工具 Schema 与当前文档能力提示使用同一版本/hash。 |
| 2026-07-28 | v1.26 | 明确 StyleOp/CharAnchor 的 start/end 按 UTF-16 code unit 计数，与 ED JavaScript/ProseMirror 偏移平面一致。 |
| 2026-07-29 | v1.27 | BDOCX-IMAGE-AGENT-EDIT-01：新增 list_images 与 image_ops 接口，锁定活动会话图片实例、workspace 项目资源、动态锚/哈希、项目源文件保护及 manifest 同源 Schema。 |
| 2026-07-29 | v1.28 | BDOCX-IMAGE-ADVANCED-AGENT-OPS-01：list_images 增加 assetId、稳定 imageId、动态能力/边界；image_ops 增加缩放、上下移动和独立段落对齐。 |
| 2026-07-29 | v1.29 | BDOCX-IMAGE-COMPOSITE-IMPORT-01：同图 resize/layout/move 组合提案；新增 import_image_asset 与 edit_blocked；list_images 增相邻文本并明确非视觉识别边界。 |
| 2026-07-30 | v1.30 | AG-RICH-BLOCK-STRUCTURED-EDIT-01：活动富文档 `AgentEditableSnapshot` 改由 ProseMirror 层级块快照直投影；新增 `block_ops` 基线/锚/指纹/节点 Schema 批校验和结构化 Diff；BDocx replace_block/replace_document 明确拒绝。 |
| 2026-08-07 | v1.31 | AG-MSG-RENDER-01：§2.1 注明事件到达顺序即时序契约（Chunk→ToolRequest→ToolResult→DiffProposed→下一轮 Chunk，Channel FIFO）；`ag.save_turn` payload 的 `assistantMessage` 可含可选 `segments[]`（additive，恢复时序交错，旧记录降级） |
| 2026-08-08 | v1.32 | 对齐 agent-edit-1.0：ag_start_turn 增显式 runtime_profile 且 1.0 只允许 agent-edit；cancel 绑定后端 CancellationToken；基于 BR-AG-DATA-002/004/005 与 BR-AG-STATE-002 冻结 IntentResultV1/PromptDataEnvelopeV1/PromptPackageV1/CapabilityGrantV1 内部契约；终态首次锁定；新建文档统一 base=empty ChangeProposal，BuildMode 未接独立 Runtime 时拒绝 |
| 2026-08-14 | v1.33 | AG-1.0-IMP-CLARIFICATION-01：AgTurnEvent 增 ClarificationRequired 非终态事件（BR-SYS-STATE-007）；AgentTurnStatus 增 clarifying；SM-AG-01 澄清语义同步（澄清不锁存终态/不发 Done，用户补充经新回合、澄清等待可取消）。 |
| 2026-08-21 | v1.34 | 工作区资源操作基线重构：结构/删除工具统一为 plan_workspace_operation + DecisionRequest + execute_workspace_operation；禁止 Agent 直调低层 WS 文件命令或生成资源 DiffCard；新建使用 CreateDocumentBundle。 |
