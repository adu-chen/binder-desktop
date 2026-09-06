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

| 字段 | 内容 |
|---|---|
| validation_id | `VAL-AG-1.0-CORE` |
| phase | DESIGN；卡执行属各 IMPLEMENTATION Scope 与最终 VALIDATION/G5 |
| owner / participants | CORE / AG、SYS、DE、ED、WS、ENT、GW、REF |
| 当前状态 | V-C1～C3、V-R1/R2、V-S1～S14 全部 `AUTOMATED`；当前无 `SYSTEM_PASS` 或 `PASS` 卡 |
| 自动化依据 | WP1～09 均有当前提交的 READY Runtime Report；2026-09-06 `e86f492` 同输入完整回归为前端 608/608、Rust lib 394/394、core_chain 5/5、sm_broadcast 4/4、TypeScript 与治理审计通过 |
| G5 当前结果 | `BLOCKED`：真实 DeepSeek 的工具选择与 ToolResult 后续轮 smoke 已通过，Binder Tauri 已启动；但 Computer Use 服务连续启动失败，无法采集真实 WKWebView 交互、DiffCard 裁决、等待/重启和隔离观察 |
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
| VAL-AG-REAL-EXPERIENCE-01 | 真实 DeepSeek 合成 smoke：主动 `read_doc`，ToolResult 后继续基于证据回答；Binder Tauri 启动成功 | 非固定表达的完整 Binder 工具选择、状态诚实和交互流畅性 | BLOCKED |
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

## §12 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-09-06 | v2.1 | 汇总 WP1～09 当前自动化证据，19 张卡推进为 AUTOMATED；记录真实 DeepSeek 工具选择 smoke 与 Tauri 启动通过；因 Computer Use 服务不可用，G5 真实 UI 场景保持 BLOCKED。 |
| 2026-08-14 | v1.0 | 建立 Agent 1.0 状态化链路验证卡。 |
| 2026-08-21 | v1.1 | 新增 WorkspaceOperation/CreateDocumentBundle 场景。 |
| 2026-08-27 | v1.2 | 澄清恢复统一为原 AgentTurn。 |
| 2026-08-28 | v1.3 | 新增 TaskRun 与 ExplicitEditTargetSet 验证。 |
| 2026-08-30 | v1.7 | 新增当时 AgentContextFacts/report_project 局部证据。 |
| 2026-08-31 | v1.9 | 收敛为唯一 Active Agent 验证卡。 |
| 2026-09-03 | v2.0 | AG-ADAPTIVE-RUNTIME-PLAN-01：按 G1/G2 重建当前卡；IntentResult/Grant 分权、TaskRun 投影化、EvidenceLedger 派生报告、自适应循环与 ProposalBatch 独立卡片成为当前验收；受语义变更影响的旧 PARTIAL 全部回到 DESIGNED。 |
