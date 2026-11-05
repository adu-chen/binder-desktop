---
文档编号：CORE-X-R-05
文档状态：A
负责模块：CORE
文档职责：LO 依赖审计与原生替换准备
上游约束：A-CORE-C-P-01_APC.md（§1/§4.1/§5/§9）、A-CORE-C-D-01_产品概述.md（REQ-CORE-004/009）、A-CORE-X-D-02_Agent1.0冻结能力基线.md、A-DOC-M-D-01_文档转换功能需求描述.md（REQ-DOC-001~006）、A-DOC-M-T-01_文档转换技术设计.md（BR-DOC-DATA-001/002/003、BR-DOC-STATE-001、X-SEQ-004）、A-DOC-I-P-01_文档转换接口协议.md、A-DOC-M-S-01_文档转换状态机设计.md、A-PRV-M-D-01_预览输出功能需求描述.md（REQ-PRV-001~005）、A-PRV-M-T-01_预览输出技术设计.md（BR-PRV-DATA-001/002）、A-PRV-I-P-01_预览输出接口协议.md、A-ED-M-T-01_编辑器技术设计.md（BR-ED-DATA-002/003）、A-CORE-X-R-01_门禁闭合债务分类台账.md（BLOCK-REL-LO-001、DOC-PLAT-001）
直接承接：本文不承接 REQ_MAP，不定义 RULE/CHAIN/MODULE/TERM/CONSTRAINT；作为后续“LO 原生替换”立项的输入，承接对象为 DOC/PRV 的技术设计、开发计划与验证卡
使用边界：本文是时点审计报告与替换准备建议（程序级 L4），不是实现方案批准书；代码不得追溯本文；本文提出的需求/规则/文档修改均为提案，须按 ADP 另行立项并由人类确认；不修改 ADU/ADP/AGENTS/CLAUDE/ADUS，不修改现有需求/规则正文
变更要求：替换方案、支持矩阵或 LO 去留决策变化时追加审计记录；后续实施完成后，应新建或更新对应 Issue Trace、L2 设计与验证卡，而不是把本文改写成实现文档
治理成熟度：draft
---

# Binder LO 依赖审计与原生替换准备（2026-11-04）

## §0 审计结论（先看）

### 0.1 一句话回答

**今天仍需要 LO 的用户能力只有三类：原生 DOCX/DOC 只读预览、PPTX/PPT 只读预览、Office 文档→PDF 导出
（docx/xlsx/pptx）以及 docx/pptx 的 PDF 打印。**（xlsx 的表格式预览与 HTML 表格打印不依赖 LO；见 §3.2。）
编辑链（Pandoc→HTML→TipTap）、xlsx/xls/csv 表格式预览（calamine/前端）、原生 PDF/图片/音视频预览、
AG 读取 docx、`doc.create_draft` 草稿复制都不依赖 LO。

但 LO 不是一个“只做预览的可替换库”：它同时承担了分页/排版渲染、字体解析与回退、PDF 文本层与嵌字体、
legacy 格式解析、包内运行时完整性、构建/测试/发布门等职责。**真正难替换的是渲染层，不是 DOCX 解析层。**

### 0.2 量化结论

| 维度 | 审计结果 | 证据 |
|---|---|---|
| 显式点名 LO 的 REQ | 4 个：REQ-DOC-003（验收句）、REQ-DOC-004、REQ-DOC-005、REQ-PRV-001 | §3.1、§4.5 |
| 当前实现绑定 LO 的 REQ | 3 个：REQ-CORE-004、REQ-PRV-002（PPTX/PDF 部分）、REQ-PRV-004（PDF/打印） | §3.2 |
| 间接受 LO 供应链影响的 REQ | 2 个：REQ-CORE-009、REQ-PRV-005（离线） | §3.2 |
| LO 专用 Rust 代码 | `libreoffice_runtime.rs` 362 行 + `lo_worker.rs` 484 行；`tool_runtime.rs`/`convert.rs`/`prv/mod.rs` 另有 LO 分支 | §4.1 |
| LO 资源体积 | `LibreOffice.app` 实测 380 MB；Pandoc 180 MB | §5.3 |
| LO 相关文档引用 | 约 130 处，集中在 DOC/PRV L1/L2/L3、债务台账、验证卡、ADUS | §4.5 |
| 直接 LO 测试 | 仅 1 个真实 E2E（`docx_to_pdf_end_to_end_when_libreoffice_available`）和若干参数/环境单测；PRV 真实预览路径无独立测试 | §4.4、§5.4 |
| 当前发布状态 | `BLOCK-REL-LO-001` 未解除；S5 未完成；干净设备/签名/公证/字体验收未闭合 | §5.1 |
| 原生替换关键缺口 | 渲染器/分页、字体 shaping、PDF 文本层、打印、PPTX 渲染、XLSX 样式、legacy、安全沙箱、语料与黄金样本 | §6 |

### 0.3 审计判断（不是验收结论）

1. **LO 当前不是“已完成、可稳定依赖”的基础设施。** 它在非沙箱 macOS 会话可工作，但在受限进程中会因 AppKit
   注册以 `signal=6` 退出；S5 干净设备、签名、公证、字体语料仍未闭合。以它为基础继续加功能，会把发布阻断
   继续向后传递。
2. **P1 spike 证明了 DOCX/PPTX 的“解析/保存”可行性，但没有证明“预览”可行性。** 渲染层、分页、字体、
   PDF、打印在仓内均无实现；不能据此宣称“已可替换 LO”。
3. **替换会触碰治理边界。** 若把 TypeScript 引擎放到前端 WebView 解析二进制文档，会与 `X-SEQ-004`
   （二进制文档解析/生成必经 DOC）和 `BR-ED-DATA-002`（ED 不含 docx 二进制路径）冲突。必须先决定
   “引擎放在 DOC 后端 / DOC 拥有的隐藏 WebView / 前端 worker”，再谈实现。
4. **建议先做 P0 决策与语料，不要直接开工。** 先锁定支持矩阵、渲染器来源、PDF 路径、字体授权、
   legacy 策略、Pandoc 关系，再进入 DOCX 原生预览 MVP；PPTX 与 legacy 单独设决策门。

---

## §1 审计范围、方法与证据等级

### 1.1 时点与基线

| 项 | 值 |
|---|---|
| 审计日期 | 2026-11-04 |
| 代码基线 | `HEAD = 665bffc` |
| 工作树 | 存在 69 项未提交变更（AG/DOC/PRV/ED 等），**本文未修改、未回滚、未纳入审计结论** |
| 治理基线 | `npm run governance:audit` 于本次审计时 `status: passed`，`findings: []`，`warnings: []` |
| LO 资源 | `src-tauri/resources/libreoffice/LibreOffice.app`，实测 380 MB，mtime 2026-07-24 |
| LO 清单 | `resources/tools/manifest.json`：`LibreOfficeDev 25.8.7.3 30742500...`，`release_status: s4_runtime_integrated` |
| 原生候选 | `.p1-spike/`（GenOffice `docx-engine`/`pptx-engine`，非产品代码） |

> 审计期间的环境说明：首次 `governance:audit` 于 2026-11-04 22:39 左右通过（`findings: []`）。
> 随后工作区出现并行的 `.claude/worktrees/fervent-elion-5fe830/` 工作树（23:03），审计脚本会把它纳入
> `_标注延期清单.md` 扫描，导致 `TRIAGE_RECORD_DUPLICATE`（5 条，均来自该工作树内 R-02 的副本）。
> 该失败与本文无关；本文未修改该工作树、R-02 或审计脚本。当前以 `git diff --check` 检查本文，
> 治理审计状态应以排除并行 worktree 后的结果为准。

### 1.2 方法

1. **需求层**：读取 REQ-CORE/DOC/PRV 正文、REQ_MAP、ACCEPTANCE 与 open_issues。
2. **规则层**：读取 BR-DOC-*/BR-PRV-*/BR-ED-*/X-SEQ-004 与 @GOV 映射。
3. **代码层**：静态扫描 `src-tauri/src`、`src`、`scripts`、`resources`，逐调用点确认 LO 的真实角色。
4. **测试层**：扫描 Rust/前端测试、测试治理注册表、验证卡与历史回归证据。
5. **运行时证据**：运行 `governance:audit`；运行 P1 spike 的只读 smoke/parse 脚本；**未启动 LO GUI、
   未执行真实 LO E2E**（避免在审计中制造 GUI/Crash Reporter 副作用）。
6. **证据冲突核对**：对“LO 20 次成功”“SIGABRT”“沙箱 signal=6”“S5 未闭合”等既有记录做时点与口径对齐。

### 1.3 证据等级

| 等级 | 含义 | 本文用法 |
|---|---|---|
| E1 | 本次直接读取的代码/文件/命令输出 | 事实陈述 |
| E2 | 既有测试、验证卡、债务台账、Issue Trace 的记录 | 标注来源与时点，不当作本次复验 |
| E3 | 基于代码结构与需求的推断 | 明确写“审计判断/推断”，不写成已验证 |

> 本文不声称“LO 在所有环境失败”或“原生方案已可行”。LO 的健康度结论见 §5，原生可行性结论见 §6–§8。

---

## §2 LO 在系统中的真实角色

### 2.1 当前架构（只列与 LO 有关的路径）

```
用户点击 docx/doc/pptx/ppt
  → PRV PrvPanel（mode = pdf）
  → prv.preview（IPC）
  → prv::render_preview（PRV 编排）
  → doc::libreoffice_runtime::convert_to_pdf
  → doc::lo_worker::LoWorker::execute
  → 包内 soffice --headless --convert-to pdf:writer_pdf_Export|calc_pdf_Export|impress_pdf_Export
  → 输出目录扫描 PDF
  → 复制到 app_data_dir/doc-runtime/preview-cache/<key>.pdf
  → asset 协议逐文件授权
  → iframe 加载 PDF

xlsx/xls/csv/tsv
  → TablePreview（前端）→ prv.table_data（calamine，xlsx/xls）或前端 CSV 解析
  → HTML 表格（不经 LO）

docx 编辑
  → doc.create_draft（字节复制）→ doc.open_for_edit（Pandoc）→ HTML/TipTap
  → doc.save_from_edit（Pandoc + WS 原子落盘）
  → 不经 LO

Office → PDF 导出
  → prv.export（docx/xlsx/pptx）→ prv::render_preview → 同一条 LO 预览链
```

### 2.2 LO 承担的 8 类职责

