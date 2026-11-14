//! Sharing & keys' key rows (Wave 04 Cycle 04 design C2.1; decision 2):
//! each row's controls send what today's sent, through the same
//! confirmation (contract 6), and after a key is removed or its file
//! deleted, focus goes to a key by its ID, never by the row it was in
//! (Part G item 4), and only while Settings is open (R-47; A6 F17). Focus
//! is read after two draws (Ruling 2). The removals are pressed by
//! keyboard, as a removed control's focus is what is given back; a pointer
//! press of a Kit button moves no focus.

use gpui_kit::{ElementId, Role, TestAppContext, test::TestWindowExt};
use manyhands::repository::{RepositoryService, SharedKeyId};

use super::super::{KeysPage, SELECTED, UNREGISTER, use_key};
use super::{
    keys, page,
    support::{Rows, holds, holds_handle, press, settle, tab_and_enter, where_focus},
    until_count,
};
use crate::desktop::{
    services::{JobKind, requests::Intent},
    test_support::fixtures::{Fixture, select_key_through, unregister_key_through},
    view::harness::Shell,
};

/// `made`, generated and in use; `spare`, generated; `mine`, imported.
/// Settings is open on Sharing & keys.
fn three_keys(cx: &mut TestAppContext) -> (Shell, [SharedKeyId; 3]) {
    let fixture = Fixture::new();
    let made = fixture.generate_key("made");
    let spare = fixture.generate_key("spare");
    let mine = fixture.import_key("mine");
    select_key_through(fixture.service(), made);
    let shell = keys(fixture, cx);
    assert_eq!(shell.rows("key-label", cx), ["made", "spare", "mine"]);
    (shell, [made, spare, mine])
}

/// What pressing a row's control does.
enum Does {
    /// One request, which `pick` picks, and no dialog.
    Sends(fn(&Intent, &[SharedKeyId; 3]) -> bool),
    /// Nothing at all.
    Nothing,
    /// The `KeyDelete` confirmation of the key at that index, and no
    /// request yet.
    Confirms(usize),
}

