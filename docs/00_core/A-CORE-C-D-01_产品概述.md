---
文档编号：CORE-C-D-01
文档状态：A
负责模块：CORE
文档职责：Binder 产品概述——产品级功能需求描述与边界，L1 需求层根文档，向各模块 M-D 需求文档分发产品级 REQ
上游约束：A-CORE-C-P-01_APC.md、A-CORE-X-L-01_总体开发路线图.md、ADU.md（§3.2 REQ 规范）
直接承接：各模块 L1 功能需求描述文档（M-D）；产品级 REQ 经各模块 REQ_MAP 进入 L2 技术规则
使用边界：本文是 L1 功能需求描述层，仅声明产品级 REQ-CORE-*，**不定义技术规则**（RULE/CHAIN/CONSTRAINT/TERM 见 L2 技术设计）；**代码不得直接映射 REQ**（ADU §3.2）；功能级详细流程（入口/输入输出/主流程/分支/异常/终态 + 流程图）归各模块 M-D 文档，本文只给产品级骨架与边界
变更要求：产品定位/范围变更须先经 APC 授权再回写本文；REQ 增删按 ADU §3.2 req_id 规则（序号不复用，废弃改 superseded）
治理成熟度：draft
---

# Binder 产品概述（L1 功能需求根）

## §1 文档定位

本文是 L1 功能需求描述层的**产品级根文档**。它回答"产品要做什么、为谁、边界在哪"，
并声明产品级 REQ-CORE-*。各模块在其 M-D 需求文档中声明模块级 REQ-[MODULE]-*，承接本文的
产品级 REQ；模块级 REQ 再经 REQ_MAP 映射到 L2 技术规则（RULE/CHAIN/CONSTRAINT/TERM）。

```
产品概述(本文, REQ-CORE-*) → 模块 M-D(REQ-[MODULE]-*) → REQ_MAP → L2 规则(SSOT) → 代码
```

边界：本文不写技术方案、不定义规则、代码不得追溯本文。权威来源是 APC（已人类确认的宪章）；
本文每条 REQ 均锚定 APC 条款，未被 APC 支撑的产品判断一律进 §7 暴露清单，不擅自杜撰。

## §2 产品定位与目标用户

**定位**（锚 APC §1）：Binder 是 AI 原生的桌面写作应用，商业化上线，Open Core（当前不开源），
首发 macOS，经官网门户 DMG 分发，不走 Mac App Store。

**核心理念**：本地优先 + 人类把关。用户内容（文档、记忆、模板、聊天历史、知识库、引用）
全部本地存储，workspace 是用户选定的本地目录，是文件操作/搜索/diff/上下文读取的边界；
AI 的所有文档修改以提案（diff）形式呈现，用户始终保有裁决权，且只有显式保存才写入文件。

**云端职责（薄）**：账号/认证、订阅/计费、AI 网关；不承载用户内容、不做内容云同步。

**目标用户**（2026-06-09 人类确认）：**v1 主用户群 = 以 AI 协同进行严肃/专业写作与多文件文档构建的
个人专业写作者**（本地内容主权敏感、愿为 AI 写作能力付费）。次级分群、画像优先级与典型任务深度
**显式延期至 G1 后**补全（可挖 L4 原料：台账 §4.1 产品定义总纲、使用场景总览/场景边界清单），
不阻断产品 G1。

## §3 能力范围（v1）

范围已于 2026-06-09 经人类确认冻结（APC §1、路线图 §2）。

**v1 IN（按模块归属，需求粒度）：**

| 能力 | 归属模块 | 产品级 REQ |
|---|---|---|
| 本地 workspace 写作与文件 I/O、pending_diffs 持久化 | WS | REQ-CORE-001 |
| 对话生成 diff → 按 LogicalState 分流裁决/保存或接受落盘（核心交互） | DE/AG/ED/WS | REQ-CORE-002 |
| 代理编辑（AgentEdit：编辑/构建双模式）+ 工具调用 | AG | REQ-CORE-002 |
| 构建：项目蓝图与多文件文档项目构建（CBT） | CBT | REQ-CORE-003 |
| 原生 DOCX 只读预览/分析、BDocx 处理副本编辑、Excel/PPT/音视频**预览** + 打印/导出 | DOC/ED/PRV | REQ-CORE-004 |
| 文本与 Markdown 的受控副本转换（TXT→Markdown、Markdown→TXT） | AG/DE/DOC/ED/WS | REQ-CORE-013 |
| 本地记忆 / 知识库 / 模板工作流 | AST/KB/TPL | REQ-CORE-005 |
| @mention 引用解析与注入 | REF | REQ-CORE-006 |
| 用户显式附加的外部文件内容读取 | REF/AG/SYS | REQ-CORE-018 |
| 用户管理的本地项目长期记忆 | AST/REF/AG/SYS | REQ-CORE-019 |
| 统一 WorkspaceTimeline：文档历史与资源操作历史回溯 | WS | REQ-CORE-007/014 |
| 账号认证 + 订阅门控 + AI 网关（云薄端） | ACCT/ENT + 云 | REQ-CORE-008 |
| 离线边界（本地功能离线可用，AI 在线鉴权+有效订阅） | 跨模块 | REQ-CORE-009 |
| 文档逻辑状态优先、CreateDocumentBundle 新建生命周期 | AG/DE/ED/WS/SYS | REQ-CORE-010 |

