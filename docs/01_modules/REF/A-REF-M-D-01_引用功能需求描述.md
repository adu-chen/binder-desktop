---
文档编号：REF-M-D-01
文档状态：A
负责模块：REF
文档职责：REF 引用模块级功能需求描述——声明模块级 REQ-REF-*，承接产品级 REQ-CORE-001/006，界定 @mention 触发与目标解析 / 引用内容读取与注入载荷 / 引用锚定与失效感知 / 触发与呈现经 ED 表面
上游约束：A-CORE-C-D-01_产品概述.md（REQ-CORE-001/006）、A-CORE-C-P-01_APC.md（§4.1 REF 职责「引用（@mention）解析与注入」）、A-SYS-C-T-01_系统总体架构.md（§4 REF→WS,AG）、A-WS-M-T-01_工作台技术设计.md（BR-WS-DATA-001 读取经 WS、X-FORBID-004）、A-ED-M-T-01_编辑器技术设计.md（BR-ED-DATA-001 blockId 锚、扩展表面）、A-AG-M-D-01_代理功能需求描述.md（REQ-AG-004 注入消费）、ADU.md（§3.2）
直接承接：REF L2 技术设计（引用目标模型 / 解析规则 / 注入载荷契约）经 REQ_MAP 映射 RULE/CHAIN/CONSTRAINT/TERM；注入载荷被 AG（chat/工具回合）与 CBT（构建上下文）消费
使用边界：本文是 L1 功能需求描述层，仅声明 REQ-REF-*，**不定义技术规则**；**代码不得直接映射 REQ**（ADU §3.2）；上下文组装与预算归 AG（REF 提供注入载荷）；文件读取归 WS；触发输入面与呈现归 ED；知识库条目检索归 KB（可引用范围联合 KB G1/G2 确认）
变更要求：REF 职责或可引用范围变更须先经 APC 授权再回写本文；REQ 增删按 ADU §3.2
治理成熟度：draft
---

# Binder REF 引用功能需求描述（L1）

## §1 文档定位

本文是 **REF 模块**的 L1 功能需求描述，P3 第三件（P3 = DE/AG/REF 的 L1 层至此齐备）：

```
产品概述(REQ-CORE-006) → 本文(REQ-REF-*) → REQ_MAP → REF L2 规则(SSOT) → 代码(@GOV)
```

REF 是 **@mention 引用解析与注入模块**：用户在对话/构建输入中以 @mention 指向 workspace 内
目标（文档/内容块/资源），REF 解析目标、经 WS 限界读取内容、组装为注入载荷供 AG/CBT 消费
（REQ-CORE-006）。REF 依赖 WS（读取限界）与 AG（消费方）；触发输入面与渲染依托 ED 表面。

## §2 模块定位与职责

| # | 职责 | 锚定 |
|---|---|---|
| R-J1 | @mention 触发与目标解析：候选列举、目标确定（workspace 内文档/内容块/资源） | REQ-CORE-006 |
| R-J2 | 引用内容读取与注入载荷：经 WS 读取目标内容，组装注入载荷交 AG/CBT | REQ-CORE-006/001 |
| R-J3 | 引用锚定与失效感知：文档级（路径）与块级（blockId）锚，失效可感知不静默断链 | A-ED-M-T-01（BR-ED-DATA-001） |
| R-J4 | 触发与呈现经 ED 表面：@ 触发输入面、候选面板、引用标记渲染依托 ED 扩展点 | A-ED-M-T-01（X-FORBID-005） |

> 边界澄清：REF **解析与供给**，AG **组装与消费**——上下文长度预算、裁剪与最终组装归 AG
> （REQ-AG-004）；REF 只保证注入载荷的解析正确、来源可溯与锚定有效性。文件读取一律经 WS
> （X-FORBID-004），REF 无直接文件 I/O。

## §3 能力范围（REF v1）

