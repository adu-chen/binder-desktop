---
文档编号：CORE-C-P-02
文档状态：A
负责模块：PCG
文档职责：产品能力图谱规范（PCG-SPEC）——定义产品能力注册表、能力变体、证据状态、主链唯一性与生成式产品能力沙盘
上游约束：A-CORE-C-D-01_产品概述.md、A-CORE-C-P-01_APC.md、ADU.md（RULE/CHAIN/TERM/@GOV）、ADP.md（Scope Lock 与验证）
直接承接：.governance/product-capability-registry.json、PCG-GENERATE、PCG-AUDIT、产品能力沙盘与能力索引
使用边界：本文定义 PCG 静态规则和数据契约；不复制模块需求正文、不替代 ADUS、不直接承接 Binder 运行时代码或产品 UI
变更要求：能力对象、状态语义、追溯关系、唯一性规则或生成物边界变更时，必须同步注册表 schema、脚本自测、验证卡并运行 governance:generate
治理成熟度：baseline
---

# 产品能力图谱规范（PCG-SPEC）

## §1 定位与对象边界

PCG-SPEC 是现有需求、技术设计、规则、测试和验证的产品视角投影规范。它回答“产品能做什么、适用于什么条件、证据在哪里、是否可承诺”，不重新定义现有 REQ、RULE、CHAIN、CONSTRAINT 或 TERM。

产品经理默认消费生成的“产品能力沙盘”；注册表与关系图仅在能力详情和治理审计中按需展开。

<!-- MODULE
module_code: PCG
module_name: 产品能力图谱
description: 将产品能力、格式/角色变体、验收点、治理引用和验证证据组织为单一注册表，并生成产品沙盘和审计结果
-->

## §2 Terminology Registry

<!-- TERM
term_id: TERM-PCG-001
zh: 产品能力项
en: ProductCapability
chains: PCG-GENERATE, PCG-AUDIT
forbidden: 功能点, feature点, 产品功能块
description: 用户可理解、可独立说明边界和结果的产品能力；只拥有一个主产品归属，不因跨模块协作重复登记
-->

<!-- TERM
term_id: TERM-PCG-002
zh: 能力变体
en: CapabilityVariant
chains: PCG-GENERATE, PCG-AUDIT
forbidden: 功能组合, 功能分支, capability组合
description: 产品能力在格式、操作者、操作或场景上形成的实际支持承诺；只物化已支持、已排除、计划或需澄清的组合
-->

<!-- TERM
term_id: TERM-PCG-003
zh: 产品能力沙盘
en: ProductCapabilityAtlas
chains: PCG-GENERATE
forbidden: 功能总表, 产品功能图, 特性大盘
description: 由产品能力注册表生成的交互式 HTML 视图，面向产品经理展示故事地图、格式矩阵、证据和缺口
-->

<!-- TERM
term_id: TERM-PCG-004
zh: 正式主链
en: CanonicalChain
chains: PCG-AUDIT
forbidden: 唯一路径, 固定链路, 主流程代码
description: 某个具外部副作用能力变体的唯一正式业务链路；明确登记的 fallback 不构成第二正式主链
-->

## §3 数据模型

注册表由以下对象组成：

| 对象 | 责任 | 关键关系 |
|---|---|---|
| ProductArea | 产品能力的唯一主归属 | 包含 Journey 与 ProductCapability |
| Journey | 用户完成任务的叙事阶段 | 组合多个 ProductCapability |
| ProductCapability | 用户可理解的能力 | 拥有多个 CapabilityVariant |
| CapabilityVariant | 具体格式/角色/操作承诺 | 拥有 Acceptance 与 Evidence |
| Acceptance | 可观察结果 | 由测试或验证证据证明 |
| Evidence | 测试、验证、规则、链路、代码的引用 | 不复制其权威正文 |

每个 `ProductCapability` 只有一个 `product_area_id`；跨产品关系使用 `related_capability_ids`，不得复制节点。每个 `CapabilityVariant` 只在真实承诺、排除或缺口存在时创建，禁止生成完整笛卡尔积。

## §4 状态与支持边界

能力变体必须分别维护：

- `definition`：`draft | confirmed | changed_needs_review`
- `delivery`：`not_started | in_progress | implemented`
- `automation`：`none | partial | passed | failed`
- `system_validation`：`not_required | pending | passed | failed | blocked`
- `release`：`planned | internal | available | not_ready | unsupported`

格式支持只使用：`supported | partial | unsupported | unknown | not_applicable`。`unsupported` 表示明确不做；`unknown` 表示尚未定义或尚未盘点，二者不得混用。

<!-- CHAIN
chain_id: PCG-GENERATE
主责模块: PCG
status: active
risk_tier: P1
integration_test: scripts/test_governance.test.mjs::生成可点击的产品故事地图和格式矩阵
-->

<!-- RULE
rule_id: BR-PCG-DATA-001
主链路: PCG-GENERATE
域: DATA
-->
产品能力注册表中 ProductArea、Journey、ProductCapability、CapabilityVariant、Acceptance 和 Evidence 的标识符必须唯一；所有引用必须指向已登记对象或既有权威 ID。产品能力只可有一个主产品归属，跨域关联只能以关系引用表达，不得复制能力节点。

