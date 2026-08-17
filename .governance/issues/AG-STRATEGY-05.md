# Issue Trace — AG-STRATEGY-05：文档代理编辑的事务安全、策略一致性与可观察性

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-12

## 1. 用户可见症状

「丰富故事/排版/节奏」类整篇重写请求产生了“建议卡和回复称有 4 个标题，编辑器中标题全消失且残留 4 段断句”的结果。未裁剪运行审计和编辑器 LogicalState 快照证明：模型提交的 proposal 确含 4 个标题插入和 5 段替换，但前端预览将其部分错误应用。

## 2. 根因

1. **执行器（P0）**：每节的 `insert_before(anchor)` 与 `replace_node(anchor)` 使用同一旧坐标。事务先插标题，再按旧节点大小 replace，覆盖标题并留下旧段尾部；函数未验证最终 LogicalState。
2. **回滚（P0）**：insert/append 的 `beforeNode` 是空对象，但 reverse 按节点解析它；接受和拒绝的 reverse preflight 因此会失败。
3. **提示词路由（P1）**：静态 prompt 按改动范围引导整篇改写使用 `replace_document`，运行时 BDocx declaration、工具 schema 和 manifest 却仍强制所有富文档使用 `block_ops`。动态输入以 system role 注入，实际覆盖静态引导。
4. **状态可观察性（P1）**：`propose_edit` 只确认 proposal 已生成；编辑器稍后异步预览，结果未回传。模型/卡片文案将“建议内容”误说成“编辑器已经完成”。

## 3. 入口点

- `src/services/edBlockOps.ts`：同锚点事务规划、reverse、postcondition。
- `src/components/ed/Editor.tsx`、`src/store/deStore.ts`、`src/services/deService.ts`：预览应用结果、错误投影、接受/拒绝安全性。
- `src-tauri/src/ag/prompts.rs`、`src-tauri/src/lib.rs`、`src-tauri/src/ag/bdocx_capabilities.rs`、`src-tauri/src/ag/tool_registry.rs`、`src-tauri/resources/bdocx-agent-capabilities.json`：一份一致的改动范围路由与预览文案。
- `src/services/agService.ts`、DiffCard 和相关测试：proposal / preview / accepted / saved 的显示与回归。

## 4. 相关规则

- BR-ED-DATA-001 / BR-ED-DATA-003：活动 LogicalState、图片/表格守卫不变。
- X-FORBID-006：仍只生成 Diff，用户裁决后才接受，保存后才落盘。
- 提示词文本、运行时能力声明和客户端状态投影是实现期契约；不新增或修改规则正文。

## 5. 生产者块

无新增 RULE/CHAIN。

## 6. 消费者块

- block_ops 执行器、编辑器草稿应用/回滚、DiffCard 状态、Agent proposal projection。
- prompt / capability manifest / backend route 的 Rust 测试。

## 7. 空规则块

未发现。

## 8. boundary 假设

- `lifecycle_assumption=`：pending proposal 可以显示为预览，但接受只确认保留、保存才落盘；任何未通过 postcondition 的预览不得写入 LogicalState。
- `document_snapshot=`：无表格、无图片的 BDocx 整篇改写可使用 replace_document；有表格或图片时必须走结构化操作。

## 9. 运行时假设

- 同锚点组合先完成全量校验，以单个锚点计划写入，最终节点的 blockId、类型和内容与 proposal 一致。
- insert/append reverse 按插入节点的稳定 blockId 删除，不能从空 beforeNode 重建。
- 模型只可把 proposal 叙述为“建议”；没有应用回执时不得称编辑器已修改。

## 10. 疑似不变量破坏

- 中风险：事务排序、insert/delete/replacement 组合可能影响列表嵌套和多锚点批次；须用最终 LogicalState 和 forward→reverse 双向断言覆盖。
- 中风险：动态 prompt、manifest 和 backend table/image guard 必须保持一致；任一层漂移都可能再把模型导向错误操作。
- 中风险：当前 dirty 草稿绝不能因失败恢复而静默覆盖；只允许无后续用户编辑的精确回滚。

## 11. 版本历史

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-08-12 | v1.0 | 初始版本：prompt 按改动范围分流 |
| 2026-08-13 | v1.1 | 人类确认扩展：先修 block_ops 事务/回滚，再统一运行时路由、预览状态和测试；不修改用户数据。 |
| 2026-08-13 | v1.2 | P0/P1 依次闭合：结构化事务和 reverse 安全修复、BDocx 动态改写边界、预览状态事实与原始回归证据完成。 |

