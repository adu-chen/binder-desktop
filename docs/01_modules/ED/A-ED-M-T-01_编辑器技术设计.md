---
文档编号：ED-M-T-01
文档状态：A
负责模块：ED
文档职责：ED 编辑器模块级 L2 技术设计（规则 SSOT）——富文本编辑运行时（TipTap）与编辑会话、block 模型与稳定 blockId 定位、md/docx 统一编辑模型、分页与图文媒体的呈现层纯化、编辑器装饰层（decoration）接口契约；承接 REQ-ED-001~005
上游约束：A-ED-M-D-01_编辑器功能需求描述.md（REQ-ED-001~005、ED-01~06）、A-CORE-C-P-01_APC.md（§4.1 ED 职责、§6 状态机阈值、§9 文件系统/离线边界）、A-SYS-C-T-01_系统总体架构.md（§4 ED→WS、X-SEQ-002）、A-SYS-I-P-01_数据结构定义.md（DocRef、X-SHARED-001）、A-SYS-I-P-02_IPC协议.md（统一错误形状 BR-SYS-OBS-002）、A-WS-M-T-01_工作台技术设计.md（BR-WS-DATA-001、BR-WS-PERSIST-001、X-FORBID-004）、A-WS-M-S-01_工作台状态机设计.md（SM-WS-01）、ADU.md（§3 RULE/CHAIN/CONSTRAINT/TERM 格式）
直接承接：本文 RULE/CHAIN/CONSTRAINT/TERM 为 ED 规则 SSOT，代码经 @GOV 映射；文档编辑会话状态机在 A-ED-M-S-01 定义并登记 A-SYS-M-S-01 总图；decoration 接口契约供 DE（diff 呈现）/ AG（局部修改呈现）G2 承接
使用边界：本文是 ED 规则 SSOT；不定义 diff 生成与终态（归 DE）、不定义 DOCX/Pandoc 二进制转换（归 DOC）、不做文件 I/O 与落盘（归 WS，ED 经 WS）、不锁定 TipTap/ProseMirror 具体版本与扩展集（延实现期选型记录）、状态机内部归 A-ED-M-S-01；inline-assist/autocomplete 业务归 AG（2026-06-11 人类确认），本文只定 decoration 承载面
变更要求：blockId 稳定性语义变更须与 DE（diff 定位）联合复核；decoration 接口契约变更须复核 DE/AG 消费方；跨模块共享结构变更回写 A-SYS-I-P-01（X-SHARED-001）；TERM 一经被 term_ref 引用其 en 不可更改（ADU §3.0）
治理成熟度：draft
---

# Binder 编辑器技术设计（ED · L2 技术设计）

## 0. Terminology Registry

> ED 主责核心业务概念首批注册（跨 session 易漂移 + 代码有实体承载 + 命名混乱致规则误判），
> 承接 A-ED-M-D-01 §7 术语说明。如人类对命名有异议，可在本区域调整后重跑 generate。

<!-- TERM
term_id: TERM-ED-001
chains: ED-EDIT
zh: 内容块
en: Block
forbidden: 内容节点, 段落块, 文本块, 块节点, chunk
-->
ED 编辑模型的结构化组成单元，每个内容块具备稳定 blockId，是 diff 定位（DE）、@mention 引用锚
（REF）与分页切分的统一定位基准（REQ-ED-002）；普通编辑不改变既有 blockId，删除即失效不复用。

<!-- TERM
term_id: TERM-ED-002
chains: ED-DECOR
zh: 编辑器装饰层
en: EditorDecoration
forbidden: 高亮层, 标注层, 覆盖层, overlay层, 装饰器层
-->
ED 在编辑视图上承载非正文呈现（diff 高亮/审阅控件、局部修改提示）的呈现层；按 blockId 定位挂载，
按消费方（DE/AG）信号呈现与复位，自身不修改文档模型与落盘内容（REQ-ED-005）。

