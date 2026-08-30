---
文档编号：CORE-GOV-01
文档状态：A
负责模块：CORE
文档职责：ADU 派生检索索引
上游约束：CORE-GOV-00
直接承接：ADU / ADP / CLAUDE 的规则、链路、代码块检索
使用边界：不定义治理规则，不定义开发过程，不替代 adu.md、ADP.md 或 CLAUDE.md
变更要求：运行 npm run governance:generate 刷新自动生成区
---

# ADU 派生检索索引

本文件由 `governance:generate` 自动生成。
所有索引内容来自设计文档标注块、源代码 @GOV 标注、测试覆盖注释和 triage 报告。
运行 `npm run governance:generate` 刷新。

边界：
- adu.md 是静态治理规则源。
- ADP.md 是开发过程运行时协议。
- CLAUDE.md 是 AI 会话入口与执行路由协议。
- adus.md 只提供派生检索索引，不定义规则、不定义过程、不承载人类裁决。

自动生成边界：
- `## 【自动生成区】` 之前仅保留元信息头部，包括文档头、边界说明和变更记录。
- 元信息头部不得写入规则、链路、模块、约束、裁决或索引内容。
- `## 【自动生成区】` 及之后内容禁止人工编辑，应由 `npm run governance:generate` 刷新。

---

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-09 | v1.0 | 初始版本 |

---

## 【自动生成区】

<!-- 自动生成。禁止人工编辑。-->
<!-- 运行：npm run governance:generate -->
<!-- 最后生成时间：2026-08-29T15:22:56.592Z -->

### 模块编码表

| module_code | module_name | description                                                              | 来源文档                                       |
| ----------- | ----------- | ------------------------------------------------------------------------ | ------------------------------------------ |
| PCG         | 产品能力图谱      | 将产品能力、格式/角色变体、验收点、治理引用和验证证据组织为单一注册表，并生成产品沙盘和审计结果                         | docs/00_core/A-CORE-C-P-02_产品能力图谱规范.md     |
| CORE        | 核心          | 项目核心与治理，跨模块产品级定义                                                         | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| SYS         | 系统底座        | 整体架构、IPC 协议、跨模块契约与模块编码基线                                                 | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| WS          | 工作台         | workspace 生命周期、文件 I/O、缓存、pending_diffs 持久化、文档级时间轴与历史回溯                   | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| ED          | 编辑器         | 编辑器运行时（TipTap，md/docx 富文本编辑）                                             | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| DE          | Diff Edit   | diff 生成、审阅、接受/拒绝/过期终态                                                    | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| AG          | 代理（Agent）   | 代理编辑（AgentEdit）面板：编辑/构建双模式回合、工具调用、辅助续写与局部修改                              | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| CBT         | 构建（Build）   | 项目蓝图与多文件文档项目构建、构建中断、构建移交/回落（旧称 Chat Build / Chat↔Build 接管，2026-07-04 更名） | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| DOC         | 文档转换        | DOCX/Pandoc 格式转换                                                         | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| PRV         | 预览输出        | Excel/PPT 预览、音视频预览与打印/导出（不含 Excel/PPT 编辑）                                | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| REF         | 引用          | 引用（@mention）解析与上下文注入                                                     | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| AST         | 记忆          | 本地记忆协同                                                                   | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| KB          | 知识库         | 本地知识库                                                                    | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| TPL         | 模板          | 模板与工作流                                                                   | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| ACCT        | 账号          | 账号、认证、会话（云端认证客户端）                                                        | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| ENT         | 订阅门控        | 订阅门控、用量与 AI 网关客户端                                                        | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |

### 规则域注册表

| domain_code | description      | 来源 |
| ----------- | ---------------- | -- |
| STATE       | 状态语义、生命周期、阶段闭合   | 内置 |
| DATA        | 数据结构、字段定义、schema | 内置 |
| PERSIST     | 持久化、恢复、重建        | 内置 |
| VERIFY      | 验证、审计、一致性检查      | 内置 |
| GOV         | 文档治理、命名、变更控制     | 内置 |
| OBS         | 可观测性、错误处理、恢复     | 内置 |

### 链路注册表

| chain_id         | 主责模块 | status | 集成测试                                                                                                          |
| ---------------- | ---- | ------ | ------------------------------------------------------------------------------------------------------------- |
| PCG-GENERATE     | PCG  | active | scripts/test_governance.test.mjs::生成可点击的产品故事地图和格式矩阵                                                           |
| PCG-AUDIT        | PCG  | active | scripts/test_governance.test.mjs::接受具有完整追溯、主链和独立状态的注册表                                                        |
| AG-TURN          | AG   | active | deferred: phase-5                                                                                             |
| AG-TOOL          | AG   | active | deferred: phase-5                                                                                             |
| AG-ASSIST        | AG   | active | deferred: phase-5                                                                                             |
| AG-MODEL-CATALOG | AG   | active | deferred: phase-5                                                                                             |
| AST-MEMORY       | AST  | active | deferred: phase-5                                                                                             |
| CBT-BUILD        | CBT  | active | deferred: phase-5                                                                                             |
| DE-DIFF          | DE   | active | src-tauri/tests/core_chain.rs::accepted_proposal_flows_through_de_to_ws_landing                               |
| DOC-CONVERT      | DOC  | active | deferred: phase-5                                                                                             |
| ED-EDIT          | ED   | active | deferred: phase-5                                                                                             |
| ED-DECOR         | ED   | active | deferred: phase-5                                                                                             |
| KB-RETRIEVE      | KB   | active | deferred: phase-5                                                                                             |
| PRV-VIEW         | PRV  | active | deferred: phase-5                                                                                             |
| REF-INJECT       | REF  | active | deferred: phase-5                                                                                             |
| DATA-MIGRATE     | SYS  | active | deferred: phase-5                                                                                             |
| IPC-CALL         | SYS  | active | src-tauri/src/ipc/system_tests.rs::accepted_draft_saves_to_disk_and_reopens_through_tauri_ipc                 |
| CLOUD-AUTH       | SYS  | active | deferred: phase-5                                                                                             |
| CLOUD-GATE       | SYS  | active | deferred: phase-5                                                                                             |
| CLOUD-INVOKE     | SYS  | active | deferred: phase-5                                                                                             |
| APP-CLOSE        | SYS  | active | deferred: phase-5                                                                                             |
| TPL-FLOW         | TPL  | active | deferred: phase-5                                                                                             |
| WS-FS            | WS   | active | deferred: phase-5                                                                                             |
| WS-DIFF-APPLY    | WS   | active | src-tauri/tests/core_chain.rs::accepted_proposal_flows_through_de_to_ws_landing                               |
| WS-TIMELINE      | WS   | active | deferred: phase-5                                                                                             |
| WS-SAVE          | WS   | active | src-tauri/src/ipc/system_tests.rs::accepted_draft_saves_to_disk_and_reopens_through_tauri_ipc                 |
| WS-OPERATION     | WS   | active | src-tauri/src/workspace/operation.rs::plan_freezes_real_baseline_and_execute_requires_bound_resolved_decision |

### 规则注册表

| rule_id            | 域       | 主链路                                  | 来源文档                                              | registry_status | 测试覆盖                                                                                                                                                                                                                                                                                                                                                                        |
| ------------------ | ------- | ------------------------------------ | ------------------------------------------------- | --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| BR-PCG-DATA-001    | DATA    | PCG-GENERATE                         | docs/00_core/A-CORE-C-P-02_产品能力图谱规范.md            | registered      | scripts/test_governance.test.mjs::file-level                                                                                                                                                                                                                                                                                                                                |
| BR-PCG-STATE-001   | STATE   | PCG-GENERATE                         | docs/00_core/A-CORE-C-P-02_产品能力图谱规范.md            | registered      | scripts/test_governance.test.mjs::file-level                                                                                                                                                                                                                                                                                                                                |
| BR-PCG-VERIFY-001  | VERIFY  | PCG-AUDIT                            | docs/00_core/A-CORE-C-P-02_产品能力图谱规范.md            | registered      | scripts/test_governance.test.mjs::file-level                                                                                                                                                                                                                                                                                                                                |
| BR-PCG-VERIFY-002  | VERIFY  | PCG-AUDIT                            | docs/00_core/A-CORE-C-P-02_产品能力图谱规范.md            | registered      | scripts/test_governance.test.mjs::file-level                                                                                                                                                                                                                                                                                                                                |
| BR-CORE-STATE-001  | STATE   | DE-DIFF, ED-EDIT, WS-SAVE            | docs/00_core/A-CORE-C-T-01_三态模型.md                | registered      | src/services/deRuntime.test.ts::file-level, src/services/edRuntime.test.ts::file-level                                                                                                                                                                                                                                                                                      |
| BR-CORE-STATE-002  | STATE   | DE-DIFF, WS-DIFF-APPLY               | docs/00_core/A-CORE-C-T-01_三态模型.md                | registered      | src/services/deRuntime.test.ts::file-level                                                                                                                                                                                                                                                                                                                                  |
| BR-CORE-STATE-003  | STATE   | DE-DIFF, ED-EDIT, ED-DECOR, WS-SAVE  | docs/00_core/A-CORE-C-T-01_三态模型.md                | registered      | src-tauri/src/ipc/system_tests.rs::file-level, src/services/deRuntime.test.ts::file-level                                                                                                                                                                                                                                                                                   |
| BR-ACCT-STATE-001  | STATE   | CLOUD-AUTH                           | docs/01_modules/ACCT/A-ACCT-M-T-01_账号会话技术设计.md    | registered      | src-tauri/src/acct/session.rs::file-level                                                                                                                                                                                                                                                                                                                                   |
| BR-ACCT-DATA-001   | DATA    | CLOUD-AUTH                           | docs/01_modules/ACCT/A-ACCT-M-T-01_账号会话技术设计.md    | registered      | src-tauri/src/acct/token.rs::file-level                                                                                                                                                                                                                                                                                                                                     |
| BR-AG-STATE-001    | STATE   | AG-TURN                              | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | src-tauri/src/ag/turn.rs::file-level, src-tauri/src/ag/turn_control.rs::file-level                                                                                                                                                                                                                                                                                          |
| BR-AG-DATA-003     | DATA    | AG-TURN                              | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | src-tauri/src/ag/target_set.rs::file-level, src-tauri/src/ag/task.rs::file-level, src-tauri/src/ag/task_runtime.rs::file-level, src-tauri/src/ag/turn_control.rs::file-level, src-tauri/src/de/generate.rs::file-level                                                                                                                                                      |
| BR-AG-VERIFY-001   | VERIFY  | AG-TURN                              | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | src-tauri/src/ag/turn.rs::file-level                                                                                                                                                                                                                                                                                                                                        |
| BR-AG-PERSIST-001  | PERSIST | AG-TURN                              | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | src-tauri/src/ag/history.rs::file-level, src-tauri/src/ag/turn.rs::file-level, src/services/agRuntime.test.ts::file-level                                                                                                                                                                                                                                                   |
| BR-AG-UI-001       | UI      | AG-TURN                              | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | src/services/agRuntime.test.ts::file-level                                                                                                                                                                                                                                                                                                                                  |
| BR-AG-STATE-002    | STATE   | AG-TOOL                              | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | src-tauri/src/ag/tool.rs::file-level, src-tauri/src/ag/turn_tool_loop.rs::file-level                                                                                                                                                                                                                                                                                        |
| BR-AG-DATA-001     | DATA    | AG-TURN                              | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | src-tauri/src/ag/context.rs::file-level                                                                                                                                                                                                                                                                                                                                     |
| BR-AG-DATA-002     | DATA    | AG-TURN                              | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | src-tauri/src/ag/assist_prompt.rs::file-level, src-tauri/src/ag/block_ops.rs::file-level, src-tauri/src/ag/context.rs::file-level                                                                                                                                                                                                                                           |
| BR-AG-STATE-004    | STATE   | AG-TURN                              | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | src-tauri/src/ag/memory_link.rs::file-level                                                                                                                                                                                                                                                                                                                                 |
| BR-AG-STATE-003    | STATE   | AG-ASSIST                            | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | src-tauri/src/ag/assist.rs::file-level, src-tauri/src/ag/assist_prompt.rs::file-level, src/components/ag/AgentUI.test.tsx::file-level, src/services/agRuntime.test.ts::file-level, src/store/assistRuntime.test.ts::file-level                                                                                                                                              |
| BR-AG-DATA-006     | DATA    | AG-MODEL-CATALOG                     | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | src/store/assistRuntime.test.ts::file-level                                                                                                                                                                                                                                                                                                                                 |
| BR-AG-DATA-007     | DATA    | AG-MODEL-CATALOG                     | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | src/services/agRuntime.test.ts::file-level, src/store/assistRuntime.test.ts::file-level                                                                                                                                                                                                                                                                                     |
| BR-AG-DATA-004     | DATA    | AG-TURN                              | docs/01_modules/AG/A-AG-M-T-02_提示词架构设计.md         | registered      | src-tauri/src/ag/context.rs::file-level, src-tauri/src/ag/prompt_runtime.rs::file-level                                                                                                                                                                                                                                                                                     |
| BR-AG-DATA-005     | DATA    | AG-TURN                              | docs/01_modules/AG/A-AG-M-T-02_提示词架构设计.md         | registered      | src-tauri/src/ag/context.rs::file-level, src-tauri/src/ag/prompt_runtime.rs::file-level, src-tauri/src/ag/tool_exec.rs::file-level                                                                                                                                                                                                                                          |
| BR-AST-DATA-001    | DATA    | AST-MEMORY                           | docs/01_modules/AST/A-AST-M-T-01_记忆协同技术设计.md      | registered      | src-tauri/src/ast/store.rs::file-level                                                                                                                                                                                                                                                                                                                                      |
| BR-AST-STATE-001   | STATE   | AST-MEMORY                           | docs/01_modules/AST/A-AST-M-T-01_记忆协同技术设计.md      | registered      | src-tauri/src/ast/capture.rs::file-level                                                                                                                                                                                                                                                                                                                                    |
| BR-AST-DATA-002    | DATA    | AST-MEMORY                           | docs/01_modules/AST/A-AST-M-T-01_记忆协同技术设计.md      | registered      | src-tauri/src/ast/supply.rs::file-level                                                                                                                                                                                                                                                                                                                                     |
| BR-CBT-STATE-001   | STATE   | CBT-BUILD                            | docs/01_modules/CBT/A-CBT-M-T-01_构建技术设计.md        | registered      | src-tauri/src/cbt/build.rs::file-level                                                                                                                                                                                                                                                                                                                                      |
| BR-CBT-DATA-001    | DATA    | CBT-BUILD                            | docs/01_modules/CBT/A-CBT-M-T-01_构建技术设计.md        | registered      | src-tauri/src/cbt/apply.rs::file-level                                                                                                                                                                                                                                                                                                                                      |
| BR-CBT-PERSIST-001 | PERSIST | CBT-BUILD                            | docs/01_modules/CBT/A-CBT-M-T-01_构建技术设计.md        | registered      | src-tauri/src/cbt/build.rs::file-level                                                                                                                                                                                                                                                                                                                                      |
| BR-CBT-STATE-002   | STATE   | CBT-BUILD                            | docs/01_modules/CBT/A-CBT-M-T-01_构建技术设计.md        | registered      | src-tauri/src/cbt/takeover.rs::file-level                                                                                                                                                                                                                                                                                                                                   |
| BR-DE-DATA-001     | DATA    | DE-DIFF                              | docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md    | registered      | src-tauri/src/de/generate.rs::file-level, src/services/edOperations.test.ts::file-level                                                                                                                                                                                                                                                                                     |
| BR-DE-DATA-002     | DATA    | DE-DIFF                              | docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md    | registered      | src-tauri/src/de/generate.rs::file-level, src-tauri/src/de/routes.rs::file-level, src-tauri/src/de/store.rs::file-level, src-tauri/src/workspace/diff_apply.rs::file-level, src/components/de/DiffReview.test.tsx::file-level, src/components/ed/EditorCore.test.tsx::file-level, src/services/deRuntime.test.ts::file-level, src/services/edOperations.test.ts::file-level |
| BR-DE-STATE-001    | STATE   | DE-DIFF                              | docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md    | registered      | src-tauri/src/de/mod.rs::file-level, src-tauri/src/de/routes.rs::file-level, src-tauri/src/ipc/system_tests.rs::file-level, src/components/de/DiffReview.test.tsx::file-level, src/services/deRuntime.test.ts::file-level                                                                                                                                                   |
| BR-DE-STATE-002    | STATE   | DE-DIFF                              | docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md    | registered      | src-tauri/src/de/expiry.rs::file-level, src-tauri/src/de/lifecycle.rs::file-level                                                                                                                                                                                                                                                                                           |
| BR-DE-DATA-003     | DATA    | DE-DIFF                              | docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md    | registered      | src-tauri/src/de/expiry.rs::file-level, src-tauri/src/de/lifecycle.rs::file-level                                                                                                                                                                                                                                                                                           |
| BR-DOC-DATA-001    | DATA    | DOC-CONVERT                          | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md      | registered      | src-tauri/src/doc/convert.rs::file-level, src-tauri/src/doc/font_trace.rs::file-level, src-tauri/src/doc/libreoffice_runtime.rs::file-level, src-tauri/src/doc/pandoc_runtime.rs::file-level, src-tauri/src/doc/tool_runtime.rs::file-level                                                                                                                                 |
| BR-DOC-STATE-001   | STATE   | DOC-CONVERT                          | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md      | registered      | src-tauri/src/doc/convert.rs::file-level                                                                                                                                                                                                                                                                                                                                    |
| BR-DOC-DATA-002    | DATA    | DOC-CONVERT                          | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md      | registered      | src-tauri/src/doc/docx_cell_shading.rs::file-level, src-tauri/src/doc/edit_chain.rs::file-level, src-tauri/src/doc/pandoc_runtime.rs::file-level, src-tauri/src/doc/style_subset.rs::file-level                                                                                                                                                                             |
| BR-DOC-DATA-003    | DATA    | DOC-CONVERT                          | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md      | registered      | src-tauri/src/doc/block_backfill.rs::file-level                                                                                                                                                                                                                                                                                                                             |
| BR-ED-STATE-001    | STATE   | ED-EDIT                              | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         | registered      | src/components/ed/EditorCore.test.tsx::file-level, src/components/ed/extensions/EditorExtensions.test.ts::file-level, src/components/shell/ShellComponents.test.tsx::file-level, src/services/deRuntime.test.ts::file-level, src/services/edOperations.test.ts::file-level, src/services/workspaceRuntime.test.ts::file-level                                               |
| BR-ED-STATE-003    | STATE   | ED-EDIT                              | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         | registered      | src/services/edRuntime.test.ts::file-level                                                                                                                                                                                                                                                                                                                                  |
| BR-ED-DATA-001     | DATA    | ED-EDIT                              | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         | registered      | src/components/ed/EditorControls.test.tsx::file-level, src/components/ed/EditorCore.test.tsx::file-level, src/components/ed/extensions/EditorExtensions.test.ts::file-level, src/services/edOperations.test.ts::file-level                                                                                                                                                  |
| BR-ED-DATA-004     | DATA    | ED-EDIT                              | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         | registered      | src/services/edRuntime.test.ts::file-level                                                                                                                                                                                                                                                                                                                                  |
| BR-ED-DATA-002     | DATA    | ED-EDIT                              | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         | registered      | src/services/edRuntime.test.ts::file-level                                                                                                                                                                                                                                                                                                                                  |
| BR-ED-DATA-006     | DATA    | ED-EDIT                              | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         | registered      | src/components/ed/extensions/EditorExtensions.test.ts::file-level                                                                                                                                                                                                                                                                                                           |
| BR-ED-VERIFY-001   | VERIFY  | ED-EDIT                              | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         | registered      | src/components/ed/extensions/EditorExtensions.test.ts::file-level                                                                                                                                                                                                                                                                                                           |
| BR-ED-DATA-005     | DATA    | ED-EDIT                              | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         | registered      | src-tauri/src/ag/blocks.rs::file-level, src/services/edOperations.test.ts::file-level                                                                                                                                                                                                                                                                                       |
| BR-ED-DATA-007     | DATA    | ED-EDIT, AG-TOOL, DE-DIFF            | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         | registered      | src/components/de/DiffReview.test.tsx::file-level, src/components/ed/EditorControls.test.tsx::file-level, src/components/ed/extensions/EditorExtensions.test.ts::file-level, src/services/edOperations.test.ts::file-level                                                                                                                                                  |
| BR-ED-DATA-003     | DATA    | ED-EDIT                              | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         | registered      | src/components/ed/EditorControls.test.tsx::file-level, src/components/ed/EditorCore.test.tsx::file-level, src/components/ed/extensions/EditorExtensions.test.ts::file-level, src/components/ed/extensions/PaginationSuite.test.ts::file-level, src/services/docService.test.ts::file-level, src/services/edOperations.test.ts::file-level                                   |
| BR-ED-STATE-002    | STATE   | ED-DECOR                             | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         | registered      | src/components/ed/extensions/EditorExtensions.test.ts::file-level                                                                                                                                                                                                                                                                                                           |
| BR-ENT-VERIFY-001  | VERIFY  | CLOUD-GATE                           | docs/01_modules/ENT/A-ENT-M-T-01_订阅门控技术设计.md      | registered      | src-tauri/src/ent/gateway.rs::file-level                                                                                                                                                                                                                                                                                                                                    |
| BR-ENT-STATE-001   | STATE   | CLOUD-INVOKE                         | docs/01_modules/ENT/A-ENT-M-T-01_订阅门控技术设计.md      | registered      | src-tauri/src/ent/gateway.rs::file-level                                                                                                                                                                                                                                                                                                                                    |
| BR-ENT-DATA-001    | DATA    | CLOUD-INVOKE                         | docs/01_modules/ENT/A-ENT-M-T-01_订阅门控技术设计.md      | registered      | src-tauri/src/ent/byok.rs::file-level                                                                                                                                                                                                                                                                                                                                       |
| BR-ENT-DATA-002    | DATA    | CLOUD-GATE                           | docs/01_modules/ENT/A-ENT-M-T-01_订阅门控技术设计.md      | registered      | src-tauri/src/ent/usage.rs::file-level                                                                                                                                                                                                                                                                                                                                      |
| BR-KB-DATA-001     | DATA    | KB-RETRIEVE                          | docs/01_modules/KB/A-KB-M-T-01_知识库技术设计.md         | registered      | src-tauri/src/kb/store.rs::file-level                                                                                                                                                                                                                                                                                                                                       |
| BR-KB-DATA-002     | DATA    | KB-RETRIEVE                          | docs/01_modules/KB/A-KB-M-T-01_知识库技术设计.md         | registered      | src-tauri/src/kb/retrieve.rs::file-level                                                                                                                                                                                                                                                                                                                                    |
| BR-PRV-DATA-002    | DATA    | PRV-VIEW                             | docs/01_modules/PRV/A-PRV-M-T-01_预览输出技术设计.md      | registered      | src-tauri/src/doc/table_preview.rs::file-level, src-tauri/src/prv/mod.rs::file-level, src/services/prvRuntime.test.ts::file-level                                                                                                                                                                                                                                           |
| BR-PRV-DATA-001    | DATA    | PRV-VIEW                             | docs/01_modules/PRV/A-PRV-M-T-01_预览输出技术设计.md      | registered      | src/services/prvRuntime.test.ts::file-level                                                                                                                                                                                                                                                                                                                                 |
| BR-REF-DATA-001    | DATA    | REF-INJECT                           | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md        | registered      | src-tauri/src/reference/resolve.rs::file-level, src-tauri/src/reference/routes.rs::file-level                                                                                                                                                                                                                                                                               |
| BR-REF-DATA-002    | DATA    | REF-INJECT                           | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md        | registered      | src-tauri/src/reference/inject.rs::file-level, src-tauri/src/reference/routes.rs::file-level                                                                                                                                                                                                                                                                                |
| BR-REF-DATA-003    | DATA    | REF-INJECT                           | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md        | registered      | src-tauri/src/ag/reference_runtime.rs::file-level, src-tauri/src/reference/context.rs::file-level, src/components/ag/AgentUI.test.tsx::file-level, src/services/referencePayload.test.ts::file-level                                                                                                                                                                        |
| BR-REF-DATA-004    | DATA    | REF-INJECT                           | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md        | registered      | src/components/ag/AgentUI.test.tsx::file-level, src/services/referencePayload.test.ts::file-level                                                                                                                                                                                                                                                                           |
| BR-REF-VERIFY-001  | VERIFY  | REF-INJECT                           | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md        | registered      | src-tauri/src/reference/context.rs::file-level, src-tauri/src/reference/inject.rs::file-level, src-tauri/src/reference/routes.rs::file-level                                                                                                                                                                                                                                |
| BR-SYS-STATE-007   | STATE   | AG-TURN, IPC-CALL                    | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md        | registered      | src-tauri/src/ag/turn.rs::file-level, src-tauri/src/ag/turn_control.rs::file-level                                                                                                                                                                                                                                                                                          |
| BR-SYS-STATE-008   | STATE   | AG-TURN, DE-DIFF                     | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md        | registered      | src-tauri/src/ag/task.rs::file-level, src-tauri/src/ag/task_runtime.rs::file-level, src-tauri/src/ag/turn_control.rs::file-level, src/services/agRuntime.test.ts::file-level, src/services/deRuntime.test.ts::file-level                                                                                                                                                    |
| BR-SYS-DATA-007    | DATA    | REF-INJECT, AST-MEMORY               | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md        | registered      | src-tauri/src/ast/store.rs::file-level, src-tauri/src/reference/context.rs::file-level, src/services/referencePayload.test.ts::file-level                                                                                                                                                                                                                                   |
| BR-SYS-DATA-002    | DATA    | DATA-MIGRATE                         | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md        | registered      | src-tauri/src/types/mod.rs::file-level                                                                                                                                                                                                                                                                                                                                      |
| BR-SYS-PERSIST-001 | PERSIST | DATA-MIGRATE                         | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md        | registered      | src-tauri/src/ag/target_set.rs::file-level, src-tauri/src/db/mod.rs::file-level                                                                                                                                                                                                                                                                                             |
| BR-SYS-DATA-003    | DATA    | IPC-CALL                             | docs/01_modules/SYS/A-SYS-I-P-02_IPC协议.md         | registered      | src-tauri/src/ipc/mod.rs::file-level, src-tauri/src/ipc/system_tests.rs::file-level                                                                                                                                                                                                                                                                                         |
| BR-SYS-OBS-002     | OBS     | IPC-CALL                             | docs/01_modules/SYS/A-SYS-I-P-02_IPC协议.md         | registered      | src-tauri/src/ipc/mod.rs::file-level                                                                                                                                                                                                                                                                                                                                        |
| BR-SYS-STATE-002   | STATE   | IPC-CALL                             | docs/01_modules/SYS/A-SYS-I-P-02_IPC协议.md         | registered      | src-tauri/src/ipc/mod.rs::file-level                                                                                                                                                                                                                                                                                                                                        |
| BR-SYS-STATE-003   | STATE   | IPC-CALL                             | docs/01_modules/SYS/A-SYS-I-P-02_IPC协议.md         | registered      | src-tauri/src/sm_events.rs::file-level, src-tauri/tests/sm_broadcast.rs::file-level                                                                                                                                                                                                                                                                                         |
| BR-SYS-STATE-001   | STATE   | CLOUD-AUTH                           | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md         | registered      | src-tauri/src/session/mod.rs::file-level                                                                                                                                                                                                                                                                                                                                    |
| BR-SYS-VERIFY-001  | VERIFY  | CLOUD-GATE                           | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md         | registered      | src-tauri/src/session/mod.rs::file-level                                                                                                                                                                                                                                                                                                                                    |
| BR-SYS-DATA-001    | DATA    | CLOUD-INVOKE                         | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md         | registered      | src-tauri/src/session/mod.rs::file-level                                                                                                                                                                                                                                                                                                                                    |
| BR-SYS-DATA-004    | DATA    | CLOUD-INVOKE                         | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md         | registered      | MISSING                                                                                                                                                                                                                                                                                                                                                                     |
| BR-SYS-OBS-001     | OBS     | CLOUD-AUTH, CLOUD-GATE, CLOUD-INVOKE | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md         | registered      | src-tauri/src/ent/gateway.rs::file-level                                                                                                                                                                                                                                                                                                                                    |
| BR-SYS-DATA-006    | DATA    | AG-TOOL                              | docs/01_modules/SYS/A-SYS-I-P-04_BDocx代理编辑能力协议.md | registered      | src-tauri/src/ag/bdocx_capabilities.rs::file-level                                                                                                                                                                                                                                                                                                                          |
| BR-SYS-STATE-004   | STATE   | APP-CLOSE                            | docs/01_modules/SYS/A-SYS-M-T-02_应用关闭技术设计.md      | registered      | src/components/shell/ShellComponents.test.tsx::file-level                                                                                                                                                                                                                                                                                                                   |
| BR-SYS-STATE-006   | STATE   | APP-CLOSE                            | docs/01_modules/SYS/A-SYS-M-T-02_应用关闭技术设计.md      | registered      | src/services/systemRuntime.test.ts::file-level                                                                                                                                                                                                                                                                                                                              |
| BR-SYS-STATE-005   | STATE   | APP-CLOSE                            | docs/01_modules/SYS/A-SYS-M-T-02_应用关闭技术设计.md      | registered      | src-tauri/src/de/lifecycle.rs::file-level                                                                                                                                                                                                                                                                                                                                   |
| BR-SYS-DATA-005    | DATA    | APP-CLOSE                            | docs/01_modules/SYS/A-SYS-M-T-02_应用关闭技术设计.md      | registered      | src-tauri/src/db/mod.rs::file-level, src-tauri/src/de/lifecycle.rs::file-level                                                                                                                                                                                                                                                                                              |
| BR-TPL-DATA-001    | DATA    | TPL-FLOW                             | docs/01_modules/TPL/A-TPL-M-T-01_模板工作流技术设计.md     | registered      | src-tauri/src/tpl/template.rs::file-level                                                                                                                                                                                                                                                                                                                                   |
| BR-TPL-STATE-001   | STATE   | TPL-FLOW                             | docs/01_modules/TPL/A-TPL-M-T-01_模板工作流技术设计.md     | registered      | src-tauri/src/tpl/workflow.rs::file-level                                                                                                                                                                                                                                                                                                                                   |
| BR-WS-DATA-001     | DATA    | WS-FS                                | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md         | registered      | src-tauri/src/ipc/system_tests.rs::file-level, src-tauri/src/workspace/fileops.rs::file-level, src-tauri/src/workspace/listing.rs::file-level, src-tauri/src/workspace/mod.rs::file-level, src/services/workspaceRuntime.test.ts::file-level                                                                                                                                |
| BR-WS-OBS-001      | OBS     | WS-FS                                | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md         | registered      | src-tauri/src/ipc/system_tests.rs::file-level, src-tauri/src/workspace/routes.rs::file-level, src/services/edRuntime.test.ts::file-level, src/services/workspaceRuntime.test.ts::file-level                                                                                                                                                                                 |
| BR-WS-STATE-001    | STATE   | WS-DIFF-APPLY                        | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md         | registered      | src-tauri/src/workspace/diff_apply.rs::file-level, src/services/deRuntime.test.ts::file-level                                                                                                                                                                                                                                                                               |
| BR-WS-DATA-002     | DATA    | WS-TIMELINE                          | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md         | registered      | src-tauri/src/workspace/timeline.rs::file-level                                                                                                                                                                                                                                                                                                                             |
| BR-WS-STATE-002    | STATE   | WS-TIMELINE                          | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md         | registered      | src-tauri/src/workspace/timeline.rs::file-level                                                                                                                                                                                                                                                                                                                             |
| BR-WS-PERSIST-002  | PERSIST | DATA-MIGRATE                         | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md         | registered      | src-tauri/src/workspace/lifecycle.rs::file-level                                                                                                                                                                                                                                                                                                                            |
| BR-WS-PERSIST-001  | PERSIST | WS-SAVE                              | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md         | registered      | src-tauri/src/ipc/system_tests.rs::file-level, src-tauri/src/workspace/save.rs::file-level                                                                                                                                                                                                                                                                                  |
| BR-WS-STATE-003    | STATE   | WS-OPERATION                         | docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md     | registered      | src-tauri/src/workspace/operation.rs::file-level, src/services/agRuntime.test.ts::file-level                                                                                                                                                                                                                                                                                |
| BR-WS-DATA-003     | DATA    | WS-OPERATION, WS-TIMELINE            | docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md     | registered      | src-tauri/src/workspace/operation.rs::file-level, src/services/agRuntime.test.ts::file-level                                                                                                                                                                                                                                                                                |
| BR-WS-OBS-002      | OBS     | WS-OPERATION, AG-TURN                | docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md     | registered      | src-tauri/src/workspace/operation.rs::file-level, src/services/agRuntime.test.ts::file-level                                                                                                                                                                                                                                                                                |
| BR-WS-PERSIST-003  | PERSIST | WS-OPERATION, WS-TIMELINE            | docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md     | registered      | src-tauri/src/workspace/operation.rs::file-level, src/services/agRuntime.test.ts::file-level                                                                                                                                                                                                                                                                                |
| BR-WS-STATE-009    | STATE   | WS-FS, WS-TIMELINE                   | docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md     | registered      | src-tauri/src/workspace/operation.rs::file-level                                                                                                                                                                                                                                                                                                                            |

### 跨模块约束表（X-INDEX）

| constraint_id | 摘要                                                                                                                                                                                                                                                                                          | 涉及模块                                                                  | 链路影响                                                                                                                                  | 来源文档                                            |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- |
| X-FORBID-008  | 三态单向数据流——显示态不得直接或间接修改逻辑态持有者（ProseMirror doc / edStore.content 等），逻辑态的修改必须经受控接口（X-FORBID-005）；已有 LogicalState 向 DiskState 的转换只能由保存动作触发。无 LogicalState 的 PendingDiff 仅可按 BR-CORE-STATE-002 在用户接受后经 WS-DIFF-APPLY 原子落盘                                                                         | ALL（SYS, WS, ED, DE, AG, CBT, DOC, PRV, REF, AST, KB, TPL, ACCT, ENT） | IPC-CALL, ED-EDIT, ED-DECOR, DE-DIFF, WS-FS, WS-SAVE, WS-DIFF-APPLY, WS-TIMELINE, AG-TURN, AG-ASSIST, AG-TOOL, CBT-BUILD, DOC-CONVERT | docs/00_core/A-CORE-C-T-01_三态模型.md              |
| X-FORBID-009  | AI 生成内容默认属于"逻辑态候选（LogicalCandidate）"；存在 LogicalState 时候选须经 DE/ED 受控预览或打开迁移进入 LogicalState，显式接受或对仍有效 pending 卡保存都会裁决；不存在 LogicalState 时候选保持 DiffCard 挂载，只有用户接受才可经 WS-DIFF-APPLY 写入 DiskState。AI 产出不得绕过三分流受控路径直写逻辑态持有者或磁盘                                                                     | DE, AG, ED, CBT                                                       | DE-DIFF, AG-TURN, AG-ASSIST, ED-EDIT, CBT-BUILD                                                                                       | docs/00_core/A-CORE-C-T-01_三态模型.md              |
| X-FORBID-006  | AG 及其工具调用不得绕过两条唯一写路径：文档内容修改必须经 AG 产出→DE 内容 Diff 裁决→WS 保存/落盘；工作区资源操作必须经 WorkspaceOperationPlan→DecisionRequest 前置裁决→WS 重验并直接修改 DiskState。资源操作不得生成 DiffCard，内容 Diff 不得创建、删除、重命名或移动资源；CreateDocumentBundle 只在资源创建完成后把初始内容接入普通 Diff                                                             | AG, DE, ED, WS, REF, SYS, CBT                                         | AG-TURN, AG-TOOL, WS-DIFF-APPLY, WS-OPERATION                                                                                         | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md        |
| X-SEQ-004     | docx 等二进制文档格式的解析/生成必须经 DOC 转换管线，任何模块不得内置二进制文档转换库或另立转换路径                                                                                                                                                                                                                                     | DOC, ED, WS, PRV, CBT, AG                                             | DOC-CONVERT, ED-EDIT                                                                                                                  | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md    |
| X-FORBID-005  | 上层模块（DE/AG/REF 等）不得绕过 ED 装饰层与编辑接口直接操纵编辑器内部文档状态；编辑器文档模型的修改必经 ED 受控接口                                                                                                                                                                                                                         | ED, DE, AG, REF                                                       | ED-EDIT, ED-DECOR                                                                                                                     | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md       |
| X-SEQ-002     | 前端不得跨层直连文件系统/数据库/网络，必经 Service → Tauri command → Rust 后端                                                                                                                                                                                                                                    | SYS, WS, ED, DE, AG, CBT, DOC, PRV, REF, AST, KB, TPL, ACCT, ENT      | CLOUD-AUTH, CLOUD-GATE, CLOUD-INVOKE                                                                                                  | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md      |
| X-FORBID-003  | 模块依赖必须无环；SYS 为架构底座不得反向依赖任何业务模块                                                                                                                                                                                                                                                              | SYS, WS, ED, DE, AG, CBT, DOC, PRV, REF, AST, KB, TPL, ACCT, ENT      | CLOUD-AUTH, CLOUD-GATE, CLOUD-INVOKE                                                                                                  | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md      |
| X-FORBID-007  | 前端 Store 不得另立权威业务状态，须投影后端状态机（SM-*）与控制态（workspace.db）；界面流转由后端状态决定，前端只呈现与转达意图，不得本地维护与后端可漂移的并行业务状态                                                                                                                                                                                             | SYS, WS, ED, DE, AG, CBT, DOC, AST, KB, TPL, REF, PRV, ACCT, ENT      | ED-EDIT, AG-TURN, DE-DIFF, WS-FS, WS-SAVE, CBT-BUILD                                                                                  | docs/01_modules/SYS/A-SYS-C-T-02_前端交互架构与流程总图.md |
| X-SHARED-001  | 跨模块传递的实体/类型/枚举集合必须经 SYS 数据结构契约层（本文）定义，模块不得私自定义跨模块共享结构                                                                                                                                                                                                                                       | SYS, WS, ED, DE, AG, CBT, DOC, PRV, REF, AST, KB, TPL, ACCT, ENT      | DATA-MIGRATE                                                                                                                          | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md      |
| X-SHARED-002  | Agent 1.0 共享运行时对象（RuntimeProfile/AgentSession/AgentTurn/IntentResult/CapabilityGrant/TaskRun/TaskStep/StructuredVisibleEvent/ExternalContentRef/MemorySelectionRef）的职责、身份、事件真值、隔离与迁移边界以 SYS 数据结构契约层（A-SYS-I-P-01 §3.1）为唯一契约来源；任何模块不得以旧设计、现有代码、已注册工具、Prompt、前端参数、兼容默认值或测试绿灯反向改写冻结需求语义或隐式扩大授权 | SYS, AG, DE, ED, WS, REF, AST                                         | AG-TURN, DE-DIFF, ED-EDIT, WS-SAVE, WS-DIFF-APPLY, REF-INJECT, AST-MEMORY, IPC-CALL                                                   | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md      |
| X-FORBID-001  | 客户端不得持有或落地模型 provider 密钥，密钥仅服务端（AiGateway）持有                                                                                                                                                                                                                                                | SYS, ENT, ACCT, AG, CBT, AST, KB, TPL                                 | CLOUD-INVOKE, CLOUD-GATE                                                                                                              | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md       |
| X-SEQ-001     | 任何模型调用前必须先过订阅门禁硬门（CLOUD-GATE），无有效订阅一律拒绝，禁止跳过门禁直连网关                                                                                                                                                                                                                                          | ENT, AG, CBT, AST, KB, TPL                                            | CLOUD-GATE, CLOUD-INVOKE                                                                                                              | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md       |
| X-FORBID-002  | 订阅门禁不得存在客户端绕过路径，裁决以服务端为准，客户端不得本地缓存通过结论跳过校验                                                                                                                                                                                                                                                  | ENT, SYS                                                              | CLOUD-GATE                                                                                                                            | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md       |
| X-SEQ-003     | 状态化核心链（APC §6 强制阈值）必须先建状态机并在总图登记 machine_id 方可进入实现，未注册者阻断进入代码                                                                                                                                                                                                                               | SYS, WS, ED, DE, AG, CBT, DOC, AST                                    | CLOUD-AUTH, CLOUD-GATE, CLOUD-INVOKE                                                                                                  | docs/01_modules/SYS/A-SYS-M-S-01_状态机总图.md       |
| X-INTER-001   | 跨模块状态转移必须在总图登记转移契约，转移两侧 owner 状态机对接点须一致，不得各自假设状态/事件                                                                                                                                                                                                                                         | SYS, WS, ED, DE, AG, CBT                                              | CLOUD-GATE, CLOUD-INVOKE                                                                                                              | docs/01_modules/SYS/A-SYS-M-S-01_状态机总图.md       |
| X-FORBID-010  | 应用正常退出（非崩溃）完成前，当前 workspace 不得残留 status=pending 的 diff、effect_status=awaiting_save 的效果或 dirty=1 的未保存文档；不满足时必须阻断退出并呈现统一决策入口，不得静默继续退出或静默丢弃                                                                                                                                                    | SYS, DE, ED, WS, AG                                                   | APP-CLOSE, DE-DIFF, ED-EDIT, WS-SAVE, AG-TURN                                                                                         | docs/01_modules/SYS/A-SYS-M-T-02_应用关闭技术设计.md    |
| X-FORBID-011  | 应用启动时，任何跨会话残留的 pending diff、awaiting_save 效果或未保存 LogicalState 快照不得被当作可信权威态直接恢复为可裁决/可编辑态；必须先经异常退出清算转终态/丢弃，只能以只读历史形态呈现                                                                                                                                                                        | SYS, DE, ED, WS                                                       | APP-CLOSE, DE-DIFF, ED-EDIT, DATA-MIGRATE                                                                                             | docs/01_modules/SYS/A-SYS-M-T-02_应用关闭技术设计.md    |
| X-FORBID-004  | 文件操作/搜索/diff/上下文读取的目标路径不得越出活动 workspace root；任一越界路径一律拒绝，不静默截断或回退到 workspace 外                                                                                                                                                                                                               | WS, ED, DE, AG, CBT, REF, AST, KB, TPL, DOC, PRV                      | WS-FS, WS-SAVE, WS-DIFF-APPLY, WS-TIMELINE                                                                                            | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md       |

