# Issue Trace — BDocx 图片媒体保真基础

## 1. 用户可见症状

BDocx 中的图片可以显示和选中，但保存时图片相对路径由前端还原为 `media/...` 后，Pandoc 却在
DOCX 父目录解析资源，而图片实际位于按基线隔离的编辑临时目录。Pandoc 找不到图片时仍可能以成功状态
生成 DOCX，并把图片替换为 alt 文本；图片尺寸在重开时又由 `width` 属性变成 CSS 长度，编辑器不能恢复。

## 2. 入口点

- 打开：`src/services/edService.ts::openDocument` →
  `src/services/docService.ts::openDocxForEdit` →
  `src-tauri/src/lib.rs::route_open_for_edit` →
  `src-tauri/src/doc/edit_chain.rs::docx_to_edit_html`。
- 会话缓存：`src/components/ed/Editor.tsx::serialize/persistEditorSessionSnapshot` →
  `src/services/edService.ts::persistSessionSnapshot`。
- 保存：`src/components/ed/Editor.tsx::saveCurrent` →
  `src/services/edService.ts::saveDocxDocument` →
  `src-tauri/src/lib.rs::route_save_from_edit` →
  `src-tauri/src/doc/edit_chain.rs::html_to_docx`。
- 图片解析：`src/components/ed/extensions/ResizableImage.ts::ResizableImage`。

## 3. 相关规则

- `BR-ED-DATA-003`：图片媒体以内容模型承载，显示投影不得取代稳定资源引用；媒体存取受 workspace
  边界约束。
- `BR-DOC-DATA-002`：StyleSubset 非已验证项不得静默丢失；嵌入图片晋级前必须有真实往返证据。
- `BR-DOC-DATA-003`：DOCX 转换后的编辑 HTML 与稳定文档身份/块身份由同一转换链生产。
- `BR-DOC-STATE-001`：保存转换失败进入 SaveError，原 DOCX 不变。
- `BR-WS-OBS-001`：保存继续服从基线冲突保护。
- `X-FORBID-004`：文件路径不得越出 workspace；后端仍是路径解析与授权边界。
- `X-SEQ-002`、`X-SEQ-004`：DOCX 二进制转换只经 DOC command，落盘继续经 WS 原子保存。
- `X-FORBID-005`：图片属性属于编辑模型，不能由显示层直接持久化。

本课题复用既有规则，不新增 RULE、CHAIN、CONSTRAINT 或 TERM。`DocxEditMedia` /
`ImageRecovery` 仅作为 DOC 转换实现 DTO，不写入 `@GOV term_ref`。

## 4. 相关链路

`ED-EDIT → DOC-CONVERT → WS-SAVE`

打开与恢复反向流：

`DOC-CONVERT → ED-EDIT`

## 5. 生产者块

- `BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-003` 位于
  `src-tauri/src/doc/edit_chain.rs`：生产可编辑 HTML 与抽取媒体。
- `BR-ED-DATA-003-RB-ED-ED-EDIT-006` 位于
  `src/services/docService.ts`：生产显示用 asset URL。
- `BR-ED-DATA-002-RB-ED-ED-EDIT-012` 位于
  `src/services/edService.ts`：生产编辑会话 LogicalState 快照。
- 本课题新增的 `src-tauri/src/doc/image_media.rs` 生产按基线隔离、可恢复的媒体目录和图片恢复统计，
  追溯 `BR-DOC-DATA-002/003`。

## 6. 消费者块

- `BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-004` 位于
  `src-tauri/src/doc/edit_chain.rs`：消费编辑 HTML 与受控媒体目录生成 DOCX。
- `BR-ED-DATA-003-RB-ED-ED-EDIT-006` 位于
  `src/services/docService.ts`：消费稳定媒体引用生成显示地址，并在保存前恢复规范引用。
- `src/components/ed/extensions/ResizableImage.ts`：消费图片 HTML 属性进入 ProseMirror 图片节点。
- `src-tauri/src/lib.rs::route_save_from_edit`：消费加载基线解析同一媒体目录，再交 WS 原子保存。

## 7. 路径上的空规则块

- `ResizableImage.ts` 当前没有 `@GOV` 块；它是 TipTap NodeView 适配层，本课题只修正属性解析，不新增
  独立业务规则，分类为 `PLATFORM_GLUE`。
- `edStore.ts` 的会话 DTO 与缓存更新属于 `DATA_ADAPTER`；本课题只补持久化基线字段。
- 未发现执行路径中需要先行 triage 的 `rules: []` @GOV 块。

## 8. boundary 假设

- `docx_to_edit_html` 假设 `document_snapshot` 是同次读取的稳定 DOCX 字节；成立。媒体抽取目录必须由
  该字节基线派生。
