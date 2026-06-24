# Issue Trace · PI-BE-08+FE-22 PRV 预览（prv.preview + 预览面板）

承接 A-PRV-I-P-01 §2（prv.preview：DocRef→只读 PDF，docx/Excel/PPT 经 DOC 预览链）/ A-PRV-M-T-01（BR-PRV-DATA-002 管线归属、只读硬边界）/ PRV-VIEW / A-DOC（libreoffice_runtime 运行时）/ X-FORBID-004 / X-FORBID-007 / X-SEQ-002。接已就绪的 LibreOffice 预览链。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - src-tauri/src/prv/mod.rs                      # render_preview：DocRef→PDF 字节（经 DOC libreoffice 链）
  - src-tauri/src/lib.rs                          # 注册 + 路由 prv.preview
  - src/services/prvService.ts                   # previewDoc（字节回传 → blob）
  - src/store/prvStore.ts                         # 预览态（previewPath）
  - src/components/prv/PrvPanel.tsx              # PDF 只读呈现 + 失败/不支持态
  - src/components/ws/FileTreeNode.tsx           # 预览类型（docx/xlsx/pptx）点击 → 预览
  - src/components/ws/FileTreeNode.test.tsx       # 预览类型可点（更新 not_supported 用例）
  - src/components/ws/FileTree.tsx               # onPreviewFile → prvStore
  - src/shell/Workbench.tsx                        # center：有预览则 PrvPanel 否则 Editor
  - src/styles.css                              # 预览面板样式
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md
  - .governance/active_scope.json
  - .governance/issues/PI-BE-08-FE-22.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/src/ipc/mod.rs
  - src-tauri/src/doc/libreoffice_runtime.rs
  - src-tauri/src/doc/pandoc_runtime.rs
confirmation: YES
scope_reason: PI-BE-08+FE-22 接已就绪的 DOC 预览链（libreoffice convert_to_pdf）为 prv.preview 命令 + PRV 只读面板；字节回传；人类已确认 B + 字节回传。
```

## Expected Behavior

- 后端 prv.preview(relPath)：守界解析（X-FORBID-004）；扩展名→PreviewKind（docx→Docx / xls(x)→Excel /
  ppt(x)→Presentation；其余明确不支持）；经 doc::libreoffice_runtime::convert_to_pdf 出 PDF，读字节回传
  { format:"pdf", bytes }；LibreOffice 缺失/渲染失败 → 明确失败态（不静默空白，BR-PRV-DATA-002）。
- 前端：文件树点击预览类型文件（docx/xlsx/pptx 等）→ 预览面板替代编辑区显示 PDF（字节→blob iframe）；
  只读无编辑（PRV 硬边界）；加载/失败/不支持/LibreOffice 未装各有提示；关闭回编辑区。
- 预览态前端持有（视图层）；不可编辑（不进 ED）；落点守界（X-FORBID-004）。

## Validation Commands

```
cargo test
npx tsc --noEmit
npm test
npm run build
npm run governance:audit -- --issue .governance/issues/PI-BE-08-FE-22.md
```

验收：cargo test 全过；点 docx/xlsx/pptx → 预览 PDF（或环境无 LibreOffice 时明确失败态）；只读；vitest 全过；scoped 门禁无越界。
