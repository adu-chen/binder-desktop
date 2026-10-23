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
| proposal_batch_id / group_index / group_count | 同次模型提案批次与组内顺序 | 仅表达关联；每个 diff_id 独立裁决 |
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

**ProposalBatch · 提案批次（跨模块关联结构，主责 DE）**

| 字段 | 语义 | 备注 |
|---|---|---|
| proposal_batch_id | 同次已校验 ChangeProposal 的批次身份 | 不透明标识符 |
| turn_id / tool_call_id | 来源 AgentTurn 与模型工具请求 | 可追溯，不构成授权 |
| diff_ids | 按 group_index 排列的 PendingDiff 身份 | 一个或多个；各自进入 SM-DE-01 |
| grouping_version | 分组算法/契约版本 | 恢复与诊断使用 |

> ProposalBatch 不具有统一接受/拒绝终态；批量 UI 操作必须展开为对各 PendingDiff 的独立裁决。位置连续
> 只是可合并的必要条件，不是充分条件；不连续或可独立裁决的块组必须拆分。同一文档同批结构化卡的
> DiffProjection 必须共享相同生成基线、持有互不重叠且可独立反向的目标集合，并由 ED 聚合成一次前向
> 事务；共享预览推进的整篇版本不能使兄弟卡互相失效。

**WorkspaceTimelineEntry · 统一工作区时间轴条目（基线）**（REQ-CORE-007/014，主责 WS）

| 字段 | 语义 | 备注 |
|---|---|---|
| entry_id | 条目不透明标识 | 标识符 |
| workspace_id | 所属 workspace | 隔离键 |
| entry_kind | 条目类型 | document_snapshot / workspace_operation |
| target_refs | 目标引用集合 | 文档内容条目通常为单 DocRef；资源条目可为冻结清单 |
| version_seq | 工作区时间轴序 | 单调递增；文档视图可再按 DocRef 投影 |
| label | 版本或操作标注 | 可空 |
| payload_ref | 类型化载荷引用 | document_snapshot 或 WorkspaceOperationEntry；不得用 label 反推恢复数据 |
| created_at | 创建时间 | — |

**WorkspaceOperationPlan · 工作区资源操作计划**（REQ-CORE-010/012/014，主责 WS）

| 字段 | 语义 | 备注 |
|---|---|---|
| plan_id / workspace_id | 不可变计划身份与隔离键 | 计划落库后 items/基线不得就地修改 |
| operation_items | 有序 create/delete/rename/move/restore 项 | 每项含来源、目标、资源类型和顺序 |
| frozen_manifest | 递归清单、资源身份、内容/元数据快照摘要 | 决策卡与执行前重验共同真值 |
| affected_sessions / affected_diffs / affected_references | 标签、内容 Diff 与引用影响清单 | dirty 会话阻断；有效内容 Diff 在成功后失效 |
| baseline_set | 磁盘、路径、资源、LogicalState 等执行基线 | 任何推进均过期或阻断 |
| compensation_policy | fail_fast_compensate | 完成项逆序补偿；不承诺严格文件系统事务 |
| idempotency_key | 重复确认/执行幂等键 | 同键返回同一终态，不重复变更磁盘 |
| decision_request_id | 前置裁决关联 | 只能绑定真实 DecisionRequest |
| status | 计划/执行状态 | planning/waiting_confirm/revalidating/executing/compensating/completed/rejected/expired/failed_compensated/partial_failed/interrupted |

**WorkspaceOperationEntry · 资源操作历史与恢复载荷**

| 字段 | 语义 | 备注 |
|---|---|---|
| operation_id / plan_id | 执行身份与来源计划 | 一次确认只产生一次执行身份 |
| before_manifest / after_manifest | 执行前后路径、目录清单、内容/元数据快照 | 类型化恢复真值；删除必须可取回实际字节/目录结构 |
| item_receipts | 每项执行结果 | 含完成顺序、失败与错误 |
| compensation_receipts | 每项补偿结果 | 补偿不完整决定 partial_failed |
| restore_kind / restore_payload_ref | 类型化恢复器与载荷 | restore 不直接执行，而生成新的 WorkspaceOperationPlan |
| final_status | completed/failed_compensated/partial_failed | 不得用通用 success 布尔压缩 |

**CreateDocumentBundle · 新建文档复合对象**

