# 情况说明：MODEL-CONFIG-01 部分改动在提交过程中丢失

**致**：并行开发者（MODEL-CONFIG-01 负责人）
**来自**：选区持久化修复（ED-SELECTION-PERSIST-01）负责人
**日期**：2026-08-07
**性质**：跨任务协作交接 / 改动丢失说明

---

## 1. 背景

在修复 ED-SELECTION-PERSIST-01（选区持久化 v2 统一链路）时，为隔离并行改动、避免混入提交，我对共享文件 `src/components/ed/Editor.tsx` 与 `src/styles.css` 做过**临时移除**操作（移除其中 MODEL-CONFIG-01 的段落，以便单独提交本次选区改动）。该移除操作中途被终止，未能完整还原。

此后并行协作者基于此状态提交了 `570e0cb`（MODEL-CONFIG-01），导致部分 MODEL-CONFIG-01 改动**未进入任何 commit**，现已丢失。

**影响确认**：tsc/cargo 均通过（不报编译错误），但为**功能静默降级 + 样式缺失**。

## 2. 丢失内容清单（共 2 项，均确认未在任何 commit 中出现）

### 2.1 Editor.tsx — autocomplete 模型能力偏好解析段（功能丢失）

`git log -S "autocompleteModel"` 全历史无结果。需恢复的内容（在 `openAutocomplete` 流程内，`const requestId = assistStore.openAutocomplete(anchor, target);` 之后）：

```ts
    const requestId = assistStore.openAutocomplete(anchor, target);
    // 模型参数按 autocomplete 能力偏好解析（BR-AG-DATA-007：跟随全局或显式指定）。
    const autocompleteModel = modelStore.resolveFor("autocomplete");
    void requestAssist({
      requestId,
      kind: "autocomplete",
      docPath: docCtx.openPath,
      loadSeq: docCtx.loadSeq,
      blockId: target.blockId,
      start: target.start,
      end: target.end,
      prefix,
      before: target.before,
      after: target.after,
      provider: autocompleteModel.provider,
      model: autocompleteModel.model,
    })
```

即：新增 `autocompleteModel` 常量 + 在 requestAssist 参数中补 `provider` / `model` 两字段。

### 2.2 styles.css — ModelConfig / 来源标记 class 定义（样式缺失）

`ModelConfig.tsx` 仍引用以下 class（2 处），但 styles.css 中定义已不存在（`git log -S "modelcfg-src"` 无结果）：

```css
/* 来源标记（BR-AG-DATA-006）：下拉内辨识预设/BYOK */
.cbar-pop-src { font-size: 10px; padding: 1px 4px; border-radius: 3px; background: var(--bg-card); border: 0.5px solid var(--rule); white-space: nowrap; }

/* 来源分层标记（BR-AG-DATA-006：preset/BYOK 分组与辨识） */
.modelcfg-src { font-size: 10px; padding: 1px 4px; border-radius: 3px; background: var(--bg-card); border: 0.5px solid var(--rule); }
.modelcfg-group-gap { margin-top: var(--sp-2); }

/* 能力模型偏好区（BR-AG-DATA-007） */
.modelcfg-cap-row { display: flex; align-items: center; justify-content: space-between; gap: var(--sp-2); }
.modelcfg-cap-label { font-size: 12px; text-transform: none; letter-spacing: 0; color: var(--ink); }
.modelcfg-cap-select {
  min-width: 0; flex: 1; max-width: 260px; padding: 4px 6px; font-size: 12px; color: var(--ink);
  border: 0.5px solid var(--rule); background: var(--bg-card); border-radius: var(--r-1);
}
```

## 3. 恢复源（唯一备份）

恢复内容已完整备份于**本机 `/tmp`**（重启可能清空，建议尽快固化）：

| 备份文件 | 内容 |
|---|---|
| `/tmp/Editor.tsx.bak` | 含丢失的 autocompleteModel 段（约 L1020-1042） |
| `/tmp/styles.css.bak` | 含全部丢失 class 定义（L1139-1140、L1179-1185 附近） |

## 4. 建议处理

1. 从备份中恢复上述 2 项内容到对应文件
2. 以独立 commit 补提（建议标题含 `MODEL-CONFIG-01` 与 BR-AG-DATA-006/007 引用）
3. 若 `/tmp` 备份已丢失，可联系我重新生成恢复 patch

## 5. 责任确认

- 丢失由 ED-SELECTION-PERSIST-01 的临时移除操作间接引发，非 MODEL-CONFIG-01 代码本身问题
- 本次选区修复（`29cfd07`）自身完整，与上述丢失无关

---

**附**：完整审计报告见会话记录（tsc/cargo 通过、选区测试 7/7、工作区干净、分支 feat/sm-state-machine-runtime）。
