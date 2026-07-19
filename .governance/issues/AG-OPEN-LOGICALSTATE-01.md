# Issue Trace · AG-OPEN-LOGICALSTATE-01

## 1. 用户可见症状

编辑器里的文档内容已经被修改，但代理回合仍基于旧文档内容回答或生成修改建议。截图中的表现是当前文档 Block 内容已变化，模型仍判断旧的 Block 2 / Block 4 重复，说明 AG 读链未感知已打开文档的最新 LogicalState。

## 2. 入口点

- 用户动作：在代理编辑面板发送消息，请求模型读取或修改文档。
- 前端入口：`src/services/agService.ts` `startTurn`
- 后端入口：`src-tauri/src/lib.rs` `ag_start_turn`
- 工具入口：`src-tauri/src/ag/tool_exec.rs` `exec_read_only`
- Diff 接受入口：`src/services/deService.ts` `acceptDiff`

## 3. 相关规则

- X-FORBID-008：三态单向数据流，LogicalState 修改必须经 ED 受控接口，LogicalState 写 DiskState 只能经保存链。
- X-FORBID-009：AI 产出在转入 LogicalState 前必须经受控裁决/应用入口，不得绕过 ED/DE。
- X-FORBID-005：上层模块不得绕过 ED 受控接口直接操纵编辑器内部文档状态。
- X-FORBID-006：AG 及工具调用不得直接写 workspace 文件内容。
- X-SHARED-001：跨模块共享结构必须由 SYS 数据结构契约定义。
- BR-AG-DATA-001：AG 回合上下文读取与注入契约。
- BR-AG-DATA-003：AG 修改目标集与编辑基准守卫。
- BR-AG-STATE-002：AG 工具调用注册/权限/执行循环约束。
- BR-DE-STATE-001：Diff 生命周期终态与裁决状态约束。
- BR-DE-DATA-002：DiffPayload 持久化与投影载荷契约。
- BR-ED-STATE-001：ED LogicalState 修改必须经受控编辑入口。
- BR-ED-DATA-004：Block / SelectionRef / CharAnchor 会话内锚语义。
- BR-SYS-DATA-002：跨模块数据结构契约。
- BR-SYS-PERSIST-001：workspace.db schema 版本化与迁移契约。

## 4. 相关链路

AG-TURN → AG-TOOL → DE-DIFF → ED-EDIT → WS-SAVE

辅助相关链路：

- REF-INJECT：ReferencePayload.doc / selection 注入。
- ED-DECOR：DiffCard / GreenReview 展示跟随 LogicalState。
- IPC-CALL：跨前后端命令与 Channel 入参结构。

## 5. 生产者块

- `BR-AG-DATA-001-QUERY-AG-AG-TURN-010, BR-DOC-DATA-003-QUERY-AG-AG-TURN-010` 位于 `src-tauri/src/ag/blocks.rs`
- `BR-AG-DATA-001-AG-AG-TURN-009` 位于 `src-tauri/src/lib.rs`
- `BR-ED-DATA-004-QUERY-ED-ED-EDIT-010` 位于 `src/services/edLocate.ts`
- `BR-ED-STATE-001-EFFECT-ED-ED-EDIT-012` 位于 `src/services/edLocate.ts`
- `BR-DE-STATE-001-RB-DE-DE-DIFF-002` 位于 `src/store/deStore.ts`
- `BR-DE-DATA-002-IO-DE-DE-DIFF-002` 位于 `src-tauri/src/de/store.rs`

## 6. 消费者块

- `BR-AG-DATA-001-DATA-AG-AG-TURN-001` 位于 `src-tauri/src/ag/context.rs`
- `BR-AG-STATE-002-RB-AG-AG-TOOL-001` 位于 `src-tauri/src/ag/tool.rs`
- `BR-AG-DATA-003-GUARD-DE-DE-DIFF-001` 位于 `src-tauri/src/de/generate.rs`
- `BR-DE-STATE-001-RB-DE-DE-DIFF-001` 位于 `src-tauri/src/de/mod.rs`
- `BR-DE-STATE-002-GUARD-DE-DE-DIFF-001` 位于 `src-tauri/src/de/expiry.rs`
- `BR-ED-STATE-001-RB-ED-ED-EDIT-002` 位于 `src/store/edStore.ts`
- `BR-ED-STATE-002-RB-ED-ED-DECOR-001` 位于 `src/components/ed/extensions/DiffDecorationExtension.ts`

## 7. 路径上的空规则块

未发现本次必须修改路径上已知 `rules: []` 空规则块。若实现中新增薄包装或工具函数且符合 ADU §4.3 不要求标注范围，应在 Task Runtime Report 记录豁免理由；若成为链路入口或业务可执行块，必须补 @GOV。

