//! Needs attention in the operation area (D6 Task 8, design C3, W-6): the
//! entry each key, host or repository gets, the action it offers, what the
//! action asks of the service, and that each is reachable by pointer at the
//! default window and by keyboard. The reasons reach the scheduler as the
//! transport gives them. The entries are the log's Hold rows, read and
//! chosen through the seam; the click and keyboard tests keep the drawn
//! controls until Task 9. No test reads fonts, colours or layout.

use gpui_kit::TestAppContext;
use manyhands::repository::transport::{HostKeyIdentity, SshAuthority, SshTransportErrorKind};

use super::attention_support::Rig;
use crate::desktop::{
    model::{
        activity::{
            rows::{Kind, RowKey},
            states::hold_key,
        },
        polling::{
            Authority, Clock, Reason,
            attention::{
                APPROVE_HOST, CHOOSE_A_KEY, OPEN_REMOTE_SETTINGS, RESUME_CHECKING, UNLOCK,
            },
        },
        stamp::SettingsPage,
    },
    services::scheduler::{Heard, Moment, Scope},
    test_support::polling::transport_error,
    view::{
        activity_support::{
            act, act_unsettled, button_focus, drawn, index_of, offered, row, rows, to_row,
        },
        settings::Shown,
    },
};

/// The log's row of the hold `reason`.
fn hold(reason: &Reason) -> RowKey {
    RowKey::Hold(hold_key(reason))
}

/// How many holds the log lists.
fn holds(rig: &Rig, cx: &mut TestAppContext) -> usize {
    rows(&rig.shell, cx)
        .iter()
        .filter(|row| matches!(row.row.key, RowKey::Hold(_)))
        .count()
}

/// Asserts the hold `reason` is listed and offers `label`, enabled.
fn offers(rig: &Rig, reason: &Reason, label: &str, cx: &mut TestAppContext) {
    let offer = offered(&rig.shell, &hold(reason), label, cx)
        .unwrap_or_else(|| panic!("{reason:?} offers {label}"));
    assert!(offer.enabled, "{label} is enabled");
}

pub(in crate::desktop::view) fn shown(rig: &Rig, cx: &mut TestAppContext) -> Shown {
    rig.shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.settings_shown(cx))
}

fn host() -> SshAuthority {
    SshAuthority {
        host: "example.invalid".to_owned(),
        port: 22,
    }
}

fn presented() -> SshTransportErrorKind {
    SshTransportErrorKind::HostApprovalRequired {
        presented: HostKeyIdentity {
            algorithm: "ssh-ed25519".to_owned(),
            sha256: "SHA256:x".to_owned(),
        },
    }
}

/// O-3, A13: a locked key met by a poll holds back every repository that
/// uses it as one entry, with no prompt, and the operation area offers
/// Unlock… on it. The poll asks the session through the real provider.
#[gpui_kit::test]
async fn attention_locked_key(cx: &mut TestAppContext) {
    let rig = Rig::new(&[("a", true), ("b", true)], cx);
    let key = rig.lock();
    rig.list(cx);
    // The repositories are listed in the order of their roots.
    let mut roots = rig.roots.clone();
    roots.sort();
    let held = [(Reason::KeyLocked(key.key_id), roots.clone())];
    rig.shell
        .until_drawn(cx, "both repositories held back", |_, cx| {
            rig.entries(cx) == held
        })
        .await;

    // No prompt, though the key is locked and the session is fresh.
    assert!(
        rig.shell
            .workspace
            .read_with(cx, |workspace, _| workspace.passphrase())
            .is_none()
    );
    let prompt = rig
        .shell
        .operations(cx)
        .update(cx, |operations, _| operations.take_prompt());
    assert!(prompt.is_none(), "a poll never prompts");

    // One entry, covering the key's repositories, with Unlock….
    let reason = Reason::KeyLocked(key.key_id);
    let held = row(&rig.shell, &hold(&reason), cx).expect("the key's hold is listed");
    assert_eq!(held.row.kind, Kind::Locked);
    assert_eq!(
        held.row.group.roots, roots,
        "it covers both repositories, in the order of their roots"
    );
    offers(&rig, &reason, UNLOCK, cx);
    assert_eq!(holds(&rig, cx), 1, "one entry per key");
}

