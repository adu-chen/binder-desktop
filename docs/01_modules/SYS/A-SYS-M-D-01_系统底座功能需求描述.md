---
文档编号：SYS-M-D-01
文档状态：A
负责模块：SYS
文档职责：SYS 系统底座模块级功能需求描述——声明模块级 REQ-SYS-*，承接产品级 REQ-CORE-*，界定 IPC 协议 / 跨模块数据结构契约 / 状态机总图 / 模块注册 / 云接口契约边界
上游约束：A-CORE-C-P-01_APC.md（§4.1 SYS 职责、§4.3 云契约、§5 门禁、§6 状态机、§7 流程图）、A-CORE-C-D-01_产品概述.md（REQ-CORE-*）、A-CORE-X-L-01_总体开发路线图.md（P1）、ADU.md（§3.2 REQ 规范）
直接承接：SYS L2 技术设计（A-SYS-C-T-01 架构 / A-SYS-I-P-01 数据结构 / A-SYS-I-P-02 IPC 协议 / A-SYS-M-S-01 状态机总图 / A-SYS-I-P-03 云接口契约）经 REQ_MAP 映射 RULE/CHAIN/CONSTRAINT/TERM
使用边界：本文是 L1 功能需求描述层，仅声明模块级 REQ-SYS-*，**不定义技术规则**（RULE/CHAIN/CONSTRAINT/TERM/STATE_MACHINE 见 SYS L2）；**代码不得直接映射 REQ**（ADU §3.2）；具体接口字段/协议/表结构/状态机枚举归 L2，本文只给契约边界与骨架
变更要求：SYS 职责或云契约边界变更须先经 APC 授权再回写本文；REQ 增删按 ADU §3.2 req_id 规则（序号不复用，废弃改 superseded）
治理成熟度：draft
---

# Binder SYS 系统底座功能需求描述（L1）

## §1 文档定位

本文是 **SYS 模块**的 L1 功能需求描述，处在产品概述（L1 根）之下、SYS L2 技术设计之上：

```
产品概述(REQ-CORE-*) → 本文(REQ-SYS-*) → REQ_MAP → SYS L2 规则(SSOT) → 代码(@GOV)
```

SYS 是被几乎所有客户端模块依赖的底座（APC §4.1、路线图 §3）。本文声明 SYS 自身的模块级
需求 REQ-SYS-*，每条承接一条或多条产品级 REQ-CORE-*；本文不写技术方案、不定义规则、
代码不得追溯本文。具体 IPC 调用签名、数据表结构、状态机枚举、云协议字段一律归 SYS L2，
本文只锁定**契约边界与职责骨架**。未被 APC / 产品概述支撑的判断一律进 §7 暴露清单，不杜撰。

## §2 模块定位与职责

**定位**：SYS 是 Binder 桌面客户端的**架构底座与跨模块契约层**，不直接面向终端用户功能，
而是为 WS/ED/DE/AG/CBT/DOC/PRV/REF/AST/KB/TPL/ACCT/ENT 提供统一的调用协议、共享数据结构、
状态机总图、模块注册与云接口契约边界（APC §4.1）。

**职责清单（v1）**：

| # | 职责 | 锚定 |
|---|---|---|
| J1 | 统一 IPC / Tauri command 协议：前端（React/TS）↔ 后端（Rust）的调用、错误、流式回传契约 | APC §4.1 |
| J2 | 跨模块共享数据结构契约：跨模块传递的实体/类型、workspace.db 持久化 schema 基线 | APC §4.1、§9 |
| J3 | 状态机总图与清单：APC §6 核心链状态机的总图框架与注册入口 | APC §6 |
| J4 | 模块注册与依赖边界：MODULE 注册块、模块依赖图（承接产品概述 PO-03） | APC §12、产品概述 §7 |
| J5 | 云接口契约边界：认证 / 订阅门禁 / AI 网关的客户端侧契约定义（密钥仅服务端、不可绕过） | APC §4.3、§13 |

