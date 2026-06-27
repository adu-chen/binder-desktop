# 阶段报告 — AG 对话代理能力 + 云端真实化（dev GW）

- 日期：2026-06-27
- 类型：阶段性闭合报告（叙述性，非治理 SSOT，不含注册类治理块）
- 范围：从「全桩占位」到「可用 AI 写作助手 + 真门禁的 dev 云网关」
- 工作树状态：干净，全部交付已逐 PI 提交（最新 `c49cbab`）

---

## 1. 概述

本阶段把 Binder 的 AI 能力从「界面有、逻辑桩」推进到端到端真实可跑：

- **设计先行**：把提示词工程调研降级为 R 级参考，新建 L3《提示词架构设计》并与旧版去重，更新集成计划为前后端双轨。
- **AG 客户端**：系统提示词 → 工具注册/执行 → 真实流式（解桩）→ 上下文供给（历史/预算截断/意图沿用/AST·KB 补强/REF @引用）→ 块级锚定编辑（文本 + docx，真实 blockId，ED 实时校验）→ 前端工具卡片与 diff 真预览。
- **云端真实化（dev GW）**：临时本地 AI 网关，三供应商归一化流式 + Anthropic function-calling 翻译 + BYOK 代持；再分四步真实化为「模块化单体 + AUTH + 真门禁 + 持久化/计量 + 订阅/额度」。
- **BYOK**：客户端零残留录入，密钥经网关代持（加密落盘），客户端永不直连供应商（X-FORBID-001）。

贯穿的硬约束：客户端不持有供应商密钥、不直连供应商（X-FORBID-001）；门禁由服务端每次裁决、客户端不缓存「已通过」（X-FORBID-002）；AI 改文档只走 AG→DE→WS（X-FORBID-006）；门禁前置于模型调用（X-SEQ-001）。

---

## 2. 交付清单（按主题，含提交哈希）

### 2.1 设计 / 治理文档
| 提交 | 内容 |
|---|---|
| `8f34f2f` | 提示词工程迭代：新建 L3《A-AG-M-T-02 提示词架构设计》（注册 BR-AG-DATA-004/005、TERM-AG-005，13 层提示词表+结构示例）；M-T-01 去重（移除与新版冲突的截断/排序，单一归属 BR-AG-DATA-005）；调研文档降级 R-AG-X-R-01（A→R 参考级） |
| `d5ece67` | PI-AG-10C 流式契约冻结：`AgTurnEvent`（6 变体，tag="type"）落 `ipc/mod.rs`，回写接口协议 §2.1（与 SYS `StreamEvent` 分层不重名） |
| `b9e1101` | A-GW-M-D-01 AI 网关功能需求：三供应商适配 + BYOK + CLOUD-GATE，REQ-GW-001~006 |
| `a1a84eb` | GW-01 裁决：平台订阅 + BYOK 双模共存 |

### 2.2 AG 客户端 — 提示词 / 工具 / 流式
| 提交 | 内容 |
|---|---|
| `0f692aa` | PI-AG-07 系统提示词正文：Chat/Build/Assist 三常量 + 当前轮次原则 |
| `1e28d10` | PI-AG-08A 工具注册表：4 工具 + ReadOnly/WriteIntent 权限分级 |
| `dbd117c` | PI-AG-08B 真实工具：function-calling 循环 + 只读三工具（read_doc/list_docs/search_text） |
| `692f419` | PI-AG-08C propose_edit 写工具：AI 改文档闭环（X-FORBID-006） |
| `e8bd76c` | PI-AG-WIRE-01 客户端接 dev GW：真实流式 AI 解桩 |
| `3a8656c` | PI-AG-09A 注入当前文档 + 提示词诚实化（修「看不到当前文档」+ 杜绝假称工具） |

### 2.3 AG 客户端 — 块级锚定编辑
| 提交 | 内容 |
|---|---|
| `a896412` | PI-AG-08D 前端工具卡片：ToolRequest/ToolResult 投影 ToolCallCard |
| `655b437` | PI-AG-08E 块级锚定编辑：编号块列表 + propose_edit 块模式 |
| `4f7ac89` | PI-AG-08F docx/html 块编辑：按 data-block-id 分块 + 非破坏重建 |
| `4442a31` | PI-AG-08G 真实 blockId 锚：按 data-block-id 稳定锚定 |
| `fe53c8e` | PI-AG-08H ED 真实锚联动：ed.locate_block 实时校验块编辑锚 |

