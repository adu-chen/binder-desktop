# Issue Trace · PI-TEST-02 组件状态机渲染测试（jsdom + @testing-library）

承接 APC §10（单元测试）/ A-CORE-X-L-03。测产品识别度最高的**互斥状态可视化**组件（已转真/spec 固定）：FileTreeNode 6 态、DiffCard 8 态、EditorTab 6 态。排除 AG 桩组件（ToolCallCard/MessageList demo）。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - package.json                                 # jsdom + @testing-library/react + @testing-library/dom
  - package-lock.json
  - src/components/ws/FileTreeNode.test.tsx       # 6 态渲染
  - src/components/de/DiffCard.test.tsx           # 8 态 + expired 原因渲染
  - src/components/ed/EditorTab.test.tsx          # 6 态标记渲染
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md
  - .governance/active_scope.json
  - .governance/issues/PI-TEST-02.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/**
confirmation: YES
scope_reason: PI-TEST-02 装 jsdom + @testing-library（本地测试库，离线运行），测互斥状态机组件渲染；排除 AG 桩；人类已授权按顺序继续测试。
```

## Expected Behavior

- 每个组件测试文件用 `@vitest-environment jsdom` docblock（仅这些文件用 jsdom，单元/契约测试仍 node）。
- FileTreeNode：normal / open（锈红左条）/ dirty_open（红点）/ pending_diff（数字 badge）/ external_changed（金 !）/
  not_supported（xlsx 灰禁用）六态各自渲染正确、互斥。
- DiffCard：8 态 badge 文案 + 可用按钮互斥（pending 接受/拒绝；conflict 接受禁用；expired 重新生成+原因文案；
  error 重试+错误码；accepted 时间轴号）。
- EditorTab：dirty 红点 / saving 旋转器 / external_changed 金 ! / pending_diff badge 各标记互斥。
- 仅断言渲染产物（不触发点击调后端）；不引入对 AG 桩的测试。

## Validation Commands

```
npm test
npx tsc --noEmit
npm run build
npm run governance:audit -- --issue .governance/issues/PI-TEST-02.md
```

验收：vitest 全过（含组件渲染用例）；tsc/build 绿；scoped 门禁无越界。
