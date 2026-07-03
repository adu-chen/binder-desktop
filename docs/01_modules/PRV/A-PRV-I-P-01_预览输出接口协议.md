---
文档编号：PRV-I-P-01
文档状态：A
负责模块：PRV
文档职责：PRV 预览与输出模块级接口协议——多格式预览 command（docx/Excel/PPT/音视频）、打印、导出
上游约束：A-SYS-I-P-02_IPC协议.md（骨架）、A-PRV-M-T-01_预览输出技术设计.md（BR-PRV-* 规则、PRV-VIEW、只读硬边界、管线归属决策）、A-DOC-I-P-01（render_preview 消费）、A-WS-I-P-01（文件读取/导出落点）、ADU.md
直接承接：实现期 PRV command 注册；docx/Excel/PPT 预览经 DOC（render_preview）；音视频系统解码；导出经 WS
使用边界：声明语义级签名；文档转换归 DOC（消费不重复）；文件经 WS；不新增注册项；只读硬边界（无编辑入口）；无状态机（豁免成立）
变更要求：只读边界变更涉 v1 冻结须 APC 授权；管线归属变更复核 DOC
治理成熟度：draft
---

# Binder 预览输出接口协议（PRV · L2 接口层）

## §1 文档定位

```
A-SYS-I-P-02（骨架）→ 本文（PRV 预览/打印/导出接口）→ 实现 + @GOV(追溯 A-PRV-M-T-01)
docx/Excel/PPT 预览 → doc.render_preview（DOC 预览链）；音视频 → 系统解码；导出落点 → WS
```

## §2 接口清单（PRV-VIEW / BR-PRV-DATA-001/002）

| command | 参数（语义） | 结果 | 特有错误 | 归属 |
|---|---|---|---|---|
| `prv.preview` | DocRef、格式 | 只读呈现资源（docx/Excel/PPT→PDF 经 DOC；音视频→播放句柄系统解码） | render_failed/decode_failed→明确失败态（不静默空白）；不支持格式→明确提示 | BR-PRV-DATA-002（管线归属决策）；**只读无编辑交互** |
| `prv.print` | DocRef 或预览句柄 | 打印任务（系统打印服务） | — | BR-PRV-DATA-001（编辑文档打印分页呈现归 ED） |
| `prv.export` | DocRef、目标格式、落点（workspace 内） | 导出文件 | 越界落点拒绝（X-FORBID-004）；格式不支持 | BR-PRV-DATA-002 |
| `prv.media_src` | relPath（音视频） | 单文件 asset 协议授权 + 绝对路径（流式播放不进内存） | PRV_NOT_FOUND / PRV_SCOPE_FAILED | BR-PRV-DATA-002（既有实现补登记，2026-07-03） |
| `prv.table_data` | relPath（xlsx/xls） | {sheets:[{name,rows}], truncated} 结构化表格数据（值投影，超限截断明示） | PRV_TABLE_PARSE_FAIL / PRV_NOT_FOUND | 表格式预览（2026-07-03 #1，BR-PRV-DATA-001 只读；解析经 DOC calamine，X-SEQ-004；csv/tsv 文本由前端解析不经此） |

> **全产品无 Excel/PPT 编辑入口**（BR-PRV-DATA-001，v1 冻结 G4/G5 强制核验）；preview 不产生
> 文档模型变更与脏态；预览/打印/导出离线可用（REQ-CORE-009）。

## §3 测试与验收

- preview 只读无编辑交互（Excel/PPT 无编辑入口，静态+交互核验）；docx/Excel/PPT 经 DOC 预览链、
  音视频系统解码、失败态明确不空白；print 系统服务；export 落 workspace 内（越界拒绝）；断网全链
  可用；与 DOC G5 联调。

## §4 债务

- 音视频支持格式清单随系统解码能力实现期界定（PRV-02）；导出格式清单与打印管线（PRV-03）；
  预览缓存策略延实现期（引入缓存编排则建机复核，PRV-04）。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（实现前置件）：PRV 接口协议——preview（docx/Excel/PPT 经 DOC、音视频系统解码、失败态明确、只读无编辑）/print/export（落 workspace 内）；全产品无 Excel/PPT 编辑入口断言；零新增注册项；债务（音视频格式/导出清单/缓存策略延后） |
| 2026-07-03 | v1.1 | §2 补登记 prv.media_src（既有实现文档滞后）+ 新增 prv.table_data（表格式预览，兑现 A-PRV-M-T-01 §9 另议管线：xlsx/xls → 结构化 sheet 数据只读值投影、超限明示；csv/tsv 文本格式前端解析不经 command） |
