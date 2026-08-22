//! PI-WS-06 · WorkspaceOperation 持久化与状态机内核 · WS-OPERATION / SM-WS-04
//!
//! 本模块持有不可变计划、生命周期转移、逐项回执、恢复载荷与 CreateDocumentBundle 绑定。
//! 文件系统执行必须经后续 execute_operation 受控入口；低层 fileops 不构成 Agent 授权。

use crate::ipc::IpcError;
use crate::types::{
    CreateDocumentBundle, CreateDocumentRejectionGuard, DocRef, WorkspaceOperationConsequences,
    WorkspaceOperationEntry, WorkspaceOperationItem, WorkspaceOperationItemReceipt,
    WorkspaceOperationItemStatus, WorkspaceOperationKind, WorkspaceOperationPlan,
    WorkspaceOperationReceipt, WorkspaceOperationStatus, WorkspaceRef, WorkspaceResourceBaseline,
    WorkspaceResourceKind,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

static OPERATION_SEQ: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct WorkspaceOperationProgress {
    item_receipts: Vec<WorkspaceOperationItemReceipt>,
    compensation_receipts: Vec<WorkspaceOperationItemReceipt>,
}

fn upsert_progress_receipt(
    receipts: &mut Vec<WorkspaceOperationItemReceipt>,
    receipt: WorkspaceOperationItemReceipt,
) {
    if let Some(existing) = receipts
        .iter_mut()
        .find(|existing| existing.item_id == receipt.item_id)
    {
        *existing = receipt;
    } else {
        receipts.push(receipt);
        receipts.sort_by_key(|value| value.seq);
    }
}

fn trace_operation(
    event: &str,
    plan_id: &str,
    operation_id: Option<&str>,
    item_seq: Option<u32>,
    status_or_code: Option<&str>,
) {
    eprintln!(
        "[workspace-operation] {}",
        serde_json::json!({
            "event": event,
            "plan_id": plan_id,
            "operation_id": operation_id,
            "item_seq": item_seq,
            "status_or_code": status_or_code,
        })
    );
}

fn generated_id(prefix: &str) -> String {
    format!(
        "{prefix}-{}-{}",
        now_millis(),
        OPERATION_SEQ.fetch_add(1, Ordering::Relaxed)
    )
}

fn encode<T: serde::Serialize>(value: &T) -> rusqlite::Result<String> {
    serde_json::to_string(value).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
}

fn decode<T: serde::de::DeserializeOwned>(value: &str) -> rusqlite::Result<T> {
    serde_json::from_str(value).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

fn status_text(status: WorkspaceOperationStatus) -> &'static str {
    match status {
        WorkspaceOperationStatus::Planning => "planning",
        WorkspaceOperationStatus::WaitingConfirm => "waiting_confirm",
        WorkspaceOperationStatus::Revalidating => "revalidating",
        WorkspaceOperationStatus::Executing => "executing",
        WorkspaceOperationStatus::Compensating => "compensating",
        WorkspaceOperationStatus::Completed => "completed",
        WorkspaceOperationStatus::Rejected => "rejected",
        WorkspaceOperationStatus::Expired => "expired",
        WorkspaceOperationStatus::FailedCompensated => "failed_compensated",
        WorkspaceOperationStatus::PartialFailed => "partial_failed",
        WorkspaceOperationStatus::Interrupted => "interrupted",
    }
}

fn parse_status(status: &str) -> rusqlite::Result<WorkspaceOperationStatus> {
    match status {
        "planning" => Ok(WorkspaceOperationStatus::Planning),
        "waiting_confirm" => Ok(WorkspaceOperationStatus::WaitingConfirm),
        "revalidating" => Ok(WorkspaceOperationStatus::Revalidating),
        "executing" => Ok(WorkspaceOperationStatus::Executing),
        "compensating" => Ok(WorkspaceOperationStatus::Compensating),
        "completed" => Ok(WorkspaceOperationStatus::Completed),
        "rejected" => Ok(WorkspaceOperationStatus::Rejected),
        "expired" => Ok(WorkspaceOperationStatus::Expired),
        "failed_compensated" => Ok(WorkspaceOperationStatus::FailedCompensated),
        "partial_failed" => Ok(WorkspaceOperationStatus::PartialFailed),
        "interrupted" => Ok(WorkspaceOperationStatus::Interrupted),
        other => Err(rusqlite::Error::InvalidParameterName(format!(
            "unknown WorkspaceOperation status: {other}"
        ))),
    }
}

/// SM-WS-04 转移事件。Interrupted 的显式恢复不会复用旧计划，因此没有指向 Planning 的原地转移。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceOperationEvent {
    PlanReady,
    UserConfirm,
    UserReject,
    BaselineValid,
    BaselineDrift,
    ItemFail,
    AllItemsOk,
    CompensationOk,
    CompensationFail,
    Interrupt,
}

/**
 * @GOV
 * codes:    BR-WS-STATE-003-RB-WS-WS-OPERATION-001
 * type:     RB
 * chain:    WS-OPERATION
 * rules:    BR-WS-STATE-003, BR-WS-PERSIST-003
 * term_ref: TERM-WS-003, TERM-WS-004
 * boundary: in=WorkspaceOperationStatusAndEvent | out=WorkspaceOperationStatus(rejectedWhenTransitionForbidden)
 */
pub fn transition(
    state: WorkspaceOperationStatus,
    event: WorkspaceOperationEvent,
) -> Option<WorkspaceOperationStatus> {
    use WorkspaceOperationEvent::*;
    use WorkspaceOperationStatus::*;
    match (state, event) {
        (Planning, PlanReady) => Some(WaitingConfirm),
        (WaitingConfirm, UserConfirm) => Some(Revalidating),
        (WaitingConfirm, UserReject) => Some(Rejected),
        (Revalidating, BaselineValid) => Some(Executing),
        (Revalidating, BaselineDrift) => Some(Expired),
        (Executing, ItemFail) => Some(Compensating),
        (Executing, AllItemsOk) => Some(Completed),
        (Compensating, CompensationOk) => Some(FailedCompensated),
        (Compensating, CompensationFail) => Some(PartialFailed),
        (Revalidating | Executing | Compensating, Interrupt) => Some(Interrupted),
        _ => None,
    }
}

/**
 * @GOV
 * codes:    BR-WS-DATA-003-EFFECT-WS-WS-OPERATION-001
 * type:     EFFECT
 * chain:    WS-OPERATION
 * rules:    BR-WS-DATA-003
 * term_ref: TERM-WS-004
 * boundary: in=ImmutableWorkspaceOperationPlan | out=persistedWorkspaceOperationPlan(rejectedWhenIdentityReused)
 */
pub fn insert_plan(conn: &Connection, plan: &WorkspaceOperationPlan) -> rusqlite::Result<()> {
    let plan_json = encode(plan)?;
    conn.execute(
        "INSERT INTO workspace_operation_plans \
         (plan_id, workspace_id, task_run_id, source_turn_id, plan_json, decision_request_id, \
          idempotency_key, status, operation_id, created_at, updated_at) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        params![
            plan.plan_id,
            plan.workspace_id,
            plan.task_run_id,
            plan.source_turn_id,
            plan_json,
            plan.decision_request_id,
            plan.idempotency_key,
            status_text(plan.status),
            plan.operation_id,
            plan.created_at,
            plan.updated_at,
        ],
    )?;
    Ok(())
}

pub fn load_plan(
    conn: &Connection,
    plan_id: &str,
) -> rusqlite::Result<Option<WorkspaceOperationPlan>> {
    conn.query_row(
        "SELECT plan_json, status, decision_request_id, operation_id, updated_at \
         FROM workspace_operation_plans WHERE plan_id=?1",
        [plan_id],
        |row| {
            let mut plan: WorkspaceOperationPlan = decode(&row.get::<_, String>(0)?)?;
            plan.status = parse_status(&row.get::<_, String>(1)?)?;
            plan.decision_request_id = row.get(2)?;
            plan.operation_id = row.get(3)?;
            plan.updated_at = row.get(4)?;
            Ok(plan)
        },
    )
    .optional()
}

pub fn list_plans(
    conn: &Connection,
    workspace_id: &str,
    status: Option<WorkspaceOperationStatus>,
) -> rusqlite::Result<Vec<WorkspaceOperationPlan>> {
    let mut stmt = conn.prepare(
        "SELECT plan_id FROM workspace_operation_plans \
         WHERE workspace_id=?1 AND (?2 IS NULL OR status=?2) ORDER BY updated_at DESC",
    )?;
    let status = status.map(status_text);
    let ids = stmt
        .query_map(params![workspace_id, status], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    ids.into_iter()
        .filter_map(|plan_id| load_plan(conn, &plan_id).transpose())
        .collect()
}

/// 只推进生命周期与执行身份；冻结的 operation_items/baseline/manifest 永不重写。
pub fn advance_plan(
    conn: &Connection,
    plan_id: &str,
    event: WorkspaceOperationEvent,
    operation_id: Option<&str>,
) -> rusqlite::Result<WorkspaceOperationPlan> {
    let mut plan = load_plan(conn, plan_id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    let next = transition(plan.status, event).ok_or_else(|| {
        rusqlite::Error::InvalidParameterName(format!(
            "forbidden WorkspaceOperation transition: {:?} + {:?}",
            plan.status, event
        ))
    })?;
    if plan.operation_id.is_none() {
        plan.operation_id = operation_id.map(str::to_owned);
    } else if operation_id.is_some() && plan.operation_id.as_deref() != operation_id {
        return Err(rusqlite::Error::InvalidParameterName(
            "operation identity mismatch".into(),
        ));
    }
    plan.status = next;
    plan.updated_at = now_millis();
    conn.execute(
        "UPDATE workspace_operation_plans SET status=?2, operation_id=?3, updated_at=?4 \
         WHERE plan_id=?1",
        params![
            plan_id,
            status_text(next),
            plan.operation_id,
            plan.updated_at
        ],
    )?;
    Ok(plan)
}

pub fn bind_decision_request(
    conn: &Connection,
    plan_id: &str,
    request_id: &str,
) -> rusqlite::Result<WorkspaceOperationPlan> {
    let mut plan = load_plan(conn, plan_id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    if plan.status != WorkspaceOperationStatus::WaitingConfirm {
        return Err(rusqlite::Error::InvalidParameterName(
            "decision request requires waiting_confirm plan".into(),
        ));
    }
    if let Some(existing) = &plan.decision_request_id {
        if existing != request_id {
            return Err(rusqlite::Error::InvalidParameterName(
                "plan already bound to another decision request".into(),
            ));
        }
        return Ok(plan);
    }
    plan.decision_request_id = Some(request_id.to_string());
    plan.updated_at = now_millis();
    conn.execute(
        "UPDATE workspace_operation_plans SET decision_request_id=?2, updated_at=?3 WHERE plan_id=?1",
        params![plan_id, request_id, plan.updated_at],
    )?;
    Ok(plan)
}

pub fn store_payload<T: serde::Serialize>(
    conn: &Connection,
    payload_ref: &str,
    workspace_id: &str,
    payload_kind: &str,
    payload: &T,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO workspace_operation_payloads \
         (payload_ref, workspace_id, payload_kind, payload_json, created_at) VALUES (?1,?2,?3,?4,?5)",
        params![payload_ref, workspace_id, payload_kind, encode(payload)?, now_millis()],
    )?;
    Ok(())
}

pub fn load_payload<T: serde::de::DeserializeOwned>(
    conn: &Connection,
    payload_ref: &str,
) -> rusqlite::Result<Option<T>> {
    conn.query_row(
        "SELECT payload_json FROM workspace_operation_payloads WHERE payload_ref=?1",
        [payload_ref],
        |row| decode(&row.get::<_, String>(0)?),
    )
    .optional()
}

pub fn replace_payload<T: serde::Serialize>(
    conn: &Connection,
    payload_ref: &str,
    payload: &T,
) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE workspace_operation_payloads SET payload_json=?2 WHERE payload_ref=?1",
        params![payload_ref, encode(payload)?],
    )
}

/**
 * @GOV
 * codes:    BR-WS-DATA-003-EFFECT-WS-WS-TIMELINE-003
 * type:     EFFECT
 * chain:    WS-TIMELINE
 * rules:    BR-WS-DATA-003, BR-WS-PERSIST-003
 * term_ref: TERM-WS-005, TERM-WS-006
 * boundary: in=WorkspaceOperationEntryAndTypedRecoveryPayloads | out=appendOnlyWorkspaceTimelineResourceEntry
 */
pub fn append_operation_entry(
    conn: &Connection,
    entry: &WorkspaceOperationEntry,
) -> rusqlite::Result<()> {
    let before_ref = format!("wop_before_{}", entry.operation_id);
    let after_ref = format!("wop_after_{}", entry.operation_id);
    let receipts_ref = format!("wop_receipts_{}", entry.operation_id);
    store_payload(
        conn,
        &before_ref,
        &entry.workspace_id,
        "before_manifest",
        &entry.before_manifest,
    )?;
    store_payload(
        conn,
        &after_ref,
        &entry.workspace_id,
        "after_manifest",
        &entry.after_manifest,
    )?;
    store_payload(
        conn,
        &receipts_ref,
        &entry.workspace_id,
        "operation_receipts",
        &(
            entry.item_receipts.clone(),
            entry.compensation_receipts.clone(),
        ),
    )?;
    conn.execute(
        "INSERT INTO workspace_operation_entries \
         (operation_id,entry_id,plan_id,workspace_id,before_ref,after_ref,receipts_ref,restore_kind,restore_payload_ref,final_status,created_at) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        params![
            entry.operation_id,
            entry.entry_id,
            entry.plan_id,
            entry.workspace_id,
            before_ref,
            after_ref,
            receipts_ref,
            entry.restore_kind,
            entry.restore_payload_ref,
            status_text(entry.final_status),
            entry.created_at,
        ],
    )?;
    Ok(())
}

