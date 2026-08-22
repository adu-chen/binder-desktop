---
文档编号：WS-M-S-01
文档状态：A
负责模块：WS
文档职责：WS 文档内容状态机设计——save 保存机（SM-WS-01）、背景内容 diff 落盘机（SM-WS-02）、文档内容时间轴还原机（SM-WS-03）；工作区资源操作状态机为 A-WS-M-S-02 的 SM-WS-04
上游约束：A-WS-M-T-01_工作台技术设计.md（WS-SAVE/WS-DIFF-APPLY/WS-TIMELINE 链路、BR-WS-PERSIST-001/BR-WS-STATE-001/BR-WS-STATE-002 规则）、A-CORE-C-P-01_APC.md（§6 状态机必备字段与强制阈值）、A-SYS-M-S-01_状态机总图.md（X-SEQ-003 注册前置、X-INTER-001 跨模块转移、§3 总图清单、SY-04）、A-SYS-I-P-01_数据结构定义.md（PendingDiff/TimelineEntry 字段契约）、A-SYS-I-P-02_IPC协议.md（BR-SYS-STATE-002 流式终态、统一错误形状）、ADU.md（§3.1 STATE_MACHINE 格式）
直接承接：本文三机经 rules 字段承载 A-WS-M-T-01 规则；machine_id 登记 A-SYS-M-S-01 总图；diff 落盘机的 accepted 入口对接 DE diff 终态机（X-INTER-001）
使用边界：本文定义 WS 状态机内部（states/transitions/guards/actions/error/recovery/forbidden）；不重定义规则（归 A-WS-M-T-01）、不定义 diff 终态转移（归 DE 状态机）、不重定义 SM-SYS-01（归 A-SYS-I-P-03）；流式终态与 BR-SYS-STATE-002 对齐
变更要求：状态机 transitions/forbidden_transitions 变更须同步 A-WS-M-T-01 对应规则并重跑测试矩阵；跨模块转移（accepted→落盘）变更须与 DE 共同复核 X-INTER-001；machine_id 一经登记总图不可更改
治理成熟度：draft
---

# Binder 工作台状态机设计（WS · L2 技术设计）

## §1 文档定位

本文承接 A-WS-M-T-01 的状态化核心链，按 APC §6 定义 WS 三台状态机并登记 A-SYS-M-S-01 总图：

```
A-WS-M-T-01（规则 SSOT：WS-SAVE/WS-DIFF-APPLY/WS-TIMELINE）→ 本文状态机（states/transitions）
本文 machine_id ──登记──→ A-SYS-M-S-01 §3 总图（save 行 planned→active，回填 SY-04）
SM-WS-02 accepted 入口 ──对接──→ DE diff 终态机（X-INTER-001 跨模块转移契约）
本文流式/中断终态 ──对齐──→ BR-SYS-STATE-002（A-SYS-I-P-02）
```

满足 X-SEQ-003（状态化核心链须先建状态机并登记总图方可实现）：save / diff 落盘 / 时间轴还原三链
在此建机，A-WS-M-T-01 §5/§6/§10 与本文共同解除 SY-04。状态机内部为本文职责；所承载规则归
A-WS-M-T-01，本文以 `rules` 字段引用不重定义。

## §2 SM-WS-01 · save 保存机

<!-- STATE_MACHINE
machine_id: SM-WS-01
owner_module: WS
status: active
summary: 单次文档 save 保存的原子落盘生命周期（无半落盘、失败可重试）
source_type: design
chain: WS-SAVE
rules: BR-WS-PERSIST-001, BR-WS-DATA-001
initial_state: Idle
terminal_states: Saved, SaveError
-->

- **machine_id**：SM-WS-01　**owner_module**：WS
- **scope**：单个 workspace 内文档一次 save 保存的完整生命周期
- **context**：目标 DocRef、待保存缓冲内容、**加载基线（内容哈希，BR-WS-OBS-001）**、落盘结果、是否脏
- **states**：Idle、Dirty、Saving、Saved、SaveError
- **events**：edit、save、saveOk、saveFail、saveConflict、retry、overrideConfirmed
- **transitions（主表）**：

