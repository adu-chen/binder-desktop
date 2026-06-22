# Handoff: Binder · 本地 AI 文档工作台 UI

> 这是一个面向 **Claude Code** 的实现交接包。下面所有 HTML/JSX 文件都是
> **设计参考稿（用 HTML + React 在浏览器里搭出的高保真原型）**，
> 不是要直接搬进生产环境的代码。
> 你的任务是：**在目标工程既有的技术栈与组件体系中复现这些设计**
>（Binder 实际是 Tauri 2 + React + TipTap，见下文），
> 而不是把这些原型文件原样拷贝上线。

---

## 1. Overview

Binder 是一个 **本地优先（local-first）的桌面 AI 文档工作台**。核心形态是
"工作区 + 编辑器 + AI 协作"三位一体：

- 选择一个本地目录作为 **workspace**
- 在文件树中管理 / 编辑 Markdown、txt、HTML 等文档
- 在右侧以 **chat / agent** 两种模式与 AI 协作
- **AI 的所有修改不直接落盘**，先生成 **Diff 候选**，由用户逐条 / 批量决策后才写入
- 所有操作记入 **时间轴（workspace.db）**，可回滚

这套设计稿的目标，是把上述产品的**全部界面状态**（不只是"正常态"）固化成可实现的规范：
8 种 Diff 状态、6 种工具调用状态、空/加载/错误态全覆盖、7 条关键交互链路。

**最高设计原则（贯穿所有界面，实现时务必保留）：**
> **先确认再生效 · 中间态必须可见 · 一切可控、可追踪、可回滚。**

---

## 2. Fidelity

**高保真（hi-fi）。** 颜色、字体、间距、状态机、交互文案都是最终值，请按本文档
**像素级复现**。唯一允许偏离的是：把内联 mock 组件换成目标工程里已有的组件库实现
（按钮、对话框、列表等），只要视觉 token 一致即可。

---

## 3. 目标技术栈（Binder 真实环境）

| 层 | 技术 |
|----|------|
| 桌面壳 | **Tauri 2**（Rust 后端 + WebView 前端） |
| 前端 | **React** |
| 富文本编辑器 | **TipTap 3 / ProseMirror** |
| 本地存储 | **SQLite**（`workspace/.binder/workspace.db`, `search.db`） |
| AI Provider | **仅 OpenAI 与 DeepSeek**（不要在 UI 里出现 Anthropic / Gemini 等） |
| 文档转换 | Pandoc / LibreOffice（外部依赖） |

**编辑器关键扩展**（TipTap/ProseMirror，实现 Diff 与引用机制的核心）：
`BlockIdExtension`、`DiffDecorationExtension`、`CopyReferenceExtension`、
`PageTopCaretExtension`、`FontSize`。

> **重要边界**（不要在 UI 上"画饼"）：
> - Excel / PPT **只能预览，不能编辑**
> - 没有多 Agent 协同
> - 全文搜索面板尚未挂载到主布局；当前左侧用文件树过滤搜索

---

## 4. 全局布局（Main Workbench）

应用主界面是经典三栏 + 上下条，CSS Grid：

```
grid-template-rows: [titlebar 32px] [work 1fr] [diffstrip auto] [statusbar 24px]

work 区 grid-template-columns:
  [rail 44px] [side 272px] [center 1fr] [chat 372px]
```

| 区域 | 说明 |
|------|------|
| **① TitleBar (32px)** | 红黄绿窗控点 + logo + 当前文件路径 + 模式/模型标识 |
| **② Left Rail (44px)** | 5 大模块切换：工作区 / 记忆 / 知识 / 模板 / 时间轴，每项带罗马编号 `I·A`～`I·E` |
| **③ Side Panel (272px)** | 当前模块的内容（文件树 / 记忆列表 / 知识列表 / 模板列表 / 时间轴） |
| **④ Center (1fr)** | 编辑器：标签栏 + 工具栏 + 纸张式正文（TipTap）+ Green Review 装饰 |
| **⑤ Chat (372px)** | chat/agent 切换 + 会话标签 + 消息流 + 工具卡 + Diff 卡 + composer |
| **⑥ Diff Strip (auto)** | 有待决策 Diff 时浮现的批量审阅条，否则隐藏 |
| **⑦ Status Bar (24px)** | 就绪状态 / workspace / 文件数 / 模式 / 模型 / 光标位置 / Pending 数 |

