---
文档编号：SYS-M-S-01
文档状态：A
负责模块：SYS
文档职责：Binder 核心链状态机总图与注册框架技术设计——状态机注册清单、注册前置约束、跨模块状态转移契约框架；承接 REQ-SYS-003
上游约束：A-SYS-M-D-01_系统底座功能需求描述.md（REQ-SYS-003、SY-04）、A-CORE-C-P-01_APC.md（§6 状态机驱动强制阈值与必备字段）、A-SYS-I-P-02_IPC协议.md（BR-SYS-STATE-002 流式终态）、A-SYS-I-P-03_云接口契约.md（SM-SYS-01）、ADP.md（SC-06/07）、ADU.md（§3.1 CONSTRAINT 格式）
直接承接：各 owner 模块在其 M-S 定义本模块状态机并向本总图登记 machine_id；当前跨模块转移点以本文框架对接，Agent 1.0 不含历史构建移交/回落
使用边界：本文是状态机总图框架与跨模块转移契约的 SSOT；**不替各 owner 模块定义其状态机内部**（states/transitions/guards 等归各模块 M-S）；不重定义已存在的 SM-SYS-01（归 A-SYS-I-P-03）；当前已登记 16 台已定义状态机，后续仅在新 REQUIREMENT/DESIGN Scope 定义新状态化链路后由 owner 回填
变更要求：核心链强制阈值变更须先经 APC §6 授权；新增状态机由 owner 模块 M-S 定义后登记本总图；跨模块转移契约变更须两侧 owner 共同复核
治理成熟度：draft
---

# Binder 状态机总图与注册框架（SYS · L2 技术设计）

## §1 文档定位

本文是 SYS 模块 L2 技术设计层文档，承接 REQ-SYS-003（状态机总图与注册），承担一项 SSOT 职责：
**核心链状态机的总图框架与注册入口**，以及**跨模块状态转移契约框架**。

```
REQ-SYS-003（L1）→ 本文（总图框架 + 注册前置/跨模块转移约束）→ REQ_MAP → CONSTRAINT → 代码(@GOV)
各 owner 模块 M-S 定义状态机内部 ──登记 machine_id──→ 本总图
本文 ←引用← A-SYS-I-P-03（SM-SYS-01）、A-SYS-I-P-02（BR-SYS-STATE-002 流式终态）
```

边界（与 A-SYS-M-D-01 SY-04 一致）：

- 本文提供总图**框架与注册前置约束**，不替各 owner 模块定义其状态机内部（states/events/
  transitions/guards/actions/error_states/recovery_paths 等归各模块 M-S，APC §6）。
- **不重定义 SM-SYS-01**（账号会话与订阅门禁状态机已在 A-SYS-I-P-03 定义），本文仅登记并对齐。
- 总图是非穷举注册表；当前 §3/§3.1 已登记的状态机均有 owner 设计，后续只在人类授权的新 REQUIREMENT/DESIGN Scope 暴露新状态化链路时追加。

## §2 状态机驱动总则与本文职责

APC §6 强制阈值（口径 b）：**有状态 / 跨层 / 多步的链路必须建状态机**，纯函数/纯展示豁免。
每个状态机必备字段（APC §6）：machine_id / owner_module / scope / context / initial_state /
states / terminal_states / events / transitions / guards / actions / error_states /
recovery_paths / forbidden_transitions / acceptance_criteria / test_matrix。

本文职责：
1. 维护状态机**注册清单**（§3），使每条核心链状态机可追溯 owner。
2. 立**注册前置约束**（§4 X-SEQ-003）：未注册/未建状态机的状态化核心链不得进入实现。
3. 立**跨模块转移契约框架**（§5 X-INTER-001）：跨模块状态转移须双侧对接一致。

## §3 状态机总图与注册清单

APC §6 列举的核心链（非穷举）及其状态机登记状态。`active` = 已在 owner 设计文档定义，
只表示“设计资产存在”，不等于任一 RuntimeProfile 已启用、实现已完成或 G5 已通过。`planned` = 待 owner 模块在其 M-S 定义后回填 machine_id。

