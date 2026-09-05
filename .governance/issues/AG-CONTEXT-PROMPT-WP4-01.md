# Issue Trace — AG-CONTEXT-PROMPT-WP4-01：Context & Prompt 当前事实收敛

文档状态：A
任务类型：CODE_CHANGE
治理阶段：VALIDATION
创建时间：2026-09-04

## 1. 用户可见症状

Agent 首次模型调用可能看不到已打开文档的当前 LogicalState；历史沿用仍由中文关键词预判，工具结果、
用户补充和逐项引用没有完整来源与裁剪追踪。总 Prompt 预算仍处于一次手动测试留下的 100 万临时放宽，
使预算门形同虚设，并增加跨会话内容误注入和动态层未真实重建的风险。

## 2. 入口点

- `src/services/agService.ts::startTurn`：冻结当前 workspace/session、历史和 AgentContextFacts 后发起回合。
- `src-tauri/src/ag/turn_runtime.rs::ag_start_turn`：核验当前事实并首次构建 PromptPackage。
- `src-tauri/src/ag/turn_prompt.rs::prepare_turn_prompt`：装配当前文档、引用、历史、续接消息和能力投影。
- `src-tauri/src/ag/turn_tool_loop.rs::run_turn_tool_loop`：ToolResult 后重新组装并再次调用 provider。

## 3. 相关规则

- BR-AG-DATA-001、BR-AG-DATA-002、BR-AG-DATA-004、BR-AG-DATA-005、BR-AG-DATA-008。
- BR-AG-STATE-001、BR-AG-STATE-002、BR-AG-STATE-004、BR-AG-VERIFY-001。
- BR-REF-DATA-003、BR-REF-VERIFY-001、BR-SYS-DATA-007、BR-SYS-DATA-008。
- X-FORBID-004、X-FORBID-006、X-SEQ-001。

## 4. 相关链路

- AG-TURN：当前事实冻结 → PromptPackage 装配 → Gate → provider → ToolResult/用户补充后重装配。
- AG-TOOL：只读工具结果作为 data-only 当前回合事实返回，不提升能力或目标。
- REF-INJECT：DocRef、SelectionRef、显式记忆/知识/模板/外部内容逐项校验与来源投影。

## 5. 生产者块

- `BR-AG-DATA-008-DATA-AG-AG-TURN-041` 位于 `src/services/agService.ts`，生产 AgentContextFacts。
- `BR-AG-DATA-001-QUERY-AG-AG-TURN-010` 位于 `src-tauri/src/ag/blocks.rs`，生产 LogicalState/DiskState 快照。
- `BR-AG-DATA-001-DATA-AG-AG-TURN-030` 位于 `src-tauri/src/ag/turn_reference.rs`，生产显式引用上下文。
- `BR-REF-DATA-003-DATA-REF-REF-INJECT-002` 位于 `src-tauri/src/reference/context.rs`，生产引用格式与失效语义。
- `BR-AG-DATA-004-DATA-AG-AG-TURN-003/005` 位于 `src-tauri/src/ag/context.rs`，当前生产关键词 Carryover 与历史分流。

## 6. 消费者块

- `BR-AG-DATA-001/002/004/005/008-DATA-AG-AG-TURN-011` 位于 `src-tauri/src/ag/prompt_runtime.rs`，消费并分层组装消息。
- `BR-AG-DATA-002/008-DATA-AG-AG-TURN-031` 位于 `src-tauri/src/ag/turn_prompt.rs`，消费来源信封、当前 grant 与动态续接消息。
- `BR-AG-STATE-001/004/BR-AG-DATA-008-API-AG-AG-TURN-018` 位于 `src-tauri/src/ag/turn_runtime.rs`，消费首次 PromptPackage。
- `AG-TURN` ToolLoop owner 位于 `src-tauri/src/ag/turn_tool_loop.rs`，消费重建后的 PromptPackage。

## 7. 路径上的空规则块

本路径的纯序列化、hash、消息元数据剥离等技术辅助函数可保持 `rules: []` 或由已登记主责块覆盖；未发现
需要先行 triage 的未知规则块。本包不新增业务规则、链路或工具。

## 8. boundary 假设

- 已打开文档一律由 OpenDocSnapshot LogicalState 供给；未打开文档才在引用读取时取得当前 DiskState。
- PromptDataEnvelope 的正文只能作为 data-only，来源、freshness、hash、trust、token 与裁剪原因可复核。
- 当前用户输入始终最后且完整；长当前文档不截前部，而以 manifest + 先读守卫降级。
- ToolResult、澄清补充和裁决结果属于同一 AgentTurn 的动态事实，每次 provider 调用前重新装配。

## 9. 运行时假设

- `ag_start_turn` 已以 active workspace、session_id、turn_id 校验 AgentContextFacts 和活动回合身份。
- 前端历史当前来自活动 AgentSession，但后端尚未要求每条 HistTurn 自带 workspace/session 身份；本包将失败关闭。
- ToolLoop 顺序追加 assistant ToolCall 与 ToolResult；Prompt 重建必须保留 provider 所需的对应关系。
- CapabilityGrantV2 仍是唯一工具可见性与执行授权来源；任何数据正文不能改变它。

