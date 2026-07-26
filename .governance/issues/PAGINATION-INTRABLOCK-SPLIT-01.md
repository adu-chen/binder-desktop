# Issue Trace · PAGINATION-INTRABLOCK-SPLIT-01 Block 内分页拆分

承接 `BR-ED-DATA-003` / `ED-EDIT`。本课题在保持单一 ProseMirror 文档与分页纯呈现边界的前提下，
将分页输入从 Block 高度序列扩展为可断开的视觉流片段，先实现普通段落的行间分页；复杂容器、表格、
代码和媒体维持显式原子降级。不得以拆分或改写 ProseMirror 文档节点伪造分页。

当前状态：`READY/CLOSED`。人类于 2026-07-26 先确认关闭 `SCROLL-JUMP-PAGINATION-01` 并开启
本课题，后在多轮正式应用手工测试、方案纠偏与残余效果复核后明确决定不再继续优化，并接受本节
§17 所列发布边界。

## Scope Lock

```text
task_type: CODE_CHANGE
allowed_files:
  - .governance/active_scope.json
  - .governance/issues/PAGINATION-INTRABLOCK-SPLIT-01.md
  - .governance/issues/SCROLL-JUMP-FIX.md
  - .governance/issues/SCROLL-JUMP-PAGINATION-01.md
  - docs/01_modules/SYS/A-SYS-C-T-02_前端交互架构与流程总图.md
  - docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md
  - docs/01_modules/ED/A-ED-X-L-02_编辑器专项优化计划.md
  - src/components/ed/Editor.tsx
  - src/components/ed/Editor.lifecycle.test.tsx
  - src/components/ed/extensions/Pagination.ts
  - src/components/ed/extensions/Pagination.test.ts
  - src/components/ed/extensions/paginationFlow.ts
  - src/components/ed/extensions/paginationFlow.test.ts
  - src/components/ed/extensions/paginationViewportAnchor.ts
  - src/components/ed/extensions/paginationViewportAnchor.test.ts
  - src/services/workspaceService.ts
  - src/services/workspaceService.test.ts
  - src/services/smListeners.ts
  - src/services/smListeners.test.ts
  - src/services/edService.ts
  - src/services/edService.test.ts
  - src/store/edStore.ts
  - src/store/edStore.test.ts
  - src/styles.css
  - docs/00_core/A-CORE-C-R-01_ADUS.md # 仅 governance:generate 生成
forbidden_files:
  - ADU.md
  - ADP.md
  - AGENTS.md
confirmation: YES
scope_reason: 人类明确要求闭合整个分页课题：完整保留现有物理分页、页缝、页码、纸张高度、
  打开收敛和既有防跳控制；普通段落行中强制断开与复杂节点原子降级作为当前版本已接受边界，
  不再继续优化。闭合审计覆盖两个Issue及其实现、测试和设计，不改变LogicalState、DiskState、
  脏态、保存与后端转换。
```

## 1. 用户可见症状

BDocx 分页模式只能在顶层 Block 边界切页，列表最多到 `listItem` 边界。较长段落若不能完整放入当前页，
会整体移到下一页，导致上一页底部出现大块空白；超整页 Block 会独占并溢出，分页观感不符合连续排版预期。

## 2. 入口点

- 用户打开或编辑 BDocx → `src/components/ed/Editor.tsx` 配置分页。
- 文档事务、首次布局、纸张几何、字体/媒体或窗口变化 →
  `src/components/ed/extensions/Pagination.ts` 的 plugin view 调度测量。
- `measurePaginationPlan` 读取 Block 与普通段落真实行盒 → `paginationFlow` 生成方案 →
  `buildDecorations` 在 Block 或普通段落自然视觉行间插入分页缝。

## 3. 相关规则

- `BR-ED-DATA-003`：分页属于 DisplayState，不得改变 LogicalState、DiskState 或脏态。
- `X-FORBID-005`：分页只经 ED 声明的 ProseMirror Decoration 扩展点工作，不绕过受控编辑接口。
- `X-FORBID-008`：DisplayState 不得反向修改 LogicalState；保存仍是 LogicalState 到 DiskState 的唯一转换路径。

未发现需要新增 RULE；本课题是既有规则下的分页算法扩展。

## 4. 相关链路

`ED-EDIT`

分页测量与 Decoration 提交均位于 ED 内部，不新增跨模块链路。

## 5. 生产者块

