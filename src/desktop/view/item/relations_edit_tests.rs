//! A ticket's dependencies and parent in the details sheet, over real
//! repositories (design, Relationships (contract 10); R-26, R-54, R-72,
//! R-74): Waiting on's chips and Add…, Parent's Change… and Clear, the
//! picker over the tab's repository and its exact short codes, the check
//! of every change and its words under the field it answered, the single
//! save gate it joins, an unresolved ticket kept, an index that is behind,
//! a late check dropped, the ID order of chips, draft and save, a clean
//! session after add and remove, the library's own cycle answer, and the
//! read-only Dependents and Children.
//!
//! The helpers are `edit_tests`'s. Each test opens the sheet and draws its
//! frame before typing, and types through the window; each waits on the
//! worker and allows parking. Draft fields would print text through
//! `Debug`, so they are compared with `assert!`.

use gpui_kit::{ElementId, component::WindowExt};
use manyhands::repository::{DependencyState, REGISTRY_FILE, RelationshipWrite};

use super::*;
use crate::desktop::{
    model::{
        edit::save_state::NO_CHANGES_TO_SAVE,
        relations_edit::{NO_MATCH, NOT_FOUND_KEPT, Proposed, SAVE_CHECKS_AGAIN, SAVE_CYCLE},
        relationships::NONE,
    },
    test_support::drafts::{Actor, second_actor},
    view::tickets::LIFECYCLE_OPEN,
};

/// A ticket the index never holds.
const MISSING: &str = "01ARZ3NDEKTSV4RRFFQ69G5FGZ";
/// Another the index never holds, as a parent.
const MISSING_PARENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FGY";

/// `extra` front matter giving a short code and `deps`.
fn with_deps(slug: &str, deps: &[&str]) -> String {
    let mut extra = format!("slug: {slug}\n");
    if !deps.is_empty() {
        extra.push_str("deps:\n");
        for dep in deps {
            extra.push_str(&format!("  - {dep}\n"));
        }
    }
    extra
}

/// Whether a details sheet is open in the window.
fn has_sheet(shell: &Shell, cx: &mut TestAppContext) -> bool {
    shell.frame(cx, |window, cx| window.has_active_sheet(cx))
}

/// Opens the sheet from All details, then draws its frame.
fn open_sheet(shell: &Shell, cx: &mut TestAppContext) {
    shell.click(ChipField::All.id(), cx);
    assert!(has_sheet(shell, cx), "the sheet opens");
    shell.frame(cx, |_, _| ());
}

/// Types `typed` where focus is, through the window.
fn type_in(shell: &Shell, typed: &str, cx: &mut TestAppContext) {
    shell.visual(cx).simulate_input(typed);
    shell.run(cx);
    shell.frame(cx, |_, _| ());
}

/// Whether the element `id` holds focus.
fn focused(shell: &Shell, id: impl Into<ElementId>, cx: &mut TestAppContext) -> bool {
    let id = id.into();
    shell.frame(cx, |window, _| {
        window
            .try_find(id)
            .is_some_and(|element| element.focused() == Some(true))
    })
}

/// Whether the element `id` is drawn inside `within`.
fn inside(
    shell: &Shell,
    id: impl Into<ElementId>,
    within: &'static str,
    cx: &mut TestAppContext,
) -> bool {
    let id = id.into();
    shell.frame(cx, |window, _| {
        window
            .try_find(id)
            .is_some_and(|element| element.path().contains(&ElementId::from(within)))
    })
}

/// Whether the picker's filter box holds focus.
fn filter_focused(shell: &Shell, cx: &mut TestAppContext) -> bool {
    let Some(input) = pane(shell, cx).read_with(cx, |pane, _| pane.picker_filter()) else {
        return false;
    };
    let handle = input.read_with(cx, |input, cx| input.focus_handle(cx));
    shell.frame(cx, move |window, _| handle.is_focused(window))
}

/// Opens the picker from `opener` and draws the frame its filter is
/// focused in.
fn open_picker(shell: &Shell, opener: &'static str, cx: &mut TestAppContext) {
    shell.click(opener, cx);
    shell.frame(cx, |_, _| ());
    assert!(shell.present("rel-picker", cx), "{opener} opens the picker");
    assert!(filter_focused(shell, cx), "its filter takes focus");
}

/// Opens the picker from `opener`, filters it with `filter`, and takes its
/// first offer.
fn pick(shell: &Shell, opener: &'static str, filter: &str, cx: &mut TestAppContext) {
    open_picker(shell, opener, cx);
    type_in(shell, filter, cx);
    assert!(
        shell.present(("rel-pick", 0usize), cx),
        "{filter} is offered"
    );
    shell.click(("rel-pick", 0usize), cx);
    shell.frame(cx, |_, _| ());
    assert!(!shell.present("rel-picker", cx), "choosing closes it");
}

/// The shown session's fields besides the body.
fn meta(shell: &Shell, cx: &mut TestAppContext) -> DraftFields {
    pane(shell, cx).read_with(cx, |pane, _| {
        pane.session_meta().cloned().expect("a session is shown")
    })
}

/// What the shown session proposes.
fn proposed(shell: &Shell, cx: &mut TestAppContext) -> Proposed {
    Proposed::of(&meta(shell, cx)).expect("a ticket's")
}

fn dirty(shell: &Shell, cx: &mut TestAppContext) -> bool {
    pane(shell, cx).read_with(cx, |pane, cx| pane.edit_dirty(cx))
}

/// Why every save waits, as the one gate says.
fn blocked(shell: &Shell, cx: &mut TestAppContext) -> Option<&'static str> {
    pane(shell, cx).read_with(cx, |pane, cx| pane.save_blocked(cx))
}

