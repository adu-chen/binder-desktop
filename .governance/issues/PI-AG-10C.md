# Issue Trace · PI-AG-10C 流式 IPC 共享契约（AgTurnEvent 落地 · 双轨同步冻结点）

> ✅ **状态：ACTIVE（2026-06-26 激活）**。人类确认并行窗口已完结、授权开始 agent 开发后，
> Scope Lock 已写入 `.governance/active_scope.json`（confirmation=YES）。并发 PI-FE-28 的暂存改动
> （EditorTab 删除、Cargo.lock）不触碰。
>
> **命名修正（落地发现）**：`src-tauri/src/ipc/mod.rs` 已存在 SYS `StreamEvent`（通用 dispatch 传输
> 契约）。为避免重名+逻辑重复，AG 回合流式事件定名 **AgTurnEvent**（per-call Channel、turn_id），
> 与 SYS StreamEvent 分层不同·非重复（见 A-AG-I-P-01 §2.1 v1.2）。

承接 A-AG-I-P-01 §2.1（流式回传契约 AgTurnEvent 六分支 + ag_start_turn 签名，**已写入设计侧**）/
BR-SYS-STATE-002（流式三终态：完成/中断/错误）/ SY-02 选型（Tauri 2 `Channel<T>` 物理传输，
A-AG-X-L-02 PI-AG-10C）。本 PI 是前后端双轨（PI-AG-10B 后端发射 / PI-AG-10F 前端呈现）的**同步锚点**：
冻结后双轨才开。

设计侧已完成（本会话 2026-06-24）：A-AG-I-P-01 §2.1 已固化 AgTurnEvent schema 与 ag_start_turn 签名；
本 PI 仅剩**代码契约落地**——把 AgTurnEvent enum 落进 `ipc/mod.rs`。

## Scope Lock（PREPARED · 待 PI-TEST-01 释放后写入 active_scope.json）

```
task_type: CODE_CHANGE
scope_id: PI-AG-10C
allowed_files:
  - src-tauri/src/ipc/mod.rs                              # AgTurnEvent enum 定义（serde tag=type snake_case，六分支）
  - docs/01_modules/AG/A-AG-I-P-01_对话代理接口协议.md   # §2.1 契约（设计侧已写，代码期如需微调）
  - .governance/active_scope.json
  - .governance/issues/PI-AG-10C.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/src/lib.rs                                  # ag_start_turn 命令注册/发射归 PI-AG-10B（后端轨）
  - src-tauri/src/ag/                                     # 提示词/工具/上下文归 PI-AG-07/08/09
  - src/                                                  # 前端流式呈现归 PI-AG-10F（前端轨）
confirmation: YES   # 已激活（2026-06-26，人类授权）
scope_reason: PI-AG-10C 把已冻结的 AgTurnEvent 契约（A-AG-I-P-01 §2.1）落为 ipc/mod.rs 的 enum 定义，作为前后端双轨同步真源；不含命令注册（10B）与前端（10F）。激活须待并发 PI-TEST-01 释放 active_scope（其 forbidden_files 含 ipc/mod.rs）。
```

## Expected Behavior

- `src-tauri/src/ipc/mod.rs` 新增 `AgTurnEvent` 枚举，六分支与 A-AG-I-P-01 §2.1 **逐字段一致**：
  `Chunk{turn_id,text}` / `ToolRequest{turn_id,tool_name,params}` / `ToolResult{turn_id,tool_name,result,success}` /
  `DiffProposed{turn_id,diff_id,rel_path}` / `Done{turn_id,stop_reason}` / `Error{turn_id,code,message,retriable}`。
- `#[derive(Clone, Serialize)]` + `#[serde(tag = "type", rename_all = "snake_case")]`，序列化形态与 §2.1 一致（供前端 PI-AG-10F 按 `type` 分发）。
- 仅类型定义，**不含命令注册、不含发射逻辑、不含前端**（分别归 10B / 10F）。
- 不新增 RULE/CHAIN/CONSTRAINT/TERM（AgTurnEvent 是接口类型，非注册类治理概念）。

## Validation Commands

```
cargo test                  # AgTurnEvent 编译 + serde 往返（tag=type、snake_case 形态）
npx tsc --noEmit            # 若前端已建对应类型镜像则校验，否则无影响
npm run governance:audit -- --issue .governance/issues/PI-AG-10C.md
```

验收：AgTurnEvent 六分支编译通过；serde 序列化形态与 A-AG-I-P-01 §2.1 一致（单测断言 `type` 标签 +
snake_case 字段）；scoped 门禁无越界（未触 lib.rs / ag/ / src/）；无新增治理块、audit 无新错误类。

## 后续（激活后）

1. 本 PI 合入后，PI-AG-10B（后端 Channel 命令 + 桩发射）与 PI-AG-10F（前端呈现）即可对同一 AgTurnEvent 并行起轨。
2. 桩阶段：`route_start_turn` 经 channel 推 `Chunk×N + Done`，供 PI-AG-10F 对桩联调；GW 就绪后 PI-AG-11 接真实发射，前端零改动。
