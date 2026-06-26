# Issue Trace · PI-FE-29　保存按 path 锁定（消除异步保存期间切标签竞态）

承接 PI-FE-27 登记的 A 类债务："saveDocument/markSaved 作用于活动会话——异步保存期间切标签的极端竞态未按 path 锁定"。
SM-WS-01 / BR-WS-OBS-001 / BR-ED-STATE-001 / X-FORBID-007。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - src/store/edStore.ts                 # setTabState/markSaved 改按 path 锁定（新增 patchSession）；markDirty 仍作用活动会话
  - src/store/edStore.test.ts            # 适配 path 签名 + 新增异步保存竞态测试
  - src/services/edService.ts            # 保存/轮询入口捕获目标 path，await 完成后按 path 回写态
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md
  - .governance/issues/PI-FE-29.md
# 注：.governance/active_scope.json 当前被并行窗口（PI-AG-10C）占用，本 PI 不触碰；scoped 审计经本 issue 的 Scope Lock 判定。
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/**
confirmation: YES
scope_reason: 修复 PI-FE-27 登记的保存竞态：saveDocument/saveDocxDocument/pollExternalChange 在 await 前捕获目标 path，完成后经按-path 锁定的 setTabState(path,...)/markSaved(path,...) 回写会话态，避免保存途中切标签误改他会话；markDirty 仍作用活动会话。多文档安全语义补强，无新依赖。
```

## Expected Behavior

- edStore：`setTabState(path, tabState)` / `markSaved(path, baseline)` 改为按路径锁定指定会话（会话不存在=空操作，如保存途中标签被关闭）；`markDirty()` 仍作用活动会话（编辑器 onUpdate）。
- edService：`saveDocument`/`saveDocxDocument`/`pollExternalChange` 进入时捕获 `openPath`（及 baseline），await 完成后用捕获的 path 回写 saving/saved/external_changed/dirty，**不再读 await 后的活动会话**。
- 行为：保存 A 期间切到 B 标签 → 保存完成只把 A 标为 saved/冲突，B 的态不被污染；活动派生字段反映 B 不变。

## Validation Commands

```
npx tsc --noEmit
npm test
npm run build
npm run governance:audit -- --issue .governance/issues/PI-FE-29.md
```

验收：edService.test 4 项绿（活动==保存目标时派生不变）；新增竞态单测验证保存期间切标签只改目标会话；vitest 全过；scoped 门禁无自有文件越界。
