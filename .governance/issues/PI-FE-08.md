# Issue Trace · PI-FE-08 三种 AI 辅助入口 UI（⌘K InlineAssist / ⌘J Autocomplete）

承接 A-CORE-X-L-03 §2 PI-FE-08 / AG L2 / AG-ASSIST / BR-AG-STATE-003 / SM-AG-02（InlineAssist）/ SM-AG-03（Autocomplete）/ X-FORBID-007 / TERM-AG-003（InlineAssist）/ TERM-AG-004（Autocomplete）。

## Scope Lock

```
task_type: CODE_CHANGE
allowed_files:
  - src/store/assistStore.ts                      # ⌘K/⌘J 辅助入口状态（互斥）
  - src/components/ag/InlineAssistPopover.tsx      # ⌘K 选区上方对话弹窗
  - src/components/ag/AutocompleteOverlay.tsx      # ⌘J 光标处续写浮层
  - src/components/ed/Editor.tsx                   # ⌘K/⌘J 键盘绑定 + 挂载浮层
  - src/styles.css                               # 浮层样式
  - docs/00_core/A-CORE-X-L-03_前端开发计划.md
  - .governance/active_scope.json
  - .governance/issues/PI-FE-08.md
forbidden_files:
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/**
  - src/store/deStore.ts
confirmation: YES
scope_reason: PI-FE-08 纯前端 ⌘K/⌘J 辅助入口 UI；两入口均不产 DiffCard、不进 Diff 链路；人类已授权连续推进。
```

## Expected Behavior

- ⌘K InlineAssist（TERM-AG-003）：有选区时 ⌘K 在选区上方弹对话框，输入指令 → 生成 → 预览 → Apply
  **直接替换选区文本**（经 ED 受控编辑接口），**不生成 DiffCard、不进入 SM-DE-01**。
- ⌘J Autocomplete（TERM-AG-004）：⌘J 在光标处弹低权重浮层，3 条续写建议，Tab/↩ 选择插入光标处，
  **不产出 DiffCard、不进入 Diff 链路**。
- 两入口行为互斥且与 Chat Diff 区分（A-SYS-C-T-02 §6.6）：⌘K 替换选区 / ⌘J 续写补全 / Chat 才走完整
  Diff 链路；⌘K、⌘J 均不经 Chat 消息流、不更新 AgentTurn。
- 模型输出桩化（云网关实现期接通）；门禁前置语义（无订阅时辅助静默不提供，SM-AG-03）注释说明。
- 辅助 UI 态前端持有（视图层交互态，非 X-FORBID-007 限制的业务态）。

## Validation Commands

```
npx tsc --noEmit
npm run build
npm run governance:audit -- --issue .governance/issues/PI-FE-08.md
```

验收：tsc/build 绿；⌘K Apply 替换选区不产 DiffCard；⌘J 不进 Diff 链路；assistStore @GOV 追溯
BR-AG-STATE-003 + term_ref TERM-AG-003/004；scoped 门禁无越界。