> 边界澄清：SYS 定义云接口**契约**（客户端如何调用云、契约形状），ACCT/ENT 是实现该契约的
> 客户端消费者；云端 AUTH/GW/LIC 由独立 APC 实现，本项目仅在 SYS L2 定义接口契约。

## §3 能力范围（SYS v1）

**IN（SYS 承载）**：
- 前后端 IPC/command 统一协议（含同步调用、流式/事件、统一错误形状）。
- 跨模块共享类型与 workspace.db schema 基线、迁移契约框架。
- 核心链状态机总图与注册（具体状态机由各 owner 模块在其 M-S 定义，SYS 提供总图与跨模块转移契约）。
- 模块编码注册（MODULE 注册块）与依赖关系声明。
- 云接口契约的客户端边界（认证/订阅门禁/网关），保证"密钥仅服务端、门禁不可客户端绕过"在契约层成立。

**OUT（非 SYS）**：
- 任何终端用户功能（编辑、对话、预览、记忆等）——归各业务模块。
- 云端服务实现（AUTH/GW/LIC 内部逻辑）——独立 APC。
- 具体接口字段、表结构、状态机枚举、协议报文——归 SYS L2（本文只定边界）。
- NFR 量化指标（IPC 延迟、迁移性能等）——延 G5 标定（APC §9）。

## §4 代表性流程（契约级，APC §7）

SYS 为基础设施，给出其所承载契约的代表性流程表，标注模块边界、人类确认点与异常/终态。

**F1 · IPC 调用契约链**（J1）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | 业务模块(UI/Store) | 调用意图 + 参数 | 经 Service 层发起统一 command 调用 | command 请求 | 2 | 参数不符契约→契约层校验拒绝 |
| 2 | SYS IPC 契约 | command 请求 | 按统一协议路由至 Rust 后端 | 后端调用 | 3 | 未注册 command→协议错误（统一错误形状） |
| 3 | Rust 后端 | 后端调用 | 执行并按契约封装结果/错误/流式分片 | 结果或错误或流 | 4 | 执行失败→统一错误形状回传 |
| 4 | 业务模块 | 结果/错误 | 按契约消费 | 调用完成 | 终态 | 流式中断→契约定义的中断/重试语义 |

**F2 · 云门控调用边界**（J5，承接 REQ-CORE-008）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | ENT 客户端 | 模型调用意图 | 按云契约携带会话凭证发起调用 | 门禁请求 | 2 | 未登录→契约要求先认证（ACCT） |
| 2 | 订阅门禁契约 | 门禁请求 | 服务端校验有效订阅（硬门，客户端无绕过路径） | 校验结果 | 3 | 无有效订阅→拒绝模型调用（终态） |
| 3 | AI 网关契约 | 通过校验的请求 | 服务端持密钥代理模型；套餐计量 / BYOK 透传不计额度 | 流式回传 | 终态 | 网关错误→统一错误形状回传 |

> F2 在 L1 仅锁定边界与硬门顺序；令牌格式、校验响应结构、网关流式协议归 A-SYS-I-P-03（L2）。

**F3 · 状态机注册与总图**（J3）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | owner 模块 | 该模块核心链状态机定义(M-S) | 向 SYS 总图登记 machine_id / 跨模块转移点 | 注册项 | 2 | 缺状态机定义→该链不得进入实现（APC §6、ADP SC-06/07） |
| 2 | SYS 总图 | 注册项集合 | 汇总核心链总图与跨模块转移契约 | 状态机总图清单 | 终态 | 跨模块转移冲突→暴露为 DESIGN_RISK 待裁决 |

## §5 模块级需求声明（REQ-SYS-*）

> owner_module = SYS。每条经 source_doc 标注所承接的产品级 REQ-CORE-* 与 APC 锚点。
> 代码不得直接映射本文 REQ（ADU §3.2）；映射到规则在 SYS L2 经 REQ_MAP 完成。

