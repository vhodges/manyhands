//! Two independent authenticated clones on one shared branch: divergence,
//! conflict inspection, canonical resolution, external repair and publication.
//!
//! Every case drives the public service boundary only. Fixture Git commits
//! stand in for the user's own Git tooling; SQL triggers stand in for a process
//! that stops at one durable transition.
#![allow(clippy::result_large_err)]
#[allow(dead_code, unused_imports)]
#[path = "../src/lib.rs"]
mod production;
pub use production::*;
#[path = "support/ssh_harness.rs"]
mod ssh_harness;
#[path = "support/ssh_privacy.rs"]
mod ssh_privacy;
#[path = "support/ssh_remote.rs"]
mod ssh_remote;

use repository::{keys::*, transport::*, *};
use ssh_remote::*;
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
};

fn main() {
    // SAFETY: real main, before any fixture, runner or runtime thread exists.
    if unsafe { ssh_harness::initialize() }.is_err() {
        eprintln!("SSH merge recovery initialization failed");
        std::process::exit(1);
    }
    ssh_harness::run_with_output_privacy(CASES, OUTPUT_CONTROLS);
}

const CASES: &[ssh_harness::Case] = &[
    (
        "two_clone_clean_ticket_divergence",
        two_clone_clean_ticket_divergence,
    ),
    (
        "two_clone_clean_document_divergence",
        two_clone_clean_document_divergence,
    ),
    ("two_clone_primary_divergence", two_clone_primary_divergence),
    (
        "two_clone_context_then_primary_ordered_merges",
        two_clone_context_then_primary_ordered_merges,
    ),
    (
        "two_clone_document_conflict_resolved_and_published",
        two_clone_document_conflict_resolved_and_published,
    ),
    (
        "confirmed_identity_completes_a_divergent_primary_merge",
        confirmed_identity_completes_a_divergent_primary_merge,
    ),
    (
        "confirmed_identity_completes_a_canonical_resolution",
        confirmed_identity_completes_a_canonical_resolution,
    ),
    (
        "two_clone_ticket_conflict_stale_and_different_resolutions",
        two_clone_ticket_conflict_stale_and_different_resolutions,
    ),
    (
        "two_clone_comment_and_multi_path_conflicts",
        two_clone_comment_and_multi_path_conflicts,
    ),
    (
        "two_clone_context_merged_then_primary_conflicted",
        two_clone_context_merged_then_primary_conflicted,
    ),
    (
        "code_conflict_false_repairs_then_external_repair",
        code_conflict_false_repairs_then_external_repair,
    ),
    (
        "binary_delete_and_rename_conflicts_externally_repaired",
        binary_delete_and_rename_conflicts_externally_repaired,
    ),
    (
        "symlink_and_mixed_conflicts_externally_repaired",
        symlink_and_mixed_conflicts_externally_repaired,
    ),
    (
        "resolved_merge_receive_race_keeps_one_candidate",
        resolved_merge_receive_race_keeps_one_candidate,
    ),
    (
        "resolved_merge_ambiguous_acceptance_and_persistence",
        resolved_merge_ambiguous_acceptance_and_persistence,
    ),
    (
        "resolved_merge_distinct_push_remote",
        resolved_merge_distinct_push_remote,
    ),
    (
        "resolved_context_merge_deleted_context_is_not_recreated",
        resolved_context_merge_deleted_context_is_not_recreated,
    ),
    (
        "endpoint_change_after_conflict_interaction_fences_restart",
        endpoint_change_after_conflict_interaction_fences_restart,
    ),
    (
        "primary_resolution_and_publication_seams_inventory",
        primary_resolution_and_publication_seams_inventory,
    ),
    (
        "context_resolution_and_publication_seams_inventory",
        context_resolution_and_publication_seams_inventory,
    ),
    (
        "context_candidate_race_continuation_stop",
        context_candidate_race_continuation_stop,
    ),
    #[cfg(unix)]
    (
        "hostile_conflict_paths_are_redacted",
        hostile_conflict_paths_are_redacted,
    ),
    (
        "cancelled_pending_conflict_stays_recoverable",
        cancelled_pending_conflict_stays_recoverable,
    ),
    ("recovery_privacy_canaries", recovery_privacy_canaries),
    (
        "recovery_capture_fails_closed",
        recovery_capture_fails_closed,
    ),
];

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const COMMENT: &str = "01CRZ3NDEKTSV4RRFFQ69G5FAV";
const NEW_COMMENT: &str = "01DRZ3NDEKTSV4RRFFQ69G5FAV";
const MAIN: &str = "refs/heads/main";
const TICKET_REF: &str = "refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAW";
const DOCUMENT_REF: &str = "refs/heads/manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAV";
const TICKET_PATH: &str = ".manyhands/tickets/01ARZ3NDEKTSV4RRFFQ69G5FAW/ticket.md";
const COMMENT_PATH: &str =
    ".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAW/01CRZ3NDEKTSV4RRFFQ69G5FAV.md";
const NEW_COMMENT_PATH: &str =
    ".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAW/01DRZ3NDEKTSV4RRFFQ69G5FAV.md";
const DOCUMENT_PATH: &str = "docs/shared.md";
const CODE_PATH: &str = "fixture.txt";
/// A non-canonical conflict path that must never be persisted or rendered.
const HOSTILE_PATH: &str = "hostile-path-canary-private.txt";
const PASSWORD: &str = "merge-recovery-passphrase-private";
const HOSTILE: &str = "server-private!!"; // receive report-status packet width: 16
/// Canonical body canaries: one per clone, one for each caller resolution.
const ALPHA: &str = "alpha-private-body-canary";
const BETA: &str = "beta-private-body-canary";
const RESOLVED: &str = "resolved-private-body-canary";
const SEED: &str = "seed-private-body-canary";
const FETCH_SENTINEL: &[u8] = b"FETCH_HEAD-preservation-sentinel";
const TICKET_TRACKING: &str = "refs/remotes/origin/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAW";
const DOCUMENT_TRACKING: &str = "refs/remotes/origin/manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAV";
const MAIN_TRACKING: &str = "refs/remotes/origin/main";
const REGULAR: u32 = 0o100644;
const LINK: u32 = 0o120000;

