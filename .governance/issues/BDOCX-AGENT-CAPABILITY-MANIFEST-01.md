# Issue Trace — BDocx 代理编辑能力协议与运行时声明

## 1. 用户可见症状

BDocx 的人工编辑、代理读取、代理修改、Diff 展示和 DOCX 保存能力分别存在，但没有一份跨模块声明能够
回答“某项能力是否可由代理安全使用并在保存重开后保持”。模型目前会收到分散且部分失真的能力描述，
例如提示词使用 `sub/sup`，而编辑器实际 mark 名为 `subscript/superscript`；无效 StyleOp 可能先生成
Diff，直到编辑器应用阶段才失败。

## 2. 入口点

- 代理回合能力注入：`src-tauri/src/ag/prompts.rs::SYSTEM_PROMPT_CHAT`、
  `src-tauri/src/lib.rs::build_current_doc_prompt_context`。
- 工具能力声明：`src-tauri/src/ag/tool_registry.rs::tool_definitions_json`。
- 当前文档能力读取：`src-tauri/src/ag/blocks.rs::AgentEditableSnapshot`、
  `src-tauri/src/ag/blocks.rs::project_blocks`。
- 代理写入校验：`src-tauri/src/lib.rs::resolve_agent_edit_operation`、
  `src-tauri/src/lib.rs::execute_agent_propose_edit`。
- ED 执行边界：`src/services/edStyleOps.ts::planStyleOps/applyStyleOps`、
  `src/services/edTableOps.ts::planTableOperations/applyTableOperations`。
- DOCX 保真来源：`src-tauri/src/doc/style_subset.rs::classify_style`。

## 3. 相关规则

- `BR-SYS-DATA-002`：跨模块共享结构单一来源。
- `X-SHARED-001`：跨模块传递结构必须经 SYS 契约层定义。
- `BR-AG-STATE-002`：模型工具必须来自注册清单并通过权限与参数边界校验。
- `BR-AG-DATA-001`：当前文档使用 AgentReadContext / LogicalState 作为主事实。
- `BR-AG-DATA-002`：提示词按系统、工具能力、上下文、用户输入统一组装。
- `BR-ED-DATA-005`：StyleOp 的投影、锚和受控编辑语义。
- `BR-ED-DATA-007`：TableOperation 的动态 capability、指纹与原子应用语义。
- `BR-DOC-DATA-002`：StyleSubset 五档保真与损失告知。
- `BR-DE-DATA-002`：DiffPayload 必须足以重建审阅呈现。
- `X-FORBID-005`：编辑修改不得绕过 ED 受控接口。
- `X-FORBID-006`：代理内容修改不得绕过 DE Diff 直接写盘。

本课题需要新增一条 SYS DATA 规则，约束 BDocx 能力清单的版本化、单一来源和各运行时投影一致性；
拟登记为 `BR-SYS-DATA-006`。同时登记核心术语 `BDocxCapabilityManifest`。静态定义须先写入技术设计并
运行 `npm run governance:generate`，再进入代码实现。

## 4. 相关链路

`AG-TURN → AG-TOOL → DE-DIFF → ED-EDIT → DOC-CONVERT`

能力声明本身不新增业务链路；它作为上述既有链路的跨模块契约输入。

## 5. 生产者块

- `BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-001` 位于
  `src-tauri/src/doc/style_subset.rs`：生产 StyleSubset 五档归类。
- `BR-ED-DATA-005-QUERY-ED-ED-EDIT-009` 位于
  `src-tauri/src/ag/blocks.rs`：生产块内 marks / blockAttrs 样式投影。
- `BR-ED-DATA-007-RB-ED-ED-EDIT-018` 位于
  `src/services/edTableOps.ts`：生产 TableOperation 前后快照与动态能力结果。
- `BR-AG-DATA-001-QUERY-AG-AG-TURN-010` 位于
  `src-tauri/src/ag/blocks.rs`：生产 AgentEditableSnapshot。
- `BR-AG-DATA-001-DATA-AG-AG-TURN-011` 位于
  `src-tauri/src/lib.rs`：把当前文档事实装配进模型消息。

本课题新增的机器可读 manifest loader/validator 将成为跨模块能力声明的直接生产者，并按
`BR-SYS-DATA-006`、`BR-AG-STATE-002` 标注。

## 6. 消费者块

- `BR-ED-DATA-005-RB-ED-ED-EDIT-008` 位于
  `src/services/edStyleOps.ts`：消费 StyleOp 并规划原子 ProseMirror 事务。
- `BR-ED-DATA-007-EFFECT-ED-ED-EDIT-014` 位于
  `src/services/edTableOps.ts`：消费已验证 TableOperation 并修改 LogicalState。
