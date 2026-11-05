# Issue Trace · PI-AG-METRICS-11 长任务指标与固定回归基线

| 项 | 内容 |
|---|---|
| 状态 | READY（指标与回归矩阵已定义；提示词瘦身延后） |
| 严重级别 | P2（缺少可对比指标，无法判断长任务优化是否真的改善） |
| 发现来源 | 2026-10-31 长文日志 + 人类确认的 v3 计划 M5 |
| 影响模块 | AG / SYS |
| 人类确认 | YES（2026-10-31「继续下一步」，承接已确认的 v3 计划 M5） |

## 1. 目标

1. 每个回合可导出只读/写入用量、剩余轮次、预览块数与锚点注入数；
2. 固定 1000 / 4000 / 4000+3000 三组长文基线，后续改动可对比；
3. 提示词瘦身只在指标稳定后做，本工作包不删现有约束。

## 2. 指标事件

| 事件 | 字段 | 用途 |
|---|---|---|
| `turn_metrics` | `stopReason`、`remainingReadCalls`、`remainingWriteCalls`、`remainingRounds`、`budgetClosing`、`previewChunkCount` | 回合末额度/进度基线与“提前收工”识别 |
| `task_anchors_injected` | `anchorCount`、`createdBlockIdCount`、`chunkCount` | 锚点命中与去重效果 |
| `execution_budget_checked`（既有） | `maxReadToolCalls`、`maxWriteToolCalls`、剩余读写、轮次 | 每轮额度快照 |
| `terminal_reason`（既有） | `remainingReadCalls`、`remainingWriteCalls`、`remainingRounds` | 终态归因 |
| `diff_proposed` / `context_frozen`（既有） | 卡数、上下文版本 | 卡数/版本一致性 |

事件均为内部记录，不进入模型上下文，不构成业务成功或保存证据。

## 3. 固定回归矩阵

| 用例 | 输入 | 期望 | 证据 |
|---|---|---|---|
| R1 短文 | 约 1000 字单块 | 1 张卡、1 次写入、无全量分页 | `turn_metrics` + 日志 |
| R2 长文 | 约 4000 字、3–5 块 | 每块 1 次写入；PreviewAck 自动续跑；锚点复用；无 332 块分页 | `turn_metrics`、`task_anchors_injected`、调用序列 |
| R3 超长文 | 4000 正文 + 3000 续写 | 全部块形成 pending 卡；统一接受 + 一次保存；中断第 k 块可从 k 续跑且无重复块 | 同 R2 + 人工复核 |

每轮固定记录：只读调用/写入调用、分页次数、锚点命中率、卡数、确认次数、零卡回合占比、错误归因。

## 4. 提示词瘦身

- 本工作包**不删除**任何现有预算/预览/锚点约束；仅登记后续候选：
  1. 合并重复的读取策略句；
  2. 把“额度/预览/锚点”三类说明各自收敛为一段；
  3. 每次删减必须附前后 `prompt_budget_checked` 对比，且 M1–M4 回归全绿。

## 5. 相关规则与链路

- BR-AG-OBS-001（证据与观测）
- BR-AG-STATE-005（额度收尾）
- BR-SYS-STATE-008（TaskRun）
- AG-TURN → AG-TOOL → AG-TURN

## 6. 生产者块

- `settle_completed_turn` 的 `turn_metrics`（src-tauri/src/ag/turn_tool_loop.rs）
- 续片段注入的 `task_anchors_injected`（src-tauri/src/ag/turn_runtime.rs）

## 7. 消费者块

- 开发验证面板/日志分析
- 长文回归对比
- 后续提示词瘦身决策

## 8. boundary 假设

- 指标是内部观测，不改变授权、终态、裁决或保存；
- 不记录正文、工具参数、用户目标或密钥；
- `previewChunkCount` 只来自 `task_runs.steps_json` 的 `content_chunk` 计数。

## 9. 运行时假设

- `turn_metrics` 在终态结算前写入；写入失败按原路径向上传播（与 `terminal_reason` 一致）。
- 无 TaskRun 的回合 `previewChunkCount=0`；锚点计数只在注入成功时记录。

## 10. 疑似不变量破坏

- 风险：指标事件暴露敏感内容。约束：只记录计数、枚举与布尔。
- 风险：指标写入失败吞掉终态。约束：沿用 `record_internal` 的 `?` 传播，不吞。
- 风险：用指标替代真实业务验收。约束：指标事件不是成功证据。

## 11. 验证计划

- `cargo test`、`tsc`、前端 689、`governance:audit`；
- 三组长文基线待人工执行；
- 指标字段与事件存在性由代码审查 + 现有场景事件断言覆盖，不新增物理测试文件。

## 12. Task Runtime Report（2026-10-31）

| 项 | 结果 |
|---|---|
| task_type / phase | CODE_CHANGE / VALIDATION → READY（仅指标与回归矩阵） |
| changed_files | `src-tauri/src/ag/turn_tool_loop.rs`、`src-tauri/src/ag/turn_runtime.rs`、`docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md`、`docs/40_validation/A-VAL-X-V-04_Agent1.0验证卡.md`、本文件 |
| Rust | `cargo test`：543 lib + 5 core_chain + 4 sm_broadcast 全通过 |
| 前端 | 本工作包未改前端；沿用 689 用例基线 |
| 治理 | `governance:audit` findings/warnings 为空；ADUS 重新生成 |
| 验证边界 | 未运行真实长文；指标事件未接可视化面板；提示词瘦身未执行 |
| unresolved_items | R1/R2/R3 人工基线；P1-4 新消息入口；提示词瘦身后续 |
