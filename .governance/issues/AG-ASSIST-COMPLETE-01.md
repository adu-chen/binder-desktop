# Issue Trace · AG-ASSIST-COMPLETE-01 辅助续写与局部修改完整性修复

承接 REQ-AG-005 / BR-AG-STATE-003 / BR-AG-VERIFY-001 / SM-AG-03 / A-ED-I-P-01 `ed.apply_edit`。
人类确认需求口径：辅助续写返回 3 个候选；新请求覆盖旧请求；局部修改无选区时按光标插入；多轮 refine
只传上一轮结果；候选以编辑器 decoration/overlay 语义挂载；每次真实模型调用前均经服务端或网关 gate。

## 1. 用户可见症状

辅助续写与局部修改当前实现不完整：辅助续写只按单候选填充，局部修改空选区不触发，旧请求返回可能覆盖新状态，
采纳路径直接 TipTap 插入而非 ED 受控编辑接口，后端 assist command 使用本地 `GateState::Passed` 假通过。

## 2. 入口点

- `Cmd/Ctrl+J` 辅助续写：`src/components/ed/Editor.tsx` `handleKeyDown`
- `Cmd/Ctrl+K` 局部修改：`src/components/ed/Editor.tsx` `handleKeyDown`
- 前端请求：`src/services/agService.ts` `requestAssist`
- 后端命令：`src-tauri/src/lib.rs` `ag_request_assist`

## 3. 相关规则

- REQ-AG-005
- BR-AG-STATE-003
- BR-AG-VERIFY-001
- SM-AG-03
- ED `ed.apply_edit` / `applyEditOps` 契约

## 4. 相关链路

`AG-ASSIST -> ED-EDIT -> CLOUD-GATE/CLOUD-INVOKE`

## 5. 生产者块

- `BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-001` 位于 `src-tauri/src/lib.rs`
- `requestAssist` 位于 `src/services/agService.ts`
- `assistStore` 位于 `src/store/assistStore.ts`

## 6. 消费者块

- `BR-AG-STATE-003-RB-AG-AG-ASSIST-002` 位于 `src/components/ed/Editor.tsx`
- `AutocompleteOverlay` 位于 `src/components/ag/AutocompleteOverlay.tsx`
- `InlineAssistPopover` 位于 `src/components/ag/InlineAssistPopover.tsx`
- `applyEditOps` 位于 `src/services/edLocate.ts`，本任务只消费不修改

## 7. 路径上的空规则块

本次不新增业务 `rules: []` 块。测试文件按 ADU §5.6 使用 `// covers:` 注释，不需要 @GOV。

## 8. boundary 假设

- `Editor.tsx` 当前 boundary 声明辅助采纳经 ED 受控接口，但实现直接 `insertContentAt`，假设不成立。
- `lib.rs` assist boundary 声明 gate-gated，但实现本地构造 `GateState::Passed`，假设不成立。
- `assist.rs` 状态机关于 gate denied/request fail 静默与 anchorLost/dismiss 的假设仍成立，但缺少前端投影闭环。

## 9. 运行时假设

- 每次请求必须有 requestId/latest-wins 语义，旧响应不可展示或应用。
- 每次候选展示和采纳前必须校验 doc/block/字符锚仍有效。
- 每次真实模型请求必须先经服务端或网关 gate，客户端不得缓存 gate pass。
- assist 接受作为用户编辑进入 ED，不进入 AgentTurn，不生成 DiffCard。

## 10. 疑似不变量破坏

- 当前 `ag_request_assist` 违反 BR-AG-VERIFY-001 的真实 gate 前置。
- 当前 `Editor.tsx` 违反 BR-AG-STATE-003 / ED 接口契约的受控写入语义。
- 当前局部修改不支持光标插入，违反已确认 REQ-AG-005。

## 11. 验证计划

- 前端测试：assist store latest-wins、requestAssist schema、Autocomplete 3 候选、InlineAssist refine 上轮 base。
- 后端测试：SM-AG-03 仍闭合；gateway gate check verdict；assist invalid kind 不误走 autocomplete。
- 验证命令：`npm test -- --run`、`./node_modules/.bin/tsc --noEmit`、相关 `cargo test`、`npm run governance:generate`、`npm run governance:audit`。

