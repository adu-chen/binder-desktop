---
文档编号：CORE-X-L-05
文档状态：A
负责模块：CORE
文档职责：Binder 项目进度跟踪台账
上游约束：A-CORE-C-P-01_APC.md、A-CORE-X-L-01_总体开发路线图.md、A-CORE-X-L-02_总开发实施计划.md、A-CORE-X-R-03_需求实现对齐审计与补全计划.md、A-CORE-C-R-01_ADUS.md、各模块 A-{M}-X-L-NN 开发计划
直接承接：项目日常进度盘点、模块推进会议、后续 G4/G5/G6 进入判断；不作为代码规则来源
使用边界：本文是手工维护的进度视图，不定义 RULE/CHAIN/MODULE/TERM/CONSTRAINT，不承接 REQ_MAP，代码 @GOV 不得追溯本文；ADUS 仍是规则/链路/代码映射索引的派生真值
变更要求：模块 G 阶段、Plan Item 状态、跨模块阻断、云端依赖或发布门禁状态变化时同步更新；若上游规则/链路变化，应先更新对应 SSOT 文档并刷新 ADUS，再回填本文进度
治理成熟度：draft
---

# Binder 项目进度跟踪台账

## §1 台账口径

### 1.1 快照范围

本台账是 **2026-07-15 当前工作区文档快照** 的进度视图，依据以下来源合成：

| 来源 | 用途 |
|---|---|
| A-CORE-X-L-01 | 阶段 P0-P5、模块顺序、里程碑 |
| A-CORE-X-L-02 | G1-G6 门禁口径、关键路径、P5 阻断 |
| A-CORE-X-R-03 | 2026-07-02 需求实现对齐审计与核心缺口 |
| A-CORE-X-L-03 | 前端 G4 Plan Item 状态 |
| A-CORE-X-L-04 | 状态机全链路修复状态 |
| A-CORE-X-L-06 | 1.0 Experience Preview 发布阻断项与非阻断项 |
| A-CORE-C-R-01_ADUS | 模块、链路、规则、代码块索引 |
| 各模块 A-{M}-X-L-NN | 模块内 Plan Item 进度 |

本文不替代上游计划或设计。若本文与模块计划冲突，以模块计划和对应 L2 设计为准，本文需回填修正。

### 1.2 状态枚举

| 状态 | 含义 |
|---|---|
| done | 对应门禁或 PI 已在上游文档明确标记完成，或已有人类宣布闭合 |
| in_progress | 已进入实现或修复，仍有未闭合项 |
| partial | 有真实实现或代码块，但计划/验证/验收未正式闭合 |
| blocked | 当前阻断影响继续推进或进入下一门禁 |
| planned | 已有计划，尚未开始或未见 G4 完成记录 |
| not_started | 未开始 |
| deferred | 显式延期，不作为当前阶段阻断 |
| needs_review | 需要回读代码/运行验证/人类裁决后才能确认 |

G 阶段口径沿用 APC §5：

| 阶段 | 口径 |
|---|---|
| G1 | 需求闭合 |
| G2 | 技术设计闭合 |
| G3 | 开发计划闭合 |
| G4 | 实现与测试闭合 |
| G5 | 跨模块集成/系统验证闭合 |
| G6 | 发布就绪闭合 |

## §2 总体进度

### 2.1 项目阶段总览

| 阶段 | 范围 | 当前状态 | 证据 | 阻断/下一步 |
|---|---|---|---|---|
| P0 治理就绪 | CORE 治理、APC、ADUS、路线图 | done | A-CORE-X-L-02 §3 P0 | 持续维护台账与审计 |
| P1 底座与契约 | SYS、ACCT、ENT、云接口契约 | partial | SYS/ACCT/ENT 多数 PI 标记 G4；云端仅文档侧 | 真云 AUTH/GW/LIC 未实现，G5 阻断 |
| P2 workspace 与编辑核心 | WS、ED、DOC | in_progress | WS 多数 PI G4；ED 专项多项 G4；DOC 有实现但 PI 未逐项闭合 | DOC 保真/打包/回归验证、ED 真机验证、状态机运行时接线 |
| P3 代理与 Diff Edit | AG、DE、REF | in_progress | AG 骨架 PI-01~06 G4；DE PI-01~03 G4、PI-04 进行中；REF G4 | AG 真实模型、工具执行、流式集成、DE 真实模型产出仍未闭合 |
| P4 上层能力 | CBT、AST、KB、TPL、PRV | in_progress | CBT/AST/KB/TPL 多数 PI G4；PRV 前端/后端预览项已实现但模块计划未闭合 | PRV/DOC 全格式保真、G5 联调、AG 真实供给 |
| P5 商业化闭环 | 云 AUTH/GW/LIC、订阅门控 E2E、发布 | blocked | A-CORE-X-R-03 §5.1、A-CORE-X-L-02 §3 P5 | 云端实现未启动；真实模型调用链未接通 |

