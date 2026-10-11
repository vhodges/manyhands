//! What every binding of an authoring write owes the boundary, one table
//! over the bound commands: the replay of a finished request, the refusal
//! of a changed one, an interruption retried to completion, a failure
//! after the checkpoint reported as partial, and nothing left behind
//! after a rejection.

use manyhands::{
    repository::{
        DocumentCreateInput, DocumentDraft, DocumentSaveInput, FailurePoint, Mutation,
        RequestState, TicketCreateInput, TicketSaveInput, TicketSlugAssignInput,
    },
    results::{CheckpointEffect, DiscoveryEffect, Effects, Outcome, ResultCode, WriteEffect},
};

use crate::support::{
    FailOnce,
    items::{self, TICKET_A},
    mutation::{
        BODY, DOCUMENT, DOCUMENT_MOVED_PATH, DOCUMENT_NEW, DOCUMENT_NEW_PATH, DOCUMENT_PATH,
        REQUEST_1, REQUEST_2, TICKET_NEW, TITLE, World, recovery_actions,
    },
};

/// What a caller read before it made its request, and sends with every
/// call of it.
struct Tokens {
    /// The token of the request's item, for a request that has one.
    item: String,
    /// The absent token of `DOCUMENT_MOVED_PATH` in the document's context.
    absent: String,
}

/// Which request of a case to make.
#[derive(Clone, Copy)]
enum Input {
    /// The request.
    Same,
    /// The same request with another input: a mismatch under its ID.
    Changed,
    /// A request that is rejected with nothing written.
    Rejected,
}

struct Case {
    command: &'static str,
    /// The item the request writes.
    id: &'static str,
    /// Whether the fixture holds the item before the request.
    exists: bool,
    /// A request made before the case's own, for a command that writes
    /// content its caller did not send: a move writes the document's
    /// title and body where it takes it, and a short code assignment
    /// writes the ticket's fields back. The fixture's own items hold no
    /// sentinel, so without this the privacy scans of those two cases
    /// could not fail. It saves the sentinel title and body, which the
    /// case's request then reads from the file and carries.
    seed: Option<fn(&World) -> Mutation>,
    request: fn(&World, &Tokens, Input) -> Mutation,
    /// The code `Input::Rejected` is rejected with.
    rejection: ResultCode,
}

const OTHER_BODY: &str = "Another body.\n";

fn other_draft() -> DocumentDraft {
    DocumentDraft {
        title: TITLE.to_owned(),
        body: OTHER_BODY.to_owned(),
    }
}

const CASES: [Case; 6] = [
    Case {
        command: "document create",
        id: DOCUMENT_NEW,
        exists: false,
        seed: None,
        request: |world, _, input| {
            let create = |path: &str, draft| {
                Mutation::DocumentCreate(DocumentCreateInput {
                    root: world.root.clone(),
                    id: items::item_id(DOCUMENT_NEW),
                    path: path.into(),
                    draft,
                })
            };
            match input {
                Input::Same => create(DOCUMENT_NEW_PATH, World::document_draft()),
                Input::Changed => create(DOCUMENT_NEW_PATH, other_draft()),
                // A document is at the path.
                Input::Rejected => create(DOCUMENT_PATH, World::document_draft()),
            }
        },
        rejection: ResultCode::OccupiedPath,
    },
    Case {
        command: "document save",
        id: DOCUMENT,
        exists: true,
        seed: None,
        request: |world, tokens, input| match input {
            Input::Same => world.document_save(DOCUMENT, &tokens.item),
            Input::Changed => Mutation::DocumentSave(DocumentSaveInput {
                draft: other_draft(),
                ..world.document_save_input(DOCUMENT, &tokens.item)
            }),
            Input::Rejected => world.document_save(DOCUMENT, "v1:stale"),
        },
        rejection: ResultCode::ExternalChange,
    },
    Case {
        command: "document move",
        id: DOCUMENT,
        exists: true,
        seed: Some(|world| world.document_save(DOCUMENT, &world.token(DOCUMENT))),
        request: |world, tokens, input| match input {
            Input::Same => {
                world.document_move(DOCUMENT, &tokens.item, DOCUMENT_MOVED_PATH, &tokens.absent)
            }
            Input::Changed => world.document_move(
                DOCUMENT,
                &tokens.item,
                "docs/moved/elsewhere.md",
                &tokens.absent,
            ),
            Input::Rejected => {
                world.document_move(DOCUMENT, "v1:stale", DOCUMENT_MOVED_PATH, &tokens.absent)
            }
        },
        rejection: ResultCode::ExternalChange,
    },
    Case {
        command: "ticket slug-assign",
        id: TICKET_A,
        exists: true,
        seed: Some(|world| world.save(TICKET_A, &world.token(TICKET_A))),
        request: |world, tokens, input| match input {
            Input::Same => world.slug_assign(TICKET_A, &tokens.item),
            Input::Changed => Mutation::TicketSlugAssign(TicketSlugAssignInput {
                initials: Some("zz".to_owned()),
                ..world.slug_assign_input(TICKET_A, &tokens.item)
            }),
            Input::Rejected => world.slug_assign(TICKET_A, "v1:stale"),
        },
        rejection: ResultCode::ExternalChange,
    },
    Case {
        command: "ticket create",
        id: TICKET_NEW,
        exists: false,
        seed: None,
        request: |world, _, input| match input {
            Input::Same => world.create(TICKET_NEW),
            Input::Changed => Mutation::TicketCreate(TicketCreateInput {
                draft: World::draft(TITLE, OTHER_BODY),
                ..world.create_input(TICKET_NEW)
            }),
            // A ticket that depends on itself.
            Input::Rejected => Mutation::TicketCreate(TicketCreateInput {
                deps: vec![TICKET_NEW.to_owned()],
                ..world.create_input(TICKET_NEW)
            }),
        },
        rejection: ResultCode::RelationshipCycle,
    },
    Case {
        command: "ticket save",
        id: TICKET_A,
        exists: true,
        seed: None,
        request: |world, tokens, input| match input {
            Input::Same => world.save(TICKET_A, &tokens.item),
            Input::Changed => Mutation::TicketSave(TicketSaveInput {
                draft: World::draft(TITLE, OTHER_BODY),
                ..world.save_input(TICKET_A, &tokens.item)
            }),
            Input::Rejected => world.save(TICKET_A, "v1:stale"),
        },
        rejection: ResultCode::ExternalChange,
    },
];