## 12. 2026-07-16 续修审计结论

上一轮已修复基础链路（3 候选字段、真实 gate、ED 受控写入、requestId/latest-wins），但仍存在以下未闭合点：

- **防抖/防重未真实实现**：`Cmd/Ctrl+J` 仍可逐次发起 `ag_request_assist`，latest-wins 只能丢弃旧响应，不能减少重复门禁/模型调用。
- **光标锚点校验不足**：cursor 目标仅校验 blockId 存在与 offset 未越界，未校验触发时前后文指纹；文档改动后仍可能在旧 offset 插入。
- **SM-AG-03 终态未闭环**：前端采纳、关闭、锚失效仅本地 close，未向后端广播 Accepted/Dismissed/AnchorLost 终态。
- **候选数量不是严格契约**：后端解析为最多 3 条，1 条/2 条仍可能展示；这与“辅助续写呈现 3 候选”不一致。
- **键盘确认范围过宽**：Autocomplete overlay 使用 window 级 keydown，未限定焦点仍在编辑器/overlay 范围，也未处理输入法 composition。
- **overlay 尺寸约束不稳定**：定位估算 420px，但 CSS 只有 min-width，长候选可能撑开后超出纸张钳制区域。

本轮修复必须先同步文档，再实现代码；不得出现实现领先文档。

## 13. 本轮补充修复设计

- `Cmd/Ctrl+J` 使用前端 leading-edge debounce：默认 450ms 内重复触发静默忽略，不发起新请求；窗口外触发不处理。
- 450ms 后再次触发视为刷新候选：重新捕获当前 DocRef/blockId/字符锚/前后文指纹，生成新 requestId；旧响应不得展示或应用。
- 辅助续写采纳位置固定为**触发瞬间捕获的字符锚**，不跟随采纳时光标；采纳前必须校验 docPath/loadSeq/blockId/range 与前后文指纹。
- autocomplete 候选必须严格为 3 条才允许采纳；不足 3 条视为 RequestFail，不展示半候选、不允许采纳，
  但保留轻量空态，避免触发后浮层瞬间消失。
- overlay 键盘采纳仅在焦点仍处于编辑器宿主或 overlay 内时生效；composition 中不采纳。
- 前端通过 `ag_finish_assist` 向 SM-AG-03 广播 `accept` / `dismiss` / `anchor_lost` 终态；该命令只记录状态机终态，不修改文档。

## 14. 2026-07-16 提示词拆分续修

审计发现：基础链路修复后，辅助续写与局部修改仍共用 `SYSTEM_PROMPT_ASSIST`，只靠 user prompt 分支区分。
这会让辅助续写的 JSON 数组契约与局部修改的单正文契约互相污染，也不符合 AI 编辑三层定义。

本轮补充设计：

- `SYSTEM_PROMPT_AUTOCOMPLETE`：只服务辅助续写，强制 JSON 字符串数组，且正好 3 项；不允许改写、解释、markdown。
- `SYSTEM_PROMPT_INLINE_ASSIST`：只服务局部修改，支持选区改写、光标插入、多轮 refine，只返回单一候选正文；不允许 JSON 数组。
- `SYSTEM_PROMPT_CHAT`：继续服务 Agent 编辑，走工具调用 / `propose_edit` / DiffCard 裁决链路。
- `ag_request_assist` 必须按 `kind` 选择对应 system prompt；不得再使用共用 `SYSTEM_PROMPT_ASSIST`。

验证留痕：

- `cargo fmt --manifest-path src-tauri/Cargo.toml`：通过。
- `cargo test --manifest-path src-tauri/Cargo.toml ag::prompts`：5 passed。
- `cargo test --manifest-path src-tauri/Cargo.toml ag::assist`：3 passed。
- `./node_modules/.bin/tsc --noEmit`：通过。
- `npm test -- --run src/store/assistStore.test.ts src/services/agService.test.ts src/components/ag/AutocompleteOverlay.test.tsx`：17 passed。
- `npm run governance:generate`：ADUS 内容无变化。
- `npm run governance:audit`：passed（仅既有 TERM_ORPHAN / BLOCK_UNANNOTATED 警告清单）。
- `git diff --check`（本 Scope 相关文件）：通过。
- 普通文档与运行时代码中已无 `SYSTEM_PROMPT_ASSIST` 引用。

