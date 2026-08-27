//! 从应用组合根迁入的模块运行时/路由；保持原 command 与状态机行为等价。

use crate::*;

/**
 * @GOV
 * codes:    BR-SYS-DATA-007-API-AST-AST-MEMORY-001, BR-AST-DATA-001-API-AST-AST-MEMORY-001
 * type:     API
 * chain:    AST-MEMORY, REF-INJECT, IPC-CALL
 * rules:    BR-SYS-DATA-007, BR-AST-DATA-001
 * term_ref: TERM-AST-002
 * boundary: in=workspaceId | out=activeManageableMemoryEntries(onlyExplicitSelectionReachesTurnNeverAutoInjectNeverCloud)
 */
/// 路由 `ast.list_entries`（显式记忆选择供给，BR-SYS-DATA-007 / TERM-AST-002）：返回当前 workspace 的
/// active 记忆条目含 content，供记忆库面板浏览/用户显式选择（构造 MemorySelectionRef 快照进当前回合）。
/// workspace 作用域隔离、纯本地不上云（BR-AST-DATA-001）；仅显式选择进入回合，不自动供给/注入。
pub(crate) fn route_ast_list_entries(state: &AppState) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let conn = lock_db(state)?;
    let list = ast::store::list_manageable(&conn, &ws.workspace_id)
        .map_err(|e| IpcError::new("AST_DB_ERROR", &e.to_string(), true, false))?;
    let entries: Vec<serde_json::Value> = list
        .iter()
        .map(|e| {
            serde_json::json!({
                "entryId": e.entry_id,
                "memoryKind": e.memory_kind,
                "content": e.content,
                "status": e.status,
                "originTurn": e.origin_turn,
            })
        })
        .collect();
    Ok(serde_json::Value::Array(entries))
}