/// P12 (C2.1; R-17, R-38; contract 6): the radio, Stop using this key, the
/// trash and Delete key file… send `KeySelect`, `KeyClear` and `KeyRemove`,
/// and open the `KeyDelete` confirmation, as Select, Clear selection,
/// Unregister and Delete key did. The radio of the key in use sends
/// nothing. Delete key file… is on generated keys only, In use on the key
/// in use only, and Stop using this key is drawn only while a key is in
/// use. The radio says which key is in use, as a radio button named for its
/// key.
#[gpui_kit::test]
async fn key_rows_send_what_todays_controls_sent(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let mut rows = Rows::new("P12");
    let table: [(&str, ElementId, Does); 5] = [
        (
            "the radio of a key not in use",
            ("key-radio", 1usize).into(),
            Does::Sends(|intent, ids| matches!(intent, Intent::KeySelect { id } if *id == ids[1])),
        ),
        (
            "the radio of the key in use",
            ("key-radio", 0usize).into(),
            Does::Nothing,
        ),
        (
            "Stop using this key",
            "keys-stop".into(),
            Does::Sends(|intent, _| matches!(intent, Intent::KeyClear)),
        ),
        (
            "the trash",
            ("key-remove", 2usize).into(),
            Does::Sends(|intent, ids| matches!(intent, Intent::KeyRemove { id } if *id == ids[2])),
        ),
        (
            "Delete key file…",
            ("key-delete-file", 1usize).into(),
            Does::Confirms(1),
        ),
    ];
    for (row, control, does) in table {
        let (shell, ids) = three_keys(cx);
        let before = shell.requests(cx, |_| true).len();
        if !press(&shell, &mut rows, row, control, cx) {
            continue;
        }
        let sent = shell.requests(cx, |_| true).len() - before;
        let dialog = shell.present("confirm-dialog", cx);
        match does {
            Does::Sends(pick) => {
                let picked = shell.requests(cx, |request| pick(&request.intent, &ids));
                rows.fact(
                    row,
                    sent == 1 && picked.len() == 1 && !dialog,
                    format!(
                        "{sent} sent, {} of its intent, a dialog {dialog}",
                        picked.len()
                    ),
                );
                if let Some(id) = picked.first() {
                    shell.until_ended(*id, cx).await;
                }
            }
            Does::Nothing => rows.fact(
                row,
                sent == 0 && !dialog,
                format!("{sent} sent, a dialog {dialog}"),
            ),
            Does::Confirms(ix) => {
                let command = shell.label("confirm-command", cx);
                let wanted = Intent::KeyDelete { id: ids[ix] }.label();
                rows.fact(
                    row,
                    sent == 0 && dialog && command.as_deref() == Some(wanted),
                    format!("{sent} sent, the dialog's command {command:?}"),
                );
                shell.keys("escape", cx);
            }
        }
    }

    let (shell, _) = three_keys(cx);
    let found: Vec<bool> = (0..3usize)
        .map(|ix| shell.present(("key-delete-file", ix), cx))
        .collect();
    rows.fact(
        "Delete key file… on generated keys only",
        found == [true, true, false],
        format!("{found:?}"),
    );
    let found: Vec<Option<String>> = (0..3usize)
        .map(|ix| shell.label(("key-selected", ix), cx))
        .collect();
    rows.fact(
        "In use on the key in use only",
        found == [Some(SELECTED.to_owned()), None, None],
        format!("{found:?}"),
    );
    for (ix, label) in ["made", "spare", "mine"].into_iter().enumerate() {
        let found = shell.frame(cx, |window, _| {
            window.try_find(("key-radio", ix)).map(|radio| {
                (
                    radio.role(),
                    radio.checked(),
                    radio.label().map(str::to_owned),
                )
            })
        });
        let wanted = Some((Some(Role::RadioButton), Some(ix == 0), Some(use_key(label))));
        rows.fact(
            &format!("the radio of {label}"),
            found == wanted,
            format!("{found:?}"),
        );
        let found = shell.label(("key-remove", ix), cx);
        rows.fact(
            &format!("the trash of {label}"),
            found.as_deref() == Some(UNREGISTER),
            format!("{found:?}"),
        );
    }
    let fixture = Fixture::new();
    fixture.generate_key("idle");
    let shell = keys(fixture, cx);
    rows.fact(
        "no key in use: Stop using this key",
        !shell.present("keys-stop", cx),
        "it is drawn",
    );
    rows.done();
}

/// Settings open on Sharing & keys over the keys `labels`, generated, in
/// that order.
fn generated(labels: &[&str], cx: &mut TestAppContext) -> (Shell, Vec<SharedKeyId>) {
    let fixture = Fixture::new();
    let ids = labels
        .iter()
        .map(|label| fixture.generate_key(label))
        .collect();
    let shell = keys(fixture, cx);
    (shell, ids)
}

fn labels(shell: &Shell, cx: &mut TestAppContext) -> Vec<String> {
    let page: gpui_kit::Entity<KeysPage> = page(shell, cx);
    page.read_with(cx, |page, _| page.key_labels())
}

