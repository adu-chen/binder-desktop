# Issue Trace — AG-1.0-D2-DE-ED-WS-STATEMACHINE-01：Agent 1.0 三态分流/裁决/连续 Proposal/保存状态机与接口 DESIGN 同步

文档状态：A
任务类型：DOC_ONLY
治理阶段：DESIGN
创建时间：2026-08-14

## 1. 用户可见症状

冻结基线（A-CORE-X-D-02 §6 第 2 项）要求 DE+ED+WS 完成「三态分流 / 预览 / 裁决 / 基准推进失效 /
连续 Proposal / 显式保存 / 失败与关闭」的状态机和接口契约。现有 DE/ED/WS 状态机已覆盖三分流基本路径，
但未显式建模三处冻结语义：① 连续 Proposal 必须以最新 LogicalState 接力、前卡基准推进失效且不形成补丁栈
（REQ-DE-009 / BR-SYS-STATE-008）；② 未保存新建文档与转换目标的预览开标签、接受只裁决、拒绝丢弃、
保存才建文件（REQ-ED-010/014 / D-NEW-001~008）；③ 保存有效 pending 预览先裁决为接受、以及未保存新文档
的保存即创建文件（REQ-WS-003/006）。三个 M-S 亦无 REQ_MAP 建立需求追溯。

## 2. 入口点

冻结记录 `A-CORE-X-D-02_Agent1.0冻结能力基线.md` §6 第 2 项与 `A-CORE-X-D-01_1.0需求基线与文档体系索引.md`
§6 第 2 步：DE+ED+WS 完成预览/裁决/基准推进失效/连续 Proposal 接力/显式保存/保存失败/关闭的状态机与接口契约。

## 3. 相关规则

现有（registered）：
- BR-CORE-STATE-001（已有 LogicalState 预览/接受/保存）、BR-CORE-STATE-002（无 LogicalState 挂载/接受落盘）、
  BR-CORE-STATE-003（pending 后打开迁移）
- BR-SYS-STATE-007（运行时事件真值）、BR-SYS-STATE-008（TaskRun 编排与 Proposal 接力，D1 注册）、
  BR-SYS-DATA-007（外部内容/记忆隔离）、BR-SYS-PERSIST-001（schema 版本化迁移）
- BR-DE-STATE-001/002、BR-DE-DATA-001/003、BR-ED-STATE-001、BR-WS-STATE-001、BR-WS-DATA-002、BR-SYS-DATA-005

D2 复用既有规则，不新增 RULE（避免无代码引用）。

## 4. 相关链路

DE-DIFF（Proposal 链）→ ED-EDIT（LogicalState 承接）→ WS-SAVE / WS-DIFF-APPLY（三分流落盘）；
AG-TURN（连续 Proposal 的回合/任务关联）；DATA-MIGRATE（未保存新文档持久化边界）。

## 5. 生产者块

- `docs/01_modules/DE/A-DE-M-S-01_DiffEdit状态机设计.md`（本轮扩展：连续 Proposal 接力契约 + REQ_MAP）
- `docs/01_modules/ED/A-ED-M-S-01_编辑器状态机设计.md`（本轮扩展：未保存新建/转换目标会话承接 + REQ_MAP）
- `docs/01_modules/WS/A-WS-M-S-01_工作台状态机设计.md`（本轮扩展：保存协调契约 + 未保存新文档保存 + REQ_MAP）
- `docs/00_core/A-CORE-X-D-02_Agent1.0冻结能力基线.md`（需求输入，不改动）
- `docs/00_core/A-CORE-C-T-01_三态模型.md`（BR-CORE-STATE-001~003，不改动）

## 6. 消费者块

- D3（AG/REF/AST）：连续 Proposal 的 TaskRun/回合关联、StructuredVisibleEvent 消费三机语义。
- 后续 G5 跨模块联调：AG→DE→ED/WS 三链状态机对接验证。
- 各模块实现阶段的 @GOV 与测试 covers（经 REQ_MAP 追溯）。

## 7. 路径上的空规则块

本任务为 DOC_ONLY，不修改代码；无 @GOV 块路径。三个 M-S 的 STATE_MACHINE 块均已登记 rules。

## 8. boundary 假设

- 冻结基线三分流（BR-CORE-STATE-001~003）、连续 Proposal 接力（BR-SYS-STATE-008）、保存即接受仍有效
  pending 预览、未保存新文档保存才建文件——本轮语义不改变，只映射为三机契约与 REQ_MAP。
- 现有 DE/ED/WS 状态机主转移表不改写；以新增契约小节承接冻结语义，避免破坏已闭合的转移矩阵。
- 不锁接口字段；只定义状态机事件语义与跨模块协调契约。

## 9. 运行时假设

