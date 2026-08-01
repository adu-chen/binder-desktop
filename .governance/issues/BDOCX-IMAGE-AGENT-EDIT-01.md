# Issue Trace — BDocx 项目图片代理编辑

## 1. 用户可见症状

BDocx 已能保真打开、缩放、移动和人工删除图片，但代理不知道 workspace 内有哪些可用图片，也没有
受控的图片插入/删除操作或可审阅的结构化 Diff。另有一个已复现阻断：DOC 的 `image` 已升级为
`verified`，机器能力清单仍声明 `required_pending`，导致 BDocxCapabilityManifest 校验失败，相关
代理工具 Schema 和提示词投影整体不可用。

## 2. 入口点

- 模型工具入口：`src-tauri/src/ag/tool_registry.rs::tool_definitions_json` →
  `src-tauri/src/lib.rs::ag_propose_edit`。
- 当前文档图片投影：`src/services/agService.ts::collectOpenDocSnapshots` →
  `src-tauri/src/ag/blocks.rs::AgentReadContext`。
- workspace 图片发现：`src-tauri/src/ag/tool_exec.rs::exec_read_only` →
  `src-tauri/src/workspace/listing.rs`。
- ED 草稿应用：`src/components/ed/Editor.tsx` 的 pending Diff effect →
  `src/services/edImageOps.ts`。
- 媒体显示/保存：`src/services/docService.ts` →
  `src-tauri/src/doc/image_media.rs` → `src-tauri/src/doc/edit_chain.rs`。
- 审阅：`src/services/agService.ts` → `src/store/deStore.ts` →
  `src/components/de/DiffCard.tsx`。

## 3. 相关规则

- `BR-SYS-DATA-006`：提示词、工具 Schema、执行前校验必须消费同一 BDocxCapabilityManifest。
- `BR-AG-STATE-002`：只读工具限 workspace；内容写意图必须进入 DE Diff，不得直接落盘。
- `BR-AG-DATA-001`：已打开文档优先消费 LogicalState，workspace 内容读取不得越界。
- `BR-DE-DATA-002`：图片操作、生成基线和结构化审阅信息必须进入可恢复 DiffPayload。
- `BR-DE-STATE-001`：pending 图片 Diff 仍服从 accepted/rejected/expired 互斥终态。
- `BR-ED-DATA-001`：插入位置和删除目标使用稳定 Block/image 动态锚；锚失效必须拒绝。
- `BR-ED-DATA-003`：图片是媒体内容块；ED 只改 LogicalState，不自行写资源文件。
- `BR-ED-STATE-001`：图片 Diff 草稿应用产生脏态，保存才进入既有 WS/DOCX 落盘链。
- `BR-DOC-DATA-002/003`：项目图片进入受控媒体目录后，实例、资源、alt、尺寸、顺序和删除结果
  才能按已验证子集声明；baseline 媒体身份不得漂移。
- `BR-WS-DATA-001`、`X-FORBID-004`：项目图片发现、读取和物化路径必须落在 active workspace。
- `X-FORBID-006`：代理图片插入/删除不能绕过 DE Diff 直接改变文件内容。
- `X-SEQ-002/004`、`X-FORBID-005`：文件资源经 WS/DOC 边界，DOCX 转换只经 DOC；显示层不得
  直接成为持久化真值。

本课题新增核心术语 `ImageOperation`，先在 ED Terminology Registry 注册为 `TERM-ED-008`，再用于
新增 `@GOV term_ref`。复用既有 RULE/CHAIN/CONSTRAINT，不新增规则 ID 或链路 ID。

## 4. 相关链路

`WS-FS → AG-TURN → AG-TOOL → DE-DIFF → ED-EDIT → DOC-CONVERT → WS-SAVE`

只读 `list_images` 停在 `AG-TOOL`；`image_ops` 写意图必须继续经过 DE/ED/DOC/WS。

## 5. 生产者块

- `BR-WS-DATA-001-QUERY-WS-WS-FS-002` 位于 `src-tauri/src/workspace/listing.rs`，生产 workspace
  相对资源列表；本课题新增图片资源投影块。
- `BR-AG-DATA-001-QUERY-AG-AG-TURN-010 / BR-DOC-DATA-003-QUERY-AG-AG-TURN-010` 位于
  `src-tauri/src/ag/blocks.rs`，生产 AgentEditableSnapshot；本课题扩展当前图片快照。