| # | 职责 | 当前落点 | 为什么是 LO | 去掉 LO 的直接影响 |
|---|---|---|---|---|
| R1 | Office 二进制→PDF 转换 | `doc/libreoffice_runtime.rs:254`、`doc/lo_worker.rs:199` | LO 的 Writer/Calc/Impress PDF 导出 filter | 预览与 PDF 导出同时失去转换器 |
| R2 | 分页/排版/渲染 | LO 内部（`soffice` 进程） | LO 负责 Word/PPT 布局、分页、页眉页脚、浮动对象、表格等 | 必须自建或引入渲染器；这是最大缺口 |
| R3 | 字体解析与回退 | LO `SystemFontList` + `binder_font_identity.patch` + `font_trace`/`font_plan` 诊断 | CoreText 名字映射、family 别名、缺字回退都在 LO 内 | 需要原生字体解析、shaping、line-break、包内字体注册 |
| R4 | PDF 文本层/嵌字体 | `pdf_filter` 的 `UseTaggedPDF/Embed*Fonts` 选项 | LO PDF 导出负责文本层与字体嵌入 | 需要 PDF 生成器或 WKWebView PDF 适配，并验证可选中/可搜索 |
| R5 | legacy 格式解析 | LO filter（`.doc/.ppt/.xls`） | LO 自带 CFB/旧格式过滤器 | 原生引擎不覆盖；需明确“不支持”或另找转换器 |
| R6 | 包内运行时/完整性 | `tool_runtime.rs` LO manifest、`resource_dir()` 唯一路径 | 以包内 `soffice` 替代系统 LO | 需要新的 native runtime 清单与完整性校验；或整体删除 |
| R7 | 构建/测试/发布门 | `verify_bundled_tools.sh`、`deploy_lo.sh`、`build_lo_macos.sh`、`BLOCK-REL-LO-001` | 打包、哈希、签名、公证、干净设备验收 | 构建链和发布门需要重写；但也会移除 380 MB 嵌套 app 的签名负担 |
| R8 | UI/缓存/错误语义 | `PrvPanel`、`prvService`、`prv/mod.rs` 缓存、`DOC_PREVIEW_*` | LO 转换是慢操作、可能失败、可能超时/崩溃 | 需要新的错误码、缓存键、进度/失败态；不能只换后端 |

> **关键判断**：R1 是“转换器”问题，R2–R4 是“渲染器”问题，R5 是“格式覆盖”问题，R6–R8 是“运行时工程”问题。
> 只替换 R1 而不解决 R2–R4，得到的只是“解析成功但无法正确显示”的预览。

---

## §3 需求追溯矩阵：哪些需求仍需要 LO

### 3.1 显式点名 LO 的需求（需求文本本身就是 LO 绑定）

| 需求 | 绑定位置 | 原文要点 | 影响 |
|---|---|---|---|
| **REQ-DOC-003** | `A-DOC-M-D-01` §5 acceptance | “预览链可按 LO 语料实测结果只读查看” | 验收标准把“LO 语料”作为预览基准；替换后必须改成新的原生语料基准 |
| **REQ-DOC-004** | `A-DOC-M-D-01` §5 summary/acceptance | “提供原生 docx→PDF 的本地预览转换（LibreOffice 预览链）” | 需求把“PDF + LibreOffice”写死；替换需改 REQ 文本（或明确 PDF 仍由 native 生成） |
| **REQ-DOC-005** | `A-DOC-M-D-01` §5 summary/acceptance | “Pandoc/LibreOffice 与默认回退字体必须随 Binder 发布包内置” | 包内运行时契约点名 LO；替换需改成 native runtime + 字体，Pandoc 保留 |
| **REQ-PRV-001** | `A-PRV-M-D-01` §5 summary/acceptance | “消费 DOC 预览链（LibreOffice→PDF）”“以 LO 语料实测为准” | PRV 的验收基准与 DOC 预览链都绑定 LO |

### 3.2 需求未点名 LO、但当前实现绑定 LO

| 需求 | 当前实现 | LO 依赖性质 | 去掉 LO 后是否仍可满足 | 必须补齐 |
|---|---|---|---|---|
| **REQ-CORE-004** 多格式预览 / 原生 DOCX 只读 | docx/pptx 预览与 office→PDF 导出走 LO | 实现绑定（产品需求只要求“可预览/可打印导出”） | 可以，但需原生预览/导出 | 原生渲染器 + PDF/打印 |
| **REQ-PRV-002** Excel/PPT 预览（无编辑入口） | xlsx/xls 走 calamine 表格预览（**不需 LO**）；pptx/ppt 走 LO→PDF；xlsx PDF 导出走 LO | 部分实现绑定 | PPTX 需原生渲染；XLSX 表格预览已满足“只读预览”，但“样式保真/PDF”仍需补齐 | PPTX 渲染；XLSX 样式渲染或明确范围 |
| **REQ-PRV-004** 打印与导出 | PDF 导出走 LO；docx→md 走 Pandoc；打印 PDF 由 WKWebView 打印 | 实现绑定（需求只说“导出格式清单 G2 定”） | 可以，但需原生 PDF 或改导出格式 | 原生 PDF/打印；或产品接受非 PDF 导出 |
| **REQ-CORE-009 / REQ-PRV-005** 离线边界 | 依赖包内 LO 离线转换 | 间接供应链 | 可以，native 运行时同样可离线 | 包内 fonts/runtime、无网络 |
| **REQ-DOC-001/002** BDocx 打开/保存 | Pandoc→HTML→DOCX，**不经 LO** | 无 | 保持 | 无（预览与编辑相互独立；LO 失败不影响草稿/编辑，见 §3.4） |

### 3.3 明确不需要 LO 的需求（避免扩大替换范围）

以下能力当前均不经 LO，替换 LO 时不应顺手改动：

- **编辑链**：REQ-DOC-001/002/003（正文转换、样式子集、保存重开）、REQ-ED-001~014、DE/WS/AG 的编辑与 diff。
- **表格式预览**：xlsx/xls（calamine）、csv/tsv（前端解析）。
- **原生预览**：pdf（WKWebView 原生查看器）、图片、音视频（asset 协议 + 系统解码）。
- **AG 读取**：docx→HTML/Markdown（Pandoc）；项目分析/引用读取；不经过 LO。
- **草稿创建**：`doc.create_draft` 是字节复制；`FileTree` 还有“跳过预览直达草稿编辑”的入口。
- **字体列表**：`fonts.list_fonts` 走 CoreText，不经 LO。
- **分页呈现**：ED 的 TipTap 分页扩展是纯呈现层，不经 LO。

### 3.4 一个容易误判的耦合：预览与编辑没有硬前置关系

初审时容易把 `REQ-CORE-004` 的“副本在预览成功后才进入可编辑 LogicalState”理解为“LO PDF 预览成功是编辑前置”。
核对代码后应纠正：

- `PrvToolbar` 的“创建草稿进行编辑”只按路径判定（`canCreate = mode === "pdf" && canCreateDraft(path)`），
  **不依赖预览 `status === "ready"`**；打印按钮才依赖 `status === "ready"`。
- `PrvPanel.enterEdit` 直接调用 `doc.create_draft` → `openDocument`，不要求 PDF 已生成。
- 文件树另有 2026-08-04 加入的“跳过预览直达草稿编辑”入口。
- `doc.create_draft` 是字节复制，`doc.open_for_edit` 用 Pandoc，均不需要 LO。

因此：**LO 预览失败不会阻断草稿创建/编辑**，只会让用户失去“先只读预览、再决定是否编辑”的 UX 步骤。
原生替换的验收应覆盖“预览失败时草稿/编辑仍可用”的独立降级，而不是把两者绑成一个成功条件。
`REQ-CORE-004` 的“副本预览成功”指 BDocx 的 DOCX→HTML 转换结果，不是 LO 的 PDF 预览；这条验收不应被
误用来要求 LO 成功。

### 3.5 需求层结论

- **4 个 REQ 文本显式绑定 LO**；不做文本变更就无法宣称“彻底拆分”。
- **3 个 REQ 是当前实现绑定**；替换后需求本身可保持不变（除非 PDF 导出格式改变）。
- **2 个离线 REQ 间接受供应链影响**；只要 native 运行时离线可用，需求可保持。
- 因此“彻底替换 LO”至少包含：**代码替换 + 4 个 REQ 文本更新 + 1 条 RULE 更新 + 1 个 TERM 退役 +
  ADUS 重新生成 + 测试/验证卡更新**，不是一次纯代码重构。

---

## §4 全量触点清单

> 统计口径：`src`、`src-tauri/src`、`scripts`、`resources`、`docs`，排除 `target/`、`node_modules/`、`.p1-spike/node_modules/`。
> 行号为审计时点行号，后续改动可能漂移。

### 4.1 后端代码（Rust）

| 文件/位置 | 角色 | 与 LO 的关系 | 替换动作 |
|---|---|---|---|
| `src-tauri/src/doc/libreoffice_runtime.rs:1-362` | 预览链运行时 | `PreviewKind`、PDF filter、soffice 路径解析、隔离 profile、命令构造、PDF 扫描、`convert_to_pdf` | 删除或替换为 native preview runtime |
| `src-tauri/src/doc/lo_worker.rs:1-484` | LO 子进程 Worker/Supervisor | 120s 超时、崩溃检测、stderr 脱敏、过期目录清理、字体诊断 | 删除；native 渲染需要新的超时/隔离/错误模型 |
| `src-tauri/src/doc/tool_runtime.rs:27,41-42,133-139,206-215` | 包内工具清单/校验 | manifest 的 `libreoffice` 与 `font_plan`；启动时哈希 `soffice` | 移除 LO spec；保留/重建 native runtime 清单 |
| `src-tauri/src/doc/convert.rs:35-39,45,98,115` | 双链抽象 | `ConvertChain::Preview`→`ConvertTool::LibreOffice`；`ensure_chain_dependency`/`tool_for` | 删除 LO 枚举/守卫，或改造成 native preview 的 probe；注意当前生产代码未使用这两个函数（仅测试） |
| `src-tauri/src/doc/font_trace.rs:252,327,380` | OOXML 字体取证/FontPlan | 只读解析 docx/pptx 字体引用；被 LO 诊断调用 | **可复用**为 native 字体解析/诊断；不要随 LO 一起删除 |
| `src-tauri/src/prv/mod.rs:7,74-128,134-175` | PRV 预览/导出编排 | `preview_kind` 映射 docx/xls/ppt；`render_preview` 调 `convert_to_pdf`；`export` 的 office→pdf 分支 | 改为 native render/export；缓存键需加渲染器版本 |
| `src-tauri/src/prv/routes.rs:8-35` | `prv.preview` 路由 | 传 relPath/ext，返回 PDF 路径并授权 asset | 保持 IPC 形状或扩展 `format`；替换内部实现 |
| `src-tauri/src/doc/routes.rs:25-72,75-150` | 草稿/编辑路由 | `doc.create_draft` 字节复制（无 LO）；`open_for_edit` 用 Pandoc（无 LO） | 不需要因 LO 替换而改；但需回归“预览失败→草稿/编辑仍可用” |
| `src-tauri/src/lib.rs:216,357,428` | command registry/dispatch/启动 | 注册 `prv.preview`；dispatch 路由；`tool_runtime::initialize` 校验 LO | 替换启动校验与路由目标；移除 LO 初始化 |
| `src-tauri/Cargo.toml:45` | 测试依赖 | `filetime` 仅用于 LO Worker 过期目录测试 | 若删除 Worker，可移除该 dev-dependency |
| `src-tauri/resources/tools/manifest.json:13-24` | 工具清单 | LO 版本/路径/SHA、font_plan 路径/SHA | 移除 LO 项；新增 native runtime/字体清单（若需要） |
| `src-tauri/resources/libreoffice/**` | 包内 LO | 380 MB app + `font-plan.json` + README | 删除；native 字体资源另立目录 |
| `src-tauri/resources/reference.docx` | Pandoc 参考样式 | 编辑链用，**不是 LO 资源** | 保留 |

