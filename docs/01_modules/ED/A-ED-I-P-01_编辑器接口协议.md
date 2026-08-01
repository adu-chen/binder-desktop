---
文档编号：ED-I-P-01
文档状态：A
负责模块：ED
文档职责：ED 编辑器模块级接口协议（L2 接口层）——按 A-SYS-I-P-02 骨架声明 ED 的编辑会话 command、装饰层挂载/复位接口、blockId 定位接口、外部变更订阅；前端编辑器运行时与后端/上层模块的契约面
上游约束：A-SYS-I-P-02_IPC协议.md（命名/信封/错误/流式骨架）、A-ED-M-T-01_编辑器技术设计.md（BR-ED-* 规则、ED-EDIT/ED-DECOR 链路、X-FORBID-005）、A-ED-M-S-01_编辑器状态机设计.md（SM-ED-01 含 Conflicted 态）、A-WS-I-P-01_工作台接口协议.md（read_doc/save_doc/external_change 复用）、A-SYS-I-P-01_数据结构定义.md（DocRef）、ADU.md
直接承接：实现期前端编辑器（TipTap）与扩展点按本文契约注册；DE（diff 呈现）/AG（局部修改）/REF（@mention）按装饰层接口挂载（X-FORBID-005：唯一受控通道）
使用边界：本文声明 ED 接口的**语义级签名**；文件 I/O 经 WS（不在本文重复，ED 经 A-WS-I-P-01）；不新增 RULE/CHAIN/CONSTRAINT/TERM；装饰层接口为进程内前端契约（非 Tauri command）与跨模块 command 混合，分节标注
变更要求：装饰层接口变更须复核 DE/AG/REF 消费方；编辑会话 command 变更须复核 SM-ED-01 一致性；blockId 契约变更联动 DE 定位
治理成熟度：draft
---

# Binder 编辑器接口协议（ED · L2 接口层）

## §1 文档定位

```
A-SYS-I-P-02（骨架）+ A-WS-I-P-01（文件 I/O 复用）→ 本文（ED 会话/装饰层/定位接口）→ 实现 + @GOV(追溯 A-ED-M-T-01)
```

ED 接口分两类：**（a）编辑会话 command**（前端 Store/Service ↔ 后端协同，经 WS 落盘，含 OpenDocSnapshot 与 session 持久化镜像）；
**（b）装饰层与定位接口**（前端进程内契约，供 DE/AG/REF 在编辑器上挂载呈现，X-FORBID-005
唯一受控通道）。后者非跨进程 Tauri command，但作为模块间契约在此声明。

## §2 编辑会话接口（ED-EDIT / SM-ED-01）

> 内容读写经 WS（`ws.read_doc`/`ws.save_doc`，A-WS-I-P-01）；以下为 ED 会话编排接口
> （前端编辑器运行时对外暴露）。

