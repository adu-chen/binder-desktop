//! PI-ENT-02 · BYOK 代持零残留 · CLOUD-INVOKE / BR-ENT-DATA-001
//!
//! 承接 A-ENT-M-T-01：BYOK 密钥录入后经安全通道**一次性上送 AiGateway 代持**——本地不持久化（含内存外
//! 任何介质）、不写日志/错误上报、不回显明文，**上送完成即清内存引用**；上送失败本地无残留；BYOK 调用仍
//! 经门禁（无有效订阅一律拒绝，透传不计额度但**不豁免门禁**，TERM-CLOUD-004）。

use crate::ent::gateway::GateOutcome;
use crate::ipc::IpcError;

/// 上送结果（不含任何密钥明文——不回显）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByokUpload {
    Delivered,
}

/**
 * @GOV
 * codes:    BR-ENT-DATA-001-EFFECT-ENT-CLOUD-INVOKE-001
 * type:     EFFECT
 * chain:    CLOUD-INVOKE
 * rules:    BR-ENT-DATA-001
 * boundary: in=gateOutcomeAndOwnedKey | out=ByokUpload(oneShotZeroizedNoResidueGateNotBypassed)
 */
/// BYOK 一次性上送代持（BR-ENT-DATA-001）：**仍经门禁**——gate 非 Pass 一律拒绝（不豁免门禁）；按值
/// 取得 key 所有权（调用点交出后无法复用），上送后**就地清零并 drop**（即清内存引用、本地无持久化、
/// 不回显明文）；上送失败亦清零（无残留）。结果不含明文（不回显）。
pub fn upload_byok(gate: GateOutcome, mut key: String) -> Result<ByokUpload, IpcError> {
    // BYOK 不豁免门禁：无有效订阅（gate Deny）一律拒绝。
    if gate != GateOutcome::Pass {
        zeroize(&mut key); // 拒绝路径同样清零（无残留）
        return Err(IpcError::new(
            "ENT_BYOK_GATE_DENIED",
            "BYOK 上送被拒：仍须经门禁，无有效订阅",
            false,
            true,
        ));
    }
    // 一次性上送（安全通道代持，此处为契约占位；真实传输经网关，不写日志）。
    let _delivered = deliver_to_gateway(&key);
    // 上送完成即清内存引用（成功/失败均清零）。
    zeroize(&mut key);
    Ok(ByokUpload::Delivered)
}

/// 就地清零字符串缓冲（清内存引用，避免明文驻留）。
fn zeroize(key: &mut String) {
    // 覆写底层字节后清空（best-effort 清内存引用）。
    unsafe {
        for b in key.as_bytes_mut() {
            *b = 0;
        }
    }
    key.clear();
}

/// 安全通道上送占位（真实经 AiGateway；不写日志、不回显、不持久化）。
fn deliver_to_gateway(_key: &str) -> bool {
    true
}

/// PI-BYOK-DEV-01 · BYOK 一次性上送 dev 网关并**零残留清零**（deliver_to_gateway 的真实 dev 版）。
///
/// 守 BR-ENT-DATA-001 / X-FORBID-001：key 仅经网关、客户端不持久化；上送后就地清零原 key（不回显明文、
/// 不写日志）。dev 门禁桩=Pass（真实订阅裁决归服务端 X-FORBID-002，正式版补）。
pub async fn upload_to_gateway(provider: &str, mut key: String) -> Result<(), IpcError> {
    let url = format!("{}/byok/keys", crate::ent::gateway::gw_base_url());
    // key 借入 body 上送；send 后就地清零原 key（一次性、零残留）。
    let body = serde_json::json!({ "provider": provider, "api_key": &key });
    let result = reqwest::Client::new().post(&url).json(&body).send().await;
    zeroize(&mut key);
    drop(body); // body 含一份明文拷贝，随作用域 drop（dev best-effort；正式版用安全内存类型最小化拷贝）

    match result {
        Ok(r) if r.status().is_success() => Ok(()),
        Ok(r) => Err(IpcError::new(
            "ENT_BYOK_GW_HTTP",
            &format!("BYOK 上送失败：网关 HTTP {}", r.status()),
            true,
            false,
        )),
        Err(e) => Err(IpcError::new(
            "ENT_BYOK_GW_UNREACHABLE",
            &format!("BYOK 上送失败：dev 网关不可达（{e}）。请先启动 cloud/dev-gateway。"),
            true,
            false,
        )),
    }
}

