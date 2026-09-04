---
文档编号：DE-M-S-01
文档状态：A
负责模块：DE
文档职责：DE Diff Edit 模块级状态机设计——ProposalBatch 生成与逐卡 diff 终态机（SM-DE-01），含 APC §6 必备字段；对接 A-SYS-M-S-01 总图、SM-AG-02 propose_edit 工具入口与 SM-WS-02/SM-ED-01 三分流
上游约束：A-DE-M-T-01_DiffEdit技术设计.md（DE-DIFF 链路、BR-DE-* 规则）、A-CORE-C-P-01_APC.md（§6）、A-SYS-I-P-01_数据结构定义.md（status 枚举集合）、A-SYS-M-S-01_状态机总图.md（X-SEQ-003/X-INTER-001）、A-WS-M-S-01_工作台状态机设计.md（SM-WS-02 accepted/rejected/expired 事件入口）、A-AG-M-S-01_代理状态机设计.md（SM-AG-02 propose_edit 工具入口）、A-ED-M-S-01_编辑器状态机设计.md（装饰呈现依附说明）、ADU.md（§3.1）
直接承接：SM-DE-01 经 rules 字段承载 A-DE-M-T-01 规则；machine_id 登记总图；裁决信号驱动 SM-WS-02/SM-ED-01，生成入口承接 SM-AG-02 的 propose_edit 工具执行
使用边界：本文定义 DE 状态机内部；不重定义 SM-WS-02/SM-AG-01 内部；DE 机状态多于 status 枚举（含 Generating/GenFailed 过程态），枚举集合语义对齐 A-SYS-I-P-01 不漂移；审阅 UI 形态不在本文（DE-04）
变更要求：transitions 变更须同步 A-DE-M-T-01 规则并重跑测试矩阵；与 WS/AG/ED 的转移点变更须对侧共同复核；machine_id 一经登记不可更改
治理成熟度：draft
---

# Binder Diff Edit 状态机设计（DE · L2 技术设计）

## §1 文档定位

```
A-DE-M-T-01（规则 SSOT：DE-DIFF）→ 本文 SM-DE-01
本文 machine_id ──登记──→ A-SYS-M-S-01 §3（diff 终态 active）
入口：SM-AG-02 ToolExecuting(propose_edit)（ChangeProposal 交接）
出口：已有 LogicalState → SM-ED-01 预览/裁决/保存；无 LogicalState → SM-WS-02 accepted/rejected/expired
```

满足 X-SEQ-003。本机状态集合多于 PendingDiff.status 枚举（增 Generating/GenFailed 过程态）；
枚举值 pending/accepted/rejected/expired 与 A-SYS-I-P-01 单一来源严格对齐（Pending 态 ↔
status=pending，三裁决终态 ↔ 对应枚举值），无第三处定义。

## §2 SM-DE-01 · diff 终态机

<!-- STATE_MACHINE
machine_id: SM-DE-01
owner_module: DE
status: active
summary: 单条待裁决修改自生成到终态闭合的生命周期（三终态互斥不可逆；按 LogicalState 存在性在预览保存与接受后台落盘间分流）
source_type: design
chain: DE-DIFF
rules: BR-DE-DATA-001, BR-DE-STATE-001, BR-DE-STATE-002, BR-DE-DATA-003, BR-CORE-STATE-001, BR-CORE-STATE-002, BR-CORE-STATE-003
initial_state: Generating
terminal_states: Accepted, Rejected, Expired, GenFailed
-->

- **machine_id**：SM-DE-01　**owner_module**：DE
- **scope**：ProposalBatch 中单条待裁决修改（PendingDiff）自分组生成到独立终态闭合；批次不共享终态
- **context**：ProposalBatch 身份/group_index/group_count、同文档批次共享基线、当前组 ChangeProposal 投影、PendingDiff（payload 含 before_snapshot）、本卡目标 blockId/fingerprint 集合、
  DiffProjection（draft_mode/snapshot_source/base_session_version/base_content_hash）、裁决输入、
  外部变更检测结果、编辑器草稿预览状态、已打开非激活文档预览状态、后台 apply 回执
