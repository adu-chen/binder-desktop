---
文档编号：ENT-M-D-01
文档状态：A
负责模块：ENT
文档职责：ENT 订阅门控模块级功能需求描述——声明模块级 REQ-ENT-*，承接产品级 REQ-CORE-008/009 与 SYS 云接口契约（CLOUD-GATE/INVOKE 客户端侧），界定订阅门禁客户端执行 / AI 网关客户端 / BYOK 客户端边界 / 用量与订阅状态呈现
上游约束：A-CORE-C-D-01_产品概述.md（REQ-CORE-008/009）、A-CORE-C-P-01_APC.md（§4.1 ENT 职责「订阅门控、用量、AI 网关客户端」、§4.3 云薄端、§13）、A-SYS-I-P-03_云接口契约.md（CLOUD-GATE/INVOKE、SM-SYS-01 门禁+网关段、TERM-CLOUD-001/002/004、X-FORBID-001/002、X-SEQ-001）、A-SYS-I-P-02_IPC协议.md（统一错误形状、流式终态）、A-CORE-X-L-01_总体开发路线图.md（§7 购买在官网门户）、ADU.md（§3.2）
直接承接：ENT L2 技术设计经 REQ_MAP 映射 RULE；ENT 承接 SM-SYS-01 门禁+网关段运行时（不重定义状态机）；AG/CBT/AST/KB/TPL 的模型调用经 ENT 客户端通道；greenfield（台账 §4.15）
使用边界：本文是 L1 功能需求描述层，仅声明 REQ-ENT-*；**代码不得直接映射 REQ**（ADU §3.2）；云协议 SSOT 归 A-SYS-I-P-03；裁决/计量/计费归服务端（GW/LIC，独立 APC）；会话令牌归 ACCT；购买流程在官网门户（桌面端仅展示与跳转）
变更要求：门禁/网关客户端语义变更涉 APC §4.3/§13 须先授权；REQ 增删按 ADU §3.2
治理成熟度：draft
---

# Binder ENT 订阅门控功能需求描述（L1）

## §1 文档定位

```
产品概述(REQ-CORE-008) + A-SYS-I-P-03(CLOUD-GATE/INVOKE 契约) → 本文(REQ-ENT-*) → REQ_MAP → ENT L2 → 代码(@GOV)
```

ENT 是**订阅门禁与 AI 网关的客户端模块**（greenfield）：承接 SM-SYS-01 的**门禁+网关段**运行时
（GateChecking/GatePassed/GateDenied/Invoking/终态），是 AG/CBT/AST/KB/TPL 一切模型调用的唯一
客户端通道。**商业化关键路径核心**：裁决权在服务端、客户端无绕过、不持密钥（APC §4.3/§13）。

## §2 模块定位与职责

| # | 职责 | 锚定 |
|---|---|---|
| EN-J1 | 订阅门禁客户端执行：每次模型调用前提交令牌过门禁，承接裁决 | X-SEQ-001/X-FORBID-002、TERM-CLOUD-001 |
| EN-J2 | AI 网关客户端：GatePassed 后经 AiGateway 发起调用，流式承接 | TERM-CLOUD-002、BR-SYS-DATA-001 |
| EN-J3 | BYOK 客户端边界：用户密钥一次性上送网关代持，客户端不落地 | TERM-CLOUD-004、X-FORBID-001 |
| EN-J4 | 用量与订阅状态呈现：服务端数据源展示 + 官网门户购买跳转 | 路线图 §7、REQ-CORE-008 |

> 边界澄清：**裁决/计量/计费全在服务端**（GW/LIC），ENT 只做提交、承接与呈现；会话令牌的持有
> 归 ACCT（ENT 消费认证态）；调用业务语义归 AG 等发起方（ENT 是通道不是发起方）。

## §3 能力范围（ENT v1）

**IN**：门禁提交与裁决承接（GateDenied 明确呈现 + 订阅引导）；网关调用通道（流式/错误统一
形状）；BYOK 录入与上送（不落地）；套餐用量/订阅状态展示；官网门户购买跳转。

**OUT**：裁决逻辑/计量/计费（服务端）；会话令牌持有（ACCT）；调用内容组装（AG 等发起方）；
桌面端内购买支付流程（v1 OUT）——购买/账单/用量明细/BYOK 密钥管理均归**个人账户门户**
（官网门户升级定位，2026-06-12 人类确认），桌面端仅展示 + 跳转。

## §4 代表性流程（契约级，APC §7）

**EN1 · 门禁与网关调用**（EN-J1/J2）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | AG 等发起方 | 调用意图 + 令牌（经 ACCT） | ENT 提交门禁（CLOUD-GATE） | allow/deny | 2 | deny→GateDenied：明确拒绝原因 + 订阅引导，不可绕过 |
| 2 | ENT | gateAllow | 经 AiGateway 发起（套餐计量或 BYOK 透传） | 流式回传 | 终态 | 流式错误→统一错误形状；断网→明确拒绝 |

**EN2 · BYOK 录入**（EN-J3）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | 用户 | 自带模型密钥 | ENT 经安全通道一次性上送网关代持；本地不持久化、不回显明文 | 网关代持确认 | 终态 | 上送失败→明确错误，本地无残留；BYOK 仍需有效订阅 |

