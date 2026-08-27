---
文档编号：KB-M-T-01
文档状态：A
负责模块：KB
文档职责：KB 知识库模块级 L2 技术设计（规则 SSOT）——知识条目模型与 workspace 级隔离（2026-06-11 人类决策）、主动入库与溯源、本地检索召回硬边界、状态机豁免复核；承接 REQ-KB-001~004
上游约束：A-KB-M-D-01_知识库功能需求描述.md（REQ-KB-001~004、KB-01 已决策）、A-CORE-C-P-01_APC.md（§4.1/§6/§9）、A-AG-M-T-01_代理技术设计.md（BR-AG-DATA-001 消费）、A-WS-M-T-01_工作台技术设计.md（X-FORBID-004、控制态持久化）、A-SYS-I-P-01_数据结构定义.md（BR-SYS-PERSIST-001、X-SHARED-001）、A-REF-M-T-01_引用技术设计.md（REF-01 联动）、ADU.md（§3）
直接承接：本文 RULE/CHAIN/TERM 为 KB 规则 SSOT；供给契约对接 AG；KB 条目纳入 @mention 的解析协议随 REF 联动（KB-03/REF-01）
使用边界：本文是 KB 规则 SSOT；不组装上下文（AG）、不做记忆（AST）；检索技术选型（关键词/向量/混合）延实现期但受本地化硬边界约束；无状态机（豁免复核 §5）
变更要求：检索选型不得违反本地化硬边界（违反须先经 APC 授权——实为产品边界变更）；供给结构进入实现前登记 A-SYS-I-P-01；TERM 一经 term_ref 引用其 en 不可更改
治理成熟度：draft
---

# Binder 知识库技术设计（KB · L2 技术设计）

## 0. Terminology Registry

<!-- TERM
term_id: TERM-KB-001
chains: KB-RETRIEVE
zh: 知识条目
en: KnowledgeEntry
forbidden: 知识点, 资料条目, 知识片段, kb条目
-->
用户主动入库的参考资料单元：含内容、分类/标签、来源（可溯源到入库源文档/文本）；workspace 级
隔离存储，可检索召回供给 AG（REQ-KB-001~003）。

## §1 文档定位

```
REQ-CORE-005 → REQ-KB-001~004（KB L1，KB-01 已人类决策）→ 本文（SSOT）→ 代码(@GOV)
本文供给 →交→ AG（BR-AG-DATA-001 组装）；@mention 纳入随 REF 联动（KB-03/REF-01）
```

无状态机（豁免复核 §5）。人类决策（2026-06-11）：**workspace 级隔离**落 §2 规则。

## §2 条目模型、隔离与主动入库（KB-RETRIEVE）

**KnowledgeEntry 字段语义**：entry_id、content、organization（分类/标签）、source_ref
（入库来源：workspace 文档 DocRef 或外部文本标注）、created_at / updated_at。

<!-- CHAIN
chain_id: KB-RETRIEVE
主责模块: KB
status: active
risk_tier: P1
integration_test: deferred: phase-5
-->

<!-- RULE
rule_id: BR-KB-DATA-001
主链路: KB-RETRIEVE
域: DATA
-->
知识条目按 **workspace 级隔离**存储（2026-06-11 人类决策）：持久化于该 workspace 控制态层
（遵守 BR-SYS-PERSIST-001 迁移），跨 workspace 不共享；入库为**用户主动操作**（AI 不静默建库），
每条可溯源到入库来源（source_ref）；条目与组织结构对用户可见可管理，删除即生效且不再召回；
库内容不上行云端（REQ-KB-004）。扩展表列定义进入实现前与 WS/SYS 联合（与 AST/AG/CBT 控制态同批）。

## §3 检索召回与本地化硬边界（KB-RETRIEVE）

<!-- RULE
rule_id: BR-KB-DATA-002
主链路: KB-RETRIEVE
域: DATA
-->
检索召回必须满足：检索全程本地执行——**选型（关键词/向量/混合）不论何种，索引构建与查询不得
将库内容经网络上行**（含禁用云端 embedding 服务；本地化是硬边界，违反即产品边界变更须 APC
授权）；仅召回当前 workspace 的存续条目（已删除不召回，无悬空）；召回供给含 source_ref 来源
标识（消费方可溯源）；无相关召回供给空集不注入噪声；供给跨模块结构进入实现前登记 A-SYS-I-P-01
（X-SHARED-001，与 MemoryInjection 同批）。预算裁剪归 AG。

## §4 跨模块约束（复用，不新增）

X-FORBID-004、X-SHARED-001、BR-SYS-PERSIST-001（经 §2 承接）。不新增 CONSTRAINT。

## §5 状态机豁免复核（解除 KB-04）

**结论：豁免成立，KB v1 不建状态机。** 入库（用户操作→写库→完成/失败）与检索（查询→召回）均为
请求-响应式同步操作，无多步异步生命周期、无跨模块状态耦合（APC §6 口径 b）。
**豁免失效触发条件**：引入异步索引构建/重建编排（如后台向量化批处理）、引入入库管线多步异步
处理（分段/清洗流水线），任一出现按 X-SEQ-003 建机并登记总图。

## §6 需求到规则映射（REQ_MAP）

<!-- REQ_MAP
req_id: REQ-KB-001
target_type: CHAIN
target_id: KB-RETRIEVE
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-KB-001
target_type: RULE
target_id: BR-KB-DATA-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-KB-002
target_type: RULE
target_id: BR-KB-DATA-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-KB-003
target_type: RULE
target_id: BR-KB-DATA-002
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-KB-004
target_type: RULE
target_id: BR-KB-DATA-002
status: mapped
-->

## §7 测试与验收

- **隔离与主动入库**（BR-KB-DATA-001）：跨 workspace 不可见；无用户操作不产生条目（静态+行为
  核验）；条目可溯源；删除即不召回；重启完整恢复。
- **本地化硬边界**（BR-KB-DATA-002）：断网时入库/组织/检索全可用；索引与查询无网络上行
  （网络核验，G4/G5 强制项）；空集不注入噪声。
- 量化 NFR（召回延迟/准确）延 G5（KB-05）。

## §8 技术债务与盲区（诚实声明）

- 检索选型（关键词/向量/混合）延实现期——本地化硬边界已锁，选型记录落地后回填本文。
- 入库管线（分段/清洗）延实现期；若引入异步流水线触发 §5 豁免失效条件。
- KB 条目纳入 @mention（KB-03/REF-01）待与 REF 联合扩展 RefInjection/解析协议。
- 供给结构与扩展表列定义进入实现前登记 SYS（控制态同批）。

## §9 后续动作

1. 注册（generate）：KB-RETRIEVE 链路、BR-KB-DATA-001/002、TERM-KB-001、REQ_MAP。
2. 待人类宣布 KB G2 闭合；实现期检索选型后回填 §3。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-11 | v1.0 | 初版（KB G2）：人类决策落规则——workspace 级隔离（BR-KB-DATA-001，含主动入库可溯源/删除即不召回）；本地检索硬边界（BR-KB-DATA-002，禁云端 embedding、违反即产品边界变更）；状态机豁免复核成立并列失效条件（解除 KB-04）；注册链路 KB-RETRIEVE、术语 KnowledgeEntry；REQ_MAP（REQ-KB-001~004） |
| 2026-07-04 | v1.1 | 上游约束文件名指针更新（A-AG-*/A-DE-* 文件更名为 代理*/DiffEdit*，2026-07-04 人类确认；文档编号不变，无语义变更） |
