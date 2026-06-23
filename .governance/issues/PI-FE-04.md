# Issue Trace · PI-FE-04 侧边栏文件树（WS 前端）

承接 A-CORE-X-L-03 §2 PI-FE-04 / A-SYS-C-T-02 §3 Side Panel WS / A-WS-I-P-01 §3（ws.list_docs）/ BR-WS-DATA-001 / WS-FS / X-FORBID-004 / TERM-WS-001。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - src-tauri/src/workspace/listing.rs          # 新增：workspace 内文件枚举（守 X-FORBID-004 边界）
  - src-tauri/src/workspace/mod.rs              # 声明 pub mod listing
  - src-tauri/src/lib.rs                        # dispatch 增 ws.list_docs 路由 + 注册
  - src/services/wsService.ts                   # 新增：listDocs service
  - src/store/wsStore.ts                        # 新增：文件树状态投影
  - src/components/ws/FileTree.tsx              # 新增：文件树容器
  - src/components/ws/FileTreeNode.tsx          # 新增：单节点（6 状态）
  - src/shell/Workbench.tsx                      # side 区按 activePanel 挂载 FileTree
  - src/styles.css                              # 文件树样式
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md
  - .governance/active_scope.json
  - .governance/issues/PI-FE-04.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/src/ipc/mod.rs
  - src-tauri/src/workspace/lifecycle.rs
confirmation: YES
scope_reason: PI-FE-04 新增 WS 文件枚举（边界守护复用 resolve_in_workspace）+ 前端文件树；人类已授权连续推进。
```

## Expected Behavior

- 后端 ws.list_docs(workspace_id) 递归枚举活动 workspace root 下文件，回 DocRef 列表；
  跳过隐藏项（.binder/.git 等）；任一路径不越出 root（X-FORBID-004）；doc_type 由扩展名推断。
- 前端 side 区在 activePanel=files 时渲染文件树；从 ws.list_docs 真实加载，按 rel_path 构树。
- FileTreeNode 6 状态视觉互斥：normal / open（锈红左条）/ dirty_open（红点）/ pending_diff（数字 badge）/
  external_changed（金 !）/ not_supported（灰禁用，xlsx/pptx）。本 PI 接通 normal + not_supported，
  其余状态预留接口（由后续 PI 的 ed/de 投影驱动）。
- 文件树状态后端投影（X-FORBID-007）；search 路径守护由后端执行，前端只转发。

## Validation Commands

```
npx tsc --noEmit
cargo test
npm run build
npm run governance:audit -- --issue .governance/issues/PI-FE-04.md
```

验收：tsc/cargo/build 绿；ws.list_docs 有越界拒绝测试（covers BR-WS-DATA-001）；scoped 门禁无越界。