**v1 OUT（明确不做，APC §1 + 本次确认）**：多人实时协同、内容云同步、插件市场、
脱离写作场景的通用聊天助手、**Excel/PPT 编辑**。

## §4 核心使用场景与主流程（产品级）

产品级仅给代表性主流程骨架；功能级详细流程图归各模块 M-D（APC §7）。

**S1 代理编辑落盘（核心交互；旧称「对话编辑」，2026-07-04 更名）** — Flow Table：

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | 用户 | 自然语言指令 | 在 workspace 内对目标文档发起对话修改 | 修改请求 | 2 | 无 workspace→提示选目录 |
| 2 | AG | 修改请求 + 文档上下文 | 调模型（经订阅门禁→网关）生成修改 | 修改草案 | 3 | 无有效订阅→拒绝并提示 |
| 3 | DE/ED | 修改草案 | 若目标已有 LogicalState，受控预览直接投影内容；若无，则 DiffCard 临时挂载内容 | 可编辑预览 + 卡片，或后台待裁决卡片 | 4 | 生成或预览失败→错误态可重试 |
| 4 | 用户 | 接受/拒绝，或在 pending 时打开目标文档 | 已有 LogicalState 时显式接受只裁决卡片；无 LogicalState 时接受触发受控落盘；打开动作使其建立带绿色增量的 LogicalState 并迁移到前者 | 接受、拒绝、失效或迁移决定 | 5 | 超时/外部变更/后续编辑→diff 过期终态 |
| 5 | 用户/WS | 显式保存（仅已有 LogicalState 分支） | 将当前文档逻辑状态写入文件并持久化；仍有效的 pending 卡由保存一并裁决为接受 | 文件落盘 + 历史节点 + 已接受卡片 | 终态 | 写入竞态或失败→保留可编辑逻辑状态并提示 |

**S1a 新建文档（S1 的特例）**：系统在确认前生成 WorkspaceOperationPlan 与初始内容，形成
CreateDocumentBundle；用户通过 DecisionRequest 前置授权后，WS 重新核验并创建空的目标磁盘文件，ED 自动打开
标签，DE/ED 再把初始内容作为普通 Diff 投影。初始 Diff 接受只改变卡片生命周期，保存写入既有文件；仅在
资源、DiskState、LogicalState 基线均未推进时，拒绝可补偿删除新建文件并关闭标签，否则卡片失效且文件保留。

**S2 多文件文档构建（CBT）**：用户描述目标 → agent 规划并跨文件构建 → 可中断/接管 →
产物写入 workspace（构建状态机，APC §6）。详见 CBT 模块 M-D/M-S。

**S3 原生 DOCX 预览与创建编辑副本**：打开原生 DOCX → 只读预览与分析；如用户请求编辑 →
通过 CreateDocumentBundle 创建新的 BDocx 编辑副本 → 在 HTML 编辑器中投影初始内容 Diff 并以模拟 Word 分页编辑 → 保存为新的 DOCX；
原文件始终不被编辑或写回。Excel/PPT/音视频仍只有预览与输出。

**S3a 最小格式转换副本**：用户请求 `TXT → Markdown` 或 `Markdown → TXT` → 系统读取打开来源的
LogicalState，或读取未打开来源的 DiskState 并标明读取时点 → 生成标明输入/输出格式、目标路径与保真/损失的
CreateDocumentBundle → 前置确认后创建目标副本并打开标签 → 投影转换结果内容 Diff → 用户裁决并保存。
来源始终不被改写；Markdown→TXT 必须明示有损。HTML 互转与通用 DOCX 转换不在本路径。

**S4 商业化门控**：用户经官网门户购买订阅 → 桌面端登录 → 调 AI 前校验有效订阅（硬门）→
经网关计量计费。密钥仅服务端持有，客户端不持密钥、不可绕过门禁。

## §5 产品级需求声明（REQ-CORE-*）

> 以下为产品级 REQ，owner_module 统一记 CORE（产品核心所有）；落到具体模块时由该模块 M-D
> 声明模块级 REQ-[MODULE]-* 承接，并 REQ_MAP 入 L2。代码不得直接映射本文 REQ（ADU §3.2）。

<!-- REQ
req_id: REQ-CORE-001
title: 本地优先存储与 workspace 边界
owner_module: CORE
status: active
summary: 用户内容全部本地存储，workspace 是文件操作/搜索/diff/上下文读取的唯一边界
source_doc: APC §1
acceptance: 用户内容文件均位于本地 workspace，无内容上行云端（可经网络/存储位置核验）
-->

