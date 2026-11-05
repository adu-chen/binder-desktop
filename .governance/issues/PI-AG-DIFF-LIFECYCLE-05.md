# PI-AG-DIFF-LIFECYCLE-05 — Diff 预览生命周期与 block_ops 多块装饰修复

| 项 | 内容 |
|---|---|
| 状态 | READY |
| 严重级别 | P1（预览内容已进草稿但无法裁决；多块修改只标一个块） |
| 发现来源 | 人类测试反馈（截图 + `ED_BLOCK_OPERATION_RESULT_MISMATCH` 卡片） |
| 影响模块 | AG / DE / ED |
| 人类确认 | YES（"按上面修复"，2026-10-31） |

## 1. 现象

1. `block_ops` 卡片预览时稳定报 `ED_BLOCK_OPERATION_RESULT_MISMATCH`，内容不进入编辑器。
2. 修复该校验后预览成功，但页面重载把 pending diff 恢复成只读历史卡：无接受/拒绝按钮，
   底部批量条仍显示"1 条待裁决"，"全部接受"无效。
3. 绿标只覆盖锚点块一行，实际插入的 9 个新块没有绿标。

## 2. 根因

| 编号 | 根因 | 位置 |
|---|---|---|
| R1 | `nodeMatchesJson` 用后端最小 attrs（仅 blockId）与 PM 节点 `toJSON()`（含 schema 默认 `textAlign:null`）严格比较 | `src/services/edBlockOps.ts` |
| R2 | 历史恢复路径对所有 diff 设 `readonly:true`，但当前会话 pending + 已预览的 diff 也被包含；`attachMeta` 浅合并保留 `draftPreviewState`，产生"只读 + 已预览"混合卡 | `src/services/agService.ts` |
| R3 | 批量条按 `state==="pending"` 计数，动作却过滤 `!readonly`，计数与动作不一致 | `src/components/de/DiffAllActionsBar.tsx` |
| R4 | 绿标映射对 block_ops 使用 `operation.blockId`（插入操作即锚点），新块 id 从未进入映射 | `src/components/ed/Editor.tsx` |
| R5 | 预览 effect 不检查 `readonly`，且重载后可能对已含预览内容的草稿再次应用 | `src/components/ed/Editor.tsx` |

## 3. 修复（Scope Lock）

- `src/services/edBlockOps.ts`：按编辑器 schema 归一化后再比较（`nodeFromJSON`）。
- `src/services/agService.ts`：活跃会话且 `status=pending` 的 diff 恢复为可裁决；终态/其他会话仍只读。
- `src/components/ed/Editor.tsx`：block_ops 绿标使用插入块的 `afterNode.attrs.blockId`；
  预览前检查这些块是否已存在，已存在则只补记 `draftApplied`，不重复插入；只读卡不自动预览。
- `src/components/de/DiffAllActionsBar.tsx`：计数与动作统一过滤 `!readonly`。
- `src/components/de/DiffCard.tsx`：只读 pending 卡显示"仅可回看，不可裁决"。

不改后端协议、不在本 Issue 处理 beforeText 重复锚点展示。

## 4. 备选与取舍

- 重载后将 pending 一律置 expired：会丢失用户已看到的预览内容，且需要草稿回滚，风险更大。
- 批量条隐藏只读卡即可：只解决不同步，卡片仍不可用；故同时恢复可裁决 + 去重应用。

## 5. 验证

- `npx tsc --noEmit` 通过
- 前端全量（排除未跟踪 `.p1-spike`）：688/688
- `npm run governance:audit` 通过
- UI 行为需人类在重载后的卡片上复核（自动化未覆盖 Electron/WebView 交互）

## 6. 风险

- 若不同 diff 的新块 id 恰已存在（概率极低），会被判定为已应用；blockId 为 UUID 兜底生成。
- 只读历史卡仍不自动预览；这是刻意行为，避免重复插入。
