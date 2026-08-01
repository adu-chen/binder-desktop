---
文档编号：VAL-X-V-02
文档状态：A
负责模块：CORE
文档职责：VAL-CORE-EDIT-DIFF-SAVE 的 G5 场景卡、自动化证据、真实环境观察项与验证裁决记录
上游约束：REQ-CORE-002 / REQ-AG-001 / REQ-DE-001~003 / REQ-ED-001、005 / REQ-WS-002、003 / APC §5.1
直接承接：.governance/test-governance.json system_validation[VAL-CORE-EDIT-DIFF-SAVE]
使用边界：只记录验证范围、执行证据和裁决；不定义业务规则，不以部分自动化替代真实系统验证
变更要求：场景、证据、环境、实际结果、缺陷或裁决变化时同步更新；未实际执行不得填写通过
---

# 核心编辑与 Diff Edit 保存验证

## 1. 验证身份与当前结论

| 字段 | 内容 |
|---|---|
| validation_id | `VAL-CORE-EDIT-DIFF-SAVE` |
| phase | G5 集成/系统验证 |
| owner / participants | CORE / AG、DE、ED、WS、SYS |
| 当前状态 | `active` |
| 自动化程度 | `partial` |
| 本轮裁决 | `IN_PROGRESS`：`AUTOMATION_GAP-01`、`AUTOMATION_GAP-03` 已在确定性自动化边界关闭，相关回归通过；真实 WKWebView、确定性 Agent 链和 provider 验证仍未闭合，不得表述为 G5 通过 |
| G5 退出条件 | §6 的 `AUTOMATION_GAP` 关闭，§4 真实环境观察完成且无阻断缺陷，相关回归与治理硬门通过 |

本卡验证的是用户可观察的完整工作流，而不是把多个测试文件的绿色结果拼成“端到端”。当前项目已有
服务编排、组件交互和 Rust 后端落盘证据，但前端自动化 mock 了 Tauri，Rust 集成没有运行真实
Agent 回合和 WKWebView 编辑器；因此这些证据只构成系统验证的可复用前置基线。

## 2. 来源与追溯

| 验证意图 | 需求来源 | 技术约束 | 主要自动化证据 |
|---|---|---|---|
| AI 建议必须形成待裁决 Diff，不能旁路写盘 | REQ-CORE-002、REQ-AG-001、REQ-DE-001 | BR-AG-STATE-002、BR-DE-DATA-001、X-FORBID-006 | `agRuntime.test.ts`、`core_chain.rs` |
| Diff 可见且接受/拒绝交互可达 | REQ-DE-002、REQ-ED-005 | BR-DE-STATE-001、BR-ED-STATE-002 | `DiffReview.test.tsx`、`deRuntime.test.ts` |
| 接受后才允许保存；拒绝后磁盘不变 | REQ-CORE-002、REQ-DE-003、REQ-WS-003 | BR-DE-STATE-001、BR-WS-STATE-001、X-INTER-001 | `deRuntime.test.ts`、`core_chain.rs` |
| 保存可靠落盘，失败不能伪装为已保存 | REQ-ED-001、REQ-WS-002 | BR-ED-STATE-001、BR-WS-PERSIST-001 | `edRuntime.test.ts` |
| 保存后关闭重开可观察到已接受内容 | REQ-ED-001、REQ-WS-002 | BR-ED-STATE-001、BR-WS-PERSIST-001、X-SEQ-002 | `src-tauri/src/lib.rs::edit_persistence_system_tests`；前端 command 信封由 `edRuntime.test.ts` 覆盖 |

辅助约束：X-FORBID-008 要求显示态、LogicalState 与 DiskState 单向受控转换；本卡分别观察 DiffCard、
编辑器内容和真实文件，不能只断言其中一层。

## 3. 统一前置条件与环境

### 3.1 自动化基线环境

- 前端：Vitest；组件用 jsdom，Service 测试 mock Tauri `invoke`，因此只证明前端编排与投影。
- 后端：Rust integration test；真实 SQLite schema 与临时文件 I/O，但不经过真实 Tauri command、
  WKWebView 或云端 Agent 回合。
- 跨层持久化系统 harness：Tauri `MockRuntime` 的 Mock Webview 发出真实 `dispatch` InvokeRequest，命中
  生产 `CommandRegistry` 和 Wry/Mock 共用路由，使用真实 SQLite 与临时文件；它证明 IPC 到 DiskState
  和第二次读取，不证明真实 WKWebView DOM、焦点或视觉反馈。
- 数据：独立测试数据与临时文件，不依赖用户 workspace，不得把测试 fixture 当作真实工作区观察结果。
- 原生工具：包内 LibreOffice 的 macOS Quartz 进程即使以 `--headless` 转换也会执行 AppKit 应用注册。
  该 E2E 与 `test:full` 必须在允许此注册的非沙箱 macOS 进程环境执行；受控沙箱内的 signal 6 只构成
  环境诊断，必须经同一测试的非沙箱对照复跑后才能裁决，不能被直接忽略。

