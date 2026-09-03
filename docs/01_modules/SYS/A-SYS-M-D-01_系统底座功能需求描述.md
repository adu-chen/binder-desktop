---
文档编号：SYS-M-D-01
文档状态：A
负责模块：SYS
文档职责：SYS 系统底座模块级功能需求描述——声明模块级 REQ-SYS-*，承接产品级 REQ-CORE-*，界定 IPC 协议 / 跨模块数据结构契约 / 状态机总图 / 模块注册 / 云接口契约 / Agent 文档生命周期与任务链共享边界
上游约束：A-CORE-C-P-01_APC.md（§4.1 SYS 职责、§4.3 云契约、§5 门禁、§6 状态机、§7 流程图）、A-CORE-C-D-01_产品概述.md（REQ-CORE-*）、A-CORE-X-L-01_总体开发路线图.md（P1）、ADU.md（§3.2 REQ 规范）
直接承接：SYS L2 技术设计（A-SYS-C-T-01 架构 / A-SYS-I-P-01 数据结构 / A-SYS-I-P-02 IPC 协议 / A-SYS-M-S-01 状态机总图 / A-SYS-I-P-03 云接口契约）经 REQ_MAP 映射 RULE/CHAIN/CONSTRAINT/TERM
使用边界：本文是 L1 功能需求描述层，仅声明模块级 REQ-SYS-*，**不定义技术规则**（RULE/CHAIN/CONSTRAINT/TERM/STATE_MACHINE 见 SYS L2）；**代码不得直接映射 REQ**（ADU §3.2）；具体接口字段/协议/表结构/状态机枚举归 L2，本文只给契约边界与骨架
变更要求：SYS 职责或云契约边界变更须先经 APC 授权再回写本文；REQ 增删按 ADU §3.2 req_id 规则（序号不复用，废弃改 superseded）
治理成熟度：draft
---

# Binder SYS 系统底座功能需求描述（L1）

## §1 文档定位

本文是 **SYS 模块**的 L1 功能需求描述，处在产品概述（L1 根）之下、SYS L2 技术设计之上：

```
产品概述(REQ-CORE-*) → 本文(REQ-SYS-*) → REQ_MAP → SYS L2 规则(SSOT) → 代码(@GOV)
```

SYS 是被几乎所有客户端模块依赖的底座（APC §4.1、路线图 §3）。本文声明 SYS 自身的模块级
需求 REQ-SYS-*，每条承接一条或多条产品级 REQ-CORE-*；本文不写技术方案、不定义规则、
代码不得追溯本文。具体 IPC 调用签名、数据表结构、状态机枚举、云协议字段一律归 SYS L2，
本文只锁定**契约边界与职责骨架**。未被 APC / 产品概述支撑的判断一律进 §7 暴露清单，不杜撰。

## §2 模块定位与职责

**定位**：SYS 是 Binder 桌面客户端的**架构底座与跨模块契约层**，不直接面向终端用户功能，
而是为 WS/ED/DE/AG/CBT/DOC/PRV/REF/AST/KB/TPL/ACCT/ENT 提供统一的调用协议、共享数据结构、
状态机总图、模块注册与云接口契约边界（APC §4.1）。

**职责清单（v1）**：

| # | 职责 | 锚定 |
|---|---|---|
| J1 | 统一 IPC / Tauri command 协议：前端（React/TS）↔ 后端（Rust）的调用、错误、流式回传契约 | APC §4.1 |
| J2 | 跨模块共享数据结构契约：跨模块传递的实体/类型、workspace.db 持久化 schema 基线 | APC §4.1、§9 |
| J3 | 状态机总图与清单：APC §6 核心链状态机的总图框架与注册入口 | APC §6 |
| J4 | 模块注册与依赖边界：MODULE 注册块、模块依赖图（承接产品概述 PO-03） | APC §12、产品概述 §7 |
| J5 | 云接口契约边界：认证 / 订阅门禁 / AI 网关的客户端侧契约定义（密钥仅服务端、不可绕过） | APC §4.3、§13 |
| J6 | Agent 文档与只读上下文共享边界：LogicalState、提案、未保存文档、AgentTurn、编辑器会话事实与跨模块状态机注册 | REQ-CORE-002/010/011 |

