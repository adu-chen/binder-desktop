---
文档编号：SYS-M-S-01
文档状态：A
负责模块：SYS
文档职责：Binder 核心链状态机总图与注册框架技术设计——状态机注册清单、注册前置约束、跨模块状态转移契约框架；承接 REQ-SYS-003
上游约束：A-SYS-M-D-01_系统底座功能需求描述.md（REQ-SYS-003、SY-04）、A-CORE-C-P-01_APC.md（§6 状态机驱动强制阈值与必备字段）、A-SYS-I-P-02_IPC协议.md（BR-SYS-STATE-002 流式终态）、A-SYS-I-P-03_云接口契约.md（SM-SYS-01）、ADP.md（SC-06/07）、ADU.md（§3.1 CONSTRAINT 格式）
直接承接：各 owner 模块在其 M-S 定义本模块状态机并向本总图登记 machine_id；跨模块转移点（diff 落盘、Chat↔Build 接管等）以本文框架对接
使用边界：本文是状态机总图框架与跨模块转移契约的 SSOT；**不替各 owner 模块定义其状态机内部**（states/transitions/guards 等归各模块 M-S）；不重定义已存在的 SM-SYS-01（归 A-SYS-I-P-03）；总图完整清单依赖各模块 G2 暴露真实链路后回填（SY-04）
变更要求：核心链强制阈值变更须先经 APC §6 授权；新增状态机由 owner 模块 M-S 定义后登记本总图；跨模块转移契约变更须两侧 owner 共同复核
治理成熟度：draft
---

# Binder 状态机总图与注册框架（SYS · L2 技术设计）

## §1 文档定位

本文是 SYS 模块 L2 技术设计层文档，承接 REQ-SYS-003（状态机总图与注册），承担一项 SSOT 职责：
**核心链状态机的总图框架与注册入口**，以及**跨模块状态转移契约框架**。

```
REQ-SYS-003（L1）→ 本文（总图框架 + 注册前置/跨模块转移约束）→ REQ_MAP → CONSTRAINT → 代码(@GOV)
各 owner 模块 M-S 定义状态机内部 ──登记 machine_id──→ 本总图
本文 ←引用← A-SYS-I-P-03（SM-SYS-01）、A-SYS-I-P-02（BR-SYS-STATE-002 流式终态）
```

边界（与 A-SYS-M-D-01 SY-04 一致）：

- 本文提供总图**框架与注册前置约束**，不替各 owner 模块定义其状态机内部（states/events/
  transitions/guards/actions/error_states/recovery_paths 等归各模块 M-S，APC §6）。
- **不重定义 SM-SYS-01**（账号会话与订阅门禁状态机已在 A-SYS-I-P-03 定义），本文仅登记并对齐。
- 总图**完整清单**依赖各业务模块 G2 暴露真实链路后回填（SY-04）；当前除 SM-SYS-01 外均为 planned。

## §2 状态机驱动总则与本文职责

APC §6 强制阈值（口径 b）：**有状态 / 跨层 / 多步的链路必须建状态机**，纯函数/纯展示豁免。
每个状态机必备字段（APC §6）：machine_id / owner_module / scope / context / initial_state /
states / terminal_states / events / transitions / guards / actions / error_states /
recovery_paths / forbidden_transitions / acceptance_criteria / test_matrix。

本文职责：
1. 维护状态机**注册清单**（§3），使每条核心链状态机可追溯 owner。
2. 立**注册前置约束**（§4 X-SEQ-003）：未注册/未建状态机的状态化核心链不得进入实现。
3. 立**跨模块转移契约框架**（§5 X-INTER-001）：跨模块状态转移须双侧对接一致。

## §3 状态机总图与注册清单

APC §6 列举的核心链（非穷举）及其状态机登记状态。`active` = 已在设计文档定义；
`planned` = 待 owner 模块 G2 在其 M-S 定义后回填 machine_id（豁免 CHAIN/状态机未标注检测）。