## 15. 2026-07-16 浮层定位续修

审计发现：辅助续写词条与局部修改弹窗的逻辑写入锚点已经稳定，但视觉浮层仍用同一套
`coordsAtPos` + 固定尺寸 + 纸张可视区钳制算法。该算法会把浮层推到编辑器窗口边缘，且局部修改有选区时
只取选区结束点，导致弹窗与真实选区/插入点关联感弱。

本轮补充设计：

- 继续保持逻辑锚点唯一：采纳写入只使用触发瞬间捕获的 DocRef/blockId/字符锚和前后文指纹。
- 新增视觉锚点：辅助续写使用光标矩形；局部修改有选区时使用选区 DOM 矩形，无选区时使用光标矩形。
- 新增放置规则：辅助续写优先 `bottom-start`，局部修改优先 `top-center`；空间不足上下翻转，横向只在编辑纸张内容区内做最小偏移，不再纯粹钳制到编辑器窗口边缘。
- 浮层尺寸需稳定：辅助续写/局部修改均使用受控宽度与最大高度，避免生成内容撑开后破坏定位。

验证留痕：

- `npm test -- --run src/components/ed/assistPlacement.test.ts src/components/ag/AutocompleteOverlay.test.tsx src/store/assistStore.test.ts src/services/agService.test.ts`：21 passed。
- `./node_modules/.bin/tsc --noEmit`：通过。
- `npm run build`：通过（Vite 仅提示 chunk size warning）。
- `git diff --check`（本 Scope 相关文件）：通过。
- `npm run governance:generate`：ADUS 已刷新，@GOV 块 144。
- `npm run governance:audit`：passed（仅既有 TERM_ORPHAN / BLOCK_UNANNOTATED 警告清单）。

## 16. 2026-07-16 右侧边界遮挡续修

审计发现：浮层当前视觉边界来自编辑纸张/ProseMirror 内容区，未把 editor-host/center 可见右边界作为最终
硬边界复核；同时辅助续写默认 `bottom-start`，只有即将溢出时才切换 `end`，在靠近右侧代理编辑面板时
容易被 center overflow 或右侧面板裁切。

本轮补充设计：

- `assistBoundaryRect()` 必须将纸张滚动视口、ProseMirror 内容区、editor-host 可见区求交集，host 右边界为硬边界。
- `computeAssistPlacement()` 对辅助续写靠右场景提前切换 `end` 对齐，不等到完全越界后才修正。
- 增加靠右边界单元测试：浮层右边必须小于等于 boundary.right - padding。

验证留痕：

- `npm test -- --run src/components/ed/assistPlacement.test.ts src/components/ag/AutocompleteOverlay.test.tsx src/store/assistStore.test.ts src/services/agService.test.ts`：23 passed。
- `./node_modules/.bin/tsc --noEmit`：通过。
- `npm run build`：通过（Vite 仅提示 chunk size warning）。
- `git diff --check`（本 Scope 相关文件）：通过。
- `npm run governance:generate`：ADUS 已刷新，@GOV 块 144。
- `npm run governance:audit`：passed（仅既有 TERM_ORPHAN / BLOCK_UNANNOTATED 警告清单）。

## 17. 2026-07-16 交互锚失效续修

审计发现：辅助续写与局部修改已绑定触发瞬间逻辑锚点，但浮层生命周期只覆盖内容变更、显式关闭、
采纳和结构性锚失效；用户点击编辑器其他位置、点击应用其他区域、键盘移动光标或改变选区时，
文档内容可能未变，因此旧浮层仍可存活，异步返回也可能继续展示。这是交互锚生命周期漏项，
不应通过扩大 `anchor_lost` 语义掩盖。

本轮补充设计：

