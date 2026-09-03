---
文档编号：ED-M-D-01
文档状态：A
负责模块：ED
文档职责：ED 编辑器模块级功能需求描述——声明模块级 REQ-ED-*，承接 APC §4.1 ED 职责与 REQ-CORE-002/004/009/010/011/012/013/014/015，界定 HTML/Markdown/纯文本与 BDocx 处理副本的富文本编辑运行时 / block 模型与 blockId 定位 / 分页系统 / 图文媒体能力 / 编辑器装饰层（供 diff 预览）/ CreateDocumentBundle 目标会话 / WorkspaceOperation 对标签与内容 Diff 的影响 / Agent 只读会话事实
上游约束：A-CORE-C-D-01_产品概述.md（REQ-CORE-002/004/009/010/011/012/013/014/015）、A-CORE-X-D-02_Agent1.0冻结能力基线.md（P0/P1 与不可突破边界）、A-WS-M-T-02_工作区资源操作技术设计.md、A-CORE-C-P-01_APC.md（§4.1 ED 职责、§9 离线边界）、A-SYS-C-T-01_系统总体架构.md、A-WS-M-T-01_工作台技术设计.md、ADU.md（§3.2 REQ 规范）
直接承接：ED L2 技术设计（编辑器运行时选型 / block 模型与 blockId / 分页 / 媒体 / decoration 接口）经 REQ_MAP 映射 RULE/CHAIN/CONSTRAINT/TERM；decoration 层供 DE（diff 呈现/审阅）与 AG（局部修改）消费
使用边界：本文是 L1 功能需求描述层，仅声明模块级 REQ-ED-*，**不定义技术规则**（RULE/CHAIN/CONSTRAINT/TERM/STATE_MACHINE 见 ED L2）；**代码不得直接映射 REQ**（ADU §3.2）；编辑器框架版本/扩展选型、blockId 生成策略、分页实现、decoration 接口归 ED L2；DOCX/Pandoc 转换归 DOC、diff 生成与终态归 DE、文件 I/O 归 WS、预览导出归 PRV
变更要求：ED 职责或编辑运行时边界变更须先经 APC 授权再回写本文；REQ 增删按 ADU §3.2 req_id 规则（序号不复用，废弃改 superseded）
治理成熟度：draft
---

# Binder ED 编辑器功能需求描述（L1）

## §1 文档定位

本文是 **ED 模块**的 L1 功能需求描述，处在产品概述（L1 根）之下、ED L2 技术设计之上：

```
产品概述(REQ-CORE-*) + APC §4.1 → 本文(REQ-ED-*) → REQ_MAP → ED L2 规则(SSOT) → 代码(@GOV)
```

ED 是 Binder 写作产品的**富文本编辑运行时**（APC §4.1：TipTap，md/docx 富文本编辑）：提供编辑核心、
内容的结构化 block 模型与稳定定位（blockId）、分页布局、图文媒体能力，以及供 diff 呈现/局部修改的
**编辑器装饰层（decoration）**。ED 依赖 WS 的文件 I/O（A-SYS-C-T-01 §4：`ED→WS`），其 decoration
面被 DE（diff 审阅）与 AG（局部修改）消费。本文只声明 ED 模块级需求，不写技术方案、不定义规则、
代码不得追溯本文；编辑器框架选型、blockId 策略、分页实现、decoration 接口一律归 ED L2。未被 APC /
产品概述 / SYS·WS 契约支撑的判断一律进 §7 暴露清单，不杜撰。

> 关于 TipTap：APC §4.1 已明确编辑运行时基于 TipTap（md/docx 富文本编辑）；具体 TipTap/ProseMirror
> 版本、扩展集与是否定制 schema 属技术选型，归 ED L2（A-SYS-C-T-01 §8 已将「ED 的 TipTap 集成」延至
> ED L2），本文 L1 不锁定实现框架细节。

## §2 模块定位与职责

**定位**：ED 是 Binder 的**富文本编辑运行时与编辑呈现底座**，是用户与 AI 修改在文档上的可视编辑面；
向上承载写作交互，向下经 WS 进行文件 I/O，横向为 DE/AG 提供 diff 与局部修改的呈现装饰面。

**职责清单（v1，APC §4.1）**：

| # | 职责 | 锚定 |
|---|---|---|
| E-J1 | 富文本编辑运行时：Markdown、纯文本、HTML 与 BDocx 处理副本的富文本编辑（基于 TipTap）；原生 DOCX 不进入编辑会话 | APC §4.1、REQ-CORE-004/009 |
| E-J2 | block 模型与稳定 blockId 定位（结构化内容锚，供 diff/引用/分页） | APC §4.1、REQ-CORE-002 |
| E-J3 | 分页系统与布局（分页模式、光标与遮罩、打印友好呈现） | APC §4.1、REQ-CORE-004 |
| E-J4 | 图文媒体能力（图片等媒体的插入、呈现与处理） | APC §4.1 |
| E-J5 | 编辑器装饰层（decoration）：承载 diff 呈现/审阅与局部修改 | REQ-CORE-002 |
| E-J6 | CreateDocumentBundle 目标会话：磁盘资源确认创建后自动打开标签、投影初始内容 Diff、承载后续编辑与关闭提醒 | REQ-CORE-010 |
| E-J7 | Agent 只读会话事实：向受限回合如实投影活动/打开文档、选区、脏态、保存基准、LogicalState 和关联 Proposal | REQ-CORE-011 |
| E-J8 | WorkspaceOperation 标签协调：脏标签阻断资源执行，成功后按真实磁盘路径更新标签并传达内容 Diff 失效 | REQ-CORE-012 |
| E-J9 | 最小格式转换目标会话：预览成功后打开目标副本标签，承接继续编辑、卡片失效与未保存提示 | REQ-CORE-013 |
| E-J10 | 可恢复删除前置：受影响已打开脏会话必须保存或明确关闭并丢弃；删除成功后标签按真实存储状态关闭或转为只读恢复入口 | REQ-CORE-014 |
| E-J11 | 多既有文档逐项会话：已确认冻结目标清单内的标签、LogicalState、预览和失败状态相互隔离 | REQ-CORE-015 |

