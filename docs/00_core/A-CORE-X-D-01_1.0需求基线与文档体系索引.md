---
文档编号：CORE-X-D-01
文档状态：A
负责模块：CORE
文档职责：Binder Agent 1.0 的需求层导航、基线传播范围与后续设计同步清单
上游约束：A-CORE-C-P-01_APC.md、A-CORE-C-D-01_产品概述.md、A-CORE-X-D-02_Agent1.0冻结能力基线.md
直接承接：各模块 M-D（L1）需求描述、后续 L2 技术设计/状态机/接口/验证卡与开发计划的对齐工作
使用边界：本文只组织和索引已确认的需求基线，不定义 RULE/CHAIN/CONSTRAINT/接口字段，不替代模块 M-D 的 REQ 或 L2 技术设计
变更要求：1.0 范围、确认项或文档层级变化时，先更新产品根与能力基线，再更新本文的映射和状态；不得由本文反向扩大 Runtime Profile 或工具权限
治理成熟度：draft
---

# Binder Agent 1.0 需求基线与文档体系索引

## §1 目的与当前结论

本文将现有文档按“产品根 → 能力基线 → 模块需求 → 技术设计 → 验证”的职责重新串联，避免把
能力地图、需求、接口、状态机和代码事实混为同一层。

本轮已确认并已进入 L1 需求层的基线有十二组：

1. 新建文档绑定链：初始内容在确认前生成，WorkspaceOperationPlan 经前置裁决后创建磁盘空文件；随后打开标签、
   建立 `LogicalState` 并投影绑定初始 Diff。接受只裁决内容卡；仅资源、磁盘和逻辑基线均未推进时，拒绝可补偿删除本次创建文件。
2. `CAP-00`：Agent Runtime Foundation 作为 1.0 P0 前提，覆盖 AgentTurn、IntentResult、按需工具授权、
   会话隔离、事件呈现、恢复与 Trace。
3. `CAP-20` 的工作区内引用：编辑器选区、文件树/编辑器标签、会话标签等显式引用进入可见令牌并在发送时
   冻结身份、锚、范围和版本快照；外部资料读取仍为 P1。
4. `CAP-01`~`04` 的 R2 感知主链：基础聊天/任务、项目分析、编辑器状态感知和文档阅读分析均为 P0；项目
   分析先给结构概览、再按任务最小化读取正文；已打开文档以 LogicalState 为准，未打开文档标识 DiskState
   新鲜度；M1/M2 为 P0，多个既有文档写入 M4 保持 P1。
5. R3 的编辑边界：原生 DOCX 只读预览/分析，编辑请求创建新 BDocx 编辑副本；BDocx 是 DOCX→HTML
   编辑与模拟 Word 分页→DOCX 的处理路径而非格式；纯文本/Markdown 边界不变；HTML/BDocx 的字符基础样式为 P0，颜色/突出显示、段落样式、字号、字体族、表格和图片为 P1，页面和复杂布局为 P2。
6. R4-A 的网络研究边界：`CAP-14` 为 P1，只有用户明确要求搜索/核验或提供 URL 时才检索和有限读取公开网页；默认仅发送查询或用户提供的 URL，资料以含标题、域名、URL、获取时间的 data-only 来源供给，禁止爬虫、认证/交互、下载/导入和内网访问。
7. 工作区资源操作边界：`CAP-11/12` 的创建、重命名、移动、删除与恢复统一先生成不可变 WorkspaceOperationPlan，
   经 DecisionRequest 前置裁决和确认时再核验后直接改变 DiskState；不生成资源 DiffCard。批量失败即停并反向补偿，dirty 目标先阻断。
