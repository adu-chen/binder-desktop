# Issue Trace · PI-FE-30　前端治理闭合（补 @GOV owner + covers，登记分页延后）

收口前端自评发现的治理可追溯断裂：已实现但无 @GOV owner / 无测试覆盖的 ED/PRV 规则补齐；
未实现的分页规则（BR-ED-DATA-003）登记为延后债务（不伪造标注骗审计，CLAUDE.md §2）。
ED-EDIT / PRV-VIEW / BR-ED-STATE-003 / BR-ED-DATA-002 / BR-PRV-DATA-001 / BR-PRV-DATA-002。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - src/services/edService.ts            # +@GOV: pollExternalChange→BR-ED-STATE-003；openDocument→BR-ED-DATA-002
  - src/services/edService.test.ts       # +covers BR-ED-STATE-003；+openDocument 路由测试 covers BR-ED-DATA-002
  - src/components/prv/PrvPanel.tsx       # +@GOV: 只读硬边界 BR-PRV-DATA-001
  - src/services/prvService.test.ts       # 新增：covers BR-PRV-DATA-001（只读命令面）+ BR-PRV-DATA-002（mediaSrc）
  - docs/00_core/A-CORE-X-R-01_门禁闭合债务分类台账.md   # 登记 BR-ED-DATA-003 分页延后 + 本次闭合记录
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md
  - .governance/issues/PI-FE-30.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/**
confirmation: YES
scope_reason: 纯治理闭合（标注+测试+登记，无功能变更）：为已实现的 BR-ED-STATE-003（外部变更/冲突）、BR-ED-DATA-002（md/docx 统一模型·docx 二进制归 DOC）、BR-PRV-DATA-001（预览只读硬边界）补 @GOV owner 与 covers 测试，并为 BR-PRV-DATA-002（媒体 asset 流式）补 covers；未实现的 BR-ED-DATA-003（分页纯呈现）如实登记为延后债务而非伪造 @GOV。active_scope.json 被并行窗口占用，本 PI 不触碰，scoped 审计经本 issue Scope Lock 判定。
```

## Expected Behavior

- edService.ts：`pollExternalChange` 加 @GOV（codes BR-ED-STATE-003-RB-ED-ED-EDIT-003，type RB，chain ED-EDIT），声明外部变更检测→Conflicted、用户显式覆盖/重载不静默；`openDocument` 加 @GOV（BR-ED-DATA-002-RB-ED-ED-EDIT-004），声明 docx 走 DOC 链得 HTML 统一模型、md 走文本，ED 不解析 docx 二进制。
- PrvPanel.tsx：加 @GOV（BR-PRV-DATA-001-GUARD-PRV-PRV-VIEW-003，type GUARD），声明预览只读硬边界（无原地编辑入口；原生 docx 仅经"创建草稿"另起编辑会话，不改预览源）。
- 测试：edService.test 既有 poll/conflict 测试加 `covers: BR-ED-STATE-003`；新增 openDocument 路由测试 `covers: BR-ED-DATA-002`；新增 prvService.test `covers: BR-PRV-DATA-001`（只读命令面断言）+ `covers: BR-PRV-DATA-002`（mediaSrc→asset URL）。
- 台账：BR-ED-DATA-003 分页登记为延后 A 类债务（audit OWNER_MISSING/uncovered 属预期，待分页 PI 或人类决策）；记录本次闭合的 4 规则。

## 闭合后预期审计状态（如实）

- referenced_rules +3、covered_rules +4（BR-ED-STATE-003/BR-ED-DATA-002/BR-PRV-DATA-001/BR-PRV-DATA-002 闭合）。
- 全局 audit 仍 failed：剩余 OWNER_MISSING/uncovered = BR-AG-DATA-004/005（AG 窗口在做，台账 B-1）、BR-SYS-DATA-004（云端，台账 C 类）、BR-ED-DATA-003（分页延后，本 PI 登记）——均为已登记/他窗口责任，非前端可独立闭合项。

## Validation Commands

```
npx tsc --noEmit
npm test
npm run build
npm run governance:audit -- --issue .governance/issues/PI-FE-30.md
node scripts/audit_taskflow_governance.mjs   # 复核 referenced/covered 计数变化
```