### 2.2 G 阶段总览

| 维度 | G1 | G2 | G3 | G4 | G5 | G6 |
|---|---|---|---|---|---|---|
| 客户端 15 模块 | done | done | done | in_progress | blocked | not_started |
| 前端体验/交互 | done | done | done | partial | not_started | not_started |
| Rust 后端/本地能力 | done | done | done | in_progress | blocked | not_started |
| 云端 AUTH/GW/LIC | partial | partial | planned | not_started | blocked | not_started |
| AI 网关/模型调用链 | done（契约） | done（契约） | in_progress | blocked | blocked | not_started |

### 2.3 当前最高优先级

| 优先级 | 项 | 状态 | 原因 |
|---|---|---|---|
| P0 | AG 真实模型调用链解桩 | blocked | 依赖云 GW / CLOUD-INVOKE，A-CORE-X-R-03 G-1/G-2/G-3 |
| P0 | DE 真实 ChangeProposal 接线 | blocked | 依赖 AG 真模型输出；当前 diff 生命周期真实但内容源仍需解桩 |
| P0 | 状态机运行时接线与前端投影 | in_progress | A-CORE-X-L-04 指出 11/12 台状态机曾仅测试调用，需接入 handler + emit + store 投影 |
| P1 | DOC/PRV 全格式转换与保真验证 | partial | DOC/PRV 模块计划未逐项标记 G4 闭合；Excel/PPT/docx/音视频需系统验证 |
| P1 | 云端 AUTH/GW/LIC 独立实现 | blocked | P5/G5 真联调必要前置 |
| P2 | 模块计划状态回填一致性 | in_progress | DOC/PRV/ED 基础计划缺少与实现同步的 PI 完成标记 |

### 2.4 1.0 Experience Preview 发布视图

1.0 EP 是 2026-07-17 人类确认的发布切片：完整功能地图不变，全功能开发不暂停；本文按
`A-CORE-X-L-06_1.0体验版发布范围与验收计划.md` 跟踪上线阻断项。

| 发布阻断项 | 模块 | 当前状态 | 下一步 |
|---|---|---|---|
| workspace 生命周期、文件边界、保存与基础搜索 | WS / SYS | partial | 核对前端入口、文件边界、保存冲突、基础搜索在真实工作流中的连续性 |
| 编辑器核心编辑 | ED / WS | in_progress | 补真机/大文档/脏状态/关闭切换保护验收 |
| DOCX 基础预览与编辑链路 | DOC / PRV / ED | partial | 关闭 LibreOffice/Pandoc 失败态和常见 DOCX 回归；失败不破坏原文件 |
| Agent 对话式编辑 | AG / ENT / GW | in_progress | 从 dev/桩状态转为 AiGateway + DeepSeek 托管模型生产链路 |
| Diff Edit 审阅和落盘 | DE / WS / ED | in_progress | 接真实 Agent 输出，完成 ChangeProposal → PendingDiff → accept/reject → save 验收 |
| AiGateway 托管模型 | CLOUD / GW / ENT | blocked | 云端实现 DeepSeek 官方 API 适配、SubscriptionGate、计量与错误形状 |
| BYOK | GW / ENT / PORTAL | blocked（客户端已实现） | 客户端 BYOK 管理已落地（MODEL-CONFIG-01，ent::byok/gateway）；云 GW 代持、删除生效与 SubscriptionGate 仍阻断 |
| 账号与订阅 | AUTH / LIC / ACCT / ENT | blocked | 完成 AUTH/LIC/PORTAL 最小实现，替换客户端桩 |
| 管理员端最小能力 | PORTAL / AUTH / LIC / GW | not_started | 实现用户、订阅、调用量、错误率、网关健康与人工处理入口 |
| 反馈与脱敏 telemetry | CORE / CLOUD / AG / DE | planned | 记录任务类型、模型档、耗时、Diff 接受率、错误码，不保存调用正文 |
| macOS 交付 | CORE / SYS | not_started | DMG、签名/公证、版本说明、隐私说明、安装冒烟 |

