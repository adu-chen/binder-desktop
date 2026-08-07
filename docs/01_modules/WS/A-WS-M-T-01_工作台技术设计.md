---
文档编号：WS-M-T-01
文档状态：A
负责模块：WS
文档职责：WS 工作台模块级 L2 技术设计（规则 SSOT）——workspace 模型与生命周期、文件 I/O 与跨文档操作的边界、pending_diffs 持久化与裁决后落盘、文档级时间轴存储模型与还原、workspace.db 完整 schema（解除 SY-03）；承接 REQ-WS-001~005
上游约束：A-WS-M-D-01_工作台功能需求描述.md（REQ-WS-001~005）、A-CORE-C-P-01_APC.md（§4.1 WS 职责、§6 save 状态机、§9 文件系统安全边界）、A-SYS-I-P-01_数据结构定义.md（WorkspaceRef/DocRef/PendingDiff/TimelineEntry、workspace.db schema 基线、BR-SYS-PERSIST-001、X-SHARED-001）、A-SYS-I-P-02_IPC协议.md（统一错误形状 BR-SYS-OBS-002、流式终态 BR-SYS-STATE-002）、A-SYS-M-S-01_状态机总图.md（X-SEQ-003 注册前置、X-INTER-001 跨模块转移、SY-04）、ADU.md（§3 RULE/CHAIN/CONSTRAINT/TERM 格式）
直接承接：本文 RULE/CHAIN/CONSTRAINT/TERM 为 WS 规则 SSOT，代码经 @GOV 映射；save/diff 落盘/时间轴还原状态机在 A-WS-M-S-01 定义并登记 A-SYS-M-S-01 总图；workspace.db 完整 schema 联合解除 SY-03
使用边界：本文是 WS 规则 SSOT；不重定义跨模块共享结构（WorkspaceRef/DocRef/PendingDiff/TimelineEntry 字段集合归 A-SYS-I-P-01，本文引用不抢定）、不定义 diff 终态转移语义（归 DE）、不定义 IPC 报文/命名（归 A-SYS-I-P-02）、不锁定具体序列化/标量类型/ID 生成算法（延实现期）、状态机内部 states/transitions 归 A-WS-M-S-01
变更要求：workspace.db schema 列变更须遵守 BR-SYS-PERSIST-001 版本化迁移；跨模块共享结构变更须回写 A-SYS-I-P-01 并经 X-SHARED-001 复核；TERM 一经被 term_ref 引用其 en 不可更改（ADU §3.0）
治理成熟度：draft
---

# Binder 工作台技术设计（WS · L2 技术设计）

## 0. Terminology Registry

> WS 主责核心业务概念首批注册（满足"跨 session 易漂移 + 代码有实体承载 + 命名混乱致规则误判"）。
> A-SYS-I-P-01 §6 已将 workspace / 文档时间轴的 TERM 主责显式延至 WS L2，本文据此注册，解除该延期。
> 如人类对命名有异议，可在本区域调整后重跑 generate。

<!-- TERM
term_id: TERM-WS-001
chains: WS-FS
zh: 工作区
en: Workspace
forbidden: 工作空间, 项目根目录, 工作目录, root目录, 项目目录
-->
用户选定的本地根目录及其边界，是文件操作 / 搜索 / diff / 上下文读取的唯一物理边界（WorkspaceRef，
A-SYS-I-P-01）；任何操作路径不得越出活动工作区 root（X-FORBID-004 / APC §9）。

<!-- TERM
term_id: TERM-WS-002
chains: WS-TIMELINE
zh: 文档时间轴
en: DocumentTimeline
forbidden: 历史记录, 版本历史, 历史栈, 版本栈, 历史链
-->
文档级版本节点的有序序列（TimelineEntry，A-SYS-I-P-01），记录落盘与还原产生的版本节点，
支撑历史回溯与还原到先前版本（REQ-WS-004 / REQ-CORE-007）；version_seq 在同一文档内单调递增。

## §1 文档定位

本文是 **WS 模块**的 L2 技术设计，是 WS 规则的唯一来源（SSOT，APC §2/§3.2），承接 WS L1
（REQ-WS-001~005）：

```
REQ-CORE-001/002/007 → REQ-WS-001~005（WS L1）→ 本文 RULE/CHAIN/CONSTRAINT/TERM（SSOT）→ 代码(@GOV)
本文 ←引用← A-SYS-I-P-01（共享结构 + workspace.db 基线）、A-SYS-I-P-02（统一错误形状/流式终态）
本文 →驱动→ A-WS-M-S-01（save/diff 落盘/时间轴还原状态机）→登记→ A-SYS-M-S-01 总图（SY-04）
```