- 辅助续写与局部修改共享交互锚生命周期：点击浮层外、移动光标或改变选区即视为用户放弃。
- 用户放弃必须关闭浮层并丢弃已返回/后续返回内容，不展示、不应用、不入历史。
- 用户放弃回写 `ag.finish_assist dismiss`；只有 blockId/range/前后文指纹结构失效才回写 `anchor_lost`。
- 前端统一监听外部 pointerdown 与 ProseMirror selectionUpdate；点击 `.assist-pop` / `.assist-ac` 内部不关闭，
  保证局部修改按钮和辅助续写候选仍可正常采纳。

验证留痕：

- `./node_modules/.bin/tsc --noEmit`：通过。
- `npm test -- --run src/store/assistStore.test.ts src/services/agService.test.ts src/components/ag/AutocompleteOverlay.test.tsx src/components/ed/assistPlacement.test.ts`：24 passed。
- `npm run governance:generate`：ADUS 已刷新，@GOV 块 145。
- `npm run governance:audit`：passed（仅既有 TERM_ORPHAN / BLOCK_UNANNOTATED 警告清单）。
- AI 语义审计：`BR-AG-STATE-003-RB-AG-AG-ASSIST-007` 对应交互锚变化关闭/丢弃，`BR-AG-STATE-003-RB-AG-AG-ASSIST-002`
  对应采纳经 ED 受控编辑接口；与 BR-AG-STATE-003 规则文本一致，未发现术语漂移。
- `git diff --check`：通过。

## 18. 2026-07-16 缩放定位续修

审计发现：辅助续写和局部修改浮层的视觉锚点来自缩放后的光标/选区 DOM 位置，但浮层布局边界同时
intersect 了缩放后的 ProseMirror 正文矩形。编辑器缩放到 50% 时，正文矩形变窄，`computeAssistPlacement`
把浮层宽高也压缩到缩放后的正文边界内，导致控件变形；同一边界还会把浮层从真实锚点推开，造成位置不稳定。

本轮补充设计：

- 视觉锚点仍使用缩放后的光标/选区 DOM rect，确保位置跟随用户看到的插入点。
- 浮层布局边界改为 editor-paper 可视区与 editor-host 的交集，不再使用缩放后的 ProseMirror content rect
  决定浮层尺寸。
- 浮层尺寸采用 preferred/min 模型：辅助续写 preferred=420px/min=320px，局部修改 preferred=360px/min=300px；
  只有宿主真实可用宽度不足时才压缩。
- 滚动、缩放、窗口 resize 后，在逻辑锚仍有效时重算视觉锚；结构锚失效才关闭并回写 `anchor_lost`。

验证留痕：

- `./node_modules/.bin/tsc --noEmit`：通过。
- `npm test -- --run src/components/ed/assistPlacement.test.ts src/store/assistStore.test.ts src/services/agService.test.ts src/components/ag/AutocompleteOverlay.test.tsx`：27 passed。
- `npm run governance:generate`：ADUS 已刷新，@GOV 块 145。
- `npm run governance:audit`：passed（仅既有 TERM_ORPHAN / BLOCK_UNANNOTATED 警告清单）。
- AI 语义审计：`BR-AG-STATE-003-RB-AG-AG-ASSIST-003/004/005/006/007` 均仍对应 AG-ASSIST
  视觉锚、布局边界、交互锚生命周期；本轮缩放修复扩展的是既有视觉放置规则，未发现术语漂移。

## 19. 2026-07-16 稳定定位续修

审计发现：局部修改浮层初始态真实 DOM 高度远小于 `ASSIST_FLOATING_SIZE.inline.height=260`，但
`computeAssistPlacement` 在 top-placement 中长期用预估高度计算 `anchorRect.top - height - gap`。
结果是弹窗顶边被放到过高位置，真实弹窗又按内容自适应高度渲染，视觉上表现为“出现在光标上方很远”。
该问题不是模型调用慢，也不是安全区本身造成；安全区只负责边界 clamp，主因是定位输入高度与真实渲染高度脱节。

曾尝试方案与回退原因：

- 尝试使用 `useLayoutEffect + ResizeObserver` 由浮层组件上报真实尺寸，再由 `Editor.tsx` 重算锚点。
- 人类实测发现辅助续写浮层不再稳定弹出，说明该测量链对当前交互过重：首帧渲染、测量、store 更新、
  重新渲染之间存在不必要的反馈环，稳定性低于收益。

