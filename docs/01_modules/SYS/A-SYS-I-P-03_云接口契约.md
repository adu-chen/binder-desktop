---
文档编号：SYS-I-P-03
文档状态：A
负责模块：SYS
文档职责：客户端↔云接口契约的技术设计 SSOT——认证会话 / 订阅门禁硬门 / AI 网关三类契约的链路、规则、跨模块约束、状态机与接口骨架；承接 REQ-SYS-005 / REQ-CORE-008
上游约束：A-SYS-M-D-01_系统底座功能需求描述.md（REQ-SYS-005）、A-CORE-C-D-01_产品概述.md（REQ-CORE-008）、A-CORE-C-P-01_APC.md（§4.2/§4.3 云契约、§6 状态机、§13 安全边界）、ADU.md（§3.2 块格式）
直接承接：ACCT（认证会话客户端）、ENT（订阅门禁/网关客户端）在其 G2 承接本契约的 RULE/CHAIN/CONSTRAINT/状态机；代码 @GOV 追溯本文规则
使用边界：本文定义**客户端侧**云接口契约（SSOT）；云端 AUTH/GW/LIC 内部实现归独立 APC，不在本文；具体 wire 格式/序列化/令牌算法等实现细节见 §11 债务与盲区，部分依赖云端 APC 协同确定
变更要求：契约语义、硬门顺序、安全约束变更须先经 APC 授权再回写本文，并同步刷新 ADUS（generate）与下游 ACCT/ENT 承接
治理成熟度：draft
---

# Binder 云接口契约（SYS · L2 技术设计 SSOT）

## 0. Terminology Registry

> 本项目首次接入、ADUS 术语注册表为空，以下为云契约核心概念首批注册（满足"跨 session 易漂移 +
> 代码有实体承载 + 命名混乱致规则误判"）。如人类对命名有异议，可在本区域调整后重跑 generate。

<!-- TERM
term_id: TERM-CLOUD-001
chains: CLOUD-GATE
zh: 订阅门禁
en: SubscriptionGate
forbidden: 订阅校验, 会员校验, 付费门槛, 鉴权门
-->
调用任何模型前由服务端裁决"当前会话是否具备有效订阅"的硬门；裁决权在服务端，客户端无绕过路径。

<!-- TERM
term_id: TERM-CLOUD-002
chains: CLOUD-INVOKE
zh: AI网关
en: AiGateway
forbidden: 模型代理, API网关, 代理服务, provider代理
-->
服务端统一代理模型调用的入口；服务端持密钥并计量；客户端不内置 provider SDK、不持密钥。

<!-- TERM
term_id: TERM-CLOUD-003
chains: CLOUD-AUTH
zh: 会话令牌
en: SessionToken
forbidden: 登录态, token, 认证令牌, 鉴权码
-->
客户端登录后持有的会话凭证，用于门禁与网关调用；过期须经续期契约刷新；失效则回落未认证态。

<!-- TERM
term_id: TERM-CLOUD-004
chains: CLOUD-INVOKE
zh: BYOK透传
en: ByokPassthrough
forbidden: 自带密钥, 自定义key, 用户密钥直连
-->
用户自带模型密钥时，密钥经网关透传由服务端代持调用、不计套餐额度，但仍需有效订阅；客户端不落地该密钥。

## §1 文档定位

本文是 SYS 模块 L2 技术设计层文档，是云接口契约的**唯一规则来源**（SSOT，APC §2/§3.2）。
承接链：

```
REQ-CORE-008(产品) → REQ-SYS-005(SYS L1) → REQ_MAP(§9) → 本文 RULE/CHAIN/CONSTRAINT/STATE_MACHINE → 代码@GOV
```

边界：本文只定义**客户端侧**契约——客户端如何认证、如何过订阅门禁硬门、如何经网关调用模型，
以及由此产生的跨模块硬约束。云端 AUTH/GW/LIC 的内部实现归独立 APC（APC §4.2），本文不写；
真正的 wire 格式/令牌算法/网关流式底层协议见 §11，部分需与云端 APC 协同后定稿。

