---
文档编号：AG-M-S-01
文档状态：A
负责模块：AG
文档职责：AG 代理模块级状态机设计——代理回合机（SM-AG-01）、工具调用循环机（SM-AG-02）、辅助回合机（SM-AG-03，承载 autocomplete 与 inline-assist），含 APC §6 必备字段；回填 A-SYS-M-S-01 总图四条 planned 行，登记 AG↔DE / AG↔ED / AG↔SYS 门禁 X-INTER-001 转移点
上游约束：A-AG-M-T-01_代理技术设计.md（AG-TURN/AG-TOOL/AG-ASSIST 链路、BR-AG-* 规则）、A-CORE-C-P-01_APC.md（§6）、A-SYS-M-S-01_状态机总图.md（X-SEQ-003/X-INTER-001）、A-SYS-I-P-03_云接口契约.md（SM-SYS-01 门禁段）、A-SYS-I-P-02_IPC协议.md（BR-SYS-STATE-002 流式终态）、A-ED-M-S-01_编辑器状态机设计.md（SM-ED-01 edit 入口）、ADU.md（§3.1）
直接承接：三机经 rules 字段承载 A-AG-M-T-01 规则；machine_id 登记总图；产出交 DE 转移点供 DE G2 对接
使用边界：本文定义 AG 状态机内部；不重定义 SM-SYS-01（门禁裁决归 SYS，本文以转移点对接）、不定义 DE diff 终态机（待 DE G2）、不定义 ED/WS 机内部；构建移交/回落状态机对接已双侧闭合（§2 takeover/fallbackIn + A-CBT-M-S-01 §3）
变更要求：transitions 变更须同步 A-AG-M-T-01 规则并重跑测试矩阵；跨模块转移点变更须与对侧 owner 共同复核；machine_id 一经登记不可更改
治理成熟度：draft
---

# Binder 代理状态机设计（AG · L2 技术设计）

## §1 文档定位

```
A-AG-M-T-01（规则 SSOT）→ 本文 SM-AG-01/02/03
本文 machine_id ──登记──→ A-SYS-M-S-01 §3（代理回合 / tool-call / autocomplete / inline-assist 四行转 active）
转移点：AG↔SM-SYS-01（门禁）、AG 产出↔DE（待 DE G2）、SM-AG-03 采纳↔SM-ED-01（双侧就绪）
```

满足 X-SEQ-003：四条状态化核心链（APC §6 列举 + 2026-06-11 归属决策）在此建机。
autocomplete 与 inline-assist 由 **SM-AG-03 同机承载**（二者生命周期同构：触发→节流→门禁→
轻量调用→装饰层呈现→采纳/弃，区别仅在触发源与候选形态，归 context 字段），总图两行回填同一
machine_id 并注明。

## §2 SM-AG-01 · 代理回合机

<!-- STATE_MACHINE
machine_id: SM-AG-01
owner_module: AG
status: active
summary: 一次代理回合的完整生命周期（组装→门禁→流式→产出交 DE；可中断、无半落盘）
source_type: design
chain: AG-TURN
rules: BR-AG-STATE-001, BR-AG-VERIFY-001, BR-AG-DATA-001, BR-AG-PERSIST-001
initial_state: Idle
terminal_states: Completed, Cancelled, Denied, HandedOver
-->

- **machine_id**：SM-AG-01　**owner_module**：AG
- **scope**：一次代理回合（用户发起→产出处置完成/中断/拒绝/移交构建；目标边界按 BR-AG-DATA-003 2026-07-04 版）
- **context**：回合输入（session_id + prompt + 注入载荷 + open_doc_snapshots）、AgentReadContext、组装后上下文、门禁结果、流式缓冲、产出（回复/修改建议）、移交上下文、历史持久化状态（completed/error/cancelled/gate_rejected/draft）
- **states**：Idle、Assembling、AwaitingGate、Streaming、HandingOff、Completed、Cancelled、Denied、TurnError、**HandedOver**
- **events**：send、assembled、assembleFail、gatePassed、gateDenied、streamEnd、streamFail、cancel、handoffOk、handoffFail、retry、**takeover、fallbackIn**
- **transitions（主表）**：

