# Issue Trace · PI-AG-STREAM-18 客户端流空闲 90s 误杀慢速上游

| 项 | 内容 |
|---|---|
| 状态 | READY（网关已重启；自动化通过；真实复验待执行） |
| 严重级别 | P1（上游仍在生成 tool_arguments 时客户端先超时） |
| 发现来源 | 2026-11-01 04:13 `t-1793528007617-1` + `cloud/dev-gateway/.data/gateway.log` |
| 影响模块 | AG / GW |
| 人类确认 | YES（“如果网关有问题，就重启网关。如果逻辑问题寻找逻辑根因”） |

## 1. 现场事实

- 回合第 1 轮：网关 04:13:28 收到请求，10.1s 收到上游 headers，随后 `tool_arguments` 进度；
  04:14:35（67s）后只有 106 个 delta；最后一次 progress 在 57.6s；
- 04:16:05（157s）**客户端主动断开**，网关记录 `client_disconnected` / `AbortError`；
- 客户端记录 `AG_GW_STREAM_TIMEOUT`（12 chunks / 63 bytes）。
- 因此不是网关崩溃，也不是 429；是客户端 **90s stream idle** 在上游慢速生成时先触发。
- 网关日志此前还有一次 `ECONNRESET`（`t-1793524981237-1` 第 6 轮，519s），属于上游连接重置。

## 2. 根因

- `ModelCapacity` 对 deepseek-flash 的 `stream_idle_deadline_secs=90`；上游在生成
  tool_arguments 时可能 >90s 无增量（日志显示 57s→157s 无进度），客户端判超时并断开。
- 上游并未返回错误；客户端过早放弃，导致 `AG_GW_STREAM_TIMEOUT`。
- 1200s round deadline 不是问题；90s idle 太短。

## 3. 处置

- **重启 dev-gateway**（旧进程 29921 → 新进程 22126；`/health` 返回 ok）。
- deepseek-flash `stream_idle_deadline_secs` **90 → 300**；round deadline 保持 1200s。
- 这是时间边界，不是执行次数额度；仍保留 round deadline 防止无限等待。
- 不同步改工具/模型轮次。

## 4. 验证

- `cargo test`：537 lib + 5 + 4，0 failed；deadline 断言更新为 300s。
- 前端 688、`tsc`、`governance:audit` 通过。
- 真实 4000+3000 复验待执行。

## 5. 未解决

- 若上游 300s 仍无任何增量，客户端仍会超时；需要网关按 provider 记录原始错误/耗时，
  或对 `AG_GW_STREAM_TIMEOUT` 做可配置重试。
- `GW_STREAM_ERR`（ECONNRESET）仍可能由上游网络触发，与 idle 超时不同。
