# Issue Trace · VAL-CORE-EDIT-DIFF-SAVE-AUTOMATION-01

## 1. 用户可见症状

当前没有新增产品缺陷；治理缺口是“接受修改 → 保存 → 关闭重开”只有前端 mock 与后端领域函数的分段证据，
没有一条自动化实际穿过 Tauri Webview IPC、生产路由、真实临时文件并再次读取重开内容。

## 2. 入口点

- 用户入口：编辑器接受 Diff 后执行保存、关闭文档并重新打开。
- 前端入口：`src/services/edService.ts::saveDocument` / `openDocument`，经
  `src/services/wsService.ts::saveDoc` / `readDoc` 发送 `dispatch` 信封。
- 后端入口：`src-tauri/src/lib.rs::dispatch` → `route_accept_draft_diff` / `route_save_doc` /
  `route_read_doc` / `route_reject_diff`。

## 3. 相关规则

`BR-SYS-DATA-003`、`BR-SYS-OBS-002`、`BR-DE-STATE-001`、`BR-ED-STATE-001`、
`BR-WS-PERSIST-001`、`BR-WS-OBS-001`、`X-INTER-001`、`X-SEQ-002`、`X-FORBID-008`。

## 4. 相关链路

接受保存正路：`ED-EDIT → IPC-CALL → DE-DIFF → WS-SAVE → IPC-CALL → WS-FS`。

拒绝正路：`ED-EDIT → IPC-CALL → DE-DIFF → WS-DIFF-APPLY → IPC-CALL → WS-FS`。

## 5. 生产者块

- `BR-SYS-DATA-003-API-SYS-IPC-CALL-002` 位于 `src-tauri/src/lib.rs::dispatch`。
- `BR-DE-STATE-001-RB-DE-DE-DIFF-001` 位于 `src-tauri/src/de/mod.rs::transition`。
- `BR-WS-PERSIST-001-EFFECT-WS-WS-SAVE-001` 位于
  `src-tauri/src/workspace/save.rs::save_atomic`。
- `BR-WS-OBS-001-RB-WS-WS-FS-001` 位于 `src-tauri/src/lib.rs::save_with_baseline_guard`。

## 6. 消费者块

- `BR-ED-STATE-003-EFFECT-ED-ED-EDIT-013` 位于 `src/services/edService.ts::saveDocument`。
- `BR-SYS-DATA-003-API-SYS-IPC-CALL-001` 位于
  `src-tauri/src/ipc/mod.rs::CommandRegistry::validate`。
- `BR-WS-PERSIST-001-RB-WS-WS-SAVE-002` 位于
  `src-tauri/src/workspace/save.rs::transition`。
- `BR-DE-STATE-001-RB-DE-DE-DIFF-002` 位于 `src/store/deStore.ts`，消费后端 Diff 终态投影。

## 7. 路径上的空规则块

本次保存、裁决、IPC 与重开路径未发现 `rules: []` 块；无需新增 triage。

## 8. boundary 假设

相关块没有显式 `lifecycle_assumption` / `document_snapshot` 字段。此次验证保持既有边界：

- 活动 workspace 已由真实 SQLite 控制态指定；目标文件始终位于 workspace 内。
- `beforeSnapshot` 与加载 `baseline` 来自第一次 `ws.read_doc`，不使用测试硬编码绕过冲突守卫。
- 接受草稿只改变 Diff 终态与 `effect_status=awaiting_save`；只有 `ws.save_doc` 可以写 DiskState。
- 拒绝路径不得写文件或制造时间轴副作用。

## 9. 运行时假设

- 生产运行时使用 Wry；自动化使用 Tauri `MockRuntime`，但必须从 Mock Webview 发出真实
  `InvokeRequest` 并命中生产注册表和共享生产路由，不能直接调用领域函数冒充 IPC。
- 保存状态广播使用泛型 `Runtime` 的 `AppHandle`；Wry 与 MockRuntime 只替换传输运行时，不改变状态机、
  baseline 守卫、SQLite 或文件 I/O。
- 临时 workspace 独立创建并清理，不读取或修改用户 workspace。

## 10. 疑似不变量破坏

- 若测试命令复制一套生产路由，会产生“测试绿、生产接线坏”的虚假证据；必须让生产 `dispatch` 与测试
  Webview IPC 复用同一个持久化路由函数。
- 泛型化不得改变 Wry 运行时 command 名、响应信封、状态机转移或文件副作用。
- MockRuntime 不能证明真实 WKWebView DOM、按钮焦点和视觉反馈；这些仍属于 `VALIDATION_ONLY-01`。

## 11. 验证计划

- 新增 Tauri Mock Webview IPC 自动化：接受草稿 → 保存 → 直接检查 DiskState → 第二次读取重开；同时
  断言 Diff `effect_status` 从 `awaiting_save` 翻转为 `saved`。
- 同一 harness 覆盖拒绝 → DiskState 不变 → 重开仍为原文，关闭 `AUTOMATION_GAP-03`。
- 运行 `npm run test:plan -- --files src-tauri/src/lib.rs` 与同输入 `npm run test:related`。
- 在非沙箱 macOS 进程环境运行 `npm run test:full`，保留 LibreOffice 原生工具环境约束。
- 运行 `npm run test:governance`、`npm run governance:generate`、`npm run governance:audit` 和
  `git diff --check`；更新测试治理注册表、ADUS 与 G5 场景卡，不修改现有断言。
