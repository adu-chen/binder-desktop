# Issue Trace · PI-AG-PROMPT-23 Diff 生命周期声明清理（规则映射链审计）

| 项 | 内容 |
|---|---|
| 状态 | READY（自动化通过；真实复验待执行） |
| 严重级别 | P1（模型反复把 diff 生命周期当作任务进度/停止条件） |
| 发现来源 | 2026-11-01 多轮日志 + 人类要求“规则映射链全链审计” |
| 影响模块 | AG / DE / ED |
| 人类确认 | YES（“把没必要的声明或提示词清理，理顺逻辑”） |

## 1. 规则映射链审计

| 模型可见声明 | 规则/契约 | 代码位置 | 模型实际行为 | 处置 |
|---|---|---|---|---|
| “按 executionStatus/completionStatus/effectStatus 分别说明执行、等待和效果” | BR-SYS-DATA-008, BR-AG-OBS-001 | `prompts.rs` 运行边界 | 每次主动复述接受/保存/生命周期 | 删除该汇报要求；只保留“据回执判断真实执行/失败” |
| 大段“预览→接受→保存→旧卡失效” | BR-SYS-STATE-008, BR-AG-DATA-001 | `prompts.rs` 工作区指令 | 把 pending 当成必须停止 | 压缩为“pending 不阻塞；currentDocument 作 base” |
| “propose_edit 仅生成待确认的修改建议…未收到回执…” | BR-DE-DIFF, BR-SYS-STATE-008 | `prompts.rs` 文档修改指令 | 反复声明“待确认/未保存” | 改为“只生成建议；不得宣称已接受/已保存” |
| “后续批次等 PreviewAck” | BR-SYS-STATE-008 | `prompts.rs` 文档修改指令 | 把 ack 当执行门 | 改为直接使用 currentDocument 最新 LogicalState |
| tool description “待用户裁决/成功后仍需编辑器预览、接受、保存” | Tool schema | `tool_registry.rs` propose_edit/write_content | 模型据此停下等待 | 改为“建议卡已生成；pending 不阻塞后续批次” |
| `proposalFacts.state/effectStatus/saved` | BR-AG-DATA-008 | `turn_prompt.rs` 模型 facts 投影 | 模型看到 accepted/saved 后复述生命周期 | 模型投影删除 `proposalFacts` |
| `pendingPreviews.state/taskRunId` | BR-AG-DATA-008 | 同上 | 模型据 state 推断任务阶段 | 模型投影只保留 `createdBlockIds` 等锚点字段 |
| Diff accept/save | DE/ED 评审与持久化 | DE/ED 路径 | 曾被当作执行信号 | 执行控制已解耦；提示词不再要求“等待接受” |

## 2. 结论

- 不是单一提示词错误，而是**提示词、工具描述、模型 facts 投影**三层同时把 diff 生命周期推到模型面前；
- 运行时真正需要 diff 状态的地方是 DE/ED 的评审/保存和 checkpoint 技术核验，不是 AgentTurn 执行控制；
- 模型只需知道“pending 不阻塞、currentDocument 是 base、createdBlockIds 可作锚点”。

## 3. 修复

- 压缩 `prompts.rs` 运行边界、工作区、文档修改三段中的 diff 生命周期表述；
- 删除“按 executionStatus/… 汇报”的要求；
- `propose_edit`/`write_content` 工具描述去掉“待用户裁决/成功后仍需接受保存”；
- `model_visible_agent_context_facts` 删除 `proposalFacts`、`pendingPreviews.state/taskRunId`；
- 保留完整 `AgentContextFacts` 供运行时校验。

## 4. 验证

- Rust 538 + 5 + 4；前端 689；`tsc`、`governance:audit` 通过。
- 提示词测试改为断言“不再包含 PreviewAck/主动预览应用声明”。
- 真实长文复验待执行。

## 5. 未解决

- 模型仍可能根据 `completionStatus=waiting`/`nextAction=await_user` 推断等待；下一步可评估是否在 proposal_created 时改 nextAction，而不改变回执真实性。
- TaskRun 晚建/partial failed 仍是独立任务层问题，不在本提示词清理范围。
