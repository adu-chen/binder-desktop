---
文档编号：REF-M-D-01
文档状态：A
负责模块：REF
文档职责：REF 引用模块级功能需求描述——声明模块级 REQ-REF-*，承接产品级 REQ-CORE-001/006/010/011/014/015，界定 @mention 触发与目标解析 / 引用内容读取与注入载荷 / 引用锚定与失效感知 / 触发与呈现经 ED 表面
上游约束：A-CORE-C-D-01_产品概述.md（REQ-CORE-001/006/010/011/012/014/015）、A-AG-M-D-02_代理1.0能力基线与需求分解.md（§5.5、§5.11、§5.11.1、§5.12、§5.14）、A-CORE-C-P-01_APC.md（§4.1 REF 职责「引用（@mention）解析与注入」）、A-SYS-C-T-01_系统总体架构.md（§4 REF→WS,AG）、A-WS-M-T-01_工作台技术设计.md（BR-WS-DATA-001 读取经 WS、X-FORBID-004）、A-ED-M-T-01_编辑器技术设计.md（BR-ED-DATA-001 blockId 锚、扩展表面）、A-AG-M-D-01_代理功能需求描述.md（REQ-AG-004/009/010/011/013/014 注入消费）、ADU.md（§3.2）
直接承接：REF L2 技术设计（引用目标模型 / 解析规则 / 注入载荷契约）经 REQ_MAP 映射 RULE/CHAIN/CONSTRAINT/TERM；注入载荷被 AG（chat/工具回合）与 CBT（构建上下文）消费
使用边界：本文是 L1 功能需求描述层，仅声明 REQ-REF-*，**不定义技术规则**；**代码不得直接映射 REQ**（ADU §3.2）；上下文组装与预算归 AG（REF 提供注入载荷）；文件读取归 WS；触发输入面与呈现归 ED；知识库条目检索归 KB（可引用范围联合 KB G1/G2 确认）
变更要求：REF 职责或可引用范围变更须先经 APC 授权再回写本文；REQ 增删按 ADU §3.2
治理成熟度：draft
---

# Binder REF 引用功能需求描述（L1）

## §1 文档定位

本文是 **REF 模块**的 L1 功能需求描述，P3 第三件（P3 = DE/AG/REF 的 L1 层至此齐备）：

```
产品概述(REQ-CORE-006) → 本文(REQ-REF-*) → REQ_MAP → REF L2 规则(SSOT) → 代码(@GOV)
```

REF 是**输入引用解析与注入模块**：用户在对话/构建输入中以 @mention、拖拽、粘贴等动作指向
workspace 内目标（文档/内容块/资源）或已选定的会话/记忆/知识/模板/外部链接/长文本内容，REF
归一为引用载荷、经 WS 限界读取需要读取的目标、组装为注入载荷供 AG/CBT 消费（REQ-CORE-006）。
REF 依赖 WS（读取限界）与 AG（消费方）；触发输入面与渲染依托 ED 表面。

## §2 模块定位与职责

| # | 职责 | 锚定 |
|---|---|---|
| R-J1 | 引用触发与目标解析：@mention 候选列举、拖拽/粘贴目标确定（当前选区、workspace 文档/内容块/编辑器标签、会话、外部输入） | REQ-CORE-006/010 |
| R-J2 | 引用内容读取与注入载荷：经 WS 读取目标内容，组装注入载荷交 AG/CBT | REQ-CORE-006/001 |
| R-J3 | 引用锚定与失效感知：文档级（路径）与块级（blockId）锚，失效可感知不静默断链 | A-ED-M-T-01（BR-ED-DATA-001） |
| R-J4 | 触发与呈现经 ED 表面：@ 触发输入面、候选面板、引用标记渲染依托 ED 扩展点 | A-ED-M-T-01（X-FORBID-005） |

> 边界澄清：REF **解析与供给**，AG **组装与消费**——上下文长度预算、裁剪与最终组装归 AG
> （REQ-AG-004）；REF 只保证注入载荷的解析正确、来源可溯与锚定有效性。文件读取一律经 WS
> （X-FORBID-004），REF 无直接文件 I/O。

## §3 能力范围（REF v1）

