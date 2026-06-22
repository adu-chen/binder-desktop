---
文档编号：ACCT-I-P-01
文档状态：A
负责模块：ACCT
文档职责：ACCT 账号会话模块级接口协议——登录登出 command、会话态查询、令牌续期、reauth 触发；承接 SM-SYS-01 会话段
上游约束：A-SYS-I-P-02_IPC协议.md（骨架、reauth_required）、A-SYS-I-P-03_云接口契约.md（CLOUD-AUTH、SM-SYS-01 会话段、SessionToken、X-FORBID-001）、A-ACCT-M-T-01_账号会话技术设计.md（BR-ACCT-* 规则）、ADU.md
直接承接：实现期 ACCT command 注册；经云端 AUTH（A-SYS-I-P-03 契约）认证；认证态供 ENT（门禁提交）与上层
使用边界：声明语义级签名；云端认证协议字段归 A-SYS-I-P-03 与云 AUTH（不重定义）；不重定义 SM-SYS-01（承接会话段）；不新增注册项；认证方式由云端决定（ACCT-01）
变更要求：会话接口变更须复核 SM-SYS-01 与云端 AUTH；令牌持有变更涉 APC §4.3 须授权
治理成熟度：draft
---

# Binder 账号会话接口协议（ACCT · L2 接口层）

## §1 文档定位

```
A-SYS-I-P-02（骨架）+ A-SYS-I-P-03（CLOUD-AUTH 契约）→ 本文（ACCT 会话 command）→ 实现 + @GOV(追溯 A-ACCT-M-T-01)
承接 SM-SYS-01 会话段（Unauthenticated⇄Authenticated、sessionExpire）；不重定义状态机
```

## §2 接口清单（CLOUD-AUTH 客户端 / SM-SYS-01 会话段）

| command/事件 | 参数（语义） | 结果 | 特有错误/状态 | 归属 |
|---|---|---|---|---|
| `acct.login` | 凭据（形式由云 AUTH 定，ACCT-01） | 认证态（持 SessionToken，安全存储） | login_failed→停留未认证 | SM-SYS-01 login/loginOk；BR-ACCT-STATE-001 |
| `acct.logout` | — | 未认证态（清令牌） | — | SM-SYS-01 logout |
| `acct.get_session_state` | — | 认证态（已认证/未认证，不含令牌明文） | — | 供 ENT/上层 |
| `acct.on_session_change`（事件） | 订阅 | 认证态变更通知（含 sessionExpire→reauth_required） | — | BR-ACCT-STATE-001（reauth 可靠触发，不静默吞） |

> SessionToken 经系统安全存储（Keychain），**不经任何 command 回传明文**（BR-ACCT-DATA-001）；
> 续期为内部行为（按云契约），失败转未认证并发 on_session_change；登录 UI 形态随认证方式定。

## §3 测试与验收

- login/logout/过期转移与 SM-SYS-01 一致；令牌无明文经 command 回传/日志（静态+运行核验）；
  reauth_required 可靠触发；断网/未登录本地功能不受影响、无认证骚扰；与云端 AUTH 契约测试。

## §4 债务

- 认证方式（ACCT-01）由云端 AUTH 决定，login 参数形态随之定；多设备会话管理界面归门户（ACCT-03）。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（实现前置件）：ACCT 接口协议——login/logout/get_session_state/on_session_change（reauth 可靠触发）；承接 SM-SYS-01 会话段不重定义；令牌不经 command 回传明文；零新增注册项；债务（认证方式待云端、多设备界面归门户） |
