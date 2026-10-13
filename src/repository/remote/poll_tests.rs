use super::super::{
    AutomaticBackoff, PollingInterval, RemoteOutcomeCategory, RemotePollInvocation,
    RemotePollingConfiguration,
};
use super::super::{RemoteRefPlan, SynchronizationError, SynchronizationTarget};
use super::*;
use crate::repository::AuthoringKind;
use crate::repository::{REGISTRY_FILE, RepositoryService};
use rusqlite::{Connection, params};
use std::{collections::BTreeMap, fs, path::PathBuf};
use time::OffsetDateTime;

fn at(seconds: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(seconds).unwrap()
}

fn policy(
    enabled: bool,
    paused: bool,
    suspended: bool,
    interval: u64,
) -> RemotePollingConfiguration {
    RemotePollingConfiguration::new(
        enabled,
        paused,
        PollingInterval::from_seconds(interval).unwrap(),
        None,
        suspended,
    )
}

#[test]
fn eligibility_policy_precedence_wins_over_due_and_future_deadlines() {
    for next in [None, Some(at(999)), Some(at(100_000))] {
        for (policy, reason) in [
            (policy(false, true, true, 300), PollIneligibility::Disabled),
            (policy(true, true, true, 300), PollIneligibility::Paused),
            (
                policy(true, false, true, 300),
                PollIneligibility::RecoverySuspended,
            ),
        ] {
            assert_eq!(
                eligibility(&policy, next, at(1000)),
                AutomaticPollEligibility::NotEligible {
                    reason,
                    next_eligible_at: next
                }
            );
        }
    }
}

#[test]
fn empty_past_and_equal_deadlines_are_due_and_future_is_not_due() {
    let policy = RemotePollingConfiguration::default();
    for next in [None, Some(at(999)), Some(at(1000))] {
        assert_eq!(
            eligibility(&policy, next, at(1000)),
            AutomaticPollEligibility::Eligible
        );
    }
    assert_eq!(
        eligibility(&policy, Some(at(1001)), at(1000)),
        AutomaticPollEligibility::NotEligible {
            reason: PollIneligibility::NotDue,
            next_eligible_at: Some(at(1001))
        }
    );
}

#[test]
fn rollback_clamp_uses_larger_of_interval_and_maximum_backoff() {
    for (interval, bound) in [(60, 1900), (300, 1900), (900, 1900), (3600, 4600)] {
        let policy = policy(true, false, false, interval);
        assert_eq!(
            eligibility(&policy, Some(at(100_000)), at(1000)),
            AutomaticPollEligibility::NotEligible {
                reason: PollIneligibility::NotDue,
                next_eligible_at: Some(at(bound))
            }
        );
        assert_eq!(
            eligibility(&policy, Some(at(bound)), at(1100)),
            AutomaticPollEligibility::NotEligible {
                reason: PollIneligibility::NotDue,
                next_eligible_at: Some(at(bound))
            }
        );
        assert_eq!(
            eligibility(&policy, Some(at(bound)), at(bound)),
            AutomaticPollEligibility::Eligible
        );
    }
}

#[test]
fn next_eligibility_result_table_covers_every_category_and_invocation() {
    let policy = RemotePollingConfiguration::default()
        .with_automatic_backoff(AutomaticBackoff::from_seconds(120).unwrap());
    let cases = [
        (
            RemoteOutcomeCategory::Completed,
            Some(at(1300)),
            Some(at(1300)),
        ),
        (
            RemoteOutcomeCategory::TransportUnavailable,
            Some(at(1240)),
            None,
        ),
        (
            RemoteOutcomeCategory::ProtocolRejected,
            Some(at(1240)),
            None,
        ),
        (
            RemoteOutcomeCategory::SelectedKeyUnavailable,
            Some(at(1300)),
            None,
        ),
        (RemoteOutcomeCategory::UnlockRequired, Some(at(1300)), None),
        (
            RemoteOutcomeCategory::HostApprovalRequired,
            Some(at(1300)),
            None,
        ),
        (
            RemoteOutcomeCategory::ConfigurationRequired,
            Some(at(1300)),
            None,
        ),
        (
            RemoteOutcomeCategory::RepositoryUnavailable,
            Some(at(1300)),
            None,
        ),
        (RemoteOutcomeCategory::Cancelled, None, None),
    ];
    for (category, automatic, explicit) in cases {
        for (invocation, expected) in [
            (RemotePollInvocation::Automatic, automatic),
            (RemotePollInvocation::Explicit, explicit),
        ] {
            assert_eq!(
                next_eligible(category, invocation, at(1000), &policy),
                expected,
                "{category:?} {invocation:?}"
            );
        }
    }
}

#[test]
fn network_retry_deadlines_start_at_sixty_double_and_cap_at_nine_hundred() {
    for category in [
        RemoteOutcomeCategory::TransportUnavailable,
        RemoteOutcomeCategory::ProtocolRejected,
    ] {
        let mut policy = RemotePollingConfiguration::default();
        for delay in [60, 120, 240, 480, 900, 900] {
            assert_eq!(
                next_eligible(category, RemotePollInvocation::Automatic, at(1000), &policy),
                Some(at(1000 + delay))
            );
            assert_eq!(
                next_eligible(category, RemotePollInvocation::Explicit, at(1000), &policy),
                None
            );
            policy = policy
                .with_automatic_backoff(AutomaticBackoff::from_seconds(delay as u64).unwrap());
        }
    }
}

