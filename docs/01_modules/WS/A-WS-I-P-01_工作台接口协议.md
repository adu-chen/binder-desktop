---
文档编号：WS-I-P-01
文档状态：A
负责模块：WS
文档职责：WS 工作台模块级接口协议（L2 接口层）——按 A-SYS-I-P-02 命名/信封/错误/流式骨架声明 WS 全部 Tauri command 的语义级签名（workspace 生命周期 / 文件 I/O 与 save / pending_diffs / 时间轴 / 外部变更事件）
上游约束：A-SYS-I-P-02_IPC协议.md（命名约定 `模块前缀.动作`、请求-响应信封、统一错误形状、流式逻辑契约——本文全部 command 在该骨架上承接，不重定义信封/错误/流式语义）、A-SYS-I-P-01_数据结构定义.md（WorkspaceRef/DocRef/PendingDiff/TimelineEntry 载荷结构）、A-WS-M-T-01_工作台技术设计.md（BR-WS-* 规则、X-FORBID-004）、A-WS-M-S-01_工作台状态机设计.md（SM-WS-01/02/03 状态机归属）、ADU.md
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
| `ws.list_docs` | workspace_id、过滤条件（可空） | DocRef 列表 | — | BR-WS-DATA-001 |
| `ws.read_doc` | DocRef | 内容 + **加载基线哈希**（BR-WS-OBS-001） | 文件不存在/不可读 | 基线供 save 校验与外部变更检测 |
| `ws.create_doc` | DocRef、初始内容（可空） | DocRef + 基线哈希 | 已存在/越界 | BR-WS-DATA-001 |
| `ws.save_doc` | DocRef、内容、**基线哈希**、override（默认 false） | 新基线哈希 | **save_conflict**（磁盘≠基线且未 override，SM-WS-01 saveConflict）；写入失败 | SM-WS-01 / BR-WS-PERSIST-001 / BR-WS-OBS-001；override=true 即 overrideConfirmed 语义，必须源自用户显式确认 |
| `ws.search` | workspace_id、查询 | 命中列表（DocRef + 定位） | — | 限界 X-FORBID-004 |

## §4 pending_diffs（WS-DIFF-APPLY）

| command | 参数（语义） | 结果 | 特有错误类 | 状态机/规则归属 |
|---|---|---|---|---|
| `ws.persist_diff` | PendingDiff（A-SYS-I-P-01） | diff_id（status=pending） | 持久化失败 | BR-DE-DATA-001 的 WS 侧 |
| `ws.list_pending_diffs` | workspace_id、过滤（status/rel_path 可空） | PendingDiff 列表 | — | 重启恢复入口 |
| `ws.apply_diff` | diff_id | Applied 回执 + 时间轴 entry_id | apply 失败（不留半节点，可重试） | SM-WS-02（仅 DE accepted 信号方可调用——调用方约束，BR-WS-STATE-001） |
| `ws.discard_diff` | diff_id、终态（rejected/expired） | Discarded 回执 | — | SM-WS-02（不写文件） |

## §5 时间轴（WS-TIMELINE）

| command | 参数（语义） | 结果 | 特有错误类 | 状态机/规则归属 |
|---|---|---|---|---|
| `ws.list_timeline` | DocRef | TimelineEntry 列表（version_seq 序） | — | BR-WS-DATA-002 |
| `ws.preview_node` | entry_id | 该版本内容（快照重建） | 快照不可取回（guard 对应 SM-WS-03） | SM-WS-03 Previewing |
| `ws.restore_node` | entry_id | Restored 回执 + 新 entry_id | 还原失败（保持现状） | SM-WS-03 / BR-WS-STATE-002 |

## §6 事件回传（流式逻辑契约，I-P-02 §5）

| 事件 | 载荷（语义） | 订阅方 | 归属 |
|---|---|---|---|
| `ws.external_change` | DocRef + 变更类型（modified/deleted/moved）+ 当前哈希 | ED（SM-ED-01 externalChange）、DE（过期重检）、REF（锚校验） | BR-WS-OBS-001 |

> 物理传输（Channel/event）维持 SY-02 延实现期选型；本表按 I-P-02 流式逻辑契约声明。

## §7 测试与验收

- 实现注册表与本文清单一致（未注册 command 协议错误，BR-SYS-DATA-003）；每 command 集成测试覆盖
  正路 + 特有错误类；`ws.save_doc` 冲突路（基线偏离 + override 路径）强制覆盖（SM-WS-01 矩阵）；
  `ws.apply_diff` 仅在 accepted 后被调用的约束在 DE/WS 联调验证。

## §8 债务与盲区

- 错误码表（code 枚举值）、序列化与标量类型延实现期统一定义（全模块一张表，归 SYS 实现期补遗）。
- 跨文档资源操作（move/copy）签名随实现需求补充本文（REQ-WS-002 范围内）。
- 媒体/二进制读写（ED 媒体、DOC 字节流）是否独立 command 或复用 read/save 延实现期定。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（首份模块接口协议，实现前置件）：按 I-P-02 骨架声明 WS 全部 command 语义签名——workspace 生命周期 4 条、文件 I/O 与 save 5 条（read 返回加载基线、save 带基线校验与 override 显式覆盖语义）、pending_diffs 4 条、时间轴 3 条、外部变更事件 1 条；零新增注册项（契约细化非规则源）；债务（错误码表/序列化/跨文档操作签名延实现期） |
