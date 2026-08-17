# Issue Trace — AG-1.0-IMP-AGENT-CREATE-SAVE-01：代理新建保存才建文件（D-NEW）

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-14

## 1. 用户可见症状

冻结基线 R6（D-NEW-001~008 / REQ-WS-006）要求：**代理新建**（Agent 建新文档）经 propose_edit base=empty
提案生成 LogicalState 预览，保存动作才创建物理文件（V-N1 预览阶段文件树/磁盘无该文件、V-N2 保存后才出现、
V-N4 目标被外部创建不覆盖）。现状：代理新建已走 base=empty 提案链（tool_registry 不授予 create_doc 直写，
context.rs 走 CreateDocument 提案），保存经 `ws.save_doc`（save_atomic 对不存在目标在**父目录存在**时可创建）；
缺口：① 父目录不存在时保存失败（WS_IO_ERROR，无 create_dir_all）；② 目标被外部创建时保存会覆盖（V-N4
缺失，baseline=null 跳过守卫）；③ 代理新建 docx 保存后未登记 BDocx 身份。
**手动新建（FileTree「＋」/右键）保持现状不变**（立即 ws.create_doc 建文件），不在此 Scope。

## 2. 入口点

- `src-tauri/src/lib.rs` `route_save_doc`（:787-838）：save_atomic_sm 前无创建语义分支；
  `save_with_baseline_guard`（:2027-2054）对 baseline=null 跳过守卫（V-N4 缺口）；
  `save_atomic`（workspace/save.rs:60-80）无 create_dir_all（父目录缺失即失败）。
- 代理新建链路（现状正确，不修改）：tool_registry/context.rs CreateDocument → propose_edit base=empty
  提案 → 前端保存走 saveDoc（baseline=null）。

## 3. 相关规则

- `BR-WS-PERSIST-001`（原子落盘，无半落盘副作用）
- `BR-WS-OBS-001`（保存前外部变更守卫，冲突不静默覆盖）
- `BR-WS-DATA-001`（workspace 物理边界）
- 契约：A-WS-M-S-01 §2.1（未保存新文档保存＝创建：校验→原子创建→记时间轴节点→建立 DocRef；失败保留
  未保存会话不伪报已创建）；D-NEW-001~008 / REQ-WS-006（冻结，R6）

## 4. 相关链路

WS-SAVE（保存落盘）、AG-TURN（代理新建提案）、DE-DIFF（base=empty 提案裁决）。

## 5. 生产者块

- `route_save_doc` 既有 @GOV（BR-WS-OBS-001-RB-WS-WS-FS-001 上游）——本实现新增创建语义分支 @GOV。

## 6. 消费者块

- 代理新建提案保存（edService.saveDocument 系，baseline=null）；V-N1~N4 验证卡执行。

## 7. 路径上的空规则块

route_save_doc 上游已标注；创建分支补 @GOV（BR-WS-PERSIST-001 / BR-WS-DATA-001）与 covers。

## 8. boundary 假设

- 未保存新文档身份 = `baseline == null`（无磁盘基线）；目标不存在 → 创建；目标存在 → WS_EXISTS（V-N4）。
- 手动新建（createDoc 立即建文件）走 ws.create_doc，不经过本创建分支（文件存在 + baseline 非 null）。
- 创建语义：create_dir_all 父目录 → save_atomic 原子落盘 → 时间轴节点（既有 record_save_node）→ docx 补
  mark_bdocx 身份；失败保留未保存会话（save 链失败前端回 dirty，不伪报已创建）。
- 不新增 RULE（D-NEW 契约 D2 已登记）；不触碰前端（手动新建逻辑不变）。

## 9. 运行时假设

- baseline=null 且目标存在仅出现于"外部创建介入"（正常文档保存 baseline 非 null；代理新建保存目标不存在）；
  故 WS_EXISTS 冲突不破坏正常保存路径。

## 10. 疑似不变量破坏

