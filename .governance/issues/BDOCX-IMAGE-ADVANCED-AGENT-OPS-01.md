# Issue Trace — BDocx 图片高级代理操作

## 1. 用户可见症状

人工已可缩放、上下移动图片，但代理目前只支持插入/删除，不能安全地缩放、移动或设置独立图片段落的
左/中/右对齐；图片实例 ID 还会随宽度或顺序变化，无法作为稳定操作目标。

## 2. 入口点

- `src-tauri/src/ag/tool_exec.rs::exec_list_images`：代理读取当前图片和项目图片。
- `src-tauri/src/lib.rs::image_ops_payload`：生成 PendingDiff 前校验 ImageOperation。
- `src/services/edImageOps.ts::applyImageOperations`：当前活动 BDocx LogicalState 原子应用。
- `src/components/de/DiffCard.tsx`：结构化图片 Diff 展示。
- `src-tauri/src/doc/edit_chain.rs`：HTML→DOCX 保存及图片往返验证。

## 3. 相关规则

复用 `BR-SYS-DATA-006`、`BR-AG-STATE-002`、`BR-AG-DATA-001`、`BR-DE-DATA-002`、
`BR-ED-DATA-001/003`、`BR-ED-STATE-001`、`BR-DOC-DATA-002/003`、`BR-WS-DATA-001`，
以及 `X-FORBID-004/005/006`、`X-SEQ-002/004`。术语复用 `TERM-ED-008 ImageOperation`、
`TERM-SYS-002 BDocxCapabilityManifest`、`TERM-WS-001 Workspace`，不新增静态定义。

## 4. 相关链路

`WS-FS → AG-TURN → AG-TOOL → DE-DIFF → ED-EDIT → DOC-CONVERT → WS-SAVE`

## 5. 生产者块

- `src/services/edImageOps.ts::projectDocumentImages` 生产稳定 imageId、当前 fingerprint 和动态能力。
- `src-tauri/src/workspace/listing.rs::list_project_image_candidates` 生产受控项目图片。
- `src-tauri/src/ag/bdocx_capabilities.rs` 生产 manifest、提示词和工具 Schema。
- `src-tauri/src/doc/image_media.rs` 生产保存后的图片资源/尺寸验证结果。

## 6. 消费者块

- `src-tauri/src/lib.rs::image_ops_payload` 消费当前快照并拒绝过期或越权操作。
- `src/services/edImageOps.ts::applyImageOperations` 消费经校验批次。
- `src/store/deStore.ts`、`src/components/de/DiffCard.tsx` 消费前后图片快照。
- `src-tauri/src/doc/edit_chain.rs` 消费规范 HTML 并保存 DOCX。

## 7. 路径上的空规则块

未发现本课题必须新增的 `rules: []` 核心块；纯类型和测试按既有豁免处理。

## 8. boundary 假设

- `document_snapshot=pre-diff`：imageId/fingerprint/capabilities 必须来自同一次活动 BDocx 快照。
- `lifecycle_assumption=active_editor_only`：高级 ImageOperation 不在后台 HTML 字符串上执行。
- `lifecycle_assumption=stable_session_identity`：imageId 在同一编辑会话内不随宽度、对齐或顺序变化。

## 9. 运行时假设

- `runtime_guard=dynamic_image_capability`：每项操作必须服从当前 capabilities、resizeBounds 和
  moveDirections。
- `runtime_guard=single_image_single_operation_per_batch`：同一图片单批最多一个修改。
- `runtime_guard=standalone_layout_only`：布局只允许独立顶层图片段落的 left/center/right。
- `runtime_guard=workspace_image_roots`：项目图片只从配置的 workspace 相对图片根发现，不跟随符号链接。

## 10. 疑似不变量破坏

- 由 ordinal 生成 imageId 会在移动后错绑目标。
- fingerprint 不随宽度/对齐变化会放过过期修改；imageId 随 fingerprint 变化又会破坏稳定身份。
- 不先验证整批即执行会产生部分应用。
- 向代理暴露未验证的 paragraph_align 会形成虚假保真声明。
- 继续递归整个 workspace 会让代理看到非预期图片目录。

## 11. 验证计划

- Vitest：稳定 imageId、重复 ID 修复、动态能力、缩放/移动/对齐、批原子、DiffCard 与历史快照。
- Rust：manifest/Schema/提示词同源、动态校验、受控图片根、DOCX `wp:extent`/顺序/`w:jc`。
- 运行 `npx tsc --noEmit`、定向 Vitest、定向 Cargo test、`cargo fmt --check`、
  `cargo check --lib`、`npm run governance:generate`、`npm run governance:audit`、
  `git diff --check`。

## Scope Lock

- scope_id：`BDOCX-IMAGE-ADVANCED-AGENT-OPS-01`
- task_type：`CODE_CHANGE`
- confirmation：人类已将 `.governance/active_scope.json` 设为 `yes`。
- allowed_files / forbidden_files：以 `.governance/active_scope.json` 为准。
- 当前状态：`READY`。

## Task Runtime Report — BDocx 图片高级代理操作