| 接口 | 参数（语义） | 结果 | 特有错误/状态 | 归属 |
|---|---|---|---|---|
| `ed.open_session` | DocRef | 会话句柄 + `{editFormat, contentFormat, content}` 编辑模型（Markdown 经 Codec / b-docx 经 DOC / text 原文） | Markdown 解析失败或非支持结构→源码保护/只读降级（SM-ED-01 ReadOnly）；读取失败→会话不建立 | BR-ED-STATE-001 / BR-ED-DATA-006 |
| `ed.serialize_session` | 会话句柄 | 与 editFormat 对应的 DiskState 内容；Markdown 返回规范化源码 | 非支持节点/样式、序列化失败、语义往返失败→`ED_MARKDOWN_SERIALIZE_FAILED`，保持 dirty 且不写盘 | BR-ED-VERIFY-001 |
| `ed.close_session` | 会话句柄 | 确认 | 有脏态未处置→拒绝（须先 save 或 confirmDiscard） | BR-ED-STATE-001 |
| `ed.get_dirty_state` | 会话句柄 | 是否脏 + 当前状态（SM-ED-01 态） | — | X-INTER-001（驱动 SM-WS-01） |
| `ed.get_open_snapshots` | — | OpenDocSnapshot[]（全部已打开文档 LogicalState 快照） | active live 序列化失败/缺 session 版本 → 错误 | AG/DE 读门禁（A-SYS-I-P-01 OpenDocSnapshot） |
| `ed.persist_open_tabs` | EditorOpenTab[] | ok | 无 workspace / active 数量非法 / open_index 冲突 → 错误 | `ed_open_tabs` 打开集合控制态 |
| `ed.list_open_tabs` | workspace_id | EditorOpenTab[] | 无打开标签 → 空列表 | 重启恢复打开集合、顺序与当前文档 |
| `ed.persist_session_snapshot` | OpenDocSnapshot；BDocx 须含 baseline 且图片为规范相对引用 | ok | 无 workspace / session 缺失 / baseline 非 u64 → 错误 | `ed_session_snapshots` 内部镜像 |
| `ed.list_session_snapshots` | workspace_id | OpenDocSnapshot[]；BDocx 恢复投影含 baseline、纯净规范 HTML | 无镜像 → 空列表 | 重启恢复已打开文档 LogicalState 镜像 |
| `ed.confirm_discard` | 会话句柄 | 确认丢弃脏态并关闭 | — | 显式确认语义 |
| `ed.resolve_conflict` | 会话句柄、选择（keep_mine / reload） | 重置态（Editing/Loading） | 非 Conflicted 态调用→协议错误 | SM-ED-01 Conflicted（BR-ED-STATE-003，对接 ws.save_doc override / external_change） |

> `ed.resolve_conflict(keep_mine)` 后续 `ws.save_doc` 须带 override=true（用户显式覆盖语义链）；
> `reload` 经 `ws.read_doc` 重载并刷新基线。
> `editFormat` 枚举为 `text / markdown / html / html_file`；`contentFormat` 枚举为 `text / html`。
> 受支持 Markdown 的 LogicalState 快照必须是 `editFormat=markdown, contentFormat=html`，源码保护路径为
> `editFormat=markdown, contentFormat=text`。`ed.get_open_snapshots` 是 AG/DE 读取已打开文档的唯一门禁：激活文档必须从 ProseMirror `doc`
> 实时序列化，非激活打开文档必须从 `EditSession.content` 读取并附带 sessionVersion/contentHash。
> 失败时调用方必须阻断，不得读取 DiskState。
> 重启恢复必须先调用 `ed.list_open_tabs` 得到打开集合、顺序与当前文档，再只恢复这些路径对应的
> `ed.list_session_snapshots` 镜像；不在 `ed_open_tabs` 中的孤立 session 镜像不得打开文档。
> BDocx 图片的 `asset://` 地址只属于当前 WebView 显示投影。持久化前必须按
> `data-binder-media-src` 还原规范相对 `src`；恢复后携 snapshot baseline 调
> `doc.restore_edit_media` 重新授权/同基线重建媒体，再生成本进程显示地址。内部存储 envelope 必须在
> `ed.list_session_snapshots` 返回前移除，不能进入 TipTap LogicalState。

## §3 装饰层接口（ED-DECOR / BR-ED-STATE-002 / X-FORBID-005）

> 前端进程内契约，DE/AG/REF 为消费方；ED 是编辑器内部状态的唯一受控通道，消费方不得绕过。

| 接口 | 参数（语义） | 结果 | 特有错误 | 归属 |
|---|---|---|---|---|
| `ed.mount_decoration` | 消费方标识、blockId、装饰描述（呈现型，不含正文写入） | 装饰句柄 | **anchor_lost**（blockId 失效，BR-ED-DATA-001） | DE diff 呈现 / AG 内联候选 |
| `ed.update_decoration` | 装饰句柄、新呈现描述 | 确认 | 句柄失效 | — |
| `ed.dismiss_decoration` | 装饰句柄 | 复位确认（不残留） | — | 消费终态后复位 |
| `ed.on_decoration_action` | 装饰句柄（订阅） | 用户交互事件（如接受/拒绝点击）转达消费方 | — | ED 不代裁（转达 DE 裁决 / AG 采纳） |

