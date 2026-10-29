//! The item's header block, over real repositories: the breadcrumb, the
//! chips and the details they ask for, the state pill, the later actions,
//! the banners that D1's status lines became, a nonconforming file's
//! "Needs fixing", and the tab order from the strip into the page.

use std::{cell::RefCell, path::Path, rc::Rc, time::Duration};

use gpui_kit::{ElementId, Entity, Role, TestAppContext, test::TestWindowExt};
use manyhands::{
    repository::{
        ItemDto, ItemDtoKind, LeaseHolderForTesting, LeaseKind, ProblemDto, RepositoryService,
        TicketFilter,
    },
    results::{ProblemCode, ResultCode},
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use super::{
    ItemPane, NOT_REFRESHED, RECOVERY,
    banners::{Banner, NEEDS_FIXING, Tone, most_severe_first},
    chips::{ChipField, JUST_NOW, OpenDetails, relative_time},
    header::{CRUMB_SEPARATOR, TICKETS},
    pill::{PillState, pill_words, viewing_state},
};
use crate::desktop::{
    model::{
        item::{APPROVE_AND_MERGE, CLOSE_TICKET, COMMENT, ItemReads, LATER_NOTE, REPAIR, SHARE},
        list_state::LABEL_BUSY,
        navigation::OpenTarget,
    },
    services::ReadResult,
    test_support::{
        drafts::store_draft,
        fixtures::{Fixture, item_id, ticket_path},
        snapshot::RepositorySnapshot,
    },
    view::harness::{Shell, key},
};

const CODED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FP0";
const UNCODED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FP1";
const PARENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FP2";
const CHILD: &str = "01ARZ3NDEKTSV4RRFFQ69G5FP3";
const OPEN_DEP: &str = "01ARZ3NDEKTSV4RRFFQ69G5FP4";
const CLOSED_DEP: &str = "01ARZ3NDEKTSV4RRFFQ69G5FP5";
const BLOCKED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FP6";
const READY: &str = "01ARZ3NDEKTSV4RRFFQ69G5FP7";
const SAVED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FP8";
const CLOSED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FP9";
const BROKEN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FPA";
const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FQ0";
const OTHER_DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FQ1";
const SLUG: &str = "ae-k9x2b";
const PARENT_SLUG: &str = "mh-vh-w03xy";
const MARKER: &str = "docs/marker.md";

/// A managed document's source at `id`, with `extra` front matter lines.
fn document_source(id: &str, title: &str, extra: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {id}\ntitle: {title}\n\
         {extra}---\nBody of {title}.\n"
    )
}

/// One repository, `a`, built by `build` and refreshed, in an open
/// window.
fn shell(
    cx: &mut TestAppContext,
    build: impl FnOnce(&Fixture, &Path),
) -> (Shell, std::path::PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    build(&fixture, &root);
    fixture.refresh(&root);
    (Shell::open(fixture, cx), root)
}

fn open(shell: &Shell, target: OpenTarget, cx: &mut TestAppContext) {
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
}

fn ticket(root: &Path, id: &str) -> OpenTarget {
    OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Ticket,
        id: item_id(id),
    }
}

fn document(root: &Path, id: &str) -> OpenTarget {
    OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Document,
        id: item_id(id),
    }
}

/// A file read by its path, from the repository's root.
fn inspect(root: &Path, path: &str) -> OpenTarget {
    OpenTarget::Inspect {
        repo: key(root),
        worktree: key(root),
        path: path.to_owned(),
    }
}

fn shown(shell: &Shell, root: &Path, id: &str) -> ItemDto {
    let service = shell.fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    service.show_item(&repo, &item_id(id)).unwrap()
}

fn pane(shell: &Shell, cx: &mut TestAppContext) -> Entity<ItemPane> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.item_pane())
        .expect("an item tab is shown")
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

fn in_header(shell: &Shell, id: impl Into<ElementId>, cx: &mut TestAppContext) -> bool {
    inside(shell, id, "item-header", cx)
}

/// The role the element `id` reports to assistive tech.
fn role(shell: &Shell, id: impl Into<ElementId>, cx: &mut TestAppContext) -> Option<Role> {
    let id = id.into();
    shell.frame(cx, |window, _| {
        window.try_find(id).and_then(|element| element.role())
    })
}

fn focused(shell: &Shell, id: impl Into<ElementId>, cx: &mut TestAppContext) -> bool {
    let id = id.into();
    shell.frame(cx, |window, _| {
        window
            .try_find(id)
            .is_some_and(|element| element.focused() == Some(true))
    })
}

