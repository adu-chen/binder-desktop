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
由 AG 修改建议生成、经 WS 持久化、等待裁决或受控自动应用的修改单元（共享结构归 A-SYS-I-P-01）；
status 枚举 pending/accepted/rejected/expired，终态互斥不可逆。当前编辑器命中的块级 PendingDiff
生成后先由 ED 应用为草稿内容，accepted 只确认裁决态，后续由 ED/WS save 落盘；已打开非激活文档的
PendingDiff 在基线校验通过后自动应用到对应 ED session LogicalState 并转 accepted；未打开文档的
后台 PendingDiff 可保留 accepted→WS apply 的旧落盘路径。

<!-- TERM
term_id: TERM-DE-002
chains: DE-DIFF
zh: 修改建议
en: ChangeProposal
forbidden: AI建议, 修改意见, 编辑建议, 产出建议
-->
AG 代理回合产出中交接 DE 的修改意图载体（目标定位 + 修改内容 + 来源元数据），是 SM-AG-01
HandingOff 与 DE 生成（SM-DE-01 Generating）之间的交接物；经 DE 生成为待裁决修改后进入裁决。

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

## §1 文档定位

```
REQ-CORE-002 → REQ-DE-001~005（DE L1）→ 本文（SSOT）→ 代码(@GOV)
本文 ←承接← A-SYS-I-P-01（字段/枚举单一来源）、A-AG-M-T-01（ChangeProposal 交接）
本文 →驱动→ A-DE-M-S-01（SM-DE-01 diff 终态机）→登记→ 总图（闭合 WS/AG 单侧转移点）
本文 →消费→ ED-EDIT/ED-DECOR（草稿应用、open session 自动应用与呈现）；未打开文档后台裁决信号 →可驱动→ SM-WS-02（apply 落盘）
```

**分层重申**：字段归 SYS、持久化与落盘归 WS、呈现归 ED、产出归 AG；本文拥有**转移语义、payload
形状、冲突与过期规则、裁决粒度**。

## §2 裁决粒度（解除 DE-01）

**裁决单元 = 内容块（Block）级**：每条待裁决修改以一个或多个完整内容块为裁决目标（blockId 锚，
BR-ED-DATA-001），接受/拒绝以整条 diff 为单位。块内细粒度对比（字符/词级高亮）是**呈现层增强**
（ED 装饰层渲染），不改变裁决单元。多块修改可拆分为多条 diff 以支持逐条裁决（拆分策略属 AG/DE
生成期协作，以"单条 diff 语义自洽可独立裁决"为原则）。

## §3 diff 生成（DE-DIFF）

<!-- CHAIN
chain_id: DE-DIFF
主责模块: DE
status: active
risk_tier: P1
-->

<!-- RULE
rule_id: BR-DE-DATA-001
主链路: DE-DIFF
域: DATA
-->
diff 生成必须满足：自修改建议（ChangeProposal）生成待裁决修改（PendingDiff），以 blockId 锚定
目标内容块；锚失效时生成失败并回报来源（AG handoffFail），不得产生错位或无锚 diff；生成后交 WS
持久化，持久化失败回统一错误形状且不进入裁决；PendingDiff 字段与 status 枚举集合
引用 A-SYS-I-P-01 单一来源（X-SHARED-001），DE 不重定义。

**跨会话边界（2026-07-21 收紧，APP-CLOSE-DIFF-LIFECYCLE-01）**：持久化保证的是当前运行期内
审阅呈现可完整重建，**不保证裁决权或未确认写盘效果跨会话存活**。任一新会话启动时，清算
（BR-SYS-STATE-005，A-SYS-M-T-02 §4）先于任何恢复执行：残留 `pending` 行统一转 `expired`
（`terminal_reason=abnormal_exit`），残留 `awaiting_save` 效果统一转 `discarded_on_abnormal_exit`；
之后仅终态记录进入只读历史呈现。pending 态只在产生它的连续运行期内可被裁决。

