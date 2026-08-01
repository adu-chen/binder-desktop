# Issue Trace · DOC-LO-RELIABILITY-FONT-IMPLEMENT-01

## 1. 用户可见症状

- Binder 启动会执行 `soffice --version`，开发机上可能出现完整 LibreOffice GUI；包内 LO 失效也会阻断 Binder 全部启动。
- 预览临时文件位于无界系统临时目录；样式报告仍为三级，字号会把 `pt` 重写为 `px`。

## 2. 入口点

- `src-tauri/src/lib.rs`：`run().setup()`、`route_preview`、command registry。
- `src-tauri/src/doc/tool_runtime.rs`：`initialize`、`validate_tool`。
- `src-tauri/src/prv/mod.rs`：`render_preview`；`src-tauri/src/doc/libreoffice_runtime.rs`：`convert_to_pdf`。
- `src-tauri/src/doc/style_subset.rs`、`src/components/ed/extensions/FontSize.ts`。

## 3. 相关规则

`BR-DOC-DATA-001`、`BR-DOC-DATA-002`、`BR-DOC-STATE-001`、`BR-PRV-DATA-002`、`X-SEQ-004`。

## 4. 相关链路

`DOC-CONVERT → PRV-VIEW`；编辑样式报告仍属于 `DOC-CONVERT`。

## 5. 生产者块

- `BR-DOC-DATA-001-GUARD-DOC-DOC-CONVERT-002`：`src-tauri/src/doc/tool_runtime.rs`。
- `BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-003/005`：`src-tauri/src/doc/libreoffice_runtime.rs`。
- `BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-001/002`：`src-tauri/src/doc/style_subset.rs`。

## 6. 消费者块

- `BR-PRV-DATA-002-IO-PRV-PRV-VIEW-001`：`src-tauri/src/prv/mod.rs`。
- `route_preview` 与前端 `prv.preview` 消费 DOC 转换结果。

## 7. 路径上的空规则块

无新增业务 `rules: []` 块；所有新增/改造块复用上述已注册规则。

## 8. boundary 假设

DOC 仍是唯一二进制转换通道；PRV 仅编排；工具和字体只来自包内资源。隔离 profile 不属于 workspace，必须由应用数据目录清理。

## 9. 运行时假设

启动只能作静态文件/完整性校验；LO 健康检查仅在真实预览子进程执行。源码级 GlyphFallback 不可由 Rust 环境变量替代。

## 10. 疑似不变量破坏

系统工具回退、启动 GUI、无界临时目录、把未验证五档冒充已实现都会违反 `BR-DOC-DATA-001/002`。未取得 LO 源码与稳定产物时不能声称字体债务闭合。

## 11. 验证计划

- Rust 单元/集成：工具静态校验、profile 位置、错误代码、样式五档。
- 前端：字号 `pt` 解析与渲染同单位。
- `npm run check:rust`、前端类型检查、`npm run governance:generate`、`npm run governance:audit`。

## 12. 2026-07-22 内置 LO 对照结论

- 以 Pandoc 生成的同一最小 DOCX 为输入，旧项目实际打包的
  `LibreOfficeDev 26.8.0.0.alpha0` 与当前项目的稳定版 `LibreOffice 25.8.7.3`，
  在 baseline、`SAL_USE_VCLPLUGIN=gen`、以及 `DYLD_LIBRARY_PATH + gen` 三种子进程环境下
  均以 `SIGABRT` 退出且未生成 PDF。
- 因而当前受控执行环境中的失败不能归因于新版本 LO、VCL 选择或动态库路径的单点差异；
  更不能把旧项目的表面成功当作内置转换稳定性的证据。
- 旧项目还具有“缓存命中即跳过转换”及“内置不可用时回退系统 LibreOffice”的路径；两者均不能迁入
  本项目的“仅包内工具”契约。下一步必须在实际 Tauri 图形会话/已打包应用中采集崩溃报告并复测，
  将该执行环境与受控命令环境区分后再决定 LO 源码级修复或分发策略。