## §4 blockId / 字符锚定位接口（ED-EDIT / BR-ED-DATA-001 / BR-ED-DATA-004）

| 接口 | 参数（语义） | 结果 | 特有错误 | 归属 |
|---|---|---|---|---|
| `ed.locate_block` | 会话句柄、blockId、可选字符锚 {start,end} | 定位结果（块范围/可视位置；带锚时含区间校验） | **anchor_lost**（块失效或偏移越界，明确失效信号不静默错位） | DE 定位 / REF 引用锚校验 / AG 字符级取址 |
| `ed.get_cursor_ref` | 会话句柄 | 光标位置（CharAnchor：blockId + 块内偏移） | 无会话/无光标 → null | AG 光标感知（按需查询，非常驻投影） |
| `ed.get_selection_ref` | 会话句柄 | SelectionRef {path, blockId, start, end, text}；折叠选区 → null | — | AG 选区感知 / 选区拖拽引用（PI-ED-10） |
| `ed.apply_edit` | 会话句柄、编辑操作序列（op 粒度表 §4.1） | 新编辑模型态 + 脏态 | 越权（绕过受控接口）→拒绝；op 目标 anchor_lost → 拒绝该 op | 采纳内联候选/样式操作的落点（AG 经此，BR-AG-STATE-003）；X-FORBID-005 唯一通道 |
| `ed.apply_session_diff` | relPath、DiffProjection、base_session_version、base_content_hash | 更新目标 open session content/version/hash + 脏态；返回新 OpenDocSnapshot | 目标未打开/基线不匹配/锚失效/StyleOp 无头事务不支持 → 拒绝且不修改 | `draft_mode=open_session` 自动应用事务；DE autoApplyOk/autoApplyFail |

> `ed.apply_edit` 是 inline-assist/autocomplete/内联样式操作采纳「作为用户编辑操作」进入模型的
> 接口——经此驱动 SM-ED-01 edit→SM-WS-01 脏态，区别于 diff 落盘路径（后者经 DE→WS）。
> 偏移均在**块内纯文本平面**（BR-ED-DATA-004）；锚有效期=编辑会话内（重开须重新取块）。
> **实现回写（2026-07-12）**：当前前端进程内落地为 `applyEditOps()`（内容类 op）与
> `applyStyleOps()`（StyleOp 批处理）。`applyEditOps()` 已覆盖 `replace_text`、`replace_block`、
> `replace_document`；StyleOp 的 `set_mark` / `unset_mark` / `set_block_attr` 由 `applyStyleOps()`
> 单事务批内原子应用。`insert_block` 仍是接口预留，未在本轮实现。
> `ed.apply_session_diff` 是 `draft_mode=open_session` 的唯一入口。它不依赖用户切换到目标窗口，
> 但必须只作用于已打开非激活文档的 `EditSession.content`；成功后调用 `ed.persist_session_snapshot`，
> 并由 DE 调用 `de.accept_draft_diff(open_session)`。失败时必须保持原 LogicalState 不变。

### §4.1 apply_edit 操作粒度表（PI-ED-10/11，正式承接 §7 既有债务）

| op | 参数 | 语义 | 归属规则 |
|---|---|---|---|
| `replace_text` | 字符锚 {blockId,start,end}、text | 区间文本替换（含 start=end 纯插入） | BR-ED-DATA-004 |
| `replace_block` | blockId、text | 替换目标块内容，blockId 保持 | BR-ED-DATA-001 / BR-ED-STATE-001 |
| `replace_document` | content、format(text/markdown/html/html_file)、contentFormat(text/html) | 整篇替换为指定 LogicalState 内容；Markdown HTML 快照不得当作 Markdown 源码重复解析 | BR-ED-STATE-001 / BR-ED-DATA-002 / BR-ED-DATA-006 |
| `insert_block` | blockId（其后）、blockJSON | 块后插入新块（新块得新 id） | BR-ED-DATA-001 |
| `set_mark` | 字符锚区间、mark {type,attrs} | 区间加字符样式（bold/color/fontSize…） | BR-ED-DATA-005（StyleOp） |
| `unset_mark` | 字符锚区间、markType | 区间去字符样式 | BR-ED-DATA-005（StyleOp） |
| `set_block_attr` | blockId、attrs（textAlign…） | 块属性设置 | BR-ED-DATA-005（StyleOp） |

