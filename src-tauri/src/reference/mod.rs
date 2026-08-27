//! PI-REF-* · @mention 触发解析/锚校验/注入组装 · REF-INJECT（无状态机豁免）
//!
//! G4 已落（全 3 规则）：PI-REF-01 resolve（@mention→引用锚，限 workspace/不可达失败不悬空，
//! BR-REF-DATA-001）、PI-REF-02 inject（锚校验保守不静默注入 BR-REF-VERIFY-001 + 读取时点载荷可溯源
//! BR-REF-DATA-002）。承接 L2 见 A-REF-M-T-01 / A-REF-X-L-01。状态机豁免（请求-响应式）。

/// PI-REF-01 · @mention 触发与解析（BR-REF-DATA-001）。
pub mod resolve;

/// PI-REF-02 · 锚校验与注入载荷组装（BR-REF-VERIFY-001 / BR-REF-DATA-002）。
pub mod inject;

/// REF-REFERENCE-REPAIR-01 F3/F4/F6 · 注入上下文格式化与选区锚校验（BR-REF-DATA-003 / VERIFY-001）。
/// 纯函数层：doc/selection 读取由 AG 完成后传入，REF 不反向依赖 AG。
pub mod context;

/// Tauri command 适配层。
pub mod routes;
