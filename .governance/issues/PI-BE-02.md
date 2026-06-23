# Issue Trace · PI-BE-02 DE diff payload 持久化 + de.* 命令接线（后端）

承接 A-DE-I-P-01 §2/§3（de.generate_diff/accept_diff/reject_diff/list_diffs）/ A-DE-M-T-01（payload 形状 BR-DE-DATA-002、生成 BR-DE-DATA-001、终态 BR-DE-STATE-001）/ SM-DE-01 / DE-DIFF / X-INTER-001（accepted→ws.apply_diff 唯一落盘）/ BR-WS-STATE-001 / TERM-DE-001（PendingDiff）/ TERM-DE-002（ChangeProposal）/ TERM-DE-004（DiffPayload，本 PI 新注册）。

文档补充（遗漏/待补项）：
- A-DE-I-P-01 §7「payload 序列化与 before_snapshot 存储形态延实现期」→ 本 PI 解除：定为 workspace.db `diff_payloads` 表（DE 拥有 payload 形状、WS db 承载持久化，A-DE-M-T-01 §1）。
- A-DE-M-T-01 §0 注册 TERM-DE-004 DiffPayload（与 ChangeProposal/PendingDiff/DiffCard 歧义隔离）。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - docs/01_modules/DE/A-DE-M-T-01_对话编辑技术设计.md   # 注册 TERM-DE-004 + payload 存储说明
  - docs/01_modules/DE/A-DE-I-P-01_对话编辑接口协议.md   # §7 解除 payload 存储延期项
  - docs/00_core/A-CORE-C-R-01_ADUS.md                  # governance:generate 自动同步
  - src-tauri/src/db/mod.rs                              # 迁移 v3：diff_payloads 表
  - src-tauri/src/de/store.rs                            # 新增：payload 持久化/读取
  - src-tauri/src/de/mod.rs                              # 声明 pub mod store
  - src-tauri/src/lib.rs                                 # dispatch 接 de.generate_diff/accept_diff/reject_diff/list_diffs + 注册
  - .governance/active_scope.json
  - .governance/issues/PI-BE-02.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/src/ipc/mod.rs
  - src-tauri/src/workspace/lifecycle.rs
confirmation: YES
scope_reason: PI-BE-02 把 DE 既有 generate/payload 逻辑接入 dispatch + 补 payload 持久化（diff_payloads 表），使 accepted 经 de.accept_diff→ws.apply_diff 真落盘；补设计文档待补项 + 注册 TERM-DE-004；人类已授权按计划开发。
```

## Expected Behavior

- 迁移 v3 新增 diff_payloads(payload_ref PK, workspace_id, rel_path, after_content BLOB, before_snapshot, is_new_doc, created_at)；追加式不破坏既有持久化态（BR-SYS-PERSIST-001）。
- ws/de payload 存储：persist_payload(DiffPayload)；load_payload_for_diff(diff_id) 经 pending_diffs.payload_ref 关联取回 after_content + rel_path（BR-DE-DATA-002，重启恢复）。
- de.generate_diff(relPath, afterContent, beforeSnapshot?, originModule, anchorLocate?)：组装 ChangeProposal→build_payload→generate_diff（修改既有须锚 Located，否则 DE_ANCHOR_LOST/GenFailed，BR-DE-DATA-001）→ persist_payload + ws.persist_diff；回 diff_id（Pending）。
- de.accept_diff(diffId)：载 payload → 解析 abs 路径（X-FORBID-004）→ ws.apply_diff 原子写 + 记一个时间轴节点 + 置 accepted（accepted 唯一落盘，X-INTER-001）；回 TimelineEntry。
- de.reject_diff(diffId)：ws.discard_diff(rejected)，不写文件、不记节点。
- de.list_diffs(status?)：活动 workspace 未裁决 diff 列表（重启恢复）。
- 全部经 dispatch 信封 + CommandRegistry（BR-SYS-DATA-003）；终态互斥不可逆（SM-DE-01）。

## Validation Commands

```
npm run governance:generate   # 同步 ADUS（TERM-DE-004）
cargo test
npx tsc --noEmit
npm run governance:audit -- --issue .governance/issues/PI-BE-02.md
```

验收：cargo test 全过（含 payload 往返 + accept 真落盘集成 covers BR-DE-DATA-002/BR-WS-STATE-001）；TERM-DE-004 入 ADUS 且被 term_ref 引用、无 TERM_UNREGISTERED；scoped 门禁无越界。
