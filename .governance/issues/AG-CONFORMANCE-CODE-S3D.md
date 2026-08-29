# Issue Trace — AG-CONFORMANCE-CODE-S3D：显式编辑目标集运行时

文档状态：A
任务类型：CODE_CHANGE
治理阶段：VALIDATION
任务状态：READY
创建时间：2026-08-28
上游基线：`35fbf87`；TERM-AG-019、BR-AG-DATA-003；HD-S2-04 / HD-S3D-01。

## 1. 用户可见症状

当前模型可对未显式授权的既有文档生成 PendingDiff，只增加 `cross_doc` 提示。若直接把既有 guard 改成拒绝，任务级 continuation 又没有可继承的权威 ExplicitEditTargetSet，会使合法多文档任务在下一片段失去授权。

## 2. 入口点

- `AG-TURN`：输入/澄清/continuation 组装 ExplicitEditTargetSet。
- `AG-TOOL`：scope DecisionRequest 冻结追加路径，propose_edit 消费集合。
- `DE-DIFF`：PendingDiff 前协同硬拒。
- `DATA-MIGRATE`：若采用后端持久化，为 TaskRun 追加权威关系表或字段。

## 3. 相关规则

- `BR-AG-DATA-003` + `TERM-AG-019`：集合来源、追加、继承与拒绝。
- `BR-SYS-STATE-007`：同 AgentTurn 澄清恢复。
- `BR-SYS-STATE-008`：TaskRun 按需创建与显式 continuation。
- `BR-SYS-PERSIST-001`：workspace.db 版本化、单事务、数据保留。
- `BR-AG-STATE-002`：ToolCall 注册、授权与 DecisionRequest 产物。

## 4. 相关链路

`AG-TURN → AG-TOOL → DE-DIFF`；持久化交叉 `DATA-MIGRATE`；CreateDocumentBundle 的既有 `AG-TURN / AG-TOOL → WS-OPERATION` 承接只在资源成功后追加，不修改 WS 执行实现。

## 5. 生产者块

- `turn_reference.rs::build_turn_reference_runtime`：一次调用内创建 `declared_targets`，无正式类型和继承。
- `context.rs::guard_target_set`：boundary 声明拒绝，实现却返回 `AcceptCrossDocument`。
- `task.rs::propose_decision/resolve_decision/begin_confirmed_continuation`：已持久化 frozen payload 与稳定 TaskRun identity，可承接 scope 确认追加，但当前没有集合存储。
- `db/mod.rs::migrate`：当前最新 schema v15；TaskRun、DecisionRequest、PreviewCheckpoint、WorkspaceOperation 均使用专用表。

## 6. 消费者块

- `turn_prompt` / CapabilityGrant：需读取集合用于模型约束和调用时校验，但本 Scope 不重构最小工具集合。
- `turn_tool_loop`：需把同一集合贯穿工具轮次，scope DecisionRequest 成功后结束片段。
- `edit_runtime::ag_propose_edit`：必须在任何 ChangeProposal/PendingDiff 前消费硬 guard。
- `de::generate`：必须进行第二道集合成员复核，而非只返回未使用的布尔值。
- `route_resolve_decision`：scope confirm 原子追加；CreateDocumentBundle 执行完成后追加真实路径。

## 7. 路径上的空规则块

TERM-AG-019 已注册，但尚无引用该术语的 @GOV 生产 owner。`tool_registry` 的 CapabilityGrant/调用校验 owner 仍是 S2 登记缺口；本 Scope 只登记与 ExplicitEditTargetSet 直接相关的 schema/参数守卫，不借机实施 S3C。

## 8. boundary 假设

- ExplicitEditTargetSet 是授权集合，不是可读集合、Prompt 上下文或 DiffCard 提示。
- 简单 AgentTurn 的集合生命周期不超过活动回合；任务级集合绑定稳定 TaskRun。
- scope DecisionRequest payload 是不可变追加候选，confirm 后才成为权威成员。
- 重复 confirm 必须幂等；不同 outcome 重放失败关闭。

