# Issue Trace — AG-ADAPTIVE-RUNTIME-DESIGN-01

## 1. 问题描述

G1 已把 Agent 1.0 需求统一为“模型负责语义与自适应路线、运行时负责事实与硬边界”，但活动设计仍将关键词式 `IntentResult.allowed_tools`、静态 `TaskPlan` 调度、`TaskStep` 预授权、强制 `report_project` 和单次提案单卡片作为运行事实。若不先统一设计，后续代码会继续同时服从两套冲突机制。

## 2. 影响范围

- AG 技术设计、提示词架构、状态机与接口协议。
- SYS 共享数据结构、状态机总图与前端交互投影。
- DE 技术设计、状态机与接口协议。
- ED 受控事务、会话状态机与接口协议中同批结构化多卡的共享基线预览和局部反向语义。
- WS 资源操作技术设计、状态机与接口协议中对 AgentTurn/TaskRun 的跨模块边界。

## 3. 相关规则

原位细化既有 `BR-AG-STATE-001/002`、`BR-AG-DATA-002/003/004/005/008`、`BR-AG-OBS-001`、`BR-SYS-STATE-007/008`、`BR-SYS-DATA-007/008` 与 `BR-DE-DATA-001/002/003`、`BR-DE-STATE-001/002`；不新增或改号。

## 4. 相关链路

`AG-TURN`、`AG-TOOL`、`DE-DIFF`，以及总图中的 `SM-SYS-01 ↔ SM-AG-01/02 ↔ SM-DE-01`。

## 5. 生产者块

- 用户输入、编辑器焦点、显式引用、RuntimeProfile、格式动态能力和 Gate 结果。
- provider 输出、工具请求、ToolResult、DecisionRequest 与 Proposal 结果。

## 6. 消费者块

- PromptPackage 的当前授权工具投影。
- AgentTurn/TaskRun UI 投影、ProjectReport 和 DiffCard。
- 持久化恢复与运行时审计。

## 7. 路径上的空规则块

无。沿用既有规则与链路，新增术语仅用于消除现有结构的命名歧义。

## 8. boundary 假设

- 模型输出可以改变语义与下一步路线，但不能改变 RuntimeProfile、Gate、ExplicitEditTargetSet、格式动态能力、预算或终态。
- CapabilityGrant 只由运行时当前事实计算；IntentResult、TaskPlan、模型文本和工具发现均不构成授权源。
- ProposalBatch 只建立同次模型提案的关联，不合并各 PendingDiff 的裁决和终态。
- 同一文档同一 ProposalBatch 的结构化多卡共享生成基线，ED 聚合预检后只提交一次前向事务；此后逐卡裁决按互不重叠的目标 after fingerprint 校验，不能用整篇哈希使批内兄弟卡互相失效。

## 9. 运行时假设

- 回合事实追加为不可变 TurnEvent；UI、TaskRun/TaskPlan、ProjectReport 和恢复状态均从事件及持久化业务事实重建。
- 简单回合不创建 TaskRun；复杂任务投影只描述已发生和当前候选路线，不自动启动未来步骤。
- 每次 provider 调用前重新 Gate；每次工具执行前重新计算并校验 CapabilityGrant；等待期间无后台执行。

## 10. 疑似不变量破坏

- 语义投影持有执行权限，导致关键词误判直接扩权或拒绝。
- 计划/步骤投影反向驱动执行，导致静态路线与真实证据分叉。
- 自报式 ProjectReport 与真实读取覆盖分叉。
- 同一提案的不连续修改强绑为一个终态，无法独立裁决。

## 11. 验证计划

- 检索所有活动设计中的旧权威表述并消解。
- 对设计术语、规则、协议和状态机做双向一致性审计。
- 运行 governance:generate、governance:audit、test:plan/test:related 与 diff check。

## Expected Behavior

活动设计只保留一套可实现事实：模型在运行时硬边界内基于真实工具结果调整语义和路线；事件是回合事实源，计划/报告是可重建投影；同批多块修改按独立裁决单元生成多个 DiffCard。

## Validation Commands

