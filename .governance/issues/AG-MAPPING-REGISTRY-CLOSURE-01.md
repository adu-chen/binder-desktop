# Issue Trace — AG-MAPPING-REGISTRY-CLOSURE-01：Agent 需求映射与 ADUS 登记闭环

文档状态：A
任务类型：CODE_CHANGE
治理阶段：VALIDATION
任务状态：SCOPED
创建时间：2026-09-01
上游基线：ADU §3.2、APC §2/§5/§8、A-AG-M-D-01、A-AG-M-T-01、A-AG-M-S-01。

## 1. 用户可见症状

当前 Agent Active 设计已经定义规则、链路、术语和四个状态机，但 ADUS 只登记 RULE/CHAIN/CONSTRAINT/TERM，不登记 REQ、REQ_MAP、STATE_MACHINE。四条 DESIGN_PENDING 需求仍标为 active 且没有正式映射处置，因此治理审计通过也不能证明需求到设计链闭合。

## 2. 入口点

- `scripts/generate_adus.mjs::parseAnnotationBlocks/main`
- `scripts/audit_taskflow_governance.mjs::parseAnnotationBlocks`
- `docs/01_modules/AG/A-AG-M-D-01_代理功能需求描述.md` 的 REQ 注册块
- `docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md` 的 REQ_MAP 注册块

## 3. 相关规则

- Agent 已设计规则：BR-AG-STATE-001～004、BR-AG-DATA-001～008、BR-AG-VERIFY-001、BR-AG-PERSIST-001、BR-AG-UI-001、BR-AG-OBS-001。
- Agent 映射涉及约束：X-FORBID-006、X-SHARED-001、X-SHARED-002。
- 本轮不新增业务 RULE；映射闭环由 ADU §3.2 与 APC §8 直接约束。

## 4. 相关链路

`REQ registry → REQ_MAP registry → RULE/CHAIN/CONSTRAINT registry → STATE_MACHINE registry → ADUS`

涉及既有业务链：AG-TURN、AG-TOOL、AG-ASSIST、AG-MODEL-CATALOG、WS-OPERATION。

## 5. 生产者块

- `scripts/generate_adus.mjs::parseAnnotationBlocks` 生产 ADUS 注册项。
- Active 需求、设计和状态机文档中的 `<!-- REQ/REQ_MAP/STATE_MACHINE -->` 块生产追溯事实。

## 6. 消费者块

- `scripts/generate_adus.mjs::main` 消费标注块并渲染 ADUS。
- `scripts/audit_taskflow_governance.mjs` 消费同类标注块并执行失败关闭。
- ADP Phase Gate、Issue Trace 和人工审计消费 ADUS 映射索引。

## 7. 路径上的空规则块

治理生成器和审计器不属于业务 @GOV 主链；本轮不新增或修改 `rules: []` 业务块，无 TRIAGE_REQUIRED。

## 8. boundary 假设

- REQ 仍是需求事实，不能直接成为代码规则来源。
- 审计必须输出分模块 active REQ 映射覆盖；Agent 本轮验收要求所有 active REQ 至少存在一条 mapped REQ_MAP，deferred/blocked/candidate/superseded REQ 以自身状态显式闭合，不伪造设计规则。
- mapped REQ_MAP 的 target 必须存在且类型匹配。
- STATE_MACHINE 只建立索引和引用校验，不改变状态机正文。
- ADUS 仍然只读派生，不承载人类裁决。

## 9. 运行时假设

- `governance:generate` 是唯一 ADUS 写入口。
- `governance:audit` 保持只读。
- 当前工作树存在用户正在进行的文档归一化改动；本轮只接触 Scope 内文件，不覆盖其他改动。

## 10. 疑似不变量破坏

- 若把 DESIGN_PENDING 需求强行映射到现有规则，会伪造设计完成；本轮改为正式 deferred 状态。
- 若生成器与审计器采用不同解析口径，可能继续出现“索引存在但审计不识别”或反向漂移。
- 若仅生成索引而不验证映射目标、状态机引用和 ADUS 新鲜度，后续仍可能出现已登记但不可解析的伪链路。

## 11. 验证计划

- 运行治理脚本相关测试和变更影响选择器。
- 运行 `npm run governance:generate`，确认 ADUS 出现需求注册表、需求映射表、状态机注册表。
- 运行 `npm run governance:audit`，确认分模块 active REQ 覆盖报告、目标类型、状态机引用和 ADUS 新鲜度检查通过。
- 通过治理回归用例确认 Agent active/deferred 需求集、共享约束映射与四个状态机均已进入 ADUS。
- 运行 `git diff --check`，确认无格式错误。
- 无业务链日志、triage 或产品行为变化。

## Expected Behavior

ADUS 完整登记当前 Active 文档中的 REQ、REQ_MAP 与 STATE_MACHINE；Agent 每条 active REQ 都能追溯到至少一个存在且类型正确的正式技术目标，非 active 需求以正式状态显式闭合；治理审计报告全局覆盖，并对伪映射、状态机断引用和 ADUS 登记滞后失败关闭。

