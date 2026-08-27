//! 从应用组合根迁入的模块运行时/路由；保持原 command 与状态机行为等价。

use crate::*;

/// ref.* command 锚参数（A-REF-I-P-01 §2）。
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RefAnchorParam {
    workspace_id: String,
    rel_path: String,
    block_ids: Option<Vec<String>>,
}

pub(crate) fn parse_ref_anchor_param(request: &CommandRequest) -> Result<RefAnchorParam, IpcError> {
    let p = request
        .params
        .get("anchor")
        .and_then(|v| v.as_object())
        .ok_or_else(|| IpcError::new("REF_PARAM_MISSING", "缺少 anchor 参数", false, false))?;
    let workspace_id = p
        .get("workspaceId")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let rel_path = p
        .get("relPath")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if rel_path.is_empty() {
        return Err(IpcError::new(
            "REF_PARAM_MISSING",
            "缺少 anchor.relPath",
            false,
            false,
        ));
    }
    let block_ids = p.get("blockIds").and_then(|v| v.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|x| x.as_str().map(String::from))
            .collect::<Vec<_>>()
    });
    Ok(RefAnchorParam {
        workspace_id,
        rel_path,
        block_ids,
    })
}

pub(crate) fn to_reference_anchor(param: &RefAnchorParam) -> reference::resolve::ReferenceAnchor {
    let target = crate::types::DocRef {
        workspace_id: param.workspace_id.clone(),
        rel_path: param.rel_path.clone(),
        doc_type: reference::resolve::infer_doc_type(&param.rel_path),
    };
    match &param.block_ids {
        Some(ids) if !ids.is_empty() => reference::resolve::ReferenceAnchor::Block {
            target,
            block_ids: ids.clone(),
        },
        _ => reference::resolve::ReferenceAnchor::Path(target),
    }
}

/** 可读引用文档过滤（F7）：@ 候选仅可读文档，资源文件不进入引用候选。 */
pub(crate) fn is_readable_reference_doc(rel_path: &str) -> bool {
    let ext = reference::resolve::infer_doc_type(rel_path);
    matches!(
        ext.as_str(),
        "md" | "markdown" | "docx" | "doc" | "html" | "htm" | "txt" | "rtf"
    )
}

/**
 * @GOV
 * codes:    BR-REF-DATA-001-API-REF-REF-INJECT-001
 * type:     API
 * chain:    REF-INJECT
 * rules:    BR-REF-DATA-001
 * term_ref: TERM-REF-001
 * boundary: in=workspaceQuery | out=referenceCandidateList(scopedWithinWorkspaceReadableDocs)
 */
/// 路由 `ref.list_candidates`（A-REF-I-P-01 §2）：活动 workspace 内候选列举，限界 X-FORBID-004，
/// 仅可读文档（F7 过滤资源文件）；当前无前端消费者，契约闭合 + 供调试/未来消费方。
pub(crate) fn route_ref_list_candidates(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let active = active_ws(state)?;
    let docs = workspace::listing::list_docs(std::path::Path::new(&active.root_path))?;
    let query = request
        .params
        .get("query")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();
    let candidates: Vec<serde_json::Value> = docs
        .iter()
        .filter(|d| !d.is_dir && is_readable_reference_doc(&d.rel_path))
        .filter(|d| query.is_empty() || d.rel_path.to_lowercase().contains(&query))
        .map(|d| {
            serde_json::json!({
                "relPath": d.rel_path,
                "docType": d.doc_type,
            })
        })
        .collect();
    Ok(serde_json::Value::Array(candidates))
}

/**
 * @GOV
 * codes:    BR-REF-DATA-001-API-REF-REF-INJECT-002
 * type:     API
 * chain:    REF-INJECT
 * rules:    BR-REF-DATA-001
 * term_ref: TERM-REF-001
 * boundary: in=workspaceMentionTargetSelection | out=ReferenceAnchor(scopedReachableElseFailNoDangling)
 */
/// 路由 `ref.resolve_anchor`（A-REF-I-P-01 §2）：守界 + 目标可达判定 → 路径/块级锚；不可达明确失败。
pub(crate) fn route_ref_resolve_anchor(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let active = active_ws(state)?;
    let rel_path = str_param(request, "relPath", "REF_PARAM_MISSING")?;
    let block_ids = request
        .params
        .get("blockIds")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect::<Vec<_>>()
        });
    let root = std::path::Path::new(&active.root_path);
    // 先守界再判存在，避免越界路径触发外部文件系统访问。
    crate::workspace::resolve_in_workspace(root, &rel_path)?;
    let target_exists = root.join(&rel_path).exists();
    let anchor = reference::resolve::resolve_mention(&active, &rel_path, block_ids, target_exists)?;
    match anchor {
        reference::resolve::ReferenceAnchor::Path(target) => Ok(serde_json::json!({
            "kind": "path",
            "workspaceId": target.workspace_id,
            "relPath": target.rel_path,
            "docType": target.doc_type,
        })),
        reference::resolve::ReferenceAnchor::Block { target, block_ids } => Ok(serde_json::json!({
            "kind": "block",
            "workspaceId": target.workspace_id,
            "relPath": target.rel_path,
            "docType": target.doc_type,
            "blockIds": block_ids,
        })),
    }
}

