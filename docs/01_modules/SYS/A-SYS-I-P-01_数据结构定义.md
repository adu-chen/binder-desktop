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
  （ed_session_snapshots、pending_diffs、时间轴等），不是用户文档正文（正文以文件形式落 workspace）。

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
| effect_status | diff 效果态（2026-07-20 新增，APP-CLOSE-DIFF-LIFECYCLE-01） | **枚举值集合：not_applicable / awaiting_save / saved / discarded_on_abnormal_exit**；`awaiting_save` 仅限当前进程会话，启动清算必须转终态；语义与转移规则 SSOT 归 `BR-SYS-DATA-005`（A-SYS-M-T-02 §4） |
| terminal_reason | 终态原因（可空，2026-07-20 新增） | expired 沿用 DE 既有过期原因集合，新增 `abnormal_exit` 一值，标识"应用启动异常退出清算所致" |

> 边界：本文锁定 `status`/`effect_status`/`terminal_reason` 的**枚举值集合**为单一来源（防跨
> session 漂移），但 "pending→accepted/rejected、超时/外部变更→expired" 的**转移、guard、终态
> 不可逆语义**归 DE 状态机（A-SYS-M-S-01 / DE M-S）；`effect_status` 的转移语义归
> `BR-SYS-DATA-005`（SYS，A-SYS-M-T-02 §4）。副作用必须遵循 `BR-CORE-STATE-001~003`：已有
> LogicalState 的 pending 可先受控预览，仍保持 `pending`；其用户显式接受只改变裁决态，保存有效 pending 预览会先置 `accepted/awaiting_save` 再写入
> DiskState。没有 LogicalState 的 pending 仅挂载在 DiffCard，用户接受后才由 DE/WS 原子写入
> DiskState。若该背景 pending 期间目标文档被打开，ED 先以同一 payload 建立并预览 LogicalState，
> 卡片仍为 `pending`，之后回到已有 LogicalState 分支。`status=accepted` 不代表内容已落盘——
> `effect_status` 才是"内容是否已生效"的权威字段，区分裁决态与落盘效果两个维度（解除历史审计
> 发现的"accepted 即完成"歧义）。

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
| block_ranges | 选区级锚集合（{blockId, start, end}，块内纯文本偏移区间） | 可空（2026-07-03 随 PI-ED-10 登记）；CharAnchor 契约归 ED BR-ED-DATA-004，有效期=编辑会话内 |
| content | 读取时点的目标内容 | 经 WS 读取，不缓存陈旧内容跨回合复用 |
| origin_label | 来源标识 | 消费方（AG/CBT）可溯源到引用目标 |

> 边界：本文只给 RefInjection 跨模块共享**字段基线**（REF→AG/CBT 传递）；解析、锚失效校验与
> 组装语义归 REF（A-REF-M-T-01）；上下文预算与裁剪归 AG（BR-AG-DATA-001）。

**ReferencePayload · 引用输入载荷（基线）**（REQ-CORE-006，主责 REF，2026-07-08 随引用令牌修复登记）

| kind | 字段 | 语义 |
|---|---|---|
| doc | relPath, label? | workspace 内文档或资源文件路径级引用 |
| selection | relPath, blockId, start?, end?, text, label? | 编辑器选区引用；text 为用户选中快照，start/end 为块内纯文本偏移 |
| session | sessionId, title, messages[] | 用户显式拖入的代理会话快照 |
| memory | entryId?, title?, content | AST 记忆条目快照 |
| knowledge | entryId?, title?, content, sourceRef? | KB 知识条目快照 |
| template | templateId?, name, content | TPL 模板条目快照 |
| url | url, title? | 外部链接引用，不包含抓取内容 |
| long_text | text, charCount | 外部长文本粘贴快照 |
| file | name, path?, size? | 外部文件引用（2026-08-07 REF-REFERENCE-REPAIR-01 F5 登记）：仅元数据，不读内容，守 X-FORBID-004 |

> 边界：ReferencePayload 是输入侧显式引用结构（Composer→AG）；它不等同于 RefInjection。workspace
> 文档内容读取、锚校验与发送时组装归 REF/AG；若引用目标是已打开文档，校验必须基于 OpenDocSnapshot
> 中的 LogicalState，不得回退 DiskState。AST/KB/TPL 条目的生成、筛选、管理归各 owner。
> 输入流展示形态为 ReferenceToken（A-REF-M-T-01 TERM-REF-004），视觉生命周期不在本文定义。

**OpenDocSnapshot · 已打开文档 LogicalState 快照（基线）**（主责 ED，AG/DE 消费，2026-07-18 登记）

| 字段 | 语义 | 备注 |
|---|---|---|
| rel_path | workspace 内文档路径 | DocRef 的路径部分；必须规范化后传输 |
| content | 当前 LogicalState 的序列化内容 | 激活文档优先来自 ProseMirror `doc` 实时序列化；active binding 未就绪时可来自当前文档 `EditSession.content`；非激活打开文档来自 `EditSession.content` |
| baseline | 可空磁盘加载基线 | BDocx 持久恢复必填；用于恢复同一来源媒体目录并继续承担保存冲突保护；旧镜像可空但不得据此静默覆盖 |
| edit_format | 磁盘解析/保存格式 | text / markdown / html / html_file；与 ED session 格式一致，不能用于推断 content 编码 |
| content_format | content 的编码格式 | text / html；受支持 Markdown 的 LogicalState 为 html，源码保护 Markdown 为 text |
| source | 快照来源 | active_editor / active_editor_session / open_editor_session |
| dirty | 当前 session 是否偏离 DiskState | 仅描述 LogicalState 与加载基线关系，不代表已保存 |
| session_version | ED session 版本 | 每次受控修改递增；用于防多重编辑与基线偏移 |
| content_hash | content 的稳定哈希 | 与 `session_version` 共同构成自动应用基线 |
| captured_at | 采集时间 | 仅用于诊断，不作为一致性判定主键 |
| block_snapshots | 可空层级块投影 | 仅活动富文档必填；必须与 content/session_version/content_hash 来自同次 ProseMirror LogicalState |

