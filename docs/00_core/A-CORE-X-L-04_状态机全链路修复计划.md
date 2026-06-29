---
文档编号：CORE-X-L-04
文档状态：A
负责模块：CORE（跨模块：SYS / DE / AG / WS / ED / CBT / AST / TPL / DOC）
文档职责：Binder 状态机全链路修复计划（L3）——基于全量审计结论，分三阶段闭合「Rust 侧漂移 → IPC 广播层缺失 → TypeScript Store 非投影」三类缺口；承接 APC §6 状态机驱动强制阈值与 X-FORBID-007
上游约束：A-CORE-C-P-01_APC.md（§2 状态机驱动、§6 强制阈值、§12 audit 硬阻断）、A-SYS-M-S-01_状态机总图.md（12 台状态机登记清单、X-SEQ-003、X-INTER-001）、A-SYS-C-T-02_前端交互架构与流程总图.md（X-FORBID-007 前端投影规则）、各模块 M-S（SM-* 设计）、A-CORE-X-L-02_总开发实施计划.md（P2/P3 阶段对接）
直接承接：本计划条目驱动跨模块 Rust 与 TypeScript 侧代码变更；@GOV 追溯对应 BR-*/SM-* 规则；不追溯本文编号
使用边界：L3 承接型计划，引用规则不定义规则；本文编号不得作为代码 @GOV 追溯来源（APC §3.2）；设计漂移修复以各模块 M-S 为准（不反向改设计）；IPC 事件名与 Payload 结构以本文为基准，如实现期发现语义偏差先修本文再改代码
变更要求：新增/删除 Task 须复核 X-FORBID-007 覆盖完整性；IPC 事件命名变更须同步各 Task 引用处；不得为代码现状反向修改 ENT/SYS 设计文档（见 §4.2）
治理成熟度：draft
---

# 状态机全链路修复计划（跨模块 · L3）

## §1 文档定位

本计划承接全量状态机审计结论（2026-06-27 会话），目标是闭合三类缺口，使项目真正进入
**APC §6 声明的状态机驱动开发**状态。

```
审计发现（2026-06-27）→ 本计划（L3 承接型）→ Phase 1/2/3 实施 → @GOV 追溯各模块 BR-*/SM-*
```

本文**不定义规则、不修改设计**，仅编排执行路径。设计冲突时以各模块 M-S 文档为准。

---

## §2 审计结论摘要

### §2.1 全量核对表（12 台状态机）

「States/Events」列为设计文档 vs Rust 枚举的逐项对比；「运行时接线」列为 `transition()`
是否在测试外被命令处理链调用（2026-06-27 复核新增，详见 §2.2 缺口 3）。

| 机器 | Rust 实现文件 | States | Events | 运行时接线 | 结论 |
|------|-------------|--------|--------|-----------|------|
| SM-DE-01 | `src-tauri/src/de/mod.rs` | ✅ 对齐 | ✅ 对齐 | ❌ 仅测试 | 装饰：handler 直写 status |
| SM-AG-01 | `src-tauri/src/ag/turn.rs` | ✅ 对齐 | ✅ 对齐 | ✅ 已接线 | lib.rs 驱动 transition |
| SM-AG-02 | `src-tauri/src/ag/tool.rs` | ✅ 对齐 | ✅ 对齐 | ❌ 仅测试 | 装饰 |
| SM-AG-03 | `src-tauri/src/ag/assist.rs` | ✅ 对齐 | ✅ 对齐 | ❌ 仅测试 | 装饰 |
| SM-WS-01 | `src-tauri/src/workspace/save.rs` | ✅ 对齐 | ✅ 对齐 | ❌ 仅测试 | 装饰 |
| SM-WS-02 | `src-tauri/src/workspace/diff_apply.rs` | ✅ 对齐 | ✅ 对齐 | ❌ 仅测试 | 装饰：apply_diff 直写 |
| SM-WS-03 | `src-tauri/src/workspace/timeline.rs` | ✅ 对齐 | ✅ 对齐 | ❌ 仅测试 | 装饰 |
| SM-CBT-01 | `src-tauri/src/cbt/build.rs` | ✅ 对齐 | ✅ 已补 `FallbackOut`（Task 1-A） | ❌ 仅测试 | 漂移已修；接线待 Phase 2 |
| SM-CBT-handoff | `src-tauri/src/cbt/takeover.rs` | ✅ 对齐 | ✅ 对齐 | ❌ 仅测试 | 装饰 |
| SM-AST-01 | `src-tauri/src/ast/capture.rs` | ✅ 对齐 | ✅ 对齐 | ❌ 仅测试 | 装饰 |
| SM-TPL-01 | `src-tauri/src/tpl/workflow.rs` | ✅ 对齐 | ✅ 对齐 | ❌ 仅测试 | 装饰 |
| SM-SYS-01 | `src-tauri/src/session/mod.rs` + `ent/gateway.rs` | ✅ 对齐 | ✅ 对齐 | ❌ 仅测试 | 双层为设计授权的 ENT 承接（§4.2，非问题）；接线待 Phase 2 |

### §2.2 系统性缺口

除上表外，存在三类系统性缺口（缺口 3 为 2026-06-27 实现前复核新发现，是本计划的核心问题）：

1. **IPC 广播层完全不存在**：状态机转移后无任何 `app.emit()`；前端 store 无任何 `listen()`，
   前后端状态传递断路。
2. **TypeScript Store 非投影**：`deStore / agStore / edStore` 等内部维护业务状态并主动推进
   （`markDirty / resolveToolCall / setDiffState` 等），违反 X-FORBID-007。
3. **🔴 Rust 状态机未驱动运行时（核心问题）**：12 台机中**仅 SM-AG-01** 在运行时被命令链
   调用（`lib.rs:680-684 / 858-861` 驱动 `ag::turn::transition`）；**其余 11 台的 `transition()`
   只在各自单元测试中被调用**，真实命令处理器绕过状态机直接写 DB / 做 I/O。
   实证：`de.accept_diff`（`route_accept_diff`, lib.rs:622）→ `apply_diff`（diff_apply.rs:122）
   直接 `UPDATE pending_diffs SET status='accepted'`，从不调用 `de::transition` 或
   `diff_apply::transition`。因机器有 `@GOV` + 通过测试，`governance:audit` 仍绿——但状态机是
   **受治理的装饰性纯函数**，并未驱动运行时行为（即原审计「装饰/空壳」类别下沉到 Rust 层）。

   **影响**：缺口 1 的「在 transition() 调用点后 emit」对 11 台机**无附着点**——必须先把状态机
   集成进命令处理器（handler 经 transition 计算/守护下一态→持久化→emit），这是真实业务逻辑
   改造，非单纯加广播行。Phase 2 据此重定义（见 §3、§5）。