| 字段 | 语义 | 备注 |
|---|---|---|
| plan_id / operation_id | 资源创建计划与执行身份 | operation_id 仅确认执行后存在 |
| target | 新文档 DocRef | 确认后成为真实 DiskState 节点 |
| initial_content_ref | 确认前生成并冻结的初始内容 | 资源创建失败不得投影 |
| initial_diff_id | 资源创建后生成的内容 Diff | 使用普通 PendingDiff 生命周期 |
| rejection_guard | 资源身份、空文件 DiskState、路径与 LogicalState 基线 | 全部未推进才允许补偿删除 |

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
> OpenDocSnapshot 时，AG 才允许读取 DiskState 快照。v2 通用准入可只携打开资源身份；此时快照集合不完整
> 不改变 opened 身份，不得以“未命中快照”为由降级为 DiskState。实际内容消费仍必须取得有效 LogicalState。
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
| location_label | 卡片位置说明 | 仅用于可见投影，不参与锚校验 |
| draft_mode | 应用路径 | current_editor / open_session / background |
| snapshot_source | 生成基线来源 | active_editor / active_editor_session / open_editor_session / disk_snapshot |
| base_session_version | 生成时 ED session 版本 | 已打开文档必填；同一文档同批结构化卡必须相同；未打开文档为空 |
| base_content_hash | 生成时内容哈希 | 已打开文档必填；同一文档同批结构化卡必须相同；未打开文档可使用 DiskState baseline |
| style_ops / block_ops / table_ops / image_ops | 可空的类型化操作载荷 | 与 patch_kind 对应的字段必填；执行与详细 Diff 的唯一真值 |
| proposal_batch_id / group_index / group_count | ProposalBatch 身份与卡片稳定顺序 | 新载荷必填；每卡独立裁决，三字段不得藏入自由形状 JSON |
| preview_checkpoint_required | 是否要求编辑器预览回执 | true 时必须完成预览检查点后才能续片段 |
| cross_doc | 是否跨文档提案 | 可空兼容旧载荷；新载荷应显式给出 |

> 边界：DiffProjection 是跨 AG/DE/ED 的最小互操作载荷。`draft_mode=open_session` 时，
> ED 必须以 `base_session_version` 与 `base_content_hash` 校验当前 LogicalState；校验失败不得应用，
> DiffCard 进入不可裁决的失效/冲突状态。`draft_mode=background` 只允许目标文档未打开。
> 对同一文档同批结构化多卡，这两个字段只校验一次聚合前向事务；事务成功后，逐卡接受/拒绝改用本卡
> 类型化载荷中的目标 after fingerprint。卡片不得复制整篇 after 充当局部裁决真值。
> 旧 DiffProjection 的 `batch:{id,index,total}` 仅允许迁移读取并转换为上述三字段，不得继续生产或作为
> ProposalBatch 的第二种活动 wire 形状。

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
> （A-CORE-X-D-02 §3/§4）。本节锚定各对象的**职责、身份关系、事件真值、隔离与迁移边界**；共享字段
> 基线与版本在 BR-SYS-DATA-008/§4 锁定，owner 私有载荷、序列化细节和状态机内部仍归各模块 L2。
> 正式术语由主责模块按 ADU §3.2 注册；本文以跨模块数据结构契约身份引用，不重复注册。