/// The save requests of this session, newest first.
fn saves(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| {
        matches!(request.intent, Intent::TicketSave { .. })
    })
}

/// Presses the header's Save, then waits until the save it sent has ended.
async fn save(shell: &Shell, cx: &mut TestAppContext) -> DesktopRequestId {
    let before = saves(shell, cx).len();
    shell.click("item-save", cx);
    let sent = saves(shell, cx);
    assert_eq!(sent.len(), before + 1, "Save sent one save");
    ended(shell, sent[0], cx).await;
    sent[0]
}

/// The `deps` and `parent` writes the save `id` sent.
fn writes(
    shell: &Shell,
    id: DesktopRequestId,
    cx: &mut TestAppContext,
) -> (RelationshipWrite<Vec<String>>, RelationshipWrite<String>) {
    shell
        .operations(cx)
        .read_with(cx, |operations, _| match &operations.request(id)?.intent {
            Intent::TicketSave { deps, parent, .. } => Some((deps.clone(), parent.clone())),
            _ => None,
        })
        .expect("a ticket save")
}

/// The IDs `item` depends on, in its file's order.
fn dep_ids(item: &ItemDto) -> Vec<String> {
    item.deps.iter().map(|dep| dep.id.clone()).collect()
}

/// Marks `root`'s index for a refresh, as the library does once it notices
/// a change, and as `tests/mutation_relationships.rs` does to make an index
/// that is behind: its reads then say `stale`.
fn mark_behind(shell: &Shell) {
    let index = rusqlite::Connection::open(shell.fixture.data_dir().join(REGISTRY_FILE)).unwrap();
    index
        .execute("UPDATE repositories SET refresh_required = 1", [])
        .unwrap();
}

/// Puts `typed` in the Title control in place of what it held, from the
/// sheet, and closes it.
fn retitle(shell: &Shell, typed: &str, cx: &mut TestAppContext) {
    open_sheet(shell, cx);
    shell.keys("secondary-a backspace", cx);
    type_in(shell, typed, cx);
    shell.keys("escape", cx);
}

/// Draws frames, with the worker and the reads running, until the review
/// shows its choices.
async fn until_review(shell: &Shell, cx: &mut TestAppContext) {
    let deadline = Instant::now() + WAIT;
    loop {
        shell.frame(cx, |_, _| ());
        shell.run(cx);
        if shell.present("review-use", cx) {
            return;
        }
        assert!(Instant::now() < deadline, "the review did not open");
        cx.executor().timer(Duration::from_millis(10)).await;
    }
}

/// Each relationship control is gone: the sheet shows no chip's Remove, no
/// Add…, Change… or Clear, and no picker.
fn no_controls(shell: &Shell, cx: &mut TestAppContext) -> bool {
    [
        "rel-deps-add",
        "rel-parent-change",
        "rel-parent-clear",
        "rel-picker",
    ]
    .into_iter()
    .all(|id| !shell.present(id, cx))
        && !shell.present(("rel-dep-remove", 0usize), cx)
}

