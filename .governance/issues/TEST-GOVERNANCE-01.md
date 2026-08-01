# Issue Trace — 面向 AI Coding 的测试治理、影响选测与验证闭环

## 1. 用户可见症状

- 当前测试资产数量多、分布广，AI 修改代码后只能凭经验决定运行哪些测试，无法稳定回答“为什么触发
  这些测试、是否遗漏相关模块”。
- `// covers: BR-*` 已能证明规则存在测试，但没有形成 changed files → module/rule/chain → test files →
  validation report 的可执行链路。
- 测试与验证的边界、动态需求下的测试演进、测试文件数量治理仅存在于讨论中，未成为 AI/CI 可复用的
  持久化能力。

## 2. 入口点

- AI 开发任务进入 ADP `VALIDATING`。
- 本地开发者或 CI 提交一组 changed files。
- `npm run governance:generate` 刷新 ADUS。
- `npm run governance:audit` 执行合入硬门。

## 3. 相关规则

- ADU §5.6：测试文件通过 `covers` 连接规则注册表，测试文件不写 `@GOV`。
- ADP §5：实现任务必须运行相关测试与治理审计。
- APC §5/§10/§12：G4–G6、核心链、状态机和合入测试硬门。
- AGENTS §1/§5.1：实际验证后才能声明完成。

## 4. 相关链路

- Requirement → Rule/Chain → @GOV → Test covers → Task Runtime Report。
- ChangedFile → TestImpact → RelatedTestPlan → AutomatedTestRun → ValidationEvidence。
- TestRegistry → governance:generate → ADUS TestCapabilityIndex。

## 5. 生产者块

- 设计文档生产 MODULE/RULE/CHAIN/CONSTRAINT/TERM。
- 源码 `@GOV` 生产 module/rules/chain 映射。
- 测试文件 `covers` 生产规则覆盖证据。
- Git changed files 生产单次开发影响输入。

## 6. 消费者块

- `scripts/test_governance.mjs` 消费注册表、源码标注、测试注释和 changed files。
- ADP VALIDATING 消费 RelatedTestPlan 并要求实际执行。
- ADUS 消费动态 inventory，生成模块测试能力索引。
- AI/CI 消费稳定 npm scripts，输出可复核的验证证据。

## 7. 路径上的空规则块

- 现有 ADU 只定义 covers 语法，没有定义测试注册表、模块归属、影响选测、文件预算和失败关闭语义。
- 现有 ADP 只要求“相关测试”，没有定义相关性的机器来源和 Runtime Report 必填证据。
- 现有 APC 只声明测试层级和硬门，没有项目风险分层、阶段加深及全量升级条件。
- 现有 AGENTS 没有稳定的测试选择命令路由。

## 8. boundary 假设

- 设计规则与模块是测试选择的治理真值；文件路径只是无法读取标注时的保守回退。
- 测试文件清单是动态资产，必须扫描产生；注册表只登记稳定分类和选择规则。
- 相关测试选择可以多跑，不能漏跑；无法映射的可执行变更必须失败关闭。
- 自动测试证明可重复断言，人工验证证明真实需求体验，两者分别留痕，互不冒充。

## 9. 运行时假设

- Node、npm、Cargo 已由项目工具链提供，不新增依赖。
- Git 工作区可能包含未跟踪的本机外部目录；仅注册的项目路径进入影响分析。
- Rust 内联测试难以按文件稳定执行，首期允许任何 Rust 业务变更保守升级为全量 Rust test。
- G5 真实 GUI E2E 尚未建设，必须登记为验证缺口，不能用 jsdom 冒充。

## 10. 疑似不变量破坏

- 仅按同名测试文件选测会漏掉跨服务、Store、组件和 Rust 内联测试。
- 强制“一模块一个测试文件”会把不同运行环境和风险域压成巨型文件，降低可维护性。
- 只看覆盖率百分比会掩盖规则、异常分支、状态机禁止转移和跨模块链路缺口。
- 将目标文件数立即设为硬门会诱发机械合并测试，增加 mock 污染和审阅成本。

## 11. 验证计划

- 脚本单元：glob、模块推断、covers/@GOV 提取、依赖扩展、测试直选、未映射失败关闭、全量升级。
- 真实 inventory：当前全部前端测试与 Rust 测试资产均被发现且无未映射项。
- 真实 select：ED、AG、治理脚本、直接测试文件及未知业务路径的选择结果符合预期。
- 集成：`governance:generate` 产生 TestCapabilityIndex，`governance:audit` 硬门包含 test audit。
- 回归：前端、Rust、构建与既有治理审计全量通过。

