---
文档编号：CLOUD-C-D-01
文档状态：A
负责模块：CLOUD（云端域）
文档职责：Binder 云端服务产品概述（云端域 L1 根）——声明域级 REQ-CLOUD-*，覆盖认证会话 / 门禁裁决 / 网关转发 / BYOK 代持 / 订阅计费用量 / 个人账户门户 / 数据边界与合规 / 契约兼容
上游约束：A-CLOUD-C-P-01_云端APC.md（§1 四模块与四项不做、§3 驻留期硬约定、§5 禁区）、A-SYS-I-P-03_云接口契约.md（CLOUD-AUTH/GATE/INVOKE、SM-SYS-01、BR-SYS-DATA-004、X-SEQ-001/X-FORBID-001/002）、A-CORE-X-L-01_总体开发路线图.md（§8.5 立项输入）、客户端 A-ACCT/A-ENT 文档（挂起项 ACCT-01/03、ENT-01/02/03）
直接承接：云端各模块 L1（A-AUTH/GW/LIC/PORTAL-M-D-NN，待建）声明模块级 REQ 承接本文；独立仓库迁移后经 REQ_MAP 映射 L2 规则
使用边界：本文是云端域 L1 根，仅声明 REQ 与边界叙述；**遵守驻留期硬约定不放注册类治理块**（云 APC §3）；客户端对接契约字段归 A-SYS-I-P-03（本文不重定义协议）；具体技术方案归各模块 L2（迁移后展开）
变更要求：REQ 增删按 ADU §3.2；涉产品边界（尤其四项不做）变更须经云 APC 授权；与客户端契约相关条款变更双侧复核
治理成熟度：draft
---

# Binder 云端服务产品概述（云端域 L1 根）

## §1 文档定位

```
A-CLOUD-C-P-01（宪章）→ 本文（REQ-CLOUD-*，域级 L1 根）→ 各模块 L1（REQ-AUTH/GW/LIC/PORTAL-*）→ L2 规则（独立仓库迁移后）
```

本文把宪章 §1 的项目身份展开为可承接的域级需求。**前提重申**：客户端对接契约 SSOT 是
A-SYS-I-P-03（本文不重定义协议字段）；调用内容不留存（BR-SYS-DATA-004）是最高级产品承诺，
本文一切需求在该承诺之内。

## §2 域级需求声明（REQ-CLOUD-*）

<!-- REQ
req_id: REQ-CLOUD-001
title: 认证与会话服务
owner_module: CLOUD
status: active
summary: AUTH 提供邮箱级身份的注册/登录/登出、SessionToken 签发与续期、设备会话清单与强制下线；身份库 PII 最小化（邮箱 + 凭据哈希 + 设备会话表）；实现 A-SYS-I-P-03 CLOUD-AUTH 契约的服务端
source_doc: A-SYS-I-P-03（CLOUD-AUTH、SM-SYS-01 会话段）、云 APC §1
acceptance: 客户端可登录/续期/登出且与 SM-SYS-01 转移一致；门户可见设备会话清单并可强制下线；身份库无超出最小化范围的 PII
open_issues: 认证方式（邮箱密码/OAuth/魔法链接）待人类决策（§4 CLOUD-01，解除客户端 ACCT-01）；多设备并存上限与下线策略细则归 AUTH G2（解除 ACCT-03）
-->

<!-- REQ
req_id: REQ-CLOUD-002
title: 订阅门禁裁决
owner_module: CLOUD
status: active
summary: 门禁裁决唯一在服务端：每次模型调用按 CLOUD-GATE 契约裁决有效订阅，deny 返回不可绕过的拒绝原因；高频轻量调用（autocomplete）的延迟优化采用服务端签发的短时裁决凭证方案——凭证为服务端签发的短时授权而非客户端缓存，设计须经 X-FORBID-002 一致性复核并回写 A-SYS-I-P-03
source_doc: A-SYS-I-P-03（CLOUD-GATE、X-FORBID-002）、客户端 ENT-03
acceptance: 无有效订阅一律拒绝；不存在使客户端可自行放行的机制；短时凭证方案（若启用）有明确时效/撤销语义且通过 X-FORBID-002 复核
open_issues: 短时裁决凭证的时效、撤销、与订阅状态变更的即时性归 GW/LIC G2（§4 CLOUD-04，解除客户端 ENT-03）
-->

<!-- REQ
req_id: REQ-CLOUD-003
title: AI 网关转发与计量
owner_module: CLOUD
status: active
summary: GW 持有 provider 密钥（客户端永不持有，X-FORBID-001 服务端对偶），统一代理模型调用并流式转发；1.0 EP 默认经 AiGateway 接入 DeepSeek 官方 API，后续自部署 Binder 预设模型作为演进项；调用正文仅瞬时过境不持久化（BR-SYS-DATA-004 落实：日志/监控/错误上报均不留正文），仅产生脱敏计量元数据（用量/时间/模型档）上报 LIC
source_doc: A-SYS-I-P-03（CLOUD-INVOKE、BR-SYS-DATA-001/004）、云 APC §1/§5
acceptance: 全部调用经网关流式转发且终态语义与客户端契约一致；服务端任何持久化介质（含日志）无调用正文（审计核验项）；计量元数据不含正文
open_issues: 1.0 EP DeepSeek 具体模型名/价格/限流与合规条款在 GW L2/实现前核对；provider 池与模型档位映射、故障转移策略归 GW G2；流式转发的背压与超时归 GW G2；自部署 Binder 预设模型与多模态模型矩阵不作为 1.0 EP 阻断项
-->

