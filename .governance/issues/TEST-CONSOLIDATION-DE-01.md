# Issue Trace — DE 前端测试资产治理整合

## 1. 用户可见症状

测试治理审计持续报告 `TEST_FILE_CONSOLIDATION_DEBT: DE 4 → 2`。现有测试覆盖 Diff Review 组件、
裁决编排和状态投影，功能覆盖稳定，但物理入口多于登记目标。

## 2. 入口点

- `TEST-GOVERNANCE-01` 冻结 DE 的 4 文件 baseline 与 2 文件 target。
- ED、SYS、WS 测试整合已证明实体合并、动态 inventory 与影响选择链路可闭环。
- 人类回复“继续下一步”，确认启动 DE 整合。

## 3. 相关规则

- ADU §5.6：baseline 禁止继续增长；target 是分期整合目标，不得牺牲诊断性和断言。
- APC：DE 为 critical 模块，相关变更扩展 AG、SYS 并保守运行 Rust。
- AGENTS §2：不得删除、注释或弱化测试制造通过。

## 4. 相关链路

`OldDeTests → LogicalSuiteMap → TargetSuites → DirectedRun → FullRegression → DynamicInventory → ADUS`。

## 5. 生产者块

- 4 个 DE owner 前端测试文件及其 import、mock、hook、describe、it 和 covers。
- `.governance/test-governance.json` 的 DE test_globs、baseline 与 target。

## 6. 消费者块

- Vitest 直接发现 2 个目标套件。
- `test_governance.mjs` 动态计算 owner、layer、test_count 和 covers。
- DE critical 影响计划扩展 AG、SYS，触发前端与 Rust 回归。

## 7. 路径上的空规则块

无新的业务规则缺口。本 Scope 只改变测试组织，不改变 RULE、CHAIN、STATE_MACHINE 或生产行为。

## 8. boundary 假设

- DiffReview 保留批量操作栏和 DiffCard 的用户可观察状态、结构摘要与交互。
- deRuntime 保留裁决副作用、草稿恢复、冲突处理和 deStore 不变量。
- 文件名不是业务真值；合并后由 owner、layer、covers 和 describe 名保持定位能力。

## 9. 运行时假设

- DiffReview 使用 jsdom；deRuntime 沿用默认 node 环境。
- Vitest hoist `vi.mock`；两个组件旧文件共用同一个 Tauri core mock。
- top-level hook 合并后可能扩大作用域；失败优先视为隔离或 mock 合并问题。

## 10. 疑似不变量破坏

- 重复 mock `@tauri-apps/api/core` 相互覆盖或 revealBlock mock 失效。
- deService 与 deStore 的 beforeEach 合并后产生状态污染。
- relative import 因目标路径变化而漂移。
- 只减少文件计数却把旧用例藏入非 test helper，形成虚假治理。

## 11. 旧文件到目标套件映射

| 目标套件 | layer | 旧文件 |
|---|---|---|
| `components/de/DiffReview.test.tsx` | component | DiffAllActionsBar / DiffCard |
| `services/deRuntime.test.ts` | service | deService / deStore |

## 12. 验证计划

1. 合并前冻结 4 文件的 58 个 owner 用例与 covers 集合，并运行定向基线。
2. 每个目标套件独立运行，修复 import、mock、hook 隔离，不改断言。
3. 动态 inventory 对比 DE frontend_owned、用例数与 covers。
4. 执行 test:plan、全量前端、TypeScript、Rust、构建与治理审计。
5. 连续生成 ADUS 验证幂等。

## Scope Lock

- scope_id：`TEST-CONSOLIDATION-DE-01`
- task_type：`CODE_CHANGE`
- governance_phase：`IMPLEMENTATION`
- confirmation：`YES`（人类于 2026-08-01 回复“继续下一步”）。
- allowed_files / forbidden_files：以 `.governance/active_scope.json` 为准。
- 当前状态：`READY`。

## 13. Task Runtime Report

task_type: CODE_CHANGE
governance_phase: IMPLEMENTATION
final_state: READY
issue_trace: TEST-CONSOLIDATION-DE-01

scope:
  mode: explicit
  status: confirmed

phase_gate:
  status: passed
  evidence: TEST-GOVERNANCE-01 已登记 DE 4→2 债务，人类已确认继续下一步。

mapping_chain:
  status: complete
  evidence: 4 个旧文件已映射到 2 个 DiffReview/deRuntime 套件；不新增业务规则。

validation:
  - 旧四文件定向基线：4/4 文件、58/58 用例通过；既有 DE/ED/WS/SYS covers 集合已冻结。
  - 两个 DE 目标套件联合运行：2/2 文件、58/58 用例通过；DiffReview=27、deRuntime=31。
  - test:governance：8/8 自测通过；动态发现 90 个资产、724 个用例；DE frontend_owned/baseline/target 为 2/2/2，owner 用例为 58。
  - test:plan：直接模块 CORE、DE、ED、SYS、WS；受影响范围保守扩展至全部登记模块，并触发前端全量、TypeScript、Rust 全量和治理审计。
  - npm test：27/27 文件、455/455 用例通过；npx tsc --noEmit 通过。
  - cargo test（沙箱外真实 LibreOffice 运行条件）：261/261 库测试与 8/8 集成测试通过。
  - npm run build：通过；Bundled DOC tools 校验通过，Vite 生产构建完成。
  - governance:generate：生成 90 个测试资产、724 个用例；第二次执行内容无变化，幂等通过。
  - governance:audit：passed；0 unmapped executable、0 unowned test、0 focused/skipped。
  - git diff --check：通过。

stop_conditions:
  - 无。

recovery:
  - 无。

unresolved_items:
  - 后续测试文件整合债务：AG 8→3、PRV 2→1；均由测试治理审计持续报告，非本 Scope 阻断项。
  - 模块 test cases 是文件级关联计数；两个 DE 套件同时覆盖 ED/WS/SYS，使关联计数随物理合并变化，但 DE owner=58、ED owner=207 等精确契约保持不变。
  - Vite 仍报告既有单 chunk 超过 500 kB；Rust 仍有既有 dead_code 警告，均未由本次纯测试组织变更引入。
