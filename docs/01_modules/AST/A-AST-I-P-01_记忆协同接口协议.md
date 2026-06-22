---
文档编号：AST-I-P-01
文档状态：A
负责模块：AST
文档职责：AST 记忆协同模块级接口协议——记忆管理 command（浏览/编辑/删除/撤销）、自动写入回注、记忆供给接口
上游约束：A-SYS-I-P-02_IPC协议.md（骨架）、A-AST-M-T-01_记忆协同技术设计.md（BR-AST-* 规则、AST-MEMORY、MemoryEntry）、A-AST-M-S-01（SM-AST-01 记忆写入机）、A-WS-I-P-01（memory_entries 控制态）、A-SYS-I-P-01（MemoryInjection）、A-AG-I-P-01（供给消费）、ADU.md
直接承接：实现期 AST command 注册；写入由 AG 回合触发（自动）；供给消费经 AG；持久化经 workspace.db memory_entries
使用边界：声明语义级签名；组装裁剪归 AG（不重复）；不新增注册项；workspace 级隔离（决策）、自动写入按类型分级（决策）
变更要求：写入策略接口变更涉产品决策须人类授权；供给结构回写 A-SYS-I-P-01（MemoryInjection）
治理成熟度：draft
---

# Binder 记忆协同接口协议（AST · L2 接口层）

## §1 文档定位

```
A-SYS-I-P-02（骨架）→ 本文（AST 管理/写入/供给接口）→ 实现 + @GOV(追溯 A-AST-M-T-01)
写入：AG 回合 → ast.capture（自动，SM-AST-01）；供给 → AG 消费；持久化 → memory_entries（workspace 级隔离）
```

## §2 接口清单（AST-MEMORY / SM-AST-01）

| command | 参数（语义） | 结果 | 特有错误/状态 | 归属 |
|---|---|---|---|---|
| `ast.capture` | workspace_id、候选事实、memory_kind、origin_turn | entry_id（自动写入，无需确认） | 无类型/无来源→拒绝（guard） | SM-AST-01 Writing；BR-AST-STATE-001（可见/可溯源/可撤销） |
| `ast.list_memories` | workspace_id、过滤（kind/status 可空） | MemoryEntry 列表（仅当前 workspace） | — | BR-AST-DATA-001（隔离，全量可见可管理） |
| `ast.edit_memory` | entry_id、修改内容 | 更新确认 | 不存在 | 用户可管理 |
| `ast.revoke_memory` | entry_id | Revoked（不可复活，即刻停止供给） | — | SM-AST-01 revoke |
| `ast.delete_memory` | entry_id | 删除确认（即不再供给） | — | BR-AST-DATA-001 |
| `ast.supply` | workspace_id、相关性上下文 | MemoryInjection 集（active + 分级优先级，含 origin_turn） | — | BR-AST-DATA-002（revoked 不供给，空集不注入噪声；裁剪归 AG） |

## §3 测试与验收

- capture 自动写入且可见可溯源、无类型/无来源拒绝、撤销不可复活；list/edit/delete 限当前 workspace
  （隔离断言，他 workspace 不可见）；supply 仅 active + 分级优先级 + 空集不注入；与 AG G5 联调。

## §4 债务

- 相关性筛选机制延实现期（AST-04）；session_context 自动失效触发延实现期；记忆纳入 @mention
  （AST-03/REF-01）待 REF 联合。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（实现前置件）：AST 接口协议——capture 自动写入（无类型无来源拒绝）/list/edit/revoke（不可复活）/delete/supply（active+分级，空集不注入）；workspace 级隔离断言；零新增注册项；债务（筛选机制/失效触发/@mention 纳入延后） |
