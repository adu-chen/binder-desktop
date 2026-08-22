---
文档编号：WS-I-P-01
文档状态：A
负责模块：WS
文档职责：WS 工作台模块级接口协议（L2 接口层）——按 A-SYS-I-P-02 命名/信封/错误/流式骨架声明 WS command 的语义级签名（workspace 生命周期 / 文件 I/O 与 save / WorkspaceOperation 计划与执行 / pending_diffs / WorkspaceTimeline / 外部变更事件）
上游约束：A-SYS-I-P-02_IPC协议.md、A-SYS-I-P-01_数据结构定义.md、A-WS-M-T-01_工作台技术设计.md、A-WS-M-S-01_工作台状态机设计.md、A-WS-M-T-02_工作区资源操作技术设计.md（BR-WS-STATE-003/BR-WS-DATA-003/BR-WS-PERSIST-003）、A-WS-M-S-02_工作区资源操作状态机设计.md（SM-WS-04）、ADU.md
直接承接：实现期 Rust command 注册表按本文清单注册（未注册即协议错误，BR-SYS-DATA-003）；ED/DE/AG/CBT 等调用方按本文签名经 Service 层调用；@GOV 标注落在实现代码（追溯 A-WS-M-T-01 规则，本文为签名契约）
使用边界：本文声明 command 的**语义级签名**（参数/结果的字段语义），不锁定序列化格式与具体标量类型（延实现，承 I-P-02 边界）；不新增 RULE/CHAIN/CONSTRAINT/TERM（命令行为已由 A-WS-M-T-01 规则覆盖，本文是契约细化非规则源）；错误仅列**特有错误类**（通用错误形状归 I-P-02）
变更要求：command 增删改须同步更新实现注册表与调用方；签名变更属接口变更，须复核全部调用方模块；涉及共享结构变更回写 A-SYS-I-P-01
治理成熟度：draft
---

# Binder 工作台接口协议（WS · L2 接口层）

## §1 文档定位

```
A-SYS-I-P-02（协议骨架：命名/信封/错误/流式）→ 本文（WS command 语义签名清单）→ 实现注册表 + @GOV(追溯 A-WS-M-T-01)
载荷结构 ←引用← A-SYS-I-P-01（WorkspaceRef/DocRef/PendingDiff/TimelineEntry）
```

约定：所有 command 经统一信封（correlation_id 等，I-P-02 §3）；失败回统一错误形状（I-P-02 §4），
本文仅标注**特有错误类**（code 语义类别，具体错误码表延实现）；路径类参数一律受 X-FORBID-004
守护（越界拒绝），表中不再逐条重复。

## §2 workspace 生命周期（WS-FS / BR-WS-DATA-001）

| command | 参数（语义） | 结果 | 特有错误类 | 状态机/规则归属 |
|---|---|---|---|---|
| `ws.open_workspace` | root_path（本地目录绝对路径） | WorkspaceRef（注册并置活动） | 目录不存在/无权限 | BR-WS-DATA-001 |
| `ws.switch_workspace` | workspace_id | 新活动 WorkspaceRef | 未注册 workspace；切换前未落盘控制态持久化失败 | §3 生命周期（A-WS-M-T-01） |
| `ws.close_workspace` | workspace_id | 确认 | — | 控制态保留可恢复 |
| `ws.list_workspaces` | — | WorkspaceRef 列表 | — | — |

## §3 文件 I/O 与 save（WS-FS / WS-SAVE）

