use super::super::test_support::*;
use super::*;
use crate::repository::{
    AuthoringKind, AuthoringTarget, ContextIntent, DocumentDraft, ExpectedPathObservation,
    LeaseKind, OperationId, PathChange, SaveDocumentRequest, coordination,
    promotion::observe::PromotionChangeEntry,
};
use std::{collections::BTreeMap, fs, net::TcpListener};

const OTHER: &str = "01BX5ZZKBKACTAV9WEVGEMMVRZ";
const DEP: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
const CLOSER: &str = "Closer Sentinel <closer-sentinel@example.invalid>";
const FIXTURE_BY: &str = "Fixture <fixture@example.invalid>";

impl Fixture {
    fn context_oid(&self) -> Oid {
        self.context.head().unwrap().target().unwrap()
    }
    fn request(
        &self,
        confirmation: Option<ConfirmedCommitIdentity>,
    ) -> PreviewTicketClosureRequest {
        PreviewTicketClosureRequest {
            root: self.root.path().into(),
            item_id: id(),
            confirmed_identity: confirmation,
        }
    }
    fn preview_with(
        &self,
        confirmation: Option<ConfirmedCommitIdentity>,
    ) -> TicketClosurePreflight {
        self.service
            .preview_ticket_closure(self.request(confirmation))
            .unwrap()
    }
    fn preview(&self) -> TicketClosurePreflight {
        self.preview_with(None)
    }
    fn ready(&self) -> TicketClosurePreview {
        match self.preview() {
            TicketClosurePreflight::Ready(p) => p,
            other => panic!("expected ready: {other:?}"),
        }
    }
    fn observation(&self, when: OffsetDateTime) -> ClosureObservation {
        match prepare_observation_at(&self.service, &self.request(None), when)
            .unwrap()
            .observed
        {
            Ok(o) => o,
            Err(refusal) => panic!("expected ready: {:?}", refusal.blockers),
        }
    }
    fn blocked(&self, blockers: &[ClosureBlocker]) {
        assert_eq!(
            self.preview(),
            TicketClosurePreflight::Blocked {
                blockers: blockers.to_vec(),
                identity_configuration: None,
            }
        );
    }
    fn missing_identity(&self) {
        let mut config = self.repo.config().unwrap();
        config.set_str("user.name", "").unwrap();
        config.set_str("user.email", "").unwrap();
    }
    /// Commits the closed ticket on the primary worktree.
    fn close_primary(&self) -> Oid {
        let closed =
            closed_ticket_bytes(&read(&self.repo, &ticket()), at(1_700_000_000), CLOSER).unwrap();
        write(&self.repo, &ticket(), closed);
        commit(&self.repo)
    }
    /// Commits the closed ticket on the context worktree.
    fn close_context(&self) -> Oid {
        let closed =
            closed_ticket_bytes(&read(&self.context, &ticket()), at(1_700_000_000), CLOSER)
                .unwrap();
        write(&self.context, &ticket(), closed);
        commit(&self.context)
    }
    fn history(&self, evidence: &str) {
        self.db().execute("INSERT INTO remote_context_states(repository_id,remote_ref,tracking_ref,kind,item_id,publication_evidence,state,observed_at) SELECT id,?1,?2,'ticket',?3,?4,?5,0 FROM repositories", rusqlite::params![format!("refs/heads/manyhands/ticket/{ID}"), format!("refs/remotes/origin/manyhands/ticket/{ID}"), ID, evidence, if evidence == "history_unknown" {"history_unknown"} else {"remotely_deleted"}]).unwrap();
        if evidence == "observed_published" {
            self.db()
                .execute(
                    "UPDATE remote_context_states SET last_advertised_oid=?1",
                    [self.context_oid().to_string()],
                )
                .unwrap();
        }
    }
    fn remote_row(&self, phase: &str) -> OperationId {
        let operation = OperationId::new();
        self.db().execute("INSERT INTO remote_operation_records(repository_id,operation_ulid,configuration_generation,remote_name,primary_branch,primary_ref,primary_tracking_ref,local_branch,action,priority,phase,created_at,updated_at) SELECT id,?1,0,'origin','main','refs/heads/main','refs/remotes/origin/main','main','synchronize_primary','manual',?2,0,0 FROM repositories", rusqlite::params![operation.to_string(), phase]).unwrap();
        operation
    }
    fn pending_conflict(&self) {
        let db = self.db();
        let oid = self.base.to_string();
        self.remote_row("interrupted");
        let id = db
            .query_row("SELECT max(id) FROM remote_operation_records", [], |r| {
                r.get::<_, i64>(0)
            })
            .unwrap();
        db.execute("INSERT INTO remote_integration_windows(operation_record_id,number,configuration_generation,owner_epoch,kind) VALUES(?1,0,0,0,'legacy')", [id]).unwrap();
        db.execute("INSERT INTO remote_integration_steps(operation_record_id,configuration_generation,owner_epoch,ordinal,stage,local_oid,incoming_oid,baseline_tree_oid,baseline_index_digest,conflict_digest,phase) VALUES(?1,0,0,0,'primary',?2,?2,?3,zeroblob(32),zeroblob(32),'conflict_pending')", rusqlite::params![id,oid,self.repo.find_commit(self.base).unwrap().tree_id().to_string()]).unwrap();
    }
    /// An `operation_records` row as a stopped ticket save leaves it:
    /// `item_id` null, the ticket only in its target (T0-6).
    fn local_row(&self, item: &str, state: &str, completed_step: Option<&str>) -> i64 {
        let db = self.db();
        db.execute("INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,target,state,completed_step,observed_at) SELECT id,root_path,?1,'save_ticket',?2,?3,?4,0 FROM repositories", rusqlite::params![OperationId::new().to_string(), format!("authoring-context-v1/ticket/{item}/Edit\0"), state, completed_step]).unwrap();
        db.last_insert_rowid()
    }
    /// A comment publication binding for a comment on the ticket, with its
    /// local row in `local_state` and, if any, its publication's remote row.
    fn comment_binding(&self, local_state: (&str, Option<&str>), remote_phase: Option<&str>) {
        let comment = OperationId::new().to_string();
        let db = self.db();
        db.execute("INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,target,state,completed_step,observed_at) SELECT id,root_path,?1,'submit_comment',?2,?3,?4,0 FROM repositories", rusqlite::params![OperationId::new().to_string(), format!("authoring-context-v1/ticket/{ID}/Edit\0.manyhands/comments\0{comment}"), local_state.0, local_state.1]).unwrap();
        let local = db.last_insert_rowid();
        let synchronization = match remote_phase {
            Some(phase) => self.remote_row(phase),
            None => OperationId::new(),
        };
        db.execute("INSERT INTO comment_publication_bindings(operation_ulid,local_record_id,root_digest,kind,item_ulid,comment_ulid,synchronization_ulid,pre_checkpoint_oid) VALUES(?1,?2,zeroblob(32),'ticket',?3,?4,?5,?6)", rusqlite::params![OperationId::new().to_string(), local, ID, comment, synchronization.to_string(), self.base.to_string()]).unwrap();
    }
}

