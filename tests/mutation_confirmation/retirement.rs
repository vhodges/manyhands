//! Public confirmation/schema contract using a real local failed index handoff.
use super::*;
use manyhands::repository::{
    AuthoringKind, ContextRetireInput, OperationAbandonInput, OperationId,
};

#[test]
fn ordinary_preserving_preview_binds_retained_resources_and_keeps_old_request() {
    let world = World::with_service(|data| {
        support::FailOnce::at(FailurePoint::BeforeIndexTransactionCommit).open_service(data)
    });
    let item = mutation::TICKET_NEW;
    let retire = || {
        Mutation::ContextRetire(ContextRetireInput {
            root: world.root.clone(),
            kind: AuthoringKind::Ticket,
            id: items::item_id(item),
        })
    };
    let pending = world.execute(REQUEST_1, retire());
    assert_eq!(pending.code, ResultCode::DiscoveryPending, "{pending:?}");
    let operation = OperationId::parse(pending.operation_id.as_deref().unwrap()).unwrap();
    let abandon = || {
        Mutation::OperationAbandon(OperationAbandonInput {
            root: world.root.join("."),
            operation_id: operation,
        })
    };
    let preview = world
        .service
        .prepare(&abandon())
        .data
        .unwrap()
        .into_preview()
        .unwrap();
    let json = serde_json::to_value(&preview).unwrap();
    let check = |value: &serde_json::Value| {
        schema::check_published("operation_abandon_preview.schema.json", value)
    };
    check(&json).unwrap();
    for key in json["details"]["retirement"].as_object().unwrap().keys() {
        let mut missing = json.clone();
        missing["details"]["retirement"]
            .as_object_mut()
            .unwrap()
            .remove(key);
        assert!(
            check(&missing).is_err(),
            "missing retirement metadata {key}"
        );
    }
    let mut missing = json.clone();
    missing["details"]
        .as_object_mut()
        .unwrap()
        .remove("retirement");
    assert!(
        check(&missing).is_err(),
        "retirement is required, even when null"
    );
    let mut extra = json;
    extra["details"]["retirement"]["source"] = "private".into();
    assert!(check(&extra).is_err(), "retirement metadata is closed");
    let PreviewDetailsDto::OperationAbandon(details) = &preview.details else {
        panic!("wrong preview")
    };
    let metadata = details.retirement.as_ref().unwrap();
    assert_eq!(metadata.request_id.as_deref(), Some(REQUEST_1));
    assert!(metadata.index_pending);
    assert!(!metadata.branch_retained);
    assert!(!effects(&preview).contains(&PreviewEffect::RequestReleased));
    // Retained resources are consent-bound as well as phase/epoch/flags.
    let git = git2::Repository::open(&world.root).unwrap();
    let branch = format!("refs/heads/manyhands/ticket/{item}");
    let tip = git.head().unwrap().target().unwrap();
    git.reference(&branch, tip, false, "recreated after preview")
        .unwrap();
    let stale = execute_confirmed(
        &world.service,
        REQUEST_2,
        &preview.confirmation_id,
        abandon(),
    );
    assert_eq!(stale.code, ResultCode::ExternalChange);
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Accepted
    );
    assert_eq!(git.refname_to_id(&branch).unwrap(), tip);
    let preview = world
        .service
        .prepare(&abandon())
        .data
        .unwrap()
        .into_preview()
        .unwrap();
    let kept = execute_confirmed(
        &world.service,
        REQUEST_3,
        &preview.confirmation_id,
        abandon(),
    );
    assert_eq!(kept.code, ResultCode::Ok, "{kept:?}");
    assert_eq!(git.refname_to_id(&branch).unwrap(), tip);
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Finished
    );
    let replay = world.execute(REQUEST_1, retire());
    assert_eq!(replay.code, ResultCode::ExternalChange);
    assert_eq!(
        serde_json::to_value(replay.data).unwrap()["state"],
        "preserved"
    );
    assert_eq!(world.pending_journal_rows(), 0);
}