| command | 参数（语义） | 结果 | 特有错误类 | 状态机/规则归属 |
|---|---|---|---|---|
| `ws.list_docs` | workspace_id、过滤条件（可空） | FsEntry 列表（rel_path+doc_type+is_dir，**含空目录**；WS 本地结构非 SYS 共享 DocRef，PI-BE-07 补） | — | BR-WS-DATA-001 |
| `ws.list_project_images`（内部只读服务） | active workspace；可选 `.binder/project_assets.json`（`version=1`、1–16 个安全相对 `imageRoots`；缺省仅 `images/`） | `imageRoots`、配置来源和图片候选相对路径列表 | 配置/图片根无效明确失败；隐藏项跳过；任何符号链接或越界来源拒绝 | BR-WS-DATA-001；供 AG list_images，格式/hash 由 DOC 校验；不退回全 workspace 扫描 |
| `ws.prepare_project_image`（内部边界，由 DOC/AG 消费） | active workspace；已由用户拖放授权的本地路径、项目文件树相对路径、剪贴板图片字节，或代理给出的 workspace 相对路径 | 配置根内直接复用；其他路径/剪贴板字节复制到首个 imageRoot 的 `imported/`，返回项目相对路径 | 不移动/删除源文件；符号链接、目录、越界、非 PNG/JPEG、重名内容漂移拒绝；同 SHA-256 去重；临时 data URL 不持久化 | BR-WS-DATA-001；图片格式与媒体身份由 DOC 校验 |
| `ws.read_doc` | DocRef | 内容 + **加载基线哈希**（BR-WS-OBS-001；IPC 为十进制字符串） | 文件不存在/不可读 | 基线供 save 校验与外部变更检测 |
| `ws.create_doc` | DocRef、初始内容（可空） | DocRef + 基线哈希 | 已存在/越界 | BR-WS-DATA-001 |
| `ws.create_folder` | DocRef（目录 rel_path） | rel_path + created/existed | 目标为文件/越界 | BR-WS-DATA-001 |
| `ws.delete_folder` | DocRef（目录 rel_path）、mode（empty \| recursive） | rel_path + deleted | 不存在（WS_NOT_FOUND）/目标为文件（WS_EXISTS）/非空（WS_NOT_EMPTY，empty 模式）/越界 | BR-WS-DATA-001 |
| `ws.rename_doc` | DocRef、新 rel_path | 新 rel_path | **WS_EXISTS**（目标已存在）/源不存在/越界 | BR-WS-DATA-001（§8 move 签名，PI-BE-04 补） |
| `ws.delete_doc` | DocRef | 删除确认 | 不存在/越界 | BR-WS-DATA-001 |
| `ws.save_doc` | DocRef、内容、**基线哈希**（十进制字符串或兼容数字）、override（默认 false） | 新基线哈希（十进制字符串） | **WS_SAVE_CONFLICT / save_conflict 语义**（磁盘≠基线且未 override，SM-WS-01 saveConflict）；写入失败 | SM-WS-01 / BR-WS-PERSIST-001 / BR-WS-OBS-001 / BR-WS-DATA-002（保存成功且内容与最近节点快照不同时记一个 `save` 时间轴节点）；override=true 即 overrideConfirmed 语义，必须源自用户显式确认 |
| `ws.search` | workspace_id、查询 | 命中列表（DocRef + 定位） | — | 限界 X-FORBID-004 |

> `ws.create_doc/create_folder/delete_folder/rename_doc/delete_doc` 是 WS 内部原子执行器或人工文件树入口的
> 低层能力，不是 Agent 资源授权接口。Agent、TaskRun 与模型工具不得直接调用这些 command；必须使用下节
> WorkspaceOperation 计划与执行接口。既有直接调用属于迁移债务。

### §3.1 WorkspaceOperation（WS-OPERATION / SM-WS-04）

| command | 参数（语义） | 结果 | 特有错误类 | 状态机/规则归属 |
|---|---|---|---|---|
| `ws.plan_operation` | workspace_id、受限 operation intents、TaskRun/AgentTurn 来源 | WorkspaceOperationPlan（不可变 items、影响清单、基线、补偿策略、idempotency_key） | 越界/不支持操作/无法冻结清单/目标冲突 | Planning；BR-WS-STATE-003 / BR-WS-DATA-003 |
| `ws.get_operation_plan` | plan_id | 完整计划 + 当前状态 + DecisionRequest 关联 | 不存在/跨 workspace | BR-WS-DATA-003 |
| `ws.execute_operation` | plan_id、resolved DecisionRequest identity、outcome=confirm | WorkspaceOperationReceipt + WorkspaceOperationEntry | 未确认、确认不匹配、计划过期、基线变化、dirty target、幂等冲突、补偿失败 | Revalidating→Executing/Compensating；BR-WS-STATE-003 / BR-WS-PERSIST-003 |
| `ws.cancel_operation` | plan_id、resolved DecisionRequest identity、outcome=cancel | Rejected 回执，零磁盘副作用 | 已执行/终态 | Rejected |
| `ws.list_operations` | workspace_id、状态过滤 | 计划与执行摘要 | — | 可观测与恢复 |

