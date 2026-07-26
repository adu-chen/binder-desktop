# Issue Trace · SCROLL-JUMP-PAGINATION-01 分页重测量滚动闪跳

承接 `BR-ED-DATA-003` / `ED-EDIT`。本任务先修复分页呈现层在重测量与末页补白更新中造成的滚动闪跳；经真机复现后扩展至 BDocx 打开控制面，收敛启动恢复、状态机监听、内容装载与纸张几何的重复/交叉控制。不改变文档模型语义、保存内容、MD/直接 HTML 编辑路径或后端转换链。

## Scope Lock

```text
task_type: CODE_CHANGE
allowed_files:
  - .governance/active_scope.json
  - .governance/issues/SCROLL-JUMP-PAGINATION-01.md
  - docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md
  - docs/01_modules/ED/A-ED-X-L-02_编辑器专项优化计划.md
  - docs/01_modules/SYS/A-SYS-C-T-02_前端交互架构与流程总图.md
  - src/components/ed/Editor.tsx
  - src/components/ed/Editor.lifecycle.test.tsx
  - src/components/ed/extensions/Pagination.ts
  - src/components/ed/extensions/Pagination.test.ts
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
scope_reason: 修复 BDocx 分页闪跳：保留分页呈现层与文档模型边界，恢复 ProseMirror 原生滚动保持；分页断点结构与 fill DOM 更新解耦，连续输入期间冻结分页计划，静默后一次结算；分页结算在同一事务末尾校验并补偿残差，BDocx 仅在 setContent 与首轮分页稳定后揭示。
```

## 1. 用户可见症状

分页模式的 BDocx 编辑区在打开后即出现页面分割缝上下振荡，后页比首页严重；内容还会出现可见的回填过程。直接 HTML 编辑（无分页）手工验证无抖动。DevTools 打开时现象显著减轻，表明除分页呈现外还存在时序/主线程负载敏感的打开控制链。

## 2. 入口点

- workspace 启动恢复 → 编辑会话快照恢复 → `loadSeq` → `Editor.setContent`。
- BDocx `doc.open_for_edit` / 文本 `ws.read_doc` 的异步完成 → `edStore.loadDoc` → `loadSeq` → `Editor.setContent`。
- 内容载入 → 分页启用/纸张几何 → 分页 meta 事务；以及编辑事务 → `PaginationExtension` plugin view → 防抖测量 → `computePageBreaks`。
- 状态机监听注册/注销 → ED/DE/AG store 投影。

## 3. 相关规则

- `BR-ED-DATA-003`：分页布局是纯呈现层，不得改变文档模型、落盘内容或脏态。
- `X-FORBID-005`：分页只经 ED 声明的 ProseMirror 呈现扩展点工作，不绕过受控编辑接口。

## 4. 相关链路

`ED-EDIT`；启动恢复与状态机广播的系统投影路径须按 `X-FORBID-007` 复核。

## 5. 生产者块

- `BR-ED-DATA-003-RB-ED-ED-EDIT-005`，`src/components/ed/extensions/Pagination.ts`：由块高度生成分页呈现计划与 DecorationSet。

## 6. 消费者块

- ProseMirror 编辑视图消费分页 DecorationSet 与 `--rm-page-end-fill`，仅产生页面呈现。
- 无其他业务 @GOV 消费者；保存/脏态不应消费分页事务。

## 7. 路径上的空规则块

- 无新增 `rules: []` 代码块；测试文件不纳入 triage。

## 8. boundary 假设

- 既有 @GOV boundary `in=blockContentHeights | out=pageBreakDecorations(presentationOnlyNoModelChangeNoDirty)` 仍成立。
- 打开控制面新增不变量：活动编辑器只接受活动路径的最新载入；异步旧结果不得推进 `loadSeq` 或重置其分页几何。非活动会话允许后台补全内容，但不得夺回 `activePath`。

## 9. 运行时假设

- 分页插件仅在分页已启用且非 IME composition 时调度重测量。
- 可见编辑 DOM 的重测量不得先写入临时 class/style；读布局不得造成可见分页缝隐藏/恢复。
- 仅末页补白变化时不得重建普通分页缝 DecorationSet。
- 不得以整个 ProseMirror 的 ResizeObserver 作为重测量触发源：分页方案提交会改变分页缝与末页补白高度，
  观察该容器会把自身输出回灌为输入，造成打开文档后持续重排，且累计误差随页数放大。
