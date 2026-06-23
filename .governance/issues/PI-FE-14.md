# Issue Trace · PI-FE-14 富文本 Markdown 往返（tiptap-markdown）

承接 A-CORE-X-L-03（前端集成增量）/ A-ED-M-T-01（md/docx 统一编辑模型）/ ED-EDIT / BR-ED-DATA-001（blockId）/ X-FORBID-007。替换 PI-FE-09 的纯文本往返为 Markdown 解析/序列化。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - package.json                                 # 新增 tiptap-markdown（peer @tiptap/core ^3，兼容已装 3.27）
  - package-lock.json
  - src/components/ed/extensions/index.ts        # 注册 Markdown 扩展
  - src/components/ed/Editor.tsx                 # md 文件 setContent(md 解析) / 保存 getMarkdown()
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md
  - .governance/active_scope.json
  - .governance/issues/PI-FE-14.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/**
confirmation: YES
scope_reason: PI-FE-14 纯前端接 tiptap-markdown 做 md WYSIWYG 往返（本地库，离线可用）；人类已确认网络装包；按计划 P2 收尾。
```

## Expected Behavior

- 接 tiptap-markdown（本地库，运行时离线）：扩展注册于 buildEditorExtensions。
- 打开 .md/.markdown 文件 → setContent 以 Markdown 解析为富文本（# → 标题、**粗体** 等可见渲染）；
  保存 → editor.storage.markdown.getMarkdown() 序列化回 Markdown 文本，经 ws.save_doc 落盘。
- 非 md（.txt 等）维持纯文本往返（每行一段落 ↔ \n），避免误解析。
- blockId 为运行时属性，不持久化到 md（往返后重新生成，BR-ED-DATA-001 稳定性在会话内有效）。
- 落盘内容/基线仍以后端为准（X-FORBID-007）；md 往返保真度限 StyleSubset，超出部分记债务。

## Validation Commands

```
npx tsc --noEmit
npm run build
npm test
npm run governance:audit -- --issue .governance/issues/PI-FE-14.md
```

验收：tsc/build/vitest 绿；打开 md 显示富文本渲染、保存写回合法 md；txt 仍纯文本往返；scoped 门禁无越界。