### 4.2 前端代码与用户可见文案

| 文件/位置 | 角色 | LO 绑定 | 替换动作 |
|---|---|---|---|
| `src/components/prv/PrvPanel.tsx:14,34,140,175,180` | 预览面板 | `mode="pdf"` 走 `previewDoc`；加载文案“首次启动 LibreOffice 可能稍慢”；错误提示“请确认已安装 LibreOffice” | 改为 native 渲染文案；**删除“已安装 LibreOffice”提示**（系统 LO 本就不允许） |
| `src/components/prv/PrvToolbar.tsx:12-13,17,61-65` | 预览工具栏 | docx/xls/pptx 的 PDF 导出/打印/创建草稿入口 | 保持交互；导出后端换 native |
| `src/components/prv/TablePreview.tsx` | 表格预览 | 无 LO | 保留 |
| `src/services/prvService.ts:4,26-38` | 预览/导出 service | `prv.preview` 返回 PDF asset URL；`prv.export` | 保持命令名；如返回 HTML/PDF 双形态，扩展类型 |
| `src/services/docService.ts:5-7,344-415` | DOC 编辑链 service | 注释提到 LO 预览；编辑本身用 Pandoc | 更新注释；不改编辑协议 |
| `src/components/ed/Editor.tsx:1623` | 编辑器导出 | 保存后 `prv.export`；PDF 走 LO | 保持“先保存再导出”；替换后端 |
| `src/components/ws/FileTree.tsx:184-205,282-299` | 文件树导出/草稿 | `exportTargets` 对 docx 给 pdf/md、对 xlsx/pptx 给 pdf；草稿直达入口 | 保持入口；替换 PDF 后端 |
| `src/services/prvRuntime.test.ts` | PRV 测试 | 只测命令白名单/CSV/表格，不测真实预览；`previewDoc` 仍 mock 旧 `bytes` 形状 | 增加 native preview 契约测试；修正 stale mock |

### 4.3 构建、资源与脚本

| 文件/位置 | 角色 | LO 绑定 | 替换动作 |
|---|---|---|---|
| `package.json:8,23` | `build` 先跑 `verify:bundled-tools` | 构建期校验 LO | 移除 LO 校验；native 资源校验按需重建 |
| `scripts/verify_bundled_tools.sh:14-31` | 工具硬门 | 哈希 `soffice` 并执行 `soffice --version` | 删除 LO 行；注意 `--version` 在无 GUI 环境会触发 AppKit，是构建期风险 |
| `scripts/deploy_lo.sh:40-81` | LO 部署 | 复制 `LibreOffice.app`、`chmod`、`--version` 探测；注释仍描述旧 `BINDER_FORCE_DEFAULT_FONTS` 补丁 | 删除；若保留过渡期，修正注释与 `--version` 探测 |
| `scripts/build_lo_macos.sh:1-160` | LO 源码构建 | 固定 commit、LODE、Xcode、patch、`make build` | 删除或归档；当前脚本 `INSTDIR` 未定义、postprocess 路径有缺陷（见 §5.3） |
| `scripts/postprocess_lo_macos.sh:23` | LO 瘦身 | 第 23 行存在独立执行字符串 `"en-US zh-CN zh-TW ja ko"`，`set -e` 下会失败 | 删除；这解释了“构建脚本与产物状态不一致”的风险 |
| `patches/libreoffice/binder_font_identity.patch` | LO 字体身份补丁 | 注册英文名/Office 别名/系统字体回退 | 删除或归档；其中“回退到 PingFang/Arial 等系统字体”与“无设备字体”目标冲突（见 §5.2） |
| `patches/libreoffice/binder_tarball_mirror_fallback.patch` | LO 构建依赖镜像补丁 | 第三方 tarball 回退 | 删除或归档 |
| `src-tauri/tauri.conf.json:42-43` | 打包资源 | `resources/` 全量打包，含 LO | 删除 LO 后包体显著减小；native 字体/资源另计 |
| `.gitignore:16` | 忽略 LO app | 380 MB 不入库 | 删除条目或保留历史说明 |
| `src-tauri/resources/libreoffice/README.md` | LO 资源说明 | 版本/体积/构建参数/字体状态 | 删除或改成“已移除”说明；当前内容与实测有漂移（见 §5.3） |

### 4.4 测试与测试治理

| 位置 | 内容 | 问题/替换动作 |
|---|---|---|
| `src-tauri/src/doc/libreoffice_runtime.rs:276-362` | 5 个单测 + 1 个 E2E | 删除 LO 环境/参数测试；保留“错误形状/隔离/不静默降级”语义到 native runtime 测试 |
| `src-tauri/src/doc/lo_worker.rs:361-484` | 超时/崩溃/过期目录/信号测试 | 删除或改写为 native 渲染超时/取消测试 |
| `src-tauri/src/doc/convert.rs:130-180` | 双链/依赖守卫测试 | 删除 LO 断言；若 native 保留“预览链依赖”语义，改测 native probe |
| `src-tauri/src/doc/tool_runtime.rs:220-249` | 包内工具解析/缺失测试 | 移除 LO 断言；保留 Pandoc/资源校验 |
| `.governance/test-governance.json:319-323` | `execution_constraints` 明确要求“macOS bundled LibreOffice E2E 需要 AppKit 注册” | 替换后删除该约束；否则测试治理仍把 LO 当运行时前提 |
| `docs/40_validation/A-VAL-X-V-02_核心编辑与DiffEdit保存验证.md:154-155` | 沙箱 FAIL signal=6 / 非沙箱 PASS | 保留为历史证据；替换后新增 native 渲染验证卡 |
| `docs/40_validation/A-VAL-X-V-04_Agent1.0验证卡.md` 多处 | 受限测试因 LO signal 6 失败 | 保留历史；不再作为 native 替换后的常态约束 |
| ADUS `BR-DOC-DATA-001` 覆盖测试 | `libreoffice_runtime.rs` 多个测试挂在规则下 | `governance:generate` 后路径需指向 native runtime；测试数不得减少（替换需等价或更多） |

### 4.5 文档与治理引用（约 130 处）

按引用密度排序（`LibreOffice|LO 语料|LO PDF|LO 转换|soffice` 计数，审计时点）：

| 文档 | 计数 | 关键位置 |
|---|---|---|
| `docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md` | 26 | §2 双链、§2.3 LO 专项、§3 BR-DOC-DATA-001、§4 样式表、§8 测试、§9 债务 |
| `docs/01_modules/DOC/A-DOC-M-D-01_文档转换功能需求描述.md` | 17 | D-J4/D-J5、D3、REQ-DOC-003/004/005、§6、DOC-01 |
| `docs/00_core/A-CORE-X-R-01_门禁闭合债务分类台账.md` | 16 | §9 `BLOCK-REL-LO-001`、`DOC-PLAT-001` |
| `docs/01_modules/DOC/A-DOC-X-L-01_文档转换开发计划.md` | 9 | PI-DOC-03、S0–S5、构建配方 |
| `docs/01_modules/PRV/A-PRV-M-T-01_预览输出技术设计.md` | 8 | §2 管线归属、§9 债务 |
| `docs/01_modules/PRV/A-PRV-M-D-01_预览输出功能需求描述.md` | 8 | P-J1、REQ-PRV-001/002/004、§6 |
| `docs/01_modules/DOC/A-DOC-I-P-01_文档转换接口协议.md` | 8 | §3 预览转换、§6 债务 |
| `docs/40_validation/A-VAL-X-V-02_核心编辑与DiffEdit保存验证.md` | 6 | 沙箱/非沙箱对照 |
| `docs/00_core/A-CORE-C-R-01_ADUS.md` | 6 | 规则覆盖、TERM-DOC-004、链路 |
| `docs/40_validation/A-VAL-X-V-04_Agent1.0验证卡.md` | 5 | 受限测试 LO signal 6 |
| 其他 | 1–3 | ED/SYS/路线图/进度台账/前端计划 |

> 这些不是“多余注释”，而是**需求/规则/验收/发布门的正文**。代码替换后若不同步，audit 仍会认为
> “预览链=LibreOffice”，需求与实现再次分叉。

### 4.6 错误码 / 日志 / 缓存语义

