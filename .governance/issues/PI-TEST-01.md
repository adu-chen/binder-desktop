# Issue Trace · PI-TEST-01 前端测试（单元 + 契约层，主干已转真部分）

承接 A-CORE-C-P-01 §10（测试策略：单元/集成）/ A-CORE-X-L-03（前端）/ 各模块 L2 规则。仅测已转真主干（WS/ED/DE 落盘/时间轴/workspace 生命周期），排除仍是桩的 AG（DEMO_TOOLS/流式模拟/⌘K⌘J 桩）与 e2e（G5）。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - src/store/deStore.test.ts                    # 新增：diff 状态投影 reducer
  - src/store/wsStore.test.ts                    # 新增：文件树状态投影
  - src/store/workspaceShell.test.ts             # 新增：外壳态投影
  - src/components/ws/FileTree.tsx               # 导出 buildTree 供测（纯函数）
  - src/components/ws/buildTree.test.ts          # 新增：扁平 DocRef → 树
  - src/services/wsService.test.ts               # 新增：mock invoke 契约（CRUD/错误）
  - src/services/edService.test.ts               # 新增：mock invoke 契约（保存/冲突）
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md
  - .governance/active_scope.json
  - .governance/issues/PI-TEST-01.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/**
confirmation: YES
scope_reason: PI-TEST-01 给已转真主干补单元+契约测试（vitest + vi.mock，无新依赖）；排除 AG 桩与 e2e；人类已授权进入测试。
```

## Expected Behavior

- deStore：addDiff/setDiffState（三终态互斥不可逆，accepted/rejected/expired 不被覆盖）/toGreenReview 映射/reset。覆盖 BR-DE-STATE-001 前端投影。
- wsStore：loading/ready/error/reset 投影；workspaceShell：active/recent/notice/reset。
- buildTree：扁平 DocRef rel_path → 嵌套树，目录在前、按名排序。
- wsService（mock invoke）：createDoc/renameDoc/deleteDoc/checkExternalChange 经 dispatch 信封正确构造 + result/error 互斥（错误抛出 IpcError）。
- edService（mock invoke）：saveDocument 成功 markSaved；WS_SAVE_CONFLICT → tabState=external_changed（不静默覆盖）；pollExternalChange changed→external_changed。
- 全部 node 环境（不渲染组件，无需 jsdom）；不引入新依赖；不测 AG 桩。

## Validation Commands

```
npm test
npx tsc --noEmit
npm run build
npm run governance:audit -- --issue .governance/issues/PI-TEST-01.md
```

验收：vitest 全过（新增用例）；tsc/build 绿；scoped 门禁无越界。