struct Provider;
impl SessionCredentialProvider for Provider {
    fn request_passphrase(&mut self, _: &UnlockRequest) -> PassphraseResponse {
        PassphraseResponse::Supplied(SecretPassphrase::new(PASSWORD.into()).unwrap())
    }
}
fn callbacks(fixture: &SshRemoteFixture) -> git2::RemoteCallbacks<'_> {
    let mut cb = git2::RemoteCallbacks::new();
    cb.credentials(|_, user, _| {
        git2::Cred::ssh_key(
            user.unwrap_or("fixture"),
            None,
            fixture.client_key_path(),
            Some(PASSWORD),
        )
    });
    let public = fixture.host_public_key().to_bytes().unwrap();
    cb.certificate_check(move |cert, _| {
        if cert.as_hostkey().and_then(|host| host.hostkey()) == Some(public.as_slice()) {
            Ok(git2::CertificateCheckStatus::CertificateOk)
        } else {
            Err(git2::Error::from_str("fixture host mismatch"))
        }
    });
    cb
}
fn clone_owned(fixture: &SshRemoteFixture, root: &Path) -> Result<git2::Repository, FixtureError> {
    let mut fetch = git2::FetchOptions::new();
    fetch.remote_callbacks(callbacks(fixture));
    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fetch);
    let repo = fixed(builder.clone(&fixture.url(), root))?;
    fixed(fixed(repo.config())?.set_str("user.name", "Fixture"))?;
    fixed(fixed(repo.config())?.set_str("user.email", "fixture@example.invalid"))?;
    Ok(repo)
}
/// One ordinary authenticated ref update from a clone's own Git tooling.
fn push_peer(
    repo: &git2::Repository,
    fixture: &SshRemoteFixture,
    reference: &str,
) -> Result<(), FixtureError> {
    let before = fixture.receive_updates().len();
    let receiver = fixed(git2::Repository::open_bare(fixture.repository_path()))?;
    let old = match receiver.refname_to_id(reference) {
        Ok(oid) => oid,
        Err(error) if error.code() == git2::ErrorCode::NotFound => git2::Oid::zero(),
        Err(_) => return Err(FixtureError),
    };
    let new = fixed(repo.refname_to_id(reference))?;
    if old == new {
        return Ok(());
    }
    let mut remote = fixed(repo.remote_anonymous(&fixture.url()))?;
    let mut options = git2::PushOptions::new();
    options.remote_callbacks(callbacks(fixture));
    fixed(remote.push(&[&format!("{reference}:{reference}")], Some(&mut options)))?;
    fixture.fixture_push_transport_returned()?;
    fixture.wait_for_receive_update(
        before,
        &ReceiveUpdate {
            reference: reference.into(),
            old_oid: old,
            new_oid: new,
            accepted: true,
        },
    )
}
/// A clone's own Git tooling catches up with the published primary branch.
fn pull_peer(
    repo: &git2::Repository,
    fixture: &SshRemoteFixture,
) -> Result<git2::Oid, FixtureError> {
    let mut remote = fixed(repo.remote_anonymous(&fixture.url()))?;
    let mut fetch = git2::FetchOptions::new();
    fetch.remote_callbacks(callbacks(fixture));
    fixed(remote.fetch(
        &[&format!("+{MAIN}:refs/fixture/peer-main")],
        Some(&mut fetch),
        None,
    ))?;
    let oid = fixed(repo.refname_to_id("refs/fixture/peer-main"))?;
    install(repo, MAIN, oid)?;
    Ok(oid)
}
type Change<'a> = (&'a str, Option<(&'a [u8], u32)>);
/// A tree derived from `base` with nested path additions, replacements, mode
/// changes and deletions, written without touching any worktree or index file.
fn tree_with<'r>(
    repo: &'r git2::Repository,
    base: &git2::Tree<'_>,
    changes: &[Change<'_>],
) -> Result<git2::Tree<'r>, FixtureError> {
    let mut index = fixed(git2::Index::new())?;
    fixed(index.read_tree(base))?;
    for (path, change) in changes {
        match change {
            None => fixed(index.remove(Path::new(path), 0))?,
            Some((bytes, mode)) => fixed(index.add(&git2::IndexEntry {
                ctime: git2::IndexTime::new(0, 0),
                mtime: git2::IndexTime::new(0, 0),
                dev: 0,
                ino: 0,
                mode: *mode,
                uid: 0,
                gid: 0,
                file_size: bytes.len() as u32,
                id: fixed(repo.blob(bytes))?,
                flags: 0,
                flags_extended: 0,
                path: path.as_bytes().to_vec(),
            }))?,
        }
    }
    fixed(repo.find_tree(fixed(index.write_tree_to(repo))?))
}
/// One fixture commit object; no ref, index or worktree is touched.
fn commit_object(
    repo: &git2::Repository,
    parents: &[git2::Oid],
    changes: &[Change<'_>],
) -> Result<git2::Oid, FixtureError> {
    let parents = parents
        .iter()
        .map(|oid| fixed(repo.find_commit(*oid)))
        .collect::<Result<Vec<_>, _>>()?;
    let tree = tree_with(repo, &fixed(parents[0].tree())?, changes)?;
    let sig = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
    fixed(repo.commit(
        None,
        &sig,
        &sig,
        "owned fixture commit",
        &tree,
        &parents.iter().collect::<Vec<_>>(),
    ))
}
/// Move a checked-out branch to a descendant the way ordinary Git does.
fn install(repo: &git2::Repository, reference: &str, oid: git2::Oid) -> Result<(), FixtureError> {
    fixed(repo.checkout_tree(
        &fixed(repo.find_object(oid, None))?,
        Some(git2::build::CheckoutBuilder::new().safe()),
    ))?;
    fixed(repo.reference(reference, oid, true, "owned fixture advance"))?;
    Ok(())
}
/// One user commit on a checked-out branch made with the clone's own tooling.
fn advance(
    repo: &git2::Repository,
    reference: &str,
    changes: &[Change<'_>],
) -> Result<git2::Oid, FixtureError> {
    let old = fixed(repo.refname_to_id(reference))?;
    let new = commit_object(repo, &[old], changes)?;
    install(repo, reference, new)?;
    Ok(new)
}
fn parents(repo: &git2::Repository, oid: git2::Oid) -> Result<Vec<git2::Oid>, FixtureError> {
    Ok(fixed(repo.find_commit(oid))?.parent_ids().collect())
}
fn blob_at(
    repo: &git2::Repository,
    commit: git2::Oid,
    path: &str,
) -> Result<Option<Vec<u8>>, FixtureError> {
    let tree = fixed(fixed(repo.find_commit(commit))?.tree())?;
    match tree.get_path(Path::new(path)) {
        Ok(entry) => Ok(Some(fixed(repo.find_blob(entry.id()))?.content().to_vec())),
        Err(error) if error.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(_) => Err(FixtureError),
    }
}
fn commit_inventory(repo: &git2::Repository) -> Result<Vec<git2::Oid>, FixtureError> {
    let odb = fixed(repo.odb())?;
    let mut ids = Vec::new();
    fixed(odb.foreach(|oid| {
        ids.push(*oid);
        true
    }))?;
    let mut commits = Vec::new();
    for id in ids {
        if fixed(odb.read(id))?.kind() == git2::ObjectType::Commit {
            commits.push(id);
        }
    }
    commits.sort();
    assert!(!commits.is_empty());
    Ok(commits)
}
fn merge_commits(repo: &git2::Repository) -> Result<usize, FixtureError> {
    let mut count = 0;
    for oid in commit_inventory(repo)? {
        if fixed(repo.find_commit(oid))?.parent_count() == 2 {
            count += 1;
        }
    }
    Ok(count)
}
/// Every ref name with its target, sorted: the exact ref scope of a repository.
fn refs(repo: &git2::Repository) -> Result<Vec<(String, Option<git2::Oid>)>, FixtureError> {
    let mut all = fixed(repo.references())?
        .map(|reference| {
            let reference = fixed(reference)?;
            Ok((
                fixed(reference.name().ok_or(FixtureError))?.to_owned(),
                reference.target(),
            ))
        })
        .collect::<Result<Vec<_>, FixtureError>>()?;
    all.sort();
    Ok(all)
}
/// Plant a FETCH_HEAD sentinel and an unrelated tracking ref, then record
/// every local ref.
fn scope_before(repo: &git2::Repository) -> Result<Vec<(String, Option<git2::Oid>)>, FixtureError> {
    fixed(std::fs::write(
        repo.path().join("FETCH_HEAD"),
        FETCH_SENTINEL,
    ))?;
    let head = fixed(repo.refname_to_id(MAIN))?;
    fixed(repo.reference("refs/remotes/origin/unrelated", head, true, "fixture"))?;
    refs(repo)
}
/// Exactly the named refs moved or appeared. FETCH_HEAD is untouched and none
/// was written under a linked worktree's own gitdir.
fn scope_after(
    repo: &git2::Repository,
    before: &[(String, Option<git2::Oid>)],
    moved: &[(&str, git2::Oid)],
    linked: Option<&git2::Repository>,
) -> Result<(), FixtureError> {
    let mut expected = before.to_vec();
    for (name, oid) in moved {
        match expected.iter_mut().find(|(known, _)| known == name) {
            Some(entry) => entry.1 = Some(*oid),
            None => expected.push(((*name).to_owned(), Some(*oid))),
        }
    }
    expected.sort();
    assert_eq!(refs(repo)?, expected);
    assert_eq!(
        fixed(std::fs::read(repo.path().join("FETCH_HEAD")))?,
        FETCH_SENTINEL
    );
    if let Some(linked) = linked {
        assert!(linked.path() != repo.path());
        assert!(!linked.path().join("FETCH_HEAD").exists());
    }
    Ok(())
}
fn merge_heads(root: &Path) -> Result<Vec<git2::Oid>, FixtureError> {
    let mut repo = fixed(git2::Repository::open(root))?;
    let mut heads = Vec::new();
    match repo.mergehead_foreach(|oid| {
        heads.push(*oid);
        true
    }) {
        Ok(()) => Ok(heads),
        Err(error) if error.code() == git2::ErrorCode::NotFound => Ok(Vec::new()),
        Err(_) => Err(FixtureError),
    }
}
/// The three index stages of one conflicted path as blob OIDs.
fn stages(root: &Path, path: &str) -> Result<[Option<git2::Oid>; 3], FixtureError> {
    let index = fixed(fixed(git2::Repository::open(root))?.index())?;
    Ok([1, 2, 3].map(|stage| index.get_path(Path::new(path), stage).map(|entry| entry.id)))
}
/// Refresh the on-disk Git index after a save. Known gap, tracked separately:
/// a normal save leaves the index stale and synchronization then refuses the
/// worktree as not clean.
fn refresh_index(worktree: &Path) -> Result<(), FixtureError> {
    let repo = fixed(git2::Repository::open(worktree))?;
    let mut index = fixed(repo.index())?;
    fixed(index.read_tree(&fixed(fixed(repo.head())?.peel_to_tree())?))?;
    fixed(index.write())
}
// Fixed-size hashes avoid ever rendering snapshots containing private bytes/paths.
fn physical(root: &Path, worktree: &Path) -> Result<[u8; 32], FixtureError> {
    let repo = fixed(git2::Repository::open(root))?;
    let mut hash = blake3::Hasher::new();
    for (name, target) in refs(&repo)? {
        if name.starts_with("refs/heads/") {
            hash.update(format!("{name} {target:?}").as_bytes());
        }
    }
    hash.update(&checkout(worktree)?);
    Ok(*hash.finalize().as_bytes())
}
/// HEAD, index, merge metadata and every worktree byte of one checkout.
fn checkout(worktree: &Path) -> Result<[u8; 32], FixtureError> {
    let linked = fixed(git2::Repository::open(worktree))?;
    let mut hash = blake3::Hasher::new();
    for name in ["HEAD", "index"] {
        hash.update(&fixed(std::fs::read(linked.path().join(name)))?);
    }
    for name in ["MERGE_HEAD", "MERGE_MSG", "MERGE_MODE"] {
        hash.update(&std::fs::read(linked.path().join(name)).unwrap_or_default());
        hash.update(&[0]);
    }
    fn files(path: &Path, hash: &mut blake3::Hasher) -> Result<(), FixtureError> {
        let mut entries = fixed(std::fs::read_dir(path))?
            .map(|e| fixed(e).map(|e| e.path()))
            .collect::<Result<Vec<_>, _>>()?;
        entries.sort();
        for entry in entries {
            if entry
                .file_name()
                .is_some_and(|name| name == ".git" || name == "worktrees")
            {
                continue;
            }
            hash.update(entry.file_name().unwrap().as_encoded_bytes());
            let meta = fixed(std::fs::symlink_metadata(&entry))?;
            if meta.is_dir() {
                files(&entry, hash)?;
            } else if meta.file_type().is_symlink() {
                hash.update(
                    fixed(std::fs::read_link(&entry))?
                        .as_os_str()
                        .as_encoded_bytes(),
                );
            } else {
                hash.update(&fixed(std::fs::read(entry))?);
            }
        }
        Ok(())
    }
    files(worktree, &mut hash)?;
    Ok(*hash.finalize().as_bytes())
}
fn accepted(server: &SshRemoteFixture) -> usize {
    server
        .receive_updates()
        .iter()
        .filter(|update| update.accepted)
        .count()
}
fn one_update(
    server: &SshRemoteFixture,
    before: usize,
    reference: &str,
    old: git2::Oid,
    new: git2::Oid,
) {
    let updates = server.receive_updates();
    assert_eq!(updates.len(), before + 1);
    assert_eq!(
        updates[before],
        ReceiveUpdate {
            reference: reference.into(),
            old_oid: old,
            new_oid: new,
            accepted: true
        }
    );
}
/// Canonical sources rendered by the public serializer, so that a later
/// public save changes only the lines its draft changes.
fn ticket_source(title: &str, body: &str) -> String {
    canonical::serialize_item(&canonical::CanonicalItem::Ticket(canonical::Ticket {
        id: TICKET.parse().unwrap(),
        title: title.into(),
        ticket_type: "task".into(),
        status: "open".into(),
        project: None,
        team: None,
        closed_at: None,
        closed_by: None,
        body: body.into(),
        unknown: serde_yaml::Mapping::new(),
    }))
    .unwrap()
}
fn comment_source(body: &str) -> String {
    comment_source_created(body, 1_767_225_600)
}
fn comment_source_created(body: &str, created_at: i64) -> String {
    canonical::serialize_item(&canonical::CanonicalItem::Comment(canonical::Comment {
        id: COMMENT.parse().unwrap(),
        item_id: TICKET.parse().unwrap(),
        parent_id: None,
        created_at: time::OffsetDateTime::from_unix_timestamp(created_at).unwrap(),
        body: body.into(),
        unknown: serde_yaml::Mapping::new(),
    }))
    .unwrap()
}
fn document_source(title: &str, tail: &str) -> String {
    canonical::serialize_item(&canonical::CanonicalItem::Document(canonical::Document {
        id: DOCUMENT.parse().unwrap(),
        title: title.into(),
        body: document_body(tail),
        unknown: serde_yaml::Mapping::new(),
    }))
    .unwrap()
}
/// Long enough that a title edit and a last-line edit never share a hunk.
fn document_body(tail: &str) -> String {
    let mut body = String::new();
    for line in 1..=12 {
        body.push_str(&format!("shared document line {line}\n"));
    }
    body.push_str(tail);
    body.push('\n');
    body
}

/// One independent clone with its own service, registry and application data.
struct Side {
    service: RepositoryService,
    root: PathBuf,
    data: PathBuf,
}
impl Side {
    fn repo(&self) -> Result<git2::Repository, FixtureError> {
        fixed(git2::Repository::open(&self.root))
    }
    fn worktree(&self, item: &str) -> PathBuf {
        self.root.join(".manyhands").join("worktrees").join(item)
    }
    fn linked(&self, item: &str) -> Result<git2::Repository, FixtureError> {
        fixed(git2::Repository::open(self.worktree(item)))
    }
    fn db(&self) -> Result<rusqlite::Connection, FixtureError> {
        fixed(rusqlite::Connection::open(self.data.join(REGISTRY_FILE)))
    }
    fn reopened(&self) -> Result<RepositoryService, FixtureError> {
        fixed(RepositoryService::open_at(&self.data))
    }
}
/// A shared owned remote and two clones, A and B, that never share local state.
struct Pair {
    server: SshRemoteFixture,
    a: Side,
    b: Side,
    _directory: tempfile::TempDir,
    probes: RefCell<Vec<Vec<u8>>>,
    paths: RefCell<Vec<Vec<u8>>>,
}
impl Pair {
    fn new() -> Result<Self, FixtureError> {
        let server = SshRemoteFixture::start()?;
        let plain = fixed(std::fs::read(server.client_key_path()))?;
        let key = fixed(ssh_key::PrivateKey::read_openssh_file(
            server.client_key_path(),
        ))?;
        // One bcrypt round: every transport connection of both clones derives
        // this key again, and the derivation cost is not what these cases prove.
        let mut salt = vec![0u8; 16];
        ssh_key::rand_core::RngCore::fill_bytes(&mut ssh_key::rand_core::OsRng, &mut salt);
        let encrypted = fixed(
            fixed(key.encrypt_with(
                ssh_key::Cipher::Aes256Ctr,
                ssh_key::Kdf::Bcrypt { salt, rounds: 1 },
                ssh_key::rand_core::RngCore::next_u32(&mut ssh_key::rand_core::OsRng),
                PASSWORD,
            ))?
            .to_openssh(ssh_key::LineEnding::LF),
        )?;
        fixed(std::fs::write(
            server.client_key_path(),
            encrypted.as_bytes(),
        ))?;
        let directory = fixed(tempfile::tempdir())?;
        let mut probes = vec![
            plain.clone(),
            encrypted.as_bytes().to_vec(),
            PASSWORD.as_bytes().to_vec(),
            server.url().into_bytes(),
            HOSTILE.as_bytes().to_vec(),
            HOSTILE_PATH.as_bytes().to_vec(),
        ];
        for body in [ALPHA, BETA, RESOLVED, SEED] {
            probes.push(body.as_bytes().to_vec());
        }
        probes.extend(
            plain
                .split(|byte| *byte == b'\n')
                .filter(|line| line.len() > 40)
                .map(<[u8]>::to_vec),
        );
        ssh_privacy::save(&probes)?;

        // Clone A enables the repository, selects the publication remote and
        // seeds the shared canonical items both clones later edit.
        let root_a = directory.path().join("clone-a");
        let repo = clone_owned(&server, &root_a)?;
        let service_a = fixed(RepositoryService::open_at(&directory.path().join("data-a")))?;
        fixed(service_a.enable(EnableRepositoryRequest {
            root: root_a.clone(),
            primary_branch: "main".into(),
            identity: None,
            operation_id: OperationId::new(),
        }))?;
        let config = root_a.join(".manyhands").join("config.toml");
        let mut source = fixed(std::fs::read_to_string(&config))?;
        source.push_str("publication_remote = \"origin\"\n");
        advance(
            &repo,
            MAIN,
            &[
                (".manyhands/config.toml", Some((source.as_bytes(), REGULAR))),
                (
                    TICKET_PATH,
                    Some((ticket_source("Shared ticket", SEED).as_bytes(), REGULAR)),
                ),
                (
                    COMMENT_PATH,
                    Some((comment_source(SEED).as_bytes(), REGULAR)),
                ),
                (
                    DOCUMENT_PATH,
                    Some((document_source("Shared document", SEED).as_bytes(), REGULAR)),
                ),
                (HOSTILE_PATH, Some((b"seed code\n", REGULAR))),
            ],
        )?;
        push_peer(&repo, &server, MAIN)?;

        // Clone B is a separate clone, registry and application data directory.
        let root_b = directory.path().join("clone-b");
        clone_owned(&server, &root_b)?;
        let service_b = fixed(RepositoryService::open_at(&directory.path().join("data-b")))?;
        let enabled = fixed(service_b.enable(EnableRepositoryRequest {
            root: root_b.clone(),
            primary_branch: "main".into(),
            identity: None,
            operation_id: OperationId::new(),
        }))?;
        assert!(matches!(
            enabled,
            EnableRepositoryOutcome::AlreadyEnabled | EnableRepositoryOutcome::IndexPending(_)
        ));
        for service in [&service_a, &service_b] {
            let RegisterSharedKeyOutcome::Registered(key) =
                fixed(service.register_shared_key(RegisterSharedKeyRequest {
                    label: "acceptance-selected".into(),
                    ownership: SharedKeyOwnership::Imported,
                    private_key_path: server.client_key_path().into(),
                    public_key_path: None,
                }))?
            else {
                return Err(FixtureError);
            };
            fixed(service.select_shared_key(key.id))?;
        }
        let a = Side {
            service: service_a,
            root: root_a,
            data: directory.path().join("data-a"),
        };
        let b = Side {
            service: service_b,
            root: root_b,
            data: directory.path().join("data-b"),
        };
        let mut paths = Vec::new();
        for side in [&a, &b] {
            for item in [TICKET, DOCUMENT] {
                paths.push(side.worktree(item).to_string_lossy().as_bytes().to_vec());
            }
        }
        let pair = Self {
            server,
            a,
            b,
            _directory: directory,
            probes: RefCell::new(probes),
            paths: RefCell::new(paths),
        };
        pair.save_probes()?;
        Ok(pair)
    }
    fn bare(&self) -> Result<git2::Repository, FixtureError> {
        fixed(git2::Repository::open_bare(self.server.repository_path()))
    }
    fn save_probes(&self) -> Result<(), FixtureError> {
        let mut all = self.probes.borrow().clone();
        all.extend(self.paths.borrow().iter().cloned());
        ssh_privacy::save(&all)
    }
    fn add_probe(&self, probe: Vec<u8>) -> Result<(), FixtureError> {
        self.probes.borrow_mut().push(probe);
        self.save_probes()
    }
    /// Rendered diagnostics never carry bodies, credentials, endpoints or paths.
    fn redacted(&self, rendered: &str) -> Result<(), FixtureError> {
        ssh_privacy::clean(rendered.as_bytes(), &self.probes.borrow())?;
        ssh_privacy::clean(rendered.as_bytes(), &self.paths.borrow())
    }
    /// Every recovery row of both clones, and each whole generated store.
    fn privacy(&self) -> Result<(), FixtureError> {
        self.scanned_rows().map(|_| ())
    }
    /// The same scan, returning the scanned row count of every recovery
    /// table of clone A and clone B.
    fn scanned_rows(&self) -> Result<[std::collections::BTreeMap<String, usize>; 2], FixtureError> {
        self.save_probes()?;
        let mut counts = [
            std::collections::BTreeMap::new(),
            std::collections::BTreeMap::new(),
        ];
        let probes = self.probes.borrow();
        let paths = self.paths.borrow();
        for (side, counts) in [&self.a, &self.b].into_iter().zip(&mut counts) {
            let db = side.db()?;
            let tables = {
                let mut statement = fixed(db.prepare(
                    "SELECT name FROM sqlite_master WHERE type='table' AND (name LIKE 'remote\\_%' ESCAPE '\\' OR name='operation_records') ORDER BY name",
                ))?;
                let rows = fixed(statement.query_map([], |row| row.get::<_, String>(0)))?;
                fixed(rows.collect::<Result<Vec<_>, _>>())?
            };
            // The recovery schema is present, not silently renamed away.
            for required in [
                "operation_records",
                "remote_operation_records",
                "remote_integration_windows",
                "remote_integration_steps",
                "remote_integration_merge_metadata",
                "remote_resolution_attempts",
                "remote_resolution_paths",
                "remote_publication_attempts",
                "remote_ref_observations",
            ] {
                assert!(tables.iter().any(|table| table == required));
            }
            let mut scans = 0;
            for table in &tables {
                let mut scanned = 0;
                let mut statement = fixed(db.prepare(&format!("SELECT * FROM {table}")))?;
                let columns = statement.column_count();
                let mut rows = fixed(statement.query([]))?;
                while let Some(row) = fixed(rows.next())? {
                    for column in 0..columns {
                        if let rusqlite::types::ValueRef::Text(bytes)
                        | rusqlite::types::ValueRef::Blob(bytes) = fixed(row.get_ref(column))?
                        {
                            ssh_privacy::clean(bytes, &probes)?;
                            ssh_privacy::clean(bytes, &paths)?;
                        }
                    }
                    scans += 1;
                    scanned += 1;
                }
                counts.insert(table.clone(), scanned);
            }
            assert!(scans > 0, "privacy row inventory must be nonempty");
            // Whole stores may retain discovery worktree paths; the rows above
            // use the stronger path probes.
            let inventory = ssh_privacy::scan(&side.data, &probes)?;
            assert!(inventory.iter().any(|(path, bytes)| {
                path.file_name().is_some_and(|name| name == REGISTRY_FILE) && *bytes > 0
            }));
        }
        Ok(counts)
    }
    fn request(&self, side: &Side, target: SynchronizationTarget) -> SynchronizeRemoteRequest {
        SynchronizeRemoteRequest {
            root: side.root.clone(),
            operation_id: OperationId::new(),
            target,
            restart: false,
            confirmed_identity: None,
            approval: Some(HostApproval {
                authority: SshAuthority {
                    host: "127.0.0.1".into(),
                    port: self.server.address().port(),
                },
                expected: None,
                presented: self.server.host_identity(),
            }),
        }
    }
    fn primary(&self, side: &Side) -> SynchronizeRemoteRequest {
        self.request(side, SynchronizationTarget::Primary)
    }
    fn ticket(&self, side: &Side) -> SynchronizeRemoteRequest {
        self.request(side, ticket_target())
    }
    fn document(&self, side: &Side) -> SynchronizeRemoteRequest {
        self.request(side, document_target())
    }
    fn sync_with(
        &self,
        service: &RepositoryService,
        request: SynchronizeRemoteRequest,
    ) -> Result<SynchronizationResult, SynchronizationError> {
        let result = service.synchronize_remote(request, &mut SessionCredentials::new(Provider));
        let rendered = match &result {
            Ok(value) => format!("{value:?}"),
            Err(error) => format!("{error:?} {error}"),
        };
        self.redacted(&rendered)
            .expect("redacted synchronization output");
        self.privacy()
            .expect("redacted durable synchronization state");
        result
    }
    fn sync(
        &self,
        side: &Side,
        request: SynchronizeRemoteRequest,
    ) -> Result<SynchronizationResult, SynchronizationError> {
        self.sync_with(&side.service, request)
    }
    fn target(&self, side: &Side, kind: AuthoringKind, item: &str) -> AuthoringTarget {
        AuthoringTarget {
            root: side.root.clone(),
            kind,
            item_id: item.parse().unwrap(),
            intent: ContextIntent::Edit,
            operation_id: OperationId::new(),
        }
    }
    /// What the editor last observed: the context worktree once it exists,
    /// otherwise the primary checkout the context will be created from.
    fn observed(&self, side: &Side, item: &str, path: &str) -> ExpectedPathObservation {
        let context = side.worktree(item);
        let file = if context.exists() {
            context.join(path)
        } else {
            side.root.join(path)
        };
        match std::fs::read(file) {
            Ok(bytes) => ExpectedPathObservation::from_bytes(&bytes),
            Err(_) => ExpectedPathObservation::Missing,
        }
    }
    fn saved(&self, outcome: SaveOutcome) -> Result<git2::Oid, FixtureError> {
        let (context, checkpoint) = match outcome {
            SaveOutcome::Saved {
                context,
                checkpoint,
            }
            | SaveOutcome::IndexPending {
                context,
                checkpoint,
            } => (context, checkpoint),
            SaveOutcome::IdentityRequired { .. } => return Err(FixtureError),
        };
        self.checkpointed(&context, checkpoint)
    }
    fn checkpointed(
        &self,
        context: &ItemContext,
        checkpoint: LocalCheckpoint,
    ) -> Result<git2::Oid, FixtureError> {
        let (LocalCheckpoint::Checkpointed { commit_oid }
        | LocalCheckpoint::RefreshPending { commit_oid }) = checkpoint
        else {
            return Err(FixtureError);
        };
        refresh_index(&context.worktree)?;
        self.paths
            .borrow_mut()
            .push(context.worktree.to_string_lossy().as_bytes().to_vec());
        self.save_probes()?;
        Ok(commit_oid)
    }
    /// An ordinary public ticket save in the clone's own item context.
    fn save_ticket(&self, side: &Side, title: &str, body: &str) -> Result<git2::Oid, FixtureError> {
        let outcome = fixed(side.service.save_ticket(SaveTicketRequest {
            target: self.target(side, AuthoringKind::Ticket, TICKET),
            draft: TicketDraft {
                title: title.into(),
                body: body.into(),
                ticket_type: "task".into(),
                status: "open".into(),
                project: None,
                team: None,
            },
            expected_path: self.observed(side, TICKET, TICKET_PATH),
        }))?;
        self.saved(outcome)
    }
    /// An ordinary public document save in the clone's own item context.
    fn save_document(
        &self,
        side: &Side,
        title: &str,
        body: &str,
    ) -> Result<git2::Oid, FixtureError> {
        let observed = self.observed(side, DOCUMENT, DOCUMENT_PATH);
        let outcome = fixed(side.service.save_document(SaveDocumentRequest {
            target: self.target(side, AuthoringKind::Document, DOCUMENT),
            source_path: Some(DOCUMENT_PATH.into()),
            destination_path: DOCUMENT_PATH.into(),
            draft: DocumentDraft {
                title: title.into(),
                body: body.into(),
            },
            expected_source: Some(observed.clone()),
            expected_destination: observed,
        }))?;
        self.saved(outcome)
    }
    /// An ordinary public comment submission on the shared ticket.
    fn submit_comment(&self, side: &Side, body: &str) -> Result<git2::Oid, FixtureError> {
        let outcome = fixed(side.service.submit_comment(SubmitCommentRequest {
            target: self.target(side, AuthoringKind::Ticket, TICKET),
            comment_id: fixed(NEW_COMMENT.parse())?,
            parent_id: None,
            body: body.into(),
            expected_destination: ExpectedPathObservation::Missing,
        }))?;
        match outcome {
            CommentSubmissionOutcome::Saved {
                context,
                checkpoint,
                ..
            }
            | CommentSubmissionOutcome::IndexPending {
                context,
                checkpoint,
                ..
            } => self.checkpointed(&context, checkpoint),
            CommentSubmissionOutcome::IdentityRequired { .. } => Err(FixtureError),
        }
    }
    fn inspect(
        &self,
        side: &Side,
        operation: OperationId,
    ) -> Result<SynchronizationConflictInspection, FixtureError> {
        let inspection = fixed(
            side.service
                .inspect_synchronization_recovery(&side.root, operation),
        )?;
        self.redacted(&format!("{inspection:?}"))?;
        Ok(inspection)
    }
    /// The observed index sides of every conflict path, keyed by token order.
    fn sides(
        &self,
        side: &Side,
        inspection: &SynchronizationConflictInspection,
    ) -> Result<Vec<EphemeralSynchronizationConflictSides>, FixtureError> {
        inspection
            .paths
            .iter()
            .map(|path| {
                let sides = fixed(side.service.read_synchronization_conflict(&path.token))?;
                self.redacted(&format!("{sides:?} {:?}", path.token))?;
                Ok(sides)
            })
            .collect()
    }
    fn resolve(
        &self,
        service: &RepositoryService,
        request: ResolveSynchronizationRequest,
    ) -> Result<ResolveSynchronizationOutcome, SynchronizationError> {
        self.redacted(&format!("{request:?}"))
            .expect("redacted resolution request");
        let result = service.resolve_synchronization(request);
        let rendered = match &result {
            Ok(value) => format!("{value:?}"),
            Err(error) => format!("{error:?} {error}"),
        };
        self.redacted(&rendered)
            .expect("redacted resolution output");
        self.privacy().expect("redacted durable resolution state");
        result
    }
}
fn ticket_target() -> SynchronizationTarget {
    SynchronizationTarget::Context {
        kind: AuthoringKind::Ticket,
        item_id: TICKET.parse().unwrap(),
    }
}
fn document_target() -> SynchronizationTarget {
    SynchronizationTarget::Context {
        kind: AuthoringKind::Document,
        item_id: DOCUMENT.parse().unwrap(),
    }
}
fn restart(request: &SynchronizeRemoteRequest) -> SynchronizeRemoteRequest {
    let mut restart = request.clone();
    restart.restart = true;
    restart
}
fn published(result: SynchronizationResult) -> Result<git2::Oid, FixtureError> {
    match result {
        SynchronizationResult::Complete(SynchronizationOutcome::Published { oid, .. }) => Ok(oid),
        _ => Err(FixtureError),
    }
}
fn outcome(
    result: SynchronizationResult,
    published: bool,
    target: SynchronizationTarget,
    oid: git2::Oid,
) {
    let expected = if published {
        SynchronizationOutcome::Published { target, oid }
    } else {
        SynchronizationOutcome::AlreadyCurrent { target, oid }
    };
    assert_eq!(result, SynchronizationResult::Complete(expected));
}
/// The resolution a caller submits: the observed local side with its body and
/// title replaced, so every immutable canonical field is retained.
fn resolution_of(local: &RedactedConflictBytes, from: &str, title: &str) -> Vec<u8> {
    let text = std::str::from_utf8(local.bytes()).unwrap();
    assert!(text.contains(from));
    text.replace(from, RESOLVED)
        .replace("Beta title", title)
        .into_bytes()
}
fn resolution_request(
    side: &Side,
    operation: OperationId,
    attempt: OperationId,
    inspection: &SynchronizationConflictInspection,
    results: Vec<Vec<u8>>,
) -> ResolveSynchronizationRequest {
    ResolveSynchronizationRequest::new(
        side.root.clone(),
        operation,
        attempt,
        inspection.observation.clone(),
        inspection
            .paths
            .iter()
            .zip(results)
            .map(|(path, bytes)| (path.token.clone(), RedactedConflictBytes::from_bytes(bytes)))
            .collect(),
        None,
    )
}
/// Discovery of one item in one context worktree: its title and thread IDs.
fn discovered(
    side: &Side,
    item: &str,
) -> Result<(String, Vec<String>, Option<git2::Oid>), FixtureError> {
    let snapshot = fixed(side.service.repository_snapshot(&side.root))?;
    let worktree = fixed(std::fs::canonicalize(side.worktree(item)))?;
    let found = snapshot
        .items
        .iter()
        .find(|found| {
            found.id.to_string() == item
                && std::fs::canonicalize(&found.context).is_ok_and(|context| context == worktree)
        })
        .ok_or(FixtureError)?;
    let head = snapshot
        .contexts
        .iter()
        .find(|context| {
            std::fs::canonicalize(&context.worktree).is_ok_and(|context| context == worktree)
        })
        .and_then(|context| context.head_oid);
    Ok((
        found.title.clone(),
        found
            .comments
            .iter()
            .map(|thread| thread.id.to_string())
            .collect(),
        head,
    ))
}

/// A synchronization that stopped at a real conflict in clone B.
struct Conflict {
    request: SynchronizeRemoteRequest,
    local: git2::Oid,
    incoming: git2::Oid,
}
fn conflict_pending(
    p: &Pair,
    request: &SynchronizeRemoteRequest,
    stage: SynchronizationStage,
) -> Result<(), FixtureError> {
    let receives = p.server.receive_updates();
    let stopped = p.sync(&p.b, request.clone());
    assert!(matches!(
        stopped,
        Err(SynchronizationError::ConflictPending {
            ref target,
            operation_id,
            stage: actual,
        }) if *target == request.target && operation_id == request.operation_id && actual == stage
    ));
    // A conflict alone never reaches receive-pack.
    assert_eq!(p.server.receive_updates(), receives);
    Ok(())
}
/// Clone A publishes its ticket edit on the shared context branch; clone B
/// independently checkpoints a conflicting edit of the same ticket on the
/// same branch and stops at the conflict.
fn ticket_conflict(p: &Pair) -> Result<Conflict, FixtureError> {
    let incoming = p.save_ticket(&p.a, "Alpha title", ALPHA)?;
    assert_eq!(published(fixed(p.sync(&p.a, p.ticket(&p.a)))?)?, incoming);
    let local = p.save_ticket(&p.b, "Beta title", BETA)?;
    let request = p.ticket(&p.b);
    conflict_pending(p, &request, SynchronizationStage::Context)?;
    Ok(Conflict {
        request,
        local,
        incoming,
    })
}
/// Both clones commit a conflicting canonical edit of the ticket on the
/// primary branch with their own Git tooling; A publishes first.
fn diverge_primary_ticket(
    p: &Pair,
    a: &git2::Repository,
) -> Result<(git2::Oid, git2::Oid), FixtureError> {
    let alpha = ticket_source("Alpha title", ALPHA);
    let incoming = advance(a, MAIN, &[(TICKET_PATH, Some((alpha.as_bytes(), REGULAR)))])?;
    push_peer(a, &p.server, MAIN)?;
    let beta = ticket_source("Beta title", BETA);
    let local = advance(
        &p.b.repo()?,
        MAIN,
        &[(TICKET_PATH, Some((beta.as_bytes(), REGULAR)))],
    )?;
    Ok((local, incoming))
}
fn primary_ticket_conflict(
    p: &Pair,
    a: &git2::Repository,
    request: SynchronizeRemoteRequest,
) -> Result<Conflict, FixtureError> {
    let (local, incoming) = diverge_primary_ticket(p, a)?;
    conflict_pending(p, &request, SynchronizationStage::Primary)?;
    Ok(Conflict {
        request,
        local,
        incoming,
    })
}
/// The real Git state a pending conflict leaves in the target worktree.
fn conflict_installed(
    worktree: &Path,
    reference: &str,
    conflict: &Conflict,
    paths: &[&str],
) -> Result<(), FixtureError> {
    let repo = fixed(git2::Repository::open(worktree))?;
    assert_eq!(fixed(repo.head())?.target(), Some(conflict.local));
    assert_eq!(fixed(repo.refname_to_id(reference))?, conflict.local);
    assert!(fixed(repo.index())?.has_conflicts());
    assert_eq!(merge_heads(worktree)?, [conflict.incoming]);
    let base = fixed(repo.merge_base(conflict.local, conflict.incoming))?;
    for path in paths {
        let blob = |commit| -> Result<Option<git2::Oid>, FixtureError> {
            Ok(blob_at(&repo, commit, path)?
                .map(|bytes| git2::Oid::hash_object(git2::ObjectType::Blob, &bytes).unwrap()))
        };
        assert_eq!(
            stages(worktree, path)?,
            [blob(base)?, blob(conflict.local)?, blob(conflict.incoming)?]
        );
        let marked = fixed(std::fs::read_to_string(worktree.join(path)))?;
        assert!(marked.contains("<<<<<<<") && marked.contains(">>>>>>>"));
    }
    Ok(())
}
/// The redacted inspection of an all-canonical conflict and its observed sides.
fn inspect_canonical(
    p: &Pair,
    worktree: &Path,
    conflict: &Conflict,
    stage: SynchronizationStage,
    paths: &[&str],
) -> Result<
    (
        SynchronizationConflictInspection,
        Vec<EphemeralSynchronizationConflictSides>,
    ),
    FixtureError,
> {
    let inspection = p.inspect(&p.b, conflict.request.operation_id)?;
    assert_eq!(inspection.operation_id, conflict.request.operation_id);
    assert_eq!(inspection.target, conflict.request.target);
    assert_eq!(inspection.stage, stage);
    assert_eq!(inspection.local_parent, conflict.local);
    assert_eq!(inspection.incoming_parent, conflict.incoming);
    assert_eq!(inspection.paths.len(), paths.len());
    assert!(
        inspection
            .paths
            .iter()
            .all(|path| path.eligibility == ConflictEligibility::EligibleCanonical)
    );
    let sides = p.sides(&p.b, &inspection)?;
    let repo = fixed(git2::Repository::open(worktree))?;
    let base = fixed(repo.merge_base(conflict.local, conflict.incoming))?;
    for (sides, path) in sides.iter().zip(paths) {
        let bytes =
            |side: &Option<RedactedConflictBytes>| side.as_ref().map(|b| b.bytes().to_vec());
        assert!(bytes(&sides.base) == blob_at(&repo, base, path)?);
        assert!(bytes(&sides.local) == blob_at(&repo, conflict.local, path)?);
        assert!(bytes(&sides.incoming) == blob_at(&repo, conflict.incoming, path)?);
        assert!(sides.current.is_none());
    }
    Ok((inspection, sides))
}
/// Submit the observed local side of every path with its body replaced.
fn resolve_all(p: &Pair, conflict: &Conflict) -> Result<(git2::Oid, Vec<Vec<u8>>), FixtureError> {
    let inspection = p.inspect(&p.b, conflict.request.operation_id)?;
    let results = p
        .sides(&p.b, &inspection)?
        .iter()
        .map(|sides| resolution_of(sides.local.as_ref().unwrap(), BETA, "Resolved title"))
        .collect::<Vec<_>>();
    let outcome = fixed(p.resolve(
        &p.b.service,
        resolution_request(
            &p.b,
            conflict.request.operation_id,
            OperationId::new(),
            &inspection,
            results.clone(),
        ),
    ))?;
    let ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid } = outcome else {
        return Err(FixtureError);
    };
    Ok((commit_oid, results))
}
/// A local-only resolution checkpoint: two ordered parents, no merge state.
fn resolved_locally(
    worktree: &Path,
    reference: &str,
    conflict: &Conflict,
    candidate: git2::Oid,
    files: &[(&str, &[u8])],
) -> Result<(), FixtureError> {
    let repo = fixed(git2::Repository::open(worktree))?;
    assert_eq!(
        parents(&repo, candidate)?,
        [conflict.local, conflict.incoming]
    );
    assert_eq!(fixed(repo.refname_to_id(reference))?, candidate);
    assert_eq!(fixed(repo.head())?.target(), Some(candidate));
    assert!(!fixed(repo.index())?.has_conflicts());
    assert!(merge_heads(worktree)?.is_empty());
    assert_eq!(repo.state(), git2::RepositoryState::Clean);
    for (path, bytes) in files {
        assert!(fixed(std::fs::read(worktree.join(path)))? == *bytes);
        assert!(blob_at(&repo, candidate, path)?.as_deref() == Some(*bytes));
    }
    assert!(fixed(repo.statuses(None))?.is_empty());
    Ok(())
}
fn open_context(
    p: &Pair,
    side: &Side,
    kind: AuthoringKind,
    item: &str,
) -> Result<(), FixtureError> {
    let context = match fixed(side.service.prepare_context(p.target(side, kind, item)))? {
        ContextProvisionOutcome::Created(context)
        | ContextProvisionOutcome::Reused(context)
        | ContextProvisionOutcome::IndexPending { context } => context,
    };
    p.paths
        .borrow_mut()
        .push(context.worktree.to_string_lossy().as_bytes().to_vec());
    p.save_probes()
}

/// A1, A6, A7. Clone A edits the ticket and clone B comments on it: both
/// checkpoint the same shared context branch independently. B's ordinary
/// synchronization merges A's published history, publishes the merge with one
/// verified non-force update and touches exactly the selected refs.
fn two_clone_clean_ticket_divergence() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let base = fixed(p.bare()?.refname_to_id(MAIN))?;
    let a_tip = p.save_ticket(&p.a, "Alpha title", ALPHA)?;
    let n = p.server.receive_updates().len();
    outcome(
        fixed(p.sync(&p.a, p.ticket(&p.a)))?,
        true,
        ticket_target(),
        a_tip,
    );
    one_update(&p.server, n, TICKET_REF, git2::Oid::zero(), a_tip);

    let b_tip = p.submit_comment(&p.b, BETA)?;
    let b_repo = p.b.repo()?;
    assert_eq!(parents(&p.a.repo()?, a_tip)?, [base]);
    assert_eq!(parents(&b_repo, b_tip)?, [base]);
    assert!(b_repo.find_commit(a_tip).is_err());
    let refs_before = scope_before(&b_repo)?;
    let primary_before = checkout(&p.b.root)?;
    let n = p.server.receive_updates().len();
    let merged = published(fixed(p.sync(&p.b, p.ticket(&p.b)))?)?;

    // Exactly one merge commit, ordered local then incoming, both contents.
    assert_eq!(parents(&b_repo, merged)?, [b_tip, a_tip]);
    assert_eq!(merge_commits(&b_repo)?, 1);
    assert!(blob_at(&b_repo, merged, TICKET_PATH)? == blob_at(&b_repo, a_tip, TICKET_PATH)?);
    assert!(
        blob_at(&b_repo, merged, NEW_COMMENT_PATH)? == blob_at(&b_repo, b_tip, NEW_COMMENT_PATH)?
    );
    let worktree = p.b.worktree(TICKET);
    let linked = p.b.linked(TICKET)?;
    assert!(fixed(std::fs::read_to_string(worktree.join(TICKET_PATH)))?.contains(ALPHA));
    assert!(fixed(std::fs::read_to_string(worktree.join(NEW_COMMENT_PATH)))?.contains(BETA));
    assert_eq!(
        fixed(fixed(linked.index())?.write_tree())?,
        fixed(linked.find_commit(merged))?.tree_id()
    );
    assert!(fixed(linked.statuses(None))?.is_empty());
    // Ordinary verified publication: one accepted fast-forward of one ref.
    one_update(&p.server, n, TICKET_REF, a_tip, merged);
    assert_eq!(
        refs(&p.bare()?)?,
        [
            (MAIN.to_owned(), Some(base)),
            (TICKET_REF.to_owned(), Some(merged))
        ]
    );
    // Exact local ref scope: the context branch moved to the merge and its
    // tracking ref records what Fetch observed, never this clone's own push.
    scope_after(
        &b_repo,
        &refs_before,
        &[(TICKET_REF, merged), (TICKET_TRACKING, a_tip)],
        Some(&linked),
    )?;
    assert_eq!(fixed(b_repo.refname_to_id(MAIN))?, base);
    assert_eq!(merge_commits(&b_repo)?, 1);
    // The context ref moved; nothing of the primary checkout did.
    assert!(checkout(&p.b.root)? == primary_before);
    // Fresh discovery of the merged content in B.
    let (title, mut threads, head) = discovered(&p.b, TICKET)?;
    threads.sort();
    assert_eq!(title, "Alpha title");
    assert_eq!(threads, [COMMENT, NEW_COMMENT]);
    assert_eq!(head, Some(merged));

    // A receives B's history by fast-forward: nothing is pushed again.
    let n = p.server.receive_updates().len();
    outcome(
        fixed(p.sync(&p.a, p.ticket(&p.a)))?,
        false,
        ticket_target(),
        merged,
    );
    assert_eq!(p.server.receive_updates().len(), n);
    assert_eq!(fixed(p.a.repo()?.refname_to_id(TICKET_REF))?, merged);
    assert!(
        fixed(std::fs::read(p.a.worktree(TICKET).join(NEW_COMMENT_PATH)))?
            == fixed(std::fs::read(worktree.join(NEW_COMMENT_PATH)))?
    );
    let (title, mut threads, head) = discovered(&p.a, TICKET)?;
    threads.sort();
    assert_eq!((title.as_str(), head), ("Alpha title", Some(merged)));
    assert_eq!(threads, [COMMENT, NEW_COMMENT]);
    Ok(())
}

