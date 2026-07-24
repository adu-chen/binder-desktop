---
文档编号：DOC-M-T-01
文档状态：A
负责模块：DOC
文档职责：DOC 文档转换模块级 L2 技术设计（规则 SSOT）——docx 双向转换管线（编辑链 Pandoc / 预览链 LibreOffice，双链分离）、样式子集三级执行契约、blockId 转换层兜底、本地转换运行时边界；承接 REQ-DOC-001~005
上游约束：A-DOC-M-D-01_文档转换功能需求描述.md（REQ-DOC-001~005、DOC-01~05）、A-CORE-C-P-01_APC.md（§4.1、§6、§9）、A-ED-M-T-01_编辑器技术设计.md（BR-ED-DATA-002 分工锁定、§11.1 样式子集与 blockId 选型记录）、A-WS-M-T-01_工作台技术设计.md（BR-WS-PERSIST-001 原子落盘、X-FORBID-004）、A-SYS-I-P-02_IPC协议.md（统一错误形状）、A-SYS-M-S-01_状态机总图.md（docx 开/存核心链、X-SEQ-003/X-INTER-001）、ADU.md（§3 格式）
直接承接：本文 RULE/CHAIN/CONSTRAINT/TERM 为 DOC 规则 SSOT，代码经 @GOV 映射；SM-DOC-01 在 A-DOC-M-S-01 定义并登记总图；样式子集契约被 ED（编辑模型承载）与 PRV（预览补偿）消费
使用边界：本文是 DOC 规则 SSOT；不定义编辑模型内部（归 ED）、不定义预览呈现（归 PRV）、不做文件落盘（经 WS）；工具具体版本由受控 manifest 锁定，状态机内部归 A-DOC-M-S-01
变更要求：样式子集三级清单变更须同步 ED（编辑模型承载）与 PRV（预览补偿）复核；转换链分工变更须经架构评审；TERM 一经被 term_ref 引用其 en 不可更改
治理成熟度：draft
---

# Binder 文档转换技术设计（DOC · L2 技术设计）

## 0. Terminology Registry

<!-- TERM
term_id: TERM-DOC-001
chains: DOC-CONVERT
zh: 样式子集
en: StyleSubset
forbidden: 样式白名单, 格式子集, 样式范围, 支持样式表
-->
b-docx 进入编辑态（旧称「编辑模式」，2026-07-04 起「模式」一词专属 AG 面板）时编辑模型所承载的样式范围。
当前实现按三级结果执行（必需、部分、明确不处理）；目标报告模型扩展为五档（已实测保证、必需待扩展、尽力保留、
仅模板继承、明确不处理）。进入编辑即子集化且对用户可知（REQ-DOC-003）。

<!-- TERM
term_id: TERM-DOC-002
chains: DOC-CONVERT
zh: 编辑草稿
en: EditDraft
forbidden: 草稿副本, draft文件, 临时编辑文件, 编辑副本
-->
EditDraft 是 b-docx 的一种来源形态：原生 docx 进入编辑前创建的同目录副本（命名
`{stem}.draft.docx`）。**原生 docx**（非 `*.draft.docx`）点击为只读 PDF 预览，经「创建 b-docx
进行编辑」复制为 EditDraft 后进 HTML 编辑态；EditDraft 点击直接进编辑。编辑作用于 EditDraft、
原 docx 不被改动（保护原始文档）；落盘经编辑链 HTML→docx 往返（§2.1）。

<!-- TERM
term_id: TERM-DOC-003
chains: DOC-CONVERT, ED-EDIT
zh: b-docx
en: BDocx
forbidden: 模拟docx, HTML模拟docx, 可编辑docx, 项目docx, 生成docx, docx编辑稿
-->
<!-- TERM
term_id: TERM-DOC-004
chains: DOC-CONVERT
zh: 字体身份修复
en: FontIdentityFix
forbidden: 字体fallback修复, 字形回退修复, 字体替换修复
-->
LO 内置的系统字体名与 OOXML 文档请求的字体名之间的映射修复。根因为 CoreText 在非英文
locale 下返回本地化显示名（如 "蘋方-簡"），而 OOXML 请求 ASCII 英文名（"PingFang SC"），
导致 LO 的 FindFontFamily 无法精确匹配。修复在字体枚举阶段注入英文名（层1）、Office
生态缺失字体别名（层2）、以及未知字体的确定性回退链（层3）。补丁见
binder_font_identity.patch。

b-docx 是 Binder 内可编辑的 docx 类文档，区别于原生 `.docx`。它的编辑态由 DOC 编辑链将 docx
字节转换为 HTML / ProseMirror 统一编辑模型，ED 只编辑该 HTML 模型，保存时再由 DOC 转回 docx
字节并经 WS 落盘。来源包括：原生 docx 经 `doc.create_draft` 生成的 EditDraft（`*.draft.docx`）、
文件树新建的 docx 文档、Agent 创建的 docx 文档。原生 `.docx` 不是 b-docx，在当前项目中只走
LibreOffice→PDF 预览路径；必须先转成 b-docx 后才能进入编辑链。

