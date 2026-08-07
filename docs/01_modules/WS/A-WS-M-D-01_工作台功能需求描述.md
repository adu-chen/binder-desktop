---
文档编号：WS-M-D-01
文档状态：A
负责模块：WS
文档职责：WS 工作台模块级功能需求描述——声明模块级 REQ-WS-*，承接产品级 REQ-CORE-001/002/007，界定 workspace 生命周期 / 文件 I/O 与 save / pending_diffs 持久化与落盘 / 文档级时间轴回溯 / 控制态缓存与重启可恢复
上游约束：A-CORE-C-D-01_产品概述.md（REQ-CORE-001/002/007）、A-CORE-C-P-01_APC.md（§4.1 WS 职责、§6 save 状态机、§9 文件系统安全边界）、A-SYS-I-P-01_数据结构定义.md（WorkspaceRef/DocRef/PendingDiff/TimelineEntry、workspace.db schema 基线、BR-SYS-PERSIST-001、X-SHARED-001）、A-SYS-M-S-01_状态机总图.md（save 核心链、SY-04）、A-CORE-X-L-01_总体开发路线图.md（P2）、ADU.md（§3.2 REQ 规范）
直接承接：WS L2 技术设计（workspace 模型 / 文件系统 / 时间轴 / workspace.db 完整 schema / save·落盘·还原状态机）经 REQ_MAP 映射 RULE/CHAIN/CONSTRAINT/TERM；解除 SYS 侧 SY-03（workspace.db 完整列）与 SY-04（save 状态机）
使用边界：本文是 L1 功能需求描述层，仅声明模块级 REQ-WS-*，**不定义技术规则**（RULE/CHAIN/CONSTRAINT/TERM/STATE_MACHINE 见 WS L2）；**代码不得直接映射 REQ**（ADU §3.2）；workspace.db 具体表结构、时间轴存储模型、状态机枚举、缓存策略归 WS L2；diff 终态转移语义归 DE（WS 只负责持久化与落盘执行）
变更要求：WS 职责或 workspace 边界变更须先经 APC 授权再回写本文；REQ 增删按 ADU §3.2 req_id 规则（序号不复用，废弃改 superseded）
治理成熟度：draft
---

# Binder WS 工作台功能需求描述（L1）

## §1 文档定位

本文是 **WS 模块**的 L1 功能需求描述，处在产品概述（L1 根）之下、WS L2 技术设计之上：

```
产品概述(REQ-CORE-*) → 本文(REQ-WS-*) → REQ_MAP → WS L2 规则(SSOT) → 代码(@GOV)
```

WS 是 Binder 本地优先写作的**工作台底座**：管理 workspace 生命周期与文件 I/O，持久化 pending_diffs，
落盘人类已裁决的修改，并提供文档级时间轴与历史回溯（APC §4.1）。WS 被 ED/DE/AG/CBT/REF/AST/KB/
TPL/DOC/PRV 等模块依赖（A-SYS-C-T-01 §4：`ED→WS`、`DE→ED,WS,AG` 等）。本文只声明 WS 模块级需求，
不写技术方案、不定义规则、代码不得追溯本文；具体表结构、时间轴模型、状态机枚举、缓存策略一律归
WS L2。未被 APC / 产品概述 / SYS 契约支撑的判断一律进 §7 暴露清单，不杜撰。

## §2 模块定位与职责

**定位**：WS 是**本地优先存储与 workspace 边界的承载模块**，是文件操作 / 搜索 / diff / 上下文读取
的唯一物理边界（REQ-CORE-001），并为上层模块提供文件 I/O、持久化与历史能力。

**职责清单（v1，APC §4.1）**：

| # | 职责 | 锚定 |
|---|---|---|
| W-J1 | workspace 生命周期：打开 / 切换 / 关闭，作为文件/搜索/diff/上下文唯一边界 | REQ-CORE-001、APC §9 |
| W-J2 | 文件 I/O、save 保存与跨文档资源操作（限定 workspace 内） | REQ-CORE-001/002、APC §4.1/§6 |
| W-J3 | pending_diffs 持久化与人类裁决后落盘（接受才写文件） | REQ-CORE-002、A-SYS-I-P-01 |
| W-J4 | 文档级时间轴与历史回溯 / 还原 | REQ-CORE-007 |
| W-J5 | 控制态缓存与重启可恢复（workspace.db，联合 SYS） | A-SYS-I-P-01 BR-SYS-PERSIST-001 |

