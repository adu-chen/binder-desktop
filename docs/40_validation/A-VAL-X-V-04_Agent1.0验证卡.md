---
文档编号：VAL-X-V-04
文档状态：A
负责模块：CORE
文档职责：Agent 1.0 唯一验证卡与证据状态入口——把冻结需求、设计、状态机和实施工作包映射为可复核的自动化与真实系统证据
上游约束：A-CORE-X-D-02、A-AG-M-D-01、A-AG-M-T-01、A-AG-M-T-02、A-AG-M-S-01、A-AG-I-P-01、A-SYS-I-P-01、A-DE-M-T-01/A-DE-M-S-01/A-DE-I-P-01、A-ED-M-T-01/A-ED-M-S-01/A-ED-I-P-01、A-WS-M-T-02/A-WS-M-S-02/A-WS-I-P-01
直接承接：A-AG-X-L-04、WP-AG10-01～09 IMPLEMENTATION Scope、G5 系统验证、产品能力 system_validation 回填
使用边界：卡状态只能由对应提交的实际 Runtime Report 与可复核证据推进；历史实现、旧测试数量、代码存在、计划完成或单层 mock 均不能提升当前状态；本文不新增产品需求、规则、术语或状态机
变更要求：冻结语义、状态机、协议或证据门禁变化时必须回写；未实际执行不得写 PASS
---

# Agent 1.0 验证卡

## §1 当前裁决

**P00 补充（2026-09-15）**：下表及 §10～§12 保留的是 v1 修复阶段的历史证据，不证明
AgentContextFacts v2 或 M1/M2/M3。新增修复验收以 §14 为准，全部从 `DESIGNED` 开始；
设计/治理检查通过只能关闭 P00，不能把产品卡提升为 `AUTOMATED` 或 `PASS`。原 G5 仍为 `BLOCKED`。

| 字段 | 内容 |
|---|---|
| validation_id | `VAL-AG-1.0-CORE` |
| phase | DESIGN；卡执行属各 IMPLEMENTATION Scope 与最终 VALIDATION/G5 |
| owner / participants | CORE / AG、SYS、DE、ED、WS、ENT、GW、REF |
| 当前状态 | V-C1～C3、V-R1/R2、V-S1～S14 全部 `AUTOMATED`；当前无 `SYSTEM_PASS` 或 `PASS` 卡 |
| 自动化依据 | WP1～09 原 READY Runtime Report；AG-CLOSURE-REPAIR-01 当前工作树同输入完整回归在正常系统权限下通过：前端 611/611、Rust lib 402/402、core_chain 5/5、sm_broadcast 4/4、TypeScript 与治理审计通过 |
| G5 当前结果 | `BLOCKED`：当前协议真实 DeepSeek 闭环、真实应用库 v20 迁移及 `empty.txt` 的 WKWebView 问候已通过；但 PPT active session 长期 loading、普通问候附带大段 ProjectReport 噪声，且 DiffCard 裁决、等待/重启和隔离操作尚未执行 |
| IMPLEMENTATION 门禁 | §8：Scope 声明卡 ID、规则/状态机、changed_files 和证据层；治理审计通过 |
| G5 退出条件 | 发布阻断卡同时具备当前提交的自动化证据与真实 Tauri/WKWebView/provider/workspace 证据 |

状态口径：

- `DESIGNED`：场景、断言和所需证据已定义，当前实现尚未按本契约重新证明；
- `AUTOMATED`：当前提交的模块/契约/集成自动化通过，但未完成必需真实系统观察；
- `SYSTEM_PASS`：真实系统场景通过，但完整影响自动化尚未闭合；
- `PASS`：当前提交的自动化和真实系统证据都闭合；
- `BLOCKED`：环境、依赖或缺陷阻断，并有明确恢复动作。

任一证据失效或冻结契约变化时，卡回到能够被现有证据真实支持的状态，不允许“只升不降”。

## §2 追溯矩阵

| 验证意图 | 需求 | 规则/约束 | 状态机/协议 | 验证卡 |
|---|---|---|---|---|
| 内容三态、预览、裁决、保存 | REQ-CORE-002/010、REQ-DE-001~003、REQ-ED-005、REQ-WS-003 | BR-CORE-STATE-001~003、BR-DE-DATA-001~003、X-FORBID-006/008/009 | SM-DE-01、SM-ED-01、SM-WS-01/02 | V-S1～S5 |
| AgentTurn、事件、澄清、取消、终态 | REQ-AG-007、REQ-SYS-010 | BR-AG-STATE-001、BR-SYS-STATE-007 | SM-AG-01/02、AgTurnEvent/TurnEvent | V-S6、V-S12 |
| 复杂任务可观察投影 | REQ-AG-015、REQ-SYS-011 | BR-SYS-STATE-008 | SM-AG-04、TaskRun/TaskPlan/TaskStep | V-S7 |
| 外部内容、显式记忆与会话隔离 | REQ-AG-016/017、REQ-SYS-009/010/012 | BR-SYS-DATA-007、BR-SYS-STATE-007 | ExternalContentRef/MemorySelectionRef/AgentSession | V-S8、V-S9 |
| 编辑目标和动态能力授权 | REQ-AG-001/007/015 | BR-AG-DATA-003、BR-AG-STATE-002、BR-SYS-STATE-007/008 | ExplicitEditTargetSet/CapabilityGrant | V-S10、V-S11 |
| 当前上下文、读取证据和项目报告 | REQ-AG-009 | BR-AG-DATA-008、BR-AG-OBS-001、BR-SYS-DATA-008 | AgentContextFacts/EvidenceLedger/ProjectReport | V-S13 |
| 同批独立卡片与跨批接力 | REQ-AG-001/015、REQ-DE-001/009、REQ-SYS-011 | BR-DE-DATA-001~003、BR-DE-STATE-001/002、BR-SYS-STATE-008 | ProposalBatch/SM-DE-01/SM-ED-01 | V-S4、V-S14 |
| CreateDocumentBundle | REQ-AG-008、REQ-ED-010、REQ-WS-006 | BR-WS-STATE-003、BR-WS-DATA-003 | SM-WS-04、SM-DE-01、SM-ED-01 | V-C1～C3 |
| WorkspaceOperation | REQ-AG-011/013、REQ-WS-008/010、REQ-CORE-012/014 | BR-WS-STATE-003、BR-WS-PERSIST-003 | SM-WS-04、DecisionRequest | V-R1/R2 |

## §3 通用证据要求

每张卡必须同时记录：

1. 提交、Scope ID、Runtime Report 和测试环境；
2. 自动化：生产 owner、直接消费者、主路径、拒绝/错误/恢复、状态机或契约测试；
3. 真实系统：用户操作、可见结果、事件序列/脱敏 trace、磁盘或数据库事实；
4. 负空间：不应发生的 provider、工具、Proposal、文件写入和跨会话数据均为零；
5. 缺陷与重测：失败归属工作包，修复后重跑 related tests 与受影响系统场景。

固定模型文本不是通用验收前提。工具选择/结构协议用确定性 provider harness 证明，真实交互质量至少用一个真实
provider 另行证明；二者不可互相替代。

## §4 内容、资源与保存卡

### §4.1 V-C1 · CreateDocumentBundle 拒绝

| 字段 | 内容 |
|---|---|
| 操作 | 请求新建带 AI 初始内容的文档；DecisionRequest 出现后拒绝 |
| 期望 | 确认前文件、标签、内容 Diff 均为零；拒绝后计划终止，DiskState 不变，AgentTurn 如实结束或继续其他已允许路线 |
| 自动化 | plan→cancel 零副作用；重复/错误身份裁决失败关闭；UI 无伪标签/卡 |
| 系统观察 | 文件树、标签和磁盘均无目标；卡片说明资源影响 |
| 状态 | AUTOMATED |

### §4.2 V-C2 · CreateDocumentBundle 确认与投影

| 字段 | 内容 |
|---|---|
| 操作 | 确认有效计划，观察资源创建、标签和初始内容 Diff，接受并保存 |
| 期望 | 运行时在同一 AgentTurn 重验后调用 WS；先建立真实目标，再打开标签并投影普通内容 Diff；接受不重复创建，保存后重开一致 |
| 自动化 | 调用顺序、身份、幂等、operation receipt、Proposal origin 与保存链 |
| 系统观察 | 文件先出现，初始内容卡随后出现；无第二次资源确认 |
| 状态 | AUTOMATED |

### §4.3 V-C3 · CreateDocumentBundle 安全拒绝

| 字段 | 内容 |
|---|---|
| 操作 | A：初始内容预览后直接拒绝；B：先推进资源/LogicalState/DiskState 任一基线再裁决旧卡 |
| 期望 | A 仅在三重基线未变时执行专用补偿；B 旧卡失效但不删除当前文件、标签或内容 |
| 自动化 | 三重 guard、补偿幂等、基线推进和错误恢复参数化测试 |
| 系统观察 | 安全拒绝可恢复；发生后续工作时绝不误删 |
| 状态 | AUTOMATED |

### §4.4 V-R1 · WorkspaceOperation 重验与 dirty 阻断

| 字段 | 内容 |
|---|---|
| 操作 | 对带 dirty 标签、pending 内容 Diff 或引用的资源生成删除/重命名/移动计划并裁决 |
| 期望 | 确认前零副作用；运行时确认后重新计算 grant 并让 WS 重验；dirty 阻断；成功操作后相关 Diff/引用按真实结果失效或重绑 |
| 自动化 | DecisionRequest 身份、grant 重算、WS revalidate、DE/ED/REF 消费者和磁盘断言 |
| 系统观察 | 卡片影响范围与真实文件树一致；无模型直调低层文件命令 |
| 状态 | AUTOMATED |

### §4.5 V-R2 · WorkspaceOperation 补偿、中断与恢复

| 字段 | 内容 |
|---|---|
| 操作 | 批量计划中注入中段失败、补偿失败和进程重启；从 WorkspaceTimeline 发起恢复 |
| 期望 | fail-fast、逆序补偿、逐项 receipt；补偿不完整为 PartialFailed；重启为 Interrupted 且不续跑；恢复重新计划和确认，冲突不覆盖 |
| 自动化 | SM-WS-04 全转移、幂等、故障注入、重启、文件/目录类型化恢复 |
| 系统观察 | 实际完成/补偿项清晰；只有一套恢复入口 |
| 状态 | AUTOMATED |

### §4.6 V-S1 · 已有 LogicalState 的预览、裁决和保存

| 字段 | 内容 |
|---|---|
| 操作 | 对已打开且有未保存内容的文档生成 Proposal，预览、接受、保存、重开 |
| 期望 | 以当前 LogicalState 为基准；预览后卡仍 pending；接受只裁决不重复应用；保存才更新 DiskState；重开一致 |
| 自动化 | DE/ED 三态、未保存基线、save receipt、重开读取 |
| 系统观察 | 绿增、卡状态和磁盘时点一致 |
| 状态 | AUTOMATED |

### §4.7 V-S2 · 无 LogicalState 的后台 Proposal

| 字段 | 内容 |
|---|---|
| 操作 | 对未打开既有文档形成 Proposal，分别接受和拒绝 |
| 期望 | 接受前磁盘不变；接受才原子落盘并产生一个时间轴节点；拒绝/过期零写入 |
| 自动化 | background 分流、原子写、重复裁决和失败恢复 |
| 系统观察 | 卡片目标明确，接受后真实文件改变 |
| 状态 | AUTOMATED |

### §4.8 V-S3 · pending 打开迁移

| 字段 | 内容 |
|---|---|
| 操作 | 后台 pending 存在时打开目标，再裁决和保存 |
| 期望 | 同一 payload 挂载到 LogicalState 并保持 pending；不再走后台落盘；身份和终态不重复 |
| 自动化 | pendingDiffPromoted、路径切换、恢复和保存断言 |
| 系统观察 | 打开后可见预览，卡仍可裁决 |
| 状态 | AUTOMATED |

### §4.9 V-S4 · 跨 ProposalBatch 接力

| 字段 | 内容 |
|---|---|
| 操作 | 先生成 Batch1，随后在当前 LogicalState 上生成 Batch2；另对 background pending 尝试续编 |
| 期望 | Batch2 只读取最新 LogicalState，不叠加旧 payload；新批预览可按既有基线规则使旧批卡失效；background pending 未迁移/裁决前不得作为编辑输入 |
| 自动化 | 跨批 base revision/hash、旧卡 expiry、拒绝无任务级回滚、background guard |
| 系统观察 | 新旧批关系清楚，旧卡不可误裁决 |
| 状态 | AUTOMATED |

### §4.10 V-S5 · 保存 pending 与保存失败

| 字段 | 内容 |
|---|---|
| 操作 | 预览后未点接受直接保存；另制造磁盘冲突/保存失败 |
| 期望 | 保存先把有效 pending 裁决为 accepted/awaiting_save 再落盘；失败保留 dirty 与待保存事实，不覆盖外部内容，不伪报成功 |
| 自动化 | 保存即接受、冲突、失败重试、状态恢复 |
| 系统观察 | 卡片、dirty、错误和磁盘一致 |
| 状态 | AUTOMATED |

