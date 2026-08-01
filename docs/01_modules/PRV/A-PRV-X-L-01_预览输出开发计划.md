---
文档编号：PRV-X-L-01
文档状态：A
负责模块：PRV
文档职责：PRV 预览输出 G3 开发计划——承接型 Plan Item，承接 PRV-VIEW 链路与 BR-PRV-* 规则，明确文件范围/@GOV/A 类债务/验收（PRV 无状态机，豁免成立）
上游约束：A-CORE-X-L-02（P4）、APC §5/§6/§12、A-PRV-M-T-01、A-PRV-I-P-01、A-DOC-X-L-01（render_preview 消费）、A-WS-X-L-01（文件读取/导出落点）、A-CORE-X-R-01
直接承接：本计划驱动 PRV 实现；@GOV 追溯 BR-PRV-*
使用边界：L3 承接型 Plan Item；文档转换归 DOC（消费不重复）、文件经 WS；@GOV 在实现代码
变更要求：只读边界变更涉 v1 冻结须 APC 授权；PI 增删保持承接可追溯
治理成熟度：draft
---

# Binder 预览输出开发计划（PRV · L3）

## §1 文档定位

PRV 多格式只读预览与输出。**只读硬边界**（全产品无 Excel/PPT 编辑入口）。docx/Excel/PPT 预览
经 DOC 预览链、音视频系统解码。无状态机（豁免成立）。

## §2 Plan Item 清单

### PI-PRV-01　多格式预览（只读硬边界）
- **承接**：REQ-PRV-001/002/003/005 / CHAIN PRV-VIEW / BR-PRV-DATA-001（只读）/ BR-PRV-DATA-002
  （管线归属）。
- **文件范围**：`src/`（预览面板：docx/Excel/PPT 经唯一公开入口 `prv.preview` 消费 DOC 内部
  转换能力后呈现；音视频系统解码 AVFoundation）；`prv.preview`。
- **@GOV**：只读无编辑/无 Excel·PPT 编辑入口 `BR-PRV-DATA-001`；管线归属（文档经 DOC、音视频
  系统解码）`BR-PRV-DATA-002`。
- **A 类债务**：音视频支持格式清单（PRV-02，系统解码能力，实现期）；预览缓存（PRV-04，引入缓存
  编排则建机复核）。
- **验收（强制）**：preview 只读无修改交互、不产生脏态；**全产品无 Excel/PPT 编辑入口（静态+交互
  核验，v1 冻结 G4/G5 强制）**；docx/Excel/PPT 经 DOC 预览链并按语料实测目标呈现、音视频系统解码；
  `DOC_TOOL_INVALID`、启动、字体覆盖和转换错误均明确不空白；断网可用。

### PI-PRV-02　打印与导出
- **承接**：REQ-PRV-004 / BR-PRV-DATA-001/002。
- **文件范围**：`src-tauri/src/prv/export.rs`（打印走系统服务；导出落 workspace 内）；
  `prv.print/export`。
- **@GOV**：打印分页呈现归 ED / 输出管线归 PRV `BR-PRV-DATA-001`；导出落点限界 `X-FORBID-004`。
- **A 类债务**：导出格式清单与打印管线（PRV-03，实现期）。
- **验收**：可打印与导出；导出落 workspace 内（越界拒绝）；失败回统一错误形状。

## §3 豁免复核（承 A-PRV-M-T-01 §5）

PRV 无状态机（请求-响应式预览/打印/导出）。**G3/G4 复核**：若引入预览转换队列/缓存重建编排或
批量导出任务编排，则触发建机（X-SEQ-003）。v1 不引入则豁免维持。

## §4 横切验收

audit 绿；@GOV 追溯 BR-PRV-*；`// covers: BR-PRV-*`（2 条）有断言；只读硬边界静态+交互核验；
豁免复核结论记录。

## §5 承接的 A 类债务（A-CORE-X-R-01 §6）

| 债务 | PI | 处置 |
|---|---|---|
| 音视频格式清单（PRV-02） | PI-PRV-01 | 实现期（系统解码） |
| 导出格式清单/打印管线（PRV-03） | PI-PRV-02 | 实现期 |
| 预览缓存（PRV-04） | PI-PRV-01 | 实现期（编排则建机复核） |
| Excel/PPT 经 LibreOffice 保真实测 | PI-PRV-01 | G4 实测（DOC 侧，不达另议） |
| 预览首屏延迟 NFR | 全 PI | G5 标定 |

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（G3，P4 收尾）：PRV 2 条 Plan Item（多格式预览只读 / 打印导出），承接 PRV-VIEW + 2 规则；无状态机（豁免维持，§3 复核）；全产品无 Excel/PPT 编辑入口 G4/G5 强制核验；docx/Excel/PPT 经 DOC、音视频系统解码；文件范围/@GOV/验收齐 |
| 2026-07-21 | v1.1 | 审计修正：PI-PRV-01 移除不存在的 `doc.render_preview`，外部预览统一走 `prv.preview` 并消费 DOC 内部能力；预览保真改以语料实测为准，工具/启动/字体/转换错误必须呈现明确失败态。 |
