# Risk Acceptance · DOC-LO-SANDBOX-VALIDATION-01

## 声明

**状态：ACCEPTED_RISK（验证延期，不是测试通过，也不是功能闭合）**

人类于 2026-08-01 明确接受：当前受限 Rust 测试进程中，
`doc::libreoffice_runtime::tests::docx_to_pdf_end_to_end_when_libreoffice_available`
启动内置 LibreOffice 后以 `DOC_PREVIEW_WORKER_CRASH (signal=6)` 失败；该结果暂不作为
Binder 已打包应用的 DOCX 预览功能缺陷裁决依据。

此接受仅允许不相关任务如实报告 `related_rust: ACCEPTED_RISK (262/263)`；不得将其报告为
全量 Rust 测试通过，不得删除、`ignore`、`skip` 或弱化该测试，也不得恢复系统 LibreOffice/PATH 回退。

## 事实与边界

- 当前设备没有 `/Applications/LibreOffice.app`；Binder 项目资源中存在受控
  `src-tauri/resources/libreoffice/LibreOffice.app`（25.8.7.3）。
- 已观察到编译后的 Binder 可执行预览；这一观察尚未构成发布态验证证据。
- 受限进程中的已知崩溃路径为 macOS AppKit 注册：LibreOffice Quartz VCL 在 `--headless` 下仍可能
  初始化 `NSApplication`，进而在受限环境中 `abort`。这不能单独证明打包应用也会失败。
- 本记录不修改 `BLOCK-REL-LO-001`：内置 LibreOffice 的签名、公证、干净设备和真实发布态预览验证仍是
  G6 发布门要求。

## 关联治理

- 规则/链路：`BR-DOC-DATA-001`、`BR-DOC-STATE-001`、`X-SEQ-004`、`DOC-CONVERT → PRV-VIEW`。
- 注册表依据：`.governance/test-governance.json` 的 `execution_constraints` 已要求在非沙箱 macOS
  进程复现后，方可作产品缺陷裁决。
- 既有发布债务：`docs/00_core/A-CORE-X-R-01_门禁闭合债务分类台账.md` 中的 `BLOCK-REL-LO-001`。

## 退出条件

在成本允许时，必须以 Binder 的已打包应用在具备 AppKit 注册条件的 macOS 用户会话完成以下验证：

1. 在未安装系统 LibreOffice 的干净设备或测试账号上，确认预览命中包内 `soffice`。
2. 以固定 DOCX 连续预览 20 次：每次生成有效 PDF、无 LibreOffice GUI、无 Crash Reporter。
3. 记录应用与 LibreOffice 构建指纹、包内工具路径、PDF 页数/文本层/嵌入字体摘要和崩溃报告检查结果。
4. 核验嵌套 LibreOffice 的签名与公证；随后在第二个干净环境复验。

若任一步失败，恢复为 DOC 功能缺陷调查，修复内置 LibreOffice 启动、打包或签名链；不得以测试环境为由降级。
若全部通过，重新设计测试执行位置：真实 LibreOffice E2E 应由非沙箱 macOS 发布态 runner 执行，普通
Rust 测试继续覆盖 Worker 的隔离、超时和错误映射。

## 并行 Scope

```yaml
task_type: GOVERNANCE_CHANGE
governance_phase: VALIDATION
allowed_files:
  - .governance/issues/DOC-LO-SANDBOX-VALIDATION-01.md
forbidden_files:
  - .governance/active_scope.json
  - src/**
  - src-tauri/**
  - docs/**
  - ADU.md
  - ADP.md
  - AGENTS.md
  - CLAUDE.md
  - governance.config.json
  - package.json
confirmation: YES
scope_reason: 记录受限测试环境下 LibreOffice E2E 失败的风险接受与发布态验证出口；不改变产品功能或测试行为。
```

## 验证记录

- 计划：`npm run test:plan -- --files .governance/issues/DOC-LO-SANDBOX-VALIDATION-01.md`
- 审计：`npm run governance:audit`
- 完整性：`git diff --check`