| from | event | guard | to | action |
|---|---|---|---|---|
| Idle | edit | — | Dirty | 标记缓冲为脏 |
| Dirty | save | 路径在 workspace root 内（X-FORBID-004） | Saving | 校验加载基线后启动原子写入 |
| Saving | saveOk | 磁盘内容与加载基线一致 | Saved | 落盘完成，原子提交，刷新基线 |
| Saving | saveConflict | 磁盘内容 ≠ 加载基线（外部修改） | SaveError | 冲突类错误码，**不覆盖外部修改**；交 ED 会话提示裁决（BR-WS-OBS-001） |
| Saving | saveFail | — | SaveError | 统一错误形状，原文件保持 save 前内容（无半落盘） |
| SaveError | retry | 非冲突失败 | Saving | 重试原子写入 |
| SaveError | overrideConfirmed | 用户显式确认覆盖（冲突场景） | Saving | 重置基线为磁盘当前内容后重试写入 |
| Saved | edit | — | Dirty | 再次编辑 |

- **error_states**：SaveError（recovery：retry / overrideConfirmed→Saving；冲突另一出路为 ED 侧
  conflictReload 放弃本次 save）
- **recovery_paths**：SaveError→retry→Saving；冲突→overrideConfirmed（显式覆盖）或 ED 重载放弃；
  中断回 SaveError 后可重试，原文件不被破坏
- **forbidden_transitions**：
  - Dirty → Saved（未经 Saving 原子写入不得直接已保存）
  - Saving → Saved 当磁盘内容 ≠ 加载基线（**不得静默覆盖外部修改**，必经 saveConflict 路径）
  - SaveError（冲突）→ Saving 无 overrideConfirmed（覆盖必须用户显式确认）
  - Saving → Idle（落盘进行中不得静默丢弃为干净态）
- **acceptance_criteria**：仅 Saving→saveOk 可达 Saved；save 中断/失败原文件保持 save 前内容、无半落盘；
  越界路径不存在进入 Saving 的路径；外部修改后无显式覆盖确认不存在通往 Saved 的路径
- **test_matrix**：见 §5（强制覆盖上述 transitions 与 forbidden_transitions，含冲突路）

### 2.1 保存协调与 CreateDocumentBundle 边界（REQ-WS-003/006）

SM-WS-01 只保存已经存在 DiskState 的文档 LogicalState；它不再承担“保存时创建新文件”。新建文档由
SM-WS-04 的 CreateDocumentBundle 子链先完成资源创建，再打开标签并投影初始内容 Diff。

固定协同：

| 场景 | 状态机归属 | 结果 |
|---|---|---|
| 初始内容 Diff 仍有效时保存 | DE 先裁决 accepted/awaiting_save，再由 SM-WS-01 写入既有目标 | 保存成功后内容条目进入 WorkspaceTimeline |
| 初始内容 Diff 接受但不保存 | DE 内容卡生命周期 | DiskState 仍为空文件，LogicalState 保持预览内容 |
| 初始内容 Diff 安全拒绝 | SM-WS-04 校验 operation_id、资源/DiskState/LogicalState 三重基线 | 全部未推进才补偿删除目标并关闭标签 |
| 初始内容 Diff 已失效 | DE expired，SM-WS-04 不补偿删除 | 文件与当前 LogicalState 保留，可正常保存 |
| 保存失败 | SM-WS-01 SaveError | 保留 LogicalState、标签与 accepted/awaiting_save 效果态，不伪报 saved |

`baseline=null 且目标不存在时由 save 创建文件` 的历史实现是迁移债务；新基线下必须拒绝该 Agent 路径，
不能绕过 WorkspaceOperationPlan 与 DecisionRequest。手动“新建空文件”入口是否直接创建属于独立人工 UI
权限，不得被 Agent 工具复用。
> 「保存有效 pending 预览先裁决」的协调入口（DE 先裁决再驱动 save）已由 A-DE-M-S-01 §2/§2.1 建模，
> 本文只承接落盘执行。

## §3 SM-WS-02 · diff 落盘机

> 本机自 DE 终态机的裁决信号入场：diff 的 pending→accepted/rejected/expired **转移语义归 DE**，
> 本机只消费裁决结果执行 WS 侧落盘。`accepted` 入口为跨模块转移点（X-INTER-001）。