| from | event | guard | to | action |
|---|---|---|---|---|
| Idle | send | open_doc_snapshots 覆盖全部已打开文档且每项含 session_version/content_hash；active live 序列化成功 | Assembling | 写入/更新 AgentSession；创建 AgentTurn `draft` payload，记录 userMessage（BR-AG-PERSIST-001）；构建 AgentReadContext；组装上下文（BR-AG-DATA-001：限界/裁剪优先级） |
| Idle | fallbackIn | CBT 构建完成/中断回落（cbt.fallback，X-INTER-001） | Assembling | 以回落上下文起新 代理回合；回 fallbackAck（无双死） |
| Assembling | assembled | 已打开文档引用和 selection 均基于 AgentReadContext 校验通过 | AwaitingGate | 提交 SM-SYS-01 门禁（X-INTER-001 转移点） |
| Assembling | assembleFail | OpenDocSnapshot 缺失/版本缺失/active live 序列化失败/注入锚失效 | TurnError | 统一错误形状；不得用 DiskState 或 loadedContent 旧快照兜底 |
| AwaitingGate | gatePassed | SM-SYS-01 达 GatePassed | Streaming | 经 AiGateway 发起流式调用 |
| AwaitingGate | gateDenied | SM-SYS-01 GateDenied / 断网 | Denied | 明确拒绝（终态，不可绕过）；AgentTurn status=`gate_rejected` |
| Streaming | streamEnd | — | HandingOff | 产出处置：修改建议交 DE，纯回复直接完成 |
| Streaming | takeover | 构建触发（2026-07-04 判据换轴，BR-CBT-STATE-002）：用户显式触发（明确指令/构建按钮，触发即视为确认）或 AI 识别**项目级构建意图**提议经用户确认；多文件修改本身不再是判据 | HandedOver | 移交 CBT（cbt.takeover，上下文含项目蓝图随移交）；回 takeoverAck；本回合移交终结（无双活）；构建在原对话窗口呈现 |
| Streaming | streamFail | — | TurnError | 统一错误形状（含可重试标记）；AgentTurn status=`error` |
| Streaming | cancel | — | Cancelled | 中断终态；无半落盘副作用（BR-SYS-STATE-002）；AgentTurn status=`cancelled` |
| HandingOff | handoffOk | 修改建议已交 DE 生成 pending_diff / 无修改建议 | Completed | AgentTurn status=`completed`；异步触发记忆写入（BR-AG-STATE-004） |
| HandingOff | handoffFail | — | TurnError | DE 生成失败（如锚失效）回报，可重试；AgentTurn status=`error` |
| TurnError | retry | — | Assembling | 重试回合 |

- **error_states**：TurnError（recovery：retry→Assembling）
- **recovery_paths**：TurnError→retry；Denied 不可重试绕过（须恢复订阅/网络后重新 send）
- **forbidden_transitions**：
  - Idle / Assembling → Streaming（未过门禁不得调用，对齐 SM-SYS-01 forbidden）
  - Denied → Streaming（被拒不得绕过门禁，终态）
  - Streaming → Completed（产出含修改建议时必经 HandingOff 交 DE，不得跳过）
  - Assembling → AwaitingGate 当已打开文档缺少 OpenDocSnapshot 或引用锚无法校验（不得以 DiskState 兜底进入模型）
  - 任意态 → 直接写盘（本机无写盘动作，X-FORBID-006）
  - takeover 无显式触发/确认（构建移交须用户显式触发或确认提议，BR-CBT-STATE-002）
  - HandedOver → 任意活动态（移交终结后本回合不得继续，无双活）
