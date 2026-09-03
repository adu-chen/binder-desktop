---
文档编号：DOC-M-D-01
文档状态：A
负责模块：DOC
文档职责：DOC 文档转换模块级功能需求描述——声明模块级 REQ-DOC-*，承接产品级 REQ-CORE-001/004/009/013，界定原生 DOCX 只读预览转换 / BDocx 编辑副本的打开与保存转换 / TXT↔Markdown 的最小副本转换 / 样式子集契约与用户明示 / 预览转换服务（供 PRV）/ 本地转换运行时与离线边界
上游约束：A-CORE-C-D-01_产品概述.md（REQ-CORE-001/004/009/013）、A-CORE-X-D-02_Agent1.0冻结能力基线.md（P0/P1 与不可突破边界）、A-CORE-C-P-01_APC.md（§4.1 DOC 职责「DOCX/Pandoc 转换」）、A-SYS-C-T-01_系统总体架构.md（§4 DOC→WS,ED 依赖）、A-ED-M-T-01_编辑器技术设计.md（BR-ED-DATA-002 转换归 DOC、§11.1 样式子集选型记录）、A-WS-M-T-01_工作台技术设计.md（X-FORBID-004、save 经 WS）、A-SYS-M-S-01_状态机总图.md（docx 开/存核心链 owner=DOC）、ADU.md（§3.2 REQ 规范）
直接承接：DOC L2 技术设计（转换管线 / 样式子集执行契约 / docx 开存状态机）经 REQ_MAP 映射 RULE/CHAIN/CONSTRAINT/TERM；解除 A-SYS-M-S-01 §3 docx 开/存 planned 行
使用边界：本文是 L1 功能需求描述层，仅声明模块级 REQ-DOC-*，**不定义技术规则**（见 DOC L2）；**代码不得直接映射 REQ**（ADU §3.2）；转换器选型细节、管线实现、缓存策略归 DOC L2/实现期；富文本编辑归 ED；预览呈现与打印导出归 PRV；文件落盘归 WS
变更要求：DOC 职责或转换边界变更须先经 APC 授权再回写本文；REQ 增删按 ADU §3.2 req_id 规则
治理成熟度：draft
---

# Binder DOC 文档转换功能需求描述（L1）

## §1 文档定位

本文是 **DOC 模块**的 L1 功能需求描述：

```
产品概述(REQ-CORE-*) + APC §4.1 → 本文(REQ-DOC-*) → REQ_MAP → DOC L2 规则(SSOT) → 代码(@GOV)
```

DOC 是 Binder 的**文档格式转换模块**（APC §4.1：DOCX/Pandoc 转换）：承载原生 DOCX 的只读预览转换，
以及仅针对 BDocx 编辑副本的 DOCX 与统一编辑模型之间的双向转换（REQ-CORE-004）、样式子集契约的执行，
并供 PRV 消费预览转换服务。
DOC 依赖 WS（文件经 WS 读写）与 ED（转换目标为 ED 统一编辑模型），被 ED/PRV 消费
（A-SYS-C-T-01 §4：`DOC→WS,ED`）。取材旧版 R-ED-M-R-01 v2.0（DOCX 现状主源，台账 §4.5 P1）与
R-ED-M-R-23（样式子集），原料不为权威，REQ 重新撰写（APC §8）。

## §2 模块定位与职责

**定位**：DOC 是 docx 等二进制文档格式与 Binder 统一编辑模型之间的**唯一转换通道**；其他模块
（ED/PRV/CBT/AG）不内置二进制格式处理。

**职责清单（v1，APC §4.1，方案承旧版实证三模式分离）**：