/// Add… opens a picker over the tab's repository's tickets, but the ticket
/// itself and those it waits on; a title filters it, ignoring case, and a
/// short code matches only exactly, through the library. A ticket is
/// chosen by click, or by keyboard, where Enter takes the first offer.
/// Escape closes the picker, not the sheet. The chips are in ID order, the
/// draft holds them, and Save writes them.
#[gpui_kit::test]
async fn a_dependency_is_chosen_from_the_list_and_by_short_code(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Main", &with_deps("mh-00001", &[DEP_A]));
    fixture.commit_ticket(&root, OTHER, "Login page", &with_deps("mh-00002", &[]));
    fixture.commit_ticket(&root, THIRD, "Logout page", &with_deps("mh-00003", &[]));
    fixture.commit_ticket(&root, DEP_A, "Already", &with_deps("mh-00004", &[]));
    fixture.commit_ticket(&root, DEP_B, "Search box", &with_deps("mh-search", &[]));
    // Another repository's ticket, listed by the workspace.
    let elsewhere = fixture.create_repository("b");
    fixture.commit_ticket(&elsewhere, DOCUMENT, "Login elsewhere", "");
    fixture.refresh(&root);
    fixture.refresh(&elsewhere);
    let shell = Shell::open(fixture, cx);
    shell.select(&elsewhere, cx);
    open(&shell, ticket(&root, TICKET), cx);
    edit(&shell, cx).await;
    open_sheet(&shell, cx);
    assert_eq!(shell.rows("rel-dep-id", cx), [DEP_A]);

    open_picker(&shell, "rel-deps-add", cx);
    let mut listed = shell.rows("rel-pick", cx);
    listed.sort();
    assert_eq!(
        listed,
        [
            "mh-00002 Login page",
            "mh-00003 Logout page",
            "mh-search Search box"
        ],
        "the tab's repository's tickets, but itself and its dependency"
    );
    type_in(&shell, "LOG", cx);
    let mut narrowed = shell.rows("rel-pick", cx);
    narrowed.sort();
    assert_eq!(narrowed, ["mh-00002 Login page", "mh-00003 Logout page"]);
    type_in(&shell, "in", cx);
    assert_eq!(shell.rows("rel-pick", cx), ["mh-00002 Login page"]);
    shell.click(("rel-pick", 0usize), cx);
    assert!(!shell.present("rel-picker", cx));
    assert!(has_sheet(&shell, cx), "the sheet stays open");
    assert_eq!(shell.rows("rel-dep-id", cx), [OTHER, DEP_A], "ID order");
    assert_eq!(
        shell.rows("rel-dep-name", cx),
        ["mh-00002 Login page", "mh-00004 Already"]
    );
    assert!(proposed(&shell, cx).deps == [OTHER, DEP_A]);
    assert!(dirty(&shell, cx));
    assert!(focused(&shell, "rel-deps-add", cx), "focus back on Add…");

    // By keyboard, by an exact short code in another case.
    let slug_reads = shell.backend.reads_seen(ReadKind::FindSlug);
    shell.press("enter", cx);
    shell.frame(cx, |_, _| ());
    assert!(shell.present("rel-picker", cx), "Enter opens it");
    assert!(filter_focused(&shell, cx));
    type_in(&shell, "mh-sea", cx);
    assert!(
        !shell.present(("rel-pick", 0usize), cx),
        "a short code matches only whole"
    );
    assert_eq!(
        shell.label("rel-picker-none", cx).as_deref(),
        Some(NO_MATCH)
    );
    type_in(&shell, "rch", cx);
    shell.keys("secondary-a", cx);
    type_in(&shell, "MH-SEARCH", cx);
    assert_eq!(shell.rows("rel-pick", cx), ["mh-search Search box"]);
    assert!(shell.backend.reads_seen(ReadKind::FindSlug) > slug_reads);
    shell.keys("enter", cx);
    shell.frame(cx, |_, _| ());
    assert!(!shell.present("rel-picker", cx), "Enter takes the first");
    assert_eq!(shell.rows("rel-dep-id", cx), [OTHER, DEP_A, DEP_B]);

    // Escape closes the picker and gives focus back; the sheet stays.
    open_picker(&shell, "rel-deps-add", cx);
    shell.keys("escape", cx);
    assert!(!shell.present("rel-picker", cx));
    assert!(has_sheet(&shell, cx), "Escape leaves the sheet open");
    assert!(focused(&shell, "rel-deps-add", cx));
    assert_eq!(shell.rows("rel-dep-id", cx), [OTHER, DEP_A, DEP_B]);
    shell.keys("escape", cx);
    assert!(!has_sheet(&shell, cx), "a second Escape closes the sheet");

    // Closed with the sheet, by a route outside the picker: the sheet opens
    // again without it, and a picker opened again has an empty filter.
    open_sheet(&shell, cx);
    open_picker(&shell, "rel-deps-add", cx);
    type_in(&shell, "Search", cx);
    shell.keys("shift-tab", cx);
    assert!(!filter_focused(&shell, cx), "focus left the picker");
    shell.keys("escape", cx);
    assert!(
        !has_sheet(&shell, cx),
        "Escape outside the picker closes the sheet"
    );
    open_sheet(&shell, cx);
    assert!(
        !shell.present("rel-picker", cx),
        "the picker went with the sheet"
    );
    open_picker(&shell, "rel-deps-add", cx);
    let filter = pane(&shell, cx)
        .read_with(cx, |pane, _| pane.picker_filter())
        .expect("the picker is open");
    assert_eq!(
        filter.read_with(cx, |input, _| input.value().to_string()),
        ""
    );
    shell.keys("escape escape", cx);
    assert!(!has_sheet(&shell, cx));

    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");
    let drafted = registry(&shell, cx).read_with(cx, |registry, _| {
        registry.session(id).unwrap().current().clone()
    });
    assert!(Proposed::of(&drafted).unwrap().deps == [OTHER, DEP_A, DEP_B]);
    let request = save(&shell, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    assert!(
        writes(&shell, request, cx).0
            == RelationshipWrite::Set(vec![OTHER.to_owned(), DEP_A.to_owned(), DEP_B.to_owned()])
    );
    assert_eq!(
        dep_ids(&shown(&shell, &root, TICKET)),
        [OTHER, DEP_A, DEP_B]
    );

    // The item's own reads name a ticket before the picker's did: renamed
    // and read again, its chip says the new title (fix 1, C).
    shell
        .fixture
        .commit_ticket(&root, OTHER, "Sign-in page", &with_deps("mh-00002", &[]));
    shell.fixture.refresh(&root);
    pane(&shell, cx).update(cx, |pane, cx| {
        pane.reload(cx);
    });
    shell.run(cx);
    open_sheet(&shell, cx);
    assert_eq!(
        shell.rows("rel-dep-name", cx),
        [
            "mh-00002 Sign-in page",
            "mh-00004 Already",
            "mh-search Search box"
        ]
    );
}

/// Parent shows its short code and title; Change… opens the picker, which
/// leaves out the ticket and its parent, and the one chosen is saved as
/// set; Clear empties it, saved as cleared.
#[gpui_kit::test]
async fn a_parent_is_changed_and_cleared(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, OTHER, "Old parent", &with_deps("mh-00011", &[]));
        fixture.commit_ticket(root, THIRD, "New parent", &with_deps("mh-00012", &[]));
        fixture.commit_ticket(
            root,
            TICKET,
            "Main",
            &format!("{}parent: {OTHER}\n", with_deps("mh-00001", &[])),
        );
    });
    open(&shell, ticket(&root, TICKET), cx);
    edit(&shell, cx).await;
    open_sheet(&shell, cx);
    assert_eq!(
        shell.label("rel-parent-name", cx).as_deref(),
        Some("mh-00011 Old parent")
    );
    assert_eq!(shell.label("rel-parent-id", cx).as_deref(), Some(OTHER));
    assert!(inside(&shell, "rel-parent-chip", "sheet-parent", cx));

    open_picker(&shell, "rel-parent-change", cx);
    assert_eq!(
        shell.rows("rel-pick", cx),
        ["mh-00012 New parent"],
        "neither itself nor its parent"
    );
    shell.click(("rel-pick", 0usize), cx);
    assert_eq!(shell.label("rel-parent-id", cx).as_deref(), Some(THIRD));
    assert_eq!(
        shell.label("rel-parent-name", cx).as_deref(),
        Some("mh-00012 New parent")
    );
    assert!(proposed(&shell, cx).parent.as_deref() == Some(THIRD));
    assert!(dirty(&shell, cx));
    assert!(focused(&shell, "rel-parent-change", cx));
    shell.keys("escape", cx);
    let request = save(&shell, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    let (deps, parent) = writes(&shell, request, cx);
    assert!(deps == RelationshipWrite::Unchanged);
    assert!(parent == RelationshipWrite::Set(THIRD.to_owned()));
    let item = shown(&shell, &root, TICKET);
    assert_eq!(item.parent.map(|parent| parent.id).as_deref(), Some(THIRD));

    open_sheet(&shell, cx);
    shell.click("rel-parent-clear", cx);
    assert_eq!(shell.label("rel-parent-none", cx).as_deref(), Some(NONE));
    assert!(!shell.present("rel-parent-clear", cx), "nothing to clear");
    assert!(proposed(&shell, cx).parent.is_none());
    shell.keys("escape", cx);
    let request = save(&shell, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    assert!(writes(&shell, request, cx).1 == RelationshipWrite::Clear);
    assert!(shown(&shell, &root, TICKET).parent.is_none());
}

/// A dependency that closes a cycle is explained under Waiting on, naming
/// its tickets by short code and title, and Save is unavailable on every
/// route until it is removed. (A ticket naming itself cannot be proposed
/// here: the picker leaves it out, and the library reads a file naming
/// itself without it; its words are the model's test.)
#[gpui_kit::test]
async fn a_proposed_dependency_cycle_is_explained_with_its_tickets_and_save_is_unavailable(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Main", &with_deps("mh-00001", &[]));
        fixture.commit_ticket(root, OTHER, "Blocker", &with_deps("mh-00021", &[TICKET]));
    });
    open(&shell, ticket(&root, TICKET), cx);
    edit(&shell, cx).await;
    open_sheet(&shell, cx);
    pick(&shell, "rel-deps-add", "Blocker", cx);
    assert_eq!(
        shell.label("rel-deps-problem", cx).as_deref(),
        Some("This makes a cycle: mh-00001 Main, mh-00021 Blocker.")
    );
    assert!(inside(&shell, "rel-deps-problem", "sheet-waiting-on", cx));
    assert!(!shell.present("rel-parent-problem", cx));
    assert_eq!(blocked(&shell, cx), Some(SAVE_CYCLE));

    // Every route: the sheet's key, the header's Save, the tab's key and
    // the close dialog's Save.
    shell.keys("secondary-s", cx);
    assert!(saves(&shell, cx).is_empty(), "the sheet's key");
    assert_eq!(
        shell.label("toolbar-status", cx).as_deref(),
        Some(SAVE_CYCLE)
    );
    shell.keys("escape", cx);
    shell.click("item-save", cx);
    shell.keys("secondary-s", cx);
    assert!(saves(&shell, cx).is_empty(), "Save and the tab's key");
    shell.keys("secondary-w", cx);
    assert_eq!(
        shell.label("close-draft-save-state", cx).as_deref(),
        Some(SAVE_CYCLE)
    );
    shell.click("close-draft-save-button", cx);
    assert!(saves(&shell, cx).is_empty(), "the close dialog's Save");
    shell.click("close-draft-cancel-button", cx);

    // Removed, nothing is said, and nothing waits.
    open_sheet(&shell, cx);
    shell.click(("rel-dep-remove", 0usize), cx);
    assert!(!shell.present("rel-deps-problem", cx));
    assert_eq!(blocked(&shell, cx), None);
    assert!(!dirty(&shell, cx));
}

