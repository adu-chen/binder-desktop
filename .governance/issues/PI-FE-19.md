# Issue Trace · PI-FE-19 状态投影接线（文件树/编辑器标签反映真实态）

承接 A-SYS-C-T-02（前端态投影后端状态机）/ 参考稿 §6.3-6.4（EditorTab/FileNode 状态）/ X-FORBID-007 / BR-ED-STATE-001 / BR-DE-STATE-001。把已建的 6 态/8 态可视化接到真实 edStore/deStore 投影（此前仅测试用 override）。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - src/store/deStore.ts                          # DiffView 补 relPath（按文件归属）
  - src/services/deService.ts                     # generateSampleDiff 填 relPath
  - src/components/ws/FileTree.tsx               # 投影打开文档的 dirty/external/pending 态到节点
  - src/components/ed/Editor.tsx                 # EditorTab 反映 pending_diff/external_changed
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md
  - .governance/active_scope.json
  - .governance/issues/PI-FE-19.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/**
confirmation: YES
scope_reason: PI-FE-19 把文件树/编辑器标签的 6 态可视化接到真实 edStore/deStore 投影；DiffView 补 relPath 使 pending 按文件正确归属；纯前端；人类已授权继续。
```

## Expected Behavior

- DiffView 增 relPath（生成时填当前打开文档），使 pending diff 按文件归属。
- 文件树：当前打开文档节点按优先级投影 external_changed（金!）> pending_diff（该文件 pending 数 badge）>
  dirty_open（红点）> open（锈红左条）> normal——均来自真实 edStore.tabState / deStore.diffs（X-FORBID-007）。
- 编辑器标签：external_changed > pending_diff（该文档 pending 数）> edStore.tabState（dirty/saving/saved）。
- 不再用写死 override；状态全部来自真实 store。各态互斥（参考稿 §6.3/6.4）。

## Validation Commands

```
npx tsc --noEmit
npm test
npm run build
npm run governance:audit -- --issue .governance/issues/PI-FE-19.md
```

验收：编辑脏态→文件树红点 + 标签红点；生成 diff→该文件 badge；外部变更→金!；vitest 全过；scoped 门禁无越界。
