//! `document promote` and `ticket close` over SSH, through the binary: the
//! Cycle's lifecycle exit evidence (Task 12a). Each case is one fresh pair of
//! clones (`CliPair`), under the harness's watchdog, split by row and verb.
//!
//! The library's tables (`tests/mutation_remote/c5_lifecycle.rs`,
//! `c5_resume.rs`, `c5_cancel.rs`) pin every answer; these pin what a person
//! meets, and what Git holds afterwards: the server's refs, each clone's
//! branches, the merges on primary, the closure checkpoint and its fields.
//! Rows the binary already proves elsewhere are not repeated here:
//! `c5_polling_promote_then_close_needs_no_poll` (both verbs published in one
//! clone, then `ticket_closed`) and `c5_polling_item_sync_of_a_copy_the_remote_still_holds`
//! (a refused deletion's first answer).
//!
//! A CLI child can't take a failure point. Where a row needs a stop the binary
//! can't stage, the first call runs in this process, on the clone's own data
//! and with the confirmation the binary issued, and everything after it runs
//! through the binary: `PrimaryAdvanced` (stopped after the context's
//! synchronization), the refused checkpoint merge (stopped after the closure
//! checkpoint), and a parent no request record holds (begun through the
//! domain's own entry point, as `c5_resume` begins one).
//!
//! Rows that reach only shared lifecycle code run for one verb, each saying so
//! (decision 27).

use std::path::PathBuf;

use super::*;
use cli::{stderr, stdout};
use cli_pair::{CONTEXT, CliClone, CliPair, ITEM, PRIMARY};
use collaboration::{connections, item_sync, request_show, save};
use repository::{
    CancellationToken, CloseTicketRequest, ConfirmationId, DocumentPromoteInput,
    DocumentPromotionPreflight, FailurePoint, Mutation, MutationCall, NoProgress, OperationId,
    PreviewDocumentPromotionRequest, PreviewTicketClosureRequest, PromoteDocumentRequest,
    RepositoryService, RequestId, TicketCloseInput, TicketClosurePreflight,
    keys::SessionCredentials,
};
use ssh_remote::{FixtureError, SshRemoteFixture, fixed};

macro_rules! both {
    ($( $name:literal => $row:expr ),+ $(,)?) => {
        &[$(
            (concat!("c5_lifecycle_", $name, "_promote"), || $row(Verb::Promote)),
            (concat!("c5_lifecycle_", $name, "_close"), || $row(Verb::Close)),
        )+]
    };
}

pub const BOTH: &[ssh_harness::Case] = both![
    "dirty_primary_blocks" => dirty_primary_blocks,
    "stale_consent_context_moved" => stale_consent_context_moved,
    "stale_consent_primary_moved" => stale_consent_primary_moved,
    "stale_consent_item_path_moved" => stale_consent_item_path_moved,
    "stale_consent_local_only_primary_moved" => stale_consent_local_only_primary_moved,
    "refused_primary_push_then_same_request" => refused_primary_push_then_same_request,
    "refused_deletion_then_same_request" => refused_deletion_then_same_request,
    "primary_advanced_continues" => primary_advanced_continues,
    "resume_with_no_record_continues" => resume_with_no_record_continues,
    "deleted_branch_request_route" => deleted_branch_request_route,
    "deleted_branch_resume_route" => deleted_branch_resume_route,
];

pub const ONE: &[ssh_harness::Case] = &[
    (
        "c5_lifecycle_host_stop_then_same_request_close",
        c5_lifecycle_host_stop_then_same_request_close,
    ),
    #[cfg(unix)]
    (
        "c5_lifecycle_unlock_stop_then_same_request_close",
        c5_lifecycle_unlock_stop_then_same_request_close,
    ),
    (
        "c5_lifecycle_refused_checkpoint_merge_repaired_close",
        c5_lifecycle_refused_checkpoint_merge_repaired_close,
    ),
    (
        "c5_lifecycle_walk_prepare_conflict_close",
        c5_lifecycle_walk_prepare_conflict_close,
    ),
    (
        "c5_lifecycle_walk_refused_merge_under_resume_close",
        c5_lifecycle_walk_refused_merge_under_resume_close,
    ),
    (
        "c5_lifecycle_walk_primary_ahead_close",
        c5_lifecycle_walk_primary_ahead_close,
    ),
    (
        "c5_lifecycle_walk_primary_missing_close",
        c5_lifecycle_walk_primary_missing_close,
    ),
    (
        "c5_lifecycle_walk_context_missing_close",
        c5_lifecycle_walk_context_missing_close,
    ),
    (
        "c5_lifecycle_exit_evidence_close",
        c5_lifecycle_exit_evidence_close,
    ),
    (
        "c5_lifecycle_walk_poll_busy_route",
        c5_lifecycle_walk_poll_busy_route,
    ),
];

const R1: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR1";
const R2: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR2";
const R3: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR3";
const R4: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR4";
const R5: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR5";
const R6: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR6";
/// The document `document promote` promotes, which clone A creates.
pub(super) const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FDC";
pub(super) const DOCUMENT_PATH: &str = "docs/lifecycle.md";
pub(super) const DOCUMENT_BRANCH: &str = "refs/heads/manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FDC";
pub(super) const MUTATION: &str = "lifecycle_mutation.schema.json";

/// Which of the two verbs a row is of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Verb {
    Promote,
    Close,
}

impl Verb {
    pub(super) fn words(self) -> [&'static str; 2] {
        match self {
            Self::Promote => ["document", "promote"],
            Self::Close => ["ticket", "close"],
        }
    }

    pub(super) fn id(self) -> &'static str {
        match self {
            Self::Promote => DOCUMENT,
            Self::Close => ITEM,
        }
    }

    pub(super) fn branch(self) -> &'static str {
        match self {
            Self::Promote => DOCUMENT_BRANCH,
            Self::Close => CONTEXT,
        }
    }

    /// The item's canonical file.
    pub(super) fn path(self) -> String {
        match self {
            Self::Promote => DOCUMENT_PATH.into(),
            Self::Close => format!(".manyhands/tickets/{ITEM}/ticket.md"),
        }
    }

    pub(super) fn worktree(self, clone: &CliClone) -> PathBuf {
        clone.root.join(".manyhands/worktrees").join(self.id())
    }

    fn args(self, clone: &CliClone) -> Vec<&str> {
        let [resource, verb] = self.words();
        vec![resource, verb, "--repo", clone.root(), "--id", self.id()]
    }

    /// Gives `clone` an editing context of the verb's item with a commit
    /// primary does not hold: a ticket save, or a document's creation.
    pub(super) fn begin(self, clone: &CliClone, request: &str) {
        match self {
            Self::Close => {
                save(clone, request, "Saved for its lifecycle.\n");
            }
            Self::Promote => {
                let created = serde_json::json!({
                    "item_id": DOCUMENT,
                    "path": DOCUMENT_PATH,
                    "title": "Lifecycle document",
                    "body": "A document to promote.\n",
                });
                let mut args = vec!["document", "create", "--repo", clone.root()];
                args.extend(["--request-id", request, "--input", "-"]);
                clone.json(&args, &serde_json::to_vec(&created).unwrap(), "ok", 0);
            }
        }
        assert!(clone.tip(self.branch()).unwrap().is_some());
    }

    /// A further save of the item's body, which moves its context.
    pub(super) fn resave(self, clone: &CliClone, request: &str, body: &str) {
        match self {
            Self::Close => {
                save(clone, request, body);
            }
            Self::Promote => {
                let shown = clone.json(
                    &["document", "show", "--repo", clone.root(), "--id", DOCUMENT],
                    &[],
                    "ok",
                    0,
                );
                let input =
                    serde_json::json!({"observation": shown["data"]["observation"], "body": body});
                let mut args = vec!["document", "save", "--repo", clone.root(), "--id", DOCUMENT];
                args.extend(["--request-id", request, "--input", "-"]);
                clone.json(&args, &serde_json::to_vec(&input).unwrap(), "ok", 0);
            }
        }
    }

    /// The item created in a clone whose primary doesn't hold it, as a
    /// `LocalOnly` clone's doesn't.
    pub(super) fn create(self, clone: &CliClone, request: &str) {
        match self {
            Self::Promote => self.begin(clone, request),
            Self::Close => {
                let created = serde_json::json!({
                    "item_id": ITEM,
                    "title": "Local ticket",
                    "type": "task",
                    "status": "open",
                    "body": "Closed while local only.\n",
                });
                let mut args = vec!["ticket", "create", "--repo", clone.root()];
                args.extend(["--request-id", request, "--input", "-"]);
                clone.json(&args, &serde_json::to_vec(&created).unwrap(), "ok", 0);
            }
        }
    }

    /// `begin`, then the context published by `item sync`.
    pub(super) fn begin_published(self, p: &CliPair, save: &str, sync: &str) {
        self.begin(&p.a, save);
        item_sync(&p.a, self.id(), sync, "ok", 0);
        assert_eq!(
            p.served(self.branch()).unwrap(),
            p.a.tip(self.branch()).unwrap()
        );
    }

    /// A `--prepare` in JSON mode.
    pub(super) fn prepare(self, clone: &CliClone, code: &str, status: i32) -> Value {
        let mut args = self.args(clone);
        args.push("--prepare");
        let value = clone.json(&args, &[], code, status);
        assert!(value["request_id"].is_null());
        value
    }

    /// The preview's confirmation under `request`.
    pub(super) fn confirm(
        self,
        clone: &CliClone,
        request: &str,
        preview: &Value,
        code: &str,
        status: i32,
    ) -> Value {
        let mut args = self.args(clone);
        args.extend([
            "--request-id",
            request,
            "--confirm",
            preview["data"]["confirmation_id"].as_str().unwrap(),
        ]);
        lifecycle_data(clone.json(&args, &[], code, status))
    }

    /// The same request again, as a caller sends it: its ID and no consent.
    pub(super) fn send(self, clone: &CliClone, request: &str, code: &str, status: i32) -> Value {
        let mut args = self.args(clone);
        args.extend(["--request-id", request]);
        lifecycle_data(clone.json(&args, &[], code, status))
    }

    /// The mutation the boundary runs for the verb, in this process.
    fn mutation(self, clone: &CliClone) -> Mutation {
        let (root, id) = (clone.root.clone(), self.id().parse().unwrap());
        match self {
            Self::Promote => Mutation::DocumentPromote(DocumentPromoteInput { root, id }),
            Self::Close => Mutation::TicketClose(TicketCloseInput { root, id }),
        }
    }
}

