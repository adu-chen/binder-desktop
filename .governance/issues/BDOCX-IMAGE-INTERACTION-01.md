# Issue Trace — BDocx 图片人工交互

## 1. 用户可见症状

BDocx 中图片可被选中并显示右下缩放手柄，但在 WebKit 中拖动时宽度不随鼠标稳定变化。图片也没有
邻近的文档流移动和删除入口。顶部“行内/独占块”按钮只写入 `data-display`，真实 Pandoc 探针证明该
属性经过 HTML→DOCX→HTML 后丢失，因此当前 UI 暗示了不存在的持久能力。

## 2. 入口点

- 图片选中与拖宽：`src/components/ed/extensions/ResizableImage.ts::ResizableImage.addNodeView`。
- 图片上下文入口：`src/components/ed/EditorToolbar.tsx::EditorToolbar`。
- 编辑器挂载：`src/components/ed/Editor.tsx::Editor`。
- 编辑事务脏态：`src/components/ed/Editor.tsx::useEditor.onUpdate` →
  `src/store/edStore.ts::markDirty`。
- DOCX 往返：`src-tauri/src/doc/edit_chain.rs::html_to_docx/docx_to_edit_html`。

## 3. 相关规则

- `BR-ED-STATE-001`：图片尺寸、位置与删除属于用户编辑，成功事务必须产生 LogicalState 脏态。
- `BR-ED-DATA-001`：独立图片段落作为 Block 整体移动时保持原 blockId；删除后锚失效。
- `BR-ED-DATA-003`：图片以媒体内容块进入编辑模型；操作不得改写媒体资源或绕过 Workspace 边界。
- `BR-ED-DATA-002`：BDocx 继续只编辑统一 HTML 模型，二进制转换归 DOC。
- `BR-DOC-DATA-002`：只把真实往返有证据的图片能力列入 StyleSubset 保证。
- `BR-DOC-DATA-003`：重开后的 Block 锚按既有重建契约处理，不宣称 blockId 跨 DOCX 保留。
- `X-FORBID-004`、`X-SEQ-002`、`X-SEQ-004`：本课题不新增文件资源 I/O 或二进制转换路径。

本课题复用既有规则，不新增 RULE、CHAIN、CONSTRAINT 或 TERM。图片选择快照、移动方向等名称是
ED 内部技术结构，不写入 `@GOV term_ref`。

## 4. 相关链路

`ED-EDIT → DOC-CONVERT → ED-EDIT`

人工操作只发生在 ED-EDIT；保存和重开证据通过既有 DOC-CONVERT 链取得。

## 5. 生产者块

- `src/components/ed/extensions/ResizableImage.ts` 的 NodeView 当前生产图片选中态和宽度事务；本课题为
  该导出扩展补充 `BR-ED-STATE-001/BR-ED-DATA-003` @GOV 映射。
- 本课题新增 `src/services/edImageOps.ts`，生产活动图片只读快照及文档流移动/删除事务，映射
  `BR-ED-DATA-001/003` 与 `BR-ED-STATE-001`。
- `BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-004` 位于
  `src-tauri/src/doc/edit_chain.rs`，生产保存后的 DOCX 字节。

## 6. 消费者块

- 本课题新增 `src/components/ed/ImageMoveControls.tsx`，消费活动图片快照并调用 ED 受控操作。
- `src/components/ed/Editor.tsx::useEditor.onUpdate` 消费所有图片文档事务，序列化 LogicalState 并驱动
  `edStore.markDirty`。
- `BR-ED-STATE-001-RB-ED-ED-EDIT-002` 位于 `src/store/edStore.ts`，消费编辑事件形成脏态。
- `BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-003` 位于
  `src-tauri/src/doc/edit_chain.rs`，消费保存后的 DOCX 并生成重开 HTML。

## 7. 路径上的空规则块

- `ResizableImage.ts` 原本没有 @GOV；它不再只是样式适配层，因为直接决定宽度事务提交。分类为
  `RULE_UNMAPPED`，本课题补挂既有 `BR-ED-STATE-001/BR-ED-DATA-003`。
- `ImageMoveControls.tsx` 与 `edImageOps.ts` 是新增核心路径，必须直接标注，不采用 `rules: []`。
- 测试文件按 ADU §5.4/§5.6 豁免 @GOV，以 `// covers:` 连接规则。

## 8. boundary 假设

- NodeView 的 `getPos()` 只在节点仍存在时有效；提交前重新读取位置并校验仍为 image。
- 图片移动只允许 NodeSelection 命中的顶层、仅含一个图片的 paragraph；整个 paragraph 是移动 Block，
  其 blockId 与图片属性保持不变。