1.0 EP 非阻断项：CBT、AST、KB、TPL、自部署 Binder 预设模型、多模态模型矩阵、复杂 ModelRouter、
深度专业领域适配与企业级组织权限；这些模块继续开发，但不阻断 1.0 EP 上线。

## §3 端侧/云侧/模型调用视图

| 视图 | 当前进度 | 主要完成项 | 主要缺口 |
|---|---|---|---|
| 前端 | partial | FE-00~31 标记 G4；编辑器专项 PI-ED-06~13 多数 G4；Workbench、文件树、编辑器、DiffCard、预览、时间轴、搜索已接入 | 手工真机验收、状态机投影彻底化、AG 真实流式体验接入 |
| Rust 本地后端 | in_progress | SYS/WS/DE/AG 骨架/REF/CBT/AST/KB/TPL/ACCT/ENT 多数 PI G4；workspace.db、状态机、文件/时间轴/diff/本地资产能力存在 | 状态机 handler 真实驱动、DOC/PRV 全格式验证、AG 真实模型集成 |
| 云端 | planned | 云 APC、CLOUD 产品概述、GW 需求文档已建 | AUTH/LIC/PORTAL 模块文档与实现未建；GW 代码未实现 |
| AI 网关/模型调用链 | blocked | 客户端门禁/网关契约、ENT 客户端桩、AG 门禁前置已建 | PI-AG-11 依赖 GW；无真实 provider 流式输出；DE 真实修改建议未接线 |

## §4 模块总表

| 模块 | G1 | G2 | G3 | G4 当前状态 | G5/G6 | 当前结论 |
|---|---|---|---|---|---|---|
| CORE | done | done | done | in_progress | not_started | 治理和计划体系已成型；本台账补齐进度入口 |
| SYS | done | done | done | in_progress | blocked | 底座 PI-01~04 G4；状态投影/运行时广播仍需全链路闭合 |
| WS | done | done | done | partial | blocked | 文件、保存、diff 落盘、时间轴、控制态多数 G4；G5 需联调 |
| ED | done | done | done | in_progress | blocked | 编辑器专项多数 G4；基础计划完成状态需回填，真机/大文档验证待做 |
| DOC | done | done | done | partial | blocked | 转换代码与链路存在；三项 PI 未在计划中正式标记 G4 闭合 |
| DE | done | done | done | in_progress | blocked | diff 生命周期 G4；DiffCard 进行中；真实模型内容源阻断 |
| AG | done | done | done | in_progress | blocked | PI-01~06 骨架 G4；PI-07~11 集成阶段未闭合，模型调用阻断 |
| REF | done | done | done | done | in_progress | 引用解析与注入 G4；G5 需与 AG/ED/KB/AST 联调 |
| CBT | done | done | done | done | in_progress | 构建侧 G4；G5 需与 AG/DE/WS 真链路联调 |
| AST | done | done | done | done | in_progress | 记忆存储/写入/供给 G4；G5 需真实 AG 回合验证 |
| KB | done | done | done | done | in_progress | 本地知识库存储/检索 G4；G5 需本地化网络核验 |
| TPL | done | done | done | done | in_progress | 模板与工作流执行机 G4；G5 需与 AG/CBT 联调 |
| PRV | done | done | done | partial | blocked | 前端预览/导出项已实现；模块 PI 未正式闭合，全格式保真待验 |
| ACCT | done | done | done | partial | blocked | 客户端会话和令牌 G4；真登录依赖云 AUTH |
| ENT | done | done | done | partial | blocked | 客户端门禁/BYOK/用量桩 G4；真裁决/计量依赖云 GW/LIC |
| CLOUD | partial | partial | planned | not_started | blocked | 云端文档起步；实现未启动，阻断 P5/G5 |

## §5 模块明细台账

### 5.1 CORE

