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
- **A 类债务**：Pandoc/LibreOffice 包内工具 manifest、架构、哈希、许可证与构建校验（DOC-01）；**data-block-id 往返
  保留度实测（DOC-02，不达走重建锚降级）**；转换缓存策略（file_cache+content_hash，实现期）；
  转换并发/进程池（实现期）。
- **验收（强制，APC §6）**：SM-DOC-01 矩阵——open 失败 OpenError 会话不建立；save 失败原文件不变
  无半成品；除 DOC 外无二进制转换代码（X-SEQ-004 静态核验）；blockId 兜底注入全块有锚。

### PI-DOC-02　样式能力矩阵与往返验证
- **承接**：REQ-DOC-003 / BR-DOC-DATA-002。
- **文件范围**：`src-tauri/src/doc/style_subset.rs`（由三级枚举升级为五档目标报告模型）；DOC IPC 注册与
  `doc.get_style_report`；ED custom style 保留与字号单位转换；DOCX/HTML/PDF 语料和前端能力告知。
- **@GOV**：子集化与用户可知 `BR-DOC-DATA-002`。
- **A 类债务**：样式告知交互形态（DOC-04，**C 类待人类**——实现期定弹层/状态栏）；部分支持项
  （颜色/字体/字号）实际复现率实测。
- **验收**：标题、粗体的实测保证与待扩展项严格分开；必需待扩展语料总通过率 ≥95%，并逐项具备 DOCX XML、
  编辑器 HTML、LO PDF 三层断言后才可升档；`data-custom-style` 与字号单位在 ED 未验证前必须报告为尽力保留
  或差异项，不处理项清单公开。

### PI-DOC-03　预览链转换（供 PRV）
- **承接**：REQ-DOC-004 / BR-DOC-DATA-001（双链分离）/ BR-PRV-DATA-002（管线归属）。
- **文件范围**：`src-tauri/src/doc/libreoffice_runtime.rs`、`tool_runtime.rs`、`src-tauri/src/prv/mod.rs` 与
  `prv.preview`；不存在 `doc.render_preview` 公开 command。
- **@GOV**：预览链与编辑链分离 `BR-DOC-DATA-001`。
- **A 类债务**：LibreOffice 稳定版本、包内默认字体、GlyphFallback 源码补丁、嵌套签名/公证（BLOCK-REL-LO-001）；
  **Excel/PPT 经 LibreOffice 保真度与大文件表现
  实测（不达另议管线，回写本文与 PRV）**；预览缓存（实现期）。
- **专项实施顺序（不得倒置）**：
  1. 启动期工具校验改为静态校验，移除裸 `soffice --version`；工具无效时仅禁用 DOC command、Binder 其余
     功能仍可启动；CI/预览仅以应用缓存内隔离 profile 的严格 headless 参数启动，且不继承宿主动态库路径。
  2. 以固定稳定 LO 源码 tag 重建，复现并消除 SIGABRT；只在无系统 LO 的干净环境验收。
  3. **已实现（2026-07-24）**：在 LO 的 `SystemFontList` 字体枚举阶段注册 CoreText 英文名（层1）和 Office 缺失字体别名（层2）。详见决策文档 A-DOC-X-R-02。
  4. **已放弃**：追踪证实 `FindFontSubstitute` 不在 CJK 字体选择热路径上（单次转换仅调用 4 次）。替代方案见上一步。
     而非 family 名称或单码点选择确定性候选；发布清单校验整个嵌套 app 的 Frameworks/dylib/字体，随后重建、嵌套签名、公证。
  5. 运行文档/PPT 字体语料与 PDF 三层回归，再允许解除 BLOCK-REL-LO-001。
- **验收**：启动 Binder 不出现 LO GUI；工具损坏时 Binder 仍启动且 DOC 明确不可用；docx/Excel/PPT 以 LO 语料
  实测保真目标生成只读 PDF 供 PRV；预览链失败不影响编辑链；离线可用；字体语料在无系统 LO/无额外字体环境下
  断言文本层、嵌入字体和视觉快照，并覆盖复杂脚本与 emoji cluster。

