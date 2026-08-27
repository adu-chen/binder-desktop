---
文档编号：AST-M-T-01
文档状态：A
负责模块：AST
文档职责：AST 记忆协同模块级 L2 技术设计（规则 SSOT）——记忆条目模型与 workspace 级隔离（2026-06-11 人类决策）、自动写入与类型分级（2026-06-11 人类决策）、可见可溯源可撤销语义、注入供给契约；承接 REQ-AST-001~004
上游约束：A-AST-M-D-01_记忆协同功能需求描述.md（REQ-AST-001~004、AST-01/02 已决策）、A-CORE-C-P-01_APC.md（§4.1/§6/§9）、A-AG-M-T-01_代理技术设计.md（BR-AG-DATA-001 消费、AgentTurn 溯源）、A-WS-M-T-01_工作台技术设计.md（控制态持久化、X-FORBID-004）、A-SYS-I-P-01_数据结构定义.md（BR-SYS-PERSIST-001、X-SHARED-001）、A-SYS-M-S-01_状态机总图.md（记忆写入核心链）、ADU.md（§3）
直接承接：本文 RULE/CHAIN/TERM 为 AST 规则 SSOT；SM-AST-01 在 A-AST-M-S-01 定义并登记总图；供给契约对接 AG（与 REF/KB 同为上下文来源）
使用边界：本文是 AST 规则 SSOT；不组装上下文（AG）、不持久化基建（经 WS workspace.db 扩展）、不做知识库（KB）；类型分级的具体参数（注入优先级数值/保留期时长）延实现期；状态机内部归 A-AST-M-S-01
变更要求：写入策略语义变更须先经 APC/人类授权（产品关键决策）；记忆类型清单变更回写本文；供给结构进入实现前登记 A-SYS-I-P-01（X-SHARED-001）；TERM 一经 term_ref 引用其 en 不可更改
治理成熟度：draft
---

# Binder 记忆协同技术设计（AST · L2 技术设计）

## 0. Terminology Registry

<!-- TERM
term_id: TERM-AST-001
chains: AST-MEMORY
zh: 记忆条目
en: MemoryEntry
forbidden: 记忆点, 记忆项, 记忆片段, memory条目
-->
AST 沉淀的跨回合事实单元：含类型（分级依据）、内容、来源回合（AgentTurn 溯源）、状态
（active/revoked）；workspace 级隔离存储，自动写入、可见、可撤销（REQ-AST-001/002）。

<!-- TERM
term_id: TERM-AST-002
chains: REF-INJECT, AST-MEMORY
zh: 显式项目记忆引用
en: MemorySelectionRef
forbidden: 记忆引用, 记忆选择, 记忆注入引用, 记忆条目引用, 记忆选中项
-->
用户管理的 workspace 本地项目记忆条目，经用户明确选择以 data-only 引用进入当前回合；Agent 1.0
不自动写入、不提议写入、不自动检索或注入；普通发送、附件、会话/编辑历史不自动转存为记忆；跨会话
不串选择事实（A-SYS-I-P-01 §3.1，BR-SYS-DATA-007）。

## §1 文档定位

```
REQ-CORE-005 → REQ-AST-001~004（AST L1，AST-01/02 已人类决策）→ 本文（SSOT）→ 代码(@GOV)
本文 →驱动→ A-AST-M-S-01（SM-AST-01 记忆写入机）→登记→ 总图（记忆写入行）
本文供给 →交→ AG（BR-AG-DATA-001 组装）
```

两项人类决策（2026-06-11）在本文落为规则：**workspace 级隔离**（§2）、
**自动写入无需确认 + 按类型分级**（§3，保持可见/可溯源/可撤销）。

## §2 记忆条目模型与 workspace 级隔离（AST-MEMORY）

**MemoryEntry 字段语义**（模块内模型；跨模块供给结构见 §4）：
entry_id（不透明标识）、memory_kind（类型，§3 分级依据）、content（事实内容）、
origin_turn（来源回合标识，溯源）、status（active/revoked）、created_at / revoked_at。

<!-- CHAIN
chain_id: AST-MEMORY
主责模块: AST
status: active
risk_tier: P1
integration_test: deferred: phase-5
-->

