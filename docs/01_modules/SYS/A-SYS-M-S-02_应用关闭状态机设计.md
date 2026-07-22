---
文档编号：SYS-M-S-02
文档状态：A
负责模块：SYS
文档职责：SYS 应用关闭模块级状态机设计——应用关闭协调机（SM-SYS-02），含 APC §6 必备字段；回填
A-SYS-M-S-01 总图应用关闭行（新增 active）；闭合与 SM-DE-01/SM-ED-01/SM-AG-01/SM-WS-01/SM-WS-02
的跨模块转移点
上游约束：A-SYS-M-T-02_应用关闭技术设计.md（APP-CLOSE 链路、BR-SYS-STATE-004/005/006、
BR-SYS-DATA-005 规则）、A-CORE-C-P-01_APC.md（§6）、A-SYS-M-S-01_状态机总图.md（X-SEQ-003/
X-INTER-001）、A-DE-M-S-01_DiffEdit状态机设计.md（SM-DE-01 终态互斥、批量裁决入口）、
A-ED-M-S-01_编辑器状态机设计.md（SM-ED-01 脏态/保存）、A-AG-M-S-01_代理状态机设计.md（SM-AG-01
cancel 事件）、A-WS-M-S-01_工作台状态机设计.md（SM-WS-01 save、SM-WS-02 diff 落盘）、ADU.md
（§3.1）
直接承接：SM-SYS-02 经 rules 字段承载 A-SYS-M-T-02 规则；machine_id 登记总图；批量裁决/保存
指令驱动 SM-DE-01/SM-ED-01；回合取消驱动 SM-AG-01
使用边界：本文定义 SM-SYS-02 内部；不重定义 SM-DE-01/SM-ED-01/SM-AG-01/SM-WS-01/SM-WS-02 内部；
决策入口具体 UI 视觉形态不在本文（延实现期，锁定行为约束见 A-SYS-M-T-02 §3）
变更要求：transitions 变更须同步 A-SYS-M-T-02 对应规则并重跑测试矩阵；与 DE/ED/AG/WS 的转移点
变更须对侧共同复核；machine_id 一经登记不可更改
治理成熟度：draft
---

# Binder 应用关闭状态机设计（SYS · L2 技术设计）

## §1 文档定位

```
A-SYS-M-T-02（规则 SSOT：APP-CLOSE）→ 本文 SM-SYS-02
本文 machine_id ──登记──→ A-SYS-M-S-01 §3（应用关闭行 新增→active）
协调对象：SM-DE-01（批量裁决）、SM-ED-01（脏态/保存）、SM-AG-01（回合取消）、
          SM-WS-01/SM-WS-02（保存/落盘复核）
```

满足 X-SEQ-003：应用关闭编排是有状态/跨层/多步的应用级核心链（清点→决策→执行→复核→退出），在
此建机后方可进入实现。SM-SYS-02 不重新定义被协调状态机的内部转移，只在既定入口调用其既有事件
（`de.accept_diff`/`de.reject_diff`/save/`SM-AG-01 cancel`），以复核结果驱动自身转移。

## §2 SM-SYS-02 · 应用关闭协调机

<!-- STATE_MACHINE
machine_id: SM-SYS-02
owner_module: SYS
status: active
summary: 单次应用退出请求（窗口关闭/Cmd+Q/Dock 退出/系统关机）到实际退出之间的协调；驱动 DE/ED
  批量裁决与保存达到 pending=0 且 dirty=0 后方可关闭；提供人类确认的强制退出兜底
source_type: design
chain: APP-CLOSE
rules: BR-SYS-STATE-004, BR-SYS-STATE-005, BR-SYS-STATE-006, BR-SYS-DATA-005
initial_state: Idle
terminal_states: Closed
-->

- **machine_id**：SM-SYS-02　**owner_module**：SYS
- **scope**：单个应用进程内、单一活动 workspace 的一次关闭请求到实际退出（或取消回到可用态）的
  完整生命周期
