---
文档编号：CBT-M-T-01
文档状态：A
负责模块：CBT
文档职责：CBT 构建模块级 L2 技术设计（规则 SSOT）——构建回合与任务分解模型、产物 diff 化（含新建文档表达）、构建中断与控制态持久化、构建移交/回落协议；承接 REQ-CBT-001~004
上游约束：A-CBT-M-D-01_构建功能需求描述.md（REQ-CBT-001~004、CBT-01~05）、A-CORE-C-P-01_APC.md（§4.1/§6）、A-AG-M-T-01_代理技术设计.md（BR-AG-VERIFY-001 门禁、SM-AG-01/02 基建、X-FORBID-006）、A-DE-M-T-01_DiffEdit技术设计.md（ChangeProposal、BR-DE-DATA-002 载荷、批量裁决债务）、A-WS-M-T-01_工作台技术设计.md（X-FORBID-004、控制态持久化）、A-SYS-I-P-01_数据结构定义.md（X-SHARED-001、BR-SYS-PERSIST-001）、A-SYS-M-S-01_状态机总图.md（CBT 构建核心链、构建移交 转移点）、ADU.md（§3）
直接承接：本文 RULE/CHAIN/TERM 为 CBT 规则 SSOT，代码经 @GOV 映射；SM-CBT-01 在 A-CBT-M-S-01 定义并登记总图；接管协议供 AG 对接（闭合总图 构建移交 转移点）
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
一次多文件文档项目构建的完整生命周期（2026-07-04 补前段定义）：承接对话阶段确认的项目蓝图
（TERM-CBT-003）→任务计划分解→计划确认→逐任务产出（含调研/知识库检索等只读工具与产出后
轻量自查）→产物裁决；执行期间不可发起对话（仅中断）；可中断、可与代理回合双向移交/回落
（REQ-CBT-001/003/004）。对话阶段（多轮需求搜集→蓝图生成）属构建模式下的普通代理回合
（SM-AG-01），不入本回合生命周期。

<!-- TERM
term_id: TERM-CBT-002
chains: CBT-BUILD
zh: 构建任务
en: BuildTask
forbidden: 构建步骤, 子任务, build单元, 任务项
-->
构建回合内的最小执行单元：一个目标文件维度的产出动作（新建/修改），逐任务经 AG 回合基建执行，
产物以修改建议（ChangeProposal）形式提交 DE。

<!-- TERM
term_id: TERM-CBT-003
chains: CBT-BUILD
zh: 项目蓝图
en: BuildBlueprint
forbidden: 项目大纲, 构建大纲, 需求蓝图, blueprint
-->
构建模式对话阶段的产物（2026-07-04 人类裁决注册）：多轮对话搜集的项目描述收敛成的项目级
蓝图/大纲（目标、结构、文件构成意向）。用户确认蓝图并显式触发构建后，作为 startBuild 输入
随移交传递，经 Planning 分解为 BuildTask 计划（蓝图=需求侧工件，任务计划=执行侧工件，
各有一道人类确认）。

## §1 文档定位

```
REQ-CORE-003 → REQ-CBT-001~004（CBT L1）→ 本文（SSOT）→ 代码(@GOV)
本文 ←基建← A-AG-M-T-01（门禁/回合/工具）；产物 →交→ DE（ChangeProposal）→ WS（落盘）
本文 →驱动→ A-CBT-M-S-01（SM-CBT-01）→登记→ 总图（CBT 行 + 构建移交 转移点闭合）
```

**前提重申**：构建的一切模型调用经 AG 基建过门禁（BR-AG-VERIFY-001）；一切产物落盘经
DE 裁决→WS（X-FORBID-006）；CBT 自身无模型通道、无写盘通道。

## §2 构建回合与任务分解（CBT-BUILD）

