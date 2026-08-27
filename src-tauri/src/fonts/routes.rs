//! 本地字体库的 Tauri command 适配层。

use crate::{fonts, ipc::IpcError};

/// 路由 `fonts.list_fonts`（ED-FONT-LOCAL-LIBRARY-01）：只读枚举本地已注册字体族名。
/// 仅返回字体族名数组，不读字体文件内容；失败回错误形状（前端回退常用子集，不阻塞编辑）。
pub(crate) fn route_list_fonts() -> Result<serde_json::Value, IpcError> {
    let families = fonts::list_local_font_families()?;
    Ok(serde_json::json!({ "families": families }))
}
