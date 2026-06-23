# Issue Trace · PI-BE-06+FE-17 workspace 全文搜索（ws.search）

承接 A-WS-I-P-01 §3（ws.search：workspace_id+查询 → 命中列表 DocRef+定位，限界 X-FORBID-004）/ BR-WS-DATA-001 / WS-FS / X-FORBID-004 / X-SEQ-002 / TERM-WS-001。收口 PI-FE-15 标注的搜索债务。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - src-tauri/src/workspace/listing.rs           # 新增 search（守界，行级命中）+ 测试
  - src-tauri/src/lib.rs                          # 路由 ws.search + 注册
  - src/services/wsService.ts                     # searchDocs service
  - src/components/ws/FileTree.tsx               # 搜索框 + 结果列表（点击打开）
  - src/styles.css                              # 搜索样式
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md
  - .governance/active_scope.json
  - .governance/issues/PI-BE-06-FE-17.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/src/ipc/mod.rs
  - src-tauri/src/workspace/lifecycle.rs
confirmation: YES
scope_reason: PI-BE-06+FE-17 实现 ws.search（守界全文搜索）+ 文件树搜索框/结果；收口搜索债务；人类已授权继续。
```

## Expected Behavior

- 后端 ws.search(query)：在活动 workspace root 下递归搜索文本文件，行级大小写不敏感子串匹配，回命中列表
  { rel_path, line, preview }（守界 X-FORBID-004，跳隐藏/非 UTF-8/超大文件，结果有上限）。空查询回空。
- 前端文件树标题区搜索框：输入即查（防抖）；有查询 → 显示结果列表（路径:行 + 预览），点击 → 打开该文件；
  清空 → 回文件树。经 Service→command（X-SEQ-002）。
- 路径守界由后端保证；状态以后端为准（X-FORBID-007）。

## Validation Commands

```
cargo test
npx tsc --noEmit
npm test
npm run build
npm run governance:audit -- --issue .governance/issues/PI-BE-06-FE-17.md
```

验收：cargo test 全过（命中/跳隐藏/守界 covers BR-WS-DATA-001）；搜索框出结果、点击打开；scoped 门禁无越界。