## 10. 疑似不变量破坏

- 首次 `IntentResultV2::unresolved()` 不要求 current_document，导致当前文档事实被错误省略。
- `decide_intent_carryover` 和 `detect_sync_from_reference` 在生产 Prompt 路径用关键词决定历史/策略，违反 BR-AG-DATA-004。
- 引用只生成一个聚合 envelope，无法逐项追踪 selection stale、DiskState 时点与各项裁剪原因。
- Prompt trace 未覆盖当前用户输入和 continuation messages，ToolResult/补充后 trace 可保持不变。
- `PROMPT_TOKEN_BUDGET=1_000_000` 是临时测试旁路；`history_budget_chars` 还重复计算当前文档字符。
- HistTurn 缺 workspace/session 身份，后端无法证明跨 session/workspace 输入被拒绝。

## 11. 验证计划

- 当前事实：首次调用始终注入活动 LogicalState；长文档仅 manifest 且未读前编辑失败；未打开 DocRef 记录 DiskState 读取时点。
- 引用：Doc/Selection/Session/Memory/Knowledge/Template/URL/LongText/File 逐项 envelope；stale、读取失败和裁剪原因可追踪。
- 历史：不再用关键词分流；同 session 有界、跨文档弱标注；错误 workspace/session 身份在 provider 前失败关闭。
- 动态重建：ToolResult、澄清补充和裁决续接改变 context trace；每次仍只投影当前 CapabilityGrant。
- 预算：恢复受控总预算，当前输入和当前文档 P0 不静默截断，引用/历史裁剪确定性且可追踪。
- 安全负向：数据正文中的“忽略规则/调用写工具”不改变 manifest、目标集或执行授权；自动 AST/KB/TPL 调用为零。
- 运行实际 changed_files 的 `test:plan` 与同输入 `test:related`，并执行 generate/audit、Rust 格式检查和禁用标识扫描。

## 12. 文件范围结论（Scope Lock 输入）

允许修改：本 Scope/Issue/ADUS；AG Context/Prompt/Reference/TurnRuntime/ToolLoop owner；前端当前会话历史身份
生产者；如 @GOV 索引变化仅由 generator 刷新 ADUS。

禁止修改：需求、设计、规则、协议、验证卡、计划、ADU/ADP/AGENTS/APC、cloud/dev-gateway、REF 公共载荷
协议，以及 WP5 Evidence/ProjectReport、WP6 Task Projection、WP7 ProposalBatch、WP8 UI 投影和 WP9 全库清理。

## Expected Behavior

每次 provider 调用都以当前 AgentTurn 的活动 LogicalState、完整当前输入、逐项可追踪显式引用和同回合动态事实
重新构建 PromptPackage；历史只作为当前 workspace/session 内的有界 DATA-ONLY 候选。任何文档、引用、历史、
ToolResult、显式记忆/知识/模板或外部正文都不能扩大当前 CapabilityGrantV2、目标集合或副作用权限。

## Validation Commands

- `npm run governance:generate`
- `npm run governance:audit -- --issue .governance/issues/AG-CONTEXT-PROMPT-WP4-01.md`
- `npm run test:plan -- --files [...]`
- `npm run test:related -- --files [...]`
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
- `cargo check --manifest-path src-tauri/Cargo.toml --lib --tests`
- `git diff --check`

## Scope Lock

```yaml
task_type: CODE_CHANGE
governance_phase: VALIDATION
allowed_files:
  - .governance/active_scope.json
  - .governance/issues/AG-CONTEXT-PROMPT-WP4-01.md
  - docs/00_core/A-CORE-C-R-01_ADUS.md
  - src-tauri/src/ag/context.rs
  - src-tauri/src/ag/blocks.rs
  - src-tauri/src/ag/reference_runtime.rs
  - src-tauri/src/ag/turn_reference.rs
  - src-tauri/src/ag/prompt_runtime.rs
  - src-tauri/src/ag/prompts.rs
  - src-tauri/src/ag/turn_prompt.rs
  - src-tauri/src/ag/turn_runtime.rs
  - src-tauri/src/ag/turn_tool_loop.rs
  - src/services/agService.ts
forbidden_files:
  - ADU.md、ADP.md、AGENTS.md、APC、需求/设计/规则/协议/验证卡/计划、公共 ReferencePayload 协议、cloud/dev-gateway、WP5-WP9 文件
confirmation: YES
scope_reason: 只按已确认 WP4 收敛当前事实、来源追踪、历史隔离、动态重装配与真实预算，不新增需求、工具或产品能力。
```

## 13. 人类确认记录

2026-09-04：人类已确认按推荐方案完整执行冻结计划并持续要求继续；本工作包为已确认的 WP-AG10-04，
`confirmation: YES`。

## 14. Task Runtime Report

