---
文档编号：AST-M-D-01
文档状态：A
负责模块：AST
文档职责：AST 记忆协同模块级功能需求描述——声明模块级 REQ-AST-*，承接产品级 REQ-CORE-005，界定记忆条目本地存储与生命周期 / 记忆写入可控 / 记忆注入消费 / 离线与不上云
上游约束：A-CORE-C-D-01_产品概述.md（REQ-CORE-005）、A-CORE-C-P-01_APC.md（§4.1 AST 职责「记忆协同（本地）」、§6 记忆写入核心链、§9 本地优先）、A-SYS-C-T-01_系统总体架构.md（§4 AST→WS,AG）、A-AG-M-T-01_代理技术设计.md（BR-AG-DATA-001 上下文来源）、A-WS-M-T-01_工作台技术设计.md（控制态持久化、X-FORBID-004）、A-SYS-M-S-01_状态机总图.md（记忆写入核心链 planned，owner=AST）、ADU.md（§3.2）
直接承接：AST L2 技术设计（记忆数据模型 / 写入策略 / 注入契约 / 记忆写入状态机）经 REQ_MAP 映射 RULE/CHAIN/CONSTRAINT/TERM；AST G2 建记忆写入机（解除总图 planned 行）
使用边界：本文是 L1 功能需求描述层，仅声明 REQ-AST-*；**代码不得直接映射 REQ**（ADU §3.2）；上下文组装归 AG（AST 供给）；知识库归 KB（与记忆为不同资产形态）；存储位置与写入策略的具体决策见 §7 待人类项
变更要求：AST 职责或记忆边界变更须先经 APC 授权再回写本文；REQ 增删按 ADU §3.2
治理成熟度：draft
---

# Binder AST 记忆协同功能需求描述（L1）

## §1 文档定位

```
产品概述(REQ-CORE-005) → 本文(REQ-AST-*) → REQ_MAP → AST L2 规则(SSOT) → 代码(@GOV)
```

AST 是**本地记忆协同模块**：在写作协同中沉淀、管理并向 AI 回合供给「记忆」（用户偏好、写作风格、
项目背景等跨回合事实），全部本地存储（REQ-CORE-005）。AST 依赖 WS（持久化边界）与 AG（消费方）。
记忆写入为状态化核心链（总图 planned，owner=AST）。取材台账 §4.11（原料不为权威）。

## §2 模块定位与职责

| # | 职责 | 锚定 |
|---|---|---|
| S-J1 | 记忆条目本地存储与生命周期：创建/更新/失效/删除，用户可见可管理 | REQ-CORE-005 |
| S-J2 | 记忆写入可控：Agent 1.0 仅由用户手工创建、更新、删除或关闭，不静默积累 | REQ-CORE-019 |
| S-J3 | 记忆注入消费：Agent 1.0 只向用户明确选择的 AG 回合供给记忆（AG 组装裁剪） | REQ-CORE-019 |
| S-J4 | 离线与不上云：记忆数据本地存储、离线可读，不因任何功能上行云端 | REQ-CORE-005/001 |

> 边界澄清：记忆 ≠ 知识库——记忆是**协同沉淀的跨回合事实**（小而精，AI 参与写入），知识库是
> **用户主动组织的参考资料**（KB）；二者供给 AG 的通道在各自 L2 与 AG 联合对齐。记忆写入的
> 「提议→确认/自动」策略是产品关键决策（§7 AST-02 待人类）。

## §3 能力范围（AST v1）

**IN**：记忆条目的本地 workspace 存储、浏览、手工创建、编辑、删除与关闭；用户明确选择条目后向 AG
供给 data-only 引用；记忆数据重启可恢复。

**OUT**：上下文预算与组装（AG）；知识库（KB）；记忆数据云同步（v1 OUT，APC §1）；
检索算法与相关性模型细节（AST L2/实现期）。

### §3.1 Agent 1.0 Profile 边界