## §2 契约角色与边界

| 角色 | 模块 | 在本契约中的职责 |
|---|---|---|
| 认证会话客户端 | ACCT | 承接 CLOUD-AUTH：登录、持有/续期会话令牌、登出 |
| 门禁/网关客户端 | ENT | 承接 CLOUD-GATE + CLOUD-INVOKE：调模型前过门禁硬门、经网关发起调用与流式回显 |
| AI 调用发起方 | AG / CBT / AST / KB / TPL | 经 ENT 发起模型调用，受 X-SEQ-001 约束（必先过门禁） |
| 云端实现 | AUTH / GW / LIC（独立 APC） | 认证、网关代理+计量、订阅/计费；本文仅消费其契约 |

> 主责模块归属说明：本文三条 CHAIN 的 `主责模块` 记为 SYS（契约编排归属）；运行时由 ACCT/ENT
> 在其 G2 承接实现，不重复声明 chain_id，只引用。

## §3 链路定义

<!-- CHAIN
chain_id: CLOUD-AUTH
主责模块: SYS
status: active
risk_tier: P0
integration_test: deferred: phase-5
-->
认证与会话续期链：客户端登录获取会话令牌 → 持有 → 过期续期 → 登出/失效回落未认证。

<!-- CHAIN
chain_id: CLOUD-GATE
主责模块: SYS
status: active
risk_tier: P0
integration_test: deferred: phase-5
-->
订阅门禁硬门链：模型调用前提交会话令牌 → 服务端裁决有效订阅 → 通过/拒绝；拒绝则不得发起模型调用。

<!-- CHAIN
chain_id: CLOUD-INVOKE
主责模块: SYS
status: active
risk_tier: P0
integration_test: deferred: phase-5
-->
网关调用链：通过门禁后经 AI 网关发起模型调用 → 套餐计量或 BYOK 透传 → 流式回显 → 完成/错误。

**链路边界（APC §7 流程，模块边界标注）**：
`AG/CBT/… → ENT(门禁客户端) → [CLOUD-GATE 服务端硬门] → ENT(网关客户端) → [CLOUD-INVOKE AiGateway] → 流式回显`；
会话凭证由 ACCT 经 CLOUD-AUTH 提供。任一步失败按 §8 异常恢复处理。

## §4 规则定义（RULE）

> rule_id 采用 BR-SYS-[域]-[序号]；域取 ADU §3.1 内置域（未新增扩展域）。

<!-- RULE
rule_id: BR-SYS-STATE-001
主链路: CLOUD-AUTH
域: STATE
-->
会话令牌（TERM-CLOUD-003）生命周期：登录成功进入已认证态并持有令牌；令牌过期前可经续期契约刷新；
续期失败或登出则回落未认证态。未认证态下一切模型调用（CLOUD-GATE/CLOUD-INVOKE）一律拒绝。
客户端只持会话令牌，绝不持有模型 provider 密钥。

<!-- RULE
rule_id: BR-SYS-VERIFY-001
主链路: CLOUD-GATE
域: VERIFY
-->
订阅门禁（TERM-CLOUD-001）硬门：每次模型调用前，客户端经门禁契约提交会话令牌，由服务端裁决
是否具备有效订阅。裁决结果为"拒绝"时，客户端不得发起任何模型调用，并按统一错误形状向上层返回
不可绕过的拒绝原因。门禁裁决以服务端为准，客户端不得本地缓存"通过"结论以跳过后续校验。

<!-- RULE
rule_id: BR-SYS-DATA-001
主链路: CLOUD-INVOKE
域: DATA
-->
AI 网关（TERM-CLOUD-002）调用契约：模型调用统一经网关封装；套餐档调用由网关计量计费；
BYOK 透传（TERM-CLOUD-004）由网关代持密钥调用、不计套餐额度但仍需通过 BR-SYS-VERIFY-001。
客户端不内置 provider SDK、不在请求中明文携带模型 provider 密钥落地到本地存储。

