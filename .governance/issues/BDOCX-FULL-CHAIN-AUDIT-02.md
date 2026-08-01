# BDocx 代理编辑全链路审计报告

**日期**：2026-07-26
**类型**：只读审计
**范围**：BDocx编辑 → 工作台缓存 → 文档交互协议 → 编辑器配置 → 引用处理 → 提示词构建 → 工具调用 → 模型编辑协议 → Diff卡生成 → 效果展示 → 保存链路
**目标**：为下一步编辑优化（稳定代理编辑docx、样式子集兼容、diff精准展示、文档存储链路清晰）提供基础数据

---

## 决策注册表（2026-07-26 人类确认）

| ID | 决策 | 状态 |
|----|------|------|
| D1 | open_session diff 不自动接受，需等用户裁决 | 已确认 |
| D2 | 无逻辑状态→accept 直写盘；有逻辑状态→走逻辑状态链（BDocx 也不例外） | 已确认 |
| D3 | background diff 在文档被打开时自动映射到逻辑状态 | 已确认 |
| D4 | diff 锚唯一性原则：锚定内容变化→diff 失效→绿增消失→DiffCard 进入失效态（只展示不可操作）。同一内容不可能产生两条 diff（diff 返回即修改逻辑状态，之前锚定的 diff 自动失效） | 已确认 |
| D4a | 手动编辑时编辑器 tabState 应转 dirty（用户修改了逻辑状态、未落盘）；"不转 dirty"仅指 DiffCard 不存在 dirty 概念。当前 `markDirty()` 中 `tabState !== "pending_diff"` 跳过是错误的——需修复 | 已确认 |
| D5 | 同一文档不允许两个草稿；无独立"创建草稿"按钮——唯一入口是原生 DOCX 预览页"编辑"按钮：草稿已存在→直接跳到草稿编辑窗口；草稿不存在→创建后进入编辑。模型收到编辑原生 DOCX 指令→后台自动创建草稿（需用户授权） | 已确认 |
| D5a | 工作台监听文件系统删除事件：草稿被外部删除→编辑器显示失效态、阻断编辑、不读缓存 | 已确认 |
| D6 | BDocx 必须显式创建（草稿按钮/工作台+/AI交互），原生 DOCX 改名不走 BDocx；需文件级识别标记 | 已确认 |
| D7 | 模型返回→直接修改 LogicalState + DiffCard + 绿增；接受=仅状态变化；拒绝=回滚 LogicalState | 已确认 |
| D8 | 保存→逻辑状态写盘→绿增消失；pending diff→auto-accept | 已确认 |
| D9 | 应用关闭→diff 终裁/丢弃；标签关闭→diff 保留、blockId 需继承 | 已确认 |
| D10 | replace_block 载荷改为 HTML innerHTML（保留内联 marks/attrs），blockId+基线校验；模块头注释"不得纯文本替换" | 已确认 |
| D11 | 标签关闭 session 全部保留（text/markdown/html 所有格式） | 已确认 |
| D12 | 媒体闭环：asset↔相对路径 映射表 + 保存时反向映射 + Pandoc --resource-path，纳入 1.0 | 已确认 |
| D13 | 保存不检测外部修改冲突，直接覆盖写盘（保存=全文覆盖）；文件被删除除外（删除→失效态，无法保存） | 已确认 |
| D14 | 关闭后外部修改 DOCX → 重开时不检测冲突，直接使用缓存逻辑状态（外部修改对逻辑状态无影响）。可记录债务 | 已确认 |
| D15 | 应用重启后历史 diff 有记录（显示正确的 diff 卡状态，如失效态），但不继承编辑功能 | 已确认 |
| D16 | **1.0 必须实现样式子集兼容继承**（color/fontSize/fontFamily 等需在 html→docx 往返中保留）。不可降为"编辑可加但保存丢"。原 P0#7 升级为 1.0 阻断 | 已确认 |

---

## 执行摘要

审计覆盖全链路 12 个环节、50+ 份文件。发现 P0 阻断点 8 个、P1 高风险 11 个、P2 中等脆弱点 7 个。

**严重问题**（按当前版本阻断程度排序）：
1. **open_session diff 到达即自动接受**——用户完全没有裁决机会
2. **background draft BDocx 拒绝**——BDocx background diff accept 时被 isDocxPath 阻止
3. **replace_block 以纯文本替换块 inner HTML**——丢失加粗、链接、颜色等富文本
4. **样式子集 html→docx 保真度为 0**——color/fontSize/fontFamily 保存后全部丢失
5. **Pending diff 保存时未 auto-accept + 绿增残留**——`markDraftSaved` 只处理 accepted 态
6. **媒体路径不可逆**——asset:// URL 不被 Pandoc 识别，保存后图片丢失
7. **background→open_session 锚点迁移缺失**（窗口有限，降级处理）