impl Case {
    fn tokens(&self, world: &World) -> Tokens {
        Tokens {
            item: if self.exists {
                world.token(self.id)
            } else {
                String::new()
            },
            absent: world.observe(DOCUMENT, DOCUMENT_MOVED_PATH),
        }
    }

    /// How many commits the item's branch has before the case's request.
    fn commits(&self) -> usize {
        usize::from(self.seed.is_some())
    }

    /// A world for the case, seeded, with what its caller read from it.
    fn world(&self) -> (World, Tokens) {
        let world = World::new();
        if let Some(seed) = self.seed {
            let seeded = world.execute(REQUEST_2, seed(&world));
            assert_eq!(seeded.code, ResultCode::Ok, "{}: the seed", self.command);
            // What the request will carry holds the sentinels.
            let source = match self.id {
                DOCUMENT => world.context_source(DOCUMENT, DOCUMENT_PATH).unwrap(),
                id => world.worktree_source(id),
            };
            assert!(source.contains(TITLE) && source.contains(BODY), "{source}");
        }
        let tokens = self.tokens(&world);
        (world, tokens)
    }

    /// The same, with a service that fails once at `point`.
    fn failing_at(&self, point: FailurePoint) -> (World, Tokens) {
        let (mut world, tokens) = self.world();
        world.service = FailOnce::at(point).open_service(world.data.path());
        (world, tokens)
    }

    /// Asserts that `envelope` is the request's result, with the one
    /// commit on the item's branch, and that its record is finished.
    fn assert_finished(
        &self,
        world: &World,
        envelope: &manyhands::results::Envelope<manyhands::repository::MutationDataDto>,
        what: &str,
    ) {
        let what = format!("{}: {what}", self.command);
        assert_eq!(
            (envelope.outcome, envelope.code),
            (Outcome::Success, ResultCode::Ok),
            "{what}"
        );
        assert_eq!(envelope.command, self.command, "{what}");
        assert_eq!(envelope.effects.write, WriteEffect::Written, "{what}");
        assert_eq!(
            envelope.effects.checkpoint,
            CheckpointEffect::Committed,
            "{what}"
        );
        assert_eq!(
            envelope.effects.discovery,
            DiscoveryEffect::Current,
            "{what}"
        );
        assert_eq!(world.branch_commits(self.id), self.commits() + 1, "{what}");
        assert_eq!(
            envelope.effects.commit_oid,
            world.branch_tip(self.id).map(|tip| tip.to_string()),
            "{what}"
        );
        assert!(recovery_actions(envelope).is_empty(), "{what}");
        let record = world.record(REQUEST_1).expect("a record");
        assert_eq!(record.state, RequestState::Finished, "{what}");
        assert_eq!(record.command, self.command, "{what}");
        assert_eq!(
            record.result.map(|result| result.effects),
            Some(envelope.effects.clone()),
            "{what}"
        );
        assert_eq!(world.pending_journal_rows(), 0, "{what}");
    }
}