- **persistence_projection**：`send` 后先持久化 `draft`；`handoffOk`→`completed`；
  `streamFail/handoffFail/命令级错误`→`error`；`cancel`→`cancelled`；`gateDenied`→`gate_rejected`；
  应用崩溃/关闭/流式中断未收到终态时保持 `draft`，重启后只能显示未完成/可重试。
- **acceptance_criteria**：不存在不经 gatePassed 通往 Streaming 的路径；Denied 为终态且不可达
  Streaming；修改建议唯一出口经 HandingOff 交 DE；cancel 后无半落盘副作用；takeover 须显式触发
  或经确认且移交后本回合终结（HandedOver 终态，无双活）；fallbackIn 起新回合时回 fallbackAck（无双死）
  ；completed/error/cancelled/gate_rejected/draft 五类历史状态均可重启恢复。
- **test_matrix**：见 §5
- **构建移交/回落（旧称 Chat↔Build 接管；B-3 补齐 2026-06-12，判据 2026-07-04 换轴）**：takeover
  （显式触发或项目级构建意图经确认→HandedOver 终态，移交 CBT，蓝图随移交）/ fallbackIn（CBT 构建
  完成/中断回落起新回合）与 SM-CBT-01（A-CBT-M-S-01 §3）双向回执对接（takeoverAck/fallbackAck），
  登记 A-SYS-M-S-01 §5。构建执行期间（SM-CBT-01 活动态）不得发起新代理回合（仅中断），由前端
  按 X-FORBID-007 投影门控。

## §3 SM-AG-02 · 工具调用循环机

> 本机嵌套运行于 SM-AG-01 的 Streaming 段（模型输出含工具请求时激活），共享其回合上下文与
> 取消信号；循环结束回归 SM-AG-01 流式终态路径。

<!-- STATE_MACHINE
machine_id: SM-AG-02
owner_module: AG
status: active
summary: 回合内工具调用循环（注册/权限校验→执行→回注→循环控制；写盘禁令、上限、可中断）
source_type: design
chain: AG-TOOL
rules: BR-AG-STATE-002, BR-AG-VERIFY-001
initial_state: ModelStreaming
terminal_states: LoopCompleted, LoopAborted
-->

- **machine_id**：SM-AG-02　**owner_module**：AG
- **scope**：一次代理回合内的工具调用循环（首个工具请求出现→终轮答复/中止）
- **context**：工具注册清单、权限边界、AgentReadContext、循环计数与上限、当前工具请求/结果、取消信号
- **states**：ModelStreaming、ToolValidating、ToolExecuting、ResultInjecting、LoopCompleted、LoopAborted
- **events**：toolRequest、validateOk、validateFail、execOk、execFail、injected、finalAnswer、loopLimit、cancel
- **transitions（主表）**：

| from | event | guard | to | action |
|---|---|---|---|---|
| ModelStreaming | toolRequest | — | ToolValidating | 校验注册清单与权限边界 |
| ModelStreaming | finalAnswer | — | LoopCompleted | 终轮答复，回归 SM-AG-01 streamEnd |
| ToolValidating | validateOk | 工具已注册且在权限边界内 | ToolExecuting | 执行（只读经 AgentReadContext：已打开文档读 LogicalState，未打开文档才读 DiskState；写意图产出 diff 建议不直写） |
| ToolValidating | validateFail | 未注册/越权 | ResultInjecting | 拒绝原因回注模型 |
| ToolExecuting | execOk | — | ResultInjecting | 结果回注 |
| ToolExecuting | execFail | — | ResultInjecting | 失败信息回注（不中断循环） |
| ResultInjecting | injected | 循环计数 < 上限 | ModelStreaming | 续调模型（仍在已过门禁的回合内） |
| ResultInjecting | loopLimit | 达循环上限 | LoopAborted | 中止并回报（统一错误形状） |
| 任意非终态 | cancel | 用户中断 | LoopAborted | 中断终态，无半落盘 |

