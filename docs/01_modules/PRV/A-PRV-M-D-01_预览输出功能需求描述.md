---
文档编号：PRV-M-D-01
文档状态：A
负责模块：PRV
文档职责：PRV 预览与输出模块级功能需求描述——声明模块级 REQ-PRV-*，承接产品级 REQ-CORE-004/009 与 v1 范围冻结（音视频预览纳入、Excel/PPT 仅预览不编辑），界定 docx 语料实测只读预览 / Excel·PPT 预览 / 音视频预览 / 打印与导出
上游约束：A-CORE-C-D-01_产品概述.md（REQ-CORE-004/009）、A-CORE-C-P-01_APC.md（§4.1 PRV 职责、§9）、A-CORE-X-L-01_总体开发路线图.md（§2 v1 范围冻结：音视频纳入、Excel/PPT 编辑确认不做）、A-SYS-C-T-01_系统总体架构.md（§4 PRV→WS）、A-DOC-M-T-01_文档转换技术设计.md（预览链 LibreOffice→PDF、BR-DOC-DATA-001 双链分离、DOC-03 归属待联合）、A-WS-M-T-01_工作台技术设计.md（X-FORBID-004）、ADU.md（§3.2）
直接承接：PRV L2 技术设计（预览呈现契约 / 打印导出管线）经 REQ_MAP 映射 RULE/CHAIN/CONSTRAINT/TERM；与 DOC 联合解除 DOC-03（Excel/PPT 等预览转换归属）
使用边界：本文是 L1 功能需求描述层，仅声明 REQ-PRV-*；**代码不得直接映射 REQ**（ADU §3.2）；docx 预览转换归 DOC（PRV 消费 PDF）；编辑归 ED；文件读取经 WS；Excel/PPT/音视频的转换/解码归属在 §7 暴露联合确认
变更要求：PRV 职责或预览范围变更（尤其涉 v1 冻结项）须先经 APC 授权再回写本文；REQ 增删按 ADU §3.2
治理成熟度：draft
---

# Binder PRV 预览与输出功能需求描述（L1）

## §1 文档定位

```
产品概述(REQ-CORE-004/009) + v1 范围冻结 → 本文(REQ-PRV-*) → REQ_MAP → PRV L2 规则(SSOT) → 代码(@GOV)
```

PRV 是**预览与输出模块**：承载多格式只读预览（docx 以 LO 语料实测为准、Excel/PPT、音视频）与打印/导出
（APC §4.1）。v1 冻结边界（2026-06-09 人类确认）：**音视频预览纳入 v1；Excel/PPT 仅预览 +
打印/导出，确认不做编辑**。PRV 依赖 WS（文件边界）并消费 DOC 预览转换。取材台账 §4.7。

## §2 模块定位与职责

| # | 职责 | 锚定 |
|---|---|---|
| P-J1 | docx 语料实测只读预览：消费 DOC 预览链（LibreOffice→PDF）只读呈现 | REQ-CORE-004、A-DOC-M-T-01 |
| P-J2 | Excel / PPT 预览：只读预览，**无编辑入口** | REQ-CORE-004、v1 冻结 |
| P-J3 | 音视频预览：workspace 内音视频资源的播放预览 | v1 冻结（纳入） |
| P-J4 | 打印与导出：预览内容的打印输出与格式导出 | REQ-CORE-004 |
| P-J5 | 离线可用：预览/打印/导出不依赖在线 | REQ-CORE-009 |

> 边界澄清：**预览与编辑完全分离**（承 DOC 双链分离决策）：PRV 只读呈现，编辑入口归 ED 且仅
> md/docx 可编辑；Excel/PPT **不存在编辑入口**（v1 冻结验收项）。docx 预览转换归 DOC；
> Excel/PPT/音视频的转换/解码归属（DOC 扩展预览链 vs PRV 自带）为 DOC-03 联合项（§7 PRV-01）。

## §3 能力范围（PRV v1）

**IN**：docx 以 LO 语料实测为准的只读预览（PDF 呈现）；Excel/PPT 只读预览；音视频播放预览；打印输出；导出
（格式清单 G2 定）；以上全部离线可用。

**OUT**：任何编辑能力（md/docx 编辑归 ED；Excel/PPT 编辑 v1 确认不做）；docx 预览转换（DOC）；
格式转换管线（DOC，归属边界 PRV-01 联合）；文件 I/O（经 WS）。

## §4 代表性流程（契约级，APC §7）

**P1 · 多格式预览**

| step | actor | input | action | output | next | exception |
|---|---|---|---|---|---|---|
| 1 | 用户 | 选定 workspace 内文件 | PRV 按格式路由预览管线（docx 经 DOC→PDF；Excel/PPT/音视频按 PRV-01 归属管线） | 只读预览呈现 | 2 | 转换/解码失败→明确失败态呈现，不静默空白 |
| 2 | 用户 | 打印/导出意图 | PRV 执行打印输出/导出 | 打印任务/导出文件 | 终态 | 导出目标在 workspace 内（越界拒绝）；失败回统一错误形状 |

> Excel/PPT 预览呈现中**不提供任何编辑交互**（v1 冻结验收：无编辑入口）。

## §5 模块级需求声明（REQ-PRV-*）

