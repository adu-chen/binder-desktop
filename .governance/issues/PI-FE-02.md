# Issue Trace · PI-FE-02 Logo 与 Left Rail

承接 A-CORE-X-L-03 §2 PI-FE-02 / A-SYS-C-T-02 §3 Left Rail 界面分区 / VI 参考 docs/binder-vi。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - src/components/brand/BinderMark.tsx        # 段落条堆叠 Logo（移植 binder-parts.jsx BinderMark）
  - src/components/brand/BinderWordmark.tsx     # 文字标 binder.（移植 BinderWordmark）
  - src/components/shell/LeftRail.tsx           # 44px 5 模块导航栏
  - src/shell/Workbench.tsx                     # 挂载 LeftRail，提升导航激活态
  - src/styles.css                              # rail 按钮样式（token 驱动）
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md   # PI-FE-02 状态回写
  - .governance/active_scope.json               # Scope 镜像
  - .governance/issues/PI-FE-02.md              # 本 Issue Trace
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/**
confirmation: YES
scope_reason: PI-FE-02 纯前端展示层（Logo + 导航栏），无后端/无新依赖；人类已授权继续开发。
```

## Expected Behavior

- 左 Rail（44px）顶部渲染 BinderMark 段落条 Logo（8 条，宽度节奏 100/64/82/100/72/100/56/100，第 4 条锈红强调），与 VI 参考稿 D-06 一致。
- Rail 含 5 个模块导航项（工作区/记忆库/知识库/模板库/时间轴，编号 I·A～I·E）+ 底部设置项；点击切换激活项，激活项有锈红左条视觉。
- 激活面板为视图层导航态（非后端业务态），由 Workbench 本地状态持有，不违反 X-FORBID-007（该约束限制的是权威业务状态，非导航态）。
- BinderWordmark 渲染 `binder` + 锈红句点，IBM Plex Sans 500，字距 −0.04em。

## Validation Commands

```
npx tsc --noEmit
npm run governance:audit -- --issue .governance/issues/PI-FE-02.md
```

验收：tsc 绿；scoped 门禁无 SCOPE_OVERREACH / ISSUE_TRACE_MALFORMED；改动文件全部落在 allowed_files。