task_type: CODE_CHANGE
governance_phase: VALIDATION
final_state: READY
issue_trace: `.governance/issues/BDOCX-IMAGE-ADVANCED-AGENT-OPS-01.md`

changed_files:
  - `.governance/issues/BDOCX-IMAGE-ADVANCED-AGENT-OPS-01.md` # 新增 Issue Trace 与运行报告
  - `docs/00_core/A-CORE-C-R-01_ADUS.md` # 机器索引刷新
  - `docs/01_modules/SYS/A-SYS-I-P-04_BDocx代理编辑能力协议.md` # 扩展 ImageOperation 能力声明
  - `docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md`、`docs/01_modules/ED/A-ED-I-P-01_编辑器接口协议.md` # 稳定图片身份与动态操作契约
  - `docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md`、`docs/01_modules/AG/A-AG-I-P-01_代理接口协议.md` # list_images 与代理操作协议
  - `docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md` # 高级图片结构化 Diff
  - `docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md`、`docs/01_modules/DOC/A-DOC-I-P-01_文档转换接口协议.md` # 图片布局与保存校验
  - `docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md`、`docs/01_modules/WS/A-WS-I-P-01_工作台接口协议.md` # 受控 imageRoots
  - `src-tauri/resources/bdocx-agent-capabilities.json`、`src-tauri/src/ag/bdocx_capabilities.rs` # manifest/提示词/Schema 同源升级
  - `src-tauri/src/ag/tool_exec.rs`、`src-tauri/src/lib.rs` # list_images 投影及 PendingDiff 前动态校验
  - `src-tauri/src/workspace/listing.rs` # 项目图片目录配置与符号链接边界
  - `src-tauri/src/doc/docx_image_layout.rs`、`src-tauri/src/doc/edit_chain.rs`、`src-tauri/src/doc/image_media.rs`、`src-tauri/src/doc/mod.rs` # w:jc 往返与数量/媒体/顺序/wp:extent 保存门
  - `src/components/ed/extensions/ImageIdExtension.ts`、`src/components/ed/extensions/index.ts` # 会话内稳定 imageId
  - `src/services/edImageOps.ts`、`src/services/agService.ts` # 动态快照与原子 resize/move/layout
  - `src/components/de/DiffCard.tsx` # 高级图片操作逐项前后值展示
  - `src/components/ed/extensions/ImageIdExtension.test.ts`、`src/services/edImageOps.test.ts`、`src/services/agService.test.ts`、`src/services/deService.test.ts`、`src/store/deStore.test.ts`、`src/components/de/DiffCard.test.tsx` # 协议、执行、历史与呈现回归

scope:
  mode: explicit
  status: confirmed

phase_gate:
  status: passed
  evidence: 复用既有 ImageOperation/BDocxCapabilityManifest 规则与术语；用户确认的 active_scope 限定实现文件，未引入新状态机、规则、术语或依赖。

mapping_chain:
  status: complete
  evidence: BR-SYS-DATA-006/BR-AG-STATE-002/BR-DE-DATA-002/BR-ED-DATA-003/BR-DOC-DATA-002/BR-WS-DATA-001 → AG-TOOL/DE-DIFF/ED-EDIT/DOC-CONVERT/WS-FS → @GOV → Vitest/Cargo 往返测试已闭合；相关 semantic_review_candidates 与规则原文人工比对，无实体名或边界语义偏差。

validation:
  - `npx tsc --noEmit` # passed
  - `npx vitest run src/services/edImageOps.test.ts src/components/ed/extensions/ImageIdExtension.test.ts src/services/agService.test.ts src/services/deService.test.ts src/store/deStore.test.ts src/components/de/DiffCard.test.tsx src/components/ed/extensions/ResizableImage.test.ts src/components/ed/ImageMoveControls.test.tsx` # passed，8 files / 90 tests
  - `cargo test bdocx_capabilities --lib -- --nocapture` # passed，7 tests
  - `cargo test image --lib -- --nocapture` # passed，23 tests，含真实 Pandoc 图片数量/媒体/顺序/extent/w:jc 往返
  - `cargo fmt -- --check` # passed
  - `cargo check --lib` # passed；仅既有 LibreOffice/AG 未使用函数 warning
  - `npm run governance:generate` # passed，212 个 @GOV 块
  - `npm run governance:audit` # passed
  - `git diff --check` # passed

stop_conditions:
  - 无

recovery:
  - 无

unresolved_items:
  - 未执行真实云模型对话与 WKWebView 人工 smoke test；协议、编辑事务、Diff 和 DOCX 保存链已有自动验证，本项作为发布前人工验收，不阻断本课题 READY。

## 人类闭合决定

- 2026-07-29：人类明确要求闭合本课题。
- 闭合边界：代理缩放、上下移动、独立图片段落左/中/右对齐，稳定 imageId、动态能力、结构化
  Diff，以及 DOCX 图片数量/媒体/顺序/`wp:extent`/`w:jc` 保存门均已有自动验证。
- 真实代理回合、WKWebView 应用、裁决、保存重开的组合验收转入
  `BDOCX-IMAGE-E2E-CLOSURE-01`，不再作为本课题遗留项。
- 当前状态：`READY/CLOSED`。
