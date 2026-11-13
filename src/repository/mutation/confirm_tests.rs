use std::{cell::Cell, path::PathBuf};

use super::{
    super::{
        super::{OperationId, recovery::PendingOperation},
        bind::{EndedWithoutCheckpoint, Prepared},
        evidence::Position,
        identity::{IntentDigestBuilder, RequestId},
    },
    *,
};

/// A binding of a command that needs confirmation which says of every
/// row that the request's work is recorded: the one answer on which
/// `reenter` compares nothing.
struct AlwaysRecorded {
    root: PathBuf,
    observed: Cell<bool>,
    /// Says `Row::Unfinished`, as a parent record does, not `Row::Recorded`.
    parent: bool,
}

impl Binding for AlwaysRecorded {
    fn command(&self) -> Command {
        Command::RepoRemove
    }

    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint {
        EndedWithoutCheckpoint::SaysNothing
    }

    fn place(&self) -> Place<'_> {
        Place::Repository(&self.root)
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

    fn prepare(&mut self, _call: &CallContext<'_>) -> Result<Prepared, Answer> {
        unreachable!("the request is entered again, not prepared")
    }

    fn run(&self, _call: &CallContext<'_>, _accepted: &Accepted) -> Ran {
        unreachable!("the request is entered again, not run")
    }

    fn reenter(
        &mut self,
        call: &CallContext<'_>,
        accepted: &Accepted,
        journal: JournalRow,
    ) -> Result<Entered, Ran> {
        reenter(self, call, accepted, journal)
    }

    fn as_confirmed(&mut self) -> Option<&mut dyn Confirmed> {
        Some(self)
    }

    fn rerun(&self, _call: &CallContext<'_>, _accepted: &Accepted, _entered: &Entered) -> Ran {
        unreachable!("only the decision to call the domain is asked")
    }
}

impl Confirmed for AlwaysRecorded {
    fn preview(&mut self, _call: &CallContext<'_>) -> Result<Previewed, Answer> {
        unreachable!("the request is entered again, not previewed")
    }

    fn row(&mut self, _call: &CallContext<'_>, _accepted: &Accepted, _journal: &JournalRow) -> Row {
        if self.parent {
            Row::Unfinished
        } else {
            Row::Recorded
        }
    }

    fn observe_again(
        &mut self,
        _call: &CallContext<'_>,
        _accepted: &Accepted,
        _journal: &JournalRow,
    ) -> Result<Seen, Ran> {
        self.observed.set(true);
        Err(Ran {
            answer: Answer::stopped(ResultCode::ExternalChange),
            standing: Standing::Stopped { journal: None },
        })
    }
}

fn entered_again(journal: JournalRow) -> (Result<Entered, Ran>, bool) {
    entered_with(journal, false)
}

fn entered_with(journal: JournalRow, parent: bool) -> (Result<Entered, Ran>, bool) {
    let data = tempfile::tempdir().expect("a data directory");
    let service = RepositoryService::open_at(data.path()).expect("a service");
    let call = CallContext {
        service: &service,
        session: &NoSession,
        confirmation: None,
    };
    let accepted = Accepted {
        request_id: RequestId::new(),
        attempt: 1,
        scope: ScopeKey::application(),
        family: None,
        operation_id: OperationId::new(),
        position: Position {
            base_ref: None,
            base_oid: None,
        },
        expected: None,
    };
    let mut binding = AlwaysRecorded {
        root: PathBuf::from("/nowhere"),
        observed: Cell::new(false),
        parent,
    };
    let entered = reenter(&mut binding, &call, &accepted, journal);
    (entered, binding.observed.get())
}

#[test]
fn a_binding_that_says_work_is_recorded_for_a_request_with_no_row_is_not_run() {
    // Nothing would be compared with what was previewed, and no row says
    // the request did anything: the domain is not called, and the record
    // is left as it is.
    let (entered, observed) = entered_again(JournalRow::Absent);
    let Err(ran) = entered else {
        panic!("the domain would be called with nothing compared");
    };
    assert_eq!(ran.answer.code, ResultCode::InternalError);
    assert!(matches!(ran.standing, Standing::Stopped { journal: None }));
    assert!(!observed);
}

#[test]
fn a_row_that_recorded_the_requests_work_is_taken_up_with_nothing_compared() {
    let (entered, observed) = entered_again(JournalRow::Pending(PendingOperation::Local {
        state: "created".to_owned(),
        step: None,
    }));
    assert!(matches!(entered, Ok(Entered::Unfinished)));
    assert!(!observed);
}

#[test]
fn a_parent_record_with_no_row_is_taken_up_with_nothing_compared() {
    let (entered, observed) = entered_with(JournalRow::Absent, true);
    assert!(matches!(entered, Ok(Entered::Unfinished)));
    assert!(!observed);
}

#[test]
fn a_parent_record_with_a_row_is_taken_up_with_nothing_compared() {
    let (entered, observed) = entered_with(
        JournalRow::Pending(PendingOperation::Local {
            state: "created".to_owned(),
            step: None,
        }),
        true,
    );
    assert!(matches!(entered, Ok(Entered::Unfinished)));
    assert!(!observed);
}