| # | 职责 | 锚定 |
|---|---|---|
| D-J1 | BDocx 编辑副本预览转换：原生 DOCX 来源 → HTML 编辑模型（Pandoc），用于新副本的预览与 ED 编辑 | REQ-CORE-004 |
| D-J2 | BDocx 编辑副本保存转换：HTML 编辑模型 → 新副本 DOCX，经 WS 原子落盘 | REQ-CORE-004 |
| D-J3 | 样式能力契约：按已确认的“格式 × 操作 × 保真等级”向用户明示支持、尽力保留与不处理边界；每项实现后真实验证 | REQ-CORE-004、A-ED-M-T-01 §11.1 |
| D-J4 | 预览转换服务：docx 等 → PDF（LibreOffice 预览链），供 PRV 以语料实测为准的只读预览 | REQ-CORE-004 |
| D-J5 | 本地转换运行时：转换全程本地执行（Pandoc/LibreOffice 本地依赖），离线可用、内容不上行 | REQ-CORE-001/009 |
| D-J6 | 最小文本转换：TXT→Markdown、Markdown→TXT 的目标副本内容转换与保真/损失告知 | REQ-CORE-013 |

> 边界澄清：**编辑链与预览链完全分离**：原生 DOCX 只走 LibreOffice→PDF 的只读预览链；只有创建后的
> BDocx 编辑副本才走 Pandoc 编辑链（DOCX→HTML→DOCX）。编辑呈现归 ED、预览呈现与打印导出归 PRV、落盘归 WS；
> DOC 只做转换。BDocx 编辑副本的开/存为状态化核心链（A-SYS-M-S-01 §3，owner=DOC），DOC G2 建机。

## §3 能力范围（DOC v1）

**IN（DOC 承载）**：
- 原生 DOCX → PDF 的只读预览转换；原生 DOCX 来源 → HTML（用于 BDocx 编辑副本预览）、HTML → 新副本 DOCX（保存转换）。
- TXT→Markdown 与 Markdown→TXT 的本地内容转换，供新的目标副本提案使用；前者不推断语义，后者明确有损。
- 样式子集三级契约的执行与转换告知（哪些保真/尽力/不处理）。
- docx 等格式 → PDF 的预览转换（供 PRV）。
- 转换过程中 blockId 锚定的协同（转换层兜底注入 data-block-id，A-ED-M-T-01 §11.1）。
- 本地转换运行时与依赖（Pandoc / LibreOffice）的打包集成边界。

**OUT（非 DOC）**：
- 富文本编辑运行时——归 ED；预览呈现/打印/导出管线——归 PRV；文件 I/O 与落盘——归 WS。
- Excel/PPT 编辑——v1 确认不做；Excel/PPT/音视频预览呈现——归 PRV（其转换服务需求待 PRV G1/G2
  暴露后联合，本文不预占，见 §7 DOC-03）。
- md 解析装载——归 ED（md 为编辑模型原生格式，无需 DOC 转换）。
- Markdown↔HTML、HTML↔TXT、原生 DOCX 与 TXT/Markdown/HTML 的通用互转、批量/原地转换——不属于 DOC 1.0 的 REQ-DOC-006；BDocx 编辑副本不因此变为通用转换。
- 转换器具体版本锁定、缓存策略——归 DOC L2/实现期。

## §4 代表性流程（契约级，APC §7）

**D1 · BDocx 编辑副本打开转换**（D-J1/D-J3）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | AG/DE/ED/用户 | “创建 BDocx 编辑副本”Proposal 与原生 DOCX 来源 | DOC 经 WS 只读读取来源，Pandoc 转换为 HTML 编辑模型 | 新副本候选内容（样式子集化） | 2 | 转换失败→统一错误形状，不产生副本 LogicalState；原生 DOCX 不进入编辑会话 |
| 2 | DE/ED | 预览成功的副本候选内容 | 建立未保存的新副本 LogicalState 与标签，注入缺失 data-block-id（兜底）并告知子集化范围 | 可编辑副本 + 子集告知 | 终态 | 不支持样式静默丢弃前必须用户可知；尚不写入文件 |

**D2 · BDocx 编辑副本保存转换（往返）**（D-J2）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | ED | BDocx 副本 HTML 编辑模型 + save 意图 | DOC 将编辑模型转换为副本 DOCX | docx 字节流 | 2 | 转换失败→统一错误形状，原生 DOCX 与副本既有落盘版本均不动 |
| 2 | WS | 副本 docx 字节流 | 经 WS 原子落盘（SM-WS-01） | 已保存的副本 | 终态 | 落盘失败归 WS 错误路径，可重试 |

