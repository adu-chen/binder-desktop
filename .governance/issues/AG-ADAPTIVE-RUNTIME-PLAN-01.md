# Issue Trace — AG-ADAPTIVE-RUNTIME-PLAN-01

## 1. 问题描述

G1/G2 已把 Agent 1.0 收敛为模型主导语义与路线、运行时主导事实和硬边界，但当前 Active 计划与验证材料仍按旧实现拆分工作：IntentResult 预先产出工具集、TaskPlan 驱动片段、模型调用 `report_project` 完成报告、连续卡片相互失效。若直接进入代码阶段，任务拆分和验收会反向覆盖唯一设计。

## 2. 影响范围

- Agent 1.0 唯一 Active 开发与验证计划。
- Agent 1.0 唯一 Active 验证卡与证据状态入口。
- TaskRun 真实环境手动测试材料。

## 3. 相关规则

只消费既有 `BR-AG-STATE-001/002`、`BR-AG-DATA-002/003/004/005/008`、`BR-AG-OBS-001`、`BR-SYS-STATE-007/008`、`BR-SYS-DATA-007/008`、`BR-DE-DATA-001/002/003`、`BR-DE-STATE-001/002`；不修改静态定义。

## 4. 相关链路

`AG-TURN`、`AG-TOOL`、`DE-DIFF`、`ED-EDIT`、`WS-OPERATION`。

## 5. 生产者块

- 已 READY 的 Agent 1.0 需求、技术设计、提示词架构、接口协议和状态机。
- 当前生产代码与测试资产只作为差距和工作量输入，不作为计划语义来源。

## 6. 消费者块

- 后续 WP1-WP9 独立 CODE_CHANGE Scope。
- changed_files 测试选择、Agent 验证卡状态和 G5 发布裁决。

## 7. 路径上的空规则块

无。本 Scope 不新增规则、链路或术语。

## 8. boundary 假设

- 计划只规定实施顺序和证据，不拥有需求、运行时状态或工具授权。
- 历史测试通过不能证明已改变语义的当前实现通过。
- 删除冗余实现必须以当前设计和等价/替代测试为依据，不得为了通过检查删除或弱化断言。

## 9. 运行时假设

- 每个工作包实施前重新核对实际代码、直接消费者、测试注册表与 ADUS。
- 每个工作包独立建 Scope、独立验证和提交；失败回到产生根因的最近工作包。

## 10. 疑似不变量破坏

- 旧计划使 IntentResult 和 TaskPlan 重新获得授权或调度职责。
- 旧验证状态把历史自动化误当成当前契约证据。
- 旧手动步骤要求同批 DiffCard 互相失效，直接否定独立裁决目标。

## 11. 验证计划

- 对计划、验证卡和手动卡做逐项设计映射审计。
- 检索活动正文中的 `IntentResult.allowed_tools`、`report_project`、静态执行游标、自动片段调度与批内失效残留。
- 运行 governance:generate、governance:audit、test:plan/test:related 与 diff check。

## Expected Behavior

后续实现只有一条执行路线：先建立事件事实源，再收敛自适应模型循环、动态授权、上下文/证据、任务投影、ProposalBatch、UI/恢复，最后删除旧权威和做 G5；每一项都由当前验证卡而非历史结论裁决。

## Validation Commands

- `npm run governance:generate`
- `npm run governance:audit -- --issue .governance/issues/AG-ADAPTIVE-RUNTIME-PLAN-01.md`
- `npm run test:plan -- --files [...]`
- `npm run test:related -- --files [...]`
- `git diff --check`

## Scope Lock

```yaml
task_type: GOVERNANCE_CHANGE
governance_phase: PLAN
allowed_files:
  - .governance/active_scope.json
  - .governance/issues/AG-ADAPTIVE-RUNTIME-PLAN-01.md
  - docs/01_modules/AG/A-AG-X-L-04_代理1.0开发与验证计划.md
  - docs/40_validation/A-VAL-X-V-04_Agent1.0验证卡.md
  - docs/40_validation/A-VAL-X-V-05_TaskRun手动测试卡.md
  - docs/00_core/A-CORE-C-R-01_ADUS.md
forbidden_files:
  - L1/L2 需求与设计、生产代码、测试代码、上述清单外的计划和验证文档
confirmation: YES
scope_reason: 只将已确认需求和设计翻译为唯一实施与验证基线。
```