## Scope Lock

- scope_id：`TEST-GOVERNANCE-01`
- task_type：`CODE_CHANGE`
- governance_phase：`IMPLEMENTATION`
- confirmation：`YES`（人类于 2026-08-01 明确确认切换到本 Scope）。
- allowed_files / forbidden_files：以 `.governance/active_scope.json` 为准。
- 当前状态：`READY`。

## 12. 预期实施结果

- 新增项目级 Test Governance Registry 与无依赖执行器。
- 新增 inventory/audit/select/run 命令和脚本自测。
- ADU/ADP/APC/AGENTS 分层登记测试治理职责。
- ADUS 自动生成模块测试能力索引。
- 当前测试文件结构先冻结增长、登记收敛目标，物理整合另开后继 Scope。

## 13. Task Runtime Report

task_type: CODE_CHANGE
governance_phase: IMPLEMENTATION
final_state: READY
issue_trace: TEST-GOVERNANCE-01

changed_files:
  - `.governance/active_scope.json`：TEST-GOVERNANCE-01 Scope 状态与边界。
  - `.governance/issues/TEST-GOVERNANCE-01.md`：Issue Trace 与运行证据。
  - `.governance/test-governance.json`：模块、风险、路径、消费者、预算与系统验证注册表。
  - `governance.config.json`：登记 test_governance_path。
  - `ADU.md` / `ADP.md` / `AGENTS.md`：静态规则、运行协议与 AI 入口。
  - `docs/00_core/A-CORE-C-P-01_APC.md`：Binder 项目测试策略与硬门。
  - `docs/00_core/A-CORE-C-R-01_ADUS.md`：动态测试覆盖、能力与系统验证索引。
  - `docs/40_validation/A-VAL-X-V-01_测试治理与验证矩阵.md`：测试治理到 G5/G6 验证方案。
  - `package.json`：稳定的 inventory/audit/plan/run/full 命令。
  - `scripts/test_governance.mjs` / `scripts/test_governance.test.mjs`：执行器与 8 项自测。
  - `scripts/generate_adus.mjs` / `scripts/audit_taskflow_governance.mjs`：ADUS 与治理硬门接线。

scope:
  mode: explicit
  status: confirmed

phase_gate:
  status: passed
  evidence: APC 已定义 G4–G6 测试门禁；本 Scope 补足机器执行能力，不新增业务功能。

mapping_chain:
  status: complete
  evidence: RULE/CHAIN/MODULE 与 covers 通过 TestRegistry、TestImpact 连接到 ADUS TestCapabilityIndex 和 Runtime Report。

test_impact:
  changed_files: 本 Scope 15 个受控文件；明确排除外部 `buildbot/`。
  direct_modules: CORE
  affected_modules: CORE
  selection: test-governance + frontend-full + governance-audit；按 Scope 验收额外执行 rust-full 与 build。

validation:
  - `npm run test:inventory`：passed；121 资产、724 用例、97 可执行源均可映射。
  - `npm run test:plan -- --files [本 Scope changed_files]`：passed；CORE → test-governance/frontend-full/governance-audit。
  - `npm run test:related -- --files [本 Scope changed_files]`：passed；自测 8/8、前端 58/58 文件与 455/455 用例、governance audit 全通过。
  - 未映射路径 `src/unknown/newCapability.ts` 负向验证：按预期 failed closed。
  - `cargo test --manifest-path src-tauri/Cargo.toml`：passed；261 单元 + 8 集成，0 failed/ignored。
  - `npm run build`：passed；bundled DOC tools、tsc、vite build 通过。
  - `npm run governance:generate` 连续执行：passed；第二次内容无变化。
  - `node --check`（三个治理脚本）与 `git diff --check`：passed。

stop_conditions:
  - 无。

recovery:
  - 无。

unresolved_items:
  - GUI E2E 自动化框架不在本 Scope；两个 G5 系统验证场景已以 partial/deferred 登记，未伪造为 passed。
  - SYS、WS、ED、DE、AG、PRV 测试文件数高于整合目标，已冻结基线并登记后继 Scope；本轮未搬迁或弱化测试。
  - `BR-SYS-DATA-004` 属于客户端仓库外云 provider catalog 契约，沿用既有 CLIENT_OUT_OF_SCOPE 裁决。
  - 构建保留既有 locale、bundle size 与 Rust dead_code 警告，均未由本 Scope 引入且不阻断本次正确性。