- `rewriteMediaSrc` 原假设进程内 Map 可覆盖保存生命周期；在重启恢复和多会话下不成立。本课题改为
  HTML 自带规范相对引用，Map 只保留向后兼容，不再是真值。
- `html_to_docx` 原假设 DOCX 父目录含 HTML 引用资源；不成立。本课题改为后端按加载基线解析并验证
  编辑媒体目录，单次转换在该目录读取资源。
- WS 保存的 `document_snapshot` 仍是加载基线；成立，外部变化保护不放宽。

## 9. 运行时假设

- `caller_context=ED-EDIT`：只有已进入 BDocx 编辑会话的文档使用媒体恢复接口。
- `runtime_guard=baseline`：媒体目录必须与当前编辑会话基线相同；磁盘已变化时不得从新 DOCX
  重建旧草稿的媒体。
- `runtime_guard=backend-resolved-path`：前端不能传入任意绝对媒体目录，后端只接受文档相对路径和
  已有加载基线并自行派生目录。
- `runtime_guard=no-silent-loss`：Pandoc 的资源缺失警告或输出包图片计数下降必须转为明确失败。

## 10. 疑似不变量破坏

- 若保存继续使用 DOCX 父目录，Pandoc 会把图片替换为 alt 文本并可能返回成功，违反
  `BR-DOC-DATA-002/BR-DOC-STATE-001`。
- 若持久化快照只保存 `asset://`，应用重启后授权和内存映射消失，LogicalState 不再可恢复。
- 若保存时根据新磁盘 DOCX 重建旧基线媒体，外部变化可能把用户草稿中的图片悄悄替换成另一份资源，
  违反 `BR-WS-OBS-001`。
- 若只检查 Pandoc 退出码而忽略 stderr/OOXML 包内容，仍会制造虚假成功。

## 11. 验证计划

- Rust 单元/集成：
  - HTML 图片规范引用解析、越界/绝对路径拒绝。
  - 媒体目录按基线派生、缓存缺失时仅从相同磁盘基线重建。
  - HTML→DOCX 资源存在时 `word/media`、relationship 与 `wp:extent` 存在。
  - 资源缺失时转换失败，不生成可落盘结果。
  - DOCX→HTML→DOCX→HTML 后图片数量、alt 与尺寸恢复统计一致。
- 前端：
  - `rewriteMediaSrc` 写入稳定相对引用，`restoreMediaSrc` 不依赖进程内 Map。
  - snapshot 持久化/恢复包含加载基线，重启恢复时重新解析媒体显示地址。
  - ResizableImage 同时解析 `width` 属性和 Pandoc CSS `width/height` 长度。
- 链路：
  - `doc.open_for_edit → ed session snapshot → doc.restore_edit_media → doc.save_from_edit`
    使用同一基线与媒体目录。
  - 保存失败保持原 DOCX 字节与基线不变，SM-DOC-01 进入 SaveError。
- 治理：
  - 不新增静态定义；文档只同步既有 REQ/RULE 的验收和计划。
  - 运行相关 Vitest、Rust tests、`cargo check --lib`、`npx tsc --noEmit`、
    `npm run governance:generate`、`npm run governance:audit`、`git diff --check`。

## Scope Lock

- scope_id：`BDOCX-IMAGE-MEDIA-FOUNDATION-01`
- task_type：`CODE_CHANGE`
- confirmation：以 `.governance/active_scope.json` 的人类确认值 `YES` 为准。
- allowed_files / forbidden_files：以 `.governance/active_scope.json` 为机器可读权威。

## 12. 实施结果

1. 新增 `doc/image_media.rs`，统一来源 baseline → 编辑媒体目录派生、安全相对引用检查、缓存资源检查、
   来源/输出 OOXML 图片计数和 `ImageRecovery`（实例/资源/尺寸/缺失资源）报告。
2. `doc.open_for_edit` 改为统一 baseline 媒体目录，图片授权失败即 OpenError；新增
   `doc.restore_edit_media`，重启时重新授权，缓存缺失只允许从同 baseline 磁盘 DOCX 重建。
3. `doc.save_from_edit` 的 baseline 改为必填；Pandoc cwd 改为 baseline 媒体目录。转换前拒绝缺失、
   绝对、本地协议或越界资源，转换中并发排空 stderr，资源丢失警告转明确失败，转换后核对最终 OOXML
   图片对象数；任一失败均不进入 WS 原子覆盖。
4. Pandoc 子进程等待改为可真实 kill 的轮询超时，stderr 不再无人读取；临时输出名增加进程内序号，
   避免并发同 PID 路径碰撞。
5. 前端图片 HTML 增 `data-binder-media-src` 稳定相对身份；`asset://` 只作显示投影，保存与持久快照
   反向规范化不依赖进程内 Map。
