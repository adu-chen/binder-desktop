---
文档编号：DE-M-D-01
文档状态：A
负责模块：DE
文档职责：DE Diff Edit 模块级功能需求描述——声明模块级 REQ-DE-*，承接产品级 REQ-CORE-002/010/013/014/015/017，界定提案生成 / 受控预览 / 提案裁决（接受/拒绝/失效）/ diff 内容载荷形状 / 基准推进后的失效处理
上游约束：A-CORE-C-D-01_产品概述.md（REQ-CORE-002/010/013/014/015/017）、A-CORE-X-D-02_Agent1.0冻结能力基线.md（P0/P1 与不可突破边界）、A-WS-M-T-02_工作区资源操作技术设计.md、A-CORE-C-P-01_APC.md（§4.1 DE 职责、§6 diff 终态核心链）、A-SYS-C-T-01_系统总体架构.md、A-SYS-I-P-01_数据结构定义.md、A-ED-M-T-01_编辑器技术设计.md、A-AG-M-D-01_代理功能需求描述.md、ADU.md（§3.2）
直接承接：DE L2 技术设计（diff 模型 / 审阅契约 / 终态机）经 REQ_MAP 映射 RULE/CHAIN/CONSTRAINT/TERM；DE G2 建 diff 终态机（解除总图 planned 行）并完成 X-INTER-001 与 SM-WS-02 的 DE 侧对接
使用边界：本文是 L1 功能需求描述层，仅声明 REQ-DE-*，**不定义技术规则**；**代码不得直接映射 REQ**（ADU §3.2）；提案控制态持久化与显式保存落盘归 WS；diff 预览承载面归 ED；AI 产出生成归 AG；共享字段与状态集合归 SYS（DE 承接转移语义不重定义字段）
变更要求：DE 职责或 diff 终态语义变更须先经 APC 授权再回写本文；status 枚举集合变更须回写 A-SYS-I-P-01（X-SHARED-001）；REQ 增删按 ADU §3.2
治理成熟度：draft
---

# Binder Diff Edit 功能需求描述（DE · L1）

## §1 文档定位

本文是 **DE 模块**的 L1 功能需求描述，P3 第二件：

```
产品概述(REQ-CORE-002) → 本文(REQ-DE-*) → REQ_MAP → DE L2 规则(SSOT) → 代码(@GOV)
```

DE 是产品核心交互「**对话生成提案 → 受控预览 → 人类裁决 → 显式保存**」的中枢模块：把 AG 的
AI 产出生成为 ChangeProposal/diff，经 ED 验证并投影到可编辑的 LogicalState，再呈现给人类审阅。
已有 LogicalState 时，显式接受、拒绝或失效只裁决该提案，不重复应用预览内容、也不直接写盘；保存当前
LogicalState 时，仍有效 pending 提案一并被裁决为接受并由 WS 进入 DiskState。无 LogicalState 时，提案只临时挂载，用户接受后才由 WS 原子写盘。
DE 依赖 ED（预览/呈现锚定）、WS（提案控制态与保存）、AG（产出来源）；三分流的 L2 同步以
`BR-CORE-STATE-001~003` 为准。

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
> 持久化与重启恢复归 WS；只有**持续无 LogicalState** 的 accepted 才进入 WS（SM-WS-02）写文件，
> DE 的该裁决信号是 SM-WS-02 的唯一入场驱动（X-INTER-001）。

## §3 能力范围（DE v1）

**IN（DE 承载）**：
- 自 AG 产出（代理回合/工具回合的修改建议）生成 pending_diff，按 blockId 定位目标内容块。
- diff 审阅交互：经 ED 装饰层呈现修改对比与接受/拒绝控件，收集用户裁决。
- 终态转移语义：pending→accepted / rejected / expired；终态不可逆、互斥。仅目标无 LogicalState 的
  accepted 通知 WS 落盘并接收回执；已有 LogicalState 的显式 accepted 只改变卡片生命周期，保存有效 pending 预览会一并转 accepted 后写盘。
- payload（diff 内容载荷）形状定义与版本化。
- 过期裁决：超时与目标文档外部变更检测触发 expired。