| 对象 | 当前 | 替换注意 |
|---|---|---|
| `DOC_LIBREOFFICE_MISSING` | 包内 LO 缺失 | native 后不应再出现；若保留兼容，需明确“旧客户端/旧数据”语义 |
| `DOC_PREVIEW_START_FAILED` | headless 启动失败 | native 渲染可能没有“启动”概念，需新的 `DOC_RENDER_START_FAILED` 或复用 |
| `DOC_PREVIEW_TIMEOUT` | 120s 超时 | native 渲染仍需超时/取消；保留错误形状 |
| `DOC_PREVIEW_WORKER_CRASH` | signal 检测 | native 渲染可能崩溃/内存溢出；需要等价隔离 |
| `DOC_PREVIEW_FAIL` | 转换失败 | 保留为统一失败，附脱敏诊断 |
| `PRV_UNSUPPORTED` | 格式不支持 | 若收窄 legacy 支持，必须继续明确报错，不静默空白 |
| `PRV_IO_ERROR`/`PRV_SCOPE_FAILED` | 缓存/授权 | 保留；native HTML 预览还涉及媒体授权 |
| 预览缓存键 | `abs_path + size + mtime` | **必须加入 renderer_version + font_plan_version + format**，否则升级后可能命中旧渲染 |
| 预览缓存位置 | `app_data_dir/doc-runtime/preview-cache` | 可保留；若 HTML 预览，需同样的生命周期/清理策略 |
| `[binder:lo]` 日志 | 诊断 | 替换为 `[binder:preview]`；仍不得包含正文 |

---

## §5 当前 LO 方案健康度与证据冲突

### 5.1 功能证据：非沙箱可工作，但不是发布证明

| 证据 | 结论 | 时点/来源 |
|---|---|---|
| 沙箱内 E2E | `DOC_PREVIEW_WORKER_CRASH signal=6`；崩溃栈为 `NSApplication → GetCurrentProcess → _RegisterApplication` | 2026-08-01，`A-VAL-X-V-02:154` |
| 非沙箱同一 E2E | `PASS`，生成 PDF；Font Trace 72 refs | 2026-08-01，`A-VAL-X-V-02:155` |
| 风险接受 | 沙箱失败不作为产品缺陷裁决；但不得删除/`ignore` 该测试；干净设备验证仍未完成 | `DOC-LO-SANDBOX-VALIDATION-01` |
| S0–S4 | 文档称已完成；S5（全嵌套 app、签名/公证、干净设备语料）未完成 | `A-DOC-X-L-01` |
| 发布门 | `BLOCK-REL-LO-001` 未解除 | `A-CORE-X-R-01:164-209` |
| README | 声称“20 次连续 docx→PDF 成功” | `resources/libreoffice/README.md` |

**审计判断**：LO 在允许 AppKit 注册的普通 macOS 会话中可工作；受限进程中的 signal=6 是环境相关，
但不能据此证明“打包应用在干净设备可用”。S5 未完成，`BLOCK-REL-LO-001` 仍有效。既不能把
“非沙箱通过”当成发布通过，也不能把“沙箱失败”当成产品必坏；正确结论是**发布态未验证**。

### 5.2 字体确定性：补丁在，但回退仍指向系统字体

E1 事实：

- `binder_font_identity.patch` 已编译进当前二进制：`nm -a .../libvclplug_osxlo.dylib` 可见局部符号
  `SystemFontList::AddEnglishAlias(CoreTextFontFace*)`。
- 补丁的层 2/层 3 回退表是 **系统字体**：`PingFang SC`、`PingFang TC`、`Hiragino Sans`、
  `Apple SD Gothic Neo`、`Arial` 等（`patches/libreoffice/binder_font_identity.patch:200-229`）。
- `binder-fonts/` 目录**不存在**；`font-plan.json` 的候选 `binder:latin-sans` 等没有对应字体文件。
- `font_trace`/`FontPlan` 只写诊断（`libreoffice_runtime.rs:124-163`、`lo_worker.rs:179-190`），
  **不参与 LO 的实际字形选择**。

结论：

- 当前“确定性”是**在同一台 macOS、同一系统字体集上的确定性**，不是“包内字体、跨设备确定性”。
- 这与 `BR-DOC-DATA-001` 的“默认回退字体随包交付/不依赖设备字体”目标存在差距；文档 §2.3 也承认
  `binder-fonts/` 为空、S5 未完成。
- 原生替换如果沿用“系统字体 + CSS fallback”，会重犯同一问题；必须把**包内字体 + FontPlan**作为
  native 渲染器的第一等输入。

### 5.3 供应链与完整性：清单只哈希 `soffice`，构建链有缺陷

| 项 | 事实 | 影响 |
|---|---|---|
| 启动校验 | `DocToolRuntime::from_resource_dir` 只对 `soffice` 做 SHA-256；未校验 `Frameworks/*.dylib`、`Resources/**` | 嵌套 app 任一 dylib 被替换仍可通过；与“整个 app 清单”目标不一致 |
| manifest | 只有 `soffice` 的 SHA；`release_status: s4_runtime_integrated`；版本串为 `LibreOfficeDev ...` | 与 `BLOCK-REL-LO-001` “禁止发布 LibreOfficeDev/alpha 产物”存在文本冲突，需确认该版本串的构建含义 |
| 构建期 | `npm run build` → `verify:bundled-tools` → `soffice --version` | 构建/CI 需要能初始化 AppKit 的会话；无 GUI 环境可能失败 |
| 部署脚本 | `deploy_lo.sh` 也执行 `soffice --version` | 同上；且注释仍描述旧 `BINDER_FORCE_DEFAULT_FONTS` 补丁，当前代码已不使用 |
| 构建脚本 | `build_lo_macos.sh:155` 使用未定义的 `$INSTDIR`；`postprocess_lo_macos.sh:23` 有独立执行字符串 | LO 重建/瘦身链当前不可靠；不能把“有脚本”当成“可重建” |
| 体积 | LO 380 MB + Pandoc 180 MB | 包体、哈希、签名、公证、下载/安装成本 |
| 文档漂移 | README 写 ~497 MB/20 次成功；台账写 SIGABRT；当前实测 380 MB | 版本与状态记录不一致，需要以构建指纹和发布态证据统一 |

### 5.4 代码质量与测试覆盖

| 问题 | 证据 | 影响 |
|---|---|---|
| 两套重复 helper | `libreoffice_runtime.rs:172-252` 与 `lo_worker.rs:303-359` 重复 `build_soffice_command`/`profile`/`find_generated_pdf` | 修一处漏一处；替换时应整体删除 |
| 双链守卫未接线 | `convert.rs` 的 `ensure_chain_dependency`/`tool_for` 仅测试调用，生产路径直接 `tool_runtime::soffice_path()` | 规则“守卫”是装饰性的；替换时不要误以为有运行时开关 |
| PRV 预览无独立测试 | `prvRuntime.test.ts` 只测命令白名单/CSV/缓存；真实预览只在 DOC E2E 覆盖 | PRV 编排回归（缓存/asset/失败态）没有独立证据 |
| 测试 stale | `previewDoc` 已返回 asset URL，测试仍 mock `{bytes}` | 契约漂移未被测试捕获 |
| xlsx 预览路径分裂 | UI 把 xlsx 走 `prv.table_data`；`prv.preview` 仍接受 xlsx 并走 LO | API 层可绕开表格预览，行为不一致 |
| 错误提示误导 | `PrvPanel.tsx:180` 提示“请确认已安装 LibreOffice” | 系统 LO 本就不允许；与包内唯一路径策略冲突 |
| 死代码/弱 API | `resolve_soffice_path` 仅测试使用；`libreoffice_available` 仅测试使用 | 替换后可直接删除，减少 surface |

### 5.5 证据冲突的裁决

| 冲突 | 审计解释 |
|---|---|
| README “20 次成功” vs 台账 “Abort trap: 6” | 不是同一环境/时点。非沙箱普通会话可成功；受限进程/无 AppKit 会话会 abort。两者都不能替代发布态验证 |
| `A-VAL-X-V-01` “解除回归阻断” vs `DOC-LO-SANDBOX-VALIDATION-01` “ACCEPTED_RISK” | 前者解除的是“把沙箱失败当产品缺陷”的回归阻断；后者保留“干净设备/发布态未验证”的风险接受。不矛盾 |
| 文档称 S0–S4 完成 vs `BLOCK-REL-LO-001` 未解除 | S0–S4 是字体/运行时专项；S5 是发布验证。专项完成不等于发布门关闭 |
| manifest 版本 `LibreOfficeDev` vs 规则禁止 dev 产物 | 需要人类/发布工程确认该版本串是构建标识还是实际 dev 产物；在确认前不能宣称合规 |

### 5.6 可立即修正的 LO 周边问题（与替换决策解耦）

这些问题不需要等 P0 决策，属于“当前 LO 方案自身的正确性/诚实性”问题；本文只登记，不修改：

| 问题 | 位置 | 建议 |
|---|---|---|
| 预览失败提示“请确认已安装 LibreOffice” | `PrvPanel.tsx:180` | 删除；系统 LO 本就不是允许路径，提示会误导用户 |
| 预览加载文案“首次启动 LibreOffice 可能稍慢” | `PrvPanel.tsx:175` | 改为中性的“渲染预览中…” |
| `deploy_lo.sh` 注释仍描述旧 `BINDER_FORCE_DEFAULT_FONTS` 补丁 | `deploy_lo.sh:23-24` | 删除/标注历史；当前代码不再设置该变量 |
| `build_lo_macos.sh` 使用未定义 `$INSTDIR` | `build_lo_macos.sh:155` | 修正为实际 `.app` 路径；否则 `set -u` 下 postprocess 不执行 |
| `postprocess_lo_macos.sh:23` 独立执行字符串 | `postprocess_lo_macos.sh:23` | 删除；当前脚本会在瘦身前失败 |
| README 体积/成功次数与实测/台账不一致 | `resources/libreoffice/README.md` | 以构建指纹和最近一次发布态证据更新，不保留无法复现的“20 次成功”作为唯一证据 |
| `prv.preview` 仍接受 xlsx 并走 LO，UI 却走表格预览 | `prv/mod.rs:74-82` | 明确 API 语义：要么移除 xlsx 的 LO 分支，要么在文档中声明“API 直调”与 UI 分流不同 |
| `previewDoc` 测试仍 mock 旧 `bytes` 形状 | `prvRuntime.test.ts` | 按当前 `{format, abs_path}` 契约修正，补真实编排测试 |
| `ensure_chain_dependency`/`tool_for` 仅测试调用 | `doc/convert.rs:98-120` | 要么接线，要么在替换时删除；不要让“规则守卫”停留在装饰层 |
| 两份 `find_generated_pdf`/`build_soffice_command` | `libreoffice_runtime.rs:172-252`、`lo_worker.rs:303-359` | 替换时整体删除；若过渡期保留，先收敛到一处 |

---

## §6 原生替换能力缺口

### 6.1 候选资产：P1 spike 到底有什么

`.p1-spike` 引入 GenOffice `docx-engine` 与 `pptx-engine`（均为 TypeScript，非产品代码）：

