# Issue Trace · PI-AG-08H ED 真实锚联动（ed.locate_block）

承接 PI-AG-08G（blockId 锚 → 由 AG 侧快照解析升级为对**实时编辑器**校验）/ A-ED（types/mod.rs：
`ed.locate_block` 真实 TipTap 定位归前端，后端消费 LocateResult）/ A-DE-M-T-01 BR-DE-DATA-001
（锚失效不误改）/ A-REF inject VERIFY-001（块级经 ed.locate_block 校验锚）。

## 背景

`ed.locate_block` 设计上归前端（实时 TipTap 定位），此前**未实现**；AG 块编辑只对 turn 起的内容快照
（edStore.loadedContent）自解析校验。本 PI 实现真实前端定位 + 在 AG 块编辑落卡前对**实时编辑器** 校验锚，
失效（用户在 AI 思考期间改了文档、目标块已不在）则卡片显式警示，不静默。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: PI-AG-08H
allowed_files:
  - src/services/edLocate.ts     # 新建：locateBlock(blockId) 实时 DOM 定位 → LocateResult
  - src/services/agService.ts    # tool_request(propose_edit) 捕获 block_id；diff_proposed 校验锚 → 卡片警示
  - .governance/active_scope.json / issues/PI-AG-08H.md
forbidden_files: ADU/ADP/CLAUDE/APC / ipc/mod.rs（契约不变）/ Editor.tsx / edStore / deStore
confirmation: YES
```

## Expected Behavior

- `edLocate.locateBlock(blockId) -> "located" | "anchor_lost"`：实时查询编辑器渲染 DOM
  `[data-block-id="…"]`（TipTap BlockIdExtension 渲染），真实反映当前编辑器是否存在该块（不需编辑器实例）。
- `agService`：`tool_request`（tool_name=propose_edit）时从 params 捕获 `block_id`（既有事件已带 params，
  不改契约）；随后的 `diff_proposed` 用 `locateBlock` 对实时编辑器校验该锚——`anchor_lost` 时 `deStore.addDiff`
  的 title 加「⚠ 文档已变动·待复核」并带 block_id；`located` 正常。turn 内捕获，每回合重置。

## Validation Commands

```
npx tsc --noEmit
# 端到端：AI 块编辑后趁机改动文档使目标块消失 → diff 卡片显示「待复核」警示（人类自验）
```

验收：tsc 干净；locateBlock 实时 DOM 命中/未命中正确；锚失效卡片显式警示（不静默误导）；正常块编辑无警示。

## 后续

- 与后端 DE 真实联动（把实时 LocateResult 回传后端，块级定位 diff 应用，替整篇重建）；
  ed.locate_block 升级为正式 ed.* 命令（经编辑器实例精确位置，非仅存在性）。
