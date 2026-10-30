# Issue Trace — ED-SESSION-LEDGER-GOV-01：编辑器会话恢复缺少 REQ/RULE，路由块无法映射

文档状态：A
任务类型：DOC_ONLY（缺口登记，实施另行立项）
治理阶段：G1 需求（未通过）
创建时间：2026-10-30
记录来源：AG 治理覆盖审计（commit 35051b6）后对 ED 模块的同类检查

## 1. 用户可见症状

本轮把治理覆盖检查从 `src-tauri/src/ag/` 推向 `ed/` 时发现：编辑器"重启恢复"这条用户可见能力
（重新打开应用后恢复打开集合、顺序、当前文档与 LogicalState 镜像）**只有接口协议定义，没有需求条目
与规则 ID**。因此实现它的路由层无法做块级 `@GOV` 标注，任何标注都只能套用语义不匹配的规则，
即"空头代码块"。

审计侧表现：`src-tauri/src/ed/routes.rs` 266 行、4 个命令、`@GOV` 数量为 0，且在 ADUS 规则视图里
完全不可见。当前审计把 ED 排除在覆盖检查外（`GOVERNED_DIRS` 仅含 `src-tauri/src/ag/`），
所以这个缺口不会让 audit 失败，也不会被发现。

## 2. 入口点

- `src-tauri/src/ed/routes.rs:13` `route_persist_session_snapshot`
- `src-tauri/src/ed/routes.rs:104` `route_list_session_snapshots`
- `src-tauri/src/ed/routes.rs:142` `route_persist_open_tabs`
- `src-tauri/src/ed/routes.rs:241` `route_list_open_tabs`
- 接口定义：`docs/01_modules/ED/A-ED-I-P-01_编辑器接口协议.md:37-40`（命令表）、`:51-52`（重启恢复顺序）

## 3. 相关规则

现有（registered，均不覆盖本缺口）：
- BR-ED-DATA-001~008：blockId、格式模型、分页、字符锚、样式、Markdown 管线、表格、编辑会话事实投影
- BR-ED-STATE-001~003：编辑生命周期、装饰层、外部变更与冲突
- BR-ED-VERIFY-001
- BR-SYS-PERSIST-001（schema 版本化迁移）：覆盖表结构迁移，不覆盖"打开集合/镜像语义"

缺失：
- 没有任何规则约束"打开集合（顺序 + 当前文档）+ LogicalState 镜像 + 重启恢复顺序"；
- 接口文档无 `REQ-ED-*` 引用，需求层没有对应条目；
- 未登记 REQ_MAP。

## 4. 相关链路

ED-EDIT（编辑器受控接口）→ DATA-MIGRATE（表 schema）→ WS-PERSIST（会话快照落库）。
重启恢复顺序另涉及前端 ED（`src/services/edService.ts`）与 `src/store/edStore.ts` 的恢复入口。

## 5. 生产者块

实现侧（待标注，当前无块）：
- `route_persist_session_snapshot`（写 `ed_session_snapshots`）
- `route_persist_open_tabs`（写 `ed_open_tabs`）

## 6. 消费者块

- `route_list_session_snapshots`（读镜像）
- `route_list_open_tabs`（读打开集合）
- 前端恢复入口（`edService` / `edStore`，路径未逐项核实，本记录不下结论）

## 7. 路径上的空规则块

`src-tauri/src/ed/routes.rs` 全部 4 个命令函数：有行为、有错误分支、有持久化副作用，但无任何规则可映射。
`src-tauri/src/ed/mod.rs`（7 行，仅 `pub mod routes;`）可按 AG 先例走 `@GOV-EXEMPT`。

## 8. boundary 假设

接口文档已给出可用的边界语句，可作为规则正文来源：
- "重启恢复必须先调用 `ed.list_open_tabs` 得到打开集合、顺序与当前文档，再只恢复这些路径对应的
  `ed.list_session_snapshots` 镜像；不在 `ed_open_tabs` 中的孤立 session 镜像不得打开文档。"
- "BDocx 恢复投影含 baseline、纯净规范 HTML；临时/非法引用必须在返回前移除。"

## 9. 运行时假设

- `ed_session_snapshots` / `ed_open_tabs` 的 schema 由 DATA-MIGRATE 版本化管理（BR-SYS-PERSIST-001）。
- 路由层不直接操纵编辑器内部文档状态（`ed/mod.rs` 模块头声明）。

## 10. 疑似不变量破坏

无运行时故障证据。本记录只陈述治理不变量破坏：**有行为的代码块无法追溯到需求与规则**，
违反 ADU §4.2/§4.4 的"规则 → 代码块"有向连接要求。

## 11. 验证计划

本轮不实施。并行任务补全后应验证：
1. 新规则注册进 ADUS（规则注册表 + 规则视图可查到新块 ID）；
2. `ed/routes.rs` 覆盖检查纳入 `GOVERNED_DIRS` 且 audit 通过；
3. 4 个命令各自有块级 `@GOV`（codes 含规则 ID 与唯一块 ID），或按 ADU §4.3 明确豁免依据；
4. `npm run governance:audit`、`npm run test:governance` 通过。

## 12. 文件范围结论（Scope Lock 输入）

记为缺口，当前**不改代码**。补全时的候选范围：
- `docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md`（新增规则）
- `docs/01_modules/ED/A-ED-I-P-01_编辑器接口协议.md`（REQ 引用与规则绑定）
- 需求文档（新增 REQ 条目，位置待定）
- `src-tauri/src/ed/routes.rs`、`src-tauri/src/ed/mod.rs`
- `scripts/audit_taskflow_governance.mjs`（把 ED 纳入覆盖检查）
- `docs/00_core/A-CORE-C-R-01_ADUS.md`（重新生成）

## 13. 规则草案（供并行任务评审，未注册）

> **BR-ED-DATA-009（会话恢复，暂定名）**：编辑器必须持久化打开集合（顺序 + 当前文档）与已打开文档的
> LogicalState 镜像；重启时先取打开集合，再只恢复集合内路径的镜像，孤立镜像不得打开文档；
> BDocx 镜像必须携带 baseline 且图片为规范相对引用，临时或非法引用在返回前移除。

主链路建议：ED-EDIT；候选块 ID：`DATA-ED-ED-EDIT-0xx` / `EFFECT-ED-ED-EDIT-0xx`。
该草案**未经人类确认，不得直接写入设计文档或代码**。
