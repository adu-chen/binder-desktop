---
文档编号：CORE-C-P-01
文档状态：A
负责模块：CORE
文档职责：Binder 项目 AI 开发宪章，定义项目身份、模块化架构与 AI 执行边界
上游约束：ADU.md、ADP.md、CLAUDE.md
直接承接：全部项目文档与实现模块
使用边界：定义项目治理边界与门禁，不替代具体设计文档；不定义治理规则细节（见 ADU）、不定义开发过程细节（见 ADP）
变更要求：修改 APC 必须同步检查治理配置、ADUS 路径、模块清单与文档体系
治理成熟度：draft
---

# Binder AI 开发宪章（APC）

## §1 项目身份

Binder 是一款 AI 原生的桌面写作应用（AI writing），目标商业化上线运营，采用 Open Core
商业模式（当前不开源）。首发 macOS，经官网门户以 DMG 分发，不走 Mac App Store。

- 形态：Tauri 2 + React + TypeScript（前端）+ Rust（后端）桌面客户端。
- 本地优先：用户内容（文档、记忆、模板、聊天历史、知识库、引用）全部本地存储；
  workspace 是用户选定的本地目录，是文件操作、搜索、diff、上下文读取的边界。
- 云端职责（薄）：账号/认证、订阅/计费、AI 网关；不承载用户内容，不做内容云同步。
- 核心交互：用户通过对话让 AI 生成文档修改 diff，用户接受/拒绝后才写入文件；
  并支持 agent 构建多文件文档项目（CBT）。
- 参考来源：旧 binder（/Users/chenzhenqiang/Desktop/test/binder-project）与后续调研文档
  仅作 L4 参考原料，不作为本项目权威源（见 §3、§8）。
- v1 范围确认（2026-06-09 人类决策）：文档级时间轴/历史回溯（归 WS）与音视频预览（归 PRV）
  纳入 v1；Excel/PPT 仅预览不编辑。
- 1.0 Experience Preview 发布切片（2026-07-17 人类决策）：完整功能地图不变，全功能开发不暂停；
  1.0 先发布核心工作流体验版，发布阻断项与非阻断项见
  `A-CORE-X-L-06_1.0体验版发布范围与验收计划.md`。构建、记忆协同、知识库、模板/工作流、
  自部署 Binder 预设模型、多模态模型矩阵等能力不作为 1.0 EP 上线阻断项，但仍在完整路线内继续推进。

明确不做：多人实时协同、内容云同步、插件市场、脱离写作场景的通用聊天助手、Excel/PPT 编辑。
（代理编辑〔AgentEdit，旧称 chat-agent/对话编辑〕是写作域内能力，保留。）

## §2 开发方法论

项目采用文档先行、规则驱动、状态机驱动开发，锚定成熟产品上线运营。

四级文档门禁基础上扩展为六级（详见 §5）：
G1 需求 → G2 技术设计 → G3 开发计划 → G4 实现验证 → G5 集成/系统验证 → G6 发布就绪。

未通过前一门禁不得进入后一阶段。AI 可建议晋级，不得自行宣布晋级；人类越级要求时，
AI 必须执行 ADP Phase Gate Check，门禁不满足触发 ADP SC-06 PHASE_GATE_BLOCKED 或
SC-07 MAPPING_CHAIN_BROKEN，不得进入代码修改。

技术设计文档是代码实现的最终规则来源。完整规则链条：
REQ -> REQ_MAP -> RULE / CHAIN / CONSTRAINT / TERM -> STATE_MACHINE（适用时） ->
Plan Item -> Issue Trace -> @GOV -> Test covers -> Task Runtime Report。

设计与代码冲突时设计优先；不得为错误实现反向修改设计文档。

## §3 文档体系声明

L0 治理控制文档：ADU.md、ADP.md、CLAUDE.md、
APC（docs/00_core/A-CORE-C-P-01_APC.md）、ADUS（docs/00_core/A-CORE-C-R-01_ADUS.md）。

业务文档三层职责（职责固定，文件名按主题/阶段命名，编号沿用 A-域-…-NN 体系）：

- 3.1 功能需求描述层：产品要做什么、为谁、边界。用 `<!-- REQ -->` 声明需求层规则；
  代码不得直接映射 REQ-*。
