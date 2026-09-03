---
文档编号：CORE-X-D-02
文档状态：A
负责模块：CORE
文档职责：Binder Agent 1.0 冻结能力基线——固定已确认的产品需求范围、能力档位、非目标、变更入口与进入 DESIGN 的门禁
上游约束：A-CORE-C-P-01_APC.md、A-CORE-C-D-01_产品概述.md、2026-08-14 人类 R6 冻结确认及后续已记录的人类确认
直接承接：后续 SYS、DE/ED/WS、AG/REF/AST 的 DESIGN Scope；本文件不替代各模块 M-D 的 REQ 或 L2 技术设计
使用边界：本文只冻结已确认的 Agent 1.0 需求范围；不新增 REQ、不定义 RULE/CHAIN/CONSTRAINT/TERM、状态机、接口、Prompt、工具或实现算法；代码不得直接映射本文
变更要求：任何产品行为、权限、档位、保存/隔离/隐私语义变化，必须新开 REQUIREMENT Scope 并同步产品根、能力基线、索引与受影响 L1；不得在 DESIGN/IMPLEMENTATION 中隐式改写
治理成熟度：baseline
---

# Binder Agent 1.0 冻结能力基线

## §1 冻结声明

**冻结版本：Agent 1.0 Baseline v1.0（2026-08-14）**

本版本冻结 R1 至 R5-C 已获得人类确认、并已同步到 L1 的 Agent 1.0 需求范围。它是后续 DESIGN 的
产品输入边界：设计只能细化，不能扩大或收缩已确认的用户可见行为。

冻结的是**需求范围**，不是实现状态。任何 P0/P1 能力在完成 L2 映射、状态机/接口（如适用）、验证设计和真实
运行验证前，均不得表述为“已可用”“已发布”或“全链完成”。

## §2 权威阅读顺序

```text
APC（项目边界）
  └─ 产品概述（产品级 REQ-CORE-*）
       └─ 本文件（Agent 1.0 冻结范围、档位与变更规则）
            ├─ 需求基线与文档体系索引（只做导航）
            └─ 各模块 Active M-D（具体 REQ）
                 └─ L2 / 状态机 / 接口 / 验证 / 实现
```

存在冲突、缺口或需要改变行为时，不以旧 L2、现有代码、Prompt、工具注册或 UI 文案裁决；必须新开
REQUIREMENT Scope，由产品根与能力基线重新确认后再传播。

## §3 冻结能力范围

| 档位 | 已冻结范围 | 解释 |
|---|---|---|
| P0 主链 | 三态模型与内容 Diff 生命周期；CreateDocumentBundle；工作区内显式引用的 data-only 内容与用户可见身份；基础聊天、项目/编辑器感知、受限阅读分析；原生 DOCX 只读与 BDocx 副本路径；`agent-edit-1.0` Runtime Profile、AgentTurn、可更新的 IntentResult 语义投影、运行时动态 CapabilityGrant、结构化可见事件、AgentSession 隔离与本地事实历史；复杂任务的 TaskRun/TaskPlan 可观察投影、真实进度、连续 Proposal 接力；由真实读取证据派生的 ProjectReport；按可独立裁决连续块组生成 DiffCard | 模型负责语义、工具选择和路线调整，运行时负责事实、授权、预算、门禁、副作用与终态；用户显式引用身份可作为既有文档目标授权元数据，但正文仍是 data-only；成员资格不替代 CapabilityGrant、基线和 Diff 裁决；仍须经过 DESIGN 和真实验证 |
| P1 受控开放 | HTML/BDocx 的颜色/突出显示、段落样式、字号、字体族、表格/图片；显式公开网页研究；WorkspaceOperation 的创建/重命名/移动/删除/恢复与 WorkspaceTimeline 类型化恢复；TXT→Markdown、Markdown→TXT 目标副本转换；确认冻结清单后的多个既有文档逐项编辑（用户输入显式引用清单，或工具发现目标经 scope DecisionRequest 确认）；用户显式外部文件的一次性只读内容引用；用户管理且显式选择的本地项目记忆 | 每项必须按具体子能力、格式/解析能力、最小授权、确认、基线再核验、补偿、恢复语义与真实验证独立开放，不能一并默认开启 |
| P2 后续 Profile | Build/CBT、Agent 记忆提议/自动写入/自动检索/自动注入、自动知识召回、模板工作流、开放式自主规划、多 Agent、任务队列、任务级事务/回滚 | 保留产品路线，不进入 `agent-edit-1.0` 默认 Profile 或 1.0 发布验收 |

## §4 不可突破的产品边界

1. 文档正文、样式、表格和图片修改先作用于 LogicalState，继续遵循内容 Diff 三态分流；WorkspaceOperation
   不属于内容修改，必须先由 DecisionRequest 前置裁决，确认且再核验后直接改变 DiskState，不产生资源 Diff。
