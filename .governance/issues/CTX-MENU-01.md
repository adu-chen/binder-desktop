# Issue Trace：CTX-MENU-01

## 目标

右键菜单优化（简化方案 v2，2026-08-06 人类确认）：统一轻量 ContextMenu 组件（SYS 前端交互，
键盘可达 + 视口钳制 + token 化），覆盖全部右键目标面——文件树文件/目录节点、编辑器选区/光标、
DiffCard、表格/图片、标签栏；零新增后端命令（全部复用既有命令面）；不新增测试文件；
同步回写 A-SYS-C-T-02 设计 SSOT。应用原生菜单栏 1.0 不实现（人类决策，已记录 L-06 v1.1）。

## 1. 用户可见症状

1. 右键菜单仅文件树文件节点有，且无键盘可达（Esc/方向键/aria）、无视口钳制、目录节点无右键。
2. 编辑器右键无任何菜单（WKWebView 无系统右键菜单）：无复制/剪切/粘贴入口，也无 AI 入口
   （局部修改/辅助续写仅快捷键可用）。
3. DiffCard、表格/图片、已打开文档标签均无右键快捷操作。

## 2. 入口点

- `src/components/ws/FileTreeNode.tsx`：`onContextMenu`（文件节点右键上抛，目录节点无）。
- `src/components/ws/FileTree.tsx`：`contextMenu` 状态 + 内联菜单 JSX（重命名/导出/删除）。
- `src/components/ed/Editor.tsx`：`openInlineAssist(view)` / `openAutocompleteAssist(view)`（⌘K/⌘J
  既有辅助入口，右键复用）；`editor-host` 为右键挂载点。
- `src/components/de/DiffCard.tsx`：卡片动作区（接受/拒绝按钮）。
- `src/components/ed/TableMoveControls.tsx` / `ImageMoveControls.tsx`：选中态浮动控件。
- `src/components/shell/TabStrip.tsx`：标签行（× 关闭）。

## 3. 相关规则

`X-FORBID-007`（菜单启停态投影后端活动态，不另立业务真源）、`BR-AG-STATE-003`（AI 辅助项门禁
前置 + 静默语义）、`BR-SYS-DATA-003`（动作经既有 command 触达后端，本任务不新增命令）、
`BR-WS-DATA-001`（文件操作守界）、`X-FORBID-004`（workspace 路径边界）、`X-FORBID-005`
（ED 受控编辑接口）。`X-SEQ-002` 复用既有 Service→command 路径。

## 4. 相关链路

`WS-FS → (create/rename/delete_doc) · PRV-EXPORT · AG-ASSIST（局部修改/辅助续写） · DE-DIFF
（accept/reject_diff） · ED-EDIT（表格/图片删除） · SYS（视图层导航态：tabsStore 关闭标签）`

## 5. 生产者块

- `BR-AG-STATE-003-RB-AG-AG-ASSIST-007`（src/components/ed/Editor.tsx）：⌘K/⌘J 辅助入口，
  右键复用同一触发路径（门禁前置 + 静默关闭语义不变）。
- `BR-ED-DATA-001-RB-ED-ED-EDIT-024`（src/components/ed/ImageMoveControls.tsx）：
  图片删除（`deleteActiveImage`）。
- `BR-ED-DATA-007-RB-ED-ED-EDIT-019`（src/components/ed/TableMoveControls.tsx）：
  表格删除（`deleteTable`）。

## 6. 消费者块

- `BR-WS-DATA-001` 相关块（src-tauri/src/workspace/fileops.rs）：ws.create/rename/delete_doc
  路由（仅调用，不改）。
- `acceptDiff/rejectDiff`（src/services/deService.ts）：DiffCard 右键动作（仅调用，不改）。
- `prv.export`（src/services/prvService.ts）：文件树导出（仅调用，不改）。
- `assistStore`（src/store/assistStore.ts）：编辑器 AI 菜单项 disabled 态投影。

## 7. 路径上的空规则块

新增 `ContextMenu` 组件为纯展示组件（无业务规则），按 SYS 纯 UI 组件惯例以 `rules: []` + triage
`VALID_EMPTY`（展示组件无独立业务规则）标注；文件树迁移后的菜单项逻辑沿用既有 RB 块覆盖
（重命名/导出/删除均为原逻辑搬移，不引入新业务判断）。

## 8. boundary 假设