<!-- TERM
term_id: TERM-ED-004
chains: ED-EDIT
zh: 字符锚
en: CharAnchor
forbidden: 字符偏移锚, 文本锚, 偏移量锚, inline锚, 字符定位符
-->
会话内字符级内容锚：blockId + 块内纯文本偏移 {start,end}（偏移定义在块内纯文本平面，跨格式统一）。
锚有效期为编辑会话内（重开必换锚，PI-ED-09 实测锁定）；是 agent 精确编辑与样式操作的定位基准
（REQ-ED-006，BR-ED-DATA-004）。

<!-- TERM
term_id: TERM-ED-005
chains: ED-EDIT
zh: 选区引用
en: SelectionRef
forbidden: 选区信息, 拖拽引用, selection数据, 选区数据
-->
编辑器选区的可传递引用：{path, blockId, start, end, text}（含字符锚 + 选区文本）。经受控查询接口
按需读取；选区拖拽入 AG 输入面时作为位置引用载荷（REQ-ED-006），跨模块传递按 REF 注入协议承接。

<!-- TERM
term_id: TERM-ED-006
chains: ED-EDIT
zh: 样式操作
en: StyleOp
forbidden: 样式指令, 格式操作, style命令, 样式patch
-->
受控样式修改操作单元：set_mark / unset_mark / set_block_attr，目标为字符锚区间或 blockId。
agent 样式修改的唯一操作形态（REQ-ED-007，BR-ED-DATA-005）；落盘路径按裁定 D-3 分流
（回合产出经 DE diff 裁决、显式内联视为用户编辑）。

## §1 文档定位

本文是 **ED 模块**的 L2 技术设计，是 ED 规则的唯一来源（SSOT，APC §2/§3.2），承接 ED L1
（REQ-ED-001~005）：

```
REQ-CORE-002/004/009 → REQ-ED-001~005（ED L1）→ 本文 RULE/CHAIN/CONSTRAINT/TERM（SSOT）→ 代码(@GOV)
本文 ←引用← A-SYS-I-P-01（DocRef）、A-WS-M-T-01（文件 I/O 经 WS）、A-WS-M-S-01（SM-WS-01 save）
本文 →驱动→ A-ED-M-S-01（文档编辑会话状态机 SM-ED-01）→登记→ A-SYS-M-S-01 总图
本文 decoration 契约 →供→ DE G2（diff 呈现）/ AG G2（局部修改呈现）承接
```

边界（承 ED L1 §6）：

- 本文定义 ED **规则/链路/约束/术语**；编辑会话状态机内部归 A-ED-M-S-01。
- **TipTap/ProseMirror 具体版本、扩展集与自定义 schema 不在本文锁定**（ED-01）：本文以运行时无关的
  契约描述编辑模型语义，框架选型延实现期并补选型记录回填（§10）；APC §4.1 已锁"基于 TipTap"的
  运行时方向，本文不重议。
- diff 终态归 DE、二进制转换归 DOC、落盘归 WS；inline-assist/autocomplete 业务归 AG
  （2026-06-11 人类确认，A-SYS-M-S-01 §3），本文仅定 decoration 承载面。

## §2 技术分层与边界

```
ED 编辑运行时（本文 SSOT）
  ├─ 编辑会话与脏态（§3）        加载（经 WS）→编辑→脏态（驱动 SM-WS-01）→save（经 WS）
  ├─ block 模型与 blockId（§4）   结构化内容块 + 稳定定位（供 DE/REF/分页）
  ├─ md/docx 统一编辑模型（§5）   docx 经 DOC 转换进出，ED 无二进制处理
  ├─ 分页与图文媒体（§6）         呈现层纯化：布局不改模型；媒体资源经 WS 限界
  └─ 编辑器装饰层（§7）           decoration 接口契约（DE/AG 消费）
```

- ED 在 X-SEQ-002 分层下运行：编辑器（前端）经 Service→注册 command 触达 WS/Rust，**ED 无直接
  文件 I/O 路径**；目标路径受 X-FORBID-004 守护。