- 混排文字中的行内图片不具备块级移动方向；删除只删除图片节点，不改写同段文字。
- `document_snapshot=post-update`：`Editor.onUpdate` 序列化图片事务后的 ProseMirror 文档；成立。
- DOCX 重开会重建 blockId；本课题验证图片次序/尺寸/删除结果，不验证跨重开锚身份。

## 9. 运行时假设

- `caller_context=react_effect`：邻近控件根据 selectionUpdate、transaction 和真实 DOM 事件重算。
- `runtime_guard=node_selection_is_image`：未选中图片时控件不显示。
- `runtime_guard=standalone_top_level_image_paragraph`：只有独立顶层图片段落可上/下移动。
- `runtime_guard=document_capture_mouse_events`：拖宽使用 document 捕获阶段 mousemove/mouseup，不依赖
  WebKit 对原手柄 pointer capture 的持续派发。
- `runtime_guard=zoom_normalized_width`：DOM 测量和鼠标位移都按 `.ProseMirror` computed zoom 还原。

## 10. 疑似不变量破坏

- 若继续依赖手柄 `pointermove`，WebKit 丢失持续事件时 UI 看似进入拖动但 LogicalState 不变。
- 若只改图片 DOM width 而不提交节点属性，显示与保存内容会分叉，违反 `BR-ED-STATE-001`。
- 若混排图片按块移动，可能连带重排或丢失同段文字，违反 Block 稳定性。
- 若把 `data-display` 继续描述为 DOCX 可持久能力，会违反 `BR-DOC-DATA-002`。真实探针输出只保留
  `src/style(width,height)/alt`，不保留 `data-display`；本课题撤下该误导入口，行内与独立图片由段落
  结构区分。
- 若移动/删除重写 `src` 或 `data-binder-media-src`，会破坏课题 A 已闭合的媒体稳定身份。

## 11. 验证计划

- 前端：
  - 真实 TipTap NodeView + DOM mouse 事件验证拖宽预览、zoom 归一化、mouseup 单事务提交、height 清除。
  - 活动图片快照验证独立/混排判定、可移动方向、上/下移动保持 blockId/媒体属性/NodeSelection。
  - 删除验证独立图片移除整个段落、唯一段落保留可编辑空段、混排只删图片。
  - 邻近控件验证 portal 锚定选中图片、方向禁用、mousedown 不丢选区、事务后继续跟随。
  - Editor 生命周期验证控件接入且加载期禁用；Toolbar 验证不再呈现不可持久的 inline/block 按钮。
- DOC：
  - HTML→DOCX→HTML 验证图片尺寸和文档顺序；删除后最终 OOXML/重开 HTML 图片数同步下降。
  - `data-display` 丢失作为明确不支持边界记录，不把探针失败改写成保证。
- 链路日志：ED 高频图片交互无成功日志要求；错误流无 catch 吞错。DOC-CONVERT 沿用既有转换错误日志。
- triage：新增导出组件/函数全部直接映射既有规则；不新增 `rules: []` 条目。
- 命令：定向 Vitest、`npx tsc --noEmit`、定向 Rust tests、`cargo fmt --check`、
  `cargo check --lib`、`npm run governance:generate`、`npm run governance:audit`、
  `git diff --check`。

## Scope Lock

- scope_id：`BDOCX-IMAGE-INTERACTION-01`
- task_type：`CODE_CHANGE`
- confirmation：`.governance/active_scope.json` 已由人类设为 `YES`。
- allowed_files / forbidden_files：以 `.governance/active_scope.json` 为机器可读权威。
- 当前状态：`READY`，等待人类完成真实 Tauri/WKWebView 手工验收后再闭合。

## Task Runtime Report — BDocx 图片人工交互

task_type: CODE_CHANGE
governance_phase: VALIDATION
final_state: READY
issue_trace: `.governance/issues/BDOCX-IMAGE-INTERACTION-01.md`

