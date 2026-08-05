---
文档编号：SYS-I-P-02
文档状态：A
负责模块：SYS
文档职责：Binder 前端（React/TS）↔ 后端（Rust）统一 IPC / Tauri command 协议技术设计——command 命名与注册约定、请求-响应信封、统一错误形状、流式/事件回传逻辑契约；承接 REQ-SYS-001
上游约束：A-SYS-M-D-01_系统底座功能需求描述.md（REQ-SYS-001、SY-02）、A-CORE-C-P-01_APC.md（§4.1）、A-SYS-C-T-01_系统总体架构.md（§2 分层、§5 X-SEQ-002）、A-SYS-I-P-01_数据结构定义.md（载荷引用共享结构）、A-SYS-I-P-03_云接口契约.md（BR-SYS-OBS-001 对齐本文）、ADU.md（§3.1 RULE/CHAIN 格式）
直接承接：各业务模块 L2 的具体 command 签名在本协议骨架上承接；A-SYS-M-S-01 状态机经 IPC 事件驱动状态转移；A-SYS-I-P-03 云调用错误形状对齐本文统一错误形状
使用边界：本文是 IPC / command 协议骨架的 SSOT；不锁定流式**物理传输机制**（Channel<T> vs event，SY-02 维持 open，延编码实现选型）、不定义各 command 的具体名称/参数签名（归各模块 L2）、不定义报文序列化格式与具体标量类型（延实现）、不定义云协议字段（归 A-SYS-I-P-03）、不定义数据结构（归 A-SYS-I-P-01）
变更要求：统一错误形状/信封字段变更须回写本文并复核 A-SYS-I-P-03 对齐；流式逻辑契约变更须复核 A-SYS-M-S-01 状态机的流式终态；command 命名约定变更影响全体模块，须经架构评审
治理成熟度：draft
---

# Binder IPC / Tauri command 协议（SYS · L2 技术设计）

## §1 文档定位

本文是 SYS 模块 L2 技术设计层文档，承接 REQ-SYS-001（统一 IPC / Tauri command 协议），是
前端 React/TS ↔ 后端 Rust 全部跨进程调用的**协议骨架 SSOT**：command 命名与注册约定、请求-响应
信封、统一错误形状、流式/事件回传的**逻辑契约**。

依赖与被引用关系：

```
REQ-SYS-001（L1）→ 本文（IPC 协议骨架 SSOT）→ REQ_MAP → RULE/CHAIN → 代码(@GOV)
本文 ←引用← A-SYS-I-P-01（command 载荷=共享结构）
本文 ←对齐← A-SYS-I-P-03（BR-SYS-OBS-001 云错误形状对齐本文统一错误形状）
本文 →驱动→ A-SYS-M-S-01（状态机经 IPC 事件/流式终态转移）
```

边界（与 A-SYS-M-D-01 SY-02 一致，人类 2026-06-10 确认）：

- 本文锁定 IPC 协议**骨架与逻辑契约**；流式回传的**物理传输机制**（Tauri 2 `Channel<T>` 抑或
  `event` 总线）**不在本文锁定**，SY-02 维持 open，延至编码实现期选型。本文以传输无关的逻辑契约
  描述流式语义，使物理选型可后置而不破坏上层契约。
- 各业务模块的**具体 command 名称与参数签名**归各模块 L2；本文只定命名/注册约定与信封形状。
- 报文具体序列化格式与标量类型属实现，延编码期；本文以字段语义描述。

## §2 IPC 分层与调用方向

复用架构约束 X-SEQ-002（A-SYS-C-T-01 §5），本文不另立同义约束：

```
前端  UI 组件 → Store → Service ──(统一 command 协议)──→  后端 Command 层 → 领域逻辑 → FS/workspace.db
                         │                                                    │
                         └────────── 流式/事件回传（逻辑契约，§5）─────────────┘
```

- 前端任何跨进程意图必经 **Service → 注册 command → Rust**，不得跨层直连 FS/DB/网络（X-SEQ-002）。
- command 是前后端唯一契约面；未注册 command 一律按协议错误拒绝（BR-SYS-DATA-003）。

