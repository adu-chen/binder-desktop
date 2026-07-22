---
文档编号：SYS-M-T-02
文档状态：A
负责模块：SYS
文档职责：SYS 应用关闭生命周期 L2 技术设计（规则 SSOT）——正常关闭前 diff/未保存内容终态化护栏、
异常退出后启动清算语义、关闭屏障与强制退出兜底；承接 REQ-SYS-007
上游约束：A-SYS-M-D-01_系统底座功能需求描述.md（REQ-SYS-007）、A-CORE-C-P-01_APC.md（§4.1、§6）、
A-CORE-C-T-01_三态模型.md（三态单向数据流、X-FORBID-008/009）、A-SYS-I-P-01_数据结构定义.md
（PendingDiff/ed_session_snapshots 字段单一来源、X-SHARED-001）、A-DE-M-T-01_DiffEdit技术设计.md
（BR-DE-STATE-001/002、diff 终态语义）、A-ED-M-T-01_编辑器技术设计.md（BR-ED-STATE-001 脏态守护）、
A-AG-M-S-01_代理状态机设计.md（SM-AG-01 cancel 事件）、ADU.md（§3）
直接承接：本文 RULE/CONSTRAINT 为应用关闭生命周期规则 SSOT，代码经 @GOV 映射；SM-SYS-02 在
A-SYS-M-S-02 定义并登记总图（A-SYS-M-S-01）
使用边界：本文是应用关闭护栏的规则 SSOT；不重定义 PendingDiff/ed_session_snapshots 字段形状
（归 SYS A-SYS-I-P-01，本文只声明新增字段的语义要求）、不做具体 UI 交互形态（归实现期产品决策，
仅锁定"接受全部强制耦合保存""处理中禁用重复提交""失败留在弹窗"等行为约束）、状态机内部归
A-SYS-M-S-02
变更要求：关闭护栏语义变更须先经 APC 授权（涉及数据完整性核心交互）；`effect_status`/
`terminal_reason` 字段变更须回写 A-SYS-I-P-01 并复核 DE
治理成熟度：draft
---

# Binder 应用关闭技术设计（SYS · L2 技术设计）

## §1 文档定位

```
REQ-CORE-002 → REQ-SYS-007（L1）→ 本文（SSOT）→ 代码(@GOV)
本文 ←承接← A-CORE-C-T-01（三态单向数据流）、A-SYS-I-P-01（字段单一来源）、
       A-DE-M-T-01（diff 终态语义）、A-ED-M-T-01（脏态守护）
本文 →驱动→ A-SYS-M-S-02（SM-SYS-02 应用关闭协调机）→登记→ 总图（A-SYS-M-S-01）
```

Binder 是本地优先的写作应用（APC §1），diff 裁决权与未保存编辑在退出前的归属是数据完整性的核心
边界：一旦允许"关闭时静默丢弃裁决权"或"重启后复活过期的裁决态"，用户会在不知情的情况下丢失或
误判修改内容。本文承接 2026-07-20 人类三轮确认口径，定义应用关闭前后 diff 与未保存编辑必须满足
的护栏规则。

**分层重申**：diff 终态互斥不可逆语义归 DE（`SM-DE-01`不变）、脏态守护语义归 ED（`SM-ED-01`不变）、
字段形状归 SYS（A-SYS-I-P-01）；本文只新增**应用级编排规则**——正常关闭前必须驱动 DE/ED 达到
"pending=0 且 dirty=0"，异常退出后必须清算残留而非当作可信恢复源。

## §2 关闭链路（APP-CLOSE）

<!-- CHAIN
chain_id: APP-CLOSE
主责模块: SYS
status: active
risk_tier: P1
-->

APP-CLOSE 覆盖：关闭请求触发（窗口关闭按钮 / Cmd+Q / Dock 退出 / 系统关机通知）→ 关闭屏障 → 清点
→（如有残留）用户决策 → 执行裁决/保存 → 复核 → 实际退出；以及应用启动时的异常退出清算前置步骤。

## §3 正常关闭护栏（解除审计发现：关闭钩子未检查 diff/脏态）

