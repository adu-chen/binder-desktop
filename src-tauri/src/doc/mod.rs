//! PI-DOC-* · 编辑链Pandoc/预览链LibreOffice/样式子集 · DOC-CONVERT / X-SEQ-004
//!
//! G4 已落（后端纯逻辑/守卫，全 4 规则）：PI-DOC-02 style_subset（样式三级子集 + 损失告知，
//! BR-DOC-DATA-002）、PI-DOC-01 convert（SM-DOC-01 转换生命周期 BR-DOC-STATE-001 + 双链分离/依赖守卫
//! BR-DOC-DATA-001）、PI-DOC-03 block_backfill（blockId 兜底 + 往返保留度评估 BR-DOC-DATA-003）。
//! Pandoc/LibreOffice 实际子进程调用与 Lua 过滤器为实现期（A 类）。承接 L2 见 A-DOC-M-T-01 / A-DOC-M-S-01。

/// PI-DOC-02 · 样式子集三级执行契约（BR-DOC-DATA-002）。
pub mod style_subset;
pub mod table_preview;

/// PI-DOC-01 · 转换生命周期 SM-DOC-01 + 管线边界（BR-DOC-STATE-001 / BR-DOC-DATA-001）。
pub mod convert;

/// PI-DOC-03 · 转换层 blockId 兜底（BR-DOC-DATA-003）。
pub mod block_backfill;

/// PI-DOC-01（实现期续接）· Pandoc 编辑链运行时（真实 docx↔markdown 子进程，BR-DOC-DATA-001）。
pub mod pandoc_runtime;

/// PI-DOC-01（实现期续接）· docx 编辑链 docx↔HTML（移植旧项目处理逻辑：docx+styles + preserve-styles.lua
/// + reference.docx，保留样式子集供 TipTap，BR-DOC-DATA-002）。
pub mod edit_chain;

/// PI-DOC-01/03（实现期续接）· LibreOffice 预览链运行时（docx/xlsx/pptx→PDF；移植旧项目处理逻辑，
/// BR-DOC-DATA-001）。
pub mod libreoffice_runtime;
