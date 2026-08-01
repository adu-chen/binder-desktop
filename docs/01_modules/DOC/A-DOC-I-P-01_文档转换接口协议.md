---
文档编号：DOC-I-P-01
文档状态：A
负责模块：DOC
文档职责：DOC 文档转换模块级接口协议（L2 接口层）——按 A-SYS-I-P-02 骨架声明 docx 打开/保存转换 command、供 PRV 消费的内部预览转换能力、样式子集告知接口；双链分离（编辑链 Pandoc / 预览链 LibreOffice）的接口面
上游约束：A-SYS-I-P-02_IPC协议.md（骨架）、A-DOC-M-T-01_文档转换技术设计.md（BR-DOC-* 规则、DOC-CONVERT、X-SEQ-004、TERM-DOC-001、样式能力模型）、A-DOC-M-S-01_文档转换状态机设计.md（SM-DOC-01）、A-WS-I-P-01_工作台接口协议.md（read/save 落盘复用）、A-ED-I-P-01_编辑器接口协议.md（装载对接 open_session）、ADU.md
直接承接：实现期 DOC command 注册（Pandoc/LibreOffice 包内受控依赖）；ED 经 open 转换装载、save 经转换回 docx 落盘 WS；PRV 经预览转换取 PDF
使用边界：本文声明 DOC 接口语义级签名；文件落盘经 WS（不重复）；编辑呈现归 ED；预览呈现归 PRV；不新增 RULE/CHAIN/CONSTRAINT/TERM；转换器版本/Lua 过滤器延实现期
变更要求：转换接口语义变更复核 SM-DOC-01 与 ED/PRV 调用方；样式子集清单变更回写 A-DOC-M-T-01 §4 并复核 ED/PRV；blockId 兜底接口变更联动 ED
治理成熟度：draft
---

# Binder 文档转换接口协议（DOC · L2 接口层）

## §1 文档定位

```
A-SYS-I-P-02（骨架）→ 本文（DOC 转换 command）→ 实现 + @GOV(追溯 A-DOC-M-T-01)
编辑链：ed.open_session(docx) → doc.open_for_edit → 编辑模型；save → doc.save_from_edit → docx → ws 落盘
预览链：PRV `prv.preview` → DOC 内部预览转换能力 → PDF（与编辑链完全分离，BR-DOC-DATA-001）
```

X-SEQ-004 接口落实：docx 等二进制格式的转换唯一经本文 command，其他模块无二进制转换入口。

## §2 编辑链转换（DOC-CONVERT / SM-DOC-01）

| command | 参数（语义） | 结果 | 特有错误/状态 | 归属 |
|---|---|---|---|---|
| `doc.open_for_edit` | DocRef（b-docx） | 统一编辑模型（HTML）+ 注入 data-block-id + baseline 媒体目录 + 同次 `style_report`（五档检测、颜色/单元格底纹/图片恢复统计、风险告知） | **convert_failed**（→SM-DOC-01 OpenError，会话不建立，对接 ed loadFail/parseFail）；媒体授权失败；依赖缺失 | BR-DOC-DATA-001/002/003、BR-DOC-STATE-001 |
| `doc.restore_edit_media` | DocRef、规范 HTML、持久快照 baseline | 媒体目录显示授权 + baseline；必要时从同基线 DOCX 重建媒体 | `DOC_IMAGE_MEDIA_RESTORE_CONFLICT`（旧缓存缺失且磁盘已偏离基线）；`DOC_IMAGE_RESOURCE_MISSING`；媒体授权失败 | BR-DOC-DATA-002/003；BR-WS-OBS-001 |
| `doc.import_project_image` | 活动 BDocx DocRef、baseline，以及三选一人工来源：系统拖放绝对 sourcePath、项目文件树规范相对 sourcePath、剪贴板 sourceName+PNG/JPEG base64 data URL | 受控项目 assetId/sourcePath/sourceHash + baseline mediaRef/显示授权；受控根内复用，否则复制到首个配置图片根的 `imported/` | 来源多选/缺失、相对路径越界、data URL 类型/编码无效、非普通文件/符号链接/空文件/超限/扩展名与 PNG/JPEG magic 不匹配/复制失败明确拒绝 | BR-DOC-DATA-002；BR-WS-DATA-001；源文件不移动、不删除、不改写；data URL 不进入 LogicalState |
| `doc.materialize_project_image`（内部服务） | workspace root、项目相对路径、预期 SHA-256、BDocx baseline | baseline 媒体目录内稳定 mediaRef + 已验证来源元数据 | `DOC_PROJECT_IMAGE_INVALID/CHANGED`；越界/符号链接/格式或 magic 不匹配 | BR-DOC-DATA-002；BR-WS-DATA-001；TERM-ED-008 |
| `doc.save_from_edit` | DocRef、编辑模型（规范相对图片引用的 HTML）、加载基线（必填）、override（默认 false） | 新基线哈希（docx 字节内容哈希，IPC 十进制字符串）+ 新 baseline 媒体显示授权 | `DOC_BASELINE_REQUIRED`；convert_failed（→SaveError，原文件不变）；`DOC_IMAGE_RESOURCE_INVALID/MISSING`；`DOC_IMAGE_ROUNDTRIP_LOSS`（图片数量、嵌入媒体内容/顺序或 `wp:extent` 不一致）；**WS_SAVE_CONFLICT**（磁盘≠基线且未 override，原文件不变） | SM-DOC-01 SaveConverting；BR-DOC-DATA-002；BR-DOC-STATE-001；BR-WS-OBS-001 |
| `doc.get_style_report` | DocRef（b-docx） | 独立取得同一转换逻辑的五档报告：实际命中特性、已实测保证/必需待扩展/尽力/模板继承/不处理、颜色/单元格底纹/图片实例/资源/尺寸检测与恢复计数及风险告知 | `DOC_EDIT_COLOR_PROCESS_FAIL`；`DOC_EDIT_CELL_SHADING_PROCESS_FAIL`；`DOC_IMAGE_OOXML_INVALID`；转换依赖错误 | TERM-DOC-001 StyleSubset；TERM-DOC-005 TableCellShading；用户可知 |

