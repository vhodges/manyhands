//! The golden envelopes of the mutation boundary under
//! `tests/fixtures/mutation_v1`, and the golden progress events under
//! `tests/fixtures/progress_v1`: the registered cases, and the check of one
//! envelope against its case.
//!
//! Nothing here names a type of the library but through `golden`, so the
//! target that compiles the library as a module of its own can hold it too.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;

use super::{golden, schema};

/// Every golden envelope of the mutation boundary, by fixture name. A
/// fixture with no entry, or an entry with no fixture, fails
/// `tests/mutation_contract.rs`.
pub const CASES: &[&str] = &[
    "context_prepare",
    "context_prepare_discovery_pending",
    "document_create",
    "document_create_invalid_path",
    "document_create_occupied",
    "document_move",
    "document_move_external_change",
    "document_repair",
    "document_repair_discovery_pending",
    "document_repair_external_change",
    "document_repair_identity_ambiguous",
    "document_repair_invalid_path",
    "document_repair_partial",
    "document_repair_path_not_found",
    "document_repair_preview",
    "document_repair_preview_noop",
    "document_repair_stale_context",
    "document_save",
    "document_save_invalid_input",
    "failure_request_not_found",
    "folder_create",
    "folder_create_discovery_pending",
    "folder_create_invalid_input",
    "folder_create_invalid_path",
    "folder_create_noop",
    "folder_create_occupied",
    "host_approve_external_change",
    "host_approve_host_mismatch",
    "host_approve_host_replacement_required",
    "host_approve_preview",
    "host_approve_preview_noop",
    "host_observe",
    "host_observe_repository_not_registered",
    "host_replace_noop",
    "host_replace_preview",
    "index_rebuild",
    "index_rebuild_bare_repository",
    "index_rebuild_not_enabled",
    "index_rebuild_not_repository",
    "index_rebuild_repository_inaccessible",
    "index_refresh",
    "index_refresh_failed",
    "index_refresh_index_unavailable",
    "index_refresh_mismatch",
    "index_refresh_repository_not_registered",
    "item_sync_publication_remote_required",
    "item_sync_remote_branch_deleted",
    "key_clear",
    "key_clear_noop",
    "key_delete",
    "key_delete_interrupted",
    "key_delete_key_not_deletable",
    "key_delete_preview",
    "key_generate",
    "key_generate_interrupted",
    "key_generate_invalid",
    "key_generate_retained",
    "key_import",
    "key_import_invalid",
    "key_import_noop",
    "key_remove",
    "key_remove_not_found",
    "key_remove_selected",
    "key_select",
    "key_select_noop",
    "operation_abandon",
    "operation_abandon_preview",
    "operation_abandon_preview_noop",
    "operation_abandon_preview_not_found",
    "operation_resume",
    "operation_resume_noop",
    "operation_resume_not_found",
    "operation_resume_original_request_required",
    "remote_add",
    "remote_add_conflict",
    "remote_add_discovery_pending",
    "remote_add_invalid",
    "remote_add_noop",
    "remote_add_not_enabled",
    "remote_remove",
    "remote_remove_preview",
    "remote_remove_preview_noop",
    "remote_remove_publication_remote_in_use",
    "remote_select",
    "remote_select_cleared",
    "remote_select_discovery_pending",
    "remote_select_invalid",
    "remote_select_noop",
    "repo_create",
    "repo_create_partial",
    "repo_create_preview",
    "repo_enable",
    "repo_enable_confirmation_expired",
    "repo_enable_confirmation_mismatch",
    "repo_enable_confirmation_not_found",
    "repo_enable_confirmation_required",
    "repo_enable_confirmation_used",
    "repo_enable_external_change",
    "repo_enable_invalid_configuration",
    "repo_enable_preview",
    "repo_enable_preview_noop",
    "repo_enable_registration_pending",
    "repo_enable_worktree_conflicted",
    "repo_enable_wrong_branch",
    "repo_identity_set",
    "repo_identity_set_external_change",
    "repo_identity_set_not_repository_root",
    "repo_identity_set_preview",
    "repo_identity_set_preview_noop",
    "repo_remove",
    "repo_remove_missing",
    "repo_remove_missing_preview",
    "repo_remove_preview",
    "repo_remove_preview_noop",
    "repo_sync_host_approval_required",
    "repo_sync_host_replacement_required",
    "repo_sync_key_rejected",
    "repo_sync_key_unavailable",
    "repo_sync_merge_required",
    "repo_sync_no_selected_key",
    "repo_sync_poll_yielding",
    "repo_sync_publication_remote_required",
    "repo_sync_push_rejected",
    "repo_sync_transport_unavailable",
    "repo_sync_unlock_failed",
    "repo_sync_unlock_required",
    "request_cancel",
    "request_cancel_not_found",
    "request_cancel_not_stoppable",
    "request_show_accepted",
    "request_show_finished",
    "ticket_create",
    "ticket_create_already_applied",
    "ticket_create_initials_required",
    "ticket_create_invalid_slug_configuration",
    "ticket_repair",
    "ticket_repair_not_repairable",
    "ticket_repair_preview",
    "ticket_save",
    "ticket_save_busy",
    "ticket_save_cancelled",
    "ticket_save_cycle",
    "ticket_save_discovery_pending",
    "ticket_save_external_change",
    "ticket_save_identity_required",
    "ticket_save_index_unavailable",
    "ticket_save_invalid_relationship",
    "ticket_save_item_not_found",
    "ticket_save_mismatch",
    "ticket_save_mismatch_partial",
    "ticket_save_noop",
    "ticket_save_not_confirmable",
    "ticket_save_not_repository_root",
    "ticket_save_partial",
    "ticket_save_reentered",
    "ticket_save_reentered_external_change",
    "ticket_save_reentered_superseded",
    "ticket_save_replay",
    "ticket_save_ticket_closed",
    "ticket_save_worktree_not_clean",
    "ticket_slug_assign",
    "ticket_slug_assign_already_assigned",
];