## 9. 运行时假设

- 同 turn_id 澄清前端已合并原 ReferencePayload；后端仍需以活动回合身份维护单调集合。
- 任务级 continuation 可以在 prompt/tool loop 前从 TaskRun 权威集合恢复。
- CreateDocumentBundle 路径可在 `route_resolve_decision` 确认 WorkspaceOperation completed 后追加，无需修改 WS owner。

## 10. 疑似不变量破坏

- JSON 塞入 `task_runs.steps_json` / `intent_summary` 会混淆现有实体，禁止。
- 只依赖前端把引用重新带回 continuation 会把后端授权真值交给临时 UI 状态。
- 仅在 AG 侧拒绝而 DE 侧继续接受，会保留绕过路径。
- scope confirm 与目标追加非原子会出现“已确认但集合未更新”或反向状态。

## 11. 验证计划

1. 迁移单调/幂等、旧库保留及集合唯一性测试。
2. 单测覆盖初始集合、澄清单调追加、read/search 不追加、scope confirm/cancel/重放及 continuation 继承。
3. 单测覆盖 AG/DE 两侧集合外拒绝零 PendingDiff。
4. 保持多文档显式引用、CreateDocumentBundle、DecisionRequest、Preview continuation 与简单无 TaskRun 回归。
5. 运行 changed_files `test:plan` / `test:related`、`test:governance`、`governance:audit`、格式与差异检查。

## 人类裁决

- `HD-S2-04`（CONFIRMED）：先注册追加规则，再实施集合外硬拒。
- `HD-S3D-01`（CONFIRMED）：显式输入/澄清追加；工具发现目标经 scope DecisionRequest 确认追加；TaskRun continuation 继承。
- `HD-S3D-02`（CONFIRMED）：采用方案 1，新增 schema v16 专用 `task_run_edit_targets` 关系表；每行保存 task_run_id、规范化相对路径、来源类型/来源 ID 与授权时间，主键保证幂等；简单 AgentTurn 仍只在内存持有。
- `HD-S3D-03`（CONFIRMED）：以新裁决为权威，先闭合 REQUIREMENT/DESIGN/PLAN 配套规则链，再恢复实现。
- `HD-S3D-04`（CONFIRMED）：LibreOffice DOCX→PDF 稳定失败登记为范围外验证阻断项；不扩大 S3D Scope，继续完成 S3D 证据并提交实现检查点，但不标记 READY。

## Task Runtime Report — S3D ExplicitEditTargetSet 运行时

task_type: CODE_CHANGE
governance_phase: VALIDATION
final_state: READY
issue_trace: `.governance/issues/AG-CONFORMANCE-CODE-S3D.md`

changed_files:
  - `src-tauri/src/ag/target_set.rs`、`db/mod.rs`：新增 ExplicitEditTargetSet 唯一 owner 与 v16 TaskRun 关系表。
  - `src-tauri/src/ag/turn_*.rs`、`task.rs`、`task_runtime.rs`、`tool_registry.rs`：接入初始来源、澄清单调追加、TaskRun 继承、scope confirm 原子追加及 CreateDocumentBundle 建盘后追加。
  - `src-tauri/src/ag/edit_runtime.rs`、`src-tauri/src/de/generate.rs`、`context.rs`：以 AG/DE 双层硬拒替代旧 AcceptCrossDocument 并行语义，守卫先于 PendingDiff 生成与持久化。
  - `docs/40_validation/A-VAL-X-V-04_Agent1.0验证卡.md`、`docs/00_core/A-CORE-C-R-01_ADUS.md`：新增 V-S10 设计证据门禁并刷新派生注册索引。

scope:
  mode: explicit
  status: confirmed

phase_gate:
  status: passed
  evidence: REQUIREMENT `fed343a` → DESIGN `a0afdff` → PLAN `97cf632` 已顺序闭合，HD-S3D-01/02/03 已确认。