| 核心链 | owner_module | machine_id | status | 定义文档 |
|---|---|---|---|---|
| 账号会话与订阅门禁 | SYS（ACCT/ENT 消费） | SM-SYS-01 | active | A-SYS-I-P-03 §6 |
| diff 终态 | DE | SM-DE-01 | active | A-DE-M-S-01 §2 |
| 代理回合 | AG | SM-AG-01 | active | A-AG-M-S-01 §2 |
| tool-call | AG | SM-AG-02 | active | A-AG-M-S-01 §3 |
| 文档编辑会话 | ED | SM-ED-01 | active | A-ED-M-S-01 §2 |
| save 保存 | WS | SM-WS-01 | active | A-WS-M-S-01 §2 |
| diff 落盘（WS 侧） | WS | SM-WS-02 | active | A-WS-M-S-01 §3 |
| 时间轴还原 | WS | SM-WS-03 | active | A-WS-M-S-01 §4 |
| docx 开 / 存 | DOC | SM-DOC-01 | active | A-DOC-M-S-01 §2 |
| CBT 构建 | CBT | SM-CBT-01 | active | A-CBT-M-S-01 §2 |
| 记忆写入 | AST | SM-AST-01 | active | A-AST-M-S-01 §2 |
| 工作流执行 | TPL | SM-TPL-01 | active | A-TPL-M-S-01 §2（owner G2 暴露新增） |
| autocomplete | AG（2026-06-11 人类确认） | SM-AG-03 | active | A-AG-M-S-01 §4（与 inline-assist 同机承载） |
| inline-assist | AG（2026-06-11 人类确认） | SM-AG-03 | active | A-AG-M-S-01 §4（与 autocomplete 同机承载） |
| 应用关闭 | SYS | SM-SYS-02 | active | A-SYS-M-S-02 §2 |

> machine_id 由各 owner 模块在其 M-S 自行分配（格式 SM-[MODULE]-[SEQ]，ADU §3），本文不预占，
> 避免跨模块 ID 归属漂移。§3 当前登记 14 个唯一 machine_id（autocomplete/inline-assist 共用 SM-AG-03），
> §3.1 另登记 SM-AG-04 与 SM-WS-04，合计 16 台已定义状态机。CBT/AST/TPL 的 active 只表示设计资产存在，不把其自动接入 Agent 1.0。

### 3.1 Agent 1.0 运行时与任务链状态机归属（D1 登记）

承接冻结基线（A-CORE-X-D-02 §6）与 REQ-SYS-010/011/012：D1 只登记**归属与契约**，状态机内部定义由
对应 D2/D3 在 owner 模块 M-S 完成并向本总图登记 machine_id（X-SEQ-003）。

| 状态化链路 | 归属判定 | owner_module | machine_id | 登记状态 | 说明 |
|---|---|---|---|---|---|
| 代理回合（AgentTurn） | AG（既有 SM-AG-01，D3 扩展澄清非终态等 R5-A 语义） | AG | SM-AG-01 | active（D3 调整） | 澄清等待、全活动态取消、首终态锁存、迟到事件无副作用（BR-SYS-STATE-007） |
| 工具调用循环 | AG | AG | SM-AG-02 | active | 既有；按 CapabilityGrant 收窄（BR-SYS-STATE-007） |
| Proposal 链（连续 Proposal 接力/失效） | DE（既有 SM-DE-01，D2 扩展接力语义） | DE | SM-DE-01 | active（D2 调整） | 以最新 LogicalState 接力、前卡基准推进失效、不形成补丁栈（BR-SYS-STATE-008） |
| 受限任务实例（TaskRun）与步骤 | AG 编排（同一用户目标内组织步骤/回合） | AG | SM-AG-04 | active（D3 建机） | 非后台队列、不自动重试/续跑、不越确认点；真实工作状态由回合/步骤/工具事件驱动（BR-SYS-STATE-008） |
| 工作区资源操作（WorkspaceOperation） | WS 承载计划、重验、磁盘执行、补偿与恢复；AG/TaskRun 只负责意图、DecisionRequest 与调用 | WS | SM-WS-04 | active（设计已登记，待实现） | 前置裁决；确认后直接 DiskState；无资源 DiffCard；fail-fast 逆序补偿；重启 Interrupted |
| 代理会话（AgentSession）生命周期 | AG（展示/持久化/恢复隔离键） | AG | —（已判定：无独立状态机） | 已判定（D3） | 判定为按 X-FORBID-007 投影 + history 双作用域（workspace+session）持久化/恢复，不建独立状态机 |
| 外部内容引用（ExternalContentRef）暂存/清除 | REF（读取路径）+ AG（当前任务消费） | REF/AG | —（已判定：无独立状态机） | 已判定（D3） | 一次性 data-only、当前任务范围、不自动持久化；payload 随发送即弃（BR-SYS-DATA-007，IMP-EXTERNAL-CONTENT-01 落地） |
| 显式项目记忆引用（MemorySelectionRef） | AST（用户管理）+ REF（选择注入） | AST/REF | —（已判定：无独立状态机） | 已判定（D3） | 仅显式选择进入当前回合、不自动写入/检索/注入（BR-SYS-DATA-007，IMP-MEMORY-SELECTION-01 落地） |