**OUT（非 DE）**：
- pending_diff 持久化、重启恢复，以及无 LogicalState 分支 accepted 后写文件与时间轴记节点——归 WS
  （SM-WS-02/BR-WS-STATE-001）。
- 装饰层承载机制与 blockId 稳定性——归 ED（DE 是消费方，锚失效信号由 ED 提供）。
- AI 产出的生成（模型调用、门禁）——归 AG；多文件构建产物——归 CBT（其落盘同样走 diff 路径，
  契约联合 CBT G2）。
- PendingDiff 字段集合与 status 枚举值集合——归 SYS A-SYS-I-P-01（DE 承接引用）。

### §3.1 1.0 文档生命周期基线

DE 的提案必须记录其生成所依据的内容基准，并按 `BR-CORE-STATE-001~003` 分流。目标已有
LogicalState 时，经 ED 成功预览后候选结果直接成为可编辑 LogicalState；显式接受只确认该提案，保存有效 pending 预览也确认该提案，拒绝仅在
原始基准仍有效时撤销它。目标尚无 LogicalState 时，候选只临时挂载于 DiffCard；接受才通知 WS 原子
落盘。pending 期间用户打开目标文档时，ED 必须以该提案建立带绿色增量的 LogicalState，卡片随即迁移为
已有 LogicalState 路径，不得再后台落盘。用户手动编辑、后续提案成功预览、外部变更或超时使原始基准
失效时，旧提案进入失效态，接受/拒绝控件不可再操作，且不得回滚当前 LogicalState。工作区资源操作不进入
DE 的 DiffCard 生命周期。CreateDocumentBundle 在 WS 已确认并创建目标 DiskState、ED 已打开标签后，才把预生成
初始内容作为普通 ChangeProposal 投影；其内容卡生命周期仍遵循上述规则，安全拒绝补偿由 WS 依据三重基线执行。

## §4 代表性流程（契约级，APC §7）

**DE1 · diff 生成与审阅裁决**（DE-J1/J2/J3，承接 REQ-CORE-002）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | AG | AI 修改建议 | DE 按当前 LogicalState 基准和 blockId 生成 ChangeProposal/diff | 待预览提案 | 2 | 锚失效→生成失败回报 AG，不产生错位提案 |
| 2 | DE/ED | 待预览提案 | 有 LogicalState 时受控预览进入内容；无 LogicalState 时只临时挂载 DiffCard | 可编辑预览 + 可审阅提案，或后台待裁决卡片 | 3 | 预览失败→不改变 LogicalState；锚失效→提案失效 |
| 3 | 用户 | 接受/拒绝，或 pending 时打开目标文档 | 有 LogicalState 时显式接受不重复应用，拒绝撤销预览；无 LogicalState 时接受触发原子落盘；打开动作迁移为前者 | 已接受、已拒绝、已落盘或已迁移提案 | 4 | 手动编辑/后续预览/外部变更→失效，控件不可用 |
| 4 | 用户→WS | 显式保存当前文档（仅已有 LogicalState） | WS 将当前 LogicalState 写入 DiskState，并将仍有效 pending 预览裁决为 accepted/awaiting_save | 已落盘 + 时间轴节点 + 已接受卡片 | 终态 | 保存失败→逻辑状态、标签与可编辑性保留 |

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
summary: 将 AG 的一次结构化内容修改生成为 ProposalBatch，并按可独立人工裁决的连续块组拆成一个或多个 pending_diff/DiffCard（PendingDiff 共享结构，blockId 定位目标内容块），交 WS 持久化；不连续块默认分卡，相邻且共同表达一个不可分修改的块可合并；定位锚失效时生成失败并回报，不产生错位 diff
source_doc: REQ-CORE-002、A-SYS-I-P-01（PendingDiff）、A-ED-M-T-01（BR-ED-DATA-001）
acceptance: AI 修改建议可生成为持久化 ProposalBatch 与一个或多个定位准确的 pending_diff；每张卡只覆盖一个可独立接受/拒绝的连续块组，同批卡保留 batch 身份和顺序但裁决互不捆绑；不连续块不得仅因同一次工具调用合并为一张卡；锚失效有明确失败回报
open_issues: 分组算法、批次字段和相邻不可分判定归 DE G2；裁决原则已由本 REQ 固定
-->

