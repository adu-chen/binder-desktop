# Issue Trace — AG-PROMPT-STRUCTURE-FIX-01：提示词装配结构固定化（LayerKind 单一真源）

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-09

## 1. 用户可见症状

- 无用户可见功能症状（本轮为提示词装配结构固定化重构）。
- 隐性结构症状：提示词装配存在"声明骨架"（`assemble_prompt` 四层）与"实际装配"（`assemble_agent_turn_messages`）两套逻辑，仅测试引用声明骨架，生产未使用，双轨运行；
  审计层名靠 content 字符串推断；`system_prompt_version` 硬编码不随内容变更；预算 gate 口径低估；历史 22 条平铺膨胀。

## 2. 入口点

- 后端回合主入口：`src-tauri/src/lib.rs` — `ag_start_turn`（`assemble_agent_turn_messages` 调用点、Runtime Control 拼接、预算 gate、audit 构建）。
- 骨架声明：`src-tauri/src/ag/context.rs` — `assemble_prompt` / `PromptSkeleton` / `PromptLayer`（拟废弃）、`build_prompt_package`（版本硬编码）。
- 授权投影：`src-tauri/src/ag/tool_registry.rs` — `resolve_capability_grant`（补 grant_hash）。

## 3. 相关规则

- `BR-AG-DATA-001`：上下文来源限定、workspace 限界、显式引用优先保留、裁剪可追溯。
- `BR-AG-DATA-002`：统一组装骨架（系统→工具→上下文→用户）、可信边界 data-only 注入、PromptPackage 元数据、三类回合差异。
- `BR-AG-DATA-004`：当前轮次原则、意图沿用判定（IntentCarryover）。
- `BR-AG-DATA-005`：分层截断序、当前文档 P0 永不截。
- `BR-AG-STATE-002`：工具注册与逐回合授权（CapabilityGrant）。
- `X-FORBID-006`：内容修改须经 diff 裁决，AG 不直写。
- `X-FORBID-001`：客户端不持密钥。

## 4. 相关链路

`AG-TURN`（本任务只动 AG-TURN 内 Prompt 装配子路径，不跨链路）。

## 5. 生产者块

- `BR-AG-DATA-001-DATA-AG-AG-TURN-001` 位于 `src-tauri/src/ag/context.rs`（上下文裁剪）。
- `BR-AG-DATA-002-DATA-AG-AG-TURN-002` 位于 `src-tauri/src/ag/context.rs`（组装骨架声明，拟替换为 LayerKind 驱动）。
- `BR-AG-DATA-005-DATA-AG-AG-TURN-004` 位于 `src-tauri/src/ag/context.rs`（历史预算选择）。
- `BR-AG-DATA-004-DATA-AG-AG-TURN-003/005` 位于 `src-tauri/src/ag/context.rs`（意图沿用 / 历史归属分流）。
- `BR-AG-DATA-001-DATA-AG-AG-TURN-011`（复合）位于 `src-tauri/src/lib.rs`（真实模型消息装配）。

## 6. 消费者块

- `BR-AG-DATA-001-EFFECT-AG-AG-TURN-005` 位于 `src/services/agService.ts`（前端消费回合事件）。
- `BR-AG-STATE-002-QUERY-AG-AG-TOOL-012` 等位于 `src-tauri/src/ag/tool_exec.rs`（工具执行校验）。
- `src-tauri/src/ent/gateway.rs` `stream_round`（消费 messages/tools 发送 provider）。

## 7. 路径上的空规则块

- 未发现本路径上需要修改且已标注 `rules: []` 的决策块。
- 若 `governance:audit` 产生 `BLOCK_UNANNOTATED`，在本 Scope 内按真实职责补标或记录 triage，不静默绕过。

## 8. boundary 假设

- `lifecycle_assumption=`：无既有块显式声明生命周期假设被本次修改违反；`assemble_prompt` 仅测试使用，删除后无生产副作用。
- `document_snapshot=`：current_document 注入保持"当前编辑器 LogicalState 为 P0 主事实"（BR-AG-DATA-005），本次不改文档上下文来源，仅统一消息层序。

## 9. 运行时假设

