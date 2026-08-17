# Issue Trace — AG-1.0-R6-BASELINE-FREEZE-01：Agent 1.0 能力需求基线冻结

文档状态：A
任务类型：DOC_ONLY
治理阶段：REQUIREMENT
创建时间：2026-08-14

## 1. 用户可见问题

经过 R1 至 R5-C 的逐项确认后，Agent 1.0 已有完整的需求边界，但若没有单一冻结记录，后续设计、Prompt 或实现仍可能把“候选/旧文档/已注册工具”误当作已授权能力，重新产生范围漂移。

## 2. 已核实的前置条件

- R1 的三态模型、新建文档与 Diff 生命周期，R2 的感知读取，R3 的编辑/样式/表格/图片边界，R4 的网络、结构、转换、删除和多文档边界均已逐轮确认并同步 L1。
- R5-A 固定 Runtime Profile、AgentTurn、IntentResult、最小授权、可见事件和 AgentSession 隔离；R5-B 固定 TaskRun、真实进度和连续 Proposal；R5-C 固定外部文件一次性读取与用户管理项目记忆。
- 上述确认只完成 L1；L2 映射、状态机、接口、实现、真实测试与发布验收尚未完成，不能伪造为已实现。

## 3. 本轮人类确认

用户明确要求进入 R6，冻结已确认的 Agent 1.0 能力需求基线。

## 4. 冻结规则

- 冻结范围以产品根、Agent 能力基线、冻结记录和模块 L1 的已确认内容为准；其中有冲突时，以产品根与冻结记录的明确边界为准，并在新的 REQUIREMENT Scope 处理。
- L2 只能将冻结需求映射为规则、状态机和接口，不能扩大行为、权限、读取范围、写入时点、保存语义或默认 Profile。
- Prompt、工具注册、兼容默认值、现有代码、测试绿灯或 UI 文案都不是增加能力的授权来源。
- 冻结不等于实现完成。进入实现前仍须完成 DESIGN Scope、REQ_MAP、必要状态机/接口、验证卡及对应验证。

## 5. 直接消费者

- CORE：提供产品级冻结边界和变更入口。
- AG 能力基线：提供 P0/P1/P2、验收场景和 Profile 解释。
- 后续 SYS、DE/ED/WS、AG/REF/AST DESIGN Scope：只能消费冻结需求，不能反向改变它。

## 6. 验收计划

- 冻结记录能明确列出 P0/P1/P2、硬边界、非目标、变更规则、设计同步顺序与“需求确认≠实现验证”。
- 产品根、能力基线和索引均指向同一冻结记录，且无“R5-C 待确认”或“R6 后再确认”的残留表述。
- 冻结记录不新增 REQ、技术规则、接口、状态机、Prompt 或实现承诺。

## 7. 待设计项

- SYS：共享对象、事件、隔离、持久化与恢复。
- DE/ED/WS：三态模型、Proposal 生命周期、保存、关闭、结构和多文档契约。
- AG/REF/AST：Runtime Profile、TaskRun、Intent/Grant、外部内容、显式记忆与 Trace。
- 各能力的验证卡、真实测试和 P1 开放证据。

## 8. 风险与迁移债务

- 若将冻结理解为代码完成，会掩盖仍存在的 L2/实现漂移与未验证风险。
- 若在设计期间修改冻结需求但不回到 REQUIREMENT Scope，模块会得到不一致的边界。
- 若以 Prompt 或工具注册代替产品授权，会重新引入隐式扩权。

## 9. 文件范围结论

允许修改：产品根、需求索引、新的冻结记录、Agent 能力基线、本 Issue Trace 与 active scope。

禁止修改：模块 L1、L2、接口、状态机、验证、代码、脚本、APC、ADUS 和治理配置。

## 10. Scope 外事项

- 技术设计同步、L2 REQ_MAP、状态机、接口、数据字段、Prompt、工具与实现。
- 任何新的需求、优先级或能力地位调整。
- P0/P1 真实功能验证、发布判定或版本发布。

## 11. Task Runtime Report

- task_type：`DOC_ONLY`
- governance_phase：`REQUIREMENT`
- final_state：`READY`
- changed_files：`docs/00_core/A-CORE-C-D-01_产品概述.md`、`docs/00_core/A-CORE-X-D-01_1.0需求基线与文档体系索引.md`、`docs/00_core/A-CORE-X-D-02_Agent1.0冻结能力基线.md`、`docs/01_modules/AG/A-AG-M-D-02_代理1.0能力基线与需求分解.md`、本 Issue Trace 与 active scope。
- delivered_artifact：`A-CORE-X-D-02_Agent1.0冻结能力基线.md`（Agent 1.0 Baseline v1.0）。
- mapping_chain：不适用；本轮不修改 REQ_MAP、RULE、CHAIN、CONSTRAINT、TERM、状态机或接口。
- test_impact：不适用；本轮无代码或测试改动。
- validation：冻结记录、产品根、能力基线和索引的同向引用复核通过；R5-C/R6 待确认残留检索为空；CORE REQ 数保持 19，R6 未新增 REQ；冻结记录链接、尾随空白与 `git diff --check` 通过；`npm run governance:audit` 的聚焦原始字段复核通过（总体、governance、TERM、test governance、product capability 均为 `passed`，`governance_findings` 为空）。
- automated_tests：不适用。
- unresolved_items：见 §7；R6 后仍须先完成 DESIGN Scope，方可进入实现；任何基线变化须新开 REQUIREMENT Scope。