<!-- REQ
req_id: REQ-DE-002
title: diff 审阅呈现
owner_module: DE
status: active
summary: 经 ED 装饰层或 DiffCard 按 ProposalBatch 中的独立裁决单元呈现文本、已确认样式以及 HTML 编辑文档/BDocx 处理副本的表格语义修改对比和接受/拒绝控件，用户交互由 ED 转达 DE 裁决；字符基础样式为 P0，颜色/突出显示、段落样式、字号与字体族为 P1；原生 DOCX 不作为可修改目标；终态后装饰复位不残留
source_doc: REQ-CORE-002、A-ED-M-T-01（ED-DECOR、BR-ED-STATE-002）
acceptance: diff 可视呈现于编辑器目标位置，同批多卡显示共同批次关系、各自目标范围和独立接受/拒绝状态，不以一个总裁决替代独立裁决；HTML 编辑文档与 BDocx 处理副本的样式操作必须展示目标范围、操作名称和修改前后结果，不显示原始 HTML、CSS 或 OOXML；HTML 编辑文档与 BDocx 处理副本的表格操作按坐标/参数展示，单元格底纹显示修改前后色块与十六进制色值；原生 DOCX 的编辑请求只可展示“创建 BDocx 编辑副本”的新文档 Proposal；裁决控件交互可达；终态后呈现复位
open_issues: 审阅 UI 形态（逐条/批量/侧栏）归 DE G2/实现期（§7 DE-04）
-->

<!-- REQ
req_id: REQ-DE-003
title: diff 终态裁决
owner_module: DE
status: active
summary: 拥有提案接受/拒绝/失效的转移语义：结果互斥；已有 LogicalState 时显式接受只裁决已预览提案，保存有效 pending 预览也裁决为接受，拒绝只在原始基准仍有效时撤销预览，失效不得覆盖后续 LogicalState；无 LogicalState 时接受才触发 WS 原子写盘，pending 打开后迁移为前者
source_doc: REQ-CORE-002/010、A-CORE-X-D-02 §3/§4、A-SYS-I-P-01（状态集合）
acceptance: 已有 LogicalState 时显式接受不重复应用预览内容且不触发写盘；保存有效 pending 预览时将其裁决为 accepted/awaiting_save 后使当前 LogicalState 落盘；无 LogicalState 时接受原子写盘；pending 打开后建立带绿色增量的 LogicalState 并不得后台写盘；拒绝只撤销有效基准上的对应预览；失效后接受/拒绝不可执行
open_issues: 提案终态机、基准比较和与保存状态机的跨模块转移归 DE/WS/ED G2 联合定义
-->

<!-- REQ
req_id: REQ-DE-004
title: diff 内容载荷形状
owner_module: DE
status: active
summary: 定义 ProposalBatch 与 payload_ref 所指 diff 内容载荷的结构（批次身份、分组顺序、独立裁决单元、修改前后内容、已确认样式操作、HTML 编辑文档/BDocx 处理副本的 TableOperation 与 ImageOperation、定位信息、来源元数据），为 A-SYS-I-P-01 显式留归 DE 的占位补形；原生 DOCX 编辑请求的载荷目标是新副本 Proposal；载荷形状版本化以支持迁移
source_doc: A-SYS-I-P-01 §3（payload 形状归 DE）
acceptance: 载荷形状有单一定义且 WS 持久化/恢复后可完整重建 ProposalBatch 关系、每张卡的连续目标组、独立裁决状态以及文本、样式、表格与图片审阅呈现；已确认样式操作保留其目标范围、操作和前后结果，不退化为原始 HTML、CSS 或 OOXML 替换；table_ops 保留操作和前后 TableSnapshot，image_ops 保留操作、项目来源、锚和前后 ImageSnapshot，不退化为原始 HTML 替换
open_issues: 载荷具体结构与版本化策略归 DE G2
-->

