# Issue Trace — PRV 前端测试资产治理整合

## 1. 用户可见症状

测试治理审计持续报告 `TEST_FILE_CONSOLIDATION_DEBT: PRV 2 → 1`。现有测试覆盖预览/导出只读服务契约
与 CSV/TSV 表格解析，功能覆盖稳定，但物理入口多于登记目标。

## 2. 入口点

- `TEST-GOVERNANCE-01` 冻结 PRV 的 2 文件 baseline 与 1 文件 target。
- ED、SYS、WS、DE、AG 测试整合已完成，PRV 是注册表中最后一项文件整合债务。
- 人类回复“继续下一步”，确认启动 PRV 整合。

## 3. 相关规则

- `BR-PRV-DATA-001`：预览纯只读，不产生内容修改、文档模型变化或脏态。
- `BR-PRV-DATA-002`：预览转换遵循 DOC 管线、离线与失败显式边界。
- ADU §5.6：baseline 禁止继续增长；target 是分期整合目标，不得牺牲诊断性和断言。
- APC：PRV 为 high 风险模块，相关变更扩展 SYS 并保守运行 Rust。
- AGENTS §2：不得删除、注释或弱化测试制造通过。

## 4. 相关链路

`OldPrvTests → PrvRuntimeSuite → DirectedRun → FullRegression → DynamicInventory → ADUS`。

## 5. 生产者块

- `prvService.test.ts` 与 `tableData.test.ts` 的 import、mock、hook、describe、it 和 covers。
- `.governance/test-governance.json` 的 PRV test_globs、baseline 与 target。

## 6. 消费者块

- Vitest 直接发现 `prvRuntime.test.ts`。
- `test_governance.mjs` 动态计算 owner、layer、test_count 和 covers。
- PRV high 风险影响计划扩展 SYS，触发前端与 Rust 回归。

## 7. 路径上的空规则块

无新的业务规则缺口。本 Scope 只改变测试组织，不改变 RULE、CHAIN 或生产行为。

## 8. boundary 假设

- service describe 保留 Tauri command、asset URL 与只读命令白名单断言。
- tableData describe 保留 CSV/TSV 解析、转义、空输入与扩展名识别断言。
- 文件名不是业务真值；合并后由 owner、layer、covers 和 describe 名保持定位能力。

## 9. 运行时假设

- 两个旧套件均使用默认 node 环境。
- Tauri core mock 只服务 prvService describe，不改变 tableData 纯函数行为。
- 合并后 top-level beforeEach 会在 tableData 用例前额外重置 mock，但不改变纯函数输入输出。

## 10. 疑似不变量破坏

- Tauri core mock 或 URL.createObjectURL 桩在合并后失效。
- relative import 因目标文件重命名而漂移。
- 表格解析断言被错误纳入服务 mock 语义。
- 只减少文件计数却把旧用例藏入非 test helper，形成虚假治理。

## 11. 旧文件到目标套件映射

| 目标套件 | layer / environment | 旧文件 |
|---|---|---|
| `services/prvRuntime.test.ts` | service / node | prvService / tableData |

## 12. 验证计划

1. 合并前冻结 2 文件的 7 个 owner 用例与 covers 集合，并运行定向基线。
2. 目标套件独立运行，修复 import、mock、hook 隔离，不改断言。
3. 动态 inventory 对比 PRV frontend_owned、用例数与 covers。
4. 执行 test:plan/test:related、真实环境 Rust、构建与治理审计。
5. 连续生成 ADUS 验证幂等。

## Scope Lock

- scope_id：`TEST-CONSOLIDATION-PRV-01`
- task_type：`CODE_CHANGE`
- governance_phase：`IMPLEMENTATION`
- confirmation：`YES`（人类于 2026-08-01 回复“继续下一步”）。
- allowed_files / forbidden_files：以 `.governance/active_scope.json` 为准。
- 当前状态：`READY`。

## 13. Task Runtime Report

task_type: CODE_CHANGE
governance_phase: IMPLEMENTATION
final_state: READY
issue_trace: TEST-CONSOLIDATION-PRV-01

scope:
  mode: explicit
  status: confirmed

phase_gate:
  status: passed
  evidence: TEST-GOVERNANCE-01 已登记 PRV 2→1 债务，人类已确认继续下一步。

mapping_chain:
  status: complete
  evidence: 2 个旧文件已映射到 1 个 prvRuntime 套件；不新增业务规则。

test_impact:
  changed_files: 本 Scope 的治理文件、2 个旧测试入口与 1 个目标套件。
  direct_modules: CORE、PRV。
  affected_modules: CORE、PRV、SYS。
  selection: 测试治理自测、CORE/PRV/SYS 相关前端、TypeScript、Rust 全量与治理审计。

validation:
  - 旧两文件定向基线：2/2 文件、7/7 用例通过；既有 PRV covers 集合已冻结。
  - PRV 目标套件独立运行：1/1 文件、7/7 用例通过。
  - 结构对照：旧/新均为 7 个 `it`；`expect(` 出现次数均为 14，未减少断言入口。
  - test:governance：8/8 自测通过；动态发现 84 个资产、724 个用例；PRV frontend_owned/baseline/target 为 1/1/1，owner 用例为 7；整合 warning 为 0。
  - test:plan：直接模块 CORE、PRV；受影响模块加入 SYS，选择 5 个相关前端套件、TypeScript、Rust 全量和治理审计。
  - test:related：相关前端 5/5 文件、86/86 用例通过；TypeScript 通过；沙箱内 Rust 的 LibreOffice 端到端用例因受限子进程 signal 6 失败，未跳过测试。
  - cargo test（沙箱外真实 macOS/LibreOffice 运行条件）：261/261 库测试与 8/8 集成测试通过。
  - npm test：21/21 文件、455/455 用例通过。
  - npm run build：通过；Bundled DOC tools 校验通过，Vite 生产构建完成。
  - governance:generate：生成 84 个测试资产、724 个用例；第二次执行明确内容无变化，幂等通过。
  - governance:audit：passed；0 unmapped executable、0 unowned test、0 focused/skipped、0 test governance warning。
  - AI 语义比对：BR-PRV-DATA-001/002 的 semantic_review_candidates 与只读预览、显式失败及 workspace 内导出规则一致；生产 boundary 未变。
  - git diff --check：通过。

stop_conditions:
  - 无。

recovery:
  - SC-04 未形成：沙箱内 LibreOffice worker 崩溃由真实环境完整 Rust 复跑解除，全部 269 个 Rust 测试通过。

unresolved_items:
  - 注册表中的测试文件整合债务已全部清零；后续工作转入独立 VALIDATION Scope，闭合两项 deferred G5 系统验证场景。
  - Vite 仍报告既有单 chunk 超过 500 kB；Rust 仍有既有 dead_code 警告；Bundled tools 校验仍有既有 locale 警告，均未由本次纯测试组织变更引入。
