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
| `ed.persist_session_snapshot` | OpenDocSnapshot | ok | 无 workspace / session 缺失 → 错误 | `ed_session_snapshots` 内部镜像 |
| `ed.list_session_snapshots` | workspace_id | OpenDocSnapshot[] | 无镜像 → 空列表 | 重启恢复已打开文档 LogicalState 镜像 |
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

### §4.2 open session 自动应用操作（2026-07-18）

`ed.apply_session_diff` 支持的最小安全集：

| patch_kind | 支持条件 | 应用语义 |
|---|---|---|
| `replace_block` | `block_id` 或 `block_index` 可定位到当前 session 内容块；基线匹配 | 替换目标块文本/HTML 内容，保持其他块不变 |
| `replace_document` | 基线匹配；after_text/after_content 为整篇内容 | 整篇替换 session content |
| `style_ops` | 存在不依赖当前 ProseMirror editor instance 的 StyleOp 事务实现 | 批内原子应用；未具备时拒绝，不降级 |

该接口必须先比对 `base_session_version` 与 `base_content_hash`，再执行内容事务；成功后递增版本、刷新哈希、
置脏并持久化 `ed_session_snapshots`。任何失败都不得产生半应用状态。

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
