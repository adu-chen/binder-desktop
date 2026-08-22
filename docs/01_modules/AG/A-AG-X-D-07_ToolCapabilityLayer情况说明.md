---
文档编号：AG-X-D-07
文档状态：A
负责模块：AG
文档职责：WP-AG10-05 Tool Capability Layer 独立治理情况说明
上游约束：A-AG-X-D-01、A-AG-X-D-04、A-AG-X-D-06、A-AG-X-L-04、A-AG-M-T-01、A-AG-I-P-01
直接承接：Execution Kernel 的工具循环、Editor/Diff Integration 的修改建议入口
使用边界：本文说明 agent-edit-1.0 的逐回合工具授权、校验、执行与反馈边界；工具已注册不等于当前回合获授权
变更要求：工具注册、权限级别、CapabilityGrant、目标范围、参数约束、工具反馈或错误恢复语义变化时同步更新
---

# WP-AG10-05 · Tool Capability Layer 情况说明

## 1. 模块定位与当前裁决

Tool Capability Layer 把“系统具有某种工具实现”与“本轮允许模型使用什么”分开。注册表描述可能能力，
Runtime Profile 与 Intent 共同产生逐回合 `CapabilityGrantV1`；任何调用都必须再次校验工具、回合、目标和参数。

当前等级：`Migration Pending`。1.0 的读工具和内容 `propose_edit` 已进入真实工具循环，未授权工具会从模型可见清单
过滤并在执行端再次拒绝；但 WorkspaceOperationPlan/DecisionRequest 资源链尚未按 A-WS-M-T-02 / M-S-02 实现，
不得以旧的低层文件命令或资源 DiffCard 证据声称本工作包已收口。

## 2. 职责、输入与输出

职责：

- 维护工具注册信息与权限类别。
- 按 Runtime Profile、`IntentResultV1` 和显式读取目标生成最小授权。
- 只把本轮允许的工具定义暴露给模型。
- 执行前校验 grant 归属、工具名、目标范围、参数和写入方式。
- 统一工具反馈、错误回注、续调和最大轮数失败关闭。

输入：

- Runtime Profile、turn id、`IntentResultV1`、显式读取目标。
- 模型返回的工具名称、参数与当前回合 grant。
- Active Workspace 和 `AgentReadContext`。

输出：

- `CapabilityGrantV1`：profile、intent/turn、allowed tools、target/readable scope、参数约束。
- 过滤后的模型工具 definitions。
- Allow/Reject 决策及结构化原因。
- 只读 ToolResult、经 `ChangeProposal → DE` 的内容修改建议，或不发生磁盘副作用的 WorkspaceOperationPlan + DecisionRequest。

## 3. agent-edit-1.0 最小授权矩阵

| Intent | 获授权工具 |
|---|---|
| Answer / Clarify | 无 |
| Search | `list_docs`、`search_text`、`read_doc` |
| Inspect / Analyze | `read_doc`、`list_blocks` |
| Edit | `read_doc`、`list_blocks`、`propose_edit`、`edit_blocked` |
| CreateDocument | 按需读取工具、`plan_workspace_operation`、`edit_blocked` |
| OrganizeWorkspace | `list_docs`、按需读取工具、`plan_workspace_operation`、`edit_blocked` |

`propose_edit` 是内容 WriteIntent，不是工作区资源授权工具。`plan_workspace_operation` 只冻结计划并生成
DecisionRequest，不执行磁盘操作；`execute_workspace_operation` 不对模型授权，只能在真实确认回执后由 TaskRun 调用。
新建文档使用 CreateDocumentBundle：初始内容在确认前生成并绑定，确认后先创建真实磁盘目标，再由系统投影普通内容 Diff。

## 4. 当前实现地图

| 位置 | 当前职责 | 治理注意点 |
|---|---|---|
| `src-tauri/src/ag/tool_registry.rs` | `CapabilityGrantV1`、grant 解析、模型定义过滤、目标/参数校验 | 本模块的主要契约 owner |
| `src-tauri/src/ag/tool.rs` | 通用 ToolDecision、工具循环状态和轮数约束 | 不应被业务特例绕过 |
| `src-tauri/src/ag/tool_exec.rs` | 只读工具实际执行与统一快照读取 | `propose_edit` 不在只读执行器中 |
| `src-tauri/src/lib.rs` | 真实模型工具循环、校验、执行、结果回注和提案专用路径 | 与 Execution/Prompt/Diff 共用文件，不能并行修改 |
| `src-tauri/src/ag/tool_registry.rs`、`tool.rs`、`tool_exec.rs` 内测试及 AG 主链测试 | grant、越权、读取、失败与循环断言 | 必须包含 Intent×Tool 负向矩阵 |

## 5. 当前工作流程

