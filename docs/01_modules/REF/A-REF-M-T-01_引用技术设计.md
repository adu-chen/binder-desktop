---
文档编号：REF-M-T-01
文档状态：A
负责模块：REF
文档职责：REF 引用模块级 L2 技术设计（规则 SSOT）——@mention 触发与目标解析、引用锚（路径级/块级）与失效校验、注入载荷组装契约、状态机豁免复核；承接 REQ-REF-001~004
上游约束：A-REF-M-D-01_引用功能需求描述.md（REQ-REF-001~004、REF-01~05）、A-CORE-C-P-01_APC.md（§4.1、§6 豁免口径）、A-SYS-I-P-01_数据结构定义.md（DocRef、RefInjection 共享结构、X-SHARED-001）、A-WS-M-T-01_工作台技术设计.md（BR-WS-DATA-001 读取经 WS、X-FORBID-004）、A-ED-M-T-01_编辑器技术设计.md（BR-ED-DATA-001 blockId 失效信号、扩展表面、X-FORBID-005）、A-AG-M-T-01_对话代理技术设计.md（BR-AG-DATA-001 注入消费与裁剪优先级）、ADU.md（§3）
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

## §1 文档定位

```
REQ-CORE-001/006 → REQ-REF-001~004（REF L1）→ 本文（SSOT）→ 代码(@GOV)
本文 ←结构基线← A-SYS-I-P-01 §3 RefInjection（X-SHARED-001，本轮登记）
本文 ←锚契约← A-ED-M-T-01（blockId 失效信号）；本文载荷 →供→ AG（BR-AG-DATA-001 消费）/ CBT（随其 G2）
```

REF 无状态机：豁免复核结论见 §6（解除 REF-04）。

## §2 引用目标模型（v1 范围）

v1 可引用目标 = workspace 内**文档**（路径级锚）、**内容块**（块级锚：DocRef + blockId）、
**资源文件**（路径级锚）。知识库条目 / 记忆资产是否纳入待 KB/AST G1（REF-01 保持开放，
本文不预占其解析协议）。候选列举与内容读取一律经 WS（X-FORBID-004 限界）。

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

## §8 测试与验收

- **触发与解析**（BR-REF-DATA-001，REQ-REF-001/004 验收）：@ 触发列举限定 workspace 内；目标
  不可达明确失败、无悬空引用；触发/候选/标记均经 ED 扩展点（静态可查无绕过路径）。
- **载荷一致与溯源**（BR-REF-DATA-002，REQ-REF-002 验收）：载荷内容与读取时点目标一致；来源标识
  可溯源；结构与 A-SYS-I-P-01 RefInjection 一致无第二定义；越界读取被拒。
- **锚失效保守性**（BR-REF-VERIFY-001，REQ-REF-003 验收）：删除/移动目标后注入收到失效信号；
  不出现静默注入空/过期/错位内容；检测不可得不注入。
- **消费联调（G5）**：AG 组装消费（显式注入优先保留）、失效信号传导至用户可重选。
- 量化 NFR（候选列举/解析延迟）延 G5（REF-05）。

## §9 技术债务与盲区（诚实声明）

- **可引用范围扩展**（REF-01）：KB 条目 / AST 记忆纳入待其 G1 联合，届时扩展 §2 目标模型与
  RefInjection（回写 A-SYS-I-P-01）。
- **失效修复交互**（REF-03）：重选/自动重定位形态延实现期。
- **候选排序与搜索匹配算法**属实现期（不入规则）。
- 豁免结论的失效触发条件已列（§6），任一出现须复核建机。

## §10 后续动作

1. 落地后随 `npm run governance:generate` 注册：REF-INJECT 链路、BR-REF-DATA-001/002、
   BR-REF-VERIFY-001 规则、TERM-REF-001/002、REQ_MAP 进 ADUS（A-SYS-I-P-01 v1.1 的 RefInjection
   同步登记）。
2. 待人类宣布 REF G2 闭合——至此 P3（AG/DE/REF）L1+L2 文档层齐备。
3. KB/AST G1 时联合解除 REF-01；CBT G2 承接构建上下文消费。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-11 | v1.0 | 初版（REF G2，P3 G2 收尾件）：REF 规则 SSOT——v1 引用目标模型（文档/内容块/资源）、触发与解析（无悬空引用、经 ED 扩展点）、注入载荷契约（RefInjection 登记 SYS 契约层解除 REF-02、读取时点一致、来源可溯、裁剪归 AG）、锚失效校验（保守不注入）；状态机豁免复核成立并列失效触发条件（解除 REF-04）；注册链路 REF-INJECT（P1）、规则 BR-REF-DATA-001/002·VERIFY-001、术语 TERM-REF-001 ReferenceAnchor / TERM-REF-002 InjectionPayload；约束全复用不新增；REQ_MAP（REQ-REF-001~004） |
| 2026-07-03 | v1.1 | 批B 选区级锚承接（A-ED-X-L-02 PI-ED-10 文档先行）：§2 目标模型增选区级锚（块级锚细化 = blockId + 块内字符区间，SelectionRef/CharAnchor 基准归 ED，有效期=编辑会话内，失效同 §5 保守语义）；§4 登记 RefInjection 区间字段扩展意向（block_ranges，按 X-SHARED-001 须回写 A-SYS-I-P-01 后进代码） |