8. R4-C 的最小格式转换：`CAP-13` 为 P1，只有 TXT→Markdown、Markdown→TXT 两个方向，且均创建目标副本；来源不改写，预览成功建立目标 LogicalState 与标签，接受不写盘、保存才落盘。TXT→Markdown 不推断语义；Markdown→TXT 明示有损。HTML 与通用 DOCX 转换仍未确认。
9. R4-D 的删除与多个既有文档编辑：`CAP-12` 为 P1 的前置裁决删除/恢复，确认后直接写盘并由统一
   WorkspaceTimeline 记录可取回资源快照和类型化恢复；永久清理历史快照不纳入。`CAP-15` 仍须先确认并冻结目标清单，再逐文档独立 Proposal、裁决与保存。
10. R5-A 的运行底座：`agent-edit-1.0` 是唯一可启动的 1.0 profile，未知或缺失 profile 均在 Gate、provider、工具和 Proposal 前失败关闭；一个用户发送对应一个 AgentTurn，澄清等待、任一活动阶段取消、首终态锁存与迟到副作用抑制均为 P0。IntentResult 固定任务类型、目标、预期副作用、所需上下文、允许工具和确认点；用户只见结构化任务事件，不见模型内部推理。workspace / AgentSession / AgentTurn 三层隔离，其中 AgentSession 是展示、持久化和恢复边界；本地事实历史进入 P0，长期项目记忆仍后置。
11. R5-B 的受限复合任务：IntentResult 将复杂目标组织为 TaskRun 的多个受限步骤/AgentTurn，不是后台队列；用户可辨别真实执行、等待补充、等待卡片裁决、中断可重试与终态。明确的同文档复合目标可以连续生成 Proposal，但每张后续 Proposal 只以最新 LogicalState 为基准，后预览使前卡失效，禁止旧 payload 续编、补丁堆叠和任务级回滚；background pending 必须迁移或裁决后才可继续。
12. R5-C 的受控外部内容与项目记忆：外部文件读取为 P1，只有用户显式附加且存在可用只读解析路径时才作为当前任务的一次性 data-only 内容引用；不支持格式退回元数据，禁止扫描、自动复读或默认持久化。项目长期记忆为 P1 的本地 workspace 用户资产，只有明确选择才进入当前回合；Agent 1.0 不自动写入、提议、检索或注入，外部内容不自动转存。

> **R6 冻结状态：已冻结。**上述十二组构成 Agent 1.0 Baseline v1.0；权威冻结结论、变更规则与 DESIGN 输入见 [A-CORE-X-D-02_Agent1.0冻结能力基线.md](A-CORE-X-D-02_Agent1.0冻结能力基线.md)。冻结的是需求范围，不是实现或发布状态。

历史候选拆分和确认过程保留在 [R-AG-M-D-02_代理1.0能力基线与需求分解.md](../01_modules/AG/R-AG-M-D-02_代理1.0能力基线与需求分解.md)，仅供追溯；它不再包含有效候选项，也不得补充或解释当前冻结范围。任何未写入 CORE 冻结基线和对应 Active 模块 M-D 的能力，均不属于已确认的 Agent 1.0 需求。

## §2 文档体系与阅读顺序

```text
APC（项目边界与授权）
  └─ 产品概述 CORE-C-D-01（产品级 REQ-CORE-*）
       └─ CORE-X-D-02（Agent 1.0 冻结范围与档位）
            ├─ 本文（只做导航与传播检查）
            └─ 模块 Active M-D（L1 REQ，按模块承接）
                 └─ L2 技术设计 / M-S 状态机 / I-P 接口
                      └─ 验证卡、测试矩阵、实现与 Trace
```

阅读和修改的规则如下：

- 讨论“要不要做、何时做、什么算完成”时，先读产品概述与 CORE-X-D-02；若需改变范围，必须新开 REQUIREMENT Scope。
- 讨论某模块应承接什么用户可见行为时，读对应 M-D 的 `REQ-*`。
- 讨论状态枚举、数据字段、接口、UI 状态或工具实现时，必须在 L1 已确认后新开 L2 同步；不得把现有 L2
  的旧语义反向当作需求来源。