/// A parent that closes a cycle is explained under Parent, and Save waits
/// until it is cleared.
#[gpui_kit::test]
async fn a_proposed_parent_cycle_is_explained(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Main", &with_deps("mh-00001", &[]));
        fixture.commit_ticket(
            root,
            OTHER,
            "Child",
            &format!("{}parent: {TICKET}\n", with_deps("mh-00031", &[])),
        );
    });
    open(&shell, ticket(&root, TICKET), cx);
    edit(&shell, cx).await;
    open_sheet(&shell, cx);
    pick(&shell, "rel-parent-change", "Child", cx);
    assert_eq!(
        shell.label("rel-parent-problem", cx).as_deref(),
        Some("This makes a cycle: mh-00001 Main, mh-00031 Child.")
    );
    assert!(inside(&shell, "rel-parent-problem", "sheet-parent", cx));
    assert!(!shell.present("rel-deps-problem", cx));
    assert_eq!(blocked(&shell, cx), Some(SAVE_CYCLE));
    shell.keys("secondary-s", cx);
    assert!(saves(&shell, cx).is_empty());
    assert_eq!(
        shell.label("toolbar-status", cx).as_deref(),
        Some(SAVE_CYCLE)
    );

    shell.click("rel-parent-clear", cx);
    assert!(!shell.present("rel-parent-problem", cx));
    assert_eq!(blocked(&shell, cx), None);
}

