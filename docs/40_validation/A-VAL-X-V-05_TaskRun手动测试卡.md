---
文档编号：VAL-X-V-05
文档状态：A
负责模块：CORE
文档职责：V-S7 真实环境手动测试材料——验证 TaskRun/TaskPlan 是复杂任务的可观察投影、DecisionRequest 恢复原 AgentTurn，且不存在静态调度或后台续跑
上游约束：REQ-AG-007/015、REQ-SYS-010/011、BR-SYS-STATE-007/008、A-AG-M-T-01、A-AG-M-S-01、A-AG-I-P-01、A-VAL-X-V-04 §5.2
直接承接：V-S7 SYSTEM_PASS/PASS 证据登记
使用边界：本文只执行和记录人工观察，不定义 TaskRun 状态或实现；自动化证据仍由 A-VAL-X-V-04 与 Runtime Report 持有
变更要求：TaskRun、DecisionRequest、AgentTurn 或 UI 投影契约变化时同步更新；未执行不得填通过
---

# V-S7 手动测试卡：TaskRun 可观察投影与裁决恢复

## §1 测试目标

本卡只验证五件事：

1. 简单目标不创建 TaskRun/TaskPlan；
2. 复杂或跨执行片段目标才显示模型当前路线和真实进度；
3. 路线可随 ToolResult 修订，候选步骤不自动执行也不授予权限；
4. DecisionRequest 恢复同一 AgentTurn，并由运行时重新 Gate、授权和重验；
5. 等待、失败、中断和重启不伪装工作中、不后台续跑。

DiffCard 只裁决内容，不裁决任务；ProposalBatch 的多卡独立性由 A-VAL-X-V-04 V-S14 单独验证。

## §2 环境与证据

- 启动目标 Tauri/WKWebView 构建，进入 `agent-edit-1.0`，打开测试 workspace 与 `notes/a.md`。
- 使用可查看脱敏 TurnEvent seq、turn_id、task_run_id、provider/Gate/tool 调用计数的观测面。
- 准备一个可按脚本返回 ToolCall/ToolResult/route revision/DecisionRequest 的确定性 provider；另准备至少一个真实 provider。
- 每个用例记录提交、应用版本、模型/provider、workspace/session/turn/task 身份、截图或录屏、事件序列和实际结果。
- “无执行”必须由 provider/tool/WS 调用数为零证明，不能只看 UI 没变化。

## §3 用例

### T1 · 简单只读回合不创建 TaskRun

| 项 | 内容 |
|---|---|
| 前置 | 新 AgentSession，当前打开 `notes/a.md` |
| 操作 | 输入“总结当前文档，不要修改”并等待完成 |
| 预期 | 只有一个 AgentTurn；可出现真实读取与回答事件；`task_run_id`/TaskPlan 为空；无 DecisionRequest、DiffCard 或写工具；完成后工作态消失 |
| 通过 | 事件和持久化均无 TaskRun，且只读负空间为零写入 |

### T2 · 简单编辑仍不创建空任务

| 项 | 内容 |
|---|---|
| 前置 | 新 AgentSession，当前文档有一个可定位段落 |
| 操作 | 请求“把第一段改得更简洁”，等待 DiffCard，接受或拒绝 |
| 预期 | 单 AgentTurn 可直接产出内容 Proposal；不为卡片创建空 TaskRun/TaskPlan；DiffCard 裁决只改变内容提案事实，不触发任务推进 |
| 通过 | 有合法 Diff 时 task_run_id 仍为空，裁决前后无任务调度事件 |

### T3 · 复杂目标才产生可观察 TaskRun

| 项 | 内容 |
|---|---|
| 前置 | 准备需要多次读取和一个确认点的复杂目标 |
| 操作 | 请求“先检查文档结构和相关材料，再给出修改建议；缺少目标时先问我” |
| 预期 | 当模型形成跨执行片段路线时才出现 TaskRun/TaskPlan；显示目标、route revision、候选路线、已完成事实和等待点；不显示隐藏推理、静态执行游标或未来授权 |
| 通过 | TaskRun 创建时点有事件依据；展示字段与事件一致，无预建假步骤 |

