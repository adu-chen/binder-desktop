---
文档编号：ACCT-M-D-01
文档状态：A
负责模块：ACCT
文档职责：ACCT 账号会话模块级功能需求描述——声明模块级 REQ-ACCT-*，承接产品级 REQ-CORE-008/009 与 SYS 云接口契约（CLOUD-AUTH 客户端侧），界定登录登出与会话生命周期 / 会话令牌安全持有与续期 / 认证态对上层供给 / 离线行为
上游约束：A-CORE-C-D-01_产品概述.md（REQ-CORE-008/009）、A-CORE-C-P-01_APC.md（§4.1 ACCT 职责「账号、认证、会话（云端认证客户端）」、§4.2/§4.3 云薄端）、A-SYS-I-P-03_云接口契约.md（CLOUD-AUTH、SM-SYS-01 会话段、TERM-CLOUD-003 SessionToken、X-FORBID-001）、A-SYS-I-P-02_IPC协议.md（统一错误形状 reauth_required）、ADU.md（§3.2）
直接承接：ACCT L2 技术设计经 REQ_MAP 映射 RULE；ACCT 承接 SM-SYS-01 会话段运行时（不重定义状态机）；greenfield（台账 §4.15 无旧文档可取材）
使用边界：本文是 L1 功能需求描述层，仅声明 REQ-ACCT-*；**代码不得直接映射 REQ**（ADU §3.2）；云端认证协议字段归 A-SYS-I-P-03（SSOT）与云端 AUTH 模块（独立 APC）；订阅门禁/网关/用量归 ENT；认证方式（邮箱/OAuth 等）由云端 AUTH 决定，本文不预占
变更要求：会话语义变更须与 SYS（I-P-03）/云端 AUTH 联合复核；REQ 增删按 ADU §3.2
治理成熟度：draft
---

# Binder ACCT 账号会话功能需求描述（L1）

## §1 文档定位

```
产品概述(REQ-CORE-008/009) + A-SYS-I-P-03(CLOUD-AUTH 契约) → 本文(REQ-ACCT-*) → REQ_MAP → ACCT L2 → 代码(@GOV)
```

ACCT 是**云端认证的客户端模块**（greenfield，台账 §4.15）：承接 SM-SYS-01 的**会话段**运行时
（Unauthenticated/Authenticated、login/logout/sessionExpire），持有并管理会话令牌
（SessionToken，TERM-CLOUD-003），向 ENT 与上层供给认证态。云协议契约 SSOT 在 A-SYS-I-P-03，
ACCT 只承接客户端侧实现，不重定义协议与状态机。

## §2 模块定位与职责

| # | 职责 | 锚定 |
|---|---|---|
| AC-J1 | 登录/登出与会话生命周期（SM-SYS-01 会话段运行时承接） | CLOUD-AUTH、REQ-CORE-008 |
| AC-J2 | 会话令牌安全持有与续期（不明文落盘、失效即清） | TERM-CLOUD-003、APC §4.3 |
| AC-J3 | 认证态供给：向 ENT（门禁提交）与上层（reauth_required 处理）供给会话态 | A-SYS-I-P-02/03 |
| AC-J4 | 离线行为：未认证/断网不影响本地功能，仅 AI 链路受限 | REQ-CORE-009 |

> 边界澄清：ACCT 管**会话**，不管订阅——订阅校验/门禁/用量归 ENT；认证服务端协议归云 AUTH
> （独立 APC）；ACCT 不持 provider 密钥（X-FORBID-001，会话令牌≠模型密钥）。

## §3 能力范围（ACCT v1）

**IN**：登录/登出流程客户端侧；SessionToken 安全持有、续期、失效清除；认证态查询与变更通知；
reauth_required 统一触发重认证。

**OUT**：认证方式与服务端协议（云 AUTH）；订阅/门禁/网关/BYOK/用量（ENT）；**账号资料管理——
归个人账户门户**（官网门户升级定位，2026-06-12 人类确认：资料/安全/设备会话/注销均在门户，
桌面端仅登录态维持与跳转）；多设备会话策略（云端裁决 + 门户管理界面）。

## §4 代表性流程（契约级，APC §7）

**AC1 · 登录与会话维持**

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | 用户 | 凭据（形式由云 AUTH 定） | 经 CLOUD-AUTH 登录 | SessionToken（安全存储） | 2 | loginFail→统一错误形状，停留未认证态 |
| 2 | ACCT | 令牌临期/失效信号 | 按云契约续期；续期失败转未认证 | 续期成功/重认证要求 | 终态 | sessionExpire→清令牌，上层收 reauth_required |
| 3 | 用户 | 登出 | 清会话与本地令牌 | 未认证态 | 终态 | — |

## §5 模块级需求声明（REQ-ACCT-*）