> 边界：OpenDocSnapshot 是已打开文档进入 AG/DE 链路的唯一读取门禁。AG 轮次、只读工具、
> ReferencePayload.selection 校验、DiffProjection 生成必须优先使用它。未打开文档没有
> OpenDocSnapshot 时，AG 才允许读取 DiskState 快照。
> `ed_session_snapshots` 现有表无独立 baseline 列时，允许持久化适配器在数据库 `content` 列内部使用
> 版本受控的短 envelope 编码 BDocx baseline，但 `ed.list_session_snapshots` 返回前必须解码并移除；
> IPC 与 ED/AG/DE 消费到的 `content` 始终是纯 LogicalState，内部 envelope 不得进入编辑器或写回 DOCX。

**AgentEditableBlockSnapshot · 富文档层级块投影（基线）**（主责 ED，AG 消费，2026-07-30 登记）

| 字段 | 语义 | 备注 |
|---|---|---|
| block_id / block_index | 稳定锚与同次投影顺序 | block_id 全文唯一；index 只作显示/诊断 |
| block_type / node_type | HTML 语义标签与 ProseMirror schema 类型 | 不得互相猜测替代 |
| role | content / container / atom | container/atom 只提供上下文 |
| parent_block_id / depth / ordinal | 父块、层级与同父顺序 | 列表等嵌套结构不得展平为一个文本块 |
| text / marks / attrs | 当前内容与样式投影 | text 偏移为 UTF-16 code unit |
| node_json | 当前 ProseMirror 节点快照 | 结构化内容编辑的节点 Schema 基线 |
| fingerprint | node_json 的稳定指纹 | 不包含易漂移文档 position |
| capabilities | 当前块动态可写能力 | `replace_node` / `style_ops`；空数组表示只读 |

> 活动富文档的 AgentEditableSnapshot.blocks 必须直接由 `block_snapshots` 构造。HTML 字符串扫描只可
> 用作未打开文档的只读兼容，不得成为富文档写操作、Diff 或回滚的结构真值。

**StructuredBlockEditPayload · 结构化内容块候选（基线）**（主责 AG/ED，DE 承载，2026-07-30 登记）

| 字段 | 语义 | 备注 |
|---|---|---|
| payload_version | 载荷版本 | 当前固定 1 |
| base | session_version + content_hash | 必须与当前活动 LogicalState 精确匹配 |
| operations[] | replace_node 批次 | 1..32，block_id 不得重复 |
| operation.block_id / fingerprint / expected_node_type | 稳定锚、节点指纹与类型前置条件 | 任一失效整批拒绝 |
| operation.before_node / after_node | 精确前后 ProseMirror 节点 JSON | before_node 由后端从可信快照补齐；类型、blockId、attrs 必须保持 |

> `operations[]` 先整批校验，再在一个 ProseMirror transaction 内按位置倒序应用。拒绝时只在当前
> 节点仍等于 after_node 指纹时精确反向恢复 before_node；否则进入 conflict，不得整篇覆盖用户后置编辑。

**EditorOpenTab · 编辑器打开标签持久态（基线）**（主责 ED，2026-07-18 登记）

| 字段 | 语义 | 备注 |
|---|---|---|
| rel_path | workspace 内文档路径 | 仅文档标签；预览标签不进入该控制态 |
| kind | 标签类型 | 当前固定为 doc，保留扩展位 |
| open_index | 关闭应用时的标签顺序 | 用于重启恢复打开集合与顺序 |
| is_active | 是否当前文档 | 每个 workspace 最多一个 active；恢复后驱动 `openPath` |
| updated_at | 写入时间 | 诊断字段，不作为一致性主键 |

> 边界：EditorOpenTab 是“哪些文档处于打开状态”的控制态真源；OpenDocSnapshot / `ed_session_snapshots`
> 是“这些打开文档的 LogicalState 内容镜像”。启动恢复必须先以 EditorOpenTab 限定打开集合，再恢复对应
> session 镜像；孤立 session 镜像不得把文档重新打开。

**DiffProjection · Diff 投影共享载荷（基线）**（主责 DE，AG 生产/ED 消费，2026-07-18 登记）

| 字段 | 语义 | 备注 |
|---|---|---|
| diff_id | diff 不透明标识 | 与 PendingDiff 关联 |
| rel_path | 目标文档路径 | DocRef 路径部分 |
| patch_kind | 修改形态 | replace_block / block_ops / replace_document / style_ops / table_ops / image_ops |
| before_text / after_text | 候选变更文本 | 内容在转正前属于 LogicalCandidate |
| block_id / block_index | 块定位信息 | 块级替换至少应提供一种；锚语义归 ED |
| draft_mode | 应用路径 | current_editor / open_session / background |
| snapshot_source | 生成基线来源 | active_editor / active_editor_session / open_editor_session / disk_snapshot |
| base_session_version | 生成时 ED session 版本 | 已打开文档必填；未打开文档为空 |
| base_content_hash | 生成时内容哈希 | 已打开文档必填；未打开文档可使用 DiskState baseline |
| block_ops | 可空 StructuredBlockEditPayload | patch_kind=block_ops 时必填；执行与详细 Diff 的唯一真值 |

