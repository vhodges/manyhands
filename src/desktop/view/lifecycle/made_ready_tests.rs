//! Made ready on a closure that reached main (D5 design, A3, decision 6;
//! ruling R-20). A real local-only close of a ticket another one depends on
//! lists the dependent by its short code, or its short ID with none (R-19),
//! with the title read from the repository; a promotion and a stop before
//! main show nothing. `index_pending` and `index_unavailable_final` cannot
//! be reached from the desktop's own repositories, so those two tests are
//! canned, built from library types, and each names the library test that
//! produces the real answer; they assert nothing of the closure's effect on
//! the repository. The second answers a real close with its canned envelope
//! (`hold_job_return_answering`), so Open ready tickets lists the
//! repository's own ready tickets.

use gpui_kit::{ElementId, TestAppContext};
use manyhands::{
    repository::{
        ClosureFilter, ItemDtoKind, LifecycleMadeReadyDto, LifecycleMadeReadyState,
        LifecyclePublicationPendingReason, LifecyclePublicationState, MutationDataDto,
        ReadinessFilter, RequestId,
    },
    results::{CleanupEffect, DiscoveryEffect, Outcome, ResultCode},
};

use super::{
    confirm_tests::confirming,
    made_ready::{COULD_NOT_WORK_OUT, MADE_READY_LEAD, MAY_BE_INCOMPLETE, OPEN_READY_TICKETS},
    outcome::ENDING_DONE,
    outcome_canned_tests::{answer, finished, intent, reached, reached_effects, reopen, retry},
    outcome_tests::stopped_at_integrating,
    short_id,
    tests::{DOCUMENT, SLUG, TICKET, closes, code, open, saved_ticket, shell, until_present},
};
use crate::desktop::{
    model::navigation::Pane,
    services::{
        JobKind,
        requests::{DesktopRequestId, Intent},
    },
    test_support::fixtures::item_id,
    view::{
        harness::{Shell, key},
        item::LIFECYCLE_ACTION,
        operations::request_id,
        workspace::Region,
    },
};

/// A ticket that depends on `TICKET` alone, so closing it makes this one
/// ready.
pub(super) const DEPENDENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5KF0";
pub(super) const DEPENDENT_TITLE: &str = "Waits on the close";
/// An open ticket that waits on nothing.
pub(super) const OTHER: &str = "01ARZ3NDEKTSV4RRFFQ69G5KG0";

/// Commits `DEPENDENT` on main, with `extra` front matter, depending on
/// `TICKET`, then `TICKET` saved once so it has a context to close.
pub(super) fn with_dependent(
    fixture: &crate::desktop::test_support::fixtures::Fixture,
    root: &std::path::Path,
    extra: &str,
) {
    fixture.commit_ticket(
        root,
        DEPENDENT,
        DEPENDENT_TITLE,
        &format!("{extra}deps:\n  - {TICKET}\n"),
    );
    saved_ticket(fixture, root, "Unblocks", "");
}

/// Closes `TICKET` through the dialog, confirmed, and waits for its Done.
async fn closed(
    shell: &Shell,
    root: &std::path::Path,
    cx: &mut TestAppContext,
) -> DesktopRequestId {
    confirming(shell, root, cx).await;
    shell.click("lifecycle-confirm", cx);
    let id = closes(shell, cx)[0];
    shell.until_ended(id, cx).await;
    assert_eq!(code(shell, id, cx), Some(ResultCode::Ok));
    until_present(shell, "lifecycle-ending", cx).await;
    assert_eq!(
        shell.label("lifecycle-ending", cx).as_deref(),
        Some(ENDING_DONE)
    );
    id
}

/// The made-ready report of the answer to `id`, as the library gave it.
pub(super) fn report(
    shell: &Shell,
    id: DesktopRequestId,
    cx: &mut TestAppContext,
) -> (
    Option<Vec<LifecycleMadeReadyDto>>,
    Option<LifecycleMadeReadyState>,
) {
    shell.operations(cx).read_with(cx, |operations, _| {
        match operations
            .lifecycle_answer(id)
            .and_then(|e| e.data.as_ref())
        {
            Some(MutationDataDto::Lifecycle(data)) => {
                (data.made_ready.clone(), data.made_ready_state)
            }
            _ => (None, None),
        }
    })
}

/// Waits until the made-ready rows read `expected`.
pub(super) async fn until_rows(shell: &Shell, expected: Vec<String>, cx: &mut TestAppContext) {
    shell
        .until_drawn(cx, "the made-ready rows", move |shell, cx| {
            shell.rows("lifecycle-made-ready", cx) == expected
        })
        .await;
}