---

## 全链路环节审计

### 环节 1：BDocx 编辑器加载与编辑态

**状态**：基本完整（A-）

**关键发现**：

| 项 | 发现 | 风险 |
|----|------|------|
| 加载门禁 | `isEditorContentReady` 三重门禁：loadedContent非空 + loadSeq>0 + settledLoadSeq匹配 | 低 |
| loadSeq机制 | 每次loadDoc/setActivePath递增，Editor仅依赖loadSeq变更setContent | 稳定 |
| 标签切换 | lastPathRef暂存草稿+scrollMemo暂存滚动位置 | 健全 |
| 加载超时 | 加载骨架无超时恢复——后端转换不返回时用户永久卡在加载态 | **中** |
| generateJSON异常 | setContent未try-catch——解析失败可能导致编辑器静默进入异常状态 | **中** |
| pending_diff编辑 | pending_diff态下手动修改不转dirty | 低 | **D4a 已澄清**："不转 dirty"仅指 DiffCard 无 dirty 概念。编辑器 tabState 必须转 dirty（逻辑态≠磁盘态）。当前 `markDirty()` 跳过 `pending_diff` 需修复 |

### 环节 2：工作台缓存

**状态**：基本可用（B）

| 项 | 发现 | 风险 |
|----|------|------|
| sessionVersion防冲突 | 文档级而非块级——一个块编辑导致所有其他块待裁决diff失效 | **已由D4消解**——锚唯一性原则使块级粒度不必要 |
| 异常退出恢复 | dirty快照在启动清理中被无条件删除——用户无法恢复未保存内容 | **中** |
| 多文档会话隔离 | patchSession按路径锁定，切换标签不污染他会话 | 健全 |
| OpenDocSnapshot | 传递给AG的read context，优先用编辑器实时内容，未找到才走磁盘 | 正确 |

### 环节 3：文档交互协议

**状态**：设计文档覆盖全面，存在3处严重漂移（A-）

| 项 | 发现 | 风险 |
|----|------|------|
| AG-I-P-01 OpenDocSnapshot | Rust结构体缺少content_format字段，SYS-I-P-01有定义 | **高** |
| BlockRef | AG-I-P-01定义但未在SYS-I-P-01注册，违反X-SHARED-001 | **高** |
| propose_edit.operation | 缺少replace_text，ED-I-P-01已定义该操作粒度 | **高** |
| TERM-ED-001 Block | 未反映v1.34引入的容器块/内容块两级分类 | 中 |
| block_index语义分歧 | ED（分页position序号）vs AG/DE（投影列表序号）同名不同义 | 中 |

### 环节 4：编辑器基础配置

**状态**：扩展注册完整，存在表格相关缺口（B+）

| 项 | 发现 | 风险 |
|----|------|------|
| BINDER_BLOCK_TYPES | 覆盖7种类型（含ul/ol容器块），缺tableCell/tableHeader | **中** |
| resolveContentBlock | table结构内的光标回退到table顶层块，charOffsetInBlock计算错误 | **高** |
| findTopBlockSnapshotById | 已修复为descendants递归搜索（2026-07-25） | 已闭环 |
| getTopBlockSnapshotByIndex | 已修复为flat block列表（2026-07-25） | 已闭环 |
| Pagination分页 | end-fill→CSS变量、flattenBlocks、overflow-anchor、handleScrollToSelection四层修复 | **已闭环** |

### 环节 5：引用处理方式及路径

**状态**：基本完整，DOM回落路径有局限（B+）

| 项 | 发现 | 风险 |
|----|------|------|
| getCursorRef | resolveContentBlock递归穿透容器，支持嵌套块定位 | 稳定 |
| getSelectionRef | text.trim()过滤全空白选区——可能丢失合法引用 | 低 |
| charOffsetInBlock | LEAF_CHAR占1字符语义一致，但offset落在叶子中间时PM position无法精确表达 | 低 |
| DOM回落路径 | 仅能获取blockId（data-block-id属性），无法计算偏移 | **中** |
| tableCell定位 | BINDER_BLOCK_TYPES不含tableCell，table结构内光标回退错误 | **高** |

