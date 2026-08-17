# Issue Trace — AG-1.0-IMP-EXTERNAL-CONTENT-01：外部内容 data-only（ExternalContentRef）实现

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-14

## 1. 用户可见症状

冻结基线 R5-C（BR-SYS-DATA-007 / TERM-REF-005）要求：外部文件内容仅在用户显式附加且存在可用只读解析
路径时，作为当前任务的一次性 data-only 内容引用；不支持/不可读只保留元数据不伪称已读；不扫描目录、
不自动复读、不导入、不编辑、不默认持久化；任务结束/重启/跨会话/跨 workspace 不自动恢复或转存。
当前实现：file 引用只有元数据（name/path/size），无任何内容读取路径；后端格式化固定输出"仅文件引用，
未读取内容"。BR-SYS-DATA-007 无任何代码 @GOV 引用或测试 covers（audit 唯一剩余缺口）。

## 2. 入口点

- `src/services/referencePayload.ts:55`（file 变体仅元数据）、`:259-270`（coerce file 分支）
- `src/components/ag/Composer.tsx:203-209`（fileReferenceFromTransfer 仅取 name/size，注释"外部文件不读内容"）
- `src-tauri/src/types/mod.rs:90-145`（ReferencePayload::File 定义）
- `src-tauri/src/reference/context.rs:211-218`（File 分支固定"未读取内容"文案）
- 既有注入基座：ag_start_turn references → format_reference_payload_context → mention_block（一次性，天然 data-only）

## 3. 相关规则

- `BR-SYS-DATA-007`（外部内容/记忆隔离：显式附加、只读解析、一次性 data-only、不自动转存；D1 注册，
  A-SYS-I-P-01 §3.1；D3 契约 A-REF-M-T-01 §6.1）
- 间接：BR-AG-DATA-001（引用上下文组装）、X-FORBID-004（workspace 限界——只读解析不越界写）

## 4. 相关链路

REF-INJECT（引用注入）、AG-TURN（回合消费）、IPC-CALL（ag_start_turn payload）。

## 5. 生产者块

- `reference/context.rs` 既有 @GOV（doc/selection 分支）——本实现新增 File 分支 @GOV（BR-SYS-DATA-007，
  term_ref: TERM-REF-005）。
- `referencePayload.ts` / `Composer.tsx` 前端引用组装。

## 6. 消费者块

- AG prompt 注入（"用户显式引用"块）；DiffCard/引用 chip 呈现。
- V-S8 验证卡（外部内容一次性引用数据面）。

## 7. 路径上的空规则块

本 Scope 改动文件均有 @GOV 标注基础；新增 File 分支补 @GOV 引用 BR-SYS-DATA-007（消除 audit 唯一缺口）。

## 8. boundary 假设

- 只读解析：前端 Web File API `file.text()`（浏览器只读读取），不进 WS 文件系统、不扫描目录。
- 可读判定：复用 Composer 既有 READABLE_DOC_EXT 白名单扩展名；不可读保持现状（元数据 + "未读取内容"）。
- data-only：readContent 只在当前回合 payload 传递（mention_block 一次性），不落盘、不写 memory_entries、
  不转存会话资产；回合结束随请求消逝。
- 不伪称已读：无 readContent 的 file 引用后端输出保留"仅文件引用，未读取内容"。

## 9. 运行时假设

- 内容容量受控（cap 截断对齐 memory 分支），不注入无界文本。
- 路径字段（path）仍不随引用携带绝对路径（X-FORBID-004 精神，file.text() 仅读取所选 File 对象）。

## 10. 疑似不变量破坏

- ReferencePayload 结构扩展（File 增可选字段）向后兼容（Option），既有 doc/selection/url 等不变。
- X-FORBID-004 workspace 限界不破坏（只读解析外部所选文件，不扫描、不写盘）。
- 不触碰 BR-SYS-DATA-007 的记忆半（显式记忆选择另立 Scope）。

## 11. 验证计划

- 补测试：referencePayload.test.ts（file coerce 带/不带 readContent）、context.rs（File 分支分叉）、
  Composer（可读/不可读扩展名）。
