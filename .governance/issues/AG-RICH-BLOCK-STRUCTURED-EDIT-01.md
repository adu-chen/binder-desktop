# Issue Trace — 富文档代理结构化块编辑协议与精确 Diff 闭环

## 1. 用户可见症状

- 原本由多个独立列表项组成的目录，经代理编辑后被压成一个列表项；界面只剩一个项目符号，后续条目
  变成同一段落中的连续行内内容。
- 修改建议表面显示为块级修改，但实际执行物是容器 inner HTML / 字符串，不能保持 ProseMirror
  LogicalState 中的列表、段落与行内标记结构。

## 2. 入口点

- `collectOpenDocSnapshots()` 捕获当前编辑器 LogicalState 序列化内容。
- `ag.list_blocks` / `read_doc` 向模型投影内容块。
- `propose_edit(operation=replace_block)` 生成 PendingDiff。
- `Editor` pending-diff effect 调用 `applyEditOps()` 将候选应用到当前 LogicalState。
- `acceptDiff()` / `rejectDiff()` 完成裁决及恢复。

## 3. 相关规则

- `BR-AG-DATA-001`：已打开文档必须以 OpenDocSnapshot LogicalState 为上下文主事实。
- `BR-AG-DATA-003`：ChangeProposal 必须基于已打开目标的 LogicalState。
- `BR-ED-STATE-001`：编辑必须经 ED 受控接口进入 LogicalState 并产生脏态。
- `BR-ED-DATA-001`：Block 结构化建模、稳定 blockId 和锚失效可感知。
- `BR-ED-DATA-004`：块内字符锚与失效边界。
- `BR-DE-DATA-001`：PendingDiff 目标锚准确，失效时不得错位生成。
- `BR-DE-DATA-002`：DiffPayload 版本化、持久化与审阅重建。
- `BR-DE-STATE-001`：pending/accepted/rejected/expired 终态语义。
- `X-FORBID-005`、`X-FORBID-008`、`X-FORBID-009`：富文档修改必须经 ED 受控接口，候选不得绕过
  裁决或由 DisplayState 反写 LogicalState。
- `X-SHARED-001`：跨 AG/ED/DE 的快照、操作和 Diff 投影结构由 SYS 契约登记。

## 4. 相关链路

- `AG-TURN → AG-TOOL → DE-DIFF → ED-EDIT`
- `DE-DIFF → ED-EDIT → WS-SAVE`
- `DE-DIFF rejected → ED-EDIT exact restore`

## 5. 生产者块

- `BR-AG-DATA-001-EFFECT-AG-AG-TURN-005`：
  `src/services/agService.ts::collectOpenDocSnapshots` 生产当前打开文档快照。
- `BR-ED-DATA-001-DATA-ED-ED-EDIT-001`：
  `src/components/ed/extensions/BlockIdExtension.ts` 生产稳定 Block 身份。
- `BR-AG-DATA-001-QUERY-AG-AG-TOOL-011`：
  `src-tauri/src/ag/blocks.rs::AgentReadContext::snapshot` 生产代理可读快照。
- `src-tauri/src/lib.rs::ag_propose_edit` 生产 ChangeProposal、DiffPayload 与 DiffProjection。

## 6. 消费者块

- `src-tauri/src/ag/tool_exec.rs::exec_read_only` 消费 AgentEditableSnapshot 并回注模型。
- `BR-ED-STATE-001-EFFECT-ED-ED-EDIT-012`：
  `src/services/edLocate.ts::applyEditOps` 消费编辑操作并修改 ProseMirror LogicalState。
- `BR-DE-STATE-001-RB-DE-DE-DIFF-002`：
  `src/store/deStore.ts` 消费 DiffProjection 并维护当前运行期 LogicalCandidate 投影。
- `src/components/ed/Editor.tsx` pending-diff effect 消费 DiffView 并触发草稿事务。
- `src/services/deService.ts::rejectDiff` 消费 Diff 前态执行恢复。

## 7. 路径上的空规则块

- `ag_propose_edit` 是 `AG-TOOL → DE-DIFF` 的内部协调入口，当前无独立 `@GOV`；此次应复用
  `BR-AG-DATA-003`、`BR-DE-DATA-001/002`，不新增游离业务规则。
- 新增 ProseMirror 结构投影和 BlockOperation 纯规划函数分别归 `BR-ED-DATA-001` 与
  `BR-DE-DATA-002`；测试文件按 ADU 豁免，不写 `@GOV`。
