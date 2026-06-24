---
文档编号：DOC-I-P-01
文档状态：A
负责模块：DOC
文档职责：DOC 文档转换模块级接口协议（L2 接口层）——按 A-SYS-I-P-02 骨架声明 docx 打开/保存转换 command、预览转换 command（供 PRV）、样式子集告知接口；双链分离（编辑链 Pandoc / 预览链 LibreOffice）的 command 面
上游约束：A-SYS-I-P-02_IPC协议.md（骨架）、A-DOC-M-T-01_文档转换技术设计.md（BR-DOC-* 规则、DOC-CONVERT、X-SEQ-004、TERM-DOC-001、样式子集三级）、A-DOC-M-S-01_文档转换状态机设计.md（SM-DOC-01）、A-WS-I-P-01_工作台接口协议.md（read/save 落盘复用）、A-ED-I-P-01_编辑器接口协议.md（装载对接 open_session）、ADU.md
直接承接：实现期 DOC command 注册（Pandoc/LibreOffice 本地依赖）；ED 经 open 转换装载、save 经转换回 docx 落盘 WS；PRV 经预览转换取 PDF
使用边界：本文声明 DOC 接口语义级签名；文件落盘经 WS（不重复）；编辑呈现归 ED；预览呈现归 PRV；不新增 RULE/CHAIN/CONSTRAINT/TERM；转换器版本/Lua 过滤器延实现期
变更要求：转换接口语义变更复核 SM-DOC-01 与 ED/PRV 调用方；样式子集清单变更回写 A-DOC-M-T-01 §4 并复核 ED/PRV；blockId 兜底接口变更联动 ED
治理成熟度：draft
---

# Binder 文档转换接口协议（DOC · L2 接口层）

## §1 文档定位

```
A-SYS-I-P-02（骨架）→ 本文（DOC 转换 command）→ 实现 + @GOV(追溯 A-DOC-M-T-01)
编辑链：ed.open_session(docx) → doc.open_for_edit → 编辑模型；save → doc.save_from_edit → docx → ws 落盘
预览链：PRV → doc.render_preview → PDF（与编辑链完全分离，BR-DOC-DATA-001）
```

X-SEQ-004 接口落实：docx 等二进制格式的转换唯一经本文 command，其他模块无二进制转换入口。

## §2 编辑链转换（DOC-CONVERT / SM-DOC-01）

| command | 参数（语义） | 结果 | 特有错误/状态 | 归属 |
|---|---|---|---|---|
| `doc.open_for_edit` | DocRef（docx） | 统一编辑模型（HTML）+ 注入 data-block-id + 样式子集告知 | **convert_failed**（→SM-DOC-01 OpenError，会话不建立，对接 ed loadFail/parseFail）；依赖缺失 | BR-DOC-DATA-001/003、BR-DOC-STATE-001 |
| `doc.save_from_edit` | DocRef、编辑模型（HTML） | docx 字节流（交 WS 原子落盘 `ws.save_doc`） | convert_failed（→SaveError，原文件不变） | SM-DOC-01 SaveConverting；BR-DOC-STATE-001 |
| `doc.get_style_report` | DocRef 或转换句柄 | 样式子集告知（必需保真/部分支持损失/不处理清单，BR-DOC-DATA-002） | — | TERM-DOC-001 StyleSubset；用户可知 |

> md 不经 DOC（编辑模型原生格式，ED 直载）；docx 往返中 blockId 经 data-block-id 兜底注入与
> 尽力保留（往返保留度实现期实测，DOC-02，不达走重建锚降级）。

## §3 预览链转换（PRV-VIEW 消费 / BR-DOC-DATA-001）

| command | 参数（语义） | 结果 | 特有错误 | 归属 |
|---|---|---|---|---|
| `doc.render_preview` | DocRef（docx / Excel / PPT）、格式 | PDF（全保真，供 PRV 呈现） | render_failed（PRV 呈现失败态，不静默空白）；依赖缺失 | LibreOffice 预览链；Excel/PPT 归 DOC 预览链扩展（A-PRV-M-T-01 §2 决策） |

> 预览链与编辑链**完全分离**：render_preview 失败不影响编辑链，反之亦然（BR-DOC-DATA-001）；
> 音视频不经 DOC（PRV 系统解码，A-PRV-M-T-01 §2）。预览为单次无状态转换（SM-DOC-01 §4 豁免）。

## §4 跨模块对接小结（仅索引）

| 环节 | 调用 | 文档 |
|---|---|---|
| docx 装载 | `ed.open_session(docx)` → `doc.open_for_edit` → ED 装载 | A-ED-I-P-01 §2 / SM-DOC-01 §3 |
| docx 落盘 | `doc.save_from_edit` → `ws.save_doc` 原子落盘 | A-WS-I-P-01 §3 / SM-DOC-01 §3 |
| 预览 | PRV → `doc.render_preview` → PDF | A-PRV-M-T-01 §2 |

## §5 测试与验收

- `doc.open_for_edit` 成功装载 + 子集告知、失败 OpenError 会话不建立；`doc.save_from_edit` 往返
  必需子集保真、失败原文件不变；双链分离（render_preview 失败不影响编辑链，静态+联调）；
  X-SEQ-004（除 DOC 外无二进制转换入口，静态可查）；blockId 兜底注入；与 ED/WS/PRV G5 联调。

## §6 债务与盲区

- Pandoc/LibreOffice 版本、Lua 过滤器、打包与按需加载延实现期（DOC-01）。
- data-block-id 往返保留度实测（DOC-02）→ 不达启用重建锚降级（回写本文与 ED）。
- Excel/PPT 经 LibreOffice 的保真度与大文件表现实测（PRV G2 联合，不达另议管线）。
- 样式子集告知的呈现形态（弹层/状态栏，DOC-04）属实现期产品决策。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（实现前置件，文档往返面）：DOC 接口协议——编辑链 3（open_for_edit 装载+子集告知、save_from_edit 往返落盘 WS、get_style_report）、预览链 1（render_preview docx/Excel/PPT→PDF，与编辑链分离）、§4 跨模块对接索引（ED/WS/PRV）；X-SEQ-004 二进制转换唯一入口固化；零新增注册项；债务（版本/往返实测/Excel·PPT 保真/告知形态延后） |
| 2026-06-24 | v1.1 | 编辑链 command 实现接线（代码同步）：`doc.open_for_edit`（docx → 可编辑 HTML，经 doc/edit_chain.rs docx+styles + preserve-styles.lua，回 baseline 供冲突保护）、`doc.save_from_edit`（编辑 HTML → docx --reference-doc reference.docx → WS save_atomic 原子落盘 + 外部变更保护）已注册 lib.rs 并路由；处理逻辑对齐旧项目 binder-project（HTML 而非 markdown，承载样式子集「部分支持」层）；资源 reference.docx/preserve-styles.lua 入 bundle.resources。data-block-id 注入兜底（BR-DOC-DATA-003）与样式子集告知（get_style_report）+ extract-media 图片接入为实现期续接（A 类）；前端 docx→TipTap 编辑流接 doc.* command |
