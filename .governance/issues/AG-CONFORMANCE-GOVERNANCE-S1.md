# Issue Trace — AG-CONFORMANCE-GOVERNANCE-S1：Agent 需求与设计语义唯一化治理

文档状态：A
任务类型：GOVERNANCE_CHANGE
治理阶段：DESIGN
任务状态：READY
创建时间：2026-08-27
上游基线：REQ-AG-001/007/008/015；BR-AG-DATA-003/004、BR-AG-STATE-001/002、BR-SYS-STATE-007/008；SM-AG-01/04；人类要求按阶段治理并对风险与取舍低阈值确认。

## 1. 用户可见症状

现有规则链结构审计通过，但同一 active 需求或设计章节中仍并存新旧语义：澄清既被写成 Completed 又被写成 AwaitingClarification；TaskRun 的 WaitingConfirm 既由 Proposal 触发又被修正为仅由 DecisionRequest 触发；正式最小授权规则与 general 全量工具实现登记并存；新建文档存在无物理文件预览与先创建 DiskState 再投影初始内容 Diff 两种需求表述。

## 2. 入口点

- `A-AG-M-D-01_代理功能需求描述.md`：REQ-AG-001/007/008/015。
- `A-AG-M-S-01_代理状态机设计.md`：SM-AG-01/04。
- `A-AG-M-T-01_代理技术设计.md`：BR-AG-DATA-003、BR-AG-STATE-001/002 与 RuntimeProfile/CapabilityGrant/TaskRun 契约。
- `A-AG-M-T-02_提示词架构设计.md`：IntentResult、PromptPackage 与历史实现登记。

## 3. 相关规则

- BR-AG-DATA-003：AgentTurn 目标集合与内容 Proposal 边界。
- BR-AG-DATA-004：IntentResult 与当前轮任务边界。
- BR-AG-STATE-001：AgentTurn 生命周期。
- BR-AG-STATE-002：工具循环与本轮最小授权。
- BR-SYS-STATE-007：RuntimeProfile、IntentResult、CapabilityGrant 与澄清非终态。
- BR-SYS-STATE-008：TaskRun/TaskPlan 受限编排。

## 4. 相关链路

AG-TURN、AG-TOOL；本阶段不修改 AG-ASSIST 与 AG-MODEL-CATALOG 语义。

## 5. 生产者块

REQ-AG-001/007/008/015、SM-AG-01/04、AG 技术设计中的 RULE 与运行时契约章节。

## 6. 消费者块

后续生产入口映射、IntentResult/CapabilityGrant/TaskRun 实现、集成测试和 ADUS 链路视图。

## 7. 路径上的空规则块

不新增平行 RULE。现有 RULE 足以承接需求；本阶段处理的是语义冲突和权威正文边界。

## 8. boundary 假设

- 需求验收条件优先于历史实现事实。
- 同一 active RULE 或 STATE_MACHINE 只能有一个 normative 语义。
- 历史实现记录可保留，但不得继续被解释为当前设计授权。
- ADUS 是派生索引，不作为第二规则源。

## 9. 运行时假设

本阶段不修改运行时。设计收敛后，当前实现中的偏差将在独立 CODE_CHANGE Scope 中修复，不以现有代码反推本阶段设计。

## 10. 疑似不变量破坏

- blocking ambiguity 同时映射 Completed 与 AwaitingClarification。
- WaitingConfirm 同时存在 Proposal 与 DecisionRequest 两种来源。
- answer/analyze 最小授权与 general 全量工具登记冲突。
- 新建文档需求同时存在空基准预览和先建 DiskState 两条路径。
- TaskRun 是否适用于简单 AgentTurn 尚未形成唯一边界。

## 11. 验证计划

- 对每项冲突执行需求→设计语义比对，记录唯一权威结论。
- 运行 `npm run governance:generate` 刷新 ADUS。
- 运行 `npm run governance:audit`，确认结构映射与语义候选无新增错误。
- 使用差异检查确认没有生产代码、测试或 Scope 外文件被修改。

## 人类裁决

人类于 2026-08-27 明确确认：

