//! PI-AG-08A · 工具注册表（只读工具 + 结构操作 + WriteIntent 权限分级）。
//!
//! 为 SM-AG-02 `tool::validate_tool` 提供「注册清单 + 权限边界」数据（BR-AG-STATE-002）：模型请求的工具
//! 必须在注册清单内且通过权限校验方可执行。清单 v1 覆盖文档列表/读取/搜索/块读取 + 非破坏性结构操作 + 修改建议。
//!
//! **执行实现**（read_doc/list_docs/search_text/list_blocks 调统一 AgentEditableSnapshot；
//! move_doc/create_folder 经 WS 边界执行；import_image_asset 经 DOC/WS 受控复制；
//! edit_blocked 结构化结束不可安全执行的编辑请求；propose_edit 产出 ChangeProposal→DE）归
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
/// （`direct_write_intent=false`）；move_doc/create_folder/import_image_asset 是文件树结构操作，
/// 不直接修改文档内容。
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
        name: "list_tables",
        permission: ToolPermission::ReadOnly,
    },
    ToolDef {
        name: "list_images",
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
        name: "import_image_asset",
        permission: ToolPermission::StructureWrite,
    },
    ToolDef {
        name: "edit_blocked",
        permission: ToolPermission::WriteIntent,
    },
    ToolDef {
        name: "propose_edit",
        permission: ToolPermission::WriteIntent,
    },
    ToolDef {
        name: "create_doc",
        permission: ToolPermission::StructureWrite,
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
pub fn tool_definitions_json(
) -> Result<serde_json::Value, super::bdocx_capabilities::CapabilityValidationError> {
    let propose_edit_parameters = super::bdocx_capabilities::propose_edit_parameters_schema()?;
    Ok(serde_json::json!([
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
                "name": "import_image_asset",
                "description": "把 workspace 内尚未位于受控 imageRoots 的 PNG/JPEG 复制到项目图片导入目录，返回 assetId/sourcePath/sourceHash。源文件保持不变。需要插入普通项目图片但 list_images 未暴露该文件时调用，随后重新调用 list_images 并用 image_ops 提案。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "source_path": { "type": "string", "description": "待复制图片的 workspace 相对路径；不得传绝对或越界路径" }
                    },
                    "required": ["source_path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "edit_blocked",
                "description": "编辑请求确实无法形成安全修改建议时，结构化报告阻断原因。仅用于缺少资产、锚点或动态能力等真实阻断；调用后可诚实结束回合，不得伪称文档已修改。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "code": { "type": "string", "description": "稳定、简短的阻断代码" },
                        "reason": { "type": "string", "description": "面向用户的具体阻断原因与下一步" }
                    },
                    "required": ["code", "reason"]
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
                "description": "读取指定文档的块列表。当前富文档直接返回 ProseMirror 层级投影：block_id/block_index/block_type/node_type/role/parent_block_id/depth/text/marks/block_attrs/node_json/fingerprint/capabilities；只有 capabilities 含 replace_node 的 content 块可用于 block_ops，容器与原子块只读。BDocx 使用 DOC 编辑模型，原生 docx 会拒绝。",
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
                "name": "list_tables",
                "description": "读取当前活动 BDocx 编辑器会话的表格结构：tableId、行列、单元格跨度、文本、backgroundColor、fingerprint、sessionVersion 与可执行能力。修改表格结构或单元格底纹前必须调用；不可用时应请用户打开目标 BDocx。",
                "parameters": {
                    "type": "object",
                    "properties": { "path": { "type": "string", "description": "当前活动 BDocx 的相对路径" } },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_images",
                "description": "读取当前活动 BDocx 的图片实例、相邻文本位置摘要与 workspace 内可插入的 PNG/JPEG 项目图片。返回同次 sessionVersion/contentHash、imageId/fingerprint、动态 capabilities/resizeBounds/moveDirections、稳定 Block、项目相对 sourcePath/sourceHash。这里只提供元数据和文档语境，不提供图片像素或视觉识别结果；不得把语境推断表述成看懂了图片。插入、删除、缩放、上下移动或设置独立图片段落对齐前必须调用；不得构造远程、绝对或 workspace 外来源。",
                "parameters": {
                    "type": "object",
                    "properties": { "path": { "type": "string", "description": "当前活动 BDocx 的相对路径" } },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "create_doc",
                "description": "在 workspace 内新建文档并写入初始内容，支持所有格式（.md / .txt / .docx / .html 等）。目标路径不存在则创建；已存在则报错不覆盖。.docx 文件会自动将内容转为 BDocx 富文档格式（纯文本按段落自动换行，HTML 内容保留结构化），创建后用户可在编辑器中打开直接使用 BDocx 编辑能力。.md/.txt 等纯文本格式直接写入。用于用户明确要求「新建文件」「创建文档」时直接创建，不经过 DiffCard 审阅。对已有文档的修改仍必须使用 propose_edit。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "新文档相对路径（相对 workspace 根）。扩展名决定格式：.md 为 Markdown，.docx 为 BDocx 富文档，.txt 为纯文本" },
                        "content": { "type": "string", "description": "新文档的初始内容。.docx 格式支持 HTML 结构化内容（以 < 开头）或纯文本（自动按段落换行）；.md 格式写 Markdown 文本；.txt 格式写纯文本" }
                    },
                    "required": ["path", "content"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "propose_edit",
                "description": "对文档提出修改建议（不直接写盘，生成待用户裁决的修改卡片；活动编辑器草稿可先应用到 LogicalState，但接受或保存才确认，拒绝精确恢复）。纯文本局部编辑使用 replace_block。富文档内容编辑必须使用 block_ops + base + operations，只能命中 list_blocks 返回 capabilities=replace_node 的内容块，并带回稳定 blockId/fingerprint/expectedNodeType 和受控 afterNode；容器/原子块不可通用替换，相关多块须同批提交；在指定块前/后插入新块使用 insert_before/insert_after（blockId 为锚点块），文档末尾追加使用 append（无需 blockId）。只改样式使用 style_ops。表格先 list_tables 再用 table_ops；图片先 list_images 再用 image_ops，支持受控插入、删除、缩放、上下移动、独立段落对齐及 resize/layout/move。不得用原始 HTML 绕过富文档结构化操作。",
                "parameters": propose_edit_parameters
            }
        }
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_registered_with_expected_permissions() {
        assert_eq!(TOOLS.len(), 12);
        assert!(is_registered("read_doc"));
        assert!(is_registered("list_docs"));
        assert!(is_registered("search_text"));
        assert!(is_registered("list_blocks"));
        assert!(is_registered("list_tables"));
        assert!(is_registered("list_images"));
        assert!(is_registered("move_doc"));
        assert!(is_registered("create_folder"));
        assert!(is_registered("import_image_asset"));
        assert!(is_registered("edit_blocked"));
        assert!(is_registered("propose_edit"));
        assert!(is_registered("create_doc"));
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
            "list_tables",
            "list_images",
            "move_doc",
            "create_folder",
            "import_image_asset",
            "edit_blocked",
            "propose_edit",
            "create_doc",
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
        let defs = tool_definitions_json().unwrap();
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
        let schema = propose["function"]["parameters"].to_string();
        assert!(schema.contains("set_cell_background"));
        assert!(schema.contains("insert_image"));
        assert!(schema.contains("subscript"));
        assert!(schema.contains("allOf"));
        assert!(schema.contains("oneOf"));
        let description = propose["function"]["description"].as_str().unwrap();
        assert!(description.contains("缩放"));
        assert!(description.contains("上下移动"));
        assert!(description.contains("独立段落对齐"));
        assert!(description.contains("resize/layout/move"));
        let list_images = defs
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["function"]["name"] == "list_images")
            .unwrap();
        let list_images_description = list_images["function"]["description"].as_str().unwrap();
        assert!(list_images_description.contains("resizeBounds"));
        assert!(list_images_description.contains("moveDirections"));
        assert!(list_images_description.contains("视觉识别"));
    }

    #[test]
    fn structure_tool_schema_exposes_move_and_create_folder() {
        let defs = tool_definitions_json().unwrap();
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
        let import_image = arr
            .iter()
            .find(|d| d["function"]["name"] == "import_image_asset")
            .unwrap();
        assert!(import_image["function"]["parameters"]["properties"]["source_path"].is_object());
        let edit_blocked = arr
            .iter()
            .find(|d| d["function"]["name"] == "edit_blocked")
            .unwrap();
        assert!(edit_blocked["function"]["parameters"]["properties"]["reason"].is_object());
        assert!(!arr.iter().any(|d| d["function"]["name"] == "delete_doc"));
    }
}
