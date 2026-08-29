//! PI-AG-08A · 工具注册表（只读工具 + 结构操作 + WriteIntent 权限分级）。
//!
//! 为 SM-AG-02 `tool::validate_tool` 提供「注册清单 + 权限边界」数据（BR-AG-STATE-002）：模型请求的工具
//! 必须在注册清单内且通过权限校验方可执行。清单 v1 覆盖文档列表/读取/搜索/块读取 + 非破坏性结构操作 + 修改建议。
//!
//! **执行实现**（read_doc/list_docs/search_text/list_blocks 调统一 AgentEditableSnapshot；
//! plan_workspace_operation 只生成 WorkspaceOperationPlan + DecisionRequest；import_image_asset 经 DOC/WS 受控复制；
//! edit_blocked 结构化结束不可安全执行的编辑请求；propose_edit 产出 ChangeProposal→DE）归
//! PI-AG-08B（与模型循环 PI-AG-11 配套，避免重复 lib.rs 既有 route_*）。本模块只持有注册与权限数据。

use super::tool::{validate_tool, ToolDecision};
use super::{
    context::{IntentResultV1, IntentTypeV1},
    turn::AGENT_EDIT_1_0_PROFILE,
};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityGrantV1 {
    pub version: String,
    pub profile: String,
    pub intent_id: String,
    pub turn_id: String,
    pub allowed_tools: Vec<String>,
    pub target_scope: Vec<String>,
    pub readable_scope: Vec<String>,
    pub parameter_constraints: Vec<String>,
    pub grant_hash: String,
}

impl CapabilityGrantV1 {
    pub fn allows(&self, tool_name: &str) -> bool {
        self.allowed_tools
            .iter()
            .any(|allowed| allowed == tool_name)
    }
}

/// IntentResult 只提出候选能力，本函数结合 1.0 profile 生成最终逐回合授权。
pub fn resolve_capability_grant(
    profile: &str,
    turn_id: &str,
    intent: &IntentResultV1,
    explicit_edit_targets: &[String],
) -> CapabilityGrantV1 {
    // AG-STRATEGY-01：工具授予档位化——chat 零工具；read-only 声明（mutation_forbidden，
    // 用户显式"不修改/只分析"）→ 只读全量；其余（edit/create/general）→ 全量工具
    // （只读 + 提案 + glob）。意图不再按类型收窄工具，模型自决具体工具；
    // 工具最小化仍由独立治理项负责；内容写入同时受 ExplicitEditTargetSet 调用时成员校验与
    // DiffCard 确认约束，不能把后置确认当作目标授权来源。
    let read_only_declared = intent
        .constraints
        .iter()
        .any(|c| c == "mutation_forbidden_by_current_turn");
    let profile_tools: &[&str] = match intent.intent_type {
        IntentTypeV1::Answer | IntentTypeV1::Clarify => &[],
        _ if read_only_declared => &[
            "read_doc",
            "list_docs",
            "search_text",
            "list_blocks",
            "list_tables",
            "list_images",
            "workspace_glob",
            "propose_decision",
        ],
        _ => &[
            "read_doc",
            "list_docs",
            "search_text",
            "list_blocks",
            "list_tables",
            "list_images",
            "workspace_glob",
            "propose_edit",
            "edit_blocked",
            "propose_decision",
            "plan_workspace_operation",
        ],
    };
    let allowed_tools = if profile == AGENT_EDIT_1_0_PROFILE {
        profile_tools
            .iter()
            .map(|tool| (*tool).to_string())
            .collect()
    } else {
        Vec::new()
    };
    let target_scope = explicit_edit_targets.to_vec();
    let mut readable_scope = explicit_edit_targets.to_vec();
    if let Some(current) = intent.target.rel_path.as_ref() {
        if !readable_scope.contains(current) {
            readable_scope.push(current.clone());
        }
    }
    if intent.intent_type == IntentTypeV1::Search {
        readable_scope.push("workspace_search".to_string());
    }
    // P4 修复（AG-R2-01）：非寒暄意图下，只读范围放宽到 workspace 内全部文件（只读不授写）。
    // X-FORBID-004 本即限定"只读工具限定 workspace 内"——此前实现收得过窄（仅当前文档），
    // 导致用户"找原文本比对"等合法只读请求被拒。target_scope 仍仅显式编辑目标集，写入不受影响。
    if !matches!(
        intent.intent_type,
        IntentTypeV1::Answer | IntentTypeV1::Clarify
    ) {
        readable_scope.push("workspace_read_all".to_string());
    }
    // 逐回合授权内容 hash（P8 修复）：由 profile + intent_type + 允许工具集 + 目标集派生，
    // 工具集/目标变化即产生新 grant_hash，供审计与回归对比区分"同版本但不同授权"的回合。
    let grant_hash = super::context::sha256_text(
        &serde_json::to_string(&[
            profile,
            &format!("{:?}", intent.intent_type),
            &allowed_tools.join(","),
            &target_scope.join(","),
        ])
        .unwrap_or_default(),
    );
    let mut parameter_constraints = vec![
        "active_workspace_only".to_string(),
        "no_direct_document_write".to_string(),
        "proposal_applies_on_user_confirm".to_string(),
        "propose_edit_requires_explicit_edit_target_set".to_string(),
    ];
    if intent.intent_type == IntentTypeV1::CreateDocument {
        parameter_constraints.push("new_document_requires_workspace_operation_plan".to_string());
    }
    CapabilityGrantV1 {
        version: "capability-grant-v1".to_string(),
        profile: profile.to_string(),
        intent_id: intent.intent_id.clone(),
        turn_id: turn_id.to_string(),
        allowed_tools,
        target_scope,
        readable_scope,
        parameter_constraints,
        grant_hash,
    }
}

