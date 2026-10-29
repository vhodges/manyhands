//! Save, over real repositories (design, Part C, Phases, steps 5 and 6;
//! Save states): the session after a committed save (R-69, R-70), the
//! design's session-outcome rows for Task 15 (R-76), the save states and
//! their words (R-14), the untouched bytes of a saved file, the keys, and
//! the remotes read after a save, never on load (fact 41).
//!
//! The helpers are `edit_tests`'s. Each test sets the caret, focuses the
//! editor and draws a frame before typing, and types through the window;
//! each waits on the worker and allows parking. Draft fields and items
//! would print text through `Debug`, so they are compared with `assert!`.

use manyhands::{
    drafts::{Draft, DraftStore, Entry},
    repository::{
        CancellationToken, FailurePoint, Mutation, MutationCall, NoProgress, RelationshipWrite,
        RequestId, TicketDraft, TicketSaveInput, keys::SessionCredentials,
    },
    results::{
        CheckpointEffect, CleanupEffect, DiscoveryEffect, Effects, IntegrationEffect, Outcome,
        PublicationEffect, WriteEffect,
    },
};

use super::*;
#[cfg(unix)]
use crate::desktop::test_support::drafts::{FolderMode, superuser};
use crate::desktop::{
    model::edit::save_state::{
        REVIEW_BEFORE_SAVING, SAVE_TOOLTIP, SaveState, before_save, save_banner, save_offer,
        save_state,
    },
    test_support::fixtures::NoPassphrase,
    view::item::pill::save_pill,
};

const SAVING: &str = "Saving\u{2026}";
const REVIEWING: &str = "Reviewing changes";
const EDITING_NOT_SHARED: &str = "Editing \u{b7} saved on this computer \u{b7} not shared yet";
/// Fact 81's first shape: a second load of what the first one gives would
/// change it again.
const FACT_81: &str = "wer $$\na $$b$$ c\n$$ wer\n";

/// The save requests of this session, newest first.
fn saves(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| {
        matches!(
            request.intent,
            Intent::TicketSave { .. } | Intent::DocumentSave { .. }
        )
    })
}

/// Commits reachable from `branch` of `root`.
fn commits(root: &Path, branch: &str) -> usize {
    let repository = git2::Repository::open(root).unwrap();
    let tip = repository
        .revparse_single(&format!("refs/heads/{branch}"))
        .unwrap()
        .peel_to_commit()
        .unwrap();
    let mut walk = repository.revwalk().unwrap();
    walk.push(tip.id()).unwrap();
    walk.count()
}

/// The branch of the item's own context, once Edit made it.
fn branch_of(shell: &Shell, root: &Path, id: &str) -> String {
    shown(shell, root, id)
        .context
        .branch
        .expect("the item's own branch")
}

/// Presses the header's Save, then waits until the save it sent has ended
/// and everything after it landed: the registry's read, the retirement or
/// the rebase, and the tab's read.
async fn save(shell: &Shell, cx: &mut TestAppContext) -> DesktopRequestId {
    let before = saves(shell, cx).len();
    shell.click("item-save", cx);
    let sent = saves(shell, cx);
    assert_eq!(sent.len(), before + 1, "Save sent one save");
    ended(shell, sent[0], cx).await;
    sent[0]
}

/// The finished request `id`'s outcome, code and effects.
fn answer(
    shell: &Shell,
    id: DesktopRequestId,
    cx: &mut TestAppContext,
) -> Option<(Outcome, ResultCode, Effects)> {
    shell
        .operations(cx)
        .read_with(cx, |operations, _| match &operations.request(id)?.state {
            RequestState::Finished(outcome, code, effects, ..) => {
                Some((*outcome, *code, effects.clone()))
            }
            _ => None,
        })
}

/// The stored draft `id`, as the store lists it.
fn stored(store: &DraftStore, id: DraftId) -> Option<Draft> {
    store
        .list()
        .unwrap()
        .into_iter()
        .find(|listing| listing.id == id)
        .and_then(|listing| match listing.entry {
            Entry::Draft(draft) => Some(draft),
            Entry::Unreadable(_) => None,
        })
}

/// Undoes in the edit editor, at most three times, until it holds
/// `before`: whether undo reaches it.
fn undo_reaches(
    shell: &Shell,
    host: &Entity<EditorHost>,
    before: &str,
    cx: &mut TestAppContext,
) -> bool {
    let editor = host_editor(host, cx);
    let end = text(&editor, cx).len();
    focus_at(shell, &editor, end, cx);
    for _ in 0..3 {
        shell.keys("secondary-z", cx);
        if text(&editor, cx) == before {
            return true;
        }
    }
    false
}

fn edit_text(host: &Entity<EditorHost>, cx: &mut TestAppContext) -> String {
    host.read_with(cx, |host, cx| host.text(cx))
}

fn dirty(host: &Entity<EditorHost>, cx: &mut TestAppContext) -> bool {
    host.read_with(cx, |host, _| host.is_dirty())
}

fn phase(shell: &Shell, cx: &mut TestAppContext) -> Phase {
    pane(shell, cx).read_with(cx, |pane, _| pane.edit_phase())
}

// The design's session-outcome rows for Task 15 (R-76), written first.

/// Row 1: Edit, type, Save, type, Save. Two commits; undo after the first
/// save still reaches the text before it (R-69: no `set_text`); the second
/// save opens no review. The ticket's file lists its `deps` out of ID
/// order: the base holds them in ID order (R-72), as the library writes
/// them, so the read after the first save equals what it sent.
#[gpui_kit::test]
async fn edit_save_edit_save_commits_twice_keeps_undo_and_opens_no_review(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, DEP_A, "First dependency", "");
        fixture.commit_ticket(root, DEP_B, "Second dependency", "");
        let deps = format!("deps:\n  - {DEP_B}\n  - {DEP_A}\n");
        fixture.commit_ticket(root, TICKET, "Saved twice", &deps);
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let branch = branch_of(&shell, &root, TICKET);
    let before = commits(&root, &branch);
    let original = edit_text(&host, cx);

    type_at_end(&shell, &host, "x", cx);
    let first = save(&shell, cx).await;
    assert_eq!(code(&shell, first, cx), Some(ResultCode::Ok));
    assert_eq!(commits(&root, &branch), before + 1, "the first commit");
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_SAVED));

    type_at_end(&shell, &host, "y", cx);
    let second = save(&shell, cx).await;
    assert_ne!(second, first, "a second save, not a review");
    assert_eq!(code(&shell, second, cx), Some(ResultCode::Ok));
    assert_eq!(commits(&root, &branch), before + 2, "two commits");
    assert_eq!(phase(&shell, cx), Phase::Editing, "no review opened");
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_SAVED));
    let item = shown(&shell, &root, TICKET);
    assert!(item.body.as_deref() == Some(format!("{original}xy").as_str()));
    let deps: Vec<&str> = item.deps.iter().map(|dep| dep.id.as_str()).collect();
    assert_eq!(
        deps,
        [DEP_A, DEP_B],
        "in ID order, as the library writes them"
    );

    assert!(
        undo_reaches(&shell, &host, &original, cx),
        "undo reaches the text before the first save"
    );
}

