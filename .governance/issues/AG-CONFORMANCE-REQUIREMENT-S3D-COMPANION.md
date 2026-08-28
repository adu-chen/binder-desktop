# Issue Trace — AG-CONFORMANCE-REQUIREMENT-S3D-COMPANION

文档状态：A
任务类型：GOVERNANCE_CHANGE
治理阶段：REQUIREMENT
任务状态：READY
创建时间：2026-08-28
人类裁决：HD-S3D-03（CONFIRMED）

## 发现与根因

S3D 实现阶段按规则链复核发现：最新 TERM-AG-019 / BR-AG-DATA-003 已把用户输入中指向 active workspace
既有文档的显式 ReferencePayload 身份纳入 ExplicitEditTargetSet，但冻结需求仍把“引用内容是 data-only”表述成
“引用身份绝不产生 propose_edit 授权”。两者混淆了内容信任级别与目标授权元数据，导致需求→设计链不唯一。

## 本阶段裁决

1. ReferencePayload 的正文、选区文本与文档内容始终是 data-only，不能改变 Prompt 指令、Intent、工具或权限。
2. 用户主动附加且可归一到 active workspace 既有文档的引用身份，是用户可见的目标授权元数据，可初始化
   ExplicitEditTargetSet；模型正文、历史、read/search/list 结果不能产生同等授权。
3. 集合成员资格只是 ChangeProposal 的必要条件；只读 IntentResult 仍没有 propose_edit，写工具还需通过
   CapabilityGrant、LogicalState/DiskState 基线和 DE/Diff 裁决。
4. 多既有文档编辑保持 P1：用户输入显式冻结引用清单，或工具发现目标经 scope DecisionRequest 确认；仍逐目标
   Proposal、独立基线、逐卡裁决，不提供批量接受、批量保存或跨文档事务。

## 范围与后续

本 Scope 只修改 CORE 冻结能力基线与 AG 能力需求基线。技术设计、Intent/Context 情况说明和开发验证计划分别在
后续 DESIGN / PLAN Scope 同步；现有 S3D 代码补丁保持不动。

## Task Runtime Report

- task_type：GOVERNANCE_CHANGE
- governance_phase：REQUIREMENT
- final_state：READY
- scope：explicit / confirmed
- phase_gate：passed；HD-S3D-03 已明确最新产品裁决，当前回到 G1 闭合需求。
- changed_files：CORE 冻结能力基线、AG 能力需求基线、Scope 与本 Issue Trace。
- validation：`git diff --check` 通过；`npm run test:governance` 19/19 通过且注册表审计通过；`npm run governance:audit` 通过。
- stop_conditions：无。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-28 | v1.0 | 建立显式引用与 ExplicitEditTargetSet 的配套 REQUIREMENT Scope，分离 data-only 内容与用户显式目标授权元数据。 |
| 2026-08-28 | v1.1 | 完成 CORE/AG 冻结需求同步：单目标 P0、多目标 P1 保持，引用身份可授权候选目标而内容仍为 data-only；治理验证通过，进入 READY。 |
