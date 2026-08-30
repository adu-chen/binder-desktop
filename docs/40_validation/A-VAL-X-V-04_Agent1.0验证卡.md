---
文档编号：VAL-X-V-04
文档状态：A
负责模块：CORE
文档职责：Agent 1.0 验证卡设计——为已冻结的状态化链路（三态分流/连续 Proposal/保存/澄清终态/TaskRun/外部内容/记忆）与 P1 开放条件定义进入 IMPLEMENTATION 的验证门禁
上游约束：A-CORE-X-D-02_Agent1.0冻结能力基线.md（§6 第 4 项）、A-CORE-X-D-01_1.0需求基线与文档体系索引.md（§6 第 4 步）、A-CORE-C-P-01_APC.md（§5.1 G5、§10 测试策略）、A-CORE-C-T-01_三态模型.md（BR-CORE-STATE-001~003）、A-SYS-I-P-01_数据结构定义.md（§3.1 共享对象契约）、A-DE-M-S-01/A-ED-M-S-01/A-WS-M-S-01/A-AG-M-S-01（D2/D3 状态机）
直接承接：后续 IMPLEMENTATION Scope（进入门禁与验收依据）、G5 集成/系统验证（执行与证据登记）、产品能力注册表（system_validation 回填）
使用边界：本文是验证卡**设计**，不执行验证、不填报通过；卡状态一律 DESIGNED；只引用已注册规则与既有状态机，不新增 RULE/TERM/状态机
变更要求：冻结需求语义变化、状态机/契约变化或证据门禁变化时回写本文；未实际执行不得把卡状态改为 PASS
---

# Agent 1.0 验证卡设计

## 1. 验证身份与当前结论

| 字段 | 内容 |
|---|---|
| validation_id | `VAL-AG-1.0-CORE` |
| phase | DESIGN（验证卡设计；执行属实现后的 VALIDATION / G5） |
| owner / participants | CORE / SYS、DE、ED、WS、AG、REF、AST |
| 当前状态 | `DESIGNED`：验证卡已设计，未执行；不表述为任何链路已验证通过 |
| 进入 IMPLEMENTATION 门禁 | §6：目标验证卡证据门禁满足，且 `governance:audit` 无该链路新增 error 级违规 |
| G5 退出条件 | 核心场景卡自动化 + 真实环境证据闭合，P1 各项按 §5 独立证据开放 |

本文按冻结基线 §6 第 4 项为 D1/D2/D3 的每个状态化链路与每项 P1 开放条件建立验证卡。卡的字段结构与
A-VAL-X-V-02 一致（前置条件 / 完整操作 / 期望结果 / 自动化证据 / 人类观察项 / 状态）。

## 2. 来源与追溯

| 验证意图 | 需求来源 | 技术约束 | 状态机 / 契约 |
|---|---|---|---|
| 三态三分流：已有 LogicalState 预览/保存、无 LogicalState 接受落盘、pending 打开迁移 | REQ-CORE-002/010、REQ-DE-003、REQ-ED-005、REQ-WS-003 | BR-CORE-STATE-001~003、X-FORBID-008/009 | SM-DE-01、SM-ED-01、SM-WS-01/02 |
| 连续 Proposal 以最新 LogicalState 接力、前卡失效、非补丁栈 | REQ-DE-009、REQ-AG-015、REQ-SYS-011 | BR-SYS-STATE-008 | SM-DE-01、SM-AG-04 |
| CreateDocumentBundle：确认前零副作用、确认后先建盘再投影、三重基线安全拒绝 | REQ-CORE-010、REQ-ED-010、REQ-WS-006 | BR-WS-STATE-003、BR-WS-DATA-003 | SM-WS-04、SM-DE-01、SM-ED-01 |
| WorkspaceOperation：前置裁决、重验、直接磁盘执行、补偿、统一恢复 | REQ-CORE-012/014、REQ-WS-008/010、REQ-AG-011/013 | BR-WS-STATE-003、BR-WS-PERSIST-003 | SM-WS-04、SM-AG-04 |
| AgentTurn：澄清非终态、任一阶段取消、首终态锁存、迟到事件无副作用 | REQ-AG-007、REQ-SYS-010 | BR-SYS-STATE-007 | SM-AG-01 |
| TaskRun：非后台队列、确认点、中断可重试、不跨重启续跑 | REQ-AG-015、REQ-SYS-011 | BR-SYS-STATE-008 | SM-AG-04 |
| 外部内容一次性 data-only、显式记忆选择 | REQ-REF-009、REQ-AST-001、REQ-SYS-012 | BR-SYS-DATA-007 | ExternalContentRef / MemorySelectionRef |
| 多会话/多 workspace 隔离 | REQ-SYS-009/010、REQ-ED-011 | BR-SYS-STATE-007 | AgentSession / AgentTurn 隔离 |
| 显式编辑目标：初始来源、澄清追加、scope 确认、TaskRun 继承与集合外硬拒 | REQ-AG-001、REQ-AG-015 | BR-AG-DATA-003、BR-SYS-STATE-007/008、BR-SYS-PERSIST-001 | ExplicitEditTargetSet / DecisionRequest / TaskRun |