#[test]
fn a_finished_request_is_replayed_and_a_changed_one_is_refused() {
    for case in &CASES {
        let command = case.command;
        let (world, tokens) = case.world();
        let first = world.execute(REQUEST_1, (case.request)(&world, &tokens, Input::Same));
        case.assert_finished(&world, &first, "the first call");
        let tip = world.branch_tip(case.id);
        let record = world.record(REQUEST_1);

        // The same request again, with the tokens it had: by now they are
        // stale, and the stored result is returned all the same.
        let replay = world.execute(REQUEST_1, (case.request)(&world, &tokens, Input::Same));
        assert_eq!(replay, first, "{command}");
        assert_eq!(world.branch_tip(case.id), tip, "{command}");
        assert_eq!(world.record(REQUEST_1), record, "{command}");

        // The same request ID with another input.
        let changed = world.execute(REQUEST_1, (case.request)(&world, &tokens, Input::Changed));
        assert_eq!(
            (changed.outcome, changed.code),
            (Outcome::Error, ResultCode::RequestMismatch),
            "{command}"
        );
        assert_eq!(changed.effects, Effects::not_requested(), "{command}");
        assert_eq!(changed.operation_id, None, "{command}");
        assert!(recovery_actions(&changed).is_empty(), "{command}");
        assert_eq!(world.branch_tip(case.id), tip, "{command}");
        assert_eq!(world.record(REQUEST_1), record, "{command}");
        assert_eq!(world.request_rows(), 1 + case.commits() as i64, "{command}");
    }
}

/// The points before its checkpoint a write can be interrupted at, and
/// whether its file was written by then.
const INTERRUPTIONS: [(FailurePoint, bool); 3] = [
    (FailurePoint::BeforeItemWrite, false),
    (
        FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence,
        true,
    ),
    (FailurePoint::BeforeCheckpointCommit, true),
];

#[test]
fn an_interrupted_request_is_finished_by_its_retry() {
    for case in &CASES {
        for (point, written) in INTERRUPTIONS {
            let what = format!("{}, {point:?}", case.command);
            let (mut world, tokens) = case.failing_at(point);
            let first = world.execute(REQUEST_1, (case.request)(&world, &tokens, Input::Same));
            // Written and not committed is an effect, so the result is
            // partial. Before the write nothing was done: an error, with
            // the operation's row pending all the same.
            let (outcome, write, checkpoint) = if written {
                (
                    Outcome::Partial,
                    WriteEffect::Written,
                    CheckpointEffect::Pending,
                )
            } else {
                (
                    Outcome::Error,
                    WriteEffect::NotRequested,
                    CheckpointEffect::NotRequested,
                )
            };
            assert_eq!(first.outcome, outcome, "{what}");
            assert_eq!(first.effects.write, write, "{what}");
            assert_eq!(first.effects.checkpoint, checkpoint, "{what}");
            assert_eq!(first.effects.commit_oid, None, "{what}");
            assert_eq!(recovery_actions(&first), ["request.retry"], "{what}");
            assert_eq!(world.branch_commits(case.id), case.commits(), "{what}");
            let record = world.record(REQUEST_1).expect("the record stays");
            assert_eq!(
                (record.state, record.attempt),
                (RequestState::Accepted, 1),
                "{what}"
            );

            // A changed reuse of the request ID is refused, and says
            // whether an effect was left.
            let changed = world.execute(REQUEST_1, (case.request)(&world, &tokens, Input::Changed));
            assert_eq!(changed.code, ResultCode::RequestMismatch, "{what}");
            assert_eq!(changed.outcome, outcome, "{what}");
            assert_eq!(changed.effects.write, write, "{what}");
            assert_eq!(changed.effects.checkpoint, checkpoint, "{what}");
            assert_eq!(world.record(REQUEST_1), Some(record), "{what}");

            // A fresh service, as the next process would open, and the
            // same input, tokens included.
            world.reopen();
            let retry = world.execute(REQUEST_1, (case.request)(&world, &tokens, Input::Same));
            case.assert_finished(&world, &retry, &what);
            assert_eq!(retry.operation_id, first.operation_id, "{what}");
            assert_eq!(world.record(REQUEST_1).unwrap().attempt, 2, "{what}");
        }
    }
}

