# Issue Trace · PI-FE-20 搜索结果跳行（打开并定位到命中行）

承接 A-CORE-X-L-03 / PI-BE-06+FE-17（全文搜索）/ ED-EDIT / X-FORBID-007。收口"点搜索结果未跳行"债务。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - src/services/edService.ts                    # openDocument 增 jumpLine 参数
  - src/store/edStore.ts                          # loadDoc 携带 jumpLine + 清除
  - src/components/ed/Editor.tsx                 # 载入后跳转到目标行（块）
  - src/components/ws/FileTree.tsx               # 搜索命中点击 → openDocument(path, line)
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md
  - .governance/active_scope.json
  - .governance/issues/PI-FE-20.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/**
confirmation: YES
scope_reason: PI-FE-20 点搜索命中 → 打开文件并定位到命中行（块），闭合搜索体验；纯前端；人类已授权继续。
```

## Expected Behavior

- 点搜索结果 → openDocument(relPath, line) → ws.read_doc 载入 → 载入后将光标定位到目标行对应块并滚动到视区。
- 行→块映射：纯文本（每行一段落）精确；md（解析为富文本）按块序号近似（块索引≈行号，钳制到块数）——记债务。
- jumpLine 随 loadDoc 原子传递，setContent 后执行跳转，跳后清除（不影响后续普通打开）。
- 经 Service→command（X-SEQ-002）；状态后端投影（X-FORBID-007）。

## Validation Commands

```
npx tsc --noEmit
npm test
npm run build
npm run governance:audit -- --issue .governance/issues/PI-FE-20.md
```

验收：点搜索命中打开文件并光标落在该行附近、滚动到视区；普通打开（无 line）行为不变；vitest 全过；scoped 门禁无越界。