// --- closed_ticket_bytes (C10, R-P1, R-T0-2) ---

#[test]
fn closed_ticket_bytes_round_trips_every_field_and_body() {
    for line_end in ["\n", "\r\n"] {
        let source = hand_written(line_end);
        let original = parse_ticket(source.as_bytes());
        let closed = closed_ticket_bytes(source.as_bytes(), at(1_700_000_000), CLOSER).unwrap();
        let mut expected = original.clone();
        expected.closed_at = Some(at(1_700_000_000));
        expected.closed_by = Some(CLOSER.into());
        assert_eq!(parse_ticket(&closed), expected, "{line_end:?}");
        assert_eq!(
            closure_text(&closed),
            ("2023-11-14T22:13:20Z".to_owned(), CLOSER.to_owned())
        );
        // The same input always gives the same bytes.
        assert_eq!(
            closed,
            closed_ticket_bytes(source.as_bytes(), at(1_700_000_000), CLOSER).unwrap()
        );
    }
}

#[test]
fn closed_ticket_bytes_normalizes_relationships_without_refusing() {
    // R-T0-2: unsorted `deps` and a null `parent` are normalized, not refused.
    let source = hand_written("\n").replace(
        "wave: 2",
        &format!("wave: 2\ndeps: [\"{OTHER}\", \"{DEP}\"]\nparent: null"),
    );
    let closed = closed_ticket_bytes(source.as_bytes(), at(1_700_000_000), CLOSER).unwrap();
    let reparsed = parse_ticket(&closed);
    assert_eq!(reparsed.closed_by.as_deref(), Some(CLOSER));
    assert_eq!(reparsed.unknown.get("parent"), None);
    assert_eq!(
        reparsed.unknown.get("deps"),
        Some(&serde_yaml::Value::Sequence(vec![DEP.into(), OTHER.into()]))
    );
    assert_eq!(
        reparsed.unknown.get("wave"),
        Some(&serde_yaml::Value::from(2))
    );
}

#[test]
fn closed_ticket_bytes_refusals() {
    use ClosureContentError::*;
    let source = hand_written("\n");
    let bytes = source.as_bytes();
    let closed = closed_ticket_bytes(bytes, at(1_700_000_000), CLOSER).unwrap();
    assert_eq!(
        closed_ticket_bytes(&closed, at(1_700_000_000), CLOSER),
        Err(AlreadyClosed)
    );
    assert_eq!(
        closed_ticket_bytes(&[0xff, 0xfe], at(1_700_000_000), CLOSER),
        Err(NotText)
    );
    let document = source.replace("\"ticket\"", "document");
    assert_eq!(
        closed_ticket_bytes(document.as_bytes(), at(1_700_000_000), CLOSER),
        Err(NotATicket)
    );
    let invalid = source.replace("status: \"open\"\n", "");
    assert_eq!(
        closed_ticket_bytes(invalid.as_bytes(), at(1_700_000_000), CLOSER),
        Err(NotATicket)
    );
    let offset = at(1_700_000_000).to_offset(time::UtcOffset::from_hms(2, 0, 0).unwrap());
    assert_eq!(closed_ticket_bytes(bytes, offset, CLOSER), Err(InvalidTime));
    let fraction = at(1_700_000_000).replace_nanosecond(5).unwrap();
    assert_eq!(
        closed_ticket_bytes(bytes, fraction, CLOSER),
        Err(InvalidTime)
    );
    assert_eq!(
        closed_ticket_bytes(bytes, at(1_700_000_000), ""),
        Err(InvalidClosedBy)
    );
    assert_eq!(
        closed_ticket_bytes(bytes, at(1_700_000_000), "a\0b"),
        Err(InvalidClosedBy)
    );
}

#[test]
fn closed_by_text_is_name_and_angled_email() {
    assert_eq!(
        closed_by_text(&CommitIdentity {
            name: "Closer Sentinel".into(),
            email: "closer-sentinel@example.invalid".into(),
        }),
        CLOSER
    );
}

// --- Each blocker alone ---

