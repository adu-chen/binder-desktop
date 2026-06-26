---
文档编号：AG-I-P-01
文档状态：A
负责模块：AG
文档职责：AG 对话代理模块级接口协议（L2 接口层）——按 A-SYS-I-P-02 骨架声明 chat 回合 command、工具调用回注接口、内联辅助/自动补全接口、上下文组装与门禁前置对接、流式回传事件；AI 调用面（一切模型调用经门禁→网关）
上游约束：A-SYS-I-P-02_IPC协议.md（骨架、流式逻辑契约）、A-SYS-I-P-03_云接口契约.md（CLOUD-GATE/INVOKE、SM-SYS-01、X-SEQ-001）、A-AG-M-T-01_对话代理技术设计.md（BR-AG-* 规则、AG-TURN/TOOL/ASSIST、X-FORBID-006）、A-AG-M-S-01_对话代理状态机设计.md（SM-AG-01/02/03）、A-DE-I-P-01_对话编辑接口协议.md（generate_diff 承接产出）、A-ED-I-P-01_编辑器接口协议.md（apply_edit/装饰层）、A-REF-M-T-01（注入载荷）、ADU.md
直接承接：实现期 AG command 注册；门禁/网关经 ENT 客户端通道（A-SYS-I-P-03 契约）；产出交 DE（de.generate_diff）；辅助采纳经 ED（ed.apply_edit）；记忆/知识/引用注入经 AST/KB/REF 供给
使用边界：本文声明 AG 接口语义级签名；门禁/网关协议字段归 A-SYS-I-P-03（不重定义）；落盘经 DE→WS（不重复）；呈现经 ED（不重复）；不新增 RULE/CHAIN/CONSTRAINT/TERM；工具清单/提示词/模型档延实现期运营配置
变更要求：门禁前置语义变更涉 APC §13 须先授权；产出接口变更复核 DE；辅助采纳接口变更复核 ED；工具回注接口变更复核 SM-AG-02
治理成熟度：draft
---

# Binder 对话代理接口协议（AG · L2 接口层）

## §1 文档定位

```
A-SYS-I-P-02（骨架）+ A-SYS-I-P-03（门禁/网关契约）→ 本文（AG 回合/工具/辅助接口）→ 实现 + @GOV(追溯 A-AG-M-T-01)
门禁前置：每个发起 command 内部必经 ENT 门禁通道（SM-SYS-01 GatePassed）方达网关——X-SEQ-001 在接口层的硬约束
产出：HandingOff → de.generate_diff；辅助采纳 → ed.apply_edit
```

**接口层门禁不变量（X-SEQ-001 / BR-AG-VERIFY-001）**：下列任一发起 command 的实现路径中，
模型调用前必经门禁；无有效订阅/断网时——chat/工具回合 command 返回 GateDenied 类错误，
辅助类 command 静默返回空候选（不报错、不打断编辑）。接口签名不暴露任何绕过门禁的入口。

## §2 chat 回合接口（AG-TURN / SM-AG-01）

