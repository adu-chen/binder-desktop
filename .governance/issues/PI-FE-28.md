# Issue Trace · PI-FE-28　清理未接线的 EditorTab 死代码（收口 PI-FE-26/27 债务）

承接 PI-FE-26（center TabStrip 取代编辑器内置标签）/ PI-FE-27（A 类债务"EditorTab 组件保留未接线，后续清理"）。
EditorTab 自 PI-FE-26 起已被 center `TabStrip` 取代，除自身测试外无任何引用；按 CLAUDE.md §2（不保留死代码）清理。

## Scope Lock

```
task_type: CODE_CHANGE_MINOR
allowed_files:
  - src/components/ed/EditorTab.tsx                # 删除：未接线组件（被 TabStrip 取代）
  - src/components/ed/EditorTab.test.tsx           # 删除：随组件移除（无 covers: BR-* 不掉规则覆盖）
  - src/styles.css                                 # 移除死 .edtab* 样式块
  - src/store/edStore.ts                           # 注释引用 EditorTab → 标签（仅注释，无逻辑变更）
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md
  - .governance/active_scope.json
  - .governance/issues/PI-FE-28.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/**
confirmation: YES
scope_reason: 纯死代码清理（CODE_CHANGE_MINOR）：删除自 PI-FE-26 起已被 center TabStrip 取代、除自身测试外零引用的 EditorTab 组件及其测试与死 CSS；无行为变更、无规则覆盖损失（EditorTab.test 无 covers: BR-*）。收口 PI-FE-27 登记的 A 类债务。
```

## Expected Behavior

- 删除 `EditorTab.tsx` 与 `EditorTab.test.tsx`：组件无生产引用（仅其测试导入），标签渲染统一由 `TabStrip`（`.tab-*`）承担。
- 移除 `styles.css` 中 `.edtab*` 死样式块。
- `edStore.ts` 中 "EditorTab 6 状态" 注释改为中性表述（TabState 仍为 6 状态枚举，逻辑不动）。
- 无行为变更：多标签编辑/脏态/保存/外部变更/pending 全部不受影响。

## Validation Commands

```
npx tsc --noEmit
npm test
npm run build
npm run governance:audit -- --issue .governance/issues/PI-FE-28.md
```

验收：EditorTab 全仓零引用（含 CSS）；tsc + vitest（删 EditorTab 5 测试后剩 62）+ build 绿；scoped 门禁无自有文件越界。