pub fn validate_requested_tool_for_grant(
    name: &str,
    params: &serde_json::Value,
    grant: &CapabilityGrantV1,
    turn_id: &str,
) -> ToolDecision {
    if grant.profile != AGENT_EDIT_1_0_PROFILE || grant.turn_id != turn_id {
        return ToolDecision::Reject("工具授权已过期或不属于当前回合".to_string());
    }
    let registered = is_registered(name);
    if !registered {
        return validate_tool(false, false, false);
    }
    if !grant.allows(name) {
        return ToolDecision::Reject(format!("工具 {name} 未获本轮 CapabilityGrant 授权"));
    }
    if name == "propose_decision" {
        if let Err(reason) = validate_scope_decision_parameters(params) {
            return ToolDecision::Reject(reason);
        }
    }
    if matches!(
        name,
        "read_doc" | "list_blocks" | "list_tables" | "list_images"
    ) {
        let target = params
            .get("path")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        if !grant.readable_scope.iter().any(|allowed| allowed == target)
            && !grant
                .readable_scope
                .iter()
                .any(|allowed| allowed == "workspace_search")
            // P4 修复（AG-R2-01）：workspace 内只读放行（X-FORBID-004 限定 workspace 内，
            // 仅影响读取，不授写；exec 层仍校验实际路径在 workspace 内）。
            && !grant.readable_scope.iter().any(|allowed| allowed == "workspace_read_all")
        {
            return ToolDecision::Reject("只读工具目标未在本轮可读范围内".to_string());
        }
    }
    if name == "propose_edit" {
        if grant
            .parameter_constraints
            .iter()
            .any(|constraint| constraint == "new_document_requires_workspace_operation_plan")
        {
            return ToolDecision::Reject(
                "新建文档必须使用 plan_workspace_operation；确认后才建立真实资源并投影初始内容 Diff"
                    .to_string(),
            );
        }
        let target = params
            .get("path")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        if target.trim().is_empty() {
            return ToolDecision::Reject("propose_edit 缺少目标 path".to_string());
        }
        if matches!(
            params.get("operation").and_then(|value| value.as_str()),
            Some("table_ops") | Some("image_ops")
        ) {
            return ToolDecision::Reject(
                "agent-edit-1.0 尚未授权表格或图片高级编辑提案".to_string(),
            );
        }
        // 路径规范化及 targetScope 成员硬拒在 ag_propose_edit / DE 双层执行；本层只有参数，缺少
        // workspace root，不能用原始字符串比较替代规范化成员判断。
    }
    if name == "plan_workspace_operation" {
        let operations = params
            .get("operations")
            .and_then(|value| value.as_array())
            .filter(|items| !items.is_empty());
        let Some(operations) = operations else {
            return ToolDecision::Reject(
                "plan_workspace_operation.operations 必须是非空数组".to_string(),
            );
        };
        let create_count = operations
            .iter()
            .filter(|item| {
                item.get("kind").and_then(|value| value.as_str()) == Some("create_document")
            })
            .count();
        let initial_content = params
            .get("initialContent")
            .and_then(|value| value.as_str())
            .filter(|value| !value.trim().is_empty());
        if create_count > 0 && (create_count != 1 || initial_content.is_none()) {
            return ToolDecision::Reject(
                "新建文档计划必须且只能包含一个 create_document，并提供确认前冻结的完整 initialContent"
                    .to_string(),
            );
        }
        if create_count == 0 && params.get("initialContent").is_some() {
            return ToolDecision::Reject(
                "initialContent 只能与单个 create_document 操作绑定".to_string(),
            );
        }
    }
    validate_tool(true, true, false)
}