- **states**：Generating、Pending、Accepted、Rejected、Expired、GenFailed
- **events**：genOk、genFail、previewApplied、previewFail、logicalStateOpened、userAccept、userSave、userReject、timeout、externalChange、appliedOk、applyFail、startupAbnormalExitSweep
- **transitions（主表）**：

| from | event | guard | to | action |
|---|---|---|---|---|
| Generating | genOk | ProposalBatch 全组锚/基线/事务校验通过，批次和全部 PendingDiff 已原子持久化 | Pending | 本卡 status=pending；按 group_index 经 ED 装饰层呈现审阅 |
| Generating | genFail | 任一组锚/基线/事务校验失败或整批持久化失败 | GenFailed | 同批全部实例失败；向 SM-AG-02 返回结构化失败 ToolResult；不进入裁决且无部分残留 |
| Pending | previewApplied | 单卡预览成功，或同文档同批结构化多卡已按共享基线聚合预检并由 ED 一次事务成功 | Pending | after 内容进入 LogicalState、置脏并显示 GreenReview；批内每卡保持独立 pending 与自身目标 guard |
| Pending | previewFail | 单卡预览失败，或同文档同批结构化事务基线不匹配/事务不支持/应用失败 | Expired | 单卡或同一文档批次 cohort 全部置 expired；LogicalState 无半应用；不触发 SM-WS-02 |
| Pending | logicalStateOpened | 原为无 LogicalState 的后台目标在 pending 期间被打开，且同一 DiffPayload/基准可投影 | Pending | ED 建立 LogicalState、应用 after 并显示 GreenReview；取消后台路径，卡片仍可接受/拒绝 |
| Pending | userAccept | 目标已有 LogicalState 且本卡全部目标仍匹配各自 after fingerprint | Accepted | status=accepted（不可逆）；不触发 SM-WS-02，后续仅由用户 save 落盘；同批其他卡不变 |
| Pending | userSave | 目标已有 LogicalState 且本卡全部目标仍匹配各自 after fingerprint | Accepted | 保存即接受：逐卡置 accepted/awaiting_save，再经 SM-WS-01 写盘；成功回 saved，失败保留 awaiting_save |
| Pending | userAccept / userSave | DiffCard 显式接受或保存时发现本卡任一目标已偏离 after fingerprint | Expired | 只使本卡失效；保留当前 LogicalState并标记 `document_revision_advanced`，保存当前内容仍可继续 |
| Pending | userAccept | draft_mode=background 且目标文档未打开 | Accepted | status=accepted（不可逆）；触发 SM-WS-02 accepted（后台 apply 路径） |
| Pending | userReject | 目标已有 LogicalState，且本卡全部目标仍匹配各自 after fingerprint | Rejected | status=rejected；只以本卡 before/反向操作单事务恢复本卡目标；同批其他目标保持，ED 装饰复位，不落盘 |
| Pending | userReject | 目标无 LogicalState | Rejected | status=rejected；解除 DiffCard 临时挂载，不写文件 |
| Pending | userReject | block_ops 当前节点与 afterNode fingerprint 不匹配 | Pending（视觉 conflict） | 不提交不可逆 reject；保留当前 LogicalState，提示目标已被后置编辑 |
| Pending | timeout | 超过可配置时限 | Expired | status=expired；通知 SM-WS-02 expired；ED 装饰复位 |
| Pending | externalChange | 预览前目标 ≠ before_snapshot，或预览后本卡目标 ≠ after fingerprint（检测可得） | Expired | 只使受影响卡失效；检测不可得保守保持 Pending（BR-DE-STATE-002） |
| Accepted | saved | ED/WS save 成功（草稿路径） | Accepted | 落盘闭合：ED GreenReview 复位（终态保持） |
| Accepted | appliedOk | SM-WS-02 Applied 回执（后台路径） | Accepted | 落盘闭合：ED 装饰复位（终态保持） |
| Accepted | applyFail | SM-WS-02 ApplyError 回执 | Accepted | 终态不回退；落盘重试归 WS 侧（SM-WS-02 retry） |
| Pending | startupAbnormalExitSweep | 应用启动清算发现该 diff 所属 workspace 存在未清零残留（BR-SYS-STATE-005，SM-SYS-02 驱动） | Expired | status=expired，terminal_reason=abnormal_exit；先于会话/历史恢复执行，不进入裁决投影 |