- `npm run governance:generate`
- `npm run governance:audit -- --issue .governance/issues/AG-ADAPTIVE-RUNTIME-DESIGN-01.md`
- `npm run test:plan -- --files [...]`
- `npm run test:related -- --files [...]`
- `git diff --check`

## Scope Lock

```yaml
task_type: GOVERNANCE_CHANGE
governance_phase: DESIGN
allowed_files:
  - .governance/active_scope.json
  - .governance/issues/AG-ADAPTIVE-RUNTIME-DESIGN-01.md
  - docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md
  - docs/01_modules/AG/A-AG-M-T-02_提示词架构设计.md
  - docs/01_modules/AG/A-AG-M-S-01_代理状态机设计.md
  - docs/01_modules/AG/A-AG-I-P-01_代理接口协议.md
  - docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md
  - docs/01_modules/SYS/A-SYS-M-S-01_状态机总图.md
  - docs/01_modules/SYS/A-SYS-C-T-02_前端交互架构与流程总图.md
  - docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md
  - docs/01_modules/DE/A-DE-M-S-01_DiffEdit状态机设计.md
  - docs/01_modules/DE/A-DE-I-P-01_DiffEdit接口协议.md
  - docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md
  - docs/01_modules/ED/A-ED-M-S-01_编辑器状态机设计.md
  - docs/01_modules/ED/A-ED-I-P-01_编辑器接口协议.md
  - docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md
  - docs/01_modules/WS/A-WS-M-S-02_工作区资源操作状态机设计.md
  - docs/01_modules/WS/A-WS-I-P-01_工作台接口协议.md
  - docs/00_core/A-CORE-C-R-01_ADUS.md
forbidden_files:
  - L1 需求、开发计划、验证文档、L4 参考文档、上述清单外的 L2 设计与生产代码
confirmation: YES
scope_reason: 承接已确认的 C1-C5 与已 READY 的 G1 需求，统一现有设计而不扩展产品能力。
```

## 人类决策

- C1：模型主导语义理解、工具选择和路线调整；运行时主导事实、授权、预算、门禁与副作用。
- C2：IntentResult 是可更新语义投影，不是工具授权真源。
- C3：TaskRun/TaskPlan 是复杂任务可观察投影，不是静态调度器。
- C4：ProjectReport 从 EvidenceLedger 派生，不依赖模型调用报告工具。
- C5：DiffCard 按可独立裁决的连续块组拆分，同批关联而逐卡裁决。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-09-03 | v1.0 | 建立 Agent 1.0 自适应运行时设计收敛 Scope。 |
| 2026-09-03 | v1.1 | 设计语义扫描发现 WS 资源操作仍将 TaskRun 写成执行编排者、SYS 前端仍按单 DiffProjection 投影；将对应活动设计纳入同一 Scope 消除跨模块冲突，不扩展能力。 |
| 2026-09-03 | v1.2 | C5 深度对账发现逐卡预览会使同批兄弟卡互相推进整篇基准；将 ED 既有受控事务设计纳入 Scope，闭合同批共享基线、一次预览和目标局部裁决语义，不新增接口能力。 |
| 2026-09-03 | v1.3 | 完成 AG/SYS/DE/ED/WS 设计、协议、状态机与前端流程归一；登记 3 个消歧术语并通过治理生成、治理审计、影响测试和完整 Rust 环境复跑。 |

## Task Runtime Report — Agent 1.0 自适应运行时设计与协议收敛

task_type: GOVERNANCE_CHANGE
governance_phase: DESIGN
final_state: READY
issue_trace: AG-ADAPTIVE-RUNTIME-DESIGN-01