/**
 * @GOV
 * codes:    BR-AG-DATA-003-GUARD-AG-AG-TOOL-033, BR-AG-STATE-002-GUARD-AG-AG-TOOL-033
 * type:     GUARD
 * chain:    AG-TOOL
 * rules:    BR-AG-DATA-003, BR-AG-STATE-002
 * term_ref: TERM-AG-019, TERM-AG-017
 * boundary: in=DecisionRequestToolParameters | out=ScopeDecisionWithFrozenNonEmptyTargetPathsOrRejection | does_not=appendTargetsOrTreatReadResultsAsAuthorization
 */
fn validate_scope_decision_parameters(params: &serde_json::Value) -> Result<(), String> {
    let kind = params.get("kind").and_then(|value| value.as_str());
    let target_paths = params.get("targetPaths");
    if kind == Some("scope") {
        let valid = target_paths
            .and_then(|value| value.as_array())
            .is_some_and(|paths| {
                !paths.is_empty()
                    && paths
                        .iter()
                        .all(|path| path.as_str().is_some_and(|path| !path.trim().is_empty()))
            });
        if !valid {
            return Err(
                "scope DecisionRequest 必须提供非空 targetPaths 既有文档路径清单".to_string(),
            );
        }
    } else if target_paths.is_some() {
        return Err("targetPaths 只能用于 kind=scope 的 DecisionRequest".to_string());
    }
    Ok(())
}