/// The breadcrumb names the repository, then a ticket's Tickets and short
/// code (its ID when it has none), or a document's or file's folder and
/// file name.
#[gpui_kit::test]
fn the_breadcrumb_names_the_repository_section_and_file(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, CODED, "Coded", &format!("slug: {SLUG}\n"));
        fixture.commit_ticket(root, UNCODED, "Uncoded", "");
        fixture.commit_file(
            root,
            "docs/notes/kept.md",
            document_source(DOCUMENT, "Kept", "").as_bytes(),
        );
        fixture.commit_nonconforming_document(root, MARKER);
    });
    let crumb = |parts: [&str; 3]| parts.join(CRUMB_SEPARATOR);

    open(&shell, ticket(&root, CODED), cx);
    assert_eq!(
        shell.label("item-breadcrumb", cx),
        Some(crumb(["a", TICKETS, SLUG]))
    );
    assert!(in_header(&shell, "item-breadcrumb", cx));

    open(&shell, ticket(&root, UNCODED), cx);
    assert_eq!(
        shell.label("item-breadcrumb", cx),
        Some(crumb(["a", TICKETS, UNCODED]))
    );

    open(&shell, document(&root, DOCUMENT), cx);
    assert_eq!(
        shell.label("item-breadcrumb", cx),
        Some(crumb(["a", "docs/notes", "kept.md"]))
    );

    open(&shell, inspect(&root, MARKER), cx);
    assert_eq!(
        shell.label("item-breadcrumb", cx),
        Some(crumb(["a", "docs", "marker.md"]))
    );
}

/// A ticket's chips: Type, Status, Project, Team and Parent by its short
/// code, then All details. One without project, team or parent has no
/// chip for them, and a ticket has no document chips.
#[gpui_kit::test]
fn ticket_chips_show_type_status_project_team_and_parent(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, PARENT, "Parent", &format!("slug: {PARENT_SLUG}\n"));
        fixture.commit_ticket(
            root,
            CHILD,
            "Child",
            &format!("project: manyhands\nteam: core\nparent: {PARENT}\n"),
        );
        fixture.commit_ticket(root, UNCODED, "Bare", "");
    });
    open(&shell, ticket(&root, CHILD), cx);
    for (field, label) in [
        (ChipField::Type, "Type task"),
        (ChipField::Status, "Status open"),
        (ChipField::Project, "Project manyhands"),
        (ChipField::Team, "Team core"),
        (ChipField::Parent, "Parent mh-vh-w03xy"),
        (ChipField::All, "All details"),
    ] {
        assert_eq!(shell.label(field.id(), cx).as_deref(), Some(label));
        assert!(in_header(&shell, field.id(), cx), "{label}");
    }
    for field in [
        ChipField::WaitingOn,
        ChipField::Author,
        ChipField::Date,
        ChipField::Updated,
    ] {
        assert!(!shell.present(field.id(), cx), "{field:?}");
    }

    open(&shell, ticket(&root, UNCODED), cx);
    assert!(shell.present(ChipField::Type.id(), cx));
    assert!(shell.present(ChipField::Status.id(), cx));
    for field in [ChipField::Project, ChipField::Team, ChipField::Parent] {
        assert!(!shell.present(field.id(), cx), "{field:?}");
    }
    assert!(shell.present(ChipField::All.id(), cx));
}

/// Waiting on shows while the ticket is blocked, naming the open
/// dependency; a ticket whose only dependency is closed, and one with
/// none, show no Waiting on.
#[gpui_kit::test]
fn waiting_on_shows_only_when_blocked(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.create_ticket(root, OPEN_DEP, "Open dependency", "open");
        fixture.commit_closed_ticket(root, CLOSED_DEP, "Closed dependency");
        fixture.create_ticket_with_deps(root, BLOCKED, "Blocked", "open", &[OPEN_DEP]);
        fixture.create_ticket_with_deps(root, READY, "Ready", "open", &[CLOSED_DEP]);
    });
    let dependency = shown(&shell, &root, OPEN_DEP);
    let code = dependency.slug.unwrap_or_else(|| OPEN_DEP.to_owned());

    open(&shell, ticket(&root, BLOCKED), cx);
    assert_eq!(
        shell.label(ChipField::WaitingOn.id(), cx),
        Some(format!("Waiting on {code}"))
    );
    assert!(in_header(&shell, ChipField::WaitingOn.id(), cx));

    open(&shell, ticket(&root, READY), cx);
    assert!(
        shell.present(ChipField::Status.id(), cx),
        "the ticket is read"
    );
    assert!(!shell.present(ChipField::WaitingOn.id(), cx));

    open(&shell, ticket(&root, OPEN_DEP), cx);
    assert!(
        shell.present(ChipField::Status.id(), cx),
        "the ticket is read"
    );
    assert!(!shell.present(ChipField::WaitingOn.id(), cx));
}