> 边界：DiffProjection 是跨 AG/DE/ED 的最小互操作载荷。`draft_mode=open_session` 时，
> ED 必须以 `base_session_version` 与 `base_content_hash` 校验当前 LogicalState；校验失败不得应用，
> DiffCard 进入不可裁决的失效/冲突状态。`draft_mode=background` 只允许目标文档未打开。

**BDocxCapabilityManifest · BDocx 代理编辑能力清单（基线）**（主责 SYS，AG/ED/DE/DOC 消费，
2026-07-28 登记）

| 字段 | 语义 | 备注 |
|---|---|---|
| manifest_id / version | 稳定清单标识与语义版本 | 运行时另计算原始清单 SHA-256，禁止手工维护第二 hash |
| editable_kind / content_format | 适用编辑模型 | 固定为 bdocx / html |
| style_ops | mark、mark attrs、块属性、批上限 | 真实操作名和参数约束 |
| table_operations | 表格操作名与字段形状 | 静态声明不取代动态 table capabilities |
| manual_only | 人工专属能力 | 不得投影进代理操作 enum |
| diff_fidelity | 各操作族 Diff 呈现等级 | block / summary / structured / none |
| roundtrip | StyleSubset 五档 | 与 DOC `classify_style` 逐项一致 |

> 完整协议、TERM 与规则 SSOT 见 A-SYS-I-P-04（TERM-SYS-002 / BR-SYS-DATA-006）。本文只登记跨模块
> 字段基线；提示词、工具 Schema 与执行前校验必须消费同一份版本化机器清单。

**AgentSession / AgentTurnHistory · 代理编辑历史控制态（基线）**（主责 AG，2026-07-17 随
`BR-AG-PERSIST-001` 具体化登记）

| 结构 | 字段 | 语义 |
|---|---|---|
| AgentSession | session_id、workspace_id、title、mode、archived、open_index、is_active、created_at、updated_at、last_active_at | 代理编辑会话标签元数据；恢复存在状态、顺序、标题、模式、归档/打开与激活态 |
| AgentTurnHistory | turn_id、session_id、workspace_id、rel_path、status、payload_ref、created_at、updated_at | 一个持久化代理回合；`status` 枚举值为 completed / error / cancelled / gate_rejected / draft |
| HistTurn | role、content、rel_path?、turn_id? | 发送给 AG 回合的历史消息投影；`rel_path` 用于按当前文档分流历史，缺失或跨文档历史不得作为当前文档事实 |
| AgentTurnPayload | payload_ref、workspace_id、payload_json、payload_size、created_at | AG 私有回合内容载荷；只保存消息、工具摘要与 Diff 引用，不保存 diff 内容快照 |

> 边界：AgentTurnPayload 的 JSON 形状归 AG；本文锁跨模块共享字段与状态枚举。`draft` 表示流式中断/
> 应用关闭留下的半成品，重启后不得投影为 completed。DiffCard 历史只保存 `diff_id`/`rel_path` 引用，
> 实际 diff 内容与当前状态仍以 DE/WS 持久化表为准。

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

### 3.1 Agent 1.0 共享运行时对象契约（R5-A / R5-B / R5-C）

> 承接 REQ-SYS-008/009/010/011/012（A-SYS-M-D-01）与冻结基线
> （A-CORE-X-D-02 §3/§4）。本轮只锚定各对象的**职责、身份关系、事件真值、隔离与迁移边界**；
> 具体字段、序列化、持久化表与状态机内部由 D2（DE/ED/WS）与 D3（AG/REF/AST）及 owner 模块 L2 承接，
> **不锁死实现细节**。这些对象的正式术语（RuntimeProfile、CapabilityGrant、TaskRun 等）由主责模块在
> 其 L2 按 ADU §3.2 注册；本文仅以数据结构契约身份引用，不抢注。

