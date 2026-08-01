# Issue Trace — SYS 前端测试资产治理整合

## 1. 用户可见症状

测试治理审计持续报告 `TEST_FILE_CONSOLIDATION_DEBT: SYS 8 → 3`。现有测试分别围绕 Shell 组件、
关闭编排、状态机监听和三个 store 生长，功能覆盖稳定，但物理入口仍多于已登记目标。

## 2. 入口点

- `TEST-GOVERNANCE-01` 冻结 SYS 的 8 文件 baseline 与 3 文件 target。
- `TEST-CONSOLIDATION-ED-01` 已证明实体合并、动态 inventory 与影响选择链路可闭环。
- 人类回复“继续下一步”，确认启动 SYS 整合。

## 3. 相关规则

- ADU §5.6：baseline 禁止继续增长；target 是分期整合目标，不得牺牲诊断性和断言。
- APC：SYS 为 critical 模块，相关变更需要保守触发前端与 Rust 回归。
- AGENTS §2：不得删除、注释或弱化测试制造通过。

## 4. 相关链路

`OldSysTests → LayerSuiteMap → TargetSuites → DirectedRun → FullRegression → DynamicInventory → ADUS`。

## 5. 生产者块

- 8 个 SYS owner 前端测试文件及其 import、mock、hook、describe、it 和 covers。
- `.governance/test-governance.json` 的 SYS test_globs、baseline 与 target。

## 6. 消费者块

- Vitest 直接发现 3 个目标套件。
- `test_governance.mjs` 动态计算 owner、layer、test_count 和 covers。
- SYS critical 影响计划触发前端全量、TypeScript、Rust 全量和治理审计。

## 7. 路径上的空规则块

无新的业务规则缺口。本 Scope 只改变测试组织，不改变 RULE、CHAIN、STATE_MACHINE 或生产行为。

## 8. boundary 假设

- component 套件验证用户事件到 Shell 可观察状态；service 套件验证关闭/监听编排；state 套件验证 store 不变量。
- App 必须继续调用真实 `requestClose`；弹窗测试只替换 `cancelClose`、`submitDecision`、`forceQuit`。
- 文件名不是业务真值；合并后由 owner、layer、covers 和 describe 名保持定位能力。

## 9. 运行时假设

- component 套件使用 jsdom；service 与 state 沿用默认 node 环境。
- Vitest hoist `vi.mock`；组件合并对 closeService 使用部分 mock，保留真实 requestClose。
- top-level hook 合并后可能扩大作用域；失败优先视为隔离或 mock 合并问题。

## 10. 疑似不变量破坏

- 全量 mock closeService 导致 App 关闭编排不再覆盖真实 service。
- mock/hook 扩大作用域造成不同旧文件之间的状态污染。
- relative import 因目标路径变化而漂移。
- 只减少文件计数却把旧用例藏入非 test helper，形成虚假治理。

## 11. 旧文件到目标套件映射

| 目标套件 | layer | 旧文件 |
|---|---|---|
| `components/shell/ShellComponents.test.tsx` | component | App / CloseConfirmDialog / TabStrip |
| `services/systemRuntime.test.ts` | service | closeService / smListeners |
| `store/systemState.test.ts` | state | closeStore / layoutStore / tabsStore |

## 12. 验证计划

1. 合并前冻结 8 文件的 52 个 owner 用例与 covers 集合，并运行定向基线。
2. 每个目标套件独立运行，修复 import、mock、hook 隔离，不改断言。
3. 动态 inventory 对比 SYS frontend_owned、用例数与 covers。
4. 执行 test:plan、全量前端、TypeScript、Rust、构建与治理审计。
5. 连续生成 ADUS 验证幂等。

Scope 运行时登记同步：系统验证记录原先引用三个旧 SYS 测试路径；物理合并后必须改指
`ShellComponents.test.tsx` 与 `systemRuntime.test.ts`，否则治理审计按证据缺失失败关闭。

## Scope Lock

- scope_id：`TEST-CONSOLIDATION-SYS-01`
- task_type：`CODE_CHANGE`
- governance_phase：`IMPLEMENTATION`
- confirmation：`YES`（人类于 2026-08-01 回复“继续下一步”）。
- allowed_files / forbidden_files：以 `.governance/active_scope.json` 为准。
- 当前状态：`READY`。

## 13. Task Runtime Report

task_type: CODE_CHANGE
governance_phase: IMPLEMENTATION
final_state: READY
issue_trace: TEST-CONSOLIDATION-SYS-01

scope:
  mode: explicit
  status: confirmed

phase_gate:
  status: passed
  evidence: TEST-GOVERNANCE-01 已登记 SYS 8→3 债务，人类已确认继续下一步。

mapping_chain:
  status: complete
  evidence: 8 个旧文件已映射到 3 个 component/service/state 套件；不新增业务规则。

validation:
  - 旧八文件定向基线：8/8 文件、52/52 用例通过；covers 集合为 BR-SYS-STATE-004、BR-SYS-STATE-006、BR-ED-STATE-001。
  - 三个 SYS 目标套件联合运行：3/3 文件、52/52 用例通过；component=13、service=15、state=24。
  - test:governance：8/8 自测通过；动态发现 96 个资产、724 个用例；SYS frontend_owned/baseline/target 为 3/3/3，owner 用例为 52。
  - test:plan：直接模块 CORE、ED、SYS；SYS critical 影响扩展至全部登记模块，并触发前端全量、TypeScript、Rust 全量和治理审计。
  - npm test：33/33 文件、455/455 用例通过；npx tsc --noEmit 通过。
  - cargo test（沙箱外真实 LibreOffice 运行条件）：261/261 库测试与 8/8 集成测试通过。
  - npm run build：通过；Bundled DOC tools 校验通过，Vite 生产构建完成。
  - governance:generate：生成 96 个测试资产、724 个用例；第二次执行内容无变化，幂等通过。
  - governance:audit：passed；0 unmapped executable、0 unowned test、0 focused/skipped，系统验证证据路径有效。
  - git diff --check：通过。

stop_conditions:
  - 无。

recovery:
  - 首次 test:governance 因系统验证记录仍引用三个旧测试路径而失败关闭；更新为 ShellComponents/systemRuntime 后通过，未放宽审计。

unresolved_items:
  - 后续测试文件整合债务：WS 6→2、DE 4→2、AG 8→3、PRV 2→1；均由测试治理审计持续报告，非本 Scope 阻断项。
  - 模块 test cases 是文件级关联计数；ShellComponents 同时覆盖 SYS/ED，使 ED 关联计数变为 355，但 ED owner 用例仍由精确契约固定为 207。
  - Vite 仍报告既有单 chunk 超过 500 kB；Rust 仍有既有 dead_code 警告，均未由本次纯测试组织变更引入。
