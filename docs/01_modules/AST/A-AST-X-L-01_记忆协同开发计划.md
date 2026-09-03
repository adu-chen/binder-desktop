---
文档编号：AST-X-L-01
文档状态：A
负责模块：AST
文档职责：AST 记忆协同 G3 开发计划——承接型 Plan Item，承接 AST-MEMORY 链路与 BR-AST-* 规则、SM-AST-01，明确文件范围/@GOV/A 类债务/验收
上游约束：A-CORE-X-L-02（P4）、A-CORE-X-D-02（自动记忆为 P2；Agent 1.0 仅显式选择的项目记忆）、APC §5/§6/§12、A-AST-M-T-01/A-AST-M-S-01、A-AST-I-P-01、A-AG-M-D-01（REQ-AG-017）、A-AG-M-T-01（BR-AG-STATE-004 关闭边界）、A-CORE-X-R-01
直接承接：本计划驱动 AST 实现；@GOV 追溯 BR-AST-*；SM-AST-01 转移矩阵
使用边界：L3 承接型 Plan Item；组装裁剪归 AG、持久化经 workspace.db（不重复）；@GOV 在实现代码
变更要求：写入触发与 AG PI-AG-05（B-2）双侧对齐；PI 增删保持承接可追溯
治理成熟度：draft
---

# Binder 记忆协同开发计划（AST · L3）

## §1 文档定位

AST 本地记忆，workspace 级隔离（人类决策）、自动写入按类型分级。**写入触发依赖 AG B-2 补齐**
（PI-AG-05）——AST 侧已就绪（capture 接口 + SM-AST-01），待 AG 侧声明回合何时调用对齐。

## §2 Plan Item 清单

### PI-AST-01　记忆条目存储与 workspace 级隔离　✅ G4 已实现（2026-06-22，store.rs 隔离查询 + 2 covers + audit 绿）
- **承接**：REQ-AST-001/004 / BR-AST-DATA-001（隔离 + 不上云）。
- **文件范围**：`src-tauri/src/ast/store.rs`（memory_entries 表 CRUD，基于 PI-SYS-03；workspace
  级隔离）；`ast.list_memories/edit_memory/delete_memory`。
- **@GOV**：workspace 隔离/不上云/可管理 `BR-AST-DATA-001`。
- **A 类债务**：无（结构已定）。
- **验收**：workspace A 记忆在 B 不可见不供给；切换即切换记忆集；删除即生效；重启恢复；
  **无内容网络上行（网络核验）**。

### PI-AST-02　自动写入机与类型分级　✅ G4 已实现（2026-06-22，capture.rs SM-AST-01 + 撤销不可复活 + 3 covers + audit 绿）
- **承接**：REQ-AST-002 / SM-AST-01 / BR-AST-STATE-001（自动写入 + 可见可溯源可撤销）。
- **文件范围**：`src-tauri/src/ast/capture.rs`（自动写入、类型分级、撤销）；`ast.capture/
  revoke_memory`。**写入触发对接 AG PI-AG-05（B-2）**。
- **@GOV**：自动写入三要素（可见/可溯源/可撤销）+ 无类型无来源拒绝 `BR-AST-STATE-001`。
- **A 类债务**：capture 判定机制（什么算可沉淀事实，模型/规则，实现期）；分级参数（保留期/优先级
  权重，实现期/G5）；session_context 自动失效触发（实现期）。
- **验收（强制，APC §6）**：SM-AST-01 矩阵——自动写入条目可见、origin_turn 可溯源、revoke 不可
  复活；无类型/无来源 guard 拒绝；不存在用户不可见的 Written 条目。**写入触发待 AG B-2 补齐**。

### PI-AST-03　记忆注入供给　✅ G4（supply.rs 仅 active+workspace 隔离/类型分级优先 preference>session_context/origin_turn 可溯源/空集不注入 BR-AST-DATA-002，+2 covers，audit 绿）
- **承接**：REQ-AST-003 / BR-AST-DATA-002（供给）。
- **文件范围**：`src-tauri/src/ast/supply.rs`（按相关性 + 分级优先级供给 MemoryInjection 供 AG）；
  `ast.supply`。
- **@GOV**：供给（active+分级/空集不注入/可溯源）`BR-AST-DATA-002`。
- **A 类债务**：相关性筛选机制（AST-04，实现期）；记忆纳入 @mention（AST-03/REF-01，待 REF 联合）。
- **验收**：仅供给当前 workspace active 条目；revoked 不供给；分级优先级生效；空集不注入噪声；
  载荷含 memory_kind/origin_turn 可溯源（与 AG G5 联调）。

## §3 横切验收

audit 绿；@GOV 追溯 BR-AST-*；SM-AST-01 矩阵全覆盖；`// covers: BR-AST-*`（3 条）有断言。

## §4 承接的债务（A-CORE-X-R-01）

| 债务 | 类 | PI | 处置 |
|---|---|---|---|
| capture 判定机制 | A | PI-AST-02 | 实现期 |
| 分级参数/session 失效触发 | A | PI-AST-02 | 实现期/G5 |
| 相关性筛选（AST-04） | A | PI-AST-03 | 实现期 |
| 记忆纳入 @mention（AST-03） | A | PI-AST-03 | 待 REF 联合 |
| 写入触发对接（B-2） | B（AG 侧） | PI-AST-02 | 待 AG B-2 补齐 |

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（G3，P4）：AST 3 条 Plan Item（存储与 workspace 隔离 / 自动写入机与分级 / 注入供给），承接 AST-MEMORY + 3 规则 + SM-AST-01；写入触发依赖 AG B-2 补齐对齐；隔离+不上云网络核验；文件范围/@GOV/验收齐 |
