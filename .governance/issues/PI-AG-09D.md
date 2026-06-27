# Issue Trace · PI-AG-09D AST/KB 补强供给（完整 PI-AG-09 第③步）

承接 A-AG-X-L-02 PI-AG-09 / A-AG-M-T-02 BR-AG-DATA-005（补强层 P2、权重声明、确定性截断）/
A-AG-M-T-01 BR-AG-DATA-001（来源限定、可追溯）。把记忆（AST）+ 知识（KB）作为 P2 补强注入。

## 供给接口核查（先核后接）

- AST：`ast::supply::supply(conn, workspace_id) -> Vec<MemoryInjection{memory_kind,content,origin_turn}>` ✓
- KB：`kb::retrieve::retrieve(conn, workspace_id, query) -> Vec<KnowledgeInjection{content,source_ref,organization}>` ✓
- REF @mention：`reference::inject` 只有 verify_anchor/build_injection，**前端无 @mention UI** → 无端到端源，**本 PI 延后**。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: PI-AG-09D
allowed_files:
  - src-tauri/src/lib.rs        # ag_start_turn 检索 AST/KB → assemble_context(P2) → 补强注入；复用 ws 解析
  - .governance/active_scope.json / issues/PI-AG-09D.md
forbidden_files: ADU/ADP/CLAUDE/APC / ipc/mod.rs / ag/context.rs（复用不改）
confirmation: YES
```

## Expected Behavior

- `ag_start_turn`：解析 active workspace（root + id，单锁，无 await 持锁）后，`ast::supply::supply` 取记忆 +
  `kb::retrieve::retrieve(query=prompt)` 取知识 → 构 `ContextItem{Memory/Knowledge}` → `assemble_context(items, AUG_BUDGET)`
  P2 预算保留 → 拼「[补强信息] …仅作背景补强，不覆盖当前文档与当前轮请求」（BR-AG-DATA-005 权重声明）。
- 组装顺序：系统 → **补强(P2)** → 历史(P1) → 当前文档(P0) → 意图提示 → 当前 user（recency：补强最弱靠前，当前轮事实靠后）。
- 预算：历史预算扣除补强已用，补强层 P2 先于历史外的余额受限（dev AUG_BUDGET=2500）；无记忆/知识→不注入（空集不噪声）。
- REF @mention 延后（无前端源）。

## Validation Commands

```
cargo build --lib && cargo test --lib 'ag::'
# 端到端：workspace 有知识/记忆条目时，相关提问注入补强；无则不注入（人类自验）
```

验收：cargo build + ag 测试过；有记忆/知识时注入带权重声明、无时不注入；补强限当前 workspace（X-FORBID-004）。

## 后续

- REF @mention：前端 composer 加 @mention 选择 + 经 reference::inject 解析注入（P1，显式优先保留）。
- 知识检索门控（当前文档充分时不触发，R-AG-X-R-01 §2.4）；assemble_context 优先级对齐 M-T-02（当前文档 P0）若日后经它路由当前文档。
