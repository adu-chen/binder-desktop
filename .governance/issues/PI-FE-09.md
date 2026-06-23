# Issue Trace · PI-FE-09 前后端落盘集成（打开/保存真实文档）

承接 A-CORE-X-L-03（前端计划集成增量）/ A-WS-I-P-01（ws.read_doc/save_doc）/ WS-FS / WS-SAVE / SM-WS-01 / BR-WS-OBS-001 / BR-WS-PERSIST-001 / BR-ED-STATE-001 / X-SEQ-002 / X-FORBID-007 / TERM-WS-001。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - src/services/wsService.ts                    # readDoc / saveDoc service
  - src/services/edService.ts                    # openDocument 编排（read_doc → 载入编辑器）
  - src/store/edStore.ts                          # loadDoc(content,baseline) + markSaved
  - src/components/ws/FileTree.tsx               # 点击文件 → openDocument
  - src/components/ed/Editor.tsx                 # 载入真实内容 + ⌘S 保存（基线/冲突）
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md
  - .governance/active_scope.json
  - .governance/issues/PI-FE-09.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/**
  - src/store/deStore.ts
confirmation: YES
scope_reason: PI-FE-09 把 PI-BE-01 落盘 command 接到前端——文件树点击打开真实文档、⌘S 保存；纯前端集成；人类已授权继续。
```

## Expected Behavior

- 文件树点击文件 → ws.read_doc(relPath) → 编辑器载入真实内容 + 记加载基线（edStore，X-SEQ-002 经 Service）；
  当前打开路径与脏态前端投影（X-FORBID-007）。
- 编辑器以纯文本忠实往返载入/保存（每行一段落 ↔ \n 连接）；富文本 md 解析/序列化（tiptap-markdown 或
  DOC pandoc 路）记为债务，本 PI 不做。
- ⌘S → ws.save_doc(relPath, content, baseline)：成功刷新基线、tab→saved；磁盘≠基线且未 override →
  WS_SAVE_CONFLICT（SM-WS-01），前端提示冲突（保留我的编辑覆盖 / 放弃重载）。
- 不支持类型（xlsx/pptx）文件树禁用打开（沿用 PI-FE-04 not_supported）。
- 编辑器活动态以后端基线为准；前端不另立可漂移的落盘态。

## Validation Commands

```
npx tsc --noEmit
npm run build
npm run governance:audit -- --issue .governance/issues/PI-FE-09.md
```

验收：tsc/build 绿；点击文件载入内容；⌘S 调 ws.save_doc 刷新基线；冲突路径有提示；scoped 门禁无越界。
