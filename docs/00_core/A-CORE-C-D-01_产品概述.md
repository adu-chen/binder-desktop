---
文档编号：CORE-C-D-01
文档状态：A
负责模块：CORE
文档职责：Binder 产品概述——产品级功能需求描述与边界，L1 需求层根文档，向各模块 M-D 需求文档分发产品级 REQ
上游约束：A-CORE-C-P-01_APC.md、A-CORE-X-L-01_总体开发路线图.md、ADU.md（§3.2 REQ 规范）
直接承接：各模块 L1 功能需求描述文档（M-D）；产品级 REQ 经各模块 REQ_MAP 进入 L2 技术规则
使用边界：本文是 L1 功能需求描述层，仅声明产品级 REQ-CORE-*，**不定义技术规则**（RULE/CHAIN/CONSTRAINT/TERM 见 L2 技术设计）；**代码不得直接映射 REQ**（ADU §3.2）；功能级详细流程（入口/输入输出/主流程/分支/异常/终态 + 流程图）归各模块 M-D 文档，本文只给产品级骨架与边界
变更要求：产品定位/范围变更须先经 APC 授权再回写本文；REQ 增删按 ADU §3.2 req_id 规则（序号不复用，废弃改 superseded）
治理成熟度：draft
---

# Binder 产品概述（L1 功能需求根）

## §1 文档定位

本文是 L1 功能需求描述层的**产品级根文档**。它回答"产品要做什么、为谁、边界在哪"，
并声明产品级 REQ-CORE-*。各模块在其 M-D 需求文档中声明模块级 REQ-[MODULE]-*，承接本文的
产品级 REQ；模块级 REQ 再经 REQ_MAP 映射到 L2 技术规则（RULE/CHAIN/CONSTRAINT/TERM）。

```
产品概述(本文, REQ-CORE-*) → 模块 M-D(REQ-[MODULE]-*) → REQ_MAP → L2 规则(SSOT) → 代码
```

边界：本文不写技术方案、不定义规则、代码不得追溯本文。权威来源是 APC（已人类确认的宪章）；
本文每条 REQ 均锚定 APC 条款，未被 APC 支撑的产品判断一律进 §7 暴露清单，不擅自杜撰。

## §2 产品定位与目标用户

**定位**（锚 APC §1）：Binder 是 AI 原生的桌面写作应用，商业化上线，Open Core（当前不开源），
首发 macOS，经官网门户 DMG 分发，不走 Mac App Store。

**核心理念**：本地优先 + 人类把关。用户内容（文档、记忆、模板、聊天历史、知识库、引用）
全部本地存储，workspace 是用户选定的本地目录，是文件操作/搜索/diff/上下文读取的边界；
AI 的所有文档修改以 diff 形式呈现，**用户接受/拒绝后才写入文件**。

**云端职责（薄）**：账号/认证、订阅/计费、AI 网关；不承载用户内容、不做内容云同步。

**目标用户**（2026-06-09 人类确认）：**v1 主用户群 = 以 AI 协同进行严肃/专业写作与多文件文档构建的
个人专业写作者**（本地内容主权敏感、愿为 AI 写作能力付费）。次级分群、画像优先级与典型任务深度
**显式延期至 G1 后**补全（可挖 L4 原料：台账 §4.1 产品定义总纲、使用场景总览/场景边界清单），
不阻断产品 G1。

## §3 能力范围（v1）

范围已于 2026-06-09 经人类确认冻结（APC §1、路线图 §2）。

**v1 IN（按模块归属，需求粒度）：**

| 能力 | 归属模块 | 产品级 REQ |
|---|---|---|
| 本地 workspace 写作与文件 I/O、pending_diffs 持久化 | WS | REQ-CORE-001 |
| 对话生成 diff → 接受/拒绝 → 落盘（核心交互） | DE/AG/WS | REQ-CORE-002 |
| 代理编辑（AgentEdit：编辑/构建双模式）+ 工具调用 | AG | REQ-CORE-002 |
| 构建：项目蓝图与多文件文档项目构建（CBT） | CBT | REQ-CORE-003 |
| DOCX 往返、Excel/PPT/音视频**预览** + 打印/导出 | DOC/PRV | REQ-CORE-004 |
| 本地记忆 / 知识库 / 模板工作流 | AST/KB/TPL | REQ-CORE-005 |
| @mention 引用解析与注入 | REF | REQ-CORE-006 |
| 文档级时间轴 / 历史回溯 | WS | REQ-CORE-007 |
| 账号认证 + 订阅门控 + AI 网关（云薄端） | ACCT/ENT + 云 | REQ-CORE-008 |
| 离线边界（本地功能离线可用，AI 在线鉴权+有效订阅） | 跨模块 | REQ-CORE-009 |

**v1 OUT（明确不做，APC §1 + 本次确认）**：多人实时协同、内容云同步、插件市场、
脱离写作场景的通用聊天助手、**Excel/PPT 编辑**。

