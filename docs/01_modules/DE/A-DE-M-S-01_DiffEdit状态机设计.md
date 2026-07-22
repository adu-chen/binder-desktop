---
文档编号：DE-M-S-01
文档状态：A
负责模块：DE
文档职责：DE Diff Edit 模块级状态机设计——diff 终态机（SM-DE-01），含 APC §6 必备字段；回填 A-SYS-M-S-01 总图 diff 终态行（planned→active），闭合 SM-WS-02 accepted 入口与 SM-AG-01 HandingOff 出口两个单侧 X-INTER-001 转移点
上游约束：A-DE-M-T-01_DiffEdit技术设计.md（DE-DIFF 链路、BR-DE-* 规则）、A-CORE-C-P-01_APC.md（§6）、A-SYS-I-P-01_数据结构定义.md（status 枚举集合）、A-SYS-M-S-01_状态机总图.md（X-SEQ-003/X-INTER-001）、A-WS-M-S-01_工作台状态机设计.md（SM-WS-02 accepted/rejected/expired 事件入口）、A-AG-M-S-01_代理状态机设计.md（SM-AG-01 HandingOff）、A-ED-M-S-01_编辑器状态机设计.md（装饰呈现依附说明）、ADU.md（§3.1）
直接承接：SM-DE-01 经 rules 字段承载 A-DE-M-T-01 规则；machine_id 登记总图；裁决信号驱动 SM-WS-02、生成入口承接 SM-AG-01
使用边界：本文定义 DE 状态机内部；不重定义 SM-WS-02/SM-AG-01 内部；DE 机状态多于 status 枚举（含 Generating/GenFailed 过程态），枚举集合语义对齐 A-SYS-I-P-01 不漂移；审阅 UI 形态不在本文（DE-04）
变更要求：transitions 变更须同步 A-DE-M-T-01 规则并重跑测试矩阵；与 WS/AG/ED 的转移点变更须对侧共同复核；machine_id 一经登记不可更改
治理成熟度：draft
---

# Binder Diff Edit 状态机设计（DE · L2 技术设计）

## §1 文档定位

```
A-DE-M-T-01（规则 SSOT：DE-DIFF）→ 本文 SM-DE-01
本文 machine_id ──登记──→ A-SYS-M-S-01 §3（diff 终态行 planned→active）
入口：SM-AG-01 HandingOff（ChangeProposal 交接，AG 侧已就绪 → 本文闭合双侧）
出口：SM-WS-02 accepted/rejected/expired 事件（WS 侧已就绪 → 本文闭合双侧）
```

满足 X-SEQ-003。本机状态集合多于 PendingDiff.status 枚举（增 Generating/GenFailed 过程态）；
枚举值 pending/accepted/rejected/expired 与 A-SYS-I-P-01 单一来源严格对齐（Pending 态 ↔
status=pending，三裁决终态 ↔ 对应枚举值），无第三处定义。

## §2 SM-DE-01 · diff 终态机

<!-- STATE_MACHINE
machine_id: SM-DE-01
owner_module: DE
status: active
summary: 单条待裁决修改自生成到终态闭合的生命周期（三终态互斥不可逆；当前编辑器草稿与已打开非激活文档 accepted 由 save 落盘）
source_type: design
chain: DE-DIFF
rules: BR-DE-DATA-001, BR-DE-STATE-001, BR-DE-STATE-002, BR-DE-DATA-003
initial_state: Generating
terminal_states: Accepted, Rejected, Expired, GenFailed
-->

- **machine_id**：SM-DE-01　**owner_module**：DE
- **scope**：单条待裁决修改（PendingDiff）自 ChangeProposal 交接到终态闭合
- **context**：ChangeProposal、PendingDiff（payload 含 before_snapshot）、目标 blockId 集合、
  DiffProjection（draft_mode/snapshot_source/base_session_version/base_content_hash）、裁决输入、
  外部变更检测结果、编辑器草稿应用状态、已打开非激活文档自动应用状态、后台 apply 回执