pub fn load_operation_entry(
    conn: &Connection,
    operation_id: &str,
) -> rusqlite::Result<Option<WorkspaceOperationEntry>> {
    conn.query_row(
        "SELECT entry_id,plan_id,workspace_id,before_ref,after_ref,receipts_ref,restore_kind,restore_payload_ref,final_status,created_at \
         FROM workspace_operation_entries WHERE operation_id=?1",
        [operation_id],
        |row| {
            let before_ref: String = row.get(3)?;
            let after_ref: String = row.get(4)?;
            let receipts_ref: String = row.get(5)?;
            let (item_receipts, compensation_receipts) = load_payload(conn, &receipts_ref)?
                .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
            Ok(WorkspaceOperationEntry {
                entry_id: row.get(0)?,
                operation_id: operation_id.to_string(),
                plan_id: row.get(1)?,
                workspace_id: row.get(2)?,
                before_manifest: load_payload(conn, &before_ref)?
                    .ok_or(rusqlite::Error::QueryReturnedNoRows)?,
                after_manifest: load_payload(conn, &after_ref)?
                    .ok_or(rusqlite::Error::QueryReturnedNoRows)?,
                item_receipts,
                compensation_receipts,
                restore_kind: row.get(6)?,
                restore_payload_ref: row.get(7)?,
                final_status: parse_status(&row.get::<_, String>(8)?)?,
                created_at: row.get(9)?,
            })
        },
    )
    .optional()
}

pub fn load_operation_entry_by_entry_id(
    conn: &Connection,
    entry_id: &str,
) -> rusqlite::Result<Option<WorkspaceOperationEntry>> {
    let operation_id: Option<String> = conn
        .query_row(
            "SELECT operation_id FROM workspace_operation_entries WHERE entry_id=?1",
            [entry_id],
            |row| row.get(0),
        )
        .optional()?;
    operation_id
        .map(|value| load_operation_entry(conn, &value))
        .transpose()
        .map(Option::flatten)
}

pub fn list_operation_entries(
    conn: &Connection,
    workspace_id: &str,
) -> rusqlite::Result<Vec<WorkspaceOperationEntry>> {
    let mut stmt = conn.prepare(
        "SELECT operation_id FROM workspace_operation_entries \
         WHERE workspace_id=?1 ORDER BY created_at ASC",
    )?;
    let ids = stmt
        .query_map([workspace_id], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    ids.into_iter()
        .filter_map(
            |operation_id| match load_operation_entry(conn, &operation_id) {
                Ok(Some(entry)) => Some(Ok(entry)),
                Ok(None) => None,
                Err(error) => Some(Err(error)),
            },
        )
        .collect()
}

pub fn entry_affects_path(entry: &WorkspaceOperationEntry, rel_path: &str) -> bool {
    entry
        .before_manifest
        .iter()
        .chain(entry.after_manifest.iter())
        .any(|baseline| {
            baseline.rel_path == rel_path
                || baseline.rel_path.starts_with(&format!("{rel_path}/"))
                || rel_path.starts_with(&format!("{}/", baseline.rel_path))
        })
}

pub fn restore_intents_from_entry(
    conn: &Connection,
    entry: &WorkspaceOperationEntry,
) -> Result<Vec<WorkspaceOperationIntent>, IpcError> {
    if entry.restore_kind == "discard_created_resource" {
        return Err(operation_error(
            "WS_OPERATION_NOT_RECOVERABLE",
            "被拒绝且从未保存的新建内容不提供资源恢复",
            false,
        ));
    }
    let plan = load_plan(conn, &entry.plan_id)
        .map_err(|error| db_error("读取历史资源操作计划失败", error))?
        .ok_or_else(|| {
            operation_error("WS_OPERATION_NOT_FOUND", "历史资源操作计划不存在", false)
        })?;
    let compensated = entry
        .compensation_receipts
        .iter()
        .filter(|receipt| receipt.status == WorkspaceOperationItemStatus::Compensated)
        .map(|receipt| receipt.item_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut completed = entry
        .item_receipts
        .iter()
        .filter(|receipt| {
            receipt.status == WorkspaceOperationItemStatus::Completed
                && !compensated.contains(receipt.item_id.as_str())
        })
        .map(|receipt| receipt.item_id.as_str())
        .collect::<BTreeSet<_>>();
    if entry.final_status == WorkspaceOperationStatus::Interrupted {
        let root_path: String = conn
            .query_row(
                "SELECT root_path FROM workspaces WHERE workspace_id=?1",
                [&entry.workspace_id],
                |row| row.get(0),
            )
            .map_err(|error| db_error("读取中断操作 workspace 失败", error))?;
        let root = Path::new(&root_path);
        for receipt in entry
            .item_receipts
            .iter()
            .filter(|receipt| receipt.status == WorkspaceOperationItemStatus::Attempted)
        {
            if let Some(item) = plan
                .operation_items
                .iter()
                .find(|item| item.item_id == receipt.item_id)
            {
                let effect_applied = match item.kind {
                    WorkspaceOperationKind::CreateDocument
                    | WorkspaceOperationKind::CreateFolder
                    | WorkspaceOperationKind::RestoreResource => item
                        .target_rel_path
                        .as_deref()
                        .map(|path| root.join(path).exists())
                        .unwrap_or(false),
                    WorkspaceOperationKind::RenameResource
                    | WorkspaceOperationKind::MoveResource => {
                        let source_exists = item
                            .source_rel_path
                            .as_deref()
                            .map(|path| root.join(path).exists())
                            .unwrap_or(false);
                        let target_exists = item
                            .target_rel_path
                            .as_deref()
                            .map(|path| root.join(path).exists())
                            .unwrap_or(false);
                        !source_exists && target_exists
                    }
                    WorkspaceOperationKind::DeleteResource
                    | WorkspaceOperationKind::DiscardCreatedResource => item
                        .source_rel_path
                        .as_deref()
                        .map(|path| !root.join(path).exists())
                        .unwrap_or(false),
                };
                if effect_applied && !compensated.contains(receipt.item_id.as_str()) {
                    completed.insert(receipt.item_id.as_str());
                }
            }
        }
    }
    let mut snapshot_by_item = entry
        .item_receipts
        .iter()
        .filter_map(|receipt| {
            receipt
                .snapshot_ref
                .as_ref()
                .map(|snapshot| (receipt.item_id.as_str(), snapshot.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    if entry.final_status == WorkspaceOperationStatus::Interrupted {
        for item in &plan.operation_items {
            if !matches!(
                item.kind,
                WorkspaceOperationKind::DeleteResource
                    | WorkspaceOperationKind::DiscardCreatedResource
            ) || snapshot_by_item.contains_key(item.item_id.as_str())
            {
                continue;
            }
            let inferred = format!("wop-snapshot-{}-{}", entry.operation_id, item.seq);
            let exists = conn
                .query_row(
                    "SELECT COUNT(*) FROM workspace_operation_snapshots WHERE snapshot_ref=?1",
                    [&inferred],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|error| db_error("核验中断删除快照失败", error))?
                > 0;
            if exists {
                snapshot_by_item.insert(item.item_id.as_str(), inferred);
            }
        }
    }
    let mut intents = Vec::new();
    for item in plan.operation_items.iter().rev() {
        if !completed.contains(item.item_id.as_str()) {
            continue;
        }
        let intent = match item.kind {
            WorkspaceOperationKind::CreateDocument | WorkspaceOperationKind::CreateFolder => {
                WorkspaceOperationIntent {
                    kind: WorkspaceOperationKind::DeleteResource,
                    source_path: item.target_rel_path.clone(),
                    target_path: None,
                    resource_kind: Some(item.resource_kind),
                    recursive: item.resource_kind == WorkspaceResourceKind::Folder,
                    restore_payload_ref: None,
                }
            }
            WorkspaceOperationKind::RenameResource | WorkspaceOperationKind::MoveResource => {
                WorkspaceOperationIntent {
                    kind: WorkspaceOperationKind::RenameResource,
                    source_path: item.target_rel_path.clone(),
                    target_path: item.source_rel_path.clone(),
                    resource_kind: Some(item.resource_kind),
                    recursive: false,
                    restore_payload_ref: None,
                }
            }
            WorkspaceOperationKind::DeleteResource => WorkspaceOperationIntent {
                kind: WorkspaceOperationKind::RestoreResource,
                source_path: None,
                target_path: item.source_rel_path.clone(),
                resource_kind: Some(item.resource_kind),
                recursive: item.recursive,
                restore_payload_ref: snapshot_by_item.get(item.item_id.as_str()).cloned(),
            },
            WorkspaceOperationKind::RestoreResource => WorkspaceOperationIntent {
                kind: WorkspaceOperationKind::DeleteResource,
                source_path: item.target_rel_path.clone(),
                target_path: None,
                resource_kind: Some(item.resource_kind),
                recursive: item.resource_kind == WorkspaceResourceKind::Folder,
                restore_payload_ref: None,
            },
            WorkspaceOperationKind::DiscardCreatedResource => continue,
        };
        intents.push(intent);
    }
    if intents.is_empty() {
        return Err(operation_error(
            "WS_OPERATION_NOT_RECOVERABLE",
            "该资源操作没有可生成的新恢复计划",
            false,
        ));
    }
    Ok(intents)
}

pub fn store_snapshot_member(
    conn: &Connection,
    snapshot_ref: &str,
    rel_path: &str,
    resource_kind: &str,
    content: Option<&[u8]>,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO workspace_operation_snapshots \
         (snapshot_ref,rel_path,resource_kind,content,created_at) VALUES (?1,?2,?3,?4,?5)",
        params![snapshot_ref, rel_path, resource_kind, content, now_millis()],
    )?;
    Ok(())
}

pub fn load_snapshot_members(
    conn: &Connection,
    snapshot_ref: &str,
) -> rusqlite::Result<Vec<(String, String, Option<Vec<u8>>)>> {
    let mut stmt = conn.prepare(
        "SELECT rel_path,resource_kind,content FROM workspace_operation_snapshots \
         WHERE snapshot_ref=?1 ORDER BY LENGTH(rel_path), rel_path",
    )?;
    let rows = stmt.query_map([snapshot_ref], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })?;
    rows.collect()
}

pub fn upsert_create_document_bundle(
    conn: &Connection,
    bundle: &CreateDocumentBundle,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO create_document_bundles \
         (plan_id,workspace_id,target_rel_path,initial_content_ref,operation_id,initial_diff_id,guard_ref,status,created_at,updated_at) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?9) \
         ON CONFLICT(plan_id) DO UPDATE SET operation_id=excluded.operation_id, initial_diff_id=excluded.initial_diff_id, \
         guard_ref=excluded.guard_ref, status=excluded.status, updated_at=excluded.updated_at",
        params![
            bundle.plan_id,
            bundle.workspace_id,
            bundle.target.rel_path,
            bundle.initial_content_ref,
            bundle.operation_id,
            bundle.initial_diff_id,
            bundle.rejection_guard_ref,
            bundle.status,
            now_millis(),
        ],
    )?;
    Ok(())
}

pub fn load_create_document_bundle(
    conn: &Connection,
    plan_id: &str,
) -> rusqlite::Result<Option<CreateDocumentBundle>> {
    conn.query_row(
        "SELECT workspace_id,target_rel_path,initial_content_ref,operation_id,initial_diff_id,guard_ref,status \
         FROM create_document_bundles WHERE plan_id=?1",
        [plan_id],
        |row| {
            let rel_path: String = row.get(1)?;
            let doc_type = rel_path.rsplit_once('.').map(|(_, ext)| ext).unwrap_or("").to_string();
            Ok(CreateDocumentBundle {
                plan_id: plan_id.to_string(),
                workspace_id: row.get(0)?,
                target: DocRef {
                    workspace_id: row.get(0)?,
                    rel_path,
                    doc_type,
                },
                initial_content_ref: row.get(2)?,
                operation_id: row.get(3)?,
                initial_diff_id: row.get(4)?,
                rejection_guard_ref: row.get(5)?,
                status: row.get(6)?,
            })
        },
    )
    .optional()
}

pub fn load_create_document_bundle_by_diff(
    conn: &Connection,
    diff_id: &str,
) -> rusqlite::Result<Option<CreateDocumentBundle>> {
    let plan_id: Option<String> = conn
        .query_row(
            "SELECT plan_id FROM create_document_bundles WHERE initial_diff_id=?1",
            [diff_id],
            |row| row.get(0),
        )
        .optional()?;
    plan_id
        .map(|value| load_create_document_bundle(conn, &value))
        .transpose()
        .map(Option::flatten)
}

pub fn set_create_document_bundle_status_by_diff(
    conn: &Connection,
    diff_id: &str,
    status: &str,
) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE create_document_bundles SET status=?2, updated_at=?3 WHERE initial_diff_id=?1",
        params![diff_id, status, now_millis()],
    )
}

pub fn mark_create_document_bundles_saved_for_path(
    conn: &Connection,
    workspace_id: &str,
    rel_path: &str,
) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE create_document_bundles SET status='saved', updated_at=?3 \
         WHERE workspace_id=?1 AND target_rel_path=?2 AND status='initial_diff_accepted'",
        params![workspace_id, rel_path, now_millis()],
    )
}

pub fn acknowledge_create_document_preview(
    conn: &Connection,
    diff_id: &str,
    session_version: u64,
    content_hash: &str,
) -> Result<bool, IpcError> {
    let Some(mut bundle) = load_create_document_bundle_by_diff(conn, diff_id)
        .map_err(|error| db_error("读取新建文档组合失败", error))?
    else {
        return Ok(false);
    };
    let guard_ref = bundle.rejection_guard_ref.as_deref().ok_or_else(|| {
        operation_error(
            "WS_CREATE_BUNDLE_GUARD_MISSING",
            "新建文档拒绝保护基线缺失",
            false,
        )
    })?;
    let mut guard: CreateDocumentRejectionGuard = load_payload(conn, guard_ref)
        .map_err(|error| db_error("读取新建文档拒绝保护基线失败", error))?
        .ok_or_else(|| {
            operation_error(
                "WS_CREATE_BUNDLE_GUARD_MISSING",
                "新建文档拒绝保护基线不存在",
                false,
            )
        })?;
    guard.logical_session_version = session_version;
    guard.logical_content_hash = content_hash.to_string();
    if replace_payload(conn, guard_ref, &guard)
        .map_err(|error| db_error("更新新建文档拒绝保护基线失败", error))?
        != 1
    {
        return Err(operation_error(
            "WS_CREATE_BUNDLE_GUARD_MISSING",
            "新建文档拒绝保护基线更新失败",
            false,
        ));
    }
    bundle.status = "initial_diff_applied".into();
    upsert_create_document_bundle(conn, &bundle)
        .map_err(|error| db_error("更新新建文档组合状态失败", error))?;
    Ok(true)
}

