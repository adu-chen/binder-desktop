---
文档编号：CORE-X-L-03
文档状态：A
负责模块：CORE（跨模块前端实施）
文档职责：Binder 前端 G3 开发计划——跨模块承接型 Plan Item，涵盖 IPC 桥接打通、DesignToken 体系与布局骨架、欢迎页、侧边栏文件树（WS 前端）、TipTap 编辑器（ED 前端）、Chat 面板（AG 前端）、Diff 状态机前端（DE 前端）；对接 A-CORE-X-L-02 P2/P3 阶段
上游约束：A-CORE-X-L-02_总开发实施计划.md（P2/P3 验收）、A-CORE-C-P-01_APC.md（§5 G3/G4、§6 状态机驱动、§12 audit 硬阻断）、A-SYS-C-T-02_前端交互架构与流程总图.md（界面骨架与投影规则 SSOT）、A-SYS-M-S-01_状态机总图.md（前端态投影来源）、各模块 M-T/I-P（规则与 command 面）、docs/binder_ui/README.md（UI 视觉参考稿 SSOT）、docs/binder-vi/README.md（VI 视觉标识参考）
直接承接：本计划驱动前端各 Plan Item 实现；@GOV 追溯对应 BR-*/SM-* 规则；term_ref 追溯已注册 TERM
使用边界：L3 承接型计划，引用规则不定义规则；不得作为代码 @GOV 追溯来源（APC §3.2 铁律）；NFR 延 G5 标定；视觉精确值以参考稿为准，本文只描述实现要点
变更要求：Plan Item 增删保持与 A-CORE-X-L-02 阶段的承接可追溯；L2 契约变更回溯影响的 PI；新 TERM 注册须先在设计文档写 TERM 块再执行 governance:generate
治理成熟度：draft
---

# Binder 前端开发计划（跨模块 · L3）

## §1 文档定位

前端当前状态：`src/` 仅有欢迎页/工作台占位骨架与空 CSS；后端各模块已有实质实现（workspace
生命周期/IPC 协议/路径边界守护等），但 `lib.rs` invoke_handler 未注册任何 command，整条
前后端链路无法运行（2026-06-22 现状）。

本计划把 A-SYS-C-T-02 界面骨架切成可执行 Plan Item，驱动前端从零打通到"对话→Diff→裁决→落盘"
核心交互可用（对接 A-CORE-X-L-02 P2/P3）。

**UI 视觉参考源**：`docs/binder_ui/`（高保真原型，Design Token 精确值以此为准）；
**VI 品牌标识参考源**：`docs/binder-vi/`（Logo/BinderMark/配色方向 D-06）。
两者均为 L4 只读参考，不拷入生产代码；实现须在 Tauri 2 + React 技术栈内复现，不原样搬移原型。

## §2 Plan Item 清单

### PI-FE-00　IPC 桥接打通（前置，阻塞所有联调）　✅ G4 已实现（2026-06-22，cargo check + cargo test 84 passed + tsc 绿；dispatch 信封路由 + plugin-dialog 目录选择器）

- **承接**：BR-SYS-DATA-003（command 注册表）/ IPC-CALL chain / X-SEQ-002（前端经 command
  触达后端，不跨层直连）。
- **文件范围**：
  - 后端：`src-tauri/src/lib.rs`（`.invoke_handler` 解注释，注册 `ws.open_workspace`）
  - 前端：`src/services/workspaceService.ts`（接入 Tauri `dialog::open()` 目录选择，替换 `rootPath:""` 占位）
- **@GOV**：command 注册点追溯 `BR-SYS-DATA-003`；service 调用层追溯 `X-SEQ-002`。
- **A 类债务**：无（此为解除阻断的最小变更）。
- **验收**：点击欢迎页"打开/新建 workspace" → 弹出 macOS 目录选择器 → 选择目录后 `ws.open_workspace`
  返回 `WorkspaceRef` → 前端切换至 Workbench 界面；全链路可实际运行（非"应该能工作"）。

---

### PI-FE-01　DesignToken 体系与 Workbench 布局骨架　✅ G4 已实现（2026-06-22，tsc 绿；styles.css token 体系 + Workbench 四栏 Grid，DiffStrip 依计数显隐）

- **承接**：REQ-SYS-006 / A-SYS-C-T-02 §3 界面骨架 / TERM-SYS-001（DesignToken）/ X-FORBID-007。
- **文件范围**：
  - `src/styles.css`：CSS 自定义属性体系（DesignToken）——参考源 `docs/binder_ui/app-styles.css`
  - `src/shell/Workbench.tsx`：CSS Grid 四栏布局骨架替换现有占位 div
- **DesignToken 精确值**（参考稿 §8，直接照搬）：

  | Token 类别 | 关键值 |
  |---|---|
  | 颜色 | `--ink: #0E0C0A` / `--bg-page: #F4F1EA` / `--rust: #C24A2A` / `--forest: #2E6F4A` / `--gold: #B58A2E` / `--bg-elevated: #F8F5EC` / `--bg-sunken: #E6E2D8` / `--bg-card: #FFFFFF` |
  | 字体 | IBM Plex Sans 400/500/600/700（正文/界面）；IBM Plex Mono（标签/编号/代码）；Noto Sans SC（中文） |
  | 间距 | 4 / 8 / 12 / 16 / 24 / 32 / 48 px |
  | 圆角 | 0（主结构硬朗）/ 2 / 4 / 8 |
  | 描边 | 0.5px 常规 / 1px 强线 / 2px 锈红强调条 |

- **布局结构**（参考稿 §4）：
  ```
  grid-template-rows: 32px 1fr auto 24px
                      [TitleBar][Work][DiffStrip][StatusBar]

  Work 区 grid-template-columns: 44px 272px 1fr 372px
                                 [Rail][Side][Center][Chat]
  ```
  `DiffStrip` 无待裁决 PendingDiff（TERM-DE-001）时隐藏（`display:none`）。

- **@GOV**：布局骨架追溯 `REQ-SYS-006`（经 A-SYS-C-T-02 §3 承接）；Store 投影追溯 `X-FORBID-007`。
- **term_ref**：`TERM-SYS-001`（DesignToken 变量体系）；`TERM-DE-001`（DiffStrip 显隐条件）。
- **A 类债务**：深色主题切换（`app-styles.css` `.theme-dark` 已有规范，实现期叠加，不阻塞 G4）。
- **验收**：`governance:audit` 绿；CSS Grid 四栏骨架渲染正确；DesignToken 变量全部生效（无硬编码原始色值残留）。

---

### PI-FE-02　Logo 与 Left Rail　✅ G4 已实现（2026-06-23，tsc 绿 + scoped audit 无越界；BinderMark/Wordmark SVG + LeftRail 5 模块导航）

