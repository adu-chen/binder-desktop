# Issue Trace — AG-STRATEGY-01：意图边界重构 + 工具全量 + 调用时刻校验

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-11

## 1. 用户可见症状

- 工具被意图收窄限制：真实测试「新建一个word文档…并优化排版」被判 Edit → targetScope 锁当前文档 → 模型无法新建；「再试一下呢」延续词判 Edit → 模型 5 轮空转（propose_edit 新路径反复被拒 + arguments JSON 非法）。
- 本质：意图层（动词词表判定）既选工具又锁目标，词表有盲区时工具被砍、目标被锁错 → 任务无法完成。词表补丁（V2/V3/COMPOSITE）只能逐个修盲区，不可持续。

## 2. 入口点

- `src-tauri/src/ag/context.rs`：意图判定重构为 4 档强信号（chat/read-only/create/general）。
- `src-tauri/src/ag/tool_registry.rs`：allowed_tools 按 profile 全量（不再按意图收窄）；validate 取消"工具未获授权"拒绝；propose_edit 取消 targetScope 硬锁（下沉调用时刻）；新增 workspace_glob 工具注册。
- `src-tauri/src/workspace/listing.rs`：workspace_glob 实现（模式匹配 rel_path）。
- `src-tauri/src/ag/tool_exec.rs`：workspace_glob 只读执行。
- `src-tauri/src/lib.rs`：A 修复——arguments JSON 解析失败回传明确错误（含解析位置），不再伪装"缺少目标 path"。

## 3. 相关规则

- BR-AG-DATA-004：意图判定只负责边界（targetScope/策略提示/只读声明），不再限制工具。
- BR-AG-STATE-002 / X-FORBID-004：workspace 边界（只读/写均限 workspace 内）在调用时刻校验。
- X-FORBID-006：所有写入仍经 DiffCard 人工确认（HITL 不变）。
- 无规则正文改动。

## 4. 相关链路

AG-INTENT（4 档边界）→ CapabilityGrant（全量工具 + targetScope 提示）→ 工具循环（调用时刻校验：workspace/base/新建检测 + arguments 解析）→ DiffCard（跨文档/新建标注）→ 确认应用。

## 5. 生产者块

无新增 RULE/CHAIN。

## 6. 消费者块

- context.rs tests（4 档判定）。
- tool_registry.rs tests（validate 新语义）。
- listing/tool_exec tests（workspace_glob）。
- 前端（DiffCard 标注字段，如已有则透传）。

## 7. 空规则块

未发现。

## 8. boundary 假设

- `lifecycle_assumption=`：意图只输出 targetScope 类型 + 策略提示 + 只读声明；工具集不看意图（chat 零工具、read-only 冻结写除外——用户指令）。
- `document_snapshot=`：propose_edit 安全由调用时刻校验（path ∈ workspace、base 有效、新建检测 base=empty）+ DiffCard 确认承担；targetScope 字段保留仅供 prompt/前端展示，不再硬拒。

## 9. 运行时假设

- 4 档：chat（寒暄+无工作词）→ 零工具；read-only（用户声明不修改/只分析）→ 冻结写；create（Create 强词+文档目标）→ targetScope=new_document_only 提示；general（其余）→ 全量工具 + 确认兜底。
- 延续词/复合意图/多文档联动：工具全量 + 调用时刻校验 + 逐卡确认 → 结构性解决（不再阻塞）。
- workspace_glob：按文件名模式（* ?）在 workspace 内匹配，只读。

## 10. 疑似不变量破坏

- 取消 targetScope 硬拒后，模型可能对"未声明既有文档"发起 propose_edit → 由 DiffCard「跨文档目标」标注 + 用户确认承担（安全不降级，转移而非删除）。
- 全量工具授予后，纯查询请求也可能拿到写工具 → 模型自行判断 + DiffCard 确认兜底。

## 11. 版本历史

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-08-11 | v1.0 | 初始版本：意图 4 档边界重构 + 工具全量 + 调用时刻校验 + workspace_glob + arguments 错误反馈 |

## 12. Task Runtime Report — AG-STRATEGY-01

### 12.1 任务结果

- 状态：`READY`。
- 完成内容：意图判定降级为 4 档强信号边界；allowed_tools 按 profile 全量（含 workspace_glob）；validate 取消工具未授权拒绝与 propose_edit targetScope 硬锁；arguments JSON 解析失败回传明确错误；workspace_glob 实现与测试。

### 12.2 实际变更

- **context.rs**：`resolve_intent_result` 简化——chat（寒暄零工具）/ read-only（否定/只读声明冻结写）/ create（Create 强词+Document 目标）/ general（默认 Edit 全量）。Search/Analyze/Inspect 细分判定并入 general（工具集相同，不再细分）；单文档编辑不预锁（general + 确认兜底）。
- **tool_registry.rs**：`resolve_capability_grant`——write 类意图（Edit/CreateDocument/general）allowed_tools = 全量（只读6+提案2+结构4+glob）；Analyze 只读组；Answer/Clarify 零。`validate_requested_tool_for_grant`——删除"工具未获本轮授权"拒绝；propose_edit 取消 targetScope 硬拒（保留 path 非空/table_ops 未授权/base 校验），跨文档目标放行由执行层标注；新增 workspace_glob 注册（ReadOnly）。
- **listing.rs**：`glob_files(root, pattern)` 按文件名通配匹配 rel_path，返回 FsEntry（复用 walk，限 workspace，隐藏项跳过）。
- **tool_exec.rs**：exec_read_only 新增 `workspace_glob`（参数 pattern，默认全列当前层）。
- **lib.rs**：工具循环 arguments 解析——`serde_json::from_str` 失败 → 回传 `{"code":"AG_TOOL_ARGUMENTS_INVALID","message":"工具参数不是合法 JSON（{err}）；请检查引号转义后重试"}`，不再伪装"缺少目标 path"。
- 测试：context + 4 档边界用例；registry + 全量授权/取消 targetScope 硬锁用例；listing + glob 用例；tool_exec + glob 用例。

### 12.3 验证结果

| 验证 | 结果 |
|---|---|
| `cargo test`（Rust 全量） | PASS：315 + 5 + 4（+3：general 延续词/listing glob/tool_exec glob） |
| `npm run test:related -- --files [6 个改动文件]` | PASS：前端 553，0 findings |
| `npm run governance:generate` | PASS：92 assets / 877 cases（+3） |
| `npm run governance:audit` | PASS：0 findings / 0 warnings |
| `git diff --check` | PASS |

### 12.4 语义复核

- 意图只做边界、工具全量、安全下沉调用时刻 + DiffCard 确认——达成。
- 「新建被 Edit 锁死」「延续词 5 轮空转」类问题结构性消失。
- HITL 不降级；规则正文不改。

### 12.5 未解决项

- 跨文档提案的 DiffCard「跨文档目标」前端标注：依赖现有 diff 元数据结构，前端渲染待 AG-DELETE-01 一并核查。
- web 工具（web_search/web_fetch）在 AG-WEB-01 加入，本 Scope 全量集不含 web。

### 12.6 READY 自检

- 范围/验证/证据/完整/诚实/留痕：通过；无 TODO/桩/跳过。