- **error_states**：GenFailed（本次 ProposalBatch 生成终态；不强制终结 AgentTurn，不复用本批身份）
- **recovery_paths**：GenFailed→模型收到失败 ToolResult 后可在同一 AgentTurn 上调整并提交新批次，或用户新回合重试；草稿路径 saveFail→ED 保持脏态可重试；
  applyFail→WS 侧 retry（本机终态不变）
- **forbidden_transitions**：
  - Generating → Accepted/Rejected/Expired（未持久化呈现不得裁决）
  - Accepted ↔ Rejected ↔ Expired（三终态互斥，任意互转禁止）
  - Expired / Rejected → Accepted（不得复活已关闭 diff）
  - Pending → 文件落盘（无 LogicalState 时仅 userAccept→SM-WS-02；已有 LogicalState 时仅用户 save 可写盘）
  - Pending → SM-WS-02 accepted，当目标文档已打开（已打开文档只改 LogicalState，不后台写 DiskState）
  - Accepted → Pending（ApplyError 不得回退终态）
  - 同一 ProposalBatch 的一张卡终态 → 自动改变其他卡状态（批次仅关联，不共享裁决）
  - 同文档同批结构化多卡 → 按卡依次推进整篇基准（必须共享基线并聚合为一次 ED 前向事务）
- **acceptance_criteria**：三终态互斥不可逆且与 status 枚举一一对应；已有 LogicalState 时 previewApplied
或 logicalStateOpened 只改 LogicalState，DiffCard 显式接受只裁决、用户 save 对有效 pending 预览同时裁决并落盘；无 LogicalState 的后台
路径通往 SM-WS-02 Applying 的唯一路径经 userAccept→Accepted；pending 后打开必须不可再走后台 apply；
rejected/expired 后文件不变、装饰复位；externalChange 检测不可得时不自动过期；不连续或可独立裁决块
形成独立实例，相邻不可分块可同实例；同文档同批结构化多卡一次预览且批内不互相失效，任一卡反向只
改变自身目标，同批卡片可分别进入不同终态
- **test_matrix**：见 §4

### 2.1 连续 Proposal 接力契约（REQ-DE-009 / BR-SYS-STATE-008）

承接 REQ-DE-009（A-DE-M-D-01）与冻结基线 R5-B：同一 AgentTurn 或复杂任务投影中明确的同文档连续编辑
（Diff1→Diff2→Diff3…），SM-DE-01 按以下契约处理，**不新增补丁栈或任务级回滚**：

| 场景 | SM-DE-01 语义 | 承接方 |
|---|---|---|
| 基准真值 | Diff2/3 的 before/base 必须来自上一张**成功预览后的当前 LogicalState**（`session_version` + `content_hash`），不是前卡 payload 叠加 | DE（生成前校验） |
| 前卡失效 | 后续 Proposal 成功预览推进基准 → 基于旧基准的既有批次卡转 `Expired`（`terminal_reason=document_revision_advanced`），保留历史、接受/拒绝禁用；当前 ProposalBatch 的兄弟卡共享本次预览，不属于“前卡”；`nextProposalPreviewApplied` 等价于跨批次 revision-advanced 检测 | SM-DE-01（Pending→Expired） |
| 非补丁栈 | 连续卡不构成可叠加补丁栈；拒绝最新卡只回到其自身有效基准，**不产生任务级回滚** | SM-DE-01（userReject guard） |
| background 限制 | 无 LogicalState 的 background pending 卡必须先迁移（BR-CORE-STATE-003）或完成背景分流裁决（BR-CORE-STATE-002），**不得作为后续编辑输入** | SM-DE-01 / SM-ED-01 / SM-WS-02 |
| 任务投影关联 | 每张卡保留 turn_id 与可选 task_run_id 供 Trace 与事件投影；关联**不授予**叠加、续编、自动继续或回滚能力 | AG（事件投影）、DE（卡片关联） |
| 保存语义 | 连续接力中保存当前 LogicalState 仍按既有规则：仍有效 pending 卡先裁决为 accepted/awaiting_save 再落盘；已失效卡不得被误裁决 | SM-WS-01 协调（A-WS-M-S-01 §2.1） |