| 共享对象 | 职责 | 身份关系 | 事件真值 | 隔离与迁移边界 |
|---|---|---|---|---|
| `RuntimeProfile`（运行时档位） | 声明一次 Agent 交互可用的运行时能力档位；1.0 唯一 `agent-edit-1.0` | profile_id 作为回合/意图/授权的最顶层收窄约束 | 缺失或未知 profile 必须在 Gate、provider、工具与 Proposal **之前**失败关闭，不产生任何可执行事件 | 已注册工具、Prompt、前端参数或兼容默认值都不构成 profile 的授权来源；Build/CBT 等 P2 不进入该 profile |
| `AgentSession`（代理会话） | 用户可见的代理编辑会话；是**展示、持久化与恢复的隔离键** | session_id 属于唯一 workspace；一个 workspace 可有多会话 | 会话内消息、工具卡、DiffCard、引用、未保存上下文与回合按事件顺序呈现，跨会话不串线 | workspace 只是外层范围，不能替代 AgentSession 作为恢复隔离键；会话归档/删除后的回合只读或不可恢复须明确 |
| `AgentTurn`（代理回合） | 一个用户发送对应一个回合；收敛当前目标、意图与一次交互结果 | turn_id 属于唯一 AgentSession；可关联 DocRef、TaskRun、Proposal 与结构化事件 | **澄清是等待用户输入的非终态**；取消可在任一活动阶段生效；完成/失败/拒绝/取消互斥且**首个终态获胜**；首终态后到达的迟到事件或工具不得产生文档、卡片或落盘副作用 | 回合上下文（引用快照、外部内容、显式记忆选择）只在回合内有效；重启不伪造成功；新回合重新校验上下文 |
| `IntentResult`（本轮意图结果） | 固定本轮任务类型、目标、预期副作用、所需上下文、允许工具与确认点 | 随回合生成；写入意图目标或范围不明时只能澄清或生成目标计划 | 是结构化收窄边界而非自然语言结论；“只分析不修改”不获得 Proposal | 不展示模型内部推理；不因历史意图覆盖当前明确请求 |
| `CapabilityGrant`（当轮能力授权） | 本轮允许的工具/操作集合；逐层收窄（profile ∩ 任务类型 ∩ 目标 ∩ 副作用等级 ∩ 格式/动态能力） | 与回合/意图绑定；工具须同时通过全部校验才可见/可执行 | 注册表存在 ≠ 本轮可调用；越权调用失败关闭 | grant 只在当前回合有效；跨回合不隐式继承，除非显式重新授权 |
| `TaskRun`（受限任务实例） | 同一用户目标内组织多个受限步骤/AgentTurn 的编排实例；**不是后台队列** | task_run_id 属于唯一 AgentSession 与用户目标；步骤/回合/Proposal 可追溯到 TaskRun | 不自动重试、不跨重启续跑、不越过确认点；“工作中”只能由真实回合、步骤或工具事件驱动，等待补充/卡片 pending/失败/取消不伪装执行 | 同文档连续 Proposal 以最新 LogicalState 接力；前卡因基准推进失效，不形成补丁栈或任务级回滚；无 LogicalState 的 background pending 必须先迁移（BR-CORE-STATE-003）或完成背景分流裁决（BR-CORE-STATE-002）后才可继续 |
| `TaskStep`（任务步骤） | TaskRun 内有依赖的受限步骤（读取/搜索/分析/生成/提案/等待确认等） | step_id 属于唯一 TaskRun；每步关联输入来源、允许工具、确认点、输出与失败语义 | 每步重新取得当前最小授权；只读/研究可连续推进；进入 W1/W2/W3 的步骤不得在不可见后台继续 | 中断可重试但用户显式重试创建新回合并重新校验；取消停止未开始步骤，保留已成功预览内容 |
| `StructuredVisibleEvent`（结构化可见事件） | 用户可见的回合/任务/Proposal 状态事实（意图结论、读取/工具、提案卡、等待确认、终态原因） | 事件顺序承载于 AgentSession；终态事件为回合锁存 | 只含真实运行事件，不含模型内部推理；等待、失败、取消、中断不得伪装为执行 | 终态事件后不得出现可产生副作用的迟到事件；具体形状、字段与 UI 投影归 D3（AG）与前端设计 |
| `ExternalContentRef`（外部内容引用） | 用户显式附加、且有可用只读解析路径的外部文件内容，作为当前任务的一次性 data-only 内容引用 | 引用携带文件身份、来源入口、已读/未读标记、读取时点与当前任务范围 | 不支持/不可读只保留元数据不伪称已读；外部内容不能改变系统约束、目标、授权或确认点 | 只在当前 AgentTurn/TaskRun 当前任务内有效；任务结束、重启、跨会话/跨 workspace 不自动恢复、复读、导入、编辑或默认持久化；不自动转存为项目记忆 |
| `MemorySelectionRef`（显式项目记忆引用） | 用户管理的 workspace 本地项目记忆条目，经用户明确选择以 data-only 引用进入当前回合 | 引用携带条目身份与选择动作；仅用户显式选择 | Agent 1.0 不自动写入、不提议写入、不自动检索或注入；普通发送、附件、会话/编辑历史不自动转存为记忆 | workspace 隔离；跨会话不串选择事实；条目删除/关闭后引用失效或明确标记 |

<!-- RULE
rule_id: BR-SYS-STATE-007
主链路: AG-TURN, IPC-CALL
域: STATE
-->
Agent 1.0 运行时共享对象（RuntimeProfile / AgentSession / AgentTurn / IntentResult / CapabilityGrant /
StructuredVisibleEvent）的事件真值与隔离以本文 §3.1 契约为唯一来源：仅 `agent-edit-1.0` 可启动，缺失或
未知 profile 必须在 Gate、provider、工具与 Proposal 前失败关闭；一个用户发送对应一个 AgentTurn，澄清是
等待用户输入的非终态，取消可在任一活动阶段生效，完成/失败/拒绝/取消互斥且首个终态获胜，首终态后的迟到
事件或工具不得产生文档、卡片或落盘副作用；已注册工具、Prompt、前端参数或兼容默认值不构成当轮授权；
可见事件只包含真实回合/工具/Proposal 事实，不包含模型内部推理。

<!-- RULE
rule_id: BR-SYS-STATE-008
主链路: AG-TURN, DE-DIFF
域: STATE
-->
TaskRun 只组织当前用户目标内的已授权步骤，不是后台队列：不自动重试、不跨重启续跑、不越过确认点；
“工作中”只能由真实回合、步骤或工具事件驱动，等待补充、卡片 pending、失败、取消或中断不得伪装为执行。
同文档连续 Proposal 必须以最新 LogicalState 为基准接力：前卡因基准推进失效且不可再接受/拒绝，不形成
补丁栈或任务级回滚；无 LogicalState 的 background pending Proposal 必须先迁移（BR-CORE-STATE-003）或
依 BR-CORE-STATE-002 完成背景分流裁决后才可继续。

<!-- RULE
rule_id: BR-SYS-DATA-007
主链路: REF-INJECT, AST-MEMORY
域: DATA
-->
外部文件内容仅在用户显式附加且存在可用只读解析路径时，作为当前任务的一次性 data-only 内容引用
（ExternalContentRef）；不支持/不可读只保留元数据不伪称已读，不得扫描目录、自动复读、导入、编辑或默认
持久化。项目记忆是用户管理的 workspace 本地资产，仅用户明确选择的条目（MemorySelectionRef）以 data-only
进入当前回合；Agent 1.0 不自动写入、不提议写入、不自动检索或注入。外部内容、会话历史与编辑历史均不得
自动转存为项目记忆。