- **context**：pendingDiffCount、awaitingSaveCount、dirtyDocuments（含非激活文档）、savingDocumentCount、
  activeAgentTurnIds、用户在决策入口的选择（acceptAllAndSave / rejectAllAndDiscard /
  rejectAllAndSave / forceQuit / cancel）、执行进度与失败详情
- **states**：Idle、BarrierUp、Inspecting、AwaitingDecision、Resolving、Verifying、Closing
- **events**：closeRequested、barrierReady、inspectClean、inspectDirty、userAcceptAllAndSave、
  userRejectAllAndDiscard、userRejectAllAndSave、userCancel、userForceQuit、resolveOk、
  resolveFail、verifyOk、verifyRetry
- **transitions（主表）**：

| from | event | guard | to | action |
|---|---|---|---|---|
| Idle | closeRequested | — | BarrierUp | 拒绝新 `AgentTurn.send`/`de.generate_diff`；对 Streaming 中回合发 `cancel`（驱动 SM-AG-01） |
| BarrierUp | barrierReady | — | Inspecting | 同步 flush 编辑器态后，查询 `sys.inspect_close`：当前 pending 数、awaiting_save 数、dirty 文档列表（含非激活）、saving 数 |
| Inspecting | inspectClean | pending=0 且 awaiting_save=0 且 dirty=0 且 saving=0 | Closing | 直接放行，不弹窗 |
| Inspecting | inspectDirty | pending>0 或 awaiting_save>0 或 dirty>0 | AwaitingDecision | 呈现决策入口，展示统计与按文档聚合的详情 |
| AwaitingDecision | userCancel | — | Idle | 撤屏障，恢复正常可编辑/可发起回合 |
| AwaitingDecision | userAcceptAllAndSave | — | Resolving | 批量 `de.accept_diff`（驱动 SM-DE-01 pending→accepted）+ 保存全部脏文档（驱动 SM-ED-01 Editing→saved） |
| AwaitingDecision | userRejectAllAndDiscard | — | Resolving | 批量 `de.reject_diff`（驱动 SM-DE-01 pending→rejected）+ 放弃全部未保存内容（不驱动 save） |
| AwaitingDecision | userRejectAllAndSave | — | Resolving | 批量 `de.reject_diff` + 保存全部脏文档（拒绝 diff 与保存其余手工修改可并存） |
| AwaitingDecision | userForceQuit | 用户显式触发（BR-SYS-STATE-006） | Closing | 不等待任何裁决/保存结果，直接放行；残留数据留待下次启动异常恢复清算 |
| Resolving | resolveOk | — | Verifying | 全部批量操作调用成功 |
| Resolving | resolveFail | — | AwaitingDecision | 展示失败项（保存冲突/裁决失败/AI 修改仍在生成），保留"重试/返回编辑" |
| Verifying | verifyOk | 重新查询 pending=0 且 awaiting_save=0 且 dirty=0 且 saving=0 | Closing | 不信任 Resolving 前的内存计数，必须复核数据库真值 |
| Verifying | verifyRetry | 复核仍有残留 | AwaitingDecision | 回决策入口而非静默放行（防止清点后新增残留被跳过） |
| Closing | — | — | Closed | 调用 `appWindow.destroy()`（沿用既有 close→destroy 兜底路径） |

- **error_states**：无独立错误终态；`resolveFail` 与 `verifyRetry` 均回 `AwaitingDecision` 由用户
  裁决后续动作（重试/强制退出/取消），不做静默重试或静默放弃
- **recovery_paths**：`resolveFail`→用户重试同一批量操作或改选其他决策；`verifyRetry`→重新清点
  （可能因屏障期间仍有回合产出新 diff，理论上应为 0，出现即视为屏障失效需排查，不在本机内静默吞掉）
