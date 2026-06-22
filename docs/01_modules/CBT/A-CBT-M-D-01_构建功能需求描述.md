---
文档编号：CBT-M-D-01
文档状态：A
负责模块：CBT
文档职责：CBT 构建模块级功能需求描述——声明模块级 REQ-CBT-*，承接产品级 REQ-CORE-003，界定多文件文档项目构建回合 / 构建产物 diff 化落盘 / 构建中断与恢复 / Chat↔Build 接管
上游约束：A-CORE-C-D-01_产品概述.md（REQ-CORE-003）、A-CORE-C-P-01_APC.md（§4.1 CBT 职责、§6 CBT 构建核心链）、A-SYS-C-T-01_系统总体架构.md（§4 CBT→AG,WS,DE）、A-AG-M-T-01_对话代理技术设计.md（BR-AG-VERIFY-001 门禁前置、X-FORBID-006、SM-AG-01/02 回合基建）、A-DE-M-T-01_对话编辑技术设计.md（diff 路径、批量裁决债务）、A-WS-M-T-01_工作台技术设计.md（X-FORBID-004）、A-SYS-M-S-01_状态机总图.md（CBT 构建核心链 planned、Chat↔Build 接管转移点）、ADU.md（§3.2）
直接承接：CBT L2 技术设计（构建任务模型 / 产物 diff 化 / 构建状态机）经 REQ_MAP 映射 RULE/CHAIN/CONSTRAINT/TERM；CBT G2 建构建机（解除总图 planned 行）并与 AG 完成接管转移点对接
使用边界：本文是 L1 功能需求描述层，仅声明 REQ-CBT-*，**不定义技术规则**；**代码不得直接映射 REQ**（ADU §3.2）；单文档回合归 AG（CBT 复用其回合/门禁基建）；diff 终态归 DE；落盘归 WS；任务分解模型与构建编排归 CBT L2
变更要求：CBT 职责或构建边界变更须先经 APC 授权再回写本文；REQ 增删按 ADU §3.2
治理成熟度：draft
---

# Binder CBT 构建功能需求描述（L1）

## §1 文档定位

```
产品概述(REQ-CORE-003) → 本文(REQ-CBT-*) → REQ_MAP → CBT L2 规则(SSOT) → 代码(@GOV)
```

CBT 是 **agent 文档项目构建模块**（Chat Build）：承载跨多文件的文档项目构建回合、构建中断与
Chat↔Build 接管（APC §4.1）。CBT 依赖 AG（回合/门禁/工具基建）、WS（文件边界）、DE（产物 diff
化裁决）。**云薄端与唯一落盘路径前提**：构建的模型调用经 AG 基建过门禁（BR-AG-VERIFY-001）；
构建产物写入 workspace 必走 diff 裁决路径（X-FORBID-006 涉及模块含 CBT）。取材台账 §4.10
（P1/P2 原料，不为权威）。

## §2 模块定位与职责

| # | 职责 | 锚定 |
|---|---|---|
| C-J1 | 多文件构建回合：跨 ≥2 文件的文档项目产物构建（任务分解→逐步产出） | REQ-CORE-003 |
| C-J2 | 产物 diff 化落盘：构建产物经 DE 裁决后由 WS 落盘，不旁路直写 | X-FORBID-006、REQ-CORE-002 |
| C-J3 | 构建中断与恢复：构建过程可中断，中断不残留半成品，控制态可恢复 | REQ-CORE-003 |
| C-J4 | Chat↔Build 接管：单文档 chat 回合与构建回合间的接管/中断对接 | REQ-CORE-003、总图 §5 转移点 |

> 边界澄清：CBT **复用 AG 基建**——模型调用、门禁前置、工具循环均经 AG（SM-AG-01/02），CBT 在其上
> 编排多文件构建任务；产物落盘语义与单文档一致：**生成 ChangeProposal 交 DE**（批量裁决形态联动
> DE 批量债务），accepted 后 WS 落盘。CBT 不自带模型调用通道、不自带写盘通道。

## §3 能力范围（CBT v1）

**IN**：多文件构建任务的分解与编排；构建产物（新建文档/修改既有文档）的 diff 化提交；构建进度
与中断控制；Chat↔Build 双向接管（chat 升级为构建 / 构建回落 chat）；构建控制态重启可恢复。

**OUT**：模型调用与门禁（AG）；diff 裁决终态（DE）；文件 I/O 与落盘（WS）；单文档 chat（AG）；
模板化工作流定义（TPL，CBT 可消费）；构建任务分解算法与编排模型细节（CBT L2）。

## §4 代表性流程（契约级，APC §7）

**C1 · 多文件构建**（C-J1/J2/J3）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | 用户 | 构建意图（含 @mention 上下文） | CBT 分解为构建任务计划 | 任务计划 | 2 | 意图不明→回落 chat 澄清 |
| 2 | CBT | 任务计划 | 逐任务经 AG 回合基建产出（过门禁） | 产物 ChangeProposal 集 | 3 | 无订阅/断网→明确拒绝；用户中断→中断终态，无半成品落盘 |
| 3 | DE/用户 | ChangeProposal 集 | diff 化裁决（批量形态联动 DE） | accepted 落盘 / rejected 不落 | 终态 | 部分接受→仅 accepted 部分落盘 |

**C2 · Chat↔Build 接管**（C-J4）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | AG | chat 回合中识别多文件构建意图 | 经接管转移点交 CBT（用户确认接管） | 构建回合接管 | 2 | 用户拒绝接管→留在 chat |
| 2 | CBT | 构建中断/完成 | 回落 chat 或结束 | chat 续接/回合结束 | 终态 | 接管中断→两侧状态一致（X-INTER-001） |