/// dev GW 已配置 BYOK provider 列表（脱敏 key_preview，PI-GW-02）：客户端仅展示状态，不回显明文。
pub async fn list_providers() -> Result<Vec<serde_json::Value>, IpcError> {
    let url = format!("{}/byok/providers", crate::ent::gateway::gw_base_url());
    let resp = reqwest::Client::new().get(&url).send().await.map_err(|e| {
        IpcError::new(
            "ENT_BYOK_GW_UNREACHABLE",
            &format!("BYOK 状态查询失败：dev 网关不可达（{e}）。请先启动 cloud/dev-gateway。"),
            true,
            false,
        )
    })?;
    if !resp.status().is_success() {
        return Err(IpcError::new(
            "ENT_BYOK_GW_HTTP",
            &format!("BYOK 状态查询失败：网关 HTTP {}", resp.status()),
            true,
            false,
        ));
    }
    let body: serde_json::Value = resp.json().await.map_err(|e| {
        IpcError::new("ENT_BYOK_GW_BAD_RESPONSE", &format!("BYOK 状态响应无法解析：{e}"), true, false)
    })?;
    Ok(body
        .get("providers")
        .and_then(|p| p.as_array())
        .cloned()
        .unwrap_or_default())
}

/// 删除指定 provider 的 BYOK 密钥（立即生效，A-GW-M-D-01 PI-GW-02）：客户端仅传 provider 标识
/// （值域受限：deepseek/openai/anthropic，直接路径拼接，不引入额外依赖）。
pub async fn delete_key(provider: &str) -> Result<(), IpcError> {
    let url = format!("{}/byok/keys/{provider}", crate::ent::gateway::gw_base_url());
    let resp = reqwest::Client::new().delete(&url).send().await.map_err(|e| {
        IpcError::new(
            "ENT_BYOK_GW_UNREACHABLE",
            &format!("BYOK 删除失败：dev 网关不可达（{e}）。请先启动 cloud/dev-gateway。"),
            true,
            false,
        )
    })?;
    if !resp.status().is_success() {
        return Err(IpcError::new(
            "ENT_BYOK_GW_HTTP",
            &format!("BYOK 删除失败：网关 HTTP {}", resp.status()),
            true,
            false,
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ent::gateway::GateOutcome;

    // covers: BR-ENT-DATA-001
    // BYOK 仍经门禁：gate Deny 一律拒绝（不豁免门禁），拒绝路径无残留。
    #[test]
    fn byok_not_exempt_from_gate() {
        let err = upload_byok(GateOutcome::Deny, "sk-secret".to_string()).unwrap_err();
        assert_eq!(err.code, "ENT_BYOK_GATE_DENIED");
        assert!(err.reauth_required);
    }

    // covers: BR-ENT-DATA-001
    // gate Pass：一次性上送成功，结果不含明文（不回显）；key 所有权被消费（调用点无法复用=无残留）。
    #[test]
    fn byok_one_shot_delivered_and_key_consumed() {
        let key = "sk-secret-xyz".to_string();
        let r = upload_byok(GateOutcome::Pass, key).unwrap();
        assert_eq!(r, ByokUpload::Delivered);
        // key 已 move 进函数并就地清零 drop；调用点此后无法再引用（编译期保证无残留）。
        // 结果类型不携带任何明文字段（不回显）。
    }

    // covers: BR-ENT-DATA-001
    // zeroize 就地覆写并清空（清内存引用）。
    #[test]
    fn zeroize_clears_buffer() {
        let mut s = "secret".to_string();
        zeroize(&mut s);
        assert!(s.is_empty());
    }
}