> 边界澄清：ED **提供编辑面与 decoration 承载机制**，但 diff 的**生成与"接受/拒绝/过期"终态语义归
> DE**、局部修改（inline-assist）/辅助续写（autocomplete）的**业务归属待确认**（A-SYS-M-S-01 §3，
> ED/AG/DE 之间）；ED 只保证装饰面可承载这些呈现，不抢定其业务语义。DOCX/Pandoc **格式转换归 DOC**，
> ED 承接转换后内容的富文本编辑呈现。文件读写经 **WS**（A-WS-M-T-01 WS-FS），ED 不直接触达文件系统。

## §3 能力范围（ED v1）

**IN（ED 承载）**：
- md / docx 内容的富文本编辑运行时（基于 TipTap）。
- 内容的 block 结构化模型与稳定 blockId 定位（供 diff 定位、@mention 引用锚、分页切分）。
- 分页系统：分页布局、光标与遮罩、打印友好呈现。
- 图文媒体：图片等媒体的插入、呈现与处理。
- 编辑器装饰层（decoration）：为 DE 的 diff 受控预览/审阅、AG 的局部修改提供呈现承载面。
- 未保存文档会话：新建提案预览成功后自动打开可编辑标签；未保存文档尚不属于 workspace 文件树。
- Agent 只读会话事实：对所属 workspace/AgentSession 的受限任务投影真实编辑器会话，不提供写入能力。
- 原生 DOCX 只读边界与 BDocx 编辑副本呈现：原生 DOCX 由 PRV 只读预览；其副本才转为 HTML 编辑模型，并以模拟 Word 分页呈现。
- TXT↔Markdown 转换目标会话：转换成功后的目标副本进入未保存 LogicalState 与标签，来源不进入编辑或写回路径。
- 可恢复删除前置与多既有文档逐项会话：删除不丢弃未保存内容；已确认目标清单内的各会话独立预览、失效、保存与呈现。

**OUT（非 ED）**：
- DOCX / Pandoc 格式转换——归 DOC；ED 承接转换后内容的编辑呈现。
- diff 的生成、终态转移语义（accepted/rejected/expired）——归 DE；ED 仅提供 decoration 呈现面。
- 文件读写、save 落盘、workspace 边界——归 WS（A-WS-M-T-01）；ED 经 WS 进行 I/O。
- 代理编辑（编辑/构建双模式）回合、工具调用——归 AG；inline-assist/autocomplete 业务归属已确认归 AG（2026-06-11，§7 ED-05）。
- 多格式预览/导出、打印输出管线——归 PRV（ED 提供打印友好的分页呈现，导出管线归 PRV）。
- 编辑器框架版本/扩展选型、blockId 生成算法、分页与 decoration 的实现——归 ED L2（本文只定边界）。
- NFR 量化指标（编辑响应延迟、大文档分页性能）——延 G5 标定（APC §9）。

## §4 代表性流程（契约级，APC §7）

**E1 · 富文本编辑与保存协同**（E-J1，承接 REQ-CORE-004/009）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | WS/DOC | workspace 内 Markdown、纯文本、HTML 或 BDocx 编辑副本 | ED 按其编辑路径装载富文本模型（Markdown 经 Markdown Codec；BDocx 副本由 DOC 转为 HTML） | 可编辑文档 | 2 | 内容无法安全装载→提示并进入源码保护/只读降级，不静默丢失；原生 DOCX 不进入本流程 |
| 2 | 用户 | 编辑操作 | ED 更新编辑模型并标记脏 | 脏编辑态 | 3 | 离线时编辑照常可用（REQ-CORE-009） |
| 3 | 用户/ED | save 触发 | ED 交 WS 落盘（A-WS-M-T-01 SM-WS-01） | 已保存 | 终态 | 落盘失败由 WS 回报，ED 保持脏态可重试 |

**E2 · diff 装饰呈现与裁决协同**（E-J5，承接 REQ-CORE-002）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | DE | 生成的 diff | ED 校验 blockId/提案基准后，在受控预览中把候选内容投影到 LogicalState，并呈现差异 | 可编辑预览 + 可视 diff | 2 | 定位锚或基准失效→回报 DE，不静默错位呈现 |
| 2 | 用户 | 接受/拒绝 | 由 DE 裁决提案；接受不重复应用，拒绝仅在原始基准有效时恢复对应预览 | accepted/rejected/expired | 3 | 手动编辑、后续预览或外部变更→expired，控件不可用 |
| 3 | 用户→WS | 显式保存 | WS 将当前 LogicalState 写入磁盘；ED 按 DE 状态同步审阅呈现 | 已落盘 + 呈现复位 | 终态 | 保存失败→保留脏态与可编辑内容 |

> E2 在 L1 锁定：ED 是受控预览进入 LogicalState 的编辑面，DE 拥有提案裁决、WS 拥有显式保存落盘；
> decoration 与 DE/AG 的具体接口契约归 ED L2 + DE/AG 联合（§7 ED-05）。

