---
文档编号：CORE-X-L-06
文档状态：A
负责模块：CORE
文档职责：Binder 1.0 Experience Preview 发布切片范围与验收计划——定义早期上线体验版的发布阻断项、非阻断项、模块映射、云端模型策略、验收门禁与体验数据闭环
上游约束：A-CORE-C-P-01_APC.md、A-CORE-X-L-01_总体开发路线图.md、A-CORE-X-L-02_总开发实施计划.md、A-CORE-X-L-05_项目进度跟踪台账.md、A-CLOUD-C-D-01_云端产品概述.md、A-GW-M-D-01_AI网关功能需求描述.md
直接承接：1.0 Experience Preview 发布排期、G5/G6 前的发布阻断项判断、A-AG-X-L-04 的 agent-edit-1.0 执行计划、云端托管模型与 BYOK 路线、体验反馈闭环
使用边界：本文是 L3 发布切片视图，不定义产品需求、技术规则、注册术语或代码规则来源；所有功能规则仍以各模块 L1/L2/L3 为准，代码 @GOV 不得追溯本文
变更要求：1.0 体验版发布阻断项、模型策略、验收线或非阻断项变化时同步更新；若变化影响完整功能地图或产品边界，须先回写 APC 与总体路线图
治理成熟度：draft
---

# Binder 1.0 Experience Preview 发布范围与验收计划

## §1 决策背景

2026-07-17 人类确认：Binder 需要在完整功能地图不变、全功能开发不暂停的前提下，先发布一个
1.0 Experience Preview（以下简称 1.0 EP）用于验证核心产品体验。

1.0 EP 的目标不是交付完整 Binder Platform，而是先把工作台、编辑器、Agent、Diff Edit、
文档预览与编辑、账号与订阅门禁、AiGateway、BYOK、体验反馈闭环打通，获得真实用户体验数据。

构建（Build）、记忆协同、知识库、模板/工作流、自部署 Binder 预设模型、多模态模型矩阵等能力
继续按完整路线推进，但不作为 1.0 EP 上线阻断项。

## §2 发布定位

1.0 EP 定位：

> 本地优先写作工作台 + AI 辅助编辑 + Diff 审阅 + 云端托管模型/BYOK 的早期体验版。

1.0 EP 必须验证：

| 验证主题 | 说明 |
|---|---|
| 核心工作流 | 用户能否在本地 workspace 内完成打开、编辑、Agent 建议、Diff 审阅、接受/拒绝、保存 |
| 人类裁决链 | 已有 LogicalState 时 AI 预览经 ED 受控写入 LogicalState，显式接受只裁决，而保存有效 pending 预览会一并裁决并写 DiskState；无 LogicalState 时卡片只挂载候选，接受后才写 DiskState；pending 后打开必须迁移为前者 |
| 模型体验 | DeepSeek 托管模型能否支撑早期写作/编辑体验；BYOK 是否能覆盖高阶用户偏好 |
| 专业短板暴露 | 通过真实任务反馈识别模型在专业文档、风格、逻辑、结构和措辞上的短板 |
| 商业化闭环 | 账号、SubscriptionGate、AiGateway、用量、管理员端最小能力是否足以支撑早期运营 |

## §3 1.0 EP 发布阻断项

下列能力是 1.0 EP 发布阻断项，未达到验收线不得发布。

| 能力 | 负责模块 | 权威来源 | 1.0 EP 验收线 |
|---|---|---|---|
| workspace 生命周期与文件边界 | WS / SYS | WS L1/L2/L3、SYS 契约 | 可打开/切换/关闭 workspace；文件访问限定在当前 workspace；越界访问被拒绝 |
| 编辑器核心编辑 | ED / WS | ED L1/L2/L3、WS 保存链 | 可打开、编辑、保存文本/Markdown；脏状态、保存失败、关闭/切换保护可用 |
| DOCX 基础链路 | DOC / ED / PRV | DOC L1/L2/L3、PRV L1/L2/L3 | DOCX 预览与基础编辑链路可用于常见文档；失败态明确；不丢失原文件 |
| Agent 面板 | AG / ENT / WS | AG L1/L2、A-AG-X-L-04、A-WS-M-T-02/M-S-02、ENT 契约 | `agent-edit-1.0` 可理解当前 LogicalState/选区/显式只读引用，完成回答、分析和当前文档编辑；WorkspaceOperation 经独立 DecisionRequest 前置裁决，新建文档经 CreateDocumentBundle 先建真实目标再投影初始内容 Diff；每轮有最小工具允许集，使用真实 GateOutcome/取消/唯一终态；内容写意图进入 Diff，资源写意图进入 WorkspaceOperationPlan，失败不伪报成功 |
| Diff Edit | DE / WS / ED | DE L1/L2/L3、三态模型 | AI 修改建议以 ChangeProposal/PendingDiff 呈现；已有 LogicalState 走预览→裁决→保存，无 LogicalState 走卡片挂载→接受落盘，pending 后打开必须迁移 |
| AiGateway 托管模型 | CLOUD / GW / ENT / AG | 云端 L1、GW L1、A-SYS-I-P-03 | 1.0 默认经 AiGateway 调用 DeepSeek 官方 API；客户端不直连 provider、不持平台密钥 |
| BYOK | GW / ENT / PORTAL | 云端 L1、GW L1、ENT 设计 | BYOK 密钥经 AiGateway 加密代持；BYOK 调用仍经 SubscriptionGate；客户端不持 provider 密钥 |
| 账号与订阅门禁 | AUTH / LIC / ACCT / ENT | 云端 L1、ACCT/ENT 设计 | 用户可登录；AI 调用前可执行 SubscriptionGate；无有效订阅拒绝托管模型与 BYOK 调用 |
| 管理员端最小能力 | PORTAL / AUTH / LIC / GW | 云端 L1 | 可查看用户、订阅状态、调用量、模型错误、网关健康，并支持早期人工处理用户问题 |
| 体验反馈与脱敏 telemetry | CORE / CLOUD / GW / DE / AG | 本文 + 云端数据边界 | 可记录任务类型、模型、耗时、是否接受 Diff、错误码等脱敏元数据；默认不保存调用正文 |
| macOS 交付 | CORE / SYS | APC §11、路线图 §7 | DMG 可安装；签名/公证策略明确；版本号与发布说明齐备 |