/// A dependency, or a parent, that no ticket the index holds has the ID of
/// says "not found, kept"; a change beside it is checked, still says it,
/// and saves, keeping it.
#[gpui_kit::test]
async fn an_unresolved_dependency_is_shown_and_saves(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, OTHER, "Other", &with_deps("mh-00041", &[]));
        fixture.commit_ticket(
            root,
            TICKET,
            "Main",
            &format!(
                "{}parent: {MISSING_PARENT}\n",
                with_deps("mh-00001", &[MISSING])
            ),
        );
    });
    open(&shell, ticket(&root, TICKET), cx);
    edit(&shell, cx).await;
    open_sheet(&shell, cx);
    assert_eq!(shell.rows("rel-dep-id", cx), [MISSING]);
    assert_eq!(shell.rows("rel-dep-state", cx), [NOT_FOUND_KEPT]);
    assert_eq!(
        shell.label("rel-parent-state", cx).as_deref(),
        Some(NOT_FOUND_KEPT)
    );

    pick(&shell, "rel-deps-add", "Other", cx);
    assert_eq!(shell.rows("rel-dep-id", cx), [OTHER, MISSING]);
    assert_eq!(
        shell.rows("rel-dep-state", cx),
        [LIFECYCLE_OPEN, NOT_FOUND_KEPT],
        "the check's answer keeps it unresolved"
    );
    assert_eq!(blocked(&shell, cx), None, "it saves");
    assert!(!shell.present("rel-deps-problem", cx));
    shell.keys("escape", cx);
    let request = save(&shell, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    let item = shown(&shell, &root, TICKET);
    assert_eq!(dep_ids(&item), [OTHER, MISSING]);
    assert_eq!(item.deps[1].state, DependencyState::Unresolved);
    assert_eq!(
        item.parent.map(|parent| parent.id).as_deref(),
        Some(MISSING_PARENT),
        "kept"
    );
}

/// An answer from an index that is not current adds "Save checks again"
/// under the relationships; one from a current, complete index adds
/// nothing.
#[gpui_kit::test]
async fn an_incomplete_index_adds_its_note(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Main", &with_deps("mh-00001", &[]));
        fixture.commit_ticket(root, OTHER, "Other", &with_deps("mh-00041", &[]));
    });
    open(&shell, ticket(&root, TICKET), cx);
    edit(&shell, cx).await;
    open_sheet(&shell, cx);
    pick(&shell, "rel-deps-add", "Other", cx);
    assert_eq!(shell.rows("rel-dep-id", cx), [OTHER]);
    assert!(!shell.present("rel-index-note", cx), "current and complete");

    mark_behind(&shell);
    shell.click(("rel-dep-remove", 0usize), cx);
    pick(&shell, "rel-deps-add", "Other", cx);
    assert_eq!(
        shell.label("rel-index-note", cx).as_deref(),
        Some(SAVE_CHECKS_AGAIN)
    );
    assert!(inside(&shell, "rel-index-note", "relationships", cx));
    assert_eq!(blocked(&shell, cx), None, "advisory: Save still saves");
}

/// R-26: a check still running does not hold Save; and one asked before a
/// newer change never replaces that change's answer when it lands late,
/// even with the same proposal and a different answer.
#[gpui_kit::test]
async fn a_late_check_does_not_replace_a_newer_one(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Main", &with_deps("mh-00001", &[]));
        fixture.commit_ticket(root, OTHER, "Later", &with_deps("mh-00051", &[]));
        fixture.commit_ticket(root, THIRD, "Cyclic", &with_deps("mh-00052", &[TICKET]));
    });
    open(&shell, ticket(&root, TICKET), cx);
    edit(&shell, cx).await;
    open_sheet(&shell, cx);
    pick(&shell, "rel-deps-add", "Cyclic", cx);
    assert_eq!(blocked(&shell, cx), Some(SAVE_CYCLE));

    // The next check is held: while it runs, nothing waits.
    let held = shell.backend.hold_read(ReadKind::RelationshipCheck);
    shell.click(("rel-dep-remove", 0usize), cx);
    pick(&shell, "rel-deps-add", "Later", cx);
    assert_eq!(blocked(&shell, cx), None, "a check in flight holds nothing");
    assert!(!shell.present("rel-deps-problem", cx));

    // A newer change of the same proposal is answered: no cycle.
    shell.click(("rel-dep-remove", 0usize), cx);
    pick(&shell, "rel-deps-add", "Later", cx);
    assert_eq!(blocked(&shell, cx), None);
    // Then the index learns of a cycle through it, and the held check,
    // asked first, reads it as it lands.
    shell
        .fixture
        .commit_ticket(&root, OTHER, "Later", &with_deps("mh-00051", &[TICKET]));
    shell.fixture.refresh(&root);
    let checks = shell.backend.reads_seen(ReadKind::RelationshipCheck);
    // The held check may already be dropped with its task; either way it
    // is let go, and its stamp is no longer current.
    let _ = held.send(());
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    assert_eq!(
        shell.backend.reads_seen(ReadKind::RelationshipCheck),
        checks
    );
    assert!(proposed(&shell, cx).deps == [OTHER]);
    assert!(
        !shell.present("rel-deps-problem", cx),
        "the late answer is dropped"
    );
    assert_eq!(blocked(&shell, cx), None);

    // A change now asks again, and is answered with the cycle.
    shell.click(("rel-dep-remove", 0usize), cx);
    pick(&shell, "rel-deps-add", "Later", cx);
    assert_eq!(
        shell.label("rel-deps-problem", cx).as_deref(),
        Some("This makes a cycle: mh-00001 Main, mh-00051 Later.")
    );
}

