---
文档编号：ED-M-S-01
文档状态：A
负责模块：ED
文档职责：ED 编辑器模块级状态机设计——文档编辑会话机（SM-ED-01），含 APC §6 必备字段；登记 A-SYS-M-S-01 总图，登记 ED↔WS 脏态/保存 X-INTER-001 跨模块转移契约；说明 decoration 呈现不独立建机的豁免依据
上游约束：A-ED-M-T-01_编辑器技术设计.md（ED-EDIT/ED-DECOR 链路、BR-ED-STATE-001/002 规则）、A-CORE-C-P-01_APC.md（§6 状态机必备字段与强制阈值）、A-SYS-M-S-01_状态机总图.md（X-SEQ-003 注册前置、X-INTER-001 跨模块转移）、A-WS-M-S-01_工作台状态机设计.md（SM-WS-01 save 保存机）、A-SYS-I-P-02_IPC协议.md（统一错误形状）、ADU.md（§3.1 STATE_MACHINE 格式）
直接承接：SM-ED-01 经 rules 字段承载 A-ED-M-T-01 规则；machine_id 登记 A-SYS-M-S-01 总图；edit/saved 转移点对接 WS SM-WS-01（X-INTER-001）
使用边界：本文定义 ED 状态机内部；不重定义规则（归 A-ED-M-T-01）、不定义 SM-WS-01 内部（归 A-WS-M-S-01）、不为 decoration 呈现独立建机（依附消费方终态机与 SM-ED-01，APC §6 纯展示豁免，§4 说明）；inline-assist/autocomplete 归属未定，不预建其状态机
变更要求：transitions/forbidden_transitions 变更须同步 A-ED-M-T-01 对应规则并重跑测试矩阵；ED↔WS 转移点变更须与 WS 共同复核 X-INTER-001；machine_id 一经登记总图不可更改
治理成熟度：draft
---

# Binder 编辑器状态机设计（ED · L2 技术设计）

## §1 文档定位

本文承接 A-ED-M-T-01 的状态化核心链，按 APC §6 定义 ED 状态机并登记 A-SYS-M-S-01 总图：

```
A-ED-M-T-01（规则 SSOT：ED-EDIT）→ 本文 SM-ED-01（states/transitions）
本文 machine_id ──登记──→ A-SYS-M-S-01 §3 总图
SM-ED-01 edit/saved 转移点 ──对接──→ WS SM-WS-01（X-INTER-001 跨模块转移契约）
```

满足 X-SEQ-003：文档编辑会话为有状态/跨层/多步核心链（加载经 WS→编辑→脏态→save 经 WS→关闭），
在此建机后方可进入实现。decoration 呈现与分页不独立建机，豁免依据见 §4。

## §2 SM-ED-01 · 文档编辑会话机

<!-- STATE_MACHINE
machine_id: SM-ED-01
owner_module: ED
status: active
summary: 单文档编辑会话生命周期（加载经 WS / 编辑或同批 diff 原子预览脏态驱动 SM-WS-01 / 解析失败降级只读 / 脏态不静默关闭）
source_type: design
chain: ED-EDIT
rules: BR-ED-STATE-001, BR-ED-DATA-001, BR-ED-DATA-002, BR-ED-DATA-006, BR-ED-VERIFY-001, BR-CORE-STATE-001, BR-CORE-STATE-003
initial_state: Closed
terminal_states: Closed
-->

- **machine_id**：SM-ED-01　**owner_module**：ED
- **scope**：单个 workspace 内文档一次编辑会话（打开→编辑→保存协同→关闭）的完整生命周期
- **context**：目标 DocRef（A-SYS-I-P-01）、编辑模型（内容块树或非激活 session 序列化 LogicalState）、脏态标记、sessionVersion、contentHash、当前 ProposalBatch 共享基线/目标集合、加载/解析结果
- **states**：Closed、Loading、Ready、Editing、ReadOnly、Conflicted
- **events**：open、loadOk、parseFail、loadFail、edit、draftDiffApplied、sessionDiffApplied、pendingDiffPromoted、sessionApplyFail、serializeFail、saved、close、confirmDiscard、retry、
  externalChange、saveConflict、conflictKeepMine、conflictReload