### 3.2 真实系统环境

- macOS 上的真实 Tauri 应用与 WKWebView，使用隔离的本地 workspace。
- 已配置可用账号、有效订阅和 Agent provider；若使用确定性测试 provider，必须单独记录实现与配置。
- 初始文档内容、文件哈希和 workspace.db 快照在执行前记录；验证结束后可对照 DiskState。
- 真实环境结果不得由代码阅读、mock 测试或推断代填。

## 4. 子场景卡

### 4.1 S1 · 建议生成与接受前磁盘隔离

| 字段 | 内容 |
|---|---|
| 前置条件 | 打开 workspace 内 Markdown 文档；记录原始文件内容与哈希；编辑器为已加载状态 |
| 完整操作 | 在 Agent 编辑模式请求修改当前文档；等待建议生成并呈现 pending Diff；接受前读取真实文件 |
| 期望结果 | 建议来自当前 LogicalState；出现可裁决 Diff；DiskState 仍为原始内容；AG 无旁路写盘 |
| 自动化证据 | `agRuntime.test.ts` 的后端 projection/当前编辑器快照断言；`core_chain.rs::accepted_proposal_flows_through_de_to_ws_landing` 的 PendingDiff 持久化与裁决前状态 |
| 人类观察项 | 流式回合、Diff 实际挂载位置、接受/拒绝按钮可达；接受前用独立读取确认文件未变 |
| 自动化实际 | `PASS`：相关前端套件纳入 128/128 通过；Rust 核心链 4/4 通过。两侧为分段证据，未贯穿真实 Tauri 边界 |
| 真实环境实际 | `NOT_RUN` |
| 状态 | `PARTIAL` |

### 4.2 S2 · 接受、显式保存与关闭重开

| 字段 | 内容 |
|---|---|
| 前置条件 | S1 的 pending Diff 已应用到当前编辑器草稿，磁盘仍为原始内容 |
| 完整操作 | 点击接受；确认内容仍为待保存；执行保存；关闭文档并重新打开 |
| 期望结果 | 接受先形成 accepted/awaiting_save；保存成功后才形成 saved；重开内容等于已接受建议；时间轴/历史无重复副作用 |
| 自动化证据 | `deRuntime.test.ts` 当前编辑器 accept 编排；`edRuntime.test.ts` 保存 command 信封；`src-tauri/src/lib.rs::edit_persistence_system_tests::accepted_draft_saves_to_disk_and_reopens_through_tauri_ipc` 穿过 Mock Webview IPC、生产注册/路由、真实文件和第二次读取；`core_chain.rs` 补充时间轴约束 |
| 人类观察项 | Diff 终态、脏态/已保存标签切换、关闭重开后的正文、磁盘内容与时间轴记录 |
| 自动化实际 | `PASS`：前端保存信封与 Mock Webview IPC→accepted/awaiting_save→saved→DiskState→第二次读取可组合闭合；系统 harness 同时断言 `status=accepted`、`effect_status=saved` 和 baseline 变化 |
| 真实环境实际 | `NOT_RUN` |
| 状态 | `PARTIAL`；确定性自动化边界已闭合，真实 WKWebView 关闭/重开仍属 `VALIDATION_ONLY-01` |

### 4.3 S3 · 拒绝后恢复且磁盘不变

| 字段 | 内容 |
|---|---|
| 前置条件 | S1 的 pending Diff 已应用到当前编辑器草稿，尚未保存 |
| 完整操作 | 点击拒绝；检查编辑器内容；关闭并重开文档；读取真实文件 |
| 期望结果 | Diff 进入 rejected 且不可逆；草稿恢复至建议前 LogicalState；文件和时间轴均不发生该建议的落盘副作用 |
| 自动化证据 | `deRuntime.test.ts` 的块级/整篇/样式/图片 LogicalState 回滚；`src-tauri/src/lib.rs::edit_persistence_system_tests::rejected_diff_keeps_disk_and_reopen_unchanged_through_tauri_ipc` 穿过 Mock Webview IPC 并断言文件、重开 baseline 不变且无时间轴节点；`core_chain.rs::rejected_proposal_does_not_land` |
| 人类观察项 | 拒绝按钮、内容回滚、终态呈现复位、关闭重开后的正文和真实文件 |
| 自动化实际 | `PASS`：前端 LogicalState 回滚证据与 Mock Webview rejection→unchanged DiskState→第二次读取→无时间轴节点按稳定 command 契约可组合闭合 |
| 真实环境实际 | `NOT_RUN` |
| 状态 | `PARTIAL` |

### 4.4 S4 · 保存即接受与保存失败