- **error_states**：无独立错误终态——工具失败回注模型继续循环（execFail→ResultInjecting）；
  循环级失败统一走 LoopAborted 回归 SM-AG-01 错误/中断路径
- **recovery_paths**：LoopAborted→SM-AG-01 TurnError/Cancelled 路径处置
- **forbidden_transitions**：
  - ToolValidating → ToolExecuting 无 validateOk（未注册/越权不得执行）
  - ToolExecuting → 直接文件写入（写意图必须产出 diff 建议，X-FORBID-006）
  - ToolExecuting → DiskState 读取，当目标文档已打开且 AgentReadContext 中应存在 OpenDocSnapshot
  - ResultInjecting → ModelStreaming 当循环已达上限（不得无限循环）
- **acceptance_criteria**：未注册/越权工具不可达 ToolExecuting；循环上限强制生效；任何工具执行
  不产生直接文件写入；read_doc/search_text/list_blocks 对已打开文档命中 OpenDocSnapshot，缺失时拒绝；
  cancel 随时可达 LoopAborted
- **test_matrix**：见 §5

## §4 SM-AG-03 · 辅助回合机（autocomplete + inline-assist 同机承载）

<!-- STATE_MACHINE
machine_id: SM-AG-03
owner_module: AG
status: active
summary: 辅助续写/局部修改回合（快捷键手动触发→防抖/latest-wins→门禁→轻量调用→装饰层呈现→采纳即编辑；辅助续写 3 候选；局部修改可多轮；失败静默）
source_type: design
chain: AG-ASSIST
rules: BR-AG-STATE-003, BR-AG-VERIFY-001
initial_state: Idle
terminal_states: Accepted, Dismissed
-->

- **machine_id**：SM-AG-03　**owner_module**：AG
- **scope**：一次局部修改或辅助续写回合（触发→候选处置）；触发源与候选形态归 context
  （assist_kind: autocomplete / inline-assist），生命周期同构
- **context**：assist_kind、requestId（latest-wins）、触发上下文（DocRef、编辑位置/选区、目标
  blockId、字符锚、原文本/前缀文本、前后文指纹）、防抖窗口（默认 450ms）、门禁结果、候选（autocomplete=3 candidates；
  inline-assist=preview）、多轮基底（局部修改：上轮候选产出）
- **states**：Idle、Throttling、Requesting、Presenting、Accepted、Dismissed
- **events**：trigger、throttlePass、throttleDrop、candidateOk、candidateUnderflow、requestFail、accept、dismiss、anchorLost、reposition、**refine**
- **transitions（主表）**：

