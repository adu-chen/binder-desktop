# Issue Trace — AG-1.0-DIFF-PREVIEW-ACK-CHAIN-01：Diff 预览回执与连续基线接力

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-20
前置：AG-1.0-TASKPLAN-EXECUTOR-01 已 READY。完整 Rust 回归中 LibreOffice DOCX→PDF Worker 的稳定 `signal=6` 已由人类接受为范围外验证风险。

## 1. 用户可见症状

用户发起“先修改内容、再继续修改同一处/再排版”等复合编辑时，当前实现虽会把 `current_editor` Diff 预览写入 LogicalState，却在同一个 AgentTurn 中继续使用启动时一次性采集的 `AgentReadContext`。后续 `propose_edit` 仍可能以旧快照生成 Diff，造成连续卡基准漂移；同时 Agent 没有等待编辑器真正预览成功的回执，无法把“卡片已生成”与“最新 LogicalState 已可安全续编”区分开。

## 2. 已验证入口与根因

- `src-tauri/src/lib.rs`：`ag_start_turn` 在回合入口一次性构造不可变 `AgentReadContext`；工具循环中每次 `ag_propose_edit` 都使用该同一上下文。当前 ToolResult 只声明 `editor_preview_status=not_reported`，但循环仍可继续接收模型下一次工具调用。
- `src/services/agService.ts`：收到 `DiffProposed` 后投影 DiffCard；`current_editor` 仅标记 `draftPreviewState=queued`，没有向 Agent 运行时回传 LogicalState 已更新的确认。
- `src/components/ed/Editor.tsx`：pending effect 会成功调用受控编辑接口并 `markDraftApplied`，但只改 DE/ED 前端投影，没有 AG continuation 回执。
- `src/services/deService.ts`：`open_session` 成功预览已推进 ED session 且使旧卡失效，但同样不向 AG 运行时传递可续编基线。
- `src-tauri/src/ag/task.rs`：TaskRun/TaskPlan 能保持同一任务身份，但没有“等待本地预览回执后才允许下一 AgentTurn”的持久守卫。

## 3. 相关冻结规则与链路

- `BR-CORE-STATE-001`：已有 LogicalState 时 Diff 返回必须先受控修改 LogicalState；接受只改卡片裁决，保存才写 DiskState。
- `BR-CORE-STATE-002`：持续无 LogicalState 的 background Diff 仅在用户接受后才原子写 DiskState；不得作为后续编辑输入。
- `BR-CORE-STATE-003`：background pending 在打开目标后必须迁移为 LogicalState 预览路径，保持 Pending。
- `BR-SYS-STATE-008` / `REQ-DE-009`：连续 Proposal 的基准只能是上一张成功预览后的最新 LogicalState；后预览使旧基准卡 Expired，不形成补丁栈或任务级回滚。
- `BR-AG-STATE-001` / `BR-AG-STATE-002` / `BR-AG-DATA-001`：AgentTurn 终态唯一、工具结果真实、Agent 读取已打开文档优先取 LogicalState。

链路：`propose_edit → DiffProposed → ED/DE 受控预览成功 → PreviewAck(版本+哈希) → 同一 TaskRun 的受控新 AgentTurn → fresh OpenDocSnapshot → 下一工具步骤`。

## 4. 本批目标与边界

1. 任一 DiffProposal 都在发送 ToolResult 与 DiffProposed 后结束当前 **执行片段**，不得在陈旧 `read_ctx` 上继续生成后续 Diff；其中 `current_editor` / `open_session` 通过 PreviewAck 自动进入受控下一片段，background 必须走既有迁移或裁决后由人类显式继续。
2. 仅在 ED/DE 实际预览成功、并获得新的 `sessionVersion/contentHash` 后，前端发送 PreviewAck；失败、过期、拒绝或未打开的 background 卡不得发送 Ack。
3. 后端将 PreviewAck 持久为同一 TaskRun 的一次性 continuation checkpoint，校验 session、turn、diff、目标路径及最新 LogicalState 基线；重复、错会话、错基线或终态卡失败关闭。
4. Ack 后由前端明确发起受控 continuation（不在后台自行调用模型），复用同一 TaskRun、重新采集当前 OpenDocSnapshots；后续 Proposal 只能以该 fresh LogicalState 为基础。
5. 当前片段完成不把仍等待预览的 TaskRun 结算为 Completed；它仍是执行中的、等待本地预览回执的工作状态。该等待不是 DecisionRequest、不是 DiffCard 接受，也不改变任何 DiskState。
6. 预览成功仍维持既有卡片生命周期：后预览推进基准使旧卡失效；用户手工编辑或保存的既有失效/隐式接受规则不回退。持续无 LogicalState 的 background 分流保持“接受才写盘”，不自动续编。

## 5. 生产者与消费者

