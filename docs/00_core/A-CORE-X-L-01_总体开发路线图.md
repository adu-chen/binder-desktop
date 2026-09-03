---
文档编号：CORE-X-L-01
文档状态：A
负责模块：CORE
文档职责：Binder 总体开发路线图与文档树规划——定义 v1 范围冻结、模块构建顺序与依赖、里程碑↔门禁映射、文档编号体系、测试与上线节奏的编排
上游约束：A-CORE-C-P-01_APC.md、A-CORE-C-R-02_文档去重分层台账.md、ADU.md、ADP.md、CLAUDE.md
直接承接：各模块 G1–G3 文档的产出顺序与编号规划（不产出规则）
使用边界：本文是 L3 程序级编排计划。**不定义任何 RULE / CHAIN / MODULE / TERM / CONSTRAINT，不作为代码规则来源，不承接 REQ_MAP，任何代码 @GOV 不得追溯本文。** 规则唯一来源是 L2 技术设计层（APC §3.2）。本文与映射链的精确关系见 §1.2。
变更要求：v1 范围、模块顺序、里程碑或文档编号体系变更时更新本表；涉及产品定位/范围扩缩须先经 APC 授权（APC §13）
治理成熟度：draft
---

# Binder 总体开发路线图与文档树

## §1 文档定位与硬边界

### 1.1 本文是什么 / 不是什么

**是**：把 APC（宪章）落地为可执行路径的程序级编排——回答"15+3 个模块按什么顺序建、
依赖怎么排、每个里程碑承接什么、要产出哪些文档、文档怎么编号、测试上线怎么走节奏"。

**不是**：不是产品概述（那是 L1 `A-CORE-C-D-01_产品概述`），不是技术方案（那是 L2 各模块
`*-M-T-*`），不是规则来源。本文只排顺序、列文档、定里程碑。

### 1.2 与 SSOT 映射链的关系（本文的硬边界）

APC §2 映射链：

```
REQ(L1) → REQ_MAP → RULE/CHAIN/CONSTRAINT/TERM(L2) → STATE_MACHINE → Plan Item(L3)
        → Issue Trace → @GOV → Test covers → Task Runtime Report
```

"开发计划"在本项目指两种不同产物，本文是前者：

| | **本文：程序级编排路线图** | L3 承接型 Plan Item |
|---|---|---|
| 位置 | 在整个 L1→L2→L3 生产**之上/之前**，是"怎么造这套文档体系"的元编排 | 在各模块 **L2 之后**，是映射链正式成员 |
| 承载规则 | 否（纯编排） | 否（引用规则，不定义），但必须可追溯到 L2 |
| 代码可否追溯 | **禁止**。代码永远只追溯 L2 | 经 RULE/CHAIN 间接关联 |

**铁律**：本文及未来任何新增文档，只能向三层体系（L1 需求 / L2 技术设计 / L3 计划）靠拢或辅助，
不得另立平行权威源。规则的唯一源永远是 L2 技术设计层。

## §2 v1 范围冻结

三项范围待定项已于 **2026-06-09 经人类确认**（同步至 APC §1/§4.1 与台账 §6）：

| 项 | 决策 | 归属模块 |
|---|---|---|
| 文档级时间轴 / 历史回溯 | **纳入 v1** | WS |
| 音视频预览 | **纳入 v1** | PRV |
| Excel / PPT 编辑 | **确认不做**（仅预览 + 打印/导出） | PRV |

**v1 IN（能力域）**：本地 workspace 写作；对话生成 diff → 接受/拒绝 → 落盘（核心交互）；
代理编辑 + 工具调用；CBT 多文件文档构建；DOCX 往返；Excel/PPT/音视频**预览** + 打印导出；
@mention 引用；本地记忆 / 知识库 / 模板工作流；文档级时间轴/历史回溯；
账号认证 + 订阅门控 + AI 网关（云薄端）。

**v1 OUT**（APC §1 + 本次确认）：多人实时协同、内容云同步、插件市场、脱离写作场景的通用聊天助手、
Excel/PPT 编辑。

### 2.1 1.0 Experience Preview 发布切片

2026-07-17 人类确认：完整功能地图不变、开发计划与节奏大体不变，但需要提前发布
**1.0 Experience Preview** 作为核心工作流体验版。该发布切片不改变 §2 的完整 v1 IN/OUT，
只改变"哪些能力是 1.0 上线阻断项"。

1.0 EP 发布阻断项：

| 能力 | 归属模块 |
|---|---|
| workspace 生命周期、文件边界、基础搜索与保存 | WS / SYS |
| 编辑器核心编辑、脏状态、基础文档打开/保存 | ED / DOC / WS |
| 文档预览与 DOCX 基础编辑链路 | DOC / PRV / ED |
| Agent 对话式编辑与真实模型调用 | AG / ENT / GW |
| Diff Edit 审阅、接受/拒绝、落盘 | DE / WS / ED |
| 账号、SessionToken、SubscriptionGate、用量展示 | ACCT / ENT / AUTH / LIC |
| AiGateway 接入 DeepSeek 托管模型与 BYOK | GW / ENT / AG |
| 管理员端最小运营能力与体验反馈闭环 | PORTAL / CLOUD / CORE |
| macOS DMG、签名/公证策略、发布说明 | CORE / SYS |

1.0 EP 非阻断项：CBT、AST、KB、TPL、自部署 Binder 预设模型、多模态模型矩阵、复杂 ModelRouter、
深度专业领域适配与企业级组织权限。上述能力继续按完整路线推进，不因非阻断而取消或停止开发。

详细发布切片、验收线与后续演进见 `A-CORE-X-L-06_1.0体验版发布范围与验收计划.md`。

## §3 模块清单与依赖关系

模块定义见 APC §4。客户端 15：CORE SYS WS ED DE AG CBT DOC PRV REF AST KB TPL ACCT ENT；
云 3：AUTH GW LIC（独立 APC，本项目仅定义接口契约）。

主要依赖（→ 表示"依赖于"）：

```
SYS  → （底座：IPC 协议 / 数据结构 / 跨模块契约 / 状态机总图，被几乎所有模块依赖）
WS   → SYS
ED   → WS
DOC  → WS, ED
DE   → ED, WS, AG（AI 产出 diff）
AG   → SYS, WS, ENT（订阅门禁）, GW（AI 网关）
REF  → WS, AG
CBT  → AG, WS, DE
AST  → WS, AG
KB   → WS, AG
TPL  → AG, WS
PRV  → WS
ACCT → AUTH（云）
ENT  → ACCT, LIC, GW（云）
```

**关键路径注记**：一切 AI 调用（AG/CBT/AST/KB/TPL 的智能能力）经 APC §4.3 都必须先过
**订阅门禁（ENT）→ AI 网关（GW）**，无有效订阅一律拒绝模型调用。因此 AI 链路要真正端到端打通（G5），
依赖 ACCT/ENT + 云 AUTH/GW/LIC。这些模块在旧 binder **无 L4 原料（greenfield，台账 §4.15）**，
是商业化关键路径的最大空白，其契约必须前置。

## §4 构建阶段与顺序

阶段是粗粒度编排，随各模块 G1/G2 暴露的真实依赖可回写调整。每阶段内各模块仍各自走
G1→G2→G3→G4，跨阶段汇入 G5，最终 G6（门禁语义见 APC §5）。

| 阶段 | 模块 | 目标 | 理由 |
|---|---|---|---|
| **P0 治理就绪** | CORE | L0 治理 + APC + 台账 + 本路线图 | 已基本完成 |
| **P1 底座与契约** | SYS、云接口契约(AUTH/GW/LIC)、ACCT/ENT 客户端契约骨架 | IPC/数据结构/状态机总图 + 云三契约（认证/订阅门禁/网关）定义 | 被全体依赖；云契约 greenfield 且在商业化关键路径，必须前置（台账 §8.3） |
| **P2 workspace 与编辑核心** | WS（含时间轴/历史）、ED、DOC | 本地文件/编辑底座 | DE/AG 依赖 ED+WS |
| **P3 代理与 Diff Edit**（旧称 对话编辑与 Agent，2026-07-04 更名） | DE、AG、REF | 产品核心交互（对话→diff→落盘）打通 | 需 P1 的 ENT/GW 契约支撑 AI 调用（开发期可对契约打桩） |
| **P4 上层能力** | CBT、AST、KB、TPL、PRV（含音视频） | 构建/记忆/知识库/模板/预览 | 建立在核心之上 |
| **P5 商业化闭环与发布** | ACCT/ENT 完整、云 AUTH/GW/LIC 实现、订阅门控 E2E | 商业化闭环 + 集成/发布 | G5 跨模块联调 + G6 发布就绪 |

## §5 里程碑 ↔ 门禁 ↔ 通道映射

对齐 APC §11 通道（alpha→beta→RC→GA）与成熟度绑定。

| 里程碑 | 覆盖阶段/门禁 | 通道 | 成熟度 |
|---|---|---|---|
| M0 治理就绪 | P0 | — | draft（现状） |
| M1 架构与契约基线 | P1 过 G2/G3 | — | draft |
| M2 编辑核心可用 | P2 + P3 核心过 G4 | alpha（内测） | draft |
| M2.5 1.0 Experience Preview | 1.0 EP 发布切片过核心 G4 + 必要 G5 冒烟，DeepSeek 托管模型/BYOK/账号订阅闭环可用 | 体验版 | draft |
| M3 全功能集成 | P4 过 G4 + 跨模块 G5 | beta | draft |
| M4 商业化闭环 | P5 + 订阅门控 E2E，过 G5 | RC | baseline（需人类批准） |
| M5 正式发布 | 过 G6（DMG 签名/公证/updater） | GA | controlled（需人类批准） |

成熟度只能由人类推进（APC §11/§14）。

## §6 文档树与编号体系

### 6.1 编号格式

沿用既有体系：`{状态}-{模块}-{范畴}-{类型}-{NN}`。

- 状态：A 现行 / R 旧体系参考 / X 废止。
- 范畴：C 产品级 / M 模块级 / I 接口级 / X 工程跨域级。
- 类型：D 描述定义 / T 技术设计 / S 规范 / P 协议或提示 / R 需求或报告 / L 计划 / V 验证。

层级映射：L1 功能需求 → `D`（描述/定义，载 `<!-- REQ -->`）；L2 技术设计 → `T`/`S`/`I-P`（载
RULE/CHAIN/CONSTRAINT/TERM/STATE_MACHINE，SSOT）；L3 计划 → `L`。

**目录结构约定（2026-06-10 启用）：**

