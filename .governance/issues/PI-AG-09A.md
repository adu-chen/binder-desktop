# Issue Trace · PI-AG-09A 当前文档注入 + 提示词诚实化（修测试暴露问题）

承接 A-AG-X-L-02 PI-AG-09（当前文档供给子集）/ A-AG-M-T-02 BR-AG-DATA-005（当前文档为 P0 主事实，
直接注入而非靠工具）/ A-AG-M-T-01 §5.1 BR-AG-DATA-002（当前文档/编辑器交互层）。

## 背景（测试暴露）

用户在 app 内问"当前文档是什么内容"，AI 答不知道、且幻觉声称"我使用 list_docs 工具…"。根因：
1. `ag_start_turn` 只发 system+user，**未注入当前编辑器文档** → AI 看不到。
2. `SYSTEM_PROMPT_CHAT` 指示"用 read_doc/list_docs/search_text"，但**工具未接线**（GW 无 function-calling、
   无工具循环）→ 模型幻觉调用工具。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: PI-AG-09A
allowed_files:
  - src-tauri/src/lib.rs            # ag_start_turn 接收并注入 active_doc_path/content
  - src-tauri/src/ag/prompts.rs     # SYSTEM_PROMPT_CHAT 诚实化（当前文档在上下文 / 去工具过度声称）
  - src/services/agService.ts       # 从 edStore.getSnapshot() 读 openPath/loadedContent 传入（只读 edStore）
  - .governance/active_scope.json
  - .governance/issues/PI-AG-09A.md
forbidden_files: ADU/ADP/CLAUDE/APC / ipc/mod.rs / edStore.ts（只读不改） / EditorTab
confirmation: YES
```

## Expected Behavior

- `ag_start_turn` 新增 `active_doc_path: Option<String>` + `active_doc_content: Option<String>`；非空时在
  system 与 user 之间插入「当前编辑器打开的文档」上下文消息（路径 + 内容；BR-AG-DATA-005 P0 主事实）。
- `SYSTEM_PROMPT_CHAT`：声明"你能看到当前编辑器打开的文档（内容在下文提供），据此回答关于当前文档的问题"；
  **去除** read_doc/list_docs/search_text 的工具调用指示（未接线）；修改以方案形式给出由用户确认（不直写）；
  其他文档浏览/搜索/自动应用等工具能力**标注尚在接入、暂不可用，不得声称已调用工具**。
- `agService.startTurn`：`edStore.getSnapshot()` 读 `openPath`/`loadedContent` → 传 `activeDocPath`/`activeDocContent`。

## Validation Commands

```
cargo build --lib
npx tsc --noEmit            # agService 无 TS 错误
# 端到端：app 内开一个文档 → 问"当前文档是什么内容" → AI 据注入内容作答（人类自验）
```

验收：cargo build 干净；tsc agService 无错误；注入逻辑在内容非空时生效；提示词不再指示调用未接线工具。

## 后续

- 完整 PI-AG-09：意图沿用判定（BR-AG-DATA-004）+ REF/AST/KB 供给 + P0–P3 分层截断（当前为整篇注入、无预算）。
- PI-AG-08B：真实工具（GW 传 tool 定义 + function-calling + SM-AG-02 工具循环），届时恢复 read_doc/list_docs 等。