> 边界澄清：SYS 定义云接口**契约**（客户端如何调用云、契约形状），ACCT/ENT 是实现该契约的
> 客户端消费者；云端 AUTH/GW/LIC 由独立 APC 实现，本项目仅在 SYS L2 定义接口契约。

## §3 能力范围（SYS v1）

**IN（SYS 承载）**：
- 前后端 IPC/command 统一协议（含同步调用、流式/事件、统一错误形状）。
- 跨模块共享类型与 workspace.db schema 基线、迁移契约框架。
- 核心链状态机总图与注册（具体状态机由各 owner 模块在其 M-S 定义，SYS 提供总图与跨模块转移契约）。
- 模块编码注册（MODULE 注册块）与依赖关系声明。
- 云接口契约的客户端边界（认证/订阅门禁/网关），保证"密钥仅服务端、门禁不可客户端绕过"在契约层成立。
- Agent 文档生命周期与只读上下文所需共享结构、事实来源隔离和跨模块状态机注册边界；不替 AG/DE/ED/WS/REF 定义其实现细节。

**OUT（非 SYS）**：
- 任何终端用户功能（编辑、对话、预览、记忆等）——归各业务模块。
- 云端服务实现（AUTH/GW/LIC 内部逻辑）——独立 APC。
- 具体接口字段、表结构、状态机枚举、协议报文——归 SYS L2（本文只定边界）。
- NFR 量化指标（IPC 延迟、迁移性能等）——延 G5 标定（APC §9）。

## §4 代表性流程（契约级，APC §7）

SYS 为基础设施，给出其所承载契约的代表性流程表，标注模块边界、人类确认点与异常/终态。

**F1 · IPC 调用契约链**（J1）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | 业务模块(UI/Store) | 调用意图 + 参数 | 经 Service 层发起统一 command 调用 | command 请求 | 2 | 参数不符契约→契约层校验拒绝 |
| 2 | SYS IPC 契约 | command 请求 | 按统一协议路由至 Rust 后端 | 后端调用 | 3 | 未注册 command→协议错误（统一错误形状） |
| 3 | Rust 后端 | 后端调用 | 执行并按契约封装结果/错误/流式分片 | 结果或错误或流 | 4 | 执行失败→统一错误形状回传 |
| 4 | 业务模块 | 结果/错误 | 按契约消费 | 调用完成 | 终态 | 流式中断→契约定义的中断/重试语义 |

**F2 · 云门控调用边界**（J5，承接 REQ-CORE-008）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | ENT 客户端 | 模型调用意图 | 按云契约携带会话凭证发起调用 | 门禁请求 | 2 | 未登录→契约要求先认证（ACCT） |
| 2 | 订阅门禁契约 | 门禁请求 | 服务端校验有效订阅（硬门，客户端无绕过路径） | 校验结果 | 3 | 无有效订阅→拒绝模型调用（终态） |
| 3 | AI 网关契约 | 通过校验的请求 | 服务端持密钥代理模型；套餐计量 / BYOK 透传不计额度 | 流式回传 | 终态 | 网关错误→统一错误形状回传 |

> F2 在 L1 仅锁定边界与硬门顺序；令牌格式、校验响应结构、网关流式协议归 A-SYS-I-P-03（L2）。

**F3 · 状态机注册与总图**（J3）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | owner 模块 | 该模块核心链状态机定义(M-S) | 向 SYS 总图登记 machine_id / 跨模块转移点 | 注册项 | 2 | 缺状态机定义→该链不得进入实现（APC §6、ADP SC-06/07） |
| 2 | SYS 总图 | 注册项集合 | 汇总核心链总图与跨模块转移契约 | 状态机总图清单 | 终态 | 跨模块转移冲突→暴露为 DESIGN_RISK 待裁决 |

## §5 模块级需求声明（REQ-SYS-*）

> owner_module = SYS。每条经 source_doc 标注所承接的产品级 REQ-CORE-* 与 APC 锚点。
> 代码不得直接映射本文 REQ（ADU §3.2）；映射到规则在 SYS L2 经 REQ_MAP 完成。

