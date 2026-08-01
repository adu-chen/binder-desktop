---
文档编号：SYS-X-L-01
文档状态：A
负责模块：SYS
文档职责：SYS 系统底座 G3 开发计划——承接型 Plan Item 清单，每条承接 REQ/RULE/CHAIN/CONSTRAINT/状态机，明确文件范围、@GOV 落点、承接的 A 类实现期债务、验收（对接 A-CORE-X-L-02 P1）
上游约束：A-CORE-X-L-02_总开发实施计划.md（P1 验收标准、G3 门禁）、A-CORE-C-P-01_APC.md（§5 G3/G4 标准、§6 状态机驱动、§12 @GOV/covers）、A-SYS-* L2（C-T-01/I-P-01/I-P-02/I-P-03/M-S-01/C-T-02 规则源）、A-CORE-X-R-01（A 类债务承接输入）
直接承接：本计划 Plan Item 驱动 SYS 实现（src/ + src-tauri/src/）；每条 @GOV 追溯已注册 BR-SYS-*；状态机 SM-SYS-01 转移测试矩阵
使用边界：本文是 L3 承接型 Plan Item（映射链正式成员，APC §2），**引用规则不定义规则**（不放注册块）、不重定义 L2 契约；@GOV 落点在实现代码、不在本文；NFR 阈值延 G5
变更要求：Plan Item 增删须保持对 REQ/RULE/状态机的承接可追溯；L2 契约变更须回溯影响的 Plan Item
治理成熟度：draft
---

# Binder 系统底座开发计划（SYS · L3）

## §1 文档定位

承接型 Plan Item 是映射链的正式成员（REQ→…→状态机→**Plan Item**→@GOV→Test）。本文把 SYS L2
的规则/链路/约束/状态机切成可编码条目，每条明确：承接对象、文件范围、@GOV 落点、承接的 A 类
实现期债务、验收。**G3 门禁**：每条 Plan Item 承接齐全且文件范围明确即过 G3（APC §5）。

> 本计划为 G3 首件，确立 Plan Item 结构供其余模块沿用。SYS 是底座，实现优先级最高。

## §2 Plan Item 清单

### PI-SYS-01　IPC 信封与统一错误形状　✅ G4 已实现（2026-06-22，cargo test 3 covers + @GOV + audit 绿）

- **承接**：REQ-SYS-001 / CHAIN IPC-CALL / BR-SYS-DATA-003（统一 command 协议）、BR-SYS-OBS-002
  （统一错误形状）、BR-SYS-STATE-002（流式逻辑契约）。
- **文件范围**：`src-tauri/src/ipc/`（信封/错误/command 注册表）、`src/services/`（前端 Service 层
  封装）。
- **@GOV 落点**：command 路由与错误封装处 `/** @GOV rules: BR-SYS-DATA-003, BR-SYS-OBS-002 */`；
  流式回传处 `BR-SYS-STATE-002`。
- **承接 A 类债务**：报文序列化格式与标量类型（实现期定）；**流式物理传输机制 SY-02**（Channel<T>
  vs event，实现期选型并回填 A-SYS-I-P-02 §5/§10）；背压/流控随选型定；**错误码总表**（code 枚举，
  SYS 实现期补遗，全模块共用一张表）。
- **验收**：未注册 command 被统一错误形状拒绝；IPC 错误与云错误字段集合一致（code/message/
  retriable/reauth_required）；流式具备完成/中断/错误三终态、中断无半落盘。

### PI-SYS-02　跨模块共享数据结构　✅ G4 已实现（2026-06-22，cargo test + @GOV + audit 绿）

- **承接**：REQ-SYS-002 / BR-SYS-DATA-002（共享结构单一来源）、X-SHARED-001。
- **文件范围**：`src-tauri/src/types/`（WorkspaceRef/DocRef/PendingDiff/TimelineEntry/RefInjection/
  MemoryInjection/KnowledgeInjection）+ 前端镜像类型 `src/types/`。
- **@GOV 落点**：共享结构定义处 `BR-SYS-DATA-002`。
- **承接 A 类债务**：不透明 ID 生成算法（ULID/UUID，实现期选型）；payload_ref/before_snapshot
  序列化（归 DE 实现期，本 PI 只占类型位）。
- **验收**：跨模块共享实体与枚举集合仅一处定义；audit 无重复定义。