/// A1. The same shared-branch divergence for a document: A retitles it and B
/// rewrites its last line, in separate hunks of one canonical file.
fn two_clone_clean_document_divergence() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let base = fixed(p.bare()?.refname_to_id(MAIN))?;
    let a_tip = p.save_document(&p.a, "Alpha title", &document_body(SEED))?;
    let n = p.server.receive_updates().len();
    outcome(
        fixed(p.sync(&p.a, p.document(&p.a)))?,
        true,
        document_target(),
        a_tip,
    );
    one_update(&p.server, n, DOCUMENT_REF, git2::Oid::zero(), a_tip);
    let b_tip = p.save_document(&p.b, "Shared document", &document_body(BETA))?;
    let b_repo = p.b.repo()?;
    assert_eq!(parents(&b_repo, b_tip)?, [base]);
    let refs_before = scope_before(&b_repo)?;
    let n = p.server.receive_updates().len();
    let merged = published(fixed(p.sync(&p.b, p.document(&p.b)))?)?;
    assert_eq!(parents(&b_repo, merged)?, [b_tip, a_tip]);
    assert_eq!(merge_commits(&b_repo)?, 1);
    one_update(&p.server, n, DOCUMENT_REF, a_tip, merged);
    scope_after(
        &b_repo,
        &refs_before,
        &[(DOCUMENT_REF, merged), (DOCUMENT_TRACKING, a_tip)],
        Some(&p.b.linked(DOCUMENT)?),
    )?;
    let expected = document_source("Alpha title", BETA);
    assert!(blob_at(&b_repo, merged, DOCUMENT_PATH)?.as_deref() == Some(expected.as_bytes()));
    assert!(
        fixed(std::fs::read(p.b.worktree(DOCUMENT).join(DOCUMENT_PATH)))? == expected.as_bytes()
    );
    assert!(fixed(p.b.linked(DOCUMENT)?.statuses(None))?.is_empty());
    assert_eq!(
        refs(&p.bare()?)?,
        [
            (MAIN.to_owned(), Some(base)),
            (DOCUMENT_REF.to_owned(), Some(merged))
        ]
    );
    let (title, _, head) = discovered(&p.b, DOCUMENT)?;
    assert_eq!((title.as_str(), head), ("Alpha title", Some(merged)));
    let n = p.server.receive_updates().len();
    outcome(
        fixed(p.sync(&p.a, p.document(&p.a)))?,
        false,
        document_target(),
        merged,
    );
    assert_eq!(p.server.receive_updates().len(), n);
    assert!(
        fixed(std::fs::read(p.a.worktree(DOCUMENT).join(DOCUMENT_PATH)))? == expected.as_bytes()
    );
    Ok(())
}