<!-- RULE
rule_id: BR-AST-DATA-001
主链路: AST-MEMORY
域: DATA
-->
记忆条目按 **workspace 级隔离**存储（2026-06-11 人类决策）：持久化于该 workspace 的控制态层
（workspace.db 扩展表，遵守 BR-SYS-PERSIST-001 迁移），跨 workspace 不共享、不可见；切换
workspace 即切换记忆集；记忆内容不上行云端（REQ-AST-004）；条目对用户全量可见可管理
（浏览/编辑/删除），删除即生效且不再供给。扩展表列定义进入实现前与 WS/SYS 联合敲定并回写
A-SYS-I-P-01（与 AG 回合历史、CBT 控制态同批）。

## §3 自动写入与类型分级（AST-MEMORY）

**v1 记忆类型基线**（memory_kind，可扩展，扩展须回写本文）：

| 类型 | 语义 | 分级处理（机制，参数延实现期） |
|---|---|---|
| preference | 用户偏好（行文偏好、禁忌词等） | 注入优先级高、长期保留 |
| style | 写作风格特征 | 注入优先级高、长期保留 |
| project_fact | 项目/作品背景事实（设定、人物、进度） | 按相关性注入、随项目存续 |
| session_context | 短时协同上下文 | 注入优先级低、短期保留/自动失效 |

<!-- RULE
rule_id: BR-AST-STATE-001
主链路: AST-MEMORY
域: STATE
-->
记忆写入采用**自动写入、无需逐条确认**（2026-06-11 人类决策），并满足可控三要素：每条写入
**可见**（写入后用户可在记忆管理面感知与查看）、**可溯源**（origin_turn 关联来源回合）、
**可撤销**（revoke 即不再供给，撤销不可复活、重新沉淀为新条目）；写入按类型分级
（memory_kind 决定注入优先级与保留期策略，§3 基线表）；不得写入无类型、无来源的条目；
不存在用户不可见的记忆积累。状态转移归 SM-AST-01（A-AST-M-S-01）。

## §4 注入供给契约（AST-MEMORY）

<!-- RULE
rule_id: BR-AST-DATA-002
主链路: AST-MEMORY
域: DATA
-->
记忆供给必须满足：仅供给当前 workspace 的 active 条目（隔离 + revoked 不供给）；按相关性与
类型分级优先级筛选（preference/style 优先，session_context 最低）；供给载荷含 memory_kind 与
origin_turn（消费方可溯源）；无相关记忆供给空集，不注入噪声；供给的跨模块结构进入实现前登记
A-SYS-I-P-01（X-SHARED-001，参照 RefInjection 模式另立 MemoryInjection 基线，不复用 RefInjection
——其 target 为 DocRef 语义不符）。预算与裁剪归 AG（BR-AG-DATA-001）。

### 4.1 显式记忆选择契约与 1.0 不自动写入边界（2026-08-14 D3 登记）

承接 R5-C（A-CORE-X-D-02 §4.6）与 BR-SYS-DATA-007（D1）：项目长期记忆是用户管理的 workspace 本地
资产，Agent 1.0 **只允许用户显式选择的条目进入当前回合**（MemorySelectionRef）：

| 契约项 | 1.0 语义 | 承接 |
|---|---|---|
| 显式选择 | 仅用户明确选择的记忆条目经 REF 以 data-only 进入当前回合（MemorySelectionRef） | BR-SYS-DATA-007、TERM-AST-002 |
| 不自动写入/提议 | Agent 1.0 不自动写入记忆、不生成写入提议 | BR-SYS-DATA-007；BR-AST-STATE-001 自动写入在 1.0 关闭 |
| 不自动检索/注入 | 普通回合不按相关性自动检索或注入记忆 | BR-AST-DATA-002 自动供给在 1.0 关闭 |
| 不自动转存 | 外部内容、会话历史、编辑历史不得自动转存为项目记忆 | BR-SYS-DATA-007 |
| 隔离与失效 | workspace 隔离；条目删除/关闭后引用失效或明确标记；跨会话不串选择事实 | BR-AST-DATA-001、BR-SYS-DATA-007 |

<!-- REQ_MAP
req_id: REQ-AST-001
target_type: RULE
target_id: BR-SYS-DATA-007
status: mapped
-->

