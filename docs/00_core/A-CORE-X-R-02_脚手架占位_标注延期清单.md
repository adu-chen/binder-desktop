---
文档编号：CORE-X-R-02
文档状态：A
负责模块：CORE
文档职责：G3 工程脚手架导出符号的标注延期清单——记录暂未有 @GOV 的可审计脚手架符号及其退出条件
上游约束：A-CORE-C-P-01_APC.md（§5 G4、§12 audit）、ADU.md（§5.3.1 标注延期清单）
直接承接：G4 实现期；符号具备领域逻辑后补 @GOV，或删除符号后关闭延期记录
使用边界：本文不是 triage 报告，不记录 rules: [] 分类，不承载技术方案决策；只作为 BLOCK_UNANNOTATED 的精确延期豁免
变更要求：符号补 @GOV、删除或不再被负空间扫描时，立即将对应记录改为已解决并记录清退原因
治理成熟度：draft
---

# Binder 脚手架占位符号标注延期清单（G3 过渡）

## §1 背景

脚手架阶段有少量导出符号只承载外壳、视图投影或 workspace 打开入口，尚未拥有可稳定追溯的领域
`@GOV` 块。负空间扫描仅按本清单的“文件 + symbol”精确匹配暂缓告警；这不是永久豁免。

## §2 活动延期记录

symbol: App
文件: src/shell/App.tsx
原因: 已复核为持久化活动 workspace 的恢复/投影外壳；对应业务边界已稳定
承接计划: 已关闭（ANNOTATION-DEFERRAL-CLOSURE-01）
退出条件: 已在 App 补 BR-WS-PERSIST-002-RB-SYS-DATA-MIGRATE-001
复核日期: 2026-08-01
需要人类介入: 否
已解决: 是

symbol: Welcome
文件: src/shell/Welcome.tsx
原因: 已复核为注册 workspace 的打开、切换和最近列表入口协调者；边界已稳定
承接计划: 已关闭（ANNOTATION-DEFERRAL-CLOSURE-01）
退出条件: 已在 Welcome 补 BR-WS-PERSIST-002-RB-WS-DATA-MIGRATE-001
复核日期: 2026-08-01
需要人类介入: 否
已解决: 是

symbol: Workbench
文件: src/shell/Workbench.tsx
原因: 已复核为活动 workspace 绑定面板和状态机投影监听的协调边界；职责已稳定
承接计划: 已关闭（ANNOTATION-DEFERRAL-CLOSURE-01）
退出条件: 已在 Workbench 补 BR-WS-PERSIST-002-RB-WS-DATA-MIGRATE-002
复核日期: 2026-08-01
需要人类介入: 否
已解决: 是

symbol: useWorkspaceShell
文件: src/store/workspaceShell.ts
原因: 已复核为后端 workspace 控制态的内存投影；不得自行决定活动 workspace
承接计划: 已关闭（ANNOTATION-DEFERRAL-CLOSURE-01）
退出条件: 已在 workspaceShell 补 BR-WS-PERSIST-002-EFFECT-WS-DATA-MIGRATE-004
复核日期: 2026-08-01
需要人类介入: 否
已解决: 是

symbol: openWorkspace
文件: src/services/workspaceService.ts
原因: 已复核为目录选择、受控 IPC 打开、下游会话清理与恢复的活动 workspace 协调入口
承接计划: 已关闭（ANNOTATION-DEFERRAL-CLOSURE-01）
退出条件: 已在 openWorkspace 补 BR-WS-PERSIST-002-EFFECT-WS-DATA-MIGRATE-005
复核日期: 2026-08-01
需要人类介入: 否
已解决: 是

## §3 已关闭的旧登记

`WorkspaceRefView` 是导出的 TypeScript interface，不属于 audit 的可执行导出符号扫描范围，故不再作为
活动延期记录。它不是未标注可执行块，也不需要用延期清单豁免。

## §4 复核记录

- 2026-07-02：原登记逐项复核，Workbench 的布局扩展仍属视图层导航态，当时无可清退项。
- 2026-08-01：迁移为结构化延期清单；移除不在扫描范围内的 `WorkspaceRefView`，其余记录改为精确匹配。
- 2026-08-01：`ANNOTATION-DEFERRAL-CLOSURE-01` 逐项复核。五个符号均已具备稳定的活动 workspace
  控制态投影或协调职责，已映射既有 `BR-WS-PERSIST-002`；未新增规则，也未改变运行时行为。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-22 | v1.0 | 初版：登记 6 个脚手架占位符号，供旧版文本匹配 audit 暂缓告警 |
| 2026-07-02 | v1.1 | 新增 G-9 复核记录：无可清退项 |
| 2026-08-01 | v2.0 | 迁移为标注延期清单：以文件 + symbol 精确匹配；`WorkspaceRefView` 因非可执行导出关闭；不再作为 triage 报告 |
| 2026-08-01 | v2.1 | 清退 5 条到期延期：App、Welcome、Workbench、workspaceShell、openWorkspace 均映射既有 BR-WS-PERSIST-002；不改变功能或测试行为 |