**E3 · 分页与媒体呈现**（E-J3/E-J4，承接 REQ-CORE-004）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | ED | 编辑模型 | 按分页规则切分布局，呈现光标与遮罩 | 分页视图 | 2 | 超长内容/复杂布局→ED L2 性能策略（延 G5 NFR） |
| 2 | 用户 | 插入图片/媒体 | ED 呈现并处理媒体（资源经 WS） | 含媒体的文档 | 3 | 媒体资源越 workspace 界→拒绝（WS X-FORBID-004） |
| 3 | 用户 | 打印/导出意图 | ED 提供打印友好分页呈现，导出管线交 PRV | 打印友好呈现 | 终态 | 导出格式能力归 PRV/DOC |

## §5 模块级需求声明（REQ-ED-*）

> owner_module = ED。每条经 source_doc 标注承接的产品级 REQ-CORE-* / APC 锚点。
> 代码不得直接映射本文 REQ（ADU §3.2）；映射到规则在 ED L2 经 REQ_MAP 完成。

<!-- REQ
req_id: REQ-ED-001
title: 富文本编辑运行时（Markdown/纯文本/HTML/BDocx 编辑副本）
owner_module: ED
status: active
summary: 提供基于 TipTap 的富文本编辑运行时，支持 workspace 内 Markdown、纯文本、HTML 与 BDocx 编辑副本按处理路径解析为结构化 LogicalState、以终态富文本呈现并进行内容与基础结构编辑，编辑能力离线可用；HTML/BDocx 的字符基础样式为 P0，颜色/突出显示、段落对齐/间距/缩进、字号与字体族为 P1 受控实现。原生 DOCX 只读，不建立编辑会话；落盘经 WS
source_doc: APC §4.1、REQ-CORE-004、REQ-CORE-009
acceptance: Markdown 的 H1-H6、强调、列表、引用、代码、链接、图片、表格、分隔线等受支持语法加载后以终态富文本呈现且源码标记不作为正文显示；未编辑 Markdown 不改写，编辑后保存保持支持子集语义等价，无法无损处理时不得静默保存；HTML 与 BDocx 编辑副本可加载为富文本并编辑，且均支持 P0 加粗/斜体/下划线/删除线，以及 P1 文字颜色/突出显示、段落左中右两端对齐、行距/段前后/首行与左右缩进、固定字号档位和字体族切换；所有 P0/P1 项均须在实现后以真实样例完成保存重开验证，未验证前不对外开放；纯文本（.txt 等）提供段落级结构内容编辑工具栏（引用/代码块/分隔线，内容可保存），文字级样式（字体/字号/颜色等）不展示（txt 保存为纯文本不持久化，ED-TXT-TOOLBAR-01）；原生 DOCX 不生成编辑器标签；离线编辑可用；save 经 WS 可靠落盘（A-WS-M-T-01）
open_issues: Markdown 扩展仍为 Beta，具体兼容范围与非标准语法降级由 ED L2 及 fixtures 门禁锁定；docx 富文本编辑与 DOC 转换边界归 ED/DOC L2（ED-04）
-->

<!-- REQ
req_id: REQ-ED-002
title: block 模型与稳定 blockId 定位
owner_module: ED
status: active
summary: 编辑内容以结构化 block 建模并赋稳定 blockId，作为 diff 定位、@mention 引用锚与分页切分的统一定位基准
source_doc: APC §4.1、REQ-CORE-002
acceptance: 编辑内容具结构化 block 与稳定 blockId；diff/引用可经 blockId 稳定定位到目标内容
open_issues: blockId 生成算法与编辑变更下的稳定性策略归 ED L2（ED-02，影响 diff 定位正确性）
-->

<!-- REQ
req_id: REQ-ED-003
title: 分页系统与布局
owner_module: ED
status: active
summary: 提供分页模式的布局、光标与遮罩处理及打印友好呈现，使长文档可分页编辑与打印输出
source_doc: APC §4.1、REQ-CORE-004
acceptance: 长文档可分页呈现与编辑，光标/遮罩行为正确，分页呈现打印友好
open_issues: 分页实现（光标/遮罩方案）与大文档分页性能归 ED L2（ED-03）；量化性能延 G5（ED-06）
-->

<!-- REQ
req_id: REQ-ED-004
title: HTML 与 BDocx 处理路径的图文媒体能力
owner_module: ED
status: active
summary: 支持 HTML 编辑文档与 BDocx 处理副本中的图片等媒体插入、呈现与处理，媒体资源经 WS 限定在 workspace 内；该能力为 P1 受控开放，原生 DOCX 不直接编辑
source_doc: APC §4.1、REQ-CORE-001（workspace 边界）
acceptance: HTML 编辑文档与 BDocx 处理副本可在编辑器内呈现和选择图片；图片宽度可通过可见手柄按固有比例调整；独立图片段落可经邻近控件在文档流内上移、下移或删除，混排文字中的行内图片不得被当作整块重排；代理可把经校验的 workspace 项目图片插入稳定 Block 前后，或对当前文档图片实例执行删除、受限宽度缩放、上下移动和独立段落左/中/右对齐，操作经结构化 Diff 裁决且不改写项目源文件；同图缩放/对齐/移动可单批原子组合，删除独占；用户从项目树、Finder 拖入或粘贴的 PNG/JPEG 经受控复制/复用后插入独立图片段落；成功操作产生脏态，BDocx 副本保存重开后图片资源、alt、尺寸、顺序、对齐和删除结果保持；原生 DOCX 不直接开放图片编辑；媒体资源不越出 workspace（WS X-FORBID-004）
open_issues: 图片视觉识别/OCR、文字环绕、行内混排重排、浮动或绝对定位、任意坐标布局、裁剪、旋转和独立高度不在当前支持子集；Pandoc 不保留自定义 data-display，故不把 inline/block 按钮声明为 BDocx 持久能力
-->