<!-- RULE
rule_id: BR-SYS-DATA-004
主链路: CLOUD-INVOKE
域: DATA
-->
网关内容不留存承诺（2026-06-12 人类确认，产品承诺级）：模型调用内容（上下文/提示/回传）在
AiGateway 仅作瞬时过境转发，服务端不得持久化调用内容，仅留存脱敏计量元数据（用量/时间/模型档，
不含正文）；BYOK 密钥经网关加密保管（代持），与用户身份数据分离存储。本规则是云端 GW 独立 APC
的前置约束：云端实现不得弱化本承诺，弱化即产品边界变更须 APC 授权。本规则将客户端「内容不上行
存储」（REQ-CORE-001）的语义延伸至服务端过境环节。

<!-- RULE
rule_id: BR-SYS-OBS-001
主链路: CLOUD-AUTH, CLOUD-GATE, CLOUD-INVOKE
域: OBS
-->
云调用统一错误形状：认证、门禁、网关三类失败均以统一错误形状回传（错误码 + 用户可读消息 +
是否可重试 + 是否需重新认证），与 IPC 统一错误形状（REQ-SYS-001）对齐，便于上层一致处理与埋点。

## §5 跨模块约束（CONSTRAINT）

<!-- CONSTRAINT
constraint_id: X-FORBID-001
摘要: 客户端不得持有或落地模型 provider 密钥，密钥仅服务端（AiGateway）持有
涉及模块: SYS, ENT, ACCT, AG, CBT, AST, KB, TPL
链路影响: CLOUD-INVOKE, CLOUD-GATE
-->

<!-- CONSTRAINT
constraint_id: X-SEQ-001
摘要: 任何模型调用前必须先过订阅门禁硬门（CLOUD-GATE），无有效订阅一律拒绝，禁止跳过门禁直连网关
涉及模块: ENT, AG, CBT, AST, KB, TPL
链路影响: CLOUD-GATE, CLOUD-INVOKE
-->

<!-- CONSTRAINT
constraint_id: X-FORBID-002
摘要: 订阅门禁不得存在客户端绕过路径，裁决以服务端为准，客户端不得本地缓存通过结论跳过校验
涉及模块: ENT, SYS
链路影响: CLOUD-GATE
-->

## §6 状态机（账号会话与订阅门禁，APC §6 强制核心链）

<!-- STATE_MACHINE
machine_id: SM-SYS-01
owner_module: SYS
status: active
summary: 会话认证与单次模型调用的订阅门禁硬门状态机（ACCT/ENT 运行时承接）
source_type: design
chain: CLOUD-AUTH, CLOUD-GATE, CLOUD-INVOKE
rules: BR-SYS-STATE-001, BR-SYS-VERIFY-001, BR-SYS-DATA-001
initial_state: Unauthenticated
terminal_states: GateDenied, InvokeCompleted, InvokeError
-->

- **machine_id**：SM-SYS-01　**owner_module**：SYS（运行时承接：ACCT 会话段 / ENT 门禁+网关段）
- **scope**：客户端一次"登录态维持 + 单次模型调用门控"的完整生命周期
- **context**：会话令牌、令牌有效期、门禁裁决结果、调用模式（套餐 / BYOK 透传）
- **states**：Unauthenticated、Authenticated、GateChecking、GatePassed、Invoking、InvokeCompleted、GateDenied、InvokeError
- **events**：login、loginOk、loginFail、startModelCall、gateAllow、gateDeny、invokeStart、invokeStreamEnd、invokeFail、sessionExpire、logout
- **transitions（主表）**：