- 未发现需要保留的 `rules: []` 业务决策块。

## 8. boundary 假设

- 活动富文档的 ProseMirror doc 是唯一结构真值；HTML 仅是保存/跨会话序列化，不是当前块拓扑的
  再解析来源。
- container Block 只提供层级上下文，不拥有文本写能力；content Block 才可作为 BlockOperation 目标。
- BlockOperation 必须携带同次 `sessionVersion/contentHash`、blockId、fingerprint、
  expectedNodeType 与受控 Node JSON。
- 同一批操作必须在一个 ProseMirror transaction 中全部通过或零应用。
- 表格和图片继续使用现有 TableOperation/ImageOperation，不进入通用 BlockOperation。

## 9. 运行时假设

- 结构化富文档修改只允许活动编辑器快照；`active_editor_session` 或非激活 open session 没有可验证
  ProseMirror 节点快照时必须拒绝，不得回落到字符串 HTML 修改。
- 应用前当前 sessionVersion/contentHash 与每个目标 fingerprint 均未变化。
- 受控 Node JSON 只允许目标 Block 原节点类型、合法 attrs、合法 inline content 和 marks；不允许借
  BlockOperation 改写列表拓扑、表格或图片集合。
- reject 前若受影响节点已被人工或其他 Diff 改写，必须报告冲突，不得覆盖较新的 LogicalState。

## 10. 疑似不变量破坏

- 后端 `parse_html_blocks` 先捕获 `ul/ol/div/table` 外层并跳过子节点，违反容器透明约定。
- `strip_tags` 和 `sanitizeInlineHtml` 把子结构压成连续文本，破坏独立 Block 不变量。
- `replace_block_by_id` 与 `applyEditOps(insertContentAt string)` 接受容器目标和字符串执行物，无法验证
  expectedNodeType 或节点指纹。
- 当前拒绝路径使用 `diff.before` 纯文本回写单块，不能精确恢复 marks、attrs 和嵌套结构。
- open session / background HTML 路径继续字符串替换会形成第二套结构真值。

## 11. 验证计划

- 前端单元：层级投影覆盖普通段落、列表容器、多个 listItem、嵌套列表、blockquote、链接与 marks；
  container 无写能力，content Block 指纹稳定。
- 前端单元：BlockOperation Schema、节点类型/指纹/会话基线校验；多目标单事务全部成功或零应用；
  reject 精确恢复；后置编辑返回冲突。
- Rust 单元：OpenDocSnapshot 结构块透传、BlockOperation 载荷校验、工具 Schema、容器目标拒绝、
  legacy rich `replace_block` 拒绝、历史 DiffProjection 重建。
- 集成：AG ToolResult → DiffCard → ED 草稿 → accept/save 与 reject restore；保存重开后列表项仍独立。
- 回归：StyleOp、TableOperation、ImageOperation 既有专项测试不退化。
- 执行 `npm run test`、`npm run check`、`npm run check:rust`、`npm run governance:generate`、
  `npm run governance:audit` 与 `git diff --check`。

## Scope Lock

- scope_id：`AG-RICH-BLOCK-STRUCTURED-EDIT-01`
- task_type：`CODE_CHANGE`
- governance_phase：`IMPLEMENTATION`
- confirmation：人类已将 `.governance/active_scope.json.confirmation` 设为 `YES`。
- phase_gate：AG/ED/DE/SYS 的需求、设计、计划和状态机均已存在；本课题在既有规则链上收紧结构化协议，
  设计同步与实现同一 Scope 完成。
- allowed_files / forbidden_files：以 `.governance/active_scope.json` 为准。
- 当前状态：`READY/CLOSED`（实现与自动验证完成；人类于 2026-07-31 指示闭合并提交，未把未执行的
  WKWebView 操作写成已验证）。

## 12. 实施结果

- 活动富文档由编辑器直接投影 ProseMirror 层级块快照；列表/列表项等容器只提供父子关系和顺序，
  只有安全的 paragraph/heading/codeBlock 内容节点开放 `replace_node`。
- 新增 `block_ops` 批协议：同次 sessionVersion/contentHash、blockId、fingerprint、
  expectedNodeType、受控 afterNode 全部通过后，后端补齐 exact beforeNode 并生成一个结构化 Diff。
- ED 在一个 ProseMirror transaction 中整批应用，任一目标失效则零应用；表格祖先、图片承载块和
  容器节点不能通过通用协议改写。
