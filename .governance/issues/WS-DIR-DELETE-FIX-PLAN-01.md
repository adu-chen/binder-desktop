# 工作台文件树·文件夹右键删除功能方案（WS-DIR-DELETE-FIX-PLAN-01）

> 性质：方案讨论稿（非 A 状态治理文档）。供人类审阅决策后，再按治理流程进入
> Issue Trace → Scope Lock → 实现。
> 本文不创建任何静态规则定义；文中出现的规则/链路 ID 均为既有注册项（BR-WS-DATA-001 / WS-FS），
> 不做新增。新增 command 属协议契约变更，须同步 A-WS-I-P-01。

## 1. 背景与目标

CTX-MENU-01（2026-08-06 人类确认 v2）曾记录「目录重命名/删除无命令面，1.0 不提供（明示后续增强）」。
本轮人类新增需求：**文件夹右键菜单也要实现删除功能**——即补齐该缺口中的「删除」项
（目录重命名仍不在 1.0 范围，维持原决策）。

目标行为（需求，2026-08-06 人类决策收敛）：

- 文件树目录节点右键菜单增加「删除」项（danger 样式，与文件节点删除一致）。
- **两级删除语义**（人类决策）：空目录直接删除（无需确认）；非空目录弹确认（提示目录名，不可撤销）后递归删除。
- 删除后刷新文件树；若目录下存在已打开文档会话，一并关闭相关会话。
- **不加「清理空文件夹」功能**（人类决策：仅删除；空目录经两级删除的快速通道覆盖，清理空目录不做独立入口）。
- 文件节点删除逻辑保持不变。

## 2. 现状与差距分析

### 2.1 现状盘点

| 层 | 现状 | 位置 |
|---|---|---|
| 前端-目录菜单 | 目录右键仅「在此新建文件」一项 | `src/components/ws/FileTree.tsx` L246-L258（menuItems） |
| 前端-删除逻辑 | `doDelete` 仅面向文件（`ws.delete_doc`），确认后刷新；若删的是当前打开文档则重置编辑器 | `src/components/ws/FileTree.tsx` L219-L231 |
| 前端-Service | `deleteDoc(relPath)` → `ws.delete_doc` | `src/services/wsService.ts` L75-L78 |
| 后端-命令 | `ws.delete_doc` → `fileops::delete_doc` → `std::fs::remove_file`（**仅文件，目录删除失败**） | `src-tauri/src/lib.rs` L735-L744 / `src-tauri/src/workspace/fileops.rs` L108-L115 |
| 后端-建目录 | `ws.create_folder` 已注册（目录幂等/文件冲突拒） | `lib.rs` L704-L714 / `fileops.rs` L51-L67 |
| 协议 | §3 已登记 `ws.create_folder`，无目录删除命令 | `docs/01_modules/WS/A-WS-I-P-01_工作台接口协议.md` |
| 身份清单 | bdocx_manifest 记录文件树/Agent 创建的 BDocx 路径；删除后不清洗 | `src-tauri/src/workspace/listing.rs` |
| AG 工具 | 工具注册表**刻意不注册** delete 工具（测试断言 `!is_registered("delete_file")`） | `src-tauri/src/ag/tool_registry.rs` |

### 2.2 差距结论

1. **无目录删除命令面**：`ws.delete_doc` 仅 `remove_file`，目录删除必然失败；UI 层也无入口。
2. **删除语义不对称**：建目录有独立命令 `ws.create_folder`（与 `ws.create_doc` 分开），
   删除侧却只有文件删除命令——命令面不对称，应补齐 `ws.delete_folder`。
3. **递归删除副作用未处理**：目录删除将带走其下 BDocx 文档，bdocx_manifest 会产生陈旧路径
   （当前仅 rename 有 `update_bdocx_for_rename` 清洗，delete 无对应逻辑）。
4. **打开会话一致性**：删除目录时其下文档可能正被打开（edStore 会话、tabs 标签），
   需一并关闭，避免指向不存在文件的残留会话。

## 3. 目标架构

