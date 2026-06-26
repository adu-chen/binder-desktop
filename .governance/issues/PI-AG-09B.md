# Issue Trace · PI-AG-09B 对话历史 + P0–P3 预算截断（完整 PI-AG-09 第①步）

承接 A-AG-X-L-02 PI-AG-09 / A-AG-M-T-02 BR-AG-DATA-005（分层截断序：当前文档/当前轮 P0、历史 P1）/
A-AG-M-T-01 BR-AG-DATA-001（确定性裁剪、可追溯不静默）。补两个缺口：AI 不带对话历史（无状态）+ 整篇
注入无预算（大文档吃 token）。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: PI-AG-09B
allowed_files:
  - src-tauri/src/ag/context.rs   # HistTurn / cap_text（P0 文档封顶）/ select_recent_history（P1 历史预算）
  - src-tauri/src/lib.rs          # ag_start_turn 接 history；组装 system→历史(预算)→文档(capped)→user（recency）
  - src/services/agService.ts     # 传 agStore 历史快照（本轮 user 入栈前，最近 30 条）
  - .governance/active_scope.json / issues/PI-AG-09B.md
forbidden_files: ADU/ADP/CLAUDE/APC / ipc/mod.rs / edStore / agStore
confirmation: YES
```

## Expected Behavior

- `ag::context`：`cap_text(s,max,note)`——P0 文档层内容封顶（超长截断 + 显式注记，不静默）；
  `select_recent_history(turns,budget)`——P1 历史按预算保留最近、drop 最旧（确定性，返回丢弃数可追溯）。
- `ag_start_turn`：新增 `history: Vec<HistTurn>` 入参；当前文档块视图封顶 DOC_CTX_CAP=9000；总预算
  CTX_BUDGET_CHARS=16000 扣 P0 后余额给历史；组装顺序 **系统 → 历史 → 当前文档(贴末尾) → 当前 user**
  （M-T-02 §3 recency，当前轮事实靠近末尾）。
- `agService`：本轮 user 入栈**前**取 agStore 历史快照（去 error/空，最近 30 条）→ 传 history（后端再按预算裁）。

## Validation Commands

```
cargo build --lib && cargo test --lib 'ag::context'   # 5/5（含 cap_text/select_recent_history）
npx tsc --noEmit
# 端到端：多轮对话 AI 记得上文；超长文档注入被封顶不爆 token（人类自验）
```

验收：cargo build + ag::context 5/5；tsc 干净；历史按预算保留最近、大文档封顶；组装顺序 recency。

## 后续（完整 PI-AG-09 余项）

- ②意图沿用判定（BR-AG-DATA-004）：在历史之上加判定——默认弱化历史、显式沿用才提权。
- ③REF @mention / AST 记忆 / KB 知识供给：经 assemble_context（P2 补强源预算 + 对齐 M-T-02 当前文档 P0 优先级）。
