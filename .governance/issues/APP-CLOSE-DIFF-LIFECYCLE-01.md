# Issue Trace · APP-CLOSE-DIFF-LIFECYCLE-01 应用关闭时 diff/未保存内容生命周期终态化

承接 `REQ-CORE-002`、新增 `REQ-SYS-007`、`BR-SYS-STATE-004`、`BR-SYS-STATE-005`、`X-FORBID-010`、
`X-FORBID-011` 与人类三轮确认口径：正常关闭前所有 pending diff 与未保存文档必须经统一弹窗完成
决策并落终态才允许退出；异常关闭（无法弹窗）后下次启动执行清算——未保存编辑一律丢弃并重载磁盘
内容，未裁决 diff 一律转 expired（reason=abnormal_exit），全部已接受但未确认落盘的 diff 标记效果丢失；
"接受全部"与"保存"强制耦合（不允许接受但不保存）；提供人类确认的强制退出兜底出口；新增
`pending_diffs.effect_status`/`terminal_reason` 两列并对存量数据一次性回填。

## 1. 用户可见症状

当前点击窗口关闭按钮只 flush 编辑器标签快照，不检查未裁决 diff、不检查非激活文档的未保存修改，
不弹窗确认；Cmd+Q / Dock 退出 / 系统关机完全不经过任何持久化钩子。重启后待裁决 diff 被当作可继续
裁决的 pending 卡片恢复，未保存的脏内容被当作正常草稿恢复，用户看不出上次是否发生过异常退出，
已接受但未真正落盘的修改在历史中无法区分"已生效"与"已丢失"。

## 2. 入口点

- 用户点击窗口关闭按钮：`src/shell/App.tsx` `appWindow.onCloseRequested`
- 用户按 Cmd+Q / 菜单栏退出 / Dock 退出 / 系统关机：macOS 通过原生
  `applicationShouldTerminate:` 拦截并发出 `sys:exit-requested`，其余退出事件走 Rust
  `RunEvent::ExitRequested`，均收敛至前端 `requestClose()`
- 应用启动：`src-tauri/src/lib.rs` `run().setup()`（新增异常退出清算，先于 workspace 恢复执行）
- 用户在关闭前弹窗中选择处理方式：新增 `src/components/shell/CloseConfirmDialog.tsx`
- 用户在 workspace 菜单中切换/关闭 workspace 的既有脏态守卫：`src/components/shell/WorkspaceMenu.tsx`
  `hasUnsaved()`（本次顺带修复其只查激活文档的漏检）

## 3. 相关规则

- `BR-SYS-STATE-004`（新增，正常关闭护栏）
- `BR-SYS-STATE-005`（新增，异常恢复护栏）
- `BR-DE-STATE-001`（终态互斥不可逆，本次不违反，只新增启动期转移源）
- `BR-DE-STATE-002`（过期语义，复用 `expire_diffs` 扩展 `terminal_reason`）
- `BR-DE-DATA-002`（payload 载荷；新增 `effect_status`/`terminal_reason` 字段登记）
- `BR-ED-STATE-001`（脏态不得静默丢失；本次新增"异常退出后清算优先于装载"边界，不违反脏态守护本身）
- `X-FORBID-008`（三态单向数据流，本次不新增例外）
- `X-FORBID-009`（LogicalCandidate 转正边界，本次不新增例外）
- `X-FORBID-010`（新增）
- `X-FORBID-011`（新增）

## 4. 相关链路

新增 `APP-CLOSE`（owner SYS）；跨接 `DE-DIFF`、`ED-EDIT`、`WS-SAVE`、`AG-TURN`、`IPC-CALL`、`DATA-MIGRATE`。

## 5. 生产者块（拟新增/修改，实现期落位）

- `BR-SYS-STATE-004-EFFECT-SYS-APP-CLOSE-001`：`src/services/closeService.ts`（关闭编排主流程）
- `BR-SYS-STATE-005-EFFECT-SYS-DATA-MIGRATE-002`：`src-tauri/src/de/lifecycle.rs`（异常退出清算扩展）
  与新增清算调用点 `src-tauri/src/lib.rs` `setup()`