```
docs/
├── 00_core/                程序级 / L0 治理 + CORE 产品级
│   ├── A-CORE-C-P-01_APC.md           （宪章，governance.config.json apc_path 指向）
│   ├── A-CORE-C-R-01_ADUS.md          （派生索引，adus_path 指向，机器生成）
│   ├── A-CORE-C-R-02_文档去重分层台账.md
│   ├── A-CORE-C-D-01_产品概述.md       （产品级 REQ 根）
│   └── A-CORE-X-L-01_总体开发路线图.md  （本文）
├── 01_modules/             每模块一子目录，模块级文档归位
│   ├── SYS/                A-SYS-*（系统底座 L1/L2 六件）
│   ├── WS/                 A-WS-*（工作台）
│   └── {ED,DE,AG,...}/     随模块推进新建
└── 02_cloud/               云端域（AUTH/GW/LIC/PORTAL，独立 APC：A-CLOUD-C-P-01）
                            驻留期硬约定：仅 REQ + 叙述设计，不放注册类治理块（云 APC §3）；
                            云端代码仓库建立时整体迁出
```

- **划分原则**：`00_core` 仅放 L0 治理与 CORE 程序级文档；模块级文档（`A-{M}-*`）一律入
  `01_modules/{模块编码}/`。文档编号体系（§6.1）不变，目录只决定物理位置不决定权威性。
- **工具链无感**：`generate_adus.mjs` 递归扫描 `docs/` 全树，迁移后 ADUS「来源文档」路径自动更新；
  `governance.config.json` 仅锁定 APC/ADUS 两条路径（均在 `00_core`），迁移不需改 config。
- **迁移基线**：SYS 六件、WS 一件已从 `00_core` 迁入 `01_modules/`，generate/audit 计数与发现项不变。

### 6.2 程序级文档（CORE/SYS）

| 编号 | 层 | 文档 | 状态 |
|---|---|---|---|
| A-CORE-C-P-01 | L0 | APC（宪章） | 已建 |
| A-CORE-C-R-01 | L0 | ADUS（派生索引） | 已建 |
| A-CORE-C-R-02 | L4路由 | 文档去重分层台账 | 已建 |
| A-CORE-X-L-01 | L3 | 本路线图 | 已建 |
| A-CORE-X-L-06 | L3发布切片 | 1.0 Experience Preview 发布范围与验收计划 | 已建（1.0 EP 阻断项/非阻断项/验收线） |
| A-CORE-C-D-01 | L1 | 产品概述（产品级 REQ 根） | 已建（G1 闭合条件具备） |
| A-SYS-M-D-01 | L1 | 系统底座功能需求描述（REQ-SYS-*） | 已建（G1 待人类宣布闭合） |
| A-SYS-I-P-03 | L2 | 云接口契约（认证/订阅门禁/网关） | 已建（G2，规则已 generate 注册进 ADUS） |
| A-SYS-C-T-01 | L2 | 系统总体架构（含 MODULE 注册块） | 已建（G2，15 模块编码已 generate 注册进 ADUS） |
| A-SYS-I-P-01 | L2 | 数据结构定义 | 已建（G2，规则/链路/约束已 generate 注册进 ADUS） |
| A-SYS-I-P-02 | L2 | IPC / Tauri command 协议 | 已建（G2，规则/链路已 generate 注册；流式传输 SY-02 延实现期选型） |
| A-SYS-M-S-01 | L2 | 状态机总图与清单 | 已建（G2，X-SEQ-003/X-INTER-001 已注册；除 SM-SYS-01 余核心链 planned，待各模块 G2 回填 SY-04） |
| A-SYS-X-L-01 | L3 | 系统底座开发计划（5 条 Plan Item） | 已建（G3 首件；确立 Plan Item 结构） |
| A-WS-X-L-01 | L3 | 工作台开发计划（5 PI，含 save 冲突保护） | 已建（G3，P2 编码起点） |
| A-ED-X-L-01 | L3 | 编辑器开发计划（5 PI，承接 TipTap/blockId 选型） | 已建（G3） |
| A-DOC-X-L-01 | L3 | 文档转换开发计划（3 PI，双链/子集≥95%回归） | 已建（G3） |
| A-DE-X-L-01 | L3 | Diff Edit 开发计划（3 PI，核心交互 DE 侧） | 已建（G3） |
| X-AG-X-L-01/02/03 | L3 历史 | AG 早期开发、集成与发布切片计划 | 已过期；当前唯一 Active 计划为 A-AG-X-L-04 |
| A-REF-X-L-01 | L3 | 引用开发计划（2 PI，无状态机豁免） | 已建（G3） |
| A-CBT-X-L-01 | L3 | 构建开发计划（3 PI，接管依赖 AG B-3） | 已建（G3，P4） |
| A-AST-X-L-01 | L3 | 记忆协同开发计划（3 PI，写入依赖 AG B-2） | 已建（G3） |
| A-KB-X-L-01 | L3 | 知识库开发计划（2 PI，本地检索硬边界） | 已建（G3） |
| A-TPL-X-L-01 | L3 | 模板工作流开发计划（2 PI，工作流执行机） | 已建（G3） |
| A-PRV-X-L-01 | L3 | 预览输出开发计划（2 PI，只读硬边界） | 已建（G3，P4 收尾） |
| A-ACCT-X-L-01 | L3 | 账号会话开发计划（2 PI，云端打桩） | 已建（G3） |
| A-ENT-X-L-01 | L3 | 订阅门控开发计划（2 PI，商业化关键路径） | 已建（G3，**G3 末件——14 模块 Plan Item 齐备**） |
| A-SYS-C-T-02 | L2视图 | 前端交互架构与流程总图（视图层 SSOT，含欢迎页） | 已建（X-FORBID-007 前端状态投影约束已注册；四条主干流程总图） |

### 6.3 每模块标准文档三件套（WS/ED/DE/AG/CBT/DOC/PRV/REF/AST/KB/TPL/ACCT/ENT）

| 层 | 编号模式 | 内容 |
|---|---|---|
| L1 需求 | `A-{M}-M-D-NN_{模块}功能需求描述` | 功能模块描述、详细描述、边界；载 `<!-- REQ -->` |
| L2 设计 | `A-{M}-M-T-NN_{模块}技术设计` | 模块技术方案；RULE/CHAIN/CONSTRAINT/TERM 定义（SSOT） |
| L2 状态机 | `A-{M}-M-S-NN_{模块}状态机设计`（有状态链路时） | machine_id…test_matrix（APC §6） |
| L2 接口 | `A-{M}-I-P-NN_{模块}接口协议`（跨模块/Tauri command 时） | command 签名、数据契约 |
| L3 计划 | `A-{M}-X-L-NN_{模块}开发计划` | 承接型 Plan Item（后于 L2） |

NN 为两位序号，同模块同类型递增。NFR/测试矩阵归 VAL 体系（对接 APC §9/§10）。

**已落地模块文档（随各模块推进登记）：**