```
文件树目录节点右键 ──▶ ContextMenu「删除」──▶ 两级删除：
      │                      ① 空目录（docs 投影无 dir/ 前缀条目）→ 直接 ws.delete_folder mode=empty
      │                      ② 非空目录 → confirm（提示目录名，不可撤销）→ ws.delete_folder mode=recursive
      ▼
wsService.deleteFolder(dirRelPath, mode)        wsService.deleteDoc 保持不动（文件删除不变）
      │  effect=IPC-ws.delete_folder
      ▼
后端 ws.delete_folder（新增注册 + 路由，参数 mode: "empty" | "recursive"）
      │
      ├─▶ fileops::delete_folder（empty：remove_dir 非空拒 WS_NOT_EMPTY；recursive：remove_dir_all）
      └─▶ listing::cleanup_bdocx_under（删除目录前缀下的 BDocx 清单路径；empty 命中为空则不写）
      │
      ▼
刷新文件树（loadDocs）；关闭被删目录下的打开会话
```

要点：

- **新增独立命令** `ws.delete_folder`，镜像 `ws.create_folder` 的拆分模式（create_doc ≠ create_folder，
  故 delete_doc ≠ delete_folder）。不动 `ws.delete_doc` 语义，对既有文件删除调用方零影响。
- **规则复用**：目录删除与文件删除同属 BR-WS-DATA-001（workspace 内文件/资源操作必经 WS 物理边界层，
  X-SEQ-002 / X-FORBID-004）。先例：`create_folder` 的 @GOV 即映射 BR-WS-DATA-001-EFFECT-WS-WS-FS-004。
  **不新增 RULE / CHAIN / TERM**。
- **不扩散到 AG**：Agent 工具注册表维持「不注册 delete 工具」现状（tool_registry 测试断言不变），
  本需求仅 UI 入口。

## 4. 方案设计

### 4.1 后端（Rust，`src-tauri/src/`）

**（1）`workspace/fileops.rs` 新增 `delete_folder(abs_path, recursive: bool)`**

```
逻辑（两级，2026-08-06 人类决策）：
  1. 不存在 → WS_NOT_FOUND「目录不存在」（不静默成功，对齐 delete_doc）
  2. 存在但不是目录（is_file 等）→ WS_EXISTS「目标已存在且不是目录」（对齐 create_folder 冲突语义）
  3. recursive=false（empty 模式）→ std::fs::remove_dir，非空报 WS_NOT_EMPTY「目录非空」
  4. recursive=true（recursive 模式）→ std::fs::remove_dir_all 递归删除；失败 → WS_IO_ERROR
```

@GOV 建议（延续既有 EFFECT-WS-WS-FS 序号，现有 FS-001~006，新块取 007）：

```
/**
 * @GOV
 * codes:    BR-WS-DATA-001-EFFECT-WS-WS-FS-007
 * type:     EFFECT
 * chain:    WS-FS
 * rules:    BR-WS-DATA-001
 * boundary: in=folderAbsolutePathAndRecursiveMode | out=deletedFolder(rejectedWhenMissingOrNotDirOrNotEmptyWhenNonRecursive) | does_not=remove_single_file
 */
```

**（2）`workspace/listing.rs` 新增 `cleanup_bdocx_under(root, dir_rel)`**

删除目录前缀下的 BDocx 清单路径（含子目录），写回清单。对齐 rename 的 `update_bdocx_for_rename` 清洗纪律。
无命中不写文件。@GOV 类型 EFFECT，chain WS-FS，rules BR-WS-DATA-001。

**（3）`lib.rs`**

- `build_registry()` 注册 `ws.delete_folder`（放在 `ws.delete_doc` 附近，L190-L192 区间）。
- `dispatch` match 增加 `"ws.delete_folder" => route_delete_folder(...)`（L295-L298 区间）。
- 新增 `route_delete_folder`：`str_param(relPath)` + `str_param(mode)` → `resolve_active`（守界，X-FORBID-004）→
  `fileops::delete_folder(abs, mode == "recursive")` → `listing::cleanup_bdocx_under` → 回 `{ rel_path, deleted: true }`。
- 可选加固：拒绝 relPath 顶层段以 `.` 开头的目录（隐藏目录）。当前 `list_docs` 本就跳过隐藏项、
  UI 无法触达，此项为纵深防御（建议做，成本一行）。

**（4）Rust 单测（并入既有 `#[cfg(test)]`，不新增测试文件）**

- `fileops.rs`：empty 模式删空目录成功、非空拒 WS_NOT_EMPTY、目标是文件拒 WS_EXISTS、缺失拒 WS_NOT_FOUND；
  recursive 模式递归删除含子项目录。
  `// covers: BR-WS-DATA-001`。