> md 不经 DOC（编辑模型原生格式，ED 直载）；docx 往返中 blockId 经 data-block-id 兜底注入与
> 尽力保留（往返保留度实现期实测，DOC-02，不达走重建锚降级）。
> 保存时 HTML 的 `data-binder-cell-shading` / `background-color` 仅接受不透明 `#RRGGBB`，由 DOC
> 适配器物化为 `<w:shd w:val="clear" w:fill="RRGGBB"/>`；重开时仅恢复显式纯色填充。主题、渐变、
> 图案与嵌套表格不进入已实测保证集，命中时计入 `cell_shading_recovery.unrecovered_cells`。
> 图片 HTML 以相对 `src` + `data-binder-media-src` 为稳定身份；`asset://` 仅是 ED 显示投影，不得进入
> 保存输入。`style_report.image_recovery` 分别报告来源图片实例、嵌入资源与显式尺寸的 detected /
> restored / unrecovered 数量以及缺失相对资源。图片段落顺序和删除结果由规范 HTML 结构进入保存；
> `data-display` 不进入持久契约，浮动/绝对定位不受支持。保存端还必须读取 Pandoc stderr，并按 HTML
> 图片顺序核对最终 OOXML 图片数、嵌入媒体 SHA-256 与显式宽高对应的 `wp:extent`；独立图片段落
> left/center/right 另核对 `w:jc`。任一不一致均在原文件覆盖前失败。
> 代理插入的项目图片同时保留 workspace 相对来源与 SHA-256。缓存缺失时
> `doc.restore_edit_media` 先按该来源/hash 重建，再检查原 baseline DOCX；来源文件永不因文档图片
> 删除而删除或改写。保存转换成功后，当前 HTML 实际引用的普通媒体文件会复制到新 baseline 的隔离
> 媒体目录并重新授权；前端按文档记录该本地显示目录，工具结果与模型上下文只携带稳定 mediaRef，
> 不暴露本地绝对缓存路径。
> 本地拖入图片的绝对路径只作为本次系统授权输入。持久文档只保存受控项目相对 sourcePath、SHA-256
> 与稳定 mediaRef；重复内容复用既有导入资产，撤销文档插入不回收项目资产。

## §3 预览链转换（PRV-VIEW 消费 / BR-DOC-DATA-001）

| 能力（非公开 IPC command） | 参数（语义） | 结果 | 特有错误 | 归属 |
|---|---|---|---|---|
| DOC 预览转换能力（仅由 `prv.preview` 消费） | DocRef（docx / Excel / PPT）、格式 | 以 LO 语料实测为准的只读 PDF | 当前错误维持既有语义；PI-DOC-03 完成后细分为 `DOC_TOOL_INVALID`（工具不可用）、`DOC_PREVIEW_START_FAILED`（headless 未启动）、`DOC_FONT_COVERAGE_MISSING`（包内字体无覆盖）、`DOC_PREVIEW_FAIL`（转换失败），PRV 呈现明确失败态 | LibreOffice 预览链；Excel/PPT 归 DOC 预览链扩展（A-PRV-M-T-01 §2 决策） |

> 预览链与编辑链**完全分离**：`prv.preview` 所消费的转换能力失败不影响编辑链，反之亦然（BR-DOC-DATA-001）；
> 音视频不经 DOC（PRV 系统解码，A-PRV-M-T-01 §2）。预览为单次无状态转换（SM-DOC-01 §4 豁免）。