| 项 | 当前状态 | 证据 | 下一步 |
|---|---|---|---|
| APC/ADU/ADP/ADUS 治理入口 | done | governance.config.json 指向 APC/ADUS；A-CORE-X-L-02 P0 done | 持续审计 |
| 总体路线图 | done | A-CORE-X-L-01 | 重大范围变更时更新 |
| 总实施计划 | done | A-CORE-X-L-02 | G5 NFR 阈值标定后回填 |
| 前端开发计划 | done | A-CORE-X-L-03 FE-00~31 G4 | 真机/交互验收补充 |
| 状态机全链路修复计划 | in_progress | A-CORE-X-L-04 | Phase 2/3 逐机推进 |
| 进度跟踪台账 | done | 本文 | 每次 PI/G 阶段变化时更新 |

### 5.2 SYS

| Plan Item | 当前状态 | 证据 | 阻断/下一步 |
|---|---|---|---|
| PI-SYS-01 IPC 信封与统一错误形状 | done | A-SYS-X-L-01 标记 G4 | 保持错误码总表一致 |
| PI-SYS-02 跨模块共享数据结构 | done | A-SYS-X-L-01 标记 G4 | 变更时同步前后端类型 |
| PI-SYS-03 workspace.db schema 与迁移 | done | A-SYS-X-L-01 标记 G4 | G5 迁移回归 |
| PI-SYS-04 会话与订阅门禁状态机 | done | A-SYS-X-L-01 标记 G4 | 真云门禁联调依赖 ENT/GW |
| PI-SYS-05 前端交互外壳与状态投影 | in_progress | A-SYS-X-L-01 未标 G4；A-CORE-X-L-04 指出 X-FORBID-007 投影缺口 | 状态机广播 + store 投影闭合 |

### 5.3 WS

| Plan Item | 当前状态 | 证据 | 阻断/下一步 |
|---|---|---|---|
| PI-WS-01 workspace 生命周期与文件 I/O 边界 | partial | A-WS-X-L-01 标记路径边界 G4，生命周期 CRUD 曾待 db | 结合 PI-WS-05 与前端实际命令做闭合复核 |
| PI-WS-02 save 保存机与外部变更冲突保护 | done | A-WS-X-L-01 标记 G4 | G5 与 ED 冲突条联调 |
| PI-WS-03 pending_diffs 持久化与裁决后落盘 | done | A-WS-X-L-01 标记 G4 | 与 DE accepted 唯一路径联调 |
| PI-WS-04 时间轴存储与还原 | done | A-WS-X-L-01 标记 G4 | G5 大文档/异常恢复验证 |
| PI-WS-05 控制态持久化 | done | A-WS-X-L-01 标记 G4 | 迁移回归 |
| WS 专项（08-05~07）：WS-NEW-BUTTON-01 / WS-DIR-DELETE-01 / WS-SAVE-TIMELINE-01 | partial | 提交 4e01e7b/3f730c7/3933092；A-WS-I-P-01 v1.8~1.9、A-WS-M-T-01 v1.8~1.9；WorkspaceUI/workspaceRuntime 测试通过 | G5 联调；模块计划 PI 状态回填 |

### 5.4 ED

| Plan Item | 当前状态 | 证据 | 阻断/下一步 |
|---|---|---|---|
| PI-ED-01 编辑器运行时与会话 | partial | A-ED-X-L-01 未标 G4；A-CORE-X-L-03/专项计划有前端实现 | 回填基础 PI 完成状态，补 SM-ED-01 运行时/投影核验 |
| PI-ED-02 block 模型与稳定 blockId | partial | A-ED-X-L-02 PI-ED-09/10 已完成；A-CORE-X-R-03 G-4 已执行 | 跨格式/跨会话锚定边界继续登记 |
| PI-ED-03 md/docx 统一编辑模型 | partial | A-ED-X-L-01 未标 G4；DOC 仍 partial | 依赖 DOC 转换验证 |
| PI-ED-04 分页与图文媒体 | done | A-ED-X-L-02 PI-ED-06/08、A-CORE-X-L-03 PI-FE-31 | 视觉/IME/大文档真机验收 |
| PI-ED-05 编辑器装饰层接口 | partial | DE/AG/ED 多处实现；A-CORE-X-L-04 仍要求投影修复 | 与 DE/AG 联调 |
| PI-ED-06~13 编辑器专项优化 | done | A-ED-X-L-02 v1.3 标记计划全部 PI 收口 | 真机验收、AG 解桩后样式 diff 数据填充 |
| ED 专项（08-05~07）：工具栏上下文分组/本地字体库/高亮收敛/txt 工具栏/选区持久化与拖动 | partial | A-ED-M-T-01 v1.56~v1.63、A-ED-X-L-02 v1.28；EditorControls/fontService/SelectionPersist 测试通过 | 真机验收；与 DE/AG 联调 |

