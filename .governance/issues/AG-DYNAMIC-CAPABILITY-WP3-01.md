# Issue Trace — AG-DYNAMIC-CAPABILITY-WP3-01：Dynamic Capability 逐动作授权收敛

文档状态：A
任务类型：CODE_CHANGE
治理阶段：VALIDATION
创建时间：2026-09-04

## 1. 用户可见症状

WP2 已让模型负责语义与路线，但工具可见性和执行校验仍复用 `CapabilityGrantV1` 兼容形状；同一 provider
响应内多个 ToolCall 共用一个 grant，grant 未绑定具体 ToolCall、当前 Gate、事件位置与剩余预算。旧
`IntentResultV1.allowed_tools` 与静态 intent×tool 授权函数仍存在，容易重新成为第二授权真源。

## 2. 入口点

- `src-tauri/src/ag/tool_registry.rs`：注册表、CapabilityGrant 计算、工具参数与目标前置校验。
- `src-tauri/src/ag/turn_prompt.rs`：当前硬事实到 Prompt 工具投影。
- `src-tauri/src/ag/turn_tool_loop.rs`：每个 ToolCall 前的实际授权计算、校验、执行与失效。
- `src-tauri/src/ag/target_set.rs` / `edit_runtime.rs` / `de/generate.rs`：ExplicitEditTargetSet 与 PendingDiff 双守。

## 3. 相关规则

- BR-AG-STATE-002、BR-AG-DATA-003、BR-AG-DATA-004、BR-AG-VERIFY-001。
- BR-SYS-STATE-007、BR-SYS-STATE-008、BR-WS-STATE-003、BR-WS-PERSIST-003。
- X-SEQ-001、X-FORBID-004、X-FORBID-006、X-FORBID-009。

## 4. 相关链路

- AG-TOOL：注册、当前硬事实投影、逐 ToolCall grant、参数/目标校验、执行和 ToolResult。
- AG-TURN：Gate、ExplicitEditTargetSet、DecisionRequest 恢复与当前事实重装配。
- DE-DIFF：propose_edit 到 PendingDiff 前的第二目标守卫。
- WS-OPERATION：模型仅生成计划，确认后由运行时内部入口请求 WS 重验执行。

## 5. 生产者块

- RuntimeProfile、工具注册表、AgentReadContext、ExplicitEditTargetSet、格式/动态能力快照。
- 当前 provider 的真实 GatePassed、循环预算、CancellationToken、terminal latch。
- 已解决 DecisionRequest 与 TaskRun 目标投影。

## 6. 消费者块

- PromptPackage/Tool Manifest 只消费不可执行的当前硬能力投影。
- ToolValidating 对当前 ToolCall 消费一次性 CapabilityGrantV2。
- `ag_propose_edit` 与 DE 目标守卫消费同一 ExplicitEditTargetSet。
- WorkspaceOperation 内部执行只消费真实确认回执，不消费模型工具授权。

## 7. 路径上的空规则块

现有 owner 已映射 BR-AG-STATE-002/BR-AG-DATA-003；本包不新增规则。CapabilityGrantV2 数据边界与逐动作
校验边界继续映射 AG-TOOL；若修改目标持久化或 WS 内部执行边界，必须复用其既有规则，不得创建平行链路。

## 8. boundary 假设

- Prompt Manifest Grant 是当前硬事实的“可见工具投影”，不绑定 ToolCall，不能直接执行。
- Execution Grant 只在真实 GatePassed 后、ToolRequest 已形成事件事实时计算，绑定唯一 tool_call_id 与
  computed_at_seq；校验后无论成功/失败都失效。
- allowed_tools 是 CapabilityGrant 的运行时输出，不是 IntentResult 字段，也不能由模型或前端提交。
- permitted_effect_paths 只描述当前工具可进入的受控内部边界，永不包含模型可见的 WS execute 命令。

## 9. 运行时假设

- 同一 AgentTurn 只有一个活动执行租约，ToolCall 顺序执行，因此每个调用可使用单调事件 seq 作为计算时点。
- GatePassed 只对当前 provider 调用有效；该响应产生的 ToolCall 可分别取得绑定本响应 Gate 的一次性 grant，
  后续 provider 必须重新 Gate。
- Prompt 投影与执行 grant 使用同一硬事实算法；执行时可因预算、取消、终态或前项产生的新事实进一步收窄，
  不允许扩展为 Prompt 未暴露的模型工具。

## 10. 疑似不变量破坏

- `CapabilityGrantV1` 缺少 grant_id、computed_at_seq、目标/格式/动态能力 hash、Decision 回执、预算和动作失效绑定。
- `turn_tool_loop` 对一个响应内多个 ToolCall 复用同一 grant。
- `resolve_capability_grant` 仍以 `IntentResultV1.allowed_tools` 为上界；`context.rs` 仍写入该字段。
- 系统提示仍声称 CapabilityGrant 由 IntentResult 派生，与 Active 设计相反。
- 当前 grant 包含 `report_project` 兼容工具；其删除属于 WP5/WP9，不在本包扩展。

## 11. 验证计划

- 硬事实组合：profile、目标集、读范围、格式/动态能力、Decision、Gate、预算、取消、终态逐项收窄。
- 生命周期：Prompt grant 不可执行；ToolCall grant 绑定 call/seq；错误身份、过期、重复、事实 hash 变化均拒绝。
- 负向：未知/未注册/未授权/参数非法/workspace 越界/目标外/格式能力不足不进入 ToolExecuting，重复失败不扩权。
- 双守与资源：AG/DE 集合外零 PendingDiff；WS execute 不在注册表/manifest；确认前零副作用，确认后仍走内部重验。
- 运行实际 changed_files 的 `test:plan`/`test:related`、governance generate/audit 与 diff check。

