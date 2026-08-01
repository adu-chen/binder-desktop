# BDocx 代理编辑 — 本轮修复后预期效果

**日期**：2026-07-26
**前提**：P0#1-#8 + 决策 D1-D9 全部落地后的稳定基线
**边界**：1.0 版本范围内交付，1.0-post 项标注

---

## 1. 草稿创建

**入口**：用户打开原生 DOCX（预览态不可编辑）→ 预览页出现"编辑"按钮 → 点击。

| 场景 | 行为 |
|------|------|
| 首次编辑 | 系统创建 BDocx 草稿（隐性标识，非文件名推断）→ 编辑器打开草稿 → 分页呈现 |
| 草稿已存在 | 直接跳到已有草稿编辑窗口，不删除重建 |
| 模型指令编辑 | 后台自动创建草稿（遵循不编辑原文档原则），弹出用户授权确认 |
| 外部删除草稿 | 工作台监听到文件丢失 → 编辑器显示失效态 → 阻断编辑 → 不读缓存 |

**1.0 边界**：BDocx 身份标记为显式创建态，重命名原生 DOCX 不被识别为 BDocx。

---

## 2. 编辑器行为

### 2.1 光标与滚动

| 场景 | 行为 |
|------|------|
| 任意位置输入 | 光标不跳，页面不抖 |
| 末页输入 | 页面底部（含 end-fill 补白）对齐窗口底部，不跳到内容底部 |
| 滚动到中间位置输入 | 保持当前视口，不动 |
| IME 组合输入 | 不触发重测量，compositionend 后再结算 |

### 2.2 分页呈现

| 场景 | 行为 |
|------|------|
| 普通段落 | 按视觉行在页间切分（贪心填页） |
| 含富文本段落（bold/italic/link） | 整段原子分页（不丢失格式）——**1.0 限制** |
| 列表（orderedList/bulletList） | 展平为 listItem，在 listItem 间切分 |
| 表格 | 整表原子分页——**1.0 限制** |
| codeBlock/blockquote | 原子分页 |

### 2.3 块模型

| 块类型 | blockId | 分页 | 光标定位 | 样式投影 |
|---------|---------|------|----------|----------|
| paragraph | ✅ | 行间切分 | ✅ | ✅ |
| heading | ✅ | 原子 | ✅ | ✅ |
| blockquote | ✅ | 原子 | ✅ | ✅ |
| codeBlock | ✅ | 原子 | ✅ | ✅ |
| listItem | ✅ | 原子 | ✅ | ✅ |
| orderedList | ✅(容器) | 透明(展开) | 穿透 | 透明 |
| bulletList | ✅(容器) | 透明(展开) | 穿透 | 透明 |
| tableCell | **待补充** | 原子(整表) | **回退错误**→需修复 | 委托内paragraph |
| image | 委托父 | 依赖父 | 间接 | mark投影 |

**1.0 边界**：tableCell 不在 BINDER_BLOCK_TYPES，表格内光标定位回退错误，表格内无法使用局部编辑/辅助续写。

---

## 3. 代理编辑流程

### 3.1 有逻辑状态文档（编辑器已打开）

```
用户触发assist
  → 捕获光标/选区 + 块上下文 + 当前文档快照
  → 构建提示词（含 BDocx 格式标识 + 样式能力描述）
  → 模型调用 read_doc/list_blocks/propose_edit

模型返回修改
  → 直接修改 LogicalState（编辑器内容）
  → DiffCard 出现在聊天框
  → 编辑器显示绿色覆盖（GreenReview）
  → 文档 tabState 变为 pending_diff
```

### 3.2 编辑态下的操作

| 操作 | LogicalState | 编辑器 tabState | DiffCard | 绿增 |
|------|-------------|---------|----------|------|
| 模型返回 | 已修改 | pending_diff | pending | 绿色 |
| 用户接受 | 不变 | pending_diff | accepted | 绿色 |
| 用户拒绝 | 回滚到 before | dirty（来自 baseDirty）/ saved | rejected | 消失 |
| 用户手动编辑 | 再次修改 | **dirty**（逻辑态≠磁盘态） | **旧 diff 自动失效**（D4） | **消失** |
| 用户保存 | 不变 | **saved** | pending→accepted | **消失** |

**关键约束（D4 锚唯一性原则）**：
- 同一内容不可能产生两条 diff——diff 返回即修改逻辑状态，之前锚定的 diff 自动失效
- 用户手动编辑改变逻辑状态→之前 diff 失效→绿增消失→DiffCard 进入失效态（只展示，不可操作）
- **D4a 澄清**："不转 dirty"仅指 DiffCard 无 dirty 概念。编辑器 tabState 在手动编辑后必须转 dirty（逻辑态≠磁盘态）。当前 `markDirty()` 跳过 `pending_diff` 是错误——需修复

