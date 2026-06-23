# Issue Trace · PI-FE-10 workspace 切换 / 返回欢迎页

承接 A-SYS-C-T-02 §4.1（WB -. 关闭/切换 .-> C）/ A-WS-I-P-01 §2（ws.switch_workspace / ws.close_workspace）/ WS-FS / BR-WS-PERSIST-002（单活动） / X-FORBID-007 / X-SEQ-002 / TERM-WS-001。补实现既有设计，不改 SSOT/APC。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - src-tauri/src/lib.rs                         # 注册 + 路由 ws.close_workspace
  - src/services/workspaceService.ts             # closeWorkspace + 切换/打开重置下游 store
  - src/store/wsStore.ts                          # reset()
  - src/store/edStore.ts                          # reset()
  - src/store/deStore.ts                          # reset()
  - src/components/shell/WorkspaceMenu.tsx        # 标题栏 workspace 菜单（切换/打开/返回 + 脏态守卫）
  - src/shell/Workbench.tsx                        # 挂菜单 + 按 workspaceId 重挂面板
  - src/styles.css                               # 菜单样式
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md
  - .governance/active_scope.json
  - .governance/issues/PI-FE-10.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/src/ipc/mod.rs
  - src-tauri/src/workspace/lifecycle.rs
confirmation: YES
scope_reason: PI-FE-10 补实现 A-SYS-C-T-02 §4.1 的切换/返回欢迎链路（后端接 close_workspace、前端菜单 + 重置下游 store + 脏态守卫）；人类已授权落地。
```

## Expected Behavior

- 后端 ws.close_workspace(workspaceId)：置 is_active=0（控制态保留可恢复，lifecycle 已实现），接入 dispatch。
- 标题栏 workspace 菜单（A-SYS-C-T-02 §3 WS 切换归侧/外壳）：
  - 切换 → 最近列表（ws.list_workspaces）→ ws.switch_workspace；
  - 打开其他文件夹 → 目录选择 → ws.open_workspace；
  - 返回欢迎页 → ws.close_workspace + 投影 activeWorkspace=null → App 回 Welcome（X-FORBID-007）。
- 切换/打开/关闭前脏态守卫：编辑器 dirty 或有 pending diff → 确认（继续/取消），避免静默丢弃（参考稿链路 G）。
- 切换/打开后重置下游 store（wsStore 文件树 / edStore 编辑会话 / deStore diff），并按 workspaceId 重挂面板，
  杜绝切到新项目仍显示旧项目内容、或按旧基线误存盘。
- 活动态/切换均为后端控制态投影，前端不另立权威态（X-FORBID-007）。

## Validation Commands

```
npx tsc --noEmit
cargo test
npm run build
npm run governance:audit -- --issue .governance/issues/PI-FE-10.md
```

验收：tsc/cargo/build 绿；可从 Workbench 切换项目/返回欢迎页；切换后文件树与编辑器为新项目内容；
脏态守卫生效；ws.close_workspace 接线；scoped 门禁无越界。