- **承接**：A-SYS-C-T-02 §3 Left Rail 界面分区 / 路线图 §9 UI 就位 / VI 参考 `docs/binder-vi/project/`。
- **文件范围**：
  - `src/components/brand/BinderMark.tsx`：段落条堆叠 Logo SVG（移植自 `binder-parts.jsx` `BinderMark`）
  - `src/components/brand/BinderWordmark.tsx`：文字标 SVG（移植自 `binder-parts.jsx` `BinderWordmark`）
  - `src/components/shell/LeftRail.tsx`：44px 图标导航栏，5 导航项（WS/AST/KB/TPL/时间轴，带罗马编号 I·A～I·E）
  - `src/shell/Workbench.tsx`：挂载 LeftRail
- **Logo 规格**（参考稿 §9，直接照搬）：
  - 8 条横条，宽度节奏 `100/64/82/100/72/100/56/100`（%），第 4 条（索引 3）用 `--rust` 锈红，其余 `--ink`；padding 14，barH 8
  - 紧凑形态（≤22px）：4 条 `100/56/100/72`，barH 16，保留锈红点
  - 文字标：`binder` + 锈红句点，IBM Plex Sans 500，字距 −0.04em
- **@GOV**：LeftRail 导航状态投影追溯 `X-FORBID-007`（当前激活面板为后端状态投影，不在前端独立维护）。
- **A 类债务**：LeftRail 徽标（badge）数量（如 PendingDiff 计数）在对应模块 PI 完成后叠加，本 PI 只建立导航切换骨架。
- **验收**：Logo 与参考稿像素级一致（含锈红强调条）；点击 5 导航项切换 Side Panel 内容区；`governance:audit` 绿。

---

### PI-FE-03　欢迎页完整实现（WS 前端 · 入口链路）　✅ G4 已实现（2026-06-23，tsc + cargo test 84 + build + scoped audit 绿；最近列表三态 + 启动恢复活动态）

- **承接**：A-SYS-C-T-02 §4.1 应用启动流程 / REQ-WS-001 / command `ws.open_workspace` + `ws.list_workspaces` / X-FORBID-007 / TERM-WS-001（Workspace）。
- **文件范围**：
  - `src/shell/Welcome.tsx`：完整欢迎页（目录选择 / 最近 Workspace 列表 / 登录入口）
  - `src/services/workspaceService.ts`：补充 `listWorkspaces()` service 方法
  - `src/store/workspaceShell.ts`：补充最近列表状态投影
- **实现要点**：
  - "打开/新建 workspace"：调用 Tauri `dialog::open()` 选择目录 → `ws.open_workspace` → 投影至 Store → 切换 Workbench（PI-FE-00 已打通此路径）
  - "最近的 workspace"：`ws.list_workspaces` 返回列表渲染，点击 → `ws.switch_workspace` → 投影
  - 登录入口：暂为跳转引导（ACCT 模块尚未完整实现），不阻塞欢迎页完成
  - 欢迎页↔工作台切换纯投影后端 `workspace.is_active`，前端不另立切换态（X-FORBID-007）
- **@GOV**：切换逻辑追溯 `X-FORBID-007`；WS command 调用追溯 `X-SEQ-002`。
- **term_ref**：`TERM-WS-001`（Workspace 状态是后端控制态，前端投影）。
- **A 类债务**：最近列表为空态 / 加载态 / 错误态 UI（实现期补充）。
- **验收**：欢迎页三入口全部可交互；最近列表真实从后端加载；无前端独立 workspace 状态（仅投影）；`governance:audit` 绿。

---

### PI-FE-04　侧边栏文件树（WS 模块前端）　✅ G4 已实现（2026-06-23，tsc + cargo test 86 + build + scoped audit 绿；ws.list_docs 枚举守界 + 文件树 6 状态接通 normal/not_supported）

- **承接**：A-SYS-C-T-02 §3 Side Panel WS / BR-WS-DATA-001（DocRef 定位）/ WS-FS chain / X-FORBID-004（不越界） / TERM-WS-001（Workspace）。
- **文件范围**：
  - `src/components/ws/FileTree.tsx`：文件树组件（递归目录结构）
  - `src/components/ws/FileTreeNode.tsx`：单节点组件，含 6 状态
  - `src/services/wsService.ts`：封装 `ws.list_docs` / `ws.search` / `ws.create_doc` 等
  - `src/store/wsStore.ts`：文件树状态投影（承 `WS-FS` command 回调）
- **FileTreeNode 6 状态**（参考稿 §6.4，视觉互斥）：
  | 状态 | 视觉 |
  |---|---|
  | `normal` | 默认 |
  | `open` | 锈红左条 |
  | `dirty_open` | 红点（未保存） |
  | `pending_diff` | 数字 badge（DiffCard 数量） |
  | `external_changed` | 金色 `!` |
  | `not_supported` | 灰色禁用（如 xlsx/pptx 不可编辑） |
- **@GOV**：文件树状态追溯 `BR-WS-DATA-001`；search 路径追溯 `X-FORBID-004`（路径守护由后端执行，前端只转发）；Store 投影追溯 `X-FORBID-007`。
- **term_ref**：`TERM-WS-001`（Workspace）。
- **A 类债务**：全文搜索面板挂载（A-SYS-C-T-02 §3 注：当前左侧用文件树过滤搜索，全文搜索面板延后）；文件树大目录虚拟滚动（NFR，G5 标定）。
- **验收**：文件树从 `ws.list_docs` 真实加载；6 状态视觉互斥可辨；`external_changed` 状态由后端事件驱动触发；`governance:audit` 绿。

---

### PI-FE-05　TipTap 编辑器接入（ED 模块前端）　✅ G4 已实现（2026-06-23，tsc + build 绿；TipTap 3 + BlockIdExtension 稳定 blockId + EditorTab 6 态 + 扩展槽预留）

- **承接**：ED 模块 L2（A-ED-M-T-01）/ SM-ED-01（编辑会话状态机）/ BR-ED-DATA-001（blockId 稳定性）/ BR-ED-STATE-001（脏态管理）/ X-FORBID-005（装饰层不替换 DOM）/ TERM-ED-001（Block）/ TERM-ED-002（EditorDecoration）/ TERM-ED-003（GreenReview，扩展槽预留）。
- **文件范围**：
  - `src/components/ed/Editor.tsx`：TipTap 3 编辑器宿主组件
  - `src/components/ed/extensions/BlockIdExtension.ts`：blockId 稳定标记扩展
  - `src/components/ed/extensions/index.ts`：扩展注册入口（预留 `DiffDecorationExtension` / `CopyReferenceExtension` / `PageTopCaretExtension` / `FontSize` 槽位）
  - `src/components/ed/EditorTab.tsx`：标签栏组件，含 6 状态
  - `src/store/edStore.ts`：编辑会话状态投影（承 SM-ED-01）