> 边界：本文只给 TimelineEntry 跨模块共享**字段基线**；时间轴**回溯/还原语义**与完整模型归 WS
> （REQ-CORE-007，PO-05 已改归 WS-G2 技术核对）。

<!-- RULE
rule_id: BR-SYS-DATA-002
主链路: DATA-MIGRATE
域: DATA
-->
跨模块传递的共享实体（WorkspaceRef / DocRef / PendingDiff / TimelineEntry / ReferencePayload / OpenDocSnapshot / DiffProjection 等）及其枚举值集合
（如 PendingDiff.status 的 pending/accepted/rejected/expired）以本文为唯一契约来源；任何模块不得
在本文之外私自重定义跨模块共享结构的字段或枚举集合。状态转移语义、guard 与终态不可逆性不属本规则，
归各 owner 模块状态机（A-SYS-M-S-01 及各模块 M-S）。

## §4 workspace.db 持久化 schema 基线（PERSIST）

> 持久化的是控制态（ed_session_snapshots、pending_diffs、时间轴），不是用户文档正文。以下为**基线**，完整列、约束与
> 索引随 WS G2 联合确定（SY-03），本文锁定基线列集合与版本化迁移契约框架。

**基线表（baseline，非最终）**

| 表 | 基线列 | 职责 | 完整定义归属 |
|---|---|---|---|
| schema_meta | schema_version（整数，单调递增）、applied_at | 记录当前 schema 版本，驱动迁移 | SYS（本文） |
| pending_diffs | diff_id(PK)、workspace_id、rel_path、origin_module、status、created_at、payload_ref、effect_status（2026-07-20 新增）、terminal_reason（2026-07-20 新增，可空） | 持久化待裁决或已自动应用 diff；重启仅可恢复只读终态审阅：pending 必须 expired，awaiting_save 必须 discarded_on_abnormal_exit，裁决权与未确认效果均不得跨会话 | 基线 SYS / 完整列 WS·DE G2 |
| timeline | entry_id(PK)、workspace_id、rel_path、version_seq、label、created_at | 持久化文档级时间轴 | 基线 SYS / 完整列 WS G2 |

**控制态扩展表（基线，2026-06-12 实现前置批量登记；均含 workspace_id 外键——workspace 级隔离，
2026-06-11 人类决策；payload/定义载荷形状归各 owner 模块；列随实现期微调，迁移受
BR-SYS-PERSIST-001 约束）**

| 表 | 基线列 | 职责 | owner |
|---|---|---|---|
| agent_sessions | session_id(PK)、workspace_id、title、mode、archived、open_index、is_active、created_at、updated_at、last_active_at | 代理编辑会话标签元数据（重启恢复打开/归档顺序与激活态） | AG |
| agent_turns | turn_id(PK)、session_id、workspace_id、rel_path(可空)、status(completed/error/cancelled/gate_rejected/draft)、payload_ref、created_at、updated_at | 回合/对话历史（BR-AG-PERSIST-001：全状态本地可恢复不上云） | AG |
| agent_turn_payloads | payload_ref(PK)、workspace_id、payload_json、payload_size、created_at | 回合内容载荷（AG 私有 v1 JSON；工具摘要只读、Diff 仅引用） | AG |
| ed_open_tabs | workspace_id、rel_path、kind、open_index、is_active、updated_at | 编辑器打开文档标签元数据；重启恢复打开集合、顺序与当前文档，正文不在此表 | ED |
| ed_session_snapshots | workspace_id、rel_path、content、edit_format、content_format、dirty、session_version、content_hash、updated_at | 已打开文档 LogicalState 的持久化镜像；只对 `ed_open_tabs` 中仍打开的文档参与恢复和自动应用基线校验；旧记录迁移时 content_format 按 edit_format 安全回填 | ED |
| build_runs | run_id(PK)、workspace_id、status、plan_ref（任务计划载荷）、interrupted_at(可空)、created_at、updated_at | 构建控制态（BR-CBT-PERSIST-001：重启恢复至中断点或明确关闭，无悬挂） | CBT |
| workflow_runs | run_id(PK)、workspace_id、workflow_id、current_step、status、created_at、updated_at | 工作流执行控制态（BR-TPL-STATE-001：可恢复无悬挂） | TPL |
| templates | template_id(PK)、workspace_id、template_kind、body、organization、created_at、updated_at | 模板定义（BR-TPL-DATA-001） | TPL |
| workflows | workflow_id(PK)、workspace_id、definition_ref（步骤序列载荷）、created_at、updated_at | 工作流定义（BR-TPL-DATA-001） | TPL |
| memory_entries | entry_id(PK)、workspace_id、memory_kind、content、origin_turn、status(active/revoked)、created_at、revoked_at(可空) | 记忆条目（BR-AST-DATA-001/STATE-001） | AST |
| knowledge_entries | entry_id(PK)、workspace_id、content、organization、source_ref、created_at、updated_at | 知识条目（BR-KB-DATA-001） | KB |

**版本化迁移契约框架**

