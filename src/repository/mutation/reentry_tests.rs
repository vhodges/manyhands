use std::path::{Path, PathBuf};

use super::*;
use crate::repository::recovery::PendingOperation;
use bind::{Before, Prepared};
use identity::IntentDigestBuilder;

/// A binding of a command whose completed journal row never records a
/// checkpoint step, whatever the command did: an index refresh, say, or a
/// confirmed command. Only what it says of such a row is asked of it.
struct NeverCheckpoints {
    root: PathBuf,
}

impl Binding for NeverCheckpoints {
    fn command(&self) -> &'static str {
        "test never checkpoints"
    }

    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint {
        EndedWithoutCheckpoint::SaysNothing
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn target(&self) -> String {
        String::new()
    }

    fn fields(&self, builder: IntentDigestBuilder) -> IntentDigestBuilder {
        builder
    }

    fn scope(&self, repository: Option<String>, _position: Option<&Position>) -> Scope {
        Scope {
            repository,
            ..Scope::default()
        }
    }

    fn prepare(&mut self, _service: &RepositoryService) -> Result<Prepared, Answer> {
        unreachable!("the decision is made before any check")
    }

    fn run(&self, _service: &RepositoryService, _accepted: &Accepted) -> Ran {
        unreachable!("the decision is made before any domain call")
    }

    fn reenter(
        &mut self,
        _service: &RepositoryService,
        _accepted: &Accepted,
        _journal: JournalRow,
    ) -> Result<Before, Ran> {
        unreachable!("the decision is made before the evidence is read")
    }

    fn rerun(&self, _service: &RepositoryService, _accepted: &Accepted, _before: &Before) -> Ran {
        unreachable!("the decision is made before any domain call")
    }
}

fn never_checkpoints() -> NeverCheckpoints {
    NeverCheckpoints {
        root: PathBuf::from("/nowhere"),
    }
}

fn a_ticket_create() -> Box<dyn Binding> {
    Mutation::TicketCreate(TicketCreateInput {
        root: PathBuf::from("/nowhere"),
        id: "01ARZ3NDEKTSV4RRFFQ69G5FAW".parse().expect("a ULID"),
        draft: draft(),
        deps: Vec::new(),
        parent: None,
        initials: None,
    })
    .into_binding()
}

fn a_ticket_save() -> Box<dyn Binding> {
    Mutation::TicketSave(TicketSaveInput {
        root: PathBuf::from("/nowhere"),
        id: "01ARZ3NDEKTSV4RRFFQ69G5FAW".parse().expect("a ULID"),
        draft: draft(),
        deps: crate::repository::RelationshipWrite::Unchanged,
        parent: crate::repository::RelationshipWrite::Unchanged,
        observation: String::new(),
    })
    .into_binding()
}

fn draft() -> crate::repository::TicketDraft {
    crate::repository::TicketDraft {
        title: "A title".to_owned(),
        ticket_type: "task".to_owned(),
        status: "open".to_owned(),
        project: None,
        team: None,
        body: String::new(),
    }
}

fn completed(owes_work: bool, checkpointed: bool) -> JournalRow {
    JournalRow::Final {
        kind: FinalKind::Completed,
        owes_work,
        checkpointed,
    }
}

/// Every row a journal can show for an operation.
fn every_row() -> Vec<JournalRow> {
    let mut rows = vec![
        JournalRow::Absent,
        JournalRow::Pending(PendingOperation::Local {
            state: "created".to_owned(),
            step: None,
        }),
    ];
    for kind in [
        FinalKind::Completed,
        FinalKind::Cancelled,
        FinalKind::RetainedForInspection,
    ] {
        for owes_work in [false, true] {
            for checkpointed in [false, true] {
                rows.push(JournalRow::Final {
                    kind,
                    owes_work,
                    checkpointed,
                });
            }
        }
    }
    rows
}

#[test]
fn a_row_that_ended_without_a_checkpoint_does_not_release_the_record_of_a_binding_that_never_checkpoints()
 {
    // The command completed and its output was lost. Its row has no
    // checkpoint step because its operation records none: the record is
    // kept, and the call is not run as a new request.
    let binding = never_checkpoints();
    assert_eq!(
        Reentry::of(&completed(false, false), binding.ended_without_checkpoint()),
        Reentry::AsRecorded
    );
}

#[test]
fn a_binding_that_never_checkpoints_is_re_entered_as_recorded_whatever_its_row() {
    let binding = never_checkpoints();
    for journal in every_row() {
        assert_eq!(
            Reentry::of(&journal, binding.ended_without_checkpoint()),
            Reentry::AsRecorded,
            "{journal:?}"
        );
    }
}

#[test]
fn a_row_without_a_checkpoint_is_taken_as_a_row_with_one_by_a_binding_that_never_checkpoints() {
    let binding = never_checkpoints();
    for kind in [
        FinalKind::Completed,
        FinalKind::Cancelled,
        FinalKind::RetainedForInspection,
    ] {
        for owes_work in [false, true] {
            let row = |checkpointed| JournalRow::Final {
                kind,
                owes_work,
                checkpointed,
            };
            assert_eq!(
                Reentry::of(&row(false), binding.ended_without_checkpoint()),
                Reentry::of(&row(true), binding.ended_without_checkpoint()),
                "{kind:?} {owes_work}"
            );
        }
    }
}

#[test]
fn a_ticket_request_that_ended_without_a_checkpoint_is_run_as_new() {
    for binding in [a_ticket_create(), a_ticket_save()] {
        assert_eq!(
            binding.ended_without_checkpoint(),
            EndedWithoutCheckpoint::DidNothing
        );
        for journal in every_row() {
            let expected = if journal == completed(false, false) {
                Reentry::AsNew
            } else {
                Reentry::AsRecorded
            };
            assert_eq!(
                Reentry::of(&journal, binding.ended_without_checkpoint()),
                expected,
                "{journal:?}"
            );
        }
    }
}