- `docx-engine`：`parseDocx` 产出带 `docxIndex`/`originalXml` 锚点的 block 树；`saveDocx` 支持原样回写、
  XML 片段替换/追加、样式/图片/表格/图表/域/批注/脚注/页眉页脚/节等模型；**未包含渲染器**。
- `pptx-engine`：`openPptx` 解析 slide/layout/master/theme，继承已解析为最终样式，元素带 XML 字节锚点；
  `savePptx` 支持元素级 patch；**未包含渲染器**。
- 引擎源码注释多次引用 “our renderer / render layer”，说明完整产品另有渲染层；**该渲染层不在本仓**。
- 许可：包声明 Apache-2.0；`GENOFFICE-NOTICE` 说明第三方 notices 需在打包时生成，当前未入库；字体许可
  另见其 docs。**不能仅凭 Apache-2.0 包声明就认为全部 vendored 代码/字体已清权**。

### 6.1.1 PPTX 预览能力专项核对（2026-11-04 追加）

**结论：当前仓内的 GenOffice 集成没有可直接使用的 PPTX 预览/渲染能力。**
它提供的是 PPTX 解析 + 只读显示模型 + 保存/patch，不是像素、HTML、SVG 全页、缩略图或 PDF 输出。

E1 证据：

| 证据 | 内容 | 含义 |
|---|---|---|
| `vendor/pptx-engine/package.json` | description 为 “pptx parsing … inheritance resolution, OOXML fragment generation, element-patch save” | 包定位就是解析/生成/保存，不含渲染 |
| `vendor/pptx-engine/src/index.ts:1-6` | “Phase 1 scope: openPptx (parsing) + savePptx's 'no changes = byte identical' path” | 顶层 API 的 Phase 1 范围不含预览 |
| 公开导出 | `openPptx`、`savePptx`、`savePptxToFile`、`reparseDeck`、元素/表格/图表/动画/主题等 patch 函数 | 没有 `renderSlide`/`preview`/`toSvg`/`toHtml`/`thumbnail` |
| `vendor/pptx-engine/src` 的 DOM 使用 | 未发现 `document.`、`canvas`、`getContext`、`ImageData`、`OffscreenCanvas` | 引擎本身不绘制像素 |
| `vendor/pptx-engine/tests` | 以 parse/patch/save/fidelity 为主；没有 screenshot/visual render 断言 | 测试也不证明“能显示” |
| `.p1-spike/src/main.ts`、`src/worker.ts` | 只 import `docx-engine` 的 `parseDocx`/`saveDocx` | P1 spike 的 UI/Worker 没有 PPTX 预览入口 |
| `vendor/pptx-engine/src/types.ts:5-11`、`parse.ts` 多处 | 注释反复引用 “rendering / render layer”“the render layer uses them directly” | 设计上存在一个外部渲染层，但**不在本仓** |

它已经具备的“预览底座”（不是预览本身）：

- `openPptx` 解析 slide/layout/master/theme，继承链已解析为最终样式，元素带 XML 字节锚点。
- `parse.ts` 为表格、图表、SmartArt 预渲染、OLE 预览、VML 预览等生成**只读显示模型**
  （`previewShapes`、`previewPicture`、`vmlPreviews`）；这些是给渲染器消费的数据，不是画面。
- `custgeom.ts` 输出归一化 SVG path（0..1），`animation.ts` 支持 motion path 的 SVG 子集；
  这些是可复用的绘制原语，但还没有“把一张 slide 组合成 SVG/HTML”的实现。
- 主题色、填充/描边/效果、几何、组变换、占位符继承、文本属性等模型都在，理论上可支撑渲染器。

仍缺的 PPTX 预览能力：

1. **画布合成**：slide 尺寸/缩放、z-order、组嵌套、旋转/翻转、裁剪、背景、母版/版式装饰层。
2. **文本排版**：字体/字号/字重、autofit（fontScale/lnSpcReduction）、行距/段距、项目符号/编号、
   制表位、竖排、CJK 断行/禁则、占位符继承后的最终样式。
3. **表格/图表/SmartArt**：表格行高/合并/边框，图表坐标轴/系列/图例/标签/组合图/伪 3D，
   SmartArt 预渲染形状或 fallback 布局的实际绘制。
4. **媒体**：PNG/JPEG/GIF/SVG/EMF/WMF/TIFF 解码、图片裁剪/透明度/效果、音视频占位。
5. **效果/3D/WordArt**：阴影、发光、倒影、柔化、渐变、图案、3D 场景、艺术字。
6. **输出**：缩略图、逐页图片、可滚动 HTML/SVG、PDF、打印。
7. **工程**：大 deck 性能/内存、取消/超时、不可信 OOXML 的安全沙箱、字体注册与缺字报告。

**关于“上游 GenOffice 是否有 renderer”**：引擎注释表明完整产品另有渲染层，但该层不在本仓；
`GENOFFICE-NOTICE` 还引用了上游 `apps/docs/src/renderer/fonts/README.md` 与 `apps/pdf/src/shared/radicals.ts`，
说明完整 GenOffice 项目很可能包含 renderer/PDF 模块，但当前 checkout 没有这些源码/包。
本审计无法确认它们是否可授权、可离线打包、是否覆盖上述 PPTX 能力。若要复用，必须单独做来源/许可/集成验证，
不能因为引擎里有 `previewShapes` 或 NOTICE 提到 renderer 就认为“已经有 PPTX 预览”。

### 6.2 本次只读 spike 证据（E1）

```text
# 1) 最小 DOCX
$ cd .p1-spike && npm run smoke -- fixtures/sample.docx
parse: 27.2 ms, blocks=12, visible=11
unchanged save: 0.9 ms, bytes=3426, byteIdentical=true

# 2) 项目 reference.docx（Pandoc 参考样式，含更多结构）
$ npx tsx scripts/smoke.ts ../src-tauri/resources/reference.docx
parse: 50.4 ms, blocks=56, visible=55
unchanged save: 0.9 ms, bytes=13968, byteIdentical=true

# 3) PPTX 解析（引擎自带 fixture）
01_standard_business.pptx: bytes=32396, slides=5, elements=11, ms=77.5
05_unicode_cjk_emoji.pptx: bytes=28318, slides=1, elements=2, ms=12.5
```

**能证明**：在本机 Node 环境，DOCX/PPTX 解析与“未修改保存字节一致”可行；小样本性能可接受。
**不能证明**：浏览器/WebView 环境、真实复杂文档、渲染/分页/字体/PDF、内存/并发/安全、产品集成。

### 6.3 能力缺口矩阵

状态口径：`已有基础`=仓内有可运行代码/模型；`有模型无渲染`=解析模型存在但无显示实现；
`无实现`=仓内没有；`未验证`=有路径但无证据。

| 能力 | DOCX | PPTX | XLSX/XLS | 说明 |
|---|---|---|---|---|
| 二进制解析 | 已有基础（spike） | 已有基础（spike） | 已有（calamine，仅值） | DOCX/PPTX 引擎覆盖较广；XLSX 无样式模型 |
| 块/元素模型 | 已有基础 | 已有基础 | 无（仅二维值） | DOCX 有段落/表格/图片/域/批注/脚注/页眉页脚；PPTX 有继承解析 |
| 分页/页面几何 | 有模型无渲染 | 不适用（固定画布） | 不适用 | DOCX 有 section/docGrid/frame/dropCap 模型，但无排版器 |
| 文本 shaping/换行 | 无实现 | 无实现 | 无实现 | 依赖 WebKit 或自建 HarfBuzz/ICU；CJK 禁则、网格、两端对齐均需处理 |
| HTML/SVG 渲染 | 无实现 | 无实现 | 无实现 | 需把模型投影为 DOM/SVG，并处理浮动/层叠/裁剪 |
| PDF 生成 | 无实现 | 无实现 | 无实现 | 可选 WKWebView `createPDF`/`NSPrintOperation`，仓内未接 |
| 打印 | 无实现（可借 WKWebView，未接） | 同左 | 表格可 `window.print` | 需保证只读、分页、页眉页脚 |
| 字体解析 | 有诊断（FontPlan） | 有诊断 | 无 | 需包内字体注册、family/主题/脚本映射、缺字报告 |
| PDF 文本层/嵌字体 | 未验证 | 未验证 | 未验证 | 需可选中/可搜索/字体嵌入断言 |
| legacy `.doc/.ppt` | 无实现 | 无实现 | `.xls` 有值解析 | 需明确不支持或另找转换器 |
| 安全沙箱 | 无实现 | 无实现 | 无实现 | 不可信 OOXML/HTML、zip bomb、XXE、远程资源、脚本、OLE/宏 |
| 缓存版本化 | 部分（现有 PDF 缓存） | 同左 | 同左 | 键需加 renderer/font 版本；否则升级后旧缓存 |
| 语料/黄金样本 | 无 | 无 | 无 | 只有 1 个 3.4KB sample + reference.docx + 2 个 pptx fixture |
| 性能/内存 | 未验证 | 未验证 | 未验证 | 大文档、100+ 页、50MB、并发预览 |
| 测试治理 | 无 native E2E | 无 | 无 | 需替换 LO E2E 并更新测试注册表 |

### 6.4 结论：缺口在“渲染”，不在“解析”

- DOCX/PPTX 解析层已有可用的开源/半成品基础，但**没有渲染器**；DOCX 还需要分页排版，PPTX 还需要
  画布/主题/图表/SmartArt/动画的显示实现。
- XLSX 只有“值投影”，没有样式/列宽/合并/图表渲染；当前产品也只承诺表格式预览，但 PDF 导出依赖 LO。
- PDF 文本层、打印、字体 shaping、安全沙箱、缓存版本化、语料，全部是新增工程，不在 spike 内。
- 因此“用 spike 替换 LO”当前不成立；正确表述是“spike 为 native 替换提供了 DOCX/PPTX 解析底座”。

### 6.5 现有非 LO 能力能走多远（不是目标架构，只用于风险判断）