- **transitions（主表）**：

| from | event | guard | to | action |
|---|---|---|---|---|
| Closed | open | 路径在 workspace root 内（X-FORBID-004，WS 侧守护） | Loading | 经 WS 读取内容 |
| Loading | loadOk | 内容按 editFormat 解析成功且 LogicalState 编码已声明 | Ready | Markdown 经 Codec 装载为终态模型；b-docx/.html 经 HTML 模型；text 逐行装载 |
| Loading | parseFail | 解析失败或命中无法无损承载结构 | ReadOnly | 保留原始源码并进入源码保护/只读呈现，不构造部分富文本后继续保存 |
| Loading | loadFail | — | Closed | 统一错误形状，会话不建立 |
| Ready | edit | — | Editing | 更新编辑模型；驱动 SM-WS-01 Idle→Dirty（X-INTER-001） |
| Ready | draftDiffApplied | 单卡，或同文档同批结构化卡已收齐、共享基线一致、目标互斥且聚合预检通过 | Editing | 单卡或整批只 dispatch 一次前向事务；逐卡 GreenReview/previewApplied，驱动 SM-WS-01 Dirty |
| Ready | sessionDiffApplied | 已有 LogicalState 且 base_session_version/base_content_hash 匹配 | Editing | 更新非激活 session LogicalState，递增版本、刷新哈希、持久化 `ed_session_snapshots`，驱动 SM-WS-01 Dirty；DiffCard 保持 pending 可裁决 |
| Ready | pendingDiffPromoted | 本次 open 后发现同路径 background PendingDiff，且同一 DiffPayload/基准可投影 | Editing | 在新建 LogicalState 中应用 after、GreenReview 标绿、取消后台落盘资格，驱动 SM-WS-01 Dirty |
| Ready | sessionApplyFail | 基线不匹配/锚失效/事务不支持 | Ready | 不修改 LogicalState；回 DE autoApplyFail |
| Ready | externalChange | 无脏态（BR-ED-STATE-003） | Loading | 自动重载磁盘内容 + 非阻断提示 |
| Editing | edit | — | Editing | 持续编辑，脏态保持 |
| Editing | draftDiffApplied | 单卡，或同文档同批结构化卡满足共享基线与聚合预检 | Editing | 单卡或整批只 dispatch 一次前向事务，脏态保持；重叠 diff 由 DE 冲突规则约束 |
| Editing | sessionDiffApplied / pendingDiffPromoted | 已有 LogicalState 或刚打开迁移，且基线匹配 | Editing | 经受控事务应用预览、保留用户裁决入口；重叠 diff 由 DE 冲突规则约束 |
| Editing | sessionApplyFail | 基线不匹配/锚失效/事务不支持 | Editing | 不修改 LogicalState；回 DE autoApplyFail |
| Editing | saved | SM-WS-01 达 Saved（X-INTER-001 回执） | Ready | 脏态清除；已 accepted 的 GreenReview 复位 |
| Editing | serializeFail | Markdown 序列化、语义校验或能力检查失败 | Editing | 禁止写盘、保持 dirty 与原 DiskState，提示可恢复错误 |
| Editing | externalChange / saveConflict | 有脏态（WS 事件/SM-WS-01 冲突回执） | Conflicted | 冲突提示，等待用户显式裁决 |
| Conflicted | conflictKeepMine | 用户显式选择保留己方编辑 | Editing | 脏态保留；后续 save 经 overrideConfirmed 覆盖确认 |
| Conflicted | conflictReload | 用户显式选择放弃并重载 | Loading | 丢弃脏态（显式）+ 重载磁盘内容 |
| Ready | close | — | Closed | 会话关闭 |
| ReadOnly | close | — | Closed | 会话关闭 |
| ReadOnly | retry | — | Loading | 重新加载解析 |
| Editing | close | 脏态已保存（saved 先行）或用户 confirmDiscard | Closed | 关闭；丢弃须经显式确认 |