<!-- REQ
req_id: REQ-CLOUD-004
title: BYOK 密钥代持
owner_module: CLOUD
status: active
summary: GW 提供 BYOK 上送 API：用户密钥加密保管、与身份数据分离存储、可经门户查看（脱敏）/轮换/删除；BYOK 调用透传不计套餐额度但仍经门禁；删除即生效
source_doc: A-SYS-I-P-03（TERM-CLOUD-004）、云 APC §1（GW 职责）、客户端 ENT-02
acceptance: 密钥存储与身份库物理/逻辑分离且加密；门户可管理（脱敏展示/轮换/删除）；无订阅时 BYOK 调用被拒；删除后调用即失效
open_issues: 上送 API 形状与加密方案归 GW G2（解除客户端 ENT-02）；密钥加密的密钥管理（KMS/信封加密）归 G2 且变更须人类批准（云 APC §5）
-->

<!-- REQ
req_id: REQ-CLOUD-005
title: 订阅、计费与用量
owner_module: CLOUD
status: active
summary: LIC 管理订阅档位、计费周期、用量额度与账单：消费 GW 计量元数据累计用量、执行额度判定供门禁裁决；向客户端与门户提供用量/订阅状态查询（客户端展示的唯一数据源）
source_doc: 云 APC §1（LIC 职责）、客户端 BR-ENT-DATA-002、ENT-01
acceptance: 用量累计与额度判定准确一致；客户端/门户查询到的状态与裁决依据同源；账单可追溯到计量元数据（不含正文）
open_issues: 订阅档位与定价待人类商业决策（§4 CLOUD-02）；用量查询粒度与刷新归 LIC G2（解除客户端 ENT-01）；支付通道选型归 PORTAL/LIC G2
-->

<!-- REQ
req_id: REQ-CLOUD-006
title: 个人账户门户
owner_module: CLOUD
status: active
summary: PORTAL 提供 Web 端账户中心（路线图 §8.5 清单全量）：订阅管理与购买、账单/发票、用量明细、设备与会话管理、BYOK 密钥管理、资料与安全（改密/改邮箱）、账号注销；是支付与账户管理的唯一界面（桌面端仅展示+跳转）
source_doc: 路线图 §8.5（2026-06-12 人类确认）、客户端 ACCT/ENT L1 v1.1
acceptance: 清单七项能力可用；桌面端无平行管理/支付界面；门户操作（强制下线/删密钥/注销）即时生效于客户端下一次门禁/会话校验
open_issues: 门户技术栈（默认随单体 Node/TS 同仓）与部署形态归 PORTAL G2；注销的数据清理时限与留存策略归 §4 CLOUD-03 合规决策
-->

<!-- REQ
req_id: REQ-CLOUD-007
title: 数据边界与合规
owner_module: CLOUD
status: active
summary: 服务端数据边界：不存储用户创作内容、不留存调用正文（最高承诺）、PII 最小化、密钥与身份分离；支持用户数据导出与删除（注销即按时限清理身份/商业数据）；数据驻留区域与隐私政策合规为发布前置
source_doc: 云 APC §1 四项不做、§5 禁区、BR-SYS-DATA-004
acceptance: 全库审计无创作内容/调用正文存储；注销后按声明时限完成清理；数据导出可用；隐私政策与数据驻留声明先于对外发布（G6 前置）
open_issues: 数据驻留区域、留存时限、合规清单（隐私政策/用户协议）待人类决策（§4 CLOUD-03）
-->

<!-- REQ
req_id: REQ-CLOUD-008
title: 契约兼容与版本联动
owner_module: CLOUD
status: active
summary: 云端接口与 A-SYS-I-P-03 契约版本联动：变更双侧复核留痕；提供最低客户端版本声明机制，破坏性变更须客户端版本协同；服务端部署节奏与客户端发布通道（alpha→beta→RC→GA）对齐
source_doc: 云 APC §2/§6、A-SYS-I-P-03 变更要求
acceptance: 不存在未经双侧复核的接口语义变更；旧版客户端收到明确的版本不兼容错误（统一错误形状）而非未定义行为
open_issues: 版本声明机制（握手/响应头）归 GW G2
-->

## §3 域级边界叙述（不放注册块，对应客户端约束的服务端对偶）

