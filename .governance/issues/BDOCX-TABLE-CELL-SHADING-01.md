# Issue Trace — BDocx TableCellShading

## 1. 用户可见症状

BDocx 表格当前没有单元格纯色底纹入口，代理也不能表达或审阅此类修改。即使直接在 HTML 的
`td/th` 上写入 `background-color`，Pandoc 保存为 DOCX 时也不会生成对应的 `w:shd`，重新打开后
底纹会丢失。

## 2. 入口点

- 人工编辑：`src/components/ed/EditorToolbar.tsx::EditorToolbar` 的表格上下文工具组。
- 代理读取：`src/services/edTableOps.ts::projectTables` → `list_tables`。
- 代理写入：`src-tauri/src/lib.rs::table_ops_payload` →
  `propose_edit(operation=table_ops)`。
- 保存与重开：`src-tauri/src/doc/edit_chain.rs::html_to_docx` /
  `docx_to_edit_html`。

## 3. 相关规则

- `BR-ED-DATA-007`：本课题在 ED 技术设计中新增，约束受限 `TableOperation`、单元格底纹属性、
  坐标/颜色校验和原子应用。
- `BR-DOC-DATA-002`：样式子集与损失告知；本课题扩展为 TableCellShading 的 OOXML 双向适配。
- `BR-DE-DATA-002`：DiffPayload 的可重建载荷与结构化审阅信息。
- `BR-DE-STATE-001`：pending 草稿的接受、拒绝和保存语义保持不变。
- `BR-AG-VERIFY-001`：代理只能经注册工具和严格参数校验产生修改。
- `X-FORBID-005`、`X-FORBID-008`、`X-FORBID-009`：人工/代理修改均须经 ED/DE 受控路径，
  DisplayState 不得直接改 LogicalState，代理产出不得绕过 Diff。

## 4. 相关链路

人工路径：`ED-EDIT → DOC-CONVERT`

代理路径：`AG-TURN → AG-TOOL → DE-DIFF → ED-EDIT → DOC-CONVERT`

## 5. 生产者块

- `src/components/ed/extensions/TableCellShading.ts`：生产 ProseMirror
  `tableCell/tableHeader.backgroundColor` 属性及 HTML 承载。
- `src/services/edTableOps.ts::projectTables`：生产含 `backgroundColor` 的 `TableSnapshot`。
- `src-tauri/src/doc/docx_cell_shading.rs::restore_html_cell_shading`：从 `w:shd` 生产编辑 HTML 属性。
- `src-tauri/src/lib.rs::execute_agent_propose_edit`：生产 `table_ops` DiffPayload。

## 6. 消费者块

- `src/services/edTableOps.ts::planTableOperations/applyTableOperations`：校验并应用
  `set_cell_background`。
- `src/components/de/DiffCard.tsx::DiffCard`：消费前后表格快照，显示底纹色块与十六进制值。
- `src-tauri/src/lib.rs::table_ops_payload`：消费模型参数并执行后端严格校验。
- `src-tauri/src/doc/docx_cell_shading.rs::materialize_marked_cell_shading`：消费编辑 HTML 中转标记，
  写入 DOCX `w:shd`。

## 7. 路径上的空规则块

当前表格实现没有 `rules: []` 执行块；`edTableOps.ts` 的导出执行函数属于课题 A 留下的
`BLOCK_UNANNOTATED` warning。本课题以新增 `BR-ED-DATA-007` 映射表格规划/应用主责块，不把
TableCellShading 错挂到 StyleOp。

## 8. boundary 假设

- `docx_to_edit_html` 的 `document_snapshot` 未显式声明；实际输入是同次打开的稳定 DOCX 字节，
  TableCellShading 恢复与颜色恢复必须基于同一份字节。
- `planTableOperations` 使用 `pre-update` 的 `TableSnapshot.fingerprint`；把
  `backgroundColor` 纳入指纹后，该假设仍成立。
- Diff 草稿应用使用 `base_session_version/content_hash`；本课题不改变其生命周期。

## 9. 运行时假设

- `table_ops` 的 `caller_context` 是当前活动 BDocx 编辑器会话；未打开、非活动或非 BDocx 目标继续拒绝。
- `set_cell_background` 只接受合并单元格左上锚、`null` 或不透明 `#RRGGBB`。
- DOCX 双向适配只处理显式纯色 `w:shd`；主题色、透明度、渐变和图案不进入保证集合。

## 10. 疑似不变量破坏

- 若底纹未进入表格指纹，旧 Diff 可能对已改变样式的表格继续应用，造成错位审阅。
- 若只改 HTML，保存后底纹静默丢失，违反 `BR-DOC-DATA-002`。
- 若用文本替换实现底纹，富内容单元格会被破坏，违反受限 TableOperation 原子语义。
- 若 DiffCard 只显示“表格操作 1 项”，用户无法确认具体颜色与坐标，违反
  `BR-DE-DATA-002` 的可重建审阅要求。

