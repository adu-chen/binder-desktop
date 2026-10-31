//! ExplicitEditTargetSet 的唯一运行时 owner：内存集合、TaskRun 持久化与成员守卫。

use super::reference_runtime::{reference_target_path, BlockRef};
use crate::ipc::IpcError;
use crate::types::ReferencePayload;
use crate::workspace;
use rusqlite::Connection;
use std::collections::BTreeMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub const SOURCE_CURRENT_DOCUMENT: &str = "current_document";
pub const SOURCE_EXPLICIT_REFERENCE: &str = "explicit_reference";
pub const SOURCE_SCOPE_DECISION: &str = "scope_decision";
pub const SOURCE_CREATE_DOCUMENT_BUNDLE: &str = "create_document_bundle";

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetAuthorization {
    pub rel_path: String,
    pub source_kind: String,
    pub source_ref: Option<String>,
}

/**
 * @GOV
 * codes:    BR-AG-DATA-003-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-007-DATA-AG-AG-TARGETSET-001, BR-SYS-STATE-008-DATA-AG-AG-TARGETSET-001
 * type:     DATA
 * chain:    AG-TURN, AG-TOOL
 * rules:    BR-AG-DATA-003, BR-SYS-STATE-007, BR-SYS-STATE-008
 * term_ref: TERM-AG-019, TERM-AG-001, TERM-AG-015, TERM-REF-003
 * boundary: in=CurrentDocumentExplicitReferenceOrConfirmedTaskScopeTarget | out=MonotonicExplicitEditTargetSet | does_not=grantFromReadSearchModelTextHistoryOrOpenTabs | consumer=BR-SYS-STATE-007,BR-SYS-STATE-008
 */
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExplicitEditTargetSet {
    entries: BTreeMap<String, TargetAuthorization>,
}

impl ExplicitEditTargetSet {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn contains(&self, rel_path: &str) -> bool {
        self.entries.contains_key(rel_path)
    }

    pub fn paths(&self) -> Vec<String> {
        self.entries.keys().cloned().collect()
    }

    pub fn authorizations(&self) -> impl Iterator<Item = &TargetAuthorization> {
        self.entries.values()
    }

    pub fn is_superset_of(&self, other: &Self) -> bool {
        other.entries.keys().all(|path| self.contains(path))
    }

    pub fn merge(&mut self, other: Self) {
        for (path, authorization) in other.entries {
            self.entries.entry(path).or_insert(authorization);
        }
    }

    pub fn insert_authorized(
        &mut self,
        rel_path: String,
        source_kind: impl Into<String>,
        source_ref: Option<String>,
    ) {
        self.entries
            .entry(rel_path.clone())
            .or_insert(TargetAuthorization {
                rel_path,
                source_kind: source_kind.into(),
                source_ref,
            });
    }
}

fn normalized_existing_rel(root: &Path, raw: &str) -> Result<Option<String>, IpcError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(None);
    }
    let absolute = workspace::resolve_in_workspace(root, raw)?;
    if !absolute.is_file() {
        return Ok(None);
    }
    let relative = absolute.strip_prefix(root).map_err(|_| {
        IpcError::new(
            "AG_EDIT_TARGET_INVALID",
            "编辑目标无法归一到当前 workspace",
            false,
            false,
        )
    })?;
    let normalized = relative.to_string_lossy().replace('\\', "/");
    if normalized.is_empty() {
        return Err(IpcError::new(
            "AG_EDIT_TARGET_INVALID",
            "workspace 根目录不能作为内容编辑目标",
            false,
            false,
        ));
    }
    Ok(Some(normalized))
}

pub fn normalize_existing_target(root: &Path, raw: &str) -> Result<String, IpcError> {
    normalized_existing_rel(root, raw)?.ok_or_else(|| {
        IpcError::new(
            "AG_EDIT_TARGET_NOT_FOUND",
            &format!("编辑目标不存在或不是既有文档：{}", raw.trim()),
            true,
            false,
        )
    })
}

pub(crate) fn initial_for_turn(
    root: &Path,
    turn_id: &str,
    active_doc_path: Option<&str>,
    mentions: &[String],
    block_refs: &[BlockRef],
    references: &[ReferencePayload],
) -> Result<ExplicitEditTargetSet, IpcError> {
    let mut targets =
        explicit_references_for_turn(root, turn_id, mentions, block_refs, references)?;
    if let Some(path) = active_doc_path {
        if let Some(path) = normalized_existing_rel(root, path)? {
            targets.insert_authorized(path, SOURCE_CURRENT_DOCUMENT, Some(turn_id.to_string()));
        }
    }
    Ok(targets)
}