**D3 · 预览转换**（D-J4）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | PRV | docx 预览请求 | DOC 经 LibreOffice 转换为 PDF | PDF（以语料实测为准） | 终态 | 工具无效、启动、字体覆盖或转换失败→统一错误形状，PRV 呈现失败态 |

**D4 · 最小文本转换**（D-J6）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | AG/DE | 已确认格式方向、来源冻结内容与目标副本 Proposal | DOC 只转换来源内容；TXT→Markdown 保留文本/换行且不推断语义，Markdown→TXT 产生有损的可读纯文本 | 目标候选内容 + 保真/损失结果 | 2 | 格式不支持、来源不可读或转换失败→统一错误形状，不建立目标 LogicalState 或文件 |
| 2 | DE/ED | 成功的目标候选内容与保真/损失结果 | 交新建目标提案链预览；DOC 不创建标签、不裁决卡片、不写盘 | 可编辑未保存目标或失败 | 终态 | 保存冲突/失败归 WS，保留已存在的目标 LogicalState；来源不被改写 |

## §5 模块级需求声明（REQ-DOC-*）

<!-- REQ
req_id: REQ-DOC-001
title: BDocx 编辑副本打开转换（DOCX→HTML 编辑模型）
owner_module: DOC
status: active
summary: 对“创建 BDocx 编辑副本”Proposal，仅读取原生 DOCX 来源并经本地 Pandoc 转换为 ED 的 HTML 编辑模型，应用样式子集契约；预览成功才建立新副本的 LogicalState。原生 DOCX 不走编辑链，转换失败回统一错误形状且不建立编辑会话
source_doc: REQ-CORE-004、APC §4.1、A-ED-M-T-01（BR-ED-DATA-002）
acceptance: 只有 BDocx 编辑副本可转换进入编辑；原生 DOCX 不产生编辑会话；转换失败不产生半成品会话或副本文件；预览成功后只生成未保存副本 LogicalState；转换全程本地执行
open_issues: Pandoc 版本与 Lua 过滤器集归 DOC L2/实现期；转换缓存策略延实现期
-->

<!-- REQ
req_id: REQ-DOC-002
title: BDocx 编辑副本保存转换（HTML 编辑模型→DOCX）
owner_module: DOC
status: active
summary: 将 BDocx 编辑副本的 HTML 编辑模型转换回其副本 DOCX 并经 WS 原子落盘；转换或落盘失败时原生 DOCX 与副本既有文件均保持不变
source_doc: REQ-CORE-004、A-WS-M-T-01（BR-WS-PERSIST-001）
acceptance: BDocx 编辑副本可往返保存；失败不破坏原生 DOCX 或副本既有文件；样式子集固定为 P0 加粗/斜体/下划线/删除线，以及 P1 颜色/突出显示、段落对齐/间距/缩进、字号与字体族；表格/图片的已确认 P1 操作在专项验证后方可开放；只有经确认并验证的样式、表格或图片操作才可写入对应 OOXML，重开后恢复到统一编辑模型；字体族切换须处理可用性与替代，并完成真实保存重开验证；嵌入图片以及经校验物化的 workspace 项目图片，其资源、alt、显式尺寸、文档流顺序与删除结果在保存重开后保持
open_issues: blockId（data-block-id）经 Pandoc 往返的保留度需实测（A-ED-M-T-01 §11.2），不达则兜底重建
-->

