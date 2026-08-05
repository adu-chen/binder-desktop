---
文档编号：ED-M-T-01
文档状态：A
负责模块：ED
文档职责：ED 编辑器模块级 L2 技术设计（规则 SSOT）——富文本编辑运行时（TipTap）与编辑会话、block 模型与稳定 blockId 定位、md/docx 统一编辑模型、分页与图文媒体的呈现层纯化、编辑器装饰层（decoration）接口契约；承接 REQ-ED-001~009
上游约束：A-ED-M-D-01_编辑器功能需求描述.md（REQ-ED-001~009、ED-01~06）、A-CORE-C-P-01_APC.md（§4.1 ED 职责、§6 状态机阈值、§9 文件系统/离线边界）、A-SYS-C-T-01_系统总体架构.md（§4 ED→WS、X-SEQ-002）、A-SYS-I-P-01_数据结构定义.md（DocRef、X-SHARED-001）、A-SYS-I-P-02_IPC协议.md（统一错误形状 BR-SYS-OBS-002）、A-WS-M-T-01_工作台技术设计.md（BR-WS-DATA-001、BR-WS-PERSIST-001、X-FORBID-004）、A-WS-M-S-01_工作台状态机设计.md（SM-WS-01）、ADU.md（§3 RULE/CHAIN/CONSTRAINT/TERM 格式）
直接承接：本文 RULE/CHAIN/CONSTRAINT/TERM 为 ED 规则 SSOT，代码经 @GOV 映射；文档编辑会话状态机在 A-ED-M-S-01 定义并登记 A-SYS-M-S-01 总图；decoration 接口契约供 DE（diff 呈现）/ AG（局部修改呈现）G2 承接
使用边界：本文是 ED 规则 SSOT；不定义 diff 生成与终态（归 DE）、不定义 DOCX/Pandoc 二进制转换（归 DOC）、不做文件 I/O 与落盘（归 WS，ED 经 WS）、不锁定 TipTap/ProseMirror 具体版本与扩展集（延实现期选型记录）、状态机内部归 A-ED-M-S-01；inline-assist/autocomplete 业务归 AG（2026-06-11 人类确认），本文定义草稿编辑接口与 decoration 承载面
变更要求：blockId 稳定性语义变更须与 DE（diff 定位）联合复核；decoration 接口契约变更须复核 DE/AG 消费方；跨模块共享结构变更回写 A-SYS-I-P-01（X-SHARED-001）；TERM 一经被 term_ref 引用其 en 不可更改（ADU §3.0）
治理成熟度：draft
---

# Binder 编辑器技术设计（ED · L2 技术设计）

## 0. Terminology Registry

> ED 主责核心业务概念首批注册（跨 session 易漂移 + 代码有实体承载 + 命名混乱致规则误判），
> 承接 A-ED-M-D-01 §7 术语说明。如人类对命名有异议，可在本区域调整后重跑 generate。

<!-- TERM
term_id: TERM-ED-001
chains: ED-EDIT
zh: 内容块
en: Block
forbidden: 内容节点, 段落块, 文本块, 块节点, chunk
-->
ED 编辑模型的结构化组成单元，每个内容块具备稳定 blockId，是 diff 定位（DE）、@mention 引用锚
（REF）与分页切分的统一定位基准（REQ-ED-002）；普通编辑不改变既有 blockId，删除即失效不复用。

<!-- TERM
term_id: TERM-ED-002
chains: ED-DECOR
zh: 编辑器装饰层
en: EditorDecoration
forbidden: 高亮层, 标注层, 覆盖层, overlay层, 装饰器层
-->
ED 在编辑视图上承载非正文呈现（diff 高亮/审阅控件、局部修改提示）的呈现层；按 blockId 定位挂载，
按消费方（DE/AG）信号呈现与复位，自身不修改文档模型与落盘内容（REQ-ED-005）。

<!-- TERM
term_id: TERM-ED-004
chains: ED-EDIT
zh: 字符锚
en: CharAnchor
forbidden: 字符偏移锚, 文本锚, 偏移量锚, inline锚, 字符定位符
-->
会话内字符级内容锚：blockId + 块内纯文本偏移 {start,end}（偏移定义在块内纯文本平面，跨格式统一）。
锚有效期为编辑会话内（重开必换锚，PI-ED-09 实测锁定）；是 agent 精确编辑与样式操作的定位基准
（REQ-ED-006，BR-ED-DATA-004）。

<!-- TERM
term_id: TERM-ED-005
chains: ED-EDIT
zh: 选区引用
en: SelectionRef
forbidden: 选区信息, 拖拽引用, selection数据, 选区数据
-->
编辑器选区的可传递引用：{path, blockId, start, end, text}（含字符锚 + 选区文本）。经受控查询接口
按需读取；选区拖拽入 AG 输入面时作为位置引用载荷（REQ-ED-006），跨模块传递按 REF 注入协议承接。

<!-- TERM
term_id: TERM-ED-006
chains: ED-EDIT
zh: 样式操作
en: StyleOp
forbidden: 样式指令, 格式操作, style命令, 样式patch
-->
受控样式修改操作单元：set_mark / unset_mark / set_block_attr，目标为字符锚区间或 blockId。
agent 样式修改的唯一操作形态（REQ-ED-007，BR-ED-DATA-005）；落盘路径按裁定 D-3 分流
（回合产出经 DE diff 裁决、显式内联视为用户编辑）。

<!-- TERM
term_id: TERM-ED-007
chains: ED-EDIT, AG-TOOL, DE-DIFF
zh: 表格操作
en: TableOperation
forbidden: 表格指令, 表格补丁, table命令
-->
针对稳定 `tableId` 与表格指纹执行的受限结构/属性修改单元。人工表格工具和代理
`table_ops` 共用同一 ProseMirror 规划器；代理产出仍须进入 DE Diff 裁决，不得提交原始 HTML。

<!-- TERM
term_id: TERM-ED-008
chains: ED-EDIT, AG-TOOL, DE-DIFF, DOC-CONVERT
zh: 图片操作
en: ImageOperation
forbidden: 图片指令, 图片补丁, image命令
-->
针对当前 BDocx 图片实例或稳定 Block 执行的受限图片修改单元。代理只可执行项目内图片的独立段落
插入与文档图片节点删除，并须进入 DE Diff 裁决；项目源文件不属于删除目标。

## §1 文档定位

本文是 **ED 模块**的 L2 技术设计，是 ED 规则的唯一来源（SSOT，APC §2/§3.2），承接 ED L1
（REQ-ED-001~009）：

```
REQ-CORE-002/004/009 → REQ-ED-001~009（ED L1）→ 本文 RULE/CHAIN/CONSTRAINT/TERM（SSOT）→ 代码(@GOV)
本文 ←引用← A-SYS-I-P-01（DocRef）、A-WS-M-T-01（文件 I/O 经 WS）、A-WS-M-S-01（SM-WS-01 save）
本文 →驱动→ A-ED-M-S-01（文档编辑会话状态机 SM-ED-01）→登记→ A-SYS-M-S-01 总图
本文 decoration 契约 →供→ DE G2（diff 呈现）/ AG G2（局部修改呈现）承接
```

边界（承 ED L1 §6）：

- 本文定义 ED **规则/链路/约束/术语**；编辑会话状态机内部归 A-ED-M-S-01。
- **TipTap/ProseMirror 具体版本、扩展集与自定义 schema 不在本文锁定**（ED-01）：本文以运行时无关的
  契约描述编辑模型语义，框架选型延实现期并补选型记录回填（§10）；APC §4.1 已锁"基于 TipTap"的
  运行时方向，本文不重议。
- diff 终态归 DE、二进制转换归 DOC、落盘归 WS；inline-assist/autocomplete 业务归 AG
  （2026-06-11 人类确认，A-SYS-M-S-01 §3），本文仅定 decoration 承载面。

## §2 技术分层与边界

```
ED 编辑运行时（本文 SSOT）
  ├─ 编辑会话与脏态（§3）        加载（经 WS）→编辑→脏态（驱动 SM-WS-01）→save（经 WS）
  ├─ block 模型与 blockId（§4）   结构化内容块 + 稳定定位（供 DE/REF/分页）
  ├─ md/docx 统一编辑模型（§5）   docx 经 DOC 转换进出，ED 无二进制处理
  ├─ 分页与图文媒体（§6）         呈现层纯化：布局不改模型；媒体资源经 WS 限界
  └─ 编辑器装饰层（§7）           decoration 接口契约（DE/AG 消费）
```

- ED 在 X-SEQ-002 分层下运行：编辑器（前端）经 Service→注册 command 触达 WS/Rust，**ED 无直接
  文件 I/O 路径**；目标路径受 X-FORBID-004 守护。
- 错误统一走 A-SYS-I-P-02 统一错误形状（BR-SYS-OBS-002）。
- 编辑功能离线可用（REQ-CORE-009）：编辑/分页/媒体呈现不依赖在线鉴权；AI 相关呈现（decoration 的
  局部修改内容）的产生归 AG（须过订阅门禁），ED 只呈现。

## §3 编辑会话与脏态（ED-EDIT）

编辑会话生命周期：打开（经 WS 读取内容→解析装载为编辑模型）→编辑（更新模型、产生脏态）→save
（经 WS SM-WS-01 落盘）→关闭（脏态须先保存或经用户确认丢弃）。解析失败降级只读（不建立可编辑
会话，不静默丢内容）；读取失败回统一错误形状，会话不建立。状态机为 SM-ED-01（A-ED-M-S-01）。

### §3.1 编辑器级操作与格式适配（ED-EDITOR-OPERATIONS-01）

两行工具栏的第一行承载编辑器级操作：保存、撤销、重做、当前文档查找替换、定位、缩放、Markdown
源码/富文本切换与导出；文档内容格式操作仍位于第二行。所有替换必须经当前 ProseMirror 会话事务执行，
使其与键入相同地进入脏态和既有 save 路径，不能直接改 DiskState。

定位语义按编辑面区分：纯文本与 Markdown 源码按真实换行定位；富文本没有稳定的视觉折行，因此按
内容块（段落、标题、列表项等）定位，界面必须显式使用“段落”而非暗示视觉行。BDocx 的页码来自
BR-ED-DATA-003 已有的纯呈现分页缝，可用于选择、前后翻页和滚动定位，但不得写入 LogicalState 或
改变保存结果。

