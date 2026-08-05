# Issue Trace：PRV-TOOLBAR-ADAPTIVE-01

## 目标

为预览模式（PrvPanel 只读预览）提供与编辑模式同风格的工具栏：抽取 EditorToolbar 的 TbBtn/Sep/ToolbarMenu 为共享组件，新建 PrvToolbar 按预览内容类型（pdf/table/image/video/audio）自适应工具集；不提供"关闭"工具（TabStrip 标签 × 承担关闭）。只承载查看/输出类工具，不承载编辑命令（BR-PRV-DATA-001 不弱化）。

## 规则与链路

- 模块：PRV（复用 ED 工具栏组件体系）
- 链路：PRV-VIEW
- 复用规则：BR-PRV-DATA-001、BR-PRV-DATA-002、X-FORBID-004、X-FORBID-007、BR-DOC-DATA-001（双链分离）
- 不新增规则、链路、约束或术语。

## 设计一致性

- 预览渲染/导出/打印沿用既有 prv.preview / prv.export / iframe.print，不新建后端能力。
- 图片/表格缩放为 CSS zoom 视图态（复用 zoom.ts 常量），不改数据、不产脏态。
- iframe 内 PDF 缩放由 WKWebView 原生查看器控制，工具栏不提供（不承诺伪能力）。
- csv/tsv 导出取决于 prv.export 后端能力，能力矩阵先隐藏、不伪称可用。
- 只读硬边界（BR-PRV-DATA-001）与双链分离（BR-DOC-DATA-001）保持；导出落 workspace 内（X-FORBID-004）。
- 与编辑工具栏共用 edToolbarParts 组件，保证风格、可访问性一致，避免两套实现漂移。

## 文件范围

- `src/components/ed/edToolbarParts.tsx`（新：TbBtn / Sep / ToolbarMenu 共享组件）
- `src/components/ed/EditorToolbar.tsx`（改为引用共享组件，删除私有副本）
- `src/components/prv/PrvToolbar.tsx`（新：预览模式自适应工具栏）
- `src/components/prv/PrvPanel.tsx`（头部接入 PrvToolbar）
- `src/components/prv/PrvToolbar.test.tsx`（新：5 模式自适应矩阵测试）
- `src/styles.css`（预览工具栏样式复用/补充）
- `docs/01_modules/ED/A-ED-X-R-03_编辑器工具栏功能与兼容性清单.md`（§2 新增预览模式行）
- `docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md`（变更记录）
- `docs/01_modules/PRV/A-PRV-M-T-01_预览输出技术设计.md`（变更记录 / 预览工具栏矩阵）
- `.governance/issues/PRV-TOOLBAR-ADAPTIVE-01.md`（本文）

## 验收

- PrvToolbar 与编辑器工具栏同风格（共用组件/样式类），按 mode 自适应工具集。
- pdf（原生 docx）：创建草稿进行编辑 + 导出(PDF/MD) + 打印；pdf（xlsx/pptx）：导出(PDF) + 打印；table：导出(PDF/按能力) + 打印 + 缩放；image：缩放；video/audio：无额外工具。
- 不提供"关闭"工具；只读边界不弱化；导出守界。
- 影响测试、治理审计和构建通过。

## Task Runtime Report — 预览模式工具栏自适应

task_type: CODE_CHANGE
governance_phase: IMPLEMENTATION
final_state: READY
issue_trace: PRV-TOOLBAR-ADAPTIVE-01

changed_files:

- `src/components/ed/edToolbarParts.tsx`：抽取 TbBtn / Sep / ToolbarMenu 为共享模块，EditorToolbar 与 PrvToolbar 复用。
- `src/components/ed/EditorToolbar.tsx`：改用共享组件，删私有副本（行为不变）。
- `src/components/prv/PrvToolbar.tsx`：预览模式工具栏，按 mode 自适应（pdf/table/image/video/audio）。
- `src/components/prv/PrvPanel.tsx`：头部接入 PrvToolbar。
- `src/components/prv/PrvToolbar.test.tsx`：5 模式自适应矩阵测试。
- `src/styles.css`：预览工具栏样式。
- `docs/01_modules/ED/A-ED-X-R-03_编辑器工具栏功能与兼容性清单.md`：§2 新增预览模式适配行。
- `docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md`、`docs/01_modules/PRV/A-PRV-M-T-01_预览输出技术设计.md`：变更记录与预览工具栏矩阵。

test_impact:

- direct: PRV
- affected: ED、SYS

validation:

- `npm run governance:generate`：passed。
- `npm run test:related -- --files …`：前端相关测试通过；TypeScript 通过。
- `npm run governance:audit`：passed。
- `npm run build`、`git diff --check`：passed。

unresolved_items:

- 预览面板 iframe PDF 缩放与 WKWebView 原生查看器绑定，工具栏不提供缩放（真机确认视需要补充）。
- csv/tsv 导出能力取决于 prv.export 后端，矩阵先隐藏、待后端确认后开放。