- `listing.rs`：`cleanup_bdocx_under` 清理目录前缀与子目录路径、保留无关路径；无命中不写。

### 4.2 前端（TS/React，`src/`）

**（1）`services/wsService.ts` 新增 `deleteFolder(relPath, mode)`**

镜像 `deleteDoc`（L75-L78）：`callCommand("ws.delete_folder", { relPath, mode })`（mode: "empty" | "recursive"）。

**（2）`components/ws/FileTree.tsx`**

- `menuItems` 目录分支（L246-L258）：在「在此新建文件」后追加
  `{ type: "separator" }` + `{ type: "item", id: "delete-folder", label: "删除", danger: true, onSelect: () => void doDeleteFolder(menu.path) }`。
- 新增 `doDeleteFolder(path)`（两级，2026-08-06 人类决策）：
  - 空判断基于 wsStore docs 投影：`!docs.some((d) => d.relPath.startsWith(path + "/"))`——无 `path/` 前缀条目即为空目录；
  - 空目录 → 直接 `deleteFolder(path, "empty")`（无需 confirm）；
  - 非空目录 → `window.confirm` 文案含「将删除文件夹「path」及其中所有内容，此操作不可撤销」→ `deleteFolder(path, "recursive")`；
  - 调 `loadDocs()` 刷新（空/非空统一）；`edStore`/`tabsStore` 中 relPath 以 `path + "/"` 开头的会话关闭/重置（防空壳残留会话，空目录路径天然无会话）。
- 文件删除路径（`doDelete`/`deleteDoc`）保持不动。

**（3）前端测试（并入既有文件，WS 前端测试文件 baseline=2 冻结）**

- `src/components/ws/WorkspaceUI.test.tsx`：目录右键菜单出现「删除」；空目录点击 → 直接触发
  `ws.delete_folder { mode: "empty" }` 且无 confirm；非空目录 → confirm 后触发 `mode: "recursive"`；刷新。
- `src/services/workspaceRuntime.test.ts`：`deleteFolder` 命令名与 mode 参数断言（镜像既有
  `deleteDoc` 断言 L157-L163）。

### 4.3 协议文档（`docs/01_modules/WS/A-WS-I-P-01_工作台接口协议.md`）

§3 表新增一行（镜像 `ws.create_folder` 行）：

| command | 参数（语义） | 结果 | 特有错误类 | 状态机/规则归属 |
|---|---|---|---|---|
| `ws.delete_folder` | DocRef（目录 rel_path）、mode（empty \| recursive） | rel_path + deleted | 不存在（WS_NOT_FOUND）/目标为文件（WS_EXISTS）/非空（WS_NOT_EMPTY，empty 模式）/越界 | BR-WS-DATA-001 |

并追加变更记录行（v1.8）。

## 5. 实施步骤（建议顺序）

| # | 步骤 | 文件 | 验证 |
|---|---|---|---|
| 1 | Rust：`delete_folder` + `cleanup_bdocx_under` + 单测 | `fileops.rs` / `listing.rs` | `cargo test`（workspace 相关） |
| 2 | Rust：`route_delete_folder` + 注册 + 路由分支 | `lib.rs` | `cargo test` |
| 3 | 前端：`deleteFolder` service + 目录菜单「删除」+ 会话清理 | `wsService.ts` / `FileTree.tsx` | 组件测试 |
| 4 | 前端测试并入 | `WorkspaceUI.test.tsx` / `workspaceRuntime.test.ts` | `vitest` |
| 5 | 协议文档更新 | `A-WS-I-P-01` | 文档头/变更记录检查 |
| 6 | 全量验证 | — | `test:related` / `governance:audit` / `tsc` / `cargo test` / `build` |

## 6. 接口契约变更

| 接口 | 变更 | 说明 |
|---|---|---|
| `ws.delete_folder`（Tauri command） | 新增 | 镜像 `ws.create_folder`；`mode: "empty" \| "recursive"`——空目录快速删/非空递归删 |
| `deleteFolder`（wsService.ts） | 新增 | 镜像 `deleteDoc`，透传 mode |
| `ws.delete_doc` | 无变更 | 文件删除语义保持，调用方零影响 |
| AG delete 工具 | 无变更 | 维持不注册（tool_registry 测试断言不变） |

安全边界不变量（不得破坏）：删除目标路径必须落在活动 workspace root 内（X-FORBID-004，
`resolve_active` 守界）；不新增任何越界/绕过路径。