<!-- REQ
req_id: REQ-DOC-003
title: 样式能力契约与用户明示
owner_module: DOC
status: active
summary: 对 BDocx 编辑副本，按 R3 已确认的样式子集建立用户可知的支持、尽力保留与明确不处理边界：字符基础样式为 P0；颜色/突出显示、段落对齐/间距/缩进、字号与字体族为 P1；页面级布局、页眉页脚、复杂样式继承和任意定位不纳入 1.0。已确认能力必须实现，真实验证是对外开放门槛，不是是否支持的待决条件
source_doc: REQ-CORE-004、A-ED-M-T-01 §11.1（选型记录）、旧版 R-ED-M-R-23（实证原料）
acceptance: 每项必须声明适用处理路径、作用范围、Diff 表达、保存重开保真等级与失败告知；HTML 与 BDocx 均实现已确认的 P0/P1 集合，字体族切换不因 BDocx 处理路径而被排除；每项实现后以真实样例完成保存重开验证，未验证不得对外开放；部分支持项的损失必须对用户明示；P1 图片仅覆盖受控资源、alt、显式尺寸、实例数量和文档流顺序/删除结果，项目图片草稿缓存缺失时可按 workspace 相对来源与内容哈希重新物化；不处理项清单明确且预览链可按 LO 语料实测结果只读查看
open_issues: 字体可用性、替代与 BDocx 保存重开保真等级；已确认 StyleOp 的 HTML/OOXML 映射、逐项检测与用户告知归 DOC L2 与实现期；`data-display`、浮动/绝对定位和任意位置布局不在图片保证集
-->

<!-- REQ
req_id: REQ-DOC-004
title: 预览转换服务（供 PRV）
owner_module: DOC
status: active
summary: 提供原生 docx→PDF 的本地预览转换（LibreOffice 预览链），与 BDocx 编辑链完全分离，供 PRV 以 LO 语料实测为准的只读预览
source_doc: REQ-CORE-004、APC §4.1（PRV 职责分界）
acceptance: 原生 DOCX 可经 DOC 转换为以 LO 语料实测为准的只读 PDF 供 PRV 呈现；预览链与 BDocx 编辑链无功能黏连；工具无效、启动、字体覆盖和转换失败均为明确失败态
open_issues: Excel/PPT/其他格式的预览转换是否归 DOC 待 PRV G1 暴露后联合确认（§7 DOC-03）
-->

<!-- REQ
req_id: REQ-DOC-005
title: 本地转换运行时与离线边界
owner_module: DOC
status: active
summary: 转换全程在本地执行，Pandoc/LibreOffice 与默认回退字体必须随 Binder 发布包内置；运行时只能执行经校验的包内工具，离线可用，文档内容不因转换上行云端；包内依赖缺失、版本不符或失效时 Binder 仍可启动，但 DOC command 必须明确报错并禁用相关入口，绝不探测或降级到设备本地工具
source_doc: REQ-CORE-001、REQ-CORE-009、APC §9
acceptance: 断网时 docx 开/存/预览转换可用；无网络传输文档内容；真实发布包只调用自身资源目录内的 Pandoc/LibreOffice；包内依赖缺失时 Binder 其余功能可启动、DOC command 返回明确错误且不调用 PATH、/Applications 或用户安装工具
open_issues: 工具版本、架构、许可证、哈希与按需加载策略归 DOC L2/实现期；LibreOffice GlyphFallback 字体债务受发布阻断项管理；DOC-PLAT-001（macOS 最低版本与 arm64/x86_64 发布矩阵）已登记为暂缓债务，不以本地工具补救
-->

<!-- REQ
req_id: REQ-DOC-006
title: 最小文本格式转换与保真/损失告知
owner_module: DOC
status: active
summary: 仅对已确认的 TXT→Markdown、Markdown→TXT 目标副本 Proposal 在本地转换来源内容。TXT→Markdown 只保留文本和换行，不自动推断标题、列表、任务或其他 Markdown 语义；Markdown→TXT 是有损转换，输出可读纯文本但不承诺样式、结构、表格、图片或可逆往返。DOC 只处理内容转换，不改写来源、不创建目标文件或标签
source_doc: REQ-CORE-013、A-CORE-X-D-02 §3/§4、A-AG-M-D-01（REQ-AG-012）
acceptance: 仅支持两条已确认方向且全部本地执行；转换结果随 Proposal 提供输入/输出格式及保真或损失说明；TXT→Markdown 不注入未在来源表达的 Markdown 语义；Markdown→TXT 不伪称无损或可逆；不支持的格式、来源读取失败或转换失败返回明确失败且不产生目标 LogicalState 或磁盘文件；HTML 互转、通用 DOCX 互转、批量/原地转换无入口，BDocx 仍仅遵循 REQ-DOC-001/002 的专用编辑副本链
open_issues: 文本编码、换行、空白、具体 Markdown 语法降级/序列化规则、转换错误形状、能力检测和测试语料归 DOC/ED/WS/DE DESIGN Scope；本 L1 不定义算法、工具或接口
-->