- workspace 恢复与状态机监听必须单飞；StrictMode 卸载期间完成的旧监听注册必须无效化，避免同一后端状态被重复投影。
- 内容载入的分页开关与纸张几何必须一次提交；同一载入不得经多个命令反复清空/重建页面计划。
- 只有分页结算事务允许分页层校验滚动残差；普通输入和显式导航不得进入该补偿路径。结算前后须保持
  可见光标或视口首内容块的屏幕 Y，且每个测量 epoch 至多补偿一次；末页补白未造成锚点位移时不得写
  `scrollTop`。

## 10. 疑似不变量破坏

- 现有 `rm-measuring` 临时隐藏/恢复分页缝会改变可见呈现树，可能扰动 WKWebView 滚动与选区。
- 现有 `samePlan` 将 `lastFill` 与 `breaks` 合并比较，末页每字符变化可触发全部 gap widget 重建，违反分页重排应稳定呈现的要求。
- 原启动恢复可并发执行，状态机监听缺少启动中守卫，文档打开没有路径版本守卫；三者叠加可产生重复或交叉的 `loadSeq → setContent → geometry → pagination` 提交，解释“内容回填”和 DevTools 改变表现的时序特征。
- 本次不改变 `BR-ED-DATA-003` 的模型/脏态边界；若发现分页 meta 事务导致 docChanged 或脏态变化，进入 `SC-04`。

## 11. 验证计划

- 单测：`Pagination.test.ts` 覆盖分页缝高度扣除、仅末页补白变化不重建、分页结构变化才重建，以及分页方案提交本身不触发下一轮测量；workspace 恢复与状态机监听覆盖单飞；文档打开覆盖迟到结果不能夺回当前编辑器。
- 静态：TypeScript、governance generate/audit、diff check。
- 真机：DevTools 关闭，覆盖首/中/末页、停顿后重测量、缩放、列表/任务列表/表格/图片、跨页编辑与 IME；光标在可视区时不得出现滚动振荡。
- triage：不新增可执行块，不新增 triage 条目。

## 12. 实现记录（2026-07-25）

- 分页重测量已移除可见缝隐藏/恢复和容器 ResizeObserver 正反馈；末页补白不再重建普通分页装饰。
- 启动恢复与监听注册均改为单飞，监听在 StrictMode 卸载后的迟到注册会自行注销。
- 文档打开增加按路径 latest-wins 守卫；已切换路径的结果仅更新原会话，保持当前 `activePath/loadSeq` 不变。
- BDocx 内容载入将分页启用和纸张几何合并为一次布局事务；后续同几何 effect 为幂等空操作。
- 本修复相关的自动验证完成；完整前端测试另有 `edLocate` 选区锚定断言失败（不在本 Scope），真机可见抖动验收尚未执行，不能据自动测试宣称视觉问题已消失。

## 13. 剩余单跳修复（2026-07-25）

真机复测后，持续振荡已消失，但内容底部偶发一次跳变，缩小比例更易出现。根因是
`handleScrollToSelection` 把 `coordsAtPos` 的视口坐标与 `scrollTop/clientHeight` 的滚动坐标混算；
CSS `zoom` 下会将仍可见的光标误判为越界，并放行 ProseMirror 默认 `scrollIntoView`。现改为仅比较
光标 rect 与 `.editor-paper` rect：两者相交即接管，完全离开才放行。该修复不触及分页模型或内容状态。

## 14. 结构分页视口锚定（2026-07-25）

完整审计确认：分页计划提交不会经过 `handleScrollToSelection`，但新增/移动 `Decoration.widget` 会真实
下推后续内容；ProseMirror 还会在 decoration 更新时执行内部 scroll-preserve。现由分页扩展独占该路径：

- 结构 breaks 改变时，提交前捕获可见光标，退化为视口首内容块；下一帧按其新旧屏幕 Y 差一次性补偿
  `.editor-paper.scrollTop`；仅 `lastFill` 改变不补偿。
- 编辑器 DOM 以**内联** `overflow-anchor:none` 禁用 ProseMirror 内置 preserve（其检查不读取父容器 CSS），
  防止双控制器竞争写 `scrollTop`。
- 每轮测量携带 epoch；timer 已触发后到达的新输入/缩放会废弃旧 RAF 结果。输入法组合或用户在补偿帧手动
  滚动时不补偿。
