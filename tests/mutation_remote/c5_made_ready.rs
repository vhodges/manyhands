//! The made-ready report of `ticket close`: a
//! `c5_made_ready_` case per row of the design's table, and the ways a closure
//! gets to its answer: retried, stored, replayed and resumed.
//!
//! **The failed read and the incomplete read** are staged by
//! `RepositoryService::set_made_ready_hook_for_testing`, which runs once, on the
//! thread that sets it, immediately before the report's
//! read, once the domain has answered. The failed one makes a stored context's
//! kind undecodable (`unreadable_index`), so the read answers `invalid_stored_data`
//! and the refresh before it is untouched. The incomplete one stores a
//! `source_unreadable` problem on the tickets directory (`incomplete_index`).

use std::path::Path;

use super::*;
use lifecycle_world::*;
use results::{CleanupEffect, IntegrationEffect};

pub(crate) const CASES: &[ssh_harness::Case] = &[
    ("c5_made_ready_one", one),
    ("c5_made_ready_many_in_id_order", many_in_id_order),
    (
        "c5_made_ready_another_open_dependency",
        another_open_dependency,
    ),
    ("c5_made_ready_unresolved_dependency", unresolved_dependency),
    (
        "c5_made_ready_status_text_is_no_closure",
        status_text_is_no_closure,
    ),
    ("c5_made_ready_none", none),
    ("c5_made_ready_before_integration", before_integration),
    ("c5_made_ready_checkpoint_only", checkpoint_only),
    ("c5_made_ready_stop_past_integration", stop_past_integration),
    (
        "c5_made_ready_index_pending_then_retry",
        index_pending_then_retry,
    ),
    ("c5_made_ready_failed_read_is_final", failed_read_is_final),
    (
        "c5_made_ready_incomplete_read_is_final",
        incomplete_read_is_final,
    ),
    (
        "c5_made_ready_failed_read_on_partial_stays_owed",
        failed_read_on_partial_stays_owed,
    ),
    (
        "c5_made_ready_partial_reports_and_retry_reads_again",
        partial_reports_and_retry_reads_again,
    ),
    (
        "c5_made_ready_push_rejected_is_index_pending",
        push_rejected_is_index_pending,
    ),
    (
        "c5_made_ready_peer_reopens_the_ticket",
        peer_reopens_the_ticket,
    ),
    (
        "c5_made_ready_remote_context_changed",
        remote_context_changed,
    ),
    ("c5_made_ready_superseded", superseded),
    ("c5_made_ready_superseding_stop", superseding_stop),
    ("c5_made_ready_replay_unchanged", replay_unchanged),
    (
        "c5_made_ready_promotion_reports_none",
        promotion_reports_none,
    ),
    (
        "c5_made_ready_promotion_stop_has_no_data",
        promotion_stop_has_no_data,
    ),
    ("c5_made_ready_resume_reports", resume_reports),
    ("c5_made_ready_resume_stop", resume_stop),
    ("c5_made_ready_resume_index_pending", resume_index_pending),
    ("c5_made_ready_resume_entered_again", resume_entered_again),
    ("c5_made_ready_pending_child", pending_child),
];

const FIRST: &str = "01ARZ3NDEKTSV4RRFFQ69G5FJ1";
const SECOND: &str = "01ARZ3NDEKTSV4RRFFQ69G5FJ2";
const RESUME: &str = "01ARZ3NDEKTSV4RRFFQ69G5FJ5";
const SYNC: &str = "01ARZ3NDEKTSV4RRFFQ69G5FJ6";

const DEPENDENT_A: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD1";
const DEPENDENT_B: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD2";
const DEPENDENT_C: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD3";
const OTHER_OPEN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD8";
const UNRESOLVED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD9";
const SLUG: &str = "mh-vh-k9x2b";
const DEPENDENT_TITLE: &str = "A title that is never reported";

const MUTATION: &str = "lifecycle_mutation.schema.json";

type Report = (
    Option<LifecycleMadeReadyState>,
    Option<Vec<LifecycleMadeReadyDto>>,
);

fn ticket(id: &str, status: &str, extra: &str) -> (String, String) {
    (
        format!(".manyhands/tickets/{id}/ticket.md"),
        format!(
            "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {id}\n\
             title: {DEPENDENT_TITLE}\ntype: task\nstatus: {status}\n{extra}---\nBody.\n"
        ),
    )
}

fn depends_on(ids: &[&str]) -> String {
    format!("deps: [{}]\n", ids.join(", "))
}

/// Commits tickets on primary, by hand, and has the index read them.
fn on_primary(
    service: &RepositoryService,
    root: &Path,
    tickets: &[(String, String)],
) -> Result<(), FixtureError> {
    for (path, source) in tickets {
        commit_in(root, MAIN, path, source.as_bytes())?;
    }
    fixed(service.refresh_repository(RefreshRepositoryRequest {
        root: root.to_owned(),
        operation_id: OperationId::new(),
    }))?;
    Ok(())
}