## 8. boundary 假设

- `read_editable_snapshot` 当前 boundary 声称 `workspaceDocOrActiveEditorSnapshot -> AgentEditableSnapshot`，但实际多数调用未传 active/open LogicalState，导致假设不成立。
- `format_reference_payload_context` 当前 boundary 声称输出 `readOnlySnapshot`，但未区分已打开文档 LogicalState 与未打开文档 DiskState。
- `exec_read_only` 当前模块注释声称文档内容读取统一走 AgentEditableSnapshot，但缺少 open session overlay，导致“统一投影”不等于“最新真源”。
- `deStore` 当前 boundary 假设 current editor 草稿路径才会进入 ED LogicalState，未覆盖 inactive open session 直接应用。

## 9. 运行时假设

- `caller_context`：AG turn 调用方必须提供当前打开文档的实时内容；本次扩展为提供所有已打开文档的 OpenDocSnapshot。
- `runtime_guard`：已打开文档存在 ED LogicalState 时，AG 读链必须拒绝退回 DiskState。
- `runtime_guard`：inactive open session diff 应用前必须校验 base hash / version 与当前 session 匹配。
- `runtime_guard`：active dirty 文档无法实时序列化时，必须阻断 AG turn，不能使用旧 loadedContent 或 DiskState 兜底。
- `runtime_guard`：selection 带锚时必须校验；无法校验时不得作为当前文档状态证据。

## 10. 疑似不变量破坏

- 可能破坏 X-FORBID-009：用户已确认 inactive open doc 可无额外操作直接应用 diff，因此需显式重写该约束语义，把“直接应用”定义为受控 ED LogicalState transaction，而不是 background accept 或直接写用户文件。
- 可能破坏 X-FORBID-008 / X-FORBID-006：如果 inactive open diff 直接走 `de.accept_diff -> ws.apply_diff`，会绕过 ED session 并写 DiskState；本次必须禁止该路径。
- 可能破坏 X-SHARED-001：OpenDocSnapshot、snapshotSource、applyMode、sessionVersion/contentHash 等字段必须进入 SYS 数据结构定义，不能由 AG/DE/ED 私自定义漂移结构。

## 11. 验证计划

- 文档：修改三态模型、AG/DE/ED/SYS 协议与状态机文档；如新增/修改 RULE/CONSTRAINT/STATE_MACHINE 标注，运行 `npm run governance:generate`。
- 前端测试：
  - `src/services/agService.test.ts`：startTurn 发送 active + inactive open snapshots；active dirty 序列化失败阻断。
  - `src/services/deService.test.ts`：open_session diff 自动应用后禁用裁决；background accept 不处理已打开文档。
  - `src/store/edStore.test.ts`：session version/hash 与 LogicalState 持久化投影。
  - `src/components/de/DiffCard.test.tsx`：applied_to_logical_state 禁用接受/拒绝。
  - `src/services/referencePayload.test.ts` / `src/services/edLocate.test.ts`：selection 校验与降级。
- 后端测试：
  - `src-tauri/src/ag/tool_exec.rs`：read_doc/list_blocks/search_text 对 open snapshot overlay 优先。
  - `src-tauri/src/ag/blocks.rs` 或新增 read context 测试：已打开文档用 LogicalState，未打开文档用 DiskState。
  - `src-tauri/src/lib.rs`：ReferencePayload.doc / legacy mention / propose_edit 使用统一 AgentReadContext。
  - `src-tauri/src/db/mod.rs` / DE store：必要 schema 迁移幂等。
- 验证命令：
  - `npm run test -- src/services/agService.test.ts src/services/deService.test.ts src/store/edStore.test.ts src/store/deStore.test.ts src/components/de/DiffCard.test.tsx src/services/referencePayload.test.ts src/services/edLocate.test.ts`
  - `npm run build`
  - `npm run governance:audit`
  - Rust 相关测试按实际模块执行。

## 文件范围控制

允许修改的文件：