---

## §3 三阶段修复总览

```
Phase 1  修复 Rust 侧漂移                     纯 Rust，已完成（Task 1-A 落地，Task 1-B 撤销）
Phase 2  状态机集成进运行时 + IPC 广播         Rust + TypeScript，按模块顺序（核心、最大）
Phase 3  TypeScript Store 降级为投影           纯 TypeScript，与 Phase 2 逐模块配套
```

**Phase 2 重定义（2026-06-27，据 §2.2 缺口 3）**：原假设「在 transition() 调用点后加 emit」
不成立——11/12 台机无运行时调用点。Phase 2 升级为**三步集成**，每台机：

```
① 集成：命令处理器的业务决策改为路由经 transition()——由状态机计算下一态并执行 guard，
        而非直接写 DB / 做 I/O（SM-AG-01 已接线，仅需补 ②③）
② 持久化：将 transition() 给出的下一态作为权威态落库（替代当前直写 status 字符串）
③ 广播：转移成功后 app.emit("sm:{id}:state_changed", payload)
```

**Phase 2 与 Phase 3 逐模块配套推进**：每台机完成 ①②③ 后立即做对应 store 降级（Phase 3），
形成一个可独立测试、可独立提交的端到端单元（emit 接线 + store 投影同批，不留中间态）。

**风险升级**：①② 改动真实业务逻辑（accept/save/build/记忆/转换…的落库路径），有行为回归风险。
每台机集成后必须：保持原有命令的对外行为不变（除非状态机守护本就应拒绝的非法转移）、
跑该模块既有测试 + 新增集成测试、人工验证关键链路。比例上 Phase 2 是本计划主体工作量。

Phase 1 决策已于 2026-06-27 人类确认并落地（见 §4）。

---

## §4 Phase 1：修复 Rust 侧漂移

### §4.1 Task 1-A：SM-CBT-01 补充 FallbackOut 事件

**文件**：`src-tauri/src/cbt/build.rs`

**问题**：设计文档 A-CBT-M-S-01 §2 events 列出了 `fallbackOut`，用于 `PlanFail→Idle`
和 `Interrupt→Interrupted` 路径发生后，向 SM-AG-01 发出回落 chat 的协调信号（X-INTER-001）。
当前 `BuildEvent` 枚举中缺少该事件，回落路径静默无信号，AG 侧无法感知。

**决策（2026-06-27 人类确认）**：`FallbackOut` 是 CBT 向 AG 发出的**输出动作信号**，
不是 CBT 自身可接收的输入事件。补入 `BuildEvent` 枚举，`transition()` 表无需新增行；
编排层在 `PlanFail→Idle` 和 `Interrupt→Interrupted` 路径发生后主动发出该信号，
驱动 AG 侧 SM-AG-01 的 `FallbackIn` 事件（X-INTER-001 双侧对称）。

**改动范围**：

```rust
// src-tauri/src/cbt/build.rs — BuildEvent 枚举末尾新增一项
pub enum BuildEvent {
    // ... 现有 12 项不变 ...
    FallbackOut,  // 回落 chat 协调信号（X-INTER-001；PlanFail/Interrupted 后由编排层发出）
}
```

**同步新增测试**：验证 `PlanFail` 路径触发 `FallbackOut` 信号意图，与 A-CBT-M-S-01 §3
`Chat↔Build 接管` X-INTER-001 回执契约对齐。

---

### §4.2 Task 1-B（已撤销）：SM-SYS-01「双实现」实为 ENT 运行时承接，无需修复

**文件**：`src-tauri/src/session/mod.rs`（SM-SYS-01 权威实现）+
`src-tauri/src/ent/gateway.rs`（`InvokeState` — SM-SYS-01 的 ENT 运行时承接段）

**审计初判（已被推翻）**：审计曾把 `session/mod.rs` 的 `SessionState` 与 `ent/gateway.rs`
的 `InvokeState` 判为 SM-SYS-01 的「双实现 / 结构问题」，并一度计划登记 SM-ENT-01（选 B）。

**实读设计后的更正（2026-06-27）**：进入代码修改前完整读取 ENT 模块设计 `A-ENT-M-T-01`，
发现该判断是**假阳性**。权威设计明确规定 ENT 不建新机：

- `A-ENT-M-T-01 §5`：**「STATE_MACHINE：SM-SYS-01 门禁+网关段已覆盖（owner=SYS），ENT 承接运行时不建新机。」**
- `A-ENT-M-T-01 §1`：`本文 ──运行时承接──→ SM-SYS-01 门禁+网关段（GateChecking→…→Invoking→终态）`
- `BR-ENT-STATE-001`（已注册规则）：`仅在 GatePassed 后发起（SM-SYS-01 forbidden 对齐）`
- `acct/session.rs` 边界 `delegatedToSysNoRedefine` 同向佐证：ENT/ACCT 委托 SYS，不另立机

即 `InvokeState` 是 SM-SYS-01「门禁+网关段」的**已登记、受治理的运行时承接实现**
（受 `BR-ENT-STATE-001` 治理、有 @GOV、有测试），`governance:audit` 不将其标记为未注册机。
`SessionState`（完整会话生命周期，owner SYS）与 `InvokeState`（ENT 承接的门禁+网关段）
是同一逻辑状态机的两层，由设计明确批准，**不是未授权的重复实现**。

**决策（2026-06-27 人类确认）：撤销 Task 1-B。** 不登记 SM-ENT-01、不修改任何 ENT/SYS 设计文档。
登记 SM-ENT-01 将构成对 `A-ENT-M-T-01 §5`「不建新机」这一明确设计决策的反向修改
（违反 APC §13 / CLAUDE.md §9），不予执行。

**IPC 广播影响**：`InvokeState` 作为 SM-SYS-01 的 ENT 段，统一在 **`sm:SM-SYS-01:state_changed`**
通道广播（携 `segment: "ent_invoke"` 区分）；原计划的 Task 2-A2 **并回 Task 2-A1**，不设独立
`sm:SM-ENT-01` 事件（见 §5.4 Task 2-A）。

---

## §5 Phase 2：建立 IPC 广播层

### §5.1 IPC 事件命名规范

