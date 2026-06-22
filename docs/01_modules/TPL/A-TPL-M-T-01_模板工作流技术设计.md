---
文档编号：TPL-M-T-01
文档状态：A
负责模块：TPL
文档职责：TPL 模板/工作流模块级 L2 技术设计（规则 SSOT）——模板模型与 workspace 级隔离（2026-06-11 人类决策）、模板应用实例化、工作流表达模型与执行编排（状态机评估结论：建机）；承接 REQ-TPL-001~004
上游约束：A-TPL-M-D-01_模板工作流功能需求描述.md（REQ-TPL-001~004、TPL-03 已决策）、A-CORE-C-P-01_APC.md（§4.1/§6/§9）、A-AG-M-T-01_对话代理技术设计.md（回合委托、BR-AG-VERIFY-001）、A-CBT-M-T-01_构建技术设计.md（构建委托、任务计划结构）、A-WS-M-T-01_工作台技术设计.md（X-FORBID-004、控制态）、A-SYS-I-P-01_数据结构定义.md（BR-SYS-PERSIST-001、X-SHARED-001）、A-SYS-M-S-01_状态机总图.md（X-SEQ-003）、ADU.md（§3）
直接承接：本文 RULE/CHAIN/TERM 为 TPL 规则 SSOT；SM-TPL-01 在 A-TPL-M-S-01 定义并登记总图（新增行）；执行委托契约对接 AG/CBT
使用边界：本文是 TPL 规则 SSOT；执行通道归 AG/CBT（TPL 仅编排）；落盘归 DE→WS；内置模板集属运营内容（TPL-04）；状态机内部归 A-TPL-M-S-01
变更要求：工作流表达模型变更须复核 AG/CBT 委托契约；模板/工作流数据结构变更遵守 BR-SYS-PERSIST-001；TERM 一经 term_ref 引用其 en 不可更改
治理成熟度：draft
---

# Binder 模板/工作流技术设计（TPL · L2 技术设计）

## 0. Terminology Registry

<!-- TERM
term_id: TERM-TPL-001
chains: TPL-FLOW
zh: 写作模板
en: Template
forbidden: 文档模板片, 提示模板项, 样板, template条目
-->
可复用的写作起点定义：文档结构模板（实例化为新文档骨架）或提示模板（实例化为回合输入），
含变量占位符；workspace 级隔离存储（REQ-TPL-001/002）。

<!-- TERM
term_id: TERM-TPL-002
chains: TPL-FLOW
zh: 写作工作流
en: WritingWorkflow
forbidden: 流程, 工作流程, pipeline, 写作流程图
-->
多步写作流程的可复用编排定义：步骤序列（每步=模板/指令 + 执行通道 AG/CBT + 可选人类确认点）；
执行经 SM-TPL-01 编排，步骤失败暂停不跳步（REQ-TPL-003）。

## §1 文档定位

```
REQ-CORE-005 → REQ-TPL-001~004（TPL L1，TPL-03 已人类决策）→ 本文（SSOT）→ 代码(@GOV)
本文 →驱动→ A-TPL-M-S-01（SM-TPL-01 工作流执行机）→登记→ 总图（新增行）
执行 →委托→ AG（单文档回合）/ CBT（多文件构建）；产物 → DE→WS（diff 路径）
```

**状态机评估结论（解除 TPL-02）**：工作流执行**建机**——多步、可暂停恢复、跨模块委托（AG/CBT）、
控制态须重启可恢复，满足 APC §6 强制阈值（有状态/跨层/多步），SM-TPL-01 见 A-TPL-M-S-01。
模板应用（单次实例化）为请求-响应式，豁免。

## §2 模板模型与应用（TPL-FLOW）

**Template 字段语义**：template_id、template_kind（document/prompt）、body（含变量占位符）、
organization、created_at/updated_at。变量占位符在实例化时绑定（变量表达语法延实现期）。

<!-- CHAIN
chain_id: TPL-FLOW
主责模块: TPL
status: active
risk_tier: P1
-->