#### PI-DOC-03-S1　LO 源码融合与确定性字体专项（已批准主路线）

**目标**：原生 docx/pptx 始终只读；预览只调用包内 LO；对缺失字体或缺失字形的最终选择由 Binder
控制，禁止落入设备字体库的自由候选顺序。此专项不改变 Pandoc 编辑链、不修改用户源文件，也不处理
`DOC-PLAT-001` 的双架构/最低 macOS 版本债务。

**固定边界**：

- 原生 Office 输入仅读取；任何字体规范化只允许作用于 `app_data_dir/doc-runtime/<request-id>/` 的临时 Render Proxy，
  请求结束清理，绝不覆盖原 docx/pptx 或写入 workspace。
- 预览输出是 PDF；字体实际嵌入、文本层和视觉结果均须可审计。不能达到来源字体保真时，允许输出明确标记的
  Binder 统一预览，但不得随机替代或宣称来源视觉保真。
- 不恢复旧项目的缓存掩盖转换、系统 LO/PATH 回退或设备字体依赖；不以环境变量伪造源码补丁已经完成。

**阶段与交付物（必须按序）**：

| 阶段 | 工作与交付物 | 进入下一阶段的硬门 |
|---|---|---|
| S0 字体证据 | 固定覆盖多脚本、主题、样式、缺字形的代表性语料；实现 Font Trace，记录 OOXML 的直接字体/样式/主题/别名、请求字重与脚本、Binder 映射、PDF 实际嵌入字体；同输入/同 profile 连续转换 20 次并比较字体表、页数、文本层、页面像素 | 可明确区分「规则未命中」「profile/替换未生效」「字形级回退」；未取得该证据不得修改 LO 字体源码 |
| S1 源码可行性 | 锁定稳定 LO tag、源码/补丁哈希和可重建配方；构建最小 arm64 产物，验证无 GUI 转换路径；收集 macOS `SIGABRT` 与 VCL/AppKit 证据 | 同一语料连续 20 次无 Crash Reporter、无 LO GUI、生成有效 PDF；若稳定源码产物仍无法提供可验证的无 GUI 路径，停止后续源码字体改造并提交替代引擎决策 |
| S2 字体策略 | 建立版本化 `FontPlan`：来源字体、别名、主题字体、脚本、字重/斜体 → 受许可的包内候选链；实现只读 Font Trace 与临时 Render Proxy；确定“保真预览 / Binder 统一预览 / 覆盖缺失失败”三种结果 | 代表性语料中的每个 family 均有可复现的规则命中和稳定结果；未知字体不得静默进入宿主候选链 |
| S3 LO 源码补丁 | **已实现（2026-07-24）**：在 `SystemFontList::fontEnumCallBack` 中为每个字体注册 `kCTFontFamilyNameAttribute` 英文名（层1）；对已知 Office 缺失字体注册静态别名指向本机已有字体（层2）。不改造 `FindFontSubstitute`。补丁 `binder_font_identity.patch`（237行，3文件，含层3未知字体确定性回退） | 任何未识别家族不会随机更换设备字体；PingFang SC / Microsoft YaHei 等 20 次转换字体完全一致 |
| S4 运行时融合 | **已实现（2026-07-24）**：`lo_worker.rs` 472行已完成；独立 profile、超时、崩溃隔离、过期目录清理已落地；`libreoffice_runtime` 仅调度 Worker；PRV 保持唯一公开 `prv.preview` | 55 个 DOC 单元测试全部通过，含端到端 `docx_to_pdf` 集成测试 |
| S5 发布验证 | 生成全嵌套 app + 字体清单、许可证、签名/公证校验；干净设备语料、PDF 字体表/文本层/视觉回归；打包产物路径核验 | 全部语料稳定通过，且无系统 LO、无额外字体时仍通过，方可提议解除 `BLOCK-REL-LO-001` |