| 共享对象 | 职责 | 身份关系 | 事件真值 | 隔离与迁移边界 |
|---|---|---|---|---|
| `RuntimeProfile`（运行时档位） | 声明一次 Agent 交互可用的运行时能力档位；1.0 唯一 `agent-edit-1.0` | profile_id 作为回合/意图/授权的最顶层收窄约束 | 缺失或未知 profile 必须在 Gate、provider、工具与 Proposal **之前**失败关闭，不产生任何可执行事件 | 已注册工具、Prompt、前端参数或兼容默认值都不构成 profile 的授权来源；Build/CBT 等 P2 不进入该 profile |
| `AgentSession`（代理会话） | 用户可见的代理编辑会话；是**展示、持久化与恢复的隔离键** | session_id 属于唯一 workspace；一个 workspace 可有多会话 | 会话内消息、工具卡、DiffCard、引用、未保存上下文与回合按事件顺序呈现，跨会话不串线 | workspace 只是外层范围，不能替代 AgentSession 作为恢复隔离键；会话归档/删除后的回合只读或不可恢复须明确 |
| `AgentTurn`（代理回合） | 一个用户发送对应一个回合；收敛当前目标、意图与一次交互结果 | turn_id 属于唯一 AgentSession；可关联 DocRef、TaskRun、Proposal 与结构化事件 | **澄清是等待用户输入的非终态**；取消可在任一活动阶段生效；完成/失败/拒绝/取消互斥且**首个终态获胜**；首终态后到达的迟到事件或工具不得产生文档、卡片或落盘副作用 | 回合上下文（引用快照、外部内容、显式记忆选择）只在回合内有效；重启不伪造成功；新回合重新校验上下文 |
| `IntentResult`（本轮意图结果） | 模型驱动且可修订的目标语义投影：目标、候选、预期副作用、所需上下文、歧义与确认点 | 与 turn_id/source_event_seq/revision 绑定；首轮可 unresolved | 只记录可见语义，不含 allowed_tools，不是授权或调度边界 | 不展示隐藏推理；后续 ToolResult 可使其修订，历史不得覆盖当前明确请求 |
| `CapabilityGrant`（当轮能力授权） | 下一次具体工具执行的短生命周期授权；由 profile、注册表、当前范围/目标、允许副作用路径、格式/动态能力、裁决、Gate、预算、取消与终态收窄 | 与 turn_id/grant_id/computed_at_seq 绑定；工具执行前重新计算 | IntentResult、TaskPlan、模型文字和注册表存在性均不授权；越权失败关闭 | 动作完成或事实变化即失效，不跨工具/回合继承 |
| `TaskRun`（受限任务实例） | 复杂目标的可观察投影，关联 AgentTurn、真实工作、等待点和当前候选路线；**不是调度器或后台队列** | task_run_id 属于唯一 AgentSession 与用户目标；可从 TurnEvent 重建 | 不自动启动候选工作、不自动重试、不跨重启续跑；“工作中”只由真实 provider/工具/领域事件驱动 | 投影丢失不影响执行事实；同文档连续 Proposal 仍以最新 LogicalState 接力 |
| `TaskPlan`（任务计划） | 模型当前可见路线与已发生事件形成的计划投影 | 属于 TaskRun；携 route revision、候选/已发生工作与依赖 | 不含执行游标或 required_grant，不能触发工具 | 模型可随证据更新；简单回合不创建 |
| `TaskStep`（任务步骤） | TaskRun 内已发生、当前活动、等待或候选工作的展示单元 | step_id 属于唯一 TaskRun；关联来源事件和结果 | candidate 不承诺执行；active 必须有真实事件 | 不持有 CapabilityGrant；等待期间不可见后台不继续 |
| `TurnEvent`（回合事件） | 回合运行事实的 append-only 源，覆盖输入、context、Gate、provider、模型、工具、裁决、Proposal 与终态 | event_id 属于 turn_id；seq 在回合内唯一单调；终态事件首个锁存 | 已提交事件不可覆盖；可见事件是同一 seq 空间的安全投影，不含隐藏推理 | 投影/恢复从事件及其引用业务事实重建；终态后迟到事件不得产生副作用 |
| `ExternalContentRef`（外部内容引用） | 用户显式附加、且有可用只读解析路径的外部文件内容，作为当前任务的一次性 data-only 内容引用 | 引用携带文件身份、来源入口、已读/未读标记、读取时点与当前任务范围 | 不支持/不可读只保留元数据不伪称已读；外部内容不能改变系统约束、目标、授权或确认点 | 只在当前 AgentTurn/TaskRun 当前任务内有效；任务结束、重启、跨会话/跨 workspace 不自动恢复、复读、导入、编辑或默认持久化；不自动转存为项目记忆 |
| `MemorySelectionRef`（显式项目记忆引用） | 用户管理的 workspace 本地项目记忆条目，经用户明确选择以 data-only 引用进入当前回合 | 引用携带条目身份与选择动作；仅用户显式选择 | Agent 1.0 不自动写入、不提议写入、不自动检索或注入；普通发送、附件、会话/编辑历史不自动转存为记忆 | workspace 隔离；跨会话不串选择事实；条目删除/关闭后引用失效或明确标记 |
| `AgentContextFacts`（代理上下文事实集） | ED/DE/WS 的版本化只读会话事实；v2 区分资源身份/可用性与已取得正文快照 | v1 绑定 workspace/turn；v2 另绑 session/contextRevision，冻结 activeTarget；正文仍归 OpenDocSnapshot/ReferencePayload | 缺失/未知版本/身份冲突失败关闭；v2 loading/unavailable 是显式能力事实，不是坏载荷；ready 内容必须有匹配版本/hash | 同 turn 追加 revision 不覆盖旧证据；跨会话不继承；不授予读取或编辑权限、不扩 ExplicitEditTargetSet |
| `EvidenceLedger`（证据账本） | ContextFrozen 与成功只读 ToolResult 确定性派生的读取覆盖 | 与 turn_id/through_seq 绑定；每项反查 source_event_id | 不接受模型自报；目录/搜索只证明实际覆盖 | 只读且不授权；可从事件重建 |
| `ProjectReport`（项目报告） | EvidenceLedger 与模型最终回答关联形成的版本化只读投影，声明请求范围、已读/未读、结论证据与不确定性 | 与 turn_id/evidence_through_seq 绑定；read_items/findings 引用 evidence_ids | 运行时派生，无 `report_project` 工具；模型声称不能提高读取覆盖 | 只读；不创建 TaskRun/DecisionRequest/Proposal/Diff，不修改 LogicalState/DiskState |