<!-- REQ
req_id: REQ-SYS-001
title: 统一 IPC / Tauri command 协议
owner_module: SYS
status: active
summary: 定义前端↔Rust 后端的统一调用协议，覆盖同步调用、流式/事件回传与统一错误形状，作为全体跨进程调用的单一契约
source_doc: REQ-CORE-002、APC §4.1
acceptance: 所有跨进程调用经统一 command 协议；存在统一错误形状；流式回传有契约化的中断/完成语义
open_issues: 传输机制选型（invoke vs channel/event 流式）属 L2 技术选型，延 G2
-->

<!-- REQ
req_id: REQ-SYS-002
title: 跨模块共享数据结构与持久化 schema 基线
owner_module: SYS
status: active
summary: 定义跨模块传递的共享实体/类型与 workspace.db 持久化 schema 基线及迁移契约框架，支撑内容提案、LogicalState、WorkspaceOperationPlan、WorkspaceOperationEntry、CreateDocumentBundle、统一 WorkspaceTimeline、AgentTurn、编辑器会话事实和引用快照等跨模块协作
source_doc: REQ-CORE-001/002/007/010/011/012/014、APC §4.1、§9、A-CORE-X-D-02 §3/§4、A-WS-M-T-02
acceptance: 跨模块共享类型有单一契约来源；LogicalState、ProposalState、WorkspaceOperationPlan/Entry、CreateDocumentBundle、AgentTurn/IntentResult/CapabilityGrant、编辑器会话事实与引用快照职责可追溯；内容 Diff 与资源操作状态不得混用；workspace.db schema 有版本化迁移契约；重启后未完成资源操作进入 Interrupted 且不自动执行，partial_failed 收据和 WorkspaceTimeline 恢复载荷可取回
open_issues: 具体字段、表结构、可恢复范围与 WS/AG/DE/ED/REF 协作确定，归 A-SYS-I-P-01 + 各 owner G2
-->

<!-- REQ
req_id: REQ-SYS-003
title: 状态机总图与注册
owner_module: SYS
status: active
summary: 提供 APC §6 核心链状态机的总图框架与注册入口，汇总跨模块转移契约；须覆盖 AgentTurn、内容 Diff 预览/裁决/失效、WorkspaceOperation 前置裁决/重新核验/磁盘执行/补偿、CreateDocumentBundle 及显式保存链路，具体状态机由各 owner 模块在其 M-S 定义
source_doc: REQ-CORE-002/003/010、APC §6、A-CORE-X-D-02 §3/§4
acceptance: 核心链状态机可在总图注册并可追溯 owner；内容 Diff 预览、基准推进失效、裁决与显式保存，以及 WorkspaceOperation 前置裁决、dirty 阻断、fail-fast 补偿、Interrupted 恢复和 CreateDocumentBundle 安全拒绝均有跨模块契约登记；未注册状态机的状态化链路被阻断进入实现
open_issues: 总图清单完整枚举依赖各模块 G2 暴露真实链路，SYS 提供框架并随后回填
-->

<!-- REQ
req_id: REQ-SYS-004
title: 模块注册与依赖边界
owner_module: SYS
status: active
summary: 落地 15 客户端模块的编码注册（MODULE 注册块）与模块依赖图，作为治理负空间扫描与跨模块约束的基线（承接产品概述 PO-03）
source_doc: APC §4.1、§12、产品概述 §7（PO-03）
acceptance: 全部客户端模块有注册块并经 governance:generate 进入 ADUS 模块编码表；模块依赖图与路线图 §3 一致
open_issues: 注册块物理归属 A-SYS-C-T-01；本文仅声明需求
-->