<!-- REQ
req_id: REQ-CORE-002
title: 对话生成文档提案并由人类裁决
owner_module: CORE
status: active
summary: AI 文档修改以 diff/ChangeProposal 呈现并由用户接受或拒绝；受控预览可更新可编辑的文档逻辑状态，显式接受不重复应用或直接写盘，而保存有效 pending 预览等同接受并写盘；含代理编辑（AgentEdit，编辑/构建双模式）与工具调用（2026-07-04 术语更名，能力细化经 REQ-AG-* 承接）
source_doc: APC §1、A-AG-M-D-02 §2（已确认文档生命周期基线）
acceptance: AI 修改在用户显式保存前不写入文件；接受不重复应用预览内容；拒绝仅在原始基准仍有效时撤销对应预览；文档逻辑状态已推进时原提案失效而非覆盖后续编辑
-->

<!-- REQ
req_id: REQ-CORE-003
title: agent 多文件文档项目构建（CBT）
owner_module: CORE
status: active
summary: 支持 agent 跨多文件构建文档项目（对话阶段→项目蓝图→显式触发→计划确认→执行），含构建中断与构建移交/回落（旧称 Chat↔Build 接管，2026-07-04 更名）
source_doc: APC §1、§4.1
acceptance: agent 可跨≥2 文件构建产物并写入 workspace，构建过程可中断且可接管
-->

<!-- REQ
req_id: REQ-CORE-004
title: 多格式预览、原生 DOCX 只读与 BDocx 编辑副本
owner_module: CORE
status: active
summary: 原生 DOCX 仅可本地预览、读取和分析，不能直接编辑或原地保存；编辑请求必须以 Proposal 创建新的 BDocx 编辑副本。BDocx 是处理路径而非文件格式：副本磁盘格式仍为 DOCX，编辑时转为 HTML 并以模拟 Word 分页呈现，保存时再转换为 DOCX。Excel/PPT/音视频仅预览、打印与导出；不含 Excel/PPT 编辑
source_doc: APC §1、路线图 §2、2026-08-13 人类 R3 确认
acceptance: 原生 DOCX 可只读预览、读取和分析，且不存在对其直接编辑/保存入口；编辑请求只产生以新副本为目标的 Proposal，原文件不被改写；副本在预览成功后才进入可编辑 LogicalState，显式保存后生成新的 DOCX；Excel/PPT/音视频可预览并可打印/导出，且 Excel/PPT 无编辑入口
-->

<!-- REQ
req_id: REQ-CORE-005
title: 本地智能资产（记忆/知识库/模板）
owner_module: CORE
status: active
summary: 记忆协同、知识库、模板工作流均本地存储与运行
source_doc: APC §4.1
acceptance: 记忆/知识库/模板数据存于本地且离线可读
-->

<!-- REQ
req_id: REQ-CORE-006
title: 引用解析与上下文注入（@mention）
owner_module: CORE
status: active
summary: 用户可通过选区、工作区文档/标签、会话标签等显式引用向 Agent 提供可追溯的 data-only 上下文；引用在发送时冻结身份、锚、范围和版本，不自动获得编辑权限。外部资料与外部文件读取不属于本要求的 P0 范围
source_doc: APC §4.1
acceptance: 当前选区、工作区文档/编辑器标签和会话标签可形成可见引用令牌；发送后的回合可回看来源身份；失效锚明确反馈；引用内容不得改变系统约束、任务目标或写入权限
-->

<!-- REQ
req_id: REQ-CORE-007
title: 文档级时间轴与历史回溯
owner_module: CORE
status: active
summary: 提供文档级版本时间轴与历史回溯/还原（v1 范围，归 WS）
source_doc: APC §1（2026-06-09 范围确认）、路线图 §2
acceptance: 文档具可浏览的历史节点，可回溯并还原到先前版本
-->

<!-- REQ
req_id: REQ-CORE-008
title: 账号认证、订阅门控与 AI 网关（云薄端）
owner_module: CORE
status: active
summary: 登录/订阅校验为调用模型硬门；密钥仅服务端持有，客户端不持密钥、不可绕过门禁；套餐经网关计量，BYOK（v1 GA 纳入）经网关透传不计额度但仍需有效订阅
source_doc: APC §1、§4.3
acceptance: 无有效订阅时模型调用被拒；客户端不存明文密钥；门禁无客户端绕过路径；BYOK 经网关透传且不计额度
-->

<!-- REQ
req_id: REQ-CORE-009
title: 离线边界
owner_module: CORE
status: active
summary: 编辑/预览/打印/本地功能离线可用；AI 功能需在线鉴权 + 有效订阅
source_doc: APC §9
acceptance: 断网时编辑/预览/打印/本地功能可用；AI 功能在断网或无有效订阅时明确拒绝
-->

<!-- REQ
req_id: REQ-CORE-010
title: 文档逻辑状态优先与新建文档绑定生命周期
owner_module: CORE
status: active
summary: 普通文档内容修改仍首先作用于 LogicalState 并遵守内容 Diff 后置裁决。Agent 新建文档是受前置裁决的 WorkspaceOperation：初始内容在确认前生成并冻结，用户确认后先创建 DiskState 空文件并记录资源操作历史，再打开标签、建立 LogicalState 并投影绑定初始内容 Diff。绑定初始 Diff 遵守普通内容 Diff 生命周期；只有文件身份、空文件磁盘基线和 LogicalState 版本均未推进时，拒绝才可补偿删除本次创建文件
source_doc: 2026-08-21 人类确认的 WorkspaceOperation 八项决策、A-AG-M-D-02 §5.10
acceptance: 未确认或初始内容生成失败时不创建文件；确认后文件立即存在且初始内容以独立 DiffCard 审阅；接受只裁决内容卡、保存才写入初始内容；安全拒绝会删除本次创建文件并记录补偿历史，任何人工/后续 Agent/外部/路径基线推进都会使旧卡失效并禁止删除；普通内容 Diff 永远不能删除文件
-->

