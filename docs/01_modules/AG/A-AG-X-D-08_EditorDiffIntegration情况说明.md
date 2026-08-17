---
文档编号：AG-X-D-08
文档状态：A
负责模块：AG
文档职责：WP-AG10-06 Editor/Diff Integration 独立治理情况说明
上游约束：A-AG-X-D-01、A-AG-X-D-03、A-AG-X-D-04、A-AG-X-D-05、A-AG-X-D-07、A-AG-X-L-04、A-AG-M-T-01、A-AG-I-P-01
直接承接：DE 修改建议与裁决链、ED LogicalState、WS 保存边界、Session/UI 的 DiffCard 投影
使用边界：本文说明 AG 产出文档修改建议到用户裁决的集成边界；DE/ED/WS 契约仍由各自模块拥有，本文不重定义其真值
变更要求：ChangeProposal、PendingDiff、目标/基准、接受拒绝冲突、保存或 DiffCard 关联语义变化时同步更新
---

# WP-AG10-06 · Editor/Diff Integration 情况说明

## 1. 模块定位与当前裁决

Editor/Diff Integration 是 Binder 文档代理安全性的最终执行边界：模型生成的内容变化不能直接写进编辑器或
磁盘，必须先转换为 `ChangeProposal`，由 DE 生成 PendingDiff，交给用户接受或拒绝，再由既有 ED/WS 链处理。

当前等级：`Integrated（自动化边界）`。既有文档编辑、新建空基准、目标守卫、Diff 生成和核心接受/保存链
已有自动化验证；真实 Tauri/WKWebView 中从 Agent 回合到人工审阅 DiffCard 的 G5 尚未执行。

## 2. 职责、输入与输出

职责：

- 把获授权的 `propose_edit` 参数转换为 DE 拥有的 `ChangeProposal`。
- 区分既有文档与新建文档的基准和目标守卫。
- 保证提案前不改 LogicalState/磁盘，用户裁决后才进入应用链。
- 将 Diff 引用关联到本轮消息，而不是把 Diff 正文复制进会话历史。
- 对版本过期、内容哈希变化、selection 失效和应用冲突失败关闭。

输入：

- `IntentResultV1` 的目标、操作和约束。
- 已通过 `CapabilityGrantV1` 校验的 `propose_edit` 参数。
- 当前 `AgentEditableSnapshot`、块锚、session version/content hash。
- 新建目标存在性与 Active Workspace 约束。

输出：

- DE `ChangeProposal`。
- PendingDiff / DiffProposed 事件与 `diff_id`、`rel_path` 引用。
- 用户 Accept/Reject 后的应用、冲突或保存结果。

## 3. 当前实现地图

| 位置 | 当前职责 | 治理注意点 |
|---|---|---|
| `src-tauri/src/lib.rs` | `ensure_new_document_target`、`ag_propose_edit`、主回合提案接线 | AG 只组装和守卫，不拥有 Diff schema |
| `src-tauri/src/de/generate.rs` | `ChangeProposal`、target 校验、PendingDiff 生成 | DE 是修改建议载荷的单一 owner |
| `src-tauri/src/de/store.rs`、WS/ED 应用边界 | Diff 持久化、接受/拒绝、冲突和保存 | 跨模块变更须扩大 Scope |
| `src/store/agStore.ts`、消息流与 DiffCard 组件 | 保存 diff reference、展示用户裁决入口 | AG 历史不得复制 Diff 正文 |
| `src-tauri/tests/core_chain.rs` 及 AG/DE/ED 相关测试 | 新建空基准、修改、冲突和保存链 | G5 仍需真实 UI 人工验证 |

## 4. 当前工作流程

既有文档编辑：

```text
用户要求修改当前/声明文档
↓
Intent 解析目标，Context 从最新 LogicalState 读取，Tool grant 授权 propose_edit
↓
模型给出 path、operation、content/锚点
↓
目标、参数、读取前置、session version/content hash 校验
↓
ChangeProposal → DE PendingDiff → DiffCard
↓
用户 Accept / Reject
↓
Accept：冲突校验后应用到 LogicalState，再由用户/既有流程保存；Reject：文档不变
```

新建文档：