> 归属原则：SYS 总图只登记契约与跨模块转移点，不定义内部；TaskRun/回合/会话等运行时状态机归 AG，
> Proposal 链归 DE，保存/落盘归 WS，LogicalState 投影归 ED，引用/记忆消费归 REF/AST。

## §4 状态机注册前置约束（CONSTRAINT）

<!-- CONSTRAINT
constraint_id: X-SEQ-003
摘要: 状态化核心链（APC §6 强制阈值）必须先建状态机并在总图登记 machine_id 方可进入实现，未注册者阻断进入代码
涉及模块: SYS, WS, ED, DE, AG, CBT, DOC, AST
链路影响: CLOUD-AUTH, CLOUD-GATE, CLOUD-INVOKE
-->

> X-SEQ-003 与 ADP SC-06（PHASE_GATE_BLOCKED）/ SC-07（MAPPING_CHAIN_BROKEN）一致：越过状态机直接
> 实现状态化核心链触发阻断（APC §9、§6）。当前登记项的设计前置已具备；任何新状态化链路在进入实现前仍必须先由 owner M-S 定义并登记本总图。

## §5 跨模块状态转移契约框架（CONSTRAINT）

部分核心链的状态转移跨模块发生，转移两侧的 owner 状态机必须在对接点保持一致，不得各自假设。
代表性跨模块转移点（随各模块 G2 回填）：

- **diff 三分流 → LogicalState 或受控落盘**：DE 的 diff 终态机（**SM-DE-01**，A-DE-M-S-01 §2）
  必须按 `BR-CORE-STATE-001~003` 与目标是否已有 LogicalState 分流。已有 LogicalState 时，pending
  立即经 SM-ED-01 受控预览并进入 Editing/Dirty，DiffCard 保持 pending；用户接受只推进卡片生命周期，
  后续保存才进入 SM-WS-01 写入。没有 LogicalState 时，pending 只挂载在 DiffCard；用户接受才触发
  SM-WS-02 原子写入（REQ-CORE-002）。背景 pending 期间若用户打开目标文档，SM-ED-01 以同一 payload
  建立带 GreenReview 的 LogicalState，取消其后台写盘资格而仍保持 pending，之后走已有 LogicalState
  分支。rejected/expired 不写文件；草稿分支经 ED 回滚/复位，后台分支经 SM-WS-02→Discarded。
  **三侧对接须以这三条规则一致性为准**，在 G5 联调验证。
- **P2 Build/CBT 隔离**：SM-CBT-01 可作为路线资产保留，但 `agent-edit-1.0` 的 SM-AG-01 不包含
  takeover、HandedOver 或 fallbackIn，也没有到 CBT 的 active 跨模块转移。未来 Profile 启用时须重新完成
  REQUIREMENT/DESIGN、双侧事件和 X-INTER-001 登记，不得复用历史“已就绪”结论。
- **Agent 1.0 项目记忆隔离**：SM-AG-01 终态不触发 `ast.capture` 或其他自动记忆副作用；仅用户在当前
  workspace 明确选择的既有项目记忆可经 REF 作为 data-only 引用进入当轮。SM-AST-01 的路线资产存在不构成 AG 激活。
- **P2 TPL 隔离**：`agent-edit-1.0` 不委托 SM-TPL-01，也不接受模板工作流自动注入；未来启用须新开需求与设计。
- **模型调用前门禁**：业务模块发起模型调用前必经 SM-SYS-01 的 GatePassed（X-SEQ-001，A-SYS-I-P-03）。
- **ED 脏态 ↔ WS 保存**：SM-ED-01 的 edit/draftDiffApplied/sessionDiffApplied 事件是 SM-WS-01 Idle→Dirty 的业务驱动源；SM-WS-01
  Saved 回执驱动 SM-ED-01 Editing→Ready 清脏态，SaveError 不清脏态（A-ED-M-S-01 §3，双侧已就绪）。
