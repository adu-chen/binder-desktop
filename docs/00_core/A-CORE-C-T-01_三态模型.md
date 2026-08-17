---
文档编号：CORE-C-T-01
文档状态：A
负责模块：CORE
文档职责：三态模型（DiskState / LogicalState / DisplayState）跨模块权威定义——三态命名与正式中英文定义、每一态的唯一持有者与转换触发条件、单向数据流契约、术语映射表、审计发现 D2~D5 的决议与落地约束
上游约束：A-CORE-C-P-01_APC.md（§6 状态机阈值）、A-SYS-C-T-01_系统总体架构.md（§2 分层、X-SEQ-002）、A-SYS-C-T-02_前端交互架构与流程总图.md（X-FORBID-007 前端投影）、A-ED-M-T-01_编辑器技术设计.md（ED-EDIT/ED-DECOR）、A-WS-M-T-01_工作台技术设计.md（WS-FS/WS-SAVE）、A-DE-M-T-01_DiffEdit技术设计.md（DE-DIFF）、A-DE-M-S-01_DiffEdit状态机设计.md（SM-DE-01）、A-ED-M-S-01_编辑器状态机设计.md（SM-ED-01）、A-WS-M-S-01_工作台状态机设计.md（SM-WS-01/02）、A-CORE-X-R-03_需求实现对齐审计与补全计划.md
直接承接：三态全局规则 X-FORBID-008 经本文件定义，供下游 ALL 模块引用；TERM-CORE-001/002/003 术语供全模块 @GOV term_ref 引用；所有涉及"保存/脏态/缓存/渲染/装饰/落盘"的代码与设计文档以此文档为权威命名与约束源
使用边界：本文是跨模块公共概念定义层（L2.5），定义三态契约与术语映射，不替代各模块 L2 状态机/规则细节（SM-* 内部转移仍归各 M-S，本文只标注每态归属）；不新增模块级业务规则；不定义具体 IPC payload 字段
变更要求：三态任一态的持有者或转换触发条件变更须架构评审；TERM 一经被 term_ref 引用其 en 不可更改（ADU §3.0）
治理成熟度：draft
---

# Binder 三态模型（CORE · L2.5 跨模块公共概念定义）

## §0 文档定位与阅读前提

本文是 Binder 项目**三态模型（DiskState / LogicalState / DisplayState）的跨模块权威定义源**。
它回答以下问题：

1. 磁盘内容、编辑内存、屏幕渲染三者如何分层？
2. 每一层的**唯一持有者**是谁、**谁有权修改**它、**什么动作触发**跨层转换？
3. 为什么显示层不能反向修改逻辑层、逻辑层不能绕过保存动作反向修改磁盘层？
4. 所有涉及"数据归属哪一态"的旧概念如何映射到新体系？
5. 审计报告 D2~D5 发现的模糊点如何决议？

使用对象：全部 L2 技术设计文档编写者、代码实现者、代码评审者。
阅读前提：已理解 Binder 分层架构（A-SYS-C-T-01 §2）与前端投影约束（X-FORBID-007，A-SYS-C-T-02）。

```
约束体系位置：
X-SEQ-002（前端不跨层直连 FS）  →  本文件（三态单向数据流）  →  X-FORBID-007（前端投影）
        ↓                              ↓
  各模块 SM-* 状态机（本文只标注每态归属，不替代转移细节）
```

---

## §1 三态正式定义

### 1.1 磁盘态（DiskState）

