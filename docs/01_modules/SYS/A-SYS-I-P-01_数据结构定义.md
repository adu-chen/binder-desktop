---
文档编号：SYS-I-P-01
文档状态：A
负责模块：SYS
文档职责：Binder 跨模块共享数据结构与 workspace.db 持久化 schema 基线技术设计——跨模块传递实体/类型契约、持久化 schema 基线与版本化迁移契约框架；承接 REQ-SYS-002
上游约束：A-SYS-M-D-01_系统底座功能需求描述.md（REQ-SYS-002）、A-CORE-C-D-01_产品概述.md（REQ-CORE-001/002/007）、A-CORE-C-P-01_APC.md（§4.1 SYS 职责、§9 安全/存储模型）、A-SYS-C-T-01_系统总体架构.md（§2 分层、§5 X-SEQ-002）、ADU.md（§3.1 RULE/CHAIN/CONSTRAINT 格式）
直接承接：A-SYS-I-P-02 IPC 协议（按本文契约封装跨进程载荷）、A-SYS-M-S-01 状态机总图（diff/timeline 状态机引用本文字段契约）、WS/DE/AG 等模块 L2（持久化字段与转移语义在此契约基线上承接）
使用边界：本文是跨模块共享数据结构与 workspace.db schema 基线的 SSOT；不定义具体序列化格式/最终列与索引（随 WS G2 联合，SY-03）、不定义 diff/timeline 的状态转移与 guard（归 DE/WS 状态机 M-S）、不定义 IPC 报文（归 A-SYS-I-P-02）、不定义云协议字段（归 A-SYS-I-P-03）
变更要求：跨模块共享实体的字段/枚举集合变更须回写本文并经 REQ_MAP 复核；schema 基线变更须遵守 BR-SYS-PERSIST-001 版本化迁移契约；ID 一经被规则/代码引用不可更改（ADU §3.0）
治理成熟度：draft
---

# Binder 系统数据结构定义（SYS · L2 技术设计）

## §1 文档定位

本文是 SYS 模块 L2 技术设计层文档，承接 REQ-SYS-002（跨模块共享数据结构与持久化 schema 基线），
承担一项 SSOT 职责：**跨模块传递的共享实体/类型契约**，以及 **workspace.db 持久化 schema 基线与
版本化迁移契约框架**。

调用与依赖关系：

```
REQ-SYS-002（L1）→ 本文（共享数据结构 SSOT）→ REQ_MAP → RULE/CHAIN/CONSTRAINT → 代码(@GOV)
本文契约 ←被引用← A-SYS-I-P-02（IPC 载荷封装）、A-SYS-M-S-01（状态机字段）、WS/DE/AG L2（持久化承接）
```

边界（与 A-SYS-M-D-01 SY-03 一致）：

- 本文定义跨模块**共享**结构与持久化**基线骨架**；某模块私有、不跨边界的结构不在本文。
- workspace.db 的**完整最终列定义与索引**随 WS G2 联合确定（SY-03），本文只锁基线列与迁移契约框架。
- diff / timeline 的**状态转移、guard、终态语义**归 DE / WS 状态机（A-SYS-M-S-01 与各模块 M-S），
  本文只锁其**字段与枚举值集合的单一契约来源**，不锁转移。
- 具体序列化格式、ID 生成算法属实现/技术选型，延 L2 实现与 G5，本文以语言无关的字段语义描述。

## §2 数据结构分层与边界

```
共享实体层（SYS SSOT，§3）      跨模块传递的实体/类型/枚举的唯一契约来源
        │  被 IPC 载荷与持久化引用
持久化基线层（SYS SSOT，§4）    workspace.db schema 基线 + 版本化迁移契约框架
        │  完整列/索引随 WS G2 联合
模块私有层（各模块 L2，非本文）   模块内部不跨边界的结构、UI 视图模型等
```

- 共享实体以**语言无关字段语义**描述：列出字段名、语义、是否标识符/枚举，不锁定具体标量类型与序列化。
- 持久化基线只给**基线列 + 迁移契约**，标注"完整列与索引归 WS G2 联合"，避免制造虚假精度。
- 本地优先（REQ-CORE-001）：用户内容文件在 workspace 内，workspace.db 持久化的是**控制态**
  （pending_diffs、时间轴等），不是用户文档正文（正文以文件形式落 workspace）。

## §3 跨模块共享实体契约（DATA）

> 字段以语义描述，不锁定具体类型/序列化。标识符字段为不透明 ID（生成算法属实现选型，§9 盲区）。

**WorkspaceRef · workspace 引用**（边界标识，REQ-CORE-001）

