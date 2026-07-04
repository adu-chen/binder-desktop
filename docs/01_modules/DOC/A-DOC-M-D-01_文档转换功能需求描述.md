---
文档编号：DOC-M-D-01
文档状态：A
负责模块：DOC
文档职责：DOC 文档转换模块级功能需求描述——声明模块级 REQ-DOC-*，承接产品级 REQ-CORE-001/004/009，界定 docx 打开转换 / docx 保存转换（往返）/ 样式子集契约与用户明示 / 预览转换服务（供 PRV）/ 本地转换运行时与离线边界
上游约束：A-CORE-C-D-01_产品概述.md（REQ-CORE-001/004/009）、A-CORE-C-P-01_APC.md（§4.1 DOC 职责「DOCX/Pandoc 转换」）、A-SYS-C-T-01_系统总体架构.md（§4 DOC→WS,ED 依赖）、A-ED-M-T-01_编辑器技术设计.md（BR-ED-DATA-002 转换归 DOC、§11.1 样式子集选型记录）、A-WS-M-T-01_工作台技术设计.md（X-FORBID-004、save 经 WS）、A-SYS-M-S-01_状态机总图.md（docx 开/存核心链 owner=DOC）、ADU.md（§3.2 REQ 规范）
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

DOC 是 Binder 的**文档格式转换模块**（APC §4.1：DOCX/Pandoc 转换）：承载 docx 与统一编辑模型之间
的双向转换（DOCX 往返，REQ-CORE-004）、样式子集契约的执行，以及供 PRV 消费的预览转换服务。
DOC 依赖 WS（文件经 WS 读写）与 ED（转换目标为 ED 统一编辑模型），被 ED/PRV 消费
（A-SYS-C-T-01 §4：`DOC→WS,ED`）。取材旧版 R-ED-M-R-01 v2.0（DOCX 现状主源，台账 §4.5 P1）与
R-ED-M-R-23（样式子集），原料不为权威，REQ 重新撰写（APC §8）。

## §2 模块定位与职责

**定位**：DOC 是 docx 等二进制文档格式与 Binder 统一编辑模型之间的**唯一转换通道**；其他模块
（ED/PRV/CBT/AG）不内置二进制格式处理。

**职责清单（v1，APC §4.1，方案承旧版实证三模式分离）**：

| # | 职责 | 锚定 |
|---|---|---|
| D-J1 | docx 打开转换：docx → 统一编辑模型（Pandoc 编辑链），交 ED 编辑 | REQ-CORE-004 |
| D-J2 | docx 保存转换：统一编辑模型 → docx（往返），经 WS 原子落盘 | REQ-CORE-004 |
| D-J3 | 样式子集契约执行：必需/部分/不处理三级，进入编辑即子集化且用户可知 | REQ-CORE-004、A-ED-M-T-01 §11.1 |
| D-J4 | 预览转换服务：docx 等 → PDF（LibreOffice 预览链），供 PRV 全保真预览 | REQ-CORE-004 |
| D-J5 | 本地转换运行时：转换全程本地执行（Pandoc/LibreOffice 本地依赖），离线可用、内容不上行 | REQ-CORE-001/009 |

> 边界澄清：**编辑链与预览链完全分离**（旧版实证决策）：编辑链 Pandoc（样式子集化、可编辑），
> 预览链 LibreOffice→PDF（全保真、只读）。编辑呈现归 ED、预览呈现与打印导出归 PRV、落盘归 WS；
> DOC 只做转换。docx 开/存为状态化核心链（A-SYS-M-S-01 §3，owner=DOC），DOC G2 建机。

## §3 能力范围（DOC v1）