## 3. 状态化链路验证矩阵

| 链路 | 验证卡 | 验证点 | 门禁证据 |
|---|---|---|---|
| DE-DIFF（Proposal 链） | V-S1/S2/S3/S4 | 三分流 + 连续接力 | 自动化（deRuntime/edOperations）+ 真实环境 |
| ED-EDIT（LogicalState 承接） | V-S1/S3 | 预览投影、pending 迁移、未保存会话 | 自动化 + 真实环境 |
| WS-SAVE / WS-DIFF-APPLY | V-S1/S2/S5 | 保存先裁决、无 LogicalState 原子落盘、失败保留 | 自动化（edRuntime/rust 落盘）+ 真实环境 |
| AG-TURN（AgentTurn） | V-S6 | 澄清/取消/首终态 | 自动化（agRuntime）+ 真实环境 |
| AG-TURN（TaskRun） | V-S7 | 编排/确认点/中断重试 | 自动化 + 真实环境 |
| WS-OPERATION | V-C1/C2/C3/R1/R2 | 前置裁决、先建盘、三重基线拒绝、dirty 阻断、补偿/恢复 | Rust/IPC/前端自动化 + 真实 workspace |
| REF-INJECT / AST-MEMORY | V-S8 | 外部内容/记忆 data-only | 自动化 + 真实环境 |
| 会话隔离 | V-S9 | 多会话/多 workspace 不串线 | 自动化 + 真实环境 |
| AG-TURN / AG-TOOL / DE-DIFF（目标授权） | V-S10 | ExplicitEditTargetSet 单调来源、持久化、确认追加与 PendingDiff 前硬拒 | Rust 自动化 + 真实 workspace/provider |

## 4. 核心场景卡

> 状态值：`DESIGNED`（已设计待执行）｜`PASS`（自动化+真实证据闭合，未经执行不得填报）。

### 4.1 V-C1 · CreateDocumentBundle：拒绝资源计划

| 字段 | 内容 |
|---|---|
| 前置条件 | 打开 workspace；请求“新建一份会议纪要”；系统已生成初始内容和 WorkspaceOperationPlan |
| 完整操作 | 查看 DecisionRequest 后拒绝 |
| 期望结果 | 确认前没有文件、标签或内容 Diff；拒绝后计划为 Rejected，DiskState 不变，TaskRun 不再执行该步骤 |
| 自动化证据设计 | WS：plan→cancel 零文件副作用；AG：DecisionRequest cancel→TaskRun 终态；UI：无 DiffCard/标签 |
| 人类观察项 | 决策卡与资源影响清楚；拒绝后文件树无目标 |
| 状态 | DESIGNED |

### 4.2 V-C2 · CreateDocumentBundle：确认后先建盘再投影

| 字段 | 内容 |
|---|---|
| 前置条件 | V-C1 同类计划，目标不存在且基线有效 |
| 完整操作 | 确认 DecisionRequest；观察文件树/标签/初始内容 Diff；接受内容卡并保存 |
| 期望结果 | WS 先创建空的目标 DiskState 并写资源条目，ED 再打开真实标签，DE 再显示初始内容 Diff；内容卡接受不再次创建/写入文件，保存后重开内容一致 |
| 自动化证据设计 | Rust/IPC 验证调用顺序与幂等；DE 验证 operation_id/origin；ED 验证标签对应真实 DocRef；WorkspaceTimeline 同时有资源与内容类型条目 |
| 人类观察项 | 确认后文件立即出现；随后才出现绿色初始内容和 DiffCard |
| 状态 | DESIGNED |

### 4.3 V-C3 · CreateDocumentBundle：安全拒绝与基线推进