| command/事件 | 参数（语义） | 结果 | 特有错误/状态 | 归属 |
|---|---|---|---|---|
| `ag.start_turn` | DocRef、prompt、注入引用集（@mention ids，可空）、`Channel<AgTurnEvent>` 句柄 | turn_id（SM-AG-01 启动）；流式经 channel 回推（§2.1） | **gate_denied**（无订阅/断网，不可绕过）；组装失败（注入锚失效） | 经门禁→网关；BR-AG-STATE-001/VERIFY-001 |
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
    active_doc_rel_path: String,
    mentions: Vec<String>,
    channel: Channel<AgTurnEvent>,
    state: State<'_, AppState>,
) -> Result<(), IpcError>
```

**三终态映射**（BR-SYS-STATE-002）：完成 → `Done`；用户中断 → `Done { stop_reason: "cancelled" }`（无半落盘）；错误 → `Error`。

**前端消费映射**（PI-AG-10F，`agStore`/`deStore`）：`Chunk`→`appendToMessage`、`ToolRequest`/`ToolResult`→工具卡片、`DiffProposed`→`deStore.stageDiff`（四态卡片）、`Done`→`finishMessage`、`Error`→`pushError`。

**关联命令形状**（同属契约，前后端对齐）：`ag.cancel_turn(turn_id)`（前端关闭 channel）；产出落 `de.generate_diff`→`DiffProposed`；裁决经 `de.accept_diff`/`de.reject_diff`（归 A-DE-I-P-01，本文不重定义）。

**冻结约定**：本 schema 一经冻结，字段增删须前后端**双侧同步**并回写本节；桩阶段（GW 未就绪）后端经 `route_start_turn` 桩从 channel 推 `Chunk×N + Done`，前端零改动切换真实。

## §3 工具调用回注接口（AG-TOOL / SM-AG-02）

| 接口 | 参数（语义） | 结果 | 特有错误/状态 | 归属 |
|---|---|---|---|---|
| `ag.tool_result`（回注） | turn_id、tool_call_id、执行结果/失败 | 续行回合 | 工具未注册/越权→拒绝并回注拒绝原因（不执行） | SM-AG-02 ToolValidating；BR-AG-STATE-002 |

> 工具执行在回合内编排：只读类工具（读/搜）经 WS（`ws.read_doc`/`ws.search`，限界 X-FORBID-004）；
> **写意图工具不直写**——产出修改建议走 `de.generate_diff`（X-FORBID-006）；循环上限/中断由
> SM-AG-02 管理，达上限 LoopAborted。工具清单与权限分级延实现期运营配置（接口不锁清单）。

## §4 内联辅助/自动补全接口（AG-ASSIST / SM-AG-03）

| command | 参数（语义） | 结果 | 特有错误/状态 | 归属 |
|---|---|---|---|---|
| `ag.request_assist` | assist_kind（autocomplete/inline_assist）、编辑上下文（DocRef、位置/选区）、blockId | 候选（或空） | 节流丢弃→空；无订阅/断网/失败→**静默空候选**（不报错） | SM-AG-03；BR-AG-STATE-003（每次过门禁，无例外） |

> 候选呈现经 `ed.mount_decoration`（装饰层）；**采纳经 `ed.apply_edit`**（作为用户编辑操作进
> 编辑模型，驱动 SM-ED-01 edit→脏态，非 AI 直写）；忽略经 `ed.dismiss_decoration` 复位。
> 节流参数（请求频率）延实现期（AG-03），接口侧由前端按 SM-AG-03 节流后再调 request_assist。

## §5 上下文组装对接（AG-TURN / BR-AG-DATA-001，仅索引）

回合上下文来源（组装在 ag.start_turn 内部完成，限界 X-FORBID-004，显式注入优先保留）：

| 来源 | 供给 | 文档 |
|---|---|---|
| 当前文档 | `ws.read_doc` | A-WS-I-P-01 §3 |
| @mention 引用 | REF 注入载荷（RefInjection） | A-REF-M-T-01 §4 |
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
- Chat↔Build 接管接口（takeover/fallback）随 CBT I-P 联合声明（本文不预定义，AG-04）。
- REF/AST/KB 供给接口待其 I-P；本文 §5 仅占消费入口。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（实现前置件，AI 调用面）：AG 接口协议——chat 回合 4（start/cancel/stream/list，门禁不变量内建）、工具回注 1（tool_result 校验回注）、内联辅助 1（request_assist 静默失败、采纳经 ed.apply_edit）、上下文组装对接索引（WS/REF/AST/KB）；门禁前置 X-SEQ-001 固化为接口不变量、产出唯一经 de.generate_diff；零新增注册项；债务（工具清单/节流/接管接口/供给接口延后） |
| 2026-06-24 | v1.1 | 新增 §2.1 流式回传契约（AgTurnEvent · PI-AG-10C 冻结点）：AgTurnEvent 六分支（Chunk/ToolRequest/ToolResult/DiffProposed/Done/Error）、ag_start_turn 命令签名（Channel<AgTurnEvent> 传输，承 SY-02 选型）、三终态映射（BR-SYS-STATE-002）、前端消费映射（agStore/deStore）、关联命令形状与冻结约定；§2 表格 start_turn/turn_stream 行改为 Channel<AgTurnEvent> 模型并指向 §2.1。前后端双轨同步真源，零新增注册类治理块 |
| 2026-06-26 | v1.2 | 命名落地修正：AG 回合流式事件定名 **AgTurnEvent**（避开 src-tauri/src/ipc/mod.rs 既有 SYS `StreamEvent` 重名），§2.1 补「与 SYS StreamEvent 分层不同·非重复」说明（通用 dispatch 传输契约 vs AG 领域 per-call Channel）；PI-AG-10C 落地同步 |