- 末页补白叠加底部页边距，保证纸张高度与 `PageGeometry` 一致；残留 scroll 调试日志删除，避免 DevTools
  改变时序。

## 15. 同帧补偿与真实光标锚定（2026-07-25）

后续真机复测表明仍有“单跳”，说明原锚定即便最终补偿成功也没有满足首帧稳定性。原因有二：

- 原实现把补偿放在下一次 `requestAnimationFrame`；分页 widget 的插入已在前一帧被绘制，用户会先看到内容
  下推、随后再看到回拉。
- 当 widget 位于光标 PM position 的同一边界，`coordsAtPos` 可能按 decoration 的 `side` 取到分页缝另一侧，
  不是实际可见的插入光标。

现在结构分页提交后在**同一 JavaScript task**中读取实际布局并写入补偿；布局读取强制完成，浏览器尚未绘制
中间下推状态。锚点优先读取属于编辑器的原生 `Selection` range，WebKit 短暂不可读时才降级为 DOM range；均失败
才使用首个可见内容块。此路径仍只在 breaks 结构变化时运行，且保留 epoch、composition 与手动滚动保护。

## 16. 控制权复盘与输入期分页冻结（2026-07-26）

继续真机复测仍不能消除输入后的偶发单跳。复核当前安装的 `prosemirror-view` 实现后确认，前一轮通过
ProseMirror DOM 的内联 `overflow-anchor:none` 不仅停用了浏览器锚定，也使 ProseMirror
`storeScrollPos/resetScrollPos` 的更新期视口保持分支不再执行；替代控制器却只覆盖后到达的分页 plan 事务，
没有覆盖普通输入事务。因而原“分页层独占全部滚动补偿”的控制权设计不完整，本轮改为按事务类型分权：

- 浏览器 Scroll Anchoring 只在 `.editor-paper` 容器禁用；删除 ProseMirror DOM 内联禁用，恢复其原生
  更新期 preserve 与 `scrollIntoView`。普通输入、换行和光标越界只由 ProseMirror 处理。
- 分页 Decoration 的身份只由 break index/页码决定；非末页 fill 与末页 fill 都改为稳定 DOM 上的 CSS
  变量。fill 变化不能再销毁/重建同一分页缝 widget。
- `docChanged` 只标记分页失效并启动 450ms 静默窗口；连续输入与 IME composition 期间保持旧 plan，
  不逐字符重新分配页面。首次装载、纸张/字体/媒体/窗口变化仍走独立即时结算。
- 每次分页结算提交前捕获可见锚点；提交内先让 ProseMirror 完成 Decoration/Selection/preserve，返回后只对
  剩余屏幕 Y 残差补偿一次。普通输入和显式导航不进入该补偿路径。
- `computePageBreaks` 的 fill 保留小数；仅比较时使用亚像素容差，避免每页取整误差向后累计。
- BDocx 的 `loadedContent` 不再直接代表可见就绪。`EditorContent` 保持挂载但在 `setContent` 和首轮分页
  settled 之前隐藏，由加载骨架遮盖；settled 结果必须属于当前 `loadSeq`。

预期契约是“编辑稳定优先于输入期间分页即时真实性”：输入期间页面断点可短暂滞后，静默后恢复真实分页；
该滞后只存在于呈现层，不进入 LogicalState、保存内容或脏态。

## 17. 本轮实现与验证记录（2026-07-26）

- 删除 ProseMirror DOM 的内联 `overflow-anchor:none`，保留 `.editor-paper` 容器级禁用；普通输入与
  `scrollIntoView` 恢复由 ProseMirror 原生更新期滚动保持负责。
- 分页插件新增输入事务 450ms 静默结算、IME 暂停、外部布局立即结算与 epoch 淘汰；删除以整个
  ProseMirror 高度变化反向触发重测量的路径。
- 分页断点 Decoration 身份与 fill 高度分离：fill/末页补白只更新稳定 DOM 的 CSS 变量，分页值保留小数，
  仅断点结构变化时重建 widget。
- 分页计划事务返回后按当前可见锚点残差至多修正一次，并钳制合法 `scrollTop`；普通输入和显式导航不进入
  该补偿路径。