无 workspace 时显示 **欢迎页**：最近工作区列表 + 打开/新建/配置 Key + 临时 chat（仅 chat 模式）。

---

## 5. 核心对象（实现时建议的数据模型）

```
Workspace ── Files (tree) ── EditorTab ── Document ── Selection
ChatSession ── ChatMessage ── ToolCall
                            └─ DiffCard ─┐
PendingDiff ────────────────────────────┴─ GreenReview (editor decoration)
Reference (file / memory / knowledge)
InlineAssist (⌘K)   AutoComplete (⌘J)
TimelineNode
```

---

## 6. 关键状态机（**实现重点 — 必须全覆盖**）

### 6.1 Diff Card · 8 状态

每条 AI 修改建议是一个 Diff。状态视觉必须**互斥可辨**：

| 状态 | 颜色语义 | UI 表现 | 可用操作 |
|------|----------|---------|----------|
| `pending` | 锈红实边 | PENDING badge + 红边 | 接受 ⏎ / 拒绝 ⌫ |
| `validating` | 金 | 旋转器 + "校验原文匹配中" | 按钮禁用 |
| `applying` | 金 | 进度条 + "写入中" | 按钮禁用 |
| `accepted` | 森林绿 | ACCEPTED badge + ✓ + 时间轴号 | 无（终态） |
| `rejected` | 灰弱化 | 降透明度 + ⌫ 说明 | 无（终态） |
| `expired` | 灰边 | ! + 失效原因 + "重新生成" | 重新生成 / 关闭 |
| `conflict` | 金 | ⌖ + 重叠的另一条 Diff id | 接受禁用，可单独拒绝 |
| `error` | 锈红边 | × + 错误码 | 重试 ↻ / 拒绝 |

**`expired` 的 5 种细分原因**（每种都要有对应文案，引导"重新生成"而非显示技术报错）：
- `document_revision_advanced` — 原文已被手动编辑
- `block_resolve_failed` — 原始块已不可定位
- `overlapping_range` — 与已接受 Diff 范围重叠
- `external_file_changed` — 文件在磁盘上被外部修改
- `apply_replace_failed` — 多次重试后写入仍失败

### 6.2 Tool Call · 6 状态
`pending(排队)` → `executing(旋转器+进度条)` → `done(OK+耗时)`；
分支：`awaiting_confirmation`（**危险操作授权门**，锈红边，需用户授权）、
`failed`（可重试 / 跳过）、`skipped`（多次失败后系统跳过）。

### 6.3 Editor Tab · 6 状态
`normal` / `dirty(红点)` / `saving(旋转器)` / `saved(回退默认，不常驻对勾)` /
`pending_diff(红色数字 badge)` / `external_changed(金色 ! )`

### 6.4 File Tree Node · 6 状态
`normal` / `open(锈红左条)` / `dirty_open(红点)` / `pending_diff(数字 badge)` /
`external_changed(金 !)` / `not_supported(灰禁用，如 xlsx/pptx)`

### 6.5 DiffAllActionsBar（批量条）· 3 状态
`有待决策(红色计数+全部接受/拒绝)` → `倒序应用中(金色进度，从文档靠后的 Diff 先应用以避免位置偏移)` → `完成(绿，含失效统计)`

### 6.6 三种 AI 辅助入口（**视觉与行为必须互斥**）
- **⌘J AutoComplete**：光标处低权重浮层，3 条续写建议，Tab/↩ 选择 —— 不产出 Diff
- **⌘K Inline Assist**：选区上方对话弹窗，输入 → 生成 → 预览 → Apply —— **直接替换选区，不进入 Diff 链路**
- **Chat Diff**：右栏卡片，有 Accept/Reject —— 进入完整 Diff 链路

