# Issue Trace · DOC-BUNDLED-RUNTIME-01 内置 Pandoc / LibreOffice 运行时修复

承接 `REQ-DOC-004`、`REQ-DOC-005`、`BR-DOC-DATA-001`、`BR-DOC-DATA-002`、`BR-DOC-STATE-001`、
`X-SEQ-004` 与 `BLOCK-REL-LO-001`。人类确认：Pandoc 与 LibreOffice 必须随 Binder 应用内置交付；
运行时不得探测、调用或回退到设备安装的工具。LibreOffice 的 GlyphFallback 字体债务必须保留为发布阻断，
并在可取得 LO 源码与重建权限后完成源码级修复。

## 1. 用户可见症状

- 在未安装 Pandoc 的设备上，DOCX 编辑/导出依赖系统 PATH；在装有不同版本 Pandoc 的设备上，转换结果可能漂移。
- 预览链会回退到设备 LibreOffice；内置 LibreOffice 当前 headless 转换直接以 `Abort trap: 6` 退出，PPTX 的字形级字体回退债务也尚未修复。

## 2. 入口点

- DOCX 编辑打开/保存：`src-tauri/src/lib.rs` 的 `route_open_for_edit` / `route_save_from_edit`，经 `doc::edit_chain`。
- 文件导出：`src-tauri/src/prv/mod.rs` 的 `export`，经 `doc::pandoc_runtime`。
- DOCX/XLSX/PPTX 预览：`src-tauri/src/lib.rs` 的 `route_preview` → `prv::render_preview` → `doc::libreoffice_runtime::convert_to_pdf`。
- 应用启动：`src-tauri/src/lib.rs` 的 `run().setup()`，本次在此初始化包内工具运行时。

## 3. 相关规则

- `BR-DOC-DATA-001`：二进制文档唯一转换通道、双链分离、本地执行、依赖缺失明确报错。
- `BR-DOC-DATA-002`：StyleSubset 往返与资源契约。
- `BR-DOC-STATE-001`：转换失败不产生半成品、可重试。
- `X-SEQ-004`：DOC 是二进制文档格式的唯一转换通道。
- `BR-PRV-DATA-002`：PRV 只编排 DOC 预览/导出链。
- `BLOCK-REL-LO-001`：发布包必须使用内置优化版 LibreOffice，不得静默降级到系统 LO。

## 4. 相关链路

`DOC-CONVERT → PRV-VIEW`；编辑方向为 `DOC-CONVERT → ED-EDIT → WS-SAVE`。

## 5. 生产者块

- `BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-001/002`：`src-tauri/src/doc/pandoc_runtime.rs`。
- `BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-003/005`：`src-tauri/src/doc/libreoffice_runtime.rs`。
- `BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-003/004`：`src-tauri/src/doc/edit_chain.rs`。
- `BR-PRV-DATA-002-IO-PRV-PRV-VIEW-001/002`：`src-tauri/src/prv/mod.rs`。

## 6. 消费者块

- `route_open_for_edit` / `route_save_from_edit`：`src-tauri/src/lib.rs`。
- `route_preview`：`src-tauri/src/lib.rs`。
- 前端 DOC Service：`src/services/docService.ts`（只消费既有 IPC 契约，本次不修改）。

## 7. 路径上的空规则块

本次新增的工具清单、构建校验与测试辅助不承载新的业务判断；它们复用 `BR-DOC-DATA-001` 的包内依赖
不降级语义。新建的包内路径校验函数必须有对应 `@GOV`，不保留未分类 `rules: []` 业务块。

## 8. boundary 假设

- 编辑链仅执行受控 Pandoc，预览链仅执行受控 LibreOffice；两链不共享中间态，假设继续成立。
- `reference.docx` 和 `preserve-styles.lua` 是编辑链资源；本次将其与 Pandoc 一并纳入受控资源根，假设成立。
- 当前 LibreOffice 的字体 family 级补丁不能覆盖 GlyphFallback；该假设已由债务台账复测确认，必须保持显式阻断。