/// A4, C3: a host answer holds its repository back as the host's entry,
/// with Approve host… and not Unlock…; opening it shows the Hosts page; the
/// host's approval clears it (R-11).
#[gpui_kit::test]
async fn attention_host(cx: &mut TestAppContext) {
    let rig = Rig::new(&[("a", true)], cx);
    let authority = Authority {
        host: host().host,
        port: host().port,
    };
    rig.backend.fail_poll(transport_error(
        &rig.roots[0],
        presented(),
        None,
        Some(&host()),
    ));
    rig.list(cx);
    rig.until_entries(1, cx).await;
    let reason = Reason::HostUnapproved(authority.clone());
    assert_eq!(rig.entries(cx), [(reason.clone(), rig.roots.clone())]);
    rig.shell.run(cx);
    offers(&rig, &reason, APPROVE_HOST, cx);
    assert!(
        offered(&rig.shell, &hold(&reason), UNLOCK, cx).is_none(),
        "Approve host…, not Unlock…"
    );

    act(&rig.shell, &hold(&reason), APPROVE_HOST, cx);
    assert_eq!(shown(&rig, cx), Shown::Settings(SettingsPage::Hosts));
    assert!(
        rig.shell.holds_focus("hosts-check", cx),
        "W-25 lands on Check host (Part D)"
    );

    rig.backend.answer_poll(poll_done(&rig));
    rig.services.scheduler().update(cx, |scheduler, cx| {
        scheduler.heard(
            vec![Heard::Reconfiguration(
                Moment::Finished,
                Scope::Host {
                    root: rig.roots[0].clone(),
                    authority,
                },
            )],
            cx,
        )
    });
    rig.until_entries(0, cx).await;
    rig.shell.run(cx);
    assert_eq!(holds(&rig, cx), 0, "the approval clears the entry");
    assert!(row(&rig.shell, &hold(&reason), cx).is_none());
}

fn poll_done(rig: &Rig) -> crate::desktop::services::poll_convert::Library {
    use manyhands::repository::PollCompletion;

    use crate::desktop::services::poll_convert::{Library, ReportParts};
    let now = rig.clock.utc();
    Library::Polled {
        parts: ReportParts {
            completion: PollCompletion::Completed,
            completed_at: now,
            next_eligible_at: Some(now + time::Duration::seconds(300)),
            targets: Vec::new(),
            outcome: None,
        },
        index_pending: false,
    }
}

/// C3: each reason offers its own action, and the action opens the place
/// that fixes it; Check now then follows the change, not before. A missing
/// key offers Choose a key…; a repository that cannot check offers Open
/// remote settings…. (Unlock… and Approve host… are the other tests'.)
#[gpui_kit::test]
async fn attention_actions_by_reason(cx: &mut TestAppContext) {
    use SshTransportErrorKind as Kind;
    let rig = Rig::new(&[("a", true), ("b", true)], cx);
    let key = rig.key();
    rig.backend.fail_poll(transport_error(
        &rig.roots[0],
        Kind::KeyMissing,
        Some(key),
        None,
    ));
    rig.backend.fail_poll(transport_error(
        &rig.roots[1],
        Kind::ConfigurationInvalid,
        None,
        None,
    ));
    rig.list(cx);
    rig.until_entries(2, cx).await;
    let reasons = [
        Reason::KeyProblem(key),
        Reason::ConfigurationRequired(rig.roots[1].clone()),
    ];
    assert_eq!(
        rig.entries(cx),
        [
            (reasons[0].clone(), vec![rig.roots[0].clone()]),
            (reasons[1].clone(), vec![rig.roots[1].clone()]),
        ]
    );
    rig.shell.run(cx);
    offers(&rig, &reasons[0], CHOOSE_A_KEY, cx);
    offers(&rig, &reasons[1], OPEN_REMOTE_SETTINGS, cx);

    // Each opens its page and sends nothing yet; Check now follows when the
    // change it was sent to make finishes, for its repository only. The
    // repositories the change clears poll again, answered at once. A key
    // change reaches every repository, so it goes last.
    for _ in 0..4 {
        rig.backend.answer_poll(poll_done(&rig));
    }
    // Each lands on the control Part D names: the add-remote Name, and the
    // key list, Make a new key with no key.
    let changes = [
        (
            1,
            Shown::Settings(SettingsPage::Remotes),
            "remote-add-name",
            Scope::Repository {
                root: rig.roots[1].clone(),
                gone: false,
            },
        ),
        (
            0,
            Shown::Settings(SettingsPage::Keys),
            "keys-make",
            Scope::Keys,
        ),
    ];
    for (n, (entry, page, control, scope)) in changes.into_iter().enumerate() {
        let (label, reason) = match entry {
            0 => (CHOOSE_A_KEY, &reasons[0]),
            _ => (OPEN_REMOTE_SETTINGS, &reasons[1]),
        };
        act(&rig.shell, &hold(reason), label, cx);
        assert_eq!(shown(&rig, cx), page);
        assert!(rig.shell.holds_focus(control, cx), "{label}: {control}");
        // Settings is a sheet: it closes before the next row's action.
        rig.shell.keys("escape", cx);
        assert_eq!(rig.checks(cx).len(), n, "nothing is sent before the change");
        rig.services.scheduler().update(cx, |scheduler, cx| {
            scheduler.heard(vec![Heard::Reconfiguration(Moment::Succeeded, scope)], cx)
        });
        rig.shell
            .until_drawn(cx, "the Check now", |_, cx| rig.checks(cx).len() == n + 1)
            .await;
        assert_eq!(rig.checks(cx)[n].to_str(), Some(rig.roots[entry].as_str()));
    }
}