- `.governance/active_scope.json`
- `.governance/issues/AG-OPEN-LOGICALSTATE-01.md`
- `docs/00_core/A-CORE-C-T-01_三态模型.md`
- `docs/00_core/A-CORE-C-R-01_ADUS.md`
- `docs/01_modules/AG/A-AG-I-P-01_代理接口协议.md`
- `docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md`
- `docs/01_modules/AG/A-AG-M-S-01_代理状态机设计.md`
- `docs/01_modules/DE/A-DE-I-P-01_DiffEdit接口协议.md`
- `docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md`
- `docs/01_modules/DE/A-DE-M-S-01_DiffEdit状态机设计.md`
- `docs/01_modules/ED/A-ED-I-P-01_编辑器接口协议.md`
- `docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md`
- `docs/01_modules/ED/A-ED-M-S-01_编辑器状态机设计.md`
- `docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md`
- `docs/01_modules/SYS/A-SYS-C-T-02_前端交互架构与流程总图.md`
- `docs/01_modules/SYS/A-SYS-M-S-01_状态机总图.md`
- `src-tauri/src/db/mod.rs`
- `src-tauri/src/ag/mod.rs`
- `src-tauri/src/ag/blocks.rs`
- `src-tauri/src/ag/tool_exec.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/de/store.rs`
- `src-tauri/src/workspace/diff_apply.rs`
- `src/store/edStore.ts`
- `src/store/deStore.ts`
- `src/services/agService.ts`
- `src/services/deService.ts`
- `src/services/edLocate.ts`
- `src/services/referencePayload.ts`
- `src/components/ed/Editor.tsx`
- `src/components/de/DiffCard.tsx`
- `src/store/edStore.test.ts`
- `src/store/deStore.test.ts`
- `src/services/agService.test.ts`
- `src/services/deService.test.ts`
- `src/services/edLocate.test.ts`
- `src/services/referencePayload.test.ts`
- `src/components/de/DiffCard.test.tsx`

禁止修改的文件：

- `ADU.md`
- `ADP.md`
- `AGENTS.md`
- `docs/00_core/A-CORE-C-P-01_APC.md`
- 不在允许列表内的文件默认禁止。

## 12. 本轮实施结果

- 前端 AgentTurn 发起前采集全部已打开文档 OpenDocSnapshot；active 文档必须来自实时编辑器序列化，失败即阻断回合，不再退回 `loadedContent` 或 DiskState。
- 后端建立 AgentReadContext；read_doc / search_text / list_blocks / ReferencePayload / propose_edit 均先读取已打开文档 LogicalState，未打开文档才读取 DiskState。
- DiffProjection 增加 `draft_mode`、`snapshot_source`、`base_session_version`、`base_content_hash`；已打开非激活文档生成 `open_session` diff。
- `open_session` diff 由前端 ED session 事务校验 version/hash 后直接写入目标 `EditSession.content`，递增版本、刷新 hash、置 dirty、持久化 `ed_session_snapshots`；成功后调用 `de.accept_draft_diff(open_session)`，DiffCard 只读禁用裁决。
- 自动应用失败或基线推进时调用 `de.expire_diff` 或投 error 卡；已打开文档的非草稿后台 accept 被拒绝，防止绕过 LogicalState。
- workspace.db 增加 `ed_session_snapshots` v7 迁移；workspace 恢复时恢复打开文档 session 镜像。

## 13. 本轮验证结果

- `npm test -- --run src/services/deService.test.ts src/services/agService.test.ts`：通过，30 tests。
- `npm run build`：通过。
- `cargo test --manifest-path src-tauri/Cargo.toml ag`：通过，含 AG read context、工具、引用相关测试。
- `cargo test --manifest-path src-tauri/Cargo.toml db::tests`：通过，含 v7 迁移重启恢复。
- `npm run governance:generate`：已刷新 ADUS。
- `npm run governance:audit`：通过；保留既有 TERM_ORPHAN / BLOCK_UNANNOTATED warning。
- `cargo test --manifest-path src-tauri/Cargo.toml`：181/182 通过；唯一失败为既有 `doc::libreoffice_runtime::tests::docx_to_pdf_end_to_end_when_libreoffice_available`，错误为 LibreOffice 预览转换失败，非本次 AG/DE/ED LogicalState 链路。

## 14. 遗留债务

- `open_session` 自动应用后 DiffCard 变只读，不支持撤销；其只读/失效/撤销视觉与文档级回滚能力仍需后续增强。普通 DiffCard 红删/绿增对比已由 DiffCard 承担，编辑器 GreenReview 不再额外渲染红删/绿增行。
- open_session StyleOp 自动应用未实现；当前策略为过期或错误，不进行半应用。
- 后端 `de.accept_draft_diff(open_session)` 已校验基线字段存在；实际内容基线匹配由前端 ED session 事务完成，后续可把 ED session 基线镜像校验下沉到后端事务。

## 15. 追加修复：active editor binding gap

### 根因

