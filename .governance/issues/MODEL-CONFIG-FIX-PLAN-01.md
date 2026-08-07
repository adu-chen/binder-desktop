# 模型配置功能修复方案（MODEL-CONFIG-FIX-PLAN-01）

> 性质：方案讨论稿（非 A 状态治理文档）。供人类审阅决策后，再按治理流程进入
> G1 需求（REQ）→ G2 技术设计（RULE/CHAIN/TERM/状态机）→ G3 开发计划 → 实现。
> 本文不创建任何静态规则定义；文中出现的规则/链路/TERM ID 均为**建议命名**，
> 需在对应设计文档中经流程确认后生效。

## 1. 背景与目标

Binder 的 AI 能力分四类调用场景：辅助续写（autocomplete）、局部修改（inline-assist）、
代理编辑（agent edit）、构建（build）。当前模型配置仅是一个「添加模型」弹窗，
存在四类缺口（详见 §2），无法支撑产品目标的模型体系：

- **预设模型**：云端配置的模型，订阅可调用（Binder 托管 / 自部署演进，见 A-GW-M-D-01 §3.5）
- **BYOK**：用户自带 provider 密钥配置的模型（TERM-CLOUD-004，密钥网关代持、客户端零残留）
- **模型选择**：全局默认 + 各能力独立配置

目标：建立「模型来源分层 + 能力级模型偏好 + 统一经网关调用」的模型配置体系，
使四类 agent 能力各自可独立配置模型，同时不破坏现有安全边界（X-FORBID-001/002、
门禁前置 X-SEQ-001、订阅门禁 TERM-CLOUD-001）。

## 2. 现状与差距分析

### 2.1 现状盘点

| 层 | 现状 | 位置 |
|---|---|---|
| UI | ModelConfig 弹窗：已配置模型（选择/删除）+ 添加模型 + BYOK 密钥上送 | `src/components/ag/ModelConfig.tsx` |
| 入口 | 仅代理编辑面板 Composer 顶栏模型下拉「＋ 添加模型」 | `src/components/ag/Composer.tsx` L456-L498 |
| 数据 | 全局单一模型列表 + 单一 selectedKey（localStorage）；默认清单硬编码 | `src/store/modelStore.ts` |
| 调用-代理编辑 | `startTurn` 传 `currentModel.provider/model` → `ag_start_turn` → GW | `agService.ts` L315-L320 / `lib.rs` L6232 |
| 调用-构建 | 复用代理编辑回合（session mode=build），无独立模型参数 | `ag_start_turn`（同命令） |
| 调用-辅助续写 | `requestAssist` **不传 model** → 后端默认 `deepseek/deepseek-chat` | `Editor.tsx` L1023 |
| 调用-局部修改 | `requestAssist` **不传 model** → 后端默认 `deepseek/deepseek-chat` | `Editor.tsx` L1848 |
| 云端-密钥 | `/invoke/stream` 密钥优先级：api_key > BYOK 代持 > 平台 env | `cloud/dev-gateway/server.mjs` L103-L105 |
| 云端-预设 | 无清单下发机制；「预设」= 客户端硬编码默认清单 + 平台 env key | — |

### 2.2 差距结论

1. **模型来源不分层**：预设（订阅可调用）与 BYOK 混在同一本地列表，无 `source` 标记。
2. **无能力维度偏好**：仅全局一个 `selectedKey`，无法按四类能力独立配置。
3. **assist 两条链路未接线**：前端不传 provider/model，后端已有 `provider/model` 参数
   （`lib.rs` L6836-L6837）但从不生效——配置后辅助续写/局部修改仍固定 deepseek。
4. **预设清单非云端下发**：本地硬编码撑不住订阅模型矩阵与档位映射（GW-06 演进项）。

## 3. 目标架构

```
模型来源层                    能力偏好层                      调用层
┌───────────────┐    ┌──────────────────────────────┐   ┌────────────────────┐
│ 预设模型       │    │ 模型配置（每项可独立指定）      │   │ 统一模型解析         │
│ 云端下发·订阅  │───▶│  辅助续写 autocomplete        │   │ 能力偏好 → provider │
├───────────────┤    │  局部修改 inline-assist        │──▶│  /model → 门禁 →   │
│ BYOK          │    │  代理编辑 agent-edit           │   │ AI 网关 → 供应商    │
│ 密钥网关代持   │───▶│  构建 build                   │   │ （X-SEQ-001）       │
└───────────────┘    └──────────────────────────────┘   └────────────────────┘
```

要点：
- **模型注册表**（ModelRegistry）：`modelKey(provider/model)` 唯一，含 `source: preset|byok`、
  `status`（可用/需订阅/不可用）、`label`。
- **能力偏好**（CapabilityPref）：四类能力各一项，值 = `follow-global`（跟随全局默认）或显式
  指定 modelKey。全局默认模型保留（现 `selectedKey` 演化为全局默认）。
- **调用统一解析**：所有调用点（assist×2 + agent + build）从「能力偏好」解析出 `provider/model`
  再进后端，后端与 GW 无需感知偏好逻辑（偏好解析放客户端，云端保持薄端）。