/// The ticket to close, with one dependent on primary, whose short code is known.
fn local_with_a_dependent() -> Result<Local, FixtureError> {
    let local = Local::holding(Verb::Close)?;
    on_primary(
        &local.service,
        &local.root,
        &[ticket(
            DEPENDENT_A,
            "open",
            &format!("slug: {SLUG}\n{}", depends_on(&[Verb::Close.id()])),
        )],
    )?;
    Ok(local)
}

/// `remote`'s closure, with one dependent a peer published on primary, observed.
fn remote_with_a_dependent() -> Result<World, FixtureError> {
    let w = remote(Verb::Close)?;
    let (path, source) = ticket(
        DEPENDENT_A,
        "open",
        &format!("slug: {SLUG}\n{}", depends_on(&[Verb::Close.id()])),
    );
    let peer = w.peer.workdir().ok_or(FixtureError)?;
    commit_in(peer, MAIN, &path, source.as_bytes())?;
    push_peer(&w.peer, &w.server, MAIN)?;
    fixed(w.sync(w.primary()))?;
    fixed(w.service.refresh_repository(RefreshRepositoryRequest {
        root: w.root.clone(),
        operation_id: OperationId::new(),
    }))?;
    Ok(w)
}

fn the_dependent() -> Vec<LifecycleMadeReadyDto> {
    vec![LifecycleMadeReadyDto {
        id: DEPENDENT_A.to_owned(),
        slug: Some(SLUG.to_owned()),
    }]
}

fn ids(tickets: &[LifecycleMadeReadyDto]) -> Vec<&str> {
    tickets.iter().map(|ticket| ticket.id.as_str()).collect()
}

fn confirmation_for(
    service: &RepositoryService,
    verb: Verb,
    root: &Path,
) -> Result<ConfirmationId, FixtureError> {
    let envelope = service.prepare(&verb.mutation(root, verb.id()));
    let preview = envelope
        .data
        .as_ref()
        .and_then(PrepareDataDto::preview)
        .ok_or(FixtureError)?;
    fixed(ConfirmationId::parse(&preview.confirmation_id))
}