> **设计 SSOT（2026-06-27 收口）**：本广播契约的权威源为 **A-SYS-I-P-02 §5.1 / 规则 BR-SYS-STATE-003**
> （承接 X-FORBID-007）。本计划为 L3 实施编排，代码 @GOV 追溯 BR-SYS-STATE-003（不追溯本文编号）。

所有状态机广播事件统一格式：

```
sm:{machine_id}:state_changed
```

12 台机事件名列表（SM-SYS-01 含 ENT 承接段，不设独立 SM-ENT-01，见 §4.2）：

```
sm:SM-SYS-01:state_changed   # session/mod.rs（会话+门禁）+ ent/gateway.rs（ENT 承接段，segment 区分）
sm:SM-DE-01:state_changed    # de/mod.rs — diff 终态
sm:SM-WS-01:state_changed    # workspace/save.rs — 保存
sm:SM-WS-02:state_changed    # workspace/diff_apply.rs — diff 落盘
sm:SM-WS-03:state_changed    # workspace/timeline.rs — 时间轴还原
sm:SM-AG-01:state_changed    # ag/turn.rs — chat-agent 回合
sm:SM-AG-02:state_changed    # ag/tool.rs — tool-call 循环
sm:SM-AG-03:state_changed    # ag/assist.rs — autocomplete/inline-assist
sm:SM-ED-01:state_changed    # ED 编辑会话
sm:SM-DOC-01:state_changed   # doc/convert.rs — docx 开/存
sm:SM-CBT-01:state_changed   # cbt/build.rs — CBT 构建回合
sm:SM-AST-01:state_changed   # ast/capture.rs — 记忆写入
sm:SM-TPL-01:state_changed   # tpl/workflow.rs — 工作流执行
```

### §5.2 Payload 统一结构

**Rust 侧**（每台机各建一个 `Payload` struct，放在该模块的 `events.rs` 或 `mod.rs` 中）：

```rust
// 示例：SM-DE-01，位于 src-tauri/src/de/mod.rs 或新建 src-tauri/src/de/events.rs
#[derive(serde::Serialize, Clone)]
pub struct SmDe01Payload {
    pub machine_id: &'static str,  // 恒为 "SM-DE-01"
    pub instance_id: String,       // 区分同类型多实例（diff_id / turn_id / run_id 等）
    pub state: String,             // DiffState 的 Debug 字符串（"Pending" / "Accepted" 等）
    pub terminal: bool,            // is_terminal(state) 结果
    // context 字段按机器需要扩展（见各 Task 说明）
}
```

**TypeScript 侧**（统一放在 `src/types/smEvents.ts`，新建文件）：

```typescript
// 以 SM-DE-01 为例
export interface SmDe01Payload {
  machine_id: "SM-DE-01";
  instance_id: string;   // diff_id
  state: "Generating" | "Pending" | "Accepted" | "Rejected" | "Expired" | "GenFailed";
  terminal: boolean;
}

// 其余 12 台机各对应一个 interface，联合类型导出：
export type SmPayload = SmSys01Payload | SmDe01Payload | SmWs01Payload | /* ... */;
```

### §5.3 集成与 AppHandle 传递策略

每台机的 ①集成 + ③广播 共用以下接线（②持久化见各 Task）：

1. IPC command handler（`src-tauri/src/lib.rs`）已有 `app: tauri::AppHandle`（dispatch 入口）
2. **集成**：handler 当前直接做的业务决策（如 `apply_diff` 内直写 `status='accepted'`）改为：
   先读当前态 → `let next = de::transition(cur, DiffEvent::UserAccept).ok_or(sm_error)?` →
   以 `next` 落库为权威态 → 执行该态对应副作用。非法转移（`None`）回统一错误，不再静默直写。
3. **AppHandle 传递**：受影响的业务函数签名增加 `app: &tauri::AppHandle`，从 handler 传入；
   或在 handler 层完成 transition+emit、业务函数只收最终态（优先后者，减少签名扩散）。
4. **广播**：转移成功后 `app.emit("sm:{id}:state_changed", payload).ok()`——失败吞掉
   （不得因广播失败中断业务流程）。

> 各 Task 下「emit 触发点」即②③要落地的转移点；除 SM-AG-01 外，这些点当前**尚不存在于运行时**，
> 需先做①集成才有附着点（§2.2 缺口 3）。

---

### §5.4 各模块集成 + emit 任务（Task 2-A 至 2-M）

按依赖链顺序排列。每个 Task 含 ①集成 ②持久化 ③广播 三步（§3）；SM-AG-01 已接线，仅 ②③。

#### Task 2-A：SM-SYS-01 emit 接线（含 ENT 承接段）

SM-SYS-01 由两个文件分层承接（见 §4.2），均广播 `sm:SM-SYS-01:state_changed`，
用 `segment` 字段区分会话段与 ENT 网关段。

**Rust 文件 1**：`src-tauri/src/session/mod.rs`（`segment: "session"`）

emit 触发点：
- `Login` → `Some(Authenticated)` 后
- `GateAllow` → `Some(GatePassed)` 后
- `GateDeny` → `Some(GateDenied)` 后
- `InvokeStreamEnd` → `Some(InvokeCompleted)` 后
- `InvokeFail` → `Some(InvokeError)` 后
- `SessionExpire` → `Some(Unauthenticated)` 后

**Rust 文件 2**：`src-tauri/src/ent/gateway.rs`（`segment: "ent_invoke"`，`InvokeState` 转移）

emit 触发点（`InvokeState` 全部转移点）：
- `Submit` → `GateSubmitting` 后
- `GatePassed` → `Streaming` 后
- `GateDenied` / `Offline` → `GateDenied` 后
- `StreamDone` → `Completed` 后
- `Cancel` → `Interrupted` 后
- `StreamError` → `Errored` 后

**Payload 字段**：`session_id`（会话段，不含令牌值）/ `turn_id`（ENT 段，与 SM-AG-01 共享）、
`segment: "session" | "ent_invoke"`

**emit 示例（会话段）**：

```rust
use tauri::Manager;

if let Some(new_state) = session::transition(cur, event) {
    // 持久化/副作用 ...
    app.emit("sm:SM-SYS-01:state_changed", SmSys01Payload {
        machine_id: "SM-SYS-01",
        segment: "session",
        instance_id: session_id.clone(),
        state: format!("{:?}", new_state),
        terminal: matches!(new_state,
            SessionState::GateDenied | SessionState::InvokeCompleted | SessionState::InvokeError
        ),
    }).ok();
}
```

