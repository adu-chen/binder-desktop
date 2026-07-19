# Issue Trace · ED-MARKDOWN-RICH-ROUNDTRIP-01

## 1. 用户可见症状

打开 `.md` 文档后，标题井号、列表符号、强调标记等 Markdown 源码字符直接显示，文档没有以标题、列表、引用、代码块、表格等终态样式呈现。

## 2. 入口点

- 文件打开入口：`src/services/edService.ts` 与 `src/store/edStore.ts`。
- 编辑器入口：`src/components/ed/Editor.tsx`。
- LogicalState 定位与替换入口：`src/services/edLocate.ts`。
- AG/DE 开放文档投影入口：`src/services/agService.ts`、`src/services/deService.ts`、`src-tauri/src/ag/blocks.rs`。
- 会话持久化入口：`src/services/workspaceService.ts`、`src-tauri/src/db/mod.rs`。

## 3. 相关规则

- `BR-ED-STATE-001`：LogicalState 修改必须经过 ED 受控入口。
- `BR-ED-DATA-002`：md/bdocx/html 统一逻辑模型与序列化边界。
- `BR-ED-DATA-005`：格式相关样式能力与 StyleOp 约束。
- `X-FORBID-005`：上层模块不得绕过 ED 直接操纵编辑器内部状态。
- `X-FORBID-008`：DiskState、LogicalState、DisplayState 单向数据流。
- `X-SHARED-001`：跨模块共享结构必须由 SYS 契约定义。
- 本任务新增 `BR-ED-DATA-006`：Markdown 解析、渲染与序列化规则。
- 本任务新增 `BR-ED-VERIFY-001`：Markdown 语义往返与无静默丢失规则。

## 4. 相关链路

继续复用 `ED-EDIT`：`open -> parse -> edit -> serialize -> save`。不新增规则链，不新增状态机；在 `SM-ED-01` 内增加 Markdown 格式分支、`serializeFail` 和有损保存 Guard。

跨模块关联链路：`AG-TURN -> AG-TOOL -> DE-DIFF -> ED-EDIT -> WS-SAVE`。

## 5. 生产者块

- `.md -> editFormat`：`src/store/edStore.ts`。
- DiskState 读取与会话装载：`src/services/edService.ts`。
- TipTap LogicalState 与序列化：`src/components/ed/Editor.tsx`、新增 `markdownCodec.ts`。
- OpenDocSnapshot：`src/services/agService.ts` 与 `src/services/workspaceService.ts`。

## 6. 消费者块

- ED 保存与定位：`src/services/edService.ts`、`src/services/edLocate.ts`。
- DE open-session 替换：`src/services/deService.ts`。
- AG block projection：`src-tauri/src/ag/blocks.rs`。
- 会话恢复：`src-tauri/src/db/mod.rs`、`src-tauri/src/lib.rs`。

## 7. 路径上的空规则块

新增 Markdown Codec 属于 ED-EDIT 的核心解析/序列化执行块，必须使用新增规则标注；纯语法 fixtures、样式声明及无业务判断的薄包装不新增 @GOV 块。

## 8. boundary 假设

- `editFormat` 表示磁盘解析/保存策略，不能继续把 Markdown 与纯文本合并为 `text`。
- `contentFormat` 表示 OpenDocSnapshot 内容编码；打开的 Markdown LogicalState 使用 ProseMirror 兼容 HTML 投影。
- 保存不得从 DisplayState 反推；必须从 LogicalState 经 Markdown Serializer 生成 DiskState。
- AG/DE 不得把 Markdown 源文件行号永久等同于 ProseMirror block index。

## 9. 运行时假设

- 支持语法范围为 CommonMark + GFM 的项目子集。
- 未编辑文档不写盘、不因打开关闭发生规范化重写。
- 编辑后保存允许规范化 Markdown，但支持范围内必须保持语义等价。
- 发现无法无损序列化的结构时必须阻止富文本保存并保留 dirty 状态，不能静默删除。
- `.txt` 继续沿用纯文本路径。

## 10. 疑似不变量破坏

- 当前 `.md -> text -> 每行 paragraph -> getText()` 破坏了 Markdown 的结构语义。
- 仅修改 Editor 渲染会导致 AG/DE block projection、会话恢复与保存格式错配。
- 若允许字体、字号、颜色等无法由目标 Markdown 方言表达的 StyleOp，保存后会产生静默样式丢失。
- 如果 parse/serialize 失败后仍清除 dirty 或写盘，会破坏 DiskState/LogicalState 边界。

