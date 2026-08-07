# ED-SELECTION-DRAG-01 修复方案：选区拖动错位（锁定原选区 + 场景分流）

**状态**：PROPOSAL（本轮不实施）
**提出**：人类 2026-08-07 观察（1-100 文本，选中 20-40，从中间 30 按下拖动）
**关联**：ED-SELECTION-PERSIST-01（v2 统一链路）、PI-ED-10（拖出引用）

---

## 1. 问题定义

在**已有非空选区**上从**选区中间位置**按下并拖动时：

1. **编辑器内 drop**：`ProseMirror` 原生 move 语义的 `tr.deleteSelection()` 删除的是**拖动中被浏览器原生拖选篡改后的当前选区**（30-35），而非原选区（20-40）；随后插入原选区内容副本（20-40）。结果：30-35 被错删、20-40 被复制一份、"原选区变成新选区"、内容被改。
2. **拖出编辑器（agent 输入框）**：编辑器内无 drop → 内容不被删；但 `selection` 已被篡改成 30-35，且 SelectionPersist v2 的 `lastNonEmpty` 跟随篡改后的 selection → **原选区 20-40 的高亮消失、持久化高亮变为 30-35**；若 `getSelectionRef()` 在篡改后执行，生成的引用锚/文本也是 30-35（错误）。

### 根因链

```
mousedown 在选区中间（30）
  → PM allowDefault=true（移动>4px 放行浏览器默认，prosemirror-view L3458-3460）
  → WebKit 原生拖选从按下点重新拖出新选区（30-35），selection 被篡改
  → 触发 selectionSet 事务 → SelectionPersist apply 更新 lastNonEmpty=30-35
  → 编辑器内 drop：PM 原生 handleDrop 的 deleteSelection() 删当前 selection（30-35）
  → 拖出：blur persist 用 lastNonEmpty=30-35 渲染
```

### 应然语义（人类确认）

| 场景 | 语义 | 原选区 |
|---|---|---|
| 编辑器文档内 drop | 移动：删原选区 + 插到停留位置 | 内容被移走（合理） |
| 拖出编辑器（agent 输入框） | 引用：把选区内容作为位置引用 | **保持不变**（只读拷贝） |

---

## 2. 方案设计：拖动锁定 + 自定义 drop 分流

### 2.1 核心思路

在 `dragstart` 时**锁定原选区范围**，拖动期间：
- 屏蔽原生拖选对 selection 的篡改（SelectionPersist 不跟随）；
- 编辑器内 drop 走**自定义 handleDrop**（删除锁定范围而非当前 selection）；
- 拖出编辑器 → 无 drop，内容不变，`lastNonEmpty` 保持锁定范围（高亮正确）。

### 2.2 改动文件与内容

#### 改动 1：`src/components/ed/extensions/SelectionPersistExtension.ts` — 增加拖动锁状态

- 插件 state 扩展：
  ```ts
  interface PersistState {
    lastNonEmpty: { from: number; to: number } | null;
    /** dragstart 锁定的原选区范围（拖动期间生效） */
    dragLock: { from: number; to: number } | null;
  }
  ```
- `applySelectionPersist` 增加：
  - 收到 `draglock` 信号 → `dragLock = 当前非空 TextSelection 范围`（持久化到 state，供 decorations 与 lastNonEmpty 判定）；
  - `dragLock` 存在期间，`selectionSet` 的 lastNonEmpty **不更新**（跳过 L71-77 的覆盖逻辑）；
  - 收到 `dragunlock` 信号 → `dragLock = null`，`lastNonEmpty` 恢复为锁定范围（保证拖出后持久化高亮正确）。
- `decorations`：
  - `dragLock` 存在时 → **优先用 dragLock 渲染 span**（高亮稳定在原选区，不跟随拖选篡改）；
  - 否则维持现有逻辑。

#### 改动 2：`src/components/ed/Editor.tsx` — 事件桥接 + 自定义 handleDrop

- `editorProps` 新增 `handleDrop`（PM 原生 drop 之前调用，返回 true 表示接管）：
  ```ts
  handleDrop: (view, event, slice, move) => {
    const locked = dragLockRef.current;            // 引用编辑模块级锁
    if (!move || !locked) return false;            // 非移动（copy/外部文件）交回 PM
    event.preventDefault();
    // 用锁定范围替代 deleteSelection()
    const $mouse = view.state.doc.resolve(view.posAtCoords(eventCoords(event))?.pos ?? 0);
    const insertPos = dropPoint(view.state.doc, $mouse.pos, slice) ?? $mouse.pos;
    let tr = view.state.tr;
    tr.delete(locked.from, locked.to);             // 删原选区（而非当前被篡改的 selection）
    const mapped = tr.mapping.map(insertPos);
    tr.replaceRange(mapped, mapped, slice);
    tr.setSelection(TextSelection.between(tr.doc.resolve(mapped), tr.doc.resolve(mapped + slice.size)));
    view.dispatch(tr.setMeta("uiEvent", "drop"));
    clearDragLock();                               // drop 后解锁
    return true;
  };
  ```
