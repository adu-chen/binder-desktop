# 工作台文件树「＋」新建按钮修复方案（WS-NEW-BUTTON-FIX-PLAN-01）

> 性质：方案讨论稿（非 A 状态治理文档）。供人类审阅决策后，再按治理流程进入
> Issue Trace → Scope Lock → 实现。
> 本文不创建任何静态规则定义；文中出现的规则/链路 ID 均为既有注册项（BR-WS-DATA-001 / WS-FS），
> 不做新增。本方案**后端零改动**（`ws.create_doc` / `ws.create_folder` 均已注册），
> 无协议契约变更。

## 1. 背景与目标

文件树头部「＋」按钮当前行为：点击后展开一条**自由文本输入行**，要求用户手输完整相对路径
（含目录与扩展名，placeholder 即"新文件相对路径，如 notes/a.md"），且恒在**根目录**新建。
存在四类缺口：

1. 无格式选择（靠手输扩展名，易错且不可发现）。
2. 无法新建**文件夹**（后端 `ws.create_folder` 已注册，前端无入口）。
3. 无"当前激活区域"概念（按钮恒根目录；仅有目录右键"在此新建文件"预填目录前缀）。
4. 新建后打开分流不完整（预览类如 csv 也应走预览而非编辑）。

目标行为（需求）：

- 点击「＋」→ 弹出**新建下拉列表**，列出可新建格式：文件夹、Markdown、纯文本、HTML、
  Word 文档、CSV 及**自定义格式**（保留任意扩展名能力）。
- 选择格式后在**当前激活区域**新建：
  - 激活区域 = 当前所在目录（用户最近点击/选中的目录节点；无则根目录）；
  - 在某文件夹下 → 建在该文件夹内；在根目录 → 建在根目录。
- 文件夹格式创建空目录；文档格式创建空文件并打开（可编辑类型进编辑器，预览类型进预览）。
- 目录右键「在此新建文件」保持现状不动（本方案不改其行为，避免范围扩散）。

## 2. 现状与差距分析

### 2.1 现状盘点

| 层 | 现状 | 位置 |
|---|---|---|
| 前端-＋按钮 | 点击 → `setInput({ mode: "create" })` + 空值 → 内联文本输入行，手输完整相对路径 | `src/components/ws/FileTree.tsx` L294-L310（header）/ L313-L332（inputRow） |
| 前端-提交 | `submitCreate(name, dirPath="")` → `createDoc(rel)` → 恒根目录（「＋」路径 dirPath 恒空） | `src/components/ws/FileTree.tsx` L162-L177 |
| 前端-打开 | 新建后一律 `openDocument(created)`（编辑）；预览类未分流 | `src/components/ws/FileTree.tsx` L170 |
| 前端-Service | 有 `createDoc`（`ws.create_doc`），**无 `createFolder` 封装** | `src/services/wsService.ts` L63-L67 |
| 前端-组件 | 统一 `ContextMenu`（纯展示：视口钳制/Esc/方向键/Enter/aria）可复用做下拉 | `src/components/shell/ContextMenu.tsx` |
| 前端-激活区域 | 无"当前目录"状态；仅 `openPath`（最近打开文件高亮）；目录行点击仅展开/折叠 | `FileTree.tsx` L70 / `FileTreeNode.tsx` L96-L108 |
| 后端-建文档 | `ws.create_doc`：任意扩展名、自动建父目录、空内容；docx 走 BDocx 链（初始 HTML → docx 字节 + mark_bdocx） | `lib.rs` L680-L702 |
| 后端-建目录 | `ws.create_folder`：目录幂等成功、目标为文件拒绝 | `lib.rs` L704-L714 |
| 编辑能力 | `md/markdown/txt/html/htm` → 文本/富文本编辑；`docx`（BDocx）→ DOC 编辑链；`xlsx/csv/pptx` 等 → 只读预览 | `edService.ts` L74-L82（isHtmlFile/isMarkdownFile）/ `FileTreeNode.tsx` L28 |

### 2.2 差距结论

