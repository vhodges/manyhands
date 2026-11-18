//! The in-flight record's problem is shown once in Activity (D6 design B3,
//! R-31; review D1 of Task 1: Task 7 consumes `take_report`). A record the
//! store could not read is set aside, and the person hears of it once, with
//! Dismiss; a window activation that follows says nothing more.

use gpui_kit::TestAppContext;

use crate::desktop::{
    model::activity::{copy::DISMISS, rows::RowKey},
    services::inflight::Report,
    test_support::fixtures::Fixture,
    view::{
        activity_support::{act, offered, row, rows},
        harness::Shell,
    },
};

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
    let note = RowKey::Note(Report::Unreadable);
    assert!(row(&shell, &note, cx).is_none(), "nothing was read yet");

    let activate = |cx: &mut TestAppContext| {
        let services = services.clone();
        cx.update(|cx| services.window_activated(cx));
        shell.run(cx);
    };
    activate(cx);
    assert!(row(&shell, &note, cx).is_some(), "the person is told");
    activate(cx);
    let notes = rows(&shell, cx)
        .iter()
        .filter(|row| matches!(row.row.key, RowKey::Note(_)))
        .count();
    assert_eq!(notes, 1, "once");
    // The record was set aside, not deleted.
    let aside = std::fs::read_dir(&directory)
        .unwrap()
        .filter(|file| {
            let name = file.as_ref().unwrap().file_name();
            name.to_string_lossy().starts_with("record.unreadable-")
        })
        .count();
    assert_eq!(aside, 1);

    assert!(offered(&shell, &note, DISMISS, cx).is_some_and(|offer| offer.enabled));
    act(&shell, &note, DISMISS, cx);
    assert!(row(&shell, &note, cx).is_none());
    // Whatever the scheduler publishes next, the record says nothing more.
    activate(cx);
    cx.update(|cx| services.repositories_listed(&[], cx));
    shell.run(cx);
    assert!(row(&shell, &note, cx).is_none(), "for the session");
}
