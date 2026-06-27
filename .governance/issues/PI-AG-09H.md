# Issue Trace · PI-AG-09H 内联 @ 自动补全（B 类客户端打磨）

承接 PI-AG-09E（按钮选择器 → 内联 @ 快捷路径）。在输入框打 @ 即弹文档下拉，加速整篇引用。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: PI-AG-09H
allowed_files:
  - src/components/ag/Composer.tsx   # 内联 @ token 检测 + 下拉 + 选中加 mention/删 @token
  - src/styles.css                    # 内联下拉样式（append）
  - .governance/active_scope.json / issues/PI-AG-09H.md
forbidden_files: ADU/ADP/CLAUDE/APC / ipc/mod.rs / edStore / wsStore
confirmation: YES
```

## Expected Behavior

- 输入变化时检测光标前的活跃 @ token（@ 前为行首/空白、@ 后到光标无空白）→ query。
- query 活跃时弹下拉：按 `relPath` 含 query（不分大小写）过滤工作区文档（非目录），最多若干条。
- 选中：加入 mentions（去重）+ 从文本删除该 @token；Esc / 无匹配 / 失焦 → 关闭。
- Enter（下拉开、无修饰键）选首条；既有 ⌘/Ctrl+Enter 发送、按钮选择器（块展开）、@引用选区 保留。

## Validation Commands

```
npx tsc --noEmit
# 端到端：输入 @ + 文件名片段 → 下拉 → 选中成 chip、@token 消失（人类自验）
```

验收：tsc 干净；@ token 检测正确（@ 前须空白/行首、后无空白）；选中加 mention 且删 @token；Esc 关闭；不破坏发送/选区/按钮选择器。

## 后续

- 内联 @ 也支持块（@文件#块）；方向键导航；高亮匹配片段。