| 字段 | 内容 |
|---|---|
| 前置条件 | V-C2 已投影初始内容 Diff |
| 完整操作 | A：不做其他修改直接拒绝；B：重新创建后先手动/代理编辑、外部修改或重命名，再尝试裁决旧卡 |
| 期望结果 | A：资源/DiskState/LogicalState 三重基线未变，WS 补偿删除本次文件并关闭标签；B：旧卡失效且不可接受/拒绝，文件、标签和当前内容保留 |
| 自动化证据设计 | 三重 guard 参数化测试；补偿删除写 WorkspaceOperationEntry；任一基线推进均拒绝补偿 |
| 人类观察项 | 安全拒绝确实删除空目标；继续编辑后旧卡失效且不误删文件 |
| 状态 | DESIGNED |

### 4.4 V-R1 · WorkspaceOperation：dirty 阻断与内容 Diff 失效

| 字段 | 内容 |
|---|---|
| 前置条件 | 删除/重命名/移动目标存在打开标签；分别构造 dirty 与 clean+pending content Diff |
| 完整操作 | 生成资源计划并确认 |
| 期望结果 | dirty 标签使执行在 Revalidating 阶段阻断且磁盘不变；clean 目标确认前内容 Diff 仍有效，资源执行成功后内容 Diff 以 resource_baseline_changed 失效，标签/引用按真实结果同步 |
| 自动化证据设计 | WS/ED/DE/REF 联调：影响清单、dirty guard、执行回执、失效事件与路径重绑定 |
| 人类观察项 | 决策卡列出受影响标签/Diff；阻断和成功后的显示都与磁盘一致 |
| 状态 | DESIGNED |

### 4.4a V-R2 · WorkspaceOperation：批量补偿、重启中断与时间轴恢复

| 字段 | 内容 |
|---|---|
| 前置条件 | 三项批量计划；第 2 项注入失败；另构造补偿失败与执行中重启 |
| 完整操作 | 确认执行；观察 fail-fast/逆序补偿；从 WorkspaceTimeline 发起恢复 |
| 期望结果 | 第 2 项失败后第 3 项不执行；第 1 项逆序补偿成功则 FailedCompensated，补偿失败则 PartialFailed 并保留逐项收据；重启后 Interrupted 且不自动执行；资源恢复生成新的 WorkspaceOperationPlan 和 DecisionRequest，冲突不覆盖 |
| 自动化证据设计 | SM-WS-04 全转移矩阵、幂等重复确认、补偿故障注入、重启恢复、文件/空目录/非空目录类型化恢复 |
| 人类观察项 | partial_failed 明确列出实际状态；恢复仍先确认；无第二套撤销 UI |
| 状态 | DESIGNED |
### 4.5 V-S1 · 已有 LogicalState：预览、接受、保存（BR-CORE-STATE-001）

| 字段 | 内容 |
|---|---|
| 完整操作 | 对已打开文档生成 diff；预览；接受；保存；重开 |
| 期望结果 | 预览进入 LogicalState 且卡片仍 pending；接受不重复应用、不写盘；保存先裁决 accepted/awaiting_save 再写盘；重开一致 |
| 自动化证据设计 | deRuntime/edRuntime 三分流编排；rust Mock-Webview IPC→DiskState→第二次读取 |
| 人类观察项 | 绿色增量、卡片终态、保存后复位 |
| 状态 | `DESIGNED` |

### 4.6 V-S2 · 无 LogicalState：挂载、接受、落盘（BR-CORE-STATE-002）

| 字段 | 内容 |
|---|---|
| 完整操作 | 对未打开文档生成后台 diff；卡片挂载；接受；读取文件 |
| 期望结果 | 预览/接受前磁盘不变；接受才原子写盘 + 恰一个时间轴节点；拒绝/过期不写文件 |
| 自动化证据设计 | deRuntime background 路径；rust：ws.apply_diff 原子落盘；无半节点 |
| 人类观察项 | 卡片挂载、接受后文件出现 |
| 状态 | `DESIGNED` |

### 4.7 V-S3 · pending 打开迁移（BR-CORE-STATE-003）

| 字段 | 内容 |
|---|---|
| 完整操作 | 后台 pending diff 存在时打开目标文档；再接受 |
| 期望结果 | 以同一 payload 建立带绿色增量的 LogicalState；卡片转已有 LogicalState 路径；不再后台落盘；接受只裁决 |
| 自动化证据设计 | edRuntime pendingDiffPromoted；deRuntime 迁移后路径断言 |
| 人类观察项 | 打开后绿增、卡片仍可裁决、保存行为 |
| 状态 | `DESIGNED` |