- 菜单启停/disabled 态全部由调用方从既有投影 store 计算传入（workspaceShell/edStore/deStore/
  assistStore），ContextMenu 不持有业务状态（X-FORBID-007）。
- 编辑器复制/剪切经 TipTap 命令（受控编辑，X-FORBID-005 不绕过）；粘贴经
  `navigator.clipboard.readText()` → `insertContent`（WKWebView 可用性列为 G4 实测项，
  失败则菜单项禁用并保留快捷键路径）。
- 目录右键仅「在此新建文件」（复用 `ws.create_doc` 子目录路径）；目录重命名/删除无命令面，
  1.0 不提供（明示后续增强）。

## 9. 运行时假设

- 编辑器右键需 `preventDefault` 并捕获触发瞬间选区/光标（ProseMirror 选区在 contextmenu 后可能
  被 DOM 重建）；AI 项复用 openInlineAssist/openAutocompleteAssist 既有锚点捕获。
- 图片/表格右键在选中态浮动控件上处理（stopPropagation），不与编辑器全局右键冲突。
- 标签右键仅投影 `tabsStore`（视图层导航态，X-FORBID-007 允许）。

## 10. 疑似不变量破坏

未发现。文件树重命名/导出/删除逻辑原样迁移（不改变行为）；AI 项沿用既有门禁路径（不新增
模型调用面）；后端零改动（无命令面变更）。唯一行为变化：编辑器右键从"无菜单"变为"有菜单"。

## 11. 验证计划

- ContextMenu 组件：渲染/点击触发/Esc 关闭/方向键导航/视口钳制（jsdom 布局受限则钳制逻辑单测）。
- 文件树：迁移回归（重命名/删除/导出路径不回归，现有 mock 扩展）。
- DiffCard 右键：pending 态接受/拒绝触发。
- 编辑器右键：有选区/无选区菜单项集正确；AI 项 disabled 投影。
- 标签右键：关闭标签。
- `npm run test:related -- --files [changed_files]`；`npm run governance:audit`。
- C-T-02 变更记录 + 交互小节回写；无需 `governance:generate`（不新增 TERM/RULE 标注块）。

## 文件范围

### 允许修改

```
.governance/active_scope.json                          # 替换活动锁为本 Scope
.governance/issues/CTX-MENU-01.md                      # 本文
src/components/shell/ContextMenu.tsx                    # 新增统一 ContextMenu 组件（SYS）
src/styles.css                                          # .ctx-menu 样式基线（提升自 .ftn-menu）
src/components/ws/FileTreeNode.tsx                      # 目录节点右键 + 文件节点复制路径
src/components/ws/FileTree.tsx                          # 右键菜单迁移到 ContextMenu
src/components/ed/Editor.tsx                            # 编辑器右键（剪切/复制/粘贴 + AI 项）
src/components/ed/TableMoveControls.tsx                 # 表格右键：删除表格
src/components/ed/ImageMoveControls.tsx                 # 图片右键：删除图片
src/components/de/DiffCard.tsx                          # DiffCard 右键：接受/拒绝（pending）
src/components/shell/TabStrip.tsx                       # 标签右键：关闭标签
src/components/shell/ShellComponents.test.tsx           # ContextMenu + 标签右键测试（并入）
src/components/ws/WorkspaceUI.test.tsx                  # 文件树迁移回归测试（并入）
src/components/ed/EditorCore.test.tsx                   # 编辑器右键测试（并入）
src/components/de/DiffReview.test.tsx                   # DiffCard 右键测试（并入）
docs/01_modules/SYS/A-SYS-C-T-02_前端交互架构与流程总图.md # 右键菜单交互小节 + 变更记录（SSOT 回写）
docs/00_core/A-CORE-C-R-01_ADUS.md                      # governance:generate 产物（如需要）
```

### 禁止修改

```
ADU.md / ADP.md / AGENTS.md / CLAUDE.md / governance.config.json
scripts/test_governance.mjs / scripts/generate_adus.mjs / scripts/audit_taskflow_governance.mjs
src-tauri/src/**                                        # 本任务零后端命令
src/services/closeService.ts / deService.ts / wsService.ts / prvService.ts / agService.ts  # 仅调用不改
```

## 状态

confirmation: YES（人类 2026-08-06 下达实现指令，方案 v2 经多轮确认：统一 ContextMenu +
零新增后端命令 + 复制/粘贴纳入 + 菜单栏 1.0 不实现）