changed_files:
  - `.governance/active_scope.json` # 更新显式 Scope 并进入 READY
  - `.governance/issues/AG-ADAPTIVE-RUNTIME-DESIGN-01.md` # 建立并闭合 Issue Trace
  - `docs/00_core/A-CORE-C-R-01_ADUS.md` # 由 governance:generate 刷新派生索引
  - `docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md` # 归一自适应回合、授权、事件、证据与批次设计
  - `docs/01_modules/AG/A-AG-M-T-02_提示词架构设计.md` # 归一模型语义责任和逐调用 Prompt 装配
  - `docs/01_modules/AG/A-AG-M-S-01_代理状态机设计.md` # 归一 AgentTurn、工具循环与任务投影状态
  - `docs/01_modules/AG/A-AG-I-P-01_代理接口协议.md` # 归一内部事件、授权、证据与提案批协议
  - `docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md` # 归一共享对象和 DiffProjection 批次字段
  - `docs/01_modules/SYS/A-SYS-M-S-01_状态机总图.md` # 归一跨模块状态机边界
  - `docs/01_modules/SYS/A-SYS-C-T-02_前端交互架构与流程总图.md` # 归一事件消费和同批多卡投影
  - `docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md` # 定义 ProposalBatch、多卡及独立裁决
  - `docs/01_modules/DE/A-DE-M-S-01_DiffEdit状态机设计.md` # 定义批内卡生命周期与局部失效
  - `docs/01_modules/DE/A-DE-I-P-01_DiffEdit接口协议.md` # 定义同批一个或多个 PendingDiff 契约
  - `docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md` # 定义同文档同批一次前向事务与局部反向
  - `docs/01_modules/ED/A-ED-M-S-01_编辑器状态机设计.md` # 定义批内预览和逐卡裁决状态语义
  - `docs/01_modules/ED/A-ED-I-P-01_编辑器接口协议.md` # 定义批预览与目标局部逆操作边界
  - `docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md` # 移除 TaskPlan 调度权威
  - `docs/01_modules/WS/A-WS-M-S-02_工作区资源操作状态机设计.md` # 归一确认后同回合重验入口
  - `docs/01_modules/WS/A-WS-I-P-01_工作台接口协议.md` # 归一资源操作调用责任

scope:
  mode: explicit
  status: confirmed；19 个 changed_files 均在 Scope Lock 内

phase_gate:
  status: passed
  evidence: G1 已 READY，C1-C5 已由人类确认；本阶段只统一既有需求对应设计，不修改生产代码或增加产品能力

mapping_chain:
  status: complete
  evidence: 新增 TERM-AG-022/023 与 TERM-DE-005 消除既有运行对象歧义；REQ 仍为 142、REQ_MAP 仍为 195、RULE 仍为 100、CHAIN 仍为 27，相关设计块由既有规则和链路继续覆盖

test_impact:
  changed_files: 上述 19 个文件
  direct_modules: AG / SYS / DE / ED / WS / CORE
  affected_modules: 全模块（关键共享设计风险扩展）
  selection: frontend-full / rust-full / governance-audit

validation:
  - `npm run governance:generate` # passed；生成 313 个 @GOV、16 个模块、142 个 REQ、195 个 REQ_MAP、100 个 RULE、27 个 CHAIN、18 个 CONSTRAINT、75 个 TERM、16 台状态机、106 个测试资产、962 个测试用例
  - `npm run governance:audit -- --issue .governance/issues/AG-ADAPTIVE-RUNTIME-DESIGN-01.md` # passed；无治理 finding，Scope Lock 通过
  - `npm run test:plan -- --files [...]` # passed；选择 npm test、完整 cargo test 与 governance:audit
  - `npm run test:related -- --files [...]` # frontend passed，26 files / 595 tests；沙箱内 Rust 仅 LibreOffice worker 因 signal 6 失败
  - `doc::libreoffice_runtime::tests::docx_to_pdf_end_to_end_when_libreoffice_available`（沙箱外） # passed；确认沙箱进程环境是单点失败原因
  - `cargo test --manifest-path src-tauri/Cargo.toml`（沙箱外完整复跑） # passed；374 unit + 5 core_chain + 4 sm_broadcast，0 failed
  - `git diff --check` # passed
  - 系统行为验收 # not_applicable；本 Scope 只收敛设计，生产实现差异由后续 IMPLEMENTATION/VALIDATION Scope 修复和验证

stop_conditions:
  - SC-04 曾由沙箱内 LibreOffice worker 崩溃触发；未修改代码或测试，以相同用例和完整套件在真实运行环境通过后解除

recovery:
  - 未弱化、跳过或删除测试；通过执行环境对照定位，并完成完整 Rust 套件复跑

unresolved_items:
  - 无；生产实现尚未对齐属于已规划的后续阶段，不作为本设计 Scope 的遗留缺陷
