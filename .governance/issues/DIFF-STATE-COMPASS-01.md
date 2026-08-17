# Issue Trace — DIFF-STATE-COMPASS-01：Diff 罗盘三分流状态语义

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-13

## 1. 用户可见症状

同一张 DiffCard 因目标文档是否已打开而应走不同副作用路径，但现有文档、提示词和测试把“接受”错误地抽象为统一的内容生效或落盘开关，导致新建/已打开/未打开文档的行为说明互相矛盾。

## 2. 入口点

- `src-tauri/src/lib.rs::route_accept_draft_diff`：已有 LogicalState 的裁决路径。
- `src-tauri/src/lib.rs::route_accept_diff`：无 LogicalState 的后台裁决与落盘路径。
- `src/services/deService.ts::acceptDiff` 与编辑器打开会话路径：前端受控预览、卡片状态和打开后的迁移。

## 3. 相关规则

- 新增：BR-CORE-STATE-001、BR-CORE-STATE-002、BR-CORE-STATE-003。
- 既有：BR-DE-STATE-001、BR-ED-STATE-001、BR-WS-STATE-001、X-FORBID-005、X-FORBID-008、X-FORBID-009。

## 4. 相关链路

`AG-TURN → DE-DIFF → ED-EDIT → WS-SAVE`（已有 LogicalState 与打开迁移）；
`AG-TURN → DE-DIFF → WS-DIFF-APPLY`（无 LogicalState 的接受后直接落盘）。

## 5. 生产者块

- ChangeProposal / PendingDiff：`src-tauri/src/lib.rs::route_de_generate_diff`。
- DiffProjection：`src-tauri/src/lib.rs::route_ag_propose_edit`。

## 6. 消费者块

- `route_accept_draft_diff`、`route_accept_diff`、`deService.acceptDiff`、`deStore`、`Editor`、DiffCard。

## 7. 路径上的空规则块

本 Scope 不新增无规则业务块；已有适配/展示块继续沿用其既有 triage 归类。

## 8. boundary 假设

- `LogicalState` 已存在：DE/ED 预览直接修改它；显式接受只推进 DiffCard 生命周期。保存仍有效的 pending 预览时，先裁决为 accepted/awaiting_save，再由 save 写 DiskState；若预览基线已推进，先使旧卡 expired，保存当前 LogicalState 但不得误裁决旧卡。
- `LogicalState` 不存在：DiffCard 只临时承载 DiffPayload；接受触发 WS-DIFF-APPLY 直接写 DiskState。
- pending 期间打开目标文档：ED 从 DiffPayload 建立 LogicalState、显示绿色增量，并切换到第一条路径。
- 新建文档的预览成功仍自动开标签并建立 LogicalState，故属于第一条路径。

## 9. 运行时假设

- 后台 PendingDiff 在目标文档打开前不得提前写 DiskState。
- 打开迁移必须以同一 diff 的基准和 after 内容建立 LogicalState；迁移成功后原卡不再走后台接受。
- 已迁移 LogicalState 的用户编辑或后续预览会推进基准，使旧卡片失效且不可裁决；随后保存必须保存当前内容，而不是把旧卡误裁决为接受。

## 10. 疑似不变量破坏

- X-FORBID-008 旧文本把所有 DiskState 写入都限定为 save，须改为“有 LogicalState 时仅 save；无 LogicalState 的 accepted 后台分支是受控例外”。
- X-FORBID-009 旧文本把候选转正绑定到 userAccept，须改为“预览/打开迁移进入 LogicalState；显式接受只裁决；保存有效 pending 预览也可裁决”。
- 不得让 DisplayState 直接改写 LogicalState 或 DiskState；所有迁移仍经 DE/ED/WS 受控接口。

## 11. 验证计划

- 为三条新规则登记到 DE-DIFF、ED-EDIT、WS-SAVE、WS-DIFF-APPLY，并刷新 ADUS。
- 覆盖：已有 LogicalState 的 preview→accept→save、preview→save（隐式接受）以及预览后编辑→save（旧卡失效）；无 LogicalState 的 pending→accept→DiskState；pending→open→green preview→accept/save；迁移后编辑导致旧卡失效。
- 运行 `test:plan`、`test:related`、`governance:generate`、`governance:audit` 与 `git diff --check`。

## 12. 文件范围

