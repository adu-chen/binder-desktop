# Issue Trace — AG-CTX-01：上下文管理重构（轻量投影 + 任务边界）

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-11

## 1. 用户可见症状

- 「把当前文档改为和参考文档一样」→ 模型读参考/当前文档（工具结果被截断到 1200 字）→ 无法构造提案 → 6 轮耗尽 → AG_EDIT_TOOL_REQUIRED。
- 「列出当前项目的文件树」→ 被误判 Edit → AG_EDIT_TOOL_REQUIRED 误报。
- list_docs 等索引结果被截断 → 模型推断文件树（幻觉风险）。

## 2. 入口点

- `src-tauri/src/ag/tool_exec.rs`：read_doc/list_blocks 返回完整快照（冗余）。
- `src-tauri/src/lib.rs`：trim_tool_results_to_budget 截断（错误范式，移除）；AG_EDIT_TOOL_REQUIRED 文案。
- `src-tauri/src/ag/context.rs`：动词表缺查询词；整篇对齐无检测。
- `src-tauri/src/ag/tool_registry.rs`：工具定义（list_blocks 描述、read_doc blocks 参数）。
- `src-tauri/src/ag/blocks.rs`：快照序列化瘦身。

## 3. 相关规则

- BR-AG-DATA-005：分层预算；数值与门控实现期标定。轻量投影=结构性压缩（零模型调用，业界共识）。
- BR-AG-STATE-002：工具须在本轮 allowed_tools 内；read_doc 仍限 workspace 只读。
- BR-AG-DATA-004：判定维度实现期；动词表补齐不改变契约。
- 无规则正文改动。

## 4. 相关链路

AG-TOOL（只读执行轻量投影）→ AG-TURN（工具循环去截断、文案区分）→ 意图识别（动词补齐、对齐检测）。

## 5. 生产者块

无新增 RULE/CHAIN。

## 6. 消费者块

- tool_exec read_doc/list_blocks 消费快照 → 轻量投影 + blocks 范围参数。
- lib.rs 工具循环 → 移除 trim 截断。
- context.rs 意图判定 → 查询动词 + 对齐检测。

## 7. 空规则块

未发现；audit 若产生 BLOCK_UNANNOTATED 在本 Scope 补标或 triage。

## 8. boundary 假设

- `lifecycle_assumption=`：list_blocks 轻量投影纯只读；read_doc blocks 范围参数只读；无副作用。
- `document_snapshot=`：块目录可独立于 node_json 投影（block_id/type/text/fingerprint/capabilities）。

## 9. 运行时假设

- 模型提案所需 node_json 经 read_doc(blocks) 按需获取（提示词已要求 base+blockId+fingerprint 原样带回）。
- 移除截断后预算仍可控（轻量投影使单次工具结果减小 80%+）。
- 查询动词命中 → Search 只读组，不触发 requires_proposal。

## 10. 疑似不变量破坏

- 中风险：list_blocks 轻量投影可能影响依赖完整块的既有测试/流程——需同步 tool_exec 测试断言。
- 中风险：read_doc blocks 参数向后兼容（省略=全量），不破坏既有调用。
- 低风险：动词补齐可能改变个别既有用例判定（测试需同步确认）。
- 未发现需改规则正文的情况。

## 11. 验证计划

- 单元：list_blocks 轻量投影字段断言；read_doc blocks 参数；查询动词→Search；对齐检测；文案分支。
- 影响：test:plan → test:related（AG critical）。
- 治理：governance:generate → governance:audit。
- 日志回归：dev 重定向日志复测「改为和参考一样」「列出文件树」。

## 文件范围（Scope Lock 输入）

允许修改：

- `src-tauri/src/ag/tool_exec.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/ag/context.rs`
- `src-tauri/src/ag/tool_registry.rs`
- `src-tauri/src/ag/blocks.rs`
- `src-tauri/src/ag/prompts.rs`
- `docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md`、`docs/01_modules/AG/A-AG-M-T-02_提示词架构设计.md`
- `docs/00_core/A-CORE-C-R-01_ADUS.md`（仅 generate 派生）
- `.governance/issues/AG-CTX-01.md`、`.governance/active_scope.json`

禁止修改：

