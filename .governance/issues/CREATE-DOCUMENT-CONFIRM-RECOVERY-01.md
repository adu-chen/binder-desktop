# Issue Trace — CREATE-DOCUMENT-CONFIRM-RECOVERY-01：新建文档确认后续执行恢复

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-24
上游基线：用户提供的失败截图、专项说明与本会话只读审计结论；现行 `BR-WS-STATE-003`、`BR-WS-DATA-003`、`BR-WS-OBS-002`、`BR-WS-PERSIST-003`、`BR-SYS-STATE-008`、`X-FORBID-006`。

## 1. 用户可见症状

Agent 新建 Markdown 文档后出现 DecisionRequest；用户点击“确认并执行”后文件一度创建，但初始内容未投影，卡片仍停留在可点击状态。再次点击后显示 `[AG_TASK_DB_ERROR] Query returned no rows`，任务无法继续。

## 2. 入口点

- `src/components/ag/DecisionBar.tsx`：确认按钮与异常后的 DecisionRequest 卡终态清理。
- `src/services/agService.ts`：`resolveDecision` 与 `projectCreateDocumentBundle`。
- `src-tauri/src/lib.rs`：`route_resolve_decision`、`route_project_create_document_bundle`。
- `src-tauri/src/ag/task.rs`：`resolve_decision`。
- `src-tauri/src/workspace/operation.rs`：CreateDocumentBundle 失败补偿。

## 3. 相关规则

- `BR-WS-STATE-003`：计划确认、重复提交幂等、终态不得重复产生副作用。
- `BR-WS-DATA-003`：新建文档的冻结计划、逻辑基线与投影身份一致。
- `BR-WS-OBS-002`：失败阶段与补偿结果必须可区分、可诊断。
- `BR-WS-PERSIST-003`：失败即停止并按逆序补偿，持久化状态反映真实阶段。
- `BR-SYS-STATE-008`：TaskRun/DecisionRequest 状态只能由真实事件推进。
- `X-FORBID-006`：资源创建与内容 Diff 保持两条唯一写路径。

## 4. 相关链路

`AG-TURN → WS-OPERATION → ED-EDIT → DE-DIFF`。确认先完成资源操作，再打开新文档建立 LogicalState，最后以空文档基准投影初始内容 Diff。

## 5. 生产者块

- `ag::task::resolve_decision`：持久化 DecisionRequest 裁决与 TaskRun 状态。
- `workspace::operation::execute_operation_plan`：执行冻结资源计划并生成 CreateDocumentBundle。
- `openDocument`：为新建空 Markdown 建立 LogicalState（合法内容为 `""`）。

## 6. 消费者块

- `route_project_create_document_bundle`：消费 LogicalState 的空基准、版本与哈希，生成初始内容 Diff。
- `DecisionBar`：消费裁决结果或“裁决已提交但后处理失败”终态，决定是否保留卡片。
- WorkspaceTimeline/CreateDocumentBundle 状态：消费补偿阶段并向恢复链提供真实终态。

## 7. 路径上的空规则块

未发现本次关键路径上需新增 owner 的 `rules: []` 块；本次为现行规则实现修复，不修改静态规则定义。

## 8. boundary 假设

- 空 Markdown 的 `beforeSnapshot=""` 是合法 DiskState/LogicalState，不等于缺少参数。
- DecisionRequest 一旦 `resolved` 即为终态；相同 outcome 重放返回同一裁决结果，不重复副作用；冲突 outcome 明确拒绝。
- 编辑器已打开但投影尚未建立 rejection guard 时的失败属于“投影前置失败”，不得记为“打开失败”。

## 9. 运行时假设

- 当前活动 workspace、plan、DecisionRequest、TaskRun 身份必须一致。
- `execute_operation_plan` 的终态回执可按同一 request/plan 幂等读取。
- 后端裁决成功后的前端打开、同步、投影异常均携带 `decisionResolved=true`，使 UI 清除已终态卡片。

## 10. 疑似不变量破坏

- `str_param` 将合法空基准误判为 `WS_CONTENT_REQUIRED`，投影在 rejection guard 创建前失败。
- 前端调用“打开失败补偿”处理投影错误，CreateDocumentBundle 被错误记录为 `open_failed_compensated`。
- 后端只查询 `status='proposed'`，重复确认已 resolved 请求时泄漏 SQLite `QueryReturnedNoRows` 为 `AG_TASK_DB_ERROR`，违反幂等要求。
- 前端只有整个后处理成功才清除卡片，导致已 resolved 的卡仍可再次提交。

## 11. 验证计划

- Rust：空 `beforeSnapshot` 参数可进入投影路由；DecisionRequest 同 outcome 重放幂等、冲突 outcome 拒绝；投影前置失败写入正确补偿终态且不重复资源副作用。
- 前端服务：新建空 Markdown 发送 `beforeSnapshot: ""`；投影补偿返回错误而非伪造 diff id；后端裁决成功后的异常携带 `decisionResolved=true`。
- 前端 UI：终态后处理错误会显示失败消息并清除 DecisionRequest 卡，无法二次点击。
- 运行 `test:plan`、同输入 `test:related`、`test:governance`、`governance:audit` 与 `git diff --check`；真实 Tauri/WKWebView 场景登记人工验证。

## Scope Lock