1. **交互缺口**：无格式下拉、无文件夹入口——全部集中在 FileTree 头部交互。
2. **区域感知缺口**：无激活目录状态，"＋"无法落在当前所在目录。
3. **后端零缺口**：`create_doc`（含 docx BDocx 化）与 `create_folder` 均已具备，本方案**不动 Rust**。
4. **打开分流缺口**：新建后按类型分流（编辑/预览）需补齐，复用文件树既有 `openDocument`/预览打开路径。

## 3. 目标架构

```
文件树头部「＋」按钮 ──▶ ContextMenu 下拉（格式列表）
      │                      ① 文件夹
      │                      ② Markdown (.md)
      │                      ③ 纯文本 (.txt)
      │                      ④ HTML (.html)
      │                      ⑤ Word 文档 (.docx)   ── 后端 BDocx 化
      │                      ⑥ CSV (.csv)          ── 预览
      │                      ⑦ 自定义格式…（任意扩展名）
      ▼
选择格式 ──▶ 命名输入行（预填 激活区域 + 扩展名，仅输名称）
      ▼
提交
  ├─ 文件夹 ──▶ wsService.createFolder(rel) ──▶ ws.create_folder ──▶ 刷新（不打开）
  └─ 文档   ──▶ wsService.createDoc(rel)   ──▶ ws.create_doc   ──▶ 打开（编辑/预览按类型分流）

激活区域（新增状态 activeDir）：
  目录节点点击 ──▶ 展开/折叠 + 设为激活目录（行高亮）
  头部显示当前激活区域 + 一键回根目录
  默认（未激活任何目录）= 根目录
```

要点：

- **后端零改动**：复用 `ws.create_doc`（任意格式 + docx BDocx 化 + 自动建父目录）与
  `ws.create_folder`；仅前端新增 `createFolder` 封装。
- **复用既有交互组件**：下拉用统一 `ContextMenu`（CTX-MENU-01 产物），命名输入沿用现有
  `ftn-input-row`；目录右键"在此新建文件"路径不改。
- **规则复用**：新建文档/目录均映射 `BR-WS-DATA-001` + 链路 `WS-FS`（先例：create_doc FS-001 /
  create_folder FS-004）。**不新增 RULE / CHAIN / TERM / MODULE / command**。
- **需求承接**：文件 I/O（含新建）归 `REQ-WS-002`，本次为 UI 交互增强，无新增需求层声明。

## 4. 方案设计（全部前端，`src/`）

### 4.1 格式清单（建议，待确认）

```ts
const CREATE_FORMATS = [
  { key: "folder", label: "文件夹",             ext: null },
  { key: "md",     label: "Markdown (.md)",   ext: "md" },
  { key: "txt",    label: "纯文本 (.txt)",    ext: "txt" },
  { key: "html",   label: "HTML (.html)",     ext: "html" },
  { key: "docx",   label: "Word 文档 (.docx)", ext: "docx" },
  { key: "csv",    label: "CSV (.csv)",       ext: "csv" },
  { key: "custom", label: "自定义格式…",       ext: "custom" },
];
```

- 固定格式项：命名输入**只输名称**，提交时名称无扩展名则自动补 `ext`（如"笔记"→"笔记.md"）。
- `custom` 项：输入完整文件名（含扩展名，如 `data.json`），覆盖任意后端可写格式。
- 新建后打开分流：
  - 可编辑（md/txt/html/docx）：`openDocument(rel)`；
  - 预览类（csv）：`tabsStore.openPreview(rel)`（与文件树文件节点 `onPreviewFile` 同路径）；
  - 文件夹：不打开，仅刷新。

### 4.2 `services/wsService.ts` 新增 `createFolder`

镜像 `createDoc`（L63-L67）：`callCommand("ws.create_folder", { relPath })`，回 rel_path。

### 4.3 `components/ws/FileTree.tsx` 改造

**（1）激活区域状态**

- 新增 `activeDir: string` state（`""` = 根目录）。
- `FileTreeNode` 新增 prop `activeDir` 与 `onActivateDir(path)`：
  - 目录行点击 → 原展开/折叠逻辑不变，同时 `onActivateDir(node.path)`；
  - 激活目录行加视觉标记（class，如 `ftn-dir-active`，样式入 `styles.css`）。