- **docx 装载与落盘**：SM-DOC-01 convertOk 对接 SM-ED-01 装载（convertFail 对接 loadFail/parseFail，
  失败语义两侧一致）；SM-DOC-01 SaveConverting 经 SM-WS-01 原子落盘（Saved/SaveError 回执驱动
  saveConvertOk/Fail），原文件不破坏由 WS 保障（A-DOC-M-S-01 §3，双侧已就绪）。
- **AG 回合门禁与产出**：SM-AG-01 AwaitingGate 提交 SM-SYS-01（gatePassed/gateDenied 驱动，
  Denied 不可绕过）；SM-AG-01 HandingOff 将修改建议（ChangeProposal）交 SM-DE-01 Generating
  （genOk/genFail 回执 handoffOk/Fail，**双侧已就绪**，A-DE-M-S-01 §3）；SM-AG-03 accept 驱动
  SM-ED-01 edit（采纳即用户编辑，双侧已就绪）（A-AG-M-S-01 §6）。
  AG 发起回合前必须携带 OpenDocSnapshot 构建 AgentReadContext；已有 LogicalState 的 diff 经 DE 分流到
  SM-ED-01 预览，未打开文档保持背景临时挂载，只有用户接受才可经 SM-WS-02 落盘；背景 pending 被打开时
  必须先迁移到 SM-ED-01。至此「代理回合生成 diff→三分流预览/挂载→用户裁决→保存或背景落盘」核心交互
  AG→DE→ED/WS 状态机链全链建机完毕。
- **应用关闭协调**（2026-07-20 新增）：SM-SYS-02（A-SYS-M-S-02 §2）在关闭请求（窗口关闭/Cmd+Q/
  Dock 退出/系统关机）建立屏障后对 Streaming 中的代理回合发 `cancel`（驱动 SM-AG-01）；
  `AwaitingDecision` 态的批量决策复用既有 `de.accept_diff`/`de.reject_diff` 驱动 SM-DE-01，复用
  既有保存链驱动 SM-ED-01/SM-WS-01 清脏态；`Verifying` 态只读复核 pending_diffs/ed_session_snapshots
  当前真值。**四个转移点双侧已就绪**（AG/DE/ED/WS 侧均复用既有事件，无需对侧改动，A-SYS-M-S-02 §3）。
- **TaskRun ↔ AgentTurn ↔ Proposal 链**（2026-08-14 D1 登记，R5-B）：同一用户目标的 TaskRun 经
  SM-AG-01（AgentTurn）组织步骤；进入写入的步骤经 DE 的 SM-DE-01 生成 Proposal 并沿 BR-CORE-STATE-001~003
  三分流交给 SM-ED-01（已有 LogicalState 预览）/ SM-WS-02（无 LogicalState 接受落盘）；连续 Proposal
  必须以最新 LogicalState 为基准接力，前卡基准推进失效且不可再裁决（BR-SYS-STATE-008）。TaskRun 状态机
  （AG）D3 建机后登记本总图；取消只停止未开始步骤、保留已成功预览内容，不形成任务级回滚。
- **TaskRun ↔ DecisionRequest ↔ WorkspaceOperation 链**（2026-08-21 登记）：资源步骤由 AG 生成
  WorkspaceOperationPlan，并以 DecisionRequest 进入 SM-AG-04 WaitingConfirm；确认后 TaskRun 立即调用
  SM-WS-04 Revalidating/Executing，拒绝零磁盘副作用。WS 完成或补偿终态回传 TaskRun 后才可推进依赖步骤。
  资源卡不进入 SM-DE-01；受影响内容 Diff 仅在资源执行成功后因基线推进失效。CreateDocumentBundle 在
  资源创建成功后再进入 SM-DE-01 投影初始内容 Diff。重启使两侧未终态都进入 Interrupted，旧确认不得重放。