- BDocx 载入增加 `loadSeq + pagination settled` 揭示门禁；`EditorContent` 隐藏但保持挂载测量，旧载入的
  settled 事务不能揭示当前文档，滚动记忆延后到首轮 settled 后恢复。
- 自动验证：分页/锚定/揭示门禁定向测试 33 项通过，`tsc --noEmit` 通过，Vite 生产构建通过。全量前端测试
  323/327 通过，剩余 4 项均为既有 `src/services/edLocate.test.ts` 选区偏移断言失败，不在本 Scope 内。
- 可见行为仍须在 DevTools 关闭的 Tauri 真机完成 BDocx 首/中/末页、50%/100% 缩放、底部连续输入、
  回车跨页、IME、列表/表格/图片与标签切换验收；在该验收完成前不得宣称视觉跳动已彻底消失。

## 18. 重新编译后的残余首输入单跳（2026-07-26）

发布构建复测推翻了上一轮可能闭合的判断：持续振荡不再出现，但“单击重新定位光标 → 第一个字符输入时
同步跳一下 → 后续连续输入稳定”可稳定复现；再次定位到其他可见位置后，首字符重新跳一次。光标位于文档
最下行时多次输入未观察到跳动。

该时序不符合分页结算路径：普通内容事务要在最后一次输入后静默 450ms 才会触发分页测量，而可见跳转与
第一个字符同步发生。当前安装的 ProseMirror 对普通文本输入事务调用 `scrollIntoView()`，且
`Editor.editorProps` 在控制权重构后没有 `handleScrollToSelection` 门控；因此本轮把残余范围锁定为
“可见光标的冗余选区滚动请求”。最下行不跳与滚动位置已受最大 `scrollTop` 钳制的表现一致，但该解释仍须
以发布构建真机验收确认，不据推断提前关闭 Issue。

## 19. 可见光标滚动预拦截（2026-07-26）

- `Editor` 新增分页专用 `handleScrollToSelection`：只处理 `.rm-paginated` 中已经揭示、非 IME composition
  的折叠文本光标；其它选择类型、不可测量状态与非分页文档全部返回 `false`，保留 ProseMirror 原生行为。
- 光标位置优先读取属于当前编辑器的原生 `Selection/Range`，不可读时按 ProseMirror position 建立 DOM
  Range；不以分页 widget 边界可能取错 side 的 `coordsAtPos` 作为主路径。
- 光标 Range 与 `.editor-paper` 的 client viewport 均使用屏幕 CSS px 比较，不混入 `scrollTop`，也不按
  内容 CSS zoom 做除法。光标完整可见（允许 1px 亚像素误差）时返回 `true`，在 ProseMirror 写
  `scrollTop` 前阻止冗余请求；任一方向真实越界时返回 `false`，保留正常跟随。
- 该门控不修改分页测量、450ms 输入静默、Decoration、Scroll Anchoring、分页结算锚点残差或模型/脏态。
  ProseMirror `storeScrollPos/resetScrollPos` 仍负责普通更新期保持，分页扩展仍只负责 plan 结算残差。
- 自动验证：新增 viewport/client rect、50% 等比屏幕坐标、边界容差与门控条件测试；相关 37 项通过，
  `tsc --noEmit` 与 Vite 生产构建通过。全量前端测试 328/332 通过，4 项失败仍为既有
  `src/services/edLocate.test.ts` 选区偏移断言。仓库统一 `npm run build` 另被当前 bundled LibreOffice
  版本校验不一致阻断，未在本 Scope 修改该独立运行时资源问题。
- 最终门禁仍是 DevTools 关闭的重新构建 Tauri 应用：50%/75%/100%，首/中/末页各重复执行“定位 → 首字
  → 连续输入 → 再定位 → 首字”，并覆盖真实越界、回车、IME 与 450ms 后分页结算。通过前 Issue 保持打开。

## 20. Range 门控真机失败与指针激活事务门控（2026-07-26）

上一节实现经真机复测仍然跳动，说明“当前 WebKit 能稳定提供与 `.editor-paper` client viewport 同坐标的
Selection/DOM Range”这一运行时假设不成立，或首字符处于 composition 路径而被保守分支放行。不得继续调大
rect 容差，也不得在跳动绘制后用下一帧回拉掩盖。

本轮增加一个与 Range/zoom 无关、作用域严格的一次性事务门控：

