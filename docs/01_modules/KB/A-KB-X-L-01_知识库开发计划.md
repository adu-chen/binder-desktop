---
文档编号：KB-X-L-01
文档状态：A
负责模块：KB
文档职责：KB 知识库 G3 开发计划——承接型 Plan Item，承接 KB-RETRIEVE 链路与 BR-KB-* 规则，明确文件范围/@GOV/A 类债务（检索选型）/验收（KB 无状态机，豁免成立）
上游约束：A-CORE-X-L-02（P4）、APC §5/§6/§12、A-KB-M-T-01、A-KB-I-P-01、A-WS-X-L-01（read 入库源）、A-AG-X-L-01（召回消费）、A-CORE-X-R-01
直接承接：本计划驱动 KB 实现；@GOV 追溯 BR-KB-*
使用边界：L3 承接型 Plan Item；组装裁剪归 AG、持久化经 workspace.db（不重复）；@GOV 在实现代码
变更要求：检索选型不得违反本地化硬边界（违反即产品边界变更须授权）；PI 增删保持承接可追溯
治理成熟度：draft
---

# Binder 知识库开发计划（KB · L3）

## §1 文档定位

KB 本地知识库，workspace 级隔离、**本地检索硬边界（禁云端 embedding）**。无状态机（豁免成立）。

## §2 Plan Item 清单

### PI-KB-01　条目存储、组织与主动入库　✅ G4 已实现（2026-06-22，store.rs 主动入库可溯源/隔离/删除即生效 + 3 covers + audit 绿）
- **承接**：REQ-KB-001/002 / BR-KB-DATA-001（隔离 + 主动入库可溯源）。
- **文件范围**：`src-tauri/src/kb/store.rs`（knowledge_entries 表 CRUD，基于 PI-SYS-03；入库源经
  `ws.read_doc`/外部文本）；`kb.ingest/list_entries/update_entry/delete_entry`。
- **@GOV**：workspace 隔离 + 主动入库（AI 不静默建库）+ 可溯源 `BR-KB-DATA-001`。
- **A 类债务**：入库管线（分段/清洗，实现期；引入异步流水线触发豁免复核）。
- **验收**：ingest 主动入库可溯源；无用户操作不建库；list/update/delete 限当前 workspace（隔离
  断言）；重启恢复。

### PI-KB-02　检索召回供给（本地化硬边界）　✅ G4 已实现（2026-06-22，retrieve.rs 本地检索无上行/存续召回/空集不注入 + 2 covers + audit 绿）
- **承接**：REQ-KB-003/004 / BR-KB-DATA-002（本地检索 + 无上行）。
- **文件范围**：`src-tauri/src/kb/retrieve.rs`（检索召回 KnowledgeInjection 供 AG）；`kb.retrieve`。
- **@GOV**：本地执行无内容上行 + 删除即不召回 + 空集不注入 `BR-KB-DATA-002`。
- **A 类债务**：**检索选型（关键词/向量/混合，KB-02，本地化硬边界已锁，选型记录落地回填 L2）**；
  KB 纳入 @mention（KB-03/REF-01，待 REF 联合）。
- **验收（强制）**：**retrieve 本地执行、无网络上行（网络核验，G4/G5 强制）**；已删条目不召回无
  悬空；空集不注入噪声；召回含 source_ref 可溯源（与 AG G5 联调）。

## §3 豁免复核（承 A-KB-M-T-01 §5）

KB 无状态机（请求-响应式）。**G3/G4 复核**：若引入异步索引构建/重建编排（如后台向量化批处理）
或入库管线多步异步，则触发建机（X-SEQ-003）。v1 不引入则豁免维持。

## §4 横切验收

audit 绿；@GOV 追溯 BR-KB-*；`// covers: BR-KB-*`（2 条）有断言；本地化硬边界网络核验；豁免
复核结论记录。

## §5 承接的 A 类债务（A-CORE-X-R-01 §6）

| 债务 | PI | 处置 |
|---|---|---|
| 检索选型（KB-02，本地化） | PI-KB-02 | 实现期选型回填 L2（硬边界已锁） |
| 入库管线（分段/清洗） | PI-KB-01 | 实现期（异步则建机复核） |
| KB 纳入 @mention（KB-03） | PI-KB-02 | 待 REF 联合 |
| 召回延迟/准确 NFR | 全 PI | G5 标定 |

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（G3，P4）：KB 2 条 Plan Item（存储组织与主动入库 / 检索召回本地硬边界），承接 KB-RETRIEVE + 2 规则；无状态机（豁免维持，§3 复核）；本地检索无上行网络核验强制；文件范围/@GOV/验收齐 |