## 4. 数据模型设计（建议）

### 4.1 ModelOption 演进

```ts
interface ModelOption {
  provider: string;
  model: string;
  label: string;
  source: "preset" | "byok";   // 新增：来源分层
  status?: "available" | "requires_subscription" | "unavailable"; // 预设模型可用性（P2）
}
```

### 4.2 能力偏好（CapabilityPref）

```ts
type AgentCapability = "autocomplete" | "inlineAssist" | "agentEdit" | "build";
interface CapabilityPref {
  autocomplete: { mode: "followGlobal" } | { mode: "explicit"; modelKey: string };
  inlineAssist:  { mode: "followGlobal" } | { mode: "explicit"; modelKey: string };
  agentEdit:     { mode: "followGlobal" } | { mode: "explicit"; modelKey: string };
  build:         { mode: "followGlobal" } | { mode: "explicit"; modelKey: string };
}
```

持久化位置：P0-P1 沿用 localStorage（与现状一致）；是否并入 workspace.db 统一设置
（涉及 schema 变更）由人类决策，属 P3 前单独立项项。

## 5. 分阶段实施计划

### P0 — 最小打通（不改变 UI 形态，收益最大）

目标：让「配置的模型」真正作用于全部四类能力。

| 改动 | 文件 | 内容 |
|---|---|---|
| 1 | `src/store/modelStore.ts` | `ModelOption` 增加 `source`（默认 `"preset"`，保持向后兼容读取）；不改动现有 API |
| 2 | `src/services/agService.ts` | `AssistReq` 增加可选 `provider`/`model`；`requestAssist` 透传给 `ag_request_assist` |
| 3 | `src/components/ed/Editor.tsx` | 两处 `requestAssist` 调用（L1023 autocomplete、L1848 inline）从能力偏好/`modelStore.current()` 解析传入 `provider/model` |
| 4 | 测试 | `src/services/agRuntime.test.ts`：`requestAssist` 契约测试补充 provider/model 透传断言；`covers` 关联既有规则 |

后端 `ag_request_assist` 已支持 `provider/model` 参数，**无需 Rust 改动**。

### P1 — 来源分层 + 能力偏好（数据模型重构）

目标：建立模型注册表与能力偏好，UI 支持按能力配置。

| 改动 | 文件 | 内容 |
|---|---|---|
| 1 | `src/store/modelStore.ts` | 重构为 ModelRegistry（source 标记 + 全局默认）+ CapabilityPref（四能力偏好）两个状态块；保留 `addModel/removeModel/select` 兼容入口 |
| 2 | `src/components/ag/ModelConfig.tsx` | 弹窗升级为分区：①已配置模型按「预设 / BYOK」分组展示 ②能力偏好区（四行下拉：跟随全局 / 指定模型） |
| 3 | `src/components/ag/Composer.tsx` | 模型下拉展示来源标记（如「BYOK」角标）；发送时按 `agentEdit` 偏好解析（构建模式按 `build` 偏好） |
| 4 | `src/components/ed/Editor.tsx` | assist 两处调用改为按 `autocomplete` / `inlineAssist` 偏好解析（替代 P0 的 `current()`） |
| 5 | 测试 | modelStore state 测试（P1-新）、ModelConfig/Composer 组件测试、agRuntime 契约测试更新 |

### P2 — 预设模型云端下发

目标：预设清单由云端下发（订阅档位映射），本地不再硬编码。

| 改动 | 文件 | 内容 |
|---|---|---|
| 1 | `cloud/dev-gateway/server.mjs`（+新增 `models.mjs`） | 新增 `GET /models`：返回预设模型清单（provider/model/档位/可用性）；与 `checkGate` 订阅裁决联动 |
| 2 | `cloud/dev-gateway/README.md` | 登记 /models 接口与预设模型配置方式 |
| 3 | 客户端新 store 或扩展 modelStore | 预设清单拉取 + 本地缓存（含 `loading/loaded/failed` 状态）；BYOK 模型与预设清单合并进注册表 |
| 4 | `src-tauri/src/ent/` | 视契约而定：新增拉取预设清单的 Tauri 命令（或前端直连，按 SYS I-P-03 云契约对齐） |
| 5 | 订阅档位映射 | 预设模型标 `requires_subscription`，调用前经既有 CLOUD-GATE 门禁（不新增绕过路径，X-FORBID-002） |

### P3 — 完整设置页 + BYOK 管理

目标：模型配置升级为全局设置页，补齐 BYOK 生命周期。

| 改动 | 文件 | 内容 |
|---|---|---|
| 1 | 新设置页组件（或扩展 ModelConfig） | 全局设置入口（如 Workbench/左栏）+ Composer 双入口；模型配置为设置页「模型」分区 |
| 2 | BYOK 状态展示 | GW 支持 `GET /byok/providers`（脱敏 key_preview）与 `DELETE /byok/keys/:provider`（A-GW-M-D-01 PI-GW-02 已有规划）；客户端展示已配 provider 与删除入口 |
| 3 | 模型能力探测 | 对配置的模型做轻量能力探测（多模态/工具调用等），A 类债务（ModelConfig/Composer 注释中既有记录） |
| 4 | 统一设置体系 | 偏好存储并入统一设置（modelStore 注释中 A 类债务），独立立项 |