- 3.2 技术设计层：系统如何实现，是代码规则的唯一来源。RULE/CHAIN/MODULE/TERM/CONSTRAINT
  优先在此定义；需求到技术规则经 `<!-- REQ_MAP -->` 映射。
- 3.3 开发计划层：落地顺序、验证、发布。不得替代设计层作为规则来源。

参考层（L4 只读）：旧 binder 文档、调研开源项目文档等，均为原料，经台账分层后引用，
不拷入本项目治理；本项目 REQ 重新撰写，不继承旧文档为权威。

ADUS 仅由 `npm run governance:generate` 生成，禁止人工编辑自动生成区。

## §4 模块化架构与模块清单

### 4.1 桌面客户端模块（本 APC 治理，src/ + src-tauri/src/）

| module_code | 职责 |
|---|---|
| CORE | 项目核心与治理 |
| SYS  | 整体架构、IPC 协议、跨模块契约 |
| WS   | workspace 生命周期、文件 I/O、缓存、pending_diffs 持久化、文档级时间轴/历史回溯 |
| ED   | 编辑器运行时（TipTap，md/docx 富文本编辑） |
| DE   | Diff Edit（2026-07-04 定名，无中文名；旧称「对话编辑」退役）：diff 生成、审阅、接受/拒绝/过期终态 |
| AG   | 代理（Agent）：代理编辑面板（编辑/构建双模式）回合基建、工具调用回合、辅助续写与局部修改 |
| CBT  | 构建（Build，旧称 Chat Build 弃用）：项目蓝图与多文件文档项目构建、构建中断、构建移交/回落 |
| DOC  | DOCX/Pandoc 转换 |
| PRV  | 预览与输出：Excel/PPT 预览、音视频预览 + 打印/导出 |
| REF  | 引用（@mention）解析与注入 |
| AST  | 记忆协同（本地） |
| KB   | 知识库（本地） |
| TPL  | 模板/工作流 |
| ACCT | 账号、认证、会话（云端认证客户端） |
| ENT  | 订阅门控、用量、AI 网关客户端 |

### 4.2 云端模块（独立 APC 实现，本 APC 仅定义接口契约）

| module_code | 职责 |
|---|---|
| AUTH | 认证、用户管理 |
| GW   | AI 网关：服务端持密钥、统一代理、用量计量 |
| LIC  | 订阅、计费、用量额度 |

### 4.3 云接口契约（客户端依赖，必须在技术设计层定义）

- 认证契约：登录、令牌、会话续期。
- 订阅门禁契约：调用模型前校验有效订阅（硬门）；无有效订阅一律拒绝模型调用。
- AI 网关契约：套餐档经网关计量计费；BYOK 经网关透传、不计额度、仍需有效订阅；
  客户端不内置 provider SDK，不持密钥。

## §5 阶段门禁与完成标准

G1–G4 沿用四级门禁语义（需求暴露清单、技术设计 SSOT、计划条目、实现真实性验证）。
门禁完成标准与问题分类沿用项目既定模板，本节补充 G1–G4 的关键要点与新增 G5、G6。

- G1 需求：每功能有稳定 REQ-* + 流程图（见 §7）+ 目标/入口/输入输出/主流程/分支/异常/终态；
  边界闭合；问题暴露清单（REQ_GAP/BOUNDARY_OPEN/REQ_CONFLICT/FLOW_INCOMPLETE/
  ACCEPTANCE_MISSING/NEEDS_HUMAN_DECISION/DESIGN_RISK）未关闭或未显式延期不得通过。
- G2 技术设计：每功能承接 REQ-*、映射 RULE/CHAIN/CONSTRAINT/TERM、定义数据/接口/Tauri
  command/状态机/异常恢复/测试方案/验收标准/债务/盲区。设计是唯一规则来源。
- G3 开发计划：每条 Plan Item 承接 REQ/RULE/CHAIN/CONSTRAINT/状态机，明确文件范围、
  测试文件、依赖、验收、风险债务。
- G4 实现验证：负空间扫描内代码块有 @GOV 或 triage；@GOV rules 引用已注册规则（不引用
  REQ-*）；boundary 描述真实 I/O 副作用；covers 有实际断言；状态机有转移测试；
  `governance:audit` 通过；Task Runtime Report 说明规则链完整。
  真实性缺口分类：ANNOTATION_ONLY/RULE_BEHAVIOR_MISSING/BOUNDARY_FALSE/
  TEST_ASSERTION_WEAK/STATE_MACHINE_DRIFT/MAPPING_DECORATIVE。

