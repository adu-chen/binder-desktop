---
文档编号：KB-I-P-01
文档状态：A
负责模块：KB
文档职责：KB 知识库模块级接口协议——条目管理 command（增删改/组织）、入库、检索召回供给接口
上游约束：A-SYS-I-P-02_IPC协议.md（骨架）、A-KB-M-T-01_知识库技术设计.md（BR-KB-* 规则、KB-RETRIEVE、KnowledgeEntry、本地检索硬边界）、A-WS-I-P-01（knowledge_entries 控制态、read 入库源）、A-SYS-I-P-01（KnowledgeInjection）、A-AG-I-P-01（召回消费）、ADU.md
直接承接：实现期 KB command 注册；入库源经 WS 读取；召回供给经 AG；持久化经 workspace.db knowledge_entries
使用边界：声明语义级签名；组装裁剪归 AG（不重复）；不新增注册项；workspace 级隔离 + 本地检索硬边界（禁云端 embedding）；无状态机（豁免成立）
变更要求：检索接口实现不得违反本地化硬边界（违反即产品边界变更须授权）；供给结构回写 A-SYS-I-P-01
治理成熟度：draft
---

# Binder 知识库接口协议（KB · L2 接口层）

## §1 文档定位

```
A-SYS-I-P-02（骨架）→ 本文（KB 管理/入库/检索接口）→ 实现 + @GOV(追溯 A-KB-M-T-01)
入库源 → ws.read_doc；召回供给 → AG 消费；持久化 → knowledge_entries（workspace 级隔离，本地检索）
```

## §2 接口清单（KB-RETRIEVE）

| command | 参数（语义） | 结果 | 特有错误 | 归属 |
|---|---|---|---|---|
| `kb.ingest` | workspace_id、来源（DocRef 或外部文本）、分类/标签 | entry_id（含 source_ref 可溯源） | 入库失败（库保持一致） | BR-KB-DATA-001（用户主动，AI 不静默建库） |
| `kb.list_entries` | workspace_id、过滤/标签（可空） | KnowledgeEntry 列表（仅当前 workspace） | — | BR-KB-DATA-001（隔离，可见可管理） |
| `kb.update_entry` | entry_id、内容/组织 | 更新确认 | 不存在 | — |
| `kb.delete_entry` | entry_id | 删除确认（即不再召回） | — | BR-KB-DATA-001 |
| `kb.retrieve` | workspace_id、查询 | KnowledgeInjection 集（含 source_ref；已删不召回，无悬空） | — | BR-KB-DATA-002（**本地执行、无内容上行**；空集不注入噪声；裁剪归 AG） |

## §3 测试与验收

- ingest 主动入库可溯源、无用户操作不建库；list/update/delete 限当前 workspace（隔离断言）；
  **retrieve 本地执行、无网络上行（网络核验，G4/G5 强制）**、删除即不召回、空集不注入；与 AG G5 联调。

## §4 债务

- 检索选型（关键词/向量/混合，本地化硬边界）延实现期（KB-02）；若引入异步索引构建触发建机复核
  （KB-04）；入库管线（分段/清洗）延实现期；KB 纳入 @mention（KB-03/REF-01）待 REF 联合。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（实现前置件）：KB 接口协议——ingest 主动入库/list/update/delete/retrieve（本地执行无上行、删除即不召回、空集不注入）；workspace 级隔离 + 本地检索硬边界断言；零新增注册项；债务（检索选型/异步索引复核/入库管线/@mention 纳入延后） |
