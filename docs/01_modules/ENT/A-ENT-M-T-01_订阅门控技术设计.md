---
文档编号：ENT-M-T-01
文档状态：A
负责模块：ENT
文档职责：ENT 订阅门控模块级 L2 技术设计（规则 SSOT）——门禁客户端执行规则、网关客户端通道规则、BYOK 客户端边界规则、用量呈现规则；承接 REQ-ENT-001~004
上游约束：A-ENT-M-D-01_订阅门控功能需求描述.md（REQ-ENT-001~004）、A-SYS-I-P-03_云接口契约.md（CLOUD-GATE/INVOKE、SM-SYS-01 门禁+网关段、BR-SYS-VERIFY-001/DATA-001/OBS-001、X-SEQ-001/X-FORBID-001/002、TERM-CLOUD-001/002/004）、A-SYS-I-P-02_IPC协议.md（BR-SYS-STATE-002、BR-SYS-OBS-002）、A-ACCT-M-T-01_账号会话技术设计.md（认证态消费）、A-CORE-C-P-01_APC.md（§4.3/§13）、ADU.md（§3）
直接承接：本文 RULE 为 ENT 规则 SSOT，代码经 @GOV 映射；规则主链路引用 CLOUD-GATE/CLOUD-INVOKE（不重复声明 chain_id）；AG/CBT/AST/KB/TPL 模型调用经本通道
使用边界：本文是 ENT 规则 SSOT；云协议/状态机/裁决语义归 A-SYS-I-P-03 与服务端（承接不重定义）；令牌归 ACCT；不注册新 TERM/CHAIN/CONSTRAINT、无新状态机（§5 说明）
变更要求：任何弱化门禁/密钥边界的变更为 APC §13 禁区（不得自主决定）；用量呈现变更随云 LIC 契约
治理成熟度：draft
---

# Binder 订阅门控技术设计（ENT · L2 技术设计）

## §1 文档定位

```
REQ-ENT-001~004（ENT L1）→ 本文（SSOT）→ 代码(@GOV)
本文规则 ──主链路引用──→ CLOUD-GATE / CLOUD-INVOKE（chain_id 归 A-SYS-I-P-03）
本文 ──运行时承接──→ SM-SYS-01 门禁+网关段（GateChecking→…→Invoking→终态）
```

**APC §13 禁区重申**：在客户端硬编码/落地密钥、绕过订阅门禁，是 AI 不得自主决定的事项；本文
全部规则在该禁区之内收紧执行，不存在弱化路径。

## §2 门禁客户端执行（CLOUD-GATE）

<!-- RULE
rule_id: BR-ENT-VERIFY-001
主链路: CLOUD-GATE
域: VERIFY
-->
ENT 是模型调用的唯一客户端门禁通道：每一次模型调用（含高频轻量调用）必须提交令牌经服务端
裁决（X-SEQ-001）；客户端不得缓存"通过"结论复用、不得本地假通过（X-FORBID-002）——高频场景
的延迟优化只能采用服务端签发的短时裁决凭证类方案（归云 GW 设计），客户端侧无权自行放行；
GateDenied 为服务端明确拒绝的终态，向发起方回不可绕过的拒绝原因；只有明确订阅拒绝才呈现订阅引导。
网络不可达、认证/门禁超时、非授权类 HTTP 异常、畸形或未知裁决返回统一调用错误，不冒充服务端拒绝；
两类路径均不得调用模型，不排队或自动重试任务。HTTP 401/403 或合法 denied 属于明确拒绝；429/5xx
标记可显式重试，其他非成功 HTTP 标记不可重试。认证凭据最多重新获取一次（401 或 reauth_required），
仍要求重认证时拒绝；即便同时出现 passed 也不得放行。认证接口网络失败不得降格为缺令牌。
dev 认证/门禁请求暂定 15 秒，覆盖响应头及响应体；Provider 标定另验。显式新请求必须重新过门禁，
不得复用失败前的通过结论；保留此前工具证据，不自动重复操作。

## §3 网关客户端通道（CLOUD-INVOKE）

<!-- RULE
rule_id: BR-ENT-STATE-001
主链路: CLOUD-INVOKE
域: STATE
-->
模型调用通道必须满足：仅在 GatePassed 后发起（SM-SYS-01 forbidden 对齐）；全部模型流量经
AiGateway，客户端不内置任何 provider SDK、不存在 provider 直连路径（X-FORBID-001 执行）；
流式回传承接 BR-SYS-STATE-002（完成/中断/错误三类终态，用户可取消）；失败回统一错误形状
（BR-SYS-OBS-001/002 对齐，含可重试标记与 reauth_required）；套餐计量与 BYOK 透传的区分由
服务端裁决，客户端不据本地状态切换计量模式。

## §4 BYOK 客户端边界与用量呈现（CLOUD-INVOKE / CLOUD-GATE）

<!-- RULE
rule_id: BR-ENT-DATA-001
主链路: CLOUD-INVOKE
域: DATA
-->
BYOK 密钥在客户端必须满足：录入后经安全通道一次性上送 AiGateway 代持，本地不持久化（含
内存外任何介质）、不写日志/错误上报、不回显明文，上送完成即清内存引用；上送失败本地无残留；
BYOK 调用仍经门禁（无有效订阅一律拒绝，透传不计额度但不豁免门禁，TERM-CLOUD-004）。