### 术语注册表

| term_id        | 所属链路                                                 | 正式中文名     | 正式英文名                   | 禁用别名                                                 | 来源文档                                              |
| -------------- | ---------------------------------------------------- | --------- | ----------------------- | ---------------------------------------------------- | ------------------------------------------------- |
| TERM-PCG-001   | PCG-GENERATE, PCG-AUDIT                              | 产品能力项     | ProductCapability       | 功能点, feature点, 产品功能块                                 | docs/00_core/A-CORE-C-P-02_产品能力图谱规范.md            |
| TERM-PCG-002   | PCG-GENERATE, PCG-AUDIT                              | 能力变体      | CapabilityVariant       | 功能组合, 功能分支, capability组合                             | docs/00_core/A-CORE-C-P-02_产品能力图谱规范.md            |
| TERM-PCG-003   | PCG-GENERATE                                         | 产品能力沙盘    | ProductCapabilityAtlas  | 功能总表, 产品功能图, 特性大盘                                    | docs/00_core/A-CORE-C-P-02_产品能力图谱规范.md            |
| TERM-PCG-004   | PCG-AUDIT                                            | 正式主链      | CanonicalChain          | 唯一路径, 固定链路, 主流程代码                                    | docs/00_core/A-CORE-C-P-02_产品能力图谱规范.md            |
| TERM-CORE-001  | IPC-CALL, ED-EDIT, ED-DECOR, DE-DIFF, WS-FS, WS-SAVE | 磁盘态       | DiskState               | 文件内容, 原始内容, 磁盘内容, 字节态, 物理态                           | docs/00_core/A-CORE-C-T-01_三态模型.md                |
| TERM-CORE-002  | IPC-CALL, ED-EDIT, ED-DECOR, DE-DIFF, WS-FS, WS-SAVE | 逻辑态       | LogicalState            | 编辑状态, 内存状态, 缓存状态, 运行态, 编辑缓存, 草稿内容                    | docs/00_core/A-CORE-C-T-01_三态模型.md                |
| TERM-CORE-003  | IPC-CALL, ED-DECOR, ED-EDIT                          | 显示态       | DisplayState            | 渲染态, 呈现层, 装饰态, 可视化状态, 界面态, 屏幕态                       | docs/00_core/A-CORE-C-T-01_三态模型.md                |
| TERM-CORE-004  | DE-DIFF, AG-TURN, AG-ASSIST, CBT-BUILD               | 逻辑态候选     | LogicalCandidate        | AI建议, 修改提议, 待定编辑, 提案态, 候选内容, 草案内容                    | docs/00_core/A-CORE-C-T-01_三态模型.md                |
| TERM-AG-001    | AG-TURN                                              | 代理回合      | AgentTurn               | 对话轮, 轮次, 会话轮, round, 聊天回合                            | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-002    | AG-TOOL                                              | 工具调用      | ToolCall                | 函数调用, function call, 工具执行, 能力调用                      | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-003    | AG-ASSIST                                            | 局部修改      | InlineAssist            | 内联辅助, 行内辅助, 内联建议, inline建议, 行内AI                     | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-004    | AG-ASSIST                                            | 辅助续写      | Autocomplete            | 自动补全, 自动完成, 补全建议, 智能补全, 续写补全                         | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-006    | AG-TURN                                              | 编辑模式      | EditMode                | 对话模式, chat模式, chat 模式, 对话编辑模式                        | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-007    | AG-TURN                                              | 代理构建      | BuildMode               | 构建模式, agent模式, agent 模式, Chat Build, chat-build      | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-008    | AG-TURN                                              | 代理编辑      | AgentEdit               | 对话编辑, 对话编辑板块                                         | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-009    | AG-TURN                                              | 代理编辑会话    | AgentSession            | 聊天标签, 对话标签, 会话标签, agent标签, chat tab, session tab     | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-010    | AG-MODEL-CATALOG, AG-ASSIST, AG-TURN                 | 预设模型      | PresetModel             | 内置模型, 托管模型, 官方模型                                     | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-011    | AG-TURN                                              | 消息片段      | MessageSegment          | 消息块, 消息段, 文本片段, 卡片锚点, segment                        | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-012    | AG-TURN, AG-TOOL                                     | 本轮意图结果    | IntentResult            | 意图布尔值, 编辑意图标记, intent flag, 意图关键词结果                  | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-013    | AG-TURN                                              | 运行时档位     | RuntimeProfile          | 运行时配置, 能力档位, runtime 配置, profile 档位, 代理档位, 运行配置      | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-014    | AG-TURN, AG-TOOL                                     | 当轮能力授权    | CapabilityGrant         | 工具授权, 能力授权, 权限授予, grant, 当轮授权, 工具许可                  | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-015    | AG-TURN                                              | 受限任务实例    | TaskRun                 | 任务实例, 后台任务, 任务队列, 子任务集合, task 实例, 复合任务对象             | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-016    | AG-TURN                                              | 任务步骤      | TaskStep                | 任务步骤, 步骤, 子步骤, step 单元, 执行步                          | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-017    | AG-TURN                                              | 任务级裁决请求   | DecisionRequest         | 确认卡, 裁决项, decision 实例, 确认请求                          | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-018    | AG-TURN                                              | 任务计划      | TaskPlan                | 计划, 任务计划清单, plan 单元, 执行计划                            | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-019    | AG-TURN, AG-TOOL                                     | 显式编辑目标集   | ExplicitEditTargetSet   | 编辑白名单, 目标白名单, 跨文档白名单, 隐式目标集, 模型目标集                   | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-005    | AG-TURN                                              | 意图沿用判定    | IntentCarryover         | 历史复用, 意图继承, 上下文继承, 意图沿袭, carryover                   | docs/01_modules/AG/A-AG-M-T-02_提示词架构设计.md         |
| TERM-AST-001   | AST-MEMORY                                           | 记忆条目      | MemoryEntry             | 记忆点, 记忆项, 记忆片段, memory条目                             | docs/01_modules/AST/A-AST-M-T-01_记忆协同技术设计.md      |
| TERM-AST-002   | REF-INJECT, AST-MEMORY                               | 显式项目记忆引用  | MemorySelectionRef      | 记忆引用, 记忆选择, 记忆注入引用, 记忆条目引用, 记忆选中项                    | docs/01_modules/AST/A-AST-M-T-01_记忆协同技术设计.md      |
| TERM-CBT-001   | CBT-BUILD                                            | 构建回合      | BuildRun                | 构建会话, 构建过程, build流程, 项目构建轮                           | docs/01_modules/CBT/A-CBT-M-T-01_构建技术设计.md        |
| TERM-CBT-002   | CBT-BUILD                                            | 构建任务      | BuildTask               | 构建步骤, 子任务, build单元, 任务项                              | docs/01_modules/CBT/A-CBT-M-T-01_构建技术设计.md        |
| TERM-CBT-003   | CBT-BUILD                                            | 项目蓝图      | BuildBlueprint          | 项目大纲, 构建大纲, 需求蓝图, blueprint                          | docs/01_modules/CBT/A-CBT-M-T-01_构建技术设计.md        |
| TERM-DE-001    | DE-DIFF                                              | 待裁决修改     | PendingDiff             | 修改提案, 变更请求, 待定diff, 修改单, 待审修改                        | docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md    |
| TERM-DE-002    | DE-DIFF                                              | 修改建议      | ChangeProposal          | AI建议, 修改意见, 编辑建议, 产出建议                               | docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md    |
| TERM-DE-004    | DE-DIFF                                              | 修改载荷      | DiffPayload             | diff载荷, 内容载荷, payload, diff内容, 落盘载荷, 提案载荷            | docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md    |
| TERM-DOC-001   | DOC-CONVERT                                          | 样式子集      | StyleSubset             | 样式白名单, 格式子集, 样式范围, 支持样式表                             | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md      |
| TERM-DOC-002   | DOC-CONVERT                                          | 编辑草稿      | EditDraft               | 草稿副本, draft文件, 临时编辑文件, 编辑副本                          | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md      |
| TERM-DOC-003   | DOC-CONVERT, ED-EDIT                                 | b-docx    | BDocx                   | 模拟docx, HTML模拟docx, 可编辑docx, 项目docx, 生成docx, docx编辑稿 | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md      |
| TERM-DOC-004   | DOC-CONVERT                                          | 字体身份修复    | FontIdentityFix         | 字体fallback修复, 字形回退修复, 字体替换修复                         | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md      |
| TERM-DOC-005   | DOC-CONVERT, ED-EDIT, DE-DIFF                        | 单元格底纹     | TableCellShading        | 单元格背景, 表格底色, 单元格填色, cell background                  | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md      |
| TERM-ED-001    | ED-EDIT                                              | 内容块       | Block                   | 内容节点, 段落块, 文本块, 块节点, chunk                           | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         |
| TERM-ED-002    | ED-DECOR                                             | 编辑器装饰层    | EditorDecoration        | 高亮层, 标注层, 覆盖层, overlay层, 装饰器层                        | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         |
| TERM-ED-004    | ED-EDIT                                              | 字符锚       | CharAnchor              | 字符偏移锚, 文本锚, 偏移量锚, inline锚, 字符定位符                     | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         |
| TERM-ED-005    | ED-EDIT                                              | 选区引用      | SelectionRef            | 选区信息, 拖拽引用, selection数据, 选区数据                        | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         |
| TERM-ED-006    | ED-EDIT                                              | 样式操作      | StyleOp                 | 样式指令, 格式操作, style命令, 样式patch                         | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         |
| TERM-ED-007    | ED-EDIT, AG-TOOL, DE-DIFF                            | 表格操作      | TableOperation          | 表格指令, 表格补丁, table命令                                  | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         |
| TERM-ED-008    | ED-EDIT, AG-TOOL, DE-DIFF, DOC-CONVERT               | 图片操作      | ImageOperation          | 图片指令, 图片补丁, image命令                                  | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         |
| TERM-KB-001    | KB-RETRIEVE                                          | 知识条目      | KnowledgeEntry          | 知识点, 资料条目, 知识片段, kb条目                                | docs/01_modules/KB/A-KB-M-T-01_知识库技术设计.md         |
| TERM-REF-001   | REF-INJECT                                           | 引用锚       | ReferenceAnchor         | 引用链接, 引用指针, mention锚, 引用定位符                          | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md        |
| TERM-REF-002   | REF-INJECT                                           | 注入载荷      | InjectionPayload        | 上下文载荷, 注入内容包, 引用载荷, 引用内容体                            | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md        |
| TERM-REF-003   | REF-INJECT                                           | 引用输入载荷    | ReferencePayload        | 引用内容包, 引用输入包, ref payload, mention payload           | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md        |
| TERM-REF-004   | REF-INJECT                                           | 引用令牌      | ReferenceToken          | 引用标签, 输入标签, inline ref chip, 引用片                     | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md        |
| TERM-REF-005   | REF-INJECT, AG-TURN                                  | 外部内容引用    | ExternalContentRef      | 外部文件引用, 外部读取引用, 外部内容令牌, 外部分片引用, 附加文件引用               | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md        |
| TERM-SYS-001   | IPC-CALL                                             | 设计令牌      | DesignToken             | CSS变量, 设计变量, 视觉令牌, token系统, 视觉常量, design token       | docs/01_modules/SYS/A-SYS-C-T-02_前端交互架构与流程总图.md   |
| TERM-DE-003    | DE-DIFF                                              | 修改卡       | DiffCard                | diff卡片, AI建议卡, 修改面板, 修改组件, 变更卡, 审阅卡                  | docs/01_modules/SYS/A-SYS-C-T-02_前端交互架构与流程总图.md   |
| TERM-ED-003    | ED-DECOR                                             | 绿色审阅层     | GreenReview             | 绿色高亮, 编辑器diff层, diff装饰, 正文高亮层, 修改高亮, green review    | docs/01_modules/SYS/A-SYS-C-T-02_前端交互架构与流程总图.md   |
| TERM-CLOUD-001 | CLOUD-GATE                                           | 订阅门禁      | SubscriptionGate        | 订阅校验, 会员校验, 付费门槛, 鉴权门                                | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md         |
| TERM-CLOUD-002 | CLOUD-INVOKE                                         | AI网关      | AiGateway               | 模型代理, API网关, 代理服务, provider代理                        | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md         |
| TERM-CLOUD-003 | CLOUD-AUTH                                           | 会话令牌      | SessionToken            | 登录态, token, 认证令牌, 鉴权码                                | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md         |
| TERM-CLOUD-004 | CLOUD-INVOKE                                         | BYOK透传    | ByokPassthrough         | 自带密钥, 自定义key, 用户密钥直连                                 | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md         |
| TERM-SYS-002   | AG-TURN, AG-TOOL, ED-EDIT, DE-DIFF, DOC-CONVERT      | BDocx能力清单 | BDocxCapabilityManifest | BDocx能力表, 代理能力表, 编辑能力列表, 能力白名单                       | docs/01_modules/SYS/A-SYS-I-P-04_BDocx代理编辑能力协议.md |
| TERM-TPL-001   | TPL-FLOW                                             | 写作模板      | Template                | 文档模板片, 提示模板项, 样板, template条目                         | docs/01_modules/TPL/A-TPL-M-T-01_模板工作流技术设计.md     |
| TERM-TPL-002   | TPL-FLOW                                             | 写作工作流     | WritingWorkflow         | 流程, 工作流程, pipeline, 写作流程图                            | docs/01_modules/TPL/A-TPL-M-T-01_模板工作流技术设计.md     |
| TERM-WS-001    | WS-FS                                                | 工作区       | Workspace               | 工作空间, 项目根目录, 工作目录, root目录, 项目目录                      | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md         |
| TERM-WS-002    | WS-TIMELINE                                          | 文档时间轴     | DocumentTimeline        | 历史记录, 版本历史, 历史栈, 版本栈, 历史链                            | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md         |
| TERM-WS-003    | WS-OPERATION, AG-TURN                                | 工作区资源操作   | WorkspaceOperation      | 文件整理,文件操作diff,资源diff,结构diff,文件树diff                  | docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md     |
| TERM-WS-004    | WS-OPERATION, AG-TURN                                | 工作区资源操作计划 | WorkspaceOperationPlan  | 删除diff,操作草稿,文件修改建议,结构Proposal                        | docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md     |
| TERM-WS-005    | WS-TIMELINE, WS-OPERATION                            | 工作区时间轴    | WorkspaceTimeline       | 回收站,撤销栈,文件历史栈,项目版本栈                                  | docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md     |
| TERM-WS-006    | WS-TIMELINE, WS-OPERATION                            | 工作区资源操作条目 | WorkspaceOperationEntry | 删除节点,文件diff节点,操作日志行                                  | docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md     |
| TERM-WS-007    | WS-OPERATION, DE-DIFF, AG-TURN                       | 新建文档绑定    | CreateDocumentBundle    | 新建diff,空文件diff,创建提案                                  | docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md     |

### 链路视图

#### AG-ASSIST

| codes                                                                             | type   | rules                                              | 文件                                        | 行号   |
| --------------------------------------------------------------------------------- | ------ | -------------------------------------------------- | ----------------------------------------- | ---- |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-009                                               | RB     | BR-AG-STATE-003 term_ref: TERM-AG-004              | src/components/ag/AutocompleteOverlay.tsx | 16   |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-007                                               | RB     | BR-AG-STATE-003 term_ref: TERM-AG-003, TERM-AG-004 | src/components/ed/Editor.tsx              | 810  |
| BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-003                                           | EFFECT | BR-AG-STATE-003 term_ref: TERM-AG-004              | src/components/ed/Editor.tsx              | 1008 |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-002                                               | RB     | BR-AG-STATE-003 term_ref: TERM-AG-003, TERM-AG-004 | src/components/ed/Editor.tsx              | 1890 |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-004                                               | RB     | BR-AG-STATE-003 term_ref: TERM-AG-003, TERM-AG-004 | src/components/ed/assistPlacement.ts      | 36   |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-005                                               | RB     | BR-AG-STATE-003 term_ref: TERM-AG-003, TERM-AG-004 | src/components/ed/assistPlacement.ts      | 60   |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-006                                               | RB     | BR-AG-STATE-003 term_ref: TERM-AG-003, TERM-AG-004 | src/components/ed/assistPlacement.ts      | 78   |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-003                                               | RB     | BR-AG-STATE-003 term_ref: TERM-AG-003, TERM-AG-004 | src/components/ed/assistPlacement.ts      | 96   |
| BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-002                                           | EFFECT | BR-AG-STATE-003 term_ref: TERM-AG-003, TERM-AG-004 | src/services/agService.ts                 | 1380 |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-008                                               | RB     | BR-AG-STATE-003 term_ref: TERM-AG-003, TERM-AG-004 | src/store/assistStore.ts                  | 94   |
| BR-AG-DATA-007-QUERY-AG-AG-MODEL-CATALOG-013                                      | QUERY  | BR-AG-DATA-007 term_ref: TERM-AG-010               | src/store/modelStore.ts                   | 162  |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-001                                               | RB     | BR-AG-STATE-003                                    | src-tauri/src/ag/assist.rs                | 40   |
| BR-AG-DATA-002-DATA-AG-AG-ASSIST-011                                              | DATA   | BR-AG-DATA-002                                     | src-tauri/src/ag/assist_prompt.rs         | 19   |
| BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-010                                           | EFFECT | BR-AG-STATE-003                                    | src-tauri/src/ag/assist_runtime.rs        | 50   |
| BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-001, BR-AG-VERIFY-001-EFFECT-AG-AG-ASSIST-001 | EFFECT | BR-AG-STATE-003, BR-AG-VERIFY-001                  | src-tauri/src/ag/assist_runtime.rs        | 92   |

#### AG-MODEL-CATALOG

| codes                                         | type   | rules                                | 文件                      | 行号  |
| --------------------------------------------- | ------ | ------------------------------------ | ----------------------- | --- |
| BR-AG-DATA-007-QUERY-AG-AG-MODEL-CATALOG-013  | QUERY  | BR-AG-DATA-007 term_ref: TERM-AG-010 | src/store/modelStore.ts | 162 |
| BR-AG-DATA-006-EFFECT-AG-AG-MODEL-CATALOG-006 | EFFECT | BR-AG-DATA-006 term_ref: TERM-AG-010 | src/store/modelStore.ts | 179 |

#### AG-TOOL

| codes                                                                                                                                                                                                                                                | type   | rules                                                                                                                                                             | 文件                                     | 行号   |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------- | ---- |
| BR-SYS-DATA-006-DATA-AG-AG-TOOL-001                                                                                                                                                                                                                  | DATA   | BR-SYS-DATA-006 term_ref: TERM-SYS-002                                                                                                                            | src-tauri/src/ag/bdocx_capabilities.rs | 611  |
| BR-SYS-DATA-006-DATA-AG-AG-TOOL-002                                                                                                                                                                                                                  | DATA   | BR-SYS-DATA-006 term_ref: TERM-SYS-002, TERM-ED-008                                                                                                               | src-tauri/src/ag/bdocx_capabilities.rs | 897  |
| BR-SYS-DATA-006-GUARD-AG-AG-TOOL-021, BR-ED-DATA-005-GUARD-AG-AG-TOOL-021                                                                                                                                                                            | GUARD  | BR-SYS-DATA-006, BR-ED-DATA-005 term_ref: TERM-SYS-002, TERM-ED-006, TERM-ED-004                                                                                  | src-tauri/src/ag/bdocx_capabilities.rs | 1566 |
| BR-SYS-DATA-006-GUARD-AG-AG-TOOL-022, BR-ED-DATA-003-GUARD-AG-AG-TOOL-022                                                                                                                                                                            | GUARD  | BR-SYS-DATA-006, BR-ED-DATA-003 term_ref: TERM-SYS-002, TERM-ED-008                                                                                               | src-tauri/src/ag/bdocx_capabilities.rs | 1918 |
| BR-AG-DATA-003-RB-AG-AG-TOOL-016, BR-ED-DATA-001-RB-AG-AG-TOOL-016, BR-DE-DATA-001-RB-AG-AG-TOOL-016                                                                                                                                                 | RB     | BR-AG-DATA-003, BR-ED-DATA-001, BR-DE-DATA-001 term_ref: TERM-ED-001                                                                                              | src-tauri/src/ag/block_ops.rs          | 299  |
| BR-AG-DATA-001-QUERY-AG-AG-TOOL-011, BR-ED-DATA-003-QUERY-AG-AG-TOOL-011                                                                                                                                                                             | QUERY  | BR-AG-DATA-001, BR-ED-DATA-003 term_ref: TERM-ED-008, TERM-DOC-003                                                                                                | src-tauri/src/ag/blocks.rs             | 190  |
| BR-AG-DATA-004-RB-AG-AG-TURN-015                                                                                                                                                                                                                     | RB     | BR-AG-DATA-004 term_ref: TERM-AG-012                                                                                                                              | src-tauri/src/ag/context.rs            | 420  |
| BR-SYS-DATA-006-GUARD-AG-AG-TOOL-004, BR-ED-DATA-001-GUARD-AG-AG-TOOL-004, BR-ED-DATA-003-GUARD-AG-AG-TOOL-004                                                                                                                                       | GUARD  | BR-SYS-DATA-006, BR-ED-DATA-001, BR-ED-DATA-003 term_ref: TERM-SYS-002, TERM-ED-008                                                                               | src-tauri/src/ag/edit_image_runtime.rs | 8    |
| BR-AG-STATE-002-EFFECT-AG-AG-TOOL-017, BR-AG-DATA-003-EFFECT-AG-AG-TOOL-017                                                                                                                                                                          | EFFECT | BR-AG-STATE-002, BR-AG-DATA-003                                                                                                                                   | src-tauri/src/ag/edit_runtime.rs       | 19   |
| BR-ED-DATA-007-GUARD-AG-AG-TOOL-018                                                                                                                                                                                                                  | GUARD  | BR-ED-DATA-007 term_ref: TERM-ED-007                                                                                                                              | src-tauri/src/ag/edit_table_runtime.rs | 8    |
| BR-AG-DATA-003-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-007-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-008-DATA-AG-AG-TARGETSET-001                                                                                                                        | DATA   | BR-AG-DATA-003, BR-SYS-STATE-007, BR-SYS-STATE-008 term_ref: TERM-AG-019, TERM-AG-001, TERM-AG-015, TERM-REF-003                                                  | src-tauri/src/ag/target_set.rs         | 32   |
| BR-AG-DATA-003-EFFECT-AG-AG-TARGETSET-002, BR-SYS-PERSIST-001-EFFECT-AG-AG-TARGETSET-002                                                                                                                                                             | EFFECT | BR-AG-DATA-003, BR-SYS-PERSIST-001 term_ref: TERM-AG-019, TERM-AG-015                                                                                             | src-tauri/src/ag/target_set.rs         | 202  |
| BR-AG-DATA-003-GUARD-AG-AG-TARGETSET-003                                                                                                                                                                                                             | GUARD  | BR-AG-DATA-003 term_ref: TERM-AG-019, TERM-DE-002                                                                                                                 | src-tauri/src/ag/target_set.rs         | 272  |
| BR-SYS-STATE-008-EFFECT-AG-AG-TASK-002                                                                                                                                                                                                               | EFFECT | BR-SYS-STATE-008 term_ref: TERM-AG-015, TERM-AG-018                                                                                                               | src-tauri/src/ag/task.rs               | 109  |
| BR-SYS-STATE-008-EFFECT-AG-AG-DECISION-001, BR-AG-DATA-003-EFFECT-AG-AG-DECISION-001                                                                                                                                                                 | EFFECT | BR-SYS-STATE-008, BR-AG-DATA-003 term_ref: TERM-AG-015, TERM-AG-017, TERM-AG-019                                                                                  | src-tauri/src/ag/task.rs               | 546  |
| BR-SYS-STATE-008-EFFECT-AG-AG-DECISION-002, BR-AG-DATA-003-EFFECT-AG-AG-DECISION-002                                                                                                                                                                 | EFFECT | BR-SYS-STATE-008, BR-AG-DATA-003 term_ref: TERM-AG-015, TERM-AG-017, TERM-AG-019                                                                                  | src-tauri/src/ag/task.rs               | 607  |
| BR-SYS-STATE-008-GUARD-AG-AG-TASK-003                                                                                                                                                                                                                | GUARD  | BR-SYS-STATE-008 term_ref: TERM-AG-015, TERM-AG-018                                                                                                               | src-tauri/src/ag/task.rs               | 903  |
| BR-AG-STATE-002-API-AG-WS-OPERATION-005, BR-AG-DATA-003-API-AG-WS-OPERATION-005, BR-SYS-STATE-008-API-AG-WS-OPERATION-005, BR-WS-STATE-003-API-AG-WS-OPERATION-005, BR-WS-PERSIST-003-API-AG-WS-OPERATION-005, BR-WS-OBS-002-API-AG-WS-OPERATION-005 | API    | BR-AG-STATE-002, BR-AG-DATA-003, BR-SYS-STATE-008, BR-WS-STATE-003, BR-WS-PERSIST-003, BR-WS-OBS-002 term_ref: TERM-AG-015, TERM-AG-017, TERM-AG-019, TERM-WS-003 | src-tauri/src/ag/task_runtime.rs       | 173  |
| BR-AG-STATE-002-RB-AG-AG-TOOL-001                                                                                                                                                                                                                    | RB     | BR-AG-STATE-002 term_ref: TERM-AG-002                                                                                                                             | src-tauri/src/ag/tool.rs               | 52   |
| BR-AG-STATE-002-GUARD-AG-AG-TOOL-001                                                                                                                                                                                                                 | GUARD  | BR-AG-STATE-002                                                                                                                                                   | src-tauri/src/ag/tool.rs               | 91   |
| BR-AG-STATE-002-QUERY-AG-AG-TOOL-012, BR-AG-DATA-001-QUERY-AG-AG-TOOL-012, BR-WS-DATA-001-QUERY-AG-AG-TOOL-012                                                                                                                                       | QUERY  | BR-AG-STATE-002, BR-AG-DATA-001, BR-WS-DATA-001 term_ref: TERM-ED-008, TERM-WS-001                                                                                | src-tauri/src/ag/tool_exec.rs          | 38   |
| BR-AG-STATE-002-DATA-AG-AG-TOOL-034, BR-AG-DATA-004-DATA-AG-AG-TOOL-034, BR-SYS-STATE-007-DATA-AG-AG-TOOL-034                                                                                                                                        | DATA   | BR-AG-STATE-002, BR-AG-DATA-004, BR-SYS-STATE-007 term_ref: TERM-AG-012, TERM-AG-013, TERM-AG-014, TERM-AG-015                                                    | src-tauri/src/ag/tool_registry.rs      | 49   |
| BR-AG-STATE-002-GUARD-AG-AG-TOOL-035, BR-SYS-STATE-007-GUARD-AG-AG-TOOL-035                                                                                                                                                                          | GUARD  | BR-AG-STATE-002, BR-SYS-STATE-007 term_ref: TERM-AG-014, TERM-AG-017                                                                                              | src-tauri/src/ag/tool_registry.rs      | 176  |
| BR-AG-DATA-003-GUARD-AG-AG-TOOL-033, BR-AG-STATE-002-GUARD-AG-AG-TOOL-033                                                                                                                                                                            | GUARD  | BR-AG-DATA-003, BR-AG-STATE-002 term_ref: TERM-AG-019, TERM-AG-017                                                                                                | src-tauri/src/ag/tool_registry.rs      | 311  |
| BR-SYS-STATE-007-GUARD-AG-AG-TURN-037                                                                                                                                                                                                                | GUARD  | BR-SYS-STATE-007 term_ref: TERM-AG-013                                                                                                                            | src-tauri/src/ag/turn_runtime.rs       | 25   |
| BR-AG-STATE-001-API-AG-AG-TURN-018, BR-SYS-STATE-007-API-AG-AG-TURN-018                                                                                                                                                                              | API    | BR-AG-STATE-001, BR-SYS-STATE-007 term_ref: TERM-AG-001, TERM-AG-012, TERM-AG-013, TERM-AG-014, TERM-AG-015, TERM-AG-016                                          | src-tauri/src/ag/turn_runtime.rs       | 47   |
| BR-AG-STATE-002-EFFECT-AG-AG-TOOL-038                                                                                                                                                                                                                | EFFECT | BR-AG-STATE-002 term_ref: TERM-AG-002                                                                                                                             | src-tauri/src/ag/turn_tool_loop.rs     | 65   |
| BR-AG-STATE-002-RB-AG-AG-TOOL-018, BR-SYS-STATE-008-RB-AG-AG-TOOL-018                                                                                                                                                                                | RB     | BR-AG-STATE-002, BR-SYS-STATE-008 term_ref: TERM-AG-014, TERM-AG-015, TERM-AG-016                                                                                 | src-tauri/src/ag/turn_tool_loop.rs     | 93   |

#### AG-TURN

