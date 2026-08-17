# Issue Trace — AG-INTENT-V3-01：乐观授权（固定工具集 + 意图仅注入上下文）

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-10

## 1. 用户可见症状

- 真实轮次（如「恢复被破坏的内容」）被误判 `Answer` → `allowedTools=[]` → 模型只能输出限制声明，无法工作。
- 根因（两轮调研确认）：意图识别被错误地当作"工具授权门"——动词枚举未命中 → 零工具 → 瘫痪。成熟方案（Claude Code 分层权限、Tool Firewall）不依赖前置意图枚举决定工具集，而是"只读默认放开 + 写操作人工确认"。

## 2. 入口点

- 意图判定：`src-tauri/src/ag/context.rs` — `resolve_intent_result`（BR-AG-DATA-004-RB-AG-AG-TURN-015）。
- 工具授权：`src-tauri/src/ag/tool_registry.rs` — `resolve_capability_grant`（BR-AG-STATE-002）。
- 装配消费：`src-tauri/src/lib.rs` — `current_doc_requires_tool_read`。

## 3. 相关规则

- `BR-AG-STATE-002`：工具必须同时在注册清单和本轮 IntentResult.allowed_tools 内（**未锁"必须由意图枚举收窄"**，本方案 allowed_tools 恒为固定集合，规则逐字满足）。
- `X-FORBID-006`：写入不得直接落盘，必须产出修改建议走 DE diff 路径（提案+人工确认=结构性 HITL，安全不变）。
- `BR-AG-DATA-004`：IntentResult 约束工具允许集（required_tools 语义保留）；判定维度属实现期。
- `X-FORBID-004`：只读工具限定 workspace 内。

## 4. 相关链路

`AG-TURN`（意图判定）、`AG-TOOL`（授权与执行）。

## 5. 生产者块

- `BR-AG-DATA-004-RB-AG-AG-TURN-015`（context.rs resolve_intent_result）
- `BR-AG-STATE-002-*`（tool_registry.rs resolve_capability_grant）

## 6. 消费者块

- `ag_start_turn` / `assemble_agent_turn_messages`（lib.rs）消费 intent.required_context / required_tools / target。
- `tool_exec` 执行期防火墙（schema/target/workspace 校验）**保持不变**。

## 7. 路径上的空规则块

- 未发现。若 audit 产生 BLOCK_UNANNOTATED，本 Scope 内按真实职责补标或记录 triage。

## 8. boundary 假设

- `lifecycle_assumption=`：resolve_intent_result 纯函数、无副作用；resolve_capability_grant 同步调用。
- `document_snapshot=`：当前文档 LogicalState 仍在非寒暄、有文档时默认注入（required_context 默认含 current_document_logical_state）。

## 9. 运行时假设

- `caller_context=async tauri command`：回合装配早期同步调用，无 I/O。
- `runtime_guard=read-only by default`：只读工具（read_doc/list_docs/search_text/list_blocks/list_tables/list_images）无条件授予，workspace 限定（X-FORBID-004）。
- `runtime_guard=write=proposal+HITL`：propose_edit/edit_blocked/create_doc 授予但绝不落盘，用户接受 DiffCard 才生效（X-FORBID-006）——结构性安全，无需按意图收窄。
- `runtime_guard=greeting=no tools`：寒暄（你好/谢谢/再见/hello 等）→ Answer 零工具（省 token、语义合理）。

## 10. 疑似不变量破坏

- 中风险：写入工具全授予后，模型可能对只读请求也生成提案——缓解：只读信号（不要修改/只分析）仍识别→降为只读组；即便漏判，提案不落盘+用户拒绝，安全不变。
- 中风险：意图识别弱化后上下文注入可能不足——缓解：非寒暄且 has_active_doc 时默认注入 current_document_logical_state。
- 未发现需改规则正文的情况：BR-AG-STATE-002 锁"allowed_tools 是授权依据"未锁"按意图枚举"；BR-AG-DATA-004 判定维度属实现期。

## 11. 验证计划

- 单元：`cargo test --lib ag::context`、`cargo test --lib tool_registry`（寒暄零工具、文档请求全工具、只读只读、识别失败仍有只读工具）。
- 影响：`npm run test:plan` 后 `npm run test:related`（AG critical）。
- 治理：`npm run governance:generate` + `governance:audit`。
- 残留：grep 确认无"动词表决定工具"残留（动词表仅作上下文意图装饰，不决定工具集）。

## 文件范围（Scope Lock 输入）

允许修改：

- 实现：`src-tauri/src/ag/context.rs`、`src-tauri/src/ag/tool_registry.rs`、`src-tauri/src/lib.rs`
- 设计文档：`docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md`、`docs/01_modules/AG/A-AG-M-T-02_提示词架构设计.md`
- 派生索引：`docs/00_core/A-CORE-C-R-01_ADUS.md`（仅 generate 生成）
- 治理运行记录：`.governance/issues/AG-INTENT-V3-01.md`、`.governance/active_scope.json`

禁止修改：

