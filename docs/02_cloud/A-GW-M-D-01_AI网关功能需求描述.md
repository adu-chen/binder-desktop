---
文档编号：GW-M-D-01
文档状态：A
负责模块：GW
文档职责：AI 网关功能需求描述（GW 模块 L1）——供应商适配层（OpenAI/Anthropic/DeepSeek）、统一流式转发、BYOK 密钥代持、订阅门禁裁决（CLOUD-GATE）、内容不留存；承接 REQ-CLOUD-003/004 并展开为模块级 REQ-GW-001~006
上游约束：A-CLOUD-C-P-01_云端APC.md（§1 GW 职责、§5 禁区）、A-CLOUD-C-D-01_云端产品概述.md（REQ-CLOUD-003/004）、A-SYS-I-P-03_云接口契约.md（CLOUD-GATE/INVOKE、BR-SYS-DATA-004、X-SEQ-001/X-FORBID-001/002）
直接承接：GW L2 技术设计（待建）；客户端 A-ENT-M-T-01（BR-ENT-STATE-001/VERIFY-001）、A-AG-X-L-02（PI-AG-11 依赖 PI-GW-01~05）
使用边界：仅含 REQ 块与叙述性设计，遵守驻留期硬约定不放注册类治理块（云 APC §3）；客户端对接契约字段归 A-SYS-I-P-03（本文不重定义协议）；具体技术方案归 GW L2（迁移后展开）
变更要求：涉 A-SYS-I-P-03 接口语义的变更须双侧复核留痕；provider 池变更须人类确认；BR-SYS-DATA-004（内容不留存）弱化即产品边界变更，须云 APC 授权
治理成熟度：draft
---

# Binder AI 网关功能需求描述（GW · L1）

## §1 文档定位

```
REQ-CLOUD-003（网关转发与计量）→ REQ-GW-001/002/003/006（供应商/流式/计量）
REQ-CLOUD-004（BYOK 代持）      → REQ-GW-004（BYOK 密钥管理）
REQ-CLOUD-002（门禁裁决）       → REQ-GW-005（CLOUD-GATE 服务端实现）
A-SYS-I-P-03（客户端契约 SSOT）→ 本文实现对应服务端行为（不重定义接口字段）
```

本文把 REQ-CLOUD-003/004 展开为可实现的 GW 模块级需求。**核心约束重申**：
- X-FORBID-001 服务端对偶：**provider 密钥唯 GW 持有**，任何路径不得下发至客户端
- BR-SYS-DATA-004 最高承诺：**调用正文不留存**——任何持久化介质（含日志/监控/错误上报）无调用正文
- X-SEQ-001 服务端对偶：**无有效门禁裁决的调用一律拒绝**，GW 不转发未授权请求

## §2 模块级需求声明（REQ-GW-*）

<!-- REQ
req_id: REQ-GW-001
title: 供应商适配层
owner_module: GW
status: active
summary: GW 统一适配初始三个 AI 供应商（OpenAI / Anthropic / DeepSeek），对外暴露统一调用接口（A-SYS-I-P-03 CLOUD-INVOKE）；每个供应商对应独立适配器，负责 provider 特定认证、请求构造、响应解析；新增供应商通过增加适配器实现，不影响统一接口
source_doc: REQ-CLOUD-003、A-SYS-I-P-03（CLOUD-INVOKE）
acceptance: 三供应商适配器可独立启用/禁用；调用方无需感知 provider 差异（统一接口）；新增第四家供应商不修改统一接口；provider 密钥仅在适配器层使用且不写日志/持久化
open_issues: provider 故障转移策略（主/备）归 GW L2；模型档位与 provider 映射规则归 L2
-->

<!-- REQ
req_id: REQ-GW-002
title: 统一流式转发与协议规范化
owner_module: GW
status: active
summary: GW 把三家 provider 各自的 SSE 格式规范化为统一客户端 StreamEvent（Chunk/ToolRequest/Done/Error），屏蔽 provider 差异；背压处理、连接超时、取消传播均在此层实现；调用正文（含 prompt、工具结果、模型输出）仅瞬时过境，不缓冲至持久化存储（BR-SYS-DATA-004）
source_doc: REQ-CLOUD-003、A-SYS-I-P-03（BR-SYS-DATA-001/004）
acceptance: 客户端 StreamEvent 格式与 A-SYS-I-P-03 §CLOUD-INVOKE 定义完全一致；三供应商 SSE 格式差异对客户端透明；取消信号正确传播至 provider 连接（不持续计费）；无任何 streaming 中间数据写日志
open_issues: 背压参数、超时值归 GW L2；多模态（图片等）延后
-->