`ws.execute_operation` 只能接受真实已解决 DecisionRequest 的身份；布尔 `confirmed=true`、模型参数、Prompt 文本或
前端任意字段均不是授权。重复 confirm/execute 由 idempotency_key 返回同一终态回执，不重复执行。重启恢复时
Planning/WaitingConfirm/Executing/Compensating 一律投影为 Interrupted，接口不得自动调用 execute。

## §4 pending_diffs（WS-DIFF-APPLY）

| command | 参数（语义） | 结果 | 特有错误类 | 状态机/规则归属 |
|---|---|---|---|---|
| `ws.persist_diff` | PendingDiff（A-SYS-I-P-01） | diff_id（status=pending） | 持久化失败 | BR-DE-DATA-001 的 WS 侧 |
| `ws.list_pending_diffs` | workspace_id、过滤（status/rel_path 可空） | PendingDiff 列表 | — | 重启恢复入口 |
| `ws.apply_diff` | diff_id | Applied 回执 + 时间轴 entry_id | apply 失败（不留半节点，可重试） | SM-WS-02（仅 DE accepted 信号方可调用——调用方约束，BR-WS-STATE-001） |
| `ws.discard_diff` | diff_id、终态（rejected/expired） | Discarded 回执 | — | SM-WS-02（不写文件） |

## §5 统一 WorkspaceTimeline（WS-TIMELINE）

| command | 参数（语义） | 结果 | 特有错误类 | 状态机/规则归属 |
|---|---|---|---|---|
| `ws.list_timeline` | workspace_id、可选 DocRef/entry_kind 过滤 | 文档内容 TimelineEntry 与 WorkspaceOperationEntry 的统一序列 | — | BR-WS-DATA-002 / BR-WS-DATA-003 |
| `ws.preview_node` | entry_id | 按 entry_kind 返回内容快照或资源操作/恢复清单 | 快照不可取回 | SM-WS-03 或 SM-WS-04 Planning |
| `ws.restore_node` | entry_id | 文档内容还原回执，或新的 WorkspaceOperationPlan（资源条目） | 还原失败/资源冲突/不可恢复条目 | 内容走 SM-WS-03；资源恢复必须新建计划并重新 DecisionRequest，不直接执行 |

## §6 事件回传（流式逻辑契约，I-P-02 §5）

| 事件 | 载荷（语义） | 订阅方 | 归属 |
|---|---|---|---|
| `ws.external_change` | DocRef + 变更类型（modified/deleted/moved）+ 当前哈希 | ED（SM-ED-01 externalChange）、DE（过期重检）、REF（锚校验） | BR-WS-OBS-001 |

> 物理传输（Channel/event）维持 SY-02 延实现期选型；本表按 I-P-02 流式逻辑契约声明。
> **当前实现（2026-07-12）**：独立 `ws.check_external_change` command 未注册；本阶段已落地的外部变更
> 保护下限是 `ws.save_doc` / `doc.save_from_edit` 保存前 baseline guard，冲突以 `WS_SAVE_CONFLICT`
> 返回 ED。后续若实现轮询或推送，需要在本文新增/恢复 command 并同步实现注册表。

## §7 测试与验收

- 实现注册表与本文清单一致（未注册 command 协议错误，BR-SYS-DATA-003）；每 command 集成测试覆盖
  正路 + 特有错误类；`ws.save_doc` 冲突路（基线偏离 + override 路径）强制覆盖（SM-WS-01 矩阵）；
  `ws.apply_diff` 仅在 accepted 后被调用的约束在 DE/WS 联调验证。

## §8 债务与盲区

- 错误码表（code 枚举值）、序列化与标量类型延实现期统一定义（全模块一张表，归 SYS 实现期补遗）。
- ~~跨文档资源操作（move/copy）签名随实现需求补充本文~~ → **move 已补（PI-BE-04，2026-06-23）**：
  §3 `ws.rename_doc`（同 workspace 内移动/改名，目标已存在拒绝）+ `ws.delete_doc`；**建目录已补**
  （AG-STRUCTURE-FILE-TOOLS-01，2026-07-16）：`ws.create_folder` 目标为目录时幂等成功、目标为文件拒绝。
  copy 仍延后（按需补）。