| codes                                                                                                                                                                                                                                                | type   | rules                                                                                                                                                             | 文件                                     | 行号   |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------- | ---- |
| BR-AG-UI-001-RB-AG-AG-TURN-013                                                                                                                                                                                                                       | RB     | BR-AG-UI-001 term_ref: TERM-AG-011                                                                                                                                | src/components/ag/MessageFlow.tsx      | 8    |
| BR-AG-UI-001-RB-AG-AG-TURN-014                                                                                                                                                                                                                       | RB     | BR-AG-UI-001 term_ref: TERM-AG-011                                                                                                                                | src/components/ag/MessageList.tsx      | 12   |
| BR-AG-UI-001-RB-AG-AG-TURN-012                                                                                                                                                                                                                       | RB     | BR-AG-UI-001 term_ref: TERM-AG-011                                                                                                                                | src/components/ag/MessageMarkdown.tsx  | 8    |
| BR-AG-DATA-001-EFFECT-AG-AG-TURN-005                                                                                                                                                                                                                 | EFFECT | BR-AG-DATA-001 term_ref: TERM-DOC-003, TERM-ED-008                                                                                                                | src/services/agService.ts              | 298  |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-006, BR-AG-DATA-001-EFFECT-AG-AG-TURN-006                                                                                                                                                                         | EFFECT | BR-SYS-STATE-008, BR-AG-DATA-001 term_ref: TERM-AG-015, TERM-ED-008                                                                                               | src/services/agService.ts              | 1060 |
| BR-SYS-STATE-007-EFFECT-AG-AG-TURN-035, BR-AG-STATE-001-EFFECT-AG-AG-TURN-035                                                                                                                                                                        | EFFECT | BR-SYS-STATE-007, BR-AG-STATE-001 term_ref: TERM-AG-001                                                                                                           | src/services/agService.ts              | 1445 |
| BR-AG-DATA-007-QUERY-AG-AG-MODEL-CATALOG-013                                                                                                                                                                                                         | QUERY  | BR-AG-DATA-007 term_ref: TERM-AG-010                                                                                                                              | src/store/modelStore.ts                | 162  |
| BR-SYS-DATA-006-DATA-AG-AG-TOOL-032                                                                                                                                                                                                                  | DATA   | BR-SYS-DATA-006 term_ref: TERM-SYS-002                                                                                                                            | src-tauri/src/ag/bdocx_capabilities.rs | 1298 |
| BR-AG-DATA-001-QUERY-AG-AG-TURN-010, BR-DOC-DATA-003-QUERY-AG-AG-TURN-010                                                                                                                                                                            | QUERY  | BR-AG-DATA-001, BR-DOC-DATA-003 term_ref: TERM-DOC-003, TERM-ED-001                                                                                               | src-tauri/src/ag/blocks.rs             | 415  |
| BR-AG-DATA-004-RB-AG-AG-TURN-015                                                                                                                                                                                                                     | RB     | BR-AG-DATA-004 term_ref: TERM-AG-012                                                                                                                              | src-tauri/src/ag/context.rs            | 420  |
| BR-AG-DATA-001-DATA-AG-AG-TURN-001                                                                                                                                                                                                                   | DATA   | BR-AG-DATA-001                                                                                                                                                    | src-tauri/src/ag/context.rs            | 845  |
| BR-AG-DATA-002-DATA-AG-AG-TURN-016                                                                                                                                                                                                                   | DATA   | BR-AG-DATA-002                                                                                                                                                    | src-tauri/src/ag/context.rs            | 915  |
| BR-AG-DATA-005-DATA-AG-AG-TURN-004                                                                                                                                                                                                                   | DATA   | BR-AG-DATA-005                                                                                                                                                    | src-tauri/src/ag/context.rs            | 966  |
| BR-AG-DATA-004-DATA-AG-AG-TURN-003                                                                                                                                                                                                                   | DATA   | BR-AG-DATA-004 term_ref: TERM-AG-005                                                                                                                              | src-tauri/src/ag/context.rs            | 1054 |
| BR-AG-DATA-004-DATA-AG-AG-TURN-005                                                                                                                                                                                                                   | DATA   | BR-AG-DATA-004 term_ref: TERM-AG-005                                                                                                                              | src-tauri/src/ag/context.rs            | 1114 |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-030                                                                                                                                                                                                              | EFFECT | BR-AG-PERSIST-001 term_ref: TERM-AG-009                                                                                                                           | src-tauri/src/ag/history.rs            | 163  |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-002                                                                                                                                                                                                               | QUERY  | BR-AG-PERSIST-001 term_ref: TERM-AG-009                                                                                                                           | src-tauri/src/ag/history.rs            | 245  |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-003                                                                                                                                                                                                              | EFFECT | BR-AG-PERSIST-001 term_ref: TERM-AG-001, TERM-AG-009                                                                                                              | src-tauri/src/ag/history.rs            | 295  |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-003                                                                                                                                                                                                               | QUERY  | BR-AG-PERSIST-001 term_ref: TERM-AG-001, TERM-AG-009                                                                                                              | src-tauri/src/ag/history.rs            | 357  |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-031                                                                                                                                                                                                              | EFFECT | BR-AG-PERSIST-001 term_ref: TERM-AG-001, TERM-AG-009                                                                                                              | src-tauri/src/ag/history.rs            | 396  |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-020                                                                                                                                                                                                               | QUERY  | BR-AG-PERSIST-001                                                                                                                                                 | src-tauri/src/ag/history_routes.rs     | 7    |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-021                                                                                                                                                                                                              | EFFECT | BR-AG-PERSIST-001                                                                                                                                                 | src-tauri/src/ag/history_routes.rs     | 23   |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-022                                                                                                                                                                                                              | EFFECT | BR-AG-PERSIST-001                                                                                                                                                 | src-tauri/src/ag/history_routes.rs     | 72   |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-023                                                                                                                                                                                                               | QUERY  | BR-AG-PERSIST-001                                                                                                                                                 | src-tauri/src/ag/history_routes.rs     | 121  |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-024                                                                                                                                                                                                              | EFFECT | BR-AG-PERSIST-001                                                                                                                                                 | src-tauri/src/ag/history_routes.rs     | 144  |
| BR-AG-STATE-004-GUARD-AG-AG-TURN-019                                                                                                                                                                                                                 | GUARD  | BR-AG-STATE-004                                                                                                                                                   | src-tauri/src/ag/memory_link.rs        | 22   |
| BR-AG-STATE-004-EFFECT-AG-AG-TURN-029                                                                                                                                                                                                                | EFFECT | BR-AG-STATE-004                                                                                                                                                   | src-tauri/src/ag/memory_link.rs        | 49   |
| BR-AG-DATA-001-DATA-AG-AG-TURN-011, BR-AG-DATA-002-DATA-AG-AG-TURN-011, BR-AG-DATA-004-DATA-AG-AG-TURN-011, BR-AG-DATA-005-DATA-AG-AG-TURN-011                                                                                                       | DATA   | BR-AG-DATA-001, BR-AG-DATA-002, BR-AG-DATA-004, BR-AG-DATA-005 term_ref: TERM-AG-005                                                                              | src-tauri/src/ag/prompt_runtime.rs     | 256  |
| BR-AG-DATA-001-QUERY-AG-AG-TURN-029                                                                                                                                                                                                                  | QUERY  | BR-AG-DATA-001                                                                                                                                                    | src-tauri/src/ag/reference_runtime.rs  | 10   |
| BR-AG-DATA-001-RB-AG-AG-TURN-017                                                                                                                                                                                                                     | RB     | BR-AG-DATA-001 term_ref: TERM-REF-003                                                                                                                             | src-tauri/src/ag/reference_runtime.rs  | 71   |
| BR-AG-DATA-003-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-007-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-008-DATA-AG-AG-TARGETSET-001                                                                                                                        | DATA   | BR-AG-DATA-003, BR-SYS-STATE-007, BR-SYS-STATE-008 term_ref: TERM-AG-019, TERM-AG-001, TERM-AG-015, TERM-REF-003                                                  | src-tauri/src/ag/target_set.rs         | 32   |
| BR-AG-DATA-003-EFFECT-AG-AG-TARGETSET-002, BR-SYS-PERSIST-001-EFFECT-AG-AG-TARGETSET-002                                                                                                                                                             | EFFECT | BR-AG-DATA-003, BR-SYS-PERSIST-001 term_ref: TERM-AG-019, TERM-AG-015                                                                                             | src-tauri/src/ag/target_set.rs         | 202  |
| BR-SYS-STATE-008-RB-AG-AG-TASK-001, BR-AG-STATE-001-RB-AG-AG-TASK-001                                                                                                                                                                                | RB     | BR-SYS-STATE-008, BR-AG-STATE-001                                                                                                                                 | src-tauri/src/ag/task.rs               | 52   |
| BR-SYS-STATE-008-EFFECT-AG-AG-TASK-002                                                                                                                                                                                                               | EFFECT | BR-SYS-STATE-008 term_ref: TERM-AG-015, TERM-AG-018                                                                                                               | src-tauri/src/ag/task.rs               | 109  |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-032, BR-AG-DATA-001-EFFECT-AG-AG-TURN-032                                                                                                                                                                         | EFFECT | BR-SYS-STATE-008, BR-AG-DATA-001 term_ref: TERM-AG-015, TERM-ED-008                                                                                               | src-tauri/src/ag/task.rs               | 148  |
| BR-SYS-STATE-008-GUARD-AG-AG-TURN-005, BR-AG-DATA-001-GUARD-AG-AG-TURN-005                                                                                                                                                                           | GUARD  | BR-SYS-STATE-008, BR-AG-DATA-001 term_ref: TERM-AG-015, TERM-ED-008                                                                                               | src-tauri/src/ag/task.rs               | 197  |
| BR-SYS-STATE-008-GUARD-AG-AG-TURN-006, BR-AG-DATA-001-GUARD-AG-AG-TURN-006                                                                                                                                                                           | GUARD  | BR-SYS-STATE-008, BR-AG-DATA-001 term_ref: TERM-AG-015, TERM-ED-008                                                                                               | src-tauri/src/ag/task.rs               | 279  |
| BR-SYS-STATE-008-EFFECT-AG-AG-DECISION-001, BR-AG-DATA-003-EFFECT-AG-AG-DECISION-001                                                                                                                                                                 | EFFECT | BR-SYS-STATE-008, BR-AG-DATA-003 term_ref: TERM-AG-015, TERM-AG-017, TERM-AG-019                                                                                  | src-tauri/src/ag/task.rs               | 546  |
| BR-SYS-STATE-008-EFFECT-AG-AG-DECISION-002, BR-AG-DATA-003-EFFECT-AG-AG-DECISION-002                                                                                                                                                                 | EFFECT | BR-SYS-STATE-008, BR-AG-DATA-003 term_ref: TERM-AG-015, TERM-AG-017, TERM-AG-019                                                                                  | src-tauri/src/ag/task.rs               | 607  |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-033, BR-AG-STATE-001-EFFECT-AG-AG-TURN-033                                                                                                                                                                        | EFFECT | BR-SYS-STATE-008, BR-AG-STATE-001 term_ref: TERM-AG-015                                                                                                           | src-tauri/src/ag/task.rs               | 760  |
| BR-SYS-STATE-008-GUARD-AG-AG-TASK-003                                                                                                                                                                                                                | GUARD  | BR-SYS-STATE-008 term_ref: TERM-AG-015, TERM-AG-018                                                                                                               | src-tauri/src/ag/task.rs               | 903  |
| BR-SYS-STATE-008-GUARD-AG-AG-TASKPLAN-001, BR-AG-STATE-001-GUARD-AG-AG-TASKPLAN-001, BR-AG-STATE-002-GUARD-AG-AG-TASKPLAN-001                                                                                                                        | GUARD  | BR-SYS-STATE-008, BR-AG-STATE-001, BR-AG-STATE-002 term_ref: TERM-AG-018                                                                                          | src-tauri/src/ag/task.rs               | 1068 |
| BR-SYS-STATE-008-EFFECT-AG-AG-TASKPLAN-001, BR-AG-STATE-001-EFFECT-AG-AG-TASKPLAN-001                                                                                                                                                                | EFFECT | BR-SYS-STATE-008, BR-AG-STATE-001 term_ref: TERM-AG-018                                                                                                           | src-tauri/src/ag/task.rs               | 1127 |
| BR-AG-DATA-003-EFFECT-AG-AG-TURN-037                                                                                                                                                                                                                 | EFFECT | BR-AG-DATA-003 term_ref: TERM-AG-015, TERM-AG-019, TERM-WS-003                                                                                                    | src-tauri/src/ag/task_runtime.rs       | 16   |
| BR-SYS-STATE-008-QUERY-AG-AG-TURN-025                                                                                                                                                                                                                | QUERY  | BR-SYS-STATE-008 term_ref: TERM-AG-015                                                                                                                            | src-tauri/src/ag/task_runtime.rs       | 45   |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-026, BR-AG-DATA-003-EFFECT-AG-AG-TURN-026                                                                                                                                                                         | EFFECT | BR-SYS-STATE-008, BR-AG-DATA-003 term_ref: TERM-AG-015, TERM-AG-017, TERM-AG-019                                                                                  | src-tauri/src/ag/task_runtime.rs       | 78   |
| BR-AG-STATE-002-API-AG-WS-OPERATION-005, BR-AG-DATA-003-API-AG-WS-OPERATION-005, BR-SYS-STATE-008-API-AG-WS-OPERATION-005, BR-WS-STATE-003-API-AG-WS-OPERATION-005, BR-WS-PERSIST-003-API-AG-WS-OPERATION-005, BR-WS-OBS-002-API-AG-WS-OPERATION-005 | API    | BR-AG-STATE-002, BR-AG-DATA-003, BR-SYS-STATE-008, BR-WS-STATE-003, BR-WS-PERSIST-003, BR-WS-OBS-002 term_ref: TERM-AG-015, TERM-AG-017, TERM-AG-019, TERM-WS-003 | src-tauri/src/ag/task_runtime.rs       | 173  |
| BR-SYS-STATE-008-QUERY-AG-AG-TURN-027                                                                                                                                                                                                                | QUERY  | BR-SYS-STATE-008 term_ref: TERM-AG-015, TERM-AG-018                                                                                                               | src-tauri/src/ag/task_runtime.rs       | 376  |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-028                                                                                                                                                                                                               | EFFECT | BR-SYS-STATE-008 term_ref: TERM-AG-015, TERM-AG-018                                                                                                               | src-tauri/src/ag/task_runtime.rs       | 398  |
| BR-AG-DATA-002-DATA-AG-AG-TURN-017, BR-SYS-STATE-008-DATA-AG-AG-TURN-017                                                                                                                                                                             | DATA   | BR-AG-DATA-002, BR-SYS-STATE-008 term_ref: TERM-AG-018                                                                                                            | src-tauri/src/ag/task_runtime.rs       | 673  |
| BR-AG-STATE-002-DATA-AG-AG-TOOL-034, BR-AG-DATA-004-DATA-AG-AG-TOOL-034, BR-SYS-STATE-007-DATA-AG-AG-TOOL-034                                                                                                                                        | DATA   | BR-AG-STATE-002, BR-AG-DATA-004, BR-SYS-STATE-007 term_ref: TERM-AG-012, TERM-AG-013, TERM-AG-014, TERM-AG-015                                                    | src-tauri/src/ag/tool_registry.rs      | 49   |
| BR-AG-STATE-001-RB-AG-AG-TURN-001, BR-SYS-STATE-007-RB-AG-AG-TURN-001                                                                                                                                                                                | RB     | BR-AG-STATE-001, BR-SYS-STATE-007                                                                                                                                 | src-tauri/src/ag/turn.rs               | 124  |
| BR-AG-VERIFY-001-GUARD-AG-AG-TURN-020                                                                                                                                                                                                                | GUARD  | BR-AG-VERIFY-001                                                                                                                                                  | src-tauri/src/ag/turn.rs               | 170  |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-001                                                                                                                                                                                                              | EFFECT | BR-AG-PERSIST-001                                                                                                                                                 | src-tauri/src/ag/turn.rs               | 199  |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-001                                                                                                                                                                                                               | QUERY  | BR-AG-PERSIST-001                                                                                                                                                 | src-tauri/src/ag/turn.rs               | 224  |
| BR-SYS-STATE-007-GUARD-AG-AG-TURN-034, BR-AG-STATE-001-GUARD-AG-AG-TURN-034                                                                                                                                                                          | GUARD  | BR-SYS-STATE-007, BR-AG-STATE-001 term_ref: TERM-AG-001, TERM-AG-015, TERM-AG-019                                                                                 | src-tauri/src/ag/turn_control.rs       | 53   |
| BR-SYS-STATE-007-EFFECT-AG-AG-TURN-034, BR-AG-STATE-001-EFFECT-AG-AG-TURN-034                                                                                                                                                                        | EFFECT | BR-SYS-STATE-007, BR-AG-STATE-001 term_ref: TERM-AG-001, TERM-AG-015                                                                                              | src-tauri/src/ag/turn_control.rs       | 179  |
| BR-AG-DATA-003-EFFECT-AG-AG-TURN-036, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-036                                                                                                                                                                         | EFFECT | BR-AG-DATA-003, BR-SYS-STATE-007 term_ref: TERM-AG-001, TERM-AG-019                                                                                               | src-tauri/src/ag/turn_control.rs       | 270  |
| BR-AG-STATE-001-API-AG-AG-TURN-019                                                                                                                                                                                                                   | API    | BR-AG-STATE-001                                                                                                                                                   | src-tauri/src/ag/turn_control.rs       | 371  |
| BR-SYS-STATE-008-API-AG-AG-TURN-003, BR-AG-DATA-001-API-AG-AG-TURN-003                                                                                                                                                                               | API    | BR-SYS-STATE-008, BR-AG-DATA-001 term_ref: TERM-AG-015, TERM-ED-008                                                                                               | src-tauri/src/ag/turn_control.rs       | 401  |
| BR-AG-DATA-002-DATA-AG-AG-TURN-031                                                                                                                                                                                                                   | DATA   | BR-AG-DATA-002 term_ref: TERM-AG-005                                                                                                                              | src-tauri/src/ag/turn_prompt.rs        | 23   |
| BR-AG-DATA-001-DATA-AG-AG-TURN-030                                                                                                                                                                                                                   | DATA   | BR-AG-DATA-001 term_ref: TERM-REF-003                                                                                                                             | src-tauri/src/ag/turn_reference.rs     | 13   |
| BR-SYS-STATE-007-GUARD-AG-AG-TURN-037                                                                                                                                                                                                                | GUARD  | BR-SYS-STATE-007 term_ref: TERM-AG-013                                                                                                                            | src-tauri/src/ag/turn_runtime.rs       | 25   |
| BR-AG-STATE-001-API-AG-AG-TURN-018, BR-SYS-STATE-007-API-AG-AG-TURN-018                                                                                                                                                                              | API    | BR-AG-STATE-001, BR-SYS-STATE-007 term_ref: TERM-AG-001, TERM-AG-012, TERM-AG-013, TERM-AG-014, TERM-AG-015, TERM-AG-016                                          | src-tauri/src/ag/turn_runtime.rs       | 47   |
| BR-AG-STATE-002-RB-AG-AG-TOOL-018, BR-SYS-STATE-008-RB-AG-AG-TOOL-018                                                                                                                                                                                | RB     | BR-AG-STATE-002, BR-SYS-STATE-008 term_ref: TERM-AG-014, TERM-AG-015, TERM-AG-016                                                                                 | src-tauri/src/ag/turn_tool_loop.rs     | 93   |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-004                                                                                                                                                                                                               | QUERY  | BR-AG-PERSIST-001 term_ref: TERM-AG-001                                                                                                                           | src-tauri/src/de/routes.rs             | 652  |

#### APP-CLOSE

| codes                                     | type   | rules            | 文件                                | 行号  |
| ----------------------------------------- | ------ | ---------------- | --------------------------------- | --- |
| BR-SYS-STATE-006-EFFECT-SYS-APP-CLOSE-001 | EFFECT | BR-SYS-STATE-006 | src/services/closeService.ts      | 230 |
| BR-SYS-STATE-004-GUARD-SYS-APP-CLOSE-003  | GUARD  | BR-SYS-STATE-004 | src/shell/App.tsx                 | 38  |
| BR-SYS-STATE-004-GUARD-SYS-APP-CLOSE-004  | GUARD  | BR-SYS-STATE-004 | src-tauri/src/lib.rs              | 482 |
| BR-SYS-STATE-004-QUERY-SYS-APP-CLOSE-001  | QUERY  | BR-SYS-STATE-004 | src-tauri/src/workspace/routes.rs | 874 |
| BR-SYS-STATE-004-EFFECT-SYS-APP-CLOSE-002 | EFFECT | BR-SYS-STATE-004 | src-tauri/src/workspace/routes.rs | 938 |

#### AST-MEMORY

| codes                                                                              | type   | rules                                                   | 文件                           | 行号  |
| ---------------------------------------------------------------------------------- | ------ | ------------------------------------------------------- | ---------------------------- | --- |
| BR-AST-STATE-001-RB-AST-AST-MEMORY-001                                             | RB     | BR-AST-STATE-001                                        | src-tauri/src/ast/capture.rs | 56  |
| BR-AST-STATE-001-EFFECT-AST-AST-MEMORY-001                                         | EFFECT | BR-AST-STATE-001                                        | src-tauri/src/ast/capture.rs | 91  |
| BR-AST-STATE-001-EFFECT-AST-AST-MEMORY-002                                         | EFFECT | BR-AST-STATE-001                                        | src-tauri/src/ast/capture.rs | 135 |
| BR-SYS-DATA-007-API-AST-AST-MEMORY-001, BR-AST-DATA-001-API-AST-AST-MEMORY-001     | API    | BR-SYS-DATA-007, BR-AST-DATA-001 term_ref: TERM-AST-002 | src-tauri/src/ast/routes.rs  | 6   |
| BR-AST-DATA-001-QUERY-AST-AST-MEMORY-004                                           | QUERY  | BR-AST-DATA-001                                         | src-tauri/src/ast/store.rs   | 22  |
| BR-AST-DATA-001-QUERY-AST-AST-MEMORY-002                                           | QUERY  | BR-AST-DATA-001                                         | src-tauri/src/ast/store.rs   | 50  |
| BR-SYS-DATA-007-QUERY-AST-AST-MEMORY-003, BR-AST-DATA-001-QUERY-AST-AST-MEMORY-003 | QUERY  | BR-SYS-DATA-007, BR-AST-DATA-001 term_ref: TERM-AST-002 | src-tauri/src/ast/store.rs   | 72  |
| BR-AST-DATA-002-QUERY-AST-AST-MEMORY-001                                           | QUERY  | BR-AST-DATA-002                                         | src-tauri/src/ast/supply.rs  | 23  |

#### CBT-BUILD

| codes                                       | type   | rules              | 文件                            | 行号  |
| ------------------------------------------- | ------ | ------------------ | ----------------------------- | --- |
| BR-CBT-DATA-001-DATA-CBT-CBT-BUILD-001      | DATA   | BR-CBT-DATA-001    | src-tauri/src/cbt/apply.rs    | 12  |
| BR-CBT-DATA-001-DATA-CBT-CBT-BUILD-002      | DATA   | BR-CBT-DATA-001    | src-tauri/src/cbt/apply.rs    | 37  |
| BR-CBT-STATE-001-RB-CBT-CBT-BUILD-002       | RB     | BR-CBT-STATE-001   | src-tauri/src/cbt/build.rs    | 59  |
| BR-CBT-PERSIST-001-EFFECT-CBT-CBT-BUILD-001 | EFFECT | BR-CBT-PERSIST-001 | src-tauri/src/cbt/build.rs    | 99  |
| BR-CBT-PERSIST-001-QUERY-CBT-CBT-BUILD-001  | QUERY  | BR-CBT-PERSIST-001 | src-tauri/src/cbt/build.rs    | 128 |
| BR-CBT-STATE-002-GUARD-CBT-CBT-BUILD-001    | GUARD  | BR-CBT-STATE-002   | src-tauri/src/cbt/takeover.rs | 35  |
| BR-CBT-STATE-002-RB-CBT-CBT-BUILD-001       | RB     | BR-CBT-STATE-002   | src-tauri/src/cbt/takeover.rs | 54  |

#### CLOUD-AUTH

| codes                                       | type   | rules             | 文件                            | 行号 |
| ------------------------------------------- | ------ | ----------------- | ----------------------------- | -- |
| BR-ACCT-STATE-001-RB-ACCT-CLOUD-AUTH-001    | RB     | BR-ACCT-STATE-001 | src-tauri/src/acct/session.rs | 24 |
| BR-ACCT-STATE-001-GUARD-ACCT-CLOUD-AUTH-001 | GUARD  | BR-ACCT-STATE-001 | src-tauri/src/acct/session.rs | 38 |
| BR-ACCT-STATE-001-GUARD-ACCT-CLOUD-AUTH-002 | GUARD  | BR-ACCT-STATE-001 | src-tauri/src/acct/session.rs | 52 |
| BR-ACCT-STATE-001-GUARD-ACCT-CLOUD-AUTH-003 | GUARD  | BR-ACCT-STATE-001 | src-tauri/src/acct/session.rs | 70 |
| BR-ACCT-DATA-001-EFFECT-ACCT-CLOUD-AUTH-001 | EFFECT | BR-ACCT-DATA-001  | src-tauri/src/acct/token.rs   | 46 |
| BR-ACCT-DATA-001-EFFECT-ACCT-CLOUD-AUTH-002 | EFFECT | BR-ACCT-DATA-001  | src-tauri/src/acct/token.rs   | 60 |
| BR-ACCT-DATA-001-EFFECT-ACCT-CLOUD-AUTH-003 | EFFECT | BR-ACCT-DATA-001  | src-tauri/src/acct/token.rs   | 73 |
| BR-SYS-STATE-001-RB-SYS-CLOUD-AUTH-001      | RB     | BR-SYS-STATE-001  | src-tauri/src/session/mod.rs  | 40 |

#### CLOUD-GATE

| codes                                                                                | type  | rules                              | 文件                           | 行号  |
| ------------------------------------------------------------------------------------ | ----- | ---------------------------------- | ---------------------------- | --- |
| BR-ENT-VERIFY-001-GUARD-ENT-CLOUD-GATE-002                                           | GUARD | BR-ENT-VERIFY-001                  | src-tauri/src/ent/gateway.rs | 132 |
| BR-ENT-DATA-002-GUARD-ENT-CLOUD-GATE-001                                             | GUARD | BR-ENT-DATA-002                    | src-tauri/src/ent/usage.rs   | 34  |
| BR-SYS-VERIFY-001-GUARD-SYS-CLOUD-GATE-001, BR-SYS-DATA-001-GUARD-SYS-CLOUD-GATE-001 | GUARD | BR-SYS-VERIFY-001, BR-SYS-DATA-001 | src-tauri/src/session/mod.rs | 76  |

#### CLOUD-INVOKE

| codes                                                                                | type   | rules                              | 文件                           | 行号  |
| ------------------------------------------------------------------------------------ | ------ | ---------------------------------- | ---------------------------- | --- |
| BR-ENT-DATA-001-EFFECT-ENT-CLOUD-INVOKE-001                                          | EFFECT | BR-ENT-DATA-001                    | src-tauri/src/ent/byok.rs    | 17  |
| BR-SYS-OBS-001-DATA-ENT-CLOUD-INVOKE-001                                             | DATA   | BR-SYS-OBS-001                     | src-tauri/src/ent/gateway.rs | 70  |
| BR-ENT-STATE-001-RB-ENT-CLOUD-INVOKE-001                                             | RB     | BR-ENT-STATE-001                   | src-tauri/src/ent/gateway.rs | 102 |
| BR-SYS-VERIFY-001-GUARD-SYS-CLOUD-GATE-001, BR-SYS-DATA-001-GUARD-SYS-CLOUD-GATE-001 | GUARD  | BR-SYS-VERIFY-001, BR-SYS-DATA-001 | src-tauri/src/session/mod.rs | 76  |

#### DATA-MIGRATE

| codes                                                                                    | type   | rules                                                                 | 文件                                   | 行号  |
| ---------------------------------------------------------------------------------------- | ------ | --------------------------------------------------------------------- | ------------------------------------ | --- |
| BR-WS-PERSIST-002-EFFECT-WS-DATA-MIGRATE-005                                             | EFFECT | BR-WS-PERSIST-002                                                     | src/services/workspaceService.ts     | 193 |
| BR-WS-PERSIST-002-RB-SYS-DATA-MIGRATE-001                                                | RB     | BR-WS-PERSIST-002                                                     | src/shell/App.tsx                    | 18  |
| BR-WS-PERSIST-002-RB-WS-DATA-MIGRATE-001                                                 | RB     | BR-WS-PERSIST-002                                                     | src/shell/Welcome.tsx                | 18  |
| BR-WS-PERSIST-002-RB-WS-DATA-MIGRATE-002                                                 | RB     | BR-WS-PERSIST-002                                                     | src/shell/Workbench.tsx              | 29  |
| BR-WS-PERSIST-002-EFFECT-WS-DATA-MIGRATE-004                                             | EFFECT | BR-WS-PERSIST-002                                                     | src/store/workspaceShell.ts          | 46  |
| BR-AG-DATA-003-EFFECT-AG-AG-TARGETSET-002, BR-SYS-PERSIST-001-EFFECT-AG-AG-TARGETSET-002 | EFFECT | BR-AG-DATA-003, BR-SYS-PERSIST-001 term_ref: TERM-AG-019, TERM-AG-015 | src-tauri/src/ag/target_set.rs       | 202 |
| BR-SYS-PERSIST-001-EFFECT-SYS-DATA-MIGRATE-001                                           | EFFECT | BR-SYS-PERSIST-001                                                    | src-tauri/src/db/mod.rs              | 433 |
| BR-SYS-STATE-005-EFFECT-SYS-DATA-MIGRATE-002                                             | EFFECT | BR-SYS-STATE-005                                                      | src-tauri/src/de/lifecycle.rs        | 199 |
| BR-SYS-STATE-005-EFFECT-SYS-DATA-MIGRATE-003                                             | EFFECT | BR-SYS-STATE-005                                                      | src-tauri/src/de/lifecycle.rs        | 237 |
| BR-SYS-DATA-002-DATA-SYS-DATA-MIGRATE-001                                                | DATA   | BR-SYS-DATA-002                                                       | src-tauri/src/types/mod.rs           | 9   |
| BR-WS-PERSIST-002-EFFECT-WS-DATA-MIGRATE-001                                             | EFFECT | BR-WS-PERSIST-002                                                     | src-tauri/src/workspace/lifecycle.rs | 34  |
| BR-WS-PERSIST-002-EFFECT-WS-DATA-MIGRATE-002                                             | EFFECT | BR-WS-PERSIST-002                                                     | src-tauri/src/workspace/lifecycle.rs | 71  |
| BR-WS-PERSIST-002-EFFECT-WS-DATA-MIGRATE-003                                             | EFFECT | BR-WS-PERSIST-002                                                     | src-tauri/src/workspace/lifecycle.rs | 99  |
| BR-WS-PERSIST-002-QUERY-WS-DATA-MIGRATE-001                                              | QUERY  | BR-WS-PERSIST-002                                                     | src-tauri/src/workspace/lifecycle.rs | 116 |
| BR-WS-PERSIST-002-QUERY-WS-DATA-MIGRATE-002                                              | QUERY  | BR-WS-PERSIST-002                                                     | src-tauri/src/workspace/lifecycle.rs | 140 |

#### DE-DIFF

| codes                                                                                                                  | type   | rules                                                                                                       | 文件                                     | 行号   |
| ---------------------------------------------------------------------------------------------------------------------- | ------ | ----------------------------------------------------------------------------------------------------------- | -------------------------------------- | ---- |
| BR-DE-DATA-002-RB-DE-DE-DIFF-003                                                                                       | RB     | BR-DE-DATA-002 term_ref: TERM-DE-003, TERM-ED-008                                                           | src/components/de/DiffCard.tsx         | 241  |
| BR-CORE-STATE-001-EFFECT-ED-ED-EDIT-001, BR-CORE-STATE-003-EFFECT-ED-ED-EDIT-001                                       | EFFECT | BR-CORE-STATE-001, BR-CORE-STATE-003                                                                        | src/components/ed/Editor.tsx           | 1723 |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-006, BR-AG-DATA-001-EFFECT-AG-AG-TURN-006                                           | EFFECT | BR-SYS-STATE-008, BR-AG-DATA-001 term_ref: TERM-AG-015, TERM-ED-008                                         | src/services/agService.ts              | 1060 |
| BR-SYS-STATE-008-EFFECT-DE-DE-DIFF-008                                                                                 | EFFECT | BR-SYS-STATE-008                                                                                            | src/services/deService.ts              | 108  |
| BR-DE-DATA-002-EFFECT-DE-DE-DIFF-009                                                                                   | EFFECT | BR-DE-DATA-002 term_ref: TERM-DE-004, TERM-ED-008                                                           | src/services/deService.ts              | 147  |
| BR-CORE-STATE-001-EFFECT-DE-ED-EDIT-010                                                                                | EFFECT | BR-CORE-STATE-001                                                                                           | src/services/deService.ts              | 193  |
| BR-CORE-STATE-003-EFFECT-DE-ED-EDIT-011                                                                                | EFFECT | BR-CORE-STATE-003                                                                                           | src/services/deService.ts              | 252  |
| BR-CORE-STATE-001-API-DE-DE-DIFF-001, BR-CORE-STATE-002-API-DE-DE-DIFF-001, BR-CORE-STATE-003-API-DE-DE-DIFF-001       | API    | BR-CORE-STATE-001, BR-CORE-STATE-002, BR-CORE-STATE-003                                                     | src/services/deService.ts              | 309  |
| BR-DE-DATA-002-RB-ED-DE-DIFF-028                                                                                       | RB     | BR-DE-DATA-002 term_ref: TERM-DE-004, TERM-ED-008                                                           | src/services/edImageOps.ts             | 493  |
| BR-CORE-STATE-001-EFFECT-ED-WS-SAVE-001, BR-CORE-STATE-003-EFFECT-ED-WS-SAVE-001                                       | EFFECT | BR-CORE-STATE-001, BR-CORE-STATE-003                                                                        | src/services/edService.ts              | 68   |
| BR-ED-DATA-007-RB-ED-ED-EDIT-018                                                                                       | RB     | BR-ED-DATA-007 term_ref: TERM-ED-007, TERM-DOC-005                                                          | src/services/edTableOps.ts             | 407  |
| BR-ED-DATA-007-EFFECT-ED-ED-EDIT-014                                                                                   | EFFECT | BR-ED-DATA-007 term_ref: TERM-ED-007, TERM-DOC-005                                                          | src/services/edTableOps.ts             | 438  |
| BR-DE-STATE-001-RB-DE-DE-DIFF-002                                                                                      | RB     | BR-DE-STATE-001 term_ref: TERM-DE-001, TERM-DE-003                                                          | src/store/deStore.ts                   | 257  |
| BR-AG-STATE-002-EFFECT-AG-AG-TOOL-017, BR-AG-DATA-003-EFFECT-AG-AG-TOOL-017                                            | EFFECT | BR-AG-STATE-002, BR-AG-DATA-003                                                                             | src-tauri/src/ag/edit_runtime.rs       | 19   |
| BR-ED-DATA-007-GUARD-AG-AG-TOOL-018                                                                                    | GUARD  | BR-ED-DATA-007 term_ref: TERM-ED-007                                                                        | src-tauri/src/ag/edit_table_runtime.rs | 8    |
| BR-AG-DATA-003-GUARD-AG-AG-TARGETSET-003                                                                               | GUARD  | BR-AG-DATA-003 term_ref: TERM-AG-019, TERM-DE-002                                                           | src-tauri/src/ag/target_set.rs         | 272  |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-032, BR-AG-DATA-001-EFFECT-AG-AG-TURN-032                                           | EFFECT | BR-SYS-STATE-008, BR-AG-DATA-001 term_ref: TERM-AG-015, TERM-ED-008                                         | src-tauri/src/ag/task.rs               | 148  |
| BR-SYS-STATE-008-GUARD-AG-AG-TURN-005, BR-AG-DATA-001-GUARD-AG-AG-TURN-005                                             | GUARD  | BR-SYS-STATE-008, BR-AG-DATA-001 term_ref: TERM-AG-015, TERM-ED-008                                         | src-tauri/src/ag/task.rs               | 197  |
| BR-SYS-STATE-008-API-AG-AG-TURN-003, BR-AG-DATA-001-API-AG-AG-TURN-003                                                 | API    | BR-SYS-STATE-008, BR-AG-DATA-001 term_ref: TERM-AG-015, TERM-ED-008                                         | src-tauri/src/ag/turn_control.rs       | 401  |
| BR-DE-STATE-002-GUARD-DE-DE-DIFF-001                                                                                   | GUARD  | BR-DE-STATE-002                                                                                             | src-tauri/src/de/expiry.rs             | 34   |
| BR-DE-DATA-003-GUARD-DE-DE-DIFF-003                                                                                    | GUARD  | BR-DE-DATA-003                                                                                              | src-tauri/src/de/expiry.rs             | 62   |
| BR-DE-DATA-002-DATA-DE-DE-DIFF-001                                                                                     | DATA   | BR-DE-DATA-002                                                                                              | src-tauri/src/de/generate.rs           | 41   |
| BR-AG-DATA-003-GUARD-DE-DE-DIFF-004                                                                                    | GUARD  | BR-AG-DATA-003 term_ref: TERM-AG-019                                                                        | src-tauri/src/de/generate.rs           | 61   |
| BR-DE-DATA-001-GUARD-DE-DE-DIFF-005                                                                                    | GUARD  | BR-DE-DATA-001                                                                                              | src-tauri/src/de/generate.rs           | 87   |
| BR-DE-STATE-002-QUERY-DE-DE-DIFF-001                                                                                   | QUERY  | BR-DE-STATE-002                                                                                             | src-tauri/src/de/lifecycle.rs          | 31   |
| BR-DE-STATE-002-GUARD-DE-DE-DIFF-002                                                                                   | GUARD  | BR-DE-STATE-002                                                                                             | src-tauri/src/de/lifecycle.rs          | 58   |
| BR-DE-DATA-003-GUARD-DE-DE-DIFF-006                                                                                    | GUARD  | BR-DE-DATA-003                                                                                              | src-tauri/src/de/lifecycle.rs          | 86   |
| BR-DE-STATE-002-EFFECT-DE-DE-DIFF-001                                                                                  | EFFECT | BR-DE-STATE-002                                                                                             | src-tauri/src/de/lifecycle.rs          | 108  |
| BR-SYS-DATA-005-EFFECT-DE-DE-DIFF-003                                                                                  | EFFECT | BR-SYS-DATA-005                                                                                             | src-tauri/src/de/lifecycle.rs          | 132  |
| BR-SYS-DATA-005-EFFECT-DE-DE-DIFF-004                                                                                  | EFFECT | BR-SYS-DATA-005                                                                                             | src-tauri/src/de/lifecycle.rs          | 155  |
| BR-DE-STATE-001-RB-DE-DE-DIFF-001                                                                                      | RB     | BR-DE-STATE-001                                                                                             | src-tauri/src/de/mod.rs                | 71   |
| BR-CORE-STATE-001-EFFECT-DE-DE-DIFF-005                                                                                | EFFECT | BR-CORE-STATE-001                                                                                           | src-tauri/src/de/routes.rs             | 123  |
| BR-CORE-STATE-003-EFFECT-DE-DE-DIFF-006                                                                                | EFFECT | BR-CORE-STATE-003                                                                                           | src-tauri/src/de/routes.rs             | 257  |
| BR-CORE-STATE-002-EFFECT-DE-DE-DIFF-007                                                                                | EFFECT | BR-CORE-STATE-002                                                                                           | src-tauri/src/de/routes.rs             | 323  |
| BR-DE-DATA-002-IO-DE-DE-DIFF-002                                                                                       | IO     | BR-DE-DATA-002 term_ref: TERM-DE-004                                                                        | src-tauri/src/de/store.rs              | 30   |
| BR-DE-DATA-002-QUERY-DE-DE-DIFF-002                                                                                    | QUERY  | BR-DE-DATA-002 term_ref: TERM-DE-004                                                                        | src-tauri/src/de/store.rs              | 65   |
| BR-DE-DATA-002-EFFECT-WS-DE-DIFF-004                                                                                   | EFFECT | BR-DE-DATA-002 term_ref: TERM-DE-004, TERM-ED-008                                                           | src-tauri/src/workspace/diff_apply.rs  | 219  |
| BR-WS-STATE-003-API-WS-WS-OPERATION-004, BR-WS-DATA-003-API-WS-WS-OPERATION-004, BR-WS-OBS-002-API-WS-WS-OPERATION-004 | API    | BR-WS-STATE-003, BR-WS-DATA-003, BR-WS-OBS-002 term_ref: TERM-WS-003, TERM-WS-007, TERM-ED-008, TERM-DE-003 | src-tauri/src/workspace/routes.rs      | 1413 |

#### DOC-CONVERT

| codes                                                                               | type  | rules                                                              | 文件                                       | 行号   |
| ----------------------------------------------------------------------------------- | ----- | ------------------------------------------------------------------ | ---------------------------------------- | ---- |
| BR-DOC-DATA-003-DATA-DOC-DOC-CONVERT-011                                            | DATA  | BR-DOC-DATA-003                                                    | src-tauri/src/doc/block_backfill.rs      | 26   |
| BR-DOC-DATA-003-DATA-DOC-DOC-CONVERT-003                                            | DATA  | BR-DOC-DATA-003                                                    | src-tauri/src/doc/block_backfill.rs      | 212  |
| BR-DOC-DATA-003-GUARD-DOC-DOC-CONVERT-015                                           | GUARD | BR-DOC-DATA-003                                                    | src-tauri/src/doc/block_backfill.rs      | 259  |
| BR-DOC-STATE-001-RB-DOC-DOC-CONVERT-001                                             | RB    | BR-DOC-STATE-001                                                   | src-tauri/src/doc/convert.rs             | 56   |
| BR-DOC-DATA-001-GUARD-DOC-DOC-CONVERT-001                                           | GUARD | BR-DOC-DATA-001                                                    | src-tauri/src/doc/convert.rs             | 89   |
| BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-008                                            | DATA  | BR-DOC-DATA-002 term_ref: TERM-DOC-005                             | src-tauri/src/doc/docx_cell_shading.rs   | 16   |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-006                                              | IO    | BR-DOC-DATA-002 term_ref: TERM-DOC-005                             | src-tauri/src/doc/docx_cell_shading.rs   | 427  |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-007                                              | IO    | BR-DOC-DATA-002 term_ref: TERM-DOC-005                             | src-tauri/src/doc/docx_cell_shading.rs   | 487  |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-016, BR-DOC-DATA-003-IO-DOC-DOC-CONVERT-016      | IO    | BR-DOC-DATA-002, BR-DOC-DATA-003                                   | src-tauri/src/doc/edit_chain.rs          | 88   |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-004                                              | IO    | BR-DOC-DATA-002                                                    | src-tauri/src/doc/edit_chain.rs          | 157  |
| BR-DOC-DATA-001-DATA-DOC-DOC-CONVERT-006                                            | DATA  | BR-DOC-DATA-001                                                    | src-tauri/src/doc/font_trace.rs          | 243  |
| BR-DOC-DATA-001-DATA-DOC-DOC-CONVERT-007                                            | DATA  | BR-DOC-DATA-001                                                    | src-tauri/src/doc/font_trace.rs          | 318  |
| BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-009, BR-DOC-DATA-003-DATA-DOC-DOC-CONVERT-009  | DATA  | BR-DOC-DATA-002, BR-DOC-DATA-003                                   | src-tauri/src/doc/image_media.rs         | 37   |
| BR-DOC-DATA-002-GUARD-DOC-DOC-CONVERT-011, BR-WS-DATA-001-GUARD-DOC-DOC-CONVERT-011 | GUARD | BR-DOC-DATA-002, BR-WS-DATA-001 term_ref: TERM-ED-008, TERM-WS-001 | src-tauri/src/doc/image_media.rs         | 334  |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-015, BR-WS-DATA-001-IO-DOC-DOC-CONVERT-015       | IO    | BR-DOC-DATA-002, BR-WS-DATA-001 term_ref: TERM-ED-008, TERM-WS-001 | src-tauri/src/doc/image_media.rs         | 568  |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-012                                              | IO    | BR-DOC-DATA-002 term_ref: TERM-ED-008                              | src-tauri/src/doc/image_media.rs         | 670  |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-013, BR-DOC-DATA-003-IO-DOC-DOC-CONVERT-013      | IO    | BR-DOC-DATA-002, BR-DOC-DATA-003 term_ref: TERM-ED-008             | src-tauri/src/doc/image_media.rs         | 766  |
| BR-DOC-DATA-002-GUARD-DOC-DOC-CONVERT-014                                           | GUARD | BR-DOC-DATA-002 term_ref: TERM-ED-008                              | src-tauri/src/doc/image_media.rs         | 1083 |
| BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-003                                              | IO    | BR-DOC-DATA-001                                                    | src-tauri/src/doc/libreoffice_runtime.rs | 77   |
| BR-DOC-DATA-001-DATA-DOC-DOC-CONVERT-004                                            | DATA  | BR-DOC-DATA-001                                                    | src-tauri/src/doc/libreoffice_runtime.rs | 106  |
| BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-005                                              | IO    | BR-DOC-DATA-001                                                    | src-tauri/src/doc/libreoffice_runtime.rs | 244  |
| BR-DOC-DATA-001-DATA-DOC-DOC-CONVERT-012                                            | DATA  | BR-DOC-DATA-001                                                    | src-tauri/src/doc/pandoc_runtime.rs      | 32   |
| BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-001                                              | IO    | BR-DOC-DATA-001                                                    | src-tauri/src/doc/pandoc_runtime.rs      | 62   |
| BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-002                                              | IO    | BR-DOC-DATA-001                                                    | src-tauri/src/doc/pandoc_runtime.rs      | 88   |
| BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-001                                            | DATA  | BR-DOC-DATA-002                                                    | src-tauri/src/doc/style_subset.rs        | 53   |
| BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-002                                            | DATA  | BR-DOC-DATA-002                                                    | src-tauri/src/doc/style_subset.rs        | 79   |
| BR-DOC-DATA-001-GUARD-DOC-DOC-CONVERT-002                                           | GUARD | BR-DOC-DATA-001                                                    | src-tauri/src/doc/tool_runtime.rs        | 118  |