- 运行 `npm run test:plan -- --files [changed_files]` 审阅影响，再 `npm run test:related`；
  `npx tsc --noEmit`；`cargo test`；`npm run governance:audit`（预期 BR-SYS-DATA-007 进入引用，audit
  缺口归零）。
- 同步 `scripts/test_governance.test.mjs`（测试计数基线，如需）。

## 12. 文件范围结论（Scope Lock 输入）

允许修改：
- `src/services/referencePayload.ts`
- `src/components/ag/Composer.tsx`
- `src-tauri/src/types/mod.rs`
- `src-tauri/src/reference/context.rs`
- `src/services/referencePayload.test.ts`
- `src/components/ag/Composer.test.tsx`（如需）
- `scripts/test_governance.test.mjs`
- `docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md`（变更记录注明落地）
- `.governance/issues/AG-1.0-IMP-EXTERNAL-CONTENT-01.md`、`.governance/active_scope.json`

禁止修改：工作区其他未提交改动；其余 src/src-tauri 文件；AST/AG 记忆相关（显式记忆选择另立 Scope）；
其他模块文档；治理控制文档；governance.config.json。

## 13. 人类确认记录

2026-08-14：人类经 AskUserQuestion 确认本 Scope（外部内容 data-only）作为 BR-SYS-DATA-007 的第一实现
环节；显式记忆选择（MemorySelectionRef）作为后续独立 Scope。confirmation: YES。

## 14. Task Runtime Report

- task_type：`CODE_CHANGE`
- governance_phase：`IMPLEMENTATION`
- final_state：`READY`
- issue_trace：本文件
- changed_files：
  - `src/services/referencePayload.ts`（file 变体增 readContent；coerce 保留内容）
  - `src/components/ag/Composer.tsx`（模块级 EXTERNAL_READABLE_EXT/EXTERNAL_CONTENT_CAP；
    fileReferenceFromTransfer 改异步只读解析；onPaste/onDrop 文件路径先阻止默认再异步注入）
  - `src-tauri/src/types/mod.rs`（ReferencePayload::File 增 read_content，serde readContent 缺省兼容）
  - `src-tauri/src/reference/context.rs`（File 分支分叉：有内容 cap 截断注入 / 无内容保留"未读取内容"；
    @GOV rules 增 BR-SYS-DATA-007、term_ref TERM-REF-005；3 个新测试）
  - `src/services/referencePayload.test.ts`（readContent data-only 用例）
  - `docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md`（§6.1 契约落地变更记录 v1.9）
  - `docs/00_core/A-CORE-C-R-01_ADUS.md`（generate 更新：测试用例 902→906）
  - 本 Issue Trace、`.governance/active_scope.json`
- mapping_chain：BR-SYS-DATA-007 → context.rs @GOV（rules+term_ref）+ 测试 covers（context 3 用例 +
  referencePayload 1 用例），**阶段一规则缺口全部闭环，audit 全绿**。
- test_impact：
  - changed_files：referencePayload.ts、Composer.tsx、types/mod.rs、reference/context.rs、referencePayload.test.ts
  - direct_modules：REF、AG、SYS（CORE）
  - selection：test:plan 判定前端全量 + tsc + cargo 全量 + audit
- validation：
  - `npm run test:related`（npm test 前端全量） # passed：26 files / 573 tests（referencePayload 13 过；
    test_governance 19 过）
  - `npx tsc --noEmit` # passed
  - `cargo test`（Rust 全量） # passed：333 tests（reference 16 过，含 File 分支 3 新用例）
  - `npm run governance:generate` # passed（测试用例 902→906）
  - `npm run governance:audit` # **passed（exit 0，全部检查通过）**——BR-SYS-DATA-007 进入代码 @GOV 引用
    与测试 covers，D1 阶段一规则缺口（007/008/DATA-007）全部闭环
  - `git diff --check` # passed
- automated_tests：前端 573 + Rust 333 全过。
- stop_conditions：无。
- unresolved_items：显式记忆选择（MemorySelectionRef）为 BR-SYS-DATA-007 的第二实现环节（另立 Scope，
  AST get-by-id + ipc + 记忆面板 UI）；docx/doc 二进制格式的外部内容无 Web 纯文本只读路径，保持元数据
  （后续可补专用解析）；大文件读取上限 20000 字符截断为实现防御值，可调。