| 字段 | 内容 |
|---|---|
| 前置条件 | 当前编辑器已有已应用但仍 pending 的 Diff |
| 完整操作 | 不先点接受，直接保存；另一次制造外部文件变更后保存，触发保存冲突 |
| 期望结果 | 直接保存先持久化 accepted 再写盘；成功后才显示 saved；失败时保持 accepted/awaiting_save 与 dirty，不伪造已保存，也不覆盖外部内容 |
| 自动化证据 | `edRuntime.test.ts` 的“保存即接受”“保存失败后保留已接受、待保存”“保存冲突保持 dirty” |
| 人类观察项 | 保存动作后的 Diff/标签状态、冲突提示、外部文件未被静默覆盖、关闭保护仍出现 |
| 自动化实际 | `PASS`：保存即接受、保存失败保留待保存、保存冲突保持 dirty 的自动化断言通过 |
| 真实环境实际 | `NOT_RUN` |
| 状态 | `PARTIAL`；真实外部变更竞态尚未执行 |

### 4.5 S5 · Diff 审阅控件与终态可观察性

| 字段 | 内容 |
|---|---|
| 前置条件 | 分别准备 pending、conflict、accepted-awaiting_save、accepted-saved、rejected 状态 |
| 完整操作 | 查看 DiffCard，尝试可用/禁用动作并观察状态变化 |
| 期望结果 | pending 可接受/拒绝；conflict 禁止接受；accepted 区分待保存与已保存；rejected 不再可裁决；控件不会制造后端不存在的终态 |
| 自动化证据 | `DiffReview.test.tsx` 的状态渲染与控件断言；`deRuntime.test.ts` 的后端/WS 状态投影不可回退断言 |
| 人类观察项 | 真实 WKWebView 中按钮可点击、焦点/反馈清晰、状态切换无残留卡片或错误动作 |
| 自动化实际 | `PASS`：DiffReview 27/27 通过，deRuntime 状态投影断言通过；真实 WKWebView 交互仍未执行 |
| 真实环境实际 | `NOT_RUN` |
| 状态 | `PARTIAL` |

## 5. 本轮自动化执行记录

| 时间 | 命令/范围 | 结果 | 证明边界 |
|---|---|---|---|
| 2026-08-01 | `npx vitest run`：`agRuntime + deRuntime + edRuntime + DiffReview` | `PASS`：4 files，128 tests | 前端投影、编排与组件交互；Tauri 边界被 mock |
| 2026-08-01 | `cargo test --manifest-path src-tauri/Cargo.toml --test core_chain` | `PASS`：4 passed，0 failed/ignored | DE→WS 的真实 DB/文件副作用；不含前端、真实 Agent、重开 UI |
| 2026-08-01 | `npm run test:governance` | `PASS`：8 tests；84 assets / 724 tests；0 finding / warning | 证据路径、owner/layer 与测试治理完整性；不证明产品行为 |
| 2026-08-01 | `npm run governance:generate` / `npm run governance:audit` | `PASS`：ADUS 刷新；audit status=passed，test governance 0 finding / warning | 注册表到 ADUS 的派生一致性与治理硬门；不证明产品行为 |
| 2026-08-01 | 受控沙箱内 `npm run test:full` / 单独复跑 LibreOffice E2E | `FAIL`：前端 21 files / 455 tests 全过；Rust 在真转换处 `DOC_PREVIEW_WORKER_CRASH` signal 6 | 原始阻断证据；崩溃报告定位 `NSApplication → GetCurrentProcess → _RegisterApplication`，需要环境对照而非弱化测试 |
| 2026-08-01 | 非沙箱 macOS 进程内单独复跑 LibreOffice E2E | `PASS`：1 passed，生成 PDF；Font Trace 72 refs | 同一代码、二进制和输入在允许 AppKit 注册的环境通过，裁决 signal 6 为沙箱执行环境假失败 |
| 2026-08-01 | 非沙箱 macOS 进程内 `npm run test:full` | `PASS`：前端 21 files / 455 tests；Rust lib 261/261；`core_chain` 4/4；`sm_broadcast` 4/4 | G5 自动化回归基线通过；不替代真实 Tauri/WKWebView 系统场景 |
| 2026-08-01 | `cargo test --manifest-path src-tauri/Cargo.toml edit_persistence_system_tests -- --nocapture` | `PASS`：2 passed | Tauri Mock Webview IPC→生产注册/共享路由→真实 SQLite/文件→第二次读取；不含真实 WKWebView UI |
| 2026-08-01 | 非沙箱 macOS 进程内 `npm run test:related -- --files src-tauri/src/lib.rs` | `PASS`：前端 21 files / 457 tests；Rust lib 263/263；`core_chain` 4/4；`sm_broadcast` 4/4；audit passed | 证明选择器能对 Rust 内联生产源触发完整相关回归；同时验证新增系统 harness |
| 2026-08-01 | 非沙箱 macOS 进程内最终 `npm run test:full` | `PASS`：前端 21 files / 457 tests；Rust lib 263/263；`core_chain` 4/4；`sm_broadcast` 4/4 | 本次治理闭合后的完整自动化基线；不替代真实 WKWebView 与 provider 验证 |