边界（承 WS L1 §6、A-SYS-I-P-01 §1）：

- 本文定义 WS **规则/链路/约束/术语与 workspace.db 完整 schema**；状态机 states/transitions/guards
  归 A-WS-M-S-01（本文只声明其存在与所承载规则）。
- 跨模块共享结构（WorkspaceRef/DocRef/PendingDiff/TimelineEntry）字段与枚举集合归 A-SYS-I-P-01，
  本文**引用不重定义**（X-SHARED-001）；diff 的"接受/拒绝/过期"**终态转移语义归 DE**，WS 仅消费
  `accepted` 信号执行落盘。
- 具体序列化格式、标量类型、不透明 ID 生成算法（ULID/UUID 等）属实现选型，延编码期与 G5。

## §2 模块技术分层与边界

```
WS 物理边界层（本文 SSOT）
  ├─ workspace 模型与生命周期（§3）            打开/切换/关闭 + root 边界守护
  ├─ 文件 I/O 与跨文档操作（§4）               读/写/列举/跨文档，限定 workspace 内
  ├─ pending_diffs 持久化与落盘（§5）          持久化 + accepted 后写文件 + 记节点
  ├─ 文档级时间轴存储与还原（§6）              节点准入 / 存储模型 / 还原执行
  └─ workspace.db 完整 schema（§7，解除 SY-03） 控制态持久化 + 版本化迁移（承 BR-SYS-PERSIST-001）
```

- WS 是文件/搜索/diff/上下文操作的**唯一物理边界**（REQ-CORE-001）；上层模块（ED/DE/AG/CBT/REF/
  AST/KB/TPL/DOC/PRV）经 WS 进行物理文件 I/O，不旁路（X-SEQ-002 / APC §9）。
- 持久化的是**控制态**（pending_diffs、时间轴节点、workspace 注册），用户文档正文以文件形式落
  workspace（本地优先，REQ-CORE-001）。
- 错误统一走 A-SYS-I-P-02 统一错误形状（BR-SYS-OBS-002）；状态化链路中断不得半落盘（对齐
  BR-SYS-STATE-002）。

## §3 workspace 模型与生命周期（WS-FS）

**模型**：活动 workspace 以 WorkspaceRef（A-SYS-I-P-01：workspace_id / root_path / is_active）
标识；root_path 为文件 I/O / 搜索 / diff / 上下文读取的唯一边界。多 workspace 可注册并持久化
（§7 workspaces 表），同一时刻至多一个 is_active。

**生命周期**：打开（选定本地目录→注册并置活动）／切换（换 is_active，切换前持久化未落盘控制态）／
关闭（解除活动，控制态保留可恢复）。生命周期的状态化部分（含切换前持久化）由 §5/§7 控制态持久化
保障，重启后由 workspace.db 恢复。

<!-- CHAIN
chain_id: WS-FS
主责模块: WS
status: active
risk_tier: P1
-->

<!-- RULE
rule_id: BR-WS-DATA-001
主链路: WS-FS
域: DATA
-->
workspace 内文档/资源的读、写、列举、建目录与跨文档操作必经 WS 物理边界层，统一以 WorkspaceRef/DocRef
（A-SYS-I-P-01）定位目标；调用方不得旁路 WS 直接触达文件系统（X-SEQ-002 / APC §9）。跨模块传递的
WorkspaceRef/DocRef 字段集合引用 A-SYS-I-P-01 共享结构，WS 不私自重定义（X-SHARED-001）。代理项目
图片发现只允许读取 `.binder/project_assets.json` 声明的 workspace 相对 `imageRoots`；配置缺失时
使用单一默认根 `images`，不得回退扫描整个 workspace。只递归列举受控根内普通文件的相对路径，跳过
隐藏目录与符号链接；格式/magic/hash 的图片语义校验由 DOC 消费侧完成。

## §4 文件 I/O 与跨文档操作的边界（WS-FS）

文件 I/O（含 AI 发起的读写）、搜索、diff 生成所读上下文、跨文档资源操作，其路径必须落在活动
workspace root 内；任一越界路径一律拒绝并回统一错误形状（不静默截断、不回退到 workspace 外）。
该硬边界以 CONSTRAINT 锁定（§8 X-FORBID-004），是 APC §9 文件系统安全边界在 WS 的技术落实。