mapping_chain:
  status: complete
  evidence: REQ-AG-001/015 → BR-AG-DATA-003、BR-SYS-STATE-007/008、BR-SYS-PERSIST-001 → AG-TURN/AG-TOOL/DE-DIFF/DATA-MIGRATE → @GOV DATA/EFFECT/GUARD/API → Rust 直接测试与 V-S10；ADUS 已生成。AI 语义审计比对 BR-AG-DATA-003 原文与本轮 boundary，核心实体和负向来源一致，未发现 possible_term_drift。

test_impact:
  changed_files: `.governance/active_scope.json`、本 Issue Trace、AG/DE/DB Rust 实现、V-S10 验证卡、ADUS。
  direct_modules: AG, CBT, CORE, DE, ED, SYS, WS
  affected_modules: ACCT, AG, AST, CBT, CORE, DE, DOC, ED, ENT, KB, PRV, REF, SYS, TPL, WS
  selection: frontend-full + rust-full + governance:audit

validation:
  - `cargo fmt --all -- --check`、`cargo check --lib`：passed。
  - S3D 直接 Rust 测试：passed；覆盖 ReferencePayload 身份/非身份隔离、集合归一与持久化、澄清单调性、scope confirm/cancel/replay、CreateDocumentBundle 建盘后追加、TaskRun workspace 身份及 DE 集合外拒绝。
  - `npm run test:plan -- --files [...]`：passed；选择 frontend-full + rust-full + governance:audit。
  - `npm run test:related -- --files [...]`：passed；在测试治理注册要求的非沙箱 macOS 进程中完整重跑，前端 26 文件 592/592 passed，Rust lib 369/369 passed，`core_chain` 5/5、`sm_broadcast` 4/4 与 doc-tests 全部通过，后续 `governance:audit` passed。
  - `cargo test --test core_chain`：5/5 passed；`cargo test --test sm_broadcast`：4/4 passed。
  - `npm run test:governance`：19/19 passed，954 个测试用例已发现、零未归属/skip/focus；`npm run governance:audit`：passed，无治理 finding。
  - `git diff --check`、禁用别名与旧 `declared_targets`/`AcceptCrossDocument` 负空间扫描：passed。

stop_conditions:
  - 无。原 SC-04 已按测试治理注册的环境裁决解除：受限沙箱中 LibreOffice signal 6 可复现，同一精确用例及完整 `test:related` 在非沙箱 macOS 进程中通过。

recovery:
  - 无；SC-04 已解除。

unresolved_items:
  - CapabilityGrant 最小化保持独立 S3C。
  - 真实 Tauri/provider 多片段交互仍按 V-S10 执行，不以自动化冒充 G5。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-28 | v1.0 | 建立 S3D CODE_CHANGE Scope；沿注册链完成运行时/持久化影响映射，提出三种权威集合持久化方案。 |
| 2026-08-28 | v1.1 | 人类确认 HD-S3D-02 方案 1：采用 v16 专用关系表保存 TaskRun 权威 ExplicitEditTargetSet。 |
| 2026-08-28 | v1.2 | 实现中按规则链复核发现四份当前 A 文档仍保留显式引用只读/跨文档写入未冻结语义，触发 SC-01 / SC-07；暂停代码推进，等待静态治理范围裁决。 |
| 2026-08-28 | v1.3 | 人类确认 HD-S3D-03；需求、设计和计划配套治理分别提交，SC-01 / SC-07 解除，恢复 IMPLEMENTATION。 |
| 2026-08-28 | v1.4 | 完成 ExplicitEditTargetSet 实现、ADUS 与 V-S10；关联验证前端 592/592、Rust 368/369，唯一范围外 LibreOffice 用例连续 signal 6。按 HD-S3D-04 记录 SC-04，允许提交实现检查点但保持 BLOCKED。 |
| 2026-08-28 | v1.5 | 按测试治理环境裁决在非沙箱 macOS 进程重跑 S3D 完整 `test:related`；前端 592/592、Rust 369/369、集成与治理审计全绿，解除 SC-04 并进入 READY。 |