<!-- REQ
req_id: REQ-SYS-005
title: 云接口契约边界（认证 / 订阅门禁 / 网关）
owner_module: SYS
status: active
summary: 定义客户端侧云接口契约边界——认证会话、调用模型前订阅门禁硬门、AI 网关代理；保证密钥仅服务端持有、客户端不持密钥、门禁不可客户端绕过、BYOK 经网关透传不计额度仍需有效订阅
source_doc: REQ-CORE-008、APC §4.3、§13
acceptance: 契约层无客户端持密钥路径；模型调用必经订阅门禁硬门；无有效订阅一律被拒；BYOK 经网关透传且不计额度
open_issues: 令牌/校验/网关流式协议字段属 L2，归 A-SYS-I-P-03（G2）
-->

<!-- REQ
req_id: REQ-SYS-006
title: 前端交互架构与一致性
owner_module: SYS
status: active
summary: 定义客户端前端交互的系统级骨架——应用外壳阶段（欢迎页 / 主工作台）、跨模块界面分区与协作点、前端状态对后端状态机/控制态的投影一致性；前端交互为视图层 SSOT，统一约束前端按图施工，不得各模块各拼一套
source_doc: REQ-CORE-001/002/004/007/009、APC §7（代表性流程）、路线图 §9（UI 板块）
acceptance: 存在统一的前端交互架构与核心流程总图；前端状态不另立权威（投影后端状态机）；欢迎页/工作台切换、对话→diff→裁决、docx 开/编/预览/存等主干交互可端到端追溯
open_issues: 视图层总图归 L2（A-SYS-C-T-02）；UI 视觉规范与组件细节 just-in-time 贴模块推进（路线图 §9.2）
-->

<!-- REQ
req_id: REQ-SYS-007
title: 应用关闭生命周期
owner_module: SYS
status: active
summary: 定义应用退出（窗口关闭/Cmd+Q/Dock 退出/系统关机）前 diff 与未保存内容必须终态化的护栏，以及异常退出（崩溃/强杀）后启动清算语义，避免裁决权或未保存修改在退出前后处于不确定状态
source_doc: REQ-CORE-002、APC §4.1、§9（数据恢复）
acceptance: 正常关闭前存在未裁决 diff 或未保存文档时必须阻断关闭并呈现统一决策入口，完成决策且复核清零后才允许退出；异常退出后下次启动先于会话/历史恢复执行清算，残留 diff 转只读终态、残留未保存内容丢弃并重载磁盘态；提供人类确认的强制退出兜底
open_issues: 无（2026-07-20 人类三轮确认落地，具体规则与状态机归 SYS L2，见 A-SYS-M-T-02/A-SYS-M-S-02）
-->

<!-- REQ
req_id: REQ-SYS-008
title: Agent 文档生命周期共享契约
owner_module: SYS
status: active
summary: 为 Agent 文档协作定义共享边界：提案的基准身份、LogicalState 与 DiskState 的分离、未保存文档元数据、AgentTurn 关联、引用快照、提案失效与显式保存的跨模块事件。实现 owner 仍分别是 AG/DE/ED/WS/REF，SYS 只定义可互操作的契约和总图登记入口
source_doc: REQ-CORE-002/010、A-CORE-X-D-02 §3/§4
acceptance: 跨模块传递必须明确区分三条 Diff 分流：已有 LogicalState 时 pending 直接预览、显式接受只改变卡片生命周期，而保存有效 pending 预览会同时裁决并写盘；无 LogicalState 时 pending 只临时挂载、接受才原子写盘；pending 期间打开目标文档时迁移为已有 LogicalState 分流并保留绿色增量。未保存文档默认不泄露给无关联会话；每个跨模块事件可追溯到 AgentTurn 或用户操作
open_issues: 结构字段、状态枚举、持久化/异常退出策略和 privacy 规则归 SYS 与 AG/DE/ED/WS/REF G2 联合定义
-->