- 本机 2026-07-22 崩溃报告已确认：`LibreOffice 25.8.7.3` 与旧包 `26.8.0.0.alpha0` 均在
  `NSApplication → GetCurrentProcess → _RegisterApplication → abort` 处以 `SIGABRT` 退出；
  实际加载的是 `libvclplug_osxlo.dylib`。包内没有 `svp` 等虚拟显示 VCL 插件，因此 macOS 的
  `--headless` 不能保证完全绕开 AppKit。用户在 Crash Reporter 中点击“重新打开”会以正常 GUI 入口
  启动完整 LibreOffice，不应作为转换成功或修复完成的证据。

## 13. 人类决策（2026-07-22）

- 继续以 LibreOffice 源码融合为主路线；不在本专项切换预览引擎，也不允许系统工具、设备字体或用户手工
  重启 GUI 作为回退。
- 源码改造前先完成 Font Trace：以华文魏碑、微软雅黑等真实语料证明 family 映射、profile 生效与 glyph
  fallback 的实际分支。`FindFontSubstitute` 是最终禁止随机回退的裁决层，不取代根因取证。
- 若稳定源码构建无法提供经验证的无 GUI 路径，或定制构建/授权成本无法接受，停止本专项并由人类另行决定
  替代引擎；在该决策前维持 `BLOCK-REL-LO-001` 发布阻断。

## 14. 范围扩展确认（2026-07-22）

人类确认执行通用实现并同步文档。允许新增 Font Trace/FontPlan、LO 源码补丁、源码构建与字体运行时验证
脚本及其依赖；具体字体仅作为语料，不得形成代码或策略中的专用特例。

## 15. S0 实现状态与 S1 阻断（2026-07-22）

- 已实现通用 OOXML Font Trace、版本化空 FontPlan 和 manifest SHA-256 校验；单元测试覆盖 DOCX/PPTX、
  真实参考 DOCX、策略重复别名拒绝及无正文泄露。
- 官方 `libreoffice-25.8.7.3` tag 已核验为 `30742500f2d3eb4366ac312fa33d3dcabdb3eba5`；源码树存在
  `vcl/headless` 与 `vcl/quartz`，可进入无 GUI构建可行性验证。
- 当前设备仅约 15GiB 可用空间，且按需下载 `salgdi.cxx` blob 时远端连接中断；完整源码 checkout/构建及
  `FindFontSubstitute` 补丁尚未执行。不得将 S0 成功表述为 LO 源码修复完成。
- 已移除旧运行时写入的 `fontsubst.xcu` 大范围宿主默认字体替换：它不是通用 FontPlan，且会把未知字体
  静默导向设备相关结果。现在 FontPlan 只生成诊断，实际确定性 glyph 选择仍待 S3 源码实现与验证。
- 后续空间已提升至约 56GiB，稳定源码可完整读取；S1 当前不能编译的直接原因是仅有 Command Line Tools，
  缺少完整 Xcode 14.3+。该余量尚未扣除完整 Xcode 约 20–30GiB 占用；若同卷构建，需增至约 80GiB 或使用外接卷。
  JDK 17+ 仅在保留 Java 组件的完整构建时需要，首轮可显式 `--without-java`。源码审计定位 `vcl/osx/salinst.cxx:create_SalInstance()` 无条件调用
  `[VCL_NSApplication sharedApplication]`，与此前 `NSApplication → GetCurrentProcess → _RegisterApplication → abort`
  崩溃栈相符。`--headless` 不足以承诺无 AppKit 初始化，不能继续以环境变量尝试掩盖。
- 已在稳定源码上验证并固化 `patches/libreoffice/binder_deterministic_glyph_fallback.patch` 的第一部分：在 Quartz
  构造 CoreText 字体集合前注册 `Contents/Resources/binder-fonts/` 中的进程级包内字体。此补丁不含具体来源字体名，
  也不宣称已完成候选排序或 glyph 覆盖裁决。
