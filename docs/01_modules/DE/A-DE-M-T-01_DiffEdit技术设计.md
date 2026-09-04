---
文档编号：DE-M-T-01
文档状态：A
负责模块：DE
文档职责：DE Diff Edit 模块级 L2 技术设计（规则 SSOT）——diff 生成与定位、payload 载荷形状、终态裁决语义（accepted/rejected/expired 互斥不可逆）、过期与外部变更检测、多 diff 并存与冲突、裁决粒度；承接 REQ-DE-001~005
上游约束：A-DE-M-D-01_DiffEdit功能需求描述.md（REQ-DE-001~005、DE-01~05）、A-CORE-C-P-01_APC.md（§4.1、§6）、A-SYS-I-P-01_数据结构定义.md（PendingDiff 字段与 status 枚举集合单一来源、payload_ref 形状归 DE、X-SHARED-001）、A-WS-M-T-01_工作台技术设计.md（BR-WS-STATE-001 仅 accepted 落盘）、A-WS-M-S-01_工作台状态机设计.md（SM-WS-02）、A-ED-M-T-01_编辑器技术设计.md（BR-ED-DATA-001 blockId、ED-DECOR、X-FORBID-005）、A-AG-M-T-01_代理技术设计.md（X-FORBID-006、修改建议 ChangeProposal 交接）、ADU.md（§3）
直接承接：本文 RULE/CHAIN/TERM 为 DE 规则 SSOT，代码经 @GOV 映射；SM-DE-01 在 A-DE-M-S-01 定义并登记总图（闭合 SM-WS-02 / SM-AG-01 单侧转移点）；CBT 构建产物的 diff 化契约在本文规则上承接（联合 CBT G2）
使用边界：本文是 DE 规则 SSOT；不重定义 PendingDiff 字段与 status 枚举集合（归 SYS A-SYS-I-P-01，本文承接转移语义与 payload 形状）、不做持久化与落盘（归 WS）、不做呈现承载（归 ED 装饰层，DE 为消费方）、不做 AI 产出生成（归 AG）；审阅 UI 形态（DE-04）延实现期人类确认；状态机内部归 A-DE-M-S-01
变更要求：终态语义变更须先经 APC 授权（产品核心交互）；payload 形状变更须版本化并复核 WS（持久化）/ED（呈现重建）；status 枚举集合变更回写 A-SYS-I-P-01；TERM 一经 term_ref 引用其 en 不可更改
治理成熟度：draft
---

# Binder Diff Edit 技术设计（DE · L2 技术设计）

## 0. Terminology Registry

> DE 主责核心业务概念注册（A-SYS-I-P-01 §6 已将 diff 终态 TERM 主责显式延至 DE，本文据此注册解除）。

<!-- TERM
term_id: TERM-DE-001
chains: DE-DIFF
zh: 待裁决修改
en: PendingDiff
forbidden: 修改提案, 变更请求, 待定diff, 修改单, 待审修改
-->
由 AG 修改建议生成、经 WS 持久化、等待裁决的修改单元（共享结构归 A-SYS-I-P-01）；status 枚举
pending/accepted/rejected/expired，终态互斥不可逆。其副作用严格按 BR-CORE-STATE-001~003 分流：已有
LogicalState 时 pending 预览直接应用内容但保持可裁决；无 LogicalState 时只作为 DiffCard 挂载；pending
期间打开目标文档则建立带绿色增量的 LogicalState 并迁移为前者。只有无 LogicalState 分支的 accepted
才进入 WS apply。

<!-- TERM
term_id: TERM-DE-002
chains: DE-DIFF
zh: 修改建议
en: ChangeProposal
forbidden: AI建议, 修改意见, 编辑建议, 产出建议
-->
AG 代理回合中 `propose_edit` 工具交接 DE 的修改意图载体（目标定位 + 修改内容 + 来源元数据），是
SM-AG-02 ToolExecuting 与 DE 生成（SM-DE-01 Generating）之间的交接物；经 DE 生成为待裁决修改后进入裁决。

<!-- TERM
term_id: TERM-DE-004
chains: DE-DIFF
zh: 修改载荷
en: DiffPayload
forbidden: diff载荷, 内容载荷, payload, diff内容, 落盘载荷, 提案载荷
-->
ChangeProposal 经 DE 组装成的**统一持久化载荷**（target + after_content + before_snapshot +
is_new_doc），是后台 apply 或编辑器草稿重建时消费的内容来源。与 ChangeProposal（AG 交接的意图
载体）、PendingDiff（等待裁决的控制态记录）、DiffCard（对话区视图卡）严格区分：DiffPayload 是
**落盘内容**的单一形状（BR-DE-DATA-002），经 payload_ref 由 PendingDiff 关联，持久化于 workspace.db
`diff_payloads` 表（DE 拥有形状、WS db 承载持久化，见 §1 分层）。

<!-- TERM
term_id: TERM-DE-005
chains: DE-DIFF
zh: 提案批次
en: ProposalBatch
forbidden: diff批次, 批量diff, 修改批次, 提案组, batch diff
-->
同一次已校验 ChangeProposal 产生的一组关联 PendingDiff。批次保留来源 AgentTurn/ToolCall、稳定顺序与
分组版本，只用于追溯和批量 UI 展开；没有统一裁决终态，每个 PendingDiff 独立进入 SM-DE-01。

## §1 文档定位

