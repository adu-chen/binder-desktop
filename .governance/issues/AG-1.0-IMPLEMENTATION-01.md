# Issue Trace — AG-1.0-IMPLEMENTATION-01：agent-edit-1.0 可靠文档编辑主链落地

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-08

## 1. 用户可见症状

- 当前 Agent 主链存在门禁常量放行、无真实取消、错误与完成终态可能混淆、注册工具等同授权等可靠性缺口。
- 当前轮意图、上下文、Prompt 和工具权限未形成同一份版本化运行契约；新建文档仍可绕过 DiffCard 直接写盘。
- 结果是“能调用模型/工具”尚不能等同于 Binder 1.0 所要求的“用户持续参与、变更可审阅、失败不破坏文档”的可靠文档编辑能力。

## 2. 入口点

- 用户入口：`src/components/ag/Composer.tsx` — `Composer` 发送、模式选择与取消交互。
- 前端运行入口：`src/services/agService.ts` — `startTurn` 及 Tauri `Channel<AgTurnEvent>` 消费。
- 前端回合投影：`src/store/agStore.ts` — 回合状态、会话隔离与消息分段。
- 后端主入口：`src-tauri/src/lib.rs` — `ag_start_turn`、工具循环、AG→DE 交接。
- 模型边界：`src-tauri/src/ent/gateway.rs` — `check_gate`、`stream_round`。
- 领域基础：`src-tauri/src/ag/turn.rs`、`context.rs`、`tool.rs`、`tool_registry.rs`、`prompts.rs`。

## 3. 相关规则

- `BR-AG-STATE-001`：AgentTurn 生命周期、真实取消、单一终态。
- `BR-AG-STATE-002`：ToolCall 注册、逐回合授权、执行回注与循环上限。
- `BR-AG-VERIFY-001`：每次模型调用消费本次服务端 GateOutcome，不得本地假通过。
- `BR-AG-DATA-001`：上下文来源、workspace 限界、裁剪与 provenance。
- `BR-AG-DATA-002`：Prompt 分层、可信边界、版本和追踪。
- `BR-AG-DATA-003`：单目标写入边界、新建文档空基准提案。
- `BR-AG-DATA-004`：当前轮 IntentResult 优先，历史只按显式沿用进入。
- `BR-AG-DATA-005`：当前 LogicalState 优先、历史和补强层级。
- `BR-AG-PERSIST-001`：会话、回合、错误和未决提案可恢复且 workspace 隔离。
- `BR-AG-UI-001`：消息、工具、Diff 与真实 Runtime 事件顺序一致。
- `BR-SYS-STATE-002`、`BR-ENT-STATE-001`、`BR-ENT-VERIFY-001`：跨进程流终态和服务端门禁。
- `X-SEQ-001`：模型调用前必须通过服务端门禁。
- `X-FORBID-001`、`X-FORBID-002`：客户端不持 provider 密钥、不本地伪造订阅通过。
- `X-FORBID-004`：上下文与工具不得越出 active workspace。
- `X-FORBID-006`：AI 内容变更只形成 ChangeProposal，经 DE 裁决后才可生效。
- `X-FORBID-008`、`X-FORBID-009`：1.0 不开放未验证的自动补强/长期能力和自治能力。

## 4. 相关链路

`AG-TURN → CLOUD-GATE → CLOUD-INVOKE → AG-TOOL → AG-TURN → DE-DIFF → ED-EDIT → WS-SAVE`

只读回答路径为：`AG-TURN → CLOUD-GATE → CLOUD-INVOKE → AG-TURN`。

## 5. 生产者块

- `BR-AG-STATE-001-RB-AG-AG-TURN-001` 位于 `src-tauri/src/ag/turn.rs`。
- `BR-AG-VERIFY-001-GUARD-AG-AG-TURN-001` 位于 `src-tauri/src/ag/turn.rs`。
- `BR-AG-DATA-001-DATA-AG-AG-TURN-001`、`BR-AG-DATA-002-DATA-AG-AG-TURN-002`、`BR-AG-DATA-004-DATA-AG-AG-TURN-003`、`BR-AG-DATA-005-DATA-AG-AG-TURN-004` 位于 `src-tauri/src/ag/context.rs`。
- `BR-AG-STATE-002-RB-AG-AG-TOOL-001`、`BR-AG-STATE-002-GUARD-AG-AG-TOOL-001` 位于 `src-tauri/src/ag/tool.rs`。
- `BR-ENT-VERIFY-001-GUARD-ENT-CLOUD-GATE-001`、`BR-ENT-STATE-001-RB-ENT-CLOUD-INVOKE-001` 位于 `src-tauri/src/ent/gateway.rs`。
- `BR-AG-DATA-001-DATA-AG-AG-TURN-011` 等主回合组装块位于 `src-tauri/src/lib.rs`。