<!-- REQ
req_id: REQ-SYS-009
title: Agent 只读上下文事实与隔离边界
owner_module: SYS
status: active
summary: 为 Agent 1.0 P0 的项目分析、编辑器感知和文档阅读声明跨模块事实边界：工作区概览、当前/打开编辑器 LogicalState、标签脏态/保存基准、任务所需选区、关联 Proposal、显式引用和未打开文档 DiskState 读取均须携带来源与适用范围，并受 workspace、AgentSession 和 AgentTurn 隔离
source_doc: REQ-CORE-011、REQ-AG-009、REQ-ED-011、REQ-WS-002、REQ-REF-002/005
acceptance: 任何只读上下文都可区分来源为当前 LogicalState、显式引用或 DiskState；已打开未保存文档不被磁盘旧内容替代；跨 workspace/AgentSession 不串标签、选区、未保存内容、引用或 Proposal；读取上下文不会自动转化为写授权
open_issues: 共享字段、刷新协议、脱敏策略和持久化边界归 SYS/AG/ED/WS/REF DESIGN Scope；本 L1 不定义 IPC payload 或数据库字段
-->

<!-- REQ
req_id: REQ-SYS-010
title: Agent 运行时共享身份、事件与恢复隔离
owner_module: SYS
status: active
summary: 为 Agent 1.0 声明模型主导、运行时受控的跨模块边界：模型维护可更新 IntentResult、选择工具并依据证据调整路线；运行时维护唯一 Runtime Profile、AgentTurn/AgentSession 身份、事实快照、动态 CapabilityGrant、Gate、预算、副作用、取消与终态。AgentTurn 的 append-only 事件是状态事实源，界面、TaskRun/TaskPlan、ProjectReport 与恢复状态均由事件和工具证据投影。仅 `agent-edit-1.0` 可启动；未知或缺失 profile 在 Gate、provider、工具与 Proposal 前失败关闭；AgentSession 是展示、持久化和恢复隔离边界
source_doc: REQ-CORE-016、REQ-AG-007、A-CORE-X-D-02 §3/§4
acceptance: 共享契约可区分 profile、workspace、AgentSession、AgentTurn、可更新意图、目标候选、副作用、所需上下文、歧义、确认点、CapabilityGrant、工具证据、结构化事件与终态原因；IntentResult 不得包含或等价承载允许工具，任一工具调用都按当时 profile/注册/目标/格式/动态能力/DecisionRequest/Gate/预算/终态重新校验；缺失/未知 profile 不产生 Gate/provider/工具/Proposal 事件；澄清不是完成，取消可从任一活动阶段进入终态，首个终态后迟到事件/工具无副作用；跨 workspace/AgentSession 不串消息、任务事件、工具卡、Diff、引用、未保存上下文或本地事实历史；投影可重建且不包含模型内部推理，恢复不伪造成功或自动续跑
open_issues: Runtime Profile 兼容迁移、状态机枚举和终态锁存、共享字段/事件形状、IPC、数据库 schema、会话管理、历史容量、异常恢复、脱敏 Trace 与跨模块投影归 SYS/AG DESIGN Scope；本 L1 不定义协议或实现
-->

<!-- REQ
req_id: REQ-SYS-011
title: 受限任务链、真实进度与连续 Proposal 共享契约
owner_module: SYS
status: active
summary: 为 Agent 1.0 的复合任务声明共享投影边界：复杂或跨执行片段目标才创建 TaskRun，其 TaskPlan 表达模型当前路线，并与 AgentTurn、工具证据、等待点和 ProposalBatch 关联；事件事实投影真实执行、等待、中断与终态。TaskRun/TaskPlan 不是静态调度器、未来工具授权或后台队列，模型可根据新证据更新未执行路线但不能跨确认点自动续跑。连续同文档 Proposal 以当前 LogicalState 接力，前卡因基准推进失效；background pending Proposal 未建立 LogicalState 前不得作为后续编辑输入
source_doc: REQ-CORE-017、REQ-AG-015、REQ-DE-009、A-CORE-X-D-02 §3/§4
acceptance: 共享契约可追溯复杂用户目标、模型当前路线、路线修订、AgentTurn、真实工具证据、ProposalBatch/DiffCard、确认点及执行/等待/中断/终态原因；TaskPlan 只能由模型表达和 append-only 事件投影更新，不能触发未由当前模型调用选择的工具或授予未来权限；仅活跃 provider/工具事件可声明工作中，等待补充、卡片 pending、失败、取消或完成不伪装执行；连续 Proposal 不使用旧 payload 作为新基准，后预览可使前卡失效，background pending 必须迁移或裁决后才可继续；中断重启不自动续跑，用户重试产生新 AgentTurn 并重新校验；跨 workspace/AgentSession 的任务、事件、卡片、引用和未保存上下文不串线
open_issues: TaskRun/步骤/Proposal 链的身份与状态、事件字段、IPC、数据库 schema、并发、恢复/重试、Trace、实际工作检测和 UI 投影归 SYS/AG/DE DESIGN Scope；TaskRun 正式 TERM 注册待 L2；本 L1 不定义队列、接口、状态机或实现
-->