## 6. 缺口、责任与退出条件

| 编号 | 分类 | 缺口 | owner | 目标 | 退出条件 | 状态 |
|---|---|---|---|---|---|---|
| AUTOMATION_GAP-01 | system harness | 前端 command 信封与 Tauri Mock Webview→生产注册/路由→真实文件→第二次读取已按稳定契约组合；真实 UI 单列验证 | ED / DE / WS / SYS | G5 | Mock Webview InvokeRequest 命中生产注册/共享路由，断言 accepted/saved、DiskState 和第二次读取；真实 WKWebView 留在 VALIDATION_ONLY-01 | resolved |
| AUTOMATION_GAP-02 | Agent integration | `agRuntime` 验证 projection，但没有真实 Agent 回合→工具→DE→前端 DiffCard 的确定性集成链 | AG / DE / SYS | G5 | 建立可重复的确定性 Agent 测试供给并跑通建议生成，不依赖不稳定外部模型输出 | open |
| AUTOMATION_GAP-03 | rejection integration | 前端 LogicalState 回滚与 Mock Webview rejection→DiskState/重开不变→无时间轴节点按稳定 command 契约组合 | DE / ED / WS | G5 | 两侧契约和后端真实边界均有确定性断言；真实 UI 观察留在 VALIDATION_ONLY-01 | resolved |
| VALIDATION_ONLY-01 | WKWebView interaction | Diff 挂载位置、真实焦点/按钮交互、状态反馈需要真实应用观察 | ED / DE | G5 | 在 §3.2 环境执行 S1–S5，记录实际结果或缺陷 | open |
| VALIDATION_ONLY-02 | real provider | 真实账号、订阅、网关与模型输出属于更大的 AI 系统链，当前证据不能覆盖 | AG / ENT / CLOUD | G5/G6 | 对接相应系统验证卡；本卡只消费其“可生成确定性修改建议”前置结论 | blocked_by_cloud |
| VAL-G5-REG-001 | regression environment | 受控沙箱禁止 macOS AppKit 应用注册，导致 DOCX→PDF 真转换 worker signal 6；同一测试在非沙箱环境通过 | DOC / PRV / CORE | G5 | 非沙箱定向真转换与 `test:full` 通过，并登记执行环境约束 | resolved |

`AUTOMATION_GAP-01`、`AUTOMATION_GAP-03` 已关闭。下一项是以确定性 Agent 供给关闭
`AUTOMATION_GAP-02`；真实 provider 不应成为本地编辑/Diff/保存链自动化的前置依赖。

## 7. 缺陷与最终裁决

| defect | scenario | severity | status | evidence |
|---|---|---|---|---|
| VAL-G5-REG-001 | G5 回归基线 | blocker | resolved | 沙箱内 signal 6 与崩溃栈已保留；非沙箱同一 E2E 1/1、全量前端 455/455、Rust 261+4+4 全过；未修改或弱化测试 |
| 真实系统验证未执行 | S1–S5 | — | not_applicable | 未执行不等于无缺陷 |

最终 G5 裁决：`NOT_READY`。`VAL-G5-REG-001` 已解除，`AUTOMATION_GAP-01`、`AUTOMATION_GAP-03`
也已在确定性边界关闭；但只有 §4 的真实系统实际结果完成、§6 的其余
G5 缺口关闭，且自动化回归、治理审计与相关 NFR 均通过后，才可将注册表状态改为 `passed`。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-01 | v1.0 | VAL-CORE-EDIT-DIFF-SAVE-01：建立首个 G5 场景卡，拆分五类核心子场景、自动化证据、真实环境观察项与退出条件；定向证据通过，但全量回归发现 VAL-G5-REG-001，当前结论为 BLOCKED/NOT_READY |
| 2026-08-01 | v1.1 | VAL-G5-REG-001：保留沙箱内 signal 6 原始失败，非沙箱复跑同一 LibreOffice E2E 与 test:full 全过，确认 AppKit 执行环境限制并解除回归阻断；真实系统验证缺口仍保持 NOT_READY |
| 2026-08-01 | v1.2 | VAL-CORE-EDIT-DIFF-SAVE-AUTOMATION-01：新增 Tauri Mock Webview→生产注册/共享路由→真实文件→第二次读取系统 harness，关闭 AUTOMATION_GAP-01/03 的确定性自动化部分；真实 WKWebView 与 Agent/provider 缺口保持 NOT_READY |
