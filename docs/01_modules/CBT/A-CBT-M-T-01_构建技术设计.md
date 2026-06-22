---
文档编号：CBT-M-T-01
文档状态：A
负责模块：CBT
文档职责：CBT 构建模块级 L2 技术设计（规则 SSOT）——构建回合与任务分解模型、产物 diff 化（含新建文档表达）、构建中断与控制态持久化、Chat↔Build 接管协议；承接 REQ-CBT-001~004
上游约束：A-CBT-M-D-01_构建功能需求描述.md（REQ-CBT-001~004、CBT-01~05）、A-CORE-C-P-01_APC.md（§4.1/§6）、A-AG-M-T-01_对话代理技术设计.md（BR-AG-VERIFY-001 门禁、SM-AG-01/02 基建、X-FORBID-006）、A-DE-M-T-01_对话编辑技术设计.md（ChangeProposal、BR-DE-DATA-002 载荷、批量裁决债务）、A-WS-M-T-01_工作台技术设计.md（X-FORBID-004、控制态持久化）、A-SYS-I-P-01_数据结构定义.md（X-SHARED-001、BR-SYS-PERSIST-001）、A-SYS-M-S-01_状态机总图.md（CBT 构建核心链、Chat↔Build 转移点）、ADU.md（§3）
直接承接：本文 RULE/CHAIN/TERM 为 CBT 规则 SSOT，代码经 @GOV 映射；SM-CBT-01 在 A-CBT-M-S-01 定义并登记总图；接管协议供 AG 对接（闭合总图 Chat↔Build 转移点）
使用边界：本文是 CBT 规则 SSOT；不自带模型调用通道（经 AG）、不定义 diff 终态（DE）、不落盘（WS）；任务分解的提示工程与模型策略延实现期；批量裁决 UI 形态联动 DE-04 待人类；状态机内部归 A-CBT-M-S-01
变更要求：接管协议变更须与 AG 共同复核（X-INTER-001）；产物 diff 化契约变更须与 DE 复核；控制态结构变更遵守 BR-SYS-PERSIST-001；TERM 一经 term_ref 引用其 en 不可更改
治理成熟度：draft
---

# Binder 构建技术设计（CBT · L2 技术设计）

## 0. Terminology Registry

<!-- TERM
term_id: TERM-CBT-001
chains: CBT-BUILD
zh: 构建回合
en: BuildRun
forbidden: 构建会话, 构建过程, build流程, 项目构建轮
-->
一次多文件文档项目构建的完整生命周期：意图分解→任务计划→逐任务产出→产物裁决；可中断、可与
chat 互相接管（REQ-CBT-001/003/004）。

<!-- TERM
term_id: TERM-CBT-002
chains: CBT-BUILD
zh: 构建任务
en: BuildTask
forbidden: 构建步骤, 子任务, build单元, 任务项
-->
构建回合内的最小执行单元：一个目标文件维度的产出动作（新建/修改），逐任务经 AG 回合基建执行，
产物以修改建议（ChangeProposal）形式提交 DE。

## §1 文档定位

```
REQ-CORE-003 → REQ-CBT-001~004（CBT L1）→ 本文（SSOT）→ 代码(@GOV)
本文 ←基建← A-AG-M-T-01（门禁/回合/工具）；产物 →交→ DE（ChangeProposal）→ WS（落盘）
本文 →驱动→ A-CBT-M-S-01（SM-CBT-01）→登记→ 总图（CBT 行 + Chat↔Build 转移点闭合）
```

**前提重申**：构建的一切模型调用经 AG 基建过门禁（BR-AG-VERIFY-001）；一切产物落盘经
DE 裁决→WS（X-FORBID-006）；CBT 自身无模型通道、无写盘通道。

## §2 构建回合与任务分解（CBT-BUILD）

**模型**：构建意图 → 任务计划（BuildTask 有序集，每任务=一个目标文件的产出动作）→ 逐任务
执行（经 SM-AG-01/02）→ 产物逐任务/批量提交裁决。任务计划对用户可见、执行前可调整；计划本身
不落盘文件（控制态，§4）。任务分解的提示工程与模型策略属实现期，本文锁结构。

<!-- CHAIN
chain_id: CBT-BUILD
主责模块: CBT
status: active
risk_tier: P1
-->

<!-- RULE
rule_id: BR-CBT-STATE-001
主链路: CBT-BUILD
域: STATE
-->
构建回合必须经完整生命周期：意图分解为对用户可见的任务计划→逐任务经 AG 回合基建执行（每次
模型调用过门禁，BR-AG-VERIFY-001）→产物经 DE 裁决落盘；回合可随时中断，中断为终态且无半成品
落盘（未裁决产物按 pending diff 留存或随中断丢弃，由用户选择）；任务失败不静默跳过（暂停待
处置）。状态转移归 SM-CBT-01（A-CBT-M-S-01）。

## §3 产物 diff 化（CBT-BUILD，解除 CBT-02 契约部分）