/// Row 2: Edit, type, Save; a second actor commits between our commit and
/// the registry's read after it; type; Save. The base the save left is
/// unverified, so the review opens before anything is written (R-36).
#[gpui_kit::test]
async fn a_second_actor_between_commit_and_read_makes_the_next_save_review(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Mine", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let branch = branch_of(&shell, &root, TICKET);
    let before = commits(&root, &branch);
    let original = edit_text(&host, cx);
    type_at_end(&shell, &host, "x", cx);

    let read = shell.backend.hold_read(ReadKind::DraftItem);
    shell.click("item-save", cx);
    let request = saves(&shell, cx)[0];
    shell.until_ended(request, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    assert_eq!(shell.backend.reads_seen(ReadKind::DraftItem), 1);
    shell
        .fixture
        .save_ticket(&root, TICKET, "Someone else's title");
    read.send(()).unwrap();
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    assert_eq!(commits(&root, &branch), before + 2, "ours, then theirs");

    type_at_end(&shell, &host, "y", cx);
    assert_eq!(
        shell.label("item-save", cx).as_deref(),
        Some("Review before saving")
    );
    assert_eq!(
        shell.label("item-save-unverified", cx).as_deref(),
        Some("Check the current version before saving. Your edits are kept.")
    );
    assert_eq!(
        shell.label("item-save-review", cx).as_deref(),
        Some("Review\u{2026}")
    );
    let sent = saves(&shell, cx).len();
    shell.click("item-save", cx);
    assert_eq!(saves(&shell, cx).len(), sent, "nothing is sent");
    assert_eq!(phase(&shell, cx), Phase::Review, "the review opens");
    assert_eq!(pill(&shell, cx).as_deref(), Some(REVIEWING));
    assert_eq!(commits(&root, &branch), before + 2, "nothing is written");
    let item = shown(&shell, &root, TICKET);
    assert_eq!(item.title.as_deref(), Some("Someone else's title"));
    assert!(item.body.as_deref() == Some(format!("{original}x").as_str()));
}

/// Row 3: typing while the save is in flight. Only the saved revision is
/// committed; the draft is not retired, the newer text stays dirty, and
/// the next Save saves it.
#[gpui_kit::test]
async fn typing_during_the_save_stays_dirty_and_is_saved_next(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "In flight", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let branch = branch_of(&shell, &root, TICKET);
    let before = commits(&root, &branch);
    let original = edit_text(&host, cx);
    type_at_end(&shell, &host, "x", cx);

    let held = shell.backend.hold_job(JobKind::TicketSave);
    shell.click("item-save", cx);
    held.wait_reached();
    shell.run(cx);
    assert_eq!(pill(&shell, cx).as_deref(), Some(SAVING));
    type_at_end(&shell, &host, "y", cx);
    held.release();
    let request = saves(&shell, cx)[0];
    ended(&shell, request, cx).await;

    let item = shown(&shell, &root, TICKET);
    assert!(item.body.as_deref() == Some(format!("{original}x").as_str()));
    assert_eq!(commits(&root, &branch), before + 1);
    assert!(edit_text(&host, cx) == format!("{original}xy"));
    assert!(dirty(&host, cx), "the newer text stays dirty");
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_UNSAVED));
    let id = attached(&shell, &root, TICKET, cx).expect("the draft is not retired");
    assert!(stored_body(&shell.fixture.draft_store(), id) == Some(format!("{original}xy")));

    save(&shell, cx).await;
    let item = shown(&shell, &root, TICKET);
    assert!(item.body.as_deref() == Some(format!("{original}xy").as_str()));
    assert_eq!(commits(&root, &branch), before + 2);
    assert!(!dirty(&host, cx));
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_SAVED));
    assert_eq!(attached(&shell, &root, TICKET, cx), None, "now it retired");
}

/// Row 4: typing after the save commits, before `DraftSaved` lands (the
/// registry's read is held). The text stays in the editor; once the event
/// lands it is drafted on the base the save left.
#[gpui_kit::test]
async fn typing_before_draft_saved_lands_is_drafted_on_the_post_save_base(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Landing", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let original = edit_text(&host, cx);
    type_at_end(&shell, &host, "x", cx);

    let read = shell.backend.hold_read(ReadKind::DraftItem);
    shell.click("item-save", cx);
    let request = saves(&shell, cx)[0];
    shell.until_ended(request, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok), "committed");
    type_at_end(&shell, &host, "y", cx);
    assert!(edit_text(&host, cx) == format!("{original}xy"));
    read.send(()).unwrap();
    shell.run(cx);
    shell.frame(cx, |_, _| ());

    assert!(
        edit_text(&host, cx) == format!("{original}xy"),
        "the text stays"
    );
    assert!(dirty(&host, cx));
    let saved = shown(&shell, &root, TICKET);
    assert!(saved.body.as_deref() == Some(format!("{original}x").as_str()));
    let id = attached(&shell, &root, TICKET, cx).expect("a draft holds it");
    let draft = stored(&shell.fixture.draft_store(), id).expect("stored");
    assert!(body_of(&draft.current) == format!("{original}xy"));
    let base = draft.base.expect("a base");
    assert_eq!(base.observation, saved.observation, "the post-save base");
    assert!(body_of(&base.fields) == format!("{original}x"));
}

/// Row 5: a clean editor after a save, read again with an equal body. The
/// body is fact 81's shape, which a second load would change: the editor
/// is left alone, with no `set_text`, so undo is kept, and the session's
/// next draft takes the read's observation.
#[gpui_kit::test]
async fn a_clean_editor_re_read_with_an_equal_body_keeps_undo(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        let source = ticket_with_body(TICKET, "Equal", "", FACT_81);
        fixture.commit_file(root, &ticket_path(TICKET), source.as_bytes());
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let original = edit_text(&host, cx);
    assert!(original == load(FACT_81));
    assert!(load(&original) != original, "a second load would change it");
    type_at_end(&shell, &host, "x", cx);
    save(&shell, cx).await;

    let saved = shown(&shell, &root, TICKET);
    assert!(saved.body.as_deref() == Some(format!("{original}x").as_str()));
    assert!(edit_text(&host, cx) == format!("{original}x"));
    assert!(!dirty(&host, cx));
    let observation = pane(&shell, cx).read_with(cx, |pane, _| {
        pane.edit_session().unwrap().base().observation.clone()
    });
    assert_eq!(observation, saved.observation, "the observation is adopted");
    // Read again, as a refresh does.
    pane(&shell, cx).update(cx, |pane, cx| {
        pane.reload(cx);
    });
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    assert!(edit_text(&host, cx) == format!("{original}x"));
    assert!(!dirty(&host, cx));

    assert!(
        undo_reaches(&shell, &host, &original, cx),
        "undo is kept: no set_text"
    );
}

// The other tests.