- 验证和代码只能追溯到已完成 REQ_MAP 的 L2 规则；本轮没有以 L1 需求文本直接驱动实现。

## §3 1.0 Profile 的能力分层

| 层级 | 进入 `agent-edit-1.0` 的含义 | 本轮状态 |
|---|---|---|
| P0 主链 | 未具备就不能宣称 Agent 1.0 闭环可用 | 已确认：CAP-00（含 R5-A 的唯一 profile、回合/意图/最小授权、结构化事件、AgentSession 隔离和本地事实历史）及 R5-B 的受限 TaskRun、真实工作状态和连续 Proposal 接力；CreateDocumentBundle、工作区内引用、CAP-01~04；原生 DOCX 只读与 BDocx 编辑副本路径；HTML/BDocx 的加粗、斜体、下划线、删除线 |
| P1 受控开放 | 可纳入 1.0，必须实现；按子能力、格式和真实验证独立开放 | HTML/BDocx 的颜色/突出显示、段落样式、字号、字体族、表格/图片；用户显式触发的公开网页研究；可用只读解析路径上的外部文件一次性内容读取；用户管理且显式选择的本地项目记忆；创建文件夹、单文件重命名/移动；可恢复删除/恢复；TXT→Markdown、Markdown→TXT 的目标副本转换；确认冻结目标清单后的多个既有文档逐项编辑 |
| P2 后续 Profile | 保留产品路线，不进入默认 Runtime Profile 或发布验收 | 构建、Agent 记忆提议/自动写入/自动注入、自动知识召回、模板工作流、复杂自主规划、多 Agent 等 |

“已注册工具”不等于“默认可调用”；P0/P1/P2 也不等于模型提示词中的文字开关。真正的调用条件必须在
后续 Runtime Profile、IntentResult、CapabilityGrant 与工具执行校验中落实。

## §4 本轮 L1 承接矩阵