pub fn validate_create_document_diff_guard(
    conn: &Connection,
    workspace: &WorkspaceRef,
    diff_id: &str,
    logical_session_version: u64,
    logical_content_hash: &str,
) -> Result<bool, IpcError> {
    let Some(bundle) = load_create_document_bundle_by_diff(conn, diff_id)
        .map_err(|error| db_error("读取新建文档组合失败", error))?
    else {
        return Ok(false);
    };
    if bundle.workspace_id != workspace.workspace_id {
        return Err(operation_error(
            "WS_CREATE_BUNDLE_WORKSPACE_MISMATCH",
            "新建文档组合不属于当前 workspace",
            false,
        ));
    }
    let guard_ref = bundle.rejection_guard_ref.as_deref().ok_or_else(|| {
        operation_error(
            "WS_CREATE_BUNDLE_GUARD_MISSING",
            "新建文档拒绝保护基线缺失",
            false,
        )
    })?;
    let guard: CreateDocumentRejectionGuard = load_payload(conn, guard_ref)
        .map_err(|error| db_error("读取新建文档拒绝保护基线失败", error))?
        .ok_or_else(|| {
            operation_error(
                "WS_CREATE_BUNDLE_GUARD_MISSING",
                "新建文档拒绝保护基线不存在",
                false,
            )
        })?;
    let current_disk = capture_baseline(Path::new(&workspace.root_path), &bundle.target.rel_path)?;
    if guard.plan_id != bundle.plan_id
        || bundle.operation_id.as_deref() != Some(guard.operation_id.as_str())
        || current_disk != guard.disk_baseline
        || logical_session_version != guard.logical_session_version
        || logical_content_hash != guard.logical_content_hash
    {
        return Err(operation_error(
            "WS_CREATE_BUNDLE_BASELINE_ADVANCED",
            "新建文档已被继续编辑或外部修改；旧初始建议已失效，文件会保留",
            false,
        ));
    }
    Ok(true)
}

pub fn compensate_create_document_open_failure(
    conn: &Connection,
    workspace: &WorkspaceRef,
    plan_id: &str,
) -> Result<WorkspaceOperationEntry, IpcError> {
    let mut bundle = load_create_document_bundle(conn, plan_id)
        .map_err(|error| db_error("读取新建文档组合失败", error))?
        .ok_or_else(|| {
            operation_error("WS_CREATE_BUNDLE_NOT_FOUND", "新建文档组合不存在", false)
        })?;
    if bundle.workspace_id != workspace.workspace_id
        || !matches!(
            bundle.status.as_str(),
            "resource_created" | "projecting_initial_diff"
        )
    {
        return Err(operation_error(
            "WS_CREATE_BUNDLE_IDENTITY_MISMATCH",
            "新建文档打开失败补偿只允许处理本 workspace 刚创建的资源",
            false,
        ));
    }
    let operation_id = bundle.operation_id.clone().ok_or_else(|| {
        operation_error(
            "WS_CREATE_BUNDLE_OPERATION_MISSING",
            "新建文档组合缺少资源操作身份",
            false,
        )
    })?;
    let create_entry = load_operation_entry(conn, &operation_id)
        .map_err(|error| db_error("读取新建文档资源历史失败", error))?
        .ok_or_else(|| {
            operation_error("WS_OPERATION_NOT_FOUND", "新建文档资源历史不存在", false)
        })?;
    let expected = create_entry
        .after_manifest
        .iter()
        .find(|baseline| baseline.rel_path == bundle.target.rel_path)
        .ok_or_else(|| {
            operation_error(
                "WS_CREATE_BUNDLE_BASELINE_MISSING",
                "新建文档资源历史缺少创建后基线",
                false,
            )
        })?;
    let current = capture_baseline(Path::new(&workspace.root_path), &bundle.target.rel_path)?;
    if &current != expected {
        return Err(operation_error(
            "WS_CREATE_BUNDLE_BASELINE_ADVANCED",
            "新建文档打开失败后磁盘基线已变化，不能自动删除",
            false,
        ));
    }
    let abs =
        super::resolve_in_workspace(Path::new(&workspace.root_path), &bundle.target.rel_path)?;
    let rollback_bytes = std::fs::read(&abs).map_err(|error| {
        operation_error(
            "WS_OPERATION_READ_FAILED",
            format!("读取新建文档补偿内容失败: {error}"),
            true,
        )
    })?;
    let was_bdocx =
        super::listing::is_bdocx(Path::new(&workspace.root_path), &bundle.target.rel_path);
    super::fileops::delete_doc(&abs)?;
    if let Err(error) = super::listing::cleanup_bdocx_under(
        Path::new(&workspace.root_path),
        &bundle.target.rel_path,
    ) {
        restore_compensated_document(
            &abs,
            &rollback_bytes,
            Path::new(&workspace.root_path),
            &bundle.target.rel_path,
            was_bdocx,
            &error,
        )?;
        return Err(error);
    }
    let compensation_operation_id = format!("{operation_id}-open-failed");
    let entry = WorkspaceOperationEntry {
        entry_id: generated_id("wop-entry"),
        operation_id: compensation_operation_id,
        plan_id: bundle.plan_id.clone(),
        workspace_id: bundle.workspace_id.clone(),
        before_manifest: vec![current],
        after_manifest: vec![capture_baseline(
            Path::new(&workspace.root_path),
            &bundle.target.rel_path,
        )?],
        item_receipts: vec![WorkspaceOperationItemReceipt {
            item_id: format!("{}-open-failed", bundle.plan_id),
            seq: 0,
            status: WorkspaceOperationItemStatus::Compensated,
            error_code: None,
            error_message: None,
            snapshot_ref: None,
            completed_at: now_millis(),
        }],
        compensation_receipts: Vec::new(),
        restore_kind: "discard_created_resource".into(),
        restore_payload_ref: None,
        final_status: WorkspaceOperationStatus::FailedCompensated,
        created_at: now_millis(),
    };
    bundle.status = "open_failed_compensated".into();
    if let Err(error) =
        persist_bundle_compensation(conn, &entry, &bundle, bundle.initial_diff_id.as_deref())
    {
        std::fs::write(&abs, rollback_bytes).map_err(|restore_error| {
            operation_error(
                "WS_CREATE_BUNDLE_PARTIAL_FAILED",
                format!("记录打开失败补偿历史失败且恢复文件失败: {error}; {restore_error}"),
                false,
            )
        })?;
        if was_bdocx {
            super::listing::mark_bdocx(Path::new(&workspace.root_path), &bundle.target.rel_path)?;
        }
        return Err(db_error("记录打开失败补偿历史失败，文件已恢复", error));
    }
    Ok(entry)
}

fn persist_bundle_compensation(
    conn: &Connection,
    entry: &WorkspaceOperationEntry,
    bundle: &CreateDocumentBundle,
    expired_diff_id: Option<&str>,
) -> rusqlite::Result<()> {
    let transaction = conn.unchecked_transaction()?;
    append_operation_entry(&transaction, entry)?;
    upsert_create_document_bundle(&transaction, bundle)?;
    if let Some(diff_id) = expired_diff_id {
        transaction.execute(
            "UPDATE pending_diffs SET status='expired', terminal_reason='create_resource_compensated' \
             WHERE diff_id=?1 AND status='pending'",
            [diff_id],
        )?;
        transaction.execute(
            "UPDATE task_preview_checkpoints SET status='cancelled', consumed_at=?2 \
             WHERE diff_id=?1 AND status IN ('awaiting_ack','acknowledged')",
            params![diff_id, now_millis()],
        )?;
    }
    transaction.commit()
}

fn restore_compensated_document(
    abs: &Path,
    rollback_bytes: &[u8],
    root: &Path,
    rel_path: &str,
    was_bdocx: bool,
    cause: &IpcError,
) -> Result<(), IpcError> {
    std::fs::write(abs, rollback_bytes).map_err(|restore_error| {
        operation_error(
            "WS_CREATE_BUNDLE_PARTIAL_FAILED",
            format!(
                "补偿元数据失败且恢复文件失败: {}; {restore_error}",
                cause.message
            ),
            false,
        )
    })?;
    if was_bdocx {
        super::listing::mark_bdocx(root, rel_path).map_err(|restore_error| {
            operation_error(
                "WS_CREATE_BUNDLE_PARTIAL_FAILED",
                format!(
                    "补偿元数据失败，文件已恢复但 BDocx 身份恢复失败: {}; {}",
                    cause.message, restore_error.message
                ),
                false,
            )
        })?;
    }
    Ok(())
}

pub fn discard_created_resource_if_guard_matches(
    conn: &Connection,
    workspace: &WorkspaceRef,
    diff_id: &str,
    logical_session_version: u64,
    logical_content_hash: &str,
) -> Result<Option<WorkspaceOperationEntry>, IpcError> {
    let Some(mut bundle) = load_create_document_bundle_by_diff(conn, diff_id)
        .map_err(|error| db_error("读取新建文档组合失败", error))?
    else {
        return Ok(None);
    };
    if bundle.workspace_id != workspace.workspace_id {
        return Err(operation_error(
            "WS_CREATE_BUNDLE_WORKSPACE_MISMATCH",
            "新建文档组合不属于当前 workspace",
            false,
        ));
    }
    let guard_ref = bundle.rejection_guard_ref.as_deref().ok_or_else(|| {
        operation_error(
            "WS_CREATE_BUNDLE_GUARD_MISSING",
            "新建文档拒绝保护基线缺失",
            false,
        )
    })?;
    let guard: CreateDocumentRejectionGuard = load_payload(conn, guard_ref)
        .map_err(|error| db_error("读取新建文档拒绝保护基线失败", error))?
        .ok_or_else(|| {
            operation_error(
                "WS_CREATE_BUNDLE_GUARD_MISSING",
                "新建文档拒绝保护基线不存在",
                false,
            )
        })?;
    let current_disk = capture_baseline(Path::new(&workspace.root_path), &bundle.target.rel_path)?;
    if guard.plan_id != bundle.plan_id
        || bundle.operation_id.as_deref() != Some(guard.operation_id.as_str())
        || current_disk != guard.disk_baseline
        || logical_session_version != guard.logical_session_version
        || logical_content_hash != guard.logical_content_hash
    {
        return Err(operation_error(
            "WS_CREATE_BUNDLE_BASELINE_ADVANCED",
            "新建文档已被继续编辑或外部修改；旧初始建议已失效，文件会保留",
            false,
        ));
    }
    let abs =
        super::resolve_in_workspace(Path::new(&workspace.root_path), &bundle.target.rel_path)?;
    let rollback_bytes = std::fs::read(&abs).map_err(|error| {
        operation_error(
            "WS_OPERATION_READ_FAILED",
            format!("读取新建文档补偿回滚内容失败: {error}"),
            true,
        )
    })?;
    let was_bdocx =
        super::listing::is_bdocx(Path::new(&workspace.root_path), &bundle.target.rel_path);
    super::fileops::delete_doc(&abs)?;
    if let Err(error) = super::listing::cleanup_bdocx_under(
        Path::new(&workspace.root_path),
        &bundle.target.rel_path,
    ) {
        restore_compensated_document(
            &abs,
            &rollback_bytes,
            Path::new(&workspace.root_path),
            &bundle.target.rel_path,
            was_bdocx,
            &error,
        )?;
        return Err(error);
    }
    let operation_id = format!("{}-discard", guard.operation_id);
    let receipt = WorkspaceOperationItemReceipt {
        item_id: format!("{}-discard-created", bundle.plan_id),
        seq: 0,
        status: WorkspaceOperationItemStatus::Completed,
        error_code: None,
        error_message: None,
        snapshot_ref: None,
        completed_at: now_millis(),
    };
    let entry = WorkspaceOperationEntry {
        entry_id: generated_id("wop-entry"),
        operation_id,
        plan_id: bundle.plan_id.clone(),
        workspace_id: bundle.workspace_id.clone(),
        before_manifest: vec![guard.disk_baseline],
        after_manifest: vec![capture_baseline(
            Path::new(&workspace.root_path),
            &bundle.target.rel_path,
        )?],
        item_receipts: vec![receipt],
        compensation_receipts: Vec::new(),
        restore_kind: "discard_created_resource".into(),
        restore_payload_ref: None,
        final_status: WorkspaceOperationStatus::Completed,
        created_at: now_millis(),
    };
    bundle.status = "discarded".into();
    if let Err(error) = persist_bundle_compensation(conn, &entry, &bundle, None) {
        std::fs::write(&abs, rollback_bytes).map_err(|restore_error| {
            operation_error(
                "WS_CREATE_BUNDLE_COMPENSATION_PARTIAL_FAILED",
                format!("删除后历史记录失败，且文件恢复失败: {error}; {restore_error}"),
                true,
            )
        })?;
        if was_bdocx {
            super::listing::mark_bdocx(Path::new(&workspace.root_path), &bundle.target.rel_path)?;
        }
        return Err(db_error("记录新建文档拒绝补偿历史失败，文件已恢复", error));
    }
    Ok(Some(entry))
}