/// Every row of the design's Save states table, and its words (R-14).
#[test]
fn save_state_maps_every_row_of_the_design_table_to_its_words() {
    let effects = |checkpoint, discovery, publication| Effects {
        write: WriteEffect::Written,
        checkpoint,
        discovery,
        publication,
        integration: IntegrationEffect::NotRequested,
        cleanup: CleanupEffect::NotRequested,
        commit_oid: None,
    };
    let committed = effects(
        CheckpointEffect::Committed,
        DiscoveryEffect::Current,
        PublicationEffect::NotRequested,
    );
    let unchanged = effects(
        CheckpointEffect::Unchanged,
        DiscoveryEffect::Current,
        PublicationEffect::NotRequested,
    );

    // Dirty, not yet saved: the pill, and Save.
    let unsaved = before_save(true);
    assert_eq!(unsaved, SaveState::Unsaved);
    assert_eq!(
        save_state(ResultCode::Cancelled, &Effects::not_requested(), false),
        SaveState::Unsaved,
        "a save that wrote nothing"
    );
    assert_eq!(save_pill(unsaved).map(pill_words), Some(EDITING_UNSAVED));
    assert_eq!(save_banner(unsaved), None);
    assert_eq!(save_offer(unsaved), Some("Save"));

    // `ok` or `already_applied`, committed or unchanged.
    for (code, effects) in [
        (ResultCode::Ok, &committed),
        (ResultCode::Ok, &unchanged),
        (ResultCode::AlreadyApplied, &unchanged),
    ] {
        let local = save_state(code, effects, false);
        assert_eq!(
            local,
            SaveState::SavedLocally {
                publication_pending: false
            }
        );
        assert_eq!(save_pill(local).map(pill_words), Some(EDITING_SAVED));
        assert_eq!(save_banner(local), None);
        assert_eq!(save_offer(local), None);
        // The same, with a selected publication remote.
        let unshared = save_state(code, effects, true);
        assert_eq!(
            unshared,
            SaveState::SavedLocally {
                publication_pending: true
            }
        );
        assert_eq!(
            save_pill(unshared).map(pill_words),
            Some(EDITING_NOT_SHARED)
        );
        assert_eq!(save_offer(unshared), None);
    }

    // `discovery_pending`: a banner, and Resume.
    let owed = effects(
        CheckpointEffect::Committed,
        DiscoveryEffect::Pending,
        PublicationEffect::NotRequested,
    );
    let pending = save_state(ResultCode::DiscoveryPending, &owed, false);
    assert_eq!(pending, SaveState::DiscoveryPending);
    assert_eq!(save_pill(pending), None);
    assert_eq!(
        save_banner(pending),
        Some("Saved on this computer, but the search index is not updated yet. Nothing is lost.")
    );
    assert_eq!(save_offer(pending), Some("Resume"));

    // A pending checkpoint, or an error after acceptance: Retry.
    let checkpoint = effects(
        CheckpointEffect::Pending,
        DiscoveryEffect::NotRequested,
        PublicationEffect::NotRequested,
    );
    for code in [ResultCode::InternalError, ResultCode::Busy, ResultCode::Ok] {
        let failed = save_state(code, &checkpoint, true);
        assert_eq!(failed, SaveState::CheckpointFailed);
        assert_eq!(save_pill(failed), None);
        assert_eq!(
            save_banner(failed),
            Some("Could not save a checkpoint on this computer. Your text is kept.")
        );
        assert_eq!(save_offer(failed), Some("Retry"));
    }

    // `external_change`: the review opens, under the change banner.
    let changed = save_state(ResultCode::ExternalChange, &Effects::not_requested(), false);
    assert_eq!(changed, SaveState::Changed);
    assert_eq!(save_pill(changed), None);
    assert_eq!(
        save_banner(changed),
        Some("Someone changed this file outside Manyhands. Your edits are kept.")
    );
    assert_eq!(save_offer(changed), None);

    // An unverified base: a banner, and Review….
    let review = before_save(false);
    assert_eq!(review, SaveState::ReviewBeforeSaving);
    assert_eq!(save_pill(review), None);
    assert_eq!(
        save_banner(review),
        Some("Check the current version before saving. Your edits are kept.")
    );
    assert_eq!(save_offer(review), Some("Review\u{2026}"));
    // Save, on that base, and its tooltip.
    assert_eq!(REVIEW_BEFORE_SAVING, "Review before saving");
    assert_eq!(SAVE_TOOLTIP, "Save a checkpoint on this computer (Ctrl S)");

    // Any other refusal: the library's message and code, after these
    // words, with its own recovery actions.
    for code in [
        ResultCode::WorktreeNotClean,
        ResultCode::TicketClosed,
        ResultCode::InvalidInput,
    ] {
        let refused = save_state(code, &Effects::not_requested(), false);
        assert_eq!(refused, SaveState::Refused(code));
        assert_eq!(save_pill(refused), None);
        assert_eq!(
            save_banner(refused),
            Some("Could not save on this computer. Your edits are kept.")
        );
        assert_eq!(save_offer(refused), None);
    }
}

/// Phases, step 5: Save is enabled only while dirty and not saving. A
/// press on a clean editor, or while a save is in flight, sends nothing.
#[gpui_kit::test]
async fn save_is_offered_only_when_dirty_and_not_while_saving(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Offered", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    assert!(!shell.present("item-save", cx), "not while viewing");
    let host = edit(&shell, cx).await;
    assert_eq!(shell.label("item-save", cx).as_deref(), Some("Save"));
    shell.click("item-save", cx);
    assert!(saves(&shell, cx).is_empty(), "clean: nothing to save");

    type_at_end(&shell, &host, "x", cx);
    let held = shell.backend.hold_job(JobKind::TicketSave);
    shell.click("item-save", cx);
    held.wait_reached();
    shell.run(cx);
    assert_eq!(saves(&shell, cx).len(), 1);
    assert_eq!(phase(&shell, cx), Phase::Saving);
    shell.click("item-save", cx);
    shell.keys("secondary-s", cx);
    assert_eq!(saves(&shell, cx).len(), 1, "not while saving");
    assert_eq!(phase(&shell, cx), Phase::Saving, "still saving");

    held.release();
    ended(&shell, saves(&shell, cx)[0], cx).await;
    assert_eq!(phase(&shell, cx), Phase::Editing);
    shell.click("item-save", cx);
    assert_eq!(saves(&shell, cx).len(), 1, "clean again");
}

/// A document's save commits once on its own branch; the pill then reads
/// "Editing · saved on this computer", and the draft retired, so the tab
/// holds none.
#[gpui_kit::test]
async fn a_document_save_commits_once_and_the_pill_reads_saved_on_this_computer(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.create_document(root, DOCUMENT, "docs/saved.md", "Saved document");
    });
    open(
        &shell,
        OpenTarget::Item {
            repo: key(&root),
            kind: ItemDtoKind::Document,
            id: item_id(DOCUMENT),
        },
        cx,
    );
    let host = edit(&shell, cx).await;
    let item = shown(&shell, &root, DOCUMENT);
    let branch = item.context.branch.clone().expect("its own branch");
    let before = commits(&root, &branch);
    let original = edit_text(&host, cx);
    type_at_end(&shell, &host, "More.", cx);
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_UNSAVED));

    let request = save(&shell, cx).await;
    let document_save = shell.operations(cx).read_with(cx, |operations, _| {
        operations
            .request(request)
            .is_some_and(|request| matches!(request.intent, Intent::DocumentSave { .. }))
    });
    assert!(document_save, "a document save");
    assert_eq!(commits(&root, &branch), before + 1, "one commit");
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_SAVED));
    let item = shown(&shell, &root, DOCUMENT);
    assert!(item.body.as_deref() == Some(format!("{original}More.").as_str()));
    assert!(stored_bodies(&shell).is_empty(), "the draft retired");
    let tab = TabKey::Item {
        repo: key(&root),
        kind: ItemDtoKind::Document,
        id: DOCUMENT.to_owned(),
    };
    assert_eq!(
        registry(&shell, cx).read_with(cx, |registry, _| registry.attached(&tab)),
        None
    );
}