| 文档 / 模块 | 1.0 关系 | 本轮处理 | 后续责任 |
|---|---|---|---|
| [产品概述](A-CORE-C-D-01_产品概述.md) / CORE | 产品级根 | REQ-CORE-010/012/014 锁定 CreateDocumentBundle、WorkspaceOperation 前置裁决直接写盘与 WorkspaceTimeline；REQ-CORE-011 锁定 R2 感知主链；REQ-CORE-004 锁定原生 DOCX 只读与 BDocx 副本编辑；REQ-CORE-013/015 保持最小转换与逐文档编辑；REQ-CORE-016~019 保持运行时、复合任务、外部内容与项目记忆 | 维持产品级边界；范围变化先经 APC |
| [冻结能力基线](A-CORE-X-D-02_Agent1.0冻结能力基线.md) / CORE | Agent 1.0 唯一冻结范围 | 持有 P0/P1/P2、不可突破边界和变更入口；不持有模块字段、状态或算法 | 范围变化只在 REQUIREMENT Scope 更新；本文不复制完整能力卡 |
| [代理](../01_modules/AG/A-AG-M-D-01_代理功能需求描述.md) / AG | P0/P1 主链 owner | REQ-AG-007 承接 R5-A 的 Runtime、IntentResult、最小授权、结构化事件、AgentSession 隔离与本地事实历史；REQ-AG-015 承接 R5-B 的 TaskRun、复合步骤、真实工作状态与连续 Proposal；REQ-AG-016/017 承接 R5-C 的外部内容消费与显式项目记忆引用；REQ-AG-008/009 承接新建与项目/文档分析；010~014 承接 R4 P1 能力 | AG L2：Runtime / TaskRun / Intent / Tool / Session / Trace |
| [Diff Edit](../01_modules/DE/A-DE-M-D-01_DiffEdit功能需求描述.md) / DE | P0 主链 owner | REQ-DE-006 承接 CreateDocumentBundle 创建目标后的初始内容 Diff；REQ-DE-007 明确资源操作不生成 DiffCard，只提供影响清单和失效；REQ-DE-008/009 保持多文档与连续 Proposal | DE L2：初始内容关联、基准比较、资源影响失效 |
| [编辑器](../01_modules/ED/A-ED-M-D-01_编辑器功能需求描述.md) / ED | P0 主链 owner | REQ-ED-010/012/014 承接确认后真实目标标签与初始内容投影；013/015 承接脏标签阻断、资源执行后路径/关闭同步；016 保持多文档隔离 | ED L2：标签事实、脏态前置、执行回执同步 |
| [工作台](../01_modules/WS/A-WS-M-D-01_工作台功能需求描述.md) / WS | P0/P1 主链 owner | REQ-WS-006 承接 CreateDocumentBundle；008 承接 WorkspaceOperation 前置裁决与直接磁盘执行；009 对齐转换目标；010 承接 WorkspaceTimeline 删除/恢复；011 保持多文档独立保存 | WS L2：A-WS-M-T-02 / M-S-02、接口、持久化与恢复 |
| [引用](../01_modules/REF/A-REF-M-D-01_引用功能需求描述.md) / REF | P0（工作区内）/P1（外部来源） | 冻结发送时引用快照、来源/新鲜度与 data-only；REQ-REF-009 承接 R5-C 的外部文件受控内容引用；用户选中项目记忆可作为引用；R4-A 公开网页来源可追溯；R4-B 路径基线推进；R4-D 删除失效与多文档目标清单身份不构成写权 | REF L2：引用身份/锚/范围/版本校验 |
| [系统底座](../01_modules/SYS/A-SYS-M-D-01_系统底座功能需求描述.md) / SYS | P0/P1 支撑 | REQ-SYS-008/009 共享生命周期与只读上下文边界；REQ-SYS-010 承接 R5-A 运行底座；REQ-SYS-011 承接 R5-B 的 TaskRun/步骤/Proposal 关联、真实进度与恢复隔离；REQ-SYS-012 承接 R5-C 的外部内容/记忆隔离与保留契约 | SYS L2：共享字段、事件、持久化与总图 |
| [构建](../01_modules/CBT/A-CBT-M-D-01_构建功能需求描述.md) / CBT | P2 | 明确退出默认 Profile，保留路线 | 未来按多文档预览/显式保存重新设计 |
| [记忆](../01_modules/AST/A-AST-M-D-01_记忆协同功能需求描述.md) / AST | P1 用户资产 / P2 自动能力 | 1.0 的本地 workspace 项目记忆由用户手工管理、显式选择才可引用；自动写入、提议、检索/注入保持 P2 | 未来自动供给需独立授权和评测 |
| [知识库](../01_modules/KB/A-KB-M-D-01_知识库功能需求描述.md) / KB | P2 自动能力 | 1.0 禁止自动召回；显式引用可用 | 未来的召回、来源呈现和评测 |
| [模板工作流](../01_modules/TPL/A-TPL-M-D-01_模板工作流功能需求描述.md) / TPL | P2 | 默认 Profile 不执行工作流 | 未来模板执行与 CBT 的状态机 |
| DOC / PRV | P0 原生 DOCX 读取与 BDocx 副本编辑路径支撑；P1 最小文本转换 | 原生 DOCX 只读预览链与 BDocx 编辑副本链分离；REQ-DOC-006 承接 TXT→Markdown、Markdown→TXT 的内容转换与损失告知；BDocx 字符样式 P0，颜色/段落/字号/字体族及表格/图片 P1 | DOC/PRV L2：转换、预览与原文件保护契约 |
| ACCT / ENT | 安全支撑 | 无语义冲突，本轮不改 | 保持模型门禁、账户和订阅边界 |

## §5 需求层已完成与未完成的同步边界

**本轮已完成：**