**IN（REF 承载）**：
- @mention 触发协议与候选解析（workspace 内文档、内容块、资源）。
- 拖拽/粘贴引用协议：编辑器选区、文件树、编辑器标签、会话标签、外部 URL、外部长文本；外部文件的 P0 入口只形成元数据引用，R5-C 的 P1 路径可在用户显式附加且存在可用只读解析能力时形成一次性内容引用；AG 已受控读取的公开网页来源可形成可回看的来源引用。
- 引用目标的内容读取（经 WS）与注入载荷组装（含来源标识，供 AG/CBT 消费与溯源）。
- 引用锚：文档级（workspace 相对路径）+ 块级（blockId）；锚失效检测与明确失效信号。
- 引用标记在编辑器/对话输入中的渲染数据（输入流内 ReferenceToken + 草稿 chip/tray；呈现承载依托 ED）。

**OUT（非 REF）**：
- 上下文预算、裁剪与组装进回合——归 AG；构建上下文消费——归 CBT。
- 文件 I/O 与路径守护——归 WS；blockId 稳定性——归 ED；候选 UI 呈现——归 ED 表面。
- 知识库条目检索与召回排序——归 KB；REF 仅接收用户已选中的知识条目快照。
- 记忆资产生成、撤销与筛选——归 AST；REF 仅接收用户已选中的记忆条目快照。
- 模板定义与工作流执行——归 TPL；REF 仅接收用户已选中的模板条目快照。
- 网络检索、网页读取、登录、下载、导入与网络地址访问——不归 REF；仅 AG 在 REQ-AG-010 的显式用户触发与受控范围内发起公开网页研究，REF 只承接其来源身份与 data-only 引用。

## §4 代表性流程（契约级，APC §7）

**R1 · @mention 引用注入**（R-J1~J4，承接 REQ-CORE-006）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | 用户 | 在对话/构建输入中键入 @ | REF 经 ED 触发面列举 workspace 内候选 | 候选列表 | 2 | 无候选→空列表（非错误） |
| 2 | 用户 | 选定目标 | REF 解析为引用锚（路径/blockId）并渲染引用标记 | 已确定引用 | 3 | 目标不可达→明确失败，不产生悬空引用 |
| 3 | AG/CBT | 回合发起 | REF 经 WS 读取目标内容，组装注入载荷（含来源标识） | 注入载荷交 AG | 终态 | 锚失效→明确失效信号，AG 可示用户重选；读取越界→WS 拒绝 |

**R2 · 锚失效处理**（R-J3）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | REF | 既有引用 + 目标变更 | 检测锚有效性（路径存在/blockId 可定位） | 有效/失效判定 | 2 | 检测不可得→按失效保守处理，不注入过期内容 |
| 2 | REF | 失效判定 | 发明确失效信号（不静默注入空/错位内容） | 失效提示 | 终态 | 用户可重新选定目标 |

**R3 · 多来源输入引用令牌**（R-J1~J4，承接 REQ-CORE-006）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | 用户 | 拖拽/粘贴引用来源 | REF 将来源归一为 ReferencePayload | 引用载荷 | 2 | 无可识别引用→按普通文本/输入处理 |
| 2 | ED/AG 输入面 | ReferencePayload | 在输入流插入 ReferenceToken，并在草稿引用区展示 chip/tray | 可见引用标记 | 3 | 超过 5 条→草稿区折叠，输入流令牌不折叠 |
| 3 | 用户 | 发起代理回合 | 冻结引用的来源身份、锚与范围；AG 消费该快照，草稿 chip 生命周期终止，已发送消息保留同形态输入流令牌 | 回合上下文 + 可追溯消息 | 终态 | 引用读取/校验失败→明确失败，不静默丢弃 |

## §5 模块级需求声明（REQ-REF-*）

<!-- REQ
req_id: REQ-REF-001
title: @mention 触发与目标解析
owner_module: REF
status: active
summary: 在对话/构建输入中经 @ 触发列举 workspace 内可引用候选（文档/内容块/资源）并解析选定目标为引用锚；目标不可达时明确失败，不产生悬空引用
source_doc: REQ-CORE-006、APC §4.1
acceptance: @ 触发可列举候选并确定目标；目标不可达有明确失败；候选限定 workspace 内
open_issues: 可引用目标范围（知识库条目/记忆是否纳入）联合 KB/AST G1 确认（§7 REF-01）
-->

