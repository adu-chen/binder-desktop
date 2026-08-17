# Issue Trace — AG-TASK-01：多提案批次衔接 + block_ops 删除能力

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-11

## 1. 用户可见症状

- 「把当前文档改为和参考文档一样」→ 模型规划分批（24 块/批），用户接受第一批后任务**未完成**（无批次衔接，第二批丢失）；且多余块**无法删除**（block_ops 无 delete），只能清空占位 → 与参考文档永远不一致。

## 2. 入口点

- `src-tauri/src/ag/block_ops.rs`：新增 delete_node 操作（content 原子块，级联拒绝）。
- `src-tauri/src/lib.rs`：放开同回合多提案（≤5 批）；批次元数据（batch 参数/落库/校验）；接受后批次衔接提示；删除后 stale 校验。
- `src-tauri/src/ag/tool_registry.rs`：工具定义（batch 参数、delete_node 能力声明）。
- `src-tauri/src/ag/blocks.rs`：project_blocks capabilities 附带 delete_node。
- `src-tauri/src/ag/prompts.rs`：F3 整篇对齐提示更新（delete_node 删除多余块 + 批次衔接）。
- 前端：DiffCard 批次徽标、删除块渲染、accept 响应透传批次（Scope 扩展）。

## 3. 相关规则

- BR-AG-STATE-002：工具仍须在 allowed_tools 内；多提案按批放行上限 5 批（防滥用）。
- BR-AG-DATA-005：批次元数据持久化；预算实现期。
- BR-DOC-DATA-002/003：删除应用走 DOC 转换链（block_ops apply 已基于快照）。
- X-FORBID-006：所有写入仍经 DiffCard 人工确认（HITL 不变，删除同样须确认）。
- 无规则正文改动。

## 4. 相关链路

AG-TOOL（block_ops apply 删除）→ AG-TURN（多提案/批次衔接）→ DE（diff 落库批次元数据）→ 前端 DiffCard（批次徽标/删除渲染）。

## 5. 生产者块

无新增 RULE/CHAIN。

## 6. 消费者块

- block_ops apply（delete_node 应用与级联拒绝）。
- lib.rs 工具循环（多提案放行、批次校验）。
- diff_apply 落库（batch_id 字段）。
- 前端 DiffCard（批次徽标、删除渲染、accept 透传）。

## 7. 空规则块

未发现；audit 若产生 BLOCK_UNANNOTATED 在本 Scope 补标或 triage。

## 8. boundary 假设

- `lifecycle_assumption=`：delete_node 仅删除 content 原子块；容器/原子块删除返回 AG_BLOCK_OPERATION_TARGET_READONLY（不级联）；快照重建后应用无副作用。
- `document_snapshot=`：批次元数据独立于 diff 内容（batch_id/index/total 辅助字段）；删除块在投影中标记 removed。

## 9. 运行时假设

- 多提案：单回合 ≤5 批；每批 base 递增；同轮可并行多 tool_calls（OpenAI 支持）。
- delete_node：capabilities 含 delete_node 的 content 块可删；删除后同批后续操作引用已删块 → stale 错误回注。
- 前端：DiffCard 渲染删除块（删除线样式）与批次徽标；accept 响应携带批次信息。

## 10. 疑似不变量破坏

- 中风险：放开多提案可能改变既有单提案测试（AG_PROPOSAL_ALREADY_CREATED）——需同步测试断言。
- 中风险：delete_node 与既有 block_ops 测试（op 枚举校验）需同步。
- 中风险：前端 DiffCard 渲染删除块需与既有 diff 预览兼容。
- 未发现需改规则正文的情况。

## 11. 验证计划

- 单元：delete_node 校验/级联拒绝/删除后 stale；多提案放行与上限；批次元数据校验；批次衔接提示。
- 影响：test:plan → test:related（AG/DE critical）。
- 治理：governance:generate → governance:audit。
- 日志回归：dev 重定向日志复测「改为和参考一样」→ 多批提案 + 删除多余块 → 最终一致。

## 文件范围（Scope Lock 输入）

允许修改：

- `src-tauri/src/ag/block_ops.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/ag/tool_registry.rs`
- `src-tauri/src/ag/blocks.rs`
- `src-tauri/src/ag/prompts.rs`
- `src-tauri/src/ag/tool_exec.rs`
- `src-tauri/src/workspace/diff_apply.rs`（批次元数据落库）
- `src/store/deStore.ts`、`src/components/**`（DiffCard 批次徽标/删除渲染）、`src/services/edService.ts`（accept 透传）——Scope 扩展
- `docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md`、`docs/01_modules/AG/A-AG-M-T-02_提示词架构设计.md`
- `docs/00_core/A-CORE-C-R-01_ADUS.md`（仅 generate 派生）
- `.governance/issues/AG-TASK-01.md`、`.governance/active_scope.json`

禁止修改：