<!-- REQ
req_id: REQ-SYS-012
title: 外部内容与项目记忆的隔离和保留契约
owner_module: SYS
status: active
summary: 为 Agent 1.0 的 P1 外部文件内容引用与用户管理项目记忆声明共享隔离边界：外部内容只在用户显式附加、存在可用只读解析路径后的当前 AgentTurn/TaskRun 当前任务中以 data-only 存在；任务结束、重启、跨 AgentSession 或跨 workspace 后不自动恢复、复读或转存。项目记忆是本地 workspace 隔离的用户资产，只有用户明确选择才由 REF 进入当前回合；Agent 1.0 不自动写入、提议、检索或注入
source_doc: REQ-CORE-018/019、REQ-AG-016/017、REQ-REF-009、A-AST-M-D-01（REQ-AST-001~003）
acceptance: 共享契约可区分外部文件元数据引用、已读/未读内容引用、当前任务范围和用户显式记忆选择；外部内容与记忆正文均为 data-only，不能改变系统约束、任务目标、工具授权、编辑目标或确认点；不支持/失败外部读取不伪称已读；跨 workspace/AgentSession 不串外部内容、记忆条目或选择事实；普通发送、附件、恢复不会自动创建/提议/注入项目记忆，外部内容不会自动成为记忆
open_issues: 可读格式能力、快照/暂存/清除、来源/选择令牌、大小/上下文预算、传输提示、脱敏 Trace、关闭/删除传播、事件、IPC、schema、状态机和 UI 归 SYS/AG/REF/AST DESIGN Scope；本 L1 不定义接口或实现
-->

## §6 模块边界与约束（边界闭合）

- **职责边界**：SYS 只承载架构契约与底座（§2 J1–J5），不实现任何业务功能；越界即范围外（CLAUDE.md §1 范围纪律）。
- **L1/L2 边界**：本文只定契约**边界**；接口字段、表结构、状态机枚举、协议报文一律归 SYS L2，代码只追溯 L2（ADU §3.2、APC §2）。
- **安全/计费边界**（REQ-SYS-005）：密钥仅服务端；订阅门禁客户端不可绕过；客户端不内置 provider SDK（APC §4.3、§13）。
- **云/客户端边界**：SYS 定义客户端侧契约，不实现云端逻辑（AUTH/GW/LIC 独立 APC）。
- **状态机边界**（REQ-SYS-003）：状态化核心链必须先有状态机定义方可实现，SYS 不替各模块定义其状态机，只提供总图与跨模块转移契约（APC §6、ADP SC-06/07）。
- **Agent 运行时边界**（REQ-SYS-010）：SYS 只定义 profile/turn/session/intent/grant/事件/恢复的共享契约，不以 workspace 代替 AgentSession 隔离，也不以兼容默认值扩大 `agent-edit-1.0`；模型内部推理、长期记忆自动写入和具体运行时实现不属于 SYS L1。
- **Agent 任务链边界**（REQ-SYS-011）：SYS 只定义 TaskRun/TaskPlan、回合、ProposalBatch、工具证据和进度的共享投影契约，不把它实现为静态调度器、未来授权、后台队列、任务级事务或自动跨重启续跑；连续 Proposal 的真值是当前 LogicalState，模型内部推理和具体进度 UI 不属于 SYS L1。
- **外部内容与记忆边界**（REQ-SYS-012）：SYS 只定义当前任务外部内容与 workspace 项目记忆的隔离/保留契约；外部内容不因会话恢复变成可复读资产，项目记忆不因附件、历史或相似任务自动产生/注入。格式、暂存、存储、事件、状态机和 UI 不属于 SYS L1。
- **L4 原料边界**：取材见台账 §4.2（SYS）；旧 binder 文档为原料不为权威，REQ 重新撰写（APC §8）。云契约部分无 L4 原料（greenfield，台账 §4.15）。

