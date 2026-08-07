# Issue Trace：ED-SELECTION-PERSIST-01

## 目标

编辑器选区持久化：失焦（blur）后选区高亮不消失，保持到下一次光标操作（任何 transaction，含点击编辑器空白处）才清除。根因：ProseMirror selection state 失焦后仍保留，但浏览器原生 `::selection` 在失焦时不渲染高亮 → 视觉上选区消失。方案 B+C：失焦时用 `Decoration.inline` 为当前选区范围加背景高亮（视觉副本）；focus/任意 transaction/selectionUpdate 时移除装饰，恢复原生行为。

## 规则与链路

- 模块：ED（编辑器）
- 链路：ED-EDIT / ED-DECOR（decoration 叠加，复用既有 infra）
- 复用规则：X-FORBID-005（decoration 不替换 DOM、不阻挡编辑）；不新增规则/链路/约束/术语。

## 设计一致性

- 新扩展 `SelectionPersistExtension`（仿 DiffDecorationExtension 范式）：PluginKey 持 DecorationSet。
- blur（复用 handleDOMEvents.blur 或扩展内）→ 若 selection 非空（TextSelection 非空 或 非 collapsed），写入持久化装饰（inline Decoration，class `pm-selection-persist`）。
- 清除时机：focus / 任意 transaction（含 docChanged、点击空白处产生的 selection 变化、输入）→ 移除装饰。
- 不修改 PM selection state；不替换 DOM、不阻挡编辑（X-FORBID-005）。
- NodeSelection（图片/表格等选中态）已有原样高亮，不需 inline decoration；本扩展只处理 TextSelection。
- 分页模式下装饰挂 doc 上，CSS 分页不影响。
- styles.css 新增 `.pm-selection-persist` 背景样式（聚焦态颜色近似，比原生略淡以区分）。

## 文件范围

- `src/components/ed/extensions/SelectionPersistExtension.ts`（新：扩展 + 纯逻辑 + 单测）
- `src/components/ed/extensions/index.ts`（注册扩展）
- `src/styles.css`（`.pm-selection-persist` 样式）
- `docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md`（变更记录 v1.59）
- `.governance/issues/ED-SELECTION-PERSIST-01.md`（本文）
- `docs/00_core/A-CORE-C-R-01_ADUS.md`（重新生成）

## 验收

- 失焦后选区高亮保持；聚焦或任何 transaction 后清除（含点击空白处）。
- 持久化装饰不干扰原生编辑、不替换 DOM。
- 相关测试、tsc、governance:generate/audit、build 通过。

## Task Runtime Report — 选区持久化（两次链路修复 + v2 统一链路 + 跨文档/输入面 1+2）

task_type: CODE_CHANGE
governance_phase: IMPLEMENTATION
final_state: READY
issue_trace: ED-SELECTION-PERSIST-01

changed_files:

- `src/components/ed/extensions/SelectionPersistExtension.ts`：新扩展——v2 统一链路：decorations 实时跟随非空 TextSelection（聚焦也渲染 span），失焦回退 lastNonEmpty 记录；persist/clear 事务桥接。
- `src/components/ed/Editor.tsx`：editorProps.handleDOMEvents 桥接 blur（延迟派发 persist，待 view.hasFocus() 归 false）→ focus（clear）；切换文档（loadSeq 变更）时 clearPersistedSelection 清记录（持久化绑定文档，问题 1）。
- `src/components/ed/extensions/index.ts`：注册扩展。
- `src/styles.css`：原生 ::selection 透明禁用，统一由 PM Decoration 渲染；.pm-selection-persist 背景 rgba(37,99,235,.16)+描边；agent 输入面（.composer-prosemirror 输入框 + .msglist 会话流）统一品牌蓝 ::selection rgba(37,99,235,.30)（问题 2，纯文本不做持久化）。
- `src/components/ed/extensions/SelectionPersistExtension.test.ts`：纯逻辑 3 用例。
- `src/components/ed/extensions/SelectionPersistExtension.integration.test.ts`：统一链路集成 7 用例（含切文档清记录）。
- `src/components/ed/extensions/SelectionPersistExtension.dom.test.ts`：DOM 渲染 5 用例。
- `docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md`：v1.59（v2 统一链路 + ④生命周期注记）。
- `docs/00_core/A-CORE-C-R-01_ADUS.md`：重新生成。

test_impact:

- direct: ED, AG（styles.css 影响 Composer 输入面样式）
- affected: 无

validation:

- `npx vitest run src/components/ed/extensions/SelectionPersistExtension.test.ts src/components/ed/extensions/SelectionPersistExtension.integration.test.ts src/components/ed/extensions/SelectionPersistExtension.dom.test.ts`：passed（15/15，DOM 断言）。
- `npm run test:related`：passed（21 文件 / 446 测试）。
- `npm run governance:generate`：passed。
- `npm run governance:audit`：passed（governance_findings 清零）。
- `npx tsc --noEmit`、`npm run build`：passed。

unresolved_items:

- 问题 3（跨编辑面选区互斥 + assist 豁免）：人类决策放弃修复，接受当前效果（2026-08-07）。
- Composer 输入框"分行"统一：人类决策放弃（接受当前品牌蓝连续块），不引入额外复杂度（2026-08-07）。

## 闭合确认

- 人类 2026-08-07：接受当前效果，闭合本 Issue（v2 统一链路 + 跨文档生命周期 1+2 已生效）。
- 遗留变更未提交：v2 统一链路与问题 1+2 的改动在工作区（与并行 MODEL-CONFIG-01 改动共存），待用户指定提交。