| 字段 | 语义 | 备注 |
|---|---|---|
| workspace_id | workspace 不透明标识 | 标识符 |
| root_path | 用户选定的本地根目录绝对路径 | 文件 I/O / 搜索 / diff / 上下文的唯一边界 |
| is_active | 是否当前活动 workspace | — |

**DocRef · 文档引用**（workspace 内文档定位）

| 字段 | 语义 | 备注 |
|---|---|---|
| workspace_id | 所属 workspace | 关联 WorkspaceRef |
| rel_path | workspace 内相对路径 | 不得越出 root_path（X-SEQ-002 / APC §9 文件系统边界） |
| doc_type | 文档类型（md / docx 等） | 枚举，扩展值随 DOC/ED 模块补充 |

**PendingDiff · 待裁决 diff（跨模块持久化共享结构）**（REQ-CORE-002）

| 字段 | 语义 | 备注 |
|---|---|---|
| diff_id | diff 不透明标识 | 标识符 |
| target | 目标文档引用 | DocRef |
| origin_module | 生成来源模块编码 | DE / AG / CBT 等 |
| status | diff 状态 | **枚举值集合：pending / accepted / rejected / expired** |
| created_at | 创建时间 | — |
| payload_ref | diff 内容载荷引用 | 内容**形状归 DE**，本文只定其在共享结构中的占位 |

> 边界：本文锁定 `status` 的**枚举值集合**为单一来源（防跨 session 漂移），但
> "pending→accepted/rejected、超时/外部变更→expired" 的**转移、guard、终态不可逆语义**归 DE
> 状态机（A-SYS-M-S-01 / DE M-S）。仅 accepted 触发落盘的写入语义亦归 DE/WS，不在本文。

**TimelineEntry · 文档级时间轴条目（基线）**（REQ-CORE-007，主责 WS）

| 字段 | 语义 | 备注 |
|---|---|---|
| entry_id | 条目不透明标识 | 标识符 |
| target | 目标文档引用 | DocRef |
| version_seq | 文档版本序 | 单调递增 |
| label | 版本标注 | 可空 |
| created_at | 创建时间 | — |

**RefInjection · 引用注入载荷（基线）**（REQ-CORE-006，主责 REF，2026-06-11 随 REF G2 登记）

| 字段 | 语义 | 备注 |
|---|---|---|
| target | 引用目标文档引用 | DocRef |
| block_ids | 块级锚集合 | 可空（路径级引用时空）；blockId 契约归 ED |
| content | 读取时点的目标内容 | 经 WS 读取，不缓存陈旧内容跨回合复用 |
| origin_label | 来源标识 | 消费方（AG/CBT）可溯源到引用目标 |

> 边界：本文只给 RefInjection 跨模块共享**字段基线**（REF→AG/CBT 传递）；解析、锚失效校验与
> 组装语义归 REF（A-REF-M-T-01）；上下文预算与裁剪归 AG（BR-AG-DATA-001）。

**MemoryInjection · 记忆注入载荷（基线）**（主责 AST，2026-06-12 随实现前置批量登记）

| 字段 | 语义 | 备注 |
|---|---|---|
| memory_kind | 记忆类型（分级依据） | 类型基线归 A-AST-M-T-01 §3 |
| content | 记忆内容 | — |
| origin_turn | 来源回合标识 | 消费方可溯源（AgentTurn） |

**KnowledgeInjection · 知识召回载荷（基线）**（主责 KB，2026-06-12 随实现前置批量登记）

| 字段 | 语义 | 备注 |
|---|---|---|
| content | 召回的条目内容 | 读取时点 |
| source_ref | 入库来源标识 | 可溯源到源文档/外部文本 |
| organization | 分类/标签 | 可空 |

> 边界：两载荷为 AST/KB→AG 跨模块传递基线；筛选/召回语义归各 owner L2，预算裁剪归 AG。
> 三类注入载荷（Ref/Memory/Knowledge）结构上并列，不强行统一（target 语义不同）。

> 边界：本文只给 TimelineEntry 跨模块共享**字段基线**；时间轴**回溯/还原语义**与完整模型归 WS
> （REQ-CORE-007，PO-05 已改归 WS-G2 技术核对）。

<!-- RULE
rule_id: BR-SYS-DATA-002
主链路: DATA-MIGRATE
域: DATA
-->
跨模块传递的共享实体（WorkspaceRef / DocRef / PendingDiff / TimelineEntry 等）及其枚举值集合
（如 PendingDiff.status 的 pending/accepted/rejected/expired）以本文为唯一契约来源；任何模块不得
在本文之外私自重定义跨模块共享结构的字段或枚举集合。状态转移语义、guard 与终态不可逆性不属本规则，
归各 owner 模块状态机（A-SYS-M-S-01 及各模块 M-S）。

## §4 workspace.db 持久化 schema 基线（PERSIST）

