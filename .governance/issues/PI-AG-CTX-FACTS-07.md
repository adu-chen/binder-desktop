# Issue Trace · PI-AG-CTX-FACTS-07 预览事实与预览三态入 AgentContextFacts

| 项 | 内容 |
|---|---|
| 状态 | READY（自动化通过；真实长文复现待人工） |
| 严重级别 | P1（模型在非 ack 回合看不到已预览的新块，反复要求先接受） |
| 发现来源 | 2026-10-31 长文任务日志（binder-model-prompt-audit-2026-10-31.log）+ 人类确认的 v3 方案 |
| 影响模块 | AG / ED / DE / SYS |
| 人类确认 | YES（2026-10-31「执行修复」，基于已确认的 v3 计划 M1） |
| Scope | src-tauri/src/types/mod.rs、src-tauri/src/ag/turn_runtime.rs、src/services/agService.ts、src/services/agRuntime.test.ts、scripts/test_governance.test.mjs、SYS I-P-01 §3.1.1、A-AG-M-T-02、验证卡 §15 |
| Runtime Report | 本文 §12 |

## 1. 用户可见症状

- `propose_edit` 已生成 pending 卡、编辑器已写入预览，但模型在下一条用户消息中仍称“pending 卡未生效、新块没有真实 id”，要求用户先接受。
- 模型只能从 `proposalFacts.state=pending` 推断，无法区分“已写入 LogicalState 待裁决”与“尚未生效”。

## 2. 根因

| 编号 | 根因 | 位置 |
|---|---|---|
| R1 | `AgentContextFactsV2` 只有 `proposalFacts`（state/effectStatus/saved），没有预览态与 `createdBlockIds` | src-tauri/src/types/mod.rs |
| R2 | PreviewAck 续跑轮之外的回合不注入 `createdBlockIds`；普通新消息只看到 pending | src-tauri/src/ag/turn_runtime.rs / src/services/agService.ts |
| R3 | 缺少“当前文档唯一版本”的派生事实；模型可引用多个 sessionVersion/contentHash | src-tauri/src/ag/turn_runtime.rs |

## 3. 相关规则

- BR-AG-DATA-008（上下文事实）
- BR-SYS-DATA-008（上下文事实共享通道）
- BR-ED-DATA-008（编辑器事实投影）
- BR-AG-DATA-011（工具回执与模型回注）

## 4. 相关链路

AG-TURN → AG-TOOL → DE-DIFF → ED-EDIT → AG-TURN

## 5. 生产者块

- `parse_agent_context_facts`（src-tauri/src/ag/turn_runtime.rs）
- `collectAgentContextV2`（src/services/agService.ts）
- `AgentContextFactsV2`（src-tauri/src/types/mod.rs）

## 6. 消费者块

- `prepare_turn_prompt` 的 agent_context_facts data-only 层
- `refresh_tool_context` 的上下文重采集
- Prompt Runtime / 事件投影

## 7. 路径上的空规则块

无（相关块 rules 均非空）。

## 8. boundary 假设

- `AgentContextFactsV2` 的 `does_not` 不变：新增事实不授予读取或编辑权限、不扩 ExplicitEditTargetSet。
- `parse_agent_context_facts` 的 `does_not` 含 `grantAccess`；本次只增加形状校验与派生，不放宽身份/版本/工作区校验。

## 9. 运行时假设

- 预览是否写入 LogicalState 由宿主确认（`draftApplied` / `draftPreviewState=applied`）；后端不读取编辑器正文，不能自行证明块存在。
- 后端只校验形状、路径归属、上限与 `previewed` 的 ready 快照一致性。
- `currentDocument` 由后端从 `activeTarget` + ready 快照派生，客户端声明必须与派生值一致，否则失败关闭。

## 10. 疑似不变量破坏

- 风险：把“已预览”误报为“已应用/已接受”。约束：`previewed` 仅来自宿主 `draftApplied` 或 `draftPreviewState=applied`；`accepted`/`saved` 仍按 DE 状态。
- 风险：`currentDocument` 成为第二版本源。约束：只由后端派生，客户端不得声明为权威；无 ready 快照时为 null。
- 风险：事实体积膨胀。约束：`pendingPreviews` ≤20 条，`createdBlockIds` 每条 ≤32，不携正文。

## 11. 验证计划

- Rust：v2 解析/拒绝用例（非法 state、超限、越界路径、currentDocument 不一致、previewed 无 ready 快照）。
- 前端：`collectAgentContextV2` 投影 previewed/accepted/saved；queued 不输出；旧 JSON 兼容。
- 全量：`cargo test`、`npm test`、`npx tsc --noEmit`、`npm run governance:audit`。
- 真实长文复现：待人工执行（自动化不替代）。

## 12. Task Runtime Report（2026-10-31）

| 项 | 结果 |
|---|---|
| task_type / phase | CODE_CHANGE / VALIDATION → READY（仅本工作包） |
| changed_files | `src-tauri/src/types/mod.rs`、`src-tauri/src/ag/turn_runtime.rs`、`src-tauri/src/ag/prompts.rs`、`src/services/agService.ts`、`src/services/agRuntime.test.ts`、`scripts/test_governance.test.mjs`、`docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md`、`docs/01_modules/AG/A-AG-M-T-02_提示词架构设计.md`、`docs/40_validation/A-VAL-X-V-04_Agent1.0验证卡.md`、本文件 |
| Rust | `cargo test --manifest-path src-tauri/Cargo.toml`：540 lib + 5 core_chain + 4 sm_broadcast 全通过；新增 `context_v2_preview_facts_are_derived_and_bounded` |
| 前端 | `npx vitest run --exclude ".p1-spike/**"`：27 文件 / 689 用例全通过；新增 agRuntime 预览事实用例；`npx tsc --noEmit` 通过 |
| 治理 | `npx vitest run scripts/test_governance.test.mjs` 29/29 通过（AG 前端用例基线 163→164，随新增用例）；`npm run governance:audit` findings/warnings 为空 |
| 测试选择 | `npm run test:plan -- --files ...` 通过；`test:related` 的命令链中 `npm test` 仅因本机未跟踪 `.p1-spike/vendor` 缺依赖/超时失败，仓库范围内 2681 用例通过 |
| 验证边界 | 未运行真实 Provider/真实编辑器长文复现；后端不读取编辑器正文，不证明块真实存在；`previewed` 仍是宿主上报事实 |
| unresolved_items | CF07-B 真实长文复现；M2 额度拆分、M3 锚点、M4 控制冲突未开始；`capabilityGrant` 仍向模型暴露 tool 额度（见 v3 计划 P0-4b）；`.governance/active_scope.json` 未切换到本工作包，当前仍指向既有 BLOCKED 任务，本工作包以本 Issue Trace 留痕 |