## 6. 消费者块

- `BR-AG-DATA-001-EFFECT-AG-AG-TURN-005` 位于 `src/services/agService.ts`，消费编辑器 LogicalState、引用和事件流。
- `AgTurnEvent` API 数据契约位于 `src-tauri/src/ipc/mod.rs`，由前端 `startTurn` 消费。
- AG→DE proposal 生成与目标守卫位于 `src-tauri/src/lib.rs`，后续由 DE/ED/WS 消费。
- `src/components/ag/Composer.tsx` 与 `src/store/agStore.ts` 消费 Runtime Profile、回合终态和取消能力。

## 7. 路径上的空规则块

- 未发现本次主链上需要修改且已标注为 `rules: []` 的决策块。
- `Composer.tsx`、`tool_registry.rs` 中的呈现/注册数据不单独拥有规则；规则裁决分别委托后端 Runtime Profile、IntentResult 和 CapabilityGrant 守卫。
- 若审计产生新的 `BLOCK_UNANNOTATED`，在本 Scope 内按真实职责补标或记录 triage，不以空规则块绕过。

## 8. boundary 假设

- `document_snapshot=stable/current LogicalState`：编辑目标必须来自发起回合时的编辑器快照；成立，并通过 revision/hash 在 proposal 时复核。
- `lifecycle_assumption=Streaming only after gatePassed`：当前常量 GatePassed 破坏真实来源，本次改为消费 `check_gate()` 的本次服务端裁决。
- `lifecycle_assumption=terminal state frozen`：当前 Error/Done 发射分散，存在双终态风险；本次由 terminal latch 集中收口。
- `document_snapshot=disk_snapshot` 只允许未打开且显式只读目标；不得覆盖当前编辑器 LogicalState。

## 9. 运行时假设

- `caller_context=platform_runtime/async_chain`：Tauri command、网关流和工具循环均在异步链运行；数据库锁必须在 `await` 前释放。
- `runtime_guard=one active turn per session`：本次新增后端单活注册；第二个回合稳定拒绝，不依赖仅前端 `sending`。
- `runtime_guard=registered AND granted AND target-valid AND params-valid`：当前只有注册校验，本次补逐回合 CapabilityGrant。
- `runtime_guard=cancel propagation`：取消信号必须唤醒 provider 流并在每次工具执行前复核，取消后不得生成 proposal。

## 10. 疑似不变量破坏

- 高风险：替换常量门禁后，未启动 dev gateway 或无会话令牌将从“假通过后失败”变为真实 GateDenied/Offline；这是设计要求的行为修复，前端必须保留真实错误。
- 高风险：移除 `agent-edit-1.0` 的 `create_doc` 授权后，新建内容必须通过 `propose_edit` 的 `base=empty` 路径；必须证明接受前文件不存在。
- 中风险：集中终态发射会改变部分测试对 Error 后 Done 的旧假设；旧假设与设计冲突，应更新为唯一终态而不弱化断言。
- 中风险：关闭 Build、结构工具和自动 AST/KB/TPL 会收窄 1.0 可见能力；完整实现保留在后续 profile，不删除底层能力。
- 未发现需要修改 ADU/ADP/AGENTS/APC 或新增静态 RULE/CHAIN/CONSTRAINT 的情况；`TERM-AG-012 IntentResult` 已注册。

## 11. 验证计划