#### ED-DECOR

| codes                                                                            | type   | rules                                              | 文件                                                      | 行号   |
| -------------------------------------------------------------------------------- | ------ | -------------------------------------------------- | ------------------------------------------------------- | ---- |
| BR-CORE-STATE-001-EFFECT-ED-ED-EDIT-001, BR-CORE-STATE-003-EFFECT-ED-ED-EDIT-001 | EFFECT | BR-CORE-STATE-001, BR-CORE-STATE-003               | src/components/ed/Editor.tsx                            | 1723 |
| BR-ED-STATE-002-RB-ED-ED-DECOR-001                                               | RB     | BR-ED-STATE-002 term_ref: TERM-ED-002, TERM-ED-003 | src/components/ed/extensions/DiffDecorationExtension.ts | 155  |
| BR-CORE-STATE-003-EFFECT-DE-ED-EDIT-011                                          | EFFECT | BR-CORE-STATE-003                                  | src/services/deService.ts                               | 252  |
| BR-CORE-STATE-003-EFFECT-DE-DE-DIFF-006                                          | EFFECT | BR-CORE-STATE-003                                  | src-tauri/src/de/routes.rs                              | 257  |

#### ED-EDIT

| codes                                                                                                                  | type   | rules                                                                                                       | 文件                                               | 行号   |
| ---------------------------------------------------------------------------------------------------------------------- | ------ | ----------------------------------------------------------------------------------------------------------- | ------------------------------------------------ | ---- |
| BR-CORE-STATE-001-EFFECT-ED-ED-EDIT-001, BR-CORE-STATE-003-EFFECT-ED-ED-EDIT-001                                       | EFFECT | BR-CORE-STATE-001, BR-CORE-STATE-003                                                                        | src/components/ed/Editor.tsx                     | 1723 |
| BR-ED-DATA-001-RB-ED-ED-EDIT-024, BR-ED-DATA-003-RB-ED-ED-EDIT-024                                                     | RB     | BR-ED-DATA-001, BR-ED-DATA-003                                                                              | src/components/ed/ImageMoveControls.tsx          | 39   |
| BR-ED-DATA-007-RB-ED-ED-EDIT-019                                                                                       | RB     | BR-ED-DATA-007 term_ref: TERM-ED-007                                                                        | src/components/ed/TableMoveControls.tsx          | 47   |
| BR-ED-DATA-001-DATA-ED-ED-EDIT-001                                                                                     | DATA   | BR-ED-DATA-001 term_ref: TERM-ED-001                                                                        | src/components/ed/extensions/BlockIdExtension.ts | 96   |
| BR-ED-DATA-003-GUARD-ED-ED-EDIT-019                                                                                    | GUARD  | BR-ED-DATA-003                                                                                              | src/components/ed/extensions/Pagination.ts       | 263  |
| BR-ED-DATA-003-GUARD-ED-ED-EDIT-020                                                                                    | GUARD  | BR-ED-DATA-003                                                                                              | src/components/ed/extensions/Pagination.ts       | 585  |
| BR-ED-DATA-003-GUARD-ED-ED-EDIT-018                                                                                    | GUARD  | BR-ED-DATA-003                                                                                              | src/components/ed/extensions/Pagination.ts       | 636  |
| BR-ED-DATA-003-RB-ED-ED-EDIT-005                                                                                       | RB     | BR-ED-DATA-003                                                                                              | src/components/ed/extensions/Pagination.ts       | 893  |
| BR-ED-DATA-003-EFFECT-ED-ED-EDIT-020, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-020                                            | EFFECT | BR-ED-DATA-003, BR-ED-STATE-001 term_ref: TERM-ED-008                                                       | src/components/ed/extensions/ResizableImage.ts   | 50   |
| BR-ED-DATA-007-DATA-ED-ED-EDIT-018                                                                                     | DATA   | BR-ED-DATA-007 term_ref: TERM-ED-007, TERM-DOC-005                                                          | src/components/ed/extensions/TableCellShading.ts | 37   |
| BR-ED-DATA-003-RB-ED-ED-EDIT-017                                                                                       | RB     | BR-ED-DATA-003                                                                                              | src/components/ed/extensions/paginationFlow.ts   | 57   |
| BR-ED-DATA-006-DATA-ED-ED-EDIT-014                                                                                     | DATA   | BR-ED-DATA-006                                                                                              | src/components/ed/markdown/markdownCodec.ts      | 82   |
| BR-ED-VERIFY-001-GUARD-ED-ED-EDIT-015, BR-ED-DATA-005-GUARD-ED-ED-EDIT-015                                             | GUARD  | BR-ED-VERIFY-001, BR-ED-DATA-005                                                                            | src/components/ed/markdown/markdownCodec.ts      | 170  |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-006, BR-AG-DATA-001-EFFECT-AG-AG-TURN-006                                           | EFFECT | BR-SYS-STATE-008, BR-AG-DATA-001 term_ref: TERM-AG-015, TERM-ED-008                                         | src/services/agService.ts                        | 1060 |
| BR-SYS-STATE-008-EFFECT-DE-DE-DIFF-008                                                                                 | EFFECT | BR-SYS-STATE-008                                                                                            | src/services/deService.ts                        | 108  |
| BR-CORE-STATE-001-EFFECT-DE-ED-EDIT-010                                                                                | EFFECT | BR-CORE-STATE-001                                                                                           | src/services/deService.ts                        | 193  |
| BR-CORE-STATE-003-EFFECT-DE-ED-EDIT-011                                                                                | EFFECT | BR-CORE-STATE-003                                                                                           | src/services/deService.ts                        | 252  |
| BR-CORE-STATE-001-API-DE-DE-DIFF-001, BR-CORE-STATE-002-API-DE-DE-DIFF-001, BR-CORE-STATE-003-API-DE-DE-DIFF-001       | API    | BR-CORE-STATE-001, BR-CORE-STATE-002, BR-CORE-STATE-003                                                     | src/services/deService.ts                        | 309  |
| BR-ED-DATA-003-RB-ED-ED-EDIT-006                                                                                       | RB     | BR-ED-DATA-003                                                                                              | src/services/docService.ts                       | 107  |
| BR-ED-DATA-003-RB-ED-ED-EDIT-026                                                                                       | RB     | BR-ED-DATA-003 term_ref: TERM-ED-008                                                                        | src/services/docService.ts                       | 184  |
| BR-ED-DATA-003-EFFECT-ED-ED-EDIT-026                                                                                   | EFFECT | BR-ED-DATA-003 term_ref: TERM-ED-008                                                                        | src/services/docService.ts                       | 203  |
| BR-ED-DATA-001-QUERY-ED-ED-EDIT-024, BR-ED-DATA-003-QUERY-ED-ED-EDIT-024                                               | QUERY  | BR-ED-DATA-001, BR-ED-DATA-003 term_ref: TERM-ED-008                                                        | src/services/edImageOps.ts                       | 228  |
| BR-SYS-DATA-006-RB-ED-ED-EDIT-027, BR-ED-DATA-003-RB-ED-ED-EDIT-027                                                    | RB     | BR-SYS-DATA-006, BR-ED-DATA-003 term_ref: TERM-SYS-002, TERM-ED-008                                         | src/services/edImageOps.ts                       | 256  |
| BR-ED-DATA-001-QUERY-ED-ED-EDIT-021, BR-ED-DATA-003-QUERY-ED-ED-EDIT-021                                               | QUERY  | BR-ED-DATA-001, BR-ED-DATA-003                                                                              | src/services/edImageOps.ts                       | 570  |
| BR-ED-DATA-001-EFFECT-ED-ED-EDIT-022, BR-ED-DATA-003-EFFECT-ED-ED-EDIT-022, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-022      | EFFECT | BR-ED-DATA-001, BR-ED-DATA-003, BR-ED-STATE-001                                                             | src/services/edImageOps.ts                       | 599  |
| BR-ED-DATA-001-EFFECT-ED-ED-EDIT-023, BR-ED-DATA-003-EFFECT-ED-ED-EDIT-023, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-023      | EFFECT | BR-ED-DATA-001, BR-ED-DATA-003, BR-ED-STATE-001                                                             | src/services/edImageOps.ts                       | 640  |
| BR-ED-DATA-001-EFFECT-ED-ED-EDIT-025, BR-ED-DATA-003-EFFECT-ED-ED-EDIT-025, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-025      | EFFECT | BR-ED-DATA-001, BR-ED-DATA-003, BR-ED-STATE-001 term_ref: TERM-ED-008                                       | src/services/edImageOps.ts                       | 731  |
| BR-ED-DATA-004-QUERY-ED-ED-EDIT-010                                                                                    | QUERY  | BR-ED-DATA-004 term_ref: TERM-ED-004, TERM-ED-005                                                           | src/services/edLocate.ts                         | 231  |
| BR-ED-STATE-001-EFFECT-ED-ED-EDIT-012                                                                                  | EFFECT | BR-ED-STATE-001 term_ref: TERM-ED-001, TERM-ED-004                                                          | src/services/edLocate.ts                         | 345  |
| BR-ED-STATE-001-EFFECT-ED-ED-EDIT-011                                                                                  | EFFECT | BR-ED-STATE-001 term_ref: TERM-ED-001                                                                       | src/services/edLocate.ts                         | 384  |
| BR-ED-DATA-004-QUERY-ED-ED-EDIT-007                                                                                    | QUERY  | BR-ED-DATA-004 term_ref: TERM-ED-004, TERM-ED-005                                                           | src/services/edLocate.ts                         | 414  |
| BR-CORE-STATE-001-EFFECT-ED-WS-SAVE-001, BR-CORE-STATE-003-EFFECT-ED-WS-SAVE-001                                       | EFFECT | BR-CORE-STATE-001, BR-CORE-STATE-003                                                                        | src/services/edService.ts                        | 68   |
| BR-ED-DATA-002-RB-ED-ED-EDIT-012, BR-ED-DATA-006-RB-ED-ED-EDIT-012                                                     | RB     | BR-ED-DATA-002, BR-ED-DATA-006                                                                              | src/services/edService.ts                        | 247  |
| BR-ED-STATE-003-EFFECT-ED-ED-EDIT-013                                                                                  | EFFECT | BR-ED-STATE-003                                                                                             | src/services/edService.ts                        | 304  |
| BR-ED-VERIFY-001-GUARD-ED-ED-EDIT-016, BR-ED-DATA-006-GUARD-ED-ED-EDIT-016                                             | GUARD  | BR-ED-VERIFY-001, BR-ED-DATA-006                                                                            | src/services/edService.ts                        | 338  |
| BR-ED-DATA-005-RB-ED-ED-EDIT-008                                                                                       | RB     | BR-ED-DATA-005 term_ref: TERM-ED-006, TERM-ED-004                                                           | src/services/edStyleOps.ts                       | 243  |
| BR-ED-DATA-007-RB-ED-ED-EDIT-018                                                                                       | RB     | BR-ED-DATA-007 term_ref: TERM-ED-007, TERM-DOC-005                                                          | src/services/edTableOps.ts                       | 407  |
| BR-ED-DATA-007-EFFECT-ED-ED-EDIT-014                                                                                   | EFFECT | BR-ED-DATA-007 term_ref: TERM-ED-007, TERM-DOC-005                                                          | src/services/edTableOps.ts                       | 438  |
| BR-ED-STATE-001-RB-ED-ED-EDIT-002                                                                                      | RB     | BR-ED-STATE-001                                                                                             | src/store/edStore.ts                             | 260  |
| BR-ED-DATA-005-QUERY-ED-ED-EDIT-009                                                                                    | QUERY  | BR-ED-DATA-005 term_ref: TERM-ED-006, TERM-ED-004                                                           | src-tauri/src/ag/blocks.rs                       | 751  |
| BR-AG-STATE-002-EFFECT-AG-AG-TOOL-017, BR-AG-DATA-003-EFFECT-AG-AG-TOOL-017                                            | EFFECT | BR-AG-STATE-002, BR-AG-DATA-003                                                                             | src-tauri/src/ag/edit_runtime.rs                 | 19   |
| BR-ED-DATA-007-GUARD-AG-AG-TOOL-018                                                                                    | GUARD  | BR-ED-DATA-007 term_ref: TERM-ED-007                                                                        | src-tauri/src/ag/edit_table_runtime.rs           | 8    |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-032, BR-AG-DATA-001-EFFECT-AG-AG-TURN-032                                           | EFFECT | BR-SYS-STATE-008, BR-AG-DATA-001 term_ref: TERM-AG-015, TERM-ED-008                                         | src-tauri/src/ag/task.rs                         | 148  |
| BR-SYS-STATE-008-GUARD-AG-AG-TURN-005, BR-AG-DATA-001-GUARD-AG-AG-TURN-005                                             | GUARD  | BR-SYS-STATE-008, BR-AG-DATA-001 term_ref: TERM-AG-015, TERM-ED-008                                         | src-tauri/src/ag/task.rs                         | 197  |
| BR-SYS-STATE-008-GUARD-AG-AG-TURN-006, BR-AG-DATA-001-GUARD-AG-AG-TURN-006                                             | GUARD  | BR-SYS-STATE-008, BR-AG-DATA-001 term_ref: TERM-AG-015, TERM-ED-008                                         | src-tauri/src/ag/task.rs                         | 279  |
| BR-SYS-STATE-008-API-AG-AG-TURN-003, BR-AG-DATA-001-API-AG-AG-TURN-003                                                 | API    | BR-SYS-STATE-008, BR-AG-DATA-001 term_ref: TERM-AG-015, TERM-ED-008                                         | src-tauri/src/ag/turn_control.rs                 | 401  |
| BR-CORE-STATE-001-EFFECT-DE-DE-DIFF-005                                                                                | EFFECT | BR-CORE-STATE-001                                                                                           | src-tauri/src/de/routes.rs                       | 123  |
| BR-CORE-STATE-003-EFFECT-DE-DE-DIFF-006                                                                                | EFFECT | BR-CORE-STATE-003                                                                                           | src-tauri/src/de/routes.rs                       | 257  |
| BR-WS-STATE-003-API-WS-WS-OPERATION-004, BR-WS-DATA-003-API-WS-WS-OPERATION-004, BR-WS-OBS-002-API-WS-WS-OPERATION-004 | API    | BR-WS-STATE-003, BR-WS-DATA-003, BR-WS-OBS-002 term_ref: TERM-WS-003, TERM-WS-007, TERM-ED-008, TERM-DE-003 | src-tauri/src/workspace/routes.rs                | 1413 |

#### IPC-CALL

| codes                                                                          | type   | rules                                                   | 文件                          | 行号  |
| ------------------------------------------------------------------------------ | ------ | ------------------------------------------------------- | --------------------------- | --- |
| BR-SYS-DATA-007-API-AST-AST-MEMORY-001, BR-AST-DATA-001-API-AST-AST-MEMORY-001 | API    | BR-SYS-DATA-007, BR-AST-DATA-001 term_ref: TERM-AST-002 | src-tauri/src/ast/routes.rs | 6   |
| BR-SYS-OBS-002-DATA-SYS-IPC-CALL-002                                           | DATA   | BR-SYS-OBS-002                                          | src-tauri/src/ipc/mod.rs    | 14  |
| BR-SYS-STATE-002-DATA-SYS-IPC-CALL-003                                         | DATA   | BR-SYS-STATE-002                                        | src-tauri/src/ipc/mod.rs    | 76  |
| BR-SYS-DATA-003-API-SYS-IPC-CALL-001                                           | API    | BR-SYS-DATA-003                                         | src-tauri/src/ipc/mod.rs    | 186 |
| BR-SYS-DATA-003-API-SYS-IPC-CALL-005                                           | API    | BR-SYS-DATA-003                                         | src-tauri/src/lib.rs        | 3   |
| BR-SYS-DATA-003-API-SYS-IPC-CALL-002                                           | API    | BR-SYS-DATA-003 term_ref: TERM-WS-001                   | src-tauri/src/lib.rs        | 293 |
| BR-SYS-STATE-003-EFFECT-SYS-IPC-CALL-001                                       | EFFECT | BR-SYS-STATE-003                                        | src-tauri/src/sm_events.rs  | 48  |

#### KB-RETRIEVE

| codes                                    | type   | rules          | 文件                           | 行号  |
| ---------------------------------------- | ------ | -------------- | ---------------------------- | --- |
| BR-KB-DATA-002-QUERY-KB-KB-RETRIEVE-001  | QUERY  | BR-KB-DATA-002 | src-tauri/src/kb/retrieve.rs | 12  |
| BR-KB-DATA-001-EFFECT-KB-KB-RETRIEVE-001 | EFFECT | BR-KB-DATA-001 | src-tauri/src/kb/store.rs    | 31  |
| BR-KB-DATA-001-QUERY-KB-KB-RETRIEVE-002  | QUERY  | BR-KB-DATA-001 | src-tauri/src/kb/store.rs    | 84  |
| BR-KB-DATA-001-EFFECT-KB-KB-RETRIEVE-002 | EFFECT | BR-KB-DATA-001 | src-tauri/src/kb/store.rs    | 106 |

#### PCG-AUDIT

| codes                                                                          | type | rules                                | 文件                                   | 行号 |
| ------------------------------------------------------------------------------ | ---- | ------------------------------------ | ------------------------------------ | -- |
| BR-PCG-VERIFY-001-RB-PCG-PCG-AUDIT-001, BR-PCG-VERIFY-002-RB-PCG-PCG-AUDIT-001 | RB   | BR-PCG-VERIFY-001, BR-PCG-VERIFY-002 | scripts/audit_product_capability.mjs | 5  |

#### PCG-GENERATE

| codes                                                                             | type | rules                             | 文件                                            | 行号 |
| --------------------------------------------------------------------------------- | ---- | --------------------------------- | --------------------------------------------- | -- |
| BR-PCG-DATA-001-RB-PCG-PCG-GENERATE-001, BR-PCG-STATE-001-RB-PCG-PCG-GENERATE-001 | RB   | BR-PCG-DATA-001, BR-PCG-STATE-001 | scripts/generate_function_ledger.mjs          | 14 |
| BR-PCG-DATA-001-RB-PCG-PCG-GENERATE-004                                           | RB   | BR-PCG-DATA-001, BR-PCG-STATE-001 | scripts/generate_function_tree.mjs            | 22 |
| BR-PCG-DATA-001-RB-PCG-PCG-GENERATE-001, BR-PCG-STATE-001-RB-PCG-PCG-GENERATE-001 | RB   | BR-PCG-DATA-001, BR-PCG-STATE-001 | scripts/generate_product_capability_atlas.mjs | 11 |
| BR-PCG-DATA-001-RB-PCG-PCG-GENERATE-003                                           | RB   | BR-PCG-DATA-001, BR-PCG-STATE-001 | scripts/seed_function_audit.mjs               | 28 |

#### PRV-VIEW

| codes                                                                    | type  | rules                            | 文件                                 | 行号  |
| ------------------------------------------------------------------------ | ----- | -------------------------------- | ---------------------------------- | --- |
| BR-PRV-DATA-001-GUARD-PRV-PRV-VIEW-003                                   | GUARD | BR-PRV-DATA-001                  | src/components/prv/PrvPanel.tsx    | 41  |
| BR-PRV-DATA-001-RB-PRV-PRV-VIEW-004, BR-PRV-DATA-002-RB-PRV-PRV-VIEW-004 | RB    | BR-PRV-DATA-001, BR-PRV-DATA-002 | src/components/prv/PrvToolbar.tsx  | 23  |
| BR-PRV-DATA-002-IO-PRV-PRV-VIEW-004                                      | IO    | BR-PRV-DATA-002                  | src-tauri/src/doc/table_preview.rs | 48  |
| BR-PRV-DATA-002-IO-PRV-PRV-VIEW-001                                      | IO    | BR-PRV-DATA-002                  | src-tauri/src/prv/mod.rs           | 84  |
| BR-PRV-DATA-002-IO-PRV-PRV-VIEW-002                                      | IO    | BR-PRV-DATA-002                  | src-tauri/src/prv/mod.rs           | 124 |
| BR-PRV-DATA-002-IO-PRV-PRV-VIEW-003                                      | IO    | BR-PRV-DATA-002                  | src-tauri/src/prv/mod.rs           | 182 |

#### REF-INJECT

| codes                                                                                                                       | type  | rules                                                                                    | 文件                                 | 行号  |
| --------------------------------------------------------------------------------------------------------------------------- | ----- | ---------------------------------------------------------------------------------------- | ---------------------------------- | --- |
| BR-REF-DATA-004-DATA-AG-REF-INJECT-033                                                                                      | DATA  | BR-REF-DATA-004 term_ref: TERM-REF-004, TERM-REF-003                                     | src/components/ag/Composer.tsx     | 40  |
| BR-REF-DATA-003-DATA-REF-REF-INJECT-005                                                                                     | DATA  | BR-REF-DATA-003 term_ref: TERM-REF-003                                                   | src/services/referenceDrag.ts      | 2   |
| BR-REF-DATA-003-GUARD-REF-REF-INJECT-004                                                                                    | GUARD | BR-REF-DATA-003 term_ref: TERM-REF-003                                                   | src/services/referenceDrag.ts      | 49  |
| BR-REF-DATA-003-GUARD-REF-REF-INJECT-005                                                                                    | GUARD | BR-REF-DATA-003 term_ref: TERM-REF-003                                                   | src/services/referenceDrag.ts      | 71  |
| BR-REF-DATA-003-DATA-REF-REF-INJECT-006, BR-SYS-DATA-002-DATA-REF-REF-INJECT-006                                            | DATA  | BR-REF-DATA-003, BR-SYS-DATA-002 term_ref: TERM-REF-003                                  | src/services/referencePayload.ts   | 2   |
| BR-REF-DATA-003-GUARD-REF-REF-INJECT-006                                                                                    | GUARD | BR-REF-DATA-003 term_ref: TERM-REF-003                                                   | src/services/referencePayload.ts   | 122 |
| BR-REF-DATA-003-GUARD-REF-REF-INJECT-007                                                                                    | GUARD | BR-REF-DATA-003 term_ref: TERM-REF-003                                                   | src/services/referencePayload.ts   | 183 |
| BR-REF-DATA-004-RB-REF-REF-INJECT-001                                                                                       | RB    | BR-REF-DATA-004 term_ref: TERM-REF-004, TERM-REF-003                                     | src/services/referencePayload.ts   | 439 |
| BR-SYS-DATA-007-API-AST-AST-MEMORY-001, BR-AST-DATA-001-API-AST-AST-MEMORY-001                                              | API   | BR-SYS-DATA-007, BR-AST-DATA-001 term_ref: TERM-AST-002                                  | src-tauri/src/ast/routes.rs        | 6   |
| BR-SYS-DATA-007-QUERY-AST-AST-MEMORY-003, BR-AST-DATA-001-QUERY-AST-AST-MEMORY-003                                          | QUERY | BR-SYS-DATA-007, BR-AST-DATA-001 term_ref: TERM-AST-002                                  | src-tauri/src/ast/store.rs         | 72  |
| BR-REF-DATA-003-DATA-REF-REF-INJECT-003                                                                                     | DATA  | BR-REF-DATA-003 term_ref: TERM-REF-003                                                   | src-tauri/src/reference/context.rs | 36  |
| BR-REF-VERIFY-001-GUARD-REF-REF-INJECT-002                                                                                  | GUARD | BR-REF-VERIFY-001 term_ref: TERM-REF-003                                                 | src-tauri/src/reference/context.rs | 54  |
| BR-REF-DATA-003-DATA-REF-REF-INJECT-002, BR-REF-VERIFY-001-DATA-REF-REF-INJECT-002, BR-SYS-DATA-007-DATA-REF-REF-INJECT-002 | DATA  | BR-REF-DATA-003, BR-REF-VERIFY-001, BR-SYS-DATA-007 term_ref: TERM-REF-003, TERM-REF-005 | src-tauri/src/reference/context.rs | 102 |
| BR-REF-DATA-003-DATA-REF-REF-INJECT-004                                                                                     | DATA  | BR-REF-DATA-003 term_ref: TERM-REF-003                                                   | src-tauri/src/reference/context.rs | 242 |
| BR-REF-VERIFY-001-GUARD-REF-REF-INJECT-001                                                                                  | GUARD | BR-REF-VERIFY-001                                                                        | src-tauri/src/reference/inject.rs  | 14  |
| BR-REF-DATA-002-DATA-REF-REF-INJECT-001                                                                                     | DATA  | BR-REF-DATA-002                                                                          | src-tauri/src/reference/inject.rs  | 54  |
| BR-REF-DATA-001-GUARD-REF-REF-INJECT-003                                                                                    | GUARD | BR-REF-DATA-001                                                                          | src-tauri/src/reference/resolve.rs | 34  |
| BR-REF-DATA-001-API-REF-REF-INJECT-001                                                                                      | API   | BR-REF-DATA-001 term_ref: TERM-REF-001                                                   | src-tauri/src/reference/routes.rs  | 75  |
| BR-REF-DATA-001-API-REF-REF-INJECT-002                                                                                      | API   | BR-REF-DATA-001 term_ref: TERM-REF-001                                                   | src-tauri/src/reference/routes.rs  | 112 |
| BR-REF-VERIFY-001-API-REF-REF-INJECT-003                                                                                    | API   | BR-REF-VERIFY-001 term_ref: TERM-REF-001                                                 | src-tauri/src/reference/routes.rs  | 159 |
| BR-REF-DATA-002-API-REF-REF-INJECT-004, BR-REF-VERIFY-001-API-REF-REF-INJECT-004                                            | API   | BR-REF-DATA-002, BR-REF-VERIFY-001 term_ref: TERM-REF-002                                | src-tauri/src/reference/routes.rs  | 187 |

#### TPL-FLOW

| codes                                    | type   | rules            | 文件                            | 行号  |
| ---------------------------------------- | ------ | ---------------- | ----------------------------- | --- |
| BR-TPL-DATA-001-EFFECT-TPL-TPL-FLOW-001  | EFFECT | BR-TPL-DATA-001  | src-tauri/src/tpl/template.rs | 43  |
| BR-TPL-DATA-001-QUERY-TPL-TPL-FLOW-001   | QUERY  | BR-TPL-DATA-001  | src-tauri/src/tpl/template.rs | 71  |
| BR-TPL-DATA-001-EFFECT-TPL-TPL-FLOW-002  | EFFECT | BR-TPL-DATA-001  | src-tauri/src/tpl/template.rs | 91  |
| BR-TPL-STATE-001-RB-TPL-TPL-FLOW-001     | RB     | BR-TPL-STATE-001 | src-tauri/src/tpl/workflow.rs | 49  |
| BR-TPL-STATE-001-EFFECT-TPL-TPL-FLOW-003 | EFFECT | BR-TPL-STATE-001 | src-tauri/src/tpl/workflow.rs | 109 |
| BR-TPL-STATE-001-QUERY-TPL-TPL-FLOW-002  | QUERY  | BR-TPL-STATE-001 | src-tauri/src/tpl/workflow.rs | 138 |

#### WS-DIFF-APPLY

| codes                                                                                                            | type   | rules                                                   | 文件                                    | 行号  |
| ---------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------------------------------- | ------------------------------------- | --- |
| BR-CORE-STATE-001-API-DE-DE-DIFF-001, BR-CORE-STATE-002-API-DE-DE-DIFF-001, BR-CORE-STATE-003-API-DE-DE-DIFF-001 | API    | BR-CORE-STATE-001, BR-CORE-STATE-002, BR-CORE-STATE-003 | src/services/deService.ts             | 309 |
| BR-CORE-STATE-002-EFFECT-DE-DE-DIFF-007                                                                          | EFFECT | BR-CORE-STATE-002                                       | src-tauri/src/de/routes.rs            | 323 |
| BR-WS-STATE-001-RB-WS-WS-DIFF-APPLY-001                                                                          | RB     | BR-WS-STATE-001                                         | src-tauri/src/workspace/diff_apply.rs | 45  |
| BR-WS-STATE-001-EFFECT-WS-WS-DIFF-APPLY-001                                                                      | EFFECT | BR-WS-STATE-001                                         | src-tauri/src/workspace/diff_apply.rs | 76  |
| BR-WS-STATE-001-EFFECT-WS-WS-DIFF-APPLY-002                                                                      | EFFECT | BR-WS-STATE-001                                         | src-tauri/src/workspace/diff_apply.rs | 359 |
| BR-WS-STATE-001-EFFECT-WS-WS-DIFF-APPLY-003                                                                      | EFFECT | BR-WS-STATE-001                                         | src-tauri/src/workspace/diff_apply.rs | 432 |
| BR-WS-STATE-001-QUERY-WS-WS-DIFF-APPLY-001                                                                       | QUERY  | BR-WS-STATE-001                                         | src-tauri/src/workspace/diff_apply.rs | 511 |

#### WS-FS

| codes                               | type   | rules                                              | 文件                                   | 行号   |
| ----------------------------------- | ------ | -------------------------------------------------- | ------------------------------------ | ---- |
| BR-WS-DATA-001-EFFECT-WS-WS-FS-001  | EFFECT | BR-WS-DATA-001 term_ref: TERM-WS-001               | src-tauri/src/workspace/fileops.rs   | 16   |
| BR-WS-DATA-001-EFFECT-WS-WS-FS-004  | EFFECT | BR-WS-DATA-001 term_ref: TERM-WS-001               | src-tauri/src/workspace/fileops.rs   | 43   |
| BR-WS-DATA-001-EFFECT-WS-WS-FS-002  | EFFECT | BR-WS-DATA-001 term_ref: TERM-WS-001               | src-tauri/src/workspace/fileops.rs   | 70   |
| BR-WS-DATA-001-EFFECT-WS-WS-FS-007  | EFFECT | BR-WS-DATA-001                                     | src-tauri/src/workspace/fileops.rs   | 100  |
| BR-WS-DATA-001-EFFECT-WS-WS-FS-003  | EFFECT | BR-WS-DATA-001 term_ref: TERM-WS-001               | src-tauri/src/workspace/fileops.rs   | 134  |
| BR-WS-DATA-001-EFFECT-WS-WS-FS-008  | EFFECT | BR-WS-DATA-001                                     | src-tauri/src/workspace/listing.rs   | 148  |
| BR-WS-DATA-001-QUERY-WS-WS-FS-002   | QUERY  | BR-WS-DATA-001 term_ref: TERM-WS-001               | src-tauri/src/workspace/listing.rs   | 174  |
| BR-WS-DATA-001-QUERY-WS-WS-FS-006   | QUERY  | BR-WS-DATA-001 term_ref: TERM-WS-001               | src-tauri/src/workspace/listing.rs   | 582  |
| BR-WS-DATA-001-QUERY-WS-WS-FS-005   | QUERY  | BR-WS-DATA-001 term_ref: TERM-WS-001               | src-tauri/src/workspace/listing.rs   | 703  |
| BR-WS-DATA-001-GUARD-WS-WS-FS-001   | GUARD  | BR-WS-DATA-001                                     | src-tauri/src/workspace/mod.rs       | 46   |
| BR-WS-STATE-009-EFFECT-WS-WS-FS-009 | EFFECT | BR-WS-STATE-009 term_ref: TERM-WS-003, TERM-WS-006 | src-tauri/src/workspace/operation.rs | 403  |
| BR-WS-STATE-009-EFFECT-WS-WS-FS-010 | EFFECT | BR-WS-STATE-009 term_ref: TERM-WS-003, TERM-WS-006 | src-tauri/src/workspace/operation.rs | 463  |
| BR-WS-OBS-001-RB-WS-WS-FS-001       | RB     | BR-WS-OBS-001                                      | src-tauri/src/workspace/routes.rs    | 1053 |

#### WS-OPERATION

| codes                                                                                                                                                                                                                                                | type   | rules                                                                                                                                                             | 文件                                   | 行号   |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------ | ---- |
| BR-AG-DATA-003-EFFECT-AG-AG-TURN-037                                                                                                                                                                                                                 | EFFECT | BR-AG-DATA-003 term_ref: TERM-AG-015, TERM-AG-019, TERM-WS-003                                                                                                    | src-tauri/src/ag/task_runtime.rs     | 16   |
| BR-AG-STATE-002-API-AG-WS-OPERATION-005, BR-AG-DATA-003-API-AG-WS-OPERATION-005, BR-SYS-STATE-008-API-AG-WS-OPERATION-005, BR-WS-STATE-003-API-AG-WS-OPERATION-005, BR-WS-PERSIST-003-API-AG-WS-OPERATION-005, BR-WS-OBS-002-API-AG-WS-OPERATION-005 | API    | BR-AG-STATE-002, BR-AG-DATA-003, BR-SYS-STATE-008, BR-WS-STATE-003, BR-WS-PERSIST-003, BR-WS-OBS-002 term_ref: TERM-AG-015, TERM-AG-017, TERM-AG-019, TERM-WS-003 | src-tauri/src/ag/task_runtime.rs     | 173  |
| BR-WS-STATE-003-RB-WS-WS-OPERATION-001, BR-WS-PERSIST-003-RB-WS-WS-OPERATION-001                                                                                                                                                                     | RB     | BR-WS-STATE-003, BR-WS-PERSIST-003 term_ref: TERM-WS-003, TERM-WS-004                                                                                             | src-tauri/src/workspace/operation.rs | 139  |
| BR-WS-DATA-003-EFFECT-WS-WS-OPERATION-001                                                                                                                                                                                                            | EFFECT | BR-WS-DATA-003 term_ref: TERM-WS-004                                                                                                                              | src-tauri/src/workspace/operation.rs | 169  |
| BR-WS-PERSIST-003-EFFECT-WS-WS-OPERATION-002                                                                                                                                                                                                         | EFFECT | BR-WS-PERSIST-003 term_ref: TERM-WS-003                                                                                                                           | src-tauri/src/workspace/operation.rs | 1513 |
| BR-WS-STATE-003-API-WS-WS-OPERATION-001, BR-WS-DATA-003-API-WS-WS-OPERATION-001, BR-WS-OBS-002-API-WS-WS-OPERATION-001                                                                                                                               | API    | BR-WS-STATE-003, BR-WS-DATA-003, BR-WS-OBS-002 term_ref: TERM-WS-003, TERM-WS-004                                                                                 | src-tauri/src/workspace/operation.rs | 1955 |
| BR-WS-STATE-003-EFFECT-WS-WS-OPERATION-003, BR-WS-DATA-003-EFFECT-WS-WS-OPERATION-003, BR-WS-OBS-002-EFFECT-WS-WS-OPERATION-003, BR-WS-PERSIST-003-EFFECT-WS-WS-OPERATION-003                                                                        | EFFECT | BR-WS-STATE-003, BR-WS-DATA-003, BR-WS-OBS-002, BR-WS-PERSIST-003 term_ref: TERM-WS-003, TERM-WS-004, TERM-WS-006                                                 | src-tauri/src/workspace/operation.rs | 2537 |
| BR-WS-STATE-003-API-WS-WS-OPERATION-002, BR-WS-DATA-003-API-WS-WS-OPERATION-002, BR-WS-OBS-002-API-WS-WS-OPERATION-002                                                                                                                               | API    | BR-WS-STATE-003, BR-WS-DATA-003, BR-WS-OBS-002 term_ref: TERM-WS-003, TERM-WS-004                                                                                 | src-tauri/src/workspace/routes.rs    | 1157 |
| BR-WS-STATE-003-API-WS-WS-OPERATION-004, BR-WS-DATA-003-API-WS-WS-OPERATION-004, BR-WS-OBS-002-API-WS-WS-OPERATION-004                                                                                                                               | API    | BR-WS-STATE-003, BR-WS-DATA-003, BR-WS-OBS-002 term_ref: TERM-WS-003, TERM-WS-007, TERM-ED-008, TERM-DE-003                                                       | src-tauri/src/workspace/routes.rs    | 1413 |

#### WS-SAVE