<!-- REQ_MAP
req_id: REQ-DE-009
target_type: RULE
target_id: BR-SYS-STATE-008
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-DE-009
target_type: RULE
target_id: BR-CORE-STATE-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-DE-009
target_type: RULE
target_id: BR-CORE-STATE-003
status: mapped
-->

> 后续 Proposal 的 `previewApplied` 是前卡 revision-advanced 检测源：运行时按 rel_path 与 base
> sessionVersion/contentHash 找到基准已落后的旧 PendingDiff，追加 `document_revision_advanced` 失效事件；
> 同一旧基准并行生成但尚未推进 LogicalState 的卡不能仅因创建顺序失效；同一 ProposalBatch 的结构化
> 兄弟卡由一次聚合预览共同推进，必须按 proposal_batch_id 排除在彼此失效检查之外。不得依赖旧 payload 续编或补丁叠加。

### 2.2 WorkspaceOperation 与 CreateDocumentBundle 边界

SM-DE-01 只承载文档内容 Diff。创建、重命名、移动、删除和恢复不得进入本机，不得伪造
PendingDiff、空 before/after 投影或资源 DiffCard。这些状态与副作用唯一归 A-WS-M-S-02 SM-WS-04。

CreateDocumentBundle 只在 WorkspaceOperation 已成功创建真实目标后，才向 SM-DE-01 提交一个仅含普通初始内容 Diff 的 ProposalBatch。
该 Diff 的接受、失效和保存仍按本机处理；拒绝触发的安全删除是 SM-WS-04 的类型化补偿，
必须校验 operation_id、资源身份、DiskState 与 LogicalState 三重基线，不是 SM-DE-01 新终态。

## §3 跨模块转移契约（X-INTER-001，本文闭合双侧）

| 转移点 | 方向 | 契约 | 双侧状态 |
|---|---|---|---|
| 建议交接 | SM-AG-02 ToolExecuting(propose_edit) → SM-DE-01 Generating | ChangeProposal 先合成同块操作，再按可独立裁决连续块组形成 ProposalBatch；全组原子持久化后 fan-out 为多个 SM-DE-01 实例；成功返回 ToolResult + DiffProposed，任一失败返回结构化失败 ToolResult 且整批零残留 | **双侧就绪**（AG 侧 A-AG-M-S-01 §3，本文闭合） |
| 草稿确认保存 | SM-DE-01 Pending/Accepted → SM-ED-01 Editing → SM-WS-01 save | 已有 LogicalState 路径中，显式接受只裁决；保存有效 Pending 预览会先转 Accepted/awaiting_save，再写文件；Saved 后 GreenReview 复位 | **双侧就绪**（ED/WS save 侧已就绪） |
| 未打开文档后台裁决落盘 | SM-DE-01 Accepted → SM-WS-02 accepted | 目标文档未打开的后台/整篇 diff 可由 accepted 触发 apply；Applied/ApplyError 回执回传本机（终态不回退） | **双侧就绪**（WS 侧 A-WS-M-S-01 §3，本文闭合） |
| 已有或后打开文档预览 | SM-DE-01 Pending → SM-ED-01 Editing → Pending | 已有 LogicalState 时直接预览；后台 pending 在用户打开时以同一载荷建立 LogicalState 并显示绿色增量；两者均不触发 SM-WS-02、仍允许用户裁决 | **双侧就绪**（ED/DE 按 BR-CORE-STATE-001/003） |
| 不落盘关闭 | SM-DE-01 Rejected/Expired → SM-WS-02 rejected/expired | 驱动 SM-WS-02 → Discarded（不写文件） | **双侧就绪** |
| 审阅呈现 | SM-DE-01 ↔ ED 装饰层 | Pending 呈现/终态复位；裁决交互经 ED 转达（X-FORBID-005）；装饰为本机投影不独立建机（A-ED-M-S-01 §4 豁免成立） | 双侧就绪 |
| 关闭前批量裁决 | SM-SYS-02 Resolving → SM-DE-01（userAccept/userReject 批量） | 应用关闭决策入口批量调用既有 `de.accept_diff`/`de.reject_diff`/`de.accept_draft_diff`，本机不新增事件，仅为既有转移的批量调用方（BR-SYS-STATE-004） | **双侧就绪**（SYS 侧 A-SYS-M-S-02 §3，2026-07-20 新增） |
| 启动异常退出清算 | SM-SYS-02（启动期）→ SM-DE-01 Pending → Expired | 应用启动、先于会话/历史恢复执行；workspace 内残留 `status=pending` 一律转 `expired`，`terminal_reason=abnormal_exit`，不进入裁决投影（BR-SYS-STATE-005） | **双侧就绪**（SYS 侧 A-SYS-M-S-02 §4，2026-07-20 新增） |