- 错误统一走 A-SYS-I-P-02 统一错误形状（BR-SYS-OBS-002）。
- 编辑功能离线可用（REQ-CORE-009）：编辑/分页/媒体呈现不依赖在线鉴权；AI 相关呈现（decoration 的
  局部修改内容）的产生归 AG（须过订阅门禁），ED 只呈现。

## §3 编辑会话与脏态（ED-EDIT）

编辑会话生命周期：打开（经 WS 读取内容→解析装载为编辑模型）→编辑（更新模型、产生脏态）→save
（经 WS SM-WS-01 落盘）→关闭（脏态须先保存或经用户确认丢弃）。解析失败降级只读（不建立可编辑
会话，不静默丢内容）；读取失败回统一错误形状，会话不建立。状态机为 SM-ED-01（A-ED-M-S-01）。

<!-- CHAIN
chain_id: ED-EDIT
主责模块: ED
status: active
risk_tier: P1
-->

<!-- RULE
rule_id: BR-ED-STATE-001
主链路: ED-EDIT
域: STATE
-->
文档编辑会话必须经完整生命周期管理：内容加载与 save 落盘全部经 WS（ED 无直接文件 I/O 路径，
X-SEQ-002/X-FORBID-004）；编辑操作产生脏态并驱动 WS save 保存机（SM-WS-01 Idle→Dirty，X-INTER-001
转移点）；有脏态的会话不得静默关闭或丢失，关闭前须保存或经用户显式确认丢弃；内容解析失败降级只读
而非静默丢内容；编辑功能离线可用，不依赖在线鉴权。状态转移归 SM-ED-01（A-ED-M-S-01）。

<!-- RULE
rule_id: BR-ED-STATE-003
主链路: ED-EDIT
域: STATE
-->
编辑会话必须处理外部变更（订阅 WS 外部变更事件，BR-WS-OBS-001，2026-06-12 缺口补遗）：
**无脏态**（Ready）收到外部变更→自动重载磁盘内容（重载后 blockId 按持久化/重建规则恢复，并给
非阻断提示）；**有脏态**（Editing）收到外部变更或保存冲突（SM-WS-01 saveConflict）→进入冲突
提示，由用户**显式**选择「保留我的编辑（后续保存须经覆盖确认，对接 overrideConfirmed）」或
「放弃我的编辑并重载」——不得静默覆盖外部修改、不得静默重载丢弃脏态，两个方向的内容损失都必须
经用户显式选择。状态转移归 SM-ED-01 的 Conflicted 态（A-ED-M-S-01）。

## §4 block 模型与稳定 blockId（ED-EDIT）

编辑内容以**内容块（Block，TERM-ED-001）**结构化建模：

- 每个内容块在创建时获得 blockId（不透明标识，生成算法延实现期，§10）。
- **稳定性语义**：不删除该块的普通编辑（改文字、改样式、块内操作）不改变其 blockId；块被删除则其
  blockId 永久失效且不复用；块分裂/合并的 blockId 继承规则延实现期敲定并回填本文（§10，ED-02）。
- **失效可感知**：上层（DE/REF）经 blockId 定位失败时必须获得明确的定位失败信号，不得静默错位。

<!-- RULE
rule_id: BR-ED-DATA-001
主链路: ED-EDIT
域: DATA
-->
编辑内容必须以内容块（Block）结构化建模并赋稳定 blockId：不删除该块的普通编辑不改变 blockId；
块删除后其 blockId 永久失效且不复用；上层模块（DE diff 定位、REF 引用锚）经 blockId 定位失败时
必须收到明确失效信号，不得静默错位呈现。blockId 若跨模块传递，其共享结构经 SYS 数据结构契约层
定义（X-SHARED-001），ED 不私自定义跨进程传输结构。

### §4.1 字符级锚定与选区引用（PI-ED-10，承 REQ-ED-006）

