# Issue Trace · PI-AG-08C propose_edit 写工具（X-FORBID-006 闭环）

承接 A-AG-X-L-02 PI-AG-08（propose_edit 写半）/ A-AG-M-T-01 X-FORBID-006（AI 修改唯一经 AG 产出→DE
diff→WS 落盘，不直写）/ BR-AG-STATE-002（工具校验）/ A-DE-M-T-01 BR-DE-DATA-001/002（diff 生成/载荷）/
X-INTER-001（accepted 唯一触发 ws.apply_diff）/ X-FORBID-004（限 workspace）。让 AI 能真正"提出修改"。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: PI-AG-08C
allowed_files:
  - src-tauri/src/lib.rs              # ag_propose_edit 助手 + 工具循环 propose_edit 分支 + emit DiffProposed
  - src-tauri/src/ag/tool_registry.rs # tool_definitions_json 补 propose_edit
  - src/services/agService.ts         # DiffProposed → deStore.addDiff（调既有 API）
  - .governance/active_scope.json / issues/PI-AG-08C.md
forbidden_files: ADU/ADP/CLAUDE/APC / ipc/mod.rs / de/generate.rs（复用不改）/ deStore.ts / edStore.ts
confirmation: YES
```

## Expected Behavior

- `propose_edit(path, content)`：模型给目标文档相对路径 + 修改后完整内容（dev：整文档替换）。
- `ag_propose_edit`（lib.rs）：复用 route_generate_diff 逻辑——解析 active workspace + resolve_in_workspace
  守界（X-FORBID-004）→ before_snapshot=现文件内容（存在则 modify、不存在则 new doc）→ ChangeProposal →
  generate_diff（Located）→ persist_payload + persist_diff（BR-DE-DATA-001/002）→ 返回 diff_id。
- 工具循环：propose_edit（WriteIntent，validate Allow）→ ag_propose_edit → emit `DiffProposed{diff_id,rel_path}`
  + 回注模型"已生成修改建议待用户确认，勿重复提交"。**AG 不直写文件**（X-FORBID-006）。
- 前端：agService 收 DiffProposed → `deStore.addDiff`（pending 卡片，before=现内容片段、after=占位）；接受经
  既有 `deService.acceptDiff`→`de.accept_diff`→`ws.apply_diff` 真落盘（X-INTER-001），编辑器重载显示结果。

## Validation Commands

```
cargo build --lib && cargo test --lib 'ag::' 'de::'
npx tsc --noEmit
# 端到端（app+key+workspace）：开文档 → 让 AI 改它 → 出 diff 卡片 → 接受 → 文档落盘更新（人类自验）
```

验收：cargo build 干净 + ag/de 测试过；tsc 干净；propose_edit 经 DE 持久化生成 pending diff（不直写文件，
X-FORBID-006）；DiffProposed→卡片；接受经既有链真落盘。

## 后续

- 块级锚定编辑（block_index+target，替整文档替换，省 token + 精确 in-editor diff）；卡片 after 真预览
  （需把提案内容给前端，AgTurnEvent::DiffProposed 形状或旁路）；前端工具卡片 UI；docx(html) before_snapshot。