#[test]
fn not_a_ticket() {
    let f = Fixture::new(false);
    f.service
        .save_document(SaveDocumentRequest {
            target: AuthoringTarget {
                root: f.root.path().into(),
                kind: AuthoringKind::Document,
                item_id: OTHER.parse().unwrap(),
                intent: ContextIntent::Create,
                operation_id: OperationId::new(),
            },
            source_path: None,
            destination_path: "docs/document.md".into(),
            draft: DocumentDraft {
                title: "Document".into(),
                body: "body\n".into(),
            },
            expected_source: None,
            expected_destination: ExpectedPathObservation::Missing,
        })
        .unwrap();
    let preflight = f
        .service
        .preview_ticket_closure(PreviewTicketClosureRequest {
            root: f.root.path().into(),
            item_id: OTHER.parse().unwrap(),
            confirmed_identity: None,
        })
        .unwrap();
    assert_eq!(
        preflight,
        TicketClosurePreflight::Blocked {
            blockers: vec![ClosureBlocker::NotATicket],
            identity_configuration: None,
        }
    );
}
#[test]
fn context_missing() {
    let f = Fixture::new(false);
    let wt = f.repo.find_worktree(ID).unwrap();
    wt.prune(Some(
        git2::WorktreePruneOptions::new()
            .valid(true)
            .working_tree(true),
    ))
    .unwrap();
    f.blocked(&[ClosureBlocker::ContextMissing]);
}
#[test]
fn context_branch_missing() {
    let f = Fixture::new(false);
    f.repo
        .find_reference(&format!("refs/heads/manyhands/ticket/{ID}"))
        .unwrap()
        .delete()
        .unwrap();
    f.blocked(&[ClosureBlocker::ContextMissing]);
}
#[test]
fn context_nonconforming() {
    let f = Fixture::new(false);
    write(&f.context, &ticket(), "not a managed ticket\n");
    commit(&f.context);
    f.blocked(&[ClosureBlocker::ContextNonconforming]);
}
#[test]
fn context_symlink_ticket_is_nonconforming() {
    let f = Fixture::new(false);
    let bytes = read(&f.context, &ticket());
    let link = tree_commit(
        &f.repo,
        f.context_oid(),
        &[(&ticket(), Some((&bytes, 0o120000)))],
    );
    f.context
        .reference(
            &format!("refs/heads/manyhands/ticket/{ID}"),
            link,
            true,
            "fixture",
        )
        .unwrap();
    f.blocked(&[
        ClosureBlocker::ContextNonconforming,
        ClosureBlocker::ContextDirty,
    ]);
}
#[test]
fn context_dirty() {
    let f = Fixture::new(false);
    write(&f.context, "ordinary.txt", "dirty");
    f.blocked(&[ClosureBlocker::ContextDirty]);
}
#[test]
fn context_conflicted() {
    let f = Fixture::new(false);
    fs::write(f.context.path().join("MERGE_HEAD"), format!("{}\n", f.base)).unwrap();
    f.blocked(&[ClosureBlocker::ContextConflicted]);
}
#[test]
fn conflict_pending() {
    let f = Fixture::new(false);
    f.pending_conflict();
    f.blocked(&[ClosureBlocker::ConflictPending]);
}
#[test]
fn local_operation_pending() {
    let f = Fixture::new(false);
    f.local_row(ID, "worktree_observed", None);
    f.blocked(&[ClosureBlocker::LocalOperationPending]);
}
#[test]
fn comment_publication_pending() {
    let f = Fixture::new(false);
    f.comment_binding(("completed", None), Some("interrupted"));
    f.blocked(&[ClosureBlocker::CommentPublicationPending]);
}
#[test]
fn primary_missing() {
    let f = Fixture::new(false);
    f.repo
        .find_reference("refs/heads/main")
        .unwrap()
        .delete()
        .unwrap();
    f.blocked(&[ClosureBlocker::PrimaryMissing]);
}
#[test]
fn primary_tracking_missing() {
    let f = Fixture::new(true);
    f.repo
        .find_reference("refs/remotes/origin/main")
        .unwrap()
        .delete()
        .unwrap();
    f.blocked(&[ClosureBlocker::PrimaryMissing]);
}
#[test]
fn primary_not_checked_out() {
    let f = Fixture::new(false);
    f.repo.set_head_detached(f.base).unwrap();
    f.blocked(&[ClosureBlocker::PrimaryNotCheckedOut]);
}
#[test]
fn primary_dirty() {
    let f = Fixture::new(false);
    write(&f.repo, "ordinary.txt", "dirty");
    f.blocked(&[ClosureBlocker::PrimaryDirty]);
}
#[test]
fn primary_conflicted() {
    let f = Fixture::new(false);
    let mut index = f.repo.index().unwrap();
    let mut entry = index.get_path(Path::new("ordinary.txt"), 0).unwrap();
    index.remove_path(Path::new("ordinary.txt")).unwrap();
    entry.flags = (entry.flags & !0x3000) | 0x1000;
    index.add(&entry).unwrap();
    index.write().unwrap();
    assert!(index.has_conflicts());
    f.blocked(&[ClosureBlocker::PrimaryConflicted]);
}
#[test]
fn primary_ahead_of_remote() {
    let f = Fixture::new(true);
    write(&f.repo, "primary-only", "new");
    commit(&f.repo);
    f.blocked(&[ClosureBlocker::PrimaryAheadOfRemote]);
}
#[test]
fn context_behind_remote() {
    let f = Fixture::new(true);
    let oid = tree_commit(
        &f.repo,
        f.context_oid(),
        &[("remote-only", Some((b"remote", 0o100644)))],
    );
    f.tracking(&format!("manyhands/ticket/{ID}"), oid);
    f.blocked(&[ClosureBlocker::ContextBehindRemote]);
}
#[test]
fn integration_conflict() {
    let f = Fixture::new(false);
    let source = String::from_utf8(read(&f.repo, &ticket()))
        .unwrap()
        .replace("original body", "primary body");
    write(&f.repo, &ticket(), source);
    commit(&f.repo);
    f.blocked(&[ClosureBlocker::IntegrationConflict]);
}
/// Final review I-1: the pre-closure merge is clean, but the closure fields
/// land next to the primary's `status` edit, so the checkpoint merge the
/// execution runs conflicts. The preview names it, and writes nothing.
#[test]
fn checkpoint_merge_conflict_blocks_at_preview() {
    let f = Fixture::new(false);
    let source = String::from_utf8(read(&f.repo, &ticket()))
        .unwrap()
        .replace("status: open", "status: doing");
    write(&f.repo, &ticket(), source);
    commit(&f.repo);
    let before = files(f.root.path());
    f.blocked(&[ClosureBlocker::IntegrationConflict]);
    assert_eq!(files(f.root.path()), before, "no file, index or object");
}
/// Final review I-1, the contained-and-open row: merged by hand without
/// closure fields, then the primary edits `status`.
#[test]
fn contained_open_checkpoint_merge_conflict_blocks_at_preview() {
    let f = Fixture::new(false);
    checkout(&f.repo, f.context_oid());
    let source = String::from_utf8(read(&f.repo, &ticket()))
        .unwrap()
        .replace("status: open", "status: doing");
    write(&f.repo, &ticket(), source);
    commit(&f.repo);
    f.blocked(&[ClosureBlocker::IntegrationConflict]);
}
/// Re-review N-1 (R-F-3): with a remote and a context that is not
/// contained, the context child merges the base into the context first, so
/// execution never merges the checkpoint against this context. The same
/// `status` edit that blocks without a remote must not block here.
#[test]
fn remote_uncontained_status_edit_stays_ready() {
    let f = Fixture::new(true);
    let source = String::from_utf8(read(&f.repo, &ticket()))
        .unwrap()
        .replace("status: open", "status: doing");
    write(&f.repo, &ticket(), source);
    let advanced = commit(&f.repo);
    f.tracking("main", advanced);
    let p = f.ready();
    assert_eq!(p.integration(), PlannedIntegration::MergeCommit);
    assert!(f.observation(at(1_700_000_000)).needs_context_child);
}
#[test]
fn remote_context_deleted() {
    let f = Fixture::new(true);
    f.history("observed_published");
    f.blocked(&[ClosureBlocker::RemoteContextDeleted]);
}
#[test]
fn history_unknown() {
    let f = Fixture::new(true);
    f.db()
        .execute("UPDATE remote_polling_state SET history_unknown=1", [])
        .unwrap();
    f.blocked(&[ClosureBlocker::HistoryUnknown]);
}
#[test]
fn already_closed() {
    // C4: no context at all (worktree and branch gone) and the ticket closed
    // on the primary.
    let f = Fixture::new(false);
    f.close_primary();
    f.repo
        .find_worktree(ID)
        .unwrap()
        .prune(Some(
            git2::WorktreePruneOptions::new()
                .valid(true)
                .working_tree(true),
        ))
        .unwrap();
    f.repo
        .find_reference(&format!("refs/heads/manyhands/ticket/{ID}"))
        .unwrap()
        .delete()
        .unwrap();
    f.blocked(&[ClosureBlocker::AlreadyClosed]);
}
#[test]
fn primary_reopened() {
    let f = Fixture::new(false);
    let closed = f.close_context();
    checkout(&f.repo, closed);
    write(&f.repo, &ticket(), tree_bytes(&f.repo, f.base, &ticket()));
    commit(&f.repo);
    f.blocked(&[ClosureBlocker::PrimaryReopened]);
}
#[test]
fn ticket_invalid_after_closure() {
    let f = Fixture::new(true);
    let remote = tree_commit(
        &f.repo,
        f.base,
        &[(
            ".manyhands/worktrees/forbidden",
            Some((b"forbidden", 0o100644)),
        )],
    );
    f.tracking("main", remote);
    f.blocked(&[ClosureBlocker::TicketInvalidAfterClosure]);
}
#[test]
fn identity_required() {
    let f = Fixture::new(false);
    f.missing_identity();
    match f.preview() {
        TicketClosurePreflight::Blocked {
            blockers,
            identity_configuration: Some(_),
        } => assert_eq!(blockers, vec![ClosureBlocker::IdentityRequired]),
        other => panic!("expected identity boundary: {other:?}"),
    }
}

