---
文档编号：SYS-C-T-01
文档状态：A
负责模块：SYS
文档职责：Binder 桌面客户端系统总体架构技术设计——分层架构、15 客户端模块编码注册（MODULE 注册块 SSOT）、模块依赖图与跨模块结构约束；承接 REQ-SYS-004
上游约束：A-SYS-M-D-01_系统底座功能需求描述.md（REQ-SYS-004）、A-CORE-C-P-01_APC.md（§4.1 模块清单、§2 架构形态、§13 边界）、A-CORE-X-L-01_总体开发路线图.md（§3 依赖）、ADU.md（§3.0 MODULE、§3.2 块格式）
直接承接：各模块设计文档引用本文注册的 module_code；A-SYS-I-P-01/02/03、A-SYS-M-S-01 等 SYS L2 文档与全体模块 RULE/CHAIN/代码块以此为模块编码基线
使用边界：本文是模块编码注册与架构结构的 SSOT；不定义各业务模块内部规则（归各模块 L2）；具体数据结构/IPC 协议/状态机分别归 A-SYS-I-P-01/02、A-SYS-M-S-01
变更要求：模块编码一旦被规则/代码块引用不可更改（ADU §3.0）；新增模块走 MODULE 创建流程；架构结构约束变更须经 APC 授权
治理成熟度：draft
---

# Binder 系统总体架构（SYS · L2 技术设计）

## §1 文档定位

本文是 SYS 模块 L2 技术设计层文档，承接 REQ-SYS-004（模块注册与依赖边界），承担两项 SSOT 职责：

1. **模块编码注册基线**：15 个桌面客户端模块的 MODULE 注册块集中于本文（§3），
   经 `governance:generate` 提取进 ADUS 模块编码表，是规则 ID / 代码块 ID / 文档编号的模块基础组件。
2. **架构结构 SSOT**：分层架构（§2）、模块依赖图（§4）、跨模块结构约束（§5）。

模块编码值由 APC §4.1 人类已确认指定，本文按权威值转录注册，不自创编码。具体业务规则归各模块 L2。

## §2 分层架构

形态（APC §1/§2）：Tauri 2 + React + TypeScript（前端）+ Rust（后端）的本地优先桌面客户端，
云端薄（账号/订阅/AI 网关，不承载用户内容）。

调用分层（自上而下）：

```
前端 React/TS:  UI 组件  →  Store(状态)  →  Service(领域服务)
                                              │  Tauri invoke / event（IPC 契约，A-SYS-I-P-02）
后端 Rust:                                  Command 层  →  领域逻辑  →  FS / workspace.db
云端(薄, 独立 APC):        Service ──经云接口契约(A-SYS-I-P-03)──→ AUTH / GW / LIC
```

- **本地优先**：文档/记忆/模板/知识库/引用/聊天历史在本地；workspace 是文件/搜索/diff/上下文边界。
- **云薄端**：仅认证、订阅门禁、AI 网关；密钥仅服务端持有，客户端不持密钥、不可绕过门禁（A-SYS-I-P-03）。
- **持久化**：workspace.db 持久化 pending_diffs、时间轴等（schema 基线归 A-SYS-I-P-01）。

## §3 模块编码注册（MODULE SSOT）

> 编码值锚定 APC §4.1（人类已确认）。云端 AUTH/GW/LIC 属独立 APC，不在本客户端模块注册表
> （APC §4.2/§12），仅经 A-SYS-I-P-03 契约约束。

<!-- MODULE
module_code: CORE
module_name: 核心
description: 项目核心与治理，跨模块产品级定义
-->
<!-- MODULE
module_code: SYS
module_name: 系统底座
description: 整体架构、IPC 协议、跨模块契约与模块编码基线
-->
<!-- MODULE
module_code: WS
module_name: 工作台
description: workspace 生命周期、文件 I/O、缓存、pending_diffs 持久化、文档级时间轴与历史回溯
-->
<!-- MODULE
module_code: ED
module_name: 编辑器
description: 编辑器运行时（TipTap，md/docx 富文本编辑）
-->
<!-- MODULE
module_code: DE
module_name: 对话编辑
description: diff 生成、审阅、接受/拒绝/过期终态
-->
<!-- MODULE
module_code: AG
module_name: AI Agent
description: 单文档 chat/agent 与工具调用回合
-->
<!-- MODULE
module_code: CBT
module_name: 文档构建
description: agent 文档项目构建（Chat Build）、构建中断、Chat↔Build 接管
-->
<!-- MODULE
module_code: DOC
module_name: 文档转换
description: DOCX/Pandoc 格式转换
-->
<!-- MODULE
module_code: PRV
module_name: 预览输出
description: Excel/PPT 预览、音视频预览与打印/导出（不含 Excel/PPT 编辑）
-->
<!-- MODULE
module_code: REF
module_name: 引用
description: 引用（@mention）解析与上下文注入
-->
<!-- MODULE
module_code: AST
module_name: 记忆
description: 本地记忆协同
-->
<!-- MODULE
module_code: KB
module_name: 知识库
description: 本地知识库
-->
<!-- MODULE
module_code: TPL
module_name: 模板
description: 模板与工作流
-->
<!-- MODULE
module_code: ACCT
module_name: 账号
description: 账号、认证、会话（云端认证客户端）
-->
<!-- MODULE
module_code: ENT
module_name: 订阅门控
description: 订阅门控、用量与 AI 网关客户端
-->