## §7 G1 问题暴露清单

按 APC §5 G1 标准（REQ_GAP/BOUNDARY_OPEN/REQ_CONFLICT/FLOW_INCOMPLETE/ACCEPTANCE_MISSING/
NEEDS_HUMAN_DECISION/DESIGN_RISK）。各项均**显式延期**至 G2/G5 并记明 owner+门禁，据此 SYS
模块级 G1 具备闭合条件。晋级由人类宣布（APC §2，AI 不得自行宣布晋级）。

| 编号 | 分类 | 状态 | 处置 |
|---|---|---|---|
| SY-01 | DESIGN_RISK | 显式延期 | 云接口契约具体协议（令牌/订阅校验响应/网关流式）属 L2，延期至 A-SYS-I-P-03（G2） |
| SY-02 | NEEDS_HUMAN_DECISION | 显式延期 | IPC 传输机制选型（invoke vs channel/event 流式）属 L2 技术选型，延期 A-SYS-I-P-02（G2） |
| SY-03 | BOUNDARY_OPEN | 显式延期 | workspace.db 具体表结构与 WS 协作，归 A-SYS-I-P-01 + WS G2 联合确定 |
| SY-04 | FLOW_INCOMPLETE | 显式延期 | 状态机总图完整清单依赖各模块 G2 暴露真实链路；SYS 提供框架，随各模块 G2 回填 |
| SY-05 | ACCEPTANCE_MISSING | 部分关闭 | 定性验收已入 §5 各 REQ-SYS-*；量化指标（IPC 延迟、迁移性能）延 G5 NFR 标定（APC §9） |
| SY-06 | FLOW_INCOMPLETE | 部分关闭 | REQ-SYS-008 所涉共享结构与状态机已同步 BR-CORE-STATE-001~003；运行时代码与跨模块自动化验证仍须按三分流收口，未验证前不得宣称全链闭合 |
| SY-07 | FLOW_INCOMPLETE | 已识别，待 G2 | REQ-SYS-010 的 profile 强制入参、AgentTurn 等待澄清/全活动态取消/首终态、IntentResult/CapabilityGrant/AgentSession 的共享字段、结构化事件、持久化恢复与现有 workspace 级历史迁移均待 SYS/AG DESIGN Scope；不得以旧兼容或 UI 表现替代。 |
| SY-08 | FLOW_INCOMPLETE | 需求已收敛，待 G2 | REQ-SYS-010/011 的 append-only 事件、TaskRun/TaskPlan 投影、ProposalBatch、动态 CapabilityGrant、真实工作状态、确认点、中断/新回合重试、并发与恢复待 SYS/AG/DE DESIGN Scope；不得以静态调度、未来预授权、后台队列、静态 spinner 或旧 payload 续编替代。 |
| SY-09 | DESIGN_RISK | 已识别，待 G2 | REQ-SYS-012 的外部内容可读能力、当前任务快照/清除、来源与记忆选择、跨会话/重启隔离、关闭/删除传播与脱敏 Trace 待 SYS/AG/REF/AST DESIGN Scope；不得把附件或历史隐式持久化/注入为项目记忆。 |

## §8 后续动作

1. 待人类宣布 SYS 模块级 G1 闭合后，进入 SYS G2（L2 技术设计，规则 SSOT），按路线图 §6.2：
   - `A-SYS-I-P-03` 云接口契约（**优先**：greenfield + 商业化关键路径，承接 REQ-SYS-005 / REQ-CORE-008）；
   - `A-SYS-C-T-01` 系统总体架构（含 MODULE 注册块，承接 REQ-SYS-004）；
   - `A-SYS-I-P-01` 数据结构定义（承接 REQ-SYS-002/008/010/011/012）；
   - `A-SYS-I-P-02` IPC / Tauri command 协议（承接 REQ-SYS-001）；
   - `A-SYS-M-S-01` 状态机总图与清单（承接 REQ-SYS-003/008/010/011/012）。
