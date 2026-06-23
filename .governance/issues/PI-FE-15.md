# Issue Trace · PI-FE-15 健壮性收口（死项目回退 + 文件树刷新）

承接 A-CORE-X-L-03（前端健壮性）/ A-SYS-C-T-02 §4.1（启动恢复活动态）/ WS-FS / X-FORBID-007 / X-SEQ-002。补 PI-FE-10 标注的"启动恢复 root 存在性探测"债务。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - src/services/workspaceService.ts             # restoreActiveWorkspace 探测死项目 + notice
  - src/services/wsService.ts                     # fetchDocs 抽取（供探测复用）
  - src/store/workspaceShell.ts                   # notice 字段
  - src/shell/Welcome.tsx                          # 显示死项目提示
  - src/components/ws/FileTree.tsx               # 手动刷新按钮 + 仅 idle 时加载
  - src/styles.css                              # 刷新/提示样式
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md
  - .governance/active_scope.json
  - .governance/issues/PI-FE-15.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/**
confirmation: YES
scope_reason: PI-FE-15 纯前端健壮性——启动探测死项目回退欢迎页 + 文件树手动刷新；空目录可见/搜索需后端枚举改动，留作后续；人类已授权按建议继续。
```

## Expected Behavior

- 启动恢复：restoreActiveWorkspace 取活动 workspace 后**探测 root 可达**（经 ws.list_docs，读目录失败=死项目）；
  可达 → 进工作台并以探测结果预置文件树（避免二次枚举）；死项目 → ws.close_workspace + 停在欢迎页 +
  提示"上次的 workspace 已不可用"。
- 文件树：标题区常驻**手动刷新**按钮（重新 ws.list_docs）；仅在 wsStore.status=idle 时自动加载（被恢复预置或切换重置驱动）。
- 空 workspace 文案保持"workspace 为空"（空目录可见/全文搜索需后端枚举/ws.search，本 PI 不做，记债务）。
- 活动态/列表仍后端投影（X-FORBID-007）；前端不另立权威态。

## Validation Commands

```
npx tsc --noEmit
npm run build
npm run governance:audit -- --issue .governance/issues/PI-FE-15.md
```

验收：tsc/build 绿；历史项目目录被删后启动回退欢迎页+提示；文件树刷新按钮可重新加载；scoped 门禁无越界。
