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
summary: 单条待裁决修改自生成到终态闭合的生命周期（三终态互斥不可逆，当前编辑器草稿 accepted 由 save 落盘）
source_type: design
chain: DE-DIFF
rules: BR-DE-DATA-001, BR-DE-STATE-001, BR-DE-STATE-002, BR-DE-DATA-003
initial_state: Generating
terminal_states: Accepted, Rejected, Expired, GenFailed
-->

- **machine_id**：SM-DE-01　**owner_module**：DE
- **scope**：单条待裁决修改（PendingDiff）自 ChangeProposal 交接到终态闭合
- **context**：ChangeProposal、PendingDiff（payload 含 before_snapshot）、目标 blockId 集合、
  裁决输入、外部变更检测结果、编辑器草稿应用状态、后台 apply 回执
- **states**：Generating、Pending、Accepted、Rejected、Expired、GenFailed
- **events**：genOk、genFail、userAccept、userReject、timeout、externalChange、appliedOk、applyFail
- **transitions（主表）**：

| from | event | guard | to | action |
|---|---|---|---|---|
| Generating | genOk | blockId 锚有效且 WS 持久化成功 | Pending | status=pending；经 ED 装饰层呈现审阅 |
| Generating | genFail | 锚失效/持久化失败 | GenFailed | 回报 AG（SM-AG-01 handoffFail）；不进入裁决 |
| Pending | userAccept | 当前编辑器草稿已应用 | Accepted | status=accepted（不可逆）；不触发 SM-WS-02，后续由 ED/WS save 落盘 |
| Pending | userAccept | 未应用为当前编辑器草稿 | Accepted | status=accepted（不可逆）；触发 SM-WS-02 accepted（后台 apply 路径） |
| Pending | userReject | — | Rejected | status=rejected；草稿已应用则 ED 回滚 before；否则通知 SM-WS-02 rejected（不落盘）；ED 装饰复位 |
| Pending | timeout | 超过可配置时限 | Expired | status=expired；通知 SM-WS-02 expired；ED 装饰复位 |
| Pending | externalChange | 目标块内容 ≠ before_snapshot（检测可得） | Expired | 同上（检测不可得保守保持 Pending，BR-DE-STATE-002） |
| Accepted | saved | ED/WS save 成功（草稿路径） | Accepted | 落盘闭合：ED GreenReview 复位（终态保持） |
| Accepted | appliedOk | SM-WS-02 Applied 回执（后台路径） | Accepted | 落盘闭合：ED 装饰复位（终态保持） |
| Accepted | applyFail | SM-WS-02 ApplyError 回执 | Accepted | 终态不回退；落盘重试归 WS 侧（SM-WS-02 retry） |

- **error_states**：GenFailed（终态，回报 AG 可发起新回合重试，不复用本条 diff）
- **recovery_paths**：GenFailed→AG 侧重新产出（新 diff 新实例）；草稿路径 saveFail→ED 保持脏态可重试；
  applyFail→WS 侧 retry（本机终态不变）
- **forbidden_transitions**：
  - Generating → Accepted/Rejected/Expired（未持久化呈现不得裁决）
  - Accepted ↔ Rejected ↔ Expired（三终态互斥，任意互转禁止）
  - Expired / Rejected → Accepted（不得复活已关闭 diff）
  - Pending → 文件落盘（无 userAccept 或 save/apply 不存在通往文件写入的路径）
  - Accepted → Pending（ApplyError 不得回退终态）
- **acceptance_criteria**：三终态互斥不可逆且与 status 枚举一一对应；当前编辑器草稿路径经
  userAccept→Accepted 后不直接落盘，必须等待 ED/WS save；后台 apply 路径通往 SM-WS-02 Applying
  的唯一路径仍经 userAccept→Accepted；rejected/expired 后文件不变、装饰复位；externalChange 检测
  不可得时不自动过期
- **test_matrix**：见 §4

## §3 跨模块转移契约（X-INTER-001，本文闭合双侧）

| 转移点 | 方向 | 契约 | 双侧状态 |
|---|---|---|---|
| 建议交接 | SM-AG-01 HandingOff → SM-DE-01 Generating | ChangeProposal（定位锚+修改内容+来源）交接；genOk→AG handoffOk，genFail→AG handoffFail | **双侧就绪**（AG 侧 A-AG-M-S-01 §2，本文闭合） |
| 草稿确认保存 | SM-DE-01 Accepted → SM-ED-01 Editing → SM-WS-01 save | 当前编辑器草稿路径 accepted 只确认，文件写入由用户 save 触发；Saved 后 GreenReview 复位 | **双侧就绪**（ED/WS save 侧已就绪） |
| 后台裁决落盘 | SM-DE-01 Accepted → SM-WS-02 accepted | 未应用为当前编辑器草稿的后台/整篇 diff 可由 accepted 触发 apply；Applied/ApplyError 回执回传本机（终态不回退） | **双侧就绪**（WS 侧 A-WS-M-S-01 §3，本文闭合） |
| 不落盘关闭 | SM-DE-01 Rejected/Expired → SM-WS-02 rejected/expired | 驱动 SM-WS-02 → Discarded（不写文件） | **双侧就绪** |
| 审阅呈现 | SM-DE-01 ↔ ED 装饰层 | Pending 呈现/终态复位；裁决交互经 ED 转达（X-FORBID-005）；装饰为本机投影不独立建机（A-ED-M-S-01 §4 豁免成立） | 双侧就绪 |