### PI-SYS-03　workspace.db schema 与版本化迁移框架　✅ G4 已实现（2026-06-22，cargo test 含重启恢复 + @GOV + audit 绿）

- **承接**：REQ-SYS-002 / CHAIN DATA-MIGRATE / BR-SYS-PERSIST-001（版本化迁移 + 重启可恢复）。
- **文件范围**：`src-tauri/src/db/`（schema_meta + 完整表 + 迁移框架）。承接 A-SYS-I-P-01 §4
  全部基线表 + 七张控制态扩展表（agent_turns/build_runs/workflow_runs/templates/workflows/
  memory_entries/knowledge_entries）。
- **@GOV 落点**：迁移驱动与恢复处 `BR-SYS-PERSIST-001`。
- **承接 A 类债务**：完整列/索引调优（实现期基准测试微调）；扩展表列定义已在 A-SYS-I-P-01 v1.2
  登记基线，实现期细化。
- **验收**：schema_version 单调递增驱动迁移；含未裁决 pending_diffs + 时间轴的 db 经一次迁移后
  重启控制态完整可恢复、无数据丢失（BR-SYS-PERSIST-001 验收）。

### PI-SYS-04　会话与订阅门禁状态机（SM-SYS-01）　✅ G4 已实现（2026-06-22，cargo test 含强制转移矩阵 + @GOV + audit 绿；OBS-001/DATA-004 主责留 PI-ENT 网关实现）

- **承接**：REQ-SYS-005 / SM-SYS-01 / BR-SYS-STATE-001 / BR-SYS-VERIFY-001 / BR-SYS-DATA-001 /
  BR-SYS-OBS-001 / BR-SYS-DATA-004（网关内容不留存）/ X-FORBID-001 / X-SEQ-001 / X-FORBID-002。
- **文件范围**：客户端会话+门禁运行时框架（ACCT/ENT 承接段实现见各自 Plan Item，本 PI 建
  SM-SYS-01 状态机骨架与转移守护）。
- **@GOV 落点**：状态机转移与守护处追溯对应 BR-SYS-*；门禁前置守护 `X-SEQ-001`、密钥边界
  `X-FORBID-001`、无绕过 `X-FORBID-002`。
- **承接 A 类/C 类**：令牌算法/格式、网关流式底层协议、门禁裁决响应结构、BYOK 传输安全——
  **依赖云端 AUTH/GW/LIC（C 类，云端 APC）**，客户端侧按契约打桩实现，真联调延 G5/P5。
- **验收**（强制，APC §6/§10）：SM-SYS-01 转移测试矩阵覆盖全 transitions + forbidden_transitions
  （无 gateAllow 不可达 Invoking；GateDenied 不可转 Invoking；未认证态调用被拒）；静态可查无
  客户端持密钥/绕过门禁路径。

### PI-SYS-05　前端交互外壳与状态投影（含欢迎页）

- **承接**：REQ-SYS-006 / X-FORBID-007（前端状态投影，不另立权威）/ X-SEQ-002（前端分层）；
  视图骨架 A-SYS-C-T-02。
- **文件范围**：`src/`（应用外壳：欢迎页 Welcome / 主工作台 Workbench 容器、Store 投影层、Service
  层）。
- **@GOV 落点**：Store 投影层 `X-FORBID-007`；前端跨进程调用分层 `X-SEQ-002`。
- **承接 A 类债务**：界面布局排布/面板停靠（贴 G4 just-in-time，A-SYS-C-T-02 §9）；多窗形态
  （C 类，待人类产品决策）。
- **验收**：欢迎页↔工作台切换投影 workspace 活动态；静态审查前端 Store 无与后端可漂移的并行业务
  状态（X-FORBID-007）；前端经 Service→command 分层、无跨层直连（X-SEQ-002）。

### PI-SYS-06　BDocx 代理编辑能力清单　✅ G4 已完成（BDOCX-AGENT-CAPABILITY-MANIFEST-01）

- **承接**：REQ-SYS-002 / BR-SYS-DATA-006 / TERM-SYS-002 / X-SHARED-001；跨链引用
  BR-AG-STATE-002、BR-ED-DATA-005/007、BR-DOC-DATA-002、BR-DE-DATA-002。
