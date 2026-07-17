---
文档编号：REF-M-T-01
文档状态：A
负责模块：REF
文档职责：REF 引用模块级 L2 技术设计（规则 SSOT）——@mention 触发与目标解析、引用锚（路径级/块级）与失效校验、注入载荷组装契约、状态机豁免复核；承接 REQ-REF-001~004
上游约束：A-REF-M-D-01_引用功能需求描述.md（REQ-REF-001~004、REF-01~05）、A-CORE-C-P-01_APC.md（§4.1、§6 豁免口径）、A-SYS-I-P-01_数据结构定义.md（DocRef、RefInjection 共享结构、X-SHARED-001）、A-WS-M-T-01_工作台技术设计.md（BR-WS-DATA-001 读取经 WS、X-FORBID-004）、A-ED-M-T-01_编辑器技术设计.md（BR-ED-DATA-001 blockId 失效信号、扩展表面、X-FORBID-005）、A-AG-M-T-01_代理技术设计.md（BR-AG-DATA-001 注入消费与裁剪优先级）、ADU.md（§3）
直接承接：本文 RULE/CHAIN/TERM 为 REF 规则 SSOT，代码经 @GOV 映射；注入载荷结构已登记 A-SYS-I-P-01 §3（RefInjection，解除 REF-02）；载荷被 AG（回合组装）与 CBT（构建上下文，随其 G2）消费
使用边界：本文是 REF 规则 SSOT；不组装回合上下文与预算裁剪（归 AG，BR-AG-DATA-001）、不做文件 I/O（经 WS）、不承载呈现（经 ED 扩展表面）、不检索知识库/记忆（归 KB/AST，可引用范围 REF-01 待其 G1）；本模块无状态机（豁免复核见 §6）
变更要求：RefInjection 字段集合变更须回写 A-SYS-I-P-01（X-SHARED-001）并复核 AG/CBT 消费方；锚失效语义变更须与 ED（blockId 契约）联合复核；TERM 一经 term_ref 引用其 en 不可更改
治理成熟度：draft
---

# Binder 引用技术设计（REF · L2 技术设计）

## 0. Terminology Registry

<!-- TERM
term_id: TERM-REF-001
chains: REF-INJECT
zh: 引用锚
en: ReferenceAnchor
forbidden: 引用链接, 引用指针, mention锚, 引用定位符
-->
@mention 解析产物：指向 workspace 内目标的定位结构——路径级（DocRef）或块级（DocRef +
blockId）；注入前必须校验有效性，失效发明确信号（REQ-REF-001/003）。

<!-- TERM
term_id: TERM-REF-002
chains: REF-INJECT
zh: 注入载荷
en: InjectionPayload
forbidden: 上下文载荷, 注入内容包, 引用载荷, 引用内容体
-->
REF 经 WS 读取引用目标当前内容组装、交 AG/CBT 消费的内容载体（含来源标识可溯源）；跨模块共享
结构基线为 A-SYS-I-P-01 的 RefInjection（REQ-REF-002）。

<!-- TERM
term_id: TERM-REF-003
chains: REF-INJECT
zh: 引用输入载荷
en: ReferencePayload
forbidden: 引用内容包, 引用输入包, ref payload, mention payload
-->
拖拽、粘贴或 @mention 选定后在前端输入面流转的统一数据结构；覆盖文档、选区、会话、记忆、知识、
模板、外部 URL 与长文本等来源。它是用户显式选中内容的快照或锚，不等同于发送时经 WS/AG 组装后的
InjectionPayload。

<!-- TERM
term_id: TERM-REF-004
chains: REF-INJECT
zh: 引用令牌
en: ReferenceToken
forbidden: 引用标签, 输入标签, inline ref chip, 引用片
-->
ReferencePayload 在输入流和已发送消息中的原子可见标记；发送前与草稿 chip/tray 同步，发送后保留
同形态可读表现，作为消息内容的一部分参与回放。

