# Issue Trace — AG-1.0-R2-REQUIREMENTS-01：对话、项目感知与阅读分析需求冻结

文档状态：A
任务类型：DOC_ONLY
治理阶段：REQUIREMENT
创建时间：2026-08-13

## 1. 用户可见问题

Agent 1.0 已有候选能力地图，但“项目分析是否属于主链、默认可读取到什么范围、编辑器未保存内容是否为事实、未打开文档如何标识新鲜度、多文档能读到什么程度”等仍可能因不同模块各自解释而漂移，不能进入设计或开发。

## 2. 本轮人类确认

1. `CAP-01` 基础聊天与任务交互为 P0；回答、分析、Proposal 和文件操作必须是不同终态，取消/失败不得伪装为完成。
2. `CAP-02` 项目感知与项目分析报告为 P0；默认先读取工作区概览，正文仅在任务需要时按最小范围读取，报告必须披露已读/未读范围、来源和不确定性。
3. `CAP-03` 当前编辑器与文档状态感知为 P0；活动文档、打开标签、脏态、保存基准、任务所需选区和关联 Proposal 必须可被真实感知；已打开文档以 LogicalState 为准。
4. `CAP-04` 文档阅读、显式引用与比较为 P0；未打开文档可按受限 workspace 读取 DiskState 并标识读取时点；M1 多文档读取/比较和 M2 当前文档修改、其他文档显式引用为 P0，M4 多个既有文档写入保持 P1。

## 3. 需求边界

- 本轮只冻结 L1 需求、能力档位和验收口径；不注册 L2 RULE/CHAIN/CONSTRAINT，不定义接口字段或实现方案。
- 项目正文不是默认全量上下文；目录概览也不等同于正文读取。
- 显式引用和未打开文档读取均为 data-only，不因读取或引用自动成为编辑目标或获得写权限。
- 已打开、包含未保存编辑的文档只能使用其 LogicalState；未打开文档才可使用 DiskState，并必须说明其新鲜度。

## 4. 直接消费者

- CORE：产品级 1.0 主链和读取范围边界。
- AG：任务终态、上下文计划、项目报告与只读分析。
- ED：编辑器会话、标签、脏态、选区和 LogicalState 快照。
- REF：显式引用令牌、来源、锚和 data-only 注入。
- WS：工作区概览、受限 DiskState 读取与新鲜度。
- SYS：AgentReadContext、AgentTurn 与跨模块事实来源的需求级职责。

## 5. 验收计划

- 文档逐项检查：CAP-01~04、M1/M2/M4 档位和两阶段读取策略在能力基线、Core 索引及直接 L1 消费者中一致。
- 检查所有改动文件的文档头、上游/直接承接关系和变更记录。
- 不运行实现测试，也不以已有实现推翻本轮需求；后续 DESIGN Scope 才建立 RULE/CHAIN/状态机/接口和验证卡。

## 6. 文件范围

允许：Scope Lock `allowed_files` 中的 Core/AG/ED/REF/WS/SYS L1 需求文档、能力基线、需求索引和本 Issue Trace。

禁止：L2 设计/状态机/接口、验证、代码、工具 manifest、Prompt、Runtime Profile 和所有治理控制文档。

## 7. 与既有 Scope 的关系

`DIFF-STATE-COMPASS-01` 的 Scope 因范围外 LibreOffice DOC-CONVERT Worker 失败保持其 `BLOCKED` Runtime Report。本轮不修改、不掩盖也不扩大该问题；人类已指示继续 Agent 需求收敛，故另开本需求 Scope。

## 8. Task Runtime Report

- `task_type`: `DOC_ONLY`
- `governance_phase`: `REQUIREMENT`
- `final_state`: `READY`
- `changed_files`: 产品根、能力基线、需求体系索引，以及 AG/ED/REF/WS/SYS 的直接 L1 消费文档；未改 L2、验证、代码、Prompt 或 Runtime Profile。
- `validation`：逐项核对 CAP-01~04、两阶段读取、LogicalState/DiskState 新鲜度、M1/M2/M4 与 data-only 边界；检查所有本轮文档头、REQ 上游/承接指针和变更记录；作用域内不存在已失效的能力基线 `§5.20` 指针；对 Scope 内改动执行空白错误检查，通过。
- `unresolved_items`：R3~R5 的能力确认仍未完成，尤其是编辑格式×操作子集、联网研究档位、文件结构变更确认/恢复、外部文件深读隐私和长期记忆边界；这些均不由 R2 隐式冻结。既有 `DIFF-STATE-COMPASS-01` 的 DOC-CONVERT 环境失败仍保持独立 `BLOCKED`，未在本 Scope 中处理。
