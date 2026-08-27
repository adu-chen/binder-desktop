# Issue Trace — AG-PROMPT-RUNTIME-EXTRACT-01：提示词运行时装配职责拆分

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-25
上游基线：A-AG-M-T-01、A-AG-M-T-02、BR-AG-DATA-001/002/004/005，以及 2026-08-25 人类确认的“提示词装配迁出 lib.rs、归入提示词自身模块”要求。

## 1. 用户可见症状

动态提示词装配、当前文档投影、层级消息构造与 provider 消息转换集中在 `src-tauri/src/lib.rs`。入口文件同时承担 Tauri 路由、业务流程和 Prompt Runtime 细节，提示词实现没有落在 AG 模块自身位置，难以独立理解、测试和继续治理。

## 2. 入口点

- `src-tauri/src/lib.rs`：`build_current_doc_prompt_context`、`intent_hint_for_prompt`、`assemble_agent_turn_messages`、RuntimeControl 消息构造、`provider_messages_for_model`。
- `src-tauri/src/ag/prompts.rs`：静态系统提示词正文，保持不变。
- `src-tauri/src/ag/context.rs`：IntentResult、PromptPackage、LayerKind、历史选择，保持既有职责。
- `src-tauri/src/ag/mod.rs`：新增 Prompt Runtime 模块注册。

## 3. 相关规则

- `BR-AG-DATA-001`：上下文来源与裁剪。
- `BR-AG-DATA-002`：提示词分层与固定装配顺序。
- `BR-AG-DATA-004`：当前轮优先与 IntentCarryover。
- `BR-AG-DATA-005`：分层预算与历史裁剪。

本次不新增或修改规则，只移动既有规则实现所有权。

## 4. 相关链路

`AG-TURN`：IntentResult → 上下文准备 → Prompt Runtime 装配 → provider request → 工具续轮。

## 5. 生产者块

- `src-tauri/src/ag/context.rs`：产生 LayerKind、PromptPackage、Carryover 和历史裁剪结果。
- `src-tauri/src/ag/prompts.rs`：产生静态系统提示词与 BDocx 能力投影。
- `src-tauri/src/ag/blocks.rs`：产生当前编辑器文档快照。

## 6. 消费者块

- `ag_start_turn`：消费装配后的 messages、history payload 与当前文档 read-before-edit 标志。
- `ent::gateway::stream_round`：消费剥离内部 layer 元数据后的 provider messages。
- Prompt Audit：消费最终 provider messages，不改变其内容。

## 7. 路径上的空规则块

无新增空规则块。原 `assemble_agent_turn_messages` 的 @GOV 映射原样迁入 AG Prompt Runtime 模块。

## 8. boundary 假设

- 静态提示词正文继续由 `ag/prompts.rs` 持有。
- 动态消息装配归 `ag/prompt_runtime.rs`。
- Tauri command、数据库、网关和工具执行仍由 `lib.rs` 编排，不反向下沉到 Prompt Runtime。
- 迁移前后 provider 收到的 `messages` 顺序、role、content 与 tools 完全一致。

## 9. 运行时假设

- `layer` 仅为内部审计元数据，provider 前继续剥离。
- 当前文档超过 inline cap 时仍使用原 manifest/read-before-edit 逻辑。
- 历史预算、跨文档历史弱注入、IntentHint 和 RuntimeControl 文本保持原语义。

## 10. 疑似不变量破坏

- 改变可选层插入条件或 RuntimeControl 位置会违反 BR-AG-DATA-002。
- 遗漏 `history_payload` 会使 PromptPackage trace 与真实模型输入不一致。
- 暴露 `layer` 给 provider 会改变最终请求。
- 当前工作区有既有未提交改动，迁移必须保留并避免覆盖。

## 11. 验证计划

- 将现有消息顺序、历史隔离、当前文档 manifest、provider layer 剥离测试迁入 `ag/prompt_runtime.rs`。
- 最终 Prompt Audit 单次输出测试随实现迁入 `ag/prompt_runtime.rs`，确保只保留最终模型请求且不生成重复分解项。
- 运行 `cargo fmt --check`、`test:plan`、同输入 `test:related`、`governance:generate`、`governance:audit` 与 `git diff --check`。

## Scope Lock

```yaml
task_type: CODE_CHANGE
allowed_files:
  - src-tauri/src/lib.rs
  - src-tauri/src/ag/mod.rs
  - src-tauri/src/ag/prompt_runtime.rs
  - docs/00_core/A-CORE-C-R-01_ADUS.md
  - .governance/issues/AG-PROMPT-RUNTIME-EXTRACT-01.md
  - .governance/active_scope.json
forbidden_files:
  - AGENTS.md
  - ADU.md
  - ADP.md
  - CLAUDE.md
  - governance.config.json
  - docs/01_modules/**
  - src-tauri/src/ent/**
  - all files not listed in allowed_files
confirmation: YES
scope_reason: 在不改变 Prompt 行为的前提下，把动态提示词装配从 Tauri 根入口迁入 AG 自有 Prompt Runtime 模块
```

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-25 | v1.0 | 人类确认终止并替换原活动 Scope，建立提示词装配行为等价迁移 Scope。 |
| 2026-08-25 | v1.1 | 动态装配迁入 `ag/prompt_runtime.rs`；定向测试、前端全量和治理审计通过，Rust 全量被范围外 LibreOffice Worker signal=6 阻断。 |

## Task Runtime Report

```yaml
status: BLOCKED
changed_files:
  - src-tauri/src/lib.rs
  - src-tauri/src/ag/mod.rs
  - src-tauri/src/ag/prompt_runtime.rs
  - docs/00_core/A-CORE-C-R-01_ADUS.md
  - .governance/issues/AG-PROMPT-RUNTIME-EXTRACT-01.md
  - .governance/active_scope.json
validation:
  targeted_rust:
    command: cargo test --manifest-path src-tauri/Cargo.toml prompt_runtime --lib
    result: 7 passed, 0 failed
  frontend_full:
    command: npm test
    result: 589 passed, 0 failed
  rust_full:
    command: cargo test --manifest-path src-tauri/Cargo.toml
    result: 354 passed, 1 failed
    failure: doc::libreoffice_runtime::tests::docx_to_pdf_end_to_end_when_libreoffice_available — DOC_PREVIEW_WORKER_CRASH signal=6
  blocker_recheck:
    result: 同一 LibreOffice 用例定向复跑仍以 signal=6 失败
  governance_generate: passed
  governance_audit: passed
  formatting:
    prompt_runtime_rustfmt_check: passed
    scoped_git_diff_check: passed
unresolved_items:
  - 必选 Rust 全量测试存在范围外 DOC/LibreOffice Worker signal=6；本 Scope 禁止修改 src-tauri/src/doc/**，因此不得进入 READY。
```
