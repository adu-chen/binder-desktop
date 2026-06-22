---
文档编号：TPL-I-P-01
文档状态：A
负责模块：TPL
文档职责：TPL 模板/工作流模块级接口协议——模板管理 command、模板应用、工作流执行（步骤委托 AG/CBT）
上游约束：A-SYS-I-P-02_IPC协议.md（骨架）、A-TPL-M-T-01_模板工作流技术设计.md（BR-TPL-* 规则、TPL-FLOW、Template/WritingWorkflow）、A-TPL-M-S-01（SM-TPL-01 工作流执行机）、A-AG-I-P-01/A-CBT-I-P-01（执行委托）、A-WS-I-P-01（templates/workflows/workflow_runs 控制态）、ADU.md
直接承接：实现期 TPL command 注册；工作流步骤委托 AG（单文档）/CBT（多文件）；产物经 DE→WS；持久化经 workspace.db
使用边界：声明语义级签名；执行通道归 AG/CBT、落盘经 DE→WS（不重复）；不新增注册项；workspace 级隔离（决策）
变更要求：工作流执行接口变更复核 SM-TPL-01 与 AG/CBT 委托；模板/工作流结构复核 SYS
治理成熟度：draft
---

# Binder 模板/工作流接口协议（TPL · L2 接口层）

## §1 文档定位

```
A-SYS-I-P-02（骨架）→ 本文（TPL 管理/应用/执行接口）→ 实现 + @GOV(追溯 A-TPL-M-T-01)
执行委托 → ag.start_turn / cbt.start_build；产物 → de.generate_diff；持久化 → templates/workflows/workflow_runs
```

## §2 模板管理与应用（TPL-FLOW / BR-TPL-DATA-001）

| command | 参数（语义） | 结果 | 特有错误 | 归属 |
|---|---|---|---|---|
| `tpl.list_templates` | workspace_id、kind（document/prompt 可空） | Template 列表（仅当前 workspace） | — | 隔离 |
| `tpl.upsert_template` | Template（含 body/变量占位/组织） | template_id | — | BR-TPL-DATA-001 |
| `tpl.delete_template` | template_id | 删除确认 | — | — |
| `tpl.apply_template` | template_id、变量绑定 | 文档模板→新文档（经正常落盘）/ 提示模板→回合输入（交 AG） | 实例化失败→无半成品 | BR-TPL-DATA-001（原子实例化） |

## §3 工作流执行（TPL-FLOW / SM-TPL-01）

| command/事件 | 参数（语义） | 结果 | 特有错误/状态 | 归属 |
|---|---|---|---|---|
| `tpl.upsert_workflow` | WritingWorkflow（步骤序列+通道+确认点） | workflow_id | — | BR-TPL-DATA-001 |
| `tpl.start_workflow` | workflow_id、初始绑定 | run_id（SM-TPL-01） | — | BR-TPL-STATE-001 |
| `tpl.confirm_step` | run_id | 进入步骤执行（委托 AG/CBT，过门禁） | 确认点不得自动通过 | SM-TPL-01 AwaitingConfirm |
| `tpl.workflow_stream`（事件） | run_id（订阅） | 步骤进度/产物/暂停/终态 | 步骤失败→Paused（不跳步） | SM-TPL-01 |
| `tpl.resume_step` | run_id、动作（retry/skip_confirmed） | 续行 | skip 须显式 | SM-TPL-01 Paused |
| `tpl.abort_workflow` | run_id | 中止确认（无半成品落盘） | — | SM-TPL-01 abort |
| `tpl.list_runs` | workspace_id | workflow_runs 列表 | — | 可恢复无悬挂 |

> 步骤产物经 DE→WS diff 路径（X-FORBID-006）；执行通道无独立模型/写盘通道。

## §4 测试与验收

- 模板原子实例化（失败无半成品）；隔离断言；工作流确认点强制暂停不自动通过、失败 Paused 不跳步、
  skip 须显式、abort 无半成品、resume 恢复无悬挂；步骤委托 AG/CBT 过门禁；与 AG/CBT/DE G5 联调。

## §5 债务

- 变量占位语法/绑定 UI、条件分支表达（v1 顺序基线）延实现期（TPL-01）；内置模板集属运营（TPL-04）。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（实现前置件）：TPL 接口协议——模板管理 4（list/upsert/delete/apply 原子实例化）+ 工作流 6（upsert/start/confirm_step/stream/resume/abort/list）；确认点强制、失败不跳步、无悬挂、产物经 diff；执行委托 AG/CBT；零新增注册项；债务（变量语法/条件分支/内置集延后） |