### 环节 6：提示词构建

**状态**：分层骨架清晰，缺失BDocx格式信息（B+）

| 项 | 发现 | 风险 |
|----|------|------|
| 四套系统提示词 | Chat/Build/InlineAssist/Autocomplete均已定义 | 健全 |
| 能力边界声明 | 已有style_ops声明但缺少具体mark类型清单和格式能力矩阵 | **中** |
| bdocx标识 | editable_kind="bdocx"未注入提示词 | **中** |
| blockId/blockIndex | 提示词声明"优先block_id"但未说明text文档无blockId时须用index | **中** |

### 环节 7：工具调用

**状态**：7工具已接线，propose_edit功能缺口（B）

| 项 | 发现 | 风险 |
|----|------|------|
| 工具注册表 | 7工具+3级权限，validate_requested_tool放行propose_edit | 健全 |
| propose_edit条件必填 | replace_block需new_text+锚，JSON Schema required数组未表达条件依赖 | **中** |
| style_ops缺乏跨端校验 | range偏移是否与后端块内纯文本长度一致无校验 | **中** |
| list_blocks block_id可为null | text文档无blockId，模型需回退到block_index但不能保证 | **中** |

### 环节 8：模型编辑协议

**状态**：三路径模型定义清晰，style_ops路径不完整（B+）

| 项 | 发现 | 风险 |
|----|------|------|
| diff三路径 | current_editor/open_session/background均有完整生命周期 | 健全 |
| style_ops open session | 完全不支持——applyOpenSessionDiff中直接expire | **高** |
| baseSessionVersion校验 | 存在但粒度过粗（文档级） | **中** |
| replace_document | 整篇替换无文档身份校验 | 低 |

### 环节 9：Diff卡生成与效果展示

**状态**：基本可用，GreenReview覆盖有缺口（B）

| 项 | 发现 | 风险 |
|----|------|------|
| GreenReview装饰 | blockId命中→绿色背景；blockId缺失→全块标记 | 健全 |
| 多块样式修改 | GreenReview只覆盖d.blockId指定单块，不跟随styleOpsPayload的blockId列表 | **高** |
| 整篇替换GreenReview | 全部currentBlockIds标绿——无法区分用户编辑与模型修改 | **中** |
| before/after预览 | 纯样式diff中before/after可能为空，preview无意义 | 低 |
| 锚失效error卡 | 用户只知道"缺少精准编辑目标"，不知道原因且无法一键重试 | **中** |

### 环节 10：效果渲染

**状态**：DiffDecorationExtension提供基础装饰（B）

| 项 | 发现 | 风险 |
|----|------|------|
| 装饰机制 | Decoration.node整块覆盖+Decoration.widget块后预览 | 健全 |
| 容器块装饰 | orderedList/bulletList有blockId→可能被DE推送装饰，视觉表现不确定 | **中** |
| Preview widget重叠 | 相邻块的preview widget堆叠在同一位置 | 低 |
| 装饰与用户编辑的交互 | 装饰随editor.docChanged通过DecorationSet.map调整位置 | 正确 |

### 环节 11：HTML→Docx保存链路

**状态**：需修复（C）

| 项 | 发现 | 风险 |
|----|------|------|
| data-block-id往返 | Pandoc HTML→docx丢弃自定义属性——blockId完全丢失 | **严重** |
| 样式往返保真 | color/fontSize/fontFamily在html→docx方向保留率=0（Pandoc不映射内联CSS到docx格式） | **严重** |
| 图片路径 | docx_to_html解压到临时目录→临时目录被清理后保存时图片路径断裂 | **中** |
| 原子保存 | 写临时文件+rename——写入中断不产生半落盘 | 健全 |
| 冲突检测 | baseline参数做内容哈希比对——WS_SAVE_CONFLICT正确拒绝 | 健全 |
| 并发保存 | 临时文件固定.`.<filename>.tmp` 命名，无请求级隔离；并发保存互相覆盖 | **中** |
| 媒体路径 | 打开时相对图片路径被改写为 `asset://` URL；修复：保持 asset↔相对路径 映射表，保存时反向映射 + Pandoc `--resource-path` | **纳入1.0** |
| 转换并发 | 打开转换无超时；保存超时不终止子进程；临时文件仅按 PID 命名 | **P1** |
| replace_block | 以 `html_escape` + 纯文本替换块 inner HTML——丢失加粗/链接/颜色等富文本 | **P0** |
| 裁决模型 | saveCurrent 允许 pending_diff 保存 ✅；但 markDraftSaved 只设 draftSaved 于 accepted 态，pending diff 保存后绿色残留且未 auto-accept | **P0** |
| open_session | diff 到达即自动 apply + auto-accept——用户零裁决机会 | **P0** |
| b-docx身份 | 重命名 DOCX 即自动登记为 b-docx；清单仅文件读取失败视为空；删除文件不清理清单 | **P2** |