#[test]
fn every_blocker_is_reported_in_fixed_order() {
    let f = Fixture::new(true);
    f.missing_identity();
    write(&f.context, "ordinary.txt", "dirty");
    f.pending_conflict();
    f.local_row(ID, "created", None);
    f.comment_binding(("completed", None), Some("failed"));
    write(&f.repo, "ordinary.txt", "dirty");
    f.history("observed_published");
    // The remote primary closed the ticket since; the context is not in it.
    let source = closed_ticket_bytes(
        &tree_bytes(&f.repo, f.base, &ticket()),
        at(1_700_000_000),
        CLOSER,
    )
    .unwrap();
    let closed = tree_commit(&f.repo, f.base, &[(&ticket(), Some((&source, 0o100644)))]);
    f.tracking("main", closed);
    // Identity is missing too, but it is reported only when nothing else is.
    f.blocked(&[
        ClosureBlocker::ContextDirty,
        ClosureBlocker::ConflictPending,
        ClosureBlocker::LocalOperationPending,
        ClosureBlocker::CommentPublicationPending,
        ClosureBlocker::PrimaryDirty,
        ClosureBlocker::RemoteContextDeleted,
        ClosureBlocker::AlreadyClosed,
    ]);
}
#[test]
fn every_blocker_has_a_distinct_fixed_position() {
    let all = super::super::record::CLOSURE_BLOCKERS;
    assert_eq!(all.len(), 21);
    assert!(all.windows(2).all(|w| w[0] < w[1]));
    assert_eq!(all[0], ClosureBlocker::NotATicket);
    assert_eq!(all[20], ClosureBlocker::IdentityRequired);
}
#[test]
fn identity_required_only_when_nothing_else_blocks() {
    let f = Fixture::new(false);
    f.missing_identity();
    write(&f.context, "ordinary.txt", "dirty");
    f.blocked(&[ClosureBlocker::ContextDirty]);
}
#[test]
fn identity_required_only_for_record() {
    let f = Fixture::new(false);
    f.close_context();
    f.missing_identity();
    let p = f.ready();
    assert!(matches!(p.closure(), PlannedClosure::Existing { .. }));
}

// --- Pending work (C12, Review Focus 4) ---

#[test]
fn pending_local_operation_blocks() {
    let f = Fixture::new(false);
    // Rows for another ticket, finished rows and a parked comment hand-off
    // are told apart by the target and the pending rule.
    f.local_row(OTHER, "worktree_observed", None);
    f.local_row(ID, "completed", Some("checkpointed"));
    f.ready();
    let row = f.local_row(ID, "completed", Some("comment_registration_pending"));
    f.blocked(&[ClosureBlocker::LocalOperationPending]);
    f.db()
        .execute(
            "UPDATE operation_records SET completed_step='checkpointed' WHERE id=?1",
            [row],
        )
        .unwrap();
    f.ready();
    // Rechecked under the lease: a row added after preparation blocks.
    let prepared = prepare_observation(&f.service, &f.request(None)).unwrap();
    assert!(prepared.observed.is_ok());
    f.local_row(ID, "failed", None);
    match revalidate_under_lease(&prepared, &f.service, &f.repo).unwrap() {
        Err(blockers) => assert_eq!(blockers, vec![ClosureBlocker::LocalOperationPending]),
        Ok(_) => panic!("pending local work must block under the lease"),
    }
}
#[test]
fn pending_comment_publication_blocks() {
    let f = Fixture::new(false);
    // A checkpointed comment whose publication finished, was cancelled or
    // never started does not block.
    f.comment_binding(("completed", None), Some("completed"));
    f.comment_binding(("completed", None), Some("cancelled"));
    f.comment_binding(("completed", None), None);
    f.ready();
    // Publication interrupted: still pending until it finishes or is abandoned.
    f.comment_binding(("completed", None), Some("interrupted"));
    f.blocked(&[ClosureBlocker::CommentPublicationPending]);
    let f = Fixture::new(false);
    // The comment's own local hand-off is unfinished: its row also names the
    // ticket, so both are reported.
    f.comment_binding(("completed", Some("comment_registration_pending")), None);
    f.blocked(&[
        ClosureBlocker::LocalOperationPending,
        ClosureBlocker::CommentPublicationPending,
    ]);
    let f = Fixture::new(false);
    // A completed publication still awaiting reconciliation is unfinished.
    f.comment_binding(("completed", None), Some("completed"));
    f.db()
        .execute(
            "UPDATE remote_operation_records SET reconciliation_required=1",
            [],
        )
        .unwrap();
    f.blocked(&[ClosureBlocker::CommentPublicationPending]);
    let f = Fixture::new(false);
    let prepared = prepare_observation(&f.service, &f.request(None)).unwrap();
    assert!(prepared.observed.is_ok());
    f.comment_binding(("completed", None), Some("interrupted"));
    match revalidate_under_lease(&prepared, &f.service, &f.repo).unwrap() {
        Err(blockers) => assert_eq!(blockers, vec![ClosureBlocker::CommentPublicationPending]),
        Ok(_) => panic!("pending publication must block under the lease"),
    }
}

// --- The integration table (Observation item 4) ---

