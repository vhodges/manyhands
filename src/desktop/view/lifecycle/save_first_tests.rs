//! The save a lifecycle makes first (D5 design, A2; decision 13; R-17),
//! over real local-only repositories: unsaved edits in the tab, and a draft
//! of the item stored with no session there, are saved before the prepare;
//! a save that does not commit, or cannot start, prepares nothing; and a
//! dialog dismissed while a stored draft waits saves nothing.
//!
//! Each test waits on the worker and allows parking. Editing tests type
//! through `Shell::type_at_end` (D3's rules).

use std::path::Path;

use gpui_kit::TestAppContext;
use manyhands::{
    drafts::{DraftBase, DraftFields, DraftId, DraftTarget, ItemKind},
    repository::ItemDtoKind,
};

use super::{
    SAVING_FIRST,
    tests::{
        TICKET, closes, commits, edit, open, pane, saved_ticket, shell, shown, ticket,
        until_present,
    },
};
use crate::desktop::{
    model::edit::{Phase, fields::missing},
    services::{JobKind, ReadKind},
    test_support::{
        drafts::{hold_elsewhere, store_draft, stored_body},
        fixtures::{Fixture, item_id},
    },
    view::{harness::Shell, item::LIFECYCLE_ACTION},
};

/// A draft of the ticket stored with no session, its fields `fields` gives
/// from the ticket's own, as `store_draft` stores one.
fn store_fields(fixture: &Fixture, root: &Path, fields: impl FnOnce(&mut DraftFields)) -> DraftId {
    let service = fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    let shown = service.show_item(&repo, &item_id(TICKET)).unwrap();
    let mut current = DraftFields::of(&shown).expect("a conforming item");
    fields(&mut current);
    let target = DraftTarget {
        repository: root.to_str().unwrap().to_owned(),
        item_kind: ItemKind::Ticket,
        item_id: item_id(TICKET),
        comment: None,
    };
    match fixture
        .draft_store()
        .create(target, DraftBase::of(&shown), current)
    {
        Ok(claim) => claim.draft().id,
        Err(error) => panic!("the draft must be stored: {error}"),
    }
}

/// A ticket with its own working copy and a draft of it stored with
/// "Stored body", in an open window showing it.
fn stored(cx: &mut TestAppContext) -> (Shell, std::path::PathBuf, DraftId) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    saved_ticket(&fixture, &root, "Stored", "");
    fixture.refresh(&root);
    let draft = store_draft(&fixture, &root, ItemDtoKind::Ticket, TICKET, "Stored body");
    let shell = Shell::open(fixture, cx);
    open(&shell, ticket(&root), cx);
    (shell, root, draft)
}

/// Waits until the dialog has closed.
async fn until_closed(shell: &Shell, cx: &mut TestAppContext) {
    shell
        .until_drawn(cx, "the dialog closing", |shell, cx| {
            !shell.present("lifecycle-dialog", cx)
        })
        .await;
}

/// Decision 13: with unsaved edits, the tab saves them first, while the
/// dialog says so; only once the save has committed is the close prepared.
#[gpui_kit::test]
async fn unsaved_edits_are_saved_before_prepare(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Saved first", "");
    });
    open(&shell, ticket(&root), cx);
    let host = edit(&shell, cx).await;
    let branch = shown(&shell, &root, TICKET)
        .context
        .branch
        .expect("Edit made the ticket's own branch");
    let before = commits(&root, &branch);
    shell.type_at_end(&host, "x", cx);

    let held = shell.backend.hold_job(JobKind::TicketSave);
    shell.click(LIFECYCLE_ACTION, cx);
    held.wait_reached();
    assert_eq!(
        shell.label("lifecycle-state", cx).as_deref(),
        Some(SAVING_FIRST)
    );
    assert_eq!(
        shell.backend.reads_seen(ReadKind::Prepare),
        0,
        "nothing is prepared while the save runs"
    );
    assert!(!shell.present("lifecycle-confirm", cx));

    held.release();
    until_present(&shell, "lifecycle-confirm", cx).await;
    assert_eq!(commits(&root, &branch), before + 1, "the save committed");
    let body = shown(&shell, &root, TICKET).body.unwrap_or_default();
    assert!(body.ends_with('x'), "the typed text was saved");
    assert_eq!(shell.backend.reads_seen(ReadKind::Prepare), 1);
    assert!(closes(&shell, cx).is_empty(), "nothing runs before Confirm");
}

