# Issue Trace — LIB-MODULE-OWNERSHIP-01：Tauri 组合根模块职责分发

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-25
上游基线：A-SYS-C-T-01 模块分层、各业务模块现行技术设计、ADUS 模块编码表，以及 2026-08-25 人类明确提出的“继续治理 lib 文件，把相应模块代码全部分发到各自模块，缺失脚手架则新建”要求。

## 1. 用户可见症状

`src-tauri/src/lib.rs` 已增长至约 1.05 万行，除应用启动和 Tauri 命令装配外，还直接实现 WS、DOC、PRV、DE、ED、AG、REF、AST、ENT 的路由、运行时辅助函数和模块测试。入口文件承担了多个业务 owner 的实现职责；ED 的后端会话路由还没有独立模块脚手架。

## 2. 入口点

- `src-tauri/src/lib.rs`：应用组合根、command registry、dispatch、Tauri invoke handler，以及当前混置的业务实现。
- `src-tauri/src/{workspace,doc,prv,de,ag,reference,ast,ent,fonts}/mod.rs`：现有模块入口。
- `src-tauri/src/ipc/**`：IPC 信封与通用参数解析。
- `.governance/test-governance.json`：模块源码 owner 与测试选择映射。

## 3. 相关规则

- `X-FORBID-003`：SYS 底座不得反向依赖业务模块；跨业务路由分派只能停留在应用组合根。
- `BR-SYS-DATA-003`：command registry 与 dispatch 的未知命令拒绝边界。
- 各模块现有 `@GOV` 块所引用规则保持原样，不因文件迁移改变 owner 或语义。

本次不新增业务规则，不修改控制文档或模块技术设计。

## 4. 相关链路

`IPC-CALL` 是组合入口；随后按命令分别进入 `WS-*`、`DOC-CONVERT`、`PRV-VIEW`、`DE-DIFF`、`ED-EDIT`、`AG-*`、`REF-INJECT`、`AST-MEMORY` 和 ENT 网关链路。

## 5. 生产者块

- `build_registry` 与 `dispatch`：只负责注册、解析信封和把请求交给 owner 模块。
- 各模块 `routes`/`runtime` 文件：产生领域响应、状态迁移和副作用结果。

## 6. 消费者块

- Tauri invoke handler：消费组合根暴露的 commands。
- 前端各模块 service：消费原 command 名和响应信封；协议保持不变。
- Rust 单元/系统测试：迁入对应 owner 文件或 IPC 系统测试脚手架。

## 7. 路径上的空规则块

无新增空规则块。迁移保留既有 `@GOV` 块；ADUS 由生成器重新定位源码路径。

## 8. boundary 假设

- `lib.rs` 只保留模块声明、应用共享状态的组合、command registry/dispatch、平台启动守卫、Tauri builder 和 invoke 注册。
- 中央 dispatch 属于应用组合根，不下沉 `ipc`，避免 SYS 反向依赖业务模块。
- WS/DOC/PRV/DE/AG/REF/AST/ENT 实现归各自模块；编辑器会话与开放标签的后端路由归新增 `ed` 脚手架。
- command 名、请求参数、响应 JSON、数据库行为、事件名与状态迁移保持行为等价。

## 9. 运行时假设

- 模块迁移不改变 Tauri managed state 的唯一实例与锁粒度。
- 业务模块可依赖组合根传入的 `AppState`，但 SYS/IPC 不新增对业务模块的依赖。
- 现有未提交修复和 Prompt Runtime 拆分均作为当前真实基线保留。

## 10. 疑似不变量破坏

- 机械移动遗漏 command 注册或 invoke handler 会造成运行时命令失联。
- 错误拆分共享辅助函数可能改变锁生命周期、事件时序或跨模块状态机衔接。
- 将 dispatch 下沉 SYS 会违反无环依赖约束。
- 迁移测试时减少测试数量或弱化断言会掩盖行为回归。

## 11. 验证计划

- 分阶段执行 Rust 编译与模块定向测试，最后执行选择器要求的完整相关测试。
- 对比迁移前后 registry 命令集合、Tauri invoke command 集合与 Rust 测试清单。
- 运行 `cargo fmt --check`、`test:plan`、同输入 `test:related`、`test:governance`、`governance:generate`、`governance:audit` 与 `git diff --check`。
- 已知 LibreOffice Worker `signal=6` 继续作为独立环境证据报告，不把失败改写为通过。

## Scope Lock