> op 序列原子应用（任一 op 锚失效则整批拒绝回明确错误，不产生半应用状态）。
> StyleOp 的代理可用子集及 DOCX 往返等级见 A-SYS-I-P-04 BDocxCapabilityManifest。ED 执行侧仍须
> 防御性拒绝未知 mark/attrs、非法 textAlign、错误 blockId、空区间或越界区间；真实上下标 mark 名为
> `subscript/superscript`，`sub/sup` 不是合法 StyleOp。

### §4.1.1 富文档结构化 BlockOperation

活动富文档内容修改使用 `applyStructuredBlockOperations(payload,direction)`，payload 为
`{payloadVersion:1,base:{sessionVersion,contentHash},operations[]}`。每项是
`{op:replace_node,blockId,fingerprint,expectedNodeType,beforeNode,afterNode}`。

前向应用先对整批完成 base、blockId 唯一性、锚、动态能力、node type、fingerprint、attrs/blockId
保持和 ProseMirror Schema 校验，再按位置倒序写入一个 transaction；任一失败不得 dispatch。
反向拒绝只在目标节点仍精确匹配 afterNode fingerprint 时恢复 beforeNode；后置人工编辑导致不匹配时
返回 conflict，不得用 draftBaseContent 整篇覆盖。表格、图片及其祖先/承载块没有 replace_node 能力，
继续使用专用 TableOperation/ImageOperation。

### §4.2 open session 自动应用操作（2026-07-18）

`ed.apply_session_diff` 支持的最小安全集：

| patch_kind | 支持条件 | 应用语义 |
|---|---|---|
| `replace_block` | 仅 contentFormat=text；基线匹配 | 替换纯文本行块 |
| `replace_document` | 仅 contentFormat=text；基线匹配 | 整篇替换纯文本 session content |
| `block_ops` | 不支持非活动 session | 明确拒绝；不得解析/改写缓存 HTML |
| `style_ops` | 存在不依赖当前 ProseMirror editor instance 的 StyleOp 事务实现 | 批内原子应用；未具备时拒绝，不降级 |

该接口必须先比对 `base_session_version` 与 `base_content_hash`，再执行内容事务；成功后递增版本、刷新哈希、
置脏并持久化 `ed_session_snapshots`。任何失败都不得产生半应用状态。

### §4.3 BDocx TableOperation

BDocx 表格修改使用 `table_ops` 专用载荷：`{table_id, fingerprint, operations[]}`。人工工具与代理建议
共用同一 ProseMirror 规划器；预览和应用均按整批原子语义执行。代理必须先从 `list_tables` 获取当前
`sessionVersion`、`contentHash`、表格指纹和动态 capabilities，不得自行拼接原始 HTML。

### §4.4 BDocx 图片人工操作

以下是 ED 前端进程内受控接口，不是公开 IPC command：