- `src-tauri/src/ag/tool_registry.rs::tool_definitions_json`：消费静态能力并投影模型工具 Schema。
- `src-tauri/src/ag/prompts.rs`：消费静态能力并生成人类可读的代理能力边界。
- `src-tauri/src/lib.rs::execute_agent_propose_edit`：消费模型 StyleOp/TableOperation，在生成 Diff 前
  执行能力、字段、锚和范围校验。

## 7. 路径上的空规则块

- `src-tauri/src/ag/prompts.rs` 明确属于实现期运营文本，当前不带 @GOV；分类为
  `PLATFORM_GLUE`，但其能力清单不得继续手写，须消费 manifest 的投影。
- `src-tauri/src/ag/tool_registry.rs` 当前是注册/Schema 适配层，没有独立 @GOV；
  现有注册与权限机制由 `BR-AG-STATE-002` 上游规则约束。本课题不把静态 JSON 拼装伪装为新的业务链路入口。
- manifest 的 JSON 数据文件属于 `CONFIG_ONLY`；语义规则由 SYS 协议与 Rust validator 主责块持有。

未发现需要先行 triage 的 `rules: []` @GOV 块。

## 8. boundary 假设

- StyleOp 字符范围继续以块内纯文本平面为坐标，不改变 `BR-ED-DATA-005` 现有锚语义。
- TableOperation 继续使用 `pre-update` 的 table fingerprint 和当前 session 基线；manifest 只声明静态
  能力，不取代 `list_tables` 的动态 capabilities / moveDirections。
- DOCX 保真等级继续以 `A-DOC-M-T-01 §4` 和 `style_subset.rs::classify_style` 为来源；manifest 只能
  引用现状，不得自行把 BestEffort/RequiredPending 提升为 Verified。
- 当前文档能力注入必须来自同一 AgentEditableSnapshot，不从文件扩展名或提示词常量猜测格式。

## 9. 运行时假设

- `caller_context=AG-TURN/AG-TOOL`：manifest 在模型调用前可读，不依赖网络或用户额外工具调用。
- `runtime_guard=current AgentEditableSnapshot`：BDocx 特定能力只在 `editable_kind=bdocx` 时注入。
- `runtime_guard=validate-before-diff`：StyleOp/TableOperation 必须在 PendingDiff 持久化前完成静态能力、
  字段、锚和范围校验；失败回注模型，不生成不可执行 Diff。
- `runtime_guard=manifest version/hash`：提示词、工具 Schema、校验器和测试读取同一版本，禁止各自维护
  第二套 mark/op 枚举。

## 10. 疑似不变量破坏

- 若 manifest 与 DOC StyleSubset 分离维护，代理会把 BestEffort/Unsupported 样式误报为可保真能力，
  违反 `BR-DOC-DATA-002`。
- 若工具 Schema 仍允许任意 mark/attrs，模型可生成编辑器不存在或参数非法的 StyleOp，违反
  `BR-AG-STATE-002` 和 `BR-ED-DATA-005`。
- 若把人工能力全部声明成代理能力，会错误开放删除整表、列宽、图片、标题结构等未接线操作。
- 若把静态 manifest 当作动态表格权限，会绕过 table fingerprint、富内容单元格和 moveDirections 校验，
  违反 `BR-ED-DATA-007`。
- 若继续保留提示词、Schema、validator 三份自由文本清单，能力漂移仍会复发。

## 11. 验证计划

- 文档与治理：
  - 新增 SYS 能力协议、TERM、RULE、REQ_MAP 和开发计划条目。
  - 运行 `npm run governance:generate`，确认 TERM/RULE 进入 ADUS。
  - 运行 `npm run governance:audit`，消费 semantic review candidates。
- manifest：
  - 唯一 `capability_id`、版本/hash、状态枚举、人工/代理/Diff/roundtrip 维度完整。
  - 与 DOC `classify_style` 逐项一致，未知保守为 unsupported。
- 提示词与工具：
  - 当前 BDocx 上下文明确注入格式、manifest 版本和准确能力摘要。
  - mark 名使用 `subscript/superscript`，不再出现 `sub/sup`。
  - StyleOp 和 TableOperation JSON Schema 具有条件字段和枚举，不再只给宽泛 object/description。
- 执行前校验：
  - 接受合法 bold/textStyle/link/textAlign；拒绝未知 mark、非法 attrs、越界 range、错误 blockId。
  - 无效操作不生成或持久化 PendingDiff。
  - 表格仍必须遵守 `list_tables` 动态能力，不因静态声明放宽。
- 回归：
  - Rust 定向测试：manifest、prompt、tool registry、blocks、propose_edit。
  - 前端定向测试：StyleOp 解析/应用与 AG Diff 投影不回退。
  - `cargo check --lib`、`npx tsc --noEmit`、相关 Vitest、`git diff --check`。