允许修改：Core/AG/DE/ED/WS/SYS 的相关需求、设计、状态机、接口、验证文档；相应运行时路由、前端 Diff/编辑器状态、提示词和回归测试；本 Issue Trace、active Scope、生成的 ADUS。

禁止修改：引用、记忆、知识库、模板、构建、账户/网关、文档转换等与三分流无直接消费者关系的实现。

## 13. Task Runtime Report — Diff 罗盘三分流状态语义同步

task_type: CODE_CHANGE  
governance_phase: IMPLEMENTATION  
final_state: READY（2026-08-14 回写：原 BLOCKED 根因 SC-04 LibreOffice Worker signal=6 经 A-VAL-X-V-02
判定为沙箱假失败 VAL-G5-REG-001 resolved——受控沙箱内 signal 6、非沙箱通过；本 Scope 代码三分流实现与
回归在 DIFF-STATE-COMPASS-01 期间已全部通过，障碍解除）  
issue_trace: DIFF-STATE-COMPASS-01

changed_files:

- Core、AG、DE、ED、WS、SYS 的三分流需求/状态机/接口文档及 ADUS：登记 `BR-CORE-STATE-001~003`，同步保存有效 pending 预览即接受的边界。
- `src/services/edService.ts`、`src/services/deService.ts`、`src/store/deStore.ts`、`src/components/ed/Editor.tsx`、`src/services/agService.ts`、`src-tauri/src/lib.rs`、`src-tauri/src/de/mod.rs`、`src-tauri/src/ag/prompts.rs`：实现三分流、后台打开迁移、保存前基准 guard 与提示词约束。
- `src/services/deRuntime.test.ts`、`src/services/edRuntime.test.ts`、`scripts/test_governance.test.mjs`：补回归，并按真实 inventory 更新 ED=240、DE=69 的测试总数断言。

scope:

- mode: explicit
- status: confirmed

phase_gate:

- status: passed
- evidence: 三条规则已登记，ADUS 生成后可按 DE-DIFF、ED-EDIT、WS-SAVE、WS-DIFF-APPLY 检索。

mapping_chain:

- status: complete
- evidence: `BR-CORE-STATE-001~003` → @GOV 运行时入口 → DE/ED 回归测试的映射已由 audit 验证。

test_impact:

- changed_files: `edService`、`deService`、`deStore`、Editor、AG service、Rust routes/prompts/tests、治理测试。
- plan: passed；选择器要求 `npm test`、`npx tsc --noEmit`、完整 `cargo test`、`governance:audit`。
- executed: `npm test` 26 files / 566 tests passed；`npx tsc --noEmit` passed；聚焦回归 3 files / 90 tests passed；新增后台打开迁移 Rust 集成测试 passed；`governance:generate` passed；`governance:audit` passed；`git diff --check` passed。
- failed: 完整 `cargo test` 319 passed / 1 failed；失败项为 `doc::libreoffice_runtime::tests::docx_to_pdf_end_to_end_when_libreoffice_available`，`DOC_PREVIEW_WORKER_CRASH`（signal=6）。单独复跑同一项仍失败，非本 Scope 的 Diff 路径。

stop_conditions:

- SC-04 VALIDATION_BLOCKED：完整 Rust 回归无法通过，且根因位于本 Scope 明确禁止修改的 DOC-CONVERT / LibreOffice Worker。
- **SC-04 已解除（2026-08-14 回写）**：A-VAL-X-V-02 判定 VAL-G5-REG-001 resolved——LibreOffice signal=6
  为受控沙箱假失败（非沙箱通过），环境阻断解除；本 Scope 代码实现与回归（npm test 26 files/566 tests、
  tsc、聚焦回归、Rust 集成测试、generate、audit、diff --check）在阻断期间已全部通过。

recovery:

- （历史）人类确认扩展 Scope 至 DOC-CONVERT 后，定位并修复 LibreOffice Worker signal=6，再重跑完整 `test:related`；或人类接受该环境阻断并调整本任务验收。
- **已处置（2026-08-14）**：沙箱假失败判定后无需代码修复；完整 `cargo test` 在非沙箱环境回归通过，本 Scope 验收成立。LibreOffice E2E 的沙箱环境脆弱性留痕（UNC-44）。

unresolved_items:

- 真实 G5 provider/Tauri/WKWebView 验收不在本 Scope 内。
- 既有跨会话 pending 数据不迁移；只约束未来运行路径。
