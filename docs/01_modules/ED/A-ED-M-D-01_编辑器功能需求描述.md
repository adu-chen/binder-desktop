---
文档编号：ED-M-D-01
文档状态：A
负责模块：ED
文档职责：ED 编辑器模块级功能需求描述——声明模块级 REQ-ED-*，承接 APC §4.1 ED 职责与 REQ-CORE-002/004/009，界定富文本编辑运行时 / block 模型与 blockId 定位 / 分页系统 / 图文媒体能力 / 编辑器装饰层（供 diff 呈现）
上游约束：A-CORE-C-D-01_产品概述.md（REQ-CORE-002/004/009）、A-CORE-C-P-01_APC.md（§4.1 ED 职责「编辑器运行时 TipTap，md/docx 富文本编辑」、§9 离线边界）、A-SYS-C-T-01_系统总体架构.md（§3 ED 职责、§4 ED→WS 依赖）、A-WS-M-T-01_工作台技术设计.md（文件 I/O 经 WS 物理边界、WS-FS）、A-SYS-M-S-01_状态机总图.md（autocomplete/inline-assist 归属待确认）、A-CORE-X-L-01_总体开发路线图.md（P2）、ADU.md（§3.2 REQ 规范）
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
| E-J1 | 富文本编辑运行时：md / docx 内容的富文本编辑（基于 TipTap） | APC §4.1、REQ-CORE-004/009 |
| E-J2 | block 模型与稳定 blockId 定位（结构化内容锚，供 diff/引用/分页） | APC §4.1、REQ-CORE-002 |
| E-J3 | 分页系统与布局（分页模式、光标与遮罩、打印友好呈现） | APC §4.1、REQ-CORE-004 |
| E-J4 | 图文媒体能力（图片等媒体的插入、呈现与处理） | APC §4.1 |
| E-J5 | 编辑器装饰层（decoration）：承载 diff 呈现/审阅与局部修改 | REQ-CORE-002 |

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
- 编辑器装饰层（decoration）：为 DE 的 diff 呈现/审阅、AG 的局部修改提供呈现承载面。

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
| 1 | WS | workspace 内文档内容 | ED 按磁盘格式解析为富文本编辑模型（Markdown 经 Markdown Codec，b-docx 经 DOC） | 可编辑文档 | 2 | 内容无法无损解析→提示并进入源码保护/只读降级，不静默丢失 |
| 2 | 用户 | 编辑操作 | ED 更新编辑模型并标记脏 | 脏编辑态 | 3 | 离线时编辑照常可用（REQ-CORE-009） |
| 3 | 用户/ED | save 触发 | ED 交 WS 落盘（A-WS-M-T-01 SM-WS-01） | 已保存 | 终态 | 落盘失败由 WS 回报，ED 保持脏态可重试 |

**E2 · diff 装饰呈现与裁决协同**（E-J5，承接 REQ-CORE-002）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | DE | 生成的 diff | ED 在 decoration 层按 blockId 定位呈现 diff | 可视 diff 呈现 | 2 | 定位锚失效→回报 DE，不静默错位呈现 |
| 2 | 用户 | 接受/拒绝 | 由 DE 终态机裁决（ED 仅呈现与转达交互） | accepted/rejected/expired | 3 | 终态归 DE |
| 3 | DE→WS | accepted 信号 | WS 落盘（SM-WS-02），ED 清除该 diff decoration | 已落盘 + 呈现复位 | 终态 | rejected/expired→ED 清除装饰，文档不变 |

> E2 在 L1 仅锁边界：ED 提供 decoration 呈现面与 blockId 定位，diff 终态归 DE、落盘归 WS；
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
title: 富文本编辑运行时（md/docx）
owner_module: ED
status: active
summary: 提供基于 TipTap 的富文本编辑运行时，支持 workspace 内 Markdown/b-docx 内容按格式解析为结构化 LogicalState、以终态样式编辑并安全序列化，编辑能力离线可用；落盘经 WS
source_doc: APC §4.1、REQ-CORE-004、REQ-CORE-009
acceptance: Markdown 的 H1-H6、强调、列表、引用、代码、链接、图片、表格、分隔线等受支持语法加载后以终态富文本呈现且源码标记不作为正文显示；未编辑 Markdown 不改写，编辑后保存保持支持子集语义等价，无法无损处理时不得静默保存；b-docx 可加载为富文本并编辑；纯文本行为不变；离线编辑可用；save 经 WS 可靠落盘（A-WS-M-T-01）
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
title: 图文媒体能力
owner_module: ED
status: active
summary: 支持图片等媒体在编辑器内的插入、呈现与处理，媒体资源经 WS 限定在 workspace 内
source_doc: APC §4.1、REQ-CORE-001（workspace 边界）
acceptance: 可在编辑器内呈现和选择图片；图片宽度可通过可见手柄按固有比例调整；独立图片段落可经邻近控件在文档流内上移、下移或删除，混排文字中的行内图片不得被当作整块重排；代理可把经校验的 workspace 项目图片插入稳定 Block 前后，或对当前文档图片实例执行删除、受限宽度缩放、上下移动和独立段落左/中/右对齐，操作经结构化 Diff 裁决且不改写项目源文件；同图缩放/对齐/移动可单批原子组合，删除独占；用户从项目树、Finder 拖入或粘贴的 PNG/JPEG 经受控复制/复用后插入独立图片段落；成功操作产生脏态，BDocx 保存重开后图片资源、alt、尺寸、顺序、对齐和删除结果保持；媒体资源不越出 workspace（WS X-FORBID-004）
open_issues: 图片视觉识别/OCR、文字环绕、行内混排重排、浮动或绝对定位、任意坐标布局、裁剪、旋转和独立高度不在当前支持子集；Pandoc 不保留自定义 data-display，故不把 inline/block 按钮声明为 BDocx 持久能力
-->

