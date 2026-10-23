---
文档编号：AG-I-P-01
文档状态：A
负责模块：AG
文档职责：AG 代理模块级接口协议（L2 接口层）——声明代理回合、工具、辅助、流式事件与 agent-edit-1.0 Runtime Profile 的冻结接口
上游约束：A-SYS-I-P-02_IPC协议.md（骨架、流式逻辑契约）、A-SYS-I-P-03_云接口契约.md（CLOUD-GATE/INVOKE、SM-SYS-01、X-SEQ-001）、A-AG-M-T-01_代理技术设计.md（BR-AG-* 规则、AG-TURN/TOOL/ASSIST、X-FORBID-006）、A-AG-M-S-01_代理状态机设计.md（SM-AG-01/02/03/04）、A-DE-I-P-01_DiffEdit接口协议.md（generate_diff 承接产出）、A-ED-I-P-01_编辑器接口协议.md（apply_edit/装饰层）、A-REF-M-T-01（注入载荷）、ADU.md
直接承接：AG command 与 AgTurnEvent wire 契约；门禁/网关经 A-SYS-I-P-03；产出交 DE（de.generate_diff）；辅助采纳经 ED（ed.apply_edit）；显式引用由 REF 供给，用户明确选择的项目记忆按引用消费
使用边界：本文声明 AG 接口语义级签名；门禁/网关协议字段归 A-SYS-I-P-03（不重定义）；落盘经 DE→WS（不重复）；呈现经 ED（不重复）；不新增 RULE/CHAIN/CONSTRAINT/TERM；工具清单/提示词/模型档延实现期运营配置
变更要求：门禁前置语义变更涉 APC §13 须先授权；产出接口变更复核 DE；辅助采纳接口变更复核 ED；工具回注接口变更复核 SM-AG-02
治理成熟度：draft
---

# Binder 代理接口协议（AG · L2 接口层）

## §1 文档定位

```
A-SYS-I-P-02（骨架）+ A-SYS-I-P-03（门禁/网关契约）→ 本文（AG 回合/工具/辅助接口）→ 实现 + @GOV(追溯 A-AG-M-T-01)
门禁前置：每个发起 command 内部必经 ENT 门禁通道（SM-SYS-01 GatePassed）方达网关——X-SEQ-001 在接口层的硬约束
产出：SM-AG-02 `propose_edit` → de.generate_diff；辅助采纳 → ed.apply_edit
```

**接口层门禁不变量（X-SEQ-001 / BR-AG-VERIFY-001）**：下列任一发起 command 的实现路径中，
模型调用前必经门禁；无有效订阅/断网时——代理/工具回合 command 返回 GateDenied 类错误，
辅助类 command 静默返回空候选（不报错、不打断编辑）。接口签名不暴露任何绕过门禁的入口。

## §2 代理回合接口（AG-TURN / SM-AG-01）

| command/事件 | 参数（语义） | 结果 | 特有错误/状态 | 归属 |
|---|---|---|---|---|
| `ag.start_turn` | session_id、`runtime_profile=agent-edit-1.0`、DocRef、prompt、已打开文档 `open_doc_snapshots`、注入引用集（`references` 主通道；旧 `mentions/block_refs` 兼容，可空）、`Channel<AgTurnEvent>` 句柄 | turn_id（SM-AG-01 启动）；流式经 channel 回推（§2.1）；后端先落 `draft`，终态更新 status | **gate_denied**（无订阅/断网，不可绕过）；`AG_RUNTIME_PROFILE_UNSUPPORTED`；事实/组装失败；模型请求澄清则进入原回合等待 | 经门禁→网关；BR-AG-STATE-001/VERIFY-001/PERSIST-001/DATA-002/004/005 |
| `ag.cancel_turn` | turn_id | 触发后端 CancellationToken；中断流和后续工具调度；返回 Cancelled 或 already_terminal | 未知 turn_id→not_found | SM-AG-01 cancel；幂等 |
| `ag.turn_stream`（`Channel<AgTurnEvent>`） | turn_id、channel（随 start_turn 传入） | append-only TurnEvent 的安全 wire 投影（§2.1） | 流式错误→Error 分支（统一错误形状） | BR-SYS-STATE-002 / BR-SYS-STATE-008 |
| `ag.list_sessions` | — | 当前 workspace 的代理编辑会话元数据列表；含 activeSessionId | 无 active workspace → 空列表 | BR-AG-PERSIST-001（标签元数据全恢复） |
| `ag.save_session` | session_id、title、mode、archived、open_index、is_active | ok | 无 active workspace → 错误 | BR-AG-PERSIST-001（标签元数据持久化） |
| `ag.save_turn` | session_id、turn_id、rel_path、AgentTurnPayloadV2（兼容 UI 物化视图） | ok；仅 upsert 可重建展示投影，不写运行状态或 TurnEvent | payload 引用未知 event/diff → 拒绝 | BR-AG-PERSIST-001（兼容投影缓存） |
| `ag.list_turns` | session_id | 指定代理编辑会话的回合历史列表（按 created_at 升序） | session 不属当前 workspace → 拒绝 | BR-AG-PERSIST-001（内容懒加载） |
| `ag.list_turn_events` | turn_id、after_seq? | 该回合按 seq 升序的安全事件投影 | turn 不属当前 workspace → 拒绝 | TERM-AG-022；恢复/补流 |
| `ag.delete_session` | session_id | 删除当前 workspace 指定代理编辑会话及其回合 payload 记录；返回删除计数 | 无 active workspace → 错误；代理回合进行中前端禁用 | BR-AG-PERSIST-001（用户删除历史会话） |