### 5.1 G5 集成/系统验证门禁（新增）

跨模块链路联调通过后，才允许进入发布就绪。完成标准：
- 核心链路端到端联调通过（如：对话 → diff → 接受 → 落盘；CBT 多文件构建；
  AI 调用 → 订阅门禁 → 网关 → 流式回显）。
- 回归基线建立并通过；关键路径 e2e 冒烟通过。
- §9 NFR 关键指标在本阶段标定并达标（性能预算、恢复、安全门）。
- 跨模块约束（CONSTRAINT）全部验证。

### 5.2 G6 发布就绪门禁（新增）

发布前必须通过。完成标准：
- 版本号符合 semver；版本说明、验收记录齐备。
- DMG 打包、macOS 签名与公证（notarization）完成。
- 自动更新（Tauri updater）通道验证。
- 订阅门控、账号/认证、AI 网关契约在生产配置下端到端验证。
- CI 治理审计 + 测试全绿（硬门）。
- 成熟度按里程碑升级（RC 升 baseline，GA 升 controlled，均需人类批准）。

## §6 状态机驱动与 SSOT

本项目为状态机驱动开发。强制阈值（口径 b）：有状态/跨层/多步的链路必须建状态机；
纯函数/纯展示豁免。以下核心链一律强制建状态机（非穷举）：
diff 终态、代理回合（旧称 chat-agent 回合）、tool-call、save 保存、docx 开/存、CBT 构建、
记忆写入、autocomplete（辅助续写）、inline-assist（局部修改）、账号会话与订阅门禁。

状态机设计必须包含：machine_id / owner_module / scope / context / initial_state / states /
terminal_states / events / transitions / guards / actions / error_states / recovery_paths /
forbidden_transitions / acceptance_criteria / test_matrix。

被声明为状态机约束的功能，其状态/事件/转移/副作用/终态以状态机为唯一真值；
代码、测试、计划、@GOV 必须承接。

## §7 功能流程图要求

每功能须有流程图或等价流程表，覆盖：用户入口、前置条件、主流程、分支、异常路径、
人类确认点、模块交互点、数据读写点、阻断条件、终态。多模块功能须标模块边界
（如 UI -> Store -> Service -> Tauri Command -> Rust -> FS；UI -> AG -> Tool -> WS -> DE）。
推荐 Mermaid；否则用 Flow Table：step_id | actor | input | action | output | next_step | exception。

## §8 需求追溯与映射

REQ 在本项目重新撰写，旧 binder 与调研文档为 L4 原料。每条 active 需求经 REQ_MAP
进入技术设计规则体系，或显式标记 blocked/deferred/needs_human_decision。
代码不得承接自然语言需求，不得承接 REQ-* 作为实现规则。文档去重分层台账（独立产出）
负责把 L4 原料映射为本项目 REQ 与降级参考。

## §9 非功能需求与生产关注（NFR）

必备项（关键指标在 G5 标定并达标）：
- 性能预算：冷启动、大文档分页、流式首字延迟、diff 渲染、文件搜索。
- 安全模型：AI 提示注入防护；文件系统边界（限定 workspace）；密钥服务端持有，
  客户端不持密钥；订阅门禁不可被客户端绕过；用量计量防篡改。
- 可观测性：结构化日志、错误上报、关键链路埋点。
- 崩溃与数据恢复：workspace.db schema 迁移；pending_diffs 持久化与重启恢复；
  保存竞态防护；外部文件修改处理。
- 离线边界：编辑/预览/打印/本地功能离线可用；AI 功能在线鉴权 + 有效订阅。

## §10 测试策略

- 单元：纯逻辑与工具函数。
- 集成：核心链路强制集成测试；状态机强制转移测试矩阵。
- e2e：关键路径冒烟（代理编辑、构建、AI 调用门禁、保存恢复）。
- covers：`// covers: BR-*` 覆盖所有已注册 RULE，须有实际断言。
- CI：`governance:audit` + 测试为合并硬门禁（见 §11、§12）。

## §11 发布与上线节奏

- 版本：semver；通道 alpha → beta → RC → GA。
- 1.0 Experience Preview 是插入 alpha/beta 与完整 RC/GA 之间的早期发布切片：
  以核心工作流、DeepSeek 托管模型、BYOK、账号订阅、管理员端最小能力与反馈闭环为上线目标；
  不替代完整 v1 功能地图，也不作为后续模块延期或取消的依据。