### 5.5 DOC

| Plan Item | 当前状态 | 证据 | 阻断/下一步 |
|---|---|---|---|
| PI-DOC-01 编辑链转换与 docx 开/存机 | partial | ADUS 有 DOC-CONVERT 代码块；A-DOC-X-L-01 未标 G4 | Pandoc/Lua/打包、blockId 往返实测回填 |
| PI-DOC-02 样式子集三级执行 | partial | ADUS 有 style_subset 代码块；A-DOC-X-L-01 未标 G4 | 必需子集保真回归 >=95% |
| PI-DOC-03 预览链转换供 PRV | partial | ADUS 有 LibreOffice/Pandoc 相关代码块；A-DOC-X-L-01 未标 G4 | Excel/PPT/docx 预览保真与大文件验证 |

### 5.6 DE

| Plan Item | 当前状态 | 证据 | 阻断/下一步 |
|---|---|---|---|
| PI-DE-01 diff 生成与载荷 | done | A-DE-X-L-01 标记 G4 | 真实模型 ChangeProposal 接线依赖 AG |
| PI-DE-02 diff 终态机与裁决 | done | A-DE-X-L-01 标记 G4 状态机 | A-CORE-X-L-04 要求 handler 真实驱动复核 |
| PI-DE-03 过期与冲突重检 | done | A-DE-X-L-01 标记 G4 | 与外部变更/多 diff G5 联调 |
| PI-DE-04 DiffCard 紧凑审阅卡 | in_progress | A-DE-X-L-01 标记 G4 实现中 | 批量/侧栏/内联整体布局仍待实现期 |
| 模型内容源 | blocked | A-CORE-X-R-03 §5.1 | 依赖 AG/GW 解桩 |

### 5.7 AG

| Plan Item | 当前状态 | 证据 | 阻断/下一步 |
|---|---|---|---|
| PI-AG-01 代理回合机与门禁前置 | done | A-AG-X-L-01 标记 G4 | PI-AG-11 接真实模型 |
| PI-AG-02 工具调用循环机 | done | A-AG-X-L-01 标记 G4 | PI-AG-08 工具清单/执行真实化 |
| PI-AG-03 门禁/网关调用通道 | done | A-AG-X-L-01 标记 G4 | 真云门禁/网关联调 |
| PI-AG-04 局部修改/辅助续写机 | done | A-AG-X-L-01 标记 G4 | 高频门禁与真模型验证 |
| PI-AG-05 记忆写入对接 | done | A-AG-X-L-01 标记 G4 | 与 AST G5 验证 |
| PI-AG-06 构建移交/回落对接 | done | A-AG-X-L-01 标记 G4 | 与 CBT G5 验证 |
| PI-AG-07 系统提示词正文 | planned | A-AG-X-L-02 未标 G4 | 需实现/回填状态 |
| PI-AG-08 工具注册清单与真实执行 | planned | A-AG-X-L-02 未标 G4 | 工具真实执行、propose_edit 路径验证 |
| PI-AG-09 上下文供给真实接入 | planned | A-AG-X-L-02 未标 G4 | 接 REF/AST/KB/WS，验证截断序 |
| PI-AG-10C/10B/10F 流式 IPC | planned | A-AG-X-L-02 未标 G4 | 契约冻结、前后端双轨实现 |
| PI-AG-11 真实模型调用集成 | blocked | A-AG-X-L-02 明确依赖 PI-GW-01~04 | 云 GW 未实现 |
| AG 专项：MODEL-CONFIG-01 模型配置体系 | partial | A-AG-M-D-01 v2.14 / A-AG-M-T-01 v2.24；agRuntime/assistRuntime 测试通过 | 预设清单（GET /models）与订阅档位映射依赖云 GW/LIC |

### 5.8 REF

| Plan Item | 当前状态 | 证据 | 阻断/下一步 |
|---|---|---|---|
| PI-REF-01 @mention 触发与目标解析 | done | A-REF-X-L-01 标记 G4 | KB/AST 纳入候选待联合 |
| PI-REF-02 锚校验与注入载荷组装 | done | A-REF-X-L-01 标记 G4 | 锚失效修复交互实现期 |

