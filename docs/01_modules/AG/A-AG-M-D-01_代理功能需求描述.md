---
文档编号：AG-M-D-01
文档状态：A
负责模块：AG
文档职责：AG 代理模块级功能需求描述——声明模块级 REQ-AG-*，承接产品级 REQ-CORE-001/002/006/008/009，界定编辑模式代理回合 / agent 工具调用回合 / 模型调用门禁与网关前置 / 上下文组装与注入消费 / 局部修改与辅助续写（2026-06-11 人类确认归 AG）
上游约束：A-CORE-C-D-01_产品概述.md（REQ-CORE-001/002/006/008/009）、A-CORE-C-P-01_APC.md（§4.1 AG 职责「代理编辑面板双模式、工具调用回合」v1.2、§4.3 云薄端、§6 核心链）、A-SYS-C-T-01_系统总体架构.md（§4 AG→SYS,WS,ENT,GW）、A-SYS-I-P-03_云接口契约.md（SM-SYS-01、X-SEQ-001 门禁前置、TERM-CLOUD-*）、A-ED-M-T-01_编辑器技术设计.md（ED-DECOR 装饰层承载）、A-WS-M-T-01_工作台技术设计.md（X-FORBID-004）、A-SYS-M-S-01_状态机总图.md（代理回合 / tool-call / autocomplete / inline-assist 核心链 owner=AG）、ADU.md（§3.2）
直接承接：AG L2 技术设计（回合模型 / 工具调用契约 / 上下文组装 / 局部修改）经 REQ_MAP 映射 RULE/CHAIN/CONSTRAINT/TERM；AG G2 建 代理回合 / tool-call / autocomplete / inline-assist 状态机并登记总图
使用边界：本文是 L1 功能需求描述层，仅声明 REQ-AG-*，**不定义技术规则**；**代码不得直接映射 REQ**（ADU §3.2）；diff 生成与终态归 DE；构建（多文件）归 CBT；@mention 解析归 REF（AG 消费注入）；订阅门禁裁决归服务端（ENT/GW 客户端契约归 SYS I-P-03）；记忆/知识库归 AST/KB（AG 消费）
变更要求：AG 职责或回合边界变更须先经 APC 授权再回写本文；REQ 增删按 ADU §3.2 req_id 规则
治理成熟度：draft
---

# Binder AG 代理功能需求描述（L1）

## §1 文档定位

本文是 **AG 模块**的 L1 功能需求描述，P3 阶段首件：

```
产品概述(REQ-CORE-*) + APC §4.1/§4.3 → 本文(REQ-AG-*) → REQ_MAP → AG L2 规则(SSOT) → 代码(@GOV)
```

AG 是 Binder 的**代理模块**（Agent，旧称对话代理）：承载代理编辑（编辑/构建双模式）回合与工具调用循环（产品核心交互
「对话生成 diff→人类裁决→落盘」的对话侧），以及局部修改与辅助续写（2026-06-11 人类确认归 AG）。
AG 依赖 SYS（IPC/云契约）、WS（上下文读取限界）、ENT/GW（订阅门禁与 AI 网关，经 SYS I-P-03 契约）；
被 DE（消费 AI 产出生成 diff）、CBT（构建移交/回落）、REF（注入消费）关联。
**云薄端硬边界（APC §4.3）**：AG 的一切模型调用必经订阅门禁（SubscriptionGate）与 AI 网关
（AiGateway），客户端不持密钥、无绕过路径；本文所有 AI 能力均在此前提下声明。

## §2 模块定位与职责

| # | 职责 | 锚定 |
|---|---|---|
| A-J1 | 代理编辑回合（编辑模式）：用户↔AI 对话，AI 修改建议交 DE 生成 diff，AG 不直接写盘；默认目标当前文档，显式指向可跨/新建；兼文件创建、文件树整理（删除须确认）、项目感知（2026-07-04 裁决） | REQ-CORE-002 |
| A-J2 | agent 工具调用回合：模型请求工具→执行→结果回注的循环，可中断 | REQ-CORE-002、APC §6 |
| A-J3 | 模型调用门禁与网关前置：每次调用必经 SM-SYS-01 GatePassed、经 GW | REQ-CORE-008、X-SEQ-001 |
| A-J4 | 上下文组装：消费 REF 的 @mention 注入与 AST/KB 资产，上下文读取经 WS 限界 | REQ-CORE-006/001 |
| A-J5 | 辅助续写（Autocomplete）与局部修改（InlineAssist）：编辑器内快捷键手动触发的轻量 AI 辅助，呈现经 ED 装饰层（AI 编辑三层之①②，2026-07-04 澄清定义） | 2026-06-11 人类确认、REQ-CORE-002/008 |

