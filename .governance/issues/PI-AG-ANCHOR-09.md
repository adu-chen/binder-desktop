# Issue Trace · PI-AG-ANCHOR-09 TaskRun 已验证锚点复用与续片段事实

| 项 | 内容 |
|---|---|
| 状态 | READY（自动化通过；新消息入口待产品决策） |
| 严重级别 | P1（同一 TaskRun 反复分页找块，长文每块都重新定位） |
| 发现来源 | 2026-10-31 长文日志 + 人类确认的 v3 计划 M3 |
| 影响模块 | AG / SYS |
| 人类确认 | YES（2026-10-31「继续下一步」，承接已确认的 v3 计划 M3） |

## 1. 用户可见症状

- 同一 TaskRun 的每个续片段都重新 `search_text` / `read_doc` / `list_blocks` 定位同一批块，分页次数高。
- 模型引用的 blockId 与上一片段已确认的锚点重复，但没有跨回合事实可复用。

## 2. 根因

| 编号 | 根因 | 位置 |
|---|---|---|
| R1 | `task_runs.steps_json` 只记录 `content_chunk`，没有已验证锚点 | src-tauri/src/ag/task.rs |
| R2 | 续片段只注入 PreviewAck 的 `createdBlockIds`，不注入已读块锚点 | src-tauri/src/ag/turn_runtime.rs |
| R3 | 没有版本/哈希失效校验，存在复用旧锚点的风险 | 同上 |

## 3. 相关规则

- BR-SYS-STATE-008（TaskRun/续片段）
- BR-AG-DATA-008（上下文事实）
- BR-AG-DATA-005（读取与分页）

## 4. 相关链路

AG-TURN → AG-TOOL → AG-TURN

## 5. 生产者块

- `record_verified_anchors` / `verified_anchors`（src-tauri/src/ag/task.rs）
- `verified_anchors_from_read` 与 `record_tool_result` 写入点（src-tauri/src/ag/turn_tool_loop.rs）
- 续片段注入（src-tauri/src/ag/turn_runtime.rs）

## 6. 消费者块

- 后续片段 `continuation_messages`
- TaskRun 进度/锚点注入消息
- 模型后续 `propose_edit` 的锚点选择

## 7. 路径上的空规则块

无。

## 8. boundary 假设

- 锚点只是控制摘要，不含正文、不授予读写权限、不替代 `read_doc` 或 `propose_edit` 的现有校验。
- 只有目标属于本轮 `ExplicitEditTargetSet` 且 `read_doc(blocks)` 成功才记录。
- 只复用同一 TaskRun 内的锚点，不跨任务、不跨文档。

## 9. 运行时假设

- 锚点以 `(relPath, blockId)` 去重，最新一次移到队尾，最多 20 条。
- 有当前块快照时按 `blockId + fingerprint` 校验；无块快照时退回 `currentDocument.relPath/sessionVersion/contentHash` 三者匹配；都没有时不注入。
- PreviewAck 的 `createdBlockIds` 与锚点合并去重；最多注入 8 条。
- P1-4“任意新用户消息续接”需要显式入口；当前覆盖 PreviewAck、DecisionRequest 确认与显式 TaskRun 续跑，普通新消息仍不自动挂 TaskRun（避免吞掉新任务意图）。

## 10. 疑似不变量破坏

- 风险：锚点过期导致改错块。约束：版本/哈希任一不匹配即丢弃；注入文案明确“不得据此跳过必要读取”。
- 风险：steps_json 膨胀。约束：最多 20 条，仅身份/类型/指纹/标签。
- 风险：跨任务复用。约束：读取/写入都带 task_run_id + workspace + session，终态 run 不写入。

## 11. 验证计划

- Rust：锚点去重/淘汰/错误 session/终态不写入；`read_doc(blocks)` 结果解析；续片段注入版本过滤。
- 回归：`cargo test`、前端 689、`tsc`、`governance:audit`。
- 真实长文：待人工执行（分页次数与锚点命中率需日志观察）。

## 12. Task Runtime Report（2026-10-31）

| 项 | 结果 |
|---|---|
| task_type / phase | CODE_CHANGE / VALIDATION → READY（仅锚点与续片段注入） |
| changed_files | `src-tauri/src/ag/task.rs`、`src-tauri/src/ag/turn_tool_loop.rs`、`src-tauri/src/ag/turn_runtime.rs`、`docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md`、`docs/40_validation/A-VAL-X-V-04_Agent1.0验证卡.md`、本文件 |
| Rust | `cargo test`：543 lib + 5 core_chain + 4 sm_broadcast 全通过；新增 `verified_anchors_are_scoped_deduped_and_capped` |
| 前端 | 本工作包未改前端；沿用 `npx vitest run --exclude ".p1-spike/**"` 689 用例基线 |
| 治理 | `test:plan` 通过；`governance:audit` findings/warnings 为空；ADUS 重新生成（测试用例 1221） |
| 验证边界 | 未运行真实长文；锚点注入需要块快照或 currentDocument 版本/哈希匹配；普通新消息续接入口未实现 |
| unresolved_items | P1-4 普通新消息显式续接入口（需产品决策/UI）；真实锚点命中率未测；`.governance/active_scope.json` 未切换 |
