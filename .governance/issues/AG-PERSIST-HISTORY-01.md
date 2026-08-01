# Issue Trace · AG-PERSIST-HISTORY-01 代理编辑历史持久化

承接 `BR-AG-PERSIST-001`、`BR-SYS-PERSIST-001`、`X-SHARED-001` 与用户确认口径：
完成态、错误、取消、门禁拒绝均保留；流式中断/崩溃遗留为 `draft`，重启后不得伪装完成；
会话标签元数据全部恢复，内容仅 eager-load 当前激活标签，其余点击懒加载；工具卡只存结构化摘要；
DiffCard 只存 `diff_id` / 文件路径引用，不存 diff 快照；历史存于 app data 下 `workspace.db`；
保留策略按 workspace 软上限 200MB，超限先软归档；用户可在历史会话列表删除单个代理编辑会话，
删除当前 workspace 内该会话的 AG 会话/回合/payload 历史记录。

## 1. 用户可见症状

代理编辑面板的历史会话标签、用户消息、助手回复、工具卡和 DiffCard 归属当前仅在运行期内存保存。
应用重启或 workspace 切换后，用户看不到先前的成功、错误、取消或门禁拒绝回合，历史上下文表现为丢失。

## 2. 入口点

- 用户打开 / 切换 / 恢复 workspace：`src/services/workspaceService.ts`
- 用户切换或关闭代理编辑会话标签：`src/components/ag/AgentPanel.tsx`
- 用户发送代理编辑消息：`src/services/agService.ts` `startTurn`
- 用户删除单个历史会话：`src/components/ag/AgentPanel.tsx` → `src/services/agService.ts`
  `deleteAgentSession` → `ag.delete_session`
- 后端流式回合入口：`src-tauri/src/lib.rs` `ag_start_turn`
- 重启恢复查询：新增 `ag.list_sessions` / `ag.list_turns`

## 3. 相关规则

- `BR-AG-PERSIST-001`
- `BR-AG-STATE-001`
- `BR-AG-VERIFY-001`
- `BR-AG-STATE-002`
- `BR-SYS-PERSIST-001`
- `BR-SYS-DATA-003`
- `X-SHARED-001`
- `X-FORBID-006`
- `X-FORBID-007`

## 4. 相关链路

`DATA-MIGRATE -> AG-TURN -> AG-TOOL -> IPC-CALL`

## 5. 生产者块

- `BR-SYS-PERSIST-001-EFFECT-SYS-DATA-MIGRATE-001` 位于 `src-tauri/src/db/mod.rs`
- `BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-001` 位于 `src-tauri/src/ag/turn.rs`
- `BR-AG-PERSIST-001-QUERY-AG-AG-TURN-001` 位于 `src-tauri/src/ag/turn.rs`
- `BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-004` 位于 `src-tauri/src/ag/history.rs`
- `BR-SYS-DATA-003-GUARD-SYS-IPC-CALL-004` 位于 `src-tauri/src/lib.rs`
- `ag_start_turn` 位于 `src-tauri/src/lib.rs`（本次补回合持久化写入）
- `startTurn` 位于 `src/services/agService.ts`（本次补 sessionId 与前端持久化投影）

## 6. 消费者块

- `src/store/agStore.ts` 消费会话、消息、工具摘要与懒加载状态，投影到 AgentPanel/MessageList
- `src/components/ag/AgentPanel.tsx` 消费会话标签元数据，恢复打开/归档顺序
- `src/components/ag/MessageList.tsx` 消费历史 turn 内容并渲染 draft/error/gate_rejected 等状态
- `src/components/ag/ToolCallCard.tsx` 消费只读历史工具摘要
- `src/store/deStore.ts` / `src/components/de/DiffCard.tsx` 消费 DiffCard 引用恢复后的 DE/WS 当前状态
- `src/services/workspaceService.ts` 在 workspace 生命周期消费恢复接口

## 7. 路径上的空规则块

本次不新增业务 `rules: []` 块。新增运行时代码若属于链路入口或核心业务块，必须补 `@GOV`；
测试文件按 ADU §5.6 使用 `// covers:` 注释。

## 8. boundary 假设

- `BR-AG-PERSIST-001` 原只锁「本地持久化 + 可恢复 + 不上云」原则，列定义尚未回写；本次先更新设计与 SYS 数据结构再实现。
- `BR-SYS-PERSIST-001` 的版本化迁移假设成立；本次只追加新迁移，不改历史迁移。
- `X-FORBID-006` 假设成立：历史 payload 不保存 diff 内容快照，DiffCard 只保存引用并与 DE/WS 当前状态合并展示。
- `X-FORBID-007` 假设成立：前端 agStore/deStore 只投影后端持久化控制态，不把本地视图态作为业务权威。

## 9. 运行时假设

