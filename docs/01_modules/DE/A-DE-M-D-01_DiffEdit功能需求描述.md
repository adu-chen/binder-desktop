---
文档编号：DE-M-D-01
文档状态：A
负责模块：DE
文档职责：DE Diff Edit 模块级功能需求描述——声明模块级 REQ-DE-*，承接产品级 REQ-CORE-002，界定 diff 生成 / diff 审阅呈现 / diff 终态裁决（接受/拒绝/过期）/ diff 内容载荷形状 / 过期与外部变更处理
上游约束：A-CORE-C-D-01_产品概述.md（REQ-CORE-002）、A-CORE-C-P-01_APC.md（§4.1 DE 职责「diff 生成、审阅、接受/拒绝/过期终态」、§6 diff 终态核心链）、A-SYS-C-T-01_系统总体架构.md（§4 DE→ED,WS,AG）、A-SYS-I-P-01_数据结构定义.md（PendingDiff 共享结构、status 枚举集合单一来源、payload_ref 形状归 DE）、A-WS-M-T-01_工作台技术设计.md（BR-WS-STATE-001 仅 accepted 落盘）、A-WS-M-S-01_工作台状态机设计.md（SM-WS-02 X-INTER-001 WS 侧已就绪）、A-ED-M-T-01_编辑器技术设计.md（ED-DECOR 装饰层、BR-ED-DATA-001 blockId 锚定）、A-AG-M-D-01_代理功能需求描述.md（REQ-AG-001 产出交 DE）、ADU.md（§3.2）
直接承接：DE L2 技术设计（diff 模型 / 审阅契约 / 终态机）经 REQ_MAP 映射 RULE/CHAIN/CONSTRAINT/TERM；DE G2 建 diff 终态机（解除总图 planned 行）并完成 X-INTER-001 与 SM-WS-02 的 DE 侧对接
使用边界：本文是 L1 功能需求描述层，仅声明 REQ-DE-*，**不定义技术规则**；**代码不得直接映射 REQ**（ADU §3.2）；pending_diff 持久化与 accepted 后落盘归 WS；diff 呈现承载面归 ED（DE 消费装饰层）；AI 产出生成归 AG（DE 消费其产出）；PendingDiff 字段与 status 枚举集合归 SYS（A-SYS-I-P-01，DE 承接转移语义不重定义字段）
变更要求：DE 职责或 diff 终态语义变更须先经 APC 授权再回写本文；status 枚举集合变更须回写 A-SYS-I-P-01（X-SHARED-001）；REQ 增删按 ADU §3.2
治理成熟度：draft
---

# Binder Diff Edit 功能需求描述（DE · L1）

## §1 文档定位

本文是 **DE 模块**的 L1 功能需求描述，P3 第二件：

```
产品概述(REQ-CORE-002) → 本文(REQ-DE-*) → REQ_MAP → DE L2 规则(SSOT) → 代码(@GOV)
```

DE 是产品核心交互「**对话生成 diff → 人类裁决 → 落盘**」的中枢模块：把 AG 的 AI 产出生成为
pending_diff，经 ED 装饰层呈现供人类审阅，裁决终态（accepted/rejected/expired）；accepted 是
WS 落盘的唯一触发（REQ-CORE-002 的「AI 修改在用户接受前不写入文件」由 DE 终态机 + WS SM-WS-02
共同保障）。DE 依赖 ED（呈现锚定）、WS（持久化与落盘）、AG（产出来源）。
**两侧契约现状**：WS 侧 SM-WS-02 已就绪（accepted 入口 + 落盘回执，X-INTER-001 单侧），
本文及 DE G2 补齐 DE 侧。

## §2 模块定位与职责

| # | 职责 | 锚定 |
|---|---|---|
| DE-J1 | diff 生成：将 AG 产出转为 pending_diff（blockId 定位 + 内容载荷），交 WS 持久化 | REQ-CORE-002、A-SYS-I-P-01 |
| DE-J2 | diff 审阅呈现：经 ED 装饰层按 blockId 呈现 diff 与裁决控件 | ED-DECOR |
| DE-J3 | diff 终态裁决：pending→accepted/rejected/expired 的转移语义与终态不可逆 | A-SYS-I-P-01 status 枚举、APC §6 |
| DE-J4 | diff 内容载荷形状：payload_ref 所指内容的结构定义（A-SYS-I-P-01 显式归 DE） | A-SYS-I-P-01 §3 |
| DE-J5 | 过期与外部变更：超时或目标文档外部变更时转 expired，不落盘 | REQ-CORE-002 |

> 边界澄清：**字段归 SYS、持久化与落盘归 WS、呈现归 ED、产出归 AG，DE 只拥有转移语义与载荷
> 形状**——PendingDiff 字段与 status 枚举集合的单一来源是 A-SYS-I-P-01（DE 不重定义）；
> 持久化、重启恢复与 accepted 后写文件归 WS（SM-WS-02）；DE 的裁决信号是 SM-WS-02 的唯一
> 入场驱动（X-INTER-001）。

