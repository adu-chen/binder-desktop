# Issue Trace · PI-FE-11 diff 真落盘前端接线（accept→ws.apply_diff）

承接 A-CORE-X-L-03（前端集成增量）/ A-DE-I-P-01 §2/§3（de.generate_diff/accept_diff/reject_diff/list_diffs）/ SM-DE-01 / DE-DIFF / X-INTER-001（accepted 唯一落盘）/ X-FORBID-007 / TERM-DE-001（PendingDiff）/ TERM-DE-003（DiffCard）。承 PI-BE-02 后端命令。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - src/services/deService.ts                    # generate/accept/reject/list 编排
  - src/store/deStore.ts                          # 去 demo，改真：addDiff + setDiffState 投影 SM-DE-01
  - src/components/de/DiffCard.tsx                # 裁决按钮接 deService
  - src/components/de/DiffAllActionsBar.tsx       # 全部接受（倒序）接 deService
  - src/components/ag/Composer.tsx               # "对当前文档生成示例修改" 入口（绕过桩模型）
  - src/styles.css                              # 入口按钮样式
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md
  - .governance/active_scope.json
  - .governance/issues/PI-FE-11.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/**
forbidden_note: 仅前端接线，不改后端
confirmation: YES
scope_reason: PI-FE-11 把 deStore/DiffCard 接到 PI-BE-02 的 de.* 真命令，使 accept 真落盘 + 编辑器重载；示例修改入口绕过仍是桩的 AG 模型；人类已授权继续。
```

## Expected Behavior

- "对当前文档生成示例修改"（Composer）：读当前打开文档内容 → 确定性本地变换（追加一段）→ de.generate_diff
  （relPath + afterContent）→ deStore 新增 DiffCard（pending），本地保留 before/after 预览。无打开文档则禁用。
- DiffCard 接受：pending→validating→applying→**de.accept_diff（真写盘+记时间轴节点）**→accepted（含时间轴号）→
  **编辑器重载该文档**（ws.read_doc 投影新内容）；失败→error（可重试）。
- DiffCard 拒绝：de.reject_diff（不写文件）→rejected。
- DiffAllActionsBar 全部接受：倒序逐条 de.accept_diff（文档靠后先写避免偏移）。
- 移除 deStore demo 数据；diff 来自真实生成/裁决。权威落盘态以后端为准（X-FORBID-007）。
- 8 视觉态仍为后端 SM-DE-01（pending/accepted/rejected/expired）投影 + 过渡态，不另立平行业务机。

## Validation Commands

```
npx tsc --noEmit
npm run build
npm run governance:audit -- --issue .governance/issues/PI-FE-11.md
```

验收：tsc/build 绿；打开文档→生成示例修改→接受→文件真被写入+编辑器显示新内容+时间轴号；拒绝不改文件；scoped 门禁无越界。
