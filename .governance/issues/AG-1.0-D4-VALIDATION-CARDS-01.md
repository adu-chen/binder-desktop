# Issue Trace — AG-1.0-D4-VALIDATION-CARDS-01：Agent 1.0 验证卡设计

文档状态：A
任务类型：DOC_ONLY
治理阶段：DESIGN
创建时间：2026-08-14

## 1. 用户可见症状

冻结基线（A-CORE-X-D-02 §6 第 4 项）要求为每项状态化链路和 P1 开放条件建立验证卡，只有存在可执行证据
才进入 IMPLEMENTATION。D1/D2/D3 已完成三态分流、连续 Proposal、保存、澄清/终态、TaskRun、外部内容/记忆
的 DESIGN 契约与状态机，但尚无对应的验证卡设计，无法作为进入实现的门禁依据。

## 2. 入口点

冻结记录 `A-CORE-X-D-02_Agent1.0冻结能力基线.md` §6 第 4 项与 `A-CORE-X-D-01_1.0需求基线与文档体系索引.md`
§6 第 4 步（验证卡至少覆盖 D-NEW 的 N1–N4 及多会话引用隔离）。

## 3. 相关规则

BR-CORE-STATE-001~003（三态分流）、BR-SYS-STATE-007/008（运行时真值/TaskRun 接力）、BR-SYS-DATA-007
（外部内容/记忆隔离）、BR-AG-STATE-001/002、BR-DE-STATE-001/002、BR-WS-STATE-001、BR-ED-STATE-001、
BR-SYS-PERSIST-001；状态机 SM-DE-01、SM-ED-01、SM-WS-01/02/03、SM-AG-01/04。

## 4. 相关链路

DE-DIFF、ED-EDIT、ED-DECOR、WS-SAVE、WS-DIFF-APPLY、AG-TURN、AG-TOOL、REF-INJECT、AST-MEMORY、
DATA-MIGRATE、IPC-CALL。

## 5. 生产者块

- `docs/40_validation/A-VAL-X-V-04_Agent1.0验证卡.md`（新建，本 Scope 核心产出）
- 既有验证文档格式参考：A-VAL-X-V-02（子场景卡字段结构）
- D1/D2/D3 的设计产出（状态机与契约，只读引用）

## 6. 消费者块

- 后续 IMPLEMENTATION Scope（以验证卡为进入门禁与验收依据）。
- G5 集成/系统验证（执行验证卡，登记自动化与真实环境证据）。
- 产品能力注册表（system_validation 证据登记，随实现后回填）。

## 7. 路径上的空规则块

本任务为 DOC_ONLY，不修改代码；无 @GOV 块路径。

## 8. boundary 假设

- 验证卡是**设计产物**（DESIGN phase）：定义场景、期望、证据设计与门禁，不执行验证；卡状态一律
  `DESIGNED`（未执行），不填报"通过"。
- 验证卡只引用已注册规则与既有状态机（D1/D2/D3），不新增 RULE/TERM/状态机。
- P1 开放条件按子能力、格式、权限、确认/保存/恢复语义与真实验证独立开放，不合并默认开启。

## 9. 运行时假设

- D4 期间不修改 src/、src-tauri/、Prompt、工具注册、测试或 UI。
- 不修改既有验证文档（A-VAL-X-V-01/02/03）与测试治理注册表（实现后回填）。

## 10. 疑似不变量破坏

未发现不变量破坏风险。新文档不注册规则标注块；验证卡引用的规则/状态机均已存在。

## 11. 验证计划

- 检查新文档的文档头、引用关系（规则/状态机 ID 存在性）与场景卡完整性；
- 运行 `npm run governance:generate`（确认无标注块新增、ADUS 无异常）与 `npm run governance:audit`
  （确认无新增 failed）。

## 12. 文件范围结论（Scope Lock 输入）

允许修改：
- `docs/40_validation/A-VAL-X-V-04_Agent1.0验证卡.md`（新建）
- `.governance/issues/AG-1.0-D4-VALIDATION-CARDS-01.md`（本文件）
- `.governance/active_scope.json`

禁止修改（不属本 Scope）：任何 src/、src-tauri/、Prompt、工具注册、测试、UI 文件；既有验证文档
（V-01/02/03）；测试治理注册表、产品能力注册表（实现后回填）；模块 L1/L2；产品根、能力基线、冻结记录；
治理控制文档；governance.config.json。ADUS 自动生成区仅通过 `npm run governance:generate` 更新。

## 13. Task Runtime Report

- task_type：`DOC_ONLY`
- governance_phase：`DESIGN`
- final_state：`READY`
- changed_files：
  - `docs/40_validation/A-VAL-X-V-04_Agent1.0验证卡.md`（新建：验证身份、追溯矩阵、状态化链路验证矩阵、
    核心场景卡 V-N1~N4 + V-S1~S9、P1 开放条件清单 V-P1-01~07、进入 IMPLEMENTATION 门禁规则）
  - 本 Issue Trace、`.governance/active_scope.json`
- delivered_artifact：Agent 1.0 验证卡设计（VAL-AG-1.0-CORE，卡状态一律 DESIGNED）。
- mapping_chain：验证卡引用已注册规则（BR-CORE-STATE-001~003、BR-SYS-STATE-007/008、BR-SYS-DATA-007 等）
  与既有状态机（SM-DE-01/SM-ED-01/SM-WS-01/02/SM-AG-01/04）；不新增 RULE/TERM/状态机。
- test_impact：不适用（DOC_ONLY，无代码/测试改动）。
- validation：
  - `npm run governance:generate` # passed（ADUS 无变化，新文档无标注块）
  - `npm run governance:audit` # failed（两项，与 D1/D2/D3 后一致，无新增：BR-SYS-STATE-007/008、
    BR-SYS-DATA-007 无代码引用/测试 covers，阶段一注册规则）；其余维度全部 passed
  - `git diff --check` # passed
  - 文档头、引用关系（引用的规则/状态机 ID 均存在）、变更记录 # passed
  - changed_files 范围核对 # passed（仅 1 个新验证文档 + 治理记录）
- automated_tests：不适用（DOC_ONLY）。
- stop_conditions：无。
- unresolved_items：验证卡实际执行与证据登记在 IMPLEMENTATION 后的 VALIDATION/G5 Scope 进行；P1 子卡
  细化（具体 fixture）由对应 P1 开放时补充；产品能力注册表 system_validation 随实现后回填。
