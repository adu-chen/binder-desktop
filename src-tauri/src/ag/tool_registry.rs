//! PI-AG-08A · 工具注册表（只读工具 + 结构操作 + WriteIntent 权限分级）。
//!
//! 为 SM-AG-02 `tool::validate_tool` 提供「注册清单 + 权限边界」数据（BR-AG-STATE-002）：模型请求的工具
//! 必须在注册清单内且通过权限校验方可执行。清单 v1 覆盖文档列表/读取/搜索/块读取 + 非破坏性结构操作 + 修改建议。
//!
//! **执行实现**（read_doc/list_docs/search_text/list_blocks 调统一 AgentEditableSnapshot；
//! move_doc/create_folder 经 WS 边界执行；propose_edit 产出 ChangeProposal→DE）归
//! PI-AG-08B（与模型循环 PI-AG-11 配套，避免重复 lib.rs 既有 route_*）。本模块只持有注册与权限数据。

use super::tool::{validate_tool, ToolDecision};

/// 工具权限分级。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolPermission {
    /// 只读：读文档/列文档/搜索，经 WS/KB 且限 active workspace（X-FORBID-004）。
    ReadOnly,
    /// 结构操作：移动/重命名/建目录，经 WS 边界执行；不修改文档内容，不走 DE。
    StructureWrite,
    /// 写意图：产出修改建议走 DE diff（X-FORBID-006）；**非直接写盘**。
    WriteIntent,
}

/// 工具定义（注册项）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolDef {
    pub name: &'static str,
    pub permission: ToolPermission,
}