/// The labels and values drawn inside the operation area's entry of `id`.
fn entry_texts(shell: &Shell, id: DesktopRequestId, cx: &mut TestAppContext) -> Vec<String> {
    let within = ElementId::from(request_id(id, "entry"));
    shell.frame(cx, |window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .filter(|snapshot| snapshot.path().contains(&within))
            .flat_map(|snapshot| {
                [snapshot.label(), snapshot.value()]
                    .into_iter()
                    .flatten()
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .collect()
    })
}

/// No made-ready part is drawn.
fn none_drawn(shell: &Shell, cx: &mut TestAppContext) {
    for id in [
        "lifecycle-made-ready-lead",
        "lifecycle-made-ready-incomplete",
        "lifecycle-made-ready-unknown",
        "lifecycle-open-ready",
    ] {
        assert!(!shell.present(id, cx), "{id} is not drawn");
    }
    assert!(shell.rows("lifecycle-made-ready", cx).is_empty());
}

/// A3, real: Done on a closure lists the ticket it made ready by its short
/// code and its title, read from the repository; the library's own report
/// holds no title (`tests/mutation_remote/c5_made_ready.rs`
/// `c5_made_ready_one`). The operation area's entry shows no title.
#[gpui_kit::test]
async fn done_lists_made_ready_with_slug_and_title(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        with_dependent(fixture, root, &format!("slug: {SLUG}\n"));
    });
    let id = closed(&shell, &root, cx).await;
    assert_eq!(
        report(&shell, id, cx),
        (
            Some(vec![LifecycleMadeReadyDto {
                id: DEPENDENT.to_owned(),
                slug: Some(SLUG.to_owned()),
            }]),
            Some(LifecycleMadeReadyState::Current)
        ),
        "the library's answer"
    );

    until_rows(&shell, vec![format!("{SLUG} \u{b7} {DEPENDENT_TITLE}")], cx).await;
    assert_eq!(
        shell.label("lifecycle-made-ready-lead", cx).as_deref(),
        Some(MADE_READY_LEAD)
    );
    assert!(!shell.present("lifecycle-made-ready-incomplete", cx));
    assert!(!shell.present("lifecycle-open-ready", cx));

    let entry = entry_texts(&shell, id, cx);
    assert!(!entry.is_empty(), "the entry is drawn");
    assert!(
        entry.iter().all(|text| !text.contains(DEPENDENT_TITLE)),
        "no title in the operation area: {entry:?}"
    );
}

/// R-19, real: a made-ready ticket with no short code is named by its short
/// ID, with its title (`c5_made_ready_many_in_id_order` reports its slug as
/// null).
#[gpui_kit::test]
async fn a_ticket_without_a_slug_shows_its_short_id(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| with_dependent(fixture, root, ""));
    let id = closed(&shell, &root, cx).await;
    let (made_ready, state) = report(&shell, id, cx);
    assert_eq!(state, Some(LifecycleMadeReadyState::Current));
    assert_eq!(
        made_ready,
        Some(vec![LifecycleMadeReadyDto {
            id: DEPENDENT.to_owned(),
            slug: None,
        }])
    );

    until_rows(
        &shell,
        vec![format!("{} \u{b7} {DEPENDENT_TITLE}", short_id(DEPENDENT))],
        cx,
    )
    .await;
}

/// R-20, canned: a closure that reached main but whose index was not yet
/// refreshed answers `discovery_pending` (Failed) with `index_pending` and
/// no tickets; the dialog says the list may be incomplete. The real answer
/// is `tests/mutation_remote/c5_made_ready.rs`
/// `c5_made_ready_index_pending_then_retry` (local-only, `RefreshFailure`).
#[gpui_kit::test]
async fn index_pending_says_the_list_may_be_incomplete(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        saved_ticket(fixture, root, "Pending", "")
    });
    let request = RequestId::new();
    let mut data = reached(ItemDtoKind::Ticket, TICKET, Vec::new());
    data.publication = Some(LifecyclePublicationState::Pending);
    data.publication_oid = None;
    data.publication_pending = Some(LifecyclePublicationPendingReason::NoPublicationRemote);
    data.made_ready = Some(Vec::new());
    data.made_ready_state = Some(LifecycleMadeReadyState::IndexPending);
    let mut effects = reached_effects(CleanupEffect::Complete);
    effects.discovery = DiscoveryEffect::Pending;
    let envelope = answer(
        "ticket close",
        &root,
        ResultCode::DiscoveryPending,
        Outcome::Partial,
        data,
        effects,
        vec![retry(request)],
        request,
    );
    let id = finished(
        &shell,
        1,
        intent(&root, ItemDtoKind::Ticket),
        request,
        envelope,
        cx,
    );
    reopen(&shell, id, cx).await;

    assert_eq!(
        shell
            .label("lifecycle-made-ready-incomplete", cx)
            .as_deref(),
        Some(MAY_BE_INCOMPLETE)
    );
    assert!(shell.rows("lifecycle-made-ready", cx).is_empty());
    assert!(!shell.present("lifecycle-made-ready-unknown", cx));
    assert!(!shell.present("lifecycle-open-ready", cx));
}

