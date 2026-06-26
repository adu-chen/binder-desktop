# Issue Trace · PI-AG-08A 工具注册表（PI-AG-08 拆分前半）

承接 A-AG-X-L-02 §3 PI-AG-08（拆分）/ A-AG-M-T-01 BR-AG-STATE-002（工具校验：注册清单 + 权限边界 +
写盘禁令）/ X-FORBID-006（写意图经 DE 不直写）/ X-FORBID-004（只读工具限 workspace）。

**拆分理由（同 10C/10B/10F 模式）**：`tool_exec` 的真实执行需运行期上下文（conn / AppState /
propose_edit 的 `anchor_locate: LocateResult`，由 ed.locate 在模型循环内提供），且与 lib.rs 既有
`route_read_doc/route_list_docs/route_generate_diff` 重叠。故 PI-AG-08 拆为：
- **08A 工具注册表**（本 PI）：纯数据——4 工具 + ReadOnly/WriteIntent 权限分级，喂 validate_tool。
- **08B 工具执行**（后续）：ws/kb/de 真实执行，与模型循环 PI-AG-11 配套，避免重复 lib.rs route_*。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: PI-AG-08A
allowed_files:
  - src-tauri/src/ag/tool_registry.rs   # 新建：4 工具 + 权限分级 + validate_requested_tool
  - src-tauri/src/ag/mod.rs            # pub mod tool_registry
  - docs/01_modules/AG/A-AG-X-L-02_对话代理集成计划.md  # 记录 08A/08B 拆分
  - .governance/active_scope.json
  - .governance/issues/PI-AG-08A.md
forbidden_files:
  - ADU/ADP/CLAUDE/APC
  - src-tauri/src/ipc/mod.rs / lib.rs / ag/tool.rs / ag/context.rs  # 不改既有
  - src/**
confirmation: YES
```

## Expected Behavior

- `tool_registry.rs`：`ToolPermission { ReadOnly, WriteIntent }`、`ToolDef { name, permission }`、
  `TOOLS` 常量（read_doc/list_docs/search_text=ReadOnly，propose_edit=WriteIntent）；`lookup`/`is_registered`。
- `validate_requested_tool(name)`：承 BR-AG-STATE-002 `validate_tool`——注册即在权限边界内、`direct_write_intent=false`
  （4 工具均不直写盘：只读工具读 WS/KB，propose_edit 产出修改建议走 DE）；未注册 → Reject。
- propose_edit 注册为 WriteIntent 但 direct_write_intent=false：它是「产出修改建议」工具（合法），执行期
  由 08B 路由到 ChangeProposal→DE，不绕过 DE 直写。
- `ag/mod.rs` 增 `pub mod tool_registry;`。

## Validation Commands

```
cargo test --lib ag::tool_registry
cargo build --lib
npm run governance:audit -- --issue .governance/issues/PI-AG-08A.md
```

验收：4 工具注册、未注册名 `is_registered=false` 且 `validate_requested_tool` Reject；read_doc=ReadOnly /
propose_edit=WriteIntent；4 工具 `validate_requested_tool` 均 Allow（含 propose_edit，非直写）；
scoped 门禁无越界（未触 tool.rs / lib.rs / ipc / src/）。