<!-- REQ
req_id: REQ-DE-005
title: 过期与外部变更处理
owner_module: DE
status: active
summary: 待裁决提案在超时、目标内容外部变更、用户手动编辑或后续提案成功预览时转失效；失效不得覆盖已推进的 LogicalState，也不可再接受或拒绝
source_doc: REQ-CORE-002/010、A-CORE-X-D-02 §3/§4、A-WS-M-D-01（REQ-WS-006）
acceptance: 任一基准推进信号使旧提案失效且文件不因失效被改写；检测不可得时不得自动回滚当前 LogicalState
open_issues: 基准版本、内容哈希和超时策略归 DE G2（§7 DE-03）
-->

<!-- REQ
req_id: REQ-DE-006
title: CreateDocumentBundle 初始内容 Diff 的预览与裁决
owner_module: DE
status: active
summary: CreateDocumentBundle 的 WorkspaceOperationPlan 确认且 WS 创建目标 DiskState 后，DE 才把预生成的初始内容作为带 operation_id/origin 的普通 ChangeProposal 投影到已打开目标；转换来源不是 DE 的可写目标。资源创建授权不由内容 Diff 的接受或拒绝替代
source_doc: REQ-CORE-010/013、A-CORE-X-D-02 §3/§4、A-WS-M-T-02 §6
acceptance: 初始内容预览后可继续手动或代理编辑；接受仅改变 Diff 卡生命周期，显式保存写入既有目标 DiskState；拒绝请求只在资源、DiskState 与 LogicalState 三重基线均未变化时由 WS 删除新建目标并关闭标签，任一基线推进使内容卡失效且目标继续存在；DE 不创建/删除资源、不改写转换来源、不把 Markdown→TXT 伪称无损
open_issues: 无；跨模块顺序和补偿由 A-WS-M-T-02 / A-WS-M-S-02 定义
-->

<!-- REQ
req_id: REQ-DE-007
title: 工作区资源操作与内容 Diff 隔离
owner_module: DE
status: active
summary: 创建、删除、重命名、移动、恢复等工作区资源操作不生成 DE DiffCard，不使用内容 Diff 的 pending/accepted/rejected 状态；它们由 WorkspaceOperationPlan + DecisionRequest 前置裁决并由 WS 执行。DE 只向计划提供受影响内容 Diff 清单，并在资源操作成功后接收失效事件
source_doc: REQ-CORE-012/014、A-CORE-X-D-02 §3/§4、A-WS-M-T-02
acceptance: 资源决策卡不得以“AI 修改建议”或 DiffCard 呈现；脏标签由 WS/ED 阻断资源执行；DecisionRequest 确认前有效内容 Diff 仍保持原状，确认并成功执行后受影响卡以 resource_baseline_changed 原因失效；资源操作失败或补偿完成前不得伪报内容卡已安全迁移；CreateDocumentBundle 初始内容 Diff 是唯一资源创建后继续进入 DE 的内容提案
open_issues: 无；资源操作状态机归 A-WS-M-S-02，DE 仅承接内容卡影响事件
-->

<!-- REQ
req_id: REQ-DE-008
title: 多文档目标计划与逐项独立审阅
owner_module: DE
status: active
summary: 多个既有文档修改前，DE 承接 AG 交付的可见目标计划确认；确认后目标成员、路径和身份冻结。每个目标单独拥有 Proposal、内容基准、预览、卡片终态和保存结果，按其自身 LogicalState/DiskState 分流；不得以一个跨文档总卡替代逐项审阅，也不得提供批量接受、批量保存或全有或全无裁决
source_doc: REQ-CORE-015、A-CORE-X-D-02 §3/§4、A-AG-M-D-01（REQ-AG-014）
acceptance: 任何既有文档写 Proposal 的目标均属于已确认冻结清单；每张卡只指向一个目标并可独立显示基准、预览、失效和保存状态；某一目标冲突、外部变化、结构变化或删除只使其自身卡失效，不静默跳过或改变其他目标卡；有 LogicalState 与背景目标分别遵循既有三分流；原生 DOCX、删除/文件树整理/格式转换混合操作不进入本多文档写入审阅路径
open_issues: 目标计划卡与逐文档卡关联、独立基准/失效信号、呈现排序、并发、保存状态和 Trace/恢复契约归 DE/AG/ED/WS/REF/SYS G2 联合定义；本 L1 不定义批处理或事务接口
-->