2. G2 各文档经 REQ_MAP 把 REQ-SYS-* 映射为 RULE/CHAIN/CONSTRAINT/TERM；状态机入 M-S。
3. SYS 职责或云契约边界变更先经 APC 授权再回写本文（APC §13）。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-10 | v1.0 | 初版：SYS 底座模块级 L1；声明 REQ-SYS-001~005（IPC 协议 / 共享数据结构 / 状态机总图 / 模块注册 / 云接口契约边界），承接 REQ-CORE-002/008 等；代表性契约流程表 F1–F3；边界闭合与 G1 暴露清单（SY-01~05 显式延期/部分关闭） |
| 2026-07-20 | v1.1 | 新增 REQ-SYS-007 应用关闭生命周期（承接 REQ-CORE-002，APP-CLOSE-DIFF-LIFECYCLE-01 三轮人类确认）：正常关闭前 diff/未保存内容必须终态化护栏、异常退出后启动清算、强制退出兜底；规则与状态机归 A-SYS-M-T-02/A-SYS-M-S-02 |
| 2026-08-13 | v1.2 | 依据 REQ-CORE-010 与 Agent 1.0 能力基线，扩展 REQ-SYS-002/003，并新增 REQ-SYS-008：系统层承接 LogicalState/DiskState、提案、未保存文档、AgentTurn 和引用快照的共享契约及状态机登记；既有“accepted 直接写盘”设计列为下一轮同步项。 |
| 2026-08-13 | v1.3 | DIFF-STATE-COMPASS-01：REQ-SYS-008 验收改为精确覆盖 BR-CORE-STATE-001~003 的三条分流，并将 SY-06 更新为“设计同步完成、运行时与自动化验证待收口”。 |
| 2026-08-13 | v1.4 | AG-1.0-R2-REQUIREMENTS-01：扩展 REQ-SYS-002，并新增 REQ-SYS-009，冻结 Agent 只读上下文的来源、新鲜度与 workspace/AgentSession/AgentTurn 隔离边界；不定义 L2 字段或协议。 |
| 2026-08-14 | v1.5 | AG-1.0-R5A-RUNTIME-REQUIREMENTS-01：新增 REQ-SYS-010，承接唯一 `agent-edit-1.0`、缺失/未知 profile 失败关闭、AgentTurn 澄清/取消/首终态、IntentResult/CapabilityGrant、非推理可见事件、AgentSession 持久化隔离和本地事实历史；具体状态机、字段、IPC 与迁移留待 DESIGN Scope。 |
| 2026-08-14 | v1.6 | AG-1.0-R5B-TASK-ORCHESTRATION-REQUIREMENTS-01：新增 REQ-SYS-011，承接受限 TaskRun、步骤/回合/Proposal 关联、真实工作状态、连续 Proposal 的 LogicalState 接力与前卡失效、中断新回合重试和隔离；队列、事务、自动续跑及字段/状态机留待 DESIGN Scope。 |
| 2026-08-14 | v1.7 | AG-1.0-R5C-MEMORY-EXTERNAL-READ-REQUIREMENTS-01：新增 REQ-SYS-012，承接外部文件一次性 data-only 内容引用的当前任务/会话/重启隔离，以及项目记忆的 workspace 本地、显式选择、无自动写入/提议/注入边界；具体格式、暂存、字段与状态机留待 DESIGN Scope。 |
| 2026-08-21 | v1.8 | 工作区资源操作基线重构：扩展 REQ-SYS-002/003，纳入 WorkspaceOperationPlan/Entry、CreateDocumentBundle、统一 WorkspaceTimeline、SM-WS-04 前置裁决/补偿与重启 Interrupted/partial_failed 跨模块边界。 |
| 2026-09-03 | v1.9 | AG-ADAPTIVE-RUNTIME-REQ-01：REQ-SYS-010/011 收敛为 append-only 事件事实源、运行时动态授权和可重建投影；TaskRun/TaskPlan 不再承担静态调度或未来授权职责。 |
