# Issue Trace — AG-INTENT-COMPOSITE-01：复合请求意图优先级（新建主导）

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-11

## 1. 用户可见症状

- 真实测试：用户请求「新建一个word文档，把当前寓言拷贝到新文档内，并优化排版」→ 意图被判为 Edit 当前文档（小马过河.txt），targetScope 锁死当前文件；模型 propose_edit 新路径被拒（target_scope_restricted）→ 无法新建文档。

## 2. 入口点

- `src-tauri/src/ag/context.rs`：`resolve_intent_result` 增加 Create 强词 + Document/Workspace 目标共现时的 Create 主导前置判断；`detect_verb` 的 Edit→Create 首命中机制保持不动。

## 3. 相关规则

- BR-AG-DATA-004：当前轮意图解析（动作-目标矩阵）。
- AG-INTENT-V2-01（单动作精度）、AG-INTENT-V3-01（乐观授权）保持语义不变——本修复只补齐**复合请求优先级**覆盖缺口。
- 无规则正文改动。

## 4. 相关链路

AG 意图层（resolve_intent_result）→ CapabilityGrant（targetScope）→ 工具循环（propose_edit 越界校验）。

## 5. 生产者块

无新增 RULE/CHAIN。

## 6. 消费者块

- `resolve_intent_result` 调用方（intent 注入）。
- context.rs 测试（复合意图用例）。

## 7. 空规则块

未发现。

## 8. boundary 假设

- `lifecycle_assumption=`：仅当 Create 强词（创建/新建/生成/建立/制作）与 Document/Workspace 目标共现时 Create 主导；无 Create 词的存量编辑（优化/整理/修改…）仍判 Edit。
- `document_snapshot=`：detect_verb 原返回逻辑（Edit→Create 首命中）保持，供既有单动作用例与调用不变。

## 9. 运行时假设

- 复合请求「新建…文档，并优化排版」→ CreateDocument（ProposeCreate，NewDocument target）。
- 单动作「创建md文档」「把标题改为红色」「优化这篇排版」行为与修复前一致（回归用例验证）。

## 10. 疑似不变量破坏

- 无；新前置分支仅作用于"Create 强词 + Document/Workspace 目标"共现，语义更强的主导动作优先。

## 11. 版本历史

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-08-11 | v1.0 | 初始版本：复合意图 Create 主导前置 + 复合意图测试 |

## 12. Task Runtime Report — AG-INTENT-COMPOSITE-01

### 12.1 任务结果

- 状态：`READY`。
- 完成内容：`resolve_intent_result` 新增 `create_requested` 前置判定（Create 强词 + Document/Workspace 目标共现 → CreateDocument），优先于 `verb == Edit` 分支；补复合意图回归测试（复合/存量编辑/单动作三用例）。

### 12.2 实际变更

- **context.rs**：在 `mutation_forbidden` 之后、`verb == Edit` 分支之前插入 `create_requested` 分支（L334 前）；`create_requested` = 非否定 ∧ 文本含 CREATE_VERBS_STRONG ∧ target ∈ {Document, Workspace}。
- **context.rs tests**：`intent_composite_create_overrides_edit_verb`——①「新建一个word文档，把当前寓言拷贝到新文档内，并优化排版」→ CreateDocument/NewDocument；②「优化这篇的排版」→ Edit（存量语义保持）；③「创建md文档」→ CreateDocument（V2 语义保持）。

### 12.3 验证结果

| 验证 | 结果 |
|---|---|
| `cargo test --lib ag::context` | PASS：14（含新复合意图用例） |
| `cargo test`（Rust 全量） | PASS：312 + 5 + 4 |
| `npm run test:related -- --files [context.rs]` | PASS：前端 186，0 findings |
| `npm run governance:generate` | PASS：92 assets / 874 cases（+1） |
| `npm run governance:audit` | PASS：0 findings / 0 warnings |
| `git diff --check` | PASS |

### 12.4 语义复核

- 复合请求主导动作仲裁补齐；detect_verb 首命中机制与 V2/V3 语义不变。
- 无规则正文改动。

### 12.5 未解决项

- 复合意图的其他组合（如「分析+新建」）未单独覆盖；当前修复仅针对本次真实症状（Create+Edit），其余组合由后续观察补齐。

### 12.6 READY 自检

- 范围/验证/证据/完整/诚实/留痕：通过；无 TODO/桩/跳过。