| from | event | guard | to | action |
|---|---|---|---|---|
| Idle | trigger | 用户快捷键手动触发（2026-07-04 澄清：非键入自动触发） | Throttling | 进入防抖/防重判定（BR-AG-STATE-003），捕获 DocRef/blockId/字符锚/前后文指纹 |
| Throttling | trigger | 新请求到达 | Throttling | 更新 requestId 与锚点，旧请求标记 superseded |
| Requesting | trigger | 新请求到达 | Throttling | 更新 requestId 与锚点，旧响应返回后丢弃 |
| Presenting | trigger | 新请求到达/继续输入 | Throttling | 复位旧装饰，更新 requestId 与锚点 |
| Throttling | throttleDrop | 防抖窗口内重复触发且不替换最新请求 | Idle | 丢弃本次触发（静默） |
| Throttling | throttlePass | 防抖通过 且 门禁可用（SM-SYS-01 GatePassed） | Requesting | 经网关轻量调用 |
| Throttling | throttlePass | 防抖通过 但 无订阅/断网 | Idle | 静默不提供，不打断编辑 |
| Requesting | candidateOk | requestId 为最新 且 文档/blockId/字符锚/前后文指纹仍有效；autocomplete 候选数=3 | Presenting | 候选经 ED 装饰层/overlay 语义呈现（ED-DECOR） |
| Requesting | candidateOk | requestId 已过期 或 锚失效 | Dismissed | 丢弃候选，不展示不应用 |
| Requesting | dismiss | 用户点击浮层外、移动光标或改变选区 | Dismissed | 关闭浮层；后续响应按旧 requestId 丢弃，不展示不应用 |
| Requesting | candidateUnderflow | autocomplete 经容错解析后候选不足 3 条（failure_reason=parse_underflow） | Presenting | 不展示半候选、不允许采纳；保留轻量空态，等待用户 dismiss 或移动锚点关闭 |
| Requesting | requestFail | — | Idle | 静默（不弹错误、不打断编辑） |
| Presenting | **refine** | assist_kind=inline-assist（局部修改多轮，2026-07-04 澄清） | Requesting | 以上轮候选产出为基底携新指令续调（仅传上一轮结果；每轮逐次过门禁；对话不持久化） |
| Presenting | reposition | 滚动/缩放/窗口 resize 且逻辑锚仍有效 | Presenting | 基于当前缩放后的视觉锚点与静态浮层尺寸重算浮层位置；浮层 UI 尺寸不随正文缩放 |
| Presenting | reposition | 逻辑锚结构失效 | Dismissed | 关闭浮层并回写 anchorLost |
| Presenting | accept | — | Accepted | 采纳=用户编辑操作：经 ED 受控接口进入编辑模型，驱动 SM-ED-01 edit（X-INTER-001） |
| Presenting | dismiss | 用户忽略/关闭弹窗、点击浮层外、移动光标或改变选区 | Dismissed | 装饰复位不残留；已返回内容丢弃，不入历史 |
| Presenting | anchorLost | blockId 失效 | Dismissed | 放弃呈现，装饰复位 |

- **error_states**：无显式错误终态——辅助回合失败不弹错、不打断；gate_denied/gate_offline/gateway_error/empty_output 静默回 Idle，parse_underflow 经 candidateUnderflow 进入轻量空态 Presenting 以避免触发后瞬间消失
- **recovery_paths**：失败静默回 Idle，下次触发自然重试
- **forbidden_transitions**：
  - Idle / Throttling → Requesting 无门禁可用（不经门禁不得调用，BR-AG-VERIFY-001）
  - Requesting / Presenting → 直接修改文档模型（必经 accept 走 ED 受控接口，X-FORBID-005）
  - Idle → Presenting（未经调用不得呈现）
  - 过期 requestId 的 candidateOk → Presenting（旧请求不得覆盖新请求）
- **acceptance_criteria**：重复触发被防抖（默认 450ms 内不产生多次调用）；无订阅/断网/失败均静默且
  编辑不受影响；辅助续写严格呈现 3 候选并按 Tab/1/2/3 采纳，候选不足不展示半候选且保留空态；新请求覆盖旧请求，旧响应不得展示或应用；
  局部修改选区态替换、光标态插入；refine 每轮均过门禁且只传上一轮候选结果，回合不入历史（不持久化）；
  编辑器缩放、滚动、窗口 resize 不改变逻辑锚，只重算浮层视觉锚；浮层 UI 尺寸不随正文缩放；
  浮层使用静态尺寸模型，避免运行时测量反向驱动状态；局部修改初始态尺寸不得过大导致远离锚点；
  用户点击浮层外、移动光标或改变选区时，Requesting/Presenting 均关闭并丢弃已返回/后续返回内容，不展示、不应用、不入历史；
  accept 产生的变更走 SM-ED-01→SM-WS-01 正常脏态/保存路径；accept/dismiss/anchorLost 均回写终态；
  终态后装饰复位不残留
- **test_matrix**：见 §5

## §5 测试与验收（强制转移测试矩阵，APC §6/§10）