> 持久化的是控制态（pending_diffs、时间轴），不是用户文档正文。以下为**基线**，完整列、约束与
> 索引随 WS G2 联合确定（SY-03），本文锁定基线列集合与版本化迁移契约框架。

**基线表（baseline，非最终）**

| 表 | 基线列 | 职责 | 完整定义归属 |
|---|---|---|---|
| schema_meta | schema_version（整数，单调递增）、applied_at | 记录当前 schema 版本，驱动迁移 | SYS（本文） |
| pending_diffs | diff_id(PK)、workspace_id、rel_path、origin_module、status、created_at、payload_ref | 持久化待裁决 diff（重启可恢复） | 基线 SYS / 完整列 WS·DE G2 |
| timeline | entry_id(PK)、workspace_id、rel_path、version_seq、label、created_at | 持久化文档级时间轴 | 基线 SYS / 完整列 WS G2 |

**控制态扩展表（基线，2026-06-12 实现前置批量登记；均含 workspace_id 外键——workspace 级隔离，
2026-06-11 人类决策；payload/定义载荷形状归各 owner 模块；列随实现期微调，迁移受
BR-SYS-PERSIST-001 约束）**

| 表 | 基线列 | 职责 | owner |
|---|---|---|---|
| agent_turns | turn_id(PK)、workspace_id、rel_path(可空)、payload_ref（回合内容载荷）、created_at | 回合/对话历史（BR-AG-PERSIST-001：本地可恢复不上云） | AG |
| build_runs | run_id(PK)、workspace_id、status、plan_ref（任务计划载荷）、interrupted_at(可空)、created_at、updated_at | 构建控制态（BR-CBT-PERSIST-001：重启恢复至中断点或明确关闭，无悬挂） | CBT |
| workflow_runs | run_id(PK)、workspace_id、workflow_id、current_step、status、created_at、updated_at | 工作流执行控制态（BR-TPL-STATE-001：可恢复无悬挂） | TPL |
| templates | template_id(PK)、workspace_id、template_kind、body、organization、created_at、updated_at | 模板定义（BR-TPL-DATA-001） | TPL |
| workflows | workflow_id(PK)、workspace_id、definition_ref（步骤序列载荷）、created_at、updated_at | 工作流定义（BR-TPL-DATA-001） | TPL |
| memory_entries | entry_id(PK)、workspace_id、memory_kind、content、origin_turn、status(active/revoked)、created_at、revoked_at(可空) | 记忆条目（BR-AST-DATA-001/STATE-001） | AST |
| knowledge_entries | entry_id(PK)、workspace_id、content、organization、source_ref、created_at、updated_at | 知识条目（BR-KB-DATA-001） | KB |

**版本化迁移契约框架**

- `schema_version` 单调递增，是迁移的唯一驱动；每次结构变更必须配一次版本递增 + 迁移步骤。
- 迁移必须保持既有 `pending_diffs` / `timeline` 持久化态不被破坏（向前迁移不丢数据）。
- 应用重启后，已持久化的控制态（含未裁决的 pending_diffs、时间轴）必须可恢复（REQ-SYS-002 验收）。

<!-- CHAIN
chain_id: DATA-MIGRATE
主责模块: SYS
status: active
risk_tier: P1
-->

<!-- RULE
rule_id: BR-SYS-PERSIST-001
主链路: DATA-MIGRATE
域: PERSIST
-->
workspace.db 的 schema 必须版本化：以 `schema_meta.schema_version` 单调递增为唯一迁移驱动，每次结构
变更配套版本递增与迁移步骤；迁移不得破坏既有 `pending_diffs` / `timeline` 持久化态（不丢数据）；
应用重启后已持久化的控制态必须可恢复。本规则只约束 schema 版本化与迁移/恢复契约，不约束业务写入时机
（diff 落盘时机归 DE/WS）。

## §5 跨模块约束（CONSTRAINT）

<!-- CONSTRAINT
constraint_id: X-SHARED-001
摘要: 跨模块传递的实体/类型/枚举集合必须经 SYS 数据结构契约层（本文）定义，模块不得私自定义跨模块共享结构
涉及模块: SYS, WS, ED, DE, AG, CBT, DOC, PRV, REF, AST, KB, TPL, ACCT, ENT
链路影响: DATA-MIGRATE
-->

## §6 术语归属说明（为何本文不注册 TERM）

workspace、diff / pending_diff、文档时间轴等核心业务名词的 **TERM 主责是其 owner 模块**
（workspace / timeline → WS，diff 终态 → DE；ADU §3.2「只主责模块注册」），将在各模块 L2 注册，
本文不抢注，避免 TERM_DUPLICATE 与主责漂移。本文以**数据结构契约**身份引用这些实体；待 owner 模块
注册 TERM、且 SYS 代码以 `term_ref` 引用时，按 ADU §3.2 关联其 TERM。SYS 数据结构层自身未引入需要
术语治理的新业务名词（schema/迁移为技术框架词，不注册）。