/// A1, A6. Both clones commit to the primary branch; each publishes through
/// its own service. The second one merges and both histories survive.
fn two_clone_primary_divergence() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let base = fixed(p.bare()?.refname_to_id(MAIN))?;
    let a_repo = p.a.repo()?;
    let a_tip = advance(&a_repo, MAIN, &[("alpha.txt", Some((b"alpha\n", REGULAR)))])?;
    let n = p.server.receive_updates().len();
    outcome(
        fixed(p.sync(&p.a, p.primary(&p.a)))?,
        true,
        SynchronizationTarget::Primary,
        a_tip,
    );
    one_update(&p.server, n, MAIN, base, a_tip);
    let b_repo = p.b.repo()?;
    let b_tip = advance(&b_repo, MAIN, &[("beta.txt", Some((b"beta\n", REGULAR)))])?;
    let refs_before = scope_before(&b_repo)?;
    let n = p.server.receive_updates().len();
    let merged = published(fixed(p.sync(&p.b, p.primary(&p.b)))?)?;
    assert_eq!(parents(&b_repo, merged)?, [b_tip, a_tip]);
    assert_eq!(merge_commits(&b_repo)?, 1);
    one_update(&p.server, n, MAIN, a_tip, merged);
    assert_eq!(refs(&p.bare()?)?, [(MAIN.to_owned(), Some(merged))]);
    for (name, bytes) in [
        ("alpha.txt", b"alpha\n".as_slice()),
        ("beta.txt", b"beta\n"),
    ] {
        assert_eq!(fixed(std::fs::read(p.b.root.join(name)))?, bytes);
    }
    assert!(fixed(b_repo.statuses(None))?.is_empty());
    // The primary branch moved to the merge; its tracking ref is A's tip.
    scope_after(
        &b_repo,
        &refs_before,
        &[(MAIN, merged), (MAIN_TRACKING, a_tip)],
        None,
    )?;
    let n = p.server.receive_updates().len();
    outcome(
        fixed(p.sync(&p.a, p.primary(&p.a)))?,
        false,
        SynchronizationTarget::Primary,
        merged,
    );
    assert_eq!(p.server.receive_updates().len(), n);
    assert_eq!(fixed(std::fs::read(p.a.root.join("beta.txt")))?, b"beta\n");
    assert_eq!(merge_commits(&a_repo)?, 1);
    Ok(())
}

/// A1. A publishes both its context branch and a primary commit. B's one
/// context synchronization integrates the fetched context first and the
/// primary branch second: two ordered merges, one publication.
fn two_clone_context_then_primary_ordered_merges() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let base = fixed(p.bare()?.refname_to_id(MAIN))?;
    let a_context = p.save_ticket(&p.a, "Alpha title", ALPHA)?;
    assert_eq!(published(fixed(p.sync(&p.a, p.ticket(&p.a)))?)?, a_context);
    let a_primary = advance(
        &p.a.repo()?,
        MAIN,
        &[("alpha.txt", Some((b"alpha\n", REGULAR)))],
    )?;
    assert_eq!(published(fixed(p.sync(&p.a, p.primary(&p.a)))?)?, a_primary);
    let b_tip = p.submit_comment(&p.b, BETA)?;
    let b_repo = p.b.repo()?;
    let refs_before = scope_before(&b_repo)?;
    let n = p.server.receive_updates().len();
    let merged = published(fixed(p.sync(&p.b, p.ticket(&p.b)))?)?;
    let first = parents(&b_repo, merged)?;
    assert_eq!(first.len(), 2);
    assert_eq!(first[1], a_primary);
    assert_eq!(parents(&b_repo, first[0])?, [b_tip, a_context]);
    assert_eq!(merge_commits(&b_repo)?, 2);
    one_update(&p.server, n, TICKET_REF, a_context, merged);
    // The context target never moves the local primary branch: the context
    // branch moved, and both tracking refs record what Fetch observed.
    assert_eq!(fixed(b_repo.refname_to_id(MAIN))?, base);
    scope_after(
        &b_repo,
        &refs_before,
        &[
            (TICKET_REF, merged),
            (TICKET_TRACKING, a_context),
            (MAIN_TRACKING, a_primary),
        ],
        Some(&p.b.linked(TICKET)?),
    )?;
    assert!(!p.b.root.join("alpha.txt").exists());
    let worktree = p.b.worktree(TICKET);
    assert_eq!(
        fixed(std::fs::read(worktree.join("alpha.txt")))?,
        b"alpha\n"
    );
    assert!(fixed(std::fs::read_to_string(worktree.join(TICKET_PATH)))?.contains(ALPHA));
    assert!(fixed(std::fs::read_to_string(worktree.join(NEW_COMMENT_PATH)))?.contains(BETA));
    assert!(fixed(p.b.linked(TICKET)?.statuses(None))?.is_empty());
    assert_eq!(
        refs(&p.bare()?)?,
        [
            (MAIN.to_owned(), Some(a_primary)),
            (TICKET_REF.to_owned(), Some(merged))
        ]
    );
    Ok(())
}

/// A2, A3, A7. Both clones save the same document on the same shared branch.
/// The conflict is inspectable and unpublished; the caller submits the
/// observed local side as its resolution, restarts explicitly, and the
/// two-parent checkpoint is published and discovered in both clones.
fn two_clone_document_conflict_resolved_and_published() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let incoming = p.save_document(&p.a, "Alpha title", &document_body(ALPHA))?;
    assert_eq!(published(fixed(p.sync(&p.a, p.document(&p.a)))?)?, incoming);
    let local = p.save_document(&p.b, "Beta title", &document_body(BETA))?;
    let worktree = p.b.worktree(DOCUMENT);
    let before = physical(&p.b.root, &p.b.root)?;
    let request = p.document(&p.b);
    conflict_pending(&p, &request, SynchronizationStage::Context)?;
    let conflict = Conflict {
        request,
        local,
        incoming,
    };
    conflict_installed(&worktree, DOCUMENT_REF, &conflict, &[DOCUMENT_PATH])?;
    // Branch, primary checkout and the remote keep their pre-merge baseline.
    assert!(physical(&p.b.root, &p.b.root)? == before);
    assert_eq!(fixed(p.bare()?.refname_to_id(DOCUMENT_REF))?, incoming);
    let (_, sides) = inspect_canonical(
        &p,
        &worktree,
        &conflict,
        SynchronizationStage::Context,
        &[DOCUMENT_PATH],
    )?;
    // The same ID without restart, and an explicit restart, stay offline.
    let pending = physical(&p.b.root, &worktree)?;
    let auth = p.server.accepted_keys().len();
    assert!(matches!(
        p.sync(&p.b, conflict.request.clone()),
        Err(SynchronizationError::RecoveryRequired)
    ));
    assert!(matches!(
        p.sync(&p.b, restart(&conflict.request)),
        Err(SynchronizationError::ConflictPending { operation_id, .. })
            if operation_id == conflict.request.operation_id
    ));
    assert_eq!(p.server.accepted_keys().len(), auth);
    assert!(physical(&p.b.root, &worktree)? == pending);

    let n = p.server.receive_updates().len();
    let (candidate, results) = resolve_all(&p, &conflict)?;
    assert!(results[0] == resolution_of(sides[0].local.as_ref().unwrap(), BETA, "Resolved title"));
    assert!(results[0] == document_source("Resolved title", RESOLVED).as_bytes());
    resolved_locally(
        &worktree,
        DOCUMENT_REF,
        &conflict,
        candidate,
        &[(DOCUMENT_PATH, &results[0])],
    )?;
    // Resolution is local only: nothing was published by it.
    assert_eq!(p.server.receive_updates().len(), n);
    assert_eq!(fixed(p.bare()?.refname_to_id(DOCUMENT_REF))?, incoming);
    assert_eq!(p.server.accepted_keys().len(), auth);

    outcome(
        fixed(p.sync(&p.b, restart(&conflict.request)))?,
        true,
        document_target(),
        candidate,
    );
    one_update(&p.server, n, DOCUMENT_REF, incoming, candidate);
    assert_eq!(merge_commits(&p.b.repo()?)?, 1);
    // The published resolution commit is dated when it was resolved.
    let now = fixed(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH))?.as_secs();
    let published_commit = p.bare()?;
    let published_commit = fixed(published_commit.find_commit(candidate))?;
    for when in [published_commit.author().when(), published_commit.time()] {
        assert!((now as i64 - when.seconds()).abs() < 3600);
    }
    let (title, _, head) = discovered(&p.b, DOCUMENT)?;
    assert_eq!((title.as_str(), head), ("Resolved title", Some(candidate)));
    // Authority replays without transport or another push.
    let auth = p.server.accepted_keys().len();
    outcome(
        fixed(p.sync(&p.b, restart(&conflict.request)))?,
        true,
        document_target(),
        candidate,
    );
    assert_eq!(p.server.accepted_keys().len(), auth);
    // A fast-forwards to the resolution and discovers it.
    outcome(
        fixed(p.sync(&p.a, p.document(&p.a)))?,
        false,
        document_target(),
        candidate,
    );
    assert_eq!(p.server.receive_updates().len(), n + 1);
    assert!(fixed(std::fs::read(p.a.worktree(DOCUMENT).join(DOCUMENT_PATH)))? == results[0]);
    let (title, _, head) = discovered(&p.a, DOCUMENT)?;
    assert_eq!((title.as_str(), head), ("Resolved title", Some(candidate)));
    Ok(())
}

/// Remove clone B's effective Git identity: empty repository-local values
/// shadow any identity of the machine running the suite.
fn without_identity(side: &Side) -> Result<(), FixtureError> {
    let mut config = fixed(side.repo()?.config())?;
    fixed(config.set_str("user.name", ""))?;
    fixed(config.set_str("user.email", ""))?;
    fixed(config.set_bool("user.useConfigOnly", true))?;
    Ok(())
}
fn confirmed_identity(expected_configuration: ExpectedConfiguration) -> ConfirmedCommitIdentity {
    ConfirmedCommitIdentity {
        confirmation_id: OperationId::new(),
        identity: CommitIdentity {
            name: "Confirmed".into(),
            email: "confirmed@example.invalid".into(),
        },
        expected_configuration,
    }
}
fn author(repo: &git2::Repository, oid: git2::Oid) -> Result<(String, String), FixtureError> {
    let commit = fixed(repo.find_commit(oid))?;
    let author = commit.author();
    Ok((
        author.name().unwrap_or_default().to_owned(),
        author.email().unwrap_or_default().to_owned(),
    ))
}

/// A divergent primary merge with no effective Git identity stops at the
/// typed identity boundary before any candidate. The caller confirms an
/// identity with the observation that boundary returned, restarts the same
/// operation, and the merge is committed with it and published.
fn confirmed_identity_completes_a_divergent_primary_merge() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let a_tip = advance(
        &p.a.repo()?,
        MAIN,
        &[("alpha.txt", Some((b"alpha\n", REGULAR)))],
    )?;
    assert_eq!(published(fixed(p.sync(&p.a, p.primary(&p.a)))?)?, a_tip);
    let b_repo = p.b.repo()?;
    let b_tip = advance(&b_repo, MAIN, &[("beta.txt", Some((b"beta\n", REGULAR)))])?;
    without_identity(&p.b)?;
    let request = p.primary(&p.b);
    let n = p.server.receive_updates().len();
    let Err(SynchronizationError::IdentityRequired {
        target,
        expected_configuration,
    }) = p.sync(&p.b, request.clone())
    else {
        return Err(FixtureError);
    };
    assert_eq!(target, SynchronizationTarget::Primary);
    assert_eq!(fixed(b_repo.refname_to_id(MAIN))?, b_tip);
    assert_eq!(merge_commits(&b_repo)?, 0);
    assert_eq!(p.server.receive_updates().len(), n);
    // Restarting without a confirmation stays at the same boundary.
    assert!(matches!(
        p.sync(&p.b, restart(&request)),
        Err(SynchronizationError::IdentityRequired { expected_configuration: again, .. })
            if again == expected_configuration
    ));
    // The identity configuration changes after that boundary was issued (still
    // incomplete): a confirmation carrying the earlier observation is stale.
    // It is refused as an external change and nothing is written.
    fixed(fixed(b_repo.config())?.set_str("user.name", "Half"))?;
    let mut stale = restart(&request);
    stale.confirmed_identity = Some(confirmed_identity(expected_configuration));
    assert!(matches!(
        p.sync(&p.b, stale),
        Err(SynchronizationError::ExternalChange)
    ));
    let config = fixed(fixed(b_repo.config())?.snapshot())?;
    assert_eq!(fixed(config.get_string("user.name"))?, "Half");
    assert_eq!(fixed(config.get_string("user.email"))?, "");
    assert_eq!(
        fixed(p.b.db()?.query_row(
            "SELECT count(*) FROM remote_identity_confirmations",
            [],
            |row| row.get::<_, i64>(0)
        ))?,
        0
    );
    assert_eq!(fixed(b_repo.refname_to_id(MAIN))?, b_tip);
    assert_eq!(merge_commits(&b_repo)?, 0);
    assert_eq!(p.server.receive_updates().len(), n);
    // The boundary reports its current observation, which is then accepted.
    let Err(SynchronizationError::IdentityRequired {
        expected_configuration: current,
        ..
    }) = p.sync(&p.b, restart(&request))
    else {
        return Err(FixtureError);
    };
    assert!(current != expected_configuration);
    let mut confirmed = restart(&request);
    confirmed.confirmed_identity = Some(confirmed_identity(current));
    let merged = published(fixed(p.sync(&p.b, confirmed))?)?;
    assert_eq!(parents(&b_repo, merged)?, [b_tip, a_tip]);
    assert_eq!(merge_commits(&b_repo)?, 1);
    assert_eq!(
        author(&b_repo, merged)?,
        ("Confirmed".into(), "confirmed@example.invalid".into())
    );
    assert_eq!(
        fixed(b_repo.find_commit(merged))?.message(),
        Some("Merge remote primary")
    );
    one_update(&p.server, n, MAIN, a_tip, merged);
    assert!(fixed(b_repo.statuses(None))?.is_empty());
    Ok(())
}

/// A canonical conflict resolved with no effective Git identity returns the
/// typed identity outcome and changes nothing. The caller confirms an identity
/// with the observation of its inspection; the resolution is committed with
/// it and the original operation publishes it.
fn confirmed_identity_completes_a_canonical_resolution() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let conflict = ticket_conflict(&p)?;
    let operation = conflict.request.operation_id;
    let worktree = p.b.worktree(TICKET);
    without_identity(&p.b)?;
    let inspection = p.inspect(&p.b, operation)?;
    let results = p
        .sides(&p.b, &inspection)?
        .iter()
        .map(|sides| resolution_of(sides.local.as_ref().unwrap(), BETA, "Resolved title"))
        .collect::<Vec<_>>();
    let mut request = resolution_request(
        &p.b,
        operation,
        OperationId::new(),
        &inspection,
        results.clone(),
    );
    let pending = physical(&p.b.root, &worktree)?;
    assert_eq!(
        fixed(p.resolve(&p.b.service, request.clone()))?,
        ResolveSynchronizationOutcome::IdentityRequired
    );
    assert!(physical(&p.b.root, &worktree)? == pending);
    conflict_installed(&worktree, TICKET_REF, &conflict, &[TICKET_PATH])?;
    request.identity = Some(confirmed_identity(
        inspection.observation.expected_configuration(),
    ));
    let n = p.server.receive_updates().len();
    let ResolveSynchronizationOutcome::LocalCheckpointComplete {
        commit_oid: candidate,
    } = fixed(p.resolve(&p.b.service, request))?
    else {
        return Err(FixtureError);
    };
    resolved_locally(
        &worktree,
        TICKET_REF,
        &conflict,
        candidate,
        &[(TICKET_PATH, &results[0])],
    )?;
    let linked = p.b.linked(TICKET)?;
    assert_eq!(
        author(&linked, candidate)?,
        ("Confirmed".into(), "confirmed@example.invalid".into())
    );
    assert_eq!(
        fixed(linked.find_commit(candidate))?.message(),
        Some(format!("Resolve synchronization ticket {TICKET}").as_str())
    );
    outcome(
        fixed(p.sync(&p.b, restart(&conflict.request)))?,
        true,
        ticket_target(),
        candidate,
    );
    one_update(&p.server, n, TICKET_REF, conflict.incoming, candidate);
    Ok(())
}