### 4.8 V-S4 · 连续 Proposal 接力（REQ-DE-009 / BR-SYS-STATE-008）

| 字段 | 内容 |
|---|---|
| 完整操作 | 同一文档复合目标连续生成 Diff1→Diff2→Diff3；中途拒绝最新卡；接受全部 |
| 期望结果 | Diff2/3 基准来自最新 LogicalState 而非旧 payload；后预览使前卡失效且不可裁决；拒绝最新卡无任务级回滚；background pending 不得作为续编输入 |
| 自动化证据设计 | deRuntime：多卡基准/失效/拒绝语义；agRuntime：TaskRun 内连续回合 |
| 人类观察项 | 卡片失效顺序、拒绝行为、内容历史 |
| 状态 | `DESIGNED` |

### 4.9 V-S5 · 保存有效 pending 先裁决 / 保存失败（REQ-WS-003）

| 字段 | 内容 |
|---|---|
| 完整操作 | 已应用但仍 pending 的 diff 直接保存；制造外部变更后保存触发冲突 |
| 期望结果 | 保存先裁决 accepted/awaiting_save 再落盘；基线推进时旧卡失效只保存当前内容；失败保留 accepted/awaiting_save 与 dirty；不覆盖外部内容 |
| 自动化证据设计 | edRuntime 保存即接受/失败保留；rust saveConflict→override 路径 |
| 人类观察项 | 保存后卡状态、冲突提示、文件未被静默覆盖 |
| 状态 | `DESIGNED` |

### 4.10 V-S6 · AgentTurn 澄清/取消/首终态/迟到事件（REQ-AG-007 / BR-SYS-STATE-007）

| 字段 | 内容 |
|---|---|
| 完整操作 | 写入歧义请求触发澄清；以原 session/workspace/turn_id 补充后继续；分别尝试错误 session、workspace、turn_id、重复恢复与终态恢复；在等待态和流式中取消；观察首终态后到达的事件 |
| 期望结果 | 澄清保持原 AgentTurn 的 AwaitingClarification 非终态（不伪装完成/取消、不创建新 turn_id）；只有匹配原身份的 ClarificationProvided 可回到 Assembling，错误、重复和终态重入失败关闭；取消任一活动阶段生效且无半落盘；完成/失败/拒绝/取消互斥、首终态锁存；迟到事件/工具无文档/卡片/落盘副作用 |
| 自动化证据设计 | agRuntime：同 turn_id 补充、不调用 cancel、不产生第二组消息、调用前快照失败保留等待身份、等待态取消、terminal_latch；rust：状态机唯一恢复事件、session/workspace/turn/task 身份 guard、重复/终态重入失败、取消竞争与终态后迟到事件抑制 |
| 人类观察项 | 澄清等待呈现、取消后无新工具/提案、终态原因 |
| 状态 | `DESIGNED` |

### 4.11 V-S7 · TaskRun 编排（REQ-AG-015 / BR-SYS-STATE-008）

| 字段 | 内容 |
|---|---|
| 完整操作 | 分别执行简单回答/分析/检索、单文档局部修改、复杂多目标修改与 CreateDocument；让简单回合动态请求任务级确认；复杂目标经 IntentResult 形成可展示 TaskPlan（步骤/依赖/当前游标/所需授权/确认点）；任务级确认后自动下一片段；存在非终态任务时尝试不带 continuation 的新任务级请求，再用稳定 task_run_id 显式续接 |
| 期望结果 | 简单 AgentTurn 不创建/绑定 TaskRun；简单 pending Diff 可见但 task_run_id 为空，不生成任务预览 checkpoint 或自动续片段；TaskPlan、DecisionRequest、WorkspaceOperationPlan 在持久化前按需创建并绑定 TaskRun。**三层分离**：DiffCard 只确认内容（接受/拒绝不改变 TaskRun）；任务级确认走 DecisionRequest，confirm 后自动开始下一执行片段（无需新聊天消息、绝不后台续跑）；只有显式 continuation 可复用非终态 TaskRun，无身份请求失败关闭且不合并目标/自行取消旧任务；同文档连续 Diff 基于最新 LogicalState、前卡因基准推进失效；信息流底部实时当前动作来自运行事件 |
| 自动化证据设计 | agRuntime：无 TaskRun 简单 Diff 不确认/不自动续片段，DecisionRequest/Preview continuation 保持稳定身份；task.rs：简单输入零 task_runs、复杂/任务级工具判据、非终态隐式复用拒绝、显式 continuation 与 plan 生命周期；turn_control：无 TaskRun 澄清同 turn_id 恢复 |
| 人类观察项 | 底部实时状态、DecisionRequest 卡裁决、gate 拦截、片段内多卡；**手动执行材料见 A-VAL-X-V-05（修订版）** |
| 状态 | `DESIGNED` |

