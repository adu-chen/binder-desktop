# Issue Trace — BDocx 表格代理编辑

## 1. 用户可见症状

BDocx 编辑器可显示基础表格，工具栏仅有固定 3×3 插入和部分行列命令；代理侧却只获得普通 Block 投影，
无法可靠识别表格、单元格坐标、合并关系或可执行操作。以 `replace_block` 直接替换 HTML 会与嵌套
`table/td/th` 块重叠，可能生成错误 Diff 或破坏表格结构。

## 2. 根因与入口

| 链路 | 当前入口 | 根因 |
|---|---|---|
| 人工编辑 | `EditorToolbar` → TipTap Table commands | 操作入口不完整，缺合并/拆分、表格文档流移动与可见的表格专属工具组 |
| 代理读取 | `list_blocks` → `AgentEditableSnapshot` | HTML 字符串块扫描不表达行、列、跨度或表格身份 |
| 代理写入 | `propose_edit` → `replace_block/style_ops` | 工具 Schema、提示词、后端校验与 DiffPayload 均无表格语义 |
| 草稿审阅 | `DiffCard` / `deService` | 只能呈现文本 before/after，不能预览结构效果或以原子方式重放表格操作 |

## 3. 本课题不变量

1. 表格结构修改必须以受限 `TableOperation` 表达；代理不得提交原始 HTML。
2. 表格目标必须同时匹配编辑会话、文档内容与表格拓扑指纹；任一不匹配即过期，不得错位应用。
3. 模拟预览和实际应用必须使用同一 ProseMirror 表格执行器；一批操作全成或全不成。
4. BDocx 表格结构建议必须在活动编辑器会话中生成与应用；原生 DOCX 预览和未打开 BDocx 均拒绝。
5. 保存沿用既有决策：已应用的 pending Diff 在保存时接受；DOCX 落盘仍只经 DOC 转换链。

## 4. 范围

纳入：插入表格、增删行列、首行表头、合并/拆分、受限单元格文本写入、文档流上移/下移，
`list_tables`、`propose_edit(operation=table_ops)`、结构化 DiffCard、过期校验和回归测试。

排除：浮动/绝对定位、自由拖放、列宽数值输入、均分列、行高、复杂 Word 表格样式、嵌套表格。

## 5. 验证计划

- 纯逻辑：表格投影、指纹、操作校验、合并/拆分与原子失败。
- 前端：工具栏可发出全部纳入的人工命令；结构 Diff 能预览、接受、拒绝并恢复。
- 后端：工具 Schema、过期/活动会话守卫、载荷持久化与投影。
- 链路：代理工具调用 → pending Diff → 编辑器草稿 → 保存；BDocx 导出后重新打开验证基础结构。

## 6. 实施状态（2026-07-28）

已实现：表格稳定标识和结构快照、动态 capabilities 与 moveDirections、前后端严格
`TableOperation` 校验、富内容单元格保护、表格内独立工具组、可见列边界拖拽、结构化 Diff 细节，
以及活动 BDocx 会话/过期锚/同一规划器的守卫。

列宽拖拽的运行态纠偏（2026-07-28）：正式手测发现 WebKit 未触发 prosemirror-tables 的隐式边缘
悬停命中，故改为当前选中表格上的显式 12px 边界 widget；widget 的 `mousedown` 只预设官方
`columnResizing` 的活动单元格，真实拖动计算和 `colwidth` 事务仍由官方插件执行。

二次运行态纠偏（2026-07-28）：显式 widget 可命中但官方插件在 WebKit 中未持续处理拖动事件。最小
运行态已验证句柄生成和逐行 `colwidth` 事务（首列 100 → 140）；实际 WKWebView 仍未持续派发
`pointermove`，故最终改为 `mousedown` 后在 `document` 捕获阶段监听 `mousemove` / `mouseup`，
且不读取 WebKit 已弃用的 `MouseEvent.which`。缩放位移按编辑器 zoom 归一化。需重启后再次人工验证
实际列边界随指针移动与保存后回读。2026-07-28 人类已在真实编辑状态确认列宽可左右拖动，拖拽交互
阻断项解除。

拖拽展示清理（2026-07-28）：隐藏官方插件与自定义热区的重复常驻蓝线，自定义热区仅在边界悬停时
显示轻量反馈；恢复由 TipTap TableView/colgroup 管理内联列宽所要求的 `table { width: 100% }`
基线，移除与显式列宽竞争的 `min-width: 100%`，避免蓝线和强制最小宽度干扰原单元格边框。

已自动验证：TypeScript 类型检查；表格投影、解析、富内容保护、操作规划测试；DiffCard 细节测试；
`list_tables`、工具注册与后端操作校验测试；Rust 编译检查；最小运行态模拟两列边界并将首列
`colwidth` 从 100 拖至 140；`governance:audit` 与 `git diff --check` 通过。

延期验证（不阻断课题 A）：清理重复蓝线后原表格边框的 Tauri/WKWebView 视觉效果、代理模型端到端
工具调用、BDocx 保存为 DOCX 后重开及 LibreOffice 视觉效果。课题 A 仅闭合已纳入的基础表格结构
编辑能力，不据此宣称复杂样式或 DOCX 往返效果已经验收。

全量前端测试的范围外发现：`src/services/edLocate.test.ts` 现有 4 项 `resolveSelectionAnchor` 断言与
已落地的嵌套 ContentBlock 定位实现不一致（偏移少 1）；本课题未修改该定位链路，故不在此修复。除该
4 项外，本课题相关前端/后端定向测试均通过。

治理留痕：`governance:audit` 为 passed，但 `edTableOps.ts` 的导出业务块仍出现在项目既有的
`BLOCK_UNANNOTATED` warning 集合中；本轮未新增专用表格规则 ID，后续应在表格链路规则稳定后补齐
@GOV 映射与 triage，而不是用现有 StyleOp 规则错误背书。

## 7. 本轮闭合检查（2026-07-28）

- final_state：CLOSED
- 人类决定：2026-07-28 明确宣布“课题 A 闭合”；未执行项按下述延期口径保留，不再阻断 A。
- 已解除：Tauri/WKWebView 列宽拖动无效；人类已确认可左右拖动。
- 自动验证：定向前端 3 文件 47/47；Rust `table_ops`、`list_tables`、工具注册定向测试通过；
  TypeScript、运行态列宽事务、治理审计、差异检查通过。
- 范围外失败：全量前端仍为 366/370，4 项均是既有 `edLocate.resolveSelectionAnchor` 偏移断言，
  与本课题表格改动无关。
- 延期项：清理后边框视觉复验、真实模型表格工具调用、DOCX 保存重开与 LibreOffice 视觉效果；后续
  独立验证，不把这些未验证项写成已通过。
