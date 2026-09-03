---
文档编号：WS-M-D-01
文档状态：A
负责模块：WS
文档职责：WS 工作台模块级功能需求描述——声明模块级 REQ-WS-*，承接产品级 REQ-CORE-001/002/004/007/010/011/012/013/014/015，界定 workspace 生命周期 / 文件 I/O 与显式 save / 工作区概览与受限 DiskState 读取 / WorkspaceOperation 前置裁决与直接磁盘执行 / CreateDocumentBundle / WorkspaceTimeline / 多文档逐项保存 / 原生 DOCX 来源保护与 BDocx 编辑副本落盘 / 提案控制态 / 控制态缓存与重启可恢复
上游约束：A-CORE-C-D-01_产品概述.md（REQ-CORE-001/002/004/007/010/011/012/013/014/015）、A-CORE-X-D-02_Agent1.0冻结能力基线.md（P0/P1 与不可突破边界）、A-WS-M-T-02_工作区资源操作技术设计.md、A-WS-M-S-02_工作区资源操作状态机设计.md、A-CORE-C-P-01_APC.md（§4.1 WS 职责、§6 save 状态机、§9 文件系统安全边界）、A-SYS-I-P-01_数据结构定义.md、A-SYS-M-S-01_状态机总图.md、ADU.md（§3.2 REQ 规范）
直接承接：WS L2 技术设计（workspace 模型 / 文件系统 / 时间轴 / workspace.db 完整 schema / save·落盘·还原状态机）经 REQ_MAP 映射 RULE/CHAIN/CONSTRAINT/TERM；解除 SYS 侧 SY-03（workspace.db 完整列）与 SY-04（save 状态机）
使用边界：本文是 L1 功能需求描述层，仅声明模块级 REQ-WS-*，**不定义技术规则**（RULE/CHAIN/CONSTRAINT/TERM/STATE_MACHINE 见 WS L2）；**代码不得直接映射 REQ**（ADU §3.2）；workspace.db 具体表结构、时间轴存储模型、状态机枚举、缓存策略归 WS L2；diff 终态转移语义归 DE（WS 只负责持久化与落盘执行）
变更要求：WS 职责或 workspace 边界变更须先经 APC 授权再回写本文；REQ 增删按 ADU §3.2 req_id 规则（序号不复用，废弃改 superseded）
治理成熟度：draft
---

# Binder WS 工作台功能需求描述（L1）

## §1 文档定位

本文是 **WS 模块**的 L1 功能需求描述，处在产品概述（L1 根）之下、WS L2 技术设计之上：

```
产品概述(REQ-CORE-*) → 本文(REQ-WS-*) → REQ_MAP → WS L2 规则(SSOT) → 代码(@GOV)
```

WS 是 Binder 本地优先写作的**工作台底座**：管理 workspace 生命周期与文件 I/O，持久化提案控制态，
仅在显式保存时落盘当前 LogicalState，并提供文档级时间轴与历史回溯（APC §4.1）。WS 被 ED/DE/AG/CBT/REF/AST/KB/
TPL/DOC/PRV 等模块依赖（A-SYS-C-T-01 §4：`ED→WS`、`DE→ED,WS,AG` 等）。本文只声明 WS 模块级需求，
不写技术方案、不定义规则、代码不得追溯本文；具体表结构、时间轴模型、状态机枚举、缓存策略一律归
WS L2。未被 APC / 产品概述 / SYS 契约支撑的判断一律进 §7 暴露清单，不杜撰。

## §2 模块定位与职责

**定位**：WS 是**本地优先存储与 workspace 边界的承载模块**，是文件操作 / 搜索 / diff / 上下文读取
的唯一物理边界（REQ-CORE-001），并为上层模块提供文件 I/O、持久化与历史能力。

**职责清单（v1，APC §4.1）**：

