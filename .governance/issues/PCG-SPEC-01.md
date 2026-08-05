# Issue Trace：PCG-SPEC-01

## 1. 用户可见症状

产品经理无法从现有文档直接判断产品能力、格式支持边界、自动化证据、系统验证状态和下一步缺口。规则、代码和测试虽已登记，但缺少产品视角的统一投影。

## 2. 入口点

- `npm run product:generate` → `scripts/generate_product_capability_atlas.mjs`
- `npm run product:audit` → `scripts/audit_product_capability.mjs`
- `npm run product:test` → `scripts/test_governance.test.mjs` 中的 `Product Capability Graph` 测试组
- 生成物入口：`docs/00_core/generated/product-capability-atlas.html`

## 3. 相关规则

- 新增：`BR-PCG-DATA-001`、`BR-PCG-STATE-001`、`BR-PCG-VERIFY-001`、`BR-PCG-VERIFY-002`。
- 复用：`X-SHARED-001`、`X-SEQ-002`、`X-FORBID-007`。
- 新规则先在 `A-CORE-C-P-02_产品能力图谱规范.md` 注册并生成 ADUS，再由脚本 `@GOV` 消费。

## 4. 相关链路

`PCG-GENERATE`：注册表 → 引用解析/状态汇总 → HTML 与机器可读生成物。  
`PCG-AUDIT`：注册表 → 追溯、主链唯一性、fallback、过期证据检查 → 审计结果。  
消费既有索引：`ED-EDIT`、`AG-TURN`、`DE-DIFF`、`WS-SAVE`、`DOC-CONVERT`。

## 5. 生产者块

当前无 PCG 生产块。本任务新增 `scripts/product_capability.mjs`、生成器、审计器及 ADUS/治理审计的 PCG 消费入口；规则注册后以 `@GOV` 标注其主链入口。

## 6. 消费者块

- 生成的 `product-capability-atlas.html`；
- ADUS 的产品能力索引；
- 测试选择器的 PCG 自测映射；
- 治理审计的 PCG 结果；
- `A-VAL-X-V-03` 的验证记录。

## 7. 路径上的空规则块

当前无 PCG `@GOV` 块。仅字符串转义、排序、HTML 编码等纯工具允许 `rules: []`，且必须以 `VALID_EMPTY` triage 说明；注册、状态判断和审计逻辑不得用空规则承载。

## 8. boundary 假设

- 注册表是能力、变体和人工状态字段的唯一编辑源；生成物不得反写。
- REQ、RULE、CHAIN、TERM、测试与验证只通过 ID 引用，不复制为第二权威源。
- 自动化证据、系统验证与发布承诺独立计算。
- 每个有外部副作用的能力变体有唯一 `canonical_chain`；fallback 必须登记触发条件。
- 沙盘不读取或改写 Binder 运行时业务状态。

## 9. 运行时假设

- 脚本离线运行，不依赖 Tauri、WKWebView、真实 Provider 或网络。
- 生成器只写 `docs/00_core/generated/**`；审计器保持只读。
- `governance:audit` 保持只读；ADUS 只由 `governance:generate` 写入。
- 当前编辑器工具栏的未提交改动不属于本链路，不能纳入本任务变更或验证结论。

## 10. 疑似不变量破坏

- 沙盘若直连运行时数据将越过治理投影边界。
- 注册表若复制 REQ/RULE 正文将形成双重权威源。
- 多条未声明主链、未登记 fallback 或第二写入责任者将破坏链路唯一性。
- 自动化通过若提升系统验证或发布状态，将形成虚假闭合。
- 术语未注册即被 `term_ref` 消费，将触发 `TERM_UNREGISTERED`。

## 11. 验证计划

- `npm run product:test`：schema、状态、追溯、唯一性与 HTML 结构。
- `npm run product:generate`：生成 HTML、覆盖 JSON 与链路审计报告。
- `npm run product:audit`：审计真实注册表的引用和缺口。
- `npm run governance:generate`、`npm run governance:audit`：验证既有治理联动。
- `npm run test:plan`、`npm run test:related`：验证 PCG 变更影响选择。
- `git diff --check`：格式检查。

## Expected Behavior

产品能力沙盘从单一注册表生成，明确区分产品定义、实现、自动化证据、系统验证和发布承诺；审计能发现缺失引用、缺失主链、重复主链、过期证据和重复候选，但不自动改写任何产品代码或证据。

## Validation Commands

- `npm run product:test`
- `npm run product:generate`
- `npm run product:audit`
- `npm run governance:generate`
- `npm run governance:audit`
- `npm run test:plan -- --files [PCG-SPEC-01 changed files]`
- `npm run test:related -- --files [PCG-SPEC-01 changed files]`
- `git diff --check`

## Scope Lock

```yaml
task_type: CODE_CHANGE
allowed_files:
  - .governance/active_scope.json
  - .governance/issues/PCG-SPEC-01.md
  - .governance/product-capability-registry.json
  - governance.config.json
  - package.json
  - docs/00_core/A-CORE-C-P-01_APC.md
  - docs/00_core/A-CORE-C-P-02_产品能力图谱规范.md
  - docs/00_core/A-CORE-C-R-01_ADUS.md
  - docs/00_core/generated/**
  - docs/40_validation/A-VAL-X-V-03_产品能力图谱验证.md
  - scripts/product_capability.mjs
  - scripts/generate_product_capability_atlas.mjs
  - scripts/audit_product_capability.mjs
  - scripts/generate_adus.mjs
  - scripts/audit_taskflow_governance.mjs
  - scripts/test_governance.mjs
  - scripts/test_governance.test.mjs
forbidden_files:
  - src/**
  - src-tauri/**
  - buildbot/**
  - ADU.md
  - ADP.md
  - AGENTS.md
  - CLAUDE.md
  - package-lock.json
  - src/components/ed/EditorToolbar.tsx
  - src/components/ed/ImageMoveControls.tsx
  - src/components/ed/TableMoveControls.tsx
  - src/components/ed/EditorControls.test.tsx
  - src/styles.css
confirmation: YES
scope_reason: PCG-SPEC-01 仅实现治理注册表、生成/审计工具和文档试点，不改产品运行时。
```
