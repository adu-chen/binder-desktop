---
文档编号：ENT-X-L-01
文档状态：A
负责模块：ENT
文档职责：ENT 订阅门控 G3 开发计划——承接型 Plan Item，承接门禁+网关段运行时（SM-SYS-01）与 BR-ENT-* 规则、三硬约束（X-SEQ-001/X-FORBID-001/002），明确文件范围/@GOV/依赖云端的 C 类/验收；商业化关键路径客户端侧
上游约束：A-CORE-X-L-02（P1 尾/P5）、APC §5/§6/§12/§13（门禁不得绕过/密钥不得落地）、A-ENT-M-T-01、A-ENT-I-P-01、A-SYS-I-P-03（CLOUD-GATE/INVOKE/SM-SYS-01/BR-SYS-DATA-004）、A-SYS-X-L-01（PI-SYS-04）、A-CLOUD-C-D-01（云端 GW/LIC）、A-CORE-X-R-01
直接承接：本计划驱动 ENT 实现；@GOV 追溯 BR-ENT-*；AG/CBT/AST/KB/TPL 模型调用经本通道
使用边界：L3 承接型 Plan Item；裁决/计量/计费归服务端（打桩实现，真联调 P5）；@GOV 在实现代码
变更要求：弱化门禁/密钥边界涉 APC §13 禁区不得自主；用量随云 LIC 契约
治理成熟度：draft
---

# Binder 订阅门控开发计划（ENT · L3）

## §1 文档定位

ENT 是 AI 链唯一客户端门禁/网关通道、**商业化关键路径核心**。承接 SM-SYS-01 门禁+网关段（依
PI-SYS-04 骨架）。**云端依赖（C 类）**：裁决/计量/计费/BYOK 代持归云端 GW/LIC（CLOUD-02/04），
客户端按 A-SYS-I-P-03 契约**打桩实现**，真联调延 P5。**APC §13 禁区**：门禁绕过、密钥落地为
AI 不得自主决定项，本计划在禁区内收紧。

**1.0 EP 优先级（2026-07-17）**：1.0 Experience Preview 将真实 SubscriptionGate、AiGateway
DeepSeek 托管模型通道、BYOK 代持和用量/订阅展示提升为上线阻断项；自部署 Binder 预设模型、
多模态模型矩阵与复杂 ModelRouter 后置，不改变 ENT 的客户端边界。

## §2 Plan Item 清单

### PI-ENT-01　门禁前置与网关调用通道　✅ G4（gateway.rs 调用通道 SM：Streaming 仅 GatePassed 后可达/流式三终态/断网拒绝 BR-ENT-STATE-001 + submit_for_verdict 服务端唯一裁决无本地假通过 BR-ENT-VERIFY-001，audit 绿）
- **承接**：REQ-ENT-001/002 / SM-SYS-01 门禁+网关段 / BR-ENT-VERIFY-001（门禁前置）/
  BR-ENT-STATE-001（网关流式）/ X-SEQ-001 / X-FORBID-001 / X-FORBID-002。
- **文件范围**：`src-tauri/src/ent/gateway.rs`（提交门禁→网关流式发起；无 provider SDK/直连）；
  `ent.invoke_model/invoke_stream`（AG/CBT/AST/KB/TPL 模型调用唯一通道）。
- **@GOV**：门禁前置无绕过 `BR-ENT-VERIFY-001`/`X-SEQ-001`/`X-FORBID-002`；网关通道无 provider
  直连 `BR-ENT-STATE-001`/`X-FORBID-001`。
- **C 类依赖**：门禁裁决响应结构、网关流式协议（CLOUD-02，云端 GW）→ 打桩实现；高频门禁延迟
  凭证方案（ENT-03/CLOUD-04）→ 云端解，**G4 前 autocomplete 逐次裁决（不得客户端缓存通过）**。