- `BR-ED-DATA-003-RB-ED-ED-EDIT-005` 位于
  `src/components/ed/extensions/Pagination.ts`：由当前文档和页面几何测量视觉流、映射 position 型计划并
  生成 Decoration。
- `BR-ED-DATA-003-RB-ED-ED-EDIT-017` 位于
  `src/components/ed/extensions/paginationFlow.ts`：由原子 Block/普通段落视觉行序列生成 position 型
  `PageBreakPlan`，执行贪心填页与超高行必须前进规则。

## 6. 消费者块

- 无独立 `@GOV` 消费者。相同生产者块内部的 ProseMirror plugin `props.decorations` 消费分页方案，
  `src/styles.css` 消费分页缝 DOM 与 fill CSS 变量。
- `src/services/docService.ts` 中同规则的 BDocx 转换块不消费分页方案，本课题不得修改该路径。

## 7. 路径上的空规则块

未发现执行路径上的 `rules: []` Block；无 `TRIAGE_REQUIRED`。

## 8. boundary 假设

- 当前 owner boundary 为
  `in=ProseMirrorDocumentAndPageGeometry | out=pageBreakDecorations(presentationOnlyNoModelChangeNoDirty)
  | delegate=paginationFlow`。
- 当前代码块未声明 `lifecycle_assumption` 或 `document_snapshot`。
- 输出边界继续成立；纯规划 delegate boundary 为
  `in=RenderedFlowBlockSequenceAndPageContentHeight |
  out=PageBreakPlan(proseMirrorPositionBreaksWithPageFill)`。
- 分页断点必须使用当前文档可映射的 ProseMirror position；不得保存为文档内容或跨会话锚。

## 9. 运行时假设

- 当前代码块未声明 `caller_context` 或 `runtime_guard`。
- 分页仅在启用且非 composition 测量期运行；内容输入继续等待 450ms 静默结算。
- 完整物理分页必须持续启用；Block 页缝、页码、纸张高度与末页补白不得因普通段落测量被撤下。
- 已有段内断点重测时，只允许在同一 rAF 回调内以呈现事务暂时过滤 `paragraph-line` widget，读取当前
  可见 ProseMirror 真实 Range 行盒后，在回调返回前恢复最终完整方案与可见锚点；中间态不得跨帧，
  不得进入 history/dirty，不得重新引入整个 ProseMirror 的 ResizeObserver。
- 冻结期内分页断点 position 必须随 `tr.mapping` 映射；失效断点安全丢弃。
- 行盒或 position 几何不可可靠取得时，必须把该 Block 降级为原子 Block，不提交猜测断点。

## 10. 疑似不变量破坏

- Block 内 widget 可能扰动 WebKit 光标、选区和 IME；必须先通过正式 Tauri/WKWebView 可行性门禁。
- 断点由 Block 序号迁移为文档 position 后，若计划与 Decoration 未同步映射，会产生错位或陈旧分页缝。
- 既有分页缝会改变后续行盒坐标；若测量未按断点 position 扣除既有 gap 高度，会重新形成累计误差。
- 行扫描若退化为整篇逐字符 rect 读取，会放大 123 页文档的主线程成本。
- 复杂容器若未经独立规则直接插入全宽 gap，可能破坏列表标记、引用边框、表格 DOM 或代码块语义。
- 分页结构变化仍可能触发滚动回归；现有 450ms 冻结、原生 preserve、激活周期和单次残差补偿均为硬约束。

## 11. 验证计划

- 可行性门禁：正式 Tauri/WKWebView、DevTools 关闭，验证普通段落行间 `span` widget 的点击、光标、
  左右移动、输入、删除、选区、复制粘贴、Undo/Redo 与 IME；失败即停止代码扩展。
- 纯逻辑测试：视觉流片段分页、精确满页、跨两页/三页、最大可容纳行、零行才换页、原子降级、fill 不变量。
- 映射测试：断点前/断点处插入与删除、断点 position 删除、冻结期 plan 与 Decoration 同步映射。
- 测量测试：Block 断点与行间断点的 gap 高度扣除、不可测 Range 降级、不得整篇逐字符扫描。
- 回归测试：既有 Pagination、viewport anchor 与编辑器生命周期测试全部通过；分页事务保持
  `docChanged=false`、不进 history、不改变 dirty。
- 真机矩阵：50%/75%/100%，中文/英文/混合字号与 marks、长段落跨多页、断点前后编辑、Enter、
  Backspace/Delete、IME、切换纸张与方向；静默结算后可见锚点残差不超过 1 CSS px且无持续抖动。