/// R-75: a save that commits into a context this process made takes it out
/// of `CreatedContexts`: it holds work now, and is never released on leave.
#[gpui_kit::test]
async fn a_committed_save_removes_the_context_from_created_contexts(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Made here", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    assert!(created(&shell, &root, TICKET, cx));
    type_at_end(&shell, &host, "x", cx);
    assert!(created(&shell, &root, TICKET, cx), "typing is no save");
    save(&shell, cx).await;
    assert!(!created(&shell, &root, TICKET, cx));
}

/// R-70: after a committed save, and after the registry's read, the tab
/// reads the item again under a new stamp, and the read editor reloads
/// from that read.
#[gpui_kit::test]
async fn a_committed_save_reads_the_tab_again_and_reloads_the_read_editor(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Read again", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let read_view = read_editor(&shell, cx);
    let host = edit(&shell, cx).await;
    let original = edit_text(&host, cx);
    type_at_end(&shell, &host, "x", cx);

    // The workspace reads its tabs again as the save finishes; the
    // registry's read is held, so the tab's own read after it is seen.
    let read = shell.backend.hold_read(ReadKind::DraftItem);
    shell.click("item-save", cx);
    let request = saves(&shell, cx)[0];
    shell.until_ended(request, cx).await;
    shell.run(cx);
    let held_at = shell.backend.reads_in_order().len();
    read.send(()).unwrap();
    shell.run(cx);
    shell.frame(cx, |_, _| ());

    let after = shell.backend.reads_in_order()[held_at..].to_vec();
    assert!(
        after.contains(&ReadKind::Item),
        "the tab read the item again after the registry's read: {after:?}"
    );
    let saved = format!("{original}x");
    assert!(shown(&shell, &root, TICKET).body.as_deref() == Some(saved.as_str()));
    assert!(
        text(&read_view, cx) == load(&saved),
        "the read editor reloaded"
    );
}

/// R-70: a read the tab sent before the save never lands after it, though
/// it reads only once the save has committed: here it would show a later
/// title.
#[gpui_kit::test]
async fn a_read_sent_before_a_save_never_lands_after_it(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Before", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    type_at_end(&shell, &host, "x", cx);
    let early = shell.backend.hold_read(ReadKind::Item);
    pane(&shell, cx).update(cx, |pane, cx| {
        pane.reload(cx);
    });
    shell.run(cx);

    save(&shell, cx).await;
    assert_eq!(shell.label("item-title", cx).as_deref(), Some("Before"));
    shell.fixture.save_ticket(&root, TICKET, "Later");
    early.send(()).ok();
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    assert_eq!(
        shell.label("item-title", cx).as_deref(),
        Some("Before"),
        "the early read did not land"
    );
}

/// A body with what the editor keeps exactly: trailing spaces, a pipe
/// table, HTML, odd list markers, a tab, `$$` beside spaces (exempt) and
/// non-ASCII text, with CRLF line endings, which the load makes LF (N-1).
const UNTOUCHED: &str = "Intro line with trailing spaces   \r\n\r\n| a | b |\r\n|---|---|\r\n| 1 | 2 |\r\n\r\n<div class=\"x\">html</div>\r\n\r\n- item one\r\n  * nested odd\r\n\r\nword here\r\n\r\n$$5 and $$10\r\n\ttabbed\r\n\u{fc}\u{f1}\u{ed}\u{e7}\u{f8}d\u{e9} end\r\n";

/// A typed edit and the toolbar's Bold, saved over a real repository: the
/// body on disk is the loaded body, byte for byte, but for the two edits
/// (the N set: CRLF made LF); the front matter is the same by value.
#[gpui_kit::test]
async fn the_saved_file_keeps_every_untouched_body_byte(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let extra = "project: alpha\nteam: core\n";
    let (shell, root) = shell(cx, |fixture, root| {
        let source = ticket_with_body(TICKET, "Untouched", extra, UNTOUCHED);
        fixture.commit_file(root, &ticket_path(TICKET), source.as_bytes());
    });
    let before = shown(&shell, &root, TICKET);
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let editor = host_editor(&host, cx);
    let loaded = load(UNTOUCHED);
    // The N set, from the raw text: this body has no `$$` pair (`$$5 and
    // $$10` is exempt), so the load changes the line endings only (N-1).
    assert!(
        loaded == UNTOUCHED.replace("\r\n", "\n"),
        "only the line endings"
    );
    assert!(edit_text(&host, cx) == loaded);

    // Bold over "word", from the toolbar.
    let word = loaded.find("word here").unwrap();
    focus_at(&shell, &editor, word, cx);
    shell.keys("shift-right shift-right shift-right shift-right", cx);
    shell.click("toolbar-bold", cx);
    // A letter typed after "Intro".
    focus_at(&shell, &editor, "Intro".len(), cx);
    shell.visual(cx).simulate_input("x");
    shell.run(cx);
    let expected =
        loaded
            .replacen("word here", "**word** here", 1)
            .replacen("Intro line", "Introx line", 1);
    assert!(edit_text(&host, cx) == expected, "the two edits");

    save(&shell, cx).await;
    let after = shown(&shell, &root, TICKET);
    let file = std::fs::read(Path::new(&after.context.worktree).join(ticket_path(TICKET))).unwrap();
    assert!(
        file.ends_with(expected.as_bytes()),
        "the body, byte for byte"
    );
    assert!(after.body.as_deref() == Some(expected.as_str()));
    assert_eq!(after.title, before.title);
    assert_eq!(after.ticket_type, before.ticket_type);
    assert_eq!(after.status, before.status);
    assert_eq!(after.project, before.project);
    assert_eq!(after.team, before.team);
    assert!(after.unknown_metadata == before.unknown_metadata);
}

/// A ticket's front matter keys Manyhands does not define (fact 78) are
/// kept by value through a save, nested ones included.
#[gpui_kit::test]
async fn a_ticket_save_keeps_unknown_metadata_by_value(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let extra = "custom_key: hello\nlabels:\n  - b\n  - a\nnested:\n  count: 3\n  flag: true\n";
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Unknown keys", extra);
    });
    let before = shown(&shell, &root, TICKET);
    assert!(before.unknown_metadata.contains_key("custom_key"));
    assert!(before.unknown_metadata.contains_key("nested"));
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    type_at_end(&shell, &host, "x", cx);
    save(&shell, cx).await;
    let after = shown(&shell, &root, TICKET);
    assert!(
        after
            .body
            .as_deref()
            .is_some_and(|body| body.ends_with('x'))
    );
    assert!(after.unknown_metadata == before.unknown_metadata);
}