<!-- RULE
rule_id: BR-SYS-STATE-004
主链路: APP-CLOSE
域: STATE
-->
应用正常退出（窗口关闭、Cmd+Q、Dock 退出、菜单退出、系统关机通知等一切非崩溃路径）必须满足：
退出前当前 workspace 的全部待裁决 diff（`pending_diffs.status='pending'`）、全部未确认写盘的已接受
效果（`status='accepted' AND effect_status='awaiting_save'`）与全部未保存文档（含非激活文档，
`ed_session_snapshots.dirty=1` 或激活文档脏态）清零后才允许实际关闭窗口/进程。检查前必须同步 flush
编辑器快照；flush 失败即拒绝放行，不得用旧数据库快照判定"无未保存内容"。不满足则必须先阻断关闭并
呈现统一决策入口，不得静默丢弃裁决权或未保存内容，也不得静默继续退出。

决策入口必须满足：批量优先（接受全部/拒绝全部，不要求逐条裁决）；选择"接受全部"时未保存文档的
处理方式强制耦合为"保存"，不得选择"放弃"（避免"已接受但主动放弃保存"复现历史不一致）；执行为
异步操作，执行期间禁止重复提交与新的关闭请求；执行失败（保存冲突/裁决失败）必须停留在决策入口
并可重试，不得静默放行关闭；执行成功后必须重新查询当前真实的 pending/awaiting_save/dirty 计数
确认清零，不得信任决策发起时刻的内存快照。

关闭请求触发后立即建立关闭屏障：拒绝新的 `AgentTurn.send`/`de.generate_diff`；正在进行中的代理
回合（`SM-AG-01` 处于 `Streaming`）发 `cancel` 使其尽快达终态，避免清点后又产生新的待裁决 diff。

用户可在决策入口显式取消关闭（撤销屏障，恢复正常可编辑/可发起回合）。

<!-- RULE
rule_id: BR-SYS-STATE-006
主链路: APP-CLOSE
域: STATE
-->
决策入口必须提供人类确认的强制退出兜底出口（2026-07-20 人类决策 3）：当常规裁决/保存路径反复
失败、或用户明确选择放弃等待时，可在决策入口内显式触发强制退出——直接销毁窗口/进程，允许残留
未终态数据（pending diff、未保存内容）。强制退出不得设计为自动触发（超时自动强退、静默降级等），
必须是用户在弹窗内可见、需额外确认的显式动作；强制退出后的残留数据按 `BR-SYS-STATE-005`
（异常恢复护栏）在下次启动时处理，不视为独立的第三套清算语义。

## §4 异常恢复护栏（解除审计发现：重启复活 pending/dirty）

<!-- RULE
rule_id: BR-SYS-STATE-005
主链路: APP-CLOSE
域: STATE
-->
应用启动时，在恢复任何编辑器会话或代理历史之前，必须对活动/待恢复 workspace 执行异常退出清算：

1. 该 workspace 内全部 `pending_diffs.status='pending'` 行，一律转 `expired`
   （`terminal_reason='abnormal_exit'`），复用 DE 既有过期落终态语义（`discard_diff`：不写文件、
   不记时间轴节点）；清算后不得再以可裁决形态呈现，只能作为只读历史展示。
2. 该 workspace 内全部 `ed_session_snapshots.dirty=1` 行代表未保存内容，一律丢弃（删除该持久化
   镜像行），对应文档重新以磁盘态（DiskState）为准装载，不得把该行内容当作可信 LogicalState 恢复。
3. 该 workspace 内全部 `pending_diffs.status='accepted' AND effect_status='awaiting_save'` 行，一律转
   `effect_status='discarded_on_abnormal_exit'`、`terminal_reason='abnormal_exit'`。`awaiting_save` 是尚未
   确认写盘的当前会话态，不能以 dirty 快照是否恰好已持久化作为保留条件（编辑快照存在防抖写入竞态）；
   历史展示需能区分"已接受且已保存"与"已接受但内容已丢失"，不得笼统显示为"已完成"。
4. 判定"上次退出是否异常"不新增独立的洁净关闭标记：由于 `BR-SYS-STATE-004` 保证正常关闭完成时
   pending、awaiting_save 与 dirty 均已清零，本步骤发现的任何残留本身即是跨会话残留，必须按本规则
   清算；`BR-SYS-STATE-006` 触发的强制退出等同于产生了这类残留，按本规则统一处理，不单独区分
   "强制退出"与"崩溃"两种子情形。
5. 清算必须先于 `ed_open_tabs`/`agent_sessions`/`agent_turns` 等控制态恢复执行，避免恢复流程读到
   将被清算删除的陈旧数据。

