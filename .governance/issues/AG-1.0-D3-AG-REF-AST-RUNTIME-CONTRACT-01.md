# Issue Trace — AG-1.0-D3-AG-REF-AST-RUNTIME-CONTRACT-01：Agent 1.0 AG/REF/AST 运行时契约与状态机 DESIGN 同步

文档状态：A
任务类型：DOC_ONLY
治理阶段：DESIGN
创建时间：2026-08-14

## 1. 用户可见症状

冻结基线（A-CORE-X-D-02 §6 第 3 项）要求 AG+REF+AST 完成「IntentResult/CapabilityGrant、受限工具消费、
引用快照、外部内容暂存/清除、显式记忆选择和 Trace 契约」。现有 AG/REF/AST L2 存在四处冻结语义缺口：
① SM-AG-01 将澄清终结为 Completed，与「澄清是等待用户输入的非终态」冲突（R5-A）；② 无 TaskRun/步骤状态机
（R5-B）；③ REF 外部文件引用仅元数据、无 ExternalContentRef 契约（R5-C）；④ AST 自动写入语义与「1.0
不自动写入、仅显式选择进入回合」冲突（R5-C）。RuntimeProfile/CapabilityGrant/TaskRun 等核心对象尚无
正式 TERM。

## 2. 入口点

冻结记录 `A-CORE-X-D-02_Agent1.0冻结能力基线.md` §6 第 3 项与 `A-CORE-X-D-01` §6 第 3 步：AG+REF+AST
完成运行时契约与状态机；D1（SYS）§10 第 5 项的 D3 设计输入。

## 3. 相关规则

现有（registered）：BR-CORE-STATE-001~003、BR-SYS-STATE-007/008、BR-SYS-DATA-007（D1 注册）、
BR-AG-STATE-001/002、BR-AG-VERIFY-001、BR-AG-DATA-001~007、BR-AG-PERSIST-001、BR-REF-DATA-001~004、
BR-REF-VERIFY-001、BR-AST-DATA-001/002、BR-AST-STATE-001。
D3 复用既有规则，不新增 RULE（冻结语义由 BR-SYS-STATE-007/008 与 BR-SYS-DATA-007 承接）。

## 4. 相关链路

AG-TURN（回合生命周期）、AG-TOOL（CapabilityGrant 消费）、DE-DIFF（Proposal 接力）、REF-INJECT
（引用与外部内容）、AST-MEMORY（显式记忆选择）、IPC-CALL（StructuredVisibleEvent 传递）。

## 5. 生产者块

- `docs/01_modules/AG/A-AG-M-S-01_代理状态机设计.md`（SM-AG-01 澄清契约、SM-AG-04 TaskRun 机）
- `docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md`（TERM-AG-013~016、Runtime/Intent/Grant 契约）
- `docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md`（TERM-REF-005、ExternalContentRef 契约）
- `docs/01_modules/AST/A-AST-M-T-01_记忆协同技术设计.md`（TERM-AST-002、MemorySelectionRef 契约）
- `docs/00_core/A-CORE-X-D-02_Agent1.0冻结能力基线.md`（需求输入，不改动）

## 6. 消费者块

- G5 跨模块联调：Runtime 门禁→回合→TaskRun→Proposal 三链；外部内容/显式记忆的回合内消费。
- 各模块实现阶段 @GOV 与测试 covers。
- D1 的 A-SYS-I-P-01 §3.1 共享对象契约的 AG/REF/AST 落地。

## 7. 路径上的空规则块

本任务为 DOC_ONLY，不修改代码；无 @GOV 块路径。

## 8. boundary 假设

- 冻结语义（澄清非终态、TaskRun 非后台队列、外部内容/记忆 data-only 隔离、不自动写入）本轮不改变，
  只映射为 AG/REF/AST L2 契约与状态机。
- 现有 SM-AG-01 主转移表不改写，以契约小节承接澄清非终态语义并登记漂移债务；SM-AG-04 为新增状态机。
- TERM 注册按 ADU §3.2 由主责模块执行（RuntimeProfile/CapabilityGrant/TaskRun/TaskStep→AG，
  ExternalContentRef→REF，MemorySelectionRef→AST），不与既有 TERM 冲突。

## 9. 运行时假设

- D3 期间不修改 src/、src-tauri/、Prompt、工具注册、测试或 UI。
- 不新增 RULE/CHAIN/CONSTRAINT；新增 TERM（6 个）与状态机（SM-AG-04）。
- AG-I-P-01 的既有接口形状（runtime_profile、IntentResultV1、CapabilityGrantV1）不改，只补契约确认。