```yaml
task_type: CODE_CHANGE
allowed_files:
  - src-tauri/src/lib.rs
  - src-tauri/src/ipc/**
  - src-tauri/src/workspace/**
  - src-tauri/src/doc/**
  - src-tauri/src/prv/**
  - src-tauri/src/fonts/**
  - src-tauri/src/de/**
  - src-tauri/src/ed/**
  - src-tauri/src/ag/**
  - src-tauri/src/reference/**
  - src-tauri/src/ast/**
  - src-tauri/src/ent/**
  - .governance/test-governance.json
  - scripts/split_lib_modules.mjs
  - docs/00_core/A-CORE-C-R-01_ADUS.md
  - .governance/issues/LIB-MODULE-OWNERSHIP-01.md
  - .governance/active_scope.json
forbidden_files:
  - AGENTS.md
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - governance.config.json
  - docs/01_modules/**
  - src/**
  - all files not listed in allowed_files
confirmation: YES
scope_reason: 人类明确要求继续全面治理 lib.rs，将业务实现和测试分发到现有 owner 模块，并为缺失的 ED 后端建立脚手架
```

## 验收标准

- `lib.rs` 不再定义业务 route、AG 回合/编辑/辅助运行时或业务模块单元测试。
- 各 command 仍由原命令名进入同一逻辑；registry 与 invoke handler 无缺项。
- 新建 `ed` 后端模块并将编辑器会话/开放标签路由归入该模块；测试治理能够识别新路径。
- 迁移前后的测试用例不减少，行为断言不弱化。
- 相关测试与治理审计给出真实结果；任何环境阻断单独记录。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-25 | v1.0 | 人类明确调整并替换上一受阻任务，建立 lib.rs 全量模块职责分发 Scope。 |
| 2026-08-25 | v1.1 | 完成业务实现与测试迁移、ED 脚手架和 owner 映射；定向/集成/前端/治理验证通过，Rust 全量仍被既有 LibreOffice Worker signal=6 阻断。 |
| 2026-08-25 | v1.2 | 继续将 AG 二级单体按 AG-TURN、AG-TOOL、TaskRun、history、reference、AG-ASSIST 拆分；补齐主回合、Diff 出口及 AG→WS-OPERATION 块级映射。 |
| 2026-08-26 | v1.3 | 深拆 turn/edit/assist runtime，归还 DE 投影、DOC 媒体授权与 ENT 默认模型 owner，补齐新执行块映射；全量前端通过，Rust 全量被 macOS 进程资源耗尽阻断。 |

## 实施结果

- `lib.rs` 从约 1.05 万行收敛到约 525 行，只保留模块声明、`AppState` 组合、72 个 command 的 registry/dispatch、Tauri builder、invoke 注册和平台退出守卫。
- WS、DOC、PRV、Fonts、DE、AG、REF、AST、ENT 的路由/运行时迁入各自模块；编辑器会话快照与开放标签迁入新建 `ed` 后端脚手架。
- 原 `lib.rs` 内联测试迁入 AG、WS、DE、REF 或 IPC 系统测试 owner；Rust 单元测试总数保持 355，不减少用例、不弱化断言。
- `.governance/test-governance.json` 登记 `src-tauri/src/ed/**`，测试治理确认无未映射源码、无无 owner 测试。
- registry 与 dispatch 静态集合核验为 72/72，missing=[]、extra=[]。
- ADUS 已重新生成，业务 `@GOV` 源码定位转向各模块；`lib.rs` 仅保留 IPC/APP-CLOSE 组合根块。
- 原 5,739 行 `ag/runtime.rs` 收敛为显式兼容门面；实现分发到 `turn_runtime.rs`、`edit_runtime.rs`、
  `task_runtime.rs`、`history_routes.rs`、`reference_runtime.rs`、`assist_runtime.rs`，各实现文件使用显式依赖。
- 新增块级映射覆盖 `ag_start_turn`、`ag_propose_edit`、取消/Assist 终态、历史 CRUD 与 TaskRun 路由；
  `route_resolve_decision` 已显式登记 `AG-TURN / AG-TOOL / WS-OPERATION` 跨链边界。
- `turn_runtime.rs` 进一步收敛为回合启动编排；活动回合控制、可见事件、引用目标、Prompt 装配与模型/工具循环分别归入
  `turn_control.rs`、`turn_events.rs`、`turn_reference.rs`、`turn_prompt.rs`、`turn_tool_loop.rs`。
- `edit_runtime.rs` 进一步收敛为修改建议校验与持久化编排；公共校验、Diff 投影、图片操作、表格操作、数据类型和测试分别归入
  `edit_support.rs`、`edit_projection.rs`、`edit_image_runtime.rs`、`edit_table_runtime.rs`、`edit_types.rs`、`edit_runtime_tests.rs`。
- Assist 的纯 Prompt 构造归入 `assist_prompt.rs`；`AgentDiffProjection` 归还 DE，媒体授权归还 DOC，默认模型选择归还 ENT；
  `ag/runtime.rs` 仅保留命令门面，不再承载跨 owner 辅助函数。
- 新增 5 组关键块级映射，覆盖 TableOperation 前置 guard、引用目标归一与锚校验、PromptPackage 装配、工具循环/TaskRun
  结算、Assist 类型化 Prompt；人工语义复核均与权威规则一致。