### 6.7 Green Review（编辑器装饰层）
基于 ProseMirror Decoration **叠加在正文之上，不替换 DOM、不阻挡编辑**。
4 态：pending(锈红左条) / applying(金) / accepted(绿) / expired(淡出)。
**与右侧 Diff Card 一一对应**：任意一侧 Accept/Reject，另一侧同步。

---

## 7. 关键链路（交互流程，每条都含异步与异常）

| 链路 | 名称 | 帧 | 要点 |
|------|------|----|------|
| **A** | AI 修改当前文档（核心主链） | 5 | 发送→思考→工具流式→Green Review→接受写入 |
| **B** | 全部接受 · 倒序执行 | 3 | 从文档靠后的 Diff 先应用，避免行号偏移；含 1 条失效 |
| **C** | Diff 失效处理 | 3 | 用户手动编辑被覆盖段 → Diff 自动 expired → 引导重新生成 |
| **D** | 外部文件被修改 | 3 | 每 5s 比对 mtime → 顶部 Banner 询问 → 加载外部更改则全部 Diff 失效 |
| **F** | ⌘K 局部修改 | 4 | 选区→输入→生成→Apply；不进 Diff 链路 |
| **G** | 关闭应用前 Pending Diff 警告 | 1 | 带蒙版 dialog，列出待决策 Diff 分布；"先处理"/"放弃并关闭" |
| **H** | 危险工具授权门 | 2 | delete/move/rename 需授权；拒绝则 AI 收到 cancellation |

---

## 8. Design Tokens（**精确值 — 直接照搬**）

### 颜色（浅色主题）
| Token | 变量 | 值 | 用途 |
|-------|------|-----|------|
| D-INK | `--ink` | `#0E0C0A` | 主文字 / 描边强调 |
| D-PAPER | `--bg-page` | `#F4F1EA` | 纸面底色 / 应用背景 |
| D-CLAY | `--rust` | `#C24A2A` | 强调点 / Pending / Reject 文案 |
| FOREST | `--forest` | `#2E6F4A` | Accept / 成功（**仅语义，不做品牌色**） |
| GOLD | `--gold` | `#B58A2E` | 中间过程 / Conflict / 警示 |
| ELEVATED | `--bg-elevated` | `#F8F5EC` | 面板/标题栏底 |
| SUNKEN | `--bg-sunken` | `#E6E2D8` | 状态栏 / 聊天头 |
| CARD | `--bg-card` | `#FFFFFF` | 纸张 / 卡片表面 |
| RULE | `--rule` | `rgba(14,12,10,0.10)` | 常规分割线 |
| RULE-STRONG | `--rule-strong` | `rgba(14,12,10,0.22)` | 强分割 / 控件边 |

文字阶 ink-2~ink-6 为 ink 的 70/50/30/16% alpha。**深色主题见 `app-styles.css` 的 `.theme-dark`**（背景 `#0E0C0A`，clay 提亮到 `#D85E3A`）。

### 字体
- Sans：**IBM Plex Sans**（400/500/600/700），display 字距 −0.04em，标题 −0.01em
- Mono：**IBM Plex Mono**（用于所有大写标签、编号、代码、数值）
- 中文：**Noto Sans SC**
- 备用衬线：Instrument Serif

字阶：Display 44 / H1 22(600) / H2 16(600) / Body 14(1.65) / Meta 12 / Label 10(mono, 0.08em tracking, uppercase)

### 间距 / 圆角 / 描边
- Spacing：4 · 8 · 12 · 16 · 24 · 32 · 48
- Radius：0（多数硬朗结构）/ 2（badge、按钮）/ 4（logo、控件）/ 8
- Stroke：0.5px 常规线 · 1px 强线 · 2px 锈红强调条
- 描边美学：**大量 0.5px 细线 + 直角**，几乎不用阴影；卡片靠描边而非圆角区分