## §6 模块边界与约束（边界闭合）

- **职责边界**：DOC 只做格式转换（§2 D-J1–D-J6）；不编辑、不呈现、不落盘、不预览展示。
- **唯一转换通道**：docx 等二进制格式的解析/生成必经 DOC（ED 无二进制代码，BR-ED-DATA-002 对偶），
  DOC L2 以 CONSTRAINT 锁定。
- **双链分离**：原生 DOCX 预览链（LibreOffice→PDF）与 BDocx 编辑链（Pandoc）完全分离，无功能黏连。
- **文件边界**：DOC 读写文档经 WS（X-FORBID-004 路径守护）；转换中间产物仅位于应用数据目录的受控请求生命周期内，
  不进入 workspace 文件树或无界系统临时目录。
- **状态机边界**：docx 开/存为状态化核心链（X-SEQ-003），DOC G2 建机（SM-DOC-01）并登记总图。
- **L4 原料边界**：取材台账 §4.5 docx 子节（A-ED-M-T-04、R-ED-M-R-01 P1 主源、R-ED-M-R-23），
  原料不为权威。

## §7 G1 问题暴露清单

| 编号 | 分类 | 状态 | 处置 |
|---|---|---|---|
| DOC-01 | BOUNDARY_OPEN | 显式延期 | Pandoc/LibreOffice 版本锁定、Lua 过滤器集、打包体积与按需加载归 DOC L2/实现期 |
| DOC-02 | DESIGN_RISK | 显式延期 | data-block-id 经 Pandoc 往返保留度未实测（影响 DE/REF 锚定），DOC G2 定兜底契约、实现期实测（联动 A-ED-M-T-01 §11.2） |
| DOC-03 | BOUNDARY_OPEN | 显式延期 | Excel/PPT/音视频预览的转换服务归属（DOC 抑或 PRV 自带）待 PRV G1 暴露后联合确认 |
| DOC-04 | NEEDS_HUMAN_DECISION | 显式延期（不阻断 G1） | 样式子集告知交互形态（打开时弹层/状态栏标识）属产品体验决策，延 G2/实现期人类确认 |
| DOC-05 | ACCEPTANCE_MISSING | 部分关闭 | 定性验收已入 §5；量化指标（转换耗时：旧版目标首开 <12s/缓存 <2s、准确率 ≥90%）延 G5 标定复核 |
| DOC-06 | PLATFORM_DEFERRED | 已登记，暂不处理 | macOS 最低版本与 arm64/x86_64 发布矩阵见 DOC-PLAT-001；本轮不扩大为跨架构实现 |
| DOC-07 | DESIGN_RISK | 已识别，待 G2 | TXT↔Markdown 的编码/换行、语法降级、保真/损失表达、失败形状和语料验证须按 REQ-DOC-006 联合 AG/DE/ED/WS 设计；不得把现有 BDocx 或内部转换函数泛化为已支持格式矩阵。 |

> 术语：样式子集为 DOC 主责核心概念，在 DOC L2 注册 TERM。

## §8 后续动作

1. 待人类宣布 DOC G1 闭合后进入 DOC G2：转换管线规则 SSOT、样式子集执行契约、SM-DOC-01
   BDocx 编辑副本开/存状态机（解除总图 planned 行）、与 ED/WS 的 X-INTER-001 转移点登记。