## 12. Task Runtime Report — AG-STRATEGY-05

### 12.1 任务结果

- 状态：`READY`。
- 本次只修复未来 proposal 的执行和显示路径；未改写或迁移用户当前工作区、数据库或已有 dirty 草稿。

### 12.2 实际变更

- **P0 · 结构化事务**：同锚点的 `insert_before` / `insert_after`、`replace_node` 和 `delete_node` 现在按稳定计划执行；先在事务内校验最终 LogicalState，再一次性 dispatch。插入和 append 的 reverse 改为按实际插入的稳定 blockId 删除，不再解析空 `beforeNode`。
- **P1 · BDocx 路由**：静态 prompt、运行时能力声明、tool schema、manifest 和服务端 guard 统一为同一边界：仅当前 BDocx 无表格、无图片时允许 `replace_document`；含有或新引入表格/图片时一律拒绝并要求结构化操作。允许插入新标题，但不允许把既有块直接转换类型。
- **P1 · 状态事实**：pending proposal 先标为 `queued`；仅编辑器预览成功后成为 `applied`，失败为 `failed` 并保留稳定错误码。queued proposal 禁止接受，卡片和 Agent 文案分别说明“建议、预览、接受、保存”四个阶段。最小 trace 只记录 diff id、预览状态和错误码，不记录正文。
- **回归与基线**：新增同锚点四标题、`insert_after`、append、delete 和 forward→reverse JSON 等值断言；补齐 queued/applied/failed UI/服务测试。实际新增 ED 4 项、DE 5 项后，只更新测试治理测试中对应的精确动态基线（ED 239、DE 66）。

### 12.3 验证结果

| 验证 | 结果 |
|---|---|
| 原始问题审计日志 | 已完整读取 `/private/tmp/binder-audit.log`：168,187 行、11,882,558 bytes；其中包含完整 proposal、动态 BDocx 声明、会话快照和无落盘证据，未以 UI 裁剪输出替代原日志 |
| `npm run test:related -- --files [AG-STRATEGY-05 changed_files]` | PASS；原始重定向日志 `/private/tmp/binder-ag-strategy-05-related.log` 为 5,612 行、219,642 bytes。前端 26 files / 562 tests，Rust lib 319/319、`core_chain` 5/5、`sm_broadcast` 4/4，TypeScript 与治理审计均通过 |
| `npm test` / `npx tsc --noEmit` | PASS：26 files / 562 tests；TypeScript 无诊断 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | PASS（非沙箱 macOS 环境）：Rust lib 319/319、`core_chain` 5/5、`sm_broadcast` 4/4，包含 LibreOffice 真转换对照环境 |
| `npm run governance:generate` / `npm run governance:audit` | PASS：test governance 92 assets、26 frontend files、64 Rust inline files、2 Rust integration files、890 tests；0 finding / 0 warning |
| `git diff --check` | PASS |

### 12.4 语义复核

- “回复中说有标题、编辑器没有标题”已不再依赖模型是否生成标题：同锚点事务会保留四个插入标题、替换正文并拒绝任何不满足 postcondition 的批次，因而不会以残缺 LogicalState 伪装成功。
- 整篇改写不再被笼统地引导到 `block_ops` 或 `replace_document`；它依据当前 BDocx 是否含受保护结构动态分流，表格/图片保护不降级。
- `propose_edit` 的成功只表示建议已创建；没有客户端应用回执时不得宣称编辑器已改，更不得宣称已保存。
- 规则正文不改。

### 12.5 未解决项

- 已存在的损坏 pending 预览不做自动修补或覆盖：它未落盘，应丢弃/重开后重新生成建议，避免覆盖用户可能的后续编辑。
- `AUTOMATION_GAP-04`：真实 Tauri/WKWebView/provider 串起 S6 的人工系统验证尚未执行；这不影响本轮自动化闭合，但 G5 仍不得表述为通过。
- `cargo fmt --check` 暴露的是工作区既有的大范围格式漂移；本 Scope 未做无关格式化写入。

### 12.6 READY 自检

- 范围：仅 Scope 文件和生成的 ADUS 受影响；未覆盖用户既有未提交改动。
- 验证：相关选择器、前端、TypeScript、Rust、generate/audit 和 diff 检查均已实际运行；原始日志路径与计数如上。
- 证据：自动化通过与真实 WKWebView/provider 未执行严格分开记录。
- 完整/诚实/留痕：无 TODO、桩、跳过或静默恢复；未解决的真实系统验证和旧 preview 处置已登记。