| # | 职责 | 锚定 |
|---|---|---|
| W-J1 | workspace 生命周期：打开 / 切换 / 关闭，作为文件/搜索/diff/上下文唯一边界 | REQ-CORE-001、APC §9 |
| W-J2 | 文件 I/O、save 保存与跨文档资源操作（限定 workspace 内） | REQ-CORE-001/002、APC §4.1/§6 |
| W-J3 | 提案控制态持久化与分流写盘：已有 LogicalState 的裁决不写文件；无 LogicalState 的接受原子写盘 | REQ-CORE-002/010、A-SYS-I-P-01 |
| W-J4 | 统一 WorkspaceTimeline：文档内容历史与工作区资源操作历史的类型化回溯 / 还原 | REQ-CORE-007/014 |
| W-J5 | 控制态缓存与重启可恢复（workspace.db，联合 SYS） | A-SYS-I-P-01 BR-SYS-PERSIST-001 |
| W-J6 | 原生 DOCX 来源保护与 BDocx 编辑副本落盘：原文件只读，新副本按 CreateDocumentBundle 创建并保存 | REQ-CORE-004 |
| W-J7 | 工作区资源操作：WorkspaceOperationPlan 前置裁决、确认时再核验、直接 DiskState 执行、失败补偿 | REQ-CORE-012 |
| W-J8 | 最小格式转换目标副本保存：转换来源不变，目标 LogicalState 仅在显式保存后写入目标文件 | REQ-CORE-013 |
| W-J9 | 可恢复删除与恢复：前置裁决、类型化资源快照、WorkspaceTimeline 统一恢复 | REQ-CORE-014 |
| W-J10 | 多既有文档逐项保存：确认冻结目标清单后，各目标独立基准、保存、失败与时间轴结果 | REQ-CORE-015 |

> 边界澄清：WS 持久化提案控制态。已有 LogicalState 的提案由用户**显式保存**把 LogicalState 写入
> DiskState；保存仍有效 pending 预览时同时将卡裁决为 accepted/awaiting_save，显式接受/拒绝仍属 DE 的卡片裁决；没有 LogicalState 的背景提案只临时挂载于 DiffCard，用户
> 接受后才可由 WS 原子写入 DiskState。pending 期间打开目标文档时必须迁移为前一分支。该罗盘只适用于
> 文档内容 Diff；工作区资源操作使用 WorkspaceOperationPlan + DecisionRequest 前置裁决，确认后直接修改
> DiskState，不生成资源 DiffCard。新建文档使用 CreateDocumentBundle：确认后先创建磁盘文件，再投影初始内容 Diff。
> workspace.db 的 schema 基线与迁移契约归 SYS（A-SYS-I-P-01），WS 承接其完整列定义（解除 SY-03）。

## §3 能力范围（WS v1）

**IN（WS 承载）**：
- workspace 打开/切换/关闭与边界守护（路径不越出 workspace root，APC §9）。
- workspace 内文档/资源的读、写、列举、跨文档操作；编辑内容的 save 保存。
- 提案控制态持久化（共享结构由 SYS 定义）；受控预览和人类裁决不写盘，显式 save 才落盘。
- 统一 WorkspaceTimeline：记录文档内容节点与类型化 WorkspaceOperationEntry，并按条目类型回溯 / 还原。
- 控制态缓存与重启恢复（workspace.db 持久化，遵守 SYS BR-SYS-PERSIST-001）。
- P1 工作区资源操作：创建、删除、重命名、移动先生成不可变计划与 DecisionRequest；确认后重新核验并直接修改 DiskState。
- P1 新建与最小格式转换目标：使用 CreateDocumentBundle，在确认后创建目标磁盘文件，再以普通内容 Diff 投影初始内容。
- P1 可恢复删除与恢复：删除写入类型化资源快照和 WorkspaceTimeline，恢复冲突不得自动覆盖，永久清理不在 1.0。
- P1 多既有文档逐项保存：确认冻结目标清单后，每个目标独立保存、失败与时间轴，不提供批量保存或跨文档事务。

**OUT（非 WS）**：
- 富文本编辑运行时（TipTap）——归 ED；WS 提供其底层文件 I/O。
- diff 的生成、审阅 UI 与终态转移语义——归 DE（WS 仅持久化与落盘执行）。
- DOCX/格式转换——归 DOC；多格式预览/导出——归 PRV。
- 跨进程 IPC 协议本身、共享数据结构契约、workspace.db schema 基线——归 SYS（WS 消费/承接）。
- 状态机枚举、表结构、时间轴存储模型、缓存策略——归 WS L2（本文只定边界）。
- NFR 量化指标（I/O 延迟、恢复耗时）——延 G5 标定（APC §9）。

## §4 代表性流程（契约级，APC §7）

