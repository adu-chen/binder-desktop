# Issue Trace — AG-1.0-IMP-MEMORY-SELECTION-01：显式记忆选择（MemorySelectionRef）实现

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-14

## 1. 用户可见症状

冻结基线 R5-C（BR-SYS-DATA-007 / TERM-AST-002）要求：仅用户显式选择的 workspace 本地项目记忆条目，
经 MemorySelectionRef（data-only）进入当前回合；1.0 不自动写入、不提议写入、不自动检索或注入；跨会话
不串选择事实。当前实现：REF 侧 memory 引用**注入链已通**（ReferencePayload.memory 变体 + AG 格式化
"引用记忆「{title}」：\n{content}" 已实现），但**供给端完全缺失**——AST 无按 ID/带内容读取 API、
无任何 ast.* ipc 暴露、前端无记忆面板 UI（Workbench 占位"记忆库"），用户无途径产生 memory 引用。

## 2. 入口点

- `src-tauri/src/ast/store.rs:49-59`（list_for_management 仅 (entry_id, kind, status)，无 content；无按 ID 读取）
- `src-tauri/src/lib.rs`（无任何 ast.* ipc 路由注册，AST 对前端不可达）
- `src/shell/Workbench.tsx:72-75`（activePanel=memory 渲染占位 span）
- `src/components/shell/LeftRail.tsx:25`（"记忆库"面板项已存在）
- `src/services/referencePayload.ts:50`（memory 变体已存在，无工厂/注入总线）
- 注入基座：`src/components/ag/Composer.tsx` insertReferencePayload（内部函数，需跨组件总线）

## 3. 相关规则

- `BR-SYS-DATA-007`（外部内容/记忆隔离：仅显式选择的记忆条目进入回合、不自动写入/检索/注入、跨会话不串；
  D1 注册；D3 契约 A-AST-M-T-01 §4.1）
- `BR-AST-DATA-001`（记忆条目 workspace 级隔离、纯本地不上云）
- `TERM-AST-002`（MemorySelectionRef，D3 注册）

## 4. 相关链路

AST-MEMORY（条目读取）、REF-INJECT（memory 引用注入，已通）、AG-TURN（回合消费）。

## 5. 生产者块

- `ast/store.rs` 既有 @GOV（BR-AST-DATA-001-QUERY-*）——本实现新增带内容列举查询。
- `lib.rs` 新 ast.* 路由（route 分发模式沿用既有 dispatch 信封）。

## 6. 消费者块

- MemoryPanel（记忆库面板，浏览/选择）；Composer 输入区（引用 token）；AG prompt 注入（memory 分支已通）。

## 7. 路径上的空规则块

本 Scope 改动文件均有 @GOV 标注基础；新增查询/路由/面板补 BR-SYS-DATA-007 / TERM-AST-002 引用与 covers。

## 8. boundary 假设

- 选择即 data-only：MemoryPanel 选择时构造 `{ kind: "memory", entryId, title, content }` 快照注入当前
  回合；不写回 AST、不触发自动供给/自动写入（1.0 关闭，memory_link 保持未接线）。
- 作用域隔离：列举/读取恒以 workspace_id 作用域（BR-AST-DATA-001），跨 workspace 不可见；纯本地不上云。
- 跨会话不串：引用随当前回合 payload 消逝（mention_block 一次性），不持久化选择事实。
- 浏览展示 active 条目（revoked 不展示/不可选择）。

## 9. 运行时假设

- 注入路径：MemoryPanel → 轻量引用注入总线（模块级回调）→ Composer insertReferencePayload；不新增全局
  store 槽（避免扩大前端状态面）。
- 面板数据经 ast.list_entries（dispatch 信封）异步加载，失败降级为空列表 + 提示。

## 10. 疑似不变量破坏

- BR-AST-DATA-001 隔离不破坏（新查询同样 workspace 作用域）。
- 自动写入/自动供给保持关闭（不触碰 memory_link / supply 生产接线）。
- REF 注入链不改变（仅新增供给端）。

## 11. 验证计划

- 补测试：store.rs 内联（列举带 content + workspace 隔离）、referencePayload（makeMemoryReference）、
  MemoryPanel 组件测试（可选）。
