# Issue Trace — WS 前端测试资产治理整合

## 1. 用户可见症状

测试治理审计持续报告 `TEST_FILE_CONSOLIDATION_DEBT: WS 6 → 2`。现有测试分别覆盖文件树组件与构造、
Workspace 状态投影、工作区恢复和 IPC 服务契约，功能覆盖稳定，但物理入口多于登记目标。

## 2. 入口点

- `TEST-GOVERNANCE-01` 冻结 WS 的 6 文件 baseline 与 2 文件 target。
- ED、SYS 测试整合已证明实体合并、动态 inventory 与影响选择链路可闭环。
- 人类回复“继续下一步”，确认启动 WS 整合。

## 3. 相关规则

- ADU §5.6：baseline 禁止继续增长；target 是分期整合目标，不得牺牲诊断性和断言。
- APC：WS 为 critical 模块，相关变更扩展 ED、DE、AG、SYS 并保守运行 Rust。
- AGENTS §2：不得删除、注释或弱化测试制造通过。

## 4. 相关链路

`OldWsTests → LogicalSuiteMap → TargetSuites → DirectedRun → FullRegression → DynamicInventory → ADUS`。

## 5. 生产者块

- 6 个 WS owner 前端测试文件及其 import、mock、hook、describe、it 和 covers。
- `.governance/test-governance.json` 的 WS test_globs、baseline 与 target。

## 6. 消费者块

- Vitest 直接发现 2 个目标套件。
- `test_governance.mjs` 动态计算 owner、layer、test_count 和 covers。
- WS critical 影响计划扩展 ED、DE、AG、SYS，触发前端与 Rust 回归。

## 7. 路径上的空规则块

无新的业务规则缺口。本 Scope 只改变测试组织，不改变 RULE、CHAIN、STATE_MACHINE 或生产行为。

## 8. boundary 假设

- WorkspaceUI 保留文件树交互、树构造与两个 UI store 的可观察状态。
- workspaceRuntime 保留恢复流程和 IPC 信封契约；统一 invoke mock 不改变各自断言。
- 文件名不是业务真值；合并后由 owner、layer、covers 和 describe 名保持定位能力。

## 9. 运行时假设

- WorkspaceUI 使用 jsdom；workspaceRuntime 沿用默认 node 环境。
- Vitest hoist `vi.mock`；两个服务旧文件必须共用 workspaceService 的 hoisted Tauri mock。
- top-level hook 合并后可能扩大作用域；失败优先视为隔离或 mock 合并问题。

## 10. 疑似不变量破坏

- 两次 mock `@tauri-apps/api/core` 相互覆盖，导致恢复或 IPC 契约测试失真。
- jsdom 与 store hook 合并后产生跨用例污染。
- relative import 因目标路径变化而漂移。
- 只减少文件计数却把旧用例藏入非 test helper，形成虚假治理。

## 11. 旧文件到目标套件映射

| 目标套件 | layer | 旧文件 |
|---|---|---|
| `components/ws/WorkspaceUI.test.tsx` | component | buildTree / FileTreeNode / workspaceShell / wsStore |
| `services/workspaceRuntime.test.ts` | service | workspaceService / wsService |

## 12. 验证计划

1. 合并前冻结 6 文件的 28 个 owner 用例与 covers 集合，并运行定向基线。
2. 每个目标套件独立运行，修复 import、mock、hook 隔离，不改断言。
3. 动态 inventory 对比 WS frontend_owned、用例数与 covers。
4. 执行 test:plan、全量前端、TypeScript、Rust、构建与治理审计。
5. 连续生成 ADUS 验证幂等。

## Scope Lock

- scope_id：`TEST-CONSOLIDATION-WS-01`
- task_type：`CODE_CHANGE`
- governance_phase：`IMPLEMENTATION`
- confirmation：`YES`（人类于 2026-08-01 回复“继续下一步”）。
- allowed_files / forbidden_files：以 `.governance/active_scope.json` 为准。
- 当前状态：`READY`。

## 13. Task Runtime Report

task_type: CODE_CHANGE
governance_phase: IMPLEMENTATION
final_state: READY
issue_trace: TEST-CONSOLIDATION-WS-01

scope:
  mode: explicit
  status: confirmed

phase_gate:
  status: passed
  evidence: TEST-GOVERNANCE-01 已登记 WS 6→2 债务，人类已确认继续下一步。

mapping_chain:
  status: complete
  evidence: 6 个旧文件已映射到 2 个 WorkspaceUI/workspaceRuntime 套件；不新增业务规则。

validation:
  - 旧六文件定向基线：6/6 文件、28/28 用例通过；covers 集合为 BR-ED-STATE-001、BR-WS-OBS-001。
  - 两个 WS 目标套件联合运行：2/2 文件、28/28 用例通过；WorkspaceUI=21、workspaceRuntime=7。
  - test:governance：8/8 自测通过；动态发现 92 个资产、724 个用例；WS frontend_owned/baseline/target 为 2/2/2，owner 用例为 28。
  - test:plan：直接模块 CORE、ED、WS；受影响模块扩展为 AG、CORE、DE、ED、SYS、WS，并触发前端全量、TypeScript、Rust 全量和治理审计。
  - npm test：29/29 文件、455/455 用例通过；npx tsc --noEmit 通过。
  - cargo test（沙箱外真实 LibreOffice 运行条件）：261/261 库测试与 8/8 集成测试通过。
  - npm run build：通过；Bundled DOC tools 校验通过，Vite 生产构建完成。
  - governance:generate：生成 92 个测试资产、724 个用例；第二次执行内容无变化，幂等通过。
  - governance:audit：passed；0 unmapped executable、0 unowned test、0 focused/skipped。
  - git diff --check：通过。

stop_conditions:
  - 无。

recovery:
  - 无。

unresolved_items:
  - 后续测试文件整合债务：DE 4→2、AG 8→3、PRV 2→1；均由测试治理审计持续报告，非本 Scope 阻断项。
  - 模块 test cases 是文件级关联计数；workspaceRuntime 同时覆盖 WS/ED，使 ED 关联计数变为 360，但 ED owner 用例仍由精确契约固定为 207。
  - Vite 仍报告既有单 chunk 超过 500 kB；Rust 仍有既有 dead_code 警告，均未由本次纯测试组织变更引入。
