# Issue Trace：ED-FONT-LOCAL-LIBRARY-01

## 目标

编辑器工具栏字体组从静态 5 字体升级为「本地字体库同步」：新增 `fonts.list_fonts` command（macOS CoreText 枚举全部已注册字体族名，只读）；前端 fontService 懒加载 + 缓存；字体下拉 = 常用子集（Arial/Helvetica/Georgia/Times New Roman/Courier New）置顶（仅可用时展示）+ 其余本地可用字体追加；`document.fonts.check` 过滤不可用字体（仅展示可用）；写入前统一 CSS 引号规范化（修复多词字体名无引号拆词隐患）；枚举失败回退硬编码 5 字体、空列表回退常用子集（不阻塞编辑）。

## 规则与链路

- 模块：ED（工具栏）+ SYS（IPC command 登记）
- 链路：ED-EDIT（字体应用）；IPC-CALL（command 信封）
- 复用规则：X-SEQ-002（IPC 信封）、BR-SYS-DATA-003（command 注册校验）、BR-PRV-DATA-002（失败态明确不静默的精神，回退不阻塞主链）
- 不新增规则、链路、约束或术语。

## 设计一致性

- 字体枚举为只读系统能力：仅取字体族名，不读字体文件内容、不解析字形；不触达 workspace（X-FORBID-004 不涉及）。
- 常用子集为固定优先项（排序置顶），本地字体按字母序追加；不可用字体不渲染（document.fonts.check）。
- 写入规范化：单词名直写、多词名加单引号，修复 `font-family: Times New Roman` 无引号被按词拆分的隐患。
- BDocx 往返口径不变：本地字体同步只增强编辑态所见即所得，字体仍为 best_effort（A-ED-X-R-03 §4.1）。
- 失败/空列表回退不阻塞编辑；枚举缓存 module 级，下拉重复打开不重复枚举。

## 文件范围

- `src-tauri/Cargo.toml`（新增 core-text / core-foundation crate）
- `src-tauri/src/lib.rs`（注册 + 路由 `fonts.list_fonts`）
- `src-tauri/src/fonts/mod.rs`（新：CoreText 枚举只读 command）
- `src/services/fontService.ts`（新：listLocalFonts + module 缓存）
- `src/services/fontService.test.ts`（新：command 契约/失败/缓存）
- `src/components/ed/EditorToolbar.tsx`（字体组改造：常用子集+本地追加+过滤+引号规范化）
- `src/components/ed/EditorControls.test.tsx`（字体组测试）
- `docs/01_modules/SYS/A-SYS-I-P-02_IPC协议.md`（command 登记）
- `docs/01_modules/ED/A-ED-X-R-03_编辑器工具栏功能与兼容性清单.md`（字体行更新）
- `docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md`（变更记录）
- `.governance/issues/ED-FONT-LOCAL-LIBRARY-01.md`（本文）

## 验收

- `fonts.list_fonts` 返回 macOS 已注册字体族名数组（只读，不读文件内容）。
- 字体下拉：常用子集置顶（仅可用展示）+ 本地字体追加；不可用字体不渲染；选择后写入带引号 CSS。
- 枚举失败/空列表回退不阻塞编辑。
- 治理审计、类型检查、相关测试、构建通过。

## Task Runtime Report — 本地字体库同步

task_type: CODE_CHANGE
governance_phase: IMPLEMENTATION
final_state: READY
issue_trace: ED-FONT-LOCAL-LIBRARY-01

changed_files:

- `src-tauri/Cargo.toml`：+core-text / +core-foundation。
- `src-tauri/src/lib.rs`：注册 + 路由 fonts.list_fonts。
- `src-tauri/src/fonts/mod.rs`：CoreText 枚举（CTFontManagerCopyAvailableFontFamilyNames）。
- `src/services/fontService.ts`：listLocalFonts + module 缓存。
- `src/services/fontService.test.ts`：契约/失败/缓存。
- `src/components/ed/EditorToolbar.tsx`：字体组（常用子集+本地追加+可用过滤+引号规范化）。
- `src/components/ed/EditorControls.test.tsx`：字体组测试。
- `docs/01_modules/SYS/A-SYS-I-P-02_IPC协议.md`：command 登记。
- `docs/01_modules/ED/A-ED-X-R-03_编辑器工具栏功能与兼容性清单.md`：字体行更新。
- `docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md`：变更记录。

test_impact:

- direct: ED、SYS
- affected: PRV（无关，仅引用样式不回归）

validation:

- `cargo test --lib fonts`：passed（macOS 枚举非空）。
- `npx vitest run src/services/fontService.test.ts src/components/ed/EditorControls.test.tsx`：passed（17/17）。
- `npm run test:related`：passed（18 文件 / 421 测试）。
- `npm run governance:generate`：passed（测试资产 87 / 用例 759）。
- `npm run governance:audit`：passed（governance_findings 清零）。
- `npx tsc --noEmit`、`npm run build`、`git diff --check`：passed。

unresolved_items:

- 字体可用性过滤依赖 WKWebView 的 document.fonts.check；极少数仅打印字体（display-only）可能被 WebView 判不可用而隐藏——可接受，后续如需再开白名单。
- 跨平台（Windows/Linux）字体枚举未纳入本 Scope（仅 macOS CoreText）。
- test_governance 自测 ED 计数断言 207 vs 208 为既有失败（源自历史未提交 EditorControls 用例），非本 Scope 引入。