## §5 Agent 运行时卡

### §5.1 V-S6 · TurnEvent、澄清、取消与首终态

| 字段 | 内容 |
|---|---|
| 操作 | 触发模型澄清并用原身份补充；测试错误身份/重复恢复；在组装、provider、工具和等待阶段取消；注入终态后迟到事件 |
| 期望 | 澄清恢复原 AgentTurn；等待不显示工作中；取消停止未开始动作；TurnEvent seq 单调；首终态后迟到事件无卡片、文档或磁盘副作用 |
| 自动化 | append/replay/乱序/身份/取消竞态/terminal_latch；前端相同事件重建相同状态 |
| 系统观察 | 补充不新建 turn；停止后无新 token/tool/proposal；终态原因唯一 |
| 状态 | AUTOMATED |

### §5.2 V-S7 · TaskRun/TaskPlan 可观察投影

| 字段 | 内容 |
|---|---|
| 操作 | 运行简单回答、单次编辑、复杂多步骤目标；模型在 ToolResult 后修订路线；触发 DecisionRequest；删除/重建投影；重启和重试 |
| 期望 | 简单目标零 TaskRun；复杂任务从事件派生 TaskRun/TaskPlan；候选步骤不自动执行、不授予权限；路线可修订；Decision 恢复原 AgentTurn 并重验；重启不续跑，重试新建 turn |
| 自动化 | 事件重建、简单/复杂、route revision、等待无 activity、投影删除无业务副作用、Decision 身份与同回合恢复 |
| 系统观察 | 当前路线和已完成事实真实；无静态游标/后台推进；详见 A-VAL-X-V-05 |
| 状态 | AUTOMATED |

### §5.3 V-S8 · 外部内容与显式记忆 data-only

| 字段 | 内容 |
|---|---|
| 操作 | 显式附加可读/不可读外部文件，显式选择项目记忆，结束/重启/换会话后再次请求 |
| 期望 | 仅当前任务消费带来源的一次性内容；不可读只报元数据；不产生目标、授权或写入；记忆不自动检索、注入、创建或更新 |
| 自动化 | 引用生命周期、来源/预算、Prompt 注入隔离、零 AST/KB/TPL 自动调用 |
| 系统观察 | 已读/未读清楚，无隐式记忆行为 |
| 状态 | AUTOMATED |

### §5.4 V-S9 · session/workspace 隔离与恢复

| 字段 | 内容 |
|---|---|
| 操作 | 两个 workspace、多个 AgentSession 交错运行，包含引用、未保存内容、证据、Decision 和 Diff；重启恢复 |
| 期望 | 事件、上下文、目标、证据、任务投影和卡片不串线；活动回合恢复为 Interrupted 或原等待/终态，不自动执行 |
| 自动化 | 复合身份键、持久化查询、事件补流、前端 store 隔离 |
| 系统观察 | 切换与重启后的信息流和文档状态一致 |
| 状态 | AUTOMATED |

### §5.5 V-S10 · ExplicitEditTargetSet 双守

| 字段 | 内容 |
|---|---|
| 操作 | 当前文档/显式 DocRef 初始化目标；澄清追加；scope Decision 分别确认/取消/重放；尝试用模型文本、历史路径、只读结果、外部内容或其他 workspace 目标提案 |
| 期望 | 只有用户可见且归一到当前 workspace 的身份可进入集合；确认原子追加，取消零追加；AG/DE 在 PendingDiff 前双重拒绝集合外目标；data-only 内容不扩权 |
| 自动化 | 初始化、单调追加、持久化、Decision 身份/重放、CreateDocument 建盘后追加、AG/DE 双守和零 PendingDiff |
| 系统观察 | 集合外提案明确阻断，无隐藏授权 |
| 状态 | AUTOMATED |

### §5.6 V-S11 · 模型语义与动态 CapabilityGrant

| 字段 | 内容 |
|---|---|
| 操作 | 使用同义/含糊/否定/多轮修正请求；让模型先读后改线；分别请求只读、内容提案、资源计划和越权工具；在目标、格式、Gate、确认、预算变化后重试 |
| 期望 | IntentResult 由模型输出并可 revision，不含 allowed_tools；本地提示不能冒充最终语义；每次 ToolCall 前按当前硬事实新建 grant，动作后失效；注册、Prompt、TaskPlan 和历史均不授权 |
| 自动化 | 结构化模型输出、revision、硬事实组合/收窄、grant 过期、格式动态能力、未知/未授权/参数/目标/workspace 越界和重复失败不扩权 |
| 系统观察 | Agent 能根据证据调整路线；工具可见/执行与当前事实一致；拒绝不导致僵死或偷偷扩权 |
| 状态 | AUTOMATED |

### §5.7 V-S12 · 自适应模型—工具循环与真实事件

| 字段 | 内容 |
|---|---|
| 操作 | 让模型连续执行读取→ToolResult→第二次 provider→另一工具/最终回答；分别注入每次 Gate deny、Prompt 超预算、工具失败、循环上限、取消、Decision 等待 |
| 期望 | 每次 provider 前重装配并 Gate；每次工具前重算 grant；ToolResult 进入事件/上下文后模型可改线；等待无活动；错误/限额/取消真实终止；Proposal/Decision/final 各走唯一合法路径 |
| 自动化 | 确定性 provider 脚本、调用计数/顺序、Gate 与 grant 次数、事件 replay、失败传播和首终态 |
| 系统观察 | 工具卡顺序和当前动作与真实调用一致，不出现固定流程或完成后继续活动 |
| 状态 | AUTOMATED |

### §5.8 V-S13 · EvidenceLedger 与运行时派生 ProjectReport

| 字段 | 内容 |
|---|---|
| 操作 | 分别读取目录、搜索命中、结构摘要、部分块和全文；注入失败/跨 workspace 读取；最终回答后查看报告 |
| 期望 | 仅成功读取进入 ledger，coverage 类型不互相冒充；evidence 与 turn/source/version/seq 绑定；ProjectReport 由最终回答和 ledger 派生，披露已读/未读/来源/不确定性；无 report_project 工具 |
| 自动化 | evidence append/replay、coverage、失败/跨域、最终 through_seq、工具注册表/Prompt 无 report_project |
| 系统观察 | 报告与实际工具卡一致，未读内容不伪装已分析 |
| 状态 | AUTOMATED |

### §5.9 V-S14 · ProposalBatch 同批独立 DiffCard

| 字段 | 内容 |
|---|---|
| 操作 | 对当前文档的两个不连续块和一个连续块组在一次模型提案中修改；分别接受第 1 卡、拒绝第 2 卡；覆盖乱序/缺组/重叠目标/预检失败/重启；再生成下一批 |
| 期望 | 一个 ProposalBatch 产生按连续且可独立裁决目标拆分的多卡；前端完整收齐后一次 ED 前向事务；每卡局部 fingerprint/inverse；裁决一张不改变兄弟卡；批内无 document_revision_advanced；下一批才按最新 LogicalState 触发旧批失效 |
| 自动化 | group id/index/count、完整性、非重叠、原子预检、一次事务、局部 accept/reject、兄弟 pending、跨批 expiry、持久化/恢复；不支持 kind 强制单卡 |
| 系统观察 | 不连续修改出现多张卡，可独立裁决；连续同块修改不被机械拆碎；最终内容和磁盘正确 |
| 状态 | AUTOMATED |

## §6 工作包—验证卡矩阵

| 工作包 | 必须达到 AUTOMATED 的卡 | 必须在该包做的负向证明 |
|---|---|---|
| WP-AG10-01 | V-S6、V-S9、V-S12 的事件部分 | 乱序、重复、终态迟到、跨身份事件不改变事实 |
| WP-AG10-02 | V-S6、V-S11、V-S12 | 关键词不授权、后续 provider 不复用旧 Gate、等待无执行 |
| WP-AG10-03 | V-S10、V-S11、V-C/V-R 授权部分 | 过期/越权/未注册/目标外/参数错/模型可见 WS 执行均拒绝 |
| WP-AG10-04 | V-S8～S12 的上下文/Prompt 部分 | 内容提示注入、旧 DiskState、跨 session/workspace、旧 grant 缓存无效 |
| WP-AG10-05 | V-S13 | 失败、目录、搜索、跨域证据不能冒充全文已读；无报告工具 |
| WP-AG10-06 | V-S7、V-C/V-R 的 Decision 部分 | 简单回合零 TaskRun、候选步骤/投影不能启动动作、重启不续跑 |
| WP-AG10-07 | V-S1～S5、V-S14 | 批内整篇 hash 不使兄弟失效；重叠/缺组整批不预览 |
| WP-AG10-08 | V-S6、V-S7、V-S9、V-S14 的 UI/恢复部分 | UI 不猜终态/工作态，旧数据不自动执行或继续写旧格式 |
| WP-AG10-09 | 全部受影响卡 | 禁用标识和旧入口零生产消费者；更新测试不弱化覆盖 |

## §7 G5 场景与发布阻断

| 场景 | 最少卡 | 发布阻断事实 |
|---|---|---|
| 只读回答/分析 | V-S11～S13 | 主动读取、证据真实、零写工具/Proposal |
| 当前文档编辑 | V-S1、V-S5、V-S10～S12 | LogicalState 正确、先 Proposal、裁决与保存分离 |
| 不连续块独立裁决 | V-S14、V-S5 | 多卡一次预览、逐卡 accept/reject、保存重开正确 |
| 跨批继续修改 | V-S4 | 新批以最新 LogicalState 为基准，旧批失效正确 |
| 澄清/Decision/复杂任务 | V-S6、V-S7、V-S12 | 同回合恢复、逐次 Gate/grant、无后台 scheduler |
| 新建和资源操作 | V-C1～C3、V-R1/R2 | 确认前零副作用、运行时重验、失败/补偿/恢复真实 |
| 错误/取消/重启 | V-S5、V-S6、V-S9、V-S12 | 唯一终态、无原文破坏、无自动续跑 |
| 隔离与隐私 | V-S8、V-S9、V-S13 | 不串线；正文/密钥/隐藏推理不进安全事件或默认 trace |

## §8 进入 IMPLEMENTATION 与更新卡状态

每个实现 Scope 必须：

1. 声明工作包和本表卡 ID；列出相关 REQ、RULE、CHAIN、STATE_MACHINE 与预计生产 owner/消费者；
2. 读取测试治理注册表和 ADUS，使用实际 changed_files 运行 `test:plan`，不得手选缩减；
3. 对被替代实现列删除清单；删除旧测试前必须有当前契约的等价或更强断言；
4. 运行 `test:related`、治理审计和工作包专项测试；需真实系统验证的卡在自动化通过后只标 `AUTOMATED`；
5. 把命令、数量、环境、失败和恢复写入 Runtime Report，再更新卡状态；没有报告不更新；
6. 卡达到 `PASS` 后若相关契约或生产 owner 改变，自动回到 `DESIGNED`/`AUTOMATED` 并重验。

## §9 P1 独立开放条件

下列能力不得因核心 Agent 通过而整体开放；每项需要独立 Scope、动态能力和真实验证。未满足时保持关闭：

| P1 能力 | 独立条件 | 复用卡 |
|---|---|---|
| TXT/Markdown 目标副本转换 | 来源不改写、CreateDocumentBundle、损失告知、三重基线安全拒绝 | V-C1～C3 |
| 外部文件内容读取 | 用户显式附加、可用只读解析、一次性 data-only、未读披露 | V-S8、V-S13 |
| 显式项目记忆选择 | 仅用户选择进入本轮，不自动检索/写入/注入 | V-S8、V-S9 |
| Workspace 创建/重命名/移动 | Decision、重验、dirty 阻断、幂等/补偿 | V-R1/R2 |
| 删除/恢复 | 冻结递归范围、类型化快照、Timeline 恢复、冲突不覆盖 | V-R1/R2 |
| 多既有文档逐项编辑 | 冻结目标清单，每目标独立基准/Proposal/裁决/保存，无批量事务承诺 | V-S10 + 独立子卡 |
| 样式/表格/图片 | 受控操作、目标局部逆操作、格式动态能力、保存重开逐项通过 | V-S11 + 独立子卡 |

## §10 当前待验证清单