/// A3. Stale, malformed and different resolution submissions for a real
/// two-clone ticket conflict return typed categories and change nothing; the
/// one accepted attempt replays identically and is published once.
fn two_clone_ticket_conflict_stale_and_different_resolutions() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let conflict = ticket_conflict(&p)?;
    let operation = conflict.request.operation_id;
    let worktree = p.b.worktree(TICKET);
    conflict_installed(&worktree, TICKET_REF, &conflict, &[TICKET_PATH])?;
    let (inspection, sides) = inspect_canonical(
        &p,
        &worktree,
        &conflict,
        SynchronizationStage::Context,
        &[TICKET_PATH],
    )?;
    let before = physical(&p.b.root, &worktree)?;
    let commits = commit_inventory(&p.b.repo()?)?;
    let local = sides[0].local.as_ref().unwrap();
    let resolved = resolution_of(local, BETA, "Resolved title");
    let submit = |attempt: OperationId, bytes: Vec<u8>| {
        p.resolve(
            &p.b.service,
            resolution_request(&p.b, operation, attempt, &inspection, vec![bytes]),
        )
    };
    let unchanged = || -> Result<(), FixtureError> {
        assert!(physical(&p.b.root, &worktree)? == before);
        assert_eq!(commit_inventory(&p.b.repo()?)?, commits);
        Ok(())
    };

    // Not canonical at all, then a changed item identity, then the wrong kind.
    let marked = fixed(std::fs::read(worktree.join(TICKET_PATH)))?;
    assert!(matches!(
        submit(OperationId::new(), marked),
        Ok(ResolveSynchronizationOutcome::ValidationFailed)
    ));
    unchanged()?;
    let renamed = String::from_utf8(resolved.clone())
        .unwrap()
        .replace(TICKET, DOCUMENT)
        .into_bytes();
    assert!(matches!(
        submit(OperationId::new(), renamed),
        Ok(ResolveSynchronizationOutcome::ValidationFailed)
    ));
    unchanged()?;
    assert!(matches!(
        submit(
            OperationId::new(),
            document_source("Resolved title", RESOLVED).into_bytes()
        ),
        Ok(ResolveSynchronizationOutcome::ValidationFailed)
    ));
    unchanged()?;

    // An observation made before the repository configuration changed is stale.
    let config = p.b.root.join(".manyhands").join("config.toml");
    let configured = fixed(std::fs::read(&config))?;
    let mut touched = configured.clone();
    touched.extend_from_slice(b"# changed after inspection\n");
    fixed(std::fs::write(&config, &touched))?;
    assert!(matches!(
        submit(OperationId::new(), resolved.clone()),
        Ok(ResolveSynchronizationOutcome::StaleObservation)
    ));
    fixed(std::fs::write(&config, &configured))?;
    unchanged()?;

    // A false repair that only stages the path is typed Recovery for both
    // resolution and restart, with no network; it is not sticky: restoring
    // the exact index makes the same conflict resolvable again.
    let linked = p.b.linked(TICKET)?;
    let index_file = linked.path().join("index");
    let conflicted = fixed(std::fs::read(&index_file))?;
    {
        let mut index = fixed(linked.index())?;
        fixed(index.add_path(Path::new(TICKET_PATH)))?;
        fixed(index.write())?;
    }
    let staged = physical(&p.b.root, &worktree)?;
    let auth = p.server.accepted_keys().len();
    assert!(matches!(
        submit(OperationId::new(), resolved.clone()),
        Err(SynchronizationError::RecoveryRequired)
    ));
    assert!(matches!(
        p.sync(&p.b, restart(&conflict.request)),
        Err(SynchronizationError::RecoveryRequired)
    ));
    assert_eq!(p.server.accepted_keys().len(), auth);
    assert!(physical(&p.b.root, &worktree)? == staged);
    fixed(std::fs::write(&index_file, &conflicted))?;
    unchanged()?;

    // The accepted attempt.
    let attempt = OperationId::new();
    let n = p.server.receive_updates().len();
    let Ok(ResolveSynchronizationOutcome::LocalCheckpointComplete {
        commit_oid: candidate,
    }) = submit(attempt, resolved.clone())
    else {
        return Err(FixtureError);
    };
    resolved_locally(
        &worktree,
        TICKET_REF,
        &conflict,
        candidate,
        &[(TICKET_PATH, &resolved)],
    )?;
    let after = physical(&p.b.root, &worktree)?;
    let commits = commit_inventory(&p.b.repo()?)?;
    // The identical attempt replays its own checkpoint.
    assert!(matches!(
        submit(attempt, resolved.clone()),
        Ok(ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid })
            if commit_oid == candidate
    ));
    // The same attempt with different bytes, and a new attempt, are refused.
    let different = String::from_utf8(resolved.clone())
        .unwrap()
        .replace("Resolved title", "Different title")
        .into_bytes();
    assert!(matches!(
        submit(attempt, different.clone()),
        Err(SynchronizationError::RecoveryRequired)
    ));
    assert!(matches!(
        submit(OperationId::new(), different),
        Err(SynchronizationError::Repository(RepositoryError {
            kind: RepositoryErrorKind::RecoveryRequired,
            ..
        }))
    ));
    assert!(physical(&p.b.root, &worktree)? == after);
    assert_eq!(commit_inventory(&p.b.repo()?)?, commits);
    assert_eq!(merge_commits(&p.b.repo()?)?, 1);
    assert_eq!(p.server.receive_updates().len(), n);

    outcome(
        fixed(p.sync(&p.b, restart(&conflict.request)))?,
        true,
        ticket_target(),
        candidate,
    );
    one_update(&p.server, n, TICKET_REF, conflict.incoming, candidate);
    let (title, _, head) = discovered(&p.b, TICKET)?;
    assert_eq!((title.as_str(), head), ("Resolved title", Some(candidate)));
    Ok(())
}

/// A3. A comment-only conflict, then a ticket-and-comment conflict set, on
/// the shared ticket branch. The set resolves only as a whole.
fn two_clone_comment_and_multi_path_conflicts() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let worktree = p.b.worktree(TICKET);
    let comment = |body: &str| comment_source(body).into_bytes();
    // Round one: both clones rewrite the same seeded comment.
    open_context(&p, &p.a, AuthoringKind::Ticket, TICKET)?;
    open_context(&p, &p.b, AuthoringKind::Ticket, TICKET)?;
    let incoming = advance(
        &p.a.linked(TICKET)?,
        TICKET_REF,
        &[(COMMENT_PATH, Some((&comment(ALPHA), REGULAR)))],
    )?;
    assert_eq!(published(fixed(p.sync(&p.a, p.ticket(&p.a)))?)?, incoming);
    let local = advance(
        &p.b.linked(TICKET)?,
        TICKET_REF,
        &[(COMMENT_PATH, Some((&comment(BETA), REGULAR)))],
    )?;
    let request = p.ticket(&p.b);
    conflict_pending(&p, &request, SynchronizationStage::Context)?;
    let conflict = Conflict {
        request,
        local,
        incoming,
    };
    conflict_installed(&worktree, TICKET_REF, &conflict, &[COMMENT_PATH])?;
    let (inspection, _) = inspect_canonical(
        &p,
        &worktree,
        &conflict,
        SynchronizationStage::Context,
        &[COMMENT_PATH],
    )?;
    // A well-formed comment that rewrites immutable provenance retained by
    // every recorded side is refused before any write.
    let before = physical(&p.b.root, &worktree)?;
    assert!(matches!(
        p.resolve(
            &p.b.service,
            resolution_request(
                &p.b,
                conflict.request.operation_id,
                OperationId::new(),
                &inspection,
                vec![comment_source_created(RESOLVED, 1_767_225_601).into_bytes()],
            ),
        ),
        Ok(ResolveSynchronizationOutcome::ValidationFailed)
    ));
    assert!(physical(&p.b.root, &worktree)? == before);
    let n = p.server.receive_updates().len();
    let (first, results) = resolve_all(&p, &conflict)?;
    assert!(results[0] == comment(RESOLVED));
    resolved_locally(
        &worktree,
        TICKET_REF,
        &conflict,
        first,
        &[(COMMENT_PATH, &results[0])],
    )?;
    outcome(
        fixed(p.sync(&p.b, restart(&conflict.request)))?,
        true,
        ticket_target(),
        first,
    );
    one_update(&p.server, n, TICKET_REF, incoming, first);
    outcome(
        fixed(p.sync(&p.a, p.ticket(&p.a)))?,
        false,
        ticket_target(),
        first,
    );

    // Round two: a public ticket save and a comment rewrite on each clone.
    p.save_ticket(&p.a, "Alpha title", ALPHA)?;
    let incoming = advance(
        &p.a.linked(TICKET)?,
        TICKET_REF,
        &[(COMMENT_PATH, Some((&comment(ALPHA), REGULAR)))],
    )?;
    assert_eq!(published(fixed(p.sync(&p.a, p.ticket(&p.a)))?)?, incoming);
    p.save_ticket(&p.b, "Beta title", BETA)?;
    let local = advance(
        &p.b.linked(TICKET)?,
        TICKET_REF,
        &[(COMMENT_PATH, Some((&comment(BETA), REGULAR)))],
    )?;
    let request = p.ticket(&p.b);
    conflict_pending(&p, &request, SynchronizationStage::Context)?;
    let conflict = Conflict {
        request,
        local,
        incoming,
    };
    let set = [COMMENT_PATH, TICKET_PATH];
    conflict_installed(&worktree, TICKET_REF, &conflict, &set)?;
    let (inspection, sides) = inspect_canonical(
        &p,
        &worktree,
        &conflict,
        SynchronizationStage::Context,
        &set,
    )?;
    // One path of a two-path set is never applied on its own. The set is
    // current and eligible, so this is the count category, not external-only.
    let before = physical(&p.b.root, &worktree)?;
    let mut partial = inspection.clone();
    partial.paths.truncate(1);
    assert!(matches!(
        p.resolve(
            &p.b.service,
            resolution_request(
                &p.b,
                conflict.request.operation_id,
                OperationId::new(),
                &partial,
                vec![resolution_of(
                    sides[0].local.as_ref().unwrap(),
                    BETA,
                    "Resolved title"
                )],
            ),
        ),
        Ok(ResolveSynchronizationOutcome::StaleObservation)
    ));
    assert!(physical(&p.b.root, &worktree)? == before);
    let n = p.server.receive_updates().len();
    let (second, results) = resolve_all(&p, &conflict)?;
    assert!(results[0] == comment(RESOLVED));
    assert!(results[1] == ticket_source("Resolved title", RESOLVED).as_bytes());
    resolved_locally(
        &worktree,
        TICKET_REF,
        &conflict,
        second,
        &[(COMMENT_PATH, &results[0]), (TICKET_PATH, &results[1])],
    )?;
    assert_eq!(p.server.receive_updates().len(), n);
    outcome(
        fixed(p.sync(&p.b, restart(&conflict.request)))?,
        true,
        ticket_target(),
        second,
    );
    one_update(&p.server, n, TICKET_REF, incoming, second);
    assert_eq!(merge_commits(&p.b.repo()?)?, 2);
    let (title, threads, head) = discovered(&p.b, TICKET)?;
    assert_eq!((title.as_str(), head), ("Resolved title", Some(second)));
    assert_eq!(threads, [COMMENT]);
    Ok(())
}

/// A2, A5. B's context synchronization merges A's published context cleanly
/// and then conflicts with A's published primary branch. The first merge is
/// retained exactly once through inspection, resolution and publication.
fn two_clone_context_merged_then_primary_conflicted() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let base = fixed(p.bare()?.refname_to_id(MAIN))?;
    let a_context = p.submit_comment(&p.a, ALPHA)?;
    assert_eq!(published(fixed(p.sync(&p.a, p.ticket(&p.a)))?)?, a_context);
    let alpha = ticket_source("Alpha title", ALPHA);
    let a_repo = p.a.repo()?;
    let a_primary = advance(
        &a_repo,
        MAIN,
        &[(TICKET_PATH, Some((alpha.as_bytes(), REGULAR)))],
    )?;
    push_peer(&a_repo, &p.server, MAIN)?;
    let b_tip = p.save_ticket(&p.b, "Beta title", BETA)?;
    let request = p.ticket(&p.b);
    conflict_pending(&p, &request, SynchronizationStage::Primary)?;
    let b_repo = p.b.repo()?;
    let first = fixed(b_repo.refname_to_id(TICKET_REF))?;
    // The context stage is a completed merge; the primary stage is pending.
    assert_eq!(parents(&b_repo, first)?, [b_tip, a_context]);
    assert_eq!(merge_commits(&b_repo)?, 1);
    assert_eq!(fixed(p.bare()?.refname_to_id(TICKET_REF))?, a_context);
    let conflict = Conflict {
        request,
        local: first,
        incoming: a_primary,
    };
    let worktree = p.b.worktree(TICKET);
    conflict_installed(&worktree, TICKET_REF, &conflict, &[TICKET_PATH])?;
    assert!(fixed(std::fs::read_to_string(worktree.join(NEW_COMMENT_PATH)))?.contains(ALPHA));
    inspect_canonical(
        &p,
        &worktree,
        &conflict,
        SynchronizationStage::Primary,
        &[TICKET_PATH],
    )?;
    let n = p.server.receive_updates().len();
    let (second, results) = resolve_all(&p, &conflict)?;
    resolved_locally(
        &worktree,
        TICKET_REF,
        &conflict,
        second,
        &[(TICKET_PATH, &results[0])],
    )?;
    assert_eq!(parents(&b_repo, second)?, [first, a_primary]);
    outcome(
        fixed(p.sync(&p.b, restart(&conflict.request)))?,
        true,
        ticket_target(),
        second,
    );
    one_update(&p.server, n, TICKET_REF, a_context, second);
    // Two merges in total: the first was neither repeated nor replaced.
    assert_eq!(merge_commits(&b_repo)?, 2);
    assert_eq!(fixed(b_repo.refname_to_id(MAIN))?, base);
    assert!(fixed(std::fs::read_to_string(worktree.join(NEW_COMMENT_PATH)))?.contains(ALPHA));
    Ok(())
}