- **states**：Generating、Pending、Accepted、Rejected、Expired、GenFailed
- **events**：genOk、genFail、autoApplyOk、autoApplyFail、userAccept、userReject、timeout、externalChange、appliedOk、applyFail、startupAbnormalExitSweep
- **transitions（主表）**：

| from | event | guard | to | action |
|---|---|---|---|---|
| Generating | genOk | blockId 锚有效且 WS 持久化成功 | Pending | status=pending；经 ED 装饰层呈现审阅 |
| Generating | genFail | 锚失效/持久化失败 | GenFailed | 回报 AG（SM-AG-01 handoffFail）；不进入裁决 |
| Pending | autoApplyOk | draft_mode=open_session 且 ED session 基线匹配 | Accepted | status=accepted（不可逆）；内容已进入已打开非激活文档 LogicalState；不触发 SM-WS-02，后续由 ED/WS save 落盘；DiffCard 禁用裁决 |
| Pending | autoApplyFail | draft_mode=open_session 且 ED session 基线不匹配/事务不支持/应用失败 | Expired | status=expired；不触发 SM-WS-02；DiffCard 禁用裁决并显示失效原因 |
| Pending | userAccept | draft_mode=current_editor 且当前编辑器草稿已应用 | Accepted | status=accepted（不可逆）；不触发 SM-WS-02，后续由 ED/WS save 落盘 |
| Pending | userAccept | draft_mode=background 且目标文档未打开 | Accepted | status=accepted（不可逆）；触发 SM-WS-02 accepted（后台 apply 路径） |
| Pending | userReject | draft_mode!=open_session | Rejected | status=rejected；草稿已应用则 ED 回滚 before；否则通知 SM-WS-02 rejected（不落盘）；ED 装饰复位 |
| Pending | timeout | 超过可配置时限 | Expired | status=expired；通知 SM-WS-02 expired；ED 装饰复位 |
| Pending | externalChange | 目标块内容 ≠ before_snapshot（检测可得） | Expired | 同上（检测不可得保守保持 Pending，BR-DE-STATE-002） |
| Accepted | saved | ED/WS save 成功（草稿路径） | Accepted | 落盘闭合：ED GreenReview 复位（终态保持） |
| Accepted | appliedOk | SM-WS-02 Applied 回执（后台路径） | Accepted | 落盘闭合：ED 装饰复位（终态保持） |
| Accepted | applyFail | SM-WS-02 ApplyError 回执 | Accepted | 终态不回退；落盘重试归 WS 侧（SM-WS-02 retry） |
| Pending | startupAbnormalExitSweep | 应用启动清算发现该 diff 所属 workspace 存在未清零残留（BR-SYS-STATE-005，SM-SYS-02 驱动） | Expired | status=expired，terminal_reason=abnormal_exit；先于会话/历史恢复执行，不进入裁决投影 |

- **error_states**：GenFailed（终态，回报 AG 可发起新回合重试，不复用本条 diff）
- **recovery_paths**：GenFailed→AG 侧重新产出（新 diff 新实例）；草稿路径 saveFail→ED 保持脏态可重试；
  applyFail→WS 侧 retry（本机终态不变）
- **forbidden_transitions**：
  - Generating → Accepted/Rejected/Expired（未持久化呈现不得裁决）
  - Accepted ↔ Rejected ↔ Expired（三终态互斥，任意互转禁止）
  - Expired / Rejected → Accepted（不得复活已关闭 diff）
  - Pending → 文件落盘（无 userAccept 或 save/apply 不存在通往文件写入的路径）
  - Pending → userAccept/userReject，当 draft_mode=open_session（自动应用路径不提供用户裁决入口）
  - Pending → SM-WS-02 accepted，当目标文档已打开（已打开文档只改 LogicalState，不后台写 DiskState）
  - Accepted → Pending（ApplyError 不得回退终态）