/// R-17: a draft of the ticket stored with no edit session in the tab is
/// unsaved edits too. It is continued and saved through the draft services
/// first, while the dialog says so; only once that save has committed, and
/// the draft is retired, is the close prepared.
#[gpui_kit::test]
async fn a_stored_draft_is_saved_before_prepare(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root, draft) = stored(cx);
    let branch = shown(&shell, &root, TICKET)
        .context
        .branch
        .expect("the ticket's own branch");
    let before = commits(&root, &branch);
    assert!(pane(&shell, cx).read_with(cx, |pane, _| !pane.has_session()));

    let held = shell.backend.hold_job(JobKind::TicketSave);
    shell.click(LIFECYCLE_ACTION, cx);
    // The draft is claimed, its context prepared and its editor opened on
    // the foreground before the save is sent: wait for that save.
    shell
        .until_drawn(cx, "the draft's save", |shell, _| {
            shell.backend.jobs_seen().contains(&JobKind::TicketSave)
        })
        .await;
    held.wait_reached();
    assert_eq!(
        shell.label("lifecycle-state", cx).as_deref(),
        Some(SAVING_FIRST)
    );
    assert_eq!(
        shell.backend.reads_seen(ReadKind::Prepare),
        0,
        "nothing is prepared while the draft is saved"
    );

    held.release();
    until_present(&shell, "lifecycle-confirm", cx).await;
    assert_eq!(commits(&root, &branch), before + 1, "the draft was saved");
    let body = shown(&shell, &root, TICKET).body.unwrap_or_default();
    assert!(
        body.trim_end() == "Stored body",
        "the stored text was saved"
    );
    assert!(
        stored_body(&shell.fixture.draft_store(), draft).is_none(),
        "the saved draft is retired"
    );
    assert_eq!(shell.backend.reads_seen(ReadKind::Prepare), 1);
    assert!(closes(&shell, cx).is_empty(), "nothing runs before Confirm");
}

/// P1: a save that does not commit (another actor changed the ticket, so
/// the library answers `external_change`) prepares nothing: the dialog
/// closes, and the tab shows the save's own outcome, the review.
#[gpui_kit::test]
async fn a_failed_save_prepares_nothing(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Mine", "");
    });
    open(&shell, ticket(&root), cx);
    let host = edit(&shell, cx).await;
    shell.type_at_end(&host, "x", cx);
    shell.fixture.save_ticket(&root, TICKET, "Theirs");

    shell.click(LIFECYCLE_ACTION, cx);
    shell
        .until_drawn(cx, "the save's review", |shell, cx| {
            pane(shell, cx).read_with(cx, |pane, _| pane.edit_phase()) == Phase::Review
                && !shell.present("lifecycle-dialog", cx)
        })
        .await;
    assert_eq!(shell.backend.reads_seen(ReadKind::Prepare), 0, "no prepare");
    assert!(closes(&shell, cx).is_empty());
    assert!(
        shell.present("item-save-changed", cx),
        "the tab shows the save's outcome"
    );
}

/// Review I-1: a stored draft whose save cannot start (its Title is
/// empty, which every save waits on) prepares nothing. The dialog closes,
/// no close is sent, and the tab says why.
#[gpui_kit::test]
async fn a_stored_draft_that_cannot_save_prepares_nothing(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    saved_ticket(&fixture, &root, "Titled", "");
    fixture.refresh(&root);
    let mut emptied = None;
    store_fields(&fixture, &root, |fields| {
        if let DraftFields::Ticket { title, .. } = fields {
            title.clear();
        }
        emptied = missing(fields);
    });
    let why = emptied.expect("an empty Title is missing");
    let shell = Shell::open(fixture, cx);
    open(&shell, ticket(&root), cx);

    shell.click(LIFECYCLE_ACTION, cx);
    until_closed(&shell, cx).await;
    assert_eq!(shell.backend.reads_seen(ReadKind::Prepare), 0, "no prepare");
    assert!(closes(&shell, cx).is_empty());
    assert_eq!(shell.label("toolbar-status", cx).as_deref(), Some(why));
}