- **error_states**：ReadOnly（解析失败降级态，recovery：retry→Loading）；loadFail 直接回 Closed
  （会话不建立，统一错误形状）
- **recovery_paths**：ReadOnly→retry→Loading；save 失败不改变本机状态（停留 Editing，SM-WS-01
  SaveError 侧重试，ED 保持脏态可重试）
- **forbidden_transitions**：
  - Closed → Editing（未经加载不得编辑）
  - Loading → Editing（加载未完成不得编辑）
  - ReadOnly → Editing（只读降级态不得编辑，须 retry 重载成功后方可）
  - Editing → Closed 且无 saved/confirmDiscard（脏态不得静默关闭，BR-ED-STATE-001）
  - Ready/Editing → sessionDiffApplied/pendingDiffPromoted 无 session 基线匹配（不得应用陈旧 DiffProjection）
  - Ready/Editing → 同文档同批结构化多卡逐卡 draftDiffApplied（必须收齐后聚合预检并一次 dispatch）
  - sessionApplyFail → 任何内容修改（失败必须无半应用状态）
  - serializeFail → Ready/Closed（未保存成功不得清脏或关闭；必须修复内容、切换源码保护或显式丢弃）
  - Markdown 非支持节点/样式 → saved（未经能力检查和语义往返校验不得写盘）
  - Editing（收到 externalChange）→ Loading 直达（**有脏态不得静默重载**，必经 Conflicted 显式裁决）
  - Conflicted → Ready/Editing/Loading 无用户显式选择（冲突不得自动消解，BR-ED-STATE-003）
- **acceptance_criteria**：唯一进入 Editing 的路径经 Ready；脏态会话关闭必经 saved 或显式
  confirmDiscard；parseFail 不丢内容（ReadOnly 可见）；loadFail 不建立会话；`sessionDiffApplied` 与
  `pendingDiffPromoted` 在基线匹配时修改 LogicalState 并持久化镜像、保留卡片裁决入口，基线不匹配时不修改；
  同文档同批结构化多卡只产生一次前向事务且批内不互相失效，逐卡反向只作用自身目标；Markdown serializeFail 保持 Editing/dirty 且 DiskState 不变；离线时 open/edit/close
  全链可用（不依赖在线鉴权）；外部变更下两个方向的内容损失（覆盖外部修改/丢弃己方脏态）均必经
  用户显式选择
- **test_matrix**：见 §5（强制覆盖上述 transitions 与 forbidden_transitions）

### 2.1 CreateDocumentBundle 与转换目标会话（REQ-ED-010/014）

承接 REQ-ED-010/014 与 A-WS-M-S-02 SM-WS-04。新建文档和最小转换目标副本**复用 SM-ED-01 会话机**，
但 ED 只在 WorkspaceOperation 已确认、重新核验成功且目标 DiskState 已创建后打开会话。
不再存在“无 DocRef/无 DiskState 的未保存新文档身份”：

| 场景 | SM-ED-01 承接 | 说明 |
|---|---|---|
| 创建成功开标签 | SM-WS-04 已创建空的真实目标后，ED 以 DocRef + DiskState 基线进入 Ready，自动打开可编辑标签 | TERM-WS-007；BR-WS-STATE-003 |
| 初始内容投影 | DE 以创建后空内容与当前 session 版本为基准投影普通 Diff；成功后进入 Editing/dirty | BR-CORE-STATE-001；CreateDocumentBundle.operation_id 绑定 |
| 接受只裁决 | 用户接受初始内容 Diff 时，DE 只改变卡片生命周期；ED 不重复应用内容，也不重复创建文件 | BR-CORE-STATE-001 |
| 安全拒绝 | operation_id、资源身份、空文件 DiskState 与 LogicalState 版本均未推进时，WS 补偿删除本次创建文件；成功后 ED 关闭标签 | BR-WS-STATE-003 |
| 基准推进 | 手动编辑、后续 Proposal、外部修改、重命名或移动任一发生 → 原卡失效，会话与真实文件保留 | BR-WS-STATE-003；BR-CORE-STATE-001/003 |
| 保存 | 用户保存时按普通已存文档链写入目标 DiskState，不执行创建语义 | SM-WS-01 |
| 转换目标副本 | TXT→Markdown、Markdown→TXT 也必须经 CreateDocumentBundle 先创建磁盘目标，再投影转换结果 Diff；来源不改写，Markdown→TXT 有损提示可见 | REQ-ED-014、REQ-WS-009 |