> 边界澄清（2026-07-04 更新）：AI **文档内容修改**落盘路径**唯一**：AG 产出 → DE 生成
> pending_diff 并裁决终态 → WS accepted 落盘（REQ-CORE-002）；工具调用中**任何内容写入意图
> 同样必须走该 diff 路径**，不得旁路直写。文件树结构操作（移动/重命名/新建文件夹）不属内容
> 写入、直接执行；**删除须用户确认**。项目级整体构建编排归 CBT（构建移交判据=项目级构建意图，
> 非多文件修改；构建在原对话窗口实现，两模式互相复用能力）。

## §3 能力范围（AG v1）

**IN（AG 承载）**：
- 编辑模式代理回合（多轮对话、流式回显、回合中断/取消）。
- agent 工具调用循环（工具注册消费、执行、结果回注、循环终止）。
- 每次模型调用的门禁/网关前置执行（消费 SYS I-P-03 契约；套餐计量与 BYOK 透传语义由服务端裁决）。
- 上下文组装：当前文档、@mention 注入（REF 提供）、记忆/知识库资产（AST/KB 提供）的组装消费。
- inline-assist 与 autocomplete 的业务回合（触发、调用、候选产出），呈现经 ED 装饰层。

**OUT（非 AG）**：
- diff 生成、审阅 UI、终态裁决——归 DE；落盘——归 WS。
- 多文件文档项目构建、构建中断与接管——归 CBT（AG 提供 chat 侧对接）。
- @mention 解析——归 REF；记忆/知识库存储——归 AST/KB；编辑呈现——归 ED。
- 订阅门禁裁决、密钥持有、计量——归服务端（AUTH/GW/LIC），客户端契约归 SYS I-P-03。
- 离线 AI——明确拒绝（REQ-CORE-009：AI 功能需在线鉴权 + 有效订阅，断网/无订阅有明确错误）。

## §4 代表性流程（契约级，APC §7）

**A1 · 编辑模式代理回合**（A-J1/A-J3，承接 REQ-CORE-002/008）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | 用户 | prompt（含上下文） | AG 组装上下文，过门禁（SM-SYS-01 GatePassed） | 已授权调用 | 2 | 无有效订阅→GateDenied，明确拒绝；断网→明确错误 |
| 2 | AG | 已授权调用 | 经 GW 发起，流式回显（BR-SYS-STATE-002 终态契约） | AI 回复/修改建议 | 3 | 流式错误→统一错误形状；用户取消→中断终态 |
| 3 | AG | AI 修改建议 | 交 DE 生成 pending_diff（AG 不直接写盘） | pending_diff 进入裁决 | 终态 | 后续归 DE/WS（裁决/落盘） |

**A2 · 工具调用循环**（A-J2）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | 模型 | 工具调用请求 | AG 校验工具在注册清单与权限边界内后执行 | 工具结果 | 2 | 未注册/越权工具→拒绝并回注错误 |
| 2 | AG | 工具结果 | 回注模型继续回合 | 下一轮输出 | 1 或 3 | 循环上限/用户中断→中断终态 |
| 3 | AG | 终轮输出 | 按 A1 步骤 3 处理修改建议 | 回合完成 | 终态 | 文件写入意图必须走 diff 路径，不得旁路 |

**A3 · autocomplete / inline-assist**（A-J5，承接 2026-06-11 归属决策）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | ED | 编辑上下文触发信号 | AG 按触发/节流策略发起轻量调用（同样过门禁） | 候选建议 | 2 | 无订阅/断网→静默不提供（不打断编辑） |
| 2 | AG | 候选建议 | 经 ED 装饰层呈现（ED-DECOR） | 内联候选 | 3 | 锚失效→放弃本次呈现 |
| 3 | 用户 | 采纳/忽略 | 采纳即作为编辑操作进入编辑模型（经 ED 受控接口） | 内容更新/无操作 | 终态 | 采纳产生的脏态走 SM-ED-01→SM-WS-01 正常路径 |