| 目标 | 现有非 LO 路径 | 可达到 | 不足 |
|---|---|---|---|
| DOCX 只读预览 | `doc.open_for_edit` 的 DOCX→HTML（Pandoc）→只读渲染 | 文本/基础样式可读 | 这是编辑模型，不是分页版面；浮动/页眉页脚/表格布局/分页会丢失；且会与编辑链耦合 |
| DOCX→PDF | 上述 HTML → WKWebView 打印 | 可生成 PDF | 版式与 Word/LO 差异大；不能作为“只读保真预览”的替代 |
| XLSX→PDF | `prv.table_data` 的值矩阵 → HTML 表格打印 | 值/行列可打印 | 无列宽、颜色、合并、公式格式；不是当前 LO PDF 的等价物 |
| PPTX 预览 | 无 | 无 | 必须新建渲染器或引入 SDK |
| legacy `.doc/.ppt` | 无 | 无 | 必须明确不支持或另找转换器 |

结论：现有非 LO 路径可以支撑“临时降级/应急开关”，但**不能作为目标态**；它会把编辑模型与预览模型混在一起，
违反双链分离的初衷。若采用，只能作为 feature flag 下的短期降级，并明确告知用户差异。

---

## §7 方案选项与取舍

### 7.1 选项

| 方案 | 做法 | 优点 | 缺点/风险 | 适用判断 |
|---|---|---|---|---|
| **A. WebView HTML/SVG 渲染 + WKWebView PDF/打印** | 解析→显示模型→沙箱 iframe 渲染；导出/打印用 WKWebView `createPDF`/`NSPrintOperation` 或 `window.print` | 复用 WebKit 文本 shaping/CSS；文本可选中；开发量相对可控；无 380MB LO | 分页/浮动/页眉页脚保真难；PDF 文本层/嵌字体需验证；XSS/远程资源风险；PPTX 仍很难 | **DOCX 首选**；PPTX 需另评估 |
| **B. Rust 原生排版/PDF 引擎** | 自建 layout + shaping + PDF | 确定性、可沙箱、无 WebView 依赖 | 工作量最大；字体/断行/表格/浮动/PPTX 都是深水区；PDF 文本层也要自建 | 仅在 A 的 PDF/确定性不达标时考虑 |
| **C. 商用 SDK** | 采购支持 DOCX/PPTX/XLSX 渲染/PDF 的 SDK | 覆盖与保真通常最好；上市快 | 许可/费用/离线/隐私/供应商锁定；需法务与安全评估 | 若 PPTX/legacy 必须短期覆盖，优先评估 |
| **D. 混合过渡** | DOCX 用 native；PPTX/legacy 暂留 LO | 风险最低；可分阶段验收 | 未达到“彻底替换”；LO 的签名/字体/包体债务继续存在 | 只作为过渡，不作为目标态 |
| **E. 收窄产品范围** | 不做 PPTX/legacy 预览；PDF 导出改为打印/HTML | 最快去除 LO | 产品能力回退；需人类明确接受；与 REQ-PRV-002 冲突 | 仅在人类接受范围变更时 |

### 7.2 推荐

**推荐 A（DOCX 先行）+ 分阶段决策**：

1. 先用 A 打通 DOCX 原生预览/打印/PDF，替换 docx/doc 的 LO 路径。
2. PPTX 单独立项：优先评估 C（商用 SDK）或 A 的 PPTX 渲染器；在决策前保留 D 作为过渡，
   但**不把 D 写成目标态**。
3. XLSX：表格预览已无 LO；若必须保留“样式化 PDF 导出”，单独评估 A 的表格渲染或 C；
   否则由人类决定是否收窄为“表格值预览 + 打印”。
4. legacy `.doc/.ppt`：优先明确“不支持，明确报错”；不建议为 legacy 自建解析器。
5. B 仅作为 A 在 PDF 文本层/确定性/性能上不达标时的备选，不作为第一实施路径。

### 7.3 方案选择前必须回答的门（D1–D9）

| 编号 | 决策 | 为什么必须先答 |
|---|---|---|
| D1 | 支持矩阵：只 docx/pptx/xlsx？是否保留 `.doc/.ppt/.xls`？ | 决定解析器/渲染器范围与工作量 |
| D2 | 渲染器来源：自建 / GenOffice renderer 授权 / 商用 SDK / Rust | 决定关键路径与许可 |
| D3 | 引擎放置：DOC 后端 / DOC 拥有的隐藏 WebView / 前端 worker | 决定是否修改 `X-SEQ-004`/`BR-ED-DATA-002` |
| D4 | PDF 是否仍是所有格式的导出目标？是否接受 HTML/打印替代？ | 决定 PDF 生成与文本层投入 |
| D5 | 字体：哪套可再分发字体、覆盖哪些语言/符号、预算与体积 | 决定确定性与许可 |
| D6 | 与 Pandoc 的关系：继续双链，还是未来统一到 native 引擎？ | 避免同时维护两套 DOCX 模型 |
| D7 | 保真基准：Word / LO / “Binder 统一预览”？允许哪些差异？ | 决定验收语料与 golden |
| D8 | 迁移策略：DOCX 先行 + LO 过渡，还是一次性全量？ | 决定发布风险与回滚 |
| D9 | 平台：仅 macOS arm64，还是未来 Windows/Intel？ | 决定渲染器/字体/PDF 的可移植性 |

---

## §8 目标架构（建议，不是已批准设计）

### 8.1 推荐形态

```
┌─────────────────────────────────────────────────────────────┐
│ PRV（编排/UI/只读边界/asset 授权/打印入口）                    │
│  prv.preview(relPath) -> { format, path|html, rendererVersion }│
└───────────────────────────┬─────────────────────────────────┘
                            │ 仅调用 DOC 的预览能力
┌───────────────────────────▼─────────────────────────────────┐
│ DOC preview runtime（唯一二进制解析/渲染 owner）               │
│  1. format probe（docx/pptx/xlsx/pdf/legacy）                 │
│  2. parse（native engine；DOC-owned）                          │
│  3. display model（HTML/SVG 或分页盒模型）                      │
│  4. font resolver（包内字体 + FontPlan + 缺字报告）             │
│  5. render surface（沙箱 iframe / DOC-owned hidden WebView）    │
│  6. print/PDF adapter（WKWebView createPDF / NSPrintOperation） │
│  7. cache（source hash + renderer version + font plan version） │
└─────────────────────────────────────────────────────────────┘
```

### 8.2 边界选择

| 方案 | 描述 | 与现有规则的关系 | 建议 |
|---|---|---|---|
| **B1** | 引擎在 DOC 后端（Rust/WASM/嵌入式 JS runtime），渲染结果交给 PRV | `X-SEQ-004` 不变；“DOC 唯一通道”成立 | **优先**，但工程量大 |
| **B2** | DOC 拥有的隐藏 WebView/worker 执行解析+渲染，PRV 只见结果 | 仍由 DOC 拥有；需在 `X-SEQ-004`/`BR-ED-DATA-002` 明确“DOC-owned render worker” | **次优**，需治理变更 |
| **B3** | 前端 `src/` worker 解析 DOCX，PRV 直接消费 | 违反 `X-SEQ-004`/`BR-ED-DATA-002`（二进制解析在前端/ED 侧） | **不推荐**；除非人类明确改规则并接受安全边界 |

> 若选 B2/B3，必须把“二进制解析归属”写成显式规则变更，而不是在代码里绕开。ADU/ADP 不允许
> “为通过实现反向改设计”，但允许在人类裁决下调整架构规则；这属于 `GOVERNANCE_CHANGE`。

### 8.3 数据/错误/缓存契约

- `prv.preview` 保持命令名与只读语义；返回增加 `format`（`pdf`/`html`）、`renderer_version`、
  `cache_key`；迁移期可同时支持 PDF/HTML。
- 错误码保留统一形状：`PRV_UNSUPPORTED`、`PRV_IO_ERROR`、`PRV_SCOPE_FAILED`；DOC 侧用
  `DOC_RENDER_*`（或保留 `DOC_PREVIEW_*` 的兼容映射），不得把内部路径/正文放进 message。
- 缓存键：`hash(abs_path) + size + mtime + renderer_version + font_plan_version + format`；
  源内容 hash 优先于 mtime（避免 mtime 精度/回拨问题）。
- 预览生命周期：请求级临时目录 → 原子写入缓存 → 过期清理；失败不留下半成品。
- 资源授权：若 HTML 预览引用抽取媒体，必须逐文件授权（复用 `doc.image_media` 的模式），
  不得开放整个目录；PDF 由渲染器内嵌，不需要逐图授权。
- PDF 生成 API 的可用性需在 P0 核实：`WKWebView.createPDF` 是 macOS 11+ API；若项目最低版本低于 11，
  必须使用 `NSPrintOperation` 或 Rust PDF 兜底，不能把 `createPDF` 当成无条件可用。

### 8.4 安全模型（必须前置设计）

原生 HTML 预览把“不可信 OOXML”变成“不可信 HTML”，风险高于 PDF：

- iframe 必须 `sandbox`（无 `allow-scripts`、无 `allow-same-origin`），禁止脚本/表单/弹窗。
- 禁止远程资源（图片/字体/超链接预取）；外部关系只显示为文本/不可点击，或经用户显式确认。
- 清理事件属性、`javascript:` URL、`data:` 非图片、SVG 脚本、CSS `url()` 外链。
- ZIP/XML 限制：条目数、解压总大小、单条大小、嵌套深度、实体扩展（XXE）、路径穿越。
- 图片解码限制：像素/字节上限、解码炸弹、EMF/WMF/TIFF 失败降级。
- 宏/OLE/嵌入对象：只显示占位，不执行、不加载。
- 渲染进程隔离：超时、内存上限、崩溃恢复；不得拖垮主 WebView。

---

## §9 迁移路线（建议）

> 以下阶段是“准备与实施”的建议，不是已批准的 Scope；每阶段都需按 ADP 立项、Issue Trace、
> Scope Lock 与验证。

### P0 · 决策与基线（1–2 周）

**目标**：把 §7.3 的 D1–D9 变成人类确认的决策；建立语料与验收基准；不做产品代码替换。

交付：

1. 支持矩阵（格式 × 能力 × 是否保留 legacy）。
2. 渲染器来源决策（A/B/C）与引擎放置决策（B1/B2/B3）。
3. PDF/打印策略（WKWebView vs Rust；PDF 是否 mandatory）。
4. 字体授权/覆盖/体积方案。
5. 语料清单与 golden 方案（§11）。
6. 原生替换 ADR（架构决策记录）+ 风险登记。
7. 治理变更清单（§10）与实施 Scope 提案。

退出条件：D1–D9 有明确答案；语料与验收标准可执行；人类确认进入 P1。

### P1 · DOCX 原生预览 MVP

**目标**：docx/doc 预览不再调用 LO；保留 LO 仅作为过渡开关（feature flag）。

范围建议：

