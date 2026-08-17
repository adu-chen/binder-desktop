---
文档编号：AG-EVOLUTION-PLAN-01
文档状态：A
负责模块：AG
文档职责：AG Agent 能力修复与阶段化演进方案——将审计报告（AG-FUNCTIONAL-AUDIT-01）的发现转化为可执行工程方案；建立 Agent Architecture Target Model（九层能力模型）；按架构依赖给出 Phase 0–4 路线图；明确 Binder AG v2 的冻结边界、持续演进边界与长期基础设施判定
上游约束：AG-FUNCTIONAL-AUDIT-01（审计发现 D1–D6、模块拆分 M1–M8）、A-AG-M-D-01/T-01/T-02/S-01/I-P-01（AG 规则与契约 SSOT）、A-CORE-C-P-01_APC.md（§6 状态机驱动、§9 NFR）
阅读对象：项目负责人与架构设计者
审计结论：本轮不做任何修改，输出演进方案
治理成熟度：draft
---

# AG Agent 能力修复与阶段化演进方案

## §0 文档定位与阅读指南

本方案是审计报告的**工程化落地层**，不重复审计问题清单（见 AG-FUNCTIONAL-AUDIT-01 §4），
而是回答三件事：

1. **每个问题的修复机制是什么**（§1）：根因 → 修复目标 → 修改点 → 验证方式，全部落到具体机制，
   不使用「优化 / 增强 / 完善」等不可验证措辞。
2. **目标架构是什么**（§2–§4）：以 Agent 能力层组织（而非文件列表），给出九层
   Agent Architecture Target Model、逐层修复方案、五大重点深度设计。
3. **按什么顺序演进**（§5–§6）：按架构依赖（而非功能数量）给出 Phase 0–4 路线图，
   以及 Binder AG v2 的冻结边界与长期基础设施判定。

阅读顺序建议：先读 §2（目标架构模型，建立全局），再读 §1（契约修复，先解矛盾），
然后 §3（逐层方案）与 §4（深度设计），最后 §5–§6（路线图与边界）。

---

## §1 契约修复方案（对应审计发现 D1–D6）

> 本节只做修复机制设计。每个修复项给出：真实根因 / 为什么当前设计会产生 / 修复目标 /
> 修复后系统状态 / 修改模块 / 数据结构与接口与状态机与 Prompt 变化 / 验证方式。
> 契约修复是 Phase 0，先于任何能力增强（§5）。

### FIX-D1 新建文档裁决语义统一（对应 D1，需人类先裁决）

**真实根因**：BR-AG-DATA-003（2026-07-04 版）规定新建文档"以空基准表达、经 DE diff 裁决闸门"，
但 2026-07 工具链引入 `create_doc`（StructureWrite）时，其工具描述与 SYSTEM_PROMPT_CHAT 均声明
"新建文件直接创建，不经过 DiffCard 审阅"。**裁决闸门语义与直写工具语义并存且未在规则层消解**，
形成「设计规则要求走 diff、实现路径提供直写」的双真值。

**为什么当前设计会产生**：BR-AG-DATA-003 修订时（2026-07-04）`create_doc` 尚不存在，规则撰写者
按"新建=空基准 ChangeProposal"假设；后续 BDocx 系列扩展（需支持 .docx 模板化新建）引入
`create_doc` 时，只在其工具描述层声明了"直写"，未回写 BR-AG-DATA-003 的豁免条款，也未在
X-FORBID-006 中区分"新建"与"修改"两种写入语义。

**修复目标（先裁决，再实现）**：把「新建文档是否豁免 diff 裁决」收敛为**唯一语义**，并在
X-FORBID-006 约束层显式表达，消除"直写 vs 裁决"双真值。

**修复后预期系统状态**：
- 裁决 A（**新建豁免 diff，推荐**，与当前实现一致）：X-FORBID-006 增补豁免条款"新建文件（目标不存在）
  属结构创建，经 create_doc 直接落盘，不产生 ChangeProposal；对既有文档的一切修改仍必须经 DE diff"。
  BR-AG-DATA-003 同步改为"新建文档经 create_doc 结构创建路径；create_doc 的目标存在性检查
  （已存在即拒绝、不覆盖）即新建的人类确认闸门"。
- 裁决 B（新建不免除 diff）：create_doc 降级为"仅产出一个以空基准表达的 ChangeProposal"，
  create_doc 不直写，统一走 propose_edit 空基准路径。

**修改模块**：AG（tool_registry、prompts、lib.rs ag_create_doc）、DE（若裁决 B）、规则文档
（M-T-01 BR-AG-DATA-003 / X-FORBID-006）。

**数据/接口/状态机/Prompt 变化**：
- 裁决 A：无数据结构变化；X-FORBID-006 规则文本增补豁免条款（GOVERNANCE_CHANGE）；
  SYSTEM_PROMPT_CHAT 维持现状（已声明新建直写）；create_doc 保持 StructureWrite 权限。
- 裁决 B：propose_edit 工具参数增加 `operation=create_doc`（WriteIntent）；
  SM-AG-02 执行路径改为产出 ChangeProposal；Prompt 删除"新建直写"声明。

**验证方式**：
- 裁决 A：测试 `ag_create_doc` 对已存在目标返回拒绝（WS_EXISTS，不覆盖）；静态断言
  X-FORBID-006 规则文本含"新建豁免"且 BR-AG-DATA-003 无残余"新建经 ChangeProposal"表述冲突
  （grep 规则文档 + governance:audit 通过）。
- 裁决 B：测试 create_doc 调用后 DE pending_diffs 增加一条空基准 diff、磁盘文件未变化（未接受前）。

---

### FIX-D2 open_session 草稿确认语义回写（对应 D2）

**真实根因**：2026-07-26 D1 审计修复把 open_session 的"自动应用事务"改为"只改 session 内容、
不调用 accept_draft_diff、accept 由后续用户操作驱动"，但 A-AG-I-P-01 v1.31 仍写"立即执行 ED session
自动应用事务并禁用卡片裁决"。"自动应用"与"不自动确认"两段语义在实现层已分离，文档层未同步，
导致 draft_mode=open_session 的卡片呈现为 readonly、却实际已改 session 内容的中间态缺乏契约定义。

**为什么当前设计会产生**：open_session 语义历史上是"自动应用（apply）+ 自动确认（accept）"一体的；
D1 审计发现"自动确认"会跳过用户对非激活文档修改的知情权，于是拆开 apply 与 accept，但契约文档
未跟随拆分。

**修复目标**：把 open_session 定义为**三段式**：`apply 自动执行（可回滚）→ accept 由用户显式触发 → 
落盘`。I-P-01 与 deService 实现逐字段对齐。

