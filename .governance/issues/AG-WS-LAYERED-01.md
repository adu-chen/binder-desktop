# Issue Trace — AG-WS-LAYERED-01：模型分层感知工作区

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-11

## 1. 用户可见症状

- 新建文档（CreateDocument，上下文 28 tokens）时模型被迫 `list_docs` → 工具返回**整个工作区文件树**（200+ 项，~30KB）→ 全量注入下一轮 prompt → 请求膨胀（~13.6k tokens）→ DeepSeek 长流挂起 9 分钟 → `GW_STREAM_ERR`，任务中断。日志证据：t-1786464551036-1 / t-1786465233848-3（第 1 轮流式成功、第 2 轮注入全量文件树后失败）。
- 违反原则：**全量树形结构 + 内容一同进入上下文**。内容（正文）本应只经 read_doc 按需获取。

## 2. 入口点

- `src-tauri/src/workspace/listing.rs`：新增 `list_docs_overview`（L0 概览 + 豁免全量）与 `list_docs_children`（L1 一层展开）；保留 `list_docs` 全量语义（前端 FileTree / search_text 使用）。
- `src-tauri/src/ag/tool_exec.rs`：AG `list_docs` 工具——无参数 → 概览（含豁免判定）；`path` 参数 → 直接子项。
- `src-tauri/src/ag/tool_registry.rs`：list_docs 参数 schema 加可选 `path`。
- `src-tauri/src/ag/prompts.rs`：分层感知引导（概览→展开→内容；CreateDocument 引导）。

## 3. 相关规则

- BR-WS-DATA-001：仅枚举活动 workspace root 下子树，跳过隐藏项。
- BR-AG-DATA-005：工具结果上下文预算（L0 概览 <200 token；children 一层）。
- AG-CTX-01 语义延伸：list_blocks/read_doc 已轻量投影，list_docs 补齐。
- 无规则正文改动。

## 4. 相关链路

AG-TOOL（list_docs 分层）→ AG 工具循环（工具结果注入）→ prompt（概览/展开/内容三层引导）→ DeepSeek（小请求，避免长流挂起）。

## 5. 生产者块

无新增 RULE/CHAIN。

## 6. 消费者块

- AG `list_docs` 工具调用方（模型）。
- prompt 分层引导。
- listing.rs 测试（overview/children/豁免边界）。

## 7. 空规则块

未发现；audit 若产生 BLOCK_UNANNOTATED 在本 Scope 补标或 triage。

## 8. boundary 假设

- `lifecycle_assumption=`：L0 概览只含索引（路径/类型/计数），**永不携带正文**；正文只经 read_doc。
- `document_snapshot=`：豁免判定（文件数 ≤30 且树序列化 ≤4KB）在服务端一次 walk 完成。

## 9. 运行时假设

- `list_docs()` 默认返回 overview（或豁免 full）；`list_docs(path=...)` 返回该目录直接子项（一层，不递归）。
- 前端 `ws.list_docs` 路由与 search_text 保持全量语义不变（复用原 `list_docs`）。
- 豁免仅服务端判定，模型不可感知（prompt 说明"工作区较小时 list_docs() 直接给出全部文件"）。

## 10. 疑似不变量破坏

- 模型对超大工作区的感知完整度下降（概览不含深目录内容）→ 由 search_text（200 条/160 字上限）内容检索兜底，三层互补，不视为破坏。

## 11. 版本历史

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-08-11 | v1.0 | 初始版本：list_docs 分层（L0 概览/豁免、L1 path 展开）+ prompt 分层引导 + 结果注入保护 |

## 12. Task Runtime Report — AG-WS-LAYERED-01

### 12.1 任务结果

- 状态：`READY`。
- 完成内容：模型分层感知工作区——`list_docs()` 概览（顶层+计数）/ 豁免全量（≤30 文件且 ≤4KB）/ `list_docs(path)` 一层展开；prompt 分层引导 + CreateDocument 专项；前端 `ws.list_docs` 与 search_text 全量语义不变。

### 12.2 实际变更

- **listing.rs**：新增 `list_docs_overview_json(root)`——一次全量 walk 后派生：mode=full 豁免（文件数 ≤30 且序列化 ≤4KB）或 mode=overview（top_level_dirs 带 file_count/docx_count + root_files + total_files，无 entries）；新增 `list_docs_children_json(root, path)`——校验目录存在/是目录（WS_LIST_DIR_MISSING / WS_LIST_DIR_NOT_DIR），返回一层直接子项（目录带子树 file_count）。`list_docs` 全量语义保留（前端 ws.list_docs / search_text 用）。
- **tool_exec.rs**：AG list_docs 解析可选 `path`——空 → overview/full；非空 → children。
- **tool_registry.rs**：list_docs 参数 schema 加可选 `path` + 描述更新（概览/展开/内容分离语义）。
- **prompts.rs**：能力边界段追加分层感知引导（先概览 → list_docs(path) 展开 → 正文 read_doc/search_text，列表永不回正文；小工作区豁免说明；CreateDocument 专项：概览判断落点后直接 propose_edit）。
- 测试：listing.rs +4（overview 豁免/overview 概览/children 一层/children 错误）；tool_exec.rs +1（分层 overview+children+错误码）。

### 12.3 验证结果

| 验证 | 结果 |
|---|---|
| `cargo test --lib workspace::listing` | PASS：10 |
| `cargo test --lib ag::tool_exec` | PASS：12 |
| `cargo test`（Rust 全量） | PASS：311 + 5 + 4 |
| `npm run test:related -- --files [4 个改动文件]` | PASS：前端 503 + Rust 全量，0 findings |
| `npm run governance:generate` | PASS：92 assets / 873 cases |
| `npm run governance:audit` | PASS：0 findings / 0 warnings |
| `git diff --check` | PASS |

### 12.4 语义复核

- 树枚举 = 索引（路径/类型/计数），内容只经 read_doc —— 满足「绝不出现全量树+内容同进上下文」（豁免仅限小工作区）。
- 前端 ws.list_docs / search_text 语义不变（X 无回归）。
- 无规则正文改动。

### 12.5 未解决项

- 结果注入上限截断（AG-CTX-01 预算边界）已存在；list_docs 分层后单次结果天然 <200 token（overview）或一层（children），无需新截断。超大目录一层子项仍可能较大 → 依赖既有注入预算兜底。

### 12.6 READY 自检

- 范围/验证/证据/完整/诚实/留痕：通过；无 TODO/桩/跳过。
