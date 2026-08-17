---
文档编号：AG-X-D-09
文档状态：A
负责模块：AG
文档职责：WP-AG10-07 Session、UI 与 Trace 独立治理情况说明
上游约束：A-AG-X-D-01、A-AG-X-D-03、A-AG-X-D-08、A-AG-X-L-04、A-AG-M-T-01、A-AG-M-S-01、A-AG-I-P-01
直接承接：会话隔离与恢复、Composer 交互、MessageFlow/Tool/Diff 展示、运行时追踪
使用边界：本文说明 agent-edit-1.0 的前端投影、历史持久化和脱敏追踪；不重定义 Execution、DE 或 workspace 状态真值
变更要求：会话归属、历史 payload、active turn、消息片段、终态投影、Diff 引用、恢复或 trace 字段变化时同步更新
---

# WP-AG10-07 · Session、UI 与 Trace 情况说明

## 1. 模块定位与当前裁决

本模块把后端 Runtime 事件投影成用户可理解、可恢复的文档协作体验：每个 workspace/session 的消息、工具
过程、Diff 引用、失败和取消都要保持归属正确，并在重启或切换后恢复。Trace 用于解释发生了什么，但不保存正文。

当前等级：`Integrated（本地）`。会话持久化、五种回合状态、active turn、停止操作、流式消息、工具/Diff
片段和本地脱敏日志已经接线并有自动化测试；持久化 AgentTrace、真实 App 重启恢复和完整 G5 尚未收口。

## 2. 职责、输入与输出

职责：

- 按 workspace/session/turn 隔离状态和历史，维护当前活动会话。
- 把 `AgTurnEvent` 投影为用户消息、助手流式内容、工具卡、DiffCard、错误和取消状态。
- 维护 `activeTurn` 与停止入口，防止旧回合或其他会话事件污染当前 UI。
- 保存和恢复回合 payload；Diff 只保存引用，不复制正文。
- 记录脱敏运行时元数据，为问题定位提供 intent/context/tool/terminal 证据。

输入：

- Execution Kernel 的有序 `AgTurnEvent` 和状态机广播。
- workspace id、session id、turn id、当前文档路径。
- Tool summary、Diff reference、失败和 incomplete reason。
- DE/ED 提供的 Diff 与编辑器真实状态。

输出：

- 本地会话列表、历史回合和草稿恢复结果。
- `activeTurn`、sending、消息 segments 与 Composer 可操作状态。
- MessageFlow 中的文本、工具、Diff 和错误/取消展示。
- 不含文档正文和秘密的 trace/log 元数据。

## 3. 当前实现地图

| 位置 | 当前职责 | 治理注意点 |
|---|---|---|
| `src-tauri/src/ag/history.rs` | `AgentTurnPayload`、session/turn 保存、加载、删除和旧 payload 兼容 | 后端持久化契约 owner |
| `src/services/agService.ts` | start/stop、事件消费、first-terminal、持久化和前后端桥接 | 与 Runtime Profile/Execution 共用，不应并行编辑 |
| `src/store/agStore.ts` | sessions、activeSession、activeTurn、sending 和消息片段投影 | 全局 sending 与逐 session 真值需持续核验 |
| `src/components/ag/Composer.tsx` | 发送、停止、Build 入口隐藏和交互门禁 | UI 不得自造后端成功状态 |
| MessageFlow、消息列表、Tool/Diff 组件 | 按事件顺序展示内容和裁决入口 | Diff 真值来自 DE，不来自历史副本 |
| `src/services/agRuntime.test.ts`、`src/components/ag/AgentUI.test.tsx`、store/history 测试 | 乱序、终态、停止、恢复、展示和隔离断言 | 真实 WKWebView 仍需 G5 |

## 4. 当前工作流程

```text
用户在当前 workspace/session 发送输入
↓
前端创建 turn，设置 activeTurn/sending，并调用 ag_start_turn
↓
后端先保存 draft，随后持续发送 Chunk / Tool / DiffProposed / Done|Error|Cancelled
↓
agService 校验 turn 归属和 first-terminal，agStore 投影为消息 segments 与 UI 状态
↓
终态保存 completed/error/cancelled/gate_rejected；中断 draft 保留 incomplete reason
↓
切换或重启时按 workspace/session 加载历史；DiffCard 通过 diff reference 读取 DE 真值
```