<!-- REQ
req_id: REQ-SYS-001
title: 统一 IPC / Tauri command 协议
owner_module: SYS
status: active
summary: 定义前端↔Rust 后端的统一调用协议，覆盖同步调用、流式/事件回传与统一错误形状，作为全体跨进程调用的单一契约
source_doc: REQ-CORE-002、APC §4.1
acceptance: 所有跨进程调用经统一 command 协议；存在统一错误形状；流式回传有契约化的中断/完成语义
open_issues: 传输机制选型（invoke vs channel/event 流式）属 L2 技术选型，延 G2
-->

<!-- REQ
req_id: REQ-SYS-002
title: 跨模块共享数据结构与持久化 schema 基线
owner_module: SYS
status: active
summary: 定义跨模块传递的共享实体/类型与 workspace.db 持久化 schema 基线及迁移契约框架，支撑 pending_diffs、时间轴等持久化
source_doc: REQ-CORE-001、REQ-CORE-002、REQ-CORE-007、APC §4.1、§9
acceptance: 跨模块共享类型有单一契约来源；workspace.db schema 有版本化迁移契约；重启后持久化态可恢复
open_issues: 具体表结构与 WS 模块协作确定，归 A-SYS-I-P-01 + WS G2
-->

<!-- REQ
req_id: REQ-SYS-003
title: 状态机总图与注册
owner_module: SYS
status: active
summary: 提供 APC §6 核心链状态机的总图框架与注册入口，汇总跨模块转移契约；具体状态机由各 owner 模块在其 M-S 定义
source_doc: REQ-CORE-002、REQ-CORE-003、APC §6
acceptance: 核心链状态机可在总图注册并可追溯 owner；跨模块转移点有契约登记；未注册状态机的状态化链路被阻断进入实现
open_issues: 总图清单完整枚举依赖各模块 G2 暴露真实链路，SYS 提供框架并随后回填
-->

<!-- REQ
req_id: REQ-SYS-004
title: 模块注册与依赖边界
owner_module: SYS
status: active
summary: 落地 15 客户端模块的编码注册（MODULE 注册块）与模块依赖图，作为治理负空间扫描与跨模块约束的基线（承接产品概述 PO-03）
source_doc: APC §4.1、§12、产品概述 §7（PO-03）
acceptance: 全部客户端模块有注册块并经 governance:generate 进入 ADUS 模块编码表；模块依赖图与路线图 §3 一致
open_issues: 注册块物理归属 A-SYS-C-T-01；本文仅声明需求
-->

<!-- REQ
req_id: REQ-SYS-005
title: 云接口契约边界（认证 / 订阅门禁 / 网关）
owner_module: SYS
status: active
summary: 定义客户端侧云接口契约边界——认证会话、调用模型前订阅门禁硬门、AI 网关代理；保证密钥仅服务端持有、客户端不持密钥、门禁不可客户端绕过、BYOK 经网关透传不计额度仍需有效订阅
source_doc: REQ-CORE-008、APC §4.3、§13
acceptance: 契约层无客户端持密钥路径；模型调用必经订阅门禁硬门；无有效订阅一律被拒；BYOK 经网关透传且不计额度
open_issues: 令牌/校验/网关流式协议字段属 L2，归 A-SYS-I-P-03（G2）
-->

<!-- REQ
req_id: REQ-SYS-006
title: 前端交互架构与一致性
owner_module: SYS
status: active
summary: 定义客户端前端交互的系统级骨架——应用外壳阶段（欢迎页 / 主工作台）、跨模块界面分区与协作点、前端状态对后端状态机/控制态的投影一致性；前端交互为视图层 SSOT，统一约束前端按图施工，不得各模块各拼一套
source_doc: REQ-CORE-001/002/004/007/009、APC §7（代表性流程）、路线图 §9（UI 板块）
acceptance: 存在统一的前端交互架构与核心流程总图；前端状态不另立权威（投影后端状态机）；欢迎页/工作台切换、对话→diff→裁决、docx 开/编/预览/存等主干交互可端到端追溯
open_issues: 视图层总图归 L2（A-SYS-C-T-02）；UI 视觉规范与组件细节 just-in-time 贴模块推进（路线图 §9.2）
-->

## §6 模块边界与约束（边界闭合）

