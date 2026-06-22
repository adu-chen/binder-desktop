---
文档编号：ACCT-M-T-01
文档状态：A
负责模块：ACCT
文档职责：ACCT 账号会话模块级 L2 技术设计（规则 SSOT）——SM-SYS-01 会话段运行时承接规则、会话令牌安全持有与续期规则；承接 REQ-ACCT-001~004
上游约束：A-ACCT-M-D-01_账号会话功能需求描述.md（REQ-ACCT-001~004）、A-SYS-I-P-03_云接口契约.md（CLOUD-AUTH、SM-SYS-01、BR-SYS-STATE-001、TERM-CLOUD-003、X-FORBID-001）、A-SYS-I-P-02_IPC协议.md（BR-SYS-OBS-002 reauth_required）、A-CORE-C-P-01_APC.md（§4.3/§13）、ADU.md（§3）
直接承接：本文 RULE 为 ACCT 规则 SSOT，代码经 @GOV 映射；规则主链路引用 CLOUD-AUTH（不重复声明 chain_id，遵 A-SYS-I-P-03 承接原则）
使用边界：本文是 ACCT 规则 SSOT；云协议字段/状态机归 A-SYS-I-P-03（承接不重定义）；订阅门禁/网关归 ENT；认证方式归云端 AUTH；不注册新 TERM/CHAIN/CONSTRAINT（§4 说明）；无新状态机（SM-SYS-01 覆盖）
变更要求：会话承接语义变更须与 SYS 复核 SM-SYS-01 一致性；令牌安全规则变更涉 APC §4.3 须先授权
治理成熟度：draft
---

# Binder 账号会话技术设计（ACCT · L2 技术设计）

## §1 文档定位

```
REQ-ACCT-001~004（ACCT L1）→ 本文（SSOT）→ 代码(@GOV)
本文规则 ──主链路引用──→ CLOUD-AUTH（chain_id 归 A-SYS-I-P-03，不重复声明）
本文 ──运行时承接──→ SM-SYS-01 会话段（不重定义状态机内部）
```

## §2 会话段运行时承接（CLOUD-AUTH）

ACCT 承接 SM-SYS-01 的会话段：`Unauthenticated ⇄ Authenticated`（login/loginOk/loginFail/
logout）与任意态 `sessionExpire → Unauthenticated`。门禁段（GateChecking 起）归 ENT 承接。

<!-- RULE
rule_id: BR-ACCT-STATE-001
主链路: CLOUD-AUTH
域: STATE
-->
ACCT 必须按 SM-SYS-01 会话段语义承接运行时：登录成功持有 SessionToken 进入认证态；登出与
sessionExpire 一律清除令牌回未认证态（无残留会话）；统一错误形状中 reauth_required=true 必须
可靠触发重认证流程（不静默吞掉、不自动重试绕过）；未认证/断网不影响本地功能且不反复弹认证
打扰；ACCT 不得修改 SM-SYS-01 的状态/转移定义（一致性以 A-SYS-I-P-03 §6 为准）。

## §3 会话令牌安全持有（CLOUD-AUTH）

<!-- RULE
rule_id: BR-ACCT-DATA-001
主链路: CLOUD-AUTH
域: DATA
-->
SessionToken 必须经系统级安全存储持有（macOS 实现优先 Keychain）：不得明文落盘（含配置文件/
workspace.db/缓存）、不得写入日志或错误上报、不得经 IPC 以外通道暴露；续期按云契约执行，续期
失败转未认证并清除旧令牌；登出/过期后本地无令牌残留。令牌与 provider 密钥严格区分：前者可持有，
后者受 X-FORBID-001 禁止（客户端永不持有）。

## §4 不新增注册项说明

- **CHAIN**：引用 CLOUD-AUTH（A-SYS-I-P-03 注册，主责 SYS），按其「运行时承接不重复声明」原则。
- **TERM**：SessionToken 已由 SYS 注册（TERM-CLOUD-003）；ACCT 无新增核心业务概念。
- **CONSTRAINT**：X-FORBID-001/X-FORBID-002 已覆盖密钥与绕过边界，无新增。
- **STATE_MACHINE**：SM-SYS-01 已覆盖（owner=SYS，总图 active），ACCT 仅承接会话段运行时，
  不建新机（APC §6：同一状态机不重复定义）。

## §5 需求到规则映射（REQ_MAP）

<!-- REQ_MAP
req_id: REQ-ACCT-001
target_type: RULE
target_id: BR-ACCT-STATE-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ACCT-002
target_type: RULE
target_id: BR-ACCT-DATA-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ACCT-003
target_type: RULE
target_id: BR-ACCT-STATE-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ACCT-004
target_type: RULE
target_id: BR-ACCT-STATE-001
status: mapped
-->

## §6 测试与验收

- **会话段一致**（BR-ACCT-STATE-001）：登录/登出/过期转移与 SM-SYS-01 一致（转移测试矩阵复用
  A-SYS-I-P-03 §10 会话段）；reauth_required 可靠触发重认证；断网本地功能完整、无认证骚扰。
- **令牌安全**（BR-ACCT-DATA-001）：静态扫描无明文令牌落盘/日志路径；登出/过期后 Keychain 与
  内存无残留（运行核验）；续期失败正确降级未认证。
- 量化（登录/续期延迟）延 G5。

## §7 技术债务与盲区（诚实声明）

- 认证方式与多设备策略归云端 AUTH/LIC（ACCT-01/03），客户端契约保持中立，云端 APC 立项后联合。
- Keychain 具体 API 与续期时机（提前量/重试退避）延实现期。
- 登录 UI 形态（内嵌/系统浏览器跳转）随认证方式确定，延云端决策后。

## §8 后续动作

1. 注册（generate）：BR-ACCT-STATE-001 / BR-ACCT-DATA-001、REQ_MAP 进 ADUS。
2. 待人类宣布 ACCT G2 闭合；云端 AUTH APC 立项时联合解除 ACCT-01/03。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（ACCT G2，greenfield）：SM-SYS-01 会话段运行时承接规则（BR-ACCT-STATE-001：reauth_required 可靠触发、过期清令牌、不改状态机定义）、令牌安全持有规则（BR-ACCT-DATA-001：Keychain 优先、无明文落盘/日志、与 provider 密钥严格区分）；主链路引用 CLOUD-AUTH 不重复声明；零新增 CHAIN/TERM/CONSTRAINT/状态机（§4 说明）；REQ_MAP（REQ-ACCT-001~004） |