fn fixture() -> (
    tempfile::TempDir,
    tempfile::TempDir,
    RepositoryService,
    Connection,
) {
    let data = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    git2::Repository::init(root.path()).unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection.execute("INSERT INTO repositories(root_path,enabled_at,accessibility,refresh_required) VALUES(?1,123,'accessible',0)", [root.path().canonicalize().unwrap().to_str().unwrap()]).unwrap();
    connection.execute_batch("CREATE TABLE eligibility_writes(n INTEGER NOT NULL); INSERT INTO eligibility_writes VALUES(0); CREATE TRIGGER count_eligibility_writes AFTER UPDATE ON remote_polling_state BEGIN UPDATE eligibility_writes SET n=n+1; END;").unwrap();
    (data, root, service, connection)
}

#[test]
fn stored_clamp_is_written_once_and_counts_down_without_reserving() {
    let (_data, root, service, connection) = fixture();
    connection.execute_batch("UPDATE remote_polling_state SET next_eligible_at=100000; UPDATE eligibility_writes SET n=0").unwrap();
    assert_eq!(
        automatic_poll_eligibility_at(&service, root.path(), at(1000)).unwrap(),
        AutomaticPollEligibility::NotEligible {
            reason: PollIneligibility::NotDue,
            next_eligible_at: Some(at(1900))
        }
    );
    assert_eq!(
        automatic_poll_eligibility_at(&service, root.path(), at(1100)).unwrap(),
        AutomaticPollEligibility::NotEligible {
            reason: PollIneligibility::NotDue,
            next_eligible_at: Some(at(1900))
        }
    );
    assert_eq!(
        automatic_poll_eligibility_at(&service, root.path(), at(1900)).unwrap(),
        AutomaticPollEligibility::Eligible
    );
    assert_eq!(
        connection
            .query_row("SELECT n FROM eligibility_writes", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM remote_operation_records", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
        0
    );
    let snapshot = service.remote_snapshot(root.path()).unwrap();
    assert_eq!(snapshot.latest_attempt_at(), None);
    assert_eq!(snapshot.next_eligible_at(), Some(at(1900)));
}

#[test]
fn ineligible_policy_writes_nothing_even_when_deadline_needs_clamping() {
    let (_data, root, service, connection) = fixture();
    for (enabled, paused, suspended, reason) in [
        (false, true, true, PollIneligibility::Disabled),
        (true, true, true, PollIneligibility::Paused),
        (true, false, true, PollIneligibility::RecoverySuspended),
    ] {
        connection.execute("UPDATE remote_polling_state SET enabled=?1,paused=?2,recovery_suspended=?3,next_eligible_at=100000", params![enabled, paused, suspended]).unwrap();
        connection
            .execute_batch("UPDATE eligibility_writes SET n=0")
            .unwrap();
        assert_eq!(
            automatic_poll_eligibility_at(&service, root.path(), at(1000)).unwrap(),
            AutomaticPollEligibility::NotEligible {
                reason,
                next_eligible_at: Some(at(100000))
            }
        );
        assert_eq!(
            connection
                .query_row("SELECT n FROM eligibility_writes", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}

#[test]
fn public_eligibility_is_due_without_an_endpoint_and_does_not_write() {
    let (_data, root, service, connection) = fixture();
    assert_eq!(
        service.automatic_poll_eligibility(root.path()).unwrap(),
        AutomaticPollEligibility::Eligible
    );
    assert_eq!(
        connection
            .query_row("SELECT n FROM eligibility_writes", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

const POLL_ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const OTHER_ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";

#[test]
fn poll_dispositions_have_only_fixed_guidance() {
    for (disposition, guidance) in [
        (PollDisposition::Current, "Local state is current."),
        (
            PollDisposition::FastForwarded,
            "Clean local state was fast-forwarded.",
        ),
        (
            PollDisposition::LocalAhead,
            "Publish local commits with deliberate synchronization.",
        ),
        (
            PollDisposition::Diverged,
            "Resolve divergent history with deliberate synchronization.",
        ),
        (
            PollDisposition::Dirty,
            "Save or otherwise handle local changes before polling again.",
        ),
        (
            PollDisposition::Conflicted,
            "Resolve the conflict or in-progress Git operation before polling again.",
        ),
        (
            PollDisposition::Inaccessible,
            "Restore the expected checked-out branch and registered worktree before polling again.",
        ),
        (
            PollDisposition::Materialized,
            "The remote context was materialized locally.",
        ),
        (
            PollDisposition::Nonconforming,
            "Repair the remote context's canonical tree before materialization.",
        ),
        (
            PollDisposition::BlockedByLocalState,
            "Inspect the existing local branch or path before materialization.",
        ),
        (
            PollDisposition::MaterializationIncomplete,
            "Inspect and repair the retained partial materialization.",
        ),
        (
            PollDisposition::Malformed,
            "Repair the remote context's branch name.",
        ),
        (
            PollDisposition::RemotelyDeleted,
            "The remote context is absent; local state was preserved.",
        ),
        (
            PollDisposition::HistoryUnknown,
            "Publication history is unknown; inspect remote state before proceeding.",
        ),
        (
            PollDisposition::NotReached,
            "This target was not reached; retry the poll when ready.",
        ),
    ] {
        assert_eq!(disposition.guidance(), guidance);
    }
}

#[test]
fn poll_relation_equal_behind_ahead_and_diverged_are_pure() {
    let a = git2::Oid::from_bytes(&[1; 20]).unwrap();
    let b = git2::Oid::from_bytes(&[2; 20]).unwrap();
    assert_eq!(
        classify_relation(a, a, |_, _| -> Result<bool, ()> {
            panic!("equal needs no ancestry")
        }),
        Ok(PollRelation::Current)
    );
    for (answers, expected) in [
        ([true, false], PollRelation::Behind),
        ([false, true], PollRelation::LocalAhead),
        ([false, false], PollRelation::Diverged),
    ] {
        let mut calls = Vec::new();
        assert_eq!(
            classify_relation(a, b, |x, y| -> Result<bool, ()> {
                calls.push((x, y));
                Ok(if (x, y) == (a, b) {
                    answers[0]
                } else {
                    assert_eq!((x, y), (b, a));
                    answers[1]
                })
            }),
            Ok(expected)
        );
        assert_eq!(calls.len(), if answers[0] { 1 } else { 2 });
    }
}

#[test]
fn poll_relation_surfaces_both_ancestry_errors_without_guessing() {
    let a = git2::Oid::from_bytes(&[1; 20]).unwrap();
    let b = git2::Oid::from_bytes(&[2; 20]).unwrap();
    for failing_call in [1, 2] {
        let mut calls = 0;
        assert_eq!(
            classify_relation(a, b, |_, _| {
                calls += 1;
                if calls == failing_call {
                    Err("ancestry unavailable")
                } else {
                    Ok(false)
                }
            }),
            Err("ancestry unavailable")
        );
        assert_eq!(calls, failing_call);
    }
}

struct LocalFixture {
    _root: tempfile::TempDir,
    _data: tempfile::TempDir,
    root: PathBuf,
    service: RepositoryService,
    plan: RemoteRefPlan,
    repository: git2::Repository,
    old: git2::Oid,
}

impl LocalFixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        let data = tempfile::tempdir().unwrap();
        let mut options = git2::RepositoryInitOptions::new();
        options.initial_head("main");
        let repository = git2::Repository::init_opts(&root, &options).unwrap();
        repository
            .config()
            .unwrap()
            .set_bool("core.autocrlf", false)
            .unwrap();
        fs::create_dir(root.join(".manyhands")).unwrap();
        fs::write(
            root.join(".manyhands/config.toml"),
            "format_version = 1\nprimary_branch = \"main\"\npublication_remote = \"origin\"\n",
        )
        .unwrap();
        fs::write(root.join(".gitignore"), ".manyhands/worktrees/\nignored\n").unwrap();
        fs::write(root.join("file"), b"old\n").unwrap();
        let mut index = repository.index().unwrap();
        for path in ["file", ".gitignore", ".manyhands/config.toml"] {
            index.add_path(Path::new(path)).unwrap();
        }
        index.write().unwrap();
        let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
        let signature =
            git2::Signature::new("Fixture", "fixture@example.invalid", &git2::Time::new(0, 0))
                .unwrap();
        let old = repository
            .commit(Some("HEAD"), &signature, &signature, "old", &tree, &[])
            .unwrap();
        drop(tree);
        repository
            .remote("origin", "ssh://fixture@example.invalid/repo")
            .unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        Self {
            _root: directory,
            _data: data,
            root,
            service,
            plan: RemoteRefPlan::from_configuration("origin", "main").unwrap(),
            repository,
            old,
        }
    }

    fn incoming(&self, parent: git2::Oid, extra: Option<&str>) -> git2::Oid {
        let parent = self.repository.find_commit(parent).unwrap();
        let mut builder = self
            .repository
            .treebuilder(Some(&parent.tree().unwrap()))
            .unwrap();
        builder
            .insert("file", self.repository.blob(b"new\n").unwrap(), 0o100644)
            .unwrap();
        if let Some(extra) = extra {
            builder
                .insert(
                    extra,
                    self.repository.blob(b"incoming\n").unwrap(),
                    0o100644,
                )
                .unwrap();
        }
        let tree = self.repository.find_tree(builder.write().unwrap()).unwrap();
        let signature =
            git2::Signature::new("Fixture", "fixture@example.invalid", &git2::Time::new(1, 0))
                .unwrap();
        self.repository
            .commit(None, &signature, &signature, "new", &tree, &[&parent])
            .unwrap()
    }

    fn context(&self) -> (SynchronizationTarget, git2::Repository) {
        let target = SynchronizationTarget::Context {
            kind: AuthoringKind::Document,
            item_id: POLL_ID.parse().unwrap(),
        };
        let reference = self
            .repository
            .branch(
                &format!("manyhands/document/{POLL_ID}"),
                &self.repository.find_commit(self.old).unwrap(),
                false,
            )
            .unwrap()
            .into_reference();
        let path = self.root.join(".manyhands/worktrees").join(POLL_ID);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut options = git2::WorktreeAddOptions::new();
        options.reference(Some(&reference));
        self.repository
            .worktree(POLL_ID, &path, Some(&options))
            .unwrap();
        (target, git2::Repository::open(path).unwrap())
    }

    fn tracking(&self, target: &SynchronizationTarget, oid: git2::Oid) {
        let name = match target {
            SynchronizationTarget::Primary => self.plan.primary().tracking_ref().to_owned(),
            SynchronizationTarget::Context { kind, item_id } => {
                self.plan.context(*kind, item_id).tracking_ref().to_owned()
            }
        };
        self.repository
            .reference(&name, oid, true, "fixture observation")
            .unwrap();
    }
}

#[derive(Debug, PartialEq, Eq)]
struct LocalSnapshot {
    files: BTreeMap<PathBuf, Vec<u8>>,
    registrations: BTreeMap<PathBuf, Vec<u8>>,
    index: Option<Vec<u8>>,
    head: Vec<u8>,
    refs: Vec<(String, String)>,
    statuses: Vec<(Vec<u8>, u32)>,
}

fn files_at(path: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, path: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
        let Ok(metadata) = fs::symlink_metadata(path) else {
            return;
        };
        let relative = path.strip_prefix(root).unwrap().to_owned();
        if metadata.file_type().is_symlink() {
            files.insert(
                relative,
                fs::read_link(path)
                    .unwrap()
                    .as_os_str()
                    .as_encoded_bytes()
                    .to_vec(),
            );
        } else if metadata.is_file() {
            files.insert(relative, fs::read(path).unwrap());
        } else if metadata.is_dir() {
            files.insert(relative, Vec::new());
            for entry in fs::read_dir(path).unwrap() {
                let entry = entry.unwrap();
                if entry.file_name() != ".git" || !entry.file_type().unwrap().is_dir() {
                    visit(root, &entry.path(), files);
                }
            }
        }
    }
    let mut files = BTreeMap::new();
    visit(path, path, &mut files);
    files
}

fn snapshot(root: &Path, repository: &git2::Repository) -> LocalSnapshot {
    let mut options = git2::StatusOptions::new();
    options.include_untracked(true).recurse_untracked_dirs(true);
    let statuses = repository
        .statuses(Some(&mut options))
        .unwrap()
        .iter()
        .map(|e| (e.path_bytes().to_vec(), e.status().bits()))
        .collect();
    let mut refs = repository
        .references()
        .unwrap()
        .map(|r| {
            let r = r.unwrap();
            (
                r.name().unwrap().to_owned(),
                r.symbolic_target()
                    .map(str::to_owned)
                    .unwrap_or_else(|| r.target().unwrap().to_string()),
            )
        })
        .collect::<Vec<_>>();
    refs.sort();
    LocalSnapshot {
        files: files_at(root),
        registrations: files_at(&repository.commondir().join("worktrees")),
        index: fs::read(repository.path().join("index")).ok(),
        head: fs::read(repository.path().join("HEAD")).unwrap(),
        refs,
        statuses,
    }
}

#[test]
fn poll_classification_dirty_and_conflicted_states_preserve_full_snapshot() {
    for (case, expected) in [
        ("modified", PollDisposition::Dirty),
        ("staged", PollDisposition::Dirty),
        ("untracked", PollDisposition::Dirty),
        ("directory", PollDisposition::Dirty),
        ("merge", PollDisposition::Conflicted),
        ("conflict", PollDisposition::Conflicted),
        ("index_tree", PollDisposition::Dirty),
    ] {
        let f = LocalFixture::new();
        let r = &f.repository;
        match case {
            "modified" | "staged" => {
                fs::write(f.root.join("file"), b"dirty").unwrap();
                if case == "staged" {
                    let mut i = r.index().unwrap();
                    i.add_path(Path::new("file")).unwrap();
                    i.write().unwrap();
                }
            }
            "untracked" => fs::write(f.root.join("untracked"), b"untracked").unwrap(),
            "directory" => {
                fs::create_dir(f.root.join("dir")).unwrap();
                fs::write(f.root.join("dir/untracked"), b"untracked").unwrap();
            }
            "merge" => fs::write(r.path().join("MERGE_HEAD"), format!("{}\n", f.old)).unwrap(),
            "conflict" => {
                let mut i = r.index().unwrap();
                let mut e = i.get_path(Path::new("file"), 0).unwrap();
                i.remove_path(Path::new("file")).unwrap();
                e.flags = (e.flags & !0x3000) | 0x1000;
                i.add(&e).unwrap();
                i.write().unwrap();
                assert!(i.has_conflicts());
            }
            "index_tree" => {
                let mut i = r.index().unwrap();
                let mut e = i.get_path(Path::new("file"), 0).unwrap();
                e.id = r.blob(b"different index tree").unwrap();
                i.add(&e).unwrap();
                i.write().unwrap();
                assert_ne!(
                    i.write_tree().unwrap(),
                    r.find_commit(f.old).unwrap().tree_id()
                );
            }
            _ => unreachable!(),
        }
        let before = snapshot(&f.root, r);
        let result = classify_local_target(
            &f.root,
            "main",
            &SynchronizationTarget::Primary,
            PollTrackingState::Present(f.old),
        )
        .unwrap();
        assert_eq!(
            result,
            LocalPollClassification::Unchanged {
                disposition: expected,
                local_oid: Some(f.old)
            },
            "{case}"
        );
        assert_eq!(snapshot(&f.root, r), before, "{case}");
    }
}

#[test]
fn poll_ignored_only_is_clean_and_remote_absence_wins_over_dirty() {
    let f = LocalFixture::new();
    fs::write(f.root.join("ignored"), b"local ignored").unwrap();
    let before = snapshot(&f.root, &f.repository);
    assert_eq!(
        classify_local_target(
            &f.root,
            "main",
            &SynchronizationTarget::Primary,
            PollTrackingState::Present(f.old)
        )
        .unwrap(),
        LocalPollClassification::Unchanged {
            disposition: PollDisposition::Current,
            local_oid: Some(f.old)
        }
    );
    assert_eq!(snapshot(&f.root, &f.repository), before);
    let (target, context) = f.context();
    fs::write(context.workdir().unwrap().join("file"), b"dirty").unwrap();
    for (state, disposition) in [
        (
            PollTrackingState::RemotelyDeleted,
            PollDisposition::RemotelyDeleted,
        ),
        (
            PollTrackingState::HistoryUnknown,
            PollDisposition::HistoryUnknown,
        ),
    ] {
        let before = snapshot(&f.root, &context);
        assert_eq!(
            classify_local_target(&f.root, "main", &target, state).unwrap(),
            LocalPollClassification::Unchanged {
                disposition,
                local_oid: None
            }
        );
        assert_eq!(snapshot(&f.root, &context), before);
    }
}

#[test]
fn poll_wrong_or_detached_head_and_primary_not_at_root_are_inaccessible() {
    for case in ["detached", "wrong", "primary_elsewhere"] {
        let f = LocalFixture::new();
        if case == "detached" {
            f.repository.set_head_detached(f.old).unwrap();
        } else {
            f.repository
                .branch("wrong", &f.repository.find_commit(f.old).unwrap(), false)
                .unwrap();
            f.repository.set_head("refs/heads/wrong").unwrap();
        }
        if case == "primary_elsewhere" {
            let reference = f.repository.find_reference("refs/heads/main").unwrap();
            let path = f.root.join(".manyhands/worktrees/primary");
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            let mut options = git2::WorktreeAddOptions::new();
            options.reference(Some(&reference));
            f.repository
                .worktree("primary", &path, Some(&options))
                .unwrap();
        }
        let before = snapshot(&f.root, &f.repository);
        assert_eq!(
            classify_local_target(
                &f.root,
                "main",
                &SynchronizationTarget::Primary,
                PollTrackingState::Present(f.old)
            )
            .unwrap(),
            LocalPollClassification::Unchanged {
                disposition: PollDisposition::Inaccessible,
                local_oid: None
            }
        );
        assert_eq!(snapshot(&f.root, &f.repository), before);
    }
}

#[test]
fn poll_missing_and_unregistered_contexts_are_inaccessible_and_unchanged() {
    for case in ["missing", "unregistered", "wrong", "detached"] {
        let f = LocalFixture::new();
        let (target, context) = f.context();
        let path = context.workdir().unwrap().to_owned();
        if case == "missing" {
            fs::rename(&path, f.root.join("kept")).unwrap();
        }
        if case == "unregistered" {
            fs::rename(
                f.repository.path().join("worktrees").join(POLL_ID),
                f.repository.path().join("kept-registration"),
            )
            .unwrap();
        }
        if case == "wrong" {
            f.repository
                .branch("wrong", &f.repository.find_commit(f.old).unwrap(), false)
                .unwrap();
            context.set_head("refs/heads/wrong").unwrap();
        }
        if case == "detached" {
            context.set_head_detached(f.old).unwrap();
        }
        let before = snapshot(&f.root, &f.repository);
        assert_eq!(
            classify_local_target(&f.root, "main", &target, PollTrackingState::Present(f.old))
                .unwrap(),
            LocalPollClassification::Unchanged {
                disposition: PollDisposition::Inaccessible,
                local_oid: None
            },
            "{case}"
        );
        assert_eq!(snapshot(&f.root, &f.repository), before, "{case}");
    }
}

#[cfg(unix)]
#[test]
fn poll_symlink_deterministic_path_is_inaccessible_without_following_it() {
    let f = LocalFixture::new();
    let (target, _context) = f.context();
    let path = f.root.join(".manyhands/worktrees").join(POLL_ID);
    let kept = f.root.join("kept");
    fs::rename(&path, &kept).unwrap();
    std::os::unix::fs::symlink(&kept, &path).unwrap();
    let before = snapshot(&f.root, &f.repository);
    assert_eq!(
        classify_local_target(&f.root, "main", &target, PollTrackingState::Present(f.old)).unwrap(),
        LocalPollClassification::Unchanged {
            disposition: PollDisposition::Inaccessible,
            local_oid: None
        }
    );
    assert_eq!(snapshot(&f.root, &f.repository), before);
    assert!(path.symlink_metadata().unwrap().file_type().is_symlink());
}

#[test]
fn poll_fast_forward_primary_and_context_advance_files_index_branch_then_are_current() {
    for is_context in [false, true] {
        let f = LocalFixture::new();
        let (target, linked) = if is_context {
            f.context()
        } else {
            (
                SynchronizationTarget::Primary,
                git2::Repository::open(&f.root).unwrap(),
            )
        };
        let incoming = f.incoming(f.old, None);
        f.tracking(&target, incoming);
        let expected = f
            .service
            .observation_configuration(&f.root, &f.plan)
            .unwrap();
        assert_eq!(
            classify_local_target(
                &f.root,
                "main",
                &target,
                PollTrackingState::Present(incoming)
            )
            .unwrap(),
            LocalPollClassification::FastForward {
                local_oid: f.old,
                tracking_oid: incoming
            }
        );
        let update = PollFastForward {
            target: target.clone(),
            before: f.old,
            after: incoming,
        };
        assert_eq!(
            f.service
                .poll_fast_forward(&f.root, &f.plan, &expected, &update)
                .unwrap(),
            PollDisposition::FastForwarded
        );
        assert_eq!(linked.head().unwrap().target(), Some(incoming));
        assert_eq!(
            linked.index().unwrap().write_tree().unwrap(),
            linked.find_commit(incoming).unwrap().tree_id()
        );
        assert_eq!(
            fs::read(linked.workdir().unwrap().join("file")).unwrap(),
            b"new\n"
        );
        let before = snapshot(&f.root, &linked);
        let update = PollFastForward {
            target: target.clone(),
            before: incoming,
            after: incoming,
        };
        assert_eq!(
            f.service
                .poll_fast_forward(&f.root, &f.plan, &expected, &update)
                .unwrap(),
            PollDisposition::Current
        );
        assert_eq!(snapshot(&f.root, &linked), before);
        if is_context {
            assert_eq!(f.repository.head().unwrap().target(), Some(f.old));
        }
    }
}

#[test]
fn poll_fast_forward_reobserves_ref_tracking_configuration_and_endpoint_before_mutation() {
    for case in ["local", "tracking", "configuration", "endpoint", "dirty"] {
        let f = LocalFixture::new();
        let target = SynchronizationTarget::Primary;
        let incoming = f.incoming(f.old, None);
        f.tracking(&target, incoming);
        let expected = f
            .service
            .observation_configuration(&f.root, &f.plan)
            .unwrap();
        assert!(matches!(
            classify_local_target(
                &f.root,
                "main",
                &target,
                PollTrackingState::Present(incoming)
            )
            .unwrap(),
            LocalPollClassification::FastForward { .. }
        ));
        match case {
            "local" => {
                f.repository
                    .reference("refs/heads/main", incoming, true, "external change")
                    .unwrap();
            }
            "tracking" => f.tracking(&target, f.old),
            "configuration" => fs::write(
                f.root.join(".manyhands/config.toml"),
                "format_version = 1\nprimary_branch = \"other\"\npublication_remote = \"origin\"\n",
            )
            .unwrap(),
            "endpoint" => f
                .repository
                .remote_set_url("origin", "ssh://other@example.invalid/repo")
                .unwrap(),
            "dirty" => fs::write(f.root.join("file"), b"dirty").unwrap(),
            _ => unreachable!(),
        }
        let before = snapshot(&f.root, &f.repository);
        let result = f.service.poll_fast_forward(
            &f.root,
            &f.plan,
            &expected,
            &PollFastForward {
                target,
                before: f.old,
                after: incoming,
            },
        );
        if case == "dirty" {
            assert_eq!(result.unwrap(), PollDisposition::Dirty);
        } else {
            assert!(
                matches!(result, Err(SynchronizationError::ExternalChange)),
                "{case}: {result:?}"
            );
        }
        assert_eq!(snapshot(&f.root, &f.repository), before, "{case}");
    }
}

#[test]
fn poll_ignored_incoming_collision_is_dirty_and_byte_exact_unchanged() {
    let f = LocalFixture::new();
    let incoming = f.incoming(f.old, Some("ignored"));
    let target = SynchronizationTarget::Primary;
    f.tracking(&target, incoming);
    fs::write(f.root.join("ignored"), b"ignored sentinel").unwrap();
    let expected = f
        .service
        .observation_configuration(&f.root, &f.plan)
        .unwrap();
    let before = snapshot(&f.root, &f.repository);
    assert_eq!(
        f.service
            .poll_fast_forward(
                &f.root,
                &f.plan,
                &expected,
                &PollFastForward {
                    target,
                    before: f.old,
                    after: incoming
                }
            )
            .unwrap(),
        PollDisposition::Dirty
    );
    assert_eq!(snapshot(&f.root, &f.repository), before);
}

fn item_source(kind: AuthoringKind, id: &str) -> Vec<u8> {
    let (kind, extra) = match kind {
        AuthoringKind::Document => ("document", ""),
        AuthoringKind::Ticket => ("ticket", "type: task\nstatus: open\n"),
    };
    format!("---\nmanyhands_managed: true\nmanyhands_kind: {kind}\nid: {id}\ntitle: Fixture\n{extra}---\n").into_bytes()
}

// Build nested trees entirely in the ODB, including modes not produced by staging.
type FixtureTreeEntries<'a> = Vec<(Vec<&'a str>, git2::Oid, i32)>;

fn fixture_tree(repository: &git2::Repository, entries: &[(&str, Vec<u8>, i32)]) -> git2::Oid {
    fn build(repository: &git2::Repository, entries: &[(Vec<&str>, git2::Oid, i32)]) -> git2::Oid {
        let mut builder = repository.treebuilder(None).unwrap();
        let mut dirs: BTreeMap<&str, FixtureTreeEntries<'_>> = BTreeMap::new();
        for (path, oid, mode) in entries {
            if path.len() == 1 {
                builder.insert(path[0], *oid, *mode).unwrap();
            } else {
                dirs.entry(path[0])
                    .or_default()
                    .push((path[1..].to_vec(), *oid, *mode));
            }
        }
        for (name, entries) in dirs {
            if name == ".git" {
                // TreeBuilder protects this name. A fetched hostile tree can
                // still hold it; create that test object directly in the ODB.
                assert!(builder.is_empty());
                let mut bytes = b"40000 .git\0".to_vec();
                bytes.extend_from_slice(build(repository, &entries).as_bytes());
                return repository
                    .odb()
                    .unwrap()
                    .write(git2::ObjectType::Tree, &bytes)
                    .unwrap();
            }
            builder
                .insert(name, build(repository, &entries), 0o040000)
                .unwrap();
        }
        builder.write().unwrap()
    }
    let entries = entries
        .iter()
        .map(|(path, bytes, mode)| {
            let oid = if *mode == 0o160000 {
                repository.head().unwrap().target().unwrap()
            } else {
                repository.blob(bytes).unwrap()
            };
            (path.split('/').collect(), oid, *mode)
        })
        .collect::<Vec<_>>();
    build(repository, &entries)
}

#[test]
fn poll_tree_conformity_matrix_rejects_without_creation_and_all_accepted_are_discoverable() {
    for case in [
        "document",
        "ticket",
        "invalid_comment",
        "duplicate",
        "ticket_id",
        "kind",
        "missing",
        "frontmatter",
        "utf8",
        "executable",
        "symlink",
        "gitlink",
        "worktrees",
        "worktrees_blob",
    ] {
        let f = LocalFixture::new();
        let kind = if case == "ticket" || case == "ticket_id" {
            AuthoringKind::Ticket
        } else {
            AuthoringKind::Document
        };
        let path = if kind == AuthoringKind::Ticket {
            format!(".manyhands/tickets/{POLL_ID}/ticket.md")
        } else {
            "docs/item.md".into()
        };
        let source = match case {
            "ticket_id" => item_source(kind, OTHER_ID),
            "kind" => item_source(AuthoringKind::Ticket, POLL_ID),
            "frontmatter" => b"---\n[broken\n---\n".to_vec(),
            "utf8" => vec![255],
            "symlink" => b"../other".to_vec(),
            _ => item_source(kind, POLL_ID),
        };
        let mode = match case {
            "executable" => 0o100755,
            "symlink" => 0o120000,
            "gitlink" => 0o160000,
            _ => 0o100644,
        };
        let mut entries = if case == "missing" {
            Vec::new()
        } else {
            vec![(path.as_str(), source, mode)]
        };
        match case {
            "duplicate" => entries.push(("docs/second.md", item_source(kind, POLL_ID), 0o100644)),
            "worktrees" => entries.push((
                ".manyhands/worktrees/forbidden/file",
                b"forbidden".to_vec(),
                0o100644,
            )),
            "worktrees_blob" => {
                entries.push((".manyhands/worktrees", b"forbidden".to_vec(), 0o100644))
            }
            "invalid_comment" => entries.push((
                ".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01ARZ3NDEKTSV4RRFFQ69G5FAW.md",
                b"---\n[broken\n---\n".to_vec(),
                0o100644,
            )),
            _ => {}
        }
        let tree_id = fixture_tree(&f.repository, &entries);
        let tree = f.repository.find_tree(tree_id).unwrap();
        let expected = matches!(case, "document" | "ticket" | "invalid_comment");
        let before = snapshot(&f.root, &f.repository);
        assert_eq!(
            conforming_context_tree(&f.repository, &tree, kind, &POLL_ID.parse().unwrap()).unwrap(),
            expected,
            "{case}"
        );
        assert_eq!(
            snapshot(&f.root, &f.repository),
            before,
            "validation creates nothing: {case}"
        );
        assert!(
            f.repository
                .find_branch(
                    &format!(
                        "manyhands/{}/{POLL_ID}",
                        if kind == AuthoringKind::Document {
                            "document"
                        } else {
                            "ticket"
                        }
                    ),
                    git2::BranchType::Local
                )
                .is_err()
        );
        assert!(f.repository.find_worktree(POLL_ID).is_err());
        assert!(!f.root.join(".manyhands/worktrees").join(POLL_ID).exists());
        if expected {
            let signature =
                git2::Signature::new("Fixture", "fixture@example.invalid", &git2::Time::new(1, 0))
                    .unwrap();
            let oid = f
                .repository
                .commit(None, &signature, &signature, "incoming", &tree, &[])
                .unwrap();
            let commit = f.repository.find_commit(oid).unwrap();
            let branch = format!(
                "manyhands/{}/{POLL_ID}",
                if kind == AuthoringKind::Document {
                    "document"
                } else {
                    "ticket"
                }
            );
            let reference = f
                .repository
                .branch(&branch, &commit, false)
                .unwrap()
                .into_reference();
            let path = f.root.join(".manyhands/worktrees").join(POLL_ID);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            let mut options = git2::WorktreeAddOptions::new();
            options.reference(Some(&reference));
            f.repository
                .worktree(POLL_ID, &path, Some(&options))
                .unwrap();
            let observation = crate::repository::discovery::observe_root(&f.repository, &f.root);
            assert_eq!(observation.active_contexts.len(), 1, "{case}");
            assert_eq!(
                observation.active_contexts[0].context.branch.as_deref(),
                Some(branch.as_str())
            );
            if case == "invalid_comment" {
                assert!(
                    !observation.active_contexts[0]
                        .validation
                        .problems
                        .is_empty()
                );
            }
        }
    }
}

#[test]
fn poll_conformity_cannot_hide_executable_duplicate_or_discovery_excluded_item() {
    let f = LocalFixture::new();
    for entries in [
        vec![
            (
                "docs/item.md",
                item_source(AuthoringKind::Document, POLL_ID),
                0o100644,
            ),
            (
                "docs/duplicate.md",
                item_source(AuthoringKind::Document, POLL_ID),
                0o100755,
            ),
        ],
        vec![(
            "docs/.git/item.md",
            item_source(AuthoringKind::Document, POLL_ID),
            0o100644,
        )],
    ] {
        let tree = f
            .repository
            .find_tree(fixture_tree(&f.repository, &entries))
            .unwrap();
        let before = snapshot(&f.root, &f.repository);
        assert!(
            !conforming_context_tree(
                &f.repository,
                &tree,
                AuthoringKind::Document,
                &POLL_ID.parse().unwrap()
            )
            .unwrap()
        );
        assert_eq!(snapshot(&f.root, &f.repository), before);
    }
}

#[test]
fn poll_classifies_actual_ahead_diverged_and_missing_ancestry_without_mutation() {
    let f = LocalFixture::new();
    let child = f.incoming(f.old, None);
    f.repository
        .reference("refs/heads/main", child, true, "fixture local commit")
        .unwrap();
    f.repository
        .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    let before = snapshot(&f.root, &f.repository);
    assert_eq!(
        classify_local_target(
            &f.root,
            "main",
            &SynchronizationTarget::Primary,
            PollTrackingState::Present(f.old)
        )
        .unwrap(),
        LocalPollClassification::Unchanged {
            disposition: PollDisposition::LocalAhead,
            local_oid: Some(child)
        }
    );
    let mut builder = f
        .repository
        .treebuilder(Some(
            &f.repository.find_commit(f.old).unwrap().tree().unwrap(),
        ))
        .unwrap();
    builder
        .insert("peer", f.repository.blob(b"peer").unwrap(), 0o100644)
        .unwrap();
    let tree = f.repository.find_tree(builder.write().unwrap()).unwrap();
    let sig =
        git2::Signature::new("Fixture", "fixture@example.invalid", &git2::Time::new(2, 0)).unwrap();
    let peer = f
        .repository
        .commit(
            None,
            &sig,
            &sig,
            "peer",
            &tree,
            &[&f.repository.find_commit(f.old).unwrap()],
        )
        .unwrap();
    assert_eq!(
        classify_local_target(
            &f.root,
            "main",
            &SynchronizationTarget::Primary,
            PollTrackingState::Present(peer)
        )
        .unwrap(),
        LocalPollClassification::Unchanged {
            disposition: PollDisposition::Diverged,
            local_oid: Some(child)
        }
    );
    assert!(matches!(
        classify_local_target(
            &f.root,
            "main",
            &SynchronizationTarget::Primary,
            PollTrackingState::Present(git2::Oid::from_bytes(&[99; 20]).unwrap())
        ),
        Err(SynchronizationError::RecoveryRequired)
    ));
    assert_eq!(snapshot(&f.root, &f.repository), before);
}

#[test]
fn poll_fast_forward_allows_clean_tracked_file_to_directory_transition() {
    let f = LocalFixture::new();
    let parent = f.repository.find_commit(f.old).unwrap();
    let mut directory = f.repository.treebuilder(None).unwrap();
    directory
        .insert("nested", f.repository.blob(b"nested\n").unwrap(), 0o100644)
        .unwrap();
    let mut builder = f
        .repository
        .treebuilder(Some(&parent.tree().unwrap()))
        .unwrap();
    builder.remove("file").unwrap();
    builder
        .insert("file", directory.write().unwrap(), 0o040000)
        .unwrap();
    let tree = f.repository.find_tree(builder.write().unwrap()).unwrap();
    let sig =
        git2::Signature::new("Fixture", "fixture@example.invalid", &git2::Time::new(2, 0)).unwrap();
    let incoming = f
        .repository
        .commit(None, &sig, &sig, "file to directory", &tree, &[&parent])
        .unwrap();
    f.tracking(&SynchronizationTarget::Primary, incoming);
    let expected = f
        .service
        .observation_configuration(&f.root, &f.plan)
        .unwrap();
    assert_eq!(
        f.service
            .poll_fast_forward(
                &f.root,
                &f.plan,
                &expected,
                &PollFastForward {
                    target: SynchronizationTarget::Primary,
                    before: f.old,
                    after: incoming
                }
            )
            .unwrap(),
        PollDisposition::FastForwarded
    );
    assert_eq!(fs::read(f.root.join("file/nested")).unwrap(), b"nested\n");
    assert_eq!(f.repository.head().unwrap().target(), Some(incoming));
    let mut index = f.repository.index().unwrap();
    index.read(true).unwrap(); // Reobserve disk after the updater's separate handle.
    assert_eq!(index.write_tree().unwrap(), tree.id());
}