> **迁移债务**：现有 BR-AST-STATE-001（自动写入无需确认）与 BR-AST-DATA-002（相关性自动供给）为
> 2026-06-11 人类决策落库（P2 记忆能力）；R5-C 冻结基线要求 Agent 1.0 关闭自动写入/提议/检索/注入。
> D3 只登记边界：1.0 入口仅显式选择（MemorySelectionRef），自动能力保留为 P2 路线；具体关闭策略与
> feature gate 由实现 Scope 承接，不得以「自动写入已关闭」之外的 Prompt 文案掩盖。

## §5 跨模块约束（复用，不新增）

X-SHARED-001（供给结构经 SYS）、X-FORBID-004（workspace 边界）、X-SEQ-003/X-INTER-001
（建机登记）。不新增 CONSTRAINT。

## §6 需求到规则映射（REQ_MAP）

<!-- REQ_MAP
req_id: REQ-AST-001
target_type: CHAIN
target_id: AST-MEMORY
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AST-001
target_type: RULE
target_id: BR-AST-DATA-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AST-002
target_type: RULE
target_id: BR-AST-STATE-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AST-003
target_type: RULE
target_id: BR-AST-DATA-002
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-AST-004
target_type: RULE
target_id: BR-AST-DATA-001
status: mapped
-->

## §7 测试与验收

- **隔离**（BR-AST-DATA-001）：workspace A 的记忆在 B 不可见不供给；切换即切换记忆集；删除即
  生效；重启完整恢复；无内容上行（网络核验）。
- **写入可控三要素**（BR-AST-STATE-001 / SM-AST-01）：自动写入后条目可见、可溯源到回合、
  revoke 后不再供给且不可复活；无类型/无来源条目被拒。
- **供给**（BR-AST-DATA-002）：revoked 不供给；分级优先级生效；空集不注入噪声；载荷可溯源。
- 量化 NFR（筛选延迟）延 G5（AST-05）。

## §8 技术债务与盲区（诚实声明）

- 分级参数（保留期时长、注入优先级权重）与相关性筛选机制（AST-04）延实现期标定。
- session_context 自动失效的触发（时间/回合数）延实现期。
- MemoryInjection 供给结构与扩展表列定义进入实现前登记 SYS（与 AG/CBT 控制态同批）。
- 记忆纳入 @mention（AST-03/REF-01）待与 REF 联合，本文供给契约对其开放。

## §9 后续动作

1. 注册（generate）：AST-MEMORY 链路、BR-AST-DATA-001/002·STATE-001、TERM-AST-001、REQ_MAP。
2. 续 A-AST-M-S-01（SM-AST-01），回填总图记忆写入行。
3. 待人类宣布 AST G2 闭合。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-11 | v1.0 | 初版（AST G2）：两项人类决策落规则——workspace 级隔离（BR-AST-DATA-001）、自动写入+类型分级（BR-AST-STATE-001，保持可见/可溯源/可撤销三要素）；v1 记忆类型基线（preference/style/project_fact/session_context）；注入供给契约（BR-AST-DATA-002，MemoryInjection 结构延实现前登记 SYS）；注册链路 AST-MEMORY、术语 MemoryEntry；REQ_MAP（REQ-AST-001~004） |
| 2026-07-04 | v1.1 | 上游约束文件名指针更新（A-AG-*/A-DE-* 文件更名为 代理*/DiffEdit*，2026-07-04 人类确认；文档编号不变，无语义变更） |
| 2026-08-14 | v1.2 | AG-1.0-D3-AG-REF-AST-RUNTIME-CONTRACT-01：注册 TERM-AST-002 MemorySelectionRef；新增 §4.1 显式记忆选择契约与 1.0 不自动写入边界（R5-C / BR-SYS-DATA-007）——仅显式选择进入当前回合、不自动写入/提议/检索/注入、不自动转存；登记 BR-AST-STATE-001/BR-AST-DATA-002 自动能力在 1.0 关闭的漂移债务；补 REQ-AST-001 REQ_MAP。 |
| 2026-08-14 | v1.3 | AG-1.0-IMP-MEMORY-SELECTION-01：§4.1 供给端落地——AST 新增 list_manageable（workspace 作用域、active 条目含 content）经 ast.list_entries 路由暴露；前端记忆库面板（MemoryPanel，浏览/显式选择）构造 MemorySelectionRef 快照经引用注入总线进 Composer 输入区；data-only 不写回 AST、跨会话不串；自动写入/自动供给保持关闭（memory_link/supply 未接线）；BR-SYS-DATA-007 记忆半进入代码 @GOV 引用与测试 covers，audit 维持全绿。 |
