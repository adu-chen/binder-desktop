# Issue Trace：PRV-PREVIEW-OPT-01

## 目标

在不更换 PDF 渲染层（保留 iframe + WKWebView，悬浮工具栏按人类决策接受现状）的前提下，执行预览相关优化：
（1）预览链路：PDF 缓存落盘 + asset 协议授权，替换 JSON number[] 字节回传；iframe 直接加载 asset URL；
（2）本地打开：新增 `prv.open_external`（系统默认应用打开，只读守界）；
（3）预览背景色统一为应用色。LO 转换链、字体补丁、FontPlan、manifest 校验链不动。

## 1. 用户可见症状

1. 预览打开慢：PDF 字节经 JSON number[] 回传（5MB→数百万数字，序列化/传输/内存放大）；每次预览冷启动 LO 子进程 + 每请求新 profile；同文件重复预览无缓存、切 tab 卸载重挂重新转换。
2. 预览区背景（`--bg-sunken`）与编辑器/应用（`--bg-page`/`--bg-card`）不一致。
3. 无"本地打开"（系统默认应用打开）能力。

## 2. 入口点

- `src-tauri/src/lib.rs`：`route_preview`（返回形态）、新路由 `prv.open_external`、注册表。
- `src-tauri/src/prv/mod.rs`：`render_preview`（缓存落盘 + 命中返回路径）、`open_external`。
- `src/components/prv/PrvPanel.tsx`：pdf 分支 iframe 加载 asset URL（去 blob revoke）。
- `src/services/prvService.ts`：`previewDoc`（返回 asset URL）、`openExternal`。

## 3. 相关规则

`BR-PRV-DATA-001`、`BR-PRV-DATA-002`、`BR-DOC-DATA-001`、`X-SEQ-004`、`X-FORBID-004`、`BR-SYS-DATA-003`。

## 4. 相关链路

`PRV-VIEW → DOC-CONVERT`；缓存命中路径仍为 DOC 预览链产物复用（不绕过 X-SEQ-004）。

## 5. 生产者块

- `BR-PRV-DATA-002-IO-PRV-PRV-VIEW-001`（src-tauri/src/prv/mod.rs）：render_preview 缓存改造。
- `BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-005`（src-tauri/src/doc/libreoffice_runtime.rs）：LO 转换（不改）。
- `BR-SYS-DATA-003-API-SYS-IPC-CALL-001`（src-tauri/src/ipc/mod.rs）：command 注册表（不改）。

## 6. 消费者块

- `BR-PRV-DATA-001-GUARD-PRV-PRV-VIEW-003`（src/components/prv/PrvPanel.tsx）。
- `prvService.ts`、`route_preview`、`PrvToolbar`（新增本地打开按钮）。

## 7. 路径上的空规则块

无新增业务 `rules: []` 块；本地打开按钮为既有 PrvToolbar 的扩展（RB 块规则已覆盖）。

## 8. boundary 假设

PRV 只读硬边界（BR-PRV-DATA-001）：预览无编辑交互、无文档模型变更；导出落 workspace（X-FORBID-004）；
缓存目录在 app_data_dir，不在 workspace；本地打开只读查看不写回。

## 9. 运行时假设

缓存命中即跳过 LO 转换，但缓存的是 LO 已产出的 PDF（不绕过 X-SEQ-004）；asset 授权逐文件最小暴露
（同 prv.media_src 纪律）；LO 二进制/字体补丁/FontPlan/manifest 不被本次改动触碰。

## 10. 疑似不变量破坏

- X-SEQ-004：缓存为 DOC 产物复用，不引入第二转换通道。
- A-PRV-M-T-01 §5 状态机豁免失效条件：引入预览缓存编排即触发，本次复核结论维持豁免
  （同键文件存在即命中的单次文件读写，无队列/批量/后台重建），留痕于设计文档与 Runtime Report。
- X-FORBID-004：本地打开不写回、落点限 workspace 内。

## 11. 验证计划

- Rust：prv 缓存键稳定/内容敏感、缓存命中不触发 LO、open_external 缺失报错测试。
- 前端：PrvToolbar pdf 本地打开按钮测试（扩展既有文件）。
- `npm run test:related -- --files [changed_files]`；`npm run governance:generate`；`npm run governance:audit`。
- 既有债务：`test_governance.test.mjs` 断言过期（ED 207/WS 28/PRV 1，HEAD 实际 208/30/3），
  沿用人类已批准授权（2026-08-05）同步修复，避免 test:related 反复受阻。

## 文件范围

### 允许修改

```
.governance/active_scope.json                          # 替换活动锁为本 Scope
.governance/issues/PRV-PREVIEW-OPT-01.md               # 本文
src-tauri/src/prv/mod.rs                               # render_preview 缓存落盘 + open_external
src-tauri/src/lib.rs                                   # route_preview 返回 abs_path + 新路由 open_external
src/components/prv/PrvPanel.tsx                        # pdf iframe 加载 asset URL；去 blob revoke
src/components/prv/PrvToolbar.tsx                      # pdf 模式新增本地打开按钮
src/components/prv/PrvToolbar.test.tsx                 # 本地打开按钮断言扩展
src/services/prvService.ts                             # previewDoc 返回 asset URL；openExternal
src/styles.css                                         # prv-body 背景统一
docs/01_modules/PRV/A-PRV-M-T-01_预览输出技术设计.md     # §2 缓存、§5 状态机豁免复核、§9 债务、变更记录
docs/01_modules/PRV/A-PRV-I-P-01_预览输出接口协议.md     # prv.preview 返回形态、prv.open_external 登记
docs/00_core/A-CORE-C-R-01_ADUS.md                     # governance:generate 产物
scripts/test_governance.test.mjs                       # 过期断言修复（人类已批准）
```

### 禁止修改

```
ADU.md / ADP.md / AGENTS.md / CLAUDE.md / governance.config.json
scripts/test_governance.mjs / scripts/generate_adus.mjs / scripts/audit_taskflow_governance.mjs
src-tauri/src/doc/lo_worker.rs / libreoffice_runtime.rs / font_trace.rs / tool_runtime.rs  # LO 链不动
src-tauri/resources/libreoffice/**                      # LO 二进制/字体/FontPlan/manifest 不动
patches/libreoffice/**                                  # 字体补丁不动
```

## 状态

confirmation: YES（人类确认方案，2026-08-05）