## 11. 验证计划

- Codec：H1-H6、强调、删除线、列表/任务列表、引用、代码、链接、图片、表格、分隔线、转义、中英文和嵌套结构。
- 语义往返：支持子集执行 `parse -> serialize -> parse` 等价检查。
- 数据保护：未编辑不改写；不支持结构禁止静默保存；序列化失败保留 dirty。
- 编辑器：DOM 中存在对应标题/列表/引用节点，普通 Markdown 标记不作为正文展示。
- 回归：`.txt` 行为不变；AG/DE、search/locate、会话恢复与 active/inactive open session 正确。
- 命令：相关 Vitest、`npm run build`、Rust 测试、`npm run governance:generate`、`npm run governance:audit`。

## Expected Behavior

- `.md` 打开后默认进入结构化终态编辑：标题、列表、引用、代码块、表格等显示为对应编辑节点，不显示 Markdown 控制符号。
- 支持子集保存满足语义往返；源码保护或序列化失败时保留原文、保持 dirty，禁止静默写盘。
- `.txt`、HTML、b-docx 既有路径不被破坏；AG/DE/会话恢复使用显式 `contentFormat`。

## Validation Commands

```bash
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
npm run governance:generate
npm run governance:audit -- --issue .governance/issues/ED-MARKDOWN-RICH-ROUNDTRIP-01.md
```

## Scope Lock

```yaml
task_type: CODE_CHANGE
scope_id: ED-MARKDOWN-RICH-ROUNDTRIP-01
allowed_files:
  - .governance/active_scope.json
  - .governance/issues/ED-MARKDOWN-RICH-ROUNDTRIP-01.md
  - package.json
  - package-lock.json
  - docs/00_core/A-CORE-C-R-01_ADUS.md
  - docs/00_core/A-CORE-C-T-01_三态模型.md
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md
  - docs/01_modules/ED/A-ED-M-D-01_编辑器功能需求描述.md
  - docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md
  - docs/01_modules/ED/A-ED-M-S-01_编辑器状态机设计.md
  - docs/01_modules/ED/A-ED-I-P-01_编辑器接口协议.md
  - docs/01_modules/ED/A-ED-X-L-02_编辑器专项优化计划.md
  - docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md
  - docs/01_modules/AG/A-AG-I-P-01_代理接口协议.md
  - docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md
  - docs/01_modules/DE/A-DE-I-P-01_DiffEdit接口协议.md
  - docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md
  - src/components/ed/Editor.tsx
  - src/components/ed/Editor.test.tsx
  - src/components/ed/extensions/index.ts
  - src/components/ed/markdown/**
  - src/styles.css
  - src/store/edStore.ts
  - src/store/edStore.test.ts
  - src/services/edService.ts
  - src/services/edService.test.ts
  - src/services/edLocate.ts
  - src/services/edLocate.test.ts
  - src/services/deService.ts
  - src/services/deService.test.ts
  - src/services/agService.ts
  - src/services/agService.test.ts
  - src/services/workspaceService.ts
  - src/services/workspaceService.test.ts
  - src-tauri/src/ag/blocks.rs
  - src-tauri/src/ag/tool_exec.rs
  - src-tauri/src/db/mod.rs
  - src-tauri/src/lib.rs
forbidden_files:
  - ADU.md
  - ADP.md
  - AGENTS.md
  - docs/00_core/A-CORE-C-P-01_APC.md
confirmation: YES
scope_reason: 人类确认在既有 ED-EDIT 链内实现 Markdown 终态富文本往返，并同步 ED/SYS/AG/DE 契约与验证。
```

## 前置裁决

- 2026-07-03 的“Markdown 按纯文本处理”裁决由本任务替代，但保留历史原因与记录。
- 默认采用 CommonMark + GFM；第一阶段对无法无损表达的非标准语法采取源码保护/阻止富文本保存策略。
- 不新增 TERM；`MarkdownCodec`、`contentFormat` 为技术实现词汇，代码不新增 `term_ref`。
- 官方 `@tiptap/markdown` 仅在实际扩展栈与 fixtures 验证通过后纳入正式依赖。