/// The bytes of the ticket file of `root`'s `id` at the tip of every
/// local branch that holds it.
fn branch_bytes(root: &Path, id: &str) -> Vec<(String, Vec<u8>)> {
    let repository = git2::Repository::open(root).unwrap();
    let path = ticket_path(id);
    let mut bytes = Vec::new();
    for branch in repository.branches(Some(git2::BranchType::Local)).unwrap() {
        let (branch, _) = branch.unwrap();
        let name = branch.name().unwrap().unwrap().to_owned();
        let tree = branch.get().peel_to_tree().unwrap();
        if let Ok(entry) = tree.get_path(Path::new(&path)) {
            let blob = repository.find_blob(entry.id()).unwrap();
            bytes.push((name, blob.content().to_vec()));
        }
    }
    bytes.sort();
    bytes
}

/// Review Focus 1: an item whose body the load normalizes, opened for
/// editing and left with Done, writes nothing. Normalization is no change:
/// the editor is clean, `secondary-s` sends nothing, and the canonical
/// bytes of every branch are the file's as committed, with no new commit.
/// The context the Edit made is released on leave: held until all that is
/// read, then let run (Task 27 fix 1, C1).
#[gpui_kit::test]
async fn a_normalized_item_opened_and_done_writes_nothing(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let source = ticket_with_body(TICKET, "Normalized", "", RAW);
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_file(root, &ticket_path(TICKET), source.as_bytes());
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    assert!(edit_text(&host, cx) == load(RAW));
    assert!(load(RAW) != RAW, "the load changed it");
    let snapshot = RepositorySnapshot::take(&root);
    let branches = branch_bytes(&root, TICKET);
    assert_eq!(branches.len(), 2, "primary's and the context's");
    assert!(
        branches
            .iter()
            .all(|(_, bytes)| *bytes == source.as_bytes()),
        "as committed"
    );
    assert!(!dirty(&host, cx));
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_SAVED));

    let held = shell.backend.hold_job(JobKind::ContextRelease);
    let editor = host_editor(&host, cx);
    focus_at(&shell, &editor, 0, cx);
    shell.keys("secondary-s", cx);
    shell.click("item-done", cx);
    shell.run(cx);
    held.wait_reached();
    assert!(saves(&shell, cx).is_empty(), "nothing is saved");
    assert!(stored_bodies(&shell).is_empty(), "nothing is drafted");
    assert_eq!(RepositorySnapshot::take(&root), snapshot, "no new commit");
    assert!(
        branch_bytes(&root, TICKET) == branches,
        "every branch's bytes"
    );

    held.release();
    let release = shell.requests(cx, |request| {
        matches!(request.intent, Intent::ContextRelease { .. })
    });
    assert_eq!(release.len(), 1, "one release");
    ended(&shell, release[0], cx).await;
    assert_eq!(code(&shell, release[0], cx), Some(ResultCode::Ok));
    let branch = format!("refs/heads/manyhands/ticket/{TICKET}");
    assert!(
        !RepositorySnapshot::take(&root)
            .refs
            .iter()
            .any(|(name, _)| *name == branch),
        "the context is released"
    );
}

/// An edit undone to the baseline is clean: neither Save nor `secondary-s`
/// sends anything, and `secondary-s` says there is nothing to save.
#[gpui_kit::test]
async fn an_edit_undone_to_the_baseline_offers_no_save_anywhere(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Undone", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let original = edit_text(&host, cx);
    type_at_end(&shell, &host, "x", cx);
    assert!(dirty(&host, cx));
    shell.keys("secondary-z", cx);
    assert!(edit_text(&host, cx) == original);
    assert!(!dirty(&host, cx));
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_SAVED));

    shell.click("item-save", cx);
    assert!(saves(&shell, cx).is_empty(), "Save sends nothing");
    let editor = host_editor(&host, cx);
    focus_at(&shell, &editor, 0, cx);
    shell.keys("secondary-s", cx);
    assert!(saves(&shell, cx).is_empty(), "secondary-s sends nothing");
    assert_eq!(
        shell.label("toolbar-status", cx).as_deref(),
        Some("No changes to save")
    );
}

/// A save whose checkpoint fails once (`FailurePoint`, a service opened to
/// fail after the write) is shown with Retry, the text kept and nothing
/// committed; Retry sends the same request again, under its own request
/// ID, and one commit follows.
#[gpui_kit::test]
async fn a_pending_checkpoint_offers_retry_and_one_commit_follows(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Retried", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let branch = branch_of(&shell, &root, TICKET);
    let before = commits(&root, &branch);
    let original = edit_text(&host, cx);
    type_at_end(&shell, &host, "x", cx);

    shell.backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            shell.fixture.data_dir(),
            FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence,
        )
        .unwrap(),
    );
    let request = save(&shell, cx).await;
    let (_, _, effects) = answer(&shell, request, cx).unwrap();
    assert_eq!(effects.checkpoint, CheckpointEffect::Pending);
    assert_eq!(
        shell.label("item-save-failed", cx).as_deref(),
        Some("Could not save a checkpoint on this computer. Your text is kept.")
    );
    assert_eq!(shell.label("item-save-retry", cx).as_deref(), Some("Retry"));
    assert_eq!(commits(&root, &branch), before, "nothing committed");
    assert!(
        edit_text(&host, cx) == format!("{original}x"),
        "the text is kept"
    );
    assert_eq!(phase(&shell, cx), Phase::Editing);
    shell.click("item-save", cx);
    assert_eq!(saves(&shell, cx), [request], "Retry, not a second save");
    let first = shell.operations(cx).read_with(cx, |operations, _| {
        operations.request(request).unwrap().request_id
    });

    shell
        .backend
        .replace_service(RepositoryService::open_at(shell.fixture.data_dir()).unwrap());
    shell.click("item-save-retry", cx);
    ended(&shell, request, cx).await;
    let again = shell.operations(cx).read_with(cx, |operations, _| {
        let request = operations.request(request).unwrap();
        (request.request_id, request.attempt)
    });
    assert_eq!(again, (first, 2), "the same request ID, a second attempt");
    assert_eq!(saves(&shell, cx), [request]);
    assert_eq!(commits(&root, &branch), before + 1, "one commit");
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_SAVED));
    assert!(!shell.present("item-save-failed", cx));
}