- 产品根的写入语义及 REQ-CORE-010；
- AG、DE、ED、WS、REF、SYS 的直接需求承接；
- CBT、AST、KB、TPL 的默认 Profile 延期边界；
- 需求文件的导航、责任和后续入口。
- R2：CAP-01~04 的 P0 档位、项目分析两阶段读取、LogicalState/DiskState 新鲜度、M1/M2/M4 与 data-only 引用边界。
- R3：原生 DOCX 只读、BDocx 编辑副本路径、纯文本/Markdown 边界；HTML/BDocx 字符基础样式 P0，颜色/突出显示、段落样式、字号、字体族、表格和图片 P1；页面级布局、页眉页脚、复杂样式继承和任意定位为 P2。
- R4-A：用户显式触发的公开网页检索与有限读取为 P1；默认仅发送查询，来源可追溯且为 data-only；自动/批量爬虫、认证/交互、下载/导入和内网访问不纳入 1.0。
- WorkspaceOperation：创建文档/文件夹、单文件重命名/移动、删除/恢复为 P1；先冻结操作计划并前置裁决，确认时再核验后直接写盘；资源操作不产生 DiffCard，dirty 目标阻断，关联内容 Diff 在确认后失效。
- R4-C：TXT→Markdown、Markdown→TXT 为 P1 的目标副本转换；来源不改写，预览建立目标 LogicalState 与标签，接受不写盘、保存才落盘；TXT→Markdown 不推断语义，Markdown→TXT 明示有损；HTML 与通用 DOCX 转换不在本子项。
- WorkspaceTimeline：现有 DocumentTimeline 保留内容分支，新增 WorkspaceOperationEntry 记录路径、目录、快照和执行回执；删除/恢复均经新的前置裁决，统一时间轴负责类型化前进式恢复，不另建撤销入口。
- CreateDocumentBundle：确认前冻结初始内容，确认后创建空文件并显示绑定初始 Diff；安全拒绝可补偿删除，任何基线推进使旧卡失效并禁止删除。
- R5-A：`agent-edit-1.0` 是唯一可启动的 P0 profile，缺失/未知 profile 在 Gate、provider、工具和 Proposal 前失败关闭；澄清等待、任一活动阶段取消、首终态锁存、IntentResult 最小授权、非推理结构化事件、AgentSession 隔离和本地事实历史均已由 CORE/AG/SYS 直接 L1 承接。
- R5-B：复杂目标由受限 TaskRun 组织步骤和 AgentTurn，真实工作状态必须可见；同文档后续 Proposal 只以当前 LogicalState 接力，后预览使前卡失效，背景 pending 不可直接续编；取消不回滚既有预览，中断不自动续跑、显式重试创建新回合。CORE/AG/DE/SYS 已完成直接 L1 承接。
- R5-C：外部文件的实际内容读取确认仅为用户显式附加、可用只读解析格式的 P1 一次性 data-only 路径；未读内容不伪装为已读，不扫描、自动复读或默认持久化。长期项目记忆确认是 P1 的本地 workspace 用户资产，只有用户选择才作为引用进入当前回合；Agent 1.0 的自动写入、提议、检索和注入保持 P2。CORE/AG/REF/AST/SYS 已完成直接 L1 承接。
- R6：上述需求范围已冻结为 Agent 1.0 Baseline v1.0；后续需求变化必须新开 REQUIREMENT Scope，不能由 DESIGN、Prompt、工具或实现隐式引入。冻结不等于技术设计、代码或真实验证完成。

**本轮刻意未做：**

- 不以现有直接删除、资源 Diff 或文档级内容时间轴实现反向定义新基线；相关实现进入独立迁移 Scope。
- 不宣布任何 P0 能力“已冻结”或“已发布”；L1 承接完成不等于接口、实现或验证已完成。
- 不调整不受本轮语义影响的 DOC、PRV、ACCT、ENT 需求文本，避免为“全覆盖”制造无意义变更。

