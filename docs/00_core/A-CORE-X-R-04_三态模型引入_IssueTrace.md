---
文档编号：CORE-X-R-04
文档状态：A
负责模块：CORE
文档职责：三态模型（DiskState / LogicalState / DisplayState）引入的 Issue Trace——变更范围溯源、影响模块清单、Scope Lock 范围结论
上游约束：A-CORE-C-T-01_三态模型.md（契约定义源）、ADU.md §6（Issue Trace 格式）、ADP.md（TRACE_GATE / Scope Lock）
直接承接：本次变更中新增/修改的全部文件，经此 Issue Trace 约束范围
使用边界：本文是 Issue Trace 执行记录，不作为源代码规则引用
变更要求：任务完成后作为执行记录保留（ADU §6.3），不得改为 R 或 X
治理成熟度：draft
---

# 三态模型引入 —— Issue Trace

## 1. 用户可见症状

本次变更是审计后契约显性化，无直接用户可见症状。变更的间接效果：

- D4 修复（draftBaseDirty）后，用户在编辑→AI diff→reject 的真实脏态不再丢失，关闭标签时可获得保存提示
- 后续设计/开发人员编写涉及"保存/脏态/装饰"的代码时，有三态模型作为统一命名体系可遵循

## 2. 入口点

审计报告的 D1~D5 发现。具体入口：用户请求的三态模型专项审计。

## 3. 相关规则

- X-FORBID-008（三态单向数据流，本次新增，A-CORE-C-T-01 §2）
- X-FORBID-009（AI 生成内容归属 LogicalCandidate，本次新增，A-CORE-C-T-01 §2）
- X-FORBID-005（不得绕过 ED 受控接口，A-ED-M-T-01 §8）
- X-SEQ-002（前端不跨层直连 FS，A-SYS-C-T-01 §5）
- X-FORBID-007（前端 store 投影，A-SYS-C-T-02 §5）
- BR-ED-STATE-001（编辑会话生命周期与脏态，A-ED-M-T-01 §3）
- BR-ED-STATE-002（decoration 纯呈现，A-ED-M-T-01 §7）
- BR-ED-DATA-003（分页纯呈现，A-ED-M-T-01 §6）
- BR-DE-STATE-001（diff 终态互斥，A-DE-M-T-01 §5）
- BR-WS-STATE-001（仅 accepted 落盘，A-WS-M-T-01 §5）

## 4. 相关链路

```
CORE-THREE-STATE（本次新增概念链路）
  ├─ IPC-CALL（IPC payload 三态标注，A-CORE-C-T-01 §6）
  ├─ ED-EDIT → ED-DECOR（逻辑态→显示态单向流，A-CORE-C-T-01 §1.4）
  ├─ DE-DIFF → WS-DIFF-APPLY（LogicalCandidate→LogicalState→DiskState，A-CORE-C-T-01 §5.5）
  └─ ED-EDIT → WS-SAVE（LogicalState→DiskState，A-CORE-C-T-01 §1.4）
```

## 5. 生产者块

- `BR-DE-STATE-001-RB-DE-DE-DIFF-002`（deStore.ts:179-191，SM-DE-01 投影）——本次增加三态归属注释
- `BR-DE-STATE-001-RB-DE-DE-DIFF-001`（de/mod.rs:64-71，SM-DE-01 转移引擎）——引用规则，无变动
- `BR-ED-STATE-001-RB-ED-ED-EDIT-002`（edStore.ts:127-137，TabState 投影）——本次增加三态注释
- `BR-AG-STATE-003-RB-AG-AG-ASSIST-002`（Editor.tsx:335-343，InlineAssist 守则）——增加三态链路标注

## 6. 消费者块

- `BR-DE-DATA-004-QUERY-ED-ED-EDIT-010`（edLocate.ts:141-149，块快照查询）——引用三态术语，无代码变动
- `BR-ED-STATE-001-EFFECT-ED-ED-EDIT-011`（edLocate.ts:204-212，受控块替换）——引用三态术语，无代码变动
- `BR-DE-STATE-001-RB-DE-DE-DIFF-002`（deStore.ts:179-191）——自身也是消费者（consumes SM-DE-01）
- 三态文档 TERM-CORE-001/002/003/004 将作为 @GOV term_ref 的新引用目标

## 7. 路径上的空规则块

未发现空规则块。本次不涉及新代码块的 @GOV 标注。

## 8. boundary 假设

本次变更为文档与注释修改（含 D4 代码修复），不涉及新的 lifecycle_assumption 或 document_snapshot 声明。已有 @GOV boundary 不变。

## 9. 运行时假设

无运行时假设变更。D4 修复仅在裁决后 TabState 恢复路径中增加一个布尔判断，不改变现有运行时行为。

## 10. 疑似不变量破坏

- X-FORBID-008（新增）不会破坏现有不变量，它是对已有实践（display→logical 禁止、save_atomic 唯一写盘）的显式合约化
- X-FORBID-009（新增）不会破坏现有不变量，AI 内容输出为 LogicalCandidate 已在 SM-DE-01 的 transition 规则中体现（userAccept 前不落盘）
- D4 修复：`draftBaseDirty` 只在 reject 后优先恢复 dirty，不降低任何既有测试覆盖的安全。`markDraftApplied` 调用方（Editor.tsx L316）传递的 baseTabState 为调用时刻的快照值，与以前相同

**未发现不变量破坏风险。**

## 11. 验证计划

- 运行 `npm run governance:generate` 生成 ADUS
- 运行 `npm run governance:audit` 检查新约束和 TERM 注册
- 运行 `npx vitest run src/store/deStore.test.ts` 验证 D4 新增的测试用例
- 运行 `npx vitest run src/services/deService.test.ts` 验证现有 deService 测试不因 D4 修改而破坏
- 运行 `npx vitest run` 全量前端测试确保无回归

---

## 文件范围控制（Scope Lock）

### 允许修改的文件

| 文件 | 修改类型 | 依据 |
|------|----------|------|
| `docs/00_core/A-CORE-C-T-01_三态模型.md` | **新增文档** | 三态权威定义源（生产者块 §5） |
| `docs/00_core/A-CORE-X-R-04_三态模型引入_IssueTrace.md` | **新增文档** | 本 Issue Trace |
| `docs/01_modules/DE/A-DE-M-S-01_DiffEdit状态机设计.md` | 修改（新增 §4 + 编号顺移） | D5 决议落地（§4 路径选择表） |
| `src/store/deStore.ts` | 修改（注释 + draftBaseDirty 字段） | D2 注释标注 + D4 修复 |
| `src/store/deStore.test.ts` | 修改（新增 D4 测试用例） | D4 修复验证 |
| `src/store/edStore.ts` | 修改（注释标注双副本关系） | D3 决议落地 |
| `src/services/deService.ts` | 修改（D4 修复逻辑 + D5 注释） | D4 修复 + D5 注释 |
| `src/components/ed/Editor.tsx` | 修改（注释标注三态链路） | D2 注释标注 |

### 禁止修改的文件

不在允许列表中的文件默认禁止。

---