- **EditorTab 6 状态**（参考稿 §6.3）：
  `normal` / `dirty（红点）` / `saving（旋转器）` / `saved（回退默认，不常驻对勾）` / `pending_diff（红色数字 badge）` / `external_changed（金色 !）`
- **扩展架构约束**：`DiffDecorationExtension` 在 PI-FE-07 实现，此处在 `extensions/index.ts` 预留注册槽位，避免后续破坏扩展初始化顺序；GreenReview（TERM-ED-003）为该扩展的 Decoration 实例。
- **@GOV**：blockId 分配追溯 `BR-ED-DATA-001`；脏态管理追溯 `BR-ED-STATE-001`；装饰层契约追溯 `X-FORBID-005`；Store 追溯 `X-FORBID-007`。
- **term_ref**：`TERM-ED-001`（Block）；`TERM-ED-002`（EditorDecoration）；`TERM-ED-003`（GreenReview，预留槽注释引用）。
- **A 类债务**：TipTap/ProseMirror 具体版本锁定（ED L2 §7 A 类债务，实现期回填）；blockId 算法选型（nanoid/ulid，实现期记录）；DOCX 往返（归 DOC 模块，本 PI 只做 md 基础编辑）。
- **验收**：可在编辑区打开并编辑 md 文件；blockId 在普通编辑后不变（删除后失效不复用）；EditorTab 6 状态随编辑/保存操作真实切换；`governance:audit` 绿。

---

### PI-FE-06　Chat 面板骨架（AG 模块前端）　✅ G4 已实现（2026-06-23，tsc + cargo test + build + scoped audit 绿；ag.start_turn 桩走门禁前置 + 流式回显模拟 + ToolCall 6 态）

- **承接**：AG 模块 L2（A-AG-M-T-01）/ SM-AG-01（代理回合状态机）/ command `ag.start_turn` + `turn_stream` / X-SEQ-001（AI 调用前订阅门禁）/ X-FORBID-007 / TERM-AG-001（AgentTurn）/ TERM-AG-002（ToolCall）/ TERM-AG-003（InlineAssist）/ TERM-AG-004（Autocomplete）。
- **文件范围**：
  - `src/components/ag/ChatPanel.tsx`：Chat 面板（chat/agent 模式切换 + 会话标签）
  - `src/components/ag/MessageList.tsx`：消息流（含流式回显）
  - `src/components/ag/ToolCallCard.tsx`：ToolCall（TERM-AG-002）6 状态卡
  - `src/components/ag/Composer.tsx`：输入框 + 发送
  - `src/store/agStore.ts`：AgentTurn（TERM-AG-001）状态投影（承 SM-AG-01）
- **ToolCallCard 6 状态**（参考稿 §6.2）：
  `pending（排队）` → `executing（旋转器+进度条）` → `done（OK+耗时）`；分支：`awaiting_confirmation（危险授权门，锈红边，需用户授权）` / `failed（可重试/跳过）` / `skipped`。
- **三种 AI 辅助入口行为边界**（实现期不得混淆）：
  - `⌘J Autocomplete`（TERM-AG-004）：光标浮层，3 条续写建议，Tab/↩ 选择，**不产出 DiffCard，不进入 Diff 链路**
  - `⌘K InlineAssist`（TERM-AG-003）：选区上方对话弹窗，Apply 后**直接替换选区，不进入 Diff 链路**
  - Chat Diff：右栏 DiffCard（TERM-DE-003），走完整 DE 8 状态链路
  - 本 PI 建立 Chat 面板骨架与流式回显；⌘J/⌘K 的 UI 组件在 PI-FE-08 实现
- **订阅门禁打桩**：AI 调用前须经 `SubscriptionGate`（TERM-CLOUD-001）校验（X-SEQ-001）；云端 ENT 未完整实现期间，按 A-SYS-I-P-03 契约打桩（本地 mock 放行 + 明确注释桩位置）。
- **@GOV**：turn 状态投影追溯 `SM-AG-01`；流式回显追溯 `BR-SYS-STATE-002`（StreamEvent 协议）；门禁桩追溯 `X-SEQ-001`。
- **term_ref**：`TERM-AG-001`（AgentTurn）；`TERM-AG-002`（ToolCall）；`TERM-CLOUD-001`（SubscriptionGate 桩位）。
- **A 类债务**：`awaiting_confirmation` 危险授权门的完整授权 UI（参考稿 §6.2 / 链路 H，G5 联调期补全）；⌘K/⌘J UI 归 PI-FE-08。
- **验收**：发送消息 → 流式字符回显（turn_stream 事件）；ToolCallCard 6 状态视觉互斥；`awaiting_confirmation` 授权门可见（即使后端尚未实际触发）；`governance:audit` 绿。

---

### PI-FE-07　Diff 状态机前端——DiffCard + GreenReview + 批量操作条（DE 模块前端）　✅ G4 已实现（2026-06-23，tsc + build + scoped audit 绿；DiffCard 8 态 + expired 5 原因 + GreenReview 装饰层填扩展槽 + 批量条倒序 + 双向同步）

- **承接**：DE 模块 L2（A-DE-M-T-01）/ SM-DE-01（diff 终态状态机）/ DE-DIFF chain / command `de.accept_diff` + `de.reject_diff` + `de.generate_diff` / X-INTER-001（DiffCard↔GreenReview 双向同步须在状态机总图登记跨模块转移） / TERM-DE-001（PendingDiff）/ TERM-DE-003（DiffCard）/ TERM-ED-002（EditorDecoration）/ TERM-ED-003（GreenReview）。
- **文件范围**：
  - `src/components/de/DiffCard.tsx`：DiffCard（TERM-DE-003），8 状态互斥可辨
  - `src/components/ed/extensions/DiffDecorationExtension.ts`：GreenReview（TERM-ED-003）ProseMirror Decoration 实现，填充 PI-FE-05 预留槽
  - `src/components/de/DiffAllActionsBar.tsx`：批量操作条，3 状态
  - `src/store/deStore.ts`：PendingDiff（TERM-DE-001）状态投影（承 SM-DE-01）
- **DiffCard 8 状态**（参考稿 §6.1 / binder-showcase.jsx §02，精确复现）：

  | 状态 | 视觉 | 可用操作 |
  |---|---|---|
  | `pending` | 锈红实边 + PENDING badge | 接受 ⏎ / 拒绝 ⌫ |
  | `validating` | 金色 + 旋转器 | 按钮禁用 |
  | `applying` | 金色 + 进度条 | 按钮禁用 |
  | `accepted` | 森林绿 + ACCEPTED badge + ✓ | 终态 |
  | `rejected` | 灰弱化 + 降透明度 | 终态 |
  | `expired` | 灰边 + `!` + 原因文案 | 重新生成 / 关闭 |
  | `conflict` | 金色 + ⌖ + 冲突 diff id | 接受禁用，可单独拒绝 |
  | `error` | 锈红边 + × + 错误码 | 重试 ↻ / 拒绝 |