/// R-72: a file's dependencies, written out of order, are chips in ID
/// order; one added goes where its ID sorts, in the chips, the session and
/// the draft.
#[gpui_kit::test]
async fn deps_are_shown_and_drafted_in_id_order(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, OTHER, "Two", &with_deps("mh-00062", &[]));
        fixture.commit_ticket(root, THIRD, "Three", &with_deps("mh-00063", &[]));
        fixture.commit_ticket(root, DEP_B, "Five", &with_deps("mh-00065", &[]));
        fixture.commit_ticket(
            root,
            TICKET,
            "Main",
            &with_deps("mh-00001", &[DEP_B, OTHER]),
        );
    });
    assert_eq!(
        dep_ids(&shown_raw(&shell, &root)),
        [DEP_B, OTHER],
        "file order"
    );
    open(&shell, ticket(&root, TICKET), cx);
    edit(&shell, cx).await;
    open_sheet(&shell, cx);
    assert_eq!(shell.rows("rel-dep-id", cx), [OTHER, DEP_B]);
    assert_eq!(
        shell.rows("rel-dep-name", cx),
        ["mh-00062 Two", "mh-00065 Five"]
    );
    assert!(proposed(&shell, cx).deps == [OTHER, DEP_B]);
    assert!(!dirty(&shell, cx), "the order is no change");

    pick(&shell, "rel-deps-add", "Three", cx);
    assert_eq!(shell.rows("rel-dep-id", cx), [OTHER, THIRD, DEP_B]);
    assert!(proposed(&shell, cx).deps == [OTHER, THIRD, DEP_B]);
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");
    let drafted = registry(&shell, cx).read_with(cx, |registry, _| {
        registry.session(id).unwrap().current().clone()
    });
    assert!(Proposed::of(&drafted).unwrap().deps == [OTHER, THIRD, DEP_B]);
}

/// The item as the library reads it now, not normalized.
fn shown_raw(shell: &Shell, root: &Path) -> ItemDto {
    shown(shell, root, TICKET)
}

/// Contract 4: a dependency added and then removed leaves the session
/// clean, so no save is sent at all; a later save of another field sends
/// both relationships as unchanged.
#[gpui_kit::test]
async fn adding_then_removing_a_dependency_returns_the_tab_to_clean_and_sends_unchanged(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, OTHER, "Kept", &with_deps("mh-00071", &[]));
        fixture.commit_ticket(root, THIRD, "Passing", &with_deps("mh-00072", &[]));
        fixture.commit_ticket(
            root,
            TICKET,
            "Main",
            &format!("{}parent: {THIRD}\n", with_deps("mh-00001", &[OTHER])),
        );
    });
    open(&shell, ticket(&root, TICKET), cx);
    edit(&shell, cx).await;
    open_sheet(&shell, cx);
    let base = meta(&shell, cx);
    pick(&shell, "rel-deps-add", "Passing", cx);
    assert!(dirty(&shell, cx));
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_UNSAVED));
    let ids = shell.rows("rel-dep-id", cx);
    let at = ids.iter().position(|id| id == THIRD).expect("the chip");
    shell.click(("rel-dep-remove", at), cx);
    assert!(meta(&shell, cx) == base, "back to the base exactly");
    assert!(!dirty(&shell, cx));
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_SAVED));
    shell.keys("secondary-s", cx);
    assert!(saves(&shell, cx).is_empty(), "clean: no save is sent");
    assert_eq!(
        shell.label("toolbar-status", cx).as_deref(),
        Some(NO_CHANGES_TO_SAVE)
    );
    shell.keys("escape", cx);

    retitle(&shell, "Renamed", cx);
    let request = save(&shell, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    let (deps, parent) = writes(&shell, request, cx);
    assert!(deps == RelationshipWrite::Unchanged, "deps unchanged");
    assert!(parent == RelationshipWrite::Unchanged, "parent unchanged");
    let item = shown(&shell, &root, TICKET);
    assert_eq!(dep_ids(&item), [OTHER]);
    assert_eq!(item.title.as_deref(), Some("Renamed"));
}

/// R-72: a save of a ticket whose file listed its dependencies out of
/// order leaves a verified base equal to the read after the save.
#[gpui_kit::test]
async fn saving_a_ticket_whose_file_has_unsorted_deps_leaves_a_verified_base(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, OTHER, "Two", &with_deps("mh-00062", &[]));
        fixture.commit_ticket(root, THIRD, "Three", &with_deps("mh-00063", &[]));
        fixture.commit_ticket(root, DEP_B, "Five", &with_deps("mh-00065", &[]));
        fixture.commit_ticket(
            root,
            TICKET,
            "Main",
            &with_deps("mh-00001", &[DEP_B, OTHER]),
        );
    });
    open(&shell, ticket(&root, TICKET), cx);
    edit(&shell, cx).await;
    open_sheet(&shell, cx);
    pick(&shell, "rel-deps-add", "Three", cx);
    shell.keys("escape", cx);
    let request = save(&shell, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    let read = shown(&shell, &root, TICKET);
    assert_eq!(dep_ids(&read), [OTHER, THIRD, DEP_B], "written in ID order");
    let pane = pane(&shell, cx);
    let observation = read.observation.clone();
    wait_until(cx, &pane, move |pane, _| {
        pane.edit_session()
            .is_some_and(|session| session.base().observation == observation)
    })
    .await;
    let after = DraftFields::of(&read).expect("a ticket's fields");
    let verified = pane.read_with(cx, |pane, _| {
        let base = pane.edit_session().expect("the session goes on").base();
        base.observation.is_some() && base.fields == after
    });
    assert!(verified, "the base is the post-save read, verified");
    assert!(!dirty(&shell, cx));
}

