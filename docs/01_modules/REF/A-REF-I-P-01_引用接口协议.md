---
文档编号：REF-I-P-01
文档状态：A
负责模块：REF
文档职责：REF 引用模块级接口协议——@mention 候选解析、引用锚校验、注入载荷组装接口
上游约束：A-SYS-I-P-02_IPC协议.md（骨架）、A-REF-M-T-01_引用技术设计.md（BR-REF-* 规则、REF-INJECT、RefInjection）、A-WS-I-P-01（read/search 复用）、A-ED-I-P-01（locate_block 锚校验）、A-SYS-I-P-01（RefInjection）、ADU.md
直接承接：实现期 REF command 注册；候选/读取经 WS，锚校验经 ED；注入载荷供 AG/CBT 消费
使用边界：声明语义级签名；文件读取经 WS、锚校验经 ED（不重复）；不新增注册项；REF 无状态机（豁免成立）
变更要求：注入载荷接口变更回写 A-SYS-I-P-01（RefInjection）并复核 AG/CBT；锚校验联动 ED blockId 契约
治理成熟度：draft
---

# Binder 引用接口协议（REF · L2 接口层）

## §1 文档定位

```
A-SYS-I-P-02（骨架）→ 本文（REF 解析/校验/组装接口）→ 实现 + @GOV(追溯 A-REF-M-T-01)
候选/读取 → ws.list_docs/ws.read_doc/ws.search；锚校验 → ed.locate_block；载荷 → AG/CBT 消费
```

## §2 接口清单（REF-INJECT）

| command | 参数（语义） | 结果 | 特有错误 | 归属 |
|---|---|---|---|---|
| `ref.list_candidates` | workspace_id、查询前缀 | 候选列表（文档/内容块/资源，限界 workspace 内） | — | BR-REF-DATA-001（无悬空，目标不可达不入候选） |
| `ref.resolve_anchor` | 选定目标 | ReferenceAnchor（路径级 DocRef 或块级 DocRef+blockId） | 目标不可达→明确失败（不产生悬空引用） | BR-REF-DATA-001 |
| `ref.verify_anchor` | ReferenceAnchor | 有效 / **anchor_invalid**（明确失效信号） | — | BR-REF-VERIFY-001（经 ed.locate_block；检测不可得保守失效） |
| `ref.build_injection` | ReferenceAnchor 集 | RefInjection 集（content 读取时点 + origin_label 可溯源） | 锚失效→剔除并回报（不注入过期/空内容） | BR-REF-DATA-002（经 ws.read_doc；裁剪归 AG） |

> 候选列举/标记渲染依托 ED 扩展点（X-FORBID-005，前端进程内，不在本 command 表）；REF 不直接
> 操纵编辑器状态。KB 条目/AST 记忆是否纳入候选待 REF-01（联动 KB/AST，本文 v1 仅 workspace 目标）。

## §2.1 前端进程内 ReferencePayload 协议（REF-INJECT）

前端拖拽/粘贴引用不经 Tauri command，使用进程内 `DataTransfer` / clipboard MIME 协议与 Composer
输入面交互：

| payload kind | 必要字段 | 来源入口 | 消费 |
|---|---|---|---|
| `doc` | `relPath`, `label?` | 文件树拖拽、编辑器标签拖拽、@mention 文档候选 | 发送时可经 WS 读取当前内容 |
| `selection` | `relPath`, `blockId`, `start?`, `end?`, `text`, `label?` | 编辑器选区复制/拖拽 | 发送时优先消费选区文本快照，并保留锚供校验 |
| `session` | `sessionId`, `title`, `messages[]` | 代理会话标签拖拽 | 发送时作为显式会话上下文快照 |
| `memory` | `entryId?`, `title?`, `content` | AST 记忆条目拖拽 | 发送时作为显式记忆快照 |
| `knowledge` | `entryId?`, `title?`, `content`, `sourceRef?` | KB 知识条目拖拽 | 发送时作为显式知识快照 |
| `template` | `templateId?`, `name`, `content` | TPL 模板条目拖拽 | 发送时作为显式模板快照 |
| `url` | `url`, `title?` | 外部 URL 粘贴/拖拽 | 仅注入链接，不自动抓取外部内容 |
| `long_text` | `text`, `charCount` | 外部纯文本粘贴超过阈值 | 作为长文本快照注入，避免直接撑大输入正文 |

`ReferencePayload` 在 Composer 内插入 `ReferenceToken`，发送时作为 `ag.start_turn.references` 传给 AG；
旧 `mentions`/`block_refs` 参数仅保留兼容，不再作为新前端路径的主通道。

## §3 测试与验收

- 候选限界 workspace、目标不可达不入候选；resolve 悬空失败；verify 失效信号（含检测不可得保守）；
  build_injection 内容读取时点一致、来源可溯、失效剔除不注入；与 AG（消费）/ED（锚）G5 联调。
- 前端进程内协议：每类 ReferencePayload 可从对应入口进入 Composer 并生成 ReferenceToken；超过阈值的
  外部纯文本生成 `long_text`；发送时传 `references`，发送后草稿 chip/tray 清空。

## §4 债务

- KB/AST 纳入候选（REF-01）的解析接口待其 I-P 联合扩展。
- 注入载荷若需跨进程结构化随实现期与 SYS 确认（当前 RefInjection 基线已登记 A-SYS-I-P-01）。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（实现前置件）：REF 接口协议——list_candidates/resolve_anchor/verify_anchor/build_injection 四接口，限界+无悬空+失效保守+读取时点一致；零新增注册项；债务（KB/AST 纳入候选待联合） |
| 2026-07-08 | v1.1 | 新增前端进程内 ReferencePayload 协议：声明 doc/selection/session/memory/knowledge/template/url/long_text 八类 payload 字段、来源入口和 AG 消费路径；明确新路径通过 `ag.start_turn.references`，旧 mentions/block_refs 仅兼容 |