1. 新建文档以 REQ-AG-008 的 CreateDocumentBundle 路径为唯一权威，并修正 REQ-AG-001 的旧验收表述。
2. TaskRun 只为复杂或跨执行片段目标建立，简单目标仅使用 AgentTurn。
3. 冲突的旧实现登记移出 active 规范正文，仅在 Issue Trace 与变更记录中保留历史。
4. 用户补充澄清信息时继续原 AgentTurn，执行 `AwaitingClarification → Assembling`；不新建 AgentTurn，也不把正常补充伪装为取消。该语义只约束 AgentTurn 澄清生命周期，不扩散到 DecisionRequest、DiffCard 或 Assist 回合。

术语复核：本裁决只复用已注册 AgentTurn，没有新增独立业务实体，不新增 TERM。

## HD-05 裁决与解除

规则链复核发现，将 `create_document` / `organize_workspace` 的 `allowed_tools` 严格限制为仅
`plan_workspace_operation` 可能破坏组合任务：根据项目内既有文档新建内容，或在重命名/移动前查看目录，
都可能需要当前任务必要的只读工具。可选语义：

1. 允许必要只读工具 + `plan_workspace_operation`；只读工具只用于取得当前任务事实，资源副作用唯一仍是 CreateDocumentBundle/WorkspaceOperationPlan 前置裁决链。
2. 严格仅 `plan_workspace_operation`；创建/整理所需事实必须在意图判定前已组装完成，不允许该 AgentTurn 再读取。

本取舍不改变“新建文档只走 CreateDocumentBundle”、“目标建立前不得 `propose_edit`”与“工具注册不等于当轮授权”三项不变式。

Stop Condition：SC-05 AMBIGUOUS_TASK。解除方式：人类确认上述两种语义之一。

人类于 2026-08-27 确认选项 1：`create_document` / `organize_workspace` 可同时获得当前任务必要的只读工具与
`plan_workspace_operation`；只读工具不产生资源副作用，资源副作用仍唯一经 CreateDocumentBundle/
WorkspaceOperationPlan 前置裁决链执行。SC-05 解除。

## 本阶段语义比对结论

| 主题 | 需求入口 | 设计权威 | 唯一语义 |
|---|---|---|---|
| 新建文档 | REQ-AG-001/008 | BR-AG-DATA-003、BR-AG-STATE-002 | 确认前形成 CreateDocumentBundle；确认后先建立 DiskState，再投影初始内容 Diff；不存在空基准新建 Proposal。 |
| AgentTurn 澄清 | REQ-AG-007 | BR-AG-STATE-001、BR-AG-DATA-004、SM-AG-01 | `AwaitingClarification → Assembling`，继续原 AgentTurn；不新建 AgentTurn、不伪造取消或完成。 |
| AgentTurn 失败重试 | REQ-AG-007/015 | BR-AG-STATE-001、SM-AG-01 | TurnError 是原 AgentTurn 终态；显式重试创建新 AgentTurn，原 error 记录保留。 |
| TaskRun 创建边界 | REQ-AG-015 | BR-SYS-STATE-008、SM-AG-04 | 简单目标只使用 AgentTurn；复杂或必须跨执行片段的目标才建立 TaskRun/TaskPlan。 |
| TaskRun 等待裁决 | REQ-AG-015 | BR-SYS-STATE-008、SM-AG-04 | WaitingConfirm 只由 DecisionRequest 触发；DiffCard 只管理内容裁决，不驱动 TaskRun。 |
| 当轮最小允许集 | REQ-AG-002/004/007 | BR-AG-STATE-002、BR-AG-DATA-004 | `allowed_tools` 是精确允许集；资源意图可获得必要只读工具 + `plan_workspace_operation`，但资源副作用唯一仍经 CreateDocumentBundle/WorkspaceOperationPlan 前置裁决链。 |

相邻语义复核：DecisionRequest、DiffCard、Assist、错误/中断重试均保持独立语义，没有被“澄清继续原 AgentTurn”泛化。
术语复核：全部使用现有 AgentTurn、IntentResult、CapabilityGrant、TaskRun、TaskPlan 与 DecisionRequest；未形成新的独立业务实体，不新增 TERM。

