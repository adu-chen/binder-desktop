# FUNCTION-LEDGER-AG10-01：AG 1.0 功能明细同步与工作簿格式修复

文档状态：A

## 1. 用户可见症状

功能明细表只登记了旧版 AG 面板能力，未覆盖 agent-edit-1.0 已确认的 Runtime、Intent、Context、Prompt、
逐回合工具授权、新建空基准 Diff、冲突保护和脱敏追踪能力；多项已接线能力仍标记为“待审计”。同时现有
xlsx 的样式索引被写成 `s1/s3` 字符串，严格解析器无法导入，影响后续审计和自动化消费。

## 2. 入口点

- 人类动作：检查并更新 `docs/00_core/generated/binder-product-function-ledger.xlsx`。
- 数据入口：`.governance/function-ledger.json`。
- 生成入口：`scripts/generate_function_ledger.mjs::main/buildSheet`。

## 3. 相关规则

- `BR-PCG-DATA-001`
- `BR-PCG-STATE-001`

## 4. 相关链路

`PCG-GENERATE`

## 5. 生产者块

- `BR-PCG-DATA-001-RB-PCG-PCG-GENERATE-001, BR-PCG-STATE-001-RB-PCG-PCG-GENERATE-001`
  位于 `scripts/generate_function_ledger.mjs`。

## 6. 消费者块

- xlsx 为 PCG-GENERATE 的生成物；当前 ADUS 未登记独立代码消费者块。
- 人工审计和产品功能盘点直接消费 `docs/00_core/generated/binder-product-function-ledger.xlsx`。

## 7. 路径上的空规则块

未发现 `rules: []` 块，无需 triage。

## 8. boundary 假设

现有 @GOV 块未声明 `lifecycle_assumption` 或 `document_snapshot`。本次保持“JSON 是唯一人工维护数据源，
xlsx 只由生成器产出”的生命周期边界，不手工修改 xlsx 内容。

## 9. 运行时假设

现有 @GOV 块未声明 `caller_context` 或 `runtime_guard`。运行时假设为 Node 环境存在系统 `zip`，输入 JSON
符合 schema 且 status 属登记枚举；本次不改变这些假设。

## 10. 疑似不变量破坏

- 若只改 xlsx，会破坏数据源单一真值。
- 若继续输出非数字样式索引，会生成严格解析器不可消费的工作簿。
- 若把本地自动化通过写成 G5/已验收，会破坏状态口径。

本次通过修改唯一数据源、修正既有样式序列化并保留“已实现 · 待核验 · EP”口径控制风险。

## 11. 验证计划

- 运行 `node scripts/generate_function_ledger.mjs`，确认只由数据源生成目标 xlsx；不刷新范围外产品图谱。
- 校验 JSON schema、ID 唯一性、状态枚举、AG 行数与预期条目。
- 使用严格工作簿解析器导入、检查关键范围、扫描公式错误并渲染唯一工作表。
- 运行变更影响选择器给出的测试、`npm run product:audit` 与 `npm run governance:audit`。
- 核验无链路日志变化；无需更新 triage。

## Scope Lock 输入

允许修改的文件：

- `.governance/function-ledger.json`
- `scripts/generate_function_ledger.mjs`
- `docs/00_core/generated/binder-product-function-ledger.xlsx`
- `.governance/issues/FUNCTION-LEDGER-AG10-01.md`
- `.governance/active_scope.json`

禁止修改的文件：

- 上述允许列表外的文件默认禁止。
- 特别禁止修改 ADU、ADP、AGENTS、APC、ADUS、产品能力注册表、AG 需求/设计/实现和测试文件。

## 人类授权

2026-08-08 用户明确要求检查并更新指定功能明细表；修复生成器的非法样式索引是生成可用目标文件的必要步骤，
不扩展到其他产品能力或运行时代码。

## Task Runtime Report — AG 1.0 功能明细同步与工作簿格式修复

task_type: CODE_CHANGE
governance_phase: IMPLEMENTATION
final_state: READY
issue_trace: FUNCTION-LEDGER-AG10-01

changed_files:
  - `.governance/function-ledger.json` # 更新既有 AG/REF 状态并新增 15 条 agent-edit-1.0 原子能力
  - `scripts/generate_function_ledger.mjs` # 把非法 `s1/s3` 样式字符串修正为标准数字索引
  - `docs/00_core/generated/binder-product-function-ledger.xlsx` # 由唯一数据源重新生成 162 行台账
  - `.governance/issues/FUNCTION-LEDGER-AG10-01.md` # Issue Trace 与验证留痕
  - `.governance/active_scope.json` # Scope Lock 与最终状态

scope:
  mode: explicit
  status: confirmed

phase_gate:
  status: passed
  evidence: PCG-GENERATE 既有规则、链路、@GOV 和测试治理映射完整；未修改静态定义。

mapping_chain:
  status: complete
  evidence: `BR-PCG-DATA-001 / BR-PCG-STATE-001 → PCG-GENERATE → generate_function_ledger.mjs → xlsx`。

test_impact:
  changed_files: `.governance/function-ledger.json`、`scripts/generate_function_ledger.mjs`、目标 xlsx、Issue Trace、Scope Lock
  direct_modules: CORE
  affected_modules: CORE
  selection: governance:audit

validation:
  - `node scripts/generate_function_ledger.mjs` # passed，162 行，状态计数合法
  - JSON ID/状态/WP 覆盖检查 # passed，32 条 F-AG、无重复 ID、八个工作包全部覆盖
  - 严格 xlsx 导入与 JSON 逐单元格比对 # passed，162 行、0 mismatch
  - 公式错误扫描与全表/AG 区域渲染 # passed，0 错误，布局和状态色保留
  - `unzip -t docs/00_core/generated/binder-product-function-ledger.xlsx` # passed
  - `npm run test:plan -- --files [...]` # passed，选择 CORE governance:audit
  - `npm run test:related -- --files [...]` # passed
  - `npm run product:audit` # passed，无 findings/warnings
  - `npm run governance:audit` # passed

stop_conditions:
  - 无

recovery:
  - 无

unresolved_items:
  - 真实账号/provider/Tauri/WKWebView G5 仍按 AG 1.0 计划保留为待验证，台账未将其写成已验收。