至此「对话生成 diff→已有 LogicalState 预览/保存，或无 LogicalState 卡片挂载/接受落盘；pending 后打开迁移」核心交互的状态机链 **AG→DE→ED/WS**，
两侧一致性在 G5 跨模块联调验证（APC §5.1）。应用关闭前后的批量裁决与异常退出清算属应用级编排，
两个新增转移点归 **SYS→DE**（A-SYS-M-S-02，2026-07-20 新增），一致性同样在 G5 验证。

## §4 三路径选择规则（A-CORE-C-T-01 §5 · D5 与 2026-07-18 决议落地）

SM-DE-01 生成 PendingDiff 后，根据 DiffProjection 的 `draft_mode` 分三支处理。选择规则如下：

| 条件 | 路径 | 三态语义 | 代码入口 | 触发落盘 |
|---|---|---|---|---|
| `draft_mode=current_editor` 或 `open_session` | **已有 LogicalState 路径** | LogicalCandidate → LogicalState（GreenReview）；卡片仍 pending，显式接受只裁决，保存有效 pending 预览则一并裁决 | ED 受控 apply → `de.accept_draft_diff` | 用户 save 后触发 |
| `draft_mode=background` 且目标文档未打开 + `isDocxPath(relPath) === false` | **无 LogicalState 后台路径** | LogicalCandidate 只临时挂载于卡片 | `de.accept_diff` → `ws.apply_diff` → `save_atomic` | accepted 时立即触发 |
| `draft_mode=background` 且目标文档在 pending 期间打开 | **打开迁移路径** | LogicalCandidate → LogicalState（GreenReview）；卡片仍 pending，显式接受只裁决，保存有效 pending 预览则一并裁决 | ED 建立会话并投影 → `de.accept_draft_diff` | 用户 save 后触发 |
| `draft_mode=background` + `isDocxPath(relPath) === true` | **拒绝（报错）** | BDocx 必须进入打开文档 LogicalState，不支持后台 raw apply | `deStore.setLocalView("error", "DE_BDOCX_DRAFT_REQUIRED")` | 无 |

> 三条路径均由同一张 DiffCard 呈现；所有已有 LogicalState 路径都提供用户裁决入口。`open_session` 表示在生成时已存在的会话，`background` 在用户打开目标文档后必须迁移，禁止继续后台接受落盘。

## §5 测试与验收（强制转移测试矩阵，APC §6/§10）

- **正路**：ProposalBatch 全组预检/原子持久化→每组 Generating→genOk→Pending→ED 草稿应用→用户 DiffCard 接受→Accepted→用户 save→saved（save 落盘 + 复位）；
  已打开非激活路径与 pending 后打开迁移均为 Generating→genOk→Pending→受控预览/迁移→Pending，再由用户
  接受或拒绝裁决；未打开后台路径为 Generating→genOk→Pending→userAccept→Accepted→appliedOk。
- **拒绝/过期路**：userReject→Rejected、timeout/externalChange→Expired，断言文件不变 + 装饰复位 +
  SM-WS-02 达 Discarded。
- **生成失败路**：genFail→GenFailed 回报 SM-AG-02 失败 ToolResult，不进入裁决、无持久化残留；模型可在同回合调整，不把单次生成失败伪装成 AgentTurn 完成。
- **批次独立裁决**：同块多操作先合成；不连续/可独立块 fan-out 为多实例；相邻不可分组可单实例；同文档
  同批结构化多卡共享基线、聚合为一次前向事务且不互相触发 `document_revision_advanced`；拒绝一张只恢复
  本卡目标，另一张仍可接受。Accepted/Rejected/Pending 混合终态合法；任一生成组失败则整批零持久化。
