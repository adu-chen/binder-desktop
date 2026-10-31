//! PI-AG-08F · 文档分块与结构化块快照。
//!
//! 两种文档形态：
//! - **text/source-protected Markdown**：按行（`\n`）切块；结构化 Markdown 的 LogicalState 以 HTML
//!   contentFormat 投影，复用 HTML 块解析，避免 Markdown 源行与编辑器块错位。
//! - **docx/html**（editFormat=html）：活动编辑器必须直接投影 ProseMirror 层级块快照。HTML 字符串扫描
//!   只用于未打开文档的只读兼容，不再作为富文档写操作的结构真值。
//!
//! dev 级轻量字符串扫描（无 HTML parser 依赖），适配 TipTap 平铺块元素（p/h/li/blockquote 等带 block-id）；
//! 嵌套同名块、畸形 HTML 不保证——真实 ED blockId 锚定位（对齐 LocateResult）后续。

use crate::ipc::IpcError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::path::Path;

fn deserialize_optional_u64<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Number(number)) => number
            .as_u64()
            .map(Some)
            .ok_or_else(|| serde::de::Error::custom("baseline 必须是非负 u64")),
        Some(serde_json::Value::String(number)) => number
            .parse::<u64>()
            .map(Some)
            .map_err(|_| serde::de::Error::custom("baseline 字符串必须是 u64")),
        Some(_) => Err(serde::de::Error::custom(
            "baseline 必须是 u64 数字、十进制字符串或 null",
        )),
    }
}

/// 一个文档块。`block_id` = HTML 块的 data-block-id（稳定锚；text 块 None）；
/// `inner_span` = HTML 块 inner 在原串中的字节区间（in-place 重建用；text 块 None）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocBlock {
    pub block_id: Option<String>,
    pub block_type: Option<String>,
    pub text: String,
    pub inner_span: Option<(usize, usize)>,
}

/// AgentEdit 统一可消费块投影：`block_index` 是当前快照内 0 起块号；`block_id` 是 HTML/BDocx
/// 的稳定锚，text 文档可为空。marks/block_attrs 仅 HTML 快照有语义。
/// AG-CTX-01 F5：序列化瘦身——空 marks/block_attrs 省略，减小工具结果体积。
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct AgentEditableBlock {
    pub block_id: Option<String>,
    pub block_index: usize,
    pub block_type: Option<String>,
    pub node_type: Option<String>,
    pub role: String,
    pub parent_block_id: Option<String>,
    pub depth: usize,
    pub ordinal: usize,
    pub text: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub marks: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "is_empty_object")]
    pub block_attrs: serde_json::Value,
    pub node_json: serde_json::Value,
    pub fingerprint: Option<String>,
    pub capabilities: Vec<String>,
}

fn is_empty_object(value: &serde_json::Value) -> bool {
    value.as_object().map(|map| map.is_empty()).unwrap_or(false)
}

/// 当前 ProseMirror LogicalState 的层级块投影。字段名与前端 OpenDocSnapshot 协议一致。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentEditableBlockSnapshot {
    pub block_id: String,
    pub block_index: usize,
    pub block_type: String,
    pub node_type: String,
    pub role: String,
    pub parent_block_id: Option<String>,
    pub depth: usize,
    pub ordinal: usize,
    pub text: String,
    pub attrs: serde_json::Value,
    pub marks: Vec<serde_json::Value>,
    pub node_json: serde_json::Value,
    pub fingerprint: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

/// AgentEdit 统一内容快照。BDocx 下游一律消费此编辑模型快照，不直接读取 docx 二进制。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AgentEditableSnapshot {
    pub rel_path: String,
    pub editable_kind: String,
    pub source: String,
    pub content_format: String,
    pub content: String,
    pub baseline: Option<u64>,
    pub session_version: Option<u64>,
    pub content_hash: Option<String>,
    pub blocks: Vec<AgentEditableBlock>,
}

/// 前端打开文档的 LogicalState 快照。active 文档来自实时编辑器序列化，非 active 打开文档来自
/// EditSession.content。AG 读取层必须优先使用该集合，未打开文档才读取 DiskState。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenDocSnapshot {
    pub rel_path: String,
    pub content: String,
    #[serde(default, deserialize_with = "deserialize_optional_u64")]
    pub baseline: Option<u64>,
    pub edit_format: String,
    pub content_format: String,
    pub source: String,
    pub dirty: bool,
    pub session_version: u64,
    pub content_hash: String,
    pub captured_at: i64,
    /// 仅活动 HTML 编辑器提供，直接由同次 ProseMirror LogicalState 投影；富文档结构写操作
    /// 必须消费本字段，不能从 content HTML 再次猜测容器和子块。
    #[serde(default)]
    pub block_snapshots: Vec<AgentEditableBlockSnapshot>,
    /// 仅活动 HTML 编辑器提供的 ProseMirror 表格投影。后端据此提供 list_tables，
    /// 不从 HTML 字符串猜测嵌套表格结构。
    #[serde(default)]
    pub table_snapshots: Vec<serde_json::Value>,
    /// 仅活动 BDocx 编辑器提供的 ProseMirror 图片实例投影。图片身份与 fingerprint
    /// 必须和同次 sessionVersion/contentHash 捕获。
    #[serde(default)]
    pub image_snapshots: Vec<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct AgentReadContext {
    root: std::path::PathBuf,
    open_docs: HashMap<String, OpenDocSnapshot>,
    resource_facts: Option<HashMap<String, crate::types::AgentContextResourceV2>>,
    verified_disk_paths: std::collections::BTreeSet<String>,
    disk_inventory_verified: bool,
}

impl AgentReadContext {
    pub fn new(root: &Path, open_docs: Vec<OpenDocSnapshot>) -> Result<Self, IpcError> {
        let mut indexed = HashMap::new();
        for snapshot in open_docs {
            let rel_path = snapshot.rel_path.trim();
            if rel_path.is_empty() {
                return Err(IpcError::new(
                    "AG_OPEN_DOC_SNAPSHOT_INVALID",
                    "已打开文档快照缺少 rel_path",
                    false,
                    false,
                ));
            }
            crate::workspace::resolve_in_workspace(root, rel_path)?;
            indexed.insert(rel_path.to_string(), snapshot);
        }
        Ok(Self {
            root: root.to_path_buf(),
            open_docs: indexed,
            resource_facts: None,
            verified_disk_paths: Default::default(),
            disk_inventory_verified: false,
        })
    }

