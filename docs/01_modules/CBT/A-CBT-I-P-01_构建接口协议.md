---
文档编号：CBT-I-P-01
文档状态：A
负责模块：CBT
文档职责：CBT 构建模块级接口协议——构建回合 command、任务计划审阅、中断恢复、Chat↔Build 接管接口
上游约束：A-SYS-I-P-02_IPC协议.md（骨架）、A-CBT-M-T-01_构建技术设计.md（BR-CBT-* 规则、CBT-BUILD）、A-CBT-M-S-01_构建状态机设计.md（SM-CBT-01）、A-AG-I-P-01（回合基建、接管）、A-DE-I-P-01（产物 diff 化）、A-WS-I-P-01（落盘、build_runs 控制态）、ADU.md
直接承接：实现期 CBT command 注册；步骤执行委托 AG（回合基建）；产物经 DE 裁决→WS 落盘；接管对接 SM-AG-01
使用边界：声明语义级签名；模型调用经 AG（门禁）、落盘经 DE→WS（不重复）；不新增注册项；任务分解策略延实现期
变更要求：接管接口变更须与 AG 共同复核（X-INTER-001）；产物 diff 化复核 DE；控制态结构复核 SYS（build_runs）
治理成熟度：draft
---

# Binder 构建接口协议（CBT · L2 接口层）

## §1 文档定位

```
A-SYS-I-P-02（骨架）→ 本文（CBT 构建接口）→ 实现 + @GOV(追溯 A-CBT-M-T-01)
步骤执行委托 AG（门禁）；产物 → de.generate_diff；落盘 → ws.apply_diff；接管 ↔ SM-AG-01
```

## §2 接口清单（CBT-BUILD / SM-CBT-01）

| command/事件 | 参数（语义） | 结果 | 特有错误/状态 | 归属 |
|---|---|---|---|---|
| `cbt.start_build` | workspace_id、构建意图、注入引用集 | run_id（SM-CBT-01 Planning）+ 任务计划 | gate_denied（经 AG 门禁）；意图不明→回落 chat | BR-CBT-STATE-001 |
| `cbt.confirm_plan` | run_id、调整后计划（可空） | 进入 TaskRunning | — | PlanReview→执行（计划须用户确认） |
| `cbt.build_stream`（事件） | run_id（订阅） | 任务进度/产物 ChangeProposal 集/终态 | 任务失败→暂停（不静默跳步） | SM-CBT-01；产物经 de.generate_diff |
| `cbt.interrupt_build` | run_id | 中断确认（无半成品落盘，未裁决产物按选择留存/丢弃） | — | SM-CBT-01 interrupt |
| `cbt.resume_build` | run_id | 恢复至中断点 / 明确关闭 | 控制态缺失 | BR-CBT-PERSIST-001（重启可恢复，无悬挂） |
| `cbt.list_builds` | workspace_id | build_runs 列表（A-SYS-I-P-01） | — | 控制态持久化 |

## §3 Chat↔Build 接管接口（X-INTER-001，与 AG 双侧）

| 接口 | 方向 | 语义 | 归属 |
|---|---|---|---|
| `cbt.takeover` | AG chat → CBT | 用户确认后从 chat 回合接管为构建（上下文随移交）；回 takeoverAck | BR-CBT-STATE-002（无双活） |
| `cbt.fallback` | CBT → AG chat | 构建完成/中断后回落 chat 续接；回 fallbackAck | BR-CBT-STATE-002（无双死） |

> 产物落盘走 diff 路径（X-FORBID-006，origin_module=CBT）；批量裁决形态联动 DE-04（CBT-02）。

## §4 测试与验收

- 跨 ≥2 文件构建；计划须 confirm 方执行；任务失败暂停不跳步；interrupt 无半成品；resume 恢复或
  明确关闭无悬挂；takeover/fallback 双向回执无双活双死（与 AG G5 联调）；产物经 DE 裁决落盘。

## §5 债务

- 批量裁决接口随 DE-04 决策（CBT-02）；接管协议细节与 AG 联合最终化；任务分解策略延实现期。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（实现前置件）：CBT 接口协议——构建回合 6（start/confirm_plan/stream/interrupt/resume/list）+ 接管 2（takeover/fallback 双向回执）；计划须确认、失败不跳步、无悬挂、无双活双死；产物经 de.generate_diff（X-FORBID-006）；零新增注册项；债务（批量裁决待 DE-04、接管细节、任务分解延后） |
