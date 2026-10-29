//! `conflict show`'s read, `RepositoryService::show_conflict`: the
//! design's "Required outcomes: conflict states" table, one
//! `c4_conflict_read_` case per row, and the cases of Review Focus 2,
//! R-5, R-11, R-12 and R-40.
//!
//! A raw peer clone makes the remote diverge, as a second user would. The
//! repaired and refused rows repair the merge with the Git steps of
//! `install_repair` and `refused_false_repairs`
//! (`tests/remote_merge_recovery.rs`). A resolution is stopped part way
//! by a library failure point, on a service of its own opened on the
//! same data directory.
//!
//! Every read is checked against the published schema, and what it
//! renders, but for the sides it was asked for, is scanned for the
//! world's private material. `request_id` is null in every row until
//! Task 4a records which request a resolution attempt belongs to.

use std::path::Path;

use serde_json::Value;

use super::*;

pub(crate) const CASES: &[ssh_harness::Case] = &[
    ("c4_conflict_read_canonical", c4_conflict_read_canonical),
    (
        "c4_conflict_read_noncanonical_or_structural",
        c4_conflict_read_noncanonical_or_structural,
    ),
    (
        "c4_conflict_read_incompatible_closures",
        c4_conflict_read_incompatible_closures,
    ),
    (
        "c4_conflict_read_markers_removed",
        c4_conflict_read_markers_removed,
    ),
    (
        "c4_conflict_read_resolution_prepared",
        c4_conflict_read_resolution_prepared,
    ),
    (
        "c4_conflict_read_commit_prepared",
        c4_conflict_read_commit_prepared,
    ),
    ("c4_conflict_read_resolved", c4_conflict_read_resolved),
    (
        "c4_conflict_read_resolved_then_saved",
        c4_conflict_read_resolved_then_saved,
    ),
    ("c4_conflict_read_repaired", c4_conflict_read_repaired),
    (
        "c4_conflict_read_repair_refused",
        c4_conflict_read_repair_refused,
    ),
    (
        "c4_conflict_read_repair_beside_a_lock",
        c4_conflict_read_repair_beside_a_lock,
    ),
    (
        "c4_conflict_read_invalid_configuration",
        c4_conflict_read_invalid_configuration,
    ),
    (
        "c4_conflict_read_merge_aborted",
        c4_conflict_read_merge_aborted,
    ),
    ("c4_conflict_read_cancelled", c4_conflict_read_cancelled),
    ("c4_conflict_read_complete", c4_conflict_read_complete),
    ("c4_conflict_read_busy", c4_conflict_read_busy),
    (
        "c4_conflict_read_unknown_operation",
        c4_conflict_read_unknown_operation,
    ),
    (
        "c4_conflict_read_not_a_synchronization",
        c4_conflict_read_not_a_synchronization,
    ),
    ("c4_conflict_read_local_sync", c4_conflict_read_local_sync),
    (
        "c4_conflict_read_comment_operation",
        c4_conflict_read_comment_operation,
    ),
    (
        "c4_conflict_read_writes_nothing",
        c4_conflict_read_writes_nothing,
    ),
    (
        "c4_conflict_read_hostile_path",
        c4_conflict_read_hostile_path,
    ),
    (
        "c4_conflict_read_display_path",
        c4_conflict_read_display_path,
    ),
];

const MAIN: &str = "refs/heads/main";
const SCHEMA: &str = "conflict.schema.json";
/// The request that brings primary up to the server, or publishes the
/// item's editing branch, before a case diverges.
const SETUP: &str = "01ARZ3NDEKTSV4RRFFQ69G5FY0";
/// The ticket a primary-branch case keeps on primary.
const TICKET: &str = merging::PRIMARY_TICKET;
/// A file that is no canonical content.
const PLAIN: &str = "conflict";
/// The comment a case adds to the world's item.
const COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC1";
const REGULAR: u32 = 0o100644;
const EXECUTABLE: u32 = 0o100755;
/// A path holding a terminal escape sequence and a newline (Review Focus
/// 2).
const HOSTILE_PATH: &[u8] = b"hostile-\x1b[31mred\nline.txt";
/// R-11's four kinds of byte: a control byte, a backslash, a non-ASCII
/// UTF-8 character and a byte that is no UTF-8.
const DISPLAY_PATH: &[u8] = b"display-\x01-\\-\xc3\xa9-\xff.txt";
const DISPLAYED: &str = "display-\\x01-\\\\-\\xc3\\xa9-\\xff.txt";

/// One file of a commit.
struct File<'a> {
    path: &'a [u8],
    bytes: &'a [u8],
    mode: u32,
}

fn file<'a>(path: &'a str, bytes: &'a [u8]) -> File<'a> {
    File {
        path: path.as_bytes(),
        bytes,
        mode: REGULAR,
    }
}

/// A regular file at a path given as its raw bytes.
fn raw_file(path: &'static [u8], bytes: &'static [u8]) -> File<'static> {
    File {
        path,
        bytes,
        mode: REGULAR,
    }
}

/// A world with a trusted server.
fn world() -> Result<World, FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    Ok(w)
}

