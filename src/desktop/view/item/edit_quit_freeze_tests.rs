//! Quitting asks about unsaved text that no draft holds: from the entry to
//! `request_quit`, while that ask is open and `QuitPhase` is still `Running`
//! (Fact 16), the polling scheduler starts nothing, and Cancel lets it go on
//! (D6 Task 10, R-33). The helpers are `edit_tests`'s.

use super::*;
use crate::desktop::view::quit::{QuitPhase, support::Rig};

#[cfg(unix)]
#[gpui_kit::test]
async fn quit_freezes_scheduler_for_the_unkept_ask(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    // Crash recovery off: the drafts folder is a link, so no draft holds
    // the text typed.
    let fixture = Fixture::new();
    let elsewhere = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(elsewhere.path(), fixture.data_dir().join("drafts")).unwrap();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Unkept", "");
    let rig = Rig::over(fixture, std::slice::from_ref(&root), cx);
    rig.backend.answer_poll(rig.completed());
    rig.list(cx);
    rig.until_applied(&rig.roots[0], cx).await;
    assert!(rig.armed(cx), "a timer waits for the next poll");
    let polls = rig.polls();

    let shell = &rig.shell;
    open(shell, ticket(&root, TICKET), cx);
    shell.click("item-edit", cx);
    shell.click("recovery-off-continue", cx);
    let host = until_editing(shell, cx).await;
    type_at_end(shell, &host, "x", cx);
    assert_eq!(
        attached(shell, &root, TICKET, cx),
        None,
        "no draft holds it"
    );

    assert!(!rig.request_quit(cx));
    assert!(shell.present("edits-not-kept-dialog", cx), "it asks first");
    assert_eq!(
        rig.phase(cx),
        QuitPhase::Running,
        "the ask opens in Running"
    );
    rig.assert_frozen(polls, cx).await;

    rig.backend.answer_poll(rig.completed());
    shell.click("edits-not-kept-cancel", cx);
    assert_eq!(rig.phase(cx), QuitPhase::Running, "it stays");
    rig.assert_resumed(polls, cx).await;
}
