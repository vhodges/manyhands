//! `item resolve-history`'s domain call: what it writes, what it refuses and
//! what it clears. The fixture is a real enabled repository with one ticket
//! context and a publication remote, so the registry holds what production
//! holds; the remote itself is never contacted.
use git2::Oid;
use rusqlite::Connection;

use super::{
    RemotePublicationEvidence, RemoteRefObservation, RemoteRefPlan, SynchronizationTarget,
    refs::{CleanIntegrationError, plan_clean_integration},
    resolve::{HistoryObservation, MarkerRetirement},
    state,
};
use crate::{
    canonical::ItemId,
    repository::{
        AddRemoteRequest, AuthoringKind, AuthoringTarget, ContextIntent, EnableRepositoryRequest,
        ExpectedPathObservation, OperationId, REGISTRY_FILE, RepositoryErrorKind,
        RepositoryService, SaveTicketRequest, SetPublicationRemoteRequest, TicketDraft, recovery,
    },
};

pub(crate) const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
pub(crate) const OTHER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";

pub(crate) fn item() -> ItemId {
    ITEM.parse().unwrap()
}

/// An enabled repository with one ticket context and `origin` selected.
pub(crate) struct Fixture {
    pub(crate) root: tempfile::TempDir,
    pub(crate) data: tempfile::TempDir,
    pub(crate) service: RepositoryService,
}