<!-- STATE_MACHINE
machine_id: SM-WS-02
owner_module: WS
status: active
summary: 无 LogicalState 的 pending_diff 裁决后 WS 侧落盘生命周期（仅 accepted 写文件 + 记一个时间轴节点）
source_type: design
chain: WS-DIFF-APPLY
rules: BR-WS-STATE-001, BR-WS-DATA-002, BR-CORE-STATE-002
initial_state: AwaitingDecision
terminal_states: Applied, Discarded, ApplyError
-->

- **machine_id**：SM-WS-02　**owner_module**：WS
- **scope**：一条已持久化 pending_diff 自等待裁决到落盘/丢弃的生命周期
- **context**：PendingDiff（A-SYS-I-P-01）、DE 裁决信号、目标 DocRef、落盘结果、所记时间轴节点
- **states**：AwaitingDecision、Applying、Applied、Discarded、ApplyError
- **events**：accepted、rejected、expired、applyOk、applyFail、retry
- **transitions（主表）**：

| from | event | guard | to | action |
|---|---|---|---|---|
| AwaitingDecision | accepted | DE 终态机裁决 accepted，且目标持续无 LogicalState（X-INTER-001 入口） | Applying | 启动落盘 |
| AwaitingDecision | rejected | — | Discarded | 不写文件（终态） |
| AwaitingDecision | expired | 超时/外部变更 | Discarded | 不写文件（终态） |
| Applying | applyOk | 路径在 workspace root 内 | Applied | 写目标文件 + 记一个时间轴节点（version_seq+1） |
| Applying | applyFail | — | ApplyError | 不记节点，统一错误形状，控制态可重试 |
| ApplyError | retry | — | Applying | 重试落盘 |

- **error_states**：ApplyError（recovery：retry→Applying）
- **recovery_paths**：ApplyError→retry→Applying；失败不留半节点，原文件不被破坏
- **forbidden_transitions**：
  - AwaitingDecision → Applied（未经 Applying 不得落盘）
  - AwaitingDecision → Applying（目标已建立 LogicalState；必须迁移至 ED/WS save 路径）
  - Discarded → Applying（rejected/expired 不得绕过裁决再落盘）
  - Applying → Applied（仅 applyOk 且路径合法触发；applyFail 不得记节点）
- **acceptance_criteria**：仅“accepted 且无 LogicalState”→Applying→applyOk 可达 Applied 并记恰好一个
  节点；rejected/expired 永不写文件；目标在 pending 期间打开后不得触发本机落盘；applyFail 不留半节点；
  落盘原子无半落盘（对齐 BR-SYS-STATE-002）
- **test_matrix**：见 §5
- **跨模块转移契约（X-INTER-001）**：`accepted` 信号来自 DE diff 终态机，须与 DE 侧对接一致——
  信号语义（仅无 LogicalState 的 accepted 触发落盘）、落盘确认回执（Applied/ApplyError 回传 DE）双侧约定，登记
  A-SYS-M-S-01 §5。

## §4 SM-WS-03 · 时间轴还原机

<!-- STATE_MACHINE
machine_id: SM-WS-03
owner_module: WS
status: active
summary: 文档历史还原生命周期（前进式记新节点，还原失败保持现状无半还原）
source_type: design
chain: WS-TIMELINE
rules: BR-WS-STATE-002, BR-WS-DATA-002
initial_state: Browsing
terminal_states: Restored, RestoreError
-->

- **machine_id**：SM-WS-03　**owner_module**：WS
- **scope**：一次文档历史回溯与还原的生命周期
- **context**：目标 DocRef、文档时间轴节点列表、选定 TimelineEntry、还原结果、新节点
- **states**：Browsing、Previewing、Restoring、Restored、RestoreError
- **events**：selectNode、confirmRestore、restoreOk、restoreFail、back
- **transitions（主表）**：

| from | event | guard | to | action |
|---|---|---|---|---|
| Browsing | selectNode | 节点快照可取回（snapshot_ref 有效） | Previewing | 预览选定版本 |
| Previewing | selectNode | — | Previewing | 切换预览另一节点 |
| Previewing | confirmRestore | — | Restoring | 执行还原 |
| Restoring | restoreOk | — | Restored | 重建为选定版本 + 记一个新节点（version_seq+1） |
| Restoring | restoreFail | — | RestoreError | 保持现状，统一错误形状 |
| Previewing | back | — | Browsing | 返回节点列表 |
| RestoreError | back | — | Browsing | 返回节点列表，可重选 |