<!-- RULE
rule_id: BR-TPL-DATA-001
主链路: TPL-FLOW
域: DATA
-->
模板与工作流定义按 **workspace 级隔离**存储（2026-06-11 人类决策，遵守 BR-SYS-PERSIST-001
迁移、不上行云端）；模板实例化必须原子：文档模板实例化经正常落盘路径创建新文档、提示模板实例化
组装回合输入交 AG，实例化失败回统一错误形状且不产生半成品（半建文档/半组装输入）；模板对用户
可见可管理，重启可恢复。

## §3 工作流表达模型与执行（TPL-FLOW，解除 TPL-01）

**WritingWorkflow 表达模型**：步骤有序集，每步 = { step_id、模板/指令引用、执行通道
（AG 回合 / CBT 构建）、可选人类确认点（该步执行前暂停待确认）}；支持顺序执行（v1 基线，
条件分支延后扩展，扩展须回写本文）。

<!-- RULE
rule_id: BR-TPL-STATE-001
主链路: TPL-FLOW
域: STATE
-->
工作流执行必须满足：逐步按序委托执行通道（AG 回合或 CBT 构建，每步模型调用经各自门禁承接，
TPL 无独立模型通道）；步骤产物一律走 DE→WS diff 路径（X-FORBID-006）；步骤失败进入暂停态待
用户处置（重试/显式跳过/中止），**不得静默跳步**；声明人类确认点的步骤执行前必须暂停待确认；
中止无半成品落盘；执行控制态（当前步、各步进度）持久化、重启可恢复或明确关闭（无悬挂执行态）。
状态转移归 SM-TPL-01（A-TPL-M-S-01）。

## §4 跨模块约束（复用，不新增）

X-FORBID-006（产物 diff 路径）、X-FORBID-004、X-SHARED-001、X-SEQ-003/X-INTER-001。
不新增 CONSTRAINT。

## §5 需求到规则映射（REQ_MAP）

<!-- REQ_MAP
req_id: REQ-TPL-001
target_type: CHAIN
target_id: TPL-FLOW
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-TPL-001
target_type: RULE
target_id: BR-TPL-DATA-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-TPL-002
target_type: RULE
target_id: BR-TPL-DATA-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-TPL-003
target_type: RULE
target_id: BR-TPL-STATE-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-TPL-004
target_type: RULE
target_id: BR-TPL-DATA-001
status: mapped
-->

## §6 测试与验收

- **隔离与原子实例化**（BR-TPL-DATA-001）：跨 workspace 不可见；实例化失败无半成品；断网可管理；
  重启可恢复。
- **工作流执行**（BR-TPL-STATE-001 / SM-TPL-01）：按序执行；失败暂停不跳步；确认点强制暂停；
  中止无半成品落盘；重启恢复或明确关闭；每步过对应门禁、产物经 diff 路径（与 AG/CBT/DE G5 联调）。
- 量化 NFR 延 G5（TPL-05）。

## §7 技术债务与盲区（诚实声明）

- 变量占位符语法与绑定 UI 延实现期；条件分支表达延后扩展（v1 顺序基线）。
- 内置模板集与默认工作流属运营（TPL-04）。
- 工作流控制态扩展表与其他控制态同批登记 SYS（实现前）。

## §8 后续动作

1. 注册（generate）：TPL-FLOW 链路、BR-TPL-DATA-001·STATE-001、TERM-TPL-001/002、REQ_MAP。
2. 续 A-TPL-M-S-01（SM-TPL-01），总图新增工作流执行行。
3. 待人类宣布 TPL G2 闭合。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-11 | v1.0 | 初版（TPL G2）：人类决策落规则——workspace 级隔离 + 原子实例化（BR-TPL-DATA-001）；工作流表达模型（步骤/通道/人类确认点，顺序基线，解除 TPL-01）与执行规则（不跳步/确认点强制/无悬挂，BR-TPL-STATE-001）；状态机评估结论=建机（解除 TPL-02，SM-TPL-01 归 M-S）；注册链路 TPL-FLOW、术语 Template/WritingWorkflow；REQ_MAP（REQ-TPL-001~004） |