pub fn fixture_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mutation_v1")
}

/// Checks one envelope of the mutation boundary against the published
/// contract, as `golden::assert_mutation_contract` checks one under
/// `read_v1`: the redaction scan, the envelope schema, the registered
/// recovery actions, the data schema when the case names one, and the
/// golden fixture `tests/fixtures/mutation_v1/<name>.json`.
pub fn assert_contract(case: &golden::ContractCase<'_>, envelope: &impl Serialize) {
    let name = case.name;
    let value = serde_json::to_value(envelope).unwrap();
    assert!(
        CASES.contains(&name),
        "{name}: not listed in mutation::CASES"
    );
    if let Some((sentinel, location)) = golden::find_sentinel(&value, case.sentinels) {
        panic!("{name}: the envelope contains {sentinel:?} at {location}");
    }
    if let Err(problem) = schema::check_published(golden::ENVELOPE_SCHEMA, &value) {
        panic!(
            "{name}: the envelope does not match {}: {problem}",
            golden::ENVELOPE_SCHEMA
        );
    }
    golden::assert_registered_recovery(name, &value);
    let data = &value["data"];
    match case.data_schema {
        Some(data_schema) => {
            assert!(!data.is_null(), "{name}: a data schema needs data");
            if let Err(problem) = schema::check_published(data_schema, data) {
                panic!("{name}: data does not match {data_schema}: {problem}");
            }
        }
        None => assert!(data.is_null(), "{name}: data needs a data schema"),
    }

    let actual = golden::render(&value, case.placeholders);
    let path = fixture_directory().join(format!("{name}.json"));
    if golden::update_requested(
        std::env::var_os(golden::UPDATE_VARIABLE).as_deref(),
        std::env::var_os("CI").as_deref(),
    ) {
        fs::create_dir_all(fixture_directory()).unwrap();
        fs::write(&path, &actual).unwrap();
        return;
    }
    let expected = fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "{name}: cannot read {}: {error}; set {}=1 to write it",
            path.display(),
            golden::UPDATE_VARIABLE
        )
    });
    assert!(
        expected == actual.as_bytes(),
        "{name}: the envelope differs from {}\n--- expected\n{}\n--- actual\n{actual}",
        path.display(),
        String::from_utf8_lossy(&expected)
    );
}

/// Every golden list of progress events, by fixture name: what
/// `tests/fixtures/progress_v1` holds, exactly.
pub const PROGRESS_CASES: &[&str] = &["index_refresh", "ticket_create"];

pub fn progress_fixture_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/progress_v1")
}

/// The fixture of a registered list of progress events. A name that is not
/// in `PROGRESS_CASES` is refused.
pub fn progress_fixture(name: &str) -> PathBuf {
    assert!(
        PROGRESS_CASES.contains(&name),
        "{name}: not listed in mutation::PROGRESS_CASES"
    );
    progress_fixture_directory().join(format!("{name}.json"))
}

/// Checks an envelope against its fixture as `assert_contract` does, with
/// what differs from run to run replaced: the repository's root, the
/// commit and the operation ID of the envelope, and `replaced`.
/// `data_schema` is the schema of the data the envelope carries, and
/// `None` says it carries none.
pub fn assert_contract_at(
    name: &str,
    root: &str,
    data_schema: Option<&str>,
    replaced: &[(&str, &str)],
    envelope: &impl Serialize,
) {
    let value = serde_json::to_value(envelope).unwrap();
    let mut placeholders = vec![(root, "<root>")];
    placeholders.extend_from_slice(replaced);
    for (pointer, placeholder) in [
        ("/effects/commit_oid", "<commit>"),
        ("/operation_id", "<operation>"),
    ] {
        if let Some(found) = value.pointer(pointer).and_then(|found| found.as_str()) {
            placeholders.push((found, placeholder));
        }
    }
    assert_contract(
        &golden::ContractCase {
            name,
            data_schema,
            placeholders: &placeholders,
            sentinels: &[],
        },
        &value,
    );
}
