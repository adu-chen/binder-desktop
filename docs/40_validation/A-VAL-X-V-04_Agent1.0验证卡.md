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
| 未保存新建文档：预览开标签、接受只裁决、拒绝丢弃、保存才建文件 | REQ-ED-010、REQ-WS-006、D-NEW-001~008 | BR-CORE-STATE-001、BR-SYS-STATE-008 | SM-ED-01、SM-WS-01 |
| AgentTurn：澄清非终态、任一阶段取消、首终态锁存、迟到事件无副作用 | REQ-AG-007、REQ-SYS-010 | BR-SYS-STATE-007 | SM-AG-01 |
| TaskRun：非后台队列、确认点、中断可重试、不跨重启续跑 | REQ-AG-015、REQ-SYS-011 | BR-SYS-STATE-008 | SM-AG-04 |
| 外部内容一次性 data-only、显式记忆选择 | REQ-REF-009、REQ-AST-001、REQ-SYS-012 | BR-SYS-DATA-007 | ExternalContentRef / MemorySelectionRef |
| 多会话/多 workspace 隔离 | REQ-SYS-009/010、REQ-ED-011 | BR-SYS-STATE-007 | AgentSession / AgentTurn 隔离 |

## 3. 状态化链路验证矩阵

| 链路 | 验证卡 | 验证点 | 门禁证据 |
|---|---|---|---|
| DE-DIFF（Proposal 链） | V-S1/S2/S3/S4 | 三分流 + 连续接力 | 自动化（deRuntime/edOperations）+ 真实环境 |
| ED-EDIT（LogicalState 承接） | V-S1/S3 | 预览投影、pending 迁移、未保存会话 | 自动化 + 真实环境 |
| WS-SAVE / WS-DIFF-APPLY | V-S1/S2/S5 | 保存先裁决、无 LogicalState 原子落盘、失败保留 | 自动化（edRuntime/rust 落盘）+ 真实环境 |
| AG-TURN（AgentTurn） | V-S6 | 澄清/取消/首终态 | 自动化（agRuntime）+ 真实环境 |
| AG-TURN（TaskRun） | V-S7 | 编排/确认点/中断重试 | 自动化 + 真实环境 |
| REF-INJECT / AST-MEMORY | V-S8 | 外部内容/记忆 data-only | 自动化 + 真实环境 |
| 会话隔离 | V-S9 | 多会话/多 workspace 不串线 | 自动化 + 真实环境 |

## 4. 核心场景卡

> 状态值：`DESIGNED`（已设计待执行）｜`PASS`（自动化+真实证据闭合，未经执行不得填报）。

### 4.1 V-N1 · 新建文档：生成、预览、拒绝（D-NEW N1）

| 字段 | 内容 |
|---|---|
| 前置条件 | 打开 workspace；请求"新建一份会议纪要" |
| 完整操作 | Agent 生成新文档 Proposal；预览成功自动打开可编辑标签；不编辑即拒绝 |
| 期望结果 | 预览成功后出现待保存标签与可编辑 LogicalState；文件树/磁盘无该文件；拒绝后标签关闭、逻辑文档清除，磁盘与文件树仍无文件（D-NEW-001/002/006） |
| 自动化证据设计 | deRuntime：空基准 Proposal→预览建 LogicalState→reject→丢弃；edRuntime：标签自动打开与关闭；rust：无 create_doc 调用 |
| 人类观察项 | 标签是否自动打开、拒绝后标签关闭、文件树无节点 |
| 状态 | `DESIGNED` |

### 4.2 V-N2 · 新建文档：生成、接受、编辑、保存（D-NEW N2）

| 字段 | 内容 |
|---|---|
| 前置条件 | V-N1 的 Proposal 已预览，未裁决 |
| 完整操作 | 接受；继续编辑；保存；关闭重开 |
| 期望结果 | 接受不重复插入内容、不写盘；保存成功后才在真实文件树出现文件；重开内容等于保存时内容（D-NEW-004、REQ-WS-006） |
| 自动化证据设计 | deRuntime：accept 只裁决；edRuntime：保存信封；rust：create 落盘 + 第二次读取；时间轴节点恰一个 |
| 人类观察项 | 接受后内容不变、保存后文件出现、重开一致 |
| 状态 | `DESIGNED` |

### 4.3 V-N3 · 新建文档：预览后编辑，原 Proposal 失效（D-NEW N3）

