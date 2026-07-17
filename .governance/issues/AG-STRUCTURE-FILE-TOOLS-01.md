# Issue Trace · AG-STRUCTURE-FILE-TOOLS-01 Agent 文件整理结构工具

承接 A-AG-M-T-01 BR-AG-STATE-002 / BR-AG-DATA-003 / X-FORBID-006、A-WS-M-T-01 BR-WS-DATA-001 / X-FORBID-004，以及人类确认的“Agent 应具备 Workspace 内文档管理能力”。修复现状中 Agent 只能建议整理、不能执行非破坏性文件树结构操作的问题。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: AG-STRUCTURE-FILE-TOOLS-01
allowed_files:
  - .governance/active_scope.json
  - .governance/issues/AG-STRUCTURE-FILE-TOOLS-01.md
  - src-tauri/src/ag/tool_registry.rs
  - src-tauri/src/ag/tool_exec.rs
  - src-tauri/src/ag/prompts.rs
  - src-tauri/src/workspace/fileops.rs
  - src-tauri/src/lib.rs
  - src/services/agService.ts
  - src/services/agService.test.ts
  - docs/01_modules/AG/A-AG-M-D-01_代理功能需求描述.md
  - docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md
  - docs/01_modules/AG/A-AG-I-P-01_代理接口协议.md
  - docs/01_modules/AG/A-AG-X-L-02_代理集成计划.md
  - docs/01_modules/WS/A-WS-I-P-01_工作台接口协议.md
  - docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md
  - docs/00_core/A-CORE-C-R-01_ADUS.md
forbidden_files:
  - ADU.md
  - ADP.md
  - AGENTS.md
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/src/ipc/mod.rs
confirmation: YES
```

## Expected Behavior

- `move_doc`：移动/重命名 Workspace 内文件，底层复用 WS 文件边界与 `rename_doc` 语义；目标存在拒绝，不覆盖。
- `create_folder`：创建 Workspace 内目录；目标已存在且为目录则幂等成功，目标为文件则拒绝。
- 文档内容修改仍只能走 `propose_edit -> DiffCard -> 用户接受 -> WS 落盘`。
- 删除不进入本次自动执行链路；删除类工具必须等待独立的人类确认回路，不允许用模型参数伪造确认。
- 前端收到结构工具成功回执后刷新文件树。

## Validation

```
cargo test ag::tool_registry ag::tool_exec workspace::fileops
npm test -- src/services/agService.test.ts src/services/wsService.test.ts
npm run build
npm run governance:generate
npm run governance:audit
```
