//! The Drafts page over real repositories and a real store: a row per
//! target state, each with its own text and exactly the actions the
//! restore policy allows; unreadable drafts; a row being checked; a draft
//! restored here and its Show; an empty store and one whose listing
//! failed; a late read after a new listing; and a newer-format file left
//! as it is.
//!
//! Draft types derive no `Debug`; tests read them through `match` and
//! compare text.

use std::path::{Path, PathBuf};

use gpui_kit::{Entity, TestAppContext, test::TestWindowExt};
use manyhands::{
    drafts::{
        CommentTarget, DraftBase, DraftError, DraftErrorKind, DraftFields, DraftId, DraftTarget,
        ItemKind, TargetState,
    },
    repository::{ItemDtoKind, LeaseKind, RepositoryService},
};

use super::{ACTIONS, CANNOT_READ, CHECKING, COMMENT_LATER, NO_DRAFTS, RESTORED_HERE, row_id};
use crate::desktop::{
    model::{
        drafts::rows::{COMMENT_ON, TICKET as TICKET_KIND},
        navigation::OpenTarget,
        stamp::{Stamp, ViewScope},
    },
    services::{ReadKind, drafts::DraftRegistry},
    test_support::{
        drafts::store_draft,
        fixtures::{Fixture, item_id, remove_registration, ticket_path},
    },
    view::{
        draft_comparison::{
            ALREADY_SAVED, CHANGED, CURRENT, ITEM_NOT_FOUND, NONCONFORMING, PARENT_NOT_FOUND,
            READ_FAILED, REPOSITORY_UNAVAILABLE, TICKET_CLOSED, UNVERIFIED,
        },
        harness::{Shell, key},
    },
};

pub(super) const ONE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD0";
pub(super) const TWO: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD1";

/// A repository `name` with the ticket `id` titled `title`, indexed.
pub(super) fn with_ticket(fixture: &Fixture, name: &str, id: &str, title: &str) -> PathBuf {
    let root = fixture.create_repository(name);
    fixture.commit_ticket(&root, id, title, "");
    fixture.refresh(&root);
    root
}

pub(super) fn ticket(root: &Path, id: &str) -> OpenTarget {
    OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Ticket,
        id: item_id(id),
    }
}

pub(super) fn open(shell: &Shell, target: OpenTarget, cx: &mut TestAppContext) {
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
}

pub(super) fn drafts_shown(shell: &Shell, cx: &mut TestAppContext) -> bool {
    shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.drafts_shown(cx))
}

pub(super) fn registry(shell: &Shell, cx: &mut TestAppContext) -> Entity<DraftRegistry> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().drafts().clone())
}

/// The actions row `id` offers, by their element parts, in `ACTIONS`
/// order.
pub(super) fn actions(shell: &Shell, id: DraftId, cx: &mut TestAppContext) -> Vec<&'static str> {
    ACTIONS
        .iter()
        .map(|(part, _)| *part)
        .filter(|part| shell.present(row_id(id, part), cx))
        .collect()
}

pub(super) fn state(shell: &Shell, id: DraftId, cx: &mut TestAppContext) -> Option<String> {
    shell.label(row_id(id, "state"), cx)
}

/// Stores a draft of the ticket `id` of `root` with `current` over
/// `base`, and lets it go.
pub(super) fn store(
    fixture: &Fixture,
    root: &str,
    id: &str,
    base: Option<DraftBase>,
    current: DraftFields,
) -> DraftId {
    store_target(
        fixture,
        DraftTarget {
            repository: root.to_owned(),
            item_kind: ItemKind::Ticket,
            item_id: item_id(id),
            comment: None,
        },
        base,
        current,
    )
}

pub(super) fn store_target(
    fixture: &Fixture,
    target: DraftTarget,
    base: Option<DraftBase>,
    current: DraftFields,
) -> DraftId {
    match fixture.draft_store().create(target, base, current) {
        Ok(claim) => claim.draft().id,
        Err(error) => panic!("the draft must be stored: {error}"),
    }
}

/// The ticket as it reads now, as a draft's base.
fn base_of(fixture: &Fixture, root: &Path, id: &str) -> DraftBase {
    let service = fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    DraftBase::of(&service.show_item(&repo, &item_id(id)).unwrap()).unwrap()
}

pub(super) fn ticket_fields(title: &str, body: &str) -> DraftFields {
    DraftFields::Ticket {
        title: title.to_owned(),
        ticket_type: "task".to_owned(),
        status: "open".to_owned(),
        project: None,
        team: None,
        deps: Vec::new(),
        parent: None,
        body: body.to_owned(),
    }
}

