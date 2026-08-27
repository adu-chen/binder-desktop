//! PI-ENT-* · 门禁前置网关通道/BYOK零残留/用量 · BR-ENT-* / X-SEQ-001/X-FORBID-002
//!
//! G4 已落（全 4 规则）：PI-ENT-01 gateway（调用通道 SM + 门禁前置，BR-ENT-STATE-001/VERIFY-001）、
//! PI-ENT-02 byok（BYOK 一次性上送零残留不豁免门禁，BR-ENT-DATA-001）+ usage（用量服务端唯一数据源不
//! 参与门禁决策，BR-ENT-DATA-002）。承接 L2 规则源见 A-ENT-M-T-01 / A-ENT-X-L-01。

/// PI-ENT-01 · 门禁前置与网关调用通道（BR-ENT-STATE-001 / BR-ENT-VERIFY-001）。
pub mod gateway;

/// PI-ENT-02 · BYOK 代持零残留（BR-ENT-DATA-001）。
pub mod byok;

/// PI-ENT-02 · 用量与订阅状态呈现，服务端唯一数据源（BR-ENT-DATA-002）。
pub mod usage;

/// Tauri BYOK 与模型目录 commands。
pub mod commands;