- 生产者：`Editor.tsx` / `deService.ts` 只在 `markDraftApplied` 对应的真实 ED LogicalState 成功后产生 Ack 输入；其载荷是实际 session 基线，不是 DiffProjection 或模型声明。
- 边界守卫：`ag/task.rs` 持久化并消费一次性 Preview continuation checkpoint；`db/mod.rs` 提供前向迁移；`lib.rs` 负责对 Ack 和 continuation 做身份/基线校验以及终止旧工具循环。
- 消费者：`agService.ts` 对 Ack 结果发起可见、受控的后续片段，并在新片段入口重新采集 `OpenDocSnapshot`；现有 TaskRun 视图显示真实等待/继续状态，不伪造完成。

## 6. 明确不做

- 不修改三态规则、DiffCard 接受/拒绝/保存语义，也不让 DiffCard 替代 TaskRun 的 DecisionRequest。
- 不让后端在无前端 Ack、应用关闭后或重启恢复时后台调用模型；重启仍按既有规则中断活动 TaskRun。
- 不实现按步骤收窄 CapabilityGrant、更多 TaskPlan 生成策略、网络检索编排或真实模型环境 V-S7。
- 不扩张为补丁堆叠、自动批量接受、任务级回滚或 background Diff 续编。

## 7. 文件范围（Scope Lock 输入）

允许修改：

- `src-tauri/src/ag/task.rs`：Preview continuation checkpoint 的持久状态、一次性消费、TaskRun 守卫和状态测试。
- `src-tauri/src/db/mod.rs`：只增加前向、幂等的 checkpoint 存储迁移及迁移测试。
- `src-tauri/src/lib.rs`：工具循环预览分段、Ack/continuation 入口校验、真实终态收束与 Rust 测试。
- `src/services/agService.ts`：接收 preview checkpoint、重新采集 LogicalState 快照并发起受控 continuation；不伪造内容或裁决。
- `src/services/deService.ts`、`src/store/deStore.ts`、`src/components/ed/Editor.tsx`：仅将真实已应用预览统一报告为 Ack，并保持失败/迁移/失效分支不 Ack。
- `src/services/agRuntime.test.ts`、`src/services/deRuntime.test.ts`、`src/components/ed/EditorCore.test.tsx`：新增/调整本批真实跨层契约测试。
- `src/store/agStore.ts`、`src/components/ag/**`：仅在现有界面无法区分“正在等待编辑器预览”与完成时，投影该真实运行状态；不重构会话 UI。
- `docs/00_core/A-CORE-C-R-01_ADUS.md`：仅由 `governance:generate` 自动生成。
- `.governance/issues/AG-1.0-DIFF-PREVIEW-ACK-CHAIN-01.md`、`.governance/active_scope.json`：本批记录与范围锁。

禁止修改：

- `AGENTS.md`、`ADU.md`、`ADP.md`、`governance.config.json`、APC、冻结需求/设计文档和 `docs/40_validation/**`。
- `src-tauri/src/ag/tool_registry.rs`、`src-tauri/src/ag/context.rs`、`src-tauri/src/ag/tool_exec.rs`：本批不更改工具授权/注册或工具实现；续片段以 fresh turn snapshot 解决陈旧上下文。
- `src-tauri/src/de/**`、`src-tauri/src/workspace/**`、`src-tauri/src/ipc/mod.rs`：不改 DE/WS 后端 Diff 协议和公共 dispatch 信封；Ack 为 AG command 边界内的受控运行时状态。
- `package.json`、`scripts/**`，以及不在允许列表的文件。

## 8. 验收与验证计划

- Rust：同一 TaskRun 生成 LogicalState Diff 后必须登记 checkpoint 且终止旧段；没有 Ack、重复 Ack、错误 session/turn/diff/基线、terminal Diff 均不得续片段；有效 Ack 仅能消费一次并允许同一 TaskRun continuation。
- Frontend：`current_editor` 与 `open_session` 两种由本段 AgentTurn 创建的 LogicalState 预览只在成功后 Ack；background-open-promote 仍只完成既有迁移/预览，不自动恢复已结束的旧 AgentTurn。预览失败、后置基线推进、background-only 卡不 Ack。
- 集成：模拟 Diff1 预览成功 → continuation 入口得到 Diff1 后的 `OpenDocSnapshot` 版本/哈希；Diff2 使用该快照；先前卡失效且不可裁决；磁盘内容直到保存保持不变。
- 回归：DecisionRequest 的 confirm continuation 不受影响；单 Diff 简单任务不因不存在继续步骤而陷入无限 continuation；关闭/取消和重启不产生后台模型调用。
- 对最终 changed_files 运行 `npm run test:plan`、`npm run test:related`、定向 Rust/前端测试、`npm run governance:generate`、`npm run governance:audit`、`git diff --check` 与 changed Rust 文件的 `rustfmt --check`。

## 9. 风险与停止条件

- 如需把“等待本地预览”加入冻结的 TaskRun 正式状态枚举、改变 DE/WS 公共命令或修改冻结设计文档，立即停止并请求范围扩展；本批优先作为既有 Executing 内的持久 checkpoint 实现。
- 若无法证明 continuation 的 `OpenDocSnapshot` 来自 Ack 后的 LogicalState（而非原回合快照），不得以“卡片已生成”替代该证据。
- 完整 Rust 回归的 LibreOffice Worker 失败仍按既有人类接受的范围外风险记录，不得声称全量 Rust 已通过。