/// The request that enables a `LocalOnly` clone.
const ENABLE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR8";

/// A clone of the remote made a Manyhands repository through the binary
/// with no publication remote and no key: local only. Its Git remote `origin`
/// is the server, as a person's clone has it.
pub(super) struct LocalOnly {
    pub(super) clone: CliClone,
    _directory: tempfile::TempDir,
}

impl LocalOnly {
    pub(super) fn new(server: &SshRemoteFixture) -> Result<Self, FixtureError> {
        let directory = fixed(tempfile::tempdir())?;
        let root = directory.path().join("local-only");
        let mut fetch = git2::FetchOptions::new();
        fetch.remote_callbacks(cli_pair::callbacks(server)?);
        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch);
        fixed(builder.clone(&server.url(), &root))?;
        let env = CliEnv::new(directory.path().join("data"));
        fixed(std::fs::write(
            env.home.path().join(".gitconfig"),
            "[core]\n\tautocrlf = false\n",
        ))?;
        let clone = CliClone { env, root };
        let input = serde_json::to_vec(&serde_json::json!({
            "primary_branch": "main",
            "identity": {"name": "Local Person", "email": "local@example.invalid"},
        }))
        .unwrap();
        let args = ["repo", "enable", "--repo", clone.root(), "--input", "-"];
        let preview = clone.json(&[&args[..], &["--prepare"][..]].concat(), &input, "ok", 0);
        clone.json(
            &[
                &args[..],
                &[
                    "--request-id",
                    ENABLE,
                    "--confirm",
                    preview["data"]["confirmation_id"].as_str().unwrap(),
                ][..],
            ]
            .concat(),
            &input,
            "ok",
            0,
        );
        Ok(Self {
            clone,
            _directory: directory,
        })
    }
}

/// An answer whose data is the lifecycle's, checked against its schema.
fn lifecycle_data(value: Value) -> Value {
    if !value["data"].is_null() {
        schema::check_published(MUTATION, &value["data"]).unwrap();
        assert_eq!(value["data"]["type"], "lifecycle", "{value}");
    }
    value
}

fn actions(value: &Value) -> Vec<&str> {
    value["recovery"]
        .as_array()
        .unwrap()
        .iter()
        .map(|action| action["action"].as_str().unwrap())
        .collect()
}

/// Every ref of `repository`, by name.
pub(super) fn refs(
    repository: &git2::Repository,
) -> Result<std::collections::BTreeMap<String, git2::Oid>, FixtureError> {
    let mut found = std::collections::BTreeMap::new();
    for reference in fixed(repository.references())? {
        let reference = fixed(reference)?;
        if let (Some(name), Some(oid)) = (reference.name(), reference.target()) {
            found.insert(name.to_owned(), oid);
        }
    }
    Ok(found)
}

/// The merge commits on primary's own line: one, however many calls.
pub(super) fn merges(repository: &git2::Repository) -> Result<usize, FixtureError> {
    let mut walk = fixed(repository.revwalk())?;
    fixed(walk.push(fixed(repository.refname_to_id(PRIMARY))?))?;
    // A context's synchronization merges primary into it, on the side.
    fixed(walk.simplify_first_parent())?;
    let mut count = 0;
    for oid in walk {
        if fixed(repository.find_commit(fixed(oid)?))?.parent_count() == 2 {
            count += 1;
        }
    }
    Ok(count)
}

/// `path`'s source at `oid`, if the commit holds it.
pub(super) fn source_at(
    repository: &git2::Repository,
    oid: git2::Oid,
    path: &str,
) -> Result<Option<String>, FixtureError> {
    let tree = fixed(fixed(repository.find_commit(oid))?.tree())?;
    let Ok(entry) = tree.get_path(std::path::Path::new(path)) else {
        return Ok(None);
    };
    let blob = fixed(repository.find_blob(entry.id()))?;
    Ok(Some(String::from_utf8_lossy(blob.content()).into_owned()))
}

/// The closure's fields in a ticket's source: its `closed_at` and `closed_by`.
pub(super) fn closure_fields(source: &str) -> Vec<String> {
    source
        .lines()
        .filter(|line| line.starts_with("closed_at:") || line.starts_with("closed_by:"))
        .map(str::to_owned)
        .collect()
}

/// The closure's fields as `reference` holds them in `repository`.
pub(super) fn closure_at(
    repository: &git2::Repository,
    reference: &str,
) -> Result<Vec<String>, FixtureError> {
    let tip = fixed(repository.refname_to_id(reference))?;
    let source = source_at(repository, tip, &Verb::Close.path())?.ok_or(FixtureError)?;
    Ok(closure_fields(&source))
}

/// The commits in primary's history that write the closure: each holds a
/// `closed_at` that none of its parents does.
pub(super) fn closure_checkpoints(repository: &git2::Repository) -> Result<usize, FixtureError> {
    let path = Verb::Close.path();
    let closed = |oid: git2::Oid| -> Result<bool, FixtureError> {
        Ok(source_at(repository, oid, &path)?
            .is_some_and(|source| source.contains("\nclosed_at: ")))
    };
    let mut walk = fixed(repository.revwalk())?;
    fixed(walk.push(fixed(repository.refname_to_id(PRIMARY))?))?;
    let mut count = 0;
    for oid in walk {
        let oid = fixed(oid)?;
        let commit = fixed(repository.find_commit(oid))?;
        if closed(oid)? {
            let mut parents = commit.parent_ids();
            if !parents.any(|parent| closed(parent).unwrap()) {
                count += 1;
            }
        }
    }
    Ok(count)
}

/// After a finished run: one merge, primary published as it stands here, the
/// context gone at both ends, and for a closure one checkpoint whose fields
/// are `closure` when given.
pub(super) fn finished_once(
    p: &CliPair,
    verb: Verb,
    closure: Option<&[String]>,
) -> Result<(), FixtureError> {
    let a = &p.a;
    let repository = a.repo()?;
    assert_eq!(merges(&repository)?, 1, "one merge");
    assert_eq!(p.served(PRIMARY)?, a.tip(PRIMARY)?, "primary is published");
    assert_eq!(p.served(verb.branch())?, None, "the remote branch is gone");
    assert_eq!(a.tip(verb.branch())?, None, "the local branch is gone");
    assert!(!verb.worktree(a).exists(), "the worktree is gone");
    let primary = fixed(repository.refname_to_id(PRIMARY))?;
    assert!(source_at(&repository, primary, &verb.path())?.is_some());
    if verb == Verb::Close {
        assert_eq!(
            closure_checkpoints(&repository)?,
            1,
            "one closure checkpoint"
        );
        let fields = closure_at(&repository, PRIMARY)?;
        assert_eq!(fields.len(), 2, "{fields:?}");
        if let Some(closure) = closure {
            assert_eq!(fields, closure, "closed_at and closed_by keep their values");
        }
    }
    Ok(())
}

/// A peer's commit on the remote's primary that writes `path` as `bytes`.
pub(super) fn peer_commit(
    p: &CliPair,
    path: &str,
    bytes: &[u8],
) -> Result<git2::Oid, FixtureError> {
    let bare = p.bare()?;
    let parent = fixed(bare.find_commit(fixed(bare.refname_to_id(PRIMARY))?))?;
    let mut index = fixed(git2::Index::new())?;
    fixed(index.read_tree(&fixed(parent.tree())?))?;
    fixed(index.add(&git2::IndexEntry {
        ctime: git2::IndexTime::new(0, 0),
        mtime: git2::IndexTime::new(0, 0),
        dev: 0,
        ino: 0,
        mode: 0o100644,
        uid: 0,
        gid: 0,
        file_size: 0,
        id: fixed(bare.blob(bytes))?,
        flags: 0,
        flags_extended: 0,
        path: path.as_bytes().to_vec(),
    }))?;
    let tree = fixed(bare.find_tree(fixed(index.write_tree_to(&bare))?))?;
    let signature = fixed(git2::Signature::now("Peer", "peer@example.invalid"))?;
    fixed(bare.commit(
        Some(PRIMARY),
        &signature,
        &signature,
        "A peer's commit",
        &tree,
        &[&parent],
    ))
}

/// One `poll once`, `ok`, through the binary.
pub(super) fn poll(clone: &CliClone, request: &str) -> Value {
    clone.json(
        &[
            "poll",
            "once",
            "--repo",
            clone.root(),
            "--request-id",
            request,
        ],
        &[],
        "ok",
        0,
    )
}