## 7. 治理要求（实施时执行）

1. **task_type / 阶段**：`CODE_CHANGE`（修改业务代码 + 协议文档）；governance_phase = IMPLEMENTATION。
   **须先执行 Issue Trace（ADU §6 十一项）→ 人类确认 Scope Lock** 后才可改码。
2. **规则/链路**：全部复用既有 `BR-WS-DATA-001` + 链路 `WS-FS`（active, P1）；
   **不新增 RULE / CHAIN / TERM / MODULE**，故无需 `governance:generate`（不改标注块文本）。
3. **@GOV**：新增 Rust 块（EFFECT-WS-WS-FS-007 等）按既有格式标注；`routes`/`wsService` 薄包装
   沿用既有标注风格（lib.rs route 与 wsService 均未逐函数标 @GOV，保持一致，不强行新增）。
4. **测试治理**：WS 模块 risk=critical、`requires_rust_on_related=true`——改 Rust 必跑 rust 回归
   （fileops/listing 内联单测 + core_chain 相关）；前端测试文件数冻结在 baseline=2，
   **新增用例并入既有文件，不新增测试文件**；新增断言带 `// covers: BR-WS-DATA-001`。
5. **决策变更留痕**：本需求覆盖 CTX-MENU-01「目录删除 1.0 不提供」决策（仅删除项；目录重命名维持原决策）。
   在 Issue Trace 或 Runtime Report 记录。

## 8. 风险与待确认项

| # | 事项 | 说明 | 决策点 |
|---|---|---|---|
| 1 | **活动 Scope 冲突（实施前置）** | 当前 active_scope.json 为 WS-NEW-BUTTON-01（已 READY 并提交）。本课题实施时需替换为新 Scope Lock | 实施前重新立项（人类确认） |
| 2 | 删除语义 | **已决策**：两级——空目录直接删（mode=empty），非空 confirm 后递归（mode=recursive）；不做回收站（记为后续债务） | 已收敛（2026-08-06） |
| 3 | 被删目录下的打开会话 | 目录删除后，其下已打开文档的编辑器会话/标签关闭/重置（空目录天然无会话） | 默认执行（方案含） |
| 4 | BDocx 清单清理 | 随删除清洗 `cleanup_bdocx_under`（防陈旧路径；empty 模式命中为空则不写） | 默认执行 |
| 5 | 隐藏目录防护 | 顶层段 `.` 开头目录拒绝删除（纵深防御；UI 当前不可触达） | 实施时确认（建议加） |
| 6 | 目录重命名 | 维持 CTX-MENU-01 决策，1.0 不提供 | 确认范围仅删除 |
| 7 | AG delete 工具 | 保持不注册（本需求只做 UI 入口） | 确认不扩散 |
| 8 | 清理空文件夹 | **已决策**：不加独立入口，空目录经两级删除快速通道覆盖 | 已收敛（2026-08-06） |

## 9. 验证计划

- Rust：`cargo test`（fileops/listing 单测：递归删除、缺失/文件冲突拒、清单清洗）。
- 前端：`npx vitest run src/components/ws/WorkspaceUI.test.tsx src/services/workspaceRuntime.test.ts`。
- 变更影响：`npm run test:plan -- --files [changed_files]` → `npm run test:related -- --files [changed_files]`
  （WS critical + Rust 变更会自动扩展 rust 回归）。
- 治理：`npm run governance:audit`（无 error 级违规）。
- 静态：`npx tsc --noEmit`；`npm run build`。
- 人工验证（UI）：目录右键 → 删除 → 空目录直接删 / 非空 confirm 后递归 → 树刷新；文件删除不回归；
  删除后其下打开会话关闭。
- 每阶段以实际 changed_files 计算，不得手选测试。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-08-06 | v0.1 | 初版：目录右键删除功能方案（新增 ws.delete_folder 命令面、递归删除 + BDocx 清单清洗 + 会话一致性、复用 BR-WS-DATA-001/WS-FS、覆盖 CTX-MENU-01 删除项决策） |
| 2026-08-06 | v0.2 | 人类决策收敛：①删除语义两级化（空目录 mode=empty 直接删 / 非空 confirm 后 mode=recursive，后端 WS_NOT_EMPTY）②不加「清理空文件夹」独立入口（空目录经两级删除快速通道覆盖） |