| validation_id | 当前可复用素材 | 当前必须重新证明 | 状态 |
|---|---|---|---|
| VAL-AG-ADAPTIVE-RUNTIME-01 | WP1/WP2/WP4 当前 Runtime Report；完整回归 | TurnEvent→模型→ToolResult→再调用→final 的真实 UI 顺序与故障观察 | AUTOMATED |
| VAL-AG-DYNAMIC-GRANT-01 | WP3 当前 Runtime Report；授权/越权/失效自动化 | 真实模型请求在当前事实变化后的工具可见性与拒绝体验 | AUTOMATED |
| VAL-AG-EVIDENCE-REPORT-01 | WP5 当前 Runtime Report；EvidenceLedger/ProjectReport 自动化 | 真实 UI 报告与工具卡、已读/未读和不确定性一致 | AUTOMATED |
| VAL-AG-PROPOSAL-BATCH-01 | WP7/WP8 当前 Runtime Report；批次原子性与 UI reducer 自动化 | 真实不连续块多卡、逐卡裁决、保存重开 | AUTOMATED |
| VAL-AG-REAL-EXPERIENCE-01 | 当前协议 DeepSeek Chat/Reasoner 合成问候、`read_doc → ToolResult → final` 与 5 次问候抽样；真实应用库/窗口只读证据 | 当前构建中的真实输入、拒绝纠正、工具卡顺序、DiffCard 裁决、状态诚实和交互流畅性 | BLOCKED |
| VAL-DOC-LIBREOFFICE-01 | 2026-09-03 非沙箱完整 Rust 套件通过 | 仅在后续 DOC/发布环境变更时回归，不是 Agent 当前阻断 | VERIFIED_NON_SANDBOX |

## §11 2026-09-06 G5 执行记录

| 字段 | 结果 |
|---|---|
| 提交 / 环境 | `e86f492`；macOS；Binder Tauri debug + WKWebView；本地 dev-gateway 8787 |
| 自动化 | `AUTOMATED`：WP1～09 READY；最终同输入 `test:related` 在正常系统权限下前端 608/608、Rust lib 394/394、core_chain 5/5、sm_broadcast 4/4、TypeScript 与治理审计通过 |
| 真实 provider smoke | `PASS`：DeepSeek 收到不含用户数据的合成“先检查 notes/a.md 再总结”请求后，主动发出 `read_doc`；回注合成 ToolResult 后继续流式输出基于证据的回答并正常 done |
| Tauri 启动 | `PASS`：复用本机 1420 Vite 与 8787 网关，`target/debug/binder` 成功运行 |
| 真实 UI 场景 | `BLOCKED`：Computer Use 服务在应用启动前后连续失败，无法操作或观察 Binder 窗口；未执行 §7 的 DiffCard、等待态、Decision、重启和多 session/workspace 场景 |
| 负空间裁决 | 网关 smoke 不等于 Binder runtime/Tauri 系统通过；没有把自动化或 provider 直连提升为 `SYSTEM_PASS` |
| 恢复动作 | Computer Use 恢复后，或人类按本文 §7 与 A-VAL-X-V-05 提供目标窗口证据后，从同一提交继续 G5；若发现缺陷，回退到对应 WP，修复后重跑 exact changed-files tests 和受影响系统场景 |
| 最终裁决 | `BLOCKED`；19 张卡保持 `AUTOMATED`，零张 `SYSTEM_PASS` / `PASS` |

## §12 2026-09-08 AG-CLOSURE-REPAIR-01 自动化记录

| 字段 | 结果 |
|---|---|
| Scope / 环境 | `AG-CLOSURE-REPAIR-01`；macOS 当前工作树；真实用户库只读定位，迁移验证使用隔离 SQLite 夹具 |
| 修复事实 | v20 兼容真实七列 `turn_events` 并从 `agent_turns` 恢复复合身份；冲突/缺失身份整批回滚；模型 ToolCall 改为响应级单份 IntentResult，副本冲突失败关闭，非法控制作为失败 ToolResult 回注并重新组装/Gate；前端将启动失败与模型正文分离并持久化恢复 |
| 精确影响计划 | 12 个实际变更文件映射到 AG/CORE/DE/ED/PCG/REF/SYS/WS 直接模块及 16 个受影响模块；选择前端全量、TypeScript、Rust 全量和治理审计 |
| 沙箱内运行 | 前端 611/611；Rust 401/402，唯一失败为 LibreOffice 子进程 signal 6。该次失败未作为通过证据 |
| 正常系统权限同输入重跑 | `PASS`：前端 611/611；TypeScript；Rust lib 402/402、core_chain 5/5、sm_broadcast 4/4；治理审计通过 |
| 测试治理 | `PASS`：21/21；发现 1006 个测试；0 未映射源码、0 无 owner、0 focused/skipped、0 warning |
| 语义复核 | `PASS`：迁移只保全/归一持久事实；IntentResult 仍由模型产生且不授权；CapabilityGrant 仍由运行时逐调用计算；非法控制不执行工具；draft failure 不成为后端终态或后续模型历史 |
| 真实 provider | `PASS`：经人类明确授权，仅发送当前 1470 字符内部协议与合成数据。DeepSeek Chat/Reasoner 均完成问候、`read_doc`、响应级 `_binderTurn`、合成 ToolResult、evidence ID 与 final；额外问候抽样各 5/5。Reasoner 在首次尝试曾出现一次控制封套缺失，随后完整闭环与抽样通过，该瞬时失败保留为稳定性观察 |
| 真实应用数据 | `PASS`（只读）：运行中应用库为 schema v20；`agent_turn_events` 1072 行，旧表 0 行，缺失 workspace/session 身份 0，重复 event_id 0，重复 turn+seq 0 |
| 真实 WKWebView 静态观察 | `PASS`：当前 binder 进程在修复源码落盘后启动；当前工作区、文档、历史 DiffCard、模型选择与 Composer 正常渲染 |
| 真实 UI 操作 | `PARTIAL`：Sky Computer Use 仍启动失败，但 macOS Accessibility 已可操作。对长时间打开的 `商务汇报.pptx` 发送合成“你好”时，在创建 AgentTurn 前返回 `AG_ACTIVE_SESSION_LOADING`；切换到空的 `文档/文本文件/empty.txt` 后，同一问候约 7 秒完成，持久化 117 个有序事件和唯一 `done`，IntentResult=`answer`，路线为直接回应问候，正文正确。其后仍显示包含 8 个打开会话 `session_metadata` 的不确定性 ProjectReport，交互噪声显著。未盲点可能绑定历史上下文的重试按钮；DiffCard、等待/重启与隔离操作未执行 |
| 最终裁决 | 自动化保持 `AUTOMATED`；零张 `SYSTEM_PASS` / `PASS`；G5 继续 `BLOCKED` |

## §14 P00 独立验证与修复验收基线

### §14.1 执行入口与证据分层

本节所有新增卡当前均为 `DESIGNED`，P00 未执行其中场景。P02/P08 应提供最小开发入口：
在隔离的临时 workspace 中选择固定模型传输响应和具名场景，运行真实解析器、核心调度、权限计算、
事件存储与生产 UI；显示 workspace/session/turn、事件 seq、当前阶段/阻塞原因/下一步、预算分项、
DecisionRequest/操作身份以及提案、预览、接受、保存的独立证据。场景按钮与故障能力仅在开发构建启用，
默认关闭，不新增第二套状态存储或运行核心，不在正式用户库重置数据。

这是**待实现入口**，不能现在声称可点击。P08 退出必须把实际启动命令、场景选择位置、
固定场景版本、证据导出位置填入本节；任一缺失则 M1 手动验证不可执行、不得通过。
通用错误/延迟场景使用明确标识的测试能力；它只替换边界外环境，不伪造被验证核心状态或 DE 成功。

五条证据轨道分开记账：①固定模型传输→真实解析器/核心；②专业适配契约（延迟、版本、回执）；
③固定事件→生产 UI；④真实模型经生产 Gate/网关的协议与行为抽样；⑤真实应用/真实专业能力。
①的 Gate 测试替身仅控制边界结果，不证明真实授权集成；真实工具成功、编辑与保存不能用测试能力代替。
核心专项测试不得启动 DOCX/LibreOffice，但治理选择器若要求全量回归仍需完整执行，不能缩减命令。

每次记录提交及 dirty diff、构建、场景/协议版本、真实或固定模型、初始资源版本、首次结果、
重试次数、脱敏事件及最终效果。失败的首次运行必须保留；重试通过不能覆盖首次失败。
下列“唯一终态”指每个 AgentTurn 首终态锁存；等待不是终态，TaskRun 只为复杂目标投影。

### §14.2 M1 自动化与手动基线

| ID / 状态 | 覆盖和断言 | 不覆盖 / 所需后续证据 |
|---|---|---|
| V-M1-01 / DESIGNED | v2 纯问候，无活动文档或无关打开文档 loading/unavailable 时正常准入；不取正文、不发工具/Proposal、不误报已读；未知版本/跨 workspace 身份仍拒绝；v1 不降级 | 不证明真实模型不误选工具；H01、P18 |
| V-M1-02 / DESIGNED | 固定模型调用生产 workspace_glob，检查临时工作区实际结果、ToolResult→再组装→Gate→final；不可用、越权及执行失败返回分型失败，无成功证据 | 不证明文档修改；H02/H06 和真实能力集成 |
| V-M1-03 / DESIGNED | 初始、工具回注、Decision、预览继续的预算分项和真实序列化输入同源；阈值上下边界、超长必需引用、重复快照和巨大结果；预算失败不调用下一次模型，已有效果保留 | 不证明模型厂商 token 数绝对精确；需记录估算方法/预留及真实模型标定；H03 |
| V-M1-04 / DESIGNED | 连续 Decision A→B：完成 A 的迟到清理不删除 B；重复点击/回调不二次继续；错误身份拒绝，按钮状态来自真实投影 | 不证明真实窗口布局；H04 |
| V-M1-05 / DESIGNED | 版本化预览异步 applied/failed/超时回执；旧版本、重复、迟到回执不推进；成功回执不等于 accepted/saved；未知效果阻止盲目重放 | 契约替身不证明 DE/ED 真实应用；H05/H08 |
| V-M1-06 / DESIGNED | 传输/工具等待取消、断流、非法 JSON/控制封套、超时；不能误执行或误完成；迟到事件无新副作用，下一请求可运行 | 不证明取消撤销已经提交效果；H02/H06；M2 补齐各生产等待点期限 |
| V-M1-07 / DESIGNED | 同会话多轮更新、切换会话及新任务隔离；显式记忆读取/注入、未经选择不注入；状态与历史回放一致 | 不承诺自动长期记忆或重启自动恢复执行；H07/H09 |

七张卡需要自动化和下列对应手动证据才能宣布 M1；专业回执契约必须验证真实核心处理，
M1 至少一次真实 workspace_glob，不等待复杂 DOCX 成功。M2 还需 P09～P15 各生产路径证据，
M3 还需真实模型/专业/UI 及原发布阻断卡，不能仅以这七张卡通过代替。

### §14.3 手动验收卡（P08 交付入口后执行）

#### H01 纯对话与按需准入

- 前置状态：开发隔离 workspace、新 session，无显式引用；准备无活动文档、一个无关 loading 资源两种状态。固定响应选择“你好”无 ToolCall。
- 输入/操作：分别输入“你好”；然后保持 loading 资源输入“列出工作区的 txt 文件”，选择 workspace_glob 固定响应。
- 可见行为：问候正常结束、不弹文档错误、不附未读取文档的大段报告；列举显示真实工具与真实匹配路径，不要求编辑器初始化。
- 状态/证据：两次各有独立 turn、合法 Gate、唯一 completed；问候工具/正文快照/文件写入为零，列举有生产 glob 证据；元数据不得成为已读正文证据。
- 失败判定：被无关文档 loading 阻断、问候误调用工具、列举假结果或把失败称完成；真实模型重跑不满足时另记模型轨道失败。
- 清理/重复：删除本场景的隔离 workspace 前先结束运行并保留脱敏证据；重新建立同文件清单、新 session 重跑；不得清理真实用户库。

#### H02 模型协议错误归因

- 前置状态：固定模型入口，新 session；准备非法 JSON、缺控制封套、流截断，以及同任务合法响应各一份。
- 输入/操作：输入“列出 txt 文件”，分别选择三种错误响应运行；记录首次结果后使用合法响应显式新建回合重试。
- 可见行为：错误显示模型协议/传输阶段与可重试说明，不显示已调用成功；重试不遮盖旧错误。
- 状态/证据：错误事件保留 turn/阶段，非法调用零执行；若既有协议以失败 ToolResult 回注则只按真实循环继续，不新增隐式纠错机制；断流无 completed，合法回合有实际 glob 证据。
- 失败判定：非法指令被执行、流截断误完成、错误统一归为编辑器错误、系统偷偷替换响应造成功。
- 清理/重复：结束场景，恢复默认传输；同响应重复执行，错误归属与执行次数应一致；真实模型抽样另记次数与首次失败。

P09 增量核验：生产网关字节解析与辅助产出共用同一事件边界，自动化必须以逐字节拆开的中文/表情、CRLF、非法 UTF-8/JSON、缺字段、空或混合损坏工具列表、重复/冲突完成、截断结束原因、尾部半帧及 provider 错误证明失败关闭；合法 `tool_calls` 不额外要求 `done`。固定字节回归只证明解析契约，不能替代本卡在真实应用中核对事件、终态、工具零执行与显式重试。P09 的自动化总验收与未完成项以计划 §14.32 的 Runtime Report 为准；H02 真实 provider 样本与系统卡仍待独立执行。