- `BR-SYS-DATA-006-DATA-AG-AG-TOOL-001/002` 与
  `BR-SYS-DATA-006-DATA-AG-AG-TURN-001` 位于
  `src-tauri/src/ag/bdocx_capabilities.rs`，生产清单、Schema 和提示词投影。
- `BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-009 /
  BR-DOC-DATA-003-DATA-DOC-DOC-CONVERT-010` 位于
  `src-tauri/src/doc/image_media.rs`，生产受控媒体身份和恢复统计；本课题扩展项目来源物化。
- `BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-004` 位于
  `src-tauri/src/doc/edit_chain.rs`，生产保存后的 DOCX 字节。

## 6. 消费者块

- `src-tauri/src/ag/tool_exec.rs::exec_read_only` 消费 workspace 图片投影并回注模型；新增核心路径后
  必须直接映射 `BR-AG-STATE-002/BR-AG-DATA-001/BR-WS-DATA-001`。
- `src-tauri/src/lib.rs::ag_propose_edit` 消费 image_ops、动态图片快照和项目资源哈希，生成
  PendingDiff；新增核心路径必须映射 `BR-SYS-DATA-006/BR-AG-STATE-002/BR-DE-DATA-002`。
- `src/components/ed/Editor.tsx` 消费 pending ImageOperation，经 ED 单事务应用为 LogicalState。
- `BR-ED-DATA-003-RB-ED-ED-EDIT-006` 位于 `src/services/docService.ts`，消费稳定媒体引用生成
  `asset://` 显示投影，并在保存前恢复规范引用。
- `BR-DE-STATE-001-RB-DE-DE-DIFF-002` 位于 `src/store/deStore.ts`，消费后端 DiffProjection。
- `src/components/de/DiffCard.tsx` 消费图片前后快照；新增结构化呈现路径必须映射
  `BR-DE-DATA-002`。

## 7. 路径上的空规则块

- `src-tauri/src/ag/prompts.rs` 明确属于运营提示词文本，不直接 `@GOV`，分类 `PLATFORM_GLUE`；
  其能力文本继续只消费 manifest 投影，不维护第二套操作枚举。
- `src/services/agService.ts` 的 IPC DTO 解析与前端事件编排是现有 `DATA_ADAPTER`；新增 image_ops
  投影若承载业务校验则直接映射规则，不以空块规避。
- `src/components/de/DiffCard.tsx` 当前无 `@GOV`；图片结构化 Diff 是 `BR-DE-DATA-002` 的真实消费者，
  本课题补直接映射。
- 未发现需保留为 `rules: []` 且尚未分类的新增核心块。

## 8. boundary 假设

- `document_snapshot=post-update`：当前图片实例必须来自与 `sessionVersion/contentHash` 同次捕获的
  ProseMirror LogicalState；成立，不能由后端重新解析 DiskState 代替。
- `lifecycle_assumption=active_editor_only`：ImageOperation 只对当前活动 BDocx 草稿开放；
  open_session/background 不具备可执行图片节点和媒体显示投影，必须拒绝。
- `lifecycle_assumption=baseline_media_cache`：项目图片复制进按 DOCX baseline 派生的编辑媒体目录；
  未保存草稿恢复时须以 workspace 来源路径+内容哈希重新校验/物化，不能假设系统临时目录永久存在。
- `document_snapshot=pre-diff`：Diff 的 before/after 图片快照、锚、资源哈希必须来自生成时同一基线；
  成立，执行前再次校验。

## 9. 运行时假设

- `caller_context=agent_tool_loop`：模型必须先调用 `list_images`，再把 base、目标 fingerprint 和项目
  图片 source hash 原样带回。
- `runtime_guard=active_bdocx_html_editor`：原生 DOCX 预览、未打开 BDocx、非 HTML 编辑模型均拒绝。
- `runtime_guard=workspace_relative_regular_image`：插入源必须是 workspace 内普通文件，且扩展名、文件
  magic 和内容哈希匹配支持集；远程/data/asset/file/绝对/越界路径拒绝。
- `runtime_guard=structured_image_operation_only`：BDocx 的 replace_block/replace_document 不得通过
  `<img>` 改变图片集合；图片增删只允许 ImageOperation。
- `runtime_guard=single_batch_atomic`：一批 ImageOperation 任一项无效，整批不得生成 Diff 或部分应用。
- `runtime_guard=source_file_preserved`：delete_image 只删文档图片节点，不删除项目源文件或媒体缓存文件。