- DOCX parse → display model → 沙箱 HTML/SVG；支持段落/标题/列表/基础表格/图片/页眉页脚/分页基础。
- 字体：包内 Latin/CJK 基础集 + FontPlan；缺字报告。
- 缓存：版本化键；失败态；超时/取消。
- 不改编辑链、不改 PDF 导出（先只替换预览）。

验收：

- 语料中文档文本顺序/分页/图片/表格与基准一致或在声明差异内。
- 不触发 LO；无系统字体依赖；离线可用；无网络请求。
- 20 份 DOCX 语料通过；无 XSS/脚本执行；首开/缓存性能达标（或记录未达标）。
- 编辑链回归全通过；`prv.preview` 契约稳定；回滚开关可用。

### P2 · DOCX PDF/打印替换

**目标**：docx→PDF 导出与打印不再调用 LO；`prv.export` 的 docx→pdf 走 native。

验收：

- PDF 页数/文本层/可选中/可搜索/嵌字体；与 P1 渲染一致。
- 打印走系统打印服务；只读；不产生脏态。
- 原有 `prv.export` 行为/落点/错误语义保持；不改变“先保存再导出”的 UI 语义。
- 移除 DOCX 的 LO 路径与相关错误码/测试。

### P3 · PPTX 决策与替换

**目标**：按 D2/D5 决策，替换 pptx/ppt 预览与 PDF；或人类明确收窄范围。

选项：

- P3-A：native PPTX 渲染器（画布/主题/文本/图片/表格/图表/SmartArt；动画可静态首帧）。
- P3-B：商用 SDK（若必须短期覆盖）。
- P3-C：产品收窄（不做 PPTX 预览/PDF），需改 REQ-PRV-002。

验收：与 P1/P2 同口径的语料/PDF/打印/安全/性能；无 LO。

### P4 · XLSX/legacy 收尾

- XLSX：表格预览已无 LO；若保留样式化 PDF，补原生表格渲染/打印；否则明确“值预览 + 打印”。
- legacy `.doc/.ppt`：明确“不支持，明确报错”，或另找非 LO 转换器（需单独立项）。
- 移除 LO 资源、manifest、脚本、patches、gitignore、构建校验、测试约束、发布门。

### P5 · 治理收尾与发布验证

- 更新 4 个 REQ、1 条 RULE、TERM、L2/L3、开发计划、债务台账、验证卡、ADUS、产品能力注册表。
- 关闭/替代 `BLOCK-REL-LO-001`；建立 native 渲染的发布门。
- 干净设备（无 LO、无额外字体）验收：预览/导出/打印/离线/字体/PDF 文本层。
- 发布签名/公证不再包含嵌套 LO；包体/启动/CI 证据更新。

---

## §10 治理与文档修改清单（提案）

> 以下修改在替换实施时执行；本文不修改任何现有文档。`GOVERNANCE_CHANGE` 与 `CODE_CHANGE` 应分开立项，
> 避免“代码先改、规则后补”的反向治理。

### 10.1 需求层

| 对象 | 当前 | 建议变更 | 类型 |
|---|---|---|---|
| REQ-DOC-003 acceptance | “预览链可按 LO 语料实测结果只读查看” | 改为“按 native 渲染语料/基准只读查看”，并声明允许的差异 | REQ |
| REQ-DOC-004 | “提供原生 docx→PDF 的本地预览转换（LibreOffice 预览链）” | 改为“native 预览转换（PDF 或声明的 HTML 预览形态）”；保留只读/双链分离/失败态 | REQ |
| REQ-DOC-005 | “Pandoc/LibreOffice 与默认回退字体必须随包内置” | 改为“Pandoc + native preview runtime + 包内字体随包内置”；保留无系统回退/离线 | REQ |
| REQ-PRV-001 | “消费 DOC 预览链（LibreOffice→PDF）”“LO 语料实测” | 改为“消费 DOC native 预览能力”；更新保真基准 | REQ |
| REQ-PRV-002 | Excel/PPT 预览 | 增加实现说明：xlsx 表格预览（calamine）与 PPTX native 渲染；PDF 导出 native | REQ（说明性） |
| REQ-PRV-004 | 打印与导出 | 明确 PDF/打印由 native 生成；导出格式清单按 D4 更新 | REQ（说明性） |
| REQ-CORE-004 | 产品级预览/导出 | 不需要改文本（未点名 LO）；但验收证据需替换 | 无文本变更 |

### 10.2 规则/术语/约束层

| 对象 | 当前 | 建议变更 | 类型 |
|---|---|---|---|
| `BR-DOC-DATA-001` | “Pandoc、LibreOffice 与默认回退字体必须随包交付” | 改成“Pandoc + native preview runtime + 包内字体”；保留“唯一通道/双链分离/不静默降级/无系统工具回退” | RULE |
| `X-SEQ-004` | 二进制转换必经 DOC | 若引擎在 DOC 后端则**不需要改**；若 DOC-owned 隐藏 WebView/worker，需明确其属于 DOC 通道；前端 worker 方案需改规则 | CONSTRAINT |
| `BR-ED-DATA-002` | ED 不含 docx 二进制路径 | 同上；native 引擎不得落到 ED/前端普通模块 | RULE |
| `TERM-DOC-004` FontIdentityFix | LO 专属术语（当前无 `term_ref` 引用） | 退役，或重定义为 native font identity；同步 ADUS | TERM |
| `TERM-DOC-001/002/003/005` | 样式子集/EditDraft/BDocx/TableCellShading | 保留；与 LO 无直接绑定（BDocx 编辑链仍是 Pandoc） | 无 |

### 10.3 设计与计划层

| 文档 | 需要更新的章节 |
|---|---|
| `A-DOC-M-T-01` | §1 技术方案、§2 双链图、§2.3 LO 专项（改为 native 字体/渲染专项或标记历史）、§3 BR-DOC-DATA-001、§4 样式表、§8 测试、§9 债务 |
| `A-DOC-I-P-01` | §3 预览转换能力、§4 对接、§6 债务 |
| `A-DOC-M-S-01` | 预览链无状态说明（若 native 引入渲染队列/缓存编排，需复核状态机豁免） |
| `A-DOC-X-L-01` | PI-DOC-03 S0–S5 状态、构建配方、验收；新增 native 替换计划 |
| `A-DOC-X-R-02` | 标记为历史决策；native 字体策略另立文档 |
| `A-PRV-M-T-01` | §2 管线归属表（docx/PPT 行）、§5 缓存/状态机豁免、§9 债务 |
| `A-PRV-M-D-01` | P-J1、REQ-PRV-001/002/004 的验收表述 |
| `A-PRV-I-P-01` | §2 `prv.preview` 返回形态/错误、§4 债务 |
| `A-ED-M-T-01:787` | “PRV 预览（LibreOffice→PDF 全保真）补偿查看”改为 native 预览补偿 |
| `A-SYS-C-T-02:110,220-233` | 预览面板/流程图的 LO 字样 |
| `A-CORE-X-R-01` | `BLOCK-REL-LO-001` 关闭/替代；`DOC-PLAT-001` 的架构/产物矩阵更新 |
| `docs/40_validation/A-VAL-X-V-02` / `V-04` | 保留历史 LO 证据；新增 native 渲染验证卡；更新执行约束 |
| `.governance/test-governance.json` | `execution_constraints` 移除 LO/AppKit；模块源码/测试映射更新 |
| `A-CORE-C-R-01_ADUS` | `governance:generate` 重新生成；@GOV/测试覆盖指向 native runtime |
| `.governance/product-capability-registry.json` | CAP-PREVIEW-OUTPUT 的 evidence/acceptance/release 状态 |
| `A-CORE-X-L-01/02/03/05` | 路线图/计划/进度中的 LO 字样与阶段 |
| `resources/libreoffice/README.md`、`deploy_lo.sh` 注释、`build_lo_macos.sh` | 删除或标记历史；修正 stale 说明 |

### 10.4 代码/构建层

| 对象 | 建议 |
|---|---|
| `doc/libreoffice_runtime.rs`、`doc/lo_worker.rs` | 删除，替换为 `doc/preview_runtime.rs`（或分拆 parse/render/pdf） |
| `doc/tool_runtime.rs` | 移除 `libreoffice` spec；保留 Pandoc；新增 native 资源/字体清单（如需要） |
| `doc/convert.rs` | 移除 `ConvertTool::LibreOffice`/`ConvertChain` 的 LO 断言；若预览仍需“依赖守卫”，改为 native probe；当前这两个函数未接线，替换时应一并清理 |
| `doc/font_trace.rs` | 保留并扩展为 native font resolver 的输入；不要随 LO 删除 |
| `prv/mod.rs`、`prv/routes.rs` | 替换 `render_preview`/`export` 的 office→pdf 分支；更新缓存键/错误映射 |
| `lib.rs` | 移除 LO 初始化；注册 native preview 命令/状态 |
| 前端 `PrvPanel`/`PrvToolbar`/`prvService` | 更新文案/返回类型/失败态；删除“安装 LibreOffice”提示 |
| `resources/libreoffice/**`、`patches/libreoffice/**` | 删除；`font-plan.json` 若仍需保留，移到 native 字体/渲染资源目录并更新 manifest；历史可留档到 archive（不入包） |
| `package.json`/`verify_bundled_tools.sh`/`tauri.conf.json`/`.gitignore` | 移除 LO 校验/打包/忽略；native 资源按新清单管理 |

---

## §11 验收与证据计划

### 11.1 语料矩阵（建议最低集）

| 类别 | 样本 | 覆盖 |
|---|---|---|
| DOCX 基础 | 纯文本、标题、列表、粗斜下、颜色、超链接 | 文本顺序/样式/链接安全 |
| DOCX 结构 | 多级标题、分节、分栏、页眉页脚、页码、脚注/尾注 | 分页/节/页眉页脚 |
| DOCX 表格 | 合并单元格、行高、列宽、边框、底纹、跨页表 | 表格布局/分页 |
| DOCX 图片 | 内联/浮动、EMF/WMF/TIFF、锚点、裁剪、旋转 | 图片解码/定位/层叠 |
| DOCX 高级 | 文本框、SmartArt、图表、OMML 公式、域/TOC、批注/修订 | 降级/占位/不丢失文本 |
| DOCX 语言 | 中/日/韩/拉丁/emoji/混排/RTL | 字体/shaping/断行 |
| DOCX 规模 | 1 页、20 页、100+ 页、50MB、含大量图片 | 性能/内存/超时 |
| DOCX 恶意 | zip bomb、XXE、路径穿越、脚本/远程图片、畸形 XML | 安全 |
| PPTX | 标准商务、母版/主题、文本自适应、表格、图表、图片、SmartArt、批注、动画 | 画布/主题/降级 |
| PPTX 语言/规模 | CJK/emoji、大 deck | 字体/性能 |
| XLSX | 多 sheet、值/公式缓存、合并、列宽、样式、图表、超大表 | 值预览/样式 PDF（若保留） |
| legacy | `.doc/.ppt/.xls` | 明确支持/明确拒绝 |
| PDF | 文本层、嵌字体、页数、可选中/搜索 | PDF 断言 |