/// The registry, open beside the service's own connection.
fn registry(rig: &Rig) -> rusqlite::Connection {
    let path = rig
        .shell
        .fixture
        .data_dir()
        .join(manyhands::repository::REGISTRY_FILE);
    rusqlite::Connection::open(path).unwrap()
}

/// Suspends the library's polling of the repository named `name` until
/// recovery, as a replaced cache leaves it (Fact 8): the one flag the
/// registry holds for it.
fn suspend(rig: &Rig, name: &str) {
    let changed = registry(rig)
        .execute(
            "UPDATE remote_polling_state SET recovery_suspended=1 WHERE repository_id IN \
             (SELECT id FROM repositories WHERE root_path LIKE ?1)",
            [format!("%/{name}")],
        )
        .unwrap();
    assert_eq!(changed, 1, "{name} has a polling row");
}

fn suspended(rig: &Rig, name: &str) -> bool {
    registry(rig)
        .query_row(
            "SELECT recovery_suspended FROM remote_polling_state WHERE repository_id IN \
             (SELECT id FROM repositories WHERE root_path LIKE ?1)",
            [format!("%/{name}")],
            |row| row.get::<_, i64>(0),
        )
        .unwrap()
        == 1
}

/// U-7, R-13, decision 10: a repository held until recovery is listed, with
/// Resume checking, which calls the library's
/// `resume_remote_polling_after_recovery` on a background task, not in the
/// click's own handler, and releases the hold: the entry goes and the
/// repository is polled again. Over a real service.
#[gpui_kit::test]
async fn attention_recovery_held(cx: &mut TestAppContext) {
    let rig = Rig::new(&[("a", true)], cx);
    suspend(&rig, "a");
    rig.list(cx);
    rig.until_entries(1, cx).await;
    let reason = Reason::RecoveryHeld(rig.roots[0].clone());
    assert_eq!(rig.entries(cx), [(reason.clone(), rig.roots.clone())]);
    rig.shell.run(cx);
    offers(&rig, &reason, RESUME_CHECKING, cx);
    assert!(
        rig.backend.polled_roots().is_empty(),
        "a held one is not polled"
    );

    // The click only starts the call: it is made off the UI thread.
    rig.backend.answer_poll(poll_done(&rig));
    act_unsettled(&rig.shell, &hold(&reason), RESUME_CHECKING, cx);
    assert!(rig.backend.resumed().is_empty(), "not inside the choice");
    rig.shell.run(cx);
    assert_eq!(rig.backend.resumed(), rig.roots);

    // The library's flag is released, and the repository polls.
    assert!(!suspended(&rig, "a"));
    rig.until_entries(0, cx).await;
    rig.shell
        .until_drawn(cx, "the poll", |_, _| {
            rig.backend.polled_roots() == rig.roots
        })
        .await;
}

/// A repository for each reason, held back at once: the five actions of the
/// operation area, in the order of the repositories.
pub(in crate::desktop::view) async fn every_reason(cx: &mut TestAppContext) -> Rig {
    use SshTransportErrorKind as Kind;
    let rig = Rig::new(
        &[
            ("a", true),
            ("b", true),
            ("c", true),
            ("d", true),
            ("e", true),
        ],
        cx,
    );
    let (locked, missing) = (rig.key(), rig.key());
    let r = &rig.roots;
    rig.backend.fail_poll(transport_error(
        &r[0],
        Kind::UnlockCancelled,
        Some(locked),
        None,
    ));
    rig.backend.fail_poll(transport_error(
        &r[1],
        Kind::KeyMissing,
        Some(missing),
        None,
    ));
    rig.backend
        .fail_poll(transport_error(&r[2], presented(), None, Some(&host())));
    rig.backend.fail_poll(transport_error(
        &r[3],
        Kind::ConfigurationInvalid,
        None,
        None,
    ));
    suspend(&rig, "e");
    rig.list(cx);
    rig.until_entries(5, cx).await;
    rig
}