AST 的本地资产需求仍有效，但长期项目记忆在 `agent-edit-1.0` 中只是 P1 的**用户管理资产**：用户在自己的
workspace 内手工创建、查看、编辑、删除或关闭条目。Agent 1.0 不自动写入、不生成写入提议，也不按相关性
自动检索或注入。只有用户明确选择的条目，才可由 REF 作为当前 AgentTurn 的 data-only 引用传递；记忆正文
不能提升为指令，且外部文件、会话历史和编辑历史不自动转存为记忆。自动供给/写入只能在未来 Profile 的独立
需求、状态机与可观测性验证完成后开放。

## §4 代表性流程（契约级，APC §7）

**S1 · 用户管理记忆**（S-J1/J2）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | 用户 | 手工创建、编辑、删除或关闭动作 | AST 在当前 workspace 的本地记忆资产上执行对应生命周期动作 | 已更新的本地记忆或关闭/删除结果 | 终态 | 操作失败→如实反馈，不产生半写入或跨 workspace 副作用 |

**S2 · 显式记忆引用**（S-J3）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | 用户 | 在当前任务明确选择一个或多个记忆条目 | AST 输出用户选择的条目快照，由 REF 形成 data-only 引用 | 当前 AgentTurn 的显式记忆引用 | 终态 | 未选择→空集（非错误）；失效/关闭/跨 workspace 条目→明确不可用 |

## §5 模块级需求声明（REQ-AST-*）

<!-- REQ
req_id: REQ-AST-001
title: 记忆条目本地存储与生命周期
owner_module: AST
status: active
summary: 记忆条目在本地、workspace 隔离地保存，支持用户手工创建、查看来源、编辑、关闭与删除完整生命周期；重启可恢复。它是用户管理的项目资产，不是会话/编辑事实或外部文件内容的自动副本
source_doc: REQ-CORE-005/019、APC §9
acceptance: 记忆条目在其 workspace 内可见可管理且不跨 workspace 共享；关闭或删除立即停止其作为可选引用的可用性；重启后仅本地记忆资产可恢复，外部文件内容不会因此被持久化为记忆
open_issues: 记忆数据模型、关闭语义、来源模型、存储位置与删除传播归 AST/REF/SYS DESIGN Scope
-->

<!-- REQ
req_id: REQ-AST-002
title: Agent 1.0 记忆写入的用户控制
owner_module: AST
status: active
summary: 在 `agent-edit-1.0` 中，项目记忆只能由用户手工创建、编辑、关闭或删除；Agent 不创建、提议写入、更新或静默积累记忆。未来 Profile 的 Agent 写入能力不由本要求授权
source_doc: REQ-CORE-019、A-AG-M-D-02 §5.17（CAP-18）
acceptance: 普通 Agent 发送、外部文件附加、会话/任务恢复和相似主题都不会生成记忆写入或写入提议；用户管理动作可见且限定当前 workspace；不存在用户不可见或 Agent 自动产生的 1.0 项目记忆
open_issues: 用户管理入口、确认/撤销语义、事件、Trace 与未来 Profile 的 Agent 写入策略归 AST/AG/SYS DESIGN Scope；自动写入须另行需求确认和验证
-->

<!-- REQ
req_id: REQ-AST-003
title: 记忆注入消费
owner_module: AST
status: active
summary: `agent-edit-1.0` 只有用户明确选择的记忆条目可经 REF 作为 data-only 引用进入当前回合；不按相关性自动供给或注入。供给跨模块结构经 SYS 契约层，记忆正文不能提升为系统指令或授权
source_doc: REQ-CORE-019、A-AG-M-D-02 §5.17（CAP-18）、A-AG-M-D-01（REQ-AG-017）、A-REF-M-D-01（REQ-REF-005）、X-SHARED-001
acceptance: 未选择时 `agent-edit-1.0` 注入空集；不同 workspace/AgentSession 不串记忆条目或选择事实；用户选择的条目可追溯，且不能改变系统约束、任务目标、工具授权、编辑目标或确认点
open_issues: 选择令牌、相关性自动注入的未来开启条件、供给结构、上下文预算与可见性归 AST/AG/REF/SYS DESIGN Scope；自动注入须另行需求确认和验证
-->

