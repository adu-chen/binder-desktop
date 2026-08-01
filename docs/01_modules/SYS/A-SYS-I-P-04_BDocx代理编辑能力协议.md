---
文档编号：SYS-I-P-04
文档状态：A
负责模块：SYS
文档职责：BDocx 代理编辑能力的跨模块协议与单一运行时声明——统一描述编辑器支持、代理可用性、Diff 呈现等级、DOCX 往返等级、动态前置条件与执行前校验；承接 REQ-SYS-002
上游约束：A-SYS-M-D-01_系统底座功能需求描述.md（REQ-SYS-002）、A-SYS-I-P-01_数据结构定义.md（BR-SYS-DATA-002、X-SHARED-001）、A-AG-M-T-01_代理技术设计.md（BR-AG-STATE-002、BR-AG-DATA-001/002）、A-ED-M-T-01_编辑器技术设计.md（BR-ED-DATA-004/005/007）、A-DOC-M-T-01_文档转换技术设计.md（BR-DOC-DATA-002）、A-DE-M-T-01_DiffEdit技术设计.md（BR-DE-DATA-002）、ADU.md（§3）
直接承接：机器可读 BDocxCapabilityManifest、AG 当前文档提示词投影、propose_edit 工具 Schema、Diff 生成前静态校验与 ED 防御性校验
使用边界：本文只声明既有 BDocx 能力及其真实等级，不把人工能力自动开放给代理，不提升 DOCX 样式往返等级，不取代 list_tables/list_images 动态快照、fingerprint 与来源哈希，不定义 Diff UI 或保存状态机
变更要求：能力项、操作名、字段约束、代理可用性、Diff 等级或往返等级任一变化，必须先更新本文及机器清单版本；运行时投影不得另建手写能力枚举；TERM/RULE ID 一经引用不可更改
治理成熟度：draft
---

# BDocx 代理编辑能力协议（SYS · L2 接口协议）

## 0. Terminology Registry

<!-- TERM
term_id: TERM-SYS-002
chains: AG-TURN, AG-TOOL, ED-EDIT, DE-DIFF, DOC-CONVERT
zh: BDocx能力清单
en: BDocxCapabilityManifest
forbidden: BDocx能力表, 代理能力表, 编辑能力列表, 能力白名单
-->
跨 SYS/AG/ED/DE/DOC 的版本化机器契约：对每项 BDocx 编辑能力同时声明编辑器是否支持、代理是否可用、
Diff 呈现等级、DOCX 往返等级、动态前置条件和参数约束。运行时提示词、工具 Schema 与执行前校验必须
从同一份清单投影，禁止各自维护第二套操作名或字段枚举。

## §1 协议目标与边界

BDocx 代理编辑的“可用”不是单一布尔值。一项编辑器 UI 能执行的操作，可能尚未开放给代理；一项代理
可生成的操作，可能只具摘要级 Diff；一项在编辑器内可见的样式，保存为 DOCX 后也可能只是尽力保留。
本协议用同一个能力条目并列描述这些维度，避免模型把“编辑器存在”误解为“代理可安全调用且往返保真”。

本协议不取代各 owner 的业务真源：

- StyleOp、CharAnchor、TableOperation 和 ImageOperation 的编辑语义仍归 ED。
- StyleSubset 五档及真实检测报告仍归 DOC；清单必须逐项引用，不能自行升档。
- Diff 载荷和呈现语义仍归 DE；清单只声明当前呈现等级。
- 工具循环、权限与错误回注仍归 AG。
- 跨模块字段、版本与投影一致性归 SYS（本文）。

## §2 BDocxCapabilityManifest 结构

机器清单必须至少包含：

| 字段 | 语义 | 约束 |
|---|---|---|
| manifestId | 清单稳定标识 | 版本间不变 |
| version | 语义版本 | 任一能力或参数契约变化必须递增 |
| editableKind / contentFormat | 适用编辑模型 | 固定为 bdocx / html |
| styleOps | StyleOp 批上限、rangeUnit、mark、mark attrs、块属性 | mark 名必须使用编辑器真实名称；rangeUnit 固定为 utf16_code_unit |
| tableOps | TableOperation 名、必填字段、字段类型 | 静态形状；动态可用性仍由 list_tables 给出 |
| imageOps | ImageOperation 名、必填字段、字段类型、项目来源约束 | 静态形状；当前图片实例、Block 锚与项目资源仍由 list_images 给出 |
| manualOnly | 仅人工可用或未开放给代理的能力 | 不得进入代理工具 enum |
| diffFidelity | 文本、样式、表格、图片的当前 Diff 等级 | 不得高于 DE 实际呈现 |
| roundtrip | StyleSubset 五档全集 | 必须与 DOC classify_style 一致 |