标签切换时 `edStore.openPath` 可先于 ProseMirror `setContent()` 完成而变更，导致短暂窗口内
`activePath=B` 但 active ProseMirror `doc` 仍承载 A。旧实现的 AG 快照采集只看 `openPath`
并直接序列化 active editor，可能把 A 的正文写入 B 的 OpenDocSnapshot，进而让模型把 B 当作 A 的内容修改。

### 决策

- 不把 TipTap 渲染完成作为 AG 读取 LogicalState 的必要条件。
- active ProseMirror 已绑定 `relPath/loadSeq/editFormat` 时，AG 读取 ProseMirror 实时序列化。
- active ProseMirror 尚未绑定当前 `openPath` 时，先把旧绑定文档的实时内容 flush 回其 `EditSession.content`；
  若当前目标 session content 已存在，AG 直接读取该 session LogicalState。
- 若当前目标 session content 尚未载入，则短暂等待；超时后阻断，不退回 DiskState 或旧快照。
- 保存与当前编辑器草稿应用必须等待 active ProseMirror 绑定一致，不能用 session fallback 替代。

### 实施结果

- `edLocate` 增加 active editor binding：记录 ProseMirror 当前承载的 `relPath/loadSeq/editFormat`。
- `Editor` 在 `setContent()` 完成后绑定 active editor；无文档或卸载时清空绑定。
- `agService.collectOpenDocSnapshots()` 增加错配处理：旧绑定文档先 flush，当前目标优先读可信 session content；
  session 尚未载入时等待后重试。
- `Editor` 保存与 pending diff 草稿应用增加绑定门禁，避免把旧 ProseMirror 内容保存或应用到新路径。

### 验证

- `src/services/agService.test.ts` 覆盖 active binding 未切到新文档时读取新文档 session LogicalState、回写旧绑定文档、
  新 session 尚未载入时等待后继续。
- `src/services/edLocate.test.ts` 覆盖 active editor binding 记录、匹配、卸载清空。

## 16. 二次修复：current doc source / empty blocks / identity drift

### 根因

- `collectOpenDocSnapshots()` 在 active ProseMirror binding 缺失时仍可把当前 active session 快照标记为
  `active_editor`，后端无法区分“实时编辑器序列化”与“当前 ED session LogicalState”。
- AgentTurn 历史保存多处在异步事件时读取 `edStore.openPath`，用户发送后切换标签会让本轮 `rel_path`
  漂移到后来的文档。
- HTML 分块只扫描 `data-block-id`，普通 HTML `<p>/<h1>` 文档会投影为空块；模型因此容易依赖历史上下文。
- `replace_document` 只检查 operation 显式性，缺少新旧全文身份守卫，无法拦截“农夫与蛇 → 小马过河”
  这类历史上下文污染导致的整篇错文档替换。

### 实施结果

- 新增 `active_editor_session` source：当前 active 文档 ProseMirror binding 未就绪但 ED session content 已存在时，
  AG 可读该 LogicalState；若 binding 完全缺失且目标仍是当前文档，则短暂等待后阻断，不退回旧快照。
- `startTurn()` 在发送瞬间冻结 `relPath/loadSeq/editFormat`，所有 `ag.save_turn` 与 `ag_start_turn`
  入参都使用冻结目标，异步期间切换文档不污染历史归属。
- 后端允许 `active_editor_session` 作为当前文档 LogicalState source，并在 DiffProjection 中原样返回。
- HTML block 解析扩展到常见块级标签；无 `data-block-id` 时保留 `block_index`，`replace_block`
  可按 index 原位替换并保留标签。
- 编辑请求若当前文档非空但 `blocks[]` 为空，后端在模型调用前返回 `AG_ACTIVE_BLOCKS_REQUIRED`。
- 既有文档 `replace_document` 增加主题/实体身份守卫；新旧全文重合度极低时返回
  `AG_REPLACE_DOCUMENT_IDENTITY_DRIFT`，不生成 diff。

### 验证新增

- 前端：`agService.test.ts` 覆盖 active binding 缺失阻断、`active_editor_session` 投影保留、
  回合 `relPath` 冻结；`edStore.test.ts` 覆盖 active session source 投影。
- 后端：`ag::blocks` 覆盖无 block id HTML 分块、按 index 替换、无关故事整篇替换拒绝、同主题润色放行。

### 边界

- `active_editor_session` 不是 DisplayState 反向定义 LogicalState；它只表示 ED session 已有可读 LogicalState，
  TipTap 渲染/绑定尚未完成。
- 身份守卫是防历史污染的硬安全线，不替代用户明确跨主题重写的产品交互；若后续要支持“整篇改成全新主题”，
  需要单独设计确认协议，而不能让模型自行绕过。

## 17. 三次修复：Prompt/History 映射链闭合