2026-09-29 P09 自动化回执：网关定向 14/14、精确五文件关联计划的前端 632/632、Rust 436/436、两组集成测试与治理审计通过；曾因受限沙箱本地端口/LibreOffice 失败的第一次全量运行不作通过证据。此处仅将生产解析契约切片标为自动化 READY；本 H02 卡仍为待手动验证。手动时需分别保留坏流对应的唯一错误终态、Gate/模型/工具次数、用户可见文案，以及显式新回合重试的独立证据；不以旧的隔离固定响应截图代替真实 provider/应用证据。

#### H03 上下文更新与预算边界

- 前置状态：隔离的长文本和短文本；入口显示当前预算配置、估算方式与分项，支持选择初始/工具回注超限场景。
- 输入/操作：先无引用问候，再显式引用短文本提问，随后选择超限长引用；再测试巨大工具结果回注及同快照重复引用。
- 可见行为：未激活正文不占据全量上下文；短引用回答基于本轮事实；必需内容超限说明具体分项、阶段和缩小范围方式，不静默丢弃引用。
- 状态/证据：revision 与快照版本可查；合法裁剪/去重有记录且不改语义；预算拒绝后无下一次 provider 调用；已执行工具的事实保留，失败不能抹掉或称业务成功。
- 失败判定：只显示统一超限文案而无定位证据、调大固定上限掩盖重复装配、沿用旧文档内容或直接忽略显式引用。
- 清理/重复：移除引用、新建回合发送短问候应成功；恢复预算配置，使用同版本夹具重复四种入口；Decision/预览继续预算在 H04/H05 联合覆盖。

P08-G 新增开发隔离入口场景“预算错误后同会话新回合”：一次运行先用超长输入触发 `initial` 预算拒绝，再在同一临时 AgentSession 以新 AgentTurn 发送“你好”。自动化应同时保留首轮 `error` 与次轮 `done`，分别显示 turnId、次轮 sessionId、两次 `initial` 预算和 Gate/模型合计 1/1；任一回合不得产生工具结果。手动在开发版点选一次并导出安全事件、预算分项、后续回合状态与请求计数；首轮错误消失、次轮仍被旧预算阻断或两轮共用同一 turnId 均判失败。该固定输入只验证核心状态释放，不替代 H03 的真实引用/快照测试。

同入口另有“预算错误后新回合延迟（可取消）”：点选运行，待第二轮模型等待时点“取消验证”。预期首轮保留预算 `error`，后续回合单独 `cancelled`，两轮 turnId 不同且 sessionId 相同，合计 Gate/模型 1/1、无 ToolResult；取消后按钮恢复可运行。若取消错指向已结束首轮、第二轮继续到 completed 或重复终态，即判失败。此固定传输场景不证明真实网关所有等待点的超时与取消，后者仍属 M2/H06。

#### H04 连续确认与确认卡位置

- 前置状态：生产 UI、新 session，固定模型响应依次请求任务确认 A、继续后确认 B；无真实破坏性操作。
- 输入/操作：发送场景请求，滚动历史，确认 A；B 出现后送达 A 的迟到回调；快速重复确认 B，并测试拒绝分支。
- 可见行为：当前唯一可操作 DecisionBar 固定在 Composer 上方；历史可保留引用但不出现第二个有效确认入口；B 不被 A 的清理删除；确认中按钮防重复。
- 状态/证据：workspace/session/turn/task/decision 身份一致；每个决定只消费一次，继续前重新组装预算和 Gate；等待态有原因与下一步；拒绝不执行被拒工具。
- 失败判定：卡漂移到不确定消息、重复推进、新消息绕确认、B 消失、拒绝后仍报告操作完成。
- 清理/重复：取消剩余等待，建新 session 按确认与拒绝各跑一轮；旧 session 保留只读证据，不直接改数据库消除等待。

P08-G 的隔离 `decision-chain` 现对 A、B 各用同 requestId 重送一次相同确认，并在自动裁决轨迹中给出 `duplicateAlreadyResolved=true`。开发版可重跑此固定场景，核对仍只有 A/B 各一次 resolved、三个 Gate/模型请求与唯一 done；重复调用若增加继续片段或使 B 消失则失败。这是后端裁决单次消费证据，**不是** H04 生产 DecisionBar 的位置、真实双击与拒绝分支验收。

2026-09-29 P08-G 手动回执：预算拒绝后同会话新回合独立 `completed`，延迟变体在实际取消后新回合独立 `cancelled`，两项均保留首轮错误且各自合计 Gate/模型 1/1；A→B 安全事件各一次裁决、唯一 `done`、Gate/模型 3/3，内部裁决轨迹两项 `duplicateAlreadyResolved=true`。取消后按钮恢复可点尚未取得直接手动回执；组件回归仅证明模拟回执到达后解锁并能再次运行，不冒称开发版人工观察。H03 的真实引用/快照、H04 生产卡位置/点击、H05 预览及 H07 会话/记忆仍未由这些固定场景签收。

#### H05 异步回执与过期目标

2026-10-01 增量入口：开发版“开发验证（隔离环境）”选择“预览检查点守卫（无编辑器）”并运行。前置无需当前文档，使用临时 workspace/内存数据库；预期状态明确显示“非 Agent 回合、非编辑器验收”，`预览检查点证据` 中错误身份、旧 V2、重复 ack/继续均拒绝，匹配 V1 仅消费一次，Gate/模型 0/0。任一布尔值为 false、误显示为真实编辑成功或第二次运行污染前次状态均失败；重新点击会创建新临时身份。此入口只验证后端契约。前端 15 秒超时、迟到回执与本进程禁止盲目重发有服务层自动化；仍须在实际应用以可控延迟的真实预览通道观察卡片、状态及效果未知提示，未获得该系统回执前 H05 保持未通过。

2026-10-01 执行回执：人类实际运行上述隔离入口，界面状态为“隔离预览检查点守卫已核验（非 Agent 回合、非编辑器验收）”，Gate/模型 0/0；`checkpointStillAwaitingAfterRejection`、`wrongIdentityRejectedBeforeAck`、`duplicateAckRejected`、`staleV2ContinuationRejected`、`matchingV1ContinuationOnce`、`duplicateContinuationRejected` 均为 `true`。AI 随后运行前端预览定向模拟 7/7 与 Rust preview 名称筛选 8/8，通过项中直接相关的 Rust 守卫/事务为 3 项。`staleV2ContinuationRejected` 仅证明传入版本与检查点不匹配，不证明真实编辑器 V1→V2 后的回执时序；测试替身也不等于桌面 UI 系统验证。H05 保持未通过。

- 前置状态：契约测试能力返回明确的测试预览，延迟回执；核心使用生产等待/事件逻辑，提供版本 V1/V2 与 applied/failed 场景。
- 输入/操作：触发操作，在回执前使目标版本变为 V2；送达 V1 applied，再测试当前版本 failed、重复 applied 和无回执至期限。
- 可见行为：旧对象或失败说明明确，不展示“已接受/已保存”；无回执不会无限转圈，给出核对或重试条件。
- 状态/证据：旧/重复回执零次继续；有效回执最多一次继续并重新预算/Gate；超时唯一 error，效果未知被标记而非伪造未执行。
- 失败判定：只凭 success 布尔值继续、版本过期仍执行、重复回执两次调用、超时当成功；用此卡宣称真实 DE 通过同样失败。
- 清理/重复：结束运行、释放延迟回执队列，新 turn 重建 V1；必须另外执行 H08 的真实文本专业链。

#### H06 能力不可用、拒绝、执行失败、取消与超时

- 前置状态：新 session，生产 glob 可用；测试能力明确标识 unavailable/denied/fail/delay 五种模式，固定模型只调用所选能力；显示实际期限。
- 输入/操作：分别执行前三种模式；delay 模式在执行中点击取消，再跑一次不取消直到超时；最后调用真实 glob。
- 可见行为：分别说明不可用条件、拒绝原因、执行错误、取消和超时；停止后恢复可输入；不承诺已经完成原业务。
- 状态/证据：拒绝前执行次数为零；执行失败带真实失败结果；取消/超时只有首终态、迟到结果不继续调用；已发生效果单独保留；glob 证明异常结束未污染下一次请求。
- 失败判定：五类都只能猜原因、取消无效或无限等待、终态重复、失败后仍生成成功证据。
- 清理/重复：取消遗留等待并恢复正常能力注册；每种模式新 turn 重复，不使用生产文件制造破坏或故意断开真实账户。

#### H07 多轮、会话、新任务与显式记忆

2026-10-01 H07 增量自动化：开发隔离入口 `session-memory` 在临时 workspace、内存数据库和固定模型传输中连续运行 A 会话更正、单轮显式 MemorySelectionRef、下一轮不选择、A 等待 DecisionRequest 时 B 会话独立完成；实际发送到模型入口的六次请求只输出分层标记布尔观察，不导出完整 Prompt。跨会话历史以 `AG_HISTORY_SCOPE_MISMATCH` 在 Gate 前拒绝；A 的等待请求保持 `waiting`，B 独立完成，Gate/模型各 6 次，隔离 memory_entries 仍为零。现有 `agRuntime` 服务测试补充历史按发起 AgentSession 装配、显式记忆不自动进入下一轮或 B，会话切回后不带 B 历史；组件测试确认隔离证据不冒称真实模型或记忆库面板验收。该自动化只签收核心装配与持久身份切片：尚未在实际应用点击 MemoryPanel“引用”、切换真实标签、查看等待条或验证重新启动后的恢复；本 H07 卡保持未通过。

- 前置状态：隔离 workspace 的会话 A/B；准备用户显式创建/选择的测试记忆“项目代号：青松”，固定模型支持读取装配信息；不含真实私人数据。
- 输入/操作：A 第一次提问后追加更正；显式选中该记忆再提问，然后去掉记忆新建任务；A 进入等待后切到 B 发问，再切回 A。
- 可见行为：A 更正被下一轮使用；只有已选择的记忆进入本轮；B 不显示 A 的可操作确认卡或载入状态；回到 A 能看到其等待原因及下一步。
- 状态/证据：历史与显式记忆标为 data-only，不变授权或当前文档事实；workspace/session/turn 分离；新任务不带旧挂起操作，未经选择无自动记忆读写副作用。
- 失败判定：串会话、旧事实覆盖新更正、隐藏等待、未选择记忆仍注入、自动写长期记忆或取消错任务。
- 清理/重复：回 A 正常取消等待；清除测试记忆选择，按 B→A 顺序再跑；记忆清理由已有用户操作完成，不修改真实库。

#### H08 真实简单能力、Diff 与最终陈述

- 前置状态：实际应用、隔离 workspace 的已打开 `sample.txt` 内容“甲”，记录磁盘与 LogicalState；不需 DOCX。真实 provider 轨道另跑，固定传输轨道不计真实模型通过。
- 输入/操作：先“读取 sample.txt”，再“把甲改为乙”；观察提案后先拒绝；重跑后接受但暂不保存，再显式保存并重开文件。
- 可见行为：读取来自当前 LogicalState；提案、已预览、已接受、已保存各自显示，拒绝撤销本提案效果；最终回答不能提前宣称接受或保存。
- 状态/证据：记录 read/Proposal/预览回执/裁决/保存与文件重开事实；completed 只证明 AgentTurn 结束；依原三态规则验证磁盘仅在对应保存路径变化，不改 DE 既有分支。
- 失败判定：测试能力替代真实读写、只凭回答判断成功、接受前写盘、拒绝后文本错误、最终陈述超过证据。
- 清理/重复：用正常编辑和保存恢复“甲”，确认无 pending 提案后新 session 重跑拒绝/接受分支；保留前后文件事实与事件证据。

#### H09 异常退出、历史恢复与重复执行边界

- 前置状态：隔离 workspace，分别准备等待确认、正在执行测试延迟操作、已完成真实文本提案三个场景；先记录身份及实际效果。
- 输入/操作：在各场景退出并重新启动应用；检查历史，尝试使用旧确认；对效果未知操作先核对资源，再显式重试。
- 可见行为：未结束任务显示 Interrupted 或原协议对应的异常终结及下一步，不自动续跑；旧确认不可复用；历史完成项只读恢复，不重新生成提案。
- 状态/证据：重启期间 provider/工具执行为零；副作用未知先核对，显式重试使用新 turn 和当前授权/版本；旧操作身份不重复消费；未保存状态按现有异常退出清算规则处理。
- 失败判定：恢复历史即重执行、旧确认恢复授权、未知效果盲目重放、把中断伪记 completed、承诺跨进程自动续跑。
- 清理/重复：核对临时资源并通过正常操作恢复初始内容；新 session 再跑；不以清空数据库作为恢复实现。本阶段只承诺历史恢复与受控显式重试，不承诺自动执行恢复。

### §14.4 P03 增量证据（2026-09-20）

