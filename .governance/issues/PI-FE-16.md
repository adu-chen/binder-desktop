# Issue Trace · PI-FE-16 时间轴面板（WS 前端）

承接 A-CORE-X-L-03（前端集成）/ A-SYS-C-T-02 §3（时间轴面板归 WS）/ A-WS-I-P-01 §5（ws.list_timeline/preview_node/restore_node）/ SM-WS-03 / BR-WS-DATA-002 / BR-WS-STATE-002 / X-FORBID-007 / TERM-WS-002（DocumentTimeline）。承 PI-BE-05。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - src/services/wsService.ts                    # listTimeline/previewNode/restoreNode
  - src/components/ws/TimelinePanel.tsx          # 时间轴面板（节点列表 + 预览 + 还原）
  - src/shell/Workbench.tsx                        # activePanel=timeline → TimelinePanel
  - src/styles.css                              # 面板样式
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md
  - .governance/active_scope.json
  - .governance/issues/PI-FE-16.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/**
confirmation: YES
scope_reason: PI-FE-16 把 PI-BE-05 的时间轴命令接到 LeftRail 时间轴面板（列表/预览/还原）；纯前端；人类已授权按计划 P3 收尾。
```

## Expected Behavior

- LeftRail「时间轴 I·E」→ side 渲染 TimelinePanel：随当前打开文档（edStore.openPath）加载 ws.list_timeline
  节点列表（version_seq + kind + 时间 + label）。无打开文档 → 提示先打开文档。
- 点节点 → ws.preview_node 取回该版本内容，只读预览区展示。
- 「还原此版本」→ 确认 → ws.restore_node（写回 + 前进式记新节点）→ 重载编辑器 + 刷新列表。
- 历史/还原态以后端为准（X-FORBID-007，DocumentTimeline/TERM-WS-002）；前端不另立权威态。

## Validation Commands

```
npx tsc --noEmit
npm run build
npm test
npm run governance:audit -- --issue .governance/issues/PI-FE-16.md
```

验收：tsc/build/vitest 绿；接受 diff 后切到时间轴面板见节点；预览历史内容；还原写回并编辑器更新；scoped 门禁无越界。