impl Fixture {
    pub(crate) fn new() -> Self {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(root.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_bool("core.autocrlf", false).unwrap();
        config.set_str("user.name", "Fixture").unwrap();
        config
            .set_str("user.email", "fixture@example.invalid")
            .unwrap();
        repo.set_head("refs/heads/main").unwrap();
        std::fs::write(root.path().join("ordinary.txt"), "original\n").unwrap();
        commit_all(&repo);
        let service = RepositoryService::open_at(data.path()).unwrap();
        service
            .enable(EnableRepositoryRequest {
                root: root.path().into(),
                primary_branch: "main".into(),
                identity: None,
                operation_id: OperationId::new(),
            })
            .unwrap();
        service
            .add_remote(AddRemoteRequest {
                root: root.path().into(),
                name: "origin".into(),
                url: "ssh://fixture@127.0.0.1:1/repository".into(),
                operation_id: OperationId::new(),
            })
            .unwrap();
        service
            .set_publication_remote(SetPublicationRemoteRequest {
                root: root.path().into(),
                name: Some("origin".into()),
                operation_id: OperationId::new(),
            })
            .unwrap();
        service
            .save_ticket(SaveTicketRequest {
                target: AuthoringTarget {
                    root: root.path().into(),
                    kind: AuthoringKind::Ticket,
                    item_id: item(),
                    intent: ContextIntent::Create,
                    operation_id: OperationId::new(),
                },
                draft: TicketDraft {
                    title: "Ticket".into(),
                    ticket_type: "task".into(),
                    status: "open".into(),
                    project: None,
                    team: None,
                    body: "body\n".into(),
                },
                expected_path: ExpectedPathObservation::Missing,
            })
            .unwrap();
        Self {
            root,
            data,
            service,
        }
    }

    pub(crate) fn db(&self) -> Connection {
        Connection::open(self.data.path().join(REGISTRY_FILE)).unwrap()
    }

    pub(crate) fn plan() -> RemoteRefPlan {
        RemoteRefPlan::from_configuration("origin", "main").unwrap()
    }

    pub(crate) fn git(&self) -> git2::Repository {
        git2::Repository::open(self.root.path()).unwrap()
    }

    /// Puts the registry in the state of a repository that lost its history:
    /// the ticket's row is replaced by `row` (`None` for no row), and the
    /// cache-loss flag is `flag`. Any batch and marker are removed.
    pub(crate) fn lose_history(&self, flag: bool, row: Option<&str>) {
        let db = self.db();
        db.execute("DELETE FROM remote_context_states", []).unwrap();
        db.execute("DELETE FROM remote_ref_observations", [])
            .unwrap();
        db.execute("DELETE FROM remote_observation_batches", [])
            .unwrap();
        db.execute(
            "UPDATE remote_polling_state SET history_unknown=?1",
            [i32::from(flag)],
        )
        .unwrap();
        if let Some(evidence) = row {
            self.insert_row(evidence);
        }
        let _ = std::fs::remove_file(self.data.path().join("remote-history-recovery-required"));
    }

    fn insert_row(&self, evidence: &str) {
        let target = Self::plan().context(AuthoringKind::Ticket, &item());
        self.db().execute("INSERT INTO remote_context_states(repository_id,remote_ref,tracking_ref,kind,item_id,publication_evidence,state,observed_at) VALUES(1,?1,?2,'ticket',?3,?4,'history_unknown',123)",
            rusqlite::params![target.remote_ref(), target.tracking_ref(), ITEM, evidence]).unwrap();
    }

    /// A second active ticket context with no row, as a repository that lost
    /// its cache holds one.
    pub(crate) fn add_context(&self) {
        self.db().execute(&format!("INSERT INTO contexts(repository_id,kind,branch,worktree_path,item_id) VALUES(1,'active','manyhands/ticket/{OTHER}','/active2','{OTHER}')"), []).unwrap();
    }

    /// A complete fetch whose batch advertises the ticket's branch at `oid`.
    pub(crate) fn advertise(&self, oid: Oid, observed_at: i64) {
        let plan = Self::plan();
        let target = plan.context(AuthoringKind::Ticket, &item());
        let observation =
            RemoteRefObservation::from_advertisement(&plan, target.remote_ref(), oid, Some(oid))
                .unwrap();
        self.git()
            .reference(target.tracking_ref(), oid, true, "fixture fetch")
            .unwrap();
        self.batch(&[observation], observed_at);
    }

    /// A complete fetch that advertises nothing.
    pub(crate) fn batch(&self, observations: &[RemoteRefObservation], observed_at: i64) {
        let plan = Self::plan();
        state::with_transaction(&self.service, self.root.path(), |tx, id| {
            let generation = state::generation(tx, id)?;
            state::complete_batch(tx, id, &plan, generation, observations, observed_at)
        })
        .unwrap();
    }

    pub(crate) fn observe(&self) -> HistoryObservation {
        self.service
            .history_observation(self.root.path(), AuthoringKind::Ticket, &item())
            .unwrap()
    }

    pub(crate) fn marker(&self) -> std::path::PathBuf {
        self.data.path().join("remote-history-recovery-required")
    }

    pub(crate) fn publish_marker(&self) {
        recovery::publish_remote_history_marker(self.data.path()).unwrap();
    }

    pub(crate) fn flag(&self) -> bool {
        self.db()
            .query_row(
                "SELECT history_unknown FROM remote_polling_state",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap()
            == 1
    }

    pub(crate) fn evidence(&self) -> RemotePublicationEvidence {
        self.observe().evidence
    }
}

fn commit_all(repo: &git2::Repository) {
    let mut index = repo.index().unwrap();
    index
        .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let sig = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "fixture",
        &tree,
        &parent.iter().collect::<Vec<_>>(),
    )
    .unwrap();
}

/// Whether a clean integration of the ticket's context may publish it now.
fn integrates(f: &Fixture) -> Result<(), CleanIntegrationError<()>> {
    let local = f.observe().local_head.unwrap();
    plan_clean_integration(
        &SynchronizationTarget::Context {
            kind: AuthoringKind::Ticket,
            item_id: item(),
        },
        local,
        Some(local),
        None,
        f.evidence(),
        |_, _| Ok::<_, ()>(false),
    )
    .map(|_| ())
}

struct Case {
    name: &'static str,
    flag: bool,
    row: Option<&'static str>,
    /// A second active context with no row.
    other: bool,
    marker: Marker,
    /// What the first resolution reports: cleared, marker, flag afterwards.
    first: (bool, MarkerRetirement, bool),
    /// The second context's resolution, when `other`: cleared and marker.
    then: Option<(bool, MarkerRetirement)>,
}

#[derive(Clone, Copy)]
enum Marker {
    Absent,
    File,
    /// Not a file this build wrote: it cannot be deleted.
    Directory,
}

#[test]
fn resolving_writes_never_published_and_clears_the_flag_with_the_last_context() {
    use MarkerRetirement::{Absent, Failed, NotApplicable, Removed};
    let cases = [
        Case {
            name: "unknown row, flag clear",
            flag: false,
            row: Some("history_unknown"),
            other: false,
            marker: Marker::Absent,
            first: (false, NotApplicable, false),
            then: None,
        },
        Case {
            name: "cache loss, no row, marker file",
            flag: true,
            row: None,
            other: false,
            marker: Marker::File,
            first: (true, Removed, false),
            then: None,
        },
        Case {
            name: "cache loss, no row, no marker file",
            flag: true,
            row: None,
            other: false,
            marker: Marker::Absent,
            first: (true, Absent, false),
            then: None,
        },
        Case {
            name: "cache loss, another context still unknown",
            flag: true,
            row: None,
            other: true,
            marker: Marker::File,
            first: (false, NotApplicable, true),
            then: Some((true, Removed)),
        },
        Case {
            name: "cache loss, a marker that cannot be deleted",
            flag: true,
            row: Some("history_unknown"),
            other: false,
            marker: Marker::Directory,
            first: (true, Failed, false),
            then: None,
        },
    ];
    for case in cases {
        let f = Fixture::new();
        f.lose_history(case.flag, case.row);
        if case.other {
            f.add_context();
        }
        match case.marker {
            Marker::Absent => {}
            Marker::File => f.publish_marker(),
            Marker::Directory => std::fs::create_dir(f.marker()).unwrap(),
        }
        let name = case.name;
        assert_eq!(
            f.evidence(),
            RemotePublicationEvidence::HistoryUnknown,
            "{name}"
        );
        assert!(
            matches!(integrates(&f), Err(CleanIntegrationError::HistoryUnknown)),
            "{name}: refused before"
        );
        let resolved = f
            .service
            .resolve_history(f.root.path(), AuthoringKind::Ticket, &item(), &f.observe())
            .unwrap();
        let (cleared, marker, flag) = case.first;
        assert_eq!(resolved.cache_loss_cleared, cleared, "{name} cleared");
        assert_eq!(resolved.marker, marker, "{name} marker");
        assert_eq!(f.flag(), flag, "{name} flag");
        assert_eq!(
            f.evidence(),
            RemotePublicationEvidence::NeverPublished,
            "{name}"
        );
        assert!(integrates(&f).is_ok(), "{name}: publishes after");
        // The marker file goes exactly when its repair finished.
        assert_eq!(
            f.marker().exists(),
            match case.marker {
                Marker::Absent => false,
                Marker::File => marker != Removed,
                Marker::Directory => true,
            },
            "{name} marker file"
        );
        if let Some((cleared, marker)) = case.then {
            let other: ItemId = OTHER.parse().unwrap();
            let observed = f
                .service
                .history_observation(f.root.path(), AuthoringKind::Ticket, &other)
                .unwrap();
            let resolved = f
                .service
                .resolve_history(f.root.path(), AuthoringKind::Ticket, &other, &observed)
                .unwrap();
            assert_eq!(resolved.cache_loss_cleared, cleared, "{name} then cleared");
            assert_eq!(resolved.marker, marker, "{name} then marker");
            assert!(!f.flag(), "{name} then flag");
            assert!(!f.marker().exists(), "{name} then marker file");
        }
    }
}

#[test]
fn resolving_refuses_what_moved_or_is_not_unknown() {
    // (name, flag, row, a change after the observation)
    type Change = fn(&Fixture);
    let cases: [(&str, bool, Option<&str>, Option<Change>); 3] = [
        (
            "a newer fetch advertises the branch",
            false,
            Some("history_unknown"),
            Some(|f| f.advertise(f.observe().local_head.unwrap(), 200)),
        ),
        (
            "a newer fetch advertises it under cache loss",
            true,
            None,
            Some(|f| f.advertise(f.observe().local_head.unwrap(), 200)),
        ),
        ("already resolved", false, None, None),
    ];
    for (name, flag, row, change) in cases {
        let f = Fixture::new();
        f.lose_history(flag, row);
        f.batch(&[], 100);
        let expected = f.observe();
        if let Some(change) = change {
            change(&f);
        }
        let before = f.observe();
        let error = f
            .service
            .resolve_history(f.root.path(), AuthoringKind::Ticket, &item(), &expected)
            .unwrap_err();
        assert_eq!(error.kind, RepositoryErrorKind::ExternalChange, "{name}");
        assert_eq!(f.observe(), before, "{name}: nothing written");
    }
}

/// A newer complete fetch that still lacks the branch moves nothing the
/// observation covers: the confirmation holds.
#[test]
fn a_newer_fetch_that_still_lacks_the_branch_does_not_stale_the_observation() {
    let f = Fixture::new();
    f.lose_history(false, Some("history_unknown"));
    f.batch(&[], 100);
    let expected = f.observe();
    f.batch(&[], 200);
    f.service
        .resolve_history(f.root.path(), AuthoringKind::Ticket, &item(), &expected)
        .unwrap();
    assert_eq!(f.evidence(), RemotePublicationEvidence::NeverPublished);
}

#[test]
fn clearing_the_flag_reports_a_marker_that_cannot_be_deleted_and_refuses_what_moved() {
    // (case, marker, expected result of clear_cache_loss)
    for (name, directory) in [("marker file", false), ("undeletable marker", true)] {
        let f = Fixture::new();
        f.lose_history(true, Some("never_published"));
        if directory {
            std::fs::create_dir(f.marker()).unwrap();
        } else {
            f.publish_marker();
        }
        let expected = f.service.cache_loss_observation(f.root.path()).unwrap();
        assert!(
            expected.cache_loss && expected.unresolved.is_empty(),
            "{name}"
        );
        let done = f
            .service
            .clear_cache_loss(f.root.path(), &expected)
            .unwrap();
        assert!(done.cache_loss_cleared, "{name}");
        assert_eq!(
            done.marker,
            if directory {
                MarkerRetirement::Failed
            } else {
                MarkerRetirement::Removed
            },
            "{name}"
        );
        assert!(!f.flag(), "{name}: the commit stands");
        // Nothing left to clear.
        let again = f
            .service
            .clear_cache_loss(f.root.path(), &expected)
            .unwrap_err();
        assert_eq!(again.kind, RepositoryErrorKind::ExternalChange, "{name}");
    }
    // A context that turns unknown after the observation is not cleared over.
    let f = Fixture::new();
    f.lose_history(true, Some("never_published"));
    let expected = f.service.cache_loss_observation(f.root.path()).unwrap();
    f.add_context();
    let error = f
        .service
        .clear_cache_loss(f.root.path(), &expected)
        .unwrap_err();
    assert_eq!(error.kind, RepositoryErrorKind::ExternalChange);
    assert!(f.flag());
}