pub(crate) fn explicit_references_for_turn(
    root: &Path,
    turn_id: &str,
    mentions: &[String],
    block_refs: &[BlockRef],
    references: &[ReferencePayload],
) -> Result<ExplicitEditTargetSet, IpcError> {
    let mut targets = ExplicitEditTargetSet::default();
    for raw in mentions
        .iter()
        .map(String::as_str)
        .chain(
            block_refs
                .iter()
                .map(|reference| reference.rel_path.as_str()),
        )
        .chain(references.iter().filter_map(reference_target_path))
    {
        if let Some(path) = normalized_existing_rel(root, raw)? {
            targets.insert_authorized(path, SOURCE_EXPLICIT_REFERENCE, Some(turn_id.to_string()));
        }
    }
    Ok(targets)
}

pub fn validated_scope_candidates(
    root: &Path,
    raw_paths: &[String],
    current: &ExplicitEditTargetSet,
) -> Result<Vec<String>, IpcError> {
    let mut candidates = BTreeMap::<String, ()>::new();
    for raw in raw_paths {
        let Some(path) = normalized_existing_rel(root, raw)? else {
            return Err(IpcError::new(
                "AG_EDIT_TARGET_NOT_FOUND",
                &format!("范围追加目标不存在或不是既有文档：{}", raw.trim()),
                true,
                false,
            ));
        };
        if !current.contains(&path) {
            candidates.insert(path, ());
        }
    }
    if candidates.is_empty() {
        return Err(IpcError::new(
            "AG_EDIT_TARGET_SCOPE_EMPTY",
            "范围裁决必须至少包含一个尚未授权的既有文档",
            false,
            false,
        ));
    }
    Ok(candidates.into_keys().collect())
}

/**
 * @GOV
 * codes:    BR-AG-DATA-003-EFFECT-AG-AG-TARGETSET-002, BR-SYS-PERSIST-001-EFFECT-AG-AG-TARGETSET-002
 * type:     EFFECT
 * chain:    AG-TURN, AG-TOOL, DATA-MIGRATE
 * rules:    BR-AG-DATA-003, BR-SYS-PERSIST-001
 * term_ref: TERM-AG-019, TERM-AG-015
 * boundary: in=TaskRunAndAuthorizedExistingDocumentPaths | out=PersistedMonotonicTaskRunEditTargets(withSourceProvenance) | effect=workspaceDb | does_not=removeTargetsOrGrantExecutionOrRewriteTaskRunIdentity | consumer=BR-AG-DATA-003
 */
pub fn persist_for_task_run(
    conn: &Connection,
    task_run_id: &str,
    targets: &ExplicitEditTargetSet,
) -> rusqlite::Result<usize> {
    let authorized_at = now_millis();
    let mut inserted = 0;
    for target in targets.authorizations() {
        inserted += conn.execute(
            "INSERT OR IGNORE INTO task_run_edit_targets \
             (task_run_id, rel_path, source_kind, source_ref, authorized_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                task_run_id,
                target.rel_path,
                target.source_kind,
                target.source_ref,
                authorized_at
            ],
        )?;
    }
    Ok(inserted)
}

pub fn append_task_run_targets(
    conn: &Connection,
    task_run_id: &str,
    rel_paths: &[String],
    source_kind: &str,
    source_ref: Option<&str>,
) -> rusqlite::Result<usize> {
    let mut targets = ExplicitEditTargetSet::default();
    for path in rel_paths {
        targets.insert_authorized(path.clone(), source_kind, source_ref.map(str::to_string));
    }
    persist_for_task_run(conn, task_run_id, &targets)
}

pub fn load_for_task_run(
    conn: &Connection,
    task_run_id: &str,
) -> rusqlite::Result<ExplicitEditTargetSet> {
    let mut statement = conn.prepare(
        "SELECT rel_path, source_kind, source_ref FROM task_run_edit_targets \
         WHERE task_run_id=?1 ORDER BY rel_path",
    )?;
    let rows = statement.query_map([task_run_id], |row| {
        Ok(TargetAuthorization {
            rel_path: row.get(0)?,
            source_kind: row.get(1)?,
            source_ref: row.get(2)?,
        })
    })?;
    let mut targets = ExplicitEditTargetSet::default();
    for row in rows {
        let target = row?;
        targets.insert_authorized(target.rel_path, target.source_kind, target.source_ref);
    }
    Ok(targets)
}

