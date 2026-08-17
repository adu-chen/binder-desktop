# Issue Trace — AG-DOCX-RO-01：原生 .docx 只读能力

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-11

## 1. 用户可见症状

用户「项目里有原文档，找一下对比」→ 原文档为原生 .docx（非 b-docx）→ `read_doc` 返回 DOC_NOT_BDOCX → 无法完成"找原文档复原/对比"核心工作流。AI 只能靠文件名推断或请求用户粘贴。

## 2. 入口点

- `src-tauri/src/ag/blocks.rs`：`read_editable_snapshot`（原生 docx 提前拒绝）、`editable_kind`（docx 恒标 "bdocx"）。
- `src-tauri/src/ag/prompts.rs`：SYSTEM_PROMPT 能力边界补充原生 docx 只读说明。
- `src-tauri/src/ag/tool_exec.rs`：只读链路消费（无需改动，确认即可）。

## 3. 相关规则

- `BR-DOC-DATA-002/003`：docx 转换链（pandoc docx_to_html 只读源文件、写临时 work_dir）已标注，复用不新增。
- `X-FORBID-004`：只读工具限 workspace 内——原生 docx 只读在其内，语义不变。
- `BR-AG-STATE-002`：工具须在本轮 allowed_tools 内；原生 docx 不在 target_scope → propose_edit 拒绝（结构性）。
- `BR-AG-DATA-005`：15MB 转换上限适用原生 docx 只读，一致。

## 4. 相关链路

AG-TOOL（只读执行）→ AG-TURN（能力声明注入）。DOC-CONVERT 转换链复用。

## 5. 生产者块

- BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-003（docx_to_html 转换链，复用不改）。
- 无新增 RULE/CHAIN。

## 6. 消费者块

- tool_exec `read_doc`/`list_blocks`/`search_text` 消费快照（editable_kind/source/blocks）。
- bdocx_capabilities 判定（editable_kind=="bdocx" 才注入编辑能力声明）——原生 docx 不注入，安全关键点。

## 7. 空规则块

未发现；audit 若产生 BLOCK_UNANNOTATED 在本 Scope 补标或 triage。

## 8. boundary 假设

- `lifecycle_assumption=`：read_editable_snapshot 纯函数、无副作用（pandoc 只读源文件、写临时 work_dir，与 b-docx 读取一致）。
- `document_snapshot=`：原生 docx 转换后 content 为 HTML 投影，editable_kind≠"bdocx"、capabilities=[]（只读）。

## 9. 运行时假设

- pandoc 依赖存在（b-docx 读取已依赖，非新增）。
- 原生 docx 只读复用 work_dir 缓存（content_hash 命名），重复读取不重复转换。
- 15MB 上限与 b-docx 一致（MAX_EDIT_HTML_BYTES）。

## 10. 疑似不变量破坏

- 中风险：editable_kind 语义变化（原生 docx "html" 而非 "bdocx"）——影响面仅快照消费（bdocx_capabilities 排除注入、能力声明不注入），编辑器路径 4 处 DOC_NOT_BDOCX 保持拒绝，编辑语义不变。
- 未发现需改规则正文的情况（转换链已标注、只读范围语义不变、能力注入由 editable_kind 判定已有）。

## 11. 验证计划

- 单元：read_editable_snapshot 原生 docx → editable_kind≠"bdocx"、source="doc_convert_readonly"、capabilities 空；bdocx_capabilities 对 editable_kind≠bdocx 排除注入。
- 集成：read_doc 原生 .docx 返回块（无 DOC_NOT_BDOCX）；propose_edit 原生 .docx 拒绝（不在 target_scope）。
- 回归：b-docx 编辑链路不变（is_bdocx 仍走 bdocx 分支）。
- 治理：cargo test / test:related / governance:generate / governance:audit。

## 文件范围（Scope Lock 输入）

允许修改：

- `src-tauri/src/ag/blocks.rs`
- `src-tauri/src/ag/prompts.rs`
- `docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md`
- `docs/01_modules/AG/A-AG-M-T-02_提示词架构设计.md`
- `docs/00_core/A-CORE-C-R-01_ADUS.md`（仅 generate 派生）
- `.governance/issues/AG-DOCX-RO-01.md`、`.governance/active_scope.json`

禁止修改：