<!-- RULE
rule_id: BR-CBT-DATA-001
主链路: CBT-BUILD
域: DATA
-->
构建产物必须逐任务生成修改建议（ChangeProposal）交 DE 生成待裁决修改，不存在旁路写盘
（X-FORBID-006）：修改既有文档按 DE 标准载荷（含 before_snapshot）；**新建文档以空基准表达**
（before_snapshot=空、target 为计划新建的 DocRef，accepted 后由 WS 创建并写入）；多任务产物
按任务序提交，支持用户按条裁决（批量裁决交互形态联动 DE 批量债务与 DE-04，本规则锁按条语义为
基线）。部分接受时仅 accepted 任务产物落盘，其余不影响。

## §4 构建控制态持久化（解除 CBT-04 原则）

<!-- RULE
rule_id: BR-CBT-PERSIST-001
主链路: CBT-BUILD
域: PERSIST
-->
构建控制态（任务计划、各任务执行/裁决进度、中断点）属 workspace 控制态：本地持久化
（workspace.db 扩展表，遵守 BR-SYS-PERSIST-001 版本化迁移），应用重启后构建回合可恢复至中断点
或由用户明确关闭，不得出现"既未恢复又未关闭"的悬挂构建态；控制态不上行云端。扩展表列定义进入
实现前与 WS/SYS 联合敲定并回写 A-SYS-I-P-01（X-SHARED-001，与 AG 回合历史表同批）。

## §5 Chat↔Build 接管协议（解除 CBT-03）

<!-- RULE
rule_id: BR-CBT-STATE-002
主链路: CBT-BUILD
域: STATE
-->
Chat↔Build 接管必须满足：chat 回合（SM-AG-01）识别多文件构建意图后，**经用户显式确认**方可
接管进入构建回合（SM-CBT-01），AG 侧回合以接管移交终结、上下文随移交传递；构建完成/中断后可
回落 chat 续接；接管与回落均为事件+回执双向确认（takeover/takeoverAck、fallback/fallbackAck），
两侧状态一致（X-INTER-001），不得出现双活（chat 与构建同时持有回合）或双死（移交中丢失回合）。

## §6 跨模块约束（复用，不新增）

X-FORBID-006（唯一落盘路径）、X-FORBID-004（文件限界）、X-SHARED-001（共享结构经 SYS）、
X-SEQ-001（门禁，经 AG 承接）、X-SEQ-003/X-INTER-001（建机与转移登记）。本文不新增 CONSTRAINT。

## §7 需求到规则映射（REQ_MAP）

<!-- REQ_MAP
req_id: REQ-CBT-001
target_type: CHAIN
target_id: CBT-BUILD
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-CBT-001
target_type: RULE
target_id: BR-CBT-STATE-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-CBT-002
target_type: RULE
target_id: BR-CBT-DATA-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-CBT-003
target_type: RULE
target_id: BR-CBT-PERSIST-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-CBT-003
target_type: RULE
target_id: BR-CBT-STATE-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-CBT-004
target_type: RULE
target_id: BR-CBT-STATE-002
status: mapped
-->

## §8 测试与验收

- **生命周期**（BR-CBT-STATE-001 / SM-CBT-01）：跨 ≥2 文件构建可完成；任务计划可见可调整；
  中断无半成品落盘；任务失败暂停不跳步；每次模型调用过门禁（静态+联调核验）。
- **diff 化**（BR-CBT-DATA-001）：修改/新建产物均经 DE 裁决；新建文档空基准表达 accepted 后
  正确创建；部分接受仅落已接受任务；无旁路写盘。
- **控制态恢复**（BR-CBT-PERSIST-001）：构建中重启→可恢复至中断点或明确关闭，无悬挂态。
- **接管协议**（BR-CBT-STATE-002）：接管经用户确认；事件+回执双向确认；无双活/双死
  （与 AG G5 联调）。
- 量化 NFR 延 G5（CBT-05）。

## §9 技术债务与盲区（诚实声明）

- 批量裁决交互形态联动 DE-04（待人类）与 DE 批量编排债务；本文锁按条裁决为基线语义。
- 任务分解提示工程/模型策略、并行任务执行（v1 按序执行为基线）延实现期。
- 控制态扩展表列定义与 AG 回合历史表同批联合 WS/SYS（实现前回写 A-SYS-I-P-01）。
- TPL 工作流委托 CBT 的消费契约随 TPL G2 对接（本文 §2 任务计划结构对其开放）。

## §10 后续动作

1. 注册（generate）：CBT-BUILD 链路、BR-CBT-STATE-001/002·DATA-001·PERSIST-001、
   TERM-CBT-001/002、REQ_MAP。
2. 续 A-CBT-M-S-01（SM-CBT-01），回填总图 CBT 行 + 闭合 Chat↔Build 转移点。
3. 待人类宣布 CBT G2 闭合。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-11 | v1.0 | 初版（CBT G2 首件）：构建回合与任务分解模型（计划可见可调整）、产物 diff 化（新建文档空基准表达、按条裁决基线，解除 CBT-02 契约部分）、控制态持久化（无悬挂态，解除 CBT-04 原则）、Chat↔Build 接管协议（显式确认 + 双向回执 + 无双活双死，解除 CBT-03）；注册链路 CBT-BUILD（P1）、规则 4 条、术语 BuildRun/BuildTask；约束全复用；REQ_MAP（REQ-CBT-001~004） |