| 编号 | 层 | 文档 | 状态 |
|---|---|---|---|
| A-WS-M-D-01 | L1 | 工作台功能需求描述（REQ-WS-001~005） | 已建（G1 待人类宣布闭合；对接 SY-03/SY-04） |
| A-WS-M-T-01 | L2 | 工作台技术设计（规则 SSOT + workspace.db 完整 schema） | 已建（G2，规则/链路/约束/术语已 generate 注册；解除 SY-03 列定义） |
| A-WS-M-S-01 | L2 | 工作台状态机设计（SM-WS-01/02/03） | 已建（G2，save/diff 落盘/时间轴还原三机；回填 A-SYS-M-S-01 总图，解除 SY-04） |
| A-ED-M-D-01 | L1 | 编辑器功能需求描述（REQ-ED-001~005） | 已建（G1 待人类宣布闭合；inline-assist/autocomplete 已确认归 AG，2026-06-11） |
| A-ED-M-T-01 | L2 | 编辑器技术设计（规则 SSOT：会话/内容块/统一模型/分页媒体/装饰层） | 已建（G2，规则/链路/约束/术语已 generate 注册；TipTap 选型延实现期） |
| A-ED-M-S-01 | L2 | 编辑器状态机设计（SM-ED-01 文档编辑会话） | 已建（G2，回填 A-SYS-M-S-01 总图 + ED↔WS X-INTER-001 转移点） |
| A-DOC-M-D-01 | L1 | 文档转换功能需求描述（REQ-DOC-001~005） | 已建（G1 待人类宣布闭合；双链分离方案承旧版实证） |
| A-DOC-M-T-01 | L2 | 文档转换技术设计（规则 SSOT：双链管线/样式子集/blockId 兜底） | 已建（G2，规则/链路/约束/术语已 generate 注册） |
| A-DOC-M-S-01 | L2 | 文档转换状态机设计（SM-DOC-01 docx 开/存） | 已建（G2，回填总图 + DOC↔ED/DOC↔WS X-INTER-001 转移点） |
| A-AG-M-D-01 | L1 | 代理功能需求描述（REQ-AG-001~005，含局部修改/辅助续写归 AG） | 已建（P3 首件，G1 待人类宣布闭合） |
| A-DE-M-D-01 | L1 | Diff Edit 功能需求描述（REQ-DE-001~005，diff 生成/审阅/终态） | 已建（G1 待人类宣布闭合；对接 SM-WS-02 X-INTER-001） |
| A-REF-M-D-01 | L1 | 引用功能需求描述（REQ-REF-001~004，@mention 解析与注入） | 已建（G1 待人类宣布闭合；P3 三模块 L1 层齐备） |
| A-AG-M-T-01 | L2 | 代理技术设计（规则 SSOT：回合/门禁承接/工具循环/上下文/辅助） | 已建（G2，规则/链路/约束/术语已 generate 注册） |
| A-AG-M-S-01 | L2 | 代理状态机设计（SM-AG-01/02/03） | 已建（G2，回填总图四行 active；AG 产出→DE 已随 DE G2 双侧闭合） |
| A-DE-M-T-01 | L2 | Diff Edit 技术设计（规则 SSOT：粒度/生成/载荷/终态/过期/冲突） | 已建（G2，规则/链路/术语已 generate 注册；约束全复用不新增） |
| A-DE-M-S-01 | L2 | Diff Edit 状态机设计（SM-DE-01 diff 终态机） | 已建（G2，回填总图；AG→DE→WS 核心交互状态机链全链建机完毕） |
| A-REF-M-T-01 | L2 | 引用技术设计（规则 SSOT：目标模型/解析/注入载荷/锚失效；状态机豁免复核成立） | 已建（G2，规则/链路/术语已 generate 注册；RefInjection 登记 SYS 契约层；**P3 三模块 L1+L2 齐备**） |
| A-CBT-M-D-01 | L1 | 构建功能需求描述（REQ-CBT-001~004，复用 AG 基建 + diff 路径） | 已建（P4 首件，G1 待人类宣布闭合） |
| A-AST-M-D-01 | L1 | 记忆协同功能需求描述（REQ-AST-001~004，写入可控不静默） | 已建（G1 前置：AST-01 存储范围 / AST-02 写入策略待人类决策） |
| A-KB-M-D-01 | L1 | 知识库功能需求描述（REQ-KB-001~004，离线不上云硬边界） | 已建（G1 前置：KB-01 库范围待人类决策，建议与 AST-01 一并） |
| A-TPL-M-D-01 | L1 | 模板工作流功能需求描述（REQ-TPL-001~004，执行委托 AG/CBT） | 已建（G1 前置：TPL-03 范围待人类决策；工作流状态机 G2 评估倾向建机） |
| A-PRV-M-D-01 | L1 | 预览输出功能需求描述（REQ-PRV-001~005，只读硬边界 + v1 冻结承接） | 已建（G1 待人类宣布闭合；**P4 五模块 L1 层齐备**） |
| A-CBT-M-T-01 | L2 | 构建技术设计（任务分解/产物 diff 化/控制态/接管协议） | 已建（G2，已 generate 注册） |
| A-CBT-M-S-01 | L2 | 构建状态机设计（SM-CBT-01） | 已建（G2，回填总图 + 构建移交 双侧闭合） |
| A-AST-M-T-01 | L2 | 记忆协同技术设计（workspace 隔离 + 自动写入分级，人类决策落规则） | 已建（G2，已 generate 注册） |
| A-AST-M-S-01 | L2 | 记忆协同状态机设计（SM-AST-01） | 已建（G2，回填总图记忆写入行） |
| A-KB-M-T-01 | L2 | 知识库技术设计（workspace 隔离/本地检索硬边界；豁免复核成立） | 已建（G2，已 generate 注册） |
| A-TPL-M-T-01 | L2 | 模板工作流技术设计（隔离/原子实例化/工作流表达；评估结论=建机） | 已建（G2，已 generate 注册） |
| A-TPL-M-S-01 | L2 | 模板工作流状态机设计（SM-TPL-01） | 已建（G2，总图新增工作流执行行） |
| A-PRV-M-T-01 | L2 | 预览输出技术设计（管线归属解除 DOC-03/只读硬边界；豁免复核成立） | 已建（G2，已 generate 注册；**P4 五模块 L2 齐备，总图 12 机全 active**） |
| A-ACCT-M-D-01 | L1 | 账号会话功能需求描述（REQ-ACCT-001~004，CLOUD-AUTH 客户端承接） | 已建（greenfield；G1 待人类宣布闭合） |
| A-ACCT-M-T-01 | L2 | 账号会话技术设计（会话段承接 + 令牌安全持有；零新增注册项） | 已建（G2，规则已 generate 注册） |
| A-ENT-M-D-01 | L1 | 订阅门控功能需求描述（REQ-ENT-001~004，门禁/网关/BYOK/用量客户端） | 已建（greenfield；G1 待人类宣布闭合） |
| A-ENT-M-T-01 | L2 | 订阅门控技术设计（禁缓存通过/无 provider 直连/BYOK 零残留/门户购买） | 已建（G2，规则已 generate 注册；**15 客户端模块文档层全部齐备**） |
| A-WS-I-P-01 | L2接口 | 工作台接口协议（17 command + 1 事件，save 基线校验/显式覆盖） | 已建（实现前置件首份；零新增注册项） |
| A-ED-I-P-01 | L2接口 | 编辑器接口协议（会话/装饰层/blockId 定位） | 已建（实现前置件；X-FORBID-005 唯一通道固化） |
| A-DE-I-P-01 | L2接口 | Diff Edit 接口协议（生成/裁决/过期，核心交互 DE 侧） | 已建（实现前置件；accept 唯一触发落盘成链） |
| A-AG-I-P-01 | L2接口 | 代理接口协议（回合/工具/辅助，AI 调用面） | 已建（实现前置件；门禁不变量内建） |
| A-DOC-I-P-01 | L2接口 | 文档转换接口协议（编辑链/预览链，双链分离） | 已建（实现前置件；X-SEQ-004 唯一入口） |
| A-REF-I-P-01 | L2接口 | 引用接口协议（候选/锚校验/注入组装） | 已建（实现前置件） |
| A-CBT-I-P-01 | L2接口 | 构建接口协议（构建回合/接管双向回执） | 已建（实现前置件） |
| A-AST-I-P-01 | L2接口 | 记忆协同接口协议（自动写入/供给，隔离） | 已建（实现前置件） |
| A-KB-I-P-01 | L2接口 | 知识库接口协议（入库/本地检索硬边界） | 已建（实现前置件） |
| A-TPL-I-P-01 | L2接口 | 模板工作流接口协议（模板/工作流委托 AG·CBT） | 已建（实现前置件） |
| A-PRV-I-P-01 | L2接口 | 预览输出接口协议（多格式预览只读/导出） | 已建（实现前置件） |
| A-ACCT-I-P-01 | L2接口 | 账号会话接口协议（会话段承接，令牌不回传明文） | 已建（实现前置件） |
| A-ENT-I-P-01 | L2接口 | 订阅门控接口协议（门禁前置唯一通道/BYOK 零残留/门户） | 已建（实现前置件） |

## §7 测试与上线节奏

不在此重定义，引用 APC：

- 测试分层（单元/集成/e2e/`// covers: BR-*`/CI 硬门）见 APC §10；状态机强制转移测试矩阵见 APC §6。
- `governance:audit` 始终硬阻断（APC §12），任一 failed 阻断 READY/合入/发布。
- 上线：semver；alpha→beta→RC→GA；DMG + macOS 签名/公证；Tauri updater；订阅购买在官网门户，
  桌面端只做门禁校验（APC §11）。
- 里程碑↔通道映射见本文 §5。

## §8 关键风险与前置项

1. **商业化链路 greenfield**：ACCT/ENT + 云 AUTH/GW/LIC 无 L4 原料，且 AI 端到端依赖订阅门禁。
   契约须在 P1 前置定义，否则 P3 的 AI 链路无法真正联调（开发期需对契约打桩）。
2. **状态机驱动覆盖**：APC §6 列出的核心链（diff 终态、代理回合、tool-call、save、docx、
   CBT 构建、记忆写入、autocomplete、inline-assist、账号会话与订阅门禁）必须先有状态机设计（L2），
   不得跳过直接实现（ADP SC-06/SC-07）。
3. **范围回写纪律**：任何范围再调整须先经 APC 授权，再回写本文 §2 与台账 §6（APC §13）。
4. **时间轴/历史新增项**：归 WS，台账 §4.3 相关旧文档已由 SCOPE 升为取材项，但需在 WS 的 G2
   核对其与新架构（pending_diffs/workspace.db 迁移）的一致性。
5. **云端 APC 立项输入（2026-06-12 人类确认两项决策）**：
   - **个人账户门户**：官网门户升级定位为个人账户门户——能力清单：订阅管理与购买、账单/发票、
     用量明细（LIC）、设备与会话管理（强制下线）、BYOK 密钥管理（GW 代持）、资料与安全、账号
     注销；桌面端契约不变（仅展示 + 跳转，BR-ENT-DATA-002）。
   - **用户库分工**：AUTH 持身份库（PII 最小化）/ LIC 持商业数据（user_id 关联不复制身份）/
     GW 持密钥保管（与身份分离）；**网关不留存调用内容**已升格产品承诺（BR-SYS-DATA-004，
     A-SYS-I-P-03 v1.1），云端实现不得弱化。

## §9 UI / VI 设计原料登记与 UI 板块路线图

本节登记人类放入 `docs/binder-vi/`（VI 规范）与 `docs/binder_ui/`（UI 原型）的设计稿，
并定义 Binder 自有的 UI 推进路线。**本节不定义任何规则**（遵 §1.2 硬边界）：
UI 稿仅描述界面形态，界面规则的 SSOT 仍在各模块 L2 技术设计层，代码 @GOV 不得追溯本节。

### 9.1 原料定位（L4 只读，非权威）

| 原料 | 路径 | 处置 | 可信部分 | 须改写部分 |
|---|---|---|---|---|
| VI 规范（方向 D 选定版） | docs/binder-vi/ | L4 设计原料 · 可升格为设计系统种子 | 色彩 / 字体阶梯 / 标志 / 间距 / 图标网格（无架构包袱） | — |
| UI 原型 | docs/binder_ui/ | L4 设计原料 · 仅参考布局/视觉/diff 交互 | 大布局骨架、diff 三态审阅交互、dark theme、与 VI token 一致 | 见 §9.3 架构对齐改写清单 |

> 与台账 §4.16 一致：设计稿是输入原料，不是权威源。
> **Binder 有独立完整的 UI 路线图，与参考稿有别**——参考稿只提供布局/风格/配色起点。

### 9.2 Binder 自有 UI 路线（just-in-time，贴模块推进）

1. **设计系统种子（一次性全局件）**：从 VI 方向 D 抽 design tokens
   （色 `#F4F1EA`/`#0E0C0A`/`#C24A2A`/`#2E6F4A`、IBM Plex + Noto SC 字阶、间距、1.5 stroke 图标），
   定为 Binder 设计基线。这是唯一现在即可固化的全局资产。
2. **L1 线框（按模块、随该模块 G1 出）**：低保真，回答布局/入口/主流程，
   不锁视觉细节（容忍 UI 细节不符）。
3. **L2 交互设计（按模块、随该模块 L2 出）**：高保真，定状态机视图、diff 三态、异常/空态；
   新颖 AI 交互（agent 工具链、diff 审阅、@引用、CBT 对照）单独做交互原型验证。
4. **架构对齐改写清单（贯穿 UI 实现的硬前置）**：见 §9.3。

> UI 不集中前置画完，而是贴每个模块 G1（线框）/L2（交互）随取随做——
> 规避"无开发基础下 UI 无法与实际对齐"的问题。

### 9.3 架构对齐改写清单（UI 实现验收前置）

参考 UI 原型承载了旧 binder 的**客户端持密钥 + 直连 OpenAI/DeepSeek**假设，
与 REQ-CORE-008 / APC §4.3（云薄端：密钥仅服务端、订阅门控硬门、BYOK 经网关）冲突。
任何沿用参考稿的界面，实现前必须按下表改写 / 补齐：

| # | 参考稿现状 | 必须改为 |
|---|---|---|
| 1 | TitleBar/Welcome 出现「配置 API Key · OpenAI/DeepSeek」「… · DEEPSEEK」 | 移除客户端密钥配置入口；改为账号登录态 + 当前订阅/模型由网关下发 |
| 2 | 无账号/登录/订阅界面 | 补齐 ACCT 登录会话 + ENT 订阅门控（无有效订阅拒绝模型调用） |
| 3 | 模型直连厂商假设 | 模型调用经 GW 网关；BYOK 亦经网关路由，客户端不持密钥、不可绕过门禁 |
| 4 | mock 数据「只支持 OpenAI/DeepSeek」 | 不在客户端硬编码 provider；可用模型由网关能力决定 |
| 5 | 缺失已确认范围界面 | 补齐音视频预览（PRV）、CBT 对照构建、@mention 引用流 三类界面 |

