# Issue Trace — Docx 代理编辑全链路审计修复

## 1. 用户可见症状

审计发现 docx 代理编辑全链路存在 44 个问题，其中 23 个需本版本修复（17 MUST-FIX + 6 SIMPLIFY），15 个延期，6 个忽略。症状包括：
- 保存时 Pandoc stdin 未显式关闭可能导致死锁（P0）
- LO 非超时失败时 stderr 完全丢失，无法诊断（P0）
- 图片在 docx 往返保存中路径可能断裂（P1）
- h1 标题每次打开被 preserve-styles.lua 强制居中（P1）
- 3 位 hex 颜色（如 #F00）不识别，输出无效 CSS（P1）
- block_backfill 手工 HTML 解析脆弱（P1）
- AG context 优先级与 BR-AG-DATA-005 P0 约束冲突（P1）
- diff apply 跨介质非原子操作（P1）
- bdocx_manifest 并发写不安全 + rename 不更新（P1）
- docx 打开/加载失败无用户可见错误（P1）
- font-family 被 Lua 保留但前端无扩展解析（P1）

## 2. 入口点

| 入口 | 文件 | 函数 |
|------|------|------|
| docx 保存 | src-tauri/src/doc/edit_chain.rs | html_to_docx |
| LO 预览 | src-tauri/src/doc/lo_worker.rs | LoWorker::execute |
| blockId 注入 | src-tauri/src/doc/block_backfill.rs | inject_missing_data_block_ids |
| AG 上下文 | src-tauri/src/ag/context.rs | assemble_context |
| diff 落盘 | src-tauri/src/workspace/diff_apply.rs | apply_diff |
| bdocx 注册 | src-tauri/src/workspace/listing.rs | mark_bdocx, rename_doc |
| 文档打开 | src/services/edService.ts | openDocument |
| 样式转换 | src-tauri/resources/preserve-styles.lua | Para, process_inline_formatting |

## 3. 相关规则

BR-DOC-DATA-002, BR-DOC-DATA-003, BR-DOC-STATE-001, BR-AG-DATA-005, BR-WS-PERSIST-001, BR-WS-DATA-001, BR-ED-DATA-001, BR-ED-STATE-003

## 4. 相关链路

DOC-CONVERT → ED-EDIT → AG-TURN → DE-DIFF → WS-DIFF-APPLY → WS-SAVE → WS-FS

## 5. 生产者块

- BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-003 (src-tauri/src/doc/edit_chain.rs) — docx_to_html
- BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-004 (src-tauri/src/doc/edit_chain.rs) — html_to_docx
- BR-DOC-DATA-003-IO-DOC-DOC-CONVERT-005 (src-tauri/src/doc/block_backfill.rs) — inject_missing_data_block_ids
- BR-WS-PERSIST-001 (src-tauri/src/workspace/diff_apply.rs) — apply_diff
- BR-WS-DATA-001 (src-tauri/src/workspace/listing.rs) — mark_bdocx

## 6. 消费者块

- src/services/edService.ts — openDocument 消费 docx 转换结果
- src/components/ed/Editor.tsx — 消费编辑内容渲染
- src/services/deService.ts — 消费 diff 块定位

## 7. 路径上的空规则块

多数前端 TypeScript 文件和 Lua 文件缺乏 @GOV 标注，不在本次 Scope 内补齐。

## 8. boundary 假设

- edit_chain.rs 假设 Pandoc 包内可用（tool_runtime 校验）
- diff_apply.rs 假设 rename 是原子的（POSIX）
- context.rs 假设 assemble_context 调用方正确传入 ContextItem

## 9. 运行时假设

- Pandoc 子进程在合理时间内完成（本次新增超时控制）
- LO worker 在 macOS 环境下正常运行
- workspace.db 的 Mutex 不会被 poisoned

## 10. 疑似不变量破坏

- html_to_docx stdin 未显式关闭 → 可能破坏 Pandoc 正常退出不变量
- diff_apply 跨介质非原子 → 可能破坏文件与 DB 一致不变量
- bdocx_manifest 并发写 → 可能破坏注册表与文件系统一致不变量
- h1 强制居中 → 破坏样式往返不变量

## 11. 验证计划

- `cargo test -p binder`（Rust 单元测试）
- `npm run governance:audit`
- 手动验证：docx 打开→编辑→保存往返
- 手动验证：h1 不再被强制居中
- 手动验证：docx 打开失败前端显示错误提示
- 更新债务台账记录 DEFER 项