- 头部「工作区」标签旁显示当前激活区域 chip：
  - 根目录 → `根目录`；有激活目录 → `notes/` 之类；附「回根目录」小按钮（`setActiveDir("")`）。
- 打开文件不改 `activeDir`（保持用户目录上下文）。

**（2）「＋」按钮下拉**

- 点击「＋」→ 取按钮 `getBoundingClientRect()` → 以 `(rect.left, rect.bottom)` 弹
  `ContextMenu`（items = CREATE_FORMATS 映射）。
- 选择格式 → `setInput({ mode: "create", dirPath: activeDir, format })` +
  `setValue(activeDir ? `${activeDir}/` : "")` → 关闭下拉。

**（3）命名输入行**

- 按 `format` 变化 placeholder：
  - `folder`：`文件夹名`；
  - 固定格式：`文件名（自动追加 .${ext}）`；
  - `custom`：`文件名.扩展名（如 data.json）`。
- 提交走扩展后的 `submitCreate`（见下），Escape/失焦关闭逻辑不变（L328-L329）。

**（4）`submitCreate` 扩展**

- 新增参数 `format`；`dirPath` 沿用现有语义（L164）。
- `folder` → `createFolder(rel)`（rel = 目录相对路径）→ `loadDocs()`，不打开。
- 固定格式 → 名称无扩展名则补 `ext` → `createDoc(rel, "")` → `loadDocs()` →
  按类型打开（编辑/预览分流）。
- `custom` → 原逻辑（`createDoc` → `openDocument`）。
- 已存在冲突等错误处理沿用现状（`window.alert`，L173-L175）。

### 4.4 样式（`src/styles.css`）

- `.ftn-dir-active`（激活目录行高亮）+ 头部激活区域 chip 样式（复用现有 token 变量）。

### 4.5 测试（并入既有文件，WS 前端测试文件 baseline=2 冻结）

- `src/components/ws/WorkspaceUI.test.tsx`：
  - 「＋」点击弹出格式下拉；选「文件夹」→ 输入名称 → 触发 `ws.create_folder` + 刷新；
  - 选「Markdown」→ 输入"笔记" → 触发 `ws.create_doc` 且 rel 为 `notes/笔记.md`（含激活目录前缀）；
  - 激活区域：点目录行 → activeDir 高亮；新建 rel 前缀随之变化；
  - 自定义格式：输入 `data.json` 走 createDoc。
- `src/services/workspaceRuntime.test.ts`：
  - `createFolder` 命令名与参数断言（镜像既有 `createDoc` 断言，L150 附近）。
- 断言带 `// covers: BR-WS-DATA-001`（并入既有 covers）。

## 5. 实施步骤（建议顺序）

| # | 步骤 | 文件 | 验证 |
|---|---|---|---|
| 1 | wsService 新增 `createFolder` | `src/services/wsService.ts` | workspaceRuntime 单测 |
| 2 | FileTree 激活区域状态（activeDir + 目录行激活 + 头部 chip） | `FileTree.tsx` / `FileTreeNode.tsx` / `styles.css` | 组件测试 |
| 3 | 「＋」按钮下拉（ContextMenu 复用） | `FileTree.tsx` | 组件测试 |
| 4 | 命名输入行 + `submitCreate` 扩展（补扩展名/文件夹/打开分流） | `FileTree.tsx` | 组件测试 |
| 5 | 测试并入 | `WorkspaceUI.test.tsx` / `workspaceRuntime.test.ts` | vitest |
| 6 | 全量验证 | — | `test:related` / `governance:audit` / `tsc` / `build` |

## 6. 接口契约变更

| 接口 | 变更 | 说明 |
|---|---|---|
| `ws.create_doc` / `ws.create_folder` | 无变更 | 后端已具备，本方案零 Rust 改动 |
| `createFolder`（wsService.ts） | 新增 | 镜像 `createDoc` |
| `ContextMenu`（shell） | 无变更 | 复用为下拉（items 传入格式列表） |
| `FileTreeNode` props | 新增 `activeDir` / `onActivateDir` | 目录行激活语义 |