- `workspace.db` 位于 app data 目录，按 workspace_id 隔离，不跟随项目目录。
- `ag_start_turn` 可在用户消息入栈后立即写 `draft`，终态时更新为 `completed/error/cancelled/gate_rejected`。
- 应用崩溃或流式中断时，未被终态更新的记录保持 `draft`；重启后只显示未完成/可重试，不触发旧工具或模型调用。
- 点击历史 draft 的重试动作复用同一条 user message 新建新 AgentTurn，不覆盖旧 draft。
- 用户删除历史会话时前端先确认且禁止活动代理回合期间触发；service 删除前等待已排队的
  `ag.save_turn` 完成，避免异步保存把旧回合写回。
- `ag.delete_session` 仅清当前 workspace 内指定会话的 AG 历史记录，不删除 DE `pending_diffs` /
  `diff_payloads`；删除成功后前端移除该会话，若删除活动会话则切到剩余会话或创建空会话。

## 10. 疑似不变量破坏

- 若把 Diff 内容快照写进 AG payload，会破坏 DE/WS 当前状态一致性；本次明确禁止。
- 若仅恢复成功回合，会破坏门禁拒绝/错误/取消的审计与排错上下文；本次必须全状态保留。
- 若一次 eager-load 所有会话 payload，可能破坏启动性能；本次只 eager-load 激活标签，其他懒加载。
- 若把历史文件放进 workspace 目录，可能造成用户历史随项目分享/提交泄露；本次继续使用 app data `workspace.db`。
- 若恢复时只按全局 pending diff 列表建卡，会把历史 DiffCard 堆到会话底部或串到错误会话；本次必须按
  `diffRefs.anchorMessageId` 锚回对应 assistant 消息，缺失锚点才进入 fallback。
- 若 DE/WS 找不到历史 diff 或其已为 accepted/rejected/expired，却在前端默认建成 pending，会让历史终态复活；
  本次必须按 DE 当前状态恢复，缺失 diff 只读标记不可用。
- 若删除 AG 历史会话时一并删除 DE pending diff，会丢失用户尚未裁决的修改建议；本次删除边界必须保留 DE 状态。
- 若活动流式回合中允许删除，排队的 draft/终态保存可能在删除后重建旧记录；本次按钮在 sending=true 时禁用。

## 11. 验证计划

- Rust：迁移版本幂等、agent session/turn/payload 往返、workspace 隔离、状态枚举校验、200MB 软归档。
- 前端 store/service：hydrate metadata、懒加载 turns、draft 重试（含引用 payload）、workspace reset、session meta
  持久化调用；历史 diffRefs 经 DE all-status 查询恢复；deleteAgentSession 调用删除命令后移除目标会话。
- UI：历史工具卡只读且锚回对应消息；历史 DiffCard 不沉底、不串会话；draft 显示未完成并可重试；历史会话标签可重开。
  历史会话行删除按钮确认后调用删除服务，取消确认不调用。
- 命令：`cargo test --manifest-path src-tauri/Cargo.toml ag::turn db`、相关 `npm test -- --run ...`、`./node_modules/.bin/tsc --noEmit`、`npm run governance:generate`、`npm run governance:audit`。
- AI 语义审计：检查 `BR-AG-PERSIST-001` 对应代码仍满足本地、可恢复、不上云、Diff 引用不快照的边界。

## Scope Lock

```json
{
  "task_type": "CODE_CHANGE",
  "scope_id": "AG-PERSIST-HISTORY-01",
  "allowed_files": [
    ".governance/active_scope.json",
    ".governance/issues/AG-PERSIST-HISTORY-01.md",
    "docs/01_modules/AG/A-AG-M-D-01_代理功能需求描述.md",
    "docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md",
    "docs/01_modules/AG/A-AG-M-S-01_代理状态机设计.md",
    "docs/01_modules/AG/A-AG-I-P-01_代理接口协议.md",
    "docs/01_modules/AG/A-AG-X-L-02_代理集成计划.md",
    "docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md",
    "docs/00_core/A-CORE-C-R-01_ADUS.md",
    "src-tauri/src/db/mod.rs",
    "src-tauri/src/ag/turn.rs",
    "src-tauri/src/ag/history.rs",
    "src-tauri/src/ag/mod.rs",
    "src-tauri/src/lib.rs",
    "src/store/agStore.ts",
    "src/store/agStore.test.ts",
    "src/services/agService.ts",
    "src/services/agService.test.ts",
    "src/services/workspaceService.ts",
    "src/services/deService.ts",
    "src/components/ag/AgentPanel.tsx",
    "src/components/ag/AgentPanel.test.tsx",
    "src/components/ag/Composer.tsx",
    "src/components/ag/MessageList.tsx",
    "src/components/ag/MessageList.test.tsx",
    "src/components/ag/ToolCallCard.tsx"
  ],
  "forbidden_files": [
    "ADU.md",
    "ADP.md",
    "AGENTS.md",
    "docs/00_core/A-CORE-C-P-01_APC.md",
    "src-tauri/src/ipc/mod.rs"
  ],
  "confirmation": "YES",
  "scope_reason": "AG-PERSIST-HISTORY-01（人类确认）：实现代理编辑会话标签与回合历史持久化，含全状态恢复、懒加载、工具摘要、Diff 引用与 200MB 软归档。"
}
```