- 链路日志序列验证：IPC correlation id 保持；接受路径观察 `SM-DE-01 Accepted`、`SM-WS-01 Saving/Saved`；
  拒绝路径观察 `SM-WS-02 Discarded`、`SM-DE-01 Rejected`。
- triage：未发现新的 `rules: []`，无需新增 triage；未解决的 Agent 供给与真实 WKWebView 观察继续留在验证卡。

## Scope Lock

允许修改：

- `.governance/active_scope.json`：本任务机器可读 Scope 与最终证据。
- `.governance/issues/VAL-CORE-EDIT-DIFF-SAVE-AUTOMATION-01.md`：Issue Trace。
- `src-tauri/src/lib.rs`：共享生产持久化路由、Runtime 泛型化与 Mock Webview IPC 自动化。
- `.governance/test-governance.json`：系统验证证据、缺口状态与治理闭合状态。
- `docs/40_validation/A-VAL-X-V-01_测试治理与验证矩阵.md`：本轮治理闭合结论。
- `docs/40_validation/A-VAL-X-V-02_核心编辑与DiffEdit保存验证.md`：场景证据与缺口裁决。
- `docs/00_core/A-CORE-C-R-01_ADUS.md`：仅由生成脚本刷新。

禁止修改：不在上述列表中的文件；尤其禁止修改现有前端测试断言、`buildbot/**`、ADU/ADP/AGENTS/APC、
业务需求与技术设计真值。

## 执行中发现 · SC-01

`npm run test:plan -- --files src-tauri/src/lib.rs` 实际只输出 `npm test`。根因是 Rust 生产源同时含
内联测试时，inventory 将同一路径登记为测试资产，选择器据此把整份文件归入 `direct_test`，提前跳过
Rust 源变化判定。因此生产 Rust 修改没有触发 `cargo test` 与 `governance:audit`，与 TEST-GOVERNANCE-01
“准确、保守触发”目标冲突。

修复需要扩展 Scope 到 `scripts/test_governance.mjs` 与 `scripts/test_governance.test.mjs`：前者区分前端
独立测试文件和 Rust 内联测试承载的生产源，后者固定 `src-tauri/src/lib.rs` 必须触发 `rust-full` 与
`governance-audit` 的回归断言。按 ADP §3.3，当前状态进入 `BLOCKED / SC-01`，等待人类确认扩展后继续。

人类已在收到上述精确文件和原因后确认扩展；`SC-01` 解除，任务回到 `SCOPED`。

## 实施与闭合结果

- 生产 `dispatch` 与 Tauri Mock Webview 系统测试复用 `route_edit_persistence_command`；只有运行时传输从
  Wry 替换为 `MockRuntime`，注册表、状态机、SQLite、baseline 守卫和文件 I/O 保持生产实现。
- 接受场景从 Mock Webview IPC 完成 read→generate→accept draft→save→真实文件→第二次 read，断言
  `status=accepted`、`effect_status=saved` 与 baseline 更新。
- 拒绝场景从同一边界完成 read→generate→reject，断言真实文件、第二次 read 与 baseline 均不变，且
  没有时间轴节点。
- 执行中发现并修复 Rust 内联测试承载文件误走 TEST_ONLY 的选择器缺陷；新增两条治理回归测试分别固定
  Rust 生产源和 Rust 集成测试的触发计划。
- `TEST-GOVERNANCE-01` 能力建设已登记为 `closed` 并转为持续门禁；`VAL-CORE-EDIT-DIFF-SAVE`
  仍为 `active/partial`，最终 G5 保持 `NOT_READY`。

## Task Runtime Report

```yaml
task_id: VAL-CORE-EDIT-DIFF-SAVE-AUTOMATION-01
task_type: CODE_CHANGE
governance_phase: VALIDATION
final_state: READY
scope:
  allowed_files: 9
  outside_scope_changes: 0
  excluded_external_tree: buildbot/
test_impact:
  direct_modules: [AG, DE, DOC, ED, PRV, REF, SYS, WS]
  affected_modules: [ACCT, AG, AST, CBT, DE, DOC, ED, ENT, KB, PRV, REF, SYS, TPL, WS]
  selection: frontend-full + rust-full + governance-audit
validation:
  targeted_system_harness: PASS (2/2)
  selector_regression: PASS (10/10)
  related_non_sandbox: PASS (frontend 457/457; rust 263+4+4; audit passed)
  full_non_sandbox: PASS (frontend 457/457; rust 263+4+4)
  test_governance: PASS (84 assets; 728 tests; 0 findings; 0 warnings)
  governance_generate: PASS
  governance_audit: PASS
  diff_check: PASS
stop_conditions:
  - SC-01 resolved by human-confirmed Scope expansion
unresolved_items:
  - AUTOMATION_GAP-02: deterministic Agent turn→DE→DiffCard integration
  - VALIDATION_ONLY-01: real Tauri/WKWebView interaction and observation
  - VALIDATION_ONLY-02: real provider/account/subscription/gateway chain
  - TEST-GOVERNANCE-01 closure does not change VAL-CORE-EDIT-DIFF-SAVE final G5=NOT_READY
```