## §7 需求到规则映射（REQ_MAP）

<!-- REQ_MAP
req_id: REQ-SYS-002
target_type: CHAIN
target_id: DATA-MIGRATE
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-002
target_type: RULE
target_id: BR-SYS-DATA-002
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-002
target_type: RULE
target_id: BR-SYS-PERSIST-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-002
target_type: CONSTRAINT
target_id: X-SHARED-001
status: mapped
-->

## §8 测试与验收

- **共享结构单一来源**（BR-SYS-DATA-002 / X-SHARED-001）：跨模块共享实体与枚举集合仅本文定义一处；
  代码阶段以 @GOV 引用 BR-SYS-DATA-002，audit 无重复定义。
- **持久化可恢复**（BR-SYS-PERSIST-001，REQ-SYS-002 验收）：构造含未裁决 pending_diffs 与时间轴的
  workspace.db，经一次 schema 版本递增迁移后重启，控制态完整可恢复、无数据丢失。
- **枚举集合一致**：PendingDiff.status 枚举集合（pending/accepted/rejected/expired）与 DE 状态机
  （A-SYS-M-S-01）的状态集合对齐，无第三处定义。
- 量化 NFR（迁移耗时、db 体量）延 G5 标定（APC §9）。

## §9 技术债务与盲区

- workspace.db **完整列、约束、索引**与 WS 持久化实现联合确定（SY-03，WS G2），本文为基线骨架。
- 不透明 ID 生成算法（ULID/UUID/自增等）属实现选型，延 L2 实现/G5，本文只约定"不透明标识符"语义。
- 具体序列化格式（payload_ref 指向的 diff 内容编码）归 DE；本文只占位。
- diff / timeline 的状态机与转移归 A-SYS-M-S-01 与 DE/WS M-S，本文只锁字段与枚举集合。

## §10 后续动作

1. 本文落地后随下一次 `npm run governance:generate` 注册：DATA-MIGRATE 链路、BR-SYS-DATA-002 /
   BR-SYS-PERSIST-001 规则、X-SHARED-001 约束、REQ-SYS-002 的 REQ_MAP 进 ADUS。
2. 继续 SYS G2：A-SYS-I-P-02（IPC / Tauri command 协议，承接 REQ-SYS-001，引用本文共享结构作为载荷）、
   A-SYS-M-S-01（状态机总图，引用本文 PendingDiff/TimelineEntry 字段契约）。
3. WS G2 启动时与本文联合敲定 workspace.db 完整 schema（解除 SY-03），并在 WS L2 注册
   workspace / timeline 相关 TERM。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-10 | v1.0 | 初版（SYS G2）：跨模块共享实体契约（WorkspaceRef/DocRef/PendingDiff/TimelineEntry，字段语义级）；workspace.db schema 基线与版本化迁移契约框架；规则 BR-SYS-DATA-002（共享结构单一来源）/ BR-SYS-PERSIST-001（schema 版本化迁移与重启可恢复）；链路 DATA-MIGRATE（P1）；约束 X-SHARED-001；REQ_MAP（REQ-SYS-002）；术语归属说明（不抢注 WS/DE 的 TERM）；测试验收与债务盲区 |
| 2026-06-11 | v1.1 | 随 REF G2 登记（X-SHARED-001 回写）：§3 新增 RefInjection 引用注入载荷共享结构基线（target/block_ids/content/origin_label，主责 REF，REF→AG/CBT 跨模块传递）；解析/校验/组装语义归 A-REF-M-T-01，裁剪归 AG；受 BR-SYS-DATA-002 单一来源约束覆盖，无新增规则 |
| 2026-06-12 | v1.2 | 进入实现前批量登记（X-SHARED-001/BR-SYS-PERSIST-001 回写，解除 AG/CBT/TPL/AST/KB 五模块"实现前登记"承诺）：§3 新增 MemoryInjection（memory_kind/content/origin_turn，主责 AST）与 KnowledgeInjection（content/source_ref/organization，主责 KB）注入共享结构基线；§4 新增七张 workspace.db 控制态扩展表基线——agent_turns（AG 回合历史）/build_runs（CBT 构建控制态）/workflow_runs（TPL 执行控制态）/templates·workflows（TPL 定义）/memory_entries（AST）/knowledge_entries（KB），均 workspace 级隔离（2026-06-11 人类决策）、payload/定义形状归各 owner、迁移受 BR-SYS-PERSIST-001 约束；无新增规则（受 BR-SYS-DATA-002 覆盖） |
