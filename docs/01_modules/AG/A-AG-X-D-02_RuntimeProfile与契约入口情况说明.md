---
文档编号：AG-X-D-02
文档状态：A
负责模块：AG
文档职责：WP-AG10-00 独立治理说明
上游约束：A-AG-X-D-01、A-AG-X-L-04、A-AG-X-L-03、A-AG-I-P-01、A-AG-M-D-01
直接承接：Execution、Intent、Context、Prompt、Tool、Diff、Session/UI 各模块的能力边界
使用边界：本文描述 Runtime Profile 当前实现和治理入口，不授权新增 profile，不重定义 Build/CBT 或模型目录
变更要求：profile 标识、默认策略或能力矩阵变化时同步前后端、接口协议、开发计划和全部直接消费者测试
---

# WP-AG10-00 · Runtime Profile 与契约入口情况说明

## 1. 模块定位与当前裁决

该模块回答“当前启动的是哪一种 Agent，以及它最多可以拥有哪些能力”。它是 Binder 1.0 防止未来能力
意外进入发布主链的第一道物理边界，不负责判断本轮意图，也不负责具体工具授权。

当前等级：`Integrated`。当前唯一开放主回合 profile 为 `agent-edit-1.0`；未知 profile 后端默认拒绝，
前端只呈现 edit，Build 入口物理关闭。

## 2. 职责、输入与输出

| 项目 | 内容 |
|---|---|
| 输入 | `ag_start_turn.runtimeProfile`、发布能力矩阵、前端模式状态 |
| 输出 | 经校验的 Runtime Profile、1.0 feature 边界、后续 Intent/Context/Tool 的上限 |
| 负责 | profile 标识、入口校验、默认拒绝、前端入口一致性 |
| 不负责 | 本轮 IntentResult、上下文选择、具体 allowed_tools、模型执行、Diff 应用 |

## 3. 当前实现地图

- `src-tauri/src/ag/turn.rs`：`AGENT_EDIT_1_0_PROFILE`、`ensure_runtime_profile`。
- `src-tauri/src/lib.rs`：`ag_start_turn` 接收/默认/校验 `runtime_profile`，之后才注册和运行回合。
- `src/services/agService.ts`：`AGENT_EDIT_RUNTIME_PROFILE`，调用 Tauri 时显式透传。
- `src/components/ag/Composer.tsx`：只显示 edit；持久化 build 状态会被收敛回 edit。
- `src/services/agRuntime.test.ts`、`src/components/ag/AgentUI.test.tsx`：协议透传和入口关闭证据。

## 4. 当前工作流程

前端 Composer
↓ 固定选择 `agentEdit` 模型能力
`startTurn` 传 `runtimeProfile=agent-edit-1.0`
↓
后端 `ensure_runtime_profile`
↓ 通过
建立 AgentTurn；后续模块只能在该 profile 上继续收窄

未知 profile 在回合注册、门禁、provider 和工具之前返回 `AG_RUNTIME_PROFILE_UNSUPPORTED`。

## 5. 当前冻结项与允许变化项

必须冻结：

- 1.0 主入口字符串为 `agent-edit-1.0`，前后端语义一致。
- 未知 profile 默认拒绝，不能回退为“尽量运行”。
- Build/CBT、结构工具、自动 AST/KB/TPL、开放式规划不属于该 profile。
- Profile 只规定能力上限；本轮能力仍须由 IntentResult 和 CapabilityGrant 继续收窄。

允许持续变化：错误文案、前端静态标签、未来 profile 的独立提案。新增 profile 不是本模块内部小改，必须先
完成需求/设计/接口/feature matrix，再评估全部消费者。

## 6. 已验证事实

- `ensure_runtime_profile(agent-edit-1.0)` 通过，`build-1.0` 被拒绝。
- Service 调用参数包含 `runtimeProfile`。
- Composer 不呈现 Build 选项，模型解析固定使用 `agentEdit`。
- 当前公共基线：AG 前端 92、AG Rust 136、全量前端 552、Rust 295+5+4 通过。

独立验证条件：

1. 已知 profile 通过，未知/空白/拼写漂移全部拒绝。
2. 拒绝发生在 Gate、provider、工具和 proposal 之前。
3. 前端不能构造或恢复出可发送的 Build 模式。
4. profile 未授权能力不会仅因注册表存在而暴露给模型。

## 7. 已知风险与待治理点

- profile 字符串在 Rust 与 TypeScript 各有常量，当前靠契约测试防漂移，尚无生成式共享 schema。
- 后端为兼容旧调用把缺失 profile 默认成 `agent-edit-1.0`；未来是否改为必填属于接口兼容裁决。
- 静态系统提示词仍描述部分长期 BDocx 能力，但 Runtime grant 已关闭相关工具；治理时应继续检查“声明能力”
  与“实际授权能力”是否一致。

## 8. 下一治理会话入口

必读顺序：AGENTS/ADP → A-AG-X-D-01 → 本文 → A-AG-I-P-01 → A-AG-X-L-03/04 →
`turn.rs` → `agService.ts` → `Composer.tsx` → `ag_start_turn` profile 入口和测试。

候选修改范围不是预授权 Scope：`turn.rs`、`lib.rs` 的 profile 入口、`agService.ts`、`Composer.tsx` 及其测试。
如需修改 CapabilityGrant、Prompt 或 Build/CBT，应停止并按影响扩展 Scope，而不是在本模块顺手修改。

建议会话任务描述：

> 治理 WP-AG10-00 Runtime Profile 与契约入口。先核验本文与当前代码一致性，检查 profile 是否真正作为
> 1.0 能力上限且未知值失败关闭；只报告本模块问题。若需要修改，先建立独立 Issue Trace/Scope Lock，
> 保持 `agent-edit-1.0` 的发布边界，不引入 Build/CBT 或通用 Agent 能力。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-08 | v1.0 | 建立 Runtime Profile 当前实现、冻结边界、验证条件、风险与独立治理会话入口 |