- 链路日志：
  - 核对 AG-TOOL 的 `ToolRequest → ToolValidating → ToolExecuting/拒绝回注 → ResultInjecting`；
    非法 StyleOp 在 ToolValidating 终止，不进入 DE-DIFF。
- triage：
  - manifest JSON 记为 `CONFIG_ONLY`；纯提示词投影记为 `PLATFORM_GLUE`；其他新增导出执行块按真实
    DATA/GUARD/RB 职责标注，不以空规则规避。

## 12. 实施范围与非目标

本课题实现：

1. 建立独立的 SYS BDocx 代理编辑能力协议。
2. 建立版本化机器可读 BDocxCapabilityManifest。
3. 由同一 manifest 投影代理提示词、工具 Schema 和执行前校验。
4. 当前文档提示词明确注入格式、能力版本、受支持操作、人工专属能力和 DOCX 往返风险。
5. 补齐精确 StyleOp/TableOperation 工具参数声明及定向测试。

本课题不实现：

- 新增任何人工编辑功能或代理编辑操作。
- 把 fontSize/fontFamily/对齐/高亮等保真等级提升为 Verified。
- 新增删除整表、调整列宽、图片编辑、标题节点类型切换等代理能力。
- 重做样式 Diff 的字符级/视觉级呈现；本轮只如实声明其当前展示等级。
- 修改 DOCX 转换器、LibreOffice、保存状态机、表格执行器或 UI 布局。

## 13. Phase Gate Check

- task_type：`CODE_CHANGE`
- governance_phase：`IMPLEMENTATION`
- REQUIREMENT：复用 `REQ-SYS-002`、`REQ-AG-003`、`REQ-ED-007/009`、`REQ-DE-002/004`、
  `REQ-DOC-003` 的既有能力、工具、Diff 与 StyleSubset 需求，不新增产品功能。
- DESIGN：需先在本 Scope 内落地 SYS 能力协议与 `BR-SYS-DATA-006`，并同步 AG/ED 接口引用。
- PLAN：需在 SYS 开发计划中新增明确 Plan Item 后进入代码。
- 当前状态：`READY`；人类已将 `.governance/active_scope.json.confirmation` 设为 `YES`，实现与验证
  已在确认后的 Scope Lock 内完成。

## 14. Scope Lock 提案

允许修改的文件：

- `.governance/issues/BDOCX-AGENT-CAPABILITY-MANIFEST-01.md`
- `.governance/active_scope.json`
- `docs/00_core/A-CORE-C-R-01_ADUS.md`
- `docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md`
- `docs/01_modules/SYS/A-SYS-I-P-04_BDocx代理编辑能力协议.md`
- `docs/01_modules/SYS/A-SYS-X-L-01_系统底座开发计划.md`
- `docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md`
- `docs/01_modules/AG/A-AG-M-T-02_提示词架构设计.md`
- `docs/01_modules/AG/A-AG-I-P-01_代理接口协议.md`
- `docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md`
- `docs/01_modules/ED/A-ED-I-P-01_编辑器接口协议.md`
- `src-tauri/resources/bdocx-agent-capabilities.json`
- `src-tauri/src/ag/bdocx_capabilities.rs`
- `src-tauri/src/ag/mod.rs`
- `src-tauri/src/ag/blocks.rs`
- `src-tauri/src/ag/prompts.rs`
- `src-tauri/src/ag/tool_registry.rs`
- `src-tauri/src/lib.rs`
- `src/services/edStyleOps.ts`
- `src/services/edStyleOps.test.ts`
- `src/services/agService.ts`
- `src/services/agService.test.ts`

禁止修改的文件：

- `ADU.md`
- `ADP.md`
- `AGENTS.md`
- `governance.config.json`
- `docs/00_core/A-CORE-C-P-01_APC.md`
- `src-tauri/src/doc/edit_chain.rs`
- `src-tauri/src/doc/style_subset.rs`
- `src-tauri/src/doc/libreoffice_runtime.rs`
- `src-tauri/src/doc/lo_worker.rs`
- `src/services/edTableOps.ts`
- `src/components/ed/EditorToolbar.tsx`
- `src/components/ed/TableMoveControls.tsx`
- `src/components/de/DiffCard.tsx`
- `src/store/deStore.ts`
- `src/services/deService.ts`
- `src/services/edService.ts`

不在允许列表中的文件默认禁止修改。

## 15. 实施结果（2026-07-28）

1. 新增独立 SYS 协议 A-SYS-I-P-04，登记 TERM-SYS-002、BR-SYS-DATA-006，并在 SYS/AG/ED 的
   数据结构、技术设计、接口协议和开发计划中同步消费边界。
2. 新增机器清单 `binder.bdocx.agent-edit` v1.1.0。清单覆盖文档替换、StyleOp、10 种
   TableOperation、人工专属能力、Diff 呈现等级和 DOCX StyleSubset 全量往返等级；以
   `rangeUnit=utf16_code_unit` 明确 CharAnchor 单位。