- `ADU.md`、`ADP.md`、`AGENTS.md`、`CLAUDE.md`、APC、`governance.config.json`
- `.governance/test-governance.json`、`scripts/**`
- 前端 `src/**`、`src-tauri/tests/**`、`turn.rs`、`prompts.rs`、ent/cbt/ast/kb/tpl
- 不在允许列表中的文件默认禁止；若验证证明必须扩展，按 ADP SC-01 停止并请求范围扩展。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-08-10 | v1.0 | 初始版本：乐观授权重构（固定工具集 + 意图仅注入上下文），修复「恢复被破坏的内容」等真实请求被误判 Answer 瘫痪 |

## 12. Task Runtime Report — AG-INTENT-V3-01

### 12.1 任务结果

- 状态：`READY`。
- 完成内容：工具授予从"按意图枚举收窄"改为"三分类固定"（只读组无条件 + 提案组不落盘）；意图识别退化为上下文注入器，识别失败不再导致零工具瘫痪；新增寒暄检测；修复「恢复被破坏的内容」被误判 Answer 的真实缺陷。
- 范围：仅 `context.rs` / `tool_registry.rs` / `lib.rs` 与两份设计文档；未改规则正文、前端、turn.rs、prompts.rs。

### 12.2 实际变更

- `src-tauri/src/ag/context.rs`：
  - `resolve_intent_result` 判定链重构：greeting（寒暄）→ Answer 零工具；咨询 → Analyze；疑问+无动作+无请求词 → Answer；只读信号 → 只读类；弱动词+有文档 → Analyze；**fallback 改为：有文档 → 默认 Edit（全工具）、无文档+创建 → CreateDocument、其余 → Clarify**（不再默认 Answer）。
  - 新增 `detect_greeting` + `GREETING_WORDS`/`WORK_HINT_WORDS`/`REQUEST_HINT_WORDS` 封闭词表（多语言可增量扩展）。
  - `required_tools` 三分类固定：Answer/Clarify 空；Analyze/Inspect/Search 只读组（read_doc/list_docs/search_text/list_blocks）；Edit/CreateDocument 全量（只读组 + propose_edit/edit_blocked）。
  - `target` 判定：默认 Edit 只要 has_active_doc 即指向 CurrentDocument（无需显式指涉词）。
  - 新增测试 `intent_v3_optimistic_grant_never_paralyzes_on_unrecognized_input`（中文/英文任意表达不瘫痪、寒暄零工具、疑问提问、只读只读）。
- `src-tauri/src/ag/tool_registry.rs`：`resolve_capability_grant` 同步三分类固定工具集；create_doc 不授予（结构工具，创建文档走 propose_edit base=empty）。测试断言更新。
- `src-tauri/src/lib.rs`：`requires_edit_tool`/`current_doc_requires_tool_read` 消费确认无改动需求（requires_proposal 语义保持）。
- 设计文档：M-T-01 §5.1（v2.30）、M-T-02 §4（v1.17）登记乐观授权。

### 12.3 验证结果

| 验证 | 结果 |
|---|---|
| `cargo test --lib ag::context` | PASS：13 tests（含新 V3 用例） |
| `cargo test --lib tool_registry` | PASS：7 tests |
| `cargo test`（Rust 全量） | PASS：301 + 5 + 4 |
| `npm test`（前端全量） | PASS：26 files / 552 tests |
| `npm run governance:generate` | PASS：245 @GOV 块，92 assets / 862 cases |
| `npm run governance:audit` | PASS：0 findings |
| `git diff --check` | PASS |

### 12.4 语义复核

- `BR-AG-STATE-002`：规则锁「工具必须在本轮 IntentResult.allowed_tools 内」——allowed_tools 现为固定集合，逐字满足；"注册≠授权"与"逐回合授权"保留（grant 仍绑定 turn_id/scope/参数约束）。
- `X-FORBID-006`：写入组（propose_edit/edit_blocked）绝不落盘、用户接受 DiffCard 才生效——结构性 HITL 不变。
- `BR-AG-DATA-004`：判定维度属实现期；IntentResultV1 契约不变；意图仍约束 Context 选择（required_context）与 Prompt 任务声明。
- 多语言：不再维护动作动词开放集决定工具；动词表仅作意图类型装饰（上下文注入），未命中不影响工具。

### 12.5 未解决项

- 无。真实 provider/Tauri/WKWebView G5 验证属后续阶段（dev 进程重定向日志 `/tmp/binder-audit.log` 可复查「恢复被破坏的内容」类请求的下一轮行为）。

### 12.6 READY 自检

- 范围：通过；仅 Scope 允许列表内文件。
- 验证：通过；test:plan 所选命令全部实际通过。
- 证据：通过；自动化结果如上表。
- 完整：通过；无 TODO、桩、跳过测试。
- 诚实：通过；测试期间真实发现并修复 2 个边界（疑问句含请求词误拦、寒暄词混工作词），断言固化。
- 留痕：通过；未解决项为零，Scope Lock 已同步 READY。