**IN（REF 承载）**：
- @mention 触发协议与候选解析（workspace 内文档、内容块、资源）。
- 引用目标的内容读取（经 WS）与注入载荷组装（含来源标识，供 AG/CBT 消费与溯源）。
- 引用锚：文档级（workspace 相对路径）+ 块级（blockId）；锚失效检测与明确失效信号。
- 引用标记在编辑器/对话输入中的渲染数据（呈现承载依托 ED）。

**OUT（非 REF）**：
- 上下文预算、裁剪与组装进回合——归 AG；构建上下文消费——归 CBT。
- 文件 I/O 与路径守护——归 WS；blockId 稳定性——归 ED；候选 UI 呈现——归 ED 表面。
- 知识库条目检索与召回——归 KB（知识库条目是否纳入可引用范围联合 KB G1 确认，§7 REF-01）。
- 记忆资产注入——归 AST（其注入通道与 REF 的关系联合 AST G1 确认）。

## §4 代表性流程（契约级，APC §7）

**R1 · @mention 引用注入**（R-J1~J4，承接 REQ-CORE-006）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | 用户 | 在对话/构建输入中键入 @ | REF 经 ED 触发面列举 workspace 内候选 | 候选列表 | 2 | 无候选→空列表（非错误） |
| 2 | 用户 | 选定目标 | REF 解析为引用锚（路径/blockId）并渲染引用标记 | 已确定引用 | 3 | 目标不可达→明确失败，不产生悬空引用 |
| 3 | AG/CBT | 回合发起 | REF 经 WS 读取目标内容，组装注入载荷（含来源标识） | 注入载荷交 AG | 终态 | 锚失效→明确失效信号，AG 可示用户重选；读取越界→WS 拒绝 |

**R2 · 锚失效处理**（R-J3）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | REF | 既有引用 + 目标变更 | 检测锚有效性（路径存在/blockId 可定位） | 有效/失效判定 | 2 | 检测不可得→按失效保守处理，不注入过期内容 |
| 2 | REF | 失效判定 | 发明确失效信号（不静默注入空/错位内容） | 失效提示 | 终态 | 用户可重新选定目标 |

## §5 模块级需求声明（REQ-REF-*）

<!-- REQ
req_id: REQ-REF-001
title: @mention 触发与目标解析
owner_module: REF
status: active
summary: 在对话/构建输入中经 @ 触发列举 workspace 内可引用候选（文档/内容块/资源）并解析选定目标为引用锚；目标不可达时明确失败，不产生悬空引用
source_doc: REQ-CORE-006、APC §4.1
acceptance: @ 触发可列举候选并确定目标；目标不可达有明确失败；候选限定 workspace 内
open_issues: 可引用目标范围（知识库条目/记忆是否纳入）联合 KB/AST G1 确认（§7 REF-01）
-->

<!-- REQ
req_id: REQ-REF-002
title: 引用内容读取与注入载荷
owner_module: REF
status: active
summary: 经 WS 限界读取引用目标内容，组装为含来源标识的注入载荷供 AG/CBT 消费；读取不越出 workspace；载荷可溯源到目标
source_doc: REQ-CORE-006、REQ-CORE-001、A-AG-M-D-01（REQ-AG-004）
acceptance: 注入载荷内容与目标一致且含来源标识；读取经 WS 且越界被拒；AG 可消费组装
open_issues: 注入载荷结构与跨模块共享化（是否入 SYS 契约层）归 REF G2 联合 SYS/AG
-->

<!-- REQ
req_id: REQ-REF-003
title: 引用锚定与失效感知
owner_module: REF
status: active
summary: 引用锚支持文档级（workspace 相对路径）与块级（blockId）；锚失效（目标删除/移动/块失效）必须可感知并发明确失效信号，不静默注入过期或错位内容
source_doc: A-ED-M-T-01（BR-ED-DATA-001 失效可感知）、REQ-CORE-006
acceptance: 锚失效有明确信号且不注入失效内容；检测不可得时保守不注入
open_issues: 失效后的修复交互（重选/自动重定位）归 REF G2/实现期
-->