<!-- REQ
req_id: REQ-GW-003
title: 用量计量元数据
owner_module: GW
status: active
summary: 每次调用完成后记录脱敏计量元数据（user_id/时间戳/模型档/token 数/BYOK 标记），供 LIC 额度判定；计量数据不含调用正文（BR-SYS-DATA-004）；计量须不可篡改（client 不可伪造）
source_doc: REQ-CLOUD-003、REQ-CLOUD-005（LIC 消费）
acceptance: 计量表无 prompt/completion 字段（审计核验项）；LIC 可从计量表累计用量并执行额度判定；client 端无法修改服务端计量结果
open_issues: 计量粒度（token 级/请求级）归 L2；与 LIC 的实时/批量同步归 L2
-->

<!-- REQ
req_id: REQ-GW-004
title: BYOK 密钥代持
owner_module: GW
status: active
summary: GW 提供 BYOK 密钥上送 API（POST /byok/keys）：接收客户端一次性上送的 provider 密钥（byok.rs 零残留语义），加密保管与身份数据分离存储；BYOK 调用时 GW 取回并使用密钥，不回传客户端；密钥可经门户查看（脱敏）/轮换/删除，删除即生效；BYOK 调用仍经门禁（不豁免）
source_doc: REQ-CLOUD-004、客户端 BR-ENT-DATA-001（BYOK 零残留）、A-SYS-I-P-03（TERM-CLOUD-004）
acceptance: 密钥存储与 AUTH 身份库物理/逻辑分离且加密（AES-256-GCM 或同等）；明文密钥不写任何日志/监控；删除操作后该密钥的调用立即失败；无有效订阅时 BYOK 调用被门禁拒绝（TERM-CLOUD-004 语义）
open_issues: 密钥加密的密钥管理（KMS/信封加密）归 L2 且变更须人类批准（云 APC §5）；BYOK 供应商选择 UI 归 PORTAL；多密钥（不同供应商）管理归 L2
-->

<!-- REQ
req_id: REQ-GW-005
title: CLOUD-GATE 订阅门禁裁决
owner_module: GW
status: active
summary: GW 实现 A-SYS-I-P-03 的 CLOUD-GATE 服务端：校验 session token 有效性（联 AUTH）+ 订阅额度（联 LIC），返回 passed/denied；门禁裁决唯一在服务端（X-FORBID-002 对偶），无客户端绕过路径；断网等同拒绝（客户端收不到 passed 即拒绝）；高频轻量调用（autocomplete）的短时裁决凭证方案（CLOUD-04）须经 X-FORBID-002 一致性复核后回写 A-SYS-I-P-03
source_doc: REQ-CLOUD-002、A-SYS-I-P-03（CLOUD-GATE、X-FORBID-002）
acceptance: 无有效订阅一律拒绝（含 BYOK 用户无订阅时）；session token 过期/无效拒绝且返回统一错误形状（reauth_required=true）；不存在服务端侧绕过门禁的路径
open_issues: 短时裁决凭证方案（CLOUD-04，高频 autocomplete 延迟优化）——须先完成 X-FORBID-002 一致性复核再实现，归 GW L2
-->

<!-- REQ
req_id: REQ-GW-006
title: 内容不留存（最高承诺落实）
owner_module: GW
status: active
summary: 所有持久化介质（数据库/日志/监控/错误上报/调试输出）均不留存模型调用正文——包括 prompt、工具调用参数与结果、模型输出文本；仅留脱敏计量元数据（REQ-GW-003）；此承诺为最高级产品约束，弱化须经 APC 授权
source_doc: REQ-CLOUD-007、A-SYS-I-P-03（BR-SYS-DATA-004）、云 APC §5
acceptance: GW 所有持久化路径（DB schema、日志框架配置、监控 exporter）审计无正文字段；错误上报（Sentry 等）明确过滤正文；发布前完成 BR-SYS-DATA-004 合规审计
open_issues: 数据驻留区域合规（CLOUD-03，归 G6 前）
-->

## §3 三供应商适配器规格

> 本节为叙述性设计（驻留期硬约定，L2 迁移后详细实现）。

### 3.1 OpenAI 适配器

**端点**：`https://api.openai.com/v1/chat/completions`
**认证**：`Authorization: Bearer {openai_key}`
**请求格式**（标准 Chat Completions）：
```json
{
  "model": "gpt-4o",
  "messages": [...],
  "tools": [...],
  "stream": true
}
```
**流式 SSE 解析**：
- `data: {"choices":[{"delta":{"content":"..."}}]}` → `StreamEvent::Chunk { text }`
- `data: {"choices":[{"delta":{"tool_calls":[...]}}]}` → `StreamEvent::ToolRequest`
- `data: {"choices":[{"finish_reason":"stop"}]}` → `StreamEvent::Done { stop_reason: "stop" }`
- `data: [DONE]` → 流结束信号
- 错误 HTTP 4xx/5xx → `StreamEvent::Error { retriable: 5xx }`