### 4.12 V-S8 · 外部内容一次性引用 / 显式记忆选择（BR-SYS-DATA-007）

| 字段 | 内容 |
|---|---|
| 完整操作 | 附加外部文件请求分析；显式选择记忆条目进入回合；任务结束后再请求 |
| 期望结果 | 仅可用只读解析路径形成一次性 data-only 内容引用；不支持格式明确未读；任务结束/重启/跨会话不恢复、不转存记忆；记忆仅显式选择条目进入，1.0 不自动写入/检索/注入 |
| 自动化证据设计 | refRuntime ExternalContentRef 生命周期；astRuntime 显式选择/不自动写入 |
| 人类观察项 | 外部内容只读呈现、记忆选择交互、无自动写入 |
| 状态 | `DESIGNED` |

### 4.13 V-S9 · 多会话/多 workspace 引用隔离（G5-AG-09）

| 字段 | 内容 |
|---|---|
| 完整操作 | 两个 AgentSession / 两个 workspace 交错回合，各自带引用、diff、未保存内容 |
| 期望结果 | 消息、终态、工具卡、DiffCard、引用与未保存上下文不串线；重启恢复不伪造成功 |
| 自动化证据设计 | agRuntime 多会话隔离；rust 会话级持久化 |
| 人类观察项 | 交错会话无串线、恢复正确 |
| 状态 | `DESIGNED` |

### 4.14 V-S10 · ExplicitEditTargetSet 授权闭环（REQ-AG-001 / BR-AG-DATA-003）

| 字段 | 内容 |
|---|---|
| 完整操作 | A：以当前文档和显式 Doc/Selection 引用发起编辑；B：同一 AgentTurn 澄清补充显式文档引用；C：让工具发现集合外既有文档并发起 scope DecisionRequest，分别确认、取消和重放；D：显式续接绑定 TaskRun；E：尝试把模型文字、历史路径、只读工具结果、外部 File/LongText 内容或打开标签作为写入目标 |
| 期望结果 | A/B：仅用户可见且归一到 active workspace 的既有文档身份进入单调集合，引用正文保持 data-only；C：请求冻结非空去重路径，只有 confirm 原子追加，cancel/失败/不同裁决重放零追加；D：只继承该 TaskRun 已持久化集合；E：`propose_edit` 在 ChangeProposal/PendingDiff 持久化前以稳定错误拒绝，且零 PendingDiff；集合成员资格不改变只读 IntentResult 的工具授权 |
| 自动化证据设计 | Rust：初始集合与非身份引用隔离、澄清单调不缩减、v16 迁移和来源持久化、scope confirm/cancel/replay、CreateDocumentBundle 建盘后追加、TaskRun workspace 身份复核、AG/DE 双守；工具注册测试：target_scope 与参数约束；系统级检查拒绝前后 PendingDiff 计数不变 |
| 人类观察项 | 显式引用目标可编辑；集合外目标提示先引用或确认范围；取消范围卡后无隐藏授权；continuation 不跨 TaskRun/workspace 串用 |
| 状态 | `DESIGNED` |

### 4.15 V-S11 · RuntimeProfile / IntentResult / CapabilityGrant 最小授权（REQ-AG-007/015 / BR-SYS-STATE-007）