**前端价值**：ENT 段使前端可感知"流式被中断"（`Interrupted`）与"正常完成"（`Completed`）、
"门禁被拒"（`GateDenied`）的细粒度状态，用于加载态 UI 与错误展示。

---

#### Task 2-B：SM-DE-01 emit 接线

**Rust 文件**：`src-tauri/src/de/lifecycle.rs`（accept/reject/expire 编排函数）

**emit 触发点**：
- `UserAccept` → `Some(Accepted)` 后
- `UserReject` → `Some(Rejected)` 后
- `Timeout` / `ExternalChange` → `Some(Expired)` 后
- `AppliedOk` / `ApplyFail` 后（Accepted 态的落盘回执）

**Payload 附加字段**：`diff_id: String`

---

#### Task 2-C：SM-WS-01 emit 接线

**Rust 文件**：`src-tauri/src/workspace/save.rs`（trigger_save 编排函数）

**emit 触发点**：
- `Edit` → `Dirty` 后
- `Save` → `Saving` 后
- `SaveOk` → `Saved` 后
- `SaveFail` / `SaveConflict` → `SaveError` 后
- `Retry` / `OverrideConfirmed` 后

**Payload 附加字段**：`rel_path: String`（区分多文档会话）

---

#### Task 2-D：SM-WS-02 emit 接线

**Rust 文件**：`src-tauri/src/workspace/diff_apply.rs`（apply/discard 编排函数）

**emit 触发点**：
- `Accepted` → `Applying` 后
- `Rejected` / `Expired` → `Discarded` 后
- `ApplyOk` → `Applied` 后
- `ApplyFail` → `ApplyError` 后

**Payload 附加字段**：`diff_id: String`、`rel_path: String`

---

#### Task 2-E：SM-AG-01 emit 接线

**Rust 文件**：`src-tauri/src/ag/turn.rs`（`persist_turn` 调用处 + 流式事件处）

**emit 触发点**：全部 `transition()` 返回 `Some` 的调用点

**Payload 附加字段**：`turn_id: String`

**流式数据通道说明**：AG 流式数据块（文字分片）走 Tauri `Channel<AgTurnEvent>`（`chunk` 事件），
与 `state_changed` 完全分离。两个通道并存，职责不同：
- `Channel<AgTurnEvent>` — 流式分片数据注入（chunk / tool_request / diff_proposed / done / error）
- `sm:SM-AG-01:state_changed` — 状态机转移广播（Assembling / AwaitingGate / Streaming / HandingOff 等）

`done` Channel 事件（流式自然结束）与 SM-AG-01 `StreamEnd→HandingOff` 广播在时序上接近但语义不同：
前者是数据流终止，后者是回合状态转移。前端对两者均需监听，不得互相替代。

---

#### Task 2-F：SM-AG-02 emit 接线

**Rust 文件**：`src-tauri/src/ag/tool.rs`（ToolLoop 编排处）

**Payload 附加字段**：`turn_id: String`、`tool_name: String`

---

#### Task 2-G：SM-AG-03 emit 接线

**Rust 文件**：`src-tauri/src/ag/assist.rs`（inline assist 编排处）

**Payload 附加字段**：`assist_id: String`（每次触发生成）

---

#### Task 2-H：SM-ED-01 emit 接线

**Rust 文件**：ED 侧编辑编排层（待确认具体文件；SM-ED-01 定义在 A-ED-M-S-01）

**Payload 附加字段**：`rel_path: String`

---

#### Task 2-I：SM-WS-03 emit 接线

**Rust 文件**：`src-tauri/src/workspace/timeline.rs`（restore 编排处）

**Payload 附加字段**：`rel_path: String`、`entry_id: String`

---

#### Task 2-J：SM-DOC-01 emit 接线

**Rust 文件**：`src-tauri/src/doc/convert.rs`（编排处）

**Payload 附加字段**：`rel_path: String`

---

#### Task 2-K：SM-CBT-01 emit 接线

**Rust 文件**：`src-tauri/src/cbt/build.rs`（build 编排处）

**Payload 附加字段**：`build_id: String`

---

#### Task 2-L：SM-AST-01 emit 接线

**Rust 文件**：`src-tauri/src/ast/capture.rs`（write 编排处）

**Payload 附加字段**：`memory_id: String`

---

#### Task 2-M：SM-TPL-01 emit 接线

**Rust 文件**：`src-tauri/src/tpl/workflow.rs`（step 编排处）

**Payload 附加字段**：`run_id: String`、`step_index: u32`

---

### §5.5 TypeScript 监听基础层（Task 2-N）

**新建文件**：`src/types/smEvents.ts`（全量 Payload 类型定义）、
`src/services/smListeners.ts`（统一监听器启停）

**`smListeners.ts` 结构示例**：

```typescript
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { deStore } from "../store/deStore";
import { agStore } from "../store/agStore";
// ... 各 store import

const unlisteners: UnlistenFn[] = [];

export async function startAllSmListeners() {
  unlisteners.push(
    await listen<SmDe01Payload>("sm:SM-DE-01:state_changed", ({ payload }) => {
      deStore.applyBackendState(payload.instance_id, payload.state, payload.terminal);
    }),
    await listen<SmWs01Payload>("sm:SM-WS-01:state_changed", ({ payload }) => {
      edStore.applyWsSaveState(payload.instance_id, payload.state);
    }),
    await listen<SmAg01Payload>("sm:SM-AG-01:state_changed", ({ payload }) => {
      agStore.applyTurnState(payload.instance_id, payload.state, payload.terminal);
    }),
    // ... 其余 10 台机
  );
}

export function stopAllSmListeners() {
  unlisteners.forEach((u) => u());
  unlisteners.length = 0;
}
```

**启动点**：`src/shell/Workbench.tsx`（workspace 打开时）、
**停止点**：workspace 关闭时 / `App.tsx` unmount 时。

---

## §6 Phase 3：TypeScript Store 降级为投影

### §6.1 改造原则

每个 store 的改造模式：

```
删除：主动推断/推进业务状态的方法（markDirty / resolveToolCall / setDiffState 等）
新增：applyBackendState() 接收 Rust 广播，内含 Rust state → 前端视觉态的映射逻辑
保留：纯 UI 操作（openDoc / setActivePath / updateContent 等，不涉及业务终态）
```

---

### §6.2 Task 3-A：`deStore.ts` 降级

**文件**：`src/store/deStore.ts`