导出不属于编辑命令：工具栏只向 PRV 已支持的 `prv.export` 发起当前已保存源文件的格式转换。文本和
Markdown 可导出 DOCX；BDocx 可导出 PDF 或 Markdown；无支持组合时不展示入口。**编辑态不提供打印**；
打印只在 PRV 只读预览成功后触发，避免把编辑应用壳、未保存内容或未收敛分页作为打印产物。

### §3.2 文档工具增强与对象上下文（ED-DOCUMENT-TOOL-ENRICHMENT-01）

第二行的文字分组在 BDocx 与 HTML 中提供文字颜色、删除线、高亮、上标和下标；不再单独建立颜色分组。
高亮可选择预设色板或 HTML 颜色选择器的自定义不透明色，并能清除已有高亮。它们都属于编辑器扩展能力；
但 BDocx 的删除线、高亮、上标和下标当前不构成转换保真承诺，具体等级以工具栏兼容性清单为准。Markdown 不展示
该组能力，避免其写盘后静默丢失。列表分组只允许当前 `listItem` 升降层级；插入分组可插入水平分隔线，
但其是文档节点，不得被解释为分页、页面设置或输出指令。

图片和表格继续使用选中对象附近的临时工具组。独立图片段落可在 left/center/right 间切换，并只修改
承载图片段落的 `textAlign`；行内图片不提供此入口。表格合并要求当前矩形单元格选择，拆分只对已合并
单元格启用，二者复用现有表格 capability 与 ProseMirror 命令。图片本地替换必须走受控导入、媒体身份
和加载基线校验；在没有可复用入口前不得以文件路径、Data URL 或直接 `src` 替换的方式伪造此能力。

当前工具栏的完整功能矩阵、格式适配和转换风险见
`A-ED-X-R-03_编辑器工具栏功能与兼容性清单.md`。其中 capability manifest 标为 `unsupported` 的入口只
能视为会话内能力，不构成 BDocx 保存、重开或导出保真承诺；后续应以该清单的风险台账决定隐藏、警告或补齐
转换链。

<!-- CHAIN
chain_id: ED-EDIT
主责模块: ED
status: active
risk_tier: P1
-->

<!-- RULE
rule_id: BR-ED-STATE-001
主链路: ED-EDIT
域: STATE
-->
文档编辑会话必须经完整生命周期管理：内容加载与 save 落盘全部经 WS（ED 无直接文件 I/O 路径，
X-SEQ-002/X-FORBID-004）；用户编辑或 DE 草稿 diff 应用都属于编辑操作，必须产生脏态并驱动 WS save 保存机（SM-WS-01 Idle→Dirty，X-INTER-001
转移点）；已打开但非激活文档的 DE 自动应用属于同等编辑操作，必须经 ED session 受控事务修改
`EditSession.content`、递增 session 版本、刷新内容哈希并持久化 `ed_session_snapshots`；有脏态的会话不得静默关闭或丢失，关闭前须保存或经用户显式确认丢弃；内容解析失败降级只读
而非静默丢内容；编辑功能离线可用，不依赖在线鉴权。状态转移归 SM-ED-01（A-ED-M-S-01）。

<!-- RULE
rule_id: BR-ED-STATE-003
主链路: ED-EDIT
域: STATE
-->
编辑会话必须处理外部变更（订阅 WS 外部变更事件，BR-WS-OBS-001，2026-06-12 缺口补遗）：
**无脏态**（Ready）收到外部变更→自动重载磁盘内容（重载后 blockId 按持久化/重建规则恢复，并给
非阻断提示）；**有脏态**（Editing）收到外部变更或保存冲突（SM-WS-01 saveConflict）→进入冲突
提示，由用户**显式**选择「保留我的编辑（后续保存须经覆盖确认，对接 overrideConfirmed）」或
「放弃我的编辑并重载」——不得静默覆盖外部修改、不得静默重载丢弃脏态，两个方向的内容损失都必须
经用户显式选择。状态转移归 SM-ED-01 的 Conflicted 态（A-ED-M-S-01）。

> **实现回写（2026-07-12）**：已落地保存冲突下限保护——`saveDocument` / `saveDocxDocument`
> 携带加载基线调用 WS/DOC 保存，收到 `WS_SAVE_CONFLICT` 后保持当前会话 dirty 并返回 `conflict`
> 信号；编辑器只在用户显式确认覆盖后以 `override=true` 重试保存。文件监视/`ws.external_change`
> 推送与无脏态自动重载仍按 WS SY-02 延后，不属于本轮已落地能力。

> **已打开文档 LogicalState 门禁（2026-07-18）**：激活文档正常态的唯一真源是已绑定
> `activePath/loadSeq/editFormat` 的 ProseMirror `doc` 实时序列化；标签切换过渡期若目标活动文档
> 的 `EditSession.content` 已存在，则该 session content 是待装载的活动 LogicalState，可供 AG
> 读取，不等待显示渲染；若目标 session 尚未载入则短暂等待，超时阻断。已打开非激活文档的唯一真源是
> `EditSession.content` 加 `sessionVersion/contentHash`。AG/DE 读取已打开文档时，ED 必须提供
> OpenDocSnapshot；已绑定 ProseMirror 无法序列化或缺少版本/哈希时阻断上游，不允许读取 DiskState 或
> `loadedContent` 旧快照。`ed_session_snapshots` 是该 LogicalState 的内部持久化镜像，不是用户文档写盘。
> `ed_open_tabs` 是编辑器打开集合、顺序与当前文档的控制态真源；启动恢复先以 `ed_open_tabs` 恢复标签集合，
> 再只对这些路径恢复 `ed_session_snapshots`。不存在 `ed_open_tabs` 的孤立 session 镜像不得恢复为打开文档；
> 恢复完成后当前文档必须同步为 ED active session，使 AG 轮次可直接读取其 LogicalState。
> workspace 切换/关闭和应用窗口关闭前必须先 flush 当前 `ed_open_tabs` 与 `ed_session_snapshots`；
> 正常编辑过程可做短防抖持久化，但退出/切换边界不得只依赖防抖。

## §4 block 模型与稳定 blockId（ED-EDIT）

编辑内容以**内容块（Block，TERM-ED-001）**结构化建模：

- 每个内容块在创建时获得 blockId（不透明标识，生成算法延实现期，§10）。
- **稳定性语义**：不删除该块的普通编辑（改文字、改样式、块内操作）不改变其 blockId；块被删除则其
  blockId 永久失效且不复用；块分裂/合并的 blockId 继承规则延实现期敲定并回填本文（§10，ED-02）。
- **失效可感知**：上层（DE/REF）经 blockId 定位失败时必须获得明确的定位失败信号，不得静默错位。

<!-- RULE
rule_id: BR-ED-DATA-001
主链路: ED-EDIT
域: DATA
-->
编辑内容必须以内容块（Block）结构化建模并赋稳定 blockId：不删除该块的普通编辑不改变 blockId；
块删除后其 blockId 永久失效且不复用；上层模块（DE diff 定位、REF 引用锚）经 blockId 定位失败时
必须收到明确失效信号，不得静默错位呈现。blockId 若跨模块传递，其共享结构经 SYS 数据结构契约层
定义（X-SHARED-001），ED 不私自定义跨进程传输结构。

### §4.1 字符级锚定与选区引用（PI-ED-10，承 REQ-ED-006）

块级锚之上提供**字符锚（CharAnchor，TERM-ED-004）**与**选区引用（SelectionRef，TERM-ED-005）**：
字符锚 = blockId + 块内纯文本偏移 {start,end}；**偏移定义在块内纯文本平面并按 UTF-16 code unit
计数**（JavaScript `String.length` / `textBetween` 语义，
跨 md/docx/html/纯文本统一，与序列化源码偏移无关——docx HTML 模型的标签偏移不作为锚定平面）。
光标/选区位置经**受控查询接口按需读取**（非常驻投影：选区高频变化，常驻投影引发重渲染风暴且
非业务态，X-FORBID-007 无涉）。

<!-- RULE
rule_id: BR-ED-DATA-004
主链路: ED-EDIT
域: DATA
-->
编辑器必须提供会话内字符级内容锚：以 blockId + 块内纯文本偏移 {start,end} 定位（偏移在块内纯文本
平面定义，跨格式统一）；光标与选区经受控查询接口按需读取（SelectionRef 含字符锚与选区文本）。
锚有效期为**编辑会话内**（保存不打断内存模型、锚持续有效；文档关闭重开后 blockId 全量重建、旧锚
失效——PI-ED-09 实测 docx 往返保留率 0%，裁定 D-2）；锚失效（块删除/偏移越界/重开）必须回明确
失效信号，不得静默错位；跨会话字符锚（内容哈希重建）为显式延后债务（§11.2）。字符锚/选区引用
跨模块传递结构经 SYS 契约层定义（X-SHARED-001）。

## §5 md / b-docx / html 统一编辑模型（ED-EDIT）

Markdown、b-docx 与 **.html**（裁定 D-1，REQ-ED-008）在 ED 内共用**同一结构化编辑模型与交互**：Markdown 经统一 Markdown Codec 解析
装载；原生 `.docx` 不进入 ED 编辑态，只走 DOC/PRV 预览链；b-docx 由 DOC 转换为统一编辑模型后进入
编辑，存盘时经 DOC 转换回 docx（DOCX 往返，REQ-CORE-004）；.html 为文本格式直接解析装载、
序列化 HTML 往返（读写经 WS 文本路径，**不经 DOC 二进制管线**，X-SEQ-004 无涉），且 .html 源可
落盘 `data-block-id`（blockId 跨保存稳定，锚定强于 md 不注入策略）。ED 不含 docx 二进制解析/生成代码。

> **格式与内容编码分离**：`editFormat` 决定 DiskState 的解析/保存管线（text/markdown/html/html_file）；
> `contentFormat` 决定 EditSession/OpenDocSnapshot 中 LogicalState 的序列化编码（text/html）。受支持
> Markdown 打开后以 `editFormat=markdown + contentFormat=html` 持有 ProseMirror 兼容 LogicalState；
> 非支持结构进入 `contentFormat=text` 源码保护路径。DiskState 原文不得与 LogicalState HTML 混用。