2. IntentResult 是模型依据输入和工具证据持续更新的语义投影，只表达目标、目标候选、副作用、上下文需求、歧义与确认点，不包含或等价承载工具授权。CapabilityGrant 由运行时依据 Profile、工具注册、ExplicitEditTargetSet、格式/动态能力、DecisionRequest、Gate、预算和终态逐次计算；引用正文、关键词分类、Prompt、模型请求、已注册工具、只读工具结果、打开标签或兼容默认值都不得扩权。
3. TaskRun/TaskPlan 只在复杂或跨执行片段任务中，作为模型当前路线与 append-only 事件事实的可观察投影；不作为静态调度器、未来工具预授权或后台队列。模型可按新证据调整未执行路线，但不能跨确认点自动续跑。同文档连续 Proposal 以最新 LogicalState 接力，不形成补丁栈或任务级自动回滚。
4. 可见“工作中”只来自真实回合、provider 或工具事件；路线与阶段是可更新投影，不展示模型内部推理，也不把等待、失败、取消或中断伪装为执行。项目读取覆盖、来源和不确定性必须由运行时依据真实工具证据派生，不接受模型自报替代事实。
5. 外部文件内容仅在用户显式附加且有可用只读解析路径时，作为当前任务的一次性 data-only 引用；不得扫描、自动复读、默认持久化或转存为项目记忆。
6. 项目长期记忆是用户管理、本地 workspace 隔离的 P1 资产；只有用户明确选择才进入当前回合。Agent 1.0 不自动写入、提议、检索或注入。
7. WorkspaceOperation 批量失败即停并反向补偿，补偿不完整如实进入 partial_failed；统一 WorkspaceTimeline
   保存内容节点和资源节点，但按类型化还原器执行，不以文档快照冒充文件树恢复。
8. CreateDocumentBundle 的绑定初始 Diff 只有在资源、DiskState 与 LogicalState 基线均未推进时，拒绝才可删除
   本次创建文件；普通 Diff 永远不得删除资源。
9. 同一 ProposalBatch 内，每个不连续或可独立接受/拒绝的块组生成独立 DiffCard；相邻且共同表达一个不可分修改的块可合并。分组只改变人工裁决粒度，不绕过 Proposal 基线、预览、失效和显式保存语义。

## §5 冻结后的变更规则

| 变化类型 | 正确入口 | 不允许的替代方式 |
|---|---|---|
| 新能力、范围扩大、P0/P1/P2 调整 | 新 REQUIREMENT Scope → 人类确认 → 产品根/能力基线/索引/模块 L1 同步 | 在 L2、Prompt、工具 manifest、feature flag 或代码中先实现后补文档 |
| 已冻结行为的技术细化 | DESIGN Scope，在不改变 §3/§4 语义的前提下形成 REQ_MAP、RULE/CHAIN/CONSTRAINT、状态机和接口 | 用字段、状态名或兼容路径隐式修改保存、授权、隔离、隐私或裁决语义 |
| 发现实现与冻结基线不一致 | DESIGN/IMPLEMENTATION Scope 中登记迁移债务；如需改变产品行为则回到新 REQUIREMENT Scope | 让旧代码、局部测试或旧 L2 反向定义产品需求 |

## §6 进入 DESIGN 的有序输入

1. SYS：定义 append-only AgentTurn 事件与共享投影边界——Runtime Profile、AgentTurn、AgentSession、TaskRun/TaskPlan 投影、Proposal 链、外部内容与显式记忆隔离；不先锁定存储实现细节。
2. DE + ED + WS：保持内容三态分流，定义 ProposalBatch 与独立 DiffCard 分组；资源操作按 A-WS-M-T-02/A-WS-M-S-02 定义前置裁决、再核验、直接写盘、补偿、统一时间轴和 CreateDocumentBundle 接口。
3. AG + REF + AST：定义可更新 IntentResult、运行时 CapabilityGrant、工具证据账本、派生 ProjectReport、受限工具消费、引用快照、外部内容暂存/清除、显式记忆选择和 Trace 契约。
4. 为每项状态化链路和 P1 开放条件建立验证卡；只有存在可执行证据，才进入 IMPLEMENTATION Scope。

## §7 真实性与验收等级

| 结论 | 当前状态 |
|---|---|
| 需求已确认 | 是：R1–R5-C 已完成产品/L1 同步，本文件冻结其范围 |
| 技术设计已完成 | 否：需按 §6 新开 DESIGN Scope |
| 代码已符合 | 未判定：不得从现有实现、工具或 Prompt 推断 |
| 真实运行验证通过 | 未判定：P0/P1 均需实施后按验证卡获得证据 |
| Agent 1.0 可发布 | 未判定：取决于实现、验证和发布验收，不由本文件宣布 |

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-14 | v1.0 | AG-1.0-R6-BASELINE-FREEZE-01：人类确认冻结 Agent 1.0 已确认需求范围；汇总 R1–R5-C 的 P0/P1/P2、不可突破边界、变更入口、DESIGN 顺序与需求确认/实现验证的真值区分。 |
| 2026-08-21 | v1.1 | WORKSPACE-OPERATION-BASELINE-01：人类确认 WorkspaceOperation 八项决策；冻结前置裁决直接写盘、统一 WorkspaceTimeline、CreateDocumentBundle、安全拒绝补偿、fail-fast 补偿和 dirty 阻断。 |
| 2026-08-28 | v1.2 | AG-CONFORMANCE-REQUIREMENT-S3D-COMPANION：按 HD-S3D-03 分离显式引用内容的 data-only 属性与用户可见引用身份的目标授权元数据属性；多既有文档编辑仍为 P1，须由输入显式冻结清单或 scope DecisionRequest 确认。 |
| 2026-08-31 | v1.3 | AG-DOC-CONSOLIDATION-01：确立本文为 Agent 1.0 唯一冻结范围真源；候选 AG-M-D-02 降为 R 参考，消除候选基线与本文互相上游的循环。 |
| 2026-09-03 | v1.4 | AG-ADAPTIVE-RUNTIME-REQ-01：不改变能力档位，收敛模型/运行时职责；IntentResult 语义投影化、CapabilityGrant 动态化、TaskRun/TaskPlan 观察化、ProjectReport 证据派生化，并冻结可独立裁决的 DiffCard 分组原则。 |