- `pointerup`/`mouseup` 在已揭示的分页编辑器内完成主键定位时，记录当时的 ProseMirror doc 引用与
  `.editor-paper.scrollTop`；编辑器 blur 时立即丢弃。
- 后续 `handleScrollToSelection` 只在当前 doc 引用已经变化（证明发生了第一次内容编辑），且滚动位置与
  指针定位时相差不超过 1px（证明用户期间没有主动滚动）时消费该记录并返回 `true`。
- 记录只消费一次；选择变化或显式导航若在 doc 未变化时先请求滚动，会消费并放行。后续连续输入、真实越界、
  用户主动滚动仍走原生路径。
- 该一次性分支在 composition 判断之前执行，因此覆盖中文输入法的首个文档变化；它不修改输入事务、选择、
  文档、分页计划或脏态。
- Range 可见性门控保留为非 composition 的一般冗余滚动保护；它不再是“指针定位 → 首输入”场景的唯一证据。

自动验证：相关测试 38/38 通过，`tsc --noEmit` 与 Vite 生产构建通过；全量前端测试 329/333，通过数增加
来自本轮新增测试，4 项失败仍全部是既有 `edLocate` 选区偏移断言。最终仍须重新构建后真机验证；若此分支下
仍发生同步跳动，即可否定 `scrollIntoView` 是当前可见写入者，下一轮必须转查 selection DOM 同步或浏览器
滚动，不再继续修改滚动门控。

## 21. 第二字符单跳与光标激活周期（2026-07-26）

真机复测得到稳定的新证据：“第一个字符不跳，第二个字符跳一次，后续输入不再跳”。这证明上一节一次性
门控确实进入 `handleScrollToSelection` 并阻止了原生滚动，但记录被消费后，第二个字符重新进入同一原生
`scrollIntoView` 归位；归位完成后，后续光标已在 ProseMirror 判断的安全区，因此不再移动。问题不是新写入者，
而是原错误归位被精确延迟一个字符。不得把计数从一次硬编码为两次或更多次。

本轮将一次性记录升级为光标激活周期：

- 主键指针定位时记录真实 `clientX/clientY`、当时 Range rect、当前 doc 引用与 `scrollTop`。
- 后续每次 doc 变化取得当前 Range rect；以激活 Range 为原点，只计算相对位移，再叠加到可信的指针屏幕点。
  绝对 Range 坐标即使受 WKWebView/CSS zoom 影响也不参与可见性判断。
- 运行时用 `computed zoom ÷ (ProseMirror rect宽 / offsetWidth)` 得到相对位移比例：浏览器rect已含zoom时
  比例为1，WKWebView返回未缩放rect时比例为zoom，避免重复换算。
- 估算光标仍完整位于 `.editor-paper` client viewport 且 `scrollTop` 与激活时相差不超过1px时，持续返回
  `true`；每次成功后只推进“上次已处理doc”，不结束激活周期，因此第二、第三及后续同一区域输入均被覆盖。
- 当前请求未发生doc变化（选择导航）、估算光标真实越界、用户滚动、blur或新指针定位时结束周期并返回
  ProseMirror原生滚动。composition同样先经过该周期，不再把第二字符无条件放行。
- 原生Selection不可读时仍可降级为按PM position取得的rect，但该rect只用于相对差，不作为绝对可见坐标。

自动验证：新增相对位移、50%未缩放Range映射、连续多次doc变化、用户滚动和自然越界测试；相关39项通过，
`tsc --noEmit` 与Vite生产构建通过。全量前端测试330/334，4项失败仍全部是既有`edLocate`选区偏移断言。
最终仍须DevTools关闭的重新构建Tauri应用验证连续输入、换行自然跟随、IME及50%/75%/100%。

## 22. 闭合前隐患收口（2026-07-26）

人类确认当前版本已满足“可编辑、无持续闪跳”的发布需求。闭合审计仍发现三个低频交叉风险：分页结构在
450ms静默结算后主动补偿`scrollTop`会被激活周期误判为用户滚动；周期只依赖blur释放，可能跨文档身份残留；
编辑器DOM的`pointerup/mouseup`自定义处理早于ProseMirror在root完成的最终选区同步，双击或拖选可能用旧折叠
光标误建周期。本轮在不改变分页算法的前提下收口：

- 分页层仅在实际写入结构补偿`scrollTop`后发出内部校正事件；Editor收到后只更新现有激活周期的滚动基线，
  不把内部补偿当成用户滚动。真实用户滚动仍由基线差异释放周期。