/// A document's Status, Author and Date are chips only when their keys
/// hold strings; Updated and All details are always there; a document has
/// no ticket chips.
#[gpui_kit::test]
fn document_chips_show_status_author_and_date_only_when_strings(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_file(
            root,
            "docs/strings.md",
            document_source(
                DOCUMENT,
                "Strings",
                "status: draft\nauthor: Vince Hodges\ndate: 2026-10-05\n",
            )
            .as_bytes(),
        );
        fixture.commit_file(
            root,
            "docs/others.md",
            document_source(
                OTHER_DOCUMENT,
                "Others",
                "status: 3\nauthor: [Ada, Grace]\ndate:\n  year: 2026\n",
            )
            .as_bytes(),
        );
    });
    open(&shell, document(&root, DOCUMENT), cx);
    for (field, label) in [
        (ChipField::Status, "Status draft"),
        (ChipField::Author, "Author Vince Hodges"),
        (ChipField::Date, "Date 2026-10-05"),
        (ChipField::All, "All details"),
    ] {
        assert_eq!(shell.label(field.id(), cx).as_deref(), Some(label));
        assert!(in_header(&shell, field.id(), cx), "{label}");
    }
    assert!(shell.present(ChipField::Updated.id(), cx));
    for field in [
        ChipField::Type,
        ChipField::Project,
        ChipField::Team,
        ChipField::Parent,
        ChipField::WaitingOn,
    ] {
        assert!(!shell.present(field.id(), cx), "{field:?}");
    }

    open(&shell, document(&root, OTHER_DOCUMENT), cx);
    for field in [ChipField::Status, ChipField::Author, ChipField::Date] {
        assert!(!shell.present(field.id(), cx), "{field:?}");
    }
    assert!(shell.present(ChipField::Updated.id(), cx));
    assert!(shell.present(ChipField::All.id(), cx));
}

/// Updated is the document's `changed_at` relative to now: just now, in
/// minutes, hours and days, then the date; a time that cannot be read is
/// shown as given.
#[gpui_kit::test]
fn updated_is_relative_to_changed_at(cx: &mut TestAppContext) {
    let now = OffsetDateTime::parse("2026-10-27T12:00:00Z", &Rfc3339).unwrap();
    for (at, words) in [
        ("2026-10-27T11:59:30Z", JUST_NOW),
        ("2026-10-27T12:05:00Z", JUST_NOW),
        ("2026-10-27T11:35:00Z", "25 min ago"),
        ("2026-10-27T13:35:00+02:00", "25 min ago"),
        ("2026-10-27T10:59:00Z", "1 hour ago"),
        ("2026-10-27T07:00:00Z", "5 hours ago"),
        ("2026-10-26T11:00:00Z", "1 day ago"),
        ("2026-10-20T12:00:00Z", "7 days ago"),
        ("2026-09-01T12:00:00Z", "2026-09-01"),
        ("not a time", "not a time"),
    ] {
        assert_eq!(relative_time(at, now), words, "{at}");
    }

    let (shell, root) = shell(cx, |fixture, root| {
        fixture.create_document(root, DOCUMENT, "docs/first.md", "First");
    });
    let changed = shown(&shell, &root, DOCUMENT)
        .changed_at
        .expect("an indexed document has a change time");
    let before = OffsetDateTime::now_utc();
    open(&shell, document(&root, DOCUMENT), cx);
    let label = shell.label(ChipField::Updated.id(), cx);
    let after = OffsetDateTime::now_utc();
    let wanted = [before, after].map(|now| format!("Updated {}", relative_time(&changed, now)));
    assert!(
        label.as_ref().is_some_and(|label| wanted.contains(label)),
        "{label:?} {wanted:?}"
    );
}