- `scripts/build_lo_macos.sh` 已实现并通过 shell 语法与当前环境预检：它锁定源码 commit、检查完整 Xcode 和
  至少 50GiB 空间、验证/应用补丁后以 `--without-java` 启动最小构建。当前正确停止在缺少完整 Xcode，未调用
  系统 LO、未触发 GUI、未把预检失败表述为构建成功。
- 人类确认不使用外接盘，改在当前本地约 46GiB 空间执行一次受限 S1 可行性构建。脚本默认门槛仍为 50GiB；本次
  仅经显式 `BINDER_LO_MIN_FREE_GIB=40` 覆盖，且 `BINDER_LO_BUILD_JOBS=2` 限制并发。空间不足的失败只作为构建诊断，
  禁止删除用户文件、缓存或快照以制造空间。
- 受限构建已实际运行至 LO configure：Xcode 15.4/SDK 14.5 与 arm64 编译器均通过；首次失败原因是系统 BSD Make
  3.81，不满足 LO 的 GNU Make 4.0+ 前置。系统已有 Homebrew `gmake` 4.4.1；构建脚本已改为显式验证并使用它，
  不通过替换系统 `/usr/bin/make` 解决。

## 16. S1 构建隔离修正（2026-07-22）

- 使用 Homebrew GNU Make 后，LO `configure` 在进入编译前主动拒绝 Homebrew `pkgconf`：它列出了宿主机的已安装
  开发包，LO 将此判为会破坏构建的非隔离环境。这不是内存或磁盘耗尽，也不能用 `--enable-bogus-pkg-config` 绕过。
- 已按官方路径在 `/private/tmp/binder-lode` 建立 LODE（LibreOffice Development Environment）；该目录只服务本机
  S1 构建，不进入 Binder 运行时包。`scripts/build_lo_macos.sh` 现要求显式 `BINDER_LO_LODE_HOME`，并只从其
  `opt/bin` 取得 `pkgconf` 与 GNU Make，仍使用已选定的完整 Xcode 编译器。
- 首次错误地从仓库根目录调用 LODE setup，脚本在根目录留下未跟踪的 `buildbot/`、`packages/`、`opt/`、`ext_tar/`、
  `adm/`、`tb/` 构建副产物。它们未被纳入源码、未被删除或移动；等待人类决定其清理方式，且不影响后续从
  `/private/tmp/binder-lode` 正确启动的隔离构建。

## 17. S1 C++ 探测失败根因（2026-07-22）

- 使用 LODE 后 configure 已通过 GNU Make 与 `pkgconf` 校验，并明确接受“默认搜索路径没有宿主包”的隔离状态。
  随后的 `Could not figure out what C++ library this is` 不是 Xcode 或 libc++ 缺失：`config.log` 的每个 C++ 探测
  都被 `ccache: error: Operation not permitted` 中断。
- LODE 自带 ccache 的默认 macOS 用户缓存目录在受控会话不可写。经把 `CCACHE_DIR`/`CCACHE_TEMPDIR` 指向
  `/private/tmp/binder-lode` 后，ccache 自检成功。构建脚本现固定该隔离目录，并将 `CCACHE_MAXSIZE` 设为 2GiB，
  防止受限 S1 构建无界占用磁盘；不使用 `--enable-bogus-pkg-config`、不取消源码检查、也不引入系统 LO。

## 18. S1 最小构建组件边界（2026-07-22）

- 修复 ccache 后 configure 已实际识别 `LLVM libc++`；下一处停止是 macOS distro 默认启用 ODK，并要求本机存在
  doxygen。ODK 文档生成不参与 DOCX/PDF 转换，也不属于 Binder 内置运行时。
- 构建命令现显式增加 `--without-doxygen`，与 `--without-java --without-help` 一起构成受限 S1 的最小运行时构建边界。
  这不是忽略编译/测试失败：配置会继续编译实际 Quartz/VCL/转换代码；正式发布若需要 ODK 另立范围处理。

## 19. S1 Meson 发现链修正（2026-07-22）

- doxygen 之后，configure 误发现 Homebrew 的 `meson` 启动脚本，却用 `/usr/bin/python3` 执行它，因对应 Python
  没有 `mesonbuild` 模块而失败。LO 源码 `download.lst` 已锁定 Meson 1.8.3 和 SHA-256；没有外部 Meson 时会走该
  内部副本，无需下载另一个未锁定的 Python 包。
