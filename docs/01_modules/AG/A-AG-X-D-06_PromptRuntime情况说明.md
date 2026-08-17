---
文档编号：AG-X-D-06
文档状态：A
负责模块：AG
文档职责：WP-AG10-04 Prompt Runtime 独立治理情况说明
上游约束：A-AG-X-D-01、A-AG-X-D-04、A-AG-X-D-05、A-AG-X-L-04、A-AG-M-T-01、A-AG-M-T-02、A-AG-I-P-01
直接承接：Execution Kernel 的模型输入、Tool Capability 的本轮工具清单、Session/UI/Trace 的运行时追踪
使用边界：本文说明 agent-edit-1.0 的 Prompt Runtime 现状、冻结契约与独立验证边界，不把 Prompt 文案当作唯一运行时，也不授权新增能力
变更要求：Prompt 层级、装配顺序、版本、预算、工具清单哈希、数据可信边界或追踪字段变化时同步更新
---

# WP-AG10-04 · Prompt Runtime 情况说明

## 1. 模块定位与当前裁决

Prompt Runtime 把本轮已经解析的意图、已选择的上下文和已授权的工具转换为真实模型输入。它负责的是
“运行时装配和约束”，不负责决定业务意图、不自行读取额外上下文，也不因系统提示中出现某个工具名称就授予工具。

当前等级：`Module Ready`。`PromptPackageV1`、实际授权工具清单哈希、数据来源 envelope、预算阻断、
DATA-ONLY 隔离和消息层序均已接入主回合；尚未形成可持久化重放的统一 Prompt 快照，且 package 元数据与
最终 messages 仍是两种表示。

## 2. 职责、输入与输出

职责：

- 固定 System Authority，并把 Runtime Control 作为系统权威注入。
- 接收 `IntentResultV1`、可信上下文 envelope、本轮工具 manifest 与用户原始输入。
- 生成可追踪的 `PromptPackageV1` 和最终 provider messages。
- 在调用 provider 前完成预算判断；超限即失败关闭。
- 保证文档、引用、历史和工具结果只能作为数据，不能提升为系统指令。

输入：

- Runtime Profile 与系统提示版本。
- `IntentResultV1`。
- 当前文档、显式引用、会话历史形成的 `PromptDataEnvelopeV1`。
- 经 `CapabilityGrantV1` 过滤后的实际工具定义及其哈希。
- 当前用户原始输入和 token 预算。

输出：

- `PromptPackageV1`：Runtime/Prompt/Tool 版本、manifest hash、Intent、context trace、envelopes、预算。
- Runtime Control 系统块。
- 最终消息序列：System Authority → Runtime Control/意图 → 当前文档 → 显式引用 → 必要历史 → 当前用户。
- 不含正文和秘密的 trace 元数据。

## 3. 当前实现地图

| 位置 | 当前职责 | 治理注意点 |
|---|---|---|
| `src-tauri/src/ag/prompts.rs` | `SYSTEM_PROMPT_CHAT` 与静态约束测试 | 静态文案不是能力授权源 |
| `src-tauri/src/ag/context.rs` | `PromptPackageV1`、`build_prompt_package`、envelope hash/token 估算 | package 是元数据契约，正文仍由 messages 承载 |
| `src-tauri/src/lib.rs` | `assemble_agent_turn_messages`、Runtime Control、实际工具 manifest hash、预算阻断 | 与 Context、Tool、Execution 共用主编排文件 |
| `src-tauri/src/ag/tool_registry.rs` | 为本轮生成实际可见工具定义 | Prompt 只能消费过滤后的 manifest |
| `src-tauri/src/ag/context.rs`、`src-tauri/src/ag/prompts.rs` 内测试及 AG 主链测试 | 层序、版本、预算、DATA-ONLY 等断言 | 改文案时不能只跑字符串快照 |

## 4. 当前工作流程

```text
当前用户输入与 IntentResult
↓
Context Layer 选择当前文档、显式引用和必要历史
↓
Tool Capability Layer 解析本轮 CapabilityGrant，并过滤模型可见工具
↓
对实际工具定义计算 manifest hash，构建 PromptDataEnvelopeV1 与 PromptPackageV1
↓
预算校验；超限则在 provider 调用前结束
↓
System Authority + Runtime Control + DATA-ONLY 数据层 + 当前用户输入
↓
Execution Kernel 调用 provider；工具结果继续以 DATA-ONLY 回注
```