/// Every chip, by click, and a focused chip by Enter, emits
/// `OpenDetails` for its own field.
#[gpui_kit::test]
fn every_chip_emits_open_details_for_its_field(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, PARENT, "Parent", &format!("slug: {PARENT_SLUG}\n"));
        fixture.create_ticket(root, OPEN_DEP, "Open dependency", "open");
        fixture.commit_ticket(
            root,
            CHILD,
            "Child",
            &format!("project: p\nteam: t\nparent: {PARENT}\ndeps: [{OPEN_DEP}]\n"),
        );
        fixture.commit_file(
            root,
            "docs/strings.md",
            document_source(
                DOCUMENT,
                "Strings",
                "status: draft\nauthor: Vince Hodges\ndate: 2026-10-05\n",
            )
            .as_bytes(),
        );
    });
    let seen = Rc::new(RefCell::new(Vec::new()));
    let listen = |pane: &Entity<ItemPane>, cx: &mut TestAppContext| {
        let seen = seen.clone();
        cx.update(|cx| {
            cx.subscribe(pane, move |_, event: &OpenDetails, _| {
                seen.borrow_mut().push(event.0)
            })
        })
    };

    open(&shell, ticket(&root, CHILD), cx);
    let _ticket = listen(&pane(&shell, cx), cx);
    let fields = [
        ChipField::Type,
        ChipField::Status,
        ChipField::Project,
        ChipField::Team,
        ChipField::Parent,
        ChipField::WaitingOn,
        ChipField::All,
    ];
    for field in fields {
        shell.click(field.id(), cx);
        shell.keys("escape", cx);
    }
    assert_eq!(*seen.borrow(), fields);

    seen.borrow_mut().clear();
    shell.keys("secondary-3", cx);
    let mut presses = 0;
    while !focused(&shell, ChipField::Type.id(), cx) {
        presses += 1;
        assert!(presses < 10, "Tab never reached the first chip");
        shell.keys("tab", cx);
    }
    shell.press("enter", cx);
    assert_eq!(*seen.borrow(), [ChipField::Type]);

    seen.borrow_mut().clear();
    open(&shell, document(&root, DOCUMENT), cx);
    let _document = listen(&pane(&shell, cx), cx);
    let fields = [
        ChipField::Status,
        ChipField::Author,
        ChipField::Date,
        ChipField::Updated,
        ChipField::All,
    ];
    for field in fields {
        shell.click(field.id(), cx);
        shell.keys("escape", cx);
    }
    assert_eq!(*seen.borrow(), fields);
}

/// The pill's words for every state, and the viewing state each item
/// reads as: the shared copy, a private copy, a stored draft, a closed
/// ticket, and a context that needs recovery. The pill is never
/// `item-state`.
#[gpui_kit::test]
fn the_pill_reads_each_viewing_state(cx: &mut TestAppContext) {
    for (state, words) in [
        (PillState::ViewingShared, "Viewing the shared copy"),
        (PillState::ViewingPrivate, "Viewing your private copy"),
        (
            PillState::ViewingDraftKept,
            "Viewing · unsaved changes kept",
        ),
        (PillState::Preparing, "Getting a private copy ready…"),
        (PillState::EditingUnsaved, "Editing · unsaved changes"),
        (PillState::Saving, "Saving…"),
        (PillState::Review, "Reviewing changes"),
        (
            PillState::EditingSaved { not_shared: false },
            "Editing · saved on this computer",
        ),
        (
            PillState::EditingSaved { not_shared: true },
            "Editing · saved on this computer · not shared yet",
        ),
        (PillState::Closed, "Closed"),
        (PillState::NeedsAttention, "Needs attention"),
    ] {
        assert_eq!(pill_words(state), words, "{state:?}");
    }

    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, CODED, "Shared", "");
    fixture.commit_ticket(&root, SAVED, "To be saved", "");
    fixture.commit_closed_ticket(&root, CLOSED, "Closed for good");
    fixture.commit_ticket(&root, UNCODED, "Drafted", "");
    fixture.refresh(&root);
    fixture.save_ticket(&root, SAVED, "Saved");
    store_draft(&fixture, &root, ItemDtoKind::Ticket, UNCODED, "Draft body");
    let shell = Shell::open(fixture, cx);

    for (id, state) in [
        (CODED, PillState::ViewingShared),
        (SAVED, PillState::ViewingPrivate),
        (CLOSED, PillState::Closed),
        (UNCODED, PillState::ViewingDraftKept),
    ] {
        open(&shell, ticket(&root, id), cx);
        assert_eq!(
            shell.label("item-pill", cx).as_deref(),
            Some(pill_words(state)),
            "{state:?}"
        );
        assert!(in_header(&shell, "item-pill", cx));
        assert!(!shell.present("item-state", cx), "{state:?}");
    }

    // Recovery comes first, and a stored draft adds its words: a closed
    // ticket with a stored draft whose context needs repair needs attention.
    let mut item = shown(&shell, &root, CLOSED);
    assert_eq!(viewing_state(&item, true), PillState::ClosedDraftKept);
    item.problems.push(ProblemDto {
        code: ProblemCode::ContextProblem,
        path: Some(format!(".manyhands/worktrees/{CLOSED}")),
        target_id: None,
    });
    assert_eq!(
        viewing_state(&item, true),
        PillState::NeedsAttentionDraftKept
    );
    let shared = shown(&shell, &root, CODED);
    assert_eq!(viewing_state(&shared, true), PillState::ViewingDraftKept);
    assert_eq!(viewing_state(&shared, false), PillState::ViewingShared);

    open(&shell, ticket(&root, CLOSED), cx);
    show_read(&shell, item, cx);
    assert_eq!(
        shell.label("item-pill", cx).as_deref(),
        Some(pill_words(PillState::NeedsAttention))
    );
}