| 字段 | 内容 |
|---|---|
| 完整操作 | A：分别省略 RuntimeProfile、传入未知值与唯一合法值；B：执行 answer/inspect/search/analyze/edit/create_document/organize_workspace/clarify；C：让普通只读回合请求副作用工具，让 CreateDocument 请求 propose_edit；D：分别执行简单 Edit 与复杂多目标 Edit 的 scope DecisionRequest；E：在普通文档、无目标结构 BDocx 与具备动态结构能力的活动 BDocx 请求表格/图片操作 |
| 期望结果 | A：缺失/未知值在 Gate、provider、ToolCall 与 Proposal 前失败关闭，唯一合法值进入主链；B：IntentResult.allowed_tools 是当前 AgentTurn 的精确最小集，CapabilityGrant 只按 RuntimeProfile、任务类型、目标、副作用与格式/动态能力继续收窄；C：只读零副作用，CreateDocument/OrganizeWorkspace 只有必要读取与 plan_workspace_operation；D：普通 Edit 无 propose_decision，只有已绑定 TaskRun 的复杂 Edit 可提出 scope DecisionRequest，其他 kind 拒绝；E：BDocx 高级工具只有意图、目标、活动格式与动态能力共同满足时可见，具体参数仍在 ToolCall 执行层失败关闭 |
| 自动化证据 | AG-CONFORMANCE-CODE-S3C：IntentResult 定向 17/17、CapabilityGrant/ToolCall 定向 10/10、RuntimeProfile 1/1、内容 Proposal/WorkspaceOperationPlan 终态区分 1/1；test:governance 19/19 与 governance:audit passed；ADUS 新增 `BR-SYS-STATE-007-GUARD-AG-AG-TURN-037`、`BR-AG-STATE-002-DATA-AG-AG-TOOL-034`、`BR-AG-STATE-002-GUARD-AG-AG-TOOL-035`。最终 test:related 两次均为前端 592/592、S3C 相关测试及其余 Rust 373/374 通过；唯一失败为范围外 LibreOffice Worker signal=6，按 HD-S3C-02 转入 DOC 环境待验证项，不反向修改 AG 实现 |
| 人类观察项 | 只读回合工具卡无写能力；资源操作只出现待确认 WorkspaceOperationPlan；复杂多目标范围卡只允许 scope；BDocx 未满足动态能力时明确阻断且不伪造修改 |
| 状态 | `PARTIAL`（本地自动化已通过；真实 Tauri/provider/编辑器链待 G5） |

### 4.16 V-S12 · PromptPackage 前置与状态机生产驱动（REQ-AG-007/015 / SM-AG-01/02/04）

| 字段 | 内容 |
|---|---|
| 完整操作 | A：构造 PromptPackage 组装失败或超预算回合，并分别执行 Gate pass/deny；B：让模型在一个轮次返回多个 ToolCall，混合允许、未获本轮允许和执行失败项；C：分别制造 provider/Gate/TaskPlan 循环级错误、达到循环上限、形成 Proposal 与形成 DecisionRequest；D：让 WaitingConfirm TaskRun 接收 Finish、对应 DecisionRequest confirm/cancel |
| 期望结果 | A：PromptPackage、CapabilityGrant 与预算在 Assembling 完成，失败不调用 Gate/provider、不广播 Streaming；B：每个 ToolCall 按顺序独立经过 toolRequest/validateOk或validateFail/可选执行/回注，校验失败项不进入 ToolExecuting；C：loopError/loopLimit 只进入 LoopAborted→AgentTurn TurnError/Cancelled，不结算 Completed/Done，Proposal/DecisionRequest 只经 handoff→LoopCompleted；D：WaitingConfirm + Finish 被拒绝，只有对应裁决可离开 |
| 自动化证据 | AG-CONFORMANCE-CODE-S3E：`cargo fmt --check`、`cargo check --lib` 通过；AG Rust 回归 164/164，通过 handoff/loopError/loopLimit/cancel/终态冻结、WaitingConfirm + Finish forbidden、RuntimeProfile、内容/资源终态区分，以及生产装配顺序/唯一状态广播 owner；PromptPackage/CapabilityGrant/预算均位于 Assembled/Gate 前，SM-AG-02 广播唯一入口为 `advance_tool_loop`，LoopAborted 结算为 Failed/Error。最终 test:related 为前端 592/592、Rust 376/377，唯一失败为范围外 LibreOffice Worker signal=6；test:governance 19/19、governance:audit passed，ADUS 已注册 `BR-AG-STATE-002-EFFECT-AG-AG-TOOL-038` |
| 人类观察项 | 真实 Tauri/provider 下 Streaming 不早于装配与 Gate；混合 ToolCall 卡片顺序与执行事实一致；循环上限显示失败而非完成；DecisionRequest 等待态不被 Done 伪装 |
| 状态 | `PARTIAL`（实现与本地自动化已完成；真实 Tauri/provider 链待 G5） |

### 4.17 V-S13 · AgentContextFacts / ProjectReport 证据闭环（REQ-AG-009 / BR-AG-DATA-008 / BR-AG-OBS-001）

