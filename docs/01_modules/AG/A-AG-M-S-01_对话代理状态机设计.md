---
文档编号：AG-M-S-01
文档状态：A
负责模块：AG
文档职责：AG 对话代理模块级状态机设计——代理回合机（SM-AG-01）、工具调用循环机（SM-AG-02）、辅助回合机（SM-AG-03，承载 autocomplete 与 inline-assist），含 APC §6 必备字段；回填 A-SYS-M-S-01 总图四条 planned 行，登记 AG↔DE / AG↔ED / AG↔SYS 门禁 X-INTER-001 转移点
上游约束：A-AG-M-T-01_对话代理技术设计.md（AG-TURN/AG-TOOL/AG-ASSIST 链路、BR-AG-* 规则）、A-CORE-C-P-01_APC.md（§6）、A-SYS-M-S-01_状态机总图.md（X-SEQ-003/X-INTER-001）、A-SYS-I-P-03_云接口契约.md（SM-SYS-01 门禁段）、A-SYS-I-P-02_IPC协议.md（BR-SYS-STATE-002 流式终态）、A-ED-M-S-01_编辑器状态机设计.md（SM-ED-01 edit 入口）、ADU.md（§3.1）
直接承接：三机经 rules 字段承载 A-AG-M-T-01 规则；machine_id 登记总图；产出交 DE 转移点供 DE G2 对接
使用边界：本文定义 AG 状态机内部；不重定义 SM-SYS-01（门禁裁决归 SYS，本文以转移点对接）、不定义 DE diff 终态机（待 DE G2）、不定义 ED/WS 机内部；Chat↔Build 接管状态机对接联合 CBT G2 后补登
变更要求：transitions 变更须同步 A-AG-M-T-01 规则并重跑测试矩阵；跨模块转移点变更须与对侧 owner 共同复核；machine_id 一经登记不可更改
治理成熟度：draft
---

# Binder 对话代理状态机设计（AG · L2 技术设计）

## §1 文档定位

