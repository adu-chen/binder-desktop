# Issue Trace · PI-AG-CONTEXT-22 自动续跑缺少当前文档快照

| 项 | 内容 |
|---|---|
| 状态 | READY（自动化通过；真实复验待执行） |
| 严重级别 | P1（自动续跑首轮无 currentDocument，模型退回“先接受卡”） |
| 发现来源 | 2026-11-01 07:39 `t-1793537940609-2` |
| 影响模块 | AG / ED / 前端 |
| 人类确认 | YES（“按建议修复”） |

## 1. 现场事实

- 第 1 回合 `t-1793537474525-1` partial，生成 `diff_00cdc327…`（约 2280 字符），TaskRun active；
- 第 2 回合同 TaskRun 自动续跑（自动继续已生效）；
- 第 2 回合首个 `context_frozen`：`openDocCount=0`、`resourceAvailability=not_requested`；
- 模型看不到预览内容，8 轮后 `clarification_required` 要求用户先接受卡片；
- 第 2 回合后期上下文才出现 ready v2（`7aec10a8`），但为时已晚。

## 2. 根因

- `collectAgentContextV2` 的 `wanted` 只来自显式 selection/previewDiffId；
- 解耦后自动续跑只带 taskRunId，没有 previewDiffId，也没有 selection；
- active target 在 openPaths 中但不在 wanted → `not_requested` → 没有 snapshot；
- 第 1 回合结束快照仍 v1/dirty=false，预览应用晚于 `partial` 续跑，进一步放大空上下文。

## 3. 修复

- 自动续跑（`autoContinue && continuation.taskRunId`）把 active target 加入 requested；
  不改变普通新回合“按需读取”的准入语义。
- `maybeAutoContinueTask` 启动前：
  - 检查 active target session 已加载且 content 非 null，否则暂停并提示等待编辑器；
  - 等待该 TaskRun 的 `previewContinuationRequired` diff 完成 `draftApplied`（10s/15s 技术等待），
    未应用则暂停，不启动空上下文模型。
- 提示词补充：`pendingPreviews` previewed/acknowledged 或 currentDocument 已含该批块时直接继续；
  单卡只含本批，不代表任务完成。

## 4. 验证

- 前端 689 用例、`tsc`、`governance:audit` 通过；新增自动续跑 context 目标断言。
- Rust 538 + 5 + 4。
- 真实长文复验待执行。

## 5. 未解决

- 调度器仍是前端过渡版；页面关闭不能续跑。
- 无进展判定仍是启发式；后续应由 completion 显式给 remainingSteps。