- **职责边界**：SYS 只承载架构契约与底座（§2 J1–J5），不实现任何业务功能；越界即范围外（CLAUDE.md §1 范围纪律）。
- **L1/L2 边界**：本文只定契约**边界**；接口字段、表结构、状态机枚举、协议报文一律归 SYS L2，代码只追溯 L2（ADU §3.2、APC §2）。
- **安全/计费边界**（REQ-SYS-005）：密钥仅服务端；订阅门禁客户端不可绕过；客户端不内置 provider SDK（APC §4.3、§13）。
- **云/客户端边界**：SYS 定义客户端侧契约，不实现云端逻辑（AUTH/GW/LIC 独立 APC）。
- **状态机边界**（REQ-SYS-003）：状态化核心链必须先有状态机定义方可实现，SYS 不替各模块定义其状态机，只提供总图与跨模块转移契约（APC §6、ADP SC-06/07）。
- **L4 原料边界**：取材见台账 §4.2（SYS）；旧 binder 文档为原料不为权威，REQ 重新撰写（APC §8）。云契约部分无 L4 原料（greenfield，台账 §4.15）。

## §7 G1 问题暴露清单

按 APC §5 G1 标准（REQ_GAP/BOUNDARY_OPEN/REQ_CONFLICT/FLOW_INCOMPLETE/ACCEPTANCE_MISSING/
NEEDS_HUMAN_DECISION/DESIGN_RISK）。各项均**显式延期**至 G2/G5 并记明 owner+门禁，据此 SYS
模块级 G1 具备闭合条件。晋级由人类宣布（APC §2，AI 不得自行宣布晋级）。

| 编号 | 分类 | 状态 | 处置 |
|---|---|---|---|
| SY-01 | DESIGN_RISK | 显式延期 | 云接口契约具体协议（令牌/订阅校验响应/网关流式）属 L2，延期至 A-SYS-I-P-03（G2） |
| SY-02 | NEEDS_HUMAN_DECISION | 显式延期 | IPC 传输机制选型（invoke vs channel/event 流式）属 L2 技术选型，延期 A-SYS-I-P-02（G2） |
| SY-03 | BOUNDARY_OPEN | 显式延期 | workspace.db 具体表结构与 WS 协作，归 A-SYS-I-P-01 + WS G2 联合确定 |
| SY-04 | FLOW_INCOMPLETE | 显式延期 | 状态机总图完整清单依赖各模块 G2 暴露真实链路；SYS 提供框架，随各模块 G2 回填 |
| SY-05 | ACCEPTANCE_MISSING | 部分关闭 | 定性验收已入 §5 各 REQ-SYS-*；量化指标（IPC 延迟、迁移性能）延 G5 NFR 标定（APC §9） |

## §8 后续动作

1. 待人类宣布 SYS 模块级 G1 闭合后，进入 SYS G2（L2 技术设计，规则 SSOT），按路线图 §6.2：
   - `A-SYS-I-P-03` 云接口契约（**优先**：greenfield + 商业化关键路径，承接 REQ-SYS-005 / REQ-CORE-008）；
   - `A-SYS-C-T-01` 系统总体架构（含 MODULE 注册块，承接 REQ-SYS-004）；
   - `A-SYS-I-P-01` 数据结构定义（承接 REQ-SYS-002）；
   - `A-SYS-I-P-02` IPC / Tauri command 协议（承接 REQ-SYS-001）；
   - `A-SYS-M-S-01` 状态机总图与清单（承接 REQ-SYS-003）。
2. G2 各文档经 REQ_MAP 把 REQ-SYS-* 映射为 RULE/CHAIN/CONSTRAINT/TERM；状态机入 M-S。
3. SYS 职责或云契约边界变更先经 APC 授权再回写本文（APC §13）。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-10 | v1.0 | 初版：SYS 底座模块级 L1；声明 REQ-SYS-001~005（IPC 协议 / 共享数据结构 / 状态机总图 / 模块注册 / 云接口契约边界），承接 REQ-CORE-002/008 等；代表性契约流程表 F1–F3；边界闭合与 G1 暴露清单（SY-01~05 显式延期/部分关闭） |
