# Issue Trace — AG-1.0-DOC-CLOSURE-DCLOSURE-01：D 类文档滞后清理

文档状态：A
任务类型：DOC_ONLY
治理阶段：DESIGN
创建时间：2026-08-14

## 1. 用户可见症状

Agent 能力基线验收发现 5 项 D 类文档滞后（UNC-33/34/35/31/37）。代码已实现但文档仍标"待实现/
planned/BLOCKED"，会误导后续实现与审计（例如误判"澄清未实现"、"SM-AG-04 未建机"、"沙箱仍阻断"）。
本轮仅清理文档与治理留痕，不改任何代码。

## 2. 入口点

- `docs/01_modules/AG/A-AG-M-S-01_代理状态机设计.md` §2.1 迁移债务（:105-107）——澄清已实现为
  AwaitingClarification 非终态（IMP-CLARIFICATION-01 落地），债务仍标"实现 Scope 须…"。
- `docs/01_modules/SYS/A-SYS-M-S-01_状态机总图.md` §3.1（:72-85）TaskRun/AgentSession/ExternalContentRef/
  MemorySelectionRef 四行仍标 planned（D3 判定）；§10（:249-250）债务仍标"待建（D1 登记债务）"。
- `docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md` §5.2（BR-AG-STATE-004，:509-521）描述"AG 须异步触发
  记忆写入"，未登记"1.0 关闭、代码实体保留未接线"的处置（A-AST-M-T-01 §4.1 已关闭）。
- `.governance/issues/DIFF-STATE-COMPASS-01.md`（final_state: BLOCKED；SC-04 VALIDATION_BLOCKED 已被
  A-VAL-X-V-02 判为沙箱假失败、VAL-G5-REG-001 resolved，但 issue 状态未回写）。
- （登记项）UNC-37 system_validation 注册表回填涉及脚本（test_governance.mjs），本 Scope 不触碰，留遗留。

## 3. 相关规则

- 文档契约：变更记录回写、迁移债务关闭（实现落地后同步）、planned→active 回填（A-SYS-M-S-01 §10 流程）。
- 治理：DOC_ONLY 不改变产品行为；不新增 RULE/TERM/状态机。

## 4. 相关链路

无（纯文档同步，不涉及代码链路）。

## 5. 生产者块

无。

## 6. 消费者块

后续实现/审计读文档者（防误判）；能力基线验收口径。

## 7. 路径上的空规则块

无代码改动。

## 8. boundary 假设

- 只改文档与治理 issue 的状态/债务/变更记录字段；不改代码、不改行为语义。
- 历史需求文档（UNC-36 open_issues 格式差异）不重写（避免大范围历史文档改动），登记留痕。
- system_validation 回填（UNC-37）涉脚本，列为遗留。

## 9. 运行时假设

无（DOC_ONLY）。

## 10. 疑似不变量破坏

无（纯文档同步；变更记录留痕保持文档治理可追溯）。

## 11. 验证计划

- `git diff --check`；无测试影响（DOC_ONLY）。
- `npm run governance:audit` 确认无新增违规（文档不含 @GOV 变更）。
- 变更记录逐项注明清理 Scope。

## 12. 文件范围结论（Scope Lock 输入）

允许修改：
- `docs/01_modules/AG/A-AG-M-S-01_代理状态机设计.md`
- `docs/01_modules/SYS/A-SYS-M-S-01_状态机总图.md`
- `docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md`
- `.governance/issues/DIFF-STATE-COMPASS-01.md`
- `.governance/issues/AG-1.0-DOC-CLOSURE-DCLOSURE-01.md`、`.governance/active_scope.json`

禁止修改：代码（src/、src-tauri/）任何文件；其他文档；scripts/（UNC-37 遗留）；治理控制文档。

## 13. 人类确认记录

2026-08-14：人类选择"进入下一步文档清理"——执行 D 类未闭合项（UNC-33/34/35/31）文档与治理回写。
confirmation: YES。

## 14. Task Runtime Report

- task_type：`DOC_ONLY`
- governance_phase：`DESIGN`
- final_state：`READY`
- issue_trace：本文件
- changed_files：
  - `docs/01_modules/AG/A-AG-M-S-01_代理状态机设计.md`（UNC-33：§2.1 澄清迁移债务标记关闭，注明
    IMP-CLARIFICATION-01 落地证据；变更记录 v2.13）
  - `docs/01_modules/SYS/A-SYS-M-S-01_状态机总图.md`（UNC-34：§3.1 四行回填——TaskRun planned→active
    SM-AG-04，AgentSession/ExternalContentRef/MemorySelectionRef 判定无独立状态机并注明落地 Scope；
    §10 D1 债务更新；变更记录 v1.13）
  - `docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md`（UNC-35：§5.2 补 1.0 关闭处置登记——memory_link.rs/
    ast capture 自动写入实体保留未接线；变更记录 v2.37）
  - `.governance/issues/DIFF-STATE-COMPASS-01.md`（UNC-31：final_state BLOCKED→READY 回写，SC-04 沙箱
    假失败判定依据（A-VAL-X-V-02 VAL-G5-REG-001 resolved）+ recovery 处置留痕）
  - 本 Issue Trace、`.governance/active_scope.json`
- mapping_chain：验收 D 类未闭合项（UNC-33/34/35/31）→ 文档/治理回写；不涉及代码。
- test_impact：
  - changed_files：4 文档 + 1 治理 issue（DOC_ONLY，无代码）
  - selection：文档变更 → `git diff --check` + `governance:audit`
- validation：
  - `git diff --check` # passed
  - `npm run governance:audit` # passed（exit 0）
  - 无测试影响（DOC_ONLY）
- automated_tests：无（DOC_ONLY）。
- stop_conditions：无。
- unresolved_items：UNC-37（system_validation 回填，涉脚本，随验证卡执行统一处理）；UNC-36（历史需求
  open_issues 格式差异登记留痕，不主动重写）；UNC-33/34/35/31 已闭合。