pub fn compensate_create_document_projection_failure(
    conn: &Connection,
    workspace: &WorkspaceRef,
    plan_id: &str,
    logical_session_version: u64,
    logical_content_hash: &str,
) -> Result<WorkspaceOperationEntry, IpcError> {
    let mut bundle = load_create_document_bundle(conn, plan_id)
        .map_err(|error| db_error("读取新建文档组合失败", error))?
        .ok_or_else(|| {
            operation_error("WS_CREATE_BUNDLE_NOT_FOUND", "新建文档组合不存在", false)
        })?;
    let guard_ref = bundle.rejection_guard_ref.as_deref().ok_or_else(|| {
        operation_error(
            "WS_CREATE_BUNDLE_GUARD_MISSING",
            "新建文档投影补偿基线缺失",
            false,
        )
    })?;
    let guard: CreateDocumentRejectionGuard = load_payload(conn, guard_ref)
        .map_err(|error| db_error("读取新建文档投影补偿基线失败", error))?
        .ok_or_else(|| {
            operation_error(
                "WS_CREATE_BUNDLE_GUARD_MISSING",
                "新建文档投影补偿基线不存在",
                false,
            )
        })?;
    let current_disk = capture_baseline(Path::new(&workspace.root_path), &bundle.target.rel_path)?;
    if bundle.workspace_id != workspace.workspace_id
        || bundle.operation_id.as_deref() != Some(guard.operation_id.as_str())
        || current_disk != guard.disk_baseline
        || logical_session_version != guard.logical_session_version
        || logical_content_hash != guard.logical_content_hash
    {
        bundle.status = "partial_failed".into();
        upsert_create_document_bundle(conn, &bundle)
            .map_err(|error| db_error("更新新建文档组合失败态失败", error))?;
        return Err(operation_error(
            "WS_CREATE_BUNDLE_PROJECTION_PARTIAL_FAILED",
            "初始内容投影失败，且空文件基线已变化，无法安全补偿删除",
            false,
        ));
    }
    let abs =
        super::resolve_in_workspace(Path::new(&workspace.root_path), &bundle.target.rel_path)?;
    let rollback_bytes = std::fs::read(&abs).map_err(|error| {
        operation_error(
            "WS_OPERATION_READ_FAILED",
            format!("读取投影失败补偿回滚内容失败: {error}"),
            true,
        )
    })?;
    let was_bdocx =
        super::listing::is_bdocx(Path::new(&workspace.root_path), &bundle.target.rel_path);
    super::fileops::delete_doc(&abs)?;
    if let Err(error) = super::listing::cleanup_bdocx_under(
        Path::new(&workspace.root_path),
        &bundle.target.rel_path,
    ) {
        restore_compensated_document(
            &abs,
            &rollback_bytes,
            Path::new(&workspace.root_path),
            &bundle.target.rel_path,
            was_bdocx,
            &error,
        )?;
        return Err(error);
    }
    let entry = WorkspaceOperationEntry {
        entry_id: generated_id("wop-entry"),
        operation_id: format!("{}-projection-failed", guard.operation_id),
        plan_id: bundle.plan_id.clone(),
        workspace_id: bundle.workspace_id.clone(),
        before_manifest: vec![guard.disk_baseline],
        after_manifest: vec![capture_baseline(
            Path::new(&workspace.root_path),
            &bundle.target.rel_path,
        )?],
        item_receipts: vec![WorkspaceOperationItemReceipt {
            item_id: format!("{}-projection-failed", bundle.plan_id),
            seq: 0,
            status: WorkspaceOperationItemStatus::Compensated,
            error_code: None,
            error_message: None,
            snapshot_ref: None,
            completed_at: now_millis(),
        }],
        compensation_receipts: Vec::new(),
        restore_kind: "create_document_projection_failure".into(),
        restore_payload_ref: None,
        final_status: WorkspaceOperationStatus::FailedCompensated,
        created_at: now_millis(),
    };
    bundle.status = "projection_failed_compensated".into();
    if let Err(error) =
        persist_bundle_compensation(conn, &entry, &bundle, bundle.initial_diff_id.as_deref())
    {
        std::fs::write(&abs, rollback_bytes).map_err(|restore_error| {
            operation_error(
                "WS_CREATE_BUNDLE_PROJECTION_PARTIAL_FAILED",
                format!("投影补偿删除后历史记录失败，且文件恢复失败: {error}; {restore_error}"),
                true,
            )
        })?;
        if was_bdocx {
            super::listing::mark_bdocx(Path::new(&workspace.root_path), &bundle.target.rel_path)?;
        }
        return Err(db_error("记录新建文档投影补偿历史失败，文件已恢复", error));
    }
    Ok(entry)
}

/**
 * @GOV
 * codes:    BR-WS-PERSIST-003-EFFECT-WS-WS-OPERATION-002
 * type:     EFFECT
 * chain:    WS-OPERATION
 * rules:    BR-WS-PERSIST-003
 * term_ref: TERM-WS-003
 * boundary: in=persistedNonTerminalWorkspaceOperationPlansAtStartup | out=interruptedWorkspaceOperationPlans(noAutomaticExecution)
 */
pub fn interrupt_nonterminal_plans(conn: &Connection) -> rusqlite::Result<usize> {
    let mut stmt = conn.prepare(
        "SELECT plan_id FROM workspace_operation_plans \
         WHERE status IN ('planning','waiting_confirm','revalidating','executing','compensating')",
    )?;
    let plan_ids = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(stmt);
    for plan_id in &plan_ids {
        let plan = load_plan(conn, plan_id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        let progress_json: Option<String> = conn
            .query_row(
                "SELECT progress_json FROM workspace_operation_plans WHERE plan_id=?1",
                [plan_id],
                |row| row.get(0),
            )
            .optional()?
            .flatten();
        let progress: WorkspaceOperationProgress = progress_json
            .as_deref()
            .map(decode)
            .transpose()?
            .unwrap_or_default();
        conn.execute(
            "UPDATE workspace_operation_plans SET status='interrupted', updated_at=?2 WHERE plan_id=?1",
            params![plan_id, now_millis()],
        )?;
        let Some(operation_id) = plan.operation_id.as_deref() else {
            continue;
        };
        if progress.item_receipts.is_empty() && progress.compensation_receipts.is_empty() {
            continue;
        }
        if load_operation_entry(conn, operation_id)?.is_some() {
            continue;
        }
        let root_path: Option<String> = conn
            .query_row(
                "SELECT root_path FROM workspaces WHERE workspace_id=?1",
                [&plan.workspace_id],
                |row| row.get(0),
            )
            .optional()?;
        let after = root_path
            .as_deref()
            .map(|root| after_manifest(Path::new(root), &plan.operation_items))
            .unwrap_or_default();
        let entry = entry_from_receipts(
            &plan,
            operation_id,
            progress.item_receipts.clone(),
            progress.compensation_receipts.clone(),
            after,
            WorkspaceOperationStatus::Interrupted,
        );
        append_operation_entry(conn, &entry)?;
        let receipt = WorkspaceOperationReceipt {
            operation_id: operation_id.to_string(),
            plan_id: plan.plan_id.clone(),
            final_status: WorkspaceOperationStatus::Interrupted,
            item_receipts: progress.item_receipts,
            compensation_receipts: progress.compensation_receipts,
            entry_id: Some(entry.entry_id),
            invalidated_diff_ids: Vec::new(),
        };
        conn.execute(
            "UPDATE workspace_operation_plans SET outcome_json=?2 WHERE plan_id=?1",
            params![plan_id, encode(&receipt)?],
        )?;
    }
    Ok(plan_ids.len())
}

/// `ws.plan_operation` 的受限输入。调用方只声明意图；item_id、顺序、清单、基线和影响由 WS 生成。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceOperationIntent {
    pub kind: WorkspaceOperationKind,
    #[serde(default)]
    pub source_path: Option<String>,
    #[serde(default)]
    pub target_path: Option<String>,
    #[serde(default)]
    pub resource_kind: Option<WorkspaceResourceKind>,
    #[serde(default)]
    pub recursive: bool,
    #[serde(default)]
    pub restore_payload_ref: Option<String>,
}

fn operation_error(code: &str, message: impl AsRef<str>, retriable: bool) -> IpcError {
    IpcError::new(code, message.as_ref(), retriable, false)
}

fn db_error(context: &str, error: rusqlite::Error) -> IpcError {
    operation_error("WS_OPERATION_DB_ERROR", format!("{context}: {error}"), true)
}

fn normalize_rel_path(value: &str) -> Result<String, IpcError> {
    let normalized = value.trim().trim_matches('/').replace('\\', "/");
    if normalized.is_empty() {
        return Err(operation_error(
            "WS_OPERATION_ROOT_DENIED",
            "WorkspaceOperation 不允许以 workspace 根目录作为资源目标",
            false,
        ));
    }
    Ok(normalized)
}

fn modified_at_millis(metadata: &std::fs::Metadata) -> Option<i64> {
    metadata
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|value| value.as_millis() as i64)
}

fn collect_directory_members(root: &Path, directory: &Path) -> Result<Vec<PathBuf>, IpcError> {
    let mut members = vec![directory.to_path_buf()];
    let mut pending = vec![directory.to_path_buf()];
    while let Some(current) = pending.pop() {
        let entries = std::fs::read_dir(&current).map_err(|error| {
            operation_error(
                "WS_OPERATION_READ_FAILED",
                format!("读取目录清单失败: {error}"),
                true,
            )
        })?;
        for entry in entries {
            let path = entry
                .map_err(|error| {
                    operation_error(
                        "WS_OPERATION_READ_FAILED",
                        format!("读取目录成员失败: {error}"),
                        true,
                    )
                })?
                .path();
            let metadata = std::fs::symlink_metadata(&path).map_err(|error| {
                operation_error(
                    "WS_OPERATION_READ_FAILED",
                    format!("读取资源元数据失败: {error}"),
                    true,
                )
            })?;
            if metadata.file_type().is_symlink() {
                return Err(operation_error(
                    "WS_OPERATION_SYMLINK_DENIED",
                    format!(
                        "资源清单包含符号链接，拒绝执行: {}",
                        path.strip_prefix(root).unwrap_or(&path).display()
                    ),
                    false,
                ));
            }
            if metadata.is_dir() {
                pending.push(path.clone());
            }
            members.push(path);
        }
    }
    members.sort();
    Ok(members)
}

fn file_content_hash(path: &Path) -> Result<String, IpcError> {
    let bytes = std::fs::read(path).map_err(|error| {
        operation_error(
            "WS_OPERATION_READ_FAILED",
            format!("读取资源内容失败: {error}"),
            true,
        )
    })?;
    Ok(super::save::content_hash(&bytes).to_string())
}

fn baseline_for_abs(
    root: &Path,
    rel_path: &str,
    abs: &Path,
) -> Result<WorkspaceResourceBaseline, IpcError> {
    if !abs.exists() {
        return Ok(WorkspaceResourceBaseline {
            rel_path: rel_path.to_string(),
            exists: false,
            resource_kind: None,
            size: None,
            modified_at: None,
            content_hash: None,
            manifest_hash: None,
        });
    }
    let metadata = std::fs::symlink_metadata(abs).map_err(|error| {
        operation_error(
            "WS_OPERATION_READ_FAILED",
            format!("读取资源元数据失败: {error}"),
            true,
        )
    })?;
    if metadata.file_type().is_symlink() {
        return Err(operation_error(
            "WS_OPERATION_SYMLINK_DENIED",
            format!("拒绝操作符号链接资源: {rel_path}"),
            false,
        ));
    }
    if metadata.is_file() {
        return Ok(WorkspaceResourceBaseline {
            rel_path: rel_path.to_string(),
            exists: true,
            resource_kind: Some(WorkspaceResourceKind::File),
            size: Some(metadata.len()),
            modified_at: modified_at_millis(&metadata),
            content_hash: Some(file_content_hash(abs)?),
            manifest_hash: None,
        });
    }
    if !metadata.is_dir() {
        return Err(operation_error(
            "WS_OPERATION_RESOURCE_UNSUPPORTED",
            format!("不支持的资源类型: {rel_path}"),
            false,
        ));
    }
    let members = collect_directory_members(root, abs)?;
    let mut manifest = String::new();
    for member in members {
        let member_rel = member
            .strip_prefix(root)
            .unwrap_or(&member)
            .to_string_lossy()
            .replace('\\', "/");
        let member_meta = std::fs::symlink_metadata(&member).map_err(|error| {
            operation_error(
                "WS_OPERATION_READ_FAILED",
                format!("读取资源清单失败: {error}"),
                true,
            )
        })?;
        manifest.push_str(&member_rel);
        manifest.push('|');
        if member_meta.is_dir() {
            manifest.push_str("dir");
        } else {
            manifest.push_str("file|");
            manifest.push_str(&member_meta.len().to_string());
            manifest.push('|');
            manifest.push_str(&file_content_hash(&member)?);
        }
        manifest.push('\n');
    }
    Ok(WorkspaceResourceBaseline {
        rel_path: rel_path.to_string(),
        exists: true,
        resource_kind: Some(WorkspaceResourceKind::Folder),
        size: None,
        modified_at: modified_at_millis(&metadata),
        content_hash: None,
        manifest_hash: Some(super::save::content_hash(manifest.as_bytes()).to_string()),
    })
}

pub fn capture_baseline(
    root: &Path,
    rel_path: &str,
) -> Result<WorkspaceResourceBaseline, IpcError> {
    let rel_path = normalize_rel_path(rel_path)?;
    let abs = super::resolve_in_workspace(root, &rel_path)?;
    baseline_for_abs(root, &rel_path, &abs)
}

fn path_is_same_or_child(candidate: &str, target: &str) -> bool {
    candidate == target || candidate.starts_with(&format!("{target}/"))
}

