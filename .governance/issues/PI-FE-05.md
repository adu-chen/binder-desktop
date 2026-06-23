# Issue Trace · PI-FE-05 TipTap 编辑器接入（ED 前端）

承接 A-CORE-X-L-03 §2 PI-FE-05 / ED 模块 L2（A-ED-M-T-01）/ SM-ED-01 / BR-ED-DATA-001（blockId 稳定）/ BR-ED-STATE-001（脏态）/ X-FORBID-005（装饰层不替换 DOM）/ TERM-ED-001（Block）/ TERM-ED-002（EditorDecoration）/ TERM-ED-003（GreenReview，扩展槽预留）。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - package.json                                 # 新增 @tiptap/react @tiptap/starter-kit @tiptap/pm
  - package-lock.json
  - src/components/ed/Editor.tsx                 # TipTap 宿主组件
  - src/components/ed/extensions/BlockIdExtension.ts  # 稳定 blockId（BR-ED-DATA-001）
  - src/components/ed/extensions/index.ts        # 扩展注册入口（预留 Diff/CopyRef/PageTop/FontSize 槽）
  - src/components/ed/EditorTab.tsx              # 标签栏 6 状态
  - src/store/edStore.ts                         # 编辑会话状态投影（SM-ED-01）
  - src/shell/Workbench.tsx                       # center 区挂载 Editor
  - src/styles.css                              # 编辑器样式
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md
  - .governance/active_scope.json
  - .governance/issues/PI-FE-05.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/**
confirmation: YES
scope_reason: PI-FE-05 纯前端 TipTap 接入（基础 md 编辑 + blockId + tab 6 态 + 扩展槽预留），需联网装 TipTap；人类已授权连续推进。
```

## Expected Behavior

- center 区接入 TipTap 3 编辑器，可编辑基础 md 文本（StarterKit）。
- BlockIdExtension 为每个顶层块赋稳定 blockId（不透明）：不删除该块的普通编辑不改变 blockId；
  块删除则其 blockId 失效不复用（BR-ED-DATA-001 / TERM-ED-001 Block）。
- EditorTab 6 状态视觉互斥：normal / dirty（红点）/ saving（旋转器）/ saved（回退默认）/
  pending_diff（红数字 badge）/ external_changed（金 !）。本 PI 接通 normal/dirty/saved（编辑→脏→存），
  saving/pending_diff/external_changed 预留接口（由保存链/DE/外部变更投影驱动）。
- extensions/index.ts 预留 DiffDecorationExtension（PI-FE-07 GreenReview/TERM-ED-003）、CopyReference、
  PageTopCaret、FontSize 注册槽，不破坏初始化顺序。
- 编辑会话/脏态前端投影（X-FORBID-007），落盘归后端（本 PI 不接 save）。

## Validation Commands

```
npx tsc --noEmit
npm run build
npm run governance:audit -- --issue .governance/issues/PI-FE-05.md
```

验收：tsc/build 绿；BlockIdExtension @GOV 追溯 BR-ED-DATA-001 + term_ref TERM-ED-001；
EditorTab 6 状态互斥；scoped 门禁无越界。
