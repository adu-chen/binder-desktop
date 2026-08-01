# Issue Trace — BDocx 图片组合编辑与受控导入

## 1. 用户可见症状

- 同一图片的“缩放并居中”被代理拆成两个基于同一旧快照的 Diff；第一条生效后第二条失效，
  最终错误被“重试”覆盖为 `DE_OPEN_DOC_REQUIRES_LOGICAL_STATE`。
- 项目内非图片根目录或 workspace 外图片无法通过拖放进入 BDocx；代理只能要求用户移动源文件，
  可能破坏其他引用。
- 图片资产不可用时，代理已经诚实说明阻断，回合仍追加 `AG_EDIT_TOOL_REQUIRED`。
- 代理只能读取图片元数据和文档语境，但提示词没有充分约束其不得把语境推断表述为视觉识别。

## 2. 入口点

- `propose_edit(image_ops)`、`list_images`、代理工具循环。
- `Editor` pending ImageOperation effect 与 Tauri Webview 文件拖放事件。
- `DiffCard` error 操作。
- `doc.import_project_image` / `import_image_asset` 受控图片导入。

## 3. 相关规则

复用 `BR-SYS-DATA-006`、`BR-AG-STATE-002`、`BR-AG-DATA-001`、
`BR-DE-DATA-002`、`BR-DE-STATE-001`、`BR-ED-DATA-001/003`、
`BR-ED-STATE-001`、`BR-DOC-DATA-002/003`、`BR-WS-DATA-001`、
`BR-WS-PERSIST-001` 与既有路径/副作用约束；不新增数据库规则或状态机。

## 4. 相关链路

- `AG-TOOL → DE-DIFF → ED-EDIT`
- `WKWebView DOM DragEvent（项目 DocRef / Finder Files）→ DOC media import → ED-EDIT → DOC-CONVERT
  → WS-SAVE`
- `WS-FS → AG-TOOL → ProjectImageAsset`

## 5. 生产者块

- BDocxCapabilityManifest 生产 ImageOperation 形状与提示词声明。
- `list_images` 生产稳定图片快照、相邻文本摘要与项目资产。
- DOC 图片资源层生产导入后的 assetId/sourcePath/sourceHash/mediaRef/displaySrc。
- Editor 生产单事务图片 LogicalState。

## 6. 消费者块

- `image_ops_payload` 与 `applyImageOperations` 消费组合操作。
- DiffCard/deService 消费错误态和裁决意图。
- Editor 消费拖放路径与导入结果。
- 代理工具循环消费 `edit_blocked` 和图片资产准备结果。

## 7. 路径上的空规则块

未发现必须新增静态规则。新增导入命令与工具复用 ProjectImageAsset、ImageOperation 和既有
workspace 路径/原子写入边界；不创建新 TERM。

## 8. boundary 假设

- 同一既有图片允许至多一个 resize、一个 layout、一个 move；delete 独占。
- 组合批次只校验一次原始 fingerprint，并在单一事务中全部成功或零应用。
- 拖入是用户明确文件授权，但持久 LogicalState 不保存绝对路径。
- 非受控根来源默认复制而非移动；撤销文档节点不删除项目资产。
- 错误态不能以接受动作伪装重试，也不能覆盖更精确的首要错误。

## 9. 运行时假设

- WKWebView DOM DragEvent 提供项目相对 DocRef 或 Finder File 字节与客户端坐标；本地绝对路径不得
  进入 IPC 或 LogicalState。
- 当前活动文档必须是 BDocx HTML 编辑会话，且拖放点位于编辑器纸张范围内。
- PNG/JPEG 以扩展名与 magic 双校验；符号链接、目录和其他格式拒绝。
- 项目根目录可由 `.binder/project_assets.json` 配置；默认根为 `images`。

## 10. 疑似不变量破坏

- 两条旧快照 Diff 串行应用导致第二条失效。
- error 卡片调用 acceptDiff，导致根因被泛化错误覆盖。
- 拒绝较早图片 Diff 可能覆盖后续候选或人工编辑。
- 拖放直接保存 `file://`/绝对路径会在重启或保存时失效。
- 自动移动源图片会破坏 workspace 中其他引用。
- 没有可执行提案时，编辑意图门禁误把诚实阻断当成模型违规。

