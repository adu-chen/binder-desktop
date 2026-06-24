# Issue Trace · PI-BE-07+FE-21 文件树空目录可见

承接 A-WS-I-P-01 §3（ws.list_docs）/ BR-WS-DATA-001 / WS-FS / X-FORBID-004 / X-FORBID-007 / TERM-WS-001。收口 PI-FE-15/17 标注的"空目录可见"债务。

文档补充：A-WS-I-P-01 §3 注记 ws.list_docs 返回项含目录条目（FsEntry：rel_path + doc_type + is_dir），不改 SYS 共享 DocRef 语义。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - docs/01_modules/WS/A-WS-I-P-01_工作台接口协议.md   # ws.list_docs 返回项含目录条目注记
  - src-tauri/src/workspace/listing.rs                  # FsEntry（含 is_dir）+ list_docs 含目录 + 测试
  - src-tauri/src/lib.rs                                # route_list_docs 序列化新结构（签名不变）
  - src/services/wsService.ts                           # DocRefDto/View 增 is_dir
  - src/store/wsStore.ts                                # DocRefView 增 isDir
  - src/store/wsStore.test.ts                           # 投影测试补 isDir
  - src/components/ws/FileTree.tsx                      # buildTree 据 isDir 显示空目录
  - src/components/ws/buildTree.test.ts                 # 空目录可见用例
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md
  - .governance/active_scope.json
  - .governance/issues/PI-BE-07-FE-21.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/src/ipc/mod.rs
  - src-tauri/src/types/mod.rs
  - src-tauri/src/workspace/lifecycle.rs
confirmation: YES
scope_reason: PI-BE-07+FE-21 让 list_docs 返回目录条目（FsEntry.is_dir）使空目录在文件树可见；不动 DocRef 共享语义；人类已授权继续。
```

## Expected Behavior

- 后端 list_docs 返回 FsEntry 列表（rel_path + doc_type + is_dir）：递归 root 内每个非隐藏目录与文件各一条
  （守界 X-FORBID-004，跳隐藏）；空目录也有条目（is_dir=true）。
- 前端 buildTree 据 is_dir 建树：目录条目（含空目录）创建目录节点，文件条目创建叶子；同路径去重；
  目录在前、按名排序。空 workspace 仍显示"为空"。
- 不改 SYS 共享 DocRef；文件树点击/搜索/状态投影行为不变。

## Validation Commands

```
cargo test
npx tsc --noEmit
npm test
npm run build
npm run governance:audit -- --issue .governance/issues/PI-BE-07-FE-21.md
```

验收：cargo test 全过（含空目录条目 covers BR-WS-DATA-001）；含空子目录的 workspace 文件树显示该目录；vitest 全过；scoped 门禁无越界。