### 3.3 裁决链

| 路径 | 触发 | 行为 |
|------|------|------|
| current_editor | 当前活动编辑器的 diff | DiffCard 等待用户接受/拒绝 |
| open_session | 已打开但非活动标签的 diff | **不再 auto-accept**，DiffCard 等待用户裁决 |
| background | 从未打开文档的 diff | accept→直写盘；reject→丢弃；doc 被打开→**自动映射到逻辑状态** |

---

## 4. 保存链路

### 4.1 主动保存

| 触发方式 | 行为 |
|---------|------|
| ⌘S / 点击保存 / 关闭标签前自动保存 | LogicalState → 序列化 → DOCX 转换（Pandoc HTML→DOCX）→ 原子文件写入（临时文件+rename） |
| 保存时 pending diff | **auto-accept**（D8），绿增消失，deStore 状态从 pending→accepted |
| 保存时 accepted diff | 不变，绿增消失 |
| 保存冲突 | **D13 已决策**：不检测外部修改，直接覆盖写盘（保存=全文覆盖）。文件被删除除外——删除后走失效路径，无法进入保存 |

### 4.2 标签关闭

| 场景 | 行为 |
|------|------|
| 关闭标签 | session.content 保留在 edStore（不 delete），pending diff 保留 |
| 重新打开同一文档 | 走 `setActivePath` 从缓存装载 → blockId 不变 → pending diff 锚有效 |
| 关闭后外部修改 DOCX | **D14 已决策**：不检测冲突，直接使用缓存逻辑状态（外部修改对逻辑状态无影响）。可记录债务 |

### 4.3 应用关闭

| 场景 | 行为 |
|------|------|
| 正常关闭（⌘Q / 菜单退出） | 所有 pending diff 必须终裁（接受/拒绝）或提示用户（APP-CLOSE-DIFF-LIFECYCLE-01） |
| 异常关闭（崩溃/断电） | 所有 pending diff 丢弃（sweep_abnormal_exit），视为"放弃修改" |
| 重启后 | **D15 已决策**：历史 diff 有记录（显示正确的 diff 卡状态，如失效态），但不继承编辑功能 |

---

## 5. Diff 展示

### 5.1 编辑器内（GreenReview）

| diff 类型 | 展示效果 |
|----------|---------|
| 单块 replace_block | 该块绿色背景 |
| 整篇 replace_document | 所有块绿色背景 |
| 多块 style_ops | 所有受影响块绿色背景（按 styleOpsPayload blockId 列表）——**待修复** |
| pending/applying 态 | 绿色 + 块后 before/after preview widget（文本截断 240 字符） |
| accepted 态 | 仅绿色背景，无 preview widget |
| saved 态 | 绿增消失 |

**1.0 边界**：GreenReview 只展示绿色/非绿色二元装饰，before/after 文本传入待补充。整篇替换不能区分用户编辑与模型修改。

### 5.2 DiffCard（聊天框）

| 状态 | 外观 |
|------|------|
| pending | 正常卡片 → 可接受/拒绝 |
| accepted | 已确认标记 |
| rejected | 已拒绝标记 |
| expired/error | 失效态，只展示不可操作 |

---

## 6. 提示词与工具

### 6.1 提示词

| 信息 | 注入 |
|------|------|
| 文档格式（BDocx/Text/Markdown） | **待注入** |
| 可用 mark 类型（bold/italic/underline/strike/highlight/sub/sup/code/link/textStyle） | **待补具体清单** |
| blockId vs blockIndex 使用规则 | 已有（优先 blockId） |
| 样式能力矩阵（BDocx 支持哪些操作） | **待补** |

### 6.2 工具调用

| 工具 | 状态 |
|------|------|
| read_doc | ✅ 已接线 |
| list_docs | ✅ 已接线 |
| search_text | ✅ 已接线 |
| list_blocks | ✅ 已接线（容器块透明） |
| propose_edit | ✅ 已接线 |
| propose_edit → replace_block | ✅ 但**载荷需改为结构化**（不再纯文本替换） |
| propose_edit → replace_document | ✅ |
| propose_edit → style_ops | **待补 open_session/background 路径** |

---

## 7. 样式支持

### 7.1 1.0 交付范围

**D16 已决策：样式子集必须兼容继承，不可降为"编辑可加但保存丢"。**

