# Issue Trace — CHAIN-INTEGRITY-REPAIR-01：拆分后链路完整性与唯一性治理

文档状态：A
任务类型：GOVERNANCE_CHANGE + CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-26
上游基线：ADU.md §3.2、§4.2–§4.6、§5.6；A-SYS-I-P-01；AG/DE/WS 现行技术设计；人类 2026-08-26 明确要求立即修复 CHAIN 模板并按审计建议治理其余问题。

## 1. 用户可见症状

拆分后的模块入口与业务实现已归位，但 AgentTurn 在 TaskRun 创建后的部分错误路径未统一结算；规则映射存在代码块 ID 复用、`rules`/`codes` 不闭合及主责/委托关系不清；DiffProjection 的代码契约存在多处定义；CHAIN 的 `integration_test` 规范、生成器字段名与设计文档声明不一致。

## 2. 入口点

- `ADU.md` 与 `scripts/generate_adus.mjs`：CHAIN `integration_test` 单数契约。
- `src-tauri/src/ag/turn_runtime.rs`、`turn_tool_loop.rs`：AgentTurn/TaskRun 收束。
- `src-tauri/src/types/mod.rs`、`src-tauri/src/de/projection.rs`、`src/services/agService.ts`：DiffProjection 共享契约。
- 拆分后的 AG/DE/WS/DOC/REF/AST/ENT 路由与运行时 `@GOV` 块，以及全库扫描发现的同类重复代码块 ID。
- 各 `<!-- CHAIN -->` 标注块与现有统一测试资产。

## 3. 相关规则

- ADU §4.4：代码块 ID 全局唯一。
- ADU §4.5：每条规则一个 primary owner；非主责引用声明 consumer/delegate/caller_context。
- ADU §5.6：CHAIN `integration_test` 必填，可复用现有统一测试或显式 deferred。
- BR-SYS-DATA-002 / X-SHARED-001：跨模块共享结构以 SYS 契约层为唯一来源。
- BR-AG-STATE-001 / BR-SYS-STATE-008：AgentTurn 与 TaskRun 状态闭合、单一终态。

## 4. 相关链路

AG-TURN、AG-TOOL、AG-ASSIST、DE-DIFF、ED-EDIT、WS-OPERATION、WS-DIFF-APPLY、DOC-CONVERT、REF-INJECT、DATA-MIGRATE、IPC-CALL，以及拆分所涉及模块的其他已实现链路。

## 5. 生产者块

AgentTurn 运行时、DiffProjection 构造器、各模块 chain entry、CHAIN 注册标注与 ADUS 生成器。

## 6. 消费者块

TaskRun/AgentTurn 历史、DE/ED/WS、前端 AG service、测试治理和 ADUS 链路注册表。

## 7. 路径上的空规则块

不新增无法解释的空规则块。薄包装入口只在 ADU 允许时由下游块覆盖；独立 CHAIN 入口必须显式标注。

## 8. boundary 假设

- 相同业务规则可约束多个代码块，但代码块 ID 唯一，且只有一个 primary owner。
- 协调者不重复声明下游 GUARD/EFFECT 已拥有的规则，以 delegate/consumer 表达关系。
- DiffProjection 字段形状归 SYS 共享契约，DE 继续拥有业务语义和生成逻辑。
- 同一统一测试可被多条真实覆盖的 CHAIN 显式引用；没有证据的链路写 deferred，不伪造覆盖。

## 9. 运行时假设

TaskRun 一旦创建，除明确等待确认/澄清/预览外，所有退出都必须收束为终态并与 AgentTurn 终态一致；正常事件顺序、命令名和持久化协议不改变。

## 10. 疑似不变量破坏

- 装配失败导致 AgentTurn error、TaskRun executing。
- 同一代码块 ID 指向多个实现，无法唯一回溯。
- Rust/TypeScript DiffProjection 字段漂移。
- CHAIN 已声明测试因生成器字段名不匹配而在 ADUS 丢失。

## 11. 验证计划

- 扩展现有测试套件覆盖 TaskRun 装配失败收束与共享投影序列化契约，不新建按链碎片测试文件。
- 运行 governance:generate、governance:audit、test:plan、同输入 test:related、test:governance、Rust 格式与 git diff 检查。
- 若 Rust 全量继续遭遇既有系统资源或 LibreOffice Worker 阻断，保留真实 BLOCKED 证据。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-26 | v1.0 | 人类确认立即修复 ADU CHAIN 模板并按审计建议治理 TaskRun、映射、共享契约与链路测试登记。 |
| 2026-08-27 | v1.1 | 人类明确接受 LibreOffice Worker `signal=6` 作为环境验证例外并要求闭合提交；保留 358/359 真实结果，不改写为通过。 |

