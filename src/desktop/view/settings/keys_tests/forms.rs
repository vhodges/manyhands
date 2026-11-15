//! Sharing & keys' forms and the typed passphrase (Wave 04 Cycle 04 design
//! B3 and C2.2): neither form shows at first, each opens from its button
//! and hides the other, the Generate form shows whenever the page is
//! retrying (R-54), and a typed passphrase lives only while Sharing & keys
//! is shown and its form is (R-13). Typed secrets would print through
//! `Debug`, so they are compared with `assert!`, and a row names no secret.

use std::path::PathBuf;

use gpui_kit::TestAppContext;
use manyhands::{repository::RequestId, results::ResultCode};

use super::super::RETRY_FIRST;
use super::{
    CONFIRMATION, PASSPHRASE, SENTINEL, keys, page,
    support::{Rows, holds, press, settle, where_focus},
    type_into, value,
};
use crate::desktop::{
    model::stamp::SettingsPage,
    services::{
        requests::{DesktopRequest, DesktopRequestId, Intent, Lane, NotSent, RequestState},
        worker::WorkerEvent,
    },
    test_support::fixtures::Fixture,
    view::{
        activity::tests::cases::{answered, fabricate},
        harness::{Shell, wait_until},
        settings::{SettingsRoute, Shown},
    },
};

/// Make a new key, then the passphrase and its confirmation typed into
/// the Generate form it shows. False, said in the row, when the key part
/// of Sharing & keys is not drawn.
fn typed(shell: &Shell, rows: &mut Rows, row: &str, cx: &mut TestAppContext) -> bool {
    if !shell.present("settings-keys", cx) {
        rows.fact(row, false, "the key part of Sharing & keys is not drawn");
        return false;
    }
    press(shell, rows, row, "keys-make", cx);
    type_into(shell, PASSPHRASE, SENTINEL, cx);
    type_into(shell, CONFIRMATION, SENTINEL, cx);
    let typed = kept(shell, cx);
    rows.fact(
        &format!("{row}: typed"),
        typed,
        "the passphrase could not be typed",
    );
    typed
}

/// Both masked inputs still hold what was typed.
fn kept(shell: &Shell, cx: &mut TestAppContext) -> bool {
    value(shell, PASSPHRASE, cx) == SENTINEL && value(shell, CONFIRMATION, cx) == SENTINEL
}

/// Both masked inputs are empty.
fn emptied(shell: &Shell, cx: &mut TestAppContext) -> bool {
    value(shell, PASSPHRASE, cx).is_empty() && value(shell, CONFIRMATION, cx).is_empty()
}

fn shown(shell: &Shell, cx: &mut TestAppContext) -> Shown {
    shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.settings_shown(cx))
}

/// Raises the way into Settings `route` as its trigger does.
fn route(shell: &Shell, route: SettingsRoute, cx: &mut TestAppContext) {
    let workspace = shell.workspace.clone();
    shell.visual(cx).update(|window, cx| {
        workspace.update(cx, |workspace, cx| {
            workspace.show_settings(route, None, window, cx)
        })
    });
    shell.run(cx);
}

/// Repository `a`, in an open window.
fn with_a_repository(cx: &mut TestAppContext) -> (Shell, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.refresh(&root);
    (Shell::open(fixture, cx), root)
}

/// P9 (B3, R-13; contract 9): a passphrase typed into the Generate form is
/// emptied when Settings closes, when the menu leaves Sharing & keys, and
/// when the Generate form hides by either button. Typed after a
/// `Hosts(path)` route, it is kept while Sharing & keys is shown, whichever
/// of its two parts the menu names (Ruling 4), the draws included.
#[gpui_kit::test]
fn a_typed_passphrase_lives_only_on_sharing_and_keys(cx: &mut TestAppContext) {
    let mut rows = Rows::new("P9");

    let row = "typed, then Settings closed";
    let shell = keys(Fixture::new(), cx);
    typed(&shell, &mut rows, row, cx);
    shell.keys("escape", cx);
    rows.fact(row, emptied(&shell, cx), "the passphrase is kept");

    let row = "typed, then the menu leaves Sharing & keys";
    let shell = keys(Fixture::new(), cx);
    typed(&shell, &mut rows, row, cx);
    shell.click(("settings-page", 0usize), cx);
    rows.fact(row, emptied(&shell, cx), "the passphrase is kept");

    let row = "typed after a Hosts(path) route";
    let (shell, root) = with_a_repository(cx);
    route(&shell, SettingsRoute::Hosts(root), cx);
    let page = shown(&shell, cx);
    rows.fact(
        &format!("{row}: the route's page"),
        page == Shown::Settings(SettingsPage::Hosts),
        format!("{page:?}"),
    );
    if typed(&shell, &mut rows, row, cx) {
        settle(&shell, cx);
        rows.fact(
            &format!("{row}, two draws later"),
            kept(&shell, cx),
            "the passphrase is emptied",
        );
        shell.click(("settings-page", 1usize), cx);
        settle(&shell, cx);
        rows.fact(
            &format!("{row}, then the Sharing & keys entry"),
            kept(&shell, cx),
            "the passphrase is emptied",
        );
        shell.keys("escape", cx);
        rows.fact(
            &format!("{row}, then Settings closed"),
            emptied(&shell, cx),
            "the passphrase is kept",
        );
    }

    let row = "typed, then Use a key I already have…";
    let shell = keys(Fixture::new(), cx);
    typed(&shell, &mut rows, row, cx);
    if press(&shell, &mut rows, row, "keys-use-existing", cx) {
        rows.fact(row, emptied(&shell, cx), "the passphrase is kept");
    }

    let row = "typed, then Make a new key again, hiding its form";
    let shell = keys(Fixture::new(), cx);
    typed(&shell, &mut rows, row, cx);
    if press(&shell, &mut rows, row, "keys-make", cx) {
        rows.fact(row, emptied(&shell, cx), "the passphrase is kept");
    }
    rows.done();
}