| 核心链 | owner_module | machine_id | status | 定义文档 |
|---|---|---|---|---|
| 账号会话与订阅门禁 | SYS（ACCT/ENT 消费） | SM-SYS-01 | active | A-SYS-I-P-03 §6 |
| diff 终态 | DE | SM-DE-01 | active | A-DE-M-S-01 §2 |
| chat-agent 回合 | AG | SM-AG-01 | active | A-AG-M-S-01 §2 |
| tool-call | AG | SM-AG-02 | active | A-AG-M-S-01 §3 |
| 文档编辑会话 | ED | SM-ED-01 | active | A-ED-M-S-01 §2 |
| save 保存 | WS | SM-WS-01 | active | A-WS-M-S-01 §2 |
| diff 落盘（WS 侧） | WS | SM-WS-02 | active | A-WS-M-S-01 §3 |
| 时间轴还原 | WS | SM-WS-03 | active | A-WS-M-S-01 §4 |
| docx 开 / 存 | DOC | SM-DOC-01 | active | A-DOC-M-S-01 §2 |
| CBT 构建 | CBT | SM-CBT-01 | active | A-CBT-M-S-01 §2 |
| 记忆写入 | AST | SM-AST-01 | active | A-AST-M-S-01 §2 |
| 工作流执行 | TPL | SM-TPL-01 | active | A-TPL-M-S-01 §2（owner G2 暴露新增） |
| autocomplete | AG（2026-06-11 人类确认） | SM-AG-03 | active | A-AG-M-S-01 §4（与 inline-assist 同机承载） |
| inline-assist | AG（2026-06-11 人类确认） | SM-AG-03 | active | A-AG-M-S-01 §4（与 autocomplete 同机承载） |

> machine_id 由各 owner 模块在其 M-S 自行分配（格式 SM-[MODULE]-[SEQ]，ADU §3），本文不预占，
> 避免跨模块 ID 归属漂移。2026-06-11 起总图全清单 active（12 台机，SY-04 闭合）；后续新模块/
> 新链路按 owner G2 暴露登记追加。

## §4 状态机注册前置约束（CONSTRAINT）

<!-- CONSTRAINT
constraint_id: X-SEQ-003
摘要: 状态化核心链（APC §6 强制阈值）必须先建状态机并在总图登记 machine_id 方可进入实现，未注册者阻断进入代码
涉及模块: SYS, WS, ED, DE, AG, CBT, DOC, AST
链路影响: CLOUD-AUTH, CLOUD-GATE, CLOUD-INVOKE
-->

> X-SEQ-003 与 ADP SC-06（PHASE_GATE_BLOCKED）/ SC-07（MAPPING_CHAIN_BROKEN）一致：越过状态机直接
> 实现状态化核心链触发阻断（APC §9、§6）。当前仅 SM-SYS-01 满足前置，其余核心链进入实现前须先补
> 状态机定义并登记本总图。

## §5 跨模块状态转移契约框架（CONSTRAINT）

部分核心链的状态转移跨模块发生，转移两侧的 owner 状态机必须在对接点保持一致，不得各自假设。
代表性跨模块转移点（随各模块 G2 回填）：

- **diff accepted → 落盘**：DE 的 diff 终态机（**SM-DE-01**，A-DE-M-S-01 §2）的 accepted 触发
  WS 的 diff 落盘机（**SM-WS-02**，A-WS-M-S-01 §3）的写入（REQ-CORE-002）；rejected/expired 驱动
  SM-WS-02→Discarded（不落盘）；Applied/ApplyError 回执回传 SM-DE-01（终态不回退）。
  **双侧已就绪**（2026-06-11 DE G2 闭合单侧），一致性在 G5 联调验证。
- **Chat ↔ Build 接管**：SM-AG-01（takeover→HandedOver 终态 / fallbackIn 起新回合，A-AG-M-S-01 §2
  B-3 补齐 2026-06-12）与 SM-CBT-01（A-CBT-M-S-01 §3）双向接管——takeover（须用户确认）/fallback +
  双向回执（takeoverAck/fallbackAck），无双活双死。**双侧已建机就绪。**
- **AG 回合 → AST 记忆写入**：SM-AG-01 HandingOff/Completed 终态后异步触发 `ast.capture`
  （传 origin_turn，不阻塞回合，BR-AG-STATE-004），AST 自动落库（SM-AST-01）。
  **双侧已就绪**（A-AG-M-T-01 §5.2 B-2 补齐 2026-06-12 + AST 侧 capture）。
- **TPL 步骤委托**：SM-TPL-01 逐步委托 SM-AG-01 / SM-CBT-01 执行（发起+回执；abort 取消传播
  延实现期细化）（A-TPL-M-S-01 §4）。
