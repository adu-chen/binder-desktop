//! PI-AG-08A · 工具注册表（4 工具 + ReadOnly/WriteIntent 权限分级）。
//!
//! 为 SM-AG-02 `tool::validate_tool` 提供「注册清单 + 权限边界」数据（BR-AG-STATE-002）：模型请求的工具
//! 必须在注册清单内且通过权限校验方可执行。清单 v1 为 4 工具，**延实现期运营扩展**。
//!
//! **执行实现**（read_doc/list_docs/search_text 调 WS/KB；propose_edit 产出 ChangeProposal→DE）归
//! PI-AG-08B（与模型循环 PI-AG-11 配套，避免重复 lib.rs 既有 route_*）。本模块只持有注册与权限数据。

use super::tool::{validate_tool, ToolDecision};

/// 工具权限分级。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolPermission {
    /// 只读：读文档/列文档/搜索，经 WS/KB 且限 active workspace（X-FORBID-004）。
    ReadOnly,
    /// 写意图：产出修改建议走 DE diff（X-FORBID-006）；**非直接写盘**。
    WriteIntent,
}

/// 工具定义（注册项）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolDef {
    pub name: &'static str,
    pub permission: ToolPermission,
}

/// v1 工具集（4 个，A-AG-X-L-02 PI-AG-08）。
///
/// propose_edit 标记 WriteIntent 表示其**执行**走 DE 修改建议路径，但它本身不直写盘
/// （`direct_write_intent=false`）——试图绕过 DE 直写的工具才会被 validate_tool 以写盘禁令拒绝。
pub const TOOLS: &[ToolDef] = &[
    ToolDef { name: "read_doc", permission: ToolPermission::ReadOnly },
    ToolDef { name: "list_docs", permission: ToolPermission::ReadOnly },
    ToolDef { name: "search_text", permission: ToolPermission::ReadOnly },
    ToolDef { name: "propose_edit", permission: ToolPermission::WriteIntent },
];

/// 按名查注册项。
pub fn lookup(name: &str) -> Option<ToolDef> {
    TOOLS.iter().copied().find(|t| t.name == name)
}

/// 是否在注册清单内。
pub fn is_registered(name: &str) -> bool {
    lookup(name).is_some()
}

/// 校验模型请求的工具（承 BR-AG-STATE-002 `validate_tool`）。
///
/// v1 策略：注册即在权限边界内；4 工具均 `direct_write_intent=false`——只读工具读 WS/KB，propose_edit
/// 经 ChangeProposal→DE 产出修改建议（非直写）。未注册 → Reject（含工具名，回注模型）。
/// 细分权限场景（如某些只读工具的范围限制）属实现期运营策略，本函数锁「注册 + 无直写」校验机制。
pub fn validate_requested_tool(name: &str) -> ToolDecision {
    let registered = is_registered(name);
    validate_tool(registered, registered, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_tools_registered_with_expected_permissions() {
        assert_eq!(TOOLS.len(), 4);
        assert!(is_registered("read_doc"));
        assert!(is_registered("list_docs"));
        assert!(is_registered("search_text"));
        assert!(is_registered("propose_edit"));
        assert!(!is_registered("delete_file"));
        assert_eq!(lookup("read_doc").unwrap().permission, ToolPermission::ReadOnly);
        assert_eq!(lookup("search_text").unwrap().permission, ToolPermission::ReadOnly);
        assert_eq!(
            lookup("propose_edit").unwrap().permission,
            ToolPermission::WriteIntent
        );
    }

    #[test]
    fn unregistered_tool_is_rejected_with_name() {
        match validate_requested_tool("rm_rf") {
            ToolDecision::Reject(r) => assert!(r.contains("未在注册清单")),
            ToolDecision::Allow => panic!("未注册工具应被拒绝"),
        }
    }

    #[test]
    fn all_four_registered_tools_pass_validation_including_propose_edit() {
        // 4 工具均合法放行：只读 3 个 + propose_edit（产出修改建议，非直写，合法）。
        for t in ["read_doc", "list_docs", "search_text", "propose_edit"] {
            assert_eq!(validate_requested_tool(t), ToolDecision::Allow, "{t} 应放行");
        }
    }
}