> 边界澄清：WS **持久化并落盘** diff，但 diff 的生成与"接受/拒绝/过期"**终态转移语义归 DE**
> （A-SYS-M-S-01 中 diff 终态机 owner=DE）；WS 在 `accepted` 触发后执行写文件与时间轴记节点。
> workspace.db 的 schema 基线与迁移契约归 SYS（A-SYS-I-P-01），WS 承接其完整列定义（解除 SY-03）。

## §3 能力范围（WS v1）

**IN（WS 承载）**：
- workspace 打开/切换/关闭与边界守护（路径不越出 workspace root，APC §9）。
- workspace 内文档/资源的读、写、列举、跨文档操作；编辑内容的 save 保存。
- pending_diffs 持久化（A-SYS-I-P-01 PendingDiff），人类 `accepted` 后落盘、`rejected/expired` 不写。
- 文档级时间轴记录与历史回溯 / 还原（A-SYS-I-P-01 TimelineEntry）。
- 控制态缓存与重启恢复（workspace.db 持久化，遵守 SYS BR-SYS-PERSIST-001）。

**OUT（非 WS）**：
- 富文本编辑运行时（TipTap）——归 ED；WS 提供其底层文件 I/O。
- diff 的生成、审阅 UI 与终态转移语义——归 DE（WS 仅持久化与落盘执行）。
- DOCX/格式转换——归 DOC；多格式预览/导出——归 PRV。
- 跨进程 IPC 协议本身、共享数据结构契约、workspace.db schema 基线——归 SYS（WS 消费/承接）。
- 状态机枚举、表结构、时间轴存储模型、缓存策略——归 WS L2（本文只定边界）。
- NFR 量化指标（I/O 延迟、恢复耗时）——延 G5 标定（APC §9）。

## §4 代表性流程（契约级，APC §7）

**W1 · workspace 生命周期**（W-J1）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | 用户 | 选定本地目录 | 打开为 workspace，设为文件/搜索/diff/上下文边界 | 活动 workspace | 2 | 目录无权限/不存在→提示重选 |
| 2 | WS | 文件操作请求 | 校验路径在 workspace root 内后执行 | I/O 结果 | 3 | 越界路径→拒绝（APC §9 边界） |
| 3 | 用户 | 切换/关闭 | 切换活动 workspace 或关闭 | 新活动态/已关闭 | 终态 | 有未落盘控制态→持久化后再切换 |

**W2 · diff 落盘链**（W-J3，承接 REQ-CORE-002）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | DE | 生成的 diff | WS 持久化为 pending_diff（A-SYS-I-P-01） | 持久化 pending_diff | 2 | 持久化失败→统一错误形状（A-SYS-I-P-02） |
| 2 | 用户 | 接受/拒绝 | 由 DE 终态机裁决 | accepted / rejected / expired | 3 | 超时/外部变更→expired（终态，不落盘） |
| 3 | WS | accepted 信号 | 写入目标文件并记一个时间轴节点 | 已落盘 + 时间轴节点 | 终态 | 写盘失败→不记节点，回报错误，控制态可重试 |

> W2 在 L1 仅锁边界与"接受才落盘"顺序；diff 终态机归 DE，WS 的 save/落盘状态机归 WS L2（解除 SY-04）。

**W3 · 时间轴回溯还原**（W-J4，承接 REQ-CORE-007）

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | 用户 | 打开文档时间轴 | WS 列出该文档历史节点 | 历史节点列表 | 2 | 无历史→空列表（非错误） |
| 2 | 用户 | 选定历史节点 | 预览该版本 | 版本预览 | 3 | 节点数据缺失→提示并跳过 |
| 3 | 用户 | 确认还原 | 还原到选定版本并记一个新节点 | 已还原 + 新节点 | 终态 | 还原失败→保持现状，回报错误 |

## §5 模块级需求声明（REQ-WS-*）

> owner_module = WS。每条经 source_doc 标注承接的产品级 REQ-CORE-* / SYS 契约与 APC 锚点。
> 代码不得直接映射本文 REQ（ADU §3.2）；映射到规则在 WS L2 经 REQ_MAP 完成。

<!-- REQ
req_id: REQ-WS-001
title: workspace 生命周期与唯一边界
owner_module: WS
status: active
summary: 管理 workspace 打开/切换/关闭，并以 workspace root 作为文件操作/搜索/diff/上下文读取的唯一边界，所有文件操作不得越出该边界
source_doc: REQ-CORE-001、APC §4.1、§9
acceptance: workspace 可打开/切换/关闭；任何文件操作路径在 workspace root 内方可执行，越界被拒
open_issues: 多 workspace 并存/切换时的控制态隔离细节归 WS L2
-->