## Task Runtime Report — 拆分后链路完整性与唯一性治理

task_type: CODE_CHANGE
governance_phase: VALIDATION
final_state: READY
issue_trace: CHAIN-INTEGRITY-REPAIR-01

changed_files:
  - `ADU.md`、`scripts/generate_adus.mjs`、`scripts/audit_taskflow_governance.mjs` # CHAIN 单数测试契约、生成与硬审计
  - `docs/00_core/A-CORE-C-P-02_产品能力图谱规范.md`、`docs/01_modules/**`、`docs/00_core/A-CORE-C-R-01_ADUS.md` # 27 条 CHAIN 登记与 DiffProjection 契约
  - `src-tauri/src/ag/**` # TaskRun 统一失败出口、拆分后规则映射和回归测试
  - `src-tauri/src/types/mod.rs`、`src/types/diffProjection.ts` # DiffProjection 后端/前端共享类型源
  - `src-tauri/src/de/**`、`src-tauri/src/workspace/**`、`src-tauri/src/doc/**`、`src-tauri/src/reference/**`、`src-tauri/src/ast/**` # 拆分模块映射闭环
  - `src-tauri/src/session/**`、`src-tauri/src/cbt/**`、`src-tauri/src/tpl/**`、`src-tauri/src/kb/**`、`src-tauri/src/prv/**` # 全库重复代码块 ID 修正
  - `src/components/ag/Composer.tsx`、`src/components/ag/MessageList.tsx`、`src/components/ed/markdown/markdownCodec.ts`、`src/services/{agService,deService,edService,referenceDrag,referencePayload}.ts` # 前端映射闭环与 DiffProjection 消费
  - `src-tauri/src/de/projection.rs` # 删除，后端重复契约源已收归 SYS types

scope:
  mode: explicit
  status: confirmed

phase_gate:
  status: passed
  evidence: 人类明确授权修改 ADU 并按审计建议修复其余链路问题；现有 REQUIREMENT/DESIGN/PLAN 基线完整。

mapping_chain:
  status: complete
  evidence: 96 条规则、27 条 CHAIN、295 个代码块通过 rules/codes 双向闭合、单块单 ID、全局 ID 唯一审计；DiffProjection 指向 SYS 共享契约。

test_impact:
  changed_files: 当前工作树全部实际变更文件（包含本连续治理任务此前未提交改动）
  direct_modules: AG, AST, CBT, CORE, DE, DOC, ED, ENT, KB, PCG, PRV, REF, SYS, TPL, WS
  affected_modules: ACCT, AG, AST, CBT, CORE, DE, DOC, ED, ENT, KB, PCG, PRV, REF, SYS, TPL, WS
  selection: product:test + test:governance + frontend-full + tsc + rust-full + governance:audit

validation:
  - `npm run governance:generate` # passed；27 条 CHAIN 全部输出 integration_test
  - `npm run governance:audit` # passed；0 governance findings，295 个代码块闭合且唯一
  - `npm run product:test` # passed；5/5 选中测试
  - `npm run test:governance` # passed；19/19，104 个测试资产、941 个测试用例登记健康
  - `npm test` # passed；589/589
  - `npx tsc --noEmit` # passed
  - `cargo check --manifest-path src-tauri/Cargo.toml` # passed（仅既有 dead_code warning）
  - `cargo test --manifest-path src-tauri/Cargo.toml` # failed；358/359 通过，唯一失败为 LibreOffice Worker signal=6
  - `cargo test --manifest-path src-tauri/Cargo.toml started_turn_failure_settles_task_run` # passed；新增 TaskRun 收束回归
  - `git diff --check` # passed

stop_conditions:
  - SC-04 VALIDATION_BLOCKED 已由人类于 2026-08-27 明确接受为环境验证例外并解除：`doc::libreoffice_runtime::tests::docx_to_pdf_end_to_end_when_libreoffice_available` 因打包 LibreOffice Worker `signal=6` 失败；该结果仍按未通过记录，不计入测试通过项。

recovery:
  - 当前代码与治理审计已通过；环境恢复后执行 `cargo test --manifest-path src-tauri/Cargo.toml`，再以相同 changed_files 重跑 `npm run test:related`。

unresolved_items:
  - 20 条尚无完整链路级证据的 CHAIN 已按 ADU 合法登记 `deferred: phase-5`，未伪造成单独测试。
  - AI 语义比对未发现新的规则名词漂移；重复规则引用均已收敛为直接主责或显式 `delegate` / `consumer` 关系。
  - Rust 全量仍为 358/359；人类仅接受其作为本次提交的环境验证例外，不代表该 LibreOffice 用例已经通过或根因已经修复。