### 5.9 CBT

| Plan Item | 当前状态 | 证据 | 阻断/下一步 |
|---|---|---|---|
| PI-CBT-01 构建回合机与任务编排 | done | A-CBT-X-L-01 标记 G4 | 与真实 AG 模型/工具联调 |
| PI-CBT-02 产物 diff 化落盘 | done | A-CBT-X-L-01 标记 G4 | 与 DE/WS G5 验证 |
| PI-CBT-03 构建移交/回落 | done | A-CBT-X-L-01 标记 G4；AG PI-06 已标 G4 | 双侧 G5 联调 |

### 5.10 AST

| Plan Item | 当前状态 | 证据 | 阻断/下一步 |
|---|---|---|---|
| PI-AST-01 记忆条目存储与 workspace 级隔离 | done | A-AST-X-L-01 标记 G4 | G5 切 workspace 验证 |
| PI-AST-02 自动写入机与类型分级 | done | A-AST-X-L-01 标记 G4 | 真实 AG 回合触发验证 |
| PI-AST-03 记忆注入供给 | done | A-AST-X-L-01 标记 G4 | 与 AG PI-09 上下文供给联调 |

### 5.11 KB

| Plan Item | 当前状态 | 证据 | 阻断/下一步 |
|---|---|---|---|
| PI-KB-01 条目存储、组织与主动入库 | done | A-KB-X-L-01 标记 G4 | 入库管线实现期优化 |
| PI-KB-02 检索召回供给 | done | A-KB-X-L-01 标记 G4 | 本地检索无上行网络核验、与 AG PI-09 联调 |

### 5.12 TPL

| Plan Item | 当前状态 | 证据 | 阻断/下一步 |
|---|---|---|---|
| PI-TPL-01 模板管理与原子实例化 | done | A-TPL-X-L-01 标记 G4 | G5 与 WS/AG 联调 |
| PI-TPL-02 工作流执行机 | done | A-TPL-X-L-01 标记 G4 | G5 异常恢复/确认点验证 |

### 5.13 PRV

| Plan Item | 当前状态 | 证据 | 阻断/下一步 |
|---|---|---|---|
| PI-PRV-01 多格式预览 | partial | A-CORE-X-L-03 PI-BE-08~10/PI-FE-22~25 G4；A-PRV-X-L-01 未标 G4 | 模块计划回填、全格式保真/失败态验证 |
| PI-PRV-02 打印与导出 | partial | A-CORE-X-L-03 PI-BE-09+FE-23 G4；A-PRV-X-L-01 未标 G4 | 导出格式清单、workspace 限界验证 |
| PRV 专项（08-05~07）：PRV-PREVIEW-OPT-01 / PRV-TOOLBAR-ADAPTIVE-01 | partial | A-PRV-M-T-01 / A-PRV-I-P-01；PrvToolbar 测试通过 | 全格式保真/失败态验证待做 |

### 5.14 ACCT

| Plan Item | 当前状态 | 证据 | 阻断/下一步 |
|---|---|---|---|
| PI-ACCT-01 会话生命周期承接 | partial | A-ACCT-X-L-01 标记 G4；认证方式仍 C 类 | 真登录依赖云 AUTH |
| PI-ACCT-02 会话令牌安全持有 | done | A-ACCT-X-L-01 标记 G4 | Keychain API/续期时机实现期复核 |

### 5.15 ENT

| Plan Item | 当前状态 | 证据 | 阻断/下一步 |
|---|---|---|---|
| PI-ENT-01 门禁前置与网关调用通道 | partial | A-ENT-X-L-01 标记 G4；云裁决/网关为桩 | 真 CLOUD-GATE/GW 联调 |
| PI-ENT-02 BYOK 代持与用量呈现 | partial | A-ENT-X-L-01 标记 G4；云 BYOK/LIC 为桩 | 真 BYOK 上送、用量计量依赖 GW/LIC |

### 5.16 CLOUD / AUTH / GW / LIC

