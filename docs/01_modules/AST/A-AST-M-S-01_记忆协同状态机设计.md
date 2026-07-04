---
文档编号：AST-M-S-01
文档状态：A
负责模块：AST
文档职责：AST 记忆协同模块级状态机设计——记忆写入机（SM-AST-01），含 APC §6 必备字段；回填 A-SYS-M-S-01 总图记忆写入行（planned→active）
上游约束：A-AST-M-T-01_记忆协同技术设计.md（AST-MEMORY、BR-AST-* 规则）、A-CORE-C-P-01_APC.md（§6 记忆写入核心链）、A-SYS-M-S-01_状态机总图.md（X-SEQ-003）、A-AG-M-S-01_代理状态机设计.md（SM-AG-01 回合，来源溯源）、ADU.md（§3.1）
直接承接：SM-AST-01 经 rules 字段承载 A-AST-M-T-01 规则；machine_id 登记总图
使用边界：本文定义 AST 状态机内部；记忆注入供给为请求-响应式不建机（§4 豁免说明）；不重定义 SM-AG-01 内部
变更要求：transitions 变更须同步 A-AST-M-T-01 并重跑测试矩阵；machine_id 一经登记不可更改
治理成熟度：draft
---

# Binder 记忆协同状态机设计（AST · L2 技术设计）

## §1 文档定位

```
A-AST-M-T-01（规则 SSOT：AST-MEMORY）→ 本文 SM-AST-01 →登记→ 总图（记忆写入行 planned→active）
```

满足 X-SEQ-003（记忆写入为 APC §6 列举核心链）。写入策略=自动写入+类型分级（2026-06-11
人类决策），本机据此无"等待用户确认"态——可控性由可见/可溯源/可撤销路径承载。

## §2 SM-AST-01 · 记忆写入机

<!-- STATE_MACHINE
machine_id: SM-AST-01
owner_module: AST
status: active
summary: 单条记忆自捕获判定到落库/撤销的生命周期（自动写入、类型分级、可见可溯源可撤销）
source_type: design
chain: AST-MEMORY
rules: BR-AST-STATE-001, BR-AST-DATA-001
initial_state: Capturing
terminal_states: Written, Revoked, Discarded
-->

- **machine_id**：SM-AST-01　**owner_module**：AST
- **scope**：单条候选记忆自回合中捕获判定到落库（及后续撤销）的生命周期
- **context**：候选事实、memory_kind 判定结果、来源回合（origin_turn）、目标 workspace、落库结果
- **states**：Capturing、Writing、Written、Revoked、Discarded、WriteError
- **events**：captureHit、captureMiss、writeOk、writeFail、retry、revoke
- **transitions（主表）**：

| from | event | guard | to | action |
|---|---|---|---|---|
| Capturing | captureHit | 类型可判定（memory_kind 明确）且来源回合可溯 | Writing | 按类型分级自动写入（无需确认，人类决策） |
| Capturing | captureMiss | 非可沉淀事实/类型不可判定 | Discarded | 不写入（终态），无残留 |
| Writing | writeOk | 落库于当前 workspace 控制态 | Written | 条目可见、可溯源（管理面可感知） |
| Writing | writeFail | — | WriteError | 统一错误形状 |
| WriteError | retry | — | Writing | 重试落库 |
| Written | revoke | 用户撤销 | Revoked | 即刻停止供给；条目标记 revoked（终态） |

- **error_states**：WriteError（recovery：retry→Writing；持续失败丢弃并回报，不静默吞）
- **recovery_paths**：WriteError→retry；Revoked 不可复活（同一事实重新沉淀=新条目新实例）
- **forbidden_transitions**：
  - Capturing → Written（未经落库不得标记已写入）
  - Capturing 无类型/无来源 → Writing（guard 拒绝：不得写入无类型、无来源条目）
  - Revoked → Written / Writing（撤销不可复活）
  - Discarded → Writing（判定不写后不得静默补写）
- **acceptance_criteria**：每条 Written 条目可见、可溯源（origin_turn 非空）、可撤销；revoke 后
  立即不再供给；无类型/无来源条目不可达 Writing；不存在用户不可见的 Written 条目
- **test_matrix**：见 §3

## §3 测试与验收（强制转移测试矩阵，APC §6/§10）

- **正路**：captureHit→Writing→writeOk→Written（断言可见+origin_turn 可溯+按类型分级属性正确）。
- **撤销路**：Written→revoke→Revoked，断言立即停止供给、不可复活、重新沉淀为新条目。
- **丢弃路**：captureMiss→Discarded 无残留；无类型/无来源 guard 拒绝。
- **失败路**：writeFail→WriteError→retry；持续失败有明确回报。
- **隔离断言**（联 BR-AST-DATA-001）：写入落当前 workspace；他 workspace 不可见。
- 跨模块（G5）：与 SM-AG-01 的来源回合溯源链。

## §4 不建机的链（豁免说明，APC §6）

**记忆注入供给**：请求-响应式（AG 发起→筛选→供给），无多步状态化生命周期，按 APC §6 豁免；
若实现期引入异步索引/批量重算编排则复核建机（同 REF/KB 豁免模式）。

## §5 总图登记

| 核心链 | machine_id | 总图回填动作 |
|---|---|---|
| 记忆写入 | SM-AST-01 | §3 行 planned→active（A-AST-M-S-01 §2） |

## §6 术语归属与债务

MemoryEntry 已在 A-AST-M-T-01 §0 注册。债务：captureHit 的判定机制（模型辅助判定 vs 规则）与
session_context 自动失效触发延实现期；批量撤销/清空的管理操作属管理面实现，不改本机转移。

## §7 后续动作

1. 同步回填 A-SYS-M-S-01 §3 记忆写入行；generate 复跑核对。
2. 待人类宣布 AST G2 闭合。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-11 | v1.0 | 初版（AST G2 状态机件）：定义 SM-AST-01 记忆写入机（Capturing/Writing/Written/Revoked/Discarded/WriteError，自动写入无确认态——可控性由可见/可溯源/可撤销承载；无类型无来源 guard 拒绝；撤销不可复活），含 APC §6 全部必备字段；注入供给豁免说明；满足 X-SEQ-003 |
| 2026-07-04 | v1.1 | 上游约束文件名指针更新（A-AG-*/A-DE-* 文件更名为 代理*/DiffEdit*，2026-07-04 人类确认；文档编号不变，无语义变更） |