- **AgentTurn 澄清等待（非终态）**（2026-08-14 D1 登记，R5-A）：澄清是等待用户输入的非终态，不是
  SM-AG-01 的 Completed；取消可在任一活动阶段进入终态，首终态后迟到事件/工具不得产生文档、卡片或落盘
  副作用（BR-SYS-STATE-007）。D3 调整 SM-AG-01 时须与 IPC 流式终态契约（BR-SYS-STATE-002）对齐。
- **外部内容 / 显式记忆引用边界**（2026-08-14 D1 登记，R5-C）：ExternalContentRef 只在当前
  AgentTurn/TaskRun 当前任务内经 REF 以 data-only 注入 AG（BR-SYS-DATA-007），任务结束/重启/跨会话
  不恢复；MemorySelectionRef 仅用户显式选择进入回合。两者不得触发 AST 自动写入（SM-AST-01 仅用户
  管理路径），D3 定义暂存/清除与来源令牌后按 X-SEQ-003 判定是否建机登记。

<!-- CONSTRAINT
constraint_id: X-INTER-001
摘要: 跨模块状态转移必须在总图登记转移契约，转移两侧 owner 状态机对接点须一致，不得各自假设状态/事件
涉及模块: SYS, WS, ED, DE, AG, CBT
链路影响: CLOUD-GATE, CLOUD-INVOKE
-->

## §6 流式终态与 IPC 对齐

经 IPC 流式回传的状态机终态须与 A-SYS-I-P-02 的 BR-SYS-STATE-002（流式逻辑契约：完成/中断/错误
三类终态）一致。已落地的 SM-SYS-01：`Invoking → InvokeCompleted`（流式完成）/ `Invoking →
InvokeError`（流式错误，统一错误形状）即对齐本契约；后续各 owner 的流式状态机登记时须同样对齐。

**状态机投影广播**：各 SM-* 转移落库后经 `sm:{machine_id}:state_changed` 广播权威态供前端投影，
契约与统一信封（SmStatePayload）归 A-SYS-I-P-02 §5.1 / BR-SYS-STATE-003（承接 X-FORBID-007）；
本总图登记的 owner 状态机实现广播时 instance_id 用各自实例键（diff_id / turn_id / rel_path / entry_id 等）。

## §7 术语归属说明（为何本文不注册 TERM）

machine_id、状态机、转移、终态等为状态机框架技术词，按 ADU §3.2 不注册 TERM。各核心链涉及的业务
名词（diff、记忆、构建等）TERM 主责归对应 owner 模块（DE/AST/CBT），在其 L2 注册，本文不抢注。

## §8 需求到规则映射（REQ_MAP）

<!-- REQ_MAP
req_id: REQ-SYS-003
target_type: CONSTRAINT
target_id: X-SEQ-003
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-003
target_type: CONSTRAINT
target_id: X-INTER-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-010
target_type: RULE
target_id: BR-SYS-STATE-007
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-010
target_type: CONSTRAINT
target_id: X-SEQ-003
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-011
target_type: RULE
target_id: BR-SYS-STATE-008
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-011
target_type: CONSTRAINT
target_id: X-SEQ-003
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-012
target_type: RULE
target_id: BR-SYS-DATA-007
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-012
target_type: CONSTRAINT
target_id: X-SEQ-003
status: mapped
-->

## §9 测试与验收

- **注册可追溯**（REQ-SYS-003 验收）：总图清单中每条核心链状态机可追溯 owner_module；planned 项
  随 owner G2 回填 machine_id 后转 active。
- **注册前置阻断**（X-SEQ-003）：状态化核心链在无状态机定义时，G4 实现验证触发 SC-06/07 阻断
  （集成阶段验证）。
- **跨模块转移一致**（X-INTER-001）：当前 Active 转移点的两侧 owner 状态机必须对接一致；diff 必须覆盖已有 LogicalState 预览、无 LogicalState 挂载接受落盘、pending 打开迁移三条分流，在 G5 跨模块联调验证（APC §5.1）。Agent 1.0 不含构建移交/回落；未来新 Profile 须另行登记和验证。
- **流式终态对齐**：经 IPC 流式的状态机终态与 BR-SYS-STATE-002 一致（SM-SYS-01 已对齐）。

## §10 技术债务与盲区（诚实声明）