## §1 文档定位

```
REQ-CORE-004 → REQ-DOC-001~005（DOC L1）→ 本文 RULE/CHAIN/CONSTRAINT/TERM（SSOT）→ 代码(@GOV)
本文 ←分工对偶← A-ED-M-T-01（BR-ED-DATA-002：转换归 DOC、编辑归 ED）
本文 →驱动→ A-DOC-M-S-01（SM-DOC-01 docx 开/存机）→登记→ A-SYS-M-S-01 总图
本文样式子集契约 →供→ ED（编辑模型承载）/ PRV（预览补偿）消费
```

技术方案采纳旧版实证（R-ED-M-R-01 v2.0 终案 + R-ED-M-R-23，台账 §4.5 P1 原料，按 APC §8 重新
声明）：**编辑链 Pandoc、预览链 LibreOffice→PDF、双链完全分离**。两个工具、Lua 过滤器、参考样式
文档和默认回退字体均为 Binder 包内受控资源，版本、架构、哈希与许可证在 `resources/tools/manifest.json`
锁定；运行时不得以系统工具作为候选。

## §2 转换管线架构（双链分离）

```
打开编辑   b-docx ──Pandoc(+Lua 过滤器)──→ HTML/统一编辑模型 ──注入缺失 data-block-id──→ ED 装载
保存往返   ED 编辑模型(HTML) ──Pandoc──→ docx 字节流 ──经 WS 原子落盘──→ 文件
只读预览   docx ──LibreOffice──→ PDF（以 LO 语料实测为准）──→ PRV 呈现
```

- 编辑链与预览链**完全分离**：无共享中间态、无功能黏连；预览链失败不影响编辑链，反之亦然。
- 转换全程**本地执行**（Pandoc/LibreOffice 包内依赖），离线可用，内容不上行（REQ-DOC-005）。
- 工具解析唯一经 Tauri `resource_dir()`：运行时只接受经过 manifest 校验的包内绝对路径；`PATH`、
  `/Applications`、当前工作目录和设备字体均不得作为转换依赖或兜底。
- 文件读写经 WS（X-FORBID-004）；转换中间产物仅在 `app_data_dir/doc-runtime/<request-id>/` 生命周期内管理，
  不进入 workspace 文件树或无界系统临时目录。

### §2.1 编辑链处理逻辑（实现 SSOT，移植旧版 binder-project 实证链路）

**编辑模型为 HTML**（非 markdown）：编辑器 TipTap/ProseMirror 是 HTML 模型，且 markdown 丢失样式子集
「部分支持」层（颜色/字体/字号）；HTML+styles 才能承载当前三级结果，并为五档目标报告收集证据。

- **打开（b-docx → 可编辑 HTML，`doc.open_for_edit`）**：
  `pandoc <docx> --from docx+styles --to html+raw_html+native_divs+native_spans --wrap=none
  --extract-media=. --preserve-tabs --lua-filter preserve-styles.lua`
  - `docx+styles` 读取扩展保留 DOCX 样式；`native_divs/spans` 把样式落为内联 `style=`/`data-custom-style`；
    `preserve-styles.lua`（资源）强制保留对齐/颜色/字号；`--extract-media` 解出图片至工作目录供编辑器显示。
  - **不加 `--standalone`**：只输出 `<body>` 正文片段（不含 `<head>/<style>` CSS），否则前端会把那坨 CSS
    当文本/代码块渲染。
  - **前端解析（关键）**：编辑器装有 `tiptap-markdown`（md 文件用），其会劫持 `setContent(string)` 按
    markdown 解析（HTML 标签变字面、缩进 CSS 变代码块）。docx 路径须用 `generateJSON(html, extensions)`
    **按 HTML 解析为 ProseMirror JSON 后 setContent**，绕过 markdown 劫持，样式经 §4.1 扩展渲染为成熟文本块。
  - 回 baseline（docx 字节内容哈希）供保存冲突保护（SM-WS-01）；HTML 超 **15MB** 拒绝（编辑态上限，防崩溃）。
- **保存（HTML → docx，`doc.save_from_edit`）**：
  `pandoc --from html --to docx --reference-doc reference.docx`（参考文档统一样式，往返保真）→ 交 WS
  保存机执行 baseline guard + `save_atomic` 原子落盘。调用方必须传入打开时加载基线；磁盘内容偏离基线且
  未 `override=true` 时返回 `WS_SAVE_CONFLICT`，原 docx 不变；`override=true` 只能来自用户显式确认。