### 2.4 AG 客户端 — 上下文供给（PI-AG-09 全量）
| 提交 | 内容 |
|---|---|
| `eb180b8` | PI-AG-09B 对话历史 + P0–P3 预算截断（AI 记上文、大文档不爆 token） |
| `f5e08b5` | PI-AG-09C 意图沿用判定：默认弱化历史、显式沿用才提权（BR-AG-DATA-004） |
| `cd6b601` | PI-AG-09D AST/KB 补强：记忆 + 知识 P2 注入 |
| `bf82ef4` | PI-AG-09E REF @mention 供给：完整 PI-AG-09 收尾 |
| `ce4c0e4` | PI-AG-09F REF 块级 @mention：选区精确锚引用 |
| `cb91c2e` | PI-AG-09G 引用非当前文档的块 + mention 样式收尾 |
| `7d54bee` | PI-AG-09H 内联 @ 自动补全：输入 @ 弹文档下拉 |
| `2c8b2a2` | PI-AG-09I diff 卡 after 真预览：显示 AI 提案内容 |

### 2.5 云端真实化（dev GW）+ BYOK
| 提交 | 内容 |
|---|---|
| `2cd872c` | PI-GW-DEV-01 临时本地 dev AI 网关：三供应商 + 归一化流式 |
| `f2439b7` | PI-GW-DEV-02 Anthropic function-calling：请求翻译 + tool_use（可选，不强制配 key） |
| `7224fe6` | PI-BYOK-DEV-01 BYOK 客户端入口：界面填 key，零残留送网关 |
| `90402bb` | PI-GW-DEV-03 云端真实化①：模块化骨架 + 最小 AUTH + 真门禁 |
| `053126b` | PI-AG-GATE-01 云端真实化②：客户端登录接线 + 门禁端到端强制 |
| `9e3469e` | PI-GW-DEV-04 云端真实化③：持久化 + 用量计量（BYOK 加密存） |
| `c49cbab` | PI-GW-DEV-05 云端真实化④：LIC 订阅/额度 — 补全 CLOUD-GATE |
| （本次） | PI-GW-DEV-06 云端真实化⑤：正式 GW-02 信封加密（KMS 同形）+ SQLite DB 替 JSON |

---

## 3. 架构（端到端链路）

```
用户输入（Composer：@引用文档/块、内联补全、🔑 BYOK 录入）
  └─ agService.ts：捕获历史 + mentions + blockRefs
       └─ ag_start_turn（Rust）：组装上下文
            · P0 当前文档（块视图，永不裁）+ 用户消息
            · P1 历史（预算截断）+ REF 引用块
            · P2 AST/KB 补强（记忆 + 知识）
            · 意图沿用判定（默认弱化历史）
            · 装配序 system→aug→history→mentions→doc→intent→user
       └─ ent/gateway.rs：ensure_session_token（dev 自动登录）→ 带 token 调 GW
            └── HTTP SSE → AgTurnEvent → Channel<AgTurnEvent> → 前端
  ┌──────────────────────── dev GW（cloud/dev-gateway，零依赖 Node ESM）────────────────────────┐
  │ server.mjs 路由：/health /auth/login /gate/check /byok/keys /invoke/stream                  │
  │   /invoke/stream：① checkGate(token)  —— X-SEQ-001 门禁前置                                  │
  │        gate.mjs = verifyToken(auth.mjs) + evaluate(lic.mjs：订阅 dev-true + 当日额度/配额)   │
  │        令牌问题→401（客户端重登录）；超额/无订阅→429（不重登录）                              │
  │   ② recordUsage（usage.mjs：脱敏元数据，不含正文，REQ-GW-006）                                │
  │   ③ 密钥优先级：请求体 > BYOK 代持（byok.mjs 加密落盘）> 平台 env                              │
  │   ④ streamProvider（providers.mjs：openai/deepseek=OpenAI兼容；anthropic=Messages 翻译）     │
  │   持久化：store.mjs（.data/ JSON，gitignore）+ crypto.mjs（AES-256-GCM，GW-02-lite）           │
  └──────────────────────────────────────────────────────────────────────────────────────────┘
       └─ 工具回路：GW 转发供应商 tool_calls → 客户端工作区侧执行 → 回灌再调（最多 6 轮）
       └─ 写编辑：propose_edit → DE 生成 diff（锚定 Located）→ DiffProposed → 用户接受 → WS 整文写回
```

---

## 4. 治理与隔离留痕