<!-- REQ_MAP
req_id: REQ-ED-010
target_type: RULE
target_id: BR-CORE-STATE-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-010
target_type: RULE
target_id: BR-WS-STATE-003
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-010
target_type: RULE
target_id: BR-SYS-STATE-008
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-014
target_type: RULE
target_id: BR-CORE-STATE-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-014
target_type: RULE
target_id: BR-CORE-STATE-003
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-014
target_type: RULE
target_id: BR-WS-STATE-003
status: mapped
-->

> 迁移债务：删除旧的无 DocRef/无 DiskState 新建会话分支；打开与关闭必须由
> SM-WS-04 的真实资源执行或补偿回执驱动。多文档逐项（REQ-ED-016）继续复用普通会话隔离语义。

## §3 跨模块转移契约（X-INTER-001：ED ↔ WS）

| 转移点 | 方向 | 契约 |
|---|---|---|
| 脏态产生 | SM-ED-01（Ready→Editing 的 edit/draftDiffApplied/sessionDiffApplied/pendingDiffPromoted）→ SM-WS-01（Idle→Dirty） | 用户编辑、DE 已有 LogicalState 的单卡或同批聚合预览、pending 后打开迁移都是 Dirty 驱动源；语义为"LogicalState 已偏离落盘内容" |
| 保存回执 | SM-WS-01（Saving→Saved）→ SM-ED-01（Editing→Ready 的 saved） | WS 落盘成功回执驱动 ED 清脏态；SaveError 不发 saved，ED 停留 Editing 可重试 |
| 关闭前批量保存 | SM-SYS-02 Resolving → SM-ED-01（edit 会话触发 save） | 应用关闭决策入口"保存全部脏文档"逐个驱动既有保存链，不新增 ED 事件（BR-SYS-STATE-004，2026-07-20 新增） |
| 启动异常退出清算 | SM-SYS-02（启动期）→ ed_session_snapshots dirty=1 行 | 先于任何会话恢复执行：清算发现的 dirty 持久化镜像一律删除，不作为可信 LogicalState 装载源；对应文档改走 SM-ED-01 正常 `open`（Closed→Loading→loadOk，从磁盘态重建），不使用 `restoreSessionSnapshots` 路径（BR-SYS-STATE-005，2026-07-20 新增） |

两侧对接一致性（事件语义、回执时机）登记 A-SYS-M-S-01 §5，在 G5 跨模块联调验证。
SM-WS-01 内部（Idle/Dirty/Saving/Saved/SaveError）归 A-WS-M-S-01，本文不重定义。

**恢复顺序边界（2026-07-20 新增，APP-CLOSE-DIFF-LIFECYCLE-01）**：`ed_session_snapshots` 持久化
镜像只在"上次为正常关闭"时可信；应用启动时异常退出清算（BR-SYS-STATE-005）必须先于本机会话恢复
执行。清算移除的 dirty 快照不得被当作可信 LogicalState 来源——对应文档改为经 `open` 事件从磁盘态
（DiskState）重新建立会话，不使用持久化快照。清算之后仍保留的快照（非 dirty，或本次运行期内正常
持久化的中间态）才进入既有恢复路径。

## §4 不独立建机的呈现链（豁免说明，APC §6）

- **decoration 呈现**（ED-DECOR）：呈现/复位由消费方信号驱动（DE diff 终态机、AG 回合机），自身
  无独立业务状态——其"状态"即消费方状态机的投影，按 APC §6 纯展示豁免不建机；约束由
  BR-ED-STATE-002 承载，挂载/复位正确性入 §5 测试。若后续暴露独立状态（如批量 diff 呈现编排），
  再补机并登记总图。