<!-- REQ
req_id: REQ-ACCT-001
title: 登录登出与会话生命周期
owner_module: ACCT
status: active
summary: 承接 SM-SYS-01 会话段运行时：登录（loginOk 持有 SessionToken）/登出（清会话）/会话过期（sessionExpire 回未认证）；客户端不重定义会话状态机
source_doc: REQ-CORE-008、A-SYS-I-P-03（CLOUD-AUTH、SM-SYS-01）
acceptance: 登录后持有有效会话；登出与过期后回未认证且令牌被清除；状态转移与 SM-SYS-01 一致
open_issues: 认证方式（邮箱/OAuth）由云端 AUTH 决定（§7 ACCT-01）；多设备会话策略归云端
-->

<!-- REQ
req_id: REQ-ACCT-002
title: 会话令牌安全持有与续期
owner_module: ACCT
status: active
summary: SessionToken 经系统安全存储持有（不明文落盘、不入日志），按云契约续期，失效即清除；令牌仅用于门禁与网关调用（经 ENT/SYS 契约）
source_doc: TERM-CLOUD-003、APC §4.3、X-FORBID-001（密钥边界对照：令牌可持有、provider 密钥不可）
acceptance: 令牌无明文落盘/日志泄漏（可静态+运行核验）；续期可用；失效后无残留
open_issues: 存储介质（macOS Keychain）选型归 ACCT G2/实现（§7 ACCT-02）
-->

<!-- REQ
req_id: REQ-ACCT-003
title: 认证态供给
owner_module: ACCT
status: active
summary: 向 ENT（门禁提交需令牌）与上层模块供给认证态查询与变更通知；统一错误形状中 reauth_required=true 一律触发重认证流程，不静默吞掉
source_doc: A-SYS-I-P-02（BR-SYS-OBS-002 reauth_required）、A-SYS-I-P-03
acceptance: 认证态可查询、变更有通知；reauth_required 错误可靠触发重认证 UI
open_issues: 无
-->

<!-- REQ
req_id: REQ-ACCT-004
title: 离线行为
owner_module: ACCT
status: active
summary: 未认证或断网不影响编辑/预览/本地功能；仅 AI 链路按门禁语义受限；断网时不反复弹认证打扰
source_doc: REQ-CORE-009
acceptance: 断网/未登录时本地功能完整可用；无认证骚扰弹层
open_issues: 无
-->

## §6 模块边界与约束（边界闭合）

- **契约承接**：协议 SSOT 归 A-SYS-I-P-03；ACCT 不重定义 CLOUD-AUTH 链路与 SM-SYS-01（只承接
  运行时会话段）。
- **密钥边界**：X-FORBID-001——ACCT 持有的是会话令牌，永不持有/落地 provider 密钥。
- **职责边界**：订阅/门禁/网关归 ENT；本文不涉。
- **状态机**：不建新机（SM-SYS-01 已覆盖，owner=SYS，总图已 active），ACCT G2 声明承接段。
- **原料**：greenfield（台账 §4.15），无旧文档取材，全部自契约推导。

## §7 G1 问题暴露清单

| 编号 | 分类 | 状态 | 处置 |
|---|---|---|---|
| ACCT-01 | NEEDS_HUMAN_DECISION | 显式延期（不阻断 G1） | 认证方式（邮箱密码/OAuth/魔法链接）属云端 AUTH 产品决策，云端 APC 立项时定，客户端契约对其中立 |
| ACCT-02 | BOUNDARY_OPEN | 显式延期 | 令牌存储介质（macOS Keychain 优先）与续期时机归 ACCT G2/实现期 |
| ACCT-03 | BOUNDARY_OPEN | 显式延期 | 多设备会话/强制下线策略归云端 AUTH/LIC，**管理界面归个人账户门户**（2026-06-12 确认）；客户端按 sessionExpire 语义被动承接 |
| ACCT-04 | ACCEPTANCE_MISSING | 部分关闭 | 定性验收已入 §5；量化（登录/续期延迟）延 G5 |

## §8 后续动作

1. 待人类宣布 ACCT G1 闭合后进入 ACCT G2（L2 技术设计：会话段承接规则 + 令牌安全持有规则）。
2. 云端 AUTH 独立 APC 立项时联合解除 ACCT-01/03。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（P1 尾部 greenfield）：ACCT 账号会话模块级 L1；声明 REQ-ACCT-001~004（会话生命周期承接 SM-SYS-01 会话段 / 令牌安全持有续期 / 认证态供给与 reauth_required 触发 / 离线行为），承接 REQ-CORE-008/009 与 CLOUD-AUTH 契约；不重定义协议与状态机；G1 暴露清单 ACCT-01~04 |
| 2026-06-12 | v1.1 | 人类决策落档：官网门户升级定位为个人账户门户（资料/安全/设备会话/注销归门户，桌面端仅登录态维持与跳转）；§3 OUT 与 ACCT-03 同步更新 |