## 11. 验证计划

- Vitest：同图 resize+layout+move、delete 独占、批原子；DiffCard 错误动作；拖放定位、导入成功/
  失败、脏态、撤销；工具结果投影。
- Rust：组合 ImageOperation 预校验；项目内/外导入、复用、复制、去重、重名、magic、符号链接、
  源文件保护；list_images 相邻文本；edit_blocked 回合终止。
- `npx tsc --noEmit`、专项 Vitest、专项 Cargo test、`cargo fmt --check`、`cargo check --lib`、
  `npm run governance:generate`、`npm run governance:audit`、`git diff --check`。

## Scope Lock

- scope_id：`BDOCX-IMAGE-COMPOSITE-IMPORT-01`
- task_type：`CODE_CHANGE`
- confirmation：人类已将 `.governance/active_scope.json.confirmation` 设为 `YES`。
- allowed_files / forbidden_files：以 `.governance/active_scope.json` 为准。
- 当前状态：`READY`。

## 实施结果

- 同一图片允许单批各一个 resize/layout/move，前后端均基于同一原始 fingerprint 预校验；ED 按
  resize→layout→move 固定顺序在单一 ProseMirror 事务执行，delete 独占，重复操作整批拒绝。
- 未应用 error DiffCard 不再调用 acceptDiff 伪装重试；保留 `ED_IMAGE_SNAPSHOT_STALE` 等首要错误，
  只提示重新生成/关闭。仅已应用但持久化失败的卡片保留重试。
- 新增 `doc.import_project_image` 与系统文件拖入：受控根内复用，其他来源复制到首个 imageRoot 的
  `imported/`；校验普通文件、大小、PNG/JPEG magic、SHA-256、去重和重名，源文件保持不变。
- BDocx 工具栏禁用不可持久的远程 URL 插图入口；拖入图片以稳定 Block before/after 锚插入独立段落，
  多图片单事务、可撤销并进入既有脏态/保存链。
- 新增代理 `import_image_asset` 与 `edit_blocked`；诚实阻断可作为编辑回合合法终点，不再追加
  `AG_EDIT_TOOL_REQUIRED`。`list_images` 增相邻文本定位，并明确元数据不等于视觉识别。
- BDocxCapabilityManifest 升至 1.4.0，增加同图批兼容/独占约束；五份接口协议同步。

## 验证结果

- `npx tsc --noEmit`：通过。
- 专项 Vitest：7 文件、75 项通过。
- Rust：`tool_registry` 5 项、`tool_exec` 10 项、`bdocx_capabilities` 7 项、
  `structured_edit_blocked...` 1 项、图片链 26 项全部通过。
- `cargo check --lib`、`cargo fmt --check`、`git diff --check`：通过。
- `npm run governance:generate`：成功，ADUS 已刷新。
- `npm run governance:audit`：`status=passed`，无 governance finding。
- 相关语义候选复核：DiffCard 的 structured ImageOperation 审阅边界与 BR-DE-DATA-002 一致；
  ImageOperation 单事务边界与 BR-ED-DATA-001/003、BR-ED-STATE-001 一致；项目图片复制/物化边界与
  BR-DOC-DATA-002/003、BR-WS-DATA-001 一致；未发现设计—代码语义冲突。

## 未解决项

- 全量 `cargo test 'ag::' --lib` 有 1 个本课题前已存在且位于 Scope 禁止文件的失败：
  `ag::blocks::tests::html_replace_escapes_special_chars`。当前实现未转义特殊字符；本课题未修改、
  未绕过该测试，建议单独建题处理。
- WKWebView 真机系统拖入、项目文件树拖入、截图/文件粘贴、文档内图片复制粘贴，以及随后撤销、保存
  重开仍需人工验收；自动测试不能替代真实系统 DragDropEvent 与 ClipboardEvent。

## 2026-07-30 人工图片入口闭合追加

- 修复含图片选区被 SelectionRef 叶子占位字符接管的问题；图片复制/拖动交还 ProseMirror 富剪贴板，
  BlockId/ImageId 插件继续负责复制实例去重。
- 项目文件树 DocRef 拖入由 ED DOM drop 消费；只允许 PNG/JPEG 规范 workspace 相对路径，后端拒绝
  `.`/`..` 等非规范组件，再复用既有项目图片导入与物化链。