## §4 payload 载荷形状（解除 REQ-DE-004 占位）

`payload_ref` 所指载荷的结构（A-SYS-I-P-01 显式归 DE）：

| 字段 | 语义 | 备注 |
|---|---|---|
| payload_version | 载荷形状版本 | 单调递增，支持迁移 |
| target_blocks | 目标内容块定位（blockId 列表 + DocRef） | 锚集合；可细化至字符锚区间（CharAnchor，A-ED-M-T-01 §4.1，2026-07-03 批B） |
| before_snapshot | 修改前目标内容快照 | 外部变更检测基准（§6）+ 审阅对比基准 |
| after_content | 修改后内容 | 当前编辑器草稿应用或后台 apply 的执行物；纯样式 diff 时可空 |
| style_ops | 样式操作序列（StyleOp，A-ED-M-T-01 §5.1；2026-07-03 裁定 D-3 新增，可空） | 回合产出的样式修改载体，与 after_content 并存或独立 |
| snapshot_source | 生成基线来源 | active_editor / active_editor_session / open_editor_session / disk_snapshot；已打开文档不得为 disk_snapshot |
| base_session_version | 生成时 ED session 版本 | `snapshot_source=active_editor/active_editor_session/open_editor_session` 时必填 |
| base_content_hash | 生成时内容哈希 | `snapshot_source=active_editor/active_editor_session/open_editor_session` 时必填；用于自动应用与冲突重检 |
| origin | 来源元数据（origin_module、回合标识） | 可溯源到 AgentTurn |

实现约定：为兼容后台 `de.accept_diff` 的统一落盘路径，纯样式 diff 的 `after_content` 可在运行时写入
目标基准内容（即文本内容不变），样式语义由 `style_ops` 承载；审阅 UI 以 `style_ops` 重建样式摘要，
当前编辑器草稿路径以 `style_ops` 应用真实样式。

运行时视图投影约定：AG 的 `propose_edit` 必须在 `DiffProposed` 前通过 `ToolResult` 返回
`DiffProjection`，字段包括 `{diff_id, rel_path, patch_kind, block_id?, block_index?, before_text,
after_text, location_label, draft_mode, snapshot_source, base_session_version?, base_content_hash?,
style_ops?}`。`draft_mode` 仅允许 `current_editor / open_session / background`。DE/前端建 DiffCard 与 ED 草稿只能消费该投影，
不得用当前编辑器整篇内容截尾或 `tool_request.params` 重建 before/after。`replace_block` 的 before/after
必须来自同一目标 Block；`replace_document` 只在显式整篇替换时使用整篇基准。`draft_mode=open_session`
必须携带 session 基线元数据，且只能用于已打开非激活文档；`draft_mode=background` 只能用于未打开文档。

<!-- RULE
rule_id: BR-DE-DATA-002
主链路: DE-DIFF
域: DATA
-->
diff 载荷必须含：载荷版本（payload_version，形状变更须版本递增并兼容迁移）、目标内容块定位
（可细化至字符锚区间）、修改前内容快照、修改后内容（纯样式 diff 可空）、样式操作序列（可空，
StyleOp 语义归 ED BR-ED-DATA-005，本规则锁其在载荷中的承载与版本化）、生成基线来源、session 版本/内容哈希（已打开文档必填）、来源元数据；经 WS 持久化
并重启恢复后必须可完整重建审阅呈现（对比基准 = before_snapshot，含样式变更语义的可视对比）；
具体序列化格式延实现期，本规则锁字段语义集合。

## §5 终态裁决语义（DE-DIFF）