/// A file in the drafts folder under a fresh draft ID: `bytes`, or a
/// folder when `None`.
fn stray(fixture: &Fixture, bytes: Option<&[u8]>) -> (DraftId, PathBuf) {
    fixture.draft_store();
    let id = DraftId::generate();
    let path = fixture.data_dir().join("drafts").join(format!("{id}.json"));
    match bytes {
        Some(bytes) => std::fs::write(&path, bytes).unwrap(),
        None => std::fs::create_dir(&path).unwrap(),
    }
    (id, path)
}

/// What a readable row offers once its item is read, Restore aside.
const REFUSED: [&str; 3] = ["inspect", "export", "discard"];

const NEWER: &[u8] = br#"{"format": "manyhands-draft", "version": 2, "id": "kept as it is"}"#;

/// Every target state and every unreadable reason, one row each, with its
/// text and exactly the actions the restore policy allows.
#[gpui_kit::test]
fn each_row_reads_its_state_and_offers_what_its_policy_allows(cx: &mut TestAppContext) {
    const CURRENT_ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE0";
    const CHANGED_ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE1";
    const SAVED_ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE2";
    const UNVERIFIED_ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE3";
    const CLOSED_ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE4";
    const GONE_ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE5";
    const BROKEN_ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE6";
    const BUSY_ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE7";
    const LOOSE_ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE8";
    const ABSENT_PARENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE9";
    const ORPHAN_COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FEA";
    const NEW_COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FEB";

    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    for (id, title) in [
        (CURRENT_ID, "Current"),
        (CHANGED_ID, "Changed"),
        (SAVED_ID, "Saved"),
        (UNVERIFIED_ID, "Unverified"),
        (GONE_ID, "Gone"),
        (BROKEN_ID, "Broken"),
    ] {
        fixture.commit_ticket(&a, id, title, "");
    }
    fixture.commit_closed_ticket(&a, CLOSED_ID, "Closed");
    fixture.refresh(&a);
    let b = with_ticket(&fixture, "b", BUSY_ID, "Busy");
    let c = with_ticket(&fixture, "c", LOOSE_ID, "Loose");
    let root = key(&a);

    let mut rows: Vec<(DraftId, &str, Vec<&str>)> = Vec::new();
    let restorable = || [&["restore"][..], &REFUSED].concat();
    let current = store_draft(&fixture, &a, ItemDtoKind::Ticket, CURRENT_ID, "Mine");
    rows.push((current, CURRENT, restorable()));
    let changed = store_draft(&fixture, &a, ItemDtoKind::Ticket, CHANGED_ID, "Mine");
    fixture.save_ticket(&a, CHANGED_ID, "Theirs");
    rows.push((changed, CHANGED, restorable()));
    // Its text is the item's own.
    let saved_base = base_of(&fixture, &a, SAVED_ID);
    let saved_text = saved_base.fields.clone();
    let saved = store(&fixture, &root, SAVED_ID, Some(saved_base), saved_text);
    rows.push((saved, ALREADY_SAVED, restorable()));
    let mut unverified_base = base_of(&fixture, &a, UNVERIFIED_ID);
    unverified_base.observation = None;
    let unverified = store(
        &fixture,
        &root,
        UNVERIFIED_ID,
        Some(unverified_base),
        ticket_fields("Unverified", "Mine"),
    );
    rows.push((unverified, UNVERIFIED, restorable()));
    let closed = store_draft(&fixture, &a, ItemDtoKind::Ticket, CLOSED_ID, "Mine");
    rows.push((closed, TICKET_CLOSED, REFUSED.to_vec()));
    let gone = store_draft(&fixture, &a, ItemDtoKind::Ticket, GONE_ID, "Mine");
    rows.push((gone, ITEM_NOT_FOUND, REFUSED.to_vec()));
    let broken = store_draft(&fixture, &a, ItemDtoKind::Ticket, BROKEN_ID, "Mine");
    rows.push((broken, NONCONFORMING, REFUSED.to_vec()));
    let busy = store_draft(&fixture, &b, ItemDtoKind::Ticket, BUSY_ID, "Mine");
    rows.push((busy, READ_FAILED, [&REFUSED[..], &["retry"]].concat()));
    let loose = store_draft(&fixture, &c, ItemDtoKind::Ticket, LOOSE_ID, "Mine");
    rows.push((loose, REPOSITORY_UNAVAILABLE, REFUSED.to_vec()));
    let orphan = store_target(
        &fixture,
        DraftTarget {
            repository: root.clone(),
            item_kind: ItemKind::Ticket,
            item_id: item_id(CURRENT_ID),
            comment: Some(CommentTarget {
                comment_id: item_id(ORPHAN_COMMENT),
                parent_id: Some(item_id(ABSENT_PARENT)),
            }),
        },
        None,
        DraftFields::Comment {
            body: "A reply".to_owned(),
        },
    );
    rows.push((orphan, PARENT_NOT_FOUND, REFUSED.to_vec()));
    let reply = store_target(
        &fixture,
        DraftTarget {
            repository: root.clone(),
            item_kind: ItemKind::Ticket,
            item_id: item_id(CURRENT_ID),
            comment: Some(CommentTarget {
                comment_id: item_id(NEW_COMMENT),
                parent_id: None,
            }),
        },
        None,
        DraftFields::Comment {
            body: "A comment".to_owned(),
        },
    );
    rows.push((reply, CURRENT, REFUSED.to_vec()));

    // The item gone in a commit; the item made unreadable by its ID; the
    // repository no longer registered.
    fixture.commit_removal(&a, &ticket_path(GONE_ID));
    fixture.refresh(&a);
    fixture.write_non_utf8(&a, &ticket_path(BROKEN_ID));
    remove_registration(fixture.service(), &c);

    let (newer, _) = stray(&fixture, Some(NEWER));
    let (damaged, _) = stray(&fixture, Some(b"not a draft"));
    let (folder, _) = stray(&fixture, None);
    let mut unreadable = vec![
        (newer, "saved by a newer Manyhands"),
        (damaged, "damaged"),
        (folder, "not a file"),
    ];
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let (closed_off, path) = stray(&fixture, Some(b"{}"));
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
        unreadable.push((closed_off, "not accessible"));
    }

    // The registry's cache held, as a refresh writing it would: every row's
    // reads stay busy through their retries, and each offers Retry. Let go,
    // Retry reads each row but the busy one again.
    let shell = Shell::open(fixture, cx);
    let lease = RepositoryService::hold_lease_for_testing(
        &b,
        shell.fixture.data_dir(),
        LeaseKind::CacheWrite,
    )
    .unwrap();
    shell.keys("secondary-shift-d", cx);
    for _ in 0..6 {
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(1));
        shell.run(cx);
    }
    for (id, _, _) in &rows {
        assert_eq!(state(&shell, *id, cx).as_deref(), Some(READ_FAILED));
    }
    drop(lease);
    // Tall enough that every row's Retry is on screen, where a click
    // reaches it.
    shell
        .visual(cx)
        .simulate_resize(gpui_kit::size(gpui_kit::px(1440.), gpui_kit::px(6000.)));
    for (id, _, _) in rows.iter().filter(|(id, _, _)| *id != busy) {
        shell.click(row_id(*id, "retry"), cx);
    }

    let mut wrong = Vec::new();
    for (id, label, offered) in &rows {
        let shown = state(&shell, *id, cx).unwrap_or_default();
        let actions = actions(&shell, *id, cx);
        if !shown.starts_with(label) || &actions != offered {
            wrong.push(format!("{label}: read {shown:?} offering {actions:?}"));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
    assert_eq!(
        shell.label(row_id(current, "kind"), cx).as_deref(),
        Some(TICKET_KIND)
    );
    assert_eq!(
        shell.label(row_id(current, "title"), cx).as_deref(),
        Some("Current")
    );
    assert_eq!(
        shell.label(row_id(current, "repository"), cx).as_deref(),
        Some(root.as_str())
    );
    assert!(shell.present(row_id(current, "saved"), cx));
    assert_eq!(
        shell.label(row_id(reply, "kind"), cx),
        Some(format!("{COMMENT_ON} Current"))
    );
    assert_eq!(
        shell.label(row_id(reply, "note"), cx).as_deref(),
        Some(COMMENT_LATER)
    );
    for (id, reason) in unreadable {
        assert_eq!(
            state(&shell, id, cx),
            Some(format!("{CANNOT_READ}: {reason}"))
        );
        assert_eq!(actions(&shell, id, cx), ["export-raw", "discard"]);
    }
}

/// While its item is read, a row says so and offers Inspect, Export, Discard.
#[gpui_kit::test]
fn a_row_being_checked_says_so(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = with_ticket(&fixture, "a", ONE, "One");
    let id = store_draft(&fixture, &a, ItemDtoKind::Ticket, ONE, "Mine");
    let shell = Shell::open(fixture, cx);
    let _held = shell.backend.hold_read(ReadKind::Item);
    shell.keys("secondary-shift-d", cx);
    assert_eq!(state(&shell, id, cx).as_deref(), Some(CHECKING));
    assert_eq!(actions(&shell, id, cx), REFUSED.to_vec());
}

/// A draft restored in this window reads so and offers Show, which shows
/// and focuses its tab; open in no other window, nothing reads as such.
#[gpui_kit::test]
fn a_draft_restored_here_offers_show_which_focuses_its_tab(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = with_ticket(&fixture, "a", ONE, "One");
    fixture.commit_ticket(&a, TWO, "Two", "");
    fixture.refresh(&a);
    let id = store_draft(&fixture, &a, ItemDtoKind::Ticket, ONE, "Mine");
    let shell = Shell::open(fixture, cx);
    open(&shell, ticket(&a, ONE), cx);
    shell.click("draft-restore-button", cx);
    open(&shell, ticket(&a, TWO), cx);

    shell.keys("secondary-shift-d", cx);
    assert_eq!(state(&shell, id, cx).as_deref(), Some(RESTORED_HERE));
    assert_eq!(actions(&shell, id, cx), [&["show"][..], &REFUSED].concat());

    shell.click(row_id(id, "show"), cx);
    assert!(!drafts_shown(&shell, cx));
    let opened = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.opened(cx));
    assert_eq!(opened, Some(ticket(&a, ONE)));
    assert!(shell.frame(cx, |window, _| {
        window.find(("tab", 0usize)).focused() == Some(true)
    }));
}