- **分页布局**：纯呈现层重排（BR-ED-DATA-003），无跨层副作用，按 APC §6 豁免。
- **inline-assist / autocomplete**：归属已确认为 AG（2026-06-11 人类确认，A-SYS-M-S-01 §3），
  其状态机由 AG G2 在 AG M-S 定义，ED 不建机、只提供 decoration 承载面。

## §5 测试与验收（强制转移测试矩阵，APC §6/§10）

- **SM-ED-01 正路**：Closed→open→Loading→loadOk→Ready→edit/draftDiffApplied/sessionDiffApplied→Editing→saved→Ready→close→Closed。
- **降级与错误路**：parseFail→ReadOnly（内容可见）→retry→Loading；loadFail→Closed 会话不建立；
  save/serializeFail 停留 Editing 可重试，且 Markdown 原 DiskState 不变（SM-WS-01 SaveError 侧验证归 WS）。
- **禁止转移**：Closed/Loading/ReadOnly→Editing 不可达；Editing 无 saved/confirmDiscard 时 close
  被拒（脏态不静默丢失）；sessionApplyFail 不改变 LogicalState。
- **跨模块转移（X-INTER-001，G5 联调）**：edit→SM-WS-01 Dirty 唯一驱动；Saved 回执→ED 清脏态；
  SaveError 不清脏态。
- **离线验收**（REQ-CORE-009）：断网下 open/edit/save（本地落盘）/close 全链可用。
- decoration 挂载/复位（非状态机项）：按 blockId 挂载、终态复位不残留，入 ED-DECOR 集成测试
  （BR-ED-STATE-002）。
- **已有/后打开会话预览**：基线匹配时 `sessionDiffApplied` 或 `pendingDiffPromoted` 更新 content/version/hash 并持久化、显示绿色增量且保留裁决；基线不匹配、锚失效、StyleOp 不支持时 `sessionApplyFail`，原 LogicalState 不变。
- **同批结构化多卡**：共享基线与互斥目标核对通过后只触发一次 `draftDiffApplied` 前向事务并产生逐卡
  previewApplied；任一卡无效则零 dispatch；拒绝一张只反向本卡目标，其他卡仍 pending 且可裁决。

## §6 总图登记（A-SYS-M-S-01）

| 核心链 | machine_id | 本文状态 | 总图回填动作 |
|---|---|---|---|
| 文档编辑会话 | SM-ED-01 | active | A-SYS-M-S-01 §3 新增「文档编辑会话｜ED｜SM-ED-01｜active」行 |
| ED↔WS 脏态/保存转移 | —（转移契约非机） | — | A-SYS-M-S-01 §5 X-INTER-001 登记本文 §3 两个转移点 |

> autocomplete / inline-assist 已确认归 AG（2026-06-11 人类确认），其 machine_id 由 AG G2 回填，本文不代填。

## §7 术语归属说明

状态机框架词不注册 TERM（ADU §3.2）。业务名词内容块（Block）/ 编辑器装饰层（EditorDecoration）
已在 A-ED-M-T-01 §0 注册（主责 ED）；diff/pending_diff 主责归 DE，本文引用不抢注。

## §8 技术债务与盲区（诚实声明）

- **多会话并发**：多文档同时打开（多 SM-ED-01 实例）的实例隔离与资源上限属实现期运行时调度，
  本文定单实例转移语义。
- **docx 会话的转换失败细分**：Loading 中 DOC 转换失败与解析失败目前同走 parseFail→ReadOnly；
  DOC G2 后如需细分（转换错误 vs 模型不兼容）回填本文。
- **confirmDiscard 的 UI 交互形态**（确认弹层/自动暂存草稿等）属实现期产品决策，本文只锁
  "脏态不得静默丢失"语义。
- **open session 的撤销呈现**：`sessionDiffApplied` 当前只保证 LogicalState 正确与脏态保存链；
  非激活文档撤销、GreenReview 补挂载和红删视觉由 DE/ED 后续 UI 债务承接。

## §9 后续动作

1. 本文落地后同步编辑 A-SYS-M-S-01：§3 新增文档编辑会话行（SM-ED-01），§5 登记 ED↔WS 转移点。
2. 随下一次 `npm run governance:generate` 复跑核对（STATE_MACHINE 块为门评追溯材料，规则由
   A-ED-M-T-01 注册）。