| codes                                                                                                              | type   | rules                                                   | 文件                                | 行号  |
| ------------------------------------------------------------------------------------------------------------------ | ------ | ------------------------------------------------------- | --------------------------------- | --- |
| BR-CORE-STATE-003-EFFECT-DE-ED-EDIT-011                                                                            | EFFECT | BR-CORE-STATE-003                                       | src/services/deService.ts         | 252 |
| BR-CORE-STATE-001-API-DE-DE-DIFF-001, BR-CORE-STATE-002-API-DE-DE-DIFF-001, BR-CORE-STATE-003-API-DE-DE-DIFF-001   | API    | BR-CORE-STATE-001, BR-CORE-STATE-002, BR-CORE-STATE-003 | src/services/deService.ts         | 309 |
| BR-CORE-STATE-001-EFFECT-ED-WS-SAVE-001, BR-CORE-STATE-003-EFFECT-ED-WS-SAVE-001                                   | EFFECT | BR-CORE-STATE-001, BR-CORE-STATE-003                    | src/services/edService.ts         | 68  |
| BR-CORE-STATE-001-EFFECT-DE-DE-DIFF-005                                                                            | EFFECT | BR-CORE-STATE-001                                       | src-tauri/src/de/routes.rs        | 123 |
| BR-CORE-STATE-003-EFFECT-DE-DE-DIFF-006                                                                            | EFFECT | BR-CORE-STATE-003                                       | src-tauri/src/de/routes.rs        | 257 |
| BR-WS-PERSIST-001-EFFECT-WS-WS-SAVE-003, BR-WS-OBS-001-EFFECT-WS-WS-SAVE-003, BR-WS-DATA-001-EFFECT-WS-WS-SAVE-003 | EFFECT | BR-WS-PERSIST-001, BR-WS-OBS-001, BR-WS-DATA-001        | src-tauri/src/workspace/routes.rs | 337 |
| BR-WS-PERSIST-001-EFFECT-WS-WS-SAVE-001                                                                            | EFFECT | BR-WS-PERSIST-001                                       | src-tauri/src/workspace/save.rs   | 51  |
| BR-WS-PERSIST-001-RB-WS-WS-SAVE-002                                                                                | RB     | BR-WS-PERSIST-001                                       | src-tauri/src/workspace/save.rs   | 103 |

#### WS-TIMELINE

| codes                                                                                 | type   | rules                                                                | 文件                                   | 行号  |
| ------------------------------------------------------------------------------------- | ------ | -------------------------------------------------------------------- | ------------------------------------ | --- |
| BR-WS-DATA-003-EFFECT-WS-WS-TIMELINE-003, BR-WS-PERSIST-003-EFFECT-WS-WS-TIMELINE-003 | EFFECT | BR-WS-DATA-003, BR-WS-PERSIST-003 term_ref: TERM-WS-005, TERM-WS-006 | src-tauri/src/workspace/operation.rs | 342 |
| BR-WS-STATE-009-EFFECT-WS-WS-FS-009                                                   | EFFECT | BR-WS-STATE-009 term_ref: TERM-WS-003, TERM-WS-006                   | src-tauri/src/workspace/operation.rs | 403 |
| BR-WS-STATE-009-EFFECT-WS-WS-FS-010                                                   | EFFECT | BR-WS-STATE-009 term_ref: TERM-WS-003, TERM-WS-006                   | src-tauri/src/workspace/operation.rs | 463 |
| BR-WS-DATA-002-EFFECT-WS-WS-TIMELINE-001                                              | EFFECT | BR-WS-DATA-002                                                       | src-tauri/src/workspace/timeline.rs  | 47  |
| BR-WS-DATA-002-QUERY-WS-WS-TIMELINE-001                                               | QUERY  | BR-WS-DATA-002                                                       | src-tauri/src/workspace/timeline.rs  | 91  |
| BR-WS-DATA-002-EFFECT-WS-WS-TIMELINE-002                                              | EFFECT | BR-WS-DATA-002                                                       | src-tauri/src/workspace/timeline.rs  | 138 |
| BR-WS-STATE-002-QUERY-WS-WS-TIMELINE-002                                              | QUERY  | BR-WS-STATE-002                                                      | src-tauri/src/workspace/timeline.rs  | 202 |
| BR-WS-STATE-002-RB-WS-WS-TIMELINE-001                                                 | RB     | BR-WS-STATE-002                                                      | src-tauri/src/workspace/timeline.rs  | 242 |

### 规则视图

#### BR-ACCT-DATA-001

| codes                                       | type   | chain      | 文件                          | 行号 |
| ------------------------------------------- | ------ | ---------- | --------------------------- | -- |
| BR-ACCT-DATA-001-EFFECT-ACCT-CLOUD-AUTH-001 | EFFECT | CLOUD-AUTH | src-tauri/src/acct/token.rs | 46 |
| BR-ACCT-DATA-001-EFFECT-ACCT-CLOUD-AUTH-002 | EFFECT | CLOUD-AUTH | src-tauri/src/acct/token.rs | 60 |
| BR-ACCT-DATA-001-EFFECT-ACCT-CLOUD-AUTH-003 | EFFECT | CLOUD-AUTH | src-tauri/src/acct/token.rs | 73 |

#### BR-ACCT-STATE-001

| codes                                       | type  | chain      | 文件                            | 行号 |
| ------------------------------------------- | ----- | ---------- | ----------------------------- | -- |
| BR-ACCT-STATE-001-RB-ACCT-CLOUD-AUTH-001    | RB    | CLOUD-AUTH | src-tauri/src/acct/session.rs | 24 |
| BR-ACCT-STATE-001-GUARD-ACCT-CLOUD-AUTH-001 | GUARD | CLOUD-AUTH | src-tauri/src/acct/session.rs | 38 |
| BR-ACCT-STATE-001-GUARD-ACCT-CLOUD-AUTH-002 | GUARD | CLOUD-AUTH | src-tauri/src/acct/session.rs | 52 |
| BR-ACCT-STATE-001-GUARD-ACCT-CLOUD-AUTH-003 | GUARD | CLOUD-AUTH | src-tauri/src/acct/session.rs | 70 |

#### BR-AG-DATA-001

| codes                                                                                                                                          | type  | chain   | 文件                                    | 行号  |
| ---------------------------------------------------------------------------------------------------------------------------------------------- | ----- | ------- | ------------------------------------- | --- |
| BR-AG-DATA-001-QUERY-AG-AG-TOOL-011, BR-ED-DATA-003-QUERY-AG-AG-TOOL-011                                                                       | QUERY | AG-TOOL | src-tauri/src/ag/blocks.rs            | 190 |
| BR-AG-DATA-001-QUERY-AG-AG-TURN-010, BR-DOC-DATA-003-QUERY-AG-AG-TURN-010                                                                      | QUERY | AG-TURN | src-tauri/src/ag/blocks.rs            | 415 |
| BR-AG-DATA-001-DATA-AG-AG-TURN-001                                                                                                             | DATA  | AG-TURN | src-tauri/src/ag/context.rs           | 845 |
| BR-AG-DATA-001-DATA-AG-AG-TURN-011, BR-AG-DATA-002-DATA-AG-AG-TURN-011, BR-AG-DATA-004-DATA-AG-AG-TURN-011, BR-AG-DATA-005-DATA-AG-AG-TURN-011 | DATA  | AG-TURN | src-tauri/src/ag/prompt_runtime.rs    | 256 |
| BR-AG-DATA-001-QUERY-AG-AG-TURN-029                                                                                                            | QUERY | AG-TURN | src-tauri/src/ag/reference_runtime.rs | 10  |
| BR-AG-STATE-002-QUERY-AG-AG-TOOL-012, BR-AG-DATA-001-QUERY-AG-AG-TOOL-012, BR-WS-DATA-001-QUERY-AG-AG-TOOL-012                                 | QUERY | AG-TOOL | src-tauri/src/ag/tool_exec.rs         | 38  |

#### BR-AG-DATA-001 term_ref: TERM-AG-015

| codes                                                                        | type   | chain                     | 文件                               | 行号   |
| ---------------------------------------------------------------------------- | ------ | ------------------------- | -------------------------------- | ---- |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-006, BR-AG-DATA-001-EFFECT-AG-AG-TURN-006 | EFFECT | AG-TURN, DE-DIFF, ED-EDIT | src/services/agService.ts        | 1060 |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-032, BR-AG-DATA-001-EFFECT-AG-AG-TURN-032 | EFFECT | AG-TURN, DE-DIFF, ED-EDIT | src-tauri/src/ag/task.rs         | 148  |
| BR-SYS-STATE-008-GUARD-AG-AG-TURN-005, BR-AG-DATA-001-GUARD-AG-AG-TURN-005   | GUARD  | AG-TURN, DE-DIFF, ED-EDIT | src-tauri/src/ag/task.rs         | 197  |
| BR-SYS-STATE-008-GUARD-AG-AG-TURN-006, BR-AG-DATA-001-GUARD-AG-AG-TURN-006   | GUARD  | AG-TURN, ED-EDIT          | src-tauri/src/ag/task.rs         | 279  |
| BR-SYS-STATE-008-API-AG-AG-TURN-003, BR-AG-DATA-001-API-AG-AG-TURN-003       | API    | AG-TURN, DE-DIFF, ED-EDIT | src-tauri/src/ag/turn_control.rs | 401  |

#### BR-AG-DATA-001 term_ref: TERM-DOC-003

| codes                                | type   | chain   | 文件                        | 行号  |
| ------------------------------------ | ------ | ------- | ------------------------- | --- |
| BR-AG-DATA-001-EFFECT-AG-AG-TURN-005 | EFFECT | AG-TURN | src/services/agService.ts | 298 |

#### BR-AG-DATA-001 term_ref: TERM-REF-003

| codes                              | type | chain   | 文件                                    | 行号 |
| ---------------------------------- | ---- | ------- | ------------------------------------- | -- |
| BR-AG-DATA-001-RB-AG-AG-TURN-017   | RB   | AG-TURN | src-tauri/src/ag/reference_runtime.rs | 71 |
| BR-AG-DATA-001-DATA-AG-AG-TURN-030 | DATA | AG-TURN | src-tauri/src/ag/turn_reference.rs    | 13 |

#### BR-AG-DATA-002

| codes                                                                                                                                          | type | chain     | 文件                                 | 行号  |
| ---------------------------------------------------------------------------------------------------------------------------------------------- | ---- | --------- | ---------------------------------- | --- |
| BR-AG-DATA-002-DATA-AG-AG-ASSIST-011                                                                                                           | DATA | AG-ASSIST | src-tauri/src/ag/assist_prompt.rs  | 19  |
| BR-AG-DATA-002-DATA-AG-AG-TURN-016                                                                                                             | DATA | AG-TURN   | src-tauri/src/ag/context.rs        | 915 |
| BR-AG-DATA-001-DATA-AG-AG-TURN-011, BR-AG-DATA-002-DATA-AG-AG-TURN-011, BR-AG-DATA-004-DATA-AG-AG-TURN-011, BR-AG-DATA-005-DATA-AG-AG-TURN-011 | DATA | AG-TURN   | src-tauri/src/ag/prompt_runtime.rs | 256 |
| BR-AG-DATA-002-DATA-AG-AG-TURN-017, BR-SYS-STATE-008-DATA-AG-AG-TURN-017                                                                       | DATA | AG-TURN   | src-tauri/src/ag/task_runtime.rs   | 673 |

#### BR-AG-DATA-002 term_ref: TERM-AG-005

| codes                              | type | chain   | 文件                              | 行号 |
| ---------------------------------- | ---- | ------- | ------------------------------- | -- |
| BR-AG-DATA-002-DATA-AG-AG-TURN-031 | DATA | AG-TURN | src-tauri/src/ag/turn_prompt.rs | 23 |

#### BR-AG-DATA-003

| codes                                                                                                                                                                                                                                                | type   | chain                          | 文件                                | 行号  |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------ | --------------------------------- | --- |
| BR-AG-DATA-003-RB-AG-AG-TOOL-016, BR-ED-DATA-001-RB-AG-AG-TOOL-016, BR-DE-DATA-001-RB-AG-AG-TOOL-016                                                                                                                                                 | RB     | AG-TOOL                        | src-tauri/src/ag/block_ops.rs     | 299 |
| BR-AG-STATE-002-EFFECT-AG-AG-TOOL-017, BR-AG-DATA-003-EFFECT-AG-AG-TOOL-017                                                                                                                                                                          | EFFECT | AG-TOOL, DE-DIFF, ED-EDIT      | src-tauri/src/ag/edit_runtime.rs  | 19  |
| BR-AG-DATA-003-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-007-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-008-DATA-AG-AG-TARGETSET-001                                                                                                                        | DATA   | AG-TURN, AG-TOOL               | src-tauri/src/ag/target_set.rs    | 32  |
| BR-AG-DATA-003-EFFECT-AG-AG-TARGETSET-002, BR-SYS-PERSIST-001-EFFECT-AG-AG-TARGETSET-002                                                                                                                                                             | EFFECT | AG-TURN, AG-TOOL, DATA-MIGRATE | src-tauri/src/ag/target_set.rs    | 202 |
| BR-AG-STATE-002-API-AG-WS-OPERATION-005, BR-AG-DATA-003-API-AG-WS-OPERATION-005, BR-SYS-STATE-008-API-AG-WS-OPERATION-005, BR-WS-STATE-003-API-AG-WS-OPERATION-005, BR-WS-PERSIST-003-API-AG-WS-OPERATION-005, BR-WS-OBS-002-API-AG-WS-OPERATION-005 | API    | AG-TURN, AG-TOOL, WS-OPERATION | src-tauri/src/ag/task_runtime.rs  | 173 |
| BR-AG-DATA-003-GUARD-AG-AG-TOOL-033, BR-AG-STATE-002-GUARD-AG-AG-TOOL-033                                                                                                                                                                            | GUARD  | AG-TOOL                        | src-tauri/src/ag/tool_registry.rs | 311 |
| BR-AG-DATA-003-EFFECT-AG-AG-TURN-036, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-036                                                                                                                                                                         | EFFECT | AG-TURN                        | src-tauri/src/ag/turn_control.rs  | 270 |

#### BR-AG-DATA-003 term_ref: TERM-AG-015

| codes                                                                                | type   | chain                 | 文件                               | 行号  |
| ------------------------------------------------------------------------------------ | ------ | --------------------- | -------------------------------- | --- |
| BR-SYS-STATE-008-EFFECT-AG-AG-DECISION-001, BR-AG-DATA-003-EFFECT-AG-AG-DECISION-001 | EFFECT | AG-TURN, AG-TOOL      | src-tauri/src/ag/task.rs         | 546 |
| BR-SYS-STATE-008-EFFECT-AG-AG-DECISION-002, BR-AG-DATA-003-EFFECT-AG-AG-DECISION-002 | EFFECT | AG-TURN, AG-TOOL      | src-tauri/src/ag/task.rs         | 607 |
| BR-AG-DATA-003-EFFECT-AG-AG-TURN-037                                                 | EFFECT | AG-TURN, WS-OPERATION | src-tauri/src/ag/task_runtime.rs | 16  |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-026, BR-AG-DATA-003-EFFECT-AG-AG-TURN-026         | EFFECT | AG-TURN               | src-tauri/src/ag/task_runtime.rs | 78  |

#### BR-AG-DATA-003 term_ref: TERM-AG-019

| codes                                    | type  | chain            | 文件                             | 行号  |
| ---------------------------------------- | ----- | ---------------- | ------------------------------ | --- |
| BR-AG-DATA-003-GUARD-AG-AG-TARGETSET-003 | GUARD | AG-TOOL, DE-DIFF | src-tauri/src/ag/target_set.rs | 272 |
| BR-AG-DATA-003-GUARD-DE-DE-DIFF-004      | GUARD | DE-DIFF          | src-tauri/src/de/generate.rs   | 61  |

#### BR-AG-DATA-004

| codes                                                                                                                                          | type | chain            | 文件                                 | 行号  |
| ---------------------------------------------------------------------------------------------------------------------------------------------- | ---- | ---------------- | ---------------------------------- | --- |
| BR-AG-DATA-001-DATA-AG-AG-TURN-011, BR-AG-DATA-002-DATA-AG-AG-TURN-011, BR-AG-DATA-004-DATA-AG-AG-TURN-011, BR-AG-DATA-005-DATA-AG-AG-TURN-011 | DATA | AG-TURN          | src-tauri/src/ag/prompt_runtime.rs | 256 |
| BR-AG-STATE-002-DATA-AG-AG-TOOL-034, BR-AG-DATA-004-DATA-AG-AG-TOOL-034, BR-SYS-STATE-007-DATA-AG-AG-TOOL-034                                  | DATA | AG-TURN, AG-TOOL | src-tauri/src/ag/tool_registry.rs  | 49  |

#### BR-AG-DATA-004 term_ref: TERM-AG-005

| codes                              | type | chain   | 文件                          | 行号   |
| ---------------------------------- | ---- | ------- | --------------------------- | ---- |
| BR-AG-DATA-004-DATA-AG-AG-TURN-003 | DATA | AG-TURN | src-tauri/src/ag/context.rs | 1054 |
| BR-AG-DATA-004-DATA-AG-AG-TURN-005 | DATA | AG-TURN | src-tauri/src/ag/context.rs | 1114 |

#### BR-AG-DATA-004 term_ref: TERM-AG-012

| codes                            | type | chain            | 文件                          | 行号  |
| -------------------------------- | ---- | ---------------- | --------------------------- | --- |
| BR-AG-DATA-004-RB-AG-AG-TURN-015 | RB   | AG-TURN, AG-TOOL | src-tauri/src/ag/context.rs | 420 |

#### BR-AG-DATA-005

| codes                              | type | chain   | 文件                          | 行号  |
| ---------------------------------- | ---- | ------- | --------------------------- | --- |
| BR-AG-DATA-005-DATA-AG-AG-TURN-004 | DATA | AG-TURN | src-tauri/src/ag/context.rs | 966 |

#### BR-AG-DATA-005 term_ref: TERM-AG-005

| codes                                                                                                                                          | type | chain   | 文件                                 | 行号  |
| ---------------------------------------------------------------------------------------------------------------------------------------------- | ---- | ------- | ---------------------------------- | --- |
| BR-AG-DATA-001-DATA-AG-AG-TURN-011, BR-AG-DATA-002-DATA-AG-AG-TURN-011, BR-AG-DATA-004-DATA-AG-AG-TURN-011, BR-AG-DATA-005-DATA-AG-AG-TURN-011 | DATA | AG-TURN | src-tauri/src/ag/prompt_runtime.rs | 256 |

#### BR-AG-DATA-006 term_ref: TERM-AG-010

| codes                                         | type   | chain            | 文件                      | 行号  |
| --------------------------------------------- | ------ | ---------------- | ----------------------- | --- |
| BR-AG-DATA-006-EFFECT-AG-AG-MODEL-CATALOG-006 | EFFECT | AG-MODEL-CATALOG | src/store/modelStore.ts | 179 |

#### BR-AG-DATA-007 term_ref: TERM-AG-010

| codes                                        | type  | chain                                | 文件                      | 行号  |
| -------------------------------------------- | ----- | ------------------------------------ | ----------------------- | --- |
| BR-AG-DATA-007-QUERY-AG-AG-MODEL-CATALOG-013 | QUERY | AG-MODEL-CATALOG, AG-ASSIST, AG-TURN | src/store/modelStore.ts | 162 |

#### BR-AG-PERSIST-001

| codes                                   | type   | chain   | 文件                                 | 行号  |
| --------------------------------------- | ------ | ------- | ---------------------------------- | --- |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-020  | QUERY  | AG-TURN | src-tauri/src/ag/history_routes.rs | 7   |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-021 | EFFECT | AG-TURN | src-tauri/src/ag/history_routes.rs | 23  |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-022 | EFFECT | AG-TURN | src-tauri/src/ag/history_routes.rs | 72  |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-023  | QUERY  | AG-TURN | src-tauri/src/ag/history_routes.rs | 121 |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-024 | EFFECT | AG-TURN | src-tauri/src/ag/history_routes.rs | 144 |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-001 | EFFECT | AG-TURN | src-tauri/src/ag/turn.rs           | 199 |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-001  | QUERY  | AG-TURN | src-tauri/src/ag/turn.rs           | 224 |

#### BR-AG-PERSIST-001 term_ref: TERM-AG-001

| codes                                   | type   | chain   | 文件                          | 行号  |
| --------------------------------------- | ------ | ------- | --------------------------- | --- |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-003 | EFFECT | AG-TURN | src-tauri/src/ag/history.rs | 295 |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-003  | QUERY  | AG-TURN | src-tauri/src/ag/history.rs | 357 |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-031 | EFFECT | AG-TURN | src-tauri/src/ag/history.rs | 396 |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-004  | QUERY  | AG-TURN | src-tauri/src/de/routes.rs  | 652 |

#### BR-AG-PERSIST-001 term_ref: TERM-AG-009

| codes                                   | type   | chain   | 文件                          | 行号  |
| --------------------------------------- | ------ | ------- | --------------------------- | --- |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-030 | EFFECT | AG-TURN | src-tauri/src/ag/history.rs | 163 |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-002  | QUERY  | AG-TURN | src-tauri/src/ag/history.rs | 245 |

#### BR-AG-STATE-001

| codes                                                                                                                         | type  | chain            | 文件                               | 行号   |
| ----------------------------------------------------------------------------------------------------------------------------- | ----- | ---------------- | -------------------------------- | ---- |
| BR-SYS-STATE-008-RB-AG-AG-TASK-001, BR-AG-STATE-001-RB-AG-AG-TASK-001                                                         | RB    | AG-TURN          | src-tauri/src/ag/task.rs         | 52   |
| BR-SYS-STATE-008-GUARD-AG-AG-TASKPLAN-001, BR-AG-STATE-001-GUARD-AG-AG-TASKPLAN-001, BR-AG-STATE-002-GUARD-AG-AG-TASKPLAN-001 | GUARD | AG-TURN          | src-tauri/src/ag/task.rs         | 1068 |
| BR-AG-STATE-001-RB-AG-AG-TURN-001, BR-SYS-STATE-007-RB-AG-AG-TURN-001                                                         | RB    | AG-TURN          | src-tauri/src/ag/turn.rs         | 124  |
| BR-AG-STATE-001-API-AG-AG-TURN-019                                                                                            | API   | AG-TURN          | src-tauri/src/ag/turn_control.rs | 371  |
| BR-AG-STATE-001-API-AG-AG-TURN-018, BR-SYS-STATE-007-API-AG-AG-TURN-018                                                       | API   | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_runtime.rs | 47   |

#### BR-AG-STATE-001 term_ref: TERM-AG-001

| codes                                                                         | type   | chain   | 文件                               | 行号   |
| ----------------------------------------------------------------------------- | ------ | ------- | -------------------------------- | ---- |
| BR-SYS-STATE-007-EFFECT-AG-AG-TURN-035, BR-AG-STATE-001-EFFECT-AG-AG-TURN-035 | EFFECT | AG-TURN | src/services/agService.ts        | 1445 |
| BR-SYS-STATE-007-GUARD-AG-AG-TURN-034, BR-AG-STATE-001-GUARD-AG-AG-TURN-034   | GUARD  | AG-TURN | src-tauri/src/ag/turn_control.rs | 53   |
| BR-SYS-STATE-007-EFFECT-AG-AG-TURN-034, BR-AG-STATE-001-EFFECT-AG-AG-TURN-034 | EFFECT | AG-TURN | src-tauri/src/ag/turn_control.rs | 179  |

#### BR-AG-STATE-001 term_ref: TERM-AG-015

| codes                                                                         | type   | chain   | 文件                       | 行号  |
| ----------------------------------------------------------------------------- | ------ | ------- | ------------------------ | --- |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-033, BR-AG-STATE-001-EFFECT-AG-AG-TURN-033 | EFFECT | AG-TURN | src-tauri/src/ag/task.rs | 760 |

#### BR-AG-STATE-001 term_ref: TERM-AG-018

| codes                                                                                 | type   | chain   | 文件                       | 行号   |
| ------------------------------------------------------------------------------------- | ------ | ------- | ------------------------ | ---- |
| BR-SYS-STATE-008-EFFECT-AG-AG-TASKPLAN-001, BR-AG-STATE-001-EFFECT-AG-AG-TASKPLAN-001 | EFFECT | AG-TURN | src-tauri/src/ag/task.rs | 1127 |

#### BR-AG-STATE-002

| codes                                                                                                                                                                                                                                                | type   | chain                          | 文件                                 | 行号  |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------ | ---------------------------------- | --- |
| BR-AG-STATE-002-EFFECT-AG-AG-TOOL-017, BR-AG-DATA-003-EFFECT-AG-AG-TOOL-017                                                                                                                                                                          | EFFECT | AG-TOOL, DE-DIFF, ED-EDIT      | src-tauri/src/ag/edit_runtime.rs   | 19  |
| BR-AG-STATE-002-API-AG-WS-OPERATION-005, BR-AG-DATA-003-API-AG-WS-OPERATION-005, BR-SYS-STATE-008-API-AG-WS-OPERATION-005, BR-WS-STATE-003-API-AG-WS-OPERATION-005, BR-WS-PERSIST-003-API-AG-WS-OPERATION-005, BR-WS-OBS-002-API-AG-WS-OPERATION-005 | API    | AG-TURN, AG-TOOL, WS-OPERATION | src-tauri/src/ag/task_runtime.rs   | 173 |
| BR-AG-STATE-002-GUARD-AG-AG-TOOL-001                                                                                                                                                                                                                 | GUARD  | AG-TOOL                        | src-tauri/src/ag/tool.rs           | 91  |
| BR-AG-STATE-002-QUERY-AG-AG-TOOL-012, BR-AG-DATA-001-QUERY-AG-AG-TOOL-012, BR-WS-DATA-001-QUERY-AG-AG-TOOL-012                                                                                                                                       | QUERY  | AG-TOOL                        | src-tauri/src/ag/tool_exec.rs      | 38  |
| BR-AG-STATE-002-DATA-AG-AG-TOOL-034, BR-AG-DATA-004-DATA-AG-AG-TOOL-034, BR-SYS-STATE-007-DATA-AG-AG-TOOL-034                                                                                                                                        | DATA   | AG-TURN, AG-TOOL               | src-tauri/src/ag/tool_registry.rs  | 49  |
| BR-AG-STATE-002-GUARD-AG-AG-TOOL-035, BR-SYS-STATE-007-GUARD-AG-AG-TOOL-035                                                                                                                                                                          | GUARD  | AG-TOOL                        | src-tauri/src/ag/tool_registry.rs  | 176 |
| BR-AG-STATE-002-RB-AG-AG-TOOL-018, BR-SYS-STATE-008-RB-AG-AG-TOOL-018                                                                                                                                                                                | RB     | AG-TURN, AG-TOOL               | src-tauri/src/ag/turn_tool_loop.rs | 93  |

#### BR-AG-STATE-002 term_ref: TERM-AG-002

| codes                                 | type   | chain   | 文件                                 | 行号 |
| ------------------------------------- | ------ | ------- | ---------------------------------- | -- |
| BR-AG-STATE-002-RB-AG-AG-TOOL-001     | RB     | AG-TOOL | src-tauri/src/ag/tool.rs           | 52 |
| BR-AG-STATE-002-EFFECT-AG-AG-TOOL-038 | EFFECT | AG-TOOL | src-tauri/src/ag/turn_tool_loop.rs | 65 |

#### BR-AG-STATE-002 term_ref: TERM-AG-018

| codes                                                                                                                         | type  | chain   | 文件                       | 行号   |
| ----------------------------------------------------------------------------------------------------------------------------- | ----- | ------- | ------------------------ | ---- |
| BR-SYS-STATE-008-GUARD-AG-AG-TASKPLAN-001, BR-AG-STATE-001-GUARD-AG-AG-TASKPLAN-001, BR-AG-STATE-002-GUARD-AG-AG-TASKPLAN-001 | GUARD | AG-TURN | src-tauri/src/ag/task.rs | 1068 |

#### BR-AG-STATE-002 term_ref: TERM-AG-019

| codes                                                                     | type  | chain   | 文件                                | 行号  |
| ------------------------------------------------------------------------- | ----- | ------- | --------------------------------- | --- |
| BR-AG-DATA-003-GUARD-AG-AG-TOOL-033, BR-AG-STATE-002-GUARD-AG-AG-TOOL-033 | GUARD | AG-TOOL | src-tauri/src/ag/tool_registry.rs | 311 |

#### BR-AG-STATE-003

| codes                                                                             | type   | chain     | 文件                                 | 行号 |
| --------------------------------------------------------------------------------- | ------ | --------- | ---------------------------------- | -- |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-001                                               | RB     | AG-ASSIST | src-tauri/src/ag/assist.rs         | 40 |
| BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-010                                           | EFFECT | AG-ASSIST | src-tauri/src/ag/assist_runtime.rs | 50 |
| BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-001, BR-AG-VERIFY-001-EFFECT-AG-AG-ASSIST-001 | EFFECT | AG-ASSIST | src-tauri/src/ag/assist_runtime.rs | 92 |

#### BR-AG-STATE-003 term_ref: TERM-AG-003

| codes                                   | type   | chain     | 文件                                   | 行号   |
| --------------------------------------- | ------ | --------- | ------------------------------------ | ---- |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-007     | RB     | AG-ASSIST | src/components/ed/Editor.tsx         | 810  |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-002     | RB     | AG-ASSIST | src/components/ed/Editor.tsx         | 1890 |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-004     | RB     | AG-ASSIST | src/components/ed/assistPlacement.ts | 36   |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-005     | RB     | AG-ASSIST | src/components/ed/assistPlacement.ts | 60   |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-006     | RB     | AG-ASSIST | src/components/ed/assistPlacement.ts | 78   |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-003     | RB     | AG-ASSIST | src/components/ed/assistPlacement.ts | 96   |
| BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-002 | EFFECT | AG-ASSIST | src/services/agService.ts            | 1380 |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-008     | RB     | AG-ASSIST | src/store/assistStore.ts             | 94   |

#### BR-AG-STATE-003 term_ref: TERM-AG-004

| codes                                   | type   | chain     | 文件                                        | 行号   |
| --------------------------------------- | ------ | --------- | ----------------------------------------- | ---- |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-009     | RB     | AG-ASSIST | src/components/ag/AutocompleteOverlay.tsx | 16   |
| BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-003 | EFFECT | AG-ASSIST | src/components/ed/Editor.tsx              | 1008 |

#### BR-AG-STATE-004

| codes                                 | type   | chain   | 文件                              | 行号 |
| ------------------------------------- | ------ | ------- | ------------------------------- | -- |
| BR-AG-STATE-004-GUARD-AG-AG-TURN-019  | GUARD  | AG-TURN | src-tauri/src/ag/memory_link.rs | 22 |
| BR-AG-STATE-004-EFFECT-AG-AG-TURN-029 | EFFECT | AG-TURN | src-tauri/src/ag/memory_link.rs | 49 |

#### BR-AG-UI-001 term_ref: TERM-AG-011

| codes                          | type | chain   | 文件                                    | 行号 |
| ------------------------------ | ---- | ------- | ------------------------------------- | -- |
| BR-AG-UI-001-RB-AG-AG-TURN-013 | RB   | AG-TURN | src/components/ag/MessageFlow.tsx     | 8  |
| BR-AG-UI-001-RB-AG-AG-TURN-014 | RB   | AG-TURN | src/components/ag/MessageList.tsx     | 12 |
| BR-AG-UI-001-RB-AG-AG-TURN-012 | RB   | AG-TURN | src/components/ag/MessageMarkdown.tsx | 8  |

#### BR-AG-VERIFY-001

| codes                                                                             | type   | chain     | 文件                                 | 行号  |
| --------------------------------------------------------------------------------- | ------ | --------- | ---------------------------------- | --- |
| BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-001, BR-AG-VERIFY-001-EFFECT-AG-AG-ASSIST-001 | EFFECT | AG-ASSIST | src-tauri/src/ag/assist_runtime.rs | 92  |
| BR-AG-VERIFY-001-GUARD-AG-AG-TURN-020                                             | GUARD  | AG-TURN   | src-tauri/src/ag/turn.rs           | 170 |

#### BR-AST-DATA-001

| codes                                    | type  | chain      | 文件                         | 行号 |
| ---------------------------------------- | ----- | ---------- | -------------------------- | -- |
| BR-AST-DATA-001-QUERY-AST-AST-MEMORY-004 | QUERY | AST-MEMORY | src-tauri/src/ast/store.rs | 22 |
| BR-AST-DATA-001-QUERY-AST-AST-MEMORY-002 | QUERY | AST-MEMORY | src-tauri/src/ast/store.rs | 50 |

#### BR-AST-DATA-001 term_ref: TERM-AST-002

| codes                                                                              | type  | chain                            | 文件                          | 行号 |
| ---------------------------------------------------------------------------------- | ----- | -------------------------------- | --------------------------- | -- |
| BR-SYS-DATA-007-API-AST-AST-MEMORY-001, BR-AST-DATA-001-API-AST-AST-MEMORY-001     | API   | AST-MEMORY, REF-INJECT, IPC-CALL | src-tauri/src/ast/routes.rs | 6  |
| BR-SYS-DATA-007-QUERY-AST-AST-MEMORY-003, BR-AST-DATA-001-QUERY-AST-AST-MEMORY-003 | QUERY | AST-MEMORY, REF-INJECT           | src-tauri/src/ast/store.rs  | 72 |

#### BR-AST-DATA-002

| codes                                    | type  | chain      | 文件                          | 行号 |
| ---------------------------------------- | ----- | ---------- | --------------------------- | -- |
| BR-AST-DATA-002-QUERY-AST-AST-MEMORY-001 | QUERY | AST-MEMORY | src-tauri/src/ast/supply.rs | 23 |

#### BR-AST-STATE-001

| codes                                      | type   | chain      | 文件                           | 行号  |
| ------------------------------------------ | ------ | ---------- | ---------------------------- | --- |
| BR-AST-STATE-001-RB-AST-AST-MEMORY-001     | RB     | AST-MEMORY | src-tauri/src/ast/capture.rs | 56  |
| BR-AST-STATE-001-EFFECT-AST-AST-MEMORY-001 | EFFECT | AST-MEMORY | src-tauri/src/ast/capture.rs | 91  |
| BR-AST-STATE-001-EFFECT-AST-AST-MEMORY-002 | EFFECT | AST-MEMORY | src-tauri/src/ast/capture.rs | 135 |

#### BR-CBT-DATA-001

| codes                                  | type | chain     | 文件                         | 行号 |
| -------------------------------------- | ---- | --------- | -------------------------- | -- |
| BR-CBT-DATA-001-DATA-CBT-CBT-BUILD-001 | DATA | CBT-BUILD | src-tauri/src/cbt/apply.rs | 12 |
| BR-CBT-DATA-001-DATA-CBT-CBT-BUILD-002 | DATA | CBT-BUILD | src-tauri/src/cbt/apply.rs | 37 |

#### BR-CBT-PERSIST-001

| codes                                       | type   | chain     | 文件                         | 行号  |
| ------------------------------------------- | ------ | --------- | -------------------------- | --- |
| BR-CBT-PERSIST-001-EFFECT-CBT-CBT-BUILD-001 | EFFECT | CBT-BUILD | src-tauri/src/cbt/build.rs | 99  |
| BR-CBT-PERSIST-001-QUERY-CBT-CBT-BUILD-001  | QUERY  | CBT-BUILD | src-tauri/src/cbt/build.rs | 128 |

#### BR-CBT-STATE-001

| codes                                 | type | chain     | 文件                         | 行号 |
| ------------------------------------- | ---- | --------- | -------------------------- | -- |
| BR-CBT-STATE-001-RB-CBT-CBT-BUILD-002 | RB   | CBT-BUILD | src-tauri/src/cbt/build.rs | 59 |

#### BR-CBT-STATE-002

| codes                                    | type  | chain     | 文件                            | 行号 |
| ---------------------------------------- | ----- | --------- | ----------------------------- | -- |
| BR-CBT-STATE-002-GUARD-CBT-CBT-BUILD-001 | GUARD | CBT-BUILD | src-tauri/src/cbt/takeover.rs | 35 |
| BR-CBT-STATE-002-RB-CBT-CBT-BUILD-001    | RB    | CBT-BUILD | src-tauri/src/cbt/takeover.rs | 54 |

#### BR-CORE-STATE-001

| codes                                                                                                            | type   | chain                                    | 文件                           | 行号   |
| ---------------------------------------------------------------------------------------------------------------- | ------ | ---------------------------------------- | ---------------------------- | ---- |
| BR-CORE-STATE-001-EFFECT-ED-ED-EDIT-001, BR-CORE-STATE-003-EFFECT-ED-ED-EDIT-001                                 | EFFECT | DE-DIFF, ED-EDIT, ED-DECOR               | src/components/ed/Editor.tsx | 1723 |
| BR-CORE-STATE-001-EFFECT-DE-ED-EDIT-010                                                                          | EFFECT | DE-DIFF, ED-EDIT                         | src/services/deService.ts    | 193  |
| BR-CORE-STATE-001-API-DE-DE-DIFF-001, BR-CORE-STATE-002-API-DE-DE-DIFF-001, BR-CORE-STATE-003-API-DE-DE-DIFF-001 | API    | DE-DIFF, ED-EDIT, WS-SAVE, WS-DIFF-APPLY | src/services/deService.ts    | 309  |
| BR-CORE-STATE-001-EFFECT-ED-WS-SAVE-001, BR-CORE-STATE-003-EFFECT-ED-WS-SAVE-001                                 | EFFECT | DE-DIFF, ED-EDIT, WS-SAVE                | src/services/edService.ts    | 68   |
| BR-CORE-STATE-001-EFFECT-DE-DE-DIFF-005                                                                          | EFFECT | DE-DIFF, ED-EDIT, WS-SAVE                | src-tauri/src/de/routes.rs   | 123  |

#### BR-CORE-STATE-002

| codes                                                                                                            | type   | chain                                    | 文件                         | 行号  |
| ---------------------------------------------------------------------------------------------------------------- | ------ | ---------------------------------------- | -------------------------- | --- |
| BR-CORE-STATE-001-API-DE-DE-DIFF-001, BR-CORE-STATE-002-API-DE-DE-DIFF-001, BR-CORE-STATE-003-API-DE-DE-DIFF-001 | API    | DE-DIFF, ED-EDIT, WS-SAVE, WS-DIFF-APPLY | src/services/deService.ts  | 309 |
| BR-CORE-STATE-002-EFFECT-DE-DE-DIFF-007                                                                          | EFFECT | DE-DIFF, WS-DIFF-APPLY                   | src-tauri/src/de/routes.rs | 323 |

#### BR-CORE-STATE-003