| 能力 | 编辑 | 保存往返 | 说明 |
|------|------|---------|------|
| heading (h1-h6) | ✅ | ✅ | Pandoc 原生支持 |
| bold | ✅ | ✅ | strong 标签往返 |
| italic | ✅ | ✅ | em 标签往返 |
| underline | ✅ | ✅ | u 标签往返 |
| list (ol/ul) | ✅ | ✅ | Pandoc 原生支持 |
| table | ✅ 编辑 | ✅ 结构 | 样式需验证 |
| image | ✅ 显示 | ✅ 闭环 | asset↔相对路径反向映射 |
| color | ✅ 编辑 | **1.0 交付** | 需 Pandoc Lua filter 保留内联 color |
| fontSize | ✅ 编辑 | **1.0 交付** | 需 Pandoc Lua filter 保留内联 font-size |
| fontFamily | ✅ 编辑 | **1.0 交付** | 需 Pandoc Lua filter 保留内联 font-family |
| textAlign | ✅ 编辑 | **1.0 交付** | 需 Pandoc Lua filter 保留 text-align |
| highlight | ✅ 编辑 | **1.0 交付** | 需验证 mark 标签映射 |
| sub/superscript | ✅ 编辑 | **1.0 交付** | sub/sup 标签往返 |

---

## 8. 媒体

### 8.1 修复后行为

| 操作 | 行为 |
|------|------|
| 打开 DOCX | Pandoc `--extract-media=<dir>` → 图片解压 → HTML 含 `<img src="media/image1.png">` |
| 前端显示 | 改写为 `asset://` URL（Tauri WebView 安全沙箱要求），保持映射表 `{asset://URL → 相对路径}` |
| 保存 DOCX | 反向映射 `asset://` → 相对路径 → Pandoc `html→docx --resource-path=<extract_dir>` → 图片重新嵌入 DOCX ✅ |
| 会话结束 | 清理临时媒体目录 + asset 授权 |

**关键修复**：补上 `docService.ts:49` `asset://` 改写对应的反向映射（约 20-30 行），使图片在 HTML→DOCX 往返中不丢失。

---

## 9. 1.0 版本边界清单

| 项 | 1.0 状态 | 说明 |
|----|---------|------|
| 代理编辑基础链路（当前编辑器） | ✅ 交付 | 三态裁决 + 绿增 + 保存 auto-accept |
| open_session 裁决 | ✅ 交付 | 取消 auto-accept |
| background BDocx 直写盘 | ✅ 交付 | 移除 isDocxPath 阻断 |
| background→open_session 锚点迁移 | ✅ 交付 | block_index→blockId 转换 |
| blockId 标签关闭→重开继承 | ✅ 交付 | closeSession 不 delete |
| replace_block 结构化载荷 | ✅ 交付 | ProseMirror JSON 或受控 DOM patch |
| markDraftSaved 扩展 | ✅ 交付 | pending→accepted on save |
| 分页滚动稳定性 | ✅ 交付 | 四层修复已闭环 |
| FlattenBlocks 列表展平 | ✅ 交付 | orderedList/bulletList |
| GreenReview 多块样式 | ✅ 交付 | 按 styleOpsPayload 块列表标记 |
| tableCell 光标定位 | **1.0-post** | BINDER_BLOCK_TYPES 扩展 |
| 样式子集 html→docx 保真 | ✅ 交付 | color/fontSize/fontFamily/textAlign/highlight/sub/sup 均需在往返中保留（Pandoc Lua filter） |
| 媒体路径闭环 | ✅ 交付 | asset:// → 相对路径反向映射（约20-30行） |
| style_ops open_session/background 路径 | **1.0-post** | 当前仅 current_editor 路径支持 |
| DOCX 转换并发治理 | **1.0-post** | 超时/子进程回收/互斥锁 |
| LibreOffice PDF 验收 | **1.0-post** | 字体/GUI 阻断 |
| BDocx 身份标记 | **1.0** 基础方案 | 显式创建 + 清单；加密签名延后 |
| 草稿冲突处理 | ✅ 交付 | 淘汰删除后重建；工作台监听文件删除 |
| 工作台文件删除监听 | ✅ 交付 | 草稿外部删除→编辑器失效态 |
| 提示词格式信息 | ✅ 交付 | 注入 BDocx/Text/Markdown + 样式能力描述 |
| GreenReview before/after | **1.0** 基础 | 传入文本对比；富文本 diff 延后 |
| 标签集统一 | ✅ 交付 | is_html_block_tag + BLOCK_TAGS 收敛 |
| sessionVersion 粒度 | **不修复** | D4 锚唯一性原则使块级粒度不必要 |
