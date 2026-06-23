# Issue Trace · PI-FE-07 Diff 状态机前端（DiffCard + GreenReview + 批量条）

承接 A-CORE-X-L-03 §2 PI-FE-07 / DE L2（A-DE-M-T-01）/ SM-DE-01 / DE-DIFF / BR-DE-STATE-001 / ED-DECOR / BR-ED-STATE-002 / X-FORBID-005（装饰叠加不替换 DOM）/ X-INTER-001（DiffCard↔GreenReview 同步）/ X-FORBID-007 / TERM-DE-001（PendingDiff）/ TERM-DE-003（DiffCard）/ TERM-ED-002（EditorDecoration）/ TERM-ED-003（GreenReview）。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - src/store/deStore.ts                          # PendingDiff 投影 + 裁决动作（8 态映射后端 SM-DE-01）
  - src/components/de/DiffCard.tsx                 # DiffCard 8 状态 + 5 expired 细分原因
  - src/components/de/DiffList.tsx                 # 对话区 diff 卡列表
  - src/components/de/DiffAllActionsBar.tsx        # 批量条 3 状态（倒序应用）
  - src/components/ed/extensions/DiffDecorationExtension.ts  # GreenReview（ProseMirror Decoration 叠加）
  - src/components/ed/extensions/index.ts          # 注册 DiffDecorationExtension（填扩展槽）
  - src/components/ed/Editor.tsx                   # 从 deStore 同步 GreenReview 状态到装饰层
  - src/components/ag/ChatPanel.tsx               # 对话区挂载 DiffList
  - src/shell/Workbench.tsx                        # DiffStrip 挂载批量条 + 待裁决计数
  - src/styles.css                               # diff 卡 / GreenReview / 批量条样式
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md
  - .governance/active_scope.json
  - .governance/issues/PI-FE-07.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/**
confirmation: YES
scope_reason: PI-FE-07 纯前端 Diff 状态机 + GreenReview 装饰层 + 批量条 + 双向同步；8 视觉态为后端 SM-DE-01 投影 + 过渡态，不另立业务机；人类已授权连续推进。
```

## Expected Behavior

- DiffCard 8 状态视觉互斥（参考稿 §6.1）：pending（锈红实边）/ validating（金旋转）/ applying（金进度）/
  accepted（森林绿）/ rejected（灰弱化）/ expired（灰边+原因）/ conflict（金+冲突 id）/ error（锈红+错误码）。
  权威态为后端 SM-DE-01（pending/accepted/rejected/expired）；validating/applying/conflict/error 为裁决
  往返的过渡/呈现态，非前端另立的平行业务态（X-FORBID-007）。
- expired 5 种细分原因各有引导文案：document_revision_advanced / block_resolve_failed /
  overlapping_range / external_file_changed / apply_replace_failed。
- GreenReview（TERM-ED-003）：DiffDecorationExtension 基于 ProseMirror Decoration 按 blockId 叠加在正文上，
  不替换 DOM、不阻挡编辑（X-FORBID-005）；4 态 pending/applying/accepted/expired。
- DiffCard ↔ GreenReview 双向同步：同一 deStore 单源，任意一侧裁决另一侧同步（X-INTER-001）。
- DiffAllActionsBar 3 状态：有待裁决（红计数+全部接受/拒绝）→ 倒序应用中（金进度，文档靠后先应用避免行号偏移）
  → 完成（绿+失效统计）。
- 裁决落盘走 accepted → ws.apply_diff（后端，实现期接线）；本 PI 前端状态机 + 装饰层为主，diff 数据为投影占位。

## Validation Commands

```
npx tsc --noEmit
npm run build
npm run governance:audit -- --issue .governance/issues/PI-FE-07.md
```

验收：tsc/build 绿；DiffCard 8 态互斥 + expired 5 原因各有文案；GreenReview↔DiffCard 双向同步；
deStore @GOV 追溯 BR-DE-STATE-001 + term_ref TERM-DE-001/003；DiffDecorationExtension @GOV 追溯
BR-ED-STATE-002 + term_ref TERM-ED-003；scoped 门禁无越界。