/// Conflict shapes the service never resolves in-process.
#[derive(Clone, Copy, PartialEq, Eq)]
enum External {
    Code,
    Binary,
    Delete,
    Rename,
    Symlink,
    Mixed,
}
type OwnedChange = (String, Option<(Vec<u8>, u32)>);
struct ExternalPlan {
    alpha: Vec<OwnedChange>,
    beta: Vec<OwnedChange>,
    /// The operator's whole-merge result, relative to the local parent.
    repair: Vec<OwnedChange>,
    conflicts: usize,
}
fn changes(owned: &[OwnedChange]) -> Vec<Change<'_>> {
    owned
        .iter()
        .map(|(path, change)| {
            (
                path.as_str(),
                change
                    .as_ref()
                    .map(|(bytes, mode)| (bytes.as_slice(), *mode)),
            )
        })
        .collect()
}
fn external_plan(b: &git2::Repository, kind: External) -> Result<ExternalPlan, FixtureError> {
    let file = |path: &str, bytes: &[u8]| (path.to_owned(), Some((bytes.to_vec(), REGULAR)));
    let gone = |path: &str| (path.to_owned(), None);
    Ok(match kind {
        External::Code => ExternalPlan {
            alpha: vec![file(HOSTILE_PATH, b"alpha code\n")],
            beta: vec![file(HOSTILE_PATH, b"beta code\n")],
            repair: vec![file(HOSTILE_PATH, b"repaired code\n")],
            conflicts: 1,
        },
        External::Binary => ExternalPlan {
            alpha: vec![file(CODE_PATH, b"alpha\0binary\n")],
            beta: vec![file(CODE_PATH, b"beta\0binary\n")],
            repair: vec![file(CODE_PATH, b"repaired binary\n")],
            conflicts: 1,
        },
        External::Delete => ExternalPlan {
            alpha: vec![gone(CODE_PATH)],
            beta: vec![file(CODE_PATH, b"beta keeps the file\n")],
            repair: vec![file(CODE_PATH, b"repaired delete\n")],
            conflicts: 1,
        },
        External::Rename => {
            let head = fixed(b.refname_to_id(MAIN))?;
            let current = blob_at(b, head, CODE_PATH)?.ok_or(FixtureError)?;
            ExternalPlan {
                alpha: vec![gone(CODE_PATH), file("renamed-alpha.txt", &current)],
                beta: vec![gone(CODE_PATH), file("renamed-beta.txt", &current)],
                repair: vec![
                    gone("renamed-beta.txt"),
                    file(CODE_PATH, b"repaired rename\n"),
                ],
                conflicts: 3,
            }
        }
        External::Symlink => ExternalPlan {
            alpha: vec![(
                CODE_PATH.to_owned(),
                Some((b"alpha-link-target".to_vec(), LINK)),
            )],
            beta: vec![(
                CODE_PATH.to_owned(),
                Some((b"beta-link-target".to_vec(), LINK)),
            )],
            repair: vec![file(CODE_PATH, b"repaired symlink\n")],
            conflicts: 1,
        },
        External::Mixed => ExternalPlan {
            alpha: vec![
                file(TICKET_PATH, ticket_source("Alpha title", ALPHA).as_bytes()),
                file(HOSTILE_PATH, b"alpha mixed code\n"),
            ],
            beta: vec![
                file(TICKET_PATH, ticket_source("Beta title", BETA).as_bytes()),
                file(HOSTILE_PATH, b"beta mixed code\n"),
            ],
            repair: vec![
                file(
                    TICKET_PATH,
                    ticket_source("Resolved title", RESOLVED).as_bytes(),
                ),
                file(HOSTILE_PATH, b"repaired mixed code\n"),
            ],
            conflicts: 2,
        },
    })
}
/// The operator's own tooling installs a whole committed merge: the commit,
/// the branch, and a worktree and index that match it.
fn install_repair(
    b: &git2::Repository,
    repaired: git2::Oid,
    cleanup: bool,
) -> Result<(), FixtureError> {
    fixed(
        b.checkout_tree(
            &fixed(b.find_object(repaired, None))?,
            Some(
                git2::build::CheckoutBuilder::new()
                    .force()
                    .remove_untracked(true),
            ),
        ),
    )?;
    fixed(b.reference(MAIN, repaired, true, "external repair"))?;
    let mut index = fixed(b.index())?;
    fixed(index.read_tree(&fixed(fixed(b.find_commit(repaired))?.tree())?))?;
    fixed(index.write())?;
    if cleanup {
        // `git commit` removes its own merge metadata; other tools leave it
        // for the service to retire.
        fixed(b.cleanup_state())?;
    }
    Ok(())
}
/// A4, A5. One external-only conflict on the primary branch between the two
/// clones: refused in-process without any write, repaired by a genuine
/// two-parent fixture commit, then resumed by one deliberate restart.
fn external_round(
    p: &Pair,
    a: &git2::Repository,
    kind: External,
    cleanup: bool,
    false_repairs: bool,
) -> Result<(), FixtureError> {
    let b = p.b.repo()?;
    let plan = external_plan(&b, kind)?;
    let merges = merge_commits(&b)?;
    let incoming = advance(a, MAIN, &changes(&plan.alpha))?;
    push_peer(a, &p.server, MAIN)?;
    let local = advance(&b, MAIN, &changes(&plan.beta))?;
    let request = p.primary(&p.b);
    conflict_pending(p, &request, SynchronizationStage::Primary)?;
    assert_eq!(fixed(b.refname_to_id(MAIN))?, local);
    assert_eq!(merge_heads(&p.b.root)?, [incoming]);
    assert!(fixed(fixed(git2::Repository::open(&p.b.root))?.index())?.has_conflicts());
    assert_eq!(fixed(p.bare()?.refname_to_id(MAIN))?, incoming);
    let inspection = p.inspect(&p.b, request.operation_id)?;
    assert_eq!(
        (inspection.local_parent, inspection.incoming_parent),
        (local, incoming)
    );
    assert_eq!(inspection.paths.len(), plan.conflicts);
    assert!(
        inspection
            .paths
            .iter()
            .all(|path| path.eligibility == ConflictEligibility::ExternalResolutionRequired)
    );
    // The whole set is external: even well-formed canonical bytes for every
    // token write nothing, stage nothing and create no attempt.
    let before = physical(&p.b.root, &p.b.root)?;
    let commits = commit_inventory(&b)?;
    assert!(matches!(
        p.resolve(
            &p.b.service,
            resolution_request(
                &p.b,
                request.operation_id,
                OperationId::new(),
                &inspection,
                vec![ticket_source("Resolved title", RESOLVED).into_bytes(); plan.conflicts],
            ),
        ),
        Err(SynchronizationError::ExternalResolutionRequired {
            target: SynchronizationTarget::Primary,
            operation_id,
        }) if operation_id == request.operation_id
    ));
    assert!(physical(&p.b.root, &p.b.root)? == before);
    assert_eq!(commit_inventory(&b)?, commits);
    let attempts: i64 = fixed(p.b.db()?.query_row(
        "SELECT count(*) FROM remote_resolution_attempts",
        [],
        |row| row.get(0),
    ))?;
    assert_eq!(attempts, 0);
    if false_repairs {
        refused_false_repairs(p, &b, &request, local, incoming, &plan)?;
        assert!(physical(&p.b.root, &p.b.root)? == before);
    }
    let repaired = commit_object(&b, &[local, incoming], &changes(&plan.repair))?;
    install_repair(&b, repaired, cleanup)?;
    assert_eq!(merge_heads(&p.b.root)?.is_empty(), cleanup);
    let n = p.server.receive_updates().len();
    outcome(
        fixed(p.sync(&p.b, restart(&request)))?,
        true,
        SynchronizationTarget::Primary,
        repaired,
    );
    one_update(&p.server, n, MAIN, incoming, repaired);
    // The service retired only its own recorded merge metadata.
    assert!(merge_heads(&p.b.root)?.is_empty());
    let b = p.b.repo()?;
    assert_eq!(b.state(), git2::RepositoryState::Clean);
    assert!(fixed(b.statuses(None))?.is_empty());
    assert_eq!(fixed(b.refname_to_id(MAIN))?, repaired);
    assert_eq!(
        merge_commits(&b)?,
        merges + 1 + 2 * usize::from(false_repairs)
    );
    for (path, change) in &plan.repair {
        assert!(blob_at(&b, repaired, path)? == change.as_ref().map(|(bytes, _)| bytes.clone()));
    }
    assert_eq!(pull_peer(a, &p.server)?, repaired);
    Ok(())
}
/// A4. Repairs that are not a whole committed merge of the recorded parents
/// are refused without network, and undoing each leaves the same conflict
/// pending and still repairable.
fn refused_false_repairs(
    p: &Pair,
    b: &git2::Repository,
    request: &SynchronizeRemoteRequest,
    local: git2::Oid,
    incoming: git2::Oid,
    plan: &ExternalPlan,
) -> Result<(), FixtureError> {
    let file = p.b.root.join(HOSTILE_PATH);
    let index_file = b.path().join("index");
    let marked = fixed(std::fs::read(&file))?;
    let conflicted = fixed(std::fs::read(&index_file))?;
    let auth = p.server.accepted_keys().len();
    let receives = p.server.receive_updates();
    let still_pending = || {
        matches!(
            p.sync(&p.b, restart(request)),
            Err(SynchronizationError::ConflictPending { operation_id, .. })
                if operation_id == request.operation_id
        )
    };
    let recovery = || {
        matches!(
            p.sync(&p.b, restart(request)),
            Err(SynchronizationError::RecoveryRequired)
        )
    };
    // Markers removed in the worktree only: the conflict is still pending and
    // the operator's bytes are left exactly as written.
    fixed(std::fs::write(&file, b"repaired code\n"))?;
    assert!(still_pending());
    assert_eq!(fixed(std::fs::read(&file))?, b"repaired code\n");
    assert!(fixed(std::fs::read(&index_file))? == conflicted);
    // The path staged without any commit.
    {
        let mut index = fixed(b.index())?;
        fixed(index.add_path(Path::new(HOSTILE_PATH)))?;
        fixed(index.write())?;
    }
    let staged = fixed(std::fs::read(&index_file))?;
    assert!(recovery());
    assert!(fixed(std::fs::read(&index_file))? == staged);
    assert_eq!(fixed(std::fs::read(&file))?, b"repaired code\n");
    assert_eq!(merge_heads(&p.b.root)?, [incoming]);
    fixed(std::fs::write(&index_file, &conflicted))?;
    fixed(std::fs::write(&file, &marked))?;
    assert!(still_pending());
    // Fully installed, clean commits that are not the recorded merge: one
    // parent, reversed parents, and a foreign second parent. Only the commit
    // graph tells them from a genuine repair.
    let base = fixed(b.merge_base(local, incoming))?;
    for parents in [vec![local], vec![incoming, local], vec![local, base]] {
        let wrong = commit_object(b, &parents, &changes(&plan.repair))?;
        install_repair(b, wrong, false)?;
        assert!(fixed(b.statuses(None))?.is_empty());
        let installed = fixed(std::fs::read(&index_file))?;
        assert!(recovery());
        assert_eq!(fixed(b.refname_to_id(MAIN))?, wrong);
        assert_eq!(merge_heads(&p.b.root)?, [incoming]);
        assert!(fixed(std::fs::read(&index_file))? == installed);
        assert_eq!(fixed(std::fs::read(&file))?, b"repaired code\n");
        // The operator undoes it: branch, index and worktree as they were.
        fixed(b.reference(MAIN, local, true, "false repair undone"))?;
        fixed(std::fs::write(&index_file, &conflicted))?;
        fixed(std::fs::write(&file, &marked))?;
    }
    assert!(still_pending());
    assert_eq!(p.server.accepted_keys().len(), auth);
    assert_eq!(p.server.receive_updates(), receives);
    Ok(())
}
fn code_conflict_false_repairs_then_external_repair() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    external_round(&p, &p.a.repo()?, External::Code, false, true)
}
fn binary_delete_and_rename_conflicts_externally_repaired() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let a = p.a.repo()?;
    external_round(&p, &a, External::Binary, true, false)?;
    external_round(&p, &a, External::Delete, false, false)?;
    external_round(&p, &a, External::Rename, true, false)
}
fn symlink_and_mixed_conflicts_externally_repaired() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let a = p.a.repo()?;
    external_round(&p, &a, External::Symlink, false, false)?;
    external_round(&p, &a, External::Mixed, true, false)
}

/// A second owned endpoint that trusts the same client key.
fn destination(p: &Pair) -> Result<SshRemoteFixture, FixtureError> {
    let destination = SshRemoteFixture::start()?;
    destination.allow_client_public_key(fixed(russh::keys::PublicKey::from_bytes(
        &p.server.allowed_client_public_key(),
    ))?);
    p.add_probe(destination.url().into_bytes())?;
    Ok(destination)
}
fn approve(
    p: &Pair,
    direction: SshDirection,
    endpoint: &SshRemoteFixture,
) -> Result<(), FixtureError> {
    fixed(p.b.service.verify_ssh_transport(
        VerifySshTransportRequest {
            root: p.b.root.clone(),
            direction,
            approval: Some(HostApproval {
                authority: SshAuthority {
                    host: "127.0.0.1".into(),
                    port: endpoint.address().port(),
                },
                expected: None,
                presented: endpoint.host_identity(),
            }),
        },
        &mut SessionCredentials::new(Provider),
    ))?;
    Ok(())
}

/// A5, A6. A resolved two-clone merge loses the receive race. The resolution
/// checkpoint is kept as the one candidate, nothing is pushed blindly, and
/// the deliberate retry observes the Push endpoint again and continues.
fn resolved_merge_receive_race_keeps_one_candidate() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let conflict = primary_ticket_conflict(&p, &p.a.repo()?, p.primary(&p.b))?;
    let (candidate, results) = resolve_all(&p, &conflict)?;
    resolved_locally(
        &p.b.root,
        MAIN,
        &conflict,
        candidate,
        &[(TICKET_PATH, &results[0])],
    )?;
    let server = p.bare()?;
    let race = commit_object(
        &server,
        &[conflict.incoming],
        &[("race.txt", Some((b"race\n", REGULAR)))],
    )?;
    p.server.race_primary_update(conflict.incoming, race)?;
    let b = p.b.repo()?;
    let n = p.server.receive_updates().len();
    assert!(matches!(
        p.sync(&p.b, restart(&conflict.request)),
        Err(SynchronizationError::PushRejected)
    ));
    let updates = p.server.receive_updates();
    assert_eq!(updates.len(), n + 1);
    assert_eq!(
        updates[n],
        ReceiveUpdate {
            reference: MAIN.into(),
            old_oid: conflict.incoming,
            new_oid: candidate,
            accepted: false
        }
    );
    assert_eq!(fixed(server.refname_to_id(MAIN))?, race);
    assert_eq!(fixed(b.refname_to_id(MAIN))?, candidate);
    assert_eq!(merge_commits(&b)?, 1);
    // No blind repush: the same ID without restart is Recovery, offline.
    let before = physical(&p.b.root, &p.b.root)?;
    let auth = p.server.accepted_keys().len();
    assert!(matches!(
        p.sync(&p.b, conflict.request.clone()),
        Err(SynchronizationError::RecoveryRequired)
    ));
    assert_eq!(p.server.accepted_keys().len(), auth);
    assert_eq!(p.server.receive_updates().len(), n + 1);
    assert!(physical(&p.b.root, &p.b.root)? == before);
    // The deliberate retry needs a new observation of the Push endpoint, and
    // builds on the retained resolution instead of regenerating it.
    let advertisements = p.server.receive_advertisements();
    let continued = published(fixed(p.sync(&p.b, restart(&conflict.request)))?)?;
    assert!(p.server.receive_advertisements() > advertisements);
    assert_eq!(parents(&b, continued)?, [candidate, race]);
    assert_eq!(parents(&b, candidate)?, [conflict.local, conflict.incoming]);
    assert_eq!(merge_commits(&b)?, 2);
    one_update(&p.server, n + 1, MAIN, race, continued);
    assert!(blob_at(&b, continued, TICKET_PATH)?.as_deref() == Some(results[0].as_slice()));
    assert_eq!(fixed(std::fs::read(p.b.root.join("race.txt")))?, b"race\n");
    assert!(fixed(b.statuses(None))?.is_empty());
    Ok(())
}

/// A5, A6. The push of a resolved merge is accepted but its outcome is lost:
/// first by a disconnect after receive-pack, then by a failed persistence of
/// the verified checkpoint. Each restart proves the candidate from a new
/// endpoint observation and never pushes or commits again.
fn resolved_merge_ambiguous_acceptance_and_persistence() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let a = p.a.repo()?;
    let b = p.b.repo()?;
    for disconnect in [true, false] {
        let merges = merge_commits(&b)?;
        let conflict = primary_ticket_conflict(&p, &a, p.primary(&p.b))?;
        let (candidate, _) = resolve_all(&p, &conflict)?;
        let n = p.server.receive_updates().len();
        let accepted_before = accepted(&p.server);
        let db = p.b.db()?;
        if disconnect {
            p.server.disconnect_at(FixtureBoundary::AfterReceivePack);
        } else {
            fixed(db.execute_batch("CREATE TRIGGER fail_verify BEFORE UPDATE ON remote_operation_records WHEN NEW.sync_checkpoint='push_verified' BEGIN SELECT RAISE(ABORT,'fixed failure'); END"))?;
        }
        // A lost transport outcome, or a failed durable write: never a
        // push rejection and never a success.
        let lost = p.sync(&p.b, restart(&conflict.request));
        assert!(if disconnect {
            matches!(lost, Err(SynchronizationError::Transport(_)))
        } else {
            matches!(
                lost,
                Err(SynchronizationError::Repository(RepositoryError {
                    kind: RepositoryErrorKind::RecoveryRequired,
                    ..
                }))
            )
        });
        if disconnect {
            assert!(p.server.receive_status_withheld());
            p.server.clear_fault();
        } else {
            fixed(db.execute_batch("DROP TRIGGER fail_verify"))?;
        }
        // The one candidate was accepted exactly once.
        assert_eq!(fixed(p.bare()?.refname_to_id(MAIN))?, candidate);
        one_update(&p.server, n, MAIN, conflict.incoming, candidate);
        assert_eq!(accepted(&p.server), accepted_before + 1);
        let before = physical(&p.b.root, &p.b.root)?;
        let commits = commit_inventory(&p.bare()?)?;
        let local_commits = commit_inventory(&b)?;
        let auth = p.server.accepted_keys().len();
        assert!(matches!(
            p.sync(&p.b, conflict.request.clone()),
            Err(SynchronizationError::RecoveryRequired)
        ));
        assert_eq!(p.server.accepted_keys().len(), auth);
        // A later process: a necessary new observation, no repeated mutation.
        let advertisements = p.server.receive_advertisements();
        outcome(
            fixed(p.sync_with(&p.b.reopened()?, restart(&conflict.request)))?,
            true,
            SynchronizationTarget::Primary,
            candidate,
        );
        assert!(p.server.accepted_keys().len() > auth);
        assert!(p.server.receive_advertisements() > advertisements);
        assert_eq!(p.server.receive_updates().len(), n + 1);
        assert_eq!(accepted(&p.server), accepted_before + 1);
        assert_eq!(commit_inventory(&p.bare()?)?, commits);
        assert_eq!(commit_inventory(&b)?, local_commits);
        assert!(physical(&p.b.root, &p.b.root)? == before);
        assert_eq!(merge_commits(&b)?, merges + 1);
        assert_eq!(pull_peer(&a, &p.server)?, candidate);
    }
    Ok(())
}

/// A6. Fetch and Push are distinct endpoints. The conflict comes from Fetch,
/// the resolved merge is published to Push only, and neither endpoint's
/// state is inferred from the other.
fn resolved_merge_distinct_push_remote() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let b = p.b.repo()?;
    let base = fixed(b.refname_to_id(MAIN))?;
    let push = destination(&p)?;
    let push_repo = fixed(git2::Repository::open_bare(push.repository_path()))?;
    let source = fixed(b.odb())?;
    let target = fixed(push_repo.odb())?;
    let mut objects = Vec::new();
    fixed(source.foreach(|oid| {
        objects.push(*oid);
        true
    }))?;
    for oid in objects {
        let object = fixed(source.read(oid))?;
        assert_eq!(fixed(target.write(object.kind(), object.data()))?, oid);
    }
    fixed(push_repo.reference(MAIN, base, true, "owned shared ancestry"))?;
    fixed(b.remote_set_pushurl("origin", Some(&push.url())))?;
    approve(&p, SshDirection::Fetch, &p.server)?;
    approve(&p, SshDirection::Push, &push)?;
    let mut request = p.primary(&p.b);
    request.approval = None;
    let conflict = primary_ticket_conflict(&p, &p.a.repo()?, request)?;
    assert!(push.receive_updates().is_empty());
    let (candidate, results) = resolve_all(&p, &conflict)?;
    let fetch_updates = p.server.receive_updates();
    outcome(
        fixed(p.sync(&p.b, restart(&conflict.request)))?,
        true,
        SynchronizationTarget::Primary,
        candidate,
    );
    // One accepted update at the Push endpoint, from its own advertised tip.
    one_update(&push, 0, MAIN, base, candidate);
    assert_eq!(fixed(push_repo.refname_to_id(MAIN))?, candidate);
    assert!(blob_at(&push_repo, candidate, TICKET_PATH)?.as_deref() == Some(results[0].as_slice()));
    // The Fetch endpoint received nothing and still holds A's tip.
    assert_eq!(p.server.receive_updates(), fetch_updates);
    assert_eq!(fixed(p.bare()?.refname_to_id(MAIN))?, conflict.incoming);
    assert_eq!(
        fixed(b.refname_to_id("refs/remotes/origin/main"))?,
        conflict.incoming
    );
    assert_eq!(merge_commits(&b)?, 1);
    p.privacy()
}

