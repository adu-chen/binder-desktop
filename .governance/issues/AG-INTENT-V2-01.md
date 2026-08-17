# Issue Trace — AG-INTENT-V2-01：意图识别语义化重构（动作-目标解析器）

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-09

## 1. 用户可见症状

- 用户明确编辑请求被误判为 `Answer`：日志 turn `t-1786318868352-4` 输入「修复后把标题改为红色」→ `intent_type=answer` → `allowedTools=[]`、当前文档未注入，模型只能话术回复。
- 同类缺陷：日志历史「根目录创建一个md文档」→ 同样解析为 `Answer`（create_signal 只匹配固定短语「创建文档/新建文档」）。
- 根因：`edit_signal` 有「改**成**」无「改**为**」；「修复」不在列表；create 信号词覆盖不足；判定依赖纯关键词包含，无动作-目标结构。

## 2. 入口点

- 后端意图判定：`src-tauri/src/ag/context.rs` — `resolve_intent_result`（BR-AG-DATA-004-RB-AG-AG-TURN-015，term_ref TERM-AG-012）。
- 历史沿用：`decide_intent_carryover` / `scope_history_for_current_doc`（复用，不修改语义）。

## 3. 相关规则

- `BR-AG-DATA-004`：当前轮 IntentResult 优先；判定维度与阈值属实现期；1.0 不要求通用规划器/小模型分类器，允许规则优先，但不得退化为单一布尔值；IntentResult 必须同时约束 Context 选择、Prompt 任务声明和工具允许集。
- `BR-AG-DATA-001`：上下文来源限定、显式引用优先。
- `BR-AG-STATE-002`：工具注册与逐回合授权（CapabilityGrant 由 IntentResult 驱动）。
- `X-FORBID-006`：内容修改须经 diff 裁决。

## 4. 相关链路

`AG-TURN`（意图判定子路径；不跨链路）。

## 5. 生产者块

- `BR-AG-DATA-004-RB-AG-AG-TURN-015` 位于 `src-tauri/src/ag/context.rs`（resolve_intent_result，@GOV 保持不变，语义复核）。

## 6. 消费者块

- `resolve_capability_grant` 位于 `src-tauri/src/ag/tool_registry.rs`（消费 IntentResultV1 输出）。
- `assemble_agent_turn_messages` / `ag_start_turn` 位于 `src-tauri/src/lib.rs`（消费 IntentResult 的 required_context / mutation / operation）。
- `IntentResultV1` 为 TERM-AG-012 承载，契约不变。

## 7. 路径上的空规则块

- 未发现需修改且已标注 `rules: []` 的决策块。若 audit 产生 BLOCK_UNANNOTATED，在本 Scope 内按真实职责补标或记录 triage。

## 8. boundary 假设

- `lifecycle_assumption=`：`resolve_intent_result` 为纯函数、无副作用；重构不改变其调用时序。
- `document_snapshot=`：当前文档 LogicalState 仍为编辑意图的必要条件（P0 主事实），`detect_target` 对 DocElement 目标与 active_doc_path 联合判定。

## 9. 运行时假设

- `caller_context=async tauri command`：resolve 在回合装配早期同步调用，无 I/O。
- `runtime_guard=deterministic rules`：不引入模型/小模型；疑问/否定/咨询拦截均为确定性规则。
- `runtime_guard=IntentResult drives grant`：意图类型正确后 CapabilityGrant 自动授予对应工具，无需改动 grant 逻辑。

## 10. 疑似不变量破坏

- 中风险：编辑/创建识别范围扩大后，`resolve_capability_grant` 对 Edit/CreateDocument 的授权行为不变，但命中面变大——必须确保疑问句（吗/？/是否）与咨询（怎么修改）不误授写工具（通过修饰语拦截保证）。
- 中风险：`IntentResultV1` 契约不变，但 `operation`/`mutation` 语义增强；既有测试若依赖旧 Answer 判定需同步更新为正确语义，不得弱化断言。
- 未发现需要修改 ADU/ADP/AGENTS/APC 或新增 RULE/CHAIN/CONSTRAINT/TERM 的情况；`IntentSignal` 为函数内私有中间结构，不注册 TERM。

## 11. 验证计划

- 单元测试：`cargo test --lib ag::context`（新增矩阵正反例、修饰语用例、两实例回归）。
- 影响测试：`npm run test:plan -- --files [changed_files]` 后 `npm run test:related -- --files [changed_files]`（AG critical，含 Rust 全量与直接消费者）。
- 治理：`npm run governance:generate`（代码块 @GOV 语义复核）后 `npm run governance:audit`。
- 残留检查：`grep` 确认旧信号列表/旧分支零引用。

## 文件范围（Scope Lock 输入）

允许修改：