### T4 · ToolResult 后路线可修订

| 项 | 内容 |
|---|---|
| 前置 | 确定性 provider 首次请求读取不存在的路径，随后根据错误改为搜索 |
| 操作 | 运行复杂目标，观察两次 provider 调用和路线显示 |
| 预期 | 第一次 ToolResult 真实失败；模型形成新的 Intent/route revision 并选择另一允许动作；旧候选路线被替换但历史事件保留；失败不会扩权或直接完成 |
| 通过 | provider/Gate/tool/route 顺序可追溯，TaskPlan 只是更新投影，不是改线执行者 |

### T5 · 候选步骤不自动执行或授权

| 项 | 内容 |
|---|---|
| 前置 | TaskPlan 显示一个尚未由模型实际请求的候选写动作 |
| 操作 | 暂停 provider 输出，观察工具/Proposal；再让模型显式请求该工具 |
| 预期 | 仅显示候选时工具调用和 Proposal 为零；模型实际请求后才计算当前 CapabilityGrant 并执行或拒绝；TaskStep 内容不能让原本未授权工具通过 |
| 通过 | 候选与执行之间有明确 model/tool 事件及新 grant，无 scheduler 调用 |

### T6 · DecisionRequest 确认后恢复原 AgentTurn

| 项 | 内容 |
|---|---|
| 前置 | 复杂目标触发 scope 或 WorkspaceOperation DecisionRequest |
| 操作 | 记录 turn_id 后确认；观察恢复、Gate、grant 和领域重验 |
| 预期 | 确认只记录该冻结请求的裁决；同一 turn_id 恢复；运行时重新组装、Gate 和计算 grant，WS 操作还需自身重验；不是 TaskRun “执行下一步”，无需创建新聊天回合 |
| 通过 | turn_id 稳定；有新的 Gate/grant/revalidate 证据；无 TaskPlan 调度入口 |

### T7 · DecisionRequest 取消和新消息不绕过

| 项 | 内容 |
|---|---|
| 前置 | DecisionRequest 未裁决 |
| 操作 | A：直接发送无关新消息；B：取消 DecisionRequest |
| 预期 | A 不使原请求恢复执行，也不把等待投影改为 Active；B 停止该待决路线，保留已成功预览的独立内容事实，不回滚其他卡，不调用 WS |
| 通过 | 等待期间 provider/tool/WS 调用为零；取消结果与事件、TaskRun 投影一致 |

### T8 · 等待状态不伪装工作中

| 项 | 内容 |
|---|---|
| 前置 | 分别进入 AwaitingClarification 和 AwaitingDecision |
| 操作 | 保持 30 秒不操作，观察 UI 和调用计数 |
| 预期 | UI 明确显示“等待补充/等待确认”，无旋转式持续执行假象；provider/tool 调用计数不增长；TaskRun 如存在只投影等待点 |
| 通过 | UI 与真实 activity 计数一致 |

### T9 · 取消与失败终态

| 项 | 内容 |
|---|---|
| 前置 | 分别在 provider 流式、工具执行前、等待态和工具失败场景运行 |
| 操作 | 点击停止或注入错误 |
| 预期 | Cancelled/Failed 只出现一个；停止未开始动作；已完成读取/预览事实保留；无自动重试、无 Error→Completed、无终态后新工具/Proposal |
| 通过 | terminal_latch 和迟到事件证据满足，任务投影不改变业务终态 |

### T10 · 重启只恢复事实，不续跑