/**
 * @GOV
 * codes:    BR-AG-DATA-003-GUARD-AG-AG-TARGETSET-003
 * type:     GUARD
 * chain:    AG-TOOL, DE-DIFF
 * rules:    BR-AG-DATA-003
 * term_ref: TERM-AG-019, TERM-DE-002
 * boundary: in=ExistingChangeProposalTargetAndExplicitEditTargetSet | out=AuthorizedTargetOrStableRejection(noPendingDiff) | consumer=BR-AG-DATA-003
 */
pub fn ensure_authorized(targets: &ExplicitEditTargetSet, rel_path: &str) -> Result<(), IpcError> {
    if targets.contains(rel_path) {
        return Ok(());
    }
    Err(IpcError::new(
        "AG_EDIT_TARGET_NOT_AUTHORIZED",
        &format!(
            "既有文档 {rel_path} 不在本轮 ExplicitEditTargetSet；请显式引用该文档，或先确认范围追加请求。"
        ),
        false,
        false,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use rusqlite::Connection;
    use std::fs;

    fn root() -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("binder-target-set-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("a.md"), "a").unwrap();
        fs::write(root.join("b.md"), "b").unwrap();
        root
    }

    // covers: BR-AG-DATA-003
    #[test]
    fn initial_targets_only_include_current_and_explicit_existing_documents() {
        let root = root();
        let references = vec![
            ReferencePayload::Doc {
                rel_path: "b.md".to_string(),
                label: Some("B".to_string()),
            },
            ReferencePayload::LongText {
                text: "other.md".to_string(),
                char_count: 8,
                label: None,
            },
            ReferencePayload::File {
                name: "b.md".to_string(),
                path: Some(root.join("b.md").to_string_lossy().into_owned()),
                size: Some(1),
                label: None,
                read_content: Some("b".to_string()),
            },
        ];
        let targets = initial_for_turn(
            &root,
            "turn-1",
            Some("./a.md"),
            &["missing.md".to_string()],
            &[],
            &references,
        )
        .unwrap();
        assert_eq!(targets.paths(), vec!["a.md", "b.md"]);
        assert!(ensure_authorized(&targets, "other.md").is_err());
        fs::remove_dir_all(root).unwrap();
    }

    // covers: BR-AG-DATA-003
    #[test]
    fn scope_candidates_are_existing_normalized_new_members_only() {
        let root = root();
        let current = initial_for_turn(&root, "turn-1", Some("a.md"), &[], &[], &[]).unwrap();
        assert_eq!(
            validated_scope_candidates(
                &root,
                &["./b.md".to_string(), "b.md".to_string()],
                &current,
            )
            .unwrap(),
            vec!["b.md"]
        );
        assert!(validated_scope_candidates(&root, &["missing.md".to_string()], &current).is_err());
        assert!(validated_scope_candidates(&root, &[".".to_string()], &current).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    // covers: BR-AG-DATA-003
    // covers: BR-SYS-PERSIST-001
    #[test]
    fn task_run_targets_persist_monotonically_with_provenance() {
        let mut conn = Connection::open_in_memory().unwrap();
        db::migrate(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO task_runs \
             (task_run_id, workspace_id, session_id, status, intent_summary, steps_json, created_at, updated_at) \
             VALUES ('task-1','ws-1','session-1','executing',NULL,'[]',1,1)",
            [],
        )
        .unwrap();
        let mut targets = ExplicitEditTargetSet::default();
        targets.insert_authorized(
            "a.md".to_string(),
            SOURCE_CURRENT_DOCUMENT,
            Some("turn-1".to_string()),
        );
        assert_eq!(persist_for_task_run(&conn, "task-1", &targets).unwrap(), 1);
        assert_eq!(persist_for_task_run(&conn, "task-1", &targets).unwrap(), 0);
        append_task_run_targets(
            &conn,
            "task-1",
            &["b.md".to_string()],
            SOURCE_SCOPE_DECISION,
            Some("decision-1"),
        )
        .unwrap();
        let loaded = load_for_task_run(&conn, "task-1").unwrap();
        assert_eq!(loaded.paths(), vec!["a.md", "b.md"]);
        let source: String = conn
            .query_row(
                "SELECT source_kind FROM task_run_edit_targets WHERE task_run_id='task-1' AND rel_path='b.md'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(source, SOURCE_SCOPE_DECISION);
    }
}