<!-- RULE
rule_id: BR-ED-DATA-002
主链路: ED-EDIT
域: DATA
-->
Markdown、b-docx 与 .html 必须共用同一结构化编辑模型（内容块模型，§4），纯文本继续使用 text 模型：Markdown 的 DiskState 必须经 Markdown Codec 进入 ProseMirror LogicalState，保存时从 LogicalState 经同一 Codec 序列化为 Markdown；b-docx 的 docx 字节解析与
生成全部归 DOC，ED 仅编辑 DOC 转换后的统一模型，ED 代码不得包含 docx 二进制处理路径；原生 docx
不得直接进入 ED 统一编辑模型，必须先由 DOC 转成 b-docx 或走 PRV 只读预览；.html 为文本格式经 WS
文本读写路径直接解析/序列化往返，不引入独立转换管线；docx 往返中编辑模型与样式子集的承载范围以
本文与 DOC L2 的联合契约为准（ED-04/ED-05 延 DOC G2 联合敲定，本规则先锁分工：转换归 DOC、
编辑归 ED）。`editFormat` 与 `contentFormat` 必须分离并经 SYS OpenDocSnapshot 契约共享，AG/DE/会话恢复不得按文件扩展名猜测 LogicalState 编码。

### §5.1 Markdown Codec 与安全往返（ED-MARKDOWN-RICH-ROUNDTRIP-01）

- 默认方言为 CommonMark + GFM 项目子集：H1-H6、粗体/斜体/删除线、有序/无序/任务列表、嵌套列表、引用、行内/围栏代码、链接、图片、表格、分隔线与转义。
- Front Matter、脚注、数学公式、Admonition、Markdown 注释及无法由当前 schema 表达的任意 HTML 不在 v1 无损承诺内；命中后进入源码保护路径。
- 未编辑会话不得触发写盘或规范化；编辑后允许规范化 Markdown，但必须通过支持子集的语义往返校验。
- 四空格或 Tab 按 CommonMark 代码块语义处理，不用改写解析规则掩盖合法语法。

<!-- RULE
rule_id: BR-ED-DATA-006
主链路: ED-EDIT
域: DATA
-->
Markdown 文档必须走独立 `markdown` 格式管线：DiskState 经统一 Markdown Codec 解析为 ProseMirror
LogicalState，DisplayState 以标题、列表、引用、代码、表格等终态样式呈现，保存时从 LogicalState 经
同一 Codec 序列化回 Markdown；`.txt` 不得进入该管线。受支持 Markdown 的 session/OpenDocSnapshot
必须声明 `contentFormat=html`，源码保护路径声明 `contentFormat=text`；调用方不得按扩展名或内容前缀
猜测编码。解析失败或命中非支持结构时必须保留原始源码并降级，不得构造部分文档后继续保存。

<!-- RULE
rule_id: BR-ED-VERIFY-001
主链路: ED-EDIT
域: VERIFY
-->
Markdown 保存必须满足无静默丢失：支持子集执行 parse→serialize→parse 语义等价验证；未编辑文档不得
写盘或因打开/关闭产生规范化差异；序列化、校验或能力检查失败时必须停止写盘、保持 dirty 与原 DiskState，
并返回可诊断错误。任何非支持节点、mark 或块属性不得被静默删除、转为普通文本后假报保存成功。

### §5.2 样式投影与样式操作协议（PI-ED-11，承 REQ-ED-007）

文档样式对 agent **可读可编辑**：

- **读侧（样式投影）**：块结构化投影为 `{blockId, type, text, marks:[{type, range:[start,end],
  attrs}], blockAttrs}`（marks range 在块内纯文本平面，与字符锚同基准）；经 AG 读接口
  （ag.list_blocks 扩展，A-AG-I-P-01）供 agent 消费。
- **写侧（样式操作，StyleOp，TERM-ED-006）**：set_mark / unset_mark / set_block_attr，目标为
  字符锚区间或 blockId；必经 ED 受控编辑接口（ed.apply_edit op 粒度表，A-ED-I-P-01 §4.1）。
- **当前实现入口（2026-07-12）**：文本/块/整篇内容修改统一经 `applyEditOps()` 进入 ProseMirror
  doc（`replace_text` / `replace_block` / `replace_document`）；样式批量修改经 `applyStyleOps()`
  规划并应用单事务。DE 草稿应用、reject 回滚、示例 diff 生成均以当前编辑器 LogicalState 为准，
  不以 `edStore.EditSession.content` 派生缓存作为真值。
- **已打开非激活文档入口（2026-07-18）**：`draft_mode=open_session` 的文本/块/整篇 diff 不要求用户切换窗口；
  ED 必须在 `EditSession.content` 上执行 session apply 事务，先校验 `base_session_version` 与
  `base_content_hash`，成功后写回 content、递增版本、刷新哈希、置脏并持久化 `ed_session_snapshots`。
  失败时不得修改 LogicalState，不得改走后台 DiskState apply。
- **落盘分流（裁定 D-3）**：chat/agent 回合产出的样式修改进 DiffPayload 经 DE diff 裁决；当前
  编辑器草稿路径在 pending 时先经 ED 受控编辑接口应用为草稿，accepted 只确认，save 经
  SM-ED-01→SM-WS-01 落盘；用户显式触发的局部修改样式操作视为用户编辑直改（BR-AG-STATE-003
  语义，产生脏态经 SM-ED-01→SM-WS-01）。
- **草稿恢复**：包含 StyleOp 的 pending diff 被拒绝时，ED 必须用草稿应用前的完整编辑器内容恢复，
  不得只回滚单块纯文本；恢复仍经当前编辑器受控接口触发普通脏态/标签态投影。
- **能力清单与防御性校验（2026-07-28）**：代理可用 StyleOp 的名称、attrs 和 DOCX 往返等级由
  A-SYS-I-P-04 BDocxCapabilityManifest 声明；AG 必须在生成 PendingDiff 前校验。ED 的
  `planStyleOps()` 仍须对不可信历史/IPC 载荷做批内原子防御：只接受真实 mark 名
  `bold/italic/underline/strike/highlight/subscript/superscript/code/link/textStyle`，严格校验
  mark attrs、textAlign 目标节点和字符范围；任何未知项整批拒绝，不得等到事务执行中途才发现。

- **富文档结构化内容块（2026-07-30）**：活动 ProseMirror doc 是层级块结构唯一真源。
  `projectAgentEditableBlocks()` 遍历所有稳定 blockId 节点，投影 node type、role、父子层级、node JSON、
  fingerprint 与动态 capabilities；列表/listItem/blockquote 等非 textblock 容器只读，其 paragraph
  子节点独立可写，表格祖先和图片承载块不开放通用写能力。`applyStructuredBlockOperations()` 先整批
  预检，再以一个 transaction 倒序 replaceWith，避免多个列表条目被拼成单段。Diff 拒绝使用 exact
  beforeNode/afterNode 反向事务；节点在提案后被人工修改时转 conflict，不使用全文恢复覆盖其他改动。

<!-- RULE
rule_id: BR-ED-DATA-005
主链路: ED-EDIT
域: DATA
-->
文档样式必须以机器可读结构投影（块级 marks/blockAttrs，range 在块内纯文本平面与字符锚同基准）
供 agent 读取；样式修改必须以样式操作（StyleOp：set_mark/unset_mark/set_block_attr，目标为字符锚
区间或 blockId）经 ED 受控编辑接口进入（X-FORBID-005），不得绕过接口直改编辑器模型。agent 样式
落盘路径分流（裁定 D-3）：chat/agent 回合产出经 DE diff 裁决；当前编辑器草稿路径由 ED 受控接口
先应用为草稿，accepted 后仍由普通 save 落盘（X-FORBID-006 不允许 AG 直接写文件），用户显式触发的
局部修改样式操作视为用户编辑直改并产生脏态；拒绝含 StyleOp 的草稿 diff 必须恢复草稿前完整内容，
不得存在绕过裁决的回合直写文件路径。样式操作必须经过格式能力检查：Markdown 仅允许目标方言可稳定
表达的 marks/blockAttrs；字体、字号、颜色、下划线、上下标、高亮、文本对齐等非标准能力在 Markdown
模式下必须禁用或返回明确不支持，不得允许编辑后保存静默丢失。已打开非激活文档的 StyleOp 自动应用需要无 ProseMirror runtime
事务；未具备时必须返回明确不支持并让 DiffCard 失效，不得降级为 DiskState 写入。

### §5.3 BDocx 表格操作协议（PI-ED-17）

BDocx 表格以表格操作（TableOperation，TERM-ED-007）表达受控结构与属性修改。操作目标由
`tableId + fingerprint` 锚定，坐标为 0 起逻辑行列；预览规划与实际应用必须共用同一执行器并整批原子
成功。支持集包括课题 A 的行列增删、表头、合并/拆分、纯文本单元格写入、文档流移动，以及课题 B 的
`set_cell_background`。

`set_cell_background` 只修改 `tableCell/tableHeader.backgroundColor`，不得改写单元格子树，因此富
文本、多段落和 inline atom 必须保持不变。颜色只接受不透明 `#RRGGBB` 或 `null`（清除）；合并单元格
只接受其左上锚。TableSnapshot 必须投影每个单元格的 `backgroundColor` 并把它纳入 fingerprint，使
底纹变化能够令旧建议过期。人工多单元格选择使用同一节点属性语义；代理只能使用 `list_tables`
公开的 capability。

人工 `move_table` 与删除表格入口采用当前活动表格邻近的临时按钮组，不在顶部工具组重复呈现。控件以当前
选区解析 tableId，并从选区 DOM 取得活动 table 所属的 TipTap `.tableWrapper`。控件须通过 portal
直接挂载到该 wrapper，并以 wrapper 内 absolute 布局固定在右上角；不得读取或换算 table、paper、
editor host、分页纸张、滚动内容或 viewport 的 top/left。TableView 的 `ignoreMutation` 明确忽略
wrapper 中 contentDOM 之外的 childList/attribute/characterData 变化，因此该外挂 UI 不进入
ProseMirror 文档模型。按钮采用右上方横向迷你工具条，wrapper 为所有表格保留一致的顶部安全区，工具
条不得覆盖表头或单元格，也不得因选区进入/离开导致表格上下跳动。选区离开表格即卸载；表格移动、
分页、缩放和滚动时由 DOM 归属自然跟随。
按钮只调用既有逐块上移/下移 TableOperation，
不可用方向禁用；`mousedown` 不得夺走编辑器选区，移动事务完成后须把选区恢复到同一 tableId，使
控件跟随移动后的表格。真实 TableView DOM 识别必须兼容 table 与 `.tableWrapper`，相同选区再次点击
也须通过 DOM 交互事件唤起控件；删除按钮调用既有 ED `deleteTable` 命令。该控件不引入拖动、任意
位置、浮动定位或嵌套表格跨容器移动。