<!-- REQ
req_id: REQ-DE-009
title: 复合任务内连续 Proposal 的基准接力
owner_module: DE
status: active
summary: 对同一复杂任务投影中明确的同文档连续编辑，DE 为每个 ProposalBatch 及其 DiffCard 保留任务/回合历史关联，后续 ProposalBatch 只可基于上一批成功预览后的当前 LogicalState 生成。同一批内的卡按各自连续块组独立裁决；后续批预览推进基准时，前批 pending 卡**不再自动失效**，保留待裁决；接受/拒绝时按各自 base 校验，实际冲突才失败并说明。多卡并存不构成补丁栈（各自独立 base/范围），拒绝不产生任务级回滚。无 LogicalState 的 background pending Proposal 不得作为后续编辑输入
source_doc: REQ-CORE-017、REQ-AG-015、A-CORE-X-D-02 §3/§4、REQ-DE-003/005
acceptance: Batch1 预览进入 LogicalState 后，Batch2/3 的 before/base 均来自最新 LogicalState 而非 Batch1 payload；同批不连续或可独立裁决块可分别接受/拒绝且结果只作用于各自范围；后续批成功预览不自动失效前批 pending 卡，前批卡保持可裁决，接受/拒绝时若实际基线冲突则失败并保留卡片；拒绝最新卡只依其有效基准恢复对应范围；背景 pending 目标必须先打开迁移为带绿色增量的 LogicalState，或按背景分流完成裁决后才可继续，不能后台叠加或错误写盘；任务取消、失败或中断不会把已成功预览内容伪装为未发生或自动回滚，卡片仍按各自生命周期呈现
open_issues: TaskRun/步骤/Proposal 关联、基准版本与失效原因、展示排序、跨模块事件、历史恢复、并发与 UI 投影归 DE/AG/ED/SYS DESIGN Scope；本 L1 不定义字段、状态机、事务、回滚或接口
-->

## §6 模块边界与约束（边界闭合）

- **预览与写入边界**：已有 LogicalState 的 DE 受控预览只能更新 LogicalState；显式接受/拒绝裁决提案，
  保存仍有效 pending 预览时也先裁决为接受，再作为唯一写盘入口；无 LogicalState 的临时挂载提案仅可按 BR-CORE-STATE-002 在用户接受后由
  WS 原子写入。DE 与 AG/ED/WS 共同保障该边界。
- **共享结构边界**：PendingDiff 字段与 status 枚举集合归 SYS（X-SHARED-001），DE 拥有转移语义与
  payload 形状，不重定义字段。
- **呈现边界**：DE 经 ED 装饰层呈现（X-FORBID-005：不绕过 ED 接口直改编辑器状态）。
- **资源操作边界**：删除、恢复、创建、重命名和移动不生成 DE Proposal/DiffCard；它们由 WorkspaceOperationPlan + DecisionRequest 前置裁决并由 WS 执行。DE 只处理 CreateDocumentBundle 创建目标后的初始内容 Diff，以及资源执行成功导致的内容卡失效。
- **多文档边界**：多文档写入先经 AG 的目标计划确认并冻结成员；DE 只管理逐目标独立 Proposal/基准/裁决/失效，不提供总卡批量裁决、批量保存或跨文档事务。
- **连续 Proposal 边界**：同文档连续任务以当前 LogicalState 接力，不以旧卡 payload 叠加；后预览推进基准不自动失效前批 pending 卡，多卡并存、各自按 base 裁决；实际基线冲突在接受/拒绝时失败并保留卡片，拒绝最新卡不构成任务级回滚。background pending 卡必须先迁移或按背景分流裁决，不能作为续编基准。
- **状态机边界**（APC §6）：diff 终态为状态化核心链（总图 owner=DE，planned），DE G2 建机
  （X-SEQ-003）并登记 X-INTER-001 DE 侧。
- **L4 原料边界**：取材台账 §4 Diff Edit/Decoration 原料（R-ED-M-R-22 等 P2），原料不为权威。

## §7 G1 问题暴露清单