/// Shows `item` in the shown tab as its read gave it, as a library that
/// reports context problems on items would (ticket 01M5B163PQTE9C1T6NMHATR3KX).
fn show_read(shell: &Shell, item: ItemDto, cx: &mut TestAppContext) {
    pane(shell, cx).update(cx, |pane, cx| {
        let stamp = pane.model.reload();
        assert!(pane.model.apply_reads(
            stamp,
            ItemReads {
                item: ReadResult::Ok(item),
                comments: None,
                dependents: None,
                children: None,
                parent: None,
            },
        ));
        cx.notify();
    });
    shell.run(cx);
}

/// Share and the primary action (Close ticket… for a ticket, Approve and
/// merge… for a document) are in the header, disabled, each with
/// "Available in a later release"; pressing one changes nothing. Comment
/// is the discussion's. Edit is available (E-6).
#[gpui_kit::test]
fn share_and_the_primary_action_are_disabled_with_their_note(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, CODED, "Ticket", "");
        fixture.create_document(root, DOCUMENT, "docs/first.md", "First");
    });
    for (target, primary) in [
        (ticket(&root, CODED), CLOSE_TICKET),
        (document(&root, DOCUMENT), APPROVE_AND_MERGE),
    ] {
        open(&shell, target, cx);
        let before = RepositorySnapshot::take(&root);
        let pill = shell.label("item-pill", cx);
        for (ix, label) in [SHARE, primary].into_iter().enumerate() {
            assert_eq!(
                shell.label(("later-action", ix), cx).as_deref(),
                Some(label)
            );
            assert!(in_header(&shell, ("later-action", ix), cx), "{label}");
            assert_eq!(
                shell.label(("later-note", ix), cx).as_deref(),
                Some(LATER_NOTE),
                "{label}"
            );
            assert!(shell.inert_button(("later-button", ix), cx), "{label}");
            // The group carries the role, so assistive tech reaches it, with
            // the note as its description (which the snapshot cannot read).
            assert_eq!(role(&shell, ("later-action", ix), cx), Some(Role::Group));
            shell.click(("later-button", ix), cx);
            assert_eq!(shell.label("item-pill", cx), pill, "{label} did nothing");
        }
        assert_eq!(
            shell.label(("later-action", 2usize), cx).as_deref(),
            Some(COMMENT)
        );
        assert!(inside(&shell, ("later-action", 2usize), "discussion", cx));
        assert!(!shell.present(("later-action", 3usize), cx));
        assert_eq!(RepositorySnapshot::take(&root), before);
    }
}

/// Holds the registry's cache lease, as a refresh writing it would: every
/// read answers `busy` while it is held.
fn hold_cache_write(shell: &Shell, root: &Path) -> LeaseHolderForTesting {
    RepositoryService::hold_lease_for_testing(root, shell.fixture.data_dir(), LeaseKind::CacheWrite)
        .unwrap()
}

/// Lets a load's busy retries run out.
fn past_the_retries(shell: &Shell, cx: &mut TestAppContext) {
    for _ in 0..6 {
        cx.executor().advance_clock(Duration::from_secs(1));
        shell.run(cx);
    }
}

