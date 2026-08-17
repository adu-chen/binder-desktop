# Issue Trace — ED-MARKDOWN-SAVE-ERROR-CLARITY-01：Markdown 保存失败提示不明确（债务登记）

文档状态：A
任务类型：DEBT
治理阶段：IMPLEMENTATION
创建时间：2026-08-15

## 1. 用户可见症状

用户在工作区新建 `a.md`，打开文档后手动粘贴非 Markdown 格式内容（如富文本/HTML 复制），保存时失败，
提示：`保存失败：a.md: Markdown 语义往返校验失败，已阻止写盘`。用户无法从提示得知失败原因
（非 Markdown 内容），误以为产品 bug。

## 2. 根因分析（已由用户实测确认）

- 粘贴的非 Markdown 内容经 Tiptap/ProseMirror 进入结构化 LogicalState 后，`serializeMarkdownDocument`
  （`src/components/ed/markdown/markdownCodec.ts:179`）执行 `parse → serialize → parse` 语义等价校验
  （BR-ED-VERIFY-001）：序列化回 Markdown 再解析后语义不等价 → 返回 `ED_MARKDOWN_ROUNDTRIP_FAILED`
  （markdownCodec.ts:192）→ 阻止写盘。
- 这是**符合设计的安全行为**（BR-ED-VERIFY-001：非支持结构/无法语义往返的内容禁止静默写盘），
  **不是 bug**；粘贴 Markdown 格式内容后可正常保存。
- 衍生：新建 `a.md` 打开后日志出现 `ed open tabs skipped without session snapshot: a.md`
  （workspaceService.ts:90，恢复打开标签时该文档无 ed_session_snapshots 记录）——为独立观察项，
  未影响本症状（保存失败主因是语义往返校验）。

## 3. 债务内容

- **失败行为保留**：语义往返校验失败继续阻止写盘（BR-ED-VERIFY-001 冻结语义，不放宽）。
- **提示需明确**：当前 `ED_MARKDOWN_ROUNDTRIP_FAILED` 文案未说明原因与处置。改进方向：
  1. 区分并提示具体原因：粘贴内容含非 Markdown 结构 / 无法语义往返（如嵌入 HTML、脚注、复杂表格、
     unsupported 节点）——提示用户"内容包含 Markdown 不支持的格式，请粘贴纯文本/Markdown 后重试"；
  2. 错误信息包含失败文档路径与可操作建议（放弃本次写盘，内容仍保留在编辑器中不丢失）；
  3. 保存失败时保持编辑器 dirty/未保存状态（当前已如此），并在 UI 上可见（已有保存失败提示）。
- 关联规则：BR-ED-VERIFY-001、BR-ED-DATA-005、BR-ED-DATA-006。

## 4. 改进验收标准（后续实现时）

- 粘贴非 Markdown 内容保存失败：提示明确（原因 + 处置建议 + 内容不丢失），不再被误认为 bug；
- 粘贴合法 Markdown 内容保存成功：语义往返校验行为不变。

## 5. 关联链路

ED-EDIT（Markdown 保存守卫）；ED-DATA（LogicalState 编码）。

## 6. 人类确认记录

2026-08-15：人类实测定位根因（复制非 Markdown 内容触发语义往返失败）；决定：**记录债务、保持失败
行为、改进提示明确性**。confirmation: YES。

## 7. Task Runtime Report

- task_type：`DEBT`（登记，不修代码）
- governance_phase：`IMPLEMENTATION`
- final_state：`READY`
- issue_trace：本文件
- changed_files：本文件（债务登记）
- validation：无代码改动；audit 不受影响。
- unresolved_items：提示明确性改进（原因区分/处置建议）待后续实现批次；`ed open tabs skipped without
  session snapshot`（新建文档打开无 snapshot）为独立观察项，列入后续排查。