```
REQ-CORE-002 → REQ-DE-001~005（DE L1）→ 本文（SSOT）→ 代码(@GOV)
本文 ←承接← A-SYS-I-P-01（字段/枚举单一来源）、A-AG-M-T-01（ChangeProposal 交接）
本文 →驱动→ A-DE-M-S-01（SM-DE-01 diff 终态机）→登记→ 总图（闭合 WS/AG 单侧转移点）
本文 →消费→ ED-EDIT/ED-DECOR（草稿预览、open session 预览与呈现）；未打开文档后台裁决信号 →可驱动→ SM-WS-02（apply 落盘）
```

**分层重申**：字段归 SYS、持久化与落盘归 WS、呈现归 ED、产出归 AG；本文拥有**转移语义、payload
形状、冲突与过期规则、裁决粒度**。

## §2 裁决粒度（解除 DE-01）

**裁决单元 = 可独立裁决的连续内容块组**：每条 PendingDiff 命中一个完整块或一组文档顺序连续、共同
表达一个不可分修改的块（blockId 锚，BR-ED-DATA-001），接受/拒绝以单条 diff 为单位。块内字符/词级
高亮只属呈现增强，不改变裁决单元。

同一次 ChangeProposal 先形成 ProposalBatch，再按以下唯一规则拆分：

1. 按目标文档和稳定文档顺序把操作归一为**块候选**：同一现有 blockId 上的多个操作先在同一基线事务
   中合成一个 before/after 候选；插入的新连续块按各自最终 blockId 建候选。不同文档或不连续块永不合并。
2. 默认每个块候选独立成卡；模型可用相同 `adjudication_group` 声明相邻块候选语义不可分，运行时只在
   目标区间相邻/重叠、共同裁决不会掩盖独立选择且可在同一编辑事务中校验时接受合并声明。
3. 未声明同组、声明不同组，或虽相邻但可独立接受/拒绝的操作必须拆卡；目标集合重叠、父子结构互相
   依赖或任一卡反向会破坏另一卡时不能伪装为独立卡，必须合并为一个不可分组或整批拒绝；整篇
   `replace_document` 天然一组。
4. 所有组先完成目标、基线、格式/动态能力与组内事务校验，再在一个持久化事务中写入 ProposalBatch 和
   全部 PendingDiff；任一组失败则整批不生成，避免部分卡片歪曲模型提案。
5. 同一文档同批的多个结构化卡共享完全相同的 `base_session_version/base_content_hash`。已有 LogicalState
   路径必须把各卡操作聚合后再做一次全批预检与一次 ED 前向事务；不得按卡依次应用并让第一卡推进的
   文档哈希使兄弟卡过期。预览成功后，每卡只以自身目标集合及 after fingerprint 作为接受/拒绝的有效性
   guard；同批其他卡的预览或裁决不改变本卡状态。

Agent 1.0 的多卡独立预览当前只适用于活动 `current_editor` 上可目标局部反向的 `block_ops`，以及具稳定
blockId 和逐块 before/after 的 `replace_block`；两者分别聚合到既有结构化操作或编辑操作的单一事务。
`replace_document`、`style_ops`、`table_ops`、`image_ops` 仍按各自完整事务生成单卡；`open_session` 的
字符串应用也保持单卡。它们只有在其 owner 协议以后明确提供目标局部 guard 与反向事务后才可拆卡，
不得仅为多显示卡片而复制整篇 before/after。

ProposalBatch 只表达关联。批量接受/拒绝必须展开为多个逐卡裁决，不能新增一个批次终态覆盖各卡结果。

## §3 diff 生成（DE-DIFF）

<!-- CHAIN
chain_id: DE-DIFF
主责模块: DE
status: active
risk_tier: P1
integration_test: src-tauri/tests/core_chain.rs::accepted_proposal_flows_through_de_to_ws_landing
-->

<!-- RULE
rule_id: BR-DE-DATA-001
主链路: DE-DIFF
域: DATA
-->
diff 生成必须满足：自一次修改建议（ChangeProposal）创建一个 ProposalBatch，并按 §2 生成一个或多个
PendingDiff；每卡只包含一个可独立裁决的连续内容块组。不同文档、不连续或可独立裁决的组不得合并；
同一块上的操作先合成为一个块候选；相邻块候选只有共享同一 adjudication_group 且组内事务可校验时才可合并；不能独立反向的目标必须合并或拒绝。
同一文档同批结构化卡共享生成基线并由 ED 聚合为一次前向事务，批内兄弟卡不得因该事务推进整篇版本而互相失效。所有组在写入前完成 blockId/
基线/格式动态能力校验；锚失效或任一组无效时整批失败并向 AG 返回结构化失败 ToolResult，不得产生错位、无锚
或部分持久化 diff。持久化成功后逐卡进入 SM-DE-01；PendingDiff/ProposalBatch 字段与 status 枚举引用
A-SYS-I-P-01 单一来源（X-SHARED-001），DE 不重定义。

**跨会话边界（2026-07-21 收紧，APP-CLOSE-DIFF-LIFECYCLE-01）**：持久化保证的是当前运行期内
审阅呈现可完整重建，**不保证裁决权或未确认写盘效果跨会话存活**。任一新会话启动时，清算
（BR-SYS-STATE-005，A-SYS-M-T-02 §4）先于任何恢复执行：残留 `pending` 行统一转 `expired`
（`terminal_reason=abnormal_exit`），残留 `awaiting_save` 效果统一转 `discarded_on_abnormal_exit`；
之后仅终态记录进入只读历史呈现。pending 态只在产生它的连续运行期内可被裁决。

