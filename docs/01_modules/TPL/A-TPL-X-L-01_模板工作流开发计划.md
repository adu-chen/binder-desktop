---
文档编号：TPL-X-L-01
文档状态：A
负责模块：TPL
文档职责：TPL 模板/工作流 G3 开发计划——承接型 Plan Item，承接 TPL-FLOW 链路与 BR-TPL-* 规则、SM-TPL-01，明确文件范围/@GOV/A 类债务/验收
上游约束：A-CORE-X-L-02（P4）、APC §5/§6/§12、A-TPL-M-T-01/A-TPL-M-S-01、A-TPL-I-P-01、A-AG-X-L-01/A-CBT-X-L-01（执行委托）、A-CORE-X-R-01
直接承接：本计划驱动 TPL 实现；@GOV 追溯 BR-TPL-*；SM-TPL-01 转移矩阵
使用边界：L3 承接型 Plan Item；执行通道归 AG/CBT、落盘经 DE→WS（不重复）；@GOV 在实现代码
变更要求：PI 增删保持承接可追溯
治理成熟度：draft
---

# Binder 模板/工作流开发计划（TPL · L3）

## §1 文档定位

TPL 模板与工作流编排，workspace 级隔离，执行委托 AG/CBT（无独立模型/写盘通道）。工作流执行
建机（SM-TPL-01，评估结论=建机）。

## §2 Plan Item 清单

### PI-TPL-01　模板管理与原子实例化　✅ G4 已实现（2026-06-22，template.rs 隔离 + 原子实例化失败无半成品 + 3 covers + audit 绿）
- **承接**：REQ-TPL-001/002 / BR-TPL-DATA-001（隔离 + 原子实例化）。
- **文件范围**：`src-tauri/src/tpl/template.rs`（templates 表 CRUD；实例化——文档模板→新文档经
  落盘、提示模板→回合输入交 AG）；`tpl.list_templates/upsert_template/delete_template/
  apply_template`。
- **@GOV**：隔离 + 原子实例化（失败无半成品）`BR-TPL-DATA-001`。
- **A 类债务**：变量占位语法/绑定 UI（TPL-01，实现期）；内置模板集（TPL-04，运营内容）。
- **验收**：模板限当前 workspace（隔离）；实例化失败无半成品；重启恢复。

### PI-TPL-02　工作流执行机　✅ G4 已实现（2026-06-22，workflow.rs SM-TPL-01 确认点强制/失败不跳步/中止无半成品/重启无悬挂 + 3 covers + audit 绿）
- **承接**：REQ-TPL-003 / SM-TPL-01 / BR-TPL-STATE-001（执行编排）；workflows/workflow_runs 表。
- **文件范围**：`src-tauri/src/tpl/workflow.rs`（步骤序列、确认点、逐步委托 `ag.start_turn`/
  `cbt.start_build`、控制态持久化）；`tpl.upsert_workflow/start_workflow/confirm_step/
  workflow_stream/resume_step/abort_workflow/list_runs`。
- **@GOV**：确认点强制 + 失败不跳步 + 无悬挂 + 产物走 diff `BR-TPL-STATE-001`。
- **A 类债务**：条件分支表达（v1 顺序基线，实现期扩展）；步骤委托取消传播（实现期）。
- **验收（强制，APC §6）**：SM-TPL-01 矩阵——确认点强制暂停不自动通过；失败 Paused 显式处置
  （retry/显式 skip/abort）不静默跳步；abort 无半成品落盘；重启恢复无悬挂；每步委托过门禁、产物
  经 diff 路径（与 AG/CBT/DE G5 联调）。

## §3 横切验收

audit 绿；@GOV 追溯 BR-TPL-*；SM-TPL-01 矩阵全覆盖；`// covers: BR-TPL-*`（2 条）有断言。

## §4 承接的 A 类债务（A-CORE-X-R-01 §6）

| 债务 | PI | 处置 |
|---|---|---|
| 变量占位语法/绑定 UI（TPL-01） | PI-TPL-01 | 实现期 |
| 内置模板集（TPL-04） | PI-TPL-01 | 运营内容 |
| 条件分支表达（v1 顺序） | PI-TPL-02 | 实现期扩展 |
| 步骤委托取消传播 | PI-TPL-02 | 实现期 |
| 量化 NFR | 全 PI | G5 标定 |

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（G3，P4）：TPL 2 条 Plan Item（模板管理与原子实例化 / 工作流执行机），承接 TPL-FLOW + 2 规则 + SM-TPL-01；执行委托 AG/CBT 无独立通道；确认点强制/不跳步/无悬挂；文件范围/@GOV/验收齐 |
