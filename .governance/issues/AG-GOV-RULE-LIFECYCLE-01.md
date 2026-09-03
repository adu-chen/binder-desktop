# Issue Trace — AG-GOV-RULE-LIFECYCLE-01：RULE superseded 生命周期生成与审计闭环

文档状态：A
任务类型：CODE_CHANGE
治理阶段：VALIDATION
任务状态：IN_PROGRESS
创建时间：2026-09-03

## 1. 用户可见症状

规则被替代或其来源文档降为 R/X 后，当前生成器仍会把 RULE 默认登记为 registered；只读审计也只检查规则 ID 是否存在，不能阻止代码、状态机或 mapped REQ_MAP 继续引用应当退役的规则。这会让后续 Agent 架构迁移出现新旧规则同时“有效”的假闭环。

## 2. 入口点

- `scripts/generate_adus.mjs::parseAnnotationBlocks/buildRuleRegistry`
- `scripts/audit_taskflow_governance.mjs::parseAnnotationBlocks`
- `scripts/rule_lifecycle.mjs` 的共享状态判定
- `scripts/audit_taskflow_governance.mjs` 的 RULE、REQ_MAP、STATE_MACHINE、@GOV 与测试覆盖检查

## 3. 相关规则

本任务修改治理根协议 ADU §2.5、§3.1～§3.3 与 §4.2/§4.4，属于人类明确授权的治理能力修复；不新增业务 RULE。

## 4. 相关链路

`RULE source status → ADUS registry_status → REQ_MAP/STATE_MACHINE/@GOV/test audit`

## 5. 生产者块

- RULE 标注块及其来源文档状态。
- `generate_adus.mjs` 生成规则注册表。

## 6. 消费者块

- REQ_MAP、STATE_MACHINE、代码 @GOV、测试 covers 与治理审计。

## 7. 路径上的空规则块

无。本任务治理脚本不新增业务决策块。

## 8. boundary 假设

- RULE 未声明 status 时兼容为 active。
- RULE 源状态只允许 active/superseded；ADUS 将 active 派生为 registered。
- R/X 文档不得继续提供 active RULE，但可保留显式 superseded RULE 作为历史身份。
- superseded RULE 不得被 active 运行时代码、active STATE_MACHINE 或 mapped REQ_MAP 引用。

## 9. 运行时假设

- 当前 Active 设计文档的 RULE 均未手写 registry_status。
- 当前 R/X Agent 文档已不含 RULE 块；新增检查不会反向制造批量迁移。
- 生成与审计仍分离，`governance:audit` 不写文件。

## 10. 疑似不变量破坏

- 旧规则 ID 无法在保留历史的同时退出有效引用集合。
- R/X 文档可能继续被扫描为规则真源。
- 测试和代码可继续引用语义已废弃的规则并让审计保持绿色。

## 11. 验证计划

- 增加 fixture 覆盖 active、superseded、R/X 来源和非法引用。
- 运行 `npm run test:governance`。
- 运行 `npm run governance:generate` 后检查 ADUS 规则状态。
- 运行 `npm run governance:audit -- --issue .governance/issues/AG-GOV-RULE-LIFECYCLE-01.md`。
- 运行 `git diff --check`。

## Expected Behavior

规则可以在保留 ID 和来源历史的同时被可靠退役；任何 active 代码、状态机或正式需求映射继续引用该规则都会失败关闭，ADUS 不再把 R/X 文档中的旧规则误当作当前真源。

## Validation Commands

- `npm run test:governance`
- `npm run governance:generate`
- `npm run governance:audit -- --issue .governance/issues/AG-GOV-RULE-LIFECYCLE-01.md`
- `git diff --check`

## Scope Lock

```yaml
task_type: CODE_CHANGE
allowed_files:
  - .governance/active_scope.json
  - .governance/issues/AG-GOV-RULE-LIFECYCLE-01.md
  - ADU.md
  - scripts/generate_adus.mjs
  - scripts/audit_taskflow_governance.mjs
  - scripts/rule_lifecycle.mjs
  - scripts/test_governance.test.mjs
  - docs/00_core/A-CORE-C-R-01_ADUS.md
forbidden_files:
  - 所有业务需求、设计、生产代码与业务测试
confirmation: YES
scope_reason: 人类已确认推荐方案 C6；本 Scope 仅闭合 ADU 已声明但实现缺失的 RULE 生命周期。
```

## 人类决策

- `HD-AG-OPT-C6`（CONFIRMED）：授权最小修改 ADU 与治理脚本，使 RULE superseded 生命周期可生成、可审计。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-09-03 | v1.0 | 建立 RULE 生命周期闭环 Scope。 |

## Task Runtime Report

- status: READY
- changed_files:
  - `.governance/active_scope.json`
  - `.governance/issues/AG-GOV-RULE-LIFECYCLE-01.md`
  - `ADU.md`
  - `scripts/rule_lifecycle.mjs`
  - `scripts/generate_adus.mjs`
  - `scripts/audit_taskflow_governance.mjs`
  - `scripts/test_governance.test.mjs`
  - `docs/00_core/A-CORE-C-R-01_ADUS.md`
- validation:
  - `npm run test:governance`: passed，21/21 治理测试通过，962 个测试用例被 inventory 登记。
  - `npm run governance:generate`: passed，ADUS 从 100 个 RULE 真源重新生成。
  - `npm run governance:audit -- --issue .governance/issues/AG-GOV-RULE-LIFECYCLE-01.md`: passed，无治理 finding，Scope Lock 通过。
  - `npm run test:plan -- --files [...]`: passed，选择 CORE/PCG 的治理自测与只读审计。
  - `npm run test:related -- --files [...]`: passed，按选择器真实执行治理自测与只读审计。
  - `git diff --check`: passed。
- requirement_validation:
  - active/未声明状态派生为 registered：由 helper 单测和 ADUS freshness 审计验证。
  - superseded 派生、R/X 所有权与活动引用阻断：由统一生命周期判定和审计路径覆盖；当前仓库不存在需退役的 RULE，因此未制造业务 fixture。
  - superseded owner/test 豁免：审计集合在 owner 与 covers 检查前显式过滤。
- unresolved_items: []