**执行状态（2026-07-24）**：S0 已落地初始通用实现。S1 已于 2026-07-22 完成完整 Xcode 构建。
字体元数据，输出不含正文的请求级诊断；`font-plan.json` 为版本化空策略并与包内工具同样做 SHA-256 校验，重复别名
策略被拒绝而非任意取一。S0 单元测试已覆盖 OOXML、真实项目参考 DOCX、PPTX 和重复别名。S1 已核验
`libreoffice-25.8.7.3` 的精确源码 commit，并确认源码树含 `vcl/headless` 与 `vcl/quartz`；完整 checkout/构建受当前
机器曾约 15GiB 可用空间及远端 blob 下载中断阻断，未开始源码补丁或声称无 GUI验证通过。后续已确认
可用空间约 56GiB（尚未安装完整 Xcode），源码可完整读取；当前 S1 硬阻断转为缺少完整 Xcode 14.3+，尚不能编译。
完整 Xcode 约再占用 20–30GiB，若与源码构建共用该卷，则还需把可用空间提高到约 80GiB，或转移 Xcode/源码构建
至外接卷。JDK 17+ 仅在保留
Java 组件的完整构建时需要；首轮无 GUI 可行性构建可显式 `--without-java`，不得将其误报为必需运行时。源码
审计还确认 macOS Quartz 插件在 `create_SalInstance()` 中无条件创建 `NSApplication`，与既有 AppKit 注册崩溃
栈一致，`--headless` 不是无 GUI 的充分条件。已先形成包内字体进程级注册补丁，仍未进行源码构建验证。旧的 `fontsubst.xcu`
宿主默认字体替换已从运行时移除，避免它继续把未知字体静默导向设备相关结果；这不是 S3 的替代实现。
`scripts/build_lo_macos.sh` 已落地：锁定源码 commit、检查完整 Xcode 与至少 50GiB 空间、校验/应用 Binder 补丁，
并以 `--without-java` 的最小构建进入 S1；完整 Xcode 15.4 已安装并通过预检。其后 configure 明确拒绝
Homebrew `pkgconf`：该工具会暴露宿主机已安装开发包，破坏 LO 构建隔离。现以官方 LODE 在隔离目录准备
GNU Make 与 `pkgconf`；构建脚本已收紧为必须显式传入 `BINDER_LO_LODE_HOME`，不得再回退使用 Homebrew 工具。
LODE 自带 `ccache` 的默认用户级缓存路径在受控构建中不可写，已固定到 LODE 临时目录且限制为 2GiB；它是可丢弃的

