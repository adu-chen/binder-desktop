# Issue Trace · PI-FE-27（PI-B）多文档标签会话（edStore 按路径会话）

承接 A-SYS-C-T-02 §5.3（单窗多 tab 增强：多文档各保留未保存编辑）/ ED-EDIT / BR-ED-STATE-001 / X-FORBID-007。承 PI-FE-26。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - src/store/edStore.ts                          # 按路径会话 sessions + activePath（派生 openPath/tabState 兼容既有消费方）
  - src/store/edStore.test.ts                     # 适配会话 API
  - src/store/tabsStore.ts                         # openDoc 允许多文档标签；activate(doc)→edStore.setActivePath
  - src/store/tabsStore.test.ts                    # 适配多文档标签
  - src/components/ed/Editor.tsx                 # 切换时快照旧会话内容 + 载入新会话（保未存编辑）
  - src/services/edService.ts                     # 保存写回活动会话内容
  - src/components/shell/TabStrip.tsx            # 每个文档标签投影各自脏态
  - src/components/ws/FileTree.tsx               # 每个打开文档投影各自脏态
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md
  - .governance/active_scope.json
  - .governance/issues/PI-FE-27.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/**
confirmation: YES
scope_reason: PI-FE-27（PI-B）edStore 重构为按路径会话使多文档标签各保留未保存编辑；派生 openPath/tabState 保既有消费方兼容；人类已确认两步走第二步。
```

## Expected Behavior

- edStore：sessions（path → {content, baseline, tabState}）+ activePath + loadSeq；派生 openPath/tabState/
  baseline/loadedContent = 活动会话（既有消费方无需大改）。loadDoc/setActivePath/updateContent/markDirty/
  markSaved/setTabState/closeSession/reset。
- 标签：openDoc 允许多文档标签（按 path 去重）；点切换 → edStore.setActivePath；Editor 在切换时**快照旧会话
  当前编辑内容**（不落盘）再载入新会话内容 → 切回标签保留未保存编辑。
- 关闭文档标签 → closeSession 移除该会话；保存写回活动会话；脏态/外部变更/pending 仍按活动/各会话投影。
- TabStrip 每个文档标签、FileTree 每个打开文档各自显示自己的脏态/pending/external（X-FORBID-007 投影）。

## Validation Commands

```
npx tsc --noEmit
npm test
npm run build
npm run governance:audit -- --issue .governance/issues/PI-FE-27.md
```

验收：开多文档标签、各自编辑 → 切换标签保留各自未保存内容；脏态各标签独立显示；保存只存当前；vitest 全过；scoped 门禁无越界。