块级锚之上提供**字符锚（CharAnchor，TERM-ED-004）**与**选区引用（SelectionRef，TERM-ED-005）**：
字符锚 = blockId + 块内纯文本偏移 {start,end}；**偏移定义在块内纯文本平面**（`textBetween` 语义，
跨 md/docx/html/纯文本统一，与序列化源码偏移无关——docx HTML 模型的标签偏移不作为锚定平面）。
光标/选区位置经**受控查询接口按需读取**（非常驻投影：选区高频变化，常驻投影引发重渲染风暴且
非业务态，X-FORBID-007 无涉）。

<!-- RULE
rule_id: BR-ED-DATA-004
主链路: ED-EDIT
域: DATA
-->
编辑器必须提供会话内字符级内容锚：以 blockId + 块内纯文本偏移 {start,end} 定位（偏移在块内纯文本
平面定义，跨格式统一）；光标与选区经受控查询接口按需读取（SelectionRef 含字符锚与选区文本）。
锚有效期为**编辑会话内**（保存不打断内存模型、锚持续有效；文档关闭重开后 blockId 全量重建、旧锚
失效——PI-ED-09 实测 docx 往返保留率 0%，裁定 D-2）；锚失效（块删除/偏移越界/重开）必须回明确
失效信号，不得静默错位；跨会话字符锚（内容哈希重建）为显式延后债务（§11.2）。字符锚/选区引用
跨模块传递结构经 SYS 契约层定义（X-SHARED-001）。

## §5 md / docx / html 统一编辑模型（ED-EDIT）

md、docx 与 **.html**（裁定 D-1，REQ-ED-008）在 ED 内共用**同一编辑模型与交互**：md 直接解析装载；
docx 由 DOC 转换为统一编辑模型后进入编辑，存盘时经 DOC 转换回 docx（DOCX 往返，REQ-CORE-004）；
.html 为文本格式直接解析装载、序列化 HTML 往返（读写经 WS 文本路径，**不经 DOC 二进制管线**，
X-SEQ-004 无涉），且 .html 源可落盘 `data-block-id`（blockId 跨保存稳定，锚定强于 md 不注入策略）。
ED 不含 docx 二进制解析/生成代码。

<!-- RULE
rule_id: BR-ED-DATA-002
主链路: ED-EDIT
域: DATA
-->
md、docx 与 .html 必须共用同一结构化编辑模型（内容块模型，§4）：docx 的二进制解析与生成全部归
DOC，ED 仅编辑 DOC 转换后的统一模型，ED 代码不得包含 docx 二进制处理路径；.html 为文本格式经 WS
文本读写路径直接解析/序列化往返，不引入独立转换管线；docx 往返中编辑模型与样式子集的承载范围以
本文与 DOC L2 的联合契约为准（ED-04/ED-05 延 DOC G2 联合敲定，本规则先锁分工：转换归 DOC、
编辑归 ED）。

### §5.1 样式投影与样式操作协议（PI-ED-11，承 REQ-ED-007）

文档样式对 agent **可读可编辑**：

- **读侧（样式投影）**：块结构化投影为 `{blockId, type, text, marks:[{type, range:[start,end],
  attrs}], blockAttrs}`（marks range 在块内纯文本平面，与字符锚同基准）；经 AG 读接口
  （ag.list_blocks 扩展，A-AG-I-P-01）供 agent 消费。
- **写侧（样式操作，StyleOp，TERM-ED-006）**：set_mark / unset_mark / set_block_attr，目标为
  字符锚区间或 blockId；必经 ED 受控编辑接口（ed.apply_edit op 粒度表，A-ED-I-P-01 §4.1）。
- **落盘分流（裁定 D-3）**：chat/agent 回合产出的样式修改进 DiffPayload 经 DE diff 裁决落盘
  （X-FORBID-006 唯一路径）；用户显式触发的局部修改样式操作视为用户编辑直改（BR-AG-STATE-003
  语义，产生脏态经 SM-ED-01→SM-WS-01）。