## Task Runtime Report

```yaml
status: BLOCKED
changed_files:
  - src-tauri/src/lib.rs
  - src-tauri/src/ipc/mod.rs
  - src-tauri/src/ipc/system_tests.rs
  - src-tauri/src/workspace/mod.rs
  - src-tauri/src/workspace/routes.rs
  - src-tauri/src/prv/mod.rs
  - src-tauri/src/prv/routes.rs
  - src-tauri/src/fonts/mod.rs
  - src-tauri/src/fonts/routes.rs
  - src-tauri/src/doc/mod.rs
  - src-tauri/src/doc/routes.rs
  - src-tauri/src/de/mod.rs
  - src-tauri/src/de/routes.rs
  - src-tauri/src/ed/mod.rs
  - src-tauri/src/ed/routes.rs
  - src-tauri/src/ag/mod.rs
  - src-tauri/src/ag/runtime.rs
  - src-tauri/src/ag/turn_runtime.rs
  - src-tauri/src/ag/turn_control.rs
  - src-tauri/src/ag/turn_events.rs
  - src-tauri/src/ag/turn_reference.rs
  - src-tauri/src/ag/turn_prompt.rs
  - src-tauri/src/ag/turn_tool_loop.rs
  - src-tauri/src/ag/edit_runtime.rs
  - src-tauri/src/ag/edit_support.rs
  - src-tauri/src/ag/edit_projection.rs
  - src-tauri/src/ag/edit_image_runtime.rs
  - src-tauri/src/ag/edit_table_runtime.rs
  - src-tauri/src/ag/edit_types.rs
  - src-tauri/src/ag/edit_runtime_tests.rs
  - src-tauri/src/ag/task_runtime.rs
  - src-tauri/src/ag/history_routes.rs
  - src-tauri/src/ag/reference_runtime.rs
  - src-tauri/src/ag/assist_runtime.rs
  - src-tauri/src/ag/assist_prompt.rs
  - src-tauri/src/reference/mod.rs
  - src-tauri/src/reference/routes.rs
  - src-tauri/src/ast/mod.rs
  - src-tauri/src/ast/routes.rs
  - src-tauri/src/ent/mod.rs
  - src-tauri/src/ent/commands.rs
  - src-tauri/src/ent/gateway.rs
  - src-tauri/src/de/projection.rs
  - src-tauri/src/doc/image_media.rs
  - .governance/test-governance.json
  - docs/00_core/A-CORE-C-R-01_ADUS.md
  - .governance/issues/LIB-MODULE-OWNERSHIP-01.md
  - .governance/active_scope.json
validation:
  rust_compile:
    cargo_check: passed
    lib_test_build: passed (355 tests compiled)
  targeted_rust:
    ag_module: 145 passed, 0 failed
    module_routes: 7 passed, 0 failed
    ipc_system_tests: 5 passed, 0 failed
  rust_integration:
    core_chain: 5 passed, 0 failed
    sm_broadcast: 4 passed, 0 failed
  test_governance:
    result: 19 passed, 0 failed
    inventory: tests_discovered=939, unmapped=0, unowned=0, focused_or_skipped=0
  frontend_full: 589 passed, 0 failed
  rust_full:
    result: blocked before test execution
    latest_failure: xcrun/cc process spawn failed — Resource temporarily unavailable (os error 35)
    attempts: 3 (including CARGO_BUILD_JOBS=1 retry)
    prior_scope_evidence: 354 passed, 1 failed — doc::libreoffice_runtime::tests::docx_to_pdf_end_to_end_when_libreoffice_available, DOC_PREVIEW_WORKER_CRASH signal=6
  registry_dispatch_equivalence: registered=72, routed=72, missing=[], extra=[]
  governance_generate: passed
  governance_audit: passed
  semantic_review:
    result: passed
    reviewed_blocks: AG-TURN-018~031, AG-TOOL-017~019, WS-OPERATION-005, AG-ASSIST-010~011, BR-ED-DATA-007-GUARD-AG-AG-TOOL-018
    conclusion: boundaries 与注册规则的实体、状态和副作用语义一致；无 BOUNDARY_FALSE 或 MAPPING_DECORATIVE
  scoped_rustfmt_check: passed
  git_diff_check: passed
unresolved_items:
  - test:related 要求 Rust 全量通过；本轮连续三次在 macOS xcrun/cc 进程创建阶段遭遇 os error 35，未进入断言执行，因此不得进入 READY。
  - 此前同一 Scope 的 DOC/LibreOffice Worker signal=6 失败仍是独立既有证据，本轮未获得可覆盖它的新全量结果。
  - 真实 Tauri 桌面运行中的全部命令逐项点击未做人工验证；自动化覆盖 registry、编译、模块测试和 IPC 系统测试。
```
