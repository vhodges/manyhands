//! Stored operations for the status read tests: rows written straight into
//! the three operation stores, with chosen IDs and times, where no public
//! call can leave them.

use std::path::Path;

use manyhands::repository::{OperationId, RemoteOperationTarget, RemoteRefPlan};
use rusqlite::{Connection, params};

use super::items;

/// Three operation IDs in ascending order.
pub const OPERATION_A: &str = "01ARZ3NDEKTSV4RRFFQ69G5FA0";
pub const OPERATION_B: &str = "01ARZ3NDEKTSV4RRFFQ69G5FA1";
pub const OPERATION_C: &str = "01ARZ3NDEKTSV4RRFFQ69G5FA2";
/// An ID no store holds.
pub const OPERATION_ABSENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FA9";

/// 2023-11-14T22:13:20Z.
pub const STORED_AT: i64 = 1_700_000_000;
pub const STORED_AT_TEXT: &str = "2023-11-14T22:13:20Z";

pub fn operation_id(id: &str) -> OperationId {
    OperationId::parse(id).unwrap()
}

/// Names the publication remote and primary branch a remote operation is
/// reserved against, as configuring the remote does.
pub fn configure_remote(data_directory: &Path) {
    items::index(data_directory)
        .execute_batch("UPDATE remote_polling_state SET remote_name='origin',primary_branch='main'")
        .unwrap();
}

pub fn poll_target() -> RemoteOperationTarget {
    RemoteOperationTarget::for_poll(&RemoteRefPlan::from_configuration("origin", "main").unwrap())
}

/// A local operation record for the one registered repository, observed at
/// `STORED_AT`. `id` is `None` for a record from before operations had IDs.
pub fn insert_local(
    data_directory: &Path,
    id: Option<&str>,
    action: &str,
    state: &str,
    completed_step: Option<&str>,
) {
    insert_local_into(
        &items::index(data_directory),
        id,
        action,
        state,
        completed_step,
    );
}

pub fn insert_local_into(
    connection: &Connection,
    id: Option<&str>,
    action: &str,
    state: &str,
    completed_step: Option<&str>,
) {
    connection
        .execute(
            "INSERT INTO operation_records (
                repository_id, root_path, operation_ulid, action, state, completed_step,
                observed_at
             ) SELECT id, root_path, ?1, ?2, ?3, ?4, ?5 FROM repositories",
            params![id, action, state, completed_step, STORED_AT],
        )
        .unwrap();
}

/// A poll of the one registered repository, created at `STORED_AT` and
/// updated 100 seconds later. `configure_remote` must have run.
pub fn insert_remote_poll(
    data_directory: &Path,
    id: &str,
    phase: &str,
    completed_step: Option<&str>,
    outcome: Option<&str>,
) {
    items::index(data_directory)
        .execute(
            "INSERT INTO remote_operation_records (
                repository_id, operation_ulid, configuration_generation, remote_name,
                primary_branch, primary_ref, primary_tracking_ref, action, priority, phase,
                completed_step, created_at, updated_at, outcome
             ) SELECT id, ?1, 0, 'origin', 'main', 'refs/heads/main',
                      'refs/remotes/origin/main', 'poll', 'poll', ?2, ?3, ?4, ?5, ?6
                 FROM repositories",
            params![
                id,
                phase,
                completed_step,
                STORED_AT,
                STORED_AT + 100,
                outcome
            ],
        )
        .unwrap();
}

/// A complete observation of the remote's branches at `STORED_AT`.
/// `configure_remote` must have run.
pub fn insert_current_observation(data_directory: &Path) {
    items::index(data_directory)
        .execute(
            "INSERT INTO remote_observation_batches (
                repository_id, remote_name, primary_branch, configuration_generation,
                observed_at, is_current
             ) SELECT id, 'origin', 'main', 0, ?1, 1 FROM repositories",
            [STORED_AT],
        )
        .unwrap();
}