V-M1-01/02 获得部分自动化证据，完整结果见计划 §14.12；本节不将任何卡提升为 SYSTEM_PASS。
专项命令：`cargo test --manifest-path src-tauri/Cargo.toml context_v2 --lib`（无 DOCX/模型依赖）、
`cargo test --manifest-path src-tauri/Cargo.toml context_response --lib`（包含真实 5 秒超时）、
`cargo test --manifest-path src-tauri/Cargo.toml fixed_runtime_v2 --lib`（需要本地回环 HTTP 权限，不调用外部模型）、
`npx vitest run src/services/agRuntime.test.ts`（生产前端采集/投影，IPC 边界夹具不替代后端核心测试）。

补充实际应用验收步骤（**未执行**）：隔离 workspace 建立 `a.txt` 并打开，发“你好”应无工具；再发“列出 txt 文件”应出现真实枚举；对未就绪富文档保持打开再问候应不因快照失败阻断。
读取时关闭原目标或切换标签：不得读成新标签目标，也不得把旧 ED 缓存/磁盘副本当原逻辑态；应显示明确失败或原目标的有效读取证据。
每次记录 session/turn、唯一终态与工具结果；关闭测试会话后新建会话重复，不清理真实用户数据。
实际模型可能不按预期选工具，需单列模型轨道；这不替代 H01/P08 固定场景的可重复手动验收。

### §14.5 P04 增量验证

P04 补充（2026-09-21，待执行记录见开发计划 §14.14）：V-M1-03 的分项对账以 `prompt_budget_checked` 为依据，逐次对比剥离 layer 后的 messages/tools；不要将 `sourcesNonAdditive` 再加入总量。开发者通过隔离测试的事件证据验证初始超限零次模型调用、真实读取成功后回注超限零次下一调用；H03 实际界面观察仍等待 P08 最小入口，本次不提升 SYSTEM_PASS。

可直接执行的最小入口：项目根目录运行 `cargo test --manifest-path src-tauri/Cargo.toml budget --lib`；前置为本地回环 HTTP 可用，不需要外部模型、DOCX 或真实编辑器。预期所有预算测试通过；其中 `fixed_runtime_budget_denial_stops_before_first_or_next_model_call` 实际创建隔离 workspace 并经过生产入口，断言唯一终态、初始零调用、读取成功后不再调用及预算事件不含夹具正文。出现失败或跳过即不通过；测试自身清理随机临时目录，可重复执行。该入口验证核心自动化行为，不替代 H03/H04/H05 的实际 UI 操作。

### §14.6 P05 增量验收入口

下列入口可由开发者直接手动触发，执行的是自动化断言，不替代实际 UI 验收；不连接外部模型，不需要 DOCX/LibreOffice。V-M1-03 不提升为 SYSTEM_PASS。

2026-09-21 自动化执行结果：三组入口中的断言均已纳入最终完整关联回归并通过（Rust 单元 429/429、集成 9/9；前端 615/615），首轮空读取夹具失败与修正过程见计划 §14.16。不是“人工在真实应用逐卡通过”。

| 卡 | 前置及操作 | 预期可见行为、状态与证据 | 失败判定、清理/重复 |
|---|---|---|---|
| P05-A 读取边界 | 项目根目录执行 `cargo test --manifest-path src-tauri/Cargo.toml bounded_reads --lib` | 隔离 txt：完整正文仅一份；25 块目录分 20/5，后页带版本；缺失块/非法参数/版本变化明确拒绝，超大全文和单块不返回截断结果；证据区分全文/块/目录 | 任一断言失败或跳过即不通过；成功自动清理随机临时目录，原命令重复，不删用户数据 |
| P05-B 注入与历史 | 执行 `cargo test --manifest-path src-tauri/Cargo.toml reference_dedup --lib`，再执行 `cargo test --manifest-path src-tauri/Cargo.toml continuation_reserve --lib` | 同次相同引用合并，不同身份/用途及匿名内容不合并；长文档目录明确部分覆盖，短对话历史预算不因预留被清空 | 任一断言失败即不通过；纯内存测试，无需清理，原命令重复 |
| P05-C 完整核心失败闭合 | 允许本地回环 HTTP；执行 `cargo test --manifest-path src-tauri/Cargo.toml fixed_runtime_oversize --lib` 及 `cargo test --manifest-path src-tauri/Cargo.toml fixed_runtime_budget_denial --lib` | 固定响应经真实生产入口/授权/只读工具：超大读取 success=false，无 full_document 证据，失败回注后唯一 completed（不等于读取成功）；初始超限零次调用，工具成功后保留的巨量参数超限不发下一调用 | 缺少唯一终态、误登记全文、第二次模型请求带巨量正文或超限仍调用即失败；自动清理隔离数据库/目录，重复同命令 |

P04 的“读取成功后超限”负向夹具在 P05 更新为“真实工具成功后，保留的巨量原始调用参数造成超限”，预算防线并未删除。实际应用卡：隔离工作区打开小 txt，请求读取，再用含 25 行的 txt 请求列目录并读取最后一块；观察模型是否遵循分页/版本与读取范围，记录会话/回合/工具成功状态，不能将生成回答当已读或已保存。此 UI 步骤尚未执行；取消、裁决继续、卡片位置由 P06/P07/P08 继续验收。

### §14.7 P06 决策隔离验收

2026-09-22 自动化执行结果：最终组件 39/39、服务 75/75、治理自测 21/21 通过；精确 11 文件完整关联回归退出 0，前端 624/624、Rust 单元 429/429、集成 9/9、TypeScript 与治理审计通过。以下自动化入口的断言已覆盖，不等于 P06-C 实际应用卡或 V-M1-05 全卡通过。

| 卡 | 前置状态与操作 | 可见行为、状态及证据 | 失败判定、清理和重复 |
|---|---|---|---|
| P06-A 组件入口 | 项目根目录运行 `npx vitest run src/components/ag/AgentUI.test.tsx -t "DecisionRequest identity isolation"`；不需要模型、编辑器或 DOCX | 真实 DecisionBar/store：A 待返回时出现 B，B 按钮可用；A 成功/失败不清 B、不污染 B 错误，旧 finally 不清新动作；重复点击只提交一次；原会话接收迟到资源失败回执；legacy 请求入站后绑定会话 | 任一断言失败即不通过。仅服务调用边界受控，不替换组件/store；测试清理 DOM 和 store，原命令可重复 |
| P06-B 服务入口 | 运行 `npx vitest run src/services/agRuntime.test.ts -t "serializes a decision|releases the submission|does not resume into another|does not clear another"` | 真实服务/store，IPC 边界夹具：同请求并发拒绝且只继续一次；A→B 仍是原 session/turn；提交失败保留请求并允许显式重试；迟到 resolved/终态不清其他请求/动作；跨工作区结果明确报已提交但未继续 | 出现替代 AgentTurn、误清 B、错会话、重复启动或隐瞒失败即不通过；内存夹具每例重置，无用户库/磁盘写入，重复原命令。不是 Rust/真实网关端到端证据 |
| P06-C 实际应用（未执行） | 隔离 workspace 与会话 A，请求一项需人工确认的任务；出现 DecisionRequest 后确认，后续再次要求确认时观察 B。另一次提交后立即切会话，观察回执位置；失败时再显式重试 | 固定裁决区域保持 Composer 上方；A 结束后 B 仍可操作；回执不进入新会话；原回合身份不变；失败可见且不宣称文档已完成或保存 | 旧回调使 B 消失、按钮继承旧错误、回执串会话、重复继续即失败；取消隔离任务并新建会话重复。真实模型未产生该场景不算通过，应记模型轨道未覆盖，等待 P08 固定入口，不改用户文档强行造场景 |

本轮自动化不覆盖 WKWebView 实际布局/滚动、真实 provider、RPC 超时但服务器已提交的重试恢复，也不解除 V-M1-05 全卡或父 G5。TimelinePanel 在恢复请求 await 前后切会话的来源绑定是上游 WS 入口问题，另行审计；本轮未修改该文件。

已提交后的失败专项可执行 `npx vitest run src/services/agRuntime.test.ts -t "marks continuation failure"`：关闭屏障的停止调用失败时，仍保留 decisionResolved 与原 taskRunId；组件断言已提交后错误写原会话且不污染后继请求的局部错误区。测试清理关闭状态后可重复，不将“已提交”混同“已继续/已完成”。

### §14.8 P07 预览回执验收

2026-09-22 执行记录：服务 79/79、Rust preview_ 专项 5/5 通过；获批相同范围完整关联回归退出 0，前端 628/628、Rust 单元 430/430、集成 9/9、TypeScript 与治理审计通过。受限环境首次 9 项 Rust 环境失败不作为通过证据，详见计划 §14.20。P07-C 尚未执行。

自动化与真实应用证据分开；以下入口不要求 DOCX 或真实模型。P07 不赋予接受/保存事实，不承诺重启续跑或不确定 RPC 自动重试。

| 卡 | 前置与操作 | 预期行为与证据 | 失败判定与清理 |
|---|---|---|---|
| P07-A 身份与失败 | 项目根目录运行 `npx vitest run src/services/agRuntime.test.ts` | 真实服务/store；IPC 夹具延迟 ack 后切会话，继续归原会话，同请求并发仅一次；版本变化、过期、工作区变化、ready=false 不启动继续；拒绝消息归原会话且不清新动作 | 断言失败即失败；测试自动清理内存 store，重复同命令；不替换被测服务逻辑 |
| P07-B 检查点与事务 | 运行 `cargo test --manifest-path src-tauri/Cargo.toml preview_ --lib` | 隔离库执行生产守卫：错误身份/版本、重复及已接受/拒绝/过期 Diff 不消费；组合存储故障回滚检查点，恢复存储后显式提交可成功 | 错误后检查点提前 acknowledged、重复消费、终态续跑即失败；内存库随用例销毁，重复命令，不触碰用户库 |
| P07-C 实际应用（未执行） | 隔离工作区打开含两段文本的小 txt，输入“请分两步完善当前文档：先润色第一段，再基于修改后的内容给第二段补充一句总结；不要替我接受或保存。”预览期间新建会话；另次在预览后手工编辑或拒绝 PendingDiff | 继续只归来源会话；状态过期停止继续并说明；记录原 session/task、版本和继续次数；预览、接受、保存分别观察，不以回答推断已落盘 | 串会话、过期继续、失败无说明或误称保存即失败；取消隔离任务、拒绝剩余 PendingDiff、新建会话重复。模型未产生分步预览场景不算通过，待 P08 固定入口 |

尚不覆盖真实 WKWebView 布局/滚动、网络未知提交结果恢复、跨重启继续及完整 V-M1-06。切离工作区后的错误不写入新工作区会话；跨工作区历史错误持久投影仍属后续验证范围。

### §14.9 P08 首批开发入口（2026-09-25）

仅开发构建：AgentPanel 顶部展开“开发验证（隔离环境）”，不使用当前文档、会话或用户数据库。
每次运行创建随机临时目录和内存数据库，结束后释放；强制结束进程可能留下临时目录，不宣称崩溃清理已验。
固定模型和固定外部门禁响应不是实际模型质量/实际订阅验证。状态 completed 仅表示 Agent 终结，工具结果独立看 success 和 evidenceIds。