<!-- REQ
req_id: REQ-ED-005
title: 编辑器装饰层（供 diff 呈现与局部修改）
owner_module: ED
status: active
summary: 提供编辑器装饰层（decoration），按 blockId 承载 diff 呈现/审阅与局部修改的可视呈现；diff 终态归 DE、局部修改业务归属待确认，ED 只保证装饰面可承载
source_doc: REQ-CORE-002、A-SYS-M-S-01（autocomplete/inline-assist 归属待确认）
acceptance: decoration 可按 blockId 定位呈现 diff 与局部修改；裁决终态由 DE 驱动，ED 据信号呈现/复位
open_issues: decoration 与 DE diff/AG inline-assist 的接口契约归 ED L2 + DE/AG 联合（ED-05）；inline-assist/autocomplete 归属待人类确认（A-SYS-M-S-01 §3）
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
title: agent 可读可编辑的样式协议
owner_module: ED
status: active
summary: 文档样式（字符 marks / 块属性）以机器可读结构投影供 agent 读取；样式修改以受控样式操作进入编辑模型，agent 落盘路径按裁定 D-3 分流（回合产出经 DE diff 裁决、显式局部修改视为用户编辑）
source_doc: REQ-CORE-002、2026-07-03 人类需求 #3 与裁定 D-3（A-ED-X-L-02 §2：A 默认 + B 限内联）
acceptance: agent 可读到结构化样式投影；样式 diff 经 DiffCard 可裁决落盘；显式内联样式操作即改即脏；不存在绕过裁决的回合直改路径（X-FORBID-006）
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
title: BDocx 表格语义编辑与单元格底纹
owner_module: ED
status: active
summary: BDocx 表格的人工交互与代理修改共用受控表格操作协议；支持基础结构操作和单元格纯色底纹，代理建议必须进入 Diff Edit 裁决
source_doc: REQ-CORE-002、REQ-CORE-004、2026-07-28 人类确认课题 A/B
acceptance: 表格可执行行列增删、表头、合并拆分、文档流移动和列边界拖拽；所选一个或多个单元格可设置或清除纯色底纹；代理可经 list_tables→table_ops 产生结构化建议，底纹差异按坐标和前后色值准确呈现；富内容单元格设置底纹时内容树不变；保存并重开 BDocx 后受支持底纹保持
open_issues: 列宽数值输入/均分列、行高、浮动或绝对定位、渐变/主题/图案填充、嵌套表格和列宽 DOCX 往返保证不在当前范围
-->

## §6 模块边界与约束（边界闭合）

- **职责边界**：ED 只承载富文本编辑运行时 / block 与 blockId / 分页 / 媒体 / decoration 呈现面
  （§2 E-J1–E-J5），不做格式转换、diff 生成、文件 I/O、预览导出、chat/agent；越界即范围外
  （CLAUDE.md §1 范围纪律）。
- **L1/L2 边界**：本文只定契约**边界**；编辑器框架选型、blockId 策略、分页实现、decoration 接口归
  ED L2，代码只追溯 L2（ADU §3.2、APC §2）。
- **与 WS 的边界**：文件读写、save 落盘、workspace 边界守护归 WS（A-WS-M-T-01 WS-FS / SM-WS-01）；
  ED 经 WS 进行 I/O，不直接触达文件系统（X-SEQ-002 / X-FORBID-004）。
- **与 DE 的边界**：diff 生成与终态转移语义归 DE；ED 提供 decoration 呈现面与 blockId 定位，按 DE
  信号呈现/复位（E2）。
- **与 DOC 的边界**：DOCX/Pandoc 格式转换归 DOC；ED 承接转换后内容的富文本编辑呈现（往返协作，
  细节边界 ED-04 延 ED/DOC L2）。
- **与 AG 的边界**：chat/agent 回合归 AG；inline-assist/autocomplete 业务**归属待确认**
  （A-SYS-M-S-01 §3），ED 不抢定，仅提供 decoration 承载（ED-05）。
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

> 术语：编辑 block、blockId、分页、decoration 等 ED 主责核心概念将在 ED L2 评估注册 TERM（block /
> blockId 作为跨模块定位锚，跨 session 易漂移且代码有实体承载，候选 TERM），本文 L1 不注册 TERM
> （ADU §3.2，TERM 在 L2 技术设计层注册）。

## §8 后续动作

1. 待人类宣布 ED 模块级 G1 闭合后，进入 ED G2（L2 技术设计，规则 SSOT），按路线图 P2：
   - 富文本编辑运行时与框架选型（承接 REQ-ED-001，解除 ED-01/ED-04）；
   - block 模型与 blockId 稳定定位（承接 REQ-ED-002，解除 ED-02，与 DE diff 定位联动）；
   - 分页系统与布局（承接 REQ-ED-003，解除 ED-03，取材台账 §4.5 分页四件）；
   - 图文媒体能力（承接 REQ-ED-004）；
   - 编辑器装饰层接口契约（承接 REQ-ED-005，与 DE/AG 联合解除 ED-05）。
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
