---
文档编号：PRV-M-T-01
文档状态：A
负责模块：PRV
文档职责：PRV 预览与输出模块级 L2 技术设计（规则 SSOT）——只读硬边界执行规则、预览管线归属（联合 DOC 解除 DOC-03/PRV-01）、音视频解码边界、打印导出管线、状态机豁免复核；承接 REQ-PRV-001~005
上游约束：A-PRV-M-D-01_预览输出功能需求描述.md（REQ-PRV-001~005、PRV-01~05）、A-CORE-C-P-01_APC.md（§4.1/§6/§9）、A-CORE-X-L-01_总体开发路线图.md（§2 v1 冻结）、A-DOC-M-T-01_文档转换技术设计.md（BR-DOC-DATA-001 双链分离、X-SEQ-004、DOC-03）、A-ED-M-T-01_编辑器技术设计.md（BR-ED-DATA-003 打印分工）、A-WS-M-T-01_工作台技术设计.md（X-FORBID-004）、ADU.md（§3）
直接承接：本文 RULE/CHAIN 为 PRV 规则 SSOT；预览转换归属决策回写 A-DOC-M-T-01 v1.1（DOC-03 解除）
使用边界：本文是 PRV 规则 SSOT；文档格式转换归 DOC（PRV 消费）；编辑归 ED；文件经 WS；导出格式清单具体项延实现期；无状态机（豁免复核 §5）
变更要求：管线归属变更须与 DOC 共同复核（X-SEQ-004）；只读边界变更涉 v1 冻结须先经 APC 授权；无 TERM 注册（§6 说明）
治理成熟度：draft
---

# Binder 预览与输出技术设计（PRV · L2 技术设计）

## §1 文档定位

```
REQ-CORE-004/009 + v1 冻结 → REQ-PRV-001~005（PRV L1）→ 本文（SSOT）→ 代码(@GOV)
预览转换 ←消费← DOC 预览链（归属决策 §2，回写 A-DOC-M-T-01 解除 DOC-03）
```

无状态机（豁免复核 §5）。

## §2 预览管线归属（G2 设计决策，解除 PRV-01 / DOC-03）

**决策**（联合 DOC，门评时供人类复核）：

| 内容类型 | 转换/解码 | 归属与理由 |
|---|---|---|
| docx | LibreOffice→PDF | **DOC 预览链**（既有，A-DOC-M-T-01 §2） |
| Excel / PPT | LibreOffice→PDF | **DOC 预览链扩展**——LibreOffice 能力已布点于 DOC，复用避免双布点、统一受 X-SEQ-004（二进制转换唯一通道）约束 |
| 音视频 | 系统解码（macOS AVFoundation） | **PRV 自带呈现**——播放解码非"文档格式转换"职责，不经 DOC；系统解码优先、不自带解码器 |

<!-- CHAIN
chain_id: PRV-VIEW
主责模块: PRV
status: active
risk_tier: P1
-->

<!-- RULE
rule_id: BR-PRV-DATA-002
主链路: PRV-VIEW
域: DATA
-->
预览管线归属必须遵守：docx/Excel/PPT 等文档格式的预览转换一律经 DOC 预览链（X-SEQ-004，PRV 不
内置文档转换库）；音视频经本地系统解码呈现（系统能力优先，不自带解码器；不支持格式明确提示）；
预览/打印/导出全链离线可用、内容不上行；转换/解码失败呈现明确失败态，不静默空白；导出文件落点
限 workspace 内（X-FORBID-004），打印走系统打印服务。

## §3 只读硬边界（PRV-VIEW）

<!-- RULE
rule_id: BR-PRV-DATA-001
主链路: PRV-VIEW
域: DATA
-->
预览为纯只读呈现：预览管线不得提供任何内容修改交互、不产生文档模型变更与脏态；**Excel/PPT 在
全产品范围不存在编辑入口**（v1 冻结执行规则，G4/G5 强制核验项）；预览与编辑链无功能黏连
（承 BR-DOC-DATA-001 双链分离）；编辑文档的打印走 ED 打印友好分页呈现 + PRV 输出管线
（BR-ED-DATA-003 分工）。

## §4 跨模块约束（复用，不新增）

X-SEQ-004（转换唯一通道）、X-FORBID-004（文件限界）。不新增 CONSTRAINT。

## §5 状态机豁免复核（解除 PRV-04）

**结论：豁免成立。** 预览（请求→转换/解码→呈现/失败态）、打印、导出均为请求-响应式单次操作，
无多步异步生命周期与跨模块状态耦合（APC §6 口径 b）。**豁免失效触发条件**：引入预览转换队列/
缓存重建编排、批量导出任务编排，任一出现按 X-SEQ-003 建机登记总图。

## §6 术语说明（不注册 TERM）

预览、导出、打印为通用技术词（非跨 session 易漂移的核心业务概念，ADU §3.2 不注册）；
样式子集（StyleSubset）归 DOC、内容块归 ED，引用不抢注。

## §7 需求到规则映射（REQ_MAP）

<!-- REQ_MAP
req_id: REQ-PRV-001
target_type: CHAIN
target_id: PRV-VIEW
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-PRV-001
target_type: RULE
target_id: BR-PRV-DATA-002
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-PRV-002
target_type: RULE
target_id: BR-PRV-DATA-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-PRV-003
target_type: RULE
target_id: BR-PRV-DATA-002
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-PRV-004
target_type: RULE
target_id: BR-PRV-DATA-002
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-PRV-005
target_type: RULE
target_id: BR-PRV-DATA-002
status: mapped
-->

## §8 测试与验收

- **只读硬边界**（BR-PRV-DATA-001）：预览无修改交互、不产生脏态；**全产品无 Excel/PPT 编辑入口**
  （静态+交互核验，G4/G5 强制）。
- **管线归属**（BR-PRV-DATA-002）：PRV 无文档转换库（静态可查）；docx/Excel/PPT 经 DOC 预览链；
  音视频系统解码、不支持格式明确提示；失败态明确；断网全链可用；导出落 workspace 内。
- 量化 NFR（预览首屏延迟）延 G5（PRV-05）。

## §9 技术债务与盲区（诚实声明）

- Excel/PPT 经 LibreOffice 的保真度与超大文件表现需实现期实测（不达另议管线，须回写本文与 DOC）。
- 音视频支持格式清单随系统解码能力在实现期界定（PRV-02 余项）。
- 导出格式清单与打印管线细节延实现期（PRV-03 余项）。
- 预览缓存策略延实现期（引入缓存重建编排则触发 §5 豁免失效条件）。

## §10 后续动作

1. 注册（generate）：PRV-VIEW 链路、BR-PRV-DATA-001/002、REQ_MAP；A-DOC-M-T-01 同步回写
   DOC-03 解除（v1.1）。
2. 待人类宣布 PRV G2 闭合——至此 P4 五模块 L2 齐备。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-11 | v1.0 | 初版（PRV G2，P4 G2 收尾件）：预览管线归属决策（Excel/PPT 归 DOC 预览链扩展、音视频系统解码归 PRV，解除 PRV-01/DOC-03，门评供人类复核）；只读硬边界执行规则（全产品无 Excel/PPT 编辑入口为 G4/G5 强制核验项）；状态机豁免复核成立并列失效条件（解除 PRV-04）；注册链路 PRV-VIEW、规则 BR-PRV-DATA-001/002；无 TERM（通用技术词说明）；REQ_MAP（REQ-PRV-001~005） |