/// A6. The remote context is deleted after the conflict was resolved locally.
/// Publication never recreates it and the resolution stays intact.
fn resolved_context_merge_deleted_context_is_not_recreated() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let conflict = ticket_conflict(&p)?;
    let (candidate, _) = resolve_all(&p, &conflict)?;
    fixed(fixed(p.bare()?.find_reference(TICKET_REF))?.delete())?;
    let worktree = p.b.worktree(TICKET);
    let before = physical(&p.b.root, &worktree)?;
    let n = p.server.receive_updates().len();
    for _ in 0..2 {
        assert!(matches!(
            p.sync(&p.b, restart(&conflict.request)),
            Err(SynchronizationError::RemoteContextDeleted)
        ));
        assert!(physical(&p.b.root, &worktree)? == before);
        assert_eq!(p.server.receive_updates().len(), n);
        assert!(p.bare()?.find_reference(TICKET_REF).is_err());
        assert_eq!(fixed(p.b.repo()?.refname_to_id(TICKET_REF))?, candidate);
        assert_eq!(merge_commits(&p.b.repo()?)?, 1);
    }
    Ok(())
}

/// A6, A8. The publication endpoint changes after the caller inspected and
/// resolved the conflict. The old generation's operation is fenced before
/// any connection; the resolution is neither published nor lost.
fn endpoint_change_after_conflict_interaction_fences_restart() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let b = p.b.repo()?;
    // Trust a second destination up front, so a later switch to it could
    // only be refused by the configuration-generation fence.
    let other = destination(&p)?;
    fixed(b.remote_set_pushurl("origin", Some(&other.url())))?;
    approve(&p, SshDirection::Push, &other)?;
    fixed(b.remote_set_pushurl("origin", None))?;
    let conflict = primary_ticket_conflict(&p, &p.a.repo()?, p.primary(&p.b))?;
    let inspection = p.inspect(&p.b, conflict.request.operation_id)?;
    let sides = p.sides(&p.b, &inspection)?;
    fixed(b.remote_set_pushurl("origin", Some(&other.url())))?;
    // Resolution is local: it is bound to the observation, not the endpoint.
    let Ok(ResolveSynchronizationOutcome::LocalCheckpointComplete {
        commit_oid: candidate,
    }) = p.resolve(
        &p.b.service,
        resolution_request(
            &p.b,
            conflict.request.operation_id,
            OperationId::new(),
            &inspection,
            vec![resolution_of(
                sides[0].local.as_ref().unwrap(),
                BETA,
                "Resolved title",
            )],
        ),
    )
    else {
        return Err(FixtureError);
    };
    let before = physical(&p.b.root, &p.b.root)?;
    let n = p.server.receive_updates().len();
    let authenticated = other.accepted_keys().len();
    let original = p.server.accepted_keys().len();
    for _ in 0..2 {
        // The generation fence itself: never a transport or push outcome.
        assert!(matches!(
            p.sync(&p.b, restart(&conflict.request)),
            Err(SynchronizationError::Repository(RepositoryError {
                kind: RepositoryErrorKind::RecoveryRequired,
                ..
            }))
        ));
        // Refused before any connection to either endpoint.
        assert_eq!(other.accepted_keys().len(), authenticated);
        assert_eq!(p.server.accepted_keys().len(), original);
        let generations: (i64, i64) = fixed(p.b.db()?.query_row(
            "SELECT (SELECT configuration_generation FROM remote_operation_records WHERE operation_ulid=?1),(SELECT configuration_generation FROM remote_polling_state)",
            [conflict.request.operation_id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ))?;
        assert!(generations.0 < generations.1);
        assert!(physical(&p.b.root, &p.b.root)? == before);
        assert_eq!(p.server.receive_updates().len(), n);
        assert!(other.receive_updates().is_empty());
        assert_eq!(fixed(b.refname_to_id(MAIN))?, candidate);
        assert_eq!(merge_commits(&b)?, 1);
    }
    Ok(())
}

const DROP_SEAM: &str = "DROP TRIGGER seam";
/// Local commits, branch parents, remote effects and endpoint observations.
type Inventory = (usize, Vec<git2::Oid>, usize, git2::Oid);
fn inventory(p: &Pair, reference: &str, since: usize) -> Result<Inventory, FixtureError> {
    let b = p.b.repo()?;
    let head = fixed(b.refname_to_id(reference))?;
    Ok((
        merge_commits(&b)?,
        parents(&b, head)?,
        p.server.receive_updates().len() - since,
        fixed(p.bare()?.refname_to_id(reference))?,
    ))
}
/// A5, A7. One resolved two-clone conflict is stopped at each durable seam in
/// turn: the resolution checkpoint, then the `push_prepared`, `push_returned`,
/// `push_verified` and `discovery_pending` checkpoints, then the completion
/// of the discovery refresh. Every restart completes only what is missing:
/// one resolution commit, one push, and finally an index-only replay with the
/// transport disconnected. Run for the primary target and for the shared
/// context branch.
fn primary_resolution_and_publication_seams_inventory() -> Result<(), FixtureError> {
    seams_inventory(false)
}
fn context_resolution_and_publication_seams_inventory() -> Result<(), FixtureError> {
    seams_inventory(true)
}
fn seams_inventory(context: bool) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let conflict = if context {
        ticket_conflict(&p)?
    } else {
        primary_ticket_conflict(&p, &p.a.repo()?, p.primary(&p.b))?
    };
    let reference = if context { TICKET_REF } else { MAIN };
    let worktree = if context {
        p.b.worktree(TICKET)
    } else {
        p.b.root.clone()
    };
    let (local, incoming) = (conflict.local, conflict.incoming);
    let operation = conflict.request.operation_id;
    let db = p.b.db()?;
    let inspection = p.inspect(&p.b, operation)?;
    let sides = p.sides(&p.b, &inspection)?;
    let resolved = resolution_of(sides[0].local.as_ref().unwrap(), BETA, "Resolved title");
    let attempt = OperationId::new();
    let submit = || {
        p.resolve(
            &p.b.service,
            resolution_request(
                &p.b,
                operation,
                attempt,
                &inspection,
                vec![resolved.clone()],
            ),
        )
    };
    let n = p.server.receive_updates().len();
    let base = fixed(p.b.repo()?.merge_base(local, incoming))?;
    assert_eq!(inventory(&p, reference, n)?, (0, vec![base], 0, incoming));

    // Seam: the resolution attempt stops before its checkpoint is recorded.
    fixed(db.execute_batch("CREATE TRIGGER seam BEFORE UPDATE ON remote_resolution_attempts WHEN NEW.phase='applied' BEGIN SELECT RAISE(ABORT,'fixed failure'); END"))?;
    assert!(matches!(
        submit(),
        Err(SynchronizationError::Repository(RepositoryError {
            kind: RepositoryErrorKind::RecoveryRequired,
            ..
        }))
    ));
    fixed(db.execute_batch(DROP_SEAM))?;
    // The two-parent commit already exists; only its record is missing.
    let stopped = (1, vec![local, incoming], 0, incoming);
    assert_eq!(inventory(&p, reference, n)?, stopped);
    let installed = fixed(p.b.repo()?.refname_to_id(reference))?;
    let commits = commit_inventory(&p.b.repo()?)?;
    // The identical attempt records that same commit and makes no other.
    let Ok(ResolveSynchronizationOutcome::LocalCheckpointComplete {
        commit_oid: candidate,
    }) = submit()
    else {
        return Err(FixtureError);
    };
    assert_eq!(candidate, installed);
    assert_eq!(commit_inventory(&p.b.repo()?)?, commits);
    assert_eq!(inventory(&p, reference, n)?, stopped);
    resolved_locally(
        &worktree,
        reference,
        &conflict,
        candidate,
        &[(TICKET_PATH, &resolved)],
    )?;

    // Seams: each legacy publication checkpoint of the restarted operation.
    for (checkpoint, pushed) in [
        ("push_prepared", false),
        ("push_returned", true),
        ("push_verified", true),
        ("discovery_pending", true),
    ] {
        fixed(db.execute_batch(&format!("CREATE TRIGGER seam BEFORE UPDATE ON remote_operation_records WHEN NEW.sync_checkpoint='{checkpoint}' AND OLD.sync_checkpoint IS NOT '{checkpoint}' BEGIN SELECT RAISE(ABORT,'fixed failure'); END")))?;
        let advertisements = p.server.receive_advertisements();
        assert!(matches!(
            p.sync_with(&p.b.reopened()?, restart(&conflict.request)),
            Err(SynchronizationError::Repository(RepositoryError {
                kind: RepositoryErrorKind::RecoveryRequired,
                ..
            }))
        ));
        fixed(db.execute_batch(DROP_SEAM))?;
        // Reaching the seam needed a new endpoint observation, not a new
        // commit or, once the push returned, another push.
        assert!(p.server.receive_advertisements() > advertisements);
        assert_eq!(
            inventory(&p, reference, n)?,
            (
                1,
                vec![local, incoming],
                usize::from(pushed),
                if pushed { candidate } else { incoming }
            )
        );
        if pushed {
            one_update(&p.server, n, reference, incoming, candidate);
        }
    }

    // Seam: verified publication whose discovery refresh does not complete.
    fixed(db.execute_batch("CREATE TRIGGER seam BEFORE UPDATE ON operation_records WHEN NEW.state='completed' AND NEW.action='refresh' BEGIN SELECT RAISE(ABORT,'fixed failure'); END"))?;
    let expected = SynchronizationOutcome::Published {
        target: conflict.request.target.clone(),
        oid: candidate,
    };
    assert_eq!(
        fixed(p.sync(&p.b, restart(&conflict.request)))?,
        SynchronizationResult::IndexPending(IndexPending::new(expected.clone()))
    );
    // The transport is gone from here on: only indexing is replayed.
    p.server.disconnect_at(FixtureBoundary::Handshake);
    let auth = p.server.accepted_keys().len();
    let commands = p.server.commands().len();
    let before = physical(&p.b.root, &worktree)?;
    assert_eq!(
        fixed(p.sync(&p.b, conflict.request.clone()))?,
        SynchronizationResult::IndexPending(IndexPending::new(expected.clone()))
    );
    fixed(db.execute_batch(DROP_SEAM))?;
    assert_eq!(
        fixed(p.sync_with(&p.b.reopened()?, conflict.request.clone()))?,
        SynchronizationResult::Complete(expected.clone())
    );
    assert_eq!(
        fixed(p.sync(&p.b, restart(&conflict.request)))?,
        SynchronizationResult::Complete(expected)
    );
    assert_eq!(p.server.accepted_keys().len(), auth);
    assert_eq!(p.server.commands().len(), commands);
    assert!(physical(&p.b.root, &worktree)? == before);
    assert_eq!(
        inventory(&p, reference, n)?,
        (1, vec![local, incoming], 1, candidate)
    );
    if context {
        // The replayed refresh discovered the resolved content.
        let (title, _, head) = discovered(&p.b, TICKET)?;
        assert_eq!((title.as_str(), head), ("Resolved title", Some(candidate)));
    }
    let refreshes: i64 = fixed(db.query_row(
        "SELECT COUNT(*) FROM operation_records WHERE operation_ulid=?1 AND action='refresh' AND state='completed'",
        [operation.to_string()],
        |row| row.get(0),
    ))?;
    assert_eq!(refreshes, 1);
    p.server.clear_fault();
    Ok(())
}

/// number, candidate_oid, phase of every appended publication attempt.
fn publication_attempts(p: &Pair) -> Result<Vec<(i64, Option<String>, String)>, FixtureError> {
    let db = p.b.db()?;
    let mut statement = fixed(db.prepare(
        "SELECT number,candidate_oid,phase FROM remote_publication_attempts ORDER BY number",
    ))?;
    let rows = fixed(statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))))?;
    fixed(rows.collect::<Result<Vec<_>, _>>())
}
/// A5, A6. A context-target merge candidate is displaced by a competing
/// write to the shared context branch. Its continuation is stopped when the
/// appended publication attempt records the returned push; the next restart
/// proves that one push and converges: one continuation merge, one accepted
/// push, one attempt row.
fn context_candidate_race_continuation_stop() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let a_tip = p.save_ticket(&p.a, "Alpha title", ALPHA)?;
    assert_eq!(published(fixed(p.sync(&p.a, p.ticket(&p.a)))?)?, a_tip);
    let b_tip = p.submit_comment(&p.b, BETA)?;
    let server = p.bare()?;
    let race = commit_object(
        &server,
        &[a_tip],
        &[("race.txt", Some((b"race\n", REGULAR)))],
    )?;
    p.server.race_update(TICKET_REF, a_tip, race)?;
    let n = p.server.receive_updates().len();
    let request = p.ticket(&p.b);
    assert!(matches!(
        p.sync(&p.b, request.clone()),
        Err(SynchronizationError::PushRejected)
    ));
    let b = p.b.repo()?;
    let merged = fixed(b.refname_to_id(TICKET_REF))?;
    assert_eq!(parents(&b, merged)?, [b_tip, a_tip]);
    let updates = p.server.receive_updates();
    assert_eq!(updates.len(), n + 1);
    assert_eq!(
        updates[n],
        ReceiveUpdate {
            reference: TICKET_REF.into(),
            old_oid: a_tip,
            new_oid: merged,
            accepted: false
        }
    );
    assert_eq!(fixed(server.refname_to_id(TICKET_REF))?, race);
    assert!(publication_attempts(&p)?.is_empty());
    assert_eq!(merge_commits(&b)?, 1);

    let db = p.b.db()?;
    fixed(db.execute_batch("CREATE TRIGGER seam BEFORE UPDATE ON remote_publication_attempts WHEN NEW.phase='returned' BEGIN SELECT RAISE(ABORT,'fixed failure'); END"))?;
    assert!(matches!(
        p.sync_with(&p.b.reopened()?, restart(&request)),
        Err(SynchronizationError::Repository(RepositoryError {
            kind: RepositoryErrorKind::RecoveryRequired,
            ..
        }))
    ));
    fixed(db.execute_batch(DROP_SEAM))?;
    // The continuation merge exists and was pushed once; only the record of
    // the returned push is missing.
    let continued = fixed(b.refname_to_id(TICKET_REF))?;
    assert_eq!(parents(&b, continued)?, [merged, race]);
    assert_eq!(merge_commits(&b)?, 2);
    one_update(&p.server, n + 1, TICKET_REF, race, continued);
    assert_eq!(
        publication_attempts(&p)?,
        [(1, Some(continued.to_string()), "prepared".to_owned())]
    );
    let worktree = p.b.worktree(TICKET);
    let before = physical(&p.b.root, &worktree)?;
    let commits = commit_inventory(&p.bare()?)?;
    outcome(
        fixed(p.sync_with(&p.b.reopened()?, restart(&request)))?,
        true,
        ticket_target(),
        continued,
    );
    assert_eq!(p.server.receive_updates().len(), n + 2);
    assert_eq!(commit_inventory(&p.bare()?)?, commits);
    assert!(physical(&p.b.root, &worktree)? == before);
    assert_eq!(merge_commits(&b)?, 2);
    assert_eq!(
        publication_attempts(&p)?,
        [(1, Some(continued.to_string()), "verified".to_owned())]
    );
    assert_eq!(fixed(std::fs::read(worktree.join("race.txt")))?, b"race\n");
    assert!(fixed(std::fs::read_to_string(worktree.join(TICKET_PATH)))?.contains(ALPHA));
    assert!(fixed(std::fs::read_to_string(worktree.join(NEW_COMMENT_PATH)))?.contains(BETA));
    assert!(fixed(p.b.linked(TICKET)?.statuses(None))?.is_empty());
    let (_, _, head) = discovered(&p.b, TICKET)?;
    assert_eq!(head, Some(continued));
    Ok(())
}

