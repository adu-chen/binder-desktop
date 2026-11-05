# Issue Trace · PI-AG-STREAM-17 大型工具参数流导致 GW_STREAM_ERR

| 项 | 内容 |
|---|---|
| 状态 | READY（自动化通过；真实复验待执行） |
| 严重级别 | P1（长任务在写入阶段被上游流中断） |
| 发现来源 | 2026-11-01 03:23 `t-1793524981237-1` |
| 影响模块 | AG / GW |
| 人类确认 | YES（“仍然中断”） |

## 1. 现场事实

- 回合 03:23:01 开始，共 6 轮模型调用。
- 第 1–5 轮：读取/搜索/结构读取；其中 `read_doc(view=structure)` 返回 33,250 字符，
  超过 12,000 字符上限被拒（`AG_TOOL_RESULT_TOO_LARGE`）；另有两次控制封套/游标错误。
- 第 6 轮：模型开始生成大型工具参数（`tool_arguments` 进度 60 次，持续约 519 秒），
  始终没有收到完整 tool_calls；上游以 `GW_STREAM_ERR` 结束。
- 本轮**没有生成任何 diff**，也没有 TaskRun；因此不是 diff 卡阻断。
- `GW_STREAM_ERR` 在已有可见输出时不会重试，TaskRun/回合直接 error。

## 2. 根因判断

- 直接原因：上游模型流在生成超大工具参数时异常结束（provider/gateway 侧），
  客户端只看到 60 个 `tool_arguments` 进度后 `GW_STREAM_ERR`。
- 放大因素：为了让模型一次写完 4000+3000 字，单次 `write_content` 允许无限块；
  模型倾向一次提交大批量（此前实测 90 块），参数流很长，增加上游超时/断流概率。
- 次要因素：模型先用 `read_doc(view=structure)` 读整篇，结果 33k 超限，浪费轮次与上下文。

## 3. 修复

- 恢复**单次调用载荷边界**（不是执行次数）：`write_content` schema `maxItems=24`，
  校验单次 >24 块拒绝；提示词明确“单次最多 24 块/12000 字符，超过拆多次，
  后续批次可在同一回合继续，不因拆批等待用户”。
- 工具不再在 proposal 后冻结（前序修复），所以拆批不会阻断任务。
- 提示词保留：`read_doc(view=structure)` 结果过大时改用 `list_blocks`/`blocks` 分页；
  `AG_TOOL_RESULT_TOO_LARGE` 是结果边界，不是任务失败。

## 4. 验证

- `cargo test`：537 lib + 5 + 4，0 failed。
- 前端 688 用例、`tsc`、`governance:audit` 通过。
- 真实 4000+3000 复验待执行。

## 5. 未解决

- `GW_STREAM_ERR` 是上游/网关错误；单次载荷边界只能降低概率，不能保证上游不中断。
  需要网关侧记录 provider 原始错误/耗时，或对 `GW_STREAM_ERR` 做显式重试。
- 若上游在 500 秒内无法完成一次大型参数流，应进一步拆小单次正文（如 12–16 块）。