<!-- RULE
rule_id: BR-ED-DATA-007
主链路: ED-EDIT, AG-TOOL, DE-DIFF
域: DATA
-->
BDocx 表格结构与受支持单元格属性必须以表格操作（TableOperation）进入 ED 受控编辑路径：
目标同时携带稳定 tableId 与 pre-update fingerprint，操作使用 0 起坐标并由同一 ProseMirror 规划器
完成预览与原子应用；任一操作、坐标、动态 capability 或基准不匹配时整批失败。单元格纯色底纹只允许
`set_cell_background(at,color)`，其中 color 为不透明 `#RRGGBB` 或 `null`，合并单元格只接受左上锚；
该操作只修改 tableCell/tableHeader 的 backgroundColor 属性，不得替换或降级富内容子树。

### §5.4 BDocx 图片操作协议

BDocx 图片代理修改以图片操作（ImageOperation，TERM-ED-008）表达。操作批次绑定当前
`sessionVersion + contentHash`；插入以稳定 Block 和 `before/after` 锚定，只生成包含一个图片的独立
顶层段落；删除以 `imageId + fingerprint` 锚定当前图片实例。预览与实际应用必须共用同一
ProseMirror 规划语义并整批原子成功。

`insert_image` 只消费 DOC 已物化的稳定媒体引用与 workspace 来源路径/内容哈希；可选 alt 与受限
正整数 width。`delete_image` 只删除 LogicalState 中的图片节点：独立图片删除其段落，唯一段落时保留
空段落，混排图片只删节点。两种操作都不得删除或改写项目源文件、接受原始 HTML、引入远程/data URL，
也不得扩展为浮动/绝对定位、任意坐标布局、裁剪、旋转或独立高度。非活动编辑器不具备图片节点事务，必须
明确拒绝而不是降级改写 DiskState。

## §6 分页与图文媒体（呈现层纯化，ED-EDIT）

- **分页**：分页布局（分页模式、光标与遮罩、打印友好呈现）是**呈现层**行为：由当前文档可映射的
  ProseMirror position 标识 Block/视觉行断点，不改变文档模型与落盘内容；分页开/关与重排不产生模型
  变更，不触发脏态。
- **普通段落视觉行拆分（PI-ED-16，2026-07-26）**：分页先测全部 Block 连续高度，只对当前页实际无法
  整块容纳的**顶层普通段落**按 grapheme 合法位置以指数探测+二分定位视觉行；自然视觉行必须在不含
  段内分页缝的当前可见 ProseMirror 真实排版中测量。已有段内断点时，插件须在同一个 rAF 回调内通过
  纯呈现事务临时过滤 `paragraph-line` widget，直接读取当前纸张宽度、字体、marks、文字方向与缩放下的
  Range 行盒，并在该回调返回前提交最终完整分页方案；不得再用克隆可见 DOM 或 schema `DOMSerializer`
  生成的离屏副本近似真实行宽。断点必须同时满足“后一个 grapheme 进入更低视觉行”与“水平方向回到
  行首侧”；仅纵坐标变化不足以证明换行。分页流采用贪心填页：
  当前页保留所有能够完整容纳的视觉行，只有一行都放不下时才换页，允许页底或页首出现单行片段；
  超整页行以保证前进为准。
  标题、`listItem`/TaskList、引用、代码、表格、图片/媒体，以及含 hardBreak 或 inline atom 的段落保持
  原子 Block；hardBreak 不得作为段内分页候选，因为它会形成未填满短行，且可能与旧 widget 邻近污染
  形态相同。
  Range/position、横向回行或文字方向几何任一不可靠时同样原子降级，不得猜测断点。
  行间断点使用 `span` widget，Block 断点使用 `div` widget，二者均为不可交互 Decoration，不得写入
  LogicalState、DiskState 或 history。
- **分页重测量稳定性**：完整物理分页是硬约束；分页启用期间不得撤下 Block 页缝、页码、纸张高度或末页
  补白，也不得把编辑器切换成无分页模式。为消除旧行间 widget 对自然换行的自证污染，唯一允许的测量
  例外是：捕获可见锚点后，在同一个 rAF 回调内以 `addToHistory=false` 的呈现事务暂时排除
  `paragraph-line` Decoration，保留所有 Block 页缝，立即读取可见 ProseMirror Range，并在回调返回前
  恢复完整方案及锚点；中间态不得跨帧、不得 `await`、不得修改 doc/selection/history/dirty。异常、重试或
  相同方案路径也必须恢复测量态。已有段内分页缝不得作为自身断点有效性的测量依据；重排后不再成立的
  断点必须随最终 Decoration 方案移除，使段落恢复自然连续排版。末页补白是独立 CSS 呈现值；仅该值变化时不得重建
  普通分页缝 DecorationSet。只有分页断点 position、类型或数量变化时才允许重建对应呈现装饰；非末页
  fill 与末页 fill 都只能更新稳定 DOM 的 CSS 变量，不属于 Decoration 身份。既有 Block/行间分页缝高度
  必须按 position 与 `side:-1` bias 从后续 Block 和行坐标扣除，不得用 Block 序号近似。
  重测量触发只能来自文档事务、纸张几何、字体/媒体加载或窗口尺寸等外部布局输入；不得用观察整个
  ProseMirror 高度的 ResizeObserver 反向触发重测量，因为分页缝和末页补白自身会改变该高度并构成正反馈。
- **打开收敛（2026-07-25）**：BDocx 打开期间，编辑器只接受活动路径的**最新**内容载入。workspace 启动恢复
  必须单飞；状态机监听启动必须单飞，且卸载时使尚未完成注册的一组监听失效；同一路径的旧异步打开结果必须
  丢弃。用户已切换到其他标签时，迟到结果只可补全其非活动 `EditSession`，不得改写 `activePath`、推进
  `loadSeq` 或触发当前 ProseMirror `setContent`。内容载入时，分页启用/几何重置必须经一次受控布局事务合并，
  不得以多个命令对同一新文档重复触发分页计划。
- **滚动控制权（2026-07-26）**：浏览器 Scroll Anchoring 只在 `.editor-paper` 滚动容器禁用，不得在
  ProseMirror DOM 写入内联 `overflow-anchor:none`，以保留 ProseMirror 的 `storeScrollPos/resetScrollPos`
  更新期保持。普通输入请求 `scrollIntoView` 时，分页模式只可预先接管**已经完整位于滚动容器 client viewport
  内**的折叠文本光标，阻止冗余滚动；真实越界、不可测量状态和非文本选区必须交还 ProseMirror 原生滚动。
  IME 未处于有效指针激活周期时同样交还原生滚动；若已有有效周期，则 composition 的 doc 变化可沿用相同的
  可见性判定阻止冗余滚动，但分页重测量仍须在 composition 期间暂停并于结束后静默结算。
  可见性只能用浏览器实际 Selection/DOM Range 与容器 client rect 的同一屏幕坐标判断，不得混入
  `scrollTop` 或对内容 CSS zoom 二次换算。若运行时不能提供可靠的绝对 Range，允许把用户在已揭示分页编辑器
  内完成的主键指针定位建立为光标激活周期：以可信的指针屏幕点为锚，只消费后续 Range 相对激活 Range 的位移，
  并按运行时检测的“computed zoom/rect实际比例”换算；估算光标仍可见且用户未改变 `scrollTop` 时可持续
  拦截后续 doc 变化滚动，不得按字符数硬编码。选择导航、真实越界、用户滚动、blur或新指针定位必须结束周期。
  周期只能在 ProseMirror 最终选区同步后的下一帧建立，双击、拖选与非折叠选区不得误建；文档身份、编辑格式、
  缩放、纸张或方向变化必须显式清除待建与现有周期。分页结构结算若主动补偿 `scrollTop`，必须以内部事件同步
  现有周期的滚动基线，不得把分页层写入误判为用户滚动；该事件不得携带或修改文档、选区、分页计划与脏态。
  搜索/大纲跳转与滚动恢复由显式导航命令负责；分页结算只允许在事务返回后校验可见锚点残差并至多补偿一次，
  不得反向补偿普通输入或显式导航。
- **输入期分页冻结（2026-07-26）**：文档内容事务只失效化分页计划；连续输入和 IME composition 期间保留
  当前 break 归属，且 position 以 `assoc=-1` 随文档事务同步映射；被删除或不再位于顶层普通段落内部的
  行间断点安全丢弃。最后一次内容变化静默 450ms 后统一测量并提交一次。首次装载、纸张几何、字体/媒体和
  窗口变化使用独立即时结算。输入期间允许分页呈现短暂滞后，但该状态不得写入模型、脏态或保存内容。
- **分页 DOM 稳定与精度（2026-07-26）**：分页缝 Decoration 身份由按页序确定的稳定 id 承载，结构相等
  由 position+断点类型判断；position 会随事务映射，不得写入 DOM dataset 充当身份。非末页 fill 与末页
  fill 均通过稳定 DOM 的 CSS 变量更新，fill 变化不得重建同一 widget。分页高度计算全程保留小数，只在
  相等判断和最终滚动写入使用亚像素容差，禁止逐页取整形成累计漂移。
- **首帧揭示（2026-07-26）**：BDocx 的 `loadedContent` 只表示转换结果到达；编辑器须继续保持加载遮罩，
  直到 `setContent`、当前 `loadSeq` 绑定和首轮分页 settled 均完成。隐藏期间 `EditorContent` 必须保持挂载
  以供真实 DOM 测量；旧 `loadSeq` 的 settled 结果不得揭示当前文档。
- **图文媒体**：图片等媒体以媒体内容块进入编辑模型；媒体资源文件经 WS 存取并限定 workspace 内
  （X-FORBID-004），ED 不自行落盘媒体文件。来源 BDocx 的已嵌入图片由 DOC 转换层抽取为按 baseline
  隔离的只读编辑媒体，不属于 ED 导入项目文件；ED 的 HTML 节点以 `data-binder-media-src` 保留规范
  相对身份，`src=asset://...` 仅为当前 WebView 显示投影。持久快照与保存输入必须恢复规范相对 `src`，
  重启后经 DOC 重新授权，禁止依赖进程内 URL Map。
- **图片代理操作**：代理插入的项目图片节点除稳定 `data-binder-media-src` 外，还保留 workspace
  相对来源与内容哈希，供未保存草稿在媒体缓存缺失时重新校验和物化；这些属性是草稿恢复元数据，不
  赋予 ED 文件写权限。代理必须先读取当前图片/Block 快照，再由 ImageOperation 单事务应用。DOC
  打开、恢复和保存结果按文档更新本进程媒体显示目录；ED 只用该本地映射把稳定 mediaRef 投影为
  `asset://`，本地绝对目录不得进入 ImageOperation 或模型工具结果。