- 成熟度绑定：RC 升 baseline，GA 升 controlled（均需人类批准）。
- 打包分发：macOS DMG，官网门户下载；签名 + 公证；Tauri updater 自动更新。
- 订阅：购买在官网门户完成，桌面端只做门禁校验；v1 GA 含订阅门控。
- 里程碑：每里程碑声明承接的 REQ/CHAIN、门禁状态、NFR 达标、发布检查清单。
- CI/CD：PR 必须通过治理审计与相关测试方可合入；发布前完成 G6 检查。

## §12 ADU/ADP 启用声明

全面启用 ADU/ADP/CLAUDE。治理范围：前端 src/（.ts/.tsx）+ 后端 src-tauri/src/（.rs，
脚本已扩展）。强制治理模块：§4.1 全部客户端模块；云模块经契约约束。
`governance:audit` 始终硬阻断——任何 failed 阻断 READY/合入/发布，不设报告-only 宽限期。
强制标注：每条 active 链路入口、跨模块约束必须标注；planned/superseded 豁免入口标注。
代码核心链用块注释 `/** @GOV ... */`；测试用 `// covers: BR-*`。
启用 ADU 的 REQ/REQ_MAP/STATE_MACHINE 与 ADP 的 governance_phase/Phase Gate Check/
SC-06/SC-07/Task Runtime Report 的 phase_gate / mapping_chain 字段。

## §13 AI 执行边界与工作风格

AI 不得自主决定：改变产品定位或扩大"明确不做"；修改 APC 内容或 ADU/ADP/CLAUDE；
人工编辑 ADUS 自动生成区；Stop Condition 未解除时推进；将 L4 参考文档误判为当前真值；
跳过状态机直接实现状态化核心链；以计划文档替代技术设计作规则来源；
创建无法追溯已注册规则的游离代码块；在客户端硬编码/落地密钥；绕过订阅门禁。

AI 必须暂停等待人类：模块职责不明、规则冲突、设计与代码冲突、Scope 需扩展、
缺静态规则/链路/模块/术语/状态机定义、代码块无法追溯规则、审计失败且当前 Scope 内无法修复。

AI 不得讨好性回复或执行，必须以成熟工程师视角审查人类决策，冲突时直接指出，
不弱化风险、不隐藏阻断、不伪造完成、不把设计漏洞转嫁代码。落地质量优先于表面顺从。

## §14 版本与变更管理

私有仓库（商业，非开源），git 版本管理。分支：main（稳定，过门禁版本）/ dev（集成）/
feature/* / fix/*。合并原则：影响实现的变更须可追溯技术设计规则；核心链 PR 须过治理审计
与测试；文档变更影响规则须刷新 ADUS 并检查 @GOV 引用；发布前完成 G6 检查清单。
APC 被新版取代时旧版改为 X。治理成熟度只能由人类推进。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-07-17 | v1.3 | 记录 1.0 Experience Preview 发布切片决策：完整功能地图不变，全功能开发不暂停；1.0 EP 先发布核心工作流体验版，构建/记忆协同/知识库/模板/自部署预设模型/多模态模型矩阵不作为上线阻断项；§11 增补 1.0 EP 与 alpha/beta/RC/GA 的关系 |
| 2026-06-09 | v1.0 | bootstrap 初版，经多轮人类确认落地（商业 Open Core / 桌面客户端主体 + 云契约 / 六级门禁 / 状态机驱动 / 上线节奏） |
| 2026-06-09 | v1.1 | 人类确认 v1 范围：纳入文档级时间轴/历史回溯（WS）与音视频预览（PRV），确认 Excel/PPT 仅预览不编辑；同步 §1 范围与"明确不做"、§4.1 WS/PRV 模块职责 |
| 2026-07-04 | v1.2 | 术语沿革澄清回写（人类三轮陈述+裁决，module_code 均不变）：§4.1 AG=「代理（Agent）」（代理编辑面板双模式）、DE=「Diff Edit」（无中文名，旧称「对话编辑」退役为 TERM-AG-008 禁用别名）、CBT=「构建（Build）」（弃 Chat Build）；§1/§6/§10 措辞同步（chat-agent→代理编辑/代理回合，Chat↔Build 接管→构建移交/回落） |
