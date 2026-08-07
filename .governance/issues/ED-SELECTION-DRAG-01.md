# Issue Trace：ED-SELECTION-DRAG-01

## 目标

修复选区拖动错位：从**选区中间**按下拖动时，WebKit 原生拖选会篡改 selection（从按下点重新拖出），导致 PM 原生 drop 的 `deleteSelection()` 删除被篡改的选区而非原选区（编辑器内：30-35 被错删、原 20-40 被复制；拖出：原选区高亮丢失、引用锚错位）。

## 决策

- 人类 2026-08-07：拖动选区预期 = 移动选中内容；拖出编辑器（agent 输入框）原选区保持不变（引用语义）；agent 引用消费逻辑不在本课题。
- 方案：dragstart 锁定原选区（插件 dragLock）→ 拖动期间 lastNonEmpty/decorations 以锁定范围为准；自定义 handleDrop 删除锁定范围；dragend 解锁。

## 实现

- `SelectionPersistExtension.ts`：
  - state 增加 `dragLock`；`draglock`/`dragunlock` 信号；锁定期间 selectionSet 不更新 lastNonEmpty；decorations 锁期间用锁定范围渲染。
  - 新增 `lockDragSelection`/`unlockDragSelection` 桥接 + `handleSelectionDragDrop`（move 型 drop 删除锁定范围、插入、解锁）。
- `Editor.tsx`：
  - dragstart 锁定 + 写引用；dragend 解锁；`editorProps.handleDrop` 挂载 `handleSelectionDragDrop`。
- 测试：integration 12 用例（新增 draglock/dragunlock/clear 清锁/handleDrop move/copy 5 用例）。

## 验证

- integration 12/12、test:related 21 文件 / 457 测试、tsc、build、governance audit 通过。

## 遗留

- SelectionRef 在编辑器内拖动时被 PM `clearData()` 清除 → 拖入 AG 输入面的位置引用退化 text/plain（人类指示不在本课题）。
- 手工验证（1-100 数字选区拖动移动/拖出）待人类执行。