**IN（DOC 承载）**：
- docx → 统一编辑模型（打开转换）、统一编辑模型 → docx（保存转换）。
- 样式子集三级契约的执行与转换告知（哪些保真/尽力/不处理）。
- docx 等格式 → PDF 的预览转换（供 PRV）。
- 转换过程中 blockId 锚定的协同（转换层兜底注入 data-block-id，A-ED-M-T-01 §11.1）。
- 本地转换运行时与依赖（Pandoc / LibreOffice）的打包集成边界。

**OUT（非 DOC）**：
- 富文本编辑运行时——归 ED；预览呈现/打印/导出管线——归 PRV；文件 I/O 与落盘——归 WS。
- Excel/PPT 编辑——v1 确认不做；Excel/PPT/音视频预览呈现——归 PRV（其转换服务需求待 PRV G1/G2
  暴露后联合，本文不预占，见 §7 DOC-03）。
- md 解析装载——归 ED（md 为编辑模型原生格式，无需 DOC 转换）。
- 转换器具体版本锁定、缓存策略——归 DOC L2/实现期。

## §4 代表性流程（契约级，APC §7）

**D1 · docx 打开转换**（D-J1/D-J3）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | ED/用户 | 打开 workspace 内 docx | DOC 经 WS 读取，Pandoc 转换为统一编辑模型 | 编辑模型（样式子集化） | 2 | 转换失败→统一错误形状，会话不建立（对接 SM-ED-01） |
| 2 | DOC | 转换结果 | 注入缺失 data-block-id（兜底），告知子集化范围 | 可编辑文档 + 子集告知 | 终态 | 不支持样式静默丢弃前必须用户可知 |

**D2 · docx 保存转换（往返）**（D-J2）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | ED | 编辑模型 + save 意图 | DOC 将编辑模型转换为 docx | docx 字节流 | 2 | 转换失败→统一错误形状，原文件不动 |
| 2 | WS | docx 字节流 | 经 WS 原子落盘（SM-WS-01） | 已保存 | 终态 | 落盘失败归 WS 错误路径，可重试 |

**D3 · 预览转换**（D-J4）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | PRV | docx 预览请求 | DOC 经 LibreOffice 转换为 PDF | PDF（全保真） | 终态 | 转换失败→统一错误形状，PRV 呈现失败态 |

## §5 模块级需求声明（REQ-DOC-*）

<!-- REQ
req_id: REQ-DOC-001
title: docx 打开转换（docx→统一编辑模型）
owner_module: DOC
status: active
summary: 将 workspace 内 docx 经本地转换（Pandoc 编辑链）转换为 ED 统一编辑模型，应用样式子集契约，转换失败回统一错误形状且不建立编辑会话
source_doc: REQ-CORE-004、APC §4.1、A-ED-M-T-01（BR-ED-DATA-002）
acceptance: docx 可转换进入编辑；转换失败不产生半成品会话；转换全程本地执行
open_issues: Pandoc 版本与 Lua 过滤器集归 DOC L2/实现期；转换缓存策略延实现期
-->

<!-- REQ
req_id: REQ-DOC-002
title: docx 保存转换（统一编辑模型→docx 往返）
owner_module: DOC
status: active
summary: 将编辑模型转换回 docx 并经 WS 原子落盘，实现 DOCX 往返；转换或落盘失败时原 docx 文件保持不变
source_doc: REQ-CORE-004、A-WS-M-T-01（BR-WS-PERSIST-001）
acceptance: docx 编辑后可往返保存；失败不破坏原文件；必需样式子集往返保真
open_issues: blockId（data-block-id）经 Pandoc 往返的保留度需实测（A-ED-M-T-01 §11.2），不达则兜底重建
-->

<!-- REQ
req_id: REQ-DOC-003
title: 样式子集契约与用户明示
owner_module: DOC
status: active
summary: 执行三级样式子集契约（必需承诺保真 / 部分尽力保留 / 明确不处理），docx 进入编辑态即子集化，子集化范围与可能损失对用户可知，不静默丢失
source_doc: REQ-CORE-004、A-ED-M-T-01 §11.1（选型记录）、旧版 R-ED-M-R-23（实证原料）
acceptance: 必需子集（标题/对齐/粗斜下/列表/基础表格/图片）往返保真；部分支持项损失对用户明示；不处理项清单明确且预览链可全保真查看
open_issues: 三级清单的执行细则与告知交互归 DOC L2 与实现期
-->

