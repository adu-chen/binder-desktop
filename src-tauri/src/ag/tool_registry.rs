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
    ToolDef {
        name: "read_doc",
        permission: ToolPermission::ReadOnly,
    },
    ToolDef {
        name: "list_docs",
        permission: ToolPermission::ReadOnly,
    },
    ToolDef {
        name: "search_text",
        permission: ToolPermission::ReadOnly,
    },
    ToolDef {
        name: "propose_edit",
        permission: ToolPermission::WriteIntent,
    },
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

/// 发送给模型的工具定义（OpenAI function-calling 格式）。
/// dev：3 个只读工具 + propose_edit 写意图工具；propose_edit 走 ChangeProposal→DE，不直接写盘。
pub fn tool_definitions_json() -> serde_json::Value {
    serde_json::json!([
        {
            "type": "function",
            "function": {
                "name": "list_docs",
                "description": "列出当前工作区（workspace）内的所有文档（相对路径与类型）。需要了解工作区有哪些文件时调用。",
                "parameters": { "type": "object", "properties": {} }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "read_doc",
                "description": "读取当前工作区内指定相对路径文档的完整内容。注意：当前编辑器打开的文档已在上下文提供，无需用本工具重复读取。",
                "parameters": {
                    "type": "object",
                    "properties": { "path": { "type": "string", "description": "文档相对路径（相对 workspace 根）" } },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "search_text",
                "description": "在当前工作区内按关键词搜索文档内容，返回命中的文件与片段。",
                "parameters": {
                    "type": "object",
                    "properties": { "query": { "type": "string", "description": "搜索关键词" } },
                    "required": ["query"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "propose_edit",
                "description": "对文档提出修改建议（不直接写盘，生成待用户确认的修改卡片，用户接受后才应用）。改某一块（优先）：传 path + new_text，并指定块——块列表里带 [id=…] 的块优先用 block_id（稳定锚），否则用 block_index（块号）。改写整篇：传 path + content。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "目标文档相对路径（通常是当前打开的文档）" },
                        "block_id": { "type": "string", "description": "要修改块的稳定锚（块列表 [id=…] 的值）；HTML 文档优先用此" },
                        "block_index": { "type": "integer", "description": "要修改的块号（块列表 Block N）；无 block_id 时用" },
                        "new_text": { "type": "string", "description": "该块修改后的新内容；块模式必填" },
                        "content": { "type": "string", "description": "整篇模式：修改后的完整文档内容" }
                    },
                    "required": ["path"]
                }
            }
        }
    ])
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
        assert_eq!(
            lookup("read_doc").unwrap().permission,
            ToolPermission::ReadOnly
        );
        assert_eq!(
            lookup("search_text").unwrap().permission,
            ToolPermission::ReadOnly
        );
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
            assert_eq!(
                validate_requested_tool(t),
                ToolDecision::Allow,
                "{t} 应放行"
            );
        }
    }
}