<!-- RULE
rule_id: BR-ED-DATA-005
主链路: ED-EDIT
域: DATA
-->
文档样式必须以机器可读结构投影（块级 marks/blockAttrs，range 在块内纯文本平面与字符锚同基准）
供 agent 读取；样式修改必须以样式操作（StyleOp：set_mark/unset_mark/set_block_attr，目标为字符锚
区间或 blockId）经 ED 受控编辑接口进入（X-FORBID-005），不得绕过接口直改编辑器模型。agent 样式
落盘路径分流（裁定 D-3）：chat/agent 回合产出经 DE diff 裁决落盘（X-FORBID-006），用户显式触发的
局部修改样式操作视为用户编辑直改并产生脏态；不得存在绕过裁决的回合直改路径。

## §6 分页与图文媒体（呈现层纯化，ED-EDIT）

- **分页**：分页布局（分页模式、光标与遮罩、打印友好呈现）是**呈现层**行为：依内容块切分页面，
  不改变文档模型与落盘内容；分页开/关与重排不产生模型变更，不触发脏态。
- **图文媒体**：图片等媒体以媒体内容块进入编辑模型；媒体资源文件经 WS 存取并限定 workspace 内
  （X-FORBID-004），ED 不自行落盘媒体文件。

<!-- RULE
rule_id: BR-ED-DATA-003
主链路: ED-EDIT
域: DATA
-->
分页布局与遮罩是纯呈现层行为，不得改变文档模型或落盘内容，分页重排不产生脏态；图片等媒体以媒体
内容块建模，其资源文件的存取必须经 WS 且限定 workspace 内（X-FORBID-004 / BR-WS-DATA-001），
ED 不得自行写入媒体文件。打印/导出管线归 PRV，ED 仅提供打印友好的分页呈现。

## §7 编辑器装饰层接口契约（ED-DECOR）

装饰层（EditorDecoration，TERM-ED-002）是 ED 供上层呈现非正文内容的唯一受控表面：

- **挂载**：消费方（DE diff 呈现、AG 局部修改呈现）以 blockId 定位请求挂载 decoration；锚失效回
  定位失败信号（BR-ED-DATA-001）。
- **呈现/复位**：decoration 按消费方信号呈现与复位（如 DE 终态 accepted/rejected/expired 后复位
  该 diff 的 decoration）；复位不残留视觉痕迹。
- **纯呈现**：decoration 不修改文档模型与落盘内容；用户在 decoration 上的交互（接受/拒绝按钮等）
  转达给消费方裁决，ED 不代裁。
- **唯一通道**：上层不得绕过装饰层与编辑接口直接操纵编辑器内部文档状态（X-FORBID-005）。

<!-- CHAIN
chain_id: ED-DECOR
主责模块: ED
status: active
risk_tier: P1
-->

<!-- RULE
rule_id: BR-ED-STATE-002
主链路: ED-DECOR
域: STATE
-->
编辑器装饰层必须满足：按 blockId 定位挂载且锚失效回明确失败信号；按消费方（DE/AG）信号呈现与
复位，消费终态后不残留；decoration 自身不修改文档模型与落盘内容，用户在 decoration 上的裁决交互
转达消费方处理，ED 不代行裁决（diff 终态归 DE 终态机）。inline-assist/autocomplete 的业务语义
归 AG（2026-06-11 人类确认，A-SYS-M-S-01 §3），本规则只约束其呈现承载面。

## §8 跨模块约束（CONSTRAINT）

<!-- CONSTRAINT
constraint_id: X-FORBID-005
摘要: 上层模块（DE/AG/REF 等）不得绕过 ED 装饰层与编辑接口直接操纵编辑器内部文档状态；编辑器文档模型的修改必经 ED 受控接口
涉及模块: ED, DE, AG, REF
链路影响: ED-EDIT, ED-DECOR
-->

**复用既有约束，不重复定义**：