fn affected_paths(items: &[WorkspaceOperationItem]) -> Vec<String> {
    let mut paths = Vec::new();
    for item in items {
        if let Some(path) = &item.source_rel_path {
            paths.push(path.clone());
        }
        if let Some(path) = &item.target_rel_path {
            paths.push(path.clone());
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

fn query_affected_rows(
    conn: &Connection,
    sql: &str,
    workspace_id: &str,
    targets: &[String],
) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare(sql)?;
    let candidates = stmt
        .query_map([workspace_id], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(candidates
        .into_iter()
        .filter(|candidate| {
            targets
                .iter()
                .any(|target| path_is_same_or_child(candidate, target))
        })
        .collect())
}

fn validate_intent(
    root: &Path,
    intent: &WorkspaceOperationIntent,
    seq: usize,
) -> Result<WorkspaceOperationItem, IpcError> {
    let source_rel_path = intent
        .source_path
        .as_deref()
        .map(normalize_rel_path)
        .transpose()?;
    let target_rel_path = intent
        .target_path
        .as_deref()
        .map(normalize_rel_path)
        .transpose()?;
    let (requires_source, requires_target, expected_kind) = match intent.kind {
        WorkspaceOperationKind::CreateDocument => (false, true, WorkspaceResourceKind::File),
        WorkspaceOperationKind::CreateFolder => (false, true, WorkspaceResourceKind::Folder),
        WorkspaceOperationKind::RenameResource | WorkspaceOperationKind::MoveResource => (
            true,
            true,
            intent.resource_kind.unwrap_or(WorkspaceResourceKind::File),
        ),
        WorkspaceOperationKind::DeleteResource => (
            true,
            false,
            intent.resource_kind.unwrap_or(WorkspaceResourceKind::File),
        ),
        WorkspaceOperationKind::RestoreResource => (
            false,
            true,
            intent.resource_kind.unwrap_or(WorkspaceResourceKind::File),
        ),
        WorkspaceOperationKind::DiscardCreatedResource => {
            (true, false, WorkspaceResourceKind::File)
        }
    };
    if !requires_source && source_rel_path.is_some() {
        return Err(operation_error(
            "WS_OPERATION_UNEXPECTED_SOURCE",
            "该资源操作不接受 sourcePath",
            false,
        ));
    }
    if !requires_target && target_rel_path.is_some() {
        return Err(operation_error(
            "WS_OPERATION_UNEXPECTED_TARGET",
            "该资源操作不接受 targetPath",
            false,
        ));
    }
    if requires_source && source_rel_path.is_none() {
        return Err(operation_error(
            "WS_OPERATION_SOURCE_REQUIRED",
            "该资源操作缺少 sourcePath",
            false,
        ));
    }
    if requires_target && target_rel_path.is_none() {
        return Err(operation_error(
            "WS_OPERATION_TARGET_REQUIRED",
            "该资源操作缺少 targetPath",
            false,
        ));
    }
    if intent.kind == WorkspaceOperationKind::MoveResource
        && expected_kind == WorkspaceResourceKind::Folder
    {
        return Err(operation_error(
            "WS_OPERATION_FOLDER_MOVE_UNSUPPORTED",
            "Agent 1.0 不支持目录移动",
            false,
        ));
    }
    if intent.kind == WorkspaceOperationKind::CreateDocument {
        let target = target_rel_path.as_deref().expect("validated target");
        let extension = Path::new(target)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !matches!(
            extension.as_str(),
            "md" | "markdown" | "html" | "htm" | "txt" | "docx"
        ) {
            return Err(operation_error(
                "WS_OPERATION_DOCUMENT_FORMAT_UNSUPPORTED",
                format!("Agent 1.0 不支持新建该文档格式: {target}"),
                false,
            ));
        }
    }
    if intent.kind == WorkspaceOperationKind::RestoreResource
        && intent
            .restore_payload_ref
            .as_deref()
            .unwrap_or("")
            .is_empty()
    {
        return Err(operation_error(
            "WS_OPERATION_RESTORE_PAYLOAD_REQUIRED",
            "恢复操作缺少 restorePayloadRef",
            false,
        ));
    }
    for path in source_rel_path.iter().chain(target_rel_path.iter()) {
        super::resolve_in_workspace(root, path)?;
    }
    Ok(WorkspaceOperationItem {
        item_id: format!("item-{seq}"),
        seq: seq as u32,
        kind: intent.kind,
        source_rel_path,
        target_rel_path,
        resource_kind: expected_kind,
        recursive: intent.recursive,
        restore_payload_ref: intent.restore_payload_ref.clone(),
    })
}

/**
 * @GOV
 * codes:    BR-WS-STATE-003-API-WS-WS-OPERATION-001, BR-WS-OBS-002-API-WS-WS-OPERATION-001
 * type:     API
 * chain:    WS-OPERATION
 * rules:    BR-WS-STATE-003, BR-WS-DATA-003, BR-WS-OBS-002
 * term_ref: TERM-WS-003, TERM-WS-004
 * boundary: in=WorkspaceOperationIntentsAndCurrentWorkspaceFacts | out=ImmutableWorkspaceOperationPlan(waitingConfirmWithVisibleConsequencesNoDiskMutation)
 */
pub fn plan_operation(
    conn: &Connection,
    workspace: &WorkspaceRef,
    intents: &[WorkspaceOperationIntent],
    task_run_id: Option<&str>,
    source_turn_id: Option<&str>,
    initial_content_ref: Option<&str>,
) -> Result<WorkspaceOperationPlan, IpcError> {
    if intents.is_empty() {
        return Err(operation_error(
            "WS_OPERATION_EMPTY",
            "WorkspaceOperationPlan 至少需要一项操作",
            false,
        ));
    }
    let root = Path::new(&workspace.root_path);
    let items = intents
        .iter()
        .enumerate()
        .map(|(seq, intent)| validate_intent(root, intent, seq))
        .collect::<Result<Vec<_>, _>>()?;
    let targets = affected_paths(&items);
    let mut baseline_set = Vec::new();
    for path in &targets {
        baseline_set.push(capture_baseline(root, path)?);
    }
    let mut frozen_manifest = Vec::new();
    let mut recursive_count = 0_u64;
    for item in &items {
        if let Some(path) = &item.source_rel_path {
            let baseline = capture_baseline(root, path)?;
            if !baseline.exists {
                return Err(operation_error(
                    "WS_OPERATION_SOURCE_NOT_FOUND",
                    format!("源资源不存在: {path}"),
                    false,
                ));
            }
            if baseline.resource_kind != Some(item.resource_kind) {
                return Err(operation_error(
                    "WS_OPERATION_RESOURCE_KIND_MISMATCH",
                    format!("源资源类型与计划不一致: {path}"),
                    false,
                ));
            }
            recursive_count += if item.resource_kind == WorkspaceResourceKind::Folder {
                collect_directory_members(root, &super::resolve_in_workspace(root, path)?)?.len()
                    as u64
            } else {
                1
            };
            frozen_manifest.push(baseline);
        }
        if item.source_rel_path.is_none() {
            recursive_count += 1;
        }
        if matches!(
            item.kind,
            WorkspaceOperationKind::CreateDocument
                | WorkspaceOperationKind::CreateFolder
                | WorkspaceOperationKind::RenameResource
                | WorkspaceOperationKind::MoveResource
                | WorkspaceOperationKind::RestoreResource
        ) {
            let target = item.target_rel_path.as_deref().expect("validated target");
            if capture_baseline(root, target)?.exists {
                return Err(operation_error(
                    "WS_OPERATION_TARGET_EXISTS",
                    format!("目标资源已存在: {target}"),
                    false,
                ));
            }
        }
    }
    let affected_sessions = query_affected_rows(
        conn,
        "SELECT rel_path FROM ed_session_snapshots WHERE workspace_id=?1",
        &workspace.workspace_id,
        &targets,
    )
    .map_err(|error| db_error("读取受影响编辑会话失败", error))?;
    let affected_diffs = query_affected_rows(
        conn,
        "SELECT rel_path FROM pending_diffs WHERE workspace_id=?1 AND status='pending'",
        &workspace.workspace_id,
        &targets,
    )
    .map_err(|error| db_error("读取受影响内容 Diff 失败", error))?;
    let now = now_millis();
    let mut plan = WorkspaceOperationPlan {
        plan_id: generated_id("wop-plan"),
        operation_id: None,
        idempotency_key: generated_id("wop-idem"),
        workspace_id: workspace.workspace_id.clone(),
        task_run_id: task_run_id.map(str::to_owned),
        source_turn_id: source_turn_id.map(str::to_owned),
        operation_items: items,
        frozen_manifest,
        baseline_set,
        consequences: WorkspaceOperationConsequences {
            affected_sessions,
            affected_diffs,
            affected_references: Vec::new(),
            recursive_count: 0,
            recoverable: intents.iter().all(|intent| {
                !matches!(intent.kind, WorkspaceOperationKind::DiscardCreatedResource)
            }),
        },
        compensation_policy: "fail_fast_compensate".into(),
        initial_content_ref: initial_content_ref.map(str::to_owned),
        decision_request_id: None,
        status: WorkspaceOperationStatus::Planning,
        created_at: now,
        updated_at: now,
    };
    plan.consequences.recursive_count = recursive_count;
    insert_plan(conn, &plan).map_err(|error| db_error("持久化资源操作计划失败", error))?;
    plan = advance_plan(
        conn,
        &plan.plan_id,
        WorkspaceOperationEvent::PlanReady,
        None,
    )
    .map_err(|error| db_error("推进资源操作计划失败", error))?;
    trace_operation(
        "plan_created",
        &plan.plan_id,
        None,
        None,
        Some("waiting_confirm"),
    );
    Ok(plan)
}

fn decision_identity(
    conn: &Connection,
    plan: &WorkspaceOperationPlan,
    request_id: &str,
    expected_outcome: &str,
) -> Result<(), IpcError> {
    if plan.decision_request_id.as_deref() != Some(request_id) {
        return Err(operation_error(
            "WS_OPERATION_DECISION_MISMATCH",
            "DecisionRequest 与 WorkspaceOperationPlan 不匹配",
            false,
        ));
    }
    let (task_run_id, status, outcome): (String, String, Option<String>) = conn
        .query_row(
            "SELECT task_run_id,status,outcome FROM decision_requests WHERE request_id=?1",
            [request_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|error| db_error("读取 DecisionRequest 失败", error))?;
    if plan.task_run_id.as_deref() != Some(task_run_id.as_str())
        || status != "resolved"
        || outcome.as_deref() != Some(expected_outcome)
    {
        return Err(operation_error(
            "WS_OPERATION_NOT_AUTHORIZED",
            "WorkspaceOperation 只能由绑定且已裁决的 DecisionRequest 执行",
            false,
        ));
    }
    Ok(())
}

fn has_dirty_affected_session(
    conn: &Connection,
    workspace_id: &str,
    targets: &[String],
) -> Result<Option<String>, IpcError> {
    let rows = query_affected_rows(
        conn,
        "SELECT rel_path FROM ed_session_snapshots WHERE workspace_id=?1 AND dirty=1",
        workspace_id,
        targets,
    )
    .map_err(|error| db_error("核验脏编辑会话失败", error))?;
    Ok(rows.into_iter().next())
}

fn current_baselines(
    root: &Path,
    frozen: &[WorkspaceResourceBaseline],
) -> Result<Vec<WorkspaceResourceBaseline>, IpcError> {
    frozen
        .iter()
        .map(|baseline| capture_baseline(root, &baseline.rel_path))
        .collect()
}

fn snapshot_resource(
    conn: &Connection,
    root: &Path,
    rel_path: &str,
    snapshot_ref: &str,
) -> Result<(), IpcError> {
    let abs = super::resolve_in_workspace(root, rel_path)?;
    let members = if abs.is_dir() {
        collect_directory_members(root, &abs)?
    } else {
        vec![abs]
    };
    for member in members {
        let relative = member
            .strip_prefix(root)
            .map_err(|_| {
                operation_error(
                    "WS_OPERATION_PATH_ESCAPE",
                    "资源快照路径越出 workspace",
                    false,
                )
            })?
            .to_string_lossy()
            .replace('\\', "/");
        let metadata = std::fs::symlink_metadata(&member).map_err(|error| {
            operation_error(
                "WS_OPERATION_SNAPSHOT_FAILED",
                format!("读取快照资源失败: {error}"),
                true,
            )
        })?;
        let (kind, content) = if metadata.is_dir() {
            ("folder", None)
        } else {
            let kind = if super::listing::is_bdocx(root, &relative) {
                "bdocx_file"
            } else {
                "file"
            };
            let bytes = std::fs::read(&member).map_err(|error| {
                operation_error(
                    "WS_OPERATION_SNAPSHOT_FAILED",
                    format!("读取文件快照失败: {error}"),
                    true,
                )
            })?;
            (kind, Some(bytes))
        };
        store_snapshot_member(conn, snapshot_ref, &relative, kind, content.as_deref())
            .map_err(|error| db_error("保存资源快照失败", error))?;
    }
    Ok(())
}

fn rebase_snapshot_path(original_root: &str, target_root: &str, member: &str) -> String {
    if member == original_root {
        return target_root.to_string();
    }
    let suffix = member
        .strip_prefix(&format!("{original_root}/"))
        .unwrap_or(member);
    format!("{target_root}/{suffix}")
}

fn restore_snapshot(
    conn: &Connection,
    root: &Path,
    snapshot_ref: &str,
    target_root: Option<&str>,
) -> Result<(), IpcError> {
    let members = load_snapshot_members(conn, snapshot_ref)
        .map_err(|error| db_error("读取资源快照失败", error))?;
    let original_root = members
        .first()
        .map(|member| member.0.clone())
        .ok_or_else(|| {
            operation_error(
                "WS_OPERATION_SNAPSHOT_MISSING",
                "资源快照为空或不可取回",
                false,
            )
        })?;
    let target_root = target_root.unwrap_or(&original_root);
    let target_abs = super::resolve_in_workspace(root, target_root)?;
    if target_abs.exists() {
        return Err(operation_error(
            "WS_OPERATION_TARGET_EXISTS",
            format!("恢复目标已存在: {target_root}"),
            false,
        ));
    }
    for (member, kind, content) in &members {
        let rebased = rebase_snapshot_path(&original_root, target_root, member);
        let abs = super::resolve_in_workspace(root, &rebased)?;
        if kind == "folder" {
            std::fs::create_dir_all(&abs).map_err(|error| {
                operation_error(
                    "WS_OPERATION_RESTORE_FAILED",
                    format!("恢复目录失败: {error}"),
                    true,
                )
            })?;
        } else {
            if let Some(parent) = abs.parent() {
                std::fs::create_dir_all(parent).map_err(|error| {
                    operation_error(
                        "WS_OPERATION_RESTORE_FAILED",
                        format!("恢复父目录失败: {error}"),
                        true,
                    )
                })?;
            }
            std::fs::write(&abs, content.as_deref().unwrap_or_default()).map_err(|error| {
                operation_error(
                    "WS_OPERATION_RESTORE_FAILED",
                    format!("恢复文件失败: {error}"),
                    true,
                )
            })?;
            if kind == "bdocx_file" {
                super::listing::mark_bdocx(root, &rebased)?;
            }
        }
    }
    Ok(())
}

fn empty_document_bytes(rel_path: &str) -> Result<Vec<u8>, IpcError> {
    if rel_path.to_ascii_lowercase().ends_with(".docx") {
        crate::doc::edit_chain::html_to_docx("<p></p>", None)
    } else {
        Ok(Vec::new())
    }
}

fn execute_item(
    conn: &Connection,
    root: &Path,
    operation_id: &str,
    item: &WorkspaceOperationItem,
) -> Result<Option<String>, IpcError> {
    match item.kind {
        WorkspaceOperationKind::CreateDocument => {
            let target = item.target_rel_path.as_deref().expect("validated target");
            let abs = super::resolve_in_workspace(root, target)?;
            let content = empty_document_bytes(target)?;
            super::fileops::create_doc(&abs, &content)?;
            if target.to_ascii_lowercase().ends_with(".docx") {
                super::listing::mark_bdocx(root, target)?;
            }
            Ok(None)
        }
        WorkspaceOperationKind::CreateFolder => {
            let target = item.target_rel_path.as_deref().expect("validated target");
            super::fileops::create_folder(&super::resolve_in_workspace(root, target)?)?;
            Ok(None)
        }
        WorkspaceOperationKind::RenameResource | WorkspaceOperationKind::MoveResource => {
            let source = item.source_rel_path.as_deref().expect("validated source");
            let target = item.target_rel_path.as_deref().expect("validated target");
            super::fileops::rename_doc(
                &super::resolve_in_workspace(root, source)?,
                &super::resolve_in_workspace(root, target)?,
            )?;
            super::listing::update_bdocx_for_rename(root, source, target)?;
            Ok(None)
        }
        WorkspaceOperationKind::DeleteResource | WorkspaceOperationKind::DiscardCreatedResource => {
            let source = item.source_rel_path.as_deref().expect("validated source");
            let snapshot_ref = format!("wop-snapshot-{operation_id}-{}", item.seq);
            snapshot_resource(conn, root, source, &snapshot_ref)?;
            let abs = super::resolve_in_workspace(root, source)?;
            if item.resource_kind == WorkspaceResourceKind::Folder {
                super::fileops::delete_folder(&abs, item.recursive)?;
            } else {
                super::fileops::delete_doc(&abs)?;
            }
            super::listing::cleanup_bdocx_under(root, source)?;
            Ok(Some(snapshot_ref))
        }
        WorkspaceOperationKind::RestoreResource => {
            let target = item.target_rel_path.as_deref().expect("validated target");
            let snapshot_ref = item
                .restore_payload_ref
                .as_deref()
                .expect("validated restore payload");
            restore_snapshot(conn, root, snapshot_ref, Some(target))?;
            Ok(Some(snapshot_ref.to_string()))
        }
    }
}

fn compensate_item(
    conn: &Connection,
    root: &Path,
    item: &WorkspaceOperationItem,
    snapshot_ref: Option<&str>,
) -> Result<(), IpcError> {
    match item.kind {
        WorkspaceOperationKind::CreateDocument => {
            let target = item.target_rel_path.as_deref().expect("validated target");
            super::fileops::delete_doc(&super::resolve_in_workspace(root, target)?)?;
            super::listing::cleanup_bdocx_under(root, target)
        }
        WorkspaceOperationKind::CreateFolder => {
            let target = item.target_rel_path.as_deref().expect("validated target");
            super::fileops::delete_folder(&super::resolve_in_workspace(root, target)?, true)
        }
        WorkspaceOperationKind::RenameResource | WorkspaceOperationKind::MoveResource => {
            let source = item.source_rel_path.as_deref().expect("validated source");
            let target = item.target_rel_path.as_deref().expect("validated target");
            super::fileops::rename_doc(
                &super::resolve_in_workspace(root, target)?,
                &super::resolve_in_workspace(root, source)?,
            )?;
            super::listing::update_bdocx_for_rename(root, target, source)
        }
        WorkspaceOperationKind::DeleteResource | WorkspaceOperationKind::DiscardCreatedResource => {
            let snapshot_ref = snapshot_ref.ok_or_else(|| {
                operation_error(
                    "WS_OPERATION_SNAPSHOT_MISSING",
                    "删除补偿缺少资源快照",
                    false,
                )
            })?;
            restore_snapshot(conn, root, snapshot_ref, None)
        }
        WorkspaceOperationKind::RestoreResource => {
            let target = item.target_rel_path.as_deref().expect("validated target");
            let abs = super::resolve_in_workspace(root, target)?;
            if item.resource_kind == WorkspaceResourceKind::Folder {
                super::fileops::delete_folder(&abs, true)
            } else {
                super::fileops::delete_doc(&abs)
            }
        }
    }
}

fn persist_outcome(
    conn: &Connection,
    plan_id: &str,
    receipt: &WorkspaceOperationReceipt,
) -> Result<(), IpcError> {
    conn.execute(
        "UPDATE workspace_operation_plans SET outcome_json=?2, updated_at=?3 WHERE plan_id=?1",
        params![
            plan_id,
            encode(receipt).map_err(|e| db_error("序列化执行回执失败", e))?,
            now_millis()
        ],
    )
    .map_err(|error| db_error("保存执行回执失败", error))?;
    Ok(())
}

fn persist_progress(
    conn: &Connection,
    plan_id: &str,
    progress: &WorkspaceOperationProgress,
) -> Result<(), IpcError> {
    conn.execute(
        "UPDATE workspace_operation_plans SET progress_json=?2, updated_at=?3 WHERE plan_id=?1",
        params![
            plan_id,
            encode(progress).map_err(|error| db_error("编码执行进度失败", error))?,
            now_millis(),
        ],
    )
    .map_err(|error| db_error("保存逐项执行进度失败", error))?;
    Ok(())
}

fn load_progress(conn: &Connection, plan_id: &str) -> Result<WorkspaceOperationProgress, IpcError> {
    let json: Option<String> = conn
        .query_row(
            "SELECT progress_json FROM workspace_operation_plans WHERE plan_id=?1",
            [plan_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| db_error("读取逐项执行进度失败", error))?
        .flatten();
    json.map(|value| decode(&value).map_err(|error| db_error("解析逐项执行进度失败", error)))
        .transpose()
        .map(Option::unwrap_or_default)
}

fn load_outcome(
    conn: &Connection,
    plan_id: &str,
) -> Result<Option<WorkspaceOperationReceipt>, IpcError> {
    let json: Option<String> = conn
        .query_row(
            "SELECT outcome_json FROM workspace_operation_plans WHERE plan_id=?1",
            [plan_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| db_error("读取执行回执失败", error))?
        .flatten();
    json.map(|value| decode(&value).map_err(|error| db_error("解析执行回执失败", error)))
        .transpose()
}

fn invalidate_affected_diffs(
    conn: &Connection,
    workspace_id: &str,
    targets: &[String],
    _operation_id: &str,
) -> Result<Vec<String>, IpcError> {
    let mut stmt = conn
        .prepare(
            "SELECT diff_id,rel_path FROM pending_diffs WHERE workspace_id=?1 AND status='pending'",
        )
        .map_err(|error| db_error("准备待失效内容 Diff 查询失败", error))?;
    let candidates = stmt
        .query_map([workspace_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| db_error("读取待失效内容 Diff 失败", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| db_error("汇总待失效内容 Diff 失败", error))?;
    let affected = candidates
        .into_iter()
        .filter(|(_, rel_path)| {
            targets
                .iter()
                .any(|target| path_is_same_or_child(rel_path, target))
        })
        .collect::<Vec<_>>();
    for (diff_id, _) in &affected {
        conn.execute(
            "UPDATE pending_diffs SET status='expired', terminal_reason='resource_baseline_changed' \
             WHERE workspace_id=?1 AND diff_id=?2 AND status='pending'",
            params![workspace_id, diff_id],
        )
        .map_err(|error| db_error("失效受影响内容 Diff 失败", error))?;
    }
    Ok(affected.into_iter().map(|(diff_id, _)| diff_id).collect())
}

fn after_manifest(root: &Path, items: &[WorkspaceOperationItem]) -> Vec<WorkspaceResourceBaseline> {
    affected_paths(items)
        .into_iter()
        .filter_map(|path| capture_baseline(root, &path).ok())
        .collect()
}

fn entry_from_receipts(
    plan: &WorkspaceOperationPlan,
    operation_id: &str,
    item_receipts: Vec<WorkspaceOperationItemReceipt>,
    compensation_receipts: Vec<WorkspaceOperationItemReceipt>,
    after_manifest: Vec<WorkspaceResourceBaseline>,
    final_status: WorkspaceOperationStatus,
) -> WorkspaceOperationEntry {
    let restore_payload_ref = item_receipts
        .iter()
        .find_map(|receipt| receipt.snapshot_ref.clone());
    WorkspaceOperationEntry {
        entry_id: generated_id("wop-entry"),
        operation_id: operation_id.to_string(),
        plan_id: plan.plan_id.clone(),
        workspace_id: plan.workspace_id.clone(),
        before_manifest: plan.baseline_set.clone(),
        after_manifest,
        item_receipts,
        compensation_receipts,
        restore_kind: "workspace_operation".into(),
        restore_payload_ref,
        final_status,
        created_at: now_millis(),
    }
}

/**
 * @GOV
 * codes:    BR-WS-STATE-003-EFFECT-WS-WS-OPERATION-003, BR-WS-PERSIST-003-EFFECT-WS-WS-OPERATION-003, BR-WS-OBS-002-EFFECT-WS-WS-OPERATION-002
 * type:     EFFECT
 * chain:    WS-OPERATION
 * rules:    BR-WS-STATE-003, BR-WS-DATA-003, BR-WS-OBS-002, BR-WS-PERSIST-003
 * term_ref: TERM-WS-003, TERM-WS-004, TERM-WS-006
 * boundary: in=ResolvedDecisionIdentityAndFrozenWorkspaceOperationPlan | out=DiskStateMutationWithItemReceiptsAndTypedTimelineEntry(idempotentFailFastReverseCompensated)
 */
pub fn execute_operation_plan(
    conn: &Connection,
    workspace: &WorkspaceRef,
    plan_id: &str,
    request_id: &str,
) -> Result<WorkspaceOperationReceipt, IpcError> {
    let mut plan = load_plan(conn, plan_id)
        .map_err(|error| db_error("读取资源操作计划失败", error))?
        .ok_or_else(|| operation_error("WS_OPERATION_NOT_FOUND", "资源操作计划不存在", false))?;
    if plan.workspace_id != workspace.workspace_id {
        return Err(operation_error(
            "WS_OPERATION_WORKSPACE_MISMATCH",
            "资源操作计划不属于当前 workspace",
            false,
        ));
    }
    decision_identity(conn, &plan, request_id, "confirm")?;
    if let Some(receipt) = load_outcome(conn, plan_id)? {
        return Ok(receipt);
    }
    if plan.status.is_terminal() || plan.status == WorkspaceOperationStatus::Interrupted {
        if let Some(operation_id) = plan.operation_id.as_deref() {
            if let Some(entry) = load_operation_entry(conn, operation_id)
                .map_err(|error| db_error("读取资源操作历史失败", error))?
            {
                let receipt = WorkspaceOperationReceipt {
                    operation_id: operation_id.to_string(),
                    plan_id: plan.plan_id.clone(),
                    final_status: entry.final_status,
                    item_receipts: entry.item_receipts,
                    compensation_receipts: entry.compensation_receipts,
                    entry_id: Some(entry.entry_id),
                    invalidated_diff_ids: Vec::new(),
                };
                persist_outcome(conn, plan_id, &receipt)?;
                return Ok(receipt);
            }
        }
        return Err(operation_error(
            "WS_OPERATION_TERMINAL",
            "资源操作已经终止，不能再次执行；恢复必须生成新计划",
            false,
        ));
    }
    let operation_id = generated_id("wop");
    plan = advance_plan(
        conn,
        plan_id,
        WorkspaceOperationEvent::UserConfirm,
        Some(&operation_id),
    )
    .map_err(|error| db_error("进入资源操作重验阶段失败", error))?;
    trace_operation(
        "revalidation_started",
        plan_id,
        Some(&operation_id),
        None,
        Some("revalidating"),
    );
    let root = Path::new(&workspace.root_path);
    let targets = affected_paths(&plan.operation_items);
    if let Some(dirty_path) = has_dirty_affected_session(conn, &plan.workspace_id, &targets)? {
        let _ = advance_plan(
            conn,
            plan_id,
            WorkspaceOperationEvent::BaselineDrift,
            Some(&operation_id),
        );
        trace_operation(
            "revalidation_failed",
            plan_id,
            Some(&operation_id),
            None,
            Some("WS_OPERATION_DIRTY_TARGET"),
        );
        return Err(operation_error(
            "WS_OPERATION_DIRTY_TARGET",
            format!("目标存在未保存的编辑会话，操作已过期: {dirty_path}"),
            false,
        ));
    }
    let current = current_baselines(root, &plan.baseline_set)?;
    if current != plan.baseline_set {
        let _ = advance_plan(
            conn,
            plan_id,
            WorkspaceOperationEvent::BaselineDrift,
            Some(&operation_id),
        );
        trace_operation(
            "revalidation_failed",
            plan_id,
            Some(&operation_id),
            None,
            Some("WS_OPERATION_BASELINE_DRIFT"),
        );
        return Err(operation_error(
            "WS_OPERATION_BASELINE_DRIFT",
            "资源在确认前已变化，旧计划已过期；请重新生成计划和确认卡",
            false,
        ));
    }
    plan = advance_plan(
        conn,
        plan_id,
        WorkspaceOperationEvent::BaselineValid,
        Some(&operation_id),
    )
    .map_err(|error| db_error("进入资源操作执行阶段失败", error))?;

    let mut progress = load_progress(conn, plan_id)?;
    let mut item_receipts = progress.item_receipts.clone();
    let mut completed: Vec<(&WorkspaceOperationItem, Option<String>)> = Vec::new();
    let mut failure: Option<IpcError> = None;
    for item in &plan.operation_items {
        let attempted = WorkspaceOperationItemReceipt {
            item_id: item.item_id.clone(),
            seq: item.seq,
            status: WorkspaceOperationItemStatus::Attempted,
            error_code: None,
            error_message: None,
            snapshot_ref: None,
            completed_at: now_millis(),
        };
        upsert_progress_receipt(&mut item_receipts, attempted);
        progress.item_receipts = item_receipts.clone();
        persist_progress(conn, plan_id, &progress)?;
        match execute_item(conn, root, &operation_id, item) {
            Ok(snapshot_ref) => {
                upsert_progress_receipt(
                    &mut item_receipts,
                    WorkspaceOperationItemReceipt {
                        item_id: item.item_id.clone(),
                        seq: item.seq,
                        status: WorkspaceOperationItemStatus::Completed,
                        error_code: None,
                        error_message: None,
                        snapshot_ref: snapshot_ref.clone(),
                        completed_at: now_millis(),
                    },
                );
                progress.item_receipts = item_receipts.clone();
                persist_progress(conn, plan_id, &progress)?;
                completed.push((item, snapshot_ref));
                trace_operation(
                    "item_receipt",
                    plan_id,
                    Some(&operation_id),
                    Some(item.seq),
                    Some("completed"),
                );
            }
            Err(error) => {
                upsert_progress_receipt(
                    &mut item_receipts,
                    WorkspaceOperationItemReceipt {
                        item_id: item.item_id.clone(),
                        seq: item.seq,
                        status: WorkspaceOperationItemStatus::Failed,
                        error_code: Some(error.code.clone()),
                        error_message: Some(error.message.clone()),
                        snapshot_ref: None,
                        completed_at: now_millis(),
                    },
                );
                progress.item_receipts = item_receipts.clone();
                persist_progress(conn, plan_id, &progress)?;
                failure = Some(error);
                trace_operation(
                    "item_receipt",
                    plan_id,
                    Some(&operation_id),
                    Some(item.seq),
                    failure.as_ref().map(|value| value.code.as_str()),
                );
                break;
            }
        }
    }

    let mut compensation_receipts = progress.compensation_receipts.clone();
    let (final_status, terminal_event) = if failure.is_none() {
        (
            WorkspaceOperationStatus::Completed,
            WorkspaceOperationEvent::AllItemsOk,
        )
    } else {
        advance_plan(
            conn,
            plan_id,
            WorkspaceOperationEvent::ItemFail,
            Some(&operation_id),
        )
        .map_err(|error| db_error("进入资源操作补偿阶段失败", error))?;
        let mut compensation_failed = false;
        for (item, snapshot_ref) in completed.into_iter().rev() {
            upsert_progress_receipt(
                &mut compensation_receipts,
                WorkspaceOperationItemReceipt {
                    item_id: item.item_id.clone(),
                    seq: item.seq,
                    status: WorkspaceOperationItemStatus::Attempted,
                    error_code: None,
                    error_message: None,
                    snapshot_ref: snapshot_ref.clone(),
                    completed_at: now_millis(),
                },
            );
            progress.compensation_receipts = compensation_receipts.clone();
            persist_progress(conn, plan_id, &progress)?;
            match compensate_item(conn, root, item, snapshot_ref.as_deref()) {
                Ok(()) => upsert_progress_receipt(
                    &mut compensation_receipts,
                    WorkspaceOperationItemReceipt {
                        item_id: item.item_id.clone(),
                        seq: item.seq,
                        status: WorkspaceOperationItemStatus::Compensated,
                        error_code: None,
                        error_message: None,
                        snapshot_ref,
                        completed_at: now_millis(),
                    },
                ),
                Err(error) => {
                    compensation_failed = true;
                    upsert_progress_receipt(
                        &mut compensation_receipts,
                        WorkspaceOperationItemReceipt {
                            item_id: item.item_id.clone(),
                            seq: item.seq,
                            status: WorkspaceOperationItemStatus::Failed,
                            error_code: Some(error.code),
                            error_message: Some(error.message),
                            snapshot_ref,
                            completed_at: now_millis(),
                        },
                    );
                }
            }
            progress.compensation_receipts = compensation_receipts.clone();
            persist_progress(conn, plan_id, &progress)?;
        }
        if compensation_failed {
            (
                WorkspaceOperationStatus::PartialFailed,
                WorkspaceOperationEvent::CompensationFail,
            )
        } else {
            (
                WorkspaceOperationStatus::FailedCompensated,
                WorkspaceOperationEvent::CompensationOk,
            )
        }
    };

    let current_after = after_manifest(root, &plan.operation_items);
    let entry = entry_from_receipts(
        &plan,
        &operation_id,
        item_receipts.clone(),
        compensation_receipts.clone(),
        current_after,
        final_status,
    );
    // 文件系统动作先完成，随后在同一数据库事务内原子写入失效 Diff、历史条目、
    // 执行回执和终态。任一步失败时计划仍停留在 Executing/Compensating，启动恢复
    // 可依据 progress_json 将真实磁盘结果收束为 Interrupted，而不会出现“终态已写入、
    // 回执或时间轴缺失”的不可恢复窗口。
    let transaction = conn
        .unchecked_transaction()
        .map_err(|error| db_error("启动资源操作收束事务失败", error))?;
    let invalidated_diff_ids = if final_status == WorkspaceOperationStatus::Completed {
        invalidate_affected_diffs(&transaction, &plan.workspace_id, &targets, &operation_id)?
    } else {
        Vec::new()
    };
    let receipt = WorkspaceOperationReceipt {
        operation_id,
        plan_id: plan.plan_id,
        final_status,
        item_receipts,
        compensation_receipts,
        entry_id: Some(entry.entry_id.clone()),
        invalidated_diff_ids,
    };
    append_operation_entry(&transaction, &entry)
        .map_err(|error| db_error("记录 WorkspaceTimeline 资源条目失败", error))?;
    if final_status == WorkspaceOperationStatus::Completed {
        transaction
            .execute(
                "UPDATE create_document_bundles SET operation_id=?2, status='resource_created', updated_at=?3 \
                 WHERE plan_id=?1 AND workspace_id=?4",
                params![plan_id, receipt.operation_id, now_millis(), plan.workspace_id],
            )
            .map_err(|error| db_error("绑定新建文档组合资源结果失败", error))?;
    }
    persist_outcome(&transaction, plan_id, &receipt)?;
    advance_plan(
        &transaction,
        plan_id,
        terminal_event,
        Some(&receipt.operation_id),
    )
    .map_err(|error| db_error("收束资源操作计划失败", error))?;
    transaction
        .commit()
        .map_err(|error| db_error("提交资源操作收束事务失败", error))?;
    trace_operation(
        "terminal",
        plan_id,
        Some(&receipt.operation_id),
        None,
        Some(status_text(receipt.final_status)),
    );
    Ok(receipt)
}

pub fn cancel_operation_plan(
    conn: &Connection,
    workspace_id: &str,
    plan_id: &str,
    request_id: &str,
) -> Result<WorkspaceOperationPlan, IpcError> {
    let plan = load_plan(conn, plan_id)
        .map_err(|error| db_error("读取资源操作计划失败", error))?
        .ok_or_else(|| operation_error("WS_OPERATION_NOT_FOUND", "资源操作计划不存在", false))?;
    if plan.workspace_id != workspace_id {
        return Err(operation_error(
            "WS_OPERATION_WORKSPACE_MISMATCH",
            "资源操作计划不属于当前 workspace",
            false,
        ));
    }
    decision_identity(conn, &plan, request_id, "cancel")?;
    if plan.status == WorkspaceOperationStatus::Rejected {
        return Ok(plan);
    }
    advance_plan(conn, plan_id, WorkspaceOperationEvent::UserReject, None)
        .map_err(|error| db_error("拒绝资源操作计划失败", error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::types::{
        WorkspaceOperationConsequences, WorkspaceOperationItem, WorkspaceOperationKind,
        WorkspaceResourceKind,
    };

    fn fresh_db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        db::migrate(&mut conn).unwrap();
        conn
    }

    fn plan() -> WorkspaceOperationPlan {
        WorkspaceOperationPlan {
            plan_id: "plan-1".into(),
            operation_id: None,
            idempotency_key: "idem-1".into(),
            workspace_id: "ws-1".into(),
            task_run_id: Some("run-1".into()),
            source_turn_id: Some("turn-1".into()),
            operation_items: vec![WorkspaceOperationItem {
                item_id: "item-1".into(),
                seq: 0,
                kind: WorkspaceOperationKind::DeleteResource,
                source_rel_path: Some("a.md".into()),
                target_rel_path: None,
                resource_kind: WorkspaceResourceKind::File,
                recursive: false,
                restore_payload_ref: None,
            }],
            frozen_manifest: vec![],
            baseline_set: vec![],
            consequences: WorkspaceOperationConsequences::default(),
            compensation_policy: "fail_fast_compensate".into(),
            initial_content_ref: None,
            decision_request_id: None,
            status: WorkspaceOperationStatus::Planning,
            created_at: 1,
            updated_at: 1,
        }
    }

    fn temp_workspace(tag: &str) -> (Connection, WorkspaceRef, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "binder-wop-{tag}-{}-{}",
            std::process::id(),
            generated_id("test")
        ));
        std::fs::create_dir_all(&root).unwrap();
        let conn = fresh_db();
        conn.execute(
            "INSERT INTO workspaces (workspace_id,root_path,is_active,opened_at) VALUES ('ws-1',?1,1,0)",
            [root.to_string_lossy().as_ref()],
        )
        .unwrap();
        let workspace = WorkspaceRef {
            workspace_id: "ws-1".into(),
            root_path: root.to_string_lossy().into_owned(),
            is_active: true,
        };
        (conn, workspace, root)
    }

    // covers: BR-WS-STATE-003
    // covers: BR-WS-PERSIST-003
    #[test]
    fn sm_ws04_forbids_skipping_confirmation_and_revalidation() {
        use WorkspaceOperationEvent::*;
        use WorkspaceOperationStatus::*;
        assert_eq!(transition(Planning, PlanReady), Some(WaitingConfirm));
        assert_eq!(transition(WaitingConfirm, UserConfirm), Some(Revalidating));
        assert_eq!(transition(Revalidating, BaselineValid), Some(Executing));
        assert_eq!(transition(Executing, AllItemsOk), Some(Completed));
        assert_eq!(transition(WaitingConfirm, AllItemsOk), None);
        assert_eq!(transition(Rejected, UserConfirm), None);
        assert_eq!(
            transition(Compensating, CompensationFail),
            Some(PartialFailed)
        );
    }

    // covers: BR-WS-DATA-003
    #[test]
    fn persisted_plan_keeps_frozen_items_and_binds_one_decision() {
        let conn = fresh_db();
        let p = plan();
        insert_plan(&conn, &p).unwrap();
        let waiting =
            advance_plan(&conn, &p.plan_id, WorkspaceOperationEvent::PlanReady, None).unwrap();
        assert_eq!(waiting.operation_items, p.operation_items);
        let bound = bind_decision_request(&conn, &p.plan_id, "decision-1").unwrap();
        assert_eq!(bound.decision_request_id.as_deref(), Some("decision-1"));
        assert!(bind_decision_request(&conn, &p.plan_id, "decision-2").is_err());
        assert!(
            insert_plan(&conn, &p).is_err(),
            "plan identity and idempotency key are unique"
        );
    }

    // covers: BR-WS-PERSIST-003
    #[test]
    fn restart_interrupts_nonterminal_plans_without_resuming_execution() {
        let conn = fresh_db();
        let p = plan();
        insert_plan(&conn, &p).unwrap();
        advance_plan(&conn, &p.plan_id, WorkspaceOperationEvent::PlanReady, None).unwrap();
        assert_eq!(interrupt_nonterminal_plans(&conn).unwrap(), 1);
        assert_eq!(
            load_plan(&conn, &p.plan_id).unwrap().unwrap().status,
            WorkspaceOperationStatus::Interrupted
        );
        assert_eq!(
            transition(
                WorkspaceOperationStatus::Interrupted,
                WorkspaceOperationEvent::UserConfirm
            ),
            None
        );
    }

    // covers: BR-WS-PERSIST-003
    // 异常退出时逐项 journal 必须投影为可审计 Interrupted 条目；恢复仍生成新计划输入，绝不续跑旧计划。
    #[test]
    fn restart_materializes_persisted_item_progress_as_interrupted_recovery_input() {
        let (conn, workspace, root) = temp_workspace("interrupted-progress");
        let mut p = plan();
        p.operation_items[0] = WorkspaceOperationItem {
            item_id: "item-create".into(),
            seq: 0,
            kind: WorkspaceOperationKind::CreateDocument,
            source_rel_path: None,
            target_rel_path: Some("created.md".into()),
            resource_kind: WorkspaceResourceKind::File,
            recursive: false,
            restore_payload_ref: None,
        };
        insert_plan(&conn, &p).unwrap();
        advance_plan(&conn, &p.plan_id, WorkspaceOperationEvent::PlanReady, None).unwrap();
        advance_plan(
            &conn,
            &p.plan_id,
            WorkspaceOperationEvent::UserConfirm,
            Some("operation-interrupted"),
        )
        .unwrap();
        advance_plan(
            &conn,
            &p.plan_id,
            WorkspaceOperationEvent::BaselineValid,
            Some("operation-interrupted"),
        )
        .unwrap();
        std::fs::write(root.join("created.md"), b"created before crash").unwrap();
        persist_progress(
            &conn,
            &p.plan_id,
            &WorkspaceOperationProgress {
                item_receipts: vec![WorkspaceOperationItemReceipt {
                    item_id: "item-create".into(),
                    seq: 0,
                    status: WorkspaceOperationItemStatus::Completed,
                    error_code: None,
                    error_message: None,
                    snapshot_ref: None,
                    completed_at: now_millis(),
                }],
                compensation_receipts: Vec::new(),
            },
        )
        .unwrap();

        assert_eq!(interrupt_nonterminal_plans(&conn).unwrap(), 1);
        let entry = load_operation_entry(&conn, "operation-interrupted")
            .unwrap()
            .expect("journal must become an interrupted timeline entry");
        assert_eq!(entry.final_status, WorkspaceOperationStatus::Interrupted);
        let reverse = restore_intents_from_entry(&conn, &entry).unwrap();
        assert_eq!(reverse.len(), 1);
        assert_eq!(reverse[0].kind, WorkspaceOperationKind::DeleteResource);
        assert_eq!(reverse[0].source_path.as_deref(), Some("created.md"));
        assert!(root.join("created.md").exists(), "启动恢复不得自动补偿磁盘");
        let _ = std::fs::remove_dir_all(root);
        let _ = workspace;
    }

    // covers: BR-WS-STATE-003
    // covers: BR-WS-DATA-003
    // covers: BR-WS-OBS-002
    #[test]
    fn plan_freezes_real_baseline_and_execute_requires_bound_resolved_decision() {
        let (conn, workspace, root) = temp_workspace("delete");
        std::fs::write(root.join("a.md"), b"body").unwrap();
        let task_run_id = crate::ag::task::begin_turn(
            &conn,
            &workspace.workspace_id,
            "session-1",
            Some("delete a"),
        )
        .unwrap();
        let plan = plan_operation(
            &conn,
            &workspace,
            &[WorkspaceOperationIntent {
                kind: WorkspaceOperationKind::DeleteResource,
                source_path: Some("a.md".into()),
                target_path: None,
                resource_kind: Some(WorkspaceResourceKind::File),
                recursive: false,
                restore_payload_ref: None,
            }],
            Some(&task_run_id),
            Some("turn-1"),
            None,
        )
        .unwrap();
        assert_eq!(plan.status, WorkspaceOperationStatus::WaitingConfirm);
        assert_eq!(
            plan.baseline_set[0].content_hash.as_deref(),
            Some(
                super::super::save::content_hash(b"body")
                    .to_string()
                    .as_str()
            )
        );
        assert_eq!(
            execute_operation_plan(&conn, &workspace, &plan.plan_id, "not-a-decision")
                .unwrap_err()
                .code,
            "WS_OPERATION_DECISION_MISMATCH"
        );
        assert!(root.join("a.md").exists(), "未授权不得产生磁盘副作用");

        let decision = crate::ag::task::propose_decision(
            &conn,
            &task_run_id,
            "permission",
            serde_json::json!({"workspaceOperationPlanId": plan.plan_id}),
        )
        .unwrap();
        bind_decision_request(&conn, &plan.plan_id, &decision.request_id).unwrap();
        crate::ag::task::resolve_decision(&conn, &decision.request_id, "confirm").unwrap();
        let receipt =
            execute_operation_plan(&conn, &workspace, &plan.plan_id, &decision.request_id).unwrap();
        assert_eq!(receipt.final_status, WorkspaceOperationStatus::Completed);
        assert!(!root.join("a.md").exists());
        assert!(receipt.item_receipts[0].snapshot_ref.is_some());
        let repeated =
            execute_operation_plan(&conn, &workspace, &plan.plan_id, &decision.request_id).unwrap();
        assert_eq!(repeated.operation_id, receipt.operation_id);
        assert_eq!(
            conn.query_row::<i64, _, _>(
                "SELECT COUNT(*) FROM workspace_operation_entries",
                [],
                |row| row.get(0)
            )
            .unwrap(),
            1,
            "重复确认/执行不能重复写盘或历史"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    // covers: BR-WS-STATE-003
    // covers: BR-WS-PERSIST-003
    #[test]
    fn baseline_drift_expires_plan_with_zero_disk_mutation() {
        let (conn, workspace, root) = temp_workspace("drift");
        std::fs::write(root.join("a.md"), b"v1").unwrap();
        let task_run_id = crate::ag::task::begin_turn(&conn, "ws-1", "session-1", None).unwrap();
        let plan = plan_operation(
            &conn,
            &workspace,
            &[WorkspaceOperationIntent {
                kind: WorkspaceOperationKind::DeleteResource,
                source_path: Some("a.md".into()),
                target_path: None,
                resource_kind: Some(WorkspaceResourceKind::File),
                recursive: false,
                restore_payload_ref: None,
            }],
            Some(&task_run_id),
            None,
            None,
        )
        .unwrap();
        let decision = crate::ag::task::propose_decision(
            &conn,
            &task_run_id,
            "permission",
            serde_json::json!({}),
        )
        .unwrap();
        bind_decision_request(&conn, &plan.plan_id, &decision.request_id).unwrap();
        crate::ag::task::resolve_decision(&conn, &decision.request_id, "confirm").unwrap();
        std::fs::write(root.join("a.md"), b"external change").unwrap();
        assert_eq!(
            execute_operation_plan(&conn, &workspace, &plan.plan_id, &decision.request_id)
                .unwrap_err()
                .code,
            "WS_OPERATION_BASELINE_DRIFT"
        );
        assert_eq!(
            std::fs::read(root.join("a.md")).unwrap(),
            b"external change"
        );
        assert_eq!(
            load_plan(&conn, &plan.plan_id).unwrap().unwrap().status,
            WorkspaceOperationStatus::Expired
        );
        let _ = std::fs::remove_dir_all(root);
    }

    // covers: BR-WS-PERSIST-003
    // covers: BR-WS-DATA-003
    #[test]
    fn batch_failure_stops_and_reverse_compensates_completed_items() {
        let (conn, workspace, root) = temp_workspace("compensate");
        let task_run_id = crate::ag::task::begin_turn(&conn, "ws-1", "session-1", None).unwrap();
        let duplicate_create = WorkspaceOperationIntent {
            kind: WorkspaceOperationKind::CreateDocument,
            source_path: None,
            target_path: Some("same.md".into()),
            resource_kind: Some(WorkspaceResourceKind::File),
            recursive: false,
            restore_payload_ref: None,
        };
        let plan = plan_operation(
            &conn,
            &workspace,
            &[duplicate_create.clone(), duplicate_create],
            Some(&task_run_id),
            None,
            None,
        )
        .unwrap();
        let decision = crate::ag::task::propose_decision(
            &conn,
            &task_run_id,
            "permission",
            serde_json::json!({}),
        )
        .unwrap();
        bind_decision_request(&conn, &plan.plan_id, &decision.request_id).unwrap();
        crate::ag::task::resolve_decision(&conn, &decision.request_id, "confirm").unwrap();
        let receipt =
            execute_operation_plan(&conn, &workspace, &plan.plan_id, &decision.request_id).unwrap();
        assert_eq!(
            receipt.final_status,
            WorkspaceOperationStatus::FailedCompensated
        );
        assert!(!root.join("same.md").exists(), "已完成的第一项必须反向补偿");
        assert_eq!(receipt.item_receipts.len(), 2);
        assert_eq!(receipt.compensation_receipts.len(), 1);
        let entry = load_operation_entry(&conn, &receipt.operation_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            restore_intents_from_entry(&conn, &entry).unwrap_err().code,
            "WS_OPERATION_NOT_RECOVERABLE",
            "已经补偿的完成项不得再次出现在恢复计划里"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    // covers: BR-WS-DATA-003
    #[test]
    fn folder_plan_reports_real_recursive_impact_count() {
        let (conn, workspace, root) = temp_workspace("recursive-count");
        std::fs::create_dir_all(root.join("folder/nested")).unwrap();
        std::fs::write(root.join("folder/a.md"), b"a").unwrap();
        std::fs::write(root.join("folder/nested/b.md"), b"b").unwrap();
        let plan = plan_operation(
            &conn,
            &workspace,
            &[WorkspaceOperationIntent {
                kind: WorkspaceOperationKind::DeleteResource,
                source_path: Some("folder".into()),
                target_path: None,
                resource_kind: Some(WorkspaceResourceKind::Folder),
                recursive: true,
                restore_payload_ref: None,
            }],
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(
            plan.consequences.recursive_count, 4,
            "目标目录本身与三个子资源均计入影响"
        );
        assert!(root.join("folder").exists(), "生成确认卡不得改变磁盘");
        let _ = std::fs::remove_dir_all(root);
    }

    // covers: BR-WS-STATE-003
    // covers: BR-WS-DATA-003
    // covers: BR-WS-PERSIST-003
    #[test]
    fn create_document_rejection_requires_resource_disk_and_logical_guard() {
        let (conn, workspace, root) = temp_workspace("create-bundle-guard");
        std::fs::write(root.join("created.md"), b"").unwrap();
        let baseline = capture_baseline(&root, "created.md").unwrap();
        let guard_ref = "guard-create-1";
        let guard = CreateDocumentRejectionGuard {
            plan_id: "plan-create-1".into(),
            operation_id: "operation-create-1".into(),
            target: DocRef {
                workspace_id: workspace.workspace_id.clone(),
                rel_path: "created.md".into(),
                doc_type: "md".into(),
            },
            disk_baseline: baseline,
            logical_session_version: 1,
            logical_content_hash: "empty-hash".into(),
            created_at: now_millis(),
        };
        store_payload(
            &conn,
            guard_ref,
            &workspace.workspace_id,
            "create_document_rejection_guard",
            &guard,
        )
        .unwrap();
        upsert_create_document_bundle(
            &conn,
            &CreateDocumentBundle {
                plan_id: guard.plan_id.clone(),
                workspace_id: workspace.workspace_id.clone(),
                target: guard.target.clone(),
                initial_content_ref: "initial-content-1".into(),
                operation_id: Some(guard.operation_id.clone()),
                initial_diff_id: Some("diff-create-1".into()),
                rejection_guard_ref: Some(guard_ref.into()),
                status: "initial_diff_pending".into(),
            },
        )
        .unwrap();

        acknowledge_create_document_preview(&conn, "diff-create-1", 2, "after-hash").unwrap();
        assert!(validate_create_document_diff_guard(
            &conn,
            &workspace,
            "diff-create-1",
            2,
            "after-hash",
        )
        .unwrap());
        assert_eq!(
            validate_create_document_diff_guard(
                &conn,
                &workspace,
                "diff-create-1",
                3,
                "manual-edit-hash",
            )
            .unwrap_err()
            .code,
            "WS_CREATE_BUNDLE_BASELINE_ADVANCED",
            "手动编辑推进基线后，旧初始 Diff 的显式接受也必须失效"
        );
        let advanced = discard_created_resource_if_guard_matches(
            &conn,
            &workspace,
            "diff-create-1",
            3,
            "manual-edit-hash",
        )
        .unwrap_err();
        assert_eq!(advanced.code, "WS_CREATE_BUNDLE_BASELINE_ADVANCED");
        assert!(root.join("created.md").exists(), "guard 失配绝不能删除文件");

        let entry = discard_created_resource_if_guard_matches(
            &conn,
            &workspace,
            "diff-create-1",
            2,
            "after-hash",
        )
        .unwrap()
        .expect("绑定初始 Diff 拒绝应记录资源补偿条目");
        assert!(!root.join("created.md").exists());
        assert_eq!(entry.restore_kind, "discard_created_resource");
        assert_eq!(
            list_operation_entries(&conn, &workspace.workspace_id)
                .unwrap()
                .len(),
            1
        );
        let _ = std::fs::remove_dir_all(root);
    }

    // covers: BR-WS-PERSIST-003
    #[test]
    fn create_document_open_failure_compensates_disk_and_bundle_atomically() {
        let (conn, workspace, root) = temp_workspace("create-open-failure");
        std::fs::write(root.join("created.md"), b"").unwrap();
        let created_baseline = capture_baseline(&root, "created.md").unwrap();
        let create_entry = WorkspaceOperationEntry {
            entry_id: "entry-create-open".into(),
            operation_id: "operation-create-open".into(),
            plan_id: "plan-create-open".into(),
            workspace_id: workspace.workspace_id.clone(),
            before_manifest: vec![WorkspaceResourceBaseline {
                rel_path: "created.md".into(),
                exists: false,
                resource_kind: None,
                size: None,
                modified_at: None,
                content_hash: None,
                manifest_hash: None,
            }],
            after_manifest: vec![created_baseline],
            item_receipts: Vec::new(),
            compensation_receipts: Vec::new(),
            restore_kind: "workspace_operation".into(),
            restore_payload_ref: None,
            final_status: WorkspaceOperationStatus::Completed,
            created_at: now_millis(),
        };
        append_operation_entry(&conn, &create_entry).unwrap();
        upsert_create_document_bundle(
            &conn,
            &CreateDocumentBundle {
                plan_id: "plan-create-open".into(),
                workspace_id: workspace.workspace_id.clone(),
                target: DocRef {
                    workspace_id: workspace.workspace_id.clone(),
                    rel_path: "created.md".into(),
                    doc_type: "md".into(),
                },
                initial_content_ref: "initial-content".into(),
                operation_id: Some("operation-create-open".into()),
                initial_diff_id: None,
                rejection_guard_ref: None,
                status: "resource_created".into(),
            },
        )
        .unwrap();

        let compensation =
            compensate_create_document_open_failure(&conn, &workspace, "plan-create-open").unwrap();
        assert_eq!(
            compensation.final_status,
            WorkspaceOperationStatus::FailedCompensated
        );
        assert!(!root.join("created.md").exists());
        assert_eq!(
            load_create_document_bundle(&conn, "plan-create-open")
                .unwrap()
                .unwrap()
                .status,
            "open_failed_compensated"
        );
        assert!(load_operation_entry(&conn, &compensation.operation_id)
            .unwrap()
            .is_some());
        let _ = std::fs::remove_dir_all(root);
    }

    // covers: BR-WS-DATA-003
    // covers: BR-WS-PERSIST-003
    #[test]
    fn workspace_timeline_restore_builds_a_new_reverse_plan_intent() {
        let (conn, workspace, root) = temp_workspace("restore-intent");
        std::fs::write(root.join("a.md"), b"body").unwrap();
        let task_run_id = crate::ag::task::begin_turn(&conn, "ws-1", "session-1", None).unwrap();
        let plan = plan_operation(
            &conn,
            &workspace,
            &[WorkspaceOperationIntent {
                kind: WorkspaceOperationKind::DeleteResource,
                source_path: Some("a.md".into()),
                target_path: None,
                resource_kind: Some(WorkspaceResourceKind::File),
                recursive: false,
                restore_payload_ref: None,
            }],
            Some(&task_run_id),
            Some("turn-delete"),
            None,
        )
        .unwrap();
        let decision = crate::ag::task::propose_decision(
            &conn,
            &task_run_id,
            "permission",
            serde_json::json!({}),
        )
        .unwrap();
        bind_decision_request(&conn, &plan.plan_id, &decision.request_id).unwrap();
        crate::ag::task::resolve_decision(&conn, &decision.request_id, "confirm").unwrap();
        let receipt =
            execute_operation_plan(&conn, &workspace, &plan.plan_id, &decision.request_id).unwrap();
        let entry = load_operation_entry(&conn, &receipt.operation_id)
            .unwrap()
            .unwrap();
        let reverse = restore_intents_from_entry(&conn, &entry).unwrap();
        assert_eq!(reverse.len(), 1);
        assert_eq!(reverse[0].kind, WorkspaceOperationKind::RestoreResource);
        assert_eq!(reverse[0].target_path.as_deref(), Some("a.md"));
        assert!(reverse[0].restore_payload_ref.is_some());
        assert!(
            !root.join("a.md").exists(),
            "读取历史只生成新计划输入，不得直接恢复磁盘"
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