```
A-AG-M-T-01（规则 SSOT）→ 本文 SM-AG-01/02/03
本文 machine_id ──登记──→ A-SYS-M-S-01 §3（chat-agent 回合 / tool-call / autocomplete / inline-assist 四行转 active）
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
- **scope**：单文档一次代理回合（用户发起→产出处置完成/中断/拒绝/移交构建）
- **context**：回合输入（prompt + 注入载荷）、组装后上下文、门禁结果、流式缓冲、产出（回复/修改建议）、移交上下文
- **states**：Idle、Assembling、AwaitingGate、Streaming、HandingOff、Completed、Cancelled、Denied、TurnError、**HandedOver**
- **events**：send、assembled、assembleFail、gatePassed、gateDenied、streamEnd、streamFail、cancel、handoffOk、handoffFail、retry、**takeover、fallbackIn**
- **transitions（主表）**：

| from | event | guard | to | action |
|---|---|---|---|---|
| Idle | send | — | Assembling | 组装上下文（BR-AG-DATA-001：限界/裁剪优先级） |
| Idle | fallbackIn | CBT 构建完成/中断回落（cbt.fallback，X-INTER-001） | Assembling | 以回落上下文起新 chat 回合；回 fallbackAck（无双死） |
| Assembling | assembled | — | AwaitingGate | 提交 SM-SYS-01 门禁（X-INTER-001 转移点） |
| Assembling | assembleFail | — | TurnError | 统一错误形状（如注入锚失效） |
| AwaitingGate | gatePassed | SM-SYS-01 达 GatePassed | Streaming | 经 AiGateway 发起流式调用 |
| AwaitingGate | gateDenied | SM-SYS-01 GateDenied / 断网 | Denied | 明确拒绝（终态，不可绕过） |
| Streaming | streamEnd | — | HandingOff | 产出处置：修改建议交 DE，纯回复直接完成 |
| Streaming | takeover | 识别多文件构建意图 + 用户显式确认 | HandedOver | 移交 CBT（cbt.takeover，上下文随移交）；回 takeoverAck；本回合移交终结（无双活） |
| Streaming | streamFail | — | TurnError | 统一错误形状（含可重试标记） |
| Streaming | cancel | — | Cancelled | 中断终态；无半落盘副作用（BR-SYS-STATE-002） |
| HandingOff | handoffOk | 修改建议已交 DE 生成 pending_diff / 无修改建议 | Completed | 回合落历史（BR-AG-PERSIST-001）；异步触发记忆写入（BR-AG-STATE-004） |
| HandingOff | handoffFail | — | TurnError | DE 生成失败（如锚失效）回报，可重试 |
| TurnError | retry | — | Assembling | 重试回合 |

- **error_states**：TurnError（recovery：retry→Assembling）
- **recovery_paths**：TurnError→retry；Denied 不可重试绕过（须恢复订阅/网络后重新 send）
- **forbidden_transitions**：
  - Idle / Assembling → Streaming（未过门禁不得调用，对齐 SM-SYS-01 forbidden）
  - Denied → Streaming（被拒不得绕过门禁，终态）
  - Streaming → Completed（产出含修改建议时必经 HandingOff 交 DE，不得跳过）
  - 任意态 → 直接写盘（本机无写盘动作，X-FORBID-006）
  - takeover 无用户确认（接管须用户显式确认，BR-CBT-STATE-002）
  - HandedOver → 任意活动态（移交终结后本回合不得继续，无双活）
- **acceptance_criteria**：不存在不经 gatePassed 通往 Streaming 的路径；Denied 为终态且不可达
  Streaming；修改建议唯一出口经 HandingOff 交 DE；cancel 后无半落盘副作用；takeover 须用户确认
  且移交后本回合终结（HandedOver 终态，无双活）；fallbackIn 起新回合时回 fallbackAck（无双死）
- **test_matrix**：见 §5
- **Chat↔Build 接管（B-3 补齐，2026-06-12）**：takeover（Streaming 识别多文件意图 + 用户确认→
  HandedOver 终态，移交 CBT）/ fallbackIn（CBT 回落起新回合）与 SM-CBT-01（A-CBT-M-S-01 §3）双向
  回执对接（takeoverAck/fallbackAck），登记 A-SYS-M-S-01 §5；解除 A-CORE-X-R-01 B-3。

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
- **context**：工具注册清单、权限边界、循环计数与上限、当前工具请求/结果、取消信号
- **states**：ModelStreaming、ToolValidating、ToolExecuting、ResultInjecting、LoopCompleted、LoopAborted
- **events**：toolRequest、validateOk、validateFail、execOk、execFail、injected、finalAnswer、loopLimit、cancel
- **transitions（主表）**：

| from | event | guard | to | action |
|---|---|---|---|---|
| ModelStreaming | toolRequest | — | ToolValidating | 校验注册清单与权限边界 |
| ModelStreaming | finalAnswer | — | LoopCompleted | 终轮答复，回归 SM-AG-01 streamEnd |
| ToolValidating | validateOk | 工具已注册且在权限边界内 | ToolExecuting | 执行（只读经 WS 限界；写意图产出 diff 建议不直写） |
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
  - ResultInjecting → ModelStreaming 当循环已达上限（不得无限循环）
- **acceptance_criteria**：未注册/越权工具不可达 ToolExecuting；循环上限强制生效；任何工具执行
  不产生直接文件写入；cancel 随时可达 LoopAborted
- **test_matrix**：见 §5

## §4 SM-AG-03 · 辅助回合机（autocomplete + inline-assist 同机承载）

<!-- STATE_MACHINE
machine_id: SM-AG-03
owner_module: AG
status: active
summary: 内联辅助/自动补全回合（节流→门禁→轻量调用→装饰层呈现→采纳即编辑；失败静默）
source_type: design
chain: AG-ASSIST
rules: BR-AG-STATE-003, BR-AG-VERIFY-001
initial_state: Idle
terminal_states: Accepted, Dismissed
-->

- **machine_id**：SM-AG-03　**owner_module**：AG
- **scope**：一次内联辅助或自动补全回合（触发→候选处置）；触发源与候选形态归 context
  （assist_kind: autocomplete / inline-assist），生命周期同构
- **context**：assist_kind、触发上下文（编辑位置/选区）、节流窗口、门禁结果、候选、目标 blockId
- **states**：Idle、Throttling、Requesting、Presenting、Accepted、Dismissed
- **events**：trigger、throttlePass、throttleDrop、candidateOk、requestFail、accept、dismiss、anchorLost
- **transitions（主表）**：

| from | event | guard | to | action |
|---|---|---|---|---|
| Idle | trigger | — | Throttling | 进入节流判定（BR-AG-STATE-003） |
| Throttling | throttleDrop | 节流窗口内/输入仍活跃 | Idle | 丢弃本次触发（静默） |
| Throttling | throttlePass | 节流通过 且 门禁可用（SM-SYS-01 GatePassed） | Requesting | 经网关轻量调用 |
| Throttling | throttlePass | 节流通过 但 无订阅/断网 | Idle | 静默不提供，不打断编辑 |
| Requesting | candidateOk | 目标锚仍有效 | Presenting | 候选经 ED 装饰层呈现（ED-DECOR） |
| Requesting | requestFail | — | Idle | 静默（不弹错误、不打断编辑） |
| Presenting | accept | — | Accepted | 采纳=用户编辑操作：经 ED 受控接口进入编辑模型，驱动 SM-ED-01 edit（X-INTER-001） |
| Presenting | dismiss | 用户忽略/继续键入 | Dismissed | 装饰复位不残留 |
| Presenting | anchorLost | blockId 失效 | Dismissed | 放弃呈现，装饰复位 |

- **error_states**：无显式错误终态——辅助回合一切失败静默回 Idle（产品语义：辅助不打扰）
- **recovery_paths**：失败静默回 Idle，下次触发自然重试
- **forbidden_transitions**：
  - Idle / Throttling → Requesting 无门禁可用（不经门禁不得调用，BR-AG-VERIFY-001）
  - Requesting / Presenting → 直接修改文档模型（必经 accept 走 ED 受控接口，X-FORBID-005）
  - Idle → Presenting（未经调用不得呈现）
- **acceptance_criteria**：高频触发被节流（不逐次调用）；无订阅/断网/失败均静默且编辑不受影响；
  accept 产生的变更走 SM-ED-01→SM-WS-01 正常脏态/保存路径；终态后装饰复位不残留
- **test_matrix**：见 §5

## §5 测试与验收（强制转移测试矩阵，APC §6/§10）

- **SM-AG-01**：send→Assembling→AwaitingGate→gatePassed→Streaming→streamEnd→HandingOff→Completed
  正路；gateDenied→Denied 终态且不可达 Streaming；cancel→Cancelled 无半落盘；handoffFail/
  streamFail→TurnError→retry；禁止 Idle/Assembling→Streaming 旁路。
- **SM-AG-02**：toolRequest→校验→执行→回注→续调循环；validateFail 回注拒绝；loopLimit 强制
  中止；cancel 随时中止；断言无任何工具执行产生直接文件写入（与 DE/WS G5 联合验证）。
- **SM-AG-03**：高频 trigger 下 throttleDrop 生效；无订阅/断网 throttlePass 后静默回 Idle；
  candidateOk→Presenting→accept→Accepted 驱动 SM-ED-01 edit；anchorLost/dismiss 复位不残留；
  禁止未经门禁到 Requesting。
- **跨模块（G5 联调）**：AG↔SM-SYS-01 门禁转移、HandingOff↔DE（待 DE G2 双侧）、
  SM-AG-03 accept↔SM-ED-01 edit（双侧就绪）。
- 量化 NFR（节流参数、延迟）延 G5 标定。

## §6 总图登记（A-SYS-M-S-01）

| 核心链 | machine_id | 总图回填动作 |
|---|---|---|
| chat-agent 回合 | SM-AG-01 | §3 行 planned→active |
| tool-call | SM-AG-02 | §3 行 planned→active |
| autocomplete | SM-AG-03 | §3 行 planned→active（与 inline-assist 同机承载，context 区分） |
| inline-assist | SM-AG-03 | §3 行 planned→active（同上） |
| 转移点 | — | §5 X-INTER-001 登记：AG↔门禁（SM-SYS-01）、AG 产出→DE（DE 侧待 DE G2）、SM-AG-03 采纳→SM-ED-01（双侧就绪） |

## §7 术语归属说明

代理回合/工具调用/内联辅助/自动补全 TERM 已在 A-AG-M-T-01 §0 注册；订阅门禁等云术语归 SYS
（TERM-CLOUD-001~004）；diff 主责归 DE。状态机框架词不注册。

## §8 技术债务与盲区（诚实声明）

- **AG 产出→DE 转移仅单侧**：HandingOff 的 AG 侧契约已定（修改建议+定位锚交 DE，回执 handoffOk/
  Fail）；DE 侧 diff 终态机待 DE G2，双侧完整对接与 G5 联调在 DE G2 后闭合。
- ~~Chat↔Build 接管未建模~~ **已建模（B-3，2026-06-12）**：SM-AG-01 增 takeover/fallbackIn 转移与
  HandedOver 终态，与 SM-CBT-01 双向回执对接，登记总图 §5。原始记录留痕——
- **Chat↔Build 接管（历史债务，已解）**：SM-AG-01 与 CBT 构建机的接管/中断对接（总图 §5 既有转移点）联合
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