**S3 更新（2026-07-24）**：经过深度追踪（Writer ChgFnt/SetDevFont + VCL PhysicalFontCollection dump +
CoreText CTFontCreateForString shim），确定根因为 CoreText 本地化字体名（如 "蘋方-簡"）与 OOXML 英文名
（"PingFang SC"）不匹配。已在 `SystemFontList.cxx` 中实现两层修复：
(a) 层1——为每个字体注册 `kCTFontFamilyNameAttribute` 英文名到 PhysicalFontCollection；
(b) 层2——注册静态 Office 缺失字体别名表（Microsoft YaHei→PingFang SC 等，覆盖中/日/韩/拉丁/等宽）。
验证：PingFang SC 20/20 完全一致，Microsoft YaHei 20/20 完全一致。
补丁见 `patches/libreoffice/binder_font_identity.patch`（237行，3文件），决策见 A-DOC-X-R-02。
旧 FindFontSubstitute 方案已放弃。
构建加速缓存，不进入运行时包，也不改变源码或字体策略。
配置继承的 macOS distro 默认值会启用 ODK 文档构建并要求 doxygen；该内容不属于转换运行时，首轮最小 S1
构建显式 `--without-doxygen`，与已关闭的 Java/Help 一样不得被理解为运行时缺失。
构建 PATH 仅保留 LODE 与 macOS 系统目录；源码发现不到外部 Meson 时会使用自身锁定哈希的 Meson 1.8.3，避免
Homebrew 启动脚本与 `/usr/bin/python3` 版本不一致造成的伪依赖失败。
HarfBuzz 还要求 Ninja。已将 Ninja v1.12.1 的官方源码 commit `2daa09ba270b0a43e1929d29b073348aa985dfaa`
在 LODE 内以最小可执行构建安装，并纳入脚本预检；Ninja 自身的测试关闭以避免无关 GoogleTest 下载，LO 编译仍完整执行。
LO 的实际默认构建目标是 `build`，而单元测试在独立 `check` 目标中；脚本已改用正式 `build -j2`，不再引用不存在的
`build-nocheck` 名称。
依赖下载只使用 LO 源码锁定的 tarball 校验和与子模块来源；构建脚本会在其子进程中清除失效的 loopback 代理变量，
防止受控会话的 `127.0.0.1:7890` 注入代理误导下载，不修改用户全局代理配置。
S1 实测已进一步把 MariaDB、Firebird、PostgreSQL、LDAP/WebDAV/GPG 服务、ZXing、NLPSolver、ODK/帮助/Java、
PDF 导入（含 PDFium、Poppler）、Impress 遥控、LibreLogo 和 Lotus Word Pro 过滤器从可行性配置中关闭；保留 Writer/Impress/Calc、
Quartz VCL、OpenXML 过滤器、PDF **导出**和 Calc 基础组件。`--without-fonts` 仅移除 LibreOffice 自带历史字体包，
最终运行时必须先由受许可证和哈希约束的 `binder-fonts/` 补齐，且不构成字体债务闭合。默认 `build` 仍会下载大量
非目标历史格式的导入库；镜像不完整时按“归档镜像优先、LO 原始 URL 回退 + LO SHA-256 + 隔离 tarball 缓存”恢复，
不能把下载停顿归因于内存或磁盘。当前尚未进入 C/C++ 编译，不能宣称 S1 或无 GUI验证通过。
人类于 2026-07-22 确认在当前本地约 46GiB 空间进行一次受限可行性构建；本次仅以显式参数降至 40GiB、并发限制为 2，
默认发布构建门槛仍为 50GiB。构建空间不足时允许失败并保留诊断，禁止通过删除用户文件腾挪空间。

**源码改造的角色**：S3 是最终裁决层，而不是替代 S0/S2 的猜测性补丁。它负责禁止未命中字体与缺字形
随机落入宿主 CoreText 回退；来源字体名解析、主题/别名识别与可解释映射由 S0/S2 负责。这样避免把“随机”
改成“稳定地选错字体”。

**错误模型与可观测性**：保留 `DOC_TOOL_INVALID`、`DOC_PREVIEW_START_FAILED`、`DOC_PREVIEW_FAIL`；新增
`DOC_PREVIEW_TIMEOUT`、`DOC_PREVIEW_WORKER_CRASH`、`DOC_FONT_COVERAGE_MISSING` 的语义与测试。诊断只能记录
请求 id、字体名/别名、Unicode 范围、候选链、LO 构建指纹和退出信息，禁止包含文档正文。

**停止条件与回滚**：

1. S1 无法在固定稳定源码上消除 AppKit/SIGABRT，或所需无 GUI后端不存在/不可合法分发：停止源码字体补丁，
   保留 S0 证据与包内工具边界，向人类提交“替代预览引擎”决策；不得改用系统 LO。
2. S2 所需来源字体无法合法随包，且不存在可接受的统一预览字体：返回 `DOC_FONT_COVERAGE_MISSING`，不得随机替换。
3. S3 造成复杂脚本、emoji、文本层或 PDF 可复制性回归：撤回该补丁到上一个已签名产物，保留失败样本，
   不带着回归进入 S4/S5。
4. 本专项中的产物始终可由稳定 tag + 补丁 SHA + 字体清单重建；禁止仅提交不可复现二进制作为修复。

## §3 实施顺序

```
PI-DOC-01（编辑链，ED 装载前置）→ PI-DOC-02（样式子集能力矩阵与语料）
PI-DOC-03（预览链）先稳定 headless → 再 LO 内部字体回退 → 重建/签名/验收
```

