# Issue Trace · PI-AG-CTX-DIAG-02 上下文基线误判与失败原因透传

## 1. 用户可见症状

- 澄清恢复（clarification resume）后首次 `propose_edit` 被拒 `AG_CONTEXT_RESOURCE_STALE`，提示“文档或选区已变化”，但目标文档实际未修改（mtime/readVersion/contentHash 均未变）。
- 失败回执与 UI 只呈现通用文案或错误码，模型无法据此定位应修正的参数或缺失的读取基线；重试回合连续 2 次 `list_blocks` 非法 `cursor`、1 次 `read_doc` `blocks`+`view` 混用后触发 `AG_MODEL_CORRECTION_LIMIT` 终止。

## 2. 入口点

- `ag_start_turn` → `run_turn_tool_loop` → `authorize_one_tool_call`（src-tauri/src/ag/turn_tool_loop.rs）
- 上下文刷新：`refresh_tool_context` → `turn_runtime::refresh_agent_context` / `parse_agent_context_facts`（src-tauri/src/ag/turn_runtime.rs）
- 工具失败回注：`execute_tool_call` → `record_tool_result` / `model_tool_result_message`（src-tauri/src/ag/turn_tool_loop.rs）
- 读工具契约：`validate_read_arguments` / `decode_read_cursor`（src-tauri/src/ag/tool_exec.rs）

## 3. 相关规则

- BR-AG-DATA-008（上下文事实校验）
- BR-AG-STATE-002（工具循环状态）
- BR-AG-DATA-011（工具回执与模型回注）
- BR-AG-DATA-005（读工具分页/结构读取）
- BR-AG-OBS-001（证据与观测）
- BR-SYS-DATA-008（上下文事实通道）

## 4. 相关链路

AG-TURN → AG-TOOL → AG-TURN（失败回注）

## 5. 生产者块

- `[BR-AG-DATA-008-GUARD-AG-AG-TURN-042]` 位于 src-tauri/src/ag/turn_runtime.rs
- `[BR-AG-DATA-008-GUARD-AG-AG-TURN-052]` 位于 src-tauri/src/ag/turn_runtime.rs
- `[BR-AG-DATA-011-EFFECT-AG-AG-TOOL-070]` 位于 src-tauri/src/ag/turn_tool_loop.rs
- `[BR-AG-DATA-005-GUARD-AG-AG-TOOL-057]` 位于 src-tauri/src/ag/tool_exec.rs

## 6. 消费者块

- `[BR-AG-STATE-002-DATA-AG-AG-TOOL-044]` 位于 src-tauri/src/ag/tool_exec.rs
- `[BR-AG-DATA-011-DATA-AG-AG-TOOL-062]` 位于 src-tauri/src/ag/turn_tool_loop.rs
- `run_turn_tool_loop` 的 continuation messages（模型下一轮输入）

## 7. 路径上的空规则块

无（相关块 rules 均非空）。

## 8. boundary 假设

- 相关块未声明 `lifecycle_assumption=` / `document_snapshot=`；boundary 以 in/out/does_not 表达。
- `parse_agent_context_facts` 的 does_not 含 “acceptUnknownVersionCrossWorkspaceFacts”，本次修改不放宽版本/工作区校验，只修正“旧基线不存在”时的相等比较。
- `record_tool_result` 的 does_not 含 “fabricateEvidence”，本次只在失败回执中补充脱敏原因文本，不新增证据。

## 9. 运行时假设

- 相关块未声明 `caller_context=` / `runtime_guard=`。
- 依赖既有假设：`propose_edit` 必须经 fresh host context 刷新；刷新仍要求 `new snapshot` 可用、选区 `Valid`。

## 10. 疑似不变量破坏

- 风险：把 stale 判定放宽后，可能放过“文档确实变化”的编辑。
- 约束：仅在 `previous.snapshot == None`（无基线）时接受本次采集；`new.snapshot == None`、选区失效、revision/目标/资源身份变化仍拒绝。
- 不得在失败原因文本中携带文档正文、令牌或密钥；只允许路径、版本号、哈希、布尔与枚举。

## 11. 验证计划

- 单测/集成：`cargo test --manifest-path src-tauri/Cargo.toml`（含 turn_runtime / turn_tool_loop / tool_exec 相关单测）。
- 变更影响：`npm run test:plan` 与 `npm run test:related -- --files <changed>`；`npm run governance:audit`。
- 日志验证：`[AG-CONTEXT-STALE]` 记录 `cause`；`[AG-CONTEXT-BASELINE]` 记录无基线接受；工具失败回执包含具体原因。
- 手工复验：澄清恢复后 `propose_edit` 不再误判；`read_doc blocks+view=text` 的拒绝文案包含冲突字段与修正动作。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: PI-AG-CTX-DIAG-02
allowed_files:
  - src-tauri/src/ag/turn_runtime.rs    # 无基线不判 stale；错误消息带具体原因
  - src-tauri/src/ag/turn_tool_loop.rs  # 上下文刷新失败向模型/事件回传具体 message
  - src-tauri/src/ag/tool_exec.rs       # 读参数拒绝文案带冲突字段与修正动作
  - .governance/issues/PI-AG-CTX-DIAG-02.md
  - .governance/active_scope.json
forbidden_files:
  - ADU.md / ADP.md / AGENTS.md / CLAUDE.md / ADUS.md / APC / governance.config.json
  - .governance/test-governance.json / scripts/**
  - src/** (前端) / cloud/**
confirmation: PENDING
scope_reason: 修复上下文基线误判并让失败原因可执行；仅触及 AG 运行时校验与回注文案。
```