<!-- REQ
req_id: REQ-ED-005
title: 编辑器装饰层（供 diff 呈现与局部修改）
owner_module: ED
status: active
summary: 提供编辑器装饰层（decoration），按 blockId 承载 diff 的受控预览、审阅与局部修改的可视呈现；已有 LogicalState 的预览成功后候选内容进入该 LogicalState，背景 pending 在打开目标文档时也必须以同一 payload 建立并进入 LogicalState，提案裁决归 DE，ED 不直接写 DiskState
source_doc: REQ-CORE-002/010、A-CORE-X-D-02 §3/§4
acceptance: decoration 可按 blockId 定位呈现 diff；成功预览准确投影到已有或 pending 打开后新建的 LogicalState 并显示绿色增量；卡片在预览/迁移后仍为 pending，显式接受不重复应用预览内容，保存有效 pending 预览会一并裁决为接受，拒绝只在基准有效时复原；基准推进后旧提案不得错位回滚或继续裁决
open_issues: decoration 与 DE diff/AG inline-assist 的接口契约归 ED L2 + DE/AG 联合（ED-05）
-->

<!-- REQ
req_id: REQ-ED-006
title: 字符级定位与选区引用
owner_module: ED
status: active
summary: 编辑器提供会话内字符级内容锚（blockId + 块内纯文本偏移）与光标/选区位置的受控查询，供 agent 精确编辑定位与位置引用生成（选区可拖拽携带位置信息入 AG 输入面）；锚失效可感知
source_doc: REQ-CORE-002、2026-07-03 人类需求 #1/#2/#6 与裁定 D-2（A-ED-X-L-02 §2：会话内锚定，跨会话显式延后）
acceptance: agent 可经 {blockId,start,end} 精确定位目标字符区间（各格式统一纯文本平面）；光标/选区可按需查询；选区拖入 AG 输入面生成含位置的引用；锚失效回明确信号不静默错位
open_issues: 跨会话字符锚（内容哈希重建）显式延后（D-2 + PI-ED-09 实测 0% 往返保留）；技术契约归 ED L2（BR-ED-DATA-004）
-->

<!-- REQ
req_id: REQ-ED-007
title: agent 可读样式投影与受控编辑边界
owner_module: ED
status: active
summary: 文档样式（字符 marks / 块属性）可提供机器可读结构投影供 agent 读取；HTML/BDocx 的已确认操作集合为：P0 加粗、斜体、下划线、删除线，P1 颜色/突出显示、段落对齐/间距/缩进、字号与字体族。样式修改仅以受控操作进入编辑模型；投影不构成原始 HTML、CSS 或 OOXML 写入授权；agent 落盘路径按裁定 D-3 分流（回合产出经 DE diff 裁决、显式局部修改视为用户编辑）
source_doc: REQ-CORE-002、2026-07-03 人类需求 #3 与裁定 D-3（A-ED-X-L-02 §2：A 默认 + B 限内联）
acceptance: agent 可读到结构化样式投影；仅已确认操作可生成 HTML/BDocx 的样式 Proposal，字符基础样式按 P0 授权，P1 项按动态能力开关授权；样式 diff 经 DiffCard 可裁决落盘，显式内联样式操作即改即脏；所有 P0/P1 项实现后须经真实保存重开验证才可对外开放；不存在绕过裁决的回合直改路径（X-FORBID-006）
open_issues: 样式 op 集与投影 schema 归 ED L2（BR-ED-DATA-005）+ ED/DE/AG I-P 联合；DiffCard 样式呈现归 DE
-->

<!-- REQ
req_id: REQ-ED-008
title: .html 富文本编辑接入
owner_module: ED
status: active
summary: 独立 .html 文件加入统一编辑模型：富文本编辑 + blockId（.html 源可落盘 data-block-id）+ 序列化往返保真；读写经 WS 文本路径（非二进制，不经 DOC 管线）
source_doc: REQ-CORE-004、2026-07-03 人类需求 #1 与裁定 D-1（A-ED-X-L-02 §2：.html 富文本入范围）
acceptance: .html 打开为富文本并可编辑；编辑保存往返保真；blockId 跨保存稳定（.html 可落盘 id，锚定强于 md）
open_issues: 统一模型扩展归 ED L2（BR-ED-DATA-002 修订）；.html 的 head/script 等非正文内容的处理策略实现期定
-->

<!-- REQ
req_id: REQ-ED-009
title: HTML 与 BDocx 处理路径的表格语义编辑与单元格底纹
owner_module: ED
status: active
summary: HTML 编辑文档与 BDocx 处理副本的表格人工交互与代理修改共用受控表格操作协议；支持基础结构操作和单元格纯色底纹，代理建议必须进入 Diff Edit 裁决；该能力为 P1 受控开放，原生 DOCX 不直接编辑
source_doc: REQ-CORE-002、REQ-CORE-004、2026-07-28 人类确认课题 A/B
acceptance: HTML 编辑文档与 BDocx 处理副本的表格可执行行列增删、表头、合并拆分、文档流移动和列边界拖拽；所选一个或多个单元格可设置或清除纯色底纹；代理可经 list_tables→table_ops 产生结构化建议，底纹差异按坐标和前后色值准确呈现；富内容单元格设置底纹时内容树不变；保存并重开 BDocx 副本后受支持底纹保持；原生 DOCX 不直接开放表格编辑
open_issues: 列宽数值输入/均分列、行高、浮动或绝对定位、渐变/主题/图案填充、嵌套表格和列宽 DOCX 往返保证不在当前范围
-->

