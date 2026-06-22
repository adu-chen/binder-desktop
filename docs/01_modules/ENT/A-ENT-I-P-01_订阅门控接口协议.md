---
文档编号：ENT-I-P-01
文档状态：A
负责模块：ENT
文档职责：ENT 订阅门控模块级接口协议——门禁提交、网关调用、BYOK 上送、用量/订阅状态查询、门户跳转；承接 SM-SYS-01 门禁+网关段
上游约束：A-SYS-I-P-02_IPC协议.md（骨架、流式终态）、A-SYS-I-P-03_云接口契约.md（CLOUD-GATE/INVOKE、SM-SYS-01 门禁+网关段、X-SEQ-001/X-FORBID-001/002、TERM-CLOUD-001/002/004）、A-ENT-M-T-01_订阅门控技术设计.md（BR-ENT-* 规则）、A-ACCT-I-P-01（消费认证态）、ADU.md
直接承接：实现期 ENT command 注册；AG/CBT/AST/KB/TPL 模型调用经本通道（唯一）；门禁/网关经云端 GW（A-SYS-I-P-03 契约）；用量经 LIC
使用边界：声明语义级签名；云协议字段归 A-SYS-I-P-03（不重定义）；裁决/计量/计费归服务端；不重定义 SM-SYS-01；不新增注册项；购买在门户（桌面端仅跳转）
变更要求：任何弱化门禁/密钥边界变更属 APC §13 禁区（不得自主）；用量呈现随云 LIC 契约
治理成熟度：draft
---

# Binder 订阅门控接口协议（ENT · L2 接口层）

## §1 文档定位

```
A-SYS-I-P-02（骨架）+ A-SYS-I-P-03（CLOUD-GATE/INVOKE 契约）→ 本文（ENT 门禁/网关/BYOK 接口）→ 实现 + @GOV(追溯 A-ENT-M-T-01)
承接 SM-SYS-01 门禁+网关段；是 AG/CBT/AST/KB/TPL 一切模型调用的唯一客户端通道
```

**接口层硬约束**：本文不暴露任何使客户端可绕过门禁/本地假通过的入口（X-FORBID-002）；
provider 密钥无任何 command 经手（X-FORBID-001）。

## §2 门禁与网关接口（CLOUD-GATE/INVOKE / SM-SYS-01）

| command/事件 | 参数（语义） | 结果 | 特有错误/状态 | 归属 |
|---|---|---|---|---|
| `ent.invoke_model` | 调用请求（发起方/上下文，令牌经 ACCT 内部取） | 流式句柄（先门禁后网关） | **gate_denied**（无订阅/断网，不可绕过 + 订阅引导）；流式错误统一形状 | X-SEQ-001/BR-ENT-VERIFY-001（门禁前置）；BR-ENT-STATE-001（网关流式三终态） |
| `ent.invoke_stream`（事件） | 调用句柄（订阅） | 流式分片（完成/中断/错误终态，可取消） | — | BR-SYS-STATE-002 对齐 |

> `ent.invoke_model` 是 AG（ag.start_turn/request_assist）、CBT 步骤、AST/KB 等模型调用的底层通道；
> 套餐计量/BYOK 透传由服务端区分（客户端不据本地状态切换）。

## §3 BYOK 接口（CLOUD-INVOKE / BR-ENT-DATA-001）

| command | 参数（语义） | 结果 | 特有错误 | 归属 |
|---|---|---|---|---|
| `ent.submit_byok` | 用户密钥 | 网关代持确认（本地不持久化/不日志/不回显、上送即清） | 上送失败→本地无残留 | BR-ENT-DATA-001（仍需有效订阅，不豁免门禁）；密钥管理（查看脱敏/轮换/删除）在门户 |

## §4 用量与门户接口（CLOUD-GATE / BR-ENT-DATA-002）

| command | 参数（语义） | 结果 | 特有错误 | 归属 |
|---|---|---|---|---|
| `ent.get_subscription_state` | — | 订阅状态 + 用量（服务端唯一数据源，本地仅展示不作决策） | — | BR-ENT-DATA-002（与裁决依据同源 LIC） |
| `ent.open_portal` | 目标（购买/账单/账户） | 门户跳转（系统浏览器/外链） | — | 桌面端不内置支付，购买在门户 |

## §5 测试与验收

- **门禁不可绕过**（BR-ENT-VERIFY-001，G4/G5 强制）：静态无缓存通过/假通过路径；无订阅时
  invoke_model 返 gate_denied（含 autocomplete 高频路径）；断网明确拒绝。
- **通道纯净**：依赖无 provider SDK（静态）；网络核验全部模型流量经网关、无 provider 直连。
- **BYOK 零残留**：submit_byok 后无持久化/日志/回显（静态+运行核验）；无订阅 BYOK 被拒。
- **呈现数据源**：用量与服务端一致、本地不参与决策；open_portal 仅跳转；与云端 GW/LIC 契约测试。

## §6 债务

- 高频门禁延迟（ENT-03）：解法限服务端短时裁决凭证（云端 REQ-CLOUD-002/CLOUD-04），客户端无权
  放行——凭证机制若引入须回写 A-SYS-I-P-03 并复核 X-FORBID-002；在此前 autocomplete 按逐次裁决
  （节流已缓解，BR-AG-STATE-003）。
- 用量粒度（ENT-01）、BYOK 上送 API（ENT-02）随云 LIC/GW 契约细化。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（实现前置件，商业化关键路径收尾）：ENT 接口协议——invoke_model（门禁前置+网关流式，唯一模型调用通道）/invoke_stream/submit_byok（零残留不豁免门禁）/get_subscription_state（服务端数据源）/open_portal（门户跳转）；X-SEQ-001/X-FORBID-001/002 接口层固化无绕过入口；零新增注册项；债务（高频凭证锚定 X-FORBID-002、用量粒度/BYOK API 延云端） |