最终修复设计：

- 撤销浮层组件的运行时 DOM 测量与 `onMeasure` 回调，避免测量链反向驱动 assist 状态。
- 定位链回到单一纯函数：`Editor.tsx` 捕获视觉锚点，`computeAssistPlacement` 使用静态尺寸、边界和翻转规则返回位置。
- 将局部修改静态高度从过大的 260 调整为接近初始输入态的 112，避免 top-placement 远离锚点。
- 将辅助续写静态高度收敛为 156，保留生成中浮层立即显示与三候选态的可用空间。
- 保留滚动、缩放、窗口 resize 的简单重定位；不追求 preview/候选内容变化后的持续精细跟随。

追加修复：人类实测辅助续写浮层仍无法弹出。排查确认终端中的 LibreOffice 字体回退与 macOS
CapsLock 日志和 AG-ASSIST UI 无关；前端链路存在更直接问题：`openAutocompleteAssist` 打开生成中浮层后，
若 `requestAssist` 很快返回空结果、门禁失败或不足 3 条候选，代码立即 `assistStore.close()`，用户体感为
快捷键无效/浮层从未出现。最终策略改为：候选不足时调用 `setGenerating(false)` 保留轻量空态，不展示半候选、
不允许采纳，由用户点击外部、移动光标或 Esc 关闭。

验证留痕：

- `./node_modules/.bin/tsc --noEmit`：通过。
- `npm test -- --run src/components/ed/assistPlacement.test.ts src/store/assistStore.test.ts src/services/agService.test.ts src/components/ag/AutocompleteOverlay.test.tsx`：30 passed。
- `npm run governance:generate`：ADUS 已刷新，@GOV 块 145。
- `npm run governance:audit`：passed（仅既有 TERM_ORPHAN / BLOCK_UNANNOTATED 警告清单）。
- `git diff --check`：通过。
- AI 语义审计：`BR-AG-STATE-003-RB-AG-AG-ASSIST-003/004/005/006/007` 仍对应 AG-ASSIST 视觉锚、
  可视边界、交互锚生命周期；本轮改为静态尺寸输入，不改变逻辑锚、采纳链路或状态机归属。

## 20. 2026-07-17 辅助续写候选链路加固

人类复核指出：辅助续写曾经可正常返回三条候选，当前出现 `暂无可用续写`，不应继续按 UI 问题盲修。
只读审计确认最后一轮 UI 修改只是把失败显性化；更早的完整化修复同时引入了三类高风险改动：

- `ag_request_assist` 从本地 `GateState::Passed` 改为每次真实 `check_gate()`，gate 失败会静默空候选。
- `SYSTEM_PROMPT_ASSIST` 被拆分为 `SYSTEM_PROMPT_AUTOCOMPLETE`，辅助续写从“候选正文”升级为“严格 JSON 三候选”。
- 后端解析后若不是正好 3 条就直接 RequestFail；解析器只覆盖纯 JSON 数组和简单编号行，对 fenced/多行 JSON、
  `{ candidates: [...] }` 包装等真实模型常见输出格式容错不足。

本轮修复设计：

- 保持产品契约：Autocomplete 发送前只展示 3 条候选；不足 3 条不展示半候选、不允许采纳。
- 加固解析器：兼容纯 JSON 数组、fenced/多行 JSON、`{ candidates: [...] }` 包装、编号/项目符号文本；去空、
  去重、超过 3 条取前 3 条。
- 增加内部 `failure_reason`：`gate_denied`、`gate_offline`、`gateway_error`、`empty_output`、
  `parse_underflow`、`stale_request`；前端可接收用于诊断/测试，UI 仍统一轻量空态。
- 增加隐私安全诊断日志：只记录 assist_id、kind、gate 结果、错误码、模型输出字符数、解析候选数，不打印完整正文。
- 提示词继续要求严格 JSON 三候选，并明确不要 markdown fenced block、不要对象包装、不要解释文字，降低偏航概率；
  解析容错不是放宽产品要求，而是承接模型输出不稳定性。