### 环节 12：后端Blocks解析

**状态**：基本可用，标签集分裂（B）

| 项 | 发现 | 风险 |
|----|------|------|
| 标签集分裂 | is_html_block_tag有td/th无table；BLOCK_TAGS有table无td/th | **高** |
| 实体解码 | 仅7种实体，数值实体/常用HTML实体退化为残留&xxx;字符串 | **中** |
| 手工字符串扫描 | 不处理HTML注释/script/style内的<标签 | 低（当前场景无） |
| replace_block_by_id | blockId不存在→Err("锚失效")——不会静默产生错位diff | 正确 |

---

## 关键阻断点排名

### P0（严重阻断——代理编辑无法可靠工作，违反核心契约）

| # | 问题 | 位置 | 影响 |
|---|------|------|------|
| 1 | **Pending diff 保存时未 auto-accept + 绿增残留** | Editor.tsx:1075, deStore.ts:320-325 | saveCurrent 正确允许 pending_diff 保存，但 markDraftSaved 只处理 accepted 态；pending diff 保存后绿色残留且未 auto-accept |
| 2 | **open_session diff 到达即自动接受** | agService.ts:970, deService.ts:198 | **D1 已决策**：取消 auto-accept，改为产生 DiffCard 等用户裁决 |
| 3 | **background draft BDocx 拒绝** | deService.ts:273-275 | **D2 已决策**：无逻辑状态→直写盘（BDocx 也不例外）；有逻辑状态→走逻辑状态链。移除 BDocx background 的 isDocxPath 阻断 |
| 4 | **background→open_session 锚点迁移缺失** | deService.ts, Editor.tsx | **D3 已决策**：文档被打开时自动将 background diff 映射到逻辑状态。需实现 block_index→blockId 转换 |
| 5 | **blockId 在标签关闭→重开时丢失** | edStore.ts:432 + edit_chain.rs | 详见下方分析 |

### P0#5 详细分析：blockId 标签关闭→重开丢失

**现象**：BDocx 文档标签关闭后重新打开，之前 pending diff 的 blockId 锚全部失效。

**根因**：双重原因叠加——

