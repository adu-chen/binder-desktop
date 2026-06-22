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

## §3 测试与验收

- 候选限界 workspace、目标不可达不入候选；resolve 悬空失败；verify 失效信号（含检测不可得保守）；
  build_injection 内容读取时点一致、来源可溯、失效剔除不注入；与 AG（消费）/ED（锚）G5 联调。

## §4 债务

- KB/AST 纳入候选（REF-01）的解析接口待其 I-P 联合扩展。
- 注入载荷若需跨进程结构化随实现期与 SYS 确认（当前 RefInjection 基线已登记 A-SYS-I-P-01）。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（实现前置件）：REF 接口协议——list_candidates/resolve_anchor/verify_anchor/build_injection 四接口，限界+无悬空+失效保守+读取时点一致；零新增注册项；债务（KB/AST 纳入候选待联合） |