- **总图清单已闭合**：§3/§3.1 当前合计 16 台已定义状态机，无 planned 登记项；清单仍为非穷举框架，新状态化链路按 X-SEQ-003 建机登记追加。
- **autocomplete / inline-assist 归属已定（AG）**：2026-06-11 人类确认，SM-AG-03 同机承载已登记。
- 跨模块转移点清单（§5）为代表性、非穷举，随各模块 G2 暴露补全。
- **Agent 1.0 运行时/任务链状态机归属已回填（D1 债务关闭，2026-08-14）**：TaskRun 由 D3 建机
  SM-AG-04（active，§3.1）；AgentSession/ExternalContentRef/MemorySelectionRef 经 D3 判定**无独立
  状态机**（投影契约 + history 双作用域 / payload 一次性 data-only），§3.1 已回填；SM-AG-01 澄清非终态
  与 SM-DE-01 连续 Proposal 接力已由 IMP-CLARIFICATION-01 / IMP-CONSECUTIVE-PROPOSAL-01 落地
  （BR-SYS-STATE-007/008）。

## §11 维护入口

1. 本文现有 CONSTRAINT / REQ_MAP 已由 ADUS 生成器注册；任何更改后仅通过 `governance:generate` 刷新，不人工维护索引。
2. D2 三分流/连续 Proposal 与 D3 AgentTurn/TaskRun/引用记忆归属已回填§3.1/§5，不再作“待建机”计划项。
3. 未来只在已授权的 REQUIREMENT/DESIGN Scope 新增状态化链路时，由 owner M-S 先定义内部，再在本文登记 machine_id 和跨模块转移。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-10 | v1.0 | 初版（SYS G2 末件）：状态机总图框架与注册清单（登记 APC §6 核心链，SM-SYS-01 active 其余 planned）；注册前置约束 X-SEQ-003（对齐 ADP SC-06/07）、跨模块转移契约框架 X-INTER-001；流式终态与 BR-SYS-STATE-002 对齐；REQ_MAP（REQ-SYS-003）；术语归属说明；债务盲区（SY-04 总图未闭合、autocomplete/inline-assist 归属待定） |
| 2026-06-10 | v1.1 | WS G2 回填（owner 模块登记，§11.3）：§3 总图清单 save 行 planned→active 回填 machine_id=SM-WS-01，新增「diff 落盘（WS 侧）SM-WS-02」「时间轴还原 SM-WS-03」两行（A-WS-M-S-01 定义）；§5 X-INTER-001「diff accepted→落盘」转移点登记 WS 侧机 SM-WS-02（DE 侧待 DE G2）；§10 SY-04 债务由「未闭合」更新为「部分闭合」（SYS+WS 已 active，余模块 planned） |
| 2026-06-11 | v1.2 | ED G2 回填（owner 模块登记，§11.3）：§3 新增「文档编辑会话｜ED｜SM-ED-01｜active」行（A-ED-M-S-01 定义）；§5 X-INTER-001 登记「ED 脏态↔WS 保存」转移点（edit→Dirty 唯一驱动、Saved 回执清脏态，双侧已就绪）；autocomplete/inline-assist 归属仍待确认，未回填 |
| 2026-06-11 | v1.3 | 人类决策落地：autocomplete / inline-assist 归属确认为 AG（§3 两行 owner 改 AG、待 AG G2 定义状态机；§10 对应债务关闭） |
| 2026-06-11 | v1.4 | DOC G2 回填（owner 模块登记，§11.3）：§3 docx 开/存行 planned→active 回填 machine_id=SM-DOC-01（A-DOC-M-S-01 定义）；§5 X-INTER-001 登记「docx 装载与落盘」转移点（DOC↔ED、DOC↔WS，双侧已就绪） |
| 2026-06-11 | v1.5 | AG G2 回填（owner 模块登记，§11.3）：§3 四行 planned→active——chat-agent 回合=SM-AG-01、tool-call=SM-AG-02、autocomplete/inline-assist=SM-AG-03（同机承载，A-AG-M-S-01 定义）；§5 X-INTER-001 登记「AG 回合门禁与产出」转移点（AG↔SM-SYS-01 门禁双侧就绪、AG 产出→DE 单侧待 DE G2、SM-AG-03 采纳→SM-ED-01 双侧就绪）；至此总图仅余 CBT 构建 / 记忆写入（AST）/ diff 终态（DE）三行 planned |
| 2026-06-11 | v1.6 | DE G2 回填（owner 模块登记，§11.3）：§3 diff 终态行 planned→active 回填 machine_id=SM-DE-01（A-DE-M-S-01 定义）；§5「diff accepted→落盘」「AG 回合门禁与产出」两条转移点更新为双侧就绪——「对话生成 diff→人类裁决→落盘」核心交互 AG→DE→WS（→ED）状态机链全链建机完毕；总图仅余 CBT 构建 / 记忆写入（AST）两行 planned |
| 2026-06-11 | v1.7 | P4 G2 回填（owner 模块登记，§11.3）：§3 CBT 构建行=SM-CBT-01、记忆写入行=SM-AST-01 转 active，新增「工作流执行｜TPL｜SM-TPL-01」行（owner G2 暴露登记）；§5「Chat↔Build 接管」更新为双侧就绪（takeover/fallback+回执），新增「TPL 步骤委托」转移点；**总图全清单无 planned 行，已登记 12 台状态机全部 active**（SY-04 闭合） |
| 2026-07-04 | v1.8 | 构建移交判据换轴（人类裁决，同步 A-AG-M-S-01 v2.0 / A-CBT-M-S-01 v2.0）：§5 转移点更名「构建移交/回落」，触发=显式触发即确认或项目级构建意图提议经确认（多文件意图废止），蓝图随移交、原窗口呈现、执行期禁新回合 |
| 2026-07-18 | v1.9 | 修正 diff 跨模块转移：accepted 不再等同落盘；current/open session 进入 SM-ED-01 LogicalState/Dirty，background 仅未打开文档进入 SM-WS-02；AG 回合前必须用 OpenDocSnapshot 建 AgentReadContext |
| 2026-07-20 | v1.10 | 新增「应用关闭｜SYS｜SM-SYS-02」行（A-SYS-M-S-02 定义，APP-CLOSE-DIFF-LIFECYCLE-01）：承接三轮人类确认——正常关闭前 pending diff/未保存文档必须清零，异常退出后启动清算不复活 pending/dirty 为可信态；§5 新增四条跨模块转移契约（关闭屏障→AG cancel、批量裁决→DE、保存驱动→ED/WS、复核查询），均双侧就绪 |
| 2026-08-13 | v1.11 | DIFF-STATE-COMPASS-01：总图将 diff 跨模块转移收敛为 BR-CORE-STATE-001~003 三分流——已有 LogicalState 直接预览且卡片 pending、无 LogicalState 临时挂载后接受才经 SM-WS-02 落盘、pending 打开目标文档迁移至 SM-ED-01；移除 “open_session 自动 accepted” 语义。 |
| 2026-08-14 | v1.12 | AG-1.0-D1-SYS-SHARED-CONTRACT-01：新增 §3.1 Agent 1.0 运行时与任务链状态机归属（AgentTurn=SM-AG-01、Proposal 链=SM-DE-01，TaskRun/步骤、AgentSession、ExternalContentRef、MemorySelectionRef 为 planned 待 D3）；§5 新增 TaskRun↔AgentTurn↔Proposal 链、澄清等待非终态、外部内容/显式记忆引用三个跨模块转移契约；REQ_MAP 补齐 REQ-SYS-010/011/012；§10 登记状态机待建债务。 |
| 2026-08-14 | v1.13 | AG-1.0-DOC-CLOSURE-DCLOSURE-01：§3.1 回填——TaskRun 行 planned→active（machine_id=SM-AG-04，D3 建机）；AgentSession/ExternalContentRef/MemorySelectionRef 三行判定为无独立状态机（投影契约+history 双作用域 / payload 一次性 data-only）并注明落地 Scope；§10 D1 债务状态更新为已回填。 |
| 2026-08-21 | v1.14 | 工作区资源操作基线重构：总图登记 SM-WS-04；新增 TaskRun→DecisionRequest→WorkspaceOperation 跨模块转移，明确资源操作不进入 SM-DE-01、CreateDocumentBundle 仅在资源创建后衔接初始内容 Diff、重启双方 Interrupted。 |
| 2026-08-31 | v1.15 | AG-DOC-CONSOLIDATION-01：Agent 1.0 跨模块边界收敛为无 AG↔CBT Build 接管/回落、无自动 AST 记忆转移、无自动 TPL 委托；重算总图为 16 台已定义状态机，清除“仅 SM-SYS-01 active”、“12 台”及 D2/D3 待回填等过期正文。 |
