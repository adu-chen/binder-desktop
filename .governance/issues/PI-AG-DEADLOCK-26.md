# Issue Trace · PI-AG-DEADLOCK-26 TaskRun 锚点注入时自锁

| 项 | 内容 |
|---|---|
| 状态 | READY（自动化通过；真实复验待执行） |
| 严重级别 | P0（回合永久卡死，无 provider 请求） |
| 发现来源 | 2026-11-01 10:08 `t-1793549323867-3` + 进程 sample |
| 影响模块 | AG |
| 人类确认 | YES（“检查日志，好像卡死了”） |

## 1. 现场事实

- 第 3 回合 `t-1793549323867-3` 只有 3 个事件：
  `turn_started`、`transport_protocol_frozen`、`reference_projection`；
- 之后 14 分钟无 `gate_requested`/`provider_started`；
- `agent_turns.status=draft`，TaskRun active；
- 进程 sample：
  - 线程 A：`run_agent_turn → TurnEventEmitter::record_internal → Mutex::lock` 等待；
  - 线程 B：`route_accept_draft_diff → lock_db → Mutex::lock` 等待；
  - 说明 DB Mutex 被持有，A 正在等同一把锁。

## 2. 根因

`run_agent_turn` 的 TaskRun 锚点注入块：

```rust
if let Ok(conn) = state.db.lock() {
    ...
    event_emitter.record_internal("task_anchors_injected", ...)?;
    ...
}
```

`record_internal` 内部再次 `state.db.lock()`；同一线程持有 `conn` 又等待同一把非重入 Mutex → 自锁。
第 3 回合有 verified anchors，才进入该分支，所以前两轮正常，第三轮卡死。

## 3. 修复

- 在 `record_internal` 前先 `drop(conn)`；
- 提前计算 `anchor_chunk_count`，不再在事件里借用 `conn`；
- 锚点消息构造继续使用已收集的 `anchors`，不依赖 DB guard。

## 4. 验证

- Rust 539 + 5 + 4；前端 689；`tsc`/`governance:audit` 通过；
- 真实复验待执行。

## 5. 未解决

- 任何“持 DB 锁时记录事件”的路径都应加静态/评审约束；本处是唯一发现点。
- TaskRun 调度器仍是前端过渡版。