**修复后预期系统状态**：open_session diff 的生命周期 = DiffProposed → 前端自动 apply 到目标
session（记录 base 基线、可 reject 回滚）→ 卡片状态 pending(readonly，展示"已应用到打开中的文档，
待你确认") → 用户切到该文档确认 → accept_draft_diff → 保存落盘。任何状态下 reject 均按 base 基线回滚。

**修改模块**：DE（deService/deStore）、AG（agService 卡片状态投影）、I-P-01 §2.1 契约文本、
ED（apply 回滚语义核对）。

**数据/接口/状态机变化**：`DiffView` 新增状态位 `appliedButUnconfirmed: boolean`（区分"纯展示"
与"已 apply 待确认"）；I-P-01 明确 draft_mode=open_session 的事件序列
（DiffProposed → apply → pending → accept/reject）；SM-DE-01（DE 状态机）补
`applied_pending_confirmation` 中间态（若 DE G2 状态机已登记，需回写其 transition）。

**验证方式**：集成测试构造 open_session diff：断言 apply 后 session 内容已变但 `accept` 未调用
（磁盘未变）；reject 后 session 恢复 base 基线；卡片呈现"已应用待确认"而非"已接受"；
`de.accept_draft_diff` 后正常落盘。文档断言：I-P-01 含"appliedButUnconfirmed"字段定义。

---

### FIX-D3 提示词分层映射表与补强层接入协议（对应 D3）

**真实根因**：M-T-02 定义 13 层 + P0–P3 截断阶梯，但实现只有 4 个 ContextSource
（ExplicitMention/CurrentDoc/Memory/Knowledge），且 Memory/Knowledge 在代码中标记为 P1（设计为 P2）。
缺"设计层 → 代码层"的显式映射，任何新补强源（TPL、任务状态、工作区依赖）接入时都无法判定
"落在哪个截断位"，必然产生实现漂移。

**为什么当前设计会产生**：M-T-02 是设计文档，规定的是"层"的语义；`context.rs` 的 ContextSource
是为 `assemble_context` 裁剪函数服务的枚举，二者建模对象不同（层 vs 源），历史上靠人脑对应，
没有一张权威映射表。TPL/任务状态层/工作区依赖层当前无消费方（1.0 EP 不阻断），所以映射缺口
尚未被触发，但接入时必然触发。

**修复目标**：建立**单一权威映射表** `PromptLayerRegistry`（设计层 ↔ 代码层 ↔ 截断位 ↔ 信任等级），
并把补强层接入协议（注入格式、权重声明、门控条件）定义为可复用机制，而非每次手写拼接。

**修复后预期系统状态**：
- `context.rs` 的 ContextSource 扩展为 6 源：ExplicitMention(P0)、CurrentDoc(P0)、HistorySameDoc(P1)、
  HistoryCrossDocWeak(P1)、Augmentation(P2，含 Memory/Knowledge/Template 子类)、WorkspaceDeps(P3)。
- `assemble_context` 与 `select_recent_history` 的优先级常量改由一张 `TRUNCATION_STEP` 表驱动
  （P0 永不截 / P1 尽量留 / P2 先截 / P3 按需），枚举值带 `tier` 标注，杜绝层号漂移。
- 新补强源接入 = 在注册表加一行（来源模块 + tier + 注入格式 + 门控条件），不再手写拼接逻辑。

**数据/接口/状态机/Prompt 变化**：
- 代码：ContextSource 枚举扩展；新增 `prompt_layer_registry.rs`（或并入 context.rs）承载映射表；
  `assemble_agent_turn_messages` 改为按注册表驱动（不再手写顺序）。
- 接口：无 wire 契约变化（组装是后端内部）。
- Prompt：新增层的注入格式统一为 `[补强信息] 来源=X 权重声明=...`，由注册表生成。
- 状态机：无变化。

**验证方式**：
- 单元测试：注册表驱动装配产物与"手写顺序"参考实现逐字节一致（黄金快照测试）。
- 截断测试：构造超预算输入，断言 P2（Memory/Knowledge/Template）先于 P1 被裁、P0 永不裁
  （已有 assemble_context 测试扩展 3 例：新 6 源枚举全覆盖）。
- 接入测试：以 Template 源为样例注册一行，断言其注入串含权重声明、超预算先被裁
  （证明"新增源只改注册表"的机制可用）。

---

### FIX-D4 turn.rs 旧表持久化双轨清理（对应 D4）

**真实根因**：`turn.rs` 的 `persist_turn/recover_turns` 是 PI-AG-01/03 G4 骨架产物（单表、
无 session_id/status）；PI-AG-12 落地三表 `history.rs` 后，新链路（ag_start_turn/save_turn）全部走
history.rs。旧函数无调用方但未删除，形成"两套持久化 API 并存"。

**为什么当前设计会产生**：迭代式交付的必然产物——骨架实现先上线验证状态机，历史持久化模型
升级为新三表后，旧 API 未同步退役（无编译错误即无感知）。

**修复目标**：确认旧函数无调用方后**退役删除**（`turn.rs` 保留状态机与门禁部分，删除
persist_turn/recover_turns 及对应测试），消除双 API 入口。

**修改模块**：`ag/turn.rs`（删函数）、`ag/mod.rs`（doc 注释同步）、`db/mod.rs`（核对旧表列
是否仍被新链路使用；若无引用则在新 migration 中降级为兼容列或加清理 migration）。

**数据/接口变化**：`agent_turns` 表结构保持不变（三表模型是权威）；仅删除未使用函数。
状态机/Prompt 无变化。

**验证方式**：`grep persist_turn|recover_turns` 静态确认无调用方后删除；`cargo test`
全绿（删对应测试）；`governance:audit` 通过（无 orphan @GOV 引用）。

---

### FIX-D5 verify_anchor 消费路径显式化（对应 D5）

**真实根因**：BR-REF-VERIFY-001 的 `verify_anchor` 在 `reference/inject.rs` 实现，但 AG 消费路径
（`lib.rs format_reference_payload_context`）的 selection 校验逻辑是否显式调用该函数，审计时未逐行
确认——存在"规则实现与规则消费分离"的可见性缺口。

**为什么当前设计会产生**：REF 的锚校验属于"供给方规则"，AG 消费时按自己的读取层（AgentReadContext）
做了等价校验（selection 失效不作当前事实），两处校验各自独立演进，没有显式接线点。

**修复目标**：把 selection 引用消费前校验**显式接到 verify_anchor**（同一函数唯一入口），
消除"两套等价校验"。

**修复后预期系统状态**：`format_reference_payload_context` 对 `ReferencePayload::Selection`
先调 `verify_anchor`（Located 通过 / AnchorLost / Undetectable 保守拒绝），再读内容；
校验失败返回 `REF_ANCHOR_INVALID` 且该引用以"失效引用"占位呈现，不进入事实层。

**修改模块**：`reference/inject.rs`（暴露 verify_anchor 供 AG 调用，或新增组合函数）、
`lib.rs`（消费路径显式调用）。

**数据/接口变化**：无 wire 变化；内部调用关系显式化。状态机/Prompt 无变化。

**验证方式**：单测构造"发送时 block 已失效"的 selection 引用：断言消费后不产生引用内容注入、
返回 REF_ANCHOR_INVALID；既有"selection 失效不作当前事实"测试改为断言显式调用 verify_anchor
（测试可观测调用链）。

---

### FIX-D6 运行时参数集中化与 G5 标定（对应 D6）

**真实根因**：CTX_BUDGET=16000 / AUG_BUDGET=2500 / DOC_CTX_CAP=9000 / MENTION_CAP=4000 /
MAX_ROUNDS=6 / 450ms 防抖等参数以 `const` 分散在 lib.rs 与 assistStore.ts，无单一配置入口，
也无标定流程。

**为什么当前设计会产生**：这些数值被设计文档明确声明"延实现期/延 G5"，实现期先用 dev 常量
占位，属预期债务；但**散落位置**使 G5 标定时无法一次改齐。

**修复目标**：把全部运行时参数收敛到**单一配置结构** `AgentRuntimeConfig`（默认值集中，
允许后续按 profile 覆盖），并登记到 ADUS/测试注册表的参数清单。

**修复后预期系统状态**：lib.rs 与 assistStore.ts 不再出现魔法数字；G5 标定时只改
`AgentRuntimeConfig` 一处；测试可注入不同 config 验证截断/循环行为。

**修改模块**：新增 `ag/config.rs`（或并入 context.rs）；lib.rs、assistStore.ts 改读配置；
相关测试改注入 config。

**数据/接口/状态机/Prompt 变化**：无 wire 变化；常量位置集中化。状态机无变化。

**验证方式**：单测断言默认值与既有行为一致（回归）；构造 config{CTX_BUDGET=100} 断言历史层
截断行为随配置变化（证明"一处改齐"机制生效）；grep 断言 lib.rs/assistStore.ts 无遗留魔法数字。

---

## §2 Agent Architecture Target Model（目标架构模型）

> 以下按 Agent 能力层组织（非文件列表）。九层模型是 Binder AG 的目标架构；
> 当前实现已具备其中 7 层的骨架/实体，2 层（Planning、Evaluation）缺失。

```
┌─────────────────────────────────────────────────────────────┐
│  L9  Evaluation / Observability 层                          │
│  （turn 追踪、prompt 版本回放、失败分型、E2E 验证）            │  ← 缺失，需新建
├─────────────────────────────────────────────────────────────┤
│  L8  Session / Memory 层                                    │
│  （代理编辑会话、五态回合、懒加载、软归档、记忆写入触发）        │  ← 已实现，S 级
├─────────────────────────────────────────────────────────────┤
│  L7  Editor Integration 层                                  │
│  （DiffProjection、draft_mode、ED 草稿链、消息片段时序）        │  ← 已实现，语义待闭合(D2)
├─────────────────────────────────────────────────────────────┤
│  L6  Execution 层                                           │
│  （SM-AG-01/02/03 状态机、AgTurnEvent、门禁、smListeners）     │  ← 已实现，A 级
├─────────────────────────────────────────────────────────────┤
│  L5  Tool Capability 层                                     │
│  （注册表、权限分级、manifest、执行器、防滥用）                │  ← 已实现，A 级
├─────────────────────────────────────────────────────────────┤
│  L4  Planning / Workflow 层                                 │
│  （任务状态层、多步骤编排、CBT 移交/回落）                     │  ← 缺失，Phase 3 建
├─────────────────────────────────────────────────────────────┤
│  L3  Prompt Runtime 层                                      │
│  （分层注册表、注入顺序、信任等级、版本管理、缓存、调试追踪）     │  ← 半实现，缺注册表/版本/追踪
├─────────────────────────────────────────────────────────────┤
│  L2  Context 层                                             │
│  （当前/历史/引用/知识/工作区五类上下文、截断阶梯、AgentRead）   │  ← 半实现，缺五类分层化
├─────────────────────────────────────────────────────────────┤
│  L1  Intent 层                                              │
│  （意图识别、目标解析、歧义检测、工具预选）                    │  ← 半实现（关键词启发式）
└─────────────────────────────────────────────────────────────┘
```

### 当前 Binder AG 位于哪个阶段

- **阶段判定**：处于 **L1–L3 骨架向 L6–L8 实体化的过渡期**——执行层（L6）、会话层（L8）、
  编辑器集成层（L7）已真实落地；意图层（L1）、上下文层（L2）、Prompt Runtime（L3）是
  "机制存在、结构化不足"；规划层（L4）与评估层（L9）缺失。

| 能力层 | 已有能力 | 缺失能力 |
|---|---|---|
| L1 Intent | 关键词意图沿用（carryover 三态）、当前轮次原则 | 结构化 IntentResult（目标/操作/约束/置信度/歧义）、意图→工具预选 |
| L2 Context | AgentReadContext、历史 relPath 分流、P0 永不截、显式引用优先 | 五类上下文显式分层、信任等级标注、任务状态层/工作区依赖层 |
| L3 Prompt Runtime | 四类提示词、组装顺序、无图表声明、manifest 投影 | 层注册表（设计↔代码映射）、版本管理、prompt cache、调试追踪 |
| L4 Planning | 无（仅 SM-AG-01 takeover/fallbackIn 边界） | 任务状态层、多步骤回合编排、CBT 移交协议 |
| L5 Tool | 12 工具、3 级权限、manifest 单一来源、BDocx 受控链 | 意图驱动的工具选择（required_tools）、防滥用度量 |
| L6 Execution | 三机状态机、AgTurnEvent、门禁无绕过、工具循环 | turn 级可观测埋点（评估层前置） |
| L7 Editor | DiffProjection、draft_mode、ED 草稿链、segments 时序 | open_session 三段式语义（D2）、身份守卫与基线一致性审计 |
| L8 Session/Memory | 三表、五态、懒加载、软归档、删除保留 diff | 记忆自动沉淀的判定机制（模型/规则） |
| L9 Evaluation | failure_reason 穿透、状态广播 | turn 追踪回放、prompt 版本 A/B、E2E 验证矩阵 |

**短期应该补什么**（Phase 0–2）：契约修复（D1/D2/D5/D4）→ L6 埋点与 L9 基础（turn 追踪）→
L1 IntentResult v1（规则引擎）→ L3 层注册表与版本管理 → L2 五类上下文分层。

**长期应该演进什么**（Phase 3–4 及以后）：L4 Planning/Workflow（任务状态层、CBT 接入）→
L2 任务状态/工作区依赖层接入 → L5 意图驱动工具选择成熟 → L9 评测驱动 prompt 迭代 →
记忆自动沉淀模型判定。

---

## §3 逐模块方案

### Intent Layer（L1）

## 当前状态
- `context.rs::decide_intent_carryover`：关键词启发式，输出三态（Refine/Select/ReferenceTarget）+ None，
  决策"跨文档历史是否弱注入"与"意图提示行"。
- `lib.rs::intent_hint_for_prompt`：把判定结果转为一行【本轮意图】提示注入 Prompt。
- 无结构化输出；无目标/操作/约束建模；无歧义检测；无工具预选。

## 当前问题
- 意图信息在内部被"压扁"为提示行，Prompt 组装、工具权限、上下文选择无法按意图结构决策
  （只能靠模型自行理解提示行）。
- 关键词启发式对"任务延续"维度无建模（编辑模式无任务概念，可接受）；阈值不可调、不可观测。
- 意图判定结果未进入工具选择（模型看到的工具清单不随意图变化）。

## 修复方案

**架构变化**：引入 `IntentAnalyzer` 组件（v1 为规则引擎，接口与模型分类器同构，可替换）：
输入 = 本轮 prompt + 会话历史摘要 + 当前文档元数据；输出 = 结构化 `IntentResult`
（字段见 §4-B）。IntentResult 成为 L2/L3/L5 的**决策输入**，取代"提示行+模型自行理解"。

**数据变化**：新增 `IntentResult` 结构（intent_type/target/operation/constraints/confidence/
ambiguity/required_tools/required_context，§4-B 全字段）；`Carryover` 枚举保留为
IntentResult 的 `intent_type` 子字段之一（回指类），不再独立决策。

**代码变化**：新增 `ag/intent.rs`（analyze 函数 + 规则集 + 输出序列化）；
`context.rs` 的 decide_intent_carryover 收敛为 intent.rs 的一个规则子集（回指判定）；
`lib.rs` 组装入口改调 `intent::analyze` 并把 IntentResult 传给 L2/L3/L5。

**接口变化**：无 wire 契约变化（纯后端内部组件）；新增 Rust 内部 trait
`trait IntentAnalyzer { fn analyze(&self, ctx) -> IntentResult }`，规则实现与模型实现
（Phase 3+）互换。

**Prompt 变化**：意图提示行改由 IntentResult 生成（保留【本轮意图】行 + 新增歧义声明行
"存在歧义：…，请先澄清或按最可能目标继续"）；`required_context` 决定哪些层注入
（§4-D）。

**状态机变化**：无新状态机；SM-AG-01 的 Assembling 段新增"意图分析"动作（与上下文组装并行）。

## 修复后的工作流程
```
用户输入（prompt + 当前文档 + 历史摘要）
↓
IntentAnalyzer 解析 → IntentResult（类型/目标/操作/约束/置信/歧义/需工具/需上下文）
↓
Agent 决策：歧义>阈值 → 追加澄清请求；无歧义 → 锁定 target/operation
↓
上下文选择（按 required_context 分层注入） + 工具预选（按 required_tools 收敛清单）
↓
工具调用/编辑产出（按 constraints.diff_required 强制 DiffProposed）
↓
结果反馈（IntentResult 附入 turn 追踪记录，供 L9 回放）
```

## 验证标准
- 规则引擎黄金测试：构造 40 条典型 prompt（回指/选择/修正/新建/查询/整理/歧义/泛编辑词），
  断言 IntentResult 各字段与人工标注一致（≥95% 匹配，具体样本集入库）。
- 替换测试：以假模型 IntentAnalyzer（返回预设结果）注入，断言 L2/L3/L5 消费路径行为一致
  （证明接口可替换，模型升级不破坏下游）。
- 歧义测试：prompt 含双目标（"改这个也改那个"）→ ambiguity 非空且 Prompt 出现澄清声明行。

---

### Context Layer（L2）

## 当前状态
- `AgentReadContext`：已打开文档 LogicalState 优先、未打开才 DiskState；active live 序列化失败阻断。
- `scope_history_for_current_doc`：同文档/跨文档/无归属三态分流，跨文档仅在显式沿用弱注入。
- `assemble_context`：4 源（ExplicitMention/CurrentDoc/Memory/Knowledge）P0/P1 贪心保留。
- `collectOpenDocSnapshots`（前端）：binding 对齐 + LogicalState 等待 + active_editor_session 降级。

## 当前问题
- 五类上下文（当前/历史/引用/知识/工作区）未显式分层：知识（Memory/Knowledge）与历史同层混排，
  TPL 与工作区依赖层无位。
- 每类上下文无**信任等级**标注（哪类可被覆盖、哪类可作事实、哪类仅补强）。
- 裁剪阶梯由 4 源枚举驱动（D3），与设计 P0–P3 无映射表。

## 修复方案

**架构变化**：Context 层按"五类 + 信任等级"重组（§4-D 详述）：
当前文档(T1 主事实) / 用户显式引用(T2) / 历史(T3 背景) / 知识补强(T4) / 工作区依赖(T5)。
每类上下文经统一 `ContextAssembler` 装配，按 `TRUNCATION_STEP` 表截断（FIX-D3 注册表）。

**数据变化**：`ContextItem` 增加 `trust_tier` 字段；新增 `ContextCategory`
（CurrentDoc/ExplicitReference/History/Augmentation/WorkspaceDeps）；`ContextSource` 扩为 6 源（D3）。

**代码变化**：`context.rs` 改为注册表驱动（D3）；`lib.rs` 的 aug/mention/doc_ctx 三段手写拼接
收敛为 `ContextAssembler::assemble(ctx_inputs) -> AssembledPromptContext`；前端 collectOpenDocSnapshots
不变（L2 后端重组，采集协议不动）。

**接口变化**：无 wire 变化。**Prompt 变化**：注入格式统一带信任等级声明
（"以下为 T4 补强信息，仅作背景"）。**状态机变化**：无。

## 修复后的工作流程
```
用户输入
↓
系统处理：采集 open_doc_snapshots（L7） + 历史分流 + 引用校验（FIX-D5） + 知识检索门控
↓
Agent 决策：按 IntentResult.required_context 选择进入模型的上下文类别
↓
ContextAssembler 按注册表装配 + 按截断阶梯裁剪（P0 永不截）
↓
注入 Prompt Runtime（每层带信任等级声明）
↓
结果反馈：裁剪记录（trimmed）进入 turn 追踪（L9）
```

## 验证标准
- 五类分层测试：构造含五类输入的回合，断言装配产物各层出现顺序与信任等级声明正确。
- 截断黄金测试：P2 先裁、P1 尽量留、P0 永不截（注册表驱动，可复现）。
- 门控测试：编辑意图 + 显式引用充分时，知识检索不触发（reason 可观测，已有，扩展断言）。
- 禁令测试：已打开文档的 DiskState 读取在任何上下文路径返回拒绝（回归现有断言）。

---

### Prompt Runtime Layer（L3）

## 当前状态
- `prompts.rs`：四类 SYSTEM_PROMPT + bdocx_capability_prompt（manifest 投影）。
- `assemble_agent_turn_messages`：手写顺序 系统→补强→同文档历史→跨文档弱→引用→当前文档→意图→输入。
- 无层注册表、无版本管理、无 prompt cache、无调试追踪。

## 当前问题
- 层与注入顺序是函数内手写常量，设计↔实现映射靠人脑（D3）。
- Prompt 文本变更无版本记录、无回放能力（改坏了无法对照）。
- 无 prompt cache：稳定区（系统/工具/格式层）每轮重复序列化（M-T-02 §3 设计未落地）。
- 无调试追踪：无法回答"这一轮模型到底看到了什么"。

## 修复方案

**架构变化**：引入 `PromptRuntime` 组件（§4-C 详述）：
- `PromptManifest`：版本化（{version, layers[], trust_tiers[], budget{}, prompt_text_hashes[]}），
  与 BDocxCapabilityManifest 同模式（version/sha256，单一来源）。
- 层注册表：设计层 ↔ 代码层 ↔ 截断位 ↔ 信任等级（FIX-D3 的注册表即本层核心）。
- 稳定区缓存：系统/工具/格式层静态文本按 manifest version 缓存，会话内复用；
  动态层（上下文）不缓存。
- 调试追踪：每次装配产出 `AssembledPromptTrace {manifest_version, layers[{name,tier,bytes,trimmed}],
  intent_result, 裁剪记录}`，随 turn 记录（L9）。

**数据变化**：新增 `PromptManifest`（Rust struct + JSON 序列化）；`AssembledPromptTrace` 结构。
**代码变化**：新增 `ag/prompt_runtime.rs`（组装/缓存/追踪）；`assemble_agent_turn_messages`
迁移为该组件的实现；prompts.rs 文本改为 manifest 承载（文本常量仍可留，但 hash 进 manifest）。
**接口变化**：无 wire 变化；新增内部 trace 输出通道（结构化日志）。**Prompt 变化**：
文本仍属运营配置，但每次变更必须 bump manifest version 且更新 hash 断言测试。
**状态机变化**：无。

## 修复后的工作流程
```
用户输入
↓
系统处理：PromptRuntime 按 manifest 取稳定区缓存 + 装配动态层
↓
Agent 决策：意图/上下文/工具清单 → 层集合确定
↓
（无工具调用概念在此层；本层产出 messages 数组）
↓
messages 注入模型调用（L6） + AssembledPromptTrace 落 L9
↓
结果反馈：失败时按 trace 回放"模型看到了什么"
```

## 验证标准
- 版本测试：manifest version bump 后缓存失效重建；同 version 重复装配产物 hash 一致。
- 追踪测试：构造一次回合，断言 trace 含各层字节数与裁剪记录（trimmed 非空时）。
- 缓存测试：同会话连续两轮，稳定区缓存命中（不重复序列化系统层，可观测）。
- 回归测试：manifest 化后装配产物与当前实现逐字节一致（黄金快照，防迁移回归）。

---

### Planning / Workflow Layer（L4）

## 当前状态
- 缺失。SM-AG-01 有 takeover/fallbackIn 转移边界（CBT 移交/回落），无任务状态层实现。
- 回合是"单次请求→单次产出"（内部 ≤6 轮工具循环），无跨轮任务上下文。

## 当前问题
- 无任务状态层：无法表达"当前任务是什么、做到哪一步、产出哪些 artifacts"（M-T-02 设计缺失）。
- 多步骤意图（"先调研再改"）只能靠模型在同轮工具循环内自组织，超循环上限即中断。

## 修复方案

**架构变化**：新增 `TaskState` 运行时（轻量，Phase 3 建）：
- `agent_tasks` 结构：{task_id, session_id, intent_chain[], current_step, artifacts[], status}，
  仅存在于活跃回合上下文，随会话持久化。
- 任务状态层注入 Prompt（M-T-02 P1 层落地）：仅 active 任务注入，任务完成即清除。
- 编排：当 IntentResult 检测多步骤意图（如 required_tools 跨读/改/建 且 operation 序列化），
  Execution 层可把回合扩展为"步骤循环"（每步仍是 SM-AG-01 子回合，共享 task 上下文）。
- CBT 接入：task 上下文是构建移交（takeover）的载荷载体，与 SM-CBT-01 对接
  （Phase 4，本层先行定义数据结构）。

**数据变化**：新增 `agent_tasks` 表（或 agent_turn_payloads 内嵌 task 段，additive）；
`AgentTurnPayload` 增可选 `task` 字段（additive，兼容旧记录）。
**代码变化**：新增 `ag/task.rs`；lib.rs 回合入口读写 task；`assemble_agent_turn_messages`
注入任务状态层（按 task 存在性）。
**接口变化**：I-P-01 增可选 `task_id` 参数（start_turn）；`AgentTurnPayload.task` additive 字段。
**Prompt 变化**：新增任务状态层注入格式 `[当前任务] 步骤 N/M：...`（P1 尽量留）。
**状态机变化**：SM-AG-01 新增 `TaskAdvancing` 状态（步骤间转移，可中断），或复用
Streaming→HandingOff→（任务未完）→Assembling 回路（不新增终态，用 context.task_step 表达）。

## 修复后的工作流程
```
用户输入（多步骤意图：调研+修改）
↓
IntentAnalyzer → intent_type=multi_step, operations=[research, edit]
↓
Execution 层创建 task（step1=research）
↓
工具调用（read/search/list）→ 结果反馈 → task.artifacts 累积
↓
步骤循环：task.step2=edit → propose_edit → DiffProposed
↓
任务完成 → task 清除，回合 Completed（终态不变）
```

## 验证标准
- 任务状态测试：多步骤回合中 task.current_step 依序推进；单步骤回合不创建 task（零开销）。
- 持久化测试：回合中途退出，task 段随 payload 持久化、恢复后当前步骤可续（draft 重试语义）。
- 注入测试：有 task 时 Prompt 含任务状态层；task 完成后注入消失。
- 中断测试：task 中 cancel → 终态 Cancelled，task 保留为"未完成"（可重试续跑）。

---

### Tool Capability Layer（L5）

## 当前状态
- `tool_registry.rs`：12 工具 + 3 级权限（ReadOnly/StructureWrite/WriteIntent）。
- `bdocx_capabilities.rs`：manifest 单一来源 → schema/提示词/校验三者同源。
- `tool_exec.rs`：只读/结构执行；`block_ops.rs`：结构化批校验。
- 防滥用机制：写盘禁令、目标集守卫、循环上限 6、删除工具未注册、edit_blocked 诚实结束。

## 当前问题
- 模型看到的工具清单**不随意图变化**（每轮 12 工具全量投影），意图驱动的工具预选未实现。
- 防滥用靠"上限+禁令"，无**用量度量**（如单轮读放大、propose_edit 失败率），无法量化防滥用效果。
- 删除类工具的人类确认回路未落地（AG-02 债务，1.0 EP 前必须闭环）。

## 修复方案

**架构变化**：工具暴露面改为**意图驱动 + 双层清单**：
- 全量注册表（不变，权威源）。
- 每轮暴露面 = `intent.required_tools ∪ 安全默认集`（读工具），由 IntentResult 计算
  （§4-E）；工具 schema 投影只含暴露面，降低模型误选与 token 占用。
- 新增 `ToolUsageMetrics`（每轮：调用次数/读放大字节/失败/拒绝原因），进 L9。

**数据变化**：新增 `ToolExposure` 结构（{registered_all, exposed_this_turn, reason}）。
**代码变化**：lib.rs 工具循环改按 exposure 投影 schema；`tool_registry.rs` 增
`exposure_for_intent(intent) -> Vec<ToolDef>`。
**接口变化**：无 wire 变化（schema 是提示词层动态投影，不是 IPC 契约）。
**Prompt 变化**：SYSTEM_PROMPT_CHAT 的工具能力清单改为"按轮暴露面"（保留稳定边界声明，
具体清单由 runtime 注入）。**状态机变化**：无（SM-AG-02 不变）。

## 修复后的工作流程
```
用户输入
↓
IntentAnalyzer → required_tools=[read_doc, propose_edit]
↓
Tool Capability Layer 计算暴露面（required ∪ 安全默认）
↓
schema 投影仅含暴露面工具
↓
模型调用 → 工具循环（校验/执行/回注）→ 结果反馈
↓
ToolUsageMetrics 落 L9（读放大/拒绝原因/失败率）
```

## 验证标准
- 暴露面测试：编辑意图回合的 schema 不含 create_folder（非 required 且非默认）；
  组织意图回合的 schema 含 move_doc/create_folder。
- 防滥用度量测试：构造超量读请求，断言 metrics 记录读放大；单轮读字节超阈值时警告可观测。
- 删除确认测试（AG-02 闭环后）：删除工具注册后，无用户确认事件时执行返回拒绝
  （模型参数不得伪造确认，既有禁令扩展为可测断言）。

---

### Execution Layer（L6）

## 当前状态
- SM-AG-01/02/03 三机完整，门禁无绕过（结构性 + ensure_gate_passed 双把守）。
- AgTurnEvent 六分支 + smListeners 状态广播。
- 工具循环 ≤6 轮；assist 独立命令。

## 当前问题
- 无 turn 级可观测埋点（状态转移、各段耗时、失败分型不集中）。
- 工具循环与回合主循环耦合在 lib.rs 一个大函数，可测试性/可组合性受限。

## 修复方案

**架构变化**：Execution 层拆分三个内部组件（Rust 模块内重组，wire 契约不变）：
- `turn_orchestrator`：回合生命周期编排（原 lib.rs ag_start_turn 主体迁入，收口状态机调用）。
- `tool_loop`：工具循环执行器（校验→执行→回注→续调，接收 exposure/metrics）。
- `assist_orchestrator`：辅助命令封装。
- 每个组件产出**结构化事件**（现有 AgTurnEvent + 新增 `TurnTrace` 内部记录：时间戳/状态转移/
  门禁结果/循环轮次/各段字节数），不改变对外 Channel 语义。

**数据变化**：新增 `TurnTrace`（L9 消费）。**代码变化**：lib.rs 瘦身，逻辑迁入
`ag/execution.rs`（或三个子模块）。**接口变化**：无 wire 变化。
**Prompt/状态机变化**：状态机不变；状态转移现在显式写入 TurnTrace。

## 修复后的工作流程
```
用户输入
↓
turn_orchestrator 接收（SM-AG-01: Idle→Assembling）
↓
门禁 → Streaming → 工具循环（tool_loop，SM-AG-02）
↓
工具调用/结果回注（AgTurnEvent 原样发射 + TurnTrace 落点）
↓
产出处置（HandingOff→Completed / 失败→TurnError）
↓
TurnTrace 落 L9（状态转移序列 + 各段耗时 + 失败分型）
```

## 验证标准
- 编排测试：构造回合序列，断言 SM-AG-01 状态转移顺序与 TurnTrace 记录一致（回放可复现）。
- 组合测试：tool_loop 独立单测（不依赖 lib.rs），断言循环上限/拒绝回注/取消语义（已有 SM-AG-02
  单测迁移/复用）。
- 无回归：对外 Channel 事件序列与迁移前一致（黄金序列测试，防重构破坏前端契约）。

---

### Editor Integration Layer（L7）

## 当前状态
- DiffProjection + draft_mode 三态 + ED 草稿链 + segments 时序 + open_session 三段式（D2 现状）。
- 已打开文档采集（LogicalState 优先）完整。

## 当前问题
- open_session 确认语义未契约化（D2）。
- 身份守卫（整篇替换）与基线一致性仅在后端单点校验，无前-后端联合断言。

## 修复方案

**架构变化**：open_session 三段式语义落地为**显式状态位**（D2 的
`appliedButUnconfirmed`）；新增前后端联合的 `EditorBaselineCheck`（发送 diff 前由前端把
`base_session_version/base_content_hash` 与当前 session 比对，不等则 DiffProposed 转
`baseline_drift` 错误卡，不进入草稿链）。

**数据变化**：`DiffView` 增 `appliedButUnconfirmed`；`DiffProposed` 事件参数可选增
`baseline_ok: bool`（或由前端比对，不扩展 wire 契约——推荐前端比对，wire 冻结不动）。
**代码变化**：agService diff_proposed 处理增加基线比对；deStore 状态位。
**接口变化**：无 wire 变化（推荐方案）；I-P-01 文本回写三段式语义。
**Prompt/状态机变化**：无。

## 修复后的工作流程
```
用户输入（编辑意图）
↓
系统处理：前端采集 OpenDocSnapshot（含基线）→ 后端 propose_edit
↓
Agent 决策：生成 DiffProjection（draft_mode 判定）
↓
前端基线比对 → 一致 → DiffCard 进入对应草稿链；不一致 → baseline_drift 错误卡
↓
open_session：自动 apply（可回滚）→ 用户确认 → accept_draft_diff → 落盘
↓
结果反馈：卡片状态机（pending→accepted/rejected/expired）完整
```

## 验证标准
- 基线漂移测试：发送 diff 后 session 版本被外部推进 → DiffProposed 呈现 baseline_drift，
  不进入草稿链、不误改。
- 三段式测试（D2）：apply 后内容已变但未 accept（磁盘不变）；reject 回滚 base；
  accept 后落盘。
- 时序测试（回归）：segments 交错渲染在 draft_mode 三态下均正确（已有 AgentUI.test 扩展）。

---

### Session / Memory Layer（L8）

## 当前状态
- 三表（agent_sessions/agent_turns/agent_turn_payloads）、五态、懒加载、200MB 软归档、
  删除保留 DE diff、turnSaveChain 串行化。
- memory_link.rs：终态异步触发 ast.capture（带 origin_turn，失败隔离）。

## 当前问题
- 记忆"可沉淀事实判定"机制未实现（on_turn_terminal 的 candidate 由谁产生、按什么规则判定，
  实现期遗留——当前无 candidate 来源，等于未触发）。
- 软归档仅"标记 archived"，无后台压缩（A 类债务，可后置）。

## 修复方案

**架构变化**：记忆判定先落**规则 v1**（不引入模型开销）：
- 判定规则：回合 Completed 且 assistant 消息含"用户明确陈述的事实/偏好"句式
  （如"我用…""我喜欢…"白名单句式 + 排除命令/修改类），产出一组 candidate 交 ast.capture。
- 规则产出的 candidate 全部带 `origin_turn`，经现有 memory_link 通道落库（机制复用）。
- v2（Phase 3+）：评估层积累正样本后，换模型判定（同一 capture 接口）。

**数据变化**：无（memory_entries 已有 origin 字段）。**代码变化**：新增
`ag/memory_extract.rs`（规则判定）；lib.rs 回合终态调 memory_link 前先过判定。
**接口变化**：无 wire 变化。**Prompt/状态机变化**：无（异步不阻塞回合）。

## 修复后的工作流程
```
用户输入 → 回合执行 → Completed 终态
↓
memory_extract 规则判定（白名单句式 + 排除修改类）
↓
产出候选（带 origin_turn）→ memory_link::try_capture → ast.capture 落库
↓
失败隔离（不阻塞回合终态，回归现有语义）
```

## 验证标准
- 判定测试："我常用 Rust" → 产出 fact 候选；"把这段改短" → 不产出（命令类排除）。
- 链路测试：终态后异步触发（非阻塞断言，turn 已 Completed）；ast 拒绝无类型候选时不传播错误。
- 回归：既有 capture 测试全绿。

---

### Evaluation / Observability Layer（L9）

## 当前状态
- failure_reason 分型（assist）、状态广播（smListeners）、console 日志。
- 无集中 turn 追踪、无 prompt 版本回放、无 E2E 验证矩阵。

## 当前问题
- 无法回答"这一轮模型看到了什么、为什么失败"（除人工日志）。
- 无评测基线：prompt 文本/意图规则改动无回归依据。

## 修复方案

**架构变化**：新增 `TurnTracer`（Phase 1 建）：
- 每回合产出 `TurnTrace {turn_id, intent_result, manifest_version, layers[{name,tier,bytes}],
  trimmed[], tool_calls[{name,exposure,result,bytes}], state_transitions[], failure, 各段耗时}`，
  写入结构化日志文件（app data `trace/` 目录，可回放）。
- 新增 `E2E 验证矩阵`（`ag_eval/` 或测试注册表扩展）：黄金场景集
  （编辑/引用/历史沿用/歧义/门禁拒绝/assist 三候选/会话恢复），脚本化回放断言。
- prompt 版本对比：同场景集跑两版 manifest，对比 IntentResult/tool 序列/产出，产出对比报告。

**数据变化**：`TurnTrace` 结构化记录。**代码变化**：新增 `ag/trace.rs`；
lib.rs/execution 组件埋点。**接口变化**：无 wire 变化（trace 为本地诊断，不上行）。
**Prompt/状态机变化**：无。

## 修复后的工作流程
```
任意回合
↓
Execution 层每段动作写 TurnTrace（状态转移/意图/层字节/工具/失败）
↓
回合终态 → trace 落盘（app data，本地）
↓
失败场景：按 trace 回放"模型看到了什么"（Prompt 快照 + 层清单）
↓
评测：E2E 矩阵回归 → prompt 版本对比报告
```

## 验证标准
- 追踪完整性测试：任一回合终态必产出 TurnTrace 且含最小字段集（非空断言）。
- 回放测试：构造失败回合，断言可按 trace 还原 Prompt 各层内容（字节级一致性）。
- E2E 矩阵：golden 场景全部通过（编辑产出/引用注入/意图沿用/门禁拒绝/assist/会话恢复）；
  任一 prompt manifest 变更必须跑矩阵（作为 test:related 关联项）。

---

## §4 重点深度分析

### A. Agent 工作策略：当前模式与未来模式

**当前 Agent 是什么**：**Reactive Agent + 机制约束的混合模式**——模型对每轮输入直接反应，
工具循环（≤6 轮）提供有限自主性；稳定执行不依赖规划，而依赖五重外部机制
（门禁硬前置 / 工具注册权限上限 / 写盘禁令 / 编辑意图门 / 读前置门）。
它**不是** Workflow Agent（无脚本化步骤），也**不是** Planning Agent（无任务分解/反思）。

**未来 Binder 应该采用什么模式**：**分层混合模式**——Execution 层保持 Reactive（每轮原子响应），
Orchestration 层状态机驱动（现状），**Task 层按需启用 Planning**（Phase 3 起）：
- **编辑模式（默认）**：Intent-First Reactive——Reactive 不变，但每轮先过 Intent Layer
  输出结构化意图，用意图门控上下文选择/工具预选/产出约束。**不引入每轮规划**。
  理由：写作意图是原子的（一次请求一个 target/operation），规划开销（额外模型调用、延迟、
  成本）与收益不匹配；diff 裁决闸门已经提供了"计划确认"的人类在环替代品。
- **任务化场景（多步骤意图 / BuildMode / CBT）**：Planning Agent——Task 层把多步骤意图分解为
  步骤序列（每步仍是 SM-AG-01 原子回合，共享 task 上下文），步骤间允许检查点确认；
  BuildMode 的蓝图确认是计划审批的显式形态。
- 演进路径：Phase 1–2 夯实 Intent-First Reactive；Phase 3 上 Task 层（Planning-Lite，
  仅多步骤意图触发）；Phase 4 CBT 接入后 Planning 完整化。**不做**每轮深度反思/自评循环
  （成本与延迟在写作场景不可接受）。

### B. Intent 系统：完整 IntentResult 设计

```rust
pub struct IntentResult {
    pub intent_type: IntentType,        // Edit|Create|Organize|Query|Assist|Reference|Build|ContinueTask|None
    pub target: IntentTarget,           // CurrentDoc | Doc{rel_path} | Block{rel_path,block_id} |
                                        // Selection{rel_path,start,end} | Workspace | Session | None
    pub operation: Option<Operation>,   // Rewrite|Insert|Delete|Restyle|CreateFolder|MoveDoc|
                                        // Search|Summarize|CreateDoc|Refine|Select|...
    pub constraints: IntentConstraints, // { diff_required: bool, confirm_required: bool,
                                        //   target_scope: "declared_set"|"current_doc"|"workspace",
                                        //   no_direct_write: bool, read_first: bool }
    pub confidence: f32,                // 0.0–1.0
    pub ambiguity: Vec<IntentOption>,   // 候选意图列表（歧义时非空）
    pub required_tools: Vec<String>,    // 本意图最小工具集（供暴露面计算）
    pub required_context: Vec<ContextCategory>, // 本意图需要的上下文类别（供 L2 选择）
}
```

**IntentResult 如何影响四个下游**：

1. **Prompt 组装（L3）**：`intent_type` 选择系统提示词变体（Edit 用 CHAT，Assist 用对应
   assist_kind，Build 用 BUILD）；`required_context` 决定注入哪些层；`ambiguity` 非空时
   追加澄清声明行；`confidence < 阈值` 时追加"按最可能目标继续，可纠正"提示。
2. **工具权限（L5）**：`required_tools ∪ 安全默认集` 为暴露面；`constraints.no_direct_write`
   强化写工具仅 propose_edit；`target_scope` 决定目标集守卫强度（declared_set 严格、
   workspace 仅读工具）。**工具清单随意图收敛**，是防误选的第一道闸。
3. **Diff 生成（L7）**：`operation` 预选 patch_kind（Rewrite→replace_block/block_ops，
   Restyle→style_ops，CreateDoc→create_doc 或空基准 ChangeProposal，依 D1 裁决）；
   `constraints.diff_required` 强制回合结束前必须有 DiffProposed（现有
   AG_EDIT_TOOL_REQUIRED 门升级为意图驱动判定）；`confirm_required` 决定 draft_mode
   （current_editor/open_session/background 的预选）。
4. **上下文选择（L2）**：`required_context` 决定五类上下文哪些进入（编辑意图默认
   CurrentDoc+ExplicitReference+HistorySameDoc；Query 意图加 WorkspaceDeps；
   显式引用意图加 ExplicitReference 全量）；`constraints.read_first` 触发
   当前文档读前置门（现有 AG_CURRENT_DOC_READ_REQUIRED 由意图驱动）。

**实现路径**：v1 规则引擎（确定性、可测）→ Phase 3 起评测集驱动换模型分类器
（同一 `IntentAnalyzer` trait，见 L1 方案）。

### C. Prompt Runtime 架构

| 层 | 定义 | 优先级/截断 | 注入顺序 | 数据可信等级 | 缓存策略 |
|---|---|---|---|---|---|
| 治理/系统层 | 角色 + 硬约束（diff 裁决/不越权/workspace 限界/当前轮次原则） | P0 永不截 | 1 | T0 不可覆盖 | 稳定区缓存（按 manifest version） |
| 工具/能力层 | 暴露面工具清单 + schema（manifest 投影） | P0 | 2 | T0 | 稳定区缓存（按暴露面 hash） |
| 格式层 | GFM 子集 / 无 Mermaid / BDocx 能力声明 | P0 | 3 | T0 | 稳定区缓存 |
| 任务状态层 | active 任务 + 步骤 + artifacts（Phase 3 建） | P1 尽量留 | 4 | T1 任务事实 | 动态 |
| 当前文档层 | LogicalState + 块目录 + 基线 | P0 | 5 | T1 当前轮主事实 | 动态 |
| 引用层 | 显式 @mention/引用载荷 | P1 最高保留 | 6 | T2 用户显式 | 动态 |
| 历史层 | 同文档背景 + 跨文档弱 | P1 可摘要化 | 7 | T3 背景 | 动态 |
| 补强层 | 记忆/知识/模板（带权重声明 + 门控） | P2 先截 | 8 | T4 仅补强 | 动态 |
| 工作区依赖层 | 文件依赖图/pending diff 状态 | P3 按需 | 9 | T5 参考 | 动态 |
| 当前输入层 | 用户消息 + 意图提示行 | P0 | 10 | T1 信号源 | 动态 |

- **信任等级语义**：T0 是规则约束（模型不可违背，提示词声明 + 执行层强制双保险）；
  T1 是当前轮事实（模型可引用不可质疑其为主事实）；T2 是用户显式授权；T3 是背景；
  T4 声明为"不覆盖 T1/T2/T3"；T5 是参考。信任等级决定"层是否可被截、是否带声明、是否可被
  其它内容覆盖"。
- **版本管理**：`PromptManifest`（version/sha256/层清单/预算表）；任何 Prompt 文本变更必须
  bump 版本并更新静态断言测试；trace 记录 manifest_version 使"改了什么"可回放。
- **缓存策略**：稳定区（T0 三层）按 manifest version 会话内缓存；工具层按暴露面 hash 缓存；
  动态层永不缓存。
- **调试追踪**：每轮 `AssembledPromptTrace`（manifest 版本、各层字节、信任等级、裁剪记录、
  意图结果、工具暴露面）落 L9；失败按 trace 回放，替代人工拼日志。

### D. Context 系统：五类上下文分层与"进/禁"规则

| 上下文类别 | 来源 | 信任等级 | 进入条件 | 禁止进入 |
|---|---|---|---|---|
| 当前文档 | OpenDocSnapshot LogicalState（active/open session） | T1 主事实 | 存在当前文档 | DiskState 或 loadedContent 旧快照（X-FORBID-006 读门）；缺结构化快照的富文档 |
| 历史 | 同会话回合（relPath 归属） | T3 背景 | 同文档历史默认进；跨文档仅显式回指弱注入 | 无归属/跨文档历史（未显式回指）；draft/error 半成品不得作为事实 |
| 引用 | ReferencePayload（doc/selection/session/memory/knowledge/template/url/long_text） | T2 显式 | 发送时锚校验通过（FIX-D5） | 锚失效引用；url 抓取内容（未抓取，仅 URL 本身）；越出 workspace |
| 知识 | AST 记忆 / KB 检索（门控触发） | T4 补强 | 门控通过（编辑上下文/显式引用充分时不触发） | 覆盖当前文档/引用/当前轮产出（带权重声明）；检索失败不降级为噪声 |
| 工作区 | 文件列表/pending diff/依赖图 | T5 参考 | 查询/整理意图按需 | 无意图需求时不注入；其它 workspace 数据 |

- **什么进入模型**：仅满足"信任等级 ≥ 注入门槛 且 通过类别门控 且 未在预算内被裁"的内容。
  注入门槛由意图决定（编辑意图：T1/T2/T3；查询意图：+T5）。
- **什么禁止进入模型**：越 workspace 数据、已打开文档的 DiskState、失效引用、
  未抓取的网页正文、二进制/原生 docx 字节、跨文档历史（未显式回指）、含指令语义的用户文档
  （提示注入防护：内容即数据非指令）、其他会话内容。

### E. Tool 系统：Tool Capability Layer 设计

| 环节 | 机制 | 防滥用点 |
|---|---|---|
| 注册 | 全量注册表（唯一权威源）+ manifest 单一来源（schema/提示词/校验同 version/hash） | 无法绕过注册表执行 |
| 权限 | 3 级（ReadOnly/StructureWrite/WriteIntent）+ 写盘禁令（X-FORBID-006） | 写意图只能 propose_edit；删除类工具未确认回路不注册 |
| 选择 | 暴露面 = `intent.required_tools ∪ 安全默认集`（§4-B） | 工具清单随意图收敛，模型误选面收窄 |
| 执行 | validate→exec→inject 三拍（SM-AG-02）；只读经 AgentReadContext 限界 | 未注册/越权拒绝回注；已打开文档禁止 DiskState |
| 反馈 | ToolResult 事件 + propose_edit 权威 projection；失败/拒绝原因回注模型 | 结果回注不中断循环（execFail 可恢复） |
| 错误恢复 | 循环上限 6（达限 LoopAborted）；edit_blocked 诚实结束；目标冲突拒绝不覆盖 | 无限循环/静默失败被结构性禁止 |
| 度量 | ToolUsageMetrics（调用/读放大/失败/拒绝原因）进 L9 | 用量可量化，异常趋势可预警 |

**避免 Agent 滥用工具的五道闸**：①暴露面收敛（意图驱动，模型看不到用不着的工具）；
②写盘唯一路径（propose_edit→DE，结构上无旁路）；③目标集守卫（未声明既有文档拒绝）；
④循环上限 + 读放大度量；⑤模型参数不得伪造用户确认（删除类工具确认回路为独立 UI 通道）。

---

## §5 阶段路线图（按架构依赖排序）

> 排序依据：**下游依赖上游**——契约不修，能力增强会建立在双真值上；执行层不埋点，
> 评估层无数据；意图不结构化，上下文/Prompt/工具的意图驱动无从谈起；任务层不建，
> CBT 接入没有载荷。

### Phase 0：修复契约断裂（先裁决，再实现）
- **目标**：消除 D1/D2 双真值、D4 双轨、D5 校验分离；参数集中（D6）。
- **修改范围**：FIX-D1（人类裁决 + X-FORBID-006/BR-AG-DATA-003 回写）、FIX-D2（I-P-01 三段式回写 +
  appliedButUnconfirmed）、FIX-D4（turn.rs 旧表退役）、FIX-D5（verify_anchor 显式接线）、
  FIX-D6（AgentRuntimeConfig 集中）。
- **完成标志**：规则文档无双重语义（grep 断言）；governance:audit 通过；三项裁决记录在案；
  契约测试（D1/D2 验证方式）全绿。

### Phase 1：建立 Agent Runtime 基础
- **目标**：Execution 层可观测、Session 层冻结、E2E 基线建立——为后续能力增强提供
  "可回放、可回归"的地基。
- **修改范围**：L6 Execution 拆分（turn_orchestrator/tool_loop/assist_orchestrator）+ TurnTrace
  埋点；L9 Evaluation 基础（trace 落盘 + E2E 黄金矩阵第一版）；L8 记忆规则 v1
  （memory_extract）；L5 ToolUsageMetrics。
- **完成标志**：任一回合终态必有可回放 TurnTrace；E2E 矩阵 8 类黄金场景全绿；
  记忆规则 v1 有判定测试；工具度量落 trace。

### Phase 2：增强 Intent/Context
- **目标**：意图从"提示行"升级为结构化决策输入；上下文五类分层；Prompt Runtime 注册表化。
- **修改范围**：L1 IntentAnalyzer v1（IntentResult 全字段 + 黄金测试）；L2 Context 五类分层
  （ContextCategory/trust_tier + TRUNCATION_STEP 注册表，D3 落地）；L3 PromptManifest +
  稳定区缓存 + AssembledPromptTrace；L5 暴露面按意图收敛。
- **完成标志**：IntentResult 黄金测试 ≥95% 匹配；五类分层/截断/门控测试全绿；
  prompt 版本 bump 有回归断言；暴露面测试（编辑/组织意图清单差异）全绿。

### Phase 3：增强 Workflow 能力
- **目标**：任务状态层落地，多步骤意图可编排；为 CBT 提供任务载荷。
- **修改范围**：L4 TaskState（agent_tasks 结构 + 任务状态层注入 + 步骤循环）；L3 任务状态层
  接入注册表；L1 意图模型升级（评测集驱动，可选）；L2 工作区依赖层接入（Phase 3 末）。
- **完成标志**：多步骤回合可跨轮续跑（task 持久化/中断/重试测试全绿）；单步骤回合零任务
  开销（回归）；任务状态层 Prompt 注入随 task 存在性正确出现/消失。

### Phase 4：BuildMode / CBT 接入
- **目标**：Planning 完整化，构建移交/回落落地（用户声明的后续版本能力）。
- **修改范围**：L4 task 载荷与 SM-CBT-01 对接（takeover 携蓝图、fallbackIn 起任务回合）；
  L3 增加 SYSTEM_PROMPT_BUILD 任务化变体；L2 任务上下文注入。
- **完成标志**：构建移交/回落双向回执测试全绿（无双活/无双死）；G5 跨模块联调通过
  （门禁→网关→流式→diff→落盘→构建）。

---

## §6 Binder AG v2：冻结边界与长期基础设施判定

### 6.1 必须冻结的能力（v2 红线，改动需双侧同步 + 人类批准）

| 冻结项 | 冻结原因 |
|---|---|
| AgTurnEvent 六分支（wire 契约） | 前后端双轨唯一同步锚点，变更破坏全部消费方 |
| DiffProjection 字段集 + draft_mode 三态 | AG→DE→ED/WS 唯一产出契约；语义变化必须三段式显式化（D2）后冻结 |
| OpenDocSnapshot 结构（含 block/table/image snapshots） | 已打开文档读取的唯一事实来源协议 |
| X-FORBID-006 唯一落盘路径（含 D1 裁决后的新建语义） | 安全模型根约束，客户端无旁路 |
| 门禁无绕过路径（X-SEQ-001） | 订阅/计费安全根约束 |
| 五态历史模型（completed/error/cancelled/gate_rejected/draft）与三表 schema | 重启恢复契约；payload 变更只允许 additive |
| TERM-AG-001~011 术语与禁用别名表 | 全仓库语言一致性 |
| BDocxCapabilityManifest 单一来源（schema/提示词/校验同源） | 防手写第二套能力清单 |

### 6.2 允许持续变化的能力

| 变化项 | 变化边界 |
|---|---|
| Prompt 文本/分层细节 | 经 PromptManifest 版本管理 + E2E 矩阵回归（L9） |
| 工具清单/参数 schema | 注册表内增删，权限分级不变；新写工具须过 manifest/暴露面机制 |
| 预算与机制参数 | AgentRuntimeConfig 一处修改，G5 标定 |
| 意图识别实现算法 | 规则引擎 → 模型分类器，同一 IntentAnalyzer 接口 |
| 浮层视觉/消息渲染样式 | 纯展示层，不改变语义契约 |

### 6.3 长期基础设施（优先投入、优先加固）

1. **Execution Layer（L6）**：三机状态机 + AgTurnEvent + TurnTrace——一切能力的执行地基。
2. **Session/Memory Layer（L8）**：三表 + 懒加载 + 软归档——用户数据与恢复契约。
3. **Tool Capability Layer（L5）**：注册表 + manifest + 执行器——能力扩展的安全闸。
4. **Editor Integration Layer（L7）**：DiffProjection + 草稿链——产品核心交互（对话→diff→裁决→落盘）。
5. **Evaluation/Observability（L9）**：turn 追踪 + E2E 矩阵——后续一切迭代的回归依据。

### 6.4 不应该过早建设的模块

| 模块 | 不建的原因 | 触发条件 |
|---|---|---|
| 完整 Planning Agent（每轮规划/反思） | 编辑意图原子化，规划开销与收益不匹配；diff 裁决已提供计划确认 | 多文档任务化成为常态（Phase 3 任务层数据证明需求） |
| 多 Agent / 子 Agent 编排 | 无真实场景支撑，复杂度和故障面急剧上升 | CBT 多文件构建需求溢出单 Agent 后 |
| 复杂 ModelRouter / 多模态矩阵 | 1.0 EP 已明确后置；DeepSeek 托管 + BYOK 已覆盖主路径 | 自部署预设模型成为发布阻断项 |
| 记忆自动沉淀的模型判定 | 无正样本积累，模型判定会污染记忆库 | L9 评测集积累后 |
| 补强层相关性打分（TopK） | KB 检索未成熟，打分形同虚设 | KB G5 就绪后 |
| Prompt 自动优化器 | 无评测基线前自动优化 = 无参照系 | E2E 矩阵 + 版本对比报告就绪后 |

---

## §7 一句话总结

**Binder AG 演进主线 = 契约先修（Phase 0）→ 运行时可观测（Phase 1）→ 意图/上下文结构化
（Phase 2）→ 任务编排（Phase 3）→ 构建接入（Phase 4）**；v2 的架构重心是
"把当前靠提示词和单轮循环的 Reactive Agent，演进为 Intent-First 的分层混合 Agent"，
其中 L1/L2/L3 是下一阶段的主战场，L6/L7/L8 是必须守住的基础设施，L4/L9 按依赖逐步建设。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-08-07 | v1.0 | 初版：AG Agent 能力修复与阶段化演进方案——FIX-D1~D6 契约修复机制、九层 Agent Architecture Target Model、逐模块方案（L1–L9）、五大深度分析（工作策略/IntentResult/Prompt Runtime/Context 分层/Tool 能力层）、Phase 0–4 架构依赖路线图、AG v2 冻结与演进边界 |