## 6. 接口契约变更

| 接口 | 变更 | 阶段 |
|---|---|---|
| `ag_request_assist`（Tauri command） | 无变更（已支持 `provider`/`model`）；前端补传 | P0 |
| `AssistReq`（agService.ts） | 新增可选 `provider`/`model` | P0 |
| `GET /models`（dev GW） | 新增：预设模型清单 | P2 |
| `GET /byok/providers`、`DELETE /byok/keys/:provider`（dev GW） | 新增：BYOK 状态/删除 | P3 |
| `ag_start_turn` | 无变更；前端按能力偏好传参 | P0/P1 |

安全边界不变量（不得破坏）：密钥永不落地客户端（X-FORBID-001）；门禁裁决仅服务端
（X-FORBID-002，客户端不缓存 Pass）；所有调用经 CLOUD-GATE（TERM-CLOUD-001）。

## 7. 治理要求（进入 G1/G2 时执行）

1. **TERM 注册建议**（ADUS 术语注册表当前缺）：`PresetModel`（预设模型，禁用别名：
   内置模型/托管模型/官方模型）。BYOK 已有 `TERM-CLOUD-004`。
2. **需求层**：新增 REQ-AG-*（能力独立配置模型、预设模型清单下发、模型选择 UI）；
   经 REQ_MAP 映射到技术规则。
3. **规则层**（建议命名，最终在设计文档确认）：
   - BR-AG-DATA-*：ModelOption.source 分层语义；能力偏好解析规则（跟随全局 / 显式覆盖）
   - BR-AG-STATE-*：预设清单拉取状态（loading/loaded/failed，或复用现有模式）
   - 复用既有：BR-ENT-DATA-001（BYOK 零残留）、BR-ENT-VERIFY-001（门禁前置）
4. **链路**：AG-TURN / AG-ASSIST 已有；如预设清单下发成为正式链路，建议新增
   `AG-MODEL-CATALOG`（planned 状态先行）。
5. **状态机**：能力偏好为纯配置，豁免状态机强制阈值；预设清单拉取若为有状态链路，
   按 APC §6 强制阈值判断是否需要状态机（建议轻量，复用现 store 模式）。
6. **@GOV**：本次改动触及 `agService.requestAssist`、`Editor.tsx` assist 调用点、
   `modelStore` 等已有 @GOV 块，实施时须同步更新 boundary（in/out 承载实体变化）。
7. **测试**：新增/更新的测试用 `// covers: BR-*` 关联规则；能力偏好解析为纯逻辑，
   须在 state/service 层自动化（APC §10 细小交互强制自动化）。

## 8. 风险与待确认项

| # | 事项 | 说明 | 决策点 |
|---|---|---|---|
| 1 | 「局部修改」语义 | 项目术语中 = inline-assist（⌘K）；DiffEdit（DE）当前不直接调模型（diff 由代理回合 propose_edit 产出）。若未来 DE 独立生成才需独立模型配置 | 确认是否含 DiffEdit |
| 2 | 预设模型权威源 | 云端（PORTAL/GW 运营配置）是配置方，桌面端只是消费方；`GET /models` 契约归 SYS I-P-03 细化 | 确认分工与接口归属 |
| 3 | 能力偏好持久化 | localStorage（P0-P1 沿用）vs workspace.db 统一设置（P3，schema 变更） | 确认是否本轮引入 |
| 4 | BYOK 与模型绑定粒度 | 当前一个 provider 一个 key；模型可自由添加，未校验「该 provider 是否有可用 key」 | 确认是否做绑定校验 |
| 5 | 构建模式模型归属 | 构建复用 AG 会话（mode=build），模型配置归 AG；若 CBT 独立于 AG 调用则需 CBT 侧配置 | 确认归属 |
| 6 | 预设模型与订阅档位 | 档位映射（哪档订阅可用哪些预设）归云端 LIC/GW 契约；1.0 EP 主路径 DeepSeek 托管 + BYOK | 确认档位模型 |

## 9. 验证计划

- **P0**：`npx vitest run src/services/agRuntime.test.ts`（requestAssist 契约含 provider/model 透传）；
  `npx tsc --noEmit`；`npm run governance:generate` + `npm run governance:audit`。
- **P1**：modelStore state 测试、ModelConfig/Composer 组件测试（vitest）；`npm run test:related`。
- **P2**：dev GW `GET /models` 接口冒烟；预设清单缓存测试；门禁联动人工验证。
- **P3**：设置页人工验收（预设/BYOK/能力映射三区）；BYOK 删除即生效验证。
- 每阶段以实际 changed_files 运行 `test:plan` / `test:related`，不得手选测试。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-08-06 | v0.1 | 初版：现状/差距分析、目标架构、四阶段实施计划、接口契约、治理要求与待确认项 |