| 操作 | 前置条件 | 结果 | 拒绝边界 |
|---|---|---|---|
| `activeImageSnapshot` | 当前选区为 Image NodeSelection | blockId、是否独立图片段落、真实可移动方向 | 非图片选区返回空 |
| `ResizableImage` 拖宽 | 左键命中选中图片右下手柄 | zoom 归一后的 `width` 节点属性事务；实际改宽时清除固定 `height` | 空拖动无事务；不写媒体资源 |
| `moveActiveImage(up/down)` | 顶层 paragraph 仅含一个 image，且目标方向有相邻顶层 Block | 整个图片段落交换一块，保持 blockId、媒体属性与 NodeSelection | 混排图片、越界方向、非图片选区均拒绝 |
| `deleteActiveImage` | 当前选区为 image | 独立图片删除段落；混排图片只删节点；唯一段落留下空段 | 不删除媒体缓存文件 |
| Finder 文件拖入 | 活动 BDocx；WKWebView DOM `Files` 拖入纸张容器；PNG/JPEG 普通文件 | 前端读取用户明确拖入的 File 字节并经 DOC 受控复制/复用，以命中或最近顶层 Block 的 before/after 锚在单一事务插入独立图片段落 | 非图片/混合文件、导入失败、文档切换或无稳定 Block 锚拒绝；本地绝对路径不进入 IPC/LogicalState；撤销不删除项目资产 |
| 项目文件树拖入 | 文件树 DocRef 指向 workspace 内 PNG/JPEG，拖放点在纸张容器内 | 相对路径经 DOC 守界校验后复用或复制到受控图片根，再按同一 Block 锚插入 | 非规范相对路径、越界、非图片或无锚拒绝；不得把 DocRef 当系统绝对路径 |
| 剪贴板图片粘贴 | Clipboard File，或不含文字的纯图片 HTML data URL 为 PNG/JPEG；当前光标命中稳定顶层 Block | 图片字节经 data URL 送 DOC，重新校验大小、媒体类型、magic、哈希并复制/去重；插入当前 Block 之后 | 远程/混合文字 HTML 图片、其他格式或读取失败拒绝；data URL 不进入 LogicalState |
| 文档内图片复制粘贴 | 复制选区包含已有 image | 保留 ProseMirror 富剪贴板和 `data-binder-media-src`，粘贴时复用既有媒体身份 | 含图片选区不得降级为 SelectionRef 叶子占位字符 |

邻近按钮通过 portal 直接挂到选中 `.ed-img-view`，由 DOM 归属自然跟随滚动、缩放、选区和事务；按钮
位于图片上方，不覆盖图片主体。BDocx 不提供 inline/block 切换：`data-display` 经 Pandoc 往返丢失，
独立与行内只按段落结构区分。

### §4.5 BDocx ImageOperation

代理图片修改使用 `image_ops` 专用载荷：
`{base:{sessionVersion,contentHash},operations:ImageOperation[]}`。`insert_image` 携带稳定
`anchorBlockId + position(before|after)`、workspace 相对 `sourcePath + sourceHash`、DOC 物化后的
`mediaRef`，以及可选 alt/width；其余操作均携带稳定 `imageId + 当前 fingerprint`：
`delete_image` 删除文档节点，`resize_image(width)` 服从动态宽度边界，
`move_image(direction=up|down)` 服从动态方向，`set_image_layout(alignment=left|center|right)`
只作用于独立顶层图片段落。

载荷只允许当前活动 BDocx editor 以单个 ProseMirror 事务整批应用；同一图片单批可各含一个
resize/layout/move，并按缩放→对齐→移动固定顺序执行，重复同类操作拒绝，delete 独占。插入
生成独立图片段落，删除遵循 §4.4 的独立/混排语义；基线、Block、图片实例、fingerprint、动态能力或
来源哈希失效时整批拒绝。`image_ops` 不进入
`ed.apply_session_diff`：非活动会话不得降级修改 HTML 字符串。拒绝 Diff 必须恢复应用前完整
LogicalState，接受只确认当前草稿，保存才进入既有 DOC/WS 链。

| op | 参数 | 语义 | 失败边界 |
|---|---|---|---|
| `set_cell_background` | `at:{row,col}`、`color:#RRGGBB \| null` | 设置或清除目标单元格纯色底纹；人工多选时逐节点应用同一属性 | 非不透明六位十六进制、越界、非合并单元格左上锚、能力或指纹不匹配时拒绝 |

该操作只修改 `tableCell/tableHeader.backgroundColor`，不得替换单元格内容。表格快照必须为每个逻辑
单元格投影 `backgroundColor`，使底纹改变参与 fingerprint 与 Diff 过期判断。

## §5 外部变更订阅（承 A-WS-I-P-01 §6）