> 与 X-SEQ-002 区别：X-SEQ-002 约束"前端不跨层直连 FS/DB/网络"（调用路径分层）；X-FORBID-004 约束
> "文件操作目标路径不得越出 workspace root"（路径包含关系），两者正交，均生效。

**外部变更感知与保存冲突保护**（2026-06-12 缺口补遗，人类确认）：workspace 内文档可能被本应用
之外的程序修改/删除/移动，WS 作为文件物理边界 owner 承担感知与保护职责：

<!-- RULE
rule_id: BR-WS-OBS-001
主链路: WS-FS
域: OBS
-->
WS 必须感知 workspace 内文档的外部变更：对处于打开会话中的文档维护**加载基线**（读取时点内容
哈希），经文件监视或惰性校验检测磁盘内容偏离基线；检测到外部变更（修改/删除/移动）时向订阅方
（ED 编辑会话、DE 过期检测、REF 锚校验）发**外部变更事件**，不得静默吞没；**save 落盘前必须
校验磁盘当前内容与加载基线一致**（SM-WS-01 Saving 前置 guard），不一致即**保存冲突**——不得
静默覆盖外部修改，以统一错误形状（冲突类错误码）回 ED 会话提示用户裁决（确认覆盖 / 放弃重载）；
用户确认覆盖后以重置基线重试落盘。监视不可得场景（如网络盘）以"保存前校验"为兜底下限。
监视机制选型（FSEvents/轮询）与防抖参数延实现期。

> **实现回写（2026-07-12）**：已落地的下限能力是保存前 guard：`ws.read_doc` /
> `doc.open_for_edit` 返回加载基线，`ws.save_doc` / `doc.save_from_edit` 在写盘前读取磁盘当前内容并
> 与基线比对，冲突返回 `WS_SAVE_CONFLICT` 且不进入 `save_atomic()`；用户显式确认后以
> `override=true` 重试。文件监视、`ws.external_change` 事件推送、独立 `check_external_change`
> command 尚未注册，仍属 SY-02 后续实现。

## §5 pending_diffs 持久化与裁决后落盘（WS-DIFF-APPLY）

WS 持久化由 DE/AG/CBT 生成的 pending_diff（A-SYS-I-P-01 PendingDiff）到 workspace.db，使未裁决
diff 重启可恢复（§7）。裁决终态（pending→accepted/rejected/expired）由 **DE 终态机**裁定；WS 仅在
收到 `accepted` 信号时执行落盘：写入目标文档并记**一个**时间轴节点；`rejected`/`expired` 不写文件。

<!-- CHAIN
chain_id: WS-DIFF-APPLY
主责模块: WS
status: active
risk_tier: P1
-->

<!-- RULE
rule_id: BR-WS-STATE-001
主链路: WS-DIFF-APPLY
域: STATE
-->
pending_diff 落盘仅由 DE 终态机的 `accepted` 信号触发：WS 在 accepted 时将 diff 写入目标文档并记
恰好一个时间轴节点；`rejected`/`expired` 一律不写文件。写盘失败时不记时间轴节点、回统一错误形状
（BR-SYS-OBS-002），控制态保持可重试且原文件不被破坏（不产生半落盘副作用，对齐 BR-SYS-STATE-002）。
diff 的终态转移语义归 DE，本规则只约束 WS 侧的落盘触发与原子性。

> 跨模块转移点：`DE.diff终态(accepted) → WS.WS-DIFF-APPLY(落盘)` 须在 A-SYS-M-S-01 §5 登记为
> X-INTER-001 转移契约，两侧对接一致（accepted 信号语义、落盘确认回执）。

## §6 文档级时间轴存储模型与还原（WS-TIMELINE）

**节点准入**：时间轴节点（TimelineEntry，A-SYS-I-P-01）在三类事件各记**一个**：（a）diff 落盘
（§5 accepted 后）、（b）历史还原成功、（c）保存成功（save，2026-08-04 人类确认解除 WS-03「save
默认否」准入——每次保存留一个历史版本；内容与最近节点快照相同则跳过，避免内容重复的空版本）。
同一文档 version_seq 单调递增。