<!-- REQ
req_id: REQ-WS-002
title: 文件 I/O、save 保存与跨文档资源操作
owner_module: WS
status: active
summary: 提供 workspace 内文档/资源的读写、列举与跨文档操作，以及编辑内容的 save 保存；包含供代理只读发现的项目图片资源，AI 文件操作限定在 workspace 内
source_doc: REQ-CORE-001、REQ-CORE-002、APC §4.1、§6
acceptance: workspace 内可读写/列举/跨文档操作；项目图片发现只返回未越界、未跟随符号链接的相对普通文件；save 保存可靠落盘；AI 文件操作不越出 workspace
open_issues: save 保存状态机（APC §6 核心链）归 WS L2（M-S，解除 SY-04）；并发写入边界归 WS L2
-->

<!-- REQ
req_id: REQ-WS-003
title: pending_diffs 持久化与裁决后落盘
owner_module: WS
status: active
summary: 持久化由 DE 生成的 pending_diffs（A-SYS-I-P-01 PendingDiff），仅在人类 accepted 后写入目标文件并记时间轴节点；rejected/expired 不写文件
source_doc: REQ-CORE-002、A-SYS-I-P-01（PendingDiff、workspace.db）
acceptance: pending_diffs 可持久化且重启可恢复；仅 accepted 触发落盘；rejected/expired 文件保持不变
open_issues: diff 终态转移语义归 DE；WS 侧落盘状态机归 WS L2
-->

<!-- REQ
req_id: REQ-WS-004
title: 文档级时间轴与历史回溯/还原
owner_module: WS
status: active
summary: 提供文档级版本时间轴（A-SYS-I-P-01 TimelineEntry）与历史回溯、还原到先前版本，落盘与还原各记时间轴节点
source_doc: REQ-CORE-007、A-SYS-I-P-01（TimelineEntry）
acceptance: 文档有可浏览历史节点，可回溯并还原到先前版本；还原失败不破坏现状
open_issues: 时间轴节点准入规则/存储模型/还原执行链归 WS L2（取材台账 §4.3 时间轴四件）
-->

<!-- REQ
req_id: REQ-WS-005
title: 控制态缓存与重启可恢复
owner_module: WS
status: active
summary: 缓存 workspace 控制态并持久化于 workspace.db，遵守 SYS schema 版本化迁移契约，使 pending_diffs、时间轴等控制态在应用重启后可恢复
source_doc: A-SYS-I-P-01（workspace.db schema 基线、BR-SYS-PERSIST-001、X-SHARED-001）
acceptance: 重启后未裁决 pending_diffs 与时间轴可恢复；workspace.db 迁移不丢既有控制态
open_issues: workspace.db 完整列/索引与缓存策略归 WS L2（联合 SYS，解除 SY-03）
-->

## §6 模块边界与约束（边界闭合）

- **职责边界**：WS 只承载 workspace/文件/持久化/时间轴（§2 W-J1–W-J5），不做编辑运行时、diff 生成、
  格式转换、预览；越界即范围外（CLAUDE.md §1 范围纪律）。
- **L1/L2 边界**：本文只定契约**边界**；表结构、时间轴存储模型、状态机枚举、缓存策略归 WS L2，
  代码只追溯 L2（ADU §3.2、APC §2）。
- **文件系统安全边界**（REQ-WS-001/002）：所有文件操作（含 AI）限定 workspace root 内，越界拒绝
  （APC §9）。
- **数据结构边界**：WS 复用 A-SYS-I-P-01 的 WorkspaceRef/DocRef/PendingDiff/TimelineEntry 与
  workspace.db schema 基线，遵守 X-SHARED-001（不私自重定义跨模块共享结构）、BR-SYS-PERSIST-001。
- **与 DE 的边界**：diff 生成与终态转移语义归 DE；WS 仅持久化 pending_diffs 并在 accepted 后落盘。
- **状态机边界**（APC §6）：save 保存、diff 落盘、时间轴还原为状态化核心链，须先建状态机方可实现
  （X-SEQ-003，A-SYS-M-S-01），WS G2 在其 M-S 定义并登记总图。