## §4 核心使用场景与主流程（产品级）

产品级仅给代表性主流程骨架；功能级详细流程图归各模块 M-D（APC §7）。

**S1 代理编辑落盘（核心交互；旧称「对话编辑」，2026-07-04 更名）** — Flow Table：

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | 用户 | 自然语言指令 | 在 workspace 内对目标文档发起对话修改 | 修改请求 | 2 | 无 workspace→提示选目录 |
| 2 | AG | 修改请求 + 文档上下文 | 调模型（经订阅门禁→网关）生成修改 | 修改草案 | 3 | 无有效订阅→拒绝并提示 |
| 3 | DE | 修改草案 | 生成 diff 并呈现，进入待审阅终态前 | diff 卡片 | 4 | 生成失败→错误态可重试 |
| 4 | 用户 | 接受/拒绝 | 裁决 diff | 接受或拒绝决定 | 5 | 超时/文档外部变更→diff 过期终态 |
| 5 | WS | 接受决定 | 写入文件并持久化 | 文件落盘 + 历史节点 | 终态 | 写入竞态→保存竞态防护 |

**S2 多文件文档构建（CBT）**：用户描述目标 → agent 规划并跨文件构建 → 可中断/接管 →
产物写入 workspace（构建状态机，APC §6）。详见 CBT 模块 M-D/M-S。

**S3 预览与输出**：打开 DOCX/Excel/PPT/音视频 → 预览渲染 → 打印/导出（不含 Excel/PPT 编辑）。

**S4 商业化门控**：用户经官网门户购买订阅 → 桌面端登录 → 调 AI 前校验有效订阅（硬门）→
经网关计量计费。密钥仅服务端持有，客户端不持密钥、不可绕过门禁。

## §5 产品级需求声明（REQ-CORE-*）

> 以下为产品级 REQ，owner_module 统一记 CORE（产品核心所有）；落到具体模块时由该模块 M-D
> 声明模块级 REQ-[MODULE]-* 承接，并 REQ_MAP 入 L2。代码不得直接映射本文 REQ（ADU §3.2）。

<!-- REQ
req_id: REQ-CORE-001
title: 本地优先存储与 workspace 边界
owner_module: CORE
status: active
summary: 用户内容全部本地存储，workspace 是文件操作/搜索/diff/上下文读取的唯一边界
source_doc: APC §1
acceptance: 用户内容文件均位于本地 workspace，无内容上行云端（可经网络/存储位置核验）
-->

<!-- REQ
req_id: REQ-CORE-002
title: 对话生成 diff 并经人类裁决后落盘
owner_module: CORE
status: active
summary: AI 文档修改以 diff 呈现，用户接受/拒绝后才写入文件；含代理编辑（AgentEdit，编辑/构建双模式）与工具调用（2026-07-04 术语更名，能力细化经 REQ-AG-* 承接）
source_doc: APC §1
acceptance: AI 修改在用户接受前不写入文件，拒绝后文件保持不变
-->

<!-- REQ
req_id: REQ-CORE-003
title: agent 多文件文档项目构建（CBT）
owner_module: CORE
status: active
summary: 支持 agent 跨多文件构建文档项目（对话阶段→项目蓝图→显式触发→计划确认→执行），含构建中断与构建移交/回落（旧称 Chat↔Build 接管，2026-07-04 更名）
source_doc: APC §1、§4.1
acceptance: agent 可跨≥2 文件构建产物并写入 workspace，构建过程可中断且可接管
-->

<!-- REQ
req_id: REQ-CORE-004
title: 多格式预览与输出
owner_module: CORE
status: active
summary: DOCX 往返、Excel/PPT/音视频预览、打印与导出；不含 Excel/PPT 编辑
source_doc: APC §1、路线图 §2
acceptance: DOCX 可往返，Excel/PPT/音视频可预览并可打印/导出，且 Excel/PPT 无编辑入口
-->

<!-- REQ
req_id: REQ-CORE-005
title: 本地智能资产（记忆/知识库/模板）
owner_module: CORE
status: active
summary: 记忆协同、知识库、模板工作流均本地存储与运行
source_doc: APC §4.1
acceptance: 记忆/知识库/模板数据存于本地且离线可读
-->

<!-- REQ
req_id: REQ-CORE-006
title: 引用解析与上下文注入（@mention）
owner_module: CORE
status: active
summary: 通过 @mention 解析引用并注入对话/构建上下文
source_doc: APC §4.1
acceptance: @mention 可解析引用目标并将其内容注入对话/构建上下文
-->

<!-- REQ
req_id: REQ-CORE-007
title: 文档级时间轴与历史回溯
owner_module: CORE
status: active
summary: 提供文档级版本时间轴与历史回溯/还原（v1 范围，归 WS）
source_doc: APC §1（2026-06-09 范围确认）、路线图 §2
acceptance: 文档具可浏览的历史节点，可回溯并还原到先前版本
-->