- 降级矩阵：标题、TaskList、引用、代码、表格、图片/媒体在第一阶段维持原子行为，不重叠、不破坏编辑。
- 链路日志：本链路当前没有强制业务日志；不得为分页高频测量新增生产日志。
- triage：不新增 `rules: []` Block；若实现中必须新增则先分类并更新本节。
- 治理：设计与 `@GOV boundary` 同步后运行 `npm run governance:generate`，再运行
  `npm run governance:audit`；同时执行 TypeScript、Vite 生产构建和 `git diff --check`。

## 实施顺序

1. 正式 WKWebView 行间 widget 可行性门禁。
2. 新增纯逻辑 `paginationFlow`，定义原子片段/视觉行片段与 position 断点。
3. PageBreakPlan 从 Block 序号迁移为可映射 position；冻结期同步映射。
4. 两级测量：先测 Block，只对实际溢出的普通段落定位视觉行。
5. 普通段落行间 widget 与 CSS；至少保留前后各两行。
6. 接回现有输入冻结和视口锚点补偿，完成自动与真机回归。
7. 列表、引用、表格和媒体不在第一阶段扩展；需基于证据另行扩 Scope。

## TERM 前置判断

本课题复用已注册 `Block`、`DisplayState`、`LogicalState`、`DiskState` 与 BDocx。`FlowSegment`、
`PageBreakPlan` 属 ED 内部技术结构，不写入 `term_ref`，不需要新增 TERM。

## 12. 实现记录（2026-07-26）

- 新增 `paginationFlow.ts`：原子 Block 规则与旧算法等价；普通段落按视觉行片段贪心分页，当前页保留
  所有完整可容纳行，仅零行可容纳时换页；超整页行接受溢出并保证规划前进。
- `PageBreakPlan` 从 Block 序号迁移为 `id+pos+kind+fill`；内容事务期间 position 以 `assoc=-1`
  映射，失效行间断点丢弃，页序 id 重新规范化，Decoration 同步映射。
- 测量改为 position/bias 归一：Block top 与视觉行首包含同 position 的 `side:-1` gap，Block bottom
  排除下一 Block 同 position gap；段落内部 gap 同时从本 Block bottom 和后续坐标扣除。
- 两级测量先读取全部 Block 连续高度，仅对当前页实际溢出的顶层普通段落定位视觉行；合法位置按 grapheme
  生成，视觉行以指数探测+二分定位，Range 不可靠时退化 `coordsAtPos`，仍不可用则整段原子降级。
- 普通段落行间 gap 使用 `span` widget，Block gap 保持 `div`；均 `contenteditable=false`、
  `aria-hidden=true`、`pointer-events:none`，不设置 `relaxedSide`，不改变文档/history/dirty。
- 标题、列表/TaskList、引用、代码、表格、图片/媒体与 inline atom 第一阶段仍原子降级；不扩大 Scope。
- 自动验证：分页相关 54 项通过；TypeScript 与 Vite 生产构建通过；全量前端 345/349，4 项失败均为
  本课题前已存在且 Scope 外的 `src/services/edLocate.test.ts` 选区偏移断言。
- 治理验证：`governance:generate` 已登记新增 delegate owner，`governance:audit` 状态 `passed`，
  `governance_findings=[]`、`triage=0`。
- 未验证：正式 Tauri/WKWebView、DevTools 关闭下的点击/光标/选区/复制粘贴/Undo/Redo/IME，以及
  50%/75%/100% 长段落跨页矩阵。未完成人工矩阵前不得把状态改为 CLOSED。

## 13. 人工验收反馈与规则纠偏（2026-07-26）

- 人工测试确认分页、普通段落拆分和跳跃问题基本正常。
- 发现同一段落跨页时，2+2防孤行规则会把上一页本可完整容纳的最后一行撤回下一页，使上一页留下额外空白。
- 根因是规划器主动平衡行数，不是position断点、行盒测量、硬件适配或滚动补偿异常。
- 人类明确选择贪心填页：当前页能放多少完整视觉行就保留多少，只有一行都放不下时才换页；接受页底或页首
  只有一行。该纠偏不改文档模型、Decoration形态、position映射、450ms冻结或视口补偿。

## 14. 行尾断点复盘与修复（2026-07-26）

- 人工验收确认新的问题不是“少填一行”，而是普通段落在原本未换行的位置被分页缝强制从行中拆开；分页缝
  移动后，旧位置还可能维持硬换行外观。