/// v1 工具集（A-AG-X-L-02 PI-AG-08）。
///
/// propose_edit 标记 WriteIntent 表示其**执行**走 DE 修改建议路径，但它本身不直写盘
/// （`direct_write_intent=false`）；move_doc/create_folder 是文件树结构操作，不修改文档内容。
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
        name: "list_blocks",
        permission: ToolPermission::ReadOnly,
    },
    ToolDef {
        name: "move_doc",
        permission: ToolPermission::StructureWrite,
    },
    ToolDef {
        name: "create_folder",
        permission: ToolPermission::StructureWrite,
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
/// v1 策略：注册即在权限边界内；只读工具读 WS/DOC 编辑模型，结构工具经 WS 边界执行，propose_edit
/// 经 ChangeProposal→DE 产出修改建议（非直写文档内容）。未注册 → Reject（含工具名，回注模型）。
/// 细分权限场景（如某些只读工具的范围限制）属实现期运营策略，本函数锁「注册 + 无直写」校验机制。
pub fn validate_requested_tool(name: &str) -> ToolDecision {
    let registered = is_registered(name);
    validate_tool(registered, registered, false)
}

/// 发送给模型的工具定义（OpenAI function-calling 格式）。
/// dev：只读工具 + 结构工具 + propose_edit 写意图工具；propose_edit 走 ChangeProposal→DE，不直接写文档内容。
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
                "name": "move_doc",
                "description": "移动或重命名当前 workspace 内的文件。用于按内容分类整理文件、改名、移动到子目录。只改变文件树结构，不修改文档内容，不生成 DiffCard；目标已存在会失败，不覆盖。不得用于删除。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "from_path": { "type": "string", "description": "源文件相对路径（相对 workspace 根）" },
                        "to_path": { "type": "string", "description": "目标文件相对路径（相对 workspace 根）" }
                    },
                    "required": ["from_path", "to_path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "create_folder",
                "description": "在当前 workspace 内创建目录。用于文件整理前建立分类目录。目标已是目录时视为成功；目标为文件时失败。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "目录相对路径（相对 workspace 根）" }
                    },
                    "required": ["path"]
                }
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
                "description": "在当前工作区内按关键词搜索文档内容，返回命中的文件与片段。b-docx 会按 DOC 编辑模型转 HTML 后搜索块文本。",
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
                "name": "list_blocks",
                "description": "读取指定文档的块列表，返回 block_id/block_index/text/marks/block_attrs。BDocx 使用 DOC 编辑模型，原生 docx 会拒绝。",
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
                "name": "propose_edit",
                "description": "对文档提出修改建议（不直接写盘，生成待用户确认的修改卡片，用户接受后才应用）。局部编辑必须传 operation=\"replace_block\" + path + new_text，并指定 block_id（优先，块列表 [id=…] 的稳定锚）或 block_index。整篇替换仅当用户明确要求整篇重写时传 operation=\"replace_document\" + content。只改样式传 operation=\"style_ops\" + style_ops（StyleOp 数组）。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "目标文档相对路径（通常是当前打开的文档）" },
                        "operation": {
                            "type": "string",
                            "enum": ["replace_block", "replace_document", "style_ops"],
                            "description": "修改类型：replace_block=精准块替换（默认用于已有文档局部编辑）；replace_document=整篇替换（仅用户明确要求整篇重写）；style_ops=只改样式"
                        },
                        "block_id": { "type": "string", "description": "要修改块的稳定锚（块列表 [id=…] 的值）；HTML 文档优先用此" },
                        "block_index": { "type": "integer", "description": "要修改的块号（块列表 Block N）；无 block_id 时用" },
                        "new_text": { "type": "string", "description": "该块修改后的新内容；operation=replace_block 必填" },
                        "content": { "type": "string", "description": "修改后的完整文档内容；仅 operation=replace_document 使用" },
                        "style_ops": {
                            "type": "array",
                            "description": "样式修改操作序列（StyleOp）：set_mark/unset_mark/set_block_attr；range 为块内纯文本偏移。样式修改必须经此字段进入 DiffCard 裁决。",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "op": { "type": "string", "enum": ["set_mark", "unset_mark", "set_block_attr"] },
                                    "blockId": { "type": "string", "description": "目标块 data-block-id" },
                                    "start": { "type": "integer", "description": "块内纯文本起始偏移，set_mark/unset_mark 使用" },
                                    "end": { "type": "integer", "description": "块内纯文本结束偏移，set_mark/unset_mark 使用" },
                                    "mark": {
                                        "type": "object",
                                        "description": "set_mark 使用，如 {\"type\":\"bold\"} 或 {\"type\":\"textStyle\",\"attrs\":{\"color\":\"#ff0000\"}}"
                                    },
                                    "markType": { "type": "string", "description": "unset_mark 使用，如 bold/italic/textStyle" },
                                    "attrs": { "type": "object", "description": "set_block_attr 使用，如 {\"textAlign\":\"center\"}" }
                                },
                                "required": ["op", "blockId"]
                            }
                        }
                    },
                    "required": ["path", "operation"]
                }
            }
        }
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_registered_with_expected_permissions() {
        assert_eq!(TOOLS.len(), 7);
        assert!(is_registered("read_doc"));
        assert!(is_registered("list_docs"));
        assert!(is_registered("search_text"));
        assert!(is_registered("list_blocks"));
        assert!(is_registered("move_doc"));
        assert!(is_registered("create_folder"));
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
            lookup("list_blocks").unwrap().permission,
            ToolPermission::ReadOnly
        );
        assert_eq!(
            lookup("move_doc").unwrap().permission,
            ToolPermission::StructureWrite
        );
        assert_eq!(
            lookup("create_folder").unwrap().permission,
            ToolPermission::StructureWrite
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
    fn all_registered_tools_pass_validation_including_propose_edit() {
        // 工具均合法放行：只读工具 + 结构操作 + propose_edit（产出修改建议，非直写文档内容，合法）。
        for t in [
            "read_doc",
            "list_docs",
            "search_text",
            "list_blocks",
            "move_doc",
            "create_folder",
            "propose_edit",
        ] {
            assert_eq!(
                validate_requested_tool(t),
                ToolDecision::Allow,
                "{t} 应放行"
            );
        }
    }

    #[test]
    fn propose_edit_schema_exposes_style_ops() {
        let defs = tool_definitions_json();
        let propose = defs
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["function"]["name"] == "propose_edit")
            .unwrap();
        assert!(propose["function"]["parameters"]["properties"]["operation"].is_object());
        assert!(propose["function"]["parameters"]["required"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("operation")));
        assert!(propose["function"]["parameters"]["properties"]["style_ops"].is_object());
        assert!(propose["function"]["description"]
            .as_str()
            .unwrap()
            .contains("style_ops"));
    }

    #[test]
    fn structure_tool_schema_exposes_move_and_create_folder() {
        let defs = tool_definitions_json();
        let arr = defs.as_array().unwrap();
        let move_doc = arr
            .iter()
            .find(|d| d["function"]["name"] == "move_doc")
            .unwrap();
        assert!(move_doc["function"]["parameters"]["properties"]["from_path"].is_object());
        assert!(move_doc["function"]["parameters"]["properties"]["to_path"].is_object());
        let create_folder = arr
            .iter()
            .find(|d| d["function"]["name"] == "create_folder")
            .unwrap();
        assert!(create_folder["function"]["parameters"]["properties"]["path"].is_object());
        assert!(!arr.iter().any(|d| d["function"]["name"] == "delete_doc"));
    }
}