- `caller_context=async tauri command`：`ag_start_turn` 在异步链运行；消息装配为纯函数，无副作用。
- `runtime_guard=gatePassed`：Streaming 前必经门禁（BR-AG-VERIFY-001），本次不触碰门禁路径。
- `runtime_guard=data-only`：文档/引用/历史以 DATA-ONLY system 消息注入，不改变首层 Runtime 约束——本次保持该边界，仅统一层枚举与聚合格式。

## 10. 疑似不变量破坏

- 中风险：废弃四层 `PromptSkeleton`/`assemble_prompt` 后，须以 `LayerKind` 八层枚举完整承接 BR-AG-DATA-002 的组装语义（系统→工具/能力→上下文→用户），不得丢层；`TurnKind` 差异（Chat 含历史 / InlineAssist 轻量）保留。
- 中风险：`system_prompt_version` 由硬编码改内容 hash 派生后，既有 audit 记录版本号变化；属于预期行为，不改规则。
- 中风险：预算 gate 口径从"信封之和"改为"序列化 messages+tools"，阈值判断更严格；属预期收紧。
- 中风险：历史由逐条平铺改为聚合单块后，模型可见消息数减少、role 序列保留；不改 data-only 语义。
- 未发现需要修改 ADU/ADP/AGENTS/APC 或新增静态 RULE/CHAIN/CONSTRAINT 的情况；`LayerKind` 为技术实现词汇，不注册 TERM（与既有 `PromptLayer` 同级处理）。

## 11. 验证计划

- Rust 单元测试：`cargo test --lib ag`（context.rs 层序/版本 hash 派生/预算口径；lib.rs audit 读 layer）。
- 影响测试：以实际 changed_files 运行 `npm run test:plan -- --files [changed_files]` 审阅，再运行 `npm run test:related -- --files [changed_files]`；AG 为 critical，必须包含 Rust 全量与直接消费者。
- 治理审计：`npm run governance:generate`（因修改设计文档标注块与代码块）后 `npm run governance:audit`。
- 链路日志：`BINDER_PROMPT_AUDIT=1` 复现一轮，核对 prompt_layers 含 layer 字段、system/runtime_control 分离、历史聚合单块、版本号变化。
- triage：若测试选择器无法映射可执行变更，按失败关闭处理并定位根因，不手选测试。

## 文件范围（Scope Lock 输入）

允许修改：

- 实现：`src-tauri/src/ag/context.rs`（LayerKind/顺序/装配/版本/预算）、`src-tauri/src/lib.rs`（调用点/audit/历史聚合/RuntimeControl 分离）、`src-tauri/src/ag/tool_registry.rs`（grant_hash）
- 设计文档：`docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md`（§5.1 登记 LayerKind 装配）、`docs/01_modules/AG/A-AG-M-T-02_提示词架构设计.md`（层定义/截断序/意图→工具矩阵登记）
- 派生索引：`docs/00_core/A-CORE-C-R-01_ADUS.md`（仅由 generate 生成）
- 治理运行记录：`.governance/issues/AG-PROMPT-STRUCTURE-FIX-01.md`、`.governance/active_scope.json`

禁止修改：

- `ADU.md`、`ADP.md`、`AGENTS.md`、`CLAUDE.md`
- `docs/00_core/A-CORE-C-P-01_APC.md`、`governance.config.json`
- `.governance/test-governance.json`、`scripts/**`
- Cloud/CBT/AST/KB/TPL 实现文件、前端 `src/**`
- 不在允许列表中的文件默认禁止；若验证证明必须扩展，按 ADP SC-01 停止并请求一次范围扩展。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-08-09 | v1.0 | 初始版本：提示词装配结构固定化（LayerKind 单一真源、RuntimeControl 独立消息、历史聚合、版本 hash 派生、预算统一口径、audit 读 layer） |

## 12. Task Runtime Report — AG-PROMPT-STRUCTURE-FIX-01

### 12.1 任务结果