6. OpenDocSnapshot 的 BDocx 恢复投影增 baseline。现有数据库表不扩列：后端只在存储层内部为 content
   使用可逆 envelope，`ed.list_session_snapshots` 返回前解码，IPC/TipTap/DOCX 均不含该元数据。
   旧 dirty 快照缺 baseline 时不冒用当前磁盘基线，恢复与保存保持阻断直至用户重载。
7. ResizableImage 保留稳定媒体身份，解析 width/height 属性以及 Pandoc CSS 的 px/in/cm/mm/pt/pc
   长度并按 96dpi 规范为 px；人工拖宽提交后清除固定 height，以固有比例重新计算。
8. StyleSubset 的嵌入图片升级为已实测保证，并由图片实例、资源、尺寸恢复报告和资源缺失保存门禁约束；
   浮动/绝对定位、图片移动、代理插入/删除、远程资源导入不在本课题闭合范围。

## 13. 语义审计

本课题新增两个 `@GOV` block，均复用既有规则：

- `BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-009`：来源/HTML 图片实例、嵌入资源、显式尺寸与缺失资源进入
  实际恢复统计，符合非已恢复项不得静默宣称保真的规则。
- `BR-DOC-DATA-003-DATA-DOC-DOC-CONVERT-010`：来源 baseline 派生稳定媒体目录并供打开、恢复、保存
  共用，符合转换编辑模型与稳定文档身份同链生产的边界。

人工比对未发现规则文本、核心实体、链路 owner 或 boundary 描述不一致；未新增 TERM、RULE、CHAIN、
CONSTRAINT 或状态机。

## 14. Task Runtime Report

- task_type：`CODE_CHANGE`
- governance_phase：`VALIDATION`
- final_state：`READY`
- issue_trace：`BDOCX-IMAGE-MEDIA-FOUNDATION-01`
- scope：写入仅发生在 active Scope 的 Issue、DOC/ED/SYS 文档、DOC 转换实现、ED service/store、
  ResizableImage 与定向测试文件；未修改 forbidden files。工作区其余脏差异均为人类/前序课题既有内容，
  本课题未覆盖或还原。
- phase_gate：REQUIREMENT 复用 REQ-ED-004 与 REQ-DOC-002/003；DESIGN 复用
  BR-ED-DATA-003、BR-DOC-DATA-002/003、BR-DOC-STATE-001、BR-WS-OBS-001；PLAN/IMPLEMENTATION/
  VALIDATION 已完成。
- validation：
  - `cargo fmt --check`：通过。
  - `cargo check --lib`：通过；仅有既有 LO/AG dead_code warning。
  - `cargo test doc::image_media --lib`：4/4 通过。
  - `cargo test doc::style_subset --lib`：3/3 通过。
  - `cargo test doc::edit_chain --lib`：6/6 通过，含真实 PNG→DOCX→HTML、`word/media`、
    `w:drawing`、`wp:extent`、alt、恢复报告、缺资源、Pandoc 警告与输出图片数下降门禁。
  - `npx tsc --noEmit`：通过。
  - 定向 Vitest：46/46 通过。
  - 全量 Vitest：389/393 通过；4 项失败均为任务开始前已登记的 `src/services/edLocate.test.ts`
    选区偏移断言，本课题未触碰该实现/测试且定向图片链无回归。
  - `npm run governance:generate`：通过。
  - `npm run governance:audit`：passed，无 governance finding。
  - `git diff --check`：通过。
- unresolved_items：
  - Tauri/WKWebView 中应用重启后图片重新显示、保存、关闭重开的人工链路尚未由本轮自动化替代；
    不据自动测试声称 UI 人工验收完成。
  - LO 自动视觉转换仍受已登记独立 SIGABRT/发布工具债务约束；本课题验证的是 Pandoc 编辑链、
    OOXML 包和重开 HTML，不宣称 LO 发布债务已解决。
  - 通用媒体缓存淘汰/容量策略、图片移动、项目图片导入与代理图片操作继续由后续课题处理。

## 15. 人类闭合决定

- 2026-07-28：人类指示进入下一课题，接受本课题以“媒体资源、稳定身份、尺寸恢复、缺失资源阻断和
  真实 Pandoc/OOXML 往返已完成自动验证”为闭合边界。
- 本决定不把未执行的 Tauri/WKWebView 人工操作写成已验证；真实拖拽缩放、文档流内移动及其保存重开
  验收转入后继 `BDOCX-IMAGE-INTERACTION-01`。
- 项目图片导入、代理图片插入/删除、Diff 展示和浮动/绝对定位仍不属于本课题闭合范围。
- final_state：`CLOSED`。