/// A commit of `files` over `base`'s tree, whose parents are `parents`.
/// No reference moves.
fn commit_files(
    repo: &git2::Repository,
    base: git2::Oid,
    parents: &[git2::Oid],
    files: &[File<'_>],
) -> git2::Oid {
    let mut index = git2::Index::new().unwrap();
    index
        .read_tree(&repo.find_commit(base).unwrap().tree().unwrap())
        .unwrap();
    for file in files {
        index
            .add(&git2::IndexEntry {
                ctime: git2::IndexTime::new(0, 0),
                mtime: git2::IndexTime::new(0, 0),
                dev: 0,
                ino: 0,
                mode: file.mode,
                uid: 0,
                gid: 0,
                file_size: 0,
                id: repo.blob(file.bytes).unwrap(),
                flags: 0,
                flags_extended: 0,
                path: file.path.to_vec(),
            })
            .unwrap();
    }
    let tree = repo.find_tree(index.write_tree_to(repo).unwrap()).unwrap();
    let parents: Vec<git2::Commit<'_>> = parents
        .iter()
        .map(|parent| repo.find_commit(*parent).unwrap())
        .collect();
    let parents: Vec<&git2::Commit<'_>> = parents.iter().collect();
    let signature = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    repo.commit(
        None,
        &signature,
        &signature,
        "owned fixture child",
        &tree,
        &parents,
    )
    .unwrap()
}

/// Commits `files` on primary in the clone's worktree, which is left
/// clean.
fn local_commit(w: &World, files: &[File<'_>]) -> git2::Oid {
    let repo = w.repo().unwrap();
    let parent = repo.refname_to_id(MAIN).unwrap();
    let new = commit_files(&repo, parent, &[parent], files);
    repo.checkout_tree(
        &repo.find_object(new, None).unwrap(),
        Some(git2::build::CheckoutBuilder::new().safe()),
    )
    .unwrap();
    repo.reference(MAIN, new, true, "owned fixture advance")
        .unwrap();
    new
}

/// The peer commits `files` on primary, and pushes it to the server.
fn remote_commit(w: &World, files: &[File<'_>]) -> git2::Oid {
    let parent = w.peer.refname_to_id(MAIN).unwrap();
    let new = commit_files(&w.peer, parent, &[parent], files);
    w.peer.reference(MAIN, new, true, "fixture").unwrap();
    push_peer(&w.peer, &w.server, MAIN).unwrap();
    new
}

/// Gives primary `base` on both clones, by the peer and a `repo sync`,
/// unless it is empty; then has the clone change it with `local` and
/// the peer with `incoming`. Returns the two tips.
fn diverge(
    w: &World,
    base: &[File<'_>],
    local: &[File<'_>],
    incoming: &[File<'_>],
) -> (git2::Oid, git2::Oid) {
    if !base.is_empty() {
        remote_commit(w, base);
        assert_current(&execute(w, SETUP, repo_sync(w)));
    }
    (local_commit(w, local), remote_commit(w, incoming))
}

/// `repo sync` under `REQUEST_1`, which stops on the conflict: its
/// operation, and the next step it gave.
fn stop(w: &World) -> (OperationId, ConflictNextAction) {
    let stopped = execute(w, REQUEST_1, repo_sync(w));
    assert_eq!(stopped.code, ResultCode::RecoveryRequired);
    let Some(MutationDataDto::SynchronizationConflict(data)) = &stopped.data else {
        panic!("a synchronization conflict's data");
    };
    (
        OperationId::parse(&data.conflict.operation_id).unwrap(),
        data.conflict.next_action,
    )
}

/// A conflict on primary in `TICKET`'s canonical source, with all three
/// sides: its operation, the next step its synchronization gave, and the
/// local and incoming parents.
fn canonical_conflict(w: &World) -> (OperationId, ConflictNextAction, git2::Oid, git2::Oid) {
    let path = merging::ticket_path(TICKET);
    let (local, incoming) = diverge(
        w,
        &[file(&path, source("Base").as_bytes())],
        &[file(&path, source("Local").as_bytes())],
        &[file(&path, source("Incoming").as_bytes())],
    );
    let (operation, next) = stop(w);
    (operation, next, local, incoming)
}

/// A conflict on primary in `PLAIN`, which each side adds with other
/// bytes. Returns the operation and the two parents.
fn plain_conflict(w: &World) -> (OperationId, git2::Oid, git2::Oid) {
    let (local, incoming) = diverge(
        w,
        &[],
        &[file(PLAIN, b"local conflict\n")],
        &[file(PLAIN, b"incoming conflict\n")],
    );
    let (operation, next) = stop(w);
    assert_eq!(next, ConflictNextAction::ExternalRepair);
    (operation, local, incoming)
}

/// `TICKET`'s source, titled `title`.
fn source(title: &str) -> String {
    merging::ticket_source(TICKET, title)
}

/// `TICKET`'s source, titled `Base` and closed at `at`.
fn closed_source(at: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {TICKET}\ntitle: Base\n\
         type: task\nstatus: done\nclosed_at: {at}\nclosed_by: Fixture <fixture@example.invalid>\n\
         ---\nBody of Base.\n"
    )
}

/// `show_conflict` of `operation`, checked as every read is: the data
/// against its schema and in its envelope, and what it renders, but the
/// sides it was asked for, and what a failure renders, scanned for the
/// world's private material.
fn show(w: &World, operation: OperationId) -> Result<ConflictDto, ReadError> {
    let repo = w.service.resolve_repository(&w.root).unwrap();
    let result = w.service.show_conflict(&repo, operation);
    match &result {
        Ok(dto) => {
            let mut value = serde_json::to_value(dto).unwrap();
            assert!(schema::check_published(SCHEMA, &value).is_ok());
            let envelope = Envelope::read_success("conflict show", repo.scope(), dto.clone());
            let envelope = serde_json::to_value(envelope).unwrap();
            assert!(schema::check_published("envelope.schema.json", &envelope).is_ok());
            for path in value["paths"].as_array_mut().unwrap() {
                for side in ["base", "local", "incoming"] {
                    path[side] = Value::Null;
                }
            }
            assert!(ssh_privacy::clean(value.to_string().as_bytes(), &w.probes).is_ok());
        }
        Err(error) => {
            let rendered = format!("{error:?} {error}");
            assert!(ssh_privacy::clean(rendered.as_bytes(), &w.probes).is_ok());
        }
    }
    result
}

/// The inspection the library issues for `operation`'s live conflict.
fn inspect(w: &World, operation: OperationId) -> SynchronizationConflictInspection {
    w.service
        .inspect_synchronization_recovery(&w.root, operation)
        .unwrap()
}

/// A resolution of `operation`'s live conflict with `sources`, one per
/// path in the inspection's order, on `service`.
fn resolve_with(
    w: &World,
    service: &RepositoryService,
    operation: OperationId,
    sources: &[&[u8]],
) -> Result<ResolveSynchronizationOutcome, SynchronizationError> {
    let inspection = inspect(w, operation);
    assert_eq!(inspection.paths.len(), sources.len());
    service.resolve_synchronization(ResolveSynchronizationRequest::new(
        w.root.clone(),
        operation,
        OperationId::new(),
        inspection.observation.clone(),
        inspection
            .paths
            .iter()
            .zip(sources)
            .map(|(path, source)| {
                (
                    path.token.clone(),
                    RedactedConflictBytes::from_bytes(source.to_vec()),
                )
            })
            .collect(),
        None,
    ))
}

/// The canonical conflict resolved and checkpointed, as `conflict
/// resolve`'s library call leaves it: its operation, and the resolution
/// commit.
fn resolved_conflict(w: &World) -> (OperationId, git2::Oid) {
    let (operation, ..) = canonical_conflict(w);
    let resolved = source("Resolved");
    let Ok(ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid }) =
        resolve_with(w, &w.service, operation, &[resolved.as_bytes()])
    else {
        panic!("a resolution checkpoint");
    };
    (operation, commit_oid)
}

/// The parents of the resolution commit `commit`: the conflict's local
/// and incoming parents.
fn parents_of(w: &World, commit: git2::Oid) -> Vec<git2::Oid> {
    let repo = w.repo().unwrap();
    let merge = repo.find_commit(commit).unwrap();
    merge.parent_ids().collect()
}

/// A restart of `operation`, as `operation resume` makes one.
fn restart(w: &World, operation: OperationId) -> SynchronizeRemoteRequest {
    SynchronizeRemoteRequest {
        operation_id: operation,
        restart: true,
        ..w.primary()
    }
}

/// The phase of the integration step whose conflict `operation` met.
fn step_phase(w: &World, operation: OperationId) -> String {
    w.db()
        .unwrap()
        .query_row(
            "SELECT step.phase FROM remote_integration_steps step
               JOIN remote_operation_records operation ON operation.id = step.operation_record_id
              WHERE operation.operation_ulid = ?1 AND step.conflict_digest IS NOT NULL
              ORDER BY step.window_number DESC, step.ordinal DESC LIMIT 1",
            [operation.to_string()],
            |row| row.get(0),
        )
        .unwrap()
}

/// What every read of a synchronization of primary with no live conflict
/// holds: the state and next step it is given, and no observation, no
/// eligibility and no path.
fn settled(
    operation: OperationId,
    local: git2::Oid,
    incoming: git2::Oid,
    state: ConflictState,
    next_action: ConflictReadNextAction,
) -> ConflictDto {
    ConflictDto {
        operation_id: operation.to_string(),
        comment_operation_id: None,
        target: ConflictTarget::Primary,
        item_id: None,
        stage: ConflictStage::Primary,
        state,
        next_action,
        request_id: None,
        eligibility: None,
        local_parent: local.to_string(),
        incoming_parent: incoming.to_string(),
        observation: None,
        historical: false,
        paths: Vec::new(),
    }
}

/// Checks `dto` against its read golden under `tests/fixtures/read_v1`,
/// in the envelope `conflict show` gives it.
fn assert_read_golden(w: &World, name: &str, dto: &ConflictDto) {
    let repo = w.service.resolve_repository(&w.root).unwrap();
    let root = repo.scope().repository.unwrap();
    let mut placeholders = vec![
        (root.as_str(), "<repository>"),
        (dto.operation_id.as_str(), "<operation>"),
        (dto.local_parent.as_str(), "<local-parent>"),
        (dto.incoming_parent.as_str(), "<incoming-parent>"),
    ];
    if let Some(observation) = &dto.observation {
        placeholders.push((observation.as_str(), "<observation>"));
    }
    for path in &dto.paths {
        placeholders.push((path.token.as_str(), "<token>"));
    }
    golden::assert_contract(
        &golden::ContractCase {
            name,
            data_schema: Some(SCHEMA),
            placeholders: &placeholders,
            sentinels: &[],
        },
        &Envelope::read_success("conflict show", repo.scope(), dto.clone()),
    );
}

/// The operator's own tooling installs a whole committed merge: the
/// commit, the branch, and a worktree and index that match it
/// (`install_repair`). `cleanup` removes the merge metadata, as `git
/// commit` does.
fn install_repair(repo: &git2::Repository, repaired: git2::Oid, cleanup: bool) {
    repo.checkout_tree(
        &repo.find_object(repaired, None).unwrap(),
        Some(
            git2::build::CheckoutBuilder::new()
                .force()
                .remove_untracked(true),
        ),
    )
    .unwrap();
    repo.reference(MAIN, repaired, true, "external repair")
        .unwrap();
    let mut index = repo.index().unwrap();
    index
        .read_tree(&repo.find_commit(repaired).unwrap().tree().unwrap())
        .unwrap();
    index.write().unwrap();
    if cleanup {
        repo.cleanup_state().unwrap();
    }
}

/// Whether `text` holds a byte that is a control character.
fn raw_control(text: &str) -> bool {
    text.bytes().any(|byte| byte < 0x20 || byte == 0x7f)
}

/// An observer that cancels the synchronization at the first safe point
/// it is told of.
struct CancelAtFirst;

impl SynchronizationObserver for CancelAtFirst {
    fn safe_point(&mut self, _: RemoteOperationSafePoint) -> SafePointRequest {
        SafePointRequest::Cancel
    }
}

fn c4_conflict_read_canonical() -> Result<(), FixtureError> {
    let w = world()?;
    let (operation, next, local, incoming) = canonical_conflict(&w);
    let dto = show(&w, operation).unwrap();
    assert_eq!(dto.state, ConflictState::ConflictPending);
    assert_eq!(dto.next_action, ConflictReadNextAction::Resolve);
    // The read and the synchronization agree on the next step.
    assert_eq!(dto.next_action.as_str(), next.as_str());
    assert_eq!(dto.operation_id, operation.to_string());
    assert_eq!(dto.comment_operation_id, None);
    assert_eq!(dto.target, ConflictTarget::Primary);
    assert_eq!(dto.item_id, None);
    assert_eq!(dto.stage, ConflictStage::Primary);
    assert_eq!(dto.request_id, None);
    assert_eq!(
        dto.eligibility,
        Some(ConflictReadEligibility::EligibleCanonical)
    );
    assert_eq!(dto.local_parent, local.to_string());
    assert_eq!(dto.incoming_parent, incoming.to_string());
    assert!(!dto.historical);
    // The observation and the path's token are the strings R-9 derives
    // from the library's inspection.
    let inspection = inspect(&w, operation);
    let observation = repository::observation_text(&inspection.observation);
    assert_eq!(dto.observation.as_deref(), Some(observation.as_str()));
    assert!(observation.starts_with("conflict-v1:"));
    assert_eq!(observation.len(), "conflict-v1:".len() + 64);
    let tokens = repository::path_tokens(&inspection);
    assert_eq!(tokens.len(), 1);
    assert!(tokens[0].0.starts_with("conflict-path-v1:"));
    assert_eq!(tokens[0].0.len(), "conflict-path-v1:".len() + 64);
    assert_eq!(
        dto.paths,
        [ConflictPathDto {
            display_path: merging::ticket_path(TICKET),
            token: tokens[0].0.clone(),
            kind: ConflictPathKind::Ticket,
            structure: ConflictPathStructure::Regular,
            content_available: true,
            base: Some(source("Base")),
            local: Some(source("Local")),
            incoming: Some(source("Incoming")),
        }]
    );
    // The token resolves to the library's own, whose sides are the
    // read's.
    let sides = w
        .service
        .read_synchronization_conflict(&tokens[0].1)
        .unwrap();
    assert_eq!(
        sides.incoming.map(|side| side.bytes().to_vec()),
        Some(source("Incoming").into_bytes())
    );
    // Two reads of unchanged state are the same.
    assert_eq!(show(&w, operation).unwrap(), dto);
    assert_read_golden(&w, "conflict_show_conflict_pending", &dto);
    Ok(())
}

/// One side of `c4_conflict_read_noncanonical_or_structural`'s merge: the
/// ticket's source, a binary file and an executable script.
fn mixed_files<'a>(
    ticket: &'a str,
    (source, binary, script): &'a (String, Vec<u8>, Vec<u8>),
) -> Vec<File<'a>> {
    vec![
        file(ticket, source.as_bytes()),
        File {
            path: b"blob.bin",
            bytes: binary,
            mode: REGULAR,
        },
        File {
            path: b"tool.sh",
            bytes: script,
            mode: EXECUTABLE,
        },
        // The same script as a regular file: text, and no item.
        File {
            path: b"plain.sh",
            bytes: script,
            mode: REGULAR,
        },
    ]
}

fn c4_conflict_read_noncanonical_or_structural() -> Result<(), FixtureError> {
    let w = world()?;
    let ticket = merging::ticket_path(TICKET);
    let sides = ["Base", "Local", "Incoming"].map(|title| {
        let name = title.to_ascii_lowercase();
        (
            source(title),
            format!("\0{name}\0").into_bytes(),
            format!("echo {name}\n").into_bytes(),
        )
    });
    let [base, local, incoming] = sides.each_ref().map(|side| mixed_files(&ticket, side));
    diverge(&w, &base, &local, &incoming);
    let (operation, next) = stop(&w);
    let dto = show(&w, operation).unwrap();
    assert_eq!(dto.state, ConflictState::ConflictPending);
    assert_eq!(dto.next_action, ConflictReadNextAction::ExternalRepair);
    assert_eq!(dto.next_action.as_str(), next.as_str());
    assert_eq!(
        dto.eligibility,
        Some(ConflictReadEligibility::ExternalResolutionRequired)
    );
    assert!(dto.observation.is_some());
    assert!(!dto.historical);
    // Every path with its kind, in raw byte order; sides only for the
    // item's canonical text (R-10).
    let shape: Vec<_> = dto
        .paths
        .iter()
        .map(|path| {
            (
                path.display_path.as_str(),
                path.kind,
                path.structure,
                path.content_available,
            )
        })
        .collect();
    assert_eq!(
        shape,
        [
            (
                ticket.as_str(),
                ConflictPathKind::Ticket,
                ConflictPathStructure::Regular,
                true
            ),
            (
                "blob.bin",
                ConflictPathKind::Noncanonical,
                ConflictPathStructure::Binary,
                false
            ),
            (
                "plain.sh",
                ConflictPathKind::Noncanonical,
                ConflictPathStructure::Binary,
                false
            ),
            (
                "tool.sh",
                ConflictPathKind::Noncanonical,
                ConflictPathStructure::Executable,
                false
            ),
        ]
    );
    assert_eq!(dto.paths[0].local, Some(source("Local")));
    for path in &dto.paths[1..] {
        assert!(path.base.is_none() && path.local.is_none() && path.incoming.is_none());
    }
    // The library would read the regular non-item file as text; the read
    // still gives it no sides.
    let tokens = repository::path_tokens(&inspect(&w, operation));
    let (_, plain) = tokens
        .iter()
        .find(|(text, _)| *text == dto.paths[2].token)
        .unwrap();
    let sides = w.service.read_synchronization_conflict(plain).unwrap();
    assert_eq!(
        sides.local.map(|side| side.bytes().to_vec()),
        Some(b"echo local\n".to_vec())
    );
    Ok(())
}

fn c4_conflict_read_incompatible_closures() -> Result<(), FixtureError> {
    let w = world()?;
    let path = merging::ticket_path(TICKET);
    let local = closed_source("2026-10-25T12:00:00Z");
    let incoming = closed_source("2026-10-26T12:00:00Z");
    diverge(
        &w,
        &[file(&path, source("Base").as_bytes())],
        &[file(&path, local.as_bytes())],
        &[file(&path, incoming.as_bytes())],
    );
    let (operation, next) = stop(&w);
    let dto = show(&w, operation).unwrap();
    assert_eq!(dto.state, ConflictState::ConflictPending);
    // The two closures cannot both stand: the set is repaired outside as
    // a whole (R-40).
    assert_eq!(dto.next_action, ConflictReadNextAction::ExternalRepair);
    assert_eq!(dto.next_action.as_str(), next.as_str());
    assert_eq!(
        dto.eligibility,
        Some(ConflictReadEligibility::ExternalResolutionRequired)
    );
    assert!(dto.observation.is_some());
    assert_eq!(dto.paths.len(), 1);
    assert_eq!(dto.paths[0].display_path, path);
    assert_eq!(dto.paths[0].kind, ConflictPathKind::Ticket);
    assert_eq!(
        dto.paths[0].structure,
        ConflictPathStructure::IncompatibleClosures
    );
    // An item's text, so its sides are given whatever its structure (R-10).
    assert!(dto.paths[0].content_available);
    assert_eq!(dto.paths[0].base, Some(source("Base")));
    assert_eq!(dto.paths[0].local.as_deref(), Some(local.as_str()));
    assert_eq!(dto.paths[0].incoming.as_deref(), Some(incoming.as_str()));
    // The library refuses to resolve the set, before any write (fact 33).
    let resolved = source("Resolved");
    assert!(matches!(
        resolve_with(&w, &w.service, operation, &[resolved.as_bytes()]),
        Err(SynchronizationError::ExternalResolutionRequired { .. })
    ));
    Ok(())
}

fn c4_conflict_read_markers_removed() -> Result<(), FixtureError> {
    let w = world()?;
    let (operation, ..) = canonical_conflict(&w);
    let before = show(&w, operation).unwrap();
    // The markers are removed in the worktree; the index's stages are
    // left as they were (fact 31).
    std::fs::write(
        w.root.join(merging::ticket_path(TICKET)),
        source("Resolved"),
    )
    .unwrap();
    let after = show(&w, operation).unwrap();
    assert_eq!(after.state, ConflictState::ConflictPending);
    assert_eq!(after.next_action, ConflictReadNextAction::Resolve);
    // The same observation, tokens and sides.
    assert_eq!(after, before);
    Ok(())
}

fn c4_conflict_read_resolution_prepared() -> Result<(), FixtureError> {
    let w = world()?;
    let (operation, ..) = canonical_conflict(&w);
    let before = show(&w, operation).unwrap();
    let failing = RepositoryService::open_at_with_failure_point_for_testing(
        &w.data(),
        FailurePoint::ResolutionAfterPathWrite,
    )
    .unwrap();
    let resolved = source("Resolved");
    let _ = resolve_with(&w, &failing, operation, &[resolved.as_bytes()]);
    assert_eq!(step_phase(&w, operation), "resolution_prepared");
    // The path holds what the attempt wrote.
    assert_eq!(
        std::fs::read_to_string(w.root.join(merging::ticket_path(TICKET))).unwrap(),
        resolved
    );
    let dto = show(&w, operation).unwrap();
    // Fact 86: the inspection still issues the observation, and the
    // recorded conflict's tokens and sides.
    assert!(dto.observation.is_some());
    assert_eq!(dto.observation, before.observation);
    assert_eq!(dto.paths, before.paths);
    assert_eq!(dto.state, ConflictState::ResolutionPrepared);
    assert_eq!(dto.next_action, ConflictReadNextAction::RetryResolution);
    assert!(dto.historical);
    assert_eq!(
        dto.eligibility,
        Some(ConflictReadEligibility::EligibleCanonical)
    );
    // Until Task 4a records the attempt's request.
    assert_eq!(dto.request_id, None);
    Ok(())
}

fn c4_conflict_read_commit_prepared() -> Result<(), FixtureError> {
    let w = world()?;
    let (operation, _, local, incoming) = canonical_conflict(&w);
    let failing = RepositoryService::open_at_with_failure_point_for_testing(
        &w.data(),
        FailurePoint::ResolutionAfterCandidatePrepared,
    )
    .unwrap();
    let resolved = source("Resolved");
    let _ = resolve_with(&w, &failing, operation, &[resolved.as_bytes()]);
    assert_eq!(step_phase(&w, operation), "commit_prepared");
    // The ref has not moved.
    assert_eq!(w.repo().unwrap().refname_to_id(MAIN).unwrap(), local);
    assert_eq!(
        show(&w, operation).unwrap(),
        settled(
            operation,
            local,
            incoming,
            ConflictState::ResolutionPrepared,
            ConflictReadNextAction::Resume
        )
    );
    Ok(())
}

fn c4_conflict_read_resolved() -> Result<(), FixtureError> {
    let w = world()?;
    let (operation, commit) = resolved_conflict(&w);
    assert_eq!(step_phase(&w, operation), "applied");
    let parents = parents_of(&w, commit);
    assert_eq!(
        show(&w, operation).unwrap(),
        settled(
            operation,
            parents[0],
            parents[1],
            ConflictState::Resolved,
            ConflictReadNextAction::Resume
        )
    );
    Ok(())
}

fn c4_conflict_read_resolved_then_saved() -> Result<(), FixtureError> {
    let mut w = world()?;
    let context = w.context()?;
    assert_eq!(execute(&w, SETUP, item_sync(&w)).code, ResultCode::Ok);
    // Each side gives the item's ticket another title.
    let path = merging::ticket_path(ITEM);
    let local = commit_in(
        &context.worktree,
        CONTEXT,
        &path,
        merging::ticket_source(ITEM, "Local").as_bytes(),
    )?;
    merging::fetch_into_peer(&w, CONTEXT)?;
    let parent = w.peer.refname_to_id(CONTEXT).unwrap();
    let incoming = commit_path(
        &w.peer,
        parent,
        &path,
        merging::ticket_source(ITEM, "Incoming").as_bytes(),
    )?;
    w.peer
        .reference(CONTEXT, incoming, true, "fixture")
        .unwrap();
    push_peer(&w.peer, &w.server, CONTEXT)?;
    let stopped = execute(&w, REQUEST_1, item_sync(&w));
    let Some(MutationDataDto::SynchronizationConflict(data)) = &stopped.data else {
        panic!("a synchronization conflict's data");
    };
    let operation = OperationId::parse(&data.conflict.operation_id).unwrap();
    let resolved = merging::ticket_source(ITEM, "Resolved");
    assert!(matches!(
        resolve_with(&w, &w.service, operation, &[resolved.as_bytes()]),
        Ok(ResolveSynchronizationOutcome::LocalCheckpointComplete { .. })
    ));
    let expected = ConflictDto {
        target: ConflictTarget::Ticket,
        item_id: Some(ITEM.to_owned()),
        stage: ConflictStage::Context,
        ..settled(
            operation,
            local,
            incoming,
            ConflictState::Resolved,
            ConflictReadNextAction::Resume,
        )
    };
    assert_eq!(show(&w, operation).unwrap(), expected);
    // A local save in the context, before the resolution is published.
    let tip = w.repo().unwrap().refname_to_id(CONTEXT).unwrap();
    let current = std::fs::read(context.worktree.join(&path)).unwrap();
    let saved = w
        .service
        .save_ticket(SaveTicketRequest {
            target: AuthoringTarget {
                root: w.root.clone(),
                kind: AuthoringKind::Ticket,
                item_id: ITEM.parse().unwrap(),
                intent: ContextIntent::Edit,
                operation_id: OperationId::new(),
            },
            draft: TicketDraft {
                title: "Saved".into(),
                body: "Saved after the resolution.\n".into(),
                ticket_type: "task".into(),
                status: "open".into(),
                project: None,
                team: None,
            },
            expected_path: ExpectedPathObservation::Blake3(*blake3::hash(&current).as_bytes()),
        })
        .unwrap();
    assert!(matches!(saved, SaveOutcome::Saved { .. }));
    let repo = w.repo().unwrap();
    let saved_tip = repo.refname_to_id(CONTEXT).unwrap();
    assert!(repo.graph_descendant_of(saved_tip, tip).unwrap());
    assert_eq!(show(&w, operation).unwrap(), expected);
    Ok(())
}

fn c4_conflict_read_repaired() -> Result<(), FixtureError> {
    let w = world()?;
    let (operation, local, incoming) = plain_conflict(&w);
    let repo = w.repo().unwrap();
    let repaired = commit_files(
        &repo,
        local,
        &[local, incoming],
        &[file(PLAIN, b"repaired conflict\n")],
    );
    // A whole committed merge, whose tool left the merge metadata.
    install_repair(&repo, repaired, false);
    assert!(repo.path().join("MERGE_HEAD").exists());
    let expected = settled(
        operation,
        local,
        incoming,
        ConflictState::Repaired,
        ConflictReadNextAction::Resume,
    );
    let dto = show(&w, operation).unwrap();
    assert_eq!(dto, expected);
    assert_read_golden(&w, "conflict_show_repaired", &dto);
    // As `git commit` leaves it.
    repo.cleanup_state().unwrap();
    assert_eq!(show(&w, operation).unwrap(), expected);
    // The restart takes the repair up, as the read said it would.
    let published = w.sync(restart(&w, operation)).unwrap();
    outcome(published, true, SynchronizationTarget::Primary, repaired);
    assert_eq!(
        show(&w, operation).unwrap(),
        settled(
            operation,
            local,
            incoming,
            ConflictState::Complete,
            ConflictReadNextAction::None
        )
    );
    Ok(())
}

fn c4_conflict_read_repair_refused() -> Result<(), FixtureError> {
    let w = world()?;
    let (operation, local, incoming) = plain_conflict(&w);
    let repo = w.repo().unwrap();
    let path = w.root.join(PLAIN);
    let index_file = repo.path().join("index");
    let marked = std::fs::read(&path).unwrap();
    let conflicted = std::fs::read(&index_file).unwrap();
    let pending = show(&w, operation).unwrap();
    assert_eq!(pending.state, ConflictState::ConflictPending);
    assert_eq!(pending.next_action, ConflictReadNextAction::ExternalRepair);
    let refused = settled(
        operation,
        local,
        incoming,
        ConflictState::RepairRefused,
        ConflictReadNextAction::ExternalRepair,
    );
    let restart_refused = || {
        matches!(
            w.sync(restart(&w, operation)),
            Err(SynchronizationError::RecoveryRequired)
        )
    };
    // Staged without a commit.
    std::fs::write(&path, b"repaired conflict\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(PLAIN)).unwrap();
    index.write().unwrap();
    assert_eq!(show(&w, operation).unwrap(), refused);
    assert!(restart_refused());
    // Undone, the conflict is pending again.
    std::fs::write(&index_file, &conflicted).unwrap();
    std::fs::write(&path, &marked).unwrap();
    assert_eq!(show(&w, operation).unwrap(), pending);
    // Committed with wrong parents: one parent, then reversed parents.
    for parents in [vec![local], vec![incoming, local]] {
        let wrong = commit_files(
            &repo,
            local,
            &parents,
            &[file(PLAIN, b"repaired conflict\n")],
        );
        install_repair(&repo, wrong, false);
        assert_eq!(show(&w, operation).unwrap(), refused);
        assert!(restart_refused());
        repo.reference(MAIN, local, true, "false repair undone")
            .unwrap();
        std::fs::write(&index_file, &conflicted).unwrap();
        std::fs::write(&path, &marked).unwrap();
    }
    assert_eq!(show(&w, operation).unwrap(), pending);
    Ok(())
}

/// A valid outside repair beside a stale Git lock: the restart refuses it
/// (`refuse_local_reconciliation_locks`), and so does the read.
fn c4_conflict_read_repair_beside_a_lock() -> Result<(), FixtureError> {
    let w = world()?;
    let (operation, local, incoming) = plain_conflict(&w);
    let repo = w.repo().unwrap();
    let repaired = commit_files(
        &repo,
        local,
        &[local, incoming],
        &[file(PLAIN, b"repaired conflict\n")],
    );
    install_repair(&repo, repaired, true);
    // A stale lock of the index, then of the branch. Every platform names
    // both files.
    for lock in [
        repo.path().join("index.lock"),
        repo.commondir().join("refs/heads/main.lock"),
    ] {
        std::fs::write(&lock, b"").unwrap();
        assert_eq!(
            show(&w, operation).unwrap_err().code(),
            ResultCode::RecoveryRequired
        );
        assert!(matches!(
            w.sync(restart(&w, operation)),
            Err(SynchronizationError::RecoveryRequired)
        ));
        std::fs::remove_file(&lock).unwrap();
    }
    // With the lock gone, the repair is read as one, and a restart takes
    // it up.
    assert_eq!(
        show(&w, operation).unwrap(),
        settled(
            operation,
            local,
            incoming,
            ConflictState::Repaired,
            ConflictReadNextAction::Resume
        )
    );
    let published = w.sync(restart(&w, operation)).unwrap();
    outcome(published, true, SynchronizationTarget::Primary, repaired);
    Ok(())
}

/// An intact conflict whose configuration cannot be read: the inspection
/// cannot be made, and the read says so as an error. It never judges the
/// conflict a refused repair.
fn c4_conflict_read_invalid_configuration() -> Result<(), FixtureError> {
    let w = world()?;
    let (operation, ..) = canonical_conflict(&w);
    let config = w.root.join(".manyhands/config.toml");
    let valid = std::fs::read(&config).unwrap();
    std::fs::write(&config, b"this is = = not a configuration\n").unwrap();
    assert!(w.repo().unwrap().index().unwrap().has_conflicts());
    assert_eq!(
        show(&w, operation).unwrap_err().code(),
        ResultCode::RecoveryRequired
    );
    // With the configuration restored, the same conflict is read.
    std::fs::write(&config, valid).unwrap();
    assert_eq!(
        show(&w, operation).unwrap().state,
        ConflictState::ConflictPending
    );
    Ok(())
}

fn c4_conflict_read_merge_aborted() -> Result<(), FixtureError> {
    let w = world()?;
    let (operation, local, incoming) = plain_conflict(&w);
    let repo = w.repo().unwrap();
    repo.reset(
        &repo.find_object(local, None).unwrap(),
        git2::ResetType::Hard,
        None,
    )
    .unwrap();
    repo.cleanup_state().unwrap();
    assert_eq!(repo.refname_to_id(MAIN).unwrap(), local);
    assert!(!repo.path().join("MERGE_HEAD").exists());
    assert!(!repo.index().unwrap().has_conflicts());
    assert_eq!(
        show(&w, operation).unwrap(),
        settled(
            operation,
            local,
            incoming,
            ConflictState::RepairRefused,
            ConflictReadNextAction::ExternalRepair
        )
    );
    Ok(())
}

fn c4_conflict_read_cancelled() -> Result<(), FixtureError> {
    let w = world()?;
    let (operation, commit) = resolved_conflict(&w);
    let parents = parents_of(&w, commit);
    // The continuation's publication is cancelled.
    let continued = w.service.synchronize_remote_observed(
        restart(&w, operation),
        &mut SessionCredentials::new(Provider),
        SynchronizationSettings::default(),
        &mut CancelAtFirst,
    );
    assert!(matches!(continued, Err(SynchronizationError::Interrupted)));
    assert_eq!(phase(&w, operation)?, "cancelled");
    assert_eq!(
        show(&w, operation).unwrap(),
        settled(
            operation,
            parents[0],
            parents[1],
            ConflictState::Cancelled,
            ConflictReadNextAction::Synchronize
        )
    );
    Ok(())
}

fn c4_conflict_read_complete() -> Result<(), FixtureError> {
    let w = world()?;
    let (operation, commit) = resolved_conflict(&w);
    let published = w.sync(restart(&w, operation)).unwrap();
    outcome(published, true, SynchronizationTarget::Primary, commit);
    let parents = parents_of(&w, commit);
    let dto = show(&w, operation).unwrap();
    assert_eq!(
        dto,
        settled(
            operation,
            parents[0],
            parents[1],
            ConflictState::Complete,
            ConflictReadNextAction::None
        )
    );
    assert_read_golden(&w, "conflict_show_complete", &dto);
    Ok(())
}

fn c4_conflict_read_busy() -> Result<(), FixtureError> {
    let w = world()?;
    let (operation, ..) = canonical_conflict(&w);
    let held = RepositoryService::hold_lease_for_testing(&w.root, &w.data(), LeaseKind::Repository)
        .unwrap();
    assert_eq!(show(&w, operation).unwrap_err().code(), ResultCode::Busy);
    drop(held);
    assert_eq!(
        show(&w, operation).unwrap().state,
        ConflictState::ConflictPending
    );
    Ok(())
}

fn c4_conflict_read_unknown_operation() -> Result<(), FixtureError> {
    let w = world()?;
    canonical_conflict(&w);
    assert_eq!(
        show(&w, OperationId::new()).unwrap_err().code(),
        ResultCode::OperationNotFound
    );
    Ok(())
}

fn c4_conflict_read_not_a_synchronization() -> Result<(), FixtureError> {
    let w = world()?;
    let created = execute(&w, REQUEST_2, create_other(&w));
    assert_eq!(created.code, ResultCode::Ok);
    let operation = OperationId::parse(created.operation_id.as_deref().unwrap()).unwrap();
    assert_eq!(
        show(&w, operation).unwrap_err().code(),
        ResultCode::InvalidInput
    );
    Ok(())
}

fn c4_conflict_read_local_sync() -> Result<(), FixtureError> {
    let w = world()?;
    let operation = leave_local_sync_conflict(&w);
    // Decision 19: found among the gate's records first, so never
    // `operation_not_found`.
    assert_eq!(
        show(&w, operation).unwrap_err().code(),
        ResultCode::InvalidInput
    );
    Ok(())
}

fn c4_conflict_read_comment_operation() -> Result<(), FixtureError> {
    let mut w = world()?;
    let context = w.context()?;
    assert_eq!(execute(&w, SETUP, item_sync(&w)).code, ResultCode::Ok);
    let path = merging::ticket_path(ITEM);
    commit_in(
        &context.worktree,
        CONTEXT,
        &path,
        merging::ticket_source(ITEM, "Local").as_bytes(),
    )?;
    merging::fetch_into_peer(&w, CONTEXT)?;
    let parent = w.peer.refname_to_id(CONTEXT).unwrap();
    let incoming = commit_path(
        &w.peer,
        parent,
        &path,
        merging::ticket_source(ITEM, "Incoming").as_bytes(),
    )?;
    w.peer
        .reference(CONTEXT, incoming, true, "fixture")
        .unwrap();
    push_peer(&w.peer, &w.server, CONTEXT)?;
    let stopped = execute(
        &w,
        REQUEST_1,
        Mutation::CommentAdd(CommentAddInput {
            root: w.root.clone(),
            kind: AuthoringKind::Ticket,
            id: ITEM.parse().unwrap(),
            comment_id: COMMENT.parse().unwrap(),
            parent_id: None,
            body: "A comment on a conflicted item.\n".into(),
            created_by: None,
        }),
    );
    assert_eq!(stopped.code, ResultCode::RecoveryRequired);
    let Some(MutationDataDto::Comment(data)) = &stopped.data else {
        panic!("a comment's data");
    };
    let comment = OperationId::parse(stopped.operation_id.as_deref().unwrap()).unwrap();
    let synchronization = OperationId::parse(&data.synchronization_id).unwrap();
    assert_ne!(comment, synchronization);
    // The comment's operation names its synchronization's conflict (R-5).
    let by_comment = show(&w, comment).unwrap();
    assert_eq!(by_comment.operation_id, synchronization.to_string());
    assert_eq!(by_comment.comment_operation_id, Some(comment.to_string()));
    assert_eq!(by_comment.state, ConflictState::ConflictPending);
    assert_eq!(by_comment.next_action, ConflictReadNextAction::Resolve);
    assert_eq!(
        Some(by_comment.next_action.as_str()),
        data.conflict
            .as_ref()
            .map(|conflict| conflict.next_action.as_str())
    );
    assert_eq!(by_comment.target, ConflictTarget::Ticket);
    assert_eq!(by_comment.item_id.as_deref(), Some(ITEM));
    assert_eq!(by_comment.stage, ConflictStage::Context);
    assert_eq!(by_comment.paths.len(), 1);
    // The synchronization's own ID reads the same.
    assert_eq!(show(&w, synchronization).unwrap(), by_comment);
    Ok(())
}

/// Folds the registry's write-ahead log into its file, so that the
/// file's bytes are every committed write.
fn checkpoint(w: &World) {
    let busy: i64 = w
        .db()
        .unwrap()
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| row.get(0))
        .unwrap();
    assert_eq!(busy, 0);
}

fn c4_conflict_read_writes_nothing() -> Result<(), FixtureError> {
    let w = world()?;
    let (operation, _, local, _) = canonical_conflict(&w);
    let registry = w.data().join(REGISTRY_FILE);
    checkpoint(&w);
    let before = std::fs::read(&registry).unwrap();
    // A live conflict, read with its sides; then the same operation
    // asked for under an unknown ID and while the lease is held.
    assert_eq!(
        show(&w, operation).unwrap().state,
        ConflictState::ConflictPending
    );
    assert_eq!(
        show(&w, OperationId::new()).unwrap_err().code(),
        ResultCode::OperationNotFound
    );
    let held = RepositoryService::hold_lease_for_testing(&w.root, &w.data(), LeaseKind::Repository)
        .unwrap();
    assert_eq!(show(&w, operation).unwrap_err().code(), ResultCode::Busy);
    drop(held);
    // The merge aborted outside: a read that judges the repair, under the
    // lease, and refuses it.
    let repo = w.repo().unwrap();
    repo.reset(
        &repo.find_object(local, None).unwrap(),
        git2::ResetType::Hard,
        None,
    )
    .unwrap();
    repo.cleanup_state().unwrap();
    assert_eq!(
        show(&w, operation).unwrap().state,
        ConflictState::RepairRefused
    );
    checkpoint(&w);
    assert!(std::fs::read(&registry).unwrap() == before);
    Ok(())
}

fn c4_conflict_read_hostile_path() -> Result<(), FixtureError> {
    // Windows cannot name such a file, so no conflict can hold one there.
    if cfg!(windows) {
        return Ok(());
    }
    let w = world()?;
    diverge(
        &w,
        &[raw_file(HOSTILE_PATH, b"base\n")],
        &[raw_file(HOSTILE_PATH, b"local\n")],
        &[raw_file(HOSTILE_PATH, b"incoming\n")],
    );
    let (operation, next) = stop(&w);
    let dto = show(&w, operation).unwrap();
    assert_eq!(dto.next_action.as_str(), next.as_str());
    assert_eq!(dto.paths.len(), 1);
    let path = &dto.paths[0];
    // Escaped once, by R-11's rule: no control byte is left in it.
    assert_eq!(path.display_path, "hostile-\\x1b[31mred\\x0aline.txt");
    assert!(!raw_control(&path.display_path));
    assert_eq!(path.kind, ConflictPathKind::Noncanonical);
    // It is text, but no item's: the read gives no sides (R-10).
    assert!(!path.content_available);
    assert!(path.base.is_none() && path.local.is_none() && path.incoming.is_none());
    // Neither rendering of the read holds a raw control byte.
    assert!(!raw_control(&format!("{dto:?}")));
    assert!(!raw_control(&serde_json::to_string(&dto).unwrap()));
    // Its token resolves to the library's own, whose side read the
    // library answers directly.
    let inspection = inspect(&w, operation);
    let tokens = repository::path_tokens(&inspection);
    let (_, token) = tokens.iter().find(|(text, _)| *text == path.token).unwrap();
    assert_eq!(token.clone(), inspection.paths[0].token);
    let sides = w.service.read_synchronization_conflict(token).unwrap();
    assert_eq!(
        sides.incoming.map(|side| side.bytes().to_vec()),
        Some(b"incoming\n".to_vec())
    );
    // Nor does a diagnostic of a read of it.
    let held = RepositoryService::hold_lease_for_testing(&w.root, &w.data(), LeaseKind::Repository)
        .unwrap();
    let error = show(&w, operation).unwrap_err();
    drop(held);
    assert_eq!(error.code(), ResultCode::Busy);
    assert!(!raw_control(&format!("{error} {error:?}")));
    Ok(())
}

fn c4_conflict_read_display_path() -> Result<(), FixtureError> {
    // R-11, byte for byte.
    assert_eq!(repository::conflict_display_path(DISPLAY_PATH), DISPLAYED);
    assert_eq!(
        repository::conflict_display_path(b" printable~ASCII/kept.md"),
        " printable~ASCII/kept.md"
    );
    assert_eq!(repository::conflict_display_path(b"\x7f\x1f"), "\\x7f\\x1f");
    // The same in a read. Only Linux names a file with every one of these
    // bytes: macOS refuses the byte that is no UTF-8, and Windows the
    // control byte and the backslash.
    if !cfg!(target_os = "linux") {
        return Ok(());
    }
    let w = world()?;
    diverge(
        &w,
        &[raw_file(DISPLAY_PATH, b"base\n")],
        &[raw_file(DISPLAY_PATH, b"local\n")],
        &[raw_file(DISPLAY_PATH, b"incoming\n")],
    );
    let (operation, _) = stop(&w);
    let dto = show(&w, operation).unwrap();
    assert_eq!(dto.paths.len(), 1);
    assert_eq!(dto.paths[0].display_path.as_bytes(), DISPLAYED.as_bytes());
    // A path that is no UTF-8 is never read for its sides (fact 32).
    assert!(!dto.paths[0].content_available);
    assert_eq!(dto.paths[0].kind, ConflictPathKind::Noncanonical);
    Ok(())
}