- **expired 5 种细分原因**（每种须有对应引导文案，不暴露技术错误码）：
  `document_revision_advanced` / `block_resolve_failed` / `overlapping_range` / `external_file_changed` / `apply_replace_failed`

- **GreenReview 约束**：
  - 基于 ProseMirror Decoration 叠加，不替换 DOM、不阻挡编辑（X-FORBID-005）
  - 4 状态（pending 锈红左条 / applying 金 / accepted 绿 / expired 淡出）
  - 与 DiffCard 一一对应，任意一侧 Accept/Reject 另一侧同步（X-INTER-001）

- **DiffAllActionsBar 3 状态**：
  `有待裁决（红色计数+全部接受/拒绝）` → `倒序应用中（金色进度，从文档靠后的 PendingDiff 先应用以避免行号偏移）` → `完成（绿，含失效统计）`

- **@GOV**：DiffCard 状态机追溯 `SM-DE-01`；GreenReview 装饰层追溯 `X-FORBID-005`；双向同步追溯 `X-INTER-001`；落盘追溯 `BR-WS-STATE-001`（经 `ws.apply_diff`）。
- **term_ref**：`TERM-DE-001`（PendingDiff）；`TERM-DE-003`（DiffCard）；`TERM-ED-003`（GreenReview）；`TERM-ED-002`（EditorDecoration）。
- **A 类债务**：expired 原因 `apply_replace_failed` 的自动重试策略（G5 联调确认次数上限）；DiffCard 与 GreenReview 位置同步（编辑器滚动时 DiffCard 对齐，实现期决策）。
- **验收（强制）**：SM-DE-01 8 状态全覆盖；expired 5 种细分原因各有文案；GreenReview↔DiffCard 双向裁决同步；倒序批量接受执行顺序正确（文档靠后先应用）；`governance:audit` 绿。

---

### PI-FE-08　三种 AI 辅助入口 UI（⌘K InlineAssist / ⌘J Autocomplete）　✅ G4 已实现（2026-06-23，tsc + build + scoped audit 绿；⌘K 替换选区 / ⌘J 续写插入，均不产 DiffCard 不进 Diff 链路）

- **承接**：AG 模块 L2 / SM-AG-02（InlineAssist）/ SM-AG-03（Autocomplete）/ TERM-AG-003（InlineAssist）/ TERM-AG-004（Autocomplete）/ X-FORBID-007。
- **文件范围**：
  - `src/components/ag/InlineAssistPopover.tsx`：选区上方对话弹窗（⌘K）
  - `src/components/ag/AutocompleteOverlay.tsx`：光标处浮层（⌘J），3 条续写建议
- **行为边界约束**（不得混淆，见 A-SYS-C-T-02 §6 三种 AI 辅助入口）：
  - **⌘K**：选区→输入→生成→Apply；Apply 后**直接替换选区文本**，不生成 DiffCard，不进入 SM-DE-01
  - **⌘J**：光标处低权重浮层，Tab/↩ 选择续写，**不产出 DiffCard，不进入 Diff 链路**
  - 两者均不经过 Chat 面板，不更新 AgentTurn（TERM-AG-001）会话消息列表
- **@GOV**：⌘K 替换操作追溯 `SM-AG-02`；⌘J 候选提供追溯 `SM-AG-03`；不产出 Diff 追溯 `SM-DE-01`（豁免入口声明）。
- **term_ref**：`TERM-AG-003`（InlineAssist）；`TERM-AG-004`（Autocomplete）。
- **A 类债务**：⌘K 生成中的流式预览（实现期选型）；⌘J 防抖/触发时机（实现期）。
- **验收**：⌘K 弹窗在选区时可用，Apply 后选区内容替换（不出现在 Chat 消息流，不生成 DiffCard）；⌘J 浮层在光标处可用，Tab 接受（不出现在 Chat 消息流）；`governance:audit` 绿。

---

### PI-FE-09　前后端落盘集成（打开/保存真实文档）　✅ G4 已实现（2026-06-23，tsc + build + scoped audit 绿；文件树点击 → ws.read_doc 载入、⌘S → ws.save_doc 存盘 + 冲突裁决）
- **承接**：A-WS-I-P-01（ws.read_doc/save_doc）/ WS-FS / WS-SAVE / SM-WS-01 / BR-WS-OBS-001 / BR-WS-PERSIST-001 / BR-ED-STATE-001 / X-SEQ-002 / X-FORBID-007 / TERM-WS-001。承 PI-BE-01 落盘 command。
- **文件范围**：`wsService`（readDoc/saveDoc）、`edService`（openDocument/saveDocument/reloadDocument 编排）、`edStore`（loadDoc/markSaved + 基线）、`FileTree`（点击 → openDocument）、`Editor`（载入真实内容 + ⌘S + 冲突 banner）。
- **实现要点**：纯文本忠实往返（行↔\n）；保存带加载基线，磁盘≠基线且未 override → WS_SAVE_CONFLICT（SM-WS-01）→ 编辑器冲突条「覆盖 / 重载」。
- **A 类债务**：富文本 md 解析/序列化（tiptap-markdown 或 DOC pandoc 路）；外部变更主动监视（ws.external_change 事件订阅）；deStore.accept → ws.apply_diff 真落盘（需真实 diff payload）。
- **验收**：点击文件载入内容；⌘S 刷新基线；冲突路径有裁决入口；`governance:audit` scoped 无越界。

---

### PI-FE-10　workspace 切换 / 返回欢迎页　✅ G4 已实现（2026-06-23，tsc + cargo test + build + scoped audit 绿；标题栏 workspace 菜单 + ws.close_workspace 接线 + 切换重置下游 store + 脏态守卫）
- **承接**：A-SYS-C-T-02 §4.1（WB→关闭/切换→C）/ A-WS-I-P-01（ws.switch_workspace/close_workspace）/ BR-WS-PERSIST-002 / X-FORBID-007 / X-SEQ-002 / TERM-WS-001。补实现既有设计，不改 SSOT。
- **文件范围**：`lib.rs`（注册+路由 ws.close_workspace）、`workspaceService`（closeWorkspace + 切换/打开重置下游 store）、`wsStore/edStore/deStore`（reset）、`WorkspaceMenu`（标题栏菜单 + 脏态守卫）、`Workbench`（挂菜单 + 按 workspaceId 重挂面板）。
- **实现要点**：返回欢迎页 = ws.close_workspace（置非活动可恢复）+ 投影 null；切换/打开后重置 wsStore/edStore/deStore 并按 workspaceId 重挂 FileTree/Editor/ChatPanel，杜绝旧项目内容残留/按旧基线误存；破坏性操作前脏态守卫（编辑器 dirty 或 pending diff → 确认，参考稿链路 G）。
- **A 类债务**：启动恢复 root 存在性探测（死项目回退欢迎页）延后；脏态守卫为通用确认，未细分「先处理/放弃」逐条处置。
- **验收**：从 Workbench 可切换/返回欢迎页；切换后文件树与编辑器为新项目内容；脏态守卫生效；`governance:audit` scoped 无越界。