<!-- REQ
req_id: REQ-CORE-011
title: Agent 1.0 的项目与文档上下文感知
owner_module: CORE
status: active
summary: Agent 1.0 P0 必须支持基础聊天/任务交互、工作区概览与可复核项目分析、当前编辑器状态感知，以及当前文档、显式引用和未打开工作区文档的受限阅读与比较。项目分析默认先读取结构概览，正文仅在任务需要时按最小范围读取；已打开文档以 LogicalState 为准，未打开文档才读取 DiskState 并标明读取时点/新鲜度
source_doc: 2026-08-13 人类确认、A-AG-M-D-02 §5.1~§5.5、CAP-01~04
acceptance: 回答、分析、Proposal 与文件操作具有可区分结果，取消/失败不伪装完成；项目报告披露已读/未读范围、来源与不确定性；Agent 可感知活动文档、打开标签、脏态、保存基准、任务所需选区和关联提案；M1 多文档读取/比较与 M2 当前文档修改、其他文档显式引用为 P0，多个既有文档写入 M4 保持 P1
-->

<!-- REQ
req_id: REQ-CORE-012
title: 工作区资源操作的前置裁决与磁盘提交
owner_module: CORE
status: active
summary: 创建文档/文件夹、重命名、移动、删除和恢复统一属于 WorkspaceOperation。系统先只读统计目标、影响和基线并冻结 WorkspaceOperationPlan，由 DecisionRequest 前置裁决；用户确认且执行前再核验成功后，WS 直接改变 DiskState，不生成资源 DiffCard或再次等待接受。批量操作采用失败即停和反向补偿，不承诺文件系统级严格事务
source_doc: 2026-08-21 人类确认的 WorkspaceOperation 八项决策、A-AG-M-D-02 §5.11（CAP-11/12）
acceptance: 未确认、拒绝、过期或再核验失败均零磁盘副作用；确认只执行一次且重复事件幂等；卡片显示完整目标、顺序、递归范围、内容 Diff/标签影响和失败策略；dirty 目标先阻断；资源操作成功后文件树、标签、引用和关联内容 Diff 状态同步；批量失败逐项记录执行与补偿结果，补偿不完整不得伪报成功
-->

<!-- REQ
req_id: REQ-CORE-013
title: 最小格式转换与目标副本生命周期
owner_module: CORE
status: active
summary: Agent 1.0 P1 仅支持 TXT→Markdown 与 Markdown→TXT 的受控格式转换，转换一律创建新的目标副本且不改写来源。已打开来源读取当前 LogicalState，未打开来源读取 DiskState 并标明读取时点；预览成功后建立目标 LogicalState 与标签，接受只裁决转换 Proposal，显式保存当前目标才写入目标 DiskState。Markdown→TXT 为有损转换，必须明示损失边界
source_doc: 2026-08-14 人类 R4-C 确认、A-AG-M-D-02 §5.13（CAP-13）
acceptance: 转换 Proposal 展示来源身份/状态时点、输入输出格式、目标路径及保真或损失说明；来源是转换时刻的冻结输入，后续来源变化不自动刷新或回写已预览目标；TXT→Markdown 不自动猜测标题、列表或任务等语义；Markdown→TXT 不承诺样式、结构、表格、图片或可逆往返；预览前失败不产生目标逻辑文档或文件，目标后续编辑使旧转换卡失效，保存冲突/失败不覆盖既有目标且保留目标 LogicalState 与标签；HTML 互转、通用 DOCX 互转、批量或原地转换不提供入口，BDocx 仍仅是 REQ-CORE-004 的专用编辑副本处理路径
-->

<!-- REQ
req_id: REQ-CORE-014
title: 工作区时间轴中的资源删除与恢复
owner_module: CORE
status: active
summary: 删除与恢复是 WorkspaceOperation：完整影响清单和资源快照在确认前冻结，DecisionRequest 前置裁决后重新核验并直接改变 DiskState。产品不建设第二套撤销入口，而将现有文档时间轴扩展为统一 WorkspaceTimeline；资源节点保存前后路径、目录清单、内容/元数据快照和逐项执行回执，由类型化还原器前进式恢复。永久清理历史快照不属于 Agent 1.0
source_doc: 2026-08-21 人类确认的 WorkspaceOperation 八项决策、A-AG-M-D-02 §5.11.1（CAP-12）
acceptance: 删除确认卡显示完整目标、递归数量、可恢复性和受影响会话；确认时来源或清单变化则卡片过期且不删除；dirty 会话先保存或明确放弃；删除成功生成可取回的 WorkspaceOperationEntry，恢复仍需新的前置裁决且不覆盖冲突目标；文件、空目录和非空目录均可由统一时间轴恢复；新建文档绑定初始 Diff 的安全拒绝按 REQ-CORE-010 记录补偿资源节点
open_issues: 历史快照容量和永久清理策略后置；不阻断 1.0 的追加式资源历史与类型化恢复
-->