- **L4 原料边界**：取材见台账 §4.3（WS：workspace 模型/生命周期/文件系统/跨文档/时间轴四件等，均
  P1 原料），为原料不为权威，REQ 重新撰写（APC §8）。

## §7 G1 问题暴露清单

按 APC §5 G1 标准（REQ_GAP/BOUNDARY_OPEN/REQ_CONFLICT/FLOW_INCOMPLETE/ACCEPTANCE_MISSING/
NEEDS_HUMAN_DECISION/DESIGN_RISK）。各项均**显式延期**至 G2/G5 并记明 owner+门禁，据此 WS 模块级
G1 具备闭合条件。晋级由人类宣布（APC §2，AI 不得自行宣布晋级）。

| 编号 | 分类 | 状态 | 处置 |
|---|---|---|---|
| WS-01 | BOUNDARY_OPEN | 显式延期 | workspace.db 完整列/索引与 SYS 联合敲定（承接 SY-03），归 A-SYS-I-P-01 + WS G2 |
| WS-02 | FLOW_INCOMPLETE | 显式延期 | save 保存 / diff 落盘 / 时间轴还原 状态机归 WS L2（M-S，承接 SY-04、X-SEQ-003） |
| WS-03 | DESIGN_RISK | 显式延期 | 时间轴节点准入规则 / 存储模型 / 还原执行链取材旧文档四件，归 WS G2 |
| WS-04 | NEEDS_HUMAN_DECISION | 显式延期 | 缓存策略与多 workspace 控制态隔离属 L2 技术选型，延 WS G2 |
| WS-05 | ACCEPTANCE_MISSING | 部分关闭 | 定性验收已入 §5 各 REQ-WS-*；量化指标（I/O 延迟、恢复耗时）延 G5 NFR 标定（APC §9） |

> 术语：workspace、文档时间轴等 WS 主责核心概念将在 WS L2 注册 TERM（解除 A-SYS-I-P-01 §6 的延期），
> 本文 L1 不注册 TERM（ADU §3.2，TERM 在 L2 技术设计层注册）。

## §8 后续动作

1. 待人类宣布 WS 模块级 G1 闭合后，进入 WS G2（L2 技术设计，规则 SSOT），按路线图：
   - workspace 模型与生命周期、文件系统与跨文档操作（承接 REQ-WS-001/002）；
   - pending_diffs 持久化与落盘、save/落盘/还原状态机（承接 REQ-WS-003，解除 SY-04）；
   - 文档级时间轴存储模型与还原执行链（承接 REQ-WS-004，取材台账 §4.3 时间轴四件）；
   - workspace.db 完整 schema 与 SYS 联合（承接 REQ-WS-005，解除 SY-03）；
   - 在 WS L2 注册 workspace / 时间轴相关 TERM。
2. G2 各文档经 REQ_MAP 把 REQ-WS-* 映射为 RULE/CHAIN/CONSTRAINT/TERM；状态机入 WS M-S 并登记 A-SYS-M-S-01 总图。
3. WS 职责或 workspace 边界变更先经 APC 授权再回写本文（APC §13）。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-10 | v1.0 | 初版：WS 工作台模块级 L1；声明 REQ-WS-001~005（workspace 生命周期与边界 / 文件 I/O 与 save / pending_diffs 持久化与落盘 / 文档级时间轴回溯 / 控制态缓存与重启可恢复），承接 REQ-CORE-001/002/007 与 A-SYS-I-P-01 契约；代表性流程 W1–W3；边界闭合（含与 DE/SYS 边界）与 G1 暴露清单（WS-01~05 显式延期/部分关闭，对接 SY-03/SY-04） |
| 2026-07-29 | v1.1 | BDOCX-IMAGE-AGENT-EDIT-01：REQ-WS-002 增加 workspace 项目图片只读发现、相对普通文件与符号链接/越界保护验收。 |
| 2026-08-07 | v1.2 | DOC-SYNC-01 回填（L1 口径，承接 L2/接口）：WS-NEW-BUTTON-01「＋」新建按钮下拉（格式选择 + 激活区域 + 文件夹入口，落地 REQ-WS-002 新建语义，命令见 A-WS-I-P-01）；WS-DIR-DELETE-01 文件夹删除两级语义（空快速删/非空确认递归，`ws.delete_folder` 见 A-WS-I-P-01 §3）；WS-SAVE-TIMELINE-01 保存成功记时间轴节点（REQ-WS-004 验收口径：保存成功且内容变化时记节点，A-WS-M-T-01 v1.7 已定档）。 |