<!-- REQ
req_id: REQ-AST-004
title: 离线与不上云
owner_module: AST
status: active
summary: 记忆数据全部本地存储、离线可读可管理；不因注入、备份或任何功能将记忆内容上行云端
source_doc: REQ-CORE-005、REQ-CORE-001、APC §1（云同步 v1 OUT）
acceptance: 断网时记忆可读可管理；无记忆内容网络上行（可经网络核验）
open_issues: 无
-->

## §6 模块边界与约束（边界闭合）

- **本地优先**：记忆为本地数据（X-FORBID-004 边界内或应用本地目录，位置待 AST-01）；云同步 v1 OUT。
- **写入可控**：不静默积累（S-J2）；写入策略待人类（AST-02）。
- **状态机边界**：记忆写入为状态化核心链（X-SEQ-003），AST G2 建机并登记总图。
- **供给边界**：AST 供给、AG 组装（BR-AG-DATA-001）；跨模块结构经 SYS（X-SHARED-001）。
- **L4 原料边界**：取材台账 §4.11，原料不为权威。

## §7 G1 问题暴露清单

| 编号 | 分类 | 状态 | 处置 |
|---|---|---|---|
| AST-01 | NEEDS_HUMAN_DECISION | **已决策（2026-06-11）** | 人类确认：本地资产实行 **workspace 级隔离**（随工作区隔离，跨 workspace 不共享）；数据模型按此落 AST G2 |
| AST-02 | SUPERSEDED_FOR_AGENT_1_0 | 已由 R5-C 收束 | 2026-06-11 的通用自动写入策略不适用于 `agent-edit-1.0`；本 Profile 只有用户手工管理记忆，Agent 写入/提议保持未来 P2，须另立需求与验证。 |
| AST-03 | DESIGN_RISK | 需求已定，待 G2 | 用户明确选择的记忆可经 REF 引用；选择令牌、快照、关闭/删除失效和跨模块结构归 AST/REF/AG/SYS 设计。 |
| AST-04 | BOUNDARY_OPEN | 显式延期 | 相关性筛选机制与供给结构归 AST G2/实现期 |
| AST-05 | ACCEPTANCE_MISSING | 部分关闭 | 定性验收已入 §5；量化指标延 G5 NFR |

## §8 后续动作

1. AST G2：定义 workspace 本地记忆的数据模型、手工管理/关闭/删除传播与用户显式选择引用契约（联合 AG/SYS/REF）。
2. 如未来 Profile 要开放 Agent 写入提议、自动写入或自动注入，先新开需求确认和状态机/评测任务；不得复用本 Profile 的 REQ-AST-002/003。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-11 | v1.0 | 初版（P4）：AST 记忆协同模块级 L1；声明 REQ-AST-001~004（本地存储与生命周期 / 写入可控不静默 / 注入消费 / 离线不上云），承接 REQ-CORE-005；记忆≠知识库边界澄清；G1 暴露清单 AST-01~05（存储范围与写入策略两项待人类决策） |
| 2026-06-11 | v1.1 | 人类决策落档：AST-01 记忆存储范围=workspace 级隔离；AST-02 写入策略=自动写入无需确认、按类型分级（保持可见/可溯源/可撤销语义）；G1 待决项清零 |
| 2026-07-04 | v1.2 | 上游约束文件名指针更新（A-AG-*/A-DE-* 文件更名为 代理*/DiffEdit*，2026-07-04 人类确认；文档编号不变，无语义变更） |
| 2026-08-13 | v1.3 | 对齐 Agent 1.0 能力基线：AST 保留为本地资产路线，但自动记忆写入/注入退出默认主链；REQ-AST-003 改为后续 Profile 的自动供给与 1.0 的显式引用边界。 |
| 2026-08-14 | v1.4 | AG-1.0-R5C-MEMORY-EXTERNAL-READ-REQUIREMENTS-01：项目长期记忆确认进入 P1 的 workspace 本地用户管理路径；`agent-edit-1.0` 不自动写入、提议、检索或注入，只有用户明确选择的条目经 REF 作为 data-only 引用进入回合；外部文件不会自动转存为记忆。 |