/// The confirmed request `request` run in this process by a service that
/// stops at `point`, on the clone's own data and with the binary's
/// confirmation: a stop no CLI child can stage.
fn stopped_in_process(
    clone: &CliClone,
    verb: Verb,
    request: &str,
    confirmation: &str,
    point: FailurePoint,
) -> String {
    let service = support::FailOnce::at(point).open_service(&clone.env.data);
    let stopped = service.execute(
        MutationCall {
            request_id: RequestId::parse(request).unwrap(),
            confirmation: Some(ConfirmationId::parse(confirmation).unwrap()),
            mutation: verb.mutation(clone),
        },
        &mut SessionCredentials::new(remote_world::Provider),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_ne!(stopped.code.as_str(), "ok", "{:?}", stopped.data);
    stopped.operation_id.clone().expect("a parent stands")
}

/// A parent begun through the domain's own entry point, as a caller that is
/// not the boundary begins one, and stopped at its first safe point: no
/// request record holds it.
fn begun_outside(clone: &CliClone, verb: Verb) -> Result<String, FixtureError> {
    let service: RepositoryService =
        support::FailOnce::at(FailurePoint::AfterConfirmed).open_service(&clone.env.data);
    let operation_id = OperationId::new();
    let item_id: manyhands::canonical::ItemId = fixed(verb.id().parse())?;
    let mut session = SessionCredentials::new(remote_world::Provider);
    match verb {
        Verb::Promote => {
            let DocumentPromotionPreflight::Ready(preview) = fixed(
                service.preview_document_promotion(PreviewDocumentPromotionRequest {
                    root: clone.root.clone(),
                    item_id: item_id.clone(),
                }),
            )?
            else {
                return Err(FixtureError);
            };
            let _stopped = service.promote_document(
                PromoteDocumentRequest {
                    root: clone.root.clone(),
                    operation_id,
                    item_id,
                    expected: preview.expectation(),
                    approval: None,
                    confirmed_identity: None,
                },
                &mut session,
            );
        }
        Verb::Close => {
            let TicketClosurePreflight::Ready(preview) =
                fixed(service.preview_ticket_closure(PreviewTicketClosureRequest {
                    root: clone.root.clone(),
                    item_id: item_id.clone(),
                    confirmed_identity: None,
                }))?
            else {
                return Err(FixtureError);
            };
            let _stopped = service.close_ticket(
                CloseTicketRequest {
                    root: clone.root.clone(),
                    operation_id,
                    item_id,
                    expected: preview.expectation(),
                    approval: None,
                    confirmed_identity: None,
                },
                &mut session,
            );
        }
    }
    Ok(operation_id.to_string())
}

fn resume_args<'a>(clone: &'a CliClone, operation: &'a str, request: &'a str) -> Vec<&'a str> {
    collaboration::resume_args(clone, operation, request)
}

/// One `operation resume` in JSON mode, its lifecycle data checked.
fn resumed(clone: &CliClone, operation: &str, request: &str, code: &str, status: i32) -> Value {
    lifecycle_data(clone.json(&resume_args(clone, operation, request), &[], code, status))
}

/// A `next:` line split as a shell would: single quotes group, a backslash
/// escapes outside them.
fn words(line: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let (mut quoted, mut any) = (false, false);
    let mut chars = line.chars();
    while let Some(character) = chars.next() {
        match character {
            '\'' => {
                quoted = !quoted;
                any = true;
            }
            '\\' if !quoted => {
                if let Some(next) = chars.next() {
                    word.push(next);
                }
            }
            ' ' if !quoted => {
                if any || !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                }
                any = false;
            }
            _ => word.push(character),
        }
    }
    if any || !word.is_empty() {
        words.push(word);
    }
    words
}

/// The commands a human answer's `next:` lines name, as argument lists.
fn commands(text: &str) -> Vec<Vec<String>> {
    text.lines()
        .filter_map(|line| line.strip_prefix("next: manyhands-cli "))
        .map(words)
        .collect()
}

/// `command` in JSON mode, with `extra` arguments.
fn json_of(clone: &CliClone, command: &[String], extra: &[&str], code: &str, status: i32) -> Value {
    let mut args: Vec<&str> = command.iter().map(String::as_str).collect();
    args.extend_from_slice(extra);
    clone.json(&args, &[], code, status)
}

fn value_after<'a>(text: &'a str, label: &str) -> &'a str {
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("{label}: ")))
        .unwrap_or_else(|| panic!("no {label}: {text}"))
}

// ---------------------------------------------------------------- prepare

/// A dirty primary blocks the prepare and names itself; nothing moves, at
/// either end, and no request is recorded. Clean again, the prepare is ready.
fn dirty_primary_blocks(verb: Verb) -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let a = &p.a;
    verb.begin(a, R1);
    fixed(std::fs::write(a.root.join("dirty-primary.txt"), b"dirty\n"))?;
    let (local, served) = (refs(&a.repo()?)?, refs(&p.bare()?)?);
    let pushes = remote_world::accepted(&p.server);

    let blocked = verb.prepare(a, "recovery_required", 3);
    assert_eq!(blocked["outcome"], "blocked");
    assert_eq!(
        blocked["data"]["blockers"],
        serde_json::json!(["primary_dirty"])
    );
    schema::check_published("lifecycle_blocked.schema.json", &blocked["data"]).unwrap();
    assert_eq!(actions(&blocked), ["repo.inspect"]);
    let mut args = verb.args(a);
    args.push("--prepare");
    let out = run(&a.env, &args, &[]);
    assert_eq!(out.status, 3);
    assert!(
        stderr(&out).contains("Blockers: primary_dirty\n"),
        "{}",
        stderr(&out)
    );
    assert_eq!(refs(&a.repo()?)?, local, "no ref moves here");
    assert_eq!(refs(&p.bare()?)?, served, "nor on the remote");
    assert_eq!(remote_world::accepted(&p.server), pushes);

    fixed(std::fs::remove_file(a.root.join("dirty-primary.txt")))?;
    let ready = verb.prepare(a, "ok", 0);
    assert!(ready["data"]["confirmation_id"].is_string());
    Ok(())
}

// ---------------------------------------------------------------- confirm

/// A stale consent: refused at confirm, with no record and nothing done.
fn assert_refused_consent(
    clone: &CliClone,
    verb: Verb,
    refused: &Value,
) -> Result<(), FixtureError> {
    assert_eq!(refused["outcome"], "blocked", "{refused}");
    assert_eq!(actions(refused), ["request.prepare"]);
    assert_eq!(refused["effects"]["integration"], "not_requested");
    checked(
        &clone.env,
        &["request", "show", "--request-id", R3],
        &[],
        "request_not_found",
        2,
        None,
    );
    assert_eq!(merges(&clone.repo()?)?, 0);
    assert!(clone.tip(verb.branch())?.is_some(), "the context stays");
    Ok(())
}

/// `assert_refused_consent`, then a fresh consent that finishes once.
fn assert_stale(p: &CliPair, verb: Verb, refused: &Value) -> Result<(), FixtureError> {
    let a = &p.a;
    assert_refused_consent(a, verb, refused)?;
    let preview = verb.prepare(a, "ok", 0);
    let done = verb.confirm(a, R4, &preview, "ok", 0);
    assert_eq!(done["effects"]["publication"], "published", "{done}");
    finished_once(p, verb, None)
}

// The consent rule is Wave 02 Cycle 09's D6, which closure follows: with a
// remote the digest leaves out the context, local-primary and tracking OIDs,
// and the item's change set and the D2 relation stand in; with none it binds
// both local OIDs.

/// The context moves between the prepare and its confirmation: a second save,
/// which changes the change set. The confirmation answers `external_change`,
/// with no record and nothing done, and a fresh consent completes.
fn stale_consent_context_moved(verb: Verb) -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let a = &p.a;
    verb.begin(a, R1);
    let preview = verb.prepare(a, "ok", 0);
    verb.resave(a, R2, "Saved again before the confirmation.\n");
    let tip = a.tip(verb.branch())?;
    let refused = verb.confirm(a, R3, &preview, "external_change", 3);
    assert_eq!(a.tip(verb.branch())?, tip);
    assert_stale(&p, verb, &refused)
}

/// With a remote, a collaborator's primary advance that leaves the item's
/// change set as it was is accepted: a peer's commit of another file, polled
/// in, and the confirmation integrates onto it and publishes, once.
fn stale_consent_primary_moved(verb: Verb) -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let a = &p.a;
    verb.begin(a, R1);
    let preview = verb.prepare(a, "ok", 0);
    let incoming = peer_commit(&p, "incoming.txt", b"incoming\n")?;
    poll(a, R2);
    assert_eq!(a.tip(PRIMARY)?, Some(incoming));
    assert_ne!(
        preview["data"]["details"]["primary_oid"],
        incoming.to_string().as_str()
    );
    let done = verb.confirm(a, R3, &preview, "ok", 0);
    assert_eq!(done["effects"]["publication"], "published", "{done}");
    finished_once(&p, verb, None)?;
    let repository = a.repo()?;
    let merge = fixed(repository.find_commit(fixed(repository.refname_to_id(PRIMARY))?))?;
    assert_eq!(merge.parent_id(0).ok(), Some(incoming), "onto the advance");
    Ok(())
}