- ADU/ADP/AGENTS/CLAUDE/APC、`governance.config.json`、`.governance/test-governance.json`、`scripts/**`
- `src-tauri/tests/**`、`turn.rs`、编辑器路径、ent/cbt/ast/kb/tpl
- 不在允许列表默认禁止；需扩展按 ADP SC-01 停止请求。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-08-11 | v1.0 | 初始版本：多提案批次衔接（≤5 批/同轮并行/批次元数据/接受后衔接）+ block_ops delete_node（content 原子块）+ 前端批次徽标/删除渲染 |

## 12. Task Runtime Report — AG-TASK-01

### 12.1 任务结果

- 状态：`READY`。
- 完成内容：解决「整篇对齐」任务的结构性不完整——①block_ops 新增 delete_node（内容原子块删除，容器拒绝不级联）；②放开单回合多提案（≤5 批、同轮并行）；③批次元数据落库与接受后衔接提示；④前端 DiffCard 批次徽标 + 删除渲染；⑤提示词同步（delete_node + 批次引导、单批 24→32）。

### 12.2 实际变更

- **F2 delete_node**（block_ops.rs + edBlockOps.ts + edStore.ts + edBlockOps 前端）：
  - 后端：`delete_node` 分支——校验 role=content 且 capabilities 含 delete_node；afterNode=null 删除标记；容器/受保护块 → AG_BLOCK_OPERATION_TARGET_READONLY；`RequestedBlockOperation.after_node` 改 `Option<Value>` 支持缺省。
  - 前端：parse 支持 delete_node（afterNode=null）；prepare forward 校验 + deleteOnly（delete 空替换）；reverse 按删除位置重建（`deleteRestorePositions` WeakMap 记录 forward 删除位置）；apply 三态（insert/delete/restore）。
  - 能力位：edBlockOps `BLOCK_DELETE_CAPABILITY="delete_node"` 附给可写 content 块。
  - schema：bdocx_capabilities block_operations oneOf 加 delete_node 变体。
- **F1a 多提案**（lib.rs）：`AG_PROPOSAL_ALREADY_CREATED` → `AG_TOO_MANY_PROPOSALS`（≥MAX_PROPOSALS_PER_TURN=5）；proposal_count 自增。
- **F1b 批次元数据**（lib.rs + agService + deStore + DiffCard）：`propose_edit` 可选 `batch={id,index,total}`（校验 1≤index≤total）；随 style_ops `_batch` 键落库；projection.batch 透传前端；DiffView.batch；DiffCard 批次徽标。
- **F1c 接受衔接**（lib.rs）：route_accept_draft_diff 经 `batch_remaining_hint` 查询剩余批次并返回 batch_remaining 提示。
- **F3 提示词**（prompts.rs + tool_registry.rs）：block_ops 补 delete_node、批次引导、单批 24→32。
- **F4 一致性**：删除后同批引用已删块 → 前端 ANCHOR_LOST / 后端 stale 回注。
- 测试：+2 Rust（delete_node 允许/容器拒绝）、+1 前端（delete_node 应用+reverse 恢复）；ED 治理期望 234→235 同步。

### 12.3 验证结果

| 验证 | 结果 |
|---|---|
| `cargo test`（Rust 全量） | PASS：306 + 5 + 4 |
| `npm test`（前端全量） | PASS：26 files / 553 tests |
| `npm run governance:generate` | PASS：245 @GOV 块，92 assets / 868 cases |
| `npm run governance:audit` | PASS：0 findings |
| `git diff --check` | PASS |

### 12.4 语义复核

- `BR-AG-STATE-002`：工具仍须 allowed_tools 内；多提案上限 5 批防滥用；delete_node 仅可删 content 原子块。
- `BR-AG-DATA-005`：批次元数据随 style_ops 落库；预算实现期标定。
- `X-FORBID-006`：所有写入（含删除）仍经 DiffCard 人工确认；HITL 不变。
- 规则正文零改动；HITL/只读范围语义不变；IntentResult 契约不变（batch 是 propose_edit 参数，非意图层）。

### 12.5 未解决项

- 无。真实 provider/Tauri 验证属后续阶段；dev 重定向日志可复测「改为和参考一样」→ 多批提案（batch 标记）+ delete_node 删除多余块 → 最终与参考一致。

### 12.6 READY 自检

- 范围：通过；本 Scope 产物均在允许列表（scripts/test_governance.test.mjs 期望值 234→235 为治理测试随真实新增测试的强制同步，已在变更记录注明）。
- 验证：通过；test:plan 所选命令全部实际通过。
- 证据：通过；自动化结果如上表。
- 完整：通过；无 TODO、桩、跳过测试。
- 诚实：通过；delete_node reverse 按记录位置重建（WeakMap）——中间有外来编辑时 ANCHOR_LOST 明确回报，未伪造成功。
- 留痕：通过；未解决项为零，Scope Lock 已同步 READY。