<!-- REQ
req_id: REQ-CORE-015
title: 多个既有文档的受控独立编辑
owner_module: CORE
status: active
summary: Agent 1.0 P1 可修改用户明确选定或确认候选清单后的多个既有、已保存且可编辑文档；目标清单在确认后冻结，不得动态扩展。每个目标独立生成 Proposal、基准、预览、裁决、保存与失败结果，并逐目标遵循 LogicalState/DiskState 三分流；不提供批量接受、批量保存、跨文档全有或全无事务。范围仅含 TXT、Markdown、HTML 与 BDocx 编辑副本，不含原生 DOCX、删除/文件树整理/格式转换混合操作
source_doc: 2026-08-14 人类 R4-D 确认、A-AG-M-D-02 §5.14（CAP-15）
acceptance: 用户能在写入前看见并确认冻结目标列表及每项范围；任一目标的冲突、外部变化、结构变化或删除只使该目标 Proposal 失效，不静默跳过、误写或阻断其他目标；每项打开文档的预览直接更新其 LogicalState、接受只裁决卡片、保存才写盘，背景目标只在其自身卡片接受时原子落盘且打开后迁移为前者；每个目标的失败、未保存状态与保存结果可独立追溯
open_issues: 目标清单卡、用户选择/候选确认交互、跨文档计划表达、独立 Proposal/基准/失效事件、每项保存队列和 Trace/恢复契约归 AG/DE/ED/WS/REF/SYS DESIGN Scope；本 L1 不定义批处理接口、并发模型或事务实现
-->

<!-- REQ
req_id: REQ-CORE-016
title: Agent 运行时、意图授权与会话隔离
owner_module: CORE
status: active
summary: Agent 1.0 P0 只能以 agent-edit-1.0 启动；未知或缺失运行配置必须在模型、工具与 Proposal 前失败关闭，Build/CBT 保持 P2。每个用户发送对应一个 AgentTurn，澄清为等待用户输入的非终态，取消可在任一活动阶段生效，首个终态后迟到事件/工具不得产生副作用。每轮 IntentResult 固定任务类型、目标、预期副作用、所需上下文、允许工具和确认点；写入目标或范围不清时只能澄清或生成目标计划。用户只见结构化任务事件，不见模型内部推理；workspace/AgentSession/AgentTurn 三层隔离，历史与编辑事实本地保存
source_doc: 2026-08-14 人类 R5-A 确认、A-AG-M-D-02 §4.2（CAP-00）、§5.15~§5.17
acceptance: 缺失/未知 profile 不会到达 Gate、provider、工具或 Proposal；澄清不被表现为完成，取消/失败/拒绝/完成互斥且不伪成功；只读任务不获得 Proposal 或写工具，写入歧义不猜测目标或执行；可见事件可追溯任务结论、读取/工具、Proposal/操作卡、等待确认和终态原因但不泄露内部推理；不同 workspace/AgentSession 的消息、任务事件、工具卡、Diff、引用和未保存上下文不串线，重启恢复不伪造成功；长期项目记忆不因本要求自动写入或注入
open_issues: Runtime Profile 的兼容迁移、AgentTurn 等待澄清/取消/终态状态机、IntentResult/CapabilityGrant/AgentSession/事件/Trace 字段、本地历史存储恢复和会话管理交互归 AG/SYS DESIGN Scope；本 L1 不定义接口、数据字段、Prompt、UI 或实现算法
-->

<!-- REQ
req_id: REQ-CORE-017
title: 受限复合任务、真实进度与连续 Proposal
owner_module: CORE
status: active
summary: Agent 1.0 的 IntentResult 是任务编排入口：简单目标可形成单回合，复杂目标形成受限任务实例（TaskRun），在同一用户目标内组织多个 AgentTurn 与已授权步骤。每一步都重新受 profile、意图、目标、上下文、最小工具和既有确认点限制；TaskRun 不是后台队列，不自动跨确认点、跨重启或脱离用户续跑。明确的同文档复合目标可连续产生 Proposal：后续 Proposal 必须读取最新 LogicalState，成功预览推进基准并使前卡失效、保留历史且不可再裁决；连续卡不构成补丁栈或任务级回滚。无 LogicalState 的背景 pending Proposal 不得作为后续编辑输入。用户可辨别真实执行、等待补充、等待卡片裁决、中断可重试及终态，且不见模型内部推理
source_doc: 2026-08-14 人类 R5-B 确认、A-AG-M-D-02 §5.15（CAP-16）、REQ-CORE-010/016
acceptance: 研究→大纲→内容、创建→编辑→排版等复合目标按可见步骤推进，步骤只获得当前最小授权且既有多文档、删除、结构保存等确认点不被越过；Diff1 预览进入 LogicalState 后，Diff2 只以该当前状态为基准，Diff2 预览成功即使 Diff1 失效且不可接受/拒绝，拒绝 Diff2 只回到其自身有效基准；背景 pending 目标必须先打开迁移为 LogicalState，或依其背景分流完成裁决后才可继续编辑；取消只停止未开始的后续步骤，不回滚已成功预览的当前 LogicalState 或伪造卡片终态；中断重启后不自动续跑，用户显式重试创建新 AgentTurn 并重新校验；当前“工作中”由真实活跃回合/步骤/工具事件支撑，等待、卡片 pending、失败、取消或完成不伪装为执行中
open_issues: TaskRun/步骤/关联 AgentTurn 与 Proposal 链的状态机、身份字段、并发、计划和进度交互、事件/Trace、持久化/重启恢复、时长与循环上限归 AG/DE/SYS DESIGN Scope；TaskRun 为本轮产品术语，正式 TERM 注册和实现命名待 L2；本 L1 不定义接口、数据结构、Prompt、UI 或算法
-->