/// 工具权限分级。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolPermission {
    /// 只读：读文档/列文档/搜索，经 WS/KB 且限 active workspace（X-FORBID-004）。
    ReadOnly,
    /// 结构操作：移动/重命名/建目录，经 WS 边界执行；不修改文档内容，不走 DE。
    StructureWrite,
    /// 写意图：产出修改建议走 DE diff（X-FORBID-006）；**非直接写盘**。
    WriteIntent,
    /// 任务控制意图：只创建任务级确认点，不改变文档、Diff 或文件树。
    TaskControl,
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
/// （`direct_write_intent=false`）；plan_workspace_operation 只生成前置裁决计划，不直写磁盘；
/// import_image_asset 是受控复制白名单，不直接修改文档内容。
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
        name: "workspace_glob",
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
        name: "plan_workspace_operation",
        permission: ToolPermission::TaskControl,
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
        name: "propose_decision",
        permission: ToolPermission::TaskControl,
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
                "name": "workspace_glob",
                "description": "按文件名模式在工作区内定位文档（通配：* 匹配任意字符序列，? 匹配单个字符）。如 workspace_glob(pattern=\"**/寓言*.txt\") 返回匹配的文件路径列表（不含正文）。用于替代逐目录展开 list_docs，快速定位目标文件。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "文件名通配模式（相对 workspace 根，支持 * 与 ?；如 \"*寓言*.txt\"、\"文档/*.docx\"）。"
                        }
                    },
                    "required": ["pattern"],
                    "additionalProperties": false
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_docs",
                "description": "感知当前工作区（workspace）结构。不传 path：返回工作区概览——顶层目录及各自文件数、根级文件列表、总文件数（工作区较小时直接返回全部文件，属豁免）；传 path（目录相对路径）：返回该目录的直接子项（一层，不递归）。文件内容一律用 read_doc 按需读取（本工具永不返回正文）。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "可选。目录相对路径（如 \"文学作品\"）；传此参数时返回该目录的直接子项。"
                        }
                    },
                    "additionalProperties": false
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "plan_workspace_operation",
                "description": "为工作区资源创建、建目录、重命名、移动或删除生成不可变操作计划和独立确认卡。工具成功只表示正在等待用户确认；确认前不修改磁盘，也不生成 DiffCard。确认后系统重新核验目标并直接执行。新建文档时必须同时提供 initialContent；系统确认后先创建真实文件，再把初始内容作为普通内容 Diff 投影。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "operations": {
                            "type": "array",
                            "minItems": 1,
                            "items": {
                                "type": "object",
                                "properties": {
                                    "kind": { "type": "string", "enum": ["create_document", "create_folder", "rename_resource", "move_resource", "delete_resource"] },
                                    "sourcePath": { "type": "string", "description": "删除/重命名/移动的源资源相对路径" },
                                    "targetPath": { "type": "string", "description": "创建/重命名/移动的目标资源相对路径" },
                                    "resourceKind": { "type": "string", "enum": ["file", "folder"] },
                                    "recursive": { "type": "boolean", "description": "删除目录时是否递归" }
                                },
                                "required": ["kind", "resourceKind"],
                                "additionalProperties": false
                            }
                        },
                        "initialContent": { "type": "string", "description": "仅 create_document：确认前生成并冻结的新文档完整初始内容" }
                    },
                    "required": ["operations"],
                    "additionalProperties": false
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
                "name": "propose_decision",
                "description": "当且仅当任务无法在当前执行片段继续、且确实需要用户作权限授予、方案选择、范围确认或人工确认时，提出任务级确认卡。成功后当前执行片段立即结束并等待用户 confirm/cancel；它不修改文档、Diff 或文件树。不要用它替代普通编辑 DiffCard，也不要在调用成功后继续调用其他工具。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "kind": {
                            "type": "string",
                            "enum": ["permission", "plan_choice", "scope", "manual_confirm"],
                            "description": "需要用户裁决的类别"
                        },
                        "summary": {
                            "type": "string",
                            "description": "面向用户的简短裁决标题或问题"
                        },
                        "detail": {
                            "type": "string",
                            "description": "可选：作出裁决所需的具体上下文"
                        },
                        "options": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "可选：用户可选方案；未提供时使用确认/取消"
                        },
                        "targetPaths": {
                            "type": "array",
                            "minItems": 1,
                            "items": { "type": "string" },
                            "description": "仅 kind=scope：冻结的 active workspace 既有文档相对路径清单；read/search 结果不会自动获得修改授权"
                        }
                    },
                    "required": ["kind", "summary"],
                    "additionalProperties": false
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "read_doc",
                "description": "读取当前工作区内指定相对路径文档的内容。省略 blocks 时返回完整内容；传入 blocks（块 id 或块索引数组）时只返回指定块的完整投影（含 node_json，供 propose_edit block_ops 从 node_json 复制节点构造 afterNode）。注意：当前编辑器打开的文档已在上下文提供，无需用本工具重复读取。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "文档相对路径（相对 workspace 根）" },
                        "blocks": { "type": "array", "items": { "type": "string" }, "description": "可选：需要读取的块 id 或块索引数组（如 [\"b_xxx\", \"0\"]），只返回这些块的完整投影" }
                    },
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
                "description": "读取指定文档的块目录（轻量投影，不含 node_json/完整文本）。返回 block_id/block_index/node_type/role/parent_block_id/depth/text(前40字)/fingerprint/capabilities；需要某块完整投影（node_json）时调用 read_doc 并传入 blocks 参数。当前富文档直接返回 ProseMirror 层级投影；只有 capabilities 含 replace_node 的 content 块可用于 block_ops，容器与原子块只读。",
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
                "name": "propose_edit",
                "description": "对文档提出待用户裁决的修改建议，不直接写盘。工具成功只表示 pending 建议卡已生成，不表示编辑器预览已应用、用户已接受或文件已保存；没有客户端状态回执时，回复只能说「已提出建议，待确认」。纯文本局部编辑使用 replace_block。当前 BDocx 的整篇重写只有运行时声明确认无表格且无图片时才可用 replace_document + 完整 HTML；否则 replace_document 会被后端拒绝，既有文字内容用 block_ops，表格/图片分别用 table_ops/image_ops。block_ops 只能命中 list_blocks 返回 capabilities=replace_node 的内容块，并带回稳定 blockId/fingerprint/expectedNodeType 和受控 afterNode；容器/原子块不可通用替换，相关多块须同批提交。可用 insert_before/insert_after/append 插入一个新的 heading 块；replace_node 必须保留既有块类型，不能把既有段落转换为标题。只改样式使用 style_ops。表格先 list_tables 再用 table_ops；图片先 list_images 再用 image_ops，支持受控插入、删除、缩放、上下移动、独立段落对齐及 resize/layout/move。不得用原始 HTML 绕过富文档受保护结构。",
                "parameters": propose_edit_parameters
            }
        }
    ]))
}