## 11. 验证计划

- 前端单元测试：HTML 属性解析/渲染、颜色规范化、单格/多格/合并单元格操作、快照指纹变化、
  富内容保持、DiffCard 前后色块。
- 后端单元测试：`table_ops_payload` 对颜色、坐标和 capability 的严格校验。
- DOC 转换测试：HTML→DOCX 断言 `w:shd w:fill="RRGGBB"` 且无中转标记；
  DOCX→HTML 断言 `data-binder-cell-shading` 与 `background-color` 恢复；报告断言检测/恢复/未恢复计数。
- 集成回归：既有表格操作、文字颜色往返、样式报告、保存与重开链路。
- 治理：修改静态 TERM/RULE 后运行 `npm run governance:generate`，再运行
  `npm run governance:audit` 并消费本次规则的语义审计候选。

## Scope Lock

- scope_id：`BDOCX-TABLE-CELL-SHADING-01`
- task_type：`CODE_CHANGE`
- confirmation：以 `.governance/active_scope.json` 的人类确认值为准。
- allowed_files / forbidden_files：以 `.governance/active_scope.json` 为机器可读权威。

## 12. 实施与验证结果（2026-07-28）

implementation_state：`READY_FOR_MANUAL_ACCEPTANCE`，尚未由人类宣布 `CLOSED`。

已实现：

- ED：TipTap `tableCell/tableHeader.backgroundColor` 全局属性、HTML 稳定 data 属性、表格上下文底纹
  颜色/清除入口，以及一个或多个选中单元格的节点属性更新。
- TableOperation：`set_cell_background` 进入 TableSnapshot、fingerprint、解析、预览/原子应用与
  合并单元格左上锚校验；富内容子树不被替换。
- AG：系统提示词、工具注册 Schema 和后端载荷校验均公开并限制该操作；只生成 `table_ops` Diff，
  不直写文档。
- DE：DiffCard 按逻辑坐标展示修改前后色块与十六进制值/无底纹状态。
- DOC：保存侧把短生命周期标记物化为 `w:tcPr/w:shd`，重开侧从显式纯色 `w:shd` 恢复 HTML；
  `StyleReport` 增加检测/恢复/未恢复统计与风险告知。

自动验证：

- 相关前端链：8 个测试文件、85/85 通过；补充真实 TipTap schema 往返与锚点无底纹时清除入口测试，
  4/4 通过。
- TypeScript：`npx tsc --noEmit` 通过。
- Rust：TableCellShading 3/3、StyleSubset 3/3、TableOperation 后端校验 1/1、工具 Schema 1/1、
  Agent Prompt 1/1 通过；`cargo check --lib` 通过。
- 真实 Pandoc 往返：`html_docx_roundtrip_preserves_subset_when_pandoc_available` 1/1 通过，
  已同时断言 DOCX `w:shd`、中转标记清除和重开 HTML 底纹恢复。
- 治理：`npm run governance:generate` 成功；`npm run governance:audit` 为 `passed`；
  `git diff --check` 通过。

范围外基线：

- 全量前端为 372/376；仅 `src/services/edLocate.test.ts` 既有 4 项
  `resolveSelectionAnchor` 偏移断言失败，与本课题文件和底纹链无关，沿用课题 A 的既有记录。
- LibreOffice 预览稳定性与 bundled tool 发布校验为已登记独立债务，本课题未触碰，也不作为
  TableCellShading 的 DOCX 语义往返证明。

尚待人工：

- Tauri/WKWebView 中对单格、多格与合并单元格实际设置/清除底纹，确认工具入口和选区行为。
- 真实代理模型发起 `list_tables→table_ops(set_cell_background)` 后的 DiffCard 视觉细节。
- 保存 BDocx、关闭并重开后的编辑器视觉确认。自动测试已经验证数据与 OOXML 链，但未据此声称
  实际 UI 人工验收完成。

## 13. 课题闭合（2026-07-28）

- final_state：`CLOSED`
- 人类决定：2026-07-28 明确确认“可以闭合”。
- 闭合范围：单元格纯色底纹的人工入口、TableOperation、代理工具协议、结构化 Diff、DOCX
  `w:shd` 双向适配、样式报告和自动验证。
- 非闭合范围：主题/渐变/图案填充、嵌套表格、自由拖放或浮动定位，以及此前登记的 LibreOffice
  稳定性与发布工具债务；这些事项不随本课题关闭而被声明解决。
- 人工验证留痕：§12 中未执行的真实 UI/模型场景保留为非阻断后续验证项，不改变人类本次闭合决定。
