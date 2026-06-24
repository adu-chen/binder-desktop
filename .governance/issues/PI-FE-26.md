# Issue Trace · PI-FE-26（PI-A）center 标签栏 + 预览标签共存

承接 A-SYS-C-T-02 §3/§5.3（实现 UI 参考稿标签栏，拍板"单窗多 tab"决策）/ 参考稿 §4 Center 标签栏 / X-FORBID-007（标签为视图层导航态）。本 PI 立标签栏 + 多预览共存（单编辑标签）；多文档会话归 PI-B。

文档同步：A-SYS-C-T-02 §3/§5.3 把"单窗多 tab"从延实现期决策记为已决策——center 顶部统一标签栏承载文档(ED) + 预览(PRV)同列共存（视图层 tabsStore）。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - docs/01_modules/SYS/A-SYS-C-T-02_前端交互架构与流程总图.md   # 记录单窗多 tab 决策
  - src/store/tabsStore.ts                       # 视图层标签态（tabs + activeTabId）
  - src/store/tabsStore.test.ts                  # 标签模型单测

  - src/store/prvStore.ts                         # 移除（被 tabsStore 取代）
  - src/components/shell/TabStrip.tsx            # center 标签栏
  - src/shell/Workbench.tsx                        # center：标签栏 + 按激活标签渲染 Editor/PrvPanel
  - src/components/ed/Editor.tsx                 # 移除内置标签栏（改由 TabStrip）
  - src/components/prv/PrvPanel.tsx              # 关闭 → tabsStore.close
  - src/components/ws/FileTree.tsx               # 打开/预览 → tabsStore
  - src/styles.css                              # 标签栏样式
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md
  - .governance/active_scope.json
  - .governance/issues/PI-FE-26.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/**
confirmation: YES
scope_reason: PI-FE-26（PI-A）立 center 标签栏 + 预览多标签共存（单编辑标签，不动 edStore 会话模型）；同步 A-SYS-C-T-02 单窗多 tab 决策；人类已确认分层与两步走。
```

## Expected Behavior

- 新 tabsStore（视图层，X-FORBID-007）：tabs[]（OpenTab{id,path,kind:'doc'|'preview'}）+ activeTabId；
  openDoc（单文档标签，复用/更新路径）、openPreview（多预览标签，按 path 去重）、close、activate、reset。
- center 顶部统一标签栏（TabStrip）列出全部标签（文档 + 预览同列），点切换、× 关闭；文档标签显脏态/pending/
  external 标记（投影 edStore/deStore）。激活文档标签 → Editor；激活预览标签 → PrvPanel(该 path)。
- 多个预览（office/媒体/图片）可同时开为多标签共存；编辑标签暂为单个（多文档会话归 PI-B）。
- prvStore 由 tabsStore 取代（移除）；切 workspace 重置标签。

## Validation Commands

```
npx tsc --noEmit
npm test
npm run build
npm run governance:audit -- --issue .governance/issues/PI-FE-26.md
```

验收：开多个图片/媒体 → 多标签共存可切换/关闭；文档标签状态标记正确；预览/编辑切换正常；vitest 全过；scoped 门禁无越界。