/// A save whose hand-off to the index is still owed is saved on this
/// computer, and a banner offers Resume, which finishes the hand-off.
#[gpui_kit::test]
async fn discovery_pending_offers_resume_in_a_banner(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Owed", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let branch = branch_of(&shell, &root, TICKET);
    let before = commits(&root, &branch);
    type_at_end(&shell, &host, "x", cx);

    shell.backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            shell.fixture.data_dir(),
            FailurePoint::BeforeIndexTransactionCommit,
        )
        .unwrap(),
    );
    let request = save(&shell, cx).await;
    assert_eq!(
        code(&shell, request, cx),
        Some(ResultCode::DiscoveryPending)
    );
    assert_eq!(
        commits(&root, &branch),
        before + 1,
        "saved on this computer"
    );
    assert_eq!(
        shell.label("item-save-pending", cx).as_deref(),
        Some("Saved on this computer, but the search index is not updated yet. Nothing is lost.")
    );
    assert_eq!(
        shell.label("item-save-resume", cx).as_deref(),
        Some("Resume")
    );
    assert!(!dirty(&host, cx));
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_SAVED));

    shell
        .backend
        .replace_service(RepositoryService::open_at(shell.fixture.data_dir()).unwrap());
    shell.click("item-save-resume", cx);
    let resumed = shell.requests(cx, |request| {
        matches!(request.intent, Intent::OperationResume { .. })
    });
    assert_eq!(resumed.len(), 1, "Resume resumes the operation");
    ended(&shell, resumed[0], cx).await;
    assert_eq!(code(&shell, resumed[0], cx), Some(ResultCode::Ok));
    assert!(
        !shell.present("item-save-pending", cx),
        "the hand-off is done"
    );
}

/// A save whose end state already holds, another actor having saved the
/// same text, answers `already_applied`: it reads as saved, and the draft
/// retires.
#[gpui_kit::test]
async fn already_applied_reads_saved_and_retires(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Applied", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    type_at_end(&shell, &host, "x", cx);
    assert!(attached(&shell, &root, TICKET, cx).is_some());

    // Another actor saves exactly this text first.
    let service = shell.fixture.service();
    let repo = service.resolve_repository(&root).unwrap();
    let item = service.show_item(&repo, &item_id(TICKET)).unwrap();
    let theirs = service.execute(
        MutationCall {
            request_id: RequestId::new(),
            confirmation: None,
            mutation: Mutation::TicketSave(TicketSaveInput {
                root: root.clone(),
                id: item_id(TICKET),
                draft: TicketDraft {
                    title: item.title.clone().unwrap(),
                    ticket_type: item.ticket_type.clone().unwrap(),
                    status: item.status.clone().unwrap(),
                    project: item.project.clone(),
                    team: item.team.clone(),
                    body: edit_text(&host, cx),
                },
                deps: RelationshipWrite::Unchanged,
                parent: RelationshipWrite::Unchanged,
                observation: item.observation.clone().unwrap(),
            }),
        },
        &mut SessionCredentials::new(NoPassphrase),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(theirs.code, ResultCode::Ok, "{}", theirs.message);

    let request = save(&shell, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::AlreadyApplied));
    assert!(!dirty(&host, cx));
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_SAVED));
    assert_eq!(
        attached(&shell, &root, TICKET, cx),
        None,
        "the draft retired"
    );
    assert!(stored_bodies(&shell).is_empty());
}

/// `secondary-s` saves from the edit editor, and `secondary-shift-m`
/// switches the mode there.
#[gpui_kit::test]
async fn secondary_s_saves_and_secondary_shift_m_switches_mode(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Keyed", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let branch = branch_of(&shell, &root, TICKET);
    let before = commits(&root, &branch);
    type_at_end(&shell, &host, "x", cx);
    shell.keys("secondary-s", cx);
    let sent = saves(&shell, cx);
    assert_eq!(sent.len(), 1, "secondary-s saved");
    ended(&shell, sent[0], cx).await;
    assert_eq!(commits(&root, &branch), before + 1);
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_SAVED));

    let editor = host_editor(&host, cx);
    focus_at(&shell, &editor, 0, cx);
    assert!(host.read_with(cx, |host, _| host.is_rich()));
    shell.keys("secondary-shift-m", cx);
    assert!(!host.read_with(cx, |host, _| host.is_rich()), "Markdown");
    shell.keys("secondary-shift-m", cx);
    assert!(host.read_with(cx, |host, _| host.is_rich()), "Rich text");
}

/// `secondary-s` on a clean editor saves nothing and says "No changes to
/// save" in the toolbar's status item, until the text changes.
#[gpui_kit::test]
async fn secondary_s_on_a_clean_editor_says_no_changes_to_save(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Clean", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let editor = host_editor(&host, cx);
    focus_at(&shell, &editor, 0, cx);
    assert_eq!(shell.label("toolbar-status", cx), None);
    shell.keys("secondary-s", cx);
    assert!(saves(&shell, cx).is_empty());
    assert_eq!(
        shell.label("toolbar-status", cx).as_deref(),
        Some("No changes to save")
    );
    type_at_end(&shell, &host, "x", cx);
    assert_ne!(
        shell.label("toolbar-status", cx).as_deref(),
        Some("No changes to save"),
        "the text changed"
    );
}

/// Fact 41: whether a publication remote is selected is read after a save
/// completes, never as the tab loads or the editor opens. With one
/// selected, a save reads "not shared yet".
#[gpui_kit::test]
async fn publication_pending_is_read_after_a_save_not_on_load(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Shared later", "");
        fixture.publish_over_ssh(root);
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    assert_eq!(
        shell.backend.reads_seen(ReadKind::Remotes),
        0,
        "not on load"
    );
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_SAVED));
    type_at_end(&shell, &host, "x", cx);
    assert_eq!(shell.backend.reads_seen(ReadKind::Remotes), 0);

    save(&shell, cx).await;
    assert_eq!(
        shell.backend.reads_seen(ReadKind::Remotes),
        1,
        "after the save"
    );
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_NOT_SHARED));
}

/// Phases, step 6: a save that retired the draft ended D2's session
/// (fact 79); the next change begins a draft again, on the base the save
/// left, so crash recovery goes on after a save.
#[gpui_kit::test]
async fn a_change_after_a_save_retired_the_draft_begins_one_on_the_base_it_left(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Begun again", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let original = edit_text(&host, cx);
    type_at_end(&shell, &host, "x", cx);
    let first = attached(&shell, &root, TICKET, cx).expect("a draft began");
    save(&shell, cx).await;
    assert_eq!(attached(&shell, &root, TICKET, cx), None, "it retired");

    type_at_end(&shell, &host, "y", cx);
    let id = attached(&shell, &root, TICKET, cx).expect("a draft began again");
    assert_ne!(id, first, "a new draft");
    let saved = shown(&shell, &root, TICKET);
    let draft = stored(&shell.fixture.draft_store(), id).expect("stored");
    assert!(body_of(&draft.current) == format!("{original}xy"));
    let base = draft.base.expect("a base");
    assert_eq!(
        base.observation, saved.observation,
        "the base the save left"
    );
    assert!(body_of(&base.fields) == format!("{original}x"));
}