- DiffCard 按独立内容块展示 before/after；显式接受和拒绝均检查 afterNode fingerprint，后置人工
  编辑时保持 Pending 并显示 conflict；拒绝成功时按 exact beforeNode 单事务恢复。
- BDocx 富文档 `replace_block/replace_document` 在生成前明确拒绝；纯文本兼容路径和表格/图片专用
  结构协议保持独立。
- BDocxCapabilityManifest、工具 Schema、提示词、AG/ED/DE/SYS 接口与设计文档已同步到同一
  `block_ops` 能力口径。

## 13. Task Runtime Report

task_type: CODE_CHANGE
governance_phase: IMPLEMENTATION
final_state: READY
issue_trace: AG-RICH-BLOCK-STRUCTURED-EDIT-01

changed_files:
  - `src/services/edBlockOps.ts` / `src/services/edBlockOps.test.ts`：层级块投影、指纹、原子应用与精确恢复。
  - `src-tauri/src/ag/block_ops.rs`：不可信 BlockOperation 批的严格校验与规范化。
  - `src/services/agService.ts`、`src-tauri/src/ag/blocks.rs`、`src-tauri/src/lib.rs`：同次 LogicalState
    快照、工具路由、Diff 生成和历史投影。
  - `src/store/edStore.ts`、`src/store/deStore.ts`：共享快照、载荷与运行期投影。
  - `src/components/ed/Editor.tsx`、`src/services/deService.ts`：单事务应用、显式接受/拒绝冲突保护。
  - `src/components/de/DiffCard.tsx`：逐块结构化审阅。
  - `src-tauri/src/ag/bdocx_capabilities.rs`、`src-tauri/resources/bdocx-agent-capabilities.json`、
    `src-tauri/src/ag/prompts.rs`、`src-tauri/src/ag/tool_registry.rs`：能力清单、Schema 与提示词同源。
  - `docs/01_modules/{SYS,AG,ED,DE}` 相关协议/设计/状态机：同步结构化块编辑契约。

scope:
  mode: explicit
  status: confirmed

phase_gate:
  status: passed
  evidence: 复用既有 AG-TURN/AG-TOOL/DE-DIFF/ED-EDIT 链和现有规则，不新增游离业务规则。

mapping_chain:
  status: complete
  evidence: ProseMirror 快照 → AgentEditableSnapshot → block_ops → PendingDiff → ED 单事务 →
    DiffCard/accept/reject 已由协议、实现、@GOV 与测试共同覆盖。

validation:
  - `npm test`：passed（57 files / 447 tests）。
  - `cargo test --lib`：passed（261 tests，包含沙箱外真实 bundled LibreOffice 端到端转换）。
  - `npx tsc --noEmit`、`cargo fmt --check`：passed。
  - `npm run verify:bundled-tools`、`npm run build`：passed。
  - `npm run governance:generate` + `npm run governance:audit`：passed，governance_findings 为空。
  - `git diff --check`：passed。
  - 语义候选人工比对：passed；BlockOperation 的活动快照、稳定 Block 锚、Diff 精确定位和 manifest
    三投影分别与 BR-AG-DATA-003、BR-ED-DATA-001、BR-DE-DATA-001、BR-SYS-DATA-006 一致。
  - WKWebView 真实代理修改目录列表、接受/拒绝、保存重开：skipped；自动验证不替代人工视觉/交互验收。

stop_conditions:
  - 无。

recovery:
  - 无。

unresolved_items:
  - 真实 Tauri/WKWebView 中的目录列表代理多项修改、接受、拒绝及保存关闭重开未在本轮执行；若后续
    人工验收发现窗口级交互问题，另开缺陷课题，不回写本轮自动测试结果。

## 14. 人类闭合决定

- 2026-07-31：人类明确指示“先把现有的改动闭合提交”，接受以现有实现及全量自动门禁通过作为本批
  改动的提交边界。
- 闭合前已修复选区锚节点起点偏移、Markdown front matter 旧预期、HTML 已净化内联结构旧预期，并
  校正 bundled LibreOffice 版本清单；前端、Rust、构建和治理门禁均已重新全量通过。
- 本决定不把尚未执行的 Tauri/WKWebView 人工操作写成已验证；后续发现的真实窗口交互缺陷应建立
  新 Issue Trace 处理。
- final_state：`CLOSED`。