- `schema_version` 单调递增，是迁移的唯一驱动；每次结构变更必须配一次版本递增 + 迁移步骤。
- 迁移必须保持既有 `ed_open_tabs` / `ed_session_snapshots` / `pending_diffs` / `timeline` 持久化态不被破坏（向前迁移不丢数据）。
- 应用重启后，已持久化的控制态（含已打开文档 LogicalState 镜像、未裁决 pending_diffs、时间轴）必须可恢复（REQ-SYS-002 验收）。

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
变更配套版本递增与迁移步骤；迁移不得破坏既有 `ed_open_tabs` / `ed_session_snapshots` / `pending_diffs` / `timeline` 持久化态（不丢数据）；
应用重启后已持久化的控制态必须可恢复。本规则只约束 schema 版本化与迁移/恢复契约，不约束业务写入时机
（diff 落盘时机归 DE/WS）。

## §5 跨模块约束（CONSTRAINT）

<!-- CONSTRAINT
constraint_id: X-SHARED-001
摘要: 跨模块传递的实体/类型/枚举集合必须经 SYS 数据结构契约层（本文）定义，模块不得私自定义跨模块共享结构
涉及模块: SYS, WS, ED, DE, AG, CBT, DOC, PRV, REF, AST, KB, TPL, ACCT, ENT
链路影响: DATA-MIGRATE
-->

<!-- CONSTRAINT
constraint_id: X-SHARED-002
摘要: Agent 1.0 共享运行时对象（RuntimeProfile/AgentSession/AgentTurn/IntentResult/CapabilityGrant/TaskRun/TaskStep/StructuredVisibleEvent/ExternalContentRef/MemorySelectionRef）的职责、身份、事件真值、隔离与迁移边界以 SYS 数据结构契约层（A-SYS-I-P-01 §3.1）为唯一契约来源；任何模块不得以旧设计、现有代码、已注册工具、Prompt、前端参数、兼容默认值或测试绿灯反向改写冻结需求语义或隐式扩大授权
涉及模块: SYS, AG, DE, ED, WS, REF, AST
链路影响: AG-TURN, DE-DIFF, ED-EDIT, WS-SAVE, WS-DIFF-APPLY, REF-INJECT, AST-MEMORY, IPC-CALL
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
<!-- REQ_MAP
req_id: REQ-SYS-008
target_type: CONSTRAINT
target_id: X-FORBID-008
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-008
target_type: CONSTRAINT
target_id: X-FORBID-009
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-009
target_type: CONSTRAINT
target_id: X-SHARED-002
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-010
target_type: RULE
target_id: BR-SYS-STATE-007
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-010
target_type: CONSTRAINT
target_id: X-SHARED-002
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-011
target_type: RULE
target_id: BR-SYS-STATE-008
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-011
target_type: CONSTRAINT
target_id: X-SHARED-002
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-012
target_type: RULE
target_id: BR-SYS-DATA-007
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-012
target_type: CONSTRAINT
target_id: X-SHARED-002
status: mapped
-->

## §8 测试与验收

- **共享结构单一来源**（BR-SYS-DATA-002 / X-SHARED-001）：跨模块共享实体与枚举集合仅本文定义一处；
  代码阶段以 @GOV 引用 BR-SYS-DATA-002，audit 无重复定义。
- **持久化可恢复**（BR-SYS-PERSIST-001，REQ-SYS-002 验收）：构造含 ed_open_tabs、ed_session_snapshots、未裁决 pending_diffs 与时间轴的
  workspace.db，经一次 schema 版本递增迁移后重启，控制态完整可恢复、无数据丢失；孤立 ed_session_snapshots 不得恢复为打开标签。
- **枚举集合一致**：PendingDiff.status 枚举集合（pending/accepted/rejected/expired）与 DE 状态机
  （A-SYS-M-S-01）的状态集合对齐，无第三处定义；AgentTurnHistory.status 枚举集合
  （completed/error/cancelled/gate_rejected/draft）与 AG 历史恢复契约对齐。
- **共享运行时契约可追溯**（BR-SYS-STATE-007/008、BR-SYS-DATA-007、X-SHARED-002，REQ-SYS-009~012 验收）：
  唯一 profile 失败关闭、澄清非终态、首终态锁存与迟到事件无副作用、TaskRun 非后台队列、外部内容/记忆
  data-only 隔离的可追溯约束在本文 §3.1 单一来源；下游 @GOV 引用这些规则时不得私自定义或反向改写。
- 量化 NFR（迁移耗时、db 体量）延 G5 标定（APC §9）。

## §9 技术债务与盲区

- workspace.db **完整列、约束、索引**与 WS 持久化实现联合确定（SY-03，WS G2），本文为基线骨架。
- 不透明 ID 生成算法（ULID/UUID/自增等）属实现选型，延 L2 实现/G5，本文只约定"不透明标识符"语义。
- 具体序列化格式（payload_ref 指向的 diff 内容编码）归 DE；AgentTurnPayload 的 JSON 形状归 AG；本文只占位共享字段。
  AG 历史恢复不得把缺失的 diff 当前状态默认视为 pending：状态查询归 DE/WS，缺失只可投影为只读不可用。
- diff / timeline 的状态机与转移归 A-SYS-M-S-01 与 DE/WS M-S，本文只锁字段与枚举集合。
- `DiffProjection.patch_kind=style_ops` 在已打开非激活文档上的自动应用需要 ED 提供无 ProseMirror runtime 的
  StyleOp 事务；若实现阶段未具备该能力，必须让对应 DiffCard 失效并记录债务，不能降级为 DiskState 应用。

### 9.1 Agent 1.0 共享契约的漂移债务（D1 登记，不反向修改需求）

以下为现有 L2/代码与冻结基线（A-CORE-X-D-02）不一致的漂移点，本轮**只登记迁移债务**，不在 D1 内修改
实现或旧设计；具体迁移由对应 D2/D3 承接：