/// D1's status lines are banners in the header, with D1's IDs and
/// actions: a first load that stays busy with Retry; a reload that kept
/// what was shown with its own Retry; a context that needs recovery,
/// which claims no copy; and an item not found with Refresh.
#[gpui_kit::test]
fn the_status_lines_are_banners_with_their_actions(cx: &mut TestAppContext) {
    // The design's Banners table, row by row, with its tone. The slot shows
    // the most severe first, and the table's order within a tone, whatever
    // order the banners were raised in.
    let table = [
        (Banner::Status { amber: false }, Tone::Info),
        (Banner::Status { amber: true }, Tone::Amber),
        (Banner::Recovery, Tone::Red),
        (Banner::StoredDraft, Tone::Amber),
        (Banner::KeptAfterDone, Tone::Amber),
        (Banner::DraftDiscarded, Tone::Info),
        (Banner::RecoveryOff, Tone::Amber),
        (Banner::CheckpointFailed, Tone::Red),
        (Banner::DiscoveryPending, Tone::Amber),
        (Banner::Changed, Tone::Amber),
        (Banner::ReviewBeforeSaving, Tone::Amber),
        (Banner::Refused, Tone::Red),
        (Banner::NeedsFixing, Tone::Amber),
        (Banner::Unmanaged, Tone::Amber),
        (Banner::Link, Tone::Info),
    ];
    for (banner, tone) in table {
        assert_eq!(banner.tone(), tone, "{banner:?}");
    }
    assert!(
        table.windows(2).all(|rows| rows[0].0 < rows[1].0),
        "the rows are ordered as the table lists them"
    );
    let order = |banners: &[Banner]| -> Vec<Banner> {
        let banners = banners.iter().map(|banner| (*banner, ())).collect();
        most_severe_first(banners)
            .into_iter()
            .map(|(banner, ())| banner)
            .collect()
    };
    assert_eq!(
        order(&[
            Banner::Status { amber: false },
            Banner::NeedsFixing,
            Banner::Status { amber: true },
            Banner::Recovery,
        ]),
        [
            Banner::Recovery,
            Banner::Status { amber: true },
            Banner::NeedsFixing,
            Banner::Status { amber: false },
        ]
    );
    assert_eq!(
        order(&[Banner::Refused, Banner::CheckpointFailed, Banner::Recovery]),
        [Banner::Recovery, Banner::CheckpointFailed, Banner::Refused]
    );
    assert_eq!(
        order(&[
            Banner::Link,
            Banner::Unmanaged,
            Banner::NeedsFixing,
            Banner::KeptAfterDone,
            Banner::StoredDraft,
            Banner::DraftDiscarded,
        ]),
        [
            Banner::StoredDraft,
            Banner::KeptAfterDone,
            Banner::NeedsFixing,
            Banner::Unmanaged,
            Banner::DraftDiscarded,
            Banner::Link,
        ]
    );

    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, CODED, "One", "");
        fixture.commit_ticket(root, UNCODED, "Going", "");
    });

    // A first load that stays busy: Busy, with Retry, which loads it.
    let lease = hold_cache_write(&shell, &root);
    open(&shell, ticket(&root, CODED), cx);
    past_the_retries(&shell, cx);
    assert_eq!(shell.label("item-state", cx).as_deref(), Some(LABEL_BUSY));
    assert!(in_header(&shell, "item-state", cx));
    assert!(in_header(&shell, "item-retry", cx));
    assert!(in_header(&shell, ("item-banner", 0usize), cx));
    drop(lease);
    shell.click("item-retry-button", cx);
    assert_eq!(shell.label("item-title", cx).as_deref(), Some("One"));
    assert!(!shell.present("item-retry", cx));
    assert!(
        !shell.present(("item-banner", 0usize), cx),
        "no banner applies"
    );

    // A reload that stays busy keeps the item and says so, with Retry.
    let lease = hold_cache_write(&shell, &root);
    pane(&shell, cx).update(cx, |pane, cx| {
        pane.reload(cx);
    });
    past_the_retries(&shell, cx);
    assert_eq!(
        shell.label("item-not-refreshed", cx).as_deref(),
        Some(NOT_REFRESHED)
    );
    assert!(in_header(&shell, "item-not-refreshed", cx));
    assert!(in_header(&shell, "item-not-refreshed-retry", cx));
    drop(lease);
    shell.click("item-not-refreshed-retry-button", cx);
    assert!(!shell.present("item-not-refreshed", cx));

    // A context that needs recovery says so, and claims no copy.
    let mut item = shown(&shell, &root, CODED);
    item.problems.push(ProblemDto {
        code: ProblemCode::ContextProblem,
        path: Some(format!(".manyhands/worktrees/{CODED}")),
        target_id: None,
    });
    show_read(&shell, item, cx);
    assert_eq!(shell.label("item-recovery", cx).as_deref(), Some(RECOVERY));
    assert!(in_header(&shell, "item-recovery", cx));
    assert_eq!(
        shell.label(("recovery-problem", 0usize), cx).as_deref(),
        Some(ProblemCode::ContextProblem.as_str())
    );
    assert!(!shell.present("provenance", cx));

    // An item not found, with Refresh.
    shell.fixture.commit_removal(&root, &ticket_path(UNCODED));
    shell.fixture.refresh(&root);
    open(&shell, ticket(&root, UNCODED), cx);
    assert_eq!(
        shell.label("item-failure", cx).as_deref(),
        Some(ResultCode::ItemNotFound.message())
    );
    assert_eq!(
        shell.label("item-failure-code", cx).as_deref(),
        Some(ResultCode::ItemNotFound.as_str())
    );
    for id in ["item-failure", "item-failure-code", "item-refresh"] {
        assert!(in_header(&shell, id, cx), "{id}");
    }
    assert!(!shell.present("item-title", cx));
}