> A3 的采纳是**编辑操作**（用户显式动作，经 ED），不是 AI 直写文件，与 REQ-CORE-002 裁决语义
> 一致；触发节流与成本控制 AG G2 设计（§7 AG-03）。

## §5 模块级需求声明（REQ-AG-*）

<!-- REQ
req_id: REQ-AG-001
title: 代理编辑回合（编辑模式）
owner_module: AG
status: active
summary: 提供代理编辑多轮回合（编辑模式）：上下文组装、流式回显、回合中断/取消；AI 修改建议交 DE 生成 pending_diff，AG 不直接写盘；默认目标当前文档，用户显式指向可跨文档/新建（2026-07-04 裁决，BR-AG-DATA-003）
source_doc: REQ-CORE-002、APC §4.1
acceptance: 可进行多轮对话并流式回显；可中断/取消；修改建议经 DE 进入 diff 裁决，AG 无直接写盘路径
open_issues: 回合/对话历史持久化模型归 AG G2（联合 WS/SYS，§7 AG-01）；代理回合状态机归 AG G2 M-S
-->

<!-- REQ
req_id: REQ-AG-002
title: agent 工具调用回合
owner_module: AG
status: active
summary: 支持模型发起的工具调用循环（校验注册与权限→执行→结果回注），循环可中断且有上限；文件写入意图必须走 DE diff 路径不得旁路
source_doc: REQ-CORE-002、APC §6（tool-call 核心链）
acceptance: 已注册工具可被调用并回注结果；未注册/越权被拒；循环可中断、有上限；无旁路写盘
open_issues: 工具注册清单与权限边界（尤其写盘类工具）归 AG G2（§7 AG-02）；tool-call 状态机归 AG G2 M-S
-->

<!-- REQ
req_id: REQ-AG-003
title: 模型调用门禁与网关前置
owner_module: AG
status: active
summary: AG 的每一次模型调用（代理/工具回合/局部修改/辅助续写）必经订阅门禁（SM-SYS-01 GatePassed）并经 AI 网关发起；无有效订阅或断网时明确拒绝，无客户端绕过路径
source_doc: REQ-CORE-008、REQ-CORE-009、A-SYS-I-P-03（X-SEQ-001、CLOUD-GATE/INVOKE）
acceptance: 不存在不经 GatePassed 通往模型调用的代码路径；无订阅/断网时 AI 功能明确拒绝且本地功能不受影响
open_issues: 无（契约已由 SYS I-P-03 锁定，AG G2 仅承接引用，不重定义）
-->

<!-- REQ
req_id: REQ-AG-004
title: 上下文组装与注入消费
owner_module: AG
status: active
summary: 组装回合上下文：当前文档内容、REF 解析的 @mention 注入、AST/KB 本地资产；上下文读取一律经 WS 且限定 workspace 内
source_doc: REQ-CORE-006、REQ-CORE-001、APC §4.1
acceptance: @mention 注入可被组装进回合上下文；上下文读取不越出 workspace（X-FORBID-004）
open_issues: 上下文长度预算与裁剪策略归 AG G2；REF/AST/KB 的消费契约随其 G2 联合
-->

<!-- REQ
req_id: REQ-AG-005
title: 辅助续写与局部修改
owner_module: AG
status: active
summary: 承载辅助续写（Autocomplete）与局部修改（InlineAssist）业务回合（2026-06-11 归 AG；2026-07-04 澄清定义）：快捷键手动触发（防抖/防重）发起轻量模型调用（同过门禁）；续写为单候选 50–100 字直接生成；局部修改支持选区/光标两态、弹窗内多轮、对话不持久化；候选经 ED 装饰层呈现，采纳作为用户编辑操作进入编辑模型
source_doc: 2026-06-11 人类归属决策（A-SYS-M-S-01 §3）、REQ-CORE-002/008、A-ED-M-T-01（ED-DECOR）
acceptance: 候选可经 ED 装饰层呈现且采纳/忽略行为正确；每次调用过门禁；无订阅/断网时静默不提供且不打断编辑
open_issues: 触发节流与调用成本策略归 AG G2（§7 AG-03）；autocomplete/inline-assist 状态机归 AG G2 M-S 并回填总图
-->