- **样式子集 ↔ 编辑器扩展**：HTML 的样式需编辑器对应 TipTap 扩展解析为「成熟文本块」，否则显示为生标记
  （映射见 §4 表）。编辑器扩展集为 DOC 编辑链可用性的前置（前端 ED 承载，A-ED-M-T-01）。
- **资源（经 bundle.resources 打包）**：`reference.docx`（样式模板）、`preserve-styles.lua`（样式保留过滤器）；
  预览链 LibreOffice 为优化版（自定义构建精简约 30%，发布期打包不得绕过，A-CORE-X-R-01 §9 BLOCK-REL-LO-001）。

### §2.2 原生 docx / b-docx 编辑流程（预览/转换/直编辑，移植旧版成熟链路）

按 TERM-DOC-003 BDocx 区分两条入口：原生 docx 只走预览链；b-docx 才能进入编辑链。

```
原生 docx（foo.docx） 点击 → 只读 PDF 预览（预览链）
                       └「创建 b-docx 进行编辑」→ doc.create_draft → foo.draft.docx（EditDraft / b-docx）
                                             → doc.open_for_edit → HTML 编辑（编辑链）
                                             → ⌘S → doc.save_from_edit → docx 落盘
b-docx（EditDraft / 文件树新建 / Agent 创建）点击 → 直接进 HTML 编辑（编辑链，同上保存）
```

- **原生 docx 不直接编辑**：保护原始文档，b-docx 是唯一可进入 DOC 编辑链的 docx 类文档。
- `doc.create_draft`：原生 docx → `{stem}.draft.docx`（EditDraft / b-docx）同目录副本
  （守界 X-FORBID-004，已存在先覆盖）。
- 文件树新建与 Agent 创建的 docx 文档直接落入 b-docx 分类，点击即进入 HTML 编辑链。
- b-docx 身份必须可持久恢复：`*.draft.docx` 可由命名兼容识别；文件树新建与 Agent 创建的 `.docx`
  由 workspace 内隐藏身份清单（`.binder/bdocx_manifest.json`）登记，`ws.list_docs` 回 `is_bdocx` 供前端
  路由。隐藏清单只记录身份，不参与内容权威；文件内容仍为 docx 字节，打开/保存仍走 DOC 编辑链。
- AgentEdit / REF / DE 读取 BDocx 时必须消费 DOC 编辑链产出的 HTML 编辑模型快照（AG 侧命名为
  `AgentEditableSnapshot`），不得经 WS 文本读取把 `.docx` 字节当 UTF-8 内容处理。对非当前编辑器的
  BDocx，读取路径为 `docx_to_html` → canonical blockId HTML → 统一 Block 投影；对当前已打开的 BDocx，
  以 ED 实时 HTML 序列化内容为准。
- 既有 BDocx 的 diff 接受不得走后台 raw apply：DiffPayload/AgentEditableSnapshot 中的 HTML 是编辑模型，
  不是 `.docx` 磁盘字节。只有当前编辑器 HTML 草稿成功应用后，accepted 才确认裁决；真正写回 `.docx`
  仍由用户 save 触发 `doc.save_from_edit` 完成 HTML→docx 转换并经 WS 原子落盘。
- 流程对偶旧版 `create_draft_docx` + `DocxPdfPreview.handleCreateDraft`；Excel/PPT 无编辑入口（恒只读预览，BR-PRV-DATA-001）。

### §2.3 LO 可靠性与确定性字体专项（DOC-LO-RELIABILITY-FONT-PLAN-01）

本节是 `BLOCK-REL-LO-001` 的修复方案；平台版本/双架构问题独立记为 DOC-PLAT-001，本轮不以宿主工具、
宿主字体或 Rosetta 规避。

1. **无界面启动与可用性降级**：应用启动只校验包内路径、哈希、签名与 manifest，不启动 `soffice` 进程。
   工具无效时记录 DOC 不可用状态，Binder 其余功能仍可启动；相关 DOC command 返回 `DOC_TOOL_INVALID`，
   不得把工具异常升级为整应用启动失败。LO 进程健康检查仅在 CI 或真实预览调用中执行，必须使用独立 profile，
   并附加 `--headless --nologo --nodefault --norestore --nolockcheck`。`--version` 裸探测不得留在应用启动路径。
2. **稳定 headless**：以固定稳定 LO 源码 tag 重建，禁止使用 `LibreOfficeDev` alpha 产物发布；运行时只设置
   包内 Frameworks/MacOS 所需库路径，不继承宿主 `DYLD_LIBRARY_PATH`。先以干净 profile 复现/消除
   `SIGABRT`，再进入字体验收；VCL 后端选择以稳定版实测为准，不强制沿用 `SAL_USE_VCLPLUGIN=gen`。profile
   与中间文件必须在 `app_data_dir/doc-runtime/<request-id>/`，请求结束清理、异常启动时清理过期目录；不使用
   workspace 路径或无界的系统临时目录。