应用启动后的清单摘要还必须包含对清单原始字节计算的 SHA-256。`version + sha256` 用于诊断同一构建内
提示词、工具 Schema 与校验器是否消费了相同清单；SHA-256 不是第二个手工版本字段。

## §3 能力维度与枚举

**代理可用性**

- `available`：可进入工具 Schema，并在生成 Diff 前通过静态和运行时校验。
- `dynamic`：静态清单允许，但必须再满足当前会话投影（例如表格 capabilities、fingerprint、
  moveDirections、富内容保护）。
- `manual_only`：编辑器人工入口存在，代理工具不得声明或接受。
- `unsupported`：当前编辑模型不承诺执行。

**Diff 呈现等级**

- `block`：块级 before/after。
- `summary`：仅操作摘要；不表示字符级视觉差异已经实现。
- `structured`：以结构化 before/after 快照呈现。
- `none`：当前没有可审阅呈现，不得开放为代理写操作。

**DOCX 往返等级**

唯一允许值为 `verified / required_pending / best_effort / template_inherited / unsupported`，逐项对齐
`BR-DOC-DATA-002`。未知特性必须保守为 `unsupported`。

## §4 当前能力基线（manifest 1.5.0）

### §4.1 文本与块

- BDocx 内容编辑仅开放 `block_ops`，Diff 为 `structured`；`replace_block` 与 `replace_document`
  在 BDocx 上均为 `none`，只保留给纯文本格式。
- 代理必须先读取当前活动 ProseMirror LogicalState 的层级块投影；容器块和原子块仅提供
  `parent_block_id/depth/ordinal` 语境，不提供通用写能力。只有 `role=content` 且
  `capabilities` 含 `replace_node` 的块可作为目标。
- 每个 `replace_node` 必须原样带回生成基线 `sessionVersion/contentHash`、稳定 `blockId`、当前
  `fingerprint` 和 `expectedNodeType`；`afterNode` 从当前 `node_json` 复制，保持节点类型、blockId
  和 attrs，只允许受控 text/hardBreak 与已声明 mark。多个相关块在同一批次原子校验、原子应用。
- 后端在 PendingDiff 前补齐精确 `beforeNode`；任何基线、锚、类型、fingerprint、attrs、Schema
  或能力不匹配均使整批零写入。不得从 HTML 字符串重新推断富文档结构。
- 原始 HTML 不是独立结构编辑能力；不得用 replace_block/replace_document 绕过 BlockOperation、
  TableOperation 或 ImageOperation。

### §4.2 StyleOp

允许的真实 mark 名：

- 无 attrs：`bold / italic / underline / strike / subscript / superscript / code`。
- `highlight`：可无 attrs，或仅带 `color=#RRGGBB`。
- `link`：必须带安全的非空 `href`，禁止 `javascript:` / `data:`。
- `textStyle`：至少带 `color / fontSize / fontFamily` 之一，不得带未知 attrs；颜色为 `#RRGGBB`，
  字号为正数加 `pt/px`，字体名为有限非空字符串。

允许的块属性只有 `textAlign=left|center|right|justify`，且目标必须是支持该属性的段落或标题块。
字符范围是块内纯文本半开区间 `[start,end)`，偏移单位为 UTF-16 code unit（与 JavaScript
`String.length`、ProseMirror 位置换算一致），要求 `0 <= start < end <= block.text.length`。一个批次
任一项出现未知 mark、未知 attrs、非法参数、blockId 缺失或范围越界，整批必须在 PendingDiff 生成前拒绝。

往返等级按具体特性声明，不按“StyleOp 整体”笼统承诺：bold/text_color 当前为 verified；
italic/underline/paragraph_align 为 required_pending；font_size/font_family 为 best_effort；
strike/highlight/subscript/superscript/code/link 当前按 unsupported 如实告知。样式 Diff 当前仅为 summary。

### §4.3 TableOperation

静态允许的操作为：

`insert_row / delete_row / insert_column / delete_column / set_header_row / merge_cells / split_cell /
set_cell_text / set_cell_background / move_table`。

每种操作的字段形状由 manifest 声明并投影到工具 JSON Schema。静态通过不代表当前表格可执行：

1. 必须先 `list_tables`。
2. `base.sessionVersion/contentHash` 与 `target.tableId/fingerprint` 必须原样带回并匹配当前会话。
3. 操作名必须出现在目标表格的动态 `capabilities`；移动方向还必须出现在 `moveDirections`。
4. `set_cell_text` 只允许 `plain_text` 单元格；`set_cell_background` 只接受合并单元格左上锚。
5. 任一项失败不得生成 PendingDiff。

表格 Diff 当前为 structured。基础表格结构仍为 required_pending；纯色单元格底纹为 verified；
合并单元格往返为 unsupported，清单不得因编辑器可执行合并而声称保存重开保真。

### §4.4 ImageOperation