| 字段 | 内容 |
|---|---|
| 完整操作 | A：以活动/非活动打开文档、显式 SelectionRef 和当前/其他 AgentSession Proposal 启动回合；B：分别执行 Inspect/Search/Analyze/Answer/Edit；C：读取完整文档、结构、搜索摘要与目录，再提交正确和伪造 ProjectReport；D：报告成功后再执行新读取并尝试结束 |
| 期望结果 | A：事实在 Gate 前逐项匹配 LogicalState，兼容 blockRefs 归一为 SelectionFact，Proposal 仅包含当前 AgentSession 的裁决/效果/保存态且无正文；B：只有 Inspect/Search/Analyze 获得 report_project 并必须成功调用后才能完成；C：只有同轮证据账本能支持 readItems，目录/搜索证据不得冒充全文，无证据路径、跨 workspace 或缺失 uncertainties 失败关闭；D：新读取使旧报告失效，需重新报告 |
| 自动化证据 | AG-CONFORMANCE-CODE-S4A：前端直接 59/59；Rust 事实身份/层序/最小授权/证据账本/终态门直接测试通过。完整 changed_files `test:plan` 命中前端/TypeScript/Rust 全量与治理门；非沙箱 `test:related` 完整通过：前端 593/593、Rust 379+5+4、TypeScript 及 governance:audit passed。test:governance 19/19，ADUS 已登记四条新规则的生产块与测试覆盖 |
| 人类观察项 | 真实 Tauri/WKWebView + provider 下层序、工具卡、已读/未读范围和不确定性与实际文档一致；失败报告不伪装 Done |
| 状态 | `PARTIAL`（实现与自动化已 LOCAL_INTEGRATED；仅真实 Tauri/WKWebView + provider G5 待完成） |

### 4.18 待验证列表（不作为已完成实现的反向修改依据）

| validation_id | 已完成边界 | 待验证内容 | 当前状态 |
|---|---|---|---|
| VAL-AG-REAL-RUNTIME-01 | S3A/S3B/S3C/S3D/S3E 的规则、生产 owner 与本地自动化 | 真实 Tauri/WKWebView + provider 的 PromptPackage→Gate→Streaming→ToolCall→Proposal/DecisionRequest 完整事件顺序 | G5_PENDING |
| VAL-AG-PROJECT-REPORT-01 | S4A 版本化 AgentContextFacts、独立 Prompt 层、证据账本、report_project 完成门与本地直接测试 | 真实 Tauri/WKWebView + provider 下 Inspect/Search/Analyze 的读取证据、工具卡顺序、ProjectReport 与最终 Markdown 一致性 | G5_PENDING |
| VAL-DOC-LIBREOFFICE-01 | 受控沙箱仍可复现 Worker signal=6；非沙箱 macOS 的单用例与完整 `test:related` 均通过 | 环境差异已确认，不作为 AG 实现缺口；DOC 后续只需维持发布环境回归 | VERIFIED_NON_SANDBOX |

## 5. P1 开放条件验证卡清单

每项 P1 按子能力、格式/解析能力、权限、确认/保存/恢复语义与真实验证**独立开放**；对应子卡在各项 P1
开放时细化（本清单为门禁框架）。未满足 §6 门禁前入口保持关闭。

| P1 能力 | 独立开放条件 | 验证卡（设计） |
|---|---|---|
| TXT→Markdown / Markdown→TXT 目标副本转换 | 来源不改写、CreateDocumentBundle 先创建目标、初始内容 Diff、损失告知与三重基线拒绝 | V-P1-01 |
| 外部文件内容读取 | 显式附加 + 可用只读解析 → 一次性 data-only；任务结束清除；不支持格式退回元数据 | V-P1-02（=V-S8 数据面） |
| 显式项目记忆选择 | workspace 本地用户管理；仅显式选择进入回合；不自动写入/提议/检索/注入 | V-P1-03（=V-S8 记忆面） |
| WorkspaceOperation：创建/重命名/移动 | DecisionRequest 前置裁决、确认时重验、直接磁盘执行、dirty 阻断、内容 Diff 失效、幂等与补偿 | V-P1-04（=V-R1/R2） |
| 删除/恢复 | 冻结递归清单、dirty 阻断、类型化快照、WorkspaceTimeline 恢复、冲突不覆盖、无永久清理 | V-P1-05（=V-R1/R2） |
| 多既有文档逐项编辑 | 确认冻结目标清单、逐目标独立 Proposal/基准/裁决/保存、无批量裁决/事务 | V-P1-06 |
| 样式 P1（颜色/突出显示/段落/字号/字体族）、表格、图片 | 受控操作、动态能力、保存重开真实验证逐项通过后开放 | V-P1-07 |