<!-- RULE
rule_id: BR-SYS-STATE-007
主链路: AG-TURN, IPC-CALL
域: STATE
-->
Agent 1.0 运行时共享对象（RuntimeProfile / AgentSession / AgentTurn / IntentResult / CapabilityGrant /
TurnEvent）的事件真值与隔离以本文 §3.1 契约为唯一来源：仅 `agent-edit-1.0` 可启动，缺失或
未知 profile 必须在 Gate、provider、工具与 Proposal 前失败关闭；一个用户发送对应一个 AgentTurn，澄清是
等待用户输入的非终态，取消可在任一活动阶段生效，完成/失败/拒绝/取消互斥且首个终态获胜，首终态后的迟到
事件或工具不得产生文档、卡片或落盘副作用；TurnEvent 必须按 turn 内唯一单调 seq 追加且不可覆盖，
可见事件是同一序列的安全投影；IntentResult 可随模型/工具证据修订但不授权。已注册工具、Prompt、
TaskPlan、前端参数或兼容默认值不构成当轮授权；可见事件不包含模型内部推理。

<!-- RULE
rule_id: BR-SYS-STATE-008
主链路: AG-TURN, DE-DIFF
域: STATE
-->
TaskRun/TaskPlan/TaskStep 是复杂目标的 TurnEvent 派生投影，不是调度器或后台队列：不得持有
CapabilityGrant、执行游标或“执行下一步”命令，不自动启动候选工作、不自动重试、不跨重启续跑、不越过
确认点；“工作中”只能由真实 provider、工具或领域事件驱动，等待补充、卡片 pending、失败、取消或中断
不得伪装为执行。简单 AgentTurn 不创建空 TaskRun/TaskPlan。
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

<!-- RULE
rule_id: BR-SYS-DATA-008
主链路: AG-TURN, AG-TOOL, ED-EDIT, DE-DIFF
域: DATA
-->
AgentContextFacts、TurnEvent、EvidenceLedger 与 ProjectReport 的跨模块字段和枚举以本文 §3.1 为单一共享契约来源。AgentContextFacts
版本 `agent-context-facts-v1` 固定包含 workspaceId、turnId、activeDoc、openDocs、selections、proposalFacts：
文档事实携 relPath/source/editFormat/contentFormat/dirty/baseline/sessionVersion/contentHash；SelectionFact
携 relPath/blockId/start/end；ProposalFact 携 diffId/sessionId/relPath/state/effectStatus/saved。TurnEvent 版本
`turn-event-v1` 固定包含 eventId/turnId/seq/occurredAt/kind/payloadVersion/payload；EvidenceLedger 版本
`evidence-ledger-v1` 固定包含 turnId/throughSeq/evidenceItems，每项携 evidenceId/relPath/source/coverage/
versionOrHash/sourceEventId。ProjectReport 版本 `project-report-v2` 固定包含 turnId/evidenceThroughSeq/
requestedScope/readItems/unreadItems/findings/uncertainties，ReadItem/Finding 只能引用 evidenceIds。字段允许 additive 扩展，
版本未知必须失败关闭；任何模块不得复制一套近义结构或把正文、显示态、磁盘旧快照混入这些事实字段。

AgentContextFacts 只能以当前 active workspace 的 ED/DE/WS 真值生成并与 AgentTurn 身份绑定；v1 保留既有完整快照
语义，v2 可在同一 turn 内追加不可变 contextRevision（从 1 单调递增），不可覆盖旧 ContextFrozen 或伪造旧证据；
ProposalFact 必须与当前 AgentSession 身份一致，不得投影其他会话或无归属 Proposal。
EvidenceLedger 只能从同一回合的 ContextFrozen 或成功只读 ToolResult 事件派生；ProjectReport 由运行时关联
EvidenceLedger 与模型最终回答生成；模型可提交 finding→evidenceIds 关联，但运行时只接受当前回合账本中
截至 evidenceThroughSeq 且覆盖等级足够的身份，未知、跨回合或覆盖不足的引用只能形成 uncertainty。
目录/搜索的覆盖不得升级为全文，模型自报不得写账。上述对象均为
data-only/只读投影，不构成 CapabilityGrant、ExplicitEditTargetSet、TaskRun 或写入授权来源。

### §3.1.0 ToolCall 执行回执（P10 加法契约）

`ToolCallReceiptV1` 是 ToolResult 的不可变技术 DTO，不是新的业务状态机或授权对象。版本为
`tool-call-receipt-v1`；绑定 workspaceId/sessionId/turnId/toolCallId/toolName/capabilityId/grantId/attempt。
本阶段每个具体 ToolCall 只执行一次，attempt=1；不由模型、历史或 UI 生成或修订回执。