    /**
     * @GOV
     * codes:    BR-AG-DATA-001-GUARD-AG-AG-TOOL-044, BR-AG-DATA-008-GUARD-AG-AG-TOOL-044
     * type:     GUARD
     * chain:    AG-TURN, AG-TOOL
     * rules:    BR-AG-DATA-001, BR-AG-DATA-008
     * term_ref: TERM-AG-020
     * boundary: in=VerifiedResourceIdentitiesOpenDocSnapshotsAndRequestedPaths | out=AgentReadContextWithExplicitDiskReadBoundary | does_not=inferClosedResourceFromMissingSnapshot | consumer=BR-AG-DATA-001,BR-AG-DATA-008
     */
    pub fn with_resources(
        root: &Path,
        snapshots: Vec<OpenDocSnapshot>,
        facts: &crate::types::AgentContextFactsV2,
        requested: &[String],
        all_documents: bool,
    ) -> Result<Self, IpcError> {
        let mut context = Self::new(root, snapshots)?;
        let mut resources = HashMap::new();
        for resource in &facts.open_resources {
            crate::workspace::resolve_in_workspace(root, &resource.rel_path)?;
            if resources
                .insert(resource.rel_path.clone(), resource.clone())
                .is_some()
            {
                return Err(IpcError::new(
                    "AG_CONTEXT_FACTS_DUPLICATE_DOC",
                    "打开资源身份重复",
                    false,
                    false,
                ));
            }
        }
        let paths = requested
            .iter()
            .map(|path| context.normalized_resource_path(path))
            .collect::<Result<_, _>>()?;
        context.resource_facts = Some(resources);
        context.verified_disk_paths = paths;
        context.disk_inventory_verified = all_documents;
        Ok(context)
    }

    pub fn check_resource_available(&self, path: &str) -> Result<(), IpcError> {
        if let Some(resources) = &self.resource_facts {
            let normalized = self.normalized_resource_path(path)?;
            if let Some(resource) = resources.get(&normalized) {
                if resource.availability != crate::types::AgentResourceAvailabilityV2::Ready
                    || !self.open_docs.contains_key(&normalized)
                {
                    return Err(IpcError::new(
                        "AG_CONTEXT_RESOURCE_UNAVAILABLE",
                        &format!(
                            "文档 {path} 的逻辑态尚不可用（{}），请就绪后重新提交；未读取磁盘副本",
                            resource
                                .reason_code
                                .as_deref()
                                .unwrap_or("AG_CONTEXT_NOT_REQUESTED")
                        ),
                        true,
                        false,
                    ));
                }
            } else if !self.disk_inventory_verified
                && !self.verified_disk_paths.contains(&normalized)
            {
                return Err(IpcError::new(
                    "AG_CONTEXT_RESOURCE_UNVERIFIED",
                    "读取前尚未核验资源是否打开，请先请求该文档",
                    true,
                    false,
                ));
            }
        }
        Ok(())
    }