- **禁止转移**：三终态互转不可达；Expired/Rejected 不可复活；保存只可裁决当前 after 基准仍有效的 Pending，不得接受已失效卡；
  无 LogicalState 的后台路径无 userAccept 不存在 apply 路径；ApplyError 后终态保持 Accepted；已有或迁移
  LogicalState 路径不存在后台 apply。
- **保守过期**：externalChange 检测不可得时保持 Pending（不误杀）。
- **冲突重检**（BR-DE-DATA-003）：先序 diff 落盘后，重叠 pending diff 触发 externalChange 重检转
  Expired；不重叠者不受影响。
- **跨模块（G5 联调）**：AG↔DE 交接回执、DE↔ED 草稿应用/回滚/呈现复位、DE↔WS save 或后台 apply
  回执三链双侧一致。

## §6 总图登记（A-SYS-M-S-01）

| 核心链 | machine_id | 总图回填动作 |
|---|---|---|
| diff 终态 | SM-DE-01 | §3 active 登记（A-DE-M-S-01 §2） |
| 转移点 | — | §5（原 §5 前）「diff accepted→LogicalState/落盘」「AG 回合门禁与产出」两条更新为双侧就绪 |

## §7 术语归属说明

PendingDiff / ChangeProposal 已在 A-DE-M-T-01 §0 注册（主责 DE）；内容块/装饰层归 ED、
Workspace/时间轴归 WS、代理回合归 AG，本文引用不抢注；状态机框架词不注册。

## §8 技术债务与盲区（诚实声明）

- **批量裁决 UI**：运行语义已固定为把批量动作展开成多个既有逐卡事件，不引入批次状态机；具体布局
  可继续优化，但不得以 UI 便利合并 PendingDiff 终态。
- **超时 timeout 的驱动源**（定时器/呈现时检查/裁决时检查）属实现期调度设计，本机只锁事件语义。
- GenFailed 回报后的 AG 重试策略归 AG（新回合新 diff），本机不管理重试计数。
- 已打开非激活或迁移文档预览后的 DiffCard 可能只展示增量、缺少完整红删效果；后续应补 DiffCard
  失效态视觉与更长期的文档级回滚能力。当前安全线是保留接受/拒绝（拒绝精确回滚）、禁止二次内容应用、
  防止基线偏移和后台 DiskState 兜底。

## §9 维护与实现入口