/// Save states: a save from a base someone else has moved answers
/// `external_change`. The review opens, under the change banner; nothing
/// is written, and Save is disabled while the review is open.
#[gpui_kit::test]
async fn an_external_change_opens_the_review_under_the_change_banner(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Mine", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let branch = branch_of(&shell, &root, TICKET);
    let original = edit_text(&host, cx);
    type_at_end(&shell, &host, "x", cx);
    shell.fixture.save_ticket(&root, TICKET, "Theirs");
    let before = commits(&root, &branch);

    let request = save(&shell, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::ExternalChange));
    assert_eq!(phase(&shell, cx), Phase::Review, "the review opens");
    assert_eq!(pill(&shell, cx).as_deref(), Some(REVIEWING));
    assert_eq!(
        shell.label("item-save-changed", cx).as_deref(),
        Some("Someone changed this file outside Manyhands. Your edits are kept.")
    );
    assert_eq!(commits(&root, &branch), before, "nothing is written");
    let item = shown(&shell, &root, TICKET);
    assert_eq!(item.title.as_deref(), Some("Theirs"));
    assert!(item.body.as_deref() == Some(original.as_str()));
    assert!(
        edit_text(&host, cx) == format!("{original}x"),
        "mine is kept"
    );
    shell.click("item-save", cx);
    assert_eq!(
        saves(&shell, cx),
        [request],
        "Save is disabled in the review"
    );
}

/// With crash recovery off there is no draft, and Save still saves: the
/// fields are sent as they are, and the next save goes on from the base
/// the first one left, with no review.
#[cfg(unix)]
#[gpui_kit::test]
async fn with_crash_recovery_off_save_commits_without_a_draft(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let elsewhere = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(elsewhere.path(), fixture.data_dir().join("drafts")).unwrap();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Unprotected", "");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    open(&shell, ticket(&root, TICKET), cx);
    shell.click("item-edit", cx);
    shell.click("recovery-off-continue", cx);
    let host = until_editing(&shell, cx).await;
    let branch = branch_of(&shell, &root, TICKET);
    let before = commits(&root, &branch);
    let original = edit_text(&host, cx);

    type_at_end(&shell, &host, "x", cx);
    assert_eq!(attached(&shell, &root, TICKET, cx), None, "no draft");
    save(&shell, cx).await;
    assert_eq!(commits(&root, &branch), before + 1);
    assert!(!dirty(&host, cx));
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_SAVED));

    type_at_end(&shell, &host, "y", cx);
    save(&shell, cx).await;
    assert_eq!(commits(&root, &branch), before + 2);
    assert_eq!(phase(&shell, cx), Phase::Editing, "no review");
    let item = shown(&shell, &root, TICKET);
    assert!(item.body.as_deref() == Some(format!("{original}xy").as_str()));
}

// Fix round 1: ruling 2 (Save after a failed draft write), and the
// review's Minors 3 and 4.

/// Edit; "x" typed and written to a draft; then the drafts folder made
/// read-only and "y" typed, so the next write fails and the toolbar says
/// so. Gives the host, the text before the edits, the draft and the hold
/// on the folder's mode.
#[cfg(unix)]
async fn failed_write(
    shell: &Shell,
    root: &Path,
    cx: &mut TestAppContext,
) -> (Entity<EditorHost>, String, DraftId, FolderMode) {
    open(shell, ticket(root, TICKET), cx);
    let host = edit(shell, cx).await;
    let original = edit_text(&host, cx);
    type_at_end(shell, &host, "x", cx);
    let id = attached(shell, root, TICKET, cx).expect("a draft began");
    assert!(stored_body(&shell.fixture.draft_store(), id) == Some(format!("{original}x")));
    let read_only = FolderMode::set(&shell.fixture.data_dir().join("drafts"), 0o500);
    type_at_end(shell, &host, "y", cx);
    cx.executor().advance_clock(IDLE);
    shell.run(cx);
    assert_eq!(
        shell.label("toolbar-status", cx).as_deref(),
        Some("Draft not kept. Save to protect your changes")
    );
    (host, original, id, read_only)
}

/// Ruling 2: the draft's last write failed, and the toolbar says "Draft
/// not kept. Save to protect your changes". Save commits all the same,
/// from the text in memory, once. The session then retires, though that
/// revision was never written: the commit holds it. The folder stays
/// read-only, so the older revision cannot be discarded: it stays
/// stored, and nothing is lost.
#[cfg(unix)]
#[gpui_kit::test]
async fn a_save_after_a_failed_draft_write_commits_and_reads_saved(cx: &mut TestAppContext) {
    if superuser() {
        return;
    }
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Unkept", "");
    });
    let (host, original, id, _read_only) = failed_write(&shell, &root, cx).await;
    let branch = branch_of(&shell, &root, TICKET);
    let before = commits(&root, &branch);

    let request = save(&shell, cx).await;
    assert_eq!(saves(&shell, cx), [request], "one save");
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    assert_eq!(commits(&root, &branch), before + 1, "one commit");
    let item = shown(&shell, &root, TICKET);
    assert!(item.body.as_deref() == Some(format!("{original}xy").as_str()));
    assert!(!dirty(&host, cx));
    assert_eq!(
        pill(&shell, cx).as_deref(),
        Some("Editing \u{b7} saved on this computer")
    );
    assert_ne!(
        shell.label("toolbar-status", cx).as_deref(),
        Some("Draft not kept. Save to protect your changes")
    );
    assert_eq!(
        attached(&shell, &root, TICKET, cx),
        None,
        "the session retired"
    );
    assert!(
        stored_body(&shell.fixture.draft_store(), id) == Some(format!("{original}x")),
        "the older revision stays stored"
    );
}

/// Ruling 2, with the older revision stored first and the folder writable
/// again before the save settles: the older revision is discarded, and no
/// draft of the item is listed.
#[cfg(unix)]
#[gpui_kit::test]
async fn a_save_after_a_failed_draft_write_discards_the_older_revision(cx: &mut TestAppContext) {
    if superuser() {
        return;
    }
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Discarded", "");
    });
    let (host, _, id, read_only) = failed_write(&shell, &root, cx).await;

    let held = shell.backend.hold_job(JobKind::TicketSave);
    shell.click("item-save", cx);
    held.wait_reached();
    drop(read_only);
    held.release();
    let request = saves(&shell, cx)[0];
    ended(&shell, request, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    assert!(!dirty(&host, cx));
    assert_eq!(attached(&shell, &root, TICKET, cx), None);
    let store = shell.fixture.draft_store();
    assert!(
        stored(&store, id).is_none(),
        "the older revision is discarded"
    );
    let listed = store.list().unwrap().into_iter().any(|listing| {
        matches!(&listing.entry, Entry::Draft(draft) if draft.target.item_id == item_id(TICKET))
    });
    assert!(!listed, "no draft of the item is listed");
}

/// Ruling 2, with typing while the save runs: the newer text stays dirty,
/// in a live session, rebased onto the base the save left.
#[cfg(unix)]
#[gpui_kit::test]
async fn typing_while_a_save_after_a_failed_write_runs_stays_dirty_and_live(
    cx: &mut TestAppContext,
) {
    if superuser() {
        return;
    }
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Newer", "");
    });
    let (host, original, _, _read_only) = failed_write(&shell, &root, cx).await;

    let held = shell.backend.hold_job(JobKind::TicketSave);
    shell.click("item-save", cx);
    held.wait_reached();
    shell.run(cx);
    type_at_end(&shell, &host, "z", cx);
    held.release();
    let request = saves(&shell, cx)[0];
    ended(&shell, request, cx).await;

    let item = shown(&shell, &root, TICKET);
    assert!(item.body.as_deref() == Some(format!("{original}xy").as_str()));
    assert!(edit_text(&host, cx) == format!("{original}xyz"));
    assert!(dirty(&host, cx), "the newer text stays dirty");
    assert_eq!(
        pill(&shell, cx).as_deref(),
        Some("Editing \u{b7} unsaved changes")
    );
    let id = attached(&shell, &root, TICKET, cx).expect("a live session");
    let (current, observation) = registry(&shell, cx).read_with(cx, |registry, _| {
        let session = registry.session(id).expect("live");
        (
            body_of(session.current()).to_owned(),
            session.base().and_then(|base| base.observation.clone()),
        )
    });
    assert!(current == format!("{original}xyz"));
    assert_eq!(
        observation, item.observation,
        "rebased onto the save's base"
    );
}