| 字段 | 枚举 / 语义 |
|---|---|
| executionStatus | succeeded / rejected / failed；分别表示执行器成功返回、执行前拒绝、执行器失败 |
| completionStatus | completed / waiting / blocked；只说明当前能力请求的处理阶段，不表示整个用户目标成功 |
| effectStatus | not_applicable / not_started / proposal_created / resource_prepared / resource_executed / unknown；resource_executed 仅凭真实 Completed WorkspaceOperationReceipt，携 workspace_operation_plan 引用，不表示文档修改已接受或保存 |
| references | kind + id，kind 为 decision_request / pending_diff / workspace_operation_plan；引用实际后端产物，不授权、不把引用视为已接受 |
| failure | 可空 code/retriable/reauthRequired，保留实际 IpcError 属性；公开回执不携原始错误正文、文档正文或凭据 |
| evidenceIds | 本次成功只读执行产生的证据引用；失败不伪造成功读取证据 |
| nextAction | none / await_user / respond_blocked / retry_explicitly / inspect_effect；可重试不构成自动重放许可 |

成功读取为 succeeded/completed/not_applicable；成功阻断报告为 succeeded/blocked/not_applicable；
创建 DecisionRequest 为 succeeded/waiting/not_started；生成 PendingDiff 为 succeeded/waiting/proposal_created；
受控资源准备为 succeeded/completed/resource_prepared，不代表文档已保存。执行前拒绝为 rejected/blocked/not_started。
有副作用的执行器失败时效果 unknown，下一步 inspect_effect，禁止自动重放；只读失败不具备写效果。
上述效果是本次调用观察值，不覆盖 PendingDiff.status/effect_status、编辑器预览、裁决或 DiskState。

ToolResult 加法字段 `executionReceipt` 可空：旧事件无此字段按原契约读取，不补造回执；新版事件与
模型 ToolResult 回注携带同一回执和 evidenceIds，模型原始工具结果只进入回注，不进入安全事件。
消费者须核对版本、枚举和所属回合/工具身份；非法或不匹配回执不得冒充成功，不扩权限、不改变真实业务对象。
共享类型在 types 定义，AG 构造，IPC 安全投影，UI 只读消费；不引入表或迁移。

执行端口补充（P10/P12，2026-10-07）：`CapabilityExecutionFactV1` 为独立效果事实投影，
含 identity（与 ToolCallReceiptIdentityV1 相同）、phase=started/settled、receipt（started 为 null，
settled 为同一安全回执）。启动与结算各追加一次，存既有 agent_turn_payloads 的保留 `ag-effect:`
命名空间，不修改回合展示缓存、不新增表/迁移，不向模型或 TurnEvent 终态后追加结果。
启动记录是已保留执行身份，不等于业务成功；结算失败或缺失时不得重放。历史 `ag.list_turns`
回合 DTO 加法字段 execution_facts 返回同 workspace/session/turn 的效果事实；旧服务无字段视为空，
不能视为已经核对成功。结算仅引用实际 DecisionRequest/PendingDiff，不存正文，不表示接受/保存。
运行核心的取消/30 秒等待期限与能力结算独立：已开始的同步操作不能强杀；原回合终态保持，迟到结果
只能更新该调用的效果事实。未结算身份保留，同会话不能接管；同 workspace 的新写请求在效果未知时拒绝。
重启只恢复事实，不恢复执行权限；未结算或 unknown 不自动转为无效果，不提供跨进程重放或通用回滚。
删除会话必须拒绝抹掉未结算/unknown 的效果事实；已结算且非 unknown 的事实可随本会话历史删除。

### §3.1.1 AgentContextFacts v2：按需专业上下文（P03 接入，验收另记）

P01 失败诊断加法契约（BR-SYS-DATA-008 / BR-SYS-STATE-007）：安全 Error、失败 ToolResult 和
AgentTurnPayload.failure 可携可选 `diagnostic`，旧数据缺字段合法。版本 `1`，字段
`observedAt`（context_assembly/runtime/gate/finalizing/tool_validation/tool_execution/frontend_start/frontend_invoke/event_recovery）、
`category`（core_context/model_protocol/authorization/professional_capability/ui_projection/unknown）、
`rootCauseStatus` 固定 `unknown`。分类只证明观察边界/明确协议错误，不宣称根因已查明；未知错误不按文案或前缀猜测。
身份复用外层 AgentTurn/eventId/seq、数据库 workspace/session 与 ToolCallId，不复制第二套身份。
诊断不得携正文/工具参数/密钥，不进入模型消息，不改变终态或 retriable，不表明专业修改是否生效。
预算错误分类 core_context；模型控制缺失/非法分类 model_protocol；其他错误按可信调用边界或 unknown。
具体预算分项属 P04，完整模型传输入口属 P02，本字段不替代两者。
此诊断契约独立于 AgentContextFacts v2，当前 v1 生产路径也可携带；不表示 v2 已接入。