**模型（2026-07-04 补前段）**：对话阶段（构建模式下多轮代理回合，需求搜集）→ 项目蓝图
（BuildBlueprint，用户确认 + 显式触发构建）→ 任务计划（BuildTask 有序集，每任务=一个目标文件
的产出动作）→ 计划确认 → 逐任务执行（经 SM-AG-01/02；任务内可用只读工具做调研/知识库检索；
产出后轻量自查一轮，深度自审延后）→ 产物逐任务/批量提交裁决。任务计划对用户可见、执行前可
调整；蓝图与计划本身不落盘文件（控制态，§4）。构建执行期间不可发起新对话回合（仅中断），
构建过程（规划/资料搜集/生成步骤/调整）在原对话窗口呈现。任务分解的提示工程与模型策略属
实现期，本文锁结构。

<!-- CHAIN
chain_id: CBT-BUILD
主责模块: CBT
status: active
risk_tier: P1
integration_test: deferred: phase-5
-->

<!-- RULE
rule_id: BR-CBT-STATE-001
主链路: CBT-BUILD
域: STATE
-->
构建回合必须经完整生命周期：承接项目蓝图（BuildBlueprint）分解为对用户可见的任务计划→计划经
用户确认→逐任务经 AG 回合基建执行（每次模型调用过门禁，BR-AG-VERIFY-001）→产物经 DE 裁决
落盘；**构建执行期间不得发起新对话回合**（仅中断可用，前端按 X-FORBID-007 投影门控，
2026-07-04 人类裁决）；回合可随时中断，中断为终态且无半成品落盘（未裁决产物按 pending diff
留存或随中断丢弃，由用户选择）；任务失败不静默跳过（暂停待处置）。状态转移归 SM-CBT-01
（A-CBT-M-S-01）。

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

## §5 构建移交/回落协议（解除 CBT-03）

<!-- RULE
rule_id: BR-CBT-STATE-002
主链路: CBT-BUILD
域: STATE
-->
构建移交/回落（旧称 Chat↔Build 接管；2026-07-04 判据换轴）必须满足：构建触发二选一——
**(a) 用户显式触发**（明确指令如「开始构建/开始生成」或点击构建按钮，触发即视为移交确认）；
**(b) 代理回合识别项目级整体构建意图后提议、经用户确认**。多文件修改意图**不再是**触发判据
（编辑模式本身可跨文件，BR-AG-DATA-003）。触发后 AG 侧回合以移交终结、上下文（含项目蓝图
BuildBlueprint）随移交传递，**构建在原对话窗口内呈现、不强制切换模式**；不再设独立的
「是否升级构建」二次弹窗（计划确认 PlanReview 为第二道人类闸门）。构建完成/中断后回落对话
续接；移交与回落均为事件+回执双向确认（takeover/takeoverAck、fallback/fallbackAck），两侧状态
一致（X-INTER-001），不得出现双活（对话与构建同时持有回合）或双死（移交中丢失回合）。

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
2. 续 A-CBT-M-S-01（SM-CBT-01），回填总图 CBT 行 + 闭合 构建移交 转移点。
3. 待人类宣布 CBT G2 闭合。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-11 | v1.0 | 初版（CBT G2 首件）：构建回合与任务分解模型（计划可见可调整）、产物 diff 化（新建文档空基准表达、按条裁决基线，解除 CBT-02 契约部分）、控制态持久化（无悬挂态，解除 CBT-04 原则）、Chat↔Build 接管协议（显式确认 + 双向回执 + 无双活双死，解除 CBT-03）；注册链路 CBT-BUILD（P1）、规则 4 条、术语 BuildRun/BuildTask；约束全复用；REQ_MAP（REQ-CBT-001~004） |
| 2026-07-04 | v2.0 | 构建模式定义回写（人类三轮陈述+裁决）：①TERM-CBT-001 补前段（对话阶段→蓝图→触发→计划→执行→裁决；执行期禁对话）；②新增 TERM-CBT-003 项目蓝图 BuildBlueprint（需求侧工件，与任务计划两道确认）；③BR-CBT-STATE-001 增蓝图承接与「执行期不得发起新对话回合」；④BR-CBT-STATE-002 重写：更名「构建移交/回落」，触发判据换轴（显式触发即确认 / 项目级构建意图提议经确认；多文件意图废止），原对话窗口呈现、无独立升级弹窗；⑤§2 模型补对话阶段/调研/轻量自查；模块正式名「构建（Build）」，弃用「Chat Build」别名 |