3. **字体证据与映射先行**：在改造 LO 字体源码前，以固定 DOCX/PPTX 语料建立 Font Trace：记录 OOXML 的
   直接字体、样式、主题、别名、文字脚本和字重，以及 Binder 映射与 PDF 实际嵌入字体；同输入、同 profile
   连续转换并比较字体表、文本层、页数和页面像素。原生 Office 源文件只读；已确认的映射仅可写入
   `app_data_dir/doc-runtime/<request-id>/` 下的临时 Render Proxy。此步骤区分 family 规则未命中、profile
   未生效与字形级回退，禁止在未取得证据时猜测性修改 `FindFontSubstitute`。
   **实现状态（2026-07-22）**：通用只读扫描与版本化空 FontPlan 已落地；扫描范围覆盖 run/style/theme/font-table，
   策略重复别名即拒绝，不访问设备字体库。具体来源字体仅作为语料，不进入代码规则。旧
   `fontsubst.xcu` 宿主默认字体替换已从运行时移除，避免未知字体被静默重定向；它不替代后续源码 GlyphFallback。
   **S1 源码证据（2026-07-22）**：稳定 tag 的 Quartz 插件在 `vcl/osx/salinst.cxx:create_SalInstance()`
   无条件调用 `[VCL_NSApplication sharedApplication]`；这与既有 `NSApplication → GetCurrentProcess →
   _RegisterApplication → abort` 一致。因此 `--headless` 仅抑制界面展示，不可作为“不会初始化 AppKit”的保证。
   需要在完整 macOS 构建中验证并改造启动/事件循环边界，禁止用环境变量或宿主 GUI 重启伪造通过。完整
   Xcode 14.3+ 是首轮构建硬前置；JDK 17+ 仅用于保留 Java 组件的完整构建，首轮可显式 `--without-java`。
4. **字体身份修复（2026-07-24 已实现）**：根因定位——CoreText 在非英文 locale 下返回字体的本地化显示名
   （如 "蘋方-簡"），而 OOXML 请求的是 ASCII 英文名（"PingFang SC"），导致 LO 的
   `FindFontFamily` hash 查找失败。修正在 `vcl/quartz/SystemFontList.cxx` 的
   `fontEnumCallBack` 中：
   (a) 层1——为每个 CoreText 字体额外注册 `kCTFontFamilyNameAttribute` 英文名到
   `PhysicalFontCollection`；
   (b) 层2——对已知 Office 生态缺失字体注册硬编码别名（`Microsoft YaHei → PingFang SC`、
   `Aptos → Arial` 等，覆盖中/日/韩/拉丁/等宽）。改动仅 `SystemFontList.hxx`+8 行、
   `SystemFontList.cxx`+130 行，不触及 FindFontSubstitute 或 PhysicalFontCollection
   查找算法。补丁：`patches/libreoffice/binder_font_identity.patch`。
5. **包内字体注册与完整性**：把经许可、带版本与哈希的 Latin、CJK（简/繁/日/韩）、符号及
   按需 emoji 字体置于 `LibreOffice.app/Contents/Resources/binder-fonts/`（后续迭代，
   当前 binder-fonts/ 为空）。发布清单必须锁定 LO 源码 tag、补丁 SHA、全部
   Frameworks/dylib/字体的哈希和许可证；CI 对整个嵌套 `LibreOffice.app` 进行签名、
   资源清单与公证前校验，不能只校验 `soffice`。
6. **失败与诊断**：无可用 glyph 时返回结构化 `DOC_FONT_COVERAGE_MISSING`（请求字体、Unicode 范围、候选链），
   headless 启动失败返回 `DOC_PREVIEW_START_FAILED`，转换失败返回 `DOC_PREVIEW_FAIL`；诊断不得包含文档正文。
   禁止随机使用本机默认字体；宿主字体扫描可作为开发诊断，绝不参与默认导出结果。
7. **验收语料**：以 DOCX/PPTX 的中英日韩、符号、emoji、粗斜体、混排与缺字样本生成 PDF，断言文本层、
   实际嵌入字体、页数和页面截图；无系统 LO/无额外字体的干净环境必须通过。

## §3 转换通道与生命周期（DOC-CONVERT）

<!-- CHAIN
chain_id: DOC-CONVERT
主责模块: DOC
status: active
risk_tier: P1
-->

<!-- RULE
rule_id: BR-DOC-DATA-001
主链路: DOC-CONVERT
域: DATA
-->
docx 等二进制文档格式的解析与生成必经 DOC 转换管线（编辑链 Pandoc / 预览链 LibreOffice），
其他模块不得内置二进制格式处理路径（与 BR-ED-DATA-002 对偶；X-SEQ-004）；编辑链与预览链完全
分离，不得共享中间态或互相依赖；转换全程本地执行，文档内容不得因转换经网络上行；Pandoc、
LibreOffice 与默认回退字体必须随 Binder 发布包交付，运行时仅执行 manifest 校验通过的包内路径；
依赖缺失、损坏或版本不符时回统一错误，不探测、不调用也不静默降级到系统工具。