| 方法 | 动作 |
|------|------|
| `setDiffState(id, state, patch)` | **删除** |
| `addDiff(view)` | 改为由 SM-DE-01 `Generating` 事件驱动（Phase 2 接入后调整） |
| `setBatch(...)` | 保留（批量 UI 编排，非 SM-DE-01 状态） |
| `reset()` | 保留 |
| 新增 | `applyBackendState(diffId, state, terminal)` |

**`applyBackendState` 实现要点**：

```typescript
// Rust DiffState → 前端 DiffCardState（8 视觉态）映射
function mapRustDiffState(s: string): DiffCardState {
  switch (s) {
    case "Generating": return "pending";   // 生成中暂显 pending
    case "Pending":    return "pending";
    case "Accepted":   return "accepted";
    case "Rejected":   return "rejected";
    case "Expired":    return "expired";
    case "GenFailed":  return "error";
    default:           return "pending";
  }
}
// 注：validating/applying/conflict 由 SM-WS-02 state_changed 补充（落盘往返过渡态）
```

---

### §6.3 Task 3-B：`agStore.ts` 降级

**文件**：`src/store/agStore.ts`

| 方法 | 动作 |
|------|------|
| `setSending(bool)` | **删除**；改由 SM-AG-01 `Assembling` 广播驱动 |
| `finishMessage(id)` | **保留**；由 `Channel<AgTurnEvent>` 的 `done` 事件直接驱动（流式终止时机准确，绕道 SM 广播会增加不必要延迟） |
| `addToolCall(name)` | **删除**；改由 SM-AG-02 `ToolValidating` 广播驱动 |
| `resolveToolCall(name, ok)` | **删除**；改由 SM-AG-02 `LoopCompleted/LoopAborted` 广播驱动 |
| `clearToolCalls()` | **删除**；由 SM-AG-01 `Idle` 广播（新回合）驱动 |
| `appendToMessage(id, chunk)` | **保留**；由 `Channel<AgTurnEvent>` 的 `chunk` 事件直接驱动（纯数据注入，不属状态机转移） |
| `pushMessage(msg)` | 保留（消息入队由上层编排控制） |
| 新增 | `applyTurnState(turnId, state, terminal)` — 接收 SM-AG-01 广播 |
| 新增 | `applyToolState(turnId, toolName, state)` — 接收 SM-AG-02 广播 |

**`finishMessage` 与 `appendToMessage` 保留的理由**（2026-06-27 确认）：AG 流式通道
（`Channel<AgTurnEvent>`）已是独立的 Tauri Channel，与 `state_changed` 完全分离。
这两个方法响应 Channel 数据事件，不是业务状态的自主推进，符合投影语义，无需改造。

---

### §6.4 Task 3-C：`edStore.ts` 降级

**文件**：`src/store/edStore.ts`

| 方法 | 动作 |
|------|------|
| `markDirty()` | **删除**；由 SM-WS-01 `Dirty` 广播驱动 |
| `markSaved(path, baseline)` | **删除**；由 SM-WS-01 `Saved` 广播驱动 |
| `setTabState(path, tabState)` | **删除**（同上驱动来源） |
| `openDoc(path)` | 保留（UI 操作） |
| `loadDoc(...)` | 保留（UI 操作） |
| `setActivePath(path)` | 保留（UI 操作） |
| `updateContent(path, content)` | 保留（暂存草稿，UI 操作） |
| `closeSession(path)` | 保留（UI 操作） |
| `reset()` | 保留 |
| 新增 | `applyWsSaveState(relPath, state)` 接收 SM-WS-01 广播 |
| 新增 | `applyEdSessionState(relPath, state)` 接收 SM-ED-01 广播 |

**`applyWsSaveState` 映射**：

```typescript
// Rust SaveState → TabState 映射
function mapSaveState(s: string): TabState {
  switch (s) {
    case "Dirty":     return "dirty";
    case "Saving":    return "saving";
    case "Saved":     return "saved";
    case "SaveError": return "dirty";  // 保存失败回显脏态（用户可重试）
    default:          return "normal";
  }
}
```

---

### §6.5 Task 3-D：`wsStore.ts`（保持投影结构，扩展 save 状态接收）

**文件**：`src/store/wsStore.ts`

当前已是投影结构（`_projectLoading/Docs/Error`），本阶段：

- 现有方法不变
- 新增 `applySaveState(relPath, state)` — 对应文件树节点的保存状态显示（如标签闪烁）

---

### §6.6 新建 Store：`acctStore.ts`（承接 SM-SYS-01）

**新建文件**：`src/store/acctStore.ts`

承接 SM-SYS-01 广播，投影门禁/会话状态到前端。核心字段：

```typescript
interface AcctState {
  sessionState: SmSys01State;  // Unauthenticated / Authenticated / GatePassed / ...
  terminal: boolean;
  sessionId: string | null;
}
```

**注意**：前端**不得缓存或展示令牌值**（X-FORBID-001；密钥服务端持有）。

---

## §7 执行顺序总图

```
Phase 1
  Task 1-A  SM-CBT-01 FallbackOut 补充（输出信号，不进 transition 表）
  Task 1-B  已撤销（InvokeState 是 SM-SYS-01 ENT 承接段，无需修复，见 §4.2）

Phase 2+3（逐模块配套，顺序如下）
  Task 2-A  + 新建 acctStore.ts    SM-SYS-01（含 ENT 承接段，segment 区分）
  Task 2-B  + Task 3-A             SM-DE-01 + deStore 降级
  Task 2-C  + Task 3-C（部分）     SM-WS-01 + edStore.markDirty 删除
  Task 2-D  + Task 3-A（补充）     SM-WS-02 + deStore 落盘过渡态
  Task 2-E  + Task 3-B（部分）     SM-AG-01 + agStore.setSending/clearToolCalls 删除
                                   （finishMessage/appendToMessage 保留，由 Channel 驱动）
  Task 2-F  + Task 3-B（工具侧）   SM-AG-02 + agStore.addToolCall/resolveToolCall 删除
  Task 2-G                         SM-AG-03（assistStore 已是投影结构，微调或保留）
  Task 2-H  + Task 3-C（完整）     SM-ED-01 + edStore 全量降级
  Task 2-I                         SM-WS-03（时间轴 UI store 按需新建）
  Task 2-J                         SM-DOC-01（doc 相关 store 按需新建）
  Task 2-K                         SM-CBT-01（CBT store 按需新建）
  Task 2-L                         SM-AST-01（AST store 按需新建）
  Task 2-M                         SM-TPL-01（TPL store 按需新建）

Task 2-N（贯穿 Phase 2，随各模块接入）
  新建 smListeners.ts + smEvents.ts  IPC 基础层，每完成一台机就接入
  wsStore.ts 扩展                    Task 3-D
```

