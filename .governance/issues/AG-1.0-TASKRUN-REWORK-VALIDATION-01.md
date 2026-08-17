# Issue Trace — AG-1.0-TASKRUN-REWORK-VALIDATION-01：V-S7 验证材料修订

文档状态：A
任务类型：VALIDATION
治理阶段：VALIDATION
创建时间：2026-08-15
前置：AG-1.0-TASKRUN-REWORK-REQ/DESIGN/IMP-RUNTIME/IMP-UI/PROMPT（阶段 0-4 已落地）

## 1. 用户可见问题

阶段 0-4 重构后，V-S7 验证卡（A-VAL-X-V-04 §4.11）与手动测试卡（A-VAL-X-V-05）仍表述旧语义
（确认点=提案卡、顶部卡呈现）。需按三层分离/确认 gate/片段内多卡/实时状态修订验证材料。

## 2. 入口点

- `docs/40_validation/A-VAL-X-V-04_Agent1.0验证卡.md`（§4.11 V-S7 期望结果/人类观察项更新）
- `docs/40_validation/A-VAL-X-V-05_TaskRun手动测试卡.md`（用例修订：三层分离语义）
- 本 Issue Trace、`.governance/active_scope.json`

## 3. 相关规则

- REQ-AG-015、BR-SYS-STATE-008、A-AG-M-D-01 §5b、A-AG-M-S-01 §2.2.2

## 4. 验证覆盖（六项）

1. 任务计划 → 读取 → 多 Proposal（单片段连续多卡，前卡因基准推进失效）
2. 独立权限/方案确认（DecisionRequest）后自动续片段（无需聊天消息、不后台续跑）
3. Diff 接受/拒绝不改变 TaskRun 状态（三层分离）
4. 任意新消息不绕过待确认（AG_DECISION_PENDING gate）
5. 实时状态流（信息流底部"正在读取 a.md"）
6. 同文档连续 Diff 失效规则

## 5. 人类确认记录

2026-08-15：人类批准分阶段实现；阶段 5 修订验证材料并登记，真实环境执行由用户完成。confirmation: YES。

## 6. Task Runtime Report（完成后填写）

- task_type：`VALIDATION`
- final_state：`READY`（材料就绪；真实环境 PASS 待用户执行后登记）
- changed_files：A-VAL-X-V-04（§4.11）、A-VAL-X-V-05（用例修订）、本 Issue Trace、active_scope
- validation：`governance:audit` # passed（exit 0）。
- unresolved_items：真实环境手动执行（V-S7 PASS/FAIL 登记）；auto-continue 与 TaskPlan 前端呈现为后续迭代。