- **澄清终态漂移（归 D3/AG）**：现有 SM-AG-01（A-AG-M-S-01 §2）将 `clarificationRequired` 终结为
  `Completed`，与冻结基线「澄清是等待用户输入的非终态」不一致；AgentTurnStatus 五枚举
  （completed/error/cancelled/gate_rejected/draft）无澄清等待非终态值。D3 需调整回合状态机与状态枚举语义，
  不得把澄清伪装为完成。
- **AgentSession 隔离键漂移（归 D3/AG）**：现有 agent_sessions/agent_turns 持久化以 workspace_id 外键
  隔离为主；冻结基线要求 AgentSession 是展示、持久化与恢复的隔离键，workspace 只是外层范围。语义边界已
  在本文件 §3.1 锚定，持久化/查询迁移由 D3 承接。
- **意图/授权对象未统一收窄（归 D3/AG）**：现有 IntentResultV1/CapabilityGrantV1（A-AG-I-P-01）已存在，
  但「已注册工具、Prompt、前端参数、兼容默认值均不是授权来源」的语义需在 D3 收敛为 BR-SYS-STATE-007 的
  运行时校验，不能依赖 Prompt 文字。
- **TaskRun 无共享契约与状态机（归 D3/AG + D2/DE）**：TaskRun/步骤/Proposal 链的身份、事件与恢复待
  D3 定义状态机并向 SYS 总图登记；Proposal 链的接力/失效语义在 BR-CORE-STATE-001~003 已有约束，
  连续 Proposal 的编排细节归 D2/DE。
- **外部内容与记忆引用（归 D3/REF/AST）**：ReferencePayload.file 目前仅元数据（A-SYS-I-P-01 §3），
  R5-C 的一次性 data-only 内容读取（ExternalContentRef）与显式记忆选择（MemorySelectionRef）的暂存/清除、
  当前任务范围与来源令牌由 D3 定义，未验证前 P1 入口保持关闭。

## §10 后续动作

1. 本文落地后随下一次 `npm run governance:generate` 注册：DATA-MIGRATE 链路、BR-SYS-DATA-002 /
   BR-SYS-PERSIST-001 规则、X-SHARED-001 约束、REQ-SYS-002 的 REQ_MAP 进 ADUS；本轮新增
   BR-SYS-STATE-007/008、BR-SYS-DATA-007、X-SHARED-002 与 REQ-SYS-008~012 的 REQ_MAP 一并注册。
2. 继续 SYS G2：A-SYS-I-P-02（IPC / Tauri command 协议，承接 REQ-SYS-001，引用本文共享结构作为载荷）、
   A-SYS-M-S-01（状态机总图，引用本文 PendingDiff/TimelineEntry 字段契约）。
3. WS G2 启动时与本文联合敲定 workspace.db 完整 schema（解除 SY-03），并在 WS L2 注册
   workspace / timeline 相关 TERM。
4. **D2（DE/ED/WS）设计输入**：三态三分流状态机与接口契约（BR-CORE-STATE-001~003）、连续 Proposal
   接力/失效、显式保存与保存失败保留、关闭清算；输出 Proposal 链与保存/裁决契约。
5. **D3（AG/REF/AST）设计输入**：RuntimeProfile/AgentTurn/AgentSession/IntentResult/CapabilityGrant/
   TaskRun/TaskStep/StructuredVisibleEvent 的运行时契约与状态机（并向 SYS 总图登记 machine_id）；
   ExternalContentRef 暂存/清除与 MemorySelectionRef 显式选择；为 RuntimeProfile、CapabilityGrant、TaskRun、
   TaskStep、ExternalContentRef、MemorySelectionRef 等核心对象注册 TERM（主责模块 L2，ADU §3.2）。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-10 | v1.0 | 初版（SYS G2）：跨模块共享实体契约（WorkspaceRef/DocRef/PendingDiff/TimelineEntry，字段语义级）；workspace.db schema 基线与版本化迁移契约框架；规则 BR-SYS-DATA-002（共享结构单一来源）/ BR-SYS-PERSIST-001（schema 版本化迁移与重启可恢复）；链路 DATA-MIGRATE（P1）；约束 X-SHARED-001；REQ_MAP（REQ-SYS-002）；术语归属说明（不抢注 WS/DE 的 TERM）；测试验收与债务盲区 |