- **X-SEQ-002**（A-SYS-C-T-01）：ED 前端经 Service→command 分层触达后端，不跨层直连 FS/DB/网络。
- **X-FORBID-004**（A-WS-M-T-01）：ED 发起的内容/媒体读写目标路径不越出 workspace root。
- **X-SHARED-001**（A-SYS-I-P-01）：blockId 等跨模块传递结构经 SYS 契约层定义。
- **X-SEQ-003 / X-INTER-001**（A-SYS-M-S-01）：编辑会话为状态化核心链，SM-ED-01 先建机并登记总图；
  ED↔WS 脏态/保存转移点登记 X-INTER-001。

## §9 需求到规则映射（REQ_MAP）

<!-- REQ_MAP
req_id: REQ-ED-001
target_type: CHAIN
target_id: ED-EDIT
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-001
target_type: RULE
target_id: BR-ED-STATE-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-001
target_type: RULE
target_id: BR-ED-DATA-002
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-001
target_type: RULE
target_id: BR-ED-STATE-003
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-002
target_type: RULE
target_id: BR-ED-DATA-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-003
target_type: RULE
target_id: BR-ED-DATA-003
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-004
target_type: RULE
target_id: BR-ED-DATA-003
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-005
target_type: CHAIN
target_id: ED-DECOR
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-005
target_type: RULE
target_id: BR-ED-STATE-002
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-005
target_type: CONSTRAINT
target_id: X-FORBID-005
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-006
target_type: RULE
target_id: BR-ED-DATA-004
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-007
target_type: RULE
target_id: BR-ED-DATA-005
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-ED-008
target_type: RULE
target_id: BR-ED-DATA-002
status: mapped
-->

## §10 测试与验收

- **会话生命周期**（BR-ED-STATE-001 / SM-ED-01，REQ-ED-001 验收）：md/docx 可打开/编辑/关闭；
  脏态关闭须保存或确认丢弃；解析失败降级只读；断网时编辑可用；ED 无直接文件 I/O 调用路径（静态可查）。
- **blockId 稳定**（BR-ED-DATA-001，REQ-ED-002 验收）：普通编辑后既有 blockId 不变；删除块后定位
  该 blockId 收到失效信号；不存在静默错位。
- **md/docx 统一模型**（BR-ED-DATA-002，REQ-ED-003 联合 DOC 验收）：docx 进入编辑与往返保存经 DOC
  转换；ED 代码无 docx 二进制处理（静态可查）。
- **呈现层纯化**（BR-ED-DATA-003，REQ-ED-003/004 验收）：分页开/关与重排不改文档模型、不产生脏态；
  媒体资源越界路径被拒（X-FORBID-004）。
- **装饰层契约**（BR-ED-STATE-002 / X-FORBID-005，REQ-ED-005 验收）：decoration 按 blockId 挂载、
  按信号呈现/复位且终态不残留；decoration 不改文档模型；不存在绕过 ED 接口直改编辑器状态的调用
  路径（与 DE/AG G4 集成验证）。
- 量化 NFR（编辑响应延迟、大文档分页性能）延 G5 标定（APC §9，ED-06）。

## §11 技术债务与盲区（诚实声明）

### §11.1 选型记录（2026-06-11，人类授权 AI 推荐，旧版仓库实证取材）

> 人类指示「参考旧版 binder-project 后由 AI 推荐方案」。以下推荐基于旧版仓库实证
> （package.json、BlockIdExtension.ts、canonical_html.rs、R-ED-M-R-01 v2.0、R-ED-M-R-23），
> 原料不为权威，按 APC §8 经本文重新声明为推荐选型；实现期验证后转正式锁定。

- **TipTap 选型（解除 ED-01）**：TipTap **v3 主版本系**（旧版实证 3.11–3.13 运行良好）+
  `@tiptap/react` + StarterKit + 扩展集基线（table 族 / image / link / text-style / color /
  font-family / text-align / underline / sub·superscript / task-list / highlight /
  character-count / placeholder）；分页移植旧版本地扩展 `tiptap-pagination-plus`。
  次版本号在实现期 lock（锁主版本 v3，避免 v2↔v3 API 断裂）。