---

### PI-FE-11　diff 真落盘前端接线（accept→ws.apply_diff）　✅ G4 已实现（2026-06-23，tsc + build + scoped audit 绿；示例修改→de.generate_diff、接受→de.accept_diff 真写盘+编辑器重载、拒绝→de.reject_diff）
- **承接**：A-DE-I-P-01 §2/§3（de.generate_diff/accept_diff/reject_diff）/ SM-DE-01 / X-INTER-001 / X-FORBID-007 / TERM-DE-001/003。承 PI-BE-02。
- **文件范围**：`deService`（generate/accept/reject/acceptAll 编排）、`deStore`（去 demo，纯状态投影 addDiff/setDiffState）、`DiffCard`/`DiffAllActionsBar`（接 deService）、`Composer`（"示例修改"入口绕过桩模型）。
- **实现要点**：示例修改对当前打开文档做确定性变换 → de.generate_diff 持久化 DiffPayload；接受 validating→applying→**de.accept_diff（ws.apply_diff 真写盘+时间轴节点）**→accepted→编辑器重载；拒绝 de.reject_diff 不写文件；全部接受倒序。8 视觉态仍投影后端 SM-DE-01，不另立平行机。
- **A 类债务**：整文档变换未绑块级 blockId → 该类 diff 无 GreenReview 高亮（块级 diff 时恢复）；de.list_diffs 重启恢复未在前端自动加载（无 before/after 预览）；conflict 态由后端重叠重检驱动（PI-DE-03 未接）。
- **验收**：打开文档→示例修改→接受→文件真被写入+编辑器显示新内容+时间轴号；拒绝不改文件；`governance:audit` scoped 无越界。

---

### PI-BE-03+FE-12　外部变更监视（补全 SM-WS-01）　✅ G4 已实现（2026-06-23，cargo test 98 + tsc + build + scoped audit 绿；ws.check_external_change + 编辑器 5s 轮询触发冲突条）
- **承接**：A-WS-I-P-01 §6（external_change 逻辑契约）/ SM-WS-01（externalChange）/ BR-WS-OBS-001 / X-FORBID-004 / X-FORBID-007。
- **文件范围**：`save.rs`（check_external_change，covers BR-WS-OBS-001 三态）、`lib.rs`（ws.check_external_change 路由）、`wsService`（checkExternalChange）、`edService`（pollExternalChange）、`Editor`（5s 轮询）。
- **实现选择**：external_change 逻辑契约本阶段以前端轮询 `ws.check_external_change` 实现（物理事件 Channel/event 延 SY-02），已补记 A-WS-I-P-01 §3/§6。磁盘≠加载基线/被删 → external_changed → 触发 PI-FE-09 已有冲突条（覆盖/重载）。
- **A 类债务**：物理事件推送转 ED/DE/REF 延 SY-02；DE 过期重检（PI-DE-03）与 REF 锚校验待事件传输接通；轮询固定 5s 未做防抖/可配。
- **验收**：外部改动/删除文件后 5s 内编辑器出现冲突条；不静默覆盖；`governance:audit` scoped 无越界。

---

### PI-FE-15　健壮性收口（死项目回退 + 文件树刷新）　✅ G4 已实现（2026-06-23，tsc + build + scoped audit 绿；启动探测死项目→回退欢迎页+提示、文件树常驻刷新）
- **承接**：A-SYS-C-T-02 §4.1 / WS-FS / X-FORBID-007 / X-SEQ-002。补 PI-FE-10 标注的启动恢复 root 探测债务。
- **文件范围**：`workspaceService`（restoreActiveWorkspace 探测 + notice）、`wsService`（fetchDocs 抽取）、`workspaceShell`（notice）、`Welcome`（提示）、`FileTree`（刷新按钮 + 仅 idle 加载）。
- **实现要点**：恢复前以 ws.list_docs 探 root 可达——死项目（读目录失败）→ ws.close_workspace + 回退欢迎页 + 金色提示；可达则预置文件树（避免二次枚举）。文件树标题区常驻 ↻ 刷新（外部新增文件后无需重开）。
- **A 类债务**：空目录可见（需后端 listing 返回目录节点）、全文搜索（ws.search 未实现）留作后续 PI。
- **验收**：历史项目被删后启动回退欢迎页+提示；刷新按钮可重载；`governance:audit` scoped 无越界。

---

### PI-BE-04+FE-13　文件操作（新建/重命名/删除 + 右键菜单）　✅ G4 已实现（2026-06-23，cargo test 101 + tsc + build + scoped audit 绿；ws.create/rename/delete_doc + 文件树右键菜单）
- **承接**：A-WS-I-P-01 §3/§8（move 签名补）/ BR-WS-DATA-001 / REQ-WS-002 / WS-FS / X-FORBID-004 / X-SEQ-002。
- **文件范围**：`fileops.rs`（create/rename/delete 守界，3 covers 测试，@GOV BR-WS-DATA-001 + term_ref TERM-WS-001）、`lib.rs`（3 路由）、`wsService`（createDoc/renameDoc/deleteDoc）、`FileTree`/`FileTreeNode`（右键菜单 + 内联输入 + 编排）。
- **实现要点**：新建（"＋"内联输入路径→ws.create_doc→刷新+打开）、重命名（右键→内联输入→ws.rename_doc，影响打开文档则重开）、删除（右键→确认→ws.delete_doc，删打开文档则清空编辑器）；已存在/越界/不存在均拒。补协议 §8 move 待补项。
- **A 类债务**：空目录可见（需 listing 返回目录节点）、全文搜索（ws.search）、copy 操作仍延后。
- **验收**：新建/重命名/删除文件并刷新树；冲突拒绝有提示；`governance:audit` scoped 无越界。

---

