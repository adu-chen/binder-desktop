# Issue Trace — AG-CONFORMANCE-CODE-S4A：REQ-AG-009 完整结构化契约

文档状态：A
任务类型：CODE_CHANGE
治理阶段：VALIDATION
任务状态：READY
创建时间：2026-08-29
上游基线：REQ-AG-009、REQ-SYS-009、REQ-ED-011；BR-AG-DATA-001/002；X-SHARED-001/002；人类选择完整结构化契约与只读 `report_project` 终结工具。

## 1. 用户可见症状

当前 AgentTurn 虽能取得活动文档和打开文档快照，但没有独立的版本化 `AgentContextFacts`，也没有可验证的 `ProjectReport` 完成契约。项目分析可用自由文本结束，系统无法证明回复所称的已读范围、来源与不确定性；REQ-AG-009 因而缺少从需求到生产证据的闭环。

## 2. 入口点

- `src/services/agService.ts::startTurn`
- `src-tauri/src/ag/turn_runtime.rs::ag_start_turn`
- `src-tauri/src/ag/turn_prompt.rs::prepare_turn_prompt`
- `src-tauri/src/ag/turn_tool_loop.rs::run_turn_tool_loop`

## 3. 相关规则

- BR-AG-DATA-001 / BR-AG-DATA-002
- X-SHARED-001 / X-SHARED-002
- BR-AG-DATA-008 / BR-AG-OBS-001
- BR-SYS-DATA-008 / BR-ED-DATA-008；DE 在 BR-DE-DATA-001 下继承投影边界，不重复注册近义规则

## 4. 相关链路

`ED/DE/WS facts → AG-TURN PromptPackage → AG-TOOL read ledger → report_project → AgentTurn terminal`

## 5. 生产者块

- `src/services/agService.ts::collectOpenDocSnapshotsReady/startTurn`
- `src-tauri/src/ag/tool_exec.rs` 的只读工具执行结果
- `src-tauri/src/types/mod.rs` 的共享跨模块类型

## 6. 消费者块

- `src-tauri/src/ag/turn_prompt.rs::prepare_turn_prompt`
- `src-tauri/src/ag/prompt_runtime.rs::assemble_agent_turn_messages`
- `src-tauri/src/ag/turn_tool_loop.rs::run_turn_tool_loop`
- 前端 `AgTurnEvent::ToolResult` 投影

## 7. 路径上的空规则块

现有代码块有规则引用，但没有覆盖 AgentContextFacts 与 ProjectReport 的规则；不是 `rules: []`，而是规则语义缺项。

## 8. boundary 假设

- AgentContextFacts 只表达当前回合开始时可验证的编辑器、选区引用和待裁决 Proposal 事实；文档正文仍由 OpenDocSnapshot/ReferencePayload 承载。
- 已打开文档的版本、脏态和来源以 LogicalState 快照为准；活动文档与已打开非活动文档必须可区分。
- ProjectReport 的已读项只能引用初始注入或本轮成功读取留下的证据；未进入证据账本的路径不能声明已读。
- report_project 只产出结构化只读报告证据，不扩大 ExplicitEditTargetSet、CapabilityGrant 写权限或 TaskRun 生命周期。

## 9. 运行时假设

- 项目 Inspect/Search/Analyze 会获得相应只读工具；其自然结束前必须成功调用 report_project。
- read_doc/list_docs/search_text/list_blocks 等成功结果按工具、路径、来源写入内存账本；失败结果不进入已读证据。
- report_project 失败会像其他工具校验失败一样回注模型，直到成功报告、循环上限、取消或错误终止。

## 10. 疑似不变量破坏

- 新增 Prompt 层必须同步 LayerKind 顺序、预算、审计与前端 trace 解析，避免形成第二套层序。
- 不能把所有 Analyze 都误判为项目报告；只有目标为 workspace/project 或实际使用工作区枚举/搜索时才应强制 ProjectReport。
- 不能因读取证据账本而把 read/search 结果加入 ExplicitEditTargetSet。
- ProjectReport 的自由文本展示不能替代结构化工具结果，结构化工具结果也不能伪装为系统验证通过。

## 11. 验证计划

- 类型与 serde/前后端字段一致性测试。
- IntentResult/CapabilityGrant 的 report_project 最小授权与非目标意图拒绝测试。
- Prompt 层序、data-only、预算与审计测试。
- 读取证据账本与 ProjectReport 成功/伪造/缺失来源/不确定性测试。
- 运行 `governance:generate`、changed_files 对应 `test:plan`/`test:related`、`test:governance` 与 `governance:audit`。
- 真实 UI/provider 链未执行则登记待验证，不将自动化证据提升为 G5。

## Scope Lock

允许修改：active Scope、本 Issue Trace、AG/SYS/ED/DE 相关技术与接口设计、Agent 计划/验证卡、机器生成 ADUS、共享类型、AG Prompt/Runtime/Tool 生产代码、前端 Agent 服务与直接测试。

禁止修改：ADU/ADP/AGENTS/CLAUDE/APC、需求正文、治理配置及所有未列入 active Scope 的文件。