/// With a remote, a primary advance that touches the item's own path changes
/// its change set: the confirmation answers `external_change`. For a ticket,
/// a peer retitles it (a change Git merges cleanly); for a document, a peer
/// adds the same path with the bytes the context holds.
fn stale_consent_item_path_moved(verb: Verb) -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let a = &p.a;
    verb.begin(a, R1);
    let preview = verb.prepare(a, "ok", 0);
    let bytes = match verb {
        Verb::Close => format!(
            "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {ITEM}\ntitle: Retitled on primary\n\
             type: task\nstatus: open\n---\nBody of Shared.\n"
        ),
        Verb::Promote => fixed(std::fs::read_to_string(verb.worktree(a).join(verb.path())))?,
    };
    let incoming = peer_commit(&p, &verb.path(), bytes.as_bytes())?;
    poll(a, R2);
    assert_eq!(a.tip(PRIMARY)?, Some(incoming));
    let pushes = remote_world::accepted(&p.server);
    let refused = verb.confirm(a, R3, &preview, "external_change", 3);
    assert_refused_consent(a, verb, &refused)?;
    assert_eq!(a.tip(PRIMARY)?, Some(incoming));
    assert_eq!(p.served(PRIMARY)?, Some(incoming));
    assert_eq!(remote_world::accepted(&p.server), pushes, "nothing pushed");
    Ok(())
}

/// Without a remote, any primary move refuses the consent: a person's own
/// commit of another file on primary, between the prepare and its
/// confirmation, answers `external_change`.
fn stale_consent_local_only_primary_moved(verb: Verb) -> Result<(), FixtureError> {
    let server = SshRemoteFixture::start()?;
    private_fixture(&server);
    let local = LocalOnly::new(&server)?;
    let clone = &local.clone;
    verb.create(clone, R1);
    let preview = verb.prepare(clone, "ok", 0);
    assert!(preview["data"]["details"]["remote"].is_null());
    let moved = own_commit(clone, "unrelated.txt", b"unrelated\n")?;
    let refused = verb.confirm(clone, R3, &preview, "external_change", 3);
    assert_refused_consent(clone, verb, &refused)?;
    assert_eq!(clone.tip(PRIMARY)?, Some(moved));
    Ok(())
}

/// A person's own commit of `path` on the clone's primary, made with Git and
/// checked out, which leaves the worktree clean.
fn own_commit(clone: &CliClone, path: &str, bytes: &[u8]) -> Result<git2::Oid, FixtureError> {
    let repository = clone.repo()?;
    let parent = fixed(repository.find_commit(fixed(repository.refname_to_id(PRIMARY))?))?;
    let mut builder = fixed(repository.treebuilder(Some(&fixed(parent.tree())?)))?;
    fixed(builder.insert(path, fixed(repository.blob(bytes))?, 0o100644))?;
    let tree = fixed(repository.find_tree(fixed(builder.write())?))?;
    let signature = fixed(git2::Signature::now("A person", "person@example.invalid"))?;
    let commit = fixed(repository.commit(
        None,
        &signature,
        &signature,
        "a person's own commit",
        &tree,
        &[&parent],
    ))?;
    fixed(repository.checkout_tree(
        &fixed(repository.find_object(commit, None))?,
        Some(git2::build::CheckoutBuilder::new().safe()),
    ))?;
    fixed(repository.reference(PRIMARY, commit, true, "a person's own commit"))?;
    Ok(commit)
}

// --------------------------------------------------- refusals at the remote

/// The remote refuses primary after integration: exit 4, cleanup pending, the
/// context kept. `operation resume` names the request that holds it. The same
/// request, refused again, then completes once the remote takes it, with one
/// merge and one checkpoint, and replays with no connection.
fn refused_primary_push_then_same_request(verb: Verb) -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let a = &p.a;
    verb.begin(a, R1);
    let preview = verb.prepare(a, "ok", 0);
    let served = p.served(PRIMARY)?;
    p.server.reject_primary_updates(true)?;
    let stopped = verb.confirm(a, R2, &preview, "push_rejected", 4);
    assert_eq!(stopped["outcome"], "partial");
    assert_eq!(stopped["effects"]["integration"], "complete");
    assert_eq!(stopped["effects"]["publication"], "pending");
    assert_eq!(stopped["effects"]["cleanup"], "pending");
    assert_eq!(actions(&stopped)[0], "request.retry");
    assert_eq!(request_show(a, R2)["state"], "accepted");
    assert_eq!(p.served(PRIMARY)?, served, "primary is refused");
    assert_eq!(merges(&a.repo()?)?, 1, "integrated here");
    assert!(a.tip(verb.branch())?.is_some(), "the context stays");
    assert!(verb.worktree(a).is_dir());
    let closure = (verb == Verb::Close).then(|| closure_at(&a.repo().unwrap(), PRIMARY).unwrap());
    let operation = stopped["operation_id"].as_str().unwrap().to_owned();

    // The resume names the request: that request continues it.
    let named = resumed(a, &operation, R3, "original_request_required", 3);
    assert_eq!(actions(&named), ["request.retry"]);
    assert!(
        named["recovery"][0].to_string().contains(R2),
        "names the request: {named}"
    );
    checked(
        &a.env,
        &["request", "show", "--request-id", R3],
        &[],
        "request_not_found",
        2,
        None,
    );

    // Still refused: the same answer, and nothing repeated.
    let again = verb.send(a, R2, "push_rejected", 4);
    assert_eq!(again["operation_id"], stopped["operation_id"]);
    assert_eq!(merges(&a.repo()?)?, 1);

    p.server.reject_primary_updates(false)?;
    let done = verb.send(a, R2, "ok", 0);
    assert_eq!(done["operation_id"], stopped["operation_id"]);
    assert_eq!(done["effects"]["publication"], "published", "{done}");
    assert_eq!(done["effects"]["cleanup"], "complete", "{done}");
    assert_eq!(request_show(a, R2)["state"], "finished");
    finished_once(&p, verb, closure.as_deref())?;

    // The finished request replays its answer, with no connection.
    let opened = connections(&p);
    assert_eq!(verb.send(a, R2, "ok", 0), done);
    assert_eq!(connections(&p), opened);
    finished_once(&p, verb, closure.as_deref())
}

/// The remote refuses the context's deletion: published, cleanup pending. The
/// same request completes once the remote takes it.
fn refused_deletion_then_same_request(verb: Verb) -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let a = &p.a;
    verb.begin_published(&p, R1, R2);
    let preview = verb.prepare(a, "ok", 0);
    p.server.reject_context_deletion(true)?;
    // The fixture's native refusal applies to the receiver's current branch.
    fixed(p.bare()?.set_head(verb.branch()))?;
    let stopped = verb.confirm(a, R3, &preview, "push_rejected", 4);
    assert_eq!(stopped["outcome"], "partial");
    assert_eq!(stopped["effects"]["publication"], "published");
    assert_eq!(stopped["effects"]["cleanup"], "pending");
    assert_eq!(actions(&stopped), ["request.retry"]);
    assert_eq!(
        stopped["data"]["cleanup_pending"],
        "remote_deletion_rejected"
    );
    assert_eq!(request_show(a, R3)["state"], "accepted");
    assert_eq!(p.served(PRIMARY)?, a.tip(PRIMARY)?, "primary is published");
    assert!(
        p.served(verb.branch())?.is_some(),
        "the remote branch stays"
    );
    assert_eq!(merges(&a.repo()?)?, 1);
    let closure = (verb == Verb::Close).then(|| closure_at(&a.repo().unwrap(), PRIMARY).unwrap());

    p.server.reject_context_deletion(false)?;
    fixed(p.bare()?.set_head(PRIMARY))?;
    let done = verb.send(a, R3, "ok", 0);
    assert_eq!(done["operation_id"], stopped["operation_id"]);
    assert_eq!(done["effects"]["cleanup"], "complete", "{done}");
    finished_once(&p, verb, closure.as_deref())?;
    let opened = connections(&p);
    assert_eq!(verb.send(a, R3, "ok", 0), done);
    assert_eq!(connections(&p), opened);
    Ok(())
}

// ------------------------------------------------------------ continuation

/// Primary moves under a synchronized context: `external_change`, the parent
/// unfinished, and the same request then starts again from the context and
/// merges once. The first call stops in this process after the context's
/// synchronization, which no CLI child can stage. The context is published
/// first, so that stop has no durable effect and the answer is exit 3.
fn primary_advanced_continues(verb: Verb) -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let a = &p.a;
    verb.begin_published(&p, R1, R2);
    let preview = verb.prepare(a, "ok", 0);
    let operation = stopped_in_process(
        a,
        verb,
        R3,
        preview["data"]["confirmation_id"].as_str().unwrap(),
        FailurePoint::AfterContextSync,
    );
    let incoming = peer_commit(&p, "incoming.txt", b"incoming\n")?;
    poll(a, R4);
    assert_eq!(a.tip(PRIMARY)?, Some(incoming));

    let moved = verb.send(a, R3, "external_change", 3);
    assert_eq!(moved["outcome"], "blocked", "{moved}");
    assert_eq!(actions(&moved), ["request.retry", "repo.inspect"]);
    assert_eq!(moved["operation_id"], operation.as_str());
    assert_eq!(request_show(a, R3)["state"], "accepted");
    assert_eq!(merges(&a.repo()?)?, 0);

    let done = verb.send(a, R3, "ok", 0);
    assert_eq!(done["operation_id"], operation.as_str());
    finished_once(&p, verb, None)?;
    let primary = fixed(a.repo()?.refname_to_id(PRIMARY))?;
    assert!(fixed(a.repo()?.graph_descendant_of(primary, incoming))?);
    Ok(())
}

