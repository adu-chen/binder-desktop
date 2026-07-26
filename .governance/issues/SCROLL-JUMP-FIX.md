# BDocx 分页末页滚动跳动 — 修复记录

> **历史诊断记录（非当前权威实现）**：本文记录早期七轮试验。第 7 轮“文档坐标空间统一”随后已被
> 正式应用证据否定，当前滚动控制权、输入冻结、首帧揭示与光标激活周期以
> `SCROLL-JUMP-PAGINATION-01` §23–24 为准；块内拆分及其接受边界以
> `PAGINATION-INTRABLOCK-SPLIT-01` §16–17 为准。不得把本文“最终修复”字样解释为当前实现契约。

**日期**：2026-07-25
**问题**：分页模式下，末页滚动到底部后激活光标/输入内容时，页面自动跳至"内容底部对齐窗口底部"，而非保持"页面底部对齐窗口底部"。非末页区域也同样存在每 keystroke 跳动。

---

## 根因链

```
末端：ProseMirror 内置 scrollIntoView 每 keystroke 强制将光标对齐视口底边
  ← 中层：Decoration.widget end-fill 每 keystroke 随 doc.content.size 重映射 → DOM 变更 → scrollHeight 抖动
  ← 中层：CSS padding-bottom 被更高优先级规则覆盖 → 修复不生效
  ← 底层：浏览器默认 overflow-anchor:auto Scroll Anchoring 每 DOM 变更调整 scrollTop
  ← 底层：coordsAtPos 与 getBoundingClientRect 因 CSS zoom 处于不同坐标空间 → 条件判断始终失败
```

---

## 每轮修复详情

### 第 1 轮（错误方向）

**现象**：末页底部激活光标后内容跳至窗口底部。