- **acceptance_criteria**：三终态互斥不可逆且与 status 枚举一一对应；当前编辑器草稿路径经
  userAccept→Accepted 后不直接落盘，必须等待 ED/WS save；已打开非激活文档经
  autoApplyOk→Accepted 后只修改 LogicalState 并禁用裁决，必须等待 ED/WS save；后台 apply 路径通往
  SM-WS-02 Applying 的唯一路径仍经 userAccept→Accepted 且目标未打开；rejected/expired 后文件不变、
  装饰复位；externalChange 检测不可得时不自动过期
- **test_matrix**：见 §4

## §3 跨模块转移契约（X-INTER-001，本文闭合双侧）

| 转移点 | 方向 | 契约 | 双侧状态 |
|---|---|---|---|
| 建议交接 | SM-AG-01 HandingOff → SM-DE-01 Generating | ChangeProposal（定位锚+修改内容+来源）交接；genOk→AG handoffOk，genFail→AG handoffFail | **双侧就绪**（AG 侧 A-AG-M-S-01 §2，本文闭合） |
| 草稿确认保存 | SM-DE-01 Accepted → SM-ED-01 Editing → SM-WS-01 save | 当前编辑器草稿路径 accepted 只确认，文件写入由用户 save 触发；Saved 后 GreenReview 复位 | **双侧就绪**（ED/WS save 侧已就绪） |
| 未打开文档后台裁决落盘 | SM-DE-01 Accepted → SM-WS-02 accepted | 目标文档未打开的后台/整篇 diff 可由 accepted 触发 apply；Applied/ApplyError 回执回传本机（终态不回退） | **双侧就绪**（WS 侧 A-WS-M-S-01 §3，本文闭合） |
| 已打开非激活文档自动应用 | SM-DE-01 Pending → SM-ED-01 Editing → Accepted | `draft_mode=open_session` 基线校验通过后直接改对应 ED session LogicalState；不要求用户切换窗口，不触发 SM-WS-02；失败转 Expired/不可裁决投影 | **双侧就绪**（ED 侧待本文后续代码承接） |
| 不落盘关闭 | SM-DE-01 Rejected/Expired → SM-WS-02 rejected/expired | 驱动 SM-WS-02 → Discarded（不写文件） | **双侧就绪** |
| 审阅呈现 | SM-DE-01 ↔ ED 装饰层 | Pending 呈现/终态复位；裁决交互经 ED 转达（X-FORBID-005）；装饰为本机投影不独立建机（A-ED-M-S-01 §4 豁免成立） | 双侧就绪 |
| 关闭前批量裁决 | SM-SYS-02 Resolving → SM-DE-01（userAccept/userReject 批量） | 应用关闭决策入口批量调用既有 `de.accept_diff`/`de.reject_diff`/`de.accept_draft_diff`，本机不新增事件，仅为既有转移的批量调用方（BR-SYS-STATE-004） | **双侧就绪**（SYS 侧 A-SYS-M-S-02 §3，2026-07-20 新增） |
| 启动异常退出清算 | SM-SYS-02（启动期）→ SM-DE-01 Pending → Expired | 应用启动、先于会话/历史恢复执行；workspace 内残留 `status=pending` 一律转 `expired`，`terminal_reason=abnormal_exit`，不进入裁决投影（BR-SYS-STATE-005） | **双侧就绪**（SYS 侧 A-SYS-M-S-02 §4，2026-07-20 新增） |

至此「对话生成 diff→裁决或自动应用→LogicalState/保存或未打开后台 apply」核心交互的状态机链 **AG→DE→ED/WS**，
两侧一致性在 G5 跨模块联调验证（APC §5.1）。应用关闭前后的批量裁决与异常退出清算属应用级编排，
两个新增转移点归 **SYS→DE**（A-SYS-M-S-02，2026-07-20 新增），一致性同样在 G5 验证。

## §4 三路径选择规则（A-CORE-C-T-01 §5 · D5 与 2026-07-18 决议落地）