**AgentContextFacts 投影边界（AG-CONFORMANCE-CODE-S4A）**：DE 仅向当前 AgentSession 投影
ProposalFact 的 `diffId/sessionId/relPath/state/effectStatus/saved`；`state` 是 DiffCard 对权威裁决态与受控过渡的
既有投影，`effectStatus/saved` 用于区分已接受待保存、已保存与异常结束后已丢弃效果。不属于当前
AgentSession 的 Proposal（含无归属旧记录）不进入该事实集；投影不携带 before/after/payload，也不改变 DE
裁决权。跨模块字段形状统一引用 BR-SYS-DATA-008，本文不重定义近义结构。

## §4 payload 载荷形状（解除 REQ-DE-004 占位）

`payload_ref` 所指载荷的结构（A-SYS-I-P-01 显式归 DE）：

| 字段 | 语义 | 备注 |
|---|---|---|
| payload_version | 载荷形状版本 | 单调递增，支持迁移 |
| proposal_batch_id / group_index / group_count | 批次身份与卡片稳定顺序 | 仅关联；不共享裁决终态 |
| target_blocks | 目标内容块定位（blockId 列表 + DocRef） | 锚集合；可细化至字符锚区间（CharAnchor，A-ED-M-T-01 §4.1，2026-07-03 批B） |
| before_snapshot | 修改前目标内容快照 | 外部变更检测基准（§6）+ 审阅对比基准 |
| after_content | 修改后内容 | 当前编辑器草稿应用或后台 apply 的执行物；纯样式 diff 时可空 |
| style_ops | 样式操作序列（StyleOp，A-ED-M-T-01 §5.1；2026-07-03 裁定 D-3 新增，可空） | 回合产出的样式修改载体，与 after_content 并存或独立 |
| table_ops | 表格操作载荷（TableOperation + before/after TableSnapshot，可空） | BDocx 表格语义修改载体；不得退化为原始 HTML 替换 |
| image_ops | 图片操作载荷（ImageOperation + 项目来源/锚 + before/after ImageSnapshot，可空） | BDocx 图片语义修改载体；不得退化为原始 HTML 替换 |
| snapshot_source | 生成基线来源 | active_editor / active_editor_session / open_editor_session / disk_snapshot；已打开文档不得为 disk_snapshot |
| base_session_version | 生成时 ED session 版本 | `snapshot_source=active_editor/active_editor_session/open_editor_session` 时必填；同文档同批卡完全相同 |
| base_content_hash | 生成时内容哈希 | `snapshot_source=active_editor/active_editor_session/open_editor_session` 时必填；同文档同批卡完全相同，用于前向批预检而非批内逐卡裁决 |
| origin | 来源元数据（origin_module、回合标识、tool_call_id） | 可溯源到 AgentTurn 与模型提案 |

实现约定：为兼容后台 `de.accept_diff` 的统一落盘路径，纯样式 diff 的 `after_content` 可在运行时写入
目标基准内容（即文本内容不变），样式语义由 `style_ops` 承载；审阅 UI 以 `style_ops` 重建样式摘要，
当前编辑器草稿路径以 `style_ops` 应用真实样式。

运行时视图投影约定：AG 的 `propose_edit` 必须在 `DiffProposed` 前通过 `ToolResult` 返回
`DiffProjection`，字段包括 `{diff_id, proposal_batch_id, group_index, group_count, rel_path, patch_kind, block_id?, block_index?, before_text,
after_text, location_label, draft_mode, snapshot_source, base_session_version?, base_content_hash?,
style_ops?, block_ops?, table_ops?, image_ops?}`。`draft_mode` 仅允许 `current_editor / open_session / background`。DE/前端建 DiffCard 与 ED 草稿只能消费该投影，
不得用当前编辑器整篇内容截尾或 `tool_request.params` 重建 before/after。`replace_block` 的 before/after
必须来自同一目标 Block；`replace_document` 只在显式整篇替换时使用整篇基准。`draft_mode=open_session`
必须携带 session 基线元数据，且只能用于已打开非激活文档；`draft_mode=background` 只能用于未打开文档。
同一文档同批块级多卡的 `target_blocks` 必须互不重叠，且各卡 `block_ops` 或稳定 `replace_block` 的
before/after fingerprint 只覆盖本卡目标；前端不得为每卡复制整篇 after 作为裁决 guard。其余
patch_kind 在当前协议中 `group_count=1`。

<!-- RULE
rule_id: BR-DE-DATA-002
主链路: DE-DIFF
域: DATA
-->
diff 载荷必须含：载荷版本（payload_version，形状变更须版本递增并兼容迁移）、目标内容块定位
（可细化至字符锚区间）、修改前内容快照、修改后内容（纯样式/结构操作 diff 可保持基线）、样式操作、BlockOperation、TableOperation
或 ImageOperation 序列（均可空；其编辑语义归 ED，本规则锁在载荷中的承载与版本化）、生成基线来源、
session 版本/内容哈希（已打开文档必填）、ProposalBatch 身份/组内顺序与来源元数据；ImageOperation 还必须保留项目来源路径/hash、
Block 或图片实例锚及 before/after ImageSnapshot。经 WS 持久化并恢复后必须可完整重建审阅呈现
（对比基准 = before_snapshot，含样式、表格和图片变更语义的可视对比）；
具体序列化格式延实现期，本规则锁字段语义集合。

## §5 终态裁决语义（DE-DIFF）