| 客户端约束 | 服务端对偶（本文承接，迁移后在云端 L2 落规则） |
|---|---|
| X-FORBID-001（客户端不持密钥） | provider 密钥仅 GW 持有；BYOK 加密代持、与身份分离（REQ-CLOUD-003/004） |
| X-SEQ-001（先门禁后调用） | 网关拒绝一切未携带有效裁决（或有效短时凭证）的调用（REQ-CLOUD-002/003） |
| X-FORBID-002（无客户端绕过） | 裁决唯一在服务端；短时凭证=服务端签发短时授权，非客户端缓存（REQ-CLOUD-002） |
| BR-SYS-DATA-004（内容不留存） | 全持久化介质（含日志/监控/错误上报）无调用正文，审计核验（REQ-CLOUD-003/007） |
| BR-ENT-DATA-002（展示数据源唯一） | 客户端/门户查询与裁决依据同源于 LIC（REQ-CLOUD-005） |

### 3.1 1.0 Experience Preview 托管模型策略

1.0 EP 为早期体验版，云端模型策略采用“DeepSeek 托管模型 + BYOK”：

| 项 | 1.0 EP 处置 |
|---|---|
| 托管模型 | AiGateway 接入 DeepSeek 官方 API，优先跟随 DeepSeek 当前推荐模型线；具体模型名、价格、限流与合规条款在 GW L2/实现前核对 |
| 自部署 Binder 预设模型 | 不作为 1.0 EP 阻断项；后续在真实体验数据、Binder 专项评测集和开源多模态模型评测后进入 Beta |
| 多模态模型矩阵 | 不作为 1.0 EP 阻断项；1.0 仅保留接口和 telemetry 基础，后续按文档页面/截图等真实场景引入 |
| BYOK | 保留为用户自带 provider 通道，但密钥仍由 AiGateway 加密代持，调用仍经 SubscriptionGate |
| 数据边界 | 不因调用第三方托管模型而弱化 BR-SYS-DATA-004；云端默认不保存调用正文，体验反馈只记录脱敏元数据 |

## §4 G1 问题暴露清单（域级）

| 编号 | 分类 | 状态 | 处置 |
|---|---|---|---|
| CLOUD-01 | NEEDS_HUMAN_DECISION | **待决策** | 认证方式（邮箱密码 / OAuth / 魔法链接，可多选组合）——产品决策，AUTH G2 前须定（联动客户端 ACCT-01） |
| CLOUD-02 | NEEDS_HUMAN_DECISION | **待决策** | 订阅档位与定价（档数/价格/额度/BYOK 档位策略）——商业决策，LIC G2 前须定 |
| CLOUD-03 | NEEDS_HUMAN_DECISION | **待决策** | 数据驻留区域、注销清理时限、合规清单（隐私政策/用户协议）——发布前置（G6），可后置于 G2 |
| CLOUD-04 | DESIGN_RISK | 显式延期 | 短时裁决凭证方案（时效/撤销/订阅变更即时性）归 GW/LIC G2，**X-FORBID-002 一致性复核为强制门禁**并回写 A-SYS-I-P-03 |
| CLOUD-05 | BOUNDARY_OPEN | 显式延期 | 计量粒度、provider 池、支付通道、版本声明机制等归各模块 G2 |
| CLOUD-06 | ACCEPTANCE_MISSING | 部分关闭 | 定性验收已入 §2；量化 NFR（门禁延迟/网关首字延迟/可用性目标）延云端 G5 标定 |

## §5 后续动作

1. 人类决策 CLOUD-01（认证方式）与 CLOUD-02（订阅档位定价）——AUTH/LIC G2 的前置；CLOUD-03
   可后置但须在 G6 前闭合。
2. 各模块 L1 展开（A-AUTH/GW/LIC/PORTAL-M-D-01，声明模块级 REQ 承接本文），遵守驻留期硬约定。
3. 客户端挂起项解除路径已分配至各 REQ 的 open_issues（ACCT-01→CLOUD-01、ACCT-03→REQ-CLOUD-001、
   ENT-01→REQ-CLOUD-005、ENT-02→REQ-CLOUD-004、ENT-03→REQ-CLOUD-002/CLOUD-04），G2 解除时同步
   回写客户端文档。
4. 独立仓库建立时随宪章一并迁出（云 APC §3 流程）。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-07-17 | v1.1 | 接入 1.0 Experience Preview 云端模型策略：REQ-CLOUD-003 明确 1.0 EP 默认经 AiGateway 接入 DeepSeek 官方 API，自部署 Binder 预设模型与多模态模型矩阵后置；新增 §3.1 说明 DeepSeek 托管模型 + BYOK 边界，强调不弱化 SubscriptionGate、AiGateway、BYOK 代持和内容不留存承诺 |
| 2026-06-12 | v1.0 | 初版（云端域 L1 根）：声明 REQ-CLOUD-001~008（认证会话 / 门禁裁决含短时凭证方向 / 网关转发与计量 / BYOK 代持 / 订阅计费用量 / 个人账户门户 / 数据边界与合规 / 契约兼容）；§3 客户端约束的服务端对偶表；G1 暴露 CLOUD-01~06（认证方式、档位定价、合规三项待人类）；客户端五个挂起项的解除路径分配完毕；遵守驻留期硬约定（仅 REQ + 叙述） |