#[test]
fn table_contained_base_closed_is_already_contained_existing() {
    let f = Fixture::new(false);
    checkout(&f.repo, f.context_oid());
    f.close_primary();
    let p = f.ready();
    assert_eq!(p.integration(), PlannedIntegration::AlreadyContained);
    assert!(p.changes().is_empty());
    let base_ticket = String::from_utf8(read(&f.repo, &ticket())).unwrap();
    match p.closure() {
        PlannedClosure::Existing {
            closed_at,
            closed_by,
        } => {
            assert_eq!(closed_by, CLOSER);
            assert_eq!(closure_text(base_ticket.as_bytes()).0, *closed_at);
        }
        other => panic!("expected existing: {other:?}"),
    }
}
#[test]
fn table_contained_open_open_records_merge_commit_without_child() {
    let f = Fixture::new(true);
    checkout(&f.repo, f.context_oid());
    f.tracking("main", f.context_oid());
    let p = f.ready();
    assert_eq!(p.integration(), PlannedIntegration::MergeCommit);
    assert_eq!(
        p.closure(),
        &PlannedClosure::Record {
            closed_by: FIXTURE_BY.into()
        }
    );
    assert!(p.changes().is_empty());
    assert!(!p.cleanup().remote_branch);
    let o = f.observation(at(1_700_000_000));
    assert!(!o.needs_context_child);
    assert_eq!(o.base, f.context_oid());
}
#[test]
fn table_contained_open_closed_is_primary_reopened() {
    let f = Fixture::new(false);
    let closed = f.close_context();
    checkout(&f.repo, closed);
    write(&f.repo, &ticket(), tree_bytes(&f.repo, f.base, &ticket()));
    commit(&f.repo);
    f.blocked(&[ClosureBlocker::PrimaryReopened]);
}
#[test]
fn table_not_contained_base_closed_is_already_closed() {
    for close_context in [false, true] {
        let f = Fixture::new(false);
        if close_context {
            f.close_context();
        }
        f.close_primary();
        f.blocked(&[ClosureBlocker::AlreadyClosed]);
    }
}
#[test]
fn table_not_contained_open_open_records_merge_commit() {
    let f = Fixture::new(true);
    let p = f.ready();
    assert_eq!(p.integration(), PlannedIntegration::MergeCommit);
    assert_eq!(
        p.closure(),
        &PlannedClosure::Record {
            closed_by: FIXTURE_BY.into()
        }
    );
    assert_eq!(
        p.changes()
            .iter()
            .map(|c| (c.path(), c.change()))
            .collect::<Vec<_>>(),
        vec![(Path::new(&ticket()), PathChange::Modified)]
    );
    assert_eq!(p.publication(), PlannedPublication::Publish);
    assert_eq!(p.context_oid(), f.context_oid());
    assert_eq!(p.primary_oid(), f.base);
    assert_eq!(p.context_branch(), format!("manyhands/ticket/{ID}"));
    assert_eq!(p.primary_branch(), "main");
    assert_eq!(p.ticket_path(), Path::new(&ticket()));
    assert_eq!(p.item_id(), &id());
    assert_eq!(p.publication_remote(), Some("origin"));
    assert_eq!(p.tracking_primary(), Some(f.base));
    assert_eq!(p.tracking_context(), None);
    assert!(p.cleanup().remote_branch);
    let o = f.observation(at(1_700_000_000));
    assert!(o.needs_context_child);
    assert!(matches!(o.identity, Some(CommittingIdentity::Effective(_))));
}
#[test]
fn context_with_closure_fields_previews_existing() {
    let f = Fixture::new(false);
    f.close_context();
    let p = f.ready();
    assert_eq!(p.integration(), PlannedIntegration::MergeCommit);
    let context_ticket = String::from_utf8(read(&f.context, &ticket())).unwrap();
    match p.closure() {
        PlannedClosure::Existing {
            closed_at,
            closed_by,
        } => {
            assert_eq!(closed_by, CLOSER);
            assert_eq!(closed_at, "2023-11-14T22:13:20Z");
            assert_eq!(closure_text(context_ticket.as_bytes()).0, *closed_at);
        }
        other => panic!("expected existing: {other:?}"),
    }
    // The pre-closure change set includes the ticket the context closed.
    assert_eq!(p.changes().len(), 1);
}
#[test]
fn table_no_context_base_closed_is_already_closed() {
    let f = Fixture::new(false);
    f.close_primary();
    let wt = f.repo.find_worktree(ID).unwrap();
    wt.prune(Some(
        git2::WorktreePruneOptions::new()
            .valid(true)
            .working_tree(true),
    ))
    .unwrap();
    // AlreadyClosed replaces ContextMissing.
    f.blocked(&[ClosureBlocker::AlreadyClosed]);
}
#[test]
fn table_no_context_base_open_is_context_missing() {
    let f = Fixture::new(false);
    let wt = f.repo.find_worktree(ID).unwrap();
    wt.prune(Some(
        git2::WorktreePruneOptions::new()
            .valid(true)
            .working_tree(true),
    ))
    .unwrap();
    f.blocked(&[ClosureBlocker::ContextMissing]);
}

// --- Change set, closed-ticket check (Review Focus 1, 5) ---

#[test]
fn preview_lists_whole_branch_changes() {
    let f = Fixture::new(false);
    write(&f.context, "ordinary.txt", "changed by the ticket\n");
    write(&f.context, "code/new.rs", "fn main() {}\n");
    write(&f.context, "docs/note.md", "not a managed document\n");
    commit(&f.context);
    let first = f.ready();
    assert_eq!(
        first
            .changes()
            .iter()
            .map(|c| (c.path().to_owned(), c.change()))
            .collect::<Vec<_>>(),
        vec![
            (PathBuf::from(ticket()), PathChange::Modified),
            (PathBuf::from("code/new.rs"), PathChange::Added),
            (PathBuf::from("docs/note.md"), PathChange::Added),
            (PathBuf::from("ordinary.txt"), PathChange::Modified),
        ]
    );
    // Any further path changes consent, including one outside the ticket.
    fs::remove_file(f.context.workdir().unwrap().join("code/new.rs")).unwrap();
    commit(&f.context);
    let second = f.ready();
    assert_eq!(second.changes().len(), 3);
    assert_ne!(first.expectation(), second.expectation());
}
#[test]
fn preview_and_execution_share_closed_ticket_check() {
    // A hand-written CRLF ticket with manyhands_* first, quoted scalars and
    // unknown keys previews; its closed form is what execution writes.
    let f = Fixture::new(false);
    let hand = hand_written("\r\n");
    write(&f.context, &ticket(), &hand);
    commit(&f.context);
    let p = f.ready();
    assert_eq!(
        p.closure(),
        &PlannedClosure::Record {
            closed_by: FIXTURE_BY.into()
        }
    );
    let closed = closed_ticket_bytes(hand.as_bytes(), at(1_700_000_000), FIXTURE_BY).unwrap();
    let reparsed = parse_ticket(&closed);
    assert_eq!(reparsed.body, parse_ticket(hand.as_bytes()).body);
    assert!(reparsed.unknown.get("nested").is_some());

    // Each side alone is a valid ticket, but the clean merge holds the same
    // key twice: closed_ticket_bytes refuses the result, so preview does.
    let f = Fixture::new(false);
    let base_source = String::from_utf8(tree_bytes(&f.repo, f.base, &ticket())).unwrap();
    let context_source = base_source.replacen("---\n", "---\nwave: 1\n", 1);
    let lines = base_source.lines().collect::<Vec<_>>();
    let close = lines.iter().skip(1).position(|l| *l == "---").unwrap() + 1;
    let mut primary_lines = lines.clone();
    primary_lines.insert(close, "wave: 2");
    let primary_source = primary_lines.join("\n") + "\n";
    for source in [&context_source, &primary_source] {
        closed_ticket_bytes(source.as_bytes(), at(1_700_000_000), CLOSER).unwrap();
    }
    write(&f.context, &ticket(), &context_source);
    commit(&f.context);
    write(&f.repo, &ticket(), &primary_source);
    commit(&f.repo);
    let mut index = f
        .repo
        .merge_commits(
            &f.repo.head().unwrap().peel_to_commit().unwrap(),
            &f.repo.find_commit(f.context_oid()).unwrap(),
            None,
        )
        .unwrap();
    assert!(!index.has_conflicts(), "the merge must be clean");
    let merged = f
        .repo
        .find_tree(index.write_tree_to(&f.repo).unwrap())
        .unwrap();
    let result = f
        .repo
        .find_blob(merged.get_path(Path::new(&ticket())).unwrap().id())
        .unwrap()
        .content()
        .to_vec();
    assert_eq!(
        closed_ticket_bytes(&result, at(1_700_000_000), CLOSER),
        Err(ClosureContentError::NotATicket)
    );
    f.blocked(&[ClosureBlocker::TicketInvalidAfterClosure]);
}