## 9. 运行时假设

- `DocToolRuntime` 在 Tauri setup 时由 `resource_dir()` 初始化；未初始化、工具缺失、版本不匹配均返回明确错误。
- 每次 Pandoc/LibreOffice 子进程只允许使用已校验的绝对包内路径；不继承 PATH 作为工具来源。
- 每个预览请求拥有独立输出目录；只接受与输入同 stem 的 PDF，禁止选择目录中“最新”文件。

## 10. 疑似不变量破坏

- 系统 PATH 或 `/Applications` 回退违反 `BLOCK-REL-LO-001` 及本次人类确认的内置依赖边界。
- 缺失 Pandoc 时跳过测试会把发布依赖缺失误判为成功，违反 `BR-DOC-DATA-001` 的明确失败语义。
- 无法在本仓修复 LO GlyphFallback 或 headless 崩溃；若没有可重建的源码与已签名产物，G6 发布门仍被阻断。

## 11. 验证计划

- 资源：验证 manifest、Pandoc 版本、SHA-256、架构、包内可执行路径；系统 PATH 中的 Pandoc/LO 不得影响结果。
- Rust：运行 DOC 单元与集成测试；Pandoc 真实 DOCX/HTML 往返必须执行，不可因资源缺失跳过。
- LO：验证只解析包内路径；真实 headless `docx→PDF` 必须通过，否则如实保留 `SC-04` 阻断及崩溃证据。
- 发布：`npm run build` 前执行工具校验，`npm run governance:generate` 后运行 `npm run governance:audit`。

## Scope Lock

```json
{
  "task_type": "CODE_CHANGE",
  "scope_id": "DOC-BUNDLED-RUNTIME-01",
  "allowed_files": [
    ".governance/active_scope.json",
    ".governance/issues/DOC-BUNDLED-RUNTIME-01.md",
    ".gitignore",
    "package.json",
    "src-tauri/Cargo.toml",
    "src-tauri/tauri.conf.json",
    "src-tauri/resources/tools/README.md",
    "src-tauri/resources/tools/manifest.json",
    "src-tauri/resources/tools/pandoc/arm64/pandoc",
    "src-tauri/resources/libreoffice/README.md",
    "scripts/deploy_lo.sh",
    "scripts/prepare_bundled_tools.sh",
    "scripts/verify_bundled_tools.sh",
    "docs/00_core/A-CORE-X-R-01_门禁闭合债务分类台账.md",
    "docs/01_modules/DOC/A-DOC-M-D-01_文档转换功能需求描述.md",
    "docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md",
    "docs/01_modules/DOC/A-DOC-I-P-01_文档转换接口协议.md",
    "docs/01_modules/DOC/A-DOC-X-L-01_文档转换开发计划.md",
    "docs/00_core/A-CORE-C-R-01_ADUS.md",
    "src-tauri/src/lib.rs",
    "src-tauri/src/prv/mod.rs",
    "src-tauri/src/doc/mod.rs",
    "src-tauri/src/doc/tool_runtime.rs",
    "src-tauri/src/doc/pandoc_runtime.rs",
    "src-tauri/src/doc/libreoffice_runtime.rs",
    "src-tauri/src/doc/edit_chain.rs"
  ],
  "forbidden_files": [
    "ADU.md",
    "ADP.md",
    "AGENTS.md",
    "CLAUDE.md",
    "docs/00_core/A-CORE-C-P-01_APC.md",
    "src-tauri/resources/libreoffice/LibreOffice.app/**",
    "src/**"
  ],
  "confirmation": "YES",
  "scope_reason": "将 Pandoc/LibreOffice 变为 Binder 包内唯一转换运行时，删除系统工具回退并建立构建校验；LO 源码级字体补丁及重建不在当前仓库范围内，保持发布阻断。"
}
```
