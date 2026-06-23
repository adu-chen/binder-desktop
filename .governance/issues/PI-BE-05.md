# Issue Trace · PI-BE-05 时间轴快照存储 + list/preview/restore 接线（后端）

承接 A-WS-I-P-01 §5（ws.list_timeline/preview_node/restore_node）/ A-WS-M-T-01（WS-03 快照存储形态延实现期）/ A-WS-M-S-01 SM-WS-03（Previewing/Restoring）/ WS-TIMELINE / BR-WS-DATA-002（节点准入/序）/ BR-WS-STATE-002（还原一致性）/ X-FORBID-004 / TERM-WS-002（DocumentTimeline）。

文档补充（解除 WS-03 债务）：A-WS-M-T-01 记录快照存储形态 = workspace.db `timeline_snapshots`（全量内容、内容寻址哈希、去重）。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md   # 解除 WS-03 快照存储形态
  - src-tauri/src/db/mod.rs                            # 迁移 v4：timeline_snapshots
  - src-tauri/src/workspace/timeline.rs                # store/load_snapshot + preview_node
  - src-tauri/src/workspace/diff_apply.rs              # apply_diff 落盘时存快照
  - src-tauri/src/lib.rs                               # 路由 ws.list_timeline/preview_node/restore_node + 注册
  - .governance/active_scope.json
  - .governance/issues/PI-BE-05.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/src/ipc/mod.rs
  - src-tauri/src/workspace/lifecycle.rs
confirmation: YES
scope_reason: PI-BE-05 实现时间轴快照存储（解除 WS-03）使 preview/restore 可用，接 3 个 timeline 路由；纯后端；人类已授权按计划 P3 继续。
```

## Expected Behavior

- 迁移 v4 timeline_snapshots(snapshot_ref PK, content BLOB, created_at)；追加式不破坏既有态（BR-SYS-PERSIST-001）。
- apply_diff 落盘时按内容寻址（blob_{hash}）存全量快照（INSERT OR IGNORE 去重）；时间轴节点的 snapshot_ref 指向之。
- ws.list_timeline(relPath)：回文档时间轴节点（version_seq 升序，BR-WS-DATA-002）。
- ws.preview_node(entryId)：经节点 snapshot_ref 取回该版本内容；快照不可取回→guard 错误（SM-WS-03 Previewing）。
- ws.restore_node(entryId)：取回快照内容→原子写回文档（守界 X-FORBID-004）→记一个新时间轴节点（kind=restore，
  version_seq 单调，BR-WS-STATE-002）；还原失败保持现状不留半节点。
- 全部经 dispatch 信封 + CommandRegistry（BR-SYS-DATA-003）。

## Validation Commands

```
cargo test
npx tsc --noEmit
npm run governance:audit -- --issue .governance/issues/PI-BE-05.md
```

验收：cargo test 全过（含快照往返 + preview/restore covers BR-WS-DATA-002/BR-WS-STATE-002）；apply→list→preview→restore 链路；scoped 门禁无越界。