**W1 · workspace 生命周期**（W-J1）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | 用户 | 选定本地目录 | 打开为 workspace，设为文件/搜索/diff/上下文边界 | 活动 workspace | 2 | 目录无权限/不存在→提示重选 |
| 2 | WS | 文件操作请求 | 校验路径在 workspace root 内后执行 | I/O 结果 | 3 | 越界路径→拒绝（APC §9 边界） |
| 3 | 用户 | 切换/关闭 | 切换活动 workspace 或关闭 | 新活动态/已关闭 | 终态 | 有未落盘控制态→持久化后再切换 |

**W2 · 提案与显式保存链**（W-J3，承接 REQ-CORE-002/010）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | DE | 生成的提案 | WS 持久化提案控制态（共享结构由 SYS 定义） | 可恢复提案 | 2 | 持久化失败→统一错误形状（A-SYS-I-P-02） |
| 2 | DE/ED/用户 | 受控预览与裁决 | 有 LogicalState 时 ED 预览、DE 只裁决卡片；无 LogicalState 时卡片临时挂载，接受才交 WS 原子写盘；pending 打开迁移为前者 | 当前逻辑文档，或背景待裁决卡片 | 3 | 基准推进→提案失效，逻辑文档保留 |
| 3 | 用户 | 显式 save（仅已有 LogicalState） | WS 校验范围后将当前 LogicalState 写入目标文件并记时间轴节点 | 已落盘 + 时间轴节点 | 终态 | 写盘失败→不记节点，保留 LogicalState、标签和可重试保存 |

> W2 在 L1 锁定三分流：已有 LogicalState 的预览/裁决不写盘、无 LogicalState 的背景卡只在接受后原子写盘、
> pending 打开迁移为已有 LogicalState；提案裁决归 DE，WS 的 save/落盘状态机归 WS L2（解除 SY-04）。

**W3 · 时间轴回溯还原**（W-J4，承接 REQ-CORE-007）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | 用户 | 打开文档时间轴 | WS 列出该文档历史节点 | 历史节点列表 | 2 | 无历史→空列表（非错误） |
| 2 | 用户 | 选定历史节点 | 预览该版本 | 版本预览 | 3 | 节点数据缺失→提示并跳过 |
| 3 | 用户 | 确认还原 | 还原到选定版本并记一个新节点 | 已还原 + 新节点 | 终态 | 还原失败→保持现状，回报错误 |

## §5 模块级需求声明（REQ-WS-*）

> owner_module = WS。每条经 source_doc 标注承接的产品级 REQ-CORE-* / SYS 契约与 APC 锚点。
> 代码不得直接映射本文 REQ（ADU §3.2）；映射到规则在 WS L2 经 REQ_MAP 完成。

<!-- REQ
req_id: REQ-WS-001
title: workspace 生命周期与唯一边界
owner_module: WS
status: active
summary: 管理 workspace 打开/切换/关闭，并以 workspace root 作为文件操作/搜索/diff/上下文读取的唯一边界，所有文件操作不得越出该边界
source_doc: REQ-CORE-001、APC §4.1、§9
acceptance: workspace 可打开/切换/关闭；任何文件操作路径在 workspace root 内方可执行，越界被拒
open_issues: 多 workspace 并存/切换时的控制态隔离细节归 WS L2
-->

<!-- REQ
req_id: REQ-WS-002
title: 文件 I/O、save 保存与跨文档资源操作
owner_module: WS
status: active
summary: 提供 workspace 内已保存文档/资源的读写、列举与跨文档操作，以及把当前 LogicalState 写入磁盘的显式 save；为 Agent 1.0 项目分析先提供结构概览（路径、类型、数量、可访问范围），正文仅按任务最小化读取。未打开文档读取 DiskState 并返回读取时点/新鲜度，已打开文档的当前事实不由 WS 磁盘旧内容替代；AI 文件操作限定在 workspace 内
source_doc: REQ-CORE-001/002/010/011、APC §4.1、§6
acceptance: workspace 内已保存文件可读写/列举/跨文档操作；项目分析可先得到结构概览且不把目录列表误称为正文；按需正文读取不越出 workspace 并可标识 DiskState 读取时点/新鲜度；项目图片发现只返回未越界、未跟随符号链接的相对普通文件；显式 save 将当前 LogicalState 可靠落盘；AI 文件操作不越出 workspace
open_issues: save 保存状态机（APC §6 核心链）归 WS L2（M-S，解除 SY-04）；并发写入边界归 WS L2
-->