#[gpui_kit::test]
fn an_empty_store_says_no_drafts_are_kept(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    shell.keys("secondary-shift-d", cx);
    assert_eq!(shell.label("drafts-empty", cx).as_deref(), Some(NO_DRAFTS));
}

/// Carry note: a listing that failed reads the store's own fixed reason.
#[gpui_kit::test]
fn a_failed_listing_reads_its_fixed_reason(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    shell.keys("secondary-shift-d", cx);
    let drafts = registry(&shell, cx);
    let gate = shell.backend.hold_read(ReadKind::Drafts);
    drafts.update(cx, |registry, cx| {
        let stamp = registry.reload_listing(cx);
        let failed = Err(DraftError {
            kind: DraftErrorKind::Access,
        });
        assert!(registry.apply_listing(stamp, failed));
        cx.notify();
    });
    shell.run(cx);
    assert_eq!(
        shell.label("drafts-reason", cx).as_deref(),
        Some("Access to the drafts folder was denied.")
    );
    assert!(!shell.present("drafts-empty", cx));
    drop(gate);
}

/// A row's item read held past a new listing never lands on it: the
/// newer read's state stays. The earlier read's answer, delivered under
/// the stamp it was asked with, is kept out by that stamp; under the
/// current one the same answer lands.
#[gpui_kit::test]
fn a_late_item_read_never_lands_after_the_listing_reloads(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = with_ticket(&fixture, "a", ONE, "One");
    let id = store_draft(&fixture, &a, ItemDtoKind::Ticket, ONE, "Mine");
    let shell = Shell::open(fixture, cx);
    let first = shell.backend.hold_read(ReadKind::Item);
    let second = shell.backend.hold_read(ReadKind::Item);
    shell.keys("secondary-shift-d", cx);
    assert_eq!(state(&shell, id, cx).as_deref(), Some(CHECKING));

    let drafts = registry(&shell, cx);
    drafts.update(cx, |registry, cx| {
        registry.reload_listing(cx);
    });
    shell.run(cx);
    drop(second);
    shell.run(cx);
    assert_eq!(state(&shell, id, cx).as_deref(), Some(CURRENT));

    let page = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.drafts_page().clone());
    let asked = |generation| Stamp {
        scope: ViewScope::DraftRow(id),
        generation,
    };
    // Shown, then rebuilt by the new listing: the first read was asked
    // under generation 1, the second under 2.
    page.update(cx, |page, cx| {
        assert!(!page.generations.is_current(&asked(1)));
        assert!(page.generations.is_current(&asked(2)));
        page.landed(id, &asked(1), TargetState::ItemNotFound, None, cx);
    });
    shell.run(cx);
    assert_eq!(state(&shell, id, cx).as_deref(), Some(CURRENT));
    page.update(cx, |page, cx| {
        page.landed(id, &asked(2), TargetState::ItemNotFound, None, cx);
    });
    shell.run(cx);
    assert_eq!(state(&shell, id, cx).as_deref(), Some(ITEM_NOT_FOUND));
    drop(first);
}

/// A file a newer Manyhands wrote is listed, not rewritten: its bytes are
/// the same after the page listed it and after the app started again.
#[gpui_kit::test]
fn a_newer_format_file_is_left_byte_identical(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let (id, path) = stray(&fixture, Some(NEWER));
    let shell = Shell::open(fixture, cx);
    shell.keys("secondary-shift-d", cx);
    assert_eq!(
        state(&shell, id, cx),
        Some(format!("{CANNOT_READ}: saved by a newer Manyhands"))
    );
    assert_eq!(std::fs::read(&path).unwrap(), NEWER);

    let Shell { fixture, .. } = shell;
    let shell = Shell::open(fixture, cx);
    shell.keys("secondary-shift-d", cx);
    assert!(shell.present(row_id(id, "row"), cx));
    assert_eq!(std::fs::read(&path).unwrap(), NEWER);
}
