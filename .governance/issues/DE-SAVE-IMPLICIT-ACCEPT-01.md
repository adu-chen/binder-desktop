# Issue Trace — 保存即接受已应用 Diff

## 1. 用户可见症状

已应用到编辑器草稿的 pending Diff 在用户普通保存时会随文档写入磁盘，但旧实现只在前端把卡片改成
`accepted`，没有同步后端的 `pending_diffs`。这会造成磁盘、前端显示和后端裁决态不一致。

## 2. 入口点

| 入口 | 文件 | 函数 |
|---|---|---|
| 文本/Markdown/DOCX 保存 | `src/services/edService.ts` | `saveDocument` / `saveMarkdownDocument` / `saveDocxDocument` |
| 草稿接受 | `src-tauri/src/lib.rs` | `route_accept_draft_diff` |
| 保存效果确认 | `src-tauri/src/lib.rs` | `route_save_doc` / `route_save_from_edit` |
| 前端投影 | `src/store/deStore.ts` | `markDraftSaved` |

## 3. 相关规则与链路

`BR-DE-STATE-001`、`BR-ED-STATE-003`、`BR-SYS-DATA-005`；`DE-DIFF → ED-EDIT → WS-SAVE`。

## 4. 不变量与决策

人类已确定：用户触发任意保存即接受该文档已应用的 pending Diff。保存前必须将这些 Diff 经
`de.accept_draft_diff` 持久化为 `accepted + awaiting_save`；保存成功后既有 WS 回调将其翻转为
`saved`。保存失败时保留 `accepted + awaiting_save`，等待下一次保存，不得伪造已保存。

## 5. 验证计划

- 三种保存入口均先调用 `de.accept_draft_diff`，再调用对应保存 command。
- 接受调用成功后前端投影为 accepted；保存成功后才写 `draftSaved=true`。
- 不含已应用 pending Diff 的保存顺序不变；接受调用或保存调用失败均不伪造 saved。
- 运行相关前端测试、类型检查与治理审计。

## 6. 验证结果

- 2026-07-27：`npm exec tsc -- --noEmit` 通过。
- 2026-07-27：`npm test -- src/services/edService.test.ts src/store/deStore.test.ts` 通过（2 个测试文件、23 个断言）；覆盖文本、Markdown、DOCX 三种保存入口的「先接受、后保存」顺序，以及保存失败保留 accepted/未 saved 的路径。
- 2026-07-27：`npm run governance:audit` 通过（无 governance findings）；仓库既有的 TERM_ORPHAN / BLOCK_UNANNOTATED 报告为 warning，非本课题新增阻断项。
- 2026-07-27：`git diff --check` 通过。

## 7. 范围外验证发现

`npm test -- src/services/deService.test.ts src/components/de/DiffCard.test.tsx src/services/edService.test.ts src/store/deStore.test.ts`
中 52 个断言有 51 个通过；唯一失败的是
`deService.test.ts` 的 `applyOpenSessionDiff：已打开非激活文档直接应用 LogicalState 并确认 accepted`。
当前 `src/services/deService.ts`（工作区既有的 2026-07-26 D1 改动）已明确把该函数调整为“只应用草稿，
不自动调用 `de.accept_draft_diff`”，但测试仍断言它会调用后端接受命令，故调用列表为空。该文件不在
DE-SAVE-IMPLICIT-ACCEPT-01 的 Scope 内，本课题未修改；建议后续在 D1 对应课题中同步测试与最终产品语义。

## 8. 闭合报告

- final_state：CLOSED
- 闭合日期：2026-07-27
- 已交付：三条保存入口（普通文本、Markdown、BDocx）均先将当前文档已应用的 pending Diff 持久化为
  `accepted + awaiting_save`，再进入真实保存；保存成功后才投影 `draftSaved=true`，保存失败不伪造已保存。
- 定向验证：`npm exec tsc -- --noEmit` 通过；`npm test -- src/services/edService.test.ts src/store/deStore.test.ts`
  通过（2 文件、23 测试）；`npm run governance:audit` 通过（无 governance findings）；`git diff --check` 通过。
- 未解决项：§7 的 `deService.test.ts` 旧断言与既有 D1 行为冲突，未纳入本 Scope；它不影响本课题保存入口
  的实现与定向验证，后续须在 D1 课题中单独裁决并修复。