## §6 后续需求与设计同步顺序

R3、网络研究、格式转换、多既有文档、运行底座、受限复合任务及外部内容/项目记忆保持既有冻结边界。2026-08-21 人类进一步确认：文件树相关能力统一改为 WorkspaceOperation 前置裁决直接写盘，恢复统一进入 WorkspaceTimeline，新建采用 CreateDocumentBundle。技术 SSOT 见 A-WS-M-T-02 与 A-WS-M-S-02；实现不得继续使用资源 Diff 或“接受后等待文件树保存”的旧链。

R6 形成完整冻结基线后，再以一个独立的设计同步任务，严格按以下顺序推进并可逐项验收：

1. SYS：确定 TaskRun/步骤/AgentTurn/Proposal、会话与真实进度的共享对象边界和跨模块事件，不先锁具体数据库字段；
2. DE + ED + WS：完成“预览 / 裁决 / 基准推进失效 / 连续 Proposal 接力 / 显式保存 / 保存失败 / 关闭”状态机和接口契约；
3. AG + REF + AST：完成 TaskRun、AgentTurn、IntentResult、CapabilityGrant、会话隔离、引用快照、外部内容暂存/清除、显式记忆引用、澄清/中断/新回合重试的运行时契约；
4. 针对已完成的状态机写验证卡，至少覆盖 CreateDocumentBundle 的确认/投影/安全拒绝/失效、WorkspaceOperation 的拒绝/重验/补偿/重启中断，以及多会话引用隔离；
5. 仅在以上设计和验证入口完成后，再修改实现、提示词组装或工具清单。