<!-- REQ
req_id: REQ-CORE-018
title: 受控外部文件内容读取
owner_module: CORE
status: active
summary: Agent 1.0 以 P1 提供用户显式附加/选择的外部文件内容读取。只有系统当时具备可用只读解析能力的格式，才可在当前用户任务中形成可追溯的一次性 data-only 内容引用；不支持或不可读的文件只保留元数据引用并如实说明。系统不得扫描外部目录、自动再次读取、导入或编辑外部文件；外部内容不授予写入或执行权限，也不默认写入会话历史或项目记忆
source_doc: 2026-08-14 人类 R5-C 确认、REQ-CORE-006、A-AG-M-D-02 §5.5（CAP-20）
acceptance: 用户可辨别文件来源、已读/未读状态和“只供当前任务”的范围；支持的只读路径只向当前任务提供 data-only 内容，分析或基于内容的修改仍受 IntentResult、最小授权与 Proposal/裁决/保存约束；不支持/失败文件不伪称已读；任务结束、重启、其他会话或 workspace 不自动再次取得该内容
open_issues: 格式清单、解析器可用性、大小/上下文预算、内容快照/暂存/清除、传输提示、脱敏 Trace、错误与 UI 归 REF/AG/SYS DESIGN Scope；本 L1 不定义读取接口、数据字段、存储位置或实现算法
-->

<!-- REQ
req_id: REQ-CORE-019
title: 用户管理的本地项目长期记忆
owner_module: CORE
status: active
summary: Agent 1.0 以 P1 提供 workspace 隔离、本地保存的项目长期记忆；用户可创建、查看来源、编辑、删除和关闭条目。它不是会话/编辑事实或外部文件的自动副本。Agent 1.0 不自动写入、不生成写入提议、不按相关性自动检索或注入；只有用户明确选择的记忆条目，才可作为 data-only 引用进入当前 AgentTurn
source_doc: 2026-08-14 人类 R5-C 确认、REQ-CORE-005/006/016、A-AG-M-D-02 §5.17（CAP-18）
acceptance: 项目记忆不跨 workspace 共享或上云，用户可管理其完整生命周期；普通 Agent 发送、外部文件附加和会话恢复不会创建、提议、检索或注入记忆；用户选择的条目可追溯来源并仅影响当前回合的 data-only 上下文，不改变系统约束、任务目标、工具授权、编辑目标或确认点；外部文件内容不会自动转存为记忆
open_issues: 记忆条目模型、手工管理/关闭交互、来源模型、显式选择令牌、删除传播、存储/事件/IPC/状态机和隐私策略归 AST/REF/AG/SYS DESIGN Scope；未来 Profile 的 Agent 记忆提议、自动写入或自动注入须独立确认和验证
-->

## §6 产品边界与约束（边界闭合）