<!-- RULE
rule_id: BR-DE-STATE-001
主链路: DE-DIFF
域: STATE
-->
待裁决修改的终态转移必须满足：pending→accepted/rejected/expired 三终态互斥且不可逆（终态后
不得再转移、不得复活）。已有 LogicalState 的当前或非激活打开文档中，块级、整篇和 StyleOp diff 在
pending 时即经 ED 受控接口把 after/样式操作应用为草稿并产生脏态；同文档同批结构化多卡必须先聚合为
一次原子预览，再分别保持可接受/拒绝。显式接受
不得再次应用内容或写盘；保存仍有效的 pending 预览时，必须先裁决为 accepted/awaiting_save，再经保存链写盘。若保存前
本卡目标不再匹配 after fingerprint，必须将该卡置 expired，保存当前 LogicalState 但不得把它裁决为 accepted；
同批预览或互不重叠兄弟卡裁决造成的整篇版本推进本身不构成失效。pending 后打开的后台目标必须以同一 DiffPayload 建立
LogicalState、显示 GreenReview 并切换到这一路径。无 LogicalState 的后台 diff 才可走 accepted→SM-WS-02
apply 路径，且仅适用于文本/.html 等可直接应用载荷。既有 BDocx 的磁盘权威是 docx 字节，DiffPayload
中的 HTML/Block 投影不得经后台 raw apply 写入 `.docx`；未在当前编辑器 HTML 草稿中成功应用的 BDocx
diff 必须进入 error（如 `DE_BDOCX_DRAFT_REQUIRED`），等待用户打开目标 BDocx 后由 BR-CORE-STATE-003
迁移或重新生成。
rejected/expired 不写文件；若普通文本草稿已应用，rejected 经 ED 受控接口按 before 回滚；若
block_ops 草稿已应用，拒绝前必须验证当前目标仍匹配 afterNode fingerprint，再在一个事务中恢复
exact beforeNode。指纹已变化则投 conflict 并停止后端拒绝，不得整篇覆盖后置人工编辑。用户裁决交互经
ED/DiffCard 转达（X-FORBID-005），DE 不绕过 ED 直接操纵编辑器。DiffCard 显式接受 block_ops
也必须先验证 afterNode fingerprint；已变化则保持 Pending 并投 conflict，不确认不可逆终态。保存动作
是对该文档中仍能逐卡通过目标 after guard 的 pending 预览执行接受并落盘的入口；不得把失效卡或同批
其他卡的终态一并改写。
状态转移归 SM-DE-01。

### §5.1 效果态（解除审计发现：accepted 不等于已落盘的历史不一致，2026-07-20 新增）

`pending_diffs` 新增 `effect_status`（`not_applicable`/`awaiting_save`/`saved`/
`discarded_on_abnormal_exit`）与 `terminal_reason`（可空）两列，字段形状单一来源为 A-SYS-I-P-01，
语义规则 SSOT 为 `BR-SYS-DATA-005`（A-SYS-M-T-02 §4，APP-CLOSE-DIFF-LIFECYCLE-01）。核心动机：
`status=accepted` 只代表裁决结果，已有 LogicalState 路径的 accepted 在用户实际保存
成功前不代表内容已落盘（本文 §5 已锁定该语义），历史展示必须能区分"裁决同意"与"内容生效"两个
维度，不得笼统显示为完成态。

DE 侧责任：`de.accept_diff`/`de.accept_draft_diff` 成功时按路径写入 `effect_status`——无 LogicalState 的
`background` 路径与 `ws.apply_diff` 同一事务原子完成，写 `saved`；已有 LogicalState 路径写
`awaiting_save`。真实保存成功（SM-ED-01 saved 事件）时由 ED/WS 侧按 `rel_path` 批量把该文档下
`awaiting_save` 的 accepted 行翻转为 `saved`（DE 不直接监听保存事件，由调用方在保存成功回调中
调用 DE 提供的批量翻转接口，避免 DE 反向依赖 ED/WS 内部状态机）。`awaiting_save` 的另一去向是
关闭流程内用户显式放弃或 `BR-SYS-STATE-005` 启动清算转 `discarded_on_abnormal_exit`（归 SYS 侧编排，
DE 侧不主动触发）；无论是否留下 dirty 快照，它都不得跨会话保留。

## §6 过期与外部变更（解除 DE-03 机制）

<!-- RULE
rule_id: BR-DE-STATE-002
主链路: DE-DIFF
域: STATE
-->
待裁决修改在以下条件转 expired：（a）超时——自创建起超过可配置时限未裁决；（b）外部变更——
预览前目标内容块与 before_snapshot 不一致，或预览后目标不再匹配本卡 after fingerprint（经目标内容
比对/文档 version_seq 辅助检测）。整篇版本变化只能触发目标重检，不能在目标仍匹配时直接误杀同批兄弟卡。外部变更检测
不可得时保守不自动过期，保持 pending 留人类裁决；expired 为终态、不落盘、装饰复位。超时时长
数值与检测触发时机（呈现时/裁决时/定时）延实现期标定，本规则锁判定条件与保守语义。

## §7 多 diff 并存与冲突（解除 DE-02）

<!-- RULE
rule_id: BR-DE-DATA-003
主链路: DE-DIFF
域: DATA
-->
同一文档多条待裁决修改并存时：呈现与裁决按 ProposalBatch/group_index 后接创建序组织；同批身份不
改变逐卡独立状态。**同一目标内容块不得并发草稿预览、open session 预览或后台落盘**——同批结构化
多卡的目标必须互不重叠，并在共同基线上聚合成一次前向事务；事务成功后按各卡目标 after fingerprint
独立裁决，任一卡反向只作用于自身目标。不同批次中，先序 diff 改变
编辑器草稿、ED session LogicalState 或后台落盘后，与其目标重叠的后续 pending diff 必须重新执行外部
变更检测（§6，before_snapshot 比对），内容已变更者转 expired（不静默用陈旧 after_content 覆盖新内容）；
互不重叠的 diff 可独立裁决；同批预览不得触发兄弟卡的 `document_revision_advanced`，同批一张卡的终态
不得自动改变其他卡。冲突判定以目标内容块集合及结构依赖关系为准，不能只看整篇 content_hash。