### 根因

- `OpenDocSnapshot -> AgentReadContext -> propose_edit` 已经接入 LogicalState，但 Prompt 装配层仍把旧会话历史作为裸 `role/content`
  消息注入，没有携带 `relPath/turnId` 归属；模型无法区分“小马过河”的旧文档历史与“农夫与蛇”的当前文档事实。
- `decide_intent_carryover()` 把“再短一点 / 改成 / 重写”等泛编辑词在有历史时直接判为沿用上一轮，违反
  BR-AG-DATA-004“默认无沿用，显式历史回指才提权”的规则。
- 当前文档 P0 仍以 `DOC_CTX_CAP=9000` 静默截前部。长文档尾部和结构被截断后，历史消息更容易补位成主事实。
- 实际消息装配顺序在 `ag_start_turn` 内联完成，ADUS 只能映射 `context.rs` 辅助函数，无法覆盖“历史裸注入 + 当前文档截断”
  这个真实业务链断点。

### 本轮补充 Scope

- `docs/01_modules/AG/A-AG-M-T-02_提示词架构设计.md`
- `src-tauri/src/ag/context.rs`
- `src-tauri/src/ag/history.rs`
- `src/store/agStore.ts`
- `src/store/agStore.test.ts`

### 修复边界

- 前端历史消息必须携带 `relPath/turnId`；后端 `HistTurn` 保留该元数据。
- 后端按当前 `active_doc_path` 过滤/降权历史：同文档历史可作为同文档背景，不同文档历史只能在显式历史回指时以“其他文档历史”
  弱提示出现，不能进入当前文档事实层。
- 意图沿用只由明确历史回指触发；“当前文档/这篇/本文”优先锚定当前 LogicalState，不得因泛编辑词继承旧目标。
- 当前文档超预算时不得只截前部继续编辑；必须注入块 manifest，并要求模型先用 `read_doc/list_blocks` 读取当前文档后才能
  `propose_edit`，否则返回可恢复工具错误。
- `ag_start_turn` 的实际消息装配函数必须有 @GOV 映射，覆盖 BR-AG-DATA-001/002/004/005。

### 实施结果

- `ChatMessage` 与 `HistTurn` 增加 `relPath/turnId` 历史来源；前端运行期消息、历史恢复消息与发送给后端的
  `history[]` 均携带冻结路径。
- `decide_intent_carryover()` 改为“必须有明确历史回指才非 None”；“再短一点 / 重写当前文档 / 优化这篇”等泛编辑请求不再继承上一轮目标。
- 新增 `scope_history_for_current_doc()`：当前文档存在时，同文档历史与跨文档/无归属历史分流；跨文档历史默认丢弃，仅显式回指时弱标签注入。
- 新增 `assemble_agent_turn_messages()` @GOV 映射真实 Prompt 装配顺序：当前文档 P0 晚于历史并贴近末尾。
- 当前文档超内联预算时，`build_current_doc_prompt_context()` 注入块目录而非前部截断；工具循环在当前文档未被
  `read_doc/list_blocks` 读取前拒绝 `propose_edit` 并返回 `AG_CURRENT_DOC_READ_REQUIRED`。
- 协议文档同步 `HistTurn`、历史分流与长文档读取门禁；`npm run governance:generate` 已刷新 ADUS。

### 验证结果

- `npm test -- --run src/services/agService.test.ts src/store/agStore.test.ts`：通过，33 tests。
- `cargo test --manifest-path src-tauri/Cargo.toml ag::context`：通过，7 tests。
- `cargo test --manifest-path src-tauri/Cargo.toml ag_turn_prompt_tests`：通过，4 tests。
- `cargo test --manifest-path src-tauri/Cargo.toml ag::history`：通过，3 tests。
- `npm test -- --run`：通过，33 files / 256 tests。
- `npm run build`：通过；保留 Vite chunk size warning。
- `cargo test --manifest-path src-tauri/Cargo.toml`：190/191 通过；唯一失败仍为既有
  `doc::libreoffice_runtime::tests::docx_to_pdf_end_to_end_when_libreoffice_available`，错误码 `DOC_PREVIEW_FAIL`，非 AG/ED/DE 链路。
- `cargo test --manifest-path src-tauri/Cargo.toml --lib -- --skip doc::libreoffice_runtime::tests::docx_to_pdf_end_to_end_when_libreoffice_available`：
  通过，190 tests。
- `npm run governance:generate`：通过，ADUS 已刷新。
- `npm run governance:audit`：通过；仅保留既有 TERM_ORPHAN / BLOCK_UNANNOTATED warning。
- `git diff --check`：通过。