| from | event | guard | to | action |
|---|---|---|---|---|
| Unauthenticated | login | — | Authenticated | 持有 SessionToken（loginOk）|
| Unauthenticated | loginFail | — | Unauthenticated | 统一错误形状返回 |
| Authenticated | startModelCall | 令牌未过期 | GateChecking | 提交令牌过门禁 |
| GateChecking | gateAllow | 服务端裁决有效订阅 | GatePassed | — |
| GateChecking | gateDeny | 无有效订阅 | GateDenied | 拒绝原因（不可绕过）|
| GatePassed | invokeStart | — | Invoking | 经网关发起（计量/BYOK 透传）|
| Invoking | invokeStreamEnd | — | InvokeCompleted | 流式回显完成 |
| Invoking | invokeFail | — | InvokeError | 统一错误形状（可重试标记）|
| Authenticated/GateChecking/Invoking | sessionExpire | — | Unauthenticated | 令牌失效，要求重认证 |
| Authenticated | logout | — | Unauthenticated | 清会话 |

- **error_states**：InvokeError（recovery：按可重试标记回 GatePassed 重试，或回 Authenticated）；sessionExpire 走 recovery 回 Unauthenticated 重认证
- **recovery_paths**：InvokeError→（可重试）GatePassed；任意态 sessionExpire→Unauthenticated→login
- **forbidden_transitions**：
  - Unauthenticated → Invoking（未认证不得调用）
  - Authenticated → Invoking（未过门禁不得调用）
  - GateDenied → Invoking（被拒不得绕过门禁）
  - GateChecking → Invoking（未取得 gateAllow 不得调用）
- **acceptance_criteria**：无 gateAllow 不存在通往 Invoking 的路径；GateDenied 为终态且不可转 Invoking；未认证态一切调用被拒
- **test_matrix**：见 §10（强制转移测试覆盖上述 transitions 与 forbidden_transitions）

## §7 接口契约骨架（契约级，非 wire 格式）

> 定义操作与语义保证；具体签名落 IPC 协议（A-SYS-I-P-02），字段级 schema 见 §11 盲区。

| 契约操作 | 链路 | 语义保证 |
|---|---|---|
| 认证·登录 | CLOUD-AUTH | 成功后持有会话令牌；失败返回统一错误形状（BR-SYS-STATE-001） |
| 认证·续期 | CLOUD-AUTH | 令牌过期前刷新；失败回落未认证态 |
| 认证·登出 | CLOUD-AUTH | 清除会话，回未认证态 |
| 门禁·校验 | CLOUD-GATE | 提交令牌→服务端裁决；返回 allow/deny + 不可绕过的拒绝原因（BR-SYS-VERIFY-001、X-FORBID-002）|
| 网关·调用 | CLOUD-INVOKE | 仅在 GatePassed 后可发起；套餐计量或 BYOK 透传；流式回显（BR-SYS-DATA-001、X-SEQ-001）|

## §8 异常恢复（APC §5 G2）

- **认证失败 / 令牌过期**：进入 Unauthenticated，上层提示重新登录；不得以过期令牌发起门禁或调用。
- **门禁拒绝**：终态 GateDenied，向上层返回不可绕过拒绝原因；不得本地"假通过"（X-FORBID-002）。
- **网关错误**：InvokeError 携可重试标记；可重试者回 GatePassed 重试有限次，否则上抛统一错误形状。
- **流式中断**：按 IPC 流式契约（REQ-SYS-001）的中断语义处理，状态保持可恢复，不产生半落盘副作用（落盘归 WS/DE，不在本契约）。

## §9 需求到规则映射（REQ_MAP）

<!-- REQ_MAP
req_id: REQ-SYS-005
target_type: CHAIN
target_id: CLOUD-AUTH
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-005
target_type: CHAIN
target_id: CLOUD-GATE
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-005
target_type: CHAIN
target_id: CLOUD-INVOKE
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-005
target_type: RULE
target_id: BR-SYS-VERIFY-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-005
target_type: RULE
target_id: BR-SYS-DATA-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-005
target_type: RULE
target_id: BR-SYS-DATA-004
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-005
target_type: RULE
target_id: BR-SYS-STATE-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-005
target_type: CONSTRAINT
target_id: X-FORBID-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-005
target_type: CONSTRAINT
target_id: X-SEQ-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-005
target_type: CONSTRAINT
target_id: X-FORBID-002
status: mapped
-->