- **范围边界**：§3 IN/OUT 为 v1 冻结边界；任何扩缩须先经 APC 授权再回写本文与路线图 §2（APC §13）。
- **内容主权边界**：用户内容不出本地，云端不承载内容、不做内容同步（APC §1）。
- **文件系统边界**：AI 文件操作限定在 workspace 内（APC §9 安全模型）。
- **安全/计费边界**：密钥服务端持有；订阅门禁客户端不可绕过；用量计量防篡改（APC §9）。
- **离线边界**：见 REQ-CORE-009。
- **写入边界**：AI 文档内容修改一律以可见提案供人类后置裁决；受控预览可更新 LogicalState，显式接受/拒绝不直接写盘，只有保存写入 DiskState。工作区资源操作走独立的前置裁决链，确认后直接修改 DiskState（REQ-CORE-002/010/012/014）。
- **读取与上下文边界**：项目报告先概览、后按任务最小化读取正文；已打开文档只读 LogicalState，未打开文档只读受限 DiskState 并披露新鲜度；显式引用均为 data-only，不自动扩大写权限（REQ-CORE-006/011）。
- **原生 DOCX 边界**：原生 DOCX 只读预览、读取和分析；其编辑请求必须创建新的 BDocx 编辑副本，原文件不进入编辑会话且不被写回。BDocx 是“DOCX → HTML 编辑模型 → DOCX”的处理路径，不是格式（REQ-CORE-004）。
- **最小转换边界**：通用转换在 1.0 仅限 REQ-CORE-013 的 TXT→Markdown 与 Markdown→TXT 目标副本；来源不被改写，目标资源使用 CreateDocumentBundle，初始内容继续走普通 Diff 生命周期。HTML 与通用 DOCX 互转、批量和原地转换不属于 1.0。
- **删除与恢复边界**：已保存文件/文件夹的删除按 REQ-CORE-014 生成 WorkspaceOperationPlan 和 DecisionRequest；确认后重新核验并直接修改磁盘，同时写入可恢复的 WorkspaceTimeline 条目。脏标签阻断执行，永久清理、自动清空和未核验递归删除不属于 1.0。
- **多文档写入边界**：多个既有文档的写入只按 REQ-CORE-015 的已确认冻结目标清单逐项进行；每项独立预览、裁决和保存，不因“多文档”获得批量接受、批量保存或跨文档事务授权。
- **Agent 运行边界**：仅 REQ-CORE-016 的 agent-edit-1.0 可启动；profile、意图、目标、副作用、上下文、工具与确认点逐层收窄。澄清等待、取消、终态、可见事件和 AgentSession 隔离必须如实表现；模型内部推理、长期记忆自动写入及 Build/CBT 不属于 1.0 主链。
- **复合任务与连续 Proposal 边界**：REQ-CORE-017 的 TaskRun 只能组织当前用户目标内的受限步骤，不能成为后台队列或跨确认点自动执行。同文档后续 Proposal 只可基于最新 LogicalState；基准推进使前卡失效，禁止补丁叠加、任务级自动回滚或背景 pending payload 续编。
- **外部内容与项目记忆边界**：REQ-CORE-018 的外部文件内容只在用户显式附加后的当前任务中以 data-only 使用；REQ-CORE-019 的长期记忆由用户在 workspace 内管理，只有明确选择才进入回合。二者都不自动写入、不自动注入、不能改变授权或成为外部文件扫描入口。
- **Agent 1.0 冻结边界**：R1–R5-C 的已确认 1.0 需求范围已由 [Agent 1.0 冻结能力基线](A-CORE-X-D-02_Agent1.0冻结能力基线.md) 固定；后续设计只能细化，任何行为、权限、档位或状态语义变动必须新开 REQUIREMENT Scope，不得由 L2、Prompt、工具或代码隐式扩大。

## §7 G1 问题暴露清单（处置结果）

按 APC §5 G1 标准；处置决策于 2026-06-09。所有项均已**决策**或**显式延期**（记明 owner + 门禁），
据此产品级 G1 具备闭合条件。晋级由人类宣布（APC §2，AI 不得自行宣布晋级）。

| 编号 | 分类 | 状态 | 处置结果 |
|---|---|---|---|
| PO-01 | NEEDS_HUMAN_DECISION | 已决策 | v1 主用户群确定（§2）；次级分群/画像深度显式延期至 G1 后，不阻断 |
| PO-02 | REQ_GAP | 显式延期 | 模块级 REQ-[MODULE]-* 在各模块 G1 于其 M-D 承接（设计使然） |
| PO-03 | DESIGN_RISK | 显式延期 | 模块编码随 SYS P1 落 MODULE 注册块 + generate 注册；注册块归 A-SYS-C-T-01 系统总体架构 |
| PO-04 | ACCEPTANCE_MISSING | 部分关闭 | 定性验收已补入 §5 各 REQ-CORE-*；NFR 量化指标显式延期至 G5（APC §9） |
| PO-05 | （改归技术） | 移交 | 时间轴↔持久化一致性属技术问题，改归 WS-G2 核对，移出产品 G1 阻断 |
| PO-06 | BOUNDARY_OPEN | 已决策 | BYOK 纳入 v1 GA（契约 APC §4.3 已锁，§5 REQ-CORE-008 已注明）；产品层呈现/切换 UX 显式延期至 ENT G1 |
| PO-07 | REQUIREMENT_ALIGNMENT | 处理中 | 已确认的“逻辑状态优先”新建、结构、转换、删除、多文档和 Agent 运行边界已同步 L1；L2 状态机/接口/实现仍须按该基线对齐 |

## §8 后续动作