<!-- RULE
rule_id: BR-DOC-STATE-001
主链路: DOC-CONVERT
域: STATE
-->
转换生命周期必须满足：打开转换失败回统一错误形状且不建立编辑会话（对接 SM-ED-01 的
parseFail/loadFail）；保存转换失败或落盘失败时原 docx 文件保持不变（原子性经 WS
BR-WS-PERSIST-001 保障），无半成品落盘；转换中断不残留破坏性中间产物；失败可重试。
状态转移归 SM-DOC-01（A-DOC-M-S-01）。

## §4 样式子集能力模型（DOC-CONVERT）

承 A-ED-M-T-01 §11.1 选型记录（旧版 R-ED-M-R-23 实证复现率），本文为执行 SSOT：

| 级别 | 清单 | 对用户的承诺 | 当前证据与落地要求 |
|---|---|---|---|
| 已实测保证 | 标题、粗体 | 现有真实 DOCX 往返已覆盖 | 维持自动回归；这是当前唯一可以称为“已验证”的样式集合 |
| 必需待扩展 | H1-H6、段落对齐、斜体/下划线、有序/无序列表、基础表格、嵌入图片 | 目标为往返保真（语料总通过率 ≥95%），未达标不得标为保证 | 每项都需 DOCX XML 断言、编辑器 HTML 断言与 LO PDF 视觉回归；完成后才转入“已实测保证” |
| 尽力保留 | 文本颜色、字体、字号、同名 custom-style（未验证） | 内容及可承载的样式尽力保留；字体度量、行距、分页、表格宽度可变化 | `docx+styles`、Lua 与 `reference.docx` 参与传递；`data-custom-style` 与字号单位在 ED 未验证前不得宣称继承，字号 pt→px 偏差修复前必须明示差异；按语料统计而非承诺无损 |
| 仅模板继承 | 页边距、页面尺寸、输出页眉页脚、reference.docx 中已注册的样式 | 输出继承 Binder `reference.docx` 的定义，不承诺复刻来源文档对应定义 | 不把“继承 Binder 模板”表述为“继承原文档布局” |
| 明确不处理 | 来源页眉页脚内容、文本框、SmartArt、VBA 宏、修订/批注、域代码、复杂样式继承链、复杂合并表格格式、浮动对象定位 | 不进入可编辑往返承诺；原文档通过只读 PDF 查看 | `doc.get_style_report` 必须列出命中的风险项；禁止静默宣称保真 |

五档是**目标报告模型**，不是当前三档 `StyleTier` 的同义改名；在 PI-DOC-02 完成枚举、IPC、前端告知和
回归语料改造前，任何实现只能对用户报告现有三级结果，且不得把“必需待扩展”伪装为“已实测保证”。

<!-- RULE
rule_id: BR-DOC-DATA-002
主链路: DOC-CONVERT
域: DATA
-->
docx 进入编辑态必须按当前实现支持度子集化：已实测项维持回归覆盖；必需待扩展项在语料达标前不得
向用户承诺保真；尽力保留项的损失必须对用户可知（不静默丢失）；明确不处理项清单公开且由只读预览
补偿查看需求。能力清单的变更必须回写本文并同步 ED（编辑模型承载范围）与 PRV（预览补偿）复核，
不得在转换代码中私自增减。

### §4.1 样式子集 ↔ 编辑器扩展映射（前端 ED 承载，实现 SSOT）

编辑链产出的 HTML 样式须由编辑器（TipTap，ED 前端）对应扩展解析为富文本「成熟文本块」，否则内联
`style=`/颜色/字号/表格在编辑器显示为生标记。扩展集为编辑可用性前置（移植旧版 binder-project）：

| 子集级别 | 样式项 | TipTap 扩展 |
|---|---|---|
| 必需子集 | 标题/列表/粗斜 | StarterKit |
| 必需子集 | 段落对齐 | `TextAlign` |
| 必需子集 | 下划线 | `Underline` |
| 必需子集 | 基础表格 | `Table` / `TableRow` / `TableHeader` / `TableCell` |
| 必需子集 | 嵌入图片 | `Image`（base64/inline） |
| 部分支持 | 文本颜色 | `TextStyle` + `Color` |
| 部分支持 | 字体 | `FontFamily` |
| 部分支持 | 字号 | `FontSize`（自定义扩展，移植旧版） |
| 增强 | 高亮/上下标 | `Highlight` / `Subscript` / `Superscript` |

> 顺序约束：`TextStyle` 须先于依赖它的 `Color`/`FontFamily`/`FontSize`；StarterKit 禁其 v3 自带
> `link`/`underline` 避重复。扩展集变更须与本表同步（ED 承载范围 ↔ DOC 子集契约一致）。