#[test]
fn a_failure_after_the_checkpoint_is_partial_and_its_retry_reports_the_commit() {
    for case in &CASES {
        let command = case.command;
        let (mut world, tokens) = case.failing_at(FailurePoint::BeforeIndexTransactionCommit);
        let first = world.execute(REQUEST_1, (case.request)(&world, &tokens, Input::Same));
        assert_eq!(
            (first.outcome, first.code),
            (Outcome::Partial, ResultCode::DiscoveryPending),
            "{command}"
        );
        assert_eq!(first.effects.write, WriteEffect::Written, "{command}");
        assert_eq!(
            first.effects.checkpoint,
            CheckpointEffect::Committed,
            "{command}"
        );
        assert_eq!(
            first.effects.discovery,
            DiscoveryEffect::Pending,
            "{command}"
        );
        assert_eq!(
            world.branch_commits(case.id),
            case.commits() + 1,
            "{command}"
        );
        let commit = world.branch_tip(case.id).map(|tip| tip.to_string());
        assert_eq!(first.effects.commit_oid, commit, "{command}");
        assert_eq!(recovery_actions(&first), ["operation.resume"], "{command}");
        assert_eq!(
            world.record(REQUEST_1).map(|record| record.state),
            Some(RequestState::Accepted),
            "{command}"
        );

        world.reopen();
        let retry = world.execute(REQUEST_1, (case.request)(&world, &tokens, Input::Same));
        case.assert_finished(&world, &retry, "the retry");
        assert_eq!(retry.effects.commit_oid, commit, "{command}");
    }
}

#[test]
fn a_lost_result_is_answered_with_the_request_s_commit() {
    for case in &CASES {
        let command = case.command;
        let (mut world, tokens) = case.failing_at(FailurePoint::BeforeRequestSettlement);
        let lost = world.execute(REQUEST_1, (case.request)(&world, &tokens, Input::Same));
        assert_eq!(
            (lost.outcome, lost.code),
            (Outcome::Error, ResultCode::InternalError),
            "{command}"
        );
        assert_eq!(
            world.branch_commits(case.id),
            case.commits() + 1,
            "{command}"
        );
        let commit = world.branch_tip(case.id).map(|tip| tip.to_string());

        world.reopen();
        let retry = world.execute(REQUEST_1, (case.request)(&world, &tokens, Input::Same));
        case.assert_finished(&world, &retry, "the retry");
        assert_eq!(retry.effects.commit_oid, commit, "{command}");
    }
}

#[test]
fn a_rejected_request_leaves_nothing_and_its_id_can_be_used_again() {
    for case in &CASES {
        let command = case.command;
        let (world, tokens) = case.world();
        let primary = world.primary_head();
        let document = world.primary_source_at(DOCUMENT_PATH);
        let tip = world.branch_tip(case.id);

        let rejected = world.execute(REQUEST_1, (case.request)(&world, &tokens, Input::Rejected));
        assert_eq!(rejected.code, case.rejection, "{command}");
        assert_eq!(rejected.effects, Effects::not_requested(), "{command}");
        assert_eq!(rejected.operation_id, None, "{command}");
        if case.seed.is_none() {
            world.assert_nothing_exists(REQUEST_1, case.id);
        }
        assert!(world.record(REQUEST_1).is_none(), "{command}");
        assert_eq!(world.pending_journal_rows(), 0, "{command}");
        assert_eq!(world.request_rows(), case.commits() as i64, "{command}");
        assert_eq!(world.branch_tip(case.id), tip, "{command}");
        assert_eq!(world.primary_head(), primary, "{command}");
        assert_eq!(
            world.primary_source_at(DOCUMENT_PATH),
            document,
            "{command}"
        );
        assert!(!world.root.join(DOCUMENT_MOVED_PATH).exists(), "{command}");
        assert!(!world.root.join(DOCUMENT_NEW_PATH).exists(), "{command}");

        // The same request ID, with the input corrected, runs at once.
        let corrected = world.execute(REQUEST_1, (case.request)(&world, &tokens, Input::Same));
        case.assert_finished(&world, &corrected, "the corrected request");
    }
}

/// Every case saves the sentinel title or body, or moves or marks an item
/// the fixture wrote; the world's own scan, when it is dropped, is what
/// shows that no request store kept any of it.
#[test]
fn the_table_covers_every_command_the_boundary_binds() {
    let mut commands: Vec<&str> = CASES.iter().map(|case| case.command).collect();
    commands.sort_unstable();
    assert_eq!(
        commands,
        [
            "document create",
            "document move",
            "document save",
            "ticket create",
            "ticket save",
            "ticket slug-assign",
        ]
    );
}