- ADU/ADP/AGENTS/CLAUDE/APC、`governance.config.json`、`.governance/test-governance.json`、`scripts/**`
- 前端 `src/**`、`src-tauri/tests/**`、`turn.rs`、`tool_registry.rs`、`lib.rs` 编辑器路径（4 处 DOC_NOT_BDOCX 保持拒绝）、ent/cbt/ast/kb/tpl
- 不在允许列表默认禁止；需扩展按 ADP SC-01 停止请求。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-08-11 | v1.0 | 初始版本：原生 .docx 只读能力（复用 pandoc 转换链，editable_kind≠bdocx 结构性排除写入） |

## 12. Task Runtime Report — AG-DOCX-RO-01

### 12.1 任务结果

- 状态：`READY`。
- 完成内容：模型具备只读原生 .docx 能力——复用既有 pandoc DOC 转换链，原生 docx 进入只读快照（`source=doc_convert_readonly`、`editable_kind` 覆盖 "html"≠bdocx、capabilities=[]），结构性排除写入。用户「找原文档对比/复原」工作流不再 DOC_NOT_BDOCX。
- 范围：仅 blocks.rs / prompts.rs 与两份设计文档；编辑器路径 4 处 DOC_NOT_BDOCX 保持拒绝；规则正文零改动、前端零改动。

### 12.2 实际变更

- `src-tauri/src/ag/blocks.rs`：
  - `read_editable_snapshot` docx 分支放行原生 .docx（移除 DOC_NOT_BDOCX 拒绝），走 `docx_to_html` 转换链；原生 docx 标记 `source=doc_convert_readonly`。
  - `snapshot_from_content` 新增 `editable_kind_override` 参数；原生 docx 传入 `Some("html")` → editable_kind≠"bdocx" → 不注入 BDocx 编辑能力声明（bdocx_capabilities 排除）、块 capabilities=[]（无 replace_node）→ 结构性只读。
  - 新增测试 `native_docx_reads_as_readonly_projection_without_bdocx_capabilities`（pandoc 可用时生成真实 docx 验证只读投影）。
- `src-tauri/src/ag/prompts.rs`：能力边界补充「原生 .docx 可只读读取（read_doc/list_blocks/search_text），不可作为编辑目标；编辑需先创建 b-docx 副本」。
- 设计文档：M-T-01 §5.1（v2.32）、M-T-02 §4（v1.19）登记原生 docx 只读。

### 12.3 验证结果

| 验证 | 结果 |
|---|---|
| `cargo test --lib ag::blocks` | PASS：21 tests（含新原生 docx 只读用例） |
| `cargo test`（Rust 全量） | PASS：304 + 5 + 4 |
| `npm test`（前端全量） | PASS：26 files / 552 tests |
| `npm run governance:generate` | PASS：245 @GOV 块，92 assets / 865 cases |
| `npm run governance:audit` | PASS：0 findings |
| `git diff --check` | PASS |

### 12.4 语义复核

- `BR-DOC-DATA-002/003`：复用既有转换链（pandoc docx+styles→html），不新增解析器；源文件不变（只读写临时 work_dir）；15MB 上限一致。
- `X-FORBID-004`：只读限 workspace 内——原生 docx 只读在其内，语义不变。
- `BR-AG-STATE-002`：propose_edit 对原生 docx 因不在 target_scope 拒绝；只读工具仍须在 allowed_tools 内。
- 编辑器路径 4 处 DOC_NOT_BDOCX 保持拒绝——编辑仍需 b-docx，能力边界不变。
- 安全：三层结构性只读（无能力声明 / 无 replace_node / 不在 target_scope），无任何写入路径。

### 12.5 未解决项

- 无。真实 provider/Tauri 验证属后续阶段；dev 重定向日志 `/tmp/binder-audit.log` 可复测「找原文档对比」→ read_doc 原生 .docx 成功。

### 12.6 READY 自检

- 范围：通过；仅 Scope 允许列表内文件。
- 验证：通过；test:plan 所选命令全部实际通过。
- 证据：通过；自动化结果如上表。
- 完整：通过；无 TODO、桩、跳过测试（pandoc 不可用时测试条件跳过属环境守卫，与 edit_chain 既有测试一致）。
- 诚实：通过；只读路径验证为真实 docx 转换（pandoc_available 时执行）。
- 留痕：通过；未解决项为零，Scope Lock 已同步 READY。
