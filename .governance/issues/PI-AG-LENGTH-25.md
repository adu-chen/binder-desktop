# Issue Trace · PI-AG-LENGTH-25 工具参数长度截断与分批预算

| 项 | 内容 |
|---|---|
| 状态 | READY（自动化通过；真实复验待执行） |
| 严重级别 | P1（单次工具参数撞 16,384 token，JSON 截断，回合失败） |
| 发现来源 | 2026-11-01 09:28 `t-1793546552854-1` |
| 影响模块 | AG / GW / Prompt |
| 人类确认 | YES（“按建议方案修复”） |

## 1. 现场事实

- 第 5 轮：71.5s、16,330 个 tool argument delta；
- `stopReason=length`，`completionTokens=16,384 = requestedMaxTokens`；
- `pendingToolArguments.argumentsUtf8Bytes=49,964`，`jsonValid=false`；
- 错误 `AG_GW_STREAM_INCOMPLETE`，无 diff、无 TaskRun。

## 2. 根因

- 模型只看到“24 块/12,000 字符”硬上限，不知道 token/JSON 包装预算；
- 初始回合没有 `[CHUNK BUDGET]`（此前只给 preview continuation）；
- 24 被当成目标批次，而不是硬上限；
- `AG_GW_STREAM_INCOMPLETE(length)` 不在重试谓词内，直接终止。

## 3. 修复

- `turn_prompt` 对所有回合注入 `[CHUNK BUDGET]`：
  - 单次输出约 `capacity.output_tokens`；
  - 建议 6–8 块、约 `output_tokens/8` 个中文字符；
  - 24 块是硬上限；参数 JSON/base/控制封套同样占预算；
  - 长文按章节拆批，同回合继续。
- `write_content` / `propose_edit` 描述增加“6–8 块/批、参数 JSON 落在单次预算内”。
- `stream_length_truncation`：识别 `AG_GW_STREAM_INCOMPLETE` + 长度/`length`/`max_tokens`；
- 重试谓词纳入 length；无可见输出、attempt<2 时自动重试；
- 重试前追加 `[LENGTH RETRY]` 系统消息：本批最多 6 块/1500 字，不重复已生成内容。

## 4. 验证

- Rust 539 + 5 + 4；长度测试改为断言 2 次请求（一次重试）；
- 前端 689、`tsc`、`governance:audit` 通过；
- 真实长文复验待执行。

## 5. 未解决

- 重试仍失败则终结；不会无限重试。
- 24 卡分组优化仍待做。