**初始支持模型**：`gpt-4o`、`gpt-4o-mini`

---

### 3.2 Anthropic 适配器

**端点**：`https://api.anthropic.com/v1/messages`
**认证**：
```
x-api-key: {anthropic_key}
anthropic-version: 2023-06-01
Content-Type: application/json
```
**请求格式**（Messages API）：
```json
{
  "model": "claude-sonnet-4-6",
  "messages": [...],
  "tools": [...],
  "stream": true,
  "max_tokens": 4096
}
```
**流式 SSE 解析**（格式与 OpenAI 不同，须独立解析）：

| Anthropic SSE 事件 | 映射到 StreamEvent |
|---|---|
| `content_block_start` (type=text) | 开始文本块，无输出 |
| `content_block_delta` (type=text_delta, text=...) | `Chunk { text }` |
| `content_block_start` (type=tool_use) | `ToolRequest` 开始（累积 input JSON）|
| `content_block_delta` (type=input_json_delta) | 累积工具参数（完整后发 ToolRequest）|
| `message_delta` (stop_reason=end_turn/tool_use) | `Done { stop_reason }` |
| `message_stop` | 流结束信号 |
| 错误 event | `Error { retriable: 529/529-类 }` |

**初始支持模型**：`claude-sonnet-4-6`（claude-sonnet-4-6）、`claude-haiku-4-5-20251001`

---

### 3.3 DeepSeek 适配器

**端点**：`https://api.deepseek.com/v1/chat/completions`（OpenAI 兼容格式）
**认证**：`Authorization: Bearer {deepseek_key}`（与 OpenAI 同格式）
**请求/响应格式**：与 OpenAI Chat Completions 完全兼容，**复用 OpenAI 适配器**，仅替换 base URL 与 key

**初始支持模型**：`deepseek-chat`、`deepseek-reasoner`

---

### 3.4 统一 StreamEvent 输出格式

GW 对客户端暴露统一 SSE 格式（client 无需感知 provider）：

```
event: chunk
data: {"turn_id":"...","text":"..."}

event: tool_request
data: {"turn_id":"...","tool_name":"...","params":{...}}

event: tool_result
data: {"turn_id":"...","tool_name":"...","result":"...","success":true}

event: diff_proposed
data: {"turn_id":"...","diff_id":"...","rel_path":"..."}

event: done
data: {"turn_id":"...","stop_reason":"stop|end_turn|tool_use|cancelled"}

event: error
data: {"turn_id":"...","code":"...","message":"...","retriable":false}
```

> 与客户端 `StreamEvent`（PI-AG-10 定义）完全对齐；格式变更须双侧同步 + A-SYS-I-P-03 回写。

## §4 BYOK 流程叙述

```
用户在 PORTAL 输入 provider 密钥（如 sk-openai-xxx）
  │
  ▼
PORTAL → POST /byok/keys { provider: "openai", key: "sk-openai-xxx" }
  │       （HTTPS，密钥不落 PORTAL 本地）
  ▼
GW → 校验 session token 有效 + 有效订阅 → 通过
GW → AES-256-GCM 加密密钥 → 写 byok_keys 表（与 users 表分离）
GW → 响应 { provider: "openai", key_preview: "sk-o...xxx" }（脱敏，不回传明文）
  │
  ▼（用户后续发起 AI 调用）
客户端 → POST /gate/check { session_token } → GW 裁决 passed
客户端 → POST /invoke/stream { model: "gpt-4o", messages: [...] }
GW → 取回加密密钥 → 解密（仅内存，不写日志）→ 调 OpenAI API
GW → SSE 流式转发 → 客户端
GW → 调用完成，计量元数据写 usage_logs（无正文，REQ-GW-006）
```

## §5 开发路线图（PI-GW-01~05）

> 叙述性，不含注册类治理块。PI 编号与 A-AG-X-L-02 的依赖引用对应。

**PI-GW-01　供应商适配层（三适配器 + 统一接口）**
- OpenAI 适配器：Chat Completions 请求构造 + SSE 解析 + StreamEvent 映射
- Anthropic 适配器：Messages API 请求构造 + 复杂 SSE 解析（多事件类型）+ StreamEvent 映射
- DeepSeek 适配器：复用 OpenAI 适配器，替换 base URL
- 统一调用函数：`invoke_stream(provider, model, messages, tools, key) → SSE<StreamEvent>`
- 验收：三适配器单测覆盖（mock SSE 响应）；统一输出格式与 §3.4 一致；provider 密钥不写任何日志