- **blockId 算法（解除 ED-02 算法部分）**：沿用旧版实证机制——TipTap Extension 为块级节点
  （paragraph/heading/blockquote/codeBlock/listItem 等）维护 `blockId` 属性（DOM 持久化为
  `data-block-id`）；生成 `block_` + `crypto.randomUUID()`；经 ProseMirror `appendTransaction`
  自动补缺；**分裂去重规则**：split 复制导致重复 blockId 时，文档序首个保留原 id、后续重复者
  重新分配（与 §4「普通编辑不变」语义一致：原块保 id、新块得新 id）。转换层兜底：canonical 化
  时为缺失 `data-block-id` 的块注入（旧版 `inject_missing_data_block_ids` 模式，归 DOC/WS
  转换缓存层）。**md 源文件不注入 blockId**（不污染纯文本源），md 的跨会话锚定经内容哈希辅助
  重建——此点为推荐而非实证，实现期与 DE 定位联合验证后转锁定。
- **docx 样式子集（解除 ED-04/ED-05 的 ED 侧）**：采纳旧版三级子集契约（R-ED-M-R-23 实证）——
  **必需子集**（标题 H1-H6 / 段落对齐 / 粗斜下 / 有序无序列表 / 基础表格 / 嵌入图片，实证复现率
  ≥95%）为编辑模型承诺承载；**部分支持**（文本颜色 / 字体 / 字号，Pandoc+Lua 实证 30–50%）尽力
  保留并对用户明示；**明确不处理**（页眉页脚 / 文本框 / SmartArt / VBA 宏 / 复杂样式继承链 /
  合并单元格等复杂表格格式）。完整往返保真由 PRV 预览（LibreOffice→PDF 全保真）补偿查看需求。
  转换管线归 DOC（DOC L2 为该子集契约的执行 SSOT，本文为编辑模型侧承载声明）。

### §11.2 仍开放的债务

- **decoration 与 AG 局部修改的业务契约**：归属已确认 AG（2026-06-11），具体接口契约待 AG G2
  联合承接。
- ~~blockId 跨 docx 往返的保持（待实测）~~ → **已实测（PI-ED-09，2026-07-03）：保留率 0%，不达**——
  Pandoc `html→docx` 侧不落自定义属性（`data-block-id` 全丢），回读全量重建。**按既定降级执行**：
  兜底重建（前端 `reconcileBlockIds` appendTransaction 补缺已等效承载，重开后全量重发 id）。
  **契约语义锁定**：blockId/字符锚**有效期 = 编辑会话内**（保存不打断内存模型、id 持续有效；
  关闭重开必换锚）——与 2026-07-03 人类裁定 D-2 一致，agent 跨重开须重新取块（ag.list_blocks）。
  附带实测发现（DOC 侧既有"部分支持"债务佐证，登记不扩范围）：保存侧 color/text-align 内联样式
  衰减、`data-custom-style` div 包裹残留。
- 分页光标/遮罩具体实现：移植 `tiptap-pagination-plus` 后按新架构适配，实现期敲定。
- §11.1 各推荐选型均待实现期验证后转正式锁定；验证不符须回写本文并复核 DE/DOC 依赖方。

## §12 后续动作

1. 本文落地后随下一次 `npm run governance:generate` 注册：ED-EDIT / ED-DECOR 链路、
   BR-ED-STATE-001/002、BR-ED-DATA-001/002/003 规则、X-FORBID-005 约束、TERM-ED-001/002 术语、
   REQ-ED-001~005 的 REQ_MAP 进 ADUS。
2. 续 ED G2：A-ED-M-S-01 定义 SM-ED-01（文档编辑会话机），登记 A-SYS-M-S-01 总图与 X-INTER-001
   转移点（ED.edit→SM-WS-01 Idle→Dirty；SM-WS-01 Saved→ED 回 Ready）。