1. 进入各模块 G1：按本文 §3 归属，在各模块 M-D 声明模块级 REQ 承接产品级 REQ-CORE-*；本轮的承接关系与文档入口见 A-CORE-X-D-01。
2. §7 暴露清单已于 2026-06-09 全部决策/显式延期；产品级 G1 待人类宣布闭合。
3. 与路线图 §4 构建阶段对齐：P1 先行 SYS 底座 + 云接口契约，再 P2 workspace/编辑核心。
4. 范围或定位变更先经 APC 授权再回写本文。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-09 | v1.0 | 初版：产品定位/目标用户、v1 能力范围（含三项范围确认）、核心场景主流程骨架、产品级 REQ-CORE-001~009、产品边界、G1 问题暴露清单 |
| 2026-06-09 | v1.1 | 处置 §7 G1 暴露清单：PO-01 确定 v1 主用户群、PO-06 BYOK 纳入 v1 GA（§2/§5/§7）；为 REQ-CORE-* 补定性验收（§5）；PO-04 量化延 G5、PO-05 改归 WS-G2、PO-02/PO-03 显式延期；产品级 G1 具备闭合条件 |
| 2026-07-04 | v1.2 | 术语沿革回写（人类裁决）：REQ-CORE-002 summary「单文档 chat-agent」→「代理编辑（编辑/构建双模式）」；REQ-CORE-003 summary 补构建全流程、「Chat↔Build 接管」→「构建移交/回落」；§3 能力表与 S1 场景名同步（「对话编辑」退役为「代理编辑」禁用别名） |
| 2026-08-13 | v1.3 | 依据已确认的 1.0 文档生命周期基线，新增 REQ-CORE-010；将 S1 与 REQ-CORE-002 统一为「受控预览进入 LogicalState → 人类裁决提案 → 显式保存写入 DiskState」，补充新建文档自动开标签、提案失效与未保存边界；L2/实现同步列为后续动作 |
| 2026-08-13 | v1.4 | DIFF-STATE-COMPASS-01：S1 明确三分流——已有 LogicalState 直接预览且接受不改内容、无 LogicalState 临时挂载且接受原子落盘、pending 打开迁移为前者；与 REQ-CORE-010 对齐。 |
| 2026-08-13 | v1.5 | AG-1.0-R2-REQUIREMENTS-01：新增 REQ-CORE-011，冻结 CAP-01~04 为 P0；项目分析采用“先概览、后最小化正文读取”，已打开文档以 LogicalState 为准、未打开文档标识 DiskState 新鲜度；M1/M2 为 P0、M4 保持 P1，并收紧 REQ-CORE-006 的显式引用 data-only 边界。 |
| 2026-08-13 | v1.6 | AG-1.0-R3-REQUIREMENTS-01：REQ-CORE-004 改为原生 DOCX 只读预览/分析，编辑请求创建新的 BDocx 编辑副本；明确 BDocx 是 DOCX→HTML 编辑→DOCX 的处理路径而非格式，原文件不可被编辑或写回。 |
| 2026-08-14 | v1.7 | AG-1.0-R4-STRUCTURE-REQUIREMENTS-01：新增 REQ-CORE-012，文件树创建文件夹、单文件重命名/移动改为先投影逻辑文件树、显式保存才写盘；复制、删除、目录移动、批量提交和格式转换保持后续确认。 |
| 2026-08-14 | v1.8 | AG-1.0-R4-CONVERSION-REQUIREMENTS-01：新增 REQ-CORE-013；TXT→Markdown 与 Markdown→TXT 固定为 P1 的新目标副本转换，来源不改写，预览创建目标 LogicalState、接受不写盘、显式保存才落盘；Markdown→TXT 明示有损，HTML 与通用 DOCX 转换继续排除。 |
| 2026-08-14 | v1.9 | AG-1.0-R4-DELETE-MULTIDOC-REQUIREMENTS-01：新增 REQ-CORE-014/015；已保存文件/文件夹删除改为红色逻辑暂存、单项显式保存移入可恢复存储且不提供永久删除；多个既有可编辑文档仅在确认并冻结目标清单后逐项 Proposal/裁决/保存，不提供批量接受、批量保存或跨文档事务。 |
| 2026-08-14 | v2.0 | AG-1.0-R5A-RUNTIME-REQUIREMENTS-01：新增 REQ-CORE-016；Agent 1.0 只允许 agent-edit-1.0 且缺失/未知 profile 失败关闭，澄清等待、全活动态取消、最小意图/工具授权、结构化可见事件、三层会话隔离和本地事实历史进入 P0；长期记忆与 Build/CBT 保持后置。 |
| 2026-08-14 | v2.1 | AG-1.0-R5B-TASK-ORCHESTRATION-REQUIREMENTS-01：新增 REQ-CORE-017；复杂目标可由受限 TaskRun 组织多步骤/多 AgentTurn，真实工作状态进入 P0；明确同文档连续 Proposal 必须以最新 LogicalState 接力、前卡基准推进失效，禁止背景 payload 续编、任务级回滚、后台队列与自动跨重启续跑。 |
| 2026-08-14 | v2.2 | AG-1.0-R5C-MEMORY-EXTERNAL-READ-REQUIREMENTS-01：新增 REQ-CORE-018/019；外部文件实际读取进入 P1 的显式、一次性 data-only 路径，未读格式退回元数据；长期项目记忆进入 P1 的 workspace 本地用户管理路径，Agent 1.0 不自动写入、提议或注入，外部内容不自动转存为记忆。 |
| 2026-08-14 | v2.3 | AG-1.0-R6-BASELINE-FREEZE-01：R1–R5-C 的 Agent 1.0 已确认需求范围冻结；新增冻结记录入口，声明后续设计不得隐式改变产品行为、权限、档位或状态语义，需求冻结不等于实现或发布完成。 |
| 2026-08-21 | v2.4 | WORKSPACE-OPERATION-BASELINE-01：人类确认八项新决策，REQ-CORE-010/012/014 改为 WorkspaceOperationPlan 前置裁决后直接写 DiskState；统一 WorkspaceTimeline 承载资源恢复；新建文档采用 CreateDocumentBundle，绑定初始 Diff 仅在基线未推进时可补偿删除。 |