## 人类决策

- `HD-S4A-01`（CONFIRMED）：采用完整结构化契约，独立注册 AgentContextFacts 与 ProjectReport。
- `HD-S4A-02`（CONFIRMED）：采用只读 `report_project` 终结工具；ProjectReport 已读范围必须由真实读取证据失败关闭。
- `HD-S4A-03`（CONFIRMED）：将 S4A 新增的 AgentContextFacts 前端直接测试计入治理基线，AG frontend test cases 由 109 调整为 110。

## Task Runtime Report

- task_type：`CODE_CHANGE`。
- governance_phase：`VALIDATION`。
- final_state：`READY`（需求、设计、生产实现与本地/非沙箱自动化已对齐；不等同 Agent 1.0 G5 或 `production_ready`）。
- issue_trace：`AG-CONFORMANCE-CODE-S4A`。
- changed_files：active Scope / S4A Issue Trace；AG/SYS/ED/DE 技术与接口设计；AG 1.0 开发验证计划与验证卡；机器生成 ADUS；`src-tauri/src/types/mod.rs`、AG Context/Prompt/Runtime/Tool 生产文件；`src/services/agService.ts`、`agRuntime.test.ts`；经 HD-S4A-03 确认纳入的 `scripts/test_governance.test.mjs`。均在 Scope Lock 内。
- implementation：注册 TERM-AG-020 `AgentContextFacts` 与 TERM-AG-021 `ProjectReport`，以 BR-AG-DATA-008 / BR-AG-OBS-001 / BR-ED-DATA-008 / BR-SYS-DATA-008 承接 REQ-AG-009 / REQ-ED-011 / REQ-SYS-009。前端在每个 AgentTurn 冻结 `agent-context-facts-v1`，只投影当前 workspace 的 ED LogicalState、本轮 SelectionRef/兼容 blockRefs 和当前 AgentSession Proposal 裁决/效果/保存态，不携带文档或 diff 正文；后端在 Gate/provider/ToolCall 前核验版本、workspace/turn/session、活动/打开文档、Selection 与 Proposal 身份。PromptPackage 新增位于 IntentHint 与 CurrentDocument 之间的独立 data-only 层并计入预算。Inspect/Search/Analyze 的最小 CapabilityGrant 加入只读 `report_project`，同轮证据账本按 Prompt/read_doc/list/search 的真实 coverage 核验 `project-report-v1`；伪造已读、跨 workspace、来源/coverage 不符、缺失 uncertainties 或报告后新读取均不能完成回合。该工具零写入、零 TaskRun/DecisionRequest/ChangeProposal，不改变 CapabilityGrant 或 ExplicitEditTargetSet。
- validation：`cargo fmt`、`cargo check --lib`、`npx tsc --noEmit` 通过；AgentContextFacts 前端定向 59/59、Rust 事实守卫/Prompt 层序/最小授权/ProjectReport 证据/终态门直接测试通过。完整 changed_files `test:plan` 选择前端全量、TypeScript、Rust 全量与治理审计；受控沙箱中首次 `test:related` 为前端 593/593、Rust 378/379，唯一失败是 LibreOffice Worker `signal=6`。同一 DOCX→PDF 用例在非沙箱 macOS 1/1 通过，随后原样非沙箱 `test:related` 完整通过：前端 593/593、Rust 379+5+4、TypeScript 与 `governance:audit` 均通过。`test:governance` 19/19 通过（107 个测试资产、965 个测试用例、0 未映射源码、0 skip/only）；治理审计 315 个唯一代码块、0 governance finding；`git diff --check` 通过。
- semantic_review：按 `REQUIREMENT → RULE/TERM → CHAIN → @GOV → 生产调用 → 测试/运行证据` 复核 REQ-AG-009 / REQ-ED-011 / REQ-SYS-009；ADUS 已登记 AgentContextFacts/ProjectReport 两个正式术语、四条 owner 规则、AG-TURN/AG-TOOL 生产块、caller/delegate/consumer 与直接测试覆盖。Proposal 继承既有 BR-DE-DATA-001，未创建近义术语或平行状态；未发现 `SEMANTIC_MISMATCH` 或 `NEEDS_HUMAN_DECISION`。
- stop_conditions：无。治理基线 109→110 的 Scope 扩展已由 HD-S4A-03 解除；LibreOffice 沙箱失败已由同用例非沙箱通过证明为环境限制。
- unresolved_items：`VAL-AG-PROJECT-REPORT-01`——真实 Tauri/WKWebView + provider 下 Inspect/Search/Analyze 的事实层序、读取证据、工具卡顺序、ProjectReport 与最终 Markdown 一致性待 G5。该项是系统验证，不是当前实现缺口。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-29 | v1.0 | 建立 S4A，冻结完整 AgentContextFacts / ProjectReport 契约与 report_project 证据校验方向。 |
| 2026-08-30 | v1.1 | 完成 REQ-AG-009 规则链、生产实现、证据账本、最小授权、直接/影响测试与 ADUS 登记；真实 provider/WKWebView 链转入 G5 待验证。 |