- **模型调用前门禁**：业务模块发起模型调用前必经 SM-SYS-01 的 GatePassed（X-SEQ-001，A-SYS-I-P-03）。
- **ED 脏态 ↔ WS 保存**：SM-ED-01 的 edit 事件是 SM-WS-01 Idle→Dirty 的唯一业务驱动源；SM-WS-01
  Saved 回执驱动 SM-ED-01 Editing→Ready 清脏态，SaveError 不清脏态（A-ED-M-S-01 §3，双侧已就绪）。
- **docx 装载与落盘**：SM-DOC-01 convertOk 对接 SM-ED-01 装载（convertFail 对接 loadFail/parseFail，
  失败语义两侧一致）；SM-DOC-01 SaveConverting 经 SM-WS-01 原子落盘（Saved/SaveError 回执驱动
  saveConvertOk/Fail），原文件不破坏由 WS 保障（A-DOC-M-S-01 §3，双侧已就绪）。
- **AG 回合门禁与产出**：SM-AG-01 AwaitingGate 提交 SM-SYS-01（gatePassed/gateDenied 驱动，
  Denied 不可绕过）；SM-AG-01 HandingOff 将修改建议（ChangeProposal）交 SM-DE-01 Generating
  （genOk/genFail 回执 handoffOk/Fail，**双侧已就绪**，A-DE-M-S-01 §3）；SM-AG-03 accept 驱动
  SM-ED-01 edit（采纳即用户编辑，双侧已就绪）（A-AG-M-S-01 §6）。
  至此「对话生成 diff→人类裁决→落盘」核心交互 AG→DE→WS（→ED 呈现）状态机链全链建机完毕。

<!-- CONSTRAINT
constraint_id: X-INTER-001
摘要: 跨模块状态转移必须在总图登记转移契约，转移两侧 owner 状态机对接点须一致，不得各自假设状态/事件
涉及模块: SYS, WS, DE, AG, CBT
链路影响: CLOUD-GATE, CLOUD-INVOKE
-->

## §6 流式终态与 IPC 对齐

经 IPC 流式回传的状态机终态须与 A-SYS-I-P-02 的 BR-SYS-STATE-002（流式逻辑契约：完成/中断/错误
三类终态）一致。已落地的 SM-SYS-01：`Invoking → InvokeCompleted`（流式完成）/ `Invoking →
InvokeError`（流式错误，统一错误形状）即对齐本契约；后续各 owner 的流式状态机登记时须同样对齐。

## §7 术语归属说明（为何本文不注册 TERM）

machine_id、状态机、转移、终态等为状态机框架技术词，按 ADU §3.2 不注册 TERM。各核心链涉及的业务
名词（diff、记忆、构建等）TERM 主责归对应 owner 模块（DE/AST/CBT），在其 L2 注册，本文不抢注。

## §8 需求到规则映射（REQ_MAP）

<!-- REQ_MAP
req_id: REQ-SYS-003
target_type: CONSTRAINT
target_id: X-SEQ-003
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-SYS-003
target_type: CONSTRAINT
target_id: X-INTER-001
status: mapped
-->

## §9 测试与验收

- **注册可追溯**（REQ-SYS-003 验收）：总图清单中每条核心链状态机可追溯 owner_module；planned 项
  随 owner G2 回填 machine_id 后转 active。
- **注册前置阻断**（X-SEQ-003）：状态化核心链在无状态机定义时，G4 实现验证触发 SC-06/07 阻断
  （集成阶段验证）。
- **跨模块转移一致**（X-INTER-001）：diff accepted→落盘、Chat↔Build 接管等转移点两侧状态机对接
  一致，在 G5 跨模块联调验证（APC §5.1）。
- **流式终态对齐**：经 IPC 流式的状态机终态与 BR-SYS-STATE-002 一致（SM-SYS-01 已对齐）。

## §10 技术债务与盲区（诚实声明）

- **总图清单已全量闭合（SY-04 闭合，2026-06-11）**：APC §6 列举核心链 + owner G2 暴露新增链
  （工作流执行）共 12 台状态机全部 active，无 planned 行；清单仍为非穷举框架，后续模块（ACCT/ENT
  等）暴露状态化链路时按 X-SEQ-003 建机登记追加。
- **autocomplete / inline-assist 归属已定（AG）**：2026-06-11 人类确认，SM-AG-03 同机承载已登记。
- 跨模块转移点清单（§5）为代表性、非穷举，随各模块 G2 暴露补全。