/// A parent no request record holds: a new prepare names its resume, and the
/// resume continues it with the record's expectation, in human mode; then its
/// replay, a no-op, a mismatch and an input failure (the design's matrix row
/// for `operation resume` of a lifecycle).
fn resume_with_no_record_continues(verb: Verb) -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let a = &p.a;
    verb.begin(a, R1);
    let operation = begun_outside(a, verb)?;
    let held = verb.prepare(a, "recovery_required", 3);
    assert!(held["data"].is_null(), "{held}");
    assert_eq!(actions(&held), ["operation.resume"]);
    assert_eq!(held["recovery"][0]["operation_id"], operation.as_str());

    // In human mode, then replayed in JSON: the lifecycle's data, and no
    // second effect.
    let out = run(&a.env, &resume_args(a, &operation, R2), &[]);
    assert_eq!(out.status, 0, "{}", stderr(&out));
    assert!(
        stdout(&out).starts_with(&format!("The request completed.\nrequest: {R2}\n")),
        "{}",
        stdout(&out)
    );
    assert_eq!(request_show(a, R2)["state"], "finished");
    finished_once(&p, verb, None)?;
    let opened = connections(&p);
    let done = resumed(a, &operation, R2, "ok", 0);
    assert_eq!(done["command"], "operation resume");
    assert_eq!(done["data"]["item_id"], verb.id());
    assert_eq!(done["effects"]["publication"], "published", "{done}");
    assert_eq!(connections(&p), opened);

    // A fresh resume of the finished parent is a no-op; the same request ID
    // for another operation is another request; a malformed operation ID is
    // the caller's. None of them connects or moves anything.
    let noop = resumed(a, &operation, R3, "already_applied", 0);
    assert_eq!(noop["outcome"], "noop", "{noop}");
    let other = OperationId::new().to_string();
    let mismatch = resumed(a, &other, R2, "request_mismatch", 2);
    assert!(mismatch["data"].is_null());
    a.json(
        &resume_args(a, "not-an-operation", R4),
        &[],
        "invalid_id",
        2,
    );
    assert_eq!(connections(&p), opened);
    finished_once(&p, verb, None)
}

// ------------------------------------------------------- a deleted branch

/// Republishes the item from the republish line given, prepared and then
/// confirmed under `request`, publishing the clone's context once.
fn republished(
    p: &CliPair,
    verb: Verb,
    line: &[String],
    request: &str,
) -> Result<(), FixtureError> {
    let a = &p.a;
    assert_eq!(
        line,
        [
            "item".to_owned(),
            "republish".into(),
            "--repo".into(),
            a.root.canonicalize().unwrap().to_str().unwrap().into(),
            "--id".into(),
            verb.id().into(),
            "--prepare".into(),
        ]
    );
    let preview = json_of(a, line, &["--json"], "ok", 0);
    let mut confirm: Vec<String> = line[..line.len() - 1].to_vec();
    confirm.extend([
        "--request-id".into(),
        request.into(),
        "--confirm".into(),
        preview["data"]["confirmation_id"].as_str().unwrap().into(),
    ]);
    let pushes = remote_world::accepted(&p.server);
    json_of(a, &confirm, &[], "ok", 0);
    assert_eq!(p.served(verb.branch())?, a.tip(verb.branch())?);
    assert_eq!(remote_world::accepted(&p.server), pushes + 1);
    Ok(())
}

/// The republish line a deleted branch's route prints.
fn republish_line(clone: &CliClone, verb: Verb) -> Vec<String> {
    let root = clone.root.canonicalize().unwrap();
    [
        "item",
        "republish",
        "--repo",
        root.to_str().unwrap(),
        "--id",
        verb.id(),
        "--prepare",
    ]
    .map(str::to_owned)
    .to_vec()
}

/// Decision 18 through the binary: the context's branch deleted at the
/// remote stops the request, whose answer offers its cancellation. The route,
/// followed: `request cancel`, the same request (`cancelled`, the child let
/// go), `item republish`, the same request (`ok`, one merge). The human
/// answer prints those four lines, with the note on the reservation, and they
/// are what is run.
fn deleted_branch_request_route(verb: Verb) -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let a = &p.a;
    verb.begin_published(&p, R1, R2);
    let preview = verb.prepare(a, "ok", 0);
    fixed(fixed(p.bare()?.find_reference(verb.branch()))?.delete())?;

    let mut args = verb.args(a);
    args.extend([
        "--request-id",
        R3,
        "--confirm",
        preview["data"]["confirmation_id"].as_str().unwrap(),
    ]);
    let out = run(&a.env, &args, &[]);
    assert_eq!(out.status, 3, "{}", stderr(&out));
    let text = stderr(&out);
    assert!(text.contains("code: remote_branch_deleted\n"), "{text}");
    let root = a.root.canonicalize().unwrap();
    let [resource, name] = verb.words();
    let again: Vec<String> = [
        resource,
        name,
        "--repo",
        root.to_str().unwrap(),
        "--id",
        verb.id(),
        "--request-id",
        R3,
    ]
    .map(str::to_owned)
    .to_vec();
    let route = vec![
        ["request", "cancel", "--request-id", R3]
            .map(str::to_owned)
            .to_vec(),
        again.clone(),
        republish_line(a, verb),
        again,
    ];
    assert_eq!(commands(text), route, "{text}");
    assert!(
        text.contains("note: the refused request's context synchronization keeps its reservation"),
        "{text}"
    );
    let stopped = verb.send(a, R3, "remote_branch_deleted", 3);
    assert_eq!(stopped["data"]["item_id"], verb.id(), "{stopped}");
    assert_eq!(actions(&stopped), ["request.cancel", "repo.inspect"]);
    assert_eq!(request_show(a, R3)["state"], "accepted");
    assert_eq!(merges(&a.repo()?)?, 0);
    let pushes = remote_world::accepted(&p.server);

    let asked = json_of(a, &route[0], &["--json"], "ok", 0);
    assert_eq!(asked["data"]["requested"], true, "{asked}");
    let cancelled = lifecycle_data(json_of(a, &route[1], &["--json"], "cancelled", 130));
    assert_eq!(cancelled["outcome"], "cancelled");
    assert_eq!(remote_world::accepted(&p.server), pushes, "nothing pushed");
    assert_eq!(merges(&a.repo()?)?, 0);
    republished(&p, verb, &route[2], R4)?;
    let done = lifecycle_data(json_of(a, &route[3], &["--json"], "ok", 0));
    assert_eq!(done["effects"]["publication"], "published", "{done}");
    assert_eq!(request_show(a, R3)["state"], "finished");
    finished_once(&p, verb, None)
}

/// Decision 19 through the binary: a parent no request holds meets the
/// deleted branch under `operation resume`, which lets the child go and keeps
/// no record. The route, followed: `item republish`, then the resume again
/// (`ok`, one merge). The human answer prints those two lines and the
/// resume's own note, never C4's comment note, and they are what is run.
fn deleted_branch_resume_route(verb: Verb) -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let a = &p.a;
    verb.begin_published(&p, R1, R2);
    let operation = begun_outside(a, verb)?;
    fixed(fixed(p.bare()?.find_reference(verb.branch()))?.delete())?;

    let out = run(&a.env, &resume_args(a, &operation, R3), &[]);
    assert_eq!(out.status, 3, "{}", stderr(&out));
    let text = stderr(&out);
    assert!(text.contains("code: remote_branch_deleted\n"), "{text}");
    let root = a.root.canonicalize().unwrap();
    let route = vec![
        republish_line(a, verb),
        [
            "operation",
            "resume",
            "--repo",
            root.to_str().unwrap(),
            "--operation-id",
            operation.as_str(),
        ]
        .map(str::to_owned)
        .to_vec(),
    ];
    assert_eq!(commands(text), route, "{text}");
    assert!(
        text.contains("note: this resume released the operation's context synchronization"),
        "{text}"
    );
    assert!(!text.contains("comment add or item sync"), "{text}");
    checked(
        &a.env,
        &["request", "show", "--request-id", R3],
        &[],
        "request_not_found",
        2,
        None,
    );
    let refused = resumed(a, &operation, R4, "remote_branch_deleted", 3);
    assert_eq!(actions(&refused), ["operation.resume", "repo.inspect"]);
    assert_eq!(refused["recovery"][0]["operation_id"], operation.as_str());
    assert_eq!(merges(&a.repo()?)?, 0);

    republished(&p, verb, &route[0], R5)?;
    let done = lifecycle_data(json_of(
        a,
        &route[1],
        &["--request-id", R6, "--json"],
        "ok",
        0,
    ));
    assert_eq!(done["effects"]["publication"], "published", "{done}");
    assert_eq!(done["data"]["item_id"], verb.id());
    finished_once(&p, verb, None)
}

// --------------------------------------------------------- host and unlock

/// The host presents another key at the context's synchronization: the stop
/// names the host, the request stands, `host replace` replaces the pin, and the
/// same request then completes. The child's synchronization is shared by both
/// verbs, so one verb (decision 27).
fn c5_lifecycle_host_stop_then_same_request_close() -> Result<(), FixtureError> {
    let verb = Verb::Close;
    let p = CliPair::new()?;
    let a = &p.a;
    verb.begin(a, R1);
    let preview = verb.prepare(a, "ok", 0);
    let old = p.server.host_identity().sha256;
    p.server.rotate_host_key()?;
    let stopped = verb.confirm(a, R2, &preview, "host_replacement_required", 3);
    assert_eq!(actions(&stopped), ["request.retry", "repo.inspect"]);
    assert_eq!(request_show(a, R2)["state"], "accepted");
    assert_eq!(merges(&a.repo()?)?, 0);

    let mut input = host_input(&p.server);
    input["old_fingerprint"] = old.into();
    let replace = prepare(&a.env, &a.root, "replace", &input);
    confirm(&a.env, &a.root, "replace", &input, &replace, "ok", 0);
    let done = verb.send(a, R2, "ok", 0);
    assert_eq!(done["operation_id"], stopped["operation_id"]);
    finished_once(&p, verb, None)
}