## 10. 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-20 | v1.0 | 建立 Scope 草案：以真实编辑器预览 Ack 作为连续 Diff 的唯一基线接力门，锁定无后台续跑与三态规则。 |
| 2026-08-20 | v1.1 | 实现 PreviewAck checkpoint、工具段终止与 fresh snapshot continuation；生成 ADUS。定向 Rust 12/12、定向前端 108/108、TypeScript 和 cargo check 通过。test:related 的前端全量仅被测试库存硬编码 104→105 阻断；该脚本在当前 Scope 禁止列表，等待人类决定是否扩展。完整 Rust 341/342，唯一失败为已接受的 LibreOffice Worker `DOC_PREVIEW_WORKER_CRASH signal=6`。 |
| 2026-08-20 | v1.2 | 人类确认扩展 Scope：仅允许同步 `scripts/test_governance.test.mjs` 的 AG 测试基线 104→105，随后重跑完整影响验证；不修改测试选择器、治理策略或产品断言。 |
| 2026-08-20 | v1.3 | Scope 扩展后 `npm test` 26 files / 585 tests 全绿，TypeScript、定向 Rust、cargo check、ADUS 生成、治理审计和 diff 检查通过；完整 Rust 再次只复现已接受的 LibreOffice Worker 环境失败。任务进入 READY。 |

## 11. Task Runtime Report — Diff 预览回执与连续基线接力

task_type: CODE_CHANGE
governance_phase: IMPLEMENTATION
final_state: READY
issue_trace: AG-1.0-DIFF-PREVIEW-ACK-CHAIN-01

changed_files:
  - `src-tauri/src/ag/task.rs` # 新增持久 PreviewAck checkpoint、一次性消费与 TaskRun 等待守卫及测试
  - `src-tauri/src/db/mod.rs` # 新增 v13 checkpoint 控制态迁移
  - `src-tauri/src/lib.rs` # Diff 段终止、Ack command、fresh-snapshot continuation 校验
  - `src/services/agService.ts` # 真实预览 Ack 与前端受控续片段
  - `src/store/deStore.ts`、`src/components/ed/Editor.tsx` # 预览完成事实投影与 Ack 触发
  - `src/store/agStore.ts`、`src/components/ag/MessageList.tsx` # 可见等待预览状态
  - `src/services/agRuntime.test.ts` # fresh LogicalState continuation 回归
  - `scripts/test_governance.test.mjs` # 经人类确认同步 AG 测试基线 104→105
  - `docs/00_core/A-CORE-C-R-01_ADUS.md` # 自动生成索引

scope:
  mode: explicit
  status: confirmed

phase_gate:
  status: passed
  evidence: 未改变 DiffCard 裁决、DiskState、DecisionRequest、DE/WS 协议或 background Diff 分流。

mapping_chain:
  status: complete
  evidence: `BR-CORE-STATE-001/002/003`、`BR-SYS-STATE-008`、`BR-AG-DATA-001` 由 AG-TURN → DE-DIFF → ED-EDIT 的 @GOV 边界、checkpoint 守卫和跨层测试承接。

test_impact:
  changed_files: task.rs、db/mod.rs、lib.rs、AG/DE/ED 前端文件与测试治理基线
  direct_modules: AG, AST, CORE, DE, DOC, ED, PCG, PRV, REF, SYS, WS
  affected_modules: ACCT, AG, AST, CBT, CORE, DE, DOC, ED, ENT, KB, PCG, PRV, REF, SYS, TPL, WS
  selection: frontend-full + rust-full + governance-audit

validation:
  - `npm run test:plan -- --files …` # passed，选择器要求前端全量、完整 Rust 与治理审计
  - `npm test` # passed，26 files / 585 tests
  - `npx tsc --noEmit` # passed
  - `cargo test --manifest-path src-tauri/Cargo.toml ag::task::tests -- --test-threads=1` # passed，12/12
  - `cargo check --manifest-path src-tauri/Cargo.toml` # passed，只有既有 unused warnings
  - `cargo test --manifest-path src-tauri/Cargo.toml` # 341/342；唯一失败为已接受的范围外 LibreOffice Worker `DOC_PREVIEW_WORKER_CRASH signal=6`
  - `npm run governance:generate` / `npm run governance:audit` # passed，93 assets / 936 tests，无 audit findings
  - `git diff --check` # passed
  - `rustfmt --edition 2021 --check …` # 未通过；输出为 `task.rs`、`lib.rs` 及其子模块的既有全文件格式漂移，未在本 Scope 做大范围格式化

stop_conditions:
  - 无

recovery:
  - 已解除：人类确认扩展测试基线文件后同步 104→105；前端全量验证改为 585/585 通过。

unresolved_items:
  - 完整 Rust 的 LibreOffice DOCX→PDF Worker `signal=6` 为既有人类接受的范围外环境风险。
  - Rustfmt 检出既有全文件/子模块格式漂移；为避免范围外机械重写，本批未处理。