ADUS 复核：AG-TURN/AG-TOOL 下的相关 RULE、TERM 与代码块索引仍可追溯，但结构注册不等于生产行为已符合新设计。本 Scope 不读取未经规则链映射的代码作为证据，也不声称实现对齐；实现对齐将进入后续独立 Scope。

## Task Runtime Report — Agent 需求与设计语义唯一化

task_type: GOVERNANCE_CHANGE
governance_phase: DESIGN
final_state: READY
issue_trace: `.governance/issues/AG-CONFORMANCE-GOVERNANCE-S1.md`

changed_files:
  - `.governance/active_scope.json` # 修改，记录 Scope Lock、五项人类裁决与 READY 状态
  - `.governance/issues/AG-CONFORMANCE-GOVERNANCE-S1.md` # 新增，Issue Trace、语义比对与运行报告
  - `docs/01_modules/AG/A-AG-M-D-01_代理功能需求描述.md` # 修改，唯一化新建文档、AgentTurn 澄清和 TaskRun 边界
  - `docs/01_modules/AG/A-AG-M-S-01_代理状态机设计.md` # 修改，唯一化 SM-AG-01/04 及测试矩阵
  - `docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md` # 修改，收敛 BR-AG-DATA-003/STATE-001/STATE-002 与 Runtime/TaskRun 契约
  - `docs/01_modules/AG/A-AG-M-T-02_提示词架构设计.md` # 修改，收敛 BR-AG-DATA-004、当轮最小允许集与 Prompt 资源边界
  - `docs/00_core/A-CORE-C-R-01_ADUS.md` # 派生更新，由 governance:generate 生成

scope:
  mode: explicit
  status: confirmed

phase_gate:
  status: passed
  evidence: 人类确认 HD-01~05；REQ-AG-001/002/004/007/008/015、相关 RULE 与 SM-AG-01/04 均存在且可追溯。

mapping_chain:
  status: complete
  evidence: 本 DESIGN Scope 内的 REQ→RULE/CHAIN/TERM→STATE_MACHINE 映射完整；ADUS 已刷新。生产代码行为符合性不在本阶段宣称范围内。

test_impact:
  changed_files: [上述治理文档与派生 ADUS]
  direct_modules: [AG, CORE]
  affected_modules: [AG, CORE]
  selection: governance-only

validation:
  - `npm run governance:generate` # passed，规则/链路/术语/测试索引成功派生；最终运行内容无额外变化
  - `npm run governance:audit` # passed，governance_findings=[]，TERM 完整性通过
  - `npm run test:governance` # passed，19/19；941 个测试用例发现、0 未映射源、0 未归属测试、0 skip/only
  - active 正文旧语义负空间扫描 # passed，无 clarification→Completed、TurnError→Assembling、全量工具、空基准新建、DiffCard→WaitingConfirm 残留
  - `git diff --check` # passed

stop_conditions:
  - 无；SC-05 已由 HD-05 裁决解除

recovery:
  - SC-05 解除：人类确认资源意图可获得必要只读工具 + plan_workspace_operation；重新执行治理生成、审计与测试并通过。

unresolved_items:
  - 生产入口与规则块的精确映射复核、实现对齐、集成验证与审计收紧须分别进入后续 Scope。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-27 | v1.0 | 建立第一阶段语义唯一化 Scope，等待人类确认范围与三项设计裁决。 |
| 2026-08-27 | v1.1 | 人类确认三项权威语义，Scope 进入 SCOPED。 |
| 2026-08-27 | v1.2 | 人类确认澄清补充继续原 AgentTurn；限定影响边界并完成 TERM 必要性复核。 |
| 2026-08-27 | v1.3 | 规则链复核发现资源意图与必要只读能力的组合取舍；记录 HD-05 并按 SC-05 暂停。 |
| 2026-08-27 | v1.4 | 人类确认 HD-05 选项 1：资源意图可获得任务必要只读工具 + plan_workspace_operation，资源副作用路径不变；SC-05 解除。 |