/// Review I-2: a stored draft of a ticket another actor has changed since
/// opens the review rather than saving: nothing is prepared or sent, and
/// the dialog closes.
#[gpui_kit::test]
async fn a_stored_draft_that_conflicts_prepares_nothing(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root, _) = stored(cx);
    shell.fixture.save_ticket(&root, TICKET, "Theirs");

    shell.click(LIFECYCLE_ACTION, cx);
    until_closed(&shell, cx).await;
    assert_eq!(shell.backend.reads_seen(ReadKind::Prepare), 0, "no prepare");
    assert!(closes(&shell, cx).is_empty());
    assert_eq!(
        pane(&shell, cx).read_with(cx, |pane, _| pane.edit_phase()),
        Phase::Review
    );
}

/// Review I-2: a stored draft another process holds cannot be claimed:
/// nothing is prepared or sent, the dialog closes, and the draft stays.
#[gpui_kit::test]
async fn a_stored_draft_held_elsewhere_prepares_nothing(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, _, draft) = stored(cx);
    let holder = hold_elsewhere(&shell.fixture, draft);

    shell.click(LIFECYCLE_ACTION, cx);
    until_closed(&shell, cx).await;
    assert_eq!(shell.backend.reads_seen(ReadKind::Prepare), 0, "no prepare");
    assert!(closes(&shell, cx).is_empty());
    drop(holder);
    assert!(stored_body(&shell.fixture.draft_store(), draft).is_some());
}

/// Review M-1: Escape while a stored draft is being continued, before its
/// save, saves nothing the person has not seen: the editor still opens on
/// the draft, which stays stored, and nothing is prepared.
#[gpui_kit::test]
async fn escape_before_a_stored_drafts_save_saves_nothing(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root, draft) = stored(cx);
    let branch = shown(&shell, &root, TICKET).context.branch.unwrap();
    let before = commits(&root, &branch);
    let held = shell.backend.hold_job(JobKind::ContextPrepare);

    shell.click(LIFECYCLE_ACTION, cx);
    shell
        .until_drawn(cx, "the draft's context prepare", |shell, _| {
            shell.backend.jobs_seen().contains(&JobKind::ContextPrepare)
        })
        .await;
    assert_eq!(
        shell.label("lifecycle-state", cx).as_deref(),
        Some(SAVING_FIRST)
    );
    shell.keys("escape", cx);
    assert!(!shell.present("lifecycle-dialog", cx));
    held.release();
    shell.until_editing(cx).await;
    shell.run(cx);

    assert!(
        !shell.backend.jobs_seen().contains(&JobKind::TicketSave),
        "nothing is saved"
    );
    assert_eq!(commits(&root, &branch), before);
    assert!(stored_body(&shell.fixture.draft_store(), draft).is_some());
    assert_eq!(shell.backend.reads_seen(ReadKind::Prepare), 0);
}

/// Review Vthen: a save first that fails after Continue editing's prepare
/// answered `discovery_pending` closes the dialog and leaves nothing armed:
/// the banner's Resume then opens the editor on the draft, which stays
/// stored, and saves nothing the person has not seen.
#[gpui_kit::test]
async fn a_failed_save_first_leaves_no_save_armed(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root, draft) = stored(cx);
    let branch = shown(&shell, &root, TICKET).context.branch.unwrap();
    let before = commits(&root, &branch);
    // The library's own prepare, answered as a hand-off still owed.
    let owed = manyhands::results::Envelope::failure(
        "context prepare",
        manyhands::results::Scope::default(),
        manyhands::results::ResultCode::DiscoveryPending,
        vec![],
    );
    shell
        .backend
        .hold_job_return_answering(JobKind::ContextPrepare, owed)
        .release();

    shell.click(LIFECYCLE_ACTION, cx);
    until_closed(&shell, cx).await;
    shell
        .until_drawn(cx, "the prepare's Resume", |shell, cx| {
            shell.present("item-prepare-resume", cx)
        })
        .await;
    assert_eq!(shell.backend.reads_seen(ReadKind::Prepare), 0, "no prepare");

    shell.click("item-prepare-resume", cx);
    shell.until_editing(cx).await;
    shell.run(cx);

    assert!(
        !shell.backend.jobs_seen().contains(&JobKind::TicketSave),
        "nothing is saved unseen"
    );
    assert_eq!(commits(&root, &branch), before);
    assert!(stored_body(&shell.fixture.draft_store(), draft).is_some());
}