### PI-FE-14　富文本 Markdown 往返（tiptap-markdown）　✅ G4 已实现（2026-06-23，tsc + build + vitest 绿；md 解析为富文本渲染 + getMarkdown 序列化回写）
- **承接**：A-ED-M-T-01（md/docx 统一编辑模型）/ ED-EDIT / X-FORBID-007。替换 PI-FE-09 纯文本往返。
- **文件范围**：`package.json`（tiptap-markdown，本地库离线可用）、`extensions/index.ts`（Markdown 扩展）、`Editor.tsx`（md 文件 setContent 解析 + getMarkdown 保存；非 md 维持纯文本）。
- **实现要点**：.md/.markdown 打开渲染为富文本（# 标题、**粗体** 等）、保存序列化回合法 md；.txt 等维持纯文本往返；blockId 为运行时属性不持久化到 md（往返后重生成）。
- **A 类债务**：md 保真度限 tiptap-markdown 支持的语法（表格/脚注等扩展按需）；docx 往返仍归 DOC/pandoc 路。
- **验收**：md 富文本渲染 + 合法 md 写回；txt 纯文本往返；`governance:audit` scoped 无越界。

---

### PI-FE-16　时间轴面板（WS 前端）　✅ G4 已实现（2026-06-23，tsc + build + vitest + scoped audit 绿；LeftRail 时间轴 → 节点列表 + 版本预览 + 还原）
- **承接**：A-SYS-C-T-02 §3（时间轴归 WS）/ A-WS-I-P-01 §5 / SM-WS-03 / BR-WS-DATA-002/STATE-002 / X-FORBID-007 / TERM-WS-002。承 PI-BE-05。
- **文件范围**：`wsService`（listTimeline/previewNode/restoreNode）、`TimelinePanel`（节点列表 + 预览 + 还原）、`Workbench`（activePanel=timeline 挂载）。
- **实现要点**：随打开文档加载时间轴节点（最近在前）；点节点 ws.preview_node 预览历史内容；「还原此版本」→ ws.restore_node 写回 + 前进式记新节点 → 重载编辑器 + 刷新列表。
- **A 类债务**：预览为整文档纯文本（无逐版本 diff 对比高亮）；非当前打开文档的时间轴未提供入口。
- **验收**：接受 diff 后时间轴见节点；预览历史；还原写回并编辑器更新；`governance:audit` scoped 无越界。

---

### PI-TEST-01　前端测试（单元 + 契约层，已转真主干）　✅ G4 已实现（2026-06-23，vitest 37 passed + tsc + build + scoped audit 绿）
- **承接**：APC §10（单元/集成测试策略）/ A-CORE-X-L-03。仅测已转真主干（WS/ED/DE 落盘/时间轴/workspace 生命周期），排除仍是桩的 AG 与 e2e（G5）。
- **文件范围**：`deStore/wsStore/workspaceShell.test.ts`（状态投影 reducer）、`buildTree.test.ts`（扁平→树，FileTree 导出 buildTree）、`wsService/edService.test.ts`（mock invoke 契约：CRUD/信封/保存冲突/外部变更）。
- **实现要点**：node 环境（不渲染组件，无需 jsdom，无新依赖）；vi.mock @tauri-apps/api/core 验信封与 result/error 互斥；deStore 三终态互斥不可逆 covers BR-DE-STATE-001。
- **A 类债务**：组件渲染测试（FileTreeNode 6 态/DiffCard 8 态/EditorTab/TimelinePanel）需后续装 jsdom + @testing-library；AG 桩面与 e2e 待模型/云接通（G5）。
- **验收**：vitest 37 全过；tsc/build 绿；`governance:audit` scoped 无越界。

---

### PI-TEST-02　组件状态机渲染测试（jsdom + @testing-library）　✅ G4 已实现（2026-06-23，vitest 56 passed + tsc + build + scoped audit 绿）
- **承接**：APC §10 / A-CORE-X-L-03。测互斥状态可视化组件：FileTreeNode 6 态 / DiffCard 8 态 / EditorTab 6 态。排除 AG 桩组件。
- **文件范围**：装 jsdom + @testing-library/react + @testing-library/dom；`FileTreeNode/DiffCard/EditorTab.test.tsx`（`@vitest-environment jsdom` docblock，仅组件测试用 jsdom，单元/契约仍 node）。
- **实现要点**：断言渲染产物（class/标记/按钮可用性）验证状态互斥；不触发点击调后端（DiffCard mock invoke 隔离）。
- **A 类债务**：容器组件（FileTree/TimelinePanel/ChatPanel）的交互流程测试与 AG 桩面、e2e 仍留后续（G5）。
- **验收**：vitest 56 全过；tsc/build 绿；`governance:audit` scoped 无越界。

---

### PI-BE-06+FE-17　workspace 全文搜索（ws.search）　✅ G4 已实现（2026-06-23，cargo test 103 + tsc + vitest + build + scoped audit 绿）
- **承接**：A-WS-I-P-01 §3（ws.search）/ BR-WS-DATA-001 / WS-FS / X-FORBID-004 / X-SEQ-002 / TERM-WS-001。收口 PI-FE-15 搜索债务。
- **文件范围**：`listing.rs`（search 守界行级匹配 + covers 测试，@GOV BR-WS-DATA-001 + term_ref TERM-WS-001）、`lib.rs`（ws.search 路由）、`wsService`（searchDocs）、`FileTree`（搜索框 + 结果列表，防抖 250ms）。
- **实现要点**：递归 root 内行级大小写不敏感子串匹配，跳隐藏/非 UTF-8/>2MB，命中封顶 200；空查询回空；前端有查询时结果列表替代树，点结果打开文件。
- **A 类债务**：点击结果未跳转到具体行（需编辑器行定位）；正则/大小写敏感选项未做；**文件树空目录可见**仍延后（需 listing 契约改动、价值偏低）。
- **验收**：搜索出命中、点击打开；跳隐藏/守界；`governance:audit` scoped 无越界。

---

### PI-FE-18　清理展示桩（编辑器空态 + 移除 DEMO_TOOLS）　✅ G4 已实现（2026-06-23，tsc + vitest 56 + build + scoped audit 绿）
- **承接**：A-CORE-X-L-03 / CLAUDE.md §2（不以占位冒充正常态）。清掉 AG 桩期遗留展示物。
- **文件范围**：`Editor.tsx`（未打开文档 → 空态提示，去假 untitled.md）、`agStore.ts`（移除 DEMO_TOOLS，toolCalls 初始空）、`MessageList.tsx`（工具卡仅在有真实 ToolCall 时渲染，去"占位演示"）。
- **实现要点**：界面回到诚实空态——编辑器空态"从文件树打开一个文档开始编辑"，对话区不再有 6 个演示工具框；ToolCallCard 组件保留，待真实工具调用（云端接通）由 SM-AG-01 驱动。
- **验收**：未打开文档时编辑器空态、对话区无演示框；打开文件后编辑正常；vitest 56 全过；scoped 门禁无越界。

---