| 字段 | 内容 |
|---|---|
| 前置条件 | V-N1 的 Proposal 已预览 |
| 完整操作 | 手动编辑待保存新文档，或再次请求 Agent 编辑该文档；查看原卡片 |
| 期望结果 | 原卡显示"文档基准已变化"且接受/拒绝禁用；标签与当前 LogicalState 保留；保存当前草稿可进行但不得接受旧卡（D-NEW-005、BR-SYS-STATE-008） |
| 自动化证据设计 | deRuntime：revision advanced→expired；edRuntime：编辑后卡失效、会话保留；agRuntime：再次编辑以最新 LogicalState 为基准 |
| 人类观察项 | 卡片失效提示、标签与内容保留、保存行为 |
| 状态 | `DESIGNED` |

### 4.4 V-N4 · 新建文档：保存冲突/失败（D-NEW N4）

| 字段 | 内容 |
|---|---|
| 前置条件 | V-N2 的已接受未保存状态；目标路径被外部创建或磁盘写入失败 |
| 完整操作 | 保存；观察错误；重试或改路径 |
| 期望结果 | 显示失败原因；保留未保存 LogicalState 与标签；不显示成功节点、不丢内容（D-NEW-007） |
| 自动化证据设计 | rust：saveConflict/saveFail→SaveError；edRuntime：失败后保留 dirty；无时间轴节点 |
| 人类观察项 | 错误提示、内容保留、可重试 |
| 状态 | `DESIGNED` |

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
| 完整操作 | 写入歧义请求触发澄清；补充后继续；在流式中取消；观察首终态后到达的事件 |
| 期望结果 | 澄清是等待输入的非终态（不伪装完成）；取消任一活动阶段生效且无半落盘；完成/失败/拒绝/取消互斥、首终态锁存；迟到事件/工具无文档/卡片/落盘副作用 |
| 自动化证据设计 | agRuntime：澄清非终态、cancel、terminal_latch；rust：终态后迟到事件抑制 |
| 人类观察项 | 澄清等待呈现、取消后无新工具/提案、终态原因 |
| 状态 | `DESIGNED` |

### 4.11 V-S7 · TaskRun 编排（REQ-AG-015 / BR-SYS-STATE-008）

| 字段 | 内容 |
|---|---|
| 完整操作 | 复杂目标经 IntentResult 形成可展示 TaskPlan（步骤/依赖/当前游标/所需授权/确认点）；执行片段内连续读取与多 Proposal；任务级确认（DecisionRequest：权限/方案/范围/人工）后自动下一片段；中断后显式重试 |
| 期望结果 | **三层分离**：DiffCard 只确认内容（接受/拒绝不改变 TaskRun）；任务级确认走 DecisionRequest，confirm 后自动开始下一执行片段（无需新聊天消息、绝不后台续跑）；任意新消息不得绕过待确认（AG_DECISION_PENDING）；同文档连续 Diff 基于最新 LogicalState、前卡因基准推进失效；信息流底部实时当前动作（step_progress），回合终态（等待确认/等待补充/完成/失败/中断）来自运行事件 |
| 自动化证据设计 | agRuntime SM-AG-04 转移矩阵（decision 驱动 waiting_confirm、resolve confirm/cancel）；task.rs 生命周期测试（gate/不绕回/plan 持久化） |
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

## 5. P1 开放条件验证卡清单

每项 P1 按子能力、格式/解析能力、权限、确认/保存/恢复语义与真实验证**独立开放**；对应子卡在各项 P1
开放时细化（本清单为门禁框架）。未满足 §6 门禁前入口保持关闭。

| P1 能力 | 独立开放条件 | 验证卡（设计） |
|---|---|---|
| TXT→Markdown / Markdown→TXT 目标副本转换 | 来源不改写、预览建目标 LogicalState、接受不写盘、保存才落盘、损失告知、保存冲突/失败保留 | V-P1-01 |
| 外部文件内容读取 | 显式附加 + 可用只读解析 → 一次性 data-only；任务结束清除；不支持格式退回元数据 | V-P1-02（=V-S8 数据面） |
| 显式项目记忆选择 | workspace 本地用户管理；仅显式选择进入回合；不自动写入/提议/检索/注入 | V-P1-03（=V-S8 记忆面） |
| 文件树：建目录/单文件重命名/移动 | 逻辑文件树暂存、接受不写盘、逐项显式保存、保存前再核验、标签暂定路径 | V-P1-04 |
| 可恢复删除/恢复 | 红色逻辑暂存、递归清单/再核验、单项显式保存、脏会话前置、无永久删除 | V-P1-05 |
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