<!-- REQ
req_id: REQ-PRV-001
title: docx 语料实测只读预览
owner_module: PRV
status: active
summary: 消费 DOC 预览链（LibreOffice→PDF）对 docx 做以 LO 语料实测为准的只读预览，补偿编辑链样式子集化的查看需求；工具无效、启动、字体覆盖或转换失败均呈现明确失败态
source_doc: REQ-CORE-004、A-DOC-M-T-01（BR-DOC-DATA-001 双链分离、REQ-DOC-004）
acceptance: docx 可按 LO 语料实测结果只读预览；预览与编辑链无功能黏连；工具无效、启动、字体覆盖和转换失败态明确
open_issues: 预览缓存与刷新策略归 PRV G2
-->

<!-- REQ
req_id: REQ-PRV-002
title: Excel / PPT 预览（无编辑入口）
owner_module: PRV
status: active
summary: workspace 内 Excel/PPT 文件可只读预览并打印/导出；整个产品不存在 Excel/PPT 编辑入口（v1 冻结验收项）
source_doc: REQ-CORE-004（acceptance：Excel/PPT 无编辑入口）、路线图 §2 v1 冻结
acceptance: Excel/PPT 可预览、可打印/导出；任何入口均无编辑能力（验收可静态+交互核验）
open_issues: 预览转换归属（DOC 扩展预览链 vs PRV 自带）联合 DOC 解除 DOC-03（§7 PRV-01）
-->

<!-- REQ
req_id: REQ-PRV-003
title: 音视频预览
owner_module: PRV
status: active
summary: workspace 内音视频资源可播放预览（v1 冻结纳入）；解码失败呈现明确失败态；格式支持范围 G2 界定
source_doc: 路线图 §2 v1 冻结（音视频纳入，归 PRV）
acceptance: 常见音视频格式可在应用内播放预览；不支持格式有明确提示
open_issues: 支持格式清单与解码能力边界（系统解码 vs 自带）归 PRV G2（§7 PRV-02）
-->

<!-- REQ
req_id: REQ-PRV-004
title: 打印与导出
owner_module: PRV
status: active
summary: 预览内容可打印输出与导出（导出格式清单 G2 定）；导出文件落 workspace 内（越界拒绝）；编辑文档的打印友好分页呈现由 ED 提供、输出管线归 PRV
source_doc: REQ-CORE-004、A-ED-M-T-01（BR-ED-DATA-003 分工）
acceptance: 可打印与导出；导出落 workspace 内；失败回统一错误形状
open_issues: 导出格式清单与管线归 PRV G2（§7 PRV-03）
-->

<!-- REQ
req_id: REQ-PRV-005
title: 离线可用
owner_module: PRV
status: active
summary: 预览/打印/导出全链离线可用，不依赖在线鉴权（REQ-CORE-009：预览打印属本地功能）
source_doc: REQ-CORE-009
acceptance: 断网时预览/打印/导出可用
open_issues: 无
-->

## §6 模块边界与约束（边界闭合）

- **只读硬边界**：PRV 无编辑能力；Excel/PPT 全产品无编辑入口（v1 冻结，G4/G5 验收强制核验）。
- **双链分离承接**：docx 预览消费 DOC 预览链，与编辑链无黏连（BR-DOC-DATA-001）。
- **文件边界**：读取与导出落点均经 WS、限 workspace 内（X-FORBID-004）。
- **状态机阈值预判**：预览/打印为请求-响应式呈现，预判豁免建机，G2 复核（若引入预览缓存编排/
  长转换队列则评估建机）。
- **L4 原料边界**：取材台账 §4.7（含音视频相关 §4.14 VAL 审计参考），原料不为权威。

## §7 G1 问题暴露清单

| 编号 | 分类 | 状态 | 处置 |
|---|---|---|---|
| PRV-01 | BOUNDARY_OPEN | 显式延期 | Excel/PPT（及音视频转码如需）预览转换归属——DOC 扩展预览链 vs PRV 自带，PRV G2 联合 DOC 解除 DOC-03 |
| PRV-02 | BOUNDARY_OPEN | 显式延期 | 音视频支持格式清单与解码边界（系统解码优先）归 PRV G2 |
| PRV-03 | BOUNDARY_OPEN | 显式延期 | 导出格式清单与打印管线归 PRV G2 |
| PRV-04 | FLOW_INCOMPLETE | 显式延期 | 状态机豁免预判待 PRV G2 复核 |
| PRV-05 | ACCEPTANCE_MISSING | 部分关闭 | 定性验收已入 §5；量化指标（预览首屏延迟）延 G5 NFR |

## §8 后续动作

1. 待人类宣布 PRV G1 闭合后进入 PRV G2：预览呈现契约、Excel/PPT 转换归属（联合 DOC 解除
   DOC-03）、音视频格式边界、打印导出管线、豁免复核。
2. G2 经 REQ_MAP 映射 REQ-PRV-001~005。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-11 | v1.0 | 初版（P4，五模块 L1 收尾）：PRV 预览与输出模块级 L1；声明 REQ-PRV-001~005（docx 全保真预览 / Excel·PPT 预览无编辑入口 / 音视频预览 / 打印导出 / 离线可用），承接 REQ-CORE-004/009 与 v1 范围冻结；只读硬边界与双链分离承接；G1 暴露清单 PRV-01~05（Excel/PPT 转换归属联合 DOC-03） |
| 2026-07-21 | v1.1 | 审计修正：REQ-PRV-001 的“全保真”改为以 LO 语料实测为准的只读预览目标，明确不构成对任意 Office 文档的无条件承诺；DOC 工具、启动、字体覆盖和转换故障均须显式呈现。 |