```yaml
task_id: AG-CONTEXT-PROMPT-WP4-01
task_type: CODE_CHANGE
final_state: READY
governance_phase: VALIDATION
changed_files:
  - .governance/active_scope.json
  - .governance/issues/AG-CONTEXT-PROMPT-WP4-01.md
  - docs/00_core/A-CORE-C-R-01_ADUS.md
  - src-tauri/src/ag/context.rs
  - src-tauri/src/ag/prompt_runtime.rs
  - src-tauri/src/ag/prompts.rs
  - src-tauri/src/ag/reference_runtime.rs
  - src-tauri/src/ag/turn_prompt.rs
  - src-tauri/src/ag/turn_reference.rs
  - src-tauri/src/ag/turn_runtime.rs
  - src-tauri/src/ag/turn_tool_loop.rs
  - src/services/agService.ts
mapping_chain:
  - REQ-AG-001 -> BR-AG-DATA-001 -> AG-TURN + REF-INJECT -> LogicalState/DiskState and individually traced explicit references
  - REQ-AG-004 -> BR-AG-DATA-004 -> AG-TURN -> current-input-first semantic intent with bounded same-session history
  - REQ-AG-005 -> BR-AG-DATA-005 -> AG-TURN -> deterministic source budgets and traceable truncation
  - REQ-AG-006 -> BR-AG-DATA-002 -> AG-TURN -> PromptDataEnvelope source/freshness/hash/trust/token metadata
  - REQ-AG-007 -> BR-AG-DATA-008 + BR-AG-STATE-004 -> AG-TURN -> frozen editor facts and per-provider-call prompt rebuild
test_impact:
  direct_modules: [AG, CORE, ED, REF, SYS]
  affected_modules: [ACCT, AG, AST, CBT, CORE, DE, DOC, ED, ENT, KB, PRV, REF, SYS, TPL, WS]
  selected_commands:
    - npm test
    - npx tsc --noEmit
    - cargo test --manifest-path src-tauri/Cargo.toml
    - npm run governance:audit
validation:
  - cargo fmt --check: passed
  - cargo check --lib --tests: passed
  - Rust Agent targeted suite: 191/191 passed
  - frontend full suite: 26 files, 596/596 passed
  - Rust lib suite: 404/404 passed
  - Rust core_chain integration: 5/5 passed
  - Rust sm_broadcast integration: 4/4 passed
  - Rust doc tests: 0 failed
  - TypeScript type check: passed
  - governance:generate: passed, ADUS refreshed
  - governance:audit: passed, governance_findings=[]
  - git diff --check: passed
semantic_review:
  result: passed
  findings:
    - 首次 provider 调用不再依赖 unresolved IntentResult 的 required_context，活动编辑器 LogicalState 始终作为当前主事实进入。
    - 历史只按当前 workspace/session、文档归属、近因和预算选择；生产 Prompt 路径不再用关键词预判 Carryover 或同步参考策略。
    - Doc、Selection、Session、Memory、Knowledge、Template、URL、LongText、File 和块引用逐项保留来源身份、新鲜度、hash、trust、字符预算与裁剪原因。
    - ToolResult、用户补充、裁决结果和模型续接均成为同回合类型化来源；每次 provider 调用重建 PromptPackage，trace 随动态事实变化。
    - 数据来源均保持 DATA-ONLY，工具清单只由当前 CapabilityGrantV2 产生；没有自动调用 AST/KB/TPL，也没有新增工具或公共引用协议。
    - 总 Prompt 预算从临时 100 万恢复为经真实最大合法输入测试覆盖的 24k dev 门；G5 仍负责真实 provider 环境最终标定。
system_validation:
  status: not_run
  note: 本工作包完成代码与仓库自动化验证；未执行真实 provider/UI 场景，系统级 V-S8/V-S9/V-S10/V-S12 证据按既定 G5 执行。
validation_environment:
  note: 首次受限环境全量回归仅 LibreOffice DOCX→PDF 用例因系统进程 signal 6 失败；同一用例在正常系统权限下 1/1 通过，随后完全相同 changed_files 的 test:related 全套通过，判定为沙箱环境限制而非 Agent 回归。
stop_conditions: []
unresolved_items: []
```

### READY 自检

- 范围：12 个实际 changed_files 全部位于已确认 allowed_files；未修改需求、设计、规则、协议、验证卡、计划或 WP5-WP9 文件。
- 验证：以同一份实际文件清单运行 `test:plan` 与 `test:related`；选择器要求的前端全量、Rust 全量、类型检查和治理审计均真实通过。
- 证据：自动化结果与尚未运行的真实 provider/UI 系统验证已分开记录；未把单元测试冒充系统验收。
- 完整：无 TODO、占位实现、跳过/弱化测试或未接线的新生产路径；规则标注已从测试包装迁移到真实引用生产 owner。
- 诚实：保留并说明受限环境失败及正常权限下的同输入复测证据；未使用 `skip`、`ignore`、`--no-verify` 或硬编码特例。
- 留痕：WP4 范围内无未解决项；24k dev 预算的真实 provider 最终标定仍按冻结计划 G5 执行，不在本包冒进。