/// A nonconforming file read by its path "Needs fixing": the banner names
/// its problem with D1's reason and guidance, and offers Repair…, which is
/// enabled: pressed, it opens the repair view and writes nothing. Its path
/// and source are still shown, it has no chips, and the header's primary
/// action is its kind's.
#[gpui_kit::test]
fn a_nonconforming_item_shows_needs_fixing_with_repair_offered(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_nonconforming_ticket(root, BROKEN);
        fixture.commit_nonconforming_document(root, MARKER);
    });
    let service = shell.fixture.service();
    let repo = service.resolve_repository(&root).unwrap();
    let list = service
        .list_tickets(&repo, &TicketFilter::default())
        .unwrap();
    let entry = list.items.iter().find(|item| item.id.is_none()).unwrap();
    open(&shell, OpenTarget::for_item(&key(&root), entry), cx);
    let before = RepositorySnapshot::take(&root);

    assert_eq!(
        shell.label("item-needs-fixing", cx).as_deref(),
        Some(NEEDS_FIXING)
    );
    assert!(in_header(&shell, "item-needs-fixing", cx));
    assert_eq!(
        shell.rows("item-problem", cx),
        [ProblemCode::MalformedFrontMatter.as_str()]
    );
    assert_eq!(
        shell.rows("item-problem-guidance", cx),
        [ProblemCode::MalformedFrontMatter.guidance()]
    );
    assert!(in_header(&shell, ("item-problem", 0usize), cx));
    assert_eq!(shell.label("item-repair", cx).as_deref(), Some(REPAIR));
    assert!(inside(&shell, "item-repair", "item-banners", cx));
    assert!(
        !shell.present(("later-action", 2usize), cx),
        "Repair is no later action"
    );

    assert_eq!(shell.label("item-path", cx), Some(entry.path.clone()));
    assert!(shell.present("item-nonconforming", cx));
    assert!(shell.present("item-source", cx));
    assert!(!shell.present(ChipField::All.id(), cx));
    assert_eq!(
        shell.label(("later-action", 1usize), cx).as_deref(),
        Some(CLOSE_TICKET)
    );

    // Enabled: pressed, it opens the repair view, and writes nothing.
    shell.click("item-repair-button", cx);
    assert!(shell.present("repair-source", cx));
    assert_eq!(
        shell.rows("item-problem", cx),
        [ProblemCode::MalformedFrontMatter.as_str()]
    );
    assert_eq!(RepositorySnapshot::take(&root), before);

    open(&shell, inspect(&root, MARKER), cx);
    assert!(shell.present("item-needs-fixing", cx));
    assert_eq!(
        shell.label(("later-action", 1usize), cx).as_deref(),
        Some(APPROVE_AND_MERGE)
    );
}