---

## §8 验收标准（阶段门禁）

### Phase 1 验收

- [x] `BuildEvent::FallbackOut` 补充入枚举（输出信号，transition 返回 None）；测试 `fallback_out_is_output_signal_never_transitions` 覆盖；`cargo test cbt::build` 4/4 通过
- [x] Task 1-B 撤销：经读 A-ENT-M-T-01 §5 确认 InvokeState 为 SM-SYS-01 ENT 承接段，不登记新机、不改设计文档
- [x] `governance:audit` 无因本次改动新增的 failed 项（既有 failed 均为预存 legacy/BOUNDARY_ABSTRACT 语义复核项）

### Phase 2 验收（每台机独立验收）

- [ ] **①集成**：该机的命令处理器业务决策已路由经 `transition()`，非法转移回错误（不再直写 DB/I-O）
- [ ] **②持久化**：`transition()` 给出的下一态作为权威态落库
- [ ] **③广播**：转移成功点均有 `app.emit("sm:{id}:state_changed", ...)`
- [ ] 该模块既有测试 + 新增集成测试通过；命令对外行为与改造前一致（除应被守护拒绝的非法转移）
- [ ] `src/types/smEvents.ts` 中对应 Payload 类型已定义；`smListeners.ts` 已注册监听
- [ ] Workbench 启动时 listener 已启动（DevTools / Tauri 日志确认事件到达）

### Phase 3 验收（每台机独立验收）

- [ ] 对应 store 中无任何主动推进业务状态的方法（`markDirty` 等已删除）
- [ ] `applyBackendState()` 是该 store 唯一修改业务状态的入口
- [ ] 相关 UI 组件（DiffCard / TabStrip / ChatPanel 等）行为与改造前一致
- [ ] `governance:audit` 通过

---

## §9 风险与注意事项

1. **🔴 状态机集成改动真实业务逻辑（最大风险）**：①集成把命令处理器的直写路径
   （如 `apply_diff` 直写 `status='accepted'`）改为经 `transition()` 计算权威态。这改变落库路径，
   有行为回归风险。缓解：每台机集成后保持命令对外行为不变（仅新增对非法转移的守护拒绝）、
   跑既有 + 新增集成测试、人工验证关键链路；逐台机独立提交，不批量。

2. **AppHandle 传递链**：受影响业务函数需增加 `AppHandle` 参数（或在 handler 层完成
   transition+emit、业务函数只收最终态——优先此式减少签名扩散）。

3. **流式数据与状态变更的分离**：AG 流式数据块经 Tauri `Channel<AgTurnEvent>` 传输（`chunk`
   / `done` / `error` 等），与 `sm:SM-AG-01:state_changed` 完全独立。`agStore` 的
   `appendToMessage` 和 `finishMessage` 由 Channel 事件直接驱动，**不经 SM 广播**，
   是投影而非自主推进（2026-06-27 确认保留）。前端实现时两个通道并存，互不替代。

4. **SM-WS-01 与 edStore 的脏态改造影响面**：`markDirty()` 当前被编辑器 `onUpdate`
   回调直接调用，删除后需将 `onUpdate` 改为向 Rust 发起 `Edit` 事件，由 Rust SM-WS-01
   转移后广播 `Dirty` 回前端。这是 Phase 3 中影响面最大的改动，需提前评估 Editor 组件
   的 IPC 调用频率（每次按键发 IPC 可能影响性能）。

5. **Phase 2+3 期间的兼容过渡**：在某台机 emit 接线完成但 store 尚未降级时，该 store 的
   旧方法与新 `applyBackendState()` 可能并存，需确保不重复更新同一状态字段。
   建议每台机「集成 + emit + store 降级」在同一次提交中完成，不留中间态。

6. **X-INTER-001 跨模块转移一致性（G5 验证）**：IPC 广播建立后，跨模块转移点
   （DE accepted → WS apply、AG HandingOff → DE Generating 等）的两侧对接一致性
   需在 G5 集成验证阶段统一联调确认。

---

## §10 决策记录（2026-06-27 人类确认）

以下三项决策已于 2026-06-27 确认，不再阻断执行：

| 项 | 决策 | 理由摘要 |
|----|------|---------|
| FallbackOut 语义 | **输出信号**（不进 transition 表） | 是 CBT 向 AG 发出的动作，不是 CBT 自身接收的输入 |
| SM-SYS-01「双实现」 | **撤销修复（假阳性）** | 读 A-ENT-M-T-01 §5 后确认 InvokeState 是 SM-SYS-01 ENT 承接段（已受 BR-ENT-STATE-001 治理），不登记新机、不改设计 |
| agStore 流式方法 | **`appendToMessage` / `finishMessage` 保留** | 由 Tauri Channel 数据事件驱动，非状态机自主推进，符合投影语义 |

---

## §12 剩余 4 台机器运行时调研结论（2026-06-27 只读调研）

对 §2.2 缺口 3 之外的剩余机器做只读调研（命令注册表 + 跨模块调用 + 前端调用）。结论：
**这 4 台与已集成的 8 台性质不同——它们的底层功能尚未接线到运行时（功能桩），而非"有运行时但绕过状态机"**。

| 机器 | 命令入口 | 运行时调用 | 前端 | 结论 |
|------|---------|-----------|------|------|
| SM-CBT-01 构建 | 无 `cbt.*` | 无任何调用 | 无 | 纯功能桩，未接线 |
| SM-TPL-01 工作流 | 无 `tpl.*` | 无任何调用 | 无 | 纯功能桩，未接线 |
| SM-AST-01 记忆写入 | 无 `ast.*` 写命令 | `ast::supply`（读/注入）在 `ag_start_turn` 已调用；**写入链 `capture::transition` / `memory_link::on_turn_terminal` 未被任何运行时调用** | 无写入入口 | 写入功能桩（读已接，且读属 A-AST-M-S-01 §4 无状态机豁免） |
| SM-AG-03 内联辅助 | 无 assist 命令 | 无 | `assistStore` + Editor ⌘K/⌘J UI 存在，但生成是**前端本地桩**（Editor.tsx：「实现期经门禁→网关产出」） | 功能桩，前端 UI 占位、后端未接线 |