## 10. 疑似不变量破坏

- 若仅把 manifest 的 `image` 改为 verified 而不增加 ImageOperation 动态校验，模型会把转换保真误解为
  可安全代理编辑，违反 `BR-SYS-DATA-006`。
- 若 `list_images` 直接接受任意绝对目录或跟随越界符号链接，会违反 workspace 边界。
- 若项目图片只复制进临时媒体目录而不保存来源路径/哈希，应用重启后未保存草稿可能无法恢复。
- 若用 replace_block/replace_document 原始 HTML 插入图片，会绕过资源、锚、Diff 和 DOCX 恢复校验。
- 若 image_ops 可进入 background apply，WS 可能把结构化占位文本当正文落盘。
- 若删除图片同时删除项目源文件，将把文档内容裁决扩大为破坏性文件操作，违反本 Scope。

## 11. 验证计划

- Rust：
  - 先复现并闭合 manifest image tier 阻断；验证 version/hash、提示词、Schema 与 image_ops 校验同源。
  - workspace 图片发现验证隐藏目录跳过、相对路径、格式/magic/hash、越界与符号链接拒绝。
  - list_images 验证只接受活动 BDocx LogicalState，返回 base、文档图片实例与项目资源。
  - ImageOperation 验证 insert/delete 字段、锚、fingerprint、session/content hash、源 hash 和批原子性。
  - 项目图片物化验证 deterministic media ref、源文件不变、缓存缺失可按来源/hash 重建。
  - HTML→DOCX→HTML 验证代理插入/删除后的图片资源、alt、宽度和文档流位置。
- 前端：
  - 图片投影与 fingerprint、独立 Block 插入、图片删除、批原子事务和媒体属性保持。
  - Agent open_doc_snapshots/list_images/image_ops DiffProjection 解析与错误投影。
  - Editor pending ImageOperation 草稿应用、脏态、reject 全文恢复和 save 前规范化。
  - DiffCard 验证插入/删除的结构化 before/after、源路径、alt、尺寸、位置和状态展示。
- 负空间：
  - BDocx replace_block/replace_document 含图片集合变化时在 Diff 前拒绝。
  - 不注册远程 URL、外部导入、代理缩放/移动、浮动定位或项目源文件删除。
- 治理：
  - `TERM-ED-008` 注册后先运行 `npm run governance:generate`。
  - 全部新增核心导出块直接映射既有规则，测试以 `covers:` 引用。
  - 运行定向 Vitest/Rust、`npx tsc --noEmit`、`cargo fmt --check`、`cargo check --lib`、
    `npm run governance:generate`、`npm run governance:audit`、`git diff --check`。
  - 消费 audit 的 semantic_review_candidates，人工比对规则文本、业务实体和 boundary。

## Scope Lock

- scope_id：`BDOCX-IMAGE-AGENT-EDIT-01`
- task_type：`CODE_CHANGE`
- confirmation：`.governance/active_scope.json` 已由人类设为 `yes`；守卫按大小写无关规则识别为 YES。
- allowed_files / forbidden_files：以 `.governance/active_scope.json` 为机器可读权威。
- 当前状态：`READY`。

## Task Runtime Report — BDocx 项目图片代理编辑

task_type: CODE_CHANGE

governance_phase: IMPLEMENTATION → VALIDATION

final_state: READY

issue_trace: `.governance/issues/BDOCX-IMAGE-AGENT-EDIT-01.md`

changed_files:

- `.governance/issues/BDOCX-IMAGE-AGENT-EDIT-01.md`
  - 完成 Issue Trace、Scope Lock 与本运行时报告。
- `docs/00_core/A-CORE-C-R-01_ADUS.md`
  - 由 `governance:generate` 刷新至 211 个 @GOV 块、49 个术语。
- `docs/01_modules/SYS/A-SYS-I-P-04_BDocx代理编辑能力协议.md`
  - 统一 BDocxCapabilityManifest 的图片支持等级、ImageOperation 与代理声明边界。
- `docs/01_modules/AG/A-AG-M-D-01_代理功能需求描述.md`
- `docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md`
- `docs/01_modules/AG/A-AG-M-T-02_提示词架构设计.md`
- `docs/01_modules/AG/A-AG-I-P-01_代理接口协议.md`
- `docs/01_modules/AG/A-AG-X-L-02_代理集成计划.md`
  - 定义项目图片发现、`list_images`、提示词/Schema/执行前校验同源及 image_ops 交接。