/// One fixture commit that sets a single path given as raw bytes.
#[cfg(unix)]
fn commit_raw_path(
    repo: &git2::Repository,
    parents: &[git2::Oid],
    path: &[u8],
    bytes: &[u8],
) -> Result<git2::Oid, FixtureError> {
    let parents = parents
        .iter()
        .map(|oid| fixed(repo.find_commit(*oid)))
        .collect::<Result<Vec<_>, _>>()?;
    let mut index = fixed(git2::Index::new())?;
    fixed(index.read_tree(&fixed(parents[0].tree())?))?;
    fixed(index.add(&git2::IndexEntry {
        ctime: git2::IndexTime::new(0, 0),
        mtime: git2::IndexTime::new(0, 0),
        dev: 0,
        ino: 0,
        mode: REGULAR,
        uid: 0,
        gid: 0,
        file_size: bytes.len() as u32,
        id: fixed(repo.blob(bytes))?,
        flags: 0,
        flags_extended: 0,
        path: path.to_vec(),
    }))?;
    let tree = fixed(repo.find_tree(fixed(index.write_tree_to(repo))?))?;
    let sig = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
    fixed(repo.commit(
        None,
        &sig,
        &sig,
        "owned fixture commit",
        &tree,
        &parents.iter().collect::<Vec<_>>(),
    ))
}
/// A4, A8. Conflicts on hostile document paths. A path with an escape
/// sequence and a newline is still a valid canonical document path, so its
/// conflict is eligible and is resolved in-process; on Linux a path that is
/// not UTF-8 is external-only and is repaired as a whole merge that keeps
/// the recorded local entry. Neither path reaches a recovery row or a
/// rendered diagnostic.
#[cfg(unix)]
fn hostile_conflict_paths_are_redacted() -> Result<(), FixtureError> {
    use std::os::unix::ffi::OsStrExt;
    const FRAGMENT: &[u8] = b"hostile-control-canary-private";
    let p = Pair::new()?;
    let a = p.a.repo()?;
    let b = p.b.repo()?;
    let source = |body: &str| {
        canonical::serialize_item(&canonical::CanonicalItem::Document(canonical::Document {
            id: "01ERZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap(),
            title: "Hostile".into(),
            body: format!("{body}\n"),
            unknown: serde_yaml::Mapping::new(),
        }))
        .unwrap()
        .into_bytes()
    };
    #[allow(unused_mut)]
    let mut hostile = vec![(
        [b"docs/esc\x1b[31m\n".as_slice(), FRAGMENT, b".md"].concat(),
        ConflictEligibility::EligibleCanonical,
    )];
    // macOS filesystems refuse names that are not valid UTF-8.
    #[cfg(target_os = "linux")]
    hostile.push((
        [b"docs/bytes\xff\xfe".as_slice(), FRAGMENT, b".md"].concat(),
        ConflictEligibility::ExternalResolutionRequired,
    ));
    let mut attempts = 0;
    for (path, eligibility) in hostile {
        let external = eligibility == ConflictEligibility::ExternalResolutionRequired;
        // Row and rendering probes: the whole path and its readable part.
        // Discovery may legitimately index a document path elsewhere.
        p.paths.borrow_mut().push(path.clone());
        p.paths.borrow_mut().push(FRAGMENT.to_vec());
        p.save_probes()?;
        // A common base on both clones, then one edit on each.
        let head = fixed(a.refname_to_id(MAIN))?;
        let base = commit_raw_path(&a, &[head], &path, &source(SEED))?;
        install(&a, MAIN, base)?;
        push_peer(&a, &p.server, MAIN)?;
        outcome(
            fixed(p.sync(&p.b, p.primary(&p.b)))?,
            false,
            SynchronizationTarget::Primary,
            base,
        );
        let incoming = commit_raw_path(&a, &[base], &path, &source(ALPHA))?;
        install(&a, MAIN, incoming)?;
        push_peer(&a, &p.server, MAIN)?;
        let local = commit_raw_path(&b, &[base], &path, &source(BETA))?;
        install(&b, MAIN, local)?;
        let request = p.primary(&p.b);
        conflict_pending(&p, &request, SynchronizationStage::Primary)?;
        let inspection = p.inspect(&p.b, request.operation_id)?;
        assert_eq!(inspection.paths.len(), 1);
        assert!(inspection.paths[0].eligibility == eligibility);
        // Reading the sides renders nothing of the path either way.
        let read =
            p.b.service
                .read_synchronization_conflict(&inspection.paths[0].token);
        p.redacted(&match &read {
            Ok(sides) => format!("{sides:?}"),
            Err(error) => format!("{error:?} {error}"),
        })?;
        let before = physical(&p.b.root, &p.b.root)?;
        let mut resolved = source(RESOLVED);
        let submitted = p.resolve(
            &p.b.service,
            resolution_request(
                &p.b,
                request.operation_id,
                OperationId::new(),
                &inspection,
                vec![source(RESOLVED)],
            ),
        );
        let merged = if external {
            assert!(matches!(
                submitted,
                Err(SynchronizationError::ExternalResolutionRequired { operation_id, .. })
                    if operation_id == request.operation_id
            ));
            assert!(physical(&p.b.root, &p.b.root)? == before);
            // New bytes at a canonical-shaped path that is not UTF-8 are not
            // an acceptable repair; keeping the recorded local entry is.
            let rewritten = commit_raw_path(&b, &[local, incoming], &path, &source(RESOLVED))?;
            install_repair(&b, rewritten, false)?;
            let auth = p.server.accepted_keys().len();
            assert!(matches!(
                p.sync(&p.b, restart(&request)),
                Err(SynchronizationError::RecoveryRequired)
            ));
            assert_eq!(p.server.accepted_keys().len(), auth);
            resolved = source(BETA);
            let repaired = commit_raw_path(&b, &[local, incoming], &path, &resolved)?;
            install_repair(&b, repaired, false)?;
            repaired
        } else {
            let Ok(ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid }) =
                submitted
            else {
                return Err(FixtureError);
            };
            attempts += 1;
            commit_oid
        };
        assert_eq!(parents(&b, merged)?, [local, incoming]);
        let n = p.server.receive_updates().len();
        outcome(
            fixed(p.sync(&p.b, restart(&request)))?,
            true,
            SynchronizationTarget::Primary,
            merged,
        );
        one_update(&p.server, n, MAIN, incoming, merged);
        // The resolved bytes are at the hostile path itself, nowhere else.
        let file = p.b.root.join(std::ffi::OsStr::from_bytes(&path));
        assert!(fixed(std::fs::read(file))? == resolved);
        assert!(fixed(b.statuses(None))?.is_empty());
        assert_eq!(pull_peer(&a, &p.server)?, merged);
    }
    // The in-process resolution recorded its attempt and path by digest.
    let [_, rows] = p.scanned_rows()?;
    assert_eq!(rows.get("remote_resolution_attempts"), Some(&attempts));
    assert_eq!(rows.get("remote_resolution_paths"), Some(&attempts));
    Ok(())
}

fn cancellation_state(
    p: &Pair,
    operation: OperationId,
) -> Result<(String, i64, String), FixtureError> {
    fixed(p.b.db()?.query_row(
        "SELECT phase,cancel_requested,(SELECT group_concat(phase) FROM remote_integration_steps) FROM remote_operation_records WHERE operation_ulid=?1",
        [operation.to_string()],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ))
}
/// A8. A cancellation that lands while a conflict is pending stops that owner
/// without ending the operation: the same operation still restarts into its
/// conflict, resolves it and publishes.
fn cancelled_pending_conflict_stays_recoverable() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let (local, incoming) = diverge_primary_ticket(&p, &p.a.repo()?)?;
    let request = p.primary(&p.b);
    let operation = request.operation_id;
    let db = p.b.db()?;
    let recoverable = ("interrupted".to_owned(), 0, "conflict_pending".to_owned());
    // The request becomes durable while the first conflicted merge is being
    // installed, after the last boundary that could have honoured it.
    // Each trigger also leaves a marker row, so the case proves it fired.
    fixed(db.execute_batch("CREATE TABLE fixture_marker(seam TEXT NOT NULL)"))?;
    let fired = |seam: &str| -> Result<i64, FixtureError> {
        fixed(db.query_row(
            "SELECT count(*) FROM fixture_marker WHERE seam=?1",
            [seam],
            |row| row.get(0),
        ))
    };
    fixed(db.execute_batch("CREATE TRIGGER seam AFTER UPDATE OF phase ON remote_integration_steps WHEN NEW.phase='applying' BEGIN INSERT INTO fixture_marker VALUES('merge'); UPDATE remote_operation_records SET cancel_requested=1; END"))?;
    conflict_pending(&p, &request, SynchronizationStage::Primary)?;
    fixed(db.execute_batch(DROP_SEAM))?;
    assert_eq!(fired("merge")?, 1);
    assert_eq!(cancellation_state(&p, operation)?, recoverable);
    let conflict = Conflict {
        request,
        local,
        incoming,
    };
    conflict_installed(&p.b.root, MAIN, &conflict, &[TICKET_PATH])?;
    let before = physical(&p.b.root, &p.b.root)?;
    let auth = p.server.accepted_keys().len();
    // A public cancel of the released operation is accepted and changes nothing.
    fixed(p.b.service.cancel_remote_operation(&p.b.root, operation))?;
    assert_eq!(cancellation_state(&p, operation)?, recoverable);
    // A cancel that becomes durable while a deliberate restart owns the
    // operation is honoured at its next boundary as a non-terminal stop.
    fixed(db.execute_batch("CREATE TRIGGER seam AFTER UPDATE OF owner_epoch ON remote_operation_records WHEN NEW.owner_epoch!=OLD.owner_epoch BEGIN INSERT INTO fixture_marker VALUES('restart'); UPDATE remote_operation_records SET cancel_requested=1 WHERE id=NEW.id; END"))?;
    assert!(matches!(
        p.sync(&p.b, restart(&conflict.request)),
        Err(SynchronizationError::Interrupted)
    ));
    fixed(db.execute_batch(DROP_SEAM))?;
    assert_eq!(fired("restart")?, 1);
    assert_eq!(cancellation_state(&p, operation)?, recoverable);
    assert!(physical(&p.b.root, &p.b.root)? == before);
    // Not terminal: a later process restarts the same operation into the
    // same conflict, offline, and resolves it.
    let later = p.b.reopened()?;
    assert!(matches!(
        p.sync_with(&later, restart(&conflict.request)),
        Err(SynchronizationError::ConflictPending { operation_id, .. }) if operation_id == operation
    ));
    assert!(physical(&p.b.root, &p.b.root)? == before);
    assert_eq!(p.server.accepted_keys().len(), auth);
    let n = p.server.receive_updates().len();
    let (candidate, results) = resolve_all(&p, &conflict)?;
    resolved_locally(
        &p.b.root,
        MAIN,
        &conflict,
        candidate,
        &[(TICKET_PATH, &results[0])],
    )?;
    outcome(
        fixed(p.sync_with(&later, restart(&conflict.request)))?,
        true,
        SynchronizationTarget::Primary,
        candidate,
    );
    one_update(&p.server, n, MAIN, incoming, candidate);
    assert_eq!(merge_commits(&p.b.repo()?)?, 1);
    Ok(())
}

/// A8. Body, credential, endpoint and hostile-path canaries through a mixed
/// conflict, a refused resolution, an external repair, a hostile receiver
/// rejection and publication: nothing reaches a recovery row, the live WAL, a
/// rollback journal, a diagnostic backup or any rendered diagnostic.
fn recovery_privacy_canaries() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let a = p.a.repo()?;
    let b = p.b.repo()?;
    let db = p.b.db()?;
    // Real synchronization writes with live WAL retention on this owned store.
    fixed(db.execute_batch("PRAGMA journal_mode=WAL;"))?;
    // First a canonical conflict, its resolution, a displaced push and the
    // continuation, so that every recovery table holds rows when scanned.
    let conflict = primary_ticket_conflict(&p, &a, p.primary(&p.b))?;
    let (candidate, _) = resolve_all(&p, &conflict)?;
    let race = commit_object(
        &p.bare()?,
        &[conflict.incoming],
        &[("race.txt", Some((b"race\n", REGULAR)))],
    )?;
    p.server.race_primary_update(conflict.incoming, race)?;
    assert!(matches!(
        p.sync(&p.b, restart(&conflict.request)),
        Err(SynchronizationError::PushRejected)
    ));
    let continued = published(fixed(p.sync(&p.b, restart(&conflict.request)))?)?;
    assert_eq!(parents(&b, continued)?, [candidate, race]);
    assert_eq!(pull_peer(&a, &p.server)?, continued);
    let plan = external_plan(&b, External::Mixed)?;
    let incoming = advance(&a, MAIN, &changes(&plan.alpha))?;
    push_peer(&a, &p.server, MAIN)?;
    let local = advance(&b, MAIN, &changes(&plan.beta))?;
    let request = p.primary(&p.b);
    conflict_pending(&p, &request, SynchronizationStage::Primary)?;
    // The canaries are really present in the conflict the service handled.
    let marked = fixed(std::fs::read_to_string(p.b.root.join(TICKET_PATH)))?;
    assert!(marked.contains(ALPHA) && marked.contains(BETA));
    assert!(stages(&p.b.root, HOSTILE_PATH)?.iter().all(Option::is_some));
    let inspection = p.inspect(&p.b, request.operation_id)?;
    assert_eq!(inspection.paths.len(), 2);
    p.redacted(&format!("{:?}", inspection.observation))?;
    let sides = p.sides(&p.b, &inspection)?;
    assert!(sides.iter().any(|sides| {
        sides.local.as_ref().is_some_and(|bytes| {
            bytes
                .bytes()
                .windows(BETA.len())
                .any(|window| window == BETA.as_bytes())
        })
    }));
    assert!(matches!(
        p.resolve(
            &p.b.service,
            resolution_request(
                &p.b,
                request.operation_id,
                OperationId::new(),
                &inspection,
                vec![ticket_source("Resolved title", RESOLVED).into_bytes(); 2],
            ),
        ),
        Err(SynchronizationError::ExternalResolutionRequired { operation_id, .. })
            if operation_id == request.operation_id
    ));
    // Read surfaces of the pending operation.
    p.redacted(&format!(
        "{:?}",
        fixed(p.b.service.active_remote_operation(&p.b.root))?
    ))?;
    p.redacted(&format!(
        "{:?}",
        fixed(p.b.service.remote_snapshot(&p.b.root))?
    ))?;
    // Failure probes: a false repair, then a hostile receiver message.
    fixed(std::fs::write(
        p.b.root.join(HOSTILE_PATH),
        b"staged only\n",
    ))?;
    let conflicted = fixed(std::fs::read(b.path().join("index")))?;
    {
        let mut index = fixed(b.index())?;
        fixed(index.add_path(Path::new(HOSTILE_PATH)))?;
        fixed(index.write())?;
    }
    assert!(matches!(
        p.sync(&p.b, restart(&request)),
        Err(SynchronizationError::RecoveryRequired)
    ));
    fixed(std::fs::write(b.path().join("index"), &conflicted))?;
    let repaired = commit_object(&b, &[local, incoming], &changes(&plan.repair))?;
    install_repair(&b, repaired, false)?;
    p.server.reject_primary_updates(true)?;
    p.server.hostile_rejection(HOSTILE);
    let n = p.server.receive_updates().len();
    assert!(matches!(
        p.sync(&p.b, restart(&request)),
        Err(SynchronizationError::PushRejected)
    ));
    assert_eq!(p.server.receive_updates().len(), n + 1);
    assert!(!p.server.receive_updates()[n].accepted);
    p.server.reject_primary_updates(false)?;
    outcome(
        fixed(p.sync(&p.b, restart(&request)))?,
        true,
        SynchronizationTarget::Primary,
        repaired,
    );
    one_update(&p.server, n + 1, MAIN, incoming, repaired);
    assert!(fixed(std::fs::read_to_string(p.b.root.join(TICKET_PATH)))?.contains(RESOLVED));
    // A generated diagnostic backup with an open rollback journal, beside the
    // live WAL, is scanned as real files. Key, Git and canonical sources are
    // not application data.
    let backup = p.b.data.join("merge-recovery-diagnostic.sqlite");
    fixed(db.execute("VACUUM INTO ?1", [backup.to_string_lossy().as_ref()]))?;
    let diagnostic = fixed(rusqlite::Connection::open(&backup))?;
    fixed(diagnostic.execute_batch("PRAGMA journal_mode=DELETE; BEGIN IMMEDIATE; CREATE TABLE fixture_diagnostic(category TEXT); INSERT INTO fixture_diagnostic VALUES('fixed');"))?;
    let scanned = ssh_privacy::scan(&p.b.data, &p.probes.borrow())?;
    for suffix in ["-journal", "-wal"] {
        assert!(
            scanned
                .iter()
                .any(|(path, bytes)| path.to_string_lossy().ends_with(suffix) && *bytes > 0)
        );
    }
    assert!(
        scanned
            .iter()
            .any(|(path, bytes)| path == &backup && *bytes > 0)
    );
    // Every table the journey should have populated was scanned non-empty.
    let [_, rows] = p.scanned_rows()?;
    for table in [
        "operation_records",
        "remote_operation_records",
        "remote_observation_batches",
        "remote_ref_observations",
        "remote_integration_windows",
        "remote_integration_steps",
        "remote_integration_merge_metadata",
        "remote_resolution_attempts",
        "remote_resolution_paths",
        "remote_resolution_index_artifacts",
        "remote_resolution_ref_log_artifacts",
        "remote_publication_attempts",
    ] {
        assert!(rows.get(table).is_some_and(|scanned| *scanned > 0));
    }
    fixed(diagnostic.execute_batch("ROLLBACK"))?;
    Ok(())
}

// Only nested runner captures execute these controlled leaks. Their raw
// buffers never reach the outer runner or tool output.
const OUTPUT_CONTROLS: &[ssh_harness::Case] = &[
    ("probe_resolved_body_stdout", probe_resolved_body_stdout),
    (
        "probe_conflict_path_stderr_failure",
        probe_conflict_path_stderr_failure,
    ),
    ("probe_clean_failure", probe_clean_failure),
    ("probe_inventory_missing", probe_inventory_missing),
];
fn save_control_probes() -> Result<(), FixtureError> {
    ssh_privacy::save(&[b"runner-private-capture-control".to_vec()])
}
/// A real two-clone resolution whose caller bytes then leak to stdout. The
/// conflict is on the primary branch: a nested child's temporary root is one
/// isolation level deeper, and a context worktree below it would push the
/// longest canonical path past the Windows path limit.
fn probe_resolved_body_stdout() -> Result<(), FixtureError> {
    use std::io::Write;
    let p = Pair::new()?;
    let conflict = primary_ticket_conflict(&p, &p.a.repo()?, p.primary(&p.b))?;
    let (_, results) = resolve_all(&p, &conflict)?;
    fixed(std::io::stdout().write_all(&results[0]))
}
/// A failing case that leaks a conflict path and the endpoint to stderr.
fn probe_conflict_path_stderr_failure() -> Result<(), FixtureError> {
    use std::io::Write;
    let p = Pair::new()?;
    let mut bytes = vec![b'x'; 20_478];
    bytes.extend_from_slice(HOSTILE_PATH.as_bytes());
    bytes.extend_from_slice(p.server.url().as_bytes());
    fixed(std::io::stderr().write_all(&bytes))?;
    Err(FixtureError)
}
fn probe_clean_failure() -> Result<(), FixtureError> {
    save_control_probes()?;
    Err(FixtureError)
}
fn probe_inventory_missing() -> Result<(), FixtureError> {
    Ok(())
}
/// A8. Captured fixture output is scanned against the case's own canaries and
/// a missing canary inventory fails closed; failures name only a category.
fn recovery_capture_fails_closed() -> Result<(), FixtureError> {
    use ssh_harness::IsolationFailure as Failure;
    save_control_probes()?;
    let mut mismatched = false;
    for (control, (case, expected)) in [
        ("probe_resolved_body_stdout", Failure::OutputPrivacy),
        ("probe_conflict_path_stderr_failure", Failure::OutputPrivacy),
        ("probe_clean_failure", Failure::Child),
        ("probe_inventory_missing", Failure::ProbeInventory),
    ]
    .into_iter()
    .enumerate()
    {
        let actual = ssh_harness::run_isolated_with_output_privacy(case);
        if actual != Err(expected) {
            // Diagnosable without private bytes: which control (by position)
            // and which category came back (0 is an unexpected success).
            ssh_harness::observation(&[
                control as u128,
                match actual {
                    Ok(()) => 0,
                    Err(Failure::Fixture) => 1,
                    Err(Failure::Child) => 2,
                    Err(Failure::ProbeInventory) => 3,
                    Err(Failure::OutputPrivacy) => 4,
                },
            ]);
            mismatched = true;
        }
    }
    assert!(!mismatched);
    Ok(())
}