## §5 blockId 转换层兜底（DOC-CONVERT）

承 A-ED-M-T-01 §11.1：转换层在 docx→编辑模型方向为缺失 `data-block-id` 的块级元素注入
blockId（兜底，保障 DE/REF 锚定可用）；编辑模型→docx 方向尽力保留 `data-block-id`，
经 Pandoc 往返的保留度实现期实测（DOC-02），不达则按"往返后兜底重建锚"降级并回写契约。

<!-- RULE
rule_id: BR-DOC-DATA-003
主链路: DOC-CONVERT
域: DATA
-->
docx→编辑模型转换必须为缺失 data-block-id 的块级元素注入 blockId 兜底（生成规则与 ED blockId
机制一致，A-ED-M-T-01 §11.1），保障上层锚定不因转换缺锚；编辑模型→docx 转换尽力保留
data-block-id；往返保留度不达标时必须显式采用"重建锚"降级契约并回写本文与 A-ED-M-T-01，
不得静默让锚漂移。

实现约定（2026-07-09 修复）：`docx_to_html` 的返回值必须是已完成 blockId canonical 化的 HTML；
`p/h1-h6/blockquote/li/pre/div/table` 等块级元素缺失 `data-block-id` 时注入新 id，已有唯一 id
保持，重复 id 重新分配。AG/DE/ED 不再对 b-docx 的磁盘转换结果另行补锚，避免同一文档在不同入口
出现不同锚模型。

## §6 跨模块约束（CONSTRAINT）

<!-- CONSTRAINT
constraint_id: X-SEQ-004
摘要: docx 等二进制文档格式的解析/生成必须经 DOC 转换管线，任何模块不得内置二进制文档转换库或另立转换路径
涉及模块: DOC, ED, WS, PRV, CBT, AG
链路影响: DOC-CONVERT, ED-EDIT
-->

**复用既有约束**：X-FORBID-004（文件路径限界）、X-SHARED-001（跨模块结构经 SYS）、
X-SEQ-003（SM-DOC-01 先建机）、X-INTER-001（DOC↔ED、DOC↔WS 转移点登记总图）。

## §7 需求到规则映射（REQ_MAP）

<!-- REQ_MAP
req_id: REQ-DOC-001
target_type: CHAIN
target_id: DOC-CONVERT
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-DOC-001
target_type: RULE
target_id: BR-DOC-STATE-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-DOC-002
target_type: RULE
target_id: BR-DOC-STATE-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-DOC-002
target_type: RULE
target_id: BR-DOC-DATA-003
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-DOC-003
target_type: RULE
target_id: BR-DOC-DATA-002
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-DOC-004
target_type: RULE
target_id: BR-DOC-DATA-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-DOC-005
target_type: RULE
target_id: BR-DOC-DATA-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-DOC-005
target_type: CONSTRAINT
target_id: X-SEQ-004
status: mapped
-->

## §8 测试与验收

- **唯一通道与双链分离**（BR-DOC-DATA-001 / X-SEQ-004）：除 DOC 外无二进制文档处理代码（静态可查）；
  预览链失败不影响编辑链；断网时开/存/预览转换可用；无文档内容网络上行（可经网络核验）。
- **必需子集往返保真**（BR-DOC-DATA-002，REQ-DOC-003 验收）：标题/对齐/粗斜下/列表/基础表格/图片
  的 b-docx→编辑→docx 往返回归测试强制覆盖；每项至少有 DOCX XML、编辑模型 HTML 和 LO PDF 三层断言。
  当前仅标题/粗体已实测，其余在语料通过前只能标记为“必需待扩展”；部分支持项损失有用户可见明示。
- **LO 无界面与字体**（BR-DOC-DATA-001）：启动 Binder 不产生 LO GUI；工具损坏时 Binder 仍可启动、DOC command
  明确不可用；真实转换只以隔离 profile headless 子进程运行。字体语料在无系统 LO、无额外字体干净环境中通过
  文本层/嵌入字体/视觉三重断言，并覆盖复杂脚本与 emoji cluster。
- **转换原子性**（BR-DOC-STATE-001 / SM-DOC-01）：注入打开/保存转换失败，会话不建立/原文件不变、
  无半成品、可重试。
- **blockId 兜底**（BR-DOC-DATA-003）：转换后所有块级元素有 blockId；往返保留度实测并记录
  （DOC-02）。
- 量化 NFR（首开 <12s / 缓存后 <2s / 准确率 ≥90%，旧版目标）延 G5 标定复核（DOC-05）。

## §9 技术债务与盲区（诚实声明）