## §11 后续动作

1. 本文落地后随下一次 `npm run governance:generate` 注册：X-SEQ-003 / X-INTER-001 约束、REQ-SYS-003
   的 REQ_MAP 进 ADUS。
2. SYS G2 五件（C-T-01 / I-P-01 / I-P-02 / I-P-03 / M-S-01）齐备；待人类宣布 SYS G2 闭合（AI 不自行
   宣布晋级，APC §2）。
3. 各业务模块进入 G2 时在其 M-S 定义状态机并登记本总图，逐条将 planned 转 active，回填 SY-04。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-10 | v1.0 | 初版（SYS G2 末件）：状态机总图框架与注册清单（登记 APC §6 核心链，SM-SYS-01 active 其余 planned）；注册前置约束 X-SEQ-003（对齐 ADP SC-06/07）、跨模块转移契约框架 X-INTER-001；流式终态与 BR-SYS-STATE-002 对齐；REQ_MAP（REQ-SYS-003）；术语归属说明；债务盲区（SY-04 总图未闭合、autocomplete/inline-assist 归属待定） |
| 2026-06-10 | v1.1 | WS G2 回填（owner 模块登记，§11.3）：§3 总图清单 save 行 planned→active 回填 machine_id=SM-WS-01，新增「diff 落盘（WS 侧）SM-WS-02」「时间轴还原 SM-WS-03」两行（A-WS-M-S-01 定义）；§5 X-INTER-001「diff accepted→落盘」转移点登记 WS 侧机 SM-WS-02（DE 侧待 DE G2）；§10 SY-04 债务由「未闭合」更新为「部分闭合」（SYS+WS 已 active，余模块 planned） |
| 2026-06-11 | v1.2 | ED G2 回填（owner 模块登记，§11.3）：§3 新增「文档编辑会话｜ED｜SM-ED-01｜active」行（A-ED-M-S-01 定义）；§5 X-INTER-001 登记「ED 脏态↔WS 保存」转移点（edit→Dirty 唯一驱动、Saved 回执清脏态，双侧已就绪）；autocomplete/inline-assist 归属仍待确认，未回填 |
| 2026-06-11 | v1.3 | 人类决策落地：autocomplete / inline-assist 归属确认为 AG（§3 两行 owner 改 AG、待 AG G2 定义状态机；§10 对应债务关闭） |
| 2026-06-11 | v1.4 | DOC G2 回填（owner 模块登记，§11.3）：§3 docx 开/存行 planned→active 回填 machine_id=SM-DOC-01（A-DOC-M-S-01 定义）；§5 X-INTER-001 登记「docx 装载与落盘」转移点（DOC↔ED、DOC↔WS，双侧已就绪） |
| 2026-06-11 | v1.5 | AG G2 回填（owner 模块登记，§11.3）：§3 四行 planned→active——chat-agent 回合=SM-AG-01、tool-call=SM-AG-02、autocomplete/inline-assist=SM-AG-03（同机承载，A-AG-M-S-01 定义）；§5 X-INTER-001 登记「AG 回合门禁与产出」转移点（AG↔SM-SYS-01 门禁双侧就绪、AG 产出→DE 单侧待 DE G2、SM-AG-03 采纳→SM-ED-01 双侧就绪）；至此总图仅余 CBT 构建 / 记忆写入（AST）/ diff 终态（DE）三行 planned |
| 2026-06-11 | v1.6 | DE G2 回填（owner 模块登记，§11.3）：§3 diff 终态行 planned→active 回填 machine_id=SM-DE-01（A-DE-M-S-01 定义）；§5「diff accepted→落盘」「AG 回合门禁与产出」两条转移点更新为双侧就绪——「对话生成 diff→人类裁决→落盘」核心交互 AG→DE→WS（→ED）状态机链全链建机完毕；总图仅余 CBT 构建 / 记忆写入（AST）两行 planned |
| 2026-06-11 | v1.7 | P4 G2 回填（owner 模块登记，§11.3）：§3 CBT 构建行=SM-CBT-01、记忆写入行=SM-AST-01 转 active，新增「工作流执行｜TPL｜SM-TPL-01」行（owner G2 暴露登记）；§5「Chat↔Build 接管」更新为双侧就绪（takeover/fallback+回执），新增「TPL 步骤委托」转移点；**总图全清单无 planned 行，已登记 12 台状态机全部 active**（SY-04 闭合） |