| codes                                                                                                            | type   | chain                                    | 文件                           | 行号   |
| ---------------------------------------------------------------------------------------------------------------- | ------ | ---------------------------------------- | ---------------------------- | ---- |
| BR-CORE-STATE-001-EFFECT-ED-ED-EDIT-001, BR-CORE-STATE-003-EFFECT-ED-ED-EDIT-001                                 | EFFECT | DE-DIFF, ED-EDIT, ED-DECOR               | src/components/ed/Editor.tsx | 1723 |
| BR-CORE-STATE-003-EFFECT-DE-ED-EDIT-011                                                                          | EFFECT | DE-DIFF, ED-EDIT, ED-DECOR, WS-SAVE      | src/services/deService.ts    | 252  |
| BR-CORE-STATE-001-API-DE-DE-DIFF-001, BR-CORE-STATE-002-API-DE-DE-DIFF-001, BR-CORE-STATE-003-API-DE-DE-DIFF-001 | API    | DE-DIFF, ED-EDIT, WS-SAVE, WS-DIFF-APPLY | src/services/deService.ts    | 309  |
| BR-CORE-STATE-001-EFFECT-ED-WS-SAVE-001, BR-CORE-STATE-003-EFFECT-ED-WS-SAVE-001                                 | EFFECT | DE-DIFF, ED-EDIT, WS-SAVE                | src/services/edService.ts    | 68   |
| BR-CORE-STATE-003-EFFECT-DE-DE-DIFF-006                                                                          | EFFECT | DE-DIFF, ED-EDIT, ED-DECOR, WS-SAVE      | src-tauri/src/de/routes.rs   | 257  |

#### BR-DE-DATA-001

| codes                               | type  | chain   | 文件                           | 行号 |
| ----------------------------------- | ----- | ------- | ---------------------------- | -- |
| BR-DE-DATA-001-GUARD-DE-DE-DIFF-005 | GUARD | DE-DIFF | src-tauri/src/de/generate.rs | 87 |

#### BR-DE-DATA-001 term_ref: TERM-ED-001

| codes                                                                                                | type | chain   | 文件                            | 行号  |
| ---------------------------------------------------------------------------------------------------- | ---- | ------- | ----------------------------- | --- |
| BR-AG-DATA-003-RB-AG-AG-TOOL-016, BR-ED-DATA-001-RB-AG-AG-TOOL-016, BR-DE-DATA-001-RB-AG-AG-TOOL-016 | RB   | AG-TOOL | src-tauri/src/ag/block_ops.rs | 299 |

#### BR-DE-DATA-002

| codes                              | type | chain   | 文件                           | 行号 |
| ---------------------------------- | ---- | ------- | ---------------------------- | -- |
| BR-DE-DATA-002-DATA-DE-DE-DIFF-001 | DATA | DE-DIFF | src-tauri/src/de/generate.rs | 41 |

#### BR-DE-DATA-002 term_ref: TERM-DE-003

| codes                            | type | chain   | 文件                             | 行号  |
| -------------------------------- | ---- | ------- | ------------------------------ | --- |
| BR-DE-DATA-002-RB-DE-DE-DIFF-003 | RB   | DE-DIFF | src/components/de/DiffCard.tsx | 241 |

#### BR-DE-DATA-002 term_ref: TERM-DE-004

| codes                                | type   | chain   | 文件                                    | 行号  |
| ------------------------------------ | ------ | ------- | ------------------------------------- | --- |
| BR-DE-DATA-002-EFFECT-DE-DE-DIFF-009 | EFFECT | DE-DIFF | src/services/deService.ts             | 147 |
| BR-DE-DATA-002-RB-ED-DE-DIFF-028     | RB     | DE-DIFF | src/services/edImageOps.ts            | 493 |
| BR-DE-DATA-002-IO-DE-DE-DIFF-002     | IO     | DE-DIFF | src-tauri/src/de/store.rs             | 30  |
| BR-DE-DATA-002-QUERY-DE-DE-DIFF-002  | QUERY  | DE-DIFF | src-tauri/src/de/store.rs             | 65  |
| BR-DE-DATA-002-EFFECT-WS-DE-DIFF-004 | EFFECT | DE-DIFF | src-tauri/src/workspace/diff_apply.rs | 219 |

#### BR-DE-DATA-003

| codes                               | type  | chain   | 文件                            | 行号 |
| ----------------------------------- | ----- | ------- | ----------------------------- | -- |
| BR-DE-DATA-003-GUARD-DE-DE-DIFF-003 | GUARD | DE-DIFF | src-tauri/src/de/expiry.rs    | 62 |
| BR-DE-DATA-003-GUARD-DE-DE-DIFF-006 | GUARD | DE-DIFF | src-tauri/src/de/lifecycle.rs | 86 |

#### BR-DE-STATE-001

| codes                             | type | chain   | 文件                      | 行号 |
| --------------------------------- | ---- | ------- | ----------------------- | -- |
| BR-DE-STATE-001-RB-DE-DE-DIFF-001 | RB   | DE-DIFF | src-tauri/src/de/mod.rs | 71 |

#### BR-DE-STATE-001 term_ref: TERM-DE-001

| codes                             | type | chain   | 文件                   | 行号  |
| --------------------------------- | ---- | ------- | -------------------- | --- |
| BR-DE-STATE-001-RB-DE-DE-DIFF-002 | RB   | DE-DIFF | src/store/deStore.ts | 257 |

#### BR-DE-STATE-002

| codes                                 | type   | chain   | 文件                            | 行号  |
| ------------------------------------- | ------ | ------- | ----------------------------- | --- |
| BR-DE-STATE-002-GUARD-DE-DE-DIFF-001  | GUARD  | DE-DIFF | src-tauri/src/de/expiry.rs    | 34  |
| BR-DE-STATE-002-QUERY-DE-DE-DIFF-001  | QUERY  | DE-DIFF | src-tauri/src/de/lifecycle.rs | 31  |
| BR-DE-STATE-002-GUARD-DE-DE-DIFF-002  | GUARD  | DE-DIFF | src-tauri/src/de/lifecycle.rs | 58  |
| BR-DE-STATE-002-EFFECT-DE-DE-DIFF-001 | EFFECT | DE-DIFF | src-tauri/src/de/lifecycle.rs | 108 |

#### BR-DOC-DATA-001

| codes                                     | type  | chain       | 文件                                       | 行号  |
| ----------------------------------------- | ----- | ----------- | ---------------------------------------- | --- |
| BR-DOC-DATA-001-GUARD-DOC-DOC-CONVERT-001 | GUARD | DOC-CONVERT | src-tauri/src/doc/convert.rs             | 89  |
| BR-DOC-DATA-001-DATA-DOC-DOC-CONVERT-006  | DATA  | DOC-CONVERT | src-tauri/src/doc/font_trace.rs          | 243 |
| BR-DOC-DATA-001-DATA-DOC-DOC-CONVERT-007  | DATA  | DOC-CONVERT | src-tauri/src/doc/font_trace.rs          | 318 |
| BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-003    | IO    | DOC-CONVERT | src-tauri/src/doc/libreoffice_runtime.rs | 77  |
| BR-DOC-DATA-001-DATA-DOC-DOC-CONVERT-004  | DATA  | DOC-CONVERT | src-tauri/src/doc/libreoffice_runtime.rs | 106 |
| BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-005    | IO    | DOC-CONVERT | src-tauri/src/doc/libreoffice_runtime.rs | 244 |
| BR-DOC-DATA-001-DATA-DOC-DOC-CONVERT-012  | DATA  | DOC-CONVERT | src-tauri/src/doc/pandoc_runtime.rs      | 32  |
| BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-001    | IO    | DOC-CONVERT | src-tauri/src/doc/pandoc_runtime.rs      | 62  |
| BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-002    | IO    | DOC-CONVERT | src-tauri/src/doc/pandoc_runtime.rs      | 88  |
| BR-DOC-DATA-001-GUARD-DOC-DOC-CONVERT-002 | GUARD | DOC-CONVERT | src-tauri/src/doc/tool_runtime.rs        | 118 |

#### BR-DOC-DATA-002

| codes                                                                               | type  | chain       | 文件                                | 行号  |
| ----------------------------------------------------------------------------------- | ----- | ----------- | --------------------------------- | --- |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-016, BR-DOC-DATA-003-IO-DOC-DOC-CONVERT-016      | IO    | DOC-CONVERT | src-tauri/src/doc/edit_chain.rs   | 88  |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-004                                              | IO    | DOC-CONVERT | src-tauri/src/doc/edit_chain.rs   | 157 |
| BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-009, BR-DOC-DATA-003-DATA-DOC-DOC-CONVERT-009  | DATA  | DOC-CONVERT | src-tauri/src/doc/image_media.rs  | 37  |
| BR-DOC-DATA-002-GUARD-DOC-DOC-CONVERT-011, BR-WS-DATA-001-GUARD-DOC-DOC-CONVERT-011 | GUARD | DOC-CONVERT | src-tauri/src/doc/image_media.rs  | 334 |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-015, BR-WS-DATA-001-IO-DOC-DOC-CONVERT-015       | IO    | DOC-CONVERT | src-tauri/src/doc/image_media.rs  | 568 |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-013, BR-DOC-DATA-003-IO-DOC-DOC-CONVERT-013      | IO    | DOC-CONVERT | src-tauri/src/doc/image_media.rs  | 766 |
| BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-001                                            | DATA  | DOC-CONVERT | src-tauri/src/doc/style_subset.rs | 53  |
| BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-002                                            | DATA  | DOC-CONVERT | src-tauri/src/doc/style_subset.rs | 79  |

#### BR-DOC-DATA-002 term_ref: TERM-DOC-005

| codes                                    | type | chain       | 文件                                     | 行号  |
| ---------------------------------------- | ---- | ----------- | -------------------------------------- | --- |
| BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-008 | DATA | DOC-CONVERT | src-tauri/src/doc/docx_cell_shading.rs | 16  |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-006   | IO   | DOC-CONVERT | src-tauri/src/doc/docx_cell_shading.rs | 427 |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-007   | IO   | DOC-CONVERT | src-tauri/src/doc/docx_cell_shading.rs | 487 |

#### BR-DOC-DATA-002 term_ref: TERM-ED-008

| codes                                     | type  | chain       | 文件                               | 行号   |
| ----------------------------------------- | ----- | ----------- | -------------------------------- | ---- |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-012    | IO    | DOC-CONVERT | src-tauri/src/doc/image_media.rs | 670  |
| BR-DOC-DATA-002-GUARD-DOC-DOC-CONVERT-014 | GUARD | DOC-CONVERT | src-tauri/src/doc/image_media.rs | 1083 |

#### BR-DOC-DATA-003

| codes                                                                              | type  | chain       | 文件                                  | 行号  |
| ---------------------------------------------------------------------------------- | ----- | ----------- | ----------------------------------- | --- |
| BR-DOC-DATA-003-DATA-DOC-DOC-CONVERT-011                                           | DATA  | DOC-CONVERT | src-tauri/src/doc/block_backfill.rs | 26  |
| BR-DOC-DATA-003-DATA-DOC-DOC-CONVERT-003                                           | DATA  | DOC-CONVERT | src-tauri/src/doc/block_backfill.rs | 212 |
| BR-DOC-DATA-003-GUARD-DOC-DOC-CONVERT-015                                          | GUARD | DOC-CONVERT | src-tauri/src/doc/block_backfill.rs | 259 |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-016, BR-DOC-DATA-003-IO-DOC-DOC-CONVERT-016     | IO    | DOC-CONVERT | src-tauri/src/doc/edit_chain.rs     | 88  |
| BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-009, BR-DOC-DATA-003-DATA-DOC-DOC-CONVERT-009 | DATA  | DOC-CONVERT | src-tauri/src/doc/image_media.rs    | 37  |

#### BR-DOC-DATA-003 term_ref: TERM-DOC-003

| codes                                                                     | type  | chain   | 文件                         | 行号  |
| ------------------------------------------------------------------------- | ----- | ------- | -------------------------- | --- |
| BR-AG-DATA-001-QUERY-AG-AG-TURN-010, BR-DOC-DATA-003-QUERY-AG-AG-TURN-010 | QUERY | AG-TURN | src-tauri/src/ag/blocks.rs | 415 |

#### BR-DOC-DATA-003 term_ref: TERM-ED-008

| codes                                                                          | type | chain       | 文件                               | 行号  |
| ------------------------------------------------------------------------------ | ---- | ----------- | -------------------------------- | --- |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-013, BR-DOC-DATA-003-IO-DOC-DOC-CONVERT-013 | IO   | DOC-CONVERT | src-tauri/src/doc/image_media.rs | 766 |

#### BR-DOC-STATE-001

| codes                                   | type | chain       | 文件                           | 行号 |
| --------------------------------------- | ---- | ----------- | ---------------------------- | -- |
| BR-DOC-STATE-001-RB-DOC-DOC-CONVERT-001 | RB   | DOC-CONVERT | src-tauri/src/doc/convert.rs | 56 |

#### BR-ED-DATA-001

| codes                                                                                                             | type   | chain   | 文件                                      | 行号  |
| ----------------------------------------------------------------------------------------------------------------- | ------ | ------- | --------------------------------------- | --- |
| BR-ED-DATA-001-RB-ED-ED-EDIT-024, BR-ED-DATA-003-RB-ED-ED-EDIT-024                                                | RB     | ED-EDIT | src/components/ed/ImageMoveControls.tsx | 39  |
| BR-ED-DATA-001-QUERY-ED-ED-EDIT-024, BR-ED-DATA-003-QUERY-ED-ED-EDIT-024                                          | QUERY  | ED-EDIT | src/services/edImageOps.ts              | 228 |
| BR-ED-DATA-001-QUERY-ED-ED-EDIT-021, BR-ED-DATA-003-QUERY-ED-ED-EDIT-021                                          | QUERY  | ED-EDIT | src/services/edImageOps.ts              | 570 |
| BR-ED-DATA-001-EFFECT-ED-ED-EDIT-022, BR-ED-DATA-003-EFFECT-ED-ED-EDIT-022, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-022 | EFFECT | ED-EDIT | src/services/edImageOps.ts              | 599 |
| BR-ED-DATA-001-EFFECT-ED-ED-EDIT-023, BR-ED-DATA-003-EFFECT-ED-ED-EDIT-023, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-023 | EFFECT | ED-EDIT | src/services/edImageOps.ts              | 640 |
| BR-ED-DATA-001-EFFECT-ED-ED-EDIT-025, BR-ED-DATA-003-EFFECT-ED-ED-EDIT-025, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-025 | EFFECT | ED-EDIT | src/services/edImageOps.ts              | 731 |
| BR-AG-DATA-003-RB-AG-AG-TOOL-016, BR-ED-DATA-001-RB-AG-AG-TOOL-016, BR-DE-DATA-001-RB-AG-AG-TOOL-016              | RB     | AG-TOOL | src-tauri/src/ag/block_ops.rs           | 299 |
| BR-SYS-DATA-006-GUARD-AG-AG-TOOL-004, BR-ED-DATA-001-GUARD-AG-AG-TOOL-004, BR-ED-DATA-003-GUARD-AG-AG-TOOL-004    | GUARD  | AG-TOOL | src-tauri/src/ag/edit_image_runtime.rs  | 8   |

#### BR-ED-DATA-001 term_ref: TERM-ED-001

| codes                              | type | chain   | 文件                                               | 行号 |
| ---------------------------------- | ---- | ------- | ------------------------------------------------ | -- |
| BR-ED-DATA-001-DATA-ED-ED-EDIT-001 | DATA | ED-EDIT | src/components/ed/extensions/BlockIdExtension.ts | 96 |

#### BR-ED-DATA-002

| codes                                                              | type | chain   | 文件                        | 行号  |
| ------------------------------------------------------------------ | ---- | ------- | ------------------------- | --- |
| BR-ED-DATA-002-RB-ED-ED-EDIT-012, BR-ED-DATA-006-RB-ED-ED-EDIT-012 | RB   | ED-EDIT | src/services/edService.ts | 247 |

#### BR-ED-DATA-003

| codes                                                                                                             | type   | chain   | 文件                                             | 行号  |
| ----------------------------------------------------------------------------------------------------------------- | ------ | ------- | ---------------------------------------------- | --- |
| BR-ED-DATA-001-RB-ED-ED-EDIT-024, BR-ED-DATA-003-RB-ED-ED-EDIT-024                                                | RB     | ED-EDIT | src/components/ed/ImageMoveControls.tsx        | 39  |
| BR-ED-DATA-003-GUARD-ED-ED-EDIT-019                                                                               | GUARD  | ED-EDIT | src/components/ed/extensions/Pagination.ts     | 263 |
| BR-ED-DATA-003-GUARD-ED-ED-EDIT-020                                                                               | GUARD  | ED-EDIT | src/components/ed/extensions/Pagination.ts     | 585 |
| BR-ED-DATA-003-GUARD-ED-ED-EDIT-018                                                                               | GUARD  | ED-EDIT | src/components/ed/extensions/Pagination.ts     | 636 |
| BR-ED-DATA-003-RB-ED-ED-EDIT-005                                                                                  | RB     | ED-EDIT | src/components/ed/extensions/Pagination.ts     | 893 |
| BR-ED-DATA-003-EFFECT-ED-ED-EDIT-020, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-020                                       | EFFECT | ED-EDIT | src/components/ed/extensions/ResizableImage.ts | 50  |
| BR-ED-DATA-003-RB-ED-ED-EDIT-017                                                                                  | RB     | ED-EDIT | src/components/ed/extensions/paginationFlow.ts | 57  |
| BR-ED-DATA-003-RB-ED-ED-EDIT-006                                                                                  | RB     | ED-EDIT | src/services/docService.ts                     | 107 |
| BR-ED-DATA-001-QUERY-ED-ED-EDIT-021, BR-ED-DATA-003-QUERY-ED-ED-EDIT-021                                          | QUERY  | ED-EDIT | src/services/edImageOps.ts                     | 570 |
| BR-ED-DATA-001-EFFECT-ED-ED-EDIT-022, BR-ED-DATA-003-EFFECT-ED-ED-EDIT-022, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-022 | EFFECT | ED-EDIT | src/services/edImageOps.ts                     | 599 |
| BR-ED-DATA-001-EFFECT-ED-ED-EDIT-023, BR-ED-DATA-003-EFFECT-ED-ED-EDIT-023, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-023 | EFFECT | ED-EDIT | src/services/edImageOps.ts                     | 640 |
| BR-ED-DATA-001-EFFECT-ED-ED-EDIT-025, BR-ED-DATA-003-EFFECT-ED-ED-EDIT-025, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-025 | EFFECT | ED-EDIT | src/services/edImageOps.ts                     | 731 |

#### BR-ED-DATA-003 term_ref: TERM-ED-008

| codes                                                                    | type   | chain   | 文件                         | 行号  |
| ------------------------------------------------------------------------ | ------ | ------- | -------------------------- | --- |
| BR-ED-DATA-003-RB-ED-ED-EDIT-026                                         | RB     | ED-EDIT | src/services/docService.ts | 184 |
| BR-ED-DATA-003-EFFECT-ED-ED-EDIT-026                                     | EFFECT | ED-EDIT | src/services/docService.ts | 203 |
| BR-ED-DATA-001-QUERY-ED-ED-EDIT-024, BR-ED-DATA-003-QUERY-ED-ED-EDIT-024 | QUERY  | ED-EDIT | src/services/edImageOps.ts | 228 |
| BR-AG-DATA-001-QUERY-AG-AG-TOOL-011, BR-ED-DATA-003-QUERY-AG-AG-TOOL-011 | QUERY  | AG-TOOL | src-tauri/src/ag/blocks.rs | 190 |

#### BR-ED-DATA-003 term_ref: TERM-SYS-002

| codes                                                                                                          | type  | chain   | 文件                                     | 行号   |
| -------------------------------------------------------------------------------------------------------------- | ----- | ------- | -------------------------------------- | ---- |
| BR-SYS-DATA-006-RB-ED-ED-EDIT-027, BR-ED-DATA-003-RB-ED-ED-EDIT-027                                            | RB    | ED-EDIT | src/services/edImageOps.ts             | 256  |
| BR-SYS-DATA-006-GUARD-AG-AG-TOOL-022, BR-ED-DATA-003-GUARD-AG-AG-TOOL-022                                      | GUARD | AG-TOOL | src-tauri/src/ag/bdocx_capabilities.rs | 1918 |
| BR-SYS-DATA-006-GUARD-AG-AG-TOOL-004, BR-ED-DATA-001-GUARD-AG-AG-TOOL-004, BR-ED-DATA-003-GUARD-AG-AG-TOOL-004 | GUARD | AG-TOOL | src-tauri/src/ag/edit_image_runtime.rs | 8    |

#### BR-ED-DATA-004 term_ref: TERM-ED-004

| codes                               | type  | chain   | 文件                       | 行号  |
| ----------------------------------- | ----- | ------- | ------------------------ | --- |
| BR-ED-DATA-004-QUERY-ED-ED-EDIT-010 | QUERY | ED-EDIT | src/services/edLocate.ts | 231 |
| BR-ED-DATA-004-QUERY-ED-ED-EDIT-007 | QUERY | ED-EDIT | src/services/edLocate.ts | 414 |

#### BR-ED-DATA-005

| codes                                                                      | type  | chain   | 文件                                          | 行号  |
| -------------------------------------------------------------------------- | ----- | ------- | ------------------------------------------- | --- |
| BR-ED-VERIFY-001-GUARD-ED-ED-EDIT-015, BR-ED-DATA-005-GUARD-ED-ED-EDIT-015 | GUARD | ED-EDIT | src/components/ed/markdown/markdownCodec.ts | 170 |

#### BR-ED-DATA-005 term_ref: TERM-ED-006

| codes                               | type  | chain   | 文件                         | 行号  |
| ----------------------------------- | ----- | ------- | -------------------------- | --- |
| BR-ED-DATA-005-RB-ED-ED-EDIT-008    | RB    | ED-EDIT | src/services/edStyleOps.ts | 243 |
| BR-ED-DATA-005-QUERY-ED-ED-EDIT-009 | QUERY | ED-EDIT | src-tauri/src/ag/blocks.rs | 751 |

#### BR-ED-DATA-005 term_ref: TERM-SYS-002

| codes                                                                     | type  | chain   | 文件                                     | 行号   |
| ------------------------------------------------------------------------- | ----- | ------- | -------------------------------------- | ---- |
| BR-SYS-DATA-006-GUARD-AG-AG-TOOL-021, BR-ED-DATA-005-GUARD-AG-AG-TOOL-021 | GUARD | AG-TOOL | src-tauri/src/ag/bdocx_capabilities.rs | 1566 |

#### BR-ED-DATA-006

| codes                                                                      | type  | chain   | 文件                                          | 行号  |
| -------------------------------------------------------------------------- | ----- | ------- | ------------------------------------------- | --- |
| BR-ED-DATA-006-DATA-ED-ED-EDIT-014                                         | DATA  | ED-EDIT | src/components/ed/markdown/markdownCodec.ts | 82  |
| BR-ED-DATA-002-RB-ED-ED-EDIT-012, BR-ED-DATA-006-RB-ED-ED-EDIT-012         | RB    | ED-EDIT | src/services/edService.ts                   | 247 |
| BR-ED-VERIFY-001-GUARD-ED-ED-EDIT-016, BR-ED-DATA-006-GUARD-ED-ED-EDIT-016 | GUARD | ED-EDIT | src/services/edService.ts                   | 338 |

#### BR-ED-DATA-007 term_ref: TERM-ED-007

| codes                                | type   | chain                     | 文件                                               | 行号  |
| ------------------------------------ | ------ | ------------------------- | ------------------------------------------------ | --- |
| BR-ED-DATA-007-RB-ED-ED-EDIT-019     | RB     | ED-EDIT                   | src/components/ed/TableMoveControls.tsx          | 47  |
| BR-ED-DATA-007-DATA-ED-ED-EDIT-018   | DATA   | ED-EDIT                   | src/components/ed/extensions/TableCellShading.ts | 37  |
| BR-ED-DATA-007-RB-ED-ED-EDIT-018     | RB     | ED-EDIT, DE-DIFF          | src/services/edTableOps.ts                       | 407 |
| BR-ED-DATA-007-EFFECT-ED-ED-EDIT-014 | EFFECT | ED-EDIT, DE-DIFF          | src/services/edTableOps.ts                       | 438 |
| BR-ED-DATA-007-GUARD-AG-AG-TOOL-018  | GUARD  | AG-TOOL, DE-DIFF, ED-EDIT | src-tauri/src/ag/edit_table_runtime.rs           | 8   |

#### BR-ED-STATE-001

| codes                                                                                                             | type   | chain   | 文件                         | 行号  |
| ----------------------------------------------------------------------------------------------------------------- | ------ | ------- | -------------------------- | --- |
| BR-ED-DATA-001-EFFECT-ED-ED-EDIT-022, BR-ED-DATA-003-EFFECT-ED-ED-EDIT-022, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-022 | EFFECT | ED-EDIT | src/services/edImageOps.ts | 599 |
| BR-ED-DATA-001-EFFECT-ED-ED-EDIT-023, BR-ED-DATA-003-EFFECT-ED-ED-EDIT-023, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-023 | EFFECT | ED-EDIT | src/services/edImageOps.ts | 640 |
| BR-ED-STATE-001-RB-ED-ED-EDIT-002                                                                                 | RB     | ED-EDIT | src/store/edStore.ts       | 260 |

#### BR-ED-STATE-001 term_ref: TERM-ED-001

| codes                                 | type   | chain   | 文件                       | 行号  |
| ------------------------------------- | ------ | ------- | ------------------------ | --- |
| BR-ED-STATE-001-EFFECT-ED-ED-EDIT-012 | EFFECT | ED-EDIT | src/services/edLocate.ts | 345 |
| BR-ED-STATE-001-EFFECT-ED-ED-EDIT-011 | EFFECT | ED-EDIT | src/services/edLocate.ts | 384 |

#### BR-ED-STATE-001 term_ref: TERM-ED-008

| codes                                                                                                             | type   | chain   | 文件                                             | 行号  |
| ----------------------------------------------------------------------------------------------------------------- | ------ | ------- | ---------------------------------------------- | --- |
| BR-ED-DATA-003-EFFECT-ED-ED-EDIT-020, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-020                                       | EFFECT | ED-EDIT | src/components/ed/extensions/ResizableImage.ts | 50  |
| BR-ED-DATA-001-EFFECT-ED-ED-EDIT-025, BR-ED-DATA-003-EFFECT-ED-ED-EDIT-025, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-025 | EFFECT | ED-EDIT | src/services/edImageOps.ts                     | 731 |

#### BR-ED-STATE-002 term_ref: TERM-ED-002

| codes                              | type | chain    | 文件                                                      | 行号  |
| ---------------------------------- | ---- | -------- | ------------------------------------------------------- | --- |
| BR-ED-STATE-002-RB-ED-ED-DECOR-001 | RB   | ED-DECOR | src/components/ed/extensions/DiffDecorationExtension.ts | 155 |

#### BR-ED-STATE-003

| codes                                 | type   | chain   | 文件                        | 行号  |
| ------------------------------------- | ------ | ------- | ------------------------- | --- |
| BR-ED-STATE-003-EFFECT-ED-ED-EDIT-013 | EFFECT | ED-EDIT | src/services/edService.ts | 304 |

#### BR-ED-VERIFY-001

| codes                                                                      | type  | chain   | 文件                                          | 行号  |
| -------------------------------------------------------------------------- | ----- | ------- | ------------------------------------------- | --- |
| BR-ED-VERIFY-001-GUARD-ED-ED-EDIT-015, BR-ED-DATA-005-GUARD-ED-ED-EDIT-015 | GUARD | ED-EDIT | src/components/ed/markdown/markdownCodec.ts | 170 |
| BR-ED-VERIFY-001-GUARD-ED-ED-EDIT-016, BR-ED-DATA-006-GUARD-ED-ED-EDIT-016 | GUARD | ED-EDIT | src/services/edService.ts                   | 338 |

#### BR-ENT-DATA-001

| codes                                       | type   | chain        | 文件                        | 行号 |
| ------------------------------------------- | ------ | ------------ | ------------------------- | -- |
| BR-ENT-DATA-001-EFFECT-ENT-CLOUD-INVOKE-001 | EFFECT | CLOUD-INVOKE | src-tauri/src/ent/byok.rs | 17 |

#### BR-ENT-DATA-002

| codes                                    | type  | chain      | 文件                         | 行号 |
| ---------------------------------------- | ----- | ---------- | -------------------------- | -- |
| BR-ENT-DATA-002-GUARD-ENT-CLOUD-GATE-001 | GUARD | CLOUD-GATE | src-tauri/src/ent/usage.rs | 34 |

#### BR-ENT-STATE-001

| codes                                    | type | chain        | 文件                           | 行号  |
| ---------------------------------------- | ---- | ------------ | ---------------------------- | --- |
| BR-ENT-STATE-001-RB-ENT-CLOUD-INVOKE-001 | RB   | CLOUD-INVOKE | src-tauri/src/ent/gateway.rs | 102 |

#### BR-ENT-VERIFY-001

| codes                                      | type  | chain      | 文件                           | 行号  |
| ------------------------------------------ | ----- | ---------- | ---------------------------- | --- |
| BR-ENT-VERIFY-001-GUARD-ENT-CLOUD-GATE-002 | GUARD | CLOUD-GATE | src-tauri/src/ent/gateway.rs | 132 |

#### BR-KB-DATA-001

| codes                                    | type   | chain       | 文件                        | 行号  |
| ---------------------------------------- | ------ | ----------- | ------------------------- | --- |
| BR-KB-DATA-001-EFFECT-KB-KB-RETRIEVE-001 | EFFECT | KB-RETRIEVE | src-tauri/src/kb/store.rs | 31  |
| BR-KB-DATA-001-QUERY-KB-KB-RETRIEVE-002  | QUERY  | KB-RETRIEVE | src-tauri/src/kb/store.rs | 84  |
| BR-KB-DATA-001-EFFECT-KB-KB-RETRIEVE-002 | EFFECT | KB-RETRIEVE | src-tauri/src/kb/store.rs | 106 |

#### BR-KB-DATA-002

| codes                                   | type  | chain       | 文件                           | 行号 |
| --------------------------------------- | ----- | ----------- | ---------------------------- | -- |
| BR-KB-DATA-002-QUERY-KB-KB-RETRIEVE-001 | QUERY | KB-RETRIEVE | src-tauri/src/kb/retrieve.rs | 12 |

#### BR-PCG-DATA-001

| codes                                                                             | type | chain        | 文件                                            | 行号 |
| --------------------------------------------------------------------------------- | ---- | ------------ | --------------------------------------------- | -- |
| BR-PCG-DATA-001-RB-PCG-PCG-GENERATE-001, BR-PCG-STATE-001-RB-PCG-PCG-GENERATE-001 | RB   | PCG-GENERATE | scripts/generate_function_ledger.mjs          | 14 |
| BR-PCG-DATA-001-RB-PCG-PCG-GENERATE-004                                           | RB   | PCG-GENERATE | scripts/generate_function_tree.mjs            | 22 |
| BR-PCG-DATA-001-RB-PCG-PCG-GENERATE-001, BR-PCG-STATE-001-RB-PCG-PCG-GENERATE-001 | RB   | PCG-GENERATE | scripts/generate_product_capability_atlas.mjs | 11 |
| BR-PCG-DATA-001-RB-PCG-PCG-GENERATE-003                                           | RB   | PCG-GENERATE | scripts/seed_function_audit.mjs               | 28 |

#### BR-PCG-STATE-001

| codes                                                                             | type | chain        | 文件                                            | 行号 |
| --------------------------------------------------------------------------------- | ---- | ------------ | --------------------------------------------- | -- |
| BR-PCG-DATA-001-RB-PCG-PCG-GENERATE-001, BR-PCG-STATE-001-RB-PCG-PCG-GENERATE-001 | RB   | PCG-GENERATE | scripts/generate_function_ledger.mjs          | 14 |
| BR-PCG-DATA-001-RB-PCG-PCG-GENERATE-004                                           | RB   | PCG-GENERATE | scripts/generate_function_tree.mjs            | 22 |
| BR-PCG-DATA-001-RB-PCG-PCG-GENERATE-001, BR-PCG-STATE-001-RB-PCG-PCG-GENERATE-001 | RB   | PCG-GENERATE | scripts/generate_product_capability_atlas.mjs | 11 |
| BR-PCG-DATA-001-RB-PCG-PCG-GENERATE-003                                           | RB   | PCG-GENERATE | scripts/seed_function_audit.mjs               | 28 |

#### BR-PCG-VERIFY-001

| codes                                                                          | type | chain     | 文件                                   | 行号 |
| ------------------------------------------------------------------------------ | ---- | --------- | ------------------------------------ | -- |
| BR-PCG-VERIFY-001-RB-PCG-PCG-AUDIT-001, BR-PCG-VERIFY-002-RB-PCG-PCG-AUDIT-001 | RB   | PCG-AUDIT | scripts/audit_product_capability.mjs | 5  |

#### BR-PCG-VERIFY-002

| codes                                                                          | type | chain     | 文件                                   | 行号 |
| ------------------------------------------------------------------------------ | ---- | --------- | ------------------------------------ | -- |
| BR-PCG-VERIFY-001-RB-PCG-PCG-AUDIT-001, BR-PCG-VERIFY-002-RB-PCG-PCG-AUDIT-001 | RB   | PCG-AUDIT | scripts/audit_product_capability.mjs | 5  |

#### BR-PRV-DATA-001

| codes                                                                    | type  | chain    | 文件                                | 行号 |
| ------------------------------------------------------------------------ | ----- | -------- | --------------------------------- | -- |
| BR-PRV-DATA-001-GUARD-PRV-PRV-VIEW-003                                   | GUARD | PRV-VIEW | src/components/prv/PrvPanel.tsx   | 41 |
| BR-PRV-DATA-001-RB-PRV-PRV-VIEW-004, BR-PRV-DATA-002-RB-PRV-PRV-VIEW-004 | RB    | PRV-VIEW | src/components/prv/PrvToolbar.tsx | 23 |

#### BR-PRV-DATA-002

| codes                                                                    | type | chain    | 文件                                 | 行号  |
| ------------------------------------------------------------------------ | ---- | -------- | ---------------------------------- | --- |
| BR-PRV-DATA-001-RB-PRV-PRV-VIEW-004, BR-PRV-DATA-002-RB-PRV-PRV-VIEW-004 | RB   | PRV-VIEW | src/components/prv/PrvToolbar.tsx  | 23  |
| BR-PRV-DATA-002-IO-PRV-PRV-VIEW-004                                      | IO   | PRV-VIEW | src-tauri/src/doc/table_preview.rs | 48  |
| BR-PRV-DATA-002-IO-PRV-PRV-VIEW-001                                      | IO   | PRV-VIEW | src-tauri/src/prv/mod.rs           | 84  |
| BR-PRV-DATA-002-IO-PRV-PRV-VIEW-002                                      | IO   | PRV-VIEW | src-tauri/src/prv/mod.rs           | 124 |
| BR-PRV-DATA-002-IO-PRV-PRV-VIEW-003                                      | IO   | PRV-VIEW | src-tauri/src/prv/mod.rs           | 182 |

#### BR-REF-DATA-001

| codes                                    | type  | chain      | 文件                                 | 行号 |
| ---------------------------------------- | ----- | ---------- | ---------------------------------- | -- |
| BR-REF-DATA-001-GUARD-REF-REF-INJECT-003 | GUARD | REF-INJECT | src-tauri/src/reference/resolve.rs | 34 |

#### BR-REF-DATA-001 term_ref: TERM-REF-001

| codes                                  | type | chain      | 文件                                | 行号  |
| -------------------------------------- | ---- | ---------- | --------------------------------- | --- |
| BR-REF-DATA-001-API-REF-REF-INJECT-001 | API  | REF-INJECT | src-tauri/src/reference/routes.rs | 75  |
| BR-REF-DATA-001-API-REF-REF-INJECT-002 | API  | REF-INJECT | src-tauri/src/reference/routes.rs | 112 |

#### BR-REF-DATA-002

| codes                                                                            | type | chain      | 文件                                | 行号  |
| -------------------------------------------------------------------------------- | ---- | ---------- | --------------------------------- | --- |
| BR-REF-DATA-002-DATA-REF-REF-INJECT-001                                          | DATA | REF-INJECT | src-tauri/src/reference/inject.rs | 54  |
| BR-REF-DATA-002-API-REF-REF-INJECT-004, BR-REF-VERIFY-001-API-REF-REF-INJECT-004 | API  | REF-INJECT | src-tauri/src/reference/routes.rs | 187 |

#### BR-REF-DATA-003

| codes                                                                                                                       | type | chain      | 文件                                 | 行号  |
| --------------------------------------------------------------------------------------------------------------------------- | ---- | ---------- | ---------------------------------- | --- |
| BR-REF-DATA-003-DATA-REF-REF-INJECT-006, BR-SYS-DATA-002-DATA-REF-REF-INJECT-006                                            | DATA | REF-INJECT | src/services/referencePayload.ts   | 2   |
| BR-REF-DATA-003-DATA-REF-REF-INJECT-002, BR-REF-VERIFY-001-DATA-REF-REF-INJECT-002, BR-SYS-DATA-007-DATA-REF-REF-INJECT-002 | DATA | REF-INJECT | src-tauri/src/reference/context.rs | 102 |

#### BR-REF-DATA-003 term_ref: TERM-REF-003

| codes                                    | type  | chain      | 文件                                 | 行号  |
| ---------------------------------------- | ----- | ---------- | ---------------------------------- | --- |
| BR-REF-DATA-003-DATA-REF-REF-INJECT-005  | DATA  | REF-INJECT | src/services/referenceDrag.ts      | 2   |
| BR-REF-DATA-003-GUARD-REF-REF-INJECT-004 | GUARD | REF-INJECT | src/services/referenceDrag.ts      | 49  |
| BR-REF-DATA-003-GUARD-REF-REF-INJECT-005 | GUARD | REF-INJECT | src/services/referenceDrag.ts      | 71  |
| BR-REF-DATA-003-GUARD-REF-REF-INJECT-006 | GUARD | REF-INJECT | src/services/referencePayload.ts   | 122 |
| BR-REF-DATA-003-GUARD-REF-REF-INJECT-007 | GUARD | REF-INJECT | src/services/referencePayload.ts   | 183 |
| BR-REF-DATA-003-DATA-REF-REF-INJECT-003  | DATA  | REF-INJECT | src-tauri/src/reference/context.rs | 36  |
| BR-REF-DATA-003-DATA-REF-REF-INJECT-004  | DATA  | REF-INJECT | src-tauri/src/reference/context.rs | 242 |

#### BR-REF-DATA-004 term_ref: TERM-REF-004

| codes                                  | type | chain      | 文件                               | 行号  |
| -------------------------------------- | ---- | ---------- | -------------------------------- | --- |
| BR-REF-DATA-004-DATA-AG-REF-INJECT-033 | DATA | REF-INJECT | src/components/ag/Composer.tsx   | 40  |
| BR-REF-DATA-004-RB-REF-REF-INJECT-001  | RB   | REF-INJECT | src/services/referencePayload.ts | 439 |