- **Pandoc / LibreOffice 供应链**（DOC-01）：Pandoc 3.10 arm64 已纳入受控资源并以 manifest 锁定；
  仍需补齐 x86_64 产物、许可证清单和发布 CI。LibreOffice 的稳定版本、补丁集、签名/公证与重建
  配方仍为发布硬门。
- **macOS 平台矩阵**（DOC-PLAT-001）：最低 macOS 版本、Intel 产物、双架构签名/公证与真机矩阵已登记，
  人类决定本轮暂不处理；不得以调用设备工具或设备字体规避。
- **LibreOffice GlyphFallback 字体债务**（BLOCK-REL-LO-001）：现有 family 级补丁不覆盖 CoreText
  字形级回退；必须在 `vcl/quartz/salgdi.cxx` 的 `FindFontSubstitute` 中优先选择 Binder 包内默认字体，
  重建并签名优化版 LO 后以 PPTX 语料复测。在本仓取得可重建源码前保持阻断，禁止系统 LO 规避。
- **data-block-id 往返保留度未实测**（DOC-02）：本文已定降级契约（重建锚），实测后回写。
- **Excel/PPT/音视频预览转换归属**（DOC-03）：**已随 PRV G2 联合解除（2026-06-11，A-PRV-M-T-01
  §2）**——Excel/PPT 预览转换归 DOC 预览链扩展（LibreOffice→PDF，受 X-SEQ-004 约束）；音视频
  系统解码归 PRV（非文档转换职责）。Excel/PPT 保真度实现期实测，不达另议须回写两文。
- **样式子集告知交互形态**（DOC-04）：延实现期人类确认，本文只锁"用户可知"语义。
- 转换缓存策略（旧版 file_cache + content_hash 模式可取材）延实现期，与 WS 控制态缓存协同设计。

## §10 后续动作

1. 落地后随 `npm run governance:generate` 注册：DOC-CONVERT 链路、BR-DOC-DATA-001/002/003、
   BR-DOC-STATE-001 规则、X-SEQ-004 约束、TERM-DOC-001~003、REQ_MAP 进 ADUS。
2. 续 A-DOC-M-S-01（SM-DOC-01 docx 开/存机），登记总图（docx 行 planned→active）与
   X-INTER-001 转移点（DOC↔ED 装载、DOC↔WS 落盘）。
