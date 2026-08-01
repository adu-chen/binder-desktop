# Issue Trace — ED 前端测试资产治理整合

## 1. 用户可见症状

ED 现有 27 个前端测试文件、约 4,174 行，文件边界多沿实现文件自然增长，已经超过测试治理注册表
目标 7 个。AI 能动态选中这些测试，但诊断入口分散、全模块加载和维护成本偏高。

## 2. 入口点

- `TEST-GOVERNANCE-01` 报告 `TEST_FILE_CONSOLIDATION_DEBT: ED 27 → 7`。
- 人类确认下一步启动 `TEST-CONSOLIDATION-ED-01`。
- ED owner 测试资产进入物理合并流程。

## 3. 相关规则

- ADU §5.6：文件 baseline 冻结增长，target 分期整合；不得牺牲诊断性和断言。
- APC §10：ED 是 critical 模块；变更扩展一跳消费者并保守运行 Rust。
- AGENTS §2：不得删除、注释或弱化测试制造通过。

## 4. 相关链路

`OldTestFiles → LogicalSuiteMap → TargetSuites → DirectedRun → FullRegression → DynamicInventory → ADUS`。

## 5. 生产者块

- 27 个 ED owner 测试文件及其 import、mock、hook、describe、it 和 covers。
- `.governance/test-governance.json` 的 ED source/test globs、baseline 与 target。

## 6. 消费者块

- Vitest 直接发现 7 个目标套件。
- `test_governance.mjs` 动态计算 owner/layer/test_count/covers。
- ED 变更影响计划扩展 DE、AG、SYS，并触发相关前端、类型、Rust 与审计。

## 7. 路径上的空规则块

无新的业务规则缺口。本 Scope 只改变测试组织，不改变 RULE/CHAIN/STATE_MACHINE 或生产行为。

## 8. boundary 假设

- 测试行为边界由用例、断言、mock、hook 和环境共同构成，不能只比较用例数量。
- 文件名不是业务真值；合并后由 covers、owner、layer 和 describe 名保持定位能力。
- 同一目标文件中的 top-level hook/mock 会互相作用，因此必须分批定向验证。

## 9. 运行时假设

- component 套件使用 jsdom；其余沿用默认 node 环境。
- Vitest hoist `vi.mock`；合并同一模块 mock 时必须提供两个旧文件所需导出与返回字段。
- 业务模块不变，任何断言失败优先视为测试隔离/合并错误。

## 10. 疑似不变量破坏

- mock 合并覆盖真实导出，导致其他用例读取 undefined。
- 多个文件的 `beforeEach` / `afterEach` 合并后扩大作用域。
- 同名 schema/helper/const 发生模块级冲突。
- relative import 因目标文件目录变化而漂移。
- 只减少文件计数却把旧用例藏入非 test helper，形成虚假治理。

## 11. 旧文件到目标套件映射

| 目标套件 | layer | 旧文件 |
|---|---|---|
| `EditorCore.test.tsx` | component | Editor.imageDrop / Editor.imageOps / Editor.lifecycle |
| `EditorControls.test.tsx` | component | EditorStatusBar / EditorToolbar / ImageMoveControls / TableMoveControls |
| `extensions/EditorExtensions.test.ts` | unit | BlockId / DiffDecoration / FontSize / ImageId / ResizableImage / TableCellShading / markdownCodec |
| `extensions/PaginationSuite.test.ts` | unit | Pagination / paginationFlow / paginationViewportAnchor / zoom |
| `services/edOperations.test.ts` | service | edBlockOps / edImageOps / edStyleOps / edTableOps |
| `services/edRuntime.test.ts` | service | edLocate / edService |
| `store/edState.test.ts` | state | edStore / edViewPrefsStore / outlineStore |

## 12. 验证计划

1. 合并前保存 27 文件的用例数、covers 集合和全量通过基线。
2. 每生成一个目标套件即定向运行；修复 import/mock/hook 隔离，不改断言。
3. 动态 inventory 对比 ED frontend_owned、用例数和 covers。
4. 运行本 Scope 的 `test:plan/test:related`、全量前端、Rust、构建与治理审计。
5. 连续生成 ADUS 验证幂等。

Scope 运行时扩展：测试治理自测原先固定引用 ED 旧文件名和 58 文件基线；合并后必须更新为七套件路径、
38 文件基线以及 ED `frontend_owned=7 / owner tests=207` 契约，因此将
`scripts/test_governance.test.mjs` 纳入 allowed_files。选择器实现不变。

## Scope Lock

- scope_id：`TEST-CONSOLIDATION-ED-01`
- task_type：`CODE_CHANGE`
- governance_phase：`IMPLEMENTATION`
- confirmation：`YES`（人类于 2026-08-01 回复“启动”）。
- allowed_files / forbidden_files：以 `.governance/active_scope.json` 为准。
- 当前状态：`READY`。

## 13. Task Runtime Report

task_type: CODE_CHANGE
governance_phase: IMPLEMENTATION
final_state: READY
issue_trace: TEST-CONSOLIDATION-ED-01

scope:
  mode: explicit
  status: confirmed

phase_gate:
  status: passed
  evidence: TEST-GOVERNANCE-01 已建立动态 inventory、文件预算、影响选择和验证硬门；本 Scope 承接 ED 债务。

mapping_chain:
  status: complete
  evidence: 27 个旧文件已映射到 7 个 component/unit/service/state 套件；不新增业务规则。

validation:
  - 七个 ED 目标套件联合运行：7/7 文件、207/207 用例通过。
  - test:governance：8/8 自测通过；动态发现 101 个资产、724 个用例；ED frontend_owned/baseline/target 为 7/7/7，owner 用例为 207。
  - test:plan：直接模块 CORE、DE、ED、WS；受影响模块扩展为 AG、CORE、DE、ED、SYS、WS；触发前端全量、TypeScript、Rust 全量和治理审计。
  - npm test：38/38 文件、455/455 用例通过；npx tsc --noEmit 通过。
  - cargo test（沙箱外真实 LibreOffice 运行条件）：261/261 库测试与 8/8 集成测试通过。
  - npm run build：通过；Bundled DOC tools 校验通过，Vite 生产构建完成。
  - governance:generate：生成 101 个测试资产、724 个用例；第二次执行内容无变化，幂等通过。
  - governance:audit：passed；0 unmapped executable、0 unowned test、0 focused/skipped、77/77 规则有测试覆盖。
  - git diff --check：通过。

stop_conditions:
  - 无。

recovery:
  - test:related 在沙箱内执行到 Rust 时，LibreOffice 子进程以 signal 6 崩溃；同一用例在沙箱外立即通过，随后 Rust 全量在沙箱外通过。未修改或弱化测试。

unresolved_items:
  - 后续测试文件整合债务：SYS 8→3、WS 6→2、DE 4→2、AG 8→3、PRV 2→1；均由测试治理审计持续报告，非本 Scope 阻断项。
  - Vite 仍报告既有单 chunk 超过 500 kB；Rust 仍有既有 dead_code 警告，均未由本次纯测试组织变更引入。