| 编号 | 分类 | 状态 | 处置 |
|---|---|---|---|
| DE-01 | BOUNDARY_OPEN | 需求已关闭，待设计 | 人工裁决粒度固定为连续且语义不可分的块组；不连续或可独立裁决块必须分卡，组内字符/行对比算法归 DE G2 |
| DE-02 | DESIGN_RISK | 显式延期 | 同一文档多 pending_diff 并存的顺序、重叠与冲突处理归 DE G2（影响落盘正确性，联动 WS） |
| DE-03 | BOUNDARY_OPEN | 显式延期 | expired 触发细则（超时时长、外部变更检测机制）归 DE G2 |
| DE-04 | NEEDS_HUMAN_DECISION | 显式延期（不阻断 G1） | 审阅 UI 形态（逐条/批量裁决、侧栏/内联）属产品体验决策，延 G2/实现期人类确认 |
| DE-05 | ACCEPTANCE_MISSING | 部分关闭 | 定性验收已入 §5；量化指标（diff 生成/呈现延迟）延 G5 NFR 标定 |
| DE-06 | IMPLEMENTATION_GAP | 设计已闭合，待实现 | CreateDocumentBundle 初始内容 Diff、operation_id 关联、三重基线安全拒绝与卡片失效须按 REQ-DE-006 和 A-WS-M-T-02 / M-S-02 实现 |
| DE-07 | DESIGN_RISK | 已识别，待 G2 | TXT↔Markdown 目标副本的转换卡、来源快照、保真/损失呈现、目标基准失效与保存错误需按 REQ-DE-006 联合 AG/DOC/ED/WS 设计；不得把转换来源加入写入目标或把 BDocx 误作通用转换。 |
| DE-08 | IMPLEMENTATION_GAP | 设计已闭合，待实现 | 资源删除/恢复卡必须从 DE DiffCard 路径移除，改由 DecisionRequest + WorkspaceOperation；DE 只维护受影响内容 Diff 的失效事件。 |
| DE-09 | DESIGN_RISK | 已识别，待 G2 | 多既有文档的目标计划确认/冻结、逐目标 Proposal/基准/失效/保存、现有宽松 cross_doc 的迁移与无批量事务边界，须按 REQ-DE-008 联合 AG/ED/WS/REF/SYS 设计。 |
| DE-10 | FLOW_INCOMPLETE | 已识别，待 G2 | REQ-DE-009 的 TaskRun/回合关联、当前 LogicalState 接力、前卡失效原因、background pending 限制、取消/中断后的卡片历史及并发投影须联合 AG/ED/SYS 设计；不得通过补丁堆叠或任务级回滚绕过三分流。 |

> 术语：diff / pending_diff 为 DE 主责核心概念（A-SYS-I-P-01 §6 已显式延期至 DE），DE L2 注册
> TERM；本文 L1 不注册。

## §8 后续动作

1. 待人类宣布 DE 模块级 G1 闭合后进入 DE G2：ProposalBatch 字段与连续块分组算法、多 diff 冲突
  （解除 DE-02）、payload 形状（REQ-DE-004）、基准失效细则（解除 DE-03）、提案/预览/保存状态机
  （REQ-DE-003/006/007/008，回填总图及 ED/WS 跨模块转移）、注册 diff 相关 TERM。
