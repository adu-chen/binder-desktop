# Issue Trace — AG-1.0-D1-SYS-SHARED-CONTRACT-01：Agent 1.0 SYS 共享契约与跨模块事件 DESIGN 同步

文档状态：A
任务类型：DOC_ONLY
治理阶段：DESIGN
创建时间：2026-08-14

## 1. 用户可见症状

Agent 1.0 Baseline v1.0 已冻结 R1–R5-C 需求范围，但 SYS L2 尚无 RuntimeProfile、AgentTurn（澄清非终态）、
IntentResult、CapabilityGrant、TaskRun、任务步骤、结构化可见事件、外部内容引用与显式项目记忆引用的
共享对象契约，也无对应 REQ_MAP 与状态机归属登记。继续 D2/D3 前缺少单一契约来源，模块会各自定义边界，
重新产生隔离、授权与事件真值漂移。

## 2. 入口点

冻结记录 `A-CORE-X-D-02_Agent1.0冻结能力基线.md` §6「进入 DESIGN 的有序输入」第 1 项：
SYS 先定义共享对象与事件边界（Runtime Profile、AgentTurn、AgentSession、TaskRun、Proposal 链、
外部内容与显式记忆隔离），不先锁定实现细节。承接 REQ-SYS-008/009/010/011/012 的 L2 映射入口。

## 3. 相关规则

现有（registered）：
- BR-CORE-STATE-001~003（三态三分流，D1 引用不修改）
- BR-SYS-DATA-002（共享结构单一来源）、BR-SYS-PERSIST-001（schema 版本化迁移）
- BR-AG-STATE-002（工具授权校验，A-AG-M-T-01）
- X-SHARED-001、X-FORBID-008/009、X-INTER-001、X-SEQ-003

D1 新增（本轮注册）：
- BR-SYS-STATE-007（Runtime Profile 唯一性与 AgentTurn 事件真值）
- BR-SYS-STATE-008（TaskRun 受限编排与 Proposal 接力）
- BR-SYS-DATA-007（外部内容引用与显式记忆引用隔离契约）
- X-SHARED-002（Agent 共享对象契约单一来源与授权非来源）

## 4. 相关链路

AG-TURN（AgentTurn 生命周期）→ DE-DIFF（Proposal 链）→ ED-EDIT（LogicalState 承接）→ WS-SAVE /
WS-DIFF-APPLY（三分流落盘）；REF-INJECT（引用与外部内容/记忆选择）→ AST-MEMORY（用户管理的项目记忆边界）；
IPC-CALL（结构化事件传递）；DATA-MIGRATE（持久化迁移契约）。D1 复用既有链路，不新增链路。

## 5. 生产者块

- `docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md`（本轮扩展，共享对象契约 SSOT）
- `docs/01_modules/SYS/A-SYS-M-S-01_状态机总图.md`（本轮扩展，状态机归属与跨模块转移契约）
- `docs/00_core/A-CORE-X-D-02_Agent1.0冻结能力基线.md`（需求输入，不改动）
- `docs/00_core/A-CORE-C-T-01_三态模型.md`（BR-CORE-STATE-001~003，不改动）

## 6. 消费者块

- D2（DE/ED/WS DESIGN Scope）：消费 Proposal 链、三态分流、保存/裁决契约。
- D3（AG/REF/AST DESIGN Scope）：消费 RuntimeProfile、AgentTurn、AgentSession、IntentResult、
  CapabilityGrant、TaskRun、结构化事件、外部内容引用、显式记忆引用契约。
- 后续 AG/DE/ED/WS/REF/AST 的 L2 规则、状态机与接口定义（经 REQ_MAP 追溯）。

## 7. 路径上的空规则块

本任务为 DOC_ONLY，不修改代码；无 @GOV 块路径。受影响文档中的既有标注块均已有规则归属。

## 8. boundary 假设

- 冻结基线语义（BR-CORE-STATE-001~003、唯一 profile、澄清非终态、TaskRun 非后台队列、外部内容/记忆
  data-only 隔离）在本轮不改变；只映射为 SYS L2 规则与契约。
- 现有设计/代码与冻结基线的漂移只登记为迁移债务，不反向修改需求语义（AGENTS.md §1 范围纪律）。
- 不锁死数据库字段与实现细节；对象以职责、身份、事件真值、隔离与迁移边界描述。

## 9. 运行时假设

- D1 期间不修改 src/、src-tauri/、Prompt、工具注册、测试或 UI。
- 不注册新 TERM；TaskRun/RuntimeProfile 等共享对象术语由主责模块（AG）在 D3 L2 统一注册
  （与 AgentTurn=TERM-AG-001、IntentResult=TERM-AG-012 的既有注册归属一致），本轮登记为建议。

## 10. 疑似不变量破坏