**存储模型**：每个节点引用一份可取回的版本快照（§7 timeline.snapshot_ref）；快照的**物理存储形态**
（workspace.db 内联 blob 抑或 workspace 内 sidecar 文件、全量抑或增量）属实现选型，延编码期（§11），
但"每节点有可取回快照、还原据此重建版本"的模型在本文锁定。

**回溯与还原**：列出文档历史节点→选定预览→确认还原；还原将文档重建为选定版本并记一个**新**节点
（不抹除历史，前进式还原）；还原失败保持现状不破坏当前版本。

<!-- CHAIN
chain_id: WS-TIMELINE
主责模块: WS
status: active
risk_tier: P1
-->

<!-- RULE
rule_id: BR-WS-DATA-002
主链路: WS-TIMELINE
域: DATA
-->
文档级时间轴节点以 A-SYS-I-P-01 的 TimelineEntry 为共享结构（WS 不重定义，X-SHARED-001）：落盘、
还原与保存成功（save，内容与最近节点快照不同才记，避免空版本）各记恰好一个节点，同一文档内
version_seq 单调递增且不复用；每个节点须关联一份可取回的版本快照（snapshot_ref），使还原可据此
重建目标版本。节点不可被静默删除以伪造历史连续性。

<!-- RULE
rule_id: BR-WS-STATE-002
主链路: WS-TIMELINE
域: STATE
-->
历史还原到选定版本必须前进式记一个新时间轴节点（不抹除既有历史）；还原失败时文档保持还原前的现状，
不得产生半还原副作用，并回统一错误形状（BR-SYS-OBS-002）。还原成功与失败为互斥终态，二者之外不得
出现"部分还原"的中间落盘态。

## §7 workspace.db 完整 schema（PERSIST，解除 SY-03）

承接 A-SYS-I-P-01 §4 基线，联合敲定完整列/约束/索引，解除 SY-03。持久化对象为**控制态**，非用户
正文。下表列语义为语言无关描述；具体标量类型、序列化、ID 生成算法延实现期（§11），不构成虚假精度。

**schema_meta**（schema 版本，单行）

| 列 | 语义 | 约束 |
|---|---|---|
| schema_version | 当前 schema 版本，单调递增，迁移唯一驱动 | 非空整数，单行 |
| applied_at | 该版本应用时间 | 非空 |

**workspaces**（workspace 注册与控制态）

| 列 | 语义 | 约束 |
|---|---|---|
| workspace_id | workspace 不透明标识 | PK |
| root_path | 用户选定本地根目录绝对路径（唯一边界） | 非空 |
| is_active | 是否当前活动 workspace | 至多一行为真 |
| opened_at | 首次打开时间 | 非空 |
| last_active_at | 最近活动时间 | 可空 |

**pending_diffs**（待裁决 diff 持久化，重启可恢复）

| 列 | 语义 | 约束 |
|---|---|---|
| diff_id | diff 不透明标识 | PK |
| workspace_id | 所属 workspace | FK→workspaces |
| rel_path | 目标文档 workspace 内相对路径 | 非空，不越界（X-FORBID-004） |
| origin_module | 生成来源模块编码（DE/AG/CBT…） | 非空 |
| status | diff 状态（pending/accepted/rejected/expired） | 枚举集合归 A-SYS-I-P-01，转移归 DE |
| payload_ref | diff 内容载荷引用（内容形状归 DE） | 非空 |
| created_at | 创建时间 | 非空 |
| decided_at | 裁决时间（accepted/rejected/expired 时写） | 可空 |
| 索引 | (workspace_id, status) | 加速按 workspace 拉取未裁决 diff |

**timeline**（文档级时间轴节点）

| 列 | 语义 | 约束 |
|---|---|---|
| entry_id | 节点不透明标识 | PK |
| workspace_id | 所属 workspace | FK→workspaces |
| rel_path | 目标文档 workspace 内相对路径 | 非空，不越界 |
| version_seq | 文档版本序，单调递增 | 同 (workspace_id, rel_path) 内唯一 |
| entry_kind | 节点来源（apply 落盘 / restore 还原 / save 保存） | 非空枚举 |
| label | 版本标注 | 可空 |
| snapshot_ref | 版本快照引用（还原据此重建） | 非空 |
| created_at | 创建时间 | 非空 |
| 索引/唯一 | UNIQUE(workspace_id, rel_path, version_seq)；索引 (workspace_id, rel_path) | 保序与去重 |