## Validation Commands

- `npm run governance:generate`
- `npm run test:plan -- --files [changed_files]`
- `npm run test:related -- --files [changed_files]`
- `npm run governance:audit`
- `git diff --check`

## Scope Lock

```yaml
task_type: CODE_CHANGE
allowed_files:
  - .governance/active_scope.json
  - .governance/issues/AG-MAPPING-REGISTRY-CLOSURE-01.md
  - docs/01_modules/AG/A-AG-M-D-01_代理功能需求描述.md
  - docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md
  - scripts/generate_adus.mjs
  - scripts/audit_taskflow_governance.mjs
  - scripts/test_governance.test.mjs
  - docs/00_core/A-CORE-C-R-01_ADUS.md
forbidden_files:
  - ADU.md
  - ADP.md
  - AGENTS.md
  - CLAUDE.md
  - governance.config.json
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src/**
  - src-tauri/**
confirmation: YES
scope_reason: 人类明确要求补全当前 Agent 规则注册和需求设计映射链；范围仅包含既有事实状态、映射、派生索引及其生成审计工具。
```

## 人类决策

- `HD-MAPPING-01`（CONFIRMED）：2026-09-01 人类明确要求“补全规则注册和映射链”。
- `HD-MAPPING-02`（INHERITED）：不新增需求、功能或业务逻辑，以收敛控制真源和派生登记为目的。

## Task Runtime Report — Agent 需求映射与 ADUS 登记闭环

task_type: CODE_CHANGE
governance_phase: VALIDATION
final_state: READY
issue_trace: AG-MAPPING-REGISTRY-CLOSURE-01

changed_files:
  - `.governance/active_scope.json` # 修改，记录 Scope、验收和运行结果
  - `.governance/issues/AG-MAPPING-REGISTRY-CLOSURE-01.md` # 新增，记录 Issue Trace 与运行报告
  - `docs/01_modules/AG/A-AG-M-D-01_代理功能需求描述.md` # 修改，将四条 DESIGN_PENDING 需求正式收口为 deferred
  - `docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md` # 修改，补齐 REQ-AG-009 到 X-SHARED-002 映射并同步需求状态
  - `scripts/generate_adus.mjs` # 修改，生成 REQ、REQ_MAP 和 STATE_MACHINE 注册表
  - `scripts/audit_taskflow_governance.mjs` # 修改，校验映射目标、状态机引用、覆盖和 ADUS 新鲜度
  - `scripts/test_governance.test.mjs` # 修改，增加 Agent 需求映射和 ADUS 登记回归
  - `docs/00_core/A-CORE-C-R-01_ADUS.md` # 派生更新，新增需求、需求映射和状态机注册表

scope:
  mode: explicit
  status: confirmed

phase_gate:
  status: passed
  evidence: Agent 需求真源、设计规则、共享约束和四个状态机均为 Active 且已登记。

mapping_chain:
  status: complete
  evidence: Agent 17 条 REQ 中 13 条 active 全部至少有一条 mapped REQ_MAP，4 条未设计需求为 deferred；所有 Agent mapped target 与四个状态机引用均可解析。

test_impact:
  changed_files: Scope 中全部 8 个文件
  direct_modules: AG, CBT, CORE, ED, PCG, SYS, WS
  affected_modules: ACCT, AG, AST, CBT, CORE, DE, DOC, ED, ENT, KB, PCG, PRV, REF, SYS, TPL, WS
  selection: governance self-test + frontend-full + rust-full + governance-audit

validation:
  - `npm run governance:generate` # passed：142 REQ、195 REQ_MAP（191 个唯一映射）、16 STATE_MACHINE、966 测试用例
  - `npm run test:plan -- --files [changed_files]` # passed：选择治理自测、前端全量、Rust 全量与治理审计
  - `npm run test:related -- --files [changed_files]` # passed（沙箱外复跑）：治理 20/20、前端 594/594、Rust 379 单元 + 9 集成、治理审计全部通过
  - `npm run governance:audit` # passed：Agent active 13 / mapped 13 / unmapped_active 0，ADUS 缺失注册 0
  - `git diff --check` # passed

stop_conditions:
  - 无

recovery:
  - 首次沙箱内 Rust 回归的 LibreOffice Worker 以 signal 6 崩溃；相同用例和完整 `test:related` 在沙箱外通过，确认为沙箱限制并已解除验证阻断。

unresolved_items:
  - Scope 外 CORE/CLOUD/GW/DE/DOC/ED/REF/WS 合计 43 条 active REQ 仍无 mapped REQ_MAP；审计已显式报告，本轮未扩大到其他模块。
  - Scope 外存在 4 条完全重复 REQ_MAP（ED 1 条、SYS 3 条）；ADUS 已去重呈现，审计保留原始重复清单。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-09-01 | v1.1 | 完成 Agent 需求、需求映射和状态机的 ADUS 登记与审计闭环，写入 READY 运行报告。 |
| 2026-09-01 | v1.0 | 建立 Agent 需求映射与 ADUS 登记闭环 Scope。 |