/// With no terminal a protected key stops the request at the context's
/// synchronization, and the request stands; past its publication, it stops
/// cleanup's deletion the same way. At a terminal the same request asks for
/// the passphrase and completes. One verb (decision 27), as above.
#[cfg(unix)]
fn c5_lifecycle_unlock_stop_then_same_request_close() -> Result<(), FixtureError> {
    use repository::{AuthoringKind, SynchronizationTarget, keys::*, transport::*};
    use support::pty::PtyCli;
    let w = remote_world::World::new()?;
    private_fixture(&w.server);
    let env = CliEnv::new(w.data());
    fixed(std::fs::write(
        env.home.path().join(".gitconfig"),
        "[core]\n\tautocrlf = false\n",
    ))?;
    let port = w.server.address().port();
    fixed(w.service.verify_ssh_transport(
        VerifySshTransportRequest {
            root: w.root.clone(),
            direction: SshDirection::Fetch,
            approval: Some(HostApproval {
                authority: SshAuthority {
                    host: "127.0.0.1".into(),
                    port,
                },
                expected: None,
                presented: w.server.host_identity(),
            }),
        },
        &mut SessionCredentials::new(remote_world::Provider),
    ))?;
    // The fixture's seed push does not move the tracking reference.
    fixed(w.sync(w.primary()))?;
    let root = w.root.to_str().unwrap();
    let created = serde_json::json!({
        "item_id": ITEM,
        "title": "Closed at a terminal",
        "type": "task",
        "status": "open",
        "body": "A ticket to close.\n",
    });
    checked(
        &env,
        &[
            "ticket",
            "create",
            "--repo",
            root,
            "--request-id",
            R1,
            "--input",
            "-",
        ],
        &fixed(serde_json::to_vec(&created))?,
        "ok",
        0,
        None,
    );
    // Published first, in this process, so the run at the terminal has the
    // fewest connections: the PTY harness waits ten seconds for its child,
    // and each connection decrypts the key again.
    fixed(w.sync(w.request(SynchronizationTarget::Context {
        kind: AuthoringKind::Ticket,
        item_id: fixed(ITEM.parse())?,
    })))?;
    let args = ["ticket", "close", "--repo", root, "--id", ITEM];
    let preview = checked(
        &env,
        &[&args[..], &["--prepare"][..]].concat(),
        &[],
        "ok",
        0,
        None,
    );
    let confirmation = preview["data"]["confirmation_id"].as_str().unwrap();
    let stopped = lifecycle_data(checked(
        &env,
        &[
            &args[..],
            &["--request-id", R2, "--confirm", confirmation][..],
        ]
        .concat(),
        &[],
        "unlock_required",
        3,
        None,
    ));
    assert_eq!(actions(&stopped)[0], "request.retry");
    let shown = checked(
        &env,
        &["request", "show", "--request-id", R2],
        &[],
        "ok",
        0,
        None,
    );
    assert_eq!(shown["data"]["state"], "accepted");
    let main = fixed(w.repo()?.refname_to_id(PRIMARY))?;

    // The PTY harness waits ten seconds for its child, and each connection
    // decrypts the key again, so the run at a terminal is left the fewest:
    // the same request goes on in this process, unlocked, and stops after its
    // publication. Without a terminal its cleanup's deletion stops on the key
    // again, past a durable effect.
    let service = support::FailOnce::at(FailurePoint::AfterPublication).open_service(&w.data());
    let published = service.execute(
        MutationCall {
            request_id: RequestId::parse(R2).unwrap(),
            confirmation: None,
            mutation: Mutation::TicketClose(TicketCloseInput {
                root: w.root.clone(),
                id: fixed(ITEM.parse())?,
            }),
        },
        &mut SessionCredentials::new(remote_world::Provider),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_ne!(published.code.as_str(), "ok");
    drop(service);
    assert_eq!(
        fixed(w.bare()?.refname_to_id(PRIMARY))?,
        fixed(w.repo()?.refname_to_id(PRIMARY))?
    );
    let again = [&args[..], &["--request-id", R2][..]].concat();
    let cleanup = lifecycle_data(checked(&env, &again, &[], "unlock_required", 4, None));
    assert_eq!(cleanup["outcome"], "partial");
    assert_eq!(actions(&cleanup)[0], "request.retry");
    assert_eq!(cleanup["effects"]["cleanup"], "pending");
    assert!(w.bare()?.refname_to_id(CONTEXT).is_ok(), "the branch stays");

    let mut pty = PtyCli::spawn(&again.iter().map(OsStr::new).collect::<Vec<_>>(), &env, b"");
    pty.expect("Passphrase for key");
    pty.send(format!("{}\n", remote_world::PASSWORD).as_bytes());
    let (out, transcript) = pty.finish();
    scan_output(&out);
    absent(transcript.as_bytes(), remote_world::PASSWORD);
    assert_eq!(out.status, 0, "{}", stderr(&out));
    assert!(
        stdout(&out).starts_with("The request completed.\n"),
        "{}",
        stdout(&out)
    );

    let repository = w.repo()?;
    assert_eq!(merges(&repository)?, 1);
    assert_ne!(fixed(repository.refname_to_id(PRIMARY))?, main);
    assert_eq!(
        fixed(w.bare()?.refname_to_id(PRIMARY))?,
        fixed(repository.refname_to_id(PRIMARY))?
    );
    assert_eq!(closure_checkpoints(&repository)?, 1);
    let done = lifecycle_data(checked(&env, &again, &[], "ok", 0, None));
    assert_eq!(done["operation_id"], stopped["operation_id"]);
    Ok(())
}

// --------------------------------------------- the refused checkpoint merge

/// The local recovery over SSH: a closure stopped past its checkpoint (in
/// this process), whose ticket a peer then moves on the remote's primary, and
/// a poll brings in. The same request's merge of the checkpoint is refused,
/// and the four `next:` lines it prints are followed through the binary, with
/// only their placeholders filled, with no Git step. The same request then
/// finishes: published, the closure integrated once, the checkpoint's
/// `closed_at` and `closed_by` kept. `tests/cli/local_sync.rs` proves the same
/// route with no remote.
fn c5_lifecycle_refused_checkpoint_merge_repaired_close() -> Result<(), FixtureError> {
    let verb = Verb::Close;
    let p = CliPair::new()?;
    let a = &p.a;
    verb.begin(a, R1);
    let preview = verb.prepare(a, "ok", 0);
    let operation = stopped_in_process(
        a,
        verb,
        R2,
        preview["data"]["confirmation_id"].as_str().unwrap(),
        FailurePoint::AfterClosureCheckpoint,
    );
    let checkpoint = closure_fields(&fixed(std::fs::read_to_string(
        verb.worktree(a).join(verb.path()),
    ))?);
    assert_eq!(checkpoint.len(), 2, "{checkpoint:?}");
    let moved = format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {ITEM}\ntitle: Moved on primary\n\
         type: task\nstatus: open\n---\nBody moved on primary too.\n"
    );
    let incoming = peer_commit(&p, &verb.path(), moved.as_bytes())?;
    poll(a, R3);
    assert_eq!(a.tip(PRIMARY)?, Some(incoming));

    let refused = run(
        &a.env,
        &[&verb.args(a)[..], &["--request-id", R2][..]].concat(),
        &[],
    );
    assert_eq!(refused.status, 4, "{}", stderr(&refused));
    let lines: Vec<Vec<String>> = commands(stderr(&refused))
        .into_iter()
        .filter(|words| words[0] == "context")
        .collect();
    assert_eq!(lines.len(), 4, "{}", stderr(&refused));
    let fill = |words: &[String], values: &[(&str, &str)]| -> Vec<String> {
        words
            .iter()
            .map(|word| {
                values
                    .iter()
                    .find(|(placeholder, _)| word == placeholder)
                    .map_or_else(|| word.clone(), |(_, value)| (*value).to_owned())
            })
            .collect()
    };
    let human = |words: Vec<String>, input: &[u8]| {
        let args: Vec<&str> = words.iter().map(String::as_str).collect();
        run(&a.env, &args, input)
    };
    assert_eq!(merges(&a.repo()?)?, 0);

    // 1. Observe.
    let observed = human(lines[0].clone(), &[]);
    assert_eq!(observed.status, 0, "{}", stderr(&observed));
    let observation = value_after(stdout(&observed), "Observation").to_owned();
    // 2. Synchronize locally, with the observation: a conflict.
    let synced = human(
        fill(&lines[1], &[("<new-request-id>", R4)]),
        serde_json::json!({ "observation": observation })
            .to_string()
            .as_bytes(),
    );
    assert_eq!(synced.status, 4, "{}", stderr(&synced));
    let sync = value_after(stderr(&synced), "Operation id").to_owned();
    // 3. Inspect it, and read the sides through --json.
    let shown = human(fill(&lines[2], &[("<sync-operation-id>", &sync)]), &[]);
    assert_eq!(shown.status, 0, "{}", stderr(&shown));
    let resolution = value_after(stdout(&shown), "Observation").to_owned();
    let token = stdout(&shown)
        .lines()
        .find_map(|line| {
            line.split_whitespace()
                .find_map(|word| word.strip_prefix("token="))
        })
        .unwrap()
        .to_owned();
    let sides = json_of(
        a,
        &fill(&lines[2], &[("<sync-operation-id>", &sync)]),
        &["--token", &token, "--json"],
        "ok",
        0,
    );
    let local = sides["data"]["local"].as_str().unwrap().to_owned();
    for line in &checkpoint {
        assert!(local.contains(line.as_str()), "{local}");
    }
    // 4. Resolve, keeping the closure.
    let body = serde_json::json!({
        "observation": resolution,
        "resolutions": [{ "token": token, "source": local }],
    });
    let resolved = human(
        fill(
            &lines[3],
            &[("<sync-operation-id>", &sync), ("<new-request-id>", R5)],
        ),
        body.to_string().as_bytes(),
    );
    assert_eq!(resolved.status, 0, "{}", stderr(&resolved));
    let pushes = remote_world::accepted(&p.server);
    assert_eq!(p.served(PRIMARY)?, Some(incoming), "nothing published yet");

    // The same request finishes, and the close merges once.
    let done = verb.send(a, R2, "ok", 0);
    assert_eq!(done["operation_id"], operation.as_str());
    assert_eq!(done["data"]["integration"], "merged", "{done}");
    assert_eq!(done["effects"]["publication"], "published", "{done}");
    assert!(remote_world::accepted(&p.server) > pushes);
    finished_once(&p, verb, Some(&checkpoint))?;
    let repository = a.repo()?;
    let mut walk = fixed(repository.revwalk())?;
    fixed(walk.push(fixed(repository.refname_to_id(PRIMARY))?))?;
    let mut subjects = Vec::new();
    for oid in walk {
        let commit = fixed(repository.find_commit(fixed(oid)?))?;
        if commit.parent_count() == 2 {
            subjects.push(commit.summary().unwrap_or_default().to_owned());
        }
    }
    assert_eq!(
        subjects,
        [
            format!("Integrate closed ticket {ITEM}"),
            "Resolve local context conflicts".to_owned(),
        ],
        "the close merges once; the resolution's merge is the repair"
    );
    assert_eq!(closure_at(&p.bare()?, PRIMARY)?, checkpoint);
    Ok(())
}