ED 会话订阅 `ws.external_change` 事件的逻辑契约：无脏态自动重载（经 `ws.read_doc` 刷新基线），
有脏态进 Conflicted 态待 `ed.resolve_conflict`（BR-ED-STATE-003）。本文不重复事件定义（归 WS I-P）。

**当前实现边界（2026-07-12）**：`ws.external_change` 物理推送/轮询 command 尚未注册；已落地的是
保存时由 WS 基线 guard 返回 `WS_SAVE_CONFLICT`，ED 将其映射为 `SaveOutcome.conflict=true` 并等待
用户显式覆盖确认或重载。后续接入事件推送时不得绕过该保存前 guard。

## §6 测试与验收

- 会话生命周期 command 覆盖 SM-ED-01 全转移（含 Conflicted 路：external_change/saveConflict→
  resolve_conflict 两分支）；装饰层 mount→update→dismiss 复位不残留 + anchor_lost 失效信号；
  `ed.apply_edit` 越过受控接口的调用被拒（X-FORBID-005 静态+集成核验）；`ed.get_open_snapshots`
  覆盖 active/open session 且失败阻断；`ed.apply_session_diff` 基线匹配成功、基线不匹配拒绝且不修改；
  `ed.persist_open_tabs` / `ed.list_open_tabs` 覆盖多标签顺序、当前文档唯一性、孤立 session 镜像不恢复；
  与 DE/AG/REF G5 联调。
- ImageOperation 覆盖插入/删除批原子应用、Block/image 锚失效、项目来源属性保持、非活动会话拒绝、
  reject 全文恢复，以及保存前规范媒体引用不含 `asset://`。

## §7 债务与盲区

- 装饰描述的呈现 schema（diff 高亮样式 / 内联候选样式）延实现期与 DE/AG 联合定。
- ~~编辑操作（apply_edit 的 op 语义）的粒度表延实现期~~ → **已承接（§4.1，2026-07-03，PI-ED-10/11
  文档先行）**：replace_text / insert_block / set_mark / unset_mark / set_block_attr，批内原子应用。
- 装饰层接口是否需要跨进程化（如后端驱动呈现）延实现期，当前定为前端进程内契约。
- 样式投影读接口经 AG 侧承载（ag.list_blocks 扩展，A-AG-I-P-01）；ED 侧只定投影 schema
  （A-ED-M-T-01 §5.1），避免双仓重复定义。
- `ed.apply_session_diff(style_ops)` 依赖无头 StyleOp 事务；若本轮代码只覆盖文本/块/整篇替换，样式类
  自动应用必须明确失败并让 DiffCard 失效。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（实现前置件）：ED 接口协议——编辑会话 command 5 条（含 resolve_conflict 对接 SM-ED-01 Conflicted）、装饰层接口 4 条（mount/update/dismiss/on_action，X-FORBID-005 唯一通道）、blockId 定位 2 条（locate_block 失效信号 / apply_edit 内联采纳落点）、外部变更订阅承 WS I-P；零新增注册项；债务（呈现 schema/编辑 op 粒度/装饰层进程边界延实现期） |