未发现不变量破坏风险。D1 只新增/扩展 SYS L2 设计文档标注块与 REQ_MAP，不修改既有规则语义；
新 RULE/CONSTRAINT 编号经 ADUS 规则注册表与跨模块约束表查重（BR-SYS-STATE-007/008、
BR-SYS-DATA-007 无冲突；X-SHARED-002 无冲突）。

## 11. 验证计划

- 修改文档标注块（RULE/CONSTRAINT/REQ_MAP）后运行 `npm run governance:generate` 刷新 ADUS；
- 运行 `npm run governance:audit`（只读）确认无 error 级违规（TERM_UNREGISTERED / 引用断裂 / 查重冲突）；
- 检查文档头、引用关系、变更记录与 REQ_MAP 可追溯性（ADP §5 DESIGN 补充验证）。

## 12. 文件范围结论（Scope Lock 输入）

允许修改：
- `docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md`（扩展共享对象契约、规则、约束、REQ_MAP）
- `docs/01_modules/SYS/A-SYS-M-S-01_状态机总图.md`（状态机归属、跨模块转移契约、REQ_MAP）
- `.governance/issues/AG-1.0-D1-SYS-SHARED-CONTRACT-01.md`（本文件）
- `.governance/active_scope.json`（Scope Lock 镜像）

禁止修改（不属本 Scope）：任何 src/、src-tauri/、Prompt、工具注册、测试、UI 文件；其他模块 L1/L2；
产品根、能力基线、冻结记录、L1 需求文档；治理控制文档（ADU/ADP/AGENTS/APC/ADUS 手动区）；governance.config.json。
ADUS 自动生成区仅通过 `npm run governance:generate` 更新。

## 13. Task Runtime Report

- task_type：`DOC_ONLY`
- governance_phase：`DESIGN`
- final_state：`READY`
- changed_files：`docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md`（新增 §3.1 共享运行时对象契约、
  BR-SYS-STATE-007/008、BR-SYS-DATA-007、X-SHARED-002、REQ_SYS-008~012 REQ_MAP、§9.1 漂移债务、§10 D2/D3 输入）、
  `docs/01_modules/SYS/A-SYS-M-S-01_状态机总图.md`（§3.1 状态机归属、§5 三个跨模块转移契约、REQ_SYS-010/011/012
  REQ_MAP、§10 待建债务）、`docs/00_core/A-CORE-C-R-01_ADUS.md`（generate 自动更新）、本 Issue Trace、
  `.governance/active_scope.json`。
- delivered_artifact：SYS L2 共享契约设计（§3.1）+ 状态机归属与跨模块转移契约（M-S-01 §3.1/§5）+ REQ_MAP。
- mapping_chain：REQ-SYS-008/009/010/011/012 → BR-SYS-STATE-007/008、BR-SYS-DATA-007、X-SHARED-002、
  X-FORBID-008/009、X-SEQ-003 已注册；REQ_MAP 标注块已写入设计文档。生成器（generate_adus.mjs）当前不实现
  REQ/REQ_MAP 提取，ADUS 无需求映射表——与既有 REQ-SYS-002 模式一致，非本轮缺陷。
- test_impact：不适用（DOC_ONLY，无代码/测试改动）。
- validation：
  - `npm run governance:generate` # passed（规则标注 91、约束标注 18、术语 56；BR-SYS-STATE-007/008、
    BR-SYS-DATA-007、X-SHARED-002 已收录 ADUS，registry_status=registered）
  - `npm run governance:audit` # failed（两项，均属阶段一注册规则的预期状态，不属本设计错误）：
    ① "@GOV blocks and references"：3 条 OWNER_MISSING（RULE not referenced by any @GOV block）——
    新规则注册于阶段一文档治理，代码 @GOV 引用由 D2/D3 实现时产生；② "test coverage comments reference
    rules"：BR-SYS-STATE-007/008/BR-SYS-DATA-007 无 covers（测试覆盖 MISSING）——同上。其余维度全部
    passed（TERM integrity 56/32、BOUNDARY_ABSTRACT 0、BLOCK_UNANNOTATED 扫描、test governance、
    product capability，findings/warnings 为空）。
  - `git diff --check` # passed（无尾随空白/格式错误）
  - 文档头、引用关系（新规则引用的 AG-TURN/IPC-CALL/DE-DIFF/REF-INJECT/AST-MEMORY 均在链路注册表）、
    变更记录、REQ_MAP 可追溯性 # passed（ADP §5 DESIGN 补充验证）
  - changed_files 范围核对 # passed（仅 4 个允许文件 + ADUS 自动生成区；未触碰 src/、src-tauri/、Prompt、
    工具注册、测试、UI 与其他模块文档）
- automated_tests：不适用（DOC_ONLY）。
- stop_conditions：无。
- unresolved_items：见 §7 待设计项与 §9 漂移债务——audit 两项 failed 的关闭条件为 D2/D3 实现代码 @GOV
  引用新规则并补测试 covers；TaskRun/RuntimeProfile 等 TERM 注册待主责模块 D3 L2。