- **error_states**：RestoreError（recovery：back→Browsing 重选/重试）
- **recovery_paths**：RestoreError→back→Browsing；还原失败文档保持还原前现状
- **forbidden_transitions**：
  - Restoring → Restored（仅 restoreOk 触发）
  - RestoreError → Restored（失败态不得跳过保持现状直接已还原）
  - Browsing → Restoring（未经预览确认不得直接还原）
- **acceptance_criteria**：仅 Restoring→restoreOk 可达 Restored 并记恰好一个新节点；还原失败保持现状、
  无半还原；节点快照缺失时不得进入 Restoring（guard 拒绝）
- **test_matrix**：见 §5

## §5 测试与验收（强制转移测试矩阵，APC §6/§10）

每台状态机的测试矩阵须覆盖其全部 transitions 与 forbidden_transitions：

- **SM-WS-01**：Idle→Dirty→Saving→Saved 正路；saveFail→SaveError→retry→Saving 恢复路；
  禁止 Dirty→Saved、越界路径进入 Saving；断言 save 中断原文件完好无半落盘。
- **SM-WS-02**：accepted→Applying→Applied 记一个节点；rejected/expired→Discarded 不写文件；
  applyFail→ApplyError→retry 恢复；禁止 AwaitingDecision→Applied、Discarded→Applying；
  断言 applyFail 不留半节点。X-INTER-001：accepted 信号来自 DE、落盘回执回传 DE 在 G5 联调验证。
- **SM-WS-03**：selectNode→Previewing→confirmRestore→Restoring→Restored 记新节点；
  restoreFail→RestoreError→back 恢复；禁止 RestoreError→Restored、Browsing→Restoring；
  断言还原失败保持现状无半还原；快照缺失 guard 拒绝进入 Restoring。
- **流式/中断终态对齐**（BR-SYS-STATE-002）：三机的失败/中断终态走统一错误形状，中断不产生半落盘
  副作用，控制态可恢复——与 A-SYS-I-P-02 §5 流式逻辑契约一致。
- 量化 NFR（落盘/还原耗时）延 G5 标定（APC §9）。

## §6 总图登记（A-SYS-M-S-01）

本文三机须在 A-SYS-M-S-01 §3 总图清单登记：

| 核心链 | machine_id | 本文状态 | 总图回填动作 |
|---|---|---|---|
| save 保存 | SM-WS-01 | active | §3「save 保存\|WS」行 planned→active，machine_id=SM-WS-01（回填 SY-04） |
| diff 落盘（WS 侧） | SM-WS-02 | active | 总图 §5 X-INTER-001 转移点「持续无 LogicalState 的 diff accepted→落盘」owner 侧机登记 SM-WS-02 |
| 时间轴还原 | SM-WS-03 | active | 总图清单新增 WS 时间轴还原行，machine_id=SM-WS-03 |

> 总图回填由本文落地后同步编辑 A-SYS-M-S-01 §3/§5 完成（owner 模块 G2 登记总图，A-SYS-M-S-01
> §11.3）；diff 终态机（owner=DE）仍为 planned，待 DE G2 定义，X-INTER-001 两侧在 DE G2 完整对接。

## §7 术语归属说明

状态机、转移、终态、guard 等为状态机框架技术词，按 ADU §3.2 不注册 TERM。本文涉及的业务名词
（Workspace、文档时间轴）TERM 已在 A-WS-M-T-01 §0 注册（主责 WS）；pending_diff/diff 终态 TERM
主责归 DE，本文引用不抢注。

## §8 技术债务与盲区（诚实声明）

- **X-INTER-001 仅单侧就绪**：SM-WS-02 的 accepted 入口与落盘回执已定 WS 侧契约；DE diff 终态机
  （owner=DE）待 DE G2 定义，跨模块两侧完整对接与 G5 联调在 DE G2 后闭合。
- **save 节点准入**：SM-WS-02/03 必记节点；SM-WS-01（save）是否记节点取决于 A-WS-M-T-01 §6 准入策略
  （默认否），最终策略待人类确认后回填，不影响三机转移结构。