<!-- REQ
req_id: REQ-CORE-008
title: 账号认证、订阅门控与 AI 网关（云薄端）
owner_module: CORE
status: active
summary: 登录/订阅校验为调用模型硬门；密钥仅服务端持有，客户端不持密钥、不可绕过门禁；套餐经网关计量，BYOK（v1 GA 纳入）经网关透传不计额度但仍需有效订阅
source_doc: APC §1、§4.3
acceptance: 无有效订阅时模型调用被拒；客户端不存明文密钥；门禁无客户端绕过路径；BYOK 经网关透传且不计额度
-->

<!-- REQ
req_id: REQ-CORE-009
title: 离线边界
owner_module: CORE
status: active
summary: 编辑/预览/打印/本地功能离线可用；AI 功能需在线鉴权 + 有效订阅
source_doc: APC §9
acceptance: 断网时编辑/预览/打印/本地功能可用；AI 功能在断网或无有效订阅时明确拒绝
-->

## §6 产品边界与约束（边界闭合）

- **范围边界**：§3 IN/OUT 为 v1 冻结边界；任何扩缩须先经 APC 授权再回写本文与路线图 §2（APC §13）。
- **内容主权边界**：用户内容不出本地，云端不承载内容、不做内容同步（APC §1）。
- **文件系统边界**：AI 文件操作限定在 workspace 内（APC §9 安全模型）。
- **安全/计费边界**：密钥服务端持有；订阅门禁客户端不可绕过；用量计量防篡改（APC §9）。
- **离线边界**：见 REQ-CORE-009。
- **写入边界**：AI 修改一律经人类裁决（diff 接受/拒绝）后才落盘（REQ-CORE-002）。

## §7 G1 问题暴露清单（处置结果）

按 APC §5 G1 标准；处置决策于 2026-06-09。所有项均已**决策**或**显式延期**（记明 owner + 门禁），
据此产品级 G1 具备闭合条件。晋级由人类宣布（APC §2，AI 不得自行宣布晋级）。

| 编号 | 分类 | 状态 | 处置结果 |
|---|---|---|---|
| PO-01 | NEEDS_HUMAN_DECISION | 已决策 | v1 主用户群确定（§2）；次级分群/画像深度显式延期至 G1 后，不阻断 |
| PO-02 | REQ_GAP | 显式延期 | 模块级 REQ-[MODULE]-* 在各模块 G1 于其 M-D 承接（设计使然） |
| PO-03 | DESIGN_RISK | 显式延期 | 模块编码随 SYS P1 落 MODULE 注册块 + generate 注册；注册块归 A-SYS-C-T-01 系统总体架构 |
| PO-04 | ACCEPTANCE_MISSING | 部分关闭 | 定性验收已补入 §5 各 REQ-CORE-*；NFR 量化指标显式延期至 G5（APC §9） |
| PO-05 | （改归技术） | 移交 | 时间轴↔持久化一致性属技术问题，改归 WS-G2 核对，移出产品 G1 阻断 |
| PO-06 | BOUNDARY_OPEN | 已决策 | BYOK 纳入 v1 GA（契约 APC §4.3 已锁，§5 REQ-CORE-008 已注明）；产品层呈现/切换 UX 显式延期至 ENT G1 |

## §8 后续动作

1. 进入各模块 G1：按本文 §3 归属，在各模块 M-D 声明模块级 REQ 承接产品级 REQ-CORE-*。
2. §7 暴露清单已于 2026-06-09 全部决策/显式延期；产品级 G1 待人类宣布闭合。
3. 与路线图 §4 构建阶段对齐：P1 先行 SYS 底座 + 云接口契约，再 P2 workspace/编辑核心。
4. 范围或定位变更先经 APC 授权再回写本文。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-09 | v1.0 | 初版：产品定位/目标用户、v1 能力范围（含三项范围确认）、核心场景主流程骨架、产品级 REQ-CORE-001~009、产品边界、G1 问题暴露清单 |
| 2026-06-09 | v1.1 | 处置 §7 G1 暴露清单：PO-01 确定 v1 主用户群、PO-06 BYOK 纳入 v1 GA（§2/§5/§7）；为 REQ-CORE-* 补定性验收（§5）；PO-04 量化延 G5、PO-05 改归 WS-G2、PO-02/PO-03 显式延期；产品级 G1 具备闭合条件 |
| 2026-07-04 | v1.2 | 术语沿革回写（人类裁决）：REQ-CORE-002 summary「单文档 chat-agent」→「代理编辑（编辑/构建双模式）」；REQ-CORE-003 summary 补构建全流程、「Chat↔Build 接管」→「构建移交/回落」；§3 能力表与 S1 场景名同步（「对话编辑」退役为「代理编辑」禁用别名） |
