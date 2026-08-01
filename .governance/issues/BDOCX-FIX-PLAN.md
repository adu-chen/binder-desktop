# BDocx 代理编辑修复优化方案

**日期**：2026-07-26
**基于**：BDOCX-FULL-CHAIN-AUDIT-02.md + BDOCX-EDIT-EXPECTED-BEHAVIOR.md + 决策注册表 D1-D16
**状态**：方案，本轮不修改

---

## 修复总览

| 轮次 | 项目数 | 依赖 | 预计改动量 |
|------|--------|------|-----------|
| R1 基础修复 | 4 | 无 | ~45 行 |
| R2 Diff 生命周期 | 4 | R1 | ~30 行 |
| R3 编辑载荷与展示 | 4 | R2 | ~80 行 |
| R4 样式与质量 | 4 | R2-R3 | ~150 行 |
| R5 1.0-post | 4 | — | 另行评估 |

---

## 第 1 轮：基础修复（无依赖，可并行）

### F1.1 — closeSession 不 delete session

**问题**：P0#5 — 标签关闭时 `delete sessions[path]` 丢弃缓存逻辑状态，重开时从磁盘重建，blockId 全换，pending diff 锚失效。

**当前代码**：[edStore.ts:432-442](file:///Users/imatstarbucks/binder/src/store/edStore.ts#L432-L442)
```typescript
closeSession(path: string) {
  if (!state.sessions[path]) return;
  const sessions = { ...state.sessions };
  delete sessions[path];    // ← 删除此行
  commit({
    sessions,               // sessions 不变（含原 path）
    activePath: state.activePath === path ? null : state.activePath,
    loadSeq: state.loadSeq,
    jumpLine: state.jumpLine,
  });
},
```

**修复**：移除 `delete sessions[path]`，session 保留在 `sessions` map 中。标签关闭后仅清 `activePath`。重开时 `setActivePath` 从缓存装载，blockId 不变。

**副作用**：workspace 切换时 `reset()` 已全清 sessions。对于关闭标签后永不重开的文档，session 内存泄漏可接受（逐个 session 几 KB~几 MB）。

**决策**：D11 — 全部格式保留（text/markdown/html）。

---

### F1.2 — markDirty 不跳过 pending_diff

**问题**：D4a — 编辑器 tabState 在用户手动编辑后必须转 dirty（逻辑态≠磁盘态）。当前 `markDirty()` 跳过 `pending_diff` 是错误的。

**当前代码**：[edStore.ts:420-424](file:///Users/imatstarbucks/binder/src/store/edStore.ts#L420-L424)
```typescript
markDirty() {
  const sess = state.activePath ? state.sessions[state.activePath] : undefined;
  if (sess && sess.tabState !== "pending_diff") {  // ← 删除此条件
    patchActive({ tabState: "dirty" });
  }
},
```

**修复**：移除 `tabState !== "pending_diff"` 条件判断，恢复为无条件 `patchActive({ tabState: "dirty" })`。

**解释**：D4 锚唯一性原则使旧 diff 在逻辑状态变化后自动失效，与 tabState 无关。tabState 只代表"逻辑态 vs 磁盘态"差异，无论 diff 状态如何，手动编辑都产生此差异。

---

### F1.3 — 统一标签集

**问题**：P1#8 — `is_html_block_tag`（blocks.rs:369）有 `td`/`th` 无 `table`；`BLOCK_TAGS`（block_backfill.rs:63）有 `table` 无 `td`/`th`。两处标签集不一致。

**修复 A**：[blocks.rs:369-386](file:///Users/imatstarbucks/binder/src-tauri/src/ag/blocks.rs#L369-L386) `is_html_block_tag`：增加 `"table"` 到 matches 列表。

**修复 B**：[block_backfill.rs:63-78](file:///Users/imatstarbucks/binder/src-tauri/src/doc/block_backfill.rs#L63-L78) `BLOCK_TAGS`：增加 `"td"`、`"th"` 到数组。

**统一后标签集**：`p | h1 | h2 | h3 | h4 | h5 | h6 | li | blockquote | pre | div | td | th | ul | ol | table`（16 个）

**不确定项**：`td`/`th` 在 backfill 中注入 blockId 是否正确？table 内的 td/th 是内容单元，blockId 应归于 td/th 还是委托给内部 paragraph？当前设计是 td/th 在 blocks.rs 被解析为独立块（有 inner 区间），paragraph 在其内部有独立 blockId。**建议仅加 `table` 到 `is_html_block_tag`、加 `td`/`th` 到 `BLOCK_TAGS`**，与 blocks.rs 当前语义一致。

---

### F1.4 — 媒体 asset:// 反向映射

**问题**：P0#8 — `rewriteMediaSrc` 把相对路径改写为 `asset://` URL 用于 WebView 显示，但保存时未反向映射回相对路径，Pandoc 不识别 `asset://`。

**当前代码**：
- [docService.ts:49-60](file:///Users/imatstarbucks/binder/src/services/docService.ts#L49-L60) 正向改写（相对→asset）
- [edit_chain.rs:142-197](file:///Users/imatstarbucks/binder/src-tauri/src/doc/edit_chain.rs#L142-L197) `html_to_docx`：设置 `current_dir` 但未设置 `--resource-path`

**修复 A**：前端 — [docService.ts](file:///Users/imatstarbucks/binder/src/services/docService.ts)
- `rewriteMediaSrc` 返回值从 `string` 改为 `{ html: string; assetMap: Map<string, string> }`（asset URL → 相对路径）
- 新增 `restoreMediaSrc(html: string, assetMap: Map<string, string>): string` 反向映射函数

**修复 B**：前端 — [edService.ts](file:///Users/imatstarbucks/binder/src/services/edService.ts) `saveDocxDocument`
- 调用 `restoreMediaSrc` 将 HTML 中 asset:// URL 还原为相对路径后再传给后端

**修复 C**：后端 — [edit_chain.rs:142](file:///Users/imatstarbucks/binder/src-tauri/src/doc/edit_chain.rs#L142) `html_to_docx`
- 增加 `--resource-path` 参数指向媒体解压目录
- 或在已有 `current_dir` 基础上确认 Pandoc 能解析相对路径（当前已设置 `current_dir`，如果媒体文件在同一目录下且 HTML 中 src 为纯文件名或相对路径，理论上已可通过 current_dir 解析）

**不确定项**：`current_dir` 是否已足够让 Pandoc 解析相对图片路径？需要实际验证：如果在 `/tmp/binder_media` 下有 `image1.png`，HTML 中有 `<img src="image1.png">`，且 `current_dir` 设为 `/tmp/binder_media`，Pandoc 是否能找到图片？如果不能，需加 `--resource-path`。

**决策**：D12 — 纳入 1.0 交付。

---

## 第 2 轮：Diff 生命周期修复（依赖 R1）

### F2.1 — markDraftSaved 扩展 pending→accepted

**问题**：P0#1 — `markDraftSaved` 只设 `draftSaved` 于 `state === "accepted"` 的 diff。pending diff 保存后绿色残留且未 auto-accept。

**当前代码**：[deStore.ts:320-326](file:///Users/imatstarbucks/binder/src/store/deStore.ts#L320-L326)
```typescript
markDraftSaved(relPath: string) {
  setDiffs(
    state.diffs.map((d) =>
      d.relPath === relPath && d.draftApplied && d.state === "accepted"
        ? { ...d, draftSaved: true }
        : d,
    ),
  );
},
```

**修复**：扩展条件为 `d.draftApplied && (d.state === "accepted" || d.state === "pending")`，pending 态同时设 `state: "accepted"` + `draftSaved: true`。

```typescript
markDraftSaved(relPath: string) {
  setDiffs(
    state.diffs.map((d) => {
      if (d.relPath !== relPath || !d.draftApplied) return d;
      if (d.state === "accepted") return { ...d, draftSaved: true };
      if (d.state === "pending") return { ...d, state: "accepted", draftSaved: true };
      return d;
    }),
  );
},
```

**解释**：D8 — 保存→pending diff auto-accept + 绿增消失。Editor effect 中 GreenReview 构建已通过 `d.draftSaved` 跳过已保存 diff（[Editor.tsx:1183](file:///Users/imatstarbucks/binder/src/components/ed/Editor.tsx#L1183)），无需额外修改。

**决策**：D8。

---

### F2.2 — open_session 取消 auto-accept

**问题**：P0#2 — `agService.ts:970` 在 diff_proposed 事件中检测 `draft_mode === "open_session"` → 自动调用 `applyOpenSessionDiff` + `de.accept_draft_diff`，用户零裁决。

**当前代码**：[agService.ts:960-972](file:///Users/imatstarbucks/binder/src/services/agService.ts#L960-L972)
```typescript
readonly: projection?.draft_mode === "open_session",
// ...
if (state === "pending" && projection?.draft_mode === "open_session") {
  void applyOpenSessionDiff(ev.diff_id);  // ← 删除此 auto-trigger
}
```

**修复**：删除 `diff_proposed` 事件处理中的 `applyOpenSessionDiff` 自动调用。open_session diff 到达后仅产生 DiffCard，等待用户裁决。

**连带修改**：
- [deService.ts:261-263](file:///Users/imatstarbucks/binder/src/services/deService.ts#L261-L263) `acceptDiff` 中的 `draftMode === "open_session"` 分支：当前调用 `applyOpenSessionDiff`，改为走标准三态流——diff 应用逻辑状态 + wait for accept（与 `current_editor` 路径一致）
- 移除 `deStore.markOpenSessionApplied` 的自动 accept 行为；改为 `markDraftApplied`（仅记录应用，不自动接受）

**决策**：D1。

---

### F2.3 — BDocx background 移除 isDocxPath 阻断

**问题**：P0#3 — `deService.ts:273-275` `isDocxPath` 检查阻止 BDocx background diff 走 `de.accept_diff` 直写盘路径。

**当前代码**：[deService.ts:269-276](file:///Users/imatstarbucks/binder/src/services/deService.ts#L269-L276)
```typescript
if (diff?.relPath && edStore.getSnapshot().sessions[diff.relPath]) {
  deStore.setLocalView(diffId, "error", { errorCode: "DE_OPEN_DOC_REQUIRES_LOGICAL_STATE" });
  return;
}
if (diff && isDocxPath(diff.relPath)) {         // ← 有逻辑状态但非活动
  deStore.setLocalView(diffId, "error", { errorCode: "DE_BDOCX_DRAFT_REQUIRED" });
  return;
}
```

**分析**：第 269-272 行已正确拦截有 session（逻辑状态存在但非活动编辑器）的 diff（需走 open_session 路径）。第 273-275 行额外拦截 BDocx 是无必要的——如果 session 不存在（已验证），说明文档从未打开，应允许直写盘。

**修复**：删除 `isDocxPath` 阻断（第 273-275 行）。保留第 269-272 行的 session 存在性检查即可覆盖所有需拦截场景。

**决策**：D2 — 无逻辑状态→直写盘；有逻辑状态→走逻辑状态链。

---

### F2.4 — background→open_session 锚点迁移

**问题**：P0#4 — background diff 基于磁盘快照生成（无编辑器 blockId）。文档后来被打开，BlockIdExtension 新生成 blockId，旧锚（block_index）需映射到新 blockId。

**触发时机**：[edStore.ts:238-257](file:///Users/imatstarbucks/binder/src/store/edStore.ts#L238-L257) `loadDoc` 设置内容后，检测 deStore 中是否有该 path 的 background diff。

**修复**：[Editor.tsx](file:///Users/imatstarbucks/binder/src/components/ed/Editor.tsx) 增加 effect：
- 依赖 `openPath + loadSeq` 变化（文档被打开/切换）
- 遍历 deStore 中 `d.relPath === openPath && d.draftMode === "background" && d.state === "pending"`
- 对每个 diff，用当前编辑器的 `currentBlockIds` 和 block_index 做映射：
  - 如果有 `d.blockIndex`，从编辑器 `getTopBlockSnapshotByIndex(d.blockIndex)` 取当前 blockId
  - 更新 diff 的 `blockId` 和 `draftMode`（从 background 变为 current_editor/open_session）
  - 触发 pending diff effect（F2.1 已扩展的路径）

**决策**：D3 — 自动映射到逻辑状态。

---

## 第 3 轮：编辑载荷与展示修复（依赖 R2）

### F3.1 — replace_block 结构化载荷

**问题**：P0#6 — `blocks.rs:461` `html_escape(new_text)` 纯文本替换块 inner HTML，丢失加粗/链接/颜色等富文本。

**当前代码**：[blocks.rs:445-467](file:///Users/imatstarbucks/binder/src-tauri/src/ag/blocks.rs#L445-L467)
```rust
Ok(format!("{}{}{}", &content[..s], html_escape(new_text), &content[e..]))
```

**修复**：`replace_block_by_id` 和 `replace_block` 接受 `new_html: &str` 参数（保留内联 HTML 标签），取消 `html_escape`。

- 函数签名改为 `new_html: &str`（语义：保留 innerHTML 而非纯文本）
- 移除 `html_escape` 调用，直接拼接 `new_html`
- 模块头注释增加"不得纯文本替换，new_html 应保留原块内联样式/marks/图片等"

**安全考量**：`new_html` 来自模型输出，需在**前端**做 sanitize（保留安全标签 p/h1-h6/strong/em/u/s/sub/sup/code/a/span/img/br，剥离 script/iframe/on* 等）。当前后端无 sanitize 逻辑，需在[agService.ts parseDiffProjectionResult](file:///Users/imatstarbucks/binder/src/services/agService.ts) 或 [deService.ts applyOpenSessionPatch](file:///Users/imatstarbucks/binder/src/services/deService.ts) 增加 HTML sanitize。

**不确定项**：sanitize 的标签白名单是否需要与 `is_html_block_tag` 对齐？还是限制为内联标签（strong/em/u/span/a/img）？

**决策**：D10。

---

### F3.2 — replaceHtmlBlock 与 parse_html_blocks 统一

**问题**：P1#12 — 前端 `replaceHtmlBlock`（deService.ts:88-93）使用正则扫描 `<p|h1-h6|li|blockquote|pre|div|td|th|ul|ol>`；后端 `parse_html_blocks`（blocks.rs:393-441）使用 `is_html_block_tag` 判断。两者标签集不完全一致且解析方式不同。

**修复**：F1.3 统一标签集后，前端 `replaceHtmlBlock` 的标签正则需与 `is_html_block_tag` 保持同步。

**当前正则**（[deService.ts:90-93](file:///Users/imatstarbucks/binder/src/services/deService.ts#L90-L93)）：
```typescript
const tagMatch = rest.match(/^<([a-zA-Z][\w:-]*)\b[^>]*>/);
if (!tagMatch) { /* skip */ }
const tag = tagMatch[1].toLowerCase();
if (!/^(p|h[1-6]|li|blockquote|pre|div|td|th|ul|ol)$/.test(tag)) continue;
```

**修复**：正则增加 `table`，统一为 16 标签集。

---

### F3.3 — GreenReview 多块样式标记

**问题**：P1#9 — `Editor.tsx:1184-1185` GreenReview 只按 `d.blockId` 标绿单块。style_ops 修改多个块时，只有 `d.blockId` 对应的块显示绿色。

**当前代码**：[Editor.tsx:1180-1191](file:///Users/imatstarbucks/binder/src/components/ed/Editor.tsx#L1180-L1191)
```typescript
if (d.blockId && d.draftApplied) {
  map.set(d.blockId, gr);
  continue;
}
```

**修复**：当 `d.styleOpsPayload?.length` 存在时，遍历 `styleOpsPayload` 中的所有 `blockId`，分别 set 到 map。

```typescript
if (d.blockId && d.draftApplied) {
  if (d.styleOpsPayload?.length) {
    for (const op of d.styleOpsPayload) {
      if (op.blockId) map.set(op.blockId, gr);
    }
  }
  map.set(d.blockId, gr);
  continue;
}
```

---

### F3.4 — 提示词注入格式信息

**问题**：P1#10 — 提示词未注入 BDocx/Text/Markdown 格式标识和具体样式能力矩阵。

**当前代码**：
- [prompts.rs](file:///Users/imatstarbucks/binder/src-tauri/src/ag/prompts.rs) 系统提示词为静态文本
- [context.rs](file:///Users/imatstarbucks/binder/src-tauri/src/ag/context.rs) `assemble_prompt` 组装上下文

**修复 A**：[prompts.rs](file:///Users/imatstarbucks/binder/src-tauri/src/ag/prompts.rs) 的 `SYSTEM_PROMPT_CHAT` 增加：
- 当前文档格式标识（从 `active_doc_format` 或 `open_doc_snapshots` 推断）
- 可用的 mark 类型清单：bold/italic/underline/strike/highlight/sub/sup/code/link/textStyle（含 color/fontSize/fontFamily）
- propose_edit 使用规则：BDocx→blockId 必须来自 `[id=...]`，text→只能用 block_index
- style_ops 支持矩阵（1.0 交付后用 F4.1 结果填充）

**修复 B**：[context.rs](file:///Users/imatstarbucks/binder/src-tauri/src/ag/context.rs) `assemble_prompt` 在 CurrentDoc 段注入 `editable_kind`（"bdocx" / "text" / "markdown"）。

---

## 第 4 轮：样式与质量修复（依赖 R2-R3）

### F4.1 — Pandoc Lua filter 实现样式往返保真

**问题**：D16 — 1.0 必须实现 color/fontSize/fontFamily/textAlign 等在 html→docx 往返中保留。

**方案**：

1. **创建 Lua filter**：[request_filter.lua](file:///Users/imatstarbucks/binder/src-tauri/resources/request_filter.lua)（或 `style_preserve.lua`）
   - 在 `html → docx` 转换时，遍历所有 `span[style]`、`p[style]` 等元素
   - 将内联 CSS（color/font-size/font-family/text-align）映射到 Pandoc AST 自定义样式或直接写入 docx 对应属性
   - 在 `docx → html` 转换时，反向解析 docx 格式到内联 CSS

2. **修改 `html_to_docx`**：[edit_chain.rs:142](file:///Users/imatstarbucks/binder/src-tauri/src/doc/edit_chain.rs#L142)
   - 增加 `--lua-filter=<path>` 参数指向新 filter
   - 同时保留 `--reference-doc` 提供默认模板

3. **修改 `docx_to_html`**：[edit_chain.rs](file:///Users/imatstarbucks/binder/src-tauri/src/doc/edit_chain.rs) 对应函数
   - 在现有 `preserve-styles.lua` 基础上扩展样式提取范围

**不确定项**：
- Pandoc Lua filter 是否支持将 CSS `color: #FF0000` 映射到 docx 的 `<w:color w:val="FF0000"/>`？需要验证 Pandoc AST 中 CustomStyle 和 Span 的属性传递能力。
- 如果 Lua filter 路径不可行，备选方案为后处理（解析 docx ZIP 包中的 `word/document.xml`，批量注入样式属性）。
- `textAlign` 在 Pandoc 中通常通过 `--reference-doc` 的段落样式控制，CSS `text-align` 映射到 docx 的对齐属性需要单独验证。

**决策**：D16 — 1.0 阻断。

---

### F4.2 — tableCell 光标定位修复

**问题**：P1#7 — tableCell 不在 BINDER_BLOCK_TYPES，`resolveContentBlock` 对 table 结构内的光标回退到 table 顶层块。

**修复**：[BlockIdExtension.ts](file:///Users/imatstarbucks/binder/src/components/ed/extensions/BlockIdExtension.ts)
- `BINDER_BLOCK_TYPES` 增加 `"tableCell"` 和 `"tableHeader"`（两处已定义为 ProseMirror node 类型）

**修复**：[edLocate.ts](file:///Users/imatstarbucks/binder/src/services/edLocate.ts) `resolveContentBlock`
- tableCell 加入 `CONTENT_BLOCK_TYPES` 和 `isContentBlock` 判断
- 验证 `charOffsetInBlock` 在 tableCell 内的计算是否正确（tableCell 的 content 为 block 序列，偏移计算与 paragraph 不同）

**不确定项**：tableCell 的 `charOffsetInBlock` 计算需要验证。tableCell 内容是 block 序列（paragraph 等），`node.textBetween(0, node.content.size)` 可能产生换行符分隔的拼接文本，偏移语义需要明确。

---

### F4.3 — 工作台文件删除监听

**问题**：D5a — BDocx 草稿被外部删除后，编辑器需显示失效态、阻断编辑、不读缓存。

**修复**：[workspace/listing.rs](file:///Users/imatstarbucks/binder/src-tauri/src/workspace/listing.rs) 或新模块
- 增加文件系统监听（`notify` crate 或 `fs::metadata` 轮询）
- 检测已打开草稿文件被删除 → 前端通过 IPC 事件（如 `ws.external_change`）通知
- 前端收到通知 → `edStore.setTabState(path, "conflicted")` → 编辑器显示失效态 → 阻断编辑操作

**不确定项**：
- 是否需要实时监听（inotify/FSEvents）还是轮询（`fs::metadata` 在焦点切换时检查）？轮询成本更低但延迟较高。
- 失效态的具体 UI 呈现不在本方案范围。

---

### F4.4 — BDocx 身份标记

**问题**：D6 — BDocx 必须显式创建，重命名原生 DOCX 不被识别为 BDocx。

**当前行为**：[listing.rs:123](file:///Users/imatstarbucks/binder/src-tauri/src/workspace/listing.rs#L123) 基于文件系统扫描 `*.draft.docx` 或 `binder/bdocx_manifest.json`。

**修复**：[listing.rs](file:///Users/imatstarbucks/binder/src-tauri/src/workspace/listing.rs)
- 将 BDocx 识别从"扫描文件系统"改为"读取 manifest"
- `is_bdocx` 判断从文件名推断改为读取 manifest 中的显式注册记录
- `create_bdocx_draft` 写入 manifest；`delete` 同步清理
- 禁用基于文件名（如 `*.draft.docx`）的自动发现
- 防止用户通过 Shell 重命名普通 DOCX 使其被误识别为 BDocx

**决策**：D6。

---

## 第 5 轮：1.0-post（本轮不实施）

| # | 项目 | 原因 |
|---|------|------|
| F5.1 | tableCell 完整 BINDER_BLOCK_TYPES 集成 + 分页支持 | 涉及表格内光标定位精度和分页整表处理，技术复杂度高 |
| F5.2 | style_ops open_session/background 路径 | 当前仅 current_editor 支持，其他路径需构造编辑器实例或解析 HTML |
| F5.3 | DOCX 转换并发治理（子进程回收、UUID 临时文件、互斥锁） | 需引入进程管理模块，当前方案可运行但不够健壮 |
| F5.4 | LibreOffice PDF 验证（字体/GUI 闭环） | 需解决 LibreOffice 包内构建和无 GUI/字体阻断 |
| F5.5 | 加载骨架超时恢复 | Editor.tsx isEditorContentReady |
| F5.6 | 测量异常静默吞没用户提示 | Pagination.ts measureAfterLayout |

---

## 实施顺序与依赖图

```
R1（无依赖，可并行）
├── F1.1 closeSession 不 delete
├── F1.2 markDirty 不跳过 pending_diff
├── F1.3 统一标签集
└── F1.4 媒体 asset:// 反向映射
      ↓
R2（依赖 R1）
├── F2.1 markDraftSaved 扩展
├── F2.2 open_session 取消 auto-accept
├── F2.3 BDocx background 移除 isDocxPath 阻断
└── F2.4 background→open_session 锚点迁移
      ↓
R3（依赖 R2）
├── F3.1 replace_block 结构化载荷
├── F3.2 replaceHtmlBlock 统一
├── F3.3 GreenReview 多块样式
└── F3.4 提示词注入格式信息
      ↓
R4（依赖 R2-R3）
├── F4.1 Pandoc Lua filter 样式往返
├── F4.2 tableCell 光标定位
├── F4.3 文件删除监听
└── F4.4 BDocx 身份标记
```

---

## 不确定问题清单

| # | 问题 | 影响 | 建议 |
|---|------|------|------|
| Q1 | F1.3 中 `td`/`th` 在 backfill 注入 blockId 是否合适？还是应委托给内部 paragraph？ | `td`/`th` 获得独立 blockId 后，AG 块列表会包含 table 内的 td/th 行 | 当前实现 td/th 已被 blocks.rs 解析为独立块（`is_html_block_tag` 已含 td/th），backfill 注入 blockId 一致。建议同步 |
| Q2 | F1.4 中 `current_dir` 是否已足够让 Pandoc 找到相对图片路径？ | 如果足够，无需额外 `--resource-path`；如果不够，图片嵌入会失败 | 实际测试验证 |
| Q3 | F3.1 中 HTML sanitize 的标签白名单应与 `is_html_block_tag` 对齐还是限制为内联标签？ | sanitize 过宽→XSS 风险；sanitize 过窄→富文本丢失 | 1.0 阶段建议限制为内联标签（strong/em/u/sub/sup/code/a/span/img/br），块级标签不应出现在 innerHTML 中 |
| Q4 | F4.1 中 Pandoc Lua filter 能否将 CSS `color` 映射到 docx `<w:color>`？ | 如果不行，需要备选方案（ZIP 后处理） | 优先验证 Pandoc Lua filter 路径，备选为后处理 `word/document.xml` |
| Q5 | F4.1 中 `textAlign` 需要验证——Pandoc 通过 `--reference-doc` 样式控制对齐，CSS `text-align` 映射到什么？ | 可能需要在 reference.docx 中预设几种对齐样式 | 验证 Pandoc AST 中 Div/Para 的 alignment 属性传递链 |
| Q6 | F4.2 中 tableCell 的 `charOffsetInBlock` 如何计算？ | tableCell 内容是 block 序列，偏移语义与 paragraph 不同 | 需在 tableCell 内按子 block 累加偏移，类似 `getTopBlockSnapshotByIndex` 的 flat 逻辑 |
| Q7 | F4.4 中 BDocx manifest 路径是否需要跨平台兼容？ | `.binder/bdocx_manifest.json` 在 Windows/macOS/Linux 均可用 | 当前已有 `.draft.docx` 后缀约定，需确认 manifest 方案替换后是否兼容 |
| Q8 | 前端 `replaceHtmlBlock` 是否需要改用 HTML parser 代替正则？ | 正则解析在嵌套同名标签（如 `<div><div>`）时会匹配错误闭合 | 当前场景（Pandoc 产出）嵌套 div 不常见，1.0 暂不切换；F3.1 结构化载荷落地后此路径可能被淘汰 |