/// 只把逐回合授权的工具暴露给模型。注册表可以包含未来能力，但 1.0 模型不可枚举未授权项。
pub fn tool_definitions_json_for_grant(
    grant: &CapabilityGrantV1,
) -> Result<serde_json::Value, super::bdocx_capabilities::CapabilityValidationError> {
    let all = tool_definitions_json()?;
    let filtered = all
        .as_array()
        .into_iter()
        .flatten()
        .filter(|definition| {
            definition["function"]["name"]
                .as_str()
                .map(|name| grant.allows(name))
                .unwrap_or(false)
        })
        .cloned()
        .collect::<Vec<_>>();
    Ok(serde_json::Value::Array(filtered))
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
        assert!(is_registered("workspace_glob"));
        assert!(!is_registered("move_doc"));
        assert!(!is_registered("create_folder"));
        assert!(is_registered("import_image_asset"));
        assert!(is_registered("edit_blocked"));
        assert!(is_registered("propose_edit"));
        assert!(is_registered("propose_decision"));
        assert!(is_registered("plan_workspace_operation"));
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
            lookup("plan_workspace_operation").unwrap().permission,
            ToolPermission::TaskControl
        );
        assert_eq!(
            lookup("propose_edit").unwrap().permission,
            ToolPermission::WriteIntent
        );
        assert_eq!(
            lookup("propose_decision").unwrap().permission,
            ToolPermission::TaskControl
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
    fn all_registered_tools_pass_validation_including_task_decision() {
        // 工具均合法放行：只读工具 + 结构操作 + 修改建议 + 任务级确认（均不直写文档内容）。
        for t in [
            "read_doc",
            "list_docs",
            "search_text",
            "list_blocks",
            "list_tables",
            "list_images",
            "plan_workspace_operation",
            "import_image_asset",
            "edit_blocked",
            "propose_edit",
            "propose_decision",
        ] {
            assert_eq!(
                validate_requested_tool(t),
                ToolDecision::Allow,
                "{t} 应放行"
            );
        }
    }

    #[test]
    fn capability_grant_is_minimal_and_registered_does_not_mean_authorized() {
        let analyze = super::super::context::resolve_intent_result(
            "t-analyze",
            "只分析当前文档，不要修改",
            Some("a.md"),
            false,
        );
        let analyze_grant =
            resolve_capability_grant(AGENT_EDIT_1_0_PROFILE, "t-analyze", &analyze, &[]);
        assert!(analyze_grant.allows("read_doc"));
        assert!(analyze_grant.allows("propose_decision"));
        assert!(!analyze_grant.allows("propose_edit"));
        assert!(!analyze_grant.allows("plan_workspace_operation"));
        assert!(matches!(
            validate_requested_tool_for_grant(
                "propose_edit",
                &serde_json::json!({"path":"a.md"}),
                &analyze_grant,
                "t-analyze"
            ),
            ToolDecision::Reject(_)
        ));

        let defs = tool_definitions_json_for_grant(&analyze_grant).unwrap();
        let names = defs
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|definition| definition["function"]["name"].as_str())
            .collect::<Vec<_>>();
        assert!(names.contains(&"read_doc"));
        assert!(!names.contains(&"propose_edit"));
        assert!(names.contains(&"propose_decision"));
        assert!(!names.contains(&"plan_workspace_operation"));

        // P4 修复（AG-R2-01）：非寒暄意图 readable_scope 含 workspace_read_all，
        // read_doc 可读取 workspace 内其他文件（只读不授写）。
        assert!(analyze_grant
            .readable_scope
            .contains(&"workspace_read_all".to_string()));
        assert!(matches!(
            validate_requested_tool_for_grant(
                "read_doc",
                &serde_json::json!({"path":"文学作品/白鹿原/白鹿原小说章节策划.docx"}),
                &analyze_grant,
                "t-analyze"
            ),
            ToolDecision::Allow
        ));
        // 只读 IntentResult 即使具有候选目标成员，也不会获得 propose_edit 工具。
        assert!(matches!(
            validate_requested_tool_for_grant(
                "propose_edit",
                &serde_json::json!({"path":"other.md","operation":"replace_document","content":"x"}),
                &analyze_grant,
                "t-analyze"
            ),
            ToolDecision::Reject(_)
        ));

        let edit = super::super::context::resolve_intent_result(
            "t-edit-targets",
            "修改当前文档并同步显式引用",
            Some("a.md"),
            false,
        );
        let edit_grant = resolve_capability_grant(
            AGENT_EDIT_1_0_PROFILE,
            "t-edit-targets",
            &edit,
            &["a.md".to_string(), "ref.md".to_string()],
        );
        assert_eq!(edit_grant.target_scope, vec!["a.md", "ref.md"]);
        assert!(edit_grant
            .parameter_constraints
            .contains(&"propose_edit_requires_explicit_edit_target_set".to_string()));

        let answer = super::super::context::resolve_intent_result("t-answer", "你好", None, false);
        let answer_grant =
            resolve_capability_grant(AGENT_EDIT_1_0_PROFILE, "t-answer", &answer, &[]);
        assert!(!answer_grant.allows("propose_decision"));
        assert!(matches!(
            validate_requested_tool_for_grant(
                "propose_decision",
                &serde_json::json!({"kind":"manual_confirm", "summary":"确认"}),
                &answer_grant,
                "t-answer"
            ),
            ToolDecision::Reject(_)
        ));
    }

    #[test]
    fn create_document_grant_uses_workspace_operation_plan_not_empty_base_proposal() {
        let intent = super::super::context::resolve_intent_result(
            "t-create",
            "新建一份会议纪要",
            None,
            false,
        );
        let grant = resolve_capability_grant(AGENT_EDIT_1_0_PROFILE, "t-create", &intent, &[]);
        // CreateDocument 获得资源计划工具；初始内容在确认前冻结，不能再走 base=empty 提案。
        assert_eq!(
            grant.allowed_tools,
            vec![
                "read_doc",
                "list_docs",
                "search_text",
                "list_blocks",
                "list_tables",
                "list_images",
                "workspace_glob",
                "propose_edit",
                "edit_blocked",
                "propose_decision",
                "plan_workspace_operation"
            ]
        );
        assert!(matches!(
            validate_requested_tool_for_grant(
                "plan_workspace_operation",
                &serde_json::json!({
                    "operations":[{"kind":"create_document","targetPath":"meeting.md","resourceKind":"file"}],
                    "initialContent":"# 会议纪要"
                }),
                &grant,
                "t-create"
            ),
            ToolDecision::Allow
        ));
        // 新建不能退回普通内容提案的空基准路径。
        assert!(matches!(
            validate_requested_tool_for_grant(
                "propose_edit",
                &serde_json::json!({"path":"meeting.md","operation":"replace_document","content":"x"}),
                &grant,
                "t-create"
            ),
            ToolDecision::Reject(_)
        ));
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
        assert!(description.contains("pending 建议卡已生成"));
        assert!(description.contains("不表示编辑器预览已应用"));
        assert!(description.contains("无表格且无图片"));
        assert!(description.contains("replace_document 会被后端拒绝"));
        assert!(description.contains("插入一个新的 heading 块"));
        assert!(description.contains("不能把既有段落转换为标题"));
        assert!(!description.contains("草稿可先应用到 LogicalState"));
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
    fn propose_decision_schema_is_task_control_not_document_write() {
        let defs = tool_definitions_json().unwrap();
        let decision = defs
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["function"]["name"] == "propose_decision")
            .unwrap();
        let parameters = &decision["function"]["parameters"];
        assert_eq!(
            parameters["required"],
            serde_json::json!(["kind", "summary"])
        );
        assert!(parameters["properties"]["kind"]["enum"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("manual_confirm")));
        assert!(decision["function"]["description"]
            .as_str()
            .unwrap()
            .contains("不修改文档、Diff 或文件树"));
        assert_eq!(
            parameters["properties"]["targetPaths"]["minItems"],
            serde_json::json!(1)
        );
        assert!(validate_scope_decision_parameters(&serde_json::json!({
            "kind": "scope",
            "summary": "允许修改",
            "targetPaths": ["a.md"]
        }))
        .is_ok());
        assert!(validate_scope_decision_parameters(&serde_json::json!({
            "kind": "scope",
            "summary": "允许修改"
        }))
        .is_err());
        assert!(validate_scope_decision_parameters(&serde_json::json!({
            "kind": "manual_confirm",
            "summary": "确认",
            "targetPaths": ["a.md"]
        }))
        .is_err());
    }

    #[test]
    fn workspace_operation_schema_replaces_direct_structure_tools() {
        let defs = tool_definitions_json().unwrap();
        let arr = defs.as_array().unwrap();
        let operation = arr
            .iter()
            .find(|d| d["function"]["name"] == "plan_workspace_operation")
            .unwrap();
        assert!(operation["function"]["parameters"]["properties"]["operations"].is_object());
        assert!(operation["function"]["parameters"]["properties"]["initialContent"].is_object());
        assert!(!arr.iter().any(|d| d["function"]["name"] == "move_doc"));
        assert!(!arr.iter().any(|d| d["function"]["name"] == "create_folder"));
        assert!(!arr.iter().any(|d| d["function"]["name"] == "create_doc"));
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