#### BR-REF-VERIFY-001

| codes                                                                                                                       | type  | chain      | 文件                                 | 行号  |
| --------------------------------------------------------------------------------------------------------------------------- | ----- | ---------- | ---------------------------------- | --- |
| BR-REF-DATA-003-DATA-REF-REF-INJECT-002, BR-REF-VERIFY-001-DATA-REF-REF-INJECT-002, BR-SYS-DATA-007-DATA-REF-REF-INJECT-002 | DATA  | REF-INJECT | src-tauri/src/reference/context.rs | 102 |
| BR-REF-VERIFY-001-GUARD-REF-REF-INJECT-001                                                                                  | GUARD | REF-INJECT | src-tauri/src/reference/inject.rs  | 14  |

#### BR-REF-VERIFY-001 term_ref: TERM-REF-001

| codes                                    | type | chain      | 文件                                | 行号  |
| ---------------------------------------- | ---- | ---------- | --------------------------------- | --- |
| BR-REF-VERIFY-001-API-REF-REF-INJECT-003 | API  | REF-INJECT | src-tauri/src/reference/routes.rs | 159 |

#### BR-REF-VERIFY-001 term_ref: TERM-REF-002

| codes                                                                            | type | chain      | 文件                                | 行号  |
| -------------------------------------------------------------------------------- | ---- | ---------- | --------------------------------- | --- |
| BR-REF-DATA-002-API-REF-REF-INJECT-004, BR-REF-VERIFY-001-API-REF-REF-INJECT-004 | API  | REF-INJECT | src-tauri/src/reference/routes.rs | 187 |

#### BR-REF-VERIFY-001 term_ref: TERM-REF-003

| codes                                      | type  | chain      | 文件                                 | 行号 |
| ------------------------------------------ | ----- | ---------- | ---------------------------------- | -- |
| BR-REF-VERIFY-001-GUARD-REF-REF-INJECT-002 | GUARD | REF-INJECT | src-tauri/src/reference/context.rs | 54 |

#### BR-SYS-DATA-001

| codes                                                                                | type  | chain                    | 文件                           | 行号 |
| ------------------------------------------------------------------------------------ | ----- | ------------------------ | ---------------------------- | -- |
| BR-SYS-VERIFY-001-GUARD-SYS-CLOUD-GATE-001, BR-SYS-DATA-001-GUARD-SYS-CLOUD-GATE-001 | GUARD | CLOUD-GATE, CLOUD-INVOKE | src-tauri/src/session/mod.rs | 76 |

#### BR-SYS-DATA-002

| codes                                     | type | chain        | 文件                         | 行号 |
| ----------------------------------------- | ---- | ------------ | -------------------------- | -- |
| BR-SYS-DATA-002-DATA-SYS-DATA-MIGRATE-001 | DATA | DATA-MIGRATE | src-tauri/src/types/mod.rs | 9  |

#### BR-SYS-DATA-002 term_ref: TERM-REF-003

| codes                                                                            | type | chain      | 文件                               | 行号 |
| -------------------------------------------------------------------------------- | ---- | ---------- | -------------------------------- | -- |
| BR-REF-DATA-003-DATA-REF-REF-INJECT-006, BR-SYS-DATA-002-DATA-REF-REF-INJECT-006 | DATA | REF-INJECT | src/services/referencePayload.ts | 2  |

#### BR-SYS-DATA-003

| codes                                | type | chain    | 文件                       | 行号  |
| ------------------------------------ | ---- | -------- | ------------------------ | --- |
| BR-SYS-DATA-003-API-SYS-IPC-CALL-001 | API  | IPC-CALL | src-tauri/src/ipc/mod.rs | 186 |
| BR-SYS-DATA-003-API-SYS-IPC-CALL-005 | API  | IPC-CALL | src-tauri/src/lib.rs     | 3   |

#### BR-SYS-DATA-003 term_ref: TERM-WS-001

| codes                                | type | chain    | 文件                   | 行号  |
| ------------------------------------ | ---- | -------- | -------------------- | --- |
| BR-SYS-DATA-003-API-SYS-IPC-CALL-002 | API  | IPC-CALL | src-tauri/src/lib.rs | 293 |

#### BR-SYS-DATA-005

| codes                                 | type   | chain   | 文件                            | 行号  |
| ------------------------------------- | ------ | ------- | ----------------------------- | --- |
| BR-SYS-DATA-005-EFFECT-DE-DE-DIFF-003 | EFFECT | DE-DIFF | src-tauri/src/de/lifecycle.rs | 132 |
| BR-SYS-DATA-005-EFFECT-DE-DE-DIFF-004 | EFFECT | DE-DIFF | src-tauri/src/de/lifecycle.rs | 155 |

#### BR-SYS-DATA-006

| codes                                                                                                          | type  | chain   | 文件                                     | 行号   |
| -------------------------------------------------------------------------------------------------------------- | ----- | ------- | -------------------------------------- | ---- |
| BR-SYS-DATA-006-RB-ED-ED-EDIT-027, BR-ED-DATA-003-RB-ED-ED-EDIT-027                                            | RB    | ED-EDIT | src/services/edImageOps.ts             | 256  |
| BR-SYS-DATA-006-GUARD-AG-AG-TOOL-021, BR-ED-DATA-005-GUARD-AG-AG-TOOL-021                                      | GUARD | AG-TOOL | src-tauri/src/ag/bdocx_capabilities.rs | 1566 |
| BR-SYS-DATA-006-GUARD-AG-AG-TOOL-022, BR-ED-DATA-003-GUARD-AG-AG-TOOL-022                                      | GUARD | AG-TOOL | src-tauri/src/ag/bdocx_capabilities.rs | 1918 |
| BR-SYS-DATA-006-GUARD-AG-AG-TOOL-004, BR-ED-DATA-001-GUARD-AG-AG-TOOL-004, BR-ED-DATA-003-GUARD-AG-AG-TOOL-004 | GUARD | AG-TOOL | src-tauri/src/ag/edit_image_runtime.rs | 8    |

#### BR-SYS-DATA-006 term_ref: TERM-SYS-002

| codes                               | type | chain   | 文件                                     | 行号   |
| ----------------------------------- | ---- | ------- | -------------------------------------- | ---- |
| BR-SYS-DATA-006-DATA-AG-AG-TOOL-001 | DATA | AG-TOOL | src-tauri/src/ag/bdocx_capabilities.rs | 611  |
| BR-SYS-DATA-006-DATA-AG-AG-TOOL-002 | DATA | AG-TOOL | src-tauri/src/ag/bdocx_capabilities.rs | 897  |
| BR-SYS-DATA-006-DATA-AG-AG-TOOL-032 | DATA | AG-TURN | src-tauri/src/ag/bdocx_capabilities.rs | 1298 |

#### BR-SYS-DATA-007

| codes                                                                              | type  | chain                            | 文件                          | 行号 |
| ---------------------------------------------------------------------------------- | ----- | -------------------------------- | --------------------------- | -- |
| BR-SYS-DATA-007-API-AST-AST-MEMORY-001, BR-AST-DATA-001-API-AST-AST-MEMORY-001     | API   | AST-MEMORY, REF-INJECT, IPC-CALL | src-tauri/src/ast/routes.rs | 6  |
| BR-SYS-DATA-007-QUERY-AST-AST-MEMORY-003, BR-AST-DATA-001-QUERY-AST-AST-MEMORY-003 | QUERY | AST-MEMORY, REF-INJECT           | src-tauri/src/ast/store.rs  | 72 |

#### BR-SYS-DATA-007 term_ref: TERM-REF-003

| codes                                                                                                                       | type | chain      | 文件                                 | 行号  |
| --------------------------------------------------------------------------------------------------------------------------- | ---- | ---------- | ---------------------------------- | --- |
| BR-REF-DATA-003-DATA-REF-REF-INJECT-002, BR-REF-VERIFY-001-DATA-REF-REF-INJECT-002, BR-SYS-DATA-007-DATA-REF-REF-INJECT-002 | DATA | REF-INJECT | src-tauri/src/reference/context.rs | 102 |

#### BR-SYS-OBS-001

| codes                                    | type | chain        | 文件                           | 行号 |
| ---------------------------------------- | ---- | ------------ | ---------------------------- | -- |
| BR-SYS-OBS-001-DATA-ENT-CLOUD-INVOKE-001 | DATA | CLOUD-INVOKE | src-tauri/src/ent/gateway.rs | 70 |

#### BR-SYS-OBS-002

| codes                                | type | chain    | 文件                       | 行号 |
| ------------------------------------ | ---- | -------- | ------------------------ | -- |
| BR-SYS-OBS-002-DATA-SYS-IPC-CALL-002 | DATA | IPC-CALL | src-tauri/src/ipc/mod.rs | 14 |

#### BR-SYS-PERSIST-001

| codes                                          | type   | chain        | 文件                      | 行号  |
| ---------------------------------------------- | ------ | ------------ | ----------------------- | --- |
| BR-SYS-PERSIST-001-EFFECT-SYS-DATA-MIGRATE-001 | EFFECT | DATA-MIGRATE | src-tauri/src/db/mod.rs | 433 |

#### BR-SYS-PERSIST-001 term_ref: TERM-AG-019

| codes                                                                                    | type   | chain                          | 文件                             | 行号  |
| ---------------------------------------------------------------------------------------- | ------ | ------------------------------ | ------------------------------ | --- |
| BR-AG-DATA-003-EFFECT-AG-AG-TARGETSET-002, BR-SYS-PERSIST-001-EFFECT-AG-AG-TARGETSET-002 | EFFECT | AG-TURN, AG-TOOL, DATA-MIGRATE | src-tauri/src/ag/target_set.rs | 202 |

#### BR-SYS-STATE-001

| codes                                  | type | chain      | 文件                           | 行号 |
| -------------------------------------- | ---- | ---------- | ---------------------------- | -- |
| BR-SYS-STATE-001-RB-SYS-CLOUD-AUTH-001 | RB   | CLOUD-AUTH | src-tauri/src/session/mod.rs | 40 |

#### BR-SYS-STATE-002

| codes                                  | type | chain    | 文件                       | 行号 |
| -------------------------------------- | ---- | -------- | ------------------------ | -- |
| BR-SYS-STATE-002-DATA-SYS-IPC-CALL-003 | DATA | IPC-CALL | src-tauri/src/ipc/mod.rs | 76 |

#### BR-SYS-STATE-003

| codes                                    | type   | chain    | 文件                         | 行号 |
| ---------------------------------------- | ------ | -------- | -------------------------- | -- |
| BR-SYS-STATE-003-EFFECT-SYS-IPC-CALL-001 | EFFECT | IPC-CALL | src-tauri/src/sm_events.rs | 48 |

#### BR-SYS-STATE-004

| codes                                     | type   | chain     | 文件                                | 行号  |
| ----------------------------------------- | ------ | --------- | --------------------------------- | --- |
| BR-SYS-STATE-004-GUARD-SYS-APP-CLOSE-003  | GUARD  | APP-CLOSE | src/shell/App.tsx                 | 38  |
| BR-SYS-STATE-004-GUARD-SYS-APP-CLOSE-004  | GUARD  | APP-CLOSE | src-tauri/src/lib.rs              | 482 |
| BR-SYS-STATE-004-QUERY-SYS-APP-CLOSE-001  | QUERY  | APP-CLOSE | src-tauri/src/workspace/routes.rs | 874 |
| BR-SYS-STATE-004-EFFECT-SYS-APP-CLOSE-002 | EFFECT | APP-CLOSE | src-tauri/src/workspace/routes.rs | 938 |

#### BR-SYS-STATE-005

| codes                                        | type   | chain        | 文件                            | 行号  |
| -------------------------------------------- | ------ | ------------ | ----------------------------- | --- |
| BR-SYS-STATE-005-EFFECT-SYS-DATA-MIGRATE-002 | EFFECT | DATA-MIGRATE | src-tauri/src/de/lifecycle.rs | 199 |
| BR-SYS-STATE-005-EFFECT-SYS-DATA-MIGRATE-003 | EFFECT | DATA-MIGRATE | src-tauri/src/de/lifecycle.rs | 237 |

#### BR-SYS-STATE-006

| codes                                     | type   | chain     | 文件                           | 行号  |
| ----------------------------------------- | ------ | --------- | ---------------------------- | --- |
| BR-SYS-STATE-006-EFFECT-SYS-APP-CLOSE-001 | EFFECT | APP-CLOSE | src/services/closeService.ts | 230 |

#### BR-SYS-STATE-007

| codes                                                                                                                         | type   | chain            | 文件                               | 行号   |
| ----------------------------------------------------------------------------------------------------------------------------- | ------ | ---------------- | -------------------------------- | ---- |
| BR-SYS-STATE-007-EFFECT-AG-AG-TURN-035, BR-AG-STATE-001-EFFECT-AG-AG-TURN-035                                                 | EFFECT | AG-TURN          | src/services/agService.ts        | 1445 |
| BR-AG-DATA-003-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-007-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-008-DATA-AG-AG-TARGETSET-001 | DATA   | AG-TURN, AG-TOOL | src-tauri/src/ag/target_set.rs   | 32   |
| BR-AG-STATE-001-RB-AG-AG-TURN-001, BR-SYS-STATE-007-RB-AG-AG-TURN-001                                                         | RB     | AG-TURN          | src-tauri/src/ag/turn.rs         | 124  |
| BR-SYS-STATE-007-GUARD-AG-AG-TURN-034, BR-AG-STATE-001-GUARD-AG-AG-TURN-034                                                   | GUARD  | AG-TURN          | src-tauri/src/ag/turn_control.rs | 53   |
| BR-SYS-STATE-007-EFFECT-AG-AG-TURN-034, BR-AG-STATE-001-EFFECT-AG-AG-TURN-034                                                 | EFFECT | AG-TURN          | src-tauri/src/ag/turn_control.rs | 179  |

#### BR-SYS-STATE-007 term_ref: TERM-AG-001

| codes                                                                        | type   | chain            | 文件                               | 行号  |
| ---------------------------------------------------------------------------- | ------ | ---------------- | -------------------------------- | --- |
| BR-AG-DATA-003-EFFECT-AG-AG-TURN-036, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-036 | EFFECT | AG-TURN          | src-tauri/src/ag/turn_control.rs | 270 |
| BR-AG-STATE-001-API-AG-AG-TURN-018, BR-SYS-STATE-007-API-AG-AG-TURN-018      | API    | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_runtime.rs | 47  |

#### BR-SYS-STATE-007 term_ref: TERM-AG-012

| codes                                                                                                         | type | chain            | 文件                                | 行号 |
| ------------------------------------------------------------------------------------------------------------- | ---- | ---------------- | --------------------------------- | -- |
| BR-AG-STATE-002-DATA-AG-AG-TOOL-034, BR-AG-DATA-004-DATA-AG-AG-TOOL-034, BR-SYS-STATE-007-DATA-AG-AG-TOOL-034 | DATA | AG-TURN, AG-TOOL | src-tauri/src/ag/tool_registry.rs | 49 |

#### BR-SYS-STATE-007 term_ref: TERM-AG-013

| codes                                 | type  | chain            | 文件                               | 行号 |
| ------------------------------------- | ----- | ---------------- | -------------------------------- | -- |
| BR-SYS-STATE-007-GUARD-AG-AG-TURN-037 | GUARD | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_runtime.rs | 25 |

#### BR-SYS-STATE-007 term_ref: TERM-AG-014

| codes                                                                       | type  | chain   | 文件                                | 行号  |
| --------------------------------------------------------------------------- | ----- | ------- | --------------------------------- | --- |
| BR-AG-STATE-002-GUARD-AG-AG-TOOL-035, BR-SYS-STATE-007-GUARD-AG-AG-TOOL-035 | GUARD | AG-TOOL | src-tauri/src/ag/tool_registry.rs | 176 |

#### BR-SYS-STATE-008

| codes                                                                                                                                                                                                                                                | type   | chain                          | 文件                               | 行号   |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------ | -------------------------------- | ---- |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-006, BR-AG-DATA-001-EFFECT-AG-AG-TURN-006                                                                                                                                                                         | EFFECT | AG-TURN, DE-DIFF, ED-EDIT      | src/services/agService.ts        | 1060 |
| BR-SYS-STATE-008-EFFECT-DE-DE-DIFF-008                                                                                                                                                                                                               | EFFECT | DE-DIFF, ED-EDIT               | src/services/deService.ts        | 108  |
| BR-SYS-STATE-008-RB-AG-AG-TASK-001, BR-AG-STATE-001-RB-AG-AG-TASK-001                                                                                                                                                                                | RB     | AG-TURN                        | src-tauri/src/ag/task.rs         | 52   |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-032, BR-AG-DATA-001-EFFECT-AG-AG-TURN-032                                                                                                                                                                         | EFFECT | AG-TURN, DE-DIFF, ED-EDIT      | src-tauri/src/ag/task.rs         | 148  |
| BR-SYS-STATE-008-GUARD-AG-AG-TURN-005, BR-AG-DATA-001-GUARD-AG-AG-TURN-005                                                                                                                                                                           | GUARD  | AG-TURN, DE-DIFF, ED-EDIT      | src-tauri/src/ag/task.rs         | 197  |
| BR-SYS-STATE-008-GUARD-AG-AG-TURN-006, BR-AG-DATA-001-GUARD-AG-AG-TURN-006                                                                                                                                                                           | GUARD  | AG-TURN, ED-EDIT               | src-tauri/src/ag/task.rs         | 279  |
| BR-SYS-STATE-008-EFFECT-AG-AG-DECISION-001, BR-AG-DATA-003-EFFECT-AG-AG-DECISION-001                                                                                                                                                                 | EFFECT | AG-TURN, AG-TOOL               | src-tauri/src/ag/task.rs         | 546  |
| BR-SYS-STATE-008-EFFECT-AG-AG-DECISION-002, BR-AG-DATA-003-EFFECT-AG-AG-DECISION-002                                                                                                                                                                 | EFFECT | AG-TURN, AG-TOOL               | src-tauri/src/ag/task.rs         | 607  |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-033, BR-AG-STATE-001-EFFECT-AG-AG-TURN-033                                                                                                                                                                        | EFFECT | AG-TURN                        | src-tauri/src/ag/task.rs         | 760  |
| BR-SYS-STATE-008-GUARD-AG-AG-TASKPLAN-001, BR-AG-STATE-001-GUARD-AG-AG-TASKPLAN-001, BR-AG-STATE-002-GUARD-AG-AG-TASKPLAN-001                                                                                                                        | GUARD  | AG-TURN                        | src-tauri/src/ag/task.rs         | 1068 |
| BR-SYS-STATE-008-EFFECT-AG-AG-TASKPLAN-001, BR-AG-STATE-001-EFFECT-AG-AG-TASKPLAN-001                                                                                                                                                                | EFFECT | AG-TURN                        | src-tauri/src/ag/task.rs         | 1127 |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-026, BR-AG-DATA-003-EFFECT-AG-AG-TURN-026                                                                                                                                                                         | EFFECT | AG-TURN                        | src-tauri/src/ag/task_runtime.rs | 78   |
| BR-AG-STATE-002-API-AG-WS-OPERATION-005, BR-AG-DATA-003-API-AG-WS-OPERATION-005, BR-SYS-STATE-008-API-AG-WS-OPERATION-005, BR-WS-STATE-003-API-AG-WS-OPERATION-005, BR-WS-PERSIST-003-API-AG-WS-OPERATION-005, BR-WS-OBS-002-API-AG-WS-OPERATION-005 | API    | AG-TURN, AG-TOOL, WS-OPERATION | src-tauri/src/ag/task_runtime.rs | 173  |
| BR-SYS-STATE-008-API-AG-AG-TURN-003, BR-AG-DATA-001-API-AG-AG-TURN-003                                                                                                                                                                               | API    | AG-TURN, DE-DIFF, ED-EDIT      | src-tauri/src/ag/turn_control.rs | 401  |

#### BR-SYS-STATE-008 term_ref: TERM-AG-014

| codes                                                                 | type | chain            | 文件                                 | 行号 |
| --------------------------------------------------------------------- | ---- | ---------------- | ---------------------------------- | -- |
| BR-AG-STATE-002-RB-AG-AG-TOOL-018, BR-SYS-STATE-008-RB-AG-AG-TOOL-018 | RB   | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_tool_loop.rs | 93 |

#### BR-SYS-STATE-008 term_ref: TERM-AG-015

| codes                                  | type   | chain            | 文件                               | 行号  |
| -------------------------------------- | ------ | ---------------- | -------------------------------- | --- |
| BR-SYS-STATE-008-EFFECT-AG-AG-TASK-002 | EFFECT | AG-TURN, AG-TOOL | src-tauri/src/ag/task.rs         | 109 |
| BR-SYS-STATE-008-GUARD-AG-AG-TASK-003  | GUARD  | AG-TURN, AG-TOOL | src-tauri/src/ag/task.rs         | 903 |
| BR-SYS-STATE-008-QUERY-AG-AG-TURN-025  | QUERY  | AG-TURN          | src-tauri/src/ag/task_runtime.rs | 45  |
| BR-SYS-STATE-008-QUERY-AG-AG-TURN-027  | QUERY  | AG-TURN          | src-tauri/src/ag/task_runtime.rs | 376 |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-028 | EFFECT | AG-TURN          | src-tauri/src/ag/task_runtime.rs | 398 |

#### BR-SYS-STATE-008 term_ref: TERM-AG-018

| codes                                                                    | type | chain   | 文件                               | 行号  |
| ------------------------------------------------------------------------ | ---- | ------- | -------------------------------- | --- |
| BR-AG-DATA-002-DATA-AG-AG-TURN-017, BR-SYS-STATE-008-DATA-AG-AG-TURN-017 | DATA | AG-TURN | src-tauri/src/ag/task_runtime.rs | 673 |

#### BR-SYS-STATE-008 term_ref: TERM-AG-019

| codes                                                                                                                         | type | chain            | 文件                             | 行号 |
| ----------------------------------------------------------------------------------------------------------------------------- | ---- | ---------------- | ------------------------------ | -- |
| BR-AG-DATA-003-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-007-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-008-DATA-AG-AG-TARGETSET-001 | DATA | AG-TURN, AG-TOOL | src-tauri/src/ag/target_set.rs | 32 |

#### BR-SYS-VERIFY-001

| codes                                                                                | type  | chain                    | 文件                           | 行号 |
| ------------------------------------------------------------------------------------ | ----- | ------------------------ | ---------------------------- | -- |
| BR-SYS-VERIFY-001-GUARD-SYS-CLOUD-GATE-001, BR-SYS-DATA-001-GUARD-SYS-CLOUD-GATE-001 | GUARD | CLOUD-GATE, CLOUD-INVOKE | src-tauri/src/session/mod.rs | 76 |

#### BR-TPL-DATA-001

| codes                                   | type   | chain    | 文件                            | 行号 |
| --------------------------------------- | ------ | -------- | ----------------------------- | -- |
| BR-TPL-DATA-001-EFFECT-TPL-TPL-FLOW-001 | EFFECT | TPL-FLOW | src-tauri/src/tpl/template.rs | 43 |
| BR-TPL-DATA-001-QUERY-TPL-TPL-FLOW-001  | QUERY  | TPL-FLOW | src-tauri/src/tpl/template.rs | 71 |
| BR-TPL-DATA-001-EFFECT-TPL-TPL-FLOW-002 | EFFECT | TPL-FLOW | src-tauri/src/tpl/template.rs | 91 |

#### BR-TPL-STATE-001

| codes                                    | type   | chain    | 文件                            | 行号  |
| ---------------------------------------- | ------ | -------- | ----------------------------- | --- |
| BR-TPL-STATE-001-RB-TPL-TPL-FLOW-001     | RB     | TPL-FLOW | src-tauri/src/tpl/workflow.rs | 49  |
| BR-TPL-STATE-001-EFFECT-TPL-TPL-FLOW-003 | EFFECT | TPL-FLOW | src-tauri/src/tpl/workflow.rs | 109 |
| BR-TPL-STATE-001-QUERY-TPL-TPL-FLOW-002  | QUERY  | TPL-FLOW | src-tauri/src/tpl/workflow.rs | 138 |

#### BR-WS-DATA-001

| codes                                                                                                              | type   | chain   | 文件                                 | 行号  |
| ------------------------------------------------------------------------------------------------------------------ | ------ | ------- | ---------------------------------- | --- |
| BR-WS-DATA-001-EFFECT-WS-WS-FS-007                                                                                 | EFFECT | WS-FS   | src-tauri/src/workspace/fileops.rs | 100 |
| BR-WS-DATA-001-EFFECT-WS-WS-FS-008                                                                                 | EFFECT | WS-FS   | src-tauri/src/workspace/listing.rs | 148 |
| BR-WS-DATA-001-GUARD-WS-WS-FS-001                                                                                  | GUARD  | WS-FS   | src-tauri/src/workspace/mod.rs     | 46  |
| BR-WS-PERSIST-001-EFFECT-WS-WS-SAVE-003, BR-WS-OBS-001-EFFECT-WS-WS-SAVE-003, BR-WS-DATA-001-EFFECT-WS-WS-SAVE-003 | EFFECT | WS-SAVE | src-tauri/src/workspace/routes.rs  | 337 |

#### BR-WS-DATA-001 term_ref: TERM-ED-008

| codes                                                                                                          | type  | chain       | 文件                               | 行号  |
| -------------------------------------------------------------------------------------------------------------- | ----- | ----------- | -------------------------------- | --- |
| BR-AG-STATE-002-QUERY-AG-AG-TOOL-012, BR-AG-DATA-001-QUERY-AG-AG-TOOL-012, BR-WS-DATA-001-QUERY-AG-AG-TOOL-012 | QUERY | AG-TOOL     | src-tauri/src/ag/tool_exec.rs    | 38  |
| BR-DOC-DATA-002-GUARD-DOC-DOC-CONVERT-011, BR-WS-DATA-001-GUARD-DOC-DOC-CONVERT-011                            | GUARD | DOC-CONVERT | src-tauri/src/doc/image_media.rs | 334 |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-015, BR-WS-DATA-001-IO-DOC-DOC-CONVERT-015                                  | IO    | DOC-CONVERT | src-tauri/src/doc/image_media.rs | 568 |

#### BR-WS-DATA-001 term_ref: TERM-WS-001

| codes                              | type   | chain | 文件                                 | 行号  |
| ---------------------------------- | ------ | ----- | ---------------------------------- | --- |
| BR-WS-DATA-001-EFFECT-WS-WS-FS-001 | EFFECT | WS-FS | src-tauri/src/workspace/fileops.rs | 16  |
| BR-WS-DATA-001-EFFECT-WS-WS-FS-004 | EFFECT | WS-FS | src-tauri/src/workspace/fileops.rs | 43  |
| BR-WS-DATA-001-EFFECT-WS-WS-FS-002 | EFFECT | WS-FS | src-tauri/src/workspace/fileops.rs | 70  |
| BR-WS-DATA-001-EFFECT-WS-WS-FS-003 | EFFECT | WS-FS | src-tauri/src/workspace/fileops.rs | 134 |
| BR-WS-DATA-001-QUERY-WS-WS-FS-002  | QUERY  | WS-FS | src-tauri/src/workspace/listing.rs | 174 |
| BR-WS-DATA-001-QUERY-WS-WS-FS-006  | QUERY  | WS-FS | src-tauri/src/workspace/listing.rs | 582 |
| BR-WS-DATA-001-QUERY-WS-WS-FS-005  | QUERY  | WS-FS | src-tauri/src/workspace/listing.rs | 703 |

#### BR-WS-DATA-002

| codes                                    | type   | chain       | 文件                                  | 行号  |
| ---------------------------------------- | ------ | ----------- | ----------------------------------- | --- |
| BR-WS-DATA-002-EFFECT-WS-WS-TIMELINE-001 | EFFECT | WS-TIMELINE | src-tauri/src/workspace/timeline.rs | 47  |
| BR-WS-DATA-002-QUERY-WS-WS-TIMELINE-001  | QUERY  | WS-TIMELINE | src-tauri/src/workspace/timeline.rs | 91  |
| BR-WS-DATA-002-EFFECT-WS-WS-TIMELINE-002 | EFFECT | WS-TIMELINE | src-tauri/src/workspace/timeline.rs | 138 |

#### BR-WS-DATA-003

| codes                                                                                                                                                                         | type   | chain                          | 文件                                   | 行号   |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------ | ------------------------------------ | ---- |
| BR-WS-DATA-003-EFFECT-WS-WS-TIMELINE-003, BR-WS-PERSIST-003-EFFECT-WS-WS-TIMELINE-003                                                                                         | EFFECT | WS-TIMELINE                    | src-tauri/src/workspace/operation.rs | 342  |
| BR-WS-STATE-003-API-WS-WS-OPERATION-001, BR-WS-DATA-003-API-WS-WS-OPERATION-001, BR-WS-OBS-002-API-WS-WS-OPERATION-001                                                        | API    | WS-OPERATION                   | src-tauri/src/workspace/operation.rs | 1955 |
| BR-WS-STATE-003-EFFECT-WS-WS-OPERATION-003, BR-WS-DATA-003-EFFECT-WS-WS-OPERATION-003, BR-WS-OBS-002-EFFECT-WS-WS-OPERATION-003, BR-WS-PERSIST-003-EFFECT-WS-WS-OPERATION-003 | EFFECT | WS-OPERATION                   | src-tauri/src/workspace/operation.rs | 2537 |
| BR-WS-STATE-003-API-WS-WS-OPERATION-002, BR-WS-DATA-003-API-WS-WS-OPERATION-002, BR-WS-OBS-002-API-WS-WS-OPERATION-002                                                        | API    | WS-OPERATION                   | src-tauri/src/workspace/routes.rs    | 1157 |
| BR-WS-STATE-003-API-WS-WS-OPERATION-004, BR-WS-DATA-003-API-WS-WS-OPERATION-004, BR-WS-OBS-002-API-WS-WS-OPERATION-004                                                        | API    | WS-OPERATION, DE-DIFF, ED-EDIT | src-tauri/src/workspace/routes.rs    | 1413 |

#### BR-WS-DATA-003 term_ref: TERM-WS-004

| codes                                     | type   | chain        | 文件                                   | 行号  |
| ----------------------------------------- | ------ | ------------ | ------------------------------------ | --- |
| BR-WS-DATA-003-EFFECT-WS-WS-OPERATION-001 | EFFECT | WS-OPERATION | src-tauri/src/workspace/operation.rs | 169 |

#### BR-WS-OBS-001

| codes                                                                                                              | type   | chain   | 文件                                | 行号   |
| ------------------------------------------------------------------------------------------------------------------ | ------ | ------- | --------------------------------- | ---- |
| BR-WS-PERSIST-001-EFFECT-WS-WS-SAVE-003, BR-WS-OBS-001-EFFECT-WS-WS-SAVE-003, BR-WS-DATA-001-EFFECT-WS-WS-SAVE-003 | EFFECT | WS-SAVE | src-tauri/src/workspace/routes.rs | 337  |
| BR-WS-OBS-001-RB-WS-WS-FS-001                                                                                      | RB     | WS-FS   | src-tauri/src/workspace/routes.rs | 1053 |

#### BR-WS-OBS-002

| codes                                                                                                                                                                         | type   | chain        | 文件                                   | 行号   |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------ | ------------------------------------ | ---- |
| BR-WS-STATE-003-EFFECT-WS-WS-OPERATION-003, BR-WS-DATA-003-EFFECT-WS-WS-OPERATION-003, BR-WS-OBS-002-EFFECT-WS-WS-OPERATION-003, BR-WS-PERSIST-003-EFFECT-WS-WS-OPERATION-003 | EFFECT | WS-OPERATION | src-tauri/src/workspace/operation.rs | 2537 |

#### BR-WS-OBS-002 term_ref: TERM-AG-015

| codes                                                                                                                                                                                                                                                | type | chain                          | 文件                               | 行号  |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---- | ------------------------------ | -------------------------------- | --- |
| BR-AG-STATE-002-API-AG-WS-OPERATION-005, BR-AG-DATA-003-API-AG-WS-OPERATION-005, BR-SYS-STATE-008-API-AG-WS-OPERATION-005, BR-WS-STATE-003-API-AG-WS-OPERATION-005, BR-WS-PERSIST-003-API-AG-WS-OPERATION-005, BR-WS-OBS-002-API-AG-WS-OPERATION-005 | API  | AG-TURN, AG-TOOL, WS-OPERATION | src-tauri/src/ag/task_runtime.rs | 173 |

#### BR-WS-OBS-002 term_ref: TERM-WS-003

| codes                                                                                                                  | type | chain                          | 文件                                   | 行号   |
| ---------------------------------------------------------------------------------------------------------------------- | ---- | ------------------------------ | ------------------------------------ | ---- |
| BR-WS-STATE-003-API-WS-WS-OPERATION-001, BR-WS-DATA-003-API-WS-WS-OPERATION-001, BR-WS-OBS-002-API-WS-WS-OPERATION-001 | API  | WS-OPERATION                   | src-tauri/src/workspace/operation.rs | 1955 |
| BR-WS-STATE-003-API-WS-WS-OPERATION-002, BR-WS-DATA-003-API-WS-WS-OPERATION-002, BR-WS-OBS-002-API-WS-WS-OPERATION-002 | API  | WS-OPERATION                   | src-tauri/src/workspace/routes.rs    | 1157 |
| BR-WS-STATE-003-API-WS-WS-OPERATION-004, BR-WS-DATA-003-API-WS-WS-OPERATION-004, BR-WS-OBS-002-API-WS-WS-OPERATION-004 | API  | WS-OPERATION, DE-DIFF, ED-EDIT | src-tauri/src/workspace/routes.rs    | 1413 |

#### BR-WS-PERSIST-001

| codes                                                                                                              | type   | chain   | 文件                                | 行号  |
| ------------------------------------------------------------------------------------------------------------------ | ------ | ------- | --------------------------------- | --- |
| BR-WS-PERSIST-001-EFFECT-WS-WS-SAVE-003, BR-WS-OBS-001-EFFECT-WS-WS-SAVE-003, BR-WS-DATA-001-EFFECT-WS-WS-SAVE-003 | EFFECT | WS-SAVE | src-tauri/src/workspace/routes.rs | 337 |
| BR-WS-PERSIST-001-EFFECT-WS-WS-SAVE-001                                                                            | EFFECT | WS-SAVE | src-tauri/src/workspace/save.rs   | 51  |
| BR-WS-PERSIST-001-RB-WS-WS-SAVE-002                                                                                | RB     | WS-SAVE | src-tauri/src/workspace/save.rs   | 103 |

#### BR-WS-PERSIST-002

| codes                                        | type   | chain        | 文件                                   | 行号  |
| -------------------------------------------- | ------ | ------------ | ------------------------------------ | --- |
| BR-WS-PERSIST-002-EFFECT-WS-DATA-MIGRATE-005 | EFFECT | DATA-MIGRATE | src/services/workspaceService.ts     | 193 |
| BR-WS-PERSIST-002-RB-SYS-DATA-MIGRATE-001    | RB     | DATA-MIGRATE | src/shell/App.tsx                    | 18  |
| BR-WS-PERSIST-002-RB-WS-DATA-MIGRATE-001     | RB     | DATA-MIGRATE | src/shell/Welcome.tsx                | 18  |
| BR-WS-PERSIST-002-RB-WS-DATA-MIGRATE-002     | RB     | DATA-MIGRATE | src/shell/Workbench.tsx              | 29  |
| BR-WS-PERSIST-002-EFFECT-WS-DATA-MIGRATE-004 | EFFECT | DATA-MIGRATE | src/store/workspaceShell.ts          | 46  |
| BR-WS-PERSIST-002-EFFECT-WS-DATA-MIGRATE-001 | EFFECT | DATA-MIGRATE | src-tauri/src/workspace/lifecycle.rs | 34  |
| BR-WS-PERSIST-002-EFFECT-WS-DATA-MIGRATE-002 | EFFECT | DATA-MIGRATE | src-tauri/src/workspace/lifecycle.rs | 71  |
| BR-WS-PERSIST-002-EFFECT-WS-DATA-MIGRATE-003 | EFFECT | DATA-MIGRATE | src-tauri/src/workspace/lifecycle.rs | 99  |
| BR-WS-PERSIST-002-QUERY-WS-DATA-MIGRATE-001  | QUERY  | DATA-MIGRATE | src-tauri/src/workspace/lifecycle.rs | 116 |
| BR-WS-PERSIST-002-QUERY-WS-DATA-MIGRATE-002  | QUERY  | DATA-MIGRATE | src-tauri/src/workspace/lifecycle.rs | 140 |

#### BR-WS-PERSIST-003

| codes                                                                                                                                                                                                                                                | type | chain                          | 文件                               | 行号  |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---- | ------------------------------ | -------------------------------- | --- |
| BR-AG-STATE-002-API-AG-WS-OPERATION-005, BR-AG-DATA-003-API-AG-WS-OPERATION-005, BR-SYS-STATE-008-API-AG-WS-OPERATION-005, BR-WS-STATE-003-API-AG-WS-OPERATION-005, BR-WS-PERSIST-003-API-AG-WS-OPERATION-005, BR-WS-OBS-002-API-AG-WS-OPERATION-005 | API  | AG-TURN, AG-TOOL, WS-OPERATION | src-tauri/src/ag/task_runtime.rs | 173 |

#### BR-WS-PERSIST-003 term_ref: TERM-WS-003

| codes                                                                                                                                                                         | type   | chain        | 文件                                   | 行号   |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------ | ------------------------------------ | ---- |
| BR-WS-STATE-003-RB-WS-WS-OPERATION-001, BR-WS-PERSIST-003-RB-WS-WS-OPERATION-001                                                                                              | RB     | WS-OPERATION | src-tauri/src/workspace/operation.rs | 139  |
| BR-WS-PERSIST-003-EFFECT-WS-WS-OPERATION-002                                                                                                                                  | EFFECT | WS-OPERATION | src-tauri/src/workspace/operation.rs | 1513 |
| BR-WS-STATE-003-EFFECT-WS-WS-OPERATION-003, BR-WS-DATA-003-EFFECT-WS-WS-OPERATION-003, BR-WS-OBS-002-EFFECT-WS-WS-OPERATION-003, BR-WS-PERSIST-003-EFFECT-WS-WS-OPERATION-003 | EFFECT | WS-OPERATION | src-tauri/src/workspace/operation.rs | 2537 |

