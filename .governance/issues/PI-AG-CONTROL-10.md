# Issue Trace · PI-AG-CONTROL-10 控制冲突可执行与当前版本唯一

| 项 | 内容 |
|---|---|
| 状态 | READY（自动化通过；真实复现待人工） |
| 严重级别 | P1（控制冲突只报字段名，模型无法一次改对；多版本号并存） |
| 发现来源 | 2026-10-31 长文日志 + 人类确认的 v3 计划 M4 |
| 影响模块 | AG / SYS |
| 人类确认 | YES（2026-10-31「继续下一步」，承接已确认的 v3 计划 M4） |

## 1. 用户可见症状

- `AG_MODEL_CONTROL_CONFLICT` 只给“差异字段：nextAction”，模型不知道期望值/实际值，反复提交同一冲突。
- 同一回合内出现多个 sessionVersion/contentHash，模型无法判断哪个是当前文档版本。

## 2. 根因

| 编号 | 根因 | 位置 |
|---|---|---|
| R1 | 冲突错误只拼字段名，不带期望/实际；诊断也没有结构化明细 | src-tauri/src/ag/model_output.rs |
| R2 | 为避免泄露用户文本，字段值被整体禁止，连枚举/数量也不给 | 同上 |
| R3 | 提示词没有明确 `currentDocument` 是唯一权威版本 | src-tauri/src/ag/prompts.rs |

## 3. 相关规则

- BR-AG-DATA-004（IntentResult/控制封套）
- BR-AG-STATE-002（工具循环与授权）
- BR-SYS-DATA-008（上下文事实）

## 4. 相关链路

AG-TURN → AG-TOOL

## 5. 生产者块

- `changed_control_fields` / `control_field_display` / 冲突构造（src-tauri/src/ag/model_output.rs）
- `WORKSPACE_OPERATION_INSTRUCTIONS`（src-tauri/src/ag/prompts.rs）

## 6. 消费者块

- 模型可读控制错误消息
- `ControlProtocolRejection::diagnostic()` 事件/前端诊断
- 后续工具副本的 `_binderTurn`

## 7. 路径上的空规则块

无。

## 8. boundary 假设

- 仍然整批拒绝实质冲突，不执行任何副本调用，不返回可执行参数。
- 冲突明细只展开枚举字面值、数组数量与版本字符串；goal/route.summary/route.steps/clarificationQuestion 等文本一律“<文本已省略>”，不进入模型消息或事件。
- `currentDocument` 唯一权威只在 facts 中存在该字段时成立；无 ready 快照时不伪造版本。

## 9. 运行时假设

- 冲突字段最多展开 3 个，其余由 `fieldPath` 指向首个冲突。
- 修正建议固定为“后续工具副本与首项逐字一致；仅 intent.goal/route.summary/route.steps 可不同”。
- pending 预览各自携带 base，不因 currentDocument 权威而提升旧 base。

## 10. 疑似不变量破坏

- 风险：为提高可执行性而泄露用户目标/路径/正文。约束：文本类只输出省略标记；测试断言不含 `PRIVATE`。
- 风险：模型把 currentDocument 当作编辑授权。约束：提示词同时声明版本权威不等于授权，编辑仍走 ExplicitEditTargetSet。
- 风险：诊断结构变化破坏前端。约束：新增 `conflict` 为可选数组，旧字段不变。

## 11. 验证计划

- `duplicate_description_compatibility_never_accepts_changed_decision_fields`：消息含期望/实际/修正，诊断含 `conflict[]`，不泄露 `PRIVATE`。
- `new_turn_prompt_exposes_business_tools_without_batch_wrapper`：提示词含 `currentDocument 为权威`。
- 全量：`cargo test`、`tsc`、前端 689、`governance:audit`。

## 12. Task Runtime Report（2026-10-31）

| 项 | 结果 |
|---|---|
| task_type / phase | CODE_CHANGE_MINOR / VALIDATION → READY（仅错误可执行性与版本口径） |
| changed_files | `src-tauri/src/ag/model_output.rs`、`src-tauri/src/ag/prompts.rs`、`docs/01_modules/AG/A-AG-I-P-01_代理接口协议.md`、`docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md`、`docs/40_validation/A-VAL-X-V-04_Agent1.0验证卡.md`、本文件 |
| Rust | `cargo test`：543 lib + 5 core_chain + 4 sm_broadcast 全通过；冲突测试新增期望/实际/修正与 `conflict[]` 断言 |
| 前端 | `npx tsc --noEmit` 通过；沿用 689 用例基线（本工作包未改前端） |
| 治理 | `governance:audit` findings/warnings 为空 |
| 验证边界 | 未运行真实 Provider；文本类字段只验证“不回显”，未验证所有 Provider 的冲突形态 |
| unresolved_items | 真实长文复现；P1-4 新消息入口；`.governance/active_scope.json` 未切换 |