### 11.2 断言

1. **文本**：全文抽取与源文档文本顺序一致；不丢字、不重复、不把 alt 当正文。
2. **布局**：页面尺寸/边距/分页点/行数/表格单元格位置/图片尺寸在声明容差内。
3. **视觉**：与 golden 截图逐页对比（像素容差 + 人工复核）；差异必须记录，不得静默。
4. **PDF**：`%PDF-` 魔数、页数、文本层可抽取、字体嵌入、可搜索；与预览一致。
5. **字体**：包内字体命中；无系统字体依赖；缺字有结构化报告；同输入多次渲染一致。
6. **性能**：首开/缓存命中/内存/并发；沿用 DOC-05 目标（首开 <12s、缓存 <2s）或记录新基线。
7. **安全**：无脚本执行、无网络请求、无路径越界、zip/XML/图片限制生效、崩溃可恢复。
8. **只读**：预览不产生脏态、不修改源文件、不写 workspace。
9. **离线**：断网/无系统 LO/无额外字体环境通过。
10. **回归**：编辑链、草稿、导出、打印、Diff/AG 读取全部通过；测试数不减少。

### 11.3 证据分离

- `automated`：Rust/前端单测、native 渲染 golden、PDF 断言、安全用例。
- `system`：真实 Tauri/WKWebView 预览、打印、干净设备。
- `manual`：复杂文档视觉复核、差异接受。
- 不得用“非沙箱通过”替代干净设备；不得用“spike smoke”替代产品 E2E。

---

## §12 风险与未决问题

| 风险 | 影响 | 缓解 |
|---|---|---|
| 渲染器来源未定（自建/授权/商用） | 关键路径无法启动 | P0 先做 D2；用 spike 做技术验证而非产品承诺 |
| DOCX 分页/浮动/表格保真不足 | 用户看到与 Word 不同 | 定义“Binder 统一预览”基准与差异告知；先覆盖高频结构 |
| PPTX 渲染工作量被低估 | 替换周期失控 | 单独立项；优先评估商用 SDK；允许阶段性收窄 |
| PDF 文本层/字体嵌入不达标 | 导出不可搜索/复制 | 早期做 WKWebView `createPDF` 验证；不达标再评估 Rust PDF |
| 字体授权/体积 | 无法打包或包体过大 | P0 定字体清单；按语言子集/按需加载；记录许可 |
| X-SEQ-004 边界冲突 | 代码通过但治理 audit 失败 | 先做 GOVERNANCE_CHANGE；引擎放 DOC 或 DOC-owned worker |
| HTML 预览 XSS/远程资源 | 安全漏洞 | 沙箱 iframe + 清理 + CSP + 网络禁用 + 安全语料 |
| 缓存未版本化 | 升级后旧预览 | 键加 renderer/font 版本；清缓存策略 |
| 测试环境仍依赖 AppKit | CI 不稳定 | native 渲染测试尽量离屏/纯模型；真实预览放系统验证 |
| 与 Pandoc 双链继续分叉 | 预览/编辑保真不一致 | 明确 D6；短期接受，长期评估统一引擎 |
| 现有未提交改动冲突 | 替换工作覆盖他人修改 | 立项前确认工作树；分批小 Scope；不使用 `reset`/`checkout` |
| 仅 macOS arm64 | 未来平台不可移植 | D9 明确；渲染器/字体/PDF 选型考虑跨平台 |

---

## §13 结论

1. **LO 今天仍然是 3 类用户能力的实现基础**：原生 DOCX/DOC 预览、PPTX/PPT 预览、Office→PDF 导出/打印；
   同时承担字体/PDF/legacy/供应链/发布门职责。
2. **LO 不是编辑链的必要条件**；编辑、表格预览、原生媒体预览、AG 读取、草稿复制均可独立于 LO。
3. **4 个 REQ 文本显式绑定 LO**；彻底替换必须同步改需求/规则/术语/ADUS/测试，不是纯代码工作。
4. **原生替换的关键路径是渲染器与 PDF/字体工程**，不是 DOCX 解析；P1 spike 只证明解析/保存，
   不能作为“可替换”证据。
5. **LO 当前仍有发布阻断与字体/供应链债务**；原生替换是消除这些债务的合理方向，但必须先做 P0 决策，
   再按 DOCX→PDF→PPTX→XLSX/legacy 分阶段实施，保留回滚开关。
6. **建议下一步不是写代码，而是完成 P0**：支持矩阵、渲染器来源、引擎放置、PDF/打印策略、字体授权、
   语料与验收基准、治理变更清单；经人类确认后再进入 P1。

---

## 附录 A：LO 引用清单（关键文件）

> 仅列审计时点的关键引用；完整 grep 可按 `LibreOffice|soffice|LO ` 复现。

### A.1 代码

```text
src-tauri/src/doc/libreoffice_runtime.rs        362 行（预览链运行时）
src-tauri/src/doc/lo_worker.rs                  484 行（LO 子进程 Worker）
src-tauri/src/doc/tool_runtime.rs               249 行（manifest/LO 校验）
src-tauri/src/doc/convert.rs                    180 行（双链/ConvertTool）
src-tauri/src/doc/font_trace.rs                 613 行（字体取证，可复用）
src-tauri/src/prv/mod.rs                        248 行（预览/导出编排）
src-tauri/src/prv/routes.rs                     115 行（prv.preview 路由）
src-tauri/src/lib.rs                            216/357/428（注册/分发/初始化）
src/components/prv/PrvPanel.tsx                 14/34/140/175/180
src/components/prv/PrvToolbar.tsx               12-13/17/61-65
src/services/prvService.ts                      4/26-38
src/services/docService.ts                      5-7/344-415
```

### A.2 资源/构建

```text
src-tauri/resources/libreoffice/LibreOffice.app  380 MB
src-tauri/resources/libreoffice/font-plan.json   20 条策略（候选 binder:* 无字体文件）
src-tauri/resources/tools/manifest.json          13-19（LO 版本/路径/SHA）
scripts/verify_bundled_tools.sh                  14-31（构建期 --version）
scripts/deploy_lo.sh                             40-81（部署/--version）
scripts/build_lo_macos.sh                        1-160（源码构建）
scripts/postprocess_lo_macos.sh                  23（独立执行字符串缺陷）
patches/libreoffice/*.patch                       2 个补丁
```

### A.3 文档

```text
docs/01_modules/DOC/A-DOC-M-D-01_文档转换功能需求描述.md
docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md
docs/01_modules/DOC/A-DOC-I-P-01_文档转换接口协议.md
docs/01_modules/DOC/A-DOC-M-S-01_文档转换状态机设计.md
docs/01_modules/DOC/A-DOC-X-L-01_文档转换开发计划.md
docs/01_modules/DOC/A-DOC-X-R-02_字体身份修复决策记录.md
docs/01_modules/PRV/A-PRV-M-D-01_预览输出功能需求描述.md
docs/01_modules/PRV/A-PRV-M-T-01_预览输出技术设计.md
docs/01_modules/PRV/A-PRV-I-P-01_预览输出接口协议.md
docs/01_modules/PRV/A-PRV-X-L-01_预览输出开发计划.md
docs/00_core/A-CORE-X-R-01_门禁闭合债务分类台账.md
docs/00_core/A-CORE-C-R-01_ADUS.md
docs/40_validation/A-VAL-X-V-02_核心编辑与DiffEdit保存验证.md
docs/40_validation/A-VAL-X-V-04_Agent1.0验证卡.md
docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md:787
docs/01_modules/SYS/A-SYS-C-T-02_前端交互架构与流程总图.md:110,220-233
```

---

## 附录 B：本次证据命令与限制

### B.1 已执行

```bash
# 治理基线（只读）
npm run governance:audit
# -> status: passed, findings: [], warnings: []

# 资源体积
du -sh src-tauri/resources/libreoffice/LibreOffice.app
# -> 380M
du -sh src-tauri/resources/tools
# -> 180M

# 字体补丁是否编译进二进制（局部符号）
nm -a src-tauri/resources/libreoffice/LibreOffice.app/Contents/Frameworks/libvclplug_osxlo.dylib \
  | grep AddEnglishAlias
# -> SystemFontList::AddEnglishAlias(CoreTextFontFace*) 存在

# P1 spike：DOCX 解析/保存
cd .p1-spike && npm run smoke -- fixtures/sample.docx
# -> parse 27.2ms, blocks=12, byteIdentical=true
npx tsx scripts/smoke.ts ../src-tauri/resources/reference.docx
# -> parse 50.4ms, blocks=56, byteIdentical=true

# P1 spike：PPTX 解析
npx tsx -e "... openPptx ..."
# -> 01_standard_business: 5 slides / 11 elements / 77.5ms
# -> 05_unicode_cjk_emoji: 1 slide / 2 elements / 12.5ms
```

### B.2 未执行/未验证

- **未启动** `soffice` GUI，未执行真实 DOCX/PPTX→PDF 转换；避免在审计中触发 AppKit/GUI/Crash Reporter。
- **未运行** 打包后的 Tauri app；未验证签名/公证/干净设备。
- **未验证** WKWebView `createPDF`/`NSPrintOperation` 的 PDF 文本层与字体嵌入。
- **未验证** PPTX/XLSX/legacy 完整渲染；spike 只覆盖解析/保存。
- **未修改** 任何现有需求/规则/代码/测试；本文仅为审计与提案。

---

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-11-04 | v1.0 | 首版：LO 依赖专项审计——8 类职责、9 个 REQ 的影响矩阵、全量代码/资源/构建/测试/文档触点、当前健康度与证据冲突、原生替换能力缺口、A–E 方案取舍、目标架构、P0–P5 迁移路线、治理变更清单、验收语料与安全计划、风险与 D1–D9 决策门 |