| 卡 | 前置状态、输入与操作 | 预期可见行为及证据 | 失败判定；清理与重复 |
|---|---|---|---|
| P08-A 对话/真实能力 | 启动开发版，展开入口，先选“纯对话”点击运行，再选“真实 txt 文件枚举”点击运行 | 每次新 runId；对话无 tool_result；枚举的安全事件显示 success=true，工具回注证据中的 toolResult 包含 verification.txt、不含 excluded.md；两者 toolCallId/evidenceIds 一致；各一个终态 | 只有成功摘要但无实际文件结果、身份/证据不一致或重复终态即失败；结束后原按钮再运行，临时目录自动清理 |
| P08-B 模型错误 | 分别选择“非法 JSON”“流截断”，点击运行 | 两者均为唯一 error 终态、无 ToolResult；非法 JSON 准确返回 `AG_GW_STREAM_INVALID_JSON`，合法控制封套后的流截断准确返回 `AG_GW_STREAM_INCOMPLETE`；两项 `diagnostic.observedAt=runtime`、`category=model_protocol`、`rootCauseStatus=unknown` | 仅检查 error 而未到目标错误码、错误响应被当正常完成即失败；新版本分类仍为 unknown 即分类修复失败；保留当前证据后重跑“纯对话”，新运行不能继承错误 |
| P08-C 拒绝/执行失败 | 分别选择“门禁拒绝”“读取失败”运行 | 前者 `gate_rejected` 且无工具执行；后者经合法宿主上下文回传后，read_doc 的 success=false，diagnostic.observedAt=`tool_execution`，工具回注与 ToolResult 同 callId 且无 `AG_CONTEXT_*`，无成功读取证据；Agent 可正常收束但不能说读取成功 | 拒绝仍执行、读取失败停在 tool_validation 或标成功即失败；结束后重新运行枚举，检查无状态残留 |
| P08-D 取消 | 选“延迟响应（可取消）”运行，出现运行事件后点击取消 | 显示取消请求，随后实际 cancelled 终态；重复运行按钮在途禁用，取消只携本次 runId | 点击即伪造终态、取消另一运行或继续产生副作用即失败；结束后新运行纯对话 |
| P08-E 预算边界（主路径与复跑已验） | 开发版展开隔离入口，选“首次装配超限”点击运行；记录结果，再选“工具后装配超限”点击运行；两次之间不改当前用户工作区 | 首次：唯一 error 为 `AG_PROMPT_BUDGET_EXCEEDED`，预算分项 `assemblyReason=initial`、`exceeded=true`，实际 Gate/模型请求均 0、无工具结果。工具后：先有真实 `workspace_glob` 成功 ToolResult，随后唯一预算 error；两条分项依次为 `initial` 未超限与 `tool_result` 超限，实际 Gate/模型请求各 1（没有第二次模型请求）；界面明确“Agent 结束不等于业务成功”，预算分项只含计数、不含夹具正文 | 任一路径被更早错误抢先、工具前失败、超限后仍请求模型、出现第二终态、原始巨量输入/参数进入显示即失败；保留事件和预算分项，重跑“纯对话”确认隔离状态无残留，再分别重复两项；临时目录由入口常规清理，强杀进程后的清理不在本卡承诺内 |
| P08-F DecisionRequest 核心继续（核心手动已验；显示项待观察；非 H04 UI 验收） | 开发版展开隔离入口，选“自动裁决 A→B（仅核心）”点击运行；此入口自动确认两个固定 `manual_confirm`，不点击生产 DecisionBar、不修改用户 workspace | 安全事件按 A proposed→A resolved→B proposed→B resolved→唯一 done 排列，同一 AgentTurn，两个不同 requestId 对应同一 TaskRun；预算检查依次 initial、decision_resume、decision_resume 且未超限；隔离 Gate/模型各 3 次；无文档、Diff、接受或保存证据。开发面板的 `decisionTrace` 是独立可见项，不是上述核心事件的替代证据 | 2026-09-29 两次用户运行各见 A/B confirm 与唯一 done；首次 Gate/模型各 3 次且三次预算未超限，核心路径通过。第二次未提供 Gate/预算数据，`decisionTrace` 显示未观察，均不记为通过。后续任一请求被跳过、重复裁决、创建第二 turn、等待被当完成、身份变化、预算/Gate 缺失即失败；保留事件与计数，新建隔离运行复验，场景结束释放临时资源。该卡不能证明卡片位置、用户按钮防重或真实用户裁决，H04 另验 |

自动化入口：`npx vitest run src/components/ag/AgentUI.test.tsx`；`cargo test --manifest-path src-tauri/Cargo.toml development_scenarios --lib`（需允许本地回环端口）。前者验证真实组件，替代 IPC 边界；后者执行真实运行核心/授权/工具及隔离库，不替代被测核心逻辑。
真实开发版 P08-A、Gate 拒绝和 P08-D 已由用户操作并提供事件证据；2026-09-28 P08-B 两项及 P08-C 执行失败项也按修正后的入口重验通过。H03～H09 所需预算选择、连续确认、专业预览、会话历史与记忆场景尚未全部接入，完整 P08/M1/G5 仍未闭合。

2026-09-27 首轮手动结果：P08-A 纯对话与真实枚举通过；P08-C 门禁拒绝通过；P08-D 等待模型响应取消通过。P08-B 两项及 P08-C 的执行失败项均被其他更早错误抢先，未取得目标路径证据。修正输入与断言后必须重新执行这三项，首轮 error/ToolResult 失败不能计入目标场景通过。

2026-09-28 重验结果：非法 JSON 的唯一错误终态为 `AG_GW_STREAM_INVALID_JSON`，没有工具事件；合法控制封套后的截断先输出 `unfinished` 片段，随后以唯一 `AG_GW_STREAM_INCOMPLETE` 错误终结；读取 `missing.txt` 时 ToolResult `success=false`、`diagnostic.observedAt=tool_execution`、`evidenceIds=[]`，实际工具回注报告文件不存在且与 ToolResult 同 `toolCallId`，ProjectReport 没有声称读取成功。该次 `completed` 只表示 Agent 收束。七个首批场景均有开发版手动证据；首批入口切片 READY。两项传输错误的 `diagnostic.category=unknown` 留作分类元数据精度问题，不能据此推断根因已经确定。

上述 `category=unknown` 是修正分类前的历史手动样本。P08-B 卡现已增加新版本的分类预期，分类函数与生产入口事件自动化断言通过；修正后的真实开发版操作尚未由人手重验，不能用旧样本或自动化断言替代。

P08-E 使用固定超长输入及固定大参数，经原预算检查与真实只读工具，预算分项从隔离库的内部检查点安全投影；它不验证真实 provider tokenizer、模型主动缩小范围、生产工作区或 H03 全部引用更新场景。2026-09-29 用户提供两种错误路径及各自复跑样本：首次超限 89877/19904、Gate/模型 0/0；工具后超限 126240/19904、先有真实 glob 成功、Gate/模型 1/1；两项新版网关流错误亦显示 `category=model_protocol` 且根因未知。工具后主要超限项是 `model_continuation`，不等于巨量工具返回正文验证。预算错误后另跑纯对话的手动回执未提供；人类明确要求收束该切片继续开发，此未观察项保留，不伪记为通过。

## 通用核心恢复补充验证（P13/P15，2026-10-07）

2026-10-07 AI 已在真实 Binder 开发窗口点击新增「批量运行核心验收」，16 项固定入口实际终态匹配，
逐项调用次数/错误/回执见开发计划 §14.48。取消由隔离设施自动请求，裁决自动提交，
不能替代真实 DecisionBar/编辑器点击验收。当前无需让人类重复跑这 16 项；总闭合还缺慢写执行端口的
取消、期限、迟到效果及重放防护实现，不是通过增加手动卡消除这一实现缺口。

使用开发版 Agent「开发验证（隔离环境）」；不打开/修改真实用户文档。每次场景创建临时工作区与
内存数据库；再次运行生成新身份，旧证据必须另存，不以重复运行成功覆盖首轮失败。

| 场景 | 操作与前置 | 可见行为和执行证据 | 失败判定 |
|---|---|---|---|
| 裁决恢复重复调用拒绝 | 空闲面板选同名场景，运行验证 | 一次确认请求/一次自动裁决，下一模型返回同 ID 后 error AG_MODEL_TOOL_ID_INVALID；Gate/模型 2/2；无第二次工具执行 | 第二次确认卡或 ToolResult；仍 completed；调用数超过声明 |
| 连续裁决预算不重置 | 空闲面板选同名场景，运行验证 | 六次独立确认请求/裁决，恢复预算耗尽 error AG_TOOL_LOOP_LIMIT；Gate/模型 6/6；未启动第七次调用 | 第七次 Gate/模型，等待无限延长，或假称业务完成 |

两卡自动裁决只验证核心，不证明真实 DecisionBar 点击/布局；状态、结果、引用接受与保存仍分别取证。
只读期限的自动测试直接经过真实等待/取消包装，延迟能力仅制造外部耗时；生产 glob/read_doc 另以
真实临时文件验证，不能由延迟能力替代真实能力。原生窗口运行证据单列，不与组件或 Rust 测试混合。

### 执行端口补充卡（P10/P12/P13，2026-10-07）

- 前置：当前开发构建，打开 Agent「开发验证（隔离环境）」；使用入口自建临时 txt/SQLite，不打开复杂编辑器。此设施仅开发/测试使用。
- 操作：选择「执行端口：取消/超时与迟到真实文本建议（不接受、不保存）」并点击运行，或点击「批量运行核心验收」。自动依次制造取消和期限，不需要人类重复点击裁决。
- 可见：单项为 `effect_port_checked`；批量应有 17 项及逐项回执。此项 Gate/模型为 0/0，不称 Agent 业务成功。
- 状态/证据：取消、超时及时返回；在途禁止接管原身份及再次写；释放真实 txt 提案后执行记录 settled、效果 proposal_created、真实 PendingDiff 身份可查；原回合只有原终态且原文件未变。应用正常回合提供只读「核对执行效果」，查询不继续模型、不重试能力。
- 失败：等待直到慢能力结束才取消；迟到覆盖终态；缺结算记录；旧身份重放；未知效果可删除/新写；提案被称接受或保存；只显示总数不核对详细回执，均不通过。
- 清理/重复：入口清理自建临时文件，重复运行生成新身份。查询只读；不手动删除真实用户文件或用删历史消除未知效果。
- 覆盖边界：验证生产执行端口、真实文本提案与独立记录，不覆盖 DOCX、生产 DecisionBar 排版、模型语义、接受/保存；H05 专业系统卡与父 G5 不因本卡通过而关闭。

2026-10-07 AI 当前原生 Binder 窗口执行回执：批量 17/17 终态匹配，逐项详细回执已核对。
effect-port 的取消/超时两例均 started→settled、proposal_created、真实 PendingDiff ID 可查，
takeoverRejected / terminalUnchanged / fileUnchanged / proposalOnly 均 true，Gate/模型 0/0。
这不证明“资源扫描持共享数据库锁”路径终态及时可见；该实际实现缺口见计划 §14.50，M2 不提升。

### 资源计划准备锁补证（2026-10-08，AI 自行窗口验证）

- 前置/操作：当前开发窗口，展开开发验证，选择「执行端口：文本提案/资源扫描取消与超时（不接受、不保存）」或批量核心验收。入口各场景使用独立临时 SQLite/真实 txt，不清除生产未知效果事实。
- 可见/证据：当前原生窗口批量 17/17，effect-port Gate/模型 0/0、四个案例；取消/期限时 terminalPersistedWhileScanPending=true，终态先落库。文本案例迟到真实 proposal_created/PendingDiff，不接受不保存；资源案例 resourceRejectedAfterTerminal/noResourceCommit=true、零计划/确认。全部 takeoverRejected/terminalUnchanged/fileUnchanged=true。
- 失败判定：终态必须等扫描释放、旧回合生成计划/确认、原文件变化、迟到终态覆盖、清除 unknown 记录才能跑下一案例，任何一项发生均失败。通用回执仍可保守 unknown/inspect_effect，不称已安全自动恢复。
- 清理/复跑：入口回收自建临时目录/数据库，重复运行取得新身份；不修改用户工作区。受控延迟围绕真实准备逻辑，不能代替实际资源执行、性能基准或生产确认卡点击。
- 自动化：41 文件完整关联回归通过，前端638、Rust455、core_chain5、sm_broadcast4、tsc/doc-tests/audit；首次独立案例共用状态引发 AG_EFFECT_PENDING 的失败已保留并修正夹具隔离。
- 剩余边界：此卡补证“计划准备不阻塞终态”，不补证“确认后真实资源执行不阻塞终态”。后者在 task_runtime.rs→execute_operation_plan 仍占共享 DB，且前端直接等待，属于当前 M2 功能阻断，见计划 §14.52；不是要求人类再跑固定卡。H04/H05 及 M3/G5 不提升。

## 裁决执行隔离自动验收补证（2026-10-08）

本补证替代上文“确认后执行仍占共享 DB”的当前阻断结论，历史失败不删除。无需人类重复固定卡。

- 实际操作入口：Binder → 开发验证（隔离环境）→ 批量运行核心验收；AI 当前窗口已取得 17/17 并读取完整回执。它不覆盖真实 Provider 或生产确认卡布局。
- 确认后执行入口：现有 Rust `confirmed_workspace_operation_recomputes_runtime_grant_before_ws_revalidation`，分别运行正常确认和执行中取消；调用真实取消/终态持久化、真实资源删除、效果事实落库，重复确认只读同一回执。受控执行器仅在真实算法前插入检查，不替换授权、文件操作或终态逻辑。
- 慢文件阶段入口：`shared_execution_releases_database_during_io_and_claims_once`，暂停同一真实执行算法已领取状态后的文件阶段；期间 DB 可立即落取消 TaskRun、重复领取失败，释放后真实删除完成、仅一条历史，TaskRun 取消终态不变。仅临时目录，每次新数据库，测试自行清理。
- 前端期限入口：agRuntime 的 `bounds decision submission and does not replay or resume after a late response`；提交超 30 秒后返回效果待核对，迟到 RPC 不继续模型，不清掉原请求，再确认不提交第二次 RPC。传输替身仅用于制造延迟，真实 resolve/继续守卫未 mock。
- 判失败：取消持久化须等文件 I/O；在途重复确认再次操作或改变原终态；超时后自动继续；真实失败被声明为成功；专业接受/保存由资源执行回执推断。
- 清理/重复：测试只清理各自临时目录与迁移 DB，不操作真实用户文档；窗口入口每次新隔离身份。生产效果未知不可用“重跑卡”清除，须读取原执行事实；本阶段不提供自动业务重放或强制回滚。
- 当前完整回归状态以计划 §14.54 / active_scope 为准；专业 H04/H05 和 G5 状态不提升。