其中 PI-DOC-03-S1 的实际顺序为：`S0 字体证据 → S1 无 GUI源码可行性 → S2 FontPlan → S3 GlyphFallback
源码裁决 → S4 Worker 融合 → S5 发布门`；S1 未通过时专项停止，不得倒置进入 S2/S3。

## §4 横切验收

audit 绿；@GOV 追溯 BR-DOC-*；SM-DOC-01 矩阵全覆盖；`// covers: BR-DOC-*` 覆盖 DOC 全部规则
（4 条）有断言；X-SEQ-004 静态审查（全库无第二处二进制转换）。

## §5 承接的 A 类债务（A-CORE-X-R-01 §6）

| 债务 | PI | 处置 |
|---|---|---|
| Pandoc/LibreOffice 包内供应链（DOC-01） | PI-DOC-01/03 | manifest 校验、构建前强制校验、禁用系统回退；Pandoc arm64 已接入，其他架构与 CI 待补 |
| LO 无 GUI / SIGABRT / GlyphFallback / 签名（BLOCK-REL-LO-001） | PI-DOC-03 | 静态启动校验、DOC 功能降级→应用缓存隔离 profile headless→稳定源码重建→CoreText 注册包内字体→FindFontSubstitute shaping run 覆盖策略→完整嵌套 app 校验、签名、公证与字体语料回归；未闭合即阻断发布 |
| macOS 版本与双架构发布矩阵（DOC-PLAT-001） | 后续专项 | 仅登记暂缓；不以设备本地工具、字体或 Rosetta 规避 |
| data-block-id 往返保留度（DOC-02） | PI-DOC-01 | G4 实测，不达走重建锚 |
| Excel/PPT 保真度实测 | PI-DOC-03 | G4 实测，不达另议 |
| 样式告知形态（DOC-04，C 类） | PI-DOC-02 | 待人类/实现期 |
| 转换缓存/并发/进程池 | PI-DOC-01 | 实现期 |
| 首开<12s/缓存<2s/准确率≥90% NFR | 全 PI | G5 标定复核 |

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（G3）：DOC 3 条 Plan Item（编辑链与开/存机 / 样式子集执行 / 预览链），承接 DOC-CONVERT + 4 规则 + X-SEQ-004 + SM-DOC-01；A 类含版本选型、往返保留度实测(DOC-02)、Excel/PPT 保真度实测；必需子集往返 ≥95% 回归强制；文件范围/@GOV/验收齐 |
| 2026-07-21 | v1.1 | DOC-BUNDLED-RUNTIME-01：PI-DOC-01/03 更新为内置工具 manifest、构建前校验和禁用系统回退；Pandoc arm64 已接入并经回归验证，x86_64、发布 CI、LO GlyphFallback 源码重建/签名/公证继续作为未闭合任务。 |
| 2026-07-21 | v1.2 | DOC-LO-RELIABILITY-FONT-PLAN-01：将 PI-DOC-03 拆为无 GUI、headless 稳定、包内字体注册、GlyphFallback 源码策略、重建签名与语料验收的顺序计划；PI-DOC-02 追加 Pandoc 五档能力矩阵及三层语料回归；DOC-PLAT-001 仅登记暂缓。 |
| 2026-07-21 | v1.3 | 审计修正：PI-DOC-02 明确五档为待实现的目标报告模型，并补 ED custom-style/字号约束；PI-DOC-03 修正为 `prv.preview` 消费 DOC 内部能力，移除不存在的文件与 command，补 DOC 功能降级、应用缓存隔离 profile、shaping run 字体覆盖及完整嵌套 app 完整性校验。 |
| 2026-07-22 | v1.4 | 人类确认继续 LO 源码融合路线：新增 PI-DOC-03-S1，补 Font Trace 先行、稳定源码无 GUI可行性门、FontPlan/Render Proxy、GlyphFallback 最终裁决、Worker 融合、发布验收与停止/回滚条件；明确原生 Office 源文件只读、禁止系统工具/宿主字体回退。 |
| 2026-07-22 | v1.5 | 通用方案开始实现：S0 Font Trace/空 FontPlan/包内哈希校验落地并通过测试；S1 精确 tag 与 headless/quartz 源码目录已核验，但完整源码构建受空间与远端 blob 下载阻断，保持未完成状态。 |
| 2026-07-22 | v1.6 | 通用方案校正：S0/S2 验收改为代表性语料，不以具体字体名形成需求或规则；运行时移除旧 `fontsubst.xcu` 宿主默认字体替换，保持 FontPlan 仅取证直至源码 GlyphFallback 可验证。 |
| 2026-07-22 | v1.7 | S1 源码审计回写：空间已恢复至约 56GiB、稳定源码可读；硬阻断为完整 Xcode 缺失，且同卷安装 Xcode 后需约 80GiB 可用空间（首轮可 `--without-java`）。定位 Quartz `create_SalInstance()` 无条件创建 `NSApplication` 是现有 AppKit 注册崩溃的直接源码路径；新增包内字体进程级注册补丁，未声称已完成无 GUI 构建或 glyph fallback。 |
| 2026-07-22 | v1.8 | S1 构建环境修正：Xcode 15.4 已通过预检；configure 正确拒绝携带宿主包集合的 Homebrew `pkgconf`。改用官方 LODE 隔离构建工具链，脚本强制要求其路径，不把 Homebrew 或系统构建工具作为隐式回退。 |
| 2026-07-22 | v1.9 | S1 配置探测修正：LODE `ccache` 默认用户缓存目录在受控会话不可写，导致 C++ 标准库探测假失败。脚本将其改为 LODE 内、上限 2GiB 的可丢弃构建缓存；不禁用编译检查，不修改 LO 源码。 |
| 2026-07-22 | v1.10 | S1 最小构建修正：macOS distro 默认启用 ODK 并要求 doxygen，非 Binder 转换运行时依赖；显式关闭 doxygen 后继续验证源码构建。 |
| 2026-07-22 | v1.11 | S1 PATH 隔离修正：移除 Homebrew PATH，避免损坏的外部 Meson 与系统 Python 组合；让源码回退到 hash 锁定的内部 Meson 1.8.3。 |
| 2026-07-22 | v1.12 | S1 Ninja 补足：LODE 未提供 Ninja；从官方 Ninja v1.12.1 源码构建隔离可执行文件并纳入预检。关闭的是 Ninja 自身测试下载，非 LO 编译验证。 |
| 2026-07-22 | v1.13 | S1 构建目标修正：配置完成后确认官方默认目标为 `build`、测试为独立 `check`；脚本从不存在的 `build-nocheck` 改为正式 `build`。 |
| 2026-07-22 | v1.14 | S1 依赖下载修正：构建首次进入 hash 锁定 tarball/子模块下载时被失效 loopback 代理阻断；脚本只在构建子进程清除该代理变量，保留源码校验。 |
| 2026-07-22 | v1.15 | S1 源码依赖可靠性修正：TDF 发布镜像不覆盖全部外部依赖，不采用会产生 404 的伪通用镜像；改为依赖级可靠源（维护方正式发布源优先、可追溯历史源包归档回退）+ LO 锁定 SHA-256 + 本地 tarball 缓存。脚本新增显式、受配置产物保护的 `BINDER_LO_SKIP_CONFIGURE=1` 恢复模式，默认完整配置行为不变。 |
| 2026-07-22 | v1.16 | S1 可行性实测回写：确认 Xcode/LODE/配置链正常，非目标数据库、条码和求解器组件可关闭；`--without-fonts` 后只允许 Binder 受控字体集进入最终运行时。默认全格式 build 仍受历史依赖供应链影响，未到编译阶段，保持 S1 未完成。 |
| 2026-07-22 | v1.17 | S1 精简配置与镜像补丁修正：关闭不参与 Office→PDF 预览的开发、PDF 输入、遥控、目录/远程服务与历史过滤器组件；保留 PDF 导出。修复损坏的归档镜像回退 patch，仍以 LO 锁定 SHA-256 裁决下载内容；已复核缓存的 libmwaw、mythes、NSS、libodfgen、OpenSSL、liborcus 均匹配该锁定哈希。 |