## §8 跨模块约束（复用，不新增）

本文不新增 CONSTRAINT：唯一落盘路径已由 **X-FORBID-006**（A-AG-M-T-01）锁定、共享结构由
**X-SHARED-001** 锁定、呈现通道由 **X-FORBID-005** 锁定、建机前置与转移登记由
**X-SEQ-003 / X-INTER-001** 锁定；本文 §3–§7 在 DE 侧落实之，避免重复定义同义约束（ADU §3.1）。

### §8.1 DiffCard 默认审阅形态（DE-04 局部闭合）

逐条 DiffCard（TERM-DE-003）默认采用紧凑审阅卡形态，固定为五个信息行：

1. 标题行：修改标题 + 状态徽标；
2. 位置行：文件路径/目标位置 + 可用时的定位操作；
3. 红删行：before_snapshot 的删除预览，默认单行省略；
4. 绿增行：after_content 的新增预览，默认单行省略；
5. 操作按钮行：接受/拒绝/重试/关闭等当前状态允许的操作。

长文本、换行内容、样式操作摘要、冲突/过期/错误提示、处理中进度等属于详情信息：默认态不得无界撑高卡片，
应以受控摘要或展开详情区呈现。展开态可以显示完整 before/after 和状态详情，但不得改变裁决语义；
当前编辑器草稿路径下，红删/绿增行必须来自同一目标块的 before/after，不得用整篇文档尾部截取充当
before。DiffCard 是 PendingDiff 在代理回合信息流中的视图投影：应锚定到触发它的会话、AgentTurn 和
assistant 消息后方，不得把多轮修改统一沉底为会话级全局列表；未带历史锚点的 legacy diff 可进入
“未归属修改建议”fallback 区以避免丢失。接受/拒绝/过期等终态仍以 SM-DE-01 和 BR-DE-STATE-001 为准。
已有 LogicalState（包括 `open_session` 与 pending 后打开迁移）的 DiffCard 在预览成功后必须显示为已应用到
LogicalState，但仍保留接受/拒绝；这些操作只能裁决卡片，不得二次应用内容。预览失败、基准推进或超时后
才显示失效/错误原因并禁用裁决。
同一 ProposalBatch 的卡片应保留批次身份与 `group_index/group_count`，但每张卡都有独立操作与终态；
批量裁决条是全局操作控件，其动作必须展开为多个逐卡命令。侧栏/内联整体布局仍属 DE-04 剩余 UI 债务。

### §8.2 WorkspaceOperation 与 CreateDocumentBundle 边界（2026-08-21）

- 创建、删除、重命名、移动、恢复等资源操作不属于 ChangeProposal，不得形成 `delete_doc/delete_folder` patch_kind、空 before/after 投影或资源 DiffCard。
- 资源操作由 WS-OPERATION / SM-WS-04 承载；DE 仅提供受影响 PendingDiff 清单，并在资源执行成功后以 `resource_baseline_changed` 使相关内容卡失效。
- CreateDocumentBundle 的资源创建成功后，DE 才接收带 `operation_id` 与 `origin=create_document_bundle` 的初始内容 ChangeProposal；其 PendingDiff 使用普通 SM-DE-01。
- 初始内容 Diff 的拒绝若请求删除本次创建资源，DE 只发出裁决结果；WS 必须独立校验资源、DiskState 与 LogicalState 三重基线，DE 不直接删文件。

## §9 需求到规则映射（REQ_MAP）

<!-- REQ_MAP
req_id: REQ-DE-001
target_type: CHAIN
target_id: DE-DIFF
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-DE-001
target_type: RULE
target_id: BR-DE-DATA-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-DE-002
target_type: RULE
target_id: BR-DE-STATE-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-DE-003
target_type: RULE
target_id: BR-DE-STATE-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-DE-003
target_type: RULE
target_id: BR-CORE-STATE-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-DE-003
target_type: RULE
target_id: BR-CORE-STATE-002
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-DE-003
target_type: RULE
target_id: BR-CORE-STATE-003
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-DE-003
target_type: RULE
target_id: BR-DE-DATA-003
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-DE-004
target_type: RULE
target_id: BR-DE-DATA-002
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-DE-005
target_type: RULE
target_id: BR-DE-STATE-002
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-DE-006
target_type: RULE
target_id: BR-WS-STATE-003
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-DE-006
target_type: RULE
target_id: BR-DE-STATE-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-DE-007
target_type: CONSTRAINT
target_id: X-FORBID-006
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-DE-007
target_type: RULE
target_id: BR-WS-OBS-002
status: mapped
-->

### §9.1 BDocx 表格语义与单元格底纹 Diff（2026-07-28）

BDocx 表格结构建议是 `table_ops` 专用载荷，不得伪装为 `replace_block` 或原始 HTML 替换。其
`after_content` 保持生成时的完整 HTML 基线，实际预览与草稿应用由同一 ProseMirror 表格操作规划器
执行受限 `TableOperation`；任一操作不能规划或执行则整批失败，不产生部分结构效果。载荷目标必须同时
匹配活动 BDocx 编辑会话、`tableId`、表格拓扑指纹、sessionVersion 与 contentHash。代理只能使用
`list_tables` 投影公开的动态 capabilities 和 moveDirections；富内容单元格不能用纯文本操作覆盖。
`set_cell_background` 只修改节点属性，before/after TableSnapshot 必须包含各逻辑单元格的
`backgroundColor`，且该属性参与表格指纹。