每一步的“完成”必须有对应的 REQ_MAP、状态机/接口引用和可执行验证证据；任何新增 Profile 能力都要回到
AG-M-D-02 先确认，再进入本索引和模块 M-D。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-13 | v1.0 | 初版：建立 Agent 1.0 需求体系索引，记录 D-NEW、CAP-00、CAP-20 工作区内引用的 L1 传播范围；列出 14 个模块需求文档的 Profile 关系、已完成的 L1 对齐和下一轮 L2 设计同步顺序。 |
| 2026-08-13 | v1.1 | AG-1.0-R2-REQUIREMENTS-01：纳入 CAP-01~04 的 P0 确认及其 Core/AG/ED/REF/WS/SYS L1 承接；明确 R3~R5 仍是需求确认，不得跳过能力冻结直接进入设计或实现。 |
| 2026-08-13 | v1.2 | AG-1.0-R3-REQUIREMENTS-01：纳入原生 DOCX 只读、BDocx 编辑副本处理路径与 HTML/BDocx 表格图片 P1；明确 R3 尚缺样式子集确认，DOC/PRV 成为直接 L1 消费者。 |
| 2026-08-13 | v1.3 | R3 自检收紧：DOC/PRV 的 P0 表述改为原生 DOCX 读取与 BDocx 副本路径支撑；不以笼统“P1 编辑支撑”掩盖表格/图片仅在副本路径 P1 开放的边界。 |
| 2026-08-14 | v1.4 | AG-1.0-R3-STYLE-REQUIREMENTS-01：R3 样式需求完成确认；字符基础样式纳入 HTML/BDocx P0，颜色/突出显示、段落样式、字号、字体族、表格和图片纳入 P1，字体族明确包含 BDocx；验证固定为实现后的开放门槛。 |
| 2026-08-14 | v1.5 | AG-1.0-R4-NETWORK-REQUIREMENTS-01：R4-A 确认 CAP-14 为 P1 受控网络研究；AG 承接显式触发、公开网页有限读取、最小数据外发、来源化分析与不直接写入，REF 承接可追溯的 data-only 来源引用。 |
| 2026-08-14 | v1.6 | AG-1.0-R4-STRUCTURE-REQUIREMENTS-01：R4-B 确认 CAP-11 的建目录、单文件重命名/移动先进入逻辑文件树，接受不写盘，逐项显式保存才写盘；CORE/AG/WS/ED/REF 已完成直接 L1 承接，现有直写实现留待 DESIGN/实现迁移。 |
| 2026-08-14 | v1.7 | AG-1.0-R4-CONVERSION-REQUIREMENTS-01：R4-C 确认 CAP-13 的最小 P1 格式矩阵：TXT→Markdown、Markdown→TXT 均创建目标副本，来源不改写，预览建立目标 LogicalState、接受不写盘、显式保存才落盘；Markdown→TXT 明示有损。CORE/AG/DE/DOC/ED/WS 已完成直接 L1 承接，HTML 与通用 DOCX 转换仍后置。 |
| 2026-08-14 | v1.8 | AG-1.0-R4-DELETE-MULTIDOC-REQUIREMENTS-01：R4-D 确认 CAP-12 的可恢复删除/恢复与 CAP-15 的已确认冻结目标清单后的逐文档编辑；CORE/AG/DE/ED/WS/REF 已完成直接 L1 承接。当前实现中的直接物理删除、接受即删除和宽松 cross_doc 只登记为后续设计/迁移债务。 |
| 2026-08-14 | v1.9 | AG-1.0-R5A-RUNTIME-REQUIREMENTS-01：R5-A 确认 CAP-00/16~18 的运行时最小边界；REQ-CORE-016、REQ-AG-007 与 REQ-SYS-010 完成直接 L1 承接。唯一 `agent-edit-1.0`、缺失/未知 profile 失败关闭、澄清/取消/终态、IntentResult 最小授权、非推理事件、AgentSession 隔离和本地事实历史进入 P0；R5-B/R5-C 及 DESIGN/实现继续后置。 |
| 2026-08-14 | v1.10 | AG-1.0-R5B-TASK-ORCHESTRATION-REQUIREMENTS-01：R5-B 确认 CAP-16 的受限 TaskRun、复杂目标多步骤/多 AgentTurn、真实工作状态和同文档连续 Proposal 接力；REQ-CORE-017、REQ-AG-015、REQ-DE-009、REQ-SYS-011 完成直接 L1 承接。前卡基准推进失效、background pending 限制、取消不回滚、重启不自动续跑进入基线；仅 R5-C 及 DESIGN/实现继续后置。 |
| 2026-08-14 | v1.11 | AG-1.0-R5C-MEMORY-EXTERNAL-READ-REQUIREMENTS-01：R5-C 确认外部文件为 P1 的显式、一次性 data-only 内容读取，未读格式退回元数据；项目长期记忆为 P1 的 workspace 本地用户资产，只有显式选择才能进入回合。REQ-CORE-018/019、REQ-AG-016/017、REQ-REF-009、REQ-SYS-012 与 AST 1.0 边界完成直接 L1 承接；自动记忆能力仍为 P2。 |
| 2026-08-14 | v1.12 | AG-1.0-R6-BASELINE-FREEZE-01：R1–R5-C 的十二组已确认需求冻结为 Agent 1.0 Baseline v1.0；索引、产品根、能力基线同向引用冻结记录。后续仅可按有序 DESIGN Scope 细化，需求变化必须另开 REQUIREMENT Scope；冻结不宣称实现、验证或发布完成。 |
| 2026-08-21 | v1.13 | WORKSPACE-OPERATION-BASELINE-01：以人类确认的八项决策替代 R4-B/R4-D 旧逻辑树暂存语义；登记 WorkspaceOperation 前置裁决直接写盘、WorkspaceTimeline 统一恢复、CreateDocumentBundle 与失败补偿边界。 |
| 2026-08-31 | v1.14 | AG-DOC-CONSOLIDATION-01：将 Agent 1.0 范围统一指向 A-CORE-X-D-02，AG-M-D-01 保留模块需求真源职责；移除候选基线作为 Active 上游的循环与竞争。 |