静态允许的操作为
`insert_image / delete_image / resize_image / move_image / set_image_layout`，且只对当前活动
BDocx HTML 编辑器开放：

1. 必须先 `list_images`，并把 `base.sessionVersion/contentHash` 原样带回。
2. `insert_image` 只接受当前 workspace 内 `list_images.assets` 返回的相对路径与内容哈希；禁止远程
   URL、data URL、绝对路径、workspace 外路径与未经格式/magic 校验的文件。
3. 插入位置只能是稳定 Block 的 `before/after`，结果必须是独立图片段落；可选 `alt` 与受限正整数
   `width`，不接受原始 HTML、浮动、绝对定位或任意坐标。
4. `delete_image / resize_image / move_image / set_image_layout` 必须命中 `list_images.images`
   返回的稳定 `imageId + 当前 fingerprint`；`resize_image` 的 `width` 必须落在动态
   `resizeBounds`，`move_image` 的方向必须在 `moveDirections`，布局只允许独立顶层图片段落的
   `left/center/right` 对齐。
5. 同一图片在单批中可各含至多一个 `resize_image`、`set_image_layout`、`move_image`，按缩放→对齐→
   移动的固定顺序在单一事务内执行；重复同类操作拒绝，`delete_image` 必须独占。整批只校验一次原始
   fingerprint；任一当前图片实例、动态能力、Block 锚、项目资源哈希或基线失效，整批不得生成
   PendingDiff。
6. 删除只作用于文档图片节点；任何操作都不得删除、移动或改写项目源文件。
7. `list_images` 的相邻文本只用于文档流定位，不表示模型读取了图片像素。模型不得把语境推断描述成
   视觉识别；workspace 内非受控 PNG/JPEG 可经 `import_image_asset` 复制准备，无法安全提案时用
   `edit_blocked` 结构化报告阻断。

图片 Diff 为 structured；受支持的嵌入图片资源、alt、显式尺寸、实例数量和文档流位置/删除结果的
DOCX 往返等级为 verified。图片段落对齐只有在 HTML→DOCX→HTML 与 OOXML `w:jc` 证据闭合后才可
由 `required_pending` 升为 `verified` 并进入 `set_image_layout`；该等级不包含文字环绕、行内混排
重排、浮动/绝对定位、任意坐标拖放、裁剪、旋转或独立高度。用户从系统拖入本地 PNG/JPEG 时，文件
先经 DOC 受控复制/复用进入项目图片根，再以独立图片段落插入；源文件保持不变。

### §4.5 人工专属能力

以下能力可在编辑器人工入口存在，但不开放给代理：

- 拖拽调整列宽。
- 图片文字环绕、行内混排重排、浮动/绝对定位、任意坐标拖放、裁剪、旋转与独立高度设置。
- 标题节点类型切换、列表/引用/代码块结构切换。
- 删除整张表格。
- 表格任意位置拖动、浮动/绝对定位、列宽数值输入/均分列、行高。

人工专属项必须进入当前 BDocx 能力提示，使模型知道“用户可手工操作但代理不能执行”，同时不得进入
`propose_edit` 的操作 enum。

## §5 三个强制运行时投影

1. **提示词投影（AG-TURN）**：当前 `AgentEditableSnapshot.editable_kind=bdocx` 时，注入格式、
   manifestId/version/sha256、代理可用能力、人工专属能力、Diff 等级与非 verified 往返风险。
2. **工具 Schema 投影（AG-TOOL）**：StyleOp mark/attrs、TableOperation/ImageOperation 分支和条件
   必填字段必须由 manifest 生成；禁止宽泛 `object` 或自由文本替代约束。
3. **执行前校验（AG-TOOL→DE-DIFF）**：模型参数必须按同一 manifest 校验；StyleOp 再校验当前块锚和
   字符范围，TableOperation 再校验动态表格快照，ImageOperation 再校验当前图片/Block 快照与项目
   资源哈希。失败回注模型，不能持久化 PendingDiff。

ED 对历史 Diff/IPC 载荷仍做防御性校验，但它不是允许“先生成无效 Diff、后在编辑器失败”的替代路径。

<!-- RULE
rule_id: BR-SYS-DATA-006
主链路: AG-TOOL
域: DATA
-->
BDocxCapabilityManifest 必须是 BDocx 代理编辑能力的唯一版本化机器来源；代理提示词、工具 JSON Schema
和 PendingDiff 生成前校验必须消费同一 manifest 版本与内容哈希，不得各自维护 BlockOperation、mark、StyleOp、
TableOperation、ImageOperation 或能力等级枚举。清单声明必须同时区分代理可用性、人工专属能力、Diff 呈现等级与 DOCX
往返等级；未知能力或未知样式保守拒绝，任何静态参数、当前锚或动态能力校验失败均不得进入 DE-DIFF
持久化。

