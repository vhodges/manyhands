//! The in-flight record's problem is shown once in Activity (D6 design B3,
//! R-31; review D1 of Task 1: Task 7 consumes `take_report`). A record the
//! store could not read is set aside, and the person hears of it once, with
//! Dismiss; a window activation that follows says nothing more.

use gpui_kit::TestAppContext;

use crate::desktop::{
    model::operations::record_note_words, services::inflight::Report,
    test_support::fixtures::Fixture, view::harness::Shell,
};

const NOTE: &str = "record-note";
const DISMISS: &str = "record-note-dismiss";

/// A record that is not JSON is set aside when the lock is taken at the
/// first activation. Activity then names it, once: another activation, with
/// the lock held already, adds no second entry, and Dismiss hides it.
#[gpui_kit::test]
fn an_unreadable_record_is_reported_once_in_activity(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let shell = Shell::open(fixture, cx);
    let directory = shell.fixture.data_dir().join("inflight");
    std::fs::write(directory.join("record.json"), b"not a record").unwrap();
    let services = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().clone())
        .with_polling();
    assert!(!shell.present((NOTE, 0usize), cx), "nothing was read yet");

    let activate = |cx: &mut TestAppContext| {
        let services = services.clone();
        cx.update(|cx| services.window_activated(cx));
        shell.run(cx);
    };
    activate(cx);
    assert_eq!(
        shell.label((NOTE, 0usize), cx).as_deref(),
        Some(record_note_words(Report::Unreadable))
    );
    assert!(!record_note_words(Report::Unreadable).is_empty());
    activate(cx);
    assert!(!shell.present((NOTE, 1usize), cx), "once");
    // The record was set aside, not deleted.
    let aside = std::fs::read_dir(&directory)
        .unwrap()
        .filter(|file| {
            let name = file.as_ref().unwrap().file_name();
            name.to_string_lossy().starts_with("record.unreadable-")
        })
        .count();
    assert_eq!(aside, 1);

    shell.click((DISMISS, 0usize), cx);
    assert!(!shell.present((NOTE, 0usize), cx));
    // Whatever the scheduler publishes next, the record says nothing more.
    activate(cx);
    cx.update(|cx| services.repositories_listed(&[], cx));
    shell.run(cx);
    assert!(!shell.present((NOTE, 0usize), cx), "for the session");
}