/// A manual synchronization of the one registered repository, created at
/// `STORED_AT` and updated 100 seconds later: of the primary branch, or of
/// `item`'s branch when one is given as its kind and ID. It has made no
/// change yet, so it carries no checkpoint. `configure_remote` must have
/// run.
pub fn insert_remote_synchronization(
    data_directory: &Path,
    id: &str,
    item: Option<(&str, &str)>,
    phase: &str,
    completed_step: Option<&str>,
    outcome: Option<&str>,
) {
    let branch = item.map(|(kind, item)| format!("manyhands/{kind}/{item}"));
    items::index(data_directory)
        .execute(
            "INSERT INTO remote_operation_records (
                repository_id, operation_ulid, configuration_generation, remote_name,
                primary_branch, primary_ref, primary_tracking_ref, context_ref,
                context_tracking_ref, kind, item_id, local_branch, action, priority, phase,
                completed_step, created_at, updated_at, outcome
             ) SELECT id, ?1, 0, 'origin', 'main', 'refs/heads/main',
                      'refs/remotes/origin/main', ?2, ?3, ?4, ?5, ?6, ?7, 'manual', ?8, ?9,
                      ?10, ?11, ?12
                 FROM repositories",
            params![
                id,
                branch.as_ref().map(|branch| format!("refs/heads/{branch}")),
                branch
                    .as_ref()
                    .map(|branch| format!("refs/remotes/origin/{branch}")),
                item.map(|(kind, _)| kind),
                item.map(|(_, item)| item),
                branch.as_deref().unwrap_or("main"),
                if item.is_some() {
                    "synchronize_context"
                } else {
                    "synchronize_primary"
                },
                phase,
                completed_step,
                STORED_AT,
                STORED_AT + 100,
                outcome
            ],
        )
        .unwrap();
}

/// The commit a completed synchronization published in these fixtures.
pub const PUBLISHED_OID: &str = "0123456789abcdef0123456789abcdef01234567";

/// A synchronization of the primary branch that published `PUBLISHED_OID`
/// and has not yet handed it to the index. `configure_remote` must have
/// run.
pub fn insert_remote_index_pending(data_directory: &Path, id: &str) {
    items::index(data_directory)
        .execute(
            "INSERT INTO remote_operation_records (
                repository_id, operation_ulid, configuration_generation, remote_name,
                primary_branch, primary_ref, primary_tracking_ref, local_branch, action,
                priority, phase, completed_step, created_at, updated_at, outcome,
                sync_checkpoint, expected_oid, local_oid, primary_tracking_oid, push_oid,
                push_advertised_oid, authoritative_kind, authoritative_oid, index_pending
             ) SELECT id, ?1, 0, 'origin', 'main', 'refs/heads/main',
                      'refs/remotes/origin/main', 'main', 'synchronize_primary', 'manual',
                      'completed', 'before_discovery', ?2, ?3, 'completed',
                      'discovery_pending', ?4, ?4, ?4, ?4, ?4, 'published', ?4, 1
                 FROM repositories",
            params![id, STORED_AT, STORED_AT + 100, PUBLISHED_OID],
        )
        .unwrap();
}

/// What a key-material operation's paths and label hold in these fixtures;
/// no read may publish it.
pub const KEY_MATERIAL_SENTINEL: &str = "SENTINEL-4e7a";

/// A key-material operation for a key of its own, whose ID is returned.
/// `action` is `generate` or `delete`.
pub fn insert_key_material(
    data_directory: &Path,
    id: &str,
    action: &str,
    phase: &str,
    failure_code: Option<&str>,
) -> String {
    let key_id = manyhands::repository::SharedKeyId::new().to_string();
    let label = (action == "generate").then(|| format!("label {KEY_MATERIAL_SENTINEL}"));
    items::index(data_directory)
        .execute(
            "INSERT INTO key_material_operations (
                operation_id, key_id, action, generation_label, private_key_path,
                public_key_path, phase, failure_code
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id,
                key_id,
                action,
                label,
                format!("/keys/{KEY_MATERIAL_SENTINEL}/{key_id}"),
                format!("/keys/{KEY_MATERIAL_SENTINEL}/{key_id}.pub"),
                phase,
                failure_code
            ],
        )
        .unwrap();
    key_id
}