/// One call of `execute`, against the published schemas. No title is reported,
/// of the closed ticket or of the tickets it made ready.
fn called(
    service: &RepositoryService,
    id: &str,
    confirmation: Option<ConfirmationId>,
    mutation: Mutation,
) -> Envelope<MutationDataDto> {
    let envelope = service.execute(
        MutationCall {
            request_id: request(id),
            confirmation,
            mutation,
        },
        &mut SessionCredentials::new(Provider),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    let rendered = serde_json::to_vec(&envelope).expect("an envelope serializes");
    ssh_privacy::clean(
        &rendered,
        &[
            BODY.as_bytes().to_vec(),
            TITLE.as_bytes().to_vec(),
            DEPENDENT_TITLE.as_bytes().to_vec(),
            b"fixture@example.invalid".to_vec(),
        ],
    )
    .expect("no source, title or closed_by in a result");
    let value = serde_json::to_value(&envelope).expect("an envelope serializes");
    schema::check_published("envelope.schema.json", &value).expect("the envelope schema");
    if !value["data"].is_null() {
        schema::check_published(MUTATION, &value["data"]).expect("the lifecycle data schema");
    }
    envelope
}

fn first_call(
    service: &RepositoryService,
    root: &Path,
    verb: Verb,
) -> Result<Envelope<MutationDataDto>, FixtureError> {
    let confirmation = confirmation_for(service, verb, root)?;
    Ok(called(
        service,
        FIRST,
        Some(confirmation),
        verb.mutation(root, verb.id()),
    ))
}

fn resent(service: &RepositoryService, id: &str, root: &Path) -> Envelope<MutationDataDto> {
    let verb = Verb::Close;
    called(service, id, None, verb.mutation(root, verb.id()))
}

fn resumed(service: &RepositoryService, root: &Path, operation: &str) -> Envelope<MutationDataDto> {
    called(
        service,
        RESUME,
        None,
        Mutation::OperationResume(OperationResumeInput {
            root: root.to_owned(),
            operation_id: OperationId::parse(operation).unwrap(),
        }),
    )
}

fn report(envelope: &Envelope<MutationDataDto>) -> Result<Report, FixtureError> {
    match envelope.data.as_ref() {
        Some(MutationDataDto::Lifecycle(data)) => {
            Ok((data.made_ready_state, data.made_ready.clone()))
        }
        _ => {
            eprintln!("no lifecycle data: {:?} {:?}", envelope.code, envelope.data);
            Err(FixtureError)
        }
    }
}

fn state_is(
    envelope: &Envelope<MutationDataDto>,
    state: LifecycleMadeReadyState,
    tickets: &[LifecycleMadeReadyDto],
) -> Result<(), FixtureError> {
    assert_eq!(
        report(envelope)?,
        (Some(state), Some(tickets.to_vec())),
        "{:?}",
        envelope.code
    );
    Ok(())
}

fn request_state(service: &RepositoryService, id: &str) -> Option<RequestState> {
    service
        .request_record(request(id))
        .ok()
        .flatten()
        .map(|record| record.state)
}

/// Serialized, since an answer and its replay are compared whole.
fn whole(envelope: &Envelope<MutationDataDto>) -> serde_json::Value {
    serde_json::to_value(envelope).expect("an envelope serializes")
}

fn registry(data: &Path) -> rusqlite::Connection {
    rusqlite::Connection::open(data.join(REGISTRY_FILE)).expect("the registry")
}

/// The failed read: a stored context's kind no read can decode. What undoes it
/// is `readable_index`.
fn unreadable_index(data: &Path) {
    registry(data)
        .execute(
            "UPDATE contexts SET kind = 'undecodable' WHERE kind = 'primary'",
            [],
        )
        .expect("an undecodable context");
}

fn readable_index(data: &Path) {
    registry(data)
        .execute(
            "UPDATE contexts SET kind = 'primary' WHERE kind = 'undecodable'",
            [],
        )
        .expect("the context restored");
}

/// The incomplete read: the last refresh could not read the tickets directory.
fn incomplete_index(data: &Path) {
    registry(data)
        .execute(
            "INSERT INTO problems (repository_id, context_id, path, code, guidance, observed_at)
             SELECT repository_id, id, '.manyhands/tickets', 'source', 'staged', 0
               FROM contexts WHERE kind = 'primary'",
            [],
        )
        .expect("a staged problem");
}

/// The dependent is reported with its ID and short code, and nothing else.
fn one() -> Result<(), FixtureError> {
    let local = local_with_a_dependent()?;
    let done = first_call(&local.service, &local.root, Verb::Close)?;
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    state_is(&done, LifecycleMadeReadyState::Current, &the_dependent())?;
    assert_eq!(
        request_state(&local.service, FIRST),
        Some(RequestState::Finished)
    );
    assert_eq!(
        whole(&done)["data"]["made_ready"],
        serde_json::json!([{"id": DEPENDENT_A, "slug": SLUG}])
    );
    golden_at(
        &local.root,
        "ticket_close_made_ready",
        Some(MUTATION),
        &done,
    );
    Ok(())
}

/// ID order, whatever order they were written in. A ticket with no short code
/// reports it as null.
fn many_in_id_order() -> Result<(), FixtureError> {
    let local = Local::holding(Verb::Close)?;
    let closes = depends_on(&[Verb::Close.id()]);
    on_primary(
        &local.service,
        &local.root,
        &[
            ticket(DEPENDENT_C, "open", &closes),
            ticket(DEPENDENT_A, "open", &closes),
            ticket(DEPENDENT_B, "open", &closes),
        ],
    )?;
    let done = first_call(&local.service, &local.root, Verb::Close)?;
    let (state, tickets) = report(&done)?;
    assert_eq!(state, Some(LifecycleMadeReadyState::Current));
    let tickets = tickets.ok_or(FixtureError)?;
    assert_eq!(ids(&tickets), [DEPENDENT_A, DEPENDENT_B, DEPENDENT_C]);
    assert!(tickets.iter().all(|ticket| ticket.slug.is_none()));
    Ok(())
}

/// A dependent still waiting for another open ticket isn't ready.
fn another_open_dependency() -> Result<(), FixtureError> {
    let local = Local::holding(Verb::Close)?;
    on_primary(
        &local.service,
        &local.root,
        &[
            ticket(OTHER_OPEN, "open", ""),
            ticket(
                DEPENDENT_A,
                "open",
                &depends_on(&[Verb::Close.id(), OTHER_OPEN]),
            ),
            ticket(DEPENDENT_B, "open", &depends_on(&[Verb::Close.id()])),
        ],
    )?;
    let done = first_call(&local.service, &local.root, Verb::Close)?;
    let (state, tickets) = report(&done)?;
    assert_eq!(state, Some(LifecycleMadeReadyState::Current));
    assert_eq!(ids(&tickets.ok_or(FixtureError)?), [DEPENDENT_B]);
    Ok(())
}

/// Review Focus 3: a dependent that also names an ID no ticket holds is left out.
fn unresolved_dependency() -> Result<(), FixtureError> {
    let local = Local::holding(Verb::Close)?;
    on_primary(
        &local.service,
        &local.root,
        &[ticket(
            DEPENDENT_A,
            "open",
            &depends_on(&[Verb::Close.id(), UNRESOLVED]),
        )],
    )?;
    let done = first_call(&local.service, &local.root, Verb::Close)?;
    state_is(&done, LifecycleMadeReadyState::Current, &[])?;
    Ok(())
}

/// Readiness is lifecycle closure. A dependent whose status text says `closed`
/// with no closure is open, so it is ready; one with a closure is not.
fn status_text_is_no_closure() -> Result<(), FixtureError> {
    let local = Local::holding(Verb::Close)?;
    let closes = depends_on(&[Verb::Close.id()]);
    on_primary(
        &local.service,
        &local.root,
        &[
            ticket(DEPENDENT_A, "closed", &closes),
            ticket(
                DEPENDENT_B,
                "open",
                &format!(
                    "closed_at: 2026-09-30T12:34:56Z\nclosed_by: Ada <ada@example.invalid>\n{closes}"
                ),
            ),
        ],
    )?;
    let done = first_call(&local.service, &local.root, Verb::Close)?;
    let (state, tickets) = report(&done)?;
    assert_eq!(state, Some(LifecycleMadeReadyState::Current));
    assert_eq!(ids(&tickets.ok_or(FixtureError)?), [DEPENDENT_A]);
    Ok(())
}

fn none() -> Result<(), FixtureError> {
    let local = Local::holding(Verb::Close)?;
    let done = first_call(&local.service, &local.root, Verb::Close)?;
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    state_is(&done, LifecycleMadeReadyState::Current, &[])
}

/// Stopped by `point` on a local world: the service that stopped, then a clean
/// one to read and finish with.
fn local_stopped(point: FailurePoint) -> Result<(Local, Envelope<MutationDataDto>), FixtureError> {
    let mut local = local_with_a_dependent()?;
    let confirmation = confirmation_for(&local.service, Verb::Close, &local.root)?;
    let failing = failing_service(&local_data(&local), point)?;
    let stopped = called(
        &failing,
        FIRST,
        Some(confirmation),
        Verb::Close.mutation(&local.root, Verb::Close.id()),
    );
    drop(failing);
    local.service = clean_service(&local_data(&local))?;
    Ok((local, stopped))
}

/// Nothing reached primary: `not_computed` and owed, and the same request reports.
fn before_integration() -> Result<(), FixtureError> {
    let (local, stopped) = local_stopped(FailurePoint::BeforeIntegrationRef)?;
    assert_ne!(stopped.code, ResultCode::Ok);
    assert_ne!(stopped.effects.integration, IntegrationEffect::Complete);
    state_is(&stopped, LifecycleMadeReadyState::NotComputed, &[])?;
    assert_eq!(
        request_state(&local.service, FIRST),
        Some(RequestState::Accepted)
    );
    let done = resent(&local.service, FIRST, &local.root);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    state_is(&done, LifecycleMadeReadyState::Current, &the_dependent())
}

/// A checkpoint is not integration.
fn checkpoint_only() -> Result<(), FixtureError> {
    let (local, stopped) = local_stopped(FailurePoint::AfterClosureCheckpoint)?;
    assert_ne!(stopped.effects.integration, IntegrationEffect::Complete);
    state_is(&stopped, LifecycleMadeReadyState::NotComputed, &[])?;
    assert_eq!(
        request_state(&local.service, FIRST),
        Some(RequestState::Accepted)
    );
    Ok(())
}

/// Stopped once primary holds the closure, which the domain answers as an
/// outcome with its cleanup pending. The domain's refresh has run by then, so
/// the index reads the ticket as closed.
fn stop_past_integration() -> Result<(), FixtureError> {
    let (local, stopped) = local_stopped(FailurePoint::AfterIntegrationRef)?;
    assert_eq!(stopped.effects.integration, IntegrationEffect::Complete);
    state_is(&stopped, LifecycleMadeReadyState::Current, &the_dependent())?;
    assert_eq!(
        request_state(&local.service, FIRST),
        Some(RequestState::Accepted)
    );
    let done = resent(&local.service, FIRST, &local.root);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    state_is(&done, LifecycleMadeReadyState::Current, &the_dependent())
}

/// Review Focus 2: the domain's `IndexPending` is owed, and the set comes from
/// the first complete read, stored when the request finishes.
fn index_pending_then_retry() -> Result<(), FixtureError> {
    let (local, pending) = local_stopped(FailurePoint::RefreshFailure)?;
    assert_eq!(
        pending.code,
        ResultCode::DiscoveryPending,
        "{:?}",
        pending.data
    );
    state_is(&pending, LifecycleMadeReadyState::IndexPending, &[])?;
    assert_eq!(
        request_state(&local.service, FIRST),
        Some(RequestState::Accepted)
    );
    let done = resent(&local.service, FIRST, &local.root);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    state_is(&done, LifecycleMadeReadyState::Current, &the_dependent())?;
    assert_eq!(
        request_state(&local.service, FIRST),
        Some(RequestState::Finished)
    );
    assert_eq!(
        whole(&resent(&local.service, FIRST, &local.root)),
        whole(&done)
    );
    Ok(())
}

/// Decision 13, by the hook's `unreadable_index`: a complete closure isn't held
/// open for its report. It finishes saying none will be computed, and a replay
/// says so too, though a read would succeed by then.
fn failed_read_is_final() -> Result<(), FixtureError> {
    let local = local_with_a_dependent()?;
    let data = local_data(&local);
    let staged = data.clone();
    RepositoryService::set_made_ready_hook_for_testing(move || unreadable_index(&staged));
    let done = first_call(&local.service, &local.root, Verb::Close)?;
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(done.effects.cleanup, CleanupEffect::Complete);
    state_is(&done, LifecycleMadeReadyState::IndexUnavailableFinal, &[])?;
    assert_eq!(
        request_state(&local.service, FIRST),
        Some(RequestState::Finished)
    );
    readable_index(&data);
    assert_eq!(
        whole(&resent(&local.service, FIRST, &local.root)),
        whole(&done)
    );
    Ok(())
}

/// Decision 13, by the hook's `incomplete_index`: a read that may lack tickets
/// reports none of them.
fn incomplete_read_is_final() -> Result<(), FixtureError> {
    let local = local_with_a_dependent()?;
    let staged = local_data(&local);
    RepositoryService::set_made_ready_hook_for_testing(move || incomplete_index(&staged));
    let done = first_call(&local.service, &local.root, Verb::Close)?;
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    state_is(&done, LifecycleMadeReadyState::IndexUnavailableFinal, &[])?;
    assert_eq!(
        request_state(&local.service, FIRST),
        Some(RequestState::Finished)
    );
    golden_at(
        &local.root,
        "ticket_close_index_unavailable_final",
        Some(MUTATION),
        &done,
    );
    assert_eq!(
        whole(&resent(&local.service, FIRST, &local.root)),
        whole(&done)
    );
    Ok(())
}

/// The closure stopped in cleanup by a file of the owner's: published, the index
/// refreshed, the domain's answer complete, and the operation unfinished. `staged`
/// runs before the call that answers.
fn stopped_in_cleanup(
    staged: impl FnOnce(&World),
) -> Result<(World, Envelope<MutationDataDto>, std::path::PathBuf), FixtureError> {
    let mut w = remote_with_a_dependent()?;
    let confirmation = confirmation_for(&w.service, Verb::Close, &w.root)?;
    let failing = failing_service(&w.data(), FailurePoint::AfterPublication)?;
    let _stopped = called(
        &failing,
        FIRST,
        Some(confirmation),
        Verb::Close.mutation(&w.root, Verb::Close.id()),
    );
    drop(failing);
    w.service = clean_service(&w.data())?;
    let untracked = Verb::Close.worktree(&w.root).join("untracked.txt");
    fixed(std::fs::write(&untracked, b"the owner's\n"))?;
    staged(&w);
    let changed = resent(&w.service, FIRST, &w.root);
    assert_eq!(
        changed.code,
        ResultCode::RecoveryRequired,
        "{:?}",
        changed.data
    );
    assert_eq!(
        request_state(&w.service, FIRST),
        Some(RequestState::Accepted)
    );
    Ok((w, changed, untracked))
}

/// The design's gap, for ruling: a read that fails while the operation itself is
/// unfinished. Decision 8 keeps the request owed, so the next call does read
/// again, and `index_unavailable_final` would say what isn't so. By the hook's
/// `unreadable_index`.
fn failed_read_on_partial_stays_owed() -> Result<(), FixtureError> {
    let (w, partial, untracked) = stopped_in_cleanup(|w| {
        let staged = w.data();
        RepositoryService::set_made_ready_hook_for_testing(move || unreadable_index(&staged));
    })?;
    state_is(&partial, LifecycleMadeReadyState::IndexPending, &[])?;
    readable_index(&w.data());
    fixed(std::fs::remove_file(&untracked))?;
    let done = resent(&w.service, FIRST, &w.root);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    state_is(&done, LifecycleMadeReadyState::Current, &the_dependent())
}

/// A partial run reports the set, and each retry reads again.
fn partial_reports_and_retry_reads_again() -> Result<(), FixtureError> {
    let (w, partial, _) = stopped_in_cleanup(|_| ())?;
    state_is(&partial, LifecycleMadeReadyState::Current, &the_dependent())?;
    // The set read now differs: primary now holds the dependent's own closure,
    // and the retry's refresh sees it.
    let (path, source) = ticket(
        DEPENDENT_A,
        "open",
        &format!(
            "closed_at: 2026-09-30T12:34:56Z\nclosed_by: Ada <ada@example.invalid>\n{}",
            depends_on(&[Verb::Close.id()])
        ),
    );
    commit_in(&w.root, MAIN, &path, source.as_bytes())?;
    fixed(w.service.refresh_repository(RefreshRepositoryRequest {
        root: w.root.clone(),
        operation_id: OperationId::new(),
    }))?;
    let again = resent(&w.service, FIRST, &w.root);
    assert_eq!(again.code, ResultCode::RecoveryRequired, "{:?}", again.data);
    state_is(&again, LifecycleMadeReadyState::Current, &[])
}

/// A refused push leaves the domain's `IndexPending` as well: the table's own
/// row, so no read is made, and the retry that gets through reports the set.
fn push_rejected_is_index_pending() -> Result<(), FixtureError> {
    let w = remote_with_a_dependent()?;
    w.server.reject_primary_updates(true)?;
    let partial = first_call(&w.service, &w.root, Verb::Close)?;
    assert_eq!(partial.code, ResultCode::PushRejected, "{:?}", partial.data);
    assert_eq!(partial.effects.discovery, DiscoveryEffect::Pending);
    state_is(&partial, LifecycleMadeReadyState::IndexPending, &[])?;
    assert_eq!(
        request_state(&w.service, FIRST),
        Some(RequestState::Accepted)
    );
    w.server.reject_primary_updates(false)?;
    let done = resent(&w.service, FIRST, &w.root);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    state_is(&done, LifecycleMadeReadyState::Current, &the_dependent())
}

/// What `ticket ready` lists now, of the dependent.
fn ticket_ready_lists_the_dependent(w: &World) -> Result<bool, FixtureError> {
    let repo = fixed(w.service.resolve_repository(&w.root))?;
    let ready = fixed(w.service.ticket_readiness(
        &repo,
        &TicketFilter {
            readiness: Some(ReadinessFilter::Ready),
            ..TicketFilter::default()
        },
    ))?;
    Ok(ready
        .items
        .iter()
        .any(|item| item.id.as_deref() == Some(DEPENDENT_A)))
}

/// A peer reopens the ticket on primary while its closure is stopped in cleanup.
/// The report says what `ticket ready` says: nothing false is stored.
fn peer_reopens_the_ticket() -> Result<(), FixtureError> {
    let (w, partial, untracked) = stopped_in_cleanup(|_| ())?;
    state_is(&partial, LifecycleMadeReadyState::Current, &the_dependent())?;
    merging::fetch_into_peer(&w, MAIN)?;
    let (path, source) = ticket(Verb::Close.id(), "open", "");
    let tip = fixed(w.peer.refname_to_id(MAIN))?;
    let reopened = commit_path(&w.peer, tip, &path, source.as_bytes())?;
    fixed(w.peer.reference(MAIN, reopened, true, "a peer's reopening"))?;
    push_peer(&w.peer, &w.server, MAIN)?;
    let synced = called(&w.service, SYNC, None, repo_sync(&w));
    // Fast-forwarding primary from what the peer pushed leaves nothing to publish.
    assert_eq!(synced.code, ResultCode::AlreadyApplied, "{:?}", synced.data);
    assert_eq!(fixed(w.repo()?.refname_to_id(MAIN))?, reopened);
    fixed(std::fs::remove_file(&untracked))?;
    let done = resent(&w.service, FIRST, &w.root);
    let (state, tickets) = report(&done)?;
    assert_eq!(
        state,
        Some(LifecycleMadeReadyState::Current),
        "{:?}",
        done.code
    );
    assert_eq!(
        !tickets.ok_or(FixtureError)?.is_empty(),
        ticket_ready_lists_the_dependent(&w)?,
        "the report and `ticket ready` agree"
    );
    Ok(())
}

/// The guarded delete meets a peer's push to the context. This row asserts the
/// stop, and no made-ready state: it answers `index_pending` with no set, so
/// a superseding request leaves no set behind in the old request's envelope,
/// contrary to the design's Known Limits. That is before the controller, and
/// pinning either answer here would choose it.
fn remote_context_changed() -> Result<(), FixtureError> {
    let mut w = remote_with_a_dependent()?;
    fixed(w.sync(w.request(SynchronizationTarget::Context {
        kind: AuthoringKind::Ticket,
        item_id: fixed(Verb::Close.id().parse())?,
    })))?;
    let confirmation = confirmation_for(&w.service, Verb::Close, &w.root)?;
    let failing = failing_service(&w.data(), FailurePoint::AfterPublication)?;
    let _stopped = called(
        &failing,
        FIRST,
        Some(confirmation),
        Verb::Close.mutation(&w.root, Verb::Close.id()),
    );
    drop(failing);
    w.service = clean_service(&w.data())?;
    let bare = fixed(w.bare())?;
    let old = fixed(bare.refname_to_id(Verb::Close.branch()))?;
    let peer = commit(&bare, old, "peer-race", b"peer\n")?;
    w.server.race_update(Verb::Close.branch(), old, peer)?;
    let changed = resent(&w.service, FIRST, &w.root);
    assert_eq!(
        changed.code,
        ResultCode::RecoveryRequired,
        "{:?}",
        changed.data
    );
    let Some(MutationDataDto::Lifecycle(data)) = changed.data.as_ref() else {
        return Err(FixtureError);
    };
    assert_eq!(
        data.cleanup_pending,
        Some(LifecycleCleanupPendingReason::RemoteContextChanged)
    );
    Ok(())
}

/// Cleanup stopped on a supersedable reason, as `c5_lifecycle`'s row has it: the
/// old request's partial answer holds the set, the superseding request reports
/// `not_applicable`, and the old request's next answer holds no report.
fn superseded_world() -> Result<(World, Envelope<MutationDataDto>), FixtureError> {
    let (w, changed, untracked) = stopped_in_cleanup(|_| ())?;
    fixed(std::fs::remove_file(&untracked))?;
    fixed(w.sync(w.primary()))?;
    Ok((w, changed))
}

fn superseded() -> Result<(), FixtureError> {
    let (w, changed) = superseded_world()?;
    state_is(&changed, LifecycleMadeReadyState::Current, &the_dependent())?;
    let confirmation = confirmation_for(&w.service, Verb::Close, &w.root)?;
    let replaced = called(
        &w.service,
        SECOND,
        Some(confirmation),
        Verb::Close.mutation(&w.root, Verb::Close.id()),
    );
    assert_eq!(replaced.code, ResultCode::Ok, "{:?}", replaced.data);
    state_is(&replaced, LifecycleMadeReadyState::NotApplicable, &[])?;
    assert_eq!(
        request_state(&w.service, SECOND),
        Some(RequestState::Finished)
    );
    let old = resent(&w.service, FIRST, &w.root);
    assert_eq!(old.code, ResultCode::RequestMismatch);
    assert_eq!(old.data, None);
    Ok(())
}

/// A superseding run that stops in its cleanup is still only a cleanup. A stop
/// with no outcome from the domain is `lifecycle.rs`'s unit row, since past
/// integration the domain answers each stop these worlds can stage as an outcome.
fn superseding_stop() -> Result<(), FixtureError> {
    let (mut w, _) = superseded_world()?;
    let confirmation = confirmation_for(&w.service, Verb::Close, &w.root)?;
    let failing = failing_service(&w.data(), FailurePoint::BeforeBranchDeletion)?;
    let stopped = called(
        &failing,
        SECOND,
        Some(confirmation),
        Verb::Close.mutation(&w.root, Verb::Close.id()),
    );
    drop(failing);
    w.service = clean_service(&w.data())?;
    assert_ne!(stopped.code, ResultCode::Ok);
    state_is(&stopped, LifecycleMadeReadyState::NotApplicable, &[])?;
    let done = resent(&w.service, SECOND, &w.root);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    state_is(&done, LifecycleMadeReadyState::NotApplicable, &[])
}

/// A finished request's envelope is stored: its replay holds the set it ended
/// with, whatever has become of the dependent since.
fn replay_unchanged() -> Result<(), FixtureError> {
    let local = local_with_a_dependent()?;
    let done = first_call(&local.service, &local.root, Verb::Close)?;
    state_is(&done, LifecycleMadeReadyState::Current, &the_dependent())?;
    let db = local.db()?;
    fixed(db.execute(
        "UPDATE discovered_items SET closed_at = 1, closed_by = 'Ada' WHERE item_id = ?1",
        [DEPENDENT_A],
    ))?;
    drop(db);
    assert_eq!(
        whole(&resent(&local.service, FIRST, &local.root)),
        whole(&done)
    );
    Ok(())
}

/// A promotion makes no ticket ready, and says so by null: neither field is a
/// report, and no read is made for one.
fn promotion_reports_none() -> Result<(), FixtureError> {
    let verb = Verb::Promote;
    let local = Local::holding(verb)?;
    let staged = local_data(&local);
    RepositoryService::set_made_ready_hook_for_testing(move || unreadable_index(&staged));
    let done = first_call(&local.service, &local.root, verb)?;
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(report(&done)?, (None, None));
    let value = whole(&done);
    assert_eq!(value["data"]["made_ready"], serde_json::Value::Null);
    assert_eq!(value["data"]["made_ready_state"], serde_json::Value::Null);
    // The hook was not taken: no read was made for a report.
    let undecodable: i64 = fixed(registry(&local_data(&local)).query_row(
        "SELECT COUNT(*) FROM contexts WHERE kind = 'undecodable'",
        [],
        |row| row.get(0),
    ))?;
    assert_eq!(undecodable, 0);
    Ok(())
}

/// A promotion's stop, before it reached primary, holds no data, as before.
fn promotion_stop_has_no_data() -> Result<(), FixtureError> {
    let verb = Verb::Promote;
    let mut local = Local::holding(verb)?;
    let confirmation = confirmation_for(&local.service, verb, &local.root)?;
    let failing = failing_service(&local_data(&local), FailurePoint::BeforeIntegrationRef)?;
    let stopped = called(
        &failing,
        FIRST,
        Some(confirmation),
        verb.mutation(&local.root, verb.id()),
    );
    drop(failing);
    local.service = clean_service(&local_data(&local))?;
    assert_eq!(stopped.data, None);
    Ok(())
}

/// A parent begun outside any request, stopped by `point`.
fn outside(point: FailurePoint) -> Result<(Local, String), FixtureError> {
    let mut local = local_with_a_dependent()?;
    let failing = failing_service(&local_data(&local), point)?;
    let operation_id = OperationId::new();
    let item_id: crate::canonical::ItemId = fixed(Verb::Close.id().parse())?;
    let TicketClosurePreflight::Ready(preview) =
        fixed(failing.preview_ticket_closure(PreviewTicketClosureRequest {
            root: local.root.clone(),
            item_id: item_id.clone(),
            confirmed_identity: None,
        }))?
    else {
        return Err(FixtureError);
    };
    let _stopped = failing.close_ticket(
        CloseTicketRequest {
            root: local.root.clone(),
            operation_id,
            item_id,
            expected: preview.expectation(),
            approval: None,
            confirmed_identity: None,
        },
        &mut SessionCredentials::new(Provider),
    );
    drop(failing);
    local.service = clean_service(&local_data(&local))?;
    Ok((local, operation_id.to_string()))
}

/// `operation resume` reports as the verb does.
fn resume_reports() -> Result<(), FixtureError> {
    let (local, operation) = outside(FailurePoint::AfterConfirmed)?;
    let done = resumed(&local.service, &local.root, &operation);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(done.command, "operation resume");
    state_is(&done, LifecycleMadeReadyState::Current, &the_dependent())
}

/// Stopped under a resume, either side of integration.
fn resume_stop() -> Result<(), FixtureError> {
    let (local, operation) = outside(FailurePoint::AfterConfirmed)?;
    let before = failing_service(&local_data(&local), FailurePoint::BeforeIntegrationRef)?;
    let stopped = resumed(&before, &local.root, &operation);
    assert_ne!(stopped.code, ResultCode::Ok);
    state_is(&stopped, LifecycleMadeReadyState::NotComputed, &[])?;
    drop(before);
    let after = failing_service(&local_data(&local), FailurePoint::AfterIntegrationRef)?;
    let stopped = resumed(&after, &local.root, &operation);
    assert_eq!(stopped.effects.integration, IntegrationEffect::Complete);
    state_is(&stopped, LifecycleMadeReadyState::Current, &the_dependent())
}

fn resume_index_pending() -> Result<(), FixtureError> {
    let (local, operation) = outside(FailurePoint::AfterConfirmed)?;
    let failing = failing_service(&local_data(&local), FailurePoint::RefreshFailure)?;
    let pending = resumed(&failing, &local.root, &operation);
    assert_eq!(
        pending.code,
        ResultCode::DiscoveryPending,
        "{:?}",
        pending.data
    );
    state_is(&pending, LifecycleMadeReadyState::IndexPending, &[])?;
    // No request holds it, so no call will read again.
    assert_eq!(
        request_state(&local.service, RESUME),
        None,
        "the record is gone"
    );
    assert_eq!(actions(&pending), [RecoveryActionKind::IndexRefresh]);
    let next = resumed(&local.service, &local.root, &operation);
    assert_eq!(next.code, ResultCode::AlreadyApplied, "{:?}", next.data);
    assert_eq!(next.data, None);
    Ok(())
}

/// R-33's route: a resume that died after finishing the parent is entered again
/// and gets the stored outcome, with the set read for it, stored and replayed.
fn resume_entered_again() -> Result<(), FixtureError> {
    let (local, operation) = outside(FailurePoint::AfterConfirmed)?;
    let lost = failing_service(&local_data(&local), FailurePoint::BeforeRequestSettlement)?;
    let died = resumed(&lost, &local.root, &operation);
    assert_eq!(died.code, ResultCode::InternalError);
    drop(lost);
    assert_eq!(
        request_state(&local.service, RESUME),
        Some(RequestState::Accepted)
    );
    let answered = resumed(&local.service, &local.root, &operation);
    assert_eq!(answered.code, ResultCode::Ok, "{:?}", answered.data);
    state_is(
        &answered,
        LifecycleMadeReadyState::Current,
        &the_dependent(),
    )?;
    assert_eq!(
        request_state(&local.service, RESUME),
        Some(RequestState::Finished)
    );
    assert_eq!(
        whole(&resumed(&local.service, &local.root, &operation)),
        whole(&answered)
    );
    Ok(())
}

/// A child is taken up by a resume of its own ID,
/// and that answer holds no data, so no made-ready; the parent's request then
/// reports it.
fn pending_child() -> Result<(), FixtureError> {
    let w = remote_with_a_dependent()?;
    let confirmation = confirmation_for(&w.service, Verb::Close, &w.root)?;
    w.server.disconnect_at(FixtureBoundary::Advertisement);
    let stopped = called(
        &w.service,
        FIRST,
        Some(confirmation),
        Verb::Close.mutation(&w.root, Verb::Close.id()),
    );
    w.server.clear_fault();
    assert_eq!(stopped.code, ResultCode::TransportUnavailable);
    state_is(&stopped, LifecycleMadeReadyState::NotComputed, &[])?;
    let operation = stopped.operation_id.clone().ok_or(FixtureError)?;
    let child: String = fixed(fixed(w.db())?.query_row(
        "SELECT context_sync_id FROM ticket_closures WHERE operation_id = ?1",
        [operation],
        |row| row.get(0),
    ))?;
    let taken_up = resumed(&w.service, &w.root, &child);
    assert_eq!(taken_up.code, ResultCode::Ok, "{:?}", taken_up.data);
    assert_eq!(taken_up.data, None, "no lifecycle's report, and no other");
    let done = resent(&w.service, FIRST, &w.root);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    state_is(&done, LifecycleMadeReadyState::Current, &the_dependent())
}