**判定**：这 4 台的状态机是**正确的「实现前先建机」状态**（X-SEQ-003：状态化核心链须先建状态机再进入实现）。
它们不是装饰性绕过——而是对应功能（CBT 构建 / TPL 工作流 / AST 记忆写入 / AG-03 辅助生成）**尚未实现运行时**。

**处置（不在本状态机修复计划内造代码）**：标注为「待功能接线后集成」。当这些功能落地命令入口与业务逻辑时，
其实现**必须**路由经已定义的状态机（X-SEQ-003 强制，APC §6），并按本计划 §5 三步模板加广播 + 前端投影。
现在为它们「集成状态机」会是给不存在的运行时加装饰，违反 CLAUDE.md §2 范围纪律与「不保留半成品」。

---

## §11 不在本计划范围内的事项

- 各模块业务逻辑本身的正确性修复（属各模块开发计划）
- 状态机设计文档本身的内容变更（属各模块 M-S 变更流程）
- G5 集成验证（跨模块联调，属 A-CORE-X-L-02 P3/G5）
- TypeScript store 以外的 UI 组件层面的 state.matches() 等价改造
  （在 store 降级完成后，UI 层已可直接读 store 字段，不需要等价 XState API）
- **未接线功能的状态机集成**（SM-CBT-01 / SM-TPL-01 / SM-AST-01 写入 / SM-AG-03）：见 §12，
  待对应功能落地命令入口后，由各功能开发计划按本计划 §5 模板集成（X-SEQ-003 强制经状态机）