<!-- REQ
req_id: REQ-WS-003
title: 提案控制态持久化与分流落盘
owner_module: WS
status: active
summary: 持久化由 DE 生成的 ChangeProposal/diff 控制态；已有 LogicalState 的受控预览、显式接受、拒绝或失效不直接写文件，保存才落盘并记时间轴节点；其中保存仍有效的 pending 预览必须先将卡片裁决为 accepted/awaiting_save；无 LogicalState 的背景卡只临时挂载，用户接受后由 WS 原子写入目标文件；pending 打开目标文档时迁移为已有 LogicalState 分支
source_doc: REQ-CORE-002/010、A-SYS-I-P-01（共享结构、workspace.db）
acceptance: 提案控制态可持久化且重启可恢复；已有 LogicalState 的显式接受不触发写盘，保存有效 pending 预览则先裁决为 accepted/awaiting_save 后落盘，基线已推进时只保存当前逻辑内容并使旧卡失效；无 LogicalState 的接受原子落盘；背景 pending 打开后不再具备后台写盘资格；拒绝/失效不误写文件；保存失败保留当前 LogicalState
open_issues: 提案裁决语义归 DE；WS 侧保存状态机与跨模块转移归 WS L2
-->

<!-- REQ
req_id: REQ-WS-004
title: 文档级时间轴与历史回溯/还原
owner_module: WS
status: active
summary: 提供文档级版本时间轴（A-SYS-I-P-01 TimelineEntry）与历史回溯、还原到先前版本，落盘与还原各记时间轴节点
source_doc: REQ-CORE-007、A-SYS-I-P-01（TimelineEntry）
acceptance: 文档有可浏览历史节点，可回溯并还原到先前版本；还原失败不破坏现状
open_issues: 时间轴节点准入规则/存储模型/还原执行链归 WS L2（取材台账 §4.3 时间轴四件）
-->

<!-- REQ
req_id: REQ-WS-005
title: 控制态缓存与重启可恢复
owner_module: WS
status: active
summary: 缓存 workspace 控制态并持久化于 workspace.db，遵守 SYS schema 版本化迁移契约，使 pending_diffs、时间轴等控制态在应用重启后可恢复
source_doc: A-SYS-I-P-01（workspace.db schema 基线、BR-SYS-PERSIST-001、X-SHARED-001）
acceptance: 重启后未裁决 pending_diffs 与时间轴可恢复；workspace.db 迁移不丢既有控制态
open_issues: workspace.db 完整列/索引与缓存策略归 WS L2（联合 SYS，解除 SY-03）
-->

<!-- REQ
req_id: REQ-WS-006
title: CreateDocumentBundle 的资源创建与内容投影边界
owner_module: WS
status: active
summary: 新建文档由 CreateDocumentBundle 同时冻结 WorkspaceOperationPlan 与预先生成的初始内容；DecisionRequest 确认后 WS 重新核验并创建空的目标 DiskState，ED 打开标签，DE/ED 再以该磁盘基线投影初始内容 Diff。资源创建与内容 Diff 是顺序衔接但生命周期独立的两段
source_doc: REQ-CORE-010、A-CORE-X-D-02 §3/§4、A-WS-M-T-02 §6
acceptance: 确认前文件树无目标；确认后目标立即成为真实磁盘节点并打开标签；初始内容 Diff 接受不再次写盘，显式保存写入既有目标；初始 Diff 仅在资源、DiskState、LogicalState 三重基线未变时可通过拒绝删除刚创建文件，否则卡片失效且目标保留；任一失败均不得产生幽灵文件或虚假完成
open_issues: 无；详细对象、状态与补偿见 A-WS-M-T-02 / A-WS-M-S-02
-->

<!-- REQ
req_id: REQ-WS-007
title: 原生 DOCX 来源保护与 BDocx 编辑副本保存
owner_module: WS
status: active
summary: 原生 DOCX 仅作为预览、读取、分析和创建 BDocx 编辑副本的只读来源；创建副本 Proposal 在预览成功后仅产生未保存 LogicalState，用户显式保存才在 workspace 中生成新的 DOCX。任何编辑、接受、拒绝、失效或保存失败都不得改写原生 DOCX
source_doc: REQ-CORE-004、REQ-CORE-010、A-CORE-X-D-02 §3/§4
acceptance: 编辑原生 DOCX 的请求没有原地写盘路径；副本 Proposal 的接受不创建物理文件，保存成功才创建新的 DOCX 并记入时间轴；拒绝有效 Proposal 或关闭未保存副本不影响原文件；转换/保存失败保留副本 LogicalState 且原文件、既有副本均不变
open_issues: 副本命名、来源关联、冲突策略及 DOC 转换与 WS 保存的状态机/接口归 DOC/WS/DE/ED DESIGN Scope
-->