**PI-GW-02　BYOK 密钥管理**
- POST `/byok/keys`：接收 provider + key，AES-256-GCM 加密存入 byok_keys 表
- GET `/byok/providers`：列当前用户已配置供应商（脱敏 key_preview）
- DELETE `/byok/keys/:provider`：删除指定供应商密钥，立即生效
- 密钥取回：调用时解密至内存，不落磁盘，不写日志
- 验收：byok_keys 表与 users 表无外键依赖（物理分离）；删除后调用立即失败；明文不出现在日志

**PI-GW-03　CLOUD-GATE 服务端实现**
- POST `/gate/check { session_token }` → `{ verdict: "passed" | "denied", reason? }`
- 校验链：token 有效（联 AUTH JWT 验证）→ 订阅额度（联 LIC 查询）→ passed/denied
- 无有效订阅一律 denied（含 BYOK 用户）
- 断网/服务不可达：客户端收不到 passed，等同拒绝（客户端侧已实现）
- 验收：token 过期→ `{denied, reauth_required: true}`；无订阅→ `{denied}`；无任何服务端绕过路径

**PI-GW-04　最小 AUTH 服务（session token 签发）**
- POST `/auth/login`：邮箱+密码 或 magic link（认证方式由 CLOUD-01 人类决策）
- POST `/auth/token/refresh`：session token 续期
- POST `/auth/logout`：使 token 失效
- JWT 签发：HS256/RS256（CLOUD-01 决策后定）；含 user_id + exp + tier 信息供 GW 使用
- 验收：客户端 SM-SYS-01 认证段可通过（login/refresh/logout 三个端点）；token 过期后 gate/check 返回 denied+reauth

**PI-GW-05　端到端联调（与 PI-AG-11 配合）**
- 本地 dev 环境：GW（Node.js）+ 三供应商 BYOK 配置 + 客户端（Tauri）联调
- 验证：完整流程 `ag.start_turn → gate → invoke → SSE → channel → agStore`
- 验证：ChangeProposal → de.generate_diff → DiffProposed → 用户 accept → ws.apply_diff 落盘
- 验证：cancel 中断 → HTTP 连接关闭 → 无半落盘
- 性能：流式首字延迟 < 1s（BYOK OpenAI gpt-4o-mini 基准），NFR 在 G5 标定

## §6 G1 问题暴露清单（GW 模块）

| 编号 | 分类 | 状态 | 处置 |
|---|---|---|---|
| GW-01 | NEEDS_HUMAN_DECISION | 待决策 | 初始支持平台订阅（GW 持平台密钥）还是仅 BYOK？——影响 LIC 设计复杂度与上线节奏；平台密钥方案需确认 provider 账户与合规 |
| GW-02 | NEEDS_HUMAN_DECISION | 待决策 | 密钥加密方案（自管 AES 密钥 / KMS / 信封加密）——安全敏感，须人类批准（云 APC §5 禁区）|
| GW-03 | DESIGN_RISK | 显式延期 | Anthropic SSE 工具调用参数是流式 JSON（input_json_delta 片段），需增量解析；工具参数较大时内存缓冲策略归 L2 |
| GW-04 | BOUNDARY_OPEN | 显式延期 | provider 故障转移（主/备 provider）/ 限流处理策略归 L2 |
| GW-05 | ACCEPTANCE_MISSING | 显式延期 | 量化 NFR（网关首字延迟 / 可用性 / 计量准确率）延 G5 标定 |

## §7 后续动作

1. 人类决策 **GW-01**（平台订阅 vs 纯 BYOK 首发）与 **GW-02**（加密方案）——PI-GW-02 实现前置
2. 建 `A-GW-M-T-01_AI网关技术设计.md`（L2，叙述性，含 DB schema、API 骨架、加密方案、SSE 解析细节）
3. 客户端挂起项解除：ENT-02（BYOK 上送 API 形状）在 PI-GW-02 完成后回写 `A-ENT-M-T-01`；ENT-03（高频门禁延迟）在 CLOUD-04 完成后回写 `A-SYS-I-P-03`
4. 独立仓库建立时随云 APC 一并迁出（云 APC §3 流程）

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-23 | v1.0 | 初版（GW 模块 L1）：REQ-GW-001~006（供应商适配层/统一流式转发/计量/BYOK/CLOUD-GATE/内容不留存）；三供应商适配器规格（OpenAI/Anthropic/DeepSeek，含 SSE 解析差异）；BYOK 流程叙述；PI-GW-01~05 开发路线图；G1 暴露 GW-01~05；遵守驻留期硬约定（无注册类治理块）|