- BR-WS-OBS-001 守卫不破坏（baseline 非 null 路径不变）；baseline=null + 目标存在从"静默覆盖"改为
  "WS_EXISTS 冲突"（V-N4 要求，现有测试均传非 null baseline，无回归）。
- SM-WS-01 保存机不变；时间轴节点恰一个（V-N2）。

## 11. 验证计划

- 补 lib.rs 内联测试：目标不存在+父目录缺失 → 创建成功（父目录+文件+时间轴节点）；baseline=null+目标
  存在 → WS_EXISTS 不覆盖。
- `npm run test:plan -- --files [changed_files]` 审阅影响，再 `npm run test:related`；`npx tsc --noEmit`
  （前端零改动仍跑）；`cargo test`；`npm run governance:audit`。
- 前端确认：现有 workspaceRuntime/edState 测试不破坏（手动新建未动）。

## 12. 文件范围结论（Scope Lock 输入）

允许修改：
- `src-tauri/src/lib.rs`（route_save_doc 创建语义分支 + @GOV + 测试）
- `docs/01_modules/WS/A-WS-M-S-01_工作台状态机设计.md`（变更记录注明落地）
- `.governance/issues/AG-1.0-IMP-AGENT-CREATE-SAVE-01.md`、`.governance/active_scope.json`

禁止修改：前端任何文件（手动新建逻辑不变）；src-tauri 其余文件（workspace/save.rs、listing.rs、
fileops.rs 等只读复用）；docs/00_core/**（ADUS 由 generate 更新除外）；治理控制文档；governance.config.json。

## 13. 人类确认记录

2026-08-14：人类明确指示"不要改变当前手动新建文件的逻辑，保存才建文件只对代理新建生效"——Scope
聚焦 route_save_doc 创建语义（仅代理新建 baseline=null 路径），手动新建（ws.create_doc 立即建文件）
保持现状。confirmation: YES。

## 14. Task Runtime Report

- task_type：`CODE_CHANGE`
- governance_phase：`IMPLEMENTATION`
- final_state：`READY`
- issue_trace：本文件
- changed_files：
  - `src-tauri/src/lib.rs`（route_save_doc 创建语义分支：baseline=null 目标不存在 → create_dir_all 父目录
    → save_atomic_sm 原子落盘 → 时间轴节点 → docx mark_bdocx 身份；目标已存在 → WS_EXISTS 冲突不覆盖；
    @GOV BR-WS-PERSIST-001-EFFECT-WS-WS-SAVE-003；2 个新测试）
  - `docs/01_modules/WS/A-WS-M-S-01_工作台状态机设计.md`（§2.1 契约落地变更记录 v1.4）
  - `docs/00_core/A-CORE-C-R-01_ADUS.md`（generate 更新：@GOV 256→257、测试用例 908→910）
  - 本 Issue Trace、`.governance/active_scope.json`
- mapping_chain：D-NEW（REQ-WS-006 冻结，R6）→ route_save_doc 创建语义（BR-WS-PERSIST-001/OBS-001/
  DATA-001 @GOV + covers）；手动新建路径未触碰（用户指示）。
- test_impact：
  - changed_files：lib.rs（前端零改动）
  - direct_modules：WS、SYS
  - selection：test:plan 判定前端全量 + tsc + cargo 全量 + audit
- validation：
  - `npm run test:related`（npm test 前端全量） # passed：26 files / 574 tests（前端零改动确认不回归）
  - `npx tsc --noEmit` # passed
  - `cargo test`（Rust 全量） # passed：336 tests（含 agent_create_save 2 新用例）
  - `npm run governance:generate` # passed（@GOV 256→257；测试用例 908→910）
  - `npm run governance:audit` # **passed（exit 0）**
  - `git diff --check` # passed
- automated_tests：前端 574 + Rust 336 全过。
- stop_conditions：无。
- unresolved_items：V-N1/V-N3 端到端人工验证留待 VALIDATION 阶段；代理新建提案在关闭/重启流程的会话
  恢复细节（无磁盘文件，重启自然消失）由后续迭代。