本节承接 BR-SYS-DATA-008 / BR-AG-DATA-008；不变更 `agent-context-facts-v1` 的字段解释。
新写入必须在前后端同时支持后显式选择 `agent-context-facts-v2`；未知版本失败关闭，旧事件只读恢复仍按
原 payloadVersion 解析，不把缺失快照推断为 v2 的合法 unavailable，也不得悄悄丢弃 v2 字段降级为 v1。

| v2 字段（camelCase） | 必填性及语义 |
|---|---|
| version | 固定 `agent-context-facts-v2` |
| workspaceId / sessionId / turnId | 必填且与请求、宿主身份一致 |
| contextRevision | 必填正整数；同 turn 递增，绑定本次 ContextFrozen 与 PromptPackage |
| activeTarget | 必填可空；非空为 `{relPath, loadSeq}`，冻结提交时 ED 活动路径及绑定代际；切换标签不改变原目标 |
| openResources | 必填数组；记录准入时完整打开资源身份，不要求每项已采集正文。条目 `{relPath, availability, reasonCode, snapshot}` |
| availability | `not_requested / loading / ready / unavailable`；必须来自宿主/专业适配器，不由模型填写 |
| reasonCode | loading/unavailable 必填稳定原因；not_requested/ready 可空，不携正文 |
| snapshot | ready 必填既有文档事实 `{source,editFormat,contentFormat,dirty,baseline,sessionVersion,contentHash}`；其他状态必须为空，不伪造 0 版本或空 hash |
| selections | 用户显式 SelectionFact；每项关联 `relPath/blockId/start/end`、`validation=unverified/valid/stale`、`contextRevision`；valid 必须有同版本快照，其他状态不能成为编辑锚 |
| proposalFacts | 沿用 v1 专业对象事实字段，必须属于本 session；不携正文，不改变 DE 状态 |

`OpenDocSnapshot` 正文与结构仍是单独对象；ready 的元数据必须与实际快照一致。availability 仅描述
采集结果，不代表所有格式操作可执行；结构化编辑仍须专业格式与 Block 守卫。活动目标存在时必须出现在
openResources；条目路径不得重复或越界。不存在 activeTarget 时允许合法空数组；未就绪与身份未知不能混用。

准入冻结身份，执行前重新核验当前打开/关闭状态和资源版本；变化需记录新 revision 或明确 stale。
冻结时打开但其后关闭的目标不得自动降为磁盘读取：要求重新取得合法资源事实或让用户重新提交。
新发现的打开目标必须由可信宿主补充资源身份后再读取，不得从不完整快照集合推断磁盘回退。

必需内容采集失败时只阻断依赖该内容的动作，输出 unavailable/stale 与下一步；无关资源未就绪不阻断纯对话。
ready 事实不能继承旧 turn；ContextFrozen 中的资源身份/可用性只证明元数据，不生成正文覆盖证据。
专业回执的具体共享 wire DTO 由后续 P07/P10 在本文对应小节收敛；P00 仅冻结
身份/版本/一次消费/效果分层要求，不以通用 success 替代既有 DE/WS 裁决或保存回执。

> 边界：本文只给统一时间轴与资源操作对象的跨模块共享**字段基线**；计划/执行/补偿/恢复转移语义归
> A-WS-M-T-02 / A-WS-M-S-02。文档内容 Diff 三态仍由 DE/ED/WS 既有状态机负责。

<!-- RULE
rule_id: BR-SYS-DATA-002
主链路: DATA-MIGRATE
域: DATA
-->
跨模块传递的共享实体（WorkspaceRef / DocRef / PendingDiff / WorkspaceTimelineEntry / WorkspaceOperationPlan /
WorkspaceOperationEntry / CreateDocumentBundle / ReferencePayload / OpenDocSnapshot / DiffProjection 等）及其枚举值集合
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
| proposal_batches | proposal_batch_id(PK)、workspace_id、turn_id、tool_call_id、grouping_version、created_at | 持久化同次 ChangeProposal 的来源与分组版本；无统一裁决终态 | DE / 基线 SYS |
| pending_diffs | diff_id(PK)、workspace_id、proposal_batch_id、group_index、group_count、rel_path、origin_module、status、created_at、payload_ref、effect_status（2026-07-20 新增）、terminal_reason（2026-07-20 新增，可空） | 持久化逐卡独立裁决状态；重启仅可恢复只读终态审阅：pending 必须 expired，awaiting_save 必须 discarded_on_abnormal_exit，裁决权与未确认效果均不得跨会话 | 基线 SYS / 完整列 WS·DE G2 |
| timeline | entry_id(PK)、workspace_id、entry_kind、target_refs、version_seq、label、payload_ref、created_at | 持久化统一 WorkspaceTimeline | 基线 SYS / 完整列 WS G2 |
| workspace_operation_plans | plan_id(PK)、workspace_id、plan_json、decision_request_id、idempotency_key、status、created_at、updated_at | 持久化不可变计划与状态；重启未终态统一 Interrupted | WS |
| workspace_operation_entries | operation_id(PK)、plan_id、workspace_id、before_ref、after_ref、receipts_ref、restore_kind、final_status、created_at | 类型化资源操作历史与恢复入口 | WS |
| create_document_bundles | plan_id(PK)、workspace_id、target_rel_path、initial_content_ref、operation_id、initial_diff_id、guard_ref、status | 关联资源创建、初始内容 Diff 与安全拒绝 guard | WS/DE/ED |