DiffCard 须呈现结构前后行列形状和每个操作的坐标/参数，而非仅把表格显示为笼统的全文替换。拒绝时以
底纹操作还须显示 `RnCn` 坐标、修改前/后色块和 `#RRGGBB`；清除态须明确显示“无底纹”，不得只显示
操作名称。该 diff 的完整草稿前 HTML 用于拒绝恢复，接受沿用已应用草稿的既有裁决语义。列宽拖拽值是当前编辑会话的交互
锚与差异上下文，不能声明为 DOCX 往返保真承诺。

### §9.1.1 富文档结构化内容块 Diff（2026-07-30）

富文档普通内容建议使用 `block_ops`，不得伪装为 replace_block、replace_document 或替换后的 HTML。
持久载荷保留生成基线和 canonical operations；每项 beforeNode 由 AG 工具层从同次可信
ProseMirror 快照补齐。Pending 时 ED 先整批预检再单事务应用，DiffCard 紧凑态显示块数，展开后按
blockId/nodeType 逐项显示精确前后文本。拒绝只精确反向这些节点；目标在应用后被人工修改时进入
conflict。接受仍只确认已应用草稿，保存进入既有 ED/WS 链。

### §9.2 BDocx 图片语义 Diff（2026-07-29）

BDocx 图片建议使用 `image_ops` 专用载荷，不得伪装为 `replace_block`、`replace_document` 或原始
HTML 图片变更。载荷必须保留生成时 sessionVersion/contentHash、插入 Block 与项目来源 path/hash，
或目标稳定 imageId/当前 fingerprint；并携带操作级 before/after ImageSnapshot。缩放须显示前后宽度，
移动须显示前后序号/Block 相对位置，对齐须显示前后 left/center/right。仅
`draft_mode=current_editor` 可应用，其他模式在生成前拒绝。

DiffCard 对插入须显示来源相对路径、目标 Block 前/后、alt、受限宽度与新增图片状态；对删除须显示
目标图片 alt、尺寸、所在 Block、媒体身份和删除后状态；缩放、移动与对齐须显示精确参数及真实前后
快照。该结构化信息必须来自后端 DiffProjection，
不得从原始 HTML 或工具请求文案猜测。Pending 时先由 ED 单事务应用草稿；拒绝恢复草稿前全文，接受只
确认现有草稿，保存进入既有链。同一图片可在一张卡中组合一次缩放、对齐和移动，展示顺序与执行顺序
固定为缩放→对齐→移动；删除必须独占。项目源文件始终不属于 Diff 删除效果。未应用或已过期的错误卡
不得以接受动作伪装“重试”并覆盖首要根因；只有已应用但持久化失败的卡才可重试持久化。

## §10 测试与验收

- **生成、分组与锚**（BR-DE-DATA-001，REQ-DE-001 验收）：同块多操作、单块、一组相邻不可分块、相邻可独立块、
  不连续块与跨文档样本分别产生预期 1/N 张卡；proposal_batch_id 相同且 group_index 稳定，每卡定位准确；
  任一锚/组校验失败整批失败回报 AG，无错位、无锚或部分持久化 diff。
- **载荷往返**（BR-DE-DATA-002，REQ-DE-004 验收）：持久化→恢复后按批次和组序完整重建审阅呈现，
  各卡 before/after 只含本组且基准一致；同文档同批结构化多卡共享基线、目标集合互不重叠；payload_version 迁移不丢载荷。
- **终态语义**（BR-DE-STATE-001 / SM-DE-01，REQ-DE-002/003 验收）：三终态互斥不可逆；已有
  LogicalState 的草稿路径在 pending 即预览、用户显式接受不直接落盘，而保存有效 pending 预览会先转 accepted/awaiting_save；后台文本/.html 路径仅在
  持续无 LogicalState 时可由 accepted 触发 apply。已打开非激活文档或 pending 后打开迁移预览成功后保持
  pending 且接受/拒绝可用，基线不匹配时不得应用、不得继续裁决；既有 BDocx 未进编辑器草稿时不得 raw apply；
  rejected/expired 文件不变，草稿已应用时 rejected 只回滚本卡目标；同批其他卡的预览与终态保持有效；ApplyError 不回退终态。
- **过期保守性**（BR-DE-STATE-002，REQ-DE-005 验收）：超时/外部变更转 expired；检测不可得不误杀。
- **冲突**（BR-DE-DATA-003）：同文档同批结构化多卡聚合预检并只 dispatch 一次前向事务；批内卡不因
  共享预览推进整篇版本而互相失效，拒绝一张只反向其目标且其他卡仍可裁决。不同批次重叠 diff 在先序
  草稿/open session 预览或后台落盘后重检，陈旧者 expired；无并发应用/落盘同一块。
- **DiffCard 紧凑态**（TERM-DE-003，DE-04 局部闭合）：默认渲染为标题/位置/红删/绿增/操作五行；
  长内容、状态提示与样式摘要不无界撑高默认卡片，展开后可查看详情；多轮 DiffCard 按各自
  AgentTurn/assistant 消息锚点插入消息流，不沉底汇总；已有或 pending 后打开迁移的预览卡仍可接受或拒绝，
  只有失效/错误卡不可裁决。
  BDocx 底纹 Diff 必须同时显示精确逻辑坐标、前后色块与色值/清除态。
  BDocx 图片 Diff 必须分别显示插入来源/位置/alt/width 或删除目标前后状态，不得退化为“修改图片”
  摘要；非活动会话不产生 image_ops 卡。
