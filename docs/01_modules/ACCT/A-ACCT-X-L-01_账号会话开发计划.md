---
文档编号：ACCT-X-L-01
文档状态：A
负责模块：ACCT
文档职责：ACCT 账号会话 G3 开发计划——承接型 Plan Item，承接会话段运行时（SM-SYS-01 会话段）与 BR-ACCT-* 规则，明确文件范围/@GOV/依赖云端的 C 类/验收
上游约束：A-CORE-X-L-02（P1 尾/P5）、APC §5/§6/§12、A-ACCT-M-T-01、A-ACCT-I-P-01、A-SYS-I-P-03（CLOUD-AUTH/SM-SYS-01）、A-SYS-X-L-01（PI-SYS-04 状态机骨架）、A-CLOUD-C-D-01（云端 AUTH）、A-CORE-X-R-01
直接承接：本计划驱动 ACCT 实现；@GOV 追溯 BR-ACCT-*；承接 SM-SYS-01 会话段（不重定义）
使用边界：L3 承接型 Plan Item；云端认证协议归 A-SYS-I-P-03 + 云 AUTH（打桩实现，真联调 P5）；@GOV 在实现代码
变更要求：会话语义变更复核 SM-SYS-01；令牌持有涉 APC §4.3 须授权
治理成熟度：draft
---

# Binder 账号会话开发计划（ACCT · L3）

## §1 文档定位

ACCT 承接 SM-SYS-01 **会话段**运行时（不建新机，依 PI-SYS-04 骨架）。**云端依赖（C 类）**：
认证方式、协议字段、多设备策略归云端 AUTH（CLOUD-01/03）；客户端按 A-SYS-I-P-03 契约**打桩
实现**，真联调延 P5（云端就绪后）。

## §2 Plan Item 清单

### PI-ACCT-01　会话生命周期承接　✅ G4（session.rs 承接 SM-SYS-01 不重定义 apply 委托/must_clear_token/reauth_action 可靠触发/离线不打扰 BR-ACCT-STATE-001，audit 绿）
- **承接**：REQ-ACCT-001/003/004 / SM-SYS-01 会话段 / BR-ACCT-STATE-001（会话段承接 +
  reauth_required 可靠触发 + 离线本地可用）。
- **文件范围**：`src/`/`src-tauri/src/acct/session.rs`（login/logout/会话态供给/过期重认证；
  承接 PI-SYS-04 SM-SYS-01 会话段转移）；`acct.login/logout/get_session_state/on_session_change`。
- **@GOV**：会话段承接 + reauth 触发 + 离线行为 `BR-ACCT-STATE-001`。
- **C 类依赖**：认证方式（CLOUD-01，云端 AUTH）→ login 参数形态随云端定，**打桩实现**；多设备
  策略（CLOUD-03）→ 客户端被动承接 sessionExpire。
- **验收**：login/logout/过期转移与 SM-SYS-01 一致；reauth_required 可靠触发重认证；**断网/未登录
  本地功能完整可用、无认证骚扰**；与云端 AUTH 契约测试（打桩，真联调 P5）。

### PI-ACCT-02　会话令牌安全持有　✅ G4（token.rs SessionToken newtype 脱敏 Debug/TokenVault store-clear-renew 零残留续期失败清旧令牌/与 provider 密钥严格区分 BR-ACCT-DATA-001，audit 绿）
- **承接**：REQ-ACCT-002 / BR-ACCT-DATA-001（令牌安全持有）/ X-FORBID-001（令牌≠provider 密钥）。
- **文件范围**：`src-tauri/src/acct/token.rs`（macOS Keychain 持有/续期/失效清除；令牌不经
  command 回传明文、不入日志）。
- **@GOV**：令牌安全持有（Keychain/无明文落盘日志/与 provider 密钥严格区分）`BR-ACCT-DATA-001`。
- **A 类债务**：Keychain 具体 API 与续期时机（提前量/重试退避，ACCT-02，实现期）；登录 UI 形态
  （随认证方式，实现期）。
- **验收（强制，APC §12）**：**静态扫描无明文令牌落盘/日志/command 回传路径**；登出/过期后
  Keychain + 内存无残留（运行核验）；续期失败降级未认证。

## §3 横切验收

audit 绿；@GOV 追溯 BR-ACCT-*；SM-SYS-01 会话段转移测试（复用 A-SYS-I-P-03 §10 矩阵会话段）；
`// covers: BR-ACCT-*`（2 条）有断言；令牌安全为 G4 强制静态核验。

## §4 承接的债务（A-CORE-X-R-01 §5）

| 债务 | 类 | PI | 处置 |
|---|---|---|---|
| 认证方式（ACCT-01/CLOUD-01） | C | PI-ACCT-01 | 云端 AUTH 立项解，打桩实现 |
| 多设备策略（ACCT-03/CLOUD-03） | C | PI-ACCT-01 | 云端解，客户端被动承接 |
| Keychain API/续期时机（ACCT-02） | A | PI-ACCT-02 | 实现期 |
| 登录 UI 形态 | A | PI-ACCT-01 | 随认证方式，实现期 |
| 登录/续期延迟 NFR | A | 全 PI | G5 标定 |

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（G3，P1 尾/商业化客户端侧）：ACCT 2 条 Plan Item（会话生命周期承接 / 令牌安全持有），承接 SM-SYS-01 会话段 + 2 规则 + X-FORBID-001；云端依赖（认证方式/多设备）打桩实现真联调 P5；令牌无明文落盘 G4 强制静态核验；文件范围/@GOV/验收齐 |