<!-- REQ
req_id: REQ-WS-008
title: WorkspaceOperation 前置裁决与直接磁盘执行
owner_module: WS
status: active
summary: 对创建文件夹、创建文档、删除、重命名、移动等工作区资源操作，WS 接收不可变 WorkspaceOperationPlan；DecisionRequest 确认前不执行，确认后重新核验路径、冲突、递归清单、打开会话、内容 Diff 与磁盘基线，再直接修改 DiskState。资源操作不进入内容 DiffCard 生命周期，也不等待第二次“保存文件树”
source_doc: REQ-CORE-012、A-CORE-X-D-02 §3/§4、A-AG-M-D-01（REQ-AG-011）、A-WS-M-T-02
acceptance: 决策卡冻结并显示所有资源项和影响；拒绝不改盘；确认后脏标签阻断删除/重命名/移动，有效内容 Diff 作为影响展示并在成功执行后失效；每个执行结果与补偿收据可追溯；批量操作 fail-fast 并逆序补偿，补偿不完整进入 partial_failed；进程重启后未完成操作进入 Interrupted，绝不自动执行；所有路径保持在 workspace 内且不覆盖冲突目标。人工文件树入口（用户 UI 直调）保持直写，但删除/重命名必须遵守 BR-WS-STATE-009 安全底线（dirty 阻断、快照/时间轴可恢复、Diff 失效）（WS-RESOURCE-WRITE-GUARD-01）。
open_issues: 无；共享对象、接口与状态机见 A-WS-M-T-02 / A-WS-M-S-02
-->

<!-- REQ
req_id: REQ-WS-009
title: 最小格式转换目标副本的 CreateDocumentBundle
owner_module: WS
status: active
summary: 对 TXT→Markdown、Markdown→TXT 的转换目标副本，来源文件或来源 LogicalState 不被 WS 改写；目标资源创建沿 CreateDocumentBundle 执行：确认后先创建目标 DiskState，再将转换结果作为初始内容 Diff 投影。来源不是资源操作或内容 Diff 的写入目标
source_doc: REQ-CORE-013、A-CORE-X-D-02 §3/§4、A-DOC-M-D-01（REQ-DOC-006）、A-DE-M-D-01（REQ-DE-006）
acceptance: 打开来源取当前 LogicalState、未打开来源取有读取时点的 DiskState；确认前不创建目标，确认后仅目标副本成为文件树/DiskState 节点并记录 WorkspaceTimeline；转换内容 Diff 的接受只改卡片生命周期，保存写入既有目标；目标后续编辑使旧转换卡失效但不删除目标；路径冲突、外部变化或写盘失败时不自动覆盖、不伪报成功；HTML、通用 DOCX、批量或原地转换不进入本能力
open_issues: 转换目标的未保存元数据、源/目标基线、格式/路径校验、冲突时机、时间轴关系、重启恢复与错误形状归 WS/DOC/DE/ED/AG/SYS DESIGN Scope；本 L1 不定义状态机、接口或 I/O 算法
-->

<!-- REQ
req_id: REQ-WS-010
title: 删除与恢复的前置裁决及 WorkspaceTimeline
owner_module: WS
status: active
summary: 对已保存文件/文件夹的删除与恢复，WS 使用 WorkspaceOperationPlan + DecisionRequest 前置裁决；确认后重新核验冻结影响清单、来源基线、冲突与脏会话，直接修改 DiskState，并写入含类型化资源快照和恢复元数据的 WorkspaceOperationEntry。永久清理不属于 Agent 1.0
source_doc: REQ-CORE-014、A-CORE-X-D-02 §3/§4、A-DE-M-D-01（REQ-DE-007）
acceptance: 确认前磁盘目标保持原状；非空文件夹递归范围、数量和来源快照冻结并在执行前重验，任一变化或脏会话均阻断；成功后真实移除目标、使相关内容 Diff 失效、更新标签/引用并写入可恢复 WorkspaceTimeline 条目；恢复按条目类型执行且冲突不自动覆盖；失败或补偿不完整必须显示真实状态，不得伪报成功。人工文件树删除同样写入可恢复 WorkspaceTimeline 条目并受 dirty 阻断（BR-WS-STATE-009，WS-RESOURCE-WRITE-GUARD-01）
open_issues: 永久清理和保留期不在 1.0；其余契约由 A-WS-M-T-02 / A-WS-M-S-02 定义
-->