- 跨模块（G5 联调）：SM-AG-02 propose_edit→DE 生成、DE pending→ED 草稿应用、DE accepted→ED/WS save
  或后台 SM-WS-02 apply、DE rejected→ED 回滚/复位三链对接。
- 量化 NFR（生成/呈现延迟）延 G5（DE-05）。

## §11 技术债务与盲区（诚实声明）

- **审阅 UI 形态剩余项**（DE-04）：逐条 DiffCard 默认紧凑形态和 ProposalBatch 的逐卡裁决语义已在
  §2/§8.1 闭合；侧栏/内联整体布局仍属后续产品体验，不影响当前分组与终态。
- **非激活/迁移预览的视觉债务**：必须保留用户裁决入口并防止二次内容应用；DiffCard 视觉仍可能只有绿增
  预览、缺少完整红删效果。后续需要补完整红删/绿增对比和 DiffCard 失效态视觉，不得借此禁用接受/拒绝。
- **超时时长与检测触发时机**（DE-03 数值部分）：判定条件已锁，数值与触发时机延实现期标定。
- **分组启发式调优**：§2 已锁唯一安全基线（默认拆分、仅相邻且显式同组/事务可校验时合并）；后续可在
  不改变该边界下优化模型对 adjudication_group 的选择，不能把调优当作恢复单卡聚合的理由。
- **P2 CBT 构建产物**：不属于 Agent 1.0 当前链路；未来 Profile 若启用，必须显式复用或扩展本文协议并重新验证。
- 块内细粒度对比的呈现算法（字符/词级 diff）属 ED 装饰层渲染实现，延实现期。

## §12 维护与实现入口

