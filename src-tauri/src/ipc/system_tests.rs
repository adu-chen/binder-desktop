//! 从应用组合根迁入的模块运行时/路由；保持原 command 与状态机行为等价。

use crate::*;

#[cfg(test)]
mod edit_persistence_system_tests {
    use super::*;
    use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, MockRuntime};
    use tauri::{Manager, State};

    /// MockRuntime 的 command 必须保持生产名 `dispatch`，并复用生产注册表与
    /// `route_edit_persistence_command`；这里只替换 Wry 运行时，不复制业务路由。
    #[tauri::command(rename = "dispatch")]
    fn mock_dispatch(
        app: tauri::AppHandle<MockRuntime>,
        request: CommandRequest,
        state: State<'_, AppState>,
    ) -> CommandResponse {
        let correlation_id = request.correlation_id.clone();
        if let Err(error) = state.registry.validate(&request) {
            return CommandResponse {
                correlation_id,
                result: None,
                error: Some(error),
            };
        }
        let routed = route_edit_persistence_command(&request, &state, &app).unwrap_or_else(|| {
            // 快照路由不依赖桌面 Runtime；与生产 dispatch 一样直接调用真实处理器。
            match request.command.as_str() {
                "ed.persist_session_snapshot" => route_persist_session_snapshot(&request, &state),
                "ed.list_session_snapshots" => route_list_session_snapshots(&state),
                _ => Err(IpcError::new(
                    "IPC_ROUTE_MISSING",
                    "G5 持久化 harness 收到范围外 command",
                    false,
                    false,
                )),
            }
        });
        match routed {
            Ok(result) => CommandResponse {
                correlation_id,
                result: Some(result),
                error: None,
            },
            Err(error) => CommandResponse {
                correlation_id,
                result: None,
                error: Some(error),
            },
        }
    }

    fn temp_workspace(tag: &str, initial: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "binder_g5_persistence_{}_{}",
            std::process::id(),
            tag
        ));
        let _ = std::fs::remove_dir_all(&root);
        let document = root.join("notes/a.md");
        std::fs::create_dir_all(document.parent().unwrap()).unwrap();
        std::fs::write(&document, initial).unwrap();
        (root, document)
    }

    fn app_for(root: &std::path::Path) -> tauri::App<MockRuntime> {
        let mut conn = Connection::open_in_memory().unwrap();
        db::migrate(&mut conn).unwrap();
        workspace::lifecycle::open_workspace(&mut conn, &root.to_string_lossy()).unwrap();
        mock_builder()
            .manage(AppState {
                db: std::sync::Arc::new(Mutex::new(conn)),
                registry: build_registry(),
                assist_terminals: std::sync::Arc::new(Mutex::new(HashMap::new())),
                active_agent_turns: Arc::new(Mutex::new(HashMap::new())),
            })
            .invoke_handler(tauri::generate_handler![mock_dispatch])
            .build(mock_context(noop_assets()))
            .unwrap()
    }

    fn invoke(
        webview: &tauri::WebviewWindow<MockRuntime>,
        command: &str,
        params: serde_json::Value,
    ) -> serde_json::Value {
        let correlation_id = format!("g5-{command}");
        let response = get_ipc_response(
            webview,
            tauri::webview::InvokeRequest {
                cmd: "dispatch".into(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: "tauri://localhost".parse().unwrap(),
                body: serde_json::json!({
                    "request": {
                        "command": command,
                        "correlation_id": correlation_id,
                        "params": params
                    }
                })
                .into(),
                headers: Default::default(),
                invoke_key: tauri::test::INVOKE_KEY.to_string(),
            },
        )
        .unwrap_or_else(|error| panic!("Tauri IPC 拒绝 {command}: {error}"))
        .deserialize::<CommandResponse>()
        .unwrap();
        assert_eq!(response.correlation_id, correlation_id);
        if let Some(error) = response.error {
            panic!("{command} 返回 {}: {}", error.code, error.message);
        }
        response
            .result
            .unwrap_or_else(|| panic!("{command} 缺少 result"))
    }

    // covers: BR-ED-DATA-008, BR-WS-OBS-001, BR-WS-PERSIST-001
    #[test]
    fn text_snapshot_restores_baseline_and_saves_existing_file_through_ipc() {
        let (root, _) = temp_workspace("text_snapshot_baseline", "");
        let app = app_for(&root);
        let webview = tauri::WebviewWindowBuilder::new(&app, "snapshot-test", Default::default())
            .build()
            .unwrap();
        for (name, format, content_format, draft) in [
            ("plain.txt", "text", "text", "粘贴正文\n第二行"),
            ("note.md", "markdown", "text", "# Markdown 草稿"),
            ("page.html", "html_file", "html", "<p>HTML 草稿</p>"),
        ] {
            let rel_path = format!("notes/{name}");
            let path = root.join(&rel_path);
            std::fs::write(&path, "").unwrap();
            let opened = invoke(
                &webview,
                "ws.read_doc",
                serde_json::json!({"relPath": rel_path}),
            );
            let baseline = opened["baseline"].as_str().unwrap();
            invoke(
                &webview,
                "ed.persist_session_snapshot",
                serde_json::json!({"snapshot": {
                    "relPath": rel_path, "content": draft, "baseline": baseline,
                    "editFormat": format, "contentFormat": content_format,
                    "source": "open_editor_session", "dirty": true, "sessionVersion": 2,
                    "contentHash": "draft-hash", "capturedAt": 1
                }}),
            );
            let restored = invoke(&webview, "ed.list_session_snapshots", serde_json::json!({}));
            let snapshot = restored["snapshots"]
                .as_array()
                .unwrap()
                .iter()
                .find(|snapshot| snapshot["relPath"] == rel_path)
                .unwrap();
            assert_eq!(snapshot["content"], draft);
            assert_eq!(snapshot["baseline"], baseline);
            assert_eq!(snapshot["dirty"], true);
            assert_eq!(snapshot["editFormat"], format);
            let saved = invoke(
                &webview,
                "ws.save_doc",
                serde_json::json!({
                    "relPath": rel_path, "content": draft, "baseline": snapshot["baseline"], "override": false
                }),
            );
            assert_ne!(saved["baseline"], opened["baseline"]);
            assert_eq!(std::fs::read_to_string(path).unwrap(), draft);
        }
        drop(webview);
        drop(app);
        std::fs::remove_dir_all(root).unwrap();
    }

    fn generate_diff(
        webview: &tauri::WebviewWindow<MockRuntime>,
        before_snapshot: &str,
        after_content: &str,
    ) -> String {
        invoke(
            webview,
            "de.generate_diff",
            serde_json::json!({
                "relPath": "notes/a.md",
                "afterContent": after_content,
                "beforeSnapshot": before_snapshot,
                "originModule": "AG",
                "originTask": "g5-system-harness",
                "anchorLocate": "located"
            }),
        )["diff_id"]
            .as_str()
            .unwrap()
            .to_string()
    }

    // covers: BR-SYS-DATA-003
    // covers: BR-DE-STATE-001
    // covers: BR-WS-PERSIST-001
    // covers: BR-WS-OBS-001
    // Tauri Mock Webview → 生产注册/路由 → DE 接受 → WS 保存 → 真实 DiskState → 第二次 IPC 重开。
    #[test]
    fn accepted_draft_saves_to_disk_and_reopens_through_tauri_ipc() {
        let initial = "# 初始\n\n正文\n";
        let accepted = "# 已接受\n\n更新正文\n";
        let (root, document) = temp_workspace("accepted", initial);
        let app = app_for(&root);
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();

        let opened = invoke(
            &webview,
            "ws.read_doc",
            serde_json::json!({ "relPath": "notes/a.md" }),
        );
        assert_eq!(opened["content"], initial);
        let baseline = opened["baseline"].as_str().unwrap();
        let diff_id = generate_diff(&webview, baseline, accepted);

        invoke(
            &webview,
            "de.accept_draft_diff",
            serde_json::json!({ "diffId": diff_id, "applyContext": "current_editor" }),
        );
        invoke(
            &webview,
            "ws.save_doc",
            serde_json::json!({
                "relPath": "notes/a.md",
                "content": accepted,
                "baseline": baseline,
                "override": false
            }),
        );

        assert_eq!(std::fs::read_to_string(&document).unwrap(), accepted);
        let reopened = invoke(
            &webview,
            "ws.read_doc",
            serde_json::json!({ "relPath": "notes/a.md" }),
        );
        assert_eq!(reopened["content"], accepted);
        assert_ne!(reopened["baseline"], opened["baseline"]);

        let state = app.state::<AppState>();
        let conn = state.db.lock().unwrap();
        let (status, effect_status): (String, String) = conn
            .query_row(
                "SELECT status, effect_status FROM pending_diffs WHERE diff_id = ?1",
                [&diff_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(status, "accepted");
        assert_eq!(effect_status, "saved");
        drop(conn);
        drop(webview);
        drop(app);
        let _ = std::fs::remove_dir_all(root);
    }

    // covers: BR-WS-PERSIST-001
    // covers: BR-WS-DATA-001
    // 代理新建保存＝创建（D-NEW / REQ-WS-006，A-WS-M-S-01 §2.1）：baseline=null 且目标（含父目录）不存在 →
    // 创建父目录 + 原子落盘成功，时间轴节点恰一（V-N2）。手动新建（ws.create_doc 立即建文件）不在此路径。
    #[test]
    fn agent_create_save_creates_file_and_parent_dir() {
        let (root, _) = temp_workspace("agent-create", "");
        let app = app_for(&root);
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();

        let result = invoke(
            &webview,
            "ws.save_doc",
            serde_json::json!({
                "relPath": "agent/new/a.md", // 父目录 agent/new 不存在
                "content": "# 代理新建\n",
                "baseline": serde_json::Value::Null,
                "override": false
            }),
        );
        assert!(
            result.get("baseline").is_some(),
            "保存＝创建应返回新 baseline"
        );

        let created = root.join("agent/new/a.md");
        assert_eq!(
            std::fs::read_to_string(&created).unwrap(),
            "# 代理新建\n",
            "代理新建保存后才创建文件（含父目录）"
        );
        // V-N2：时间轴节点恰一个。
        let state = app.state::<AppState>();
        let conn = state.db.lock().unwrap();
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM timeline WHERE rel_path = 'agent/new/a.md'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);
        drop(conn);
        drop(webview);
        drop(app);
        let _ = std::fs::remove_dir_all(root);
    }

    // covers: BR-WS-OBS-001
    // V-N4：代理新建保存时目标已被外部创建 → WS_EXISTS 冲突，不覆盖、保留外部内容。
    #[test]
    fn agent_create_save_refuses_externally_created_target() {
        let (root, document) = temp_workspace("agent-conflict", "外部创建的内容\n");
        let app = app_for(&root);
        let app_handle = app.handle();
        let state = app.state::<AppState>();

        let routed = route_edit_persistence_command(
            &CommandRequest {
                command: "ws.save_doc".into(),
                correlation_id: "test-agent-conflict".into(),
                params: serde_json::json!({
                    "relPath": "notes/a.md",
                    "content": "代理内容\n",
                    "baseline": serde_json::Value::Null,
                    "override": false
                }),
            },
            &state,
            &app_handle,
        )
        .expect("ws.save_doc 已注册");
        let err = routed.expect_err("baseline=null 且目标已存在应拒绝覆盖");
        assert_eq!(err.code, "WS_EXISTS");
        assert_eq!(
            std::fs::read_to_string(&document).unwrap(),
            "外部创建的内容\n",
            "不得覆盖外部创建文件"
        );
        drop(app);
        let _ = std::fs::remove_dir_all(root);
    }

    // covers: BR-CORE-STATE-003
    // 后台卡待裁决期间打开目标文档后，迁移确认只保留 Pending；不接受、不写 DiskState。
    #[test]
    fn opened_background_diff_migrates_to_logical_review_without_disk_write() {
        let initial = "# 初始\n\n正文\n";
        let proposed = "# 初始\n\n迁移后的绿色预览\n";
        let (root, document) = temp_workspace("promote-open", initial);
        let app = app_for(&root);
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();

        let opened = invoke(
            &webview,
            "ws.read_doc",
            serde_json::json!({ "relPath": "notes/a.md" }),
        );
        let diff_id = generate_diff(&webview, opened["baseline"].as_str().unwrap(), proposed);

        let promoted = invoke(
            &webview,
            "de.promote_pending_diff_on_open",
            serde_json::json!({ "diffId": diff_id }),
        );

        assert_eq!(promoted["pending"], true);
        assert_eq!(std::fs::read_to_string(&document).unwrap(), initial);
        let state = app.state::<AppState>();
        let conn = state.db.lock().unwrap();
        let status: String = conn
            .query_row(
                "SELECT status FROM pending_diffs WHERE diff_id = ?1",
                [&diff_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, "pending");
        drop(conn);
        drop(webview);
        drop(app);
        let _ = std::fs::remove_dir_all(root);
    }

    // covers: BR-SYS-DATA-003
    // covers: BR-DE-STATE-001
    // covers: BR-WS-PERSIST-001
    // 同一 Webview IPC harness 验证拒绝不写 DiskState，第二次读取仍观察原文且无时间轴节点。
    #[test]
    fn rejected_diff_keeps_disk_and_reopen_unchanged_through_tauri_ipc() {
        let initial = "# 原文\n\n保持不变\n";
        let (root, document) = temp_workspace("rejected", initial);
        let app = app_for(&root);
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();

        let opened = invoke(
            &webview,
            "ws.read_doc",
            serde_json::json!({ "relPath": "notes/a.md" }),
        );
        let diff_id = generate_diff(
            &webview,
            opened["baseline"].as_str().unwrap(),
            "# 不应落盘\n",
        );
        invoke(
            &webview,
            "de.reject_diff",
            serde_json::json!({ "diffId": diff_id }),
        );

        assert_eq!(std::fs::read_to_string(&document).unwrap(), initial);
        let reopened = invoke(
            &webview,
            "ws.read_doc",
            serde_json::json!({ "relPath": "notes/a.md" }),
        );
        assert_eq!(reopened["content"], initial);
        assert_eq!(reopened["baseline"], opened["baseline"]);

        let state = app.state::<AppState>();
        let conn = state.db.lock().unwrap();
        let status: String = conn
            .query_row(
                "SELECT status FROM pending_diffs WHERE diff_id = ?1",
                [&diff_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, "rejected");
        let workspace = workspace::lifecycle::active_workspace(&conn)
            .unwrap()
            .unwrap();
        let target = crate::types::DocRef {
            workspace_id: workspace.workspace_id,
            rel_path: "notes/a.md".into(),
            doc_type: "md".into(),
        };
        assert!(workspace::timeline::list_timeline(&conn, &target)
            .unwrap()
            .is_empty());
        drop(conn);
        drop(webview);
        drop(app);
        let _ = std::fs::remove_dir_all(root);
    }
}