- 运行 `npm run test:plan -- --files [changed_files]` 审阅影响，再 `npm run test:related`；
  `npx tsc --noEmit`；`cargo test`；`npm run governance:audit`（维持全绿）。
- 同步 `scripts/test_governance.test.mjs`（测试计数基线，如需）；AST-M-T-01 变更记录。

## 12. 文件范围结论（Scope Lock 输入）

允许修改：
- `src-tauri/src/ast/store.rs`
- `src-tauri/src/lib.rs`
- `src/services/referencePayload.ts`
- `src/services/referencePayload.test.ts`
- `src/components/ws/MemoryPanel.tsx`（新建）
- `src/components/ag/Composer.tsx`（注册注入总线回调）
- `src/shell/Workbench.tsx`（memory 面板渲染）
- `scripts/test_governance.test.mjs`
- `docs/01_modules/AST/A-AST-M-T-01_记忆协同技术设计.md`（变更记录注明落地）
- `.governance/issues/AG-1.0-IMP-MEMORY-SELECTION-01.md`、`.governance/active_scope.json`

禁止修改：工作区其他未提交改动；其余 src/src-tauri 文件；ag/memory_link.rs（自动写入保持未接线）；
ag/context.rs（用户既有改动）；其他模块文档；治理控制文档；governance.config.json。

## 13. 人类确认记录

2026-08-14：人类经 AskUserQuestion 确认本 Scope（显式记忆选择）作为 BR-SYS-DATA-007 的第二实现环节；
外部内容 data-only（第一环节）已完成。confirmation: YES。

## 14. Task Runtime Report

- task_type：`CODE_CHANGE`
- governance_phase：`IMPLEMENTATION`
- final_state：`READY`
- issue_trace：本文件
- changed_files：
  - `src-tauri/src/ast/store.rs`（新增 list_manageable：workspace 作用域 active 条目含 content 供显式选择；
    @GOV BR-SYS-DATA-007-QUERY-AST-AST-MEMORY-003 + TERM-AST-002；新测试）
  - `src-tauri/src/lib.rs`（注册 ast.list_entries 路由 + route_ast_list_entries；
    @GOV BR-SYS-DATA-007-API-AST-AST-MEMORY-001）
  - `src/services/referencePayload.ts`（makeMemoryReference + 引用注入总线 register/requestReferenceInsert）
  - `src/components/ws/MemoryPanel.tsx`（新建记忆库面板：浏览 active 条目、显式选择注入）
  - `src/components/ag/Composer.tsx`（挂载时注册注入总线回调）
  - `src/shell/Workbench.tsx`（memory 面板渲染替换占位）
  - `src/services/referencePayload.test.ts`（memory 引用构造 + 注入总线用例）
  - `docs/01_modules/AST/A-AST-M-T-01_记忆协同技术设计.md`（§4.1 供给端落地变更记录 v1.3）
  - `docs/00_core/A-CORE-C-R-01_ADUS.md`（generate 更新：@GOV 254→256、测试用例 906→908）
  - 本 Issue Trace、`.governance/active_scope.json`
- mapping_chain：BR-SYS-DATA-007 记忆半 → ast/store.rs @GOV + covers + lib.rs 路由 @GOV + 前端
  referencePayload covers；**audit 维持全绿**（阶段一规则缺口全部闭环）。
- test_impact：
  - changed_files：store.rs、lib.rs、referencePayload.ts/.test.ts、MemoryPanel.tsx、Composer.tsx、Workbench.tsx
  - direct_modules：AST、REF、AG、SYS
  - selection：test:plan 判定前端全量 + tsc + cargo 全量 + audit
- validation：
  - `npm run test:related`（npm test 前端全量） # passed：26 files / 574 tests（referencePayload 14 过）
  - `npx tsc --noEmit` # passed
  - `cargo test`（Rust 全量） # passed：334 tests（ast::store 3 过，含 list_manageable）
  - `npm run governance:generate` # passed（@GOV 254→256；测试用例 906→908）
  - `npm run governance:audit` # **passed（exit 0）**
  - `git diff --check` # passed
- automated_tests：前端 574 + Rust 334 全过。
- stop_conditions：无。
- unresolved_items：记忆条目管理面（增删改/撤销 UI）为后续 AST 管理 PI；面板分页/搜索为可选项；
  D-NEW（未保存新文档保存才建文件）为下一实现候选。