/// The library's own `relationship_cycle` answer to a save, its IDs in
/// `rejected`, is explained in the same words and place as the check's, and
/// Save waits; the session stays dirty, its draft kept. The answer comes
/// from a race: the check found no cycle, then the index learned of one
/// before Save.
#[gpui_kit::test]
async fn a_library_relationship_cycle_answer_is_explained_the_same_way(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Main", &with_deps("mh-00001", &[MISSING]));
        fixture.commit_ticket(root, OTHER, "Racer", &with_deps("mh-00081", &[]));
    });
    open(&shell, ticket(&root, TICKET), cx);
    edit(&shell, cx).await;
    open_sheet(&shell, cx);
    pick(&shell, "rel-deps-add", "Racer", cx);
    assert_eq!(blocked(&shell, cx), None, "the check finds no cycle");
    assert_eq!(shell.rows("rel-dep-id", cx), [OTHER, MISSING]);
    assert_eq!(
        shell.rows("rel-dep-state", cx),
        [LIFECYCLE_OPEN, NOT_FOUND_KEPT]
    );
    shell.keys("escape", cx);

    // Another actor makes the racer depend on this ticket.
    shell
        .fixture
        .commit_ticket(&root, OTHER, "Racer", &with_deps("mh-00081", &[TICKET]));
    shell.fixture.refresh(&root);
    let request = save(&shell, cx).await;
    assert_eq!(
        code(&shell, request, cx),
        Some(ResultCode::RelationshipCycle)
    );
    assert_eq!(
        shell.label("item-save-code", cx).as_deref(),
        Some("relationship_cycle")
    );
    open_sheet(&shell, cx);
    let words = "This makes a cycle: mh-00001 Main, mh-00081 Racer.";
    assert_eq!(shell.label("rel-deps-problem", cx).as_deref(), Some(words));
    assert!(inside(&shell, "rel-deps-problem", "sheet-waiting-on", cx));
    assert_eq!(blocked(&shell, cx), Some(SAVE_CYCLE));
    shell.keys("secondary-s", cx);
    assert_eq!(saves(&shell, cx).len(), 1, "Save waits");
    assert!(dirty(&shell, cx), "nothing is lost");
    assert!(proposed(&shell, cx).deps == [OTHER, MISSING]);
    // The refusal changes only the rejection: what the check said of the
    // unresolved dependency stays (fix 1, I1).
    assert_eq!(
        shell.rows("rel-dep-state", cx),
        [LIFECYCLE_OPEN, NOT_FOUND_KEPT],
        "still not found, kept"
    );
    assert!(
        attached(&shell, &root, TICKET, cx).is_some(),
        "the draft is kept"
    );

    // The check, asked again, says the same in the same place.
    shell.click(("rel-dep-remove", 0usize), cx);
    assert!(!shell.present("rel-deps-problem", cx));
    pick(&shell, "rel-deps-add", "Racer", cx);
    assert_eq!(shell.label("rel-deps-problem", cx).as_deref(), Some(words));
    assert_eq!(blocked(&shell, cx), Some(SAVE_CYCLE));

    // Saved while its check still runs, which holds nothing: the refusal is
    // held, and the check, landing after it, keeps it and says the
    // unresolved dependency is still not found, kept (fix 1, I1).
    shell.click(("rel-dep-remove", 0usize), cx);
    let held = shell.backend.hold_read(ReadKind::RelationshipCheck);
    pick(&shell, "rel-deps-add", "Racer", cx);
    assert_eq!(blocked(&shell, cx), None, "the check is in flight");
    shell.keys("secondary-s", cx);
    let sent = saves(&shell, cx);
    assert_eq!(sent.len(), 2, "the sheet's key saved");
    ended(&shell, sent[0], cx).await;
    assert_eq!(
        code(&shell, sent[0], cx),
        Some(ResultCode::RelationshipCycle)
    );
    held.send(()).expect("the check is still running");
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    assert_eq!(shell.label("rel-deps-problem", cx).as_deref(), Some(words));
    assert_eq!(blocked(&shell, cx), Some(SAVE_CYCLE));
    assert_eq!(
        shell.rows("rel-dep-state", cx),
        [LIFECYCLE_OPEN, NOT_FOUND_KEPT]
    );
}

/// R-54: Dependents and Children are D1's rows while editing too, each
/// opening its ticket, which closes the sheet first; nothing typed on them
/// changes the session, and they offer no control.
#[gpui_kit::test]
async fn dependents_and_children_stay_read_only(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Main", &with_deps("mh-00001", &[]));
        fixture.commit_ticket(root, OTHER, "Dependent", &with_deps("mh-00091", &[TICKET]));
        fixture.commit_ticket(
            root,
            THIRD,
            "Child",
            &format!("{}parent: {TICKET}\n", with_deps("mh-00031", &[])),
        );
    });
    open(&shell, ticket(&root, TICKET), cx);
    edit(&shell, cx).await;
    open_sheet(&shell, cx);
    let base = meta(&shell, cx);
    for (row, within, cell, id) in [
        (
            ("rel-dependent", 0usize),
            "sheet-dependents",
            "rel-dependent-0-id",
            OTHER,
        ),
        (
            ("rel-child", 0usize),
            "sheet-children",
            "rel-child-0-id",
            THIRD,
        ),
    ] {
        assert!(inside(&shell, row, within, cx), "{within}");
        assert_eq!(shell.label(cell, cx).as_deref(), Some(id), "{cell}");
    }
    assert!(!shell.present(("rel-dependent", 1usize), cx));
    cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("pasted".to_owned()));
    for row in [("rel-dependent", 0usize), ("rel-child", 0usize)] {
        let mut presses = 0;
        while !focused(&shell, row, cx) {
            presses += 1;
            assert!(presses < 40, "Tab never reached {row:?}");
            shell.keys("tab", cx);
        }
        type_in(&shell, "q", cx);
        shell.keys("backspace delete secondary-v", cx);
        assert!(meta(&shell, cx) == base, "{row:?} changes nothing");
    }
    assert!(!dirty(&shell, cx));
    assert!(shell.present(("rel-dependent", 0usize), cx));
    assert!(shell.present(("rel-child", 0usize), cx));

    // Opening one closes the sheet first.
    shell.press("enter", cx);
    assert!(!has_sheet(&shell, cx));
    let shown_target = pane(&shell, cx).read_with(cx, |pane, _| pane.target().clone());
    assert_eq!(shown_target, ticket(&root, THIRD));
}