- `docs/01_modules/ED/A-ED-M-D-01_编辑器功能需求描述.md`
- `docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md`
- `docs/01_modules/ED/A-ED-I-P-01_编辑器接口协议.md`
- `docs/01_modules/ED/A-ED-X-L-02_编辑器专项优化计划.md`
  - 注册 TERM-ED-008 ImageOperation，定义稳定锚、项目来源、单事务应用和排除能力。
- `docs/01_modules/DE/A-DE-M-D-01_DiffEdit功能需求描述.md`
- `docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md`
- `docs/01_modules/DE/A-DE-I-P-01_DiffEdit接口协议.md`
- `docs/01_modules/DE/A-DE-X-L-01_DiffEdit开发计划.md`
  - 定义 image_ops、结构化 DiffCard、`payloadVersion=1`、真实前后 ImageSnapshot 持久化与历史恢复。
- `docs/01_modules/DOC/A-DOC-M-D-01_文档转换功能需求描述.md`
- `docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md`
- `docs/01_modules/DOC/A-DOC-I-P-01_文档转换接口协议.md`
- `docs/01_modules/DOC/A-DOC-X-L-01_文档转换开发计划.md`
  - 定义项目图片物化、稳定媒体身份、来源/hash 重建和 DOCX 往返验收。
- `docs/01_modules/WS/A-WS-M-D-01_工作台功能需求描述.md`
- `docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md`
- `docs/01_modules/WS/A-WS-I-P-01_工作台接口协议.md`
- `docs/01_modules/WS/A-WS-X-L-01_工作台开发计划.md`
  - 定义 workspace 图片发现、越界/符号链接拒绝与媒体缓存边界。
- `src-tauri/resources/bdocx-agent-capabilities.json`
  - 图片支持等级闭合为 verified，并声明 insert/delete 支持集与排除集。
- `src-tauri/src/ag/bdocx_capabilities.rs`
  - ImageOperation manifest 投影、工具 Schema、提示词能力文本和执行前结构校验。
- `src-tauri/src/ag/blocks.rs`
  - 将当前 LogicalState 图片实例快照投影给代理读取上下文。
- `src-tauri/src/ag/tool_registry.rs`
- `src-tauri/src/ag/tool_exec.rs`
  - 注册并执行只读 `list_images`，返回当前图片实例与可直接用于 image_ops 的项目资源字段。
- `src-tauri/src/workspace/listing.rs`
  - 确定性枚举 workspace 内图片候选，跳过隐藏项/符号链接并验证格式、magic 与 SHA-256。
- `src-tauri/src/workspace/diff_apply.rs`
  - 严格校验并一次性、幂等固化 imageBefore/imageAfter/imageAffectedBlockIds。
- `src-tauri/src/doc/image_media.rs`
  - 校验/物化项目图片、保持来源/hash、保存后携带仍被引用的媒体、缓存缺失时重建。
- `src-tauri/src/doc/edit_chain.rs`
  - 保存链保留图片媒体；新增真实 Pandoc 项目图片插入/删除往返验证。
- `src-tauri/src/lib.rs`
  - 接入 image_ops 生成与防绕过校验、媒体授权、Diff 投影、快照更新 command 与历史恢复。
- `src/services/docService.ts`
- `src/services/docService.test.ts`
  - 稳定媒体引用与本地显示 URL 分层、baseline 感知显示缓存和恢复测试。
- `src/services/edImageOps.ts`
- `src/services/edImageOps.test.ts`
  - ImageOperation 严格解析、图片实例投影、批原子应用、删除/插入与历史快照防御解析。
- `src/services/agService.ts`
- `src/services/agService.test.ts`
  - 当前图片快照采集、序列化事件消费、image_ops DiffProjection 与历史结构化恢复。
- `src/services/deService.ts`
- `src/services/deService.test.ts`
  - 新增 `de.update_image_diff_snapshots` IPC 写入和 DTO 历史字段。
- `src/store/deStore.ts`
- `src/store/deStore.test.ts`
  - 保存图片操作载荷、前后快照和受影响 Block 的当前会话投影。
- `src/components/ed/Editor.tsx`
  - 当前活动 BDocx 应用 image_ops，记录前后快照，标记草稿并立即持久化审阅快照。
- `src/components/ed/extensions/ResizableImage.ts`
- `src/components/ed/extensions/ResizableImage.test.ts`
  - 保留稳定媒体引用和项目来源/hash 属性。
