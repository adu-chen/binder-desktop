# Issue Trace · PI-AG-09C 意图沿用判定（完整 PI-AG-09 第②步，BR-AG-DATA-004）

承接 A-AG-M-T-02 BR-AG-DATA-004（当前轮次原则 / 意图沿用判定，TERM-AG-005 IntentCarryover）/
R-AG-X-R-01 §4.3。默认弱化历史、显式沿用才提权——在 PI-AG-09B 历史之上加判定信号。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: PI-AG-09C
allowed_files:
  - src-tauri/src/ag/context.rs   # Carryover 枚举 + decide_intent_carryover（dev 启发式）
  - src-tauri/src/lib.rs          # ag_start_turn 据判定注入【本轮意图】系统提示（当前轮前）
  - .governance/active_scope.json / issues/PI-AG-09C.md
forbidden_files: ADU/ADP/CLAUDE/APC / ipc/mod.rs / edStore
confirmation: YES
```

## Expected Behavior

- `Carryover { None, Refine, Select, ReferenceTarget }`（ContinueTask 需任务态，延后）。
- `decide_intent_carryover(prompt, has_history) -> Carryover`（dev 启发式，**识别策略延实现期、机制锁定**）：
  无历史→None；修正追加词（再…/改成/短一点/把刚才…）→Refine；显式选择（第N个/按刚才/用上面）→Select；
  指代回指/续写（继续/接着/它/那个/上面/刚才）→ReferenceTarget；否则 None。
- `ag_start_turn`：据 carryover 在**当前轮 user 前**注入一条【本轮意图】system——None=「新请求，历史仅作背景、
  不要凌驾当前」；Refine/Select/ReferenceTarget=「据相应历史意图继续」。即 M-T-02 §4.3 默认弱化、显式沿用提权。

## Validation Commands

```
cargo build --lib && cargo test --lib 'ag::context'
# 端到端：「继续」「再短一点」触发沿用提示；新话题不提示历史优先（人类自验）
```

验收：cargo build + ag::context 测试过（含 carryover 判定）；None/Refine/Select/ReferenceTarget 分类正确；
提示注入位置在当前轮前。

## 后续

- 识别升级（小模型/更细规则）；ContinueTask（接 agent 任务态）；③REF/AST/KB 供给。