- 剪贴板 PNG/JPEG File 或不含文字的纯图片 data URL 经同一 `doc.import_project_image` 进入
  `images/imported`；后端重新校验 base64、大小、媒体类型、magic、SHA-256 和重复内容。远程图片与
  混合文字 HTML 图片拒绝，临时 data URL 不进入 LogicalState。
- Tauri 系统拖放补进入态反馈；坐标转换、监听注册等异常进入用户可见错误，不再只写控制台。
- 验证：`npx tsc --noEmit` 通过；图片入口/操作/媒体专项 Vitest 7 文件 57 项通过；
  `doc::image_media::tests` 8 项通过；`cargo check --lib`、`cargo fmt --check` 通过；
  `npm run governance:generate` 成功、`npm run governance:audit` 为 passed；`npx vite build` 成功并刷新
  包含人工图片入口的前端产物。

## 2026-07-30 拖放运行时冲突修复

- 人工复测确认复制粘贴成功而拖动无效，定位到拖放入口而非 DOC 媒体/LogicalState 下游：
  Tauri 窗口默认启用内部 drag-drop handler，而文件树 DocRef、ProseMirror 图片移动依赖 HTML5 DOM
  DragEvent；两套机制在 WKWebView 中不能作为并行入口。
- `tauri.conf.json` 显式关闭内部 drag-drop handler。纸张容器在捕获阶段成为统一入口：
  项目文件树 DocRef 走相对路径导入，Finder `Files` 读取 PNG/JPEG 字节并复用剪贴板 data URL 导入；
  文档内图片/富文本拖动不被接管，继续交给 ProseMirror。
- 外部图片 drop 在捕获阶段阻止 ProseMirror 把文件 URI/路径误解析为正文；混合文件整批拒绝。
  精确坐标无 Block 命中时按纵向距离选择最近稳定 Block，覆盖纸张留白、分页间隙和末页补白。
- 自动验证：`npx tsc --noEmit` 通过；图片拖放/操作/媒体相关 Vitest 7 文件 65 项通过；
  `npx vite build` 与 `git diff --check` 通过；Tauri 开发应用按新窗口配置编译并启动成功。
- Computer Use 服务在本会话不可用，无法替代人类执行 Finder/项目文件树/文档内图片三类真实鼠标拖放；
  该 WKWebView 人工矩阵仍是本课题最终验收项，不以 jsdom 测试冒充完成。

## 人类闭合决定

- 2026-07-30：人类确认真实应用内拖动插入图片已实现；保存、关闭、重新编译打开后图片读取正常。
- 闭合审计确认 manifest、提示词投影、工具 Schema/描述、执行前校验、组合 ImageOperation、结构化
  Diff、拒绝恢复、保存与 DOCX 重开链路同源；同步修正了需求/设计中的历史排除项及 E2E 的旧单操作
  不变量。
- 仍不支持图片视觉识别/OCR、文字环绕、浮动/绝对定位、任意坐标布局、裁剪、旋转或独立高度；这些是
  明确排除能力，不是本课题遗漏。
- 当前状态：`READY/CLOSED`。

## 闭合验证

- `npx tsc --noEmit`：通过。
- 图片代理/Diff/ED 专项 Vitest：7 文件、97 项通过，覆盖同图组合操作、过期零应用、拒绝全文恢复、
  DiffCard 五类操作、拖放锚与内部图片拖动分流。
- Rust：`tool_registry` 5 项、`bdocx_capabilities` 7 项、图片链 26 项通过，覆盖清单/提示词/Schema/
  工具描述、静态与动态校验、受控导入、结构化阻断、真实 Pandoc/OOXML 保存重开及失败不覆盖原文档。
- `cargo fmt --check`、`cargo check --lib`、`npx vite build`、`git diff --check`：通过；仅保留既有
  LibreOffice/AG dead_code warning 与 Vite 大 chunk 提示。
- `npm run governance:generate`：213 个 @GOV 块，派生索引无变化；`npm run governance:audit`：
  `status=passed`，`governance_findings=[]`。ImageOperation DiffCard 语义候选与
  BR-DE-DATA-002/TERM-DE-003/TERM-ED-008 一致。