## §4 1.0 EP 非阻断项

下列能力不作为 1.0 EP 上线阻断项，但不代表取消、降级或停止开发。

| 能力 | 负责模块 | 1.0 EP 处置 | 后续方向 |
|---|---|---|---|
| 构建（Build） | CBT | 非阻断，继续开发 | 作为 1.1+ 多文件项目构建能力进入发布切片 |
| AG 高级能力 | AG / DE / ED / WS | 多个既有文档写入、复杂 Planning、参考风格/排版、多 Agent 默认关闭；辅助续写/局部修改和高级 BDocx 编辑可保留但不阻断发布 | 先完成 A-AG-X-L-04 的可靠当前文档编辑闭环，再按冻结的 P1 子能力、独立授权与验证逐项开放 |
| 记忆协同 | AST | 非阻断，继续开发 | 基于真实反馈决定写入策略、可见性与召回形式 |
| 知识库 | KB | 非阻断，继续开发 | 结合专业场景反馈设计本地知识组织和检索供给 |
| 模板/工作流 | TPL | 非阻断，继续开发 | 先收集用户高频文档类型，再沉淀模板体系 |
| 自部署 Binder 预设模型 | CLOUD / GW | 非阻断 | 先以 DeepSeek 托管模型 + BYOK 收集体验数据，再启动自部署模型评测 |
| 多模态模型矩阵 | CLOUD / GW / AG | 非阻断 | 先保留接口和数据闭环，后续按真实场景引入视觉模型/模型路由 |
| 自动模型分发策略 | CLOUD / GW | 非阻断 | 1.0 只允许简单双档位/规则路由，复杂 ModelRouter 后置 |
| 深度专业领域适配 | CORE / AG / KB / TPL | 非阻断 | 以评测集、模板、知识库和后续微调逐步推进 |
| 企业级权限与组织管理 | AUTH / PORTAL | 非阻断 | 个人/小团队体验反馈稳定后再进入企业线 |
| 应用原生菜单栏 | SYS | 非阻断，1.0 不单独实现（2026-08-06 人类决策） | 保持现状（系统默认菜单 + 前端自绘 Logo/工具栏下拉）；后续版本再评估 Tauri 原生 Menu 或统一前端菜单体系 |

## §5 模型与 AiGateway 策略

1.0 EP 不交付自部署 Binder 预设模型，不交付完整模型矩阵。模型策略为：

| 项 | 1.0 EP 决策 |
|---|---|
| 前端展示 | 显示单一 Binder 托管模型体验，不向普通用户暴露内部 provider 细节 |
| 托管模型 | 经 AiGateway 接入 DeepSeek 官方 API，优先使用 DeepSeek 当前推荐的 V4 系列模型 |
| BYOK | 用户可配置自带 provider 密钥，但密钥仍经 AiGateway 加密代持，不由客户端保存或直连 |
| 路由复杂度 | 允许最小规则路由，例如轻量编辑走 fast 档、复杂 Diff/审稿走 pro 档；不引入复杂模型矩阵 |
| 数据闭环 | 记录脱敏效果元数据，用于后续 Binder 预设模型评测；默认不保存用户正文 |
| 后续演进 | 1.1+ 启动开源多模态模型评测；1.2+ 建 Binder 专项评测集；预设模型 Beta 后置 |

该策略不改变云端硬边界：

- 客户端不持平台 provider 密钥。
- 所有 AI 调用先经过 SubscriptionGate。
- AiGateway 不持久化模型调用正文。
- BYOK 不绕过订阅门禁。
- 用量与错误上报只记录脱敏元数据。

## §6 验收门禁

1.0 EP 上线前至少满足：

