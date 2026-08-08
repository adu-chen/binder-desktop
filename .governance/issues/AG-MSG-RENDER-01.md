# Issue Trace — AG-MSG-RENDER-01：AI 消息 Markdown 渲染与回合内有序流式排列

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-07

## 1. 用户可见症状

- AI 回复显示原始 `#`、`*`、表格竖线等 Markdown 结构符号，排版原始。
- 工具调用卡与 Diff 卡全部沉在单条回复正文底部，与正文无时序交错（模型文本→工具→文本→工具的流被拍平为"全部正文在上、全部卡片在下"）。

## 2. 入口点

- 前端消息渲染：`src/components/ag/MessageList.tsx` — `MessageList`（`m.content` 直出、`ToolCallList`/`DiffList` 按 `anchorMessageId` 沉底）。
- 前端事件消费：`src/services/agService.ts` — `startTurn` 的 `onEvent`（chunk/tool_request/tool_result/diff_proposed）。
- 前端回合状态：`src/store/agStore.ts` — `ChatMessage`（`content: string`）、`appendToMessage`、`addToolCall`/`resolveToolCall`。
- 后端流式发射：`src-tauri/src/ipc/mod.rs` — `AgTurnEvent` 六分支（已按时间顺序发射）。
- 后端持久化：`src-tauri/src/ag/history.rs` — `AgentTurnPayload`（`save_turn` / `list_turns`）。
- 后端提示词：`src-tauri/src/ag/prompts.rs` — `SYSTEM_PROMPT_CHAT` 输出风格。

## 3. 相关规则

- `BR-AG-STATE-001`（回合生命周期 / DiffCard 消息锚点内嵌）
- `BR-AG-STATE-002`（工具循环 / 工具卡展示）
- `BR-AG-PERSIST-001`（回合历史持久化 / payload 结构 / 恢复锚点）
- `BR-AG-DATA-002`（组装骨架 / 格式层）
- `BR-SYS-STATE-002`（流式三终态，本改动不破坏）
- `X-FORBID-006`（AG 不直写内容；渲染为纯展示，不触及）
- 新增 `BR-AG-UI-001`（消息流渲染与时序契约，随本文注册）

## 4. 相关链路

`AG-TURN → AG-TOOL → AG-TURN`（工具循环回注）＋ 展示子域（归 AG-TURN，渲染/时序契约见 BR-AG-UI-001）。

## 5. 生产者块

- `BR-AG-DATA-001-EFFECT-AG-AG-TURN-005` 位于 `src/services/agService.ts`（OpenDocSnapshot 组装）
- `BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-002` 位于 `src/services/agService.ts`（assist 先例）
- 后端 `AgTurnEvent`（Chunk/ToolRequest/ToolResult/DiffProposed）位于 `src-tauri/src/ipc/mod.rs`、`src-tauri/src/ent/gateway.rs`、`src-tauri/src/lib.rs`

## 6. 消费者块

- `MessageList` / 新增 `MessageFlow` / `MessageMarkdown`（**当前无 @GOV，本次随 BR-AG-UI-001 注册补标**）
- `ToolCallCard`（工具段消费）

## 7. 路径上的空规则块

- 无 `rules: []` 块；但 `MessageList.tsx` / `AgentPanel.tsx` 缺 @GOV → 本次范围内按 `BLOCK_UNANNOTATED` 补齐（MessageList）。

## 8. boundary 假设

- 现有 EFFECT 块均为 `effect=memory`（快照/assist），渲染为纯展示无副作用，假设成立。
- `document_snapshot=` 不适用于本路径（无文档状态读写）。

## 9. 运行时假设

- `caller_context=agentTurnSnapshotAssembly`（agService 快照路径）不受渲染改动影响。
- 新增 `MessageMarkdown` 为纯渲染，无 runtime_guard 需求。
- 事件到达顺序（Tauri Channel FIFO）为时序重建的运行时依据——成立。

## 10. 疑似不变量破坏

- 潜在：`BR-AG-PERSIST-001` payload 结构（采用 additive `assistantMessage.segments`，旧记录无字段 → `None` → 前端降级，可验证）。
- 潜在：`BR-AG-STATE-001` DiffCard 锚点语义由"消息后"细化为"消息内时序位置"（随本文修订，需人类确认）。
- 未发现写盘路径、订阅门禁、workspace 限界（X-FORBID-004/006）不变量破坏。

## 11. 验证计划

- 组件测试：`MessageMarkdown` 渲染快照（标题/加粗/表格/代码块/列表/链接）、sanitize/XSS 用例；`MessageFlow` 时序断言（text→tool→text→tool→diff）；legacy 无 segments 降级。
- 服务测试：`agService` 事件消费构造 segments、多同名工具按 id 配对、payload 序列化/恢复重建。
- Rust：`history.rs` struct 反序列化兼容（旧 payload 无新字段 → `None`）。
- `npm run test:related -- --files [changed]`（AG critical → 联动 SYS/Rust）、`npm run governance:audit`。
- 链路日志序列：Chunk→ToolRequest→ToolResult→DiffProposed→下一轮 Chunk 顺序重建正确。

## 文件范围（Scope Lock 输入）

允许修改：
- `src/components/ag/MessageList.tsx`、`src/components/ag/AgentPanel.tsx`（仅若需）、新增 `MessageFlow.tsx` / `MessageMarkdown.tsx`
- `src/store/agStore.ts`、`src/services/agService.ts`、`src/services/agRuntime.test.ts`、`src/components/ag/AgentUI.test.tsx`
- `src/styles.css`
- `src-tauri/src/ag/prompts.rs`、`src-tauri/src/ag/history.rs`、`src-tauri/src/ipc/mod.rs`（仅注释）
- `package.json` / `package-lock.json`（新依赖 react-markdown / remark-gfm / rehype-sanitize）
- `docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md`、`A-AG-M-T-02_提示词架构设计.md`、`A-AG-I-P-01_代理接口协议.md`、`docs/01_modules/SYS/A-SYS-C-T-02_前端交互架构与流程总图.md`
- `docs/00_core/A-CORE-C-R-01_ADUS.md`（仅 generate 产物）

禁止修改：
- 治理控制文档（ADU/ADP/AGENTS/CLAUDE）、APC、governance.config.json
- 不在允许列表中的文件默认禁止