## §1 文档定位

```
REQ-CORE-001/006 → REQ-REF-001~004（REF L1）→ 本文（SSOT）→ 代码(@GOV)
本文 ←结构基线← A-SYS-I-P-01 §3 RefInjection（X-SHARED-001，本轮登记）
本文 ←锚契约← A-ED-M-T-01（blockId 失效信号）；本文载荷 →供→ AG（BR-AG-DATA-001 消费）/ CBT（随其 G2）
```

REF 无状态机：豁免复核结论见 §6（解除 REF-04）。

## §2 引用目标模型（v1 范围）

v1 可引用目标 = workspace 内**文档**（路径级锚）、**内容块**（块级锚：DocRef + blockId）、
**资源文件**（路径级锚）、**代理会话**、**记忆条目**、**知识条目**、**模板条目**、**外部 URL**、
**外部长文本**。候选列举与 workspace 内容读取一律经 WS（X-FORBID-004 限界）；AST/KB/TPL
条目的生成、筛选、检索和管理归各 owner，REF 只接收用户已选中的 ReferencePayload 快照。

**选区级锚（2026-07-03 批B 扩展，A-ED-X-L-02 PI-ED-10）**：块级锚的细化——DocRef + blockId +
块内字符区间 {start,end}（SelectionRef，TERM-ED-005 / CharAnchor 基准，A-ED-M-T-01 §4.1）。
产生入口 = 编辑器选区拖拽入 AG 输入面；**有效期 = 编辑会话内**（BR-ED-DATA-004：文档重开后
blockId 重建、区间锚失效），失效校验同 §5 保守语义（失效不注入、明确提示）。

## §3 触发与目标解析（REF-INJECT）

<!-- CHAIN
chain_id: REF-INJECT
主责模块: REF
status: active
risk_tier: P1
-->

<!-- RULE
rule_id: BR-REF-DATA-001
主链路: REF-INJECT
域: DATA
-->
@mention 触发与解析必须满足：候选列举限定 workspace 内（经 WS，X-FORBID-004）；解析产物为引用锚
（ReferenceAnchor：路径级 DocRef 或块级 DocRef+blockId）；目标不可达时解析失败并明确回报，不产生
悬空引用；触发输入面、候选面板与引用标记渲染必经 ED 声明的扩展点（X-FORBID-005），REF 不直接
操纵编辑器内部状态。

## §4 注入载荷契约（REF-INJECT）

注入载荷的跨模块共享结构基线已登记 **A-SYS-I-P-01 §3 RefInjection**（X-SHARED-001，解除 REF-02）：
target（DocRef）、block_ids（可空，块级锚集合）、content（读取时点的目标内容）、origin_label
（来源标识，可溯源到引用目标）。字段集合归 SYS 单一来源，本文锁组装与一致性语义。

> **选区级锚字段扩展登记（2026-07-03，PI-ED-10 实现时执行）**：选区引用需 RefInjection 增可空
> 区间字段（如 `block_ranges: [{blockId, start, end}]`）——按 X-SHARED-001 该扩展**须回写
> A-SYS-I-P-01 落字段后方可进代码**，本文只登记语义需求，不私自扩展跨模块字段。

<!-- RULE
rule_id: BR-REF-DATA-002
主链路: REF-INJECT
域: DATA
-->
注入载荷组装必须满足：内容经 WS 读取引用目标在**读取时点**的当前内容（不缓存陈旧内容跨回合复用）；
载荷含来源标识，消费方（AG/CBT）可溯源到引用目标；载荷结构引用 A-SYS-I-P-01 RefInjection
（X-SHARED-001），REF 不私自扩展跨模块字段；读取越界路径由 WS 拒绝（X-FORBID-004）。
上下文预算与裁剪归 AG（BR-AG-DATA-001：显式注入优先保留），REF 不做裁剪。