| 项目 | 内容 |
|------|------|
| **正式中英文名** | 磁盘态 / DiskState |
| **定义** | 文件在磁盘文件系统上的原始字节内容。编辑器"获取文件信息"时读取它；用户执行"保存"操作后它才被修改。磁盘态是持久化、断电不丢的物理数据 |
| **唯一持有者** | 文件系统（OS 文件系统 + workspace 目录） |
| **谁有权修改** | **仅** `save_atomic()`（[save.rs:L60-80](file:///Users/imatstarbucks/binder/src-tauri/src/workspace/save.rs#L60-L80)）、`create_doc()`（[fileops.rs:L26-40](file:///Users/imatstarbucks/binder/src-tauri/src/workspace/fileops.rs#L26-L40)）、`rename_doc()`（[fileops.rs:L53-70](file:///Users/imatstarbucks/binder/src-tauri/src/workspace/fileops.rs#L53-L70)）三个物理写入口 |
| **修改触发器** | 已存在 LogicalState 时的用户 ⌘S 保存；不存在 LogicalState 的 PendingDiff 经用户接受后由 `ws.apply_diff` 原子落盘；文件 CRUD（创建/重命名/删除） |
| **修改路径必须经过** | Rust 后端（X-SEQ-002），前端不直接触达文件系统 |
| **读取触发器** | 打开文档、外部变更检测、重载 |

**实现回写（2026-07-12）**：当前代码已把"逻辑态 → 磁盘态"保存路径落为 `ws.save_doc` /
`doc.save_from_edit` 统一携带加载基线与 `override`。后端在调用 `save_atomic()` 之前执行
`save_with_baseline_guard()`：若磁盘当前内容哈希偏离加载基线且 `override=false`，返回
`WS_SAVE_CONFLICT`，不写文件；只有用户显式确认后以 `override=true` 重试，才允许覆盖磁盘态。
`baseline` 的内核语义仍是 FNV-1a 64 `u64`，但 IPC 载荷以十进制字符串承载，避免 JS `number`
精度丢失。

### 1.2 逻辑态（LogicalState）

| 项目 | 内容 |
|------|------|
| **正式中英文名** | 逻辑态 / LogicalState |
| **定义** | 编辑器加载磁盘态、或由新建预览/打开迁移建立的内存结构化模型与持久化控制态，是所有"真实编辑内容"的权威层。存在 LogicalState 时，AI diff 的受控预览直接修改这一层；显式接受或对仍有效的 pending diff 执行保存，都会改变其裁决生命周期。只有执行保存时，才把"保存那一刻的逻辑态"写回磁盘，从而修改磁盘态 |
| **唯一持有者** | **ProseMirror `doc` 结构化模型**（激活文档权威真源） + `edStore.EditSession`（已打开非激活文档的序列化 LogicalState，激活文档为派生缓存） + workspace.db 中的 `ed_open_tabs`/`ed_session_snapshots`/`pending_diffs`/`timeline`/`diff_payloads`（持久化控制态） |
| **谁有权修改** | 用户编辑操作（经受控 editor command）、存在 LogicalState 时 AI diff 的受控预览或打开迁移（均经 ED 受控接口）、样式操作（StyleOp 经受控接口）；**显示层不得直接修改**（见 §1.4） |
| **修改路径必须经过** | ED 受控编辑接口（`editor.chain().command()`、`insertContentAt`、`setContent` 等，X-FORBID-005） |
| **保存写入磁盘的前提** | 必须触发 SM-WS-01 保存动作（`save_atomic`），不绕过 |

**实现回写（2026-07-12）**：当前 ED 受控编辑入口收敛为 `applyEditOps()`（块替换、字符区间替换、
整篇替换）与 `applyStyleOps()`（StyleOp 批内原子）。`deStore` 的 pending diff 进入当前编辑器草稿、
reject 回滚、示例 diff 取当前内容，都必须读取当前 ProseMirror doc 或经该受控入口修改，不再以
`edStore.EditSession.content` 派生缓存作为逻辑真值。

**异常退出后的可信度边界（2026-07-20，APP-CLOSE-DIFF-LIFECYCLE-01）**：`ed_session_snapshots`/
`pending_diffs`（status=pending）等持久化控制态，只在"上次为正常关闭"的前提下可信任为可恢复的
LogicalState/待裁决态。若上次退出为异常退出（崩溃/强杀/断电，未经 `BR-SYS-STATE-004` 关闭护栏），
应用启动必须先执行异常退出清算（`BR-SYS-STATE-005`，A-SYS-M-T-02 §4）：dirty 的 LogicalState
持久化镜像一律丢弃、改从 DiskState 重建；残留 pending diff 一律转 expired 只读终态。清算完成前，
不得把这些持久化行当作权威 LogicalState 或可裁决 LogicalCandidate 加载。

**已打开文档决议（2026-07-18）**：只要文档已经打开，AG/DE/ED 链路必须以该文档的
LogicalState 为唯一真源。激活文档在 ProseMirror `doc` 与 `activePath/loadSeq/editFormat`
绑定一致时读取实时序列化结果；标签切换过渡期若目标文档 `EditSession.content` 已存在，则该
session content 是待装载的活动 LogicalState，AG 可直接读取，不以 TipTap 渲染完成作为前提；若
目标 session content 仍为空则短暂等待，超时后阻断。已打开但非激活文档读取
`edStore.EditSession.content` 及其持久化镜像 `ed_session_snapshots`。AG 发起轮次、只读工具、
ReferencePayload 校验和 DiffProjection 生成均不得退回读取 DiskState。若已绑定的 active
ProseMirror 无法序列化、缺少 session 版本或基线校验失败，必须阻断本次 AgentTurn 或使对应
DiffCard 失效，不允许用 DiskState 或旧快照兜底。

**关于双副本的决议（针对审计 D3）：**

逻辑态在内存中有两个形态的副本：

- **权威源**：ProseMirror `doc` 结构化模型（TipTap `editor.state.doc`）
- **派生缓存**：`edStore.EditSession.content`（字符串形式，由 `serialize(editor)` 序列化得出）

关系：`doc` 是唯一真源，`content` 是**派生缓存**，用于跨组件通信、切标签时的占位恢复、与 IPC 后端交互时的载荷序列化。

- `serialize()`（[Editor.tsx:L61-L68](file:///Users/imatstarbucks/binder/src/components/ed/Editor.tsx#L61-L68)）是**只读导出**操作，从 `doc` → `content`，不反向灌入其他逻辑
- 加载时 `setContent()`（[Editor.tsx:L216-L219](file:///Users/imatstarbucks/binder/src/components/ed/Editor.tsx#L216-L219)）从 `content` → `doc`，是**外部数据（磁盘态 / 上一会话缓存）加载人逻辑态**，不是逻辑态内部交换
- `edStore.updateContent()` 对已绑定的激活文档仅用于切换标签或 AG 快照采集时的**回写占位**，任何人需要已绑定激活文档真值时必须从 `editor.state.doc` 取；标签切换过渡期的目标活动文档尚未绑定 ProseMirror 时，`EditSession.content` 是该目标文档当前 LogicalState 的序列化持有者；对已打开非激活文档，`EditSession.content` 同样是当前 LogicalState 的序列化持有者，必须携带版本与内容哈希用于基线校验

**信息丢失声明**：`serialize`→`setContent` 的序列化-反序列化循环可能产生信息丢失的风险等级：
- **text/md 格式**（纯文本 + `getText()`→`textToDoc()`）：**低风险**。纯文本无格式信息，往返保真（裁定 B）
- **html/html_file 格式**（docx 编辑链 / .html 独立文件）：**中等风险**。`editor.getHTML()`→`generateJSON()` 双向转换是已知限制（参见 A-ED-M-T-01 §11.1 docx 样式子集三级契约），属于**已知技术债务**，非待修复缺陷。实现期的限制包括：特殊 HTML 实体序列化、data-block-id 跨格式保留度等

### 1.3 显示态（DisplayState）

| 项目 | 内容 |
|------|------|
| **正式中英文名** | 显示态 / DisplayState |
| **定义** | 编辑器实际渲染出来的内容。数据源 = 逻辑态（ProseMirror doc） + 纯装饰性叠加（decoration set）。显示态永远不能反向修改逻辑态（单向数据流：逻辑 → 显示，禁止 显示 → 逻辑） |
| **唯一持有者** | ProseMirror `DecorationSet`（DiffDecorationExtension + Pagination + 其他 Decoration） |
| **谁有权修改** | 显示态由逻辑态变更（ProseMirror `tr.docChanged`）或外部 `setMeta` 驱动重新计算。decoration 代码只读 `tr.doc`，输出 `DecorationSet`，不写回文档模型 |
| **禁止行为** | 任何 decoration 代码不得直接调用 `tr.insertText`、`tr.replaceRangeWith`、`tr.setNodeMarkup` 等修改文档模型的 API。X-FORBID-005 是此禁令在 ED 层的技术落实 |
| **叠加层类型** | `Decoration.node`（块级背景覆盖，如 GreenReview 绿色背景）、`Decoration.widget`（行间元素，如分页缝、before/after 预览）、DOM 级预览（ResizableImage 拖拽中 `img.setAttribute`）|

### 1.4 单向数据流全局契约

```
                                  save_atomic(仅写入口)
  磁盘态(DiskState) ──────────→  磁盘态(DiskState)new
       ↑    ↑                        ↑
       │    │ save (⌘S)              │ save
       │    │   ↓                    │
       │  逻辑态(LogicalState) ──────┘
       │       ↑    │
       │       │    │ 编辑/预览
       │       │    ↓
       │   显示态(DisplayState)
       │       │
       └───────┘ 加载/Load
```

**三条单向约束（不可逆违反）：**

1. **显示态 → 逻辑态：禁止**。显示态的任何计算/交互不得直接或间接修改逻辑态的持有者（ProseMirror doc / edStore.content 等）。逻辑态的修改必须经受控接口（editor command / insertContentAt / setContent 等，X-FORBID-005）
2. **逻辑态 → 磁盘态：仅通过保存动作**。已经存在的 LogicalState 向 DiskState 的转换只能由保存动作（`save_atomic` 或其等价路径）触发，不得绕过保存链直接写盘；不存在 LogicalState 的 PendingDiff 不发生这条转换，只有用户接受后才可由 `WS-DIFF-APPLY` 原子写入 DiskState
3. **磁盘态 → 逻辑态：仅通过加载/重载**。磁盘态变为逻辑态的唯一路径是文档打开/重载（`read_doc` / `open_for_edit`），加载后的编辑全部在逻辑态层面，不影响磁盘态直到保存

---

## §2 全局强制规则

### 2.1 Diff 罗盘三分流规则

<!-- RULE
rule_id: BR-CORE-STATE-001
主链路: DE-DIFF, ED-EDIT, WS-SAVE
域: STATE
-->
**已有 LogicalState 的受控预览与保存**：目标文档已经拥有 LogicalState 时，DE 返回的 PendingDiff 必须经 ED
受控接口直接把 diff 内容投影到该 LogicalState，并以 GreenReview 等 DisplayState 标示增量。用户显式接受不得再次
应用、回滚或写入内容，只能推进 DiffCard 的裁决生命周期；用户保存当前 LogicalState 时，若该 pending diff 的 after
基准仍有效，保存动作等同接受，必须先把卡置为 `Accepted/awaiting_save` 再写入 DiskState。用户编辑、后续预览、
外部变更或超时推进基准时，旧 DiffCard 必须先失效且不可再接受/拒绝；此后的保存只写当前 LogicalState，不得误裁决旧卡。

<!-- RULE
rule_id: BR-CORE-STATE-002
主链路: DE-DIFF, WS-DIFF-APPLY
域: STATE
-->
**无 LogicalState 的卡片挂载与接受落盘**：目标文档尚无 LogicalState 时，PendingDiff 只能由 DiffCard 临时挂载
DiffPayload/前后对比，不得提前建立或修改 LogicalState，也不得提前写 DiskState。用户接受该仍为后台分支的
PendingDiff 时，DE 必须驱动 WS-DIFF-APPLY 原子写入 DiskState；拒绝、过期或落盘失败均不得写入文件。

<!-- RULE
rule_id: BR-CORE-STATE-003
主链路: DE-DIFF, ED-EDIT, ED-DECOR, WS-SAVE
域: STATE
-->
**pending 后打开文档的迁移**：无 LogicalState 的 PendingDiff 在 pending 期间若用户打开目标文档，ED 必须以
同一 DiffPayload 和基准建立该文档的 LogicalState，把 diff 修改继承进内容并显示绿色增量；该 DiffCard 随即
切换为已有 LogicalState 的路径。之后显式接受只改变卡片生命周期；对仍有效 pending 卡执行保存则同时表达接受意图并写入 DiskState。不得再走后台接受落盘。

<!-- CONSTRAINT
constraint_id: X-FORBID-008
摘要: 三态单向数据流——显示态不得直接或间接修改逻辑态持有者（ProseMirror doc / edStore.content 等），逻辑态的修改必须经受控接口（X-FORBID-005）；已有 LogicalState 向 DiskState 的转换只能由保存动作触发。无 LogicalState 的 PendingDiff 仅可按 BR-CORE-STATE-002 在用户接受后经 WS-DIFF-APPLY 原子落盘
涉及模块: ALL（SYS, WS, ED, DE, AG, CBT, DOC, PRV, REF, AST, KB, TPL, ACCT, ENT）
链路影响: IPC-CALL, ED-EDIT, ED-DECOR, DE-DIFF, WS-FS, WS-SAVE, WS-DIFF-APPLY, WS-TIMELINE, AG-TURN, AG-ASSIST, AG-TOOL, CBT-BUILD, DOC-CONVERT
-->

<!-- CONSTRAINT
constraint_id: X-FORBID-009
摘要: AI 生成内容默认属于"逻辑态候选（LogicalCandidate）"；存在 LogicalState 时候选须经 DE/ED 受控预览或打开迁移进入 LogicalState，显式接受或对仍有效 pending 卡保存都会裁决；不存在 LogicalState 时候选保持 DiffCard 挂载，只有用户接受才可经 WS-DIFF-APPLY 写入 DiskState。AI 产出不得绕过三分流受控路径直写逻辑态持有者或磁盘
涉及模块: DE, AG, ED, CBT
链路影响: DE-DIFF, AG-TURN, AG-ASSIST, ED-EDIT, CBT-BUILD
-->

> X-FORBID-009 覆盖审计 D2/D5：
> - deStore 是**逻辑候选态的前端投影**（不是 DisplayState），其投影必须按 BR-CORE-STATE-001~003 经 ED/WS 受控接口产生副作用
> - 已有 LogicalState 的路径是“预览进入 LogicalState → 接受仅裁决 → 保存”；未打开文档路径是“卡片挂载 → 接受后原子落盘”；pending 后打开文档必须从后者迁移至前者
> - 路径选择判断表见 A-DE-M-S-01 §4

---

## §3 术语注册

<!-- TERM
term_id: TERM-CORE-001
chains: IPC-CALL, ED-EDIT, ED-DECOR, DE-DIFF, WS-FS, WS-SAVE
zh: 磁盘态
en: DiskState
forbidden: 文件内容, 原始内容, 磁盘内容, 字节态, 物理态
-->
文件在磁盘文件系统上的原始字节内容。唯一持有者：文件系统，仅 save_atomic / create_doc / rename_doc 三个入口可修改。已有 LogicalState 转化为 DiskState 的唯一路径是保存动作；无 LogicalState 的 PendingDiff 仅按 BR-CORE-STATE-002 在接受后原子落盘（§1.1）。

<!-- TERM
term_id: TERM-CORE-002
chains: IPC-CALL, ED-EDIT, ED-DECOR, DE-DIFF, WS-FS, WS-SAVE
zh: 逻辑态
en: LogicalState
forbidden: 编辑状态, 内存状态, 缓存状态, 运行态, 编辑缓存, 草稿内容
-->
编辑器加载磁盘态后建立的内存结构化模型与持久化控制态，是"真实编辑内容"的权威层。唯一持有者：ProseMirror doc（权威真源），edStore.EditSession.content 为派生缓存。保存动作将其写回磁盘态（§1.2）。

<!-- TERM
term_id: TERM-CORE-003
chains: IPC-CALL, ED-DECOR, ED-EDIT
zh: 显示态
en: DisplayState
forbidden: 渲染态, 呈现层, 装饰态, 可视化状态, 界面态, 屏幕态
-->
编辑器实际渲染出来的内容。数据源 = 逻辑态 + 纯装饰性叠加（DecorationSet）。显示态永远不能反向修改逻辑态。持有者：ProseMirror DecorationSet（§1.3）。

<!-- TERM
term_id: TERM-CORE-004
chains: DE-DIFF, AG-TURN, AG-ASSIST, CBT-BUILD
zh: 逻辑态候选
en: LogicalCandidate
forbidden: AI建议, 修改提议, 待定编辑, 提案态, 候选内容, 草案内容
-->
AI 产出在进入受控路径前的内容分类。存在 LogicalState 时，受控预览或 pending 后打开迁移使其进入 LogicalState，接受仅裁决；不存在 LogicalState 时，它保留为 DiffCard 临时挂载的内容，接受后直接原子写入 DiskState。deStore 是逻辑候选态的前端投影层（§2.1、X-FORBID-009）。

---

## §4 术语映射表

| 旧概念/载体 | 所属新术语 | 说明 |
|---|---|---|
| 磁盘文件内容（file on disk） | `DiskState` §1.1 | 唯一持有者：文件系统 |
| `save_atomic()` / `create_doc()` / `rename_doc()` | `DiskState` 写入口 | 仅此三处允许物理写盘 |
| `save_with_baseline_guard()` | `DiskState` 保存前 guard | 比对当前磁盘哈希与加载基线；冲突时不进入 `save_atomic()`，避免静默覆盖 |
| TipTap ProseMirror `doc`（`editor.state.doc`） | `LogicalState` 权威真源 §1.2 | 逻辑态的唯一权威形态，任何人取内容真值必须从此取 |
| `edStore.EditSession.content` | `LogicalState` 序列化持有者 §1.2 | 激活文档为 ProseMirror `doc` 的派生缓存；已打开非激活文档为当前 LogicalState 的序列化持有者，必须携带 session 版本与内容哈希 |
| `ed_session_snapshots` | `LogicalState` 持久化镜像 §1.2 | 已打开文档 LogicalState 的内部持久化镜像，用于恢复、AG 轮次基线校验和防止非激活文档自动应用后丢失 |
| `ed_open_tabs` | `LogicalState` 控制态元数据 §1.2 | 记录关闭前打开的文档集合、顺序和当前文档；不保存正文内容。启动恢复以它限定打开集合，再恢复对应 `ed_session_snapshots` |
| `applyEditOps()` / `applyStyleOps()` / ED session apply | `LogicalState` 受控修改入口 | AI 草稿应用、已打开非激活文档自动应用、reject 回滚、显式局部修改都经受控入口修改 LogicalState |
| `pending_diffs` 行 status=pending（+ `DiffPayload`） | `LogicalCandidate` §3 | 持久化的未裁决 diff；已有 LogicalState 时可被预览投影，无 LogicalState 时只由 DiffCard 临时挂载 |
| `pending_diffs` 行 status=accepted | `LogicalState` 持久化控制态 §1.2 | 已完成裁决的记录，已失去候选语义 |
| `TabState` 枚举（`dirty/normal/saving/saved/pending_diff`） | `LogicalState` 的裁决态投影 | 编辑会话的逻辑态裁决投影（是否偏离落盘内容），不是显示态。由 edStore 管理，经 SM-WS-01 广播驱动 |
| `deStore`（`DiffView[]` + `DiffCardState`） | `LogicalCandidate` 前端投影 §3 | 逻辑候选态的前端投影层（X-FORBID-007），不是显示态。其变化通过 ED 受控接口驱动 ProseMirror doc 修改 |
| `DiffDecorationExtension`（GreenReview） | `DisplayState` §1.3 | 只读 `tr.doc`，输出 `DecorationSet`。不替换 DOM、不阻挡编辑（X-FORBID-005） |
| `PaginationExtension`（分页缝） | `DisplayState` §1.3 | 纯呈现层 decoration，不修改文档模型、不产生脏态（BR-ED-DATA-003） |
| `ResizableImage` 拖拽中 DOM 预览 | `DisplayState` 临时态 §1.3 | `ignoreMutation` 阻止 ProseMirror 回灌；commit 时经受控 command 落属性转逻辑态 |
| AI 生成的 diff（未转正） | `LogicalCandidate` | 处于 `pending` 状态的 `PendingDiff`。已有 LogicalState 时由预览进入 LogicalState；无 LogicalState 时仅挂载于 DiffCard，接受后经后台分支写入 DiskState（X-FORBID-009） |
| `DiffPayload.after_content` | `LogicalCandidate` 内容载荷 | DE 持久化的 after 内容；已有 LogicalState 时经预览成为其内容，无 LogicalState 时由接受后的后台分支写入 DiskState |
| `edViewPrefsStore`（纸张/缩放） | 视图偏好（非三态） | 视图偏好（localStorage）不反映三态中任何一态的数据——它不是磁盘态（不是文件内容）、不是逻辑态（不编辑）、不是显示态（不叠加渲染）。单独归类为"视图层用户偏好" |
| `outlineStore`（大纲树） | `LogicalState` 派生投影 | 从 ProseMirror doc 提取的标题树，是逻辑态的只读投影。仅用于导航，不容许反向写回逻辑态 |
| `TabStrip` 标签栏上的脏点（`tab-dot`） | `LogicalState` 裁决态投影 → `DisplayState` 视觉标记 | 数据源是 `edStore.tabState`（逻辑态裁决态投影），渲染为红点（显示态）。显示态不反向修改 tabState |

---

## §5 D2~D5 决议落地

### D2（deStore 归属）

**决议**：deStore 是**逻辑候选态（LogicalCandidate）的前端投影层**（X-FORBID-007），不是显示态。

- 其所持 `DiffView[]` 中的 state 来自后端 SM-DE-01 广播（逻辑候选的裁决态）
- 其变化通过 Editor.tsx L281-331 的 `useEffect`（[Editor.tsx:L281-331](file:///Users/imatstarbucks/binder/src/components/ed/Editor.tsx#L281-L331)）触发 ED 受控接口操作 ProseMirror doc ——这是"逻辑候选态变化 → 经受控接口修改逻辑态"的合法链路，不是"显示态变化触发逻辑态"
- 该链路的每一步状态归属已在 Editor.tsx 对应代码区块注释中标注

**代码落地**：deStore.ts 顶部文件注释补充三态文档引用与归属声明（见 §7）。

### D3（逻辑态双副本同步契约）

**决议已体现在 §1.2**：ProseMirror doc 为权威真源，`edStore.EditSession.content` 为派生缓存。

- `serialize()` 是只读导出，`setContent()` 是外部数据加载人逻辑态
- 信息丢失风险按格式分级声明：text/md 低风险，html/html_file 中等风险（已知债务，非待修复缺陷）
- 真值需求一律从 `editor.state.doc` 取

**代码落地**：
- [edStore.ts](file:///Users/imatstarbucks/binder/src/store/edStore.ts) `EditSession.content` 注释补充三态标注
- [Editor.tsx](file:///Users/imatstarbucks/binder/src/components/ed/Editor.tsx) `serialize()` / `setContent()` 调用处补充三态标注

### D4（脏态判定有遗漏）

**决议**：这是一个真实缺陷（非"模糊"），需要修复。

**根因**：`markDraftApplied` 只记录了 `draftBaseTabState`（快照当时的完整 TabState 字符串），而不是记录"diff 应用前用户是否真实编辑过"的布尔脏位。当用户按 编辑(dirty) → AI 生成 diff(pending_diff) → reject 的顺序操作时，`draftBaseTabState` 捕获的是 `pending_diff` 前那一刻的状态（可能被其他操作覆盖），reject 后恢复的 TabState 可能丢失原始 dirty 标记。

**修复方案**：在 `DiffView` 新增 `draftBaseDirty: boolean` 字段，记录 diff 应用前 edStore 对应路径的 tabState 是否为 dirty（布尔值）。裁决完成（accepted/rejected）后，若 `draftBaseDirty === true` 则恢复为 dirty。

**代码落地见 §7**。

### D5（AI diff 双路径缺乏统一说明）

**决议**：以 `BR-CORE-STATE-001~003` 注册三条独立规则，并在 DE-DIFF、ED-EDIT、WS-SAVE/
WS-DIFF-APPLY 链路共同引用；路径选择不得再由单一 `draft_mode` 文案隐含。

**路径定义**：
- **已有 LogicalState 路径（BR-CORE-STATE-001）**：PendingDiff → 受控预览 → LogicalState（内容进入
  激活 ProseMirror doc 或已打开非激活 ED session），不触发保存。用户可显式接受后再保存；也可直接 ⌘S，
  此时保存把仍有效 pending 卡先裁决为 Accepted/awaiting_save，再写入 DiskState。
- **无 LogicalState 路径（BR-CORE-STATE-002）**：PendingDiff 仅挂载在 DiffCard，不产生 LogicalState
  或 DiskState；用户接受才直接触发 SM-WS-02 apply → DiskState。
- **pending 后打开路径（BR-CORE-STATE-003）**：背景 PendingDiff → 打开目标文档 → 基线校验 → 同一 payload
  受控预览进入新建 LogicalState（绿色增量、卡片仍 pending）→ 回到已有 LogicalState 路径。它不是自动
  接受，也不再具备后台直接落盘资格。

**判断规则表已追加至 A-DE-M-S-01 §5**（见 §7 设计文档更新）。

---

## §6 各跨态 IPC payload 字段标注

以下 IPC command 的 payload 中包含"内容"类字段，按三态体系标注其归属：

| IPC command | 字段 | 三态归属 | 说明 |
|---|---|---|---|
| `ws.read_doc` | `content` | `DiskState` 读取结果 → `LogicalState` 初始值 | 读取磁盘内容到逻辑层，response.content 是加载人逻辑态的材料 |
| `ws.read_doc` | `baseline` (content_hash) | `DiskState` 快照指纹 | 加载时磁盘内容的哈希，用于保存前校验；IPC 以十进制字符串承载 FNV-1a 64 |
| `ws.save_doc` | `content` | `LogicalState` 序列化载荷 | 前端逻辑态序列化结果，后端接收后写回磁盘转为新 DiskState |
| `ws.save_doc` | `baseline` | `DiskState` 校验指纹 | 用于 SM-WS-01 的加载基线校验；磁盘哈希不一致时返回保存冲突 |
| `ws.save_doc` | `override` | 用户裁决信号（非三态数据） | `true` 表示用户显式确认覆盖外部修改；无确认不得置 `true` |
| `ws.apply_diff` | `after_content` | `DiffPayload` 的候选内容 | 仅 BR-CORE-STATE-002 的无 LogicalState 分支在用户接受后消费；它不先转为 LogicalState，而是在同一原子操作中写成新 DiskState；可带加载基线兜底校验 |
| `de.generate_diff` | `afterContent` | `LogicalCandidate` 候选内容 | AG 产出的候选内容，持久化为 DiffPayload。裁决前不是 LogicalState |
| `ag_start_turn` | `open_doc_snapshots` | 已打开文档 `LogicalState` 快照 | 前端在发起 AgentTurn 前采集所有已打开文档的当前 LogicalState；AG 只读工具和 ReferencePayload 校验必须优先使用该集合 |
| `DiffProjection` | `snapshot_source`, `base_session_version`, `base_content_hash` | `LogicalState` 基线元数据 | 标识候选内容基于 active/open/disk 哪个源生成；`active_editor_session` 表示当前文档在 ProseMirror binding 未就绪时读取 ED session LogicalState；已打开文档自动应用必须用这些字段校验基线 |
| `de.accept_diff` | `diff_id` | — | 标识符字段，不适用三态归属 |
| `doc.open_for_edit` | `html`, `baseline` | `DiskState` → `LogicalState` 初始值 | docx 原始字节 → DOC 转换 → HTML 进逻辑态 |
| `doc.save_from_edit` | `html`, `baseline`, `override` | `LogicalState` → `DiskState` | 编辑链 HTML 序列化为 docx 字节，走同一保存前 guard 与原子落盘 |

> 以上标注为最低契约。各模块 L2 在新增 IPC payload 字段时，若包含"内容"类数据，须在本表追加并标注三态归属。

---

## §7 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-07-10 | v1.0 | 初版：三态模型跨模块权威定义（§1 三态定义与单向数据流契约）；X-FORBID-008/009 全局规则（§2）；TERM-CORE-001/002/003/004 术语注册（§3）；术语映射表全覆盖（§4）；D2~D5 决议与落地（§5）；IPC payload 标注（§6） |
| 2026-07-12 | v1.1 | 实现回写：补充保存前 baseline guard、`WS_SAVE_CONFLICT` 与 `override` 显式覆盖语义；声明 baseline IPC 字符串化；补 `applyEditOps`/`applyStyleOps` 作为 LogicalState 受控入口；更新跨态 IPC payload 标注 |
| 2026-07-18 | v1.2 | 补充已打开文档 LogicalState 门禁：AG/DE/ED 必须使用 open doc snapshots，已打开非激活文档 diff 走基线校验后的自动应用事务，禁止退回 DiskState 或旧快照 |
| 2026-07-18 | v1.3 | 细化 active 文档读取协议：`active_editor` 为实时 ProseMirror 快照，`active_editor_session` 为当前文档 ED session LogicalState 快照；二者均属 LogicalState，不依赖 DisplayState 反向定义 |
| 2026-07-18 | v1.4 | 补充 `ed_open_tabs` 三态归属：打开集合、顺序和当前文档属于 LogicalState 控制态元数据；启动恢复以其限定打开文档集合 |
| 2026-07-20 | v1.5 | 承接 APP-CLOSE-DIFF-LIFECYCLE-01：补充异常退出后的可信度边界——持久化 LogicalState/pending diff 只在上次为正常关闭时可信，异常退出必须先经启动清算（BR-SYS-STATE-005）才能进入恢复 |
| 2026-08-13 | v1.6 | DIFF-STATE-COMPASS-01：新增 BR-CORE-STATE-001~003 并注册到 DE-DIFF、ED-EDIT、WS-SAVE/WS-DIFF-APPLY；D5、IPC 载荷与单向约束同步为三分流：已有 LogicalState 预览后保存、无 LogicalState 挂载后接受原子落盘、pending 打开后迁移为预览。 |
| 2026-08-13 | v1.7 | 人类复核：保存仍有效 pending 逻辑预览即表达接受；保存编排先裁决为 accepted/awaiting_save 再写盘。若预览基线已推进，旧卡 expired，保存只提交当前 LogicalState。 |