- D2 期间不修改 src/、src-tauri/、Prompt、工具注册、测试或 UI。
- 不新增 RULE/CHAIN/CONSTRAINT/TERM；复用 D1 已注册的 BR-SYS-STATE-007/008、BR-SYS-DATA-007 等。
- 转换目标、多文档逐项、删除/恢复等 P1 专项状态机不在本 Scope（属各自 DESIGN/实现），只登记语义承接点。

## 10. 疑似不变量破坏

未发现不变量破坏风险。D2 只扩展三个 M-S 设计文档的契约小节并补 REQ_MAP，不修改既有规则与转移语义；
REQ_MAP 的 target 均引用已注册规则（BR-CORE-STATE-001~003、BR-SYS-STATE-007/008、BR-SYS-DATA-007、
BR-SYS-PERSIST-001），不新增未注册引用。

## 11. 验证计划

- 修改文档后运行 `npm run governance:generate` 刷新 ADUS（本任务不新增标注块类型，generate 应保持通过）；
- 运行 `npm run governance:audit`（只读）核对引用断裂/TERM 冲突（新增 REQ_MAP 不触发既有规则 OWNER_MISSING 增量）；
- 检查文档头、引用关系、变更记录与 REQ_MAP 可追溯性（ADP §5 DESIGN 补充验证）。

## 12. 文件范围结论（Scope Lock 输入）

允许修改：
- `docs/01_modules/DE/A-DE-M-S-01_DiffEdit状态机设计.md`
- `docs/01_modules/ED/A-ED-M-S-01_编辑器状态机设计.md`
- `docs/01_modules/WS/A-WS-M-S-01_工作台状态机设计.md`
- `.governance/issues/AG-1.0-D2-DE-ED-WS-STATEMACHINE-01.md`（本文件）
- `.governance/active_scope.json`（Scope Lock 镜像）

禁止修改（不属本 Scope）：任何 src/、src-tauri/、Prompt、工具注册、测试、UI 文件；其他模块 L1/L2；
三模块的 M-T/I-P 文档（除非其 REQ_MAP 归属本文，另评）；产品根、能力基线、冻结记录；治理控制文档；
governance.config.json。ADUS 自动生成区仅通过 `npm run governance:generate` 更新。

## 13. Task Runtime Report

- task_type：`DOC_ONLY`
- governance_phase：`DESIGN`
- final_state：`READY`
- changed_files：
  - `docs/01_modules/DE/A-DE-M-S-01_DiffEdit状态机设计.md`（新增 §2.1 连续 Proposal 接力契约 + REQ-DE-009 REQ_MAP）
  - `docs/01_modules/ED/A-ED-M-S-01_编辑器状态机设计.md`（新增 §2.1 未保存新建/转换目标会话承接 + REQ-ED-010/014 REQ_MAP）
  - `docs/01_modules/WS/A-WS-M-S-01_工作台状态机设计.md`（新增 §2.1 保存协调与未保存新文档保存契约 + REQ-WS-003/006 REQ_MAP）
  - `docs/00_core/A-CORE-C-R-01_ADUS.md`（generate 无变化跳过写入）
  - 本 Issue Trace、`.governance/active_scope.json`
- delivered_artifact：三模块 M-S 的冻结语义契约小节 + REQ_MAP（DE/ED/WS 各补 REQ-DE-009、REQ-ED-010/014、REQ-WS-003/006）。
- mapping_chain：REQ_MAP 的 target 全部为已注册规则（BR-CORE-STATE-001~003、BR-SYS-STATE-008、BR-SYS-PERSIST-001）；不新增 RULE/CHAIN/CONSTRAINT/TERM。
- test_impact：不适用（DOC_ONLY，无代码/测试改动）。
- validation：
  - `npm run governance:generate` # passed（ADUS 内容无变化跳过写入；规则 91、约束 18 与 D1 后一致）
  - `npm run governance:audit` # failed（两项，与 D1 后完全一致，无新增）：
    ① "@GOV blocks and references"：3 条 OWNER_MISSING（BR-SYS-STATE-007/008、BR-SYS-DATA-007 无代码 @GOV 引用——阶段一注册规则，待 D3/实现引用）；
    ② "test coverage comments reference rules"：同上 3 条无 covers（MISSING）。其余维度全部 passed。
  - `git diff --check` # passed（无尾随空白/格式错误）
  - 文档头、引用关系（REQ_MAP target 均在规则注册表）、变更记录 # passed（ADP §5 DESIGN 补充验证）
  - changed_files 范围核对 # passed（仅 3 个 M-S + 治理记录 + ADUS 自动区；未触碰 src/、src-tauri/、Prompt、工具注册、测试、UI、其他模块文档）
- automated_tests：不适用（DOC_ONLY）。
- stop_conditions：无。
- unresolved_items：audit 两项 failed 的关闭条件 = D2 契约落地实现后由代码 @GOV 引用与测试 covers；连续 Proposal 的 TaskRun 关联字段、未保存新文档元数据、转换/多文档/删除 P1 专项状态机待 D3 与各自开放时建模。
