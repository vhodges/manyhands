//! P21, `browse_says_why_beside_its_button`, its Add rows (Wave 04 Cycle
//! 05a design B6 item 1; Part G items 1 and 2): the note under Add's
//! Browse… says "Choosing a folder…" while the chooser is open, why Browse…
//! is unavailable once the platform failed (Browse… then disabled), and,
//! after a choice that is not text, that it cannot be used, Browse… still
//! enabled. Create's rows are Task 6's.

use gpui_kit::TestAppContext;

use super::{settings, support::Table};
use crate::desktop::{
    test_support::fixtures::Fixture,
    view::{
        add_repository::{BROWSE_UNAVAILABLE, CHOOSING},
        harness::Shell,
    },
};

/// Add's note under Browse…, if one is drawn.
fn note(shell: &Shell, cx: &mut TestAppContext) -> Option<String> {
    shell.label("add-browse-note", cx)
}

pub(super) fn rows(table: &mut Table, cx: &mut TestAppContext) {
    let shell = settings(Fixture::new(), cx);
    table.fact(
        "No note before Browse…",
        note(&shell, cx).is_none(),
        "no note",
    );

    shell.click("add-browse", cx);
    let found = note(&shell, cx);
    table.fact(
        "While the chooser is open",
        cx.did_prompt_for_paths() && found.as_deref() == Some(CHOOSING),
        &format!("the chooser open, the note {CHOOSING:?} (it is {found:?})"),
    );
    if cx.did_prompt_for_paths() {
        cx.simulate_path_prompt_response(|_| None);
        shell.run(cx);
    }
    table.fact("After a dismissal", note(&shell, cx).is_none(), "no note");

    #[cfg(unix)]
    not_text(table, &shell, cx);

    super::support::browse_failed(&shell, cx);
    let found = note(&shell, cx);
    table.fact(
        "Unavailable",
        found.as_deref() == Some(BROWSE_UNAVAILABLE),
        &format!("the note {BROWSE_UNAVAILABLE:?} (it is {found:?})"),
    );
    shell.click("add-browse", cx);
    let found = note(&shell, cx);
    table.fact(
        "Unavailable, Browse… pressed",
        !cx.did_prompt_for_paths() && found.as_deref() == Some(BROWSE_UNAVAILABLE),
        &format!("Browse… disabled, no chooser (the note is {found:?})"),
    );
}

/// A choice that is not text: the note says so, the field is unchanged and
/// Browse… stays enabled.
#[cfg(unix)]
fn not_text(table: &mut Table, shell: &Shell, cx: &mut TestAppContext) {
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt, path::PathBuf};

    use crate::desktop::view::add_repository::BROWSE_NOT_TEXT;

    super::set(shell, "add-path", "/typed/by/hand", cx);
    shell.click("add-browse", cx);
    if cx.did_prompt_for_paths() {
        let chosen = PathBuf::from(OsStr::from_bytes(b"/not-\xff-text"));
        cx.simulate_path_prompt_response(move |_| Some(vec![chosen]));
        shell.run(cx);
    }
    let found = note(shell, cx);
    table.fact(
        "A choice that is not text",
        found.as_deref() == Some(BROWSE_NOT_TEXT)
            && super::value(shell, "add-path", cx) == "/typed/by/hand",
        &format!("the note {BROWSE_NOT_TEXT:?}, the field unchanged (the note is {found:?})"),
    );
    shell.click("add-browse", cx);
    table.fact(
        "A choice that is not text, Browse… still enabled",
        cx.did_prompt_for_paths(),
        "the chooser open again",
    );
    if cx.did_prompt_for_paths() {
        cx.simulate_path_prompt_response(|_| None);
        shell.run(cx);
    }
}
