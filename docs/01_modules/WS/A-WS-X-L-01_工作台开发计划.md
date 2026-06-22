---
文档编号：WS-X-L-01
文档状态：A
负责模块：WS
文档职责：WS 工作台 G3 开发计划——承接型 Plan Item，承接 WS-FS/WS-SAVE/WS-DIFF-APPLY/WS-TIMELINE 链路与 BR-WS-* 规则、X-FORBID-004、SM-WS-01/02/03，明确文件范围/@GOV/A 类债务/验收（对接 A-CORE-X-L-02 P2）
上游约束：A-CORE-X-L-02_总开发实施计划.md（P2 验收）、A-CORE-C-P-01_APC.md（§5 G3/G4、§6、§12）、A-WS-M-T-01/A-WS-M-S-01（规则/状态机源）、A-WS-I-P-01（command 面）、A-SYS-X-L-01（PI-SYS-03 db 依赖）、A-CORE-X-R-01（A 类债务输入）
直接承接：本计划驱动 WS 实现；@GOV 追溯 BR-WS-*；SM-WS-01/02/03 转移测试矩阵
使用边界：L3 承接型 Plan Item，引用规则不定义规则；@GOV 落点在实现代码；NFR 延 G5
变更要求：Plan Item 增删保持承接可追溯；L2 契约变更回溯影响的 PI
治理成熟度：draft
---

# Binder 工作台开发计划（WS · L3）

## §1 文档定位

WS 是 P2 编码起点（文件/编辑底座）。本计划把 WS L2 规则/状态机切成 Plan Item，依赖 PI-SYS-03
（workspace.db 迁移框架）。G3 门禁：承接齐全 + 文件范围明确。

## §2 Plan Item 清单

### PI-WS-01　workspace 生命周期与文件 I/O 边界　✅ G4 路径边界守护已实现（2026-06-22，cargo test 3 covers + @GOV + audit 绿；生命周期 CRUD 待 PI-WS-05/db）
- **承接**：REQ-WS-001/002 / CHAIN WS-FS / BR-WS-DATA-001 / X-FORBID-004。
- **文件范围**：`src-tauri/src/workspace/`（生命周期 open/switch/close + 文件 I/O + 搜索 + 路径守护）；
  command 实现 `ws.open_workspace/switch/close/list_workspaces/list_docs/read_doc/create_doc/search`
  （A-WS-I-P-01 §2/§3）。
- **@GOV**：文件 I/O 边界层 `BR-WS-DATA-001`；路径守护 `X-FORBID-004`。
- **A 类债务**：缓存策略与多 workspace 控制态隔离（WS-04，实现期）。
- **验收**：越界路径被拒（X-FORBID-004 测试）；workspace 可开/切/关；I/O 经 WS 物理边界、无旁路。

### PI-WS-02　save 保存机与外部变更冲突保护　✅ G4 已实现（2026-06-22，cargo test 含原子写入/冲突路/SM-WS-01 矩阵 + 3 @GOV + audit 绿）
- **承接**：REQ-WS-002 / CHAIN WS-SAVE / SM-WS-01（含 saveConflict/overrideConfirmed）/
  BR-WS-PERSIST-001 / **BR-WS-OBS-001**（外部变更感知 + 加载基线）。
- **文件范围**：`src-tauri/src/workspace/save.rs`（原子写入 + 基线校验 + 冲突路）；文件监视
  `watcher.rs`；command `ws.save_doc`（带基线/override）+ 事件 `ws.external_change`。
- **@GOV**：save 落盘 `BR-WS-PERSIST-001`；外部变更感知/冲突 `BR-WS-OBS-001`。
- **A 类债务**：文件监视机制（FSEvents/轮询）与防抖参数（实现期，网络盘以保存前校验兜底）。
- **验收（强制，APC §6）**：SM-WS-01 转移矩阵含冲突路（saveConflict→override/reload）；save 中断
  原文件完好无半落盘；外部修改后无显式覆盖确认不可达 Saved。