安全边界不变量（不得破坏）：新建目标路径必须落在活动 workspace root 内（X-FORBID-004，
后端 `resolve_active` 守界，前端只组装 relPath 不校验路径合法性）；不新增任何越界/绕过路径。

## 7. 治理要求（实施时执行）

1. **task_type / 阶段**：`CODE_CHANGE`（修改业务代码）；governance_phase = IMPLEMENTATION。
   **须先执行 Issue Trace（ADU §6 十一项）→ 人类确认 Scope Lock** 后才可改码。
2. **规则/链路**：全部复用既有 `BR-WS-DATA-001` + 链路 `WS-FS`（active, P1）；
   **不新增 RULE / CHAIN / TERM / MODULE**，不改协议文档，故无需 `governance:generate`。
3. **@GOV**：FileTree/wsService 均为既有未逐函数标注的组件/薄服务层，沿用现状标注风格；
   不新增 Rust @GOV 块（后端零改动）。激活区域为 UI 视图态（同 `openPath` 层级），
   不引入新业务真源（对齐 X-FORBID-007 视图层导航态定位）。
4. **测试治理**：WS 模块 risk=critical；前端测试文件数冻结在 baseline=2，
   **新增用例并入既有文件，不新增测试文件**；新增断言带 `// covers: BR-WS-DATA-001`。
5. **范围边界**：目录右键「在此新建文件」、目录删除（WS-DIR-DELETE-FIX-PLAN-01）、
   「＋」自定义格式能力均保持各自既有/既定行为，本方案不互相扩散。

## 8. 风险与待确认项

| # | 事项 | 说明 | 决策点 |
|---|---|---|---|
| 1 | **活动 Scope 冲突（实施前置）** | `.governance/active_scope.json` 当前为 MODEL-CONFIG-01（ACTIVE, confirmation=YES，分支 feat/sm-state-machine-runtime 有未提交改动）。本次涉及文件（FileTree/FileTreeNode/wsService/styles/测试）均不在其 allowed_files，直接改码触发 SC-01 | 人类决定：先收尾/提交 MODEL-CONFIG-01 再立项，还是直接替换 Scope Lock（建议先收尾） |
| 2 | 格式清单范围 | 建议 6 固定项 + 自定义；是否含 xlsx/pptx 等预览格式（建后可预览但不可编辑） | 确认清单（默认不含预览格式，保持"可编辑优先"） |
| 3 | 激活区域语义 | 目录行点击即设为激活目录（展开/折叠 + 高亮）；打开文件不改变激活目录 | 确认该语义（或改为仅选中不联动展开） |
| 4 | 命名自动补扩展名 | 固定格式输入"笔记"自动补".md"；用户显式输入"a.md"则不重复追加 | 确认（默认是） |
| 5 | 与目录删除方案关系 | 两案独立成期；实施顺序可并行（不同文件集：本案不动 lib.rs/fileops.rs） | 确认独立排期 |

## 9. 验证计划

- 前端：`npx vitest run src/components/ws/WorkspaceUI.test.tsx src/services/workspaceRuntime.test.ts`。
- 变更影响：`npm run test:plan -- --files [changed_files]` → `npm run test:related -- --files [changed_files]`
  （WS critical，前端-only 变更不触发 rust 回归）。
- 治理：`npm run governance:audit`（无 error 级违规）。
- 静态：`npx tsc --noEmit`；`npm run build`。
- 人工验证（UI）：＋ → 下拉（7 项）→ 根目录/子目录新建各格式 → 文件夹新建空目录 →
  文件打开分流（md 编辑 / csv 预览 / docx BDocx 编辑）→ 激活区域 chip 与回根目录。
- 每阶段以实际 changed_files 计算，不得手选测试。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-08-06 | v0.1 | 初版：「＋」新建按钮下拉方案（格式清单 + 激活区域 + 文件夹入口 + 打开分流；后端零改动，复用 ContextMenu/ws.create_doc/ws.create_folder） |