- 根因是行间 `span` widget 以 `display:block` 进入段落 inline formatting context；结算重测虽然按
  position 扣除了分页缝高度，却仍在包含旧 widget 的可见 DOM 上定位视觉行，无法撤销 widget 已改变的
  换行拓扑，因此旧 position 会被再次识别为“行首”并形成自证闭环。贪心规划和 `side:-1` 均不是根因。
- 人类验收口径：允许在浏览器自然视觉行尾形成硬分页换行，不允许在自然视觉行中间拆分。优先级固定为
  “禁止行中拆分 > 分页稳定 > 贪心填满”；无法可靠识别自然行尾时整段原子换页。
- 修复采用当前纸张宽度、字体和 inline 样式的离屏段落副本，克隆时排除 `.rm-page-gap`，不隐藏或改写
  正在编辑的 ProseMirror DOM。每个候选断点除进入更低视觉行外，还必须按文字方向回到行首侧；字号、
  上下标等只改变字形纵坐标但横向仍连续的情况不得误判为换行。
- 已有段内 gap 的段落即使按污染高度看似不溢出，也必须先以自然副本恢复 Block 高度；新方案不再包含旧
  position 时重建 DecorationSet 并移除 widget，浏览器据原文自动恢复连续排版。该恢复不写 ProseMirror
  doc、history、dirty 或保存内容；如果移除 widget 后仍存在硬断行，则属于源文档 `hardBreak`/段落结构
  或另一路模型事务，需要按独立证据处理。
- 自动验证：分页相关57项通过，TypeScript与Vite生产构建通过；全量前端348/352，4项失败仍全部来自本
  课题前既有、Scope外的 `edLocate` 选区偏移断言；`governance:audit` 为 `passed` 且
  `governance_findings=[]`。正式Tauri/WKWebView中50%/75%/100%行尾视觉、重排恢复与输入跳动仍待人工验收。

## 15. 二次人工失败与测量真值纠偏（2026-07-26）

- 二次人工测试仍出现：同一上方内容保留至少两处历史拆分，下方当前分页缝继续在无明显规律的行中截断。
  该证据否定§14“过滤 `.rm-page-gap` 后克隆可见段落即可得到自然排版”的假设。
- v1.31副本虽然排除了分页缝元素，却仍从当前 contenteditable DOM 克隆正文；WKWebView/ProseMirror可能
  在 block widget 邻近位置留下辅助 `<br>`、拆分文本节点或尚未清理的残留结构，这些节点会被原样复制，
  因而离屏副本仍不是自然排版真值。
- 纠偏后，测量段落只能从当前 ProseMirror LogicalState 通过 schema `DOMSerializer` 重新生成；模型与
  序列化DOM按text/hardBreak逻辑offset重新建立Range映射，不再调用可见DOM `domAtPos` 复制结构。
- 含 hardBreak 的段落整体原子降级，不把显式短行或疑似旧widget邻近污染当成自然行尾。普通纯文本段落仍须
  通过纵向进入下一行+按文字方向横向回到行首的双重验证；任一条件不可靠即整段换页。
- 历史位置若已经真实进入ProseMirror模型成为多个paragraph/hardBreak，本轮不会猜测并删除正文结构，以免
  误删用户原有换行；本轮先阻断新增错误拆分。是否存在模型污染须以分页前后`editor.getJSON()`差异确认。
- 自动验证：分页与视口相关58项通过，TypeScript与Vite生产构建通过；全量前端349/353，4项失败仍全部来自
  本课题前既有、Scope外的`edLocate`选区偏移断言；`governance:generate`生成176个`@GOV`索引块，
  `governance:audit`为`passed`且`governance_findings=[]`。正式Tauri/WKWebView中的当前行尾、历史断点
  与反复重排结果仍待人工验收，未经人工结果不得宣称彻底修复或将课题置为CLOSED。

## 16. 物理分页边界纠正与真实可见行测量（2026-07-26）

- 人类明确纠正：本课题只修普通段落 Block 内拆分，不能移除分页功能；页缝、页码、纸张高度、末页补白、
  450ms输入冻结和既有防跳控制均属于必须保留的发布能力。
- 重新核对发现，schema `DOMSerializer` 离屏副本虽规避了可见DOM残留，却仍需人工复制纸张宽度、字体、
  marks、行高、方向与缩放环境；它仍可能只得到“逻辑行尾”，不等同当前WKWebView最终排版的真实行尾。