/// P24 (Part G item 4; decision 12; C2.1; R-47; A6 F17): a key removed by
/// its trash, or its file deleted through Delete key file…'s confirmation,
/// gives focus to the next key's radio by key ID, even when the list's
/// order changes on the read that follows; else to the previous key's;
/// else to Make a new key. Focus on a row's button stays with its key when
/// a row above it goes. A removal that lands after Settings closed moves
/// no focus.
#[gpui_kit::test]
async fn removing_a_key_returns_focus_by_key(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let mut rows = Rows::new("P24");

    // Another actor unregisters `first` while `middle`'s removal is held:
    // the read after it lists `last` alone, at row 0 where it was at row 2.
    let row = "the middle of three removed, the list's order changed on the read";
    let (shell, ids) = generated(&["first", "middle", "last"], cx);
    let held = shell.backend.hold_job(JobKind::KeyRemove);
    if tab_and_enter(&shell, &mut rows, row, ("key-remove", 1usize), cx) {
        held.wait_reached();
        let other = RepositoryService::open_at(shell.fixture.data_dir()).unwrap();
        unregister_key_through(&other, ids[0]);
        held.release();
        until_count(&shell, 1, cx).await;
        let listed = labels(&shell, cx);
        rows.fact(
            &format!("{row}: listed"),
            listed == ["last"],
            format!("{listed:?}"),
        );
        rows.fact(
            row,
            holds(&shell, ("key-radio", 0usize), cx),
            where_focus(&shell, cx),
        );
    } else {
        held.release();
    }

    let row = "the middle of three's file deleted, confirmed by Enter";
    let (shell, _) = generated(&["first", "middle", "last"], cx);
    if tab_and_enter(&shell, &mut rows, row, ("key-delete-file", 1usize), cx) {
        rows.fact(
            &format!("{row}: the confirmation"),
            shell.present("confirm-ok", cx),
            "no confirmation",
        );
        shell.keys("enter", cx);
        until_count(&shell, 2, cx).await;
        rows.fact(
            row,
            holds(&shell, ("key-radio", 1usize), cx),
            where_focus(&shell, cx),
        );
    }

    let row = "the last of two removed";
    let (shell, _) = generated(&["first", "last"], cx);
    if tab_and_enter(&shell, &mut rows, row, ("key-remove", 1usize), cx) {
        until_count(&shell, 1, cx).await;
        rows.fact(
            row,
            holds(&shell, ("key-radio", 0usize), cx),
            where_focus(&shell, cx),
        );
    }

    let row = "the only key removed";
    let (shell, _) = generated(&["only"], cx);
    if tab_and_enter(&shell, &mut rows, row, ("key-remove", 0usize), cx) {
        until_count(&shell, 0, cx).await;
        rows.fact(row, holds(&shell, "keys-make", cx), where_focus(&shell, cx));
    }

    // A Kit button keeps its focus by its element ID: keyed by key ID, it
    // stays with its key when a row above it goes on a read, and is not
    // handed to the key that takes its row.
    let row = "focus on a row's button, a row above it gone on a read";
    let (shell, ids) = generated(&["first", "middle", "last", "after"], cx);
    if shell.tab_reaches(("key-copy", 2usize), 60, cx) {
        let other = RepositoryService::open_at(shell.fixture.data_dir()).unwrap();
        unregister_key_through(&other, ids[0]);
        let keys_page = page(&shell, cx);
        keys_page.update(cx, |page, cx| page.opened(cx));
        until_count(&shell, 3, cx).await;
        rows.fact(
            row,
            holds(&shell, ("key-copy", 1usize), cx),
            where_focus(&shell, cx),
        );
    } else {
        rows.fact(row, false, "Tab never reached last's Copy public key");
    }

    // F17: Escape while the removal is held; it lands after the close.
    let row = "F17 a removal landing after Settings closed";
    let fixture = Fixture::new();
    fixture.generate_key("first");
    fixture.generate_key("middle");
    let shell = Shell::open(fixture, cx);
    let origin = shell
        .frame(cx, |window, cx| window.focused(cx))
        .expect("something holds focus");
    shell.keys("secondary-,", cx);
    press(&shell, &mut rows, row, ("settings-page", 1usize), cx);
    let held = shell.backend.hold_job(JobKind::KeyRemove);
    if tab_and_enter(&shell, &mut rows, row, ("key-remove", 1usize), cx) {
        held.wait_reached();
        shell.keys("escape", cx);
        settle(&shell, cx);
        rows.fact(
            &format!("{row}: the close gave focus back"),
            holds_handle(&shell, &origin, cx),
            where_focus(&shell, cx),
        );
        held.release();
        until_count(&shell, 1, cx).await;
        rows.fact(
            row,
            holds_handle(&shell, &origin, cx),
            where_focus(&shell, cx),
        );
    } else {
        held.release();
    }
    rows.done();
}