- 媒体/二进制读写（ED 媒体、DOC 字节流）是否独立 command 或复用 read/save 延实现期定。
- `ws.external_change` 物理事件和独立外部变更检测 command 延 SY-02；当前不得在调用方假设
  `ws.check_external_change` 已注册。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（首份模块接口协议，实现前置件）：按 I-P-02 骨架声明 WS 全部 command 语义签名——workspace 生命周期 4 条、文件 I/O 与 save 5 条（read 返回加载基线、save 带基线校验与 override 显式覆盖语义）、pending_diffs 4 条、时间轴 3 条、外部变更事件 1 条；零新增注册项（契约细化非规则源）；债务（错误码表/序列化/跨文档操作签名延实现期） |
| 2026-07-12 | v1.1 | 三态实现回写：`ws.read_doc`/`ws.save_doc` baseline IPC 字符串化；保存冲突错误码对齐 `WS_SAVE_CONFLICT`；移除当前 command 清单中的未注册 `ws.check_external_change`，将外部变更推送/轮询登记为 SY-02 后续能力；明确当前已落地保护为保存前 baseline guard |
| 2026-07-16 | v1.2 | 新增 `ws.create_folder` command：Workspace 内建目录，目标为目录时幂等成功并返回 existed，目标为文件/越界拒绝；供 Agent 文件整理结构工具复用 |
| 2026-07-29 | v1.3 | BDOCX-IMAGE-AGENT-EDIT-01：新增内部只读 ws.list_project_images 语义，锁定 workspace 相对路径、普通文件、隐藏目录/符号链接/越界边界。 |
| 2026-07-29 | v1.4 | BDOCX-IMAGE-ADVANCED-AGENT-OPS-01：项目图片发现收口为 `.binder/project_assets.json` 受控 `imageRoots`，缺省仅 `images/`；返回根目录与配置来源，禁止回退扫描整个 workspace。 |
| 2026-08-04 | v1.5 | `ws.save_doc` 副作用补充（人类决策，A-WS-M-T-01 v1.7）：保存成功且内容与最近节点快照不同时记一个 `save` 时间轴节点，规则归属补 BR-WS-DATA-002 |
| 2026-07-29 | v1.5 | BDOCX-IMAGE-COMPOSITE-IMPORT-01：新增受控项目图片准备边界；配置根内复用，其他来源复制到 imported 子目录，默认保留源文件并按内容哈希去重。 |
| 2026-07-30 | v1.6 | BDOCX-IMAGE-COMPOSITE-IMPORT-01 人工入口闭合：受控项目图片准备边界纳入项目文件树相对路径和剪贴板 PNG/JPEG 字节，继续复用同一图片根、哈希去重与源文件保护约束。 |
| 2026-08-04 | v1.7 | `ws.save_doc` 副作用补充（人类决策，A-WS-M-T-01 v1.7）：保存成功且内容与最近节点快照不同时记一个 `save` 时间轴节点，规则归属补 BR-WS-DATA-002 |
| 2026-08-06 | v1.8 | WS-DIR-DELETE-01：新增 `ws.delete_folder` command（镜像 `ws.create_folder`）：两级语义——mode=empty 仅删空目录（非空拒 WS_NOT_EMPTY）、mode=recursive 递归删除；缺失/目标为文件拒绝；删除后由实现侧清洗 bdocx_manifest。文件删除 `ws.delete_doc` 语义不变 |
| 2026-08-07 | v1.9 | WS-NEW-BUTTON-01：文件树「＋」新建按钮下拉（格式选择 + 激活区域 + 文件夹入口）为前端入口组合，复用既有 `ws.create_doc` / `ws.create_folder` command，无新增命令面；登记本行避免前端入口与协议脱节。 |
| 2026-08-21 | v1.10 | 工作区资源操作基线重构：新增 ws.plan/get/execute/cancel/list_operation 语义接口；Agent 资源操作不得直接调用低层文件 command；统一 WorkspaceTimeline 支持类型化资源条目，资源恢复返回新的前置裁决计划。 |