## §6 错误与失败语义

| 错误 | 触发 | retriable |
|---|---|---|
| `AG_CAPABILITY_MANIFEST_INVALID` | 包内清单无法解析、版本/ID/枚举/唯一性或 DOC tier 对齐失败 | false |
| `AG_STYLE_OPERATION_INVALID` | StyleOp 名称、字段、attrs 或范围非法 | false |
| `AG_STYLE_ANCHOR_LOST` | blockId 不存在或已变化 | true |
| `AG_TABLE_OPERATION_INVALID` | TableOperation 静态字段形状或坐标非法 | false |
| `AG_TABLE_OPERATION_UNAVAILABLE` | 当前表格动态能力或移动方向不允许 | false |
| `AG_IMAGE_OPERATION_INVALID` | ImageOperation 静态字段、来源路径、尺寸或位置非法 | false |
| `AG_IMAGE_OPERATION_UNAVAILABLE` | 当前图片实例、Block 锚、来源哈希或活动编辑器条件不满足 | true |

manifest 本身无效属于构建错误，不得静默退回手写默认清单。

## §7 需求到规则映射（REQ_MAP）

<!-- REQ_MAP
req_id: REQ-SYS-002
target_type: RULE
target_id: BR-SYS-DATA-006
status: mapped
-->

## §8 验收

- manifest 所有 capabilityId 唯一，version 非空，适用格式为 bdocx/html。
- manifest 每个 roundtripFeature 的 tier 与 DOC `classify_style` 一致，未知特性只允许 unsupported。
- 当前 BDocx 提示词包含 manifest 版本/hash、`subscript/superscript`，不得包含错误操作名 `sub/sup`。
- propose_edit JSON Schema 对 `replace_block/replace_document`（纯文本）及
  `block_ops/style_ops/table_ops/image_ops` 提供条件必填；BlockOperation/StyleOp/TableOperation/
  ImageOperation 使用分支 Schema。
- 当前 BDocx 提示词只声明 `block_ops` 文档内容能力；容器/原子块只读，独立内容块携带
  node_json/fingerprint/capabilities；富文档 replace_block/replace_document 在生成前拒绝。
- 合法 bold/textStyle/link/textAlign 可通过；未知 mark、非法 attrs、越界 range、错误 blockId 在生成
  PendingDiff 前拒绝。
- 表格仍服从 list_tables 动态能力、fingerprint、moveDirections 与富内容保护。
- 图片操作仍服从 list_images 的当前基线、image fingerprint、Block 锚和项目资源内容哈希；原始
  HTML 不得改变 BDocx 图片集合。
- 没有人工专属能力出现在代理操作 enum。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-07-28 | v1.0 | 建立独立 BDocxCapabilityManifest 协议：统一能力维度、StyleOp/TableOperation 基线、人工专属项、提示词/工具 Schema/执行前校验三投影、版本/hash 和保守失败语义；新增 TERM-SYS-002 与 BR-SYS-DATA-006。 |
| 2026-07-28 | v1.1 | BDocxCapabilityManifest 升至 1.1.0：新增 rangeUnit=utf16_code_unit，并统一 StyleOp 投影、Schema、校验和 ED CharAnchor 的偏移单位。 |
| 2026-07-29 | v1.2 | BDOCX-IMAGE-AGENT-EDIT-01：新增 ImageOperation 与 list_images 动态前置条件，锁定项目内图片来源、独立段落插入、文档节点删除、structured Diff 与原始 HTML 防绕过；将图片人工专属项收敛为缩放/移动/布局。 |
| 2026-07-29 | v1.3 | BDOCX-IMAGE-ADVANCED-AGENT-OPS-01：图片代理操作扩展缩放、上下移动与独立段落左中右对齐；锁定稳定会话 imageId、动态 fingerprint/capabilities/resizeBounds/moveDirections、同图单批一操作和对齐实证门禁。 |
| 2026-07-29 | v1.4 | BDOCX-IMAGE-COMPOSITE-IMPORT-01：同图 resize/layout/move 可组合且 delete 独占；新增受控本地图片复制导入、相邻文本定位与非视觉识别声明、import_image_asset/edit_blocked 工具边界。 |
| 2026-07-30 | v1.5 | 图片代理编辑闭合审计：当前能力基线标题对齐机器清单 1.4.0；人工确认本地拖入、保存、关闭及重编译重开链路正常，未扩大 ImageOperation 支持子集。 |
| 2026-07-30 | v1.6 | AG-RICH-BLOCK-STRUCTURED-EDIT-01：机器清单升至 1.5.0；BDocx 内容写入由 HTML replace_block/replace_document 收敛为活动 ProseMirror LogicalState 的 `block_ops`，登记层级块投影、动态写能力、基线/指纹/节点 Schema 整批校验、结构化 Diff 与精确节点回滚。 |