允许修改：

- `.governance/active_scope.json`
- `.governance/issues/CREATE-DOCUMENT-CONFIRM-RECOVERY-01.md`
- `src-tauri/src/lib.rs`
- `src-tauri/src/ag/task.rs`
- `src-tauri/src/workspace/operation.rs`
- `src/services/agService.ts`
- `src/components/ag/DecisionBar.tsx`
- `src/services/agRuntime.test.ts`
- `src/components/ws/WorkspaceUI.test.tsx`

禁止修改：

- `AGENTS.md`、`ADU.md`、`ADP.md`、`governance.config.json`、APC、ADUS 及设计文档。
- DE/DOC 核心、依赖、数据库 schema 与所有不在允许列表中的文件。
- 当前工作区既有未提交改动不得回退；重叠文件只修改本 Issue Trace 所列函数与测试。

confirmation: YES（用户在收到审计结论与建议修复范围后明确回复“修复”）。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-24 | v1.0 | 基于现场证据与只读审计建立专项 Issue Trace，用户明确授权修复，Scope Lock confirmation=YES。 |
| 2026-08-24 | v1.1 | 修复空基准、裁决幂等、投影补偿分类与终态卡清理；定向测试、前端全量、TypeScript、治理测试与审计通过。Rust 全量仅范围外 LibreOffice Worker signal=6 失败，触发 SC-04，等待人类裁决。 |

## 12. 实施结果

- `beforeSnapshot` 改为“参数必须存在但允许空字符串”，新建空 Markdown 不再被误判为缺少内容。
- `resolve_decision` 支持同一 outcome 幂等重放并返回 `already_resolved`；冲突 outcome 返回 `AG_DECISION_ALREADY_RESOLVED`，不再泄漏 SQLite `QueryReturnedNoRows`。
- 重放确认可读取既有 WorkspaceOperation 回执，但不会再次返回 CreateDocumentBundle，避免重复投影初始内容。
- 新增投影前置失败补偿入口，CreateDocumentBundle 写入 `projection_failed_compensated`，不再伪装为 `open_failed_compensated`。
- 后端裁决成功后的前端后处理异常统一携带 `decisionResolved=true`；DecisionBar 显示错误并清除终态卡，用户无法再次点击。
- 后端返回 `projection_failed_compensated` 时前端抛出真实终态错误，不再伪造 `projection-failed-compensated` diff id。

## 13. Task Runtime Report

task_type: CODE_CHANGE
governance_phase: IMPLEMENTATION
final_state: BLOCKED
issue_trace: `.governance/issues/CREATE-DOCUMENT-CONFIRM-RECOVERY-01.md`

changed_files:

- `src-tauri/src/lib.rs`：空基准参数解析、投影前置补偿命令/路由、DecisionRequest 终态查询与幂等响应；路由级回归。
- `src-tauri/src/ag/task.rs`：相同 outcome 重放幂等，冲突 outcome 拒绝；更新任务裁决测试。
- `src-tauri/src/workspace/operation.rs`：打开失败/投影前置失败共用安全补偿骨架并保留不同终态；扩展原子补偿测试。
- `src/services/agService.ts`：裁决后处理终态错误、投影失败分类与诊断日志、补偿结果不伪造 Diff。
- `src/components/ag/DecisionBar.tsx`：已裁决后处理失败时清除卡片。
- `src/services/agRuntime.test.ts`、`src/components/ws/WorkspaceUI.test.tsx`：空基准、两类投影补偿、终态卡不可重提的跨层回归。
- `.governance/active_scope.json`、本 Issue Trace：Scope 与运行证据。

validation:

- 定向前端：`agRuntime.test.ts` + `WorkspaceUI.test.tsx`，88/88 passed。
- 前端全量：26 文件、589/589 passed。
- TypeScript：`npx tsc --noEmit` passed。
- Rust 关键路径：command/route 空基准与裁决重放 1/1、AG task 12/12、WorkspaceOperation 13/13 passed。
- Rust 全量：354/355 passed；唯一失败 `doc::libreoffice_runtime::tests::docx_to_pdf_end_to_end_when_libreoffice_available`，`DOC_PREVIEW_WORKER_CRASH` / LO Worker `signal=6`。该用例定向复跑仍失败。
- `test:plan` passed；`test:related` 因上述 Rust 范围外失败返回失败，未伪报通过。
- `test:governance` passed：19/19；tests_discovered=953、unmapped=0、unowned=0、focused_or_skipped=0。
- `governance:audit` passed：48 项检查通过、governance_findings=[]。
- `git diff --check` passed。

stop_conditions:

- `SC-04 VALIDATION_BLOCKED`：测试选择器要求 Rust 全量；失败点属于本 Scope 明确禁止修改的 `src-tauri/src/doc/**`，无法在当前边界内修复。解除方式：人类明确接受该已知范围外风险，或扩展 Scope 处理 LibreOffice Worker 崩溃。

unresolved_items:

- 真实 Tauri/WKWebView 中执行“新建 Markdown → 确认 → 打开 → 初始内容 Diff”仍需人工验收；自动化不代替真实界面验证。
- 当前工作区包含上一个 `WS-RESOURCE-WRITE-GUARD-01` 的未提交改动，本次均已保留，未回退或混改其范围外逻辑。
