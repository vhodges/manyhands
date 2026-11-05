//! A conflict's reads (D5 B1), through the services over a real
//! repository.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use gpui_kit::{AppContext, Entity, Task, TestAppContext};
use manyhands::{
    repository::{
        ConflictDto, ConflictPathDto, ConflictPathKind, ConflictPathStructure,
        ConflictReadEligibility, ConflictReadNextAction, ConflictStage, ConflictState,
        ConflictTarget, OperationId,
    },
    results::ResultCode,
};

use super::ConflictSource;
use crate::desktop::{
    model::{
        conflict::{ConflictModel, ConflictSet},
        stamp::{Generations, Stamp, TabId, ViewScope},
    },
    services::{ReadKind, ReadResult, Services},
    test_support::{
        delaying::DelayingBackend,
        fixtures::{Fixture, ticket_path, ticket_source},
    },
};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG0";

/// A view that asks for a conflict and keeps what it was given.
#[derive(Default)]
struct Probe {
    generations: Generations,
    applied: Vec<(Stamp, ReadResult<ConflictModel>)>,
    task: Option<Task<()>>,
}

impl Probe {
    fn read(
        &mut self,
        services: &Services,
        source: ConflictSource,
        root: PathBuf,
        operation: OperationId,
        cx: &mut gpui_kit::Context<Self>,
    ) {
        let stamp = self.generations.next(ViewScope::Tab(TabId(1)));
        self.task = Some(services.read_conflict(
            source,
            root,
            operation,
            stamp,
            cx,
            |probe, stamp, result, _| probe.applied.push((stamp, result)),
        ));
    }
}

fn setup(cx: &mut TestAppContext) -> (Fixture, Arc<DelayingBackend>, Services, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let backend = fixture.delaying_backend();
    let services = cx.update(|cx| Services::start(backend.clone(), cx).without_launch_refresh());
    (fixture, backend, services, root)
}

fn read(
    probe: &Entity<Probe>,
    services: &Services,
    source: ConflictSource,
    root: &Path,
    operation: OperationId,
    cx: &mut TestAppContext,
) {
    let root = root.to_owned();
    probe.update(cx, |probe, cx| {
        probe.read(services, source, root, operation, cx)
    });
    cx.run_until_parked();
}

/// A real local conflict is read by its inspection and the content of its
/// one token, into a canonical set with the inspection's observation
/// (R-11).
#[gpui_kit::test]
fn a_local_conflict_is_read_into_its_model(cx: &mut TestAppContext) {
    let (fixture, backend, services, root) = setup(cx);
    let operation = fixture.conflict_locally(&root, TICKET, "Mine", "Theirs");
    let (inspection, _) = fixture.read_local_conflict(&root, operation);
    let probe = cx.new(|_| Probe::default());

    read(
        &probe,
        &services,
        ConflictSource::Local,
        &root,
        operation,
        cx,
    );

    // The root, the inspection and the one token's content.
    assert_eq!(backend.reads_seen(ReadKind::Conflict), 3);
    probe.read_with(cx, |probe, _| {
        let [(stamp, ReadResult::Ok(model))] = &probe.applied[..] else {
            panic!("one model");
        };
        assert_eq!(stamp.generation, 1);
        assert_eq!(model.observation(), inspection.observation.as_deref());
        let ConflictSet::Canonical(paths) = model.set() else {
            panic!("a canonical set");
        };
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].display_path(), ticket_path(TICKET));
        assert_eq!(paths[0].fields()[0].key, "title");
    });
}

/// A local read of an operation the repository does not have answers the
/// library's failure, with no model.
#[gpui_kit::test]
fn a_local_read_of_no_operation_fails(cx: &mut TestAppContext) {
    let (_fixture, _backend, services, root) = setup(cx);
    let probe = cx.new(|_| Probe::default());

    read(
        &probe,
        &services,
        ConflictSource::Local,
        &root,
        OperationId::new(),
        cx,
    );

    probe.read_with(cx, |probe, _| {
        let [(_, ReadResult::Failed(failure))] = &probe.applied[..] else {
            panic!("one failure");
        };
        assert_ne!(failure.code, ResultCode::Ok);
    });
}

/// Canned: no desktop test reaches a remote conflict. The library's real
/// `show_conflict` is still called, and a conflict of the shape
/// `tests/mutation_remote/conflict_read.rs` `c4_conflict_read_canonical`
/// reads stands in place of its answer (`Backend::after_conflict_read`).
/// Nothing is asserted of the repository.
#[gpui_kit::test]
fn a_remote_read_takes_the_conflict_the_library_read(cx: &mut TestAppContext) {
    let (_fixture, backend, services, root) = setup(cx);
    let operation = OperationId::new();
    let source = |title: &str| ticket_source(TICKET, title, "");
    backend.answer_next_conflict_read(ConflictDto {
        operation_id: operation.to_string(),
        comment_operation_id: None,
        target: ConflictTarget::Ticket,
        item_id: Some(TICKET.to_owned()),
        stage: ConflictStage::Context,
        state: ConflictState::ConflictPending,
        next_action: ConflictReadNextAction::Resolve,
        request_id: None,
        eligibility: Some(ConflictReadEligibility::EligibleCanonical),
        local_parent: "1".repeat(40),
        incoming_parent: "2".repeat(40),
        observation: Some(format!("conflict-v1:{}", "0".repeat(64))),
        historical: false,
        paths: vec![ConflictPathDto {
            display_path: ticket_path(TICKET),
            token: format!("conflict-path-v1:{}", "1".repeat(64)),
            kind: ConflictPathKind::Ticket,
            structure: ConflictPathStructure::Regular,
            content_available: true,
            base: Some(source("Base")),
            local: Some(source("Local")),
            incoming: Some(source("Incoming")),
        }],
    });
    let probe = cx.new(|_| Probe::default());

    read(
        &probe,
        &services,
        ConflictSource::Remote,
        &root,
        operation,
        cx,
    );

    // The root, then the conflict itself.
    assert_eq!(backend.reads_seen(ReadKind::Conflict), 2);
    probe.read_with(cx, |probe, _| {
        let [(_, ReadResult::Ok(model))] = &probe.applied[..] else {
            panic!("one model");
        };
        assert_eq!(
            model.observation(),
            Some(format!("conflict-v1:{}", "0".repeat(64)).as_str())
        );
        let ConflictSet::Canonical(paths) = model.set() else {
            panic!("a canonical set");
        };
        assert_eq!(paths[0].fields()[0].mine.as_deref(), Some("Local"));
        assert_eq!(paths[0].fields()[0].theirs.as_deref(), Some("Incoming"));
    });

    // With no canned conflict, the library's own answer is delivered: no
    // operation of that ID.
    read(
        &probe,
        &services,
        ConflictSource::Remote,
        &root,
        operation,
        cx,
    );
    probe.read_with(cx, |probe, _| {
        let (_, ReadResult::Failed(failure)) = &probe.applied[1] else {
            panic!("the library's failure");
        };
        assert_eq!(failure.code, ResultCode::OperationNotFound);
    });
}