后端持久化允许的状态为 `completed`、`error`、`cancelled`、`gate_rejected`、`draft`。前端不能把 Error、
Cancelled 或 gate rejection 显示成成功，也不能让迟到 Done 覆盖已确定终态。

## 5. 必须冻结与允许变化

必须冻结：

- workspace、session、turn 三层归属；一个回合的事件不能进入其他会话或 workspace。
- 同一回合只有第一个终态有效；Done、Error、Cancelled、GateRejected 互斥。
- Error/Cancelled/GateRejected 不是成功；停止操作必须作用于精确 active turn。
- draft、completed、error、cancelled、gate_rejected 和 incomplete reason 可持久化恢复。
- Diff 只以 `diff_id`、`rel_path` 等 reference 存入 AG 历史，正文和裁决真值由 DE 持有。
- UI 是 Runtime/DE/ED 状态的投影，不能凭本地组件状态伪造成功、应用或保存。
- trace 禁止包含文档正文、显式引用正文、密钥、认证头和完整工具敏感结果。

允许持续变化：

- 消息布局、工具卡和 DiffCard 的表现、加载反馈与可访问性。
- Trace 后端、检索和调试界面；字段新增必须有隐私分类和版本。
- 历史分页、压缩和保留策略，但必须保持会话隔离和旧 payload 兼容。

## 6. 已知风险与待治理点

- 当前 trace 主要是结构化 `eprintln` 日志，不是可查询、可关联、可持久化的 AgentTrace 基础设施。
- 计划中的遥测指标、用户反馈与质量评测尚未完整实现，不应把普通日志宣称为 Evaluation 闭环。
- 前端 `sending` 仍有全局投影，而后端 active turn 按 session 管理；多会话切换和后台回合需持续验证。
- 新 Runtime 行为尚未在真实 Tauri/WKWebView 中完成重启、停止、切换会话和 Diff 恢复的 G5。
- 尚无单一 harness 从 `ag_start_turn` 覆盖到持久化、UI segments 和 DiffCard。

## 7. 独立验证与收口标准

至少验证：

1. 两个 session、两个 workspace 和交错 turn 事件不会串消息、active state、工具卡或 DiffCard。
2. Done/Error/Cancelled 任意乱序组合只有第一个终态生效，迟到事件不改变终态。
3. Stop 只取消精确 active turn；取消后 Composer 恢复可用，历史记录为 cancelled 而非 completed。
4. draft、五种状态、失败信息、工具摘要和 Diff reference 在重载后恢复；旧版 payload 可读取。
5. 历史中不保存 Diff 正文；DE 内容变化或裁决后，重新打开仍显示 DE 当前真值。
6. 切换 workspace/session 时历史、active turn 和当前文档关联正确，后台事件不污染前台。
7. trace 捕获 profile、turn/session、intent、context hash、grant、terminal 等元数据，但扫描不到正文和凭证。
8. 真实 App G5 覆盖流式输出、停止、错误、会话切换、重启恢复、ToolCard 和 DiffCard。

自动化收口：1–7 全通过。产品 G5 收口：再完成第 8 项，并保留环境、账号、provider 和观察结果证据。

## 8. 下一治理会话入口

必读顺序：A-AG-M-S-01 → A-AG-I-P-01 → A-AG-M-T-01 Session/UI/Trace 部分 → `ag/history.rs` →
`agService.ts` → `agStore.ts` → Composer/MessageFlow/Tool/Diff 组件 → 前后端相关测试。

候选修改范围：历史契约、前端服务/Store/展示和 trace。若改变 `AgTurnEvent` 或 Diff 状态，应回到 Execution 或
DE owner 扩展 Scope；不能在 UI 中用推断状态修补后端契约。

建议会话任务描述：

> 治理 WP-AG10-07 Session、UI 与 Trace。核验 workspace/session/turn 隔离、first-terminal、停止、五状态
> 持久化恢复、Diff reference 单一真值和 trace 脱敏。若修改，建立独立 Scope，并用多会话乱序、重启恢复、
> 隐私扫描及真实 Tauri/WKWebView 操作证明收口。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-08 | v1.0 | 建立 Session/UI/Trace 的归属、持久化、终态投影、隐私边界、风险与独立验证入口 |