- **forbidden_transitions**：
  - `Idle → Closing`（未经 `Inspecting` 判定不得直接放行，即便主观认为"应该没有残留"）
  - `Inspecting → Closing`，当 pending>0、awaiting_save>0 或 dirty>0 且未经 `AwaitingDecision`
  - `AwaitingDecision → Closing`，除 `userForceQuit` 外的任何路径跳过 `Resolving`/`Verifying`
  - `Resolving → Closing` 直达（必须经 `Verifying` 复核，不信任执行请求本身的返回值）
  - `BarrierUp` 期间响应 `AgentTurn.send`/`de.generate_diff`（屏障必须先于清点生效）
  - `userForceQuit` 由非用户显式动作触发（不得超时自动强退，不得在 `resolveFail` 时自动降级为强退）
- **acceptance_criteria**：正常关闭完成时刻 `pending=0` 且 `awaiting_save=0` 且 `dirty=0` 且 `saving=0`（`BR-SYS-STATE-004`）；
  `AwaitingDecision` 呈现批量决策入口且"接受全部"强制耦合保存（不提供"接受但放弃保存"组合，
  2026-07-20 人类决策 2）；执行失败不静默放行；复核基于重新查询而非缓存；强制退出仅用户显式触发
  且不参与常规 `Verifying` 复核（直接 `Closing`）；`userCancel` 后系统恢复到与关闭请求前一致的
  可用态（屏障已撤销）
- **test_matrix**：见 §5

## §3 跨模块转移契约（X-INTER-001，本文闭合）

| 转移点 | 方向 | 契约 | 双侧状态 |
|---|---|---|---|
| 关闭屏障→回合取消 | SM-SYS-02 BarrierUp → SM-AG-01 cancel | 屏障建立时对 `Streaming` 回合发 `cancel`；`SM-AG-01` 按既有语义转 `Cancelled` | **双侧就绪**（AG 侧复用既有事件，A-AG-M-S-01 §2） |
| 批量裁决 | SM-SYS-02 Resolving → SM-DE-01（userAccept/userReject 批量） | 复用既有 `de.accept_diff`/`de.reject_diff`/`de.accept_draft_diff` 命令逐条调用；SM-SYS-02 不新增 DE 内部事件，只是批量调用方 | **双侧就绪**（DE 侧无需改动，A-DE-M-S-01 §2 既有转移覆盖） |
| 保存驱动 | SM-SYS-02 Resolving → SM-ED-01（saved）→ SM-WS-01（save_atomic） | "保存全部脏文档"逐个驱动既有保存链，回执驱动 SM-ED-01 Editing→Ready 清脏态 | **双侧就绪**（ED/WS 侧无需改动） |
| 复核查询 | SM-SYS-02 Verifying → 查询 pending_diffs/ed_session_snapshots 当前真值 | 直接查库，不经其余状态机事件；只读 | 双侧就绪（新增只读 IO，见 A-SYS-M-T-02 §6） |

至此「关闭请求→屏障→清点→决策→批量裁决/保存→复核→退出」应用关闭核心交互的状态机链
**SYS→AG/DE/ED/WS** 全链建机完毕，一致性在 G5 跨模块联调验证（APC §5.1）。

## §4 强制退出兜底（BR-SYS-STATE-006 落地）

`userForceQuit` 是 `AwaitingDecision` 态内的显式用户动作，不是常规路径的一部分：触发后直接进
`Closing`，不等待 `Resolving`/`Verifying`。其残留的 `pending`/`dirty`/`awaiting_save` 数据在下次
启动由 `BR-SYS-STATE-005`（异常恢复护栏，A-SYS-M-T-02 §4）统一清算——即强制退出在本机语义上等价
于"用户主动选择让这次退出被下次启动当作异常退出处理"，不新增第三套清算路径，保持异常恢复护栏的
单一权威。

## §5 测试与验收（强制转移测试矩阵，APC §6/§10）

- **正路（清零）**：`closeRequested→BarrierUp→Inspecting→inspectClean→Closing`（无残留直接放行）。
- **正路（决策）**：`Inspecting→inspectDirty→AwaitingDecision→userAcceptAllAndSave→Resolving→
  resolveOk→Verifying→verifyOk→Closing`；同路径覆盖 `userRejectAllAndDiscard`/`userRejectAllAndSave`。