<!-- RULE
rule_id: BR-PCG-STATE-001
主链路: PCG-GENERATE
域: STATE
-->
产品定义、实现交付、自动化证据、系统验证和发布承诺必须独立表达。任何自动化测试通过、代码存在或 HTML 生成成功均不得自动把系统验证或发布承诺提升为通过或可用。

<!-- CHAIN
chain_id: PCG-AUDIT
主责模块: PCG
status: active
risk_tier: P1
integration_test: scripts/test_governance.test.mjs::接受具有完整追溯、主链和独立状态的注册表
-->

<!-- RULE
rule_id: BR-PCG-VERIFY-001
主链路: PCG-AUDIT
域: VERIFY
-->
每个已确认的能力变体必须至少追溯到需求来源或明确内部来源；P0 变体必须追溯到规则、链路、自动化证据和系统验证状态。上游 requirement、rule、chain 或 term 发生版本变化时，关联证据必须转为待复核，不得沿用旧绿色状态。

<!-- RULE
rule_id: BR-PCG-VERIFY-002
主链路: PCG-AUDIT
域: VERIFY
-->
具有外部副作用的能力变体必须声明且只能声明一条正式主链。替代、兼容或恢复路径必须显式包含 fallback_of 与触发条件；未经登记的第二主链、第二写入责任者或相同业务结果的并行实现必须报告为唯一性冲突或 duplicate_candidate。

## §5 主链唯一性

`side_effect=true` 的能力变体必须声明 `canonical_chain`。同一变体的 `fallbacks` 只能引用该主链，并声明 `when`。审计器检查：重复变体 ID、重复主链声明、缺失主链、无效 fallback、未知引用和疑似重复实现候选。

唯一性约束针对业务责任和写入路径，不要求所有跨模块协作只经过一个函数；合法的模块协作、容错和降级必须可解释、可追溯。

## §6 生成物与治理联动

注册表是唯一人工编辑机器源；以下产物由脚本生成且禁止人工编辑：

- `product-capability-atlas.html`：产品故事地图、详情、格式矩阵和缺口视图；
- `product-capability-coverage.json`：机器可读汇总；
- `product-capability-chain-audit.md`：唯一性与追溯审计报告；
- `feature-status-tree.html`：功能状态树（人工审计与跟进视图）——L1–L4 层级 + L5 原子行为叶子，三态审计状态、链路/规则映射标签；数据源为 `function-ledger.json` 的 `feature_audit`（**L5 级**，key = 台账行 id），链路/规则引用经 ADUS 注册表只读校验，不进入 ADUS 索引、不参与 governance:generate。三态为**实现健康度**口径：设计=未实现；未闭合=已实现但半实现/有缺陷/有残留；已闭合=已实现且正常完整（默认按台账"已实现"状态，最终人工审计兜底）。L4 分支状态按最差进展聚合。`evidence` 为判定留痕（图谱自动化 / 测试通过 / 验证卡 / 人工确认），缺失时生成器仅告警不阻断；
- ADUS 的产品能力索引。

PCG 通过 REQ/RULE/CHAIN/TERM/测试/验证 ID 连接现有治理体系。代码仍以 RULE/CHAIN 的 `@GOV` 标注为准，不要求每段代码直接声明 ProductCapability。

## §7 验收与非目标

首期只覆盖文档协作域：Markdown、HTML、纯文本、原生 DOCX、BDocx 及代理编辑。沙盘不进入 Binder 桌面 UI，不访问 workspace、数据库、Tauri 或云端，也不替代真实系统验证。

## §8 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-02 | v1.0 | PCG-SPEC 首版：建立能力/变体模型、独立状态维度、主链唯一性、生成产物边界与 PCG-GENERATE/PCG-AUDIT 静态规则。 |
| 2026-08-25 | v1.1 | 生成物边界变更：PCG-GENERATE 产物清单新增 `feature-status-tree.html`（功能状态树，人工审计人类友好视图）。数据源为 `function-ledger.json` 的 `feature_audit`（scripts/seed_function_audit.mjs 一次性种子化），生成器 scripts/generate_function_tree.mjs（npm `product:tree`）。该产物仅只读校验 ADUS 链路/规则注册表，不写入 ADUS、不参与 governance:generate，因此本变更未运行 governance:generate（经人工确认，保持 ADUS 逻辑零改动）。 |
| 2026-08-25 | v1.2 | 三态语义修正为「实现健康度」口径：verified=已实现且正常完整（默认按台账"已实现"状态，人工审计兜底）；unclosed=已实现但半实现/有缺陷/有残留（台账缺陷信号命中）；designed=未实现。`evidence` 由硬性门槛放宽为判定留痕（缺失仅告警）。种子策略同步更新。 |
| 2026-08-25 | v1.3 | 状态三展示名与字段值由 `verified`（已验证）更名为 `closed`（已闭合），与「未闭合」对称，字段枚举同步：`designed | unclosed | closed`。 |
| 2026-08-25 | v1.4 | 审计粒度下沉到 L5 原子行为：`feature_audit` 每行对应一个 L5（key = 台账行 id），L4 为最差进展聚合分支，L5 为树叶子展示状态/链路/证据。 |