<!-- REQ
req_id: REQ-REF-002
title: 引用内容读取与注入载荷
owner_module: REF
status: active
summary: 经 WS 限界读取引用目标内容，组装为含来源标识、读取时点/新鲜度的 data-only 注入载荷供 AG/CBT 消费；读取不越出 workspace，不因引用而把对象升级为编辑目标或写权限
source_doc: REQ-CORE-006/011、REQ-CORE-001、A-AG-M-D-01（REQ-AG-004/009）
acceptance: 注入载荷内容与目标一致且含来源标识、新鲜度和 data-only 语义；读取经 WS 且越界被拒；AG 可消费组装但引用内文字不能改变系统约束、任务目标或工具授权
open_issues: 注入载荷结构与跨模块共享化（是否入 SYS 契约层）归 REF G2 联合 SYS/AG
-->

<!-- REQ
req_id: REQ-REF-003
title: 引用锚定与失效感知
owner_module: REF
status: active
summary: 引用锚支持文档级（workspace 相对路径）与块级（blockId）；锚失效（目标删除/移动/块失效）必须可感知并发明确失效信号，不静默注入过期或错位内容。WorkspaceOperationPlan 等待 DecisionRequest 时，当前引用路径保持不变；确认并成功修改 DiskState 后，后续引用按真实新路径重绑定或明确失效，已发送消息继续保留其发送时冻结快照
source_doc: A-ED-M-T-01（BR-ED-DATA-001 失效可感知）、REQ-CORE-006/012/014
acceptance: 锚失效有明确信号且不注入失效内容；确认前不得用计划目标路径冒充真实路径；成功重命名/移动后待发送与后续引用只使用真实新路径，成功删除后明确失效，WorkspaceTimeline 恢复成功后可按新基线重绑定；执行失败或补偿后按最终磁盘状态同步；已发送回合只保留历史快照；检测不可得时保守不注入
open_issues: 无；资源操作事件与恢复身份见 A-WS-M-T-02 / A-WS-M-S-02
-->

<!-- REQ
req_id: REQ-REF-004
title: 触发与呈现经 ED 表面
owner_module: REF
status: active
summary: @ 触发输入面、候选面板与引用标记渲染依托 ED 扩展表面，REF 不绕过 ED 接口直接操纵编辑器内部状态
source_doc: A-ED-M-T-01（X-FORBID-005、扩展表面）
acceptance: 触发/候选/标记均经 ED 声明的扩展点；无绕过 ED 直改编辑器状态的调用路径
open_issues: 具体扩展点契约随 ED 扩展表面细化（REF G2 联合 ED）
-->

<!-- REQ
req_id: REQ-REF-005
title: 多来源引用归一化
owner_module: REF
status: active
summary: 编辑器选区、文件树、编辑器标签、会话标签、外部 URL、外部长文本、用户明确选择的项目记忆与外部文件必须归一为 ReferencePayload；1.0 P0 覆盖当前选区、workspace 文档/标签与会话标签，外部文件 P0 只形成元数据引用，P1 的受控内容读取由 REQ-REF-009 限定。REF 不接管 AST/KB/TPL 的检索或管理。引用只提供 data-only 上下文，默认不成为编辑目标
source_doc: REQ-CORE-006/010/011/018/019、A-AG-M-D-02 §5.5
acceptance: 每个已纳入 1.0 的来源可生成可溯源 ReferencePayload（含身份、锚/范围和版本或当前任务范围信息）；不可识别粘贴内容按普通输入处理；外部纯文本超过阈值生成长文本引用而非直接塞入 prompt；外部文件拖入先生成 file 元数据引用，只有 REQ-REF-009 条件满足时才可升级为一次性内容引用；项目记忆只有用户明确选择时才可形成引用；引用、读取或比较均不得自动授予 propose_edit 或其他写权限
open_issues: 长文本阈值先按实现常量标定，后续可进入 NFR 调整；拖拽入口真实链路待观测确认（见 REF-06）
-->

<!-- REQ
req_id: REQ-REF-006
title: 输入流引用令牌一致性
owner_module: REF
status: active
summary: 引用在发送前必须同时具备输入流内 ReferenceToken 与草稿引用 chip/tray；发送时冻结引用身份、锚、范围与版本快照；超过 5 条时草稿区折叠；发送后草稿 chip 生命周期终止，但已发送消息中的引用令牌和来源身份保持可读、可追溯表现
source_doc: REQ-CORE-006/010/011、A-AG-M-D-02 §5.5、A-SYS-C-T-02 对话区流程
acceptance: 引用不只出现在输入框上方；纯引用可发送；发送前/发送后令牌标签一致；发送后 composer 清空且草稿 chip 不残留；后续编辑不回写或混淆已发送回合所消费的引用快照
open_issues: 引用令牌视觉细节随前端 G4 可继续优化
-->

