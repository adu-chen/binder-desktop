# Issue Trace · PI-BE-04+FE-13 文件操作（新建/重命名/删除 + 右键菜单）

承接 A-WS-I-P-01 §3（ws.create_doc）+ §8（move/copy 签名随实现补充）/ A-WS-M-T-01 BR-WS-DATA-001 / REQ-WS-002（读写/列举/跨文档操作）/ WS-FS / X-FORBID-004 / X-FORBID-007 / X-SEQ-002 / TERM-WS-001。

文档补充（解除 §8 待补项）：A-WS-I-P-01 §3 新增 `ws.rename_doc` / `ws.delete_doc`，§8 标记 move 签名已补。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - docs/01_modules/WS/A-WS-I-P-01_工作台接口协议.md   # 新增 rename/delete + §8 解除
  - src-tauri/src/workspace/fileops.rs                  # 新增：create/rename/delete（守界）
  - src-tauri/src/workspace/mod.rs                      # 声明 pub mod fileops
  - src-tauri/src/lib.rs                                # 路由 ws.create_doc/rename_doc/delete_doc + 注册
  - src/services/wsService.ts                           # createDoc/renameDoc/deleteDoc
  - src/components/ws/FileTree.tsx                      # 右键菜单 + 新建/重命名输入 + 操作编排
  - src/components/ws/FileTreeNode.tsx                  # onContextMenu 透传
  - src/styles.css                                     # 菜单/输入样式
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md
  - .governance/active_scope.json
  - .governance/issues/PI-BE-04-FE-13.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/src/ipc/mod.rs
  - src-tauri/src/workspace/lifecycle.rs
confirmation: YES
scope_reason: PI-BE-04+FE-13 补 WS 文件 CRUD（create/rename/delete 守界）+ 文件树右键菜单；补协议 §8 move 签名；人类已授权按计划 P2 继续。
```

## Expected Behavior

- 后端（守界 X-FORBID-004，越界拒绝）：
  - ws.create_doc(relPath, content?)：目标已存在则拒（WS_EXISTS）；建父目录 + 写入；回 { rel_path, baseline }。
  - ws.rename_doc(relPath, newRelPath)：目标已存在则拒；同 workspace 内移动（rename）；回 { rel_path }。
  - ws.delete_doc(relPath)：删除文件；不存在/越界拒。
- 前端文件树：
  - 标题区"+"新建 → 内联输入文件名 → ws.create_doc → 刷新 + 打开新文件。
  - 右键文件节点 → 菜单：重命名（内联输入）/ 删除（确认）。重命名/删除影响当前打开文档时同步编辑器
    （重命名→重开新路径；删除→清空编辑器）。
  - 操作后刷新文件树（loadDocs）。
- 文件操作经 Service→command（X-SEQ-002），不跨层直连；活动态后端投影（X-FORBID-007）。

## Validation Commands

```
cargo test
npx tsc --noEmit
npm run build
npm run governance:audit -- --issue .governance/issues/PI-BE-04-FE-13.md
```

验收：cargo test 全过（create/rename/delete 守界 + 已存在拒绝 covers BR-WS-DATA-001）；前端可新建/重命名/删除文件并刷新树；scoped 门禁无越界。
