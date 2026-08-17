# Issue Trace — AG-DELETE-01：删除能力（确认卡）+ 跨文档目标放行

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-11

## 1. 用户可见症状

- agent-edit-1.0 无删除能力：用户无法让 AG 删除文档/文件夹（前端手工可删，AG 不可）。
- AG-STRATEGY-01 遗留：ag_propose_edit 内 guard_target_set / ensure_target_declared 仍硬拒"未声明既有文档"，多文档联动（改 B 同步 A）仍被锁死，与"跨文档标注确认"方案不符。

## 2. 入口点

- `src-tauri/src/lib.rs`：ag_delete_proposal（删除提案生成）+ 工具循环 propose_edit delete 分支 + route_apply_delete_diff（删除应用）+ ag_propose_edit 跨文档标注。
- `src-tauri/src/ag/context.rs`：guard_target_set 增 AcceptCrossDocument。
- `src-tauri/src/de/generate.rs`：ensure_target_declared → target_declared（bool，不再硬拒）。
- `src-tauri/src/ag/tool_registry.rs` / bdocx_capabilities.rs：propose_edit operation schema 增 delete_doc/delete_folder。
- 前端：`src/store/deStore.ts`（DiffView patchKind/delete 投影）+ `src/components/de/DiffCard.tsx`（删除卡渲染）+ 接受动作。

## 3. 相关规则

- BR-DE-DATA-002：DiffPayload 形状；删除走独立提案（origin_module='AG_DELETE'，payload_ref 含 action/recursive/cascade），复用 pending_diffs 持久化与 DiffCard 通道。
- X-FORBID-006：写入/删除均经确认卡人工裁决（HITL 不降级）。
- X-FORBID-004：删除目标限 workspace 内；禁止删除 workspace 根与保留目录。
- BR-AG-DATA-003：目标声明规则放宽——未声明既有文档放行但标注 cross_doc（确认卡兜底）。
- 无规则正文改动。

## 4. 相关链路

AG propose_edit(operation=delete_*) → ag_delete_proposal（workspace 边界+存在性+保留目录校验）→ PendingDiff(AG_DELETE) + DiffProposed → 前端删除卡 → ws.apply_delete_diff → fileops::delete + timeline。

## 5. 生产者块

无新增 RULE/CHAIN。

## 6. 消费者块

- lib.rs tests（删除提案/应用/跨文档标注）。
- context.rs tests（AcceptCrossDocument）。
- de/generate.rs tests（target_declared）。
- 前端 deStore/DiffCard tests。

## 7. 空规则块

未发现。

## 8. boundary 假设

- `lifecycle_assumption=`：删除必须经确认卡裁决（pending → accepted 才执行删除）；拒绝/过期不删。
- `document_snapshot=`：删除无内容 payload；确认卡展示路径 + 级联清单（目录递归计数）。

## 9. 运行时假设

- propose_edit(operation=delete_doc, path=...) / delete_folder(path=..., recursive=true) → 删除卡。
- 跨文档内容编辑（propose_edit path 非当前文档且未声明）→ 放行 + projection 标注 crossDoc=true。
- 删除 workspace 根/保留目录 → 拒绝（WS_DELETE_ROOT_DENIED）。

## 10. 疑似不变量破坏

- 跨文档硬拒取消 → 由 DiffCard「跨文档目标」标注 + 用户确认承担（安全转移非删除）。
- 删除经确认卡 → 与内容 diff 相同 HITL 强度。

## 11. 版本历史

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-08-11 | v1.0 | 初始版本：删除确认卡闭环 + 跨文档目标放行 |

## 12. Task Runtime Report — AG-DELETE-01

### 12.1 任务结果

- 状态：`READY`。

### 12.2 实际变更

- **lib.rs**：`ag_delete_proposal`（operation=delete_doc/delete_folder → PendingDiff AG_DELETE + 级联计数 + DiffProposed 投影）；工具循环 propose_edit 分支检测 delete 操作分流；`route_apply_delete_diff`（ws.apply_delete_diff：SM-DE-01 守护 + fileops 删除 + 状态 accepted + timeline 节点）；`ag_propose_edit` 跨文档放行（guard_target_set AcceptCrossDocument / target_declared → projection.cross_doc 标注）；AgentDiffProjection 增 cross_doc 字段。
- **context.rs**：TargetDecision::RejectUndeclared → AcceptCrossDocument。
- **de/generate.rs**：ensure_target_declared → target_declared(bool)。
- **bdocx_capabilities.rs**：propose_edit operation schema 增 delete_doc/delete_folder。
- **前端**：deStore DiffView.patchKind + deService applyDeleteDiff（ws.apply_delete_diff）+ DiffCard 删除卡（红色高亮、确认删除按钮）+ styles.css。

### 12.3 验证结果

| 验证 | 结果 |
|---|---|
| `cargo test`（Rust 全量） | PASS：315 + 5 + 4，0 失败 |
| `npm run test:related -- --files [deService/DiffCard/deStore]` | PASS：前端 553，0 findings |
| `npm run governance:generate` / `governance:audit` | PASS：877 cases / 0 findings 0 warnings |
| `git diff --check` | PASS |

### 12.4 语义复核

- 删除经确认卡裁决（HITL 不降级）；删除限 workspace 内、禁删根（WS_DELETE_ROOT_DENIED）。
- 跨文档目标放行 + crossDoc 标注（多文档联动不再被锁死）。
- 规则正文不改。

### 12.5 未解决项

- 删除提案/应用的端到端（G5：Tauri + WKWebView + 真实 provider）未在本机验证——route 层依赖运行环境，核心原子（fileops 删除、timeline append、pending diff 状态机）均有单元覆盖。
- 删除后引用 stale 提示（被删文档若被 @ 引用）——依赖引用模块联动，后续观察补齐。

### 12.6 READY 自检

- 范围/验证/证据/完整/诚实/留痕：通过；无 TODO/桩/跳过。