<!-- RULE
rule_id: BR-WS-PERSIST-002
主链路: DATA-MIGRATE
域: PERSIST
-->
WS 承接并落实 workspace.db 完整 schema（§7），其结构变更必须遵守 SYS 版本化迁移契约
（BR-SYS-PERSIST-001）：每次结构变更配套 `schema_meta.schema_version` 递增与迁移步骤，迁移不得
破坏既有 workspaces/pending_diffs/timeline 持久化态；应用重启后未裁决 pending_diffs 与时间轴节点
必须完整可恢复（REQ-WS-005 验收）。本规则约束 WS 持久化结构与恢复，不重定义 SYS 迁移驱动语义。

## §8 跨模块约束（CONSTRAINT）

<!-- CONSTRAINT
constraint_id: X-FORBID-004
摘要: 文件操作/搜索/diff/上下文读取的目标路径不得越出活动 workspace root；任一越界路径一律拒绝，不静默截断或回退到 workspace 外
涉及模块: WS, ED, DE, AG, CBT, REF, AST, KB, TPL, DOC, PRV
链路影响: WS-FS, WS-SAVE, WS-DIFF-APPLY, WS-TIMELINE
-->

**复用既有约束，不重复定义**：

- **X-SEQ-002**（A-SYS-C-T-01）：前端不跨层直连 FS/DB/网络，必经 Service→command→Rust；WS 文件 I/O
  在此分层下经注册 command 触达。
- **X-SHARED-001**（A-SYS-I-P-01）：WorkspaceRef/DocRef/PendingDiff/TimelineEntry 跨模块共享结构经
  SYS 契约层定义，WS 引用不重定义。
- **X-SEQ-003**（A-SYS-M-S-01）：WS save/diff 落盘/时间轴还原为状态化核心链，须先建状态机并登记总图
  方可实现——本文 §5/§6 与 A-WS-M-S-01 满足之。
- **X-INTER-001**（A-SYS-M-S-01）：`DE.diff终态(accepted)→WS 落盘` 跨模块转移点须双侧对接一致。

## §9 需求到规则映射（REQ_MAP）

<!-- REQ_MAP
req_id: REQ-WS-001
target_type: CHAIN
target_id: WS-FS
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-WS-001
target_type: CONSTRAINT
target_id: X-FORBID-004
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-WS-002
target_type: RULE
target_id: BR-WS-DATA-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-WS-002
target_type: CHAIN
target_id: WS-SAVE
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-WS-002
target_type: RULE
target_id: BR-WS-PERSIST-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-WS-002
target_type: RULE
target_id: BR-WS-OBS-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-WS-003
target_type: CHAIN
target_id: WS-DIFF-APPLY
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-WS-003
target_type: RULE
target_id: BR-WS-STATE-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-WS-004
target_type: CHAIN
target_id: WS-TIMELINE
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-WS-004
target_type: RULE
target_id: BR-WS-DATA-002
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-WS-004
target_type: RULE
target_id: BR-WS-STATE-002
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-WS-005
target_type: RULE
target_id: BR-WS-PERSIST-002
status: mapped
-->

> save 保存链（WS-SAVE）与其规则 BR-WS-PERSIST-001 定义于 §10（save 落盘可靠性），承接 REQ-WS-002。

## §10 save 保存链（WS-SAVE）

编辑内容的 save 保存须可靠原子写入 workspace 内目标文档：save 进行中中断不得产生半落盘副作用，
失败时原文件保持不变并回统一错误形状，控制态可重试。save 的状态化生命周期（Idle/Dirty/Saving/
Saved/SaveError）由 A-WS-M-S-01 的 SM-WS-01 定义。

<!-- CHAIN
chain_id: WS-SAVE
主责模块: WS
status: active
risk_tier: P1
-->

<!-- RULE
rule_id: BR-WS-PERSIST-001
主链路: WS-SAVE
域: PERSIST
-->
save 保存必须原子可靠地写入活动 workspace 内的目标文档（路径不越界，X-FORBID-004）：写入中断不得
产生半落盘副作用，失败时目标文件保持 save 前内容并回统一错误形状（BR-SYS-OBS-002），控制态保持
可重试。本规则约束 save 落盘的原子性与可恢复性，save 的状态转移归 SM-WS-01（A-WS-M-S-01）。

## §11 测试与验收

- **边界守护**（X-FORBID-004 / BR-WS-DATA-001，REQ-WS-001/002 验收）：构造越出 workspace root 的
  读/写/搜索/diff 上下文路径，全部被拒；workspace 内合法路径正常 I/O。