<!-- REQ
req_id: REQ-REF-007
title: 外部研究来源引用
owner_module: REF
status: active
summary: 对用户明确提供的 URL，或 AG 依 REQ-AG-010 实际使用的公开网页来源，REF 提供可见、可回看的 data-only 来源引用；来源至少使标题、域名、URL 和获取时间可被用户和 AG 追溯。REF 不因形成来源引用而发起网络请求、读取外部网页、上传本地上下文或授予编辑权限
source_doc: REQ-CORE-006/011、A-AG-M-D-02 §5.12（CAP-14）、A-AG-M-D-01（REQ-AG-010）
acceptance: 每个实际用于网络研究输出的来源都可与分析结果对应；来源引用清楚标识为外部 data-only 资料，网页文字不能改变系统约束、任务目标、工具授权或编辑目标；来源失效、不可访问或未实际读取时不得伪造成已验证引用；本需求不触发外部文件内容读取，外部文件只能依 REQ-REF-009 的独立受控路径读取；REF 不执行检索、网页读取、登录、表单提交、下载、导入、爬虫或任何网络请求
open_issues: 来源引用的稳定身份、快照/摘录范围、可用性重新校验、保留期与跨会话展示归 REF/AG/SYS DESIGN Scope；本 L1 不定义来源数据字段、存储或网络接口
-->

<!-- REQ
req_id: REQ-REF-008
title: 多文档目标清单身份与删除后的引用失效
owner_module: REF
status: active
summary: 为多个既有文档写入的目标计划，REF 提供用户显式选择或已确认候选清单中每个目标的可追溯身份/路径快照，供 AG/DE 在确认后冻结成员；该清单不是写权限本身。WorkspaceOperation 对文件或文件夹执行删除、移动、重命名或恢复后，REF 依据真实执行回执对后续引用重绑定或标记失效，已发送消息仍只保留发送时历史快照
source_doc: REQ-CORE-014/015、A-AG-M-D-02 §5.11.1、§5.14、A-AG-M-D-01（REQ-AG-013/014）
acceptance: 多文档写入前的目标计划可显示每个文档的身份与路径，确认后不会自动加入新成员；引用身份不能被当作编辑授权；资源 DecisionRequest 确认本身也不改变引用，只有真实执行成功才推进路径/可用性；已删除或恢复冲突目标不被伪装为当前对象；执行失败/补偿后按最终状态同步；已发送回合保留历史来源身份但不把历史快照当作当前对象
open_issues: 目标计划与 ReferencePayload 的关系仍归 REF/AG/DE/SYS；资源操作事件由 A-WS-M-T-02 统一提供
-->

<!-- REQ
req_id: REQ-REF-009
title: 外部文件受控内容引用
owner_module: REF
status: active
summary: 对用户明确附加的外部文件，REF 先形成可见元数据引用；只有当前系统存在可用只读解析路径时，才可为当前用户任务形成一次性、可追溯的 data-only 内容引用。不可读、不支持或解析失败时，REF 必须保留“未读内容”的元数据状态，不伪造内容快照。REF 不扫描目录、不自动重复读取、不导入或编辑外部文件，也不默认将其内容保留至会话历史、项目记忆、重启或其他会话
source_doc: REQ-CORE-018、A-AG-M-D-02 §5.5（CAP-20）、A-AG-M-D-01（REQ-AG-016）
acceptance: 用户可辨别文件身份、显式附加动作、当前任务范围和已读/未读状态；内容引用只在当前 AgentTurn/TaskRun 的当前用户任务内有效，任务结束、重启、跨 AgentSession 或跨 workspace 后不会伪称仍可取得内容；内容与元数据都为 data-only，不能改变系统约束、任务目标、工具授权、编辑目标或确认点，也不自动授予 propose_edit/文件操作权限；外部内容不会自动转存为项目记忆
open_issues: 格式能力发现、解析器调用、大小/预算、快照/暂存/清除、来源令牌、传输提示、失败形状、脱敏 Trace 和可访问性归 REF/AG/SYS DESIGN Scope；本 L1 不定义文件 I/O、接口、存储位置或数据字段
-->

## §6 模块边界与约束（边界闭合）