## §3 command 协议骨架（命名 · 注册 · 信封）

**命名与注册约定**

- command 名称按 `模块前缀.动作` 约定（如 `ws.read_file`、`de.accept_diff`），模块前缀取模块编码小写；
  具体名称由各模块 L2 声明并注册，本文只定约定与唯一性要求。
- 每个 command 必须在后端注册表登记方可被前端调用；未注册即协议错误。

**请求信封（语义字段，非最终序列化）**

| 字段 | 语义 | 备注 |
|---|---|---|
| command | 已注册 command 名 | 唯一标识被调用契约 |
| correlation_id | 调用关联标识 | 关联响应与流式分片（§5）；不透明 ID |
| params | 调用参数 | 形状按各 command 契约，跨模块部分引用 A-SYS-I-P-01 共享结构 |

**响应信封**

| 字段 | 语义 | 备注 |
|---|---|---|
| correlation_id | 回填发起调用的关联标识 | — |
| result | 成功结果 | 与 error 互斥 |
| error | 失败时的统一错误形状（§4） | 与 result 互斥 |

<!-- CHAIN
chain_id: IPC-CALL
主责模块: SYS
status: active
risk_tier: P1
-->

<!-- RULE
rule_id: BR-SYS-DATA-003
主链路: IPC-CALL
域: DATA
-->
前后端跨进程调用必须经统一 command 协议：调用方经 Service 层发起，携带已注册的 command 名、
correlation_id 与符合该 command 契约的 params；后端按注册表路由，未注册 command 一律按统一错误形状
（§4）拒绝。请求/响应以统一信封承载，跨模块传递的 params/result 字段引用 A-SYS-I-P-01 共享结构契约，
不得在协议外另立跨进程传输结构。

## §4 统一错误形状（OBS）

所有 command 失败（含未注册、参数不符契约、后端执行失败、流式错误）必须以**统一错误形状**回传，
为 IPC 与云调用错误的规范父形状：

| 字段 | 语义 |
|---|---|
| code | 机器可判错误码 |
| message | 用户可读消息 |
| retriable | 是否可重试 |
| reauth_required | 是否需重新认证 |

<!-- RULE
rule_id: BR-SYS-OBS-002
主链路: IPC-CALL
域: OBS
-->
IPC 统一错误形状：所有 command 失败以统一错误形状回传，含错误码、用户可读消息、是否可重试、
是否需重新认证四要素，便于上层一致处理与埋点。本规则是 IPC 与云调用错误形状的**规范父契约**；
A-SYS-I-P-03 的 BR-SYS-OBS-001（云调用错误形状）为其在认证/门禁/网关场景的特化并对齐本规则，
两者字段集合一致，不得各自漂移。

## §5 流式 / 事件回传逻辑契约（STATE）

AI token 流、构建进度、diff 推送等以流式回传。本文锁定**逻辑契约**，物理传输延实现选型（SY-02）：

- 流由一次 command 发起，所有分片以 `correlation_id` 关联到该发起调用。
- 分片有序投递；存在**明确的完成事件**标识流正常结束（进入完成终态）。
- 支持**中断/取消**：调用方可取消；中断为终态，不再投递后续分片。
- 流式错误以统一错误形状（§4）回传，进入错误终态。
- **中断不得产生半落盘副作用**：落盘归 WS/DE 并经人类裁决（REQ-CORE-002），流中断时控制态保持可恢复。

<!-- RULE
rule_id: BR-SYS-STATE-002
主链路: IPC-CALL
域: STATE
-->
流式回传逻辑契约：流式调用的分片必须经 correlation_id 关联到发起 command，有序投递，具备明确的
完成终态、中断/取消终态与错误终态（错误走统一错误形状）；流中断不得产生半落盘副作用，控制态保持
可恢复。本规则只约束流式的**逻辑生命周期语义**，不锁定物理传输机制（Channel<T> 或 event，SY-02
延实现选型）；A-SYS-M-S-01 状态机的流式终态须与本契约一致。