> 本清单挂为 UI 实现的硬前置：相关界面在对应模块 G2/G4 实现时须逐条核对。

## §10 后续动作

1. 进入 **P1**：先建 `A-CORE-C-D-01_产品概述`（L1 根）与 SYS 底座设计 + 云接口契约（L2），各走 G1→G2。
2. 各模块按 §6.3 三件套产出，严守 L1→L2→L3 顺序与 SSOT。
3. 每里程碑声明承接的 REQ/CHAIN、门禁状态、NFR 达标（APC §11）。
4. 1.0 EP 以 `A-CORE-X-L-06` 为发布切片入口；模块规则仍回到各模块 L1/L2/L3，不另立规则源。
5. 本路线图随 P1/P2 暴露的真实依赖回写调整。
6. UI 板块按 §9：先固化设计系统种子，其余线框/交互贴模块 G1/L2 随取随做；
   沿用参考稿处须逐条过 §9.3 架构对齐改写清单。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-09 | v1.0 | 初版：v1 范围冻结（含三项确认）、模块依赖与构建阶段 P0–P5、里程碑↔门禁↔通道映射、文档编号体系与三件套、测试上线节奏引用、关键风险前置项 |
| 2026-06-10 | v1.1 | 新增 §9 UI/VI 设计原料登记与 UI 板块路线图（L4 原料定位、Binder 自有 UI 路线 just-in-time、§9.3 架构对齐改写清单）；原 §9 后续动作顺延为 §10 并补 UI 板块动作项 |
| 2026-06-10 | v1.2 | P1 路径 A 推进：§6.2 文档树补 A-SYS-M-D-01（SYS L1）行，SYS L2 五件调整为 G2 待建（云接口契约 A-SYS-I-P-03 优先），并修正已建文档（路线图/产品概述）状态 |
| 2026-06-10 | v1.3 | SYS G2 首件落地：A-SYS-I-P-03 云接口契约（SSOT）建成，§6.2 状态更新；标注 A-SYS-C-T-01（MODULE 注册块）须先于 governance:generate |
| 2026-06-10 | v1.4 | A-SYS-C-T-01 系统总体架构建成（15 模块编码 MODULE 注册基线 + 模块依赖图 + X-SEQ-002/X-FORBID-003）；首次 governance:generate 落地 ADUS（15 模块 / 4 规则 / 3 链路 / 5 约束 / 4 术语）；§6.2 A-SYS-C-T-01、A-SYS-I-P-03 更新为已注册 |
| 2026-06-10 | v1.5 | A-SYS-I-P-01 数据结构定义建成（跨模块共享实体契约 + workspace.db schema 基线与迁移契约框架）；注册 DATA-MIGRATE 链路、BR-SYS-DATA-002 / BR-SYS-PERSIST-001 规则、X-SHARED-001 约束（承接 REQ-SYS-002）；ADUS 累计 6 规则 / 4 链路 / 6 约束；§6.2 状态更新 |
| 2026-06-10 | v1.6 | A-SYS-I-P-02 IPC / Tauri command 协议建成（command 命名/注册约定、请求-响应信封、统一错误形状、流式逻辑契约）；注册 IPC-CALL 链路、BR-SYS-DATA-003 / BR-SYS-OBS-002 / BR-SYS-STATE-002 规则（承接 REQ-SYS-001），复用 X-SEQ-002 不新增约束；人类确认流式物理传输（SY-02）延实现期选型；ADUS 累计 9 规则 / 5 链路 / 6 约束；§6.2 状态更新 |
| 2026-06-10 | v1.7 | A-SYS-M-S-01 状态机总图建成（核心链注册清单、注册前置约束 X-SEQ-003、跨模块转移契约 X-INTER-001，承接 REQ-SYS-003），SM-SYS-01 active 其余核心链 planned；**SYS G2 五件（C-T-01/I-P-01/I-P-02/I-P-03/M-S-01）齐备**；ADUS 累计 9 规则 / 5 链路 / 8 约束 / 4 术语；待人类宣布 SYS G2 闭合 |
| 2026-06-10 | v1.8 | 进入 WS 工作台模块：A-WS-M-D-01 工作台功能需求描述（L1）建成，声明 REQ-WS-001~005（workspace 生命周期与边界 / 文件 I/O 与 save / pending_diffs 持久化与落盘 / 时间轴回溯 / 控制态缓存与重启可恢复），承接 REQ-CORE-001/002/007 与 A-SYS-I-P-01 契约，对接 SY-03/SY-04；§6.3 新增「已落地模块文档」跟踪表登记 A-WS-M-D-01 |
| 2026-06-10 | v1.9 | 文档树结构化划分：新增 §6.1 目录结构约定——`00_core` 仅留 L0 治理 + CORE 程序级（5 件），模块级文档迁入 `01_modules/{模块编码}/`；SYS 六件迁入 `01_modules/SYS/`、WS 一件迁入 `01_modules/WS/`；generate 递归扫描全树，ADUS 来源路径自动更新，audit 计数与发现项不变（15 模块 / 9 规则 / 5 链路 / 8 约束 / 4 术语，发现项均为「无代码」预期空），governance.config.json 无需改动（APC/ADUS 仍在 00_core） |
| 2026-06-10 | v1.10 | WS G2 技术设计落地（两件）：A-WS-M-T-01 工作台技术设计（规则 SSOT + workspace.db 完整 schema 解除 SY-03）注册链路 WS-FS/WS-SAVE/WS-DIFF-APPLY/WS-TIMELINE、规则 BR-WS-DATA-001/002·STATE-001/002·PERSIST-001/002、约束 X-FORBID-004、术语 TERM-WS-001 Workspace / TERM-WS-002 DocumentTimeline；A-WS-M-S-01 状态机设计定义 SM-WS-01 save / SM-WS-02 diff 落盘 / SM-WS-03 时间轴还原（解除 SY-04），回填 A-SYS-M-S-01 总图（save 行 active + 两行新增，§5 X-INTER-001 登记 WS 侧）；§6.3 跟踪表登记两件；ADUS 累计 15 模块 / 15 规则 / 9 链路 / 9 约束 / 6 术语，audit 发现项均为「无代码」预期空（15 OWNER_MISSING / 9 CHAIN_ENTRY_UNANNOTATED / 6 TERM_ORPHAN） |
| 2026-06-10 | v1.11 | 进入 ED 编辑器模块（P2 第二件）：A-ED-M-D-01 编辑器功能需求描述（L1）建成，声明 REQ-ED-001~005（富文本编辑运行时 md/docx / block 模型与 blockId 定位 / 分页系统 / 图文媒体能力 / 编辑器装饰层供 diff 呈现），承接 APC §4.1 ED 职责与 REQ-CORE-002/004/009，依赖 WS（ED→WS）；TipTap 选型/blockId 策略/decoration 接口延 ED L2，inline-assist·autocomplete 归属待人类确认（A-SYS-M-S-01 §3）；§6.3 跟踪表登记；ED L1 仅载 REQ（不入 ADUS 自动抽取），generate 计数不变、audit 发现项不变 |
| 2026-06-11 | v1.12 | ED G2 两件落地：A-ED-M-T-01 编辑器技术设计（规则 SSOT）注册链路 ED-EDIT/ED-DECOR、规则 BR-ED-STATE-001/002·BR-ED-DATA-001/002/003、约束 X-FORBID-005、术语 TERM-ED-001 Block / TERM-ED-002 EditorDecoration；A-ED-M-S-01 状态机设计定义 SM-ED-01 文档编辑会话机（decoration/分页豁免建机并记依据），回填 A-SYS-M-S-01 总图 v1.2（新增文档编辑会话行 + §5 ED↔WS 脏态/保存 X-INTER-001 转移点）；§6.3 跟踪表登记 G2 两件；ADUS 累计 15 模块 / 20 规则 / 11 链路 / 10 约束 / 8 术语，audit 发现项均为「无代码」预期空（20 OWNER_MISSING / 11 CHAIN_ENTRY_UNANNOTATED / 8 TERM_ORPHAN）；TipTap 选型/blockId 算法/docx 样式子集延后记明 owner |
| 2026-06-11 | v1.13 | 人类决策落地 + P2 文档补齐 + 进入 P3：① inline-assist/autocomplete 归属确认 AG（总图 v1.3、ED 三件同步）；② ED 选型记录回填（A-ED-M-T-01 §11.1，人类授权 AI 推荐，旧版仓库实证：TipTap v3 系 + tiptap-pagination-plus、blockId=block_+UUID+appendTransaction、docx 样式子集三级）；③ DOC 模块三件落地（L1 REQ-DOC-001~005 + T-01 双链分离管线/样式子集执行契约 + S-01 SM-DOC-01，总图 v1.4 回填 docx 行 active）；④ P2（WS/ED/DOC）L1+L2 自检完整，进入 P3：A-AG-M-D-01 对话代理 L1（REQ-AG-001~005）建成；ADUS 累计 15 模块 / 24 规则 / 12 链路 / 11 约束 / 9 术语，audit 发现项均为「无代码」预期空 |
| 2026-06-11 | v1.14 | P3 L1 层齐备：A-DE-M-D-01 对话编辑 L1（REQ-DE-001~005：diff 生成/审阅呈现/终态裁决/载荷形状/过期与外部变更，对接 SM-WS-02 X-INTER-001 WS 侧与 ED-DECOR，终态机待 DE G2 建机回填总图）、A-REF-M-D-01 引用 L1（REQ-REF-001~004：@mention 触发解析/读取注入/锚定失效感知/经 ED 表面，状态机豁免预判待 G2 复核）建成；§6.3 跟踪表登记两件；L1 仅载 REQ 不入 ADUS 抽取，generate 计数不变（15 模块 / 24 规则 / 12 链路 / 11 约束 / 9 术语），audit 发现项不变 |
| 2026-06-11 | v1.15 | AG G2 两件落地（P3 G2 起步）：A-AG-M-T-01 对话代理技术设计（规则 SSOT）注册链路 AG-TURN/AG-TOOL/AG-ASSIST、规则 BR-AG-STATE-001/002/003·VERIFY-001·DATA-001·PERSIST-001、约束 X-FORBID-006（AI 修改落盘唯一路径 AG→DE→WS）、术语 TERM-AG-001~004（AgentTurn/ToolCall/InlineAssist/Autocomplete）；A-AG-M-S-01 定义 SM-AG-01 回合机 / SM-AG-02 工具循环机 / SM-AG-03 辅助回合机（autocomplete 与 inline-assist 同机承载），总图 v1.5 四行转 active（仅余 DE/CBT/AST 三行 planned）；§6.3 跟踪表登记两件；ADUS 累计 15 模块 / 30 规则 / 15 链路 / 12 约束 / 13 术语，audit 发现项均为「无代码」预期空 |
| 2026-06-11 | v1.16 | DE G2 两件落地：A-DE-M-T-01 对话编辑技术设计（规则 SSOT）——裁决粒度定为内容块级（解除 DE-01）、payload 载荷形状版本化 + before_snapshot 基准（解除 REQ-DE-004 占位）、三终态互斥不可逆与 accepted 唯一触发落盘、保守过期（解除 DE-03 机制）、多 diff 冲突重检（解除 DE-02），注册链路 DE-DIFF、规则 BR-DE-DATA-001/002/003·STATE-001/002、术语 TERM-DE-001 PendingDiff / TERM-DE-002 ChangeProposal（解除 A-SYS-I-P-01 §6 延期），约束全复用不新增；A-DE-M-S-01 定义 SM-DE-01 diff 终态机并闭合 AG 交接与 WS 落盘两个单侧转移点——**「对话生成 diff→人类裁决→落盘」核心交互 AG→DE→WS（→ED）状态机链全链建机完毕**（总图 v1.6，仅余 CBT/AST 两行 planned）；ADUS 累计 15 模块 / 35 规则 / 16 链路 / 12 约束 / 15 术语，audit 发现项均为「无代码」预期空 |
| 2026-06-11 | v1.17 | REF G2 落地（P3 G2 收尾）：A-REF-M-T-01 引用技术设计（规则 SSOT）——v1 引用目标模型（文档/内容块/资源）、触发与解析（无悬空引用、经 ED 扩展点）、注入载荷契约（读取时点一致/来源可溯/裁剪归 AG）、锚失效保守校验；REF-02 经 X-SHARED-001 回写解除：RefInjection 共享结构登记 A-SYS-I-P-01 v1.1；REF-04 状态机豁免复核成立（请求-响应式，列明失效触发条件）；注册链路 REF-INJECT、规则 BR-REF-DATA-001/002·VERIFY-001、术语 TERM-REF-001/002，约束零新增；**P3（AG/DE/REF）L1+L2 文档层齐备**；ADUS 累计 15 模块 / 38 规则 / 17 链路 / 12 约束 / 17 术语，audit 发现项均为「无代码」预期空 |
| 2026-06-11 | v1.18 | 进入 P4，五模块 L1 层齐备：A-CBT-M-D-01（多文件构建/产物 diff 化/中断恢复/Chat↔Build 接管，复用 AG 门禁基建与 X-FORBID-006 路径）、A-AST-M-D-01（记忆本地存储/写入可控不静默/注入消费/不上云）、A-KB-M-D-01（知识条目存储组织/主动入库/检索召回/离线硬边界）、A-TPL-M-D-01（模板管理/应用/工作流执行委托 AG·CBT）、A-PRV-M-D-01（docx 全保真预览/Excel·PPT 预览无编辑入口/音视频预览/打印导出，承接 v1 冻结）；三项同构待人类决策集中暴露——AST-01/KB-01/TPL-03（workspace 级 vs 应用全局级，建议一并定）+ AST-02（记忆写入策略）；L1 仅载 REQ 不入 ADUS 抽取，计数维持 15 模块 / 38 规则 / 17 链路 / 12 约束 / 17 术语 |
| 2026-06-12 | v1.19 | 人类决策落档 + P4 G2 八件齐备：① 决策——本地资产（AST/KB/TPL）一律 workspace 级隔离、记忆自动写入无需确认按类型分级（三份 L1 v1.1 落档）；② CBT G2 两件（任务分解/产物 diff 化含新建文档空基准/控制态无悬挂/接管协议双向回执，SM-CBT-01）；③ AST G2 两件（决策落规则 + 可见可溯源可撤销三要素、类型基线四级，SM-AST-01）；④ KB G2（本地检索硬边界禁云端 embedding、主动入库可溯源，豁免复核成立）；⑤ TPL G2 两件（工作流表达模型、评估结论=建机，SM-TPL-01 总图新增行）；⑥ PRV G2（管线归属决策解除 DOC-03/PRV-01：Excel/PPT 归 DOC 预览链扩展、音视频系统解码归 PRV；只读硬边界 G4/G5 强制核验）；总图 v1.7：**12 台状态机全 active、无 planned 行（SY-04 闭合）**；ADUS 累计 15 模块 / 51 规则 / 22 链路 / 12 约束 / 23 术语，audit 发现项均为「无代码」预期空（51/22/23） |
| 2026-06-12 | v1.20 | P1 尾部 greenfield 收尾（ACCT/ENT 客户端契约骨架四件）：ACCT L1+T（SM-SYS-01 会话段运行时承接、reauth_required 可靠触发、SessionToken Keychain 安全持有与 provider 密钥严格区分）；ENT L1+T（门禁逐次裁决禁缓存通过 X-FORBID-002、网关通道无 provider SDK/直连 X-FORBID-001、BYOK 一次性上送零残留不豁免门禁、用量服务端唯一数据源 + 官网门户购买；ENT-03 高频门禁延迟解法限定服务端凭证类）；两模块按 A-SYS-I-P-03 承接原则零新增 CHAIN/TERM/CONSTRAINT/状态机，仅 +6 规则；**15 个客户端模块（CORE+14 业务）文档层（L1+L2）全部齐备**；ADUS 累计 15 模块 / 57 规则 / 22 链路 / 12 约束 / 23 术语，audit 发现项均为「无代码」预期空（57/22/23）；剩余文档工作：云端 AUTH/GW/LIC 独立 APC、各模块 I-P command 签名、L3 开发计划、控制态扩展表批量登记 SYS |
| 2026-06-12 | v1.21 | 账号管理两项人类决策落档：① 官网门户升级为个人账户门户（能力清单入 §8.5，ACCT/ENT L1 v1.1 同步，桌面端契约不变）；② 网关内容不留存升格产品承诺——A-SYS-I-P-03 v1.1 新增 BR-SYS-DATA-004（调用内容瞬时过境不持久化/脱敏计量元数据/BYOK 密钥与身份分离，云端 GW APC 前置约束），REQ_MAP 补 REQ-SYS-005 映射；用户库分工（AUTH 身份 PII 最小化 / LIC 商业数据 / GW 密钥保管）入 §8.5 作云端 APC 立项输入；ADUS 累计 58 规则（+1） |
| 2026-06-12 | v1.22 | Q2 缺口补遗（人类确认采纳建议方案）：WS 新增 BR-WS-OBS-001（外部变更感知：加载基线/文件监视/事件供 ED·DE·REF 订阅；保存冲突保护：save 前基线校验、不静默覆盖外部修改、覆盖须显式确认）+ SM-WS-01 扩展冲突路径（saveConflict/overrideConfirmed）；ED 新增 BR-ED-STATE-003（无脏态自动重载、有脏态进 Conflicted 显式裁决，双向内容损失均须用户选择）+ SM-ED-01 新增 Conflicted 态；Q5 澄清项留痕至 A-AG-M-T-01 §10 债务（G3 前裁定）；ADUS 累计 60 规则（+2）；audit 发现项均为「无代码」预期空 |
| 2026-06-12 | v1.23 | 云端立项落地（bootstrap：四项决策→草案→人类确认→豁免生效）：创建 A-CLOUD-C-P-01 云端 APC（A 状态，docs/02_cloud/）——Node/TS 模块化单体、AUTH/GW/LIC/PORTAL 四模块、复用同款治理 audit 阻断、内容不留存为最高产品承诺、驻留期不放注册类治理块硬约定、独立仓库迁移流程；本仓库 governance.config.json 不改（仍指客户端 APC）；§6.1 目录树补 02_cloud；客户端挂起项（ACCT-01/03、ENT-01/02/03）的解除路径登记于云 APC §7 |
| 2026-06-12 | v1.24 | 云端域 L1 根落地：A-CLOUD-C-D-01 云端产品概述——声明 REQ-CLOUD-001~008（认证会话/门禁裁决/网关转发计量/BYOK 代持/订阅计费用量/个人账户门户/数据边界合规/契约兼容）；客户端约束的服务端对偶表（X-FORBID-001/002、X-SEQ-001、BR-SYS-DATA-004 的服务端落实）；客户端五个挂起项（ACCT-01/03、ENT-01/02/03）解除路径分配至对应 REQ；G1 暴露三项待人类决策（CLOUD-01 认证方式、CLOUD-02 订阅档位定价、CLOUD-03 合规清单）；驻留期硬约定遵守（仅 REQ+叙述），ADUS 计数零变化 |
| 2026-06-12 | v1.25 | 云端转入「独立域后续完善」状态（人类指示，CLOUD-01/02/03 决策与四模块 L1 暂挂，宪章+L1 根已就绪随时可续）；客户端进入实现前置件推进：① A-SYS-I-P-01 v1.2 批量登记——MemoryInjection/KnowledgeInjection 注入结构 + 七张控制态扩展表基线（agent_turns/build_runs/workflow_runs/templates/workflows/memory_entries/knowledge_entries，均 workspace 级隔离），解除 AG/CBT/TPL/AST/KB 五模块「实现前登记」承诺；② A-WS-I-P-01 工作台接口协议建成（首份 command 签名：17 条 command + 1 事件，save 带基线校验与显式覆盖、read 返基线哈希），零新增注册项；ADUS 计数不变 |
| 2026-06-12 | v1.26 | 实现前置件续（核心交互链 command 面）：A-ED-I-P-01 编辑器接口协议（会话 command 5 含 resolve_conflict 对接 Conflicted、装饰层 4 X-FORBID-005 唯一通道、blockId 定位 2 含 apply_edit 内联采纳落点）；A-DE-I-P-01 对话编辑接口协议（生成 3 接 ChangeProposal+锚校验、裁决 2 accept 唯一触发 ws.apply_diff 不可逆、过期重检 2）+ §5 跨模块对接索引（AG/ED/WS）；至此 AG→DE→WS→ED 核心交互的 command 面成链；零新增注册项，ADUS 计数不变 |
| 2026-06-12 | v1.27 | 实现前置件续（AI 调用面 + 文档往返面）：A-AG-I-P-01 对话代理接口协议（chat 回合 4 含门禁不变量内建、工具回注 1、内联辅助 1 静默失败、上下文组装对接索引 WS/REF/AST/KB；X-SEQ-001 门禁前置固化为接口不变量、产出唯一经 de.generate_diff）；A-DOC-I-P-01 文档转换接口协议（编辑链 3 open/save/style_report、预览链 1 render_preview 双链分离、X-SEQ-004 二进制唯一入口）；至此核心 AI 链（AG 门禁→网关→产出→裁决→落盘）与文档往返（DOC↔ED↔WS）command 面完整；零新增注册项，ADUS 计数不变 |
| 2026-06-12 | v1.28 | 实现前置件 I-P 全量收齐（剩余 8 份）：REF/CBT/AST/KB/TPL/PRV/ACCT/ENT 接口协议建成——REF（候选/锚校验/注入组装）、CBT（构建回合+接管双向回执）、AST（自动写入+供给，隔离）、KB（入库+本地检索硬边界）、TPL（模板+工作流委托 AG/CBT）、PRV（多格式预览只读+导出）、ACCT（会话段承接，令牌不经 command 回传）、ENT（invoke_model 门禁前置唯一通道+BYOK 零残留+门户跳转）；**13 个有 command 面的客户端模块 I-P 全部齐备（CORE/SYS 除外）**；全部零新增注册项，ADUS 计数不变（15/60/22/12/23），audit 发现项均为「无代码」预期空 |
| 2026-06-12 | v1.29 | 补前端交互收口空白（回应 Q3）：A-SYS-M-D-01 新增 REQ-SYS-006（前端交互架构与一致性）；A-SYS-C-T-02 前端交互架构与流程总图建成（视图层 SSOT）——应用外壳两阶段（**欢迎页/主工作台**，投影 workspace 活动态）、界面分区→模块归属映射、四条核心流程总图（启动欢迎页/写作保存含冲突/对话→diff→裁决→落盘/docx 开编预览存）、跨模块界面协作点 5 项；注册约束 X-FORBID-007（前端状态投影、不另立权威，防与后端状态机漂移）；ADUS 约束 12→13；audit 发现项均为「无代码」预期空 |
| 2026-06-12 | v1.30 | 进入 L3：A-CORE-X-L-02 总开发实施计划建成（程序级，与本路线图分工——L-01 排顺序/列文档，L-02 排实施/定验收）；P0–P5 逐阶段范围/前置/产出/可核验验收标准/门禁映射（对齐 APC §5 G1–G6、M0–M5）；横切验收 6 项（audit 硬阻断/@GOV 追溯/状态机矩阵/covers/前端投影/契约一致）；关键路径与风险 5 项；§6 待人类动作（逐模块宣布 G1/G2 解锁 G3、云端立项解锁 P5、挂起决策、NFR 标定）；纯计划不放注册块，ADUS 计数不变 |
| 2026-06-12 | v1.31 | G2 闭合前债务分类盘点（回应人类「齐备≠闭合」）：A-CORE-X-R-01 门禁闭合债务分类台账建成——全模块 ~70 项 G2 后债务分 A/B/C（A ~52 实现期 G3 兜底 / B 6 设计缺口 / C ~12 待决策）；核心发现：B 类真缺口仅 4 项且全集中在 AG 模块跨模块对接（提示词组装/记忆写入触发/接管 AG 侧建模/跨文档产出边界），余 12 模块无 B 类；判定唯 AG 需补 4 项 + 回填 2 项留痕滞后后，13 客户端模块即具备 G2 闭合条件；纯报告不放注册块，ADUS 计数不变 |
| 2026-06-12 | v1.32 | **人类宣布全部客户端模块 G1/G2 闭合，正式进入 G3 开发计划**：AG 4 项 B 类（提示词组装/记忆写入触发/接管 AG 侧建模/跨文档边界）经人类决定降级为 G3 强制承接（留痕 A-CORE-X-L-02 §1 + A-CORE-X-R-01 §7/§8），G4 实现前补齐（B-4 DE 复核）；其余 12 模块无 B 类直接闭合；ACCT/ENT 客户端侧闭合（C 类随云端立项解）；G3 首件 A-SYS-X-L-01 系统底座开发计划建成（5 条 Plan Item：IPC 信封/共享类型/workspace.db 迁移/会话门禁状态机/前端外壳含欢迎页，确立 Plan Item 结构）；纯计划不放注册块，ADUS 计数不变 |
| 2026-06-12 | v1.33 | G3 续（P2 编码核心计划）：A-WS-X-L-01（5 PI：生命周期与 IO 边界/save 含外部变更冲突/diff 落盘/时间轴/控制态）、A-ED-X-L-01（5 PI：运行时与会话含 Conflicted/blockId/md·docx 统一模型/分页媒体/装饰层，承接 §11.1 已定选型）、A-DOC-X-L-01（3 PI：编辑链开存机/样式子集 ≥95% 回归/预览链）建成；各 PI 承接链路+规则+约束+状态机，明确文件范围/@GOV 落点/A 类债务/强制验收（状态机矩阵）；A 类债务正式落 Plan Item 接管；纯计划不放注册块，ADUS 计数不变 |
| 2026-06-12 | v1.34 | G3 续（P3 对话编辑与 Agent 计划）：A-DE-X-L-01（3 PI：生成与载荷/终态机与裁决/过期与冲突，accept 唯一触发落盘不可逆，DE-04 审阅 UI C 类留扩展位）、A-AG-X-L-01（**含门禁例外**：§2bis 单列 4 项 B 类强制承接条目 B-1~B-4 先补设计后实现+人类授权+B-4 DE 复核；6 PI：回合机与门禁前置/工具/门禁通道/辅助/记忆对接/接管对接，门禁不可绕过 G4 强制核验）、A-REF-X-L-01（2 PI：触发解析/锚校验注入，无状态机豁免维持）建成；纯计划不放注册块，ADUS 计数不变 |
| 2026-06-12 | v1.35 | G3 续（P4 上层能力计划，五件）：A-CBT-X-L-01（3 PI：构建回合机/产物 diff 化/接管对接，接管依赖 AG B-3）、A-AST-X-L-01（3 PI：存储隔离/自动写入机/注入供给，写入触发依赖 AG B-2）、A-KB-X-L-01（2 PI：入库/检索本地硬边界网络核验，无状态机豁免）、A-TPL-X-L-01（2 PI：模板实例化/工作流执行机）、A-PRV-X-L-01（2 PI：多格式预览只读/打印导出，无 Excel·PPT 编辑入口 G4 强制，无状态机豁免）建成；P4 与 AG 跨模块对接点（B-2 记忆写入、B-3 接管）在各计划标注依赖；纯计划不放注册块，ADUS 计数不变 |
| 2026-06-12 | v1.36 | **G3 全量收尾**：A-ACCT-X-L-01（2 PI：会话生命周期承接/令牌安全持有，令牌无明文落盘 G4 强制）、A-ENT-X-L-01（2 PI：门禁前置与网关通道/BYOK 代持与用量，门禁不可绕过+BYOK 零残留+无 provider 直连 G4/G5 强制安全核验，APC §13 禁区）建成，云端依赖打桩实现真联调 P5；**14 个客户端模块 G3 Plan Item 全部齐备（共 14 份计划、~35 条 PI）**——SYS+WS/ED/DOC+DE/AG/REF+CBT/AST/KB/TPL/PRV+ACCT/ENT；下一门槛=搭工程脚手架（Tauri 2 + React/TS + Rust）进 G4 编码；AG 4 项 B 类须在对应 PI G4 前补齐（人类授权）；纯计划不放注册块，ADUS 计数不变 |
| 2026-06-12 | v1.37 | AG 门禁例外闭合（人类授权补 B 类）：4 项 B 类设计缺口全部补齐——B-1 提示词/上下文组装架构（BR-AG-DATA-002：组装分层/注入顺序/chat·build·assist 三类回合骨架）、B-2 记忆写入触发（BR-AG-STATE-004：回合终态后异步触发 ast.capture 不阻塞，总图登记 AG→AST 转移）、B-3 接管 AG 侧建模（SM-AG-01 新增 takeover/fallbackIn/HandedOver，与 SM-CBT-01 双侧就绪）、B-4 跨文档产出边界（BR-AG-DATA-003：单文档回合限当前文档/跨文档升级 CBT，DE 协同校验）；ADUS 规则 60→63；audit 发现项均为「无代码」预期空（63/22/23）；CBT/AST 计划中依赖 AG B-3/B-2 的对接点同步就绪 |
| 2026-06-22 | v1.38 | **工程脚手架搭建并验证（文档→代码首步）**：Tauri 2 + React 18/TS + Rust 骨架就位——前端（vite/react/Service 层 X-SEQ-002/Store 投影层 X-FORBID-007/欢迎页·工作台外壳 PI-SYS-05）tsc 类型检查 PASS + vite build PASS；后端（Cargo/tauri.conf/capabilities + 16 模块骨架按 PI 文件范围 ipc/types/db/session/workspace/doc/de/ag/reference/cbt/ast/kb/tpl/prv/acct/ent）cargo check PASS（Tauri 2.11，数百 crate 编译通过）；保留 governance 脚本，audit 仍仅「无代码」预期空（@GOV 块待 G4 实现落入）；工具链 Node 23/Rust 1.91；下一步=按 Plan Item 进 G4 编码（核心链优先，AG B 类已补可直接实现） |
| 2026-06-22 | v1.39 | **进入 G4 编码（首个 PI 完整落地）**：PI-SYS-02 跨模块共享类型实现（src-tauri/src/types/mod.rs）——WorkspaceRef/DocRef/DiffStatus/PendingDiff/TimelineEntry/RefInjection/MemoryInjection/KnowledgeInjection（serde 派生），带真实 @GOV 块（codes/type=DATA/chain=DATA-MIGRATE/rules=BR-SYS-DATA-002/boundary 业务实体）+ 2 个 covers 测试（枚举集合单一来源/serde 往返）；验证：cargo test PASS、BR-SYS-DATA-002 OWNER_MISSING 消解、@GOV 五字段齐无结构性硬错误、DATA-MIGRATE 链路入口已标注；VI/UI 本批后端无涉（留前端组件阶段 PI-SYS-05/PI-ED）；整体 audit 仍因其余规则「注册无代码」预期空（随各 PI 实现逐步消解）|
| 2026-06-22 | v1.40 | G4 续：PI-SYS-01 IPC 信封/统一错误形状/command 注册实现（src-tauri/src/ipc/mod.rs）——IpcError（code/message/retriable/reauth_required，OBS 域）+ CommandRequest/Response 信封 + StreamEvent 三终态枚举（chunk/completed/interrupted/error，STATE 域 BR-SYS-STATE-002）+ CommandRegistry 未注册拒绝（API 类型 BR-SYS-DATA-003）；3 个 @GOV 块（codes/type=DATA·DATA·API/chain=IPC-CALL/rules）+ 3 covers 测试；验证：cargo test 5 passed（含 PI-SYS-02）、OWNER_MISSING 63→59（消解 BR-SYS-DATA-003/OBS-002/STATE-002）、IPC-CALL 链路入口已标注、@GOV 五字段齐无结构性硬错误；SYS 5 PI 已完成 2（01/02），剩 03 db迁移/04 会话门禁状态机/05 前端外壳 |
| 2026-06-22 | v1.41 | G4 续：PI-SYS-03 workspace.db schema 与版本化迁移实现（src-tauri/src/db/mod.rs，rusqlite bundled）——schema_meta + 基线表（workspaces/pending_diffs/timeline 含索引/UNIQUE）+ 七张控制态扩展表；migrate() 单调递增驱动、单事务、幂等；@GOV BR-SYS-PERSIST-001（EFFECT/DATA-MIGRATE）+ 2 covers（迁移幂等/重启可恢复——写入→关闭→重开数据完整）；验证 cargo test 7 passed、OWNER_MISSING 59→58、无结构性硬错误；G4 累计 5 条规则已落 @GOV+covers（DATA-002/003·OBS-002·STATE-002·PERSIST-001，63→58）；SYS 5 PI 完成 3（01/02/03），剩 04 会话门禁状态机(APC §6 强制转移矩阵)/05 前端外壳 |
| 2026-06-22 | v1.42 | G4 续（SYS 后端底座完成）：PI-SYS-04 会话与订阅门禁状态机 SM-SYS-01 实现（src-tauri/src/session/mod.rs）——SessionState 8 态/SessionEvent 11 事件/transition 引擎（forbidden 由无对应转移天然保证）/can_invoke 门禁守卫；2 个 @GOV 块（transition RB CLOUD-AUTH→BR-SYS-STATE-001；can_invoke GUARD CLOUD-GATE/INVOKE→BR-SYS-VERIFY-001+DATA-001）+ 3 covers（含 APC §6 强制转移矩阵：无 gateAllow 不可达 Invoking、GateDenied 终态不可绕过、仅 GatePassed 授权调用）；验证 cargo test 10 passed、OWNER_MISSING 58→55、三云链入口已标注、BOUNDARY_ABSTRACT passed；BR-SYS-OBS-001/DATA-004 主责留网关实现（PI-ENT）；**SYS 后端底座 G4 完成（PI-SYS-01/02/03/04），8 条规则已落 @GOV+covers（63→55）**；剩 PI-SYS-05 前端外壳（涉 VI/UI，留前端阶段）|
| 2026-06-22 | v1.43 | G4 续（进入 WS 后端，编辑底座起步）：PI-WS-01 路径边界守护实现（src-tauri/src/workspace/mod.rs）——resolve_in_workspace 逐组件解析（普通入栈/. 跳过/.. 出栈栈空即越界/拒绝绝对路径与盘符）+ resolve_doc（DocRef 定位）；@GOV BR-WS-DATA-001（GUARD/WS-FS）+ 3 covers（合法路径解析/越界(.. 逃逸·绝对路径)拒绝回统一错误形状/DocRef 定位守护，落实 X-FORBID-004）；验证 cargo test 13 passed、OWNER_MISSING 55→54、WS-FS 链路入口已标注、BOUNDARY_ABSTRACT passed；G4 累计 9 条规则已落 @GOV+covers（63→54）；生命周期 CRUD（open/switch/close）依赖 db 落 PI-WS-05 |
| 2026-06-22 | v1.44 | G4 续：PI-WS-05 workspace 注册与生命周期持久化实现（src-tauri/src/workspace/lifecycle.rs）——open/switch/close/active_workspace/list_workspaces 基于 PI-SYS-03 workspaces 表；单活动不变量（set_unique_active 清其余 is_active）+ 关闭保留控制态 + active_workspace 重启恢复入口；5 个 @GOV 块（EFFECT×3 open/switch/close + QUERY×2 active/list，均 BR-WS-PERSIST-002/DATA-MIGRATE，consumer/enforcer 语义）+ 2 covers（单活动不变量/workspace 重启恢复）；验证 cargo test 15 passed、OWNER_MISSING 54→53、BOUNDARY_ABSTRACT passed、block_unannotated 0；G4 累计 10 条规则已落 @GOV+covers（63→53）；workspace 生命周期补全（与 PI-WS-01 边界守护配套）|
| 2026-06-22 | v1.45 | G4 续：PI-WS-02 save 机与外部变更冲突保护实现（src-tauri/src/workspace/save.rs）——save_atomic（写临时文件+rename 原子落盘，无半落盘）+ check_baseline（内容哈希基线校验，磁盘≠基线即 WS_SAVE_CONFLICT 不静默覆盖，override 须显式）+ SM-WS-01 transition（Idle/Dirty/Saving/Saved/SaveError 含 saveConflict/overrideConfirmed/retry）；3 @GOV（save_atomic EFFECT BR-WS-PERSIST-001 / check_baseline GUARD BR-WS-OBS-001 / transition RB enforcer）+ 4 covers（原子往返/外部变更冲突→override/SM-WS-01 强制矩阵含 forbidden）；过程修真实缺陷：gen_workspace_id 毫秒时间戳同毫秒碰撞 UNIQUE→加原子计数器（根因修复非绕过）；验证 cargo test 18 passed、OWNER_MISSING 53→51、WS-SAVE 入口已标注；G4 累计 12 条规则已落 @GOV+covers（63→51）；WS 5 PI 完成 3（01/02/05），剩 03 diff落盘/04 时间轴 |
| 2026-06-22 | v1.46 | G4 续：PI-WS-04 时间轴存储与还原实现（src-tauri/src/workspace/timeline.rs）——append_node（version_seq 同文档 max+1 单调、UNIQUE 防复用不可伪造历史）+ list_timeline（按 seq 有序）+ SM-WS-03 transition（Browsing/Previewing/Restoring/Restored/RestoreError）；3 @GOV（append EFFECT/list QUERY→BR-WS-DATA-002，transition RB→BR-WS-STATE-002，均 WS-TIMELINE）+ 2 covers（单调序+UNIQUE 拒复用/SM-WS-03 矩阵含还原失败保持现状+快照缺失 guard+forbidden）；验证 cargo test 20 passed、OWNER_MISSING 51→49；G4 累计 14 条规则已落 @GOV+covers（63→49）；**WS 5 PI 完成 4（01/02/04/05），仅剩 03 diff 落盘（接 DE）** |
| 2026-06-22 | v1.47 | G4 续（WS 编辑底座 5 PI 全闭合）：PI-WS-03 pending_diffs 持久化与裁决后落盘实现（src-tauri/src/workspace/diff_apply.rs）——SM-WS-02 transition（AwaitingDecision/Applying/Applied/Discarded/ApplyError）+ persist_diff（入库重启可恢复）+ apply_diff（accepted→save_atomic 落盘+append_node 记恰好一个节点+status=accepted，复用 PI-WS-02/04）+ discard_diff（rejected/expired 不写文件不记节点）；4 @GOV（transition RB + persist/apply/discard EFFECT，均 BR-WS-STATE-001/WS-DIFF-APPLY）+ 3 covers（SM-WS-02 矩阵含 forbidden/accepted 落盘记一个节点/rejected·expired 不写）；验证 cargo test 23 passed、OWNER_MISSING 49→48、WS 7 条规则全部实现；**WS 编辑底座 5 PI 全闭合（01 边界/02 save 冲突/03 diff 落盘/04 时间轴/05 生命周期）**；G4 累计 15 条规则已落 @GOV+covers（63→48，SYS 8+WS 7） |
| 2026-06-22 | v1.48 | G4 续：PI-DE-02 diff 终态机 SM-DE-01 实现（src-tauri/src/de/mod.rs）——transition（Generating/Pending/Accepted/Rejected/Expired/GenFailed）+ is_terminal；@GOV BR-DE-STATE-001（RB/DE-DIFF）+ 2 covers（生命周期+accepted 唯一通往落盘/三终态互斥不可逆+未呈现不得裁决+Accepted 不回退）；验证 cargo test 25 passed、OWNER_MISSING 48→47、DE-DIFF 入口已标注；**核心交互状态机两端齐备**——DE 终态机(SM-DE-01 accepted)↔WS 落盘机(SM-WS-02)可配对，X-INTER-001 双侧代码就位；G4 累计 16 条规则已落 @GOV+covers（63→47）；DE 剩 PI-DE-01 生成/载荷、PI-DE-03 过期冲突 |
| 2026-06-22 | v1.49 | G4 续：PI-DE-03 过期保守判定与冲突重检实现（src-tauri/src/de/expiry.rs）——check_expiry（外部变更/超时→Expire；检测不可得 Undetectable 不误杀保持 Pending，超时独立生效）+ recheck_overlap（先序落盘后重叠且 before_snapshot 哈希不匹配→Expire 不覆盖新内容）；2 @GOV（check_expiry GUARD BR-DE-STATE-002 / recheck_overlap GUARD BR-DE-DATA-003，均 DE-DIFF）+ 2 covers（保守过期含 Undetectable/重叠重检防陈旧覆盖）；验证 cargo test 27 passed、OWNER_MISSING 47→45；G4 累计 18 条规则已落 @GOV+covers（63→45）；DE 剩 PI-DE-01 生成/载荷（依赖 ED locate_block）|
| 2026-06-22 | v1.50 | G4 续：AST 本地记忆簇——PI-AST-01 store.rs（workspace 级隔离查询：list_active/list_for_management 恒以 workspace_id 作用域、跨 workspace 不可见、纯本地不上云，2 QUERY @GOV BR-AST-DATA-001）+ PI-AST-02 capture.rs（自动写入机 SM-AST-01：transition RB + capture_memory EFFECT 无类型/无来源拒绝 + revoke_memory EFFECT 撤销不可复活，3 @GOV BR-AST-STATE-001）；5 covers（SM 矩阵/隔离/active 排除 revoked/撤销幂等不可复活）；验证 cargo test 32 passed、OWNER_MISSING 45→43；G4 累计 20 条规则已落 @GOV+covers（63→43）；AST 剩 PI-AST-03 供给（依赖 AG 消费侧）|
| 2026-06-22 | v1.51 | G4 续：KB 知识库（本地，全 2 规则闭合）——PI-KB-01 store.rs（主动入库 ingest：缺 source_ref/空内容拒绝=AI 不静默建库、每条可溯源；list_for_management/delete_entry 恒以 workspace_id 作用域、删除即生效不跨 workspace 误删，1 EFFECT+1 QUERY+1 EFFECT @GOV BR-KB-DATA-001）+ PI-KB-02 retrieve.rs（本地关键词检索无内容上行、仅当前 workspace 存续条目、空查询/无匹配空集不注入、命中含 source_ref 可溯源，1 QUERY @GOV BR-KB-DATA-002）；5 covers（入库拒绝/隔离/删除即生效/本地检索空集/删除即不召回）；验证 cargo test 37 passed、OWNER_MISSING 43→41；G4 累计 22 条规则已落（63→41）；KB v1 状态机豁免成立 |
| 2026-06-22 | v1.52 | G4 续：TPL 模板/工作流（全 2 规则闭合）——PI-TPL-01 template.rs（create_template/list_templates 恒以 workspace_id 隔离；instantiate_to_path 原子实例化：跨 workspace 模板拒绝、渲染 {{key}} 后经 WS save_atomic 临时文件+rename 原子写盘、模板缺失/写盘失败 Err 无半成品，2 EFFECT+1 QUERY @GOV BR-TPL-DATA-001）+ PI-TPL-02 workflow.rs（SM-TPL-01 执行机 transition：确认点强制/stepFail→Paused 不静默跳步/skipConfirmed 显式留痕/abort→Aborted 无半成品；save_run+recover_runs 重启无悬挂仅恢复非终态 run，1 RB+1 EFFECT+1 QUERY @GOV BR-TPL-STATE-001）；6 covers（隔离/原子实例化/失败无半成品/确认点强制/失败不跳步/无悬挂恢复）；验证 cargo test 43 passed、OWNER_MISSING 41→39；G4 累计 24 条规则已落（63→39）|
| 2026-06-22 | v1.53 | G4 续：CBT 构建（全 4 规则闭合）——PI-CBT-01 build.rs（SM-CBT-01 transition：计划确认强制/taskFail→TaskPaused 不静默跳步/任意非终态 interrupt→Interrupted 无半成品/不绕过裁决；save_run+recover_runs 控制态无悬挂仅恢复非终态，1 RB BR-CBT-STATE-001 + 1 EFFECT+1 QUERY BR-CBT-PERSIST-001）+ PI-CBT-02 apply.rs（build_proposal/build_proposal_sequence：产物只构造 ChangeProposal 交 DE 无旁路写盘、新建文档 before_snapshot=None 空基准、按任务序支持按条裁决，2 DATA @GOV BR-CBT-DATA-001/X-FORBID-006）+ PI-CBT-03 takeover.rs（接管须用户确认+对侧 ack 双向回执转移、active_owner 全函数结构性证明无双活双死，1 GUARD+1 RB @GOV BR-CBT-STATE-002/X-INTER-001，AG 侧对接待 PI-AG-06）；7 covers；验证 cargo test 50 passed、OWNER_MISSING 39→35；G4 累计 28 条规则已落（63→35）|
| 2026-06-22 | v1.54 | G4 续：AG 对话代理中枢（全 9 规则闭合，6 PI）——turn.rs（SM-AG-01 transition：Streaming 仅经 AwaitingGate+gatePassed 可达=门禁前置结构性无绕过、gateDenied/cancel 终态无半落盘、takeover→HandedOver 须确认、fallbackIn 起新回合；ensure_gate_passed 门禁守卫统一错误；persist_turn/recover_turns 本地可恢复不上云，BR-AG-STATE-001 RB + VERIFY-001 GUARD + PERSIST-001 EFFECT/QUERY）、context.rs（assemble_context 显式@mention永不裁剪+裁剪可追溯 DATA-001、assemble_prompt 系统→工具→上下文→用户分层+三类回合差异 DATA-002、guard_single_doc_target 跨文档升级CBT DATA-003）、tool.rs（SM-AG-02 校验/回注/上限/cancel + validate_tool 未注册/越权/写意图禁直写 BR-AG-STATE-002/X-FORBID-006）、assist.rs（SM-AG-03 节流/门禁可用方请求/失败静默/采纳即编辑/锚失效复位 BR-AG-STATE-003）、memory_link.rs（on_turn_terminal 仅合格终态+候选触发传 origin_turn、try_capture 失败隔离不影响回合 BR-AG-STATE-004）；14 covers；验证 cargo test 64 passed、OWNER_MISSING 35→26；G4 累计 37 条规则已落（63→26）；PI-AG-03/06 由 turn.rs 承接（门禁通道/接管）|
| 2026-06-22 | v1.55 | G4 续：ENT 订阅门控（全 4 规则闭合）——PI-ENT-01 gateway.rs（transition 调用通道 SM：Streaming 仅经 GateSubmitting+gatePassed 可达=门禁前置、gateDenied/offline→GateDenied 断网等同拒绝、流式三终态 streamDone/cancel/streamError，RB BR-ENT-STATE-001；submit_for_verdict 门禁守卫：Pass 仅源服务端 verdict 无本地假通过/无令牌拒绝/断网 Deny，GUARD BR-ENT-VERIFY-001/X-FORBID-002）+ PI-ENT-02 byok.rs（upload_byok 按值消费 key+就地 zeroize 零残留、gate 非 Pass 拒绝不豁免门禁、结果不回显明文，EFFECT BR-ENT-DATA-001）+ usage.rs（usage_admissible_for 用量仅 Display、GateDecision 一律 false 不据本地缓存/用量、DESKTOP_INLINE_PAYMENT=false 跳门户，GUARD BR-ENT-DATA-002）；6 covers；验证 cargo test 70 passed、OWNER_MISSING 26→22；G4 累计 41 条规则已落（63→22）；闭合客户端门禁/网关契约（AG VERIFY-001 委托侧）|
| 2026-06-22 | v1.56 | G4 续：ACCT 账号会话（全 2 规则闭合）——PI-ACCT-01 session.rs（apply 委托 SYS session::transition 承接 SM-SYS-01 会话段不重定义 RB；must_clear_token 转未认证须清令牌无残留 / reauth_action reauth_required 可靠触发优先于 retriable 不静默重试 / should_prompt_auth 断网不打扰不反复弹，3 GUARD，BR-ACCT-STATE-001）+ PI-ACCT-02 token.rs（SessionToken newtype + 脱敏 Debug 不回显；TokenVault store/clear/renew 安全持有零残留、续期失败清旧令牌转未认证、与 provider 密钥严格区分无 provider key API，3 EFFECT，BR-ACCT-DATA-001/X-FORBID-001）；5 covers；验证 cargo test 75 passed、OWNER_MISSING 22→20；G4 累计 43 条规则已落（63→20）；账户会话↔ENT 令牌门禁配套 |
| 2026-06-22 | v1.57 | G4 续：REF 引用（全 3 闭合）+ DE-01 收尾（DE 模块全 5 闭合）+ 架构归位——types/ 新增共享 LocateResult（ed.locate_block 回执，REF/DE 共用，Undetectable 保守）；DE 单一拥有 ChangeProposal（自 cbt/apply.rs 移入 de/generate.rs，cbt 改 import，单一真源）。de/generate.rs：build_payload 统一 DiffPayload 形状（新建 before_snapshot=None 空基准 DATA-002）+ generate_diff（修改既有须锚 Located，AnchorLost/Undetectable→DE_ANCHOR_LOST GenFailed 回报不产悬空 diff，新建跳过定位 DATA-001）。reference/resolve.rs：resolve_mention @mention→ReferenceAnchor（路径级/块级），经 WS 限 workspace 越界拒绝、不可达 REF_UNREACHABLE 不悬空（DATA-001）。reference/inject.rs：verify_anchor 路径存在/块级 Located 方过、失效及 Undetectable 保守→REF_ANCHOR_INVALID 不静默注入（VERIFY-001）+ build_injection 读取时点内容+origin_label 可溯源 RefInjection 不扩展字段（DATA-002）。12 covers；cargo test 81 passed、OWNER_MISSING 20→15；G4 累计 48 条规则已落（63→15）；**后端 cargo 可验证规则全部落地**，剩 15 条为 ED(6)/DOC(4)/PRV(2)/其余前端·外部工具阶段 |
| 2026-06-22 | v1.58 | G4 续：AST-03 收尾（AST 全 3 闭合）+ ENT 补 BR-SYS-OBS-001——supply.rs（仅 active+workspace 隔离/类型分级优先 preference·style>其它>session_context/origin_turn 可溯源/空集不注入，QUERY BR-AST-DATA-002）；ent/gateway.rs 新增 cloud_error 统一云错误形状（认证/门禁/网关三类失败统一 码+消息+可重试+需重认证，DATA BR-SYS-OBS-001，submit_for_verdict 错误改经此）；3 covers；cargo test 84 passed、OWNER_MISSING 15→13。**客户端后端可验证规则全部落地 50/63**。剩 13：ED 6（TipTap 前端）/DOC 4（Pandoc·LibreOffice 外部工具）/PRV 2（前端预览+导出）属前端·外部工具阶段；BR-SYS-DATA-004（网关内容不留存）为云端 GW 服务端责任（云端独立 APC owner），客户端无可落 @GOV，OWNER_MISSING 属正确（详见 A-CORE-X-R-01 §C 类） |
| 2026-06-24 | v1.59 | 实现期：DOC 双链 + docx 编辑全链落地（文档·设计·代码同步，移植旧版 binder-project 跑通链路）。后端：style_subset/convert(SM-DOC-01)/block_backfill + pandoc_runtime(docx↔md) + edit_chain(docx↔HTML：docx+styles+preserve-styles.lua+native_divs/spans / html→docx --reference-doc) + libreoffice_runtime(docx→PDF 预览，优化版 LO 内置打包)；命令 doc.open_for_edit/save_from_edit/create_draft（lib.rs）。前端：docService + Editor/edStore/edService docx HTML 载入保存 + FileTreeNode 路由 + PrvPanel「创建草稿进行编辑」+ 富文本扩展集（TextStyle/Color/TextAlign/FontFamily/FontSize/Underline/Table*/Image/Highlight/Sub·Superscript，解析样式子集为成熟文本块）。成熟流程：原生 docx→预览→建草稿(*.draft.docx)→HTML 编辑→存回 docx；草稿/生成 docx→直编辑。部署：scripts/deploy_lo.sh + bundle.resources（优化版 LO ~687M 减 30%，签名/公证 G6 阻断 BLOCK-REL-LO-001 A-CORE-X-R-01 §9）。文档同步：A-DOC-M-T-01 v1.2（§2.1 编辑链处理逻辑/§2.2 编辑流程草稿/§4.1 扩展↔子集映射/TERM-DOC-002 EditDraft）、A-DOC-I-P-01 v1.1~v1.3。集成测试 tests/core_chain.rs（AG→DE→WS→生命周期端到端）。covers 审计扩展扫描 src-tauri Rust 内联。验证：cargo test 114、tsc 0、vitest 63、build ✓。术语注册表 +TERM-DOC-002、规则链映射经 generate 刷新（规则 65/链路 22，DOC 规则链稳定无新增） |
| 2026-07-04 | v1.60 | 术语统一清扫（承 2026-07-04 人类三轮裁决）：正文禁用别名替换——内联辅助→局部修改、自动补全→辅助续写、对话编辑→Diff Edit、Chat↔Build 接管→构建移交/回落、chat/chat-agent 回合→代理回合；文件名与变更记录历史行不改写 |
| 2026-07-17 | v1.61 | 新增 1.0 Experience Preview 发布切片：完整 v1 IN/OUT 不变，全功能开发不暂停；§2.1 定义 1.0 EP 发布阻断项与非阻断项；§5 插入 M2.5 体验版里程碑；§6.2 登记 A-CORE-X-L-06；§10 后续动作明确 1.0 EP 只作为发布切片入口，不作为模块规则源 |
