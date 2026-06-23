# Issue Trace · PI-BE-01 WS 落盘命令接线（read/save/diff-apply）

承接 A-WS-I-P-01 §3/§4（ws.read_doc/save_doc/persist_diff/list_pending_diffs/apply_diff/discard_diff）/ A-WS-X-L-01 PI-WS-01/02/03 的 command 接线 / WS-FS / WS-SAVE / WS-DIFF-APPLY / BR-WS-DATA-001 / BR-WS-OBS-001 / BR-WS-PERSIST-001 / BR-WS-STATE-001 / X-FORBID-004。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - src-tauri/src/workspace/save.rs              # 新增 read_doc（内容 + 加载基线，BR-WS-OBS-001）
  - src-tauri/src/workspace/diff_apply.rs        # 新增 list_pending_diffs（重启恢复入口）
  - src-tauri/src/lib.rs                         # dispatch 接 6 条 WS 落盘 command + 注册
  - .governance/active_scope.json
  - .governance/issues/PI-BE-01.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/src/ipc/mod.rs
  - src-tauri/src/workspace/lifecycle.rs
confirmation: YES
scope_reason: PI-BE-01 把 WS 既有落盘原语（save_atomic/apply_diff/persist/discard）接入 dispatch，补 read_doc/list_pending_diffs 两个查询并加测试；纯后端 IPC 接线；人类已授权继续后端落盘。
```

## Expected Behavior

- ws.read_doc(relPath)：经 resolve_in_workspace 守界（X-FORBID-004）读活动 workspace 内文档，
  回 { content, baseline }（加载基线供 save 校验与外部变更检测，BR-WS-OBS-001）。
- ws.save_doc(relPath, content, baseline?, override?)：原子写入（BR-WS-PERSIST-001）；磁盘≠基线且
  未 override → WS_SAVE_CONFLICT（SM-WS-01 saveConflict）；回新基线。
- ws.persist_diff(PendingDiff)：持久化为 status=pending（重启恢复）。
- ws.list_pending_diffs(status?)：回活动 workspace 未裁决 diff 列表（重启恢复入口）。
- ws.apply_diff(diffId, relPath, afterContent, baseline?)：accepted 唯一落盘路径——原子写 + 记恰好一个
  时间轴节点 + 置 accepted（BR-WS-STATE-001）；写失败不留半节点。afterContent 由 DE 产出方提供
  （payload_ref→内容解析归 DE，此处接线取已产出内容）。
- ws.discard_diff(diffId, terminal)：rejected/expired 仅置终态，不写文件、不记节点。
- 全部经统一 dispatch 信封 + CommandRegistry 校验（BR-SYS-DATA-003）；越界路径一律拒绝（X-FORBID-004）。

## Validation Commands

```
cargo test
npx tsc --noEmit
npm run governance:audit -- --issue .governance/issues/PI-BE-01.md
```

验收：cargo test 全过（含 read_doc / list_pending_diffs 新测试，covers BR-WS-OBS-001 / BR-WS-STATE-001）；
新 @GOV 追溯已注册规则；scoped 门禁无越界。