/// Ruling 2, against a store that keeps failing (every write, through the
/// store's own failure point): Save sends exactly one save, which
/// commits, and no write loop follows it.
#[gpui_kit::test]
async fn a_save_against_a_store_that_keeps_failing_is_sent_once(cx: &mut TestAppContext) {
    use manyhands::drafts::{DraftErrorKind, FailureAction, FailurePoint as DraftFailurePoint};

    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Failing", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let branch = branch_of(&shell, &root, TICKET);
    let before = commits(&root, &branch);
    type_at_end(&shell, &host, "x", cx);
    assert!(
        attached(&shell, &root, TICKET, cx).is_some(),
        "a draft began"
    );
    // Clones share the failure point: the desktop's store fails too.
    let _ = shell.fixture.draft_store().with_failure_point_for_testing(
        DraftFailurePoint::CreateTemp,
        FailureAction::Fail(DraftErrorKind::NoSpace),
    );
    type_at_end(&shell, &host, "y", cx);
    cx.executor().advance_clock(IDLE);
    shell.run(cx);
    assert_eq!(
        shell.label("toolbar-status", cx).as_deref(),
        Some("Draft not kept. Save to protect your changes")
    );
    let writes = shell.backend.reads_seen(ReadKind::DraftWrite);

    let request = save(&shell, cx).await;
    assert_eq!(saves(&shell, cx), [request], "exactly one save");
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    assert_eq!(commits(&root, &branch), before + 1);
    let more = shell.backend.reads_seen(ReadKind::DraftWrite) - writes;
    assert!(more <= 2, "one write before the save, one after: {more}");
    assert_eq!(
        pill(&shell, cx).as_deref(),
        Some("Editing \u{b7} saved on this computer")
    );
}

/// Minor 3: while D2 finishes the last save, retiring its revision or
/// storing newer text again, the tab is still saving: the pill says so,
/// and Save is disabled. Ctrl/Cmd-S meanwhile is not dropped: it waits,
/// and saves once D2 is done.
#[gpui_kit::test]
async fn save_waits_while_d2_finishes_the_last_save(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Finishing", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let branch = branch_of(&shell, &root, TICKET);
    let before = commits(&root, &branch);
    let original = edit_text(&host, cx);
    type_at_end(&shell, &host, "x", cx);

    // The retirement is the next store call that writes.
    let retirement = shell.backend.hold_read(ReadKind::DraftWrite);
    save(&shell, cx).await;
    assert_eq!(commits(&root, &branch), before + 1);
    type_at_end(&shell, &host, "y", cx);
    assert_eq!(
        pill(&shell, cx).as_deref(),
        Some(SAVING),
        "D2 is still saving"
    );
    shell.click("item-save", cx);
    assert_eq!(saves(&shell, cx).len(), 1, "Save is disabled meanwhile");
    let editor = host_editor(&host, cx);
    let end = text(&editor, cx).len();
    focus_at(&shell, &editor, end, cx);
    shell.keys("secondary-s", cx);
    assert_eq!(saves(&shell, cx).len(), 1, "the key waits");

    retirement.send(()).unwrap();
    shell.run(cx);
    let sent = saves(&shell, cx);
    assert_eq!(sent.len(), 2, "then it saves");
    ended(&shell, sent[0], cx).await;
    assert_eq!(commits(&root, &branch), before + 2);
    let item = shown(&shell, &root, TICKET);
    assert!(item.body.as_deref() == Some(format!("{original}xy").as_str()));
    assert_eq!(
        pill(&shell, cx).as_deref(),
        Some("Editing \u{b7} saved on this computer")
    );
}

/// Minor 4: a pending checkpoint whose operation is abandoned, as the
/// operation area does it. D2 ends that save, and the tab follows: the
/// banner goes, and Save is enabled again while the editor is dirty, with
/// no Retry bound to fail.
#[gpui_kit::test]
async fn an_abandoned_pending_checkpoint_clears_its_banner_and_frees_save(cx: &mut TestAppContext) {
    use manyhands::repository::{
        ConfirmationId, OperationAbandonInput, OperationId, PrepareDataDto,
    };

    use crate::desktop::services::{Backend, requests::Lane};

    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Abandoned", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    type_at_end(&shell, &host, "x", cx);
    shell.backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            shell.fixture.data_dir(),
            FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence,
        )
        .unwrap(),
    );
    let request = save(&shell, cx).await;
    assert_eq!(
        shell.label("item-save-failed", cx).as_deref(),
        Some("Could not save a checkpoint on this computer. Your text is kept.")
    );
    shell.click("item-save", cx);
    assert_eq!(saves(&shell, cx), [request], "Save waits for Retry");
    shell
        .backend
        .replace_service(RepositoryService::open_at(shell.fixture.data_dir()).unwrap());

    // The operation area's Abandon: `operation abandon` of the operation
    // the pending save named, confirmed.
    let operation = shell
        .operations(cx)
        .read_with(cx, |operations, _| {
            match &operations.request(request).unwrap().state {
                RequestState::Finished(_, _, _, recovery, _) => recovery
                    .iter()
                    .find_map(|action| action.operation_id.clone()),
                _ => None,
            }
        })
        .expect("a pending save names its operation");
    let operation_id = OperationId::parse(&operation).unwrap();
    let prepared =
        shell
            .backend
            .service()
            .prepare(&Mutation::OperationAbandon(OperationAbandonInput {
                root: root.clone(),
                operation_id,
            }));
    let Some(PrepareDataDto::Preview(preview)) = prepared.data else {
        panic!("operation abandon gave no preview: {}", prepared.message);
    };
    let confirmation = ConfirmationId::parse(&preview.confirmation_id).unwrap();
    let services = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().clone());
    let abandon = services.worker().submit(
        Intent::OperationAbandon {
            root: root.clone(),
            operation_id,
        },
        Lane::User,
        Some(confirmation),
        None,
    );
    ended(&shell, abandon, cx).await;
    assert_eq!(code(&shell, abandon, cx), Some(ResultCode::Ok));

    assert!(!shell.present("item-save-failed", cx), "the banner goes");
    assert!(!shell.present("item-save-retry", cx));
    assert_eq!(phase(&shell, cx), Phase::Editing);
    assert!(dirty(&host, cx));
    shell.click("item-save", cx);
    let sent = saves(&shell, cx);
    assert_eq!(sent.len(), 2, "Save is enabled again");
    ended(&shell, sent[0], cx).await;
}