## 12. 文件范围结论（Scope Lock 输入）

允许修改：本 Scope/Issue/ADUS、CapabilityGrant/Prompt/ToolLoop/Intent 兼容字段、ExplicitEditTargetSet 与
AG/DE 双守、Decision/WorkspaceOperation 授权相关 owner，以及新增 Rust 回归导致的动态 inventory 事实断言。

禁止修改：需求、设计、规则、协议、验证卡、计划、ADU/ADP/AGENTS/APC、cloud/dev-gateway，以及
WP4-WP9 的独立功能实现。

## 13. 人类确认记录

2026-09-04：人类已确认按推荐方案完整执行并多次要求继续。本工作包是冻结计划 WP-AG10-03，
`confirmation: YES`。

## 14. Task Runtime Report

```yaml
task_id: AG-DYNAMIC-CAPABILITY-WP3-01
task_type: CODE_CHANGE
final_state: READY
governance_phase: VALIDATION
changed_files:
  - .governance/active_scope.json
  - .governance/issues/AG-DYNAMIC-CAPABILITY-WP3-01.md
  - docs/00_core/A-CORE-C-R-01_ADUS.md
  - src-tauri/src/ag/context.rs
  - src-tauri/src/ag/prompt_runtime.rs
  - src-tauri/src/ag/prompts.rs
  - src-tauri/src/ag/task_runtime.rs
  - src-tauri/src/ag/tool_registry.rs
  - src-tauri/src/ag/turn_prompt.rs
  - src-tauri/src/ag/turn_runtime.rs
  - src-tauri/src/ag/turn_tool_loop.rs
mapping_chain:
  - REQ-AG-002 -> BR-AG-STATE-002 -> AG-TOOL -> CapabilityGrantV2 / per-ToolCall validation
  - REQ-AG-001 -> BR-AG-DATA-003 -> AG-TURN + AG-TOOL + DE-DIFF -> ExplicitEditTargetSet / target double guard
  - REQ-AG-004 -> BR-AG-DATA-004 -> AG-TURN -> PromptPackage capability projection without authority
  - REQ-AG-007 -> BR-SYS-STATE-007 -> SM-AG-01 + AG-TURN -> cancellation / terminal fail-close facts
  - REQ-AG-015 -> BR-SYS-STATE-008 -> SM-AG-04 + AG-TURN -> TaskRun and DecisionRequest facts without pre-authorization
  - REQ-WS-008 + REQ-WS-010 -> WS-OPERATION -> BR-WS-STATE-003 + BR-WS-PERSIST-003 + BR-WS-OBS-002 -> confirmed internal effect and WS revalidation
test_impact:
  direct_modules: [AG, CORE, ED, SYS, WS]
  affected_modules: [ACCT, AG, AST, CBT, CORE, DE, DOC, ED, ENT, KB, PRV, REF, SYS, TPL, WS]
  selected_commands:
    - npm test
    - cargo test --manifest-path src-tauri/Cargo.toml
    - npm run governance:audit
validation:
  - cargo fmt --check: passed
  - cargo check --lib --tests: passed
  - CapabilityGrantV2 targeted tests: 12/12 passed
  - per-ToolCall loop targeted tests: 5/5 passed
  - DecisionRequest / runtime WorkspaceOperation targeted tests: 7/7 passed
  - frontend full suite: 26 files, 596/596 passed
  - Rust lib suite: 398/398 passed
  - Rust core_chain integration: 5/5 passed
  - Rust sm_broadcast integration: 4/4 passed
  - Rust doc tests: 0 failed
  - LibreOffice host E2E included in Rust suite: passed
  - governance:generate: passed, ADUS refreshed
  - governance:audit: passed, governance_findings=[]
semantic_review:
  result: passed
  findings:
    - 工具注册、Prompt 可见投影、具体动作授权已分离；注册存在与 IntentResult/Prompt/TaskPlan 文本均不能授权执行。
    - 每个 ToolCall 在 ToolRequest 事实形成后按 call/seq/Gate/预算/取消/终态重新计算并单次消费 grant。
    - 目标、workspace 路径、格式与动态能力、Decision 回执和参数形状均在 ToolExecuting 前失败关闭。
    - propose_edit 仍由 AG 与 DE 两侧共同校验 ExplicitEditTargetSet，集合外请求不能生成 PendingDiff。
    - WorkspaceOperation execute 未进入模型工具注册表；confirm 绑定仍存活的 AwaitingDecision 原回合，并在裁决落库前取得效果锁，随后仍由 WS 重新核验。
system_validation:
  status: not_run
  note: 本工作包为运行时授权内核；未执行独立真实模型 UI 场景，端到端系统验收按既定 G5 执行。
stop_conditions: []
unresolved_items: []
```

### READY 自检

- 范围：全部改动均在已确认 allowed_files 内；未修改需求、设计、规则、协议或后续工作包文件。
- 验证：同一份 11 文件输入的 `test:plan` 与 `test:related` 均实际通过，生成与审计通过。
- 证据：自动化测试与尚未执行的真实模型 UI 系统验证已明确分开记录。
- 完整：无 TODO、占位实现、跳过测试或未接线的 V2 生产路径。
- 诚实：未把注册表、Prompt 投影或兼容语义结果伪装成执行授权；未使用绕过参数。
- 留痕：无 WP3 范围内未解决项；`report_project` 的既定 WP5/WP9 收敛不在本包冒进实施。