- **save 原子可靠**（BR-WS-PERSIST-001 / SM-WS-01）：注入 save 中断/失败，原文件不被破坏、无半落盘、
  可重试成功。
- **diff 仅 accepted 落盘**（BR-WS-STATE-001 / SM-WS-02）：accepted 触发写文件 + 记一个节点；
  rejected/expired 不写文件；落盘失败不留半节点、控制态可重试。
- **时间轴节点与还原**（BR-WS-DATA-002 / BR-WS-STATE-002 / SM-WS-03）：落盘与还原各记一个节点，
  version_seq 单调递增；还原成功记新节点，还原失败保持现状无半还原。
- **持久化可恢复**（BR-WS-PERSIST-002，承 BR-SYS-PERSIST-001，REQ-WS-005 验收）：构造含未裁决
  pending_diffs 与时间轴的 workspace.db，经一次 schema_version 递增迁移后重启，控制态完整恢复、无丢失。
- 量化 NFR（I/O 延迟、恢复耗时）延 G5 标定（APC §9）。

## §12 技术债务与盲区（诚实声明）

- ~~时间轴快照物理存储形态未锁定~~ → **已锁定（PI-BE-05，2026-06-23）**：选型 = workspace.db
  `timeline_snapshots` 表**内联全量快照 BLOB**，**内容寻址**（snapshot_ref=`blob_{内容哈希}`）**去重**
  （同内容多节点共享一份）。timeline.snapshot_ref 指向之；`ws.preview_node` 据此取回版本内容、
  `ws.restore_node` 取回后原子写回并前进式记新节点。增量快照为后续优化（大文档体积），暂用全量。
- ~~**save 节点准入策略待定**~~ → **已定档（2026-08-04 人类决策）**：每次保存成功且内容与最近节点
  快照不同时记一个 `save` 节点（entry_kind=save，§7 schema 已预留枚举）；内容未变的保存跳过，
  避免内容重复的空版本；节点经内容寻址快照去重共享存储。
- **多 workspace 控制态隔离细节**：workspaces 表支持多注册，但并发多活动/切换时的控制态隔离与缓存
  失效策略延实现期（承 WS L1 WS-04）。
- ~~不透明 ID 生成算法（ULID/UUID/自增）延实现期~~ → **已定档（G-4，2026-07-02，A-CORE-X-R-03 §7）**：
  选型 = **前缀 + UUIDv4**（`ws_{uuid}` / `diff_{uuid}` / `pl_{uuid}`，Rust `uuid` crate v4；前端 blockId
  归 ED，选型 `block_` + `crypto.randomUUID()`，见 A-ED-M-T-01 §11.1）。全局唯一、跨进程无碰撞、
  无时序信息泄漏；具体标量类型与序列化仍延实现期。
- **内容基线哈希已定档（G-5，2026-07-02，A-CORE-X-R-03 §7）**：选型 = **FNV-1a 64**（save.rs
  `content_hash`）。定档理由：用途为**非对抗性**外部变更检测（磁盘 vs 加载基线比对）与时间轴快照
  内容寻址去重（snapshot_ref=`blob_{hash}`），文档级碰撞概率 ~2⁻⁶⁴/对，个人工作台量级下可忽略；
  零依赖、与内核契约（baseline: u64）及既有持久化基线兼容，换算法将使已存基线失配。IPC 层以
  十进制字符串承载该 u64，避免 JS `number` 精度丢失。**边界声明**：
  本哈希不承诺抗对抗碰撞，不得挪用为完整性/安全校验；若时间轴去重未来进入对抗场景须另立加密哈希。
- workspace.db 完整列已在 §7 敲定（解除 SY-03 的列定义部分）；具体 DDL 与索引调优随实现期基准测试微调。

## §13 后续动作

1. 本文落地后随下一次 `npm run governance:generate` 注册：WS-FS/WS-SAVE/WS-DIFF-APPLY/WS-TIMELINE
   链路、BR-WS-DATA-001/002、BR-WS-STATE-001/002、BR-WS-PERSIST-001/002 规则、X-FORBID-004 约束、
   TERM-WS-001/002 术语、REQ-WS-001~005 的 REQ_MAP 进 ADUS。
2. 续 WS G2：A-WS-M-S-01 定义 SM-WS-01（save）/ SM-WS-02（diff 落盘）/ SM-WS-03（时间轴还原）状态机，
   登记 A-SYS-M-S-01 总图（save 行 planned→active，回填 SY-04），并在 §5 登记 X-INTER-001 转移契约。