<!-- REQ
req_id: REQ-ED-010
title: CreateDocumentBundle 的编辑器会话
owner_module: ED
status: active
summary: CreateDocumentBundle 的 DecisionRequest 确认且 WS 创建目标 DiskState 后，ED 自动打开真实目标标签；随后承载初始内容 Diff 的受控预览、LogicalState、继续手动或代理编辑与普通未保存提示。确认前不得出现目标标签
source_doc: REQ-CORE-010、A-CORE-X-D-02 §3/§4、A-WS-M-T-02 §6
acceptance: 确认后标签对应真实磁盘节点；初始内容投影后可继续编辑；三重基线有效的拒绝由 WS 删除目标后 ED 自动关闭标签，任一手动/代理编辑或磁盘变化使卡片失效且标签保留；显式保存写入既有目标，关闭脏标签仍提示未保存；创建或投影失败不显示幽灵标签
open_issues: 无；详细顺序与补偿见 A-WS-M-T-02 / A-WS-M-S-02
-->

<!-- REQ
req_id: REQ-ED-011
title: Agent 可感知的编辑器会话事实
owner_module: ED
status: active
summary: 为 Agent 1.0 的 P0 阅读、分析和提案提供受限编辑器会话事实：当前 workspace 内的活动文档、已打开标签、每个标签的格式/脏态/保存基准、任务所需的选区或焦点、当前 LogicalState 版本以及关联 Proposal 状态。ED 只提供真实会话投影，不把显示态或磁盘旧内容伪装成当前编辑内容
source_doc: REQ-CORE-011、A-CORE-X-D-02 §3/§4、REQ-AG-009
acceptance: 用户在 A 文档产生未保存修改、切换至 B 后，Agent 仍可按任务读取 A 的 LogicalState、脏态和保存基准，且不会把 B 或 A 的 DiskState 混入；选区仅在任务需要时供给并带所属文档/锚；关联 Proposal 的 pending、冲突、失效与保存状态可被如实呈现；跨 workspace 或跨 AgentSession 不泄露标签、选区或未保存内容
open_issues: 会话快照字段、刷新时机和跨模块事件归 ED/AG/SYS DESIGN Scope；本 L1 不定义 store、IPC 或序列化结构
-->

<!-- REQ
req_id: REQ-ED-012
title: 原生 DOCX 只读与 BDocx 编辑副本呈现
owner_module: ED
status: active
summary: 原生 DOCX 只经 PRV 进入只读预览、读取和分析，不建立 ED 编辑会话。用户或 Agent 请求编辑时，新 BDocx 编辑副本按 CreateDocumentBundle 创建；资源确认并创建目标磁盘副本后，ED 以 HTML 编辑模型打开并投影初始内容 Diff，以模拟 Word 分页呈现，保存目标仍为新的 DOCX
source_doc: REQ-CORE-004、A-CORE-X-D-02 §3/§4
acceptance: 原生 DOCX 没有可编辑标签、样式/表格/图片写入工具或原地 save；副本资源确认创建后才出现可编辑标签，初始内容以普通 Diff 投影；副本编辑时呈现模拟 Word 分页，保存不改变原生 DOCX；初始 Diff 拒绝或失效遵循 CreateDocumentBundle 三重基线规则
open_issues: 副本命名、来源关联、处理身份和 Pandoc/HTML/分页实现契约归 ED/DOC/WS DESIGN Scope；已确认样式操作的技术映射、能力开关与真实验证矩阵归 ED/DOC DESIGN Scope
-->

<!-- REQ
req_id: REQ-ED-013
title: WorkspaceOperation 执行前置与标签协调
owner_module: ED
status: active
summary: WorkspaceOperationPlan 进入 DecisionRequest 时，ED 提供受影响打开标签、脏态、LogicalState 与有效内容 Diff 清单；删除、重命名或移动涉及脏标签时阻断执行。确认并成功修改 DiskState 后，ED 才按真实结果关闭标签或更新路径，并使相关内容 Diff 失效
source_doc: REQ-CORE-012、A-CORE-X-D-02 §3/§4、A-WS-M-D-01（REQ-WS-008）
acceptance: 确认前标签路径不变；脏标签存在时资源操作不得执行；有效内容 Diff 作为确认影响展示但在确认前不失效；成功后标签路径/关闭状态与磁盘一致，受影响内容 Diff 以 resource_baseline_changed 失效；执行失败或补偿完成时标签按最终真实状态同步，不得显示暂定路径冒充磁盘结果
open_issues: 无；事件与状态机见 A-WS-M-T-02 / A-WS-M-S-02
-->

<!-- REQ
req_id: REQ-ED-014
title: 最小格式转换目标副本的编辑器会话
owner_module: ED
status: active
summary: TXT→Markdown、Markdown→TXT 的转换目标副本经 CreateDocumentBundle 确认并创建磁盘目标后，ED 自动打开标签并投影转换结果初始内容 Diff；来源文档不被改写或关闭。目标标签清晰呈现输入/输出格式、目标路径和 Markdown→TXT 的有损提示
source_doc: REQ-CORE-013、A-CORE-X-D-02 §3/§4、A-DE-M-D-01（REQ-DE-006）、A-DOC-M-D-01（REQ-DOC-006）
acceptance: 确认或资源创建失败时不出现目标标签；资源创建后目标立即是已存在的磁盘节点，初始内容 Diff 接受不再次创建文件，保存写入既有目标；Markdown→TXT 的有损边界可见；目标手动或后续代理编辑使旧卡失效但目标标签/LogicalState 保留；HTML、通用 DOCX、批量或原地转换不产生本会话
open_issues: 转换目标的标签标识、格式/路径/损失提示、来源快照呈现、提案失效事件与保存失败同步归 ED/DE/DOC/WS/AG/SYS DESIGN Scope；本 L1 不定义 UI、状态字段或接口
-->