- **SM-AG-01**：send→Assembling→AwaitingGate→gatePassed→Streaming→streamEnd→HandingOff→Completed
  正路；gateDenied→Denied 终态且不可达 Streaming；cancel→Cancelled 无半落盘；handoffFail/
  streamFail→TurnError→retry；禁止 Idle/Assembling→Streaming 旁路；持久化投影覆盖
  completed/error/cancelled/gate_rejected/draft，draft 重启后不得显示为完成态；已打开文档缺
  OpenDocSnapshot 或 selection 校验失败时 assembleFail，不得进入 AwaitingGate。
- **SM-AG-02**：toolRequest→校验→执行→回注→续调循环；validateFail 回注拒绝；loopLimit 强制
  中止；cancel 随时中止；断言无任何工具执行产生直接文件写入（与 DE/WS G5 联合验证）；
  read_doc/search_text/list_blocks 已打开文档覆盖 LogicalState，未打开文档读取 DiskState。
- **SM-AG-03**：重复 trigger 下 throttleDrop 或 latest-wins 覆盖生效；无订阅/断网 throttlePass 后静默回
  Idle；过期 requestId 的 candidateOk 被丢弃；候选不足 3 条 candidateUnderflow 且记录 parse_underflow；candidateOk→Presenting→accept→Accepted
  驱动 SM-ED-01 edit；辅助续写 3 候选采纳；局部修改光标插入；Presenting→refine→Requesting 多轮循环（只传上一轮
  候选结果，每轮过门禁、不持久化）；anchorLost/dismiss 回写终态且复位不残留；禁止未经门禁到 Requesting。
- **跨模块（G5 联调）**：AG↔SM-SYS-01 门禁转移、HandingOff↔DE（待 DE G2 双侧）、
  SM-AG-03 accept↔SM-ED-01 edit（双侧就绪）。
- 量化 NFR（节流参数、延迟）延 G5 标定。

## §6 总图登记（A-SYS-M-S-01）

| 核心链 | machine_id | 总图回填动作 |
|---|---|---|
| 代理回合 | SM-AG-01 | §3 行 planned→active |
| tool-call | SM-AG-02 | §3 行 planned→active |
| autocomplete | SM-AG-03 | §3 行 planned→active（与 inline-assist 同机承载，context 区分） |
| inline-assist | SM-AG-03 | §3 行 planned→active（同上） |
| 转移点 | — | §5 X-INTER-001 登记：AG↔门禁（SM-SYS-01）、AG 产出→DE（DE 侧待 DE G2）、SM-AG-03 采纳→SM-ED-01（双侧就绪） |

## §7 术语归属说明

代理回合/工具调用/局部修改/辅助续写 TERM 已在 A-AG-M-T-01 §0 注册；订阅门禁等云术语归 SYS
（TERM-CLOUD-001~004）；diff 主责归 DE。状态机框架词不注册。

## §8 技术债务与盲区（诚实声明）

- **AG 产出→DE 转移仅单侧**：HandingOff 的 AG 侧契约已定（修改建议+定位锚交 DE，回执 handoffOk/
  Fail）；DE 侧 diff 终态机待 DE G2，双侧完整对接与 G5 联调在 DE G2 后闭合。
- ~~构建移交/回落未建模~~ **已建模（B-3，2026-06-12）**：SM-AG-01 增 takeover/fallbackIn 转移与
  HandedOver 终态，与 SM-CBT-01 双向回执对接，登记总图 §5。原始记录留痕——
- **构建移交/回落（历史债务，已解）**：SM-AG-01 与 CBT 构建机的接管/中断对接（总图 §5 既有转移点）联合
  CBT G2 补登，本文不预定义接管协议。
- **SM-AG-02 嵌套语义**：与 SM-AG-01 Streaming 段的嵌套激活/取消传播在实现期细化（单实例语义
  已锁，嵌套调度属实现）。
- 节流窗口、循环上限数值延 G5/运营标定（机制已锁）。

## §9 后续动作