<!-- REQ
req_id: REQ-DOC-004
title: 预览转换服务（供 PRV）
owner_module: DOC
status: active
summary: 提供 docx→PDF 的本地预览转换（LibreOffice 预览链），与编辑链完全分离，供 PRV 全保真只读预览，补偿编辑链样式子集化的查看需求
source_doc: REQ-CORE-004、APC §4.1（PRV 职责分界）
acceptance: docx 可经 DOC 转换为全保真 PDF 供 PRV 呈现；预览链与编辑链无功能黏连
open_issues: Excel/PPT/其他格式的预览转换是否归 DOC 待 PRV G1 暴露后联合确认（§7 DOC-03）
-->

<!-- REQ
req_id: REQ-DOC-005
title: 本地转换运行时与离线边界
owner_module: DOC
status: active
summary: 转换全程在本地执行（Pandoc/LibreOffice 本地依赖打包），离线可用，文档内容不因转换上行云端；转换依赖缺失时明确报错不静默降级
source_doc: REQ-CORE-001、REQ-CORE-009、APC §9
acceptance: 断网时 docx 开/存/预览转换可用；无网络传输文档内容；依赖缺失有明确错误
open_issues: Pandoc/LibreOffice 的打包体积与按需加载策略归 DOC L2/实现期
-->

## §6 模块边界与约束（边界闭合）

- **职责边界**：DOC 只做格式转换（§2 D-J1–D-J5）；不编辑、不呈现、不落盘、不预览展示。
- **唯一转换通道**：docx 等二进制格式的解析/生成必经 DOC（ED 无二进制代码，BR-ED-DATA-002 对偶），
  DOC L2 以 CONSTRAINT 锁定。
- **双链分离**：编辑链（Pandoc）与预览链（LibreOffice→PDF）完全分离，无功能黏连（旧版实证决策）。
- **文件边界**：DOC 读写文档经 WS（X-FORBID-004 路径守护），转换中间产物不越 workspace 管理边界。
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

> 术语：样式子集为 DOC 主责核心概念，在 DOC L2 注册 TERM。

## §8 后续动作

1. 待人类宣布 DOC G1 闭合后进入 DOC G2：转换管线规则 SSOT、样式子集执行契约、SM-DOC-01
   docx 开/存状态机（解除总图 planned 行）、与 ED/WS 的 X-INTER-001 转移点登记。
2. G2 经 REQ_MAP 把 REQ-DOC-001~005 映射为 RULE/CHAIN/CONSTRAINT/TERM。
3. PRV G1 时联合确认 DOC-03（预览转换服务归属）。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-11 | v1.0 | 初版：DOC 文档转换模块级 L1；声明 REQ-DOC-001~005（docx 打开转换 / 保存转换往返 / 样式子集契约与用户明示 / 预览转换服务 / 本地转换运行时与离线边界），承接 REQ-CORE-001/004/009 与 APC §4.1；采纳旧版实证三模式分离（编辑链 Pandoc / 预览链 LibreOffice→PDF / 保存链 HTML→Pandoc→DOCX，R-ED-M-R-01 v2.0 原料）；代表性流程 D1–D3；G1 暴露清单 DOC-01~05 |
| 2026-07-04 | v1.1 | 术语统一清扫（承 2026-07-04 人类三轮裁决）：正文禁用别名替换——内联辅助→局部修改、自动补全→辅助续写、对话编辑→Diff Edit、Chat↔Build 接管→构建移交/回落、chat/chat-agent 回合→代理回合；文件名与变更记录历史行不改写 |