- **图片人工交互**：图片宽度修改必须由 NodeView 手柄提交单个 ProseMirror 属性事务，DOM 预览不能
  取代 LogicalState；鼠标位移和 DOM 宽度按 computed zoom 归一化，WebKit 使用 document 捕获阶段
  `mousemove/mouseup`，空拖动不产生事务。只有顶层且段落仅含一个图片的媒体 Block 可与相邻顶层 Block
  交换位置，交换时保持段落 blockId、图片媒体身份和 NodeSelection；混排图片不提供块移动方向。
  删除独立图片时删除其段落，文档仅剩该段时保留空段落；混排图片只删除图片节点。
- **inline/block 边界**：真实 Pandoc 探针确认 `data-display` 在 HTML→DOCX→HTML 后丢失。BDocx 中
  行内图片与独立图片只由段落结构区分，不提供虚假的 inline/block 持久切换；旧 `data-display` 仅用于
  会话 HTML 显示兼容。浮动、绝对定位和任意位置拖放不在当前支持集。

<!-- RULE
rule_id: BR-ED-DATA-003
主链路: ED-EDIT
域: DATA
-->
分页布局与遮罩是纯呈现层行为，不得改变文档模型或落盘内容，分页重排不产生脏态；图片等媒体以媒体
内容块建模，其资源文件的存取必须经 WS 且限定 workspace 内（X-FORBID-004 / BR-WS-DATA-001），
ED 不得自行写入媒体文件。图片宽度、文档流位置与删除必须通过 ED 受控 LogicalState 事务修改并产生
脏态（BR-ED-STATE-001）；只有独立顶层图片段落可整体上移/下移，混排文字中的图片不得按块重排，
代理仅可通过图片操作（ImageOperation）把经 DOC 物化的 workspace 图片插入稳定 Block 前后，或按
稳定会话 imageId 与当前 fingerprint 删除、缩放、上下移动和设置独立顶层图片段落的左/中/右对齐；
每个图片快照必须给出动态 capabilities、resizeBounds 与 moveDirections。同一图片单批可各含至多
一个缩放、对齐和移动并按固定顺序在单一事务执行；重复同类操作拒绝，删除必须独占。批次基线、动态
能力、锚或来源哈希失效时必须整批拒绝，且不得删除或改写项目源文件。所有操作必须保持未删除图片的
稳定媒体身份。打印/导出管线归 PRV，ED 仅提供打印友好的分页呈现。

## §7 编辑器装饰层接口契约（ED-DECOR）

装饰层（EditorDecoration，TERM-ED-002）是 ED 供上层呈现非正文内容的唯一受控表面：

- **挂载**：消费方（DE diff 呈现、AG 局部修改呈现）以 blockId 定位请求挂载 decoration；锚失效回
  定位失败信号（BR-ED-DATA-001）。
- **呈现/复位**：decoration 按消费方信号呈现与复位（如 DE 终态 accepted/rejected/expired 后复位
  该 diff 的 decoration）；复位不残留视觉痕迹。
- **GreenReview 草稿语义**：对当前编辑器命中的块级 diff，ED 先把 after 内容替换进目标 Block；
  对显式 `replace_document` 整篇 diff，ED 先把 after 全文替换为编辑器草稿。未携带 blockId 且
  不是显式 `replace_document` / StyleOp 的 diff 必须投影为 error，不得自动整篇替换。decoration
  只在当前 after 内容上显示 GreenReview（绿色背景/绿增预览），不在正文中额外渲染红删预览；
  红删对比保留在 DiffCard。只有已成功应用为 ED 草稿的 diff 才能挂载 GreenReview；锚失效或
  error 卡不得残留绿色装饰。
- **DiffProjection 消费边界**：AG 产生的草稿 diff 必须先携带后端权威 `DiffProjection`，ED 只消费
  其中的 `patch_kind/block_id/block_index/before_text/after_text/style_ops/draft_mode/snapshot_source/base_session_version/base_content_hash`。前端不得从工具请求参数、
  当前整篇内容或 DiffCard 文案反向推断编辑器草稿。文本/md 文档缺 blockId 时可在当前编辑会话内用
  `block_index` 映射到实时 Block；BDocx 必须使用 DOC canonical HTML 中的 `data-block-id`，缺失或失效
  即 error，不得用整篇替换或重建锚作为隐式降级。
- **非激活打开文档呈现边界**：`draft_mode=open_session` 成功后内容已进入目标 session LogicalState；
  DiffCard 只显示已应用/不可裁决状态。目标文档被激活时，ED 不得再次应用该 diff，只能按当前
  LogicalState 渲染；GreenReview 是否补挂载属于呈现增强，不得影响内容状态。
- **纯呈现**：decoration 不修改文档模型与落盘内容；用户在 decoration 上的交互（接受/拒绝按钮等）
  转达给消费方裁决，ED 不代裁。
- **唯一通道**：上层不得绕过装饰层与编辑接口直接操纵编辑器内部文档状态（X-FORBID-005）。

<!-- CHAIN
chain_id: ED-DECOR
主责模块: ED
status: active
risk_tier: P1
-->

<!-- RULE
rule_id: BR-ED-STATE-002
主链路: ED-DECOR
域: STATE
-->
编辑器装饰层必须满足：按 blockId 定位挂载且锚失效回明确失败信号；整篇 content diff 可在草稿替换
完成后按当前编辑器块集合批量挂载 GreenReview；按消费方（DE/AG）信号呈现与复位，消费终态或保存后
不残留；decoration 自身不修改文档模型与落盘内容，当前编辑器草稿 diff 的正文替换必须走 ED-EDIT
受控接口，用户在 decoration 上的裁决交互转达消费方处理，ED 不代行裁决（diff 终态归 DE 终态机）。inline-assist/autocomplete 的业务语义
归 AG（2026-06-11 人类确认，A-SYS-M-S-01 §3），本规则只约束其呈现承载面。

## §8 跨模块约束（CONSTRAINT）

<!-- CONSTRAINT
constraint_id: X-FORBID-005
摘要: 上层模块（DE/AG/REF 等）不得绕过 ED 装饰层与编辑接口直接操纵编辑器内部文档状态；编辑器文档模型的修改必经 ED 受控接口
涉及模块: ED, DE, AG, REF
链路影响: ED-EDIT, ED-DECOR
-->

**复用既有约束，不重复定义**：

- **X-SEQ-002**（A-SYS-C-T-01）：ED 前端经 Service→command 分层触达后端，不跨层直连 FS/DB/网络。
- **X-FORBID-004**（A-WS-M-T-01）：ED 发起的内容/媒体读写目标路径不越出 workspace root。
- **X-SHARED-001**（A-SYS-I-P-01）：blockId 等跨模块传递结构经 SYS 契约层定义。
- **X-SEQ-003 / X-INTER-001**（A-SYS-M-S-01）：编辑会话为状态化核心链，SM-ED-01 先建机并登记总图；
  ED↔WS 脏态/保存转移点登记 X-INTER-001。

## §9 需求到规则映射（REQ_MAP）

<!-- REQ_MAP
req_id: REQ-ED-001
target_type: CHAIN
target_id: ED-EDIT
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-001
target_type: RULE
target_id: BR-ED-STATE-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-001
target_type: RULE
target_id: BR-ED-DATA-002
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-001
target_type: RULE
target_id: BR-ED-STATE-003
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-001
target_type: RULE
target_id: BR-ED-DATA-006
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-001
target_type: RULE
target_id: BR-ED-VERIFY-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-002
target_type: RULE
target_id: BR-ED-DATA-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-003
target_type: RULE
target_id: BR-ED-DATA-003
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-004
target_type: RULE
target_id: BR-ED-DATA-003
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-005
target_type: CHAIN
target_id: ED-DECOR
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-005
target_type: RULE
target_id: BR-ED-STATE-002
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-005
target_type: CONSTRAINT
target_id: X-FORBID-005
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-006
target_type: RULE
target_id: BR-ED-DATA-004
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-007
target_type: RULE
target_id: BR-ED-DATA-005
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-008
target_type: RULE
target_id: BR-ED-DATA-002
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-009
target_type: RULE
target_id: BR-ED-DATA-007
status: mapped
-->

## §10 测试与验收

- **会话生命周期**（BR-ED-STATE-001 / SM-ED-01，REQ-ED-001 验收）：md/docx 可打开/编辑/关闭；
  脏态关闭须保存或确认丢弃；解析失败降级只读；断网时编辑可用；ED 无直接文件 I/O 调用路径（静态可查）。
  已打开非激活文档自动应用 diff 后 session content/version/hash 更新、置脏并持久化，切回该文档时不重复应用；
  重启恢复继承关闭前打开的文档标签、顺序和当前文档，孤立 `ed_session_snapshots` 不恢复为打开文档，恢复后的当前文档可进入 AG 工作序列。
- **blockId 稳定**（BR-ED-DATA-001，REQ-ED-002 验收）：普通编辑后既有 blockId 不变；删除块后定位
  该 blockId 收到失效信号；不存在静默错位。
- **Markdown/b-docx 统一模型**（BR-ED-DATA-002/006、BR-ED-VERIFY-001）：Markdown 受支持语法渲染为结构化节点，支持子集语义往返；未编辑不改写、非支持结构不静默保存；docx 进入编辑与往返保存经 DOC
  转换；ED 代码无 docx 二进制处理（静态可查）。
- **呈现层纯化**（BR-ED-DATA-003，REQ-ED-003/004 验收）：分页开/关与重排不改文档模型、不产生脏态；
  媒体资源越界路径被拒（X-FORBID-004）；启动恢复、监听与异步打开并发时，当前活动编辑器只发生一次受控
  内容载入，旧结果不得回填或重新触发分页几何。普通段落行间拆分须覆盖 position 映射、最大可容纳行数、
  零行可容纳才换页、既有 gap position/bias 扣除和复杂节点原子降级。正式应用多轮人工测试后，人类接受
  当前普通段落仍可能行中强制断开、复杂节点仍原子换页的发布边界；该限制不改变分页纯呈现验收。
- **图片人工交互**（BR-ED-DATA-001/003、BR-ED-STATE-001，REQ-ED-004 验收）：真实 NodeView +
  document 捕获鼠标事件在 zoom 下可拖宽，空拖动不产生事务；独立图片段落可移动并保持 blockId、媒体
  属性和选中态；混排图片移动被拒且同段文字不变；删除后的 LogicalState、HTML 序列化和 BDocx
  保存重开图片数量/顺序一致。`data-display` 丢失测试作为明确排除证据，不得转写为已支持。