## §4 模块依赖图

依赖方向（→ 表示"依赖于"，对齐路线图 §3、APC §4.1）：

```
SYS  →  （底座：IPC/数据结构/跨模块契约/模块编码，被全体依赖，自身不反向依赖业务模块）
WS   →  SYS
ED   →  WS
DOC  →  WS, ED
DE   →  ED, WS, AG
AG   →  SYS, WS, ENT, GW(云)
REF  →  WS, AG
CBT  →  AG, WS, DE
AST  →  WS, AG
KB   →  WS, AG
TPL  →  AG, WS
PRV  →  WS
ACCT →  AUTH(云)
ENT  →  ACCT, LIC(云), GW(云)
```

关键路径：一切 AI 能力（AG/CBT/AST/KB/TPL）经 ENT 订阅门禁 → GW 网关方可调用模型（X-SEQ-001，
A-SYS-I-P-03）。依赖图无环；SYS 为底座不反向依赖业务模块（X-FORBID-003）。

## §5 跨模块结构约束（CONSTRAINT）

<!-- CONSTRAINT
constraint_id: X-SEQ-002
摘要: 前端不得跨层直连文件系统/数据库/网络，必经 Service → Tauri command → Rust 后端
涉及模块: SYS, WS, ED, DE, AG, CBT, DOC, PRV, REF, AST, KB, TPL, ACCT, ENT
链路影响: CLOUD-AUTH, CLOUD-GATE, CLOUD-INVOKE
-->

<!-- CONSTRAINT
constraint_id: X-FORBID-003
摘要: 模块依赖必须无环；SYS 为架构底座不得反向依赖任何业务模块
涉及模块: SYS, WS, ED, DE, AG, CBT, DOC, PRV, REF, AST, KB, TPL, ACCT, ENT
链路影响: CLOUD-AUTH, CLOUD-GATE, CLOUD-INVOKE
-->

## §6 需求到规则映射（REQ_MAP）

<!-- REQ_MAP
req_id: REQ-SYS-004
target_type: CONSTRAINT
target_id: X-SEQ-002
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-004
target_type: CONSTRAINT
target_id: X-FORBID-003
status: mapped
-->

> REQ-SYS-004 的"模块注册"部分由 §3 MODULE 注册块 + generate 进 ADUS 模块编码表实现
> （模块注册非 RULE/CHAIN/CONSTRAINT 目标，故不经 REQ_MAP，由 ADUS 模块编码表直接体现）。

## §7 测试与验收

- **模块注册验收**（REQ-SYS-004）：`governance:generate` 后 ADUS 模块编码表含全部 15 客户端模块编码；
  与 APC §4.1 一致。
- **依赖图一致性**：§4 依赖图与路线图 §3 一致，无环；SYS 不反向依赖业务模块。
- **结构约束**：X-SEQ-002（分层调用方向）、X-FORBID-003（无环+底座）在架构评审与后续模块 G4 集成中验证。
- 架构性能 NFR（冷启动等）延 G5 标定（APC §9）。

## §8 技术债务与盲区

- 各模块内部架构（如 ED 的 TipTap 集成、WS 的 db schema）归各模块/各 SYS 接口文档，本文只定编码与依赖骨架。
- 前后端 IPC 具体传输与错误形状归 A-SYS-I-P-02；数据结构归 A-SYS-I-P-01；状态机总图归 A-SYS-M-S-01。

## §9 后续动作

1. **本文落地后立即运行 `npm run governance:generate`**：首次填充 ADUS 模块编码表、规则/链路/约束/术语/
   需求映射注册表（含 A-SYS-I-P-03 的 BR-SYS-*/CLOUD-*/X-*/TERM-CLOUD-*/REQ_MAP）。
2. generate 后运行 `npm run governance:audit` 查看状态：首次接入解除后，关注块提取是否成功；
   @GOV/代码相关检测因尚无代码而为预期空，非缺陷。
3. 继续 SYS G2 剩余件：A-SYS-I-P-01 数据结构、A-SYS-I-P-02 IPC 协议、A-SYS-M-S-01 状态机总图。
4. 新增模块走 ADU §3.0 MODULE 创建流程；编码一旦被引用不可更改。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-10 | v1.0 | 初版（SYS G2）：分层架构；注册 15 客户端模块编码（MODULE SSOT，承接 REQ-SYS-004）；模块依赖图（无环、SYS 底座）；跨模块结构约束 X-SEQ-002/X-FORBID-003；REQ_MAP；测试验收与债务盲区 |