| 门禁 | 验收要求 |
|---|---|
| 核心链路 | workspace → editor → Agent → Diff → accept/reject → save 连续可用 |
| Agent Runtime | `agent-edit-1.0` 的 IntentResult、可信 Context/Prompt、逐回合工具授权、真实门禁、后端取消和唯一终态成立；回答/分析不产生写调用；WorkspaceOperation 只经 DecisionRequest 前置裁决后执行，新建文档按 CreateDocumentBundle 先建真实目标、再投影初始内容 Diff |
| 数据安全 | AI 失败、Diff 应用失败、保存失败均不破坏原文件；无 LogicalState 的后台分支在用户接受前不落盘，已有 LogicalState 的分支仅保存可写盘 |
| 模型链路 | AiGateway + DeepSeek 托管模型生产配置可用；BYOK 可用；错误码明确 |
| 账号订阅 | 登录、会话、SubscriptionGate、用量展示可用；无订阅拒绝调用 |
| 管理员端 | 可定位用户、订阅、调用错误、网关状态，并支持早期运营处理 |
| 文档预览 | DOCX/PDF/常见预览链路可用；不支持项有明确提示 |
| 验证 | `npm run governance:audit` 通过；相关前端/Rust 测试通过；发布前打包验收通过 |
| 合规 | 隐私说明明确：托管模型经 AiGateway 调用第三方模型服务；默认不保存调用正文 |
| 反馈 | 用户可对 AI 结果反馈；系统可统计接受率、失败率、任务类型、模型档位 |

## §7 与完整功能开发的关系

1.0 EP 是完整路线图上的发布切片，不是独立分支需求：

```text
完整功能地图继续推进
        ↓
1.0 EP 只选择其中一组能力作为早期发布阻断项
        ↓
功能规则仍回到各模块 L1/L2/L3
        ↓
后续版本继续把 CBT/AST/KB/TPL/自部署模型/多模态纳入发布切片
```

因此，模块文档中的 G4/G5 推进不因 1.0 EP 非阻断而停止。本文只改变发布判定，不改变完整产品目标。

## §8 后续版本迁移路线

| 版本 | 目标 |
|---|---|
| 1.0 EP | 核心工作流体验、DeepSeek 托管模型、BYOK、账号订阅、反馈闭环 |
| 1.1 | 多模态候选模型评测与截图/文档页面理解试点；构建/模板/知识库按反馈选择首批能力 |
| 1.2 | Binder 专项模型评测集、模型路由 Beta、专业文档类型样例集 |
| 1.3 | 自部署 Binder 预设模型 Beta；模型矩阵小流量验证 |
| 2.0 | 完整 Binder Platform：模型矩阵、自动分发、领域调优、知识库/模板/记忆深度联动 |

## §9 非阻断未决项

| 项 | 状态 | 处理 |
|---|---|---|
| DeepSeek V4 具体模型名与价格 | 待云端实现前确认 | 在 GW L2/实现前核对 DeepSeek 官方当前模型列表 |
| 订阅档位与定价 | 待人类商业决策 | 不阻断本文成立，但阻断 LIC/PORTAL 生产实现 |
| BYOK 加密方案 | 待人类安全决策 | GW-02 前置，推荐 KMS/信封加密路线 |
| 1.0 EP 对外命名 | 待人类市场决策 | 建议使用 Binder 1.0 Experience Preview 或 Binder 1.0 体验版 |

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-07-17 | v1.0 | 初版：基于人类确认的早期发布决策，建立 1.0 EP 发布切片；明确发布阻断项（WS/ED/DOC/PRV/AG/DE/AiGateway/BYOK/账号订阅/管理员端/反馈/DMG）、非阻断项（CBT/AST/KB/TPL/自部署预设模型/多模态/模型矩阵等）、DeepSeek 托管模型 + BYOK 策略、验收门禁、与完整功能开发的关系和后续版本迁移路线 |
| 2026-08-06 | v1.1 | 记录人类决策：1.0 版本不单独实现应用原生菜单栏（§4 非阻断项补充；右键菜单体系改按 SYS 统一 ContextMenu 简化方案推进，不并入菜单栏） |
| 2026-08-08 | v1.2 | 同步 AG 1.0 落地基线：Agent 发布阻断项收敛为 agent-edit-1.0 可靠当前文档编辑闭环；补充 IntentResult、可信 Context/Prompt、逐回合工具授权、真实门禁/取消/唯一终态和空基准新建提案验收；跨文档写入、文件自动整理、复杂 Planning、参考风格/排版与多 Agent 明确为非阻断且默认关闭 |
| 2026-08-13 | v1.3 | DIFF-STATE-COMPASS-01：发布验收中的 Diff/保存主链按 BR-CORE-STATE-001~003 收敛；无 LogicalState 的挂载卡在接受前不得落盘，已有或打开后迁移出的 LogicalState 只能经保存写盘。 |
| 2026-08-21 | v1.4 | WorkspaceOperation 发布验收同步：资源操作经 DecisionRequest 前置裁决、重新核验后直接写盘；新建改为 CreateDocumentBundle，废止空基准接受后创建链。 |
| 2026-08-31 | v1.5 | AG-DOC-CONSOLIDATION-01：Agent 1.0 发布承接统一指向 A-AG-X-L-04；旧 AG 计划不再作当前范围、设计或状态真源。 |