// --- The digest ---

fn digest_of(o: &ClosureObservation) -> ClosureExpectation {
    expectation(o)
}

#[test]
fn digest_is_stable_and_debug_is_redacted() {
    let f = Fixture::new(false);
    let first = f.ready();
    assert_eq!(first.expectation(), f.ready().expectation());
    assert_eq!(first.expectation(), digest_of(&f.observation(at(0))));
    let debug = format!("{first:?} {:?}", first.closure());
    assert!(!debug.contains("Fixture"), "{debug}");
    assert!(!debug.contains("example.invalid"), "{debug}");
    assert_eq!(
        format!("{:?}", first.expectation()),
        "ClosureExpectation(<redacted>)"
    );
    let request = format!("{:?}", f.request(None));
    assert!(!request.contains(&f.root.path().display().to_string()));
}
#[test]
fn digest_binds_each_fact() {
    let f = Fixture::new(false);
    let mut o = f.observation(at(1_700_000_000));
    assert_eq!(o.changes.len(), 1);
    let original = digest_of(&o);
    let mut seen = vec![original];
    let mut differs = |o: &ClosureObservation, what: &str| {
        let d = digest_of(o);
        assert!(!seen.contains(&d), "{what} must change the digest");
        seen.push(d);
    };
    macro_rules! vary {
        ($field:expr, $value:expr, $what:literal) => {{
            let old = $field.clone();
            $field = $value;
            differs(&o, $what);
            $field = old;
        }};
    }
    vary!(o.item_id, OTHER.parse().unwrap(), "item");
    vary!(o.ticket_path, PathBuf::from("other"), "ticket path");
    vary!(o.context_branch, "other".into(), "context branch");
    vary!(o.primary_branch, "other".into(), "primary branch");
    vary!(o.remote, Some("origin".into()), "remote");
    vary!(
        o.integration,
        PlannedIntegration::AlreadyContained,
        "integration kind"
    );
    vary!(
        o.changes[0].change.path,
        PathBuf::from("other"),
        "change path"
    );
    vary!(o.changes[0].change.change, PathChange::Added, "change kind");
    vary!(o.changes[0].entry.oid, Some(f.base), "change oid");
    vary!(o.changes[0].entry.oid, None, "change oid absence");
    vary!(o.changes[0].entry.mode, 0o100755, "executable mode");
    vary!(o.changes[0].entry.mode, 0o120000, "symlink mode");
    vary!(
        o.closure,
        PlannedClosure::Existing {
            closed_at: "2023-11-14T22:13:20Z".into(),
            closed_by: FIXTURE_BY.into()
        },
        "closure tag"
    );
    vary!(
        o.closure,
        PlannedClosure::Record {
            closed_by: CLOSER.into()
        },
        "closed_by"
    );
    vary!(
        o.closure,
        PlannedClosure::Existing {
            closed_at: "2023-11-14T22:13:21Z".into(),
            closed_by: FIXTURE_BY.into()
        },
        "existing closed_at"
    );
    for member in 0..3 {
        let mut cleanup = o.cleanup;
        match member {
            0 => cleanup.remote_branch = !cleanup.remote_branch,
            1 => cleanup.worktree = !cleanup.worktree,
            _ => cleanup.local_branch = !cleanup.local_branch,
        }
        let old = std::mem::replace(&mut o.cleanup, cleanup);
        differs(&o, "cleanup member");
        o.cleanup = old;
    }
    // Without a remote, each local OID is bound.
    vary!(o.primary_oid, f.context_oid(), "primary oid");
    vary!(o.context_oid, f.base, "context oid");
    // `closed_at` of a record closure, the base and the identity are not
    // digest input: the observation is unchanged otherwise.
    assert_eq!(digest_of(&o), original);
}
#[test]
fn digest_mode_only_changes_are_bound() {
    let f = Fixture::new(false);
    let mut o = f.observation(at(1_700_000_000));
    let regular = digest_of(&o);
    assert_eq!(
        o.changes[0].entry,
        PromotionChangeEntry {
            oid: o.changes[0].entry.oid,
            mode: 0o100644
        }
    );
    o.changes[0].entry.mode = 0o100755;
    let executable = digest_of(&o);
    o.changes[0].entry.mode = 0o120000;
    let symlink = digest_of(&o);
    assert_ne!(regular, executable);
    assert_ne!(regular, symlink);
    assert_ne!(executable, symlink);
}
#[test]
fn remote_digest_excludes_moving_oids() {
    let f = Fixture::new(true);
    let mut o = f.observation(at(1_700_000_000));
    let first = digest_of(&o);
    o.primary_oid = f.context_oid();
    o.context_oid = f.base;
    assert_eq!(digest_of(&o), first, "D6: OIDs are not bound with a remote");
    // A primary-only advance on the remote leaves consent unchanged.
    let remote = tree_commit(
        &f.repo,
        f.base,
        &[("primary-only", Some((b"primary", 0o100644)))],
    );
    f.tracking("main", remote);
    let after = f.ready();
    assert_eq!(after.tracking_primary(), Some(remote));
    assert_eq!(after.expectation(), first);
}
#[test]
fn digest_is_independent_of_time_of_day() {
    let f = Fixture::new(false);
    assert_eq!(
        digest_of(&f.observation(at(1_700_000_000))),
        digest_of(&f.observation(at(1_700_043_210)))
    );
}

// --- Identity (C2) ---