<!-- REQ
req_id: REQ-WS-011
title: 多个既有文档的独立保存与失败隔离
owner_module: WS
status: active
summary: 在 AG 已确认并冻结的多既有文档目标清单内，WS 对每个目标独立持久化 Proposal 控制态、执行其自身 LogicalState/DiskState 分流、接受后的背景原子写盘或显式保存、失败保留和时间轴结果；不得把多个目标合并为批量保存、全有或全无事务或共同回滚
source_doc: REQ-CORE-015、A-CORE-X-D-02 §3/§4、A-DE-M-D-01（REQ-DE-008）
acceptance: WS 只接收冻结目标清单中的逐目标保存请求；每个目标保存前各自核验路径、基准和外部变化，单一目标冲突/删除/结构变化/写盘失败不写入该目标且不影响其他目标；有 LogicalState 的目标仅在其显式保存时写盘，背景目标仅在其自身卡接受时原子写盘并在打开后失去该资格；每个成功、失败和未保存状态可独立追溯；原生 DOCX、删除/文件树整理/格式转换混合操作不进入本保存路径
open_issues: 冻结目标身份、逐目标队列/并发、控制态持久化、跨目标事件排序、时间轴/重启恢复、失败形状与 Trace 归 WS/DE/ED/AG/REF/SYS DESIGN Scope；本 L1 不定义批处理或事务接口
-->

## §6 模块边界与约束（边界闭合）

- **职责边界**：WS 只承载 workspace/文件/持久化/时间轴（§2 W-J1–W-J10），不做编辑运行时、diff 生成、
  格式转换、预览；越界即范围外（CLAUDE.md §1 范围纪律）。
- **L1/L2 边界**：本文只定契约**边界**；表结构、时间轴存储模型、状态机枚举、缓存策略归 WS L2，
  代码只追溯 L2（ADU §3.2、APC §2）。
- **文件系统安全边界**（REQ-WS-001/002）：所有文件操作（含 AI）限定 workspace root 内，越界拒绝
  （APC §9）。
- **数据结构边界**：WS 复用 A-SYS-I-P-01 的 WorkspaceRef/DocRef/PendingDiff/TimelineEntry 与
  workspace.db schema 基线，遵守 X-SHARED-001（不私自重定义跨模块共享结构）、BR-SYS-PERSIST-001。
- **与 DE 的边界**：diff 生成与提案裁决语义归 DE；WS 持久化提案控制态，并仅在显式 save 时落盘。
- **与 AG 的读取边界**：WS 供给工作区概览和未打开文档的受限 DiskState；读取不等同写权限，也不得用磁盘内容替代 ED 已打开文档的 LogicalState（REQ-WS-002）。
- **资源操作边界**：工作区资源操作不复用内容 Diff。REQ-WS-008 的 WorkspaceOperationPlan 经 DecisionRequest 前置授权后，由 WS 重新核验并直接修改 DiskState；没有资源 DiffCard，也没有第二次保存裁决。
- **新建与转换边界**：新建文档及 TXT↔Markdown 转换目标使用 CreateDocumentBundle；确认后先创建目标文件，再投影初始内容 Diff。来源不被改写；HTML、通用 DOCX、批量和原地转换不属于 REQ-WS-009。
- **删除/恢复边界**：删除和恢复按 REQ-WS-010 前置裁决并直接改变磁盘；脏标签阻断，类型化快照进入 WorkspaceTimeline，恢复冲突不覆盖，永久清理不在 1.0。
- **多文档落盘边界**：多个既有文档只按已确认冻结的目标清单逐项保存；每项独立核验、失败与时间轴，不提供批量保存、共同回滚或跨文档事务。
- **状态机边界**（APC §6）：save 保存、diff 落盘、时间轴还原为状态化核心链，须先建状态机方可实现
  （X-SEQ-003，A-SYS-M-S-01），WS G2 在其 M-S 定义并登记总图。
- **L4 原料边界**：取材见台账 §4.3（WS：workspace 模型/生命周期/文件系统/跨文档/时间轴四件等，均
  P1 原料），为原料不为权威，REQ 重新撰写（APC §8）。