/**
 * @GOV
 * codes:    BR-REF-VERIFY-001-API-REF-REF-INJECT-003
 * type:     API
 * chain:    REF-INJECT
 * rules:    BR-REF-VERIFY-001
 * term_ref: TERM-REF-001
 * boundary: in=referenceAnchor | out=validOrAnchorInvalid(conservativeNoInjection) | effect=IPC-ref.verify_anchor | delegate=reference::inject::verify_anchor
 */
/// 路由 `ref.verify_anchor`（A-REF-I-P-01 §2）：路径级判存在；块级在 command 路径无 LogicalState 快照
/// （OpenDocSnapshot 门禁归 AG 回合），保守按 Undetectable 处理（检测不可得 → 不注入）。
pub(crate) fn route_ref_verify_anchor(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let active = active_ws(state)?;
    let param = parse_ref_anchor_param(request)?;
    let root = std::path::Path::new(&active.root_path);
    crate::workspace::resolve_in_workspace(root, &param.rel_path)?;
    let path_exists = root.join(&param.rel_path).exists();
    let locate = match &param.block_ids {
        Some(ids) if !ids.is_empty() => crate::types::LocateResult::Undetectable,
        _ => crate::types::LocateResult::Located,
    };
    reference::inject::verify_anchor(&to_reference_anchor(&param), path_exists, locate)?;
    Ok(serde_json::json!({ "valid": true }))
}

/**
 * @GOV
 * codes:    BR-REF-DATA-002-API-REF-REF-INJECT-004, BR-REF-VERIFY-001-API-REF-REF-INJECT-004
 * type:     API
 * chain:    REF-INJECT
 * rules:    BR-REF-DATA-002, BR-REF-VERIFY-001
 * term_ref: TERM-REF-002
 * boundary: in=verifiedAnchorAndReadTimeContent | out=RefInjection(freshContentTraceableNoSelfExtend) | effect=IPC-ref.build_injection | delegate=reference::inject::verify_anchor
 */
/// 路由 `ref.build_injection`（A-REF-I-P-01 §2）：verify 通过后经 WS 读取**读取时点**内容组装
/// RefInjection（A-SYS-I-P-01 结构）。command 路径无 LogicalState 快照（已打开文档的引用消费归
/// AG 回合），内容经 workspace 读盘；块级锚同 verify 保守 Undetectable → 不注入。
pub(crate) fn route_ref_build_injection(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let active = active_ws(state)?;
    let param = parse_ref_anchor_param(request)?;
    let root = std::path::Path::new(&active.root_path);
    crate::workspace::resolve_in_workspace(root, &param.rel_path)?;
    let path_exists = root.join(&param.rel_path).exists();
    let locate = match &param.block_ids {
        Some(ids) if !ids.is_empty() => crate::types::LocateResult::Undetectable,
        _ => crate::types::LocateResult::Located,
    };
    let anchor = to_reference_anchor(&param);
    reference::inject::verify_anchor(&anchor, path_exists, locate)?;
    let content = std::fs::read_to_string(root.join(&param.rel_path)).map_err(|e| {
        IpcError::new(
            "REF_READ_FAILED",
            &format!("读取引用目标失败: {e}"),
            true,
            false,
        )
    })?;
    let block_suffix = match &param.block_ids {
        Some(ids) if !ids.is_empty() => format!("#{}", ids.join(",")),
        _ => String::new(),
    };
    let injection = reference::inject::build_injection(
        &anchor,
        content,
        format!("@{}{}", param.rel_path, block_suffix),
    );
    serde_json::to_value(injection).map_err(|e| {
        IpcError::new(
            "REF_SERIALIZE_FAILED",
            &format!("序列化注入载荷失败: {e}"),
            false,
            false,
        )
    })
}

#[cfg(test)]
mod ref_route_tests {
    use super::*;