## §5 模块级需求声明（REQ-ENT-*）

<!-- REQ
req_id: REQ-ENT-001
title: 订阅门禁客户端执行
owner_module: ENT
status: active
summary: 为一切模型调用提供唯一客户端门禁通道：每次调用前提交令牌过门禁（X-SEQ-001）；不缓存通过结论复用（X-FORBID-002）；GateDenied 呈现不可绕过的明确拒绝与订阅引导
source_doc: REQ-CORE-008、A-SYS-I-P-03（CLOUD-GATE、SM-SYS-01 门禁段）
acceptance: 无有效订阅时调用被拒且原因明确；不存在缓存通过/本地假通过路径（静态+联调核验）
open_issues: 无（契约已由 SYS 锁定，ENT 承接执行）
-->

<!-- REQ
req_id: REQ-ENT-002
title: AI 网关客户端
owner_module: ENT
status: active
summary: GatePassed 后经 AiGateway 发起模型调用（套餐计量/BYOK 透传由服务端区分），流式回传承接 BR-SYS-STATE-002 终态契约，失败回统一错误形状；客户端无 provider SDK、无直连 provider 路径
source_doc: A-SYS-I-P-03（CLOUD-INVOKE、BR-SYS-DATA-001）、X-FORBID-001
acceptance: 全部模型流量经网关（网络核验无 provider 直连）；流式完成/中断/错误终态正确；无 provider SDK 依赖（静态核验）
open_issues: 无
-->

<!-- REQ
req_id: REQ-ENT-003
title: BYOK 客户端边界
owner_module: ENT
status: active
summary: 用户自带密钥经安全通道一次性上送网关代持：客户端不持久化、不写日志、不回显明文、上送后本地即清；BYOK 透传不计套餐额度但仍需有效订阅（门禁不豁免）
source_doc: TERM-CLOUD-004、REQ-CORE-008（acceptance：BYOK 经网关透传且不计额度）、X-FORBID-001
acceptance: BYOK 密钥无本地持久化/日志残留（静态+运行核验）；无订阅时 BYOK 调用同样被拒
open_issues: 录入安全通道细节（直送网关 API）归 ENT G2/实现
-->

<!-- REQ
req_id: REQ-ENT-004
title: 用量与订阅状态呈现
owner_module: ENT
status: active
summary: 展示套餐用量与订阅状态，数据源唯一为服务端（客户端不自行计量作裁决依据）；购买/升级跳转官网门户，桌面端不内置支付
source_doc: 路线图 §7（购买在官网门户）、REQ-CORE-008
acceptance: 用量/状态与服务端一致；购买入口为门户跳转；桌面端无支付流程
open_issues: 用量展示粒度随云 LIC 契约（云端 APC 立项时定）
-->

## §6 模块边界与约束（边界闭合）

- **不可绕过三约束承接**：X-SEQ-001（先门禁）、X-FORBID-001（不持密钥）、X-FORBID-002
  （不缓存通过/无本地假通过）——ENT 是这三条在客户端的执行主体。
- **状态机**：不建新机（SM-SYS-01 门禁+网关段已覆盖，owner=SYS），ENT G2 声明承接段。
- **职责边界**：令牌归 ACCT、内容归发起方、裁决归服务端。
- **原料**：greenfield（台账 §4.15）。

## §7 G1 问题暴露清单

| 编号 | 分类 | 状态 | 处置 |
|---|---|---|---|
| ENT-01 | BOUNDARY_OPEN | 显式延期 | 用量展示粒度与刷新策略随云 LIC 契约（云端 APC 立项） |
| ENT-02 | BOUNDARY_OPEN | 显式延期 | BYOK 录入安全通道与网关代持 API 细节归 ENT G2/实现（联合云 GW） |
| ENT-03 | DESIGN_RISK | 显式延期 | 高频轻量调用（autocomplete）下门禁延迟与服务端裁决频率的性能权衡——不得以客户端缓存通过解决（X-FORBID-002 硬边界），方案归云 GW 侧（如服务端短时裁决凭证），G2/云端联合 |
| ENT-04 | ACCEPTANCE_MISSING | 部分关闭 | 定性验收已入 §5；量化（门禁延迟）延 G5 |

## §8 后续动作

1. 待人类宣布 ENT G1 闭合后进入 ENT G2（门禁/网关/BYOK 客户端规则）。
2. 云端 AUTH/GW/LIC 独立 APC 立项时联合解除 ENT-01/02/03。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（P1 尾部 greenfield，商业化关键路径）：ENT 订阅门控模块级 L1；声明 REQ-ENT-001~004（门禁客户端执行不可绕过 / 网关客户端无 provider 直连 / BYOK 不落地仍需订阅 / 用量呈现服务端数据源 + 门户购买跳转），承接 REQ-CORE-008/009 与 CLOUD-GATE/INVOKE 契约及 X-SEQ-001/X-FORBID-001/002；G1 暴露清单 ENT-01~04（autocomplete 门禁频率权衡显式暴露且锚定不缓存硬边界） |
| 2026-06-12 | v1.1 | 人类决策落档：个人账户门户定位确认（购买/账单/用量明细/BYOK 密钥管理归门户）；§3 OUT 同步更新；桌面端展示+跳转契约不变 |