```text
用户明确新建文档与目标路径
↓
目标必须不存在，operation 必须为 replace_document
↓
ChangeProposal(base=None / empty baseline) → PendingDiff
↓
用户审核后接受或拒绝；接受前不得创建目标文件
```

## 5. 必须冻结与允许变化

必须冻结：

- 用户接受前，AG 和 Tool 均不得直接改变 LogicalState 或磁盘文档内容。
- 1.0 每个提案只能针对一个明确目标；目标必须属于当前文档、显式声明目标或合法新建目标。
- 既有文档基准来自最新 LogicalState；新建文档基准必须为空且目标必须不存在。
- 同一回合的修改目标和授权范围一致；未读长文档、过期 selection、版本/hash 冲突失败关闭。
- `ChangeProposal` / PendingDiff schema 由 DE 单一拥有，AG 不复制定义。
- Accept、Reject、Conflict、Save 是不同状态；生成 Diff 不等于已修改或已保存。
- 会话历史仅保存 Diff reference，Diff 真值从 DE 读取。

允许持续变化：

- Diff 展示样式、分块审阅和操作反馈，但不得改变接受前不写入的原则。
- 写作、结构、布局智能产生提案的方式；它们必须输出同一受控修改建议契约。
- 未来表格、图片、版式提案 schema；在冻结新契约前不应塞入 1.0 文本提案参数。

## 6. 已知风险与待治理点

- 尚无一个确定性 harness 覆盖 `ag_start_turn → gateway/tool → DE → 前端 DiffCard` 全链路。
- 真实 Tauri/WKWebView 中 DiffCard 的到达、滚动定位、接受/拒绝及错误反馈尚未完成 G5。
- 自动化能证明边界和状态正确，不能证明模型生成内容的业务质量；内容质量需单独评测。
- 表格、图片和 BDocx 高级编辑底层能力可能已存在，但 1.0 grant 明确排除，不能由集成会话顺带开放。
- AG、DE、ED、WS 的状态跨模块，任何 schema 或应用语义变化都不是 AG 单模块修改。

## 7. 独立验证与收口标准

至少验证：

1. 修改既有文档生成 Diff 前后，LogicalState 和磁盘内容均不变；Accept 后才发生预期变化。
2. Reject 后 LogicalState、磁盘和 dirty/save 状态符合既有 ED/WS 契约。
3. 新建目标存在时拒绝；合法新建以空基准生成 Diff，接受前文件不存在。
4. 未声明目标、跨 workspace 路径、过期 selection、session version/hash 变化分别失败关闭。
5. 一个回合不能产出多个越界目标提案；同一提案不能同时修改多个文档。
6. DiffCard 使用 DE 的 diff id/内容，AG 历史只保存 reference；重载会话后仍能正确关联。
7. 接受时发生冲突必须展示 Conflict，不得静默覆盖；保存失败不得显示为已保存。
8. G5 人工验证覆盖真实 App 中提案出现、内容审阅、接受、拒绝、冲突提示和保存结果。

自动化收口：1–7 全通过。产品 G5 收口：再完成第 8 项，并保存真实账号/provider/Tauri 环境证据。

## 8. 下一治理会话入口

必读顺序：A-AG-M-T-01 编辑/Diff 部分 → A-AG-I-P-01 → DE 技术设计 → ED/WS 相关设计 →
`lib.rs` 的 `ag_propose_edit` → `de/generate.rs` → Diff store/apply → 前端 DiffCard 与核心链测试。

候选修改范围：AG 提案适配和集成测试。若改变 `ChangeProposal`、PendingDiff、应用或保存状态，必须建立
AG+DE/ED/WS 跨模块 Scope；不得在 AG 文件内复制或旁路下游契约。

建议会话任务描述：

> 治理 WP-AG10-06 Editor/Diff Integration。核验所有文档修改在用户接受前只形成 DE 修改建议，既有文档
> 使用最新 LogicalState、新建使用空基准，目标/版本/冲突失败关闭，会话只保存 Diff reference。若修改，
> 先确认是否跨 AG/DE/ED/WS，并用写入前后快照和真实 App 审阅链证明收口。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-08 | v1.0 | 建立 Editor/Diff 集成的提案边界、目标与基准规则、跨模块风险和独立验证入口 |