<!-- RULE
rule_id: BR-SYS-DATA-005
主链路: APP-CLOSE
域: DATA
-->
`pending_diffs` 新增 `effect_status`（`not_applicable` / `awaiting_save` / `saved` /
`discarded_on_abnormal_exit`）与 `terminal_reason`（可空，取值含 `abnormal_exit`，其余复用 DE
既有过期原因集合）两列，字段登记单一来源为 A-SYS-I-P-01（X-SHARED-001，本文不重复定义形状，只
锁语义）。取值转移：`pending`/`rejected`/`expired` 期间恒为 `not_applicable`；`accepted` 瞬间按
DE 三路径分流（A-DE-M-S-01 §4）写入 `saved`（`background` 路径，写盘与裁决同一事务原子完成）或
`awaiting_save`（`current_editor`/`open_session` 路径，等待 ED/WS save）；`awaiting_save` 后续
经真实保存成功转 `saved`，或经 `BR-SYS-STATE-004` 的显式放弃、`BR-SYS-STATE-005` 的启动清算转
`discarded_on_abnormal_exit`。`awaiting_save` 是严格的进程会话态：任一新会话开始前必须转出该态。
`saved` 与 `discarded_on_abnormal_exit` 均为终态，不可逆。

**存量数据回填（2026-07-20 人类决策 4，随迁移一次性执行）**：迁移上线前已存在的 `pending_diffs`
行中，`status IN ('pending','rejected','expired')` 回填 `effect_status='not_applicable'`；
`status='accepted'` 回填 `effect_status='saved'`（历史数据无法逐行区分是否仍
`awaiting_save`，保守假设已完成，避免误标"效果丢失"；此假设为已知限制，非缺陷，记入 §9）。

**丢弃触发源（`terminal_reason` 细化）**：`discarded_on_abnormal_exit` 是唯一的"内容随未保存草稿
丢失"效果态取值，覆盖两个触发源——启动期异常退出清算（`terminal_reason='abnormal_exit'`，
`BR-SYS-STATE-005`）与正常关闭流程内用户显式选择放弃未保存修改（`terminal_reason=
'user_discarded_on_close'`，`BR-SYS-STATE-004` 决策入口"拒绝全部并放弃"分支）。两者效果态取值
相同（内容确未落盘），仅 `terminal_reason` 区分具体原因供历史展示；不为"用户主动放弃"新增独立
effect_status 枚举值，避免枚举膨胀。

## §5 跨模块约束

<!-- CONSTRAINT
constraint_id: X-FORBID-010
摘要: 应用正常退出（非崩溃）完成前，当前 workspace 不得残留 status=pending 的 diff、effect_status=awaiting_save 的效果或 dirty=1 的未保存文档；不满足时必须阻断退出并呈现统一决策入口，不得静默继续退出或静默丢弃
涉及模块: SYS, DE, ED, WS, AG
链路影响: APP-CLOSE, DE-DIFF, ED-EDIT, WS-SAVE, AG-TURN
-->

<!-- CONSTRAINT
constraint_id: X-FORBID-011
摘要: 应用启动时，任何跨会话残留的 pending diff、awaiting_save 效果或未保存 LogicalState 快照不得被当作可信权威态直接恢复为可裁决/可编辑态；必须先经异常退出清算转终态/丢弃，只能以只读历史形态呈现
涉及模块: SYS, DE, ED, WS
链路影响: APP-CLOSE, DE-DIFF, ED-EDIT, DATA-MIGRATE
-->

X-FORBID-010/011 与既有 X-FORBID-008（三态单向数据流）、X-FORBID-009（LogicalCandidate 转正边界）
不冲突：本规则约束的是"退出前后的编排时序"，不改变 diff/编辑的转正路径本身。

## §6 关闭屏障与代理回合协同

关闭屏障建立后：`AgentTurn.send` 与 `de.generate_diff` 一律拒绝（返回可重试=false 的统一错误
形状）；正在 `Streaming` 的回合经既有 `cancel` 事件驱动至 `Cancelled`（复用 `SM-AG-01`，不新增
AG 内部状态，A-AG-M-S-01 §2）。屏障与决策入口的取消/强制退出联动：用户取消关闭须撤销屏障恢复
正常可用；强制退出不要求等待回合 `cancel` 完成，直接进入 `BR-SYS-STATE-006` 路径。

## §7 需求到规则映射（REQ_MAP）

<!-- REQ_MAP
req_id: REQ-SYS-007
target_type: CHAIN
target_id: APP-CLOSE
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-007
target_type: RULE
target_id: BR-SYS-STATE-004
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-007
target_type: RULE
target_id: BR-SYS-STATE-005
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-007
target_type: RULE
target_id: BR-SYS-STATE-006
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-007
target_type: RULE
target_id: BR-SYS-DATA-005
status: mapped
-->