### PI-FE-19　状态投影接线（文件树/编辑器标签反映真实态）　✅ G4 已实现（2026-06-23，tsc + vitest 56 + build + scoped audit 绿）
- **承接**：A-SYS-C-T-02（前端态投影后端）/ 参考稿 §6.3/6.4 / X-FORBID-007 / BR-ED-STATE-001 / BR-DE-STATE-001。
- **文件范围**：`deStore`（DiffView 补 relPath）、`deService`（generateSampleDiff 填 relPath）、`FileTree`（投影打开文档节点态）、`Editor`（EditorTab 有效态）。
- **实现要点**：把已建的 6 态文件节点 / 6 态编辑标签接到**真实 edStore/deStore 投影**（此前仅测试 override）；优先级 external_changed > pending_diff（按文件归属计数）> dirty_open > open > normal，各态互斥。
- **验收**：编辑脏态→文件树红点+标签红点；生成 diff→该文件 pending badge；外部变更→金!；vitest 56 全过；scoped 门禁无越界。

---

### PI-FE-20　搜索结果跳行（打开并定位到命中行）　✅ G4 已实现（2026-06-23，tsc + vitest 56 + build + scoped audit 绿）
- **承接**：PI-BE-06+FE-17（全文搜索）/ ED-EDIT / X-FORBID-007。收口"点搜索结果未跳行"债务。
- **文件范围**：`edStore`（loadDoc 携带 jumpLine + clearJump）、`edService`（openDocument 增 jumpLine）、`Editor`（jumpToLine：行→块索引定位光标+滚动）、`FileTree`（搜索命中 → openDocument(path, line)）。
- **实现要点**：jumpLine 随 loadDoc 原子传递，setContent 后定位到目标行对应块并 scrollIntoView，跳后清除；纯文本精确（行=段落），md 近似（块索引≈行号，记债务）。
- **验收**：点搜索命中打开文件并光标落该行附近+滚动到视区；普通打开行为不变；vitest 56 全过；scoped 门禁无越界。

---

### PI-BE-07+FE-21　文件树空目录可见　✅ G4 已实现（2026-06-23，cargo test 111 + tsc + vitest 58 + build + scoped audit 绿）
- **承接**：A-WS-I-P-01 §3 / BR-WS-DATA-001 / WS-FS / X-FORBID-004 / X-FORBID-007 / TERM-WS-001。收口空目录可见债务。
- **文件范围**：`listing.rs`（FsEntry 含 is_dir，list_docs 返回文件+目录含空目录，covers 测试）、`lib.rs`（route 签名简化）、`wsService`/`wsStore`（FsEntry 投影，DocRefView 去 workspaceId 加 isDir）、`FileTree.buildTree`（isDir 建空目录 + 去重）、测试更新。
- **实现要点**：list_docs 不再仅返回文件——递归每个非隐藏目录与文件各一条；空目录 is_dir=true 在文件树可见；不动 SYS 共享 DocRef 语义（FsEntry 为 WS 本地结构）。
- **验收**：含空子目录的 workspace 文件树显示该目录；隐藏项跳过；vitest 58 全过；scoped 门禁无越界。

---

### PI-BE-08+FE-22　PRV 预览（prv.preview + 只读预览面板）　✅ G4 已实现（2026-06-23，cargo test 112 + tsc + vitest 58 + build + scoped audit 绿）
- **承接**：A-PRV-I-P-01 §2 / A-PRV-M-T-01（BR-PRV-DATA-002 管线归属、只读硬边界）/ PRV-VIEW / DOC libreoffice_runtime / X-FORBID-004 / X-FORBID-007。接已就绪的 DOC 预览链。
- **文件范围**：`prv/mod.rs`（render_preview：扩展名→PreviewKind→convert_to_pdf→PDF 字节，@GOV BR-PRV-DATA-002）、`lib.rs`（prv.preview 路由，字节回传）、`prvService`/`prvStore`/`PrvPanel`（字节→blob iframe 只读）、`FileTreeNode`（预览类型可点）、`FileTree`（onPreviewFile）、`Workbench`（center 预览/编辑切换）。
- **实现要点**：点 docx/Excel/PPT → DOC 预览链出 PDF（LibreOffice），字节回传组 blob URL 以 iframe 只读呈现；不支持格式/LibreOffice 缺失/渲染失败各有明确态；只读无编辑（PRV 硬边界）；守界 X-FORBID-004。
- **A 类债务**：大 PDF 字节走 JSON 数组（base64/流式优化延后）；音视频预览/导出/打印延后；切换 workspace 未自动关预览。
- **验收**：点预览类型文件出 PDF（或无 LibreOffice 时明确失败态）；只读；vitest 58/cargo 112 全过；scoped 门禁无越界。

---

### PI-BE-09+FE-23　PRV 导出 + 打印　✅ G4 已实现（2026-06-23，cargo build + tsc + vitest 58 + build + scoped audit 绿）
- **承接**：A-PRV-I-P-01 §2（prv.export/print）/ BR-PRV-DATA-002 / PRV-VIEW / DOC pandoc/libreoffice / X-FORBID-004。
- **文件范围**：`prv/mod.rs`（export：md/txt→docx、docx→md（pandoc）、office→pdf（libreoffice），@GOV BR-PRV-DATA-002）、`lib.rs`（prv.export 路由，落点守界）、`prvService`（exportDoc）、`PrvPanel`（打印当前预览 iframe.print）、`FileTree`（右键按源类型导出菜单）。
- **实现要点**：导出落点=同目录同名换扩展（workspace 内守界）；前端右键 md→DOCX / docx→PDF·MD / xls·ppt→PDF，导出后刷新树+提示；打印用 iframe 系统打印对话框（前端 prv.print）。
- **A 类债务**：音视频预览另起（需 asset 协议）；md→pdf（需 latex）未做；导出覆盖同名不二次确认；编辑器内打印未做。
- **验收**：导出生成目标文件并刷新树（有 pandoc/libreoffice 时）；预览打印唤起；scoped 门禁无越界。

---

### PI-BE-10+FE-24　音视频预览（asset 协议流式）　✅ G4 已实现（2026-06-23，cargo build + tsc + vitest 58 + build + scoped audit 绿）
- **承接**：A-PRV-I-P-01 §2（音视频→系统解码）/ A-PRV-M-T-01（只读硬边界）/ PRV-VIEW / X-FORBID-004。媒体走 asset 协议流式（非字节回传）。
- **文件范围**：`Cargo.toml`（tauri protocol-asset feature）、`tauri.conf.json`（开 assetProtocol，scope 空运行时授权）、`lib.rs`（dispatch 增 AppHandle + prv.media_src 守界 + allow_file 仅授权该文件）、`prvService.mediaSrc`（convertFileSrc）、`PrvPanel`（按扩展名分流 PDF/video/audio）、`FileTreeNode`（音视频可点预览）。
- **实现要点**：点 mp4/mp3 等 → prv.media_src 守界解析 + 把**该单个文件**加入 asset scope（最小暴露）→ convertFileSrc → `<video>/<audio>` 流式系统解码（不进内存，与 PDF 字节回传分流）；只读无编辑。
- **A 类债务**：媒体格式以浏览器/系统解码能力为准（部分编码不支持）；切 workspace 未清 asset 授权（累积，重启清）。
- **验收**：点音视频 → 播放器流式播放；PDF 预览不受影响；scoped 门禁无越界。

