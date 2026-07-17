//! PI-ENT-02 · 用量与订阅状态呈现（服务端唯一数据源）· CLOUD-GATE / BR-ENT-DATA-002
//!
//! 承接 A-ENT-M-T-01：用量与订阅状态呈现的数据源**唯一为服务端**（LIC 经网关下发）；客户端可本地缓存
//! 展示数据，但**不得将本地缓存/自行计量用作任何门禁或调用决策依据**（门禁决策归 BR-ENT-VERIFY-001 服务端
//! 裁决）；购买/升级/续费一律跳转官网门户，**桌面端不内置支付流程**；订阅状态变更以服务端裁决为准，即时
//! 生效于下一次门禁。

/// 用量数据来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageSource {
    Server,     // 服务端下发（唯一权威）
    LocalCache, // 本地缓存（仅供展示）
}

/// 数据用途。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsagePurpose {
    Display,      // 展示
    GateDecision, // 门禁/调用决策
}

/// 用量快照（展示用；不参与门禁决策）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsageSnapshot {
    pub source: UsageSource,
    pub used: u64,
    pub quota: u64,
}

/// 桌面端是否内置支付流程——**恒为 false**（购买跳官网门户）。
pub const DESKTOP_INLINE_PAYMENT: bool = false;

/**
 * @GOV
 * codes:    BR-ENT-DATA-002-GUARD-ENT-CLOUD-GATE-001
 * type:     GUARD
 * chain:    CLOUD-GATE
 * rules:    BR-ENT-DATA-002
 * boundary: in=usageSnapshotAndPurpose | out=admissibility(displayOnlyNeverGateFromLocalOrUsage)
 */
/// 用量数据可用性守卫（BR-ENT-DATA-002）：用量数据**仅可用于展示**；用于**门禁/调用决策一律不可**
/// （无论来源 Server 或 LocalCache——门禁决策须经服务端 verdict，不据用量数字/本地缓存）。返回是否准入。
pub fn usage_admissible_for(_snapshot: &UsageSnapshot, purpose: UsagePurpose) -> bool {
    match purpose {
        UsagePurpose::Display => true, // 可展示（含本地缓存）
        // 门禁/调用决策一律不得用用量数据或本地缓存（须服务端裁决）。
        UsagePurpose::GateDecision => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(src: UsageSource) -> UsageSnapshot {
        UsageSnapshot {
            source: src,
            used: 10,
            quota: 100,
        }
    }

    // covers: BR-ENT-DATA-002
    // 用量数据仅供展示；门禁决策一律不得用用量/本地缓存（无论来源）；桌面端不内置支付。
    #[test]
    fn usage_is_display_only_never_for_gating() {
        // 展示可用（含本地缓存）
        assert!(usage_admissible_for(
            &snap(UsageSource::Server),
            UsagePurpose::Display
        ));
        assert!(usage_admissible_for(
            &snap(UsageSource::LocalCache),
            UsagePurpose::Display
        ));
        // 门禁决策一律不可——即便服务端来源也不得据用量数字自行放行（须 verdict）
        assert!(!usage_admissible_for(
            &snap(UsageSource::Server),
            UsagePurpose::GateDecision
        ));
        assert!(!usage_admissible_for(
            &snap(UsageSource::LocalCache),
            UsagePurpose::GateDecision
        ));
        // 桌面端不内置支付流程（购买跳官网门户）
        assert!(!DESKTOP_INLINE_PAYMENT);
    }
}
