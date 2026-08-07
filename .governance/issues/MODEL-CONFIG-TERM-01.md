# Issue Trace · MODEL-CONFIG-TERM-01 构建模式中文称呼更名「代理构建」

承接人类 2026-08-07 指令：注册「代理构建」替换「构建模式」中文称呼（TERM-AG-007 zh 更名，
en BuildMode 与 term_id 不变；旧中文名「构建模式」退役为禁用别名）；弹窗内「构建」改为「代理构建」。

## 1. 用户可见症状

构建模式中文称呼不统一；模型配置弹窗能力偏好区显示「构建」。

## 2. 入口点

- TERM-AG-007（docs/01_modules/AG/A-AG-M-T-01 术语注册表）
- src/components/ag/ModelConfig.tsx（能力偏好区 label）
- src/components/ag/Composer.tsx（模式下拉 hint/title）

## 3. 相关规则

- 修改：TERM-AG-007（zh=构建模式 → 代理构建；forbidden + 构建模式）
- 无 RULE/CHAIN/CONSTRAINT 变更；REQ-AG-006 summary 表述同步（构建 BuildMode → 代理构建 BuildMode）
- 代码无 term_ref 引用 TERM-AG-007（已 grep 确认），term_id/en 不变，不破坏既有引用

## 4. 相关链路

AG-TURN（TERM-AG-007 主链路不变）。

## 5. 生产者块

- A-AG-M-T-01 TERM-AG-007 块（术语注册表）
- 无 @GOV 块变更（纯字符串/术语改名，不涉及代码块边界）

## 6. 消费者块

- ModelConfig / Composer / agStore / modelStore / AgentPanel 中的「构建」文本（BuildMode 指代）

## 7. 路径上的空规则块

- 无（不新增/修改 @GOV）

## 8. boundary 假设

- 无代码块 boundary 变化

## 9. 运行时假设

- 纯命名变更，无运行时行为变化；AGModes 枚举值 "build"（代码标识符）不变

## 10. 疑似不变量破坏

- 未发现。term_id / en / 代码标识符不变；仅中文展示与术语注册表 zh 变更。

## 11. 验证计划

- `npm run governance:generate` + `governance:audit`
- `npx tsc --noEmit`；AG 相关 vitest（assistRuntime/agRuntime）
- grep 确认「构建模式」不再出现在 AG 文档正文与 UI（BuildMode 指代处已换「代理构建」）

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: MODEL-CONFIG-TERM-01
governance_phase: IMPLEMENTATION
allowed_files:
  - .governance/active_scope.json / issues/MODEL-CONFIG-TERM-01.md
  - docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md   # TERM-AG-007 zh/forbidden + 正文引用
  - docs/01_modules/AG/A-AG-M-D-01_代理功能需求描述.md # REQ-AG-006 summary 表述
  - docs/00_core/A-CORE-C-R-01_ADUS.md               # governance:generate 输出
  - src/components/ag/ModelConfig.tsx                 # 弹窗「构建」→「代理构建」
  - src/components/ag/Composer.tsx                     # 模式下拉 hint/title/注释
  - src/components/ag/AgentPanel.tsx                   # 注释
  - src/store/agStore.ts                               # 注释
  - src/store/modelStore.ts                            # 注释
forbidden_files:
  - ADU.md / ADP.md / AGENTS.md / CLAUDE.md / APC / governance.config.json
  - scripts/test_governance.mjs / scripts/test_governance.test.mjs
  - docs/01_modules/CBT/**  # 模块名「构建（Build）」语义域，不在本次范围
  - docs/00_core/generated/**  # 机器生成产物，禁止手工编辑
confirmation: YES  # 人类 2026-08-07 指令「只改中文称呼…把弹窗里的构建改为代理构建」
scope_reason: 术语中文称呼更名 + 同术语 UI/文档引用同步（最小范围）
```