## §7 G1 问题暴露清单

按 APC §5 G1 标准（REQ_GAP/BOUNDARY_OPEN/REQ_CONFLICT/FLOW_INCOMPLETE/ACCEPTANCE_MISSING/
NEEDS_HUMAN_DECISION/DESIGN_RISK）。各项均**显式延期**至 G2/G5 并记明 owner+门禁，据此 WS 模块级
G1 具备闭合条件。晋级由人类宣布（APC §2，AI 不得自行宣布晋级）。

| 编号 | 分类 | 状态 | 处置 |
|---|---|---|---|
| WS-01 | BOUNDARY_OPEN | 显式延期 | workspace.db 完整列/索引与 SYS 联合敲定（承接 SY-03），归 A-SYS-I-P-01 + WS G2 |
| WS-02 | FLOW_INCOMPLETE | 显式延期 | save 保存 / diff 落盘 / 时间轴还原 状态机归 WS L2（M-S，承接 SY-04、X-SEQ-003） |
| WS-03 | DESIGN_RISK | 显式延期 | 时间轴节点准入规则 / 存储模型 / 还原执行链取材旧文档四件，归 WS G2 |
| WS-04 | NEEDS_HUMAN_DECISION | 显式延期 | 缓存策略与多 workspace 控制态隔离属 L2 技术选型，延 WS G2 |
| WS-05 | ACCEPTANCE_MISSING | 部分关闭 | 定性验收已入 §5 各 REQ-WS-*；量化指标（I/O 延迟、恢复耗时）延 G5 NFR 标定（APC §9） |
| WS-06 | IMPLEMENTATION_GAP | 设计已闭合，待实现 | CreateDocumentBundle 的先建盘、初始内容 Diff 与三重基线安全拒绝补偿须按 REQ-WS-006 和 A-WS-M-T-02 / M-S-02 实现 |
| WS-07 | IMPLEMENTATION_GAP | 设计已闭合，待实现 | WorkspaceOperationPlan、DecisionRequest 前置裁决、确认时核验、直接磁盘执行、fail-fast 补偿与 Interrupted 恢复须按 A-WS-M-T-02 / M-S-02 实现；旧逻辑文件树/后置保存链必须迁移 |
| WS-08 | DESIGN_RISK | 已识别，待 G2 | TXT↔Markdown 转换目标的来源快照、未保存元数据、显式保存、冲突/失败、时间轴与恢复需按 REQ-WS-009 联合 AG/DE/DOC/ED/SYS 设计；不得改写来源或由现有直接文件工具绕过。 |
| WS-09 | IMPLEMENTATION_GAP | 设计已闭合，待实现 | 删除/恢复须迁移到前置裁决、类型化 WorkspaceOperationEntry 与统一 WorkspaceTimeline；现有不完整删除快照不能承担恢复。 |
| WS-10 | DESIGN_RISK | 已识别，待 G2 | 多既有文档冻结目标清单后的逐目标控制态、分流保存、失败隔离、事件排序和无批量事务边界，须按 REQ-WS-011 联合 DE/ED/AG/REF/SYS 设计。 |

> 术语：workspace、文档时间轴等 WS 主责核心概念将在 WS L2 注册 TERM（解除 A-SYS-I-P-01 §6 的延期），
> 本文 L1 不注册 TERM（ADU §3.2，TERM 在 L2 技术设计层注册）。

## §8 后续动作

1. 待人类宣布 WS 模块级 G1 闭合后，进入 WS G2（L2 技术设计，规则 SSOT），按路线图：
   - workspace 模型与生命周期、文件系统与跨文档操作（承接 REQ-WS-001/002）；
   - 提案控制态与显式保存、save/落盘/还原状态机（承接 REQ-WS-003/006/009/010/011，解除 SY-04）；
   - 文档级时间轴存储模型与还原执行链（承接 REQ-WS-004，取材台账 §4.3 时间轴四件）；
   - workspace.db 完整 schema 与 SYS 联合（承接 REQ-WS-005，解除 SY-03）；
   - 在 WS L2 注册 workspace / 时间轴相关 TERM。