- **图片代理操作**（BR-ED-DATA-001/003、BR-ED-STATE-001，REQ-ED-004 验收）：ImageOperation
  必须校验当前 session/content 基线、Block/image fingerprint 和项目来源哈希；插入只形成稳定 Block
  前后的独立图片段落；删除、缩放、上下移动和 left/center/right 对齐服从稳定 imageId、当前
  fingerprint 与动态能力，删除不触碰项目源文件；整批原子应用、拒绝时恢复草稿前全文，保存重开后的
  图片实例、资源、alt、受限宽度、文档流位置和受支持段落对齐一致。
- **装饰层契约**（BR-ED-STATE-002 / X-FORBID-005，REQ-ED-005 验收）：decoration 按 blockId 挂载、
  按信号呈现/复位且终态不残留；decoration 不改文档模型；不存在绕过 ED 接口直改编辑器状态的调用
  路径（与 DE/AG G4 集成验证）；`open_session` 已应用卡切换到目标文档时不触发二次内容变更。
- **BDocx 表格操作**（BR-ED-DATA-007，REQ-ED-009 验收）：人工和代理操作共用 TableOperation
  规划语义；表格指纹、坐标、动态能力和合并单元格左上锚校验失败时整批拒绝；一个或多个选中单元格
  设置/清除纯色底纹时只改变节点属性，富内容子树保持不变；表格邻近移动按钮随选区/滚动/缩放/事务
  重定位，移动后仍锚定同一 tableId；删除表格入口与移动按钮同组并调用 ED 受控命令，顶部工具组不
  保留重复移动或删除入口。
- 量化 NFR（编辑响应延迟、大文档分页性能）延 G5 标定（APC §9，ED-06）。

## §11 技术债务与盲区（诚实声明）

### §11.1 选型记录（2026-06-11，人类授权 AI 推荐，旧版仓库实证取材）

> 人类指示「参考旧版 binder-project 后由 AI 推荐方案」。以下推荐基于旧版仓库实证
> （package.json、BlockIdExtension.ts、canonical_html.rs、R-ED-M-R-01 v2.0、R-ED-M-R-23），
> 原料不为权威，按 APC §8 经本文重新声明为推荐选型；实现期验证后转正式锁定。

- **TipTap 选型（解除 ED-01）**：TipTap **v3 主版本系**（旧版实证 3.11–3.13 运行良好）+
  `@tiptap/react` + StarterKit + 扩展集基线（table 族 / image / link / text-style / color /
  font-family / text-align / underline / sub·superscript / task-list / highlight /
  character-count / placeholder）；分页移植旧版本地扩展 `tiptap-pagination-plus`。
  次版本号在实现期 lock（锁主版本 v3，避免 v2↔v3 API 断裂）。
- **blockId 算法（解除 ED-02 算法部分）**：沿用旧版实证机制——TipTap Extension 为块级节点
  （paragraph/heading/blockquote/codeBlock/listItem 等）维护 `blockId` 属性（DOM 持久化为
  `data-block-id`）；生成 `block_` + `crypto.randomUUID()`；经 ProseMirror `appendTransaction`
  自动补缺；**分裂去重规则**：split 复制导致重复 blockId 时，文档序首个保留原 id、后续重复者
  重新分配（与 §4「普通编辑不变」语义一致：原块保 id、新块得新 id）。转换层兜底：canonical 化
  时为缺失 `data-block-id` 的块注入（旧版 `inject_missing_data_block_ids` 模式，归 DOC/WS
  转换缓存层）。**md 源文件不注入 blockId**（不污染纯文本源），md 的跨会话锚定经内容哈希辅助
  重建——此点为推荐而非实证，实现期与 DE 定位联合验证后转锁定。
- **docx 样式子集（解除 ED-04/ED-05 的 ED 侧）**：采纳旧版三级子集契约（R-ED-M-R-23 实证）——
  **必需子集**（标题 H1-H6 / 段落对齐 / 粗斜下 / 有序无序列表 / 基础表格 / 嵌入图片，实证复现率
  ≥95%）为编辑模型承诺承载；**部分支持**（文本颜色 / 字体 / 字号，Pandoc+Lua 实证 30–50%）尽力
  保留并对用户明示；**明确不处理**（页眉页脚 / 文本框 / SmartArt / VBA 宏 / 复杂样式继承链 /
  合并单元格等复杂表格格式）。完整往返保真由 PRV 预览（LibreOffice→PDF 全保真）补偿查看需求。
  转换管线归 DOC（DOC L2 为该子集契约的执行 SSOT，本文为编辑模型侧承载声明）。
  嵌入图片在 `BDOCX-IMAGE-MEDIA-FOUNDATION-01` 已完成资源、alt、宽高与重开证据：TipTap Image 节点
  同时保留稳定媒体身份，并把 Pandoc 的 px/in/cm/mm/pt/pc 长度按 96dpi 规范为 width/height px 属性；
  `BDOCX-IMAGE-INTERACTION-01` 进一步完成 WebKit document 捕获拖宽、独立图片段落文档流移动和人工
  删除；拖宽提交新 width 时清除来源固定 height，由浏览器按固有比例重算。浮动/绝对定位、任意位置
  拖放与外部图片导入仍不属于当前支持集。`BDOCX-IMAGE-AGENT-EDIT-01` 只新增 workspace 项目图片的
  代理独立段落插入和文档节点删除，不开放代理缩放/移动。

### §11.2 仍开放的债务

- **decoration 与 AG 局部修改的业务契约**：归属已确认 AG（2026-06-11），具体接口契约待 AG G2
  联合承接。
- ~~blockId 跨 docx 往返的保持（待实测）~~ → **已实测（PI-ED-09，2026-07-03）：保留率 0%，不达**——
  Pandoc `html→docx` 侧不落自定义属性（`data-block-id` 全丢），回读全量重建。**按既定降级执行**：
  兜底重建（前端 `reconcileBlockIds` appendTransaction 补缺已等效承载，重开后全量重发 id）。
  **契约语义锁定**：blockId/字符锚**有效期 = 编辑会话内**（保存不打断内存模型、id 持续有效；
  关闭重开必换锚）——与 2026-07-03 人类裁定 D-2 一致，agent 跨重开须重新取块（ag.list_blocks）。
  附带实测发现（DOC 侧既有"部分支持"债务佐证，登记不扩范围）：保存侧 color/text-align 内联样式
  衰减、`data-custom-style` div 包裹残留。
- ~~分页光标/遮罩具体实现待敲定~~ → **已实现（PI-ED-15，2026-07-26）**：当前控制权、输入期冻结、
  光标激活周期与首帧揭示以 §6 为准；人类已确认当前发布需求并闭合 `SCROLL-JUMP-PAGINATION-01`，
  后续分页算法修改必须继续满足该课题的滚动稳定性回归门槛。
- **Block 内分页拆分第一阶段已实现并按当前边界闭合**（`PAGINATION-INTRABLOCK-SPLIT-01`）：
  顶层普通段落已按视觉行和可映射 position 拆分；自动测试覆盖分页流、事务映射与既有 gap 扣除。标题、
  `listItem`/TaskList、引用、代码、表格、图片/媒体和含 inline atom 的段落仍按第一阶段设计原子降级，
  因而这些长 Block 仍可能整块后移或超页；扩展它们必须另行设计容器内全宽 gap 与编辑兼容性。正式
  Tauri/WKWebView 中普通段落 `paragraph-line` widget 仍可能形成已观察到的行中强制断开。人类于
  2026-07-26 明确决定不再继续优化，并接受上述两项为当前版本限制；本课题因此闭合，但不得把限制描述为
  已修复。未来若要求 Word 级自然行分页，必须新开架构课题并继续满足 PI-ED-15 防跳硬门槛。
- **open_session StyleOp 无头事务**：文本/块/整篇替换可在 `EditSession.content` 上执行；StyleOp 需要不依赖
  当前 ProseMirror editor instance 的事务实现。未落地前应返回不支持并让 DiffCard 失效，不能后台写 DiskState。
- **孤立 ED session 镜像清理**：本轮恢复链路只保证孤立 `ed_session_snapshots` 不会打开文档；历史镜像的
  GC/清理策略暂不实现，后续可在 workspace 维护任务中按 `ed_open_tabs` 差集清理。
- §11.1 各推荐选型均待实现期验证后转正式锁定；验证不符须回写本文并复核 DE/DOC 依赖方。

### §11.3 BDocx 基础表格结构编辑（2026-07-28）

光标位于 BDocx 表格内时，编辑器必须提供独立、可见的表格工具组：行列增删、首行表头、合并/拆分、
文档流上移/下移及删除表格。合并操作以当前矩形单元格选择为前提；移动仅限顶层文档流中相邻的非表格
块，边界或相邻表格时禁用。列宽允许通过可见列边界拖拽调整，当前会话列宽进入表格快照；不提供数值
列宽、均分列、行高、浮动定位或自由拖放。

富内容单元格的人工编辑保持既有富文本能力，但代理纯文本 `set_cell_text` 必须拒绝，避免覆盖标记或
多个段落。单元格纯色底纹由 §5.3 的 `set_cell_background` 独立承载，只支持不透明 `#RRGGBB` 与清除；
渐变、透明度、图案、主题继承、整行/整表填充和复杂 Word 表格样式仍不属于支持范围。

## §12 后续动作

1. 本文落地后随下一次 `npm run governance:generate` 注册：ED-EDIT / ED-DECOR 链路、
   BR-ED-STATE-001/002、BR-ED-DATA-001/002/003 规则、X-FORBID-005 约束、TERM-ED-001/002 术语、
   REQ-ED-001~005 的 REQ_MAP 进 ADUS。
2. 续 ED G2：A-ED-M-S-01 定义 SM-ED-01（文档编辑会话机），登记 A-SYS-M-S-01 总图与 X-INTER-001
   转移点（ED.edit→SM-WS-01 Idle→Dirty；SM-WS-01 Saved→ED 回 Ready）。