## §4 跨模块对接小结（仅索引）

| 环节 | 调用 | 文档 |
|---|---|---|
| docx 装载/重启恢复 | `ed.open_session(docx)` → `doc.open_for_edit`；持久镜像 → `doc.restore_edit_media` → ED 装载 | A-ED-I-P-01 §2 / SM-DOC-01 §3 |
| docx 落盘 | `doc.save_from_edit`（HTML→docx）→ WS 保存机 baseline guard + 原子落盘 | A-WS-I-P-01 §3 / SM-DOC-01 §3 |
| 预览 | PRV `prv.preview` → DOC 内部预览转换能力 → PDF | A-PRV-M-T-01 §2 |

## §5 测试与验收

- `doc.open_for_edit` 成功装载 + 子集告知、失败 OpenError 会话不建立；`doc.save_from_edit` 往返
  必需子集保真、文本颜色/单元格纯色底纹/嵌入图片的 OOXML 与重开 HTML 语义一致；图片缓存重启
  恢复、资源缺失、越界引用、Pandoc 资源警告、图片计数下降、文档流顺序和删除结果均有定向测试；
  `data-display` 丢失有明确排除断言；转换失败时原文件不变；双链分离（`prv.preview` 的内部预览转换失败不影响编辑链，静态+联调）；
  X-SEQ-004（除 DOC 外无二进制转换入口，静态可查）；blockId 兜底注入；与 ED/WS/PRV G5 联调。

## §6 债务与盲区

- Pandoc/LibreOffice 只能从 Binder resource_dir 的 manifest 受控资源解析；系统工具永不参与候选或兜底。
- Pandoc arm64 版本、Lua 过滤器与打包清单已锁定；x86_64 与发布 CI 待补。LibreOffice 源码级 GlyphFallback
  修复、稳定构建、签名与公证仍受 BLOCK-REL-LO-001 阻断。
- DOC-PLAT-001（macOS 最低版本、arm64/x86_64 矩阵）仅登记暂缓，本轮不以宿主工具或宿主字体回退解决。
- LO 专项先消除启动期 GUI 与 headless SIGABRT，再通过 CoreText 注册包内字体和 GlyphFallback 源码策略实现
  确定性回退；宿主字体仅可用于开发诊断。
