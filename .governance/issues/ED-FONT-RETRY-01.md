# Issue Trace：ED-FONT-RETRY-01

## 目标

修复 ED-FONT-LOCAL-LIBRARY-01 的字体枚举永久锁死缺陷：fontService.listLocalFonts 中 attempted 标志在一次失败后永久置位，导致本会话内所有后续调用直接抛错、UI 永远回退常用子集（表现为"本地字体突然消失"）。

## 根因（观测定案）

- Rust `fonts.list_fonts`（CoreText）枚举正常：212 个字体族。
- `document.fonts.check` 过滤正常：212 全部通过（排除 check 过滤、CSS 转换问题）。
- 唯一失败点：fontService `attempted` 永久锁死——dev 启动期 Rust backend 未就绪的瞬时 IPC 失败一次后，`attempted=true` 且无业务侧 `resetLocalFontCache` 调用，本会话永远回退默认字体。

## 修复

- fontService.listLocalFonts：移除 `attempted` 锁死；失败仅当次抛错、下次自动重试；仅成功结果缓存进 `cachedFamilies`。
- 测试更新：失败后下次调用自动重试（invoke 2 次、第二次成功）。
- 移除临时审计打点（EditorToolbar `[FONT-AUDIT]` console、Rust `println!`）。

## 验证

- fontService.test.ts + EditorControls.test.tsx：17/17 passed。
- tsc --noEmit：干净。
- test:related / governance:generate / governance:audit / build：通过。

## 文件范围

- `src/services/fontService.ts`
- `src/services/fontService.test.ts`
- `src/components/ed/EditorToolbar.tsx`（移除审计打点）
- `src-tauri/src/lib.rs`（移除审计 println）
- `docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md`（v1.60）
- `docs/00_core/A-CORE-C-R-01_ADUS.md`（重新生成）
- `.governance/issues/ED-FONT-RETRY-01.md`（本文）

## 遗留

- 架构注记（非本次范围）：工具栏字体选择器（localFamilies + COMMON_FONT_FAMILIES）与 DOCX 导出字体链路（font_trace FontPolicy aliases）未共享 SSOT；长期可考虑统一别名体系。