## 6. 进入 IMPLEMENTATION 的门禁规则

1. 每个 IMPLEMENTATION Scope 必须声明其承接的验证卡（如 V-S1/S2/S3 → 三分流实现）。
2. 该卡的设计证据门禁：相关自动化测试（按 ADP §5 `test:related` 选择）与 `governance:audit`
   （无该链路新增 error 级违规）通过；状态化链路必须有对应状态机转移测试（APC §10）。
3. 自动化证据只证明实现正确性；真实系统证据（WKWebView、真实 provider、真实 workspace）在 VALIDATION
   阶段登记，未登记前卡状态保持 `DESIGNED`/`PARTIAL`，不表述为 G5 通过（APC §5.1）。
4. 冻结语义变化、状态机/契约变化或证据门禁变化时回写本文并复核受影响验证卡。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-08-14 | v1.0 | AG-1.0-D4-VALIDATION-CARDS-01：建立 Agent 1.0 验证卡设计——追溯矩阵、状态化链路验证矩阵、核心场景卡 V-N1~N4（D-NEW）与 V-S1~S9（三态分流/连续接力/保存/澄清终态/TaskRun/外部内容记忆/会话隔离）、P1 独立开放条件清单（V-P1-01~07）与进入 IMPLEMENTATION 的门禁规则；卡状态一律 DESIGNED，未执行不填报通过。 |
| 2026-08-27 | v1.1 | AG-CONFORMANCE-CODE-S3A：细化 V-S6 为同一 AgentTurn 澄清受控重入，补充身份 guard、调用前失败恢复与等待态取消证据设计；卡状态保持 DESIGNED。 |
| 2026-08-28 | v1.2 | AG-CONFORMANCE-CODE-S3B：细化 V-S7 为简单回合零 TaskRun、任务级产物按需创建、显式 continuation 唯一复用与无 TaskRun Diff 不自动续片段；卡状态保持 DESIGNED。 |
| 2026-08-28 | v1.3 | AG-CONFORMANCE-CODE-S3D：新增 V-S10，覆盖 ExplicitEditTargetSet 初始来源、澄清单调追加、scope 确认追加、TaskRun 持久继承、CreateDocumentBundle 建盘后追加及集合外零 PendingDiff 硬拒；卡状态保持 DESIGNED。 |
| 2026-08-28 | v1.4 | AG-CONFORMANCE-CODE-S3C：新增 V-S11，覆盖 RuntimeProfile 缺失/未知失败关闭、八类 IntentResult 精确 allowed_tools、五因素 CapabilityGrant、CreateDocument/OrganizeWorkspace 资源计划、复杂 Edit scope DecisionRequest 与 BDocx 动态能力；本地定向自动化和 ADUS 注册已通过，卡状态 PARTIAL/G5_PENDING。 |
| 2026-08-29 | v1.5 | AG-CONFORMANCE-CODE-S3E：新增 V-S12 与统一待验证列表；覆盖 PromptPackage/Gate 前置、SM-AG-02 loopError/handoff/多 ToolCall 顺序生产驱动、SM-AG-04 WaitingConfirm forbidden；真实 Tauri/provider 与范围外 LibreOffice 环境验证保持独立待办。 |
| 2026-08-29 | v1.6 | 回填 S3E 自动化与治理证据：AG 164/164、前端 592/592、Rust 376/377（唯一失败为范围外 LibreOffice Worker signal=6）、test:governance/governance:audit 通过；V-S12 保持 PARTIAL，未把本地证据升级为 G5。 |
| 2026-08-30 | v1.7 | AG-CONFORMANCE-CODE-S4A：新增 V-S13 与 VAL-AG-PROJECT-REPORT-01，分开登记 AgentContextFacts/ProjectReport 生产实现、本地自动化与真实 provider/WKWebView G5 证据。 |
| 2026-08-30 | v1.8 | 回填 S4A 完整影响证据：前端 593/593、Rust 379+5+4、TypeScript、test:governance 与 governance:audit 通过；LibreOffice 用例非沙箱复核通过；V-S13 仅保留真实 provider/WKWebView G5。 |
| 2026-08-21 | v1.1 | 工作区资源操作基线重构：用 V-C1~C3 替代 D-NEW 场景，新增 V-R1/R2 覆盖 DecisionRequest、直接磁盘执行、dirty 阻断、内容 Diff 失效、fail-fast 补偿、partial_failed、重启 Interrupted 与 WorkspaceTimeline 类型化恢复。 |