/// R-74: while the tab is Preparing or Loading, in the review, and in a
/// session kept after Done, the chips, Remove, Add…, Change… and Clear are
/// not offered: the sheet shows the relationships read only. Once Editing,
/// they are controls again.
#[gpui_kit::test]
async fn the_relationship_controls_are_read_only_while_preparing_loading_in_review_and_kept(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, OTHER, "Dep", &with_deps("mh-00091", &[]));
        fixture.commit_ticket(root, THIRD, "Up", &with_deps("mh-00092", &[]));
        fixture.commit_ticket(
            root,
            TICKET,
            "Mine",
            &format!("{}parent: {THIRD}\n", with_deps("mh-00093", &[OTHER])),
        );
        fixture.commit_ticket(root, DEP_A, "Pending", &with_deps("mh-00094", &[OTHER]));
    });

    // Preparing.
    open(&shell, ticket(&root, TICKET), cx);
    let held = shell.backend.hold_job(JobKind::ContextPrepare);
    shell.click("item-edit", cx);
    held.wait_reached();
    shell.run(cx);
    assert!(matches!(
        pane(&shell, cx).read_with(cx, |pane, _| pane.edit_phase()),
        Phase::Preparing(_)
    ));
    open_sheet(&shell, cx);
    assert!(no_controls(&shell, cx), "preparing");
    assert!(shell.present(("rel-dependency", 0usize), cx), "D1's rows");
    shell.keys("escape", cx);
    held.release();
    until_editing(&shell, cx).await;
    open_sheet(&shell, cx);
    assert!(shell.present("rel-deps-add", cx), "Editing: controls");
    assert!(shell.present(("rel-dep-remove", 0usize), cx));
    assert!(shell.present("rel-parent-change", cx));
    assert!(shell.present("rel-parent-clear", cx));
    shell.keys("escape", cx);

    // In the review: the session's relationships, read only; the review
    // shows the one added as mine, and Use keeps it.
    open_sheet(&shell, cx);
    pick(&shell, "rel-deps-add", "Pending", cx);
    shell.keys("escape", cx);
    retitle(&shell, "My title", cx);
    second_actor(
        &shell.fixture,
        Actor::SaveTicket {
            root: &root,
            id: TICKET,
            title: "Their title",
            status: "open",
        },
    );
    let request = save(&shell, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::ExternalChange));
    until_review(&shell, cx).await;
    open_sheet(&shell, cx);
    assert!(no_controls(&shell, cx), "in the review");
    assert_eq!(
        shell.rows("rel-dep-id", cx),
        [OTHER, DEP_A],
        "mine, read only"
    );
    assert_eq!(shell.label("rel-parent-id", cx).as_deref(), Some(THIRD));
    shell.keys("escape", cx);
    let deps = shell
        .rows("review-field", cx)
        .iter()
        .position(|field| field == "Dependencies")
        .expect("the review lists the dependencies");
    assert_eq!(
        shell.label(("review-mine", deps), cx),
        Some(format!("{OTHER}, {DEP_A}")),
        "the review shows the change as mine"
    );
    let title = shell
        .rows("review-field", cx)
        .iter()
        .position(|field| field == "Title")
        .expect("the review lists the title");
    shell.click(("review-take-theirs", title), cx);
    let checks = shell.backend.reads_seen(ReadKind::RelationshipCheck);
    shell.click("review-use", cx);
    assert!(
        proposed(&shell, cx).deps == [OTHER, DEP_A],
        "Use keeps the chosen relationships"
    );
    assert_eq!(
        shell.backend.reads_seen(ReadKind::RelationshipCheck),
        checks + 1,
        "Use moved the base: what the session proposes is checked again"
    );
    open_sheet(&shell, cx);
    assert!(shell.present("rel-deps-add", cx), "Editing again");
    assert_eq!(shell.rows("rel-dep-id", cx), [OTHER, DEP_A]);
    shell.keys("escape", cx);

    // Kept after Done: the saved copy's rows.
    retitle(&shell, "Kept title", cx);
    shell.click("item-done", cx);
    let kept = pane(&shell, cx).read_with(cx, |pane, _| {
        pane.edit_session().is_some_and(|session| session.kept())
    });
    assert!(kept, "the session is kept");
    open_sheet(&shell, cx);
    assert!(no_controls(&shell, cx), "kept");
    assert!(shell.present(("rel-dependency", 0usize), cx), "D1's rows");
    shell.keys("escape", cx);

    // Loading: a prepare answered `discovery_pending` waits there.
    open(&shell, ticket(&root, DEP_A), cx);
    shell.backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            shell.fixture.data_dir(),
            manyhands::repository::FailurePoint::BeforeIndexTransactionCommit,
        )
        .unwrap(),
    );
    shell.click("item-edit", cx);
    let request = prepares(&shell, cx)[0];
    ended(&shell, request, cx).await;
    assert_eq!(
        pane(&shell, cx).read_with(cx, |pane, _| pane.edit_phase()),
        Phase::Loading
    );
    open_sheet(&shell, cx);
    assert!(no_controls(&shell, cx), "loading");
    assert!(shell.present(("rel-dependency", 0usize), cx));
    shell.keys("escape", cx);
    shell
        .backend
        .replace_service(RepositoryService::open_at(shell.fixture.data_dir()).unwrap());
}