> 含修改建议的回合产出：回合内部经 `de.generate_diff` 交 DE（AG 无直接写盘，X-FORBID-006）；
> 流式仅回显，落盘必经 diff 裁决链。

### §2.1 流式回传契约（AgTurnEvent · 前后端双轨同步冻结点）

> 本节是 AgTurnEvent 前后端 wire contract 的**唯一真源**，由 A-AG-X-L-04 的实现与验证工作承接。
> **物理传输** Tauri 2 `Channel<AgTurnEvent>`（SY-02 选型，per-call channel，不经
> dispatch 信封）；**事件语义**承 BR-SYS-STATE-002 三终态。本节为语义级接口签名，不新增注册类治理块。
>
> **与 SYS `ipc::StreamEvent` 的关系（分层不同·非重复）**：`ipc::StreamEvent` 是通用 IPC 传输契约
> （dispatch 信封 + correlation_id，BR-SYS-STATE-002）；`AgTurnEvent` 是 AG 回合**领域**事件，走
> 独立 per-call `Channel<T>`（channel 即回合关联，故用 turn_id 而非 correlation_id）。终态语义相通，
> 但传输层不同故用独立 wire 类型，**不复用** `StreamEvent`（避免把领域事件埋进通用 `data`），
> 也不与之重名。

**AgTurnEvent envelope 与 payload**（`src-tauri/src/ipc/mod.rs`；字段 camelCase，payload `tag="type"` snake_case）：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub struct AgTurnEvent {
    event_id: String,
    turn_id: String,
    seq: u64,
    occurred_at: i64,
    payload_version: u32,
    event: AgTurnEventPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgTurnEventPayload {
    Chunk        { text: String },
    IntentUpdated { revision: u64, projection: IntentResultV2 },
    RouteUpdated { revision: u64, summary: String, steps: Vec<TaskStepProjectionV2> },
    ToolRequest  { tool_call_id: String, tool_name: String, params: serde_json::Value },
    ToolResult   { tool_call_id: String, tool_name: String, result: String, success: bool,
                   evidence_ids: Vec<String> },
    DiffProposed { diff_id: String, proposal_batch_id: String, group_index: u32,
                   group_count: u32, rel_path: String, task_run_id: Option<String> },
    StepProgress { action: String, label: String, detail: Option<String> },
    DecisionRequestProposed { request_id: String, kind: String, payload: serde_json::Value },
    DecisionRequestResolved { request_id: String, outcome: String },
    ClarificationRequired { text: String },
    Done         { stop_reason: String },
    Error        { code: String, message: String, retriable: bool },
}
```

后端持久化 TurnEvent 可包含不向 UI 公开的 Gate/Context/Provider 元数据事件；AgTurnEvent 只能是同一 seq
空间的安全投影，不得另造顺序或覆盖已发送事件。`IntentUpdated/RouteUpdated` 只呈现可见语义与候选工作，
不得携带隐藏推理、CapabilityGrant 或“执行下一步”命令。

> **澄清等待事件（2026-08-14，AG-1.0-IMP-CLARIFICATION-01 / BR-SYS-STATE-007）**：
> `ClarificationRequired` 是**非终态**事件——回合不终结、不锁存终态、不发 Done；前端投影为
> `clarifying` 等待态（AgentTurnStatus）；用户补充输入时使用同一 `turn_id` 受控重入原 AgentTurn 的
> Assembling，不取消、不创建新 AgentTurn；用户也可在此阶段显式取消。
> 澄清事件由模型可见输出产生；进入等待后不调用门禁/网关/工具。用户补充后重新组装和过 Gate。

> **事件顺序即时序契约（2026-08-07，承接 BR-AG-UI-001 / TERM-AG-011）**：同一回合内
> `Chunk / IntentUpdated / RouteUpdated / StepProgress / ToolRequest / ToolResult / DiffProposed / DecisionRequest* / ClarificationRequired / 终态`
> 按实际发射顺序经 Channel
> FIFO 到达前端；前端据此重建"正文↔工具卡↔Diff 卡"的交错时序（消息片段），接口分支本身不变。

**ag_start_turn 现有 v1 命令签名**（独立 `tauri::command`，不走 dispatch 信封；P00 的 v2 设计见 §2.3，尚未接入）：

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
    agent_context_facts: Option<AgentContextFactsV1>, // 缺失/未知/身份漂移在 Gate 前失败关闭
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

`agent_context_facts` 使用 A-SYS-I-P-01 BR-SYS-DATA-008 的 `agent-context-facts-v1` 共享结构：只携
workspace/turn 身份、活动与打开会话元数据、显式 SelectionFact 和当前 AgentSession 的 ProposalFact，
不复制文档正文。ProposalFact 固定携 `diffId/sessionId/relPath/state/effectStatus/saved`，不携带 diff 内容。
后端必须逐项与同轮 `open_doc_snapshots`、active workspace、active_doc_path、AgentSession、
`references.selection` 及兼容 `block_refs` 交叉核验；任何 source/sessionVersion/contentHash/路径/身份不一致均在
Gate/provider/ToolCall 前拒绝。
该参数是事实载荷，不是 CapabilityGrant 或 ExplicitEditTargetSet 的授权来源。

`HistTurn` 为历史对话投影，serde 使用 camelCase：

```rust
struct HistTurn {
    role: String,
    content: String,
    rel_path: Option<String>, // 本历史消息所属文档；缺失时不得当作当前文档事实
    turn_id: Option<String>,
}
```

边界：`history` 不再允许作为裸 `role/content` 事实层输入。AG 后端按 `active_doc_path` 和预算形成带来源的
有限历史候选：同 `rel_path` 可作为同文档 P1 背景，其他历史只能以明确标注来源的弱摘要进入，均不得
覆盖当前 OpenDocSnapshot。是否沿用由模型写入 IntentResult revision；后端不得通过关键词先行固定语义，
历史身份也不得扩大 ExplicitEditTargetSet 或 CapabilityGrant。

历史 wire DTO 不变。运行时只对本次模型输入生成有标记的历史摘录，不覆盖持久化消息；同 AgentTurn
用户与助手消息优先作为一个对话单元处理。全文、摘录与未纳入历史必须可区分，摘录保留身份但不授予
任何目标或工具权限。`history_assembled` 为内部脱敏事件，记录候选、纳入、丢弃、摘录数量、字符数及
由完整请求剩余容量派生的历史额度；不作为用户回答、业务完成或长期记忆落库证据。

### §2.2 历史恢复接口（BR-AG-PERSIST-001）

历史恢复走 dispatch 信封，属于 workspace 控制态查询/更新：

```typescript
type AgentTurnStatus = "completed" | "error" | "cancelled" | "gate_rejected" | "draft" | "clarifying" | "awaiting_decision";

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
  payload: AgentTurnPayloadV2;
  created_at: number;
  updated_at: number;
}