- `BR-SYS-STATE-004-IO-SYS-IPC-CALL-00X`：新增只读查询 `sys.inspect_close`（pending 数 + awaiting_save
  数 + dirty 文档
  列表）；批量裁决/保存复用既有 `de.accept_diff`/`de.accept_draft_diff`/`de.reject_diff`/
  `ws.save_doc`/`doc.save_from_edit`（不新增裁决/保存命令，避免重复定义已有转移路径）；新增
  `ws.discard_unsaved_sessions`（放弃未保存修改分支的显式清算，`src-tauri/src/lib.rs`）
- `BR-DE-DATA-002-EFFECT-DE-DE-DIFF-00X`：`effect_status` 迁移与写入点（`src-tauri/src/db/mod.rs`
  迁移 v10、`src-tauri/src/workspace/diff_apply.rs`、accept/save 路径）

## 6. 消费者块

- `src/store/closeStore.ts`（新增）消费 `SM-SYS-02` 投影，驱动 `CloseConfirmDialog`
- `src/shell/App.tsx` 消费统一关闭入口，替换现有仅 flush 的 `onCloseRequested`
- `src/components/shell/WorkspaceMenu.tsx` 消费本次新增的聚合脏态/pending 判定服务
- `src/services/deService.ts` `loadPersistedDiffs` 消费清算后的只读终态数据（不得复活可裁决 pending）
- `src/services/agService.ts` 历史 hydrate 路径消费终态 diff 引用（只读）

## 7. 路径上的空规则块

不预期新增无法追溯的游离代码块；`sys.inspect_close`/`ws.discard_unsaved_sessions` 属新 IO/EFFECT，
落地时必须补 `@GOV`；迁移 SQL 与清算函数按 ADU §5.6 补 `// covers:`。

## 8. boundary 假设

- 单窗口假设成立：当前应用为单窗口单活动 workspace 架构（`App.tsx` 单一 `getCurrentWindow()`），
  本设计不处理多窗口聚合场景；若未来引入多窗口需重新评估关闭屏障范围。
- `de::lifecycle::sweep_expiry`/`expire_diffs`/`list_pending` 现有实现的过期终态语义（不写文件、
  不记时间轴节点）在异常退出清算场景下继续成立，只扩展 `terminal_reason` 参数，不改变其副作用边界。
- `pending`、`awaiting_save` 或 `ed_session_snapshots.dirty=1` 的跨会话残留即须清算，不新增独立
  "洁净关闭标记"表；`awaiting_save` 不能依赖 dirty 快照作为保留证据（编辑快照的防抖写入可被异常中断）。

## 9. 运行时假设

- 关闭屏障建立后拒绝新的 `AgentTurn.send`/`de.generate_diff`；正在 `Streaming` 的回合发 `cancel`
  （复用 `SM-AG-01` 既有事件，不新增 AG 内部状态）。
- `sys.resolve_close` 执行期间弹窗进入处理中态，禁用重复提交；完成后必须重新查询数据库真值
  （不信任发起决策时刻的内存计数）才允许放行关闭。
- 强制退出（人类决策 3）是用户在弹窗内显式触发的兜底动作，仅在常规解决路径反复失败或明确需要时
  出现，不作为默认或自动路径；触发后允许残留未终态数据，不视为违反 `BR-SYS-STATE-004`（该规则约束
  "常规关闭路径"，强制退出是用户显式豁免后的独立出口，等价于下次启动按异常退出处理）。
- 存量数据回填（人类决策 4）：本次迁移上线前已存在的 `pending_diffs` 行中，`status IN
  ('pending','rejected','expired')` 回填 `effect_status='not_applicable'`；`status='accepted'`
  回填 `effect_status='saved'`（历史上 accepted 只有 background 路径同步落盘或用户已继续正常使用
  应用多次保存，无法逐行区分是否仍 awaiting_save，保守假设为已完成，避免历史数据被误标为"效果丢失"
  造成误导；此假设记入本节，供后续如发现历史数据展示异常时回查）。
