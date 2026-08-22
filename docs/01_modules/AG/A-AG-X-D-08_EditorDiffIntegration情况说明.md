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

Editor/Diff Integration 是 Binder 文档内容代理的受控边界：模型生成的内容变化不能直接写入 DiskState，
必须先转换为 `ChangeProposal`，由 DE 生成 PendingDiff，再由 DE/ED 受控预览进入 LogicalState。
用户接受只裁决卡片，拒绝在基准有效时精确回滚，保存才把当前 LogicalState 写入 DiskState。

当前等级：`Migration Pending`。既有文档内容 Diff 的基准、预览、裁决和保存链已有自动化证据；
但旧的空基准新建链已被 CreateDocumentBundle 取代，且真实 Tauri/WKWebView 中从 Agent 回合到人工审阅 DiffCard 的 G5 尚未执行。

## 2. 职责、输入与输出

职责：

- 把获授权的 `propose_edit` 参数转换为 DE 拥有的 `ChangeProposal`。
- 既有文档只使用最新 LogicalState 基准；CreateDocumentBundle 只在 WS 已创建真实目标后绑定初始内容 Diff。
- 保证提案生成本身不直接写 DiskState；受控预览可修改 LogicalState，卡片裁决不重复应用内容。
- 将 Diff 引用关联到本轮消息，而不是把 Diff 正文复制进会话历史。
- 对版本过期、内容哈希变化、selection 失效和应用冲突失败关闭。

输入：

- `IntentResultV1` 的目标、操作和约束。
- 已通过 `CapabilityGrantV1` 校验的 `propose_edit` 参数。
- 当前 `AgentEditableSnapshot`、块锚、session version/content hash。
- CreateDocumentBundle 的 operation_id、创建后 DiskState 基准与 ED session 基准。

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
ChangeProposal → DE PendingDiff → ED 受控预览进入 LogicalState → DiffCard
↓
用户 Accept / Reject
↓
Accept：只改卡片生命周期，不重复应用；Reject：基准有效时精确回滚；Save：写入 DiskState
```

新建文档：

```text
用户明确新建文档，系统在确认前生成目标路径、WorkspaceOperationPlan 与初始内容
↓
用户确认 DecisionRequest，WS 重新核验并创建空的真实目标 DiskState
↓
系统打开目标标签，以 operation_id + 创建后空基准投影普通初始内容 Diff
↓
接受只裁决内容卡；三重基线未变时拒绝由 WS 补偿删除本次创建文件，否则卡失效且文件保留
```

## 5. 必须冻结与允许变化

必须冻结：

- AG 和 Tool 不得直接写入 DiskState；内容只能经 DE/ED 受控预览改变 LogicalState。
- 1.0 每个提案只能针对一个明确目标；目标必须属于当前文档、显式声明目标或合法新建目标。
- 既有文档基准来自最新 LogicalState；新建初始内容 Diff 的基准来自 CreateDocumentBundle 创建后真实空目标与 ED session，不得再使用无 DiskState 空基准创建链。
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

1. 修改既有文档生成 Diff 后，受控预览修改 LogicalState 但 DiskState 不变；Accept 不重复应用，Save 才写入。
2. Reject 在基准有效时精确恢复 LogicalState；基准已推进时卡失效，不覆盖后续编辑。
3. CreateDocumentBundle 确认前无磁盘副作用；确认后先建真实目标再投影初始内容 Diff；安全拒绝的三重 guard 与补偿结果可验证。
4. 未声明目标、跨 workspace 路径、过期 selection、session version/hash 变化分别失败关闭。
5. 一个回合不能产出多个越界目标提案；同一提案不能同时修改多个文档。
6. DiffCard 使用 DE 的 diff id/内容，AG 历史只保存 reference；重载会话后仍能正确关联。
7. 接受时发生冲突必须展示 Conflict，不得静默覆盖；保存失败不得显示为已保存。
8. G5 人工验证覆盖真实 App 中提案出现、内容审阅、接受、拒绝、冲突提示和保存结果。

自动化收口：1–7 全通过。产品 G5 收口：再完成第 8 项，并保存真实账号/provider/Tauri 环境证据。

## 8. 下一治理会话入口

必读顺序：A-AG-M-T-01 编辑/Diff 部分 → A-AG-I-P-01 → A-WS-M-T-02 / A-WS-M-S-02 → DE 技术设计 → ED/WS 相关设计 →
`lib.rs` 的 `ag_propose_edit` → `de/generate.rs` → Diff store/apply → 前端 DiffCard 与核心链测试。

候选修改范围：AG 提案适配和集成测试。若改变 `ChangeProposal`、PendingDiff、应用或保存状态，必须建立
AG+DE/ED/WS 跨模块 Scope；不得在 AG 文件内复制或旁路下游契约。

建议会话任务描述：

> 治理 WP-AG10-06 Editor/Diff Integration。核验所有文档内容修改只经 DE/ED 受控预览进入 LogicalState，既有文档
> 使用最新 LogicalState，新建经 CreateDocumentBundle 先创建真实目标再投影初始内容 Diff，目标/版本/冲突失败关闭，会话只保存 Diff reference。若修改，
> 先确认是否跨 AG/DE/ED/WS，并用写入前后快照和真实 App 审阅链证明收口。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-08 | v1.0 | 建立 Editor/Diff 集成的提案边界、目标与基准规则、跨模块风险和独立验证入口 |
| 2026-08-21 | v1.1 | 同步三态罗盘与 CreateDocumentBundle：内容 Diff 预览可受控修改 LogicalState，接受只裁决、保存才写盘；旧空基准新建链改为先创建真实目标、再投影初始内容 Diff。当前等级转为 Migration Pending。 |