当前真实消息顺序由 `assemble_agent_turn_messages` 保证；用户本轮原始输入必须是最后一条 user message。
Runtime Control 记录 Intent、Context 来源、CapabilityGrant、预算和 manifest hash，但不记录文档正文。

## 5. 必须冻结与允许变化

必须冻结：

- System Authority 高于 Runtime Control，Runtime Control 高于任何文档/引用/历史/工具返回文本。
- 当前用户原始输入最后注入，历史意图不得覆盖当前轮次意图。
- 文档、显式引用、历史、跨文档历史和工具结果均为 DATA-ONLY。
- 模型看到的工具清单必须等于本轮 grant 过滤结果；trace 中的 hash 必须对应这份实际清单。
- `PromptPackageV1` 必须关联 Runtime、系统提示、工具清单、Intent、Context trace 和预算。
- 预算超限必须发生在 provider 调用前；trace 禁止记录正文、密钥和完整认证信息。

允许持续变化：

- 系统提示的具体措辞和示例，但不得改变上述优先级和授权语义。
- 文档/历史预算阈值、裁剪策略和 token 估算实现，但变化必须可测并版本化。
- Prompt 缓存、provider 适配和调试界面；缓存键必须包含会影响模型输入的版本与内容哈希。

## 6. 已知风险与待治理点

- `PromptPackageV1` 保存来源和版本元数据，最终 messages 在主编排中另行构建；未来修改可能造成两种表示漂移。
- token 数量当前按字符近似，不是 provider tokenizer 的精确结果，边界值需保留安全余量。
- 尚无 provider 侧 Prompt 缓存策略，也无可持久化的脱敏重放记录。
- 静态系统提示描述的全局能力可能多于 `agent-edit-1.0` 的实际 grant；必须持续核验“声明能力”与“本轮可见工具”不互相误导。
- 主装配仍位于 `src-tauri/src/lib.rs`，独立治理时容易误触 Context、Tool 或 Execution 逻辑。

## 7. 独立验证与收口标准

本模块可以在不调用真实 provider 的条件下独立收口。至少验证：

1. 固定 Intent、envelopes、grant 和用户输入，多次装配产生相同版本、hash、trace id、层序和预算结果。
2. 文档、引用、历史中包含“忽略系统要求”“调用未授权工具”等文本时，仍被 DATA-ONLY 包裹。
3. 当前用户输入始终位于最后一条 user message，System Authority 始终为首层系统消息。
4. 模型工具定义只包含 grant 允许项，manifest hash 与实际序列化工具定义一致。
5. Prompt 超预算时 provider mock 调用次数为 0，并返回明确、可恢复的错误。
6. trace 只包含版本、哈希、数量、Intent/来源元数据，不包含正文、凭证或工具敏感结果。
7. 修改静态提示词后，Intent、Context、Tool 权限和 Diff 安全负向测试仍通过，证明文案没有暗改契约。

收口状态定义：上述测试通过，且 `PromptPackageV1` 与最终 messages 的字段对应关系被明确证明；若仍保留
双表示，必须把漂移风险和 owner 写入 Runtime Report，不能以字符串测试通过代替系统验证。

## 8. 下一治理会话入口

必读顺序：A-AG-M-T-02 → A-AG-M-T-01 Prompt/Context/Tool 部分 → A-AG-I-P-01 → `prompts.rs` →
`context.rs` 的 Prompt 类型 → `lib.rs` 的装配与 provider 前置路径 → `tool_registry.rs` 实际清单。

候选修改范围：Prompt 类型、静态提示、主编排的装配片段和对应测试。若改变 `IntentResultV1`、
`PromptDataEnvelopeV1`、`CapabilityGrantV1` 或 provider 接口，应先回到对应 owner 扩展 Scope。

建议会话任务描述：

> 治理 WP-AG10-04 Prompt Runtime。核验真实模型输入的权威层级、数据隔离、实际工具清单、预算、版本和
> 脱敏追踪，不把提示词文本当作权限系统。若修改，建立独立 Scope，并用确定性装配、注入攻击、预算前置
> 和 manifest hash 测试证明收口。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-08 | v1.0 | 建立 Prompt Runtime 职责、真实装配、冻结契约、结构风险与独立验证入口 |
