# Issue Trace — AG 前端测试资产治理整合

## 1. 用户可见症状

测试治理审计持续报告 `TEST_FILE_CONSOLIDATION_DEBT: AG 8 → 3`。现有测试覆盖代理界面、会话与回合、
服务编排、状态投影、辅助续写和浮层定位，功能覆盖稳定，但物理入口多于登记目标。

## 2. 入口点

- `TEST-GOVERNANCE-01` 冻结 AG 的 8 文件 baseline 与 3 文件 target。
- ED、SYS、WS、DE 测试整合已证明实体合并、动态 inventory 与影响选择链路可闭环。
- 人类回复“继续下一步”，确认启动 AG 整合。

## 3. 相关规则

- `BR-AG-STATE-003`：辅助续写与局部修改的状态、终态和 latest-wins 行为。
- `BR-AG-PERSIST-001`：代理会话历史恢复与删除持久化。
- `BR-AG-DATA-001`、`BR-AG-DATA-003`：回合上下文、引用载荷与 Diff 投影。
- `BR-REF-DATA-003`、`BR-REF-DATA-004`、`BR-DE-STATE-001`：跨 REF/DE 的引用与回合归属。
- ADU §5.6：baseline 禁止继续增长；target 是分期整合目标，不得牺牲诊断性和断言。
- APC：AG 为 critical 模块，相关变更扩展 SYS 并保守运行 Rust。
- AGENTS §2：不得删除、注释或弱化测试制造通过。

## 4. 相关链路

`OldAgTests → LogicalSuiteMap → TargetSuites → DirectedRun → FullRegression → DynamicInventory → ADUS`。

## 5. 生产者块

- 8 个 AG owner 前端测试文件及其 import、mock、hook、describe、it 和 covers。
- `.governance/test-governance.json` 的 AG test_globs、baseline 与 target。

## 6. 消费者块

- Vitest 直接发现 3 个目标套件。
- `test_governance.mjs` 动态计算 owner、layer、test_count 和 covers。
- AG critical 影响计划扩展 SYS，触发前端与 Rust 回归。

## 7. 路径上的空规则块

无新的业务规则缺口。本 Scope 只改变测试组织，不改变 RULE、CHAIN、STATE_MACHINE 或生产行为。

## 8. boundary 假设

- AgentUI 保留 AgentPanel、Composer、MessageList 和 AutocompleteOverlay 的用户可观察交互。
- agRuntime 保留 agService 编排、持久化、Diff 投影和 agStore 状态不变量。
- assistRuntime 保留 assistStore latest-wins 与浮层定位纯函数边界。
- 文件名不是业务真值；合并后由 owner、layer、covers 和 describe 名保持定位能力。

## 9. 运行时假设

- AgentUI 使用 jsdom；agRuntime 与 assistRuntime 沿用默认 node 环境。
- Vitest hoist `vi.mock`；四个 UI 旧文件必须共用一个覆盖全部既有导出的 agService mock。
- top-level hook 合并后可能扩大作用域；失败优先视为隔离或 mock 合并问题。

## 10. 疑似不变量破坏

- agService mock 合并时遗漏 AgentPanel、Composer 或 MessageList 所需导出。
- agService 与 agStore、assistStore 与 placement 的 beforeEach 合并后产生状态污染。
- relative import 因目标路径变化而漂移。
- node 测试被错误并入 jsdom，改变环境契约。
- 只减少文件计数却把旧用例藏入非 test helper，形成虚假治理。

## 11. 旧文件到目标套件映射

| 目标套件 | layer / environment | 旧文件 |
|---|---|---|
| `components/ag/AgentUI.test.tsx` | component / jsdom | AgentPanel / AutocompleteOverlay / Composer / MessageList |
| `services/agRuntime.test.ts` | service / node | agService / agStore |
| `store/assistRuntime.test.ts` | state / node | assistStore / assistPlacement |

## 12. 验证计划