## §3 能力范围（DE v1）

**IN（DE 承载）**：
- 自 AG 产出（代理回合/工具回合的修改建议）生成 pending_diff，按 blockId 定位目标内容块。
- diff 审阅交互：经 ED 装饰层呈现修改对比与接受/拒绝控件，收集用户裁决。
- 终态转移语义：pending→accepted / rejected / expired；终态不可逆、互斥；accepted 通知 WS 落盘
  并接收落盘回执（Applied/ApplyError）。
- payload（diff 内容载荷）形状定义与版本化。
- 过期裁决：超时与目标文档外部变更检测触发 expired。

**OUT（非 DE）**：
- pending_diff 持久化、重启恢复、accepted 后写文件与时间轴记节点——归 WS（SM-WS-02/BR-WS-STATE-001）。
- 装饰层承载机制与 blockId 稳定性——归 ED（DE 是消费方，锚失效信号由 ED 提供）。
- AI 产出的生成（模型调用、门禁）——归 AG；多文件构建产物——归 CBT（其落盘同样走 diff 路径，
  契约联合 CBT G2）。
- PendingDiff 字段集合与 status 枚举值集合——归 SYS A-SYS-I-P-01（DE 承接引用）。

## §4 代表性流程（契约级，APC §7）

**DE1 · diff 生成与审阅裁决**（DE-J1/J2/J3，承接 REQ-CORE-002）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | AG | AI 修改建议 | DE 按 blockId 定位生成 pending_diff，交 WS 持久化 | 持久化 pending_diff | 2 | 锚失效→生成失败回报 AG，不产生错位 diff |
| 2 | DE | pending_diff | 经 ED 装饰层呈现修改对比与裁决控件 | 可审阅呈现 | 3 | 呈现锚失效→回报并转过期处理路径 |
| 3 | 用户 | 接受/拒绝 | DE 终态机裁决（终态不可逆） | accepted / rejected | 4 | 超时/外部变更→expired（终态，不落盘） |
| 4 | DE→WS | accepted 信号 | 触发 SM-WS-02 落盘，接收 Applied/ApplyError 回执 | 已落盘 + 装饰复位 | 终态 | ApplyError→diff 保持 accepted 可重试落盘（WS 侧） |

**DE2 · 过期与外部变更**（DE-J5）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | DE | 待裁决 diff + 超时/变更信号 | 检测目标内容块是否已被外部修改或超时 | 过期判定 | 2 | 检测不可得→保守不自动过期，留人类裁决 |
| 2 | DE | 过期判定 | 转 expired（终态），通知 WS 不落盘、ED 复位装饰 | diff 关闭 | 终态 | — |

## §5 模块级需求声明（REQ-DE-*）

<!-- REQ
req_id: REQ-DE-001
title: diff 生成
owner_module: DE
status: active
summary: 将 AG 的 AI 修改建议生成为 pending_diff（PendingDiff 共享结构，blockId 定位目标内容块），交 WS 持久化；定位锚失效时生成失败并回报，不产生错位 diff
source_doc: REQ-CORE-002、A-SYS-I-P-01（PendingDiff）、A-ED-M-T-01（BR-ED-DATA-001）
acceptance: AI 修改建议可生成为持久化 pending_diff 且定位准确；锚失效有明确失败回报
open_issues: diff 粒度（块级/行级/字符级对比）归 DE G2（§7 DE-01）
-->

<!-- REQ
req_id: REQ-DE-002
title: diff 审阅呈现
owner_module: DE
status: active
summary: 经 ED 装饰层按 blockId 呈现 diff 修改对比与接受/拒绝控件，用户交互由 ED 转达 DE 裁决；终态后装饰复位不残留
source_doc: REQ-CORE-002、A-ED-M-T-01（ED-DECOR、BR-ED-STATE-002）
acceptance: diff 可视呈现于编辑器目标位置；裁决控件交互可达；终态后呈现复位
open_issues: 审阅 UI 形态（逐条/批量/侧栏）归 DE G2/实现期（§7 DE-04）
-->

<!-- REQ
req_id: REQ-DE-003
title: diff 终态裁决
owner_module: DE
status: active
summary: 拥有 pending→accepted/rejected/expired 的转移语义：终态互斥且不可逆；accepted 是 WS 落盘的唯一触发（X-INTER-001 对接 SM-WS-02）并接收落盘回执；rejected/expired 通知不落盘
source_doc: REQ-CORE-002、A-SYS-I-P-01（status 枚举集合）、A-WS-M-S-01（SM-WS-02）
acceptance: 终态不可逆且互斥；仅 accepted 触发落盘；终态机与 SM-WS-02 对接一致（G5 联调）
open_issues: diff 终态机归 DE G2 M-S（解除总图 planned 行、补齐 X-INTER-001 DE 侧）
-->