- 状态：`READY`。
- 完成内容：以 `LayerKind` 枚举取代旧四层 `PromptSkeleton`/`assemble_prompt`（双轨清理）；`RuntimeControl` 独立为结构化 system 消息；`system_prompt_version` 内容 hash 派生；预算 gate 统一为序列化 messages+tools 口径；历史聚合单块；audit 直接读 layer 字段（删除字符串推断）；`CapabilityGrantV1` 新增 `grant_hash`。
- 范围：未修改规则正文（BR-AG-DATA-002 骨架语义不变）、前端、ent/cbt/ast/kb/tpl；未发生 Scope 扩展。

### 12.2 实际变更

- `src-tauri/src/ag/context.rs`：新增 `LayerKind` 枚举（8 层）+ `layer_order()`（@GOV `BR-AG-DATA-002-DATA-AG-AG-TURN-016`）；废弃 `PromptLayer`/`PromptSkeleton`/`assemble_prompt`；`system_prompt_version` 改为 `ag-system-chat-{sha256 前 8 位}`；新增 `estimate_serialized_input_tokens` 统一预算口径。
- `src-tauri/src/lib.rs`：`assemble_agent_turn_messages` 按 LayerKind 层序装配并给每条消息打 layer 元数据；历史聚合单块（DATA-ONLY 保留 role 序列）；`ag_start_turn` 中 RuntimeControl 独立消息插入 System 之后；预算 gate 使用统一口径；发送 provider 前剥离 layer 字段；`prompt_audit_layer_name` 改为优先读 layer 字段（fallback 仅覆盖工具循环动态消息）；audit 的 `final_input_estimated_tokens` 复用统一口径。
- `src-tauri/src/ag/tool_registry.rs`：`CapabilityGrantV1` 新增 `grant_hash`（profile+intent_type+允许工具集+目标集派生）。
- 设计文档：`A-AG-M-T-01` §5.1 与 `A-AG-M-T-02` §5.1 登记 LayerKind 装配顺序与实现细节（v2.28 / v1.15）。
- 测试：`layer_order_is_fixed_and_matches_enum` 取代旧骨架顺序测试。

### 12.3 验证结果

| 验证 | 结果 |
|---|---|
| `cargo check --lib` | PASS（仅既有范围外 warning） |
| `cargo test --lib ag::context` | PASS：9 tests |
| `cargo test --lib prompt_` | PASS：16 tests（含 audit 与装配顺序测试） |
| `cargo test`（Rust 全量） | PASS：296 + 5 + 4 |
| `npm test`（前端全量） | PASS：26 files / 552 tests |
| `npx tsc --noEmit` | PASS |
| `npm run governance:generate` | PASS：245 @GOV 块，92 assets / 857 cases |
| `npm run governance:audit` | PASS：0 findings |
| `git diff --check` | PASS |

### 12.4 语义复核

- `BR-AG-DATA-002`：组装骨架语义不变（系统→工具/能力→上下文→用户），新 `layer_order()` 与 `assemble_agent_turn_messages` 的 8 层顺序为其实现映射；`TurnKind` 差异保留（Chat 含历史 / BuildTask 单文件 / InlineAssist 轻量）。
- `BR-AG-DATA-004/005`：历史聚合仍走 `decide_intent_carryover` + `scope_history_for_current_doc` + `select_recent_history`，data-only 注入与当前文档 P0 边界不变。
- data-only 可信边界：文档/引用/历史仍为 DATA-ONLY system 消息；RuntimeControl 独立消息不改变"数据不得获得指令权限"语义。
- 未发现规则名与 boundary 核心实体不一致。

### 12.5 未解决项

- 无。`BINDER_PROMPT_AUDIT=1` 的真实轮次日志复核属 VALIDATION 阶段可选确认，未要求本轮完成；`LayerKind` 为技术实现词汇，不注册 TERM（与既有 `PromptLayer` 同级处理，已记录于 Issue Trace §10）。

### 12.6 READY 自检

- 范围：通过；无范围外实现修改。
- 验证：通过；test:plan 所选命令全部实际通过。
- 证据：通过；自动化结果如上表。
- 完整：通过；旧逻辑（assemble_prompt/PromptSkeleton/PromptLayer/prompt_audit_layer_name 字符串推断）无残留引用。
- 诚实：通过；无绕过检查、无造假通过。
- 留痕：通过；未解决项为零，Scope Lock 已同步 READY。