- 人类复测仍显示 `暂无可用续写` 后，直连 dev gateway 复核：`/health` 正常、`POST /auth/login` 正常、
  `POST /gate/check` 返回 passed、`POST /invoke/stream` 可返回三候选；该次真实返回形态为
  `{ "candidates": [...] }` 包装对象，已被本轮解析器覆盖。
- 同步加固 `check_gate()`：缓存 token 若在 `/gate/check` 返回 401 或 `reauth_required=true`，立即清除并重新登录重试一次；
  该重试仍走服务端裁决，不引入本地 pass 缓存，避免旧 token 让辅助续写首轮请求静默空态。

验证留痕：

- `cargo fmt --manifest-path src-tauri/Cargo.toml`：通过。
- `./node_modules/.bin/tsc --noEmit`：通过。
- `cargo test --manifest-path src-tauri/Cargo.toml ent::gateway`：4 passed。
- `cargo test --manifest-path src-tauri/Cargo.toml ag::assist`：5 passed。
- `cargo test --manifest-path src-tauri/Cargo.toml ag::prompts`：5 passed。
- `npm test -- --run src/components/ed/assistPlacement.test.ts src/store/assistStore.test.ts src/services/agService.test.ts src/components/ag/AutocompleteOverlay.test.tsx`：31 passed。
- `git diff --check`：通过。
- `npm run governance:generate`：ADUS 已刷新，@GOV 块 145。
- `npm run governance:audit`：passed（仅既有 TERM_ORPHAN / BLOCK_UNANNOTATED 警告清单）。

## 21. 2026-07-17 AG-ASSIST 规则映射链收敛与空态分型修复

人类复核指出：映射链外的文档描述、技术设计与代码实现均不能作为有效闭环依据。只读审计确认
AG-ASSIST 的 ADUS 映射只覆盖了后端 `ag_request_assist`、`ag::assist`、Editor 部分锚点/采纳与
assistPlacement；但真实运行链路还依赖 `assistStore`、`agService.requestAssist`、`AutocompleteOverlay`。
这些实现点此前只用普通注释或测试承接规则，导致 failure_reason 在 Editor 层被统一压成
`暂无可用续写`，无法区分 gate、网关、解析不足、旧请求、invoke 异常等原因。

本轮修复设计：

- 文档先行收敛唯一链：Editor 快捷键与锚点 → assistStore → requestAssist/finishAssist →
  `ag_request_assist` → 候选解析/failure_reason → overlay 呈现与采纳。
- 为 `assistStore`、`requestAssist`、`AutocompleteOverlay`、Editor 辅助续写结果处理补 AG-ASSIST
  @GOV 映射，避免链外实现继续承接核心状态。
- `assistStore` 新增 `autocompleteFailureReason` 与 `setAutocompleteFailure`，空态保留内部诊断原因；
  普通 UI 不弹错，overlay 通过 `data-failure-reason` 暴露测试/诊断点。
- `requestAssist` 不再把 invoke 异常吞成 `null`，而是返回空候选 + `failureReason=invoke_error`。
- SM-AG-03 清理状态分叉：候选不足不再走普通 `RequestFail→Idle`，改为独立
  `CandidateUnderflow→Presenting`，与前端轻量空态一致；普通 requestFail 仍表示 gate/网关/空输出静默回 Idle。

验证留痕：

- `./node_modules/.bin/tsc --noEmit`：通过。
- `npm test -- --run src/store/assistStore.test.ts src/services/agService.test.ts src/components/ag/AutocompleteOverlay.test.tsx src/components/ed/assistPlacement.test.ts`：32 passed。
- `cargo fmt --manifest-path src-tauri/Cargo.toml`：通过。
- `cargo test --manifest-path src-tauri/Cargo.toml ag::assist`：5 passed。
- `cargo test --manifest-path src-tauri/Cargo.toml ent::gateway`：4 passed。
- `npm run governance:generate`：ADUS 已刷新，@GOV 块 149（新增 assistStore / requestAssist / AutocompleteOverlay / Editor 结果处理映射）。
- `npm run governance:audit`：passed（仅既有 TERM_ORPHAN / BLOCK_UNANNOTATED 警告清单）。
- `git diff --check`：通过。

