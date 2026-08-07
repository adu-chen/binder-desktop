# Issue Trace · WS-DIR-DELETE-01 文件树文件夹删除（两级语义：空快速删 / 非空确认递归）

承接 WS-DIR-DELETE-FIX-PLAN-01 v0.2（2026-08-06 人类确认）：两级删除语义、不加清理空文件夹独立入口、
后端零既有命令行为变更、复用 BR-WS-DATA-001/WS-FS。

## 1. 用户可见症状

目录节点右键菜单仅有「在此新建文件」，无删除入口；文件级删除不回收父目录，workspace 残留大量空目录
（`list_docs` 使空目录可见）却无法清除。后端 `ws.delete_doc` 仅 `remove_file`，对目录必然失败。

## 2. 入口点

- `src/components/ws/FileTree.tsx`：`menuItems` 目录分支（右键菜单，CTX-MENU-01 迁移后）——新增「删除」项 → `doDeleteFolder`。
- `src-tauri/src/lib.rs#route_delete_doc`（L736-L744）→ `fileops::delete_doc`（L108-L115，仅文件）。
- 后端目标：新增 `ws.delete_folder` 命令面（镜像 `ws.create_folder` 拆分模式）。

## 3. 相关规则

- `BR-WS-DATA-001`（workspace 内文档/资源读、写、列举、建目录与跨文档操作必经 WS 物理边界层）
- `X-FORBID-004`（workspace 路径边界，`resolve_active` 守界）
- `X-SEQ-002`（前端经 Service→command 触达后端，不跨层）
- 需求承接：REQ-WS-002（文件 I/O，含删除），本次补目录删除命令面

## 4. 相关链路

`WS-FS`（active, P1）：新增目录删除块仍在该链路（create_folder FS-004 / delete_doc FS-003 同链）。

## 5. 生产者块

- `BR-WS-DATA-001-EFFECT-WS-WS-FS-003`（src-tauri/src/workspace/fileops.rs#delete_doc）——不改，语义保持
- 新增 `BR-WS-DATA-001-EFFECT-WS-WS-FS-007`（fileops.rs#delete_folder，mode: empty|recursive）
- 新增 `cleanup_bdocx_under`（listing.rs，@GOV EFFECT，chain WS-FS，rules BR-WS-DATA-001）

## 6. 消费者块

- 新增 `route_delete_folder`（lib.rs）——str_param(relPath/mode) → resolve_active → delete_folder → cleanup_bdocx_under
- 新增 `wsService.deleteFolder(relPath, mode)`——前端命令封装
- `FileTree.tsx#doDeleteFolder`——目录右键菜单动作（两级：docs 投影判空 → empty/confirm+recursive）

## 7. 路径上的空规则块

- 无新增 `rules: []` 块。wsService 薄服务层、FileTree 组件沿用既有无 @GOV 状态（不新增业务判断
  之外的导出执行块）；目录删除为空判断为 UI 视图投影，不引入新 triage。

## 8. boundary 假设

- `delete_doc` @GOV `in=docAbsolutePath | out=deletedDoc(rejectedWhenMissing)` 不变化（不触碰）。
- 新增 `delete_folder` boundary：`in=folderAbsolutePathAndRecursiveMode | out=deletedFolder(rejectedWhenMissingOrNotDirOrNotEmptyWhenNonRecursive)`。
- 协议文档 A-WS-I-P-01 §3 新增行（命令面契约），无既有行变更。

## 9. 运行时假设

- `caller_context=async_chain`：doDeleteFolder 为异步链（deleteFolder → loadDocs → 会话清理）。
- 空判断基于 wsStore docs 投影（`!docs.some(d => d.relPath.startsWith(path + "/"))`）；投影与实际磁盘
  可能存在竞态，由后端 empty 模式 WS_NOT_EMPTY 兜底（错误 alert，不静默）。
- confirm 在用户手势内执行（window.confirm）。

## 10. 疑似不变量破坏

- 未发现。`ws.delete_doc` 文件删除语义与调用方（FileTree.doDelete）零影响；AG 工具注册表不变
  （`!is_registered("delete_file")` 断言保持）；BDocx 清单仅新增清洗路径（rename 清洗先例已有）。

## 11. 验证计划

- Rust：fileops 单测（empty 删空/非空拒/文件拒/缺失拒 + recursive 递归）；listing 单测（cleanup 前缀/子目录/保留无关/无命中不写）；
  cargo test 全量（lib.rs 注册/路由编译验证）。
- 前端：WorkspaceUI.test.tsx（目录右键「删除」空/非空两级 + 刷新）；workspaceRuntime.test.ts（deleteFolder 命令名/mode 断言）。
- 治理：`npm run test:plan` / `test:related -- --files [changed_files]`（WS critical + Rust 变更→rust 回归）；
  `npm run governance:audit`；`npx tsc --noEmit`；`npm run build`。
- 无需 `governance:generate`（不新增/修改标注块静态定义；A-WS-I-P-01 为协议行新增非规则标注）。
- 人工验证（UI，列为未自动化项）：空/非空删除视觉与确认、BDocx 目录删除后清单、会话关闭。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: WS-DIR-DELETE-01
governance_phase: IMPLEMENTATION
allowed_files:
  - .governance/active_scope.json              # 替换活动锁为本 Scope
  - .governance/issues/WS-DIR-DELETE-01.md     # 本文
  - .governance/issues/WS-DIR-DELETE-FIX-PLAN-01.md  # 方案（v0.2 已收敛）
  - src-tauri/src/workspace/fileops.rs         # delete_folder(mode)
  - src-tauri/src/workspace/listing.rs         # cleanup_bdocx_under
  - src-tauri/src/lib.rs                       # route_delete_folder + registry
  - docs/01_modules/WS/A-WS-I-P-01_工作台接口协议.md  # §3 新增命令行 + 变更记录
  - src/services/wsService.ts                  # deleteFolder 封装
  - src/components/ws/FileTree.tsx             # 目录右键「删除」两级
  - src/components/ws/WorkspaceUI.test.tsx     # 删除交互测试（并入）
  - src/services/workspaceRuntime.test.ts      # deleteFolder 断言（并入）
  - scripts/test_governance.test.mjs           # WS 用例数快照同步（如新增用例）
forbidden_files:
  - ADU.md / ADP.md / AGENTS.md / CLAUDE.md / APC / governance.config.json
  - scripts/test_governance.mjs / scripts/generate_adus.mjs
  - src-tauri/src/ag/** / src-tauri/src/doc/**  # 不扩散 AG/DOC
confirmation: YES  # 人类 2026-08-06 确认 WS-DIR-DELETE-FIX-PLAN-01 v0.2 并指示实施
scope_reason: 文件夹删除两级语义（前端-only 交互 + 后端新增 ws.delete_folder 命令面），复用既有规则/链路
```