### §5.1 状态机投影广播契约（STATE，承接 X-FORBID-007）

后端各核心链状态机（A-SYS-M-S-01 登记的 SM-*）每次转移成功并落库后，经统一事件
`sm:{machine_id}:state_changed` 向前端广播其**权威态**；前端 store 据此投影（X-FORBID-007：
前端不另立与后端可漂移的并行业务态）。本节锁定该广播的**逻辑契约**（物理事件总线 SY-02 延选型）：

- **事件名**：`sm:{machine_id}:state_changed`（machine_id 为 A-SYS-M-S-01 登记的 SM-* 编号）。
- **载荷字段**（统一信封 `SmStatePayload`）：
  - `machine_id`：SM-* 编号（如 "SM-DE-01"）。
  - `instance_id`：实例标识，区分同机多实例（diff_id / turn_id / rel_path / entry_id …）。
  - `state`：状态枚举名（后端状态机当前态）。
  - `terminal`：是否终态。
  - `segment`（可选）：同机分层承接段（如 SM-SYS-01 的 `session` / `ent_invoke`），无分层时省略。
- **权威先于广播**：状态以落库为权威，广播是其投影；**广播失败不得中断业务流程**（落库已先行完成）。
- **不替代流式数据通道**：token 流 / 工具回执等数据分片仍走 §5 流式契约（Channel）；本广播只传**状态转移**，
  两通道并存、互不替代。
- 与 §5 一致：经 IPC 流式的状态机终态须同时满足 BR-SYS-STATE-002（完成/中断/错误三类终态）。

<!-- RULE
rule_id: BR-SYS-STATE-003
主链路: IPC-CALL
域: STATE
-->
状态机投影广播契约（承接 X-FORBID-007 的传输实现）：后端核心链状态机（SM-*）转移成功并落库后，
须经统一事件 `sm:{machine_id}:state_changed` 广播权威态，载荷为统一信封 SmStatePayload
（machine_id / instance_id / state / terminal / 可选 segment）；前端 store 仅投影该广播，不维护与
后端可漂移的并行业务态。状态以落库为权威、广播为其投影，广播失败不得中断业务流程。本广播只传状态
转移，不替代 §5 流式数据通道；经 IPC 流式的终态须与 BR-SYS-STATE-002 一致。

## §6 结构约束（复用，不新增）

本文不新增 CONSTRAINT：前端不得跨层直连 FS/DB/网络、必经 Service→command→Rust 的结构约束已由
**X-SEQ-002**（A-SYS-C-T-01 §5）锁定，本文 §2/§3 在协议层落实之，并以 BR-SYS-DATA-003 约束"必经
注册 command"。避免重复定义同义约束（ADU §3.1 CONSTRAINT 去重）。

## §7 术语归属说明（为何本文不注册 TERM）

command、信封、错误形状、correlation_id 等均为 IPC 协议技术框架词，非跨 session 易漂移的核心业务
概念，按 ADU §3.2「普通技术实现词不注册」不登记 TERM。本文涉及的业务实体（PendingDiff 等）其 TERM
主责归 DE/WS（见 A-SYS-I-P-01 §6），本文不抢注。

## §8 需求到规则映射（REQ_MAP）

<!-- REQ_MAP
req_id: REQ-SYS-001
target_type: CHAIN
target_id: IPC-CALL
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-001
target_type: RULE
target_id: BR-SYS-DATA-003
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-001
target_type: RULE
target_id: BR-SYS-OBS-002
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-001
target_type: RULE
target_id: BR-SYS-STATE-002
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-001
target_type: RULE
target_id: BR-SYS-STATE-003
status: mapped
-->

## §9 测试与验收

- **统一 command 协议**（BR-SYS-DATA-003，REQ-SYS-001 验收）：所有跨进程调用经注册 command；
  未注册 command 调用被统一错误形状拒绝；跨模块 params/result 引用 A-SYS-I-P-01 共享结构。
- **统一错误形状一致**（BR-SYS-OBS-002）：IPC 错误与 A-SYS-I-P-03 云错误（BR-SYS-OBS-001）字段集合
  一致（code/message/retriable/reauth_required），上层可统一处理。