---

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-27 | v1.0 | 初版：基于全量审计结论（12 台状态机核对完毕，2 处漂移/结构问题 + IPC 广播层缺失 + Store 非投影），三阶段修复计划（Phase 1 漂移修复 / Phase 2 IPC 广播层 / Phase 3 Store 降级），含 emit 示例代码、store 改造清单、验收标准与风险说明 |
| 2026-06-27 | v1.1 | 落入三项人类决策：①FallbackOut 定位为输出信号（§4.1 去决策框，直接落方案）；②SM-ENT-01 独立登记（§4.2 重写为登记方案，§5.1 事件名列表补 SM-ENT-01，Task 2-A 拆为 2-A1/2-A2）；③agStore.finishMessage/appendToMessage 保留由 Channel 驱动（§6.3 Task 3-B 更新，§9 风险 2 更新）；§10 由「待确认阻断」改为「决策记录」；§8 Phase 1 验收标准更新为确定文字 |
| 2026-06-27 | v1.2 | Task 1-A 落地（cbt/build.rs 加 FallbackOut + 测试，4/4 通过）；**Task 1-B 撤销**——实现前读 A-ENT-M-T-01 §5「ENT 承接运行时不建新机」，确认 InvokeState 是 SM-SYS-01 ENT 承接段（受 BR-ENT-STATE-001 治理、audit 不标记），v1.1 选 B「登记 SM-ENT-01」属对既有设计的反向修改，经人类确认撤销；同步更新 §4.2（重写为撤销说明）、§5.1（13→12 台机，SM-SYS-01 含 ENT 段）、§5.4（Task 2-A2 并回 2-A，segment 区分）、§7/§8/§10 与文档头变更要求 |
| 2026-06-27 | v1.3 | **Phase 2 重定义**——实现前复核发现 12 台机仅 SM-AG-01 接入运行时，其余 11 台 `transition()` 只被测试调用、命令处理器绕过状态机直写 DB（§2.2 缺口 3，核心问题）。原 Phase 2「在 transition 后加 emit」对 11 台机无附着点，升级为三步集成（①集成进 handler ②持久化权威态 ③广播）。更新 §2.1（新增「运行时接线」列）、§2.2（新增缺口 3）、§3（Phase 2 重定义 + 风险升级）、§5.3/§5.4（集成策略）、§8（Phase 2 验收三步）、§9（新增最大风险项）。经人类确认「先改计划再实现」 |
| 2026-06-27 | v1.4 | **SM-DE-01 端到端纵切落地（Task 2-B + 3-A + 2-N 基础层）**：Rust 侧 `route_accept_diff/route_reject_diff` 经 `de::transition` 守护（已终态拒绝再裁决，回 `DE_ILLEGAL_TRANSITION`）+ `diff_apply::current_status`/`de::state_from_status` 读当前态 + `sm_events::emit_sm_state` 广播 `sm:SM-DE-01:state_changed`；新建 `src-tauri/src/sm_events.rs`（统一载荷 + 广播助手）。前端新建 `src/types/smEvents.ts` + `src/services/smListeners.ts`；`deStore` 删 `setDiffState`、加 `applyBackendState`（广播投影，权威态唯一入口）/`setLocalView`（过渡呈现）/`attachMeta`（元数据）；`deService` 接受/拒绝不再乐观置终态（改由广播驱动）；`Workbench` 挂载启停监听。验证：cargo 143 测试通过、vitest 65 通过、tsc 干净、governance:audit 无新增 finding（既有 failed 为预存）。为其余 10 台机的集成模板 |
| 2026-06-27 | v1.11 | **设计/规则同步缺口收口（人类指示：确认补齐）**——自审发现广播契约此前仅存于本 L3 计划、设计层 SSOT 缺位、emit 层无 @GOV 锚点。补齐四步：①A-SYS-I-P-02 §5.1 新增「状态机投影广播契约」+ 注册规则 **BR-SYS-STATE-003**（承接 X-FORBID-007）+ REQ_MAP(REQ-SYS-001) + §9 验收；②`sm_events::emit_sm_state` 加 @GOV 追溯 BR-SYS-STATE-003（EFFECT/IPC-CALL）；③A-SYS-M-S-01 §6 与本计划 §5.1 加设计 SSOT 交叉引用；④`governance:generate` 注册（@GOV 块 117、BR-SYS-STATE-003 进规则注册表 + @GOV 索引）+ `sm_events` 单测 `// covers: BR-SYS-STATE-003`。复核：cargo 148（140 lib + 4 + 4）、vitest 69、tsc 干净、audit 仅余 2 项预存基线失败（8 条 AG/ED/PRV/SYS 未覆盖规则 + @GOV 结构检查，均非本次），BR-SYS-STATE-003 已不在未覆盖列表。**实现↔设计↔规则现已同步** |
| 2026-06-27 | v1.10 | **广播原语实跑验证（人类指示：启用并自行验证）**——`sm_events::emit_sm_state` 泛型化（`R: Runtime`，运行期 Wry / 测试期 MockRuntime，路由处理器签名不变）；新增 `Cargo.toml` dev-dep `tauri features=["test"]` + `tests/sm_broadcast.rs`：用 Tauri MockRuntime **实跑** emit→listen 回路，验证事件名 `sm:{id}:state_changed`、载荷 JSON 字段（machine_id/instance_id/state/terminal、segment 省略/携带）与前端 smEvents.ts 对齐（DE-01/WS-01/AG-01/SYS-01 段 4 例）。整体验证：cargo 147（139 lib + 4 core_chain + 4 sm_broadcast）、vitest 69、tsc 干净、`npm run build` 前端生产构建通过、audit 0 新增 finding。**验证边界（诚实）**：emit→listen 广播原语已实跑；transition/守护/投影逻辑单测覆盖；**未做**的是「真实 GUI 窗口里点击→后端→广播→JS listen→React 重渲染」的全 e2e（需人工跑 `npm run tauri dev` 或 computer-use 驱动原生窗口） |
| 2026-06-27 | v1.9 | **剩余 4 台只读调研（人类指示：继续调研）**——经命令注册表 + 跨模块调用 + 前端调用核查，确认 SM-CBT-01/SM-TPL-01/SM-AST-01(写入)/SM-AG-03 均为**功能桩**（无命令入口、底层功能未接线；AST 仅读/注入已接且属无状态机豁免；AG-03 前端 UI 为本地桩）。新增 §12 调研结论：判定为「实现前先建机」正确状态（X-SEQ-003），非装饰性绕过；标注「待功能接线后集成」，现在集成将给不存在的运行时加装饰（违反范围纪律）。§11 补充对应不在范围条目。至此 12 台：8 台已集成（核心链全闭合）、4 台待功能接线 |
| 2026-06-27 | v1.8 | **三台有运行时路径的机器集成（人类决策：继续推进有路径的三台）**——SM-WS-03 时间轴还原：`route_restore_node` 经 `timeline::transition`（Previewing→ConfirmRestore→Restoring→RestoreOk/Fail）+ 广播 `sm:SM-WS-03`，`timeline` 加 `is_terminal`。SM-DOC-01 docx 开/存：`route_open_for_edit`（Idle→OpenConverting→ConvertOk/Fail）、`route_save_from_edit`（Ready→SaveConverting→SaveConvertOk/Fail，内嵌 SM-WS-01 落盘，DOC↔WS 跨模块转移）+ 广播 `sm:SM-DOC-01`。SM-AG-02 工具循环：`ag_start_turn` 循环顶/分支广播 `sm:SM-AG-02`（ModelStreaming/ToolValidating/ToolExecuting/ResultInjecting/LoopCompleted/LoopAborted）。**三台均为后端集成 + 广播**：前端已分别经 await-reload（WS-03/DOC-01）/ Channel（AG-02 工具卡）/ 本地组件 nav 投影，无 X-FORBID-007 并行态违规，故不新增前端 store/listener（广播为标准投影通道 + 观测，供未来状态 UI 消费）。验证：cargo 143、vitest 69、tsc 干净、audit 无新增 finding |
| 2026-06-27 | v1.7 | **SM-AG-01 回合机广播集成落地（Task 2-E + 3-B 部分）**：人类决策（2026-06-27）AG 在已有 Channel 投影基础上加 `sm:` 广播 + sending 由其驱动。Rust：`ag_start_turn` 加 `app` 参数，每步转移广播 `sm:SM-AG-01:state_changed`（Assembling/AwaitingGate/Streaming）+ 完成/错误终态（`emit_ag01`/`emit_ag01_complete`：Streaming→HandingOff→Completed；流式错误→TurnError）。前端：`agStore` 加 `applyTurnState`（活动态 Assembling/AwaitingGate/Streaming/HandingOff→sending=true，终态/TurnError/Idle→false）+ `RustTurnState`；`smEvents` 加 `SmAg01Payload`、`smListeners` 加监听；`agService` 保留乐观 `setSending(true)` 防双发，`finish()` 去 `setSending(false)`（改广播释放），命令级 Err catch 兜底释放。messages/toolCalls 仍由 Channel 投影（互不替代）。验证：cargo 143、vitest 69（新增 agStore.test.ts）、tsc 干净、audit 无新增 finding |
| 2026-06-27 | v1.6 | **SM-WS-01 保存机集成落地（Task 2-C）**：人类决策（2026-06-27）脏态（Idle/Dirty）留前端高频本地标记，仅保存**动作**走状态机。Rust：`route_save_doc`/`route_save_from_edit` 经共享助手 `save_atomic_sm` 路由 `save::transition`（Dirty→Saving→Saved/SaveError）并广播 `sm:SM-WS-01:state_changed`（instance=rel_path）。前端：`edStore` 保留 `markDirty`/`setTabState`，加 `applyWsSaveState`（Saving→saving、Saved→saved，不覆盖 external_changed/pending_diff）+ `setBaseline`（仅刷基线）+ `RustSaveState`/`mapSaveState`；`smEvents` 加 `SmWs01Payload`、`smListeners` 加监听；`edService.saveDocument/saveDocxDocument` 去乐观 saving/saved（改广播驱动），基线由后端返回值回填，冲突/失败细分仍由 catch 据错误码定。验证：cargo 143、vitest 67、tsc 干净、audit 无新增 finding |
| 2026-06-27 | v1.5 | **SM-WS-02 落盘机集成落地（Task 2-D）**：四个落盘入口（`route_accept_diff`/`route_apply_diff`/`route_reject_diff`/`route_discard_diff`）经新增共享助手 `apply_diff_sm`/`discard_diff_sm` 路由 `diff_apply::transition`（AwaitingDecision→Applying→Applied/ApplyError / Discarded）并广播 `sm:SM-WS-02:state_changed`；`diff_apply` 加 `is_terminal`。前端 `deStore` 加 `applyWsApplyState`（Applying→applying 过渡、Applied→accepted、ApplyError→error、Discarded/Awaiting 不投影）+ `RustDiffApplyState`/`mapWsApplyState`；`smEvents.ts` 加 `SmWs02Payload`、`smListeners` 加 SM-WS-02 监听；`deService.acceptDiff` 去掉本地乐观 validating/applying（改由 SM-WS-02 广播驱动）。验证：cargo 143、vitest 66、tsc 干净、audit 无新增 finding。落盘过渡态现真正由后端落盘机驱动（X-FORBID-007 闭合度提升） |