// ------------------------------------------------------- the recovery walk

/// The repair's four `context` lines a human answer prints, followed with
/// only their placeholders filled, as `c5_lifecycle_refused_checkpoint_merge_repaired_close`
/// follows them: observe, synchronize (a conflict), show, and resolve keeping
/// the local side. Returns the local side the resolution kept.
fn repaired(clone: &CliClone, lines: &[Vec<String>], first: &str, second: &str) -> String {
    let fill = |words: &[String], values: &[(&str, &str)]| -> Vec<String> {
        words
            .iter()
            .map(|word| {
                values
                    .iter()
                    .find(|(placeholder, _)| word == placeholder)
                    .map_or_else(|| word.clone(), |(_, value)| (*value).to_owned())
            })
            .collect()
    };
    let human = |words: Vec<String>, input: &[u8]| {
        let args: Vec<&str> = words.iter().map(String::as_str).collect();
        run(&clone.env, &args, input)
    };
    assert_eq!(lines.len(), 4, "{lines:?}");
    let observed = human(lines[0].clone(), &[]);
    assert_eq!(observed.status, 0, "{}", stderr(&observed));
    let observation = value_after(stdout(&observed), "Observation").to_owned();
    let synced = human(
        fill(&lines[1], &[("<new-request-id>", first)]),
        serde_json::json!({ "observation": observation })
            .to_string()
            .as_bytes(),
    );
    assert_eq!(synced.status, 4, "{}", stderr(&synced));
    let sync = value_after(stderr(&synced), "Operation id").to_owned();
    let show = fill(&lines[2], &[("<sync-operation-id>", &sync)]);
    let shown = human(show.clone(), &[]);
    assert_eq!(shown.status, 0, "{}", stderr(&shown));
    let resolution = value_after(stdout(&shown), "Observation").to_owned();
    let token = stdout(&shown)
        .lines()
        .find_map(|line| {
            line.split_whitespace()
                .find_map(|word| word.strip_prefix("token="))
        })
        .unwrap()
        .to_owned();
    let sides = json_of(clone, &show, &["--token", &token, "--json"], "ok", 0);
    let local = sides["data"]["local"].as_str().unwrap().to_owned();
    let body = serde_json::json!({
        "observation": resolution,
        "resolutions": [{ "token": token, "source": local }],
    });
    let resolved = human(
        fill(
            &lines[3],
            &[("<sync-operation-id>", &sync), ("<new-request-id>", second)],
        ),
        body.to_string().as_bytes(),
    );
    assert_eq!(resolved.status, 0, "{}", stderr(&resolved));
    local
}

/// A peer's retitling of the ticket on the remote's primary, with its body
/// moved too, polled into A's primary: it conflicts with A's context.
fn ticket_moved_on_primary(p: &CliPair, request: &str) -> Result<git2::Oid, FixtureError> {
    let moved = format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {ITEM}\ntitle: Moved on primary\n\
         type: task\nstatus: open\n---\nBody moved on primary too.\n"
    );
    let incoming = peer_commit(p, &Verb::Close.path(), moved.as_bytes())?;
    poll(&p.a, request);
    assert_eq!(p.a.tip(PRIMARY)?, Some(incoming));
    Ok(incoming)
}

/// The `next:` lines a blocked prepare prints, in human mode, and its note.
fn prepare_lines(clone: &CliClone, verb: Verb) -> (Vec<Vec<String>>, String) {
    let out = run(
        &clone.env,
        &[&verb.args(clone)[..], &["--prepare"][..]].concat(),
        &[],
    );
    assert_eq!(out.status, 3, "{}", stderr(&out));
    (commands(stderr(&out)), stderr(&out).to_owned())
}

/// A prepare blocked by `integration_conflict` prints the four `context`
/// lines and "Then prepare again" (Task 7's review). Followed through the
/// binary over SSH: the repair, then a prepare that is ready, and a confirm
/// that closes once, keeping the context's side. One verb: a promotion's
/// document never reached primary, so its conflict there is an add/add,
/// external only (decision 30), and the same lines lead to the external repair.
fn c5_lifecycle_walk_prepare_conflict_close() -> Result<(), FixtureError> {
    let verb = Verb::Close;
    let p = CliPair::new()?;
    let a = &p.a;
    verb.begin(a, R1);
    ticket_moved_on_primary(&p, R2)?;
    let (lines, text) = prepare_lines(a, verb);
    assert!(text.contains("Blockers: integration_conflict\n"), "{text}");
    assert!(text.contains("Then prepare again."), "{text}");
    let local = repaired(a, &lines, R3, R4);
    assert!(local.contains("Saved for its lifecycle."), "{local}");
    let preview = verb.prepare(a, "ok", 0);
    let done = verb.confirm(a, R5, &preview, "ok", 0);
    assert_eq!(done["effects"]["publication"], "published", "{done}");
    finished_once(&p, verb, None)?;
    let primary = fixed(a.repo()?.refname_to_id(PRIMARY))?;
    let source = source_at(&a.repo()?, primary, &verb.path())?.ok_or(FixtureError)?;
    assert!(source.contains("Saved for its lifecycle."), "{source}");
    Ok(())
}

/// Under `operation resume`, a closure's refused checkpoint merge prints the
/// four `context` lines and the resume (Task 7's review, item 1). The closure
/// no request holds is stopped past its checkpoint in this process; a peer
/// then moves the ticket. Followed through the binary: the repair, then the
/// resume line, which merges once and keeps the checkpoint's fields.
fn c5_lifecycle_walk_refused_merge_under_resume_close() -> Result<(), FixtureError> {
    let verb = Verb::Close;
    let p = CliPair::new()?;
    let a = &p.a;
    verb.begin(a, R1);
    let operation = begun_outside_at(a, FailurePoint::AfterClosureCheckpoint)?;
    let checkpoint = closure_fields(&fixed(std::fs::read_to_string(
        verb.worktree(a).join(verb.path()),
    ))?);
    assert_eq!(checkpoint.len(), 2, "{checkpoint:?}");
    ticket_moved_on_primary(&p, R2)?;

    let refused = run(&a.env, &resume_args(a, &operation, R3), &[]);
    assert_eq!(refused.status, 4, "{}", stderr(&refused));
    let lines = commands(stderr(&refused));
    assert_eq!(lines.len(), 5, "{}", stderr(&refused));
    assert!(
        stderr(&refused).contains("Then resume it again"),
        "{}",
        stderr(&refused)
    );
    repaired(a, &lines[..4], R4, R5);
    let resume: Vec<&str> = lines[4].iter().map(String::as_str).collect();
    assert_eq!(&resume[..2], ["operation", "resume"]);
    let done = lifecycle_data(json_of(
        a,
        &lines[4],
        &["--request-id", R6, "--json"],
        "ok",
        0,
    ));
    assert_eq!(done["data"]["integration"], "merged", "{done}");
    assert_eq!(done["effects"]["publication"], "published", "{done}");
    finished_once(&p, verb, Some(&checkpoint))
}

/// `begun_outside`, stopped at `point` in place of the first safe point.
fn begun_outside_at(clone: &CliClone, point: FailurePoint) -> Result<String, FixtureError> {
    let service: RepositoryService = support::FailOnce::at(point).open_service(&clone.env.data);
    let operation_id = OperationId::new();
    let item_id: manyhands::canonical::ItemId = fixed(ITEM.parse())?;
    let TicketClosurePreflight::Ready(preview) =
        fixed(service.preview_ticket_closure(PreviewTicketClosureRequest {
            root: clone.root.clone(),
            item_id: item_id.clone(),
            confirmed_identity: None,
        }))?
    else {
        return Err(FixtureError);
    };
    let _stopped = service.close_ticket(
        CloseTicketRequest {
            root: clone.root.clone(),
            operation_id,
            item_id,
            expected: preview.expectation(),
            approval: None,
            confirmed_identity: None,
        },
        &mut SessionCredentials::new(remote_world::Provider),
    );
    Ok(operation_id.to_string())
}