- `src/components/de/DiffCard.tsx`
- `src/components/de/DiffCard.test.tsx`
  - 逐项展示插入/删除、锚、alt、尺寸、媒体、项目来源/hash、前后实例数及源文件保留。
- `src/styles.css`
  - 增加图片结构化 Diff 明细样式。

scope:

  mode: explicit

  status: confirmed

phase_gate:

  status: passed

  evidence: 需求、设计、接口和计划层已定义 ImageOperation、能力清单、DE/ED/DOC/WS 边界及验收；TERM-ED-008 已注册，Scope 由人类确认。

mapping_chain:

  status: complete

  evidence: REQ/设计规则 → BR-SYS-DATA-006、BR-AG-DATA-001、BR-AG-STATE-002、BR-DE-DATA-002、BR-ED-DATA-001/003、BR-DOC-DATA-002/003、BR-WS-DATA-001 → AG-TOOL/DE-DIFF/ED-EDIT/DOC-CONVERT/WS-FS @GOV → Rust/Vitest covers → 本运行时报告。

validation:

- `npx tsc --noEmit`
  - passed。
- `npx vitest run src/services/edImageOps.test.ts src/services/docService.test.ts src/services/agService.test.ts src/services/deService.test.ts src/store/deStore.test.ts src/components/de/DiffCard.test.tsx src/components/ed/extensions/ResizableImage.test.ts src/components/ed/Editor.lifecycle.test.tsx`
  - passed，8 个测试文件、101/101。
- `cargo test image --lib -- --nocapture`
  - passed，21/21；含真实 Pandoc 项目图片插入后的资源、alt、宽度、Block 位置与删除/保存重开往返。
- `cargo test ag_edit_base_tests --lib -- --nocapture`
  - passed，10/10；含 image_ops 类型隔离、动态基线/锚/hash 校验和 raw HTML 防绕过。
- `cargo fmt --check`
  - passed。
- `cargo check --lib`
  - passed；仅输出本课题外既有 LibreOffice/辅助函数 dead_code warning。
- `npm run governance:generate`
  - passed；211 个 @GOV 块、78 条规则标注、23 条链路、49 个术语。
- `npm run governance:audit`
  - passed；governance_findings 为空、77 条已注册规则均有测试覆盖、TERM integrity 与 BOUNDARY_ABSTRACT 检查通过。
- `semantic_review_candidates` 人工语义比对
  - passed；新增 `persist_image_diff_snapshots`、`persistImageDiffSnapshots`、`parseImageDiffSnapshotProjection`
    以及既有 ImageOperation/DiffCard 候选的 DiffPayload、ImageSnapshot、LogicalState、项目来源/hash、
    文档/源图片排除副作用实体均与规则原文一致，未发现 possible_term_drift。
- `git diff --check`
  - passed。

stop_conditions:

- 无。

recovery:

- VALIDATING 深审发现 imageBefore/imageAfter 原先只存在前端内存，历史恢复不能重建结构化图片 Diff；
  已在确认 Scope 内复用既有 DiffPayload JSON 修复，未新增数据库 schema，新增一次性幂等写入、历史查询、
  防御解析与后端/前端回归测试后重新通过全部验证。

unresolved_items:

- 未执行桌面应用人工 UI 冒烟；建议人类用实际 workspace 图片完成一次
  “list_images → 代理插入/删除 → 拒绝恢复 → 接受/保存 → 关闭重开 → 历史 DiffCard 展开”复测。
  自动化已覆盖协议、持久化、LogicalState 操作与真实 Pandoc DOCX 往返，该项不阻断 G4 READY。

## 人类闭合决定

- 2026-07-30：人类要求在图片代理编辑完整链路审计、声明与设计同步后闭合；后继
  `BDOCX-IMAGE-ADVANCED-AGENT-OPS-01`、`BDOCX-IMAGE-E2E-CLOSURE-01` 和
  `BDOCX-IMAGE-COMPOSITE-IMPORT-01` 已补齐本课题初始范围外的缩放、移动、对齐、组合操作与受控导入。
- 当前有效能力以 BDocxCapabilityManifest 1.4.0 为准：insert/delete/resize/move/layout 均经
  `list_images → image_ops → structured Diff → ED LogicalState → DOCX save`，项目源文件不属于文档
  Diff 副作用。
- final_state：`CLOSED`。