<!-- RULE
rule_id: BR-ENT-DATA-002
主链路: CLOUD-GATE
域: DATA
-->
用量与订阅状态呈现的数据源唯一为服务端（LIC 经网关下发）：客户端可本地缓存展示数据但不得将
本地缓存/自行计量用作任何门禁或调用决策依据；购买/升级/续费一律跳转官网门户，桌面端不内置
支付流程；订阅状态变更（升级/到期）以服务端裁决为准即时生效于下一次门禁。

## §5 不新增注册项说明

- **CHAIN**：引用 CLOUD-GATE / CLOUD-INVOKE（A-SYS-I-P-03 注册），按承接原则不重复声明。
- **TERM**：SubscriptionGate/AiGateway/ByokPassthrough 已由 SYS 注册（TERM-CLOUD-001/002/004）。
- **CONSTRAINT**：X-SEQ-001 / X-FORBID-001 / X-FORBID-002 已锁三条硬边界，本文为其客户端执行
  规则，无新增。
- **STATE_MACHINE**：SM-SYS-01 门禁+网关段已覆盖（owner=SYS），ENT 承接运行时不建新机。

## §6 需求到规则映射（REQ_MAP）

<!-- REQ_MAP
req_id: REQ-ENT-001
target_type: RULE
target_id: BR-ENT-VERIFY-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ENT-002
target_type: RULE
target_id: BR-ENT-STATE-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ENT-003
target_type: RULE
target_id: BR-ENT-DATA-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ENT-004
target_type: RULE
target_id: BR-ENT-DATA-002
status: mapped
-->

## §7 测试与验收

- **门禁不可绕过**（BR-ENT-VERIFY-001，G4/G5 强制）：静态无缓存通过/假通过路径；无订阅时一切
  模型调用（含 autocomplete）被拒；断网明确错误且零后续模型调用；转移测试复用 A-SYS-I-P-03 §10 门禁段矩阵。
- **通道纯净**（BR-ENT-STATE-001）：依赖清单无 provider SDK（静态）；网络核验全部模型流量经
  网关、无 provider 直连；流式三终态正确、可取消。
- **BYOK 无残留**（BR-ENT-DATA-001）：静态+运行核验密钥无持久化/日志/回显；无订阅时 BYOK 被拒。
- **呈现数据源**（BR-ENT-DATA-002）：用量与服务端一致；本地缓存不参与任何决策路径（静态核验）；
  购买入口仅门户跳转。
- 量化（门禁延迟）延 G5（ENT-04）。

## §8 技术债务与盲区（诚实声明）

- **高频调用门禁延迟**（ENT-03）：解法限定在服务端短时裁决凭证类方案（客户端无权放行），具体
  设计归云 GW 独立 APC，立项时联合；在此之前 autocomplete 按逐次裁决承担延迟（节流已由
  BR-AG-STATE-003 缓解频率）。
- 用量展示粒度/刷新（ENT-01）与 BYOK 上送 API（ENT-02）随云 LIC/GW 契约细化。
- 服务端裁决凭证若引入，须回写本文与 A-SYS-I-P-03 复核 X-FORBID-002 一致性（凭证=服务端签发
  短时授权，非客户端缓存，边界须明示）。

## §9 后续动作

1. 注册（generate）：BR-ENT-VERIFY-001 / BR-ENT-STATE-001 / BR-ENT-DATA-001/002、REQ_MAP。
2. 待人类宣布 ENT G2 闭合——**至此 15 个客户端模块文档层全部齐备**。
3. 云端 AUTH/GW/LIC 独立 APC 立项时联合解除 ENT-01/02/03。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-10-06 | v1.2 | 经人类确认区分明确授权拒绝与认证/门禁调用失败；失败关闭不放宽门禁，限定一次凭据刷新及新请求重新裁决。 |
| 2026-06-12 | v1.0 | 初版（ENT G2，greenfield 商业化关键路径收尾）：门禁客户端执行（逐次裁决、禁缓存通过、断网即拒）、网关通道（无 provider SDK/直连、流式三终态）、BYOK 边界（一次性上送、零残留、不豁免门禁）、用量呈现（服务端唯一数据源、门户购买）；主链路引用 CLOUD-GATE/INVOKE 不重复声明；零新增注册项（§5 说明）；ENT-03 高频门禁延迟解法限定服务端凭证类并锚定 X-FORBID-002；REQ_MAP（REQ-ENT-001~004） |
| 2026-08-07 | v1.1 | MODEL-CONFIG-01（DOC-SYNC-01 补登）：BYOK 管理客户端实现闭合——`ent::byok` 与 `ent::gateway` 落地 BYOK 代持提交与校验，前端模型配置界面提供 BYOK 录入/管理（来源分层 preset/byok，见 A-AG-M-T-01 §6.1 AG-MODEL-CATALOG）；零残留与不豁免门禁口径不变（BR-ENT-DATA-001）。 |