## AG-STREAM-RELIABILITY-01：模型协议受阻与长流期限验证（2026-10-14）

本卡只验证本次两个故障的分层归因与修复，不把模型每次都生成合法控制封套、专业编辑完成、确认卡点击或文件保存纳入通过条件。历史失败 `t-1791904349674-5` 必须保留，不能被复跑成功覆盖。

| 验证点 | 入口与输入 | 必须观察的事实 | 失败判定 |
|---|---|---|---|
| 控制冲突、零执行 | `model_output::tests` 的两份不同 `_binderTurn` 固定原生 ToolCall；再运行真实 Agent 验证入口中的协议错误场景 | `AG_MODEL_CONTROL_CONFLICT` 只暴露来源和字段名；整批执行回执 `rejected/not_started`、无成功证据；可见工具卡为“受阻，未执行”，没有无动作的“重试/跳过”按钮 | 把第一份封套当授权继续执行、回执记为已执行失败、泄漏目标路径/正文、卡片暗示按钮可操作 |
| 有进度的慢流 | `gateway.rs` 本机 HTTP 夹具在总期限内持续给 `progress`，再给 `chunk`/`done`；`providers.test.mjs` 分别输入 DeepSeek 隐藏推理、Anthropic ping/工具参数与分片工具调用 | `reasoning`/`upstream` 进度不出现在回答正文，也不完成回合；按进度刷新空闲期限，最终仍依真实 `done` 或合法 `tool_calls` 结束；工具调用参数不被改写 | 进度变成回答/终态、活跃流被空闲期限误杀、丢工具参数 |
| 真正无进度或不完整 | 本机夹具给一次 `progress` 后停止；固定流只给 `progress` 后 EOF | 前者在空闲期限报 `AG_GW_STREAM_TIMEOUT`，后者报 `AG_GW_STREAM_INCOMPLETE`；均不自动重放已有工具效果 | 无限等待、伪造 completed、自动重试可能产生副作用的步骤 |
| 隔离真实供应商观察 | 仅在 8788 隔离网关用无用户数据测试提示；记录上游首字节、隐藏推理事件数、正文事件数、客户端断连；原 8787 网关和真实工作区不动 | 可确认网关发内容为空的进度帧，取消连接后上游请求停止；若供应商再现超过 180 秒响应，记录完整阶段时序 | 用短样本宣称供应商长期稳定，或把网关总时长直接归因于模型/编辑器之一 |

实际 Binder 窗口复验：新建空白会话，不打开真实文档；输入“把那个 TXT 文件重命名为 final.txt”，若出现目标歧义则只核对本卡的工具状态/最终错误，不借此签收意图识别。若出现受阻工具，展开回执并确认 `executionStatus=rejected`、`effectStatus=not_started`；界面显示“受阻，未执行”，没有无处理按钮；真实成功 `workspace_glob` 仍单独显示成功证据。若模型长时间只有进度，不得出现回答正文或“已完成”；按取消后必须保留已发生只读证据并出现取消终态。重复时新建回合并记录 `turnId`，不得用成功回合覆盖失败回合。此窗口卡在取得实际点击与日志前保持**未验证**，自动化与隔离供应商抽样不能代签。

2026-10-14 当前执行记录：AI 已在真实 Binder 开发窗口通过可访问控件运行“批量运行核心验收”，显示 17/17 终态匹配；它使用固定传输/临时数据，不能替代上述真实供应商冲突卡片。独立 8788 网关对真实 DeepSeek 单句请求见 `progress → chunk → done`，约 3.68 秒完成；记录隐藏推理增量 13、正文增量 1，未输出推理正文。固定供应商适配测试 4 项、工具回执/组件定向测试及本机 HTTP 活跃/停滞测试均通过。历史 194 秒样本无法逐帧复现，窗口里也未重新触发真实控制冲突；这两项继续作为证据边界，不伪记通过。原 8787 网关及当前工作区文件未由隔离入口修改。

## AG-TOOL-RELIABILITY-01：工具调用失败纠错与收敛验证卡（2026-10-22）

本卡只验工具协议/参数失败如何结束及只读工具重复失败如何收敛；不验模型回答质量、文档专业编辑、编辑器确认卡 UI 或文件保存。固定模型只替换外部 transport，必须经过真实生产回合解析、Gate、授权、执行、事件和终态代码。每个场景新建独立临时 workspace/session/turn；失败证据不得被重跑覆盖。

| 场景 | 前置与操作 | 预期事件/调用证据 | 失败判定 |
|---|---|---|---|
| T01 协议/参数一次纠错 | 固定传输分别先返回缺少结构化控制的 tool call、以及 schema 类型错误的只读调用；下一响应修正 envelope/参数后继续 | 首次请求 `rejected/not_started`，安全回注指出协议或参数错误；纠错额度减一、真实工具执行预算不因拒绝而减；修正请求经 Gate 后恰执行一次并有唯一成功证据 | 首轮无效工具被执行、拒绝批次占真实执行额度、纠错后重复执行或最终状态与回执不一致 |
| T02 协议错误达到上限 | 固定传输连续返回不同调用 ID 但同类无效协议请求，超过配置的有限纠错额度 | `AG_MODEL_CORRECTION_LIMIT`（或计划中实现的稳定专属错误码）唯一 error 终态；每轮有模型/Gate计数；从未获准工具零执行、零效果证据；不超过硬模型轮次上限 | 无限纠错、到通用 `AG_TOOL_LOOP_LIMIT` 才结束、任何无效批次执行或伪造 completed |
| T03 超大只读结果后修正范围 | 在临时 workspace 创建可稳定触发预算边界的文本；首个 `read_doc` 全文超限，下一轮按 `list_blocks`/较小范围读取 | 首次 `success=false`、码 `AG_TOOL_RESULT_TOO_LARGE`、无全文 evidence；回注说明不截断及窄读路径；后续成功只登记其真实覆盖范围，最终结论不声称读到全文 | 截断正文、超限仍登记 full_document、提示不具体、成功覆盖范围夸大 |
| T04 同一只读失败两次 | 固定传输重复同一只读工具与规范化参数并保持同一失败码（如 oversized） | 第二个真实失败回执先记录，再进入仅 final/clarify 收尾；允许剩余原预算内的一次正常回答，不允许第三次工具执行。模型若继续发工具则执行前拒绝，仍受原纠错/Gate/期限约束；失败读取无 evidence，回答不得宣称业务成功。§14.60 修订旧“无第三次模型请求”标准 | 第三次工具仍执行、失败变成功、已有证据丢失、无限纠错，或私密参数进入诊断日志 |
| T05 不同请求不误判 | 首次只读失败后，第二次改变参数或错误码 | 不触发重复失败守卫；允许在既定纠错/工具预算内继续，并按真实结果结束 | 仅因工具名相同就提前终止 |
| T06 副作用/效果未知边界 | 用固定传输触发一个受确认保护的写/任务工具；不实际裁决或通过隔离的失败执行器制造受控失败 | 本卡重复只读守卫不对写能力自动重放/短路；确认前仍等待且 `effectStatus=not_started`，或失败后保留既有 unknown/核对规则；不声明保存成功 | 自动再次执行有副作用调用、把未知效果写成无效果，或绕过确认 |
| T07 schema 与 runtime 一致 | 对 `list_blocks` 首/后页、`expected_hash`、`read_doc.blocks` 和结果上限做 schema/执行器同输入测试 | 首页可省 hash；`offset>0` 明确要求上一页 `version_hash`；非法页数与块参数给稳定错误码；上限说明与 12,000 字符 runtime 限制一致 | schema 暗示可省必填条件、同一输入 UI/模型契约和执行器接受结果不同 |

手动入口：开发版 Agent →「开发验证（隔离环境）」；若当前面板没有可选择的 T01–T07 固定传输入口，则先用对应 Rust 固定 transport integration tests 作为自动入口，不要求人类尝试真实模型碰运气。手动测试仅在新版本窗口用临时 QA 文件执行 T03；不得向真实用户文档写入或删除。每张记录 `turnId`、Gate/模型次数、最终事件、ToolResult executionReceipt/evidenceIds 和内部 prompt-budget 分层；不得复制完整 Prompt/正文/工具参数。

清理：隔离入口自动回收每场景的临时目录和内存数据库；真实临时文本只由测试夹具创建并由其 owner 清理。失败后重跑须使用新身份；不得删除历史回执或把新成功覆盖成旧回合成功。

2026-10-22 补充参数组合回归：`page-contract-recovery` 重现成功 read_doc→非首页缺 hash→重新首页→用真实 version_hash 读取第二页→回答；缺 hash 必须在 host 采集/执行前拒绝，码 AG_READ_PAGE_VERSION_REQUIRED、category=model_protocol、无失败读取 evidence。`page-contract-exhausted` 验证持续错误有界且旧读取证据保留。`repeated-read-tools-blocked` 验证仅收尾后违规原生工具不能执行。单元测试另验页末空页、页外起点、空 path/blocks/hash 以及版本变化；固定 transport 不替代真实供应商或窗口验收。执行结果以 §14.60 Runtime Report 为准。

追加交互边界：`read-failure-same-batch` 禁止同批后续调用，`read-failure-with-evidence` 保留原成功读取证据，`read-failure-gate-denied` 保留收尾阶段 Gate 拒绝；`read-failure-clarification` 必须在同回合用户补充后恢复窄读且累计预算不重置，不能用永久冻结制造“安全通过”。任何既有非只读调用均不得因此被重放。收尾模式须以真实传输请求证明 native tools 清单为空，而非只检查提示词文字。

执行回执（2026-10-22，§14.60）：本轮8个场景已加入原隔离选择/批量入口。AI在真实Binder窗口自行运行25项批量验收，25/25终态匹配，并读取窗口实际JSON深核本轮8项执行回执/计数/工具冻结与恢复，8/8符合条件。相同15文件关联回归前端652/Rust466/core_chain5/sm_broadcast4/tsc/治理审计通过，另test:governance21/build/格式通过。真实Provider单响应探针在最终清单修复前7/8，失败反例已回放，不记真实业务全通过；父G5和实际编辑/接受/保存不提升。

自动验证记录（2026-10-22）：精确 Scope 的 `test:related` 在获准启动本机测试服务后通过：前端 651/651、Rust 464/464、`core_chain` 5/5、`sm_broadcast` 4/4、doc-tests、TypeScript 与 `governance:audit` 均通过；另 `test:governance` 21/21、构建、定向 rustfmt 与 `git diff --check` 通过。首次受限沙箱的端口权限失败与 LibreOffice signal 6 已保留为环境限制证据，不计通过。生产回合固定 transport 定向场景覆盖一次协议纠正、参数纠正、协议纠错耗尽、重复 oversized read，以及“超限→块目录→选定块读取”恢复；回执、Gate/模型次数、终态、精确证据覆盖与无第三次请求断言通过。单元测试覆盖不同失败签名和 schema 描述。尚未验证真实 Provider 随机错误，也未在桌面面板逐场手动点选 T01–T07；T06 副作用/效果未知边界未由本次新增专项生产回合完整覆盖，不能标为整卡闭合。父 G5 仍 BLOCKED。

## 响应协议可靠性补验（AG-RESPONSE-PROTOCOL-01）

自动入口：`cargo test --manifest-path src-tauri/Cargo.toml --lib ag::verification::tests::development_scenarios_execute_real_core_and_release_isolated_state`。固定传输经真实生产入口、临时数据库和文件；无需人工反复触发随机模型错误。该测试需本机环回端口权限，不调用真实 Provider。

| 场景 | 通过标准 |
|---|---|
| text-control/invalid/incomplete/empty-corrects | 第一响应不泄漏未验证正文、零工具执行；第二次经重新装配、预算与 Gate，唯一 completed |
| tool-preamble | 普通说明被隔离；原生合法控制授权链不被提前拒绝，工具只执行一次 |
| tool-text-invalid-corrects | 明确损坏文本控制不能被合法工具封套覆盖；首次整批零执行，修正后仅一次执行 |
| text-protocol-exhausted | 三次不合规响应（最多两次纠错），唯一 AG_MODEL_CORRECTION_LIMIT，零工具 |
| after-read-text-corrects | 已有只读 evidence 保留，仅修正后续回答，不重放原工具 |
| decision-text-corrects/replay-blocked | 已有非只读请求后正文纠错只允许 final/clarify；新的原生工具拒绝，原裁决只创建/消费一次 |
| text-decision-shared-limit/model-budget | 确认等待续行保留原纠错次数；最多六次模型调用，耗尽后模型和 Gate 不再启动 |
| argument-decision-shared-limit/decision-native-replay-blocked | 参数拒绝同样计入共享纠错次数；等待后本批控制拒绝也不能使模型重新提交非只读工具 |
| text-correction-gate-denied/delay/incomplete/http402 | 纠错期间分别保持 Gate 拒绝、取消、断流、上游拒绝的真实终态；不混成协议无限重试 |

