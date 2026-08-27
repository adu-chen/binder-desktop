//! 从应用组合根迁入的模块运行时/路由；保持原 command 与状态机行为等价。

use crate::*;

/// PI-BYOK-DEV-01 · BYOK 客户端入口：用户 provider 密钥一次性上送 dev 网关代持（零残留，客户端不持久化，
/// X-FORBID-001）。key 经 ent::byok 上送后即清零；不回显明文。
#[tauri::command]
pub(crate) async fn byok_upload(provider: String, key: String) -> Result<(), IpcError> {
    if key.trim().is_empty() {
        return Err(IpcError::new("ENT_BYOK_EMPTY", "密钥为空", false, false));
    }
    ent::byok::upload_to_gateway(&provider, key).await
}

/// MODEL-CONFIG-01 · 预设模型清单（TERM-AG-010 / AG-MODEL-CATALOG）：桌面端仅消费标识符，不承载配置权威源。
#[tauri::command]
pub(crate) async fn list_models() -> Result<Vec<ent::gateway::PresetModel>, IpcError> {
    ent::gateway::fetch_models().await
}

/// MODEL-CONFIG-01 · BYOK 已配置 provider 列表（脱敏 preview，不回显明文）。
#[tauri::command]
pub(crate) async fn byok_list_providers() -> Result<Vec<serde_json::Value>, IpcError> {
    ent::byok::list_providers().await
}

/// MODEL-CONFIG-01 · 删除指定 provider 的 BYOK 密钥（立即生效，A-GW-M-D-01 PI-GW-02）。
#[tauri::command]
pub(crate) async fn byok_delete_key(provider: String) -> Result<(), IpcError> {
    ent::byok::delete_key(&provider).await
}