- ADU/ADP/AGENTS/CLAUDE/APC、`governance.config.json`、`.governance/test-governance.json`、`scripts/**`
- 前端 `src/**`、`src-tauri/tests/**`、`turn.rs`、`tool_registry.rs` 工具授权逻辑（仅工具定义文本）、编辑器路径
- 不在允许列表默认禁止；需扩展按 ADP SC-01 停止请求。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-08-11 | v1.0 | 初始版本：上下文管理重构——移除截断（错误范式）、list_blocks 轻量投影、read_doc blocks 范围参数、查询动词补齐、整篇对齐检测、文案区分、序列化瘦身 |

## 12. Task Runtime Report — AG-CTX-01

### 12.1 任务结果

- 状态：`READY`。
- 完成内容：上下文管理从"事后截断"重构为"源头轻量投影+按需读取"（Claude Code Tool Result Budget 同思路），解决「改为和参考一样」被截断致无法提案、「列出文件树」被误判 Edit 等真实失败。
- 范围：tool_exec / lib.rs / context.rs / tool_registry（工具定义）/ blocks（序列化）与两份设计文档；规则正文零改动、前端零改动、编辑器路径零改动。

### 12.2 实际变更

- **F1 轻量投影 + 按需读取**：
  - `list_blocks`（tool_exec.rs）：只返回块目录（block_id/block_index/node_type/role/parent/depth/text 前40字/fingerprint/capabilities），省略 node_json/全文/marks/attrs。
  - `read_doc`（tool_exec.rs）：新增可选 `blocks` 范围参数——只返回指定块完整投影（含 node_json），省略整篇 content；不传则全量（向后兼容）。
  - `lib.rs`：删除 `trim_tool_results_to_budget` 及其调用——不再截断任何工具结果。
  - 工具定义（tool_registry.rs）：read_doc 增加 blocks 参数说明；list_blocks 描述改轻量目录。
- **F2 查询动词**（context.rs）：`SEARCH_VERBS_STRONG` 补 列出/列举/显示/展示/有哪些/看看有什么 → 查询请求判 Search 只读组，不再默认 Edit。
- **F3 整篇对齐检测**（context.rs + lib.rs）：`detect_sync_from_reference` 检测「改成和 X 一样/替换成 X」→ intent_hint 注入能力边界提示（block_ops ≤24 引导分批/关键章节），不阻断。
- **F4 文案区分**（lib.rs）：`AG_EDIT_TOOL_REQUIRED` 两处按 `any_tool_called` 分支——「已读未提案」（引导直接 propose_edit）vs「零工具调用」（引导先读再提案）。
- **F5 序列化瘦身**（blocks.rs）：`AgentEditableBlock` 空 marks 用 `skip_serializing_if` 省略、空 block_attrs 用 `is_empty_object` 省略。
- 测试：新增 4 个（list_blocks 轻量投影无 node_json、read_doc blocks 范围、查询动词→Search、整篇对齐检测）。

### 12.3 验证结果

| 验证 | 结果 |
|---|---|
| `cargo test`（Rust 全量） | PASS：305 + 5 + 4 |
| `npm test`（前端全量） | PASS：26 files / 552 tests |
| `npm run governance:generate` | PASS：245 @GOV 块，92 assets / 866 cases |
| `npm run governance:audit` | PASS：0 findings |
| `git diff --check` | PASS |

### 12.4 语义复核

- `BR-AG-DATA-005`：预算数值与门控实现期标定；轻量投影=结构性压缩（源头减量），不改变审计口径。
- `BR-AG-STATE-002`：read_doc 仍限 workspace 只读；查询动词命中 → Search 只读组，不触发 requires_proposal；工具仍须在 allowed_tools 内。
- `BR-AG-DATA-004`：判定维度实现期；IntentResultV1 契约不变（sync_from_reference 是辅助提示，不改 intent/授权）。
- `X-FORBID-004`：只读范围语义不变（workspace 内）。
- 规则正文零改动；F3 为提示注入不阻断，F4 为文案区分。

### 12.5 未解决项

- 无。真实 provider/Tauri 验证属后续阶段；dev 重定向日志 `/tmp/binder-audit.log` 可复测「改为和参考一样」→ 模型经 read_doc(blocks) 按需读取后成功提案；「列出文件树」→ intent_type=Search 非 Edit。

### 12.6 READY 自检

- 范围：通过；仅 Scope 允许列表内文件。
- 验证：通过；test:plan 所选命令全部实际通过。
- 证据：通过；自动化结果如上表。
- 完整：通过；无 TODO、桩、跳过测试。
- 诚实：通过；F2 更新查询动词判定按新语义断言，未弱化安全断言（写入仍 HITL、只读范围不变）。
- 留痕：通过；未解决项为零，Scope Lock 已同步 READY。