- 批量裁决/保存的执行方是前端 `closeService.ts` 编排，逐条调用既有 `de.accept_diff`/
  `de.accept_draft_diff`/`de.reject_diff`/`ws.save_doc`/`doc.save_from_edit`（与既有 SM-DE-01/
  SM-ED-01/SM-WS-01 转移完全复用，不新增裁决/保存类命令）；"放弃未保存修改"分支因不涉及逐文档
  差异化内容，走新增的单一后端命令 `ws.discard_unsaved_sessions`（清空该 workspace 全部
  `ed_session_snapshots.dirty=1` 行，并无条件终结该 workspace 全部 awaiting_save 效果；与异常退出
  清算共享同一丢弃语义但由用户显式触发、非启动期）。

## 10. 疑似不变量破坏

- 若清算时只处理 `pending_diffs` 而不处理 `ed_session_snapshots.dirty=1`，未保存内容会在只读历史
  之外继续被当作可信 LogicalState 恢复，违反决策 1；本次两者必须同一清算流程内联动处理。
- 若正常关闭弹窗允许"接受但不保存"，会重现原审计发现的"已接受但未落盘"历史不一致；本次强制耦合
  （人类决策 2 已确认）杜绝该组合。
- 若 `sys.resolve_close` 成功回执基于请求发起时刻的计数而非重新查询，存在"清点时 3 条、处理中又新
  增 1 条"的竞态放行风险；本次 `Verifying` 态必须重新查询。
- 若强制退出兜底做成自动触发（如超时自动强退），会违反"用户显式确认"原则，蜕变为静默丢数据；本次
  必须是弹窗内的显式按钮。

## 11. 验证计划

- Rust：迁移 v10 幂等与回填正确性；`de::lifecycle` 扩展后 `terminal_reason` 落库正确；异常退出清算
  函数端到端（模拟遗留 pending + dirty 快照，以及**无 dirty 快照**的 awaiting_save → 清算后
  pending=0、dirty 快照被清、全部 `awaiting_save` 正确翻转）；macOS `applicationShouldTerminate:` /
  `RunEvent::ExitRequested` 与窗口 `CloseRequested` 收敛到同一入口（手动/集成验证）；
  `effect_status` 在 accept/save 路径的翻转（background 立即 saved；current_editor/open_session
  路径 awaiting_save→saved）。
- 前端：`closeStore`/`closeService` 状态转移测试（含取消、失败重试、强制退出）；`CloseConfirmDialog`
  UI 测试（接受全部强制耦合保存、详情展开、处理中禁用）；`WorkspaceMenu` 聚合脏态修复回归测试；
  `deService.loadPersistedDiffs` 不再返回可裁决 pending（清算后应恒为空或仅只读终态）。
- 命令：`cargo test --manifest-path src-tauri/Cargo.toml`、`npm test -- --run`、
  `./node_modules/.bin/tsc --noEmit`、`npm run governance:generate`、`npm run governance:audit`。
- AI 语义审计：检查 `BR-SYS-STATE-004`/`BR-SYS-STATE-005` 对应代码是否真实满足"关闭前清零、异常后
  清算、不复活 pending"边界，而非仅注释声明。

## 12. 本轮运行记录（2026-07-21）

- 修复：启动期对每个 workspace 无条件清算 pending 与 awaiting_save；任何 workspace 清算失败令
  `setup()` 失败，防止其 pending 被恢复为可执行卡片。
- 修复：普通关闭在 `sys.inspect_close` 前同步 flush 编辑器快照，并将 `awaitingSaveCount` 纳入检查与
  复核 guard；flush 失败停留决策入口，不以旧快照放行。