3. 待人类宣布 WS G2 闭合（AI 不自行宣布晋级，APC §2）。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-10 | v1.0 | 初版（WS G2 首件）：WS 规则 SSOT——workspace 模型与生命周期、文件 I/O 与跨文档边界、pending_diffs 持久化与 accepted 后落盘、文档级时间轴存储模型与还原、save 保存链、workspace.db 完整 schema（解除 SY-03 列定义）；注册链路 WS-FS/WS-SAVE/WS-DIFF-APPLY/WS-TIMELINE（均 P1）、规则 BR-WS-DATA-001/002·BR-WS-STATE-001/002·BR-WS-PERSIST-001/002、约束 X-FORBID-004（复用 X-SEQ-002/003·X-SHARED-001·X-INTER-001）、术语 TERM-WS-001 Workspace / TERM-WS-002 DocumentTimeline（解除 A-SYS-I-P-01 §6 延期）；REQ_MAP（REQ-WS-001~005）；债务盲区（快照存储形态/ save 节点准入 / 多 workspace 隔离延实现期） |
| 2026-06-12 | v1.1 | Q2 缺口补遗（人类确认采纳）：§4 新增 BR-WS-OBS-001 外部变更感知与保存冲突保护——加载基线（内容哈希）、文件监视/惰性校验、外部变更事件供订阅方（ED/DE/REF）、save 前基线校验、冲突不静默覆盖须用户显式裁决、网络盘以保存前校验兜底；REQ_MAP 补 REQ-WS-002→BR-WS-OBS-001 |
| 2026-07-02 | v1.2 | §12 选型定档回填（人类授权执行 A-CORE-X-R-03 §7 补全计划）：G-4 不透明 ID = 前缀 + UUIDv4（Rust uuid crate；blockId 归 ED §11.1）；G-5 内容基线哈希 = FNV-1a 64 定档（非对抗性变更检测/内容寻址去重的用途边界声明，与既有 IPC 契约及持久化基线兼容） |
| 2026-07-12 | v1.3 | 三态实现回写：保存前 baseline guard 已落地到 `ws.save_doc` / `doc.save_from_edit`，冲突返回 `WS_SAVE_CONFLICT` 且不写文件，`override=true` 仅源自用户显式确认；声明 baseline IPC 字符串化；文件监视、`ws.external_change` 推送与独立检测 command 仍延后 |
| 2026-07-16 | v1.4 | 补齐 WS-FS 建目录语义：建目录属于 workspace 内文件树结构操作，必经 WS 物理边界层，供 Agent `create_folder` 结构工具复用；目标为目录幂等成功，目标为文件拒绝 |
| 2026-07-29 | v1.5 | BDOCX-IMAGE-AGENT-EDIT-01：BR-WS-DATA-001 增加 workspace 项目图片只读发现，锁定相对普通文件、隐藏目录跳过、符号链接和越界拒绝边界。 |
| 2026-07-29 | v1.6 | BDOCX-IMAGE-ADVANCED-AGENT-OPS-01：项目图片发现收敛到 `.binder/project_assets.json` 的相对 imageRoots，缺失时只使用 `images`，禁止全 workspace 回退。 |
| 2026-08-04 | v1.7 | WS-TIMELINE save 准入定档（人类决策）：§6 节点准入与 BR-WS-DATA-002 将保存成功纳入节点准入——每次保存成功且内容与最近节点快照不同时记 `save` 节点，内容未变跳过；§12 债务「save 节点准入策略待定」改为已定档，解除 WS-03 该子项 |
| 2026-08-07 | v1.8 | WS-DIR-DELETE-01：新增 `ws.delete_folder`（mode=empty\|recursive）——fileops::delete_folder 实现两级删除语义（empty 非空拒 WS_NOT_EMPTY / recursive 递归删除），listing::cleanup_bdocx_under 清洗删除目录前缀下的 BDocx 清单路径（empty 无命中不写）；目标越界经 resolve_active 守界。 |
| 2026-08-07 | v1.9 | WS-NEW-BUTTON-01：文件树「＋」新建按钮下拉（格式选择 + 激活区域 + 文件夹入口）复用 WS-FS `create_doc` / `create_folder`，前端激活区域与文件夹入口为 UI 层组合，无后端命令面变化。 |
