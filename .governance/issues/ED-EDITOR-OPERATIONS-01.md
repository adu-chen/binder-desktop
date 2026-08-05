# Issue Trace：ED-EDITOR-OPERATIONS-01

## 目标

在既有两行编辑器工具栏中补齐编辑器级操作：保存、撤销重做、当前文档查找替换、定位、BDocx 页码导航、缩放、Markdown 源码/富文本切换和按格式导出。

## 规则与链路

- 模块：ED、PRV（复用既有导出接口）
- 链路：ED-EDIT、PRV-VIEW
- 复用规则：BR-ED-STATE-001、BR-ED-DATA-003、BR-ED-DATA-004、BR-PRV-DATA-002
- 不新增规则、链路、约束或术语。

## 设计一致性

- 查找、替换和定位只操作当前编辑会话的 LogicalState；替换使用既有 ProseMirror 编辑事务，继承脏态和保存链。
- 纯文本与 Markdown 源码按真实换行定位；富文本没有稳定的视觉行概念，因此按内容块/段落定位并在界面明示。
- BDocx 页码来自已有纯呈现分页缝，只负责滚动定位，不改变文档模型、纸张偏好或落盘内容。
- 编辑态不打印。打印留在既有 PRV 只读预览面，避免把应用壳或未收敛的编辑态内容送入系统打印。
- 导出复用既有 `prv.export`：文本/Markdown 仅导出 DOCX；DOCX/BDocx 仅导出 PDF 或 Markdown；不支持的格式不展示入口。

## 文件范围

- `docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md`
- `src/components/ed/Editor.tsx`
- `src/components/ed/EditorToolbar.tsx`
- `src/components/ed/EditorControls.test.tsx`
- `src/styles.css`

## 验收

- 工具栏保持两行，编辑器操作位于第一行。
- 支持格式可查找替换并定位；BDocx 可选择页面和上下翻页。
- 导出只显示实际支持的目标格式，并有成功/失败反馈。
- 预览打印入口保持不变，编辑态无打印入口。
