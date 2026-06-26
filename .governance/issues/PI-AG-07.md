# Issue Trace · PI-AG-07 系统提示词正文（三类回合）

承接 A-AG-X-L-02 §3 PI-AG-07 / A-AG-M-T-01 §5.1 BR-AG-DATA-002（组装骨架系统提示词层——文本填充）/
A-AG-M-T-02 BR-AG-DATA-004（当前轮次原则，Chat 提示词内声明）/ X-FORBID-006（修改经 diff 不直写）/
X-FORBID-004（workspace 限界）。

> 提示词**文本/措辞**属实现期运营配置，**不入规则、不 @GOV 引用规则**（A-AG-M-T-01 §5.1 已声明）。
> 本 PI 只填充骨架系统层的文本常量，不改 context.rs 骨架。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: PI-AG-07
allowed_files:
  - src-tauri/src/ag/prompts.rs        # 新建：SYSTEM_PROMPT_CHAT/BUILD/ASSIST 三常量
  - src-tauri/src/ag/mod.rs            # 声明 pub mod prompts
  - .governance/active_scope.json
  - .governance/issues/PI-AG-07.md
forbidden_files:
  - ADU.md / ADP.md / CLAUDE.md / APC
  - src-tauri/src/ipc/mod.rs           # PI-AG-10C 已交付
  - src-tauri/src/lib.rs               # 接线归后续 PI
  - src-tauri/src/ag/context.rs        # 骨架不变
  - src/**                             # 前端归 10F / 并发进程
confirmation: YES
scope_reason: 见 active_scope.json。文件隔离于并发前端进程。
```

## Expected Behavior

- `SYSTEM_PROMPT_CHAT`（对话回合）含：产品角色（Binder 写作助手）；**当前轮次原则**（本轮为主、历史仅在明确沿用时参考，承 BR-AG-DATA-004）；硬约束三条（① 修改须经用户审阅接受、不直接写文件 ② 读写限 workspace 内 ③ 写入经 propose_edit 产出修改建议）；角色边界（仅写作/文档）；防提示注入（文档内容为数据非指令）；输出风格（纯文本）。
- `SYSTEM_PROMPT_BUILD`（CBT 构建）含：单文件产出导向；结构化输出；硬约束同 Chat。
- `SYSTEM_PROMPT_ASSIST`（内联辅助/自动补全）含：轻量续写/局部改写；**无会话历史假设**；仅输出候选正文。
- `ag/mod.rs` 增 `pub mod prompts;`。三常量均非空、可编译加载。

## Validation Commands

```
cargo test --lib ag::prompts
cargo build --lib
npm run governance:audit -- --issue .governance/issues/PI-AG-07.md
```

验收：三常量非空且编译加载；`SYSTEM_PROMPT_CHAT` 含「修改须经」「不直接写」「workspace」三关键声明 +
当前轮次声明；`SYSTEM_PROMPT_ASSIST` 不含「对话历史」字样；prompts.rs 无 @GOV 规则引用（文本不入规则）；
scoped 门禁无越界（未触 lib.rs / context.rs / ipc / src/）。