/// Which of the two forms are drawn: Generate, Import.
fn forms(shell: &Shell, cx: &mut TestAppContext) -> (bool, bool) {
    (
        shell.present("key-generate-heading", cx),
        shell.present("key-import-heading", cx),
    )
}

/// A protected generation that the operation area retried without its
/// passphrase: queued again, then not sent for it, as the worker answers.
fn not_sent_for_its_passphrase(shell: &Shell, cx: &mut TestAppContext) -> DesktopRequestId {
    let id = DesktopRequestId(6_000_001);
    let request = DesktopRequest {
        id,
        request_id: RequestId::new(),
        intent: Intent::KeyGenerate {
            label: "observed".to_owned(),
            protected: true,
        },
        confirmation: None,
        lane: Lane::User,
        attempt: 1,
        state: RequestState::Queued,
    };
    shell.operations(cx).update(cx, |operations, cx| {
        operations.apply(WorkerEvent::Queued(request));
        operations.apply(WorkerEvent::NotSent(id, NotSent::SecretRequired));
        cx.notify();
    });
    shell.run(cx);
    id
}

/// P13 (C2.2; R-18, R-54, R-57; W-26): neither form at first; each button
/// shows its form and hides the other; pressing it again hides it. A
/// route's retry (W-26) shows the Generate form with focus in its
/// passphrase, read after two draws (A6 F1); while the page retries, Use a
/// key I already have… is unavailable and says why, and Make a new key
/// leaves the form shown. A retry the page enters by itself, when the
/// operation area's Retry is not sent for its passphrase, shows the
/// Generate form too.
#[gpui_kit::test]
async fn the_key_forms_open_from_their_buttons(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let mut rows = Rows::new("P13");
    let shell = keys(Fixture::new(), cx);
    let found = forms(&shell, cx);
    rows.fact(
        "at first, neither form",
        found == (false, false),
        format!("{found:?}"),
    );
    for (row, button, wanted) in [
        ("Make a new key", "keys-make", (true, false)),
        (
            "then Use a key I already have…",
            "keys-use-existing",
            (false, true),
        ),
        ("then Make a new key", "keys-make", (true, false)),
        ("then Make a new key again", "keys-make", (false, false)),
        (
            "then Use a key I already have…",
            "keys-use-existing",
            (false, true),
        ),
        (
            "then Use a key I already have… again",
            "keys-use-existing",
            (false, false),
        ),
    ] {
        if press(&shell, &mut rows, row, button, cx) {
            let found = forms(&shell, cx);
            rows.fact(row, found == wanted, format!("{found:?}"));
        }
    }

    shell.keys("escape", cx);
    let retry = fabricate(
        &shell,
        1,
        Intent::KeyGenerate {
            label: "locked".to_owned(),
            protected: true,
        },
        answered("key generate", ResultCode::Busy),
        cx,
    );
    route(&shell, SettingsRoute::RetryGeneration(retry), cx);
    let row = "W-26, the forms hidden before";
    let found = forms(&shell, cx);
    rows.fact(row, found == (true, false), format!("{found:?}"));
    rows.fact(
        &format!("{row}: focus in the passphrase"),
        holds(&shell, PASSPHRASE, cx),
        where_focus(&shell, cx),
    );
    let reason = shell.label("keys-use-existing-reason", cx);
    rows.fact(
        "retrying: Use a key I already have… says why it is unavailable",
        reason.as_deref() == Some(RETRY_FIRST),
        format!("{reason:?}"),
    );
    for (row, button) in [
        ("retrying: Use a key I already have…", "keys-use-existing"),
        ("retrying: Make a new key", "keys-make"),
    ] {
        if press(&shell, &mut rows, row, button, cx) {
            let found = forms(&shell, cx);
            rows.fact(row, found == (true, false), format!("{found:?}"));
        }
    }
    let row = "the retry cancelled";
    if press(&shell, &mut rows, row, "key-generate-retry-cancel", cx) {
        let found = forms(&shell, cx);
        let reason = shell.present("keys-use-existing-reason", cx);
        rows.fact(
            row,
            found == (true, false) && !reason,
            format!("{found:?}, the reason drawn {reason}"),
        );
    }

    let row = "a retry the page enters by itself, the forms hidden before";
    press(&shell, &mut rows, row, "keys-make", cx);
    let found = forms(&shell, cx);
    rows.fact(
        &format!("{row}: hidden"),
        found == (false, false),
        format!("{found:?}"),
    );
    let id = not_sent_for_its_passphrase(&shell, cx);
    let keys_page = page(&shell, cx);
    wait_until(cx, &keys_page, move |page, _| page.retrying() == Some(id)).await;
    let found = forms(&shell, cx);
    rows.fact(row, found == (true, false), format!("{found:?}"));
    rows.done();
}