<!-- REQ
req_id: REQ-DE-004
title: diff 内容载荷形状
owner_module: DE
status: active
summary: 定义 payload_ref 所指 diff 内容载荷的结构（修改前后内容、定位信息、来源元数据），为 A-SYS-I-P-01 显式留归 DE 的占位补形；载荷形状版本化以支持迁移
source_doc: A-SYS-I-P-01 §3（payload 形状归 DE）
acceptance: 载荷形状有单一定义且 WS 持久化/恢复后可完整重建审阅呈现
open_issues: 载荷具体结构与版本化策略归 DE G2
-->

<!-- REQ
req_id: REQ-DE-005
title: 过期与外部变更处理
owner_module: DE
status: active
summary: 待裁决 diff 在超时或目标内容被外部修改时转 expired（终态，不落盘）；外部变更检测不可得时保守不自动过期，留人类裁决
source_doc: REQ-CORE-002（W2 异常路径）、A-WS-M-D-01（REQ-WS-003）
acceptance: 超时/外部变更触发 expired 且文件不变；检测不可得时不误杀待裁决 diff
open_issues: 超时时长与外部变更检测机制（内容哈希/版本序）归 DE G2（§7 DE-03）
-->

## §6 模块边界与约束（边界闭合）

- **唯一落盘路径的 DE 侧**：accepted 是落盘唯一触发、终态不可逆——与 WS（BR-WS-STATE-001/
  SM-WS-02）、AG（无旁路写盘）共同构成 REQ-CORE-002 的完整保障链。
- **共享结构边界**：PendingDiff 字段与 status 枚举集合归 SYS（X-SHARED-001），DE 拥有转移语义与
  payload 形状，不重定义字段。
- **呈现边界**：DE 经 ED 装饰层呈现（X-FORBID-005：不绕过 ED 接口直改编辑器状态）。
- **状态机边界**（APC §6）：diff 终态为状态化核心链（总图 owner=DE，planned），DE G2 建机
  （X-SEQ-003）并登记 X-INTER-001 DE 侧。
- **L4 原料边界**：取材台账 §4 Diff Edit/Decoration 原料（R-ED-M-R-22 等 P2），原料不为权威。

## §7 G1 问题暴露清单

| 编号 | 分类 | 状态 | 处置 |
|---|---|---|---|
| DE-01 | BOUNDARY_OPEN | 显式延期 | diff 粒度（块级/行级/字符级对比与混合策略）归 DE G2 |
| DE-02 | DESIGN_RISK | 显式延期 | 同一文档多 pending_diff 并存的顺序、重叠与冲突处理归 DE G2（影响落盘正确性，联动 WS） |
| DE-03 | BOUNDARY_OPEN | 显式延期 | expired 触发细则（超时时长、外部变更检测机制）归 DE G2 |
| DE-04 | NEEDS_HUMAN_DECISION | 显式延期（不阻断 G1） | 审阅 UI 形态（逐条/批量裁决、侧栏/内联）属产品体验决策，延 G2/实现期人类确认 |
| DE-05 | ACCEPTANCE_MISSING | 部分关闭 | 定性验收已入 §5；量化指标（diff 生成/呈现延迟）延 G5 NFR 标定 |

> 术语：diff / pending_diff 为 DE 主责核心概念（A-SYS-I-P-01 §6 已显式延期至 DE），DE L2 注册
> TERM；本文 L1 不注册。

## §8 后续动作

1. 待人类宣布 DE 模块级 G1 闭合后进入 DE G2：diff 模型与粒度（解除 DE-01）、多 diff 冲突
  （解除 DE-02）、payload 形状（REQ-DE-004）、过期细则（解除 DE-03）、diff 终态机（M-S，回填
  总图 planned 行 + X-INTER-001 DE 侧，与 SM-WS-02 完成双侧对接）、注册 diff 相关 TERM。
2. G2 经 REQ_MAP 把 REQ-DE-001~005 映射为 RULE/CHAIN/CONSTRAINT/TERM。
3. CBT G1/G2 时联合确认构建产物走 diff 路径的契约。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-11 | v1.0 | 初版（P3 第二件）：DE 对话编辑模块级 L1；声明 REQ-DE-001~005（diff 生成 / 审阅呈现 / 终态裁决 / 载荷形状 / 过期与外部变更），承接 REQ-CORE-002 与 A-SYS-I-P-01 PendingDiff 契约，对接 SM-WS-02（X-INTER-001 WS 侧已就绪）与 ED-DECOR；代表性流程 DE1/DE2；G1 暴露清单 DE-01~05 |
| 2026-07-04 | v1.1 | 术语统一清扫（承 2026-07-04 人类三轮裁决）：正文禁用别名替换——内联辅助→局部修改、自动补全→辅助续写、对话编辑→Diff Edit、Chat↔Build 接管→构建移交/回落、chat/chat-agent 回合→代理回合；文件名与变更记录历史行不改写 |