- 状态机的并发实例隔离（多文档同时 save / 多 diff 同时落盘）属实现期运行时调度，本文定单实例转移
  语义，并发隔离延实现期（承 A-WS-M-T-01 §12 多 workspace 隔离）。

## §9 后续动作

1. 本文落地后同步编辑 A-SYS-M-S-01 §3 总图：save 行 planned→active 回填 machine_id=SM-WS-01，
   新增 SM-WS-03 行，§5 X-INTER-001 转移点登记 SM-WS-02（owner 侧）。
2. 随下一次 `npm run governance:generate` 复跑核对：本文 STATE_MACHINE 块为门评追溯材料（不入 ADUS
   自动抽取），rules 字段引用的 BR-WS-* 规则由 A-WS-M-T-01 注册。
3. 待人类宣布 WS G2 闭合（AI 不自行宣布晋级，APC §2）；DE G2 时完成 X-INTER-001 两侧对接与 G5 联调。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-10 | v1.0 | 初版（WS G2 状态机件）：定义 SM-WS-01 save 保存机 / SM-WS-02 diff 落盘机 / SM-WS-03 时间轴还原机，含 APC §6 全部必备字段（states/transitions/guards/actions/error_states/recovery_paths/forbidden_transitions/acceptance_criteria/test_matrix）；承载 A-WS-M-T-01 规则（BR-WS-PERSIST-001/STATE-001/STATE-002/DATA-001/002）；满足 X-SEQ-003 注册前置；SM-WS-02 accepted 入口登记 X-INTER-001 跨模块转移（对接 DE diff 终态机）；流式/中断终态对齐 BR-SYS-STATE-002；§6 登记 A-SYS-M-S-01 总图（save 行 planned→active，回填 SY-04）；债务（X-INTER-001 单侧就绪待 DE G2、save 节点准入待定、并发隔离延实现期） |
| 2026-06-12 | v1.1 | Q2 缺口补遗：SM-WS-01 扩展保存冲突路径——context 增加载基线；新增 saveConflict/overrideConfirmed 事件与转移；forbidden 增「磁盘≠基线不得达 Saved」「冲突覆盖必须 overrideConfirmed」；acceptance/test_matrix 同步（承载 BR-WS-OBS-001） |
| 2026-08-13 | v1.2 | DIFF-STATE-COMPASS-01：SM-WS-02 rules 增 BR-CORE-STATE-002，入口 guard 固定为“accepted 且持续无 LogicalState”；目标 pending 期间打开后不得进入本机。 |
| 2026-08-14 | v1.3 | AG-1.0-D2-DE-ED-WS-STATEMACHINE-01：新增 §2.1 保存协调与未保存新文档保存契约（REQ-WS-003/006）——保存有效 pending 预览由 DE 先裁决为 accepted/awaiting_save 再驱动 SM-WS-01 落盘、未保存新文档保存＝创建文件（D-NEW-002/004/007）；补 REQ-WS-003/006 的 REQ_MAP（BR-CORE-STATE-001~003、BR-SYS-PERSIST-001）；登记未保存新文档创建语义与旧直写路径的迁移债务。 |
| 2026-08-14 | v1.4 | AG-1.0-IMP-AGENT-CREATE-SAVE-01：§2.1 契约落地（仅代理新建生效，手动新建保持 ws.create_doc 立即建文件不变）——route_save_doc 对 baseline=null（未保存新文档身份）目标不存在时 create_dir_all 父目录 + 原子落盘 + 时间轴节点（V-N2 恰一）+ docx 补 mark_bdocx 身份；目标已存在 → WS_EXISTS 冲突（V-N4 不覆盖外部创建，保留未保存内容）；失败经既有保存机 SaveError 传播不伪报已创建；route_save_doc 补 @GOV（BR-WS-PERSIST-001/OBS-001/DATA-001）+ 2 测试。 |
| 2026-08-21 | v1.5 | 工作区资源操作基线重构：本文限定为文档内容 SM-WS-01/02/03；§2.1 旧未保存新建链降为历史实现证据；新建、资源执行、补偿与资源恢复统一由 A-WS-M-S-02 的 SM-WS-04 承载。 |