- 实现：`src-tauri/src/ag/context.rs`
- 设计文档：`docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md`、`docs/01_modules/AG/A-AG-M-T-02_提示词架构设计.md`（登记 V2 语义化判定）
- 派生索引：`docs/00_core/A-CORE-C-R-01_ADUS.md`（仅 generate 生成）
- 治理运行记录：`.governance/issues/AG-INTENT-V2-01.md`、`.governance/active_scope.json`

禁止修改：

- `ADU.md`、`ADP.md`、`AGENTS.md`、`CLAUDE.md`、APC、`governance.config.json`
- `.governance/test-governance.json`、`scripts/**`
- 前端 `src/**`、`src-tauri/tests/**`、`tool_registry.rs`、`lib.rs`、`turn.rs`、`prompts.rs`、ent/cbt/ast/kb/tpl
- 不在允许列表中的文件默认禁止；若验证证明必须扩展，按 ADP SC-01 停止并请求范围扩展。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-08-09 | v1.0 | 初始版本：意图识别语义化重构（动作-目标解析 + 修饰语拦截 + 优先级链），修复「改为红色」「创建md文档」被误判 Answer |

## 12. Task Runtime Report — AG-INTENT-V2-01

### 12.1 任务结果

- 状态：`READY`。
- 完成内容：`resolve_intent_result` 内部重构为 Parsing（动词×目标×修饰语）→ Adjudication（确定性优先级链）；修复两实例误判；新增疑问/否定/咨询/祈使拦截；IntentResultV1 契约不变；旧信号列表与旧分支同次提交清除。
- 范围：仅 `src-tauri/src/ag/context.rs` 与两份设计文档；未修改规则正文、tool_registry、lib.rs、turn.rs、prompts.rs 或前端。

### 12.2 实际变更

- `src-tauri/src/ag/context.rs`：
  - 新增私有 Parsing 结构：`VerbKind`/`SignalStrength`/`TargetKind`/`Modifiers`，及动词矩阵（Edit/Create/Search/Analyze/Inspect × 强/弱）、目标识别（DocElement>Document>Workspace+Create 量词）、修饰语识别（negated/advisory/interrogative/imperative）。
  - 新增 `detect_verb`/`detect_target`/`detect_modifiers` 三个私有纯函数。
  - `resolve_intent_result` 判定链升级为优先级链：advisory→Analyze；疑问+编辑动词+无祈使→Answer（不授写）；mutation_forbidden→只读类动词；强动词直映；弱动词回落（无文档→Clarify，有文档→Edit/Analyze）；寒暄→Answer。
  - 删除旧 `create_signal`/`edit_signal`/`search_signal`/`analyze_signal`/`inspect_signal`/`document_context_signal`/`advisory_only` 列表与旧 if-else 分支；`mutation_forbidden` 语义由 `modifiers.negated` 保留。
  - 新增 3 个测试：`intent_v2_parses_action_target_and_creates_proposal`、`intent_v2_modifiers_block_write_tools`、`intent_v2_weak_signal_falls_to_clarify_or_analyze`。
- 设计文档：A-AG-M-T-01 §5.1（v2.29）、A-AG-M-T-02 §4（v1.16）登记 V2 语义化判定实现。

### 12.3 验证结果

| 验证 | 结果 |
|---|---|
| `cargo test --lib ag::context` | PASS：12 tests（含 3 新 + 既有兼容） |
| `cargo test --lib tool_registry` | PASS：7 tests |
| `cargo test`（Rust 全量） | PASS：299 + 5 + 4 |
| `npm test`（前端全量） | PASS：26 files / 552 tests |
| `npm run governance:generate` | PASS：245 @GOV 块，92 assets / 860 cases |
| `npm run governance:audit` | PASS：0 findings |
| `git diff --check` | PASS |
| 残留 grep | PASS：旧信号变量（create_signal 等 7 个）零引用；仅保留 mutation_forbidden（语义保留） |

### 12.4 语义复核

- `BR-AG-DATA-004`：判定维度属实现期，V2 为规则优先的确定性实现，未引入模型分类器；IntentResultV1 契约与 TERM-AG-012 语义不变；意图仍同时约束 Context/工具集。
- 复用：意图沿用继续走 `decide_intent_carryover`，未新建继承层；`mutation_forbidden_by_current_turn` 约束保留。
- 未发现规则名与 boundary 核心实体不一致；@GOV `BR-AG-DATA-004-RB-AG-AG-TURN-015` 语义复核通过。

### 12.5 未解决项

- 无。真实 provider/Tauri/WKWebView G5 验证属后续阶段，不在本任务范围。

### 12.6 READY 自检

- 范围：通过；仅 Scope 允许列表内文件。
- 验证：通过；test:plan 所选命令全部实际通过。
- 证据：通过；自动化结果如上表。
- 完整：通过；无 TODO、桩、跳过测试。
- 诚实：通过；测试期间真实发现的 3 个边界回归（只读回落、寒暄误判、新建量词）已修复并断言，未弱化既有断言。
- 留痕：通过；未解决项为零，Scope Lock 已同步 READY。