- **取消路**：`AwaitingDecision→userCancel→Idle`，断言屏障已撤销、可正常发起新回合/新 diff。
- **失败重试路**：`Resolving→resolveFail→AwaitingDecision`，断言未产生半执行的裁决/保存残留（复用
  DE/ED 各自事务边界，SM-SYS-02 只做编排不重新定义原子性）。
- **复核不信任缓存**：模拟 `Resolving` 执行期间又新增一条 pending（理论上屏障应阻止，测试断言
  `Verifying` 仍能正确探测到并回 `verifyRetry`，不误放行）。
- **强制退出路**：`AwaitingDecision→userForceQuit→Closing`，不经 `Resolving`/`Verifying`；断言仅
  用户显式动作可达，任何自动/超时路径不可达该转移。
- **禁止转移**：`Idle`/`Inspecting`（有残留）→`Closing` 直达不可达；`Resolving→Closing` 直达不可达；
  `BarrierUp` 期间新回合/新 diff 请求被拒绝。
- **跨模块（G5 联调）**：屏障→AG cancel、批量裁决→DE/ED/WS 既有链路、复核→数据库真值三段对接一致；
  macOS `applicationShouldTerminate:`（Cmd+Q/Dock 退出）与窗口 `CloseRequested` 收敛至同一 `closeRequested` 入口。

## §6 总图登记（A-SYS-M-S-01）

| 核心链 | machine_id | 总图回填动作 |
|---|---|---|
| 应用关闭 | SM-SYS-02 | §3 新增「应用关闭｜SYS｜SM-SYS-02｜active」行（A-SYS-M-S-02 §2） |
| 转移点 | — | §5 新增本文 §3 四条跨模块转移契约 |

## §7 术语归属说明

关闭屏障、决策入口、清点、复核等为应用关闭编排的框架词，按 ADU §3.2 不注册 TERM；
`PendingDiff`/`DiffPayload` 等业务名词主责仍归 DE（A-DE-M-T-01 §0），本文引用不抢注。

## §8 技术债务与盲区（诚实声明）

- **决策入口的具体交互形态**（弹窗视觉、详情展开层级、失败提示文案）属实现期产品决策，本文只锁
  行为约束（批量优先、接受全部强制耦合保存、失败留决策入口、复核不信任缓存）。
- **屏障失效的可观测性**：`verifyRetry` 理论上不应发生（屏障应保证 `Resolving` 期间不产生新
  pending），若实现期发现频繁触发需要回查屏障实现是否存在竞态漏洞，本文只定义"发生时不得静默
  放行"，不承诺屏障绝对无竞态。
- **多窗口/多 workspace 并发**：延期评估，见 A-SYS-M-T-02 §9。

## §9 后续动作

1. 落地后同步编辑 A-SYS-M-S-01：§3 新增应用关闭行（active），§5 登记本文 §3 四个转移点。
2. 随 `npm run governance:generate` 复跑核对。
3. 待人类宣布本部分 G2 闭合。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-07-21 | v1.1 | 收紧 SM-SYS-02 放行 guard：Inspecting/Verifying 均须先同步 flush 编辑态，并把 `awaiting_save=0` 纳入普通关闭清零条件；macOS Cmd+Q/Dock 退出按 `applicationShouldTerminate:` 原生回调进入同一关闭入口。 |
| 2026-07-20 | v1.0 | 初版：定义 SM-SYS-02 应用关闭协调机（Idle/BarrierUp/Inspecting/AwaitingDecision/Resolving/Verifying/Closing，含 APC §6 全部必备字段）；闭合与 SM-AG-01（回合取消）、SM-DE-01（批量裁决）、SM-ED-01/SM-WS-01（保存）三个单侧转移点 + 新增只读复核转移；强制退出兜底（BR-SYS-STATE-006）语义落地为异常恢复护栏的等价触发源；测试矩阵（正路/取消/失败重试/复核不信任缓存/强制退出/禁止转移）；债务（决策入口交互形态延实现期、屏障竞态可观测性、多窗口延期评估）；承接 APP-CLOSE-DIFF-LIFECYCLE-01 |