<!-- REQ
req_id: REQ-ED-015
title: 删除/恢复前置裁决与受影响标签处理
owner_module: ED
status: active
summary: 对已保存文件/文件夹删除或恢复，ED 在 WorkspaceOperationPlan 决策前提供受影响标签事实；任一受影响会话有未保存内容即阻断删除/重命名/移动。DecisionRequest 确认并由 WS 成功修改 DiskState 后，ED 按真实磁盘结果关闭、重建或更新标签
source_doc: REQ-CORE-014、A-CORE-X-D-02 §3/§4、A-DE-M-D-01（REQ-DE-007）
acceptance: 确认前不关闭标签、不丢内容；任何脏会话未决时不得执行；成功删除后标签、路径和可编辑性不伪装目标仍存在，成功恢复后按真实资源重建会话；恢复冲突不自动覆盖；执行失败或补偿后标签严格同步最终磁盘状态；CreateDocumentBundle 初始 Diff 的安全拒绝继续按 REQ-ED-010
open_issues: 无；受影响标签和事件契约见 A-WS-M-T-02 / A-WS-M-S-02
-->

<!-- REQ
req_id: REQ-ED-016
title: 多个既有文档的独立编辑器会话
owner_module: ED
status: active
summary: 对已确认并冻结的多既有文档目标清单，ED 为每个已打开目标独立承载 LogicalState、受控预览、绿色差异、脏态、卡片失效和显式保存结果；ED 不因一个目标冲突、外部变化、结构变化或删除而改写、关闭或阻断其他目标会话。背景目标打开后仍按其自身三分流迁移为 LogicalState 预览
source_doc: REQ-CORE-015、A-CORE-X-D-02 §3/§4、A-DE-M-D-01（REQ-DE-008）
acceptance: 编辑器只为确认冻结清单中的可编辑目标显示多文档写入预览；每个目标的绿增、脏态、保存基准、Proposal 终态与失败提示可独立识别；任一目标失效仅禁用该目标卡，不静默跳过、回滚或污染其他目标；不提供跨文档一键接受、一键保存或共同事务呈现；原生 DOCX 与删除/文件树整理/格式转换混合操作不进入本会话
open_issues: 多目标标签/卡片关联、并发预览与排序、背景目标迁移、逐项保存回执、独立失败/恢复和跨模块事件归 ED/DE/AG/WS/REF/SYS DESIGN Scope；本 L1 不定义 UI、状态字段、批处理或事务接口
-->

## §6 模块边界与约束（边界闭合）

- **职责边界**：ED 只承载富文本编辑运行时 / block 与 blockId / 分页 / 媒体 / decoration 呈现面 / 未保存文档会话
  （§2 E-J1–E-J11），不做格式转换、diff 生成、文件 I/O、预览导出、chat/agent；原生 DOCX 只读，越界即范围外
  （CLAUDE.md §1 范围纪律）。
- **L1/L2 边界**：本文只定契约**边界**；编辑器框架选型、blockId 策略、分页实现、decoration 接口归
  ED L2，代码只追溯 L2（ADU §3.2、APC §2）。
- **与 WS 的边界**：文件读写、save 落盘、workspace 边界守护归 WS（A-WS-M-T-01 WS-FS / SM-WS-01）；
  ED 经 WS 进行 I/O，不直接触达文件系统（X-SEQ-002 / X-FORBID-004）。
- **与 DE 的边界**：diff 生成与提案裁决语义归 DE；ED 提供受控预览、decoration 呈现面、blockId 定位
  与未保存文档标签，按 DE 信号呈现/复位（E2）；面向 AG 的会话事实只读投影归 REQ-ED-011，不授予写入。
- **与 DOC 的边界**：DOCX/Pandoc 格式转换归 DOC；ED 承接转换后内容的富文本编辑呈现（往返协作，
  细节边界 ED-04 延 ED/DOC L2）。
- **与 AG 的边界**：chat/agent 回合归 AG；inline-assist/autocomplete 业务**归属待确认**
  （A-SYS-M-S-01 §3），ED 不抢定，仅提供 decoration 承载（ED-05）。
- **资源操作与多文档边界**：ED 为 WorkspaceOperation 提供标签、脏态、LogicalState 和内容 Diff 影响事实，并在 WS 执行后按真实回执同步；资源决策卡不归 DE。ED 不提供永久清理、批量内容裁决、批量保存或跨文档事务。
- **数据结构边界**：ED 编辑模型若需跨模块传递（如 blockId 供 DE 定位、供 REF 引用锚），其跨模块共享
  结构须经 SYS 数据结构契约层（X-SHARED-001，A-SYS-I-P-01），ED 不私自定义跨模块结构（细节 ED L2）。
- **L4 原料边界**：取材见台账 §4.5（ED：编辑器架构 / block 模型 / 分页系统 / 图文媒体 / docx 能力等，
  均 P1/P2 原料），为原料不为权威，REQ 重新撰写（APC §8）。

## §7 G1 问题暴露清单

按 APC §5 G1 标准（REQ_GAP/BOUNDARY_OPEN/REQ_CONFLICT/FLOW_INCOMPLETE/ACCEPTANCE_MISSING/
NEEDS_HUMAN_DECISION/DESIGN_RISK）。各项均**显式延期**至 G2/G5 并记明 owner+门禁，据此 ED 模块级
G1 具备闭合条件。晋级由人类宣布（APC §2，AI 不得自行宣布晋级）。