| 项 | 当前状态 | 证据 | 阻断/下一步 |
|---|---|---|---|
| 云端 APC | done | docs/02_cloud/A-CLOUD-C-P-01_云端APC.md | 独立仓库/独立治理待推进 |
| 云端产品概述 | done | docs/02_cloud/A-CLOUD-C-D-01_云端产品概述.md | AUTH/GW/LIC/PORTAL 模块 L1/L2 继续展开 |
| GW 需求描述 | done | docs/02_cloud/A-GW-M-D-01_AI网关功能需求描述.md | GW L2/计划/实现未启动 |
| AUTH 实现 | not_started | 未见 AUTH 模块实现文档/代码 | 阻断 ACCT 真登录 |
| GW 实现 | not_started | 未见 GW 实现代码 | 阻断 AG PI-AG-11、DE 真实产出 |
| LIC 实现 | not_started | 未见 LIC 模块实现文档/代码 | 阻断 ENT 真计费/用量/G5 |

## §6 跨模块阻断清单

| 阻断 ID | 阻断项 | 影响模块 | 当前状态 | 解除条件 |
|---|---|---|---|---|
| BLK-01 | 云 GW 未实现 | AG、DE、ENT、CBT、AST、KB、TPL、G5 | blocked | 建立云端 GW L2/G3/G4，实现 CLOUD-GATE/CLOUD-INVOKE |
| BLK-02 | AG 真模型调用未接通 | AG、DE、CBT、AST、KB、TPL | blocked | PI-AG-11 完成，真实流式模型输出可用 |
| BLK-03 | DE 真实 ChangeProposal 未接线 | DE、WS、AG | blocked | AG 输出真实 ChangeProposal，经 DE 生成 PendingDiff |
| BLK-04 | 状态机运行时接线未全闭合 | SYS、WS、DE、AG、CBT、AST、TPL、DOC、ED | in_progress | A-CORE-X-L-04 Phase 2/3 完成 |
| BLK-05 | DOC/PRV 全格式保真未闭合 | DOC、PRV、ED | in_progress | docx/Excel/PPT/音视频预览与导出矩阵通过 |
| BLK-06 | G5 NFR 阈值未标定 | ALL | deferred | 进入 G5 时标定性能/恢复/安全指标 |
| BLK-07 | 1.0 EP 云端最小闭环未实现 | AUTH、LIC、GW、PORTAL、ACCT、ENT、AG | blocked | DeepSeek 托管模型、SubscriptionGate、BYOK、管理员端最小能力与反馈 telemetry 形成生产链路 |

## §7 维护规则

1. 每次完成一个 Plan Item，更新对应模块 §5 明细表，并在 §4 模块总表同步 G4 状态。
2. 每次发现阻断，新增或更新 §6 阻断清单，写清解除条件。
3. 每次人类宣布 G 阶段晋级，更新 §2.2 与 §4。
4. 若进度来源来自代码但模块计划未回填，状态只能写 partial 或 needs_review，不得直接写 done。
5. 若上游设计/规则变化，先更新 SSOT 文档和 ADUS，再更新本文。
6. 本文不放注册类标注块；仅维护进度，不承担规则定义职责。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-08-07 | v1.2 | DOC-SYNC-01 回填 08-05~08-07 课题进度：§5.3 WS 新增专项行（WS-NEW-BUTTON-01 / WS-DIR-DELETE-01 / WS-SAVE-TIMELINE-01）；§5.4 ED 新增专项行（工具栏/本地字体库/高亮收敛/txt 工具栏/选区持久化与拖动）；§5.7 AG 新增专项行（MODEL-CONFIG-01）；§5.13 PRV 新增专项行（PRV-PREVIEW-OPT-01 / PRV-TOOLBAR-ADAPTIVE-01）；§2.4 BYOK 发布行注明客户端已实现（云 GW 仍阻断）；同步功能明细台账与产品能力图谱（function-ledger 147 行、注册表 19 变体） |
| 2026-07-17 | v1.1 | 接入 1.0 Experience Preview 发布切片：来源新增 A-CORE-X-L-06；新增 §2.4 发布视图，按上线阻断项重排当前状态；§6 新增 BLK-07 云端最小闭环阻断 |
| 2026-07-15 | v1.0 | 初版：建立项目进度跟踪台账；定义状态枚举、P0-P5/G1-G6 总览、端侧/云侧/模型调用视图、15 个客户端模块 + CLOUD 明细、跨模块阻断清单与维护规则；明确本文不定义规则、不承接 REQ_MAP、不作为代码追溯来源 |