2. G2 经 REQ_MAP 把 REQ-DE-001~008 映射为 RULE/CHAIN/CONSTRAINT/TERM。
3. CBT G1/G2 时联合确认构建产物走 diff 路径的契约。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-11 | v1.0 | 初版（P3 第二件）：DE 对话编辑模块级 L1；声明 REQ-DE-001~005（diff 生成 / 审阅呈现 / 终态裁决 / 载荷形状 / 过期与外部变更），承接 REQ-CORE-002 与 A-SYS-I-P-01 PendingDiff 契约，对接 SM-WS-02（X-INTER-001 WS 侧已就绪）与 ED-DECOR；代表性流程 DE1/DE2；G1 暴露清单 DE-01~05 |
| 2026-07-04 | v1.1 | 术语统一清扫（承 2026-07-04 人类三轮裁决）：正文禁用别名替换——内联辅助→局部修改、自动补全→辅助续写、对话编辑→Diff Edit、Chat↔Build 接管→构建移交/回落、chat/chat-agent 回合→代理回合；文件名与变更记录历史行不改写 |
| 2026-07-27 | v1.6 | DE-SAVE-IMPLICIT-ACCEPT-01：人类决策固化——当前文档的任意保存是已应用 pending Diff 的接受动作；保存失败保留 accepted 待保存，不得将前端显示伪装为已保存。 |
| 2026-07-28 | v1.7 | BDocx TableCellShading：REQ-DE-002/004 纳入 TableOperation 载荷以及单元格底纹按坐标、前后色块和十六进制色值的精确审阅呈现。 |
| 2026-07-29 | v1.8 | BDOCX-IMAGE-AGENT-EDIT-01：REQ-DE-004 纳入 ImageOperation、项目来源/锚与图片前后快照，要求持久化后可重建 structured 图片审阅。 |
| 2026-08-13 | v1.9 | 依据 REQ-CORE-010 与 D-NEW-001~008，将 DE 链路改为「提案生成→受控预览进入 LogicalState→人类裁决→显式保存写入 DiskState」；接受不再触发落盘，新建提案新增 REQ-DE-006，手动/后续编辑导致旧提案失效并列为 DE G2 状态机输入。 |
| 2026-08-13 | v1.10 | DIFF-STATE-COMPASS-01：REQ-DE-003/DE1 对齐 BR-CORE-STATE-001~003：已有 LogicalState 预览后保持 pending，背景无 LogicalState 卡片仅挂载并在接受后落盘，pending 期间打开目标文档迁移为绿色预览后再裁决。 |
| 2026-08-13 | v1.11 | 人类复核：保存有效 pending 预览不是绕过裁决，而是先裁决为 accepted/awaiting_save 后写盘；基线推进后保存当前逻辑内容并将旧卡失效。 |
| 2026-08-13 | v1.12 | AG-1.0-R3-REQUIREMENTS-01：将表格/图片 diff 限定为 HTML 编辑文档与 BDocx 处理副本；原生 DOCX 的编辑请求只生成创建新副本的 Proposal，不成为可修改目标。 |
| 2026-08-14 | v1.13 | AG-1.0-R3-STYLE-REQUIREMENTS-01：样式 Diff 收紧为已确认的 HTML/BDocx 受控操作；明确 P0/P1 档位、可读前后对比和禁止原始 HTML/CSS/OOXML 载荷。 |
| 2026-08-14 | v1.14 | AG-1.0-R4-CONVERSION-REQUIREMENTS-01：REQ-DE-006 扩展承接 TXT→Markdown、Markdown→TXT 的目标副本 Proposal；转换目标预览后沿用新建逻辑文档、卡片裁决、基准失效与显式保存链，来源不进入写入目标。 |
| 2026-08-14 | v1.15 | AG-1.0-R4-DELETE-MULTIDOC-REQUIREMENTS-01：新增 REQ-DE-007/008；删除/恢复卡只预览红色逻辑暂存、接受不执行物理变更、显式保存才交 WS；多既有文档先确认冻结目标计划，再以逐目标独立 Proposal/基准/裁决/保存处理，不提供批量裁决或事务。 |
| 2026-10-31 | v1.19 | PI-AG-CONTINUATION-13：REQ-DE-009 改为多批 pending 卡并存，后预览不自动失效前卡；接受/拒绝时按各自 base 校验，实际冲突才失败；仍禁止旧 payload 叠补丁与任务级回滚。 |
| 2026-08-14 | v1.16 | AG-1.0-R5B-TASK-ORCHESTRATION-REQUIREMENTS-01：新增 REQ-DE-009，固定复合任务中连续 Proposal 必须以最新 LogicalState 接力、后预览使前卡基准推进失效；禁止旧 payload 续编、补丁堆叠和任务级回滚，background pending 先迁移或裁决才可继续。 |
| 2026-08-21 | v1.17 | 工作区资源操作基线重构：REQ-DE-006 改为 CreateDocumentBundle 创建磁盘目标后的初始内容 Diff；REQ-DE-007 明确资源操作不生成 DiffCard，DE 仅提供影响清单并接收资源基线变化导致的内容卡失效。 |
| 2026-09-03 | v1.18 | AG-ADAPTIVE-RUNTIME-REQ-01：REQ-DE-001/002/004/009 引入 ProposalBatch 与可独立裁决连续块组；不连续块分卡、相邻不可分块可合并，同批关系不捆绑裁决。 |
