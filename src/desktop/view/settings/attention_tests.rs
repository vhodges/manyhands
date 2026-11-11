//! The Checking for updates page's needs-attention rows (D6 Task 8, design
//! C2, C3): a repository held back for the person carries the action its
//! reason names, and the action does what the operation area's does. The
//! schedule is seeded through the model's own transitions; what an action
//! sends is read from the requests the service received.

use gpui_kit::TestAppContext;

use super::polling_support::Rig;
use crate::desktop::{
    model::{
        polling::{
            Answer, KeyId, NotEligible, TransportFailure,
            attention::{CHOOSE_A_KEY, RESUME_CHECKING, UNLOCK},
            status::HELD,
        },
        stamp::SettingsPage,
    },
    services::requests::Intent,
    view::settings::Shown,
};

const ACTION: &str = "polling-attention";

/// Tab presses enough to go round the whole window.
const TABS: usize = 60;

/// A locked key offers Unlock…, a missing one Choose a key… and a
/// repository held until recovery Resume checking, in the repository's row;
/// each does what it says: Unlock… sends Check now, Choose a key… shows the
/// Keys page, and Resume checking releases the library's hold, so the row
/// no longer says held. Each is reached by Tab.
#[gpui_kit::test]
async fn settings_polling_attention_actions(cx: &mut TestAppContext) {
    let rig = Rig::new(&[("a", true), ("b", true), ("c", true), ("d", true)], cx);
    for ix in 0..4 {
        rig.schedule(ix, cx);
    }
    let (locked, missing) = (KeyId::new(), KeyId::new());
    let key_locked = TransportFailure::KeyLocked { key: Some(locked) };
    rig.answer(0, Answer::Transport(key_locked), cx);
    rig.answer(
        1,
        Answer::Transport(TransportFailure::KeyProblem { key: Some(missing) }),
        cx,
    );
    rig.answer(2, Answer::NotEligible(NotEligible::RecoverySuspended), cx);
    rig.answer(3, Answer::Completed, cx);
    rig.open(4, cx).await;
    let (a, b, c, d) = (
        rig.row("a", cx),
        rig.row("b", cx),
        rig.row("c", cx),
        rig.row("d", cx),
    );
    let action = |ix: usize, cx: &mut TestAppContext| rig.shell.label((ACTION, ix), cx);
    assert_eq!(action(a, cx).as_deref(), Some(UNLOCK));
    assert_eq!(action(b, cx).as_deref(), Some(CHOOSE_A_KEY));
    assert_eq!(action(c, cx).as_deref(), Some(RESUME_CHECKING));
    assert_eq!(action(d, cx), None, "a repository that checks has none");
    assert_eq!(rig.status(c, cx).as_deref(), Some(HELD));

    // Unlock… sends Check now for the key's repository.
    rig.shell.click((ACTION, a), cx);
    let root = rig.roots[0].clone();
    let sent = rig.sent(
        cx,
        |intent| matches!(intent, Intent::PollOnce { root: asked } if *asked == root),
    );
    assert_eq!(sent.len(), 1, "Unlock… sent Check now");

    // Choose a key… shows the Keys page.
    rig.shell.click((ACTION, b), cx);
    assert_eq!(
        rig.shell
            .workspace
            .read_with(cx, |workspace, cx| workspace.settings_shown(cx)),
        Shown::Settings(SettingsPage::Keys)
    );

    // Resume checking is reached by Tab and pressed by Enter; the library's
    // hold is released, and the row no longer says held.
    rig.shell.keys("secondary-,", cx);
    rig.shell.click(("settings-page", 4usize), cx);
    assert!(rig.shell.tab_reaches((ACTION, c), TABS, cx));
    rig.shell.press("enter", cx);
    rig.shell
        .until_drawn(cx, "the hold to be released", |shell, cx| {
            shell.label(("polling-status", c), cx).as_deref() != Some(HELD)
        })
        .await;
}