**尝试修复**：[Editor.tsx - `scrollThreshold`/`scrollMargin`](file:///Users/imatstarbucks/binder/src/components/ed/Editor.tsx)

```ts
// ❌ 错误：scrollThreshold 越大 ProseMirror 越激进地滚动，而非减小滚动
scrollThreshold: 100,
scrollMargin: 16,
```

**结果**：无效。`scrollThreshold` 语义是"光标距离窗口边缘多近时触发滚动"，值越大越激进，方向相反。

**回退**：已移除该配置。

---

### 第 2 轮（end-fill widget → CSS padding，方向正确但遗漏 CSS 优先级）

**现象**：用户反馈"每一次输入都会跳一下，好像是跳到之前窗口固定的地方，但是就跳一下，又回到内容底部对齐窗口底部了"。

**根因分析**：end-fill 是 `Decoration.widget`，位置在 `doc.content.size`。每次 keystroke 导致 `doc.content.size` 变化 → `prev.decorations.map(tr.mapping, tr.doc)` 重映射 widget 位置 → DOM 变更 → 浏览器调整 `.editor-paper` 的 scrollTop。

**修复**：[Pagination.ts](file:///Users/imatstarbucks/binder/src/components/ed/extensions/Pagination.ts)

```ts
// ✅ 移除 renderEndFill 函数和 widget 创建
// 新增 applyEndFill 设置 CSS 变量
function applyEndFill(dom: HTMLElement, lastFill: number) {
  dom.style.setProperty("--rm-page-end-fill", `${Math.max(0, lastFill)}px`);
}
```

```ts
// ✅ buildDecorations 不再包含 end-fill widget
function buildDecorations(doc: PMNode, plan: PageBreakPlan): DecorationSet {
  if (plan.breaks.length === 0) return DecorationSet.empty;
  // ... 只建 page-gap widgets
  return DecorationSet.create(doc, decos);
}
```

```ts
// ✅ view.update 每帧同步 CSS 变量
update: (view: EditorView) => {
  applyEndFill(view.dom as HTMLElement, st?.enabled ? (st.plan.lastFill || 0) : 0);
  // ...
}
```

[styles.css](file:///Users/imatstarbucks/binder/src/styles.css)
```css
/* ⚠️ 选择器优先级不够，被下文覆盖 */
.ProseMirror.rm-paginated {
  padding-bottom: var(--rm-page-end-fill, 0px);
}
```

**结果**：表面无变化。CSS 变量设置正确，但 `padding-bottom` 规则被更高优先级的 `.editor-paper .ProseMirror.rm-paginated { padding: ... }` 覆盖。

**关键日志**（此时尚未加日志，现象来自手工测试）：
- 每 keystroke 内容底部跳至窗口底部
- "跳到之前固定的地方" → 浏览器 Scroll Anchoring 的行为特征

---

### 第 3 轮（CSS 优先级修复）

**现象**：与第 2 轮相同。

**根因分析**：[styles.css:672](file:///Users/imatstarbucks/binder/src/styles.css#L672) 的 `.editor-paper .ProseMirror.rm-paginated`（优先级 0,2,1）中的 `padding` 简写覆盖了 [styles.css:690](file:///Users/imatstarbucks/binder/src/styles.css#L690) 的 `.ProseMirror.rm-paginated`（优先级 0,2,0）中的 `padding-bottom`。

**修复**：将 `padding-bottom` 提升到同优先级规则块内。

```css
/* ✅ 移入 .editor-paper .ProseMirror.rm-paginated 规则块（同优先级） */
.editor-paper .ProseMirror.rm-paginated {
  width: var(--rm-page-width);
  max-width: var(--rm-page-width);
  flex: none;
  padding: var(--rm-page-margin-y) var(--rm-page-margin-x);
  /* 审计修复（2026-07-25）：末页补白，JS 设置 --rm-page-end-fill 变量。 */
  padding-bottom: var(--rm-page-end-fill, 0px);
  /* ... */
}
```

**结果**：仍无效。CSS 已正确应用，但浏览器默认 `overflow-anchor: auto` 每 keystroke 仍在调整 scrollTop，覆盖了 CSS 效果。

---

### 第 4 轮（overflow-anchor 修复）

**现象**：用户报告"只要窗口停留不是在内容最上方，输入内容时就是会伴随输入跳"。

**根因分析**：`.editor-paper { overflow: auto }` 未设置 `overflow-anchor`，浏览器默认启用 `overflow-anchor: auto`。ProseMirror 每 keystroke 更新 DOM → 浏览器检测滚动容器内 DOM 变化 → 立即调整 `scrollTop` → 内容跳动。当 `scrollTop=0` 时无调整空间，所以顶部不跳。

**修复**：[styles.css:591](file:///Users/imatstarbucks/binder/src/styles.css#L591)

```css
/* ✅ 禁用浏览器 Scroll Anchoring */
.editor-paper { flex: 1; overflow: auto; overflow-anchor: none; display: flex; justify-content: center; padding: var(--sp-8) var(--sp-6); }
```

**结果**：仍无效。`overflow-anchor` 修复是正确的，但被最后一个因素覆盖——ProseMirror 内置 `scrollIntoView` 每 keystroke 仍在执行。

---

### 第 5 轮（诊断日志 + 定位到 ProseMirror scrollIntoView）

**添加的诊断日志**：

[Editor.tsx - handleScrollToSelection](file:///Users/imatstarbucks/binder/src/components/ed/Editor.tsx#L479)
```ts
handleScrollToSelection(view) {
  const st = (view.dom as HTMLElement).closest(".editor-paper") as HTMLElement | null;
  const before = st?.scrollTop ?? -1;
  console.log("[PM.scrollIntoView] triggered, scrollTop before=", before,
    "cursor=", view.state.selection.from, "docSize=", view.state.doc.content.size);
  return false; // 不阻止默认行为，仅记录
},
```

[Editor.tsx - paper scroll 监听]
```ts
const onScrollDebug = () => {
  const st = paper?.scrollTop ?? -1;
  console.log("[paper.scroll] scrollTop=", st, "scrollHeight=", paper?.scrollHeight, "clientHeight=", paper?.clientHeight);
};
```

[Pagination.ts - 各处关键路径日志]

**关键日志证据**：
```
[PM.scrollIntoView] triggered, scrollTop before=614, cursor=931, docSize=932
[paper.scroll] scrollTop=771, scrollHeight=1190, clientHeight=223
[paper.scroll] scrollTop=614, scrollHeight=1190, clientHeight=223

[PM.scrollIntoView] triggered, scrollTop before=614, cursor=932, docSize=933
[paper.scroll] scrollTop=771, scrollHeight=1190, clientHeight=223
[paper.scroll] scrollTop=614, scrollHeight=1190, clientHeight=223
```

**日志揭示**：
- `scrollHeight=1190` 恒定 — 第 2/3/4 轮修复已生效，总高度不再抖动
- `scrollTop=614→771→614` 每 keystroke 振荡 — ProseMirror `scrollIntoView` 是唯一剩余跳动源
- `614` 是 ProseMirror 根据 cursor 位置计算出的目标 scrollTop

**结论**：前四轮修复已将"多因复合跳动"收敛为"ProseMirror scrollIntoView 单一原因"。

---

### 第 6 轮（条件接管 scrollIntoView — 坐标空间错误）

**修复**：[Editor.tsx - handleScrollToSelection](file:///Users/imatstarbucks/binder/src/components/ed/Editor.tsx#L479)

```ts
// ⚠️ 坐标空间不一致：coordsAtPos (受 CSS zoom 影响) vs getBoundingClientRect
handleScrollToSelection(view) {
  const editorPaper = (view.dom as HTMLElement).closest(".editor-paper") as HTMLElement | null;
  if (!editorPaper) return false;
  const cursorRect = view.coordsAtPos(view.state.selection.from);
  const paperRect = editorPaper.getBoundingClientRect();
  const margin = 8;
  const visible = cursorRect.top >= paperRect.top + margin
    && cursorRect.bottom <= paperRect.bottom - margin;
  return visible;
},
```

**关键日志**：
```
[scrollIntoView] allow scrollTop=476 cursorY=788 paperH=623 docSize=1022
[scrollIntoView] allow scrollTop=476 cursorY=788 paperH=623 docSize=1023
```

**分析**：`cursorY=788` > `paperH=623` → `visible` 始终 `false` → 每次走 `allow` → 修复未生效。

**根因**：`coordsAtPos` 与 `getBoundingClientRect` 在 CSS zoom 下处于不同坐标空间。Pagination 扩展中已有 `rectScale()` 专门处理此问题，但 `handleScrollToSelection` 未使用。

---

### 第 7 轮（最终修复 — 文档坐标空间统一）

**修复**：[Editor.tsx - handleScrollToSelection](file:///Users/imatstarbucks/binder/src/components/ed/Editor.tsx#L479)

```ts
// ✅ 转为文档坐标（scrollTop + viewport偏移），避免 CSS zoom 导致坐标空间不一致
handleScrollToSelection(view) {
  const editorPaper = (view.dom as HTMLElement).closest(".editor-paper") as HTMLElement | null;
  if (!editorPaper) return false;
  const scrollTop = editorPaper.scrollTop;
  const clientH = editorPaper.clientHeight;
  const cursorViewportY = view.coordsAtPos(view.state.selection.head).top;
  const paperTop = editorPaper.getBoundingClientRect().top;
  const cursorDocY = scrollTop + (cursorViewportY - paperTop);
  const margin = 8;
  const visible = cursorDocY >= scrollTop + margin
    && cursorDocY <= scrollTop + clientH - margin;
  return visible;
},
```

**逻辑**：将光标坐标转换为文档坐标系（`cursorDocY`），与 `scrollTop` 在同一空间比较。`cursorDocY` 在 `[scrollTop+8, scrollTop+clientH-8]` 内 → 光标已在视口 → 返回 `true` 阻止 ProseMirror 滚动。光标出界 → 返回 `false` 放行默认 `scrollIntoView`。

---

## 最终生效的 4 层修复

| 层 | 文件 | 行 | 修复 |
|----|------|----|------|
| 1 | [styles.css](file:///Users/imatstarbucks/binder/src/styles.css#L591) | 591 | `overflow-anchor: none` — 禁用浏览器 Scroll Anchoring |
| 2 | [styles.css](file:///Users/imatstarbucks/binder/src/styles.css#L676-L678) | 676-678 | `padding-bottom: var(--rm-page-end-fill)` 写入高优先级规则块 |
| 3 | [Pagination.ts](file:///Users/imatstarbucks/binder/src/components/ed/extensions/Pagination.ts) | 165 | end-fill 从 `Decoration.widget` 改为 CSS `--rm-page-end-fill` 变量 |
| 4 | [Editor.tsx](file:///Users/imatstarbucks/binder/src/components/ed/Editor.tsx#L479) | 479-496 | `handleScrollToSelection` 条件接管（文档坐标统一） |