/// R-20, canned: a closure that finished but could not read which tickets
/// it made ready (`index_unavailable_final`) says so, and Open ready
/// tickets selects its repository, shows its open, ready tickets and
/// focuses the navigation, whatever repository and filters were shown
/// before: the query holds through the repository's selection. The close is
/// real, held once its library call returned and answered with the canned
/// envelope; the real answer is `tests/mutation_remote/c5_made_ready.rs`
/// `c5_made_ready_failed_read_is_final`. The ready list read afterwards is
/// the repository's own.
#[gpui_kit::test]
async fn index_unavailable_final_offers_open_ready_tickets(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, OTHER, "Ready one", "");
        saved_ticket(fixture, root, "Closing", "");
        let other = fixture.create_repository("b");
        fixture.refresh(&other);
    });
    let other = root.with_file_name("b");
    shell.select(&other, cx);
    shell.workspace.update(cx, |workspace, cx| {
        workspace.set_closure(ClosureFilter::Closed, cx);
        workspace.set_readiness(Some(ReadinessFilter::Blocked), cx);
    });
    shell.run(cx);

    confirming(&shell, &root, cx).await;
    let request = RequestId::new();
    let mut data = reached(ItemDtoKind::Ticket, TICKET, Vec::new());
    data.made_ready = Some(Vec::new());
    data.made_ready_state = Some(LifecycleMadeReadyState::IndexUnavailableFinal);
    let envelope = answer(
        "ticket close",
        &root,
        ResultCode::Ok,
        Outcome::Success,
        data,
        reached_effects(CleanupEffect::Complete),
        Vec::new(),
        request,
    );
    let gate = shell
        .backend
        .hold_job_return_answering(JobKind::TicketClose, envelope);
    shell.click("lifecycle-confirm", cx);
    let id = closes(&shell, cx)[0];
    gate.release();
    shell.until_ended(id, cx).await;
    until_present(&shell, "lifecycle-ending", cx).await;
    assert_eq!(
        shell.label("lifecycle-ending", cx).as_deref(),
        Some(ENDING_DONE)
    );

    assert_eq!(
        shell.label("lifecycle-made-ready-unknown", cx).as_deref(),
        Some(COULD_NOT_WORK_OUT)
    );
    assert!(!shell.present("lifecycle-made-ready-incomplete", cx));
    assert!(!shell.present("lifecycle-made-ready-lead", cx));
    assert_eq!(
        shell.label("lifecycle-open-ready", cx).as_deref(),
        Some(OPEN_READY_TICKETS)
    );
    let selected = |shell: &Shell, cx: &mut TestAppContext| {
        shell.workspace.read_with(cx, |workspace, _| {
            (
                workspace.navigation().selected().cloned(),
                workspace.navigation().pane(),
            )
        })
    };
    assert_eq!(selected(&shell, cx).0, Some(key(&other)), "b is shown");

    shell.click("lifecycle-open-ready", cx);
    shell.run(cx);
    assert!(!shell.present("lifecycle-dialog", cx), "the dialog closed");
    shell
        .until_drawn(cx, "the ready tickets of a", |shell, cx| {
            shell.rows("ticket-row", cx) == ["Ready one"]
        })
        .await;
    assert_eq!(selected(&shell, cx), (Some(key(&root)), Pane::Tickets));
    assert!(shell.selected("filter-open", cx));
    assert!(shell.selected("filter-ready", cx));
    assert!(
        shell.present(Region::Navigation.ring_id(), cx),
        "the navigation has focus"
    );
}

/// R-20, real: a promotion reports no made-ready (its `made_ready_state`
/// is null, as `c5_made_ready_promotion_reports_none` pins), and Done shows
/// none.
#[gpui_kit::test]
async fn a_promotion_shows_no_made_ready(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.create_document(root, DOCUMENT, "docs/plan.md", "Plan");
        // A ticket waiting on nothing the promotion touches.
        with_dependent(fixture, root, "");
    });
    open(
        &shell,
        crate::desktop::model::navigation::OpenTarget::Item {
            repo: key(&root),
            kind: ItemDtoKind::Document,
            id: item_id(DOCUMENT),
        },
        cx,
    );
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-confirm", cx).await;
    shell.click("lifecycle-confirm", cx);
    let id = shell.requests(cx, |request| {
        matches!(request.intent, Intent::DocumentPromote { .. })
    })[0];
    shell.until_ended(id, cx).await;
    assert_eq!(code(&shell, id, cx), Some(ResultCode::Ok));
    until_present(&shell, "lifecycle-ending", cx).await;
    assert_eq!(
        shell.label("lifecycle-ending", cx).as_deref(),
        Some(ENDING_DONE)
    );
    assert_eq!(report(&shell, id, cx), (None, None), "the library's answer");
    shell.run(cx);
    none_drawn(&shell, cx);
}

/// R-20, real: a closure stopped before main reports `not_computed`
/// (`c5_made_ready_before_integration`), and shows no made-ready.
#[gpui_kit::test]
async fn a_stop_before_main_shows_no_made_ready(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| with_dependent(fixture, root, ""));
    let id = stopped_at_integrating(&shell, &root, cx).await;
    assert_eq!(
        report(&shell, id, cx),
        (Some(Vec::new()), Some(LifecycleMadeReadyState::NotComputed)),
        "the library's answer"
    );
    shell.run(cx);
    none_drawn(&shell, cx);
}
