//! PI-ED-* · 编辑器后端会话与开放标签投影。
//!
//! 编辑器正文模型仍由前端 ED 受控接口持有；本模块仅承接持久化会话快照、开放标签和
//! 相关 Tauri command 适配，不直接操纵编辑器内部文档状态。

/// Tauri command 适配层。
pub mod routes;