/// A conforming item read by its path, as the details sheet's Inspect link
/// will open one (R-64), needs no fixing: no "Needs fixing" banner and no
/// Repair. Only a nonconforming file, which has no ID, shows them.
#[gpui_kit::test]
fn an_inspect_tab_of_a_conforming_item_needs_no_fixing(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, CODED, "Readable", "");
    });
    open(&shell, inspect(&root, &ticket_path(CODED)), cx);
    assert_eq!(shell.label("item-title", cx).as_deref(), Some("Readable"));
    assert!(shell.present("item-source", cx), "the file is read");
    assert!(!shell.present("item-needs-fixing", cx));
    assert!(
        !shell.present(("item-banner", 0usize), cx),
        "no banner applies"
    );
    assert_eq!(
        shell.rows("later-action", cx),
        [SHARE, CLOSE_TICKET],
        "the header's actions only, with no Repair"
    );
}

/// Tab runs from the tab strip into the header, through Edit and the
/// chips in order, then into the page; the header's unavailable actions
/// take no focus. Shift-Tab runs back.
#[gpui_kit::test]
fn tab_order_runs_strip_header_chips_page(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, UNCODED, "Walked", "project: p\nteam: t\n");
    });
    open(&shell, ticket(&root, UNCODED), cx);
    shell.keys("secondary-3", cx);
    assert!(focused(&shell, ("tab", 0usize), cx), "the strip has focus");

    let chips = [
        ChipField::Type,
        ChipField::Status,
        ChipField::Project,
        ChipField::Team,
        ChipField::All,
    ];
    let mut presses = 0;
    while !focused(&shell, "item-comments", cx) {
        presses += 1;
        assert!(presses < 4, "Tab never left the strip for the header");
        assert!(
            on_the_strip(&shell, cx),
            "Tab stays in the strip until the header"
        );
        shell.keys("tab", cx);
    }
    shell.keys("tab", cx);
    assert!(focused(&shell, "item-edit", cx), "Edit after Comments");
    shell.keys("tab", cx);
    assert!(focused(&shell, chips[0].id(), cx), "the chips after Edit");
    for later in 0..2usize {
        assert!(!focused(&shell, ("later-action", later), cx));
    }
    for pair in chips.windows(2) {
        shell.keys("tab", cx);
        assert!(
            focused(&shell, pair[1].id(), cx),
            "{:?} after {:?}",
            pair[1],
            pair[0]
        );
    }
    shell.keys("tab", cx);
    assert!(focused(&shell, "item-page", cx), "the page holds focus");

    shell.keys("shift-tab", cx);
    assert!(focused(&shell, ChipField::All.id(), cx));
    for _ in 1..chips.len() {
        shell.keys("shift-tab", cx);
    }
    assert!(focused(&shell, chips[0].id(), cx));
    shell.keys("shift-tab", cx);
    assert!(focused(&shell, "item-edit", cx), "Edit before the chips");
    shell.keys("shift-tab", cx);
    assert!(focused(&shell, "item-comments", cx), "Comments before Edit");
    shell.keys("shift-tab", cx);
    assert!(on_the_strip(&shell, cx), "back in the strip");
}

/// Whether the focus is on the strip's only tab or its close button.
fn on_the_strip(shell: &Shell, cx: &mut TestAppContext) -> bool {
    focused(shell, ("tab", 0usize), cx) || focused(shell, ("tab-close", 0usize), cx)
}

/// A kept or stored draft is never hidden by the pill: a closed ticket and
/// one whose context needs recovery add "unsaved changes kept" to their
/// words.
#[gpui_kit::test]
fn the_pill_adds_a_kept_draft_to_closed_and_needs_attention(cx: &mut TestAppContext) {
    for (state, words) in [
        (PillState::ClosedDraftKept, "Closed · unsaved changes kept"),
        (
            PillState::NeedsAttentionDraftKept,
            "Needs attention · unsaved changes kept",
        ),
    ] {
        assert_eq!(pill_words(state), words, "{state:?}");
    }

    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_closed_ticket(&root, CLOSED, "Closed for good");
    fixture.refresh(&root);
    store_draft(&fixture, &root, ItemDtoKind::Ticket, CLOSED, "Draft body");
    let shell = Shell::open(fixture, cx);

    open(&shell, ticket(&root, CLOSED), cx);
    assert_eq!(
        shell.label("item-pill", cx).as_deref(),
        Some("Closed · unsaved changes kept")
    );

    let mut item = shown(&shell, &root, CLOSED);
    item.problems.push(ProblemDto {
        code: ProblemCode::ContextProblem,
        path: Some(format!(".manyhands/worktrees/{CLOSED}")),
        target_id: None,
    });
    show_read(&shell, item, cx);
    assert_eq!(
        shell.label("item-pill", cx).as_deref(),
        Some("Needs attention · unsaved changes kept")
    );
}
