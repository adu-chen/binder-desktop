# Issue Trace — BDocx 图片代理编辑 E2E 闭合

## 1. 用户可见症状

图片插入、删除、缩放、上下移动和独立段落对齐已分别实现，但尚缺一组可重复的组合证据证明：
代理建议进入 PendingDiff 后能在活动 BDocx 正确应用、拒绝恢复、保存接受，并在 DOCX 重开后保持
图片内容、尺寸、顺序和对齐；失败路径也尚未以同一组合测试证明原 DOCX 不被覆盖。

## 2. 入口点

- `src-tauri/src/ag/tool_exec.rs::exec_list_images`：产生当前图片与项目图片快照。
- `src-tauri/src/lib.rs::image_ops_payload`：PendingDiff 前静态与动态校验。
- `src/components/ed/Editor.tsx`：消费 pending `image_ops` 并写入 LogicalState。
- `src/services/deService.ts` / `src/store/deStore.ts`：持久化前后快照及接受/拒绝。
- `src/services/edService.ts` / `src-tauri/src/lib.rs::route_save_from_edit`：保存时接受并转换落盘。
- `src-tauri/src/doc/edit_chain.rs`：真实 Pandoc/OOXML 保存重开。

## 3. 相关规则

复用 `BR-SYS-DATA-006`、`BR-AG-STATE-002`、`BR-DE-DATA-002`、`BR-DE-STATE-001`、
`BR-ED-DATA-001/003`、`BR-ED-STATE-001`、`BR-DOC-DATA-002/003`、
`BR-WS-DATA-001`、`BR-WS-PERSIST-001`，以及 `X-FORBID-004/005/006`、
`X-SEQ-002/004`。术语复用 `TERM-ED-008 ImageOperation`、
`TERM-SYS-002 BDocxCapabilityManifest`、`TERM-DE-003 DiffPayload`。

## 4. 相关链路

`WS-FS → AG-TOOL → DE-DIFF → ED-EDIT → DOC-CONVERT → WS-SAVE`

## 5. 生产者块

- `exec_list_images` 生产当前 ImageSnapshot 与受控 ProjectImageAsset。
- `image_ops_payload` 生产经校验且已物化媒体的 ImageOperation DiffPayload。
- `applyImageOperations` 生产单事务后的 LogicalState 与 before/after ImageSnapshot。
- `html_to_docx` 生产经图片保存门验证的 DOCX 字节。

## 6. 消费者块

- `agService` 消费工具投影并建立 PendingDiff。
- `Editor` 消费 PendingDiff 并调用 `applyImageOperations`。
- `deService/deStore/DiffCard` 消费图片前后快照完成审阅与裁决。
- `edService/route_save_from_edit` 消费当前 LogicalState 并保存。
- `docx_to_edit_html` 消费保存结果并重建图片编辑模型。

## 7. 路径上的空规则块

未发现必须新增的静态规则块。本课题只组合验证已注册规则；测试辅助代码不建立新的业务边界。

## 8. boundary 假设

- `document_snapshot=active_editor`：图片操作只针对同一活动 BDocx 的当前 LogicalState。
- `diff_snapshot=pre_apply`：拒绝恢复必须使用应用前全文，不从当前 HTML 猜测逆操作。
- `save_boundary=convert_then_atomic_write`：转换或图片校验失败发生在 WS 覆盖原文件之前。
- `model_boundary=deterministic_protocol_input`：自动测试使用固定合法工具参数，不模拟云模型文本生成。

## 9. 运行时假设

- ImageOperation 必须携带当前 sessionVersion/contentHash、稳定 imageId 与当前 fingerprint。
- 同一图片单批可各含一次 resize/layout/move，并按固定顺序原子执行；delete 独占。任一操作失败不得
  产生部分 LogicalState 修改。
- 保存会将已应用 pending Diff 视为接受；拒绝则完整恢复应用前内容。
- 项目源图片只读，保存与拒绝均不得删除或改写源文件。

## 10. 疑似不变量破坏

