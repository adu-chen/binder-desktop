# Issue Trace · PI-FE-18 清理展示桩（编辑器空态 + 移除 DEMO_TOOLS）

承接 A-CORE-X-L-03 / CLAUDE.md §2（不保留半成品/占位冒充正常态）。清掉 AG 桩期遗留的展示物，使界面回到诚实空态；不影响已转真功能。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - src/components/ed/Editor.tsx                 # 无打开文档 → 空态（不显示假 untitled.md）
  - src/store/agStore.ts                          # 移除 DEMO_TOOLS（toolCalls 初始空）
  - src/components/ag/MessageList.tsx            # 工具卡仅在有真实 ToolCall 时渲染（去"占位演示"）
  - src/styles.css                              # 编辑器空态样式
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md
  - .governance/active_scope.json
  - .governance/issues/PI-FE-18.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/**
confirmation: YES
scope_reason: PI-FE-18 移除 AG 桩期展示数据（编辑器假 untitled.md 占位 + DEMO_TOOLS 6 态演示），界面回到诚实空态；ToolCallCard 组件保留待真实调用；人类已确认收掉。
```

## Expected Behavior

- 编辑器：未打开真实文档时显示空态提示（"从文件树打开一个文档开始编辑"），不再显示可编辑的假 untitled.md/
  "开始写作…" 占位；打开真实文件后正常渲染编辑区（标签/纸张/扩展/装饰不变）。
- 对话区：移除 agStore.DEMO_TOOLS（初始 toolCalls=[]）；MessageList 仅在存在真实 ToolCall 时渲染工具卡区，
  去掉"6 态占位演示"标题。ToolCallCard 组件保留（真实工具调用接通后由 SM-AG-01 驱动）。
- 不改动任何已转真功能（保存/diff/时间轴/搜索等）；不引入新依赖。

## Validation Commands

```
npx tsc --noEmit
npm test
npm run build
npm run governance:audit -- --issue .governance/issues/PI-FE-18.md
```

验收：未打开文档时编辑器为空态、对话区无演示工具框；打开文件后编辑正常；vitest 全过；scoped 门禁无越界。