**控制态扩展表（基线，2026-06-12 实现前置批量登记；均含 workspace_id 外键——workspace 级隔离，
2026-06-11 人类决策；payload/定义载荷形状归各 owner 模块；列随实现期微调，迁移受
BR-SYS-PERSIST-001 约束）**

| 表 | 基线列 | 职责 | owner |
|---|---|---|---|
| agent_sessions | session_id(PK)、workspace_id、title、mode、archived、open_index、is_active、created_at、updated_at、last_active_at | 代理编辑会话标签元数据（重启恢复打开/归档顺序与激活态） | AG |
| agent_turns | turn_id(PK)、session_id、workspace_id、rel_path(可空)、status(completed/error/cancelled/gate_rejected/draft)、payload_ref、created_at、updated_at | 回合/对话历史（BR-AG-PERSIST-001：全状态本地可恢复不上云） | AG |
| agent_turn_events | event_id(PK)、workspace_id、session_id、turn_id、seq、kind、payload_version、payload_json、occurred_at | append-only 回合事实；(turn_id,seq) 唯一，禁止更新/删除单项事件；删除整个 AgentSession 时随历史一起删除 | AG |
| agent_turn_payloads | payload_ref(PK)、workspace_id、payload_json、payload_size、created_at | 可重建的回合展示物化缓存（AG 私有 v2 JSON；lastEventSeq、工具摘要只读、Diff 仅引用），不得覆盖事件/状态事实 | AG |
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
- 应用重启后，已持久化的控制态必须按真实语义恢复：文档会话/时间轴可恢复；WorkspaceOperation 未终态统一 Interrupted，绝不自动执行；partial_failed 保留逐项收据供诊断与人工恢复。

<!-- CHAIN
chain_id: DATA-MIGRATE
主责模块: SYS
status: active
risk_tier: P1
integration_test: deferred: phase-5
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
摘要: Agent 1.0 共享运行时对象（RuntimeProfile/AgentSession/AgentTurn/IntentResult/CapabilityGrant/TaskRun/TaskPlan/TaskStep/TurnEvent/EvidenceLedger/ExternalContentRef/MemorySelectionRef/AgentContextFacts/ProjectReport）的职责、身份、事件真值、隔离与迁移边界以 SYS 数据结构契约层（A-SYS-I-P-01 §3.1）为唯一契约来源；任何模块不得以旧设计、现有代码、已注册工具、Prompt、前端参数、兼容默认值或测试绿灯反向改写冻结需求语义或隐式扩大授权
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
req_id: REQ-SYS-009
target_type: RULE
target_id: BR-SYS-DATA-008
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
  唯一 profile 失败关闭、TurnEvent 追加/首终态锁存、CapabilityGrant 逐动作授权、TaskRun 纯投影、
  EvidenceLedger/ProjectReport 派生及外部内容/记忆 data-only 隔离均以本文 §3.1 为单一来源；下游 @GOV
  引用这些规则时不得私自定义或反向改写。
- **ProposalBatch 关联而不合并终态**：同批多 PendingDiff 共享 batch 身份和稳定顺序，但逐卡 status/
  effect_status 独立；同文档结构化多卡共享基线并只做一次 ED 前向事务，拒绝一张只反向该卡目标且其他
  卡仍可裁决；批量裁决等价于多个逐卡命令，不得新增批次终态。
- 量化 NFR（迁移耗时、db 体量）延 G5 标定（APC §9）。

## §9 技术债务与盲区

- workspace.db **完整列、约束、索引**与 WS 持久化实现联合确定（SY-03，WS G2），本文为基线骨架。
- 不透明 ID 生成算法（ULID/UUID/自增等）属实现选型，延 L2 实现/G5，本文只约定"不透明标识符"语义。
- 具体序列化格式（payload_ref 指向的 diff 内容编码）归 DE；AgentTurnPayload 的 JSON 形状归 AG；本文只占位共享字段。
  AG 历史恢复不得把缺失的 diff 当前状态默认视为 pending：状态查询归 DE/WS，缺失只可投影为只读不可用。
  TaskRun/TaskPlan/TaskStep 可保存为性能缓存，但不列入不可替代基线；删除后必须能由 agent_turn_events 重建。