1. 规则、术语、链路或映射变化后运行 `npm run governance:generate`，不得人工修改 ADUS 自动生成区。
2. A-DE-M-S-01 是逐卡状态机真源，A-DE-I-P-01 是接口真源；分组字段变化须同步 SYS 共享结构和 WS 批次持久化。
3. 实现与验证按同块多操作、单块、相邻不可分、相邻可独立、不连续和跨文档样本覆盖 1/N 分组及独立终态。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-09-03 | v1.24 | AG-ADAPTIVE-RUNTIME-DESIGN-01：注册 ProposalBatch；BR-DE-DATA-001 锁定可独立裁决连续块组的默认拆分/受控合并、整批预检与原子持久化；同文档同批结构化卡共享基线并聚合为一次 ED 前向事务，逐卡以目标 after fingerprint 独立裁决。 |
| 2026-08-30 | v1.23 | AG-CONFORMANCE-CODE-S4A：在 BR-DE-DATA-001 下补充当前 AgentSession ProposalFact 投影边界，携带裁决/效果/保存态但不携带内容，不引入新近义规则。 |
| 2026-07-21 | v1.12 | 收紧跨会话边界：pending 仅在产生它的连续运行期内可裁决；任一启动清算均把残留 pending 转 expired、残留 awaiting_save 转 discarded_on_abnormal_exit，之后只恢复终态只读历史。 |
| 2026-06-11 | v1.0 | 初版（DE G2 首件）：DE 规则 SSOT——裁决粒度定为内容块级（解除 DE-01）、diff 生成与锚失效回报、payload 载荷形状（版本化 + before_snapshot 基准，解除 REQ-DE-004 占位）、终态裁决语义（三终态互斥不可逆、accepted 唯一触发落盘、ApplyError 不回退）、过期与外部变更（保守语义，解除 DE-03 机制）、多 diff 冲突（重叠重检、无并发落盘，解除 DE-02）；注册链路 DE-DIFF（P1）、规则 5 条、术语 TERM-DE-001 PendingDiff / TERM-DE-002 ChangeProposal（解除 A-SYS-I-P-01 §6 延期）；约束全复用不新增；REQ_MAP（REQ-DE-001~005）；债务（审阅 UI 待人类、数值延实现、CBT 联合） |
| 2026-07-03 | v1.1 | 批B 样式 diff 承接（A-ED-X-L-02 PI-ED-11 文档先行，人类裁定 D-3 方案A：回合样式修改经 DE 裁决）：§4 载荷表 +style_ops（StyleOp 序列，可空；语义归 ED BR-ED-DATA-005，载荷承载与版本化归本文）、after_content 纯样式 diff 可空、target_blocks 可细化字符锚区间；BR-DE-DATA-002 规则文本同步（含样式变更语义的可视对比重建）；DiffCard 样式呈现形态延实现期（DE-04 审阅 UI 债务口径） |
| 2026-07-04 | v1.2 | 术语统一清扫（承 2026-07-04 人类三轮裁决）：正文禁用别名替换——内联辅助→局部修改、自动补全→辅助续写、对话编辑→Diff Edit、Chat↔Build 接管→构建移交/回落、chat/chat-agent 回合→代理回合；文件名与变更记录历史行不改写 |
| 2026-07-08 | v1.3 | DE-04 局部闭合：逐条 DiffCard 默认审阅形态确定为标题行、位置行、红删行、绿增行、操作按钮行；长内容、样式摘要和状态详情进入受控摘要/展开区，批量裁决与侧栏/内联整体布局继续作为剩余 UI 债务 |
| 2026-07-08 | v1.4 | 修正代理编辑 diff 生命周期：当前编辑器命中的块级 diff 在 pending 时先经 ED 应用为草稿并置脏态，accepted 只确认、rejected 回滚、save 成功后复位 GreenReview；后台/整篇 diff 保留 accepted→WS apply 路径；DiffCard before/after 必须来自同一目标块 |
| 2026-07-09 | v1.5 | 修正 DiffCard 信息流归属：逐条卡片按 sessionId/turnId/assistant 消息锚点渲染在对应代理回合消息后，legacy 未归属卡进入 fallback 区；批量裁决条保留全局操作属性 |
| 2026-07-09 | v1.6 | 补齐 StyleOp 载荷运行约定：`propose_edit.style_ops` 随 DiffPayload 持久化，纯样式 diff 可用基准内容填充 `after_content`，真实样式语义由 `style_ops` 进入 ED 草稿应用与审阅摘要 |
| 2026-07-09 | v1.7 | 补齐整篇 content diff 草稿路径：当前打开编辑器的整篇 diff 与块级/StyleOp diff 一样在 pending 时进入 ED 草稿，accepted 仅确认、rejected 恢复草稿前全文，后台不可应用 diff 才走 WS apply |
| 2026-07-09 | v1.8 | 收紧整篇 diff 语义：只有 `propose_edit.operation=replace_document` 产生的整篇 diff 可进入 ED 全文草稿替换；已有文档裸 `content` 不再生成正常 PendingDiff，避免 BDocx 局部编辑定位失败时退化为全文替换 |
| 2026-07-09 | v1.9 | 补齐 DiffProjection 与 BDocx 裁决路径：DiffCard/ED 草稿只消费后端权威投影，禁止前端预测 before/after；既有 BDocx 未应用到当前编辑器 HTML 草稿时不得走后台 raw apply 写入 docx 字节 |
| 2026-07-18 | v1.10 | 新增 `draft_mode=open_session` 技术契约：已打开非激活文档 diff 基线校验通过后直接应用到 ED session LogicalState 并禁用裁决；DiffProjection 增 source/session/hash 基线字段；记录视觉与撤销债务 |
| 2026-07-20 | v1.11 | 承接 APP-CLOSE-DIFF-LIFECYCLE-01：§3 精确化"重启可恢复"边界（只保证正常运行期审阅呈现，不保证裁决权跨异常退出存活，异常退出清算先于恢复执行）；新增 §5.1 效果态（`effect_status`/`terminal_reason`，语义 SSOT 归 `BR-SYS-DATA-005`，A-SYS-M-T-02 §4）解决"accepted 不等于已落盘"历史不一致 |
| 2026-07-27 | v1.13 | DE-SAVE-IMPLICIT-ACCEPT-01：保存即接受产品决策落地；当前文档保存前必须把已应用 pending Diff 持久化为 accepted/awaiting_save，保存成功后复用既有效果态回调转 saved，禁止仅改前端卡片状态。 |
| 2026-07-28 | v1.14 | BDocx 表格结构编辑：新增 §9.1，锁定 `table_ops` 的活动会话/表格指纹/动态能力校验、共享 ProseMirror 原子规划与草稿恢复、富内容单元格保护和结构化 DiffCard；列宽拖拽不构成 DOCX 往返保真承诺。 |
| 2026-07-28 | v1.15 | BDocx TableCellShading：`table_ops` 载荷纳入 before/after `backgroundColor`；DiffCard 锁定逻辑坐标、前后色块、十六进制色值与清除态的精确呈现。 |
| 2026-07-29 | v1.16 | BDOCX-IMAGE-AGENT-EDIT-01：新增 image_ops 载荷与 §9.2，锁定项目来源/动态锚、前后 ImageSnapshot、current_editor 草稿、结构化 DiffCard 与源文件保护。 |
| 2026-07-30 | v1.17 | 图片代理编辑闭合审计：§9.2 同步同图缩放/对齐/移动组合、删除独占及错误卡不得用接受动作伪装重试的语义。 |
| 2026-07-29 | v1.17 | BDOCX-IMAGE-ADVANCED-AGENT-OPS-01：图片结构化 Diff 扩展缩放宽度、文档流移动与段落对齐的精确前后快照。 |
| 2026-07-30 | v1.18 | AG-RICH-BLOCK-STRUCTURED-EDIT-01：新增 §9.1.1 block_ops 结构化 Diff；exact beforeNode/afterNode 成为应用/审阅/回滚共同真值，列表子块逐项展示，fingerprint 变化转 conflict 而非全文恢复。 |
| 2026-07-30 | v1.19 | AG-RICH-BLOCK-STRUCTURED-EDIT-01 闭合补强：DiffCard 显式接受 block_ops 增 afterNode fingerprint 前置校验；后置编辑时不提交 accepted，并明确保存即接受仍按既有产品决策执行。 |
| 2026-08-13 | v1.20 | DIFF-STATE-COMPASS-01：BR-DE-STATE-001 与审阅/验收口径对齐 BR-CORE-STATE-001~003；移除 open_session 自动 accepted/禁用裁决及保存隐式接受，明确已有或迁移 LogicalState 预览后卡片仍 pending。 |
| 2026-08-13 | v1.21 | 人类复核更正：保留“预览后仍 pending”，但保存仍有效 pending 预览即裁决为 accepted/awaiting_save 后写盘；仅基准已推进时使旧卡 expired，不得误裁决。 |
| 2026-08-21 | v1.22 | 工作区资源操作基线重构：新增 §8.2，禁止资源操作进入 ChangeProposal/DiffCard；CreateDocumentBundle 仅在资源创建后接入普通 SM-DE-01，安全拒绝删除由 WS 三重基线 guard 执行；补 REQ-DE-006/007 映射。 |