1. 状态或跨模块转移变化后同步 A-SYS-M-S-01 与双方接口，并运行 `npm run governance:generate`。
2. ProposalBatch 不新增共享终态机；每个 PendingDiff 独立进入 SM-DE-01，批量操作只调用多次逐卡事件。
3. 未来 CBT Profile 若需要不同批次语义，须重新进入需求/设计，不得复用历史登记隐式扩展。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-09-03 | v1.11 | AG-ADAPTIVE-RUNTIME-DESIGN-01：SM-DE-01 明确为 ProposalBatch fan-out 后的逐卡实例；全组预检与原子持久化；同文档同批结构化多卡共享基线、一次 ED 前向事务并以目标 fingerprint 独立裁决，批内不互相失效。 |
| 2026-06-11 | v1.0 | 初版（DE G2 状态机件）：定义 SM-DE-01 diff 终态机（Generating/Pending/Accepted/Rejected/Expired/GenFailed，含 APC §6 全部必备字段；三终态互斥不可逆、accepted 唯一触发落盘、ApplyError 不回退终态、保守过期）；闭合 AG 建议交接与 WS 裁决落盘两个单侧 X-INTER-001 转移点——「对话生成 diff→裁决→落盘」核心交互状态机链全链建机完毕；满足 X-SEQ-003；测试矩阵（正路/拒绝过期/生成失败/禁止转移/冲突重检）；债务（批量裁决形态待 CBT G2/DE-04、timeout 驱动源延实现） |
| 2026-07-04 | v1.1 | 术语统一清扫（承 2026-07-04 人类三轮裁决）：正文禁用别名替换——内联辅助→局部修改、自动补全→辅助续写、对话编辑→Diff Edit、Chat↔Build 接管→构建移交/回落、chat/chat-agent 回合→代理回合；文件名与变更记录历史行不改写 |
| 2026-07-08 | v1.2 | 修正 SM-DE-01 accepted 副作用：当前编辑器草稿路径 accepted 不触发 SM-WS-02，后续由 ED/WS save 落盘并复位 GreenReview；后台/整篇路径仍可 accepted→SM-WS-02 apply；rejected 对已应用草稿必须回滚 before |
| 2026-07-10 | v1.3 | 三态模型落地（A-CORE-C-T-01）：新增 §4 草稿路径 vs 后台路径选择规则（D5 决议）；后续各节编号顺移 |
| 2026-07-18 | v1.4 | 新增已打开非激活文档 `open_session` 自动应用路径：Pending 后基线校验通过即修改 ED session LogicalState 并转 accepted；失败转 expired/error 投影；禁止用户裁决和后台落盘 |
| 2026-07-20 | v1.5 | 承接 APP-CLOSE-DIFF-LIFECYCLE-01（SYS 应用关闭协调机 SM-SYS-02）：新增 `startupAbnormalExitSweep` 事件与 Pending→Expired(terminal_reason=abnormal_exit) 启动清算转移；§3 新增「关闭前批量裁决」「启动异常退出清算」两条 SYS→DE 跨模块转移点，均双侧就绪 |
| 2026-07-27 | v1.6 | DE-SAVE-IMPLICIT-ACCEPT-01：保存即接受已应用草稿 Diff；保存动作在写盘前映射为既有 userAccept→Accepted，保存成功后进入 saved 效果态；显式 DiffCard 接受仍保留。 |
| 2026-07-30 | v1.7 | AG-RICH-BLOCK-STRUCTURED-EDIT-01：userReject 增 block_ops afterNode fingerprint 前置 guard；匹配则 exact node 反向后进入 Rejected，不匹配保持后端 Pending 并以 conflict 视觉态阻止覆盖用户后置编辑。 |
| 2026-07-30 | v1.8 | AG-RICH-BLOCK-STRUCTURED-EDIT-01 闭合补强：DiffCard 显式接受 block_ops 同样以前态 afterNode fingerprint 为门禁；后置编辑时保持 Pending/conflict。保存动作继续遵循已确认的“保存即接受”，不改变产品语义。 |
| 2026-08-13 | v1.9 | DIFF-STATE-COMPASS-01：SM-DE-01 三分流与强制矩阵同步：已有或迁移 LogicalState 的预览保持 Pending，接受/拒绝仍可操作；保存不再映射 userAccept；仅持续无 LogicalState 的后台接受进入 SM-WS-02。 |
| 2026-08-13 | v1.10 | 人类复核：保存有效 pending 预览应映射为 `userSave → Accepted/awaiting_save → save`；只有基线推进时 `userSave → Expired`，随后继续保存当前 LogicalState。 |
| 2026-08-14 | v1.11 | AG-1.0-D2-DE-ED-WS-STATEMACHINE-01：新增 §2.1 连续 Proposal 接力契约（REQ-DE-009 / BR-SYS-STATE-008）——Diff2/3 以最新 LogicalState 为基准、后预览推进基准使前卡 Expired(document_revision_advanced)、非补丁栈/任务级回滚、background pending 先迁移或裁决；补 REQ-DE-009 的 REQ_MAP（BR-SYS-STATE-008、BR-CORE-STATE-001/003）。 |
| 2026-08-14 | v1.12 | AG-1.0-IMP-CONSECUTIVE-PROPOSAL-01：§2.1 契约落地——前端 deService.expireStalePendingForPath（同文档旧 pending 卡按 base 版本/哈希比较失效 document_revision_advanced，并行同基准卡不失效、background 卡不参与），在 applyOpenSessionDiff 成功预览后与 diff_proposed 新卡加入后触发；deRuntime +4 用例；BR-SYS-STATE-008 进入代码 @GOV 引用与测试 covers。 |
| 2026-08-21 | v1.13 | WorkspaceOperation 基线同步：新增 §2.2，锁定资源操作不得进入 SM-DE-01；CreateDocumentBundle 只在磁盘目标创建后投影普通初始内容 Diff，安全拒绝删除归 SM-WS-04 补偿。 |
