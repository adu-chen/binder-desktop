---
文档编号：AG-X-D-02
文档状态：R
负责模块：AG
文档职责：WP-AG10-00 历史治理情况参考
上游约束：R-AG-X-D-01、A-AG-X-L-04、A-AG-I-P-01、A-AG-M-D-01
直接承接：无（仅供历史审计与问题定位参考）
使用边界：L4 reference-only；不定义需求、规则、接口、状态或实现真值。Runtime Profile 以 A-AG-M-D-01、A-AG-M-T-01、A-AG-M-S-01 与 A-AG-I-P-01 为准
变更要求：不再随实现同步；发现差异时回到对应 Active 控制真源处理
---

# WP-AG10-00 · Runtime Profile 与契约入口情况说明（历史快照）

> **Reference only**：本文保留 2026-08 阶段治理上下文，不再作为当前能力边界或实现状态的控制源。

> [!IMPORTANT]
> 以下全部内容是 2026-08 时点快照。“当前”“必须”“不得”“冻结”“验证”和“会话入口”等用语仅记录当时的审计口径，不产生现行规范、Scope、任务授权或验收证据。
> 本文不含正式 `REQ` / `REQ_MAP` / `RULE` / `CHAIN` / `CONSTRAINT` / `TERM` 注册块；其中的标识符只是 Active 真源的历史引用指针，代码、测试和计划不得映射到本文。
> 需要当前结论时，必须回到文档头列出的 Active 需求、设计、状态机、协议与计划，并重新核验代码和测试。

## 1. 历史模块定位与快照时裁决

该模块回答“当前启动的是哪一种 Agent，以及它最多可以拥有哪些能力”。它是 Binder 1.0 防止未来能力
意外进入发布主链的第一道物理边界，不负责判断本轮意图，也不负责具体工具授权。

快照时等级：`Integrated`。当时唯一开放主回合 profile 为 `agent-edit-1.0`；未知 profile 后端默认拒绝，
前端只呈现 edit，Build 入口物理关闭。

## 2. 职责、输入与输出

| 项目 | 内容 |
|---|---|
| 输入 | `ag_start_turn.runtimeProfile`、发布能力矩阵、前端模式状态 |
| 输出 | 经校验的 Runtime Profile、1.0 feature 边界、后续 Intent/Context/Tool 的上限 |
| 负责 | profile 标识、入口校验、默认拒绝、前端入口一致性 |
| 不负责 | 本轮 IntentResult、上下文选择、具体 allowed_tools、模型执行、Diff 应用 |

## 3. 快照时实现地图

- `src-tauri/src/ag/turn.rs`：`AGENT_EDIT_1_0_PROFILE`、`ensure_runtime_profile`。
- `src-tauri/src/lib.rs`：`ag_start_turn` 接收/默认/校验 `runtime_profile`，之后才注册和运行回合。
- `src/services/agService.ts`：`AGENT_EDIT_RUNTIME_PROFILE`，调用 Tauri 时显式透传。
- `src/components/ag/Composer.tsx`：只显示 edit；持久化 build 状态会被收敛回 edit。
- `src/services/agRuntime.test.ts`、`src/components/ag/AgentUI.test.tsx`：协议透传和入口关闭证据。

## 4. 快照时工作流程

前端 Composer
↓ 固定选择 `agentEdit` 模型能力
`startTurn` 传 `runtimeProfile=agent-edit-1.0`
↓
后端 `ensure_runtime_profile`
↓ 通过
建立 AgentTurn；后续模块只能在该 profile 上继续收窄

未知 profile 在回合注册、门禁、provider 和工具之前返回 `AG_RUNTIME_PROFILE_UNSUPPORTED`。

## 5. 历史冻结记录与可变项（不产生现行规则）

当时记录为冻结的事项：

- 1.0 主入口字符串为 `agent-edit-1.0`，前后端语义一致。
- 未知 profile 默认拒绝，不能回退为“尽量运行”。
- Build/CBT、结构工具、自动 AST/KB/TPL、开放式规划不属于该 profile。
- Profile 只规定能力上限；本轮能力仍须由 IntentResult 和 CapabilityGrant 继续收窄。

当时记录为可变的事项：错误文案、前端静态标签、未来 profile 的独立提案。新增 profile 当时被视为非模块内部小改，要求先
完成需求/设计/接口/feature matrix，再评估全部消费者。

## 6. 快照时验证记录（不得作为当前证据）

- `ensure_runtime_profile(agent-edit-1.0)` 通过，`build-1.0` 被拒绝。
- Service 调用参数包含 `runtimeProfile`。
- Composer 不呈现 Build 选项，模型解析固定使用 `agentEdit`。
- 当前公共基线：AG 前端 92、AG Rust 136、全量前端 552、Rust 295+5+4 通过。

当时的独立验证建议：

1. 已知 profile 通过，未知/空白/拼写漂移全部拒绝。
2. 拒绝发生在 Gate、provider、工具和 proposal 之前。
3. 前端不能构造或恢复出可发送的 Build 模式。
4. profile 未授权能力不会仅因注册表存在而暴露给模型。

## 7. 快照时风险记录（需重新核验）

- profile 字符串在 Rust 与 TypeScript 各有常量，当前靠契约测试防漂移，尚无生成式共享 schema。
- 后端为兼容旧调用把缺失 profile 默认成 `agent-edit-1.0`；未来是否改为必填属于接口兼容裁决。
- 静态系统提示词仍描述部分长期 BDocx 能力，但 Runtime grant 已关闭相关工具；治理时应继续检查“声明能力”
  与“实际授权能力”是否一致。

## 8. 历史会话入口（已关闭）

> 以下阅读顺序、候选范围和任务描述只保留历史，不得直接复用。当前工作必须从 Active 真源重新立项。

历史阅读顺序：R-AG-X-D-01 → 本文；当前工作必须回到 A-AG-M-D-01 / M-T-01 / M-S-01 / I-P-01 / X-L-04 →
`turn.rs` → `agService.ts` → `Composer.tsx` → `ag_start_turn` profile 入口和测试。

当时的候选修改范围不是预授权 Scope：`turn.rs`、`lib.rs` 的 profile 入口、`agService.ts`、`Composer.tsx` 及其测试。
如需修改 CapabilityGrant、Prompt 或 Build/CBT，应停止并按影响扩展 Scope，而不是在本模块顺手修改。

历史任务描述（不得直接复用）：

> 治理 WP-AG10-00 Runtime Profile 与契约入口。先核验本文与当前代码一致性，检查 profile 是否真正作为
> 1.0 能力上限且未知值失败关闭；只报告本模块问题。若需要修改，先建立独立 Issue Trace/Scope Lock，
> 保持 `agent-edit-1.0` 的发布边界，不引入 Build/CBT 或通用 Agent 能力。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-08 | v1.0 | 建立 Runtime Profile 当前实现、冻结边界、验证条件、风险与独立治理会话入口 |
| 2026-08-31 | v1.1 | 文档真源收敛：全文定性为非规范时点快照，规则 ID 仅作引用指针；历史化冻结、验证、风险和会话入口。 |