## §6 模块边界与约束（边界闭合）

- **云薄端硬边界**（APC §4.3/§13）：AG 不持密钥、不内置 provider SDK、不绕过订阅门禁；一切模型
  调用经 SYS I-P-03 契约（CLOUD-AUTH/GATE/INVOKE）。
- **唯一落盘路径**：AG 产出的文件修改必经 DE diff 裁决 → WS 落盘；工具调用不得旁路直写
  （AG G2 以规则/约束锁定）。
- **职责边界**：代理回合归 AG，项目级构建编排归 CBT（构建移交/回落，BR-CBT-STATE-002）；解析归 REF、存储归
  AST/KB、呈现归 ED。
- **状态机边界**（APC §6）：代理回合、tool-call、autocomplete、inline-assist 四条状态化
  核心链均 owner=AG（总图 §3），AG G2 在其 M-S 定义并回填。
- **L4 原料边界**：取材台账 §4 相关 AG/对话原料（P1/P2），原料不为权威。

## §7 G1 问题暴露清单

| 编号 | 分类 | 状态 | 处置 |
|---|---|---|---|
| AG-01 | BOUNDARY_OPEN | 显式延期 | 对话/回合历史的持久化模型（是否入 workspace.db、保留策略）归 AG G2 联合 WS/SYS |
| AG-02 | DESIGN_RISK | 显式延期 | 工具注册清单与权限边界（写盘类工具必须走 diff 路径的强制机制）归 AG G2，与 DE/WS 联合 |
| AG-03 | DESIGN_RISK | 显式延期 | autocomplete 触发节流与调用成本（高频触发 × 门禁/网关延迟与计量）归 AG G2 设计 |
| AG-04 | BOUNDARY_OPEN | 显式延期 | 构建移交/回落边界（与 CBT 的状态机对接 X-INTER-001）联合 CBT G2 |
| AG-05 | ACCEPTANCE_MISSING | 部分关闭 | 定性验收已入 §5；量化指标（首 token 延迟、补全延迟）延 G5 NFR 标定 |

> 术语：回合、工具调用、局部修改等 AG 主责核心概念在 AG L2 评估注册 TERM；订阅门禁/AI网关/会话
> 令牌/BYOK透传已由 SYS 注册（TERM-CLOUD-001~004），AG 引用不重注册。

## §8 后续动作

1. 待人类宣布 AG 模块级 G1 闭合后进入 AG G2：回合模型与历史持久化（解除 AG-01）、工具调用契约
   与权限边界（解除 AG-02）、局部修改触发与成本策略（解除 AG-03）、四条核心链状态机（M-S，回填
   总图 planned 行）、与 DE/CBT/REF 的对接契约。
2. G2 经 REQ_MAP 把 REQ-AG-001~005 映射为 RULE/CHAIN/CONSTRAINT/TERM。
3. DE G1/G2 启动时与 AG 对接「AI 产出→diff 生成」契约（P3 内联动）。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-11 | v1.0 | 初版（P3 首件）：AG 对话代理模块级 L1；声明 REQ-AG-001~005（单文档 chat 回合 / 工具调用回合 / 门禁网关前置 / 上下文组装 / 内联辅助与自动补全），承接 REQ-CORE-001/002/006/008/009 与 2026-06-11 人类归属决策（inline-assist/autocomplete 归 AG）；代表性流程 A1–A3；云薄端硬边界与唯一落盘路径锁定；G1 暴露清单 AG-01~05 |
| 2026-07-04 | v2.0 | 术语沿革澄清回写（人类裁决）：REQ-AG-001 更名「代理编辑回合（编辑模式）」、去单文档限定（默认当前文档+显式指向可跨/新建）；REQ-AG-005 更名「辅助续写与局部修改」（手动触发/防抖、局部修改多轮不持久化、续写单候选 50–100 字）；A-J1/A-J5 职责与边界澄清同步（文件树整理直接执行·删除须确认；构建移交判据=项目级构建意图，原对话窗口实现）；模块正式名「代理（Agent）」 |