- data-block-id 往返保留度实测（DOC-02）→ 不达启用重建锚降级（回写本文与 ED）。
- Excel/PPT 经 LibreOffice 的保真度与大文件表现实测（PRV G2 联合，不达另议管线）。
- 当前告知形态为 ED 状态栏的“样式提示：N”，完整告知通过 `title`/无障碍标签可读；弹层或报告面板仍可作为后续产品增强。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（实现前置件，文档往返面）：DOC 接口协议——编辑链 3（open_for_edit 装载+子集告知、save_from_edit 往返落盘 WS、get_style_report）、预览链 1（render_preview docx/Excel/PPT→PDF，与编辑链分离）、§4 跨模块对接索引（ED/WS/PRV）；X-SEQ-004 二进制转换唯一入口固化；零新增注册项；债务（版本/往返实测/Excel·PPT 保真/告知形态延后） |
| 2026-07-21 | v1.7 | 审计修正：移除不存在的公开 `doc.render_preview`，唯一公开预览入口统一为 `prv.preview`，DOC 只提供内部转换能力；预览错误细分为工具无效、headless 启动、字体覆盖与转换失败；五档样式报告标注为 PI-DOC-02 的目标接口，`doc.get_style_report` 在完成前不得调用。 |
| 2026-06-24 | v1.1 | 编辑链 command 实现接线（代码同步）：`doc.open_for_edit`（docx → 可编辑 HTML，经 doc/edit_chain.rs docx+styles + preserve-styles.lua，回 baseline 供冲突保护）、`doc.save_from_edit`（编辑 HTML → docx --reference-doc reference.docx → WS save_atomic 原子落盘 + 外部变更保护）已注册 lib.rs 并路由；处理逻辑对齐旧项目 binder-project（HTML 而非 markdown，承载样式子集「部分支持」层）；资源 reference.docx/preserve-styles.lua 入 bundle.resources。data-block-id 注入兜底（BR-DOC-DATA-003）与样式子集告知（get_style_report）+ extract-media 图片接入为实现期续接（A 类）；前端 docx→TipTap 编辑流接 doc.* command |
| 2026-06-24 | v1.2 | 编辑器扩展集对齐（格式修复，代码同步）：docx→HTML 的样式子集需编辑器对应 TipTap 扩展解析为「成熟文本块」，否则内联 style/颜色/字号/表格在编辑器显示为生标记。按旧项目补齐扩展集 ↔ 三级子集映射：必需子集（对齐 TextAlign / 下划线 Underline / 基础表格 Table* / 嵌入图片 Image）、部分支持（颜色 TextStyle+Color / 字体 FontFamily / 字号 FontSize 自定义扩展）、增强（高亮 Highlight / 上下标 Sub·Superscript）；StarterKit 禁 v3 自带 link/underline 避重复；前端 src/components/ed/extensions（index.ts + FontSize.ts）。待续：原生 docx「预览→编辑建草稿」流程与 草稿/生成docx 直编辑流程（旧版成熟链路） |
| 2026-06-24 | v1.3 | docx 成熟编辑流程接线（旧版链路，代码同步）：新增 `doc.create_draft`（原生 docx → 同目录 `{stem}.draft.docx` 副本，守界）。流程区分按文件名——**原生 docx（非 *.draft.docx）点击 → 只读 PDF 预览**（PrvPanel「创建草稿进行编辑」按钮 → create_draft → openDoc 进编辑模式）；**草稿/生成 docx（*.draft.docx）点击 → 直接编辑**（FileTreeNode 路由 + isDocxEditable 草稿专属）。对齐旧项目 create_draft_docx + DocxPdfPreview 成熟逻辑。前端：docService.createDocxDraft/isNativeDocxPreview、FileTreeNode 原生预览·草稿编辑、PrvPanel 编辑按钮 |
| 2026-07-12 | v1.4 | 三态实现回写：`doc.save_from_edit` 签名补加载基线与 override；保存结果回新基线哈希字符串；转换成功后仍经 WS 保存前 baseline guard 与原子落盘，冲突返回 `WS_SAVE_CONFLICT` 且原 docx 不变 |
| 2026-07-21 | v1.5 | DOC-BUNDLED-RUNTIME-01：补充工具运行时契约——转换接口仅调用 resource_dir 内经 manifest 校验的 Pandoc/LibreOffice；设备本地工具不再是候选路径，并登记 LO 字形回退与稳定构建的未闭合发布债务。 |
| 2026-07-21 | v1.6 | DOC-LO-RELIABILITY-FONT-PLAN-01：扩充样式告知为五档能力矩阵；预览失败语义纳入 headless/缺字诊断；记录 LO 内部确定性字体修复路径与暂缓平台矩阵。 |
| 2026-07-27 | v1.8 | DOCX-COLOR-ROUNDTRIP-01：注册 `doc.get_style_report`；`doc.open_for_edit` 回同次 `style_report`；明确文本颜色的 OOXML 恢复统计与 ED 状态栏风险告知。 |
| 2026-07-28 | v1.9 | BDocx TableCellShading：样式报告增加 `cell_shading_recovery`；锁定 HTML 标记到 OOXML `w:shd`、重开恢复、未恢复计数和 `DOC_EDIT_CELL_SHADING_PROCESS_FAIL` 错误语义。 |
| 2026-07-28 | v1.10 | BDOCX-IMAGE-MEDIA-FOUNDATION-01：新增 `doc.restore_edit_media`；`open_for_edit`/样式报告增加图片实例、资源、尺寸恢复统计；`save_from_edit` 锁定 baseline 媒体目录、资源/路径/Pandoc 警告/OOXML 图片数门禁与明确错误。 |
| 2026-07-28 | v1.11 | BDOCX-IMAGE-INTERACTION-01：图片保存契约补文档流顺序和删除结果；明确 data-display 不持久、浮动/绝对定位不支持，并增加真实往返排除断言。 |
| 2026-07-29 | v1.12 | BDOCX-IMAGE-AGENT-EDIT-01：新增项目图片内部物化契约与来源/hash 草稿重建；明确稳定 mediaRef、越界/格式/magic/哈希错误和项目源文件保护。 |
| 2026-07-29 | v1.13 | BDOCX-IMAGE-ADVANCED-AGENT-OPS-01：`save_from_edit` 图片门禁从计数扩展为实例数、媒体内容与顺序、`wp:extent` 及独立图片段落 `w:jc` 核验；失败继续保持原 DOCX 不变。 |
| 2026-07-29 | v1.14 | BDOCX-IMAGE-COMPOSITE-IMPORT-01：新增 `doc.import_project_image`，锁定受控根复用/原子复制、格式 magic、大小、SHA-256、去重、源文件保护和 baseline 媒体物化。 |
| 2026-07-30 | v1.15 | BDOCX-IMAGE-COMPOSITE-IMPORT-01 人工入口闭合：`doc.import_project_image` 同时接受系统绝对路径、项目文件树规范相对路径或剪贴板 PNG/JPEG data URL；所有来源统一做大小、magic、哈希、去重和原子复制。 |