interface AgentSessionDeleteResult {
  deleted_sessions: number;
  deleted_turns: number;
  deleted_payloads: number;
  deleted_events: number;
}
```

`mode="build"` 仅是历史/additive 会话元数据兼容值，不对应 Agent 1.0 可启动 RuntimeProfile、入口或模型调用；
`ag.start_turn` 在 1.0 仍只接受 `agent-edit-1.0`。

`ag.list_sessions` 必须返回全部代理编辑会话元数据；前端打开/切换 workspace 后仅 eager-load
`is_active=true` 的会话 turns，其余会话点击时调用 `ag.list_turns(session_id)` 懒加载。
`ag.save_session` 用于新建、切换、关闭/归档、标题或模式变更后的元数据持久化。
后端 TurnEvent 与领域事实是运行真源：`ag_start_turn` 先创建 `draft` 并开始追加事件，状态只由状态机与
terminal_latch 派生。`ag.save_turn` 仅保留为前端展示物化缓存的兼容入口，不能覆盖 status、事件顺序、
工具结果、DecisionRequest 或 Diff 事实；缺失/损坏时必须能用 `ag.list_turn_events` 与 DE/WS 事实重建。
`ag.delete_session` 只删除当前 workspace 内指定代理编辑会话的 AG 历史记录（含 TurnEvent），不删除 DE `pending_diffs` /
`diff_payloads`；前端必须在无活动代理回合时调用，成功后从历史/标签列表移除该会话。若删除的是活动
会话则切换到剩余会话，没有剩余会话时新建空会话并重新持久化元数据。

`AgentTurnPayloadV2` 至少包含：

```typescript
{
  version: 2;
  lastEventSeq: number;
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
以下完整采集要求仅为 v1 兼容契约；v2 按 §2.3 实施，不得静默将 v1 缺失字段视为 v2。
v1 前端发起代理回合前必须采集全部已打开文档的 OpenDocSnapshot：激活文档取当前 ED 编辑器实时序列化内容；
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
未命中且可信打开资源清单确认目标未打开时才读取 DiskState；v2 已打开但尚未采集的条目必须先取得
LogicalState，不能仅凭快照未命中回退。`ReferencePayload.doc`、`ReferencePayload.selection`、`read_doc`、
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
进入 DE 裁决，不得只用自然语言声明“已加粗/已改样式”。若模型当前 IntentResult revision 明确表达
`side_effect_intent=content_proposal`，但最终回答前没有成功 `propose_edit`，运行时必须在预算内把
“尚未产生 Proposal”作为可恢复约束回注；仍未产生时以结构化 incomplete/error 结束，不得投影为
“已修改”。该检查只消费模型语义事件与真实 Proposal 事件，不得重新引入关键词编辑分类器。
`style_ops` 在生成 PendingDiff 前必须按 A-SYS-I-P-04 BDocxCapabilityManifest 校验真实 mark 名、
attrs 与操作字段，并按同一 AgentEditableSnapshot 校验 blockId 和块内纯文本半开区间。未知 mark、
错误别名（如 `sub/sup`）、非法 attrs、错误 blockId 或越界/空 range 必须返回
`AG_STYLE_OPERATION_INVALID` / `AG_STYLE_ANCHOR_LOST`，不得持久化 Diff。

成功的 `propose_edit` 必须先通过 `ToolResult { tool_name: "propose_edit" }` 返回后端权威
`DiffProjection` JSON，再发送 `DiffProposed` 通知。`DiffProjection` 至少包含
`{diff_id, proposal_batch_id, group_index, group_count, rel_path, patch_kind, block_id?, block_index?, before_text, after_text, location_label,
draft_mode, snapshot_source, base_session_version?, base_content_hash?, style_ops?, block_ops?}`，其中 before/after
必须来自同一 `AgentEditableSnapshot` 的目标块或显式 `replace_document` 全文基准。`draft_mode`
枚举为 `current_editor / open_session / background`：目标是激活文档时为 `current_editor`；目标是已打开非激活文档时为
`open_session` 且必须携带 session 基线；目标未打开时为 `background`。前端不得再从 `tool_request.params`
或当前编辑器尾部内容推测 before/after。同一 `propose_edit` 可返回一个 ProposalBatch 内的多个
DiffProjection：不连续或可独立裁决的块组各自一个 diff_id，连续且语义不可分的相邻块可合并；每个投影
独立进入 SM-DE-01。缺少任一权威 DiffProjection 时必须投 error 卡（`DE_DIFF_PROJECTION_MISSING`），
不进入 ED 草稿链。

**三终态映射**（BR-SYS-STATE-002）：完成 → `Done`；用户中断 → `Done { stop_reason: "cancelled" }`（无半落盘）；错误 → `Error`。
AG 历史持久化投影：完成→`completed`；`Done(cancelled)`→`cancelled`；门禁拒绝→`gate_rejected`；
`Error`/模型或工具链错误→`error`；命令发起后未到任一终态即退出/崩溃→保留 `draft`。
同一 `turn_id` 的 Done/Error/cancelled/gate_rejected 由 terminal_latch 首次写入即锁定；后续任何终态事件
必须被拒绝和记录诊断，尤其网关 Error 不得被转换为 `RoundOutcome::Done` 后再投 Completed。

**前端消费映射**（PI-AG-10F，`agStore`/`deStore`）：`Chunk`→`appendToMessage`、`ToolRequest`/`ToolResult`→工具卡片；
成功 `ToolResult(propose_edit).projections[]`→按 ProposalBatch 缓存完整权威 DiffProjection 集合，随后
核对全部 `DiffProposed` 的 proposal_batch_id、group_index/group_count 后一次 fan-out 到消息流。`current_editor`
同文档同批结构化多卡必须收齐后聚合 operations，只提交一次 ED 前向事务，再把各卡标记为已预览且
保持 pending；不得逐卡应用导致兄弟卡因整篇版本推进而失效。`open_session` 只处理其受支持的单组或可
原子聚合载荷，`background` 等待人类接受后落盘；异常投 error/expired 卡。各卡接受/拒绝仍为独立
SM-DE-01 事件，反向只作用于本卡目标；
`Done`→`finishMessage`、`Error`→`pushError`。

**关联命令形状**（同属契约，前后端对齐）：`ag.cancel_turn(turn_id)`（前端关闭 channel）；产出落
`de.generate_diff`→`DiffProposed`；当前编辑器草稿路径裁决经 `de.accept_draft_diff`/`de.reject_diff`，
后台文本/.html 路径仍可经 `de.accept_diff`/`de.reject_diff`（归 A-DE-I-P-01，本文不重定义）。既有
BDocx 修改的磁盘内容是 docx 字节，未应用到当前编辑器 HTML 草稿的 BDocx diff 不得走后台 raw apply；前端应投
`DE_BDOCX_DRAFT_REQUIRED`，避免把 HTML 当 docx 字节写入。

**冻结约定**：本 schema 一经冻结，字段增删须前后端**双侧同步**并回写本节；历史桩或测试事件不得反向定义当前 wire contract。

### §2.1.1 1.0 内部运行契约

以下对象不作为前端必须解释的 UI wire，但属于后端模块间冻结契约，必须版本化并可由 Trace 观察：

```text
IntentResultV2
  revision / source_event_seq
  goal / intent_type
  target_candidates[]
  side_effect_intent: none | content_proposal | structure_plan
  ambiguity: none | non_blocking | blocking
  required_context[] / confirmation_points[]
  // 不含 allowed_tools，不是授权对象

TurnEventV1 (TERM-AG-022)
  event_id / turn_id / seq / occurred_at / kind / payload_version / payload

EvidenceLedgerV1 (TERM-AG-023)
  turn_id / through_seq
  evidence_items[]: evidence_id, rel_path, source, coverage, version_or_hash, source_event_id

ModelFinalEnvelopeV2
  answer_markdown
  findings[]?: statement, evidence_ids[], uncertainty?

TaskRunProjectionV2 / TaskPlanProjectionV2 / TaskStepProjectionV2
  task_run_id / source_turn_ids[] / through_seq / goal / status
  observed_steps[] / waiting_on[] / current_route_revision? / candidate_steps[]
  // 不含 required_grant、执行游标或执行命令

ProjectReportV2 (TERM-AG-021, runtime-derived)
  version=project-report-v2 / turn_id / evidence_through_seq / requested_scope[]
  read_items[]: rel_path, source, coverage, evidence_ids[]
  unread_items[]: rel_path, reason
  findings[]: statement, evidence_ids[], uncertainty
  uncertainties[]

兼容读取期的 IntentResultV1（只读迁移，不得用于授权）
  intent_type: answer | inspect | search | analyze | edit | create_document | organize_workspace | clarify
  target: active_document | selection | explicit_document | workspace | none
  mutation: none | content_proposal | structure_operation
  ambiguity: none | non_blocking | blocking
  required_context[]
  allowed_tools[]

AgentContextFactsV1 (TERM-AG-020)
  version=agent-context-facts-v1 / workspace_id / turn_id
  active_doc? / open_docs[]: rel_path, source, edit_format, content_format, dirty, baseline?, session_version, content_hash
  selections[]: rel_path, block_id, start?, end?
  proposal_facts[]: diff_id, rel_path, state

PromptDataEnvelopeV1
  source_kind / provenance / freshness / trust_level / content_hash / token_estimate / content(data-only)

PromptPackageV2
  runtime_version / intent_revision / context_trace_id / capability_grant_id / tool_manifest_hash / estimated_tokens / messages

CapabilityGrantV2
  grant_id / turn_id / computed_at_seq / allowed_tools / readable_scope
  explicit_edit_targets_hash / permitted_effect_paths / format_capability_hash
  dynamic_capability_hash / decision_receipt_ids[] / budgets / expires_after_tool_call
```

`CapabilityGrantV2` 在每次工具请求前按当前事实重新计算，动作结束即失效；IntentResult、TaskPlan 和兼容
IntentResultV1.allowed_tools 不参与授权。注册表中存在但当前 grant 未包含的工具必须拒绝，Prompt/tool
manifest 只投影 provider 调用前计算的工具集。TurnEvent 持久化不可变事实；其余 projection 可重建。
成功只读 ToolResult 的 `evidence_ids` 由运行时生成并与 EvidenceLedger 同源，模型不可自定义。运行时从
ModelFinalEnvelopeV2 派生 ProjectReport：只保留当前 turn、`evidence_through_seq` 范围内存在且覆盖等级
足够的引用；未知/跨回合/覆盖不足的 finding 转入 uncertainties。该 envelope 不是工具调用或授权来源。
默认不持久化额外用户正文副本、密钥或模型隐藏推理。

### §2.3 v2 通用准入、按需上下文与独立验证（P00）

**状态**：P03 已开始接入生产 v2 DTO、准入和宿主回传；当前验证结论见开发计划 §14.12，不以本节登记提升 G5。共享字段唯一来源为
A-SYS-I-P-01 §3.1.1，状态转移为 A-AG-M-S-01 §2。v1 完整快照路径与历史读取保持原解释。
v2 不新增产品 Profile；前后端通过显式 facts version 选择契约，在同一运行核心汇合，禁止两套执行真值。

| step | actor | input | action/output | next / exception |
|---|---|---|---|---|
| 1 | UI→AG | 当前输入、workspace/session/turn、RuntimeProfile、显式引用 | 冻结 activeTarget/openResources 身份与可用性，持久化输入 | 身份无效拒绝；无关文档 loading 不拒绝 |
| 2 | AG | AgentContextFacts v2、当前输入、必要历史 | 组装轻量 PromptPackage；专业正文未激活时不注入；经过真实 Gate | 非法版本/身份、预算失败→TurnError |
| 3 | 模型→AG | 现有工具请求或可见上下文需求 | 核心校验协议/工具参数，专业适配核验读取范围与就绪性 | 需求不等于授权，不按关键词预判写权限 |
| 4 | 专业适配→AG | 冻结目标、当前版本、需要的内容/结构 | 按需采集 LogicalState，记录新 contextRevision；核验后才执行读取/提案 | stale/unavailable→明确 ToolResult 或必需装配失败，不走旧磁盘兜底 |
| 5 | AG | ToolResult/专业回执、当前事实 | 按现有状态机重新装配并 Gate；每次执行重新 grant | 不直接由 TaskPlan 调度、不自动重放 |

**必需与可选**：显式引用必须保留选择及可用性事实；合法未就绪引用可作为元数据让模型解释缺口，
不能伪称已读或作为编辑锚。纯对话和 workspace_glob 不要求文档正文；一旦动作需要正文/Block，
必先取得有效快照。采集等待处于当前 Assembling 或工具 Executing 的有界操作，由取消信号及配置期限约束；
期限届满返回明确 unavailable/错误，不新增无期限后台轮询。人工补充/裁决继续使用既有等待态。

**目标稳定**：activeTarget 在提交时冻结；切换标签不重定向。继续必须携原 session/turn/task/request 身份，
在使用资源前验证当前版本；旧 selection 和旧回执不能作用于新内容。资源清单而非快照命中决定 opened 状态。

**兼容**：实现期将 `agent_context_facts` 改为受 version 分派的输入，v1 进入严格校验适配，v2 进入按需路径；
`active_doc_content` 在 v2 未取得内容时允许省略，已提供则必须与对应 ready 快照一致，绝不作为兜底。
重放旧事件只恢复事实；不把 v1 draft 转成 v2 活动执行。前后端仅一侧支持 v2 时显式不支持，不自动降级。

**P03 物理接线（2026-09-20）**：`ag_start_turn` 新增可缺省 `onContextRequest` 专用 Channel；
v1 旧调用不需要该 Channel，v2 的内容操作缺通道时返回 `AG_CONTEXT_HOST_UNAVAILABLE`。
请求为 `{requestId,workspaceId,sessionId,turnId,contextRevision,paths,allDocuments}`；宿主经
`ag_supply_context({response:{requestId,facts,snapshots}})` 回传完整 v2 事实及仅本次需要的快照。
`requestId` 为运行时生成的 UUID；响应必须匹配待处理项的四项身份及精确 revision，一次消费。
采集最多等待 5 秒，取消检查间隔最多 20ms，阻塞等待不占用异步执行线程；超时、取消、Future 丢弃后清除待处理项，
迟到/重复/跨身份响应拒绝。回传正文不写入安全 TurnEvent，不进入全局核心提示词；ContextFrozen 仅记录版本/身份/可用性与证据元数据。
初始请求不采集无关活动正文；显式选区按需核验。工具采集先过权限预检，随后仍计算并消费独立的真实执行 grant。
`read_doc/list_blocks/list_tables/list_images/search_text/propose_edit` 请求所需事实；纯对话、list_docs、workspace_glob 不请求正文。
每次有效采集追加 revision；恢复原 AgentTurn 要求 revision 大于已持久化版本且 activeTarget 相同。
打开资源清单来自文档标签及活动路径，不能以 ED 保留的已关闭缓存代替；本 turn 曾打开后关闭的路径保留 unavailable，不能改走磁盘。
读取失败不表示专业修改成功；资源不可用的安全工具结果附稳定错误码与重新提交提示。

**确认/回执**：DecisionRequestResolved 只消费对应 requestId；按钮 Promise、迟到事件和 finally 只能
清理自己的 session/turn/request。确认 A 后产生 B 时，A 收尾不得删除 B。DecisionBar 保持 Composer 上方
固定区域，消息历史保留来源与裁决；当前会话只操作有效请求。专业预览继续区分提案生成、预览 applied/failed、
ack 接受/拒绝、continuation 启动/失败；失败必须有可见原因和下一步，不只清空 loading 或打印 console。
重复 ack 不重复启动；预览继续不等于接受，也不等于保存。专业状态由 DE/ED/WS 回执证明。

P06 实现登记（2026-09-22）：DecisionBar 的 busy/error 绑定 session/turn/request，提交回执写原会话；
请求清理使用三项精确匹配，按钮 finally 只清理自己安装的动作对象，终态只清理原 AgentTurn 的动作。
服务在同一 requestId 提交期间拒绝重复调用（AG_DECISION_IN_FLIGHT），结束或失败后释放在途锁；
仅保有原等待身份的请求可继续原 AgentTurn。工作区切换后的迟到结果拒绝在新工作区投影/继续
（AG_DECISION_CONTEXT_CHANGED）；若裁决已成功，错误明确标记 decisionResolved，不把它说成未执行。
旧调用未传 sessionId 时在 store 入站时固定会话，只保证入站之后不随活动会话漂移；不替代上游来源绑定。
自动化状态见计划 §14.18；不承诺页面重启自动恢复或后端副作用重放，不修改确认权限与保存语义。

**测试边界**：确定性模型响应只能替换传输源，字节分片/非法 JSON/缺结束必须经过生产解析与核心；
如需受控 Gate 响应，只能在测试注入边界替代外部服务，不能删除 Gate 检查，不作为真实订阅通过证据。
测试能力必须显式 test 标识且默认关闭，不允许覆盖生产工具或授予额外权限。真实 workspace_glob 和纯文本
能力持续验证契约；核心套件不得初始化复杂编辑器、DOCX 转换或 LibreOffice。测试入口必须仅在开发/测试构建
可用；不提供任意生产正文录制、用户库修改或通用脚本执行入口。具体 command 注册由 P02/P08 Scope 定义。

## §3 工具调用回注接口（AG-TOOL / SM-AG-02）

| 接口 | 参数（语义） | 结果 | 特有错误/状态 | 归属 |
|---|---|---|---|---|
| `ag.tool_result`（内部记录） | turn_id、tool_call_id、执行结果/失败、运行时生成的 evidence_ids（只读成功时） | 追加 ToolResult TurnEvent；从成功只读结果重建 EvidenceLedger；原回合返回 Assembling | 工具未注册/当前未授权/越权→记录拒绝（不执行）；模型自定义 evidence_id → 丢弃 | SM-AG-02 ToolValidating；BR-AG-STATE-002 |

> 工具执行在回合内编排：只读类工具（读/搜/块读取）经 workspace 限界后统一产出
> `AgentEditableSnapshot`，BDocx 走 DOC 编辑模型而非 WS UTF-8 文本读取；
> 工作区资源工具（创建文档/文件夹、删除、重命名、移动、恢复）只调用 `ws.plan_operation` 生成
> WorkspaceOperationPlan，并通过 DecisionRequest 请求前置授权；确认后原 AgentTurn 的运行时 effect handler 调用 `ws.execute_operation`。
> 它们不生成 DiffCard，也不得直接调用低层 `ws.create_* / delete_* / rename_doc`；
> **写意图工具不直写**——产出修改建议走 `de.generate_diff`（X-FORBID-006）；循环上限/中断由
> SM-AG-02 管理，达上限 LoopAborted。确有资产/锚/动态能力阻断时，`edit_blocked` 返回
> `{status,code,reason,documentModified:false}`，可诚实结束编辑回合而不追加 `AG_EDIT_TOOL_REQUIRED`。
> DecisionRequest 必须引用 plan_id、冻结摘要、影响和基线，模型参数不能伪造确认。
> 用户明确新建文档时生成 CreateDocumentBundle：初始内容在确认前生成；确认后先执行资源创建，再以
> operation_id/origin 调用 `de.generate_diff` 投影初始内容。
> 每次工具结果记录后先回到 SM-AG-01 Assembling；若要再次调用 provider，必须重建 PromptPackage 并取得
> 新的真实 GatePassed。首轮或上一轮 GateOutcome 不得复用，门禁拒绝/错误结束当前循环并投影 Error。

| 工具 | 参数（语义） | 结果 | 权限 | 约束 |
|---|---|---|---|---|
| `plan_workspace_operation` | operation intents（create/delete/rename/move/restore）、目标与来源 | plan_id/summary/impact/baselines | StructureWrite | 只生成不可变计划与 DecisionRequest，不执行磁盘操作 |
| `import_image_asset` | source_path | assetId/sourcePath/sourceHash/mediaType/byteSize/sourceFilePreserved | StructureWrite | 仅 workspace 相对 PNG/JPEG；受控根内复用，否则复制到图片根 imported 子目录；不得移动或删除源文件 |
| `edit_blocked` | code、reason | 结构化阻断结果 | WriteIntent | 仅真实无法安全提案时使用；不修改文档、不生成 Diff |

`execute_workspace_operation` 不是模型工具：仅 SM-AG-01 在收到与冻结 plan 匹配的真实 confirm 回执后调用
WS，WS 仍执行重验、幂等和 fail-fast 补偿。ProjectReport 也不是模型工具：运行时从 EvidenceLedger 与
最终回答派生，工具清单不得注册或暴露 `report_project`。

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

#### §4.1.1 模型只读工具的有界返回（P05）

本节针对模型工具 `read_doc/list_blocks`，不改变上表 IPC `ag.list_blocks` 的完整块投影接口。

- `read_doc(path, view?, offset?, limit_chars?, expected_hash?)` 默认为正文阅读视图（`view=text`）。返回去除编辑标记的正文 `content`、原来源/版本元数据、`source_content_format`、`version_hash`、`offset/end_offset/next_offset/total_chars/complete/has_more`；HTML 保留段落及表格行列分隔，不重复容器与子节点正文。offset 为阅读投影中的 Unicode 字符位置（不是块号、UTF-16编辑坐标），默认0；limit_chars 默认/最大10,000，实际每页还按原序列化安全上限收缩。非首页必须提供 expected_hash；应使用返回的 next_offset，不猜游标。长单段允许明确分片，但绝不截断 JSON 或编辑节点。
- `read_doc(path, view=structure)` 保留原完整 content 与元数据；原 `read_doc(path, blocks=[...])` 默认保留结构视图，返回所选块的完整 node_json 等字段且省略全文。需要编辑节点时沿用此接口；任一块不存在即 AG_READ_BLOCK_NOT_FOUND，不返回部分成功。结构视图不能使用正文分页参数；正文 view 不与 blocks 混用。
- `list_blocks(path, offset?, limit?, expected_hash?)` 返回轻量目录、`offset/total_blocks/next_offset/complete/version_hash`。默认 offset=0、limit=20，limit 范围1～50，为每页条目数上限；容量不足时减少完整条目，不截断条目，next_offset反映实际返回数量，禁止以offset+limit猜下一页。后续页必须携带上一页version_hash作为expected_hash。complete只表示本次目录覆盖全部块，不表示正文已读；末页不等于全目录返回。
- `version_hash` 为完整 AgentEditableSnapshot 序列化的 SHA-256；`read_doc` 也接受 `expected_hash`。版本变化返回 `AG_READ_VERSION_STALE`，重新定位，不将新旧页拼作同一版本。此 hash 不替代编辑提案原有 contentHash/sessionVersion/fingerprint。
- 只读结果按 JSON 字符串再次序列化后的字符数限制在运行时单次上限内，当前为12,000，不扩大该限制。正文阅读按容量成页；显式结构结果仍在超限时 AG_TOOL_RESULT_TOO_LARGE，编辑节点不可截断。目录页过大应减小 limit；大结构块可用正文视图阅读，但不等于取得可编辑完整节点。其他只读列表超限仍明确失败，不保证均支持分页。
- 只有成功结果才能登记证据：正文单页从0覆盖至末尾（complete=true）或完整结构读取为 full_document；任何部分页（包括末页）及选块为 selected_blocks，目录为 structure_index。正文证据的覆盖范围同时记录 offset/end_offset/total_chars，跨页不自动升级全文；版本绑定成功返回值，不二次读取磁盘替换版本。分页是新的只读 ToolCall，仍受原授权、调用/Prompt预算和取消约束，不是自动重试或后台执行恢复。

兼容：原 path 和 blocks 参数保留；需要原HTML/编辑模型全文须显式 view=structure，默认正文的 content_format=text 不可作为HTML编辑基准。IPC 块读取、授权、编辑和保存契约不变。本限制约束返回规模，不承诺底层文件读取内存有界。

### §4.2 富文档 BlockOperation

`propose_edit(operation=block_ops)` 仅接受当前活动 HTML LogicalState。`base` 必须精确匹配当前
`sessionVersion/contentHash`；operations 必须非空、不重复，并同时满足当轮 manifest/schema 与执行 guard
给出的有效上限；接口文档不另设数字。后端先校验
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
| 用户明确选择的项目记忆 | REF/AST 归一的 data-only ReferencePayload | A-AST-M-D-01、A-REF-M-T-01 |
| 知识库/模板 | `agent-edit-1.0` 不自动召回、不自动注入 | A-CORE-X-D-02 §3 |

> 本文只定义 AG 消费入口；项目记忆必须有当前 workspace 的用户明确选择事实，工具注册或存在资产不构成注入授权。

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
- **工具循环**：每个工具请求都重新计算 CapabilityGrant；IntentResult/TaskPlan 变化不直接扩权；未注册/越权工具拒绝记录；ToolResult 后回 Assembling 并重新 Gate；上限生效；写意图工具不直写。
- **事件与恢复**：TurnEvent seq 单调且不可覆盖；Channel 断线可由 after_seq 补流；删除 UI payload 后能从
  事件、DE/WS 事实重建同一终态、工具卡顺序、IntentResult/TaskRun 投影和 ProjectReport。
- **证据关联**：只读成功结果由运行时分配 evidence_id；ProjectReport 仅接受同回合账本中覆盖等级足够的
  finding 引用，伪造/跨回合/目录或搜索冒充全文的引用进入 uncertainties，不提高 readItems 覆盖。
- **提案分组**：一个 ProposalBatch 的不连续或可独立裁决块产生多个 DiffProposed，group_index 连续且
  每个 diff_id 独立裁决；相邻但语义可分的修改不得仅因位置连续强制合并。
- **辅助采纳路径**：request_assist→mount_decoration→apply_edit 走正常脏态（非 diff），与 ED G5 联调。
- 流式三终态、回合历史恢复、与 DE/ED/WS/REF 的当前主链接口联调；项目记忆仅验证用户明确选择的
  data-only 引用，KB/TPL 自动供给不属于 Agent 1.0 验收。
  历史恢复须覆盖 completed/error/cancelled/gate_rejected/draft；只 eager-load 激活会话，其余懒加载。

## §7 债务与盲区

- 工具注册清单、提示词与模型档延实现期运营配置（接口不锁）；权限算法的输入边界已由 CapabilityGrantV2 锁定。
- request_assist 的节流（前端侧 vs 接口侧）实现期定；候选呈现 schema 联合 ED/DE。
- P2 Build/CBT 不属于本接口协议；未来独立 RuntimeProfile 必须重新定义接口，不复用历史 takeover/fallback。
- 外部内容、项目记忆等 P1 能力只有完成各自 DESIGN Scope 后才扩展消费接口；当前不得由 `ReferencePayload` 类型存在推断能力已开放。
- 历史保留策略覆盖 200MB 软上限、软归档与用户删除单个历史会话。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-10-23 | v1.42 | AG-HISTORY-BUDGET-01：历史wire不变，按身份保留连续对话单元、有标记原文摘录和脱敏装配证据；不改变授权。 |
| 2026-10-23 | v1.41 | AG-BOUNDED-READING-01：正文/编辑结构视图分离、容量与版本绑定分页、目录自适应及保守页证据；编辑和保存边界不变。 |
| 2026-09-22 | v1.40 | P06：请求身份清理、单次在途提交、原会话回执及工作区变化拒绝继续；保留恢复边界。 |
| 2026-09-21 | v1.39 | P05：模型读取的单正文、选块完整性、目录分页和版本绑定；区分 IPC 与工具契约。 |
| 2026-09-20 | v1.38 | P03：登记按需宿主 Channel/回传 command、一次消费与身份/版本/取消/超时边界；实现验收见计划 §14.12。 |
| 2026-09-15 | v1.37 | P00：分离 v1 现行路径与 v2 按需准入设计；明确上下文取得、DecisionRequest/预览回执身份及开发验证入口边界，不改变生产授权与保存语义。 |
| 2026-09-03 | v1.36 | AG-ADAPTIVE-RUNTIME-DESIGN-01：AgTurnEvent 改为带 seq 的 append-only 安全投影并新增 Intent/Route 事件；内部契约升级为 IntentResult/CapabilityGrant/PromptPackage V2、TurnEvent/EvidenceLedger 与任务投影；移除 report_project 和模型可见 WS 执行工具；ProposalBatch 支持多 DiffProjection。 |
| 2026-08-30 | v1.26 | AG-CONFORMANCE-CODE-S4A：`ag_start_turn` 新增版本化 AgentContextFacts；新增只读 `report_project` 完成工具与同轮读取证据失败关闭协议。 |
| 2026-08-31 | v1.35 | AG-DOC-CONSOLIDATION-01：事件协议收敛到当前十个 AgTurnEvent 分支；澄清复用原 turn_id 并回到 Assembling；CapabilityGrant 仅携带当轮精确允许集，每次 provider continuation 必须重新门禁；数值上限仅由运行时 manifest/schema/guard 作真源。 |
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