1. 合并前冻结 8 文件的 73 个 owner 用例与 covers 集合，并运行定向基线。
2. 每个目标套件独立运行，修复 import、mock、hook 隔离，不改断言。
3. 动态 inventory 对比 AG frontend_owned、用例数与 covers。
4. 执行 test:plan、全量前端、TypeScript、Rust、构建与治理审计。
5. 连续生成 ADUS 验证幂等。

## Scope Lock

- scope_id：`TEST-CONSOLIDATION-AG-01`
- task_type：`CODE_CHANGE`
- governance_phase：`IMPLEMENTATION`
- confirmation：`YES`（人类于 2026-08-01 回复“继续下一步”）。
- allowed_files / forbidden_files：以 `.governance/active_scope.json` 为准。
- 当前状态：`READY`。

## 13. Task Runtime Report

task_type: CODE_CHANGE
governance_phase: IMPLEMENTATION
final_state: READY
issue_trace: TEST-CONSOLIDATION-AG-01

scope:
  mode: explicit
  status: confirmed

phase_gate:
  status: passed
  evidence: TEST-GOVERNANCE-01 已登记 AG 8→3 债务，人类已确认继续下一步。

mapping_chain:
  status: complete
  evidence: 8 个旧文件已映射到 3 个 AgentUI/agRuntime/assistRuntime 套件；不新增业务规则。

test_impact:
  changed_files: 本 Scope 的治理文件、8 个旧测试入口与 3 个目标套件。
  direct_modules: AG、CORE、DE、ED、REF。
  affected_modules: AG、CORE、DE、ED、REF、SYS。
  selection: 测试治理自测、前端全量、TypeScript、Rust 全量与治理审计。

validation:
  - 旧八文件定向基线：8/8 文件、73/73 用例通过；既有 AG/DE/REF covers 集合已冻结。
  - 三个 AG 目标套件联合运行：3/3 文件、73/73 用例通过；AgentUI=20、agRuntime=37、assistRuntime=16。
  - 结构对照：旧/新均为 73 个 `it`；`expect(` 出现次数均为 173，未减少断言入口。
  - test:governance：8/8 自测通过；动态发现 85 个资产、724 个用例；AG frontend_owned/baseline/target 为 3/3/3，owner 用例为 73。
  - test:plan：直接模块 AG、CORE、DE、ED、REF；受影响模块加入 SYS，并触发前端全量、TypeScript、Rust 全量和治理审计。
  - test:related：前端 22/22 文件、455/455 用例通过；TypeScript 通过；沙箱内 Rust 的 LibreOffice 端到端用例因受限子进程 signal 6 失败，未跳过测试。
  - cargo test（沙箱外真实 macOS/LibreOffice 运行条件）：261/261 库测试与 8/8 集成测试通过。
  - npm run build：通过；Bundled DOC tools 校验通过，Vite 生产构建完成。
  - governance:generate：生成 85 个测试资产、724 个用例；第二次执行明确内容无变化，幂等通过。
  - governance:audit：passed；0 unmapped executable、0 unowned test、0 focused/skipped。
  - AI 语义比对：本次 covers 涉及规则的 semantic_review_candidates 与规则原文一致；生产 boundary 未变，无术语漂移发现。
  - npx tsc --noEmit、git diff --check：通过。

stop_conditions:
  - 无。

recovery:
  - SC-04 未形成：沙箱内 LibreOffice worker 崩溃由真实环境完整 Rust 复跑解除，全部 269 个 Rust 测试通过。

unresolved_items:
  - 后续测试文件整合债务仅剩 PRV 2→1；由测试治理审计持续报告，非本 Scope 阻断项。
  - 模块 test cases 是文件级关联计数；合并跨模块 covers 后 ED/DE/REF 的关联资产或用例计数会变化，但 AG owner=73、ED owner=207、DE owner=58 等精确契约保持不变。
  - Vite 仍报告既有单 chunk 超过 500 kB；Rust 仍有既有 dead_code 警告；Bundled tools 校验仍有既有 locale 警告，均未由本次纯测试组织变更引入。