隔离返回 `protocolTrace` 仅为格式类别、错误码、阶段和次数；生产控制台 `[AG-AG-TURN]` 同样输出安全字段。实际窗口复验使用新编译进程；`provider_started.metadata.protocolVersion=binder-response-v2` 识别新处理路径。先纯对话，再使用临时文本只读枚举；记录首次合规率及纠错后结果，不能由有限真实抽样推导永不出错。HTTP_402 需独立处理供应商账户余额，不能签收为本包修复。

清理：自动入口回收临时目录、数据库和测试服务，失败事实保留；每次重跑使用新身份。不代替专业编辑、真实保存或父 G5 系统验收。

自动验收（2026-10-22）：上述18个新增固定场景经真实生产入口全部通过；最终12文件 `test:related` exit 0，前端651/651、Rust464/464、core_chain5/5、sm_broadcast4/4、doc-tests和治理审计通过。`test:governance`21/21、build含TypeScript、Rust格式与diff检查通过。首次沙箱本机端口拒绝不计通过，获准后重验通过。仅本响应协议切片READY；真实供应商/实际窗口未复验，父G5保持BLOCKED。

## 阅读预算与工具契约精简补验（AG-BOUNDED-READING-01，2026-10-23）

本卡只验能力内读取、分页、证据及契约，不以正常结束代替全文分析完成；父G5保持原状态。

| 项目 | 操作与通过标准 | 当前证据 |
|---|---|---|
| 正文与编辑投影隔离 | 大HTML含大量属性/表格时，默认read_doc返回text页，编辑节点仍用blocks或structure；正文无重复容器内容，保留顺序、表格行列分隔 | 自动测试PASS；源码reading_projection_tests及生产核心 |
| 有界正文续读 | 按真实next_offset/version_hash续页；拼接等于完整阅读投影；单个长段、引号/反斜杠、中文/emoji不丢失；各页序列化≤12,000 | 自动测试PASS；实际装页及无损拼接断言 |
| 目录自适应 | limit=50的富目录过大时仅减少完整条目，返回真实next_offset；连续分页无重复遗漏 | 自动测试PASS；80条目录多页核对 |
| 覆盖与失败 | 部分页（含末页）仍为selected_blocks；失败无成功证据；无版本续页、旧版本、越界与混合参数明确拒绝；结构超限仍失败 | 自动测试PASS；原结构失败断言与新分页证据断言均保留 |
| 契约精简 | native工具名与CapabilityGrant精确一致且无重名；未开放table/image操作不发布schema，开放时保持原约束；控制封套及执行守卫不删除 | 自动测试PASS；四种动态能力组合及schema前后测量 |
| 生产核心 | 固定传输只模拟模型，读取/Gate/预算/版本/回执/证据均走真实生产实现；合成大HTML完成两页读取、三次模型请求且无越权写入 | 集成PASS；活动HTML16250字，原输入预算内；不是实际Provider验收 |

实际应用复验（不是要求人工逐张点击）：打开测试文档，要求“完整读取当前文档，按具体条目分析，不修改”。观察read_result_observed中offset/end/has_more/version，确认后页连续且同版本；对全文请求若仍有未读范围，回答必须说明，不能自称全文分析完成。模型选择提前final需与预算错误分开记录。可通过现有隔离入口回归原错误恢复场景；固定模型传输不能替代真实Provider自主阅读质量验收，不新增付费调用。

## 历史装配与统一请求容量补验（AG-HISTORY-BUDGET-01，2026-10-23）

| 项目 | 操作与通过标准 | 验证层 |
|---|---|---|
| 长回答后续接 | 上一AgentTurn含2868/3863/12000字符回答；同AgentSession提交“按上一轮口径继续”。真实模型入参中保留前次用户需求与回答，不因旧6000额度扣减而清空历史 | 生产装配单元；真实模型语义另验 |
| 摘录而非伪摘要 | 最新单元超过派生历史额度时保留用户需求和回答首尾、原始长度/hash/省略标记；不改持久化消息，不把摘录说成完整记忆；空间不足不能只留助手孤立回答 | Unicode/确定性/边界单元 |
| 单一请求容量 | 根据messages+tools实际序列化余量装历史；能力说明/来源信封/转义/同回合回执全部计入最终检查；旧字符总盘子/RuntimeControl固定字符预留/6000历史额度不再独立拒绝 | 生产装配与实际网关入参对账 |
| 四页与历史压力 | 合成活动HTML正文40000字符，依次读取四页、五次Gate/模型请求；同时携带100000字符历史候选，预算紧张只缩历史；每次保留用户需求、回答首尾，完整拼接正文且未升级假全文证据 | 真实生产核心/固定模型传输，不是编辑器或真实Provider |
| 失败关闭与隔离 | 当前必需输入本身超限仍在Gate前结束，不能靠剪历史掩盖；跨AgentSession历史仍拒绝，显式记忆仅在选择后注入；取消/裁决/版本/六次额度不变 | 生产核心与原完整关联回归 |
| 可观察性 | `history_assembled` 中候选/纳入/丢弃/摘录数与实际请求一致，仅数字不含正文；预算显示新configVersion、总额及派生输入上限，原错误仍保留 | 脱敏投影及窗口隔离入口 |

本包自动结果与真实窗口结果分别登记于开发计划§14.62；不能以固定传输宣称模型必然理解“继续”。
本次自动判定：上述装配、四页压力、失败关闭与原关联回归通过；最终前端652/652、Rust474/474及集成/audit通过。真实窗口隔离批量25/25、恢复深核8/8及长历史布尔检查通过，但最后两项核心边界修正后的窗口重验受审批服务额度限制未执行；最终修正已包含在最后一次完整自动化，真实Provider语义未新验。父G5不提升。
最小真实复验：新会话要求“列出A/B两组的详细说明，只读”；待回答超过约3000字符后输入“按上一轮口径说明下一组”。
判断依据为同AgentSession、不同AgentTurn、历史纳入数量/摘录数、实际后续任务与终态，而非仅看一句“我记得”。
现有“隔离会话与显式引用”入口已扩展长回答装配，AI可直接运行，无需用户重跑A–H。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-10-23 | v3.20 | AG-HISTORY-BUDGET-01：长历史/摘录、统一请求测量、四页容量与失败关闭补验；固定传输与真实语义分开。 |
| 2026-10-23 | v3.19 | AG-BOUNDED-READING-01：正文与结构、版本续页、目录容量、真实页覆盖与契约裁剪自动验收；不替代真实模型自主阅读或G5。 |
| 2026-10-22 | v3.18 | 共用读取契约/受约束收尾8项进入现有隔离入口，记录真实窗口25/25与8项回执深核、完整回归及真实模型7/8反例边界。 |
| 2026-10-22 | v3.17 | 响应级封套判定、有界纠错、跨等待计数、防重放与诊断补验；独立于真实 Provider 和专业验收。 |
| 2026-10-22 | v3.16 | AG-TOOL-RELIABILITY-01 实施与验证结果登记；自动化相关回归通过，真实 Provider 与桌面逐场验证仍保留边界，父 G5 保持 BLOCKED。 |
| 2026-10-22 | v3.15 | 新增 AG-TOOL-RELIABILITY-01 工具协议纠错、只读重复失败收敛、超限读取和 schema/runtime 一致性验证卡；父 G5 保持 BLOCKED。 |
| 2026-10-14 | v3.14 | 增加模型控制执行前拒绝、隐藏推理进度、空闲/总期限与窗口复验分层卡；历史失败保留，窗口卡未验证。 |
| 2026-10-08 | v3.13 | 补真实资源执行取消/迟到独立结算、防重复确认及前端期限自动验收，区分当前窗口 17 项、后端真实集成与专业验收。 |
| 2026-10-08 | v3.12 | AI 当前窗口17项与四个独立执行端口案例补证；计划扫描锁边界通过，裁决真实资源执行仍阻断M2，保留首次失败与专业验收边界。 |
| 2026-10-01 | v3.11 | H05 隔离检查点手动回执与 AI 自动模拟分层登记，不提升真实编辑器验收状态。 |
| 2026-09-29 | v3.10 | P09 自动化协议切片通过并登记证据边界；H02 真实应用、provider 与显式重试仍待验。 |
| 2026-09-29 | v3.9 | H02 增列 P09 字节流/辅助路径自动化矩阵与真实应用证据边界；系统卡不因固定输入测试提升。 |
| 2026-09-29 | v3.8 | P08-F 两次手动核心事件与首次预算/Gate 证据分项登记；开发面板 decisionTrace 显示和 H04 保持待验。 |
| 2026-09-29 | v3.7 | P08-F 连续裁决守卫及 ID 撞号已修复，完整自动化通过；卡片转为待真实开发应用手动观察，仍非 H04 UI 验收。 |
| 2026-09-29 | v3.6 | P08-F 定向测试发现第二裁决请求失败，卡片标为 BLOCKED，避免用开发 UI 对失败链签收。 |
| 2026-09-29 | v3.5 | 记录 P08-E 人工预算主路径与复跑证据及未观察的隔离复验；新增 P08-F 固定 DecisionRequest 核心继续卡，明确不能替代 H04 UI 验收。 |
| 2026-09-28 | v3.4 | P08-E 增加首次与工具后超限的开发版手动卡，区分安全预算分项、实际调用与原工具证据；当前待手动执行。 |
| 2026-09-28 | v3.3 | P08-B 增加网关流错误诊断分类与根因状态验收口径；保留旧手动样本并要求新版本复验。 |
| 2026-09-28 | v3.2 | P08-B 与 P08-C 三项修正后真实开发版重验通过；首批七场景切片 READY，保留诊断分类残余与完整 P08/M1/G5 未完成边界。 |
| 2026-09-27 | v3.1 | 记录开发版首轮七场景实际反馈；三项被抢先错误遮蔽，修正预期错误码与执行阶段并列为待重验。 |
| 2026-09-25 | v3.0 | P08 首批七场景入口与四张可操作卡；其余 M1 场景及真实 UI 仍未验。 |
| 2026-09-22 | v2.9 | P07：预览回执身份、版本、终态及数据库事务验证入口；实际应用保持未执行。 |
| 2026-09-22 | v2.8 | P06：组件与服务隔离验收入口，真实应用步骤保持未执行。 |
| 2026-09-21 | v2.7 | P05 追加三个可手动触发的核心验证入口，明确与真实 UI 的证据边界。 |
| 2026-09-21 | v2.6 | P04 追加预算分项验证口径，系统验收不提升。 |
| 2026-09-20 | v2.5 | P03：追加准入/按需上下文专项命令与未执行的实际应用验证步骤；自动化和系统验收继续分开。 |
| 2026-09-15 | v2.4 | P00：新增 M1 七张分层验证卡及 H01～H09 六要素手动卡，明确待实现开发入口、M1/M2/M3 证据边界与历史证据隔离；全部新卡 DESIGNED，保留 G5 BLOCKED。 |
| 2026-09-09 | v2.3 | macOS Accessibility 恢复后补充真实 UI 最小问候证据：空文本回合完成且语义正确；记录 PPT active session loading 缺陷与普通问候的 ProjectReport 交互噪声，未扩大为完整 G5 通过。 |
| 2026-09-08 | v2.2 | AG-CLOSURE-REPAIR-01：记录真实七列迁移、响应级模型控制和失败恢复修复的自动化、当前协议 DeepSeek、真实应用库迁移及 WKWebView 静态证据；UI 操作环境仍不可用，G5 保持 BLOCKED。 |
| 2026-09-06 | v2.1 | 汇总 WP1～09 当前自动化证据，19 张卡推进为 AUTOMATED；记录真实 DeepSeek 工具选择 smoke 与 Tauri 启动通过；因 Computer Use 服务不可用，G5 真实 UI 场景保持 BLOCKED。 |
| 2026-08-14 | v1.0 | 建立 Agent 1.0 状态化链路验证卡。 |
| 2026-08-21 | v1.1 | 新增 WorkspaceOperation/CreateDocumentBundle 场景。 |
| 2026-08-27 | v1.2 | 澄清恢复统一为原 AgentTurn。 |
| 2026-08-28 | v1.3 | 新增 TaskRun 与 ExplicitEditTargetSet 验证。 |
| 2026-08-30 | v1.7 | 新增当时 AgentContextFacts/report_project 局部证据。 |
| 2026-08-31 | v1.9 | 收敛为唯一 Active Agent 验证卡。 |
| 2026-09-03 | v2.0 | AG-ADAPTIVE-RUNTIME-PLAN-01：按 G1/G2 重建当前卡；IntentResult/Grant 分权、TaskRun 投影化、EvidenceLedger 派生报告、自适应循环与 ProposalBatch 独立卡片成为当前验收；受语义变更影响的旧 PARTIAL 全部回到 DESIGNED。 |