<!-- RULE
rule_id: BR-REF-DATA-003
主链路: REF-INJECT
域: DATA
-->
输入侧多来源引用必须归一为 ReferencePayload：`doc`/`selection` 保留 workspace 相对路径与可校验锚；
`session` 保留会话标识、标题与发送时可用的消息快照；`memory`/`knowledge`/`template` 保留 owner 条目
标识、标题与内容快照；`url` 仅保存外部链接与可选标题，不自动联网抓取；`long_text` 保存粘贴文本
快照与字符数。外部纯文本超过实现阈值时必须生成 `long_text` ReferencePayload，而非直接注入输入
正文；不可识别内容按普通文本处理。

<!-- RULE
rule_id: BR-REF-DATA-004
主链路: REF-INJECT
域: DATA
-->
ReferencePayload 在输入面必须插入输入流内 ReferenceToken，并同步生成草稿引用 chip/tray；草稿引用
超过 5 条时折叠显示，但输入流内 ReferenceToken 不折叠、不丢失；发送后草稿 chip/tray 生命周期终止，
已发送消息保留同一 ReferenceToken 可读表现。纯 ReferenceToken 消息可发送；发送时 AG 消费
ReferencePayload 集合，不能只发送视觉标签。

## §5 锚失效校验（REF-INJECT）

<!-- RULE
rule_id: BR-REF-VERIFY-001
主链路: REF-INJECT
域: VERIFY
-->
注入前必须校验引用锚有效性：路径级锚校验目标存在于 workspace；块级锚经 ED blockId 定位
（BR-ED-DATA-001 失效信号）。锚失效时发明确失效信号（消费方可示用户重选），**不得静默注入空内容、
过期内容或错位内容**；有效性检测不可得时按失效保守处理（不注入）。失效后的修复交互（重选/自动
重定位）延实现期（REF-03），本规则锁校验时机与保守语义。

## §6 状态机豁免复核（解除 REF-04）

**结论：豁免成立，REF v1 不建状态机。** 复核依据（APC §6 强制阈值口径 b）：

- @mention 解析注入为**请求-响应式**：触发→候选→解析→（回合发起时）读取组装，每步同步完成、
  无跨回合持久业务状态（引用锚本身是数据非状态机）、无多步异步生命周期、无跨模块状态耦合
  （锚校验是同步判定，不是状态转移）。
- 候选面板的开/关属 ED 呈现层交互（纯展示豁免）；载荷组装失败走统一错误形状即时回报，无恢复路径
  编排需求。

**豁免失效触发条件**（任一出现须建机并登记总图）：引入异步批量解析/索引重建编排；引入引用锚的
后台失效扫描与自动修复流程；KB/AST 纳入后出现跨模块检索会话状态。届时按 X-SEQ-003 先建机后实现。

## §7 需求到规则映射（REQ_MAP）

<!-- REQ_MAP
req_id: REQ-REF-001
target_type: CHAIN
target_id: REF-INJECT
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-REF-001
target_type: RULE
target_id: BR-REF-DATA-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-REF-002
target_type: RULE
target_id: BR-REF-DATA-002
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-REF-003
target_type: RULE
target_id: BR-REF-VERIFY-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-REF-004
target_type: RULE
target_id: BR-REF-DATA-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-REF-005
target_type: RULE
target_id: BR-REF-DATA-003
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-REF-006
target_type: RULE
target_id: BR-REF-DATA-004
status: mapped
-->

## §8 测试与验收

- **触发与解析**（BR-REF-DATA-001，REQ-REF-001/004 验收）：@ 触发列举限定 workspace 内；目标
  不可达明确失败、无悬空引用；触发/候选/标记均经 ED 扩展点（静态可查无绕过路径）。
- **载荷一致与溯源**（BR-REF-DATA-002，REQ-REF-002 验收）：载荷内容与读取时点目标一致；来源标识
  可溯源；结构与 A-SYS-I-P-01 RefInjection 一致无第二定义；越界读取被拒。