- 构建脚本的 PATH 改为 LODE + macOS 系统目录，不再继承交互环境中的 `/opt/homebrew/bin`。这同时确保 LODE 的
  `pkgconf`/Make 是唯一可发现的构建工具，外部 Meson 不可见时由 LO 选择其锁定的内部 Meson。

## 20. S1 Ninja 隔离补足（2026-07-22）

- LO 的 HarfBuzz 构建要求 Ninja；LODE setup 未安装该程序，Xcode 不提供独立 Ninja，而宿主唯一副本来自 Homebrew，
  不可作为隔离构建输入。已下载 Ninja 官方 `v1.12.1` 源码（commit
  `2daa09ba270b0a43e1929d29b073348aa985dfaa`），使用 LODE CMake 构建并安装至
  `/private/tmp/binder-lode/opt/bin/ninja`，实测版本 `1.12.1`。
- Ninja 上游默认启用自身测试并下载 GoogleTest；该下载被网络隔离阻断，且与获得 Ninja 可执行文件无关，故仅对 Ninja
  构建使用 `-DBUILD_TESTING=OFF`。未关闭 LibreOffice 代码编译、未跳过 S1 的无 GUI 验证。LO 构建脚本现强制检查
  该隔离 Ninja 存在。

## 21. S1 正式构建目标修正（2026-07-22）

- LO 配置已完整完成，并由 `make help` 确认默认目标为 `build`；单元/系统测试在独立 `check` 系列目标中。先前脚本
  误用了不存在的 `build-nocheck`，因此在尚未编译前以 target-resolution 错误停止。
- 脚本现在调用 `make build -j<BINDER_LO_BUILD_JOBS>`。这不等于绕过测试：S1 的成功判据仍要求后续专用的无 GUI
  连续转换验证；构建完成后会单独运行该验证，而不是把不存在的 Make 目标当成“已跳过测试”。

## 22. S1 锁定依赖下载阻断（2026-07-22）

- `make build` 已实际进入 LO 的下载/子模块阶段。失败原因为会话环境将 `curl`/`git` 指向不存在的
  `127.0.0.1:7890` loopback 代理，导致 `libabw-0.1.3.tar.xz` 与 `dictionaries` 子模块均无法连接；不是内存、
  本地磁盘、Xcode 或源码补丁失败。
- 构建脚本仅在自己的子进程 `unset` 代理相关变量，避免改变用户的全局配置；随后仍由 LO `Makefile.fetch` 执行
  原始 SHA-256 校验与 submodule 来源校验。下载需要受控网络授权，完成后再进入实际编译。

## 23. S1 依赖镜像可靠性修正（2026-07-22）

- 解除 loopback 代理后，`dev-www.libreoffice.org/src` 可访问但吞吐不稳定，会长期停滞或断开；这仍不是内存、磁盘、
  Xcode 或字体补丁问题。TDF 的发布镜像仅覆盖 LibreOffice 发布源码，不覆盖本构建所需的全部第三方 tarball；经抽样验证，
  不能把不完整的发行版缓存伪装成单一镜像。
- 本次改为“依赖级可靠源”：优先从相应项目的正式发布地址取得源码；当维护方重发同名文件或没有发布资产时，才从
  Debian/NetBSD 等可追溯的历史源包归档取原始 tarball。每一份下载在进入隔离的 LODE tarball 缓存前后均以 LO
  `download.lst` 的 SHA-256 复核。已验证并缓存 Boost、HarfBuzz、Hunspell、ICU 源码/数据、LCMS、libjpeg-turbo、
  libatomic_ops、libassuan 与 libeot；不接受未校验的第三方二进制，也未修改 Binder 运行时供应链。
- `scripts/build_lo_macos.sh` 新增 `BINDER_LO_SKIP_CONFIGURE=1`：仅当已有 `config_host.mk` 时才允许跳过
  重复配置以恢复被下载中断的构建；默认仍执行完整 configure。此开关不跳过 `build`，更不跳过后续无 GUI 验收。