3. 待人类宣布 ED G2 闭合（AI 不自行宣布晋级，APC §2）；DOC G2 时联合解除 ED-04/样式子集与 blockId
   往返保持；DE/AG G2 时承接 decoration 消费契约。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-11 | v1.0 | 初版（ED G2 首件）：ED 规则 SSOT——编辑会话与脏态（经 WS、离线可用、解析失败降级只读）、内容块模型与稳定 blockId（失效可感知）、md/docx 统一编辑模型（二进制归 DOC）、分页与媒体呈现层纯化、编辑器装饰层接口契约（DE/AG 消费）；注册链路 ED-EDIT / ED-DECOR（均 P1）、规则 BR-ED-STATE-001/002·BR-ED-DATA-001/002/003、约束 X-FORBID-005（复用 X-SEQ-002/003·X-FORBID-004·X-SHARED-001·X-INTER-001）、术语 TERM-ED-001 Block / TERM-ED-002 EditorDecoration；REQ_MAP（REQ-ED-001~005）；债务（TipTap 选型/blockId 算法/样式子集/往返保持延后并记明 owner） |
| 2026-06-11 | v1.1 | 人类决策落地：inline-assist/autocomplete 归属确认 AG（边界表述更新）；§11.1 新增选型记录（人类授权 AI 推荐，旧版仓库实证取材）——TipTap v3 系 + React + StarterKit + 扩展集基线 + tiptap-pagination-plus 移植（解除 ED-01）、blockId = TipTap Extension + block_+UUID + appendTransaction 补缺/分裂去重 + 转换层兜底注入、md 源不注入（解除 ED-02 算法部分）、docx 样式子集三级契约（必需/部分/不处理，执行 SSOT 归 DOC L2）；§11.2 余债收敛（往返保留度/分页适配/验证后转锁定） |
| 2026-06-12 | v1.2 | Q2 缺口补遗（人类确认采纳）：§3 新增 BR-ED-STATE-003 外部变更会话处理——无脏态自动重载、有脏态/保存冲突进入显式裁决（保留己方→覆盖确认 / 放弃重载），双向内容损失均须用户显式选择；REQ_MAP 补 REQ-ED-001→BR-ED-STATE-003 |
| 2026-07-03 | v1.3 | §11.2 实测回填（PI-ED-09，A-ED-X-L-02 批B 地基）：blockId 跨 docx 往返保留率 0%（Pandoc 不落自定义属性）→ 按既定降级走兜底重建（reconcileBlockIds 补缺）；契约语义锁定「锚有效期 = 编辑会话内、重开必换锚」（与裁定 D-2 一致）；附带登记保存侧样式衰减与 data-custom-style 残留（DOC 侧既有债务佐证） |
| 2026-07-03 | v1.4 | 批B 契约注册（A-ED-X-L-02 §4，人类裁定 D-1/D-2/D-3）：§0 注册 TERM-ED-004 CharAnchor / TERM-ED-005 SelectionRef / TERM-ED-006 StyleOp；§4.1 字符级锚定与选区引用 + BR-ED-DATA-004（会话内字符锚、纯文本偏移平面、失效可感知）；§5 扩展为 md/docx/html 统一模型 + BR-ED-DATA-002 修订（.html 文本路径不经 DOC）；§5.1 样式投影与操作协议 + BR-ED-DATA-005（可读投影 + StyleOp 受控写 + D-3 落盘分流）；REQ_MAP 补 REQ-ED-006→004 / 007→005 / 008→002 |
| 2026-07-04 | v1.5 | 术语统一清扫（承 2026-07-04 人类三轮裁决）：正文禁用别名替换——内联辅助→局部修改、自动补全→辅助续写、对话编辑→Diff Edit、Chat↔Build 接管→构建移交/回落、chat/chat-agent 回合→代理回合；文件名与变更记录历史行不改写 |