- **职责边界**：REF 只做解析与供给（§2 R-J1–R-J4）；不组装回合上下文（AG）、不读盘（经 WS）、
  不呈现（经 ED）、不检索知识库（KB）。
- **限界边界**：候选与读取一律限定 workspace 内（X-FORBID-004）；REF 无直接文件 I/O（X-SEQ-002）。
- **锚边界**：块级锚消费 ED blockId 契约（BR-ED-DATA-001），失效信号语义与 ED 一致；红色删除暂存、删除保存和恢复冲突均不得把旧锚伪装为当前可用对象，已发送历史保持冻结快照。
- **多文档边界**：REF 可为用户选择或已确认候选目标提供身份快照，供 AG/DE 冻结目标清单；REF 不把引用、候选或快照升级为写权限，不自行追加目标成员。
- **共享结构边界**：注入载荷若跨模块传递，结构经 SYS 契约层（X-SHARED-001，REF G2 联合确认）。
- **状态机阈值**：@mention 解析注入为请求-响应式解析流（无多步状态化生命周期），按 APC §6 预判
  豁免建机；REF G2 若暴露状态化链路（如异步批量解析）再建机登记，本文不预判结论为最终。
- **L4 原料边界**：取材台账 §4.4（REF 引用 @mention 原料），原料不为权威。
- **外部来源边界**：REF 仅保存并供给 REQ-REF-007 的来源引用；网络访问由 AG 的 REQ-AG-010 受控发起，外部来源和其内容始终为 data-only。
- **外部文件边界**：REF 的 P0 文件引用只保留元数据；REQ-REF-009 的 P1 内容引用只可由用户显式附加和当前可用只读解析路径触发，范围仅限当前任务。REF 不将文件内容自动转存到会话历史或项目记忆，也不扫描、导入或编辑外部文件。

## §7 G1 问题暴露清单

| 编号 | 分类 | 状态 | 处置 |
|---|---|---|---|
| REF-01 | BOUNDARY_OPEN | 本轮关闭 | 可引用目标范围纳入会话、AST 记忆、KB 知识、TPL 模板、外部 URL 与长文本；REF 仅接收已选中快照，不接管各 owner 的检索/管理 |
| REF-02 | BOUNDARY_OPEN | 显式延期 | 注入载荷结构是否入 SYS 共享契约层归 REF G2 联合 SYS/AG |
| REF-03 | BOUNDARY_OPEN | 显式延期 | 锚失效修复交互（重选/自动重定位）归 REF G2/实现期 |
| REF-04 | FLOW_INCOMPLETE | 显式延期 | 状态机豁免预判须在 REF G2 复核（若暴露状态化链路则建机登记总图） |
| REF-05 | ACCEPTANCE_MISSING | 部分关闭 | 定性验收已入 §5；量化指标（候选列举/解析延迟）延 G5 NFR 标定 |
| REF-06 | IMPLEMENTATION_DEBT | 已修复·待观测确认 | 拖拽引用入口真实链路已闭合（REF-REFERENCE-REPAIR-01 F1）：统一 drag session store（text/plain marker 保底）+ 编辑器 document 冒泡回写解决 PM clearData；文件树/编辑器标签/会话标签/编辑器选区四源接线完成；真实 Tauri/WebKit 事件观测与冒烟待人工确认后正式关闭 |
| REF-07 | DESIGN_RISK | 已识别，待 G2 | REQ-REF-009 的外部文件内容引用身份、可读格式判定、一次性任务范围、快照/清除、失败、来源显示与隔离，须由 REF/AG/SYS 联合定义；不得退化为目录扫描、自动复读或默认持久化。 |
| REF-08 | DESIGN_RISK | 已识别，待 G2 | 公开网页研究来源的稳定身份、快照/摘录边界、可用性重新校验和跨会话可见性，须按 REQ-REF-007 与 AG/SYS 联合设计；REF 不得代行网络访问 |
| REF-09 | DESIGN_RISK | 已识别，待 G2 | 删除暂存/保存/恢复与多文档目标冻结后的引用身份、失效、历史快照和跨模块事件，须按 REQ-REF-003/008 联合 AG/DE/ED/WS/SYS 设计；不得把引用当作写授权或继续注入已删除目标。 |

> 术语：引用、注入载荷等 REF 主责核心概念在 REF L2 评估注册 TERM。

## §8 后续动作