- S1 仍未完成：后续外部依赖、编译、无 GUI 连续转换和字体语料回归均未通过，不能宣称官方 LO 已可交付。

## 24. S1 可行性构建范围与供应链实测（2026-07-22）

- 本机已在隔离 LODE、Xcode 15.4、arm64/SDK 14.5 组合下实际完成多次 `configure`。当前 S1 构建明确关闭
  数据库驱动（MariaDB、Firebird、PostgreSQL）、LDAP/WebDAV/GPG 服务、ZXing 条码组件、NLPSolver 扩展、ODK/帮助/Java、PDF 导入
  （含 PDFium、Poppler）、Impress 遥控、LibreLogo 与 Lotus Word Pro 过滤器。重新生成的 `config_host.mk` 已证明
  前三项数据库驱动和 ZXing 不再进入 `BUILD_TYPE`。Writer/Impress/Calc、Quartz VCL、OpenXML 过滤器、PDF **导出**
  及 Excel 预览所需 Calc 基础组件没有被关闭；PDF 导入不是 Binder 的 Office 预览输入路径。
- `--without-fonts` 同样已被配置接受：它移除了 LO 随套件分发的历史字体包，避免把未由 Binder FontPlan 管理的
  默认字体带入最终策略。该开关只适用于本次 S1 可行性构建；在 Binder 包可运行前必须先放入经许可、锁定哈希的
  `binder-fonts/`，并完成字体语料验收，不能把“无字体构建成功”误报为字体债务已闭合。
- 上述配置之后的 `make build -j2` 仍首先停在外部 tarball 获取。已核验并缓存 `libmwaw`、`mythes`、NSS、
  `libodfgen`、OpenSSL 和 `liborcus`；其中 `libmwaw`/`mythes` 使用其维护方或 Debian 源码归档，NSS 使用 Mozilla
  官方发布归档，OpenSSL 与 `liborcus` 使用各自官方发布页，全部与 `download.lst` 的 SHA-256 一致。尚未到 C/C++ 编译
  或链接阶段。因此当前的阻力是默认全格式办公套件的历史依赖
  供应链，不是可用磁盘、内存、Xcode、LLVM libc++、Quartz 补丁或运行时 LO 崩溃。
- `binder_tarball_mirror_fallback.patch` 原有第二个 hunk 损坏，已在已获确认的范围内改为可验证的标准 Git patch；
  它先尝试 OpenBSD LibreOffice distfiles 归档，再回退 LO 原始 URL，并继续由 `download.lst` 的 SHA-256 裁决。
  镜像只是加速路径，不再被表述为覆盖全部历史依赖的保证。
- 已验证并放入隔离缓存的新增 tarball 均先以 LO `download.lst` 精确 SHA-256 验证，包括 `libwebp`、`xmlsec1`、
  `lp_solve`、`mdds`、`zxcvbn-c`、Meson、`libmspub`；未接纳任一哈希不符的同名上游资产。单一 OpenBSD
  镜像实测覆盖不完整，不能作为唯一构建源；正确策略仍是“镜像优先、依赖级正式/归档源回退、哈希验证后入缓存”。
- 仓库中的 `binder_tarball_mirror_fallback.patch` 第二个 hunk 计数损坏：已应用到临时源码树的等价差异可正常工作，
  但脚本重新读取仓库补丁时会报 `corrupt patch`。该补丁文件不在当前 Scope Lock 的允许列表，故本轮未将修正写回
  仓库；持久化修正前须由人类确认扩展 Scope。当前没有 Binder 运行时包、签名产物、无 GUI转换结果或字体验收产物。

## Scope Lock

允许修改：`.governance/active_scope.json` 中 `allowed_files` 列表；均为上述生产者、消费者、验证或实现状态文档。

禁止修改：`src-tauri/resources/libreoffice/LibreOffice.app/**`、治理控制文档与未列文件。LO 源码补丁/稳定重建须在具备源码与发布签名权限的专项范围执行。