- 状态机与 Rust 单元测试：Gate denied/offline、取消各阶段、Error 不再 Done、terminal latch race、同 session 双发、Runtime Profile 拒绝。
- Intent/Context/Prompt 契约测试：answer/analyze/edit/create/clarify、否定变更、目标缺失、跨文档历史、LogicalState 优先、自动 AST/KB/TPL 调用数为 0、data-only 注入与固定层序。
- Tool 测试：Intent×Tool allow/deny 矩阵；注册但未授权、过期 grant、错误 turn/target/params、结构工具与 `create_doc` 在 1.0 拒绝。
- Editor/Diff 测试：当前文档 proposal、新建空基准 proposal、拒绝、基准冲突、取消/错误不产生 proposal；复用并扩展 `VAL-CORE-EDIT-DIFF-SAVE`。
- 前端服务/组件测试：runtime profile 透传、Build 入口关闭、真实取消命令、唯一终态、会话隔离和消息事件顺序。
- 影响验证：以实际 changed_files 运行 `npm run test:plan`、`npm run test:related`；AG 为 critical，必须包含 Rust 和直接消费者。
- 全量验证：`npm run test:full`、`npm run governance:generate`、`npm run governance:audit`；有 semantic review candidates 时逐项做规则语义比对。
- 链路日志：核对 `Assembling→AwaitingGate→Streaming→(Completed|Cancelled|TurnError|Denied)`，以及 `ModelStreaming→ToolValidating→ToolExecuting→ResultInjecting` 的真实发生顺序；终态后不得有 token/tool/proposal。
- triage：记录真实网关/BYOK/生产订阅环境若无法在本机完成的 G5 项，不用 mock 冒充 G5。

## 文件范围（Scope Lock 输入）

允许修改：

- Runtime/领域：`src-tauri/src/ag/turn.rs`、`context.rs`、`tool.rs`、`tool_registry.rs`、`prompts.rs`、`history.rs`、`mod.rs`
- 编排/协议/网关：`src-tauri/src/lib.rs`、`src-tauri/src/ipc/mod.rs`、`src-tauri/src/ent/gateway.rs`
- 前端接线：`src/services/agService.ts`、`src/store/agStore.ts`、`src/components/ag/Composer.tsx`
- 验证：`src/services/agRuntime.test.ts`、`src/components/ag/AgentUI.test.tsx`、`src/store/assistRuntime.test.ts`、`src-tauri/tests/core_chain.rs`
- 计划与验证证据：`docs/01_modules/AG/A-AG-X-L-04_代理1.0开发与验证计划.md`、`docs/40_validation/A-VAL-X-V-02_核心编辑与DiffEdit保存验证.md`
- 派生索引：`docs/00_core/A-CORE-C-R-01_ADUS.md`（仅由 generate 生成）
- 治理运行记录：`.governance/issues/AG-1.0-IMPLEMENTATION-01.md`、`.governance/active_scope.json`

禁止修改：

- `ADU.md`、`ADP.md`、`AGENTS.md`、`CLAUDE.md`
- `docs/00_core/A-CORE-C-P-01_APC.md`、`governance.config.json`
- `.governance/test-governance.json`、`scripts/**`
- Cloud 生产服务、CBT、AST、KB、TPL 的实现文件
- 不在允许列表中的文件默认禁止；若验证证明必须扩展，按 ADP SC-01 停止并请求一次范围扩展。

## 12. Task Runtime Report

### 12.1 任务结果

- 状态：`READY`（本地实现与自动化已完成；G5 真实系统验证明确保留为下一阶段）。
- 完成工作包：WP-AG10-00～WP-AG10-07，均达到 Module Ready；直接消费者达到本地 Integrated。
- 范围：未修改 Cloud、CBT、AST、KB、TPL、测试治理注册表或治理控制文档；未发生 Scope 扩展。

### 12.2 实际变更

- Execution：服务端真实 GateOutcome、每次模型续调重验、同 session 单活、provider abort、工具前取消、
  首终态锁存；拒绝/错误/取消后不再产生完成终态。
- Intent/Context：`IntentResultV1` 统一意图、目标、操作、约束、置信度、歧义、所需工具与上下文；
  缺编辑目标进入澄清；不相关问答不自动注入当前文档；当前 LogicalState 与显式引用边界保持。
- Prompt Runtime：`PromptDataEnvelopeV1` 与 `PromptPackageV1` 落地；固定分层顺序；历史选择与实际注入
  共用同一结果；记录真实 tool manifest hash；日志仅保留脱敏元数据。
- Tool：`CapabilityGrantV1` 落地；模型只看到本轮授权工具，执行器再次校验 registry/grant/target/params；
  Build、结构工具、自动 AST/KB/TPL、表格/图片高级提案不进入 `agent-edit-1.0`。
- Editor/Diff：当前文档编辑和新建内容统一走 ChangeProposal；新建目标必须不存在、使用空基准；
  同回合最多生成一个 proposal；接受前不创建文件。
