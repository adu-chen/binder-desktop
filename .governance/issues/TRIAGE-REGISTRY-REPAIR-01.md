# Issue Trace · TRIAGE-REGISTRY-REPAIR-01

## 1. 用户可见症状

ADUS 的 triage 汇总始终显示占位内容。现有脚手架延期和字体决策文件都使用 `_triage.md`，但不符合正式 triage 的代码块分类格式；audit 又以任意文本包含方式把它们当作未标注符号豁免。

## 2. 入口点

- 索引入口：`npm run governance:generate` → `scripts/generate_adus.mjs`。
- 审计入口：`npm run governance:audit` → `scripts/audit_taskflow_governance.mjs`。

## 3. 相关规则

本任务修改治理静态定义，不涉及业务 `BR-*` 或 `X-*` 规则。直接依据为 `ADU.md §5.1–§5.4`、`ADP.md §1/§3/§5` 和 APC §12。

## 4. 相关链路

治理链路：结构化登记源文档 → 共享解析器 → generate 的 ADUS 汇总；共享解析器 → audit 的负空间扫描。

## 5. 生产者块

本任务不修改业务执行路径，暂无相关 `@GOV` 生产者块。生成器和 audit 读取的是治理文档中的结构化记录。

## 6. 消费者块

本任务不修改业务执行路径，暂无相关 `@GOV` 消费者块。`generate_adus.mjs` 消费记录生成索引，`audit_taskflow_governance.mjs` 消费记录精确判断延期标注豁免。

## 7. 路径上的空规则块

本次不改业务代码。修复后 audit 将验证每个已有 `@GOV rules: []` 块是否具有一条有效的正式 triage 记录；暂未发现需要迁移的正式 triage 记录。

## 8. boundary 假设

无业务 `lifecycle_assumption` 或 `document_snapshot` 字段。本次边界是：生成器只生成 ADUS，audit 只读；两者必须消费同一结构化登记语义。

## 9. 运行时假设

脚本在 Node.js 环境读取仓库内 Markdown 和源文件。未标注符号的延期豁免必须按 `文件 + symbol` 精确匹配，不能依赖文件名或自然语言文本包含。

## 10. 疑似不变量破坏

- 文本包含匹配会造成同名、子串或文件名错误豁免。
- 将决策记录纳入 triage 会把技术裁决误报为代码空规则例外。
- 将 `BLOCK_UNANNOTATED` 当作正式分类，违反 ADU 要求对未标注块进一步归类的规则。
- 批量豁免当前警告会掩盖既有治理债务。

## 11. 验证计划

- 为共享解析器添加 schema、精确匹配和陈旧延期记录回归测试。
- 运行 `npm run test:governance`、`npm run governance:generate`、`npm run governance:audit`。
- 运行 `npm run test:plan -- --files [实际改动文件]`，再按同一文件列表运行 `npm run test:related`。
- 运行 `git diff --check`；检查 ADUS 汇总和源文件一致。

## Expected Behavior

正式 triage 只记录已有 `@GOV rules: []` 块；脚手架延期只按精确文件与符号豁免；字体方案保留为决策记录且不影响 audit 或 triage 汇总。

## Validation Commands

- `npm run test:governance`
- `npm run governance:generate`
- `npm run governance:audit`
- `npm run test:plan -- --files [changed_files]`
- `npm run test:related -- --files [changed_files]`
- `git diff --check`

## Scope Lock

```yaml
task_type: GOVERNANCE_CHANGE
governance_phase: VALIDATION
allowed_files:
  - .governance/active_scope.json
  - .governance/issues/TRIAGE-REGISTRY-REPAIR-01.md
  - ADU.md
  - scripts/triage_registry.mjs
  - scripts/generate_adus.mjs
  - scripts/audit_taskflow_governance.mjs
  - scripts/test_governance.test.mjs
  - docs/00_core/A-CORE-X-R-02_脚手架占位_标注延期清单.md
  - docs/00_core/A-CORE-X-R-02_脚手架占位_triage.md
  - docs/00_core/A-CORE-X-R-03_需求实现对齐审计与补全计划.md
  - docs/01_modules/DOC/A-DOC-X-R-02_字体身份修复决策记录.md
  - docs/01_modules/DOC/A-DOC-X-R-02_字体身份修复决策_triage.md
  - docs/01_modules/DOC/A-DOC-X-L-01_文档转换开发计划.md
  - docs/00_core/A-CORE-C-R-01_ADUS.md
forbidden_files:
  - src/**
  - src-tauri/src/**
  - buildbot/**
  - ADP.md
  - AGENTS.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - governance.config.json
  - package.json
  - package-lock.json
confirmation: YES
scope_reason: 修复 triage、标注延期与决策记录的治理语义及其生成/审计实现，不改变产品代码或业务设计。
```

## 实施结果

- 新增共享 `triage_registry.mjs`，由 generate 和 audit 共同解析正式 triage 与标注延期清单。
- 正式 triage 仅接受七种分类，并校验其 `codes`、文件与现存 `@GOV rules: []` 的对应关系。
- audit 删除任意文本包含豁免，改为精确 `文件 + symbol` 匹配，并检测文件不存在、符号不可扫描、已标注和复核逾期的陈旧延期记录。
- 脚手架登记迁移为 5 条活动延期记录；`WorkspaceRefView` 是 interface，已从可执行符号延期中关闭。
- 字体专项文件迁移为技术决策记录，不再被 triage 解析或作为 audit 豁免。
- ADUS 已生成：正式 triage 0 条，标注延期 5 条。负空间警告为 231，较旧文本匹配的 226 多出 5 条，均为旧 `workspaceService.ts` 文件名豁免曾连带隐藏的其他导出符号；本次不批量豁免该既有债务。

## Task Runtime Report

```yaml
task_id: TRIAGE-REGISTRY-REPAIR-01
task_type: GOVERNANCE_CHANGE
governance_phase: VALIDATION
final_state: READY
scope:
  allowed_files: 14
  outside_scope_changes: 0
  excluded_external_tree: buildbot/
validation:
  triage_registry_tests: PASS (13/13)
  test_governance: PASS (84 assets; 731 tests; 0 findings; 0 warnings)
  governance_generate: PASS (triage 0; annotation deferrals 5)
  governance_audit: PASS (0 governance findings; BLOCK_UNANNOTATED warning count 231)
  test_plan: PASS
  related_frontend: PASS (21 files; 460 tests)
  related_rust: ACCEPTED_RISK (262/263; existing LibreOffice docx_to_pdf end-to-end worker crashed with signal=6)
  diff_check: PASS
stop_conditions:
  - SC-04 resolved by human acceptance: the existing LibreOffice Worker signal=6 failure is outside this governance Scope and remains separately tracked
unresolved_items:
  - Diagnose and repair the LibreOffice worker signal=6 in a separately approved DOC/runtime Scope, then rerun test:related for this change.
  - The 231 BLOCK_UNANNOTATED warnings remain visible governance debt; this repair intentionally did not suppress them in bulk.
```

人类于 2026-08-01 指示“闭合本次修复并提交 git”，接受上述范围外 Rust 回归失败不阻断本次治理修复交付。
