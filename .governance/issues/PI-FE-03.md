# Issue Trace · PI-FE-03 欢迎页完整实现

承接 A-CORE-X-L-03 §2 PI-FE-03 / A-SYS-C-T-02 §4.1 应用启动流程 / A-WS-I-P-01 §2（ws.list_workspaces/switch_workspace）/ X-FORBID-007 / TERM-WS-001。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - src-tauri/src/lib.rs                        # dispatch 增 ws.list_workspaces/switch_workspace/active_workspace 路由 + 注册
  - src/services/workspaceService.ts            # listWorkspaces/switchWorkspace/restoreActive service
  - src/store/workspaceShell.ts                 # 最近列表 + 恢复态投影
  - src/shell/Welcome.tsx                        # 最近列表 + 空/加载/错误态
  - src/shell/App.tsx                            # 启动恢复活动 workspace
  - src/styles.css                              # 欢迎页最近列表样式
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md   # PI-FE-03 状态回写
  - .governance/active_scope.json
  - .governance/issues/PI-FE-03.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/src/ipc/mod.rs
  - src-tauri/src/workspace/lifecycle.rs
confirmation: YES
scope_reason: PI-FE-03 复用后端既有 lifecycle 函数（list/switch/active），仅在 lib.rs 加路由 + 前端欢迎页；人类已授权连续推进。
```

## Expected Behavior

- 启动时 App 调 ws.active_workspace 恢复上次活动 workspace；有则进工作台，无则欢迎页（投影后端控制态，X-FORBID-007）。
- 欢迎页"打开/新建 workspace"沿用 PI-FE-00 目录选择器链路。
- 欢迎页"最近的 workspace"调 ws.list_workspaces 渲染列表，点击项调 ws.switch_workspace 切换并进工作台。
- 最近列表具备空态（无历史）、加载态（请求中）、错误态（统一错误形状 BR-SYS-OBS-002）三态。
- workspace 活动态/列表均后端投影，前端不另立权威态。

## Validation Commands

```
npx tsc --noEmit
cargo test
npm run build
npm run governance:audit -- --issue .governance/issues/PI-FE-03.md
```

验收：tsc/cargo/build 绿；scoped 门禁结构/confirmation/scope 全过，无越界。
