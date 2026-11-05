# Issue Trace · PI-AG-STREAM-19 网关连接重试与客户端失败重试

| 项 | 内容 |
|---|---|
| 状态 | READY（网关已重启；自动化通过；真实复验待执行） |
| 严重级别 | P1（瞬时 TCP 连接失败直接中断整个回合） |
| 发现来源 | 2026-11-01 04:26 `t-1793528804462-1` 网关日志 |
| 影响模块 | AG / GW |
| 人类确认 | YES（“确认”） |

## 1. 现场事实

- 网关 04:26:44 收到请求，10.5 秒后 `upstream_failed`，`causeCode=UND_ERR_CONNECT_TIMEOUT`；
- 没有收到上游 headers/首字节；
- 客户端立即收到 `GW_STREAM_ERR`，回合在 11 秒内 error；
- 网关 `/health` 正常；随后 curl 直连/代理访问 DeepSeek 均可达（401 无 key）；
- 因此是**瞬时连接失败 + 无重试**，不是模型慢、不是 diff 卡、不是额度。

## 2. 根因

1. `providers.mjs` 的 `fetch` 只调用一次；Undici 默认 10s connect timeout 一到就把
   `UND_ERR_CONNECT_TIMEOUT` 抛给 `server.mjs`，直接返回 `GW_STREAM_ERR`。
2. `run_provider_round` 把网关 `RoundOutcome::Failed` 包装成 `ProviderRoundResult::Ready`，
   绕过了 `stream_timeout_is_retryable`；客户端虽然知道 `retriable=true`，但不会重试。

## 3. 修复

- 网关新增 `fetchUpstream`：对 `UND_ERR_CONNECT_TIMEOUT` / `ECONNRESET` / 无 code 的
  TypeError 做**一次**连接重试（300ms 退避，客户端取消则不重试）；OpenAI 兼容与 Anthropic 共用。
- `run_provider_round` 将 `RoundOutcome::Failed` 转为 `ProviderRoundResult::Failed`，
  使既有 `stream_timeout_is_retryable` 生效（无可见输出、attempt<2）。
- 不新增执行次数配额；仍保留 round deadline / stream idle 时间边界。

## 4. 验证

- `node --check providers.mjs` 通过；网关重启后 `/health` ok，build 更新为 `a5fddb…`。
- `cargo test`：537 lib + 5 + 4，0 failed。
- 前端 688、`tsc`、`governance:audit` 通过。
- 真实 4000+3000 复验待执行。

## 5. 未解决

- 连续两次 connect timeout 仍会失败；如需更强韧性，可做指数退避多次，但会增加等待。
- 客户端重试只在无可见输出时发生；有可见片段后仍不重试，避免重复正文。