- `handleDOMEvents` 增加 `dragstart` / `dragend`：
  ```ts
  dragstart: (view, event) => {
    if (selectionContainsImage(...)) return false;
    const sel = view.state.selection;
    if (sel instanceof TextSelection && !sel.empty) {
      setDragLock({ from: sel.from, to: sel.to });   // 锁定原选区
    }
    const ref = getSelectionRef();                    // 用锁定/当前选区生成引用
    ... // 原 writeSelectionRefData 逻辑保留
    return false;                                     // 编辑器内拖动仍交回 PM（但 drop 已接管）
  },
  dragend: (view) => { clearDragLock(); return false; },
  ```
- 模块级锁（`Editor.tsx` 文件作用域）：
  ```ts
  let dragLockRef: { from: number; to: number } | null = null;
  function setDragLock(r) { dragLockRef = r; }
  function clearDragLock() { dragLockRef = null; }
  ```
- **拖出引用正确性**（修复 `getSelectionRef` 篡改后取错值）：
  - `dragstart` 中 `getSelectionRef()` 在**锁定时立即执行**（事件顺序：handleDOMEvents.dragstart 先于 PM 原生 dragstart，此时 selection 仍是原选区）→ 引用锚/文本为 20-40 ✓。

#### 改动 3：`src/components/ag/Composer.tsx`（可选，本次不动）

- 现有 `payloadFromTransfer` 读 `SELECTION_REF_MIME`（被 PM clearData 清掉时会退化为 text/plain）。本方案不改——按人类指示"后续逻辑不在本次课题"。

### 2.3 时序（修复后）

```
mousedown 在 30（选区中间）→ PM allowDefault=true → 原生拖选篡改 selection=30-35
dragstart（编辑器）→ setDragLock(20-40) + 写 SelectionRef(20-40)
  拖动期间：SelectionPersist dragLock 生效 → 高亮 span 稳定 20-40，lastNonEmpty 不跟随篡改
  ├─ 编辑器内 drop → 自定义 handleDrop → 删 20-40 + 插入 drop 位置 + 新选区 → 解锁
  └─ 拖出（无 drop）→ blur persist → lastNonEmpty=20-40（未被篡改）→ 高亮 20-40 保留 ✓
dragend → 解锁
```

---

## 3. 边界与风险

| 项 | 处理 |
|---|---|
| copy 拖动（macOS 按住 alt / Windows ctrl） | `handleDrop` 收到 `move=false` → return false 交回 PM 原生复制（不删原选区，天然正确） |
| 选区含图片（NodeSelection） | dragstart 保持 `selectionContainsImage` 早退，不锁定、不走自定义 drop |
| 拖出编辑器后 drop 在编辑器外 | 编辑器无 drop 事件，`dragLock` 在 dragend 清除；`lastNonEmpty` 已在锁定期间保持不变 ✓ |
| 快速点击（非拖动） | 不触发 dragstart → 无锁 → 行为不变 |
| 拖动过程中 selection 被篡改仍会触发 selectionSet 事务 | apply 中 `dragLock` 存在时跳过 lastNonEmpty 更新；decorations 用 dragLock 渲染，视觉稳定 |
| handleDrop 与现有图片 drop（`handlePaperImageDrop`） | 图片走 image MIME 早退路径，互不干扰 |
| SelectionPersist 测试 | 需新增 draglock/dragunlock 用例（apply 不跟随 + decorations 用锁 + 解锁恢复） |

## 4. 测试计划

- `SelectionPersistExtension.integration.test.ts`：
  1. draglock 后 selectionSet（模拟原生拖选篡改）→ lastNonEmpty 不更新、decorations 用锁范围；
  2. dragunlock → lastNonEmpty 恢复锁定范围；
  3. 自定义 handleDrop（move）→ 删除锁定范围、插入 drop 位置、新选区正确；
  4. 自定义 handleDrop（copy，move=false）→ 不删原选区。
- `EditorControls.test.tsx` 保持 12/12 不回归。

## 5. 验证命令

- `npx vitest run src/components/ed/extensions/SelectionPersistExtension.integration.test.ts src/components/ed/extensions/SelectionPersistExtension.dom.test.ts`
- `npm run test:related`
- `npx tsc --noEmit` / `npm run build`
- `npm run governance:generate` / `npm run governance:audit`
- 手工：1-100 文本选中 20-40，从 30 按下拖动（编辑器内移动正确；拖到 agent 输入框原选区保持、引用为 20-40）

## 6. 治理

- issue_trace：`ED-SELECTION-DRAG-01.md`
- 文档：A-ED-M-T-01（v1.61 注记）、ADUS 重新生成
- scope：替换当前活动锁（WS-NEW-BUTTON-01 闭合后进入）