## §8 测试与验收

- **正常关闭清零**（BR-SYS-STATE-004）：存在 pending/dirty 时关闭被阻断并弹出决策入口；接受全部
  强制耦合保存（不可单独选放弃）；执行成功后重新查询确认 pending=0 且 dirty=0 才真正退出；执行
  失败停留决策入口可重试；用户可取消关闭恢复正常可用；屏障期间新回合/新 diff 生成被拒绝，
  Streaming 回合被 cancel。
- **强制退出**（BR-SYS-STATE-006）：仅用户显式触发才生效，不自动/超时触发；触发后允许残留数据，
  下次启动按异常恢复护栏处理。
- **异常恢复清算**（BR-SYS-STATE-005，先于会话/历史恢复执行）：残留 pending 全部转 expired
  （reason=abnormal_exit）且不可再裁决；残留 dirty 快照被丢弃且对应文档重载磁盘态；
  全部 accepted+awaiting_save 转 discarded_on_abnormal_exit（不依赖 dirty 快照）；清算顺序验证（先于
  `ed_open_tabs`/`agent_sessions` 恢复）。
- **效果态字段**（BR-SYS-DATA-005）：background 路径 accepted 时 effect_status 立即 saved；
  current_editor/open_session 路径 accepted 时 awaiting_save，真实保存成功后转 saved；存量数据
  回填正确（accepted→saved，其余→not_applicable）。
- **跨模块（G5 联调）**：AG 回合取消 ↔ 关闭屏障、DE 批量裁决 ↔ ED/WS 保存 ↔ 复核清零、Rust
  macOS `applicationShouldTerminate:`（Cmd+Q/Dock 退出）与窗口 `CloseRequested` 收敛同一入口。

## §9 技术债务与盲区（诚实声明）

- **存量数据回填的保守假设**：迁移前 `status='accepted'` 行统一回填 `saved`，无法逐行还原历史
  真实落盘状态；若后续发现历史数据展示与实际不符，需人工核查个案，不在本次范围内做逐行取证。
- **多窗口场景未覆盖**：当前单窗口单活动 workspace 架构下本设计成立；若未来引入多窗口/多
  workspace 并发，需重新评估关闭屏障与清点范围（是否按窗口还是按 workspace 聚合）。
- **强制退出后的用户告知**：强制退出允许残留数据丢失，当前只锁"必须显式触发"，未定义是否需要
  在下次启动时对用户做一次性告知（如"上次强制退出丢失了 N 处未保存修改"），延实现期产品决策。
- **关闭耗时的 NFR**：清点/裁决/保存的性能预算（大量 pending diff 或大文档场景下决策入口响应
  延迟）延 G5 标定（APC §9）。

## §10 后续动作

1. 落地后随 `npm run governance:generate` 注册：APP-CLOSE 链路、BR-SYS-STATE-004/005/006、
   BR-SYS-DATA-005 规则、X-FORBID-010/011 约束、REQ_MAP 进 ADUS。
2. 续 A-SYS-M-S-02（SM-SYS-02 应用关闭协调机），回填总图（A-SYS-M-S-01）。
3. 待人类宣布 SYS G2 本部分闭合；多窗口场景延期评估随产品路线回填。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-07-21 | v1.1 | 收紧跨会话终态约束：`awaiting_save` 不再依赖 dirty 快照命中，启动期对其无条件转 `discarded_on_abnormal_exit`；普通关闭检查/复核把 awaiting_save 纳入清零条件并要求检查前同步 flush 编辑器态；macOS Cmd+Q/Dock 退出改按 `applicationShouldTerminate:` 原生回调收敛。 |
| 2026-07-20 | v1.0 | 初版：应用关闭生命周期规则 SSOT——正常关闭护栏（BR-SYS-STATE-004，pending/dirty 清零前置、批量决策、接受全部强制耦合保存、执行失败留决策入口、复核不信任内存快照）、关闭屏障（拒绝新回合/新 diff、Streaming 回合 cancel）、强制退出兜底（BR-SYS-STATE-006，仅用户显式触发）、异常恢复护栏（BR-SYS-STATE-005，先于会话恢复清算 pending→expired 与 dirty 丢弃、accepted+awaiting_save 命中残留转 discarded_on_abnormal_exit）、效果态字段语义（BR-SYS-DATA-005，effect_status/terminal_reason 转移与存量回填口径）、新增约束 X-FORBID-010/011；承接 APP-CLOSE-DIFF-LIFECYCLE-01 三轮人类确认决策 |
