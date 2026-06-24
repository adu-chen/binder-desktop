# Issue Trace · PI-BE-09+FE-23 PRV 导出 + 打印

承接 A-PRV-I-P-01 §2（prv.export 落 workspace 内、prv.print 系统打印）/ A-PRV-M-T-01（BR-PRV-DATA-002）/ PRV-VIEW / DOC（pandoc/libreoffice 运行时）/ X-FORBID-004 / X-SEQ-002。复用已就绪转换链。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - src-tauri/src/prv/mod.rs                      # export：md↔docx（pandoc）/ office→pdf（libreoffice）落 workspace
  - src-tauri/src/lib.rs                          # 注册 + 路由 prv.export
  - src/services/prvService.ts                   # exportDoc
  - src/components/prv/PrvPanel.tsx              # 打印当前预览（iframe print）
  - src/components/ws/FileTree.tsx               # 右键导出菜单（按源类型）
  - src/styles.css                              # 样式
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md
  - .governance/active_scope.json
  - .governance/issues/PI-BE-09-FE-23.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/src/ipc/mod.rs
  - src-tauri/src/doc/libreoffice_runtime.rs
  - src-tauri/src/doc/pandoc_runtime.rs
confirmation: YES
scope_reason: PI-BE-09+FE-23 复用 pandoc/libreoffice 做 prv.export（落 workspace 守界）+ PrvPanel 打印当前预览；音视频预览另起（需 asset 协议）；人类已授权继续。
```

## Expected Behavior

- 后端 prv.export(relPath, format)：目标落点 = 同目录同名换扩展（workspace 内，守界 X-FORBID-004）；
  md/txt→docx（pandoc markdown_to_docx）、docx→md（pandoc docx_to_markdown）、office(docx/xls/ppt)→pdf
  （libreoffice render_preview 字节）；不支持组合明确拒绝；回导出 rel_path。
- 前端：文件树右键按源类型给导出项（md→DOCX；docx→PDF/MD；xls/ppt→PDF），导出后刷新文件树 + 提示落点。
- 打印：PrvPanel 打印按钮 → 打印当前预览 PDF（iframe 系统打印对话框）。
- 落点守界（X-FORBID-004）；经 Service→command（X-SEQ-002）。

## Validation Commands

```
cargo test
npx tsc --noEmit
npm test
npm run build
npm run governance:audit -- --issue .governance/issues/PI-BE-09-FE-23.md
```

验收：导出生成目标文件并刷新树（环境有 pandoc/libreoffice 时）；预览打印唤起打印；scoped 门禁无越界。