3. 待人类宣布 DOC G2 闭合；PRV G1 时联合解除 DOC-03。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-11 | v1.0 | 初版（DOC G2 首件）：DOC 规则 SSOT——双链分离转换管线（编辑链 Pandoc / 预览链 LibreOffice→PDF，旧版 R-ED-M-R-01 v2.0 实证采纳）、转换生命周期与原子性、样式子集三级执行契约（R-ED-M-R-23 实证清单）、blockId 转换层兜底与往返降级契约；注册链路 DOC-CONVERT（P1）、规则 BR-DOC-DATA-001/002/003·BR-DOC-STATE-001、约束 X-SEQ-004、术语 TERM-DOC-001 StyleSubset；REQ_MAP（REQ-DOC-001~005）；债务（版本/过滤器/往返实测/PRV 归属/告知形态延后记明） |
| 2026-06-11 | v1.1 | DOC-03 解除（随 PRV G2 联合，A-PRV-M-T-01 §2）：Excel/PPT 预览转换归 DOC 预览链扩展（LibreOffice→PDF，X-SEQ-004 覆盖）、音视频系统解码归 PRV；§9 债务更新；保真度实测留实现期 |
| 2026-06-24 | v1.2 | 实现期处理逻辑细化（文档·设计·代码同步，移植旧版 binder-project 跑通链路）：新增 §2.1 编辑链处理逻辑（编辑模型为 HTML 非 markdown；打开 `pandoc docx+styles → html+raw_html+native_divs+native_spans + preserve-styles.lua + extract-media`，保存 `html → docx --reference-doc reference.docx`；HTML 15MB 上限；资源 reference.docx/preserve-styles.lua 经 bundle）、§2.2 docx 编辑流程（原生 docx→预览→「创建草稿进行编辑」doc.create_draft→编辑；`*.draft.docx`/生成 docx→直编辑；原文档不改动）、§4.1 样式子集↔TipTap 编辑器扩展映射表（必需/部分/增强 ↔ TextAlign/Underline/Table*/Image/TextStyle+Color/FontFamily/FontSize/Highlight/Sub·Superscript）；新增术语 TERM-DOC-002 编辑草稿 EditDraft；命令 doc.open_for_edit/save_from_edit/create_draft 见 A-DOC-I-P-01 v1.1~v1.3；预览链优化版 LibreOffice 打包阻断见 A-CORE-X-R-01 §9。规则/链路不变（BR-DOC-* + DOC-CONVERT），REQ_MAP 不变 |
| 2026-06-24 | v1.3 | docx 编辑渲染修复（设计·代码同步）：§2.1 去 `--standalone`（pandoc 只输出 <body> 正文片段，不含 <head>/<style> CSS——否则 CSS 被当代码块渲染）；明确前端须用 `generateJSON(html, extensions)` 按 HTML 解析绕过 tiptap-markdown 对 setContent(string) 的 markdown 劫持（旧项目编辑器无 markdown 扩展故 HTML 原生解析正常，本项目为 md 编辑装了该扩展需显式绕过）。规则/术语/链路不变 |
| 2026-07-04 | v1.4 | 术语统一清扫（承 2026-07-04 人类三轮裁决）：正文禁用别名替换——内联辅助→局部修改、自动补全→辅助续写、对话编辑→Diff Edit、Chat↔Build 接管→构建移交/回落、chat/chat-agent 回合→代理回合；文件名与变更记录历史行不改写 |
| 2026-07-09 | v1.5 | 注册 TERM-DOC-003 b-docx/BDocx：明确原生 `.docx` 只走 LibreOffice→PDF 预览路径，b-docx 才进入 HTML / ProseMirror 编辑链；同步 EditDraft 为 b-docx 来源形态，补充文件树新建与 Agent 创建 docx 文档的编辑分类 |
| 2026-07-09 | v1.6 | 补充 BDocx 身份持久化实现约定：`*.draft.docx` 命名兼容识别，文件树新建/Agent 创建 `.docx` 通过 `.binder/bdocx_manifest.json` 登记并由 `ws.list_docs.is_bdocx` 投影；身份清单不替代 docx 字节内容权威 |
| 2026-07-09 | v1.7 | 接线 blockId 转换层兜底实现约定：`docx_to_html` 返回前必须为块级元素注入/去重 `data-block-id`，AG/DE/ED 共用该 canonical HTML 作为 b-docx 编辑锚模型 |
| 2026-07-09 | v1.8 | 补齐 AgentEdit 消费 BDocx 的转换契约：AG/REF/DE 读取统一走 DOC HTML 编辑模型快照，禁止 WS 文本读取 `.docx` 字节；既有 BDocx diff 未进入当前编辑器 HTML 草稿时禁止后台 raw apply，保存仍由 `doc.save_from_edit` 转换落盘 |
| 2026-07-12 | v1.9 | 三态实现回写：`doc.save_from_edit` 保存链补加载基线与 `override` 语义，转换成功后经 WS baseline guard + `save_atomic` 落盘；磁盘偏离基线时返回 `WS_SAVE_CONFLICT` 且原 docx 不变 |
| 2026-07-21 | v2.0 | DOC-BUNDLED-RUNTIME-01：新增受校验包内工具清单、resource_dir 唯一路径和构建前校验设计；Pandoc 版本/哈希/许可证锁定。明确 LO 的 GlyphFallback 源码改造、重建、签名与公证仍为发布阻断，当前包内 LO 转换 SIGABRT 不得由系统回退掩盖。 |
| 2026-07-21 | v2.1 | DOC-LO-RELIABILITY-FONT-PLAN-01：补充 LO 无 GUI 启动、稳定 headless、CoreText 包内字体注册及 GlyphFallback 源码策略；将 Pandoc 样式能力改为“已实测 / 必需待扩展 / 尽力 / 模板继承 / 不处理”五档，并锁定三层回归验收。DOC-PLAT-001 仅登记暂缓。 |
| 2026-07-21 | v2.2 | 审计修正：工具失效改为 DOC 功能降级而非 Binder 启动失败；LO profile/临时文件收敛至应用缓存；字体完整性扩展为整个嵌套 app 清单与签名；GlyphFallback 改为 shaping run/grapheme cluster 覆盖；五档样式明确为待 PI-DOC-02 实现的目标模型。 |
| 2026-07-22 | v2.3 | 人类确认继续 LO 源码融合路线：在 §2.3 增加 Font Trace 与只读 Render Proxy 前置，明确先分流 family 规则、profile 与 glyph fallback，再进入 CoreText/GlyphFallback 源码裁决；原生 Office 源文件始终不修改。 |
| 2026-07-22 | v2.4 | S0 实现回写：Font Trace/空 FontPlan 已开始落地并通过 DOCX/PPTX/真实参考文档测试；源码 tree 已确认有 headless/quartz 路径，完整源码重建仍受本机构建资源阻断。 |
| 2026-07-22 | v2.5 | 通用方案校正：明确具体字体只作语料；移除运行时旧 `fontsubst.xcu` 宿主默认字体替换，未知字体不再被外部规则静默定向，等待可验证的包内 FontPlan 与源码 GlyphFallback。 |
| 2026-07-22 | v2.6 | S1 源码证据回写：确认 Quartz `create_SalInstance()` 无条件初始化 `NSApplication`，与既有崩溃栈一致；`--headless` 不等于不触发 AppKit。空间恢复后，实际编译仍由完整 Xcode/JDK 前置门控。 |