---

### PI-FE-25　图片预览（asset 协议）　✅ G4 已实现（2026-06-23，tsc + vitest 58 + build + scoped audit 绿）
- **承接**：A-PRV-I-P-01 §2 / PRV-VIEW / PI-BE-10（prv.media_src 格式无关）。修复"图片无法预览"。
- **文件范围**：`PrvPanel`（image 模式 <img>，prv.media_src→convertFileSrc）、`FileTreeNode`（图片类型并入可点预览）、`styles.css`。
- **实现要点**：图片复用已就绪的 asset 协议路径（后端无改动）；png/jpg/gif/webp/svg/bmp/avif 点击 → asset URL → <img> 只读呈现。
- **验收**：点图片显示图片；PDF/音视频不受影响；scoped 门禁无越界。

---

### PI-FE-26（PI-A）　center 标签栏 + 预览标签共存　✅ G4 已实现（2026-06-23，tsc + vitest 63 + build + scoped audit 绿）
- **承接**：A-SYS-C-T-02 §3/§5.3（拍板"单窗多 tab"决策）/ 参考稿 §4 Center 标签栏 / X-FORBID-007。
- **文件范围**：`tabsStore`（视图层 tabs+activeTabId，openDoc/openPreview/close/activate/reset，+5 covers 测试）、`TabStrip`（center 标签栏，文档标签投影脏态/pending/external）、`Workbench`（标签栏 + 按激活标签渲染 Editor/PrvPanel + 切 workspace 重置标签）、`Editor`（移除内置标签栏）、`PrvPanel`（关闭→tabsStore.close）、`FileTree`（打开/预览→tabsStore）；移除 prvStore（被 tabsStore 取代）。
- **实现要点**：center 统一标签栏，文档(ED)+预览(PRV)同列共存；**多个预览（office/媒体/图片）可同时开为多标签**；本 PI 编辑标签单个（多文档会话归 PI-B）。同步 A-SYS-C-T-02 记录单窗多 tab 决策。
- **A 类债务**：多文档标签各自保留未保存编辑（按路径会话）= PI-B；EditorTab 组件保留（PI-B/复用）。
- **验收**：开多个图片/媒体 → 多标签共存可切换/关闭；文档标签状态标记正确；vitest 63 全过；scoped 门禁无越界。

---

## §3 实施顺序

```
PI-FE-00（IPC 桥接打通，前置）
  ↓
PI-FE-01（DesignToken + 布局骨架）→ PI-FE-02（Logo + Left Rail）
  ↓
PI-FE-03（欢迎页完整实现，依赖 PI-FE-00）
  ↓
PI-FE-04（侧边栏文件树）→ PI-FE-05（TipTap 编辑器接入）
  ↓
PI-FE-06（Chat 面板骨架）
  ↓
PI-FE-07（Diff 状态机前端：DiffCard + GreenReview + 批量条，依赖 PI-FE-05 扩展槽）
  ↓
PI-FE-08（⌘K InlineAssist / ⌘J Autocomplete，依赖 PI-FE-05 + PI-FE-06）
```

PI-FE-01 与 PI-FE-02 可并行；PI-FE-03 依赖 PI-FE-00；PI-FE-07 的 `DiffDecorationExtension`
依赖 PI-FE-05 的扩展槽预留。

## §4 横切验收

每个 PI 完成时：

- `governance:audit` 绿（APC §12 硬阻断，无宽限期）
- @GOV 追溯对应 BR-*/SM-*（不引用 REQ-*，不引用本文编号）
- term_ref 指向已注册 TERM（TERM_UNREGISTERED 为硬错误）
- 状态可视化：所有「N 状态」组件须满足视觉互斥可辨（不同状态不得靠代码知道，要靠视觉区分）
- 前端 Store 中无权威业务状态（全部为后端投影，X-FORBID-007）
- 已验证可工作（非"应该能工作"），无法验证的须明确声明"未验证"

## §5 新增 TERM 声明

本计划引入三个新核心业务概念，已在 A-SYS-C-T-02_前端交互架构与流程总图.md §0 注册
TERM 标注块（格式见 ADU §3.2），执行 `npm run governance:generate` 后同步至 ADUS 术语注册表：

| term_id | 正式中文名 | 正式英文名 | 来源文档 |
|---|---|---|---|
| TERM-SYS-001 | 设计令牌 | DesignToken | A-SYS-C-T-02 |
| TERM-DE-003 | 修改卡 | DiffCard | A-SYS-C-T-02 |
| TERM-ED-003 | 绿色审阅层 | GreenReview | A-SYS-C-T-02 |

**执行指令**（bootstrap 豁免外的刷新）：
```
npm run governance:generate
```
刷新后检查 ADUS 术语注册表出现上述三条，audit 不报 TERM_UNREGISTERED。

## §6 承接的 A 类债务（输入 A-CORE-X-R-01）

| 债务描述 | 关联 PI | 处置时机 |
|---|---|---|
| 深色主题切换（`.theme-dark` CSS 变量集） | PI-FE-01 | 实现期叠加 |
| Left Rail 模块徽标计数（PendingDiff 等） | PI-FE-02 | 对应模块 PI 完成后叠加 |
| 最近 Workspace 列表空态/加载/错误态 UI | PI-FE-03 | 实现期 |
| 文件树大目录虚拟滚动（性能） | PI-FE-04 | G5 NFR 标定 |
| TipTap/ProseMirror 版本锁定 + blockId 算法选型 | PI-FE-05 | 实现期选型回填 ED L2 §7 |
| DOCX 往返编辑（归 DOC 模块） | PI-FE-05 | DOC 模块 G4 完成后叠加 |
| `awaiting_confirmation` 完整授权 UI 联调 | PI-FE-06 | G5 联调 |
| DiffCard 与 GreenReview 滚动位置同步策略 | PI-FE-07 | 实现期决策 |
| expired `apply_replace_failed` 自动重试上限 | PI-FE-07 | G5 联调确认 |
| ⌘K 流式预览、⌘J 防抖触发时机 | PI-FE-08 | 实现期 |

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-06-22 | v1.0 | 初版：8 条前端 Plan Item（PI-FE-00～PI-FE-08），承接 IPC 桥接/DesignToken + 布局/Logo+Rail/欢迎页/文件树/TipTap/Chat 面板/Diff 状态机/三种 AI 辅助入口；对接 A-CORE-X-L-02 P2/P3；注册 TERM-SYS-001/TERM-DE-003/TERM-ED-003；A 类债务 10 项 |
