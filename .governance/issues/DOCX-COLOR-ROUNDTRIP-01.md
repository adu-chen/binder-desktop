# Issue Trace — DOCX 文字颜色往返保真

## 1. 用户可见症状

在编辑器中设置的文字颜色写回 DOCX 后丢失；再次打开时恢复为默认颜色。现有链路把 `text_color`
列为 BestEffort，却没有针对实际文档的检测、报告或界面告知，`doc.get_style_report` 也未注册。

## 2. 入口点

| 入口 | 文件 | 函数 |
|---|---|---|
| DOCX 打开 | `src-tauri/src/doc/edit_chain.rs` | `docx_to_html` |
| DOCX 保存 | `src-tauri/src/doc/edit_chain.rs` | `html_to_docx` |
| 命令接口 | `src-tauri/src/lib.rs` | `route_open_for_edit` / style report 路由 |
| 编辑器装载 | `src/services/edService.ts` | `openDocument` |
| 用户告知 | `src/components/ed/EditorStatusBar.tsx` | `EditorStatusBar` |

## 3. 相关规则

`BR-DOC-DATA-002`、`BR-DOC-DATA-003`、`BR-DOC-STATE-001`、`BR-ED-DATA-002`。

## 4. 相关链路

`DOC-CONVERT → ED-EDIT → WS-SAVE`。

## 5. 生产者块

- `BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-003`：DOCX → HTML。
- `BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-004`：HTML → DOCX。
- `BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-001/002`：样式能力分类与风险告知。

## 6. 消费者块

- `src/services/docService.ts`：消费 `doc.open_for_edit` 与 `doc.get_style_report` 结果。
- `src/services/edService.ts`、`src/store/edStore.ts`：将样式报告绑定到当前编辑会话。
- `src/components/ed/Editor.tsx`、`EditorStatusBar.tsx`：呈现风险告知。

## 7. 路径上的空规则块

前端样式告知为既有 `BR-DOC-DATA-002` 的消费端投影，不新增规则或链路。

## 8. boundary 假设

- Pandoc 负责结构与基础文字转换，但不可靠地写回/读出 HTML `color` CSS。
- DOCX 的 `word/document.xml` 是颜色的权威存储位置，直接字体色位于 `w:rPr/w:color@w:val`。
- 编辑器仅消费 HTML 内联 `color:#RRGGBB`，由既有 `TextStyle + Color` 扩展渲染。

## 9. 运行时假设

- 所有 DOCX 均为可读取的 ZIP/OOXML 包，且 `word/document.xml` 存在。
- 颜色所对应的文本在 Pandoc 生成 HTML 后可按顺序匹配；不能匹配时必须计入报告并提示，而不是静默声称保真。

## 10. 疑似不变量破坏

- HTML `color` 未落入 `w:color`，破坏“编辑器样式写回 DOCX”的往返不变量。
- DOCX `w:color` 未恢复为 HTML 内联样式，破坏“重开可继续编辑”的往返不变量。
- 风险未检测和未告知，破坏 `BR-DOC-DATA-002` 的不静默丢失约束。

## 11. 验证计划

- Rust：HTML 红字 → DOCX `w:color` XML 断言 → DOCX 重开 HTML `color` 断言。
- Rust：颜色报告包含实测结果与不能恢复时的风险。
- 前端：服务层透传报告、会话绑定、状态栏告知测试。
- 运行相关 Rust/前端测试、类型检查和 `npm run governance:audit`。

## 验证结果（2026-07-27）

- `cargo test -p binder docx_color --lib`：3/3 通过。
- `cargo test -p binder style_subset --lib`：3/3 通过。
- `cargo test -p binder html_docx_roundtrip_preserves_subset_when_pandoc_available --lib`：通过；断言最终
  `word/document.xml` 含 `w:color w:val="FF0000"`、无中转标记，重开 HTML 含 `color:#FF0000`。
- `npm exec tsc -- --noEmit`：通过。
- 定向前端回归（docService、edService、edStore、EditorStatusBar）：37/37 通过。
- `npm run governance:audit` 与 `git diff --check`：通过。
- 全量 `cargo test -p binder`：212/214 通过；两个失败均在本 Scope 外且未修改：
  `ag::blocks::tests::html_replace_escapes_special_chars`（HTML 转义期望不符）和
  `doc::libreoffice_runtime::tests::docx_to_pdf_end_to_end_when_libreoffice_available`（LO Worker signal=6）。
