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
由 AG 修改建议生成、经 WS 持久化、等待人类裁决的修改单元（共享结构归 A-SYS-I-P-01）；
status 枚举 pending/accepted/rejected/expired，终态互斥不可逆，仅 accepted 触发落盘（REQ-DE-003）。

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
is_new_doc），是 accepted 落盘时 `ws.apply_diff` 消费的内容来源。与 ChangeProposal（AG 交接的意图
载体）、PendingDiff（等待裁决的控制态记录）、DiffCard（对话区视图卡）严格区分：DiffPayload 是
**落盘内容**的单一形状（BR-DE-DATA-002），经 payload_ref 由 PendingDiff 关联，持久化于 workspace.db
`diff_payloads` 表（DE 拥有形状、WS db 承载持久化，见 §1 分层）。

## §1 文档定位

```
REQ-CORE-002 → REQ-DE-001~005（DE L1）→ 本文（SSOT）→ 代码(@GOV)
本文 ←承接← A-SYS-I-P-01（字段/枚举单一来源）、A-AG-M-T-01（ChangeProposal 交接）
本文 →驱动→ A-DE-M-S-01（SM-DE-01 diff 终态机）→登记→ 总图（闭合 WS/AG 单侧转移点）
本文 →消费→ ED-DECOR（呈现）；本文裁决信号 →驱动→ SM-WS-02（落盘）
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
持久化（重启可恢复），持久化失败回统一错误形状且不进入裁决；PendingDiff 字段与 status 枚举集合
引用 A-SYS-I-P-01 单一来源（X-SHARED-001），DE 不重定义。

## §4 payload 载荷形状（解除 REQ-DE-004 占位）

`payload_ref` 所指载荷的结构（A-SYS-I-P-01 显式归 DE）：

| 字段 | 语义 | 备注 |
|---|---|---|
| payload_version | 载荷形状版本 | 单调递增，支持迁移 |
| target_blocks | 目标内容块定位（blockId 列表 + DocRef） | 锚集合；可细化至字符锚区间（CharAnchor，A-ED-M-T-01 §4.1，2026-07-03 批B） |
| before_snapshot | 修改前目标内容快照 | 外部变更检测基准（§6）+ 审阅对比基准 |
| after_content | 修改后内容 | 落盘执行物（WS 消费）；纯样式 diff 时可空 |
| style_ops | 样式操作序列（StyleOp，A-ED-M-T-01 §5.1；2026-07-03 裁定 D-3 新增，可空） | 回合产出的样式修改载体，与 after_content 并存或独立 |
| origin | 来源元数据（origin_module、回合标识） | 可溯源到 AgentTurn |

<!-- RULE
rule_id: BR-DE-DATA-002
主链路: DE-DIFF
域: DATA
-->
diff 载荷必须含：载荷版本（payload_version，形状变更须版本递增并兼容迁移）、目标内容块定位
（可细化至字符锚区间）、修改前内容快照、修改后内容（纯样式 diff 可空）、样式操作序列（可空，
StyleOp 语义归 ED BR-ED-DATA-005，本规则锁其在载荷中的承载与版本化）、来源元数据；经 WS 持久化
并重启恢复后必须可完整重建审阅呈现（对比基准 = before_snapshot，含样式变更语义的可视对比）；
具体序列化格式延实现期，本规则锁字段语义集合。

## §5 终态裁决语义（DE-DIFF）

<!-- RULE
rule_id: BR-DE-STATE-001
主链路: DE-DIFF
域: STATE
-->
待裁决修改的终态转移必须满足：pending→accepted/rejected/expired 三终态互斥且不可逆（终态后
不得再转移、不得复活）；**accepted 是 WS 落盘的唯一触发**（驱动 SM-WS-02，X-INTER-001），并接收
落盘回执（Applied 后通知 ED 复位装饰；ApplyError 时 diff 保持 accepted、由 WS 侧重试，DE 不回退
终态）；rejected/expired 通知 WS 不落盘、通知 ED 复位装饰，文件保持不变。用户裁决交互经 ED
装饰层转达（X-FORBID-005），DE 不绕过 ED 直接操纵编辑器。状态转移归 SM-DE-01。

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
同一文档多条待裁决修改并存时：呈现与裁决按创建序组织；**同一目标内容块不得并发落盘**——先序
diff accepted 落盘后，与其目标重叠的后续 pending diff 必须重新执行外部变更检测（§6，
before_snapshot 比对），内容已变更者转 expired（不静默落用陈旧 after_content 覆盖新内容）；
互不重叠的 diff 可独立裁决落盘。冲突判定以目标内容块集合的交集为准。

## §8 跨模块约束（复用，不新增）

本文不新增 CONSTRAINT：唯一落盘路径已由 **X-FORBID-006**（A-AG-M-T-01）锁定、共享结构由
**X-SHARED-001** 锁定、呈现通道由 **X-FORBID-005** 锁定、建机前置与转移登记由
**X-SEQ-003 / X-INTER-001** 锁定；本文 §3–§7 在 DE 侧落实之，避免重复定义同义约束（ADU §3.1）。

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
- **终态语义**（BR-DE-STATE-001 / SM-DE-01，REQ-DE-002/003 验收）：三终态互斥不可逆；仅 accepted
  触发 SM-WS-02；rejected/expired 文件不变 + 装饰复位；ApplyError 不回退终态。
- **过期保守性**（BR-DE-STATE-002，REQ-DE-005 验收）：超时/外部变更转 expired；检测不可得不误杀。
- **冲突**（BR-DE-DATA-003）：重叠 diff 在先序落盘后重检，陈旧者 expired；不重叠者独立落盘；
  无并发落盘同一块。
- 跨模块（G5 联调）：AG HandingOff→DE 生成、DE accepted→SM-WS-02、DE↔ED 装饰呈现/复位三链对接。
- 量化 NFR（生成/呈现延迟）延 G5（DE-05）。

## §11 技术债务与盲区（诚实声明）

- **审阅 UI 形态**（DE-04，待人类决策）：逐条/批量裁决、侧栏/内联呈现属产品体验，延实现期人类
  确认；本文规则对 UI 形态中立。
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
| 2026-06-11 | v1.0 | 初版（DE G2 首件）：DE 规则 SSOT——裁决粒度定为内容块级（解除 DE-01）、diff 生成与锚失效回报、payload 载荷形状（版本化 + before_snapshot 基准，解除 REQ-DE-004 占位）、终态裁决语义（三终态互斥不可逆、accepted 唯一触发落盘、ApplyError 不回退）、过期与外部变更（保守语义，解除 DE-03 机制）、多 diff 冲突（重叠重检、无并发落盘，解除 DE-02）；注册链路 DE-DIFF（P1）、规则 5 条、术语 TERM-DE-001 PendingDiff / TERM-DE-002 ChangeProposal（解除 A-SYS-I-P-01 §6 延期）；约束全复用不新增；REQ_MAP（REQ-DE-001~005）；债务（审阅 UI 待人类、数值延实现、CBT 联合） |
| 2026-07-03 | v1.1 | 批B 样式 diff 承接（A-ED-X-L-02 PI-ED-11 文档先行，人类裁定 D-3 方案A：回合样式修改经 DE 裁决）：§4 载荷表 +style_ops（StyleOp 序列，可空；语义归 ED BR-ED-DATA-005，载荷承载与版本化归本文）、after_content 纯样式 diff 可空、target_blocks 可细化字符锚区间；BR-DE-DATA-002 规则文本同步（含样式变更语义的可视对比重建）；DiffCard 样式呈现形态延实现期（DE-04 审阅 UI 债务口径） |
| 2026-07-04 | v1.2 | 术语统一清扫（承 2026-07-04 人类三轮裁决）：正文禁用别名替换——内联辅助→局部修改、自动补全→辅助续写、对话编辑→Diff Edit、Chat↔Build 接管→构建移交/回落、chat/chat-agent 回合→代理回合；文件名与变更记录历史行不改写 |