```text
Runtime Profile + IntentResult + 显式目标
↓
resolve_capability_grant 生成逐回合最小授权
↓
过滤工具 definitions，Prompt Runtime 只向模型公开允许项
↓
模型提出工具调用
↓
校验 profile/turn → 注册状态 → grant → readable/target scope → 参数约束
↓
只读工具经 AgentReadContext 执行；内容 propose_edit 经 ChangeProposal → DE；
资源意图经 plan_workspace_operation → WorkspaceOperationPlan → DecisionRequest；拒绝原因回注模型
↓
普通工具续调或输出最终结果；DecisionRequest 确认则 TaskRun 调用 WS 重验与执行；达到上限、取消或错误则失败关闭
```

## 6. 必须冻结与允许变化

必须冻结：

- 工具注册、Runtime Profile 授权、Intent 授权、目标范围和参数约束必须全部通过，任一失败即拒绝。
- 模型只能看到本轮 grant 中的工具；执行端仍要二次校验，不能信任模型可见清单。
- grant 必须绑定 profile、intent id 和 turn id；跨回合或 profile 不匹配必须拒绝。
- 读工具只能读取 Active Workspace；打开文档优先使用最新 LogicalState。
- 文档内容修改只能产出 `ChangeProposal` 交给 DE，禁止工具直接写 LogicalState 或磁盘。
- 新建提案以空基准开始；未声明既有目标、表格/图片高级操作和越权参数失败关闭。
- 拒绝不能触发自动扩权；错误反馈必须结构化、可理解且不泄露敏感信息。

允许持续变化：

- 新增工具实现和注册元数据，但默认不进入 agent-edit-1.0 grant。
- Intent 到最小工具集的映射；每次变化都必须更新能力矩阵和负向测试。
- 错误恢复次数、工具结果压缩和观测字段，但不得绕过授权或 Diff 边界。

## 7. 已知风险与待治理点

- `CapabilityGrantV1` 通过 profile、intent id、turn id 绑定，但没有独立签名或时间戳；当前生命周期依赖回合状态。
- Search 的 `workspace_search` readable scope 会允许该搜索回合读取 workspace 中命中的文档，范围语义需持续审计。
- 工具循环和 `propose_edit` 特殊路径仍集中在 `src-tauri/src/lib.rs`，模块物理边界不完整。
- 全局注册表含未来工具，静态 Prompt 或文档若直接枚举注册表，可能造成“看似可用、实际未授权”的漂移。
- 尚无从 `ag_start_turn` 到多轮工具恢复的单一确定性集成 harness；真实 provider 工具调用也未完成 G5。

## 8. 独立验证与收口标准

至少建立以下测试矩阵：

1. 每一种 Intent 的 grant 精确等于 1.0 矩阵，不多一个工具；未知 profile 得到空授权。
2. 未注册、已注册但未授权、错误 turn、错误 profile、目标越界和非法参数分别返回可区分的 Reject。
3. 模型 definitions 与 grant 完全一致；被过滤工具即使由伪造模型响应请求，也在执行端被拒绝。
4. 当前打开文档读取命中 LogicalState；workspace 越界、过期 selection 和未声明目标失败关闭。
5. `propose_edit` 不直接改变 DiskState，只产生内容 Diff；资源意图不能进入该工具。CreateDocumentBundle 只有在真实目标创建后才投影初始内容 Diff。
6. 工具失败会作为数据反馈给模型，不能自动扩权；达到最大轮数、取消或 gate 拒绝产生唯一失败终态。
7. 工具结果包含提示注入文本时仍以 DATA-ONLY 回注，不改变 Runtime Control。

收口状态定义：Intent×Tool×Target×Parameter 的正负矩阵全通过，且以写入前后快照证明所有文档内容修改都只
经过 DE。只验证“工具能调用”不足以收口。

## 9. 下一治理会话入口

必读顺序：A-AG-M-T-01 工具体系 → A-AG-I-P-01 → A-WS-M-T-02 / A-WS-M-S-02 → A-AG-X-D-04 → `tool_registry.rs` → `tool.rs` →
`tool_exec.rs` → `lib.rs` 工具循环与提案路径 → DE 修改建议契约。

候选修改范围：工具注册/授权/校验/执行和对应测试。若新增 1.0 可见工具、扩大可读范围或改变
`ChangeProposal`，属于能力范围或跨模块契约变化，不能在本模块会话中隐式完成。

建议会话任务描述：

> 治理 WP-AG10-05 Tool Capability Layer。以 agent-edit-1.0 最小权限为准，核验注册不等于授权、模型清单
> 与执行端双重校验、目标和参数范围、LogicalState 读取、错误恢复及所有写意图经 DE。若修改，建立独立
> Scope，并用 Intent×Tool×Target×Parameter 正负矩阵证明收口。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-08 | v1.0 | 建立 Tool Capability 的最小授权矩阵、双重校验、写入边界、风险与独立验证入口 |
| 2026-08-21 | v1.1 | WorkspaceOperation 基线同步：当前等级改为 Migration Pending；新增 CreateDocument/OrganizeWorkspace 计划工具授权，执行工具仅由真实确认后的 TaskRun 调用；新建改用 CreateDocumentBundle。 |