3. Rust loader 在构建内从同一 JSON 字节加载、校验 manifestId/语义版本/适用格式/唯一能力 ID、
   参数种类、Diff 汇总及 DOC `classify_style` 等级，并计算 SHA-256。
4. `SYSTEM_PROMPT_CHAT` 不再手写 BDocx mark/op 清单；活动 BDocx 当前文档上下文动态注入
   editable kind、content format、manifest version/hash、真实能力、人工专属能力、Diff 等级与
   DOCX 往返风险。
5. `propose_edit` 工具参数由 manifest 投影条件 JSON Schema：StyleOp 使用真实
   `subscript/superscript` 与受控 attrs，TableOperation 按 op 声明必填字段、类型、enum 与
   `additionalProperties=false`。
6. StyleOp 在 PendingDiff 持久化前校验批上限、精确字段、mark/attrs、blockId、块类型和范围；
   TableOperation 在既有动态 capabilities/fingerprint/moveDirections 校验前增加 manifest 静态形状
   校验。不同 operation 的载荷字段不得混用。
7. 后端 HTML mark 投影、模型提示计数和 StyleOp 范围校验统一为 ED 实际使用的 UTF-16 code unit，
   补齐 emoji 回归；ED 对历史 Diff/IPC 载荷继续做严格批内原子防御，无效样式载荷投影为不可裁决
   error Diff。
8. 未新增任何编辑能力，未提升 StyleSubset 等级，未修改 DOCX 转换器、表格执行器、Diff UI、
   保存状态机或编辑器布局。

## 16. 语义审计

对 BR-SYS-DATA-006 的 4 个新增 semantic review candidates 完成规则—boundary 人工语义比对：

- manifest loader：输入 bundled manifest JSON，输出经版本与一致性校验的唯一运行时来源，符合规则。
- 工具 Schema 投影：输入同一 manifest，输出条件 StyleOp/TableOperation Schema，符合规则。
- 提示词投影：输入同一 manifest，输出 version/hash/代理与人工边界/Diff/往返声明，符合规则。
- Diff 前 guard：输入不可信 StyleOp 与当前 AgentEditableSnapshot，静态、锚或范围失败即拒绝，
  不进入 DE-DIFF，符合规则。

结论：未发现规则文本、核心实体、链路职责或 boundary 描述不一致。

## 17. Task Runtime Report

- task_type：`CODE_CHANGE`
- governance_phase：`VALIDATION`
- final_state：`READY`
- issue_trace：`BDOCX-AGENT-CAPABILITY-MANIFEST-01`
- scope：本轮写入仅发生在 `.governance/active_scope.json.allowed_files` 中的协议、manifest、AG/ED
  接线和测试；未写入 forbidden_files。当前脏工作区的部分 forbidden_files 保留有人类/前序课题既有
  差异，本轮未覆盖、还原或据此扩大范围。
- phase_gate：REQUIREMENT 复用既有需求；DESIGN 新增 A-SYS-I-P-04 与 BR-SYS-DATA-006；
  PLAN 新增并完成 PI-SYS-06；IMPLEMENTATION/VALIDATION 已完成。
- mapping_chain：REQ-SYS-002 → BR-SYS-DATA-006 → PI-SYS-06 →
  manifest loader/prompt/schema/pre-diff guard → Rust/TypeScript/Vitest/governance 验证。
- validation：
  - `cargo fmt -- --check`：通过。
  - `cargo check --lib`：通过；仅有项目既有 dead_code warning。
  - `cargo test bdocx --lib`：11/11 通过。
  - `cargo test bdocx_capabilities --lib`：6/6 通过。
  - UTF-16 mark 投影与 operation 混载拒绝定向测试：各 1/1 通过。
  - `npx tsc --noEmit`：通过。
  - `npx vitest run src/services/edStyleOps.test.ts src/services/agService.test.ts`：34/34 通过。
  - `npm run governance:generate`：通过；生成 187 个 @GOV 块、78 条规则、48 个术语。
  - `npm run governance:audit`：`status=passed`，无 governance finding。
  - 本 Scope 文件 `git diff --check`：通过。
- unresolved_items：
  - `cargo test ag:: --lib` 为 66/67；唯一失败是既有
    `ag::blocks::tests::html_replace_escapes_special_chars`。当前实现已在本课题前按 D10 改为保留经
    sanitize 的内联 HTML，而旧断言仍要求转义；本课题未修改 replace_block 行为或该旧断言，故作为
    范围外测试债务保留，不影响本课题新增 manifest、Schema、校验和 UTF-16 回归的通过结果。
  - 未做真实模型供应商端到端调用；本课题的运行时投影、工具 Schema、拒绝路径与前端消费由自动测试
    覆盖，模型行为质量仍需后续场景评测，不将其伪报为已验证。