changed_files:
  - `.governance/issues/BDOCX-IMAGE-INTERACTION-01.md` # 新增 Issue Trace、Scope Lock 与运行报告
  - `docs/00_core/A-CORE-C-R-01_ADUS.md` # 由 governance:generate 刷新 @GOV 派生索引
  - `docs/01_modules/ED/A-ED-M-D-01_编辑器功能需求描述.md` # 声明图片缩放、结构化移动、删除及混排边界
  - `docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md` # 补充图片交互设计并收紧 BR-ED-DATA-003
  - `docs/01_modules/ED/A-ED-I-P-01_编辑器接口协议.md` # 声明 ED 内部图片操作契约
  - `docs/01_modules/ED/A-ED-X-L-02_编辑器专项优化计划.md` # 更新图片交互课题与验收项
  - `docs/01_modules/DOC/A-DOC-M-D-01_文档转换功能需求描述.md` # 声明图片尺寸、顺序、删除往返边界
  - `docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md` # 收紧 BR-DOC-DATA-002 的图片保证范围
  - `docs/01_modules/DOC/A-DOC-I-P-01_文档转换接口协议.md` # 声明 data-display/浮动定位不受保证
  - `docs/01_modules/DOC/A-DOC-X-L-01_文档转换开发计划.md` # 补充真实 Pandoc 往返验收
  - `src/components/ed/Editor.tsx` # 接入选中图片邻近操作控件
  - `src/components/ed/EditorToolbar.tsx` # 移除无法持久化的行内/独占块入口
  - `src/components/ed/EditorToolbar.test.tsx` # 验证不再暴露误导入口
  - `src/components/ed/ImageMoveControls.tsx` # 新增锚定图片的上移、下移、删除控件
  - `src/components/ed/ImageMoveControls.test.tsx` # 验证 portal 锚定、禁用边界与受控调用
  - `src/components/ed/extensions/ResizableImage.ts` # 改为 document 级鼠标拖宽并单事务提交
  - `src/components/ed/extensions/ResizableImage.test.ts` # 验证 zoom、拖动、空点击与媒体身份
  - `src/services/edImageOps.ts` # 新增图片快照、结构化移动与删除事务
  - `src/services/edImageOps.test.ts` # 验证 Block/媒体属性/NodeSelection 和混排保护
  - `src/styles.css` # 图片选中、手柄与邻近控件样式
  - `src-tauri/src/doc/edit_chain.rs` # 增加图片顺序、宽度与删除的真实 Pandoc 往返测试

scope:
  mode: explicit
  status: confirmed

phase_gate:
  status: passed
  evidence: 需求、设计、计划、实现和验证均在确认后的活动 Scope 内推进；未触达 forbidden_files。

mapping_chain:
  status: complete
  evidence: REQ-ED-004 → BR-ED-STATE-001/BR-ED-DATA-001/003、BR-DOC-DATA-002/003
    → ED-EDIT/DOC-CONVERT → ResizableImage/edImageOps/ImageMoveControls/edit_chain → 前端与真实 Pandoc 测试。

validation:
  - `npx vitest run src/services/edImageOps.test.ts src/components/ed/ImageMoveControls.test.tsx src/components/ed/extensions/ResizableImage.test.ts src/components/ed/EditorToolbar.test.tsx src/components/ed/Editor.lifecycle.test.tsx` # passed，5 文件 14 项
  - `npx tsc --noEmit` # passed
  - `cargo test doc::edit_chain --lib` # passed，7 项
  - `cargo test doc::style_subset --lib` # passed，3 项
  - `cargo check --lib` # passed，仅有既有 dead_code warning
  - `cargo fmt --check` # passed
  - `npx vitest run` # partial，53 文件中 52 通过；401 项中 397 通过，4 项既有 edLocate 选区偏移断言失败
  - `npm run governance:generate` # passed，ADUS 已刷新
  - `npm run governance:audit` # passed，governance_findings 为空
  - `git diff --check` # passed
  - AI 语义比对 # passed：ResizableImage 的宽度事务对应 BR-ED-DATA-003/BR-ED-STATE-001；
    edImageOps 与 ImageMoveControls 的 Block 移动/删除对应 BR-ED-DATA-001/003；edit_chain 的图片
    往返断言严格落在 BR-DOC-DATA-002 已声明的尺寸、实例数量、文档流顺序与删除保证内，未发现语义漂移

stop_conditions:
  - 无

recovery:
  - 无

unresolved_items:
  - 真实 Tauri/WKWebView 中的拖宽连续性、邻近按钮位置和保存重开效果仍需人类手工验收；自动化已覆盖
    NodeView DOM 事件、事务和真实 Pandoc 往返，但不等同于实际窗口视觉验收。
  - 全量 Vitest 仍有 `src/services/edLocate.test.ts` 的 4 项既有选区偏移断言失败（实际值整体比旧预期少
    一个字符位置）；该文件不在当前 Scope，本课题未修改或绕过。

## 人类闭合决定

- 2026-07-28：人类指示继续下一课题，接受本课题以“图片拖宽事务、独立图片块移动/删除、混排保护、
  不可持久入口撤销，以及尺寸/顺序/删除的真实 Pandoc 往返已完成自动验证”为闭合边界。
- 本决定不把尚未单独反馈的 Tauri/WKWebView 人工操作写成已验证；若后续真实窗口发现拖动或控件定位
  问题，应另开缺陷课题，不回写本课题测试结果。
- 项目图片资源发现、代理图片插入/删除、结构化 Diff 与能力声明继续转入后继
  `BDOCX-IMAGE-AGENT-EDIT-01`。
- final_state：`CLOSED`。