至此「对话生成 diff→人类裁决→草稿/保存或后台 apply」核心交互的状态机链 **AG→DE→ED/WS**，
两侧一致性在 G5 跨模块联调验证（APC §5.1）。

## §4 草稿路径 vs 后台路径选择规则（A-CORE-C-T-01 §5.5 · D5 决议落地）

SM-DE-01 `Pending→Accepted` 转移后，accepted 信号的处理路径有两支。选择规则如下：

| 条件 | 路径 | 三态语义 | 代码入口 | 触发落盘 |
|---|---|---|---|---|
| `DiffView.draftApplied === true`（after 已应用到当前编辑器草稿） | **草稿路径** | LogicalCandidate → LogicalState（已扩散入 ProseMirror doc），仅确认裁决态 | `de.accept_draft_diff`（[deService.ts:L73](file:///Users/imatstarbucks/binder/src/services/deService.ts#L73)）| 等待用户 ⌘S → `save_atomic` |
| `draftApplied` 为 falsy + `isDocxPath(relPath) === false` | **后台路径** | LogicalCandidate → LogicalState + 同步触发 DiskState 写入 | `de.accept_diff` → `ws.apply_diff` → `save_atomic`（[deService.ts:L82](file:///Users/imatstarbucks/binder/src/services/deService.ts#L82)）| accepted 时立即触发 |
| `draftApplied` 为 falsy + `isDocxPath(relPath) === true` | **拒绝（报错）** | BDocx 必须先进编辑器草稿，不支持后台 apply | `deStore.setLocalView("error", "DE_BDOCX_DRAFT_REQUIRED")`（[deService.ts:L78](file:///Users/imatstarbucks/binder/src/services/deService.ts#L78)）| 无 |
| `patchKind === "replace_document"`（整篇替换） | **草稿路径** | 整篇内容替换 → LogicalState | `setActiveDocumentContent`（[Editor.tsx:L300](file:///Users/imatstarbucks/binder/src/components/ed/Editor.tsx#L300)）| 等待用户 ⌘S |
| `patchKind === "style_ops"`（纯样式操作）| **草稿路径** | StyleOp 经受控接口进入编辑模型 → LogicalState | `applyStyleOps`（[Editor.tsx:L309](file:///Users/imatstarbucks/binder/src/components/ed/Editor.tsx#L309)）| 等待用户 ⌘S |

> 所有路径的裁决入口统一：均经 `Pending → userAccept → Accepted`（SM-DE-01 transition）。
> 分歧发生在 Accepted 后的 side-effect 层，不在状态机转移层。deService.ts 是路径选择的编排方。

## §5 测试与验收（强制转移测试矩阵，APC §6/§10）

- **正路**：Generating→genOk→Pending→ED 草稿应用→userAccept→Accepted→saved（save 落盘 + 复位）；
  后台路径为 Generating→genOk→Pending→userAccept→Accepted→appliedOk。
- **拒绝/过期路**：userReject→Rejected、timeout/externalChange→Expired，断言文件不变 + 装饰复位 +
  SM-WS-02 达 Discarded。
- **生成失败路**：genFail→GenFailed 回报 AG，不进入裁决、无持久化残留。
- **禁止转移**：三终态互转不可达；Expired/Rejected 不可复活；无 userAccept 不存在通往保存/apply 路径；
  ApplyError 后终态保持 Accepted。
- **保守过期**：externalChange 检测不可得时保持 Pending（不误杀）。
- **冲突重检**（BR-DE-DATA-003）：先序 diff 落盘后，重叠 pending diff 触发 externalChange 重检转
  Expired；不重叠者不受影响。
- **跨模块（G5 联调）**：AG↔DE 交接回执、DE↔ED 草稿应用/回滚/呈现复位、DE↔WS save 或后台 apply
  回执三链双侧一致。

## §6 总图登记（A-SYS-M-S-01）

| 核心链 | machine_id | 总图回填动作 |
|---|---|---|
| diff 终态 | SM-DE-01 | §3 行 planned→active（A-DE-M-S-01 §2） |
| 转移点 | — | §5（原 §5 前）「diff accepted→落盘」「AG 回合门禁与产出」两条更新为双侧就绪 |

## §7 术语归属说明

PendingDiff / ChangeProposal 已在 A-DE-M-T-01 §0 注册（主责 DE）；内容块/装饰层归 ED、
Workspace/时间轴归 WS、代理回合归 AG，本文引用不抢注；状态机框架词不注册。

## §8 技术债务与盲区（诚实声明）

- **批量裁决形态**：多条 diff 的批量 accept/reject（含 CBT 构建产物批量裁决）是否引入批量编排态
  待 CBT G2 与审阅 UI 决策（DE-04）后评估，可能扩展本机或加编排机，届时回填总图。
- **超时 timeout 的驱动源**（定时器/呈现时检查/裁决时检查）属实现期调度设计，本机只锁事件语义。
- GenFailed 回报后的 AG 重试策略归 AG（新回合新 diff），本机不管理重试计数。

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