- Frontend：Runtime Profile 显式透传；Build 入口物理关闭；停止按钮调用后端真实取消；首个终态获胜，
  Error 不会被迟到 Done 覆盖；活动回合状态与 session/turn 绑定。

实际代码/测试文件：

- `src-tauri/src/ag/turn.rs`
- `src-tauri/src/ag/context.rs`
- `src-tauri/src/ag/tool_registry.rs`
- `src-tauri/src/ag/prompts.rs`
- `src-tauri/src/ent/gateway.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/tests/core_chain.rs`
- `src/services/agService.ts`
- `src/store/agStore.ts`
- `src/components/ag/Composer.tsx`
- `src/services/agRuntime.test.ts`
- `src/components/ag/AgentUI.test.tsx`

证据文档：

- `docs/01_modules/AG/A-AG-X-L-04_代理1.0开发与验证计划.md`
- `docs/40_validation/A-VAL-X-V-02_核心编辑与DiffEdit保存验证.md`
- `docs/00_core/A-CORE-C-R-01_ADUS.md`（generate 派生）

### 12.3 验证结果

| 验证 | 结果 |
|---|---|
| `npx vitest run` AG 三文件 | PASS：3 files / 92 tests |
| `cargo test --lib ag` | PASS：136 passed |
| `cargo test --test core_chain` | PASS：5 passed |
| `npx tsc --noEmit` | PASS |
| `npm run test:inventory` | PASS：92 assets / 856 cases，AG 238 cases |
| `npm run test:plan -- --files ...` | PASS：选择前端全量、TypeScript、Rust 全量、governance audit |
| 受控沙箱 `test:related` | FAIL：仅 LibreOffice E2E 因 AppKit 注册限制 signal 6；293 其余 Rust tests 通过；未修改/跳过测试 |
| 非沙箱 `npm run test:related -- --files ...` | PASS：前端 552；Rust 295 + 5 + 4；TypeScript、audit 通过 |
| 非沙箱 `npm run test:full` | PASS：前端 552；Rust 295 + 5 + 4；LibreOffice 真转换通过 |
| `npm run governance:generate` | PASS：245 @GOV；92 assets / 856 cases |
| `npm run governance:audit` | PASS：0 governance finding；0 test governance warning |
| `git diff --check` | PASS |

### 12.4 语义复核

- `BR-AG-DATA-004-RB-AG-AG-TURN-015`：输入为当前用户原文与编辑器目标，输出结构化 IntentResult；
  当前轮否定约束优先，阻断歧义停止 mutation，与 BR-AG-DATA-004 一致。
- `BR-AG-DATA-001/002/004/005-DATA-AG-AG-TURN-011`：实际装配为系统/Intent/当前文档/显式引用/
  必要历史/当前用户；数据层全部 data-only，历史默认弱化，当前 LogicalState 优先，和规则正文一致。
- CapabilityGrant 与执行器四重校验承接 BR-AG-STATE-002；新建空基准、目标不存在与接受前无写入承接
  BR-AG-DATA-003/X-FORBID-006。未发现规则名与 boundary 核心实体不一致。

### 12.5 未解决项

- `G5_PENDING`：尚未使用真实账号/订阅/AiGateway/托管模型或 BYOK，在真实 Tauri/WKWebView 中执行
  Read/Analyze/Edit/Create/Safety/Conflict/Isolation 场景；不得据本报告宣称 Release Ready。
- `AUTOMATION_GAP-02`：尚无单一确定性 harness 贯穿真实 `ag_start_turn`→网关流→工具→DE→前端
  DiffCard；现有模块与消费者证据不能冒充该 G5 系统链。
- 项目级 `cargo fmt -- --check` 会报告既有范围外文件及 `lib.rs` 旧区域格式差异；不属于测试选择器门禁，
  本轮没有扩大 Scope 清理全库格式。

### 12.6 READY 自检

- 范围：通过；无范围外实现修改。
- 验证：通过；test:plan 所选命令在所需非沙箱环境全部实际通过。
- 证据：通过；自动化与 G5 系统验证分开记录。
- 完整：通过；无 TODO、桩、跳过测试或半成品接线。
- 诚实：通过；保留沙箱原始失败并以同测试非沙箱对照复跑，没有制造假绿。
- 留痕：通过；G5 与确定性 Agent harness 缺口已登记。
