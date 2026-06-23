# Issue Trace · PI-BE-03+FE-12 外部变更监视（补全 SM-WS-01 另一半）

承接 A-WS-I-P-01 §6（ws.external_change 逻辑契约，物理传输延 SY-02）/ A-WS-M-S-01 SM-WS-01（externalChange→Conflicted）/ A-WS-M-T-01 BR-WS-OBS-001（外部变更感知 + 加载基线）/ WS-FS / X-FORBID-004 / X-FORBID-007。

文档补充（实现选择记录）：
- A-WS-I-P-01：本阶段以**前端轮询 `ws.check_external_change(relPath, baseline)` 命令**实现 external_change 逻辑契约（每 5s 比对磁盘哈希 vs 加载基线，UI 参考稿链路 D）；物理事件推送（Channel/event）仍延 SY-02。新增该命令到 §3 + §6/§7 记录。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - docs/01_modules/WS/A-WS-I-P-01_工作台接口协议.md   # 新增 ws.check_external_change + 实现选择记录
  - src-tauri/src/workspace/save.rs                    # check_external_change（BR-WS-OBS-001）
  - src-tauri/src/lib.rs                               # 路由 + 注册 ws.check_external_change
  - src/services/wsService.ts                          # checkExternalChange service
  - src/services/edService.ts                          # pollExternalChange 编排
  - src/components/ed/Editor.tsx                       # 5s 轮询
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md
  - .governance/active_scope.json
  - .governance/issues/PI-BE-03-FE-12.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/src/ipc/mod.rs
  - src-tauri/src/workspace/lifecycle.rs
confirmation: YES
scope_reason: PI-BE-03+FE-12 补全 SM-WS-01 外部变更感知——后端 check_external_change 比对基线、前端 5s 轮询触发 external_changed 冲突条（PI-FE-09 已有覆盖/重载）；人类已授权按计划 P2 开发。
```

## Expected Behavior

- ws.check_external_change(relPath, baseline)：经 resolve_in_workspace 守界（X-FORBID-004）比对磁盘当前哈希
  与传入加载基线（BR-WS-OBS-001）；回 { changed, deleted, current }（文件被外部删除 → deleted=changed=true）。
- 前端编辑器打开文档时每 5s 轮询；changed → edStore.tabState=external_changed → 触发已有冲突条
  （保留我的编辑覆盖 / 放弃重载，PI-FE-09）。无打开文档/无基线则不轮询。
- 不静默覆盖外部修改（SM-WS-01：外部改后无显式 override 不达 Saved）；状态以后端基线为准（X-FORBID-007）。

## Validation Commands

```
cargo test
npx tsc --noEmit
npm run build
npm run governance:audit -- --issue .governance/issues/PI-BE-03-FE-12.md
```

验收：cargo test 全过（含 check_external_change covers BR-WS-OBS-001：变更/未变/删除三态）；外部改动文件后 5s 内编辑器出现冲突条；scoped 门禁无越界。