1. 待人类宣布 REF 模块级 G1 闭合后进入 REF G2：引用目标模型与候选解析规则、注入载荷契约
  （解除 REF-02）、锚失效语义细则、与 ED 扩展点/AG 消费的对接契约；复核状态机豁免（REF-04）。
2. G2 经 REQ_MAP 把 REQ-REF-001~009 映射为 RULE/CHAIN/CONSTRAINT/TERM，并补齐 REF-07~09 的快照、可见性与来源契约。
3. KB/AST/TPL 后续若提供资产面板条目，只需输出 ReferencePayload，不改变 REF 输入令牌协议。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-11 | v1.0 | 初版（P3 第三件，P3 L1 层齐备）：REF 引用模块级 L1；声明 REQ-REF-001~004（@mention 触发与解析 / 内容读取与注入载荷 / 锚定与失效感知 / 经 ED 表面），承接 REQ-CORE-001/006，对接 AG（REQ-AG-004 消费）与 ED（blockId 锚、扩展表面）；代表性流程 R1/R2；G1 暴露清单 REF-01~05 |
| 2026-07-04 | v1.1 | 上游约束文件名指针更新（A-AG-*/A-DE-* 文件更名为 代理*/DiffEdit*，2026-07-04 人类确认；文档编号不变，无语义变更） |
| 2026-07-08 | v1.2 | 引用输入扩展：新增多来源引用归一化与输入流 ReferenceToken 一致性需求（REQ-REF-005/006）；代表性流程增 R3；REF-01 关闭为“接收已选中快照、不接管 AST/KB/TPL 检索管理” |
| 2026-07-08 | v1.3 | 登记 REF-06 拖拽引用入口真实链路未闭合：复制/粘贴路径可用，拖拽路径暂停优化，技术侧详见 A-REF-M-T-01 §9 |
| 2026-08-07 | v1.4 | REF-REFERENCE-REPAIR-01：REQ-REF-005 目标范围扩至外部文件（file 引用仅元数据）；REF-06 状态更新为「已修复·待观测确认」（drag session store + 编辑器 document 冒泡回写落地，真实观测待人工） |
| 2026-08-13 | v1.5 | 依据 1.0 能力基线同步引用输入：P0 明确当前选区、workspace 文档/编辑器标签与会话标签；发送时冻结身份/锚/范围/版本快照；外部文件保留元数据引用，未保存文档默认仅当前明确关联任务可见。 |
| 2026-08-13 | v1.6 | AG-1.0-R2-REQUIREMENTS-01：引用读取与多来源归一化承接 CAP-04 P0；所有注入载荷带来源/新鲜度且为 data-only，引用或比较不自动升级为编辑目标或写权限；修正能力基线章节索引。 |
| 2026-08-14 | v1.7 | AG-1.0-R4-NETWORK-REQUIREMENTS-01：新增 REQ-REF-007，承接 AG 已受控读取的公开网页来源引用；REF 只提供可追溯的 data-only 来源身份，不发起网络请求，也不扩大外部文件读取范围。 |
| 2026-08-14 | v1.8 | AG-1.0-R4-STRUCTURE-REQUIREMENTS-01：REQ-REF-003 承接逻辑文件树的路径基线推进；已发送引用保持冻结快照，后续引用使用暂定新路径或明确失效，禁止把旧路径伪装为当前路径。 |
| 2026-08-14 | v1.9 | AG-1.0-R4-DELETE-MULTIDOC-REQUIREMENTS-01：REQ-REF-003 扩展承接红色删除暂存与删除保存后的失效；新增 REQ-REF-008，目标清单身份只能在用户确认后冻结供逐文档 Proposal 使用，引用不构成写权，已删除/恢复冲突目标不伪装为当前对象。 |
| 2026-08-14 | v2.0 | AG-1.0-R5C-MEMORY-EXTERNAL-READ-REQUIREMENTS-01：新增 REQ-REF-009；外部文件由 P0 元数据引用扩展为 P1 的用户显式、一次性 data-only 内容引用，未读格式如实退回元数据；项目记忆仅接收用户已选中的条目快照，不自动写入或注入。 |
| 2026-08-21 | v2.1 | 工作区资源操作基线重构：REQ-REF-003/008 改为确认前路径不变、WorkspaceOperation 成功后按真实回执重绑定或失效、WorkspaceTimeline 恢复后按新基线恢复引用；历史快照不冒充当前对象。 |