#[test]
fn effective_identity_wins_over_confirmation() {
    let f = Fixture::new(false);
    let confirmation = ConfirmedCommitIdentity {
        confirmation_id: OperationId::new(),
        identity: CommitIdentity {
            name: "Confirmed".into(),
            email: "confirmed@example.invalid".into(),
        },
        expected_configuration: ExpectedConfiguration::for_testing(),
    };
    match f.preview_with(Some(confirmation)) {
        TicketClosurePreflight::Ready(p) => assert_eq!(
            p.closure(),
            &PlannedClosure::Record {
                closed_by: FIXTURE_BY.into()
            }
        ),
        other => panic!("expected ready: {other:?}"),
    }
}
#[test]
fn confirmation_must_match_configuration() {
    let f = Fixture::new(false);
    f.missing_identity();
    let expected = match f.preview() {
        TicketClosurePreflight::Blocked {
            identity_configuration: Some(expected),
            ..
        } => expected,
        other => panic!("expected identity boundary: {other:?}"),
    };
    let confirmation = |expected_configuration| ConfirmedCommitIdentity {
        confirmation_id: OperationId::new(),
        identity: CommitIdentity {
            name: "Confirmed".into(),
            email: "confirmed@example.invalid".into(),
        },
        expected_configuration,
    };
    let confirmed = match f.preview_with(Some(confirmation(expected))) {
        TicketClosurePreflight::Ready(p) => p,
        other => panic!("expected ready: {other:?}"),
    };
    assert_eq!(
        confirmed.closure(),
        &PlannedClosure::Record {
            closed_by: "Confirmed <confirmed@example.invalid>".into()
        }
    );
    // The confirmed identity is consent: another one is another digest.
    let mut other = confirmation(expected);
    other.identity.email = "other@example.invalid".into();
    match f.preview_with(Some(other)) {
        TicketClosurePreflight::Ready(p) => {
            assert_ne!(p.expectation(), confirmed.expectation())
        }
        other => panic!("expected ready: {other:?}"),
    }
    // The configuration changed since: the old confirmation is no
    // confirmation, and the new expected configuration is returned.
    f.repo.config().unwrap().set_str("user.email", "x").unwrap();
    match f.preview_with(Some(confirmation(expected))) {
        TicketClosurePreflight::Blocked {
            blockers,
            identity_configuration: Some(fresh),
        } => {
            assert_eq!(blockers, vec![ClosureBlocker::IdentityRequired]);
            assert_ne!(fresh, expected);
        }
        other => panic!("expected identity boundary: {other:?}"),
    }
}

// --- Read-only ---