1. 落地后同步编辑 A-SYS-M-S-01：§3 四行 planned→active（SM-AG-01/02/03），§5 登记转移点。
2. 随 `npm run governance:generate` 复跑核对。
3. 待人类宣布 AG G2 闭合；DE G2 完成 AG 产出→DE 双侧对接；CBT G2 补登接管转移点。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-11 | v1.0 | 初版（AG G2 状态机件）：定义 SM-AG-01 代理回合机（组装/门禁/流式/HandingOff 交 DE，Denied 终态不可绕过）、SM-AG-02 工具调用循环机（校验/执行/回注/上限/写盘禁令，嵌套于回合 Streaming 段）、SM-AG-03 辅助回合机（autocomplete 与 inline-assist 同机承载：节流/门禁/装饰层/采纳即编辑/失败静默），均含 APC §6 必备字段；满足 X-SEQ-003；转移点登记（门禁/交 DE 单侧/采纳→SM-ED-01 双侧）；债务（DE 侧待 G2、CBT 接管未建模、嵌套调度与参数延实现期） |
| 2026-07-04 | v2.0 | 双模式定义回写（人类裁决）：①SM-AG-01 takeover guard 判据换轴——「识别多文件构建意图」废止，改「构建触发：显式触发（指令/构建按钮，触发即确认）或项目级构建意图提议经确认」，移交携项目蓝图、构建在原对话窗口呈现；「Chat↔Build 接管」更名「构建移交/回落」；新增构建执行期禁发起新回合（前端 X-FORBID-007 投影门控）；②SM-AG-03：触发语义改快捷键手动+防抖/防重（原「节流」为自动触发遗留），新增 Presenting→refine→Requesting 多轮转移（局部修改，每轮过门禁、不持久化），test_matrix 同步 |
| 2026-07-16 | v2.1 | 细化 SM-AG-03：引入 requestId latest-wins、新请求覆盖旧请求、过期响应丢弃；辅助续写 3 候选；局部修改光标态插入，多轮只传上一轮候选结果；candidateOk 与 accept 前均以文档/block/字符锚有效为守卫 |
| 2026-07-16 | v2.2 | 续修 SM-AG-03：补默认 450ms 防抖、触发锚前后文指纹、候选不足 3 条静默失败、accept/dismiss/anchorLost 前端终态回写 |
| 2026-07-16 | v2.3 | 补充 SM-AG-03 交互锚失效投影：Requesting/Presenting 状态下点击浮层外、移动光标或改变选区均走 dismiss 终态，关闭浮层并丢弃已返回/后续返回内容 |
| 2026-07-16 | v2.4 | 补充 SM-AG-03 reposition 转移：滚动、缩放、窗口 resize 时逻辑锚不变，按当前视觉锚重算浮层位置，结构失效才关闭 |
| 2026-07-16 | v2.5 | 修正 SM-AG-03 稳定定位：撤销浮层 DOM 尺寸变化触发 reposition，改为静态尺寸模型，避免测量链影响浮层显示 |
| 2026-07-16 | v2.6 | 修正 SM-AG-03 辅助续写失败态：候选不足 3 条转为空态 Presenting，不展示半候选、不允许采纳，用户 dismiss 后关闭 |
| 2026-07-17 | v2.7 | 加固 SM-AG-03 失败分型：gate/网关/空输出静默回 Idle，parse_underflow 进入轻量空态；failure_reason 仅用于内部诊断 |
| 2026-07-17 | v2.8 | 清理 SM-AG-03 状态分叉：候选不足改用 candidateUnderflow 独立事件进入 Presenting，普通 requestFail 仅表示静默回 Idle |
| 2026-07-17 | v2.9 | 补齐 SM-AG-01 历史持久化投影：send 先写 draft，handoffOk→completed，streamFail/handoffFail→error，cancel→cancelled，gateDenied→gate_rejected；崩溃/关闭遗留 draft 重启后不得伪装完成 |
| 2026-07-18 | v2.10 | 加固 SM-AG-01/02 已打开文档门禁：send 阶段必须携 open_doc_snapshots 并构建 AgentReadContext；只读工具对已打开文档禁止 DiskState 兜底 |
