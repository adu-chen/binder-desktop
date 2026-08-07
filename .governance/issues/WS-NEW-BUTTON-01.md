# Issue Trace · WS-NEW-BUTTON-01 文件树「＋」新建按钮修复（格式下拉 + 激活区域 + 文件夹入口）

承接 WS-NEW-BUTTON-FIX-PLAN-01（方案讨论稿）人类确认（2026-08-06）：格式清单按默认
（文件夹/md/txt/html/docx/csv/自定义，不含预览类格式）、激活区域=最近点击目录、
固定格式自动补扩展名、后端零改动。

## 1. 用户可见症状

文件树头部「＋」按钮点击后直接展开一条自由文本输入行，要求手输完整相对路径（含扩展名）；
无格式选择、无法新建文件夹、恒在根目录新建，无法在当前所在子目录直接新建。

## 2. 入口点

- `src/components/ws/FileTree.tsx`：header「＋」按钮 onClick（L300）→ `setInput({mode:"create"})`
  → inputRow（L313-L332）→ `submitCreate`（L162-L177）。
- `src/components/ws/FileTreeNode.tsx`：目录行点击（L96-L108，仅展开/折叠）。

## 3. 相关规则

- `BR-WS-DATA-001`（workspace 内文档/资源读、写、列举、建目录必经 WS 物理边界层）
- `X-FORBID-004`（workspace 路径边界，后端 resolve_active 守界）
- `X-SEQ-002`（前端经 Service→command 触达后端，不跨层）
- `X-FORBID-007`（视图层导航态不另立业务真源——activeDir 定位为视图态，同 openPath 层级）
- 需求承接：REQ-WS-002（文件 I/O），本次为 UI 交互增强，无新增需求层声明

## 4. 相关链路

`WS-FS`（active, P1）：新建文档（create_doc）与新建目录（create_folder）均在该链路。

## 5. 生产者块

- `BR-WS-DATA-001-EFFECT-WS-WS-FS-001`（src-tauri/src/workspace/fileops.rs#create_doc）— 本次仅调用，不改
- `BR-WS-DATA-001-EFFECT-WS-WS-FS-004`（fileops.rs#create_folder）— 本次仅调用，不改
- `route_create_doc / route_create_folder`（src-tauri/src/lib.rs）— 本次仅调用，不改

## 6. 消费者块

- `src/components/ws/FileTree.tsx#submitCreate` — 新建后 loadDocs + 打开（本次扩展 format/分流）
- `src/services/wsService.ts` — 新增 `createFolder` 封装（镜像 createDoc）
- `src/components/ws/FileTreeNode.tsx` — 新增 activeDir 渲染/激活回调
- `src/components/shell/ContextMenu.tsx` — 复用为「＋」格式下拉（纯展示，不改）

## 7. 路径上的空规则块

- FileTree/FileTreeNode 组件与 wsService 薄服务层均无 @GOV（既有状态，非本次新增业务判断）；
  激活区域为 UI 视图态，不引入新的 `rules: []` 块，不新增 triage 记录。

## 8. boundary 假设

- 路径上无 @GOV 块声明 `lifecycle_assumption=` / `document_snapshot=`，不适用。

## 9. 运行时假设

- `caller_context=async_chain`：submitCreate 为异步链（createFolder/createDoc → loadDocs → 打开）。
- 新建下拉以按钮 `getBoundingClientRect()` 坐标弹 ContextMenu；jsdom 中该值恒为 0，
  组件测试以触发 onSelect 断言行为（不依赖坐标像素）。
- 打开分流复用 `PREVIEW_ONLY` 集合（自 FileTreeNode 导出，单一来源），不与文件节点
  `onPreviewFile` 逻辑漂移。

## 10. 疑似不变量破坏

未发现。后端命令语义不变（create_doc/create_folder 零改动）；目录右键「在此新建文件」路径
不改（submitCreate 的 dirPath 语义保持，format 参数为可选新增）；文件节点打开逻辑不动。

## 11. 验证计划

- 测试：workspaceRuntime.test.ts（createFolder 命令名/参数断言）；WorkspaceUI.test.tsx
  （「＋」下拉弹出、文件夹创建、Markdown 补扩展名、激活区域前缀、自定义格式）——均并入既有文件。
- 治理：`npm run test:plan` / `test:related -- --files [changed_files]`；`npm run governance:audit`
  无 error；`npx tsc --noEmit`；`npm run build`。
- 无需 `governance:generate`（不新增/修改标注块、协议文档、静态定义）。
- 人工验证（UI，列为未自动化项）：下拉视觉、激活区域高亮、docx BDocx 新建编辑链。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: WS-NEW-BUTTON-01
governance_phase: IMPLEMENTATION
allowed_files:
  - .governance/active_scope.json              # 替换活动锁为本 Scope
  - .governance/issues/WS-NEW-BUTTON-01.md     # 本文
  - src/services/wsService.ts                  # 新增 createFolder 封装
  - src/components/ws/FileTree.tsx             # ＋下拉 + activeDir + submitCreate 扩展
  - src/components/ws/FileTreeNode.tsx         # activeDir 高亮 + 导出 PREVIEW_ONLY
  - src/styles.css                             # .ftn-dir-active + 激活区域 chip 样式
  - src/components/ws/WorkspaceUI.test.tsx     # 新建交互测试（并入）
  - src/services/workspaceRuntime.test.ts      # createFolder 断言（并入）
forbidden_files:
  - ADU.md / ADP.md / AGENTS.md / CLAUDE.md / APC / governance.config.json
  - scripts/test_governance.mjs / scripts/test_governance.test.mjs / scripts/generate_adus.mjs
  - src-tauri/src/**                           # 本方案后端零改动
  - docs/**（除 Scope 内不存在）                # 本方案不改协议/设计文档
confirmation: YES  # 人类 2026-08-06 确认 WS-NEW-BUTTON-FIX-PLAN-01 并指示实现
scope_reason: 「＋」新建按钮修复（格式下拉 + 激活区域 + 文件夹入口 + 打开分流），前端-only，复用既有 command
```