- **EXEMPTION-GW-DEV-01**（人类 2026-06-26 授权）：临时偏离云 APC「云代码迁出独立仓库」约定，扩展为「云端真实化在本仓 `cloud/` 下隔离迭代」。理由：真实云仓库尚不便频繁调试。
- **隔离已验证**：`cloud/` 对客户端治理不可见 —— 0 个 ADUS 引用、0 个治理块（@GOV/RULE/CHAIN/CONSTRAINT/TERM），不参与 `governance:generate` / `audit`，等同客户端外部依赖。
- **正式版仍按既定路线**：迁独立云仓库、改 TypeScript、补正式 AUTH/LIC、真实订阅门禁、GW-02 正式密钥加密（KMS/信封）、内容不留存审计。dev GW 的 README 与各模块头注均显式标注「临时 dev 脚手架，非生产」。
- **工作树纪律**：全程逐 PI 显式 `git add <具体文件>` 提交，从不用 `git add -A`；与并发 FE 进程（docx 编辑链等）互不覆盖；一次 ADUS regen 残留已 `git checkout` 还原。

---

## 5. 验证

- **门禁端到端**：无 token 调用 `/invoke/stream` → 401；配额=0 → `/gate/check` 返 `quota_exceeded` + 调用 429；默认配额 → passed（used:0, limit:50）。
- **持久化**：会话令牌、BYOK 密钥跨 GW 重启不丢。
- **BYOK 加密**：落盘密文，磁盘 0 明文。
- **Anthropic 翻译**：tool_use 流式归一化为同一 `tool_calls` 事件。
- **语法/构建**：dev GW `node --check` 全通过；Rust 侧各 PI 编译通过。
- **上下文供给**：blocks 解析单测 8/8；历史预算截断、意图沿用启发式、当前文档 P0 永不裁均按 BR-AG-DATA-004/005 落实。

> 说明（验证纪律）：UI 交互态（卡片渲染、@ 补全弹层视觉）依赖人工运行确认；已验证项为代码/接口/网关行为层。

---

## 6. 如何运行

```bash
# 1) 起 dev GW（用你自己的 provider key；默认 deepseek）
cd cloud/dev-gateway
GW_DEEPSEEK_KEY=sk-你的key node server.mjs
# [dev-gateway] http://localhost:8787 — providers: openai, deepseek, anthropic

# 2) 客户端正常起 Tauri；对话面板即走真实流式
#    - 自动 dev 登录拿会话令牌 → 每次调用经服务端门禁
#    - 🔑 BYOK 录入：界面填 key，零残留经网关代持
#    - 可选 anthropic：仅 provider=anthropic 且配 GW_ANTHROPIC_KEY/BYOK 时启用
```

配额可调：`GW_DAILY_QUOTA=100`。端口可调：`GW_PORT=9000`。

---

## 7. 未竟事项（决策/基建门控，非本阶段自启）

| 项 | 现状（dev） | 正式版需要 |
|---|---|---|
| 订阅/计费 | free 档恒有效 | 真实订阅态/档位/计费、额度窗口与重置 |
| 密钥加密 | ✅ 信封加密（KMS 同形，KEK 与密文分离；PI-GW-DEV-06） | KEK 换托管 KMS（KEK 不出 KMS） |
| 存储 | ✅ SQLite（`node:sqlite`，sessions/byok_keys/usage；PI-GW-DEV-06） | 换托管数据库（Postgres 等）+ 迁移脚本 |
| 仓库 | 本仓 `cloud/` 隔离（豁免） | 迁独立云仓库、TypeScript 重写 |
| 审计 | 脱敏元数据计量 | 正式「内容不留存」审计合规 |

> PI-GW-DEV-06（2026-06-27）已把「密钥加密」「存储」两项从 lite/文件推进到与正式版**同形**的 dev 实现：
> 信封加密（每条独立 DEK、KEK 库外分离、与云 KMS 同一 wrap/unwrap 接口）+ SQLite 持久化。
> 余下为「换托管 KMS/DB + 迁独立仓」的基建迁移，非功能缺口。详见 `.governance/issues/PI-GW-DEV-06.md`。

---

## 8. 安全提醒（必办）

- ⚠️ 会话中明文出现过 DeepSeek key `[REDACTED-API-KEY]` —— **请立即在 DeepSeek 控制台吊销/轮换**，勿继续使用，勿提交进仓库。
- dev GW 密钥仅取自 BYOK 代持 / env，不写日志；`.env` 与 `.data/` 已 gitignore。