- diff / timeline 的状态机与转移归 A-SYS-M-S-01 与 DE/WS M-S，本文只锁字段与枚举集合。
- `DiffProjection.patch_kind=style_ops` 在已打开非激活文档上的自动应用需要 ED 提供无 ProseMirror runtime 的
  StyleOp 事务；若实现阶段未具备该能力，必须让对应 DiffCard 失效并记录债务，不能降级为 DiskState 应用。

### 9.1 Agent 1.0 共享契约的实现迁移边界

- 旧 IntentResultV1/CapabilityGrantV1/AgentTurnPayloadV1 只允许迁移读取；不得继续作为授权或事件真源。
- agent_turn_events 的迁移、索引和 crash-safe append 由 AG 实现阶段完成；落地前不得宣称事件重建已验证。
- TaskRun/TaskPlan 的既有表若保留，只能作可丢弃投影缓存；需用重建测试证明删表/清缓存不改变行为。
- 外部内容与记忆引用仍按各自既有 P1 设计/验证状态开放，本次共享契约收敛不改变能力档位。

## §10 维护与实现入口

1. 共享对象、字段、版本或枚举变化后运行 `npm run governance:generate`，不得人工改写 ADUS 自动生成区。
2. wire 私有字段由各 owner 接口协议维护；跨模块字段不得绕过本文另建近义结构。
3. workspace.db 结构变化必须增加 schema_version 与前向迁移，并同步 WS/DE/AG 的生产者和消费者。
4. ProposalBatch/PendingDiff 的分组、三分流、连续 Proposal 与裁决终态以 DE/ED/WS 当前设计为准；本文只锁共享结构。
5. Agent 实现按 A-AG-I-P-01 的 V2 契约落地 TurnEvent、IntentResult/TaskRun 投影、逐动作
   CapabilityGrant 和 EvidenceLedger/ProjectReport；外部内容与 MemorySelectionRef 仍由各自既有能力状态约束。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-10-08 | v1.22 | ToolCallReceiptV1 增实际资源执行结算 resource_executed 和 workspace_operation_plan 引用，仅源自 Completed WorkspaceOperationReceipt，不代表编辑接受/保存；无 schema 迁移或新权限。 |
| 2026-10-07 | v1.21 | 增 CapabilityExecutionFactV1 started/settled 事实与既有 payload 命名空间契约；迟到效果独立于回合终态，未结算/未知效果禁止删除与盲目重放；无迁移或新授权。 |
| 2026-10-07 | v1.20 | P10：登记不可变 ToolCallReceiptV1 执行/完成/效果分离契约，模型回注与安全事件同源；只引用既有等待或修改对象，不替代裁决、接受、保存真值，不新增迁移或授权。 |
| 2026-09-20 | v1.19 | P03：共享 v2 类型及显式版本分派接入，保留 v1 严格解释；不以类型实现等同系统验收。 |
| 2026-09-18 | v1.18 | P01：安全事件与历史 failure 增可选诊断契约；观察阶段、有限分类与未确认根因分离，旧载荷兼容，不变授权/终态。 |
| 2026-09-15 | v1.17 | P00：登记 AgentContextFacts v2 按需上下文的身份、资源可用性、快照与 revision 字段；保留 v1 解释和 LogicalState 来源约束，声明设计尚未接入代码。 |
| 2026-09-03 | v1.16 | AG-ADAPTIVE-RUNTIME-DESIGN-01：共享运行时对象归一为模型可修订 IntentResult、逐动作 CapabilityGrant、append-only TurnEvent、纯投影 TaskRun/TaskPlan/TaskStep、EvidenceLedger 派生 ProjectReport；增加 agent_turn_events 基线与 ProposalBatch 关联字段，并锁定同文档同批结构化卡共享基线、一次前向事务和目标局部裁决。 |
| 2026-08-29 | v1.15 | AG-CONFORMANCE-CODE-S4A：将 AgentContextFacts / ProjectReport 纳入 Agent 1.0 共享对象，新增 BR-SYS-DATA-008 字段、版本、失败关闭与只读授权边界，并补 REQ-SYS-009 规则映射。 |
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
| 2026-08-21 | v1.15 | 工作区资源操作基线重构：共享契约新增 WorkspaceTimelineEntry、WorkspaceOperationPlan、WorkspaceOperationEntry、CreateDocumentBundle；schema 基线新增资源计划/历史/新建绑定表；重启未终态资源操作统一 Interrupted 且不自动执行。 |
| 2026-08-26 | v1.16 | CHAIN-INTEGRITY-REPAIR-01：补齐 DiffProjection 实际跨模块传输字段，明确设计文档为字段语义源，Rust 共享类型模块为唯一后端定义。 |