- 现行方案不再创建离屏段落。首次无段内页缝时直接读取可见ProseMirror；已有段内页缝时先捕获可见锚点，
  在同一个rAF回调内用`addToHistory=false`的分页meta暂时从DecorationSet排除paragraph-line widget，
  Block页缝始终保留；随后直接用`domAtPos + Range`测量当前纸张上的自然grapheme行盒，并在rAF返回前
  提交最终完整plan、恢复所有段内页缝与锚点。浏览器不得绘制中间态。
- 回归规则已显式化：自然行测量态下`block`页缝必须渲染，只有`paragraph-line`页缝可暂时排除；任何
  null、异常、重试和same-plan分支都必须退出测量态。该过程不改变doc、selection、history、dirty或保存内容。
- 自动验证：分页/分页流/视口锚定定向测试55/55通过，TypeScript检查与Vite生产构建通过；
  全量前端349/353通过，4项失败仍全部是本课题前已存在且Scope外的`edLocate`选区偏移断言；
  `governance:generate`生成176个`@GOV`索引块，`governance:audit`状态`passed`且
  `governance_findings=[]`。统一`npm run build`仍被独立DOC课题的bundled LibreOffice版本不一致阻断，
  但其后置的TypeScript与Vite步骤已分别运行并通过。
- 正式Tauri/WKWebView仍待人工验收；至少覆盖分页仍存在、行中不拆分、旧错误断点可恢复、
  50%/75%/100%以及输入静默结算无新增跳动。未经该结果不得将课题置为CLOSED。

## 17. 课题闭合与接受边界（2026-07-26）

人类在正式应用中完成了分页、块拆分与跳跃行为的多轮手工测试，并在确认当前仍存在普通段落行中
强制断开后，明确决定“不再修复，闭合分页课题并提交”。该最终裁决取代 §12、§15、§16 中
“正式矩阵未完成不得 CLOSED”的执行期门禁；历史段落继续保留，用于说明方案演进和失败证据，
不再表示当前存在未解除 Stop Condition。

闭合时实现、设计与发布边界一致：

- 物理分页、页缝、页码、纸张高度、末页补白、450ms 输入冻结、首帧揭示、打开单飞/latest-wins
  和光标激活周期均保留；本课题没有通过关闭分页或撤回防跳控制换取表面稳定。
- 分页仍是 ProseMirror Decoration/CSS 的 DisplayState 行为；分页事务不改 doc、history、dirty，
  不进入 LogicalState、DiskState 或保存内容。
- 普通段落使用可映射 position 的 `paragraph-line` `span` widget。虽然断点测量读取同帧暂时排除
  旧段内 widget 后的当前可见 Range 行盒，正式 WKWebView 仍可能呈现已观察到的行中强制断开。
  该表现未被声明为已修复，而是由人类明确接受为当前版本限制。
- 标题、列表/TaskList、引用、代码、表格、图片/媒体、含 hardBreak 或 inline atom 的段落，以及
  几何不可可靠测量的段落继续原子降级；它们可能整块后移、留下页尾空白或超页。
- 不再为本课题建立后续优化 Issue。若未来重新要求 Word 级自然行分页，应新开课题并重新评估编辑器
  架构；不得直接恢复 §14/§15 已被否定的离屏近似方案。

闭合验证基于仅包含本分页提交的隔离索引工作树执行：

- 分页、分页流、视口锚定、编辑器揭示门禁、打开单飞/latest-wins 与会话投影专项测试 `86/86` 通过；
  完整前端回归 `346/346` 通过。
- `tsc --noEmit` 与 Vite 生产构建通过；仅保留 Vite 既有大 chunk 警告，不影响构建结果。
- `governance:generate` 生成 `173` 个 `@GOV` 索引块；`governance:audit` 状态 `passed`、
  `governance_findings=[]`。对本课题 `BR-ED-DATA-003` 候选块执行语义比对，分页规划与 Decoration
  boundary 均保持 DisplayState-only 语义，未发现规则/实现漂移。
- `git diff --cached --check` 通过。本闭合轮未新增一套完整真机矩阵；最终发布边界采用此前多轮正式应用
  手工测试证据与本轮人类明确接受裁决，不把未解决的行中强制断开伪装成验证通过。

最终状态：

```text
task_type: CODE_CHANGE
governance_phase: VALIDATION
final_state: READY
issue_trace: PAGINATION-INTRABLOCK-SPLIT-01
scope_status: CLOSED
human_acceptance: YES
successor_issue: NONE
accepted_limitations:
  - paragraph-line widget may still force a visual break inside a natural line in WKWebView
  - complex or unreliable blocks remain atomic and may leave blank space or overflow
```