1. **[edStore.ts:432](file:///Users/imatstarbucks/binder/src/store/edStore.ts#L432) `closeSession` 直接 delete session**，丢弃了缓存的 HTML 逻辑状态：
```typescript
closeSession(path: string) {
  const sessions = { ...state.sessions };
  delete sessions[path];  // ← 逻辑状态被丢弃
  commit({ sessions, activePath: ..., loadSeq, jumpLine });
},
```

2. 标签重开时走 `openDoc` + `loadDoc`（读盘 → Pandoc DOCX→HTML → `inject_missing_data_block_ids` 全新生成 blockId），而非走 `setActivePath`（从缓存 session.content 装载）。

**历史行为**：标签关闭时应保留 session.content（与 `setActivePath` 注释一致："切换活动会话到已打开文档，不重新读盘 → 保留该会话未保存编辑"）。`closeSession` 中新增的 `delete sessions[path]` 破坏了这一缓存链。

**修复路径**：

| 方案 | 改动 | 成本 | 推荐 |
|------|------|------|------|
| `closeSession` 不 delete（保留 session） | 1 行删除 | 零 | ✅ |
| `closeSession` 加 `preserveContent` 标志，BDocx 标签关闭时保留 | 3-5 行 | 极低 | 可替代 |
| Pandoc 往返保留 data-block-id（Lua filter 持久化到 docx 自定义属性） | 百行级 | 高 | 长期方案 |

**副作用评估**：保留 session 意味着 HTML 内容常驻内存（KB~MB 级，可忽略）。workspace 切换时 `reset()` 已全清 session，不会跨 workspace 污染。

### P0 阻断点排名（续）
| 6 | **replace_block 以纯文本替换块 inner HTML** | blocks.rs:311 | 代理改一段文本时丢失该段已有加粗、链接、颜色、行内图片等富文本 |
| 7 | **样式子集 html→docx 保真度=0** | edit_chain.rs + style_subset.rs | **D16 已决策：1.0 必须实现样式兼容继承**。需 Pandoc Lua filter 保留内联 CSS（color/fontSize/fontFamily/textAlign 等），不可降为"编辑可加但保存丢" |
| 8 | **媒体路径不可逆（asset:// URL）** | docService.ts:49, edit_chain.rs:142 | 前端改写 asset:// 但保存时未反向映射回相对路径。修复：保持映射表 + 保存时反向映射 + Pandoc --resource-path**（不必然丢失，约 20-30 行，纳入 1.0 交付）** |

### P1（高风险——特定场景失效）

| # | 问题 | 位置 |
|---|------|------|
| 7 | tableCell 光标定位回退错误 | edLocate.ts resolveContentBlock |
| 8 | 标签集分裂（is_html_block_tag vs BLOCK_TAGS） | blocks.rs vs block_backfill.rs |
| 9 | 多块样式修改 GreenReview 只覆盖单块 | Editor.tsx:1184-1185 |
| 10 | 文档格式标识未注入提示词（bdocx vs text） | prompts.rs |
| 11 | sessionVersion 文档级粒度过粗（一个块编辑使所有 diff 失效） | edStore.ts applySessionContent |
| 12 | replaceHtmlBlock 与 parse_html_blocks 块切分不一致 | deService.ts vs blocks.rs |
| 13 | DOCX 转换缺少并发治理（无超时、不杀子进程、临时文件只用 PID 命名） | edit_chain.rs |
| 14 | StyleSubset 不可交付（仅 heading/bold Verified；其余 RequiredPending） | style_subset.rs |
| 15 | DOCX 预览被 LibreOffice 稳定性阻断（无法可靠 PDF 对比验收） | DOC-I-P-01 |
| 16 | GreenReview before/after 内容未传入（仅状态字符串，无文本对比） | Editor.tsx:1180 |
| 17 | `doc.get_style_report` 未注册（设计中要求的能力缺口） | DOC-I-P-01

### P2（中等风险——体验与健壮性）

| # | 问题 | 位置 |
|---|------|------|
| 18 | 加载骨架无超时恢复 | Editor.tsx isEditorContentReady |
| 19 | 测量异常静默吞没（分页不正确但加载遮罩解除） | Pagination.ts measureAfterLayout |
| 20 | BlockRef 未在 SYS-I-P-01 注册 | AG-I-P-01 vs SYS-I-P-01 |
| 21 | AG-I-P-01 OpenDocSnapshot 缺 content_format 字段 | AG-I-P-01 |
| 22 | b-docx 身份弱（基于文件系统自动发现） | listing.rs:123 | **D6 已决策**：BDocx 必须显式创建。需实现文件级识别标记（非文件名推断），禁止重命名原生 DOCX 自动获得 BDocx 身份 |
| 23 | 并发保存临时文件命名碰撞（固定 .tmp 无请求级隔离） | save.rs:60 |
| 24 | 创建草稿时删除同名后重建 | lib.rs:785 | **D5 已决策**：淘汰删除后重建路径。唯一入口是预览页"编辑"按钮；草稿存在→直接跳到编辑窗口。需加工作台文件删除监听阻断已打开草稿的编辑 |

---

## 与优化目标的差距矩阵

| 优化目标 | 当前状态 | 差距 | 优先级 |
|---------|---------|------|--------|
| 裁决模型 | markDraftSaved 只处理 accepted 态 diff，pending 保存后绿增残留且未 auto-accept；open_session zero-review | markDraftSaved 扩展 pending→accepted on save；open_session 需裁决机会 | P0 |
| open_session 需用户裁决 | diff 到达即 apply + auto-accept，零裁决机会 | 取消 auto-accept；open_session diff 必须产生 DiffCard 等待裁决 | P0 |
| 稳定代理编辑 docx | blockId 往返丢失导致跨保存锚失效 | 需解决 blockId 持久化（Pandoc Lua filter 或后处理注入） | P0 |
| 富文本块编辑不丢失样式 | replace_block 用 HTML escape + 纯文本替换 inner | 改为 ProseMirror JSON 或受控 DOM patch 表达块替换 | P0 |
| 样式子集兼容适配编辑 | color/fontSize/fontFamily 在 html→docx 方向丢失；仅 heading/bold Verified | 需 Pandoc Lua filter 保留样式 + round-trip 语料测试 + StyleSubset 五档报告 | P0 |
| 媒体路径闭环 | asset:// URL 在保存时不被 Pandoc 识别 | 保持 asset↔相对路径 映射表；保存时反向映射 + --resource-path | P0→1.0交付 |
| 工具健全且稳定 | 7 工具已接线，propose_edit 条件必填缺校验 | 补参数校验 + style_ops 全路径支持 + 替换载荷改为结构化 | P1 |
| Diff 效果展示准确细致 | 多块样式标记覆盖不足、before/after 未传入、整篇替换粗粒度 | 改 GreenReview 按 styleOpsPayload 块列表标记 + 传入 before/after 文本 | P1 |
| 文档存储链路清晰 | 原子保存健全、冲突检测正确 | sessionVersion 需块级粒度；并发保存加互斥锁；docx 转换加超时/子进程回收 | P1 |
| DOCX 预览可验收 | LibreOffice 无 GUI/字体阻断；无法可靠 PDF 对比 | 闭合包内 LO 可靠性 → PDF → 像素/文本/XML 三层比对 | P2 |
| b-docx 身份可靠 | 重命名 DOCX 自动登记；清单仅文件不清理 | 改为受迁移保护的原子读写控制态 | P2 |

## 建议实施顺序

前四项为"稳定代理编辑 DOCX"入口门禁：

1. **补全裁决模型**：markDraftSaved 覆盖 pending diff（save 时 auto-accept + greenReview清除）；open_session 需用户裁决机会而非 auto-accept；D8 补充 docx 保存后差量消解
2. **统一结构化编辑载荷**：以 ProseMirror JSON 或受控 DOM patch 表达块替换，携带 marks、attrs、blockId 与基线；禁止字符串扫描和 plain-text 覆盖富文本块
3. **建立媒体资源会话**：编辑模型使用逻辑媒体 ID；显示时映射 asset URL；保存时反向解析到真实抽取文件并重嵌入 DOCX；会话结束清理临时媒体与 asset 授权
4. **将 StyleSubset 变为真实协议**：特征扫描 → 五档报告 → 前端告知 → 可编辑样式白名单 → round-trip 语料测试；达标前只允许已验证子集的 Agent style_ops
5. **分离并串行化转换任务**：每次请求使用 UUID 私有目录/输出文件；打开和保存都有超时、取消、子进程回收；同文档保存加互斥锁
6. **重做 diff 呈现**：块级 rich before/after、字符级 mark diff、表格/图片专用展示；整篇替换必须有独立全文审阅视图
7. **使 b-docx 身份成为受迁移保护的控制态**：原子读写、create/rename/delete 全生命周期同步、草稿冲突显式提示
8. **闭合包内 LibreOffice 稳定性**：无 GUI/字体可靠性 → PDF 像素/文本/XML 三层比对作为 DOCX AgentEdit 的 G5 验收

---

## 审计范围清单

**已审计文件**（按链路顺序）：
- Editor.tsx, Pagination.ts, paginationFlow.ts, edLocate.ts, edStyleOps.ts, edService.ts
- edStore.ts, assistStore.ts, deStore.ts, outlineStore.ts, edViewPrefsStore.ts
- BlockIdExtension.ts, DiffDecorationExtension.ts, extensions/index.ts
- referencePayload.ts, deService.ts, agService.ts
- blocks.rs, block_backfill.rs, edit_chain.rs, style_subset.rs
- de/mod.rs, de/generate.rs, de/expiry.rs, de/lifecycle.rs, de/store.rs
- ag/mod.rs, ag/turn.rs, ag/context.rs, ag/prompts.rs, ag/tool.rs, ag/tool_registry.rs, ag/tool_exec.rs, ag/assist.rs, ag/history.rs, ag/memory_link.rs
- workspace/save.rs, workspace/diff_apply.rs, workspace/listing.rs
- A-ED-M-T-01, A-ED-M-D-01, A-ED-I-P-01, A-DE-M-T-01, A-DE-M-S-01, A-DE-I-P-01
- A-AG-M-T-01, A-AG-M-T-02, A-AG-M-S-01, A-AG-I-P-01
- A-SYS-I-P-01, A-CORE-X-R-01, A-CORE-C-P-01, CLAUDE.md
- styles.css

**未审计**（不在本次范围或需未来专项审计）：
- EditorToolbar, EditorStatusBar 纯UI组件
- assistPlacement.ts 浮层定位
- agStore.ts AG状态机前端投影
- PrvPanel 预览面板
- 后端Tauri command注册与IPC路由
