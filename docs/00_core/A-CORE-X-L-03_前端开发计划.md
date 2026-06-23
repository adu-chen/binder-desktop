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