## 人类决策

- 按推荐方案完整执行；每个阶段进行进度与完成度自检。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-09-03 | v1.0 | 建立 Agent 1.0 实施与验证计划收敛 Scope。 |
| 2026-09-03 | v1.1 | 完成 WP1～09 唯一实施路线、V-C/V-R/V-S1～S14 当前验证卡与 TaskRun 投影手动卡；旧 PARTIAL 证据降为历史素材。 |

## Task Runtime Report — Agent 1.0 实施与验证计划收敛

task_type: GOVERNANCE_CHANGE
governance_phase: PLAN
final_state: READY
issue_trace: AG-ADAPTIVE-RUNTIME-PLAN-01

changed_files:
  - `.governance/active_scope.json` # 建立显式 Scope 并进入 READY
  - `.governance/issues/AG-ADAPTIVE-RUNTIME-PLAN-01.md` # 建立并闭合 Issue Trace
  - `docs/01_modules/AG/A-AG-X-L-04_代理1.0开发与验证计划.md` # 重建 WP1～09 唯一实施路线、删除项和阶段门禁
  - `docs/40_validation/A-VAL-X-V-04_Agent1.0验证卡.md` # 重建当前验证卡、证据状态和工作包映射
  - `docs/40_validation/A-VAL-X-V-05_TaskRun手动测试卡.md` # 重写 TaskRun 投影、同回合裁决和无后台执行手动场景

scope:
  mode: explicit
  status: confirmed；5 个 changed_files 均在 Scope Lock 内

phase_gate:
  status: passed
  evidence: G1/G2 已 READY；本阶段只将当前需求和设计翻译为实施与验证控制面，不修改 L1/L2 或生产代码

mapping_chain:
  status: complete
  evidence: WP1～09 均映射上游对象和 V-C/V-R/V-S 卡；未新增 REQ/RULE/CHAIN/TERM/状态机，生成计数保持 142/195/100/27/75/16

test_impact:
  changed_files: 上述 5 个文件
  direct_modules: AG / CORE
  affected_modules: AG / CORE / SYS
  selection: 11 个相关前端套件 / rust-full / governance-audit

validation:
  - `npm run governance:generate` # passed；ADUS 内容无变化，计数保持 313 @GOV、16 模块、142 REQ、195 REQ_MAP、100 RULE、27 CHAIN、18 CONSTRAINT、75 TERM、16 状态机、106 测试资产、962 测试用例
  - 活动计划语义扫描 # passed；旧关键词授权、report_project、静态计划执行和批内失效只剩明确禁止/删除项或历史记录
  - `npm run governance:audit -- --issue .governance/issues/AG-ADAPTIVE-RUNTIME-PLAN-01.md` # passed；无治理 finding，Scope Lock 通过
  - `npm run test:plan -- --files [...]` # passed；选择 AG/CORE/SYS 相关前端、完整 Rust 和治理审计
  - `npm run test:related -- --files [...]` # 前端 passed，11 files / 295 tests；沙箱内 Rust 373/374，唯一失败为已知 LibreOffice worker signal 6
  - `cargo test --manifest-path src-tauri/Cargo.toml`（沙箱外完整复跑） # passed；374 unit + 5 core_chain + 4 sm_broadcast，0 failed
  - `git diff --check` # passed
  - 系统行为验收 # not_applicable；本 Scope 只建立实施和验证基线，V-C/V-R/V-S 卡均保持 DESIGNED

stop_conditions:
  - SC-04 曾由沙箱内 LibreOffice worker 崩溃触发；未修改代码或测试，以相同完整命令在真实 macOS 环境通过后解除

recovery:
  - 未借用上一阶段结果、未跳过失败测试；在本提交状态完整复跑 Rust 并通过

unresolved_items:
  - 无；所有实现和系统验证工作已显式进入 WP1～09/G5，不作为本 PLAN Scope 的隐性完成项