## §10 测试方案与验收

- **状态机转移测试矩阵（强制，APC §6/§10）**：覆盖 §6 全部 transitions 与 forbidden_transitions；
  重点断言：无 gateAllow 不可达 Invoking；GateDenied 不可转 Invoking；Unauthenticated/Authenticated 直达 Invoking 被拒。
- **规则 covers**：BR-SYS-VERIFY-001 / BR-SYS-DATA-001 / BR-SYS-STATE-001 / BR-SYS-OBS-001 各有 `// covers: BR-SYS-*` 断言（实现于 ENT/ACCT G4）。
- **约束验证**：X-FORBID-001（客户端无密钥落地路径）、X-SEQ-001（调用前必过门禁）、X-FORBID-002（无本地绕过）在集成测试中验证。
- **验收标准**：无有效订阅时模型调用被拒；客户端不存明文密钥；门禁无客户端绕过路径；BYOK 经网关透传且不计额度——对齐 REQ-CORE-008 / REQ-SYS-005 验收。
- **NFR**：门禁/首字延迟等量化指标延 G5 标定（APC §9）。

## §11 技术债务与盲区（诚实声明）

- 令牌算法/格式（JWT vs 不透明令牌）、续期机制细节：依赖云端 AUTH APC，**待协同定稿**，本文只锁语义。
- 网关流式底层协议（SSE/分片/取消信号）字段级 schema：归 A-SYS-I-P-02 + 云端 GW APC。
- 门禁裁决响应的具体结构与计量回执格式：依赖 LIC APC。
- BYOK 透传的密钥传输安全细节（传输期保护、是否会话级缓存）：需安全评审，列 G5 安全门。
- 以上盲区不阻断本契约 G2 的语义与硬约束闭合；定稿前下游 ENT/ACCT 对契约打桩开发（路线图 §4 P3 注记）。

## §12 后续动作

1. **先行落地 `A-SYS-C-T-01`（系统总体架构 + MODULE 注册块）再跑 `governance:generate`**——
   本文引用 SYS/ACCT/ENT 等模块编码，注册块归 A-SYS-C-T-01（REQ-SYS-004）；generate 前本文规则 ID 为 generated 态。
2. generate 后核对 ADUS：TERM-CLOUD-001~004、CLOUD-* 链路、BR-SYS-*、X-* 均进注册表。
3. ACCT G2 承接 CLOUD-AUTH + 会话段状态机；ENT G2 承接 CLOUD-GATE/CLOUD-INVOKE + 门禁/网关段。
4. §11 盲区随云端 APC 协同定稿后回写本文并刷新下游。
5. 术语命名如人类有异议，于 §0 调整后重跑 generate（首次接入，注册表为空，调整成本低）。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-10 | v1.0 | 初版（SYS G2）：云接口契约 SSOT；注册 TERM-CLOUD-001~004；定义 CLOUD-AUTH/GATE/INVOKE 三链路、BR-SYS-STATE/VERIFY/DATA/OBS-001 四规则、X-FORBID-001/SEQ-001/FORBID-002 三约束、SM-SYS-01 会话与订阅门禁状态机；REQ_MAP 承接 REQ-SYS-005；测试矩阵/验收/债务盲区；锁定 A-SYS-C-T-01 须先于 generate |
| 2026-06-12 | v1.1 | 人类确认产品承诺升格：新增 BR-SYS-DATA-004（网关内容不留存——调用内容瞬时过境不持久化、仅留脱敏计量元数据；BYOK 密钥加密保管与用户身份数据分离；作为云端 GW 独立 APC 前置约束，弱化即产品边界变更）；REQ_MAP 补 REQ-SYS-005→BR-SYS-DATA-004 |