- 验证通过：`cargo test --manifest-path src-tauri/Cargo.toml de::lifecycle::tests --lib`（7）；
  `npx vitest run src/services/closeService.test.ts src/components/shell/CloseConfirmDialog.test.tsx src/shell/App.test.tsx`
  （26）；`npm test -- --run`（40 文件、305 测试）；`npm run build`；
  `cargo build --manifest-path src-tauri/Cargo.toml`；`npm run governance:audit`。
- 非本范围环境项：完整 `cargo test --manifest-path src-tauri/Cargo.toml --lib` 为 196 通过、1 失败；失败项
  `doc::libreoffice_runtime::tests::docx_to_pdf_end_to_end_when_libreoffice_available` 返回既有
  `DOC_PREVIEW_FAIL`（LibreOffice 转换无消息），与 APP-CLOSE 链路无共享代码。
- 人工验证已通过（人类于 2026-07-21 确认）：最新原生 macOS 构建的 Cmd+Q 与 Dock 退出均进入统一
  关闭决策框；取消后应用不退出。APP-CLOSE 的原生退出入口验证闭合。

## Scope Lock

```json
{
  "task_type": "CODE_CHANGE",
  "scope_id": "APP-CLOSE-DIFF-LIFECYCLE-01",
  "allowed_files": [
    ".governance/active_scope.json",
    ".governance/issues/APP-CLOSE-DIFF-LIFECYCLE-01.md",
    "docs/00_core/A-CORE-C-T-01_三态模型.md",
    "docs/01_modules/SYS/A-SYS-M-D-01_系统底座功能需求描述.md",
    "docs/01_modules/SYS/A-SYS-M-S-01_状态机总图.md",
    "docs/01_modules/SYS/A-SYS-M-T-02_应用关闭技术设计.md",
    "docs/01_modules/SYS/A-SYS-M-S-02_应用关闭状态机设计.md",
    "docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md",
    "docs/01_modules/SYS/A-SYS-I-P-02_IPC协议.md",
    "docs/01_modules/DE/A-DE-M-S-01_DiffEdit状态机设计.md",
    "docs/01_modules/DE/A-DE-M-T-01_DiffEdit技术设计.md",
    "docs/01_modules/ED/A-ED-M-S-01_编辑器状态机设计.md",
    "docs/00_core/A-CORE-C-R-01_ADUS.md",
    "src-tauri/src/db/mod.rs",
    "src-tauri/src/de/lifecycle.rs",
    "src-tauri/src/de/store.rs",
    "src-tauri/src/de/expiry.rs",
    "src-tauri/src/workspace/diff_apply.rs",
    "src-tauri/src/workspace/mod.rs",
    "src-tauri/src/lib.rs",
    "src/shell/App.tsx",
    "src/services/workspaceService.ts",
    "src/services/closeService.ts",
    "src/services/closeService.test.ts",
    "src/services/deService.ts",
    "src/services/deService.test.ts",
    "src/services/agService.ts",
    "src/store/closeStore.ts",
    "src/store/closeStore.test.ts",
    "src/store/deStore.ts",
    "src/store/edStore.ts",
    "src/store/tabsStore.ts",
    "src/components/shell/CloseConfirmDialog.tsx",
    "src/components/shell/CloseConfirmDialog.test.tsx",
    "src/components/shell/WorkspaceMenu.tsx",
    "src/components/shell/WorkspaceMenu.test.tsx",
    "src/styles.css"
  ],
  "forbidden_files": [
    "ADU.md",
    "ADP.md",
    "AGENTS.md",
    "CLAUDE.md",
    "docs/00_core/A-CORE-C-P-01_APC.md"
  ],
  "confirmation": "YES",
  "scope_reason": "APP-CLOSE-DIFF-LIFECYCLE-01（人类三轮确认）：应用关闭前 diff/未保存内容必须终态化处理，异常退出后启动清算，新建 SYS 应用关闭状态机文档，新增 pending_diffs 效果态字段与存量回填，含强制退出兜底出口。"
}
```