## 10. 疑似不变量破坏

未发现不变量破坏风险。TERM 注册经 ADUS 术语注册表查重（TERM-AG-013~016、TERM-REF-005、TERM-AST-002
均无冲突）；SM-AG-04 为新增 machine_id（SM-AG-01~03 已占用，04 空闲）；REQ_MAP target 均引用已注册规则。

## 11. 验证计划

- 修改文档标注块（TERM/STATE_MACHINE/REQ_MAP）后运行 `npm run governance:generate` 刷新 ADUS；
- 运行 `npm run governance:audit`（只读）核对 TERM_DUPLICATE / TERM_UNREGISTERED / 引用断裂；
- 检查文档头、引用关系、变更记录与 REQ_MAP 可追溯性。

## 12. 文件范围结论（Scope Lock 输入）

允许修改：
- `docs/01_modules/AG/A-AG-M-S-01_代理状态机设计.md`
- `docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md`
- `docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md`
- `docs/01_modules/AST/A-AST-M-T-01_记忆协同技术设计.md`
- `.governance/issues/AG-1.0-D3-AG-REF-AST-RUNTIME-CONTRACT-01.md`（本文件）
- `.governance/active_scope.json`

禁止修改（不属本 Scope）：任何 src/、src-tauri/、Prompt、工具注册、测试、UI 文件；其他模块 L1/L2；
AG/REF/AST 的 M-D/I-P 文档（接口形状不改）；产品根、能力基线、冻结记录；治理控制文档；governance.config.json。
ADUS 自动生成区仅通过 `npm run governance:generate` 更新。

## 13. Task Runtime Report

- task_type：`DOC_ONLY`
- governance_phase：`DESIGN`
- final_state：`READY`
- changed_files：
  - `docs/01_modules/AG/A-AG-M-S-01_代理状态机设计.md`（§2.1 澄清非终态契约、§2.2 SM-AG-04 TaskRun 机、REQ-AG-007/015 REQ_MAP）
  - `docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md`（TERM-AG-013~016、§7.1 运行时契约落地）
  - `docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md`（TERM-REF-005、§6.1 外部内容引用契约、REQ-REF-009 REQ_MAP）
  - `docs/01_modules/AST/A-AST-M-T-01_记忆协同技术设计.md`（TERM-AST-002、§4.1 显式记忆选择契约、REQ-AST-001 REQ_MAP）
  - `docs/00_core/A-CORE-C-R-01_ADUS.md`（generate 更新，术语 56→62）
  - 本 Issue Trace、`.governance/active_scope.json`
- delivered_artifact：AG/REF/AST 运行时契约（澄清非终态、SM-AG-04 TaskRun 机、RuntimeProfile/CapabilityGrant/IntentResult/StructuredVisibleEvent 落地、ExternalContentRef、MemorySelectionRef）+ 6 个 TERM。
- mapping_chain：REQ_MAP target 全部为已注册规则（BR-SYS-STATE-007/008、BR-SYS-DATA-007、BR-CORE-STATE-001）；不新增 RULE/CHAIN/CONSTRAINT。
- test_impact：不适用（DOC_ONLY，无代码/测试改动）。
- validation：
  - `npm run governance:generate` # passed（术语标注 56→62，新增 6 TERM 全部收录 ADUS）
  - `npm run governance:audit` # failed（两项，与 D1/D2 后一致，无新增）：BR-SYS-STATE-007/008、BR-SYS-DATA-007 无代码 @GOV 引用与测试 covers（阶段一注册规则，待实现）；TERM integrity # passed（62/32，无 UNREGISTERED/DUPLICATE）；其余维度全部 passed
  - `git diff --check` # passed
  - 文档头、引用关系、变更记录、REQ_MAP 可追溯性 # passed（ADP §5 DESIGN 补充验证）
  - changed_files 范围核对 # passed（仅 4 个 L2 文档 + 治理记录 + ADUS 自动区）
- automated_tests：不适用（DOC_ONLY）。
- stop_conditions：无。
- unresolved_items：SM-AG-04 建机后须回填 A-SYS-M-S-01 §3.1（TaskRun planned→active，SYS 文档不在本 Scope，作为后续登记）；澄清非终态的现有代码迁移（clarificationRequired→Completed）与 AST 自动写入关闭由实现 Scope 承接；StructuredVisibleEvent 字段形状由前端设计承接。