<!-- REQ
req_id: REQ-REF-004
title: 触发与呈现经 ED 表面
owner_module: REF
status: active
summary: @ 触发输入面、候选面板与引用标记渲染依托 ED 扩展表面，REF 不绕过 ED 接口直接操纵编辑器内部状态
source_doc: A-ED-M-T-01（X-FORBID-005、扩展表面）
acceptance: 触发/候选/标记均经 ED 声明的扩展点；无绕过 ED 直改编辑器状态的调用路径
open_issues: 具体扩展点契约随 ED 扩展表面细化（REF G2 联合 ED）
-->

## §6 模块边界与约束（边界闭合）

- **职责边界**：REF 只做解析与供给（§2 R-J1–R-J4）；不组装回合上下文（AG）、不读盘（经 WS）、
  不呈现（经 ED）、不检索知识库（KB）。
- **限界边界**：候选与读取一律限定 workspace 内（X-FORBID-004）；REF 无直接文件 I/O（X-SEQ-002）。
- **锚边界**：块级锚消费 ED blockId 契约（BR-ED-DATA-001），失效信号语义与 ED 一致。
- **共享结构边界**：注入载荷若跨模块传递，结构经 SYS 契约层（X-SHARED-001，REF G2 联合确认）。
- **状态机阈值**：@mention 解析注入为请求-响应式解析流（无多步状态化生命周期），按 APC §6 预判
  豁免建机；REF G2 若暴露状态化链路（如异步批量解析）再建机登记，本文不预判结论为最终。
- **L4 原料边界**：取材台账 §4.4（REF 引用 @mention 原料），原料不为权威。

## §7 G1 问题暴露清单

| 编号 | 分类 | 状态 | 处置 |
|---|---|---|---|
| REF-01 | BOUNDARY_OPEN | 显式延期 | 可引用目标范围（KB 条目/AST 记忆是否纳入 @mention）联合 KB/AST G1 确认 |
| REF-02 | BOUNDARY_OPEN | 显式延期 | 注入载荷结构是否入 SYS 共享契约层归 REF G2 联合 SYS/AG |
| REF-03 | BOUNDARY_OPEN | 显式延期 | 锚失效修复交互（重选/自动重定位）归 REF G2/实现期 |
| REF-04 | FLOW_INCOMPLETE | 显式延期 | 状态机豁免预判须在 REF G2 复核（若暴露状态化链路则建机登记总图） |
| REF-05 | ACCEPTANCE_MISSING | 部分关闭 | 定性验收已入 §5；量化指标（候选列举/解析延迟）延 G5 NFR 标定 |

> 术语：引用、注入载荷等 REF 主责核心概念在 REF L2 评估注册 TERM。

## §8 后续动作

1. 待人类宣布 REF 模块级 G1 闭合后进入 REF G2：引用目标模型与候选解析规则、注入载荷契约
  （解除 REF-02）、锚失效语义细则、与 ED 扩展点/AG 消费的对接契约；复核状态机豁免（REF-04）。
2. G2 经 REQ_MAP 把 REQ-REF-001~004 映射为 RULE/CHAIN/CONSTRAINT/TERM。
3. KB/AST G1 时联合解除 REF-01。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-11 | v1.0 | 初版（P3 第三件，P3 L1 层齐备）：REF 引用模块级 L1；声明 REQ-REF-001~004（@mention 触发与解析 / 内容读取与注入载荷 / 锚定与失效感知 / 经 ED 表面），承接 REQ-CORE-001/006，对接 AG（REQ-AG-004 消费）与 ED（blockId 锚、扩展表面）；代表性流程 R1/R2；G1 暴露清单 REF-01~05 |
| 2026-07-04 | v1.1 | 上游约束文件名指针更新（A-AG-*/A-DE-* 文件更名为 代理*/DiffEdit*，2026-07-04 人类确认；文档编号不变，无语义变更） |