#### BR-WS-PERSIST-003 term_ref: TERM-WS-005

| codes                                                                                 | type   | chain       | 文件                                   | 行号  |
| ------------------------------------------------------------------------------------- | ------ | ----------- | ------------------------------------ | --- |
| BR-WS-DATA-003-EFFECT-WS-WS-TIMELINE-003, BR-WS-PERSIST-003-EFFECT-WS-WS-TIMELINE-003 | EFFECT | WS-TIMELINE | src-tauri/src/workspace/operation.rs | 342 |

#### BR-WS-STATE-001

| codes                                       | type   | chain         | 文件                                    | 行号  |
| ------------------------------------------- | ------ | ------------- | ------------------------------------- | --- |
| BR-WS-STATE-001-RB-WS-WS-DIFF-APPLY-001     | RB     | WS-DIFF-APPLY | src-tauri/src/workspace/diff_apply.rs | 45  |
| BR-WS-STATE-001-EFFECT-WS-WS-DIFF-APPLY-001 | EFFECT | WS-DIFF-APPLY | src-tauri/src/workspace/diff_apply.rs | 76  |
| BR-WS-STATE-001-EFFECT-WS-WS-DIFF-APPLY-002 | EFFECT | WS-DIFF-APPLY | src-tauri/src/workspace/diff_apply.rs | 359 |
| BR-WS-STATE-001-EFFECT-WS-WS-DIFF-APPLY-003 | EFFECT | WS-DIFF-APPLY | src-tauri/src/workspace/diff_apply.rs | 432 |
| BR-WS-STATE-001-QUERY-WS-WS-DIFF-APPLY-001  | QUERY  | WS-DIFF-APPLY | src-tauri/src/workspace/diff_apply.rs | 511 |

#### BR-WS-STATE-002

| codes                                    | type  | chain       | 文件                                  | 行号  |
| ---------------------------------------- | ----- | ----------- | ----------------------------------- | --- |
| BR-WS-STATE-002-QUERY-WS-WS-TIMELINE-002 | QUERY | WS-TIMELINE | src-tauri/src/workspace/timeline.rs | 202 |
| BR-WS-STATE-002-RB-WS-WS-TIMELINE-001    | RB    | WS-TIMELINE | src-tauri/src/workspace/timeline.rs | 242 |

#### BR-WS-STATE-003

| codes                                                                                                                                                                                                                                                | type   | chain                          | 文件                                   | 行号   |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------ | ------------------------------------ | ---- |
| BR-AG-STATE-002-API-AG-WS-OPERATION-005, BR-AG-DATA-003-API-AG-WS-OPERATION-005, BR-SYS-STATE-008-API-AG-WS-OPERATION-005, BR-WS-STATE-003-API-AG-WS-OPERATION-005, BR-WS-PERSIST-003-API-AG-WS-OPERATION-005, BR-WS-OBS-002-API-AG-WS-OPERATION-005 | API    | AG-TURN, AG-TOOL, WS-OPERATION | src-tauri/src/ag/task_runtime.rs     | 173  |
| BR-WS-STATE-003-RB-WS-WS-OPERATION-001, BR-WS-PERSIST-003-RB-WS-WS-OPERATION-001                                                                                                                                                                     | RB     | WS-OPERATION                   | src-tauri/src/workspace/operation.rs | 139  |
| BR-WS-STATE-003-API-WS-WS-OPERATION-001, BR-WS-DATA-003-API-WS-WS-OPERATION-001, BR-WS-OBS-002-API-WS-WS-OPERATION-001                                                                                                                               | API    | WS-OPERATION                   | src-tauri/src/workspace/operation.rs | 1955 |
| BR-WS-STATE-003-EFFECT-WS-WS-OPERATION-003, BR-WS-DATA-003-EFFECT-WS-WS-OPERATION-003, BR-WS-OBS-002-EFFECT-WS-WS-OPERATION-003, BR-WS-PERSIST-003-EFFECT-WS-WS-OPERATION-003                                                                        | EFFECT | WS-OPERATION                   | src-tauri/src/workspace/operation.rs | 2537 |
| BR-WS-STATE-003-API-WS-WS-OPERATION-002, BR-WS-DATA-003-API-WS-WS-OPERATION-002, BR-WS-OBS-002-API-WS-WS-OPERATION-002                                                                                                                               | API    | WS-OPERATION                   | src-tauri/src/workspace/routes.rs    | 1157 |
| BR-WS-STATE-003-API-WS-WS-OPERATION-004, BR-WS-DATA-003-API-WS-WS-OPERATION-004, BR-WS-OBS-002-API-WS-WS-OPERATION-004                                                                                                                               | API    | WS-OPERATION, DE-DIFF, ED-EDIT | src-tauri/src/workspace/routes.rs    | 1413 |

#### BR-WS-STATE-009 term_ref: TERM-WS-003

| codes                               | type   | chain              | 文件                                   | 行号  |
| ----------------------------------- | ------ | ------------------ | ------------------------------------ | --- |
| BR-WS-STATE-009-EFFECT-WS-WS-FS-009 | EFFECT | WS-FS, WS-TIMELINE | src-tauri/src/workspace/operation.rs | 403 |
| BR-WS-STATE-009-EFFECT-WS-WS-FS-010 | EFFECT | WS-FS, WS-TIMELINE | src-tauri/src/workspace/operation.rs | 463 |

#### TERM-AG-001

| codes                                                                                                                         | type | chain            | 文件                             | 行号 |
| ----------------------------------------------------------------------------------------------------------------------------- | ---- | ---------------- | ------------------------------ | -- |
| BR-AG-DATA-003-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-007-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-008-DATA-AG-AG-TARGETSET-001 | DATA | AG-TURN, AG-TOOL | src-tauri/src/ag/target_set.rs | 32 |

#### TERM-AG-004

| codes                                   | type   | chain     | 文件                                   | 行号   |
| --------------------------------------- | ------ | --------- | ------------------------------------ | ---- |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-007     | RB     | AG-ASSIST | src/components/ed/Editor.tsx         | 810  |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-002     | RB     | AG-ASSIST | src/components/ed/Editor.tsx         | 1890 |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-004     | RB     | AG-ASSIST | src/components/ed/assistPlacement.ts | 36   |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-005     | RB     | AG-ASSIST | src/components/ed/assistPlacement.ts | 60   |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-006     | RB     | AG-ASSIST | src/components/ed/assistPlacement.ts | 78   |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-003     | RB     | AG-ASSIST | src/components/ed/assistPlacement.ts | 96   |
| BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-002 | EFFECT | AG-ASSIST | src/services/agService.ts            | 1380 |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-008     | RB     | AG-ASSIST | src/store/assistStore.ts             | 94   |

#### TERM-AG-009

| codes                                   | type   | chain   | 文件                          | 行号  |
| --------------------------------------- | ------ | ------- | --------------------------- | --- |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-003 | EFFECT | AG-TURN | src-tauri/src/ag/history.rs | 295 |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-003  | QUERY  | AG-TURN | src-tauri/src/ag/history.rs | 357 |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-031 | EFFECT | AG-TURN | src-tauri/src/ag/history.rs | 396 |

#### TERM-AG-012

| codes                                                                   | type | chain            | 文件                               | 行号 |
| ----------------------------------------------------------------------- | ---- | ---------------- | -------------------------------- | -- |
| BR-AG-STATE-001-API-AG-AG-TURN-018, BR-SYS-STATE-007-API-AG-AG-TURN-018 | API  | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_runtime.rs | 47 |

#### TERM-AG-013

| codes                                                                                                         | type | chain            | 文件                                | 行号 |
| ------------------------------------------------------------------------------------------------------------- | ---- | ---------------- | --------------------------------- | -- |
| BR-AG-STATE-002-DATA-AG-AG-TOOL-034, BR-AG-DATA-004-DATA-AG-AG-TOOL-034, BR-SYS-STATE-007-DATA-AG-AG-TOOL-034 | DATA | AG-TURN, AG-TOOL | src-tauri/src/ag/tool_registry.rs | 49 |
| BR-AG-STATE-001-API-AG-AG-TURN-018, BR-SYS-STATE-007-API-AG-AG-TURN-018                                       | API  | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_runtime.rs  | 47 |

#### TERM-AG-014

| codes                                                                                                         | type | chain            | 文件                                | 行号 |
| ------------------------------------------------------------------------------------------------------------- | ---- | ---------------- | --------------------------------- | -- |
| BR-AG-STATE-002-DATA-AG-AG-TOOL-034, BR-AG-DATA-004-DATA-AG-AG-TOOL-034, BR-SYS-STATE-007-DATA-AG-AG-TOOL-034 | DATA | AG-TURN, AG-TOOL | src-tauri/src/ag/tool_registry.rs | 49 |
| BR-AG-STATE-001-API-AG-AG-TURN-018, BR-SYS-STATE-007-API-AG-AG-TURN-018                                       | API  | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_runtime.rs  | 47 |

#### TERM-AG-015

| codes                                                                                                                         | type   | chain                          | 文件                                 | 行号  |
| ----------------------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------ | ---------------------------------- | --- |
| BR-AG-DATA-003-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-007-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-008-DATA-AG-AG-TARGETSET-001 | DATA   | AG-TURN, AG-TOOL               | src-tauri/src/ag/target_set.rs     | 32  |
| BR-AG-DATA-003-EFFECT-AG-AG-TARGETSET-002, BR-SYS-PERSIST-001-EFFECT-AG-AG-TARGETSET-002                                      | EFFECT | AG-TURN, AG-TOOL, DATA-MIGRATE | src-tauri/src/ag/target_set.rs     | 202 |
| BR-AG-STATE-002-DATA-AG-AG-TOOL-034, BR-AG-DATA-004-DATA-AG-AG-TOOL-034, BR-SYS-STATE-007-DATA-AG-AG-TOOL-034                 | DATA   | AG-TURN, AG-TOOL               | src-tauri/src/ag/tool_registry.rs  | 49  |
| BR-SYS-STATE-007-GUARD-AG-AG-TURN-034, BR-AG-STATE-001-GUARD-AG-AG-TURN-034                                                   | GUARD  | AG-TURN                        | src-tauri/src/ag/turn_control.rs   | 53  |
| BR-SYS-STATE-007-EFFECT-AG-AG-TURN-034, BR-AG-STATE-001-EFFECT-AG-AG-TURN-034                                                 | EFFECT | AG-TURN                        | src-tauri/src/ag/turn_control.rs   | 179 |
| BR-AG-STATE-001-API-AG-AG-TURN-018, BR-SYS-STATE-007-API-AG-AG-TURN-018                                                       | API    | AG-TURN, AG-TOOL               | src-tauri/src/ag/turn_runtime.rs   | 47  |
| BR-AG-STATE-002-RB-AG-AG-TOOL-018, BR-SYS-STATE-008-RB-AG-AG-TOOL-018                                                         | RB     | AG-TURN, AG-TOOL               | src-tauri/src/ag/turn_tool_loop.rs | 93  |

#### TERM-AG-016

| codes                                                                   | type | chain            | 文件                                 | 行号 |
| ----------------------------------------------------------------------- | ---- | ---------------- | ---------------------------------- | -- |
| BR-AG-STATE-001-API-AG-AG-TURN-018, BR-SYS-STATE-007-API-AG-AG-TURN-018 | API  | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_runtime.rs   | 47 |
| BR-AG-STATE-002-RB-AG-AG-TOOL-018, BR-SYS-STATE-008-RB-AG-AG-TOOL-018   | RB   | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_tool_loop.rs | 93 |

#### TERM-AG-017

| codes                                                                                                                                                                                                                                                | type   | chain                          | 文件                                | 行号  |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------ | --------------------------------- | --- |
| BR-SYS-STATE-008-EFFECT-AG-AG-DECISION-001, BR-AG-DATA-003-EFFECT-AG-AG-DECISION-001                                                                                                                                                                 | EFFECT | AG-TURN, AG-TOOL               | src-tauri/src/ag/task.rs          | 546 |
| BR-SYS-STATE-008-EFFECT-AG-AG-DECISION-002, BR-AG-DATA-003-EFFECT-AG-AG-DECISION-002                                                                                                                                                                 | EFFECT | AG-TURN, AG-TOOL               | src-tauri/src/ag/task.rs          | 607 |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-026, BR-AG-DATA-003-EFFECT-AG-AG-TURN-026                                                                                                                                                                         | EFFECT | AG-TURN                        | src-tauri/src/ag/task_runtime.rs  | 78  |
| BR-AG-STATE-002-API-AG-WS-OPERATION-005, BR-AG-DATA-003-API-AG-WS-OPERATION-005, BR-SYS-STATE-008-API-AG-WS-OPERATION-005, BR-WS-STATE-003-API-AG-WS-OPERATION-005, BR-WS-PERSIST-003-API-AG-WS-OPERATION-005, BR-WS-OBS-002-API-AG-WS-OPERATION-005 | API    | AG-TURN, AG-TOOL, WS-OPERATION | src-tauri/src/ag/task_runtime.rs  | 173 |
| BR-AG-STATE-002-GUARD-AG-AG-TOOL-035, BR-SYS-STATE-007-GUARD-AG-AG-TOOL-035                                                                                                                                                                          | GUARD  | AG-TOOL                        | src-tauri/src/ag/tool_registry.rs | 176 |
| BR-AG-DATA-003-GUARD-AG-AG-TOOL-033, BR-AG-STATE-002-GUARD-AG-AG-TOOL-033                                                                                                                                                                            | GUARD  | AG-TOOL                        | src-tauri/src/ag/tool_registry.rs | 311 |

#### TERM-AG-018

| codes                                  | type   | chain            | 文件                               | 行号  |
| -------------------------------------- | ------ | ---------------- | -------------------------------- | --- |
| BR-SYS-STATE-008-EFFECT-AG-AG-TASK-002 | EFFECT | AG-TURN, AG-TOOL | src-tauri/src/ag/task.rs         | 109 |
| BR-SYS-STATE-008-GUARD-AG-AG-TASK-003  | GUARD  | AG-TURN, AG-TOOL | src-tauri/src/ag/task.rs         | 903 |
| BR-SYS-STATE-008-QUERY-AG-AG-TURN-027  | QUERY  | AG-TURN          | src-tauri/src/ag/task_runtime.rs | 376 |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-028 | EFFECT | AG-TURN          | src-tauri/src/ag/task_runtime.rs | 398 |

#### TERM-AG-019

| codes                                                                                                                                                                                                                                                | type   | chain                          | 文件                               | 行号  |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------ | -------------------------------- | --- |
| BR-SYS-STATE-008-EFFECT-AG-AG-DECISION-001, BR-AG-DATA-003-EFFECT-AG-AG-DECISION-001                                                                                                                                                                 | EFFECT | AG-TURN, AG-TOOL               | src-tauri/src/ag/task.rs         | 546 |
| BR-SYS-STATE-008-EFFECT-AG-AG-DECISION-002, BR-AG-DATA-003-EFFECT-AG-AG-DECISION-002                                                                                                                                                                 | EFFECT | AG-TURN, AG-TOOL               | src-tauri/src/ag/task.rs         | 607 |
| BR-AG-DATA-003-EFFECT-AG-AG-TURN-037                                                                                                                                                                                                                 | EFFECT | AG-TURN, WS-OPERATION          | src-tauri/src/ag/task_runtime.rs | 16  |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-026, BR-AG-DATA-003-EFFECT-AG-AG-TURN-026                                                                                                                                                                         | EFFECT | AG-TURN                        | src-tauri/src/ag/task_runtime.rs | 78  |
| BR-AG-STATE-002-API-AG-WS-OPERATION-005, BR-AG-DATA-003-API-AG-WS-OPERATION-005, BR-SYS-STATE-008-API-AG-WS-OPERATION-005, BR-WS-STATE-003-API-AG-WS-OPERATION-005, BR-WS-PERSIST-003-API-AG-WS-OPERATION-005, BR-WS-OBS-002-API-AG-WS-OPERATION-005 | API    | AG-TURN, AG-TOOL, WS-OPERATION | src-tauri/src/ag/task_runtime.rs | 173 |
| BR-SYS-STATE-007-GUARD-AG-AG-TURN-034, BR-AG-STATE-001-GUARD-AG-AG-TURN-034                                                                                                                                                                          | GUARD  | AG-TURN                        | src-tauri/src/ag/turn_control.rs | 53  |
| BR-AG-DATA-003-EFFECT-AG-AG-TURN-036, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-036                                                                                                                                                                         | EFFECT | AG-TURN                        | src-tauri/src/ag/turn_control.rs | 270 |

#### TERM-DE-002

| codes                                    | type  | chain            | 文件                             | 行号  |
| ---------------------------------------- | ----- | ---------------- | ------------------------------ | --- |
| BR-AG-DATA-003-GUARD-AG-AG-TARGETSET-003 | GUARD | AG-TOOL, DE-DIFF | src-tauri/src/ag/target_set.rs | 272 |

#### TERM-DE-003

| codes                                                                                                                  | type | chain                          | 文件                                | 行号   |
| ---------------------------------------------------------------------------------------------------------------------- | ---- | ------------------------------ | --------------------------------- | ---- |
| BR-DE-STATE-001-RB-DE-DE-DIFF-002                                                                                      | RB   | DE-DIFF                        | src/store/deStore.ts              | 257  |
| BR-WS-STATE-003-API-WS-WS-OPERATION-004, BR-WS-DATA-003-API-WS-WS-OPERATION-004, BR-WS-OBS-002-API-WS-WS-OPERATION-004 | API  | WS-OPERATION, DE-DIFF, ED-EDIT | src-tauri/src/workspace/routes.rs | 1413 |

#### TERM-DOC-003

| codes                                                                    | type  | chain   | 文件                         | 行号  |
| ------------------------------------------------------------------------ | ----- | ------- | -------------------------- | --- |
| BR-AG-DATA-001-QUERY-AG-AG-TOOL-011, BR-ED-DATA-003-QUERY-AG-AG-TOOL-011 | QUERY | AG-TOOL | src-tauri/src/ag/blocks.rs | 190 |

#### TERM-DOC-005

| codes                                | type   | chain            | 文件                                               | 行号  |
| ------------------------------------ | ------ | ---------------- | ------------------------------------------------ | --- |
| BR-ED-DATA-007-DATA-ED-ED-EDIT-018   | DATA   | ED-EDIT          | src/components/ed/extensions/TableCellShading.ts | 37  |
| BR-ED-DATA-007-RB-ED-ED-EDIT-018     | RB     | ED-EDIT, DE-DIFF | src/services/edTableOps.ts                       | 407 |
| BR-ED-DATA-007-EFFECT-ED-ED-EDIT-014 | EFFECT | ED-EDIT, DE-DIFF | src/services/edTableOps.ts                       | 438 |

#### TERM-ED-001

| codes                                                                     | type  | chain   | 文件                         | 行号  |
| ------------------------------------------------------------------------- | ----- | ------- | -------------------------- | --- |
| BR-AG-DATA-001-QUERY-AG-AG-TURN-010, BR-DOC-DATA-003-QUERY-AG-AG-TURN-010 | QUERY | AG-TURN | src-tauri/src/ag/blocks.rs | 415 |

#### TERM-ED-003

| codes                              | type | chain    | 文件                                                      | 行号  |
| ---------------------------------- | ---- | -------- | ------------------------------------------------------- | --- |
| BR-ED-STATE-002-RB-ED-ED-DECOR-001 | RB   | ED-DECOR | src/components/ed/extensions/DiffDecorationExtension.ts | 155 |

#### TERM-ED-004

| codes                                                                     | type   | chain   | 文件                                     | 行号   |
| ------------------------------------------------------------------------- | ------ | ------- | -------------------------------------- | ---- |
| BR-ED-STATE-001-EFFECT-ED-ED-EDIT-012                                     | EFFECT | ED-EDIT | src/services/edLocate.ts               | 345  |
| BR-ED-DATA-005-RB-ED-ED-EDIT-008                                          | RB     | ED-EDIT | src/services/edStyleOps.ts             | 243  |
| BR-SYS-DATA-006-GUARD-AG-AG-TOOL-021, BR-ED-DATA-005-GUARD-AG-AG-TOOL-021 | GUARD  | AG-TOOL | src-tauri/src/ag/bdocx_capabilities.rs | 1566 |
| BR-ED-DATA-005-QUERY-ED-ED-EDIT-009                                       | QUERY  | ED-EDIT | src-tauri/src/ag/blocks.rs             | 751  |

#### TERM-ED-005

| codes                               | type  | chain   | 文件                       | 行号  |
| ----------------------------------- | ----- | ------- | ------------------------ | --- |
| BR-ED-DATA-004-QUERY-ED-ED-EDIT-010 | QUERY | ED-EDIT | src/services/edLocate.ts | 231 |
| BR-ED-DATA-004-QUERY-ED-ED-EDIT-007 | QUERY | ED-EDIT | src/services/edLocate.ts | 414 |

#### TERM-ED-006

| codes                                                                     | type  | chain   | 文件                                     | 行号   |
| ------------------------------------------------------------------------- | ----- | ------- | -------------------------------------- | ---- |
| BR-SYS-DATA-006-GUARD-AG-AG-TOOL-021, BR-ED-DATA-005-GUARD-AG-AG-TOOL-021 | GUARD | AG-TOOL | src-tauri/src/ag/bdocx_capabilities.rs | 1566 |

#### TERM-ED-008

| codes                                                                                                                  | type   | chain                          | 文件                                     | 行号   |
| ---------------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------ | -------------------------------------- | ---- |
| BR-DE-DATA-002-RB-DE-DE-DIFF-003                                                                                       | RB     | DE-DIFF                        | src/components/de/DiffCard.tsx         | 241  |
| BR-AG-DATA-001-EFFECT-AG-AG-TURN-005                                                                                   | EFFECT | AG-TURN                        | src/services/agService.ts              | 298  |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-006, BR-AG-DATA-001-EFFECT-AG-AG-TURN-006                                           | EFFECT | AG-TURN, DE-DIFF, ED-EDIT      | src/services/agService.ts              | 1060 |
| BR-DE-DATA-002-EFFECT-DE-DE-DIFF-009                                                                                   | EFFECT | DE-DIFF                        | src/services/deService.ts              | 147  |
| BR-SYS-DATA-006-RB-ED-ED-EDIT-027, BR-ED-DATA-003-RB-ED-ED-EDIT-027                                                    | RB     | ED-EDIT                        | src/services/edImageOps.ts             | 256  |
| BR-DE-DATA-002-RB-ED-DE-DIFF-028                                                                                       | RB     | DE-DIFF                        | src/services/edImageOps.ts             | 493  |
| BR-SYS-DATA-006-DATA-AG-AG-TOOL-002                                                                                    | DATA   | AG-TOOL                        | src-tauri/src/ag/bdocx_capabilities.rs | 897  |
| BR-SYS-DATA-006-GUARD-AG-AG-TOOL-022, BR-ED-DATA-003-GUARD-AG-AG-TOOL-022                                              | GUARD  | AG-TOOL                        | src-tauri/src/ag/bdocx_capabilities.rs | 1918 |
| BR-SYS-DATA-006-GUARD-AG-AG-TOOL-004, BR-ED-DATA-001-GUARD-AG-AG-TOOL-004, BR-ED-DATA-003-GUARD-AG-AG-TOOL-004         | GUARD  | AG-TOOL                        | src-tauri/src/ag/edit_image_runtime.rs | 8    |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-032, BR-AG-DATA-001-EFFECT-AG-AG-TURN-032                                           | EFFECT | AG-TURN, DE-DIFF, ED-EDIT      | src-tauri/src/ag/task.rs               | 148  |
| BR-SYS-STATE-008-GUARD-AG-AG-TURN-005, BR-AG-DATA-001-GUARD-AG-AG-TURN-005                                             | GUARD  | AG-TURN, DE-DIFF, ED-EDIT      | src-tauri/src/ag/task.rs               | 197  |
| BR-SYS-STATE-008-GUARD-AG-AG-TURN-006, BR-AG-DATA-001-GUARD-AG-AG-TURN-006                                             | GUARD  | AG-TURN, ED-EDIT               | src-tauri/src/ag/task.rs               | 279  |
| BR-SYS-STATE-008-API-AG-AG-TURN-003, BR-AG-DATA-001-API-AG-AG-TURN-003                                                 | API    | AG-TURN, DE-DIFF, ED-EDIT      | src-tauri/src/ag/turn_control.rs       | 401  |
| BR-DE-DATA-002-EFFECT-WS-DE-DIFF-004                                                                                   | EFFECT | DE-DIFF                        | src-tauri/src/workspace/diff_apply.rs  | 219  |
| BR-WS-STATE-003-API-WS-WS-OPERATION-004, BR-WS-DATA-003-API-WS-WS-OPERATION-004, BR-WS-OBS-002-API-WS-WS-OPERATION-004 | API    | WS-OPERATION, DE-DIFF, ED-EDIT | src-tauri/src/workspace/routes.rs      | 1413 |

#### TERM-REF-003

| codes                                                                                                                         | type | chain            | 文件                               | 行号  |
| ----------------------------------------------------------------------------------------------------------------------------- | ---- | ---------------- | -------------------------------- | --- |
| BR-REF-DATA-004-DATA-AG-REF-INJECT-033                                                                                        | DATA | REF-INJECT       | src/components/ag/Composer.tsx   | 40  |
| BR-REF-DATA-004-RB-REF-REF-INJECT-001                                                                                         | RB   | REF-INJECT       | src/services/referencePayload.ts | 439 |
| BR-AG-DATA-003-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-007-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-008-DATA-AG-AG-TARGETSET-001 | DATA | AG-TURN, AG-TOOL | src-tauri/src/ag/target_set.rs   | 32  |

#### TERM-REF-005

| codes                                                                                                                       | type | chain      | 文件                                 | 行号  |
| --------------------------------------------------------------------------------------------------------------------------- | ---- | ---------- | ---------------------------------- | --- |
| BR-REF-DATA-003-DATA-REF-REF-INJECT-002, BR-REF-VERIFY-001-DATA-REF-REF-INJECT-002, BR-SYS-DATA-007-DATA-REF-REF-INJECT-002 | DATA | REF-INJECT | src-tauri/src/reference/context.rs | 102 |

#### TERM-WS-001

| codes                                                                                                          | type  | chain       | 文件                               | 行号  |
| -------------------------------------------------------------------------------------------------------------- | ----- | ----------- | -------------------------------- | --- |
| BR-AG-STATE-002-QUERY-AG-AG-TOOL-012, BR-AG-DATA-001-QUERY-AG-AG-TOOL-012, BR-WS-DATA-001-QUERY-AG-AG-TOOL-012 | QUERY | AG-TOOL     | src-tauri/src/ag/tool_exec.rs    | 38  |
| BR-DOC-DATA-002-GUARD-DOC-DOC-CONVERT-011, BR-WS-DATA-001-GUARD-DOC-DOC-CONVERT-011                            | GUARD | DOC-CONVERT | src-tauri/src/doc/image_media.rs | 334 |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-015, BR-WS-DATA-001-IO-DOC-DOC-CONVERT-015                                  | IO    | DOC-CONVERT | src-tauri/src/doc/image_media.rs | 568 |

#### TERM-WS-003

| codes                                                                                                                                                                                                                                                | type   | chain                          | 文件                               | 行号  |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------ | -------------------------------- | --- |
| BR-AG-DATA-003-EFFECT-AG-AG-TURN-037                                                                                                                                                                                                                 | EFFECT | AG-TURN, WS-OPERATION          | src-tauri/src/ag/task_runtime.rs | 16  |
| BR-AG-STATE-002-API-AG-WS-OPERATION-005, BR-AG-DATA-003-API-AG-WS-OPERATION-005, BR-SYS-STATE-008-API-AG-WS-OPERATION-005, BR-WS-STATE-003-API-AG-WS-OPERATION-005, BR-WS-PERSIST-003-API-AG-WS-OPERATION-005, BR-WS-OBS-002-API-AG-WS-OPERATION-005 | API    | AG-TURN, AG-TOOL, WS-OPERATION | src-tauri/src/ag/task_runtime.rs | 173 |

#### TERM-WS-004

| codes                                                                                                                                                                         | type   | chain        | 文件                                   | 行号   |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------ | ------------------------------------ | ---- |
| BR-WS-STATE-003-RB-WS-WS-OPERATION-001, BR-WS-PERSIST-003-RB-WS-WS-OPERATION-001                                                                                              | RB     | WS-OPERATION | src-tauri/src/workspace/operation.rs | 139  |
| BR-WS-STATE-003-API-WS-WS-OPERATION-001, BR-WS-DATA-003-API-WS-WS-OPERATION-001, BR-WS-OBS-002-API-WS-WS-OPERATION-001                                                        | API    | WS-OPERATION | src-tauri/src/workspace/operation.rs | 1955 |
| BR-WS-STATE-003-EFFECT-WS-WS-OPERATION-003, BR-WS-DATA-003-EFFECT-WS-WS-OPERATION-003, BR-WS-OBS-002-EFFECT-WS-WS-OPERATION-003, BR-WS-PERSIST-003-EFFECT-WS-WS-OPERATION-003 | EFFECT | WS-OPERATION | src-tauri/src/workspace/operation.rs | 2537 |
| BR-WS-STATE-003-API-WS-WS-OPERATION-002, BR-WS-DATA-003-API-WS-WS-OPERATION-002, BR-WS-OBS-002-API-WS-WS-OPERATION-002                                                        | API    | WS-OPERATION | src-tauri/src/workspace/routes.rs    | 1157 |

#### TERM-WS-006

| codes                                                                                                                                                                         | type   | chain              | 文件                                   | 行号   |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------------ | ------------------------------------ | ---- |
| BR-WS-DATA-003-EFFECT-WS-WS-TIMELINE-003, BR-WS-PERSIST-003-EFFECT-WS-WS-TIMELINE-003                                                                                         | EFFECT | WS-TIMELINE        | src-tauri/src/workspace/operation.rs | 342  |
| BR-WS-STATE-009-EFFECT-WS-WS-FS-009                                                                                                                                           | EFFECT | WS-FS, WS-TIMELINE | src-tauri/src/workspace/operation.rs | 403  |
| BR-WS-STATE-009-EFFECT-WS-WS-FS-010                                                                                                                                           | EFFECT | WS-FS, WS-TIMELINE | src-tauri/src/workspace/operation.rs | 463  |
| BR-WS-STATE-003-EFFECT-WS-WS-OPERATION-003, BR-WS-DATA-003-EFFECT-WS-WS-OPERATION-003, BR-WS-OBS-002-EFFECT-WS-WS-OPERATION-003, BR-WS-PERSIST-003-EFFECT-WS-WS-OPERATION-003 | EFFECT | WS-OPERATION       | src-tauri/src/workspace/operation.rs | 2537 |

#### TERM-WS-007

| codes                                                                                                                  | type | chain                          | 文件                                | 行号   |
| ---------------------------------------------------------------------------------------------------------------------- | ---- | ------------------------------ | --------------------------------- | ---- |
| BR-WS-STATE-003-API-WS-WS-OPERATION-004, BR-WS-DATA-003-API-WS-WS-OPERATION-004, BR-WS-OBS-002-API-WS-WS-OPERATION-004 | API  | WS-OPERATION, DE-DIFF, ED-EDIT | src-tauri/src/workspace/routes.rs | 1413 |

### 测试能力索引

动态发现：前端测试文件 26；Rust 内联测试文件 79；Rust 集成测试文件 2；测试用例约 962。

| module | risk | test assets | test cases | layers | frontend baseline → target |
|---|---|---:|---:|---|---:|
| CORE | high | 6 | 104 | governance:1, service:2, unit:3 | 1/1 → 1 |
| SYS | critical | 35 | 342 | component:2, integration:2, service:5, state:1, unit:25 | 3/3 → 3 |
| WS | critical | 21 | 271 | component:1, integration:1, service:4, unit:15 | 2/2 → 2 |
| ED | critical | 22 | 410 | component:4, service:6, state:1, unit:11 | 11/11 → 11 |
| DE | critical | 18 | 225 | component:3, integration:1, service:2, unit:12 | 2/2 → 2 |
| AG | critical | 34 | 293 | component:1, integration:1, service:1, state:1, unit:30 | 3/3 → 3 |
| CBT | critical | 4 | 12 | unit:4 | 0/0 → 0 |
| DOC | critical | 16 | 96 | service:1, unit:15 | 1/1 → 1 |
| PRV | high | 4 | 24 | component:1, service:1, unit:2 | 2/2 → 2 |
| REF | high | 9 | 138 | component:1, service:2, unit:6 | 1/1 → 1 |
| AST | high | 4 | 10 | unit:4 | 0/0 → 0 |
| KB | high | 2 | 5 | unit:2 | 0/0 → 0 |
| TPL | high | 2 | 6 | unit:2 | 0/0 → 0 |
| ACCT | critical | 2 | 5 | unit:2 | 0/0 → 0 |
| ENT | critical | 3 | 9 | unit:3 | 0/0 → 0 |

系统验证登记（自动化证据与人工验证证据分开）：

| validation | phase | modules | automation | status |
|---|---|---|---|---|
| VAL-CORE-EDIT-DIFF-SAVE | G5 | ED, DE, AG, WS, SYS | partial | active |
| VAL-CORE-CLOSE-RECOVERY | G5 | SYS, ED, DE, WS | partial | deferred |

### 产品能力索引

> 来源：`.governance/product-capability-registry.json`；由 `product:generate` 和 `product:audit` 派生。
> 注册表追溯审计通过。

| 变体 | 能力 | 用户旅程 | 格式 | 正式主链 | 自动化 | 系统验证 | 当前结论 |
|---|---|---|---|---|---|---|---|
| VAR-EDIT-MARKDOWN-SAFE-SAVE | 编辑 Markdown 并安全保存 | 完成一次文档协作 | markdown | ED-EDIT | passed | pending | 自动化已通过，待系统验证 |
| VAR-EDIT-HTML-ROUNDTRIP | 编辑 HTML 正文并保持块锚点 | 完成一次文档协作 | html | ED-EDIT | partial | pending | 自动化部分覆盖 |
| VAR-EDIT-TEXT-LINE-BLOCKS | 编辑纯文本并按行定位内容块 | 完成一次文档协作 | text | ED-EDIT | partial | pending | 自动化部分覆盖 |
| VAR-EDIT-BDOCX-SUPPORTED-SUBSET | 编辑 BDocx 支持子集并保存重开 | 完成一次文档协作 | bdocx | DOC-CONVERT | passed | pending | 自动化已通过，待系统验证 |
| VAR-OPEN-NATIVE-DOCX-PREVIEW | 打开原生 DOCX 并创建可编辑草稿 | 完成一次文档协作 | docx | — | partial | pending | 自动化部分覆盖 |
| VAR-AGENT-TEXTUAL-DIFF | 代理修改文本、Markdown 或 HTML 内容块 | 完成一次文档协作 | text / markdown / html | AG-TURN | partial | blocked | 自动化部分覆盖 |
| VAR-AGENT-BDOCX-STRUCTURED-EDIT | 代理以结构化操作修改 BDocx | 完成一次文档协作 | bdocx | AG-TURN | passed | blocked | 自动化已通过，待系统验证 |
| VAR-WS-CREATE-RESOURCE | 新建文件或文件夹 | 完成一次文档协作 | markdown / text / html / bdocx / folder | WS-FS | partial | pending | 自动化部分覆盖 |
| VAR-WS-DELETE-FOLDER | 文件夹删除两级语义 | 完成一次文档协作 | folder | WS-FS | partial | pending | 自动化部分覆盖 |
| VAR-WS-SAVE-TIMELINE | 保存成功记时间轴节点 | 完成一次文档协作 | markdown / text / html / bdocx | WS-TIMELINE | partial | pending | 自动化部分覆盖 |
| VAR-AG-MODEL-CONFIG | 模型配置体系（来源分层+能力偏好+预设清单+BYOK管理） | 完成一次文档协作 | markdown / text / html / bdocx | — | partial | pending | 自动化部分覆盖 |
| VAR-PRV-CACHE-LOCAL-OPEN | 预览缓存落盘与本地打开 | 完成一次文档协作 | docx / xlsx / pptx / pdf | PRV-VIEW | partial | pending | 自动化部分覆盖 |
| VAR-PRV-TOOLBAR-ADAPTIVE | 预览模式工具栏自适应 | 完成一次文档协作 | docx / xlsx / pptx | — | partial | pending | 自动化部分覆盖 |
| VAR-CTX-MENU-UNIFIED | 统一右键菜单 | 完成一次文档协作 | markdown / text / html / bdocx / folder | — | partial | pending | 自动化部分覆盖 |
| VAR-ED-SELECTION-PERSIST | 选区失焦持久化 | 完成一次文档协作 | markdown / text / html / bdocx | — | partial | pending | 自动化部分覆盖 |
| VAR-ED-SELECTION-DRAG | 选区拖动不错位 | 完成一次文档协作 | markdown / text / html / bdocx | — | partial | pending | 自动化部分覆盖 |
| VAR-EDIT-TOOLBAR-CONTEXTUAL | 工具栏上下文分组与编辑操作富化 | 完成一次文档协作 | markdown / text / html / bdocx | — | partial | pending | 自动化部分覆盖 |
| VAR-EDIT-FONT-LOCAL-LIBRARY | 本地字体库同步 | 完成一次文档协作 | markdown / html / bdocx | — | partial | pending | 自动化部分覆盖 |
| VAR-EDIT-HIGHLIGHT-BACKGROUND | 高亮收敛为文字背景 | 完成一次文档协作 | markdown / html / bdocx | — | partial | pending | 自动化部分覆盖 |

### triage 汇总

（暂无有效 triage 记录）

### 标注延期汇总

| 符号                | 文件                               | 承接计划                                | 复核日期       | 已解决 |
| ----------------- | -------------------------------- | ----------------------------------- | ---------- | --- |
| App               | src/shell/App.tsx                | 已关闭（ANNOTATION-DEFERRAL-CLOSURE-01） | 2026-08-01 | 是   |
| Welcome           | src/shell/Welcome.tsx            | 已关闭（ANNOTATION-DEFERRAL-CLOSURE-01） | 2026-08-01 | 是   |
| Workbench         | src/shell/Workbench.tsx          | 已关闭（ANNOTATION-DEFERRAL-CLOSURE-01） | 2026-08-01 | 是   |
| useWorkspaceShell | src/store/workspaceShell.ts      | 已关闭（ANNOTATION-DEFERRAL-CLOSURE-01） | 2026-08-01 | 是   |
| openWorkspace     | src/services/workspaceService.ts | 已关闭（ANNOTATION-DEFERRAL-CLOSURE-01） | 2026-08-01 | 是   |