    fn temp_root(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "binder-ref-route-{tag}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn state_with_ws(root: &std::path::Path) -> AppState {
        let mut conn = Connection::open_in_memory().unwrap();
        db::migrate(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO workspaces (workspace_id, root_path, is_active, opened_at) VALUES ('ws-ref', ?1, 1, 0)",
            [root.to_str().unwrap()],
        )
        .unwrap();
        AppState {
            db: Mutex::new(conn),
            registry: build_registry(),
            assist_terminals: Mutex::new(HashMap::new()),
            active_agent_turns: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn request(command: &str, params: serde_json::Value) -> CommandRequest {
        CommandRequest {
            command: command.to_string(),
            correlation_id: "ref-route".into(),
            params,
        }
    }

    // covers: BR-REF-DATA-001
    // 候选列举限 workspace 内且仅可读文档（F7 过滤资源文件）。
    #[test]
    fn list_candidates_filters_to_readable_docs() {
        let root = temp_root("candidates");
        std::fs::create_dir_all(root.join("notes")).unwrap();
        std::fs::write(root.join("notes/a.md"), "x").unwrap();
        std::fs::create_dir_all(root.join("assets")).unwrap();
        std::fs::write(root.join("assets/pic.png"), b"\x89PNG").unwrap();
        let state = state_with_ws(&root);

        let out = route_ref_list_candidates(
            &request("ref.list_candidates", serde_json::json!({})),
            &state,
        )
        .unwrap();
        let arr = out.as_array().unwrap();
        assert_eq!(arr.len(), 1, "资源文件不得进入引用候选");
        assert_eq!(arr[0]["relPath"], "notes/a.md");
        assert_eq!(arr[0]["docType"], "md");
        let _ = std::fs::remove_dir_all(&root);
    }

    // covers: BR-REF-DATA-001
    // resolve：可达路径级锚正确；越界拒绝；不可达明确失败不悬空；doc_type 按扩展名推断。
    #[test]
    fn resolve_anchor_scopes_rejects_escape_and_unreachable() {
        let root = temp_root("resolve");
        std::fs::create_dir_all(root.join("notes")).unwrap();
        std::fs::write(root.join("notes/a.md"), "x").unwrap();
        std::fs::write(root.join("notes/b.docx"), "x").unwrap();
        let state = state_with_ws(&root);

        let ok = route_ref_resolve_anchor(
            &request(
                "ref.resolve_anchor",
                serde_json::json!({ "relPath": "notes/b.docx" }),
            ),
            &state,
        )
        .unwrap();
        assert_eq!(ok["kind"], "path");
        assert_eq!(ok["docType"], "docx");

        let esc = route_ref_resolve_anchor(
            &request(
                "ref.resolve_anchor",
                serde_json::json!({ "relPath": "../../etc/passwd" }),
            ),
            &state,
        )
        .unwrap_err();
        assert_eq!(esc.code, "WS_PATH_ESCAPE");

        let un = route_ref_resolve_anchor(
            &request(
                "ref.resolve_anchor",
                serde_json::json!({ "relPath": "gone.md" }),
            ),
            &state,
        )
        .unwrap_err();
        assert_eq!(un.code, "REF_UNREACHABLE");
        let _ = std::fs::remove_dir_all(&root);
    }

    // covers: BR-REF-VERIFY-001
    // verify：路径存在通过；缺失拒绝；块级（command 路径无 LogicalState 快照）保守 Undetectable 拒绝。
    #[test]
    fn verify_anchor_rejects_missing_and_block_undetectable() {
        let root = temp_root("verify");
        std::fs::write(root.join("a.md"), "x").unwrap();
        let state = state_with_ws(&root);

        let ok = route_ref_verify_anchor(
            &request(
                "ref.verify_anchor",
                serde_json::json!({ "anchor": { "workspaceId": "ws-ref", "relPath": "a.md" } }),
            ),
            &state,
        )
        .unwrap();
        assert_eq!(ok["valid"], true);

        let missing = route_ref_verify_anchor(
            &request(
                "ref.verify_anchor",
                serde_json::json!({ "anchor": { "workspaceId": "ws-ref", "relPath": "gone.md" } }),
            ),
            &state,
        )
        .unwrap_err();
        assert_eq!(missing.code, "REF_ANCHOR_INVALID");

        let block = route_ref_verify_anchor(
            &request(
                "ref.verify_anchor",
                serde_json::json!({
                    "anchor": { "workspaceId": "ws-ref", "relPath": "a.md", "blockIds": ["b1"] }
                }),
            ),
            &state,
        )
        .unwrap_err();
        assert_eq!(block.code, "REF_ANCHOR_INVALID");
        let _ = std::fs::remove_dir_all(&root);
    }

    // covers: BR-REF-DATA-002
    // build_injection：路径级读取时点内容 + origin_label 可溯源；块级保守拒绝。
    #[test]
    fn build_injection_reads_fresh_content_and_traceable() {
        let root = temp_root("inject");
        std::fs::write(root.join("a.md"), "读取时点内容").unwrap();
        let state = state_with_ws(&root);

        let out = route_ref_build_injection(
            &request(
                "ref.build_injection",
                serde_json::json!({ "anchor": { "workspaceId": "ws-ref", "relPath": "a.md" } }),
            ),
            &state,
        )
        .unwrap();
        assert_eq!(out["content"], "读取时点内容");
        assert_eq!(out["originLabel"], "@a.md");

        let block = route_ref_build_injection(
            &request(
                "ref.build_injection",
                serde_json::json!({
                    "anchor": { "workspaceId": "ws-ref", "relPath": "a.md", "blockIds": ["b1"] }
                }),
            ),
            &state,
        )
        .unwrap_err();
        assert_eq!(block.code, "REF_ANCHOR_INVALID");
        let _ = std::fs::remove_dir_all(&root);
    }
}