| 项 | 内容 |
|---|---|
| 前置 | 复杂 TaskRun 正在 provider/工具活动中 |
| 操作 | 强制退出应用后重启同一 workspace/session |
| 预期 | 旧活动回合/任务显示 Interrupted；事件和已完成事实可恢复；provider/tool 不自动启动；用户显式重试创建新 AgentTurn 并重新校验 |
| 通过 | 重启后零自动调用；新 turn_id 与旧事件关联方式符合重试契约 |

### T11 · 投影重建确定性

| 项 | 内容 |
|---|---|
| 前置 | 已完成一组含 route revision、Decision 和工具结果的复杂任务 |
| 操作 | 记录 UI；重启并从事件恢复；若提供维护入口，删除可重建 TaskPlan 投影后再次恢复 |
| 预期 | 相同事件序列得到相同 TaskRun/TaskPlan 用户可见事实；删除投影不删除 Proposal、Decision、ToolResult 或副作用回执，也不触发动作 |
| 通过 | 恢复前后内容一致，业务事实数量和磁盘不变 |

### T12 · 跨 session/workspace 隔离

| 项 | 内容 |
|---|---|
| 前置 | workspace A/B 各有 AgentSession；A 有待决 TaskRun |
| 操作 | 交错切换并在 B 运行简单/复杂回合，再回到 A 裁决 |
| 预期 | TaskRun、TaskPlan、Decision、目标、事件和工具结果按复合身份隔离；B 的消息不能恢复或取消 A；A 的 pending 不污染 B 的工作态 |
| 通过 | UI、事件查询和持久化均无串线 |

### T13 · 澄清补充恢复原 AgentTurn

| 项 | 内容 |
|---|---|
| 前置 | 模型提出明确的澄清问题 |
| 操作 | 记录 turn_id，提交所需补充；另尝试错误 session/workspace/turn 身份和重复补充 |
| 预期 | 合法补充追加用户事实并恢复原 AgentTurn，重新组装/Gate；不创建新 TaskRun 来承担澄清；错误身份、重复或终态后补充失败关闭 |
| 通过 | turn_id 稳定且调用顺序正确，无跨身份恢复 |

## §4 执行记录

```text
执行人：
执行日期：
提交/应用版本：
Tauri/macOS：
provider/model：
workspace/session 样例：

T1: PASS / FAIL / BLOCKED；证据：
T2: PASS / FAIL / BLOCKED；证据：
T3: PASS / FAIL / BLOCKED；证据：
T4: PASS / FAIL / BLOCKED；证据：
T5: PASS / FAIL / BLOCKED；证据：
T6: PASS / FAIL / BLOCKED；证据：
T7: PASS / FAIL / BLOCKED；证据：
T8: PASS / FAIL / BLOCKED；证据：
T9: PASS / FAIL / BLOCKED；证据：
T10: PASS / FAIL / BLOCKED；证据：
T11: PASS / FAIL / BLOCKED；证据：
T12: PASS / FAIL / BLOCKED；证据：
T13: PASS / FAIL / BLOCKED；证据：

自动化 Runtime Report：
缺陷与归属工作包：
最终裁决：DESIGNED / AUTOMATED / SYSTEM_PASS / PASS / BLOCKED
```

全部人工用例通过只能提供 V-S7 的系统证据；只有当前提交的自动化证据也闭合后，才可在 A-VAL-X-V-04
将 V-S7 标为 `PASS`。

## 变更记录

| 日期 | 版本 | 说明 |
|---|---|---|
| 2026-08-15 | v1.0 | 建立 TaskRun 手动测试卡，后续按当时静态编排语义修订。 |
| 2026-08-31 | v1.2 | 澄清补充统一复用原 AgentTurn。 |
| 2026-09-03 | v2.0 | AG-ADAPTIVE-RUNTIME-PLAN-01：重写为 TaskRun/TaskPlan 事件投影验证；删除静态游标、确认后 TaskRun 自动调度和批内前卡失效预期，新增路线修订、候选零执行、同回合重验与投影重建证据。 |