fn files(root: &Path) -> BTreeMap<PathBuf, [u8; 32]> {
    fn visit(base: &Path, path: &Path, out: &mut BTreeMap<PathBuf, [u8; 32]>) {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                visit(base, &entry.path(), out);
            } else {
                out.insert(
                    entry.path().strip_prefix(base).unwrap().into(),
                    coordination::snapshot_file_hash(base, &entry.path()),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    visit(root, root, &mut out);
    out
}
fn registry_rows(f: &Fixture) -> BTreeMap<String, Vec<String>> {
    let db = f.db();
    let tables = db
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    tables
        .into_iter()
        .map(|table| {
            let mut stmt = db
                .prepare(&format!("SELECT * FROM \"{}\"", table.replace('"', "\"\"")))
                .unwrap();
            let count = stmt.column_count();
            let mut rows = stmt
                .query_map([], |row| {
                    (0..count)
                        .map(|n| row.get::<_, rusqlite::types::Value>(n))
                        .collect::<Result<Vec<_>, _>>()
                        .map(|values| format!("{values:?}"))
                })
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            rows.sort();
            (table, rows)
        })
        .collect()
}
fn refs(repo: &git2::Repository) -> Vec<(String, Option<Oid>)> {
    let mut refs = repo
        .references()
        .unwrap()
        .map(|r| {
            let r = r.unwrap();
            (r.name().unwrap().to_owned(), r.target())
        })
        .collect::<Vec<_>>();
    refs.sort();
    refs
}
#[test]
fn preview_is_read_only() {
    let f = Fixture::new(true);
    // Diverge the primary so the preview builds an in-memory merge.
    let remote = tree_commit(
        &f.repo,
        f.base,
        &[("primary-only", Some((b"primary", 0o100644)))],
    );
    f.tracking("main", remote);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    f.repo
        .remote_set_url(
            "origin",
            &format!(
                "ssh://fixture@127.0.0.1:{}/repository",
                listener.local_addr().unwrap().port()
            ),
        )
        .unwrap();
    // The repository lease is held throughout: preview never takes it.
    let _lease = RepositoryService::hold_lease_for_testing(
        f.root.path(),
        f.data.path(),
        LeaseKind::Repository,
    )
    .unwrap();
    let before = files(f.root.path());
    let data = registry_rows(&f);
    let references = refs(&f.repo);
    let head = f.repo.head().unwrap().target();
    let context_head = f.context.head().unwrap().target();
    let p = f.ready();
    f.ready();
    assert_eq!(p.changes().len(), 1);
    assert_eq!(
        files(f.root.path()),
        before,
        "no file, lease, index or object"
    );
    assert_eq!(registry_rows(&f), data, "no SQL write");
    assert_eq!(refs(&f.repo), references, "no ref");
    assert_eq!(f.repo.head().unwrap().target(), head);
    assert_eq!(f.context.head().unwrap().target(), context_head);
    // No transport: the only listener counted no connection.
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

/// The closure fields exactly as a file's front matter has them.
fn closure_text(bytes: &[u8]) -> (String, String) {
    match file_closure(bytes) {
        FileClosure::Closed {
            closed_at,
            closed_by,
        } => (closed_at, closed_by),
        _ => panic!("not closed"),
    }
}

// --- Fix round 1 ---

/// D5 does not apply to a contained context: no context child runs, so a
/// deleted (or unknown) remote context is cleanup-only, never a blocker.
#[test]
fn contained_closed_with_deleted_remote_is_cleanup_only() {
    let f = Fixture::new(true);
    f.history("observed_published");
    checkout(&f.repo, f.context_oid());
    let closed = f.close_primary();
    f.tracking("main", closed);
    let p = f.ready();
    assert_eq!(p.integration(), PlannedIntegration::AlreadyContained);
    assert!(!p.cleanup().remote_branch);
    assert_eq!(p.tracking_context(), None);
}
#[test]
fn contained_closed_with_unknown_remote_history_is_cleanup_only() {
    let f = Fixture::new(true);
    f.history("history_unknown");
    f.db()
        .execute("UPDATE remote_polling_state SET history_unknown=1", [])
        .unwrap();
    checkout(&f.repo, f.context_oid());
    let closed = f.close_primary();
    f.tracking("main", closed);
    let p = f.ready();
    assert_eq!(p.integration(), PlannedIntegration::AlreadyContained);
    assert!(!p.cleanup().remote_branch);
}
#[test]
fn contained_open_with_deleted_remote_records_without_child() {
    let f = Fixture::new(true);
    f.history("observed_published");
    checkout(&f.repo, f.context_oid());
    f.tracking("main", f.context_oid());
    let p = f.ready();
    assert_eq!(p.integration(), PlannedIntegration::MergeCommit);
    assert_eq!(
        p.closure(),
        &PlannedClosure::Record {
            closed_by: FIXTURE_BY.into()
        }
    );
    assert!(!p.cleanup().remote_branch);
    assert!(!f.observation(at(1_700_000_000)).needs_context_child);
}

#[test]
fn preserved_allows_only_canonical_relationship_forms() {
    let base = parse_ticket(hand_written("\n").as_bytes());
    let with = |deps: Option<serde_yaml::Value>, parent: Option<serde_yaml::Value>| {
        let mut t = base.clone();
        if let Some(deps) = deps {
            t.unknown.insert("deps".into(), deps);
        }
        if let Some(parent) = parent {
            t.unknown.insert("parent".into(), parent);
        }
        t
    };
    let seq =
        |ids: &[&str]| serde_yaml::Value::Sequence(ids.iter().map(|id| (*id).into()).collect());
    // Canonical forms: reordered, repeated, empty or null deps; null parent.
    assert!(preserved(
        &with(Some(seq(&[OTHER, DEP, DEP])), Some(serde_yaml::Value::Null)),
        &with(Some(seq(&[DEP, OTHER])), None)
    ));
    assert!(preserved(&with(Some(seq(&[])), None), &base));
    assert!(preserved(&with(Some(serde_yaml::Value::Null), None), &base));
    // A lost dependency is refused.
    assert!(!preserved(
        &with(Some(seq(&[OTHER, DEP])), None),
        &with(Some(seq(&[DEP])), None)
    ));
    assert!(!preserved(&with(Some(seq(&[DEP])), None), &base));
    // A changed or dropped non-null parent is refused.
    assert!(!preserved(
        &with(None, Some(DEP.into())),
        &with(None, Some(OTHER.into()))
    ));
    assert!(!preserved(&with(None, Some(DEP.into())), &base));
}

/// A fixed, hand-built observation and its digest. The wire bytes below are
/// written out by hand from the design's encoding ("Observation, Change Set
/// And Expectation"), independently of `expectation`; the pinned hex was
/// frozen from the first run of this test (Task 3 fix round 1) and must
/// never change.
#[test]
fn closure_digest_golden_vector() {
    let oid = |digit: char| Oid::from_str(&digit.to_string().repeat(40)).unwrap();
    let observation = ClosureObservation {
        item_id: id(),
        ticket_path: PathBuf::from(ticket()),
        context_oid: oid('c'),
        base: oid('b'),
        remote: None,
        changes: vec![
            ChangeEvidence {
                change: crate::repository::PromotionPathChange {
                    path: PathBuf::from(ticket()),
                    change: PathChange::Modified,
                },
                entry: PromotionChangeEntry {
                    oid: Some(oid('1')),
                    mode: 0o100755,
                },
            },
            ChangeEvidence {
                change: crate::repository::PromotionPathChange {
                    path: PathBuf::from("gone.txt"),
                    change: PathChange::Deleted,
                },
                entry: PromotionChangeEntry { oid: None, mode: 0 },
            },
        ],
        closure: PlannedClosure::Existing {
            closed_at: "2023-11-14T22:13:20Z".into(),
            closed_by: CLOSER.into(),
        },
        integration: PlannedIntegration::MergeCommit,
        needs_context_child: false,
        cleanup: PlannedCleanup {
            remote_branch: false,
            worktree: true,
            local_branch: true,
        },
        identity: None,
        expectation: ClosureExpectation::from_bytes([0; 32]),
        context_branch: format!("manyhands/ticket/{ID}"),
        primary_branch: "main".into(),
        primary_oid: oid('a'),
        tracking_primary: None,
        tracking_context: None,
    };
    fn field(wire: &mut Vec<u8>, bytes: &[u8]) {
        wire.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
        wire.extend_from_slice(bytes);
    }
    let mut wire = b"manyhands closure v1".to_vec();
    field(&mut wire, ID.as_bytes());
    field(&mut wire, ticket().as_bytes());
    field(&mut wire, format!("manyhands/ticket/{ID}").as_bytes());
    field(&mut wire, b"main");
    wire.push(0); // no remote
    wire.push(0); // merge commit
    wire.extend_from_slice(&2u64.to_be_bytes());
    field(&mut wire, ticket().as_bytes());
    wire.push(1); // modified
    wire.push(1);
    wire.extend_from_slice(oid('1').as_bytes());
    wire.extend_from_slice(&0o100755u32.to_be_bytes());
    field(&mut wire, b"gone.txt");
    wire.push(2); // deleted
    wire.push(0);
    wire.extend_from_slice(&0u32.to_be_bytes());
    wire.push(1); // existing
    field(&mut wire, CLOSER.as_bytes());
    field(&mut wire, b"2023-11-14T22:13:20Z");
    wire.extend_from_slice(&[0, 1, 1]); // remote branch, worktree, local branch
    wire.extend_from_slice(oid('a').as_bytes()); // no remote: primary, context
    wire.extend_from_slice(oid('c').as_bytes());
    let digest = expectation(&observation).to_bytes();
    assert_eq!(digest, *blake3::hash(&wire).as_bytes());
    let hex = digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    assert_eq!(hex, GOLDEN);
}
const GOLDEN: &str = "a585a50f6038f601a962c5ca2051657a80030a17a572fed3b415cbb35f03590c";

/// Identity selection applies only to a `Record` closure: an `Existing`
/// closure previews even when the identity configuration is unreadable as
/// an identity (a stale confirmation for another configuration).
#[test]
fn existing_closure_ignores_identity_selection() {
    let f = Fixture::new(false);
    f.close_context();
    f.missing_identity();
    let stale = ConfirmedCommitIdentity {
        confirmation_id: OperationId::new(),
        identity: CommitIdentity {
            name: String::new(),
            email: String::new(),
        },
        expected_configuration: ExpectedConfiguration::for_testing(),
    };
    match f.preview_with(Some(stale)) {
        TicketClosurePreflight::Ready(p) => {
            assert!(matches!(p.closure(), PlannedClosure::Existing { .. }))
        }
        other => panic!("expected ready: {other:?}"),
    }
    // With an effective identity present, an `Existing` closure still
    // selects none: no selection runs for it at all.
    let f = Fixture::new(false);
    f.close_context();
    let o = match prepare_observation(&f.service, &f.request(None))
        .unwrap()
        .observed
    {
        Ok(o) => o,
        Err(r) => panic!("{:?}", r.blockers),
    };
    assert!(o.identity.is_none());
}
