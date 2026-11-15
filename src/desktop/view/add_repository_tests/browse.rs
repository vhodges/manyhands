//! P21, `browse_says_why_beside_its_button` (Wave 04 Cycle 05a design B6
//! item 1, B8 item 2; Part G items 1 and 2): the note under Browse…, in
//! each mode, says "Choosing a folder…" while the chooser is open, why
//! Browse… is unavailable once the platform failed (Browse… then
//! disabled), and, after a choice that is not text, that it cannot be
//! used, Browse… still enabled. Create's Browse… fills Location and checks
//! nothing.

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
    create_rows(table, cx);
}

/// Create's note under its Browse…, if one is drawn.
fn create_note(shell: &Shell, cx: &mut TestAppContext) -> Option<String> {
    shell.label("create-browse-note", cx)
}

/// P21's Create rows (B8 item 2): the same three notes under Create's
/// Browse…, which fills Location.
fn create_rows(table: &mut Table, cx: &mut TestAppContext) {
    let shell = settings(Fixture::new(), cx);
    let chosen = shell.fixture.parent();
    super::new_mode(&shell, cx);
    table.fact(
        "Create: no note before Browse…",
        create_note(&shell, cx).is_none(),
        "no note",
    );

    shell.click("create-browse", cx);
    let found = create_note(&shell, cx);
    table.fact(
        "Create: while the chooser is open",
        cx.did_prompt_for_paths() && found.as_deref() == Some(CHOOSING),
        &format!("the chooser open, the note {CHOOSING:?} (it is {found:?})"),
    );
    if cx.did_prompt_for_paths() {
        let answer = chosen.clone();
        cx.simulate_path_prompt_response(move |_| Some(vec![answer]));
        shell.run(cx);
    }
    table.fact(
        "Create: after a choice",
        create_note(&shell, cx).is_none()
            && super::value(&shell, "create-location", cx) == super::text(&chosen)
            && !shell.present("add-checks", cx),
        "no note, Location filled, nothing checked",
    );

    shell.click("create-browse", cx);
    if cx.did_prompt_for_paths() {
        cx.simulate_path_prompt_response(|_| None);
        shell.run(cx);
    }
    table.fact(
        "Create: after a dismissal",
        create_note(&shell, cx).is_none()
            && super::value(&shell, "create-location", cx) == super::text(&chosen),
        "no note, Location unchanged",
    );

    #[cfg(unix)]
    create_not_text(table, &shell, cx);

    super::support::browse_failed(&shell, cx);
    let found = create_note(&shell, cx);
    table.fact(
        "Create: unavailable",
        found.as_deref() == Some(BROWSE_UNAVAILABLE),
        &format!("the note {BROWSE_UNAVAILABLE:?} (it is {found:?})"),
    );
    shell.click("create-browse", cx);
    table.fact(
        "Create: unavailable, Browse… pressed",
        !cx.did_prompt_for_paths(),
        "Browse… disabled, no chooser",
    );
}

/// Create's choice that is not text: the note says so, Location is
/// unchanged and Browse… stays enabled.
#[cfg(unix)]
fn create_not_text(table: &mut Table, shell: &Shell, cx: &mut TestAppContext) {
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt, path::PathBuf};

    use crate::desktop::view::add_repository::BROWSE_NOT_TEXT;

    super::set(shell, "create-location", "/typed/by/hand", cx);
    shell.click("create-browse", cx);
    if cx.did_prompt_for_paths() {
        let chosen = PathBuf::from(OsStr::from_bytes(b"/not-\xff-text"));
        cx.simulate_path_prompt_response(move |_| Some(vec![chosen]));
        shell.run(cx);
    }
    let found = create_note(shell, cx);
    table.fact(
        "Create: a choice that is not text",
        found.as_deref() == Some(BROWSE_NOT_TEXT)
            && super::value(shell, "create-location", cx) == "/typed/by/hand",
        &format!("the note {BROWSE_NOT_TEXT:?}, Location unchanged (the note is {found:?})"),
    );
    shell.click("create-browse", cx);
    table.fact(
        "Create: a choice that is not text, Browse… still enabled",
        cx.did_prompt_for_paths(),
        "the chooser open again",
    );
    if cx.did_prompt_for_paths() {
        cx.simulate_path_prompt_response(|_| None);
        shell.run(cx);
    }
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