- 单元测试分别通过但 Editor effect 未正确串联，可能导致 Diff 已显示而 LogicalState 未应用。
- 应用后快照持久化失败或拒绝路径按块恢复，可能留下图片节点或错误顺序。
- 保存接受 pending Diff 与转换失败组合不当，可能出现裁决状态与磁盘内容不一致。
- 多图片操作后 imageId/fingerprint 更新若错位，可能在后续操作或历史 Diff 中指向错误图片。
- 图片资源或 OOXML 校验失败若发生在写盘后，可能破坏原 DOCX。

## 11. 验证计划

- Vitest：Editor 对 pending image_ops 的真实 effect、脏态、before/after 快照持久化、拒绝恢复；
  edService 保存接受顺序及失败状态；DiffCard 历史重建。
- Rust：受控项目图片发现与校验、ImageOperation 预校验、真实 Pandoc 保存重开、多图片内容/顺序/
  `wp:extent`/`w:jc`，以及缺资源或校验失败时原文件不变。
- 运行 `npx tsc --noEmit`、定向 Vitest、定向 Cargo test、`cargo fmt --check`、
  `cargo check --lib`、`npm run governance:audit`、`git diff --check`。

## Scope Lock

- scope_id：`BDOCX-IMAGE-E2E-CLOSURE-01`
- task_type：`CODE_CHANGE`
- confirmation：人类已将 `.governance/active_scope.json.confirmation` 设为 `YES`。
- allowed_files / forbidden_files：以 `.governance/active_scope.json` 为准。
- 当前状态：`READY`，等待人类确认闭合。

## 实施结果

- 未新增图片能力、工具、状态或 UI；仅将 `Editor` 中既有 pending `image_ops` 编排提取为
  `applyPendingImageDiffToEditor` 可测试边界，生产 effect 仍复用同一实现。
- 新增 Editor 组合测试：在真实 ProseMirror 文档中对三张稳定图片分别执行缩放、上下移动和独立段落
  对齐，验证脏态、before/after ImageSnapshot、affectedBlockIds、拒绝恢复基线与快照持久化参数；
  sessionVersion/contentHash 过期时验证零修改、零持久化。
- 将 DOCX 保存契约测试具体化为 `image_ops` PendingDiff，验证保存调用顺序仍为
  `de.accept_draft_diff → doc.save_from_edit`，成功后才进入 accepted + draftSaved。
- 新增真实 Pandoc/OOXML 组合测试：验证多图片顺序、240/120px 尺寸、right/center 对齐、删除结果、
  重开恢复报告、媒体源只读；缺失媒体转换失败后目标 DOCX 字节保持不变。
- 测试未暴露新的产品实现断点，因此未修改代理工具协议、manifest、数据库或图片编辑算法。

## Task Runtime Report

- task_type：`CODE_CHANGE`
- scope：只触及 Scope Lock 允许的 Issue、ADUS、Editor/测试、edService 测试和 DOCX 编辑链测试。
- validation：
  - `npx tsc --noEmit`：通过。
  - 定向 Vitest 7 文件：92 tests passed。
  - `cargo test image --lib -- --nocapture`：24 tests passed。
  - 新增真实保存重开用例单独执行：1 passed。
  - `cargo check --lib`：通过；仅有既存 dead_code warnings。
  - `cargo fmt --check`：通过。
  - `npm run governance:generate`：完成，ADUS 已刷新。
  - `npm run governance:audit`：status=passed。
  - `git diff --check`：通过。
- unresolved_items：
  - 未新增 WKWebView 人工 UI 项；本课题不改变人工图片控件，自动闭环验证不替代用户后续对现有 UI
    的体验性回归。
  - `cargo check` 的 LibreOffice/AG 既存 dead_code warnings 不属于本 Scope。
- final_state：`READY`；实现与自动验证已完成，尚未代替人类写入闭合决定。

## 人类闭合决定

- 2026-07-30：人类在真实应用中确认图片拖动插入已实现，保存、关闭、重新编译打开后读取正常，并要求
  对图片代理编辑完整修复审计后闭合。
- 既有人工代理回合已覆盖插入、删除、缩放、上下移动与独立段落对齐的建议生成和 Diff 裁决；自动 E2E
  覆盖组合事务、拒绝恢复、保存接受、真实 Pandoc/OOXML 与重开。闭合不扩展为图片视觉识别/OCR、高级
  布局或任意坐标定位。
- final_state：`CLOSED`。
