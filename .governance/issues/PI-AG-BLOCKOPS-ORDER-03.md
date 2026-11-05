# Issue Trace · PI-AG-BLOCKOPS-ORDER-03 块插入顺序与裁决粒度统一

## 1. 用户可见症状

- 反复失败后，`propose_edit` 已能生成 DiffCard，但应用后文档结构错位：正文各小节被插到「六、续写」标题之后，续写被拆成两截夹在正文前后。
- 一次 4000+3000 字请求产生 **82 张单操作卡**（2 个 replace_node + 38 + 42 个插入），逐张应用后顺序不可控。

## 2. 入口点

- `propose_edit` → `resolve_agent_edit_operation`（src-tauri/src/ag/edit_support.rs）→ `ag::block_ops::split_adjudication_groups`（src-tauri/src/ag/block_ops.rs）
- 编辑器应用：`applyStructuredBlockOperations`（src/services/deService.ts，单卡内按 operations 顺序应用）

## 3. 相关规则

- BR-AG-STATE-002（工具循环状态）
- BR-AG-DATA-010（block_ops 契约与预算）
- BR-DE-DATA-001（Diff/裁决投影）
- AG-R2-01（锚点矛盾校验，P5）
- 设计约束：A-AG-M-T-01「同锚点插入/替换互相覆盖：replace/delete 先写、插入后写，同坐标多插入按原 proposal 顺序展示」

## 4. 相关链路

AG-TURN → AG-TOOL → DE-DIFF → ED-EDIT

## 5. 生产者块

- `[BR-AG-STATE-002-GUARD-AG-AG-TOOL-059, BR-AG-DATA-010-GUARD-AG-AG-TOOL-059]` 位于 src-tauri/src/ag/edit_support.rs
- `[BR-DE-DATA-001-*]` 位于 src-tauri/src/ag/block_ops.rs（split_adjudication_groups）
- `[BR-AG-DATA-010-GUARD-AG-AG-TOOL-061]` 位于 src-tauri/src/ag/edit_support.rs（预算）

## 6. 消费者块

- `applyStructuredBlockOperations`（前端 DE/ED 应用）
- `[BR-AG-DATA-011-DATA-AG-AG-TOOL-062]` 回执与 DiffCard 投影

## 7. 路径上的空规则块

无。

## 8. boundary 假设

- 相关块未声明 `lifecycle_assumption=` / `document_snapshot=`。
- `find_block_ops_anchor_conflict` 的 does_not 含 "rewriteOperationsOrExecuteEdit"：本次只收窄判定，不改写操作、不执行编辑。
- `split_adjudication_groups` 的 effect=none：本次只调整分组，仍保持 base 不变、顺序确定。

## 9. 运行时假设

- 编辑器按单卡内 operations 顺序应用（deService `applyStructuredBlockOperations`）；
- 跨卡按卡片接受顺序应用，无法保证同锚点相对顺序 → 因此同锚点插入必须在同一卡内。

## 10. 疑似不变量破坏

- 风险：放开同锚点重复插入后，若操作分散在多张卡，顺序仍不可控。
- 约束：同一锚点的全部插入（以及该锚点的 replace/delete）必须落入同一裁决组；未显式分组时由服务端自动合并，显式分组若拆散同锚点则报错。
- 不得放宽：同一锚点混用 insert_before 与 insert_after 仍拒绝；append 仍仅文末且保持逐块/可按组。

## 11. 验证计划

- 单测：`block_ops` 同锚点多插入合成一组且顺序保持；混向仍拒绝；显式分组跨锚点仍按原规则；90 个 append 未分组仍逐块（不回归）。
- `cargo test --manifest-path src-tauri/Cargo.toml`、`npm test`（仓库内 688 项）、`npm run governance:audit`。
- 手工复验：同一提示词只产生少量卡（每段一张或同锚点一张），应用后正文在「五、正文」下、续写在「六、续写」下，顺序正确。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: PI-AG-BLOCKOPS-ORDER-03
allowed_files:
  - src-tauri/src/ag/edit_support.rs      # P5 锚点：同向重复放行，混向仍拒绝
  - src-tauri/src/ag/block_ops.rs         # 同锚点操作自动合并为一个裁决组；测试
  - src-tauri/src/ag/bdocx_capabilities.rs# 能力声明对齐（同锚点连续插入/顺序/同卡）
  - src-tauri/src/ag/tool_registry.rs     # propose_edit 工具描述对齐
  - src-tauri/src/ag/prompts.rs           # 提示词契约断言的同步（仅测试断言）
  - .governance/issues/PI-AG-BLOCKOPS-ORDER-03.md
forbidden_files:
  - ADU.md / ADP.md / AGENTS.md / CLAUDE.md / ADUS.md / APC / governance.config.json
  - .governance/test-governance.json / scripts/**
  - src/**（除 deService 之外的前端）/ cloud/**
confirmation: YES # 人类 2026-10-31 确认本 Scope
scope_reason: 统一“同锚点有序插入”的规则链：守卫、分组、能力声明与测试一致；不改变 base 与执行边界。
```