| 2026-07-20 | v1.1 | 承接 APP-CLOSE-DIFF-LIFECYCLE-01：PendingDiff 新增 `effect_status`（not_applicable/awaiting_save/saved/discarded_on_abnormal_exit）与 `terminal_reason`（新增 abnormal_exit 取值）两列，语义 SSOT 归 `BR-SYS-DATA-005`（A-SYS-M-T-02 §4）；明确 `status=accepted` 不等于内容已落盘，区分裁决态与落盘效果两个维度；workspace.db 基线表同步 |
| 2026-07-21 | v1.2 | 收紧跨会话边界：`awaiting_save` 仅为当前进程会话态，启动清算必须转 `discarded_on_abnormal_exit`，不得依赖 dirty 快照残留；`pending_diffs` 重启只恢复终态只读审阅。 |
| 2026-06-11 | v1.1 | 随 REF G2 登记（X-SHARED-001 回写）：§3 新增 RefInjection 引用注入载荷共享结构基线（target/block_ids/content/origin_label，主责 REF，REF→AG/CBT 跨模块传递）；解析/校验/组装语义归 A-REF-M-T-01，裁剪归 AG；受 BR-SYS-DATA-002 单一来源约束覆盖，无新增规则 |
| 2026-06-12 | v1.2 | 进入实现前批量登记（X-SHARED-001/BR-SYS-PERSIST-001 回写，解除 AG/CBT/TPL/AST/KB 五模块"实现前登记"承诺）：§3 新增 MemoryInjection（memory_kind/content/origin_turn，主责 AST）与 KnowledgeInjection（content/source_ref/organization，主责 KB）注入共享结构基线；§4 新增七张 workspace.db 控制态扩展表基线——agent_turns（AG 回合历史）/build_runs（CBT 构建控制态）/workflow_runs（TPL 执行控制态）/templates·workflows（TPL 定义）/memory_entries（AST）/knowledge_entries（KB），均 workspace 级隔离（2026-06-11 人类决策）、payload/定义形状归各 owner、迁移受 BR-SYS-PERSIST-001 约束；无新增规则（受 BR-SYS-DATA-002 覆盖） |
| 2026-07-03 | v1.3 | 随 ED 批B PI-ED-10 登记（X-SHARED-001 回写，承 A-REF-M-T-01 v1.1 扩展意向）：§3 RefInjection 增可空 block_ranges 选区级锚集合（{blockId,start,end} 块内纯文本偏移区间；CharAnchor 契约归 ED BR-ED-DATA-004、有效期=编辑会话内、失效校验归 REF §5 保守语义）；无新增规则（受 BR-SYS-DATA-002 覆盖） |
| 2026-07-08 | v1.4 | 随引用令牌修复登记 ReferencePayload 共享结构基线：新增 doc/selection/session/memory/knowledge/template/url/long_text 八类输入侧显式引用字段集合；声明其为 Composer→AG 主通道，与 RefInjection 分层不同；无新增规则（受 BR-SYS-DATA-002 覆盖） |
| 2026-07-28 | v1.5 | 登记 BDocxCapabilityManifest 跨模块字段基线，完整版本化能力协议、TERM-SYS-002 与 BR-SYS-DATA-006 委派 A-SYS-I-P-04。 |
| 2026-07-17 | v1.5 | 随 AG 历史持久化具体化登记 AgentSession / AgentTurnHistory / AgentTurnPayload 基线；`agent_sessions`、增强 `agent_turns`、`agent_turn_payloads` 入 workspace.db 控制态扩展表；锁定 AgentTurnHistory.status 枚举 completed/error/cancelled/gate_rejected/draft 与 Diff 仅引用不快照边界 |
| 2026-07-18 | v1.6 | 登记 OpenDocSnapshot、DiffProjection 基线与 `ed_session_snapshots` 表：已打开文档 AG/DE/ED 链路必须以 LogicalState 快照为唯一读取门禁，未打开文档才允许读取 DiskState |
| 2026-07-18 | v1.7 | 登记 HistTurn 历史消息投影字段：`rel_path/turn_id` 用于 AG 按当前文档分流历史，防跨文档历史作为当前文档事实 |
| 2026-07-18 | v1.8 | 登记 EditorOpenTab 与 `ed_open_tabs` 表：打开集合、顺序和当前文档由独立控制态恢复；`ed_session_snapshots` 只作为打开文档 LogicalState 内容镜像，孤立镜像不得打开文档 |
| 2026-07-18 | v1.9 | ED-MARKDOWN-RICH-ROUNDTRIP-01：OpenDocSnapshot/edit session 增 content_format，edit_format 墄 markdown；`ed_session_snapshots` 墠 content_format 迁移列，使 Markdown DiskState 格式与 LogicalState HTML 编码不再混淆 |
| 2026-07-28 | v1.10 | BDOCX-IMAGE-MEDIA-FOUNDATION-01：OpenDocSnapshot 增可空 baseline；BDocx 持久镜像以存储层内部 envelope 兼容现有表结构，列表 IPC 返回前必须解码，保证 LogicalState 不含缓存协议元数据。 |
| 2026-08-07 | v1.12 | 随 REF-REFERENCE-REPAIR-01 F5 登记（X-SHARED-001 回写）：§3 ReferencePayload 基线增 file 类型（name/path?/size?，外部文件仅元数据不读内容）；无新增规则（受 BR-SYS-DATA-002 覆盖） |
| 2026-07-30 | v1.11 | AG-RICH-BLOCK-STRUCTURED-EDIT-01：OpenDocSnapshot 增同次 ProseMirror `block_snapshots`；登记 AgentEditableBlockSnapshot 与 StructuredBlockEditPayload，锁定层级结构不展平、动态写能力、基线/指纹/节点 Schema 批原子应用及冲突安全的精确反向恢复。 |
| 2026-08-13 | v1.13 | DIFF-STATE-COMPASS-01：PendingDiff 共享契约同步 BR-CORE-STATE-001~003——已有 LogicalState 预览但卡片仍 pending、无 LogicalState 仅挂载后接受原子落盘、pending 期间打开文档迁移为前者；移除“open_session 自动 accepted”的旧语义。 |
| 2026-08-14 | v1.14 | AG-1.0-D1-SYS-SHARED-CONTRACT-01：新增 §3.1 Agent 1.0 共享运行时对象契约（RuntimeProfile/AgentSession/AgentTurn/IntentResult/CapabilityGrant/TaskRun/TaskStep/StructuredVisibleEvent/ExternalContentRef/MemorySelectionRef 的职责、身份、事件真值、隔离与迁移边界，不锁实现细节）；注册 BR-SYS-STATE-007/008、BR-SYS-DATA-007 与 X-SHARED-002；REQ_MAP 补齐 REQ-SYS-008~012；§9.1 登记冻结基线漂移债务（澄清终态、AgentSession 隔离键、意图/授权收窄、TaskRun 契约、外部内容/记忆引用）；§10 明确 D2/D3 设计输入。 |