- **流式逻辑契约**（BR-SYS-STATE-002）：流式调用具备完成/中断/错误三类终态，分片经 correlation_id
  关联；中断后无半落盘副作用、控制态可恢复；与 A-SYS-M-S-01 状态机流式终态对齐。
- **状态机投影广播**（BR-SYS-STATE-003，承接 X-FORBID-007）：SM-* 转移落库后经
  `sm:{machine_id}:state_changed` 广播 SmStatePayload（machine_id/instance_id/state/terminal/可选 segment）；
  前端只投影、不另立并行业务态；广播失败不中断业务；emit→listen 回路经 Tauri 事件系统验证（实现期
  `tests/sm_broadcast.rs` 实跑 + 前端 `smEvents.ts` 字段对齐）。
- 量化 NFR（IPC 往返延迟、流式首片延迟）延 G5 标定（APC §9）。

## §10 技术债务与盲区（诚实声明）

- **流式物理传输机制未锁定**（SY-02，人类 2026-06-10 确认延期）：`Channel<T>` 与 `event` 总线的取舍
  延编码实现期；本文逻辑契约对两种实现均成立，但实现期需补一份选型记录回填本文 §5。
- 报文序列化格式与具体标量类型延实现期；本文以字段语义描述。
- 各 command 的具体名称、参数与结果签名由各业务模块 L2 承接注册，本文仅定命名/注册约定与信封骨架。
- 背压/流控（高频分片下的缓冲策略）随物理传输选型一并在实现期确定。

## §11 后续动作

1. 本文落地后随下一次 `npm run governance:generate` 注册：IPC-CALL 链路、BR-SYS-DATA-003 /
   BR-SYS-OBS-002 / BR-SYS-STATE-002 规则、REQ-SYS-001 的 REQ_MAP 进 ADUS。
2. 续 SYS G2 末件 A-SYS-M-S-01（状态机总图）：汇总核心链状态机注册框架，流式终态与本文 §5 对齐，
   会话/门禁状态机引用 A-SYS-I-P-03 的 SM-SYS-01。
3. 编码实现期解除 SY-02：定 `Channel<T>` / `event` 物理机制并回填 §5、§10。
4. 本次新增 BR-SYS-STATE-003 随 `npm run governance:generate` 注册进 ADUS；代码侧
   `sm_events::emit_sm_state` @GOV 追溯之（A-CORE-X-L-04 收口）。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-10 | v1.0 | 初版（SYS G2）：IPC / Tauri command 协议骨架——命名/注册约定、请求-响应信封、统一错误形状（BR-SYS-OBS-002，云错误 BR-SYS-OBS-001 对齐其上）、流式回传逻辑契约（BR-SYS-STATE-002，物理传输 SY-02 延实现）；链路 IPC-CALL（P1）、规则 BR-SYS-DATA-003；复用 X-SEQ-002 不新增约束；REQ_MAP（REQ-SYS-001）；术语归属说明；测试验收与债务盲区 |
| 2026-06-27 | v1.1 | 收口状态机投影广播的设计 SSOT（A-CORE-X-L-04 §3 缺口）：§5.1 新增「状态机投影广播契约」+ 注册规则 **BR-SYS-STATE-003**（事件名 `sm:{machine_id}:state_changed`、统一信封 SmStatePayload、权威先于广播、不替代流式数据通道、承接 X-FORBID-007）；§8 REQ_MAP 补 BR-SYS-STATE-003（承 REQ-SYS-001）；§9 验收补广播条；§11 后续动作补 generate 注册与代码 @GOV 追溯。此前该广播仅存于 L3 计划，本次落入 L2 设计 SSOT |
| 2026-08-05 | v1.2 | ED-FONT-LOCAL-LIBRARY-01：登记新 command `fonts.list_fonts`（ED 字体组本地字体库同步）——无参数、返回 `{ families: string[] }`（macOS CoreText 只读枚举字体族名）；遵循 §3 命名/注册约定与 §4 统一错误形状；只读系统能力，不触达 workspace。 |