- **多来源归一**（BR-REF-DATA-003，REQ-REF-005 验收）：编辑器选区、文件树、编辑器标签、会话标签、
  记忆/知识/模板条目、外部 URL、长文本粘贴均能形成 ReferencePayload；普通短文本不被误识别。
- **输入流一致**（BR-REF-DATA-004，REQ-REF-006 验收）：发送前输入流内有 ReferenceToken，草稿区
  超过 5 条折叠；纯 ReferenceToken 可发送；发送后 composer 草稿引用清空，消息内令牌保留。
- **锚失效保守性**（BR-REF-VERIFY-001，REQ-REF-003 验收）：删除/移动目标后注入收到失效信号；
  不出现静默注入空/过期/错位内容；检测不可得不注入。
- **消费联调（G5）**：AG 组装消费（显式注入优先保留）、失效信号传导至用户可重选。
- 量化 NFR（候选列举/解析延迟）延 G5（REF-05）。

## §9 技术债务与盲区（诚实声明）

- **拖拽引用入口未闭环（REF-06，2026-07-08 暂停优化）**：复制编辑器选区并粘贴到 Composer
  已可经 clipboard MIME 形成 ReferencePayload / ReferenceToken；但拖拽路径在真实界面中仍未稳定形成
  ReferenceToken，本轮停止继续优化，整体登记为实现债务。

  已做尝试：
  1. 建立统一 `ReferencePayload` / `ReferenceToken` 协议，覆盖 `doc` / `selection` / `session` /
     `memory` / `knowledge` / `template` / `url` / `long_text`。
  2. 文件树、编辑器标签、会话标签和编辑器选区源头均尝试通过 `DataTransfer` 写入
     `application/x-binder-reference`；文档/选区同时保留旧 MIME 兼容。
  3. Composer 从 textarea 改为 TipTap inline atom 承载 ReferenceToken，并在输入流内插入标签；
     草稿 chip/tray 只做摘要。
  4. Composer 拖拽接收改为 capture 阶段处理，补 `preventDefault` / `stopPropagation`，并兼容
     DOMStringList、array-like 等不同 `DataTransfer.types` 形态。
  5. 增加前端单测覆盖 MIME 读写、DOMStringList 识别、drop 落在 Composer TipTap 内容区仍生成
     ReferenceToken；这些测试通过，但未覆盖真实 Tauri/WebKit 拖拽事件链。

  预计根因与分流判断：
  - **编辑器选区拖拽**：ED `dragstart` 先写入 `writeSelectionRefData` 后返回 `false`，ProseMirror
    内置 `dragstart` 随后执行 `DataTransfer.clearData()` 并仅重写 `text/html` / `text/plain`，
    导致 Composer drop 阶段读不到 `application/x-binder-selection-ref` 或
    `application/x-binder-reference`。这是已读源码确认的高概率根因。
  - **文件树 / 编辑器标签 / 会话标签拖拽**：源头已尝试写入统一 MIME；若真实界面仍失败，优先怀疑
    Tauri/WebKit 实际 drop 事件中 custom `application/x-*` MIME 不可见、被清洗，或 `types`
    与 `getData` 行为和 jsdom 单测不同。当前 `text/plain` 兜底只允许 URL/长文本进入引用，
    不会把普通路径或会话标题误识别为引用，因此 custom MIME 丢失时不会形成标签。
  - **记忆 / 知识 / 模板拖拽**：协议已定义，owner UI 来源端尚未完整接线；若实现，应同样走
    `ReferencePayload`，但在真实拖拽链路未确认前不应继续堆实现。
  - **外部 URL / 长文本拖拽**：Composer 已按 `text/uri-list` / `text/plain` 做识别设计；真实
    WebKit drop 行为仍需实测。

  后续重启条件：先做真实 Tauri/WebKit 事件观测（dragstart / dragover / drop 三点记录
  `types`、各 MIME `getData`、`defaultPrevented`、事件目标），再决定方案。候选方案包括：
  ED 源头在 ProseMirror 内置 `dragstart` 后置写回 MIME、专用 SelectionRef 拖动手柄、或放弃
  custom MIME 改用应用内临时 drag session store。未完成该观测前，不继续修拖拽引用。