3. 待人类宣布 ED G2 闭合（AI 不自行宣布晋级，APC §2）；DOC G2 时联合解除 ED-04/样式子集与 blockId
   往返保持；DE/AG G2 时承接 decoration 消费契约。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-11 | v1.0 | 初版（ED G2 首件）：ED 规则 SSOT——编辑会话与脏态（经 WS、离线可用、解析失败降级只读）、内容块模型与稳定 blockId（失效可感知）、md/docx 统一编辑模型（二进制归 DOC）、分页与媒体呈现层纯化、编辑器装饰层接口契约（DE/AG 消费）；注册链路 ED-EDIT / ED-DECOR（均 P1）、规则 BR-ED-STATE-001/002·BR-ED-DATA-001/002/003、约束 X-FORBID-005（复用 X-SEQ-002/003·X-FORBID-004·X-SHARED-001·X-INTER-001）、术语 TERM-ED-001 Block / TERM-ED-002 EditorDecoration；REQ_MAP（REQ-ED-001~005）；债务（TipTap 选型/blockId 算法/样式子集/往返保持延后并记明 owner） |
| 2026-06-11 | v1.1 | 人类决策落地：inline-assist/autocomplete 归属确认 AG（边界表述更新）；§11.1 新增选型记录（人类授权 AI 推荐，旧版仓库实证取材）——TipTap v3 系 + React + StarterKit + 扩展集基线 + tiptap-pagination-plus 移植（解除 ED-01）、blockId = TipTap Extension + block_+UUID + appendTransaction 补缺/分裂去重 + 转换层兜底注入、md 源不注入（解除 ED-02 算法部分）、docx 样式子集三级契约（必需/部分/不处理，执行 SSOT 归 DOC L2）；§11.2 余债收敛（往返保留度/分页适配/验证后转锁定） |
| 2026-06-12 | v1.2 | Q2 缺口补遗（人类确认采纳）：§3 新增 BR-ED-STATE-003 外部变更会话处理——无脏态自动重载、有脏态/保存冲突进入显式裁决（保留己方→覆盖确认 / 放弃重载），双向内容损失均须用户显式选择；REQ_MAP 补 REQ-ED-001→BR-ED-STATE-003 |
| 2026-07-03 | v1.3 | §11.2 实测回填（PI-ED-09，A-ED-X-L-02 批B 地基）：blockId 跨 docx 往返保留率 0%（Pandoc 不落自定义属性）→ 按既定降级走兜底重建（reconcileBlockIds 补缺）；契约语义锁定「锚有效期 = 编辑会话内、重开必换锚」（与裁定 D-2 一致）；附带登记保存侧样式衰减与 data-custom-style 残留（DOC 侧既有债务佐证） |
| 2026-07-03 | v1.4 | 批B 契约注册（A-ED-X-L-02 §4，人类裁定 D-1/D-2/D-3）：§0 注册 TERM-ED-004 CharAnchor / TERM-ED-005 SelectionRef / TERM-ED-006 StyleOp；§4.1 字符级锚定与选区引用 + BR-ED-DATA-004（会话内字符锚、纯文本偏移平面、失效可感知）；§5 扩展为 md/docx/html 统一模型 + BR-ED-DATA-002 修订（.html 文本路径不经 DOC）；§5.1 样式投影与操作协议 + BR-ED-DATA-005（可读投影 + StyleOp 受控写 + D-3 落盘分流）；REQ_MAP 补 REQ-ED-006→004 / 007→005 / 008→002 |
| 2026-07-08 | v1.5 | 修正当前编辑器草稿 diff 语义：pending 时经 ED 受控接口把 after 替换进目标 Block 并置脏态，GreenReview 只给 after 内容加绿色背景；accepted 不再改正文，rejected 回滚 before，save 成功后复位 GreenReview |
| 2026-07-04 | v1.5 | 术语统一清扫（承 2026-07-04 人类三轮裁决）：正文禁用别名替换——内联辅助→局部修改、自动补全→辅助续写、对话编辑→Diff Edit、Chat↔Build 接管→构建移交/回落、chat/chat-agent 回合→代理回合；文件名与变更记录历史行不改写 |
| 2026-07-09 | v1.6 | 同步 TERM-DOC-003 b-docx/BDocx：§5 与 BR-ED-DATA-002 将 ED 可编辑 docx 类输入收敛为 b-docx，明确原生 `.docx` 不进入 ED 编辑态，只走 DOC/PRV 预览链或先由 DOC 转为 b-docx |
| 2026-07-09 | v1.7 | 补齐 StyleOp 草稿应用与拒绝恢复约定：Agent 回合 `style_ops` 经 DE 裁决后由 ED 受控接口应用为草稿；拒绝含 StyleOp 的草稿 diff 必须恢复草稿前完整内容 |
| 2026-07-09 | v1.8 | 补齐整篇 content diff 与 GreenReview 绿增呈现约定：整篇 diff 在 pending 时经 ED 受控接口替换为草稿并按当前块集合挂载 GreenReview；正文内只显示 after 绿增效果，红删留在 DiffCard |
| 2026-07-09 | v1.9 | 收紧 GreenReview 挂载条件：只有已成功应用为 ED 草稿的 diff 才能挂载绿色审阅层；锚失效或 error 卡不得残留绿色装饰 |
| 2026-07-09 | v1.10 | 收紧整篇草稿替换入口：ED 仅对 `patchKind=replace_document` 的显式整篇 diff 执行全文替换；无 blockId 且非 StyleOp 的局部 diff 进入 error，避免 BDocx 精准编辑失败时误走整篇替换 |
| 2026-07-09 | v1.11 | 补齐 DiffProjection 消费边界：ED 草稿/GreenReview 只消费后端权威投影；文本/md 可在当前会话用 block_index 映射实时 Block，BDocx 必须使用 canonical `data-block-id`，缺失即 error 不降级 |
| 2026-07-12 | v1.12 | 三态实现回写：保存冲突路径已以 `WS_SAVE_CONFLICT` + `override=true` 显式覆盖确认落地；文件监视/`ws.external_change` 推送仍延后；补 `applyEditOps`/`applyStyleOps` 当前实现入口，明确 DE 草稿与回滚读取当前 ProseMirror LogicalState 而非派生缓存 |
| 2026-07-18 | v1.13 | 补齐已打开文档 LogicalState 门禁：active 取 ProseMirror 实时序列化，非 active open session 取 `EditSession.content + sessionVersion/contentHash`；`draft_mode=open_session` 经 ED session 事务自动应用并持久化 `ed_session_snapshots` |
| 2026-07-18 | v1.14 | 补齐启动恢复打开标签协议：`ed_open_tabs` 作为打开集合、顺序和当前文档真源；恢复只对打开集合取 session 镜像，恢复后当前文档直接进入 AG LogicalState 读取链 |
| 2026-07-18 | v1.15 | ED-MARKDOWN-RICH-ROUNDTRIP-01：替代 Markdown 纯文本实现裁决，修订 BR-ED-DATA-002/005；新增 BR-ED-DATA-006 Markdown Codec 与 BR-ED-VERIFY-001 无静默丢失规则；区分 editFormat/contentFormat，补 CommonMark+GFM 支持矩阵、源码保护与 REQ-ED-001 映射 |
| 2026-07-25 | v1.16 | 分页滚动闪跳修复设计：重测量改为读取当前分页树并扣除既有分页缝几何，禁止以临时隐藏/恢复可见分页缝换取无缝测量；末页补白与普通分页缝 DecorationSet 更新解耦，仅分页结构变化才重建普通装饰。复用 BR-ED-DATA-003 / ED-EDIT，不新增规则。 |
| 2026-07-25 | v1.17 | 分页打开即振荡复盘：禁止以整个 ProseMirror 的 ResizeObserver 作为重测量源，改为仅由文档事务、纸张几何、字体/媒体加载与窗口尺寸等外部输入失效化；分页方案提交不能递归触发下一轮重排。 |
| 2026-07-25 | v1.18 | BDocx 打开控制面收敛：启动恢复、状态机监听均单飞；异步打开按路径 latest-wins，非活动路径迟到结果仅补全会话不重置当前编辑器；内容载入将分页启用与纸张几何合并为一次布局事务。 |
| 2026-07-25 | v1.19 | 修正缩放下的光标自动滚动判定：统一以视口 rect 判断可见性，移除 scrollTop/clientHeight 混算与高频日志；底部仍可见光标不再错误放行 ProseMirror 默认滚动。 |
| 2026-07-25 | v1.20 | 分页视口锚定落地：结构分页变化前后保持可见光标/首内容块的屏幕位置，分页层成为唯一布局滚动控制器；停用 ProseMirror scroll-preserve 竞争写入、淘汰过期测量并修正末页底边距叠加。 |
| 2026-07-25 | v1.21 | 分页锚定首帧修正：将补偿由下一帧移入分页提交同一任务，优先以原生 Selection/Dom range 而非 widget 边界 `coordsAtPos` 作为可见光标锚，避免先下推再回拉的单跳。 |
| 2026-07-26 | v1.22 | 分页控制权重新收敛：恢复 ProseMirror 原生 preserve/scrollIntoView；输入期冻结 break plan，静默后一次结算；Decoration 结构与 fill CSS 更新解耦并保留小数精度；分页事务只做末尾残差校验；BDocx 等待当前 loadSeq 首轮 settled 后揭示。 |
| 2026-07-26 | v1.23 | 发布构建复测后收敛残余首输入单跳：保留 ProseMirror 更新期 preserve 与真实越界滚动，分页模式仅在折叠文本光标已经完整可见时预拦截冗余 `scrollIntoView`；Selection/DOM Range 与 client viewport 统一使用屏幕坐标，不混入 scrollTop/zoom。 |
| 2026-07-26 | v1.24 | Range可见性门控真机失败后增加一次性指针激活事务门控：主键定位记录doc引用与scrollTop，只对其后的第一次doc变化且用户未主动滚动时吞掉scrollIntoView；消费/非内容请求/blur即失效，composition首输入纳入但不接管后续原生滚动。 |
| 2026-07-26 | v1.25 | 第二字符稳定单跳证明一次性门控仅延迟原生归位；升级为光标激活周期，以指针真实屏幕点和Range相对位移估算后续光标，运行时适配zoom是否已进入rect，仍可见且用户未滚动时持续拦截，真实越界/导航/blur释放。 |
| 2026-07-26 | v1.26 | 闭合前隐患收口：分页内部scrollTop补偿以专用事件更新激活周期基线；文档/格式/缩放/纸张身份变化显式清理；指针守卫延至ProseMirror最终选区同步后建立，排除双击、拖选与非折叠选区。 |
| 2026-07-26 | v1.27 | 实现一致性同步：明确fill只更新稳定DOM、不重建Decoration；IME仅在无有效激活周期时无条件交还原生滚动；关闭“分页方案待敲定”旧债务，并登记当前仅支持顶层块/listItem边界、尚不支持块内行盒拆分页。 |
| 2026-07-26 | v1.28 | 课题迁移：人类确认闭合 SCROLL-JUMP-PAGINATION-01；Block 内分页拆分转入 PAGINATION-INTRABLOCK-SPLIT-01，锁定普通段落视觉行先行、复杂类型原子降级、纯呈现不改模型及滚动稳定性硬回归门槛。 |
| 2026-07-26 | v1.29 | PI-ED-16 第一阶段实现同步：分页计划迁移为可映射 position+稳定页序id；顶层普通段落按grapheme合法位置以指数探测+二分定位视觉行，并按2+2优先规则拆分；既有Block/行间gap按position+bias扣除，复杂节点和不可测几何原子降级。自动验证完成，正式WKWebView交互与缩放矩阵仍待人工验收。 |
| 2026-07-26 | v1.30 | PI-ED-16人工验收纠偏：2+2防孤行会主动撤回上一页本可容纳的一行并制造页尾空白；现行规则改为贪心填页——保留当前页所有完整可容纳视觉行，仅零行可容纳时换页，允许单行页首/页尾片段。测量、position映射、输入冻结与滚动补偿边界不变。 |
| 2026-07-26 | v1.31 | PI-ED-16行尾断点修复：确认旧段内`display:block`分页缝会污染可见DOM换行并自证旧position；自然行改由无分页缝离屏段落副本测量，断点增加纵向换行+横向回行双验证，无法证明即整段原子降级；旧断点移除后由Decoration自然恢复连续排版，不修改LogicalState。 |
| 2026-07-26 | v1.32 | PI-ED-16二次人工失败纠偏：v1.31离屏副本仍克隆了受WebKit辅助BR/残留结构污染的可见DOM，不能作为自然排版真值；现改由ProseMirror LogicalState经schema DOMSerializer重新生成段落，并将含hardBreak段落原子降级，禁止短行/疑似旧widget污染成为分页候选。 |
| 2026-07-26 | v1.33 | PI-ED-16物理分页边界纠正：完整保留页缝、页码、纸张高度和既有防跳链；废止离屏近似。已有段内断点重测时，仅在同一rAF内以呈现事务暂时过滤paragraph-line widget，Block页缝始终保留，直接读取当前可见ProseMirror真实行盒并在回调返回前恢复最终完整分页。 |
| 2026-07-26 | v1.34 | 块模型A+D方案落地：引入"容器块"（orderedList/bulletList）与"内容块"（paragraph/heading/listItem/codeBlock/blockquote）两级分类，容器块获得blockId但对分页/投影透明；BlockIdExtension导出isContentBlock/isContainerBlock/iterContentBlocks；edLocate的topBlockAt改为resolveContentBlock递归穿透容器；edStyleOps/outlineStore改用descendants遍历；Pagination新增flattenBlocks展平列表容器；findTopBlockSnapshotById/getTopBlockSnapshotByIndex/getTopBlockIdByIndex改为descendants收集所有blockId节点。后端blocks.rs/deService.ts/block_backfill.rs同步扩展ul/ol标签。BR-ED-DATA-003分页闭环，PI-FE-30债务台账更新。 |
| 2026-07-26 | v1.34 | 分页课题闭合：人类接受普通段落仍可能行中强制断开及复杂节点原子降级作为当前版本边界；移除正式人工矩阵开放门禁，保留物理分页、纯呈现规则、打开收敛和PI-ED-15防跳控制。 |
| 2026-07-28 | v1.35 | BDocx 表格基础交互实现同步：新增 §11.3，表格内独立可见工具组覆盖行列、表头、合并/拆分、文档流移动与删除；列边界拖拽可见且写入会话快照。锁定不支持数值/均分列、行高、浮动/自由拖放、背景底纹和复杂样式往返；富内容单元格拒绝代理纯文本覆盖。 |
| 2026-07-28 | v1.36 | BDocx TableCellShading：注册 TERM-ED-007 TableOperation 与 BR-ED-DATA-007；新增 `set_cell_background` 的颜色、坐标、富内容保护、指纹和原子规划契约；§11.3 将纯色底纹从排除项收敛为明确支持子集。 |
| 2026-07-28 | v1.37 | BDocx 表格移动入口迁移：顶部工具组移除 `↑表/↓表`，改为活动表格附近的临时上下按钮；锁定选区/滚动/缩放/事务重定位、不可用方向禁用及移动后同一 tableId 继续激活，不扩大为拖动或任意位置移动。 |
| 2026-07-28 | v1.38 | 表格邻近控件真实 UI 修复：活动表格 DOM 改由选区 `domAtPos` 优先解析并兼容 TableView wrapper，补充 pointerup/keyup/focusin 与嵌套滚动重定位；删除表格入口一并迁入邻近按钮组，顶部不再重复展示。 |
| 2026-07-28 | v1.39 | 表格邻近控件位置修复：依据人工截图确认 WebKit 下 viewport fixed 坐标与编辑器容器错位；改为 editor-host 内 absolute 局部坐标，控件默认贴表格右上侧、空间不足回落左侧，并保持 paper 可视区钳制。 |
| 2026-07-28 | v1.40 | 表格邻近控件锚定方案重构：人工复测证明 editor-host 局部坐标仍受分页/缩放布局影响；废止祖先坐标换算，改由 `document.body` portal 承载 fixed 浮层，直接使用活动 table 的 viewport DOMRect 作为唯一位置真值。 |
| 2026-07-28 | v1.41 | 表格邻近控件改为 DOM 归属锚定：人工复测证明 table viewport rect 路径仍不可靠；删除全部几何计算，React portal 直接挂到活动 TipTap `.tableWrapper`，由 wrapper 内 absolute 右上角布局自然跟随表格。 |
| 2026-07-28 | v1.42 | 表格邻近控件防遮挡优化：依据人工截图将右上角竖向按钮组改为横向迷你工具条；所有 tableWrapper 保留一致顶部安全区，避免遮挡最后一列表头/单元格及选区切换时的布局跳动。 |
| 2026-07-28 | v1.43 | StyleOp 接入 A-SYS-I-P-04 BDocxCapabilityManifest；明确 AG Diff 前静态/锚/范围校验与 ED 对历史/IPC 载荷的严格批内原子防御，真实 mark 名固定为 subscript/superscript。 |
| 2026-07-28 | v1.44 | BDOCX-IMAGE-MEDIA-FOUNDATION-01：锁定 BDocx 图片稳定相对媒体身份与 `asset://` 显示投影分层；持久快照/保存去投影、重启经 DOC 重新授权；ResizableImage 恢复 Pandoc 物理宽高并明确本课题不含移动或代理图片操作。 |
| 2026-07-28 | v1.44 | 明确 CharAnchor 偏移单位为 UTF-16 code unit，与 JavaScript/ProseMirror 实现及 BDocx 代理执行前校验统一。 |
| 2026-07-28 | v1.45 | BDOCX-IMAGE-INTERACTION-01：BR-ED-DATA-003 纳入图片属性事务、独立图片段落移动/删除和媒体身份保持边界；WebKit 拖宽改用 document 捕获鼠标事件；真实 Pandoc 探针确认 data-display 不持久，撤下 inline/block 按钮声明。 |
| 2026-07-29 | v1.46 | BDOCX-IMAGE-AGENT-EDIT-01：注册 TERM-ED-008 ImageOperation；定义项目内图片独立段落插入、图片实例删除、动态基线/锚/来源哈希校验、批原子草稿事务、源文件保护与代理缩放/移动排除边界。 |
| 2026-07-29 | v1.47 | BDOCX-IMAGE-ADVANCED-AGENT-OPS-01：ImageOperation 扩展受控缩放、上下移动与独立段落对齐，锁定稳定 imageId、当前 fingerprint、动态能力和同图单批一操作。 |
| 2026-07-30 | v1.48 | 图片编辑闭合审计：移除“代理缩放/移动不可用”的历史排除描述；当前子集对齐 ImageOperation 组合缩放/布局/移动与受控拖入，继续排除高级布局。 |
| 2026-08-01 | v1.50 | ED-EDITOR-OPERATIONS-01：定义编辑器级保存、撤销重做、查找替换、格式感知定位、BDocx 纯呈现页码导航与格式化导出；明确编辑态无打印入口，打印仅归 PRV 只读预览。 |
| 2026-08-01 | v1.51 | ED-DOCUMENT-TOOL-ENRICHMENT-01：补齐受支持文字、列表和分隔线工具；图片对齐与表格合并/拆分保持对象就近入口，并明确本地替换图片须待受控导入链接线。 |
| 2026-08-01 | v1.52 | ED-DOCUMENT-TOOL-ENRICHMENT-01：高亮增加预设色板、自定义不透明底色与清除入口；Markdown 继续不暴露该样式。 |
| 2026-08-01 | v1.53 | ED-DOCUMENT-TOOL-ENRICHMENT-01：已登记的删除线命令同时开放给 Markdown、BDocx 与 HTML 富文本。 |
| 2026-08-01 | v1.54 | ED-DOCUMENT-TOOL-ENRICHMENT-01：文字颜色并入文字分组，取消第二行独立颜色分组。 |
| 2026-08-01 | v1.55 | ED-DOCUMENT-TOOLBAR-COMPATIBILITY-01：回链工具栏功能与兼容性清单；明确 UI 已暴露能力不等于 BDocx 往返保真。 |
| 2026-08-05 | v1.56 | PRV-TOOLBAR-ADAPTIVE-01：抽取 EditorToolbar 的 TbBtn/Sep/ToolbarMenu 为共享组件 edToolbarParts（EditorToolbar 行为不变）；预览模式 PrvToolbar 按内容类型自适应工具集（创建草稿/导出/缩放/打印），只读边界不弱化、不提供关闭工具。 |
| 2026-08-05 | v1.57 | ED-FONT-LOCAL-LIBRARY-01：字体组本地字体库同步——新增 fonts.list_fonts（macOS CoreText 只读枚举）+ fontService 缓存；字体下拉常用子集置顶（仅可用展示）+ 本地字体追加（document.fonts.check 过滤）；写入前 toCssFontFamily 引号规范化（修复多词字体名无引号拆词）；枚举失败/空列表回退常用子集，不阻塞编辑。 |
| 2026-07-30 | v1.49 | AG-RICH-BLOCK-STRUCTURED-EDIT-01：活动富文档块投影直接来自 ProseMirror 层级节点；容器/原子块只读，内容子块独立写；BlockOperation 批原子应用及 fingerprint 冲突安全的 exact node 回滚取代 HTML 字符串替换。 |