| 2026-07-03 | v1.1 | 批B 契约扩展（A-ED-X-L-02 PI-ED-10/11 文档先行，承 BR-ED-DATA-004/005）：§4 扩字符锚定位（locate_block 带 {start,end} 区间校验）+ 新增 ed.get_cursor_ref / ed.get_selection_ref（按需查询）；新增 §4.1 apply_edit 操作粒度表（replace_text/insert_block/set_mark/unset_mark/set_block_attr，批内原子）解除既有债务；§7 登记样式投影读接口归 AG I-P 承载 |
| 2026-07-04 | v1.2 | 术语统一清扫（承 2026-07-04 人类三轮裁决）：正文禁用别名替换——内联辅助→局部修改、自动补全→辅助续写、对话编辑→Diff Edit、Chat↔Build 接管→构建移交/回落、chat/chat-agent 回合→代理回合；文件名与变更记录历史行不改写 |
| 2026-07-12 | v1.3 | 三态实现回写：补 `replace_block` / `replace_document` 为当前 `applyEditOps()` 已实现内容 op；声明 StyleOp 由 `applyStyleOps()` 批内原子承载；标注 `ws.external_change` 推送/轮询尚未注册，当前已落地能力为保存时 `WS_SAVE_CONFLICT` 冲突映射与显式覆盖确认 |
| 2026-07-18 | v1.4 | 新增已打开文档接口：`ed.get_open_snapshots`、`ed.persist_session_snapshot`、`ed.list_session_snapshots` 与 `ed.apply_session_diff`；`draft_mode=open_session` 必须基线校验后自动应用 LogicalState，不得后台写盘 |
| 2026-07-18 | v1.5 | 新增编辑器打开标签持久化接口：`ed.persist_open_tabs` / `ed.list_open_tabs`；启动恢复以打开集合为门禁，只恢复对应 session 镜像并同步当前文档 |
| 2026-07-18 | v1.6 | ED-MARKDOWN-RICH-ROUNDTRIP-01：接口增加 editFormat=markdown 与 contentFormat；open/serialize/replace_document 明确 Markdown Codec、源码保护、语义校验失败不写盘契约 |
| 2026-07-28 | v1.7 | BDocx TableCellShading：新增 §4.3 TableOperation 接口，锁定 `set_cell_background`、多选节点属性语义、合并单元格左上锚、动态能力/指纹校验及富内容保护。 |
| 2026-07-28 | v1.8 | StyleOp 接入 BDocxCapabilityManifest，补真实 mark 名、attrs/textAlign、blockId 与字符范围的 ED 防御性批内原子校验边界。 |
| 2026-07-28 | v1.9 | BDOCX-IMAGE-MEDIA-FOUNDATION-01：BDocx session snapshot 增 baseline 与规范相对媒体引用要求；重启恢复经 `doc.restore_edit_media` 重建显示授权，禁止持久化 `asset://` 或让存储 envelope 进入 LogicalState。 |
| 2026-07-28 | v1.10 | BDOCX-IMAGE-INTERACTION-01：新增 §4.4 图片前端受控操作，锁定真实拖宽、独立图片段落移动、删除、混排保护、DOM 归属锚定及 data-display 非持久边界。 |
| 2026-07-29 | v1.11 | BDOCX-IMAGE-AGENT-EDIT-01：新增 §4.5 ImageOperation 载荷、动态锚/基线、项目来源、当前活动编辑器批原子应用、拒绝恢复和保存边界。 |
| 2026-07-29 | v1.12 | BDOCX-IMAGE-ADVANCED-AGENT-OPS-01：ImageOperation 增加缩放、上下移动与独立图片段落对齐，明确稳定 imageId、当前 fingerprint、动态能力及同图单批一操作。 |
| 2026-07-29 | v1.13 | BDOCX-IMAGE-COMPOSITE-IMPORT-01：同图 resize/layout/move 单事务组合；新增系统本地图片受控拖入、Block 坐标锚、源文件保护、撤销/脏态边界；BDocx 工具栏禁用远程 URL 插图。 |
| 2026-07-30 | v1.14 | BDOCX-IMAGE-COMPOSITE-IMPORT-01 人工入口闭合：系统拖入增加可见反馈和错误提示；项目文件树拖入、剪贴板 PNG/JPEG、文档内图片复制粘贴统一进入受控媒体与 Block 锚链。 |
| 2026-07-30 | v1.15 | 拖放运行时冲突修复：关闭 Tauri 内部 drag-drop handler，WKWebView DOM 纸张容器成为 Finder Files、项目 DocRef 与 ProseMirror 内部图片拖动的单一事件入口；外部图片捕获阶段分流，内部拖动不被接管；空白/分页间隙按最近稳定 Block 降级。 |
| 2026-07-30 | v1.16 | AG-RICH-BLOCK-STRUCTURED-EDIT-01：新增活动富文档 BlockOperation 单事务接口、整批基线/锚/类型/指纹/Schema 校验与精确节点反向恢复；非活动富文档及 HTML replace_block/replace_document 不再支持。 |
