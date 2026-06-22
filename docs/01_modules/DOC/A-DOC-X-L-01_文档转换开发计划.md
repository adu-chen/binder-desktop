---
文档编号：DOC-X-L-01
文档状态：A
负责模块：DOC
文档职责：DOC 文档转换 G3 开发计划——承接型 Plan Item，承接 DOC-CONVERT 链路与 BR-DOC-* 规则、X-SEQ-004、SM-DOC-01，明确文件范围/@GOV/A 类债务（版本/往返实测/保真度）/验收
上游约束：A-CORE-X-L-02（P2）、APC §5/§6/§12、A-DOC-M-T-01/A-DOC-M-S-01、A-DOC-I-P-01、A-ED-X-L-01（编辑模型对接）、A-WS-X-L-01（落盘）、A-PRV（预览消费）、A-CORE-X-R-01
直接承接：本计划驱动 DOC 实现；@GOV 追溯 BR-DOC-*；SM-DOC-01 转移矩阵
使用边界：L3 承接型 Plan Item；@GOV 在实现代码；NFR 延 G5
变更要求：PI 增删保持承接可追溯
治理成熟度：draft
---

# Binder 文档转换开发计划（DOC · L3）

## §1 文档定位

DOC 是 P2 docx 双链转换（编辑链 Pandoc / 预览链 LibreOffice）。本计划承接 A-DOC-M-T-01 已锁的
双链分离方向，A 类是版本选型与往返实测。

## §2 Plan Item 清单

### PI-DOC-01　编辑链转换与 docx 开/存机
- **承接**：REQ-DOC-001/002 / CHAIN DOC-CONVERT / SM-DOC-01 / BR-DOC-STATE-001 / BR-DOC-DATA-001
  / BR-DOC-DATA-003（blockId 兜底）/ X-SEQ-004。
- **文件范围**：`src-tauri/src/doc/edit_chain.rs`（Pandoc 调用 + data-block-id 注入）；command
  `doc.open_for_edit/save_from_edit`；存盘经 `ws.save_doc`。
- **@GOV**：转换唯一通道 `X-SEQ-004` + `BR-DOC-DATA-001`；生命周期原子性 `BR-DOC-STATE-001`；
  blockId 兜底 `BR-DOC-DATA-003`。
- **A 类债务**：Pandoc 版本/Lua 过滤器/打包（DOC-01，实现期选型回填 L2）；**data-block-id 往返
  保留度实测（DOC-02，不达走重建锚降级）**；转换缓存策略（file_cache+content_hash，实现期）；
  转换并发/进程池（实现期）。
- **验收（强制，APC §6）**：SM-DOC-01 矩阵——open 失败 OpenError 会话不建立；save 失败原文件不变
  无半成品；除 DOC 外无二进制转换代码（X-SEQ-004 静态核验）；blockId 兜底注入全块有锚。

### PI-DOC-02　样式子集三级执行
- **承接**：REQ-DOC-003 / BR-DOC-DATA-002。
- **文件范围**：`src-tauri/src/doc/style_subset.rs`（三级子集映射 + 告知）；`doc.get_style_report`。
- **@GOV**：子集化与用户可知 `BR-DOC-DATA-002`。
- **A 类债务**：样式告知交互形态（DOC-04，**C 类待人类**——实现期定弹层/状态栏）；部分支持项
  （颜色/字体/字号）实际复现率实测。
- **验收**：必需子集（标题/对齐/粗斜下/列表/基础表格/图片）docx→编辑→docx 往返**保真 ≥95%
  回归测试强制**；部分支持项损失对用户可见明示；不处理项清单公开。

### PI-DOC-03　预览链转换（供 PRV）
- **承接**：REQ-DOC-004 / BR-DOC-DATA-001（双链分离）/ BR-PRV-DATA-002（管线归属）。
- **文件范围**：`src-tauri/src/doc/preview_chain.rs`（LibreOffice→PDF）；`doc.render_preview`
  （docx/Excel/PPT）。
- **@GOV**：预览链与编辑链分离 `BR-DOC-DATA-001`。
- **A 类债务**：LibreOffice 版本/打包（DOC-01）；**Excel/PPT 经 LibreOffice 保真度与大文件表现
  实测（不达另议管线，回写本文与 PRV）**；预览缓存（实现期）。
- **验收**：docx/Excel/PPT 全保真 PDF 供 PRV；预览链失败不影响编辑链（双链分离测试）；离线可用。

## §3 实施顺序

```
PI-DOC-01（编辑链，ED 装载前置）→ PI-DOC-02（样式子集）
PI-DOC-03（预览链）独立，可并行（供 PRV）
```

## §4 横切验收

audit 绿；@GOV 追溯 BR-DOC-*；SM-DOC-01 矩阵全覆盖；`// covers: BR-DOC-*` 覆盖 DOC 全部规则
（4 条）有断言；X-SEQ-004 静态审查（全库无第二处二进制转换）。

## §5 承接的 A 类债务（A-CORE-X-R-01 §6）

| 债务 | PI | 处置 |
|---|---|---|
| Pandoc/LibreOffice 版本/Lua/打包（DOC-01） | PI-DOC-01/03 | 实现期选型回填 L2 |
| data-block-id 往返保留度（DOC-02） | PI-DOC-01 | G4 实测，不达走重建锚 |
| Excel/PPT 保真度实测 | PI-DOC-03 | G4 实测，不达另议 |
| 样式告知形态（DOC-04，C 类） | PI-DOC-02 | 待人类/实现期 |
| 转换缓存/并发/进程池 | PI-DOC-01 | 实现期 |
| 首开<12s/缓存<2s/准确率≥90% NFR | 全 PI | G5 标定复核 |

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（G3）：DOC 3 条 Plan Item（编辑链与开/存机 / 样式子集执行 / 预览链），承接 DOC-CONVERT + 4 规则 + X-SEQ-004 + SM-DOC-01；A 类含版本选型、往返保留度实测(DOC-02)、Excel/PPT 保真度实测；必需子集往返 ≥95% 回归强制；文件范围/@GOV/验收齐 |