- **文件范围**：`src-tauri/resources/bdocx-agent-capabilities.json`（机器清单）、
  `src-tauri/src/ag/bdocx_capabilities.rs`（加载、版本/hash、Schema/提示词投影、执行前校验）、
  `src-tauri/src/ag/prompts.rs` / `tool_registry.rs` / `blocks.rs` / `lib.rs`（消费面），以及
  `src/services/edStyleOps.ts`（ED 防御性校验）。
- **@GOV 落点**：manifest 加载与一致性校验、提示词投影、工具 Schema 投影、Diff 前参数校验引用
  BR-SYS-DATA-006；StyleOp 当前锚/范围继续引用 BR-ED-DATA-005。
- **非目标**：不新增编辑功能，不提升 DOCX 往返等级，不修改转换器、Diff UI、保存状态机或表格执行器。
- **验收**：同一 manifest version/hash 投影提示词、工具 Schema 和校验器；错误 `sub/sup` 不再声明；
  StyleOp 未知 mark/非法 attrs/错误锚/越界范围在 PendingDiff 前拒绝；TableOperation 有条件 Schema 且
  继续服从 list_tables 动态能力；治理、Rust、TypeScript 与定向 Vitest 通过。

## §3 实施顺序与依赖

```
PI-SYS-02（类型）→ PI-SYS-01（IPC）→ PI-SYS-03（db/迁移）→ PI-SYS-04（会话门禁骨架）
PI-SYS-05（前端外壳）可与 PI-SYS-01 并行（依赖 IPC 信封就绪）
```

- PI-SYS-04 的真联调依赖云端（P5），G3/G4 阶段以契约打桩实现。
- 错误码总表（PI-SYS-01 承接）是全模块共用前置，须早定。

## §4 横切验收（承 A-CORE-X-L-02 §4）

- `governance:audit` 绿；@GOV 追溯已注册 BR-SYS-*（不引用 REQ）；SM-SYS-01 转移测试矩阵；
  `// covers: BR-SYS-*` 覆盖 SYS 全部已注册规则（10 条）且有实际断言。

## §5 承接的 A 类债务汇总（来自 A-CORE-X-R-01 §6）

| 债务 | 承接 PI | 处置 |
|---|---|---|
| SY-02 流式物理传输选型 | PI-SYS-01 | 实现期选型，回填 A-SYS-I-P-02 |
| 报文序列化/标量类型/ID 算法 | PI-SYS-01/02 | 实现期 |
| 错误码总表 | PI-SYS-01 | 实现期补遗（全模块共用） |
| workspace.db 完整列/索引调优 | PI-SYS-03 | 实现期基准测试 |
| 令牌/网关/门禁/BYOK 协议细节 | PI-SYS-04 | C 类，云端 APC 解，打桩实现 |
| 界面布局/多窗形态 | PI-SYS-05 | G4 just-in-time / C 类 |
| NFR（IPC 延迟/迁移性能） | 全 PI | G5 标定 |

## §6 后续动作

1. 本计划为 G3 SYS 首件；其余模块按实施计划 §7 序（WS→ED→DOC→DE→AG→…）产出各自
   A-{M}-X-L-01。
2. **AG Plan Item 须强制承接 4 项 B 类**（B-1~B-4，门禁例外，A-CORE-X-R-01）——AG 计划产出时
   单列强制条目。
3. G3 各模块 Plan Item 齐备后搭工程脚手架（Tauri 2 + React/TS + Rust），进 G4 编码。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（G3 首件，确立 Plan Item 结构）：SYS 5 条 Plan Item（IPC 信封与错误/共享数据结构/workspace.db 迁移/会话门禁状态机/前端外壳含欢迎页），每条承接 REQ/RULE/CHAIN/CONSTRAINT/状态机 + 文件范围 + @GOV 落点 + A 类债务承接 + 验收；实施顺序与依赖；横切验收（audit/状态机矩阵/covers）；承接 A 类债务汇总；标注 AG Plan Item 须强制承接 4 项 B 类门禁例外 |
| 2026-07-28 | v1.1 | 新增 PI-SYS-06 BDocxCapabilityManifest，实现独立能力协议、机器清单、提示词/工具 Schema/执行前校验同源投影与定向验收。 |
| 2026-07-28 | v1.2 | PI-SYS-06 完成：manifest 1.1.0、运行时格式/能力/人工边界/往返风险注入、条件工具 Schema、Diff 前静态与 UTF-16 CharAnchor 校验均已接线并通过定向验证。 |