## §5 模块级需求声明（REQ-CBT-*）

<!-- REQ
req_id: REQ-CBT-001
title: 多文件构建回合
owner_module: CBT
status: active
summary: 支持跨 ≥2 文件的文档项目构建：构建意图分解为任务计划，逐任务经 AG 回合基建产出（模型调用过门禁），产物可跨新建与修改既有文档
source_doc: REQ-CORE-003、A-AG-M-T-01（BR-AG-VERIFY-001、SM-AG-01/02）
acceptance: agent 可跨 ≥2 文件构建产物；构建模型调用全部经 AG 门禁基建，无独立调用通道
open_issues: 任务分解与编排模型归 CBT G2（§7 CBT-01）
-->

<!-- REQ
req_id: REQ-CBT-002
title: 构建产物 diff 化落盘
owner_module: CBT
status: active
summary: 构建产物（新建/修改文档）一律生成 ChangeProposal 交 DE 裁决，accepted 后经 WS 落盘；CBT 无旁路写盘路径；支持按条/批量裁决（批量形态联动 DE）
source_doc: X-FORBID-006、REQ-CORE-002、A-DE-M-T-01（§11 批量裁决债务）
acceptance: 构建产物在用户接受前不写入文件；部分接受时仅 accepted 部分落盘；无旁路直写
open_issues: 批量裁决交互形态联动 DE-04 / DE G2 债务（§7 CBT-02）；新建文档的 diff 表达（无 before 基准）归 CBT/DE G2 联合
-->

<!-- REQ
req_id: REQ-CBT-003
title: 构建中断与恢复
owner_module: CBT
status: active
summary: 构建过程用户可随时中断；中断为终态、不产生半成品落盘（未裁决产物保持待裁决或丢弃）；构建控制态持久化、重启后可恢复或明确关闭
source_doc: REQ-CORE-003、A-SYS-I-P-02（BR-SYS-STATE-002 中断语义）、A-WS-M-T-01（控制态持久化）
acceptance: 中断后无半成品落盘；重启后构建控制态可恢复或一致关闭
open_issues: 构建控制态持久化结构（是否入 workspace.db）归 CBT G2 联合 WS/SYS
-->

<!-- REQ
req_id: REQ-CBT-004
title: Chat↔Build 接管
owner_module: CBT
status: active
summary: chat 回合与构建回合可双向接管（chat 识别构建意图经用户确认升级为构建；构建可回落 chat），接管/中断两侧状态一致（X-INTER-001 转移点）
source_doc: REQ-CORE-003、A-SYS-M-S-01 §5（Chat↔Build 接管转移点）、A-AG-M-S-01 §8（AG 侧待联合）
acceptance: 接管经用户确认；接管与中断后 AG/CBT 两侧状态一致；构建可中断且可接管（REQ-CORE-003 验收）
open_issues: 接管协议（事件/回执语义）归 CBT G2 联合 AG（§7 CBT-03）
-->

## §6 模块边界与约束（边界闭合）

- **基建复用**：模型调用/门禁/工具循环经 AG（无独立通道）；落盘经 DE→WS diff 路径
  （X-FORBID-006）；文件边界 X-FORBID-004。
- **状态机边界**（APC §6）：CBT 构建为状态化核心链（总图 planned，owner=CBT），CBT G2 建机
  （X-SEQ-003）并登记 Chat↔Build 接管转移点（X-INTER-001）。
- **L4 原料边界**：取材台账 §4.10，原料不为权威。

## §7 G1 问题暴露清单

| 编号 | 分类 | 状态 | 处置 |
|---|---|---|---|
| CBT-01 | BOUNDARY_OPEN | 显式延期 | 构建任务分解与编排模型归 CBT G2 |
| CBT-02 | DESIGN_RISK | 显式延期 | 批量 diff 裁决形态联动 DE-04 与 DE G2 批量债务；新建文档 diff 表达（无 before 基准）联合 DE G2 |
| CBT-03 | BOUNDARY_OPEN | 显式延期 | Chat↔Build 接管协议（事件/回执）归 CBT G2 联合 AG |
| CBT-04 | BOUNDARY_OPEN | 显式延期 | 构建控制态持久化结构归 CBT G2 联合 WS/SYS |
| CBT-05 | ACCEPTANCE_MISSING | 部分关闭 | 定性验收已入 §5；量化指标（构建吞吐/中断响应）延 G5 NFR |

> 术语：构建任务、构建回合等 CBT 主责核心概念在 CBT L2 注册 TERM。

## §8 后续动作

1. 待人类宣布 CBT G1 闭合后进入 CBT G2：任务分解模型、产物 diff 化契约（含新建文档表达，联合
   DE）、构建状态机（解除总图 planned 行）、接管协议（联合 AG，闭合总图 Chat↔Build 转移点）。
2. G2 经 REQ_MAP 映射 REQ-CBT-001~004。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-11 | v1.0 | 初版（P4 首件）：CBT 构建模块级 L1；声明 REQ-CBT-001~004（多文件构建回合 / 产物 diff 化落盘 / 中断与恢复 / Chat↔Build 接管），承接 REQ-CORE-003，复用 AG 门禁基建与 DE→WS diff 路径（X-FORBID-006）；代表性流程 C1/C2；G1 暴露清单 CBT-01~05 |