| 编号 | 分类 | 状态 | 处置 |
|---|---|---|---|
| ED-01 | NEEDS_HUMAN_DECISION | 显式延期 | TipTap/ProseMirror 版本、扩展集与自定义 schema 选型归 ED L2 |
| ED-02 | DESIGN_RISK | 显式延期 | blockId 生成算法与编辑变更下稳定性策略归 ED L2（影响 diff 定位正确性，与 DE 联动） |
| ED-03 | FLOW_INCOMPLETE | 显式延期 | 分页实现（光标/遮罩方案）归 ED L2，取材台账 §4.5 分页四件 |
| ED-04 | BOUNDARY_OPEN | 显式延期 | docx 富文本编辑与 DOC 格式转换的往返边界细节归 ED/DOC L2 联合 |
| ED-05 | BOUNDARY_OPEN | 显式延期 | decoration 与 DE diff/AG inline-assist 接口契约归 ED L2 + DE/AG；inline-assist/autocomplete 归属待人类确认（A-SYS-M-S-01 §3） |
| ED-06 | ACCEPTANCE_MISSING | 部分关闭 | 定性验收已入 §5 各 REQ-ED-*；量化指标（编辑响应延迟、大文档分页性能）延 G5 NFR 标定（APC §9） |
| ED-07 | IMPLEMENTATION_GAP | 设计已闭合，待实现 | CreateDocumentBundle 的确认后开标签、初始内容 Diff、三重基线拒绝补偿与失效后保留标签须按 REQ-ED-010 和 A-WS-M-T-02 / M-S-02 实现 |
| ED-08 | FLOW_INCOMPLETE | 已识别，待 G2 | TXT↔Markdown 转换目标副本的标签、来源快照提示、损失呈现、提案失效与保存失败恢复须按 REQ-ED-014 联合 AG/DE/DOC/WS/SYS 设计；不得把来源或 BDocx 专用路径误作目标会话。 |
| ED-09 | IMPLEMENTATION_GAP | 设计已闭合，待实现 | WorkspaceOperation 执行前脏标签阻断、执行后真实路径/关闭同步及内容 Diff 失效须按 A-WS-M-T-02 / M-S-02 实现。 |
| ED-10 | DESIGN_RISK | 已识别，待 G2 | 多既有文档目标冻结后的标签/卡片关联、逐目标预览/失效/保存和背景目标迁移，须按 REQ-ED-016 联合 DE/AG/WS/REF/SYS 设计；不得提供批量裁决或事务。 |

> 术语：编辑 block、blockId、分页、decoration 等 ED 主责核心概念将在 ED L2 评估注册 TERM（block /
> blockId 作为跨模块定位锚，跨 session 易漂移且代码有实体承载，候选 TERM），本文 L1 不注册 TERM
> （ADU §3.2，TERM 在 L2 技术设计层注册）。

## §8 后续动作

1. 待人类宣布 ED 模块级 G1 闭合后，进入 ED G2（L2 技术设计，规则 SSOT），按路线图 P2：
   - 富文本编辑运行时与框架选型（承接 REQ-ED-001，解除 ED-01/ED-04）；
   - block 模型与 blockId 稳定定位（承接 REQ-ED-002，解除 ED-02，与 DE diff 定位联动）；
   - 分页系统与布局（承接 REQ-ED-003，解除 ED-03，取材台账 §4.5 分页四件）；
   - 图文媒体能力（承接 REQ-ED-004）；
   - 编辑器装饰层接口契约（承接 REQ-ED-005，与 DE/AG 联合解除 ED-05）；
   - CreateDocumentBundle 与转换目标会话、初始内容 Diff 协同（承接 REQ-ED-010/014，与 DE/DOC/WS 联合解除 ED-07/08）；
   - 可恢复删除前置与多既有文档逐项会话（承接 REQ-ED-015/016，与 DE/AG/WS/REF 联合解除 ED-09/10）。
2. G2 各文档经 REQ_MAP 把 REQ-ED-* 映射为 RULE/CHAIN/CONSTRAINT/TERM；如有状态化编辑链路（如 docx
   编辑态切换），入 ED M-S 并登记 A-SYS-M-S-01 总图。