SM-DE-01 生成 PendingDiff 后，根据 DiffProjection 的 `draft_mode` 分三支处理。选择规则如下：

| 条件 | 路径 | 三态语义 | 代码入口 | 触发落盘 |
|---|---|---|---|---|
| `draft_mode=current_editor` | **激活草稿路径** | LogicalCandidate → LogicalState（已进入 ProseMirror doc），userAccept 只确认裁决态 | `de.accept_draft_diff` | 等待用户 ⌘S → `save_atomic` |
| `draft_mode=open_session` 且基线匹配 | **已打开非激活自动应用路径** | LogicalCandidate → LogicalState（已进入目标 ED session），autoApplyOk 直接 Accepted；DiffCard 禁用裁决 | ED session apply → `de.accept_draft_diff` | 等待用户保存目标文档 |
| `draft_mode=open_session` 且基线不匹配/事务不支持 | **失效路径** | LogicalCandidate 不转正；DiffCard 禁用裁决并记录原因 | `deStore.setLocalView("expired"/"error")` | 无 |
| `draft_mode=background` 且目标文档未打开 + `isDocxPath(relPath) === false` | **未打开后台路径** | LogicalCandidate → LogicalState + 同步触发 DiskState 写入 | `de.accept_diff` → `ws.apply_diff` → `save_atomic` | accepted 时立即触发 |
| `draft_mode=background` 且目标文档已打开 | **拒绝（报错）** | 已打开文档禁止后台 apply，必须使用 open_session/current_editor | `deStore.setLocalView("error", "DE_OPEN_DOC_REQUIRES_LOGICAL_STATE")` | 无 |
| `draft_mode=background` + `isDocxPath(relPath) === true` | **拒绝（报错）** | BDocx 必须进入打开文档 LogicalState，不支持后台 raw apply | `deStore.setLocalView("error", "DE_BDOCX_DRAFT_REQUIRED")` | 无 |

> 只有 `current_editor/background` 路径提供用户裁决入口。`open_session` 路径在模型回传 DiffProjection 后立即尝试自动应用，
> 成功即 accepted，失败即不可裁决；禁止要求用户先切换或打开目标窗口。

## §5 测试与验收（强制转移测试矩阵，APC §6/§10）

- **正路**：Generating→genOk→Pending→ED 草稿应用→userAccept→Accepted→saved（save 落盘 + 复位）；
  已打开非激活路径为 Generating→genOk→Pending→autoApplyOk→Accepted（不触发 SM-WS-02）；
  未打开后台路径为 Generating→genOk→Pending→userAccept→Accepted→appliedOk。
- **拒绝/过期路**：userReject→Rejected、timeout/externalChange→Expired，断言文件不变 + 装饰复位 +
  SM-WS-02 达 Discarded。
- **生成失败路**：genFail→GenFailed 回报 AG，不进入裁决、无持久化残留。
- **禁止转移**：三终态互转不可达；Expired/Rejected 不可复活；无 userAccept 不存在通往保存/apply 路径；
  ApplyError 后终态保持 Accepted；`open_session` 不存在 userAccept/userReject；已打开文档不存在后台 apply。
- **保守过期**：externalChange 检测不可得时保持 Pending（不误杀）。
- **冲突重检**（BR-DE-DATA-003）：先序 diff 落盘后，重叠 pending diff 触发 externalChange 重检转
  Expired；不重叠者不受影响。
- **跨模块（G5 联调）**：AG↔DE 交接回执、DE↔ED 草稿应用/回滚/呈现复位、DE↔WS save 或后台 apply
  回执三链双侧一致。

## §6 总图登记（A-SYS-M-S-01）

| 核心链 | machine_id | 总图回填动作 |
|---|---|---|
| diff 终态 | SM-DE-01 | §3 行 planned→active（A-DE-M-S-01 §2） |
| 转移点 | — | §5（原 §5 前）「diff accepted→LogicalState/落盘」「AG 回合门禁与产出」两条更新为双侧就绪 |