## 22. 2026-07-17 人类实测确认与链路残留清理

人类实测确认：辅助续写当前可以正常返回并展示 3 条续写建议，采纳/关闭等交互逻辑正常。
该结论用于固化 AG-ASSIST 运行链路终态，不替代自动化测试，但作为 UI 行为验收依据。

本轮残留清理：

- `requestAssist` 类型收敛为总是返回 `AssistResponse`；invoke 异常统一返回空候选 +
  `failureReason=invoke_error`，不再保留旧的 `null` 分叉。
- Editor 结果处理移除临时 `console.info("[ag-assist] autocomplete empty")` 诊断输出；诊断点保留在
  `assistStore.autocompleteFailureReason` 与 overlay `data-failure-reason`，作为测试/排障契约。
- `assistStore`、`AutocompleteOverlay`、`InlineAssistPopover` 顶部说明从早期参考稿/PI 编号改为
  AG-ASSIST / BR-AG-STATE-003 正式链路表述。
- `A-AG-X-L-01` 当前验收项中“候选不足静默关闭”改为“进入轻量空态且不展示半候选”，避免计划层继续指向旧失败态。
- 历史 Issue Trace 中关于多轮失败尝试、候选不足策略演进、网关/gate 排查的记录保留为追溯依据；
  当前有效设计以 §21 与本节为准。

验证留痕：

- `./node_modules/.bin/tsc --noEmit`：通过。
- `npm test -- --run src/store/assistStore.test.ts src/services/agService.test.ts src/components/ag/AutocompleteOverlay.test.tsx src/components/ed/assistPlacement.test.ts`：32 passed。
- `cargo test --manifest-path src-tauri/Cargo.toml ag::assist`：5 passed。
- `npm run governance:generate`：ADUS 已刷新，@GOV 块 149。
- `npm run governance:audit`：passed（仅既有 TERM_ORPHAN / BLOCK_UNANNOTATED 警告清单）。
- `git diff --check`：通过。

## Scope Lock

```text
task_type: CODE_CHANGE
scope_id: AG-ASSIST-COMPLETE-01
allowed_files:
  - .governance/active_scope.json
  - .governance/issues/AG-ASSIST-COMPLETE-01.md
  - docs/01_modules/AG/A-AG-M-D-01_代理功能需求描述.md
  - docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md
  - docs/01_modules/AG/A-AG-M-S-01_代理状态机设计.md
  - docs/01_modules/AG/A-AG-I-P-01_代理接口协议.md
  - docs/01_modules/AG/A-AG-M-T-02_提示词架构设计.md
  - docs/01_modules/SYS/A-SYS-C-T-02_前端交互架构与流程总图.md
  - docs/01_modules/AG/A-AG-X-L-01_代理开发计划.md
  - docs/01_modules/AG/A-AG-X-L-02_代理集成计划.md
  - src/store/assistStore.ts
  - src/store/assistStore.test.ts
  - src/components/ag/AutocompleteOverlay.tsx
  - src/components/ag/AutocompleteOverlay.test.tsx
  - src/components/ag/InlineAssistPopover.tsx
  - src/components/ed/assistPlacement.ts
  - src/components/ed/assistPlacement.test.ts
  - src/components/ed/Editor.tsx
  - src/styles.css
  - src/services/agService.ts
  - src/services/agService.test.ts
  - src-tauri/src/ag/assist.rs
  - src-tauri/src/ag/prompts.rs
  - src-tauri/src/ent/gateway.rs
  - src-tauri/src/lib.rs
  - docs/00_core/A-CORE-C-R-01_ADUS.md
forbidden_files:
  - ADU.md
  - ADP.md
  - AGENTS.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/src/ipc/mod.rs
confirmation: YES
scope_reason: AG-ASSIST-COMPLETE-01（人类确认续修）：闭合辅助续写/局部修改 AG-ASSIST 链路，先同步文档，再修防抖、防重、锚点指纹、严格 3 候选、终态回写与 overlay 键盘范围。
```