- **资产面板来源接线**：AST/KB/TPL 条目 UI 若尚未暴露拖拽入口，必须按 BR-REF-DATA-003 输出
  ReferencePayload，不改变 REF 协议。
- **失效修复交互**（REF-03）：重选/自动重定位形态延实现期。
- **候选排序与搜索匹配算法**属实现期（不入规则）。
- 豁免结论的失效触发条件已列（§6），任一出现须复核建机。

## §10 后续动作

1. 落地后随 `npm run governance:generate` 注册：REF-INJECT 链路、BR-REF-DATA-001/002/003/004、
   BR-REF-VERIFY-001 规则、TERM-REF-001/002/003/004、REQ_MAP 进 ADUS（A-SYS-I-P-01 v1.1 的
   RefInjection 同步登记）。
2. 待人类宣布 REF G2 闭合——至此 P3（AG/DE/REF）L1+L2 文档层齐备。
3. KB/AST G1 时联合解除 REF-01；CBT G2 承接构建上下文消费。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-11 | v1.0 | 初版（REF G2，P3 G2 收尾件）：REF 规则 SSOT——v1 引用目标模型（文档/内容块/资源）、触发与解析（无悬空引用、经 ED 扩展点）、注入载荷契约（RefInjection 登记 SYS 契约层解除 REF-02、读取时点一致、来源可溯、裁剪归 AG）、锚失效校验（保守不注入）；状态机豁免复核成立并列失效触发条件（解除 REF-04）；注册链路 REF-INJECT（P1）、规则 BR-REF-DATA-001/002·VERIFY-001、术语 TERM-REF-001 ReferenceAnchor / TERM-REF-002 InjectionPayload；约束全复用不新增；REQ_MAP（REQ-REF-001~004） |
| 2026-07-03 | v1.1 | 批B 选区级锚承接（A-ED-X-L-02 PI-ED-10 文档先行）：§2 目标模型增选区级锚（块级锚细化 = blockId + 块内字符区间，SelectionRef/CharAnchor 基准归 ED，有效期=编辑会话内，失效同 §5 保守语义）；§4 登记 RefInjection 区间字段扩展意向（block_ranges，按 X-SHARED-001 须回写 A-SYS-I-P-01 后进代码） |
| 2026-07-04 | v1.2 | 上游约束文件名指针更新（A-AG-*/A-DE-* 文件更名为 代理*/DiffEdit*，2026-07-04 人类确认；文档编号不变，无语义变更） |
| 2026-07-08 | v1.3 | 引用令牌修复设计：新增 TERM-REF-003 ReferencePayload 与 TERM-REF-004 ReferenceToken；目标模型扩到会话、记忆、知识、模板、外部 URL、长文本；新增 BR-REF-DATA-003/004 覆盖多来源归一、输入流令牌、草稿折叠与发送后一致性；映射 REQ-REF-005/006 |
| 2026-07-08 | v1.4 | 登记 SelectionRef 拖拽入口未闭环债务：复制/粘贴路径已可形成 ReferenceToken，拖拽到 Composer 仍未稳定闭合；记录 ProseMirror 清理 DataTransfer 自定义 MIME 的疑似根因，本轮暂停继续优化 |
| 2026-07-08 | v1.5 | 扩展拖拽引用债务登记为 REF-06：记录全部拖拽引用入口的尝试、现状、预计根因与后续重启条件；明确未完成真实 Tauri/WebKit 事件观测前不再继续优化拖拽引用 |
