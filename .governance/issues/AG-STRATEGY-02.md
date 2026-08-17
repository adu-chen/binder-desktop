# Issue Trace — AG-STRATEGY-02：运行时注入同步 + 新建 b-docx 指导

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-11

## 1. 用户可见症状

真实测试「新建一个word文档，把当前寓言拷贝到新文档内，并优化排版」：意图判 CreateDocument 正确、propose_edit 已全量授权、validate 已取消 targetScope 硬锁，但模型读完文档后**未调用 propose_edit 直接以文字结束** → `AG_EDIT_TOOL_REQUIRED` 拦截，任务失败。

根因：模型感知层（运行时注入 + prompt 指导）未随 AG-STRATEGY-01 同步——`parameterConstraints` 仍注入 `proposal_target_must_match_intent` 旧硬约束，且 prompt 对"新建 b-docx（无当前 BDocx 能力声明时）"指导不足，模型推断无法创建富文档而自我放弃。

## 2. 入口点

- `src-tauri/src/ag/tool_registry.rs`：parameterConstraints 移除 `proposal_target_must_match_intent`（语义已下沉调用时刻），替换为提示性 `proposal_applies_on_user_confirm`。
- `src-tauri/src/ag/prompts.rs`：明确新建 word 文档（.b-docx）= propose_edit(path=新路径.b-docx, base=empty, operation=replace_document, content=富 HTML)，无需额外 BDocx 能力声明。

## 3. 相关规则

- BR-AG-DATA-003 / BR-AG-STATE-002：目标安全由调用时刻校验 + DiffCard 确认承担（AG-STRATEGY-01 已改 validate）；本 scope 同步运行时注入与 prompt 语义。
- 无规则正文改动。

## 4. 相关链路

runtime_control（CapabilityGrant 序列化）→ prompt 注入 → 模型决策 propose_edit → 调用时刻校验 + DiffCard。

## 5. 生产者块

无新增 RULE/CHAIN。

## 6. 消费者块

- tool_registry.rs tests（parameterConstraints 断言）。
- prompts.rs tests（新建指导断言，若有）。

## 7. 空规则块

未发现。

## 8. boundary 假设

- `lifecycle_assumption=`：targetScope 字段保留仅作预期目标提示（prompt/前端），不再表达硬边界。
- `document_snapshot=`：新建 b-docx 走 propose_edit replace_document + base=empty（AG-TASK-01 已验证路径）。

## 9. 运行时假设

- 模型读到的 parameterConstraints 不含"目标必须匹配意图"式硬约束；写入（含跨文档/新建）经 DiffCard 确认。
- prompt 指导：新建 word 文档直接用 propose_edit 新路径 + base=empty + replace_document（content 富 HTML：h1/p/ul 等）。

## 10. 疑似不变量破坏

- 移除"proposal_target_must_match_intent"注入 → 模型对新目标的保守度下降 → 由 DiffCard 确认兜底（安全不降级，转移非删除）。

## 11. 版本历史

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-08-11 | v1.0 | 初始版本：运行时注入同步 + 新建 b-docx 指导 |

## 12. Task Runtime Report — AG-STRATEGY-02

### 12.1 任务结果

- 状态：`READY`。

### 12.2 实际变更

- **tool_registry.rs**：`parameter_constraints` 第三条 `proposal_target_must_match_intent` → `proposal_applies_on_user_confirm`（提示性：写入经 DiffCard 确认，跨文档/新建目标均放行）；targetScope 注释保持"预期目标提示"。
- **prompts.rs**：能力边界增补——"新建 word 文档（.b-docx）= propose_edit(path=新路径.b-docx, base={sessionVersion:0,contentHash:""}, operation=replace_document, content=富 HTML（h1/p/ul 等）)，无需额外 BDocx 能力声明；当前文档无 BDocx 声明不影响新建富文档"。
- **（修复补丁 2026-08-11）prompts.rs 新建路径后缀修正**：`.b-docx` → `.docx`（含"路径后缀必须为 .docx，勿用 .b-docx 等其他后缀；接受后编辑器打开→保存转换为标准 Word 文档"）。原因：`.b-docx` 非项目合法后缀，apply_diff 原样写 HTML 字节产出畸形文件（实测小马过河.b-docx / 萤火虫与月亮.b-docx 已清理）。

### 12.3 验证结果

| 验证 | 结果 |
|---|---|
| `cargo test`（Rust 全量） | PASS：315 + 5 + 4，0 失败 |
| `npm run test:related -- --files [tool_registry/prompts]` | PASS |
| `npm run governance:generate` / `governance:audit` | PASS：0 warnings |
| `git diff --check` | PASS |

### 12.4 语义复核

- 模型不再读到"目标必须匹配意图"硬约束；新建 b-docx 有明确操作指导 → 读完当前文档后可直接 propose_edit 新路径（DiffCard 确认兜底）。

### 12.5 未解决项

- 真实 G5 运行验证（Tauri + 真实 provider）待用户复测。

### 12.6 READY 自检

- 范围/验证/证据/完整/诚实/留痕：通过；无 TODO/桩/跳过。
