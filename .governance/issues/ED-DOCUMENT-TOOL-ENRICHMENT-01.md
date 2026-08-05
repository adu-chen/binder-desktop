# Issue Trace：ED-DOCUMENT-TOOL-ENRICHMENT-01

## 目标

补齐第二行文档工具与选中对象就近工具：高亮、上下标、列表层级、富文本分隔线、独立图片段落对齐和表格单元格合并/拆分。

## 规则与链路

- 模块：ED
- 链路：ED-EDIT
- 复用规则：BR-ED-STATE-001、BR-ED-DATA-002、BR-ED-DATA-003、BR-ED-DATA-007
- 复用术语：TERM-ED-007、TERM-ED-008
- 不新增规则、链路、约束或术语。

## 设计一致性

- 删除线、高亮、上下标已在 TipTap 扩展集和 DOC 样式子集登记；Markdown 继续隐藏高亮和上下标等不可无损往返的样式。
- 高亮使用预设色板和自定义不透明色；清除入口仅删除高亮 mark，不改变其他文字格式。
- 文字颜色归入文字分组，不再为颜色建立独立的第二行菜单。
- 工具栏菜单使用原生 `details` 保留可访问的开关语义；在文档级 pointer/focus 激活落到菜单外部时关闭，
  不截获菜单内部的输入、色板或格式操作。
- 工具栏功能、格式适配和 BDocx 往返风险已独立登记为 `ED-X-R-03`；发现删除线、高亮、上下标及合并/拆分
  虽有 UI 入口，但当前 capability manifest 标记为 `unsupported`，已作为需产品决策的高风险项记录，未在本轮伪称兼容。
- 列表层级只使用当前 `listItem` 的受控 TipTap 命令；普通段落不伪装成可缩进。
- 分隔线复用已有水平线节点命令，作为文档内容而非分页或页面设置。
- 图片只改变独立图片所在段落的 `textAlign`，沿已验证的 left/center/right DOCX 往返路径。
- 表格合并/拆分仅走现有表格命令和当前单元格选择；动态 capability 决定是否可用。
- 本地替换图片需经 DOC 受控导入与媒体基线校验，当前没有可安全复用的 UI 接口，因此不添加按钮。

## 文件范围

- `docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md`
- `docs/01_modules/ED/A-ED-X-R-03_编辑器工具栏功能与兼容性清单.md`
- `src/components/ed/EditorToolbar.tsx`
- `src/components/ed/ImageMoveControls.tsx`
- `src/components/ed/TableMoveControls.tsx`
- `src/components/ed/EditorControls.test.tsx`
- `src/styles.css`

## 验收

- 非 Markdown 富文本文字、列表和插入分组提供受支持能力。
- 图片和表格仍仅在选中对象时出现就近控件，且不新增第三条常驻工具栏。
- 不支持能力不呈现为可点击入口。
- 已打开的工具栏弹出组在鼠标或键盘焦点进入其他区域时自动关闭，组内操作不受影响。

## 验证记录

- 通过：ADUS 生成检查；影响计划；`EditorControls.test.tsx` 11/11（含高亮、删除线及菜单外部交互关闭）；
  关联前端回归 17 文件 / 415 测试；TypeScript 检查；治理审计；生产构建；差异格式检查。
- 已接受风险：完整 Rust 回归的 LibreOffice PDF 端到端测试发生 worker `signal=6`（262/263）；人类已确认接受该
  本轮前端工具栏范围外的既有外部运行时失败。

## Task Runtime Report — 文档工具增强与弹出组交互收束

task_type: CODE_CHANGE  
governance_phase: VALIDATION  
final_state: READY  
issue_trace: ED-DOCUMENT-TOOL-ENRICHMENT-01

changed_files:

- `src/components/ed/EditorToolbar.tsx`：两行工具栏分组、文字与高亮入口、菜单外部交互关闭。
- `src/components/ed/ImageMoveControls.tsx`、`src/components/ed/TableMoveControls.tsx`：选中对象就近工具。
- `src/components/ed/EditorControls.test.tsx`：工具栏及对象工具回归，包含菜单自动关闭。
- `src/styles.css`：两行工具栏与弹出组样式。
- `docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md`、`docs/01_modules/ED/A-ED-X-R-03_编辑器工具栏功能与兼容性清单.md`：实现和兼容性记录。

test_impact:

- direct: ED
- affected: AG、DE、ED、SYS

validation:

- `npm run governance:generate`：passed（ADUS 内容无变化）。
- `npm run test:plan -- --files …`：passed。
- `npm run test:related -- --files …`：前端 17 文件 / 415 测试通过；TypeScript 通过。
- `npm run governance:audit`：passed，0 governance findings。
- `npm run build`、`git diff --check`：passed。

unresolved_items:

- BDocx 删除线、高亮、上下标与表格合并/拆分仍有 UI 入口，但 capability manifest 标为 `unsupported`；风险和后续产品决策已记录于 ED-X-R-03，本 Scope 不将其伪称为往返兼容。
- LibreOffice DOCX→PDF 端到端环境测试仍为已接受风险（262/263，worker `signal=6`），与本轮前端改动无关。