/// The hold rows the list draws, in its order, each with its button's
/// label.
pub(in crate::desktop::view) fn hold_rows(
    rig: &Rig,
    cx: &mut TestAppContext,
) -> Vec<(RowKey, String)> {
    drawn(&rig.shell, cx)
        .into_iter()
        .filter(|row| matches!(row.row.key, RowKey::Hold(_)))
        .map(|row| {
            let label = row.row.button.map(|offer| offer.label).unwrap_or_default();
            (row.row.key, label)
        })
        .collect()
}

/// The row of the hold whose button reads `label`.
pub(in crate::desktop::view) fn hold_with(
    rig: &Rig,
    label: &str,
    cx: &mut TestAppContext,
) -> RowKey {
    hold_rows(rig, cx)
        .into_iter()
        .find(|(_, shown)| shown == label)
        .unwrap_or_else(|| panic!("no hold row offers {label}"))
        .0
}

/// W-6, decision 13: every action of the list is where a click reaches it,
/// and does what it says: five holds, one for each reason, each with its
/// button drawn on its row (the rows' one-line layout keeps each button
/// inside the list: P21, Task 10).
#[gpui_kit::test]
async fn attention_actions_hit(cx: &mut TestAppContext) {
    let rig = every_reason(cx).await;
    let labels = [
        UNLOCK,
        CHOOSE_A_KEY,
        APPROVE_HOST,
        OPEN_REMOTE_SETTINGS,
        RESUME_CHECKING,
    ];
    let listed: Vec<String> = hold_rows(&rig, cx)
        .into_iter()
        .map(|(_, label)| label)
        .collect();
    assert_eq!(listed.len(), labels.len(), "a row for each hold");
    for expected in labels {
        assert!(listed.iter().any(|label| label == expected), "{expected}");
    }
    let press = |label: &str, cx: &mut TestAppContext| {
        let at = index_of(&rig.shell, &hold_with(&rig, label, cx), cx);
        rig.shell.click(("activity-button", at), cx);
    };

    press(UNLOCK, cx);
    assert_eq!(rig.checks(cx), [std::path::PathBuf::from(&rig.roots[0])]);
    // Settings is a sheet, whose scrim would take the next row's click:
    // Escape closes it between them.
    press(CHOOSE_A_KEY, cx);
    assert_eq!(shown(&rig, cx), Shown::Settings(SettingsPage::Keys));
    assert!(rig.shell.holds_focus("keys-make", cx), "W-23, no key");
    rig.shell.keys("escape", cx);
    press(APPROVE_HOST, cx);
    assert_eq!(shown(&rig, cx), Shown::Settings(SettingsPage::Hosts));
    assert!(rig.shell.holds_focus("hosts-check", cx), "W-25");
    rig.shell.keys("escape", cx);
    press(OPEN_REMOTE_SETTINGS, cx);
    assert_eq!(shown(&rig, cx), Shown::Settings(SettingsPage::Remotes));
    assert!(rig.shell.holds_focus("remote-add-name", cx), "W-24");
    rig.shell.keys("escape", cx);
    press(RESUME_CHECKING, cx);
    assert_eq!(rig.backend.resumed(), [rig.roots[4].clone()]);
}

/// Every action is reachable by the arrows to its row and Tab to its
/// button, and Enter presses it.
#[gpui_kit::test]
async fn attention_actions_by_keyboard(cx: &mut TestAppContext) {
    let rig = every_reason(cx).await;
    let rows = hold_rows(&rig, cx);
    assert_eq!(rows.len(), 5, "a row for each hold");
    for (key, label) in &rows {
        to_row(&rig.shell, key, cx);
        rig.shell.keys("tab", cx);
        let button = button_focus(&rig.shell, cx);
        assert!(
            rig.shell.frame(cx, |window, _| button.is_focused(window)),
            "Tab reaches the button of {label}"
        );
    }
    // The last row reached is whichever the list draws last; the Resume
    // checking row's Enter resumes.
    let resume = hold_with(&rig, RESUME_CHECKING, cx);
    to_row(&rig.shell, &resume, cx);
    rig.shell.keys("tab", cx);
    rig.shell.press("enter", cx);
    assert_eq!(rig.backend.resumed(), [rig.roots[4].clone()]);
}