<!-- RULE
rule_id: BR-DE-STATE-001
主链路: DE-DIFF
域: STATE
-->
待裁决修改的终态转移必须满足：pending→accepted/rejected/expired 三终态互斥且不可逆（终态后
不得再转移、不得复活）。当前打开编辑器且可定位的块级 diff、显式 `replace_document` 整篇 diff、
StyleOp diff 在 pending 时即经 ED 受控接口把 after/样式操作应用为草稿并产生脏态；accepted 只确认该草稿，落盘由
用户 save 触发 SM-WS-01，save 成功后 ED 复位 GreenReview。已打开非激活文档的 `draft_mode=open_session`
diff 必须在 pending 后立即经 ED session 事务应用到目标 LogicalState：基线匹配则自动 accepted、DiffCard
禁用接受/拒绝；基线不匹配、事务不支持或应用失败则 expired/error 投影，不允许用户继续裁决该卡。
未打开文档的后台 diff 可继续走 accepted→SM-WS-02 apply 路径，但该路径仅适用于文本/.html 等可直接应用载荷的文档。既有 BDocx 的磁盘
权威是 docx 字节，DiffPayload 中的 HTML/Block 投影不得经后台 raw apply 写入 `.docx`；未在当前编辑器
HTML 草稿中成功应用的 BDocx diff 必须进入 error（如 `DE_BDOCX_DRAFT_REQUIRED`），等待用户打开目标
BDocx 后重新生成。
rejected/expired 不写文件；若草稿已应用，rejected 必须经 ED 受控接口按 before 回滚。用户裁决交互经
ED/DiffCard 转达（X-FORBID-005），DE 不绕过 ED 直接操纵编辑器。状态转移归 SM-DE-01。

### §5.1 效果态（解除审计发现：accepted 不等于已落盘的历史不一致，2026-07-20 新增）

`pending_diffs` 新增 `effect_status`（`not_applicable`/`awaiting_save`/`saved`/
`discarded_on_abnormal_exit`）与 `terminal_reason`（可空）两列，字段形状单一来源为 A-SYS-I-P-01，
语义规则 SSOT 为 `BR-SYS-DATA-005`（A-SYS-M-T-02 §4，APP-CLOSE-DIFF-LIFECYCLE-01）。核心动机：
`status=accepted` 只代表裁决结果，`current_editor`/`open_session` 路径的 accepted 在用户实际保存
成功前不代表内容已落盘（本文 §5 已锁定该语义），历史展示必须能区分"裁决同意"与"内容生效"两个
维度，不得笼统显示为完成态。

DE 侧责任：`de.accept_diff`/`de.accept_draft_diff` 成功时按路径写入 `effect_status`——`background`
路径与 `ws.apply_diff` 同一事务原子完成，写 `saved`；`current_editor`/`open_session` 路径写
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
目标内容块当前内容与 before_snapshot 不一致（经内容比对/文档 version_seq 检测）。外部变更检测
不可得时保守不自动过期，保持 pending 留人类裁决；expired 为终态、不落盘、装饰复位。超时时长
数值与检测触发时机（呈现时/裁决时/定时）延实现期标定，本规则锁判定条件与保守语义。

## §7 多 diff 并存与冲突（解除 DE-02）

<!-- RULE
rule_id: BR-DE-DATA-003
主链路: DE-DIFF
域: DATA
-->
同一文档多条待裁决修改并存时：呈现与裁决按创建序组织；**同一目标内容块不得并发草稿应用、open session 自动应用或后台
落盘**——先序 diff 改变编辑器草稿、ED session LogicalState 或后台落盘后，与其目标重叠的后续 pending diff 必须重新执行外部
变更检测（§6，before_snapshot 比对），内容已变更者转 expired（不静默用陈旧 after_content 覆盖新内容）；
互不重叠的 diff 可独立裁决。冲突判定以目标内容块集合的交集为准。

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
`draft_mode=open_session` 的 DiffCard 在自动应用成功后必须显示为已应用到 LogicalState 且禁用接受/拒绝；
自动应用失败时显示失效/错误原因并禁用接受/拒绝。该卡不得提供会触发二次应用的裁决入口。
批量裁决条是全局操作控件，可继续放在输入框上方；侧栏/内联整体布局仍属 DE-04 剩余 UI 债务，不在
本局部闭合范围内。

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

## §10 测试与验收