3. 待人类宣布 ED G2 闭合（AI 不自行宣布晋级，APC §2）；DOC G2 后细分 docx 转换失败路径（§8）。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-09-03 | v1.10 | AG-ADAPTIVE-RUNTIME-DESIGN-01：`draftDiffApplied` 承接同文档同批结构化多卡共享基线、聚合预检和单次前向事务；禁止逐卡推进整篇基准，逐卡反向只作用本卡目标。 |
| 2026-06-11 | v1.0 | 初版（ED G2 状态机件）：定义 SM-ED-01 文档编辑会话机（Closed/Loading/Ready/Editing/ReadOnly，含 APC §6 全部必备字段）；ED↔WS 脏态/保存 X-INTER-001 转移契约（edit→Dirty 唯一驱动、Saved 回执清脏态）；decoration/分页/未归属辅助链不建机的豁免说明；满足 X-SEQ-003 注册前置；测试矩阵（正路/降级/禁止转移/离线验收）；债务（多会话并发、docx 失败细分、confirmDiscard 形态延后） |
| 2026-06-11 | v1.1 | 人类决策同步：inline-assist/autocomplete 归属确认为 AG（§4/§6 表述更新，状态机由 AG G2 定义，ED 仅承载呈现） |
| 2026-06-12 | v1.2 | Q2 缺口补遗：SM-ED-01 新增 Conflicted 态与 externalChange/saveConflict/conflictKeepMine/conflictReload 事件；forbidden 增「有脏态不得静默重载」「冲突不得自动消解」；acceptance 同步（承载 BR-ED-STATE-003，对接 SM-WS-01 冲突路径与 BR-WS-OBS-001 事件） |
| 2026-07-08 | v1.3 | 增加 draftDiffApplied 事件：DE pending diff 命中当前编辑器块时由 ED 受控接口替换 after、进入 Editing 并驱动保存链；Saved 后清除已 accepted 的 GreenReview |
| 2026-07-18 | v1.4 | 增加 sessionDiffApplied/sessionApplyFail：已打开非激活文档 diff 基线匹配后修改 session LogicalState、递增版本/哈希并持久化，失败不得半应用；脏态转移接入 SM-WS-01 |
| 2026-07-18 | v1.5 | ED-MARKDOWN-RICH-ROUNDTRIP-01：SM-ED-01 复用既有状态，增加 BR-ED-DATA-006/BR-ED-VERIFY-001、serializeFail 事件、Markdown 解析/源码保护 Guard 与禁止有损保存转移；不新建规则链或状态机 |
| 2026-07-20 | v1.6 | 承接 APP-CLOSE-DIFF-LIFECYCLE-01：§3 新增「关闭前批量保存」「启动异常退出清算」两条 SYS→ED 跨模块转移点（均双侧就绪）；补充恢复顺序边界——异常退出清算先于会话恢复，dirty 持久化镜像不可信时改走磁盘态重建 |
| 2026-08-13 | v1.7 | DIFF-STATE-COMPASS-01：SM-ED-01 增 `pendingDiffPromoted` 并承接 BR-CORE-STATE-001/003；已打开与 pending 后打开迁移的 diff 都在预览后保持 pending、绿色增量和用户裁决入口。 |
| 2026-08-14 | v1.8 | AG-1.0-D2-DE-ED-WS-STATEMACHINE-01：新增 §2.1 未保存新建文档与转换目标会话承接（D-NEW-001~008 / REQ-ED-010/014）——预览开标签、接受只裁决、拒绝丢弃、保存才建文件、保存失败保留；补 REQ-ED-010/014 的 REQ_MAP（BR-CORE-STATE-001/003、BR-SYS-STATE-008）；登记未保存身份迁移债务。 |
| 2026-08-21 | v1.9 | WorkspaceOperation 基线同步：§2.1 替换为 CreateDocumentBundle 真实磁盘目标会话；确认后先创建、再开标签并投影初始内容 Diff，安全拒绝由 WS 补偿，保存不再承担创建语义。 |