## §7 术语归属说明

PendingDiff / ChangeProposal 已在 A-DE-M-T-01 §0 注册（主责 DE）；内容块/装饰层归 ED、
Workspace/时间轴归 WS、代理回合归 AG，本文引用不抢注；状态机框架词不注册。

## §8 技术债务与盲区（诚实声明）

- **批量裁决形态**：多条 diff 的批量 accept/reject（含 CBT 构建产物批量裁决）是否引入批量编排态
  待 CBT G2 与审阅 UI 决策（DE-04）后评估，可能扩展本机或加编排机，届时回填总图。
- **超时 timeout 的驱动源**（定时器/呈现时检查/裁决时检查）属实现期调度设计，本机只锁事件语义。
- GenFailed 回报后的 AG 重试策略归 AG（新回合新 diff），本机不管理重试计数。
- 已打开非激活文档自动应用后的 DiffCard 当前只展示增量效果，缺少红删效果与独立撤销能力；后续应补
  DiffCard 失效态视觉、diff 撤销或文档级回滚能力。当前安全线是不允许二次裁决、基线偏移或 DiskState 兜底。

## §9 后续动作

1. 落地后同步编辑 A-SYS-M-S-01：§3 diff 终态行 planned→active（SM-DE-01），§5（原 §5 前）两条转移点更新为
   双侧就绪。
2. 随 `npm run governance:generate` 复跑核对。
3. 待人类宣布 DE G2 闭合；批量裁决形态随 CBT G2 / DE-04 决策后评估回填。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-11 | v1.0 | 初版（DE G2 状态机件）：定义 SM-DE-01 diff 终态机（Generating/Pending/Accepted/Rejected/Expired/GenFailed，含 APC §6 全部必备字段；三终态互斥不可逆、accepted 唯一触发落盘、ApplyError 不回退终态、保守过期）；闭合 AG 建议交接与 WS 裁决落盘两个单侧 X-INTER-001 转移点——「对话生成 diff→裁决→落盘」核心交互状态机链全链建机完毕；满足 X-SEQ-003；测试矩阵（正路/拒绝过期/生成失败/禁止转移/冲突重检）；债务（批量裁决形态待 CBT G2/DE-04、timeout 驱动源延实现） |
| 2026-07-04 | v1.1 | 术语统一清扫（承 2026-07-04 人类三轮裁决）：正文禁用别名替换——内联辅助→局部修改、自动补全→辅助续写、对话编辑→Diff Edit、Chat↔Build 接管→构建移交/回落、chat/chat-agent 回合→代理回合；文件名与变更记录历史行不改写 |
| 2026-07-08 | v1.2 | 修正 SM-DE-01 accepted 副作用：当前编辑器草稿路径 accepted 不触发 SM-WS-02，后续由 ED/WS save 落盘并复位 GreenReview；后台/整篇路径仍可 accepted→SM-WS-02 apply；rejected 对已应用草稿必须回滚 before |
| 2026-07-10 | v1.3 | 三态模型落地（A-CORE-C-T-01）：新增 §4 草稿路径 vs 后台路径选择规则（D5 决议）；后续各节编号顺移 |
| 2026-07-18 | v1.4 | 新增已打开非激活文档 `open_session` 自动应用路径：Pending 后基线校验通过即修改 ED session LogicalState 并转 accepted；失败转 expired/error 投影；禁止用户裁决和后台落盘 |
| 2026-07-20 | v1.5 | 承接 APP-CLOSE-DIFF-LIFECYCLE-01（SYS 应用关闭协调机 SM-SYS-02）：新增 `startupAbnormalExitSweep` 事件与 Pending→Expired(terminal_reason=abnormal_exit) 启动清算转移；§3 新增「关闭前批量裁决」「启动异常退出清算」两条 SYS→DE 跨模块转移点，均双侧就绪 |
