---
文档编号：CBT-X-L-01
文档状态：A
负责模块：CBT
文档职责：CBT 构建 G3 开发计划——承接型 Plan Item，承接 CBT-BUILD 链路与 BR-CBT-* 规则、SM-CBT-01，明确文件范围/@GOV/A 类债务/验收
上游约束：A-CORE-X-L-02（P4）、A-CORE-X-D-02（Build/CBT 为 P2 后续 Profile）、APC §5/§6/§12、A-CBT-M-T-01/A-CBT-M-S-01、A-CBT-I-P-01、A-AG-M-T-01/A-AG-M-S-01（仅共享 AgentTurn 基础；agent-edit-1.0 不激活构建接管）、A-DE-X-L-01（产物 diff 化）、A-CORE-X-R-01
直接承接：本计划驱动 CBT 实现；@GOV 追溯 BR-CBT-*；SM-CBT-01 转移矩阵
使用边界：L3 承接型 Plan Item；模型调用经 AG、落盘经 DE→WS（不重复）；@GOV 在实现代码
变更要求：接管 PI 与 AG PI-AG-06（B-3）双侧对齐；PI 增删保持承接可追溯
治理成熟度：draft
---

# Binder 构建开发计划（CBT · L3）

## §1 文档定位

CBT 多文件构建，复用 AG 门禁基建、产物走 DE→WS diff 路径。**接管对接依赖 AG B-3 补齐**
（PI-AG-06）——CBT 侧已就绪，待 AG 侧建模对齐。

## §2 Plan Item 清单

### PI-CBT-01　构建回合机与任务编排　✅ G4 已实现（2026-06-22，build.rs SM-CBT-01 计划确认/失败不跳步/中断无半成品 + 无悬挂恢复 + 3 covers + audit 绿）
- **承接**：REQ-CBT-001/003 / CHAIN CBT-BUILD / SM-CBT-01 / BR-CBT-STATE-001 / BR-CBT-PERSIST-001。
- **文件范围**：`src-tauri/src/cbt/build.rs`（意图分解→任务计划→逐任务经 AG 回合基建→控制态
  持久化 build_runs 表）；`cbt.start_build/confirm_plan/build_stream/interrupt/resume/list`。
- **@GOV**：生命周期/计划确认/失败不跳步 `BR-CBT-STATE-001`；控制态无悬挂 `BR-CBT-PERSIST-001`。
- **A 类债务**：任务分解提示工程/模型策略（实现期，承 AG B-1 提示词架构）；并行任务执行（v1 按序
  基线）。
- **验收（强制，APC §6）**：SM-CBT-01 矩阵——计划须 confirm 方执行；任务失败暂停不静默跳步；
  interrupt 无半成品落盘；重启恢复至中断点或明确关闭无悬挂；每次模型调用过门禁。

### PI-CBT-02　产物 diff 化落盘　✅ G4 已实现（2026-06-22，apply.rs 产物 diff 化无旁路写盘/新建空基准/按任务序 + 2 covers + audit 绿）
- **承接**：REQ-CBT-002 / BR-CBT-DATA-001 / X-FORBID-006。
- **文件范围**：`src-tauri/src/cbt/apply.rs`（逐任务产出 ChangeProposal 交 `de.generate_diff`，
  origin=CBT；新建文档空基准表达；按条裁决）。
- **@GOV**：产物 diff 化无旁路写盘 `BR-CBT-DATA-001`/`X-FORBID-006`。
- **A 类债务**：批量裁决形态（联动 DE-04，C 类待人类；本 PI 按条裁决基线）。
- **验收**：构建产物在用户接受前不写入文件；部分接受仅 accepted 任务落盘；新建文档空基准 accepted
  后正确创建；无旁路直写（与 DE/WS G5 联合）。

### PI-CBT-03　构建移交/回落（对接 AG PI-AG-06 / B-3）　✅ G4 已实现（2026-06-22，takeover.rs 接管须确认+ack 双向回执/active_owner 结构性无双活双死 + 2 covers + audit 绿；AG 侧对接待 PI-AG-06）
- **承接**：REQ-CBT-004 / BR-CBT-STATE-002（接管协议）；X-INTER-001（与 SM-AG-01）。
- **文件范围**：`cbt.takeover/fallback`（双向回执，对接 SM-AG-01 takeover/fallback）。
- **@GOV**：接管无双活双死 `BR-CBT-STATE-002`。
- **依赖**：**AG B-3 补齐（SM-AG-01 接管转移）后双侧联调**；CBT 侧已定（A-CBT-M-S-01 §3）。
- **验收**：接管经用户确认；事件+回执双向确认；无双活/双死（与 AG G5 联调）。

## §3 横切验收

audit 绿；@GOV 追溯 BR-CBT-*；SM-CBT-01 矩阵全覆盖；`// covers: BR-CBT-*`（4 条）有断言；
CBT 构建 G5 端到端（含接管，依赖 AG B-3）。

## §4 承接的债务（A-CORE-X-R-01）

| 债务 | 类 | PI | 处置 |
|---|---|---|---|
| 任务分解提示工程 | A | PI-CBT-01 | 实现期（承 AG B-1） |
| 并行执行（v1 按序） | A | PI-CBT-01 | 实现期 |
| 批量裁决形态（DE-04） | C | PI-CBT-02 | 待人类/实现期 |
| 接管 AG 侧建模（B-3） | B（AG 侧） | PI-CBT-03 | 待 AG B-3 补齐后联调 |
| 构建吞吐/中断响应 NFR | A | 全 PI | G5 标定 |

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（G3，P4）：CBT 3 条 Plan Item（构建回合机与编排 / 产物 diff 化 / 接管对接），承接 CBT-BUILD + 4 规则 + SM-CBT-01；接管依赖 AG B-3 补齐后联调；产物走 DE→WS diff 路径（X-FORBID-006）；文件范围/@GOV/验收齐 |
| 2026-07-04 | v1.1 | 术语统一清扫（承 2026-07-04 人类三轮裁决）：正文禁用别名替换——内联辅助→局部修改、自动补全→辅助续写、对话编辑→Diff Edit、Chat↔Build 接管→构建移交/回落、chat/chat-agent 回合→代理回合；文件名与变更记录历史行不改写 |
