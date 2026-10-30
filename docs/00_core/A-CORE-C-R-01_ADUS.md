---
文档编号：CORE-GOV-01
文档状态：A
负责模块：CORE
文档职责：ADU 派生检索索引
上游约束：CORE-GOV-00
直接承接：ADU / ADP / CLAUDE 的规则、链路、代码块检索
使用边界：不定义治理规则，不定义开发过程，不替代 adu.md、ADP.md 或 CLAUDE.md
变更要求：运行 npm run governance:generate 刷新自动生成区
---

# ADU 派生检索索引

本文件由 `governance:generate` 自动生成。
所有索引内容来自设计文档标注块、源代码 @GOV 标注、测试覆盖注释和 triage 报告。
运行 `npm run governance:generate` 刷新。

边界：
- adu.md 是静态治理规则源。
- ADP.md 是开发过程运行时协议。
- CLAUDE.md 是 AI 会话入口与执行路由协议。
- adus.md 只提供派生检索索引，不定义规则、不定义过程、不承载人类裁决。

自动生成边界：
- `## 【自动生成区】` 之前仅保留元信息头部，包括文档头、边界说明和变更记录。
- 元信息头部不得写入规则、链路、模块、约束、裁决或索引内容。
- `## 【自动生成区】` 及之后内容禁止人工编辑，应由 `npm run governance:generate` 刷新。

---

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-09 | v1.0 | 初始版本 |

---

## 【自动生成区】

<!-- 自动生成。禁止人工编辑。-->
<!-- 运行：npm run governance:generate -->
<!-- 最后生成时间：2026-10-30T16:31:19.035Z -->

### 模块编码表

| module_code | module_name | description                                                              | 来源文档                                       |
| ----------- | ----------- | ------------------------------------------------------------------------ | ------------------------------------------ |
| PCG         | 产品能力图谱      | 将产品能力、格式/角色变体、验收点、治理引用和验证证据组织为单一注册表，并生成产品沙盘和审计结果                         | docs/00_core/A-CORE-C-P-02_产品能力图谱规范.md     |
| CORE        | 核心          | 项目核心与治理，跨模块产品级定义                                                         | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| SYS         | 系统底座        | 整体架构、IPC 协议、跨模块契约与模块编码基线                                                 | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| WS          | 工作台         | workspace 生命周期、文件 I/O、缓存、pending_diffs 持久化、文档级时间轴与历史回溯                   | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| ED          | 编辑器         | 编辑器运行时（TipTap，md/docx 富文本编辑）                                             | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| DE          | Diff Edit   | diff 生成、审阅、接受/拒绝/过期终态                                                    | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| AG          | 代理（Agent）   | 代理编辑（AgentEdit）面板：编辑/构建双模式回合、工具调用、辅助续写与局部修改                              | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| CBT         | 构建（Build）   | 项目蓝图与多文件文档项目构建、构建中断、构建移交/回落（旧称 Chat Build / Chat↔Build 接管，2026-07-04 更名） | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| DOC         | 文档转换        | DOCX/Pandoc 格式转换                                                         | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| PRV         | 预览输出        | Excel/PPT 预览、音视频预览与打印/导出（不含 Excel/PPT 编辑）                                | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| REF         | 引用          | 引用（@mention）解析与上下文注入                                                     | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| AST         | 记忆          | 本地记忆协同                                                                   | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| KB          | 知识库         | 本地知识库                                                                    | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| TPL         | 模板          | 模板与工作流                                                                   | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| ACCT        | 账号          | 账号、认证、会话（云端认证客户端）                                                        | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| ENT         | 订阅门控        | 订阅门控、用量与 AI 网关客户端                                                        | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |

### 规则域注册表

| domain_code | description      | 来源 |
| ----------- | ---------------- | -- |
| STATE       | 状态语义、生命周期、阶段闭合   | 内置 |
| DATA        | 数据结构、字段定义、schema | 内置 |
| PERSIST     | 持久化、恢复、重建        | 内置 |
| VERIFY      | 验证、审计、一致性检查      | 内置 |
| GOV         | 文档治理、命名、变更控制     | 内置 |
| OBS         | 可观测性、错误处理、恢复     | 内置 |

### 需求注册表

| req_id        | title                                  | owner_module | status   | summary                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | 来源文档                                             |
| ------------- | -------------------------------------- | ------------ | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------ |
| REQ-CORE-001  | 本地优先存储与 workspace 边界                   | CORE         | active   | 用户内容全部本地存储，workspace 是文件操作/搜索/diff/上下文读取的唯一边界                                                                                                                                                                                                                                                                                                                                                                                                                                             | docs/00_core/A-CORE-C-D-01_产品概述.md               |
| REQ-CORE-002  | 对话生成文档提案并由人类裁决                         | CORE         | active   | AI 文档修改以 diff/ChangeProposal 呈现并由用户接受或拒绝；受控预览可更新可编辑的文档逻辑状态，显式接受不重复应用或直接写盘，而保存有效 pending 预览等同接受并写盘；Agent 1.0 只包含 `agent-edit-1.0` 的代理编辑与工具调用，构建由独立后续 Profile 承接                                                                                                                                                                                                                                                                                                                            | docs/00_core/A-CORE-C-D-01_产品概述.md               |
| REQ-CORE-003  | agent 多文件文档项目构建（CBT）                   | CORE         | active   | 支持 agent 跨多文件构建文档项目（对话阶段→项目蓝图→显式触发→计划确认→执行），含构建中断与构建移交/回落（旧称 Chat↔Build 接管，2026-07-04 更名）                                                                                                                                                                                                                                                                                                                                                                                                 | docs/00_core/A-CORE-C-D-01_产品概述.md               |
| REQ-CORE-004  | 多格式预览、原生 DOCX 只读与 BDocx 编辑副本           | CORE         | active   | 原生 DOCX 仅可本地预览、读取和分析，不能直接编辑或原地保存；编辑请求必须以 Proposal 创建新的 BDocx 编辑副本。BDocx 是处理路径而非文件格式：副本磁盘格式仍为 DOCX，编辑时转为 HTML 并以模拟 Word 分页呈现，保存时再转换为 DOCX。Excel/PPT/音视频仅预览、打印与导出；不含 Excel/PPT 编辑                                                                                                                                                                                                                                                                                                           | docs/00_core/A-CORE-C-D-01_产品概述.md               |
| REQ-CORE-005  | 本地智能资产（记忆/知识库/模板）                      | CORE         | active   | 记忆协同、知识库、模板工作流均本地存储与运行                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | docs/00_core/A-CORE-C-D-01_产品概述.md               |
| REQ-CORE-006  | 引用解析与上下文注入（@mention）                   | CORE         | active   | 用户可通过选区、工作区文档/标签、会话标签等显式引用向 Agent 提供可追溯的 data-only 上下文；引用在发送时冻结身份、锚、范围和版本，不自动获得编辑权限。外部资料与外部文件读取不属于本要求的 P0 范围                                                                                                                                                                                                                                                                                                                                                                              | docs/00_core/A-CORE-C-D-01_产品概述.md               |
| REQ-CORE-007  | 文档级时间轴与历史回溯                            | CORE         | active   | 提供文档级版本时间轴与历史回溯/还原（v1 范围，归 WS）                                                                                                                                                                                                                                                                                                                                                                                                                                                            | docs/00_core/A-CORE-C-D-01_产品概述.md               |
| REQ-CORE-008  | 账号认证、订阅门控与 AI 网关（云薄端）                  | CORE         | active   | 登录/订阅校验为调用模型硬门；密钥仅服务端持有，客户端不持密钥、不可绕过门禁；套餐经网关计量，BYOK（v1 GA 纳入）经网关透传不计额度但仍需有效订阅                                                                                                                                                                                                                                                                                                                                                                                                             | docs/00_core/A-CORE-C-D-01_产品概述.md               |
| REQ-CORE-009  | 离线边界                                   | CORE         | active   | 编辑/预览/打印/本地功能离线可用；AI 功能需在线鉴权 + 有效订阅                                                                                                                                                                                                                                                                                                                                                                                                                                                       | docs/00_core/A-CORE-C-D-01_产品概述.md               |
| REQ-CORE-010  | 文档逻辑状态优先与新建文档绑定生命周期                    | CORE         | active   | 普通文档内容修改仍首先作用于 LogicalState 并遵守内容 Diff 后置裁决。Agent 新建文档是受前置裁决的 WorkspaceOperation：初始内容在确认前生成并冻结，用户确认后先创建 DiskState 空文件并记录资源操作历史，再打开标签、建立 LogicalState 并投影绑定初始内容 Diff。绑定初始 Diff 遵守普通内容 Diff 生命周期；只有文件身份、空文件磁盘基线和 LogicalState 版本均未推进时，拒绝才可补偿删除本次创建文件                                                                                                                                                                                                                                        | docs/00_core/A-CORE-C-D-01_产品概述.md               |
| REQ-CORE-011  | Agent 1.0 的项目与文档上下文感知                  | CORE         | active   | Agent 1.0 P0 必须支持基础聊天/任务交互、工作区概览与可复核项目分析、当前编辑器状态感知，以及当前文档、显式引用和未打开工作区文档的受限阅读与比较。项目分析默认先读取结构概览，模型再按任务与现有证据选择最小必要正文；运行时记录实际读取证据并派生项目报告。已打开文档以 LogicalState 为准，未打开文档才读取 DiskState 并标明读取时点/新鲜度                                                                                                                                                                                                                                                                                                | docs/00_core/A-CORE-C-D-01_产品概述.md               |
| REQ-CORE-012  | 工作区资源操作的前置裁决与磁盘提交                      | CORE         | active   | 创建文档/文件夹、重命名、移动、删除和恢复统一属于 WorkspaceOperation。系统先只读统计目标、影响和基线并冻结 WorkspaceOperationPlan，由 DecisionRequest 前置裁决；用户确认且执行前再核验成功后，WS 直接改变 DiskState，不生成资源 DiffCard或再次等待接受。批量操作采用失败即停和反向补偿，不承诺文件系统级严格事务                                                                                                                                                                                                                                                                                         | docs/00_core/A-CORE-C-D-01_产品概述.md               |
| REQ-CORE-013  | 最小格式转换与目标副本生命周期                        | CORE         | active   | Agent 1.0 P1 仅支持 TXT→Markdown 与 Markdown→TXT 的受控格式转换，转换一律创建新的目标副本且不改写来源。已打开来源读取当前 LogicalState，未打开来源读取 DiskState 并标明读取时点；预览成功后建立目标 LogicalState 与标签，接受只裁决转换 Proposal，显式保存当前目标才写入目标 DiskState。Markdown→TXT 为有损转换，必须明示损失边界                                                                                                                                                                                                                                                                  | docs/00_core/A-CORE-C-D-01_产品概述.md               |
| REQ-CORE-014  | 工作区时间轴中的资源删除与恢复                        | CORE         | active   | 删除与恢复是 WorkspaceOperation：完整影响清单和资源快照在确认前冻结，DecisionRequest 前置裁决后重新核验并直接改变 DiskState。产品不建设第二套撤销入口，而将现有文档时间轴扩展为统一 WorkspaceTimeline；资源节点保存前后路径、目录清单、内容/元数据快照和逐项执行回执，由类型化还原器前进式恢复。永久清理历史快照不属于 Agent 1.0                                                                                                                                                                                                                                                                                     | docs/00_core/A-CORE-C-D-01_产品概述.md               |
| REQ-CORE-015  | 多个既有文档的受控独立编辑                          | CORE         | active   | Agent 1.0 P1 可修改用户明确选定或确认候选清单后的多个既有、已保存且可编辑文档；目标清单在确认后冻结，不得动态扩展。每个目标独立生成 Proposal、基准、预览、裁决、保存与失败结果，并逐目标遵循 LogicalState/DiskState 三分流；不提供批量接受、批量保存、跨文档全有或全无事务。范围仅含 TXT、Markdown、HTML 与 BDocx 编辑副本，不含原生 DOCX、删除/文件树整理/格式转换混合操作                                                                                                                                                                                                                                                              | docs/00_core/A-CORE-C-D-01_产品概述.md               |
| REQ-CORE-016  | Agent 运行时、意图授权与会话隔离                    | CORE         | active   | Agent 1.0 P0 只能以 agent-edit-1.0 启动；未知或缺失运行配置必须在模型、工具与 Proposal 前失败关闭，Build/CBT 保持 P2。每个用户发送对应一个 AgentTurn，澄清为等待用户输入的非终态，取消可在任一活动阶段生效，首个终态后迟到事件/工具不得产生副作用。模型负责结合当前输入与真实工具证据形成并持续更新 IntentResult、选择工具和调整路线；IntentResult 只表达任务目标、编辑目标候选、预期副作用、所需上下文、歧义与确认点，不授予工具权限。运行时依据 profile、工具注册、显式目标、格式/动态能力、DecisionRequest、Gate、预算和终态生成并逐次校验 CapabilityGrant。用户只见结构化任务事件，不见模型内部推理；workspace/AgentSession/AgentTurn 三层隔离，历史与编辑事实本地保存                                                                  | docs/00_core/A-CORE-C-D-01_产品概述.md               |
| REQ-CORE-017  | 受限复合任务、真实进度与连续 Proposal                | CORE         | active   | 简单目标由单个 AgentTurn 完成；复杂或跨执行片段目标可形成 TaskRun，作为同一用户目标、当前路线、阶段、已完成事实、等待点与关联 AgentTurn/Proposal 的可观察投影。模型可依据新证据更新后续路线，运行时从事件事实投影 TaskRun/TaskPlan，不以静态计划调度模型、不预授权未来工具，也不自动跨确认点、跨重启或脱离用户续跑。明确的同文档复合目标可连续产生 Proposal：后续 Proposal 必须读取最新 LogicalState，成功预览推进基准并使前卡失效、保留历史且不可再裁决；连续卡不构成补丁栈或任务级回滚。无 LogicalState 的背景 pending Proposal 不得作为后续编辑输入。用户可辨别真实执行、等待补充、等待卡片裁决、中断可重试及终态，且不见模型内部推理                                                                                                          | docs/00_core/A-CORE-C-D-01_产品概述.md               |
| REQ-CORE-018  | 受控外部文件内容读取                             | CORE         | active   | Agent 1.0 以 P1 提供用户显式附加/选择的外部文件内容读取。只有系统当时具备可用只读解析能力的格式，才可在当前用户任务中形成可追溯的一次性 data-only 内容引用；不支持或不可读的文件只保留元数据引用并如实说明。系统不得扫描外部目录、自动再次读取、导入或编辑外部文件；外部内容不授予写入或执行权限，也不默认写入会话历史或项目记忆                                                                                                                                                                                                                                                                                                             | docs/00_core/A-CORE-C-D-01_产品概述.md               |
| REQ-CORE-019  | 用户管理的本地项目长期记忆                          | CORE         | active   | Agent 1.0 以 P1 提供 workspace 隔离、本地保存的项目长期记忆；用户可创建、查看来源、编辑、删除和关闭条目。它不是会话/编辑事实或外部文件的自动副本。Agent 1.0 不自动写入、不生成写入提议、不按相关性自动检索或注入；只有用户明确选择的记忆条目，才可作为 data-only 引用进入当前 AgentTurn                                                                                                                                                                                                                                                                                                                  | docs/00_core/A-CORE-C-D-01_产品概述.md               |
| REQ-ACCT-001  | 登录登出与会话生命周期                            | ACCT         | active   | 承接 SM-SYS-01 会话段运行时：登录（loginOk 持有 SessionToken）/登出（清会话）/会话过期（sessionExpire 回未认证）；客户端不重定义会话状态机                                                                                                                                                                                                                                                                                                                                                                                             | docs/01_modules/ACCT/A-ACCT-M-D-01_账号会话功能需求描述.md |
| REQ-ACCT-002  | 会话令牌安全持有与续期                            | ACCT         | active   | SessionToken 经系统安全存储持有（不明文落盘、不入日志），按云契约续期，失效即清除；令牌仅用于门禁与网关调用（经 ENT/SYS 契约）                                                                                                                                                                                                                                                                                                                                                                                                                | docs/01_modules/ACCT/A-ACCT-M-D-01_账号会话功能需求描述.md |
| REQ-ACCT-003  | 认证态供给                                  | ACCT         | active   | 向 ENT（门禁提交需令牌）与上层模块供给认证态查询与变更通知；统一错误形状中 reauth_required=true 一律触发重认证流程，不静默吞掉                                                                                                                                                                                                                                                                                                                                                                                                              | docs/01_modules/ACCT/A-ACCT-M-D-01_账号会话功能需求描述.md |
| REQ-ACCT-004  | 离线行为                                   | ACCT         | active   | 未认证或断网不影响编辑/预览/本地功能；仅 AI 链路按门禁语义受限；断网时不反复弹认证打扰                                                                                                                                                                                                                                                                                                                                                                                                                                            | docs/01_modules/ACCT/A-ACCT-M-D-01_账号会话功能需求描述.md |
| REQ-AG-001    | 代理编辑回合（编辑模式）                           | AG           | active   | 提供代理多轮回合（编辑、回答与分析）：上下文组装、流式回显、任务澄清、回合中断/取消和重试；AI 修改建议交 DE 生成 pending_diff，AG 不直接写盘；默认目标当前文档，用户显式指向可跨既有文档，有初始内容的新建文档按 REQ-AG-008 的 CreateDocumentBundle 前置裁决路径执行，明确零字节 TXT 按同一 WorkspaceOperationPlan 前置裁决路径直接创建。原生 DOCX 只读，编辑请求转为创建 BDocx 编辑副本的受控新建流程                                                                                                                                                                                                                                     | docs/01_modules/AG/A-AG-M-D-01_代理功能需求描述.md       |
| REQ-AG-002    | agent 工具调用回合                           | AG           | active   | 支持模型依据当前输入、上下文和工具证据自适应选择下一工具、澄清或结束的调用循环；每次调用由运行时按 profile、工具注册、显式目标、格式/动态能力、DecisionRequest、Gate、预算、取消与终态计算并校验当前 CapabilityGrant，循环可中断且有上限。文档内容写入必须走 DE diff 路径；HTML 编辑文档与 BDocx 处理副本的已确认样式、表格和项目图片修改生成受控结构化 Diff；原生 DOCX 不获得写工具；WorkspaceOperation 只能生成计划并前置裁决，确认后由 WS 再核验并直接修改 DiskState；新建、多既有文档和格式转换分别遵循 REQ-AG-008/014/012                                                                                                                                                           | docs/01_modules/AG/A-AG-M-D-01_代理功能需求描述.md       |
| REQ-AG-003    | 模型调用门禁与网关前置                            | AG           | active   | AG 的每一次模型调用（代理/工具回合/局部修改/辅助续写）必经订阅门禁（SM-SYS-01 GatePassed）并经 AI 网关发起；无有效订阅或断网时明确拒绝，无客户端绕过路径                                                                                                                                                                                                                                                                                                                                                                                               | docs/01_modules/AG/A-AG-M-D-01_代理功能需求描述.md       |
| REQ-AG-004    | 上下文组装与注入消费                             | AG           | active   | 模型先基于当前输入和已知事实形成结构化 IntentResult，并可随工具证据更新任务类型、目标候选、预期副作用、所需上下文、歧义与确认点；运行时据此组装当前编辑器 LogicalState、REF 显式引用、同会话必要历史与按策略启用的本地补强资产。项目分析先消费工作区概览，正文按任务需要和预算最小化读取；上下文读取一律经 WS 且限定 workspace 内，不可信文档/引用/知识仅作为数据                                                                                                                                                                                                                                                                                  | docs/01_modules/AG/A-AG-M-D-01_代理功能需求描述.md       |
| REQ-AG-005    | 辅助续写与局部修改                              | AG           | active   | 承载辅助续写（Autocomplete）与局部修改（InlineAssist）业务回合（2026-06-11 归 AG；2026-07-04 澄清定义；2026-07-16 人类确认细化）：快捷键手动触发（防抖/防重）发起轻量模型调用（同过门禁）；新请求覆盖旧请求；辅助续写一次严格返回 3 个 50–100 字候选；局部修改支持选区/光标两态（无选区为光标插入）、弹窗内多轮、对话不持久化，多轮只传上一轮候选结果与新指令；候选经 ED 装饰层/overlay 语义呈现，视觉浮层必须贴近光标/选区视觉锚点并支持上下翻转，不得仅钳制到编辑器窗口边缘；浮层放置采用稳定静态尺寸模型，尺寸应贴近初始输入态/候选态的真实控件大小，不引入运行时测量反向驱动状态；编辑器缩放只影响视觉锚点坐标，不得压缩浮层控件尺寸，滚动/缩放/窗口尺寸变化时应重算浮层位置；采纳作为用户编辑操作进入编辑模型；采纳位置固定为触发瞬间捕获的文档/block/字符锚，不跟随采纳时光标；用户点击浮层外、移动光标或改变选区即视为放弃本次辅助回合，关闭浮层并丢弃已返回或后续返回内容 | docs/01_modules/AG/A-AG-M-D-01_代理功能需求描述.md       |
| REQ-AG-006    | 能力模型独立配置                               | AG           | active   | 模型设置可为辅助续写 Autocomplete、局部修改 InlineAssist、代理编辑 AgentEdit，以及后续 Profile 的代理构建 BuildMode 分别保存 provider/model 偏好，首次默认跟随全局默认模型、可显式指定覆盖；已选模型不可用时须提示并停止调用，不得静默回退；AgentTurn 开始后固定模型，确认、澄清及预览续行不可切换；模型分预设（云端下发、订阅可调用）与 BYOK（用户自带密钥、网关代持）两类来源分层呈现与选择；保存 BuildMode 偏好不代表 Agent 1.0 启用 Build 调用，模型配置的密钥语义仍受 X-FORBID-001 约束（客户端不持密钥）                                                                                                                                                                  | docs/01_modules/AG/A-AG-M-D-01_代理功能需求描述.md       |
| REQ-AG-007    | Agent 运行时、意图与会话基础能力                    | AG           | active   | 提供可恢复、可观察的 Agent 运行时：仅 `agent-edit-1.0` 可启动，未知或缺失 profile 在 Gate、provider、工具和 Proposal 前失败关闭；每次用户发送形成独立 AgentTurn，澄清是等待用户输入的非终态，取消可在任一活动阶段生效且首个终态后迟到事件/工具无副作用。模型维护可更新 IntentResult、选择工具并依据证据调整路线；运行时维护事实、动态 CapabilityGrant、Gate、预算、副作用、取消与终态。AgentSession 是消息、append-only 任务事件、工具/DiffCard、引用与未保存上下文的展示、持久化和恢复隔离边界                                                                                                                                                                       | docs/01_modules/AG/A-AG-M-D-01_代理功能需求描述.md       |
| REQ-AG-008    | 新建文档意图与 CreateDocumentBundle 协同        | AG           | active   | 对明确且有初始内容的新建文档意图，AG 在确认前完成目标路径、资源计划和初始内容生成，形成 CreateDocumentBundle；DecisionRequest 确认后由 WS 重新核验并创建空的目标 DiskState，ED 打开标签，随后 DE/ED 以该磁盘基线生成并预览初始内容 Diff。用户明确要求零字节 TXT 时只冻结单文件、无初始内容的 WorkspaceOperationPlan；DecisionRequest 确认后由 WS 重验并直接创建零字节文件，不生成无变化的初始 Diff。资源创建授权与内容 Diff 裁决相互独立                                                                                                                                                                                                      | docs/01_modules/AG/A-AG-M-D-01_代理功能需求描述.md       |
| REQ-AG-009    | 项目感知、编辑器事实与受限文档分析                      | AG           | active   | Agent 1.0 提供 P0 项目感知和阅读分析：先获取 workspace 结构概览，再由模型按用户任务和当前证据选择最小必要正文读取；运行时记录每次读取的目标、来源、状态时点、结果与失败，并据此派生可复核 ProjectReport。Agent 同时消费当前 workspace、活动文档、打开标签、脏态、保存基准、任务所需选区和关联 Proposal；当前/已打开文档以 LogicalState 为准，未打开文档才按受限 workspace 读取 DiskState                                                                                                                                                                                                                                            | docs/01_modules/AG/A-AG-M-D-01_代理功能需求描述.md       |
| REQ-AG-010    | 受控网络研究与来源化分析                           | AG           | deferred | 将用户明确要求搜索/核验或明确提供 URL 的请求识别为 P1 受控网络研究：只检索和有限读取公开网页，并以标题、域名、URL、获取时间可追溯地输出分析；外部资料始终为 data-only，可支持回答或 ChangeProposal，但不直接获得编辑或写入能力                                                                                                                                                                                                                                                                                                                                                        | docs/01_modules/AG/A-AG-M-D-01_代理功能需求描述.md       |
| REQ-AG-011    | 工作区资源操作计划与前置裁决                         | AG           | active   | 对创建文件夹、创建文档、删除、重命名、移动等明确的工作区资源整理请求，AG 只生成不可变 WorkspaceOperationPlan 与影响说明，并以 DecisionRequest 请求前置授权；确认后由 WS 对目标、来源、冲突、递归清单、脏会话和基线重新核验，再直接修改 DiskState。工作区资源操作不生成内容 DiffCard，也不等待后续“保存文件树”                                                                                                                                                                                                                                                                                                 | docs/01_modules/AG/A-AG-M-D-01_代理功能需求描述.md       |
| REQ-AG-012    | 最小格式转换提案                               | AG           | deferred | 对用户明确提出的 TXT→Markdown 或 Markdown→TXT 请求，AG 先以冻结来源生成转换结果，并将目标资源创建与初始内容绑定为 CreateDocumentBundle；已打开来源以当前 LogicalState 为输入，未打开来源以标明读取时点的 DiskState 为输入。DecisionRequest 必须展示来源、输入/输出格式、目标路径和保真或损失说明，AG 不改写来源                                                                                                                                                                                                                                                                                  | docs/01_modules/AG/A-AG-M-D-01_代理功能需求描述.md       |
| REQ-AG-013    | 删除与恢复的前置裁决                             | AG           | active   | 对用户明确提出的已保存文件或文件夹删除/恢复请求，AG 生成包含冻结影响清单、递归数量、来源快照和恢复信息的 WorkspaceOperationPlan，并通过 DecisionRequest 取得前置授权；确认后 WS 重新核验并直接修改 DiskState，同时写入统一 WorkspaceTimeline。永久清理不属于 Agent 1.0                                                                                                                                                                                                                                                                                                            | docs/01_modules/AG/A-AG-M-D-01_代理功能需求描述.md       |
| REQ-AG-014    | 多个既有文档的目标计划与逐项 Proposal                | AG           | deferred | 对多个既有文档的修改请求，AG 必须先呈现候选或用户显式选择形成的目标计划，待用户确认后冻结目标成员、路径和身份；随后只对 TXT、Markdown、HTML 与 BDocx 编辑副本逐目标生成独立 Proposal。每个目标的读取状态、基准、预览、裁决、保存和失败独立，AG 不把未声明文档或后续搜索结果加入目标集                                                                                                                                                                                                                                                                                                                            | docs/01_modules/AG/A-AG-M-D-01_代理功能需求描述.md       |
| REQ-AG-015    | 受限复合任务编排与真实工作状态                        | AG           | active   | 简单目标只由单个 AgentTurn 完成，不创建 TaskRun 或 TaskPlan；复杂或跨执行片段目标才建立 TaskRun，并将模型当前路线、已完成事实、等待点及关联 AgentTurn/Proposal 投影为可观察 TaskPlan。模型可根据新证据更新未执行路线；运行时不按静态计划调度、不预授权后续工具、不自动重试、不跨确认点或重启续跑。每次实际动作仍按当前 profile、目标、上下文、CapabilityGrant 和确认点重新校验。明确的同文档复合目标可连续生成 Proposal，但后续卡只以最新 LogicalState 为基准，前卡因基准推进失效；用户只见事件支持的真实状态，不见内部推理                                                                                                                                                                    | docs/01_modules/AG/A-AG-M-D-01_代理功能需求描述.md       |
| REQ-AG-016    | 受控外部文件内容消费                             | AG           | deferred | 对用户明确附加外部文件且当前系统存在可用只读解析路径的任务，AG 只消费 REF 提供的当前任务一次性 data-only 内容引用，并在输出中保持来源、已读/未读和不确定性可见。外部文件内容不等于 workspace 文档、编辑目标、写入权限或可浏览文件系统；不支持/不可读文件只能按元数据说明，不能伪称已分析                                                                                                                                                                                                                                                                                                                              | docs/01_modules/AG/A-AG-M-D-01_代理功能需求描述.md       |
| REQ-AG-017    | 显式项目记忆引用消费                             | AG           | active   | AG 只消费用户在当前 workspace 中明确选择、由 REF 提供的项目记忆条目；该条目作为当前 AgentTurn 的 data-only 上下文，保留来源和用户选择事实。Agent 1.0 不创建、提议写入、更新、删除、按相关性检索或自动注入长期记忆；外部文件、会话历史和编辑历史不自动转存为项目记忆                                                                                                                                                                                                                                                                                                                               | docs/01_modules/AG/A-AG-M-D-01_代理功能需求描述.md       |
| REQ-AST-001   | 记忆条目本地存储与生命周期                          | AST          | active   | 记忆条目在本地、workspace 隔离地保存，支持用户手工创建、查看来源、编辑、关闭与删除完整生命周期；重启可恢复。它是用户管理的项目资产，不是会话/编辑事实或外部文件内容的自动副本                                                                                                                                                                                                                                                                                                                                                                                              | docs/01_modules/AST/A-AST-M-D-01_记忆协同功能需求描述.md   |
| REQ-AST-002   | Agent 1.0 记忆写入的用户控制                    | AST          | active   | 在 `agent-edit-1.0` 中，项目记忆只能由用户手工创建、编辑、关闭或删除；Agent 不创建、提议写入、更新或静默积累记忆。未来 Profile 的 Agent 写入能力不由本要求授权                                                                                                                                                                                                                                                                                                                                                                                       | docs/01_modules/AST/A-AST-M-D-01_记忆协同功能需求描述.md   |
| REQ-AST-003   | 记忆注入消费                                 | AST          | active   | `agent-edit-1.0` 只有用户明确选择的记忆条目可经 REF 作为 data-only 引用进入当前回合；不按相关性自动供给或注入。供给跨模块结构经 SYS 契约层，记忆正文不能提升为系统指令或授权                                                                                                                                                                                                                                                                                                                                                                                 | docs/01_modules/AST/A-AST-M-D-01_记忆协同功能需求描述.md   |
| REQ-AST-004   | 离线与不上云                                 | AST          | active   | 记忆数据全部本地存储、离线可读可管理；不因注入、备份或任何功能将记忆内容上行云端                                                                                                                                                                                                                                                                                                                                                                                                                                                  | docs/01_modules/AST/A-AST-M-D-01_记忆协同功能需求描述.md   |
| REQ-CBT-001   | 多文件构建回合                                | CBT          | active   | 支持跨多文件的文档项目构建：承接对话阶段确认的项目蓝图分解为任务计划（计划须确认），逐任务经 AG 回合基建产出（模型调用过门禁；任务内可调研/检索、产出后轻量自查；执行期不可对话），产物可跨新建与修改既有文档（2026-07-04 补前段定义）                                                                                                                                                                                                                                                                                                                                                               | docs/01_modules/CBT/A-CBT-M-D-01_构建功能需求描述.md     |
| REQ-CBT-002   | 构建产物 diff 化落盘                          | CBT          | active   | 构建产物（新建/修改文档）一律生成 ChangeProposal 交 DE/ED 受控预览和裁决；接受不直接写盘，用户显式保存各目标的当前 LogicalState 后由 WS 落盘；CBT 无旁路写盘路径；支持按条/批量审阅                                                                                                                                                                                                                                                                                                                                                                         | docs/01_modules/CBT/A-CBT-M-D-01_构建功能需求描述.md     |
| REQ-CBT-003   | 构建中断与恢复                                | CBT          | active   | 构建过程用户可随时中断；中断为终态、不产生半成品落盘（未裁决产物保持待裁决或丢弃）；构建控制态持久化、重启后可恢复或明确关闭                                                                                                                                                                                                                                                                                                                                                                                                                            | docs/01_modules/CBT/A-CBT-M-D-01_构建功能需求描述.md     |
| REQ-CBT-004   | 构建移交/回落（旧称 Chat↔Build 接管）              | CBT          | active   | 代理回合与构建回合可双向移交（触发=用户显式触发即确认，或 AI 识别项目级构建意图提议经确认——多文件意图不再是判据，2026-07-04 换轴；构建在原对话窗口呈现；构建完成/中断回落对话），移交/中断两侧状态一致（X-INTER-001 转移点）                                                                                                                                                                                                                                                                                                                                                            | docs/01_modules/CBT/A-CBT-M-D-01_构建功能需求描述.md     |
| REQ-DE-001    | diff 生成                                | DE           | active   | 将 AG 的一次结构化内容修改生成为 ProposalBatch，并按可独立人工裁决的连续块组拆成一个或多个 pending_diff/DiffCard（PendingDiff 共享结构，blockId 定位目标内容块），交 WS 持久化；不连续块默认分卡，相邻且共同表达一个不可分修改的块可合并；定位锚失效时生成失败并回报，不产生错位 diff                                                                                                                                                                                                                                                                                                             | docs/01_modules/DE/A-DE-M-D-01_DiffEdit功能需求描述.md |
| REQ-DE-002    | diff 审阅呈现                              | DE           | active   | 经 ED 装饰层或 DiffCard 按 ProposalBatch 中的独立裁决单元呈现文本、已确认样式以及 HTML 编辑文档/BDocx 处理副本的表格语义修改对比和接受/拒绝控件，用户交互由 ED 转达 DE 裁决；字符基础样式为 P0，颜色/突出显示、段落样式、字号与字体族为 P1；原生 DOCX 不作为可修改目标；终态后装饰复位不残留                                                                                                                                                                                                                                                                                                            | docs/01_modules/DE/A-DE-M-D-01_DiffEdit功能需求描述.md |
| REQ-DE-003    | diff 终态裁决                              | DE           | active   | 拥有提案接受/拒绝/失效的转移语义：结果互斥；已有 LogicalState 时显式接受只裁决已预览提案，保存有效 pending 预览也裁决为接受，拒绝只在原始基准仍有效时撤销预览，失效不得覆盖后续 LogicalState；无 LogicalState 时接受才触发 WS 原子写盘，pending 打开后迁移为前者                                                                                                                                                                                                                                                                                                                          | docs/01_modules/DE/A-DE-M-D-01_DiffEdit功能需求描述.md |
| REQ-DE-004    | diff 内容载荷形状                            | DE           | active   | 定义 ProposalBatch 与 payload_ref 所指 diff 内容载荷的结构（批次身份、分组顺序、独立裁决单元、修改前后内容、已确认样式操作、HTML 编辑文档/BDocx 处理副本的 TableOperation 与 ImageOperation、定位信息、来源元数据），为 A-SYS-I-P-01 显式留归 DE 的占位补形；原生 DOCX 编辑请求的载荷目标是新副本 Proposal；载荷形状版本化以支持迁移                                                                                                                                                                                                                                                                 | docs/01_modules/DE/A-DE-M-D-01_DiffEdit功能需求描述.md |
| REQ-DE-005    | 过期与外部变更处理                              | DE           | active   | 待裁决提案在超时、目标内容外部变更、用户手动编辑或后续提案成功预览时转失效；失效不得覆盖已推进的 LogicalState，也不可再接受或拒绝                                                                                                                                                                                                                                                                                                                                                                                                                   | docs/01_modules/DE/A-DE-M-D-01_DiffEdit功能需求描述.md |
| REQ-DE-006    | CreateDocumentBundle 初始内容 Diff 的预览与裁决  | DE           | active   | CreateDocumentBundle 的 WorkspaceOperationPlan 确认且 WS 创建目标 DiskState 后，DE 才把预生成的初始内容作为带 operation_id/origin 的普通 ChangeProposal 投影到已打开目标；转换来源不是 DE 的可写目标。资源创建授权不由内容 Diff 的接受或拒绝替代                                                                                                                                                                                                                                                                                                           | docs/01_modules/DE/A-DE-M-D-01_DiffEdit功能需求描述.md |
| REQ-DE-007    | 工作区资源操作与内容 Diff 隔离                     | DE           | active   | 创建、删除、重命名、移动、恢复等工作区资源操作不生成 DE DiffCard，不使用内容 Diff 的 pending/accepted/rejected 状态；它们由 WorkspaceOperationPlan + DecisionRequest 前置裁决并由 WS 执行。DE 只向计划提供受影响内容 Diff 清单，并在资源操作成功后接收失效事件                                                                                                                                                                                                                                                                                                         | docs/01_modules/DE/A-DE-M-D-01_DiffEdit功能需求描述.md |
| REQ-DE-008    | 多文档目标计划与逐项独立审阅                         | DE           | active   | 多个既有文档修改前，DE 承接 AG 交付的可见目标计划确认；确认后目标成员、路径和身份冻结。每个目标单独拥有 Proposal、内容基准、预览、卡片终态和保存结果，按其自身 LogicalState/DiskState 分流；不得以一个跨文档总卡替代逐项审阅，也不得提供批量接受、批量保存或全有或全无裁决                                                                                                                                                                                                                                                                                                                                 | docs/01_modules/DE/A-DE-M-D-01_DiffEdit功能需求描述.md |
| REQ-DE-009    | 复合任务内连续 Proposal 的基准接力                 | DE           | active   | 对同一复杂任务投影中明确的同文档连续编辑，DE 为每个 ProposalBatch 及其 DiffCard 保留任务/回合历史关联，但后续 ProposalBatch 只可基于上一批成功预览后的当前 LogicalState 生成。同一批内的卡按各自连续块组独立裁决；后续批预览成功即推进基准，使所有基于旧基准的前批卡失效、保留历史且不可再裁决；连续 Proposal 不构成补丁栈或任务级回滚。无 LogicalState 的 background pending Proposal 不得作为后续编辑输入                                                                                                                                                                                                                             | docs/01_modules/DE/A-DE-M-D-01_DiffEdit功能需求描述.md |
| REQ-DOC-001   | BDocx 编辑副本打开转换（DOCX→HTML 编辑模型）         | DOC          | active   | 对“创建 BDocx 编辑副本”Proposal，仅读取原生 DOCX 来源并经本地 Pandoc 转换为 ED 的 HTML 编辑模型，应用样式子集契约；预览成功才建立新副本的 LogicalState。原生 DOCX 不走编辑链，转换失败回统一错误形状且不建立编辑会话                                                                                                                                                                                                                                                                                                                                                  | docs/01_modules/DOC/A-DOC-M-D-01_文档转换功能需求描述.md   |
| REQ-DOC-002   | BDocx 编辑副本保存转换（HTML 编辑模型→DOCX）         | DOC          | active   | 将 BDocx 编辑副本的 HTML 编辑模型转换回其副本 DOCX 并经 WS 原子落盘；转换或落盘失败时原生 DOCX 与副本既有文件均保持不变                                                                                                                                                                                                                                                                                                                                                                                                                | docs/01_modules/DOC/A-DOC-M-D-01_文档转换功能需求描述.md   |
| REQ-DOC-003   | 样式能力契约与用户明示                            | DOC          | active   | 对 BDocx 编辑副本，按 R3 已确认的样式子集建立用户可知的支持、尽力保留与明确不处理边界：字符基础样式为 P0；颜色/突出显示、段落对齐/间距/缩进、字号与字体族为 P1；页面级布局、页眉页脚、复杂样式继承和任意定位不纳入 1.0。已确认能力必须实现，真实验证是对外开放门槛，不是是否支持的待决条件                                                                                                                                                                                                                                                                                                                                 | docs/01_modules/DOC/A-DOC-M-D-01_文档转换功能需求描述.md   |
| REQ-DOC-004   | 预览转换服务（供 PRV）                          | DOC          | active   | 提供原生 docx→PDF 的本地预览转换（LibreOffice 预览链），与 BDocx 编辑链完全分离，供 PRV 以 LO 语料实测为准的只读预览                                                                                                                                                                                                                                                                                                                                                                                                             | docs/01_modules/DOC/A-DOC-M-D-01_文档转换功能需求描述.md   |
| REQ-DOC-005   | 本地转换运行时与离线边界                           | DOC          | active   | 转换全程在本地执行，Pandoc/LibreOffice 与默认回退字体必须随 Binder 发布包内置；运行时只能执行经校验的包内工具，离线可用，文档内容不因转换上行云端；包内依赖缺失、版本不符或失效时 Binder 仍可启动，但 DOC command 必须明确报错并禁用相关入口，绝不探测或降级到设备本地工具                                                                                                                                                                                                                                                                                                                             | docs/01_modules/DOC/A-DOC-M-D-01_文档转换功能需求描述.md   |
| REQ-DOC-006   | 最小文本格式转换与保真/损失告知                       | DOC          | active   | 仅对已确认的 TXT→Markdown、Markdown→TXT 目标副本 Proposal 在本地转换来源内容。TXT→Markdown 只保留文本和换行，不自动推断标题、列表、任务或其他 Markdown 语义；Markdown→TXT 是有损转换，输出可读纯文本但不承诺样式、结构、表格、图片或可逆往返。DOC 只处理内容转换，不改写来源、不创建目标文件或标签                                                                                                                                                                                                                                                                                                   | docs/01_modules/DOC/A-DOC-M-D-01_文档转换功能需求描述.md   |
| REQ-ED-001    | 富文本编辑运行时（Markdown/纯文本/HTML/BDocx 编辑副本） | ED           | active   | 提供基于 TipTap 的富文本编辑运行时，支持 workspace 内 Markdown、纯文本、HTML 与 BDocx 编辑副本按处理路径解析为结构化 LogicalState、以终态富文本呈现并进行内容与基础结构编辑，编辑能力离线可用；HTML/BDocx 的字符基础样式为 P0，颜色/突出显示、段落对齐/间距/缩进、字号与字体族为 P1 受控实现。原生 DOCX 只读，不建立编辑会话；落盘经 WS                                                                                                                                                                                                                                                                             | docs/01_modules/ED/A-ED-M-D-01_编辑器功能需求描述.md      |
| REQ-ED-002    | block 模型与稳定 blockId 定位                 | ED           | active   | 编辑内容以结构化 block 建模并赋稳定 blockId，作为 diff 定位、@mention 引用锚与分页切分的统一定位基准                                                                                                                                                                                                                                                                                                                                                                                                                         | docs/01_modules/ED/A-ED-M-D-01_编辑器功能需求描述.md      |
| REQ-ED-003    | 分页系统与布局                                | ED           | active   | 提供分页模式的布局、光标与遮罩处理及打印友好呈现，使长文档可分页编辑与打印输出                                                                                                                                                                                                                                                                                                                                                                                                                                                   | docs/01_modules/ED/A-ED-M-D-01_编辑器功能需求描述.md      |
| REQ-ED-004    | HTML 与 BDocx 处理路径的图文媒体能力               | ED           | active   | 支持 HTML 编辑文档与 BDocx 处理副本中的图片等媒体插入、呈现与处理，媒体资源经 WS 限定在 workspace 内；该能力为 P1 受控开放，原生 DOCX 不直接编辑                                                                                                                                                                                                                                                                                                                                                                                               | docs/01_modules/ED/A-ED-M-D-01_编辑器功能需求描述.md      |
| REQ-ED-005    | 编辑器装饰层（供 diff 呈现与局部修改）                 | ED           | active   | 提供编辑器装饰层（decoration），按 blockId 承载 diff 的受控预览、审阅与局部修改的可视呈现；已有 LogicalState 的预览成功后候选内容进入该 LogicalState，背景 pending 在打开目标文档时也必须以同一 payload 建立并进入 LogicalState，提案裁决归 DE，ED 不直接写 DiskState                                                                                                                                                                                                                                                                                                      | docs/01_modules/ED/A-ED-M-D-01_编辑器功能需求描述.md      |
| REQ-ED-006    | 字符级定位与选区引用                             | ED           | active   | 编辑器提供会话内字符级内容锚（blockId + 块内纯文本偏移）与光标/选区位置的受控查询，供 agent 精确编辑定位与位置引用生成（选区可拖拽携带位置信息入 AG 输入面）；锚失效可感知                                                                                                                                                                                                                                                                                                                                                                                          | docs/01_modules/ED/A-ED-M-D-01_编辑器功能需求描述.md      |
| REQ-ED-007    | agent 可读样式投影与受控编辑边界                    | ED           | active   | 文档样式（字符 marks / 块属性）可提供机器可读结构投影供 agent 读取；HTML/BDocx 的已确认操作集合为：P0 加粗、斜体、下划线、删除线，P1 颜色/突出显示、段落对齐/间距/缩进、字号与字体族。样式修改仅以受控操作进入编辑模型；投影不构成原始 HTML、CSS 或 OOXML 写入授权；agent 落盘路径按裁定 D-3 分流（回合产出经 DE diff 裁决、显式局部修改视为用户编辑）                                                                                                                                                                                                                                                                           | docs/01_modules/ED/A-ED-M-D-01_编辑器功能需求描述.md      |
| REQ-ED-008    | .html 富文本编辑接入                          | ED           | active   | 独立 .html 文件加入统一编辑模型：富文本编辑 + blockId（.html 源可落盘 data-block-id）+ 序列化往返保真；读写经 WS 文本路径（非二进制，不经 DOC 管线）                                                                                                                                                                                                                                                                                                                                                                                        | docs/01_modules/ED/A-ED-M-D-01_编辑器功能需求描述.md      |
| REQ-ED-009    | HTML 与 BDocx 处理路径的表格语义编辑与单元格底纹         | ED           | active   | HTML 编辑文档与 BDocx 处理副本的表格人工交互与代理修改共用受控表格操作协议；支持基础结构操作和单元格纯色底纹，代理建议必须进入 Diff Edit 裁决；该能力为 P1 受控开放，原生 DOCX 不直接编辑                                                                                                                                                                                                                                                                                                                                                                             | docs/01_modules/ED/A-ED-M-D-01_编辑器功能需求描述.md      |
| REQ-ED-010    | CreateDocumentBundle 的编辑器会话            | ED           | active   | CreateDocumentBundle 的 DecisionRequest 确认且 WS 创建目标 DiskState 后，ED 自动打开真实目标标签；随后承载初始内容 Diff 的受控预览、LogicalState、继续手动或代理编辑与普通未保存提示。确认前不得出现目标标签                                                                                                                                                                                                                                                                                                                                               | docs/01_modules/ED/A-ED-M-D-01_编辑器功能需求描述.md      |
| REQ-ED-011    | Agent 可感知的编辑器会话事实                      | ED           | active   | 为 Agent 1.0 的 P0 阅读、分析和提案提供受限编辑器会话事实：当前 workspace 内的活动文档、已打开标签、每个标签的格式/脏态/保存基准、任务所需的选区或焦点、当前 LogicalState 版本以及关联 Proposal 状态。ED 只提供真实会话投影，不把显示态或磁盘旧内容伪装成当前编辑内容                                                                                                                                                                                                                                                                                                                            | docs/01_modules/ED/A-ED-M-D-01_编辑器功能需求描述.md      |
| REQ-ED-012    | 原生 DOCX 只读与 BDocx 编辑副本呈现               | ED           | active   | 原生 DOCX 只经 PRV 进入只读预览、读取和分析，不建立 ED 编辑会话。用户或 Agent 请求编辑时，新 BDocx 编辑副本按 CreateDocumentBundle 创建；资源确认并创建目标磁盘副本后，ED 以 HTML 编辑模型打开并投影初始内容 Diff，以模拟 Word 分页呈现，保存目标仍为新的 DOCX                                                                                                                                                                                                                                                                                                                     | docs/01_modules/ED/A-ED-M-D-01_编辑器功能需求描述.md      |
| REQ-ED-013    | WorkspaceOperation 执行前置与标签协调           | ED           | active   | WorkspaceOperationPlan 进入 DecisionRequest 时，ED 提供受影响打开标签、脏态、LogicalState 与有效内容 Diff 清单；删除、重命名或移动涉及脏标签时阻断执行。确认并成功修改 DiskState 后，ED 才按真实结果关闭标签或更新路径，并使相关内容 Diff 失效                                                                                                                                                                                                                                                                                                                          | docs/01_modules/ED/A-ED-M-D-01_编辑器功能需求描述.md      |
| REQ-ED-014    | 最小格式转换目标副本的编辑器会话                       | ED           | active   | TXT→Markdown、Markdown→TXT 的转换目标副本经 CreateDocumentBundle 确认并创建磁盘目标后，ED 自动打开标签并投影转换结果初始内容 Diff；来源文档不被改写或关闭。目标标签清晰呈现输入/输出格式、目标路径和 Markdown→TXT 的有损提示                                                                                                                                                                                                                                                                                                                                         | docs/01_modules/ED/A-ED-M-D-01_编辑器功能需求描述.md      |
| REQ-ED-015    | 删除/恢复前置裁决与受影响标签处理                      | ED           | active   | 对已保存文件/文件夹删除或恢复，ED 在 WorkspaceOperationPlan 决策前提供受影响标签事实；任一受影响会话有未保存内容即阻断删除/重命名/移动。DecisionRequest 确认并由 WS 成功修改 DiskState 后，ED 按真实磁盘结果关闭、重建或更新标签                                                                                                                                                                                                                                                                                                                                          | docs/01_modules/ED/A-ED-M-D-01_编辑器功能需求描述.md      |
| REQ-ED-016    | 多个既有文档的独立编辑器会话                         | ED           | active   | 对已确认并冻结的多既有文档目标清单，ED 为每个已打开目标独立承载 LogicalState、受控预览、绿色差异、脏态、卡片失效和显式保存结果；ED 不因一个目标冲突、外部变化、结构变化或删除而改写、关闭或阻断其他目标会话。背景目标打开后仍按其自身三分流迁移为 LogicalState 预览                                                                                                                                                                                                                                                                                                                                        | docs/01_modules/ED/A-ED-M-D-01_编辑器功能需求描述.md      |
| REQ-ENT-001   | 订阅门禁客户端执行                              | ENT          | active   | 为一切模型调用提供唯一客户端门禁通道：每次调用前提交令牌过门禁（X-SEQ-001）；不缓存通过结论复用（X-FORBID-002）；GateDenied 呈现不可绕过的明确拒绝与订阅引导                                                                                                                                                                                                                                                                                                                                                                                            | docs/01_modules/ENT/A-ENT-M-D-01_订阅门控功能需求描述.md   |
| REQ-ENT-002   | AI 网关客户端                               | ENT          | active   | GatePassed 后经 AiGateway 发起模型调用（套餐计量/BYOK 透传由服务端区分），流式回传承接 BR-SYS-STATE-002 终态契约，失败回统一错误形状；客户端无 provider SDK、无直连 provider 路径                                                                                                                                                                                                                                                                                                                                                               | docs/01_modules/ENT/A-ENT-M-D-01_订阅门控功能需求描述.md   |
| REQ-ENT-003   | BYOK 客户端边界                             | ENT          | active   | 用户自带密钥经安全通道一次性上送网关代持：客户端不持久化、不写日志、不回显明文、上送后本地即清；BYOK 透传不计套餐额度但仍需有效订阅（门禁不豁免）                                                                                                                                                                                                                                                                                                                                                                                                               | docs/01_modules/ENT/A-ENT-M-D-01_订阅门控功能需求描述.md   |
| REQ-ENT-004   | 用量与订阅状态呈现                              | ENT          | active   | 展示套餐用量与订阅状态，数据源唯一为服务端（客户端不自行计量作裁决依据）；购买/升级跳转官网门户，桌面端不内置支付                                                                                                                                                                                                                                                                                                                                                                                                                                 | docs/01_modules/ENT/A-ENT-M-D-01_订阅门控功能需求描述.md   |
| REQ-KB-001    | 知识条目本地存储与组织                            | KB           | active   | 知识条目本地存储，支持增删改与分类/标签组织；用户可见可管理；重启可恢复                                                                                                                                                                                                                                                                                                                                                                                                                                                      | docs/01_modules/KB/A-KB-M-D-01_知识库功能需求描述.md      |
| REQ-KB-002    | 内容入库                                   | KB           | active   | 支持从 workspace 文档与外部文本入库建条目；入库为用户主动操作（AI 不静默建库）；入库内容可溯源到来源                                                                                                                                                                                                                                                                                                                                                                                                                                 | docs/01_modules/KB/A-KB-M-D-01_知识库功能需求描述.md      |
| REQ-KB-003    | 检索召回供给                                 | KB           | active   | 按查询/相关性向后续 Agent Profile 召回知识条目；`agent-edit-1.0` 不自动召回，仅消费用户显式选择且经 REF 规范化的知识引用（含来源标识，AG 负责预算裁剪）；已删除条目不召回，无悬空供给                                                                                                                                                                                                                                                                                                                                                                           | docs/01_modules/KB/A-KB-M-D-01_知识库功能需求描述.md      |
| REQ-KB-004    | 离线与不上云                                 | KB           | active   | 知识库数据全部本地存储、离线可用（存储/组织/检索均不依赖在线）；不因检索或注入将库内容上行云端                                                                                                                                                                                                                                                                                                                                                                                                                                          | docs/01_modules/KB/A-KB-M-D-01_知识库功能需求描述.md      |
| REQ-PRV-001   | docx 语料实测只读预览                          | PRV          | active   | 消费 DOC 预览链（LibreOffice→PDF）对原生 docx 做以 LO 语料实测为准的只读预览；用户或 Agent 的编辑请求不在预览链内执行，而是另行创建 BDocx 编辑副本 Proposal；工具无效、启动、字体覆盖或转换失败均呈现明确失败态                                                                                                                                                                                                                                                                                                                                                        | docs/01_modules/PRV/A-PRV-M-D-01_预览输出功能需求描述.md   |
| REQ-PRV-002   | Excel / PPT 预览（无编辑入口）                  | PRV          | active   | workspace 内 Excel/PPT 文件可只读预览并打印/导出；整个产品不存在 Excel/PPT 编辑入口（v1 冻结验收项）                                                                                                                                                                                                                                                                                                                                                                                                                      | docs/01_modules/PRV/A-PRV-M-D-01_预览输出功能需求描述.md   |
| REQ-PRV-003   | 音视频预览                                  | PRV          | active   | workspace 内音视频资源可播放预览（v1 冻结纳入）；解码失败呈现明确失败态；格式支持范围 G2 界定                                                                                                                                                                                                                                                                                                                                                                                                                                   | docs/01_modules/PRV/A-PRV-M-D-01_预览输出功能需求描述.md   |
| REQ-PRV-004   | 打印与导出                                  | PRV          | active   | 预览内容可打印输出与导出（导出格式清单 G2 定）；导出文件落 workspace 内（越界拒绝）；编辑文档的打印友好分页呈现由 ED 提供、输出管线归 PRV                                                                                                                                                                                                                                                                                                                                                                                                          | docs/01_modules/PRV/A-PRV-M-D-01_预览输出功能需求描述.md   |
| REQ-PRV-005   | 离线可用                                   | PRV          | active   | 预览/打印/导出全链离线可用，不依赖在线鉴权（REQ-CORE-009：预览打印属本地功能）                                                                                                                                                                                                                                                                                                                                                                                                                                            | docs/01_modules/PRV/A-PRV-M-D-01_预览输出功能需求描述.md   |
| REQ-REF-001   | @mention 触发与目标解析                       | REF          | active   | 在对话/构建输入中经 @ 触发列举 workspace 内可引用候选（文档/内容块/资源）并解析选定目标为引用锚；目标不可达时明确失败，不产生悬空引用                                                                                                                                                                                                                                                                                                                                                                                                               | docs/01_modules/REF/A-REF-M-D-01_引用功能需求描述.md     |
| REQ-REF-002   | 引用内容读取与注入载荷                            | REF          | active   | 经 WS 限界读取引用目标内容，组装为含来源标识、读取时点/新鲜度的 data-only 注入载荷供 AG/CBT 消费；读取不越出 workspace，不因引用而把对象升级为编辑目标或写权限                                                                                                                                                                                                                                                                                                                                                                                          | docs/01_modules/REF/A-REF-M-D-01_引用功能需求描述.md     |
| REQ-REF-003   | 引用锚定与失效感知                              | REF          | active   | 引用锚支持文档级（workspace 相对路径）与块级（blockId）；锚失效（目标删除/移动/块失效）必须可感知并发明确失效信号，不静默注入过期或错位内容。WorkspaceOperationPlan 等待 DecisionRequest 时，当前引用路径保持不变；确认并成功修改 DiskState 后，后续引用按真实新路径重绑定或明确失效，已发送消息继续保留其发送时冻结快照                                                                                                                                                                                                                                                                                           | docs/01_modules/REF/A-REF-M-D-01_引用功能需求描述.md     |
| REQ-REF-004   | 触发与呈现经 ED 表面                           | REF          | active   | @ 触发输入面、候选面板与引用标记渲染依托 ED 扩展表面，REF 不绕过 ED 接口直接操纵编辑器内部状态                                                                                                                                                                                                                                                                                                                                                                                                                                    | docs/01_modules/REF/A-REF-M-D-01_引用功能需求描述.md     |
| REQ-REF-005   | 多来源引用归一化                               | REF          | active   | 编辑器选区、文件树、编辑器标签、会话标签、外部 URL、外部长文本、用户明确选择的项目记忆与外部文件必须归一为 ReferencePayload；1.0 P0 覆盖当前选区、workspace 文档/标签与会话标签，外部文件 P0 只形成元数据引用，P1 的受控内容读取由 REQ-REF-009 限定。REF 不接管 AST/KB/TPL 的检索或管理。引用只提供 data-only 上下文，默认不成为编辑目标                                                                                                                                                                                                                                                                           | docs/01_modules/REF/A-REF-M-D-01_引用功能需求描述.md     |
| REQ-REF-006   | 输入流引用令牌一致性                             | REF          | active   | 引用在发送前必须同时具备输入流内 ReferenceToken 与草稿引用 chip/tray；发送时冻结引用身份、锚、范围与版本快照；超过 5 条时草稿区折叠；发送后草稿 chip 生命周期终止，但已发送消息中的引用令牌和来源身份保持可读、可追溯表现                                                                                                                                                                                                                                                                                                                                                            | docs/01_modules/REF/A-REF-M-D-01_引用功能需求描述.md     |
| REQ-REF-007   | 外部研究来源引用                               | REF          | active   | 对用户明确提供的 URL，或 AG 依 REQ-AG-010 实际使用的公开网页来源，REF 提供可见、可回看的 data-only 来源引用；来源至少使标题、域名、URL 和获取时间可被用户和 AG 追溯。REF 不因形成来源引用而发起网络请求、读取外部网页、上传本地上下文或授予编辑权限                                                                                                                                                                                                                                                                                                                                         | docs/01_modules/REF/A-REF-M-D-01_引用功能需求描述.md     |
| REQ-REF-008   | 多文档目标清单身份与删除后的引用失效                     | REF          | active   | 为多个既有文档写入的目标计划，REF 提供用户显式选择或已确认候选清单中每个目标的可追溯身份/路径快照，供 AG/DE 在确认后冻结成员；该清单不是写权限本身。WorkspaceOperation 对文件或文件夹执行删除、移动、重命名或恢复后，REF 依据真实执行回执对后续引用重绑定或标记失效，已发送消息仍只保留发送时历史快照                                                                                                                                                                                                                                                                                                                      | docs/01_modules/REF/A-REF-M-D-01_引用功能需求描述.md     |
| REQ-REF-009   | 外部文件受控内容引用                             | REF          | active   | 对用户明确附加的外部文件，REF 先形成可见元数据引用；只有当前系统存在可用只读解析路径时，才可为当前用户任务形成一次性、可追溯的 data-only 内容引用。不可读、不支持或解析失败时，REF 必须保留“未读内容”的元数据状态，不伪造内容快照。REF 不扫描目录、不自动重复读取、不导入或编辑外部文件，也不默认将其内容保留至会话历史、项目记忆、重启或其他会话                                                                                                                                                                                                                                                                                                     | docs/01_modules/REF/A-REF-M-D-01_引用功能需求描述.md     |
| REQ-SYS-001   | 统一 IPC / Tauri command 协议              | SYS          | active   | 定义前端↔Rust 后端的统一调用协议，覆盖同步调用、流式/事件回传与统一错误形状，作为全体跨进程调用的单一契约                                                                                                                                                                                                                                                                                                                                                                                                                                  | docs/01_modules/SYS/A-SYS-M-D-01_系统底座功能需求描述.md   |
| REQ-SYS-002   | 跨模块共享数据结构与持久化 schema 基线                | SYS          | active   | 定义跨模块传递的共享实体/类型与 workspace.db 持久化 schema 基线及迁移契约框架，支撑内容提案、LogicalState、WorkspaceOperationPlan、WorkspaceOperationEntry、CreateDocumentBundle、统一 WorkspaceTimeline、AgentTurn、编辑器会话事实和引用快照等跨模块协作                                                                                                                                                                                                                                                                                              | docs/01_modules/SYS/A-SYS-M-D-01_系统底座功能需求描述.md   |
| REQ-SYS-003   | 状态机总图与注册                               | SYS          | active   | 提供 APC §6 核心链状态机的总图框架与注册入口，汇总跨模块转移契约；须覆盖 AgentTurn、内容 Diff 预览/裁决/失效、WorkspaceOperation 前置裁决/重新核验/磁盘执行/补偿、CreateDocumentBundle 及显式保存链路，具体状态机由各 owner 模块在其 M-S 定义                                                                                                                                                                                                                                                                                                                           | docs/01_modules/SYS/A-SYS-M-D-01_系统底座功能需求描述.md   |
| REQ-SYS-004   | 模块注册与依赖边界                              | SYS          | active   | 落地 15 客户端模块的编码注册（MODULE 注册块）与模块依赖图，作为治理负空间扫描与跨模块约束的基线（承接产品概述 PO-03）                                                                                                                                                                                                                                                                                                                                                                                                                       | docs/01_modules/SYS/A-SYS-M-D-01_系统底座功能需求描述.md   |
| REQ-SYS-005   | 云接口契约边界（认证 / 订阅门禁 / 网关）                | SYS          | active   | 定义客户端侧云接口契约边界——认证会话、调用模型前订阅门禁硬门、AI 网关代理；保证密钥仅服务端持有、客户端不持密钥、门禁不可客户端绕过、BYOK 经网关透传不计额度仍需有效订阅                                                                                                                                                                                                                                                                                                                                                                                                 | docs/01_modules/SYS/A-SYS-M-D-01_系统底座功能需求描述.md   |
| REQ-SYS-006   | 前端交互架构与一致性                             | SYS          | active   | 定义客户端前端交互的系统级骨架——应用外壳阶段（欢迎页 / 主工作台）、跨模块界面分区与协作点、前端状态对后端状态机/控制态的投影一致性；前端交互为视图层 SSOT，统一约束前端按图施工，不得各模块各拼一套                                                                                                                                                                                                                                                                                                                                                                                   | docs/01_modules/SYS/A-SYS-M-D-01_系统底座功能需求描述.md   |
| REQ-SYS-007   | 应用关闭生命周期                               | SYS          | active   | 定义应用退出（窗口关闭/Cmd+Q/Dock 退出/系统关机）前 diff 与未保存内容必须终态化的护栏，以及异常退出（崩溃/强杀）后启动清算语义，避免裁决权或未保存修改在退出前后处于不确定状态                                                                                                                                                                                                                                                                                                                                                                                         | docs/01_modules/SYS/A-SYS-M-D-01_系统底座功能需求描述.md   |
| REQ-SYS-008   | Agent 文档生命周期共享契约                       | SYS          | active   | 为 Agent 文档协作定义共享边界：提案的基准身份、LogicalState 与 DiskState 的分离、未保存文档元数据、AgentTurn 关联、引用快照、提案失效与显式保存的跨模块事件。实现 owner 仍分别是 AG/DE/ED/WS/REF，SYS 只定义可互操作的契约和总图登记入口                                                                                                                                                                                                                                                                                                                                    | docs/01_modules/SYS/A-SYS-M-D-01_系统底座功能需求描述.md   |
| REQ-SYS-009   | Agent 只读上下文事实与隔离边界                     | SYS          | active   | 为 Agent 1.0 P0 的项目分析、编辑器感知和文档阅读声明跨模块事实边界：工作区概览、当前/打开编辑器 LogicalState、标签脏态/保存基准、任务所需选区、关联 Proposal、显式引用和未打开文档 DiskState 读取均须携带来源与适用范围，并受 workspace、AgentSession 和 AgentTurn 隔离                                                                                                                                                                                                                                                                                                             | docs/01_modules/SYS/A-SYS-M-D-01_系统底座功能需求描述.md   |
| REQ-SYS-010   | Agent 运行时共享身份、事件与恢复隔离                  | SYS          | active   | 为 Agent 1.0 声明模型主导、运行时受控的跨模块边界：模型维护可更新 IntentResult、选择工具并依据证据调整路线；运行时维护唯一 Runtime Profile、AgentTurn/AgentSession 身份、事实快照、动态 CapabilityGrant、Gate、预算、副作用、取消与终态。AgentTurn 的 append-only 事件是状态事实源，界面、TaskRun/TaskPlan、ProjectReport 与恢复状态均由事件和工具证据投影。仅 `agent-edit-1.0` 可启动；未知或缺失 profile 在 Gate、provider、工具与 Proposal 前失败关闭；AgentSession 是展示、持久化和恢复隔离边界                                                                                                                                     | docs/01_modules/SYS/A-SYS-M-D-01_系统底座功能需求描述.md   |
| REQ-SYS-011   | 受限任务链、真实进度与连续 Proposal 共享契约            | SYS          | active   | 为 Agent 1.0 的复合任务声明共享投影边界：复杂或跨执行片段目标才创建 TaskRun，其 TaskPlan 表达模型当前路线，并与 AgentTurn、工具证据、等待点和 ProposalBatch 关联；事件事实投影真实执行、等待、中断与终态。TaskRun/TaskPlan 不是静态调度器、未来工具授权或后台队列，模型可根据新证据更新未执行路线但不能跨确认点自动续跑。连续同文档 Proposal 以当前 LogicalState 接力，前卡因基准推进失效；background pending Proposal 未建立 LogicalState 前不得作为后续编辑输入                                                                                                                                                                                     | docs/01_modules/SYS/A-SYS-M-D-01_系统底座功能需求描述.md   |
| REQ-SYS-012   | 外部内容与项目记忆的隔离和保留契约                      | SYS          | active   | 为 Agent 1.0 的 P1 外部文件内容引用与用户管理项目记忆声明共享隔离边界：外部内容只在用户显式附加、存在可用只读解析路径后的当前 AgentTurn/TaskRun 当前任务中以 data-only 存在；任务结束、重启、跨 AgentSession 或跨 workspace 后不自动恢复、复读或转存。项目记忆是本地 workspace 隔离的用户资产，只有用户明确选择才由 REF 进入当前回合；Agent 1.0 不自动写入、提议、检索或注入                                                                                                                                                                                                                                                    | docs/01_modules/SYS/A-SYS-M-D-01_系统底座功能需求描述.md   |
| REQ-TPL-001   | 模板本地存储与管理                              | TPL          | active   | 文档模板与提示模板本地存储，支持增删改与组织；用户可见可管理；重启可恢复                                                                                                                                                                                                                                                                                                                                                                                                                                                      | docs/01_modules/TPL/A-TPL-M-D-01_模板工作流功能需求描述.md  |
| REQ-TPL-002   | 模板应用                                   | TPL          | active   | 支持以模板创建新文档（文档骨架入 workspace，经正常落盘路径）与以模板组装 AI 回合输入（交 AG）；模板实例化失败不产生半成品                                                                                                                                                                                                                                                                                                                                                                                                                     | docs/01_modules/TPL/A-TPL-M-D-01_模板工作流功能需求描述.md  |
| REQ-TPL-003   | 模板化工作流                                 | TPL          | active   | 支持后续 Profile 的多步写作流程定义（步骤序列+每步模板/指令）与可复用执行；`agent-edit-1.0` 不执行模板工作流。启用后执行逐步委托 AG（单文档回合）或 CBT（多文件构建），均过门禁、经受控预览并由显式保存落盘；步骤失败暂停可恢复，不静默跳步                                                                                                                                                                                                                                                                                                                                                   | docs/01_modules/TPL/A-TPL-M-D-01_模板工作流功能需求描述.md  |
| REQ-TPL-004   | 离线与不上云                                 | TPL          | active   | 模板与工作流定义全部本地存储、离线可管理；执行中 AI 步骤需在线（AG 门禁语义），非 AI 步骤离线可用；模板内容不上行云端（提示模板作为回合输入经网关属正常调用路径，不属上行存储）                                                                                                                                                                                                                                                                                                                                                                                             | docs/01_modules/TPL/A-TPL-M-D-01_模板工作流功能需求描述.md  |
| REQ-WS-001    | workspace 生命周期与唯一边界                    | WS           | active   | 管理 workspace 打开/切换/关闭，并以 workspace root 作为文件操作/搜索/diff/上下文读取的唯一边界，所有文件操作不得越出该边界                                                                                                                                                                                                                                                                                                                                                                                                           | docs/01_modules/WS/A-WS-M-D-01_工作台功能需求描述.md      |
| REQ-WS-002    | 文件 I/O、save 保存与跨文档资源操作                 | WS           | active   | 提供 workspace 内已保存文档/资源的读写、列举与跨文档操作，以及把当前 LogicalState 写入磁盘的显式 save；为 Agent 1.0 项目分析先提供结构概览（路径、类型、数量、可访问范围），正文仅按任务最小化读取。未打开文档读取 DiskState 并返回读取时点/新鲜度，已打开文档的当前事实不由 WS 磁盘旧内容替代；AI 文件操作限定在 workspace 内                                                                                                                                                                                                                                                                                       | docs/01_modules/WS/A-WS-M-D-01_工作台功能需求描述.md      |
| REQ-WS-003    | 提案控制态持久化与分流落盘                          | WS           | active   | 持久化由 DE 生成的 ChangeProposal/diff 控制态；已有 LogicalState 的受控预览、显式接受、拒绝或失效不直接写文件，保存才落盘并记时间轴节点；其中保存仍有效的 pending 预览必须先将卡片裁决为 accepted/awaiting_save；无 LogicalState 的背景卡只临时挂载，用户接受后由 WS 原子写入目标文件；pending 打开目标文档时迁移为已有 LogicalState 分支                                                                                                                                                                                                                                                              | docs/01_modules/WS/A-WS-M-D-01_工作台功能需求描述.md      |
| REQ-WS-004    | 文档级时间轴与历史回溯/还原                         | WS           | active   | 提供文档级版本时间轴（A-SYS-I-P-01 TimelineEntry）与历史回溯、还原到先前版本，落盘与还原各记时间轴节点                                                                                                                                                                                                                                                                                                                                                                                                                          | docs/01_modules/WS/A-WS-M-D-01_工作台功能需求描述.md      |
| REQ-WS-005    | 控制态缓存与重启可恢复                            | WS           | active   | 缓存 workspace 控制态并持久化于 workspace.db，遵守 SYS schema 版本化迁移契约，使 pending_diffs、时间轴等控制态在应用重启后可恢复                                                                                                                                                                                                                                                                                                                                                                                                 | docs/01_modules/WS/A-WS-M-D-01_工作台功能需求描述.md      |
| REQ-WS-006    | CreateDocumentBundle 的资源创建与内容投影边界      | WS           | active   | 新建文档由 CreateDocumentBundle 同时冻结 WorkspaceOperationPlan 与预先生成的初始内容；DecisionRequest 确认后 WS 重新核验并创建空的目标 DiskState，ED 打开标签，DE/ED 再以该磁盘基线投影初始内容 Diff。资源创建与内容 Diff 是顺序衔接但生命周期独立的两段                                                                                                                                                                                                                                                                                                              | docs/01_modules/WS/A-WS-M-D-01_工作台功能需求描述.md      |
| REQ-WS-007    | 原生 DOCX 来源保护与 BDocx 编辑副本保存             | WS           | active   | 原生 DOCX 仅作为预览、读取、分析和创建 BDocx 编辑副本的只读来源；创建副本 Proposal 在预览成功后仅产生未保存 LogicalState，用户显式保存才在 workspace 中生成新的 DOCX。任何编辑、接受、拒绝、失效或保存失败都不得改写原生 DOCX                                                                                                                                                                                                                                                                                                                                               | docs/01_modules/WS/A-WS-M-D-01_工作台功能需求描述.md      |
| REQ-WS-008    | WorkspaceOperation 前置裁决与直接磁盘执行         | WS           | active   | 对创建文件夹、创建文档、删除、重命名、移动等工作区资源操作，WS 接收不可变 WorkspaceOperationPlan；DecisionRequest 确认前不执行，确认后重新核验路径、冲突、递归清单、打开会话、内容 Diff 与磁盘基线，再直接修改 DiskState。资源操作不进入内容 DiffCard 生命周期，也不等待第二次“保存文件树”                                                                                                                                                                                                                                                                                                          | docs/01_modules/WS/A-WS-M-D-01_工作台功能需求描述.md      |
| REQ-WS-009    | 最小格式转换目标副本的 CreateDocumentBundle       | WS           | active   | 对 TXT→Markdown、Markdown→TXT 的转换目标副本，来源文件或来源 LogicalState 不被 WS 改写；目标资源创建沿 CreateDocumentBundle 执行：确认后先创建目标 DiskState，再将转换结果作为初始内容 Diff 投影。来源不是资源操作或内容 Diff 的写入目标                                                                                                                                                                                                                                                                                                                          | docs/01_modules/WS/A-WS-M-D-01_工作台功能需求描述.md      |
| REQ-WS-010    | 删除与恢复的前置裁决及 WorkspaceTimeline          | WS           | active   | 对已保存文件/文件夹的删除与恢复，WS 使用 WorkspaceOperationPlan + DecisionRequest 前置裁决；确认后重新核验冻结影响清单、来源基线、冲突与脏会话，直接修改 DiskState，并写入含类型化资源快照和恢复元数据的 WorkspaceOperationEntry。永久清理不属于 Agent 1.0                                                                                                                                                                                                                                                                                                                | docs/01_modules/WS/A-WS-M-D-01_工作台功能需求描述.md      |
| REQ-WS-011    | 多个既有文档的独立保存与失败隔离                       | WS           | active   | 在 AG 已确认并冻结的多既有文档目标清单内，WS 对每个目标独立持久化 Proposal 控制态、执行其自身 LogicalState/DiskState 分流、接受后的背景原子写盘或显式保存、失败保留和时间轴结果；不得把多个目标合并为批量保存、全有或全无事务或共同回滚                                                                                                                                                                                                                                                                                                                                                  | docs/01_modules/WS/A-WS-M-D-01_工作台功能需求描述.md      |
| REQ-CLOUD-001 | 认证与会话服务                                | CLOUD        | active   | AUTH 提供邮箱级身份的注册/登录/登出、SessionToken 签发与续期、设备会话清单与强制下线；身份库 PII 最小化（邮箱 + 凭据哈希 + 设备会话表）；实现 A-SYS-I-P-03 CLOUD-AUTH 契约的服务端                                                                                                                                                                                                                                                                                                                                                                     | docs/02_cloud/A-CLOUD-C-D-01_云端产品概述.md           |
| REQ-CLOUD-002 | 订阅门禁裁决                                 | CLOUD        | active   | 门禁裁决唯一在服务端：每次模型调用按 CLOUD-GATE 契约裁决有效订阅，deny 返回不可绕过的拒绝原因；高频轻量调用（autocomplete）的延迟优化采用服务端签发的短时裁决凭证方案——凭证为服务端签发的短时授权而非客户端缓存，设计须经 X-FORBID-002 一致性复核并回写 A-SYS-I-P-03                                                                                                                                                                                                                                                                                                                           | docs/02_cloud/A-CLOUD-C-D-01_云端产品概述.md           |
| REQ-CLOUD-003 | AI 网关转发与计量                             | CLOUD        | active   | GW 持有 provider 密钥（客户端永不持有，X-FORBID-001 服务端对偶），统一代理模型调用并流式转发；1.0 EP 默认经 AiGateway 接入 DeepSeek 官方 API，后续自部署 Binder 预设模型作为演进项；调用正文仅瞬时过境不持久化（BR-SYS-DATA-004 落实：日志/监控/错误上报均不留正文），仅产生脱敏计量元数据（用量/时间/模型档）上报 LIC                                                                                                                                                                                                                                                                                  | docs/02_cloud/A-CLOUD-C-D-01_云端产品概述.md           |
| REQ-CLOUD-004 | BYOK 密钥代持                              | CLOUD        | active   | GW 提供 BYOK 上送 API：用户密钥加密保管、与身份数据分离存储、可经门户查看（脱敏）/轮换/删除；BYOK 调用透传不计套餐额度但仍经门禁；删除即生效                                                                                                                                                                                                                                                                                                                                                                                                          | docs/02_cloud/A-CLOUD-C-D-01_云端产品概述.md           |
| REQ-CLOUD-005 | 订阅、计费与用量                               | CLOUD        | active   | LIC 管理订阅档位、计费周期、用量额度与账单：消费 GW 计量元数据累计用量、执行额度判定供门禁裁决；向客户端与门户提供用量/订阅状态查询（客户端展示的唯一数据源）                                                                                                                                                                                                                                                                                                                                                                                                       | docs/02_cloud/A-CLOUD-C-D-01_云端产品概述.md           |
| REQ-CLOUD-006 | 个人账户门户                                 | CLOUD        | active   | PORTAL 提供 Web 端账户中心（路线图 §8.5 清单全量）：订阅管理与购买、账单/发票、用量明细、设备与会话管理、BYOK 密钥管理、资料与安全（改密/改邮箱）、账号注销；是支付与账户管理的唯一界面（桌面端仅展示+跳转）                                                                                                                                                                                                                                                                                                                                                                       | docs/02_cloud/A-CLOUD-C-D-01_云端产品概述.md           |
| REQ-CLOUD-007 | 数据边界与合规                                | CLOUD        | active   | 服务端数据边界：不存储用户创作内容、不留存调用正文（最高承诺）、PII 最小化、密钥与身份分离；支持用户数据导出与删除（注销即按时限清理身份/商业数据）；数据驻留区域与隐私政策合规为发布前置                                                                                                                                                                                                                                                                                                                                                                                           | docs/02_cloud/A-CLOUD-C-D-01_云端产品概述.md           |
| REQ-CLOUD-008 | 契约兼容与版本联动                              | CLOUD        | active   | 云端接口与 A-SYS-I-P-03 契约版本联动：变更双侧复核留痕；提供最低客户端版本声明机制，破坏性变更须客户端版本协同；服务端部署节奏与客户端发布通道（alpha→beta→RC→GA）对齐                                                                                                                                                                                                                                                                                                                                                                                        | docs/02_cloud/A-CLOUD-C-D-01_云端产品概述.md           |
| REQ-GW-001    | 供应商适配层                                 | GW           | active   | GW 统一适配初始三个 AI 供应商（OpenAI / Anthropic / DeepSeek），对外暴露统一调用接口（A-SYS-I-P-03 CLOUD-INVOKE）；1.0 EP 默认托管模型优先经 DeepSeek 适配器接入 DeepSeek 官方 API；每个供应商对应独立适配器，负责 provider 特定认证、请求构造、响应解析；新增供应商通过增加适配器实现，不影响统一接口                                                                                                                                                                                                                                                                                    | docs/02_cloud/A-GW-M-D-01_AI网关功能需求描述.md          |
| REQ-GW-002    | 统一流式转发与协议规范化                           | GW           | active   | GW 把三家 provider 各自的 SSE 格式规范化为统一客户端 StreamEvent（Chunk/ToolRequest/Done/Error），屏蔽 provider 差异；背压处理、连接超时、取消传播均在此层实现；调用正文（含 prompt、工具结果、模型输出）仅瞬时过境，不缓冲至持久化存储（BR-SYS-DATA-004）                                                                                                                                                                                                                                                                                                                | docs/02_cloud/A-GW-M-D-01_AI网关功能需求描述.md          |
| REQ-GW-003    | 用量计量元数据                                | GW           | active   | 每次调用完成后记录脱敏计量元数据（user_id/时间戳/模型档/token 数/BYOK 标记），供 LIC 额度判定；计量数据不含调用正文（BR-SYS-DATA-004）；计量须不可篡改（client 不可伪造）                                                                                                                                                                                                                                                                                                                                                                             | docs/02_cloud/A-GW-M-D-01_AI网关功能需求描述.md          |
| REQ-GW-004    | BYOK 密钥代持                              | GW           | active   | GW 提供 BYOK 密钥上送 API（POST /byok/keys）：接收客户端一次性上送的 provider 密钥（byok.rs 零残留语义），加密保管与身份数据分离存储；BYOK 调用时 GW 取回并使用密钥，不回传客户端；密钥可经门户查看（脱敏）/轮换/删除，删除即生效；BYOK 调用仍经门禁（不豁免）                                                                                                                                                                                                                                                                                                                            | docs/02_cloud/A-GW-M-D-01_AI网关功能需求描述.md          |
| REQ-GW-005    | CLOUD-GATE 订阅门禁裁决                      | GW           | active   | GW 实现 A-SYS-I-P-03 的 CLOUD-GATE 服务端：校验 session token 有效性（联 AUTH）+ 订阅额度（联 LIC），返回 passed/denied；门禁裁决唯一在服务端（X-FORBID-002 对偶），无客户端绕过路径；断网等同拒绝（客户端收不到 passed 即拒绝）；高频轻量调用（autocomplete）的短时裁决凭证方案（CLOUD-04）须经 X-FORBID-002 一致性复核后回写 A-SYS-I-P-03                                                                                                                                                                                                                                                | docs/02_cloud/A-GW-M-D-01_AI网关功能需求描述.md          |
| REQ-GW-006    | 内容不留存（最高承诺落实）                          | GW           | active   | 所有持久化介质（数据库/日志/监控/错误上报/调试输出）均不留存模型调用正文——包括 prompt、工具调用参数与结果、模型输出文本；仅留脱敏计量元数据（REQ-GW-003）；此承诺为最高级产品约束，弱化须经 APC 授权                                                                                                                                                                                                                                                                                                                                                                          | docs/02_cloud/A-GW-M-D-01_AI网关功能需求描述.md          |

### 需求映射表

| req_id       | target_type | target_id          | status | reason | 来源文档                                                                                  |
| ------------ | ----------- | ------------------ | ------ | ------ | ------------------------------------------------------------------------------------- |
| REQ-ACCT-001 | RULE        | BR-ACCT-STATE-001  | mapped |        | docs/01_modules/ACCT/A-ACCT-M-T-01_账号会话技术设计.md                                        |
| REQ-ACCT-002 | RULE        | BR-ACCT-DATA-001   | mapped |        | docs/01_modules/ACCT/A-ACCT-M-T-01_账号会话技术设计.md                                        |
| REQ-ACCT-003 | RULE        | BR-ACCT-STATE-001  | mapped |        | docs/01_modules/ACCT/A-ACCT-M-T-01_账号会话技术设计.md                                        |
| REQ-ACCT-004 | RULE        | BR-ACCT-STATE-001  | mapped |        | docs/01_modules/ACCT/A-ACCT-M-T-01_账号会话技术设计.md                                        |
| REQ-AG-007   | RULE        | BR-SYS-STATE-007   | mapped |        | docs/01_modules/AG/A-AG-M-S-01_代理状态机设计.md                                             |
| REQ-AG-015   | RULE        | BR-SYS-STATE-008   | mapped |        | docs/01_modules/AG/A-AG-M-S-01_代理状态机设计.md                                             |
| REQ-AG-015   | RULE        | BR-CORE-STATE-001  | mapped |        | docs/01_modules/AG/A-AG-M-S-01_代理状态机设计.md                                             |
| REQ-AG-001   | CHAIN       | AG-TURN            | mapped |        | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md                                              |
| REQ-AG-001   | RULE        | BR-AG-STATE-001    | mapped |        | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md                                              |
| REQ-AG-001   | RULE        | BR-AG-PERSIST-001  | mapped |        | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md                                              |
| REQ-AG-002   | CHAIN       | AG-TOOL            | mapped |        | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md                                              |
| REQ-AG-002   | RULE        | BR-AG-STATE-002    | mapped |        | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md                                              |
| REQ-AG-002   | CONSTRAINT  | X-FORBID-006       | mapped |        | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md                                              |
| REQ-AG-003   | RULE        | BR-AG-VERIFY-001   | mapped |        | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md                                              |
| REQ-AG-004   | RULE        | BR-AG-DATA-001     | mapped |        | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md                                              |
| REQ-AG-005   | CHAIN       | AG-ASSIST          | mapped |        | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md                                              |
| REQ-AG-005   | RULE        | BR-AG-STATE-003    | mapped |        | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md                                              |
| REQ-AG-004   | RULE        | BR-AG-DATA-002     | mapped |        | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md                                              |
| REQ-AG-017   | RULE        | BR-AG-STATE-004    | mapped |        | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md                                              |
| REQ-AG-001   | RULE        | BR-AG-DATA-003     | mapped |        | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md                                              |
| REQ-AG-001   | RULE        | BR-AG-UI-001       | mapped |        | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md                                              |
| REQ-AG-006   | CHAIN       | AG-MODEL-CATALOG   | mapped |        | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md                                              |
| REQ-AG-006   | RULE        | BR-AG-DATA-006     | mapped |        | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md                                              |
| REQ-AG-006   | RULE        | BR-AG-DATA-007     | mapped |        | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md                                              |
| REQ-AG-008   | CHAIN       | WS-OPERATION       | mapped |        | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md                                              |
| REQ-AG-008   | RULE        | BR-WS-STATE-003    | mapped |        | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md                                              |
| REQ-AG-011   | CHAIN       | WS-OPERATION       | mapped |        | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md                                              |
| REQ-AG-011   | RULE        | BR-WS-PERSIST-003  | mapped |        | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md                                              |
| REQ-AG-013   | RULE        | BR-WS-DATA-003     | mapped |        | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md                                              |
| REQ-AG-009   | CHAIN       | AG-TURN            | mapped |        | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md                                              |
| REQ-AG-009   | RULE        | BR-AG-DATA-008     | mapped |        | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md                                              |
| REQ-AG-009   | RULE        | BR-AG-OBS-001      | mapped |        | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md                                              |
| REQ-AG-009   | CONSTRAINT  | X-SHARED-001       | mapped |        | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md                                              |
| REQ-AG-009   | CONSTRAINT  | X-SHARED-002       | mapped |        | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md                                              |
| REQ-AG-004   | RULE        | BR-AG-DATA-004     | mapped |        | docs/01_modules/AG/A-AG-M-T-02_提示词架构设计.md                                             |
| REQ-AG-004   | RULE        | BR-AG-DATA-005     | mapped |        | docs/01_modules/AG/A-AG-M-T-02_提示词架构设计.md                                             |
| REQ-AST-001  | RULE        | BR-SYS-DATA-007    | mapped |        | docs/01_modules/AST/A-AST-M-T-01_记忆协同技术设计.md                                          |
| REQ-AST-001  | CHAIN       | AST-MEMORY         | mapped |        | docs/01_modules/AST/A-AST-M-T-01_记忆协同技术设计.md                                          |
| REQ-AST-001  | RULE        | BR-AST-DATA-001    | mapped |        | docs/01_modules/AST/A-AST-M-T-01_记忆协同技术设计.md                                          |
| REQ-AST-002  | RULE        | BR-AST-STATE-001   | mapped |        | docs/01_modules/AST/A-AST-M-T-01_记忆协同技术设计.md                                          |
| REQ-AST-003  | RULE        | BR-AST-DATA-002    | mapped |        | docs/01_modules/AST/A-AST-M-T-01_记忆协同技术设计.md                                          |
| REQ-AST-004  | RULE        | BR-AST-DATA-001    | mapped |        | docs/01_modules/AST/A-AST-M-T-01_记忆协同技术设计.md                                          |
| REQ-CBT-001  | CHAIN       | CBT-BUILD          | mapped |        | docs/01_modules/CBT/A-CBT-M-T-01_构建技术设计.md                                            |
| REQ-CBT-001  | RULE        | BR-CBT-STATE-001   | mapped |        | docs/01_modules/CBT/A-CBT-M-T-01_构建技术设计.md                                            |
| REQ-CBT-002  | RULE        | BR-CBT-DATA-001    | mapped |        | docs/01_modules/CBT/A-CBT-M-T-01_构建技术设计.md                                            |
| REQ-CBT-003  | RULE        | BR-CBT-PERSIST-001 | mapped |        | docs/01_modules/CBT/A-CBT-M-T-01_构建技术设计.md                                            |
| REQ-CBT-003  | RULE        | BR-CBT-STATE-001   | mapped |        | docs/01_modules/CBT/A-CBT-M-T-01_构建技术设计.md                                            |
| REQ-CBT-004  | RULE        | BR-CBT-STATE-002   | mapped |        | docs/01_modules/CBT/A-CBT-M-T-01_构建技术设计.md                                            |
| REQ-DE-009   | RULE        | BR-SYS-STATE-008   | mapped |        | docs/01_modules/DE/A-DE-M-S-01_DiffEdit状态机设计.md                                       |
| REQ-DE-009   | RULE        | BR-CORE-STATE-001  | mapped |        | docs/01_modules/DE/A-DE-M-S-01_DiffEdit状态机设计.md                                       |
| REQ-DE-009   | RULE        | BR-CORE-STATE-003  | mapped |        | docs/01_modules/DE/A-DE-M-S-01_DiffEdit状态机设计.md                                       |
| REQ-DE-001   | CHAIN       | DE-DIFF            | mapped |        | docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md                                        |
| REQ-DE-001   | RULE        | BR-DE-DATA-001     | mapped |        | docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md                                        |
| REQ-DE-002   | RULE        | BR-DE-STATE-001    | mapped |        | docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md                                        |
| REQ-DE-003   | RULE        | BR-DE-STATE-001    | mapped |        | docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md                                        |
| REQ-DE-003   | RULE        | BR-CORE-STATE-001  | mapped |        | docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md                                        |
| REQ-DE-003   | RULE        | BR-CORE-STATE-002  | mapped |        | docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md                                        |
| REQ-DE-003   | RULE        | BR-CORE-STATE-003  | mapped |        | docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md                                        |
| REQ-DE-003   | RULE        | BR-DE-DATA-003     | mapped |        | docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md                                        |
| REQ-DE-004   | RULE        | BR-DE-DATA-002     | mapped |        | docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md                                        |
| REQ-DE-005   | RULE        | BR-DE-STATE-002    | mapped |        | docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md                                        |
| REQ-DE-006   | RULE        | BR-WS-STATE-003    | mapped |        | docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md                                        |
| REQ-DE-006   | RULE        | BR-DE-STATE-001    | mapped |        | docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md                                        |
| REQ-DE-007   | CONSTRAINT  | X-FORBID-006       | mapped |        | docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md                                        |
| REQ-DE-007   | RULE        | BR-WS-OBS-002      | mapped |        | docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md                                        |
| REQ-DOC-001  | CHAIN       | DOC-CONVERT        | mapped |        | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md                                          |
| REQ-DOC-001  | RULE        | BR-DOC-STATE-001   | mapped |        | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md                                          |
| REQ-DOC-002  | RULE        | BR-DOC-STATE-001   | mapped |        | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md                                          |
| REQ-DOC-002  | RULE        | BR-DOC-DATA-003    | mapped |        | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md                                          |
| REQ-DOC-003  | RULE        | BR-DOC-DATA-002    | mapped |        | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md                                          |
| REQ-DOC-004  | RULE        | BR-DOC-DATA-001    | mapped |        | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md                                          |
| REQ-DOC-005  | RULE        | BR-DOC-DATA-001    | mapped |        | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md                                          |
| REQ-DOC-005  | CONSTRAINT  | X-SEQ-004          | mapped |        | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md                                          |
| REQ-ED-010   | RULE        | BR-CORE-STATE-001  | mapped |        | docs/01_modules/ED/A-ED-M-S-01_编辑器状态机设计.md, docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md |
| REQ-ED-010   | RULE        | BR-WS-STATE-003    | mapped |        | docs/01_modules/ED/A-ED-M-S-01_编辑器状态机设计.md                                            |
| REQ-ED-010   | RULE        | BR-SYS-STATE-008   | mapped |        | docs/01_modules/ED/A-ED-M-S-01_编辑器状态机设计.md                                            |
| REQ-ED-014   | RULE        | BR-CORE-STATE-001  | mapped |        | docs/01_modules/ED/A-ED-M-S-01_编辑器状态机设计.md                                            |
| REQ-ED-014   | RULE        | BR-CORE-STATE-003  | mapped |        | docs/01_modules/ED/A-ED-M-S-01_编辑器状态机设计.md                                            |
| REQ-ED-014   | RULE        | BR-WS-STATE-003    | mapped |        | docs/01_modules/ED/A-ED-M-S-01_编辑器状态机设计.md                                            |
| REQ-ED-001   | CHAIN       | ED-EDIT            | mapped |        | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md                                             |
| REQ-ED-001   | RULE        | BR-ED-STATE-001    | mapped |        | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md                                             |
| REQ-ED-001   | RULE        | BR-ED-DATA-002     | mapped |        | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md                                             |
| REQ-ED-001   | RULE        | BR-ED-STATE-003    | mapped |        | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md                                             |
| REQ-ED-001   | RULE        | BR-ED-DATA-006     | mapped |        | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md                                             |
| REQ-ED-001   | RULE        | BR-ED-VERIFY-001   | mapped |        | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md                                             |
| REQ-ED-002   | RULE        | BR-ED-DATA-001     | mapped |        | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md                                             |
| REQ-ED-003   | RULE        | BR-ED-DATA-003     | mapped |        | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md                                             |
| REQ-ED-004   | RULE        | BR-ED-DATA-003     | mapped |        | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md                                             |
| REQ-ED-005   | CHAIN       | ED-DECOR           | mapped |        | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md                                             |
| REQ-ED-005   | RULE        | BR-ED-STATE-002    | mapped |        | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md                                             |
| REQ-ED-005   | CONSTRAINT  | X-FORBID-005       | mapped |        | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md                                             |
| REQ-ED-006   | RULE        | BR-ED-DATA-004     | mapped |        | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md                                             |
| REQ-ED-007   | RULE        | BR-ED-DATA-005     | mapped |        | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md                                             |
| REQ-ED-008   | RULE        | BR-ED-DATA-002     | mapped |        | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md                                             |
| REQ-ED-009   | RULE        | BR-ED-DATA-007     | mapped |        | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md                                             |
| REQ-ED-010   | RULE        | BR-CORE-STATE-003  | mapped |        | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md                                             |
| REQ-ED-011   | RULE        | BR-ED-DATA-008     | mapped |        | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md                                             |
| REQ-ED-011   | CONSTRAINT  | X-SHARED-001       | mapped |        | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md                                             |
| REQ-ENT-001  | RULE        | BR-ENT-VERIFY-001  | mapped |        | docs/01_modules/ENT/A-ENT-M-T-01_订阅门控技术设计.md                                          |
| REQ-ENT-002  | RULE        | BR-ENT-STATE-001   | mapped |        | docs/01_modules/ENT/A-ENT-M-T-01_订阅门控技术设计.md                                          |
| REQ-ENT-003  | RULE        | BR-ENT-DATA-001    | mapped |        | docs/01_modules/ENT/A-ENT-M-T-01_订阅门控技术设计.md                                          |
| REQ-ENT-004  | RULE        | BR-ENT-DATA-002    | mapped |        | docs/01_modules/ENT/A-ENT-M-T-01_订阅门控技术设计.md                                          |
| REQ-KB-001   | CHAIN       | KB-RETRIEVE        | mapped |        | docs/01_modules/KB/A-KB-M-T-01_知识库技术设计.md                                             |
| REQ-KB-001   | RULE        | BR-KB-DATA-001     | mapped |        | docs/01_modules/KB/A-KB-M-T-01_知识库技术设计.md                                             |
| REQ-KB-002   | RULE        | BR-KB-DATA-001     | mapped |        | docs/01_modules/KB/A-KB-M-T-01_知识库技术设计.md                                             |
| REQ-KB-003   | RULE        | BR-KB-DATA-002     | mapped |        | docs/01_modules/KB/A-KB-M-T-01_知识库技术设计.md                                             |
| REQ-KB-004   | RULE        | BR-KB-DATA-002     | mapped |        | docs/01_modules/KB/A-KB-M-T-01_知识库技术设计.md                                             |
| REQ-PRV-001  | CHAIN       | PRV-VIEW           | mapped |        | docs/01_modules/PRV/A-PRV-M-T-01_预览输出技术设计.md                                          |
| REQ-PRV-001  | RULE        | BR-PRV-DATA-002    | mapped |        | docs/01_modules/PRV/A-PRV-M-T-01_预览输出技术设计.md                                          |
| REQ-PRV-002  | RULE        | BR-PRV-DATA-001    | mapped |        | docs/01_modules/PRV/A-PRV-M-T-01_预览输出技术设计.md                                          |
| REQ-PRV-003  | RULE        | BR-PRV-DATA-002    | mapped |        | docs/01_modules/PRV/A-PRV-M-T-01_预览输出技术设计.md                                          |
| REQ-PRV-004  | RULE        | BR-PRV-DATA-002    | mapped |        | docs/01_modules/PRV/A-PRV-M-T-01_预览输出技术设计.md                                          |
| REQ-PRV-005  | RULE        | BR-PRV-DATA-002    | mapped |        | docs/01_modules/PRV/A-PRV-M-T-01_预览输出技术设计.md                                          |
| REQ-REF-009  | RULE        | BR-SYS-DATA-007    | mapped |        | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md                                            |
| REQ-REF-001  | CHAIN       | REF-INJECT         | mapped |        | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md                                            |
| REQ-REF-001  | RULE        | BR-REF-DATA-001    | mapped |        | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md                                            |
| REQ-REF-002  | RULE        | BR-REF-DATA-002    | mapped |        | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md                                            |
| REQ-REF-003  | RULE        | BR-REF-VERIFY-001  | mapped |        | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md                                            |
| REQ-REF-004  | RULE        | BR-REF-DATA-001    | mapped |        | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md                                            |
| REQ-REF-005  | RULE        | BR-REF-DATA-003    | mapped |        | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md                                            |
| REQ-REF-006  | RULE        | BR-REF-DATA-004    | mapped |        | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md                                            |
| REQ-SYS-004  | CONSTRAINT  | X-SEQ-002          | mapped |        | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md                                            |
| REQ-SYS-004  | CONSTRAINT  | X-FORBID-003       | mapped |        | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md                                            |
| REQ-SYS-006  | CONSTRAINT  | X-FORBID-007       | mapped |        | docs/01_modules/SYS/A-SYS-C-T-02_前端交互架构与流程总图.md                                       |
| REQ-SYS-002  | CHAIN       | DATA-MIGRATE       | mapped |        | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md                                            |
| REQ-SYS-002  | RULE        | BR-SYS-DATA-002    | mapped |        | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md                                            |
| REQ-SYS-002  | RULE        | BR-SYS-PERSIST-001 | mapped |        | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md                                            |
| REQ-SYS-002  | CONSTRAINT  | X-SHARED-001       | mapped |        | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md                                            |
| REQ-SYS-008  | CONSTRAINT  | X-FORBID-008       | mapped |        | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md                                            |
| REQ-SYS-008  | CONSTRAINT  | X-FORBID-009       | mapped |        | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md                                            |
| REQ-SYS-009  | CONSTRAINT  | X-SHARED-002       | mapped |        | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md                                            |
| REQ-SYS-009  | RULE        | BR-SYS-DATA-008    | mapped |        | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md                                            |
| REQ-SYS-010  | RULE        | BR-SYS-STATE-007   | mapped |        | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md, docs/01_modules/SYS/A-SYS-M-S-01_状态机总图.md |
| REQ-SYS-010  | CONSTRAINT  | X-SHARED-002       | mapped |        | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md                                            |
| REQ-SYS-011  | RULE        | BR-SYS-STATE-008   | mapped |        | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md, docs/01_modules/SYS/A-SYS-M-S-01_状态机总图.md |
| REQ-SYS-011  | CONSTRAINT  | X-SHARED-002       | mapped |        | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md                                            |
| REQ-SYS-012  | RULE        | BR-SYS-DATA-007    | mapped |        | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md, docs/01_modules/SYS/A-SYS-M-S-01_状态机总图.md |
| REQ-SYS-012  | CONSTRAINT  | X-SHARED-002       | mapped |        | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md                                            |
| REQ-SYS-001  | CHAIN       | IPC-CALL           | mapped |        | docs/01_modules/SYS/A-SYS-I-P-02_IPC协议.md                                             |
| REQ-SYS-001  | RULE        | BR-SYS-DATA-003    | mapped |        | docs/01_modules/SYS/A-SYS-I-P-02_IPC协议.md                                             |
| REQ-SYS-001  | RULE        | BR-SYS-OBS-002     | mapped |        | docs/01_modules/SYS/A-SYS-I-P-02_IPC协议.md                                             |
| REQ-SYS-001  | RULE        | BR-SYS-STATE-002   | mapped |        | docs/01_modules/SYS/A-SYS-I-P-02_IPC协议.md                                             |
| REQ-SYS-001  | RULE        | BR-SYS-STATE-003   | mapped |        | docs/01_modules/SYS/A-SYS-I-P-02_IPC协议.md                                             |
| REQ-SYS-005  | CHAIN       | CLOUD-AUTH         | mapped |        | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md                                             |
| REQ-SYS-005  | CHAIN       | CLOUD-GATE         | mapped |        | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md                                             |
| REQ-SYS-005  | CHAIN       | CLOUD-INVOKE       | mapped |        | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md                                             |
| REQ-SYS-005  | RULE        | BR-SYS-VERIFY-001  | mapped |        | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md                                             |
| REQ-SYS-005  | RULE        | BR-SYS-DATA-001    | mapped |        | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md                                             |
| REQ-SYS-005  | RULE        | BR-SYS-DATA-004    | mapped |        | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md                                             |
| REQ-SYS-005  | RULE        | BR-SYS-STATE-001   | mapped |        | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md                                             |
| REQ-SYS-005  | CONSTRAINT  | X-FORBID-001       | mapped |        | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md                                             |
| REQ-SYS-005  | CONSTRAINT  | X-SEQ-001          | mapped |        | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md                                             |
| REQ-SYS-005  | CONSTRAINT  | X-FORBID-002       | mapped |        | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md                                             |
| REQ-SYS-002  | RULE        | BR-SYS-DATA-006    | mapped |        | docs/01_modules/SYS/A-SYS-I-P-04_BDocx代理编辑能力协议.md                                     |
| REQ-SYS-003  | CONSTRAINT  | X-SEQ-003          | mapped |        | docs/01_modules/SYS/A-SYS-M-S-01_状态机总图.md                                             |
| REQ-SYS-003  | CONSTRAINT  | X-INTER-001        | mapped |        | docs/01_modules/SYS/A-SYS-M-S-01_状态机总图.md                                             |
| REQ-SYS-010  | CONSTRAINT  | X-SEQ-003          | mapped |        | docs/01_modules/SYS/A-SYS-M-S-01_状态机总图.md                                             |
| REQ-SYS-011  | CONSTRAINT  | X-SEQ-003          | mapped |        | docs/01_modules/SYS/A-SYS-M-S-01_状态机总图.md                                             |
| REQ-SYS-012  | CONSTRAINT  | X-SEQ-003          | mapped |        | docs/01_modules/SYS/A-SYS-M-S-01_状态机总图.md                                             |
| REQ-SYS-007  | CHAIN       | APP-CLOSE          | mapped |        | docs/01_modules/SYS/A-SYS-M-T-02_应用关闭技术设计.md                                          |
| REQ-SYS-007  | RULE        | BR-SYS-STATE-004   | mapped |        | docs/01_modules/SYS/A-SYS-M-T-02_应用关闭技术设计.md                                          |
| REQ-SYS-007  | RULE        | BR-SYS-STATE-005   | mapped |        | docs/01_modules/SYS/A-SYS-M-T-02_应用关闭技术设计.md                                          |
| REQ-SYS-007  | RULE        | BR-SYS-STATE-006   | mapped |        | docs/01_modules/SYS/A-SYS-M-T-02_应用关闭技术设计.md                                          |
| REQ-SYS-007  | RULE        | BR-SYS-DATA-005    | mapped |        | docs/01_modules/SYS/A-SYS-M-T-02_应用关闭技术设计.md                                          |
| REQ-TPL-001  | CHAIN       | TPL-FLOW           | mapped |        | docs/01_modules/TPL/A-TPL-M-T-01_模板工作流技术设计.md                                         |
| REQ-TPL-001  | RULE        | BR-TPL-DATA-001    | mapped |        | docs/01_modules/TPL/A-TPL-M-T-01_模板工作流技术设计.md                                         |
| REQ-TPL-002  | RULE        | BR-TPL-DATA-001    | mapped |        | docs/01_modules/TPL/A-TPL-M-T-01_模板工作流技术设计.md                                         |
| REQ-TPL-003  | RULE        | BR-TPL-STATE-001   | mapped |        | docs/01_modules/TPL/A-TPL-M-T-01_模板工作流技术设计.md                                         |
| REQ-TPL-004  | RULE        | BR-TPL-DATA-001    | mapped |        | docs/01_modules/TPL/A-TPL-M-T-01_模板工作流技术设计.md                                         |
| REQ-WS-001   | CHAIN       | WS-FS              | mapped |        | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md                                             |
| REQ-WS-001   | CONSTRAINT  | X-FORBID-004       | mapped |        | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md                                             |
| REQ-WS-002   | RULE        | BR-WS-DATA-001     | mapped |        | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md                                             |
| REQ-WS-002   | CHAIN       | WS-SAVE            | mapped |        | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md                                             |
| REQ-WS-002   | RULE        | BR-WS-PERSIST-001  | mapped |        | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md                                             |
| REQ-WS-002   | RULE        | BR-WS-OBS-001      | mapped |        | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md                                             |
| REQ-WS-003   | CHAIN       | WS-DIFF-APPLY      | mapped |        | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md                                             |
| REQ-WS-003   | RULE        | BR-WS-STATE-001    | mapped |        | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md                                             |
| REQ-WS-003   | RULE        | BR-CORE-STATE-002  | mapped |        | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md                                             |
| REQ-WS-004   | CHAIN       | WS-TIMELINE        | mapped |        | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md                                             |
| REQ-WS-004   | RULE        | BR-WS-DATA-002     | mapped |        | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md                                             |
| REQ-WS-004   | RULE        | BR-WS-STATE-002    | mapped |        | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md                                             |
| REQ-WS-005   | RULE        | BR-WS-PERSIST-002  | mapped |        | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md                                             |
| REQ-WS-006   | RULE        | BR-WS-STATE-003    | mapped |        | docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md                                         |
| REQ-WS-006   | RULE        | BR-WS-DATA-003     | mapped |        | docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md                                         |
| REQ-WS-008   | CHAIN       | WS-OPERATION       | mapped |        | docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md                                         |
| REQ-WS-008   | RULE        | BR-WS-PERSIST-003  | mapped |        | docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md                                         |
| REQ-WS-009   | RULE        | BR-WS-STATE-003    | mapped |        | docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md                                         |
| REQ-WS-010   | RULE        | BR-WS-DATA-003     | mapped |        | docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md                                         |
| REQ-WS-010   | RULE        | BR-WS-OBS-002      | mapped |        | docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md                                         |
| REQ-WS-008   | RULE        | BR-WS-STATE-009    | mapped |        | docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md                                         |
| REQ-WS-010   | RULE        | BR-WS-STATE-009    | mapped |        | docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md                                         |

### 链路注册表

| chain_id         | 主责模块 | status | 集成测试                                                                                                          |
| ---------------- | ---- | ------ | ------------------------------------------------------------------------------------------------------------- |
| PCG-GENERATE     | PCG  | active | scripts/test_governance.test.mjs::生成可点击的产品故事地图和格式矩阵                                                           |
| PCG-AUDIT        | PCG  | active | scripts/test_governance.test.mjs::接受具有完整追溯、主链和独立状态的注册表                                                        |
| AG-TURN          | AG   | active | deferred: phase-5                                                                                             |
| AG-TOOL          | AG   | active | deferred: phase-5                                                                                             |
| AG-ASSIST        | AG   | active | deferred: phase-5                                                                                             |
| AG-MODEL-CATALOG | AG   | active | deferred: phase-5                                                                                             |
| AST-MEMORY       | AST  | active | deferred: phase-5                                                                                             |
| CBT-BUILD        | CBT  | active | deferred: phase-5                                                                                             |
| DE-DIFF          | DE   | active | src-tauri/tests/core_chain.rs::accepted_proposal_flows_through_de_to_ws_landing                               |
| DOC-CONVERT      | DOC  | active | deferred: phase-5                                                                                             |
| ED-EDIT          | ED   | active | deferred: phase-5                                                                                             |
| ED-DECOR         | ED   | active | deferred: phase-5                                                                                             |
| KB-RETRIEVE      | KB   | active | deferred: phase-5                                                                                             |
| PRV-VIEW         | PRV  | active | deferred: phase-5                                                                                             |
| REF-INJECT       | REF  | active | deferred: phase-5                                                                                             |
| DATA-MIGRATE     | SYS  | active | deferred: phase-5                                                                                             |
| IPC-CALL         | SYS  | active | src-tauri/src/ipc/system_tests.rs::accepted_draft_saves_to_disk_and_reopens_through_tauri_ipc                 |
| CLOUD-AUTH       | SYS  | active | deferred: phase-5                                                                                             |
| CLOUD-GATE       | SYS  | active | deferred: phase-5                                                                                             |
| CLOUD-INVOKE     | SYS  | active | deferred: phase-5                                                                                             |
| APP-CLOSE        | SYS  | active | deferred: phase-5                                                                                             |
| TPL-FLOW         | TPL  | active | deferred: phase-5                                                                                             |
| WS-FS            | WS   | active | deferred: phase-5                                                                                             |
| WS-DIFF-APPLY    | WS   | active | src-tauri/tests/core_chain.rs::accepted_proposal_flows_through_de_to_ws_landing                               |
| WS-TIMELINE      | WS   | active | deferred: phase-5                                                                                             |
| WS-SAVE          | WS   | active | src-tauri/src/ipc/system_tests.rs::accepted_draft_saves_to_disk_and_reopens_through_tauri_ipc                 |
| WS-OPERATION     | WS   | active | src-tauri/src/workspace/operation.rs::plan_freezes_real_baseline_and_execute_requires_bound_resolved_decision |

### 规则注册表

| rule_id            | 域       | 主链路                                  | 来源文档                                              | registry_status | 测试覆盖                                                                                                                                                                                                                                                                                                                                                                        |
| ------------------ | ------- | ------------------------------------ | ------------------------------------------------- | --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| BR-PCG-DATA-001    | DATA    | PCG-GENERATE                         | docs/00_core/A-CORE-C-P-02_产品能力图谱规范.md            | registered      | scripts/test_governance.test.mjs::file-level                                                                                                                                                                                                                                                                                                                                |
| BR-PCG-STATE-001   | STATE   | PCG-GENERATE                         | docs/00_core/A-CORE-C-P-02_产品能力图谱规范.md            | registered      | scripts/test_governance.test.mjs::file-level                                                                                                                                                                                                                                                                                                                                |
| BR-PCG-VERIFY-001  | VERIFY  | PCG-AUDIT                            | docs/00_core/A-CORE-C-P-02_产品能力图谱规范.md            | registered      | scripts/test_governance.test.mjs::file-level                                                                                                                                                                                                                                                                                                                                |
| BR-PCG-VERIFY-002  | VERIFY  | PCG-AUDIT                            | docs/00_core/A-CORE-C-P-02_产品能力图谱规范.md            | registered      | scripts/test_governance.test.mjs::file-level                                                                                                                                                                                                                                                                                                                                |
| BR-CORE-STATE-001  | STATE   | DE-DIFF, ED-EDIT, WS-SAVE            | docs/00_core/A-CORE-C-T-01_三态模型.md                | registered      | src/services/deRuntime.test.ts::file-level, src/services/edRuntime.test.ts::file-level                                                                                                                                                                                                                                                                                      |
| BR-CORE-STATE-002  | STATE   | DE-DIFF, WS-DIFF-APPLY               | docs/00_core/A-CORE-C-T-01_三态模型.md                | registered      | src/services/deRuntime.test.ts::file-level                                                                                                                                                                                                                                                                                                                                  |
| BR-CORE-STATE-003  | STATE   | DE-DIFF, ED-EDIT, ED-DECOR, WS-SAVE  | docs/00_core/A-CORE-C-T-01_三态模型.md                | registered      | src-tauri/src/ipc/system_tests.rs::file-level, src/services/deRuntime.test.ts::file-level                                                                                                                                                                                                                                                                                   |
| BR-ACCT-STATE-001  | STATE   | CLOUD-AUTH                           | docs/01_modules/ACCT/A-ACCT-M-T-01_账号会话技术设计.md    | registered      | src-tauri/src/acct/session.rs::file-level                                                                                                                                                                                                                                                                                                                                   |
| BR-ACCT-DATA-001   | DATA    | CLOUD-AUTH                           | docs/01_modules/ACCT/A-ACCT-M-T-01_账号会话技术设计.md    | registered      | src-tauri/src/acct/token.rs::file-level                                                                                                                                                                                                                                                                                                                                     |
| BR-AG-STATE-001    | STATE   | AG-TURN                              | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | src-tauri/src/ag/model_output.rs::file-level, src-tauri/src/ag/turn.rs::file-level, src-tauri/src/ag/turn_control.rs::file-level, src-tauri/src/ag/turn_runtime.rs::file-level                                                                                                                                                                                              |
| BR-AG-DATA-003     | DATA    | AG-TURN                              | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | src-tauri/src/ag/target_set.rs::file-level, src-tauri/src/ag/task.rs::file-level, src-tauri/src/ag/task_runtime.rs::file-level, src-tauri/src/ag/turn_control.rs::file-level, src-tauri/src/de/generate.rs::file-level                                                                                                                                                      |
| BR-AG-VERIFY-001   | VERIFY  | AG-TURN                              | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | src-tauri/src/ag/turn.rs::file-level                                                                                                                                                                                                                                                                                                                                        |
| BR-AG-STATE-005    | STATE   | AG-TURN                              | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | src-tauri/src/ag/turn_tool_loop.rs::file-level                                                                                                                                                                                                                                                                                                                              |
| BR-AG-PERSIST-001  | PERSIST | AG-TURN                              | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | src-tauri/src/ag/capability_runtime.rs::file-level, src-tauri/src/ag/history.rs::file-level, src-tauri/src/ag/turn_events.rs::file-level, src-tauri/src/db/mod.rs::file-level, src/services/agRuntime.test.ts::file-level                                                                                                                                                   |
| BR-AG-UI-001       | UI      | AG-TURN                              | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | src/components/ag/AgentUI.test.tsx::file-level, src/services/agRuntime.test.ts::file-level                                                                                                                                                                                                                                                                                  |
| BR-AG-STATE-002    | STATE   | AG-TOOL                              | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | src-tauri/src/ag/capability_runtime.rs::file-level, src-tauri/src/ag/model_output.rs::file-level, src-tauri/src/ag/tool.rs::file-level, src-tauri/src/ag/tool_exec.rs::file-level, src-tauri/src/ag/tool_registry.rs::file-level, src-tauri/src/ag/turn_runtime.rs::file-level, src-tauri/src/ag/turn_tool_loop.rs::file-level                                              |
| BR-AG-DATA-011     | DATA    | AG-TOOL                              | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | src-tauri/src/ag/turn_tool_loop.rs::file-level                                                                                                                                                                                                                                                                                                                              |
| BR-AG-DATA-012     | DATA    | AG-TOOL                              | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | src-tauri/src/ag/turn_tool_loop.rs::file-level                                                                                                                                                                                                                                                                                                                              |
| BR-AG-DATA-013     | DATA    | AG-TOOL                              | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | src-tauri/src/ag/turn_tool_loop.rs::file-level                                                                                                                                                                                                                                                                                                                              |
| BR-AG-DATA-001     | DATA    | AG-TURN                              | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | src-tauri/src/ag/block_ops.rs::file-level, src-tauri/src/ag/context.rs::file-level, src-tauri/src/ag/turn_prompt.rs::file-level, src-tauri/src/ag/turn_runtime.rs::file-level, src/services/agRuntime.test.ts::file-level                                                                                                                                                   |
| BR-AG-DATA-008     | DATA    | AG-TURN                              | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | src-tauri/src/ag/tool_exec.rs::file-level, src-tauri/src/ag/turn_runtime.rs::file-level, src/services/agRuntime.test.ts::file-level                                                                                                                                                                                                                                         |
| BR-AG-OBS-001      | OBS     | AG-TURN, AG-TOOL                     | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | src-tauri/src/ag/tool_exec.rs::file-level, src-tauri/src/ag/turn_events.rs::file-level, src-tauri/src/ag/turn_runtime.rs::file-level                                                                                                                                                                                                                                        |
| BR-AG-DATA-002     | DATA    | AG-TURN                              | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | src-tauri/src/ag/assist_prompt.rs::file-level, src-tauri/src/ag/block_ops.rs::file-level, src-tauri/src/ag/context.rs::file-level, src-tauri/src/ag/turn_prompt.rs::file-level, src-tauri/src/ag/turn_runtime.rs::file-level                                                                                                                                                |
| BR-AG-STATE-004    | STATE   | AG-TURN                              | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | src-tauri/src/ag/turn_runtime.rs::file-level                                                                                                                                                                                                                                                                                                                                |
| BR-AG-STATE-003    | STATE   | AG-ASSIST                            | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | src-tauri/src/ag/assist.rs::file-level, src-tauri/src/ag/assist_prompt.rs::file-level, src-tauri/src/ent/gateway.rs::file-level, src/components/ag/AgentUI.test.tsx::file-level, src/services/agRuntime.test.ts::file-level, src/store/assistRuntime.test.ts::file-level                                                                                                    |
| BR-AG-DATA-006     | DATA    | AG-MODEL-CATALOG                     | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | src/store/assistRuntime.test.ts::file-level                                                                                                                                                                                                                                                                                                                                 |
| BR-AG-DATA-007     | DATA    | AG-MODEL-CATALOG                     | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | src-tauri/src/ag/turn_control.rs::file-level, src/components/ag/AgentUI.test.tsx::file-level, src/services/agRuntime.test.ts::file-level, src/store/assistRuntime.test.ts::file-level                                                                                                                                                                                       |
| BR-AG-DATA-009     | DATA    | AG-TOOL                              | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | MISSING                                                                                                                                                                                                                                                                                                                                                                     |
| BR-AG-DATA-010     | DATA    | AG-TOOL                              | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          | registered      | MISSING                                                                                                                                                                                                                                                                                                                                                                     |
| BR-AG-DATA-004     | DATA    | AG-TURN                              | docs/01_modules/AG/A-AG-M-T-02_提示词架构设计.md         | registered      | src-tauri/src/ag/context.rs::file-level, src-tauri/src/ag/model_output.rs::file-level, src-tauri/src/ag/prompt_runtime.rs::file-level, src-tauri/src/ag/turn_prompt.rs::file-level, src-tauri/src/ag/turn_runtime.rs::file-level                                                                                                                                            |
| BR-AG-DATA-005     | DATA    | AG-TURN                              | docs/01_modules/AG/A-AG-M-T-02_提示词架构设计.md         | registered      | src-tauri/src/ag/blocks.rs::file-level, src-tauri/src/ag/context.rs::file-level, src-tauri/src/ag/prompt_runtime.rs::file-level, src-tauri/src/ag/tool_exec.rs::file-level, src-tauri/src/ag/turn_events.rs::file-level, src-tauri/src/ag/turn_prompt.rs::file-level, src-tauri/src/ag/turn_runtime.rs::file-level, src/components/ag/AgentUI.test.tsx::file-level          |
| BR-AST-DATA-001    | DATA    | AST-MEMORY                           | docs/01_modules/AST/A-AST-M-T-01_记忆协同技术设计.md      | registered      | src-tauri/src/ast/store.rs::file-level                                                                                                                                                                                                                                                                                                                                      |
| BR-AST-STATE-001   | STATE   | AST-MEMORY                           | docs/01_modules/AST/A-AST-M-T-01_记忆协同技术设计.md      | registered      | src-tauri/src/ast/capture.rs::file-level                                                                                                                                                                                                                                                                                                                                    |
| BR-AST-DATA-002    | DATA    | AST-MEMORY                           | docs/01_modules/AST/A-AST-M-T-01_记忆协同技术设计.md      | registered      | src-tauri/src/ast/supply.rs::file-level                                                                                                                                                                                                                                                                                                                                     |
| BR-CBT-STATE-001   | STATE   | CBT-BUILD                            | docs/01_modules/CBT/A-CBT-M-T-01_构建技术设计.md        | registered      | src-tauri/src/cbt/build.rs::file-level                                                                                                                                                                                                                                                                                                                                      |
| BR-CBT-DATA-001    | DATA    | CBT-BUILD                            | docs/01_modules/CBT/A-CBT-M-T-01_构建技术设计.md        | registered      | src-tauri/src/cbt/apply.rs::file-level                                                                                                                                                                                                                                                                                                                                      |
| BR-CBT-PERSIST-001 | PERSIST | CBT-BUILD                            | docs/01_modules/CBT/A-CBT-M-T-01_构建技术设计.md        | registered      | src-tauri/src/cbt/build.rs::file-level                                                                                                                                                                                                                                                                                                                                      |
| BR-CBT-STATE-002   | STATE   | CBT-BUILD                            | docs/01_modules/CBT/A-CBT-M-T-01_构建技术设计.md        | registered      | src-tauri/src/cbt/takeover.rs::file-level                                                                                                                                                                                                                                                                                                                                   |
| BR-DE-DATA-001     | DATA    | DE-DIFF                              | docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md    | registered      | src-tauri/src/ag/block_ops.rs::file-level, src-tauri/src/ag/capability_runtime.rs::file-level, src-tauri/src/de/generate.rs::file-level, src/services/edOperations.test.ts::file-level                                                                                                                                                                                      |
| BR-DE-DATA-002     | DATA    | DE-DIFF                              | docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md    | registered      | src-tauri/src/de/generate.rs::file-level, src-tauri/src/de/routes.rs::file-level, src-tauri/src/de/store.rs::file-level, src-tauri/src/workspace/diff_apply.rs::file-level, src/components/de/DiffReview.test.tsx::file-level, src/components/ed/EditorCore.test.tsx::file-level, src/services/deRuntime.test.ts::file-level, src/services/edOperations.test.ts::file-level |
| BR-DE-STATE-001    | STATE   | DE-DIFF                              | docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md    | registered      | src-tauri/src/de/mod.rs::file-level, src-tauri/src/de/routes.rs::file-level, src-tauri/src/ipc/system_tests.rs::file-level, src/components/de/DiffReview.test.tsx::file-level, src/services/deRuntime.test.ts::file-level                                                                                                                                                   |
| BR-DE-STATE-002    | STATE   | DE-DIFF                              | docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md    | registered      | src-tauri/src/de/expiry.rs::file-level, src-tauri/src/de/lifecycle.rs::file-level                                                                                                                                                                                                                                                                                           |
| BR-DE-DATA-003     | DATA    | DE-DIFF                              | docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md    | registered      | src-tauri/src/de/expiry.rs::file-level, src-tauri/src/de/lifecycle.rs::file-level                                                                                                                                                                                                                                                                                           |
| BR-DOC-DATA-001    | DATA    | DOC-CONVERT                          | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md      | registered      | src-tauri/src/doc/convert.rs::file-level, src-tauri/src/doc/font_trace.rs::file-level, src-tauri/src/doc/libreoffice_runtime.rs::file-level, src-tauri/src/doc/pandoc_runtime.rs::file-level, src-tauri/src/doc/tool_runtime.rs::file-level                                                                                                                                 |
| BR-DOC-STATE-001   | STATE   | DOC-CONVERT                          | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md      | registered      | src-tauri/src/doc/convert.rs::file-level                                                                                                                                                                                                                                                                                                                                    |
| BR-DOC-DATA-002    | DATA    | DOC-CONVERT                          | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md      | registered      | src-tauri/src/doc/docx_cell_shading.rs::file-level, src-tauri/src/doc/edit_chain.rs::file-level, src-tauri/src/doc/pandoc_runtime.rs::file-level, src-tauri/src/doc/style_subset.rs::file-level                                                                                                                                                                             |
| BR-DOC-DATA-003    | DATA    | DOC-CONVERT                          | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md      | registered      | src-tauri/src/doc/block_backfill.rs::file-level                                                                                                                                                                                                                                                                                                                             |
| BR-ED-STATE-001    | STATE   | ED-EDIT                              | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         | registered      | src/components/ed/EditorCore.test.tsx::file-level, src/components/ed/extensions/EditorExtensions.test.ts::file-level, src/components/shell/ShellComponents.test.tsx::file-level, src/services/deRuntime.test.ts::file-level, src/services/edOperations.test.ts::file-level, src/services/workspaceRuntime.test.ts::file-level                                               |
| BR-ED-STATE-003    | STATE   | ED-EDIT                              | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         | registered      | src/components/ed/EditorControls.test.tsx::file-level, src/services/edRuntime.test.ts::file-level                                                                                                                                                                                                                                                                           |
| BR-ED-DATA-001     | DATA    | ED-EDIT                              | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         | registered      | src/components/ed/EditorControls.test.tsx::file-level, src/components/ed/EditorCore.test.tsx::file-level, src/components/ed/extensions/EditorExtensions.test.ts::file-level, src/services/edOperations.test.ts::file-level                                                                                                                                                  |
| BR-ED-DATA-004     | DATA    | ED-EDIT                              | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         | registered      | src/services/edRuntime.test.ts::file-level                                                                                                                                                                                                                                                                                                                                  |
| BR-ED-DATA-002     | DATA    | ED-EDIT                              | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         | registered      | src/services/edRuntime.test.ts::file-level                                                                                                                                                                                                                                                                                                                                  |
| BR-ED-DATA-006     | DATA    | ED-EDIT                              | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         | registered      | src/components/ed/extensions/EditorExtensions.test.ts::file-level                                                                                                                                                                                                                                                                                                           |
| BR-ED-VERIFY-001   | VERIFY  | ED-EDIT                              | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         | registered      | src/components/ed/extensions/EditorExtensions.test.ts::file-level                                                                                                                                                                                                                                                                                                           |
| BR-ED-DATA-005     | DATA    | ED-EDIT                              | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         | registered      | src-tauri/src/ag/blocks.rs::file-level, src/services/edOperations.test.ts::file-level                                                                                                                                                                                                                                                                                       |
| BR-ED-DATA-007     | DATA    | ED-EDIT, AG-TOOL, DE-DIFF            | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         | registered      | src/components/de/DiffReview.test.tsx::file-level, src/components/ed/EditorControls.test.tsx::file-level, src/components/ed/extensions/EditorExtensions.test.ts::file-level, src/services/edOperations.test.ts::file-level                                                                                                                                                  |
| BR-ED-DATA-003     | DATA    | ED-EDIT                              | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         | registered      | src/components/ed/EditorControls.test.tsx::file-level, src/components/ed/EditorCore.test.tsx::file-level, src/components/ed/extensions/EditorExtensions.test.ts::file-level, src/components/ed/extensions/PaginationSuite.test.ts::file-level, src/services/docService.test.ts::file-level, src/services/edOperations.test.ts::file-level                                   |
| BR-ED-STATE-002    | STATE   | ED-DECOR                             | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         | registered      | src/components/ed/extensions/EditorExtensions.test.ts::file-level                                                                                                                                                                                                                                                                                                           |
| BR-ED-DATA-008     | DATA    | ED-EDIT, AG-TURN                     | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         | registered      | src-tauri/src/ag/turn_runtime.rs::file-level, src/services/agRuntime.test.ts::file-level                                                                                                                                                                                                                                                                                    |
| BR-ENT-VERIFY-001  | VERIFY  | CLOUD-GATE                           | docs/01_modules/ENT/A-ENT-M-T-01_订阅门控技术设计.md      | registered      | src-tauri/src/ag/turn_runtime.rs::file-level, src-tauri/src/ent/gateway.rs::file-level                                                                                                                                                                                                                                                                                      |
| BR-ENT-STATE-001   | STATE   | CLOUD-INVOKE                         | docs/01_modules/ENT/A-ENT-M-T-01_订阅门控技术设计.md      | registered      | src-tauri/src/ent/gateway.rs::file-level                                                                                                                                                                                                                                                                                                                                    |
| BR-ENT-DATA-001    | DATA    | CLOUD-INVOKE                         | docs/01_modules/ENT/A-ENT-M-T-01_订阅门控技术设计.md      | registered      | src-tauri/src/ent/byok.rs::file-level                                                                                                                                                                                                                                                                                                                                       |
| BR-ENT-DATA-002    | DATA    | CLOUD-GATE                           | docs/01_modules/ENT/A-ENT-M-T-01_订阅门控技术设计.md      | registered      | src-tauri/src/ent/usage.rs::file-level                                                                                                                                                                                                                                                                                                                                      |
| BR-KB-DATA-001     | DATA    | KB-RETRIEVE                          | docs/01_modules/KB/A-KB-M-T-01_知识库技术设计.md         | registered      | src-tauri/src/kb/store.rs::file-level                                                                                                                                                                                                                                                                                                                                       |
| BR-KB-DATA-002     | DATA    | KB-RETRIEVE                          | docs/01_modules/KB/A-KB-M-T-01_知识库技术设计.md         | registered      | src-tauri/src/kb/retrieve.rs::file-level                                                                                                                                                                                                                                                                                                                                    |
| BR-PRV-DATA-002    | DATA    | PRV-VIEW                             | docs/01_modules/PRV/A-PRV-M-T-01_预览输出技术设计.md      | registered      | src-tauri/src/doc/table_preview.rs::file-level, src-tauri/src/prv/mod.rs::file-level, src/services/prvRuntime.test.ts::file-level                                                                                                                                                                                                                                           |
| BR-PRV-DATA-001    | DATA    | PRV-VIEW                             | docs/01_modules/PRV/A-PRV-M-T-01_预览输出技术设计.md      | registered      | src/services/prvRuntime.test.ts::file-level                                                                                                                                                                                                                                                                                                                                 |
| BR-REF-DATA-001    | DATA    | REF-INJECT                           | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md        | registered      | src-tauri/src/reference/resolve.rs::file-level, src-tauri/src/reference/routes.rs::file-level                                                                                                                                                                                                                                                                               |
| BR-REF-DATA-002    | DATA    | REF-INJECT                           | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md        | registered      | src-tauri/src/reference/inject.rs::file-level, src-tauri/src/reference/routes.rs::file-level                                                                                                                                                                                                                                                                                |
| BR-REF-DATA-003    | DATA    | REF-INJECT                           | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md        | registered      | src-tauri/src/ag/reference_runtime.rs::file-level, src-tauri/src/reference/context.rs::file-level, src/components/ag/AgentUI.test.tsx::file-level, src/services/referencePayload.test.ts::file-level                                                                                                                                                                        |
| BR-REF-DATA-004    | DATA    | REF-INJECT                           | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md        | registered      | src/components/ag/AgentUI.test.tsx::file-level, src/services/referencePayload.test.ts::file-level                                                                                                                                                                                                                                                                           |
| BR-REF-VERIFY-001  | VERIFY  | REF-INJECT                           | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md        | registered      | src-tauri/src/reference/context.rs::file-level, src-tauri/src/reference/inject.rs::file-level, src-tauri/src/reference/routes.rs::file-level                                                                                                                                                                                                                                |
| BR-SYS-STATE-007   | STATE   | AG-TURN, IPC-CALL                    | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md        | registered      | src-tauri/src/ag/capability_runtime.rs::file-level, src-tauri/src/ag/tool_exec.rs::file-level, src-tauri/src/ag/turn.rs::file-level, src-tauri/src/ag/turn_control.rs::file-level, src-tauri/src/ag/turn_events.rs::file-level, src-tauri/src/ag/turn_runtime.rs::file-level, src/components/ag/AgentUI.test.tsx::file-level, src/services/agRuntime.test.ts::file-level    |
| BR-SYS-STATE-008   | STATE   | AG-TURN, DE-DIFF                     | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md        | registered      | src-tauri/src/ag/task.rs::file-level, src-tauri/src/ag/task_runtime.rs::file-level, src-tauri/src/ag/turn_control.rs::file-level, src/components/ag/AgentUI.test.tsx::file-level, src/services/agRuntime.test.ts::file-level, src/services/deRuntime.test.ts::file-level                                                                                                    |
| BR-SYS-DATA-007    | DATA    | REF-INJECT, AST-MEMORY               | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md        | registered      | src-tauri/src/ast/store.rs::file-level, src-tauri/src/reference/context.rs::file-level, src/services/referencePayload.test.ts::file-level                                                                                                                                                                                                                                   |
| BR-SYS-DATA-008    | DATA    | AG-TURN, AG-TOOL, ED-EDIT, DE-DIFF   | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md        | registered      | src-tauri/src/ag/tool_exec.rs::file-level, src-tauri/src/ag/turn_runtime.rs::file-level, src/services/agRuntime.test.ts::file-level                                                                                                                                                                                                                                         |
| BR-SYS-DATA-002    | DATA    | DATA-MIGRATE                         | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md        | registered      | src-tauri/src/types/mod.rs::file-level                                                                                                                                                                                                                                                                                                                                      |
| BR-SYS-PERSIST-001 | PERSIST | DATA-MIGRATE                         | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md        | registered      | src-tauri/src/ag/target_set.rs::file-level, src-tauri/src/db/mod.rs::file-level                                                                                                                                                                                                                                                                                             |
| BR-SYS-DATA-003    | DATA    | IPC-CALL                             | docs/01_modules/SYS/A-SYS-I-P-02_IPC协议.md         | registered      | src-tauri/src/ipc/mod.rs::file-level, src-tauri/src/ipc/system_tests.rs::file-level                                                                                                                                                                                                                                                                                         |
| BR-SYS-OBS-002     | OBS     | IPC-CALL                             | docs/01_modules/SYS/A-SYS-I-P-02_IPC协议.md         | registered      | src-tauri/src/ipc/mod.rs::file-level                                                                                                                                                                                                                                                                                                                                        |
| BR-SYS-STATE-002   | STATE   | IPC-CALL                             | docs/01_modules/SYS/A-SYS-I-P-02_IPC协议.md         | registered      | src-tauri/src/ipc/mod.rs::file-level                                                                                                                                                                                                                                                                                                                                        |
| BR-SYS-STATE-003   | STATE   | IPC-CALL                             | docs/01_modules/SYS/A-SYS-I-P-02_IPC协议.md         | registered      | src-tauri/src/sm_events.rs::file-level, src-tauri/tests/sm_broadcast.rs::file-level                                                                                                                                                                                                                                                                                         |
| BR-SYS-STATE-001   | STATE   | CLOUD-AUTH                           | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md         | registered      | src-tauri/src/session/mod.rs::file-level                                                                                                                                                                                                                                                                                                                                    |
| BR-SYS-VERIFY-001  | VERIFY  | CLOUD-GATE                           | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md         | registered      | src-tauri/src/session/mod.rs::file-level                                                                                                                                                                                                                                                                                                                                    |
| BR-SYS-DATA-001    | DATA    | CLOUD-INVOKE                         | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md         | registered      | src-tauri/src/session/mod.rs::file-level                                                                                                                                                                                                                                                                                                                                    |
| BR-SYS-DATA-004    | DATA    | CLOUD-INVOKE                         | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md         | registered      | MISSING                                                                                                                                                                                                                                                                                                                                                                     |
| BR-SYS-OBS-001     | OBS     | CLOUD-AUTH, CLOUD-GATE, CLOUD-INVOKE | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md         | registered      | src-tauri/src/ent/gateway.rs::file-level                                                                                                                                                                                                                                                                                                                                    |
| BR-SYS-DATA-006    | DATA    | AG-TOOL                              | docs/01_modules/SYS/A-SYS-I-P-04_BDocx代理编辑能力协议.md | registered      | src-tauri/src/ag/bdocx_capabilities.rs::file-level                                                                                                                                                                                                                                                                                                                          |
| BR-SYS-STATE-004   | STATE   | APP-CLOSE                            | docs/01_modules/SYS/A-SYS-M-T-02_应用关闭技术设计.md      | registered      | src/components/shell/ShellComponents.test.tsx::file-level                                                                                                                                                                                                                                                                                                                   |
| BR-SYS-STATE-006   | STATE   | APP-CLOSE                            | docs/01_modules/SYS/A-SYS-M-T-02_应用关闭技术设计.md      | registered      | src/services/systemRuntime.test.ts::file-level                                                                                                                                                                                                                                                                                                                              |
| BR-SYS-STATE-005   | STATE   | APP-CLOSE                            | docs/01_modules/SYS/A-SYS-M-T-02_应用关闭技术设计.md      | registered      | src-tauri/src/de/lifecycle.rs::file-level                                                                                                                                                                                                                                                                                                                                   |
| BR-SYS-DATA-005    | DATA    | APP-CLOSE                            | docs/01_modules/SYS/A-SYS-M-T-02_应用关闭技术设计.md      | registered      | src-tauri/src/db/mod.rs::file-level, src-tauri/src/de/lifecycle.rs::file-level                                                                                                                                                                                                                                                                                              |
| BR-TPL-DATA-001    | DATA    | TPL-FLOW                             | docs/01_modules/TPL/A-TPL-M-T-01_模板工作流技术设计.md     | registered      | src-tauri/src/tpl/template.rs::file-level                                                                                                                                                                                                                                                                                                                                   |
| BR-TPL-STATE-001   | STATE   | TPL-FLOW                             | docs/01_modules/TPL/A-TPL-M-T-01_模板工作流技术设计.md     | registered      | src-tauri/src/tpl/workflow.rs::file-level                                                                                                                                                                                                                                                                                                                                   |
| BR-WS-DATA-001     | DATA    | WS-FS                                | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md         | registered      | src-tauri/src/ipc/system_tests.rs::file-level, src-tauri/src/workspace/fileops.rs::file-level, src-tauri/src/workspace/listing.rs::file-level, src-tauri/src/workspace/mod.rs::file-level, src/services/workspaceRuntime.test.ts::file-level                                                                                                                                |
| BR-WS-OBS-001      | OBS     | WS-FS                                | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md         | registered      | src-tauri/src/ipc/system_tests.rs::file-level, src-tauri/src/workspace/routes.rs::file-level, src/services/edRuntime.test.ts::file-level, src/services/workspaceRuntime.test.ts::file-level                                                                                                                                                                                 |
| BR-WS-STATE-001    | STATE   | WS-DIFF-APPLY                        | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md         | registered      | src-tauri/src/workspace/diff_apply.rs::file-level, src/services/deRuntime.test.ts::file-level                                                                                                                                                                                                                                                                               |
| BR-WS-DATA-002     | DATA    | WS-TIMELINE                          | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md         | registered      | src-tauri/src/workspace/timeline.rs::file-level                                                                                                                                                                                                                                                                                                                             |
| BR-WS-STATE-002    | STATE   | WS-TIMELINE                          | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md         | registered      | src-tauri/src/workspace/timeline.rs::file-level                                                                                                                                                                                                                                                                                                                             |
| BR-WS-PERSIST-002  | PERSIST | DATA-MIGRATE                         | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md         | registered      | src-tauri/src/workspace/lifecycle.rs::file-level                                                                                                                                                                                                                                                                                                                            |
| BR-WS-PERSIST-001  | PERSIST | WS-SAVE                              | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md         | registered      | src-tauri/src/ipc/system_tests.rs::file-level, src-tauri/src/workspace/save.rs::file-level                                                                                                                                                                                                                                                                                  |
| BR-WS-STATE-003    | STATE   | WS-OPERATION                         | docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md     | registered      | src-tauri/src/ag/capability_runtime.rs::file-level, src-tauri/src/workspace/operation.rs::file-level, src-tauri/src/workspace/routes.rs::file-level, src/services/agRuntime.test.ts::file-level                                                                                                                                                                             |
| BR-WS-DATA-003     | DATA    | WS-OPERATION, WS-TIMELINE            | docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md     | registered      | src-tauri/src/workspace/operation.rs::file-level, src-tauri/src/workspace/routes.rs::file-level, src/services/agRuntime.test.ts::file-level                                                                                                                                                                                                                                 |
| BR-WS-OBS-002      | OBS     | WS-OPERATION, AG-TURN                | docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md     | registered      | src-tauri/src/workspace/operation.rs::file-level, src-tauri/src/workspace/routes.rs::file-level, src/services/agRuntime.test.ts::file-level                                                                                                                                                                                                                                 |
| BR-WS-PERSIST-003  | PERSIST | WS-OPERATION, WS-TIMELINE            | docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md     | registered      | src-tauri/src/workspace/operation.rs::file-level, src/services/agRuntime.test.ts::file-level                                                                                                                                                                                                                                                                                |
| BR-WS-STATE-009    | STATE   | WS-FS, WS-TIMELINE                   | docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md     | registered      | src-tauri/src/workspace/operation.rs::file-level                                                                                                                                                                                                                                                                                                                            |

### 跨模块约束表（X-INDEX）

| constraint_id | 摘要                                                                                                                                                                                                                                                                                                                                     | 涉及模块                                                                  | 链路影响                                                                                                                                  | 来源文档                                            |
| ------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- |
| X-FORBID-008  | 三态单向数据流——显示态不得直接或间接修改逻辑态持有者（ProseMirror doc / edStore.content 等），逻辑态的修改必须经受控接口（X-FORBID-005）；已有 LogicalState 向 DiskState 的转换只能由保存动作触发。无 LogicalState 的 PendingDiff 仅可按 BR-CORE-STATE-002 在用户接受后经 WS-DIFF-APPLY 原子落盘                                                                                                                    | ALL（SYS, WS, ED, DE, AG, CBT, DOC, PRV, REF, AST, KB, TPL, ACCT, ENT） | IPC-CALL, ED-EDIT, ED-DECOR, DE-DIFF, WS-FS, WS-SAVE, WS-DIFF-APPLY, WS-TIMELINE, AG-TURN, AG-ASSIST, AG-TOOL, CBT-BUILD, DOC-CONVERT | docs/00_core/A-CORE-C-T-01_三态模型.md              |
| X-FORBID-009  | AI 生成内容默认属于"逻辑态候选（LogicalCandidate）"；存在 LogicalState 时候选须经 DE/ED 受控预览或打开迁移进入 LogicalState，显式接受或对仍有效 pending 卡保存都会裁决；不存在 LogicalState 时候选保持 DiffCard 挂载，只有用户接受才可经 WS-DIFF-APPLY 写入 DiskState。AI 产出不得绕过三分流受控路径直写逻辑态持有者或磁盘                                                                                                                | DE, AG, ED, CBT                                                       | DE-DIFF, AG-TURN, AG-ASSIST, ED-EDIT, CBT-BUILD                                                                                       | docs/00_core/A-CORE-C-T-01_三态模型.md              |
| X-FORBID-006  | AG 及其工具调用不得绕过两条唯一写路径：文档内容修改必须经 SM-AG-02 propose_edit→DE 内容 Diff 裁决→WS 保存/落盘；工作区资源操作必须经 WorkspaceOperationPlan→DecisionRequest 前置裁决→WS 重验并直接修改 DiskState。资源操作不得生成 DiffCard，内容 Diff 不得创建、删除、重命名或移动资源；CreateDocumentBundle 只在资源创建完成后把初始内容接入普通 Diff                                                                                        | AG, DE, ED, WS, REF, SYS, CBT                                         | AG-TURN, AG-TOOL, WS-DIFF-APPLY, WS-OPERATION                                                                                         | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md        |
| X-SEQ-004     | docx 等二进制文档格式的解析/生成必须经 DOC 转换管线，任何模块不得内置二进制文档转换库或另立转换路径                                                                                                                                                                                                                                                                                | DOC, ED, WS, PRV, CBT, AG                                             | DOC-CONVERT, ED-EDIT                                                                                                                  | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md    |
| X-FORBID-005  | 上层模块（DE/AG/REF 等）不得绕过 ED 装饰层与编辑接口直接操纵编辑器内部文档状态；编辑器文档模型的修改必经 ED 受控接口                                                                                                                                                                                                                                                                    | ED, DE, AG, REF                                                       | ED-EDIT, ED-DECOR                                                                                                                     | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md       |
| X-SEQ-002     | 前端不得跨层直连文件系统/数据库/网络，必经 Service → Tauri command → Rust 后端                                                                                                                                                                                                                                                                               | SYS, WS, ED, DE, AG, CBT, DOC, PRV, REF, AST, KB, TPL, ACCT, ENT      | CLOUD-AUTH, CLOUD-GATE, CLOUD-INVOKE                                                                                                  | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md      |
| X-FORBID-003  | 模块依赖必须无环；SYS 为架构底座不得反向依赖任何业务模块                                                                                                                                                                                                                                                                                                         | SYS, WS, ED, DE, AG, CBT, DOC, PRV, REF, AST, KB, TPL, ACCT, ENT      | CLOUD-AUTH, CLOUD-GATE, CLOUD-INVOKE                                                                                                  | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md      |
| X-FORBID-007  | 前端 Store 不得另立权威业务状态，须投影后端状态机（SM-*）与控制态（workspace.db）；界面流转由后端状态决定，前端只呈现与转达意图，不得本地维护与后端可漂移的并行业务状态                                                                                                                                                                                                                                        | SYS, WS, ED, DE, AG, CBT, DOC, AST, KB, TPL, REF, PRV, ACCT, ENT      | ED-EDIT, AG-TURN, DE-DIFF, WS-FS, WS-SAVE, CBT-BUILD                                                                                  | docs/01_modules/SYS/A-SYS-C-T-02_前端交互架构与流程总图.md |
| X-SHARED-001  | 跨模块传递的实体/类型/枚举集合必须经 SYS 数据结构契约层（本文）定义，模块不得私自定义跨模块共享结构                                                                                                                                                                                                                                                                                  | SYS, WS, ED, DE, AG, CBT, DOC, PRV, REF, AST, KB, TPL, ACCT, ENT      | DATA-MIGRATE                                                                                                                          | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md      |
| X-SHARED-002  | Agent 1.0 共享运行时对象（RuntimeProfile/AgentSession/AgentTurn/IntentResult/CapabilityGrant/TaskRun/TaskPlan/TaskStep/TurnEvent/EvidenceLedger/ExternalContentRef/MemorySelectionRef/AgentContextFacts/ProjectReport）的职责、身份、事件真值、隔离与迁移边界以 SYS 数据结构契约层（A-SYS-I-P-01 §3.1）为唯一契约来源；任何模块不得以旧设计、现有代码、已注册工具、Prompt、前端参数、兼容默认值或测试绿灯反向改写冻结需求语义或隐式扩大授权 | SYS, AG, DE, ED, WS, REF, AST                                         | AG-TURN, DE-DIFF, ED-EDIT, WS-SAVE, WS-DIFF-APPLY, REF-INJECT, AST-MEMORY, IPC-CALL                                                   | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md      |
| X-FORBID-001  | 客户端不得持有或落地模型 provider 密钥，密钥仅服务端（AiGateway）持有                                                                                                                                                                                                                                                                                           | SYS, ENT, ACCT, AG, CBT, AST, KB, TPL                                 | CLOUD-INVOKE, CLOUD-GATE                                                                                                              | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md       |
| X-SEQ-001     | 任何模型调用前必须先过订阅门禁硬门（CLOUD-GATE），无有效订阅一律拒绝，禁止跳过门禁直连网关                                                                                                                                                                                                                                                                                     | ENT, AG, CBT, AST, KB, TPL                                            | CLOUD-GATE, CLOUD-INVOKE                                                                                                              | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md       |
| X-FORBID-002  | 订阅门禁不得存在客户端绕过路径，裁决以服务端为准，客户端不得本地缓存通过结论跳过校验                                                                                                                                                                                                                                                                                             | ENT, SYS                                                              | CLOUD-GATE                                                                                                                            | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md       |
| X-SEQ-003     | 状态化核心链（APC §6 强制阈值）必须先建状态机并在总图登记 machine_id 方可进入实现，未注册者阻断进入代码                                                                                                                                                                                                                                                                          | SYS, WS, ED, DE, AG, CBT, DOC, AST                                    | CLOUD-AUTH, CLOUD-GATE, CLOUD-INVOKE                                                                                                  | docs/01_modules/SYS/A-SYS-M-S-01_状态机总图.md       |
| X-INTER-001   | 跨模块状态转移必须在总图登记转移契约，转移两侧 owner 状态机对接点须一致，不得各自假设状态/事件                                                                                                                                                                                                                                                                                    | SYS, WS, ED, DE, AG, CBT                                              | CLOUD-GATE, CLOUD-INVOKE                                                                                                              | docs/01_modules/SYS/A-SYS-M-S-01_状态机总图.md       |
| X-FORBID-010  | 应用正常退出（非崩溃）完成前，当前 workspace 不得残留 status=pending 的 diff、effect_status=awaiting_save 的效果或 dirty=1 的未保存文档；不满足时必须阻断退出并呈现统一决策入口，不得静默继续退出或静默丢弃                                                                                                                                                                                               | SYS, DE, ED, WS, AG                                                   | APP-CLOSE, DE-DIFF, ED-EDIT, WS-SAVE, AG-TURN                                                                                         | docs/01_modules/SYS/A-SYS-M-T-02_应用关闭技术设计.md    |
| X-FORBID-011  | 应用启动时，任何跨会话残留的 pending diff、awaiting_save 效果或未保存 LogicalState 快照不得被当作可信权威态直接恢复为可裁决/可编辑态；必须先经异常退出清算转终态/丢弃，只能以只读历史形态呈现                                                                                                                                                                                                                   | SYS, DE, ED, WS                                                       | APP-CLOSE, DE-DIFF, ED-EDIT, DATA-MIGRATE                                                                                             | docs/01_modules/SYS/A-SYS-M-T-02_应用关闭技术设计.md    |
| X-FORBID-004  | 文件操作/搜索/diff/上下文读取的目标路径不得越出活动 workspace root；任一越界路径一律拒绝，不静默截断或回退到 workspace 外                                                                                                                                                                                                                                                          | WS, ED, DE, AG, CBT, REF, AST, KB, TPL, DOC, PRV                      | WS-FS, WS-SAVE, WS-DIFF-APPLY, WS-TIMELINE                                                                                            | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md       |

### 术语注册表

| term_id        | 所属链路                                                 | 正式中文名     | 正式英文名                   | 禁用别名                                                 | 来源文档                                              |
| -------------- | ---------------------------------------------------- | --------- | ----------------------- | ---------------------------------------------------- | ------------------------------------------------- |
| TERM-PCG-001   | PCG-GENERATE, PCG-AUDIT                              | 产品能力项     | ProductCapability       | 功能点, feature点, 产品功能块                                 | docs/00_core/A-CORE-C-P-02_产品能力图谱规范.md            |
| TERM-PCG-002   | PCG-GENERATE, PCG-AUDIT                              | 能力变体      | CapabilityVariant       | 功能组合, 功能分支, capability组合                             | docs/00_core/A-CORE-C-P-02_产品能力图谱规范.md            |
| TERM-PCG-003   | PCG-GENERATE                                         | 产品能力沙盘    | ProductCapabilityAtlas  | 功能总表, 产品功能图, 特性大盘                                    | docs/00_core/A-CORE-C-P-02_产品能力图谱规范.md            |
| TERM-PCG-004   | PCG-AUDIT                                            | 正式主链      | CanonicalChain          | 唯一路径, 固定链路, 主流程代码                                    | docs/00_core/A-CORE-C-P-02_产品能力图谱规范.md            |
| TERM-CORE-001  | IPC-CALL, ED-EDIT, ED-DECOR, DE-DIFF, WS-FS, WS-SAVE | 磁盘态       | DiskState               | 文件内容, 原始内容, 磁盘内容, 字节态, 物理态                           | docs/00_core/A-CORE-C-T-01_三态模型.md                |
| TERM-CORE-002  | IPC-CALL, ED-EDIT, ED-DECOR, DE-DIFF, WS-FS, WS-SAVE | 逻辑态       | LogicalState            | 编辑状态, 内存状态, 缓存状态, 运行态, 编辑缓存, 草稿内容                    | docs/00_core/A-CORE-C-T-01_三态模型.md                |
| TERM-CORE-003  | IPC-CALL, ED-DECOR, ED-EDIT                          | 显示态       | DisplayState            | 渲染态, 呈现层, 装饰态, 可视化状态, 界面态, 屏幕态                       | docs/00_core/A-CORE-C-T-01_三态模型.md                |
| TERM-CORE-004  | DE-DIFF, AG-TURN, AG-ASSIST, CBT-BUILD               | 逻辑态候选     | LogicalCandidate        | AI建议, 修改提议, 待定编辑, 提案态, 候选内容, 草案内容                    | docs/00_core/A-CORE-C-T-01_三态模型.md                |
| TERM-AG-001    | AG-TURN                                              | 代理回合      | AgentTurn               | 对话轮, 轮次, 会话轮, round, 聊天回合                            | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-002    | AG-TOOL                                              | 工具调用      | ToolCall                | 函数调用, function call, 工具执行, 能力调用                      | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-003    | AG-ASSIST                                            | 局部修改      | InlineAssist            | 内联辅助, 行内辅助, 内联建议, inline建议, 行内AI                     | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-004    | AG-ASSIST                                            | 辅助续写      | Autocomplete            | 自动补全, 自动完成, 补全建议, 智能补全, 续写补全                         | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-006    | AG-TURN                                              | 编辑模式      | EditMode                | 对话模式, chat模式, chat 模式, 对话编辑模式                        | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-007    | AG-TURN                                              | 代理构建      | BuildMode               | 构建模式, agent模式, agent 模式, Chat Build, chat-build      | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-008    | AG-TURN                                              | 代理编辑      | AgentEdit               | 对话编辑, 对话编辑板块                                         | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-009    | AG-TURN                                              | 代理编辑会话    | AgentSession            | 聊天标签, 对话标签, 会话标签, agent标签, chat tab, session tab     | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-010    | AG-MODEL-CATALOG, AG-ASSIST, AG-TURN                 | 预设模型      | PresetModel             | 内置模型, 托管模型, 官方模型                                     | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-011    | AG-TURN                                              | 消息片段      | MessageSegment          | 消息块, 消息段, 文本片段, 卡片锚点, segment                        | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-012    | AG-TURN, AG-TOOL                                     | 本轮意图结果    | IntentResult            | 意图布尔值, 编辑意图标记, intent flag, 意图关键词结果                  | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-013    | AG-TURN                                              | 运行时档位     | RuntimeProfile          | 运行时配置, 能力档位, runtime 配置, profile 档位, 代理档位, 运行配置      | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-014    | AG-TURN, AG-TOOL                                     | 当轮能力授权    | CapabilityGrant         | 工具授权, 能力授权, 权限授予, grant, 当轮授权, 工具许可                  | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-015    | AG-TURN                                              | 受限任务实例    | TaskRun                 | 任务实例, 后台任务, 任务队列, 子任务集合, task 实例, 复合任务对象             | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-016    | AG-TURN                                              | 任务步骤      | TaskStep                | 任务步骤, 步骤, 子步骤, step 单元, 执行步                          | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-017    | AG-TURN                                              | 任务级裁决请求   | DecisionRequest         | 确认卡, 裁决项, decision 实例, 确认请求                          | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-018    | AG-TURN                                              | 任务计划      | TaskPlan                | 计划, 任务计划清单, plan 单元, 执行计划                            | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-019    | AG-TURN, AG-TOOL                                     | 显式编辑目标集   | ExplicitEditTargetSet   | 编辑白名单, 目标白名单, 跨文档白名单, 隐式目标集, 模型目标集                   | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-020    | AG-TURN                                              | 代理上下文事实集  | AgentContextFacts       | 上下文状态, 编辑器上下文, 项目上下文, AgentContext, context facts    | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-021    | AG-TURN, AG-TOOL                                     | 项目报告      | ProjectReport           | 项目总结, 项目扫描报告, workspace 报告, 分析总结, 普通回答               | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-022    | AG-TURN, AG-TOOL                                     | 回合事件      | TurnEvent               | 运行日志, 状态日志, 事件记录, event log, 回合日志                    | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-023    | AG-TURN, AG-TOOL                                     | 证据账本      | EvidenceLedger          | 读取日志, 证据列表, 阅读记录, evidence log, 工具证据                 | docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md          |
| TERM-AG-005    | AG-TURN                                              | 意图沿用判定    | IntentCarryover         | 历史复用, 意图继承, 上下文继承, 意图沿袭, carryover                   | docs/01_modules/AG/A-AG-M-T-02_提示词架构设计.md         |
| TERM-AST-001   | AST-MEMORY                                           | 记忆条目      | MemoryEntry             | 记忆点, 记忆项, 记忆片段, memory条目                             | docs/01_modules/AST/A-AST-M-T-01_记忆协同技术设计.md      |
| TERM-AST-002   | REF-INJECT, AST-MEMORY                               | 显式项目记忆引用  | MemorySelectionRef      | 记忆引用, 记忆选择, 记忆注入引用, 记忆条目引用, 记忆选中项                    | docs/01_modules/AST/A-AST-M-T-01_记忆协同技术设计.md      |
| TERM-CBT-001   | CBT-BUILD                                            | 构建回合      | BuildRun                | 构建会话, 构建过程, build流程, 项目构建轮                           | docs/01_modules/CBT/A-CBT-M-T-01_构建技术设计.md        |
| TERM-CBT-002   | CBT-BUILD                                            | 构建任务      | BuildTask               | 构建步骤, 子任务, build单元, 任务项                              | docs/01_modules/CBT/A-CBT-M-T-01_构建技术设计.md        |
| TERM-CBT-003   | CBT-BUILD                                            | 项目蓝图      | BuildBlueprint          | 项目大纲, 构建大纲, 需求蓝图, blueprint                          | docs/01_modules/CBT/A-CBT-M-T-01_构建技术设计.md        |
| TERM-DE-001    | DE-DIFF                                              | 待裁决修改     | PendingDiff             | 修改提案, 变更请求, 待定diff, 修改单, 待审修改                        | docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md    |
| TERM-DE-002    | DE-DIFF                                              | 修改建议      | ChangeProposal          | AI建议, 修改意见, 编辑建议, 产出建议                               | docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md    |
| TERM-DE-004    | DE-DIFF                                              | 修改载荷      | DiffPayload             | diff载荷, 内容载荷, payload, diff内容, 落盘载荷, 提案载荷            | docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md    |
| TERM-DE-005    | DE-DIFF                                              | 提案批次      | ProposalBatch           | diff批次, 批量diff, 修改批次, 提案组, batch diff                | docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md    |
| TERM-DOC-001   | DOC-CONVERT                                          | 样式子集      | StyleSubset             | 样式白名单, 格式子集, 样式范围, 支持样式表                             | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md      |
| TERM-DOC-002   | DOC-CONVERT                                          | 编辑草稿      | EditDraft               | 草稿副本, draft文件, 临时编辑文件, 编辑副本                          | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md      |
| TERM-DOC-003   | DOC-CONVERT, ED-EDIT                                 | b-docx    | BDocx                   | 模拟docx, HTML模拟docx, 可编辑docx, 项目docx, 生成docx, docx编辑稿 | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md      |
| TERM-DOC-004   | DOC-CONVERT                                          | 字体身份修复    | FontIdentityFix         | 字体fallback修复, 字形回退修复, 字体替换修复                         | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md      |
| TERM-DOC-005   | DOC-CONVERT, ED-EDIT, DE-DIFF                        | 单元格底纹     | TableCellShading        | 单元格背景, 表格底色, 单元格填色, cell background                  | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md      |
| TERM-ED-001    | ED-EDIT                                              | 内容块       | Block                   | 内容节点, 段落块, 文本块, 块节点, chunk                           | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         |
| TERM-ED-002    | ED-DECOR                                             | 编辑器装饰层    | EditorDecoration        | 高亮层, 标注层, 覆盖层, overlay层, 装饰器层                        | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         |
| TERM-ED-004    | ED-EDIT                                              | 字符锚       | CharAnchor              | 字符偏移锚, 文本锚, 偏移量锚, inline锚, 字符定位符                     | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         |
| TERM-ED-005    | ED-EDIT                                              | 选区引用      | SelectionRef            | 选区信息, 拖拽引用, selection数据, 选区数据                        | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         |
| TERM-ED-006    | ED-EDIT                                              | 样式操作      | StyleOp                 | 样式指令, 格式操作, style命令, 样式patch                         | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         |
| TERM-ED-007    | ED-EDIT, AG-TOOL, DE-DIFF                            | 表格操作      | TableOperation          | 表格指令, 表格补丁, table命令                                  | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         |
| TERM-ED-008    | ED-EDIT, AG-TOOL, DE-DIFF, DOC-CONVERT               | 图片操作      | ImageOperation          | 图片指令, 图片补丁, image命令                                  | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md         |
| TERM-KB-001    | KB-RETRIEVE                                          | 知识条目      | KnowledgeEntry          | 知识点, 资料条目, 知识片段, kb条目                                | docs/01_modules/KB/A-KB-M-T-01_知识库技术设计.md         |
| TERM-REF-001   | REF-INJECT                                           | 引用锚       | ReferenceAnchor         | 引用链接, 引用指针, mention锚, 引用定位符                          | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md        |
| TERM-REF-002   | REF-INJECT                                           | 注入载荷      | InjectionPayload        | 上下文载荷, 注入内容包, 引用载荷, 引用内容体                            | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md        |
| TERM-REF-003   | REF-INJECT                                           | 引用输入载荷    | ReferencePayload        | 引用内容包, 引用输入包, ref payload, mention payload           | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md        |
| TERM-REF-004   | REF-INJECT                                           | 引用令牌      | ReferenceToken          | 引用标签, 输入标签, inline ref chip, 引用片                     | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md        |
| TERM-REF-005   | REF-INJECT, AG-TURN                                  | 外部内容引用    | ExternalContentRef      | 外部文件引用, 外部读取引用, 外部内容令牌, 外部分片引用, 附加文件引用               | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md        |
| TERM-SYS-001   | IPC-CALL                                             | 设计令牌      | DesignToken             | CSS变量, 设计变量, 视觉令牌, token系统, 视觉常量, design token       | docs/01_modules/SYS/A-SYS-C-T-02_前端交互架构与流程总图.md   |
| TERM-DE-003    | DE-DIFF                                              | 修改卡       | DiffCard                | diff卡片, AI建议卡, 修改面板, 修改组件, 变更卡, 审阅卡                  | docs/01_modules/SYS/A-SYS-C-T-02_前端交互架构与流程总图.md   |
| TERM-ED-003    | ED-DECOR                                             | 绿色审阅层     | GreenReview             | 绿色高亮, 编辑器diff层, diff装饰, 正文高亮层, 修改高亮, green review    | docs/01_modules/SYS/A-SYS-C-T-02_前端交互架构与流程总图.md   |
| TERM-CLOUD-001 | CLOUD-GATE                                           | 订阅门禁      | SubscriptionGate        | 订阅校验, 会员校验, 付费门槛, 鉴权门                                | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md         |
| TERM-CLOUD-002 | CLOUD-INVOKE                                         | AI网关      | AiGateway               | 模型代理, API网关, 代理服务, provider代理                        | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md         |
| TERM-CLOUD-003 | CLOUD-AUTH                                           | 会话令牌      | SessionToken            | 登录态, token, 认证令牌, 鉴权码                                | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md         |
| TERM-CLOUD-004 | CLOUD-INVOKE                                         | BYOK透传    | ByokPassthrough         | 自带密钥, 自定义key, 用户密钥直连                                 | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md         |
| TERM-SYS-002   | AG-TURN, AG-TOOL, ED-EDIT, DE-DIFF, DOC-CONVERT      | BDocx能力清单 | BDocxCapabilityManifest | BDocx能力表, 代理能力表, 编辑能力列表, 能力白名单                       | docs/01_modules/SYS/A-SYS-I-P-04_BDocx代理编辑能力协议.md |
| TERM-TPL-001   | TPL-FLOW                                             | 写作模板      | Template                | 文档模板片, 提示模板项, 样板, template条目                         | docs/01_modules/TPL/A-TPL-M-T-01_模板工作流技术设计.md     |
| TERM-TPL-002   | TPL-FLOW                                             | 写作工作流     | WritingWorkflow         | 流程, 工作流程, pipeline, 写作流程图                            | docs/01_modules/TPL/A-TPL-M-T-01_模板工作流技术设计.md     |
| TERM-WS-001    | WS-FS                                                | 工作区       | Workspace               | 工作空间, 项目根目录, 工作目录, root目录, 项目目录                      | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md         |
| TERM-WS-002    | WS-TIMELINE                                          | 文档时间轴     | DocumentTimeline        | 历史记录, 版本历史, 历史栈, 版本栈, 历史链                            | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md         |
| TERM-WS-003    | WS-OPERATION, AG-TURN                                | 工作区资源操作   | WorkspaceOperation      | 文件整理,文件操作diff,资源diff,结构diff,文件树diff                  | docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md     |
| TERM-WS-004    | WS-OPERATION, AG-TURN                                | 工作区资源操作计划 | WorkspaceOperationPlan  | 删除diff,操作草稿,文件修改建议,结构Proposal                        | docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md     |
| TERM-WS-005    | WS-TIMELINE, WS-OPERATION                            | 工作区时间轴    | WorkspaceTimeline       | 回收站,撤销栈,文件历史栈,项目版本栈                                  | docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md     |
| TERM-WS-006    | WS-TIMELINE, WS-OPERATION                            | 工作区资源操作条目 | WorkspaceOperationEntry | 删除节点,文件diff节点,操作日志行                                  | docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md     |
| TERM-WS-007    | WS-OPERATION, DE-DIFF, AG-TURN                       | 新建文档绑定    | CreateDocumentBundle    | 新建diff,空文件diff,创建提案                                  | docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md     |

### 状态机注册表

| machine_id | owner_module | status | source_type | chain                                | rules                                                                                                                     | initial_state    | terminal_states                                                | 来源文档                                            |
| ---------- | ------------ | ------ | ----------- | ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------- | ---------------- | -------------------------------------------------------------- | ----------------------------------------------- |
| SM-AG-01   | AG           | active | design      | AG-TURN                              | BR-AG-STATE-001, BR-AG-VERIFY-001, BR-AG-DATA-001, BR-AG-DATA-002, BR-AG-DATA-004, BR-AG-DATA-005, BR-AG-PERSIST-001      | Idle             | Completed, Cancelled, Denied, TurnError                        | docs/01_modules/AG/A-AG-M-S-01_代理状态机设计.md       |
| SM-AG-04   | AG           | active | design      | AG-TURN                              | BR-SYS-STATE-008, BR-AG-STATE-001, BR-AG-STATE-002                                                                        | Projected        | Completed, Failed, Cancelled, Interrupted                      | docs/01_modules/AG/A-AG-M-S-01_代理状态机设计.md       |
| SM-AG-02   | AG           | active | design      | AG-TOOL                              | BR-AG-STATE-002, BR-AG-VERIFY-001                                                                                         | ModelStreaming   | LoopCompleted, LoopAborted                                     | docs/01_modules/AG/A-AG-M-S-01_代理状态机设计.md       |
| SM-AG-03   | AG           | active | design      | AG-ASSIST                            | BR-AG-STATE-003, BR-AG-VERIFY-001                                                                                         | Idle             | Accepted, Dismissed                                            | docs/01_modules/AG/A-AG-M-S-01_代理状态机设计.md       |
| SM-AST-01  | AST          | active | design      | AST-MEMORY                           | BR-AST-STATE-001, BR-AST-DATA-001                                                                                         | Capturing        | Written, Revoked, Discarded                                    | docs/01_modules/AST/A-AST-M-S-01_记忆协同状态机设计.md   |
| SM-CBT-01  | CBT          | active | design      | CBT-BUILD                            | BR-CBT-STATE-001, BR-CBT-STATE-002, BR-CBT-DATA-001, BR-CBT-PERSIST-001                                                   | Idle             | Completed, Interrupted                                         | docs/01_modules/CBT/A-CBT-M-S-01_构建状态机设计.md     |
| SM-DE-01   | DE           | active | design      | DE-DIFF                              | BR-DE-DATA-001, BR-DE-STATE-001, BR-DE-STATE-002, BR-DE-DATA-003, BR-CORE-STATE-001, BR-CORE-STATE-002, BR-CORE-STATE-003 | Generating       | Accepted, Rejected, Expired, GenFailed                         | docs/01_modules/DE/A-DE-M-S-01_DiffEdit状态机设计.md |
| SM-DOC-01  | DOC          | active | design      | DOC-CONVERT                          | BR-DOC-STATE-001, BR-DOC-DATA-001, BR-DOC-DATA-003                                                                        | Idle             | Idle                                                           | docs/01_modules/DOC/A-DOC-M-S-01_文档转换状态机设计.md   |
| SM-ED-01   | ED           | active | design      | ED-EDIT                              | BR-ED-STATE-001, BR-ED-DATA-001, BR-ED-DATA-002, BR-ED-DATA-006, BR-ED-VERIFY-001, BR-CORE-STATE-001, BR-CORE-STATE-003   | Closed           | Closed                                                         | docs/01_modules/ED/A-ED-M-S-01_编辑器状态机设计.md      |
| SM-SYS-01  | SYS          | active | design      | CLOUD-AUTH, CLOUD-GATE, CLOUD-INVOKE | BR-SYS-STATE-001, BR-SYS-VERIFY-001, BR-SYS-DATA-001                                                                      | Unauthenticated  | GateDenied, InvokeCompleted, InvokeError                       | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md       |
| SM-SYS-02  | SYS          | active | design      | APP-CLOSE                            | BR-SYS-STATE-004, BR-SYS-STATE-005, BR-SYS-STATE-006, BR-SYS-DATA-005                                                     | Idle             | Closed                                                         | docs/01_modules/SYS/A-SYS-M-S-02_应用关闭状态机设计.md   |
| SM-TPL-01  | TPL          | active | design      | TPL-FLOW                             | BR-TPL-STATE-001, BR-TPL-DATA-001                                                                                         | Idle             | Completed, Aborted                                             | docs/01_modules/TPL/A-TPL-M-S-01_模板工作流状态机设计.md  |
| SM-WS-01   | WS           | active | design      | WS-SAVE                              | BR-WS-PERSIST-001, BR-WS-DATA-001                                                                                         | Idle             | Saved, SaveError                                               | docs/01_modules/WS/A-WS-M-S-01_工作台状态机设计.md      |
| SM-WS-02   | WS           | active | design      | WS-DIFF-APPLY                        | BR-WS-STATE-001, BR-WS-DATA-002, BR-CORE-STATE-002                                                                        | AwaitingDecision | Applied, Discarded, ApplyError                                 | docs/01_modules/WS/A-WS-M-S-01_工作台状态机设计.md      |
| SM-WS-03   | WS           | active | design      | WS-TIMELINE                          | BR-WS-STATE-002, BR-WS-DATA-002                                                                                           | Browsing         | Restored, RestoreError                                         | docs/01_modules/WS/A-WS-M-S-01_工作台状态机设计.md      |
| SM-WS-04   | WS           | active | design      | WS-OPERATION                         | BR-WS-STATE-003, BR-WS-DATA-003, BR-WS-OBS-002, BR-WS-PERSIST-003                                                         | Planning         | Completed, Rejected, Expired, FailedCompensated, PartialFailed | docs/01_modules/WS/A-WS-M-S-02_工作区资源操作状态机设计.md  |

### 链路视图

#### AG-ASSIST

| codes                                                                             | type   | rules                                              | 文件                                        | 行号   |
| --------------------------------------------------------------------------------- | ------ | -------------------------------------------------- | ----------------------------------------- | ---- |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-009                                               | RB     | BR-AG-STATE-003 term_ref: TERM-AG-004              | src/components/ag/AutocompleteOverlay.tsx | 16   |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-007                                               | RB     | BR-AG-STATE-003 term_ref: TERM-AG-003, TERM-AG-004 | src/components/ed/Editor.tsx              | 810  |
| BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-003                                           | EFFECT | BR-AG-STATE-003 term_ref: TERM-AG-004              | src/components/ed/Editor.tsx              | 1008 |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-002                                               | RB     | BR-AG-STATE-003 term_ref: TERM-AG-003, TERM-AG-004 | src/components/ed/Editor.tsx              | 1938 |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-004                                               | RB     | BR-AG-STATE-003 term_ref: TERM-AG-003, TERM-AG-004 | src/components/ed/assistPlacement.ts      | 36   |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-005                                               | RB     | BR-AG-STATE-003 term_ref: TERM-AG-003, TERM-AG-004 | src/components/ed/assistPlacement.ts      | 60   |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-006                                               | RB     | BR-AG-STATE-003 term_ref: TERM-AG-003, TERM-AG-004 | src/components/ed/assistPlacement.ts      | 78   |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-003                                               | RB     | BR-AG-STATE-003 term_ref: TERM-AG-003, TERM-AG-004 | src/components/ed/assistPlacement.ts      | 96   |
| BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-002                                           | EFFECT | BR-AG-STATE-003 term_ref: TERM-AG-003, TERM-AG-004 | src/services/agService.ts                 | 2111 |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-008                                               | RB     | BR-AG-STATE-003 term_ref: TERM-AG-003, TERM-AG-004 | src/store/assistStore.ts                  | 94   |
| BR-AG-DATA-007-QUERY-AG-AG-MODEL-CATALOG-013                                      | QUERY  | BR-AG-DATA-007 term_ref: TERM-AG-010               | src/store/modelStore.ts                   | 161  |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-001                                               | RB     | BR-AG-STATE-003                                    | src-tauri/src/ag/assist.rs                | 40   |
| BR-AG-DATA-002-DATA-AG-AG-ASSIST-011                                              | DATA   | BR-AG-DATA-002                                     | src-tauri/src/ag/assist_prompt.rs         | 19   |
| BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-010                                           | EFFECT | BR-AG-STATE-003                                    | src-tauri/src/ag/assist_runtime.rs        | 50   |
| BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-001, BR-AG-VERIFY-001-EFFECT-AG-AG-ASSIST-001 | EFFECT | BR-AG-STATE-003, BR-AG-VERIFY-001                  | src-tauri/src/ag/assist_runtime.rs        | 92   |

#### AG-MODEL-CATALOG

| codes                                         | type   | rules                                | 文件                      | 行号  |
| --------------------------------------------- | ------ | ------------------------------------ | ----------------------- | --- |
| BR-AG-DATA-007-QUERY-AG-AG-MODEL-CATALOG-013  | QUERY  | BR-AG-DATA-007 term_ref: TERM-AG-010 | src/store/modelStore.ts | 161 |
| BR-AG-DATA-006-EFFECT-AG-AG-MODEL-CATALOG-006 | EFFECT | BR-AG-DATA-006 term_ref: TERM-AG-010 | src/store/modelStore.ts | 187 |

#### AG-TOOL

| codes                                                                                                                                                                                                                                                   | type   | rules                                                                                                                                                                                                                                            | 文件                                     | 行号   |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------- | ---- |
| BR-AG-DATA-008-DATA-AG-AG-TURN-054, BR-SYS-DATA-008-DATA-AG-AG-TURN-054                                                                                                                                                                                 | DATA   | BR-AG-DATA-008, BR-SYS-DATA-008 term_ref: TERM-AG-020                                                                                                                                                                                            | src/services/agService.ts              | 545  |
| BR-SYS-DATA-006-DATA-AG-AG-TOOL-001                                                                                                                                                                                                                     | DATA   | BR-SYS-DATA-006 term_ref: TERM-SYS-002                                                                                                                                                                                                           | src-tauri/src/ag/bdocx_capabilities.rs | 611  |
| BR-SYS-DATA-006-DATA-AG-AG-TOOL-002                                                                                                                                                                                                                     | DATA   | BR-SYS-DATA-006 term_ref: TERM-SYS-002, TERM-ED-008                                                                                                                                                                                              | src-tauri/src/ag/bdocx_capabilities.rs | 897  |
| BR-SYS-DATA-006-GUARD-AG-AG-TOOL-021, BR-ED-DATA-005-GUARD-AG-AG-TOOL-021                                                                                                                                                                               | GUARD  | BR-SYS-DATA-006, BR-ED-DATA-005 term_ref: TERM-SYS-002, TERM-ED-006, TERM-ED-004                                                                                                                                                                 | src-tauri/src/ag/bdocx_capabilities.rs | 1566 |
| BR-SYS-DATA-006-GUARD-AG-AG-TOOL-022, BR-ED-DATA-003-GUARD-AG-AG-TOOL-022                                                                                                                                                                               | GUARD  | BR-SYS-DATA-006, BR-ED-DATA-003 term_ref: TERM-SYS-002, TERM-ED-008                                                                                                                                                                              | src-tauri/src/ag/bdocx_capabilities.rs | 1918 |
| BR-AG-DATA-003-RB-AG-AG-TOOL-016, BR-ED-DATA-001-RB-AG-AG-TOOL-016, BR-DE-DATA-001-RB-AG-AG-TOOL-016                                                                                                                                                    | RB     | BR-AG-DATA-003, BR-ED-DATA-001, BR-DE-DATA-001 term_ref: TERM-ED-001                                                                                                                                                                             | src-tauri/src/ag/block_ops.rs          | 305  |
| BR-AG-DATA-001-GUARD-AG-AG-TOOL-044, BR-AG-DATA-008-GUARD-AG-AG-TOOL-044                                                                                                                                                                                | GUARD  | BR-AG-DATA-001, BR-AG-DATA-008 term_ref: TERM-AG-020                                                                                                                                                                                             | src-tauri/src/ag/blocks.rs             | 175  |
| BR-AG-DATA-001-QUERY-AG-AG-TOOL-011, BR-ED-DATA-003-QUERY-AG-AG-TOOL-011                                                                                                                                                                                | QUERY  | BR-AG-DATA-001, BR-ED-DATA-003 term_ref: TERM-ED-008, TERM-DOC-003                                                                                                                                                                               | src-tauri/src/ag/blocks.rs             | 307  |
| BR-AG-DATA-005-DATA-AG-AG-TOOL-058                                                                                                                                                                                                                      | DATA   | BR-AG-DATA-005                                                                                                                                                                                                                                   | src-tauri/src/ag/blocks.rs             | 620  |
| BR-AG-PERSIST-001-QUERY-AG-AG-TOOL-046, BR-SYS-STATE-007-QUERY-AG-AG-TOOL-046                                                                                                                                                                           | QUERY  | BR-AG-PERSIST-001, BR-SYS-STATE-007                                                                                                                                                                                                              | src-tauri/src/ag/capability_runtime.rs | 66   |
| BR-AG-STATE-002-EFFECT-AG-AG-TOOL-048, BR-SYS-STATE-007-EFFECT-AG-AG-TOOL-048, BR-AG-PERSIST-001-EFFECT-AG-AG-TOOL-048                                                                                                                                  | EFFECT | BR-AG-STATE-002, BR-SYS-STATE-007, BR-AG-PERSIST-001                                                                                                                                                                                             | src-tauri/src/ag/capability_runtime.rs | 239  |
| BR-AG-STATE-002-EFFECT-AG-AG-TOOL-046, BR-SYS-STATE-007-EFFECT-AG-AG-TOOL-046, BR-AG-PERSIST-001-EFFECT-AG-AG-TOOL-046                                                                                                                                  | EFFECT | BR-AG-STATE-002, BR-SYS-STATE-007, BR-AG-PERSIST-001                                                                                                                                                                                             | src-tauri/src/ag/capability_runtime.rs | 308  |
| BR-SYS-DATA-006-GUARD-AG-AG-TOOL-004, BR-ED-DATA-001-GUARD-AG-AG-TOOL-004, BR-ED-DATA-003-GUARD-AG-AG-TOOL-004                                                                                                                                          | GUARD  | BR-SYS-DATA-006, BR-ED-DATA-001, BR-ED-DATA-003 term_ref: TERM-SYS-002, TERM-ED-008                                                                                                                                                              | src-tauri/src/ag/edit_image_runtime.rs | 8    |
| BR-DE-DATA-002-DATA-AG-AG-TOOL-060, BR-DE-DATA-001-DATA-AG-AG-TOOL-060                                                                                                                                                                                  | DATA   | BR-DE-DATA-002, BR-DE-DATA-001                                                                                                                                                                                                                   | src-tauri/src/ag/edit_projection.rs    | 12   |
| BR-AG-STATE-002-EFFECT-AG-AG-TOOL-047, BR-AG-DATA-003-EFFECT-AG-AG-TOOL-047                                                                                                                                                                             | EFFECT | BR-AG-STATE-002, BR-AG-DATA-003                                                                                                                                                                                                                  | src-tauri/src/ag/edit_runtime.rs       | 18   |
| BR-AG-STATE-002-EFFECT-AG-AG-TOOL-017, BR-AG-DATA-003-EFFECT-AG-AG-TOOL-017, BR-DE-DATA-001-EFFECT-AG-AG-TOOL-017, BR-DE-DATA-002-EFFECT-AG-AG-TOOL-017                                                                                                 | EFFECT | BR-AG-STATE-002, BR-AG-DATA-003, BR-DE-DATA-001, BR-DE-DATA-002                                                                                                                                                                                  | src-tauri/src/ag/edit_runtime.rs       | 73   |
| BR-AG-STATE-002-GUARD-AG-AG-TOOL-058, BR-AG-DATA-009-GUARD-AG-AG-TOOL-058                                                                                                                                                                               | GUARD  | BR-AG-STATE-002, BR-AG-DATA-009                                                                                                                                                                                                                  | src-tauri/src/ag/edit_support.rs       | 102  |
| BR-AG-STATE-002-GUARD-AG-AG-TOOL-059, BR-AG-DATA-010-GUARD-AG-AG-TOOL-059                                                                                                                                                                               | GUARD  | BR-AG-STATE-002, BR-AG-DATA-010                                                                                                                                                                                                                  | src-tauri/src/ag/edit_support.rs       | 293  |
| BR-AG-DATA-010-GUARD-AG-AG-TOOL-061                                                                                                                                                                                                                     | GUARD  | BR-AG-DATA-010                                                                                                                                                                                                                                   | src-tauri/src/ag/edit_support.rs       | 302  |
| BR-ED-DATA-007-GUARD-AG-AG-TOOL-018                                                                                                                                                                                                                     | GUARD  | BR-ED-DATA-007 term_ref: TERM-ED-007                                                                                                                                                                                                             | src-tauri/src/ag/edit_table_runtime.rs | 8    |
| BR-AG-DATA-004-DATA-AG-AG-TOOL-061                                                                                                                                                                                                                      | DATA   | BR-AG-DATA-004                                                                                                                                                                                                                                   | src-tauri/src/ag/model_output.rs       | 342  |
| BR-AG-DATA-003-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-007-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-008-DATA-AG-AG-TARGETSET-001                                                                                                                           | DATA   | BR-AG-DATA-003, BR-SYS-STATE-007, BR-SYS-STATE-008 term_ref: TERM-AG-019, TERM-AG-001, TERM-AG-015, TERM-REF-003                                                                                                                                 | src-tauri/src/ag/target_set.rs         | 32   |
| BR-AG-DATA-003-EFFECT-AG-AG-TARGETSET-002, BR-SYS-PERSIST-001-EFFECT-AG-AG-TARGETSET-002                                                                                                                                                                | EFFECT | BR-AG-DATA-003, BR-SYS-PERSIST-001 term_ref: TERM-AG-019, TERM-AG-015                                                                                                                                                                            | src-tauri/src/ag/target_set.rs         | 202  |
| BR-AG-DATA-003-GUARD-AG-AG-TARGETSET-003                                                                                                                                                                                                                | GUARD  | BR-AG-DATA-003 term_ref: TERM-AG-019, TERM-DE-002                                                                                                                                                                                                | src-tauri/src/ag/target_set.rs         | 272  |
| BR-SYS-STATE-008-EFFECT-AG-AG-TASK-002                                                                                                                                                                                                                  | EFFECT | BR-SYS-STATE-008 term_ref: TERM-AG-015, TERM-AG-018                                                                                                                                                                                              | src-tauri/src/ag/task.rs               | 70   |
| BR-SYS-STATE-008-EFFECT-AG-AG-DECISION-001, BR-AG-DATA-003-EFFECT-AG-AG-DECISION-001                                                                                                                                                                    | EFFECT | BR-SYS-STATE-008, BR-AG-DATA-003 term_ref: TERM-AG-015, TERM-AG-017, TERM-AG-019                                                                                                                                                                 | src-tauri/src/ag/task.rs               | 505  |
| BR-SYS-STATE-008-EFFECT-AG-AG-DECISION-002, BR-AG-DATA-003-EFFECT-AG-AG-DECISION-002                                                                                                                                                                    | EFFECT | BR-SYS-STATE-008, BR-AG-DATA-003 term_ref: TERM-AG-015, TERM-AG-017, TERM-AG-019                                                                                                                                                                 | src-tauri/src/ag/task.rs               | 580  |
| BR-SYS-STATE-008-GUARD-AG-AG-TASK-004                                                                                                                                                                                                                   | GUARD  | BR-SYS-STATE-008 term_ref: TERM-AG-015, TERM-AG-018                                                                                                                                                                                              | src-tauri/src/ag/task.rs               | 995  |
| BR-SYS-STATE-008-GUARD-AG-AG-TASK-003                                                                                                                                                                                                                   | GUARD  | BR-SYS-STATE-008 term_ref: TERM-AG-015, TERM-AG-018                                                                                                                                                                                              | src-tauri/src/ag/task.rs               | 1019 |
| BR-AG-STATE-002-API-AG-WS-OPERATION-005, BR-AG-DATA-003-API-AG-WS-OPERATION-005, BR-SYS-STATE-008-API-AG-WS-OPERATION-005, BR-WS-STATE-003-API-AG-WS-OPERATION-005, BR-WS-PERSIST-003-API-AG-WS-OPERATION-005, BR-WS-OBS-002-API-AG-WS-OPERATION-005    | API    | BR-AG-STATE-002, BR-AG-DATA-003, BR-SYS-STATE-008, BR-WS-STATE-003, BR-WS-PERSIST-003, BR-WS-OBS-002 term_ref: TERM-AG-015, TERM-AG-017, TERM-AG-019, TERM-WS-003                                                                                | src-tauri/src/ag/task_runtime.rs       | 197  |
| BR-AG-STATE-002-RB-AG-AG-TOOL-001                                                                                                                                                                                                                       | RB     | BR-AG-STATE-002 term_ref: TERM-AG-002                                                                                                                                                                                                            | src-tauri/src/ag/tool.rs               | 60   |
| BR-AG-STATE-002-GUARD-AG-AG-TOOL-001                                                                                                                                                                                                                    | GUARD  | BR-AG-STATE-002                                                                                                                                                                                                                                  | src-tauri/src/ag/tool.rs               | 102  |
| BR-AG-STATE-002-DATA-AG-AG-TOOL-044, BR-SYS-DATA-008-DATA-AG-AG-TOOL-044                                                                                                                                                                                | DATA   | BR-AG-STATE-002, BR-SYS-DATA-008 term_ref: TERM-AG-002, TERM-AG-014                                                                                                                                                                              | src-tauri/src/ag/tool_exec.rs          | 18   |
| BR-AG-OBS-001-DATA-AG-AG-TOOL-043, BR-AG-DATA-008-DATA-AG-AG-TOOL-043, BR-SYS-DATA-008-DATA-AG-AG-TOOL-043                                                                                                                                              | DATA   | BR-AG-OBS-001, BR-AG-DATA-008, BR-SYS-DATA-008 term_ref: TERM-AG-020, TERM-AG-021, TERM-AG-023                                                                                                                                                   | src-tauri/src/ag/tool_exec.rs          | 173  |
| BR-AG-STATE-002-QUERY-AG-AG-TOOL-012, BR-AG-DATA-001-QUERY-AG-AG-TOOL-012, BR-WS-DATA-001-QUERY-AG-AG-TOOL-012                                                                                                                                          | QUERY  | BR-AG-STATE-002, BR-AG-DATA-001, BR-WS-DATA-001 term_ref: TERM-ED-008, TERM-WS-001                                                                                                                                                               | src-tauri/src/ag/tool_exec.rs          | 483  |
| BR-AG-DATA-005-DATA-AG-AG-TOOL-059, BR-AG-OBS-001-DATA-AG-AG-TOOL-059                                                                                                                                                                                   | DATA   | BR-AG-DATA-005, BR-AG-OBS-001                                                                                                                                                                                                                    | src-tauri/src/ag/tool_exec.rs          | 844  |
| BR-AG-DATA-005-GUARD-AG-AG-TOOL-057, BR-AG-OBS-001-GUARD-AG-AG-TOOL-057                                                                                                                                                                                 | GUARD  | BR-AG-DATA-005, BR-AG-OBS-001                                                                                                                                                                                                                    | src-tauri/src/ag/tool_exec.rs          | 922  |
| BR-AG-STATE-002-GUARD-AG-AG-TOOL-045, BR-SYS-STATE-007-GUARD-AG-AG-TOOL-045                                                                                                                                                                             | GUARD  | BR-AG-STATE-002, BR-SYS-STATE-007                                                                                                                                                                                                                | src-tauri/src/ag/tool_exec.rs          | 954  |
| BR-AG-STATE-002-DATA-AG-AG-TOOL-039, BR-AG-DATA-004-DATA-AG-AG-TOOL-039                                                                                                                                                                                 | DATA   | BR-AG-STATE-002, BR-AG-DATA-004 term_ref: TERM-AG-013, TERM-AG-014, TERM-AG-019                                                                                                                                                                  | src-tauri/src/ag/tool_registry.rs      | 169  |
| BR-SYS-STATE-008-GUARD-AG-AG-TOOL-040, BR-WS-STATE-003-GUARD-AG-AG-TOOL-040                                                                                                                                                                             | GUARD  | BR-SYS-STATE-008, BR-WS-STATE-003 term_ref: TERM-AG-014, TERM-AG-015, TERM-WS-003                                                                                                                                                                | src-tauri/src/ag/tool_registry.rs      | 415  |
| BR-AG-STATE-002-GUARD-AG-AG-TOOL-035, BR-SYS-STATE-007-GUARD-AG-AG-TOOL-035, BR-AG-DATA-003-GUARD-AG-AG-TOOL-035, BR-WS-DATA-001-GUARD-AG-AG-TOOL-035                                                                                                   | GUARD  | BR-AG-STATE-002, BR-SYS-STATE-007, BR-AG-DATA-003, BR-WS-DATA-001 term_ref: TERM-AG-014, TERM-AG-017, TERM-AG-019, TERM-WS-001                                                                                                                   | src-tauri/src/ag/tool_registry.rs      | 456  |
| BR-AG-DATA-003-GUARD-AG-AG-TOOL-033, BR-AG-STATE-002-GUARD-AG-AG-TOOL-033                                                                                                                                                                               | GUARD  | BR-AG-DATA-003, BR-AG-STATE-002 term_ref: TERM-AG-019, TERM-AG-017                                                                                                                                                                               | src-tauri/src/ag/tool_registry.rs      | 850  |
| BR-AG-OBS-001-EFFECT-AG-AG-TURN-044, BR-AG-STATE-001-EFFECT-AG-AG-TURN-044, BR-AG-STATE-002-EFFECT-AG-AG-TURN-044, BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-044, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-044                                                      | EFFECT | BR-AG-OBS-001, BR-AG-STATE-001, BR-AG-STATE-002, BR-AG-PERSIST-001, BR-SYS-STATE-007 term_ref: TERM-AG-001, TERM-AG-002, TERM-AG-022, TERM-AG-023                                                                                                | src-tauri/src/ag/turn_events.rs        | 476  |
| BR-AG-OBS-001-QUERY-AG-AG-TURN-048, BR-AG-PERSIST-001-QUERY-AG-AG-TURN-048, BR-SYS-DATA-008-QUERY-AG-AG-TURN-048                                                                                                                                        | QUERY  | BR-AG-OBS-001, BR-AG-PERSIST-001, BR-SYS-DATA-008 term_ref: TERM-AG-021, TERM-AG-023                                                                                                                                                             | src-tauri/src/ag/turn_events.rs        | 988  |
| BR-AG-DATA-008-GUARD-AG-AG-TURN-053, BR-SYS-STATE-007-GUARD-AG-AG-TURN-053                                                                                                                                                                              | GUARD  | BR-AG-DATA-008, BR-SYS-STATE-007 term_ref: TERM-AG-020                                                                                                                                                                                           | src-tauri/src/ag/turn_runtime.rs       | 88   |
| BR-SYS-STATE-007-GUARD-AG-AG-TURN-037                                                                                                                                                                                                                   | GUARD  | BR-SYS-STATE-007 term_ref: TERM-AG-013                                                                                                                                                                                                           | src-tauri/src/ag/turn_runtime.rs       | 340  |
| BR-AG-OBS-001-API-AG-AG-TURN-018, BR-AG-STATE-001-API-AG-AG-TURN-018, BR-AG-STATE-004-API-AG-AG-TURN-018, BR-AG-DATA-008-API-AG-AG-TURN-018, BR-ED-DATA-008-API-AG-AG-TURN-018, BR-SYS-STATE-007-API-AG-AG-TURN-018, BR-SYS-DATA-008-API-AG-AG-TURN-018 | API    | BR-AG-OBS-001, BR-AG-STATE-001, BR-AG-STATE-004, BR-AG-DATA-008, BR-ED-DATA-008, BR-SYS-STATE-007, BR-SYS-DATA-008 term_ref: TERM-AG-001, TERM-AG-012, TERM-AG-013, TERM-AG-014, TERM-AG-015, TERM-AG-016, TERM-AG-020, TERM-AG-021, TERM-AG-023 | src-tauri/src/ag/turn_runtime.rs       | 753  |
| BR-AG-STATE-002-EFFECT-AG-AG-TOOL-038                                                                                                                                                                                                                   | EFFECT | BR-AG-STATE-002 term_ref: TERM-AG-002                                                                                                                                                                                                            | src-tauri/src/ag/turn_tool_loop.rs     | 555  |
| BR-AG-DATA-011-DATA-AG-AG-TOOL-062                                                                                                                                                                                                                      | DATA   | BR-AG-DATA-011                                                                                                                                                                                                                                   | src-tauri/src/ag/turn_tool_loop.rs     | 583  |
| BR-AG-DATA-012-DATA-AG-AG-TOOL-063                                                                                                                                                                                                                      | DATA   | BR-AG-DATA-012                                                                                                                                                                                                                                   | src-tauri/src/ag/turn_tool_loop.rs     | 654  |
| BR-AG-DATA-013-RB-AG-AG-TOOL-065                                                                                                                                                                                                                        | RB     | BR-AG-DATA-013                                                                                                                                                                                                                                   | src-tauri/src/ag/turn_tool_loop.rs     | 772  |
| BR-SYS-STATE-008-RB-AG-AG-TOOL-064                                                                                                                                                                                                                      | RB     | BR-SYS-STATE-008 term_ref: TERM-AG-012, TERM-AG-015                                                                                                                                                                                              | src-tauri/src/ag/turn_tool_loop.rs     | 848  |
| BR-AG-DATA-011-EFFECT-AG-AG-TOOL-070                                                                                                                                                                                                                    | EFFECT | BR-AG-DATA-011                                                                                                                                                                                                                                   | src-tauri/src/ag/turn_tool_loop.rs     | 978  |
| BR-AG-STATE-002-RB-AG-AG-TOOL-068                                                                                                                                                                                                                       | RB     | BR-AG-STATE-002                                                                                                                                                                                                                                  | src-tauri/src/ag/turn_tool_loop.rs     | 1301 |
| BR-SYS-STATE-008-EFFECT-AG-AG-TOOL-071                                                                                                                                                                                                                  | EFFECT | BR-SYS-STATE-008                                                                                                                                                                                                                                 | src-tauri/src/ag/turn_tool_loop.rs     | 1711 |
| BR-AG-STATE-002-RB-AG-AG-TOOL-067                                                                                                                                                                                                                       | RB     | BR-AG-STATE-002                                                                                                                                                                                                                                  | src-tauri/src/ag/turn_tool_loop.rs     | 1849 |
| BR-AG-STATE-002-GUARD-AG-AG-TOOL-066                                                                                                                                                                                                                    | GUARD  | BR-AG-STATE-002                                                                                                                                                                                                                                  | src-tauri/src/ag/turn_tool_loop.rs     | 1990 |
| BR-AG-STATE-002-RB-AG-AG-TOOL-069                                                                                                                                                                                                                       | RB     | BR-AG-STATE-002                                                                                                                                                                                                                                  | src-tauri/src/ag/turn_tool_loop.rs     | 2143 |
| BR-AG-STATE-002-RB-AG-AG-TOOL-018                                                                                                                                                                                                                       | RB     | BR-AG-STATE-002 term_ref: TERM-AG-014, TERM-AG-015, TERM-AG-016, TERM-AG-020                                                                                                                                                                     | src-tauri/src/ag/turn_tool_loop.rs     | 2677 |
| BR-AG-OBS-001-DATA-AG-AG-TURN-043, BR-AG-STATE-001-DATA-AG-AG-TURN-043, BR-AG-STATE-002-DATA-AG-AG-TURN-043, BR-AG-PERSIST-001-DATA-AG-AG-TURN-043, BR-SYS-STATE-007-DATA-AG-AG-TURN-043                                                                | DATA   | BR-AG-OBS-001, BR-AG-STATE-001, BR-AG-STATE-002, BR-AG-PERSIST-001, BR-SYS-STATE-007 term_ref: TERM-AG-001, TERM-AG-002, TERM-AG-021, TERM-AG-022                                                                                                | src-tauri/src/ipc/mod.rs               | 127  |

#### AG-TURN

| codes                                                                                                                                                                                                                                                   | type   | rules                                                                                                                                                                                                                                            | 文件                                           | 行号   |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------- | ---- |
| BR-AG-UI-001-IO-AG-AG-TURN-062                                                                                                                                                                                                                          | IO     | BR-AG-UI-001                                                                                                                                                                                                                                     | src/components/ag/AgentVerificationPanel.tsx | 15   |
| BR-SYS-STATE-007-EFFECT-AG-AG-TURN-058, BR-AG-UI-001-EFFECT-AG-AG-TURN-058                                                                                                                                                                              | EFFECT | BR-SYS-STATE-007, BR-AG-UI-001 term_ref: TERM-AG-017                                                                                                                                                                                             | src/components/ag/DecisionBar.tsx            | 40   |
| BR-AG-UI-001-RB-AG-AG-TURN-013                                                                                                                                                                                                                          | RB     | BR-AG-UI-001 term_ref: TERM-AG-011                                                                                                                                                                                                               | src/components/ag/MessageFlow.tsx            | 9    |
| BR-AG-UI-001-RB-AG-AG-TURN-014                                                                                                                                                                                                                          | RB     | BR-AG-UI-001 term_ref: TERM-AG-011                                                                                                                                                                                                               | src/components/ag/MessageList.tsx            | 12   |
| BR-AG-UI-001-RB-AG-AG-TURN-012                                                                                                                                                                                                                          | RB     | BR-AG-UI-001 term_ref: TERM-AG-011                                                                                                                                                                                                               | src/components/ag/MessageMarkdown.tsx        | 8    |
| BR-AG-OBS-001-RB-AG-AG-TURN-050, BR-AG-UI-001-RB-AG-AG-TURN-050                                                                                                                                                                                         | RB     | BR-AG-OBS-001, BR-AG-UI-001 term_ref: TERM-AG-021, TERM-AG-023                                                                                                                                                                                   | src/components/ag/ProjectReportCard.tsx      | 5    |
| BR-AG-DATA-008-DATA-AG-AG-TURN-041, BR-ED-DATA-008-DATA-AG-AG-TURN-041, BR-SYS-DATA-008-DATA-AG-AG-TURN-041                                                                                                                                             | DATA   | BR-AG-DATA-008, BR-ED-DATA-008, BR-SYS-DATA-008 term_ref: TERM-AG-020                                                                                                                                                                            | src/services/agService.ts                    | 182  |
| BR-AG-DATA-008-DATA-AG-AG-TURN-054, BR-SYS-DATA-008-DATA-AG-AG-TURN-054                                                                                                                                                                                 | DATA   | BR-AG-DATA-008, BR-SYS-DATA-008 term_ref: TERM-AG-020                                                                                                                                                                                            | src/services/agService.ts                    | 545  |
| BR-AG-DATA-001-EFFECT-AG-AG-TURN-005                                                                                                                                                                                                                    | EFFECT | BR-AG-DATA-001 term_ref: TERM-DOC-003, TERM-ED-008                                                                                                                                                                                               | src/services/agService.ts                    | 648  |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-006, BR-AG-DATA-001-EFFECT-AG-AG-TURN-006                                                                                                                                                                            | EFFECT | BR-SYS-STATE-008, BR-AG-DATA-001 term_ref: TERM-AG-015, TERM-ED-008                                                                                                                                                                              | src/services/agService.ts                    | 1535 |
| BR-SYS-STATE-007-EFFECT-AG-AG-TURN-060, BR-SYS-STATE-008-EFFECT-AG-AG-TURN-060                                                                                                                                                                          | EFFECT | BR-SYS-STATE-007, BR-SYS-STATE-008 term_ref: TERM-AG-017, TERM-AG-001                                                                                                                                                                            | src/services/agService.ts                    | 1650 |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-067, BR-SYS-STATE-007-QUERY-AG-AG-TURN-067                                                                                                                                                                           | QUERY  | BR-AG-PERSIST-001, BR-SYS-STATE-007                                                                                                                                                                                                              | src/services/agService.ts                    | 1850 |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-068, BR-SYS-STATE-007-QUERY-AG-AG-TURN-068                                                                                                                                                                           | QUERY  | BR-AG-PERSIST-001, BR-SYS-STATE-007                                                                                                                                                                                                              | src/services/agService.ts                    | 1889 |
| BR-AG-OBS-001-EFFECT-AG-AG-TURN-035, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-035, BR-AG-STATE-001-EFFECT-AG-AG-TURN-035                                                                                                                                      | EFFECT | BR-AG-OBS-001, BR-SYS-STATE-007, BR-AG-STATE-001 term_ref: TERM-AG-001, TERM-AG-021                                                                                                                                                              | src/services/agService.ts                    | 2178 |
| BR-SYS-STATE-007-EFFECT-AG-AG-TURN-059, BR-SYS-STATE-008-EFFECT-AG-AG-TURN-059                                                                                                                                                                          | EFFECT | BR-SYS-STATE-007, BR-SYS-STATE-008 term_ref: TERM-AG-017                                                                                                                                                                                         | src/store/agStore.ts                         | 492  |
| BR-AG-OBS-001-EFFECT-AG-AG-TURN-042, BR-AG-STATE-001-EFFECT-AG-AG-TURN-042, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-042, BR-SYS-STATE-008-EFFECT-AG-AG-TURN-042                                                                                              | EFFECT | BR-AG-OBS-001, BR-AG-STATE-001, BR-SYS-STATE-007, BR-SYS-STATE-008 term_ref: TERM-AG-021, TERM-AG-022                                                                                                                                            | src/store/agStore.ts                         | 560  |
| BR-AG-DATA-007-QUERY-AG-AG-MODEL-CATALOG-013                                                                                                                                                                                                            | QUERY  | BR-AG-DATA-007 term_ref: TERM-AG-010                                                                                                                                                                                                             | src/store/modelStore.ts                      | 161  |
| BR-SYS-DATA-006-DATA-AG-AG-TOOL-032                                                                                                                                                                                                                     | DATA   | BR-SYS-DATA-006 term_ref: TERM-SYS-002                                                                                                                                                                                                           | src-tauri/src/ag/bdocx_capabilities.rs       | 1298 |
| BR-AG-DATA-001-GUARD-AG-AG-TOOL-044, BR-AG-DATA-008-GUARD-AG-AG-TOOL-044                                                                                                                                                                                | GUARD  | BR-AG-DATA-001, BR-AG-DATA-008 term_ref: TERM-AG-020                                                                                                                                                                                             | src-tauri/src/ag/blocks.rs                   | 175  |
| BR-AG-DATA-001-QUERY-AG-AG-TURN-010, BR-DOC-DATA-003-QUERY-AG-AG-TURN-010                                                                                                                                                                               | QUERY  | BR-AG-DATA-001, BR-DOC-DATA-003 term_ref: TERM-DOC-003, TERM-ED-001                                                                                                                                                                              | src-tauri/src/ag/blocks.rs                   | 533  |
| BR-AG-PERSIST-001-QUERY-AG-AG-TOOL-046, BR-SYS-STATE-007-QUERY-AG-AG-TOOL-046                                                                                                                                                                           | QUERY  | BR-AG-PERSIST-001, BR-SYS-STATE-007                                                                                                                                                                                                              | src-tauri/src/ag/capability_runtime.rs       | 66   |
| BR-AG-STATE-002-EFFECT-AG-AG-TOOL-048, BR-SYS-STATE-007-EFFECT-AG-AG-TOOL-048, BR-AG-PERSIST-001-EFFECT-AG-AG-TOOL-048                                                                                                                                  | EFFECT | BR-AG-STATE-002, BR-SYS-STATE-007, BR-AG-PERSIST-001                                                                                                                                                                                             | src-tauri/src/ag/capability_runtime.rs       | 239  |
| BR-AG-STATE-002-EFFECT-AG-AG-TOOL-046, BR-SYS-STATE-007-EFFECT-AG-AG-TOOL-046, BR-AG-PERSIST-001-EFFECT-AG-AG-TOOL-046                                                                                                                                  | EFFECT | BR-AG-STATE-002, BR-SYS-STATE-007, BR-AG-PERSIST-001                                                                                                                                                                                             | src-tauri/src/ag/capability_runtime.rs       | 308  |
| BR-AG-DATA-004-DATA-AG-AG-TURN-046, BR-AG-STATE-001-DATA-AG-AG-TURN-046                                                                                                                                                                                 | DATA   | BR-AG-DATA-004, BR-AG-STATE-001 term_ref: TERM-AG-012                                                                                                                                                                                            | src-tauri/src/ag/context.rs                  | 53   |
| BR-AG-DATA-002-DATA-AG-AG-TURN-055, BR-AG-DATA-005-DATA-AG-AG-TURN-055                                                                                                                                                                                  | DATA   | BR-AG-DATA-002, BR-AG-DATA-005                                                                                                                                                                                                                   | src-tauri/src/ag/context.rs                  | 164  |
| BR-AG-DATA-002-DATA-AG-AG-TURN-016                                                                                                                                                                                                                      | DATA   | BR-AG-DATA-002                                                                                                                                                                                                                                   | src-tauri/src/ag/context.rs                  | 344  |
| BR-AG-DATA-005-DATA-AG-AG-TURN-004                                                                                                                                                                                                                      | DATA   | BR-AG-DATA-005                                                                                                                                                                                                                                   | src-tauri/src/ag/context.rs                  | 436  |
| BR-AG-DATA-004-DATA-AG-AG-TURN-005                                                                                                                                                                                                                      | DATA   | BR-AG-DATA-004 term_ref: TERM-AG-005                                                                                                                                                                                                             | src-tauri/src/ag/context.rs                  | 567  |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-030                                                                                                                                                                                                                 | EFFECT | BR-AG-PERSIST-001 term_ref: TERM-AG-009                                                                                                                                                                                                          | src-tauri/src/ag/history.rs                  | 172  |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-002                                                                                                                                                                                                                  | QUERY  | BR-AG-PERSIST-001 term_ref: TERM-AG-009                                                                                                                                                                                                          | src-tauri/src/ag/history.rs                  | 254  |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-003                                                                                                                                                                                                                 | EFFECT | BR-AG-PERSIST-001 term_ref: TERM-AG-001, TERM-AG-009                                                                                                                                                                                             | src-tauri/src/ag/history.rs                  | 304  |
| BR-AG-PERSIST-001-GUARD-AG-AG-TURN-048, BR-SYS-STATE-007-GUARD-AG-AG-TURN-048                                                                                                                                                                           | GUARD  | BR-AG-PERSIST-001, BR-SYS-STATE-007 term_ref: TERM-AG-001, TERM-AG-009, TERM-AG-022                                                                                                                                                              | src-tauri/src/ag/history.rs                  | 366  |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-046                                                                                                                                                                                                                 | EFFECT | BR-AG-PERSIST-001 term_ref: TERM-AG-001, TERM-AG-009, TERM-AG-022                                                                                                                                                                                | src-tauri/src/ag/history.rs                  | 398  |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-003                                                                                                                                                                                                                  | QUERY  | BR-AG-PERSIST-001 term_ref: TERM-AG-001, TERM-AG-009                                                                                                                                                                                             | src-tauri/src/ag/history.rs                  | 462  |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-031                                                                                                                                                                                                                 | EFFECT | BR-AG-PERSIST-001 term_ref: TERM-AG-001, TERM-AG-009                                                                                                                                                                                             | src-tauri/src/ag/history.rs                  | 509  |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-020                                                                                                                                                                                                                  | QUERY  | BR-AG-PERSIST-001                                                                                                                                                                                                                                | src-tauri/src/ag/history_routes.rs           | 7    |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-021                                                                                                                                                                                                                 | EFFECT | BR-AG-PERSIST-001                                                                                                                                                                                                                                | src-tauri/src/ag/history_routes.rs           | 23   |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-022                                                                                                                                                                                                                 | EFFECT | BR-AG-PERSIST-001                                                                                                                                                                                                                                | src-tauri/src/ag/history_routes.rs           | 72   |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-023                                                                                                                                                                                                                  | QUERY  | BR-AG-PERSIST-001                                                                                                                                                                                                                                | src-tauri/src/ag/history_routes.rs           | 111  |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-047, BR-SYS-STATE-007-QUERY-AG-AG-TURN-047                                                                                                                                                                           | QUERY  | BR-AG-PERSIST-001, BR-SYS-STATE-007 term_ref: TERM-AG-001, TERM-AG-022                                                                                                                                                                           | src-tauri/src/ag/history_routes.rs           | 131  |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-024                                                                                                                                                                                                                 | EFFECT | BR-AG-PERSIST-001                                                                                                                                                                                                                                | src-tauri/src/ag/history_routes.rs           | 171  |
| BR-AG-DATA-004-GUARD-AG-AG-TURN-047, BR-AG-STATE-001-GUARD-AG-AG-TURN-047                                                                                                                                                                               | GUARD  | BR-AG-DATA-004, BR-AG-STATE-001 term_ref: TERM-AG-012                                                                                                                                                                                            | src-tauri/src/ag/model_output.rs             | 54   |
| BR-AG-DATA-004-DATA-AG-AG-TOOL-061                                                                                                                                                                                                                      | DATA   | BR-AG-DATA-004                                                                                                                                                                                                                                   | src-tauri/src/ag/model_output.rs             | 342  |
| BR-AG-DATA-001-DATA-AG-AG-TURN-011, BR-AG-DATA-002-DATA-AG-AG-TURN-011, BR-AG-DATA-004-DATA-AG-AG-TURN-011, BR-AG-DATA-005-DATA-AG-AG-TURN-011, BR-AG-DATA-008-DATA-AG-AG-TURN-011                                                                      | DATA   | BR-AG-DATA-001, BR-AG-DATA-002, BR-AG-DATA-004, BR-AG-DATA-005, BR-AG-DATA-008 term_ref: TERM-AG-005, TERM-AG-020                                                                                                                                | src-tauri/src/ag/prompt_runtime.rs           | 283  |
| BR-AG-DATA-001-RB-AG-AG-TURN-017                                                                                                                                                                                                                        | RB     | BR-AG-DATA-001 term_ref: TERM-REF-003                                                                                                                                                                                                            | src-tauri/src/ag/reference_runtime.rs        | 167  |
| BR-AG-DATA-001-QUERY-AG-AG-TURN-029                                                                                                                                                                                                                     | QUERY  | BR-AG-DATA-001                                                                                                                                                                                                                                   | src-tauri/src/ag/reference_runtime.rs        | 282  |
| BR-AG-DATA-003-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-007-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-008-DATA-AG-AG-TARGETSET-001                                                                                                                           | DATA   | BR-AG-DATA-003, BR-SYS-STATE-007, BR-SYS-STATE-008 term_ref: TERM-AG-019, TERM-AG-001, TERM-AG-015, TERM-REF-003                                                                                                                                 | src-tauri/src/ag/target_set.rs               | 32   |
| BR-AG-DATA-003-EFFECT-AG-AG-TARGETSET-002, BR-SYS-PERSIST-001-EFFECT-AG-AG-TARGETSET-002                                                                                                                                                                | EFFECT | BR-AG-DATA-003, BR-SYS-PERSIST-001 term_ref: TERM-AG-019, TERM-AG-015                                                                                                                                                                            | src-tauri/src/ag/target_set.rs               | 202  |
| BR-SYS-STATE-008-EFFECT-AG-AG-TASK-002                                                                                                                                                                                                                  | EFFECT | BR-SYS-STATE-008 term_ref: TERM-AG-015, TERM-AG-018                                                                                                                                                                                              | src-tauri/src/ag/task.rs                     | 70   |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-032, BR-AG-DATA-001-EFFECT-AG-AG-TURN-032                                                                                                                                                                            | EFFECT | BR-SYS-STATE-008, BR-AG-DATA-001 term_ref: TERM-AG-015, TERM-ED-008                                                                                                                                                                              | src-tauri/src/ag/task.rs                     | 110  |
| BR-SYS-STATE-008-GUARD-AG-AG-TURN-005, BR-AG-DATA-001-GUARD-AG-AG-TURN-005                                                                                                                                                                              | GUARD  | BR-SYS-STATE-008, BR-AG-DATA-001 term_ref: TERM-AG-015, TERM-ED-008                                                                                                                                                                              | src-tauri/src/ag/task.rs                     | 169  |
| BR-SYS-STATE-008-GUARD-AG-AG-TURN-006, BR-AG-DATA-001-GUARD-AG-AG-TURN-006                                                                                                                                                                              | GUARD  | BR-SYS-STATE-008, BR-AG-DATA-001 term_ref: TERM-AG-015, TERM-ED-008                                                                                                                                                                              | src-tauri/src/ag/task.rs                     | 287  |
| BR-SYS-STATE-008-EFFECT-AG-AG-DECISION-001, BR-AG-DATA-003-EFFECT-AG-AG-DECISION-001                                                                                                                                                                    | EFFECT | BR-SYS-STATE-008, BR-AG-DATA-003 term_ref: TERM-AG-015, TERM-AG-017, TERM-AG-019                                                                                                                                                                 | src-tauri/src/ag/task.rs                     | 505  |
| BR-SYS-STATE-008-EFFECT-AG-AG-DECISION-002, BR-AG-DATA-003-EFFECT-AG-AG-DECISION-002                                                                                                                                                                    | EFFECT | BR-SYS-STATE-008, BR-AG-DATA-003 term_ref: TERM-AG-015, TERM-AG-017, TERM-AG-019                                                                                                                                                                 | src-tauri/src/ag/task.rs                     | 580  |
| BR-AG-DATA-004-QUERY-AG-AG-TURN-052, BR-SYS-STATE-008-QUERY-AG-AG-TURN-052                                                                                                                                                                              | QUERY  | BR-AG-DATA-004, BR-SYS-STATE-008 term_ref: TERM-AG-017, TERM-AG-015                                                                                                                                                                              | src-tauri/src/ag/task.rs                     | 750  |
| BR-SYS-STATE-007-EFFECT-AG-AG-TURN-047, BR-AG-STATE-001-EFFECT-AG-AG-TURN-047, BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-047                                                                                                                                  | EFFECT | BR-SYS-STATE-007, BR-AG-STATE-001, BR-AG-PERSIST-001 term_ref: TERM-AG-015                                                                                                                                                                       | src-tauri/src/ag/task.rs                     | 859  |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-033, BR-AG-STATE-001-EFFECT-AG-AG-TURN-033                                                                                                                                                                           | EFFECT | BR-SYS-STATE-008, BR-AG-STATE-001 term_ref: TERM-AG-015                                                                                                                                                                                          | src-tauri/src/ag/task.rs                     | 929  |
| BR-SYS-STATE-008-GUARD-AG-AG-TASK-004                                                                                                                                                                                                                   | GUARD  | BR-SYS-STATE-008 term_ref: TERM-AG-015, TERM-AG-018                                                                                                                                                                                              | src-tauri/src/ag/task.rs                     | 995  |
| BR-SYS-STATE-008-GUARD-AG-AG-TASK-003                                                                                                                                                                                                                   | GUARD  | BR-SYS-STATE-008 term_ref: TERM-AG-015, TERM-AG-018                                                                                                                                                                                              | src-tauri/src/ag/task.rs                     | 1019 |
| BR-SYS-STATE-008-QUERY-AG-AG-TASK-004, BR-AG-PERSIST-001-QUERY-AG-AG-TASK-004                                                                                                                                                                           | QUERY  | BR-SYS-STATE-008, BR-AG-PERSIST-001 term_ref: TERM-AG-015, TERM-AG-016, TERM-AG-018, TERM-AG-022                                                                                                                                                 | src-tauri/src/ag/task.rs                     | 1137 |
| BR-AG-DATA-003-EFFECT-AG-AG-TURN-037                                                                                                                                                                                                                    | EFFECT | BR-AG-DATA-003 term_ref: TERM-AG-015, TERM-AG-019, TERM-WS-003                                                                                                                                                                                   | src-tauri/src/ag/task_runtime.rs             | 16   |
| BR-SYS-STATE-008-QUERY-AG-AG-TURN-025                                                                                                                                                                                                                   | QUERY  | BR-SYS-STATE-008 term_ref: TERM-AG-015                                                                                                                                                                                                           | src-tauri/src/ag/task_runtime.rs             | 46   |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-026, BR-AG-DATA-003-EFFECT-AG-AG-TURN-026                                                                                                                                                                            | EFFECT | BR-SYS-STATE-008, BR-AG-DATA-003 term_ref: TERM-AG-015, TERM-AG-017, TERM-AG-019                                                                                                                                                                 | src-tauri/src/ag/task_runtime.rs             | 79   |
| BR-AG-STATE-002-API-AG-WS-OPERATION-005, BR-AG-DATA-003-API-AG-WS-OPERATION-005, BR-SYS-STATE-008-API-AG-WS-OPERATION-005, BR-WS-STATE-003-API-AG-WS-OPERATION-005, BR-WS-PERSIST-003-API-AG-WS-OPERATION-005, BR-WS-OBS-002-API-AG-WS-OPERATION-005    | API    | BR-AG-STATE-002, BR-AG-DATA-003, BR-SYS-STATE-008, BR-WS-STATE-003, BR-WS-PERSIST-003, BR-WS-OBS-002 term_ref: TERM-AG-015, TERM-AG-017, TERM-AG-019, TERM-WS-003                                                                                | src-tauri/src/ag/task_runtime.rs             | 197  |
| BR-AG-STATE-001-API-AG-AG-TURN-070, BR-AG-PERSIST-001-API-AG-AG-TURN-070                                                                                                                                                                                | API    | BR-AG-STATE-001, BR-AG-PERSIST-001 term_ref: TERM-AG-001, TERM-WS-004                                                                                                                                                                            | src-tauri/src/ag/task_runtime.rs             | 725  |
| BR-SYS-STATE-008-QUERY-AG-AG-TURN-027                                                                                                                                                                                                                   | QUERY  | BR-SYS-STATE-008 term_ref: TERM-AG-015, TERM-AG-018                                                                                                                                                                                              | src-tauri/src/ag/task_runtime.rs             | 775  |
| BR-AG-OBS-001-DATA-AG-AG-TOOL-043, BR-AG-DATA-008-DATA-AG-AG-TOOL-043, BR-SYS-DATA-008-DATA-AG-AG-TOOL-043                                                                                                                                              | DATA   | BR-AG-OBS-001, BR-AG-DATA-008, BR-SYS-DATA-008 term_ref: TERM-AG-020, TERM-AG-021, TERM-AG-023                                                                                                                                                   | src-tauri/src/ag/tool_exec.rs                | 173  |
| BR-AG-STATE-002-DATA-AG-AG-TOOL-039, BR-AG-DATA-004-DATA-AG-AG-TOOL-039                                                                                                                                                                                 | DATA   | BR-AG-STATE-002, BR-AG-DATA-004 term_ref: TERM-AG-013, TERM-AG-014, TERM-AG-019                                                                                                                                                                  | src-tauri/src/ag/tool_registry.rs            | 169  |
| BR-AG-STATE-001-RB-AG-AG-TURN-001, BR-SYS-STATE-007-RB-AG-AG-TURN-001                                                                                                                                                                                   | RB     | BR-AG-STATE-001, BR-SYS-STATE-007                                                                                                                                                                                                                | src-tauri/src/ag/turn.rs                     | 112  |
| BR-AG-VERIFY-001-GUARD-AG-AG-TURN-020                                                                                                                                                                                                                   | GUARD  | BR-AG-VERIFY-001                                                                                                                                                                                                                                 | src-tauri/src/ag/turn.rs                     | 160  |
| BR-SYS-STATE-007-GUARD-AG-AG-TURN-034, BR-AG-STATE-001-GUARD-AG-AG-TURN-034                                                                                                                                                                             | GUARD  | BR-SYS-STATE-007, BR-AG-STATE-001 term_ref: TERM-AG-001, TERM-AG-015, TERM-AG-019                                                                                                                                                                | src-tauri/src/ag/turn_control.rs             | 85   |
| BR-SYS-STATE-007-EFFECT-AG-AG-TURN-034, BR-AG-STATE-001-EFFECT-AG-AG-TURN-034                                                                                                                                                                           | EFFECT | BR-SYS-STATE-007, BR-AG-STATE-001 term_ref: TERM-AG-001, TERM-AG-015                                                                                                                                                                             | src-tauri/src/ag/turn_control.rs             | 300  |
| BR-AG-DATA-003-EFFECT-AG-AG-TURN-036, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-036                                                                                                                                                                            | EFFECT | BR-AG-DATA-003, BR-SYS-STATE-007 term_ref: TERM-AG-001, TERM-AG-019                                                                                                                                                                              | src-tauri/src/ag/turn_control.rs             | 475  |
| BR-AG-STATE-001-API-AG-AG-TURN-019, BR-AG-PERSIST-001-API-AG-AG-TURN-019, BR-SYS-STATE-007-API-AG-AG-TURN-019                                                                                                                                           | API    | BR-AG-STATE-001, BR-AG-PERSIST-001, BR-SYS-STATE-007 term_ref: TERM-AG-001, TERM-AG-022                                                                                                                                                          | src-tauri/src/ag/turn_control.rs             | 620  |
| BR-SYS-STATE-008-API-AG-AG-TURN-003, BR-AG-DATA-001-API-AG-AG-TURN-003                                                                                                                                                                                  | API    | BR-SYS-STATE-008, BR-AG-DATA-001 term_ref: TERM-AG-015, TERM-ED-008                                                                                                                                                                              | src-tauri/src/ag/turn_control.rs             | 654  |
| BR-AG-OBS-001-EFFECT-AG-AG-TURN-044, BR-AG-STATE-001-EFFECT-AG-AG-TURN-044, BR-AG-STATE-002-EFFECT-AG-AG-TURN-044, BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-044, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-044                                                      | EFFECT | BR-AG-OBS-001, BR-AG-STATE-001, BR-AG-STATE-002, BR-AG-PERSIST-001, BR-SYS-STATE-007 term_ref: TERM-AG-001, TERM-AG-002, TERM-AG-022, TERM-AG-023                                                                                                | src-tauri/src/ag/turn_events.rs              | 476  |
| BR-AG-DATA-005-IO-AG-AG-TURN-056, BR-AG-OBS-001-IO-AG-AG-TURN-056                                                                                                                                                                                       | IO     | BR-AG-DATA-005, BR-AG-OBS-001                                                                                                                                                                                                                    | src-tauri/src/ag/turn_events.rs              | 730  |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-045, BR-SYS-STATE-007-QUERY-AG-AG-TURN-045                                                                                                                                                                           | QUERY  | BR-AG-PERSIST-001, BR-SYS-STATE-007 term_ref: TERM-AG-001, TERM-AG-022                                                                                                                                                                           | src-tauri/src/ag/turn_events.rs              | 933  |
| BR-AG-OBS-001-QUERY-AG-AG-TURN-048, BR-AG-PERSIST-001-QUERY-AG-AG-TURN-048, BR-SYS-DATA-008-QUERY-AG-AG-TURN-048                                                                                                                                        | QUERY  | BR-AG-OBS-001, BR-AG-PERSIST-001, BR-SYS-DATA-008 term_ref: TERM-AG-021, TERM-AG-023                                                                                                                                                             | src-tauri/src/ag/turn_events.rs              | 988  |
| BR-AG-STATE-001-EFFECT-AG-AG-TURN-069, BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-069                                                                                                                                                                          | EFFECT | BR-AG-STATE-001, BR-AG-PERSIST-001 term_ref: TERM-AG-001, TERM-WS-004                                                                                                                                                                            | src-tauri/src/ag/turn_events.rs              | 1245 |
| BR-AG-DATA-002-DATA-AG-AG-TURN-031, BR-AG-DATA-008-DATA-AG-AG-TURN-031, BR-SYS-DATA-008-DATA-AG-AG-TURN-031                                                                                                                                             | DATA   | BR-AG-DATA-002, BR-AG-DATA-008, BR-SYS-DATA-008 term_ref: TERM-AG-005, TERM-AG-020                                                                                                                                                               | src-tauri/src/ag/turn_prompt.rs              | 95   |
| BR-AG-DATA-001-DATA-AG-AG-TURN-030                                                                                                                                                                                                                      | DATA   | BR-AG-DATA-001 term_ref: TERM-REF-003                                                                                                                                                                                                            | src-tauri/src/ag/turn_reference.rs           | 15   |
| BR-AG-DATA-008-GUARD-AG-AG-TURN-053, BR-SYS-STATE-007-GUARD-AG-AG-TURN-053                                                                                                                                                                              | GUARD  | BR-AG-DATA-008, BR-SYS-STATE-007 term_ref: TERM-AG-020                                                                                                                                                                                           | src-tauri/src/ag/turn_runtime.rs             | 88   |
| BR-SYS-STATE-007-GUARD-AG-AG-TURN-037                                                                                                                                                                                                                   | GUARD  | BR-SYS-STATE-007 term_ref: TERM-AG-013                                                                                                                                                                                                           | src-tauri/src/ag/turn_runtime.rs             | 340  |
| BR-AG-DATA-004-GUARD-AG-AG-TURN-049                                                                                                                                                                                                                     | GUARD  | BR-AG-DATA-004 term_ref: TERM-AG-001                                                                                                                                                                                                             | src-tauri/src/ag/turn_runtime.rs             | 362  |
| BR-AG-DATA-008-GUARD-AG-AG-TURN-042, BR-ED-DATA-008-GUARD-AG-AG-TURN-042, BR-SYS-DATA-008-GUARD-AG-AG-TURN-042                                                                                                                                          | GUARD  | BR-AG-DATA-008, BR-ED-DATA-008, BR-SYS-DATA-008 term_ref: TERM-AG-020                                                                                                                                                                            | src-tauri/src/ag/turn_runtime.rs             | 391  |
| BR-AG-DATA-008-GUARD-AG-AG-TURN-052, BR-SYS-DATA-008-GUARD-AG-AG-TURN-052                                                                                                                                                                               | GUARD  | BR-AG-DATA-008, BR-SYS-DATA-008 term_ref: TERM-AG-020                                                                                                                                                                                            | src-tauri/src/ag/turn_runtime.rs             | 574  |
| BR-AG-OBS-001-API-AG-AG-TURN-018, BR-AG-STATE-001-API-AG-AG-TURN-018, BR-AG-STATE-004-API-AG-AG-TURN-018, BR-AG-DATA-008-API-AG-AG-TURN-018, BR-ED-DATA-008-API-AG-AG-TURN-018, BR-SYS-STATE-007-API-AG-AG-TURN-018, BR-SYS-DATA-008-API-AG-AG-TURN-018 | API    | BR-AG-OBS-001, BR-AG-STATE-001, BR-AG-STATE-004, BR-AG-DATA-008, BR-ED-DATA-008, BR-SYS-STATE-007, BR-SYS-DATA-008 term_ref: TERM-AG-001, TERM-AG-012, TERM-AG-013, TERM-AG-014, TERM-AG-015, TERM-AG-016, TERM-AG-020, TERM-AG-021, TERM-AG-023 | src-tauri/src/ag/turn_runtime.rs             | 753  |
| BR-AG-DATA-004-DATA-AG-AG-TURN-062                                                                                                                                                                                                                      | DATA   | BR-AG-DATA-004                                                                                                                                                                                                                                   | src-tauri/src/ag/turn_tool_loop.rs           | 121  |
| BR-AG-STATE-005-EFFECT-AG-AG-TURN-071                                                                                                                                                                                                                   | EFFECT | BR-AG-STATE-005                                                                                                                                                                                                                                  | src-tauri/src/ag/turn_tool_loop.rs           | 686  |
| BR-AG-DATA-013-RB-AG-AG-TOOL-065                                                                                                                                                                                                                        | RB     | BR-AG-DATA-013                                                                                                                                                                                                                                   | src-tauri/src/ag/turn_tool_loop.rs           | 772  |
| BR-SYS-STATE-008-RB-AG-AG-TOOL-064                                                                                                                                                                                                                      | RB     | BR-SYS-STATE-008 term_ref: TERM-AG-012, TERM-AG-015                                                                                                                                                                                              | src-tauri/src/ag/turn_tool_loop.rs           | 848  |
| BR-AG-OBS-001-EFFECT-AG-AG-TURN-072                                                                                                                                                                                                                     | EFFECT | BR-AG-OBS-001 term_ref: TERM-AG-021, TERM-AG-023                                                                                                                                                                                                 | src-tauri/src/ag/turn_tool_loop.rs           | 901  |
| BR-AG-STATE-001-EFFECT-AG-AG-TURN-074                                                                                                                                                                                                                   | EFFECT | BR-AG-STATE-001                                                                                                                                                                                                                                  | src-tauri/src/ag/turn_tool_loop.rs           | 1107 |
| BR-AG-DATA-001-EFFECT-AG-AG-TURN-078                                                                                                                                                                                                                    | EFFECT | BR-AG-DATA-001                                                                                                                                                                                                                                   | src-tauri/src/ag/turn_tool_loop.rs           | 1177 |
| BR-AG-DATA-008-EFFECT-AG-AG-TURN-073                                                                                                                                                                                                                    | EFFECT | BR-AG-DATA-008                                                                                                                                                                                                                                   | src-tauri/src/ag/turn_tool_loop.rs           | 1609 |
| BR-AG-VERIFY-001-RB-AG-AG-TURN-076                                                                                                                                                                                                                      | RB     | BR-AG-VERIFY-001                                                                                                                                                                                                                                 | src-tauri/src/ag/turn_tool_loop.rs           | 2291 |
| BR-AG-VERIFY-001-EFFECT-AG-AG-TURN-077                                                                                                                                                                                                                  | EFFECT | BR-AG-VERIFY-001                                                                                                                                                                                                                                 | src-tauri/src/ag/turn_tool_loop.rs           | 2374 |
| BR-AG-STATE-001-EFFECT-AG-AG-TURN-075                                                                                                                                                                                                                   | EFFECT | BR-AG-STATE-001                                                                                                                                                                                                                                  | src-tauri/src/ag/turn_tool_loop.rs           | 2537 |
| BR-AG-STATE-005-EFFECT-AG-AG-TURN-079, BR-AG-OBS-001-EFFECT-AG-AG-TURN-079                                                                                                                                                                              | EFFECT | BR-AG-STATE-005, BR-AG-OBS-001                                                                                                                                                                                                                   | src-tauri/src/ag/turn_tool_loop.rs           | 2614 |
| BR-AG-STATE-002-RB-AG-AG-TOOL-018                                                                                                                                                                                                                       | RB     | BR-AG-STATE-002 term_ref: TERM-AG-014, TERM-AG-015, TERM-AG-016, TERM-AG-020                                                                                                                                                                     | src-tauri/src/ag/turn_tool_loop.rs           | 2677 |
| BR-AG-OBS-001-API-AG-AG-TURN-061, BR-AG-STATE-001-API-AG-AG-TURN-061                                                                                                                                                                                    | API    | BR-AG-OBS-001, BR-AG-STATE-001                                                                                                                                                                                                                   | src-tauri/src/ag/verification.rs             | 1156 |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-004                                                                                                                                                                                                                  | QUERY  | BR-AG-PERSIST-001 term_ref: TERM-AG-001                                                                                                                                                                                                          | src-tauri/src/de/routes.rs                   | 630  |
| BR-AG-OBS-001-DATA-AG-AG-TURN-043, BR-AG-STATE-001-DATA-AG-AG-TURN-043, BR-AG-STATE-002-DATA-AG-AG-TURN-043, BR-AG-PERSIST-001-DATA-AG-AG-TURN-043, BR-SYS-STATE-007-DATA-AG-AG-TURN-043                                                                | DATA   | BR-AG-OBS-001, BR-AG-STATE-001, BR-AG-STATE-002, BR-AG-PERSIST-001, BR-SYS-STATE-007 term_ref: TERM-AG-001, TERM-AG-002, TERM-AG-021, TERM-AG-022                                                                                                | src-tauri/src/ipc/mod.rs                     | 127  |
| BR-WS-STATE-003-EFFECT-WS-WS-OPERATION-008, BR-WS-PERSIST-003-EFFECT-WS-WS-OPERATION-008                                                                                                                                                                | EFFECT | BR-WS-STATE-003, BR-WS-PERSIST-003 term_ref: TERM-WS-003, TERM-WS-004                                                                                                                                                                            | src-tauri/src/workspace/operation.rs         | 2966 |

#### APP-CLOSE

| codes                                     | type   | rules            | 文件                                | 行号   |
| ----------------------------------------- | ------ | ---------------- | --------------------------------- | ---- |
| BR-SYS-STATE-006-EFFECT-SYS-APP-CLOSE-001 | EFFECT | BR-SYS-STATE-006 | src/services/closeService.ts      | 230  |
| BR-SYS-STATE-004-GUARD-SYS-APP-CLOSE-003  | GUARD  | BR-SYS-STATE-004 | src/shell/App.tsx                 | 38   |
| BR-SYS-STATE-004-GUARD-SYS-APP-CLOSE-004  | GUARD  | BR-SYS-STATE-004 | src-tauri/src/lib.rs              | 498  |
| BR-SYS-STATE-004-QUERY-SYS-APP-CLOSE-001  | QUERY  | BR-SYS-STATE-004 | src-tauri/src/workspace/routes.rs | 1184 |
| BR-SYS-STATE-004-EFFECT-SYS-APP-CLOSE-002 | EFFECT | BR-SYS-STATE-004 | src-tauri/src/workspace/routes.rs | 1248 |

#### AST-MEMORY

| codes                                                                              | type   | rules                                                   | 文件                           | 行号  |
| ---------------------------------------------------------------------------------- | ------ | ------------------------------------------------------- | ---------------------------- | --- |
| BR-AST-STATE-001-RB-AST-AST-MEMORY-001                                             | RB     | BR-AST-STATE-001                                        | src-tauri/src/ast/capture.rs | 56  |
| BR-AST-STATE-001-EFFECT-AST-AST-MEMORY-001                                         | EFFECT | BR-AST-STATE-001                                        | src-tauri/src/ast/capture.rs | 91  |
| BR-AST-STATE-001-EFFECT-AST-AST-MEMORY-002                                         | EFFECT | BR-AST-STATE-001                                        | src-tauri/src/ast/capture.rs | 135 |
| BR-SYS-DATA-007-API-AST-AST-MEMORY-001, BR-AST-DATA-001-API-AST-AST-MEMORY-001     | API    | BR-SYS-DATA-007, BR-AST-DATA-001 term_ref: TERM-AST-002 | src-tauri/src/ast/routes.rs  | 6   |
| BR-AST-DATA-001-QUERY-AST-AST-MEMORY-004                                           | QUERY  | BR-AST-DATA-001                                         | src-tauri/src/ast/store.rs   | 22  |
| BR-AST-DATA-001-QUERY-AST-AST-MEMORY-002                                           | QUERY  | BR-AST-DATA-001                                         | src-tauri/src/ast/store.rs   | 50  |
| BR-SYS-DATA-007-QUERY-AST-AST-MEMORY-003, BR-AST-DATA-001-QUERY-AST-AST-MEMORY-003 | QUERY  | BR-SYS-DATA-007, BR-AST-DATA-001 term_ref: TERM-AST-002 | src-tauri/src/ast/store.rs   | 72  |
| BR-AST-DATA-002-QUERY-AST-AST-MEMORY-001                                           | QUERY  | BR-AST-DATA-002                                         | src-tauri/src/ast/supply.rs  | 23  |

#### CBT-BUILD

| codes                                       | type   | rules              | 文件                            | 行号  |
| ------------------------------------------- | ------ | ------------------ | ----------------------------- | --- |
| BR-CBT-DATA-001-DATA-CBT-CBT-BUILD-001      | DATA   | BR-CBT-DATA-001    | src-tauri/src/cbt/apply.rs    | 12  |
| BR-CBT-DATA-001-DATA-CBT-CBT-BUILD-002      | DATA   | BR-CBT-DATA-001    | src-tauri/src/cbt/apply.rs    | 37  |
| BR-CBT-STATE-001-RB-CBT-CBT-BUILD-002       | RB     | BR-CBT-STATE-001   | src-tauri/src/cbt/build.rs    | 59  |
| BR-CBT-PERSIST-001-EFFECT-CBT-CBT-BUILD-001 | EFFECT | BR-CBT-PERSIST-001 | src-tauri/src/cbt/build.rs    | 99  |
| BR-CBT-PERSIST-001-QUERY-CBT-CBT-BUILD-001  | QUERY  | BR-CBT-PERSIST-001 | src-tauri/src/cbt/build.rs    | 128 |
| BR-CBT-STATE-002-GUARD-CBT-CBT-BUILD-001    | GUARD  | BR-CBT-STATE-002   | src-tauri/src/cbt/takeover.rs | 35  |
| BR-CBT-STATE-002-RB-CBT-CBT-BUILD-001       | RB     | BR-CBT-STATE-002   | src-tauri/src/cbt/takeover.rs | 54  |

#### CLOUD-AUTH

| codes                                       | type   | rules             | 文件                            | 行号 |
| ------------------------------------------- | ------ | ----------------- | ----------------------------- | -- |
| BR-ACCT-STATE-001-RB-ACCT-CLOUD-AUTH-001    | RB     | BR-ACCT-STATE-001 | src-tauri/src/acct/session.rs | 24 |
| BR-ACCT-STATE-001-GUARD-ACCT-CLOUD-AUTH-001 | GUARD  | BR-ACCT-STATE-001 | src-tauri/src/acct/session.rs | 38 |
| BR-ACCT-STATE-001-GUARD-ACCT-CLOUD-AUTH-002 | GUARD  | BR-ACCT-STATE-001 | src-tauri/src/acct/session.rs | 52 |
| BR-ACCT-STATE-001-GUARD-ACCT-CLOUD-AUTH-003 | GUARD  | BR-ACCT-STATE-001 | src-tauri/src/acct/session.rs | 70 |
| BR-ACCT-DATA-001-EFFECT-ACCT-CLOUD-AUTH-001 | EFFECT | BR-ACCT-DATA-001  | src-tauri/src/acct/token.rs   | 46 |
| BR-ACCT-DATA-001-EFFECT-ACCT-CLOUD-AUTH-002 | EFFECT | BR-ACCT-DATA-001  | src-tauri/src/acct/token.rs   | 60 |
| BR-ACCT-DATA-001-EFFECT-ACCT-CLOUD-AUTH-003 | EFFECT | BR-ACCT-DATA-001  | src-tauri/src/acct/token.rs   | 73 |
| BR-SYS-STATE-001-RB-SYS-CLOUD-AUTH-001      | RB     | BR-SYS-STATE-001  | src-tauri/src/session/mod.rs  | 40 |

#### CLOUD-GATE

| codes                                                                                | type  | rules                              | 文件                           | 行号  |
| ------------------------------------------------------------------------------------ | ----- | ---------------------------------- | ---------------------------- | --- |
| BR-ENT-VERIFY-001-GUARD-ENT-CLOUD-GATE-002                                           | GUARD | BR-ENT-VERIFY-001                  | src-tauri/src/ent/gateway.rs | 133 |
| BR-ENT-DATA-002-GUARD-ENT-CLOUD-GATE-001                                             | GUARD | BR-ENT-DATA-002                    | src-tauri/src/ent/usage.rs   | 34  |
| BR-SYS-VERIFY-001-GUARD-SYS-CLOUD-GATE-001, BR-SYS-DATA-001-GUARD-SYS-CLOUD-GATE-001 | GUARD | BR-SYS-VERIFY-001, BR-SYS-DATA-001 | src-tauri/src/session/mod.rs | 76  |

#### CLOUD-INVOKE

| codes                                                                                | type   | rules                              | 文件                           | 行号   |
| ------------------------------------------------------------------------------------ | ------ | ---------------------------------- | ---------------------------- | ---- |
| BR-ENT-DATA-001-EFFECT-ENT-CLOUD-INVOKE-001                                          | EFFECT | BR-ENT-DATA-001                    | src-tauri/src/ent/byok.rs    | 17   |
| BR-SYS-OBS-001-DATA-ENT-CLOUD-INVOKE-001                                             | DATA   | BR-SYS-OBS-001                     | src-tauri/src/ent/gateway.rs | 71   |
| BR-ENT-STATE-001-RB-ENT-CLOUD-INVOKE-001                                             | RB     | BR-ENT-STATE-001                   | src-tauri/src/ent/gateway.rs | 103  |
| BR-ENT-STATE-001-RB-ENT-CLOUD-INVOKE-002                                             | RB     | BR-ENT-STATE-001                   | src-tauri/src/ent/gateway.rs | 1324 |
| BR-SYS-VERIFY-001-GUARD-SYS-CLOUD-GATE-001, BR-SYS-DATA-001-GUARD-SYS-CLOUD-GATE-001 | GUARD  | BR-SYS-VERIFY-001, BR-SYS-DATA-001 | src-tauri/src/session/mod.rs | 76   |

#### DATA-MIGRATE

| codes                                                                                    | type   | rules                                                                            | 文件                                   | 行号  |
| ---------------------------------------------------------------------------------------- | ------ | -------------------------------------------------------------------------------- | ------------------------------------ | --- |
| BR-WS-PERSIST-002-EFFECT-WS-DATA-MIGRATE-005                                             | EFFECT | BR-WS-PERSIST-002                                                                | src/services/workspaceService.ts     | 193 |
| BR-WS-PERSIST-002-RB-SYS-DATA-MIGRATE-001                                                | RB     | BR-WS-PERSIST-002                                                                | src/shell/App.tsx                    | 18  |
| BR-WS-PERSIST-002-RB-WS-DATA-MIGRATE-001                                                 | RB     | BR-WS-PERSIST-002                                                                | src/shell/Welcome.tsx                | 18  |
| BR-WS-PERSIST-002-RB-WS-DATA-MIGRATE-002                                                 | RB     | BR-WS-PERSIST-002                                                                | src/shell/Workbench.tsx              | 29  |
| BR-WS-PERSIST-002-EFFECT-WS-DATA-MIGRATE-004                                             | EFFECT | BR-WS-PERSIST-002                                                                | src/store/workspaceShell.ts          | 46  |
| BR-AG-DATA-003-EFFECT-AG-AG-TARGETSET-002, BR-SYS-PERSIST-001-EFFECT-AG-AG-TARGETSET-002 | EFFECT | BR-AG-DATA-003, BR-SYS-PERSIST-001 term_ref: TERM-AG-019, TERM-AG-015            | src-tauri/src/ag/target_set.rs       | 202 |
| BR-SYS-PERSIST-001-EFFECT-SYS-DATA-MIGRATE-001                                           | EFFECT | BR-SYS-PERSIST-001                                                               | src-tauri/src/db/mod.rs              | 526 |
| BR-SYS-STATE-005-EFFECT-SYS-DATA-MIGRATE-002                                             | EFFECT | BR-SYS-STATE-005                                                                 | src-tauri/src/de/lifecycle.rs        | 199 |
| BR-SYS-STATE-005-EFFECT-SYS-DATA-MIGRATE-003                                             | EFFECT | BR-SYS-STATE-005                                                                 | src-tauri/src/de/lifecycle.rs        | 237 |
| BR-SYS-DATA-002-DATA-SYS-DATA-MIGRATE-001, BR-SYS-DATA-008-DATA-SYS-DATA-MIGRATE-001     | DATA   | BR-SYS-DATA-002, BR-SYS-DATA-008 term_ref: TERM-AG-020, TERM-AG-021, TERM-AG-023 | src-tauri/src/types/mod.rs           | 9   |
| BR-WS-PERSIST-002-EFFECT-WS-DATA-MIGRATE-001                                             | EFFECT | BR-WS-PERSIST-002                                                                | src-tauri/src/workspace/lifecycle.rs | 34  |
| BR-WS-PERSIST-002-EFFECT-WS-DATA-MIGRATE-002                                             | EFFECT | BR-WS-PERSIST-002                                                                | src-tauri/src/workspace/lifecycle.rs | 71  |
| BR-WS-PERSIST-002-EFFECT-WS-DATA-MIGRATE-003                                             | EFFECT | BR-WS-PERSIST-002                                                                | src-tauri/src/workspace/lifecycle.rs | 99  |
| BR-WS-PERSIST-002-QUERY-WS-DATA-MIGRATE-001                                              | QUERY  | BR-WS-PERSIST-002                                                                | src-tauri/src/workspace/lifecycle.rs | 116 |
| BR-WS-PERSIST-002-QUERY-WS-DATA-MIGRATE-002                                              | QUERY  | BR-WS-PERSIST-002                                                                | src-tauri/src/workspace/lifecycle.rs | 140 |

#### DE-DIFF

| codes                                                                                                                                                         | type   | rules                                                                                                       | 文件                                     | 行号   |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ----------------------------------------------------------------------------------------------------------- | -------------------------------------- | ---- |
| BR-DE-DATA-002-RB-DE-DE-DIFF-003, BR-DE-DATA-001-RB-DE-DE-DIFF-003, BR-DE-STATE-001-RB-DE-DE-DIFF-003                                                         | RB     | BR-DE-DATA-001, BR-DE-DATA-002, BR-DE-STATE-001 term_ref: TERM-DE-003, TERM-DE-005, TERM-ED-008             | src/components/de/DiffCard.tsx         | 241  |
| BR-CORE-STATE-001-EFFECT-ED-ED-EDIT-001, BR-CORE-STATE-003-EFFECT-ED-ED-EDIT-001, BR-DE-DATA-001-EFFECT-ED-ED-EDIT-001, BR-DE-STATE-001-EFFECT-ED-ED-EDIT-001 | EFFECT | BR-CORE-STATE-001, BR-CORE-STATE-003, BR-DE-DATA-001, BR-DE-STATE-001                                       | src/components/ed/Editor.tsx           | 1725 |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-006, BR-AG-DATA-001-EFFECT-AG-AG-TURN-006                                                                                  | EFFECT | BR-SYS-STATE-008, BR-AG-DATA-001 term_ref: TERM-AG-015, TERM-ED-008                                         | src/services/agService.ts              | 1535 |
| BR-SYS-STATE-008-EFFECT-DE-DE-DIFF-008                                                                                                                        | EFFECT | BR-SYS-STATE-008                                                                                            | src/services/deService.ts              | 126  |
| BR-DE-DATA-002-EFFECT-DE-DE-DIFF-009                                                                                                                          | EFFECT | BR-DE-DATA-002 term_ref: TERM-DE-004, TERM-ED-008                                                           | src/services/deService.ts              | 165  |
| BR-CORE-STATE-001-EFFECT-DE-ED-EDIT-010                                                                                                                       | EFFECT | BR-CORE-STATE-001                                                                                           | src/services/deService.ts              | 238  |
| BR-CORE-STATE-003-EFFECT-DE-ED-EDIT-011                                                                                                                       | EFFECT | BR-CORE-STATE-003                                                                                           | src/services/deService.ts              | 297  |
| BR-CORE-STATE-001-API-DE-DE-DIFF-001, BR-CORE-STATE-002-API-DE-DE-DIFF-001, BR-CORE-STATE-003-API-DE-DE-DIFF-001                                              | API    | BR-CORE-STATE-001, BR-CORE-STATE-002, BR-CORE-STATE-003                                                     | src/services/deService.ts              | 354  |
| BR-DE-DATA-002-RB-ED-DE-DIFF-028                                                                                                                              | RB     | BR-DE-DATA-002 term_ref: TERM-DE-004, TERM-ED-008                                                           | src/services/edImageOps.ts             | 493  |
| BR-CORE-STATE-001-EFFECT-ED-WS-SAVE-001, BR-CORE-STATE-003-EFFECT-ED-WS-SAVE-001                                                                              | EFFECT | BR-CORE-STATE-001, BR-CORE-STATE-003                                                                        | src/services/edService.ts              | 91   |
| BR-ED-DATA-007-RB-ED-ED-EDIT-018                                                                                                                              | RB     | BR-ED-DATA-007 term_ref: TERM-ED-007, TERM-DOC-005                                                          | src/services/edTableOps.ts             | 407  |
| BR-ED-DATA-007-EFFECT-ED-ED-EDIT-014                                                                                                                          | EFFECT | BR-ED-DATA-007 term_ref: TERM-ED-007, TERM-DOC-005                                                          | src/services/edTableOps.ts             | 438  |
| BR-DE-DATA-001-RB-DE-DE-DIFF-007, BR-DE-STATE-001-RB-DE-DE-DIFF-007                                                                                           | RB     | BR-DE-DATA-001, BR-DE-STATE-001 term_ref: TERM-DE-001, TERM-DE-005                                          | src/store/deStore.ts                   | 222  |
| BR-DE-STATE-001-RB-DE-DE-DIFF-002                                                                                                                             | RB     | BR-DE-STATE-001 term_ref: TERM-DE-001, TERM-DE-003                                                          | src/store/deStore.ts                   | 320  |
| BR-DE-DATA-001-RB-DE-DE-DIFF-006                                                                                                                              | RB     | BR-DE-DATA-001 term_ref: TERM-DE-005                                                                        | src-tauri/src/ag/block_ops.rs          | 579  |
| BR-DE-DATA-002-DATA-AG-AG-TOOL-060, BR-DE-DATA-001-DATA-AG-AG-TOOL-060                                                                                        | DATA   | BR-DE-DATA-002, BR-DE-DATA-001                                                                              | src-tauri/src/ag/edit_projection.rs    | 12   |
| BR-AG-STATE-002-EFFECT-AG-AG-TOOL-047, BR-AG-DATA-003-EFFECT-AG-AG-TOOL-047                                                                                   | EFFECT | BR-AG-STATE-002, BR-AG-DATA-003                                                                             | src-tauri/src/ag/edit_runtime.rs       | 18   |
| BR-AG-STATE-002-EFFECT-AG-AG-TOOL-017, BR-AG-DATA-003-EFFECT-AG-AG-TOOL-017, BR-DE-DATA-001-EFFECT-AG-AG-TOOL-017, BR-DE-DATA-002-EFFECT-AG-AG-TOOL-017       | EFFECT | BR-AG-STATE-002, BR-AG-DATA-003, BR-DE-DATA-001, BR-DE-DATA-002                                             | src-tauri/src/ag/edit_runtime.rs       | 73   |
| BR-ED-DATA-007-GUARD-AG-AG-TOOL-018                                                                                                                           | GUARD  | BR-ED-DATA-007 term_ref: TERM-ED-007                                                                        | src-tauri/src/ag/edit_table_runtime.rs | 8    |
| BR-AG-DATA-003-GUARD-AG-AG-TARGETSET-003                                                                                                                      | GUARD  | BR-AG-DATA-003 term_ref: TERM-AG-019, TERM-DE-002                                                           | src-tauri/src/ag/target_set.rs         | 272  |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-032, BR-AG-DATA-001-EFFECT-AG-AG-TURN-032                                                                                  | EFFECT | BR-SYS-STATE-008, BR-AG-DATA-001 term_ref: TERM-AG-015, TERM-ED-008                                         | src-tauri/src/ag/task.rs               | 110  |
| BR-SYS-STATE-008-GUARD-AG-AG-TURN-005, BR-AG-DATA-001-GUARD-AG-AG-TURN-005                                                                                    | GUARD  | BR-SYS-STATE-008, BR-AG-DATA-001 term_ref: TERM-AG-015, TERM-ED-008                                         | src-tauri/src/ag/task.rs               | 169  |
| BR-SYS-STATE-008-API-AG-AG-TURN-003, BR-AG-DATA-001-API-AG-AG-TURN-003                                                                                        | API    | BR-SYS-STATE-008, BR-AG-DATA-001 term_ref: TERM-AG-015, TERM-ED-008                                         | src-tauri/src/ag/turn_control.rs       | 654  |
| BR-DE-STATE-002-GUARD-DE-DE-DIFF-001                                                                                                                          | GUARD  | BR-DE-STATE-002                                                                                             | src-tauri/src/de/expiry.rs             | 34   |
| BR-DE-DATA-003-GUARD-DE-DE-DIFF-003                                                                                                                           | GUARD  | BR-DE-DATA-003                                                                                              | src-tauri/src/de/expiry.rs             | 62   |
| BR-DE-DATA-002-DATA-DE-DE-DIFF-001                                                                                                                            | DATA   | BR-DE-DATA-002                                                                                              | src-tauri/src/de/generate.rs           | 41   |
| BR-AG-DATA-003-GUARD-DE-DE-DIFF-004                                                                                                                           | GUARD  | BR-AG-DATA-003 term_ref: TERM-AG-019                                                                        | src-tauri/src/de/generate.rs           | 61   |
| BR-DE-DATA-001-GUARD-DE-DE-DIFF-005                                                                                                                           | GUARD  | BR-DE-DATA-001                                                                                              | src-tauri/src/de/generate.rs           | 87   |
| BR-DE-STATE-002-QUERY-DE-DE-DIFF-001                                                                                                                          | QUERY  | BR-DE-STATE-002                                                                                             | src-tauri/src/de/lifecycle.rs          | 31   |
| BR-DE-STATE-002-GUARD-DE-DE-DIFF-002                                                                                                                          | GUARD  | BR-DE-STATE-002                                                                                             | src-tauri/src/de/lifecycle.rs          | 58   |
| BR-DE-DATA-003-GUARD-DE-DE-DIFF-006                                                                                                                           | GUARD  | BR-DE-DATA-003                                                                                              | src-tauri/src/de/lifecycle.rs          | 86   |
| BR-DE-STATE-002-EFFECT-DE-DE-DIFF-001                                                                                                                         | EFFECT | BR-DE-STATE-002                                                                                             | src-tauri/src/de/lifecycle.rs          | 108  |
| BR-SYS-DATA-005-EFFECT-DE-DE-DIFF-003                                                                                                                         | EFFECT | BR-SYS-DATA-005                                                                                             | src-tauri/src/de/lifecycle.rs          | 132  |
| BR-SYS-DATA-005-EFFECT-DE-DE-DIFF-004                                                                                                                         | EFFECT | BR-SYS-DATA-005                                                                                             | src-tauri/src/de/lifecycle.rs          | 155  |
| BR-DE-STATE-001-RB-DE-DE-DIFF-001                                                                                                                             | RB     | BR-DE-STATE-001                                                                                             | src-tauri/src/de/mod.rs                | 71   |
| BR-CORE-STATE-001-EFFECT-DE-DE-DIFF-005                                                                                                                       | EFFECT | BR-CORE-STATE-001                                                                                           | src-tauri/src/de/routes.rs             | 123  |
| BR-CORE-STATE-003-EFFECT-DE-DE-DIFF-006                                                                                                                       | EFFECT | BR-CORE-STATE-003                                                                                           | src-tauri/src/de/routes.rs             | 252  |
| BR-CORE-STATE-002-EFFECT-DE-DE-DIFF-007                                                                                                                       | EFFECT | BR-CORE-STATE-002                                                                                           | src-tauri/src/de/routes.rs             | 285  |
| BR-DE-DATA-002-IO-DE-DE-DIFF-002                                                                                                                              | IO     | BR-DE-DATA-002 term_ref: TERM-DE-004                                                                        | src-tauri/src/de/store.rs              | 31   |
| BR-DE-DATA-001-IO-DE-DE-DIFF-006, BR-DE-DATA-002-IO-DE-DE-DIFF-006                                                                                            | IO     | BR-DE-DATA-001, BR-DE-DATA-002 term_ref: TERM-DE-001, TERM-DE-004, TERM-DE-005                              | src-tauri/src/de/store.rs              | 66   |
| BR-DE-DATA-002-QUERY-DE-DE-DIFF-002                                                                                                                           | QUERY  | BR-DE-DATA-002 term_ref: TERM-DE-004                                                                        | src-tauri/src/de/store.rs              | 101  |
| BR-DE-DATA-002-EFFECT-WS-DE-DIFF-004                                                                                                                          | EFFECT | BR-DE-DATA-002 term_ref: TERM-DE-004, TERM-ED-008                                                           | src-tauri/src/workspace/diff_apply.rs  | 232  |
| BR-WS-STATE-003-API-WS-WS-OPERATION-004, BR-WS-DATA-003-API-WS-WS-OPERATION-004, BR-WS-OBS-002-API-WS-WS-OPERATION-004                                        | API    | BR-WS-STATE-003, BR-WS-DATA-003, BR-WS-OBS-002 term_ref: TERM-WS-003, TERM-WS-007, TERM-ED-008, TERM-DE-003 | src-tauri/src/workspace/routes.rs      | 1861 |

#### DOC-CONVERT

| codes                                                                               | type  | rules                                                              | 文件                                       | 行号   |
| ----------------------------------------------------------------------------------- | ----- | ------------------------------------------------------------------ | ---------------------------------------- | ---- |
| BR-DOC-DATA-003-DATA-DOC-DOC-CONVERT-011                                            | DATA  | BR-DOC-DATA-003                                                    | src-tauri/src/doc/block_backfill.rs      | 26   |
| BR-DOC-DATA-003-DATA-DOC-DOC-CONVERT-003                                            | DATA  | BR-DOC-DATA-003                                                    | src-tauri/src/doc/block_backfill.rs      | 212  |
| BR-DOC-DATA-003-GUARD-DOC-DOC-CONVERT-015                                           | GUARD | BR-DOC-DATA-003                                                    | src-tauri/src/doc/block_backfill.rs      | 259  |
| BR-DOC-STATE-001-RB-DOC-DOC-CONVERT-001                                             | RB    | BR-DOC-STATE-001                                                   | src-tauri/src/doc/convert.rs             | 56   |
| BR-DOC-DATA-001-GUARD-DOC-DOC-CONVERT-001                                           | GUARD | BR-DOC-DATA-001                                                    | src-tauri/src/doc/convert.rs             | 89   |
| BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-008                                            | DATA  | BR-DOC-DATA-002 term_ref: TERM-DOC-005                             | src-tauri/src/doc/docx_cell_shading.rs   | 16   |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-006                                              | IO    | BR-DOC-DATA-002 term_ref: TERM-DOC-005                             | src-tauri/src/doc/docx_cell_shading.rs   | 427  |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-007                                              | IO    | BR-DOC-DATA-002 term_ref: TERM-DOC-005                             | src-tauri/src/doc/docx_cell_shading.rs   | 487  |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-016, BR-DOC-DATA-003-IO-DOC-DOC-CONVERT-016      | IO    | BR-DOC-DATA-002, BR-DOC-DATA-003                                   | src-tauri/src/doc/edit_chain.rs          | 88   |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-004                                              | IO    | BR-DOC-DATA-002                                                    | src-tauri/src/doc/edit_chain.rs          | 157  |
| BR-DOC-DATA-001-DATA-DOC-DOC-CONVERT-006                                            | DATA  | BR-DOC-DATA-001                                                    | src-tauri/src/doc/font_trace.rs          | 243  |
| BR-DOC-DATA-001-DATA-DOC-DOC-CONVERT-007                                            | DATA  | BR-DOC-DATA-001                                                    | src-tauri/src/doc/font_trace.rs          | 318  |
| BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-009, BR-DOC-DATA-003-DATA-DOC-DOC-CONVERT-009  | DATA  | BR-DOC-DATA-002, BR-DOC-DATA-003                                   | src-tauri/src/doc/image_media.rs         | 37   |
| BR-DOC-DATA-002-GUARD-DOC-DOC-CONVERT-011, BR-WS-DATA-001-GUARD-DOC-DOC-CONVERT-011 | GUARD | BR-DOC-DATA-002, BR-WS-DATA-001 term_ref: TERM-ED-008, TERM-WS-001 | src-tauri/src/doc/image_media.rs         | 334  |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-015, BR-WS-DATA-001-IO-DOC-DOC-CONVERT-015       | IO    | BR-DOC-DATA-002, BR-WS-DATA-001 term_ref: TERM-ED-008, TERM-WS-001 | src-tauri/src/doc/image_media.rs         | 568  |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-012                                              | IO    | BR-DOC-DATA-002 term_ref: TERM-ED-008                              | src-tauri/src/doc/image_media.rs         | 670  |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-013, BR-DOC-DATA-003-IO-DOC-DOC-CONVERT-013      | IO    | BR-DOC-DATA-002, BR-DOC-DATA-003 term_ref: TERM-ED-008             | src-tauri/src/doc/image_media.rs         | 766  |
| BR-DOC-DATA-002-GUARD-DOC-DOC-CONVERT-014                                           | GUARD | BR-DOC-DATA-002 term_ref: TERM-ED-008                              | src-tauri/src/doc/image_media.rs         | 1083 |
| BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-003                                              | IO    | BR-DOC-DATA-001                                                    | src-tauri/src/doc/libreoffice_runtime.rs | 77   |
| BR-DOC-DATA-001-DATA-DOC-DOC-CONVERT-004                                            | DATA  | BR-DOC-DATA-001                                                    | src-tauri/src/doc/libreoffice_runtime.rs | 106  |
| BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-005                                              | IO    | BR-DOC-DATA-001                                                    | src-tauri/src/doc/libreoffice_runtime.rs | 244  |
| BR-DOC-DATA-001-DATA-DOC-DOC-CONVERT-012                                            | DATA  | BR-DOC-DATA-001                                                    | src-tauri/src/doc/pandoc_runtime.rs      | 32   |
| BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-001                                              | IO    | BR-DOC-DATA-001                                                    | src-tauri/src/doc/pandoc_runtime.rs      | 62   |
| BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-002                                              | IO    | BR-DOC-DATA-001                                                    | src-tauri/src/doc/pandoc_runtime.rs      | 88   |
| BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-001                                            | DATA  | BR-DOC-DATA-002                                                    | src-tauri/src/doc/style_subset.rs        | 53   |
| BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-002                                            | DATA  | BR-DOC-DATA-002                                                    | src-tauri/src/doc/style_subset.rs        | 79   |
| BR-DOC-DATA-001-GUARD-DOC-DOC-CONVERT-002                                           | GUARD | BR-DOC-DATA-001                                                    | src-tauri/src/doc/tool_runtime.rs        | 118  |

#### ED-DECOR

| codes                                                                                                                                                         | type   | rules                                                                 | 文件                                                      | 行号   |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | --------------------------------------------------------------------- | ------------------------------------------------------- | ---- |
| BR-CORE-STATE-001-EFFECT-ED-ED-EDIT-001, BR-CORE-STATE-003-EFFECT-ED-ED-EDIT-001, BR-DE-DATA-001-EFFECT-ED-ED-EDIT-001, BR-DE-STATE-001-EFFECT-ED-ED-EDIT-001 | EFFECT | BR-CORE-STATE-001, BR-CORE-STATE-003, BR-DE-DATA-001, BR-DE-STATE-001 | src/components/ed/Editor.tsx                            | 1725 |
| BR-ED-STATE-002-RB-ED-ED-DECOR-001                                                                                                                            | RB     | BR-ED-STATE-002 term_ref: TERM-ED-002, TERM-ED-003                    | src/components/ed/extensions/DiffDecorationExtension.ts | 155  |
| BR-CORE-STATE-003-EFFECT-DE-ED-EDIT-011                                                                                                                       | EFFECT | BR-CORE-STATE-003                                                     | src/services/deService.ts                               | 297  |
| BR-CORE-STATE-003-EFFECT-DE-DE-DIFF-006                                                                                                                       | EFFECT | BR-CORE-STATE-003                                                     | src-tauri/src/de/routes.rs                              | 252  |

#### ED-EDIT

| codes                                                                                                                                                         | type   | rules                                                                                                       | 文件                                               | 行号   |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ----------------------------------------------------------------------------------------------------------- | ------------------------------------------------ | ---- |
| BR-CORE-STATE-001-EFFECT-ED-ED-EDIT-001, BR-CORE-STATE-003-EFFECT-ED-ED-EDIT-001, BR-DE-DATA-001-EFFECT-ED-ED-EDIT-001, BR-DE-STATE-001-EFFECT-ED-ED-EDIT-001 | EFFECT | BR-CORE-STATE-001, BR-CORE-STATE-003, BR-DE-DATA-001, BR-DE-STATE-001                                       | src/components/ed/Editor.tsx                     | 1725 |
| BR-ED-DATA-001-RB-ED-ED-EDIT-024, BR-ED-DATA-003-RB-ED-ED-EDIT-024                                                                                            | RB     | BR-ED-DATA-001, BR-ED-DATA-003                                                                              | src/components/ed/ImageMoveControls.tsx          | 39   |
| BR-ED-DATA-007-RB-ED-ED-EDIT-019                                                                                                                              | RB     | BR-ED-DATA-007 term_ref: TERM-ED-007                                                                        | src/components/ed/TableMoveControls.tsx          | 47   |
| BR-ED-DATA-001-DATA-ED-ED-EDIT-001                                                                                                                            | DATA   | BR-ED-DATA-001 term_ref: TERM-ED-001                                                                        | src/components/ed/extensions/BlockIdExtension.ts | 96   |
| BR-ED-DATA-003-GUARD-ED-ED-EDIT-019                                                                                                                           | GUARD  | BR-ED-DATA-003                                                                                              | src/components/ed/extensions/Pagination.ts       | 263  |
| BR-ED-DATA-003-GUARD-ED-ED-EDIT-020                                                                                                                           | GUARD  | BR-ED-DATA-003                                                                                              | src/components/ed/extensions/Pagination.ts       | 585  |
| BR-ED-DATA-003-GUARD-ED-ED-EDIT-018                                                                                                                           | GUARD  | BR-ED-DATA-003                                                                                              | src/components/ed/extensions/Pagination.ts       | 636  |
| BR-ED-DATA-003-RB-ED-ED-EDIT-005                                                                                                                              | RB     | BR-ED-DATA-003                                                                                              | src/components/ed/extensions/Pagination.ts       | 893  |
| BR-ED-DATA-003-EFFECT-ED-ED-EDIT-020, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-020                                                                                   | EFFECT | BR-ED-DATA-003, BR-ED-STATE-001 term_ref: TERM-ED-008                                                       | src/components/ed/extensions/ResizableImage.ts   | 50   |
| BR-ED-DATA-007-DATA-ED-ED-EDIT-018                                                                                                                            | DATA   | BR-ED-DATA-007 term_ref: TERM-ED-007, TERM-DOC-005                                                          | src/components/ed/extensions/TableCellShading.ts | 37   |
| BR-ED-DATA-003-RB-ED-ED-EDIT-017                                                                                                                              | RB     | BR-ED-DATA-003                                                                                              | src/components/ed/extensions/paginationFlow.ts   | 57   |
| BR-ED-DATA-006-DATA-ED-ED-EDIT-014                                                                                                                            | DATA   | BR-ED-DATA-006                                                                                              | src/components/ed/markdown/markdownCodec.ts      | 82   |
| BR-ED-VERIFY-001-GUARD-ED-ED-EDIT-015, BR-ED-DATA-005-GUARD-ED-ED-EDIT-015                                                                                    | GUARD  | BR-ED-VERIFY-001, BR-ED-DATA-005                                                                            | src/components/ed/markdown/markdownCodec.ts      | 170  |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-006, BR-AG-DATA-001-EFFECT-AG-AG-TURN-006                                                                                  | EFFECT | BR-SYS-STATE-008, BR-AG-DATA-001 term_ref: TERM-AG-015, TERM-ED-008                                         | src/services/agService.ts                        | 1535 |
| BR-SYS-STATE-008-EFFECT-DE-DE-DIFF-008                                                                                                                        | EFFECT | BR-SYS-STATE-008                                                                                            | src/services/deService.ts                        | 126  |
| BR-CORE-STATE-001-EFFECT-DE-ED-EDIT-010                                                                                                                       | EFFECT | BR-CORE-STATE-001                                                                                           | src/services/deService.ts                        | 238  |
| BR-CORE-STATE-003-EFFECT-DE-ED-EDIT-011                                                                                                                       | EFFECT | BR-CORE-STATE-003                                                                                           | src/services/deService.ts                        | 297  |
| BR-CORE-STATE-001-API-DE-DE-DIFF-001, BR-CORE-STATE-002-API-DE-DE-DIFF-001, BR-CORE-STATE-003-API-DE-DE-DIFF-001                                              | API    | BR-CORE-STATE-001, BR-CORE-STATE-002, BR-CORE-STATE-003                                                     | src/services/deService.ts                        | 354  |
| BR-ED-DATA-003-RB-ED-ED-EDIT-006                                                                                                                              | RB     | BR-ED-DATA-003                                                                                              | src/services/docService.ts                       | 107  |
| BR-ED-DATA-003-RB-ED-ED-EDIT-026                                                                                                                              | RB     | BR-ED-DATA-003 term_ref: TERM-ED-008                                                                        | src/services/docService.ts                       | 184  |
| BR-ED-DATA-003-EFFECT-ED-ED-EDIT-026                                                                                                                          | EFFECT | BR-ED-DATA-003 term_ref: TERM-ED-008                                                                        | src/services/docService.ts                       | 203  |
| BR-ED-DATA-001-QUERY-ED-ED-EDIT-024, BR-ED-DATA-003-QUERY-ED-ED-EDIT-024                                                                                      | QUERY  | BR-ED-DATA-001, BR-ED-DATA-003 term_ref: TERM-ED-008                                                        | src/services/edImageOps.ts                       | 228  |
| BR-SYS-DATA-006-RB-ED-ED-EDIT-027, BR-ED-DATA-003-RB-ED-ED-EDIT-027                                                                                           | RB     | BR-SYS-DATA-006, BR-ED-DATA-003 term_ref: TERM-SYS-002, TERM-ED-008                                         | src/services/edImageOps.ts                       | 256  |
| BR-ED-DATA-001-QUERY-ED-ED-EDIT-021, BR-ED-DATA-003-QUERY-ED-ED-EDIT-021                                                                                      | QUERY  | BR-ED-DATA-001, BR-ED-DATA-003                                                                              | src/services/edImageOps.ts                       | 570  |
| BR-ED-DATA-001-EFFECT-ED-ED-EDIT-022, BR-ED-DATA-003-EFFECT-ED-ED-EDIT-022, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-022                                             | EFFECT | BR-ED-DATA-001, BR-ED-DATA-003, BR-ED-STATE-001                                                             | src/services/edImageOps.ts                       | 599  |
| BR-ED-DATA-001-EFFECT-ED-ED-EDIT-023, BR-ED-DATA-003-EFFECT-ED-ED-EDIT-023, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-023                                             | EFFECT | BR-ED-DATA-001, BR-ED-DATA-003, BR-ED-STATE-001                                                             | src/services/edImageOps.ts                       | 640  |
| BR-ED-DATA-001-EFFECT-ED-ED-EDIT-025, BR-ED-DATA-003-EFFECT-ED-ED-EDIT-025, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-025                                             | EFFECT | BR-ED-DATA-001, BR-ED-DATA-003, BR-ED-STATE-001 term_ref: TERM-ED-008                                       | src/services/edImageOps.ts                       | 731  |
| BR-ED-DATA-004-QUERY-ED-ED-EDIT-010                                                                                                                           | QUERY  | BR-ED-DATA-004 term_ref: TERM-ED-004, TERM-ED-005                                                           | src/services/edLocate.ts                         | 231  |
| BR-ED-STATE-001-EFFECT-ED-ED-EDIT-012                                                                                                                         | EFFECT | BR-ED-STATE-001 term_ref: TERM-ED-001, TERM-ED-004                                                          | src/services/edLocate.ts                         | 345  |
| BR-ED-STATE-001-EFFECT-ED-ED-EDIT-011                                                                                                                         | EFFECT | BR-ED-STATE-001 term_ref: TERM-ED-001                                                                       | src/services/edLocate.ts                         | 384  |
| BR-ED-DATA-004-QUERY-ED-ED-EDIT-007                                                                                                                           | QUERY  | BR-ED-DATA-004 term_ref: TERM-ED-004, TERM-ED-005                                                           | src/services/edLocate.ts                         | 414  |
| BR-CORE-STATE-001-EFFECT-ED-WS-SAVE-001, BR-CORE-STATE-003-EFFECT-ED-WS-SAVE-001                                                                              | EFFECT | BR-CORE-STATE-001, BR-CORE-STATE-003                                                                        | src/services/edService.ts                        | 91   |
| BR-ED-DATA-002-RB-ED-ED-EDIT-012, BR-ED-DATA-006-RB-ED-ED-EDIT-012                                                                                            | RB     | BR-ED-DATA-002, BR-ED-DATA-006                                                                              | src/services/edService.ts                        | 270  |
| BR-ED-STATE-003-EFFECT-ED-ED-EDIT-013                                                                                                                         | EFFECT | BR-ED-STATE-003                                                                                             | src/services/edService.ts                        | 327  |
| BR-ED-VERIFY-001-GUARD-ED-ED-EDIT-016, BR-ED-DATA-006-GUARD-ED-ED-EDIT-016                                                                                    | GUARD  | BR-ED-VERIFY-001, BR-ED-DATA-006                                                                            | src/services/edService.ts                        | 361  |
| BR-ED-DATA-005-RB-ED-ED-EDIT-008                                                                                                                              | RB     | BR-ED-DATA-005 term_ref: TERM-ED-006, TERM-ED-004                                                           | src/services/edStyleOps.ts                       | 243  |
| BR-ED-DATA-007-RB-ED-ED-EDIT-018                                                                                                                              | RB     | BR-ED-DATA-007 term_ref: TERM-ED-007, TERM-DOC-005                                                          | src/services/edTableOps.ts                       | 407  |
| BR-ED-DATA-007-EFFECT-ED-ED-EDIT-014                                                                                                                          | EFFECT | BR-ED-DATA-007 term_ref: TERM-ED-007, TERM-DOC-005                                                          | src/services/edTableOps.ts                       | 438  |
| BR-DE-DATA-001-RB-DE-DE-DIFF-007, BR-DE-STATE-001-RB-DE-DE-DIFF-007                                                                                           | RB     | BR-DE-DATA-001, BR-DE-STATE-001 term_ref: TERM-DE-001, TERM-DE-005                                          | src/store/deStore.ts                             | 222  |
| BR-ED-STATE-001-RB-ED-ED-EDIT-002                                                                                                                             | RB     | BR-ED-STATE-001                                                                                             | src/store/edStore.ts                             | 262  |
| BR-ED-DATA-005-QUERY-ED-ED-EDIT-009                                                                                                                           | QUERY  | BR-ED-DATA-005 term_ref: TERM-ED-006, TERM-ED-004                                                           | src-tauri/src/ag/blocks.rs                       | 1010 |
| BR-AG-STATE-002-EFFECT-AG-AG-TOOL-017, BR-AG-DATA-003-EFFECT-AG-AG-TOOL-017, BR-DE-DATA-001-EFFECT-AG-AG-TOOL-017, BR-DE-DATA-002-EFFECT-AG-AG-TOOL-017       | EFFECT | BR-AG-STATE-002, BR-AG-DATA-003, BR-DE-DATA-001, BR-DE-DATA-002                                             | src-tauri/src/ag/edit_runtime.rs                 | 73   |
| BR-ED-DATA-007-GUARD-AG-AG-TOOL-018                                                                                                                           | GUARD  | BR-ED-DATA-007 term_ref: TERM-ED-007                                                                        | src-tauri/src/ag/edit_table_runtime.rs           | 8    |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-032, BR-AG-DATA-001-EFFECT-AG-AG-TURN-032                                                                                  | EFFECT | BR-SYS-STATE-008, BR-AG-DATA-001 term_ref: TERM-AG-015, TERM-ED-008                                         | src-tauri/src/ag/task.rs                         | 110  |
| BR-SYS-STATE-008-GUARD-AG-AG-TURN-005, BR-AG-DATA-001-GUARD-AG-AG-TURN-005                                                                                    | GUARD  | BR-SYS-STATE-008, BR-AG-DATA-001 term_ref: TERM-AG-015, TERM-ED-008                                         | src-tauri/src/ag/task.rs                         | 169  |
| BR-SYS-STATE-008-GUARD-AG-AG-TURN-006, BR-AG-DATA-001-GUARD-AG-AG-TURN-006                                                                                    | GUARD  | BR-SYS-STATE-008, BR-AG-DATA-001 term_ref: TERM-AG-015, TERM-ED-008                                         | src-tauri/src/ag/task.rs                         | 287  |
| BR-SYS-STATE-008-API-AG-AG-TURN-003, BR-AG-DATA-001-API-AG-AG-TURN-003                                                                                        | API    | BR-SYS-STATE-008, BR-AG-DATA-001 term_ref: TERM-AG-015, TERM-ED-008                                         | src-tauri/src/ag/turn_control.rs                 | 654  |
| BR-CORE-STATE-001-EFFECT-DE-DE-DIFF-005                                                                                                                       | EFFECT | BR-CORE-STATE-001                                                                                           | src-tauri/src/de/routes.rs                       | 123  |
| BR-CORE-STATE-003-EFFECT-DE-DE-DIFF-006                                                                                                                       | EFFECT | BR-CORE-STATE-003                                                                                           | src-tauri/src/de/routes.rs                       | 252  |
| BR-WS-STATE-003-API-WS-WS-OPERATION-004, BR-WS-DATA-003-API-WS-WS-OPERATION-004, BR-WS-OBS-002-API-WS-WS-OPERATION-004                                        | API    | BR-WS-STATE-003, BR-WS-DATA-003, BR-WS-OBS-002 term_ref: TERM-WS-003, TERM-WS-007, TERM-ED-008, TERM-DE-003 | src-tauri/src/workspace/routes.rs                | 1861 |

#### IPC-CALL

| codes                                                                          | type   | rules                                                                 | 文件                          | 行号   |
| ------------------------------------------------------------------------------ | ------ | --------------------------------------------------------------------- | --------------------------- | ---- |
| BR-SYS-STATE-007-EFFECT-AG-AG-TURN-060, BR-SYS-STATE-008-EFFECT-AG-AG-TURN-060 | EFFECT | BR-SYS-STATE-007, BR-SYS-STATE-008 term_ref: TERM-AG-017, TERM-AG-001 | src/services/agService.ts   | 1650 |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-068, BR-SYS-STATE-007-QUERY-AG-AG-TURN-068  | QUERY  | BR-AG-PERSIST-001, BR-SYS-STATE-007                                   | src/services/agService.ts   | 1889 |
| BR-SYS-DATA-007-API-AST-AST-MEMORY-001, BR-AST-DATA-001-API-AST-AST-MEMORY-001 | API    | BR-SYS-DATA-007, BR-AST-DATA-001 term_ref: TERM-AST-002               | src-tauri/src/ast/routes.rs | 6    |
| BR-SYS-OBS-002-DATA-SYS-IPC-CALL-002                                           | DATA   | BR-SYS-OBS-002                                                        | src-tauri/src/ipc/mod.rs    | 15   |
| BR-SYS-STATE-002-DATA-SYS-IPC-CALL-003                                         | DATA   | BR-SYS-STATE-002                                                      | src-tauri/src/ipc/mod.rs    | 77   |
| BR-SYS-DATA-003-API-SYS-IPC-CALL-001                                           | API    | BR-SYS-DATA-003                                                       | src-tauri/src/ipc/mod.rs    | 335  |
| BR-SYS-DATA-003-API-SYS-IPC-CALL-005                                           | API    | BR-SYS-DATA-003                                                       | src-tauri/src/lib.rs        | 3    |
| BR-SYS-DATA-003-API-SYS-IPC-CALL-002                                           | API    | BR-SYS-DATA-003 term_ref: TERM-WS-001                                 | src-tauri/src/lib.rs        | 295  |
| BR-SYS-STATE-003-EFFECT-SYS-IPC-CALL-001                                       | EFFECT | BR-SYS-STATE-003                                                      | src-tauri/src/sm_events.rs  | 48   |

#### KB-RETRIEVE

| codes                                    | type   | rules          | 文件                           | 行号  |
| ---------------------------------------- | ------ | -------------- | ---------------------------- | --- |
| BR-KB-DATA-002-QUERY-KB-KB-RETRIEVE-001  | QUERY  | BR-KB-DATA-002 | src-tauri/src/kb/retrieve.rs | 12  |
| BR-KB-DATA-001-EFFECT-KB-KB-RETRIEVE-001 | EFFECT | BR-KB-DATA-001 | src-tauri/src/kb/store.rs    | 31  |
| BR-KB-DATA-001-QUERY-KB-KB-RETRIEVE-002  | QUERY  | BR-KB-DATA-001 | src-tauri/src/kb/store.rs    | 84  |
| BR-KB-DATA-001-EFFECT-KB-KB-RETRIEVE-002 | EFFECT | BR-KB-DATA-001 | src-tauri/src/kb/store.rs    | 106 |

#### PCG-AUDIT

| codes                                                                          | type | rules                                | 文件                                   | 行号 |
| ------------------------------------------------------------------------------ | ---- | ------------------------------------ | ------------------------------------ | -- |
| BR-PCG-VERIFY-001-RB-PCG-PCG-AUDIT-001, BR-PCG-VERIFY-002-RB-PCG-PCG-AUDIT-001 | RB   | BR-PCG-VERIFY-001, BR-PCG-VERIFY-002 | scripts/audit_product_capability.mjs | 5  |

#### PCG-GENERATE

| codes                                                                             | type | rules                             | 文件                                            | 行号 |
| --------------------------------------------------------------------------------- | ---- | --------------------------------- | --------------------------------------------- | -- |
| BR-PCG-DATA-001-RB-PCG-PCG-GENERATE-001, BR-PCG-STATE-001-RB-PCG-PCG-GENERATE-001 | RB   | BR-PCG-DATA-001, BR-PCG-STATE-001 | scripts/generate_function_ledger.mjs          | 14 |
| BR-PCG-DATA-001-RB-PCG-PCG-GENERATE-004                                           | RB   | BR-PCG-DATA-001, BR-PCG-STATE-001 | scripts/generate_function_tree.mjs            | 22 |
| BR-PCG-DATA-001-RB-PCG-PCG-GENERATE-001, BR-PCG-STATE-001-RB-PCG-PCG-GENERATE-001 | RB   | BR-PCG-DATA-001, BR-PCG-STATE-001 | scripts/generate_product_capability_atlas.mjs | 11 |
| BR-PCG-DATA-001-RB-PCG-PCG-GENERATE-003                                           | RB   | BR-PCG-DATA-001, BR-PCG-STATE-001 | scripts/seed_function_audit.mjs               | 28 |

#### PRV-VIEW

| codes                                                                    | type  | rules                            | 文件                                 | 行号  |
| ------------------------------------------------------------------------ | ----- | -------------------------------- | ---------------------------------- | --- |
| BR-PRV-DATA-001-GUARD-PRV-PRV-VIEW-003                                   | GUARD | BR-PRV-DATA-001                  | src/components/prv/PrvPanel.tsx    | 41  |
| BR-PRV-DATA-001-RB-PRV-PRV-VIEW-004, BR-PRV-DATA-002-RB-PRV-PRV-VIEW-004 | RB    | BR-PRV-DATA-001, BR-PRV-DATA-002 | src/components/prv/PrvToolbar.tsx  | 23  |
| BR-PRV-DATA-002-IO-PRV-PRV-VIEW-004                                      | IO    | BR-PRV-DATA-002                  | src-tauri/src/doc/table_preview.rs | 48  |
| BR-PRV-DATA-002-IO-PRV-PRV-VIEW-001                                      | IO    | BR-PRV-DATA-002                  | src-tauri/src/prv/mod.rs           | 84  |
| BR-PRV-DATA-002-IO-PRV-PRV-VIEW-002                                      | IO    | BR-PRV-DATA-002                  | src-tauri/src/prv/mod.rs           | 124 |
| BR-PRV-DATA-002-IO-PRV-PRV-VIEW-003                                      | IO    | BR-PRV-DATA-002                  | src-tauri/src/prv/mod.rs           | 182 |

#### REF-INJECT

| codes                                                                                                                       | type  | rules                                                                                    | 文件                                 | 行号  |
| --------------------------------------------------------------------------------------------------------------------------- | ----- | ---------------------------------------------------------------------------------------- | ---------------------------------- | --- |
| BR-REF-DATA-004-DATA-AG-REF-INJECT-033                                                                                      | DATA  | BR-REF-DATA-004 term_ref: TERM-REF-004, TERM-REF-003                                     | src/components/ag/Composer.tsx     | 40  |
| BR-REF-DATA-003-DATA-REF-REF-INJECT-005                                                                                     | DATA  | BR-REF-DATA-003 term_ref: TERM-REF-003                                                   | src/services/referenceDrag.ts      | 2   |
| BR-REF-DATA-003-GUARD-REF-REF-INJECT-004                                                                                    | GUARD | BR-REF-DATA-003 term_ref: TERM-REF-003                                                   | src/services/referenceDrag.ts      | 49  |
| BR-REF-DATA-003-GUARD-REF-REF-INJECT-005                                                                                    | GUARD | BR-REF-DATA-003 term_ref: TERM-REF-003                                                   | src/services/referenceDrag.ts      | 71  |
| BR-REF-DATA-003-DATA-REF-REF-INJECT-006, BR-SYS-DATA-002-DATA-REF-REF-INJECT-006                                            | DATA  | BR-REF-DATA-003, BR-SYS-DATA-002 term_ref: TERM-REF-003                                  | src/services/referencePayload.ts   | 2   |
| BR-REF-DATA-003-GUARD-REF-REF-INJECT-006                                                                                    | GUARD | BR-REF-DATA-003 term_ref: TERM-REF-003                                                   | src/services/referencePayload.ts   | 122 |
| BR-REF-DATA-003-GUARD-REF-REF-INJECT-007                                                                                    | GUARD | BR-REF-DATA-003 term_ref: TERM-REF-003                                                   | src/services/referencePayload.ts   | 183 |
| BR-REF-DATA-004-RB-REF-REF-INJECT-001                                                                                       | RB    | BR-REF-DATA-004 term_ref: TERM-REF-004, TERM-REF-003                                     | src/services/referencePayload.ts   | 439 |
| BR-SYS-DATA-007-API-AST-AST-MEMORY-001, BR-AST-DATA-001-API-AST-AST-MEMORY-001                                              | API   | BR-SYS-DATA-007, BR-AST-DATA-001 term_ref: TERM-AST-002                                  | src-tauri/src/ast/routes.rs        | 6   |
| BR-SYS-DATA-007-QUERY-AST-AST-MEMORY-003, BR-AST-DATA-001-QUERY-AST-AST-MEMORY-003                                          | QUERY | BR-SYS-DATA-007, BR-AST-DATA-001 term_ref: TERM-AST-002                                  | src-tauri/src/ast/store.rs         | 72  |
| BR-REF-DATA-003-DATA-REF-REF-INJECT-003                                                                                     | DATA  | BR-REF-DATA-003 term_ref: TERM-REF-003                                                   | src-tauri/src/reference/context.rs | 36  |
| BR-REF-VERIFY-001-GUARD-REF-REF-INJECT-002                                                                                  | GUARD | BR-REF-VERIFY-001 term_ref: TERM-REF-003                                                 | src-tauri/src/reference/context.rs | 54  |
| BR-REF-DATA-003-DATA-REF-REF-INJECT-002, BR-REF-VERIFY-001-DATA-REF-REF-INJECT-002, BR-SYS-DATA-007-DATA-REF-REF-INJECT-002 | DATA  | BR-REF-DATA-003, BR-REF-VERIFY-001, BR-SYS-DATA-007 term_ref: TERM-REF-003, TERM-REF-005 | src-tauri/src/reference/context.rs | 102 |
| BR-REF-DATA-003-DATA-REF-REF-INJECT-004                                                                                     | DATA  | BR-REF-DATA-003 term_ref: TERM-REF-003                                                   | src-tauri/src/reference/context.rs | 242 |
| BR-REF-VERIFY-001-GUARD-REF-REF-INJECT-001                                                                                  | GUARD | BR-REF-VERIFY-001                                                                        | src-tauri/src/reference/inject.rs  | 14  |
| BR-REF-DATA-002-DATA-REF-REF-INJECT-001                                                                                     | DATA  | BR-REF-DATA-002                                                                          | src-tauri/src/reference/inject.rs  | 54  |
| BR-REF-DATA-001-GUARD-REF-REF-INJECT-003                                                                                    | GUARD | BR-REF-DATA-001                                                                          | src-tauri/src/reference/resolve.rs | 34  |
| BR-REF-DATA-001-API-REF-REF-INJECT-001                                                                                      | API   | BR-REF-DATA-001 term_ref: TERM-REF-001                                                   | src-tauri/src/reference/routes.rs  | 75  |
| BR-REF-DATA-001-API-REF-REF-INJECT-002                                                                                      | API   | BR-REF-DATA-001 term_ref: TERM-REF-001                                                   | src-tauri/src/reference/routes.rs  | 112 |
| BR-REF-VERIFY-001-API-REF-REF-INJECT-003                                                                                    | API   | BR-REF-VERIFY-001 term_ref: TERM-REF-001                                                 | src-tauri/src/reference/routes.rs  | 159 |
| BR-REF-DATA-002-API-REF-REF-INJECT-004, BR-REF-VERIFY-001-API-REF-REF-INJECT-004                                            | API   | BR-REF-DATA-002, BR-REF-VERIFY-001 term_ref: TERM-REF-002                                | src-tauri/src/reference/routes.rs  | 187 |

#### TPL-FLOW

| codes                                    | type   | rules            | 文件                            | 行号  |
| ---------------------------------------- | ------ | ---------------- | ----------------------------- | --- |
| BR-TPL-DATA-001-EFFECT-TPL-TPL-FLOW-001  | EFFECT | BR-TPL-DATA-001  | src-tauri/src/tpl/template.rs | 43  |
| BR-TPL-DATA-001-QUERY-TPL-TPL-FLOW-001   | QUERY  | BR-TPL-DATA-001  | src-tauri/src/tpl/template.rs | 71  |
| BR-TPL-DATA-001-EFFECT-TPL-TPL-FLOW-002  | EFFECT | BR-TPL-DATA-001  | src-tauri/src/tpl/template.rs | 91  |
| BR-TPL-STATE-001-RB-TPL-TPL-FLOW-001     | RB     | BR-TPL-STATE-001 | src-tauri/src/tpl/workflow.rs | 49  |
| BR-TPL-STATE-001-EFFECT-TPL-TPL-FLOW-003 | EFFECT | BR-TPL-STATE-001 | src-tauri/src/tpl/workflow.rs | 109 |
| BR-TPL-STATE-001-QUERY-TPL-TPL-FLOW-002  | QUERY  | BR-TPL-STATE-001 | src-tauri/src/tpl/workflow.rs | 138 |

#### WS-DIFF-APPLY

| codes                                                                                                            | type   | rules                                                   | 文件                                    | 行号  |
| ---------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------------------------------- | ------------------------------------- | --- |
| BR-CORE-STATE-001-API-DE-DE-DIFF-001, BR-CORE-STATE-002-API-DE-DE-DIFF-001, BR-CORE-STATE-003-API-DE-DE-DIFF-001 | API    | BR-CORE-STATE-001, BR-CORE-STATE-002, BR-CORE-STATE-003 | src/services/deService.ts             | 354 |
| BR-CORE-STATE-002-EFFECT-DE-DE-DIFF-007                                                                          | EFFECT | BR-CORE-STATE-002                                       | src-tauri/src/de/routes.rs            | 285 |
| BR-WS-STATE-001-RB-WS-WS-DIFF-APPLY-001                                                                          | RB     | BR-WS-STATE-001                                         | src-tauri/src/workspace/diff_apply.rs | 45  |
| BR-WS-STATE-001-EFFECT-WS-WS-DIFF-APPLY-001                                                                      | EFFECT | BR-WS-STATE-001                                         | src-tauri/src/workspace/diff_apply.rs | 76  |
| BR-WS-STATE-001-EFFECT-WS-WS-DIFF-APPLY-002                                                                      | EFFECT | BR-WS-STATE-001                                         | src-tauri/src/workspace/diff_apply.rs | 372 |
| BR-WS-STATE-001-EFFECT-WS-WS-DIFF-APPLY-003                                                                      | EFFECT | BR-WS-STATE-001                                         | src-tauri/src/workspace/diff_apply.rs | 445 |
| BR-WS-STATE-001-QUERY-WS-WS-DIFF-APPLY-001                                                                       | QUERY  | BR-WS-STATE-001                                         | src-tauri/src/workspace/diff_apply.rs | 524 |

#### WS-FS

| codes                               | type   | rules                                              | 文件                                   | 行号   |
| ----------------------------------- | ------ | -------------------------------------------------- | ------------------------------------ | ---- |
| BR-WS-DATA-001-EFFECT-WS-WS-FS-001  | EFFECT | BR-WS-DATA-001 term_ref: TERM-WS-001               | src-tauri/src/workspace/fileops.rs   | 16   |
| BR-WS-DATA-001-EFFECT-WS-WS-FS-004  | EFFECT | BR-WS-DATA-001 term_ref: TERM-WS-001               | src-tauri/src/workspace/fileops.rs   | 43   |
| BR-WS-DATA-001-EFFECT-WS-WS-FS-002  | EFFECT | BR-WS-DATA-001 term_ref: TERM-WS-001               | src-tauri/src/workspace/fileops.rs   | 70   |
| BR-WS-DATA-001-EFFECT-WS-WS-FS-007  | EFFECT | BR-WS-DATA-001                                     | src-tauri/src/workspace/fileops.rs   | 100  |
| BR-WS-DATA-001-EFFECT-WS-WS-FS-003  | EFFECT | BR-WS-DATA-001 term_ref: TERM-WS-001               | src-tauri/src/workspace/fileops.rs   | 134  |
| BR-WS-DATA-001-EFFECT-WS-WS-FS-008  | EFFECT | BR-WS-DATA-001                                     | src-tauri/src/workspace/listing.rs   | 148  |
| BR-WS-DATA-001-QUERY-WS-WS-FS-002   | QUERY  | BR-WS-DATA-001 term_ref: TERM-WS-001               | src-tauri/src/workspace/listing.rs   | 174  |
| BR-WS-DATA-001-QUERY-WS-WS-FS-006   | QUERY  | BR-WS-DATA-001 term_ref: TERM-WS-001               | src-tauri/src/workspace/listing.rs   | 582  |
| BR-WS-DATA-001-QUERY-WS-WS-FS-005   | QUERY  | BR-WS-DATA-001 term_ref: TERM-WS-001               | src-tauri/src/workspace/listing.rs   | 703  |
| BR-WS-DATA-001-GUARD-WS-WS-FS-001   | GUARD  | BR-WS-DATA-001                                     | src-tauri/src/workspace/mod.rs       | 46   |
| BR-WS-STATE-009-EFFECT-WS-WS-FS-009 | EFFECT | BR-WS-STATE-009 term_ref: TERM-WS-003, TERM-WS-006 | src-tauri/src/workspace/operation.rs | 410  |
| BR-WS-STATE-009-EFFECT-WS-WS-FS-010 | EFFECT | BR-WS-STATE-009 term_ref: TERM-WS-003, TERM-WS-006 | src-tauri/src/workspace/operation.rs | 470  |
| BR-WS-OBS-001-RB-WS-WS-FS-001       | RB     | BR-WS-OBS-001                                      | src-tauri/src/workspace/routes.rs    | 1363 |

#### WS-OPERATION

| codes                                                                                                                                                                                                                                                | type   | rules                                                                                                                                                             | 文件                                   | 行号   |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------ | ---- |
| BR-AG-DATA-004-QUERY-AG-AG-TURN-052, BR-SYS-STATE-008-QUERY-AG-AG-TURN-052                                                                                                                                                                           | QUERY  | BR-AG-DATA-004, BR-SYS-STATE-008 term_ref: TERM-AG-017, TERM-AG-015                                                                                               | src-tauri/src/ag/task.rs             | 750  |
| BR-AG-DATA-003-EFFECT-AG-AG-TURN-037                                                                                                                                                                                                                 | EFFECT | BR-AG-DATA-003 term_ref: TERM-AG-015, TERM-AG-019, TERM-WS-003                                                                                                    | src-tauri/src/ag/task_runtime.rs     | 16   |
| BR-AG-STATE-002-API-AG-WS-OPERATION-005, BR-AG-DATA-003-API-AG-WS-OPERATION-005, BR-SYS-STATE-008-API-AG-WS-OPERATION-005, BR-WS-STATE-003-API-AG-WS-OPERATION-005, BR-WS-PERSIST-003-API-AG-WS-OPERATION-005, BR-WS-OBS-002-API-AG-WS-OPERATION-005 | API    | BR-AG-STATE-002, BR-AG-DATA-003, BR-SYS-STATE-008, BR-WS-STATE-003, BR-WS-PERSIST-003, BR-WS-OBS-002 term_ref: TERM-AG-015, TERM-AG-017, TERM-AG-019, TERM-WS-003 | src-tauri/src/ag/task_runtime.rs     | 197  |
| BR-AG-STATE-001-API-AG-AG-TURN-070, BR-AG-PERSIST-001-API-AG-AG-TURN-070                                                                                                                                                                             | API    | BR-AG-STATE-001, BR-AG-PERSIST-001 term_ref: TERM-AG-001, TERM-WS-004                                                                                             | src-tauri/src/ag/task_runtime.rs     | 725  |
| BR-SYS-STATE-008-GUARD-AG-AG-TOOL-040, BR-WS-STATE-003-GUARD-AG-AG-TOOL-040                                                                                                                                                                          | GUARD  | BR-SYS-STATE-008, BR-WS-STATE-003 term_ref: TERM-AG-014, TERM-AG-015, TERM-WS-003                                                                                 | src-tauri/src/ag/tool_registry.rs    | 415  |
| BR-AG-STATE-001-EFFECT-AG-AG-TURN-069, BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-069                                                                                                                                                                       | EFFECT | BR-AG-STATE-001, BR-AG-PERSIST-001 term_ref: TERM-AG-001, TERM-WS-004                                                                                             | src-tauri/src/ag/turn_events.rs      | 1245 |
| BR-WS-STATE-003-RB-WS-WS-OPERATION-001, BR-WS-PERSIST-003-RB-WS-WS-OPERATION-001                                                                                                                                                                     | RB     | BR-WS-STATE-003, BR-WS-PERSIST-003 term_ref: TERM-WS-003, TERM-WS-004                                                                                             | src-tauri/src/workspace/operation.rs | 143  |
| BR-WS-DATA-003-EFFECT-WS-WS-OPERATION-001                                                                                                                                                                                                            | EFFECT | BR-WS-DATA-003 term_ref: TERM-WS-004                                                                                                                              | src-tauri/src/workspace/operation.rs | 176  |
| BR-WS-PERSIST-003-EFFECT-WS-WS-OPERATION-002                                                                                                                                                                                                         | EFFECT | BR-WS-PERSIST-003 term_ref: TERM-WS-003                                                                                                                           | src-tauri/src/workspace/operation.rs | 1818 |
| BR-WS-STATE-003-API-WS-WS-OPERATION-001, BR-WS-DATA-003-API-WS-WS-OPERATION-001, BR-WS-OBS-002-API-WS-WS-OPERATION-001                                                                                                                               | API    | BR-WS-STATE-003, BR-WS-DATA-003, BR-WS-OBS-002 term_ref: TERM-WS-003, TERM-WS-004                                                                                 | src-tauri/src/workspace/operation.rs | 2261 |
| BR-WS-STATE-003-QUERY-WS-WS-OPERATION-006, BR-WS-DATA-003-QUERY-WS-WS-OPERATION-006                                                                                                                                                                  | QUERY  | BR-WS-STATE-003, BR-WS-DATA-003 term_ref: TERM-WS-003, TERM-WS-004                                                                                                | src-tauri/src/workspace/operation.rs | 2291 |
| BR-WS-STATE-003-EFFECT-WS-WS-OPERATION-007, BR-WS-DATA-003-EFFECT-WS-WS-OPERATION-007, BR-WS-OBS-002-EFFECT-WS-WS-OPERATION-007                                                                                                                      | EFFECT | BR-WS-STATE-003, BR-WS-DATA-003, BR-WS-OBS-002 term_ref: TERM-WS-003, TERM-WS-004                                                                                 | src-tauri/src/workspace/operation.rs | 2417 |
| BR-WS-STATE-003-EFFECT-WS-WS-OPERATION-003, BR-WS-DATA-003-EFFECT-WS-WS-OPERATION-003, BR-WS-OBS-002-EFFECT-WS-WS-OPERATION-003, BR-WS-PERSIST-003-EFFECT-WS-WS-OPERATION-003                                                                        | EFFECT | BR-WS-STATE-003, BR-WS-DATA-003, BR-WS-OBS-002, BR-WS-PERSIST-003 term_ref: TERM-WS-003, TERM-WS-004, TERM-WS-006                                                 | src-tauri/src/workspace/operation.rs | 2943 |
| BR-WS-STATE-003-EFFECT-WS-WS-OPERATION-008, BR-WS-PERSIST-003-EFFECT-WS-WS-OPERATION-008                                                                                                                                                             | EFFECT | BR-WS-STATE-003, BR-WS-PERSIST-003 term_ref: TERM-WS-003, TERM-WS-004                                                                                             | src-tauri/src/workspace/operation.rs | 2966 |
| BR-WS-STATE-003-API-WS-WS-OPERATION-002, BR-WS-DATA-003-API-WS-WS-OPERATION-002, BR-WS-OBS-002-API-WS-WS-OPERATION-002                                                                                                                               | API    | BR-WS-STATE-003, BR-WS-DATA-003, BR-WS-OBS-002 term_ref: TERM-WS-003, TERM-WS-004                                                                                 | src-tauri/src/workspace/routes.rs    | 1467 |
| BR-WS-STATE-003-API-WS-WS-OPERATION-004, BR-WS-DATA-003-API-WS-WS-OPERATION-004, BR-WS-OBS-002-API-WS-WS-OPERATION-004                                                                                                                               | API    | BR-WS-STATE-003, BR-WS-DATA-003, BR-WS-OBS-002 term_ref: TERM-WS-003, TERM-WS-007, TERM-ED-008, TERM-DE-003                                                       | src-tauri/src/workspace/routes.rs    | 1861 |

#### WS-SAVE

| codes                                                                                                              | type   | rules                                                   | 文件                                | 行号  |
| ------------------------------------------------------------------------------------------------------------------ | ------ | ------------------------------------------------------- | --------------------------------- | --- |
| BR-CORE-STATE-003-EFFECT-DE-ED-EDIT-011                                                                            | EFFECT | BR-CORE-STATE-003                                       | src/services/deService.ts         | 297 |
| BR-CORE-STATE-001-API-DE-DE-DIFF-001, BR-CORE-STATE-002-API-DE-DE-DIFF-001, BR-CORE-STATE-003-API-DE-DE-DIFF-001   | API    | BR-CORE-STATE-001, BR-CORE-STATE-002, BR-CORE-STATE-003 | src/services/deService.ts         | 354 |
| BR-CORE-STATE-001-EFFECT-ED-WS-SAVE-001, BR-CORE-STATE-003-EFFECT-ED-WS-SAVE-001                                   | EFFECT | BR-CORE-STATE-001, BR-CORE-STATE-003                    | src/services/edService.ts         | 91  |
| BR-CORE-STATE-001-EFFECT-DE-DE-DIFF-005                                                                            | EFFECT | BR-CORE-STATE-001                                       | src-tauri/src/de/routes.rs        | 123 |
| BR-CORE-STATE-003-EFFECT-DE-DE-DIFF-006                                                                            | EFFECT | BR-CORE-STATE-003                                       | src-tauri/src/de/routes.rs        | 252 |
| BR-WS-PERSIST-001-EFFECT-WS-WS-SAVE-003, BR-WS-OBS-001-EFFECT-WS-WS-SAVE-003, BR-WS-DATA-001-EFFECT-WS-WS-SAVE-003 | EFFECT | BR-WS-PERSIST-001, BR-WS-OBS-001, BR-WS-DATA-001        | src-tauri/src/workspace/routes.rs | 647 |
| BR-WS-PERSIST-001-EFFECT-WS-WS-SAVE-001                                                                            | EFFECT | BR-WS-PERSIST-001                                       | src-tauri/src/workspace/save.rs   | 51  |
| BR-WS-PERSIST-001-RB-WS-WS-SAVE-002                                                                                | RB     | BR-WS-PERSIST-001                                       | src-tauri/src/workspace/save.rs   | 103 |

#### WS-TIMELINE

| codes                                                                                 | type   | rules                                                                | 文件                                   | 行号  |
| ------------------------------------------------------------------------------------- | ------ | -------------------------------------------------------------------- | ------------------------------------ | --- |
| BR-WS-DATA-003-EFFECT-WS-WS-TIMELINE-003, BR-WS-PERSIST-003-EFFECT-WS-WS-TIMELINE-003 | EFFECT | BR-WS-DATA-003, BR-WS-PERSIST-003 term_ref: TERM-WS-005, TERM-WS-006 | src-tauri/src/workspace/operation.rs | 349 |
| BR-WS-STATE-009-EFFECT-WS-WS-FS-009                                                   | EFFECT | BR-WS-STATE-009 term_ref: TERM-WS-003, TERM-WS-006                   | src-tauri/src/workspace/operation.rs | 410 |
| BR-WS-STATE-009-EFFECT-WS-WS-FS-010                                                   | EFFECT | BR-WS-STATE-009 term_ref: TERM-WS-003, TERM-WS-006                   | src-tauri/src/workspace/operation.rs | 470 |
| BR-WS-DATA-002-EFFECT-WS-WS-TIMELINE-001                                              | EFFECT | BR-WS-DATA-002                                                       | src-tauri/src/workspace/timeline.rs  | 47  |
| BR-WS-DATA-002-QUERY-WS-WS-TIMELINE-001                                               | QUERY  | BR-WS-DATA-002                                                       | src-tauri/src/workspace/timeline.rs  | 91  |
| BR-WS-DATA-002-EFFECT-WS-WS-TIMELINE-002                                              | EFFECT | BR-WS-DATA-002                                                       | src-tauri/src/workspace/timeline.rs  | 138 |
| BR-WS-STATE-002-QUERY-WS-WS-TIMELINE-002                                              | QUERY  | BR-WS-STATE-002                                                      | src-tauri/src/workspace/timeline.rs  | 202 |
| BR-WS-STATE-002-RB-WS-WS-TIMELINE-001                                                 | RB     | BR-WS-STATE-002                                                      | src-tauri/src/workspace/timeline.rs  | 242 |

### 规则视图

#### BR-ACCT-DATA-001

| codes                                       | type   | chain      | 文件                          | 行号 |
| ------------------------------------------- | ------ | ---------- | --------------------------- | -- |
| BR-ACCT-DATA-001-EFFECT-ACCT-CLOUD-AUTH-001 | EFFECT | CLOUD-AUTH | src-tauri/src/acct/token.rs | 46 |
| BR-ACCT-DATA-001-EFFECT-ACCT-CLOUD-AUTH-002 | EFFECT | CLOUD-AUTH | src-tauri/src/acct/token.rs | 60 |
| BR-ACCT-DATA-001-EFFECT-ACCT-CLOUD-AUTH-003 | EFFECT | CLOUD-AUTH | src-tauri/src/acct/token.rs | 73 |

#### BR-ACCT-STATE-001

| codes                                       | type  | chain      | 文件                            | 行号 |
| ------------------------------------------- | ----- | ---------- | ----------------------------- | -- |
| BR-ACCT-STATE-001-RB-ACCT-CLOUD-AUTH-001    | RB    | CLOUD-AUTH | src-tauri/src/acct/session.rs | 24 |
| BR-ACCT-STATE-001-GUARD-ACCT-CLOUD-AUTH-001 | GUARD | CLOUD-AUTH | src-tauri/src/acct/session.rs | 38 |
| BR-ACCT-STATE-001-GUARD-ACCT-CLOUD-AUTH-002 | GUARD | CLOUD-AUTH | src-tauri/src/acct/session.rs | 52 |
| BR-ACCT-STATE-001-GUARD-ACCT-CLOUD-AUTH-003 | GUARD | CLOUD-AUTH | src-tauri/src/acct/session.rs | 70 |

#### BR-AG-DATA-001

| codes                                                                                                                                                                              | type   | chain            | 文件                                    | 行号   |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ---------------- | ------------------------------------- | ---- |
| BR-AG-DATA-001-GUARD-AG-AG-TOOL-044, BR-AG-DATA-008-GUARD-AG-AG-TOOL-044                                                                                                           | GUARD  | AG-TURN, AG-TOOL | src-tauri/src/ag/blocks.rs            | 175  |
| BR-AG-DATA-001-QUERY-AG-AG-TOOL-011, BR-ED-DATA-003-QUERY-AG-AG-TOOL-011                                                                                                           | QUERY  | AG-TOOL          | src-tauri/src/ag/blocks.rs            | 307  |
| BR-AG-DATA-001-QUERY-AG-AG-TURN-010, BR-DOC-DATA-003-QUERY-AG-AG-TURN-010                                                                                                          | QUERY  | AG-TURN          | src-tauri/src/ag/blocks.rs            | 533  |
| BR-AG-DATA-001-DATA-AG-AG-TURN-011, BR-AG-DATA-002-DATA-AG-AG-TURN-011, BR-AG-DATA-004-DATA-AG-AG-TURN-011, BR-AG-DATA-005-DATA-AG-AG-TURN-011, BR-AG-DATA-008-DATA-AG-AG-TURN-011 | DATA   | AG-TURN          | src-tauri/src/ag/prompt_runtime.rs    | 283  |
| BR-AG-DATA-001-QUERY-AG-AG-TURN-029                                                                                                                                                | QUERY  | AG-TURN          | src-tauri/src/ag/reference_runtime.rs | 282  |
| BR-AG-STATE-002-QUERY-AG-AG-TOOL-012, BR-AG-DATA-001-QUERY-AG-AG-TOOL-012, BR-WS-DATA-001-QUERY-AG-AG-TOOL-012                                                                     | QUERY  | AG-TOOL          | src-tauri/src/ag/tool_exec.rs         | 483  |
| BR-AG-DATA-001-EFFECT-AG-AG-TURN-078                                                                                                                                               | EFFECT | AG-TURN          | src-tauri/src/ag/turn_tool_loop.rs    | 1177 |

#### BR-AG-DATA-001 term_ref: TERM-AG-015

| codes                                                                        | type   | chain                     | 文件                               | 行号   |
| ---------------------------------------------------------------------------- | ------ | ------------------------- | -------------------------------- | ---- |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-006, BR-AG-DATA-001-EFFECT-AG-AG-TURN-006 | EFFECT | AG-TURN, DE-DIFF, ED-EDIT | src/services/agService.ts        | 1535 |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-032, BR-AG-DATA-001-EFFECT-AG-AG-TURN-032 | EFFECT | AG-TURN, DE-DIFF, ED-EDIT | src-tauri/src/ag/task.rs         | 110  |
| BR-SYS-STATE-008-GUARD-AG-AG-TURN-005, BR-AG-DATA-001-GUARD-AG-AG-TURN-005   | GUARD  | AG-TURN, DE-DIFF, ED-EDIT | src-tauri/src/ag/task.rs         | 169  |
| BR-SYS-STATE-008-GUARD-AG-AG-TURN-006, BR-AG-DATA-001-GUARD-AG-AG-TURN-006   | GUARD  | AG-TURN, ED-EDIT          | src-tauri/src/ag/task.rs         | 287  |
| BR-SYS-STATE-008-API-AG-AG-TURN-003, BR-AG-DATA-001-API-AG-AG-TURN-003       | API    | AG-TURN, DE-DIFF, ED-EDIT | src-tauri/src/ag/turn_control.rs | 654  |

#### BR-AG-DATA-001 term_ref: TERM-DOC-003

| codes                                | type   | chain   | 文件                        | 行号  |
| ------------------------------------ | ------ | ------- | ------------------------- | --- |
| BR-AG-DATA-001-EFFECT-AG-AG-TURN-005 | EFFECT | AG-TURN | src/services/agService.ts | 648 |

#### BR-AG-DATA-001 term_ref: TERM-REF-003

| codes                              | type | chain   | 文件                                    | 行号  |
| ---------------------------------- | ---- | ------- | ------------------------------------- | --- |
| BR-AG-DATA-001-RB-AG-AG-TURN-017   | RB   | AG-TURN | src-tauri/src/ag/reference_runtime.rs | 167 |
| BR-AG-DATA-001-DATA-AG-AG-TURN-030 | DATA | AG-TURN | src-tauri/src/ag/turn_reference.rs    | 15  |

#### BR-AG-DATA-002

| codes                                                                                                                                                                              | type | chain     | 文件                                 | 行号  |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---- | --------- | ---------------------------------- | --- |
| BR-AG-DATA-002-DATA-AG-AG-ASSIST-011                                                                                                                                               | DATA | AG-ASSIST | src-tauri/src/ag/assist_prompt.rs  | 19  |
| BR-AG-DATA-002-DATA-AG-AG-TURN-055, BR-AG-DATA-005-DATA-AG-AG-TURN-055                                                                                                             | DATA | AG-TURN   | src-tauri/src/ag/context.rs        | 164 |
| BR-AG-DATA-002-DATA-AG-AG-TURN-016                                                                                                                                                 | DATA | AG-TURN   | src-tauri/src/ag/context.rs        | 344 |
| BR-AG-DATA-001-DATA-AG-AG-TURN-011, BR-AG-DATA-002-DATA-AG-AG-TURN-011, BR-AG-DATA-004-DATA-AG-AG-TURN-011, BR-AG-DATA-005-DATA-AG-AG-TURN-011, BR-AG-DATA-008-DATA-AG-AG-TURN-011 | DATA | AG-TURN   | src-tauri/src/ag/prompt_runtime.rs | 283 |
| BR-AG-DATA-002-DATA-AG-AG-TURN-031, BR-AG-DATA-008-DATA-AG-AG-TURN-031, BR-SYS-DATA-008-DATA-AG-AG-TURN-031                                                                        | DATA | AG-TURN   | src-tauri/src/ag/turn_prompt.rs    | 95  |

#### BR-AG-DATA-003

| codes                                                                                                                                                                                                                                                | type   | chain                          | 文件                                | 行号  |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------ | --------------------------------- | --- |
| BR-AG-DATA-003-RB-AG-AG-TOOL-016, BR-ED-DATA-001-RB-AG-AG-TOOL-016, BR-DE-DATA-001-RB-AG-AG-TOOL-016                                                                                                                                                 | RB     | AG-TOOL                        | src-tauri/src/ag/block_ops.rs     | 305 |
| BR-AG-STATE-002-EFFECT-AG-AG-TOOL-047, BR-AG-DATA-003-EFFECT-AG-AG-TOOL-047                                                                                                                                                                          | EFFECT | AG-TOOL, DE-DIFF               | src-tauri/src/ag/edit_runtime.rs  | 18  |
| BR-AG-STATE-002-EFFECT-AG-AG-TOOL-017, BR-AG-DATA-003-EFFECT-AG-AG-TOOL-017, BR-DE-DATA-001-EFFECT-AG-AG-TOOL-017, BR-DE-DATA-002-EFFECT-AG-AG-TOOL-017                                                                                              | EFFECT | AG-TOOL, DE-DIFF, ED-EDIT      | src-tauri/src/ag/edit_runtime.rs  | 73  |
| BR-AG-DATA-003-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-007-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-008-DATA-AG-AG-TARGETSET-001                                                                                                                        | DATA   | AG-TURN, AG-TOOL               | src-tauri/src/ag/target_set.rs    | 32  |
| BR-AG-DATA-003-EFFECT-AG-AG-TARGETSET-002, BR-SYS-PERSIST-001-EFFECT-AG-AG-TARGETSET-002                                                                                                                                                             | EFFECT | AG-TURN, AG-TOOL, DATA-MIGRATE | src-tauri/src/ag/target_set.rs    | 202 |
| BR-AG-STATE-002-API-AG-WS-OPERATION-005, BR-AG-DATA-003-API-AG-WS-OPERATION-005, BR-SYS-STATE-008-API-AG-WS-OPERATION-005, BR-WS-STATE-003-API-AG-WS-OPERATION-005, BR-WS-PERSIST-003-API-AG-WS-OPERATION-005, BR-WS-OBS-002-API-AG-WS-OPERATION-005 | API    | AG-TURN, AG-TOOL, WS-OPERATION | src-tauri/src/ag/task_runtime.rs  | 197 |
| BR-AG-STATE-002-GUARD-AG-AG-TOOL-035, BR-SYS-STATE-007-GUARD-AG-AG-TOOL-035, BR-AG-DATA-003-GUARD-AG-AG-TOOL-035, BR-WS-DATA-001-GUARD-AG-AG-TOOL-035                                                                                                | GUARD  | AG-TOOL                        | src-tauri/src/ag/tool_registry.rs | 456 |
| BR-AG-DATA-003-GUARD-AG-AG-TOOL-033, BR-AG-STATE-002-GUARD-AG-AG-TOOL-033                                                                                                                                                                            | GUARD  | AG-TOOL                        | src-tauri/src/ag/tool_registry.rs | 850 |
| BR-AG-DATA-003-EFFECT-AG-AG-TURN-036, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-036                                                                                                                                                                         | EFFECT | AG-TURN                        | src-tauri/src/ag/turn_control.rs  | 475 |

#### BR-AG-DATA-003 term_ref: TERM-AG-015

| codes                                                                                | type   | chain                 | 文件                               | 行号  |
| ------------------------------------------------------------------------------------ | ------ | --------------------- | -------------------------------- | --- |
| BR-SYS-STATE-008-EFFECT-AG-AG-DECISION-001, BR-AG-DATA-003-EFFECT-AG-AG-DECISION-001 | EFFECT | AG-TURN, AG-TOOL      | src-tauri/src/ag/task.rs         | 505 |
| BR-SYS-STATE-008-EFFECT-AG-AG-DECISION-002, BR-AG-DATA-003-EFFECT-AG-AG-DECISION-002 | EFFECT | AG-TURN, AG-TOOL      | src-tauri/src/ag/task.rs         | 580 |
| BR-AG-DATA-003-EFFECT-AG-AG-TURN-037                                                 | EFFECT | AG-TURN, WS-OPERATION | src-tauri/src/ag/task_runtime.rs | 16  |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-026, BR-AG-DATA-003-EFFECT-AG-AG-TURN-026         | EFFECT | AG-TURN               | src-tauri/src/ag/task_runtime.rs | 79  |

#### BR-AG-DATA-003 term_ref: TERM-AG-019

| codes                                    | type  | chain            | 文件                             | 行号  |
| ---------------------------------------- | ----- | ---------------- | ------------------------------ | --- |
| BR-AG-DATA-003-GUARD-AG-AG-TARGETSET-003 | GUARD | AG-TOOL, DE-DIFF | src-tauri/src/ag/target_set.rs | 272 |
| BR-AG-DATA-003-GUARD-DE-DE-DIFF-004      | GUARD | DE-DIFF          | src-tauri/src/de/generate.rs   | 61  |

#### BR-AG-DATA-004

| codes                                                                                                                                                                              | type  | chain                 | 文件                                 | 行号  |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----- | --------------------- | ---------------------------------- | --- |
| BR-AG-DATA-004-DATA-AG-AG-TURN-046, BR-AG-STATE-001-DATA-AG-AG-TURN-046                                                                                                            | DATA  | AG-TURN               | src-tauri/src/ag/context.rs        | 53  |
| BR-AG-DATA-004-GUARD-AG-AG-TURN-047, BR-AG-STATE-001-GUARD-AG-AG-TURN-047                                                                                                          | GUARD | AG-TURN               | src-tauri/src/ag/model_output.rs   | 54  |
| BR-AG-DATA-004-DATA-AG-AG-TOOL-061                                                                                                                                                 | DATA  | AG-TOOL, AG-TURN      | src-tauri/src/ag/model_output.rs   | 342 |
| BR-AG-DATA-001-DATA-AG-AG-TURN-011, BR-AG-DATA-002-DATA-AG-AG-TURN-011, BR-AG-DATA-004-DATA-AG-AG-TURN-011, BR-AG-DATA-005-DATA-AG-AG-TURN-011, BR-AG-DATA-008-DATA-AG-AG-TURN-011 | DATA  | AG-TURN               | src-tauri/src/ag/prompt_runtime.rs | 283 |
| BR-AG-DATA-004-QUERY-AG-AG-TURN-052, BR-SYS-STATE-008-QUERY-AG-AG-TURN-052                                                                                                         | QUERY | AG-TURN, WS-OPERATION | src-tauri/src/ag/task.rs           | 750 |
| BR-AG-DATA-004-DATA-AG-AG-TURN-062                                                                                                                                                 | DATA  | AG-TURN               | src-tauri/src/ag/turn_tool_loop.rs | 121 |

#### BR-AG-DATA-004 term_ref: TERM-AG-001

| codes                               | type  | chain   | 文件                               | 行号  |
| ----------------------------------- | ----- | ------- | -------------------------------- | --- |
| BR-AG-DATA-004-GUARD-AG-AG-TURN-049 | GUARD | AG-TURN | src-tauri/src/ag/turn_runtime.rs | 362 |

#### BR-AG-DATA-004 term_ref: TERM-AG-005

| codes                              | type | chain   | 文件                          | 行号  |
| ---------------------------------- | ---- | ------- | --------------------------- | --- |
| BR-AG-DATA-004-DATA-AG-AG-TURN-005 | DATA | AG-TURN | src-tauri/src/ag/context.rs | 567 |

#### BR-AG-DATA-004 term_ref: TERM-AG-013

| codes                                                                   | type | chain            | 文件                                | 行号  |
| ----------------------------------------------------------------------- | ---- | ---------------- | --------------------------------- | --- |
| BR-AG-STATE-002-DATA-AG-AG-TOOL-039, BR-AG-DATA-004-DATA-AG-AG-TOOL-039 | DATA | AG-TURN, AG-TOOL | src-tauri/src/ag/tool_registry.rs | 169 |

#### BR-AG-DATA-005

| codes                                                                                                                                                                              | type  | chain   | 文件                                 | 行号  |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----- | ------- | ---------------------------------- | --- |
| BR-AG-DATA-005-DATA-AG-AG-TOOL-058                                                                                                                                                 | DATA  | AG-TOOL | src-tauri/src/ag/blocks.rs         | 620 |
| BR-AG-DATA-002-DATA-AG-AG-TURN-055, BR-AG-DATA-005-DATA-AG-AG-TURN-055                                                                                                             | DATA  | AG-TURN | src-tauri/src/ag/context.rs        | 164 |
| BR-AG-DATA-005-DATA-AG-AG-TURN-004                                                                                                                                                 | DATA  | AG-TURN | src-tauri/src/ag/context.rs        | 436 |
| BR-AG-DATA-001-DATA-AG-AG-TURN-011, BR-AG-DATA-002-DATA-AG-AG-TURN-011, BR-AG-DATA-004-DATA-AG-AG-TURN-011, BR-AG-DATA-005-DATA-AG-AG-TURN-011, BR-AG-DATA-008-DATA-AG-AG-TURN-011 | DATA  | AG-TURN | src-tauri/src/ag/prompt_runtime.rs | 283 |
| BR-AG-DATA-005-DATA-AG-AG-TOOL-059, BR-AG-OBS-001-DATA-AG-AG-TOOL-059                                                                                                              | DATA  | AG-TOOL | src-tauri/src/ag/tool_exec.rs      | 844 |
| BR-AG-DATA-005-GUARD-AG-AG-TOOL-057, BR-AG-OBS-001-GUARD-AG-AG-TOOL-057                                                                                                            | GUARD | AG-TOOL | src-tauri/src/ag/tool_exec.rs      | 922 |
| BR-AG-DATA-005-IO-AG-AG-TURN-056, BR-AG-OBS-001-IO-AG-AG-TURN-056                                                                                                                  | IO    | AG-TURN | src-tauri/src/ag/turn_events.rs    | 730 |

#### BR-AG-DATA-006 term_ref: TERM-AG-010

| codes                                         | type   | chain            | 文件                      | 行号  |
| --------------------------------------------- | ------ | ---------------- | ----------------------- | --- |
| BR-AG-DATA-006-EFFECT-AG-AG-MODEL-CATALOG-006 | EFFECT | AG-MODEL-CATALOG | src/store/modelStore.ts | 187 |

#### BR-AG-DATA-007 term_ref: TERM-AG-010

| codes                                        | type  | chain                                | 文件                      | 行号  |
| -------------------------------------------- | ----- | ------------------------------------ | ----------------------- | --- |
| BR-AG-DATA-007-QUERY-AG-AG-MODEL-CATALOG-013 | QUERY | AG-MODEL-CATALOG, AG-ASSIST, AG-TURN | src/store/modelStore.ts | 161 |

#### BR-AG-DATA-008

| codes                                                                                                                                                                                                                                                   | type   | chain            | 文件                                 | 行号   |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ---------------- | ---------------------------------- | ---- |
| BR-AG-DATA-008-DATA-AG-AG-TURN-041, BR-ED-DATA-008-DATA-AG-AG-TURN-041, BR-SYS-DATA-008-DATA-AG-AG-TURN-041                                                                                                                                             | DATA   | AG-TURN          | src/services/agService.ts          | 182  |
| BR-AG-DATA-008-DATA-AG-AG-TURN-054, BR-SYS-DATA-008-DATA-AG-AG-TURN-054                                                                                                                                                                                 | DATA   | AG-TURN, AG-TOOL | src/services/agService.ts          | 545  |
| BR-AG-OBS-001-DATA-AG-AG-TOOL-043, BR-AG-DATA-008-DATA-AG-AG-TOOL-043, BR-SYS-DATA-008-DATA-AG-AG-TOOL-043                                                                                                                                              | DATA   | AG-TURN, AG-TOOL | src-tauri/src/ag/tool_exec.rs      | 173  |
| BR-AG-DATA-002-DATA-AG-AG-TURN-031, BR-AG-DATA-008-DATA-AG-AG-TURN-031, BR-SYS-DATA-008-DATA-AG-AG-TURN-031                                                                                                                                             | DATA   | AG-TURN          | src-tauri/src/ag/turn_prompt.rs    | 95   |
| BR-AG-DATA-008-GUARD-AG-AG-TURN-053, BR-SYS-STATE-007-GUARD-AG-AG-TURN-053                                                                                                                                                                              | GUARD  | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_runtime.rs   | 88   |
| BR-AG-DATA-008-GUARD-AG-AG-TURN-042, BR-ED-DATA-008-GUARD-AG-AG-TURN-042, BR-SYS-DATA-008-GUARD-AG-AG-TURN-042                                                                                                                                          | GUARD  | AG-TURN          | src-tauri/src/ag/turn_runtime.rs   | 391  |
| BR-AG-DATA-008-GUARD-AG-AG-TURN-052, BR-SYS-DATA-008-GUARD-AG-AG-TURN-052                                                                                                                                                                               | GUARD  | AG-TURN          | src-tauri/src/ag/turn_runtime.rs   | 574  |
| BR-AG-OBS-001-API-AG-AG-TURN-018, BR-AG-STATE-001-API-AG-AG-TURN-018, BR-AG-STATE-004-API-AG-AG-TURN-018, BR-AG-DATA-008-API-AG-AG-TURN-018, BR-ED-DATA-008-API-AG-AG-TURN-018, BR-SYS-STATE-007-API-AG-AG-TURN-018, BR-SYS-DATA-008-API-AG-AG-TURN-018 | API    | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_runtime.rs   | 753  |
| BR-AG-DATA-008-EFFECT-AG-AG-TURN-073                                                                                                                                                                                                                    | EFFECT | AG-TURN          | src-tauri/src/ag/turn_tool_loop.rs | 1609 |

#### BR-AG-DATA-008 term_ref: TERM-AG-005

| codes                                                                                                                                                                              | type | chain   | 文件                                 | 行号  |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---- | ------- | ---------------------------------- | --- |
| BR-AG-DATA-001-DATA-AG-AG-TURN-011, BR-AG-DATA-002-DATA-AG-AG-TURN-011, BR-AG-DATA-004-DATA-AG-AG-TURN-011, BR-AG-DATA-005-DATA-AG-AG-TURN-011, BR-AG-DATA-008-DATA-AG-AG-TURN-011 | DATA | AG-TURN | src-tauri/src/ag/prompt_runtime.rs | 283 |

#### BR-AG-DATA-008 term_ref: TERM-AG-020

| codes                                                                    | type  | chain            | 文件                         | 行号  |
| ------------------------------------------------------------------------ | ----- | ---------------- | -------------------------- | --- |
| BR-AG-DATA-001-GUARD-AG-AG-TOOL-044, BR-AG-DATA-008-GUARD-AG-AG-TOOL-044 | GUARD | AG-TURN, AG-TOOL | src-tauri/src/ag/blocks.rs | 175 |

#### BR-AG-DATA-009

| codes                                                                     | type  | chain   | 文件                               | 行号  |
| ------------------------------------------------------------------------- | ----- | ------- | -------------------------------- | --- |
| BR-AG-STATE-002-GUARD-AG-AG-TOOL-058, BR-AG-DATA-009-GUARD-AG-AG-TOOL-058 | GUARD | AG-TOOL | src-tauri/src/ag/edit_support.rs | 102 |

#### BR-AG-DATA-010

| codes                                                                     | type  | chain   | 文件                               | 行号  |
| ------------------------------------------------------------------------- | ----- | ------- | -------------------------------- | --- |
| BR-AG-STATE-002-GUARD-AG-AG-TOOL-059, BR-AG-DATA-010-GUARD-AG-AG-TOOL-059 | GUARD | AG-TOOL | src-tauri/src/ag/edit_support.rs | 293 |
| BR-AG-DATA-010-GUARD-AG-AG-TOOL-061                                       | GUARD | AG-TOOL | src-tauri/src/ag/edit_support.rs | 302 |

#### BR-AG-DATA-011

| codes                                | type   | chain   | 文件                                 | 行号  |
| ------------------------------------ | ------ | ------- | ---------------------------------- | --- |
| BR-AG-DATA-011-DATA-AG-AG-TOOL-062   | DATA   | AG-TOOL | src-tauri/src/ag/turn_tool_loop.rs | 583 |
| BR-AG-DATA-011-EFFECT-AG-AG-TOOL-070 | EFFECT | AG-TOOL | src-tauri/src/ag/turn_tool_loop.rs | 978 |

#### BR-AG-DATA-012

| codes                              | type | chain   | 文件                                 | 行号  |
| ---------------------------------- | ---- | ------- | ---------------------------------- | --- |
| BR-AG-DATA-012-DATA-AG-AG-TOOL-063 | DATA | AG-TOOL | src-tauri/src/ag/turn_tool_loop.rs | 654 |

#### BR-AG-DATA-013

| codes                            | type | chain            | 文件                                 | 行号  |
| -------------------------------- | ---- | ---------------- | ---------------------------------- | --- |
| BR-AG-DATA-013-RB-AG-AG-TOOL-065 | RB   | AG-TOOL, AG-TURN | src-tauri/src/ag/turn_tool_loop.rs | 772 |

#### BR-AG-OBS-001

| codes                                                                                                                                                                                                                                                   | type   | chain            | 文件                                      | 行号   |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ---------------- | --------------------------------------- | ---- |
| BR-AG-OBS-001-RB-AG-AG-TURN-050, BR-AG-UI-001-RB-AG-AG-TURN-050                                                                                                                                                                                         | RB     | AG-TURN          | src/components/ag/ProjectReportCard.tsx | 5    |
| BR-AG-OBS-001-EFFECT-AG-AG-TURN-035, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-035, BR-AG-STATE-001-EFFECT-AG-AG-TURN-035                                                                                                                                      | EFFECT | AG-TURN          | src/services/agService.ts               | 2178 |
| BR-AG-OBS-001-EFFECT-AG-AG-TURN-042, BR-AG-STATE-001-EFFECT-AG-AG-TURN-042, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-042, BR-SYS-STATE-008-EFFECT-AG-AG-TURN-042                                                                                              | EFFECT | AG-TURN          | src/store/agStore.ts                    | 560  |
| BR-AG-OBS-001-DATA-AG-AG-TOOL-043, BR-AG-DATA-008-DATA-AG-AG-TOOL-043, BR-SYS-DATA-008-DATA-AG-AG-TOOL-043                                                                                                                                              | DATA   | AG-TURN, AG-TOOL | src-tauri/src/ag/tool_exec.rs           | 173  |
| BR-AG-DATA-005-DATA-AG-AG-TOOL-059, BR-AG-OBS-001-DATA-AG-AG-TOOL-059                                                                                                                                                                                   | DATA   | AG-TOOL          | src-tauri/src/ag/tool_exec.rs           | 844  |
| BR-AG-DATA-005-GUARD-AG-AG-TOOL-057, BR-AG-OBS-001-GUARD-AG-AG-TOOL-057                                                                                                                                                                                 | GUARD  | AG-TOOL          | src-tauri/src/ag/tool_exec.rs           | 922  |
| BR-AG-OBS-001-EFFECT-AG-AG-TURN-044, BR-AG-STATE-001-EFFECT-AG-AG-TURN-044, BR-AG-STATE-002-EFFECT-AG-AG-TURN-044, BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-044, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-044                                                      | EFFECT | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_events.rs         | 476  |
| BR-AG-DATA-005-IO-AG-AG-TURN-056, BR-AG-OBS-001-IO-AG-AG-TURN-056                                                                                                                                                                                       | IO     | AG-TURN          | src-tauri/src/ag/turn_events.rs         | 730  |
| BR-AG-OBS-001-QUERY-AG-AG-TURN-048, BR-AG-PERSIST-001-QUERY-AG-AG-TURN-048, BR-SYS-DATA-008-QUERY-AG-AG-TURN-048                                                                                                                                        | QUERY  | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_events.rs         | 988  |
| BR-AG-OBS-001-API-AG-AG-TURN-018, BR-AG-STATE-001-API-AG-AG-TURN-018, BR-AG-STATE-004-API-AG-AG-TURN-018, BR-AG-DATA-008-API-AG-AG-TURN-018, BR-ED-DATA-008-API-AG-AG-TURN-018, BR-SYS-STATE-007-API-AG-AG-TURN-018, BR-SYS-DATA-008-API-AG-AG-TURN-018 | API    | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_runtime.rs        | 753  |
| BR-AG-STATE-005-EFFECT-AG-AG-TURN-079, BR-AG-OBS-001-EFFECT-AG-AG-TURN-079                                                                                                                                                                              | EFFECT | AG-TURN          | src-tauri/src/ag/turn_tool_loop.rs      | 2614 |
| BR-AG-OBS-001-API-AG-AG-TURN-061, BR-AG-STATE-001-API-AG-AG-TURN-061                                                                                                                                                                                    | API    | AG-TURN          | src-tauri/src/ag/verification.rs        | 1156 |
| BR-AG-OBS-001-DATA-AG-AG-TURN-043, BR-AG-STATE-001-DATA-AG-AG-TURN-043, BR-AG-STATE-002-DATA-AG-AG-TURN-043, BR-AG-PERSIST-001-DATA-AG-AG-TURN-043, BR-SYS-STATE-007-DATA-AG-AG-TURN-043                                                                | DATA   | AG-TURN, AG-TOOL | src-tauri/src/ipc/mod.rs                | 127  |

#### BR-AG-OBS-001 term_ref: TERM-AG-021

| codes                               | type   | chain   | 文件                                 | 行号  |
| ----------------------------------- | ------ | ------- | ---------------------------------- | --- |
| BR-AG-OBS-001-EFFECT-AG-AG-TURN-072 | EFFECT | AG-TURN | src-tauri/src/ag/turn_tool_loop.rs | 901 |

#### BR-AG-PERSIST-001

| codes                                                                                                                                                                                              | type   | chain             | 文件                                     | 行号   |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ----------------- | -------------------------------------- | ---- |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-067, BR-SYS-STATE-007-QUERY-AG-AG-TURN-067                                                                                                                      | QUERY  | AG-TURN           | src/services/agService.ts              | 1850 |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-068, BR-SYS-STATE-007-QUERY-AG-AG-TURN-068                                                                                                                      | QUERY  | AG-TURN, IPC-CALL | src/services/agService.ts              | 1889 |
| BR-AG-PERSIST-001-QUERY-AG-AG-TOOL-046, BR-SYS-STATE-007-QUERY-AG-AG-TOOL-046                                                                                                                      | QUERY  | AG-TOOL, AG-TURN  | src-tauri/src/ag/capability_runtime.rs | 66   |
| BR-AG-STATE-002-EFFECT-AG-AG-TOOL-048, BR-SYS-STATE-007-EFFECT-AG-AG-TOOL-048, BR-AG-PERSIST-001-EFFECT-AG-AG-TOOL-048                                                                             | EFFECT | AG-TOOL, AG-TURN  | src-tauri/src/ag/capability_runtime.rs | 239  |
| BR-AG-STATE-002-EFFECT-AG-AG-TOOL-046, BR-SYS-STATE-007-EFFECT-AG-AG-TOOL-046, BR-AG-PERSIST-001-EFFECT-AG-AG-TOOL-046                                                                             | EFFECT | AG-TOOL, AG-TURN  | src-tauri/src/ag/capability_runtime.rs | 308  |
| BR-AG-PERSIST-001-GUARD-AG-AG-TURN-048, BR-SYS-STATE-007-GUARD-AG-AG-TURN-048                                                                                                                      | GUARD  | AG-TURN           | src-tauri/src/ag/history.rs            | 366  |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-020                                                                                                                                                             | QUERY  | AG-TURN           | src-tauri/src/ag/history_routes.rs     | 7    |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-021                                                                                                                                                            | EFFECT | AG-TURN           | src-tauri/src/ag/history_routes.rs     | 23   |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-022                                                                                                                                                            | EFFECT | AG-TURN           | src-tauri/src/ag/history_routes.rs     | 72   |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-023                                                                                                                                                             | QUERY  | AG-TURN           | src-tauri/src/ag/history_routes.rs     | 111  |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-047, BR-SYS-STATE-007-QUERY-AG-AG-TURN-047                                                                                                                      | QUERY  | AG-TURN           | src-tauri/src/ag/history_routes.rs     | 131  |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-024                                                                                                                                                            | EFFECT | AG-TURN           | src-tauri/src/ag/history_routes.rs     | 171  |
| BR-AG-STATE-001-API-AG-AG-TURN-019, BR-AG-PERSIST-001-API-AG-AG-TURN-019, BR-SYS-STATE-007-API-AG-AG-TURN-019                                                                                      | API    | AG-TURN           | src-tauri/src/ag/turn_control.rs       | 620  |
| BR-AG-OBS-001-EFFECT-AG-AG-TURN-044, BR-AG-STATE-001-EFFECT-AG-AG-TURN-044, BR-AG-STATE-002-EFFECT-AG-AG-TURN-044, BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-044, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-044 | EFFECT | AG-TURN, AG-TOOL  | src-tauri/src/ag/turn_events.rs        | 476  |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-045, BR-SYS-STATE-007-QUERY-AG-AG-TURN-045                                                                                                                      | QUERY  | AG-TURN           | src-tauri/src/ag/turn_events.rs        | 933  |
| BR-AG-OBS-001-QUERY-AG-AG-TURN-048, BR-AG-PERSIST-001-QUERY-AG-AG-TURN-048, BR-SYS-DATA-008-QUERY-AG-AG-TURN-048                                                                                   | QUERY  | AG-TURN, AG-TOOL  | src-tauri/src/ag/turn_events.rs        | 988  |
| BR-AG-OBS-001-DATA-AG-AG-TURN-043, BR-AG-STATE-001-DATA-AG-AG-TURN-043, BR-AG-STATE-002-DATA-AG-AG-TURN-043, BR-AG-PERSIST-001-DATA-AG-AG-TURN-043, BR-SYS-STATE-007-DATA-AG-AG-TURN-043           | DATA   | AG-TURN, AG-TOOL  | src-tauri/src/ipc/mod.rs               | 127  |

#### BR-AG-PERSIST-001 term_ref: TERM-AG-001

| codes                                                                          | type   | chain                 | 文件                               | 行号   |
| ------------------------------------------------------------------------------ | ------ | --------------------- | -------------------------------- | ---- |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-003                                        | EFFECT | AG-TURN               | src-tauri/src/ag/history.rs      | 304  |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-046                                        | EFFECT | AG-TURN               | src-tauri/src/ag/history.rs      | 398  |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-003                                         | QUERY  | AG-TURN               | src-tauri/src/ag/history.rs      | 462  |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-031                                        | EFFECT | AG-TURN               | src-tauri/src/ag/history.rs      | 509  |
| BR-AG-STATE-001-API-AG-AG-TURN-070, BR-AG-PERSIST-001-API-AG-AG-TURN-070       | API    | AG-TURN, WS-OPERATION | src-tauri/src/ag/task_runtime.rs | 725  |
| BR-AG-STATE-001-EFFECT-AG-AG-TURN-069, BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-069 | EFFECT | AG-TURN, WS-OPERATION | src-tauri/src/ag/turn_events.rs  | 1245 |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-004                                         | QUERY  | AG-TURN               | src-tauri/src/de/routes.rs       | 630  |

#### BR-AG-PERSIST-001 term_ref: TERM-AG-009

| codes                                   | type   | chain   | 文件                          | 行号  |
| --------------------------------------- | ------ | ------- | --------------------------- | --- |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-030 | EFFECT | AG-TURN | src-tauri/src/ag/history.rs | 172 |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-002  | QUERY  | AG-TURN | src-tauri/src/ag/history.rs | 254 |

#### BR-AG-PERSIST-001 term_ref: TERM-AG-015

| codes                                                                                                                  | type   | chain   | 文件                       | 行号   |
| ---------------------------------------------------------------------------------------------------------------------- | ------ | ------- | ------------------------ | ---- |
| BR-SYS-STATE-007-EFFECT-AG-AG-TURN-047, BR-AG-STATE-001-EFFECT-AG-AG-TURN-047, BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-047 | EFFECT | AG-TURN | src-tauri/src/ag/task.rs | 859  |
| BR-SYS-STATE-008-QUERY-AG-AG-TASK-004, BR-AG-PERSIST-001-QUERY-AG-AG-TASK-004                                          | QUERY  | AG-TURN | src-tauri/src/ag/task.rs | 1137 |

#### BR-AG-STATE-001

| codes                                                                                                                                                                                                                                                   | type   | chain                 | 文件                                 | 行号   |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | --------------------- | ---------------------------------- | ---- |
| BR-AG-OBS-001-EFFECT-AG-AG-TURN-042, BR-AG-STATE-001-EFFECT-AG-AG-TURN-042, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-042, BR-SYS-STATE-008-EFFECT-AG-AG-TURN-042                                                                                              | EFFECT | AG-TURN               | src/store/agStore.ts               | 560  |
| BR-SYS-STATE-007-EFFECT-AG-AG-TURN-047, BR-AG-STATE-001-EFFECT-AG-AG-TURN-047, BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-047                                                                                                                                  | EFFECT | AG-TURN               | src-tauri/src/ag/task.rs           | 859  |
| BR-AG-STATE-001-API-AG-AG-TURN-070, BR-AG-PERSIST-001-API-AG-AG-TURN-070                                                                                                                                                                                | API    | AG-TURN, WS-OPERATION | src-tauri/src/ag/task_runtime.rs   | 725  |
| BR-AG-STATE-001-RB-AG-AG-TURN-001, BR-SYS-STATE-007-RB-AG-AG-TURN-001                                                                                                                                                                                   | RB     | AG-TURN               | src-tauri/src/ag/turn.rs           | 112  |
| BR-AG-STATE-001-API-AG-AG-TURN-019, BR-AG-PERSIST-001-API-AG-AG-TURN-019, BR-SYS-STATE-007-API-AG-AG-TURN-019                                                                                                                                           | API    | AG-TURN               | src-tauri/src/ag/turn_control.rs   | 620  |
| BR-AG-OBS-001-EFFECT-AG-AG-TURN-044, BR-AG-STATE-001-EFFECT-AG-AG-TURN-044, BR-AG-STATE-002-EFFECT-AG-AG-TURN-044, BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-044, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-044                                                      | EFFECT | AG-TURN, AG-TOOL      | src-tauri/src/ag/turn_events.rs    | 476  |
| BR-AG-STATE-001-EFFECT-AG-AG-TURN-069, BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-069                                                                                                                                                                          | EFFECT | AG-TURN, WS-OPERATION | src-tauri/src/ag/turn_events.rs    | 1245 |
| BR-AG-OBS-001-API-AG-AG-TURN-018, BR-AG-STATE-001-API-AG-AG-TURN-018, BR-AG-STATE-004-API-AG-AG-TURN-018, BR-AG-DATA-008-API-AG-AG-TURN-018, BR-ED-DATA-008-API-AG-AG-TURN-018, BR-SYS-STATE-007-API-AG-AG-TURN-018, BR-SYS-DATA-008-API-AG-AG-TURN-018 | API    | AG-TURN, AG-TOOL      | src-tauri/src/ag/turn_runtime.rs   | 753  |
| BR-AG-STATE-001-EFFECT-AG-AG-TURN-074                                                                                                                                                                                                                   | EFFECT | AG-TURN               | src-tauri/src/ag/turn_tool_loop.rs | 1107 |
| BR-AG-STATE-001-EFFECT-AG-AG-TURN-075                                                                                                                                                                                                                   | EFFECT | AG-TURN               | src-tauri/src/ag/turn_tool_loop.rs | 2537 |
| BR-AG-OBS-001-API-AG-AG-TURN-061, BR-AG-STATE-001-API-AG-AG-TURN-061                                                                                                                                                                                    | API    | AG-TURN               | src-tauri/src/ag/verification.rs   | 1156 |
| BR-AG-OBS-001-DATA-AG-AG-TURN-043, BR-AG-STATE-001-DATA-AG-AG-TURN-043, BR-AG-STATE-002-DATA-AG-AG-TURN-043, BR-AG-PERSIST-001-DATA-AG-AG-TURN-043, BR-SYS-STATE-007-DATA-AG-AG-TURN-043                                                                | DATA   | AG-TURN, AG-TOOL      | src-tauri/src/ipc/mod.rs           | 127  |

#### BR-AG-STATE-001 term_ref: TERM-AG-001

| codes                                                                                                              | type   | chain   | 文件                               | 行号   |
| ------------------------------------------------------------------------------------------------------------------ | ------ | ------- | -------------------------------- | ---- |
| BR-AG-OBS-001-EFFECT-AG-AG-TURN-035, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-035, BR-AG-STATE-001-EFFECT-AG-AG-TURN-035 | EFFECT | AG-TURN | src/services/agService.ts        | 2178 |
| BR-SYS-STATE-007-GUARD-AG-AG-TURN-034, BR-AG-STATE-001-GUARD-AG-AG-TURN-034                                        | GUARD  | AG-TURN | src-tauri/src/ag/turn_control.rs | 85   |
| BR-SYS-STATE-007-EFFECT-AG-AG-TURN-034, BR-AG-STATE-001-EFFECT-AG-AG-TURN-034                                      | EFFECT | AG-TURN | src-tauri/src/ag/turn_control.rs | 300  |

#### BR-AG-STATE-001 term_ref: TERM-AG-012

| codes                                                                     | type  | chain   | 文件                               | 行号 |
| ------------------------------------------------------------------------- | ----- | ------- | -------------------------------- | -- |
| BR-AG-DATA-004-DATA-AG-AG-TURN-046, BR-AG-STATE-001-DATA-AG-AG-TURN-046   | DATA  | AG-TURN | src-tauri/src/ag/context.rs      | 53 |
| BR-AG-DATA-004-GUARD-AG-AG-TURN-047, BR-AG-STATE-001-GUARD-AG-AG-TURN-047 | GUARD | AG-TURN | src-tauri/src/ag/model_output.rs | 54 |

#### BR-AG-STATE-001 term_ref: TERM-AG-015

| codes                                                                         | type   | chain   | 文件                       | 行号  |
| ----------------------------------------------------------------------------- | ------ | ------- | ------------------------ | --- |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-033, BR-AG-STATE-001-EFFECT-AG-AG-TURN-033 | EFFECT | AG-TURN | src-tauri/src/ag/task.rs | 929 |

#### BR-AG-STATE-002

| codes                                                                                                                                                                                                                                                | type   | chain                          | 文件                                     | 行号   |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------ | -------------------------------------- | ---- |
| BR-AG-STATE-002-EFFECT-AG-AG-TOOL-048, BR-SYS-STATE-007-EFFECT-AG-AG-TOOL-048, BR-AG-PERSIST-001-EFFECT-AG-AG-TOOL-048                                                                                                                               | EFFECT | AG-TOOL, AG-TURN               | src-tauri/src/ag/capability_runtime.rs | 239  |
| BR-AG-STATE-002-EFFECT-AG-AG-TOOL-046, BR-SYS-STATE-007-EFFECT-AG-AG-TOOL-046, BR-AG-PERSIST-001-EFFECT-AG-AG-TOOL-046                                                                                                                               | EFFECT | AG-TOOL, AG-TURN               | src-tauri/src/ag/capability_runtime.rs | 308  |
| BR-AG-STATE-002-EFFECT-AG-AG-TOOL-047, BR-AG-DATA-003-EFFECT-AG-AG-TOOL-047                                                                                                                                                                          | EFFECT | AG-TOOL, DE-DIFF               | src-tauri/src/ag/edit_runtime.rs       | 18   |
| BR-AG-STATE-002-EFFECT-AG-AG-TOOL-017, BR-AG-DATA-003-EFFECT-AG-AG-TOOL-017, BR-DE-DATA-001-EFFECT-AG-AG-TOOL-017, BR-DE-DATA-002-EFFECT-AG-AG-TOOL-017                                                                                              | EFFECT | AG-TOOL, DE-DIFF, ED-EDIT      | src-tauri/src/ag/edit_runtime.rs       | 73   |
| BR-AG-STATE-002-GUARD-AG-AG-TOOL-058, BR-AG-DATA-009-GUARD-AG-AG-TOOL-058                                                                                                                                                                            | GUARD  | AG-TOOL                        | src-tauri/src/ag/edit_support.rs       | 102  |
| BR-AG-STATE-002-GUARD-AG-AG-TOOL-059, BR-AG-DATA-010-GUARD-AG-AG-TOOL-059                                                                                                                                                                            | GUARD  | AG-TOOL                        | src-tauri/src/ag/edit_support.rs       | 293  |
| BR-AG-STATE-002-API-AG-WS-OPERATION-005, BR-AG-DATA-003-API-AG-WS-OPERATION-005, BR-SYS-STATE-008-API-AG-WS-OPERATION-005, BR-WS-STATE-003-API-AG-WS-OPERATION-005, BR-WS-PERSIST-003-API-AG-WS-OPERATION-005, BR-WS-OBS-002-API-AG-WS-OPERATION-005 | API    | AG-TURN, AG-TOOL, WS-OPERATION | src-tauri/src/ag/task_runtime.rs       | 197  |
| BR-AG-STATE-002-GUARD-AG-AG-TOOL-001                                                                                                                                                                                                                 | GUARD  | AG-TOOL                        | src-tauri/src/ag/tool.rs               | 102  |
| BR-AG-STATE-002-DATA-AG-AG-TOOL-044, BR-SYS-DATA-008-DATA-AG-AG-TOOL-044                                                                                                                                                                             | DATA   | AG-TOOL                        | src-tauri/src/ag/tool_exec.rs          | 18   |
| BR-AG-STATE-002-QUERY-AG-AG-TOOL-012, BR-AG-DATA-001-QUERY-AG-AG-TOOL-012, BR-WS-DATA-001-QUERY-AG-AG-TOOL-012                                                                                                                                       | QUERY  | AG-TOOL                        | src-tauri/src/ag/tool_exec.rs          | 483  |
| BR-AG-STATE-002-GUARD-AG-AG-TOOL-045, BR-SYS-STATE-007-GUARD-AG-AG-TOOL-045                                                                                                                                                                          | GUARD  | AG-TOOL                        | src-tauri/src/ag/tool_exec.rs          | 954  |
| BR-AG-STATE-002-DATA-AG-AG-TOOL-039, BR-AG-DATA-004-DATA-AG-AG-TOOL-039                                                                                                                                                                              | DATA   | AG-TURN, AG-TOOL               | src-tauri/src/ag/tool_registry.rs      | 169  |
| BR-AG-STATE-002-GUARD-AG-AG-TOOL-035, BR-SYS-STATE-007-GUARD-AG-AG-TOOL-035, BR-AG-DATA-003-GUARD-AG-AG-TOOL-035, BR-WS-DATA-001-GUARD-AG-AG-TOOL-035                                                                                                | GUARD  | AG-TOOL                        | src-tauri/src/ag/tool_registry.rs      | 456  |
| BR-AG-OBS-001-EFFECT-AG-AG-TURN-044, BR-AG-STATE-001-EFFECT-AG-AG-TURN-044, BR-AG-STATE-002-EFFECT-AG-AG-TURN-044, BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-044, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-044                                                   | EFFECT | AG-TURN, AG-TOOL               | src-tauri/src/ag/turn_events.rs        | 476  |
| BR-AG-STATE-002-RB-AG-AG-TOOL-068                                                                                                                                                                                                                    | RB     | AG-TOOL                        | src-tauri/src/ag/turn_tool_loop.rs     | 1301 |
| BR-AG-STATE-002-RB-AG-AG-TOOL-067                                                                                                                                                                                                                    | RB     | AG-TOOL                        | src-tauri/src/ag/turn_tool_loop.rs     | 1849 |
| BR-AG-STATE-002-GUARD-AG-AG-TOOL-066                                                                                                                                                                                                                 | GUARD  | AG-TOOL                        | src-tauri/src/ag/turn_tool_loop.rs     | 1990 |
| BR-AG-STATE-002-RB-AG-AG-TOOL-069                                                                                                                                                                                                                    | RB     | AG-TOOL                        | src-tauri/src/ag/turn_tool_loop.rs     | 2143 |
| BR-AG-OBS-001-DATA-AG-AG-TURN-043, BR-AG-STATE-001-DATA-AG-AG-TURN-043, BR-AG-STATE-002-DATA-AG-AG-TURN-043, BR-AG-PERSIST-001-DATA-AG-AG-TURN-043, BR-SYS-STATE-007-DATA-AG-AG-TURN-043                                                             | DATA   | AG-TURN, AG-TOOL               | src-tauri/src/ipc/mod.rs               | 127  |

#### BR-AG-STATE-002 term_ref: TERM-AG-002

| codes                                 | type   | chain   | 文件                                 | 行号  |
| ------------------------------------- | ------ | ------- | ---------------------------------- | --- |
| BR-AG-STATE-002-RB-AG-AG-TOOL-001     | RB     | AG-TOOL | src-tauri/src/ag/tool.rs           | 60  |
| BR-AG-STATE-002-EFFECT-AG-AG-TOOL-038 | EFFECT | AG-TOOL | src-tauri/src/ag/turn_tool_loop.rs | 555 |

#### BR-AG-STATE-002 term_ref: TERM-AG-014

| codes                             | type | chain            | 文件                                 | 行号   |
| --------------------------------- | ---- | ---------------- | ---------------------------------- | ---- |
| BR-AG-STATE-002-RB-AG-AG-TOOL-018 | RB   | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_tool_loop.rs | 2677 |

#### BR-AG-STATE-002 term_ref: TERM-AG-019

| codes                                                                     | type  | chain   | 文件                                | 行号  |
| ------------------------------------------------------------------------- | ----- | ------- | --------------------------------- | --- |
| BR-AG-DATA-003-GUARD-AG-AG-TOOL-033, BR-AG-STATE-002-GUARD-AG-AG-TOOL-033 | GUARD | AG-TOOL | src-tauri/src/ag/tool_registry.rs | 850 |

#### BR-AG-STATE-003

| codes                                                                             | type   | chain     | 文件                                 | 行号 |
| --------------------------------------------------------------------------------- | ------ | --------- | ---------------------------------- | -- |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-001                                               | RB     | AG-ASSIST | src-tauri/src/ag/assist.rs         | 40 |
| BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-010                                           | EFFECT | AG-ASSIST | src-tauri/src/ag/assist_runtime.rs | 50 |
| BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-001, BR-AG-VERIFY-001-EFFECT-AG-AG-ASSIST-001 | EFFECT | AG-ASSIST | src-tauri/src/ag/assist_runtime.rs | 92 |

#### BR-AG-STATE-003 term_ref: TERM-AG-003

| codes                                   | type   | chain     | 文件                                   | 行号   |
| --------------------------------------- | ------ | --------- | ------------------------------------ | ---- |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-007     | RB     | AG-ASSIST | src/components/ed/Editor.tsx         | 810  |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-002     | RB     | AG-ASSIST | src/components/ed/Editor.tsx         | 1938 |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-004     | RB     | AG-ASSIST | src/components/ed/assistPlacement.ts | 36   |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-005     | RB     | AG-ASSIST | src/components/ed/assistPlacement.ts | 60   |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-006     | RB     | AG-ASSIST | src/components/ed/assistPlacement.ts | 78   |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-003     | RB     | AG-ASSIST | src/components/ed/assistPlacement.ts | 96   |
| BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-002 | EFFECT | AG-ASSIST | src/services/agService.ts            | 2111 |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-008     | RB     | AG-ASSIST | src/store/assistStore.ts             | 94   |

#### BR-AG-STATE-003 term_ref: TERM-AG-004

| codes                                   | type   | chain     | 文件                                        | 行号   |
| --------------------------------------- | ------ | --------- | ----------------------------------------- | ---- |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-009     | RB     | AG-ASSIST | src/components/ag/AutocompleteOverlay.tsx | 16   |
| BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-003 | EFFECT | AG-ASSIST | src/components/ed/Editor.tsx              | 1008 |

#### BR-AG-STATE-004

| codes                                                                                                                                                                                                                                                   | type | chain            | 文件                               | 行号  |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---- | ---------------- | -------------------------------- | --- |
| BR-AG-OBS-001-API-AG-AG-TURN-018, BR-AG-STATE-001-API-AG-AG-TURN-018, BR-AG-STATE-004-API-AG-AG-TURN-018, BR-AG-DATA-008-API-AG-AG-TURN-018, BR-ED-DATA-008-API-AG-AG-TURN-018, BR-SYS-STATE-007-API-AG-AG-TURN-018, BR-SYS-DATA-008-API-AG-AG-TURN-018 | API  | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_runtime.rs | 753 |

#### BR-AG-STATE-005

| codes                                                                      | type   | chain   | 文件                                 | 行号   |
| -------------------------------------------------------------------------- | ------ | ------- | ---------------------------------- | ---- |
| BR-AG-STATE-005-EFFECT-AG-AG-TURN-071                                      | EFFECT | AG-TURN | src-tauri/src/ag/turn_tool_loop.rs | 686  |
| BR-AG-STATE-005-EFFECT-AG-AG-TURN-079, BR-AG-OBS-001-EFFECT-AG-AG-TURN-079 | EFFECT | AG-TURN | src-tauri/src/ag/turn_tool_loop.rs | 2614 |

#### BR-AG-UI-001

| codes                          | type | chain   | 文件                                           | 行号 |
| ------------------------------ | ---- | ------- | -------------------------------------------- | -- |
| BR-AG-UI-001-IO-AG-AG-TURN-062 | IO   | AG-TURN | src/components/ag/AgentVerificationPanel.tsx | 15 |

#### BR-AG-UI-001 term_ref: TERM-AG-011

| codes                          | type | chain   | 文件                                    | 行号 |
| ------------------------------ | ---- | ------- | ------------------------------------- | -- |
| BR-AG-UI-001-RB-AG-AG-TURN-013 | RB   | AG-TURN | src/components/ag/MessageFlow.tsx     | 9  |
| BR-AG-UI-001-RB-AG-AG-TURN-014 | RB   | AG-TURN | src/components/ag/MessageList.tsx     | 12 |
| BR-AG-UI-001-RB-AG-AG-TURN-012 | RB   | AG-TURN | src/components/ag/MessageMarkdown.tsx | 8  |

#### BR-AG-UI-001 term_ref: TERM-AG-017

| codes                                                                      | type   | chain   | 文件                                | 行号 |
| -------------------------------------------------------------------------- | ------ | ------- | --------------------------------- | -- |
| BR-SYS-STATE-007-EFFECT-AG-AG-TURN-058, BR-AG-UI-001-EFFECT-AG-AG-TURN-058 | EFFECT | AG-TURN | src/components/ag/DecisionBar.tsx | 40 |

#### BR-AG-UI-001 term_ref: TERM-AG-021

| codes                                                           | type | chain   | 文件                                      | 行号 |
| --------------------------------------------------------------- | ---- | ------- | --------------------------------------- | -- |
| BR-AG-OBS-001-RB-AG-AG-TURN-050, BR-AG-UI-001-RB-AG-AG-TURN-050 | RB   | AG-TURN | src/components/ag/ProjectReportCard.tsx | 5  |

#### BR-AG-VERIFY-001

| codes                                                                             | type   | chain     | 文件                                 | 行号   |
| --------------------------------------------------------------------------------- | ------ | --------- | ---------------------------------- | ---- |
| BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-001, BR-AG-VERIFY-001-EFFECT-AG-AG-ASSIST-001 | EFFECT | AG-ASSIST | src-tauri/src/ag/assist_runtime.rs | 92   |
| BR-AG-VERIFY-001-GUARD-AG-AG-TURN-020                                             | GUARD  | AG-TURN   | src-tauri/src/ag/turn.rs           | 160  |
| BR-AG-VERIFY-001-RB-AG-AG-TURN-076                                                | RB     | AG-TURN   | src-tauri/src/ag/turn_tool_loop.rs | 2291 |
| BR-AG-VERIFY-001-EFFECT-AG-AG-TURN-077                                            | EFFECT | AG-TURN   | src-tauri/src/ag/turn_tool_loop.rs | 2374 |

#### BR-AST-DATA-001

| codes                                    | type  | chain      | 文件                         | 行号 |
| ---------------------------------------- | ----- | ---------- | -------------------------- | -- |
| BR-AST-DATA-001-QUERY-AST-AST-MEMORY-004 | QUERY | AST-MEMORY | src-tauri/src/ast/store.rs | 22 |
| BR-AST-DATA-001-QUERY-AST-AST-MEMORY-002 | QUERY | AST-MEMORY | src-tauri/src/ast/store.rs | 50 |

#### BR-AST-DATA-001 term_ref: TERM-AST-002

| codes                                                                              | type  | chain                            | 文件                          | 行号 |
| ---------------------------------------------------------------------------------- | ----- | -------------------------------- | --------------------------- | -- |
| BR-SYS-DATA-007-API-AST-AST-MEMORY-001, BR-AST-DATA-001-API-AST-AST-MEMORY-001     | API   | AST-MEMORY, REF-INJECT, IPC-CALL | src-tauri/src/ast/routes.rs | 6  |
| BR-SYS-DATA-007-QUERY-AST-AST-MEMORY-003, BR-AST-DATA-001-QUERY-AST-AST-MEMORY-003 | QUERY | AST-MEMORY, REF-INJECT           | src-tauri/src/ast/store.rs  | 72 |

#### BR-AST-DATA-002

| codes                                    | type  | chain      | 文件                          | 行号 |
| ---------------------------------------- | ----- | ---------- | --------------------------- | -- |
| BR-AST-DATA-002-QUERY-AST-AST-MEMORY-001 | QUERY | AST-MEMORY | src-tauri/src/ast/supply.rs | 23 |

#### BR-AST-STATE-001

| codes                                      | type   | chain      | 文件                           | 行号  |
| ------------------------------------------ | ------ | ---------- | ---------------------------- | --- |
| BR-AST-STATE-001-RB-AST-AST-MEMORY-001     | RB     | AST-MEMORY | src-tauri/src/ast/capture.rs | 56  |
| BR-AST-STATE-001-EFFECT-AST-AST-MEMORY-001 | EFFECT | AST-MEMORY | src-tauri/src/ast/capture.rs | 91  |
| BR-AST-STATE-001-EFFECT-AST-AST-MEMORY-002 | EFFECT | AST-MEMORY | src-tauri/src/ast/capture.rs | 135 |

#### BR-CBT-DATA-001

| codes                                  | type | chain     | 文件                         | 行号 |
| -------------------------------------- | ---- | --------- | -------------------------- | -- |
| BR-CBT-DATA-001-DATA-CBT-CBT-BUILD-001 | DATA | CBT-BUILD | src-tauri/src/cbt/apply.rs | 12 |
| BR-CBT-DATA-001-DATA-CBT-CBT-BUILD-002 | DATA | CBT-BUILD | src-tauri/src/cbt/apply.rs | 37 |

#### BR-CBT-PERSIST-001

| codes                                       | type   | chain     | 文件                         | 行号  |
| ------------------------------------------- | ------ | --------- | -------------------------- | --- |
| BR-CBT-PERSIST-001-EFFECT-CBT-CBT-BUILD-001 | EFFECT | CBT-BUILD | src-tauri/src/cbt/build.rs | 99  |
| BR-CBT-PERSIST-001-QUERY-CBT-CBT-BUILD-001  | QUERY  | CBT-BUILD | src-tauri/src/cbt/build.rs | 128 |

#### BR-CBT-STATE-001

| codes                                 | type | chain     | 文件                         | 行号 |
| ------------------------------------- | ---- | --------- | -------------------------- | -- |
| BR-CBT-STATE-001-RB-CBT-CBT-BUILD-002 | RB   | CBT-BUILD | src-tauri/src/cbt/build.rs | 59 |

#### BR-CBT-STATE-002

| codes                                    | type  | chain     | 文件                            | 行号 |
| ---------------------------------------- | ----- | --------- | ----------------------------- | -- |
| BR-CBT-STATE-002-GUARD-CBT-CBT-BUILD-001 | GUARD | CBT-BUILD | src-tauri/src/cbt/takeover.rs | 35 |
| BR-CBT-STATE-002-RB-CBT-CBT-BUILD-001    | RB    | CBT-BUILD | src-tauri/src/cbt/takeover.rs | 54 |

#### BR-CORE-STATE-001

| codes                                                                                                                                                         | type   | chain                                    | 文件                           | 行号   |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ---------------------------------------- | ---------------------------- | ---- |
| BR-CORE-STATE-001-EFFECT-ED-ED-EDIT-001, BR-CORE-STATE-003-EFFECT-ED-ED-EDIT-001, BR-DE-DATA-001-EFFECT-ED-ED-EDIT-001, BR-DE-STATE-001-EFFECT-ED-ED-EDIT-001 | EFFECT | DE-DIFF, ED-EDIT, ED-DECOR               | src/components/ed/Editor.tsx | 1725 |
| BR-CORE-STATE-001-EFFECT-DE-ED-EDIT-010                                                                                                                       | EFFECT | DE-DIFF, ED-EDIT                         | src/services/deService.ts    | 238  |
| BR-CORE-STATE-001-API-DE-DE-DIFF-001, BR-CORE-STATE-002-API-DE-DE-DIFF-001, BR-CORE-STATE-003-API-DE-DE-DIFF-001                                              | API    | DE-DIFF, ED-EDIT, WS-SAVE, WS-DIFF-APPLY | src/services/deService.ts    | 354  |
| BR-CORE-STATE-001-EFFECT-ED-WS-SAVE-001, BR-CORE-STATE-003-EFFECT-ED-WS-SAVE-001                                                                              | EFFECT | DE-DIFF, ED-EDIT, WS-SAVE                | src/services/edService.ts    | 91   |
| BR-CORE-STATE-001-EFFECT-DE-DE-DIFF-005                                                                                                                       | EFFECT | DE-DIFF, ED-EDIT, WS-SAVE                | src-tauri/src/de/routes.rs   | 123  |

#### BR-CORE-STATE-002

| codes                                                                                                            | type   | chain                                    | 文件                         | 行号  |
| ---------------------------------------------------------------------------------------------------------------- | ------ | ---------------------------------------- | -------------------------- | --- |
| BR-CORE-STATE-001-API-DE-DE-DIFF-001, BR-CORE-STATE-002-API-DE-DE-DIFF-001, BR-CORE-STATE-003-API-DE-DE-DIFF-001 | API    | DE-DIFF, ED-EDIT, WS-SAVE, WS-DIFF-APPLY | src/services/deService.ts  | 354 |
| BR-CORE-STATE-002-EFFECT-DE-DE-DIFF-007                                                                          | EFFECT | DE-DIFF, WS-DIFF-APPLY                   | src-tauri/src/de/routes.rs | 285 |

#### BR-CORE-STATE-003

| codes                                                                                                                                                         | type   | chain                                    | 文件                           | 行号   |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ---------------------------------------- | ---------------------------- | ---- |
| BR-CORE-STATE-001-EFFECT-ED-ED-EDIT-001, BR-CORE-STATE-003-EFFECT-ED-ED-EDIT-001, BR-DE-DATA-001-EFFECT-ED-ED-EDIT-001, BR-DE-STATE-001-EFFECT-ED-ED-EDIT-001 | EFFECT | DE-DIFF, ED-EDIT, ED-DECOR               | src/components/ed/Editor.tsx | 1725 |
| BR-CORE-STATE-003-EFFECT-DE-ED-EDIT-011                                                                                                                       | EFFECT | DE-DIFF, ED-EDIT, ED-DECOR, WS-SAVE      | src/services/deService.ts    | 297  |
| BR-CORE-STATE-001-API-DE-DE-DIFF-001, BR-CORE-STATE-002-API-DE-DE-DIFF-001, BR-CORE-STATE-003-API-DE-DE-DIFF-001                                              | API    | DE-DIFF, ED-EDIT, WS-SAVE, WS-DIFF-APPLY | src/services/deService.ts    | 354  |
| BR-CORE-STATE-001-EFFECT-ED-WS-SAVE-001, BR-CORE-STATE-003-EFFECT-ED-WS-SAVE-001                                                                              | EFFECT | DE-DIFF, ED-EDIT, WS-SAVE                | src/services/edService.ts    | 91   |
| BR-CORE-STATE-003-EFFECT-DE-DE-DIFF-006                                                                                                                       | EFFECT | DE-DIFF, ED-EDIT, ED-DECOR, WS-SAVE      | src-tauri/src/de/routes.rs   | 252  |

#### BR-DE-DATA-001

| codes                                                                                                                                                         | type   | chain                      | 文件                                  | 行号   |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | -------------------------- | ----------------------------------- | ---- |
| BR-DE-DATA-002-RB-DE-DE-DIFF-003, BR-DE-DATA-001-RB-DE-DE-DIFF-003, BR-DE-STATE-001-RB-DE-DE-DIFF-003                                                         | RB     | DE-DIFF                    | src/components/de/DiffCard.tsx      | 241  |
| BR-CORE-STATE-001-EFFECT-ED-ED-EDIT-001, BR-CORE-STATE-003-EFFECT-ED-ED-EDIT-001, BR-DE-DATA-001-EFFECT-ED-ED-EDIT-001, BR-DE-STATE-001-EFFECT-ED-ED-EDIT-001 | EFFECT | DE-DIFF, ED-EDIT, ED-DECOR | src/components/ed/Editor.tsx        | 1725 |
| BR-DE-DATA-001-RB-DE-DE-DIFF-007, BR-DE-STATE-001-RB-DE-DE-DIFF-007                                                                                           | RB     | DE-DIFF, ED-EDIT           | src/store/deStore.ts                | 222  |
| BR-DE-DATA-002-DATA-AG-AG-TOOL-060, BR-DE-DATA-001-DATA-AG-AG-TOOL-060                                                                                        | DATA   | AG-TOOL, DE-DIFF           | src-tauri/src/ag/edit_projection.rs | 12   |
| BR-AG-STATE-002-EFFECT-AG-AG-TOOL-017, BR-AG-DATA-003-EFFECT-AG-AG-TOOL-017, BR-DE-DATA-001-EFFECT-AG-AG-TOOL-017, BR-DE-DATA-002-EFFECT-AG-AG-TOOL-017       | EFFECT | AG-TOOL, DE-DIFF, ED-EDIT  | src-tauri/src/ag/edit_runtime.rs    | 73   |
| BR-DE-DATA-001-GUARD-DE-DE-DIFF-005                                                                                                                           | GUARD  | DE-DIFF                    | src-tauri/src/de/generate.rs        | 87   |
| BR-DE-DATA-001-IO-DE-DE-DIFF-006, BR-DE-DATA-002-IO-DE-DE-DIFF-006                                                                                            | IO     | DE-DIFF                    | src-tauri/src/de/store.rs           | 66   |

#### BR-DE-DATA-001 term_ref: TERM-DE-005

| codes                            | type | chain   | 文件                            | 行号  |
| -------------------------------- | ---- | ------- | ----------------------------- | --- |
| BR-DE-DATA-001-RB-DE-DE-DIFF-006 | RB   | DE-DIFF | src-tauri/src/ag/block_ops.rs | 579 |

#### BR-DE-DATA-001 term_ref: TERM-ED-001

| codes                                                                                                | type | chain   | 文件                            | 行号  |
| ---------------------------------------------------------------------------------------------------- | ---- | ------- | ----------------------------- | --- |
| BR-AG-DATA-003-RB-AG-AG-TOOL-016, BR-ED-DATA-001-RB-AG-AG-TOOL-016, BR-DE-DATA-001-RB-AG-AG-TOOL-016 | RB   | AG-TOOL | src-tauri/src/ag/block_ops.rs | 305 |

#### BR-DE-DATA-002

| codes                                                                                                                                                   | type   | chain                     | 文件                                  | 行号  |
| ------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------------------- | ----------------------------------- | --- |
| BR-DE-DATA-002-RB-DE-DE-DIFF-003, BR-DE-DATA-001-RB-DE-DE-DIFF-003, BR-DE-STATE-001-RB-DE-DE-DIFF-003                                                   | RB     | DE-DIFF                   | src/components/de/DiffCard.tsx      | 241 |
| BR-DE-DATA-002-DATA-AG-AG-TOOL-060, BR-DE-DATA-001-DATA-AG-AG-TOOL-060                                                                                  | DATA   | AG-TOOL, DE-DIFF          | src-tauri/src/ag/edit_projection.rs | 12  |
| BR-AG-STATE-002-EFFECT-AG-AG-TOOL-017, BR-AG-DATA-003-EFFECT-AG-AG-TOOL-017, BR-DE-DATA-001-EFFECT-AG-AG-TOOL-017, BR-DE-DATA-002-EFFECT-AG-AG-TOOL-017 | EFFECT | AG-TOOL, DE-DIFF, ED-EDIT | src-tauri/src/ag/edit_runtime.rs    | 73  |
| BR-DE-DATA-002-DATA-DE-DE-DIFF-001                                                                                                                      | DATA   | DE-DIFF                   | src-tauri/src/de/generate.rs        | 41  |

#### BR-DE-DATA-002 term_ref: TERM-DE-001

| codes                                                              | type | chain   | 文件                        | 行号 |
| ------------------------------------------------------------------ | ---- | ------- | ------------------------- | -- |
| BR-DE-DATA-001-IO-DE-DE-DIFF-006, BR-DE-DATA-002-IO-DE-DE-DIFF-006 | IO   | DE-DIFF | src-tauri/src/de/store.rs | 66 |

#### BR-DE-DATA-002 term_ref: TERM-DE-004

| codes                                | type   | chain   | 文件                                    | 行号  |
| ------------------------------------ | ------ | ------- | ------------------------------------- | --- |
| BR-DE-DATA-002-EFFECT-DE-DE-DIFF-009 | EFFECT | DE-DIFF | src/services/deService.ts             | 165 |
| BR-DE-DATA-002-RB-ED-DE-DIFF-028     | RB     | DE-DIFF | src/services/edImageOps.ts            | 493 |
| BR-DE-DATA-002-IO-DE-DE-DIFF-002     | IO     | DE-DIFF | src-tauri/src/de/store.rs             | 31  |
| BR-DE-DATA-002-QUERY-DE-DE-DIFF-002  | QUERY  | DE-DIFF | src-tauri/src/de/store.rs             | 101 |
| BR-DE-DATA-002-EFFECT-WS-DE-DIFF-004 | EFFECT | DE-DIFF | src-tauri/src/workspace/diff_apply.rs | 232 |

#### BR-DE-DATA-003

| codes                               | type  | chain   | 文件                            | 行号 |
| ----------------------------------- | ----- | ------- | ----------------------------- | -- |
| BR-DE-DATA-003-GUARD-DE-DE-DIFF-003 | GUARD | DE-DIFF | src-tauri/src/de/expiry.rs    | 62 |
| BR-DE-DATA-003-GUARD-DE-DE-DIFF-006 | GUARD | DE-DIFF | src-tauri/src/de/lifecycle.rs | 86 |

#### BR-DE-STATE-001

| codes                                                                                                                                                         | type   | chain                      | 文件                           | 行号   |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | -------------------------- | ---------------------------- | ---- |
| BR-CORE-STATE-001-EFFECT-ED-ED-EDIT-001, BR-CORE-STATE-003-EFFECT-ED-ED-EDIT-001, BR-DE-DATA-001-EFFECT-ED-ED-EDIT-001, BR-DE-STATE-001-EFFECT-ED-ED-EDIT-001 | EFFECT | DE-DIFF, ED-EDIT, ED-DECOR | src/components/ed/Editor.tsx | 1725 |
| BR-DE-STATE-001-RB-DE-DE-DIFF-001                                                                                                                             | RB     | DE-DIFF                    | src-tauri/src/de/mod.rs      | 71   |

#### BR-DE-STATE-001 term_ref: TERM-DE-001

| codes                                                               | type | chain            | 文件                   | 行号  |
| ------------------------------------------------------------------- | ---- | ---------------- | -------------------- | --- |
| BR-DE-DATA-001-RB-DE-DE-DIFF-007, BR-DE-STATE-001-RB-DE-DE-DIFF-007 | RB   | DE-DIFF, ED-EDIT | src/store/deStore.ts | 222 |
| BR-DE-STATE-001-RB-DE-DE-DIFF-002                                   | RB   | DE-DIFF          | src/store/deStore.ts | 320 |

#### BR-DE-STATE-001 term_ref: TERM-DE-003

| codes                                                                                                 | type | chain   | 文件                             | 行号  |
| ----------------------------------------------------------------------------------------------------- | ---- | ------- | ------------------------------ | --- |
| BR-DE-DATA-002-RB-DE-DE-DIFF-003, BR-DE-DATA-001-RB-DE-DE-DIFF-003, BR-DE-STATE-001-RB-DE-DE-DIFF-003 | RB   | DE-DIFF | src/components/de/DiffCard.tsx | 241 |

#### BR-DE-STATE-002

| codes                                 | type   | chain   | 文件                            | 行号  |
| ------------------------------------- | ------ | ------- | ----------------------------- | --- |
| BR-DE-STATE-002-GUARD-DE-DE-DIFF-001  | GUARD  | DE-DIFF | src-tauri/src/de/expiry.rs    | 34  |
| BR-DE-STATE-002-QUERY-DE-DE-DIFF-001  | QUERY  | DE-DIFF | src-tauri/src/de/lifecycle.rs | 31  |
| BR-DE-STATE-002-GUARD-DE-DE-DIFF-002  | GUARD  | DE-DIFF | src-tauri/src/de/lifecycle.rs | 58  |
| BR-DE-STATE-002-EFFECT-DE-DE-DIFF-001 | EFFECT | DE-DIFF | src-tauri/src/de/lifecycle.rs | 108 |

#### BR-DOC-DATA-001

| codes                                     | type  | chain       | 文件                                       | 行号  |
| ----------------------------------------- | ----- | ----------- | ---------------------------------------- | --- |
| BR-DOC-DATA-001-GUARD-DOC-DOC-CONVERT-001 | GUARD | DOC-CONVERT | src-tauri/src/doc/convert.rs             | 89  |
| BR-DOC-DATA-001-DATA-DOC-DOC-CONVERT-006  | DATA  | DOC-CONVERT | src-tauri/src/doc/font_trace.rs          | 243 |
| BR-DOC-DATA-001-DATA-DOC-DOC-CONVERT-007  | DATA  | DOC-CONVERT | src-tauri/src/doc/font_trace.rs          | 318 |
| BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-003    | IO    | DOC-CONVERT | src-tauri/src/doc/libreoffice_runtime.rs | 77  |
| BR-DOC-DATA-001-DATA-DOC-DOC-CONVERT-004  | DATA  | DOC-CONVERT | src-tauri/src/doc/libreoffice_runtime.rs | 106 |
| BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-005    | IO    | DOC-CONVERT | src-tauri/src/doc/libreoffice_runtime.rs | 244 |
| BR-DOC-DATA-001-DATA-DOC-DOC-CONVERT-012  | DATA  | DOC-CONVERT | src-tauri/src/doc/pandoc_runtime.rs      | 32  |
| BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-001    | IO    | DOC-CONVERT | src-tauri/src/doc/pandoc_runtime.rs      | 62  |
| BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-002    | IO    | DOC-CONVERT | src-tauri/src/doc/pandoc_runtime.rs      | 88  |
| BR-DOC-DATA-001-GUARD-DOC-DOC-CONVERT-002 | GUARD | DOC-CONVERT | src-tauri/src/doc/tool_runtime.rs        | 118 |

#### BR-DOC-DATA-002

| codes                                                                               | type  | chain       | 文件                                | 行号  |
| ----------------------------------------------------------------------------------- | ----- | ----------- | --------------------------------- | --- |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-016, BR-DOC-DATA-003-IO-DOC-DOC-CONVERT-016      | IO    | DOC-CONVERT | src-tauri/src/doc/edit_chain.rs   | 88  |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-004                                              | IO    | DOC-CONVERT | src-tauri/src/doc/edit_chain.rs   | 157 |
| BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-009, BR-DOC-DATA-003-DATA-DOC-DOC-CONVERT-009  | DATA  | DOC-CONVERT | src-tauri/src/doc/image_media.rs  | 37  |
| BR-DOC-DATA-002-GUARD-DOC-DOC-CONVERT-011, BR-WS-DATA-001-GUARD-DOC-DOC-CONVERT-011 | GUARD | DOC-CONVERT | src-tauri/src/doc/image_media.rs  | 334 |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-015, BR-WS-DATA-001-IO-DOC-DOC-CONVERT-015       | IO    | DOC-CONVERT | src-tauri/src/doc/image_media.rs  | 568 |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-013, BR-DOC-DATA-003-IO-DOC-DOC-CONVERT-013      | IO    | DOC-CONVERT | src-tauri/src/doc/image_media.rs  | 766 |
| BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-001                                            | DATA  | DOC-CONVERT | src-tauri/src/doc/style_subset.rs | 53  |
| BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-002                                            | DATA  | DOC-CONVERT | src-tauri/src/doc/style_subset.rs | 79  |

#### BR-DOC-DATA-002 term_ref: TERM-DOC-005

| codes                                    | type | chain       | 文件                                     | 行号  |
| ---------------------------------------- | ---- | ----------- | -------------------------------------- | --- |
| BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-008 | DATA | DOC-CONVERT | src-tauri/src/doc/docx_cell_shading.rs | 16  |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-006   | IO   | DOC-CONVERT | src-tauri/src/doc/docx_cell_shading.rs | 427 |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-007   | IO   | DOC-CONVERT | src-tauri/src/doc/docx_cell_shading.rs | 487 |

#### BR-DOC-DATA-002 term_ref: TERM-ED-008

| codes                                     | type  | chain       | 文件                               | 行号   |
| ----------------------------------------- | ----- | ----------- | -------------------------------- | ---- |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-012    | IO    | DOC-CONVERT | src-tauri/src/doc/image_media.rs | 670  |
| BR-DOC-DATA-002-GUARD-DOC-DOC-CONVERT-014 | GUARD | DOC-CONVERT | src-tauri/src/doc/image_media.rs | 1083 |

#### BR-DOC-DATA-003

| codes                                                                              | type  | chain       | 文件                                  | 行号  |
| ---------------------------------------------------------------------------------- | ----- | ----------- | ----------------------------------- | --- |
| BR-DOC-DATA-003-DATA-DOC-DOC-CONVERT-011                                           | DATA  | DOC-CONVERT | src-tauri/src/doc/block_backfill.rs | 26  |
| BR-DOC-DATA-003-DATA-DOC-DOC-CONVERT-003                                           | DATA  | DOC-CONVERT | src-tauri/src/doc/block_backfill.rs | 212 |
| BR-DOC-DATA-003-GUARD-DOC-DOC-CONVERT-015                                          | GUARD | DOC-CONVERT | src-tauri/src/doc/block_backfill.rs | 259 |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-016, BR-DOC-DATA-003-IO-DOC-DOC-CONVERT-016     | IO    | DOC-CONVERT | src-tauri/src/doc/edit_chain.rs     | 88  |
| BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-009, BR-DOC-DATA-003-DATA-DOC-DOC-CONVERT-009 | DATA  | DOC-CONVERT | src-tauri/src/doc/image_media.rs    | 37  |

#### BR-DOC-DATA-003 term_ref: TERM-DOC-003

| codes                                                                     | type  | chain   | 文件                         | 行号  |
| ------------------------------------------------------------------------- | ----- | ------- | -------------------------- | --- |
| BR-AG-DATA-001-QUERY-AG-AG-TURN-010, BR-DOC-DATA-003-QUERY-AG-AG-TURN-010 | QUERY | AG-TURN | src-tauri/src/ag/blocks.rs | 533 |

#### BR-DOC-DATA-003 term_ref: TERM-ED-008

| codes                                                                          | type | chain       | 文件                               | 行号  |
| ------------------------------------------------------------------------------ | ---- | ----------- | -------------------------------- | --- |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-013, BR-DOC-DATA-003-IO-DOC-DOC-CONVERT-013 | IO   | DOC-CONVERT | src-tauri/src/doc/image_media.rs | 766 |

#### BR-DOC-STATE-001

| codes                                   | type | chain       | 文件                           | 行号 |
| --------------------------------------- | ---- | ----------- | ---------------------------- | -- |
| BR-DOC-STATE-001-RB-DOC-DOC-CONVERT-001 | RB   | DOC-CONVERT | src-tauri/src/doc/convert.rs | 56 |

#### BR-ED-DATA-001

| codes                                                                                                             | type   | chain   | 文件                                      | 行号  |
| ----------------------------------------------------------------------------------------------------------------- | ------ | ------- | --------------------------------------- | --- |
| BR-ED-DATA-001-RB-ED-ED-EDIT-024, BR-ED-DATA-003-RB-ED-ED-EDIT-024                                                | RB     | ED-EDIT | src/components/ed/ImageMoveControls.tsx | 39  |
| BR-ED-DATA-001-QUERY-ED-ED-EDIT-024, BR-ED-DATA-003-QUERY-ED-ED-EDIT-024                                          | QUERY  | ED-EDIT | src/services/edImageOps.ts              | 228 |
| BR-ED-DATA-001-QUERY-ED-ED-EDIT-021, BR-ED-DATA-003-QUERY-ED-ED-EDIT-021                                          | QUERY  | ED-EDIT | src/services/edImageOps.ts              | 570 |
| BR-ED-DATA-001-EFFECT-ED-ED-EDIT-022, BR-ED-DATA-003-EFFECT-ED-ED-EDIT-022, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-022 | EFFECT | ED-EDIT | src/services/edImageOps.ts              | 599 |
| BR-ED-DATA-001-EFFECT-ED-ED-EDIT-023, BR-ED-DATA-003-EFFECT-ED-ED-EDIT-023, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-023 | EFFECT | ED-EDIT | src/services/edImageOps.ts              | 640 |
| BR-ED-DATA-001-EFFECT-ED-ED-EDIT-025, BR-ED-DATA-003-EFFECT-ED-ED-EDIT-025, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-025 | EFFECT | ED-EDIT | src/services/edImageOps.ts              | 731 |
| BR-AG-DATA-003-RB-AG-AG-TOOL-016, BR-ED-DATA-001-RB-AG-AG-TOOL-016, BR-DE-DATA-001-RB-AG-AG-TOOL-016              | RB     | AG-TOOL | src-tauri/src/ag/block_ops.rs           | 305 |
| BR-SYS-DATA-006-GUARD-AG-AG-TOOL-004, BR-ED-DATA-001-GUARD-AG-AG-TOOL-004, BR-ED-DATA-003-GUARD-AG-AG-TOOL-004    | GUARD  | AG-TOOL | src-tauri/src/ag/edit_image_runtime.rs  | 8   |

#### BR-ED-DATA-001 term_ref: TERM-ED-001

| codes                              | type | chain   | 文件                                               | 行号 |
| ---------------------------------- | ---- | ------- | ------------------------------------------------ | -- |
| BR-ED-DATA-001-DATA-ED-ED-EDIT-001 | DATA | ED-EDIT | src/components/ed/extensions/BlockIdExtension.ts | 96 |

#### BR-ED-DATA-002

| codes                                                              | type | chain   | 文件                        | 行号  |
| ------------------------------------------------------------------ | ---- | ------- | ------------------------- | --- |
| BR-ED-DATA-002-RB-ED-ED-EDIT-012, BR-ED-DATA-006-RB-ED-ED-EDIT-012 | RB   | ED-EDIT | src/services/edService.ts | 270 |

#### BR-ED-DATA-003

| codes                                                                                                             | type   | chain   | 文件                                             | 行号  |
| ----------------------------------------------------------------------------------------------------------------- | ------ | ------- | ---------------------------------------------- | --- |
| BR-ED-DATA-001-RB-ED-ED-EDIT-024, BR-ED-DATA-003-RB-ED-ED-EDIT-024                                                | RB     | ED-EDIT | src/components/ed/ImageMoveControls.tsx        | 39  |
| BR-ED-DATA-003-GUARD-ED-ED-EDIT-019                                                                               | GUARD  | ED-EDIT | src/components/ed/extensions/Pagination.ts     | 263 |
| BR-ED-DATA-003-GUARD-ED-ED-EDIT-020                                                                               | GUARD  | ED-EDIT | src/components/ed/extensions/Pagination.ts     | 585 |
| BR-ED-DATA-003-GUARD-ED-ED-EDIT-018                                                                               | GUARD  | ED-EDIT | src/components/ed/extensions/Pagination.ts     | 636 |
| BR-ED-DATA-003-RB-ED-ED-EDIT-005                                                                                  | RB     | ED-EDIT | src/components/ed/extensions/Pagination.ts     | 893 |
| BR-ED-DATA-003-EFFECT-ED-ED-EDIT-020, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-020                                       | EFFECT | ED-EDIT | src/components/ed/extensions/ResizableImage.ts | 50  |
| BR-ED-DATA-003-RB-ED-ED-EDIT-017                                                                                  | RB     | ED-EDIT | src/components/ed/extensions/paginationFlow.ts | 57  |
| BR-ED-DATA-003-RB-ED-ED-EDIT-006                                                                                  | RB     | ED-EDIT | src/services/docService.ts                     | 107 |
| BR-ED-DATA-001-QUERY-ED-ED-EDIT-021, BR-ED-DATA-003-QUERY-ED-ED-EDIT-021                                          | QUERY  | ED-EDIT | src/services/edImageOps.ts                     | 570 |
| BR-ED-DATA-001-EFFECT-ED-ED-EDIT-022, BR-ED-DATA-003-EFFECT-ED-ED-EDIT-022, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-022 | EFFECT | ED-EDIT | src/services/edImageOps.ts                     | 599 |
| BR-ED-DATA-001-EFFECT-ED-ED-EDIT-023, BR-ED-DATA-003-EFFECT-ED-ED-EDIT-023, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-023 | EFFECT | ED-EDIT | src/services/edImageOps.ts                     | 640 |
| BR-ED-DATA-001-EFFECT-ED-ED-EDIT-025, BR-ED-DATA-003-EFFECT-ED-ED-EDIT-025, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-025 | EFFECT | ED-EDIT | src/services/edImageOps.ts                     | 731 |

#### BR-ED-DATA-003 term_ref: TERM-ED-008

| codes                                                                    | type   | chain   | 文件                         | 行号  |
| ------------------------------------------------------------------------ | ------ | ------- | -------------------------- | --- |
| BR-ED-DATA-003-RB-ED-ED-EDIT-026                                         | RB     | ED-EDIT | src/services/docService.ts | 184 |
| BR-ED-DATA-003-EFFECT-ED-ED-EDIT-026                                     | EFFECT | ED-EDIT | src/services/docService.ts | 203 |
| BR-ED-DATA-001-QUERY-ED-ED-EDIT-024, BR-ED-DATA-003-QUERY-ED-ED-EDIT-024 | QUERY  | ED-EDIT | src/services/edImageOps.ts | 228 |
| BR-AG-DATA-001-QUERY-AG-AG-TOOL-011, BR-ED-DATA-003-QUERY-AG-AG-TOOL-011 | QUERY  | AG-TOOL | src-tauri/src/ag/blocks.rs | 307 |

#### BR-ED-DATA-003 term_ref: TERM-SYS-002

| codes                                                                                                          | type  | chain   | 文件                                     | 行号   |
| -------------------------------------------------------------------------------------------------------------- | ----- | ------- | -------------------------------------- | ---- |
| BR-SYS-DATA-006-RB-ED-ED-EDIT-027, BR-ED-DATA-003-RB-ED-ED-EDIT-027                                            | RB    | ED-EDIT | src/services/edImageOps.ts             | 256  |
| BR-SYS-DATA-006-GUARD-AG-AG-TOOL-022, BR-ED-DATA-003-GUARD-AG-AG-TOOL-022                                      | GUARD | AG-TOOL | src-tauri/src/ag/bdocx_capabilities.rs | 1918 |
| BR-SYS-DATA-006-GUARD-AG-AG-TOOL-004, BR-ED-DATA-001-GUARD-AG-AG-TOOL-004, BR-ED-DATA-003-GUARD-AG-AG-TOOL-004 | GUARD | AG-TOOL | src-tauri/src/ag/edit_image_runtime.rs | 8    |

#### BR-ED-DATA-004 term_ref: TERM-ED-004

| codes                               | type  | chain   | 文件                       | 行号  |
| ----------------------------------- | ----- | ------- | ------------------------ | --- |
| BR-ED-DATA-004-QUERY-ED-ED-EDIT-010 | QUERY | ED-EDIT | src/services/edLocate.ts | 231 |
| BR-ED-DATA-004-QUERY-ED-ED-EDIT-007 | QUERY | ED-EDIT | src/services/edLocate.ts | 414 |

#### BR-ED-DATA-005

| codes                                                                      | type  | chain   | 文件                                          | 行号  |
| -------------------------------------------------------------------------- | ----- | ------- | ------------------------------------------- | --- |
| BR-ED-VERIFY-001-GUARD-ED-ED-EDIT-015, BR-ED-DATA-005-GUARD-ED-ED-EDIT-015 | GUARD | ED-EDIT | src/components/ed/markdown/markdownCodec.ts | 170 |

#### BR-ED-DATA-005 term_ref: TERM-ED-006

| codes                               | type  | chain   | 文件                         | 行号   |
| ----------------------------------- | ----- | ------- | -------------------------- | ---- |
| BR-ED-DATA-005-RB-ED-ED-EDIT-008    | RB    | ED-EDIT | src/services/edStyleOps.ts | 243  |
| BR-ED-DATA-005-QUERY-ED-ED-EDIT-009 | QUERY | ED-EDIT | src-tauri/src/ag/blocks.rs | 1010 |

#### BR-ED-DATA-005 term_ref: TERM-SYS-002

| codes                                                                     | type  | chain   | 文件                                     | 行号   |
| ------------------------------------------------------------------------- | ----- | ------- | -------------------------------------- | ---- |
| BR-SYS-DATA-006-GUARD-AG-AG-TOOL-021, BR-ED-DATA-005-GUARD-AG-AG-TOOL-021 | GUARD | AG-TOOL | src-tauri/src/ag/bdocx_capabilities.rs | 1566 |

#### BR-ED-DATA-006

| codes                                                                      | type  | chain   | 文件                                          | 行号  |
| -------------------------------------------------------------------------- | ----- | ------- | ------------------------------------------- | --- |
| BR-ED-DATA-006-DATA-ED-ED-EDIT-014                                         | DATA  | ED-EDIT | src/components/ed/markdown/markdownCodec.ts | 82  |
| BR-ED-DATA-002-RB-ED-ED-EDIT-012, BR-ED-DATA-006-RB-ED-ED-EDIT-012         | RB    | ED-EDIT | src/services/edService.ts                   | 270 |
| BR-ED-VERIFY-001-GUARD-ED-ED-EDIT-016, BR-ED-DATA-006-GUARD-ED-ED-EDIT-016 | GUARD | ED-EDIT | src/services/edService.ts                   | 361 |

#### BR-ED-DATA-007 term_ref: TERM-ED-007

| codes                                | type   | chain                     | 文件                                               | 行号  |
| ------------------------------------ | ------ | ------------------------- | ------------------------------------------------ | --- |
| BR-ED-DATA-007-RB-ED-ED-EDIT-019     | RB     | ED-EDIT                   | src/components/ed/TableMoveControls.tsx          | 47  |
| BR-ED-DATA-007-DATA-ED-ED-EDIT-018   | DATA   | ED-EDIT                   | src/components/ed/extensions/TableCellShading.ts | 37  |
| BR-ED-DATA-007-RB-ED-ED-EDIT-018     | RB     | ED-EDIT, DE-DIFF          | src/services/edTableOps.ts                       | 407 |
| BR-ED-DATA-007-EFFECT-ED-ED-EDIT-014 | EFFECT | ED-EDIT, DE-DIFF          | src/services/edTableOps.ts                       | 438 |
| BR-ED-DATA-007-GUARD-AG-AG-TOOL-018  | GUARD  | AG-TOOL, DE-DIFF, ED-EDIT | src-tauri/src/ag/edit_table_runtime.rs           | 8   |

#### BR-ED-DATA-008

| codes                                                                                                                                                                                                                                                   | type  | chain            | 文件                               | 行号  |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----- | ---------------- | -------------------------------- | --- |
| BR-AG-DATA-008-DATA-AG-AG-TURN-041, BR-ED-DATA-008-DATA-AG-AG-TURN-041, BR-SYS-DATA-008-DATA-AG-AG-TURN-041                                                                                                                                             | DATA  | AG-TURN          | src/services/agService.ts        | 182 |
| BR-AG-DATA-008-GUARD-AG-AG-TURN-042, BR-ED-DATA-008-GUARD-AG-AG-TURN-042, BR-SYS-DATA-008-GUARD-AG-AG-TURN-042                                                                                                                                          | GUARD | AG-TURN          | src-tauri/src/ag/turn_runtime.rs | 391 |
| BR-AG-OBS-001-API-AG-AG-TURN-018, BR-AG-STATE-001-API-AG-AG-TURN-018, BR-AG-STATE-004-API-AG-AG-TURN-018, BR-AG-DATA-008-API-AG-AG-TURN-018, BR-ED-DATA-008-API-AG-AG-TURN-018, BR-SYS-STATE-007-API-AG-AG-TURN-018, BR-SYS-DATA-008-API-AG-AG-TURN-018 | API   | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_runtime.rs | 753 |

#### BR-ED-STATE-001

| codes                                                                                                             | type   | chain   | 文件                         | 行号  |
| ----------------------------------------------------------------------------------------------------------------- | ------ | ------- | -------------------------- | --- |
| BR-ED-DATA-001-EFFECT-ED-ED-EDIT-022, BR-ED-DATA-003-EFFECT-ED-ED-EDIT-022, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-022 | EFFECT | ED-EDIT | src/services/edImageOps.ts | 599 |
| BR-ED-DATA-001-EFFECT-ED-ED-EDIT-023, BR-ED-DATA-003-EFFECT-ED-ED-EDIT-023, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-023 | EFFECT | ED-EDIT | src/services/edImageOps.ts | 640 |
| BR-ED-STATE-001-RB-ED-ED-EDIT-002                                                                                 | RB     | ED-EDIT | src/store/edStore.ts       | 262 |

#### BR-ED-STATE-001 term_ref: TERM-ED-001

| codes                                 | type   | chain   | 文件                       | 行号  |
| ------------------------------------- | ------ | ------- | ------------------------ | --- |
| BR-ED-STATE-001-EFFECT-ED-ED-EDIT-012 | EFFECT | ED-EDIT | src/services/edLocate.ts | 345 |
| BR-ED-STATE-001-EFFECT-ED-ED-EDIT-011 | EFFECT | ED-EDIT | src/services/edLocate.ts | 384 |

#### BR-ED-STATE-001 term_ref: TERM-ED-008

| codes                                                                                                             | type   | chain   | 文件                                             | 行号  |
| ----------------------------------------------------------------------------------------------------------------- | ------ | ------- | ---------------------------------------------- | --- |
| BR-ED-DATA-003-EFFECT-ED-ED-EDIT-020, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-020                                       | EFFECT | ED-EDIT | src/components/ed/extensions/ResizableImage.ts | 50  |
| BR-ED-DATA-001-EFFECT-ED-ED-EDIT-025, BR-ED-DATA-003-EFFECT-ED-ED-EDIT-025, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-025 | EFFECT | ED-EDIT | src/services/edImageOps.ts                     | 731 |

#### BR-ED-STATE-002 term_ref: TERM-ED-002

| codes                              | type | chain    | 文件                                                      | 行号  |
| ---------------------------------- | ---- | -------- | ------------------------------------------------------- | --- |
| BR-ED-STATE-002-RB-ED-ED-DECOR-001 | RB   | ED-DECOR | src/components/ed/extensions/DiffDecorationExtension.ts | 155 |

#### BR-ED-STATE-003

| codes                                 | type   | chain   | 文件                        | 行号  |
| ------------------------------------- | ------ | ------- | ------------------------- | --- |
| BR-ED-STATE-003-EFFECT-ED-ED-EDIT-013 | EFFECT | ED-EDIT | src/services/edService.ts | 327 |

#### BR-ED-VERIFY-001

| codes                                                                      | type  | chain   | 文件                                          | 行号  |
| -------------------------------------------------------------------------- | ----- | ------- | ------------------------------------------- | --- |
| BR-ED-VERIFY-001-GUARD-ED-ED-EDIT-015, BR-ED-DATA-005-GUARD-ED-ED-EDIT-015 | GUARD | ED-EDIT | src/components/ed/markdown/markdownCodec.ts | 170 |
| BR-ED-VERIFY-001-GUARD-ED-ED-EDIT-016, BR-ED-DATA-006-GUARD-ED-ED-EDIT-016 | GUARD | ED-EDIT | src/services/edService.ts                   | 361 |

#### BR-ENT-DATA-001

| codes                                       | type   | chain        | 文件                        | 行号 |
| ------------------------------------------- | ------ | ------------ | ------------------------- | -- |
| BR-ENT-DATA-001-EFFECT-ENT-CLOUD-INVOKE-001 | EFFECT | CLOUD-INVOKE | src-tauri/src/ent/byok.rs | 17 |

#### BR-ENT-DATA-002

| codes                                    | type  | chain      | 文件                         | 行号 |
| ---------------------------------------- | ----- | ---------- | -------------------------- | -- |
| BR-ENT-DATA-002-GUARD-ENT-CLOUD-GATE-001 | GUARD | CLOUD-GATE | src-tauri/src/ent/usage.rs | 34 |

#### BR-ENT-STATE-001

| codes                                    | type | chain        | 文件                           | 行号   |
| ---------------------------------------- | ---- | ------------ | ---------------------------- | ---- |
| BR-ENT-STATE-001-RB-ENT-CLOUD-INVOKE-001 | RB   | CLOUD-INVOKE | src-tauri/src/ent/gateway.rs | 103  |
| BR-ENT-STATE-001-RB-ENT-CLOUD-INVOKE-002 | RB   | CLOUD-INVOKE | src-tauri/src/ent/gateway.rs | 1324 |

#### BR-ENT-VERIFY-001

| codes                                      | type  | chain      | 文件                           | 行号  |
| ------------------------------------------ | ----- | ---------- | ---------------------------- | --- |
| BR-ENT-VERIFY-001-GUARD-ENT-CLOUD-GATE-002 | GUARD | CLOUD-GATE | src-tauri/src/ent/gateway.rs | 133 |

#### BR-KB-DATA-001

| codes                                    | type   | chain       | 文件                        | 行号  |
| ---------------------------------------- | ------ | ----------- | ------------------------- | --- |
| BR-KB-DATA-001-EFFECT-KB-KB-RETRIEVE-001 | EFFECT | KB-RETRIEVE | src-tauri/src/kb/store.rs | 31  |
| BR-KB-DATA-001-QUERY-KB-KB-RETRIEVE-002  | QUERY  | KB-RETRIEVE | src-tauri/src/kb/store.rs | 84  |
| BR-KB-DATA-001-EFFECT-KB-KB-RETRIEVE-002 | EFFECT | KB-RETRIEVE | src-tauri/src/kb/store.rs | 106 |

#### BR-KB-DATA-002

| codes                                   | type  | chain       | 文件                           | 行号 |
| --------------------------------------- | ----- | ----------- | ---------------------------- | -- |
| BR-KB-DATA-002-QUERY-KB-KB-RETRIEVE-001 | QUERY | KB-RETRIEVE | src-tauri/src/kb/retrieve.rs | 12 |

#### BR-PCG-DATA-001

| codes                                                                             | type | chain        | 文件                                            | 行号 |
| --------------------------------------------------------------------------------- | ---- | ------------ | --------------------------------------------- | -- |
| BR-PCG-DATA-001-RB-PCG-PCG-GENERATE-001, BR-PCG-STATE-001-RB-PCG-PCG-GENERATE-001 | RB   | PCG-GENERATE | scripts/generate_function_ledger.mjs          | 14 |
| BR-PCG-DATA-001-RB-PCG-PCG-GENERATE-004                                           | RB   | PCG-GENERATE | scripts/generate_function_tree.mjs            | 22 |
| BR-PCG-DATA-001-RB-PCG-PCG-GENERATE-001, BR-PCG-STATE-001-RB-PCG-PCG-GENERATE-001 | RB   | PCG-GENERATE | scripts/generate_product_capability_atlas.mjs | 11 |
| BR-PCG-DATA-001-RB-PCG-PCG-GENERATE-003                                           | RB   | PCG-GENERATE | scripts/seed_function_audit.mjs               | 28 |

#### BR-PCG-STATE-001

| codes                                                                             | type | chain        | 文件                                            | 行号 |
| --------------------------------------------------------------------------------- | ---- | ------------ | --------------------------------------------- | -- |
| BR-PCG-DATA-001-RB-PCG-PCG-GENERATE-001, BR-PCG-STATE-001-RB-PCG-PCG-GENERATE-001 | RB   | PCG-GENERATE | scripts/generate_function_ledger.mjs          | 14 |
| BR-PCG-DATA-001-RB-PCG-PCG-GENERATE-004                                           | RB   | PCG-GENERATE | scripts/generate_function_tree.mjs            | 22 |
| BR-PCG-DATA-001-RB-PCG-PCG-GENERATE-001, BR-PCG-STATE-001-RB-PCG-PCG-GENERATE-001 | RB   | PCG-GENERATE | scripts/generate_product_capability_atlas.mjs | 11 |
| BR-PCG-DATA-001-RB-PCG-PCG-GENERATE-003                                           | RB   | PCG-GENERATE | scripts/seed_function_audit.mjs               | 28 |

#### BR-PCG-VERIFY-001

| codes                                                                          | type | chain     | 文件                                   | 行号 |
| ------------------------------------------------------------------------------ | ---- | --------- | ------------------------------------ | -- |
| BR-PCG-VERIFY-001-RB-PCG-PCG-AUDIT-001, BR-PCG-VERIFY-002-RB-PCG-PCG-AUDIT-001 | RB   | PCG-AUDIT | scripts/audit_product_capability.mjs | 5  |

#### BR-PCG-VERIFY-002

| codes                                                                          | type | chain     | 文件                                   | 行号 |
| ------------------------------------------------------------------------------ | ---- | --------- | ------------------------------------ | -- |
| BR-PCG-VERIFY-001-RB-PCG-PCG-AUDIT-001, BR-PCG-VERIFY-002-RB-PCG-PCG-AUDIT-001 | RB   | PCG-AUDIT | scripts/audit_product_capability.mjs | 5  |

#### BR-PRV-DATA-001

| codes                                                                    | type  | chain    | 文件                                | 行号 |
| ------------------------------------------------------------------------ | ----- | -------- | --------------------------------- | -- |
| BR-PRV-DATA-001-GUARD-PRV-PRV-VIEW-003                                   | GUARD | PRV-VIEW | src/components/prv/PrvPanel.tsx   | 41 |
| BR-PRV-DATA-001-RB-PRV-PRV-VIEW-004, BR-PRV-DATA-002-RB-PRV-PRV-VIEW-004 | RB    | PRV-VIEW | src/components/prv/PrvToolbar.tsx | 23 |

#### BR-PRV-DATA-002

| codes                                                                    | type | chain    | 文件                                 | 行号  |
| ------------------------------------------------------------------------ | ---- | -------- | ---------------------------------- | --- |
| BR-PRV-DATA-001-RB-PRV-PRV-VIEW-004, BR-PRV-DATA-002-RB-PRV-PRV-VIEW-004 | RB   | PRV-VIEW | src/components/prv/PrvToolbar.tsx  | 23  |
| BR-PRV-DATA-002-IO-PRV-PRV-VIEW-004                                      | IO   | PRV-VIEW | src-tauri/src/doc/table_preview.rs | 48  |
| BR-PRV-DATA-002-IO-PRV-PRV-VIEW-001                                      | IO   | PRV-VIEW | src-tauri/src/prv/mod.rs           | 84  |
| BR-PRV-DATA-002-IO-PRV-PRV-VIEW-002                                      | IO   | PRV-VIEW | src-tauri/src/prv/mod.rs           | 124 |
| BR-PRV-DATA-002-IO-PRV-PRV-VIEW-003                                      | IO   | PRV-VIEW | src-tauri/src/prv/mod.rs           | 182 |

#### BR-REF-DATA-001

| codes                                    | type  | chain      | 文件                                 | 行号 |
| ---------------------------------------- | ----- | ---------- | ---------------------------------- | -- |
| BR-REF-DATA-001-GUARD-REF-REF-INJECT-003 | GUARD | REF-INJECT | src-tauri/src/reference/resolve.rs | 34 |

#### BR-REF-DATA-001 term_ref: TERM-REF-001

| codes                                  | type | chain      | 文件                                | 行号  |
| -------------------------------------- | ---- | ---------- | --------------------------------- | --- |
| BR-REF-DATA-001-API-REF-REF-INJECT-001 | API  | REF-INJECT | src-tauri/src/reference/routes.rs | 75  |
| BR-REF-DATA-001-API-REF-REF-INJECT-002 | API  | REF-INJECT | src-tauri/src/reference/routes.rs | 112 |

#### BR-REF-DATA-002

| codes                                                                            | type | chain      | 文件                                | 行号  |
| -------------------------------------------------------------------------------- | ---- | ---------- | --------------------------------- | --- |
| BR-REF-DATA-002-DATA-REF-REF-INJECT-001                                          | DATA | REF-INJECT | src-tauri/src/reference/inject.rs | 54  |
| BR-REF-DATA-002-API-REF-REF-INJECT-004, BR-REF-VERIFY-001-API-REF-REF-INJECT-004 | API  | REF-INJECT | src-tauri/src/reference/routes.rs | 187 |

#### BR-REF-DATA-003

| codes                                                                                                                       | type | chain      | 文件                                 | 行号  |
| --------------------------------------------------------------------------------------------------------------------------- | ---- | ---------- | ---------------------------------- | --- |
| BR-REF-DATA-003-DATA-REF-REF-INJECT-006, BR-SYS-DATA-002-DATA-REF-REF-INJECT-006                                            | DATA | REF-INJECT | src/services/referencePayload.ts   | 2   |
| BR-REF-DATA-003-DATA-REF-REF-INJECT-002, BR-REF-VERIFY-001-DATA-REF-REF-INJECT-002, BR-SYS-DATA-007-DATA-REF-REF-INJECT-002 | DATA | REF-INJECT | src-tauri/src/reference/context.rs | 102 |

#### BR-REF-DATA-003 term_ref: TERM-REF-003

| codes                                    | type  | chain      | 文件                                 | 行号  |
| ---------------------------------------- | ----- | ---------- | ---------------------------------- | --- |
| BR-REF-DATA-003-DATA-REF-REF-INJECT-005  | DATA  | REF-INJECT | src/services/referenceDrag.ts      | 2   |
| BR-REF-DATA-003-GUARD-REF-REF-INJECT-004 | GUARD | REF-INJECT | src/services/referenceDrag.ts      | 49  |
| BR-REF-DATA-003-GUARD-REF-REF-INJECT-005 | GUARD | REF-INJECT | src/services/referenceDrag.ts      | 71  |
| BR-REF-DATA-003-GUARD-REF-REF-INJECT-006 | GUARD | REF-INJECT | src/services/referencePayload.ts   | 122 |
| BR-REF-DATA-003-GUARD-REF-REF-INJECT-007 | GUARD | REF-INJECT | src/services/referencePayload.ts   | 183 |
| BR-REF-DATA-003-DATA-REF-REF-INJECT-003  | DATA  | REF-INJECT | src-tauri/src/reference/context.rs | 36  |
| BR-REF-DATA-003-DATA-REF-REF-INJECT-004  | DATA  | REF-INJECT | src-tauri/src/reference/context.rs | 242 |

#### BR-REF-DATA-004 term_ref: TERM-REF-004

| codes                                  | type | chain      | 文件                               | 行号  |
| -------------------------------------- | ---- | ---------- | -------------------------------- | --- |
| BR-REF-DATA-004-DATA-AG-REF-INJECT-033 | DATA | REF-INJECT | src/components/ag/Composer.tsx   | 40  |
| BR-REF-DATA-004-RB-REF-REF-INJECT-001  | RB   | REF-INJECT | src/services/referencePayload.ts | 439 |

#### BR-REF-VERIFY-001

| codes                                                                                                                       | type  | chain      | 文件                                 | 行号  |
| --------------------------------------------------------------------------------------------------------------------------- | ----- | ---------- | ---------------------------------- | --- |
| BR-REF-DATA-003-DATA-REF-REF-INJECT-002, BR-REF-VERIFY-001-DATA-REF-REF-INJECT-002, BR-SYS-DATA-007-DATA-REF-REF-INJECT-002 | DATA  | REF-INJECT | src-tauri/src/reference/context.rs | 102 |
| BR-REF-VERIFY-001-GUARD-REF-REF-INJECT-001                                                                                  | GUARD | REF-INJECT | src-tauri/src/reference/inject.rs  | 14  |

#### BR-REF-VERIFY-001 term_ref: TERM-REF-001

| codes                                    | type | chain      | 文件                                | 行号  |
| ---------------------------------------- | ---- | ---------- | --------------------------------- | --- |
| BR-REF-VERIFY-001-API-REF-REF-INJECT-003 | API  | REF-INJECT | src-tauri/src/reference/routes.rs | 159 |

#### BR-REF-VERIFY-001 term_ref: TERM-REF-002

| codes                                                                            | type | chain      | 文件                                | 行号  |
| -------------------------------------------------------------------------------- | ---- | ---------- | --------------------------------- | --- |
| BR-REF-DATA-002-API-REF-REF-INJECT-004, BR-REF-VERIFY-001-API-REF-REF-INJECT-004 | API  | REF-INJECT | src-tauri/src/reference/routes.rs | 187 |

#### BR-REF-VERIFY-001 term_ref: TERM-REF-003

| codes                                      | type  | chain      | 文件                                 | 行号 |
| ------------------------------------------ | ----- | ---------- | ---------------------------------- | -- |
| BR-REF-VERIFY-001-GUARD-REF-REF-INJECT-002 | GUARD | REF-INJECT | src-tauri/src/reference/context.rs | 54 |

#### BR-SYS-DATA-001

| codes                                                                                | type  | chain                    | 文件                           | 行号 |
| ------------------------------------------------------------------------------------ | ----- | ------------------------ | ---------------------------- | -- |
| BR-SYS-VERIFY-001-GUARD-SYS-CLOUD-GATE-001, BR-SYS-DATA-001-GUARD-SYS-CLOUD-GATE-001 | GUARD | CLOUD-GATE, CLOUD-INVOKE | src-tauri/src/session/mod.rs | 76 |

#### BR-SYS-DATA-002

| codes                                                                                | type | chain        | 文件                         | 行号 |
| ------------------------------------------------------------------------------------ | ---- | ------------ | -------------------------- | -- |
| BR-SYS-DATA-002-DATA-SYS-DATA-MIGRATE-001, BR-SYS-DATA-008-DATA-SYS-DATA-MIGRATE-001 | DATA | DATA-MIGRATE | src-tauri/src/types/mod.rs | 9  |

#### BR-SYS-DATA-002 term_ref: TERM-REF-003

| codes                                                                            | type | chain      | 文件                               | 行号 |
| -------------------------------------------------------------------------------- | ---- | ---------- | -------------------------------- | -- |
| BR-REF-DATA-003-DATA-REF-REF-INJECT-006, BR-SYS-DATA-002-DATA-REF-REF-INJECT-006 | DATA | REF-INJECT | src/services/referencePayload.ts | 2  |

#### BR-SYS-DATA-003

| codes                                | type | chain    | 文件                       | 行号  |
| ------------------------------------ | ---- | -------- | ------------------------ | --- |
| BR-SYS-DATA-003-API-SYS-IPC-CALL-001 | API  | IPC-CALL | src-tauri/src/ipc/mod.rs | 335 |
| BR-SYS-DATA-003-API-SYS-IPC-CALL-005 | API  | IPC-CALL | src-tauri/src/lib.rs     | 3   |

#### BR-SYS-DATA-003 term_ref: TERM-WS-001

| codes                                | type | chain    | 文件                   | 行号  |
| ------------------------------------ | ---- | -------- | -------------------- | --- |
| BR-SYS-DATA-003-API-SYS-IPC-CALL-002 | API  | IPC-CALL | src-tauri/src/lib.rs | 295 |

#### BR-SYS-DATA-005

| codes                                 | type   | chain   | 文件                            | 行号  |
| ------------------------------------- | ------ | ------- | ----------------------------- | --- |
| BR-SYS-DATA-005-EFFECT-DE-DE-DIFF-003 | EFFECT | DE-DIFF | src-tauri/src/de/lifecycle.rs | 132 |
| BR-SYS-DATA-005-EFFECT-DE-DE-DIFF-004 | EFFECT | DE-DIFF | src-tauri/src/de/lifecycle.rs | 155 |

#### BR-SYS-DATA-006

| codes                                                                                                          | type  | chain   | 文件                                     | 行号   |
| -------------------------------------------------------------------------------------------------------------- | ----- | ------- | -------------------------------------- | ---- |
| BR-SYS-DATA-006-RB-ED-ED-EDIT-027, BR-ED-DATA-003-RB-ED-ED-EDIT-027                                            | RB    | ED-EDIT | src/services/edImageOps.ts             | 256  |
| BR-SYS-DATA-006-GUARD-AG-AG-TOOL-021, BR-ED-DATA-005-GUARD-AG-AG-TOOL-021                                      | GUARD | AG-TOOL | src-tauri/src/ag/bdocx_capabilities.rs | 1566 |
| BR-SYS-DATA-006-GUARD-AG-AG-TOOL-022, BR-ED-DATA-003-GUARD-AG-AG-TOOL-022                                      | GUARD | AG-TOOL | src-tauri/src/ag/bdocx_capabilities.rs | 1918 |
| BR-SYS-DATA-006-GUARD-AG-AG-TOOL-004, BR-ED-DATA-001-GUARD-AG-AG-TOOL-004, BR-ED-DATA-003-GUARD-AG-AG-TOOL-004 | GUARD | AG-TOOL | src-tauri/src/ag/edit_image_runtime.rs | 8    |

#### BR-SYS-DATA-006 term_ref: TERM-SYS-002

| codes                               | type | chain   | 文件                                     | 行号   |
| ----------------------------------- | ---- | ------- | -------------------------------------- | ---- |
| BR-SYS-DATA-006-DATA-AG-AG-TOOL-001 | DATA | AG-TOOL | src-tauri/src/ag/bdocx_capabilities.rs | 611  |
| BR-SYS-DATA-006-DATA-AG-AG-TOOL-002 | DATA | AG-TOOL | src-tauri/src/ag/bdocx_capabilities.rs | 897  |
| BR-SYS-DATA-006-DATA-AG-AG-TOOL-032 | DATA | AG-TURN | src-tauri/src/ag/bdocx_capabilities.rs | 1298 |

#### BR-SYS-DATA-007

| codes                                                                              | type  | chain                            | 文件                          | 行号 |
| ---------------------------------------------------------------------------------- | ----- | -------------------------------- | --------------------------- | -- |
| BR-SYS-DATA-007-API-AST-AST-MEMORY-001, BR-AST-DATA-001-API-AST-AST-MEMORY-001     | API   | AST-MEMORY, REF-INJECT, IPC-CALL | src-tauri/src/ast/routes.rs | 6  |
| BR-SYS-DATA-007-QUERY-AST-AST-MEMORY-003, BR-AST-DATA-001-QUERY-AST-AST-MEMORY-003 | QUERY | AST-MEMORY, REF-INJECT           | src-tauri/src/ast/store.rs  | 72 |

#### BR-SYS-DATA-007 term_ref: TERM-REF-003

| codes                                                                                                                       | type | chain      | 文件                                 | 行号  |
| --------------------------------------------------------------------------------------------------------------------------- | ---- | ---------- | ---------------------------------- | --- |
| BR-REF-DATA-003-DATA-REF-REF-INJECT-002, BR-REF-VERIFY-001-DATA-REF-REF-INJECT-002, BR-SYS-DATA-007-DATA-REF-REF-INJECT-002 | DATA | REF-INJECT | src-tauri/src/reference/context.rs | 102 |

#### BR-SYS-DATA-008 term_ref: TERM-AG-001

| codes                                                                                                                                                                                                                                                   | type | chain            | 文件                               | 行号  |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---- | ---------------- | -------------------------------- | --- |
| BR-AG-OBS-001-API-AG-AG-TURN-018, BR-AG-STATE-001-API-AG-AG-TURN-018, BR-AG-STATE-004-API-AG-AG-TURN-018, BR-AG-DATA-008-API-AG-AG-TURN-018, BR-ED-DATA-008-API-AG-AG-TURN-018, BR-SYS-STATE-007-API-AG-AG-TURN-018, BR-SYS-DATA-008-API-AG-AG-TURN-018 | API  | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_runtime.rs | 753 |

#### BR-SYS-DATA-008 term_ref: TERM-AG-002

| codes                                                                    | type | chain   | 文件                            | 行号 |
| ------------------------------------------------------------------------ | ---- | ------- | ----------------------------- | -- |
| BR-AG-STATE-002-DATA-AG-AG-TOOL-044, BR-SYS-DATA-008-DATA-AG-AG-TOOL-044 | DATA | AG-TOOL | src-tauri/src/ag/tool_exec.rs | 18 |

#### BR-SYS-DATA-008 term_ref: TERM-AG-005

| codes                                                                                                       | type | chain   | 文件                              | 行号 |
| ----------------------------------------------------------------------------------------------------------- | ---- | ------- | ------------------------------- | -- |
| BR-AG-DATA-002-DATA-AG-AG-TURN-031, BR-AG-DATA-008-DATA-AG-AG-TURN-031, BR-SYS-DATA-008-DATA-AG-AG-TURN-031 | DATA | AG-TURN | src-tauri/src/ag/turn_prompt.rs | 95 |

#### BR-SYS-DATA-008 term_ref: TERM-AG-020

| codes                                                                                                          | type  | chain            | 文件                               | 行号  |
| -------------------------------------------------------------------------------------------------------------- | ----- | ---------------- | -------------------------------- | --- |
| BR-AG-DATA-008-DATA-AG-AG-TURN-041, BR-ED-DATA-008-DATA-AG-AG-TURN-041, BR-SYS-DATA-008-DATA-AG-AG-TURN-041    | DATA  | AG-TURN          | src/services/agService.ts        | 182 |
| BR-AG-DATA-008-DATA-AG-AG-TURN-054, BR-SYS-DATA-008-DATA-AG-AG-TURN-054                                        | DATA  | AG-TURN, AG-TOOL | src/services/agService.ts        | 545 |
| BR-AG-OBS-001-DATA-AG-AG-TOOL-043, BR-AG-DATA-008-DATA-AG-AG-TOOL-043, BR-SYS-DATA-008-DATA-AG-AG-TOOL-043     | DATA  | AG-TURN, AG-TOOL | src-tauri/src/ag/tool_exec.rs    | 173 |
| BR-AG-DATA-008-GUARD-AG-AG-TURN-042, BR-ED-DATA-008-GUARD-AG-AG-TURN-042, BR-SYS-DATA-008-GUARD-AG-AG-TURN-042 | GUARD | AG-TURN          | src-tauri/src/ag/turn_runtime.rs | 391 |
| BR-AG-DATA-008-GUARD-AG-AG-TURN-052, BR-SYS-DATA-008-GUARD-AG-AG-TURN-052                                      | GUARD | AG-TURN          | src-tauri/src/ag/turn_runtime.rs | 574 |
| BR-SYS-DATA-002-DATA-SYS-DATA-MIGRATE-001, BR-SYS-DATA-008-DATA-SYS-DATA-MIGRATE-001                           | DATA  | DATA-MIGRATE     | src-tauri/src/types/mod.rs       | 9   |

#### BR-SYS-DATA-008 term_ref: TERM-AG-021

| codes                                                                                                            | type  | chain            | 文件                              | 行号  |
| ---------------------------------------------------------------------------------------------------------------- | ----- | ---------------- | ------------------------------- | --- |
| BR-AG-OBS-001-QUERY-AG-AG-TURN-048, BR-AG-PERSIST-001-QUERY-AG-AG-TURN-048, BR-SYS-DATA-008-QUERY-AG-AG-TURN-048 | QUERY | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_events.rs | 988 |

#### BR-SYS-OBS-001

| codes                                    | type | chain        | 文件                           | 行号 |
| ---------------------------------------- | ---- | ------------ | ---------------------------- | -- |
| BR-SYS-OBS-001-DATA-ENT-CLOUD-INVOKE-001 | DATA | CLOUD-INVOKE | src-tauri/src/ent/gateway.rs | 71 |

#### BR-SYS-OBS-002

| codes                                | type | chain    | 文件                       | 行号 |
| ------------------------------------ | ---- | -------- | ------------------------ | -- |
| BR-SYS-OBS-002-DATA-SYS-IPC-CALL-002 | DATA | IPC-CALL | src-tauri/src/ipc/mod.rs | 15 |

#### BR-SYS-PERSIST-001

| codes                                          | type   | chain        | 文件                      | 行号  |
| ---------------------------------------------- | ------ | ------------ | ----------------------- | --- |
| BR-SYS-PERSIST-001-EFFECT-SYS-DATA-MIGRATE-001 | EFFECT | DATA-MIGRATE | src-tauri/src/db/mod.rs | 526 |

#### BR-SYS-PERSIST-001 term_ref: TERM-AG-019

| codes                                                                                    | type   | chain                          | 文件                             | 行号  |
| ---------------------------------------------------------------------------------------- | ------ | ------------------------------ | ------------------------------ | --- |
| BR-AG-DATA-003-EFFECT-AG-AG-TARGETSET-002, BR-SYS-PERSIST-001-EFFECT-AG-AG-TARGETSET-002 | EFFECT | AG-TURN, AG-TOOL, DATA-MIGRATE | src-tauri/src/ag/target_set.rs | 202 |

#### BR-SYS-STATE-001

| codes                                  | type | chain      | 文件                           | 行号 |
| -------------------------------------- | ---- | ---------- | ---------------------------- | -- |
| BR-SYS-STATE-001-RB-SYS-CLOUD-AUTH-001 | RB   | CLOUD-AUTH | src-tauri/src/session/mod.rs | 40 |

#### BR-SYS-STATE-002

| codes                                  | type | chain    | 文件                       | 行号 |
| -------------------------------------- | ---- | -------- | ------------------------ | -- |
| BR-SYS-STATE-002-DATA-SYS-IPC-CALL-003 | DATA | IPC-CALL | src-tauri/src/ipc/mod.rs | 77 |

#### BR-SYS-STATE-003

| codes                                    | type   | chain    | 文件                         | 行号 |
| ---------------------------------------- | ------ | -------- | -------------------------- | -- |
| BR-SYS-STATE-003-EFFECT-SYS-IPC-CALL-001 | EFFECT | IPC-CALL | src-tauri/src/sm_events.rs | 48 |

#### BR-SYS-STATE-004

| codes                                     | type   | chain     | 文件                                | 行号   |
| ----------------------------------------- | ------ | --------- | --------------------------------- | ---- |
| BR-SYS-STATE-004-GUARD-SYS-APP-CLOSE-003  | GUARD  | APP-CLOSE | src/shell/App.tsx                 | 38   |
| BR-SYS-STATE-004-GUARD-SYS-APP-CLOSE-004  | GUARD  | APP-CLOSE | src-tauri/src/lib.rs              | 498  |
| BR-SYS-STATE-004-QUERY-SYS-APP-CLOSE-001  | QUERY  | APP-CLOSE | src-tauri/src/workspace/routes.rs | 1184 |
| BR-SYS-STATE-004-EFFECT-SYS-APP-CLOSE-002 | EFFECT | APP-CLOSE | src-tauri/src/workspace/routes.rs | 1248 |

#### BR-SYS-STATE-005

| codes                                        | type   | chain        | 文件                            | 行号  |
| -------------------------------------------- | ------ | ------------ | ----------------------------- | --- |
| BR-SYS-STATE-005-EFFECT-SYS-DATA-MIGRATE-002 | EFFECT | DATA-MIGRATE | src-tauri/src/de/lifecycle.rs | 199 |
| BR-SYS-STATE-005-EFFECT-SYS-DATA-MIGRATE-003 | EFFECT | DATA-MIGRATE | src-tauri/src/de/lifecycle.rs | 237 |

#### BR-SYS-STATE-006

| codes                                     | type   | chain     | 文件                           | 行号  |
| ----------------------------------------- | ------ | --------- | ---------------------------- | --- |
| BR-SYS-STATE-006-EFFECT-SYS-APP-CLOSE-001 | EFFECT | APP-CLOSE | src/services/closeService.ts | 230 |

#### BR-SYS-STATE-007

| codes                                                                                                                                                                                                                                                   | type   | chain             | 文件                                     | 行号   |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ----------------- | -------------------------------------- | ---- |
| BR-SYS-STATE-007-EFFECT-AG-AG-TURN-058, BR-AG-UI-001-EFFECT-AG-AG-TURN-058                                                                                                                                                                              | EFFECT | AG-TURN           | src/components/ag/DecisionBar.tsx      | 40   |
| BR-SYS-STATE-007-EFFECT-AG-AG-TURN-060, BR-SYS-STATE-008-EFFECT-AG-AG-TURN-060                                                                                                                                                                          | EFFECT | AG-TURN, IPC-CALL | src/services/agService.ts              | 1650 |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-067, BR-SYS-STATE-007-QUERY-AG-AG-TURN-067                                                                                                                                                                           | QUERY  | AG-TURN           | src/services/agService.ts              | 1850 |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-068, BR-SYS-STATE-007-QUERY-AG-AG-TURN-068                                                                                                                                                                           | QUERY  | AG-TURN, IPC-CALL | src/services/agService.ts              | 1889 |
| BR-AG-OBS-001-EFFECT-AG-AG-TURN-035, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-035, BR-AG-STATE-001-EFFECT-AG-AG-TURN-035                                                                                                                                      | EFFECT | AG-TURN           | src/services/agService.ts              | 2178 |
| BR-SYS-STATE-007-EFFECT-AG-AG-TURN-059, BR-SYS-STATE-008-EFFECT-AG-AG-TURN-059                                                                                                                                                                          | EFFECT | AG-TURN           | src/store/agStore.ts                   | 492  |
| BR-AG-OBS-001-EFFECT-AG-AG-TURN-042, BR-AG-STATE-001-EFFECT-AG-AG-TURN-042, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-042, BR-SYS-STATE-008-EFFECT-AG-AG-TURN-042                                                                                              | EFFECT | AG-TURN           | src/store/agStore.ts                   | 560  |
| BR-AG-PERSIST-001-QUERY-AG-AG-TOOL-046, BR-SYS-STATE-007-QUERY-AG-AG-TOOL-046                                                                                                                                                                           | QUERY  | AG-TOOL, AG-TURN  | src-tauri/src/ag/capability_runtime.rs | 66   |
| BR-AG-STATE-002-EFFECT-AG-AG-TOOL-048, BR-SYS-STATE-007-EFFECT-AG-AG-TOOL-048, BR-AG-PERSIST-001-EFFECT-AG-AG-TOOL-048                                                                                                                                  | EFFECT | AG-TOOL, AG-TURN  | src-tauri/src/ag/capability_runtime.rs | 239  |
| BR-AG-STATE-002-EFFECT-AG-AG-TOOL-046, BR-SYS-STATE-007-EFFECT-AG-AG-TOOL-046, BR-AG-PERSIST-001-EFFECT-AG-AG-TOOL-046                                                                                                                                  | EFFECT | AG-TOOL, AG-TURN  | src-tauri/src/ag/capability_runtime.rs | 308  |
| BR-AG-DATA-003-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-007-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-008-DATA-AG-AG-TARGETSET-001                                                                                                                           | DATA   | AG-TURN, AG-TOOL  | src-tauri/src/ag/target_set.rs         | 32   |
| BR-SYS-STATE-007-EFFECT-AG-AG-TURN-047, BR-AG-STATE-001-EFFECT-AG-AG-TURN-047, BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-047                                                                                                                                  | EFFECT | AG-TURN           | src-tauri/src/ag/task.rs               | 859  |
| BR-AG-STATE-002-GUARD-AG-AG-TOOL-045, BR-SYS-STATE-007-GUARD-AG-AG-TOOL-045                                                                                                                                                                             | GUARD  | AG-TOOL           | src-tauri/src/ag/tool_exec.rs          | 954  |
| BR-AG-STATE-002-GUARD-AG-AG-TOOL-035, BR-SYS-STATE-007-GUARD-AG-AG-TOOL-035, BR-AG-DATA-003-GUARD-AG-AG-TOOL-035, BR-WS-DATA-001-GUARD-AG-AG-TOOL-035                                                                                                   | GUARD  | AG-TOOL           | src-tauri/src/ag/tool_registry.rs      | 456  |
| BR-AG-STATE-001-RB-AG-AG-TURN-001, BR-SYS-STATE-007-RB-AG-AG-TURN-001                                                                                                                                                                                   | RB     | AG-TURN           | src-tauri/src/ag/turn.rs               | 112  |
| BR-SYS-STATE-007-GUARD-AG-AG-TURN-034, BR-AG-STATE-001-GUARD-AG-AG-TURN-034                                                                                                                                                                             | GUARD  | AG-TURN           | src-tauri/src/ag/turn_control.rs       | 85   |
| BR-SYS-STATE-007-EFFECT-AG-AG-TURN-034, BR-AG-STATE-001-EFFECT-AG-AG-TURN-034                                                                                                                                                                           | EFFECT | AG-TURN           | src-tauri/src/ag/turn_control.rs       | 300  |
| BR-AG-OBS-001-API-AG-AG-TURN-018, BR-AG-STATE-001-API-AG-AG-TURN-018, BR-AG-STATE-004-API-AG-AG-TURN-018, BR-AG-DATA-008-API-AG-AG-TURN-018, BR-ED-DATA-008-API-AG-AG-TURN-018, BR-SYS-STATE-007-API-AG-AG-TURN-018, BR-SYS-DATA-008-API-AG-AG-TURN-018 | API    | AG-TURN, AG-TOOL  | src-tauri/src/ag/turn_runtime.rs       | 753  |

#### BR-SYS-STATE-007 term_ref: TERM-AG-001

| codes                                                                                                                                                                                              | type   | chain            | 文件                                 | 行号  |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ---------------- | ---------------------------------- | --- |
| BR-AG-PERSIST-001-GUARD-AG-AG-TURN-048, BR-SYS-STATE-007-GUARD-AG-AG-TURN-048                                                                                                                      | GUARD  | AG-TURN          | src-tauri/src/ag/history.rs        | 366 |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-047, BR-SYS-STATE-007-QUERY-AG-AG-TURN-047                                                                                                                      | QUERY  | AG-TURN          | src-tauri/src/ag/history_routes.rs | 131 |
| BR-AG-DATA-003-EFFECT-AG-AG-TURN-036, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-036                                                                                                                       | EFFECT | AG-TURN          | src-tauri/src/ag/turn_control.rs   | 475 |
| BR-AG-STATE-001-API-AG-AG-TURN-019, BR-AG-PERSIST-001-API-AG-AG-TURN-019, BR-SYS-STATE-007-API-AG-AG-TURN-019                                                                                      | API    | AG-TURN          | src-tauri/src/ag/turn_control.rs   | 620 |
| BR-AG-OBS-001-EFFECT-AG-AG-TURN-044, BR-AG-STATE-001-EFFECT-AG-AG-TURN-044, BR-AG-STATE-002-EFFECT-AG-AG-TURN-044, BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-044, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-044 | EFFECT | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_events.rs    | 476 |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-045, BR-SYS-STATE-007-QUERY-AG-AG-TURN-045                                                                                                                      | QUERY  | AG-TURN          | src-tauri/src/ag/turn_events.rs    | 933 |
| BR-AG-OBS-001-DATA-AG-AG-TURN-043, BR-AG-STATE-001-DATA-AG-AG-TURN-043, BR-AG-STATE-002-DATA-AG-AG-TURN-043, BR-AG-PERSIST-001-DATA-AG-AG-TURN-043, BR-SYS-STATE-007-DATA-AG-AG-TURN-043           | DATA   | AG-TURN, AG-TOOL | src-tauri/src/ipc/mod.rs           | 127 |

#### BR-SYS-STATE-007 term_ref: TERM-AG-013

| codes                                 | type  | chain            | 文件                               | 行号  |
| ------------------------------------- | ----- | ---------------- | -------------------------------- | --- |
| BR-SYS-STATE-007-GUARD-AG-AG-TURN-037 | GUARD | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_runtime.rs | 340 |

#### BR-SYS-STATE-007 term_ref: TERM-AG-020

| codes                                                                      | type  | chain            | 文件                               | 行号 |
| -------------------------------------------------------------------------- | ----- | ---------------- | -------------------------------- | -- |
| BR-AG-DATA-008-GUARD-AG-AG-TURN-053, BR-SYS-STATE-007-GUARD-AG-AG-TURN-053 | GUARD | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_runtime.rs | 88 |

#### BR-SYS-STATE-008

| codes                                                                                                                                                                                                                                                | type   | chain                          | 文件                                 | 行号   |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------ | ---------------------------------- | ---- |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-006, BR-AG-DATA-001-EFFECT-AG-AG-TURN-006                                                                                                                                                                         | EFFECT | AG-TURN, DE-DIFF, ED-EDIT      | src/services/agService.ts          | 1535 |
| BR-SYS-STATE-008-EFFECT-DE-DE-DIFF-008                                                                                                                                                                                                               | EFFECT | DE-DIFF, ED-EDIT               | src/services/deService.ts          | 126  |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-032, BR-AG-DATA-001-EFFECT-AG-AG-TURN-032                                                                                                                                                                         | EFFECT | AG-TURN, DE-DIFF, ED-EDIT      | src-tauri/src/ag/task.rs           | 110  |
| BR-SYS-STATE-008-GUARD-AG-AG-TURN-005, BR-AG-DATA-001-GUARD-AG-AG-TURN-005                                                                                                                                                                           | GUARD  | AG-TURN, DE-DIFF, ED-EDIT      | src-tauri/src/ag/task.rs           | 169  |
| BR-SYS-STATE-008-GUARD-AG-AG-TURN-006, BR-AG-DATA-001-GUARD-AG-AG-TURN-006                                                                                                                                                                           | GUARD  | AG-TURN, ED-EDIT               | src-tauri/src/ag/task.rs           | 287  |
| BR-SYS-STATE-008-EFFECT-AG-AG-DECISION-001, BR-AG-DATA-003-EFFECT-AG-AG-DECISION-001                                                                                                                                                                 | EFFECT | AG-TURN, AG-TOOL               | src-tauri/src/ag/task.rs           | 505  |
| BR-SYS-STATE-008-EFFECT-AG-AG-DECISION-002, BR-AG-DATA-003-EFFECT-AG-AG-DECISION-002                                                                                                                                                                 | EFFECT | AG-TURN, AG-TOOL               | src-tauri/src/ag/task.rs           | 580  |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-033, BR-AG-STATE-001-EFFECT-AG-AG-TURN-033                                                                                                                                                                        | EFFECT | AG-TURN                        | src-tauri/src/ag/task.rs           | 929  |
| BR-SYS-STATE-008-QUERY-AG-AG-TASK-004, BR-AG-PERSIST-001-QUERY-AG-AG-TASK-004                                                                                                                                                                        | QUERY  | AG-TURN                        | src-tauri/src/ag/task.rs           | 1137 |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-026, BR-AG-DATA-003-EFFECT-AG-AG-TURN-026                                                                                                                                                                         | EFFECT | AG-TURN                        | src-tauri/src/ag/task_runtime.rs   | 79   |
| BR-AG-STATE-002-API-AG-WS-OPERATION-005, BR-AG-DATA-003-API-AG-WS-OPERATION-005, BR-SYS-STATE-008-API-AG-WS-OPERATION-005, BR-WS-STATE-003-API-AG-WS-OPERATION-005, BR-WS-PERSIST-003-API-AG-WS-OPERATION-005, BR-WS-OBS-002-API-AG-WS-OPERATION-005 | API    | AG-TURN, AG-TOOL, WS-OPERATION | src-tauri/src/ag/task_runtime.rs   | 197  |
| BR-SYS-STATE-008-GUARD-AG-AG-TOOL-040, BR-WS-STATE-003-GUARD-AG-AG-TOOL-040                                                                                                                                                                          | GUARD  | AG-TOOL, WS-OPERATION          | src-tauri/src/ag/tool_registry.rs  | 415  |
| BR-SYS-STATE-008-API-AG-AG-TURN-003, BR-AG-DATA-001-API-AG-AG-TURN-003                                                                                                                                                                               | API    | AG-TURN, DE-DIFF, ED-EDIT      | src-tauri/src/ag/turn_control.rs   | 654  |
| BR-SYS-STATE-008-EFFECT-AG-AG-TOOL-071                                                                                                                                                                                                               | EFFECT | AG-TOOL                        | src-tauri/src/ag/turn_tool_loop.rs | 1711 |

#### BR-SYS-STATE-008 term_ref: TERM-AG-012

| codes                              | type | chain            | 文件                                 | 行号  |
| ---------------------------------- | ---- | ---------------- | ---------------------------------- | --- |
| BR-SYS-STATE-008-RB-AG-AG-TOOL-064 | RB   | AG-TOOL, AG-TURN | src-tauri/src/ag/turn_tool_loop.rs | 848 |

#### BR-SYS-STATE-008 term_ref: TERM-AG-015

| codes                                  | type   | chain            | 文件                               | 行号   |
| -------------------------------------- | ------ | ---------------- | -------------------------------- | ---- |
| BR-SYS-STATE-008-EFFECT-AG-AG-TASK-002 | EFFECT | AG-TURN, AG-TOOL | src-tauri/src/ag/task.rs         | 70   |
| BR-SYS-STATE-008-GUARD-AG-AG-TASK-004  | GUARD  | AG-TURN, AG-TOOL | src-tauri/src/ag/task.rs         | 995  |
| BR-SYS-STATE-008-GUARD-AG-AG-TASK-003  | GUARD  | AG-TURN, AG-TOOL | src-tauri/src/ag/task.rs         | 1019 |
| BR-SYS-STATE-008-QUERY-AG-AG-TURN-025  | QUERY  | AG-TURN          | src-tauri/src/ag/task_runtime.rs | 46   |
| BR-SYS-STATE-008-QUERY-AG-AG-TURN-027  | QUERY  | AG-TURN          | src-tauri/src/ag/task_runtime.rs | 775  |

#### BR-SYS-STATE-008 term_ref: TERM-AG-017

| codes                                                                          | type   | chain                 | 文件                        | 行号   |
| ------------------------------------------------------------------------------ | ------ | --------------------- | ------------------------- | ---- |
| BR-SYS-STATE-007-EFFECT-AG-AG-TURN-060, BR-SYS-STATE-008-EFFECT-AG-AG-TURN-060 | EFFECT | AG-TURN, IPC-CALL     | src/services/agService.ts | 1650 |
| BR-SYS-STATE-007-EFFECT-AG-AG-TURN-059, BR-SYS-STATE-008-EFFECT-AG-AG-TURN-059 | EFFECT | AG-TURN               | src/store/agStore.ts      | 492  |
| BR-AG-DATA-004-QUERY-AG-AG-TURN-052, BR-SYS-STATE-008-QUERY-AG-AG-TURN-052     | QUERY  | AG-TURN, WS-OPERATION | src-tauri/src/ag/task.rs  | 750  |

#### BR-SYS-STATE-008 term_ref: TERM-AG-019

| codes                                                                                                                         | type | chain            | 文件                             | 行号 |
| ----------------------------------------------------------------------------------------------------------------------------- | ---- | ---------------- | ------------------------------ | -- |
| BR-AG-DATA-003-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-007-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-008-DATA-AG-AG-TARGETSET-001 | DATA | AG-TURN, AG-TOOL | src-tauri/src/ag/target_set.rs | 32 |

#### BR-SYS-STATE-008 term_ref: TERM-AG-021

| codes                                                                                                                                                      | type   | chain   | 文件                   | 行号  |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------- | -------------------- | --- |
| BR-AG-OBS-001-EFFECT-AG-AG-TURN-042, BR-AG-STATE-001-EFFECT-AG-AG-TURN-042, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-042, BR-SYS-STATE-008-EFFECT-AG-AG-TURN-042 | EFFECT | AG-TURN | src/store/agStore.ts | 560 |

#### BR-SYS-VERIFY-001

| codes                                                                                | type  | chain                    | 文件                           | 行号 |
| ------------------------------------------------------------------------------------ | ----- | ------------------------ | ---------------------------- | -- |
| BR-SYS-VERIFY-001-GUARD-SYS-CLOUD-GATE-001, BR-SYS-DATA-001-GUARD-SYS-CLOUD-GATE-001 | GUARD | CLOUD-GATE, CLOUD-INVOKE | src-tauri/src/session/mod.rs | 76 |

#### BR-TPL-DATA-001

| codes                                   | type   | chain    | 文件                            | 行号 |
| --------------------------------------- | ------ | -------- | ----------------------------- | -- |
| BR-TPL-DATA-001-EFFECT-TPL-TPL-FLOW-001 | EFFECT | TPL-FLOW | src-tauri/src/tpl/template.rs | 43 |
| BR-TPL-DATA-001-QUERY-TPL-TPL-FLOW-001  | QUERY  | TPL-FLOW | src-tauri/src/tpl/template.rs | 71 |
| BR-TPL-DATA-001-EFFECT-TPL-TPL-FLOW-002 | EFFECT | TPL-FLOW | src-tauri/src/tpl/template.rs | 91 |

#### BR-TPL-STATE-001

| codes                                    | type   | chain    | 文件                            | 行号  |
| ---------------------------------------- | ------ | -------- | ----------------------------- | --- |
| BR-TPL-STATE-001-RB-TPL-TPL-FLOW-001     | RB     | TPL-FLOW | src-tauri/src/tpl/workflow.rs | 49  |
| BR-TPL-STATE-001-EFFECT-TPL-TPL-FLOW-003 | EFFECT | TPL-FLOW | src-tauri/src/tpl/workflow.rs | 109 |
| BR-TPL-STATE-001-QUERY-TPL-TPL-FLOW-002  | QUERY  | TPL-FLOW | src-tauri/src/tpl/workflow.rs | 138 |

#### BR-WS-DATA-001

| codes                                                                                                              | type   | chain   | 文件                                 | 行号  |
| ------------------------------------------------------------------------------------------------------------------ | ------ | ------- | ---------------------------------- | --- |
| BR-WS-DATA-001-EFFECT-WS-WS-FS-007                                                                                 | EFFECT | WS-FS   | src-tauri/src/workspace/fileops.rs | 100 |
| BR-WS-DATA-001-EFFECT-WS-WS-FS-008                                                                                 | EFFECT | WS-FS   | src-tauri/src/workspace/listing.rs | 148 |
| BR-WS-DATA-001-GUARD-WS-WS-FS-001                                                                                  | GUARD  | WS-FS   | src-tauri/src/workspace/mod.rs     | 46  |
| BR-WS-PERSIST-001-EFFECT-WS-WS-SAVE-003, BR-WS-OBS-001-EFFECT-WS-WS-SAVE-003, BR-WS-DATA-001-EFFECT-WS-WS-SAVE-003 | EFFECT | WS-SAVE | src-tauri/src/workspace/routes.rs  | 647 |

#### BR-WS-DATA-001 term_ref: TERM-AG-014

| codes                                                                                                                                                 | type  | chain   | 文件                                | 行号  |
| ----------------------------------------------------------------------------------------------------------------------------------------------------- | ----- | ------- | --------------------------------- | --- |
| BR-AG-STATE-002-GUARD-AG-AG-TOOL-035, BR-SYS-STATE-007-GUARD-AG-AG-TOOL-035, BR-AG-DATA-003-GUARD-AG-AG-TOOL-035, BR-WS-DATA-001-GUARD-AG-AG-TOOL-035 | GUARD | AG-TOOL | src-tauri/src/ag/tool_registry.rs | 456 |

#### BR-WS-DATA-001 term_ref: TERM-ED-008

| codes                                                                                                          | type  | chain       | 文件                               | 行号  |
| -------------------------------------------------------------------------------------------------------------- | ----- | ----------- | -------------------------------- | --- |
| BR-AG-STATE-002-QUERY-AG-AG-TOOL-012, BR-AG-DATA-001-QUERY-AG-AG-TOOL-012, BR-WS-DATA-001-QUERY-AG-AG-TOOL-012 | QUERY | AG-TOOL     | src-tauri/src/ag/tool_exec.rs    | 483 |
| BR-DOC-DATA-002-GUARD-DOC-DOC-CONVERT-011, BR-WS-DATA-001-GUARD-DOC-DOC-CONVERT-011                            | GUARD | DOC-CONVERT | src-tauri/src/doc/image_media.rs | 334 |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-015, BR-WS-DATA-001-IO-DOC-DOC-CONVERT-015                                  | IO    | DOC-CONVERT | src-tauri/src/doc/image_media.rs | 568 |

#### BR-WS-DATA-001 term_ref: TERM-WS-001

| codes                              | type   | chain | 文件                                 | 行号  |
| ---------------------------------- | ------ | ----- | ---------------------------------- | --- |
| BR-WS-DATA-001-EFFECT-WS-WS-FS-001 | EFFECT | WS-FS | src-tauri/src/workspace/fileops.rs | 16  |
| BR-WS-DATA-001-EFFECT-WS-WS-FS-004 | EFFECT | WS-FS | src-tauri/src/workspace/fileops.rs | 43  |
| BR-WS-DATA-001-EFFECT-WS-WS-FS-002 | EFFECT | WS-FS | src-tauri/src/workspace/fileops.rs | 70  |
| BR-WS-DATA-001-EFFECT-WS-WS-FS-003 | EFFECT | WS-FS | src-tauri/src/workspace/fileops.rs | 134 |
| BR-WS-DATA-001-QUERY-WS-WS-FS-002  | QUERY  | WS-FS | src-tauri/src/workspace/listing.rs | 174 |
| BR-WS-DATA-001-QUERY-WS-WS-FS-006  | QUERY  | WS-FS | src-tauri/src/workspace/listing.rs | 582 |
| BR-WS-DATA-001-QUERY-WS-WS-FS-005  | QUERY  | WS-FS | src-tauri/src/workspace/listing.rs | 703 |

#### BR-WS-DATA-002

| codes                                    | type   | chain       | 文件                                  | 行号  |
| ---------------------------------------- | ------ | ----------- | ----------------------------------- | --- |
| BR-WS-DATA-002-EFFECT-WS-WS-TIMELINE-001 | EFFECT | WS-TIMELINE | src-tauri/src/workspace/timeline.rs | 47  |
| BR-WS-DATA-002-QUERY-WS-WS-TIMELINE-001  | QUERY  | WS-TIMELINE | src-tauri/src/workspace/timeline.rs | 91  |
| BR-WS-DATA-002-EFFECT-WS-WS-TIMELINE-002 | EFFECT | WS-TIMELINE | src-tauri/src/workspace/timeline.rs | 138 |

#### BR-WS-DATA-003

| codes                                                                                                                                                                         | type   | chain                          | 文件                                   | 行号   |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------ | ------------------------------------ | ---- |
| BR-WS-DATA-003-EFFECT-WS-WS-TIMELINE-003, BR-WS-PERSIST-003-EFFECT-WS-WS-TIMELINE-003                                                                                         | EFFECT | WS-TIMELINE                    | src-tauri/src/workspace/operation.rs | 349  |
| BR-WS-STATE-003-API-WS-WS-OPERATION-001, BR-WS-DATA-003-API-WS-WS-OPERATION-001, BR-WS-OBS-002-API-WS-WS-OPERATION-001                                                        | API    | WS-OPERATION                   | src-tauri/src/workspace/operation.rs | 2261 |
| BR-WS-STATE-003-EFFECT-WS-WS-OPERATION-007, BR-WS-DATA-003-EFFECT-WS-WS-OPERATION-007, BR-WS-OBS-002-EFFECT-WS-WS-OPERATION-007                                               | EFFECT | WS-OPERATION                   | src-tauri/src/workspace/operation.rs | 2417 |
| BR-WS-STATE-003-EFFECT-WS-WS-OPERATION-003, BR-WS-DATA-003-EFFECT-WS-WS-OPERATION-003, BR-WS-OBS-002-EFFECT-WS-WS-OPERATION-003, BR-WS-PERSIST-003-EFFECT-WS-WS-OPERATION-003 | EFFECT | WS-OPERATION                   | src-tauri/src/workspace/operation.rs | 2943 |
| BR-WS-STATE-003-API-WS-WS-OPERATION-002, BR-WS-DATA-003-API-WS-WS-OPERATION-002, BR-WS-OBS-002-API-WS-WS-OPERATION-002                                                        | API    | WS-OPERATION                   | src-tauri/src/workspace/routes.rs    | 1467 |
| BR-WS-STATE-003-API-WS-WS-OPERATION-004, BR-WS-DATA-003-API-WS-WS-OPERATION-004, BR-WS-OBS-002-API-WS-WS-OPERATION-004                                                        | API    | WS-OPERATION, DE-DIFF, ED-EDIT | src-tauri/src/workspace/routes.rs    | 1861 |

#### BR-WS-DATA-003 term_ref: TERM-WS-003

| codes                                                                               | type  | chain        | 文件                                   | 行号   |
| ----------------------------------------------------------------------------------- | ----- | ------------ | ------------------------------------ | ---- |
| BR-WS-STATE-003-QUERY-WS-WS-OPERATION-006, BR-WS-DATA-003-QUERY-WS-WS-OPERATION-006 | QUERY | WS-OPERATION | src-tauri/src/workspace/operation.rs | 2291 |

#### BR-WS-DATA-003 term_ref: TERM-WS-004

| codes                                     | type   | chain        | 文件                                   | 行号  |
| ----------------------------------------- | ------ | ------------ | ------------------------------------ | --- |
| BR-WS-DATA-003-EFFECT-WS-WS-OPERATION-001 | EFFECT | WS-OPERATION | src-tauri/src/workspace/operation.rs | 176 |

#### BR-WS-OBS-001

| codes                                                                                                              | type   | chain   | 文件                                | 行号   |
| ------------------------------------------------------------------------------------------------------------------ | ------ | ------- | --------------------------------- | ---- |
| BR-WS-PERSIST-001-EFFECT-WS-WS-SAVE-003, BR-WS-OBS-001-EFFECT-WS-WS-SAVE-003, BR-WS-DATA-001-EFFECT-WS-WS-SAVE-003 | EFFECT | WS-SAVE | src-tauri/src/workspace/routes.rs | 647  |
| BR-WS-OBS-001-RB-WS-WS-FS-001                                                                                      | RB     | WS-FS   | src-tauri/src/workspace/routes.rs | 1363 |

#### BR-WS-OBS-002

| codes                                                                                                                                                                         | type   | chain        | 文件                                   | 行号   |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------ | ------------------------------------ | ---- |
| BR-WS-STATE-003-EFFECT-WS-WS-OPERATION-003, BR-WS-DATA-003-EFFECT-WS-WS-OPERATION-003, BR-WS-OBS-002-EFFECT-WS-WS-OPERATION-003, BR-WS-PERSIST-003-EFFECT-WS-WS-OPERATION-003 | EFFECT | WS-OPERATION | src-tauri/src/workspace/operation.rs | 2943 |

#### BR-WS-OBS-002 term_ref: TERM-AG-015

| codes                                                                                                                                                                                                                                                | type | chain                          | 文件                               | 行号  |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---- | ------------------------------ | -------------------------------- | --- |
| BR-AG-STATE-002-API-AG-WS-OPERATION-005, BR-AG-DATA-003-API-AG-WS-OPERATION-005, BR-SYS-STATE-008-API-AG-WS-OPERATION-005, BR-WS-STATE-003-API-AG-WS-OPERATION-005, BR-WS-PERSIST-003-API-AG-WS-OPERATION-005, BR-WS-OBS-002-API-AG-WS-OPERATION-005 | API  | AG-TURN, AG-TOOL, WS-OPERATION | src-tauri/src/ag/task_runtime.rs | 197 |

#### BR-WS-OBS-002 term_ref: TERM-WS-003

| codes                                                                                                                           | type   | chain                          | 文件                                   | 行号   |
| ------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------ | ------------------------------------ | ---- |
| BR-WS-STATE-003-API-WS-WS-OPERATION-001, BR-WS-DATA-003-API-WS-WS-OPERATION-001, BR-WS-OBS-002-API-WS-WS-OPERATION-001          | API    | WS-OPERATION                   | src-tauri/src/workspace/operation.rs | 2261 |
| BR-WS-STATE-003-EFFECT-WS-WS-OPERATION-007, BR-WS-DATA-003-EFFECT-WS-WS-OPERATION-007, BR-WS-OBS-002-EFFECT-WS-WS-OPERATION-007 | EFFECT | WS-OPERATION                   | src-tauri/src/workspace/operation.rs | 2417 |
| BR-WS-STATE-003-API-WS-WS-OPERATION-002, BR-WS-DATA-003-API-WS-WS-OPERATION-002, BR-WS-OBS-002-API-WS-WS-OPERATION-002          | API    | WS-OPERATION                   | src-tauri/src/workspace/routes.rs    | 1467 |
| BR-WS-STATE-003-API-WS-WS-OPERATION-004, BR-WS-DATA-003-API-WS-WS-OPERATION-004, BR-WS-OBS-002-API-WS-WS-OPERATION-004          | API    | WS-OPERATION, DE-DIFF, ED-EDIT | src-tauri/src/workspace/routes.rs    | 1861 |

#### BR-WS-PERSIST-001

| codes                                                                                                              | type   | chain   | 文件                                | 行号  |
| ------------------------------------------------------------------------------------------------------------------ | ------ | ------- | --------------------------------- | --- |
| BR-WS-PERSIST-001-EFFECT-WS-WS-SAVE-003, BR-WS-OBS-001-EFFECT-WS-WS-SAVE-003, BR-WS-DATA-001-EFFECT-WS-WS-SAVE-003 | EFFECT | WS-SAVE | src-tauri/src/workspace/routes.rs | 647 |
| BR-WS-PERSIST-001-EFFECT-WS-WS-SAVE-001                                                                            | EFFECT | WS-SAVE | src-tauri/src/workspace/save.rs   | 51  |
| BR-WS-PERSIST-001-RB-WS-WS-SAVE-002                                                                                | RB     | WS-SAVE | src-tauri/src/workspace/save.rs   | 103 |

#### BR-WS-PERSIST-002

| codes                                        | type   | chain        | 文件                                   | 行号  |
| -------------------------------------------- | ------ | ------------ | ------------------------------------ | --- |
| BR-WS-PERSIST-002-EFFECT-WS-DATA-MIGRATE-005 | EFFECT | DATA-MIGRATE | src/services/workspaceService.ts     | 193 |
| BR-WS-PERSIST-002-RB-SYS-DATA-MIGRATE-001    | RB     | DATA-MIGRATE | src/shell/App.tsx                    | 18  |
| BR-WS-PERSIST-002-RB-WS-DATA-MIGRATE-001     | RB     | DATA-MIGRATE | src/shell/Welcome.tsx                | 18  |
| BR-WS-PERSIST-002-RB-WS-DATA-MIGRATE-002     | RB     | DATA-MIGRATE | src/shell/Workbench.tsx              | 29  |
| BR-WS-PERSIST-002-EFFECT-WS-DATA-MIGRATE-004 | EFFECT | DATA-MIGRATE | src/store/workspaceShell.ts          | 46  |
| BR-WS-PERSIST-002-EFFECT-WS-DATA-MIGRATE-001 | EFFECT | DATA-MIGRATE | src-tauri/src/workspace/lifecycle.rs | 34  |
| BR-WS-PERSIST-002-EFFECT-WS-DATA-MIGRATE-002 | EFFECT | DATA-MIGRATE | src-tauri/src/workspace/lifecycle.rs | 71  |
| BR-WS-PERSIST-002-EFFECT-WS-DATA-MIGRATE-003 | EFFECT | DATA-MIGRATE | src-tauri/src/workspace/lifecycle.rs | 99  |
| BR-WS-PERSIST-002-QUERY-WS-DATA-MIGRATE-001  | QUERY  | DATA-MIGRATE | src-tauri/src/workspace/lifecycle.rs | 116 |
| BR-WS-PERSIST-002-QUERY-WS-DATA-MIGRATE-002  | QUERY  | DATA-MIGRATE | src-tauri/src/workspace/lifecycle.rs | 140 |

#### BR-WS-PERSIST-003

| codes                                                                                                                                                                                                                                                | type | chain                          | 文件                               | 行号  |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---- | ------------------------------ | -------------------------------- | --- |
| BR-AG-STATE-002-API-AG-WS-OPERATION-005, BR-AG-DATA-003-API-AG-WS-OPERATION-005, BR-SYS-STATE-008-API-AG-WS-OPERATION-005, BR-WS-STATE-003-API-AG-WS-OPERATION-005, BR-WS-PERSIST-003-API-AG-WS-OPERATION-005, BR-WS-OBS-002-API-AG-WS-OPERATION-005 | API  | AG-TURN, AG-TOOL, WS-OPERATION | src-tauri/src/ag/task_runtime.rs | 197 |

#### BR-WS-PERSIST-003 term_ref: TERM-WS-003

| codes                                                                                                                                                                         | type   | chain                 | 文件                                   | 行号   |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | --------------------- | ------------------------------------ | ---- |
| BR-WS-STATE-003-RB-WS-WS-OPERATION-001, BR-WS-PERSIST-003-RB-WS-WS-OPERATION-001                                                                                              | RB     | WS-OPERATION          | src-tauri/src/workspace/operation.rs | 143  |
| BR-WS-PERSIST-003-EFFECT-WS-WS-OPERATION-002                                                                                                                                  | EFFECT | WS-OPERATION          | src-tauri/src/workspace/operation.rs | 1818 |
| BR-WS-STATE-003-EFFECT-WS-WS-OPERATION-003, BR-WS-DATA-003-EFFECT-WS-WS-OPERATION-003, BR-WS-OBS-002-EFFECT-WS-WS-OPERATION-003, BR-WS-PERSIST-003-EFFECT-WS-WS-OPERATION-003 | EFFECT | WS-OPERATION          | src-tauri/src/workspace/operation.rs | 2943 |
| BR-WS-STATE-003-EFFECT-WS-WS-OPERATION-008, BR-WS-PERSIST-003-EFFECT-WS-WS-OPERATION-008                                                                                      | EFFECT | WS-OPERATION, AG-TURN | src-tauri/src/workspace/operation.rs | 2966 |

#### BR-WS-PERSIST-003 term_ref: TERM-WS-005

| codes                                                                                 | type   | chain       | 文件                                   | 行号  |
| ------------------------------------------------------------------------------------- | ------ | ----------- | ------------------------------------ | --- |
| BR-WS-DATA-003-EFFECT-WS-WS-TIMELINE-003, BR-WS-PERSIST-003-EFFECT-WS-WS-TIMELINE-003 | EFFECT | WS-TIMELINE | src-tauri/src/workspace/operation.rs | 349 |

#### BR-WS-STATE-001

| codes                                       | type   | chain         | 文件                                    | 行号  |
| ------------------------------------------- | ------ | ------------- | ------------------------------------- | --- |
| BR-WS-STATE-001-RB-WS-WS-DIFF-APPLY-001     | RB     | WS-DIFF-APPLY | src-tauri/src/workspace/diff_apply.rs | 45  |
| BR-WS-STATE-001-EFFECT-WS-WS-DIFF-APPLY-001 | EFFECT | WS-DIFF-APPLY | src-tauri/src/workspace/diff_apply.rs | 76  |
| BR-WS-STATE-001-EFFECT-WS-WS-DIFF-APPLY-002 | EFFECT | WS-DIFF-APPLY | src-tauri/src/workspace/diff_apply.rs | 372 |
| BR-WS-STATE-001-EFFECT-WS-WS-DIFF-APPLY-003 | EFFECT | WS-DIFF-APPLY | src-tauri/src/workspace/diff_apply.rs | 445 |
| BR-WS-STATE-001-QUERY-WS-WS-DIFF-APPLY-001  | QUERY  | WS-DIFF-APPLY | src-tauri/src/workspace/diff_apply.rs | 524 |

#### BR-WS-STATE-002

| codes                                    | type  | chain       | 文件                                  | 行号  |
| ---------------------------------------- | ----- | ----------- | ----------------------------------- | --- |
| BR-WS-STATE-002-QUERY-WS-WS-TIMELINE-002 | QUERY | WS-TIMELINE | src-tauri/src/workspace/timeline.rs | 202 |
| BR-WS-STATE-002-RB-WS-WS-TIMELINE-001    | RB    | WS-TIMELINE | src-tauri/src/workspace/timeline.rs | 242 |

#### BR-WS-STATE-003

| codes                                                                                                                                                                                                                                                | type   | chain                          | 文件                                   | 行号   |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------ | ------------------------------------ | ---- |
| BR-AG-STATE-002-API-AG-WS-OPERATION-005, BR-AG-DATA-003-API-AG-WS-OPERATION-005, BR-SYS-STATE-008-API-AG-WS-OPERATION-005, BR-WS-STATE-003-API-AG-WS-OPERATION-005, BR-WS-PERSIST-003-API-AG-WS-OPERATION-005, BR-WS-OBS-002-API-AG-WS-OPERATION-005 | API    | AG-TURN, AG-TOOL, WS-OPERATION | src-tauri/src/ag/task_runtime.rs     | 197  |
| BR-WS-STATE-003-RB-WS-WS-OPERATION-001, BR-WS-PERSIST-003-RB-WS-WS-OPERATION-001                                                                                                                                                                     | RB     | WS-OPERATION                   | src-tauri/src/workspace/operation.rs | 143  |
| BR-WS-STATE-003-API-WS-WS-OPERATION-001, BR-WS-DATA-003-API-WS-WS-OPERATION-001, BR-WS-OBS-002-API-WS-WS-OPERATION-001                                                                                                                               | API    | WS-OPERATION                   | src-tauri/src/workspace/operation.rs | 2261 |
| BR-WS-STATE-003-QUERY-WS-WS-OPERATION-006, BR-WS-DATA-003-QUERY-WS-WS-OPERATION-006                                                                                                                                                                  | QUERY  | WS-OPERATION                   | src-tauri/src/workspace/operation.rs | 2291 |
| BR-WS-STATE-003-EFFECT-WS-WS-OPERATION-007, BR-WS-DATA-003-EFFECT-WS-WS-OPERATION-007, BR-WS-OBS-002-EFFECT-WS-WS-OPERATION-007                                                                                                                      | EFFECT | WS-OPERATION                   | src-tauri/src/workspace/operation.rs | 2417 |
| BR-WS-STATE-003-EFFECT-WS-WS-OPERATION-003, BR-WS-DATA-003-EFFECT-WS-WS-OPERATION-003, BR-WS-OBS-002-EFFECT-WS-WS-OPERATION-003, BR-WS-PERSIST-003-EFFECT-WS-WS-OPERATION-003                                                                        | EFFECT | WS-OPERATION                   | src-tauri/src/workspace/operation.rs | 2943 |
| BR-WS-STATE-003-EFFECT-WS-WS-OPERATION-008, BR-WS-PERSIST-003-EFFECT-WS-WS-OPERATION-008                                                                                                                                                             | EFFECT | WS-OPERATION, AG-TURN          | src-tauri/src/workspace/operation.rs | 2966 |
| BR-WS-STATE-003-API-WS-WS-OPERATION-002, BR-WS-DATA-003-API-WS-WS-OPERATION-002, BR-WS-OBS-002-API-WS-WS-OPERATION-002                                                                                                                               | API    | WS-OPERATION                   | src-tauri/src/workspace/routes.rs    | 1467 |
| BR-WS-STATE-003-API-WS-WS-OPERATION-004, BR-WS-DATA-003-API-WS-WS-OPERATION-004, BR-WS-OBS-002-API-WS-WS-OPERATION-004                                                                                                                               | API    | WS-OPERATION, DE-DIFF, ED-EDIT | src-tauri/src/workspace/routes.rs    | 1861 |

#### BR-WS-STATE-003 term_ref: TERM-AG-014

| codes                                                                       | type  | chain                 | 文件                                | 行号  |
| --------------------------------------------------------------------------- | ----- | --------------------- | --------------------------------- | --- |
| BR-SYS-STATE-008-GUARD-AG-AG-TOOL-040, BR-WS-STATE-003-GUARD-AG-AG-TOOL-040 | GUARD | AG-TOOL, WS-OPERATION | src-tauri/src/ag/tool_registry.rs | 415 |

#### BR-WS-STATE-009 term_ref: TERM-WS-003

| codes                               | type   | chain              | 文件                                   | 行号  |
| ----------------------------------- | ------ | ------------------ | ------------------------------------ | --- |
| BR-WS-STATE-009-EFFECT-WS-WS-FS-009 | EFFECT | WS-FS, WS-TIMELINE | src-tauri/src/workspace/operation.rs | 410 |
| BR-WS-STATE-009-EFFECT-WS-WS-FS-010 | EFFECT | WS-FS, WS-TIMELINE | src-tauri/src/workspace/operation.rs | 470 |

#### TERM-AG-001

| codes                                                                                                                         | type   | chain             | 文件                             | 行号   |
| ----------------------------------------------------------------------------------------------------------------------------- | ------ | ----------------- | ------------------------------ | ---- |
| BR-SYS-STATE-007-EFFECT-AG-AG-TURN-060, BR-SYS-STATE-008-EFFECT-AG-AG-TURN-060                                                | EFFECT | AG-TURN, IPC-CALL | src/services/agService.ts      | 1650 |
| BR-AG-DATA-003-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-007-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-008-DATA-AG-AG-TARGETSET-001 | DATA   | AG-TURN, AG-TOOL  | src-tauri/src/ag/target_set.rs | 32   |

#### TERM-AG-002

| codes                                                                                                                                                                                              | type   | chain            | 文件                              | 行号  |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ---------------- | ------------------------------- | --- |
| BR-AG-OBS-001-EFFECT-AG-AG-TURN-044, BR-AG-STATE-001-EFFECT-AG-AG-TURN-044, BR-AG-STATE-002-EFFECT-AG-AG-TURN-044, BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-044, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-044 | EFFECT | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_events.rs | 476 |
| BR-AG-OBS-001-DATA-AG-AG-TURN-043, BR-AG-STATE-001-DATA-AG-AG-TURN-043, BR-AG-STATE-002-DATA-AG-AG-TURN-043, BR-AG-PERSIST-001-DATA-AG-AG-TURN-043, BR-SYS-STATE-007-DATA-AG-AG-TURN-043           | DATA   | AG-TURN, AG-TOOL | src-tauri/src/ipc/mod.rs        | 127 |

#### TERM-AG-004

| codes                                   | type   | chain     | 文件                                   | 行号   |
| --------------------------------------- | ------ | --------- | ------------------------------------ | ---- |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-007     | RB     | AG-ASSIST | src/components/ed/Editor.tsx         | 810  |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-002     | RB     | AG-ASSIST | src/components/ed/Editor.tsx         | 1938 |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-004     | RB     | AG-ASSIST | src/components/ed/assistPlacement.ts | 36   |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-005     | RB     | AG-ASSIST | src/components/ed/assistPlacement.ts | 60   |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-006     | RB     | AG-ASSIST | src/components/ed/assistPlacement.ts | 78   |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-003     | RB     | AG-ASSIST | src/components/ed/assistPlacement.ts | 96   |
| BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-002 | EFFECT | AG-ASSIST | src/services/agService.ts            | 2111 |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-008     | RB     | AG-ASSIST | src/store/assistStore.ts             | 94   |

#### TERM-AG-009

| codes                                                                         | type   | chain   | 文件                          | 行号  |
| ----------------------------------------------------------------------------- | ------ | ------- | --------------------------- | --- |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-003                                       | EFFECT | AG-TURN | src-tauri/src/ag/history.rs | 304 |
| BR-AG-PERSIST-001-GUARD-AG-AG-TURN-048, BR-SYS-STATE-007-GUARD-AG-AG-TURN-048 | GUARD  | AG-TURN | src-tauri/src/ag/history.rs | 366 |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-046                                       | EFFECT | AG-TURN | src-tauri/src/ag/history.rs | 398 |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-003                                        | QUERY  | AG-TURN | src-tauri/src/ag/history.rs | 462 |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-031                                       | EFFECT | AG-TURN | src-tauri/src/ag/history.rs | 509 |

#### TERM-AG-012

| codes                                                                                                                                                                                                                                                   | type | chain            | 文件                               | 行号  |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---- | ---------------- | -------------------------------- | --- |
| BR-AG-OBS-001-API-AG-AG-TURN-018, BR-AG-STATE-001-API-AG-AG-TURN-018, BR-AG-STATE-004-API-AG-AG-TURN-018, BR-AG-DATA-008-API-AG-AG-TURN-018, BR-ED-DATA-008-API-AG-AG-TURN-018, BR-SYS-STATE-007-API-AG-AG-TURN-018, BR-SYS-DATA-008-API-AG-AG-TURN-018 | API  | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_runtime.rs | 753 |

#### TERM-AG-013

| codes                                                                                                                                                                                                                                                   | type | chain            | 文件                               | 行号  |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---- | ---------------- | -------------------------------- | --- |
| BR-AG-OBS-001-API-AG-AG-TURN-018, BR-AG-STATE-001-API-AG-AG-TURN-018, BR-AG-STATE-004-API-AG-AG-TURN-018, BR-AG-DATA-008-API-AG-AG-TURN-018, BR-ED-DATA-008-API-AG-AG-TURN-018, BR-SYS-STATE-007-API-AG-AG-TURN-018, BR-SYS-DATA-008-API-AG-AG-TURN-018 | API  | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_runtime.rs | 753 |

#### TERM-AG-014

| codes                                                                                                                                                                                                                                                   | type | chain            | 文件                                | 行号  |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---- | ---------------- | --------------------------------- | --- |
| BR-AG-STATE-002-DATA-AG-AG-TOOL-044, BR-SYS-DATA-008-DATA-AG-AG-TOOL-044                                                                                                                                                                                | DATA | AG-TOOL          | src-tauri/src/ag/tool_exec.rs     | 18  |
| BR-AG-STATE-002-DATA-AG-AG-TOOL-039, BR-AG-DATA-004-DATA-AG-AG-TOOL-039                                                                                                                                                                                 | DATA | AG-TURN, AG-TOOL | src-tauri/src/ag/tool_registry.rs | 169 |
| BR-AG-OBS-001-API-AG-AG-TURN-018, BR-AG-STATE-001-API-AG-AG-TURN-018, BR-AG-STATE-004-API-AG-AG-TURN-018, BR-AG-DATA-008-API-AG-AG-TURN-018, BR-ED-DATA-008-API-AG-AG-TURN-018, BR-SYS-STATE-007-API-AG-AG-TURN-018, BR-SYS-DATA-008-API-AG-AG-TURN-018 | API  | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_runtime.rs  | 753 |

#### TERM-AG-015

| codes                                                                                                                                                                                                                                                   | type   | chain                          | 文件                                 | 行号   |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------ | ---------------------------------- | ---- |
| BR-AG-DATA-003-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-007-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-008-DATA-AG-AG-TARGETSET-001                                                                                                                           | DATA   | AG-TURN, AG-TOOL               | src-tauri/src/ag/target_set.rs     | 32   |
| BR-AG-DATA-003-EFFECT-AG-AG-TARGETSET-002, BR-SYS-PERSIST-001-EFFECT-AG-AG-TARGETSET-002                                                                                                                                                                | EFFECT | AG-TURN, AG-TOOL, DATA-MIGRATE | src-tauri/src/ag/target_set.rs     | 202  |
| BR-AG-DATA-004-QUERY-AG-AG-TURN-052, BR-SYS-STATE-008-QUERY-AG-AG-TURN-052                                                                                                                                                                              | QUERY  | AG-TURN, WS-OPERATION          | src-tauri/src/ag/task.rs           | 750  |
| BR-SYS-STATE-008-GUARD-AG-AG-TOOL-040, BR-WS-STATE-003-GUARD-AG-AG-TOOL-040                                                                                                                                                                             | GUARD  | AG-TOOL, WS-OPERATION          | src-tauri/src/ag/tool_registry.rs  | 415  |
| BR-SYS-STATE-007-GUARD-AG-AG-TURN-034, BR-AG-STATE-001-GUARD-AG-AG-TURN-034                                                                                                                                                                             | GUARD  | AG-TURN                        | src-tauri/src/ag/turn_control.rs   | 85   |
| BR-SYS-STATE-007-EFFECT-AG-AG-TURN-034, BR-AG-STATE-001-EFFECT-AG-AG-TURN-034                                                                                                                                                                           | EFFECT | AG-TURN                        | src-tauri/src/ag/turn_control.rs   | 300  |
| BR-AG-OBS-001-API-AG-AG-TURN-018, BR-AG-STATE-001-API-AG-AG-TURN-018, BR-AG-STATE-004-API-AG-AG-TURN-018, BR-AG-DATA-008-API-AG-AG-TURN-018, BR-ED-DATA-008-API-AG-AG-TURN-018, BR-SYS-STATE-007-API-AG-AG-TURN-018, BR-SYS-DATA-008-API-AG-AG-TURN-018 | API    | AG-TURN, AG-TOOL               | src-tauri/src/ag/turn_runtime.rs   | 753  |
| BR-SYS-STATE-008-RB-AG-AG-TOOL-064                                                                                                                                                                                                                      | RB     | AG-TOOL, AG-TURN               | src-tauri/src/ag/turn_tool_loop.rs | 848  |
| BR-AG-STATE-002-RB-AG-AG-TOOL-018                                                                                                                                                                                                                       | RB     | AG-TURN, AG-TOOL               | src-tauri/src/ag/turn_tool_loop.rs | 2677 |

#### TERM-AG-016

| codes                                                                                                                                                                                                                                                   | type  | chain            | 文件                                 | 行号   |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----- | ---------------- | ---------------------------------- | ---- |
| BR-SYS-STATE-008-QUERY-AG-AG-TASK-004, BR-AG-PERSIST-001-QUERY-AG-AG-TASK-004                                                                                                                                                                           | QUERY | AG-TURN          | src-tauri/src/ag/task.rs           | 1137 |
| BR-AG-OBS-001-API-AG-AG-TURN-018, BR-AG-STATE-001-API-AG-AG-TURN-018, BR-AG-STATE-004-API-AG-AG-TURN-018, BR-AG-DATA-008-API-AG-AG-TURN-018, BR-ED-DATA-008-API-AG-AG-TURN-018, BR-SYS-STATE-007-API-AG-AG-TURN-018, BR-SYS-DATA-008-API-AG-AG-TURN-018 | API   | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_runtime.rs   | 753  |
| BR-AG-STATE-002-RB-AG-AG-TOOL-018                                                                                                                                                                                                                       | RB    | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_tool_loop.rs | 2677 |

#### TERM-AG-017

| codes                                                                                                                                                                                                                                                | type   | chain                          | 文件                                | 行号  |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------ | --------------------------------- | --- |
| BR-SYS-STATE-008-EFFECT-AG-AG-DECISION-001, BR-AG-DATA-003-EFFECT-AG-AG-DECISION-001                                                                                                                                                                 | EFFECT | AG-TURN, AG-TOOL               | src-tauri/src/ag/task.rs          | 505 |
| BR-SYS-STATE-008-EFFECT-AG-AG-DECISION-002, BR-AG-DATA-003-EFFECT-AG-AG-DECISION-002                                                                                                                                                                 | EFFECT | AG-TURN, AG-TOOL               | src-tauri/src/ag/task.rs          | 580 |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-026, BR-AG-DATA-003-EFFECT-AG-AG-TURN-026                                                                                                                                                                         | EFFECT | AG-TURN                        | src-tauri/src/ag/task_runtime.rs  | 79  |
| BR-AG-STATE-002-API-AG-WS-OPERATION-005, BR-AG-DATA-003-API-AG-WS-OPERATION-005, BR-SYS-STATE-008-API-AG-WS-OPERATION-005, BR-WS-STATE-003-API-AG-WS-OPERATION-005, BR-WS-PERSIST-003-API-AG-WS-OPERATION-005, BR-WS-OBS-002-API-AG-WS-OPERATION-005 | API    | AG-TURN, AG-TOOL, WS-OPERATION | src-tauri/src/ag/task_runtime.rs  | 197 |
| BR-AG-STATE-002-GUARD-AG-AG-TOOL-035, BR-SYS-STATE-007-GUARD-AG-AG-TOOL-035, BR-AG-DATA-003-GUARD-AG-AG-TOOL-035, BR-WS-DATA-001-GUARD-AG-AG-TOOL-035                                                                                                | GUARD  | AG-TOOL                        | src-tauri/src/ag/tool_registry.rs | 456 |
| BR-AG-DATA-003-GUARD-AG-AG-TOOL-033, BR-AG-STATE-002-GUARD-AG-AG-TOOL-033                                                                                                                                                                            | GUARD  | AG-TOOL                        | src-tauri/src/ag/tool_registry.rs | 850 |

#### TERM-AG-018

| codes                                                                         | type   | chain            | 文件                               | 行号   |
| ----------------------------------------------------------------------------- | ------ | ---------------- | -------------------------------- | ---- |
| BR-SYS-STATE-008-EFFECT-AG-AG-TASK-002                                        | EFFECT | AG-TURN, AG-TOOL | src-tauri/src/ag/task.rs         | 70   |
| BR-SYS-STATE-008-GUARD-AG-AG-TASK-004                                         | GUARD  | AG-TURN, AG-TOOL | src-tauri/src/ag/task.rs         | 995  |
| BR-SYS-STATE-008-GUARD-AG-AG-TASK-003                                         | GUARD  | AG-TURN, AG-TOOL | src-tauri/src/ag/task.rs         | 1019 |
| BR-SYS-STATE-008-QUERY-AG-AG-TASK-004, BR-AG-PERSIST-001-QUERY-AG-AG-TASK-004 | QUERY  | AG-TURN          | src-tauri/src/ag/task.rs         | 1137 |
| BR-SYS-STATE-008-QUERY-AG-AG-TURN-027                                         | QUERY  | AG-TURN          | src-tauri/src/ag/task_runtime.rs | 775  |

#### TERM-AG-019

| codes                                                                                                                                                                                                                                                | type   | chain                          | 文件                                | 行号  |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------ | --------------------------------- | --- |
| BR-SYS-STATE-008-EFFECT-AG-AG-DECISION-001, BR-AG-DATA-003-EFFECT-AG-AG-DECISION-001                                                                                                                                                                 | EFFECT | AG-TURN, AG-TOOL               | src-tauri/src/ag/task.rs          | 505 |
| BR-SYS-STATE-008-EFFECT-AG-AG-DECISION-002, BR-AG-DATA-003-EFFECT-AG-AG-DECISION-002                                                                                                                                                                 | EFFECT | AG-TURN, AG-TOOL               | src-tauri/src/ag/task.rs          | 580 |
| BR-AG-DATA-003-EFFECT-AG-AG-TURN-037                                                                                                                                                                                                                 | EFFECT | AG-TURN, WS-OPERATION          | src-tauri/src/ag/task_runtime.rs  | 16  |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-026, BR-AG-DATA-003-EFFECT-AG-AG-TURN-026                                                                                                                                                                         | EFFECT | AG-TURN                        | src-tauri/src/ag/task_runtime.rs  | 79  |
| BR-AG-STATE-002-API-AG-WS-OPERATION-005, BR-AG-DATA-003-API-AG-WS-OPERATION-005, BR-SYS-STATE-008-API-AG-WS-OPERATION-005, BR-WS-STATE-003-API-AG-WS-OPERATION-005, BR-WS-PERSIST-003-API-AG-WS-OPERATION-005, BR-WS-OBS-002-API-AG-WS-OPERATION-005 | API    | AG-TURN, AG-TOOL, WS-OPERATION | src-tauri/src/ag/task_runtime.rs  | 197 |
| BR-AG-STATE-002-DATA-AG-AG-TOOL-039, BR-AG-DATA-004-DATA-AG-AG-TOOL-039                                                                                                                                                                              | DATA   | AG-TURN, AG-TOOL               | src-tauri/src/ag/tool_registry.rs | 169 |
| BR-AG-STATE-002-GUARD-AG-AG-TOOL-035, BR-SYS-STATE-007-GUARD-AG-AG-TOOL-035, BR-AG-DATA-003-GUARD-AG-AG-TOOL-035, BR-WS-DATA-001-GUARD-AG-AG-TOOL-035                                                                                                | GUARD  | AG-TOOL                        | src-tauri/src/ag/tool_registry.rs | 456 |
| BR-SYS-STATE-007-GUARD-AG-AG-TURN-034, BR-AG-STATE-001-GUARD-AG-AG-TURN-034                                                                                                                                                                          | GUARD  | AG-TURN                        | src-tauri/src/ag/turn_control.rs  | 85  |
| BR-AG-DATA-003-EFFECT-AG-AG-TURN-036, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-036                                                                                                                                                                         | EFFECT | AG-TURN                        | src-tauri/src/ag/turn_control.rs  | 475 |

#### TERM-AG-020

| codes                                                                                                                                                                                                                                                   | type | chain            | 文件                                 | 行号   |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---- | ---------------- | ---------------------------------- | ---- |
| BR-AG-DATA-001-DATA-AG-AG-TURN-011, BR-AG-DATA-002-DATA-AG-AG-TURN-011, BR-AG-DATA-004-DATA-AG-AG-TURN-011, BR-AG-DATA-005-DATA-AG-AG-TURN-011, BR-AG-DATA-008-DATA-AG-AG-TURN-011                                                                      | DATA | AG-TURN          | src-tauri/src/ag/prompt_runtime.rs | 283  |
| BR-AG-DATA-002-DATA-AG-AG-TURN-031, BR-AG-DATA-008-DATA-AG-AG-TURN-031, BR-SYS-DATA-008-DATA-AG-AG-TURN-031                                                                                                                                             | DATA | AG-TURN          | src-tauri/src/ag/turn_prompt.rs    | 95   |
| BR-AG-OBS-001-API-AG-AG-TURN-018, BR-AG-STATE-001-API-AG-AG-TURN-018, BR-AG-STATE-004-API-AG-AG-TURN-018, BR-AG-DATA-008-API-AG-AG-TURN-018, BR-ED-DATA-008-API-AG-AG-TURN-018, BR-SYS-STATE-007-API-AG-AG-TURN-018, BR-SYS-DATA-008-API-AG-AG-TURN-018 | API  | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_runtime.rs   | 753  |
| BR-AG-STATE-002-RB-AG-AG-TOOL-018                                                                                                                                                                                                                       | RB   | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_tool_loop.rs | 2677 |

#### TERM-AG-021

| codes                                                                                                                                                                                                                                                   | type   | chain            | 文件                               | 行号   |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ---------------- | -------------------------------- | ---- |
| BR-AG-OBS-001-EFFECT-AG-AG-TURN-035, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-035, BR-AG-STATE-001-EFFECT-AG-AG-TURN-035                                                                                                                                      | EFFECT | AG-TURN          | src/services/agService.ts        | 2178 |
| BR-AG-OBS-001-DATA-AG-AG-TOOL-043, BR-AG-DATA-008-DATA-AG-AG-TOOL-043, BR-SYS-DATA-008-DATA-AG-AG-TOOL-043                                                                                                                                              | DATA   | AG-TURN, AG-TOOL | src-tauri/src/ag/tool_exec.rs    | 173  |
| BR-AG-OBS-001-API-AG-AG-TURN-018, BR-AG-STATE-001-API-AG-AG-TURN-018, BR-AG-STATE-004-API-AG-AG-TURN-018, BR-AG-DATA-008-API-AG-AG-TURN-018, BR-ED-DATA-008-API-AG-AG-TURN-018, BR-SYS-STATE-007-API-AG-AG-TURN-018, BR-SYS-DATA-008-API-AG-AG-TURN-018 | API    | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_runtime.rs | 753  |
| BR-AG-OBS-001-DATA-AG-AG-TURN-043, BR-AG-STATE-001-DATA-AG-AG-TURN-043, BR-AG-STATE-002-DATA-AG-AG-TURN-043, BR-AG-PERSIST-001-DATA-AG-AG-TURN-043, BR-SYS-STATE-007-DATA-AG-AG-TURN-043                                                                | DATA   | AG-TURN, AG-TOOL | src-tauri/src/ipc/mod.rs         | 127  |
| BR-SYS-DATA-002-DATA-SYS-DATA-MIGRATE-001, BR-SYS-DATA-008-DATA-SYS-DATA-MIGRATE-001                                                                                                                                                                    | DATA   | DATA-MIGRATE     | src-tauri/src/types/mod.rs       | 9    |

#### TERM-AG-022

| codes                                                                                                                                                                                              | type   | chain            | 文件                                 | 行号   |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ---------------- | ---------------------------------- | ---- |
| BR-AG-OBS-001-EFFECT-AG-AG-TURN-042, BR-AG-STATE-001-EFFECT-AG-AG-TURN-042, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-042, BR-SYS-STATE-008-EFFECT-AG-AG-TURN-042                                         | EFFECT | AG-TURN          | src/store/agStore.ts               | 560  |
| BR-AG-PERSIST-001-GUARD-AG-AG-TURN-048, BR-SYS-STATE-007-GUARD-AG-AG-TURN-048                                                                                                                      | GUARD  | AG-TURN          | src-tauri/src/ag/history.rs        | 366  |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-046                                                                                                                                                            | EFFECT | AG-TURN          | src-tauri/src/ag/history.rs        | 398  |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-047, BR-SYS-STATE-007-QUERY-AG-AG-TURN-047                                                                                                                      | QUERY  | AG-TURN          | src-tauri/src/ag/history_routes.rs | 131  |
| BR-SYS-STATE-008-QUERY-AG-AG-TASK-004, BR-AG-PERSIST-001-QUERY-AG-AG-TASK-004                                                                                                                      | QUERY  | AG-TURN          | src-tauri/src/ag/task.rs           | 1137 |
| BR-AG-STATE-001-API-AG-AG-TURN-019, BR-AG-PERSIST-001-API-AG-AG-TURN-019, BR-SYS-STATE-007-API-AG-AG-TURN-019                                                                                      | API    | AG-TURN          | src-tauri/src/ag/turn_control.rs   | 620  |
| BR-AG-OBS-001-EFFECT-AG-AG-TURN-044, BR-AG-STATE-001-EFFECT-AG-AG-TURN-044, BR-AG-STATE-002-EFFECT-AG-AG-TURN-044, BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-044, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-044 | EFFECT | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_events.rs    | 476  |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-045, BR-SYS-STATE-007-QUERY-AG-AG-TURN-045                                                                                                                      | QUERY  | AG-TURN          | src-tauri/src/ag/turn_events.rs    | 933  |
| BR-AG-OBS-001-DATA-AG-AG-TURN-043, BR-AG-STATE-001-DATA-AG-AG-TURN-043, BR-AG-STATE-002-DATA-AG-AG-TURN-043, BR-AG-PERSIST-001-DATA-AG-AG-TURN-043, BR-SYS-STATE-007-DATA-AG-AG-TURN-043           | DATA   | AG-TURN, AG-TOOL | src-tauri/src/ipc/mod.rs           | 127  |

#### TERM-AG-023

| codes                                                                                                                                                                                                                                                   | type   | chain            | 文件                                      | 行号  |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ---------------- | --------------------------------------- | --- |
| BR-AG-OBS-001-RB-AG-AG-TURN-050, BR-AG-UI-001-RB-AG-AG-TURN-050                                                                                                                                                                                         | RB     | AG-TURN          | src/components/ag/ProjectReportCard.tsx | 5   |
| BR-AG-OBS-001-DATA-AG-AG-TOOL-043, BR-AG-DATA-008-DATA-AG-AG-TOOL-043, BR-SYS-DATA-008-DATA-AG-AG-TOOL-043                                                                                                                                              | DATA   | AG-TURN, AG-TOOL | src-tauri/src/ag/tool_exec.rs           | 173 |
| BR-AG-OBS-001-EFFECT-AG-AG-TURN-044, BR-AG-STATE-001-EFFECT-AG-AG-TURN-044, BR-AG-STATE-002-EFFECT-AG-AG-TURN-044, BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-044, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-044                                                      | EFFECT | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_events.rs         | 476 |
| BR-AG-OBS-001-QUERY-AG-AG-TURN-048, BR-AG-PERSIST-001-QUERY-AG-AG-TURN-048, BR-SYS-DATA-008-QUERY-AG-AG-TURN-048                                                                                                                                        | QUERY  | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_events.rs         | 988 |
| BR-AG-OBS-001-API-AG-AG-TURN-018, BR-AG-STATE-001-API-AG-AG-TURN-018, BR-AG-STATE-004-API-AG-AG-TURN-018, BR-AG-DATA-008-API-AG-AG-TURN-018, BR-ED-DATA-008-API-AG-AG-TURN-018, BR-SYS-STATE-007-API-AG-AG-TURN-018, BR-SYS-DATA-008-API-AG-AG-TURN-018 | API    | AG-TURN, AG-TOOL | src-tauri/src/ag/turn_runtime.rs        | 753 |
| BR-AG-OBS-001-EFFECT-AG-AG-TURN-072                                                                                                                                                                                                                     | EFFECT | AG-TURN          | src-tauri/src/ag/turn_tool_loop.rs      | 901 |
| BR-SYS-DATA-002-DATA-SYS-DATA-MIGRATE-001, BR-SYS-DATA-008-DATA-SYS-DATA-MIGRATE-001                                                                                                                                                                    | DATA   | DATA-MIGRATE     | src-tauri/src/types/mod.rs              | 9   |

#### TERM-DE-002

| codes                                    | type  | chain            | 文件                             | 行号  |
| ---------------------------------------- | ----- | ---------------- | ------------------------------ | --- |
| BR-AG-DATA-003-GUARD-AG-AG-TARGETSET-003 | GUARD | AG-TOOL, DE-DIFF | src-tauri/src/ag/target_set.rs | 272 |

#### TERM-DE-003

| codes                                                                                                                  | type | chain                          | 文件                                | 行号   |
| ---------------------------------------------------------------------------------------------------------------------- | ---- | ------------------------------ | --------------------------------- | ---- |
| BR-DE-STATE-001-RB-DE-DE-DIFF-002                                                                                      | RB   | DE-DIFF                        | src/store/deStore.ts              | 320  |
| BR-WS-STATE-003-API-WS-WS-OPERATION-004, BR-WS-DATA-003-API-WS-WS-OPERATION-004, BR-WS-OBS-002-API-WS-WS-OPERATION-004 | API  | WS-OPERATION, DE-DIFF, ED-EDIT | src-tauri/src/workspace/routes.rs | 1861 |

#### TERM-DE-004

| codes                                                              | type | chain   | 文件                        | 行号 |
| ------------------------------------------------------------------ | ---- | ------- | ------------------------- | -- |
| BR-DE-DATA-001-IO-DE-DE-DIFF-006, BR-DE-DATA-002-IO-DE-DE-DIFF-006 | IO   | DE-DIFF | src-tauri/src/de/store.rs | 66 |

#### TERM-DE-005

| codes                                                                                                 | type | chain            | 文件                             | 行号  |
| ----------------------------------------------------------------------------------------------------- | ---- | ---------------- | ------------------------------ | --- |
| BR-DE-DATA-002-RB-DE-DE-DIFF-003, BR-DE-DATA-001-RB-DE-DE-DIFF-003, BR-DE-STATE-001-RB-DE-DE-DIFF-003 | RB   | DE-DIFF          | src/components/de/DiffCard.tsx | 241 |
| BR-DE-DATA-001-RB-DE-DE-DIFF-007, BR-DE-STATE-001-RB-DE-DE-DIFF-007                                   | RB   | DE-DIFF, ED-EDIT | src/store/deStore.ts           | 222 |
| BR-DE-DATA-001-IO-DE-DE-DIFF-006, BR-DE-DATA-002-IO-DE-DE-DIFF-006                                    | IO   | DE-DIFF          | src-tauri/src/de/store.rs      | 66  |

#### TERM-DOC-003

| codes                                                                    | type  | chain   | 文件                         | 行号  |
| ------------------------------------------------------------------------ | ----- | ------- | -------------------------- | --- |
| BR-AG-DATA-001-QUERY-AG-AG-TOOL-011, BR-ED-DATA-003-QUERY-AG-AG-TOOL-011 | QUERY | AG-TOOL | src-tauri/src/ag/blocks.rs | 307 |

#### TERM-DOC-005

| codes                                | type   | chain            | 文件                                               | 行号  |
| ------------------------------------ | ------ | ---------------- | ------------------------------------------------ | --- |
| BR-ED-DATA-007-DATA-ED-ED-EDIT-018   | DATA   | ED-EDIT          | src/components/ed/extensions/TableCellShading.ts | 37  |
| BR-ED-DATA-007-RB-ED-ED-EDIT-018     | RB     | ED-EDIT, DE-DIFF | src/services/edTableOps.ts                       | 407 |
| BR-ED-DATA-007-EFFECT-ED-ED-EDIT-014 | EFFECT | ED-EDIT, DE-DIFF | src/services/edTableOps.ts                       | 438 |

#### TERM-ED-001

| codes                                                                     | type  | chain   | 文件                         | 行号  |
| ------------------------------------------------------------------------- | ----- | ------- | -------------------------- | --- |
| BR-AG-DATA-001-QUERY-AG-AG-TURN-010, BR-DOC-DATA-003-QUERY-AG-AG-TURN-010 | QUERY | AG-TURN | src-tauri/src/ag/blocks.rs | 533 |

#### TERM-ED-003

| codes                              | type | chain    | 文件                                                      | 行号  |
| ---------------------------------- | ---- | -------- | ------------------------------------------------------- | --- |
| BR-ED-STATE-002-RB-ED-ED-DECOR-001 | RB   | ED-DECOR | src/components/ed/extensions/DiffDecorationExtension.ts | 155 |

#### TERM-ED-004

| codes                                                                     | type   | chain   | 文件                                     | 行号   |
| ------------------------------------------------------------------------- | ------ | ------- | -------------------------------------- | ---- |
| BR-ED-STATE-001-EFFECT-ED-ED-EDIT-012                                     | EFFECT | ED-EDIT | src/services/edLocate.ts               | 345  |
| BR-ED-DATA-005-RB-ED-ED-EDIT-008                                          | RB     | ED-EDIT | src/services/edStyleOps.ts             | 243  |
| BR-SYS-DATA-006-GUARD-AG-AG-TOOL-021, BR-ED-DATA-005-GUARD-AG-AG-TOOL-021 | GUARD  | AG-TOOL | src-tauri/src/ag/bdocx_capabilities.rs | 1566 |
| BR-ED-DATA-005-QUERY-ED-ED-EDIT-009                                       | QUERY  | ED-EDIT | src-tauri/src/ag/blocks.rs             | 1010 |

#### TERM-ED-005

| codes                               | type  | chain   | 文件                       | 行号  |
| ----------------------------------- | ----- | ------- | ------------------------ | --- |
| BR-ED-DATA-004-QUERY-ED-ED-EDIT-010 | QUERY | ED-EDIT | src/services/edLocate.ts | 231 |
| BR-ED-DATA-004-QUERY-ED-ED-EDIT-007 | QUERY | ED-EDIT | src/services/edLocate.ts | 414 |

#### TERM-ED-006

| codes                                                                     | type  | chain   | 文件                                     | 行号   |
| ------------------------------------------------------------------------- | ----- | ------- | -------------------------------------- | ---- |
| BR-SYS-DATA-006-GUARD-AG-AG-TOOL-021, BR-ED-DATA-005-GUARD-AG-AG-TOOL-021 | GUARD | AG-TOOL | src-tauri/src/ag/bdocx_capabilities.rs | 1566 |

#### TERM-ED-008

| codes                                                                                                                  | type   | chain                          | 文件                                     | 行号   |
| ---------------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------ | -------------------------------------- | ---- |
| BR-DE-DATA-002-RB-DE-DE-DIFF-003, BR-DE-DATA-001-RB-DE-DE-DIFF-003, BR-DE-STATE-001-RB-DE-DE-DIFF-003                  | RB     | DE-DIFF                        | src/components/de/DiffCard.tsx         | 241  |
| BR-AG-DATA-001-EFFECT-AG-AG-TURN-005                                                                                   | EFFECT | AG-TURN                        | src/services/agService.ts              | 648  |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-006, BR-AG-DATA-001-EFFECT-AG-AG-TURN-006                                           | EFFECT | AG-TURN, DE-DIFF, ED-EDIT      | src/services/agService.ts              | 1535 |
| BR-DE-DATA-002-EFFECT-DE-DE-DIFF-009                                                                                   | EFFECT | DE-DIFF                        | src/services/deService.ts              | 165  |
| BR-SYS-DATA-006-RB-ED-ED-EDIT-027, BR-ED-DATA-003-RB-ED-ED-EDIT-027                                                    | RB     | ED-EDIT                        | src/services/edImageOps.ts             | 256  |
| BR-DE-DATA-002-RB-ED-DE-DIFF-028                                                                                       | RB     | DE-DIFF                        | src/services/edImageOps.ts             | 493  |
| BR-SYS-DATA-006-DATA-AG-AG-TOOL-002                                                                                    | DATA   | AG-TOOL                        | src-tauri/src/ag/bdocx_capabilities.rs | 897  |
| BR-SYS-DATA-006-GUARD-AG-AG-TOOL-022, BR-ED-DATA-003-GUARD-AG-AG-TOOL-022                                              | GUARD  | AG-TOOL                        | src-tauri/src/ag/bdocx_capabilities.rs | 1918 |
| BR-SYS-DATA-006-GUARD-AG-AG-TOOL-004, BR-ED-DATA-001-GUARD-AG-AG-TOOL-004, BR-ED-DATA-003-GUARD-AG-AG-TOOL-004         | GUARD  | AG-TOOL                        | src-tauri/src/ag/edit_image_runtime.rs | 8    |
| BR-SYS-STATE-008-EFFECT-AG-AG-TURN-032, BR-AG-DATA-001-EFFECT-AG-AG-TURN-032                                           | EFFECT | AG-TURN, DE-DIFF, ED-EDIT      | src-tauri/src/ag/task.rs               | 110  |
| BR-SYS-STATE-008-GUARD-AG-AG-TURN-005, BR-AG-DATA-001-GUARD-AG-AG-TURN-005                                             | GUARD  | AG-TURN, DE-DIFF, ED-EDIT      | src-tauri/src/ag/task.rs               | 169  |
| BR-SYS-STATE-008-GUARD-AG-AG-TURN-006, BR-AG-DATA-001-GUARD-AG-AG-TURN-006                                             | GUARD  | AG-TURN, ED-EDIT               | src-tauri/src/ag/task.rs               | 287  |
| BR-SYS-STATE-008-API-AG-AG-TURN-003, BR-AG-DATA-001-API-AG-AG-TURN-003                                                 | API    | AG-TURN, DE-DIFF, ED-EDIT      | src-tauri/src/ag/turn_control.rs       | 654  |
| BR-DE-DATA-002-EFFECT-WS-DE-DIFF-004                                                                                   | EFFECT | DE-DIFF                        | src-tauri/src/workspace/diff_apply.rs  | 232  |
| BR-WS-STATE-003-API-WS-WS-OPERATION-004, BR-WS-DATA-003-API-WS-WS-OPERATION-004, BR-WS-OBS-002-API-WS-WS-OPERATION-004 | API    | WS-OPERATION, DE-DIFF, ED-EDIT | src-tauri/src/workspace/routes.rs      | 1861 |

#### TERM-REF-003

| codes                                                                                                                         | type | chain            | 文件                               | 行号  |
| ----------------------------------------------------------------------------------------------------------------------------- | ---- | ---------------- | -------------------------------- | --- |
| BR-REF-DATA-004-DATA-AG-REF-INJECT-033                                                                                        | DATA | REF-INJECT       | src/components/ag/Composer.tsx   | 40  |
| BR-REF-DATA-004-RB-REF-REF-INJECT-001                                                                                         | RB   | REF-INJECT       | src/services/referencePayload.ts | 439 |
| BR-AG-DATA-003-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-007-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-008-DATA-AG-AG-TARGETSET-001 | DATA | AG-TURN, AG-TOOL | src-tauri/src/ag/target_set.rs   | 32  |

#### TERM-REF-005

| codes                                                                                                                       | type | chain      | 文件                                 | 行号  |
| --------------------------------------------------------------------------------------------------------------------------- | ---- | ---------- | ---------------------------------- | --- |
| BR-REF-DATA-003-DATA-REF-REF-INJECT-002, BR-REF-VERIFY-001-DATA-REF-REF-INJECT-002, BR-SYS-DATA-007-DATA-REF-REF-INJECT-002 | DATA | REF-INJECT | src-tauri/src/reference/context.rs | 102 |

#### TERM-WS-001

| codes                                                                                                                                                 | type  | chain       | 文件                                | 行号  |
| ----------------------------------------------------------------------------------------------------------------------------------------------------- | ----- | ----------- | --------------------------------- | --- |
| BR-AG-STATE-002-QUERY-AG-AG-TOOL-012, BR-AG-DATA-001-QUERY-AG-AG-TOOL-012, BR-WS-DATA-001-QUERY-AG-AG-TOOL-012                                        | QUERY | AG-TOOL     | src-tauri/src/ag/tool_exec.rs     | 483 |
| BR-AG-STATE-002-GUARD-AG-AG-TOOL-035, BR-SYS-STATE-007-GUARD-AG-AG-TOOL-035, BR-AG-DATA-003-GUARD-AG-AG-TOOL-035, BR-WS-DATA-001-GUARD-AG-AG-TOOL-035 | GUARD | AG-TOOL     | src-tauri/src/ag/tool_registry.rs | 456 |
| BR-DOC-DATA-002-GUARD-DOC-DOC-CONVERT-011, BR-WS-DATA-001-GUARD-DOC-DOC-CONVERT-011                                                                   | GUARD | DOC-CONVERT | src-tauri/src/doc/image_media.rs  | 334 |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-015, BR-WS-DATA-001-IO-DOC-DOC-CONVERT-015                                                                         | IO    | DOC-CONVERT | src-tauri/src/doc/image_media.rs  | 568 |

#### TERM-WS-003

| codes                                                                                                                                                                                                                                                | type   | chain                          | 文件                                | 行号  |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------ | --------------------------------- | --- |
| BR-AG-DATA-003-EFFECT-AG-AG-TURN-037                                                                                                                                                                                                                 | EFFECT | AG-TURN, WS-OPERATION          | src-tauri/src/ag/task_runtime.rs  | 16  |
| BR-AG-STATE-002-API-AG-WS-OPERATION-005, BR-AG-DATA-003-API-AG-WS-OPERATION-005, BR-SYS-STATE-008-API-AG-WS-OPERATION-005, BR-WS-STATE-003-API-AG-WS-OPERATION-005, BR-WS-PERSIST-003-API-AG-WS-OPERATION-005, BR-WS-OBS-002-API-AG-WS-OPERATION-005 | API    | AG-TURN, AG-TOOL, WS-OPERATION | src-tauri/src/ag/task_runtime.rs  | 197 |
| BR-SYS-STATE-008-GUARD-AG-AG-TOOL-040, BR-WS-STATE-003-GUARD-AG-AG-TOOL-040                                                                                                                                                                          | GUARD  | AG-TOOL, WS-OPERATION          | src-tauri/src/ag/tool_registry.rs | 415 |

#### TERM-WS-004

| codes                                                                                                                                                                         | type   | chain                 | 文件                                   | 行号   |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | --------------------- | ------------------------------------ | ---- |
| BR-AG-STATE-001-API-AG-AG-TURN-070, BR-AG-PERSIST-001-API-AG-AG-TURN-070                                                                                                      | API    | AG-TURN, WS-OPERATION | src-tauri/src/ag/task_runtime.rs     | 725  |
| BR-AG-STATE-001-EFFECT-AG-AG-TURN-069, BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-069                                                                                                | EFFECT | AG-TURN, WS-OPERATION | src-tauri/src/ag/turn_events.rs      | 1245 |
| BR-WS-STATE-003-RB-WS-WS-OPERATION-001, BR-WS-PERSIST-003-RB-WS-WS-OPERATION-001                                                                                              | RB     | WS-OPERATION          | src-tauri/src/workspace/operation.rs | 143  |
| BR-WS-STATE-003-API-WS-WS-OPERATION-001, BR-WS-DATA-003-API-WS-WS-OPERATION-001, BR-WS-OBS-002-API-WS-WS-OPERATION-001                                                        | API    | WS-OPERATION          | src-tauri/src/workspace/operation.rs | 2261 |
| BR-WS-STATE-003-QUERY-WS-WS-OPERATION-006, BR-WS-DATA-003-QUERY-WS-WS-OPERATION-006                                                                                           | QUERY  | WS-OPERATION          | src-tauri/src/workspace/operation.rs | 2291 |
| BR-WS-STATE-003-EFFECT-WS-WS-OPERATION-007, BR-WS-DATA-003-EFFECT-WS-WS-OPERATION-007, BR-WS-OBS-002-EFFECT-WS-WS-OPERATION-007                                               | EFFECT | WS-OPERATION          | src-tauri/src/workspace/operation.rs | 2417 |
| BR-WS-STATE-003-EFFECT-WS-WS-OPERATION-003, BR-WS-DATA-003-EFFECT-WS-WS-OPERATION-003, BR-WS-OBS-002-EFFECT-WS-WS-OPERATION-003, BR-WS-PERSIST-003-EFFECT-WS-WS-OPERATION-003 | EFFECT | WS-OPERATION          | src-tauri/src/workspace/operation.rs | 2943 |
| BR-WS-STATE-003-EFFECT-WS-WS-OPERATION-008, BR-WS-PERSIST-003-EFFECT-WS-WS-OPERATION-008                                                                                      | EFFECT | WS-OPERATION, AG-TURN | src-tauri/src/workspace/operation.rs | 2966 |
| BR-WS-STATE-003-API-WS-WS-OPERATION-002, BR-WS-DATA-003-API-WS-WS-OPERATION-002, BR-WS-OBS-002-API-WS-WS-OPERATION-002                                                        | API    | WS-OPERATION          | src-tauri/src/workspace/routes.rs    | 1467 |

#### TERM-WS-006

| codes                                                                                                                                                                         | type   | chain              | 文件                                   | 行号   |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------------ | ------------------------------------ | ---- |
| BR-WS-DATA-003-EFFECT-WS-WS-TIMELINE-003, BR-WS-PERSIST-003-EFFECT-WS-WS-TIMELINE-003                                                                                         | EFFECT | WS-TIMELINE        | src-tauri/src/workspace/operation.rs | 349  |
| BR-WS-STATE-009-EFFECT-WS-WS-FS-009                                                                                                                                           | EFFECT | WS-FS, WS-TIMELINE | src-tauri/src/workspace/operation.rs | 410  |
| BR-WS-STATE-009-EFFECT-WS-WS-FS-010                                                                                                                                           | EFFECT | WS-FS, WS-TIMELINE | src-tauri/src/workspace/operation.rs | 470  |
| BR-WS-STATE-003-EFFECT-WS-WS-OPERATION-003, BR-WS-DATA-003-EFFECT-WS-WS-OPERATION-003, BR-WS-OBS-002-EFFECT-WS-WS-OPERATION-003, BR-WS-PERSIST-003-EFFECT-WS-WS-OPERATION-003 | EFFECT | WS-OPERATION       | src-tauri/src/workspace/operation.rs | 2943 |

#### TERM-WS-007

| codes                                                                                                                  | type | chain                          | 文件                                | 行号   |
| ---------------------------------------------------------------------------------------------------------------------- | ---- | ------------------------------ | --------------------------------- | ---- |
| BR-WS-STATE-003-API-WS-WS-OPERATION-004, BR-WS-DATA-003-API-WS-WS-OPERATION-004, BR-WS-OBS-002-API-WS-WS-OPERATION-004 | API  | WS-OPERATION, DE-DIFF, ED-EDIT | src-tauri/src/workspace/routes.rs | 1861 |

### 测试能力索引

动态发现：前端测试文件 26；Rust 内联测试文件 85；Rust 集成测试文件 2；测试用例约 1177。

| module | risk | test assets | test cases | layers | frontend baseline → target |
|---|---|---:|---:|---|---:|
| CORE | high | 6 | 113 | governance:1, service:2, unit:3 | 1/1 → 1 |
| SYS | critical | 43 | 597 | component:3, integration:2, service:5, state:1, unit:32 | 3/3 → 3 |
| WS | critical | 23 | 345 | component:1, integration:1, service:4, unit:17 | 2/2 → 2 |
| ED | critical | 25 | 550 | component:4, service:7, state:1, unit:13 | 11/11 → 11 |
| DE | critical | 22 | 365 | component:3, integration:1, service:3, unit:15 | 2/2 → 2 |
| AG | critical | 41 | 495 | component:1, integration:1, service:1, state:1, unit:37 | 3/3 → 3 |
| CBT | critical | 4 | 12 | unit:4 | 0/0 → 0 |
| DOC | critical | 16 | 97 | service:1, unit:15 | 1/1 → 1 |
| PRV | high | 4 | 24 | component:1, service:1, unit:2 | 2/2 → 2 |
| REF | high | 9 | 192 | component:1, service:2, unit:6 | 1/1 → 1 |
| AST | high | 3 | 8 | unit:3 | 0/0 → 0 |
| KB | high | 2 | 5 | unit:2 | 0/0 → 0 |
| TPL | high | 2 | 6 | unit:2 | 0/0 → 0 |
| ACCT | critical | 2 | 5 | unit:2 | 0/0 → 0 |
| ENT | critical | 4 | 62 | unit:4 | 0/0 → 0 |

系统验证登记（自动化证据与人工验证证据分开）：

| validation | phase | modules | automation | status |
|---|---|---|---|---|
| VAL-CORE-EDIT-DIFF-SAVE | G5 | ED, DE, AG, WS, SYS | partial | active |
| VAL-CORE-CLOSE-RECOVERY | G5 | SYS, ED, DE, WS | partial | deferred |

### 产品能力索引

> 来源：`.governance/product-capability-registry.json`；由 `product:generate` 和 `product:audit` 派生。
> 注册表追溯审计通过。

| 变体 | 能力 | 用户旅程 | 格式 | 正式主链 | 自动化 | 系统验证 | 当前结论 |
|---|---|---|---|---|---|---|---|
| VAR-EDIT-MARKDOWN-SAFE-SAVE | 编辑 Markdown 并安全保存 | 完成一次文档协作 | markdown | ED-EDIT | passed | pending | 自动化已通过，待系统验证 |
| VAR-EDIT-HTML-ROUNDTRIP | 编辑 HTML 正文并保持块锚点 | 完成一次文档协作 | html | ED-EDIT | partial | pending | 自动化部分覆盖 |
| VAR-EDIT-TEXT-LINE-BLOCKS | 编辑纯文本并按行定位内容块 | 完成一次文档协作 | text | ED-EDIT | partial | pending | 自动化部分覆盖 |
| VAR-EDIT-BDOCX-SUPPORTED-SUBSET | 编辑 BDocx 支持子集并保存重开 | 完成一次文档协作 | bdocx | DOC-CONVERT | passed | pending | 自动化已通过，待系统验证 |
| VAR-OPEN-NATIVE-DOCX-PREVIEW | 打开原生 DOCX 并创建可编辑草稿 | 完成一次文档协作 | docx | — | partial | pending | 自动化部分覆盖 |
| VAR-AGENT-TEXTUAL-DIFF | 代理修改文本、Markdown 或 HTML 内容块 | 完成一次文档协作 | text / markdown / html | AG-TURN | partial | blocked | 自动化部分覆盖 |
| VAR-AGENT-BDOCX-STRUCTURED-EDIT | 代理以结构化操作修改 BDocx | 完成一次文档协作 | bdocx | AG-TURN | passed | blocked | 自动化已通过，待系统验证 |
| VAR-WS-CREATE-RESOURCE | 新建文件或文件夹 | 完成一次文档协作 | markdown / text / html / bdocx / folder | WS-FS | partial | pending | 自动化部分覆盖 |
| VAR-WS-DELETE-FOLDER | 文件夹删除两级语义 | 完成一次文档协作 | folder | WS-FS | partial | pending | 自动化部分覆盖 |
| VAR-WS-SAVE-TIMELINE | 保存成功记时间轴节点 | 完成一次文档协作 | markdown / text / html / bdocx | WS-TIMELINE | partial | pending | 自动化部分覆盖 |
| VAR-AG-MODEL-CONFIG | 模型配置体系（来源分层+能力偏好+预设清单+BYOK管理） | 完成一次文档协作 | markdown / text / html / bdocx | — | partial | pending | 自动化部分覆盖 |
| VAR-PRV-CACHE-LOCAL-OPEN | 预览缓存落盘与本地打开 | 完成一次文档协作 | docx / xlsx / pptx / pdf | PRV-VIEW | partial | pending | 自动化部分覆盖 |
| VAR-PRV-TOOLBAR-ADAPTIVE | 预览模式工具栏自适应 | 完成一次文档协作 | docx / xlsx / pptx | — | partial | pending | 自动化部分覆盖 |
| VAR-CTX-MENU-UNIFIED | 统一右键菜单 | 完成一次文档协作 | markdown / text / html / bdocx / folder | — | partial | pending | 自动化部分覆盖 |
| VAR-ED-SELECTION-PERSIST | 选区失焦持久化 | 完成一次文档协作 | markdown / text / html / bdocx | — | partial | pending | 自动化部分覆盖 |
| VAR-ED-SELECTION-DRAG | 选区拖动不错位 | 完成一次文档协作 | markdown / text / html / bdocx | — | partial | pending | 自动化部分覆盖 |
| VAR-EDIT-TOOLBAR-CONTEXTUAL | 工具栏上下文分组与编辑操作富化 | 完成一次文档协作 | markdown / text / html / bdocx | — | partial | pending | 自动化部分覆盖 |
| VAR-EDIT-FONT-LOCAL-LIBRARY | 本地字体库同步 | 完成一次文档协作 | markdown / html / bdocx | — | partial | pending | 自动化部分覆盖 |
| VAR-EDIT-HIGHLIGHT-BACKGROUND | 高亮收敛为文字背景 | 完成一次文档协作 | markdown / html / bdocx | — | partial | pending | 自动化部分覆盖 |

### triage 汇总

（暂无有效 triage 记录）

### 标注延期汇总

| 符号                | 文件                               | 承接计划                                | 复核日期       | 已解决 |
| ----------------- | -------------------------------- | ----------------------------------- | ---------- | --- |
| App               | src/shell/App.tsx                | 已关闭（ANNOTATION-DEFERRAL-CLOSURE-01） | 2026-08-01 | 是   |
| Welcome           | src/shell/Welcome.tsx            | 已关闭（ANNOTATION-DEFERRAL-CLOSURE-01） | 2026-08-01 | 是   |
| Workbench         | src/shell/Workbench.tsx          | 已关闭（ANNOTATION-DEFERRAL-CLOSURE-01） | 2026-08-01 | 是   |
| useWorkspaceShell | src/store/workspaceShell.ts      | 已关闭（ANNOTATION-DEFERRAL-CLOSURE-01） | 2026-08-01 | 是   |
| openWorkspace     | src/services/workspaceService.ts | 已关闭（ANNOTATION-DEFERRAL-CLOSURE-01） | 2026-08-01 | 是   |
