---
文档编号：CORE-X-R-02
文档状态：A
负责模块：CORE
文档职责：G3 工程脚手架占位符号 triage 登记——脚手架阶段导出的占位可执行符号尚无 @GOV（领域逻辑未实现），在此登记豁免负空间扫描；@GOV 块在 G4 各模块实现时随真实逻辑落入对应文件
上游约束：A-CORE-C-P-01_APC.md（§5 G4「代码块有 @GOV 或 triage」、§12 audit）、A-CORE-X-L-02_总开发实施计划.md（P0–P5）、各模块 A-{M}-X-L-01（Plan Item，@GOV 落点）
直接承接：G4 实现期，下列符号补 @GOV（追溯对应 BR-* 规则）后从本登记移除
使用边界：本文是 triage 登记（_triage.md，audit 负空间豁免机制）；不定义注册块；登记仅为脚手架过渡期，G4 实现后逐项清退
变更要求：符号补 @GOV 后从本表删除；新增脚手架占位符号追加登记
治理成熟度：draft
---

# Binder 脚手架占位符号 triage 登记（G3 过渡）

## §1 背景

2026-06-22 搭建工程脚手架（Tauri 2 + React/TS + Rust）后，前端导出了若干**占位可执行符号**
（外壳/Service/Store 骨架），其领域逻辑待 G4 实现，故尚无 `@GOV` 标注。

audit 负空间扫描（`scripts/audit_taskflow_governance.mjs` BLOCK_UNANNOTATED，**警告级、不阻断**）
对这些符号报 `block_unannotated`；脚本支持「triage 登记豁免」（符号名出现在 `*_triage.md` 即豁免）。
本文据此登记，使 audit 警告清零。**这是过渡机制**：G4 各模块实现时，符号补 `@GOV`（追溯
对应 `BR-*` 规则）后从本表逐项移除。

## §2 占位符号登记（@GOV 待 G4）

| 符号 | 文件 | 承接 PI | G4 应补 @GOV 追溯 |
|---|---|---|---|
| `App` | src/shell/App.tsx | PI-SYS-05 | X-FORBID-007（前端状态投影） |
| `Welcome` | src/shell/Welcome.tsx | PI-SYS-05 | 欢迎页外壳（A-SYS-C-T-02 §4.1） |
| `Workbench` | src/shell/Workbench.tsx | PI-SYS-05 | 主工作台分区（A-SYS-C-T-02 §3） |
| `useWorkspaceShell` | src/store/workspaceShell.ts | PI-SYS-05 | X-FORBID-007（投影后端活动态） |
| `WorkspaceRefView` | src/store/workspaceShell.ts | PI-SYS-02 | WorkspaceRef 前端视图（A-SYS-I-P-01） |
| `openWorkspace` | src/services/workspaceService.ts | PI-WS-01 | BR-WS-DATA-001（经 ws.open_workspace，X-SEQ-002 分层） |

> 后端 16 模块为占位 `mod.rs`（仅注释、无导出实质逻辑），不触发负空间扫描；G4 各模块实现时
> 在领域函数上落 `@GOV`。

## §3 清退规则

- G4 实现某符号的真实逻辑时，在其上方加 `/** @GOV rules: BR-... */` 块，并从 §2 删除该行。
- §2 清空即本文可废止（状态改 X）。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-22 | v1.0 | 初版：登记 6 个脚手架占位符号（App/Welcome/Workbench/useWorkspaceShell/WorkspaceRefView/openWorkspace）豁免 audit 负空间警告；@GOV 待 G4 各 PI 实现期落入，清退规则 §3 |