2. G2 各文档经 REQ_MAP 把 REQ-WS-* 映射为 RULE/CHAIN/CONSTRAINT/TERM；状态机入 WS M-S 并登记 A-SYS-M-S-01 总图。
3. WS 职责或 workspace 边界变更先经 APC 授权再回写本文（APC §13）。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-10 | v1.0 | 初版：WS 工作台模块级 L1；声明 REQ-WS-001~005（workspace 生命周期与边界 / 文件 I/O 与 save / pending_diffs 持久化与落盘 / 文档级时间轴回溯 / 控制态缓存与重启可恢复），承接 REQ-CORE-001/002/007 与 A-SYS-I-P-01 契约；代表性流程 W1–W3；边界闭合（含与 DE/SYS 边界）与 G1 暴露清单（WS-01~05 显式延期/部分关闭，对接 SY-03/SY-04） |
| 2026-07-29 | v1.1 | BDOCX-IMAGE-AGENT-EDIT-01：REQ-WS-002 增加 workspace 项目图片只读发现、相对普通文件与符号链接/越界保护验收。 |
| 2026-08-07 | v1.2 | DOC-SYNC-01 回填（L1 口径，承接 L2/接口）：WS-NEW-BUTTON-01「＋」新建按钮下拉（格式选择 + 激活区域 + 文件夹入口，落地 REQ-WS-002 新建语义，命令见 A-WS-I-P-01）；WS-DIR-DELETE-01 文件夹删除两级语义（空快速删/非空确认递归，`ws.delete_folder` 见 A-WS-I-P-01 §3）；WS-SAVE-TIMELINE-01 保存成功记时间轴节点（REQ-WS-004 验收口径：保存成功且内容变化时记节点，A-WS-M-T-01 v1.7 已定档）。 |
| 2026-08-13 | v1.3 | 依据 REQ-CORE-010 与 D-NEW-001~008，将 W-J3/W2/REQ-WS-003 从“accepted 落盘”改为“提案控制态 + 显式保存落盘”；新增 REQ-WS-006，锁定未保存 LogicalState 不进文件树、默认不跨会话可读及保存失败保留边界。 |
| 2026-08-13 | v1.4 | DIFF-STATE-COMPASS-01：WS L1 对齐 BR-CORE-STATE-001~003，明确已有 LogicalState 仅保存写盘、无 LogicalState 接受原子写盘、pending 打开迁移至前者。 |
| 2026-08-13 | v1.5 | AG-1.0-R2-REQUIREMENTS-01：REQ-WS-002 承接项目感知 P0 的两阶段读取——先结构概览，正文按任务最小化读取；未打开文档标识 DiskState 新鲜度，已打开 LogicalState 不得被磁盘旧内容替代。 |
| 2026-08-13 | v1.6 | AG-1.0-R3-REQUIREMENTS-01：新增 REQ-WS-007，原生 DOCX 收敛为只读来源，编辑请求的 BDocx 副本仅在显式保存时创建为新的 DOCX；原文件全程不可被编辑或写回。 |
| 2026-08-14 | v1.7 | AG-1.0-R4-STRUCTURE-REQUIREMENTS-01：新增 REQ-WS-008，建目录、单文件重命名/移动改为逻辑文件树预览与逐项显式保存；现有直写文件操作登记为后续迁移债务。 |
| 2026-08-14 | v1.8 | AG-1.0-R4-CONVERSION-REQUIREMENTS-01：新增 REQ-WS-009；TXT→Markdown、Markdown→TXT 的目标副本预览后走未保存 LogicalState 与显式保存链，来源不改写；保存冲突/失败保留目标，HTML 与通用 DOCX 转换继续排除。 |
| 2026-08-14 | v1.9 | AG-1.0-R4-DELETE-MULTIDOC-REQUIREMENTS-01：新增 REQ-WS-010/011；删除/恢复先进入红色逻辑暂存，单项显式保存且递归/来源/脏会话再核验后才改变磁盘，永久删除不纳入；多个既有文档按确认冻结目标清单逐项保存并隔离失败，不提供批量保存或事务。 |
| 2026-08-21 | v1.10 | 工作区资源操作基线重构：REQ-WS-006/008/009/010 改为 CreateDocumentBundle、WorkspaceOperationPlan + DecisionRequest 前置裁决、确认后直接 DiskState 执行、fail-fast 逆序补偿与统一 WorkspaceTimeline。 |
| 2026-08-22 | v1.11 | WS-RESOURCE-WRITE-GUARD-01：REQ-WS-008/010 acceptance 补充人工文件树入口口径——保持直写但删除/重命名遵守 BR-WS-STATE-009（dirty 阻断、快照/时间轴可恢复、Diff 失效）。 |