/// `line`, a `next:` command, with each `<new-request-id>` filled by `request`,
/// in JSON mode.
fn followed(clone: &CliClone, line: &[String], request: &str, code: &str, status: i32) -> Value {
    let filled: Vec<String> = line
        .iter()
        .map(|word| {
            if word == "<new-request-id>" {
                request.to_owned()
            } else {
                word.clone()
            }
        })
        .collect();
    json_of(clone, &filled, &["--json"], code, status)
}

/// Primary holds a commit the remote lacks: the prepare is blocked by
/// `primary_ahead_of_remote` and prints `poll once`, then `repo sync`, then
/// "prepare again". Followed: the poll, the sync that publishes primary, and a
/// ready prepare that closes once. One verb: `way_out` is shared (decision 27).
fn c5_lifecycle_walk_primary_ahead_close() -> Result<(), FixtureError> {
    let verb = Verb::Close;
    let p = CliPair::new()?;
    let a = &p.a;
    verb.begin(a, R1);
    let ahead = own_commit(a, "ahead.txt", b"ahead\n")?;
    let (lines, text) = prepare_lines(a, verb);
    assert!(
        text.contains("Blockers: primary_ahead_of_remote\n"),
        "{text}"
    );
    assert!(text.contains("Then prepare again."), "{text}");
    assert_eq!(lines.len(), 2, "{text}");
    assert_eq!(&lines[0][..2], ["poll", "once"]);
    assert_eq!(&lines[1][..2], ["repo", "sync"]);
    followed(a, &lines[0], R2, "ok", 0);
    followed(a, &lines[1], R3, "ok", 0);
    assert_eq!(p.served(PRIMARY)?, Some(ahead), "primary is published");
    let preview = verb.prepare(a, "ok", 0);
    verb.confirm(a, R4, &preview, "ok", 0);
    finished_once(&p, verb, None)
}

/// No tracking ref of the remote's primary: the prepare is blocked by
/// `primary_missing` and prints `poll once`, then "prepare again". Followed:
/// the poll reads it, and the prepare is ready and closes once.
fn c5_lifecycle_walk_primary_missing_close() -> Result<(), FixtureError> {
    let verb = Verb::Close;
    let p = CliPair::new()?;
    let a = &p.a;
    verb.begin(a, R1);
    fixed(fixed(a.repo()?.find_reference("refs/remotes/origin/main"))?.delete())?;
    let (lines, text) = prepare_lines(a, verb);
    assert!(text.contains("primary_missing"), "{text}");
    assert!(text.contains("Then prepare again."), "{text}");
    assert_eq!(lines.len(), 1, "{text}");
    assert_eq!(&lines[0][..2], ["poll", "once"]);
    followed(a, &lines[0], R2, "ok", 0);
    let preview = verb.prepare(a, "ok", 0);
    verb.confirm(a, R3, &preview, "ok", 0);
    finished_once(&p, verb, None)
}

/// A ticket with no editing context: the prepare is blocked by
/// `context_missing` and prints the save that makes one. Followed with the
/// note's input (the observation `ticket show` prints, and a body): the save,
/// then a ready prepare that closes once.
fn c5_lifecycle_walk_context_missing_close() -> Result<(), FixtureError> {
    let verb = Verb::Close;
    let p = CliPair::new()?;
    let a = &p.a;
    assert_eq!(a.tip(verb.branch())?, None);
    let (lines, text) = prepare_lines(a, verb);
    assert!(text.contains("Blockers: context_missing\n"), "{text}");
    assert!(text.contains("Then prepare again."), "{text}");
    assert_eq!(lines.len(), 1, "{text}");
    assert_eq!(&lines[0][..2], ["ticket", "save"]);
    let shown = a.json(
        &["ticket", "show", "--repo", a.root(), "--id", ITEM],
        &[],
        "ok",
        0,
    );
    let input = serde_json::json!({
        "observation": shown["data"]["observation"],
        "body": "Saved from the route.\n",
    });
    let filled: Vec<String> = lines[0]
        .iter()
        .map(|word| {
            if word == "<new-request-id>" {
                R2.to_owned()
            } else {
                word.clone()
            }
        })
        .collect();
    let mut args: Vec<&str> = filled.iter().map(String::as_str).collect();
    args.push("--json");
    a.json(&args, &serde_json::to_vec(&input).unwrap(), "ok", 0);
    let preview = verb.prepare(a, "ok", 0);
    verb.confirm(a, R3, &preview, "ok", 0);
    finished_once(&p, verb, None)
}

/// Exit evidence for codes a C5 verb reaches only with the server. After a
/// close: `confirmation_used`, its spent consent under a new request; then
/// `key_rejected` from `poll once` at a server that lets the client in without
/// its key; then `poll_yielding` from `poll once` while a poll holds the
/// reservation, staged as C4's `c4_interruption_poll_holds_the_reservation`.
/// The staged reservation has no owner to finish it, so it comes last.
fn c5_lifecycle_exit_evidence_close() -> Result<(), FixtureError> {
    use repository::{
        RemoteOperationPriority, RemoteOperationTarget, RemoteRefPlan, RemoteReservationOutcome,
    };
    let verb = Verb::Close;
    let p = CliPair::new()?;
    let a = &p.a;
    verb.begin_published(&p, R1, R2);
    let preview = verb.prepare(a, "ok", 0);
    verb.confirm(a, R3, &preview, "ok", 0);
    finished_once(&p, verb, None)?;

    let used = verb.confirm(a, R4, &preview, "confirmation_used", 3);
    assert_eq!(used["outcome"], "blocked", "{used}");
    assert_eq!(merges(&a.repo()?)?, 1);

    let poll = |request: &str, code: &str, status: i32| {
        a.json(
            &["poll", "once", "--repo", a.root(), "--request-id", request],
            &[],
            code,
            status,
        )
    };
    p.server.accept_anonymous();
    poll(R5, "key_rejected", 3);

    let service = fixed(RepositoryService::open_at(&a.env.data))?;
    let plan = fixed(RemoteRefPlan::from_configuration("origin", "main"))?;
    let held = fixed(service.reserve_remote_operation_with_priority(
        &a.root,
        OperationId::new(),
        &RemoteOperationTarget::for_poll(&plan),
        RemoteOperationPriority::Poll,
    ))?;
    assert!(matches!(held, RemoteReservationOutcome::Reserved(_)));
    let opened = connections(&p);
    let yielded = poll(R6, "poll_yielding", 5);
    assert_eq!(actions(&yielded), ["request.retry"], "{yielded}");
    assert_eq!(connections(&p), opened, "no connection");
    drop((held, service));
    Ok(())
}

/// `poll once`'s `busy` route (Task 9), walked: the repository's lease held
/// outside the CLI, as `c5_polling_a_second_process_holds_the_lease` holds it.
/// The answer prints `operation list`, then `operation resume` with an
/// `<operation-id>` placeholder. Followed: the list answers, and the resume of
/// this request's own operation (from `request show`) names only this request
/// (`original_request_required`), which the note says to send again once the
/// holder lets go. Sent again, the same request polls, under its operation.
/// Named under `c5_lifecycle_walk` with Task 13b's walks.
fn c5_lifecycle_walk_poll_busy_route() -> Result<(), FixtureError> {
    use fs4::fs_std::FileExt;
    let p = CliPair::new()?;
    let a = &p.a;
    let incoming = peer_commit(&p, "incoming.txt", b"incoming\n")?;
    let lease = fixed(
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(a.repo()?.commondir().join("manyhands-operation.lock")),
    )?;
    assert!(fixed(FileExt::try_lock_exclusive(&lease))?);
    let out = run(
        &a.env,
        &["poll", "once", "--repo", a.root(), "--request-id", R1],
        &[],
    );
    assert_eq!(out.status, 5, "{}", stderr(&out));
    let lines = commands(stderr(&out));
    assert_eq!(lines.len(), 2, "{}", stderr(&out));
    assert_eq!(&lines[0][..2], ["operation", "list"]);
    assert_eq!(&lines[1][..2], ["operation", "resume"]);
    let own = request_show(a, R1);
    assert_eq!(own["state"], "accepted", "{own}");
    let operation = own["operations"][0]["operation_id"]
        .as_str()
        .unwrap()
        .to_owned();

    // The list answers; what it names is not asserted. Here the holder is a
    // lease outside any operation, so it names nothing (ledger, Task 13b).
    json_of(a, &lines[0], &["--json"], "ok", 0);
    let resume: Vec<String> = lines[1]
        .iter()
        .map(|word| {
            if word == "<operation-id>" {
                operation.clone()
            } else {
                word.clone()
            }
        })
        .collect();
    let named = json_of(
        a,
        &resume,
        &["--request-id", R2, "--json"],
        "original_request_required",
        3,
    );
    assert!(
        named["recovery"][0].to_string().contains(R1),
        "names this request: {named}"
    );

    fixed(FileExt::unlock(&lease))?;
    let polled = a.json(
        &["poll", "once", "--repo", a.root(), "--request-id", R1],
        &[],
        "ok",
        0,
    );
    assert_eq!(polled["operation_id"], operation.as_str());
    assert_eq!(a.tip(PRIMARY)?, Some(incoming));
    Ok(())
}