### z-index 层级
| 层 | 组件 | z |
|----|------|---|
| L1 内容 | 编辑器 / 文件树 / 聊天 | 0 |
| L2 装饰 | Green Review (Decoration) | 1 |
| L3 固定栏 | 顶栏 / 状态栏 / DiffAllActionsBar | 5 |
| L4 浮层 | AutoComplete / Inline Assist / Reference Picker | 11 |
| L5 弹窗 | Confirm Dialog / 授权卡 / 关闭警告（带蒙版） | 20 |
| L6 系统 | Toast / 全局错误条 | 30 |

---

## 9. Logo / 品牌标记（Binder Core VI · D-06）

"段落条堆叠"标记，120×120 grid：
- **主形态**：8 条横条，宽度节奏 `100/64/82/100/72/100/56/100`（百分比），第 4 条（0-indexed 3）用 D-CLAY 锈红点缀，其余 D-INK；padding 14，barH 8
- **紧凑形态（≤22px）**：4 条 `100/56/100/72`，barH 16，保留锈红点
- **favicon 形态**：4 条单色，无强调点
- **文字标**：`binder` + 锈红句点 `.`，IBM Plex Sans 500，字距 −0.04em

实现见 `binder-parts.jsx` 的 `BinderMark` / `BinderWordmark`，是无依赖的纯 SVG，可直接移植。

---

## 10. Assets

无外部图片资源。所有图形（logo、图标）均由 SVG / Unicode 字符 + CSS 绘制。
文件类型图标用 mono 字符（`M↓` `TXT` `XLS` 等），实现时可替换为目标工程的图标库。

---

## 11. 本包文件清单

| 文件 | 内容 |
|------|------|
| `index.html` | 入口，design_canvas 编排全部 22 张画板 + Tweaks 面板 |
| `app-styles.css` | **核心 token + 全部主界面组件样式**（浅/深主题） |
| `app-styles-ext.css` | 展示页 / 流程帧 / 浮层 / 对话框扩展样式 |
| `mock-data.jsx` | 全部示例数据 + 状态目录（DIFF_STATES / TOOL_STATES / FLOW_* 等） |
| `binder-parts.jsx` | Logo、Left Rail、5 个 Side Panel |
| `binder-editor.jsx` | 编辑器标签 / 工具栏 / 纸张正文 / 状态栏 / Diff Strip / Inline Assist |
| `binder-chat.jsx` | 聊天面板：模式切换 / 消息 / 工具卡 / Diff 卡 / composer |
| `binder-app.jsx` | 主 App 组合 + 欢迎页 |
| `binder-showcase.jsx` | §02 Diff Card 8 状态 |
| `binder-showcase-2.jsx` | §03 Tool · §04 Tab/Node · §05 Green Review · §06 浮层 |
| `binder-showcase-3.jsx` | §07 链路 A~H · §08 空/加载/错误态 · §09 Tokens |

**阅读顺序建议**：先读本 README → `app-styles.css` 拿 token →
`binder-app.jsx` 看主布局组合 → 按需深入各组件文件。
状态机的真值表在 `mock-data.jsx`（`DIFF_STATES` / `EXPIRED_REASONS` /
`TOOL_STATES` / `TAB_STATES` / `FILE_NODE_STATES`）。

---

## 12. 实现建议

1. 先复现 **§01 主工作台**的三栏骨架与 token，确保布局与配色像素对齐
2. 再实现 **Diff 状态机**（§02 + §06.7 Green Review + §04.5 批量条）——这是产品识别度最高、最复杂的部分，建议优先且独立测试每个状态
3. **TipTap 集成**：用 ProseMirror Decoration 实现 Green Review，确保装饰层不阻挡编辑、与右侧 Diff Card 双向同步
4. 三种 AI 入口（⌘J / ⌘K / Chat Diff）的行为差异不能混淆，尤其 **⌘K 不产出 Diff**
5. 异常链路（C/D/G/H）容易被忽略，但它们决定产品"可信赖"的体感，请完整实现