- **验收（强制，APC §6/§12/§13）**：**静态可查无缓存通过/假通过路径、无 provider SDK/直连
  （网络核验）**；无订阅时 invoke_model 返 gate_denied（含 autocomplete 高频路径）；断网明确拒绝；
  流式三终态正确可取消；SM-SYS-01 门禁+网关段转移矩阵（无 gateAllow 不可达 Invoking）。

### PI-ENT-02　BYOK 代持与用量呈现　✅ G4（byok.rs 一次性上送 move 消费+就地清零零残留/不豁免门禁 BR-ENT-DATA-001 + usage.rs 用量仅展示门禁决策不用本地缓存/桌面端不内置支付 BR-ENT-DATA-002，audit 绿）
- **承接**：REQ-ENT-003/004 / BR-ENT-DATA-001（BYOK 零残留）/ BR-ENT-DATA-002（用量服务端数据源）。
- **文件范围**：`src-tauri/src/ent/byok.rs`（一次性上送代持、本地零残留）；`src/`（用量/订阅展示
  + 门户跳转）；`ent.submit_byok/get_subscription_state/open_portal`。
- **@GOV**：BYOK 零残留不豁免门禁 `BR-ENT-DATA-001`；用量服务端唯一数据源不参与决策
  `BR-ENT-DATA-002`。
- **C 类依赖**：BYOK 上送 API（ENT-02/CLOUD-04，云端 GW）→ 打桩；用量粒度（ENT-01/CLOUD-02，
  云端 LIC）→ 打桩。
- **验收（强制）**：**submit_byok 后本地无持久化/日志/回显（静态+运行核验）**；无订阅 BYOK 被拒；
  用量与服务端一致、本地缓存不参与任何决策（静态核验）；open_portal 仅跳转、桌面端无支付流程。

## §3 横切验收

audit 绿；@GOV 追溯 BR-ENT-*；SM-SYS-01 门禁+网关段转移矩阵；`// covers: BR-ENT-*`（4 条）有
断言；**门禁不可绕过 + BYOK 零残留 + 无 provider 直连为 G4/G5 强制安全核验项**（APC §9 安全门）；
AI 调用→门禁→网关→流式 G5 端到端（与 AG/云端联调）。

## §4 承接的债务（A-CORE-X-R-01 §5）

| 债务 | 类 | PI | 处置 |
|---|---|---|---|
| 门禁裁决/网关协议（CLOUD-02） | C | PI-ENT-01 | 云端 GW 打桩，真联调 P5 |
| 高频门禁延迟凭证（ENT-03/CLOUD-04） | C | PI-ENT-01 | 云端解；G4 前逐次裁决（锚 X-FORBID-002） |
| BYOK 上送 API（ENT-02/CLOUD-04） | C | PI-ENT-02 | 云端 GW 打桩 |
| 用量粒度（ENT-01/CLOUD-02） | C | PI-ENT-02 | 云端 LIC 打桩 |
| 门禁延迟 NFR | A | 全 PI | G5 标定 |
| 1.0 EP 云端最小闭环 | C | 全 PI | DeepSeek 托管模型通道、SubscriptionGate、BYOK、用量展示和管理员端最小能力形成生产链路 |

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-07-17 | v1.1 | 对齐 1.0 Experience Preview：真实 SubscriptionGate、AiGateway DeepSeek 托管模型通道、BYOK 代持和用量/订阅展示提升为上线阻断项；自部署 Binder 预设模型、多模态模型矩阵与复杂 ModelRouter 后置，不改变 ENT 客户端边界 |
| 2026-06-12 | v1.0 | 初版（G3，P1 尾/商业化关键路径收尾，**G3 末件**）：ENT 2 条 Plan Item（门禁前置与网关通道 / BYOK 代持与用量呈现），承接 SM-SYS-01 门禁+网关段 + 4 规则 + X-SEQ-001/X-FORBID-001/002；云端依赖打桩实现真联调 P5；门禁不可绕过/BYOK 零残留/无 provider 直连为 G4/G5 强制安全核验（APC §13 禁区）；高频门禁延迟 G4 前逐次裁决锚 X-FORBID-002；文件范围/@GOV/验收齐 |