    fn normalized_resource_path(&self, path: &str) -> Result<String, IpcError> {
        let absolute = crate::workspace::resolve_in_workspace(&self.root, path)?;
        absolute
            .strip_prefix(&self.root)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .map_err(|_| {
                IpcError::new(
                    "AG_CONTEXT_RESOURCE_INVALID",
                    "资源路径不属于当前工作区",
                    false,
                    false,
                )
            })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn open_snapshot(&self, rel_path: &str) -> Option<&OpenDocSnapshot> {
        if self.resource_facts.is_some() {
            self.normalized_resource_path(rel_path)
                .ok()
                .and_then(|path| self.open_docs.get(&path))
        } else {
            self.open_docs.get(rel_path)
        }
    }

    pub fn table_snapshots(&self, rel_path: &str) -> Result<&[serde_json::Value], IpcError> {
        if self.resource_facts.is_some() {
            self.check_resource_available(rel_path)?;
            if self
                .open_snapshot(rel_path)
                .is_some_and(|s| s.source != "active_editor")
            {
                return Err(IpcError::new(
                    "AG_TABLE_EDIT_REQUIRES_ACTIVE_EDITOR",
                    "表格结构需要当前活动编辑器快照",
                    true,
                    false,
                ));
            }
        }
        self.open_snapshot(rel_path)
            .map(|snapshot| snapshot.table_snapshots.as_slice())
            .ok_or_else(|| {
                IpcError::new(
                    "AG_TABLE_EDIT_REQUIRES_ACTIVE_EDITOR",
                    "表格结构读取必须在活动编辑器会话中进行",
                    false,
                    false,
                )
            })
    }

    /**
     * @GOV
     * codes:    BR-AG-DATA-001-QUERY-AG-AG-TOOL-011, BR-ED-DATA-003-QUERY-AG-AG-TOOL-011
     * type:     QUERY
     * chain:    AG-TOOL
     * rules:    BR-AG-DATA-001, BR-ED-DATA-003
     * term_ref: TERM-ED-008, TERM-DOC-003
     * boundary: in=ActiveBDocxOpenDocSnapshot | out=CurrentImageSnapshots(sameSessionVersionAndContentHash) | consumer=BR-AG-DATA-001
     */
    pub fn image_snapshots(&self, rel_path: &str) -> Result<&[serde_json::Value], IpcError> {
        let snapshot = self.open_snapshot(rel_path).ok_or_else(|| {
            IpcError::new(
                "AG_IMAGE_OPERATION_REQUIRES_ACTIVE_EDITOR",
                "图片操作必须在当前活动 BDocx 编辑器中进行",
                false,
                false,
            )
        })?;
        if snapshot.source != "active_editor"
            || snapshot.edit_format != "html"
            || snapshot.content_format != "html"
            || !is_docx_rel_path(rel_path)
            || !crate::workspace::listing::is_bdocx(&self.root, rel_path)
        {
            return Err(IpcError::new(
                "AG_IMAGE_OPERATION_REQUIRES_ACTIVE_EDITOR",
                "图片操作只支持当前活动 BDocx HTML 编辑器",
                false,
                false,
            ));
        }
        Ok(snapshot.image_snapshots.as_slice())
    }

    pub fn snapshot(&self, rel_path: &str) -> Result<AgentEditableSnapshot, IpcError> {
        self.check_resource_available(rel_path)?;
        if let Some(open) = self.open_snapshot(rel_path) {
            if open.content_format == "html" && open.source == "active_editor" {
                if open.block_snapshots.is_empty() {
                    return Err(IpcError::new(
                        "AG_STRUCTURED_SNAPSHOT_REQUIRED",
                        "当前富文档缺少 ProseMirror 层级块快照，请等待编辑器完成绑定后重试",
                        true,
                        false,
                    ));
                }
                return Ok(snapshot_from_open_doc(rel_path, open));
            }
            return Ok(snapshot_from_content(
                rel_path,
                &open.content,
                &open.source,
                Some(open.edit_format.as_str()),
                Some(open.content_format.as_str()),
                open.baseline,
                Some(open.session_version),
                Some(open.content_hash.clone()),
                None,
            ));
        }
        let abs = crate::workspace::resolve_in_workspace(&self.root, rel_path)?;
        read_editable_snapshot(&self.root, rel_path, &abs, None, None)
    }
}

fn snapshot_from_open_doc(rel_path: &str, open: &OpenDocSnapshot) -> AgentEditableSnapshot {
    AgentEditableSnapshot {
        rel_path: rel_path.to_string(),
        editable_kind: editable_kind(rel_path, true),
        source: open.source.clone(),
        content_format: open.content_format.clone(),
        content: open.content.clone(),
        baseline: open.baseline,
        session_version: Some(open.session_version),
        content_hash: Some(open.content_hash.clone()),
        blocks: open
            .block_snapshots
            .iter()
            .map(|block| AgentEditableBlock {
                block_id: Some(block.block_id.clone()),
                block_index: block.block_index,
                block_type: Some(block.block_type.clone()),
                node_type: Some(block.node_type.clone()),
                role: block.role.clone(),
                parent_block_id: block.parent_block_id.clone(),
                depth: block.depth,
                ordinal: block.ordinal,
                text: block.text.clone(),
                marks: block.marks.clone(),
                block_attrs: block.attrs.clone(),
                node_json: block.node_json.clone(),
                fingerprint: Some(block.fingerprint.clone()),
                capabilities: block.capabilities.clone(),
            })
            .collect(),
    }
}

fn doc_type_of_rel(rel_path: &str) -> String {
    Path::new(rel_path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_else(|| "bin".to_string())
}

fn is_docx_rel_path(rel_path: &str) -> bool {
    matches!(doc_type_of_rel(rel_path).as_str(), "docx" | "doc")
}

fn editable_kind(rel_path: &str, content_is_html: bool) -> String {
    if is_docx_rel_path(rel_path) {
        "bdocx".to_string()
    } else if content_is_html {
        "html".to_string()
    } else {
        "text".to_string()
    }
}

fn snapshot_from_content(
    rel_path: &str,
    content: &str,
    source: &str,
    format_hint: Option<&str>,
    content_format_hint: Option<&str>,
    baseline: Option<u64>,
    session_version: Option<u64>,
    content_hash: Option<String>,
    // AG-DOCX-RO-01：原生 docx 只读投影时覆盖 editable_kind（"html" 而非 "bdocx"），
    // 使 bdocx_capabilities 不注入编辑能力声明、结构性排除写入。
    editable_kind_override: Option<&str>,
) -> AgentEditableSnapshot {
    let content_is_html = content_format_hint
        .map(|format| format == "html")
        .unwrap_or_else(|| is_html(content, format_hint));
    let content_format = if content_is_html { "html" } else { "text" }.to_string();
    let blocks = project_blocks(content, content_is_html);
    AgentEditableSnapshot {
        rel_path: rel_path.to_string(),
        editable_kind: editable_kind_override
            .map(str::to_string)
            .unwrap_or_else(|| editable_kind(rel_path, content_is_html)),
        source: source.to_string(),
        content_format,
        content: content.to_string(),
        baseline,
        session_version,
        content_hash,
        blocks,
    }
}

/// 是否按 HTML 处理（editFormat=html 或内容以 `<` 开头）。
pub fn is_html(content: &str, edit_format: Option<&str>) -> bool {
    matches!(edit_format, Some("html" | "html_file")) || content.trim_start().starts_with('<')
}

/// 解析文档块。
pub fn parse_blocks(content: &str, is_html: bool) -> Vec<DocBlock> {
    if is_html {
        parse_html_blocks(content)
    } else {
        content
            .split('\n')
            .map(|s| DocBlock {
                block_id: None,
                block_type: None,
                text: s.to_string(),
                inner_span: None,
            })
            .collect()
    }
}

/// 构建统一块投影。当前编辑器上下文、`ag.list_blocks`、只读 tool 与 `propose_edit`
/// 必须复用该函数，避免同一 BDocx 在不同入口形成不同块列表。
pub fn project_blocks(content: &str, is_html: bool) -> Vec<AgentEditableBlock> {
    parse_blocks(content, is_html)
        .into_iter()
        .enumerate()
        .map(|(block_index, b)| {
            let (marks, block_attrs) = match b.inner_span {
                Some((s, e)) => {
                    let (_, spans) = extract_marks(&content[s..e]);
                    let marks = spans
                        .into_iter()
                        .map(|m| {
                            let mut o = serde_json::json!({
                                "type": m.mark_type,
                                "range": [m.range.0, m.range.1],
                            });
                            if let Some(attrs) = m.attrs {
                                o["attrs"] = attrs;
                            }
                            o
                        })
                        .collect();
                    let open = content[..s]
                        .rfind('<')
                        .map(|lt| &content[lt..s])
                        .unwrap_or("");
                    (marks, extract_block_attrs(open))
                }
                None => (Vec::new(), serde_json::json!({})),
            };
            AgentEditableBlock {
                block_id: b.block_id,
                block_index,
                node_type: b.block_type.clone(),
                block_type: b.block_type,
                role: "legacy".to_string(),
                parent_block_id: None,
                depth: 0,
                ordinal: block_index,
                text: b.text,
                marks,
                block_attrs,
                node_json: serde_json::Value::Null,
                fingerprint: None,
                capabilities: Vec::new(),
            }
        })
        .collect()
}

/**
 * @GOV
 * codes:    BR-AG-DATA-001-QUERY-AG-AG-TURN-010, BR-DOC-DATA-003-QUERY-AG-AG-TURN-010
 * type:     QUERY
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-001, BR-DOC-DATA-003
 * term_ref: TERM-DOC-003, TERM-ED-001
 * boundary: in=WorkspaceDocumentOrActiveEditorSnapshot | out=AgentEditableSnapshot(activeProseMirrorHierarchyOrReadOnlyLegacyProjection) | consumer=BR-AG-DATA-001
 */
/// AgentEdit 统一快照读取：当前编辑器实时内容优先；非当前 BDocx 只经 DOC 转换链读取 HTML 编辑模型；
/// 普通文本文件走 WS 文本读取。AG/REF/DE 下游不得绕过该快照另读 BDocx 二进制。
pub fn read_editable_snapshot(
    root: &Path,
    rel_path: &str,
    abs: &Path,
    active_content: Option<&str>,
    active_format: Option<&str>,
) -> Result<AgentEditableSnapshot, IpcError> {
    let (content, source, baseline, format_hint) = if let Some(content) = active_content {
        (
            content.to_string(),
            "active_editor".to_string(),
            None,
            active_format.map(str::to_string),
        )
    } else if is_docx_rel_path(rel_path) {
        // AG-DOCX-RO-01：原生 docx 也复用 DOC 转换链只读读取（pandoc 只读源文件、写临时 work_dir），
        // 与 b-docx 转换同链路，不新增解析器；源文件不变。原生 docx 标记 doc_convert_readonly 且
        // editable_kind 覆盖为 "html"（≠bdocx）→ 不注入 BDocx 编辑能力声明、无 replace_node → 结构性只读。
        let is_native_docx = !crate::workspace::listing::is_bdocx(root, rel_path);
        let bytes = std::fs::read(abs).map_err(|e| {
            IpcError::new(
                "DOC_READ_FAILED",
                &format!("读 docx 失败: {e}"),
                true,
                false,
            )
        })?;
        let baseline = crate::workspace::save::content_hash(&bytes);
        let work_dir = std::env::temp_dir()
            .join("binder_ag_snapshot")
            .join(format!("{baseline:x}"));
        (
            crate::doc::edit_chain::docx_to_html(abs, &work_dir)?,
            if is_native_docx {
                "doc_convert_readonly".to_string()
            } else {
                "doc_convert".to_string()
            },
            Some(baseline),
            Some("html".to_string()),
        )
    } else {
        let read = crate::workspace::save::read_doc(abs)?;
        (
            read.content,
            "text_file".to_string(),
            Some(read.baseline),
            None,
        )
    };
    Ok(snapshot_from_content(
        rel_path,
        &content,
        &source,
        format_hint.as_deref(),
        None,
        baseline,
        None,
        None,
        // 原生 docx 只读投影：editable_kind 覆盖为 "html"（≠bdocx），不进入 BDocx 编辑能力判定。
        if source == "doc_convert_readonly" {
            Some("html")
        } else {
            None
        },
    ))
}

/// 仅取各块展示文本（块列表注入用）。
pub fn block_texts(content: &str, is_html: bool) -> Vec<String> {
    parse_blocks(content, is_html)
        .into_iter()
        .map(|b| b.text)
        .collect()
}

/**
 * @GOV
 * codes: BR-AG-DATA-005-DATA-AG-AG-TOOL-058
 * type: DATA
 * chain: AG-TOOL
 * rules: BR-AG-DATA-005
 * boundary: in=AgentEditableSnapshot | out=ReadingTextWithParagraphAndTableSeparators | does_not=ChangeEditorProjectionOrCreateEditBaseline | consumer=BR-AG-DATA-005
 */
/// Reading projection, not an edit baseline. Scan the original HTML once so parent
/// containers never duplicate their descendants. Keep table cell/row boundaries.
pub(super) fn reading_text(snapshot: &AgentEditableSnapshot) -> String {
    if snapshot.content_format != "html" {
        return snapshot.content.clone();
    }
    let html = &snapshot.content;
    let lower = html.to_ascii_lowercase();
    let mut out = String::new();
    let mut pos = 0;
    let mut in_cell = false;
    while pos < html.len() {
        let rest = &html[pos..];
        if rest.starts_with("<!--") {
            pos += rest.find("-->").map_or(rest.len(), |end| end + 3);
            continue;
        }
        if rest.starts_with('<') {
            let mut quote = None;
            let end = rest.char_indices().skip(1).find_map(|(at, ch)| {
                if let Some(current) = quote {
                    if ch == current {
                        quote = None;
                    }
                } else if ch == '\'' || ch == '"' {
                    quote = Some(ch);
                } else if ch == '>' {
                    return Some(at);
                }
                None
            });
            let Some(end) = end else {
                out.push_str(rest);
                break;
            };
            let body = rest[1..end].trim();
            let closing = body.starts_with('/');
            let name = body
                .trim_start_matches('/')
                .split(|c: char| c.is_whitespace() || c == '/')
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();
            pos += end + 1;
            if !closing && matches!(name.as_str(), "script" | "style" | "head") {
                let close = format!("</{name}");
                if let Some(at) = lower[pos..].find(&close) {
                    pos += at;
                } else {
                    break;
                }
                continue;
            }
            match (closing, name.as_str()) {
                (false, "td" | "th") => in_cell = true,
                (true, "td" | "th") => {
                    in_cell = false;
                    out.push('\t');
                }
                (true, "tr") => {
                    if out.ends_with('\t') {
                        out.pop();
                    }
                    out.push('\n');
                }
                (false, "br") => out.push('\n'),
                (false, "img") => out.push_str("[图片]"),
                (
                    true,
                    "p" | "div" | "li" | "blockquote" | "pre" | "h1" | "h2" | "h3" | "h4" | "h5"
                    | "h6",
                ) => out.push(if in_cell { ' ' } else { '\n' }),
                _ => {}
            }
            continue;
        }
        if rest.starts_with('&') {
            if let Some(end) = rest
                .char_indices()
                .take(13)
                .find_map(|(at, ch)| (ch == ';').then_some(at))
            {
                let entity = &rest[1..end];
                let decoded = decode_entity(entity).or_else(|| {
                    let number = entity
                        .strip_prefix("#x")
                        .or_else(|| entity.strip_prefix("#X"))
                        .and_then(|value| u32::from_str_radix(value, 16).ok())
                        .or_else(|| {
                            entity
                                .strip_prefix('#')
                                .and_then(|value| value.parse().ok())
                        });
                    number.and_then(char::from_u32)
                });
                if let Some(ch) = decoded {
                    out.push(ch);
                    pos += end + 1;
                    continue;
                }
            }
        }
        let ch = rest.chars().next().expect("nonempty HTML remainder");
        out.push(ch);
        pos += ch.len_utf8();
    }
    out
}

/// 把第 `index` 块替换为 `new_html`，返回整篇新内容。
/// HTML：原串 in-place 替换该块 inner（非破坏）；text：替块后 join("\n")。越界返回 Err。
/// 审计修复（2026-07-26 · D10）：不再 html_escape，new_html 保留内联样式（已由前端 sanitize）。
pub fn replace_block(
    content: &str,
    is_html: bool,
    index: usize,
    new_html: &str,
) -> Result<String, String> {
    let blocks = parse_blocks(content, is_html);
    if index >= blocks.len() {
        return Err(format!(
            "block_index {index} 越界（当前共 {} 块）",
            blocks.len()
        ));
    }
    if is_html {
        let (s, e) = blocks[index]
            .inner_span
            .ok_or_else(|| "HTML 块缺 inner 区间".to_string())?;
        Ok(format!("{}{}{}", &content[..s], new_html, &content[e..]))
    } else {
        let mut texts: Vec<String> = blocks.into_iter().map(|b| b.text).collect();
        texts[index] = new_html.to_string();
        Ok(texts.join("\n"))
    }
}

#[cfg(test)]
mod reading_projection_tests {
    // covers: BR-AG-DATA-005
    #[test]
    fn reading_projection_preserves_order_cells_entities_and_avoids_nested_duplicates() {
        let html = "<head><style>hidden</style></head><p data-block-id='p' title='a > b'>甲&amp;&#x1F642;</p><!-- skip --><table><tr><th><p>名称</p></th><th><p>预算</p></th></tr><tr><td><p>Q7</p></td><td><p>81000</p></td></tr></table><ul><li><p>末尾</p></li></ul><script>unsafe</script>";
        let snapshot = super::snapshot_from_content(
            "a.html",
            html,
            "test",
            None,
            Some("html"),
            None,
            None,
            None,
            None,
        );
        let reading = super::reading_text(&snapshot);
        assert!(reading.starts_with("甲&🙂\n"));
        assert!(reading.contains("名称 \t预算 \nQ7 \t81000 \n"));
        assert_eq!(reading.matches("81000").count(), 1);
        assert_eq!(reading.matches("末尾").count(), 1);
        assert!(
            !reading.contains("hidden")
                && !reading.contains("unsafe")
                && !reading.contains("data-block-id")
        );
    }
}

fn strip_tags(s: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

/// 审计修复（2026-07-25 · A+D）：新增 ul/ol 容器块识别，列表容器在 project_blocks 中以空文本占位。
fn is_html_block_tag(tag: &str) -> bool {
    matches!(
        tag,
        "p" | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "li"
            | "blockquote"
            | "pre"
            | "div"
            | "td"
            | "th"
            | "ul"
            | "ol"
            | "table"
    )
}

/// 扫描 HTML 块级元素，记录 inner 字节区间 + strip 文本。
///
/// `data-block-id` 是稳定锚，有则保留；普通 `<p>/<h1>/...` 没有稳定锚时仍投影为 block_index，
/// 避免 HTML 编辑模型在 AG 侧变成空块，进而让模型只依赖历史上下文。
fn parse_html_blocks(html: &str) -> Vec<DocBlock> {
    let mut blocks = Vec::new();
    let mut search = 0usize;
    let lower = html.to_ascii_lowercase();
    while let Some(rel_lt) = lower[search..].find('<') {
        let open_lt = search + rel_lt;
        let name_start = open_lt + 1;
        let Some(first) = lower[name_start..].chars().next() else {
            break;
        };
        if matches!(first, '/' | '!' | '?') {
            search = name_start + first.len_utf8();
            continue;
        }
        let Some(rel_gt) = lower[open_lt..].find('>') else {
            break;
        };
        let open_gt = open_lt + rel_gt;
        let name_end = lower[name_start..open_gt]
            .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
            .map(|p| name_start + p)
            .unwrap_or(open_gt);
        let tag = lower[name_start..name_end].trim();
        if !is_html_block_tag(tag) {
            search = open_gt + 1;
            continue;
        }
        let open_tag = &html[open_lt..=open_gt];
        if open_tag.trim_end().ends_with("/>") {
            search = open_gt + 1;
            continue;
        }
        let inner_start = open_gt + 1;
        let close = format!("</{tag}>");
        let Some(rel_close) = lower[inner_start..].find(&close) else {
            search = inner_start;
            continue;
        };
        let inner_end = inner_start + rel_close;
        let inner = &html[inner_start..inner_end];
        blocks.push(DocBlock {
            block_id: attr_value(open_tag, "data-block-id"),
            block_type: Some(tag.to_string()),
            text: strip_tags(inner).trim().to_string(),
            inner_span: Some((inner_start, inner_end)),
        });
        search = inner_end + close.len();
    }
    blocks
}

/// 按 **data-block-id 稳定锚** 替换块 inner（HTML）。id 未找到 → Err（锚失效，文档已变化），
/// 不产错位 diff（对齐 A-DOC-M-T-01 blockId 锚 / BR-DE-DATA-001 锚失效不误改）。
pub fn replace_block_by_id(
    content: &str,
    block_id: &str,
    new_html: &str,
) -> Result<String, String> {
    let blk = parse_html_blocks(content)
        .into_iter()
        .find(|b| b.block_id.as_deref() == Some(block_id))
        .ok_or_else(|| {
            format!(
                "锚失效：未找到 data-block-id={block_id} 的块（文档可能已变化，请重新获取块列表）"
            )
        })?;
    let (s, e) = blk
        .inner_span
        .ok_or_else(|| "块缺 inner 区间".to_string())?;
    Ok(format!("{}{}{}", &content[..s], new_html, &content[e..]))
}

// —— 样式投影（PI-ED-11 后端件 · A-AG-I-P-01 §4.1 / schema 单一来源 A-ED-M-T-01 §5.1） ——

/// 字符 mark 段：range 在**块内纯文本平面**（与 ED 字符锚同基准，BR-ED-DATA-004）。
#[derive(Debug, Clone, PartialEq)]
pub struct MarkSpan {
    pub mark_type: String,
    pub range: (usize, usize),
    pub attrs: Option<serde_json::Value>,
}

/// 内联标签 → mark 类型映射（TipTap 序列化 HTML 的规整子集；span 依 style 判 textStyle）。
fn mark_of_tag(tag: &str, attrs_str: &str) -> Option<(String, Option<serde_json::Value>)> {
    match tag {
        "strong" | "b" => Some(("bold".into(), None)),
        "em" | "i" => Some(("italic".into(), None)),
        "u" => Some(("underline".into(), None)),
        "s" | "del" | "strike" => Some(("strike".into(), None)),
        "mark" => Some(("highlight".into(), None)),
        "sub" => Some(("subscript".into(), None)),
        "sup" => Some(("superscript".into(), None)),
        "code" => Some(("code".into(), None)),
        "a" => {
            let href = attr_value(attrs_str, "href");
            Some((
                "link".into(),
                href.map(|h| serde_json::json!({ "href": h })),
            ))
        }
        "span" => {
            let style = attr_value(attrs_str, "style")?;
            let mut o = serde_json::Map::new();
            for kv in style.split(';') {
                let (k, v) = kv.split_once(':')?;
                match k.trim() {
                    "color" => {
                        o.insert("color".into(), v.trim().into());
                    }
                    "font-size" => {
                        o.insert("fontSize".into(), v.trim().into());
                    }
                    "font-family" => {
                        o.insert("fontFamily".into(), v.trim().into());
                    }
                    _ => {}
                }
            }
            if o.is_empty() {
                None
            } else {
                Some(("textStyle".into(), Some(serde_json::Value::Object(o))))
            }
        }
        _ => None,
    }
}

/// 取属性串中 `name="…"` / `name='…'` 的值（dev 级扫描，属性名大小写不敏感）。
fn attr_value(attrs_str: &str, name: &str) -> Option<String> {
    let lower = attrs_str.to_ascii_lowercase();
    let pat = name.to_ascii_lowercase();
    let mut search = 0usize;
    while let Some(rel) = lower[search..].find(&pat) {
        let name_start = search + rel;
        let name_end = name_start + pat.len();
        let before_ok = lower[..name_start]
            .chars()
            .next_back()
            .map(|c| c.is_whitespace() || c == '<')
            .unwrap_or(true);
        let after = lower[name_end..].trim_start();
        if before_ok && after.starts_with('=') {
            let eq = name_end + lower[name_end..].find('=')? + 1;
            let value_start = eq
                + lower[eq..]
                    .chars()
                    .take_while(|c| c.is_whitespace())
                    .map(char::len_utf8)
                    .sum::<usize>();
            let quote = attrs_str[value_start..].chars().next()?;
            if quote == '"' || quote == '\'' {
                let start = value_start + quote.len_utf8();
                let end_rel = attrs_str[start..].find(quote)?;
                return Some(attrs_str[start..start + end_rel].to_string());
            }
        }
        search = name_end;
    }
    None
}

/// 有限 HTML 实体解码（只接受当前投影所需的常用集）。
fn decode_entity(ent: &str) -> Option<char> {
    match ent {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "#39" | "apos" => Some('\''),
        "nbsp" => Some('\u{a0}'),
        _ => None,
    }
}

/**
 * @GOV
 * codes:    BR-ED-DATA-005-QUERY-ED-ED-EDIT-009
 * type:     QUERY
 * chain:    ED-EDIT
 * rules:    BR-ED-DATA-005
 * term_ref: TERM-ED-006, TERM-ED-004
 * boundary: in=blockInnerHtml | out=plainTextWithMarkSpans(offsetsOnInBlockPlainTextPlane)
 */
/// 样式投影提取（BR-ED-DATA-005 读侧后端承载）：块 inner HTML → 纯文本 + mark 段（range 在块内
/// 纯文本平面，与 ED 字符锚同基准）。栈式扫描 TipTap 规整 HTML（嵌套标签配对、有限实体解码、
/// void 标签 1 字符占位对齐叶子 pos）；本模块 dev 级定位（畸形 HTML 不保证）已在模块头声明。
pub fn extract_marks(inner: &str) -> (String, Vec<MarkSpan>) {
    let mut text = String::new();
    let mut offset = 0usize; // CharAnchor：UTF-16 code unit（与 JS String.length / ProseMirror 对齐）
    let mut spans: Vec<MarkSpan> = Vec::new();
    // 开标签栈：(tag, mark_type/attrs 可空, 起始偏移)
    let mut stack: Vec<(String, Option<(String, Option<serde_json::Value>)>, usize)> = Vec::new();
    let bytes: Vec<char> = inner.chars().collect();
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i];
        if c == '<' {
            // 收集到 '>'。
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() && bytes[j] != '>' {
                j += 1;
            }
            let tag_body: String = bytes[start..j.min(bytes.len())].iter().collect();
            i = (j + 1).min(bytes.len());
            let closing = tag_body.starts_with('/');
            let self_closing = tag_body.ends_with('/');
            let name_src = tag_body.trim_start_matches('/');
            let name_end = name_src
                .find(|ch: char| ch.is_whitespace() || ch == '/')
                .unwrap_or(name_src.len());
            let name = name_src[..name_end].to_ascii_lowercase();
            if closing {
                // 弹出最近同名开标签，产出 mark 段（空区间不产）。
                if let Some(pos) = stack.iter().rposition(|(t, _, _)| *t == name) {
                    let (_, mark, from) = stack.remove(pos);
                    if let Some((mt, attrs)) = mark {
                        if offset > from {
                            spans.push(MarkSpan {
                                mark_type: mt,
                                range: (from, offset),
                                attrs,
                            });
                        }
                    }
                }
            } else if matches!(name.as_str(), "br" | "img" | "hr") || self_closing {
                // void/自闭合 → 1 字符占位（对齐 ED 叶子内联 pos 语义）。
                text.push('\u{fffc}');
                offset += 1;
            } else {
                let attrs_str = &name_src[name_end..];
                stack.push((name.clone(), mark_of_tag(&name, attrs_str), offset));
            }
        } else if c == '&' {
            // 实体：& 到 ';'（最长 6 字符），解码失败原样输出 '&'。
            let mut j = i + 1;
            while j < bytes.len() && j - i <= 6 && bytes[j] != ';' {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == ';' {
                let ent: String = bytes[i + 1..j].iter().collect();
                if let Some(ch) = decode_entity(&ent) {
                    text.push(ch);
                    offset += ch.len_utf16();
                    i = j + 1;
                    continue;
                }
            }
            text.push('&');
            offset += 1;
            i += 1;
        } else {
            text.push(c);
            offset += c.len_utf16();
            i += 1;
        }
    }
    (text, spans)
}

/// 块开标签的呈现属性投影（blockAttrs）：style 中 text-align（TipTap TextAlign 序列化形态）。
pub fn extract_block_attrs(open_tag: &str) -> serde_json::Value {
    let mut o = serde_json::Map::new();
    if let Some(style) = attr_value(open_tag, "style") {
        for kv in style.split(';') {
            if let Some((k, v)) = kv.split_once(':') {
                if k.trim() == "text-align" {
                    o.insert("textAlign".into(), v.trim().into());
                }
            }
        }
    }
    serde_json::Value::Object(o)
}

fn is_cjk(c: char) -> bool {
    matches!(
        c as u32,
        0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0xF900..=0xFAFF
    )
}

fn is_common_identity_term(term: &str) -> bool {
    matches!(
        term,
        "一个"
            | "这个"
            | "那个"
            | "他们"
            | "我们"
            | "自己"
            | "故事"
            | "内容"
            | "标题"
            | "当前"
            | "文档"
            | "于是"
            | "然后"
            | "开始"
            | "回来"
            | "看着"
            | "觉得"
            | "知道"
            | "没有"
            | "说道"
            | "起来"
    )
}

fn push_cjk_bigrams(terms: &mut BTreeSet<String>, run: &str) {
    let chars: Vec<char> = run.chars().collect();
    for pair in chars.windows(2) {
        let term: String = pair.iter().collect();
        if !is_common_identity_term(&term) {
            terms.insert(term);
        }
    }
}

fn identity_terms(content: &str) -> BTreeSet<String> {
    let plain = strip_tags(content);
    let mut terms = BTreeSet::new();
    let mut cjk_run = String::new();
    let mut ascii_run = String::new();

    let flush_cjk = |terms: &mut BTreeSet<String>, run: &mut String| {
        if !run.is_empty() {
            push_cjk_bigrams(terms, run);
            run.clear();
        }
    };
    let flush_ascii = |terms: &mut BTreeSet<String>, run: &mut String| {
        if run.len() >= 3 {
            terms.insert(run.to_ascii_lowercase());
        }
        run.clear();
    };

    for c in plain.chars() {
        if is_cjk(c) {
            flush_ascii(&mut terms, &mut ascii_run);
            cjk_run.push(c);
        } else if c.is_ascii_alphanumeric() {
            flush_cjk(&mut terms, &mut cjk_run);
            ascii_run.push(c);
        } else {
            flush_cjk(&mut terms, &mut cjk_run);
            flush_ascii(&mut terms, &mut ascii_run);
        }
    }
    flush_cjk(&mut terms, &mut cjk_run);
    flush_ascii(&mut terms, &mut ascii_run);
    terms
}

/// 整篇替换的主题/实体守卫。
///
/// 只拦截“长文档 → 长文档”且新旧身份词重合极低的情况；短文、空文、新建文档不在这里拦截，
/// 仍由 operation/目标集/基线校验负责。该守卫不是语义审核器，只是防历史上下文污染导致的错文档整篇替换。
pub fn rewrite_preserves_document_identity(before: &str, after: &str) -> bool {
    if before.trim().chars().count() < 60 || after.trim().chars().count() < 60 {
        return true;
    }
    let before_terms = identity_terms(before);
    let after_terms = identity_terms(after);
    let denominator = before_terms.len().min(after_terms.len());
    if denominator < 12 {
        return true;
    }
    let overlap = before_terms
        .iter()
        .filter(|term| after_terms.contains(*term))
        .count();
    overlap * 100 >= denominator * 12
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_blocks_split_by_editor_lines_including_empty_blocks() {
        let b = block_texts("第一行\n\n第三行", false);
        assert_eq!(b, vec!["第一行", "", "第三行"]);
    }

    // covers: BR-DOC-DATA-002, X-FORBID-004
    // AG-DOCX-RO-01：原生 .docx（非 b-docx）可只读读取——复用 DOC 转换链，
    // editable_kind≠"bdocx"、source=doc_convert_readonly、capabilities=[]（无 replace_node）。
    #[test]
    fn native_docx_reads_as_readonly_projection_without_bdocx_capabilities() {
        use crate::doc::edit_chain::html_to_docx;
        if crate::doc::pandoc_runtime::pandoc_available() {
            let root = std::env::temp_dir().join(format!("ag-native-docx-{}", std::process::id()));
            let _ = std::fs::create_dir_all(&root);
            let docx_bytes =
                html_to_docx("<h1>第二章</h1><p>正文内容</p>", None).expect("生成测试 docx");
            std::fs::write(root.join("原版.docx"), &docx_bytes).expect("写入原生 docx");

            let snapshot =
                read_editable_snapshot(&root, "原版.docx", &root.join("原版.docx"), None, None)
                    .expect("原生 docx 应可只读读取");
            assert_ne!(snapshot.editable_kind, "bdocx");
            assert_eq!(snapshot.source, "doc_convert_readonly");
            assert_eq!(snapshot.content_format, "html");
            // 只读投影：所有块无 replace_node 能力。
            assert!(
                snapshot
                    .blocks
                    .iter()
                    .all(|b| !b.capabilities.contains(&"replace_node".to_string())),
                "原生 docx 只读投影不得暴露 replace_node"
            );
            assert!(
                snapshot.blocks.iter().any(|b| b.text.contains("第二章")),
                "转换内容应包含正文"
            );
            let _ = std::fs::remove_dir_all(root);
        }
    }

    #[test]
    fn text_replace_block_preserves_editor_line_granularity() {
        let out = replace_block("a\n\nc", false, 2, "C").unwrap();
        assert_eq!(out, "a\n\nC");
    }

    #[test]
    fn html_blocks_parsed_by_block_id_with_stripped_text() {
        let html = r#"<p data-block-id="b1">你好<strong>世界</strong></p><h1 data-block-id="b2">标题</h1>"#;
        assert!(is_html(html, Some("html")));
        let texts = block_texts(html, true);
        assert_eq!(texts, vec!["你好世界", "标题"]);
    }

    #[test]
    fn markdown_open_snapshot_uses_explicit_html_content_format() {
        let root = std::env::temp_dir().join(format!("ag-markdown-html-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&root);
        let ctx = AgentReadContext::new(
            &root,
            vec![OpenDocSnapshot {
                rel_path: "notes.md".to_string(),
                content: "<h1>标题</h1><p>正文</p>".to_string(),
                baseline: None,
                edit_format: "markdown".to_string(),
                content_format: "html".to_string(),
                source: "open_editor_session".to_string(),
                dirty: true,
                session_version: 2,
                content_hash: "h2".to_string(),
                captured_at: 1,
                block_snapshots: Vec::new(),
                table_snapshots: Vec::new(),
                image_snapshots: Vec::new(),
            }],
        )
        .unwrap();
        let snapshot = ctx.snapshot("notes.md").unwrap();
        assert_eq!(snapshot.content_format, "html");
        assert_eq!(snapshot.blocks.len(), 2);
        assert_eq!(snapshot.blocks[0].text, "标题");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn active_html_uses_prosemirror_hierarchy_instead_of_flat_html_scanner() {
        let root =
            std::env::temp_dir().join(format!("ag-structured-blocks-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&root);
        let block = |id: &str, parent: Option<&str>, text: &str| AgentEditableBlockSnapshot {
            block_id: id.to_string(),
            block_index: if id == "p-1" { 2 } else { 4 },
            block_type: "p".to_string(),
            node_type: "paragraph".to_string(),
            role: "content".to_string(),
            parent_block_id: parent.map(str::to_string),
            depth: 2,
            ordinal: 0,
            text: text.to_string(),
            attrs: serde_json::json!({"blockId": id}),
            marks: Vec::new(),
            node_json: serde_json::json!({
                "type": "paragraph",
                "attrs": {"blockId": id},
                "content": [{"type": "text", "text": text}]
            }),
            fingerprint: format!("fp-{id}"),
            capabilities: vec!["replace_node".to_string(), "style_ops".to_string()],
        };
        let ctx = AgentReadContext::new(
            &root,
            vec![OpenDocSnapshot {
                rel_path: "a.docx".to_string(),
                content: "<ul><li><p>第一条</p></li><li><p>第二条</p></li></ul>".to_string(),
                baseline: Some(1),
                edit_format: "html".to_string(),
                content_format: "html".to_string(),
                source: "active_editor".to_string(),
                dirty: true,
                session_version: 7,
                content_hash: "h7".to_string(),
                captured_at: 1,
                block_snapshots: vec![
                    block("p-1", Some("item-1"), "第一条"),
                    block("p-2", Some("item-2"), "第二条"),
                ],
                table_snapshots: Vec::new(),
                image_snapshots: Vec::new(),
            }],
        )
        .unwrap();

        let snapshot = ctx.snapshot("a.docx").unwrap();
        assert_eq!(snapshot.blocks.len(), 2);
        assert_eq!(snapshot.blocks[0].text, "第一条");
        assert_eq!(snapshot.blocks[1].text, "第二条");
        assert_eq!(
            snapshot.blocks[0].parent_block_id.as_deref(),
            Some("item-1")
        );
        assert_eq!(
            snapshot.blocks[0].capabilities,
            vec!["replace_node", "style_ops"]
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn active_html_without_structured_blocks_is_rejected() {
        let root =
            std::env::temp_dir().join(format!("ag-structured-missing-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&root);
        let ctx = AgentReadContext::new(
            &root,
            vec![OpenDocSnapshot {
                rel_path: "a.html".to_string(),
                content: "<p data-block-id=\"p-1\">正文</p>".to_string(),
                baseline: None,
                edit_format: "html".to_string(),
                content_format: "html".to_string(),
                source: "active_editor".to_string(),
                dirty: true,
                session_version: 1,
                content_hash: "h1".to_string(),
                captured_at: 1,
                block_snapshots: Vec::new(),
                table_snapshots: Vec::new(),
                image_snapshots: Vec::new(),
            }],
        )
        .unwrap();

        assert_eq!(
            ctx.snapshot("a.html").unwrap_err().code,
            "AG_STRUCTURED_SNAPSHOT_REQUIRED"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn html_blocks_without_block_id_fall_back_to_block_index() {
        let html = r#"<p>农夫发现了一条冻僵的蛇。</p><h2>蛇<strong>苏醒</strong></h2>"#;
        let blocks = parse_blocks(html, true);

        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].block_id, None);
        assert_eq!(blocks[0].block_type.as_deref(), Some("p"));
        assert_eq!(blocks[0].text, "农夫发现了一条冻僵的蛇。");
        assert_eq!(blocks[1].block_id, None);
        assert_eq!(blocks[1].block_type.as_deref(), Some("h2"));
        assert_eq!(blocks[1].text, "蛇苏醒");
    }

    #[test]
    fn project_blocks_uses_single_block_shape_with_marks() {
        let html =
            r#"<p data-block-id="b1" style="text-align: center">你好<strong>世界</strong></p>"#;
        let blocks = project_blocks(html, true);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_id.as_deref(), Some("b1"));
        assert_eq!(blocks[0].block_index, 0);
        assert_eq!(blocks[0].block_type.as_deref(), Some("p"));
        assert_eq!(blocks[0].text, "你好世界");
        assert_eq!(blocks[0].marks[0]["type"], "bold");
        assert_eq!(blocks[0].block_attrs["textAlign"], "center");
    }

    #[test]
    fn extract_marks_uses_utf16_offsets_like_editor_char_anchor() {
        let (text, marks) = extract_marks("A<strong>😀B</strong>C");
        assert_eq!(text, "A😀BC");
        assert_eq!(marks.len(), 1);
        assert_eq!(marks[0].range, (1, 4));
    }

    #[test]
    fn html_replace_block_is_inplace_nonlossy() {
        let html = r#"<p data-block-id="b1">旧</p><h1 data-block-id="b2">标题</h1>"#;
        let out = replace_block(html, true, 0, "新内容").unwrap();
        // 仅 b1 inner 被替换，b2 与标签结构原样保留。
        assert_eq!(
            out,
            r#"<p data-block-id="b1">新内容</p><h1 data-block-id="b2">标题</h1>"#
        );
    }

    #[test]
    fn html_replace_block_without_block_id_uses_index_and_preserves_tags() {
        let html = r#"<p>旧段落</p><p>保留段落</p>"#;
        let out = replace_block(html, true, 0, "新段落").unwrap();

        assert_eq!(out, r#"<p>新段落</p><p>保留段落</p>"#);
    }

    #[test]
    fn html_replace_preserves_sanitized_inline_html() {
        let html = r#"<p data-block-id="b1">x</p>"#;
        let out = replace_block(html, true, 0, "a<strong>b</strong>&amp;c").unwrap();
        assert_eq!(
            out,
            r#"<p data-block-id="b1">a<strong>b</strong>&amp;c</p>"#
        );
    }

    // covers: BR-ED-DATA-005
    // 样式投影提取：range 在块内纯文本平面（与字符锚同基准）；嵌套配对；span style→textStyle attrs；
    // 实体解码偏移正确；void 标签 1 字符占位。
    #[test]
    fn extract_marks_offsets_on_plain_text_plane() {
        let (text, spans) =
            extract_marks(r#"普通<strong>加粗</strong><span style="color: #ff0000">红字</span>"#);
        assert_eq!(text, "普通加粗红字");
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].mark_type, "bold");
        assert_eq!(spans[0].range, (2, 4));
        assert_eq!(spans[1].mark_type, "textStyle");
        assert_eq!(spans[1].range, (4, 6));
        assert_eq!(spans[1].attrs.as_ref().unwrap()["color"], "#ff0000");
    }

    // covers: BR-ED-DATA-005
    #[test]
    fn extract_marks_nested_entities_and_void() {
        // 嵌套 <strong><em>…</em></strong> + 实体 &amp;（1 字符）+ <br/>（占位 1 字符）
        let (text, spans) = extract_marks("a<strong>b<em>c</em></strong>&amp;<br/>d");
        assert_eq!(text, "abc&\u{fffc}d");
        // em 段 (2,3)、strong 段 (1,3)
        assert!(spans.contains(&MarkSpan {
            mark_type: "italic".into(),
            range: (2, 3),
            attrs: None
        }));
        assert!(spans.contains(&MarkSpan {
            mark_type: "bold".into(),
            range: (1, 3),
            attrs: None
        }));
    }

    // covers: BR-ED-DATA-005
    #[test]
    fn extract_block_attrs_text_align() {
        let attrs = extract_block_attrs(r#"<p data-block-id="b1" style="text-align: center">"#);
        assert_eq!(attrs["textAlign"], "center");
        assert_eq!(extract_block_attrs("<p>"), serde_json::json!({}));
    }

    #[test]
    fn out_of_range_errors() {
        assert!(replace_block("a\n\nb", false, 5, "x").is_err());
        assert!(replace_block(r#"<p data-block-id="b1">a</p>"#, true, 3, "x").is_err());
    }

    #[test]
    fn block_id_captured_and_anchored_replace() {
        let html = r#"<p data-block-id="b1">旧</p><h1 data-block-id="b2">标题</h1>"#;
        let blocks = parse_blocks(html, true);
        assert_eq!(blocks[0].block_id.as_deref(), Some("b1"));
        assert_eq!(blocks[1].block_id.as_deref(), Some("b2"));
        // 按 id 锚定替换（与位置无关）。
        let out = replace_block_by_id(html, "b2", "新标题").unwrap();
        assert_eq!(
            out,
            r#"<p data-block-id="b1">旧</p><h1 data-block-id="b2">新标题</h1>"#
        );
    }

    #[test]
    fn replace_by_unknown_id_is_anchor_lost() {
        let r = replace_block_by_id(r#"<p data-block-id="b1">x</p>"#, "gone", "y");
        assert!(r.unwrap_err().contains("锚失效"));
    }

    #[test]
    fn rewrite_identity_guard_rejects_unrelated_story_replacement() {
        let before = "农夫在寒冷的冬天回家，路上看见一条冻僵的蛇。他心生怜悯，把蛇放进怀里，用体温温暖它。蛇渐渐苏醒，却露出了冷酷的本性，猛地咬了农夫一口。农夫临死前明白，对恶人施恩不会改变恶人的本性。";
        let after = "小马驮着一袋麦子去磨坊，来到河边时不敢过河。老牛说水很浅，松鼠却说水很深。小马拿不定主意，只好回家问妈妈。妈妈告诉他要自己试一试，小马终于鼓起勇气过了河。";

        assert!(!rewrite_preserves_document_identity(before, after));
    }

    #[test]
    fn rewrite_identity_guard_allows_same_story_cleanup() {
        let before = "农夫在寒冷的冬天回家，路上看见一条冻僵的蛇。他心生怜悯，把蛇放进怀里，用体温温暖它。蛇渐渐苏醒，却露出了冷酷的本性，猛地咬了农夫一口。农夫临死前明白，对恶人施恩不会改变恶人的本性。";
        let after = "寒冷的冬日，农夫在回家途中发现一条冻僵的蛇。他怜悯蛇的遭遇，把它放在怀里，用自己的体温让蛇苏醒。可是蛇恢复后仍保持冷酷本性，狠狠咬了农夫。农夫临终才明白，不能指望恶人的本性因施恩而改变。";

        assert!(rewrite_preserves_document_identity(before, after));
    }
}