2. G2 经 REQ_MAP 把 REQ-DOC-001~006 映射为 RULE/CHAIN/CONSTRAINT/TERM。
3. PRV G1 时联合确认 DOC-03（预览转换服务归属）。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-11 | v1.0 | 初版：DOC 文档转换模块级 L1；声明 REQ-DOC-001~005（docx 打开转换 / 保存转换往返 / 样式子集契约与用户明示 / 预览转换服务 / 本地转换运行时与离线边界），承接 REQ-CORE-001/004/009 与 APC §4.1；采纳旧版实证三模式分离（编辑链 Pandoc / 预览链 LibreOffice→PDF / 保存链 HTML→Pandoc→DOCX，R-ED-M-R-01 v2.0 原料）；代表性流程 D1–D3；G1 暴露清单 DOC-01~05 |
| 2026-07-04 | v1.1 | 术语统一清扫（承 2026-07-04 人类三轮裁决）：正文禁用别名替换——内联辅助→局部修改、自动补全→辅助续写、对话编辑→Diff Edit、Chat↔Build 接管→构建移交/回落、chat/chat-agent 回合→代理回合；文件名与变更记录历史行不改写 |
| 2026-07-21 | v1.2 | DOC-BUNDLED-RUNTIME-01：REQ-DOC-005 收紧为 Pandoc、LibreOffice 与默认字体均须由 Binder 包内受控资源提供；禁止 PATH、`/Applications` 或其他设备本地安装回退；包内资源校验失败必须显式失败。 |
| 2026-07-21 | v1.3 | DOC-LO-RELIABILITY-FONT-PLAN-01：登记 DOC-06 / DOC-PLAT-001 为暂缓平台债务；本轮修复范围聚焦无界面 LO 运行、稳定 headless、确定性包内字体回退和 Pandoc 样式能力可告知。 |
| 2026-07-21 | v1.4 | 审计修正：REQ-DOC-005 明确包内工具失效只禁用 DOC command、不得阻断 Binder 其余功能启动；运行时仍无系统工具/字体回退。 |
| 2026-07-21 | v1.5 | 审计修正：预览保真改为以 LO 语料实测为准的只读目标；REQ-DOC-003 明确当前三级结果、五档目标报告和 ED 样式闭环待实现；转换中间产物改由应用数据目录受控生命周期管理。 |
| 2026-07-27 | v1.6 | DOCX-COLOR-ROUNDTRIP-01：回写文本颜色的保存/重开保真、实际检测报告与 ED 状态栏告知闭合范围；保留其余样式子集与高级 OOXML 检测为未闭合项。 |
| 2026-07-28 | v1.7 | BDocx TableCellShading：REQ-DOC-002/003 纳入单元格纯色底纹的 OOXML `w:shd` 保存、重开恢复、实际检测与风险报告；渐变、主题与图案填充仍不在保证集。 |
| 2026-07-28 | v1.8 | BDOCX-IMAGE-INTERACTION-01：REQ-DOC-002/003 增加图片文档流顺序和删除结果的保存重开验收；明确 data-display、浮动/绝对定位不属于嵌入图片保证集。 |
| 2026-07-29 | v1.9 | BDOCX-IMAGE-AGENT-EDIT-01：REQ-DOC-002/003 增加 workspace 项目图片的受控物化、来源/hash 草稿恢复以及资源、alt、尺寸、位置和删除结果验收。 |
| 2026-08-13 | v2.0 | AG-1.0-R3-REQUIREMENTS-01：原生 DOCX 收敛为只读预览/分析；仅新建的 BDocx 编辑副本走 DOCX→HTML→DOCX 编辑转换，原文件不进入编辑链或被写回；HTML/BDocx 样式子集仍待人类确认。 |
| 2026-08-13 | v2.1 | R3 自检收紧：删除文字颜色等旧有“已闭合”样式承诺；样式操作须先获 R3 矩阵确认并专项验证，才可进入 OOXML 保存保证。 |
| 2026-08-14 | v2.2 | AG-1.0-R3-STYLE-REQUIREMENTS-01：R3 确认样式子集进入 DOC L1；字符基础样式 P0，颜色/突出显示、段落样式、字号与字体族 P1，字体族含 BDocx；真实验证是实现后的开放门槛。 |
| 2026-08-14 | v2.3 | AG-1.0-R4-CONVERSION-REQUIREMENTS-01：新增 REQ-DOC-006；仅 TXT→Markdown、Markdown→TXT 承接为本地目标副本转换，前者不推断语义、后者明示有损；HTML 与通用 DOCX 转换仍排除，BDocx 保持专用编辑副本链。 |
