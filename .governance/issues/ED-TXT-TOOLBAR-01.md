# Issue Trace：ED-TXT-TOOLBAR-01

## 目标

修正"纯文本（.txt 等）编辑不展示内容编辑工具栏"的误读决策：txt 编辑模式第二行提供段落组内容编辑工具栏（引用/代码块/分隔线）。仅展示可在纯段落文档（textToDoc 装载，每行 paragraph）生效且内容可保存（serialize→getText）的段落级结构工具；文字级样式（加粗/颜色/字体/字号等）不展示——txt 保存回磁盘为纯文本，文字样式不持久化。用户多轮确认：最终工具集 = 仅段落组（引用/代码块/分隔线），不含样式（标题）、文字、列表、插入、页面视图。

## 规则与链路

- 模块：ED（工具栏）
- 链路：ED-EDIT（文字样式应用 / 保存序列化）
- 复用规则：不新增规则/链路/约束/术语；A-ED-X-R-03 为工具栏功能矩阵 SSOT，本次更新纯文本行。

## 设计一致性

- txt 仍以 `editFormat="text"` 打开、`textToDoc` 纯段落装载、`serialize→getText` 保存（不改变装载/保存路径）。
- 第二行对 txt 展示：提示文案 + 段落组（引用/代码块/分隔线）。引用/代码块在纯段落文档上生效（Paragraph→Blockquote/CodeBlock），分隔线插入 hr；保存时文本内容保留。
- 不展示：样式（标题保存后丢失）、文字（字体/字号/颜色等保存必丢）、列表、插入、页面视图。
- 文档同步：A-ED-X-R-03 §2 纯文本行 + 变更记录 v1.5；A-ED-M-T-01 v1.58；A-ED-M-D-01 acceptance（"纯文本行为不变"→段落级结构工具栏表述）。

## 文件范围

- `src/components/ed/EditorToolbar.tsx`（纯文本第二行：提示 + 段落组）
- `src/components/ed/EditorControls.test.tsx`（纯文本断言更新）
- `docs/01_modules/ED/A-ED-X-R-03_编辑器工具栏功能与兼容性清单.md`（§2 纯文本行 + v1.5）
- `docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md`（v1.58）
- `docs/01_modules/ED/A-ED-M-D-01_编辑器功能需求描述.md`（acceptance）
- `.governance/issues/ED-TXT-TOOLBAR-01.md`（本文）
- `docs/00_core/A-CORE-C-R-01_ADUS.md`（重新生成）

## 验收

- txt 编辑第二行显示提示文案 + 段落组（引用/代码块/分隔线）；无样式/文字/列表/插入组。
- 引用/代码块/分隔线在 txt 会话内可应用且内容保存不丢。
- 相关测试、tsc、governance:generate/audit、build 通过。

## Task Runtime Report — txt 内容编辑工具栏

task_type: CODE_CHANGE
governance_phase: IMPLEMENTATION
final_state: READY
issue_trace: ED-TXT-TOOLBAR-01

changed_files:

- `src/components/ed/EditorToolbar.tsx`：纯文本第二行"纯文本模式"提示 → 提示 + 段落组（引用/代码块/分隔线）。
- `src/components/ed/EditorControls.test.tsx`：纯文本断言（段落组存在、样式/列表/插入不渲染）。
- `docs/01_modules/ED/A-ED-X-R-03_编辑器工具栏功能与兼容性清单.md`：§2 纯文本行 + v1.5。
- `docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md`：v1.58。
- `docs/01_modules/ED/A-ED-M-D-01_编辑器功能需求描述.md`：acceptance 纯文本表述更新。
- `docs/00_core/A-CORE-C-R-01_ADUS.md`：重新生成。

test_impact:

- direct: ED
- affected: 无

validation:

- `npx vitest run src/components/ed/EditorControls.test.tsx`：passed（12/12）。
- `npm run test:related`：passed（18 文件 / 431 测试）。
- `npm run governance:generate`：passed。
- `npm run governance:audit`：passed（governance_findings 清零）。
- `npx tsc --noEmit`、`npm run build`：passed。

unresolved_items:

- 无。