3. ED 职责或编辑运行时边界变更先经 APC 授权再回写本文（APC §13）。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-10 | v1.0 | 初版：ED 编辑器模块级 L1；声明 REQ-ED-001~005（富文本编辑运行时 / block 模型与 blockId 定位 / 分页系统 / 图文媒体能力 / 编辑器装饰层供 diff 呈现），承接 APC §4.1 ED 职责与 REQ-CORE-002/004/009；代表性流程 E1–E3；边界闭合（含与 WS/DE/DOC/AG 边界，TipTap 选型延 ED L2）与 G1 暴露清单（ED-01~06 显式延期/部分关闭；inline-assist/autocomplete 归属待人类确认） |
| 2026-07-03 | v1.1 | 编辑器专项优化承接（A-ED-X-L-02 批B，人类裁定 D-1/D-2/D-3）：追加 REQ-ED-006 字符级定位与选区引用（会话内锚，跨会话显式延后）、REQ-ED-007 agent 可读可编辑的样式协议（落盘路径 A 默认+B 限内联）、REQ-ED-008 .html 富文本编辑接入；技术契约分别归 ED L2 BR-ED-DATA-004/005 与 BR-ED-DATA-002 修订 |
| 2026-07-04 | v1.2 | 术语统一清扫（承 2026-07-04 人类三轮裁决）：正文禁用别名替换——内联辅助→局部修改、自动补全→辅助续写、对话编辑→Diff Edit、Chat↔Build 接管→构建移交/回落、chat/chat-agent 回合→代理回合；文件名与变更记录历史行不改写 |
| 2026-07-18 | v1.3 | 人类裁决替代 2026-07-03 Markdown 纯文本路径：修订 REQ-ED-001，要求 Markdown 经格式 Codec 进入结构化 LogicalState、默认终态富文本呈现、未编辑不改写、编辑后语义往返且非支持结构不得静默丢失；纯文本路径保持不变 |
| 2026-07-28 | v1.4 | BDocx 表格课题 A/B：新增 REQ-ED-009，锁定基础表格交互、受控 TableOperation、单元格纯色底纹、结构化 Diff 与 DOCX 往返验收边界。 |
| 2026-07-28 | v1.5 | BDOCX-IMAGE-INTERACTION-01：收紧 REQ-ED-004 验收，明确真实拖宽、独立图片段落文档流移动/删除、混排保护、脏态及 BDocx 保存重开结果；撤下不能由 Pandoc 持久的 inline/block 能力声明。 |
| 2026-07-29 | v1.6 | BDOCX-IMAGE-AGENT-EDIT-01：REQ-ED-004 纳入 workspace 项目图片的代理插入与当前图片实例删除、结构化 Diff、源文件保护和保存重开验收；继续排除外部导入、代理缩放/移动及自由布局。 |
| 2026-07-30 | v1.7 | 图片编辑闭合审计：REQ-ED-004 同步代理缩放/上下移动/独立段落对齐、同图组合操作，以及项目树/Finder/粘贴受控导入；排除项收敛为视觉识别和高级布局。 |
| 2026-08-07 | v1.8 | DOC-SYNC-01 落地说明：ED-SELECTION-PERSIST-01 选区失焦持久化与 ED-SELECTION-DRAG-01 选区拖动不错位，为 REQ-ED-006「字符级定位与选区引用」的交互落地（技术细节见 A-ED-M-T-01 v1.59/v1.61）；失焦持久化不改变锚语义、跨文档生命周期不残留，拖动不篡改原选区、拖出编辑器保持引用语义。 |
| 2026-08-13 | v1.9 | 依据 REQ-CORE-010 与 D-NEW-001~008，新增 E-J6/REQ-ED-010；将 diff 呈现明确为受控预览进入 LogicalState，接受不再写盘，补齐新建预览自动开标签、提案失效不关标签、拒绝丢弃与未保存关闭提醒的需求边界。 |
| 2026-08-13 | v1.10 | DIFF-STATE-COMPASS-01：REQ-ED-005 增加背景 pending 打开后的同 payload 绿色预览迁移；预览后卡片保持 pending，由 DE 保留用户裁决入口。 |
| 2026-08-13 | v1.11 | AG-1.0-R2-REQUIREMENTS-01：新增 REQ-ED-011，冻结 CAP-03 的 P0 会话事实边界：活动/打开标签、脏态、保存基准、任务所需选区、LogicalState 与关联 Proposal 可被受限读取，且严格隔离 workspace/AgentSession。 |
| 2026-08-13 | v1.12 | AG-1.0-R3-REQUIREMENTS-01：新增 REQ-ED-012，原生 DOCX 改为只读预览/分析，编辑只创建 BDocx 处理副本；表格与图片限定为 HTML/BDocx 的 P1 受控能力，BDocx 明确不是文件格式。 |
| 2026-08-13 | v1.13 | R3 自检收紧：REQ-ED-001 与 REQ-ED-007 不再把 HTML/BDocx 的任一具体样式操作表述为已开放；样式投影可读不等于可写授权。 |
| 2026-08-14 | v1.14 | AG-1.0-R3-STYLE-REQUIREMENTS-01：确认 HTML/BDocx 的字符基础样式 P0，以及颜色/突出显示、段落样式、字号与字体族 P1；字体族含 BDocx，真实验证是实现后的开放门槛。 |
| 2026-08-14 | v1.15 | R3 语义收紧：REQ-ED-001/007 明确全部确认的 P0/P1 样式项均须在实现后真实保存重开验证，不能只验证 P1。 |
| 2026-08-14 | v1.16 | AG-1.0-R4-STRUCTURE-REQUIREMENTS-01：新增 REQ-ED-013，单文件重命名/移动先更新逻辑文件树与标签暂定路径，保留内容/脏态；结构基线推进使关联内容 Proposal 失效，显式保存前不得伪报磁盘路径。 |
| 2026-08-14 | v1.17 | AG-1.0-R4-CONVERSION-REQUIREMENTS-01：新增 REQ-ED-014；TXT→Markdown、Markdown→TXT 的目标副本预览成功后打开未保存标签，接受不写盘、目标继续编辑使旧卡失效；Markdown→TXT 的有损提示可见，HTML 与通用 DOCX 转换不进入会话。 |
| 2026-08-14 | v1.18 | AG-1.0-R4-DELETE-MULTIDOC-REQUIREMENTS-01：新增 REQ-ED-015/016；删除保存前保留并逐项解决受影响脏会话，删除/恢复后标签不伪装磁盘状态；多个既有文档只在确认冻结目标清单后逐项承载独立 LogicalState、预览、失效和保存，不提供批量裁决或事务。 |
| 2026-08-21 | v1.19 | 工作区资源操作基线重构：REQ-ED-010/012/013/014/015 对齐 CreateDocumentBundle、DecisionRequest 前置裁决、脏标签阻断、执行后真实标签同步与内容 Diff 失效。 |