### PI-WS-03　pending_diffs 持久化与裁决后落盘　✅ G4 已实现（2026-06-22，cargo test 含 SM-WS-02 矩阵/accepted 落盘记一个节点/rejected 不写 + 4 @GOV + audit 绿）
- **承接**：REQ-WS-003 / CHAIN WS-DIFF-APPLY / SM-WS-02 / BR-WS-STATE-001。
- **文件范围**：`src-tauri/src/workspace/diff_apply.rs`（持久化 + accepted 落盘 + 记节点）；
  command `ws.persist_diff/list_pending_diffs/apply_diff/discard_diff`。
- **@GOV**：落盘触发与原子性 `BR-WS-STATE-001`。
- **A 类债务**：无（接 DE 裁决信号，DE 侧 PI 提供）。
- **验收（强制）**：SM-WS-02 矩阵——accepted→Applied 记一个节点；rejected/expired 不写文件；
  applyFail 不留半节点；X-INTER-001（accepted 来自 DE、回执回传）G5 联调。

### PI-WS-04　时间轴存储与还原　✅ G4 已实现（2026-06-22，cargo test 含单调序/UNIQUE 防复用/SM-WS-03 矩阵 + 3 @GOV + audit 绿）
- **承接**：REQ-WS-004 / CHAIN WS-TIMELINE / SM-WS-03 / BR-WS-DATA-002 / BR-WS-STATE-002。
- **文件范围**：`src-tauri/src/workspace/timeline.rs`（节点准入 + 快照 + 还原）；command
  `ws.list_timeline/preview_node/restore_node`。
- **@GOV**：节点准入与单调序 `BR-WS-DATA-002`；还原一致性 `BR-WS-STATE-002`。
- **A 类债务**：**快照物理存储形态**（内联 blob vs sidecar、全量 vs 增量，WS-03，实现期选型回填
  A-WS-M-T-01 §6/§7）；save 是否记节点的准入策略（默认否，实现期确认）。
- **验收（强制）**：SM-WS-03 矩阵——还原记新节点、version_seq 单调；还原失败保持现状无半还原；
  快照缺失 guard 拒绝进入 Restoring。

### PI-WS-05　控制态持久化（承 PI-SYS-03）　✅ G4 已实现（2026-06-22，cargo test 含单活动不变量+重启恢复 + 5 @GOV + audit 绿）
- **承接**：REQ-WS-005 / BR-WS-PERSIST-002（承 BR-SYS-PERSIST-001）。
- **文件范围**：WS 侧 workspace.db 读写（workspaces/pending_diffs/timeline 表，基于 PI-SYS-03
  迁移框架）。
- **@GOV**：控制态持久化与恢复 `BR-WS-PERSIST-002`。
- **验收**：重启后未裁决 pending_diffs + 时间轴可恢复；迁移不丢控制态。

## §3 实施顺序

```
PI-SYS-03（db 框架，前置）→ PI-WS-01（生命周期/IO）→ PI-WS-05（控制态）
                                              → PI-WS-02（save/冲突）
                                              → PI-WS-03（diff 落盘，接 DE）
                                              → PI-WS-04（时间轴）
```

## §4 横切验收

`governance:audit` 绿；@GOV 追溯 BR-WS-*（不引用 REQ）；SM-WS-01/02/03 转移矩阵全覆盖；
`// covers: BR-WS-*` 覆盖 WS 全部规则（7 条，含 BR-WS-OBS-001）有实际断言。

## §5 承接的 A 类债务（A-CORE-X-R-01 §6）

| 债务 | PI | 处置 |
|---|---|---|
| 缓存策略/多 workspace 隔离（WS-04） | PI-WS-01 | 实现期 |
| 文件监视机制/防抖 | PI-WS-02 | 实现期 |
| 时间轴快照存储形态（WS-03） | PI-WS-04 | 实现期选型回填 L2 |
| I/O 延迟/恢复耗时 NFR | 全 PI | G5 标定 |

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（G3，P2 编码起点）：WS 5 条 Plan Item（生命周期与 IO 边界 / save 机含外部变更冲突 / diff 落盘 / 时间轴 / 控制态持久化），承接 4 链路 + 7 规则 + X-FORBID-004 + SM-WS-01/02/03；文件范围/@GOV/A 类债务/验收齐；依赖 PI-SYS-03 |
