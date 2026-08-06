# Issue Trace：ED-HIGHLIGHT-BACKGROUND-01

## 目标

编辑器工具栏"文字"组的高亮工具收敛为与"文字颜色"同款 UI：名称改为"文字背景"，用 `ed-tb-color-field` 标签 + `<input type="color">` 取色器直接 `toggleHighlight({color})`；删除"清除高亮"按钮（取色器自带去除选项）；删除 5 预设色板组 `ed-tb-highlight-control` 与 `HIGHLIGHT_COLORS` 常量及相关样式；aria 语义同步调整。功能语义不变（highlight mark），仅 UI 形态对齐文字颜色。

## 规则与链路

- 模块：ED（工具栏）
- 链路：ED-EDIT（文字样式应用）
- 复用规则：不新增规则/链路/约束/术语；BDocx 对高亮的兼容性口径不变（会话内可见不等于保存保留）。

## 设计一致性

- UI 形态对齐文字颜色（setColor）：`ed-tb-color-field` 标签 + 取色器，aria-label="文字背景"。
- 交互对齐文字颜色（取色器无显式清除按钮）：移除"清除高亮"独立入口。
- 删除 `HIGHLIGHT_COLORS` 常量与 `ed-tb-highlight-*` 样式（无其他引用后）。
- 文档 A-ED-X-R-03 §4.1 高亮行同步"文字背景（取色器）"表述 + 变更记录。

## 文件范围

- `src/components/ed/EditorToolbar.tsx`（高亮 UI → 文字背景取色器；删除常量）
- `src/components/ed/EditorControls.test.tsx`（高亮断言同步更新）
- `src/styles.css`（删除 ed-tb-highlight-* 样式，保留 ed-tb-color-field 复用）
- `docs/01_modules/ED/A-ED-X-R-03_编辑器工具栏功能与兼容性清单.md`（高亮行 + 变更记录）
- `.governance/issues/ED-HIGHLIGHT-BACKGROUND-01.md`（本文）
- `docs/00_core/A-CORE-C-R-01_ADUS.md`（重新生成）

## 验收

- 文字组显示"文字背景"取色器，样式与"文字颜色"一致；无"高亮"按钮、"高亮底色"色板组、"清除高亮"入口。
- 选择颜色 → toggleHighlight({color})；aria-label 语义正确。
- 相关测试、tsc、governance:generate/audit、build 通过。

## Task Runtime Report — 高亮收敛为文字背景

task_type: CODE_CHANGE
governance_phase: IMPLEMENTATION
final_state: READY
issue_trace: ED-HIGHLIGHT-BACKGROUND-01

changed_files:

- `src/components/ed/EditorToolbar.tsx`：高亮三件套 → "文字背景"取色器（ed-tb-color-field 同款）；删除 HIGHLIGHT_COLORS 常量与 ed-tb-highlight-control 组。
- `src/components/ed/EditorControls.test.tsx`：高亮断言更新（"文字背景"取色器、toggleHighlight 调用、无清除高亮/色板组）。
- `src/styles.css`：删除 ed-tb-highlight-* 样式。
- `docs/01_modules/ED/A-ED-X-R-03_编辑器工具栏功能与兼容性清单.md`：高亮行更新 + 变更记录。
- `docs/00_core/A-CORE-C-R-01_ADUS.md`：重新生成。

test_impact:

- direct: ED
- affected: 无（PRV 不引用高亮控件）

validation:

- `npx vitest run src/components/ed/EditorControls.test.tsx`：passed（12/12）。
- `npm run test:related`：passed（23 文件 / 499 测试）。
- `npm run governance:generate`：passed（@GOV 块 225 / 测试资产 88 / 用例 775）。
- `npm run governance:audit`：passed（governance_findings 清零）。
- `npx tsc --noEmit`、`npm run build`：passed。

unresolved_items:

- macOS 系统取色器"去除颜色"行为由 WKWebView 原生提供，与文字颜色一致，无额外风险。
