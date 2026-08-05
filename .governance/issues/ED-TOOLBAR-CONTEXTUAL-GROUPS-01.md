# Issue Trace：ED-TOOLBAR-CONTEXTUAL-GROUPS-01

## 目标

在既有 TipTap 编辑器中实现方案 A：两行常驻工具栏按编辑格式动态适配；选中图片和表格时展示就近上下文工具组。

## 规则与链路

- 模块：ED
- 链路：ED-EDIT
- 复用规则：BR-ED-STATE-001、BR-ED-DATA-003、BR-ED-DATA-005、BR-ED-DATA-006、BR-ED-DATA-007、BR-ED-VERIFY-001
- 不新增规则、链路、约束或术语。

## 设计一致性

- `A-ED-M-T-01` 将工具栏视为用户经 TipTap 受控命令发起的编辑入口；本次不绕过该入口。
- 表格移动与删除仍通过现有表格邻近临时控件执行；本次只扩展其同一上下文工具组的展示与操作归类。
- 图片仅使用既有的宽度、文档流移动、删除及受控图片命令；不引入浮动定位、裁剪或旋转。
- Markdown 和纯文本只显示可被其格式承载的控件。
- 人类确认继续实现保存、字体族和清除文字格式入口；查找替换、页码导航及预览渲染仍不进入本次范围。

## 文件范围

- `src/components/ed/EditorToolbar.tsx`
- `src/components/ed/ImageMoveControls.tsx`
- `src/components/ed/TableMoveControls.tsx`
- `src/components/ed/EditorControls.test.tsx`
- `src/styles.css`

## 验收

- 工具栏稳定为两行；对象工具不产生第三条常驻栏。
- 图片、表格工具组仅在对象被选中时出现，且 `mousedown` 不丢失编辑器选区。
- Markdown/纯文本/BDocx 的可见操作符合其能力边界。
- 影响测试、治理审计和构建通过。