- 周期显式绑定当前`openPath/loadSeq/editFormat/contentFormat/zoom/纸张/方向`；任一身份或几何变化及组件
  卸载时取消待建帧并删除现有周期。纯选区事务同样释放现有周期，避免键盘或程序化导航后沿用旧锚。
- 指针事件不再同步建周期，而是在下一帧等待原生selectionchange与ProseMirror最终选区同步；只有主键单击
  形成的折叠TextSelection仍在已揭示分页viewport内时才建立。双击、拖选、NodeSelection和失焦均不建立。
- 分页内部事件只同步瞬态滚动基线，不修改ProseMirror doc、selection、分页plan、脏态或保存内容。

自动验证新增单击/双击/非折叠选区建周期判定及内部补偿后滚动基线继续保护场景；相关40项、
`tsc --noEmit`与Vite生产构建通过。全量前端测试331/335，4项失败仍全部是既有`edLocate`选区偏移断言；
统一`npm run build`仍被独立的bundled LibreOffice版本校验不一致阻断。最终闭合仍以重新构建Tauri应用执行
“输入→停顿超过450ms→继续输入”、双击选区后输入、切换文档后立即输入、真实越界/IME以及
50%/75%/100%为准。

## 23. 当前权威实现口径与历史段落索引（2026-07-26）

本Issue保留多轮失败方案作为执行追溯，不应把每一节都视为当前并行生效：

- 当前分页重测量与滚动分权以§16–17为准：`.editor-paper`禁用浏览器Scroll Anchoring，ProseMirror DOM
  保留原生preserve；输入静默450ms后结算，分页结构事务只补偿残差。
- 当前普通输入门控以§21–22为准：一般Range门控只处理非composition可见光标；有效指针激活周期先于
  composition保守分支，可覆盖IME的doc变化；分页内部补偿同步周期基线，身份/几何变化显式清理。
- §13“相交即接管”、§14“分页层独占滚动且ProseMirror DOM内联禁用”、§19“所有IME均不接管”和
  §20“一次性首字符门控”均为已被后续真机证据替代的历史实现，不是当前契约。
- 当前分页断点仍为块级：顶层块边界，列表可到`listItem`边界；不支持普通段落等节点的块内行盒拆分。
  该能力缺口已登记到ED技术设计§11.2与专项计划§7，另开分页算法任务处理，不在本闪跳Scope内伪装闭合。

## 24. 课题闭合与后续迁移（2026-07-26）

人类依据发布构建手工测试确认：当前版本已达到“可编辑、无持续闪跳”的本版本需求，并明确要求闭合本课题。
本 Issue 因此以 `READY/CLOSED` 结束；Issue Trace 继续保持 A 状态作为历史执行记录。

闭合边界：

- 本课题解决并收口分页打开振荡、输入逐字闪跳、内容回填控制链和低频激活周期交叉风险。
- 2026-07-26 重新运行定向测试 40/40、TypeScript 与 Vite 生产构建，均通过。
- `governance:generate` 确认 ADUS 无变化，`governance:audit` 与 `git diff --check` 均通过。
- 人类真机验收优先于历史段落中的“待验收”表述；§23 仍是最终权威实现口径。
- 统一 `npm run build` 的 bundled LibreOffice 校验不一致属于独立 DOC 构建课题，不改变本 ED Scope 的闭合。
- Block 内分页拆分不是本 Issue 的未完成修复，已迁移至
  `.governance/issues/PAGINATION-INTRABLOCK-SPLIT-01.md`；新课题必须把本 Issue 的滚动稳定性结果作为
  硬回归门槛，不得通过重新引入逐字跳动换取页面填充。
- 后继 `PAGINATION-INTRABLOCK-SPLIT-01` 已于 2026-07-26 由人类接受当前发布边界并闭合；整个分页课题
  不再有待迁移的开放 Issue。其普通段落行中强制断开与复杂节点原子降级限制以该 Issue §17 为准。

最终状态：

```text
task_type: CODE_CHANGE
governance_phase: VALIDATION
final_state: READY
issue_trace: SCROLL-JUMP-PAGINATION-01
scope_status: CLOSED
successor_issue: PAGINATION-INTRABLOCK-SPLIT-01
successor_status: CLOSED
human_acceptance: YES
```