- **生成与锚**（BR-DE-DATA-001，REQ-DE-001 验收）：建议可生成为持久化 PendingDiff 且定位准确；
  锚失效生成失败回报 AG；无错位/无锚 diff。
- **载荷往返**（BR-DE-DATA-002，REQ-DE-004 验收）：持久化→重启→恢复后审阅呈现完整重建（对比
  基准一致）；payload_version 迁移不丢载荷。
- **终态语义**（BR-DE-STATE-001 / SM-DE-01，REQ-DE-002/003 验收）：三终态互斥不可逆；当前编辑器
  草稿路径 accepted 不直接落盘，save 成功后复位 GreenReview；后台文本/.html 路径可由 accepted 触发 apply，
  既有 BDocx 未进当前编辑器草稿时不得 raw apply；已打开非激活文档 `open_session` 自动应用成功后
  status=accepted 但不触发后台落盘，DiffCard 接受/拒绝禁用；基线不匹配时不得应用、不得继续裁决；
  rejected/expired 文件不变，草稿已应用时 rejected 回滚；ApplyError 不回退终态。
- **过期保守性**（BR-DE-STATE-002，REQ-DE-005 验收）：超时/外部变更转 expired；检测不可得不误杀。
- **冲突**（BR-DE-DATA-003）：重叠 diff 在先序草稿应用、open session 自动应用或后台落盘后重检，陈旧者 expired；
  不重叠者独立裁决；无并发应用/落盘同一块。
- **DiffCard 紧凑态**（TERM-DE-003，DE-04 局部闭合）：默认渲染为标题/位置/红删/绿增/操作五行；
  长内容、状态提示与样式摘要不无界撑高默认卡片，展开后可查看详情；多轮 DiffCard 按各自
  AgentTurn/assistant 消息锚点插入消息流，不沉底汇总；`open_session` 已应用/失效卡不可接受或拒绝。
- 跨模块（G5 联调）：AG HandingOff→DE 生成、DE pending→ED 草稿应用、DE accepted→ED/WS save
  或后台 SM-WS-02 apply、DE rejected→ED 回滚/复位三链对接。
- 量化 NFR（生成/呈现延迟）延 G5（DE-05）。

## §11 技术债务与盲区（诚实声明）

- **审阅 UI 形态剩余项**（DE-04）：逐条 DiffCard 默认紧凑形态已在 §8.1 局部闭合；批量裁决、侧栏/
  内联整体布局仍属产品体验，延实现期人类确认。
- **open_session DiffCard 视觉债务**：当前安全线是准确修改 LogicalState、禁用二次裁决并防止基线偏移；
  DiffCard 视觉仍可能只有绿增预览、缺少红删效果。后续需要补完整红删/绿增对比、DiffCard 失效态视觉和
  非激活文档撤销/回滚能力。
- **超时时长与检测触发时机**（DE-03 数值部分）：判定条件已锁，数值与触发时机延实现期标定。
- **diff 拆分策略**：多块修改拆分为可独立裁决单条的具体算法（语义自洽边界判定）延实现期，与 AG
  生成侧联合调优。
- **CBT 构建产物 diff 化**：构建（多文件批量产出）走 diff 路径的批量裁决形态联合 CBT G2。
- 块内细粒度对比的呈现算法（字符/词级 diff）属 ED 装饰层渲染实现，延实现期。

## §12 后续动作

1. 落地后随 `npm run governance:generate` 注册：DE-DIFF 链路、BR-DE-DATA-001/002/003、
   BR-DE-STATE-001/002 规则、TERM-DE-001/002、REQ_MAP 进 ADUS。
2. 续 A-DE-M-S-01（SM-DE-01 diff 终态机），回填总图 diff 终态行，闭合 SM-WS-02 accepted 入口与
   SM-AG-01 HandingOff 出口两个单侧转移点。
3. 待人类宣布 DE G2 闭合；CBT G2 联合构建产物 diff 化契约。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
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
