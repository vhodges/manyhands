//! Part D's menu, state by state.

use super::*;

pub(super) type Shown = Vec<(MenuChoice, String, bool)>;

pub(super) fn shown(items: Vec<MenuItem>) -> Shown {
    items
        .into_iter()
        .map(|item| (item.choice, item.label, item.enabled))
        .collect()
}

pub(super) struct Menu {
    pub name: &'static str,
    pub set: Set,
    pub items: Shown,
}

fn sentence(text: &str) -> (MenuChoice, String, bool) {
    (MenuChoice::Sentence, text.to_owned(), false)
}

pub(super) fn entry(choice: MenuChoice, label: &str, enabled: bool) -> (MenuChoice, String, bool) {
    (choice, label.to_owned(), enabled)
}

/// The items every state ends with, when nothing is disabled.
fn tail(share: &str, share_enabled: bool, folder_enabled: bool, settings: bool) -> Shown {
    let mut items = vec![
        entry(MenuChoice::ShareMain, share, share_enabled),
        entry(MenuChoice::OpenFolder, OPEN_FOLDER, folder_enabled),
    ];
    if settings {
        items.push(entry(MenuChoice::Settings, SETTINGS, true));
    }
    items.push(entry(MenuChoice::Remove, REMOVE_FROM_MANYHANDS, true));
    items
}

fn menu(name: &'static str, set: Set, head: Shown, tail: Shown) -> Menu {
    Menu {
        name,
        set,
        items: head.into_iter().chain(tail).collect(),
    }
}

const UP_TO_DATE_LINE: &str = "Checked 7 minutes ago \u{b7} up to date";

pub(super) fn menus() -> Vec<Menu> {
    use MenuChoice::*;
    let unchanged = "Its files are unchanged. Retry, or remove it from Manyhands.";
    vec![
        menu(
            "up to date: Check now enabled",
            |s| s.checked(Vec::new(), Vec::new()),
            vec![sentence(UP_TO_DATE_LINE), entry(CheckNow, CHECK_NOW, true)],
            tail(SHARE_MAIN, true, true, true),
        ),
        menu(
            "no sentence while polling is off, and Check now disabled with no schedule",
            |s| {
                s.unscheduled(Some("origin"));
                s.polling = false;
            },
            vec![entry(CheckNow, CHECK_NOW, false)],
            tail(SHARE_MAIN, true, true, true),
        ),
        menu(
            "checking: Check now disabled",
            |s| s.checking_now = true,
            vec![
                sentence("Checking alpha for updates\u{2026}"),
                entry(CheckNow, CHECK_NOW, false),
            ],
            tail(SHARE_MAIN, true, true, true),
        ),
        menu(
            "paused: its action is Check now, not repeated",
            |s| s.schedule().policy.paused = true,
            vec![
                sentence(PAUSED_SENTENCE),
                entry(StateAction, CHECK_NOW, true),
            ],
            tail(SHARE_MAIN, true, true, true),
        ),
        menu(
            "can't reach: Retry now",
            |s| s.schedule().last = Some(attempt(unreachable())),
            vec![
                sentence("Can't reach alpha. Your work is safe."),
                entry(StateAction, RETRY_NOW, true),
            ],
            tail(SHARE_MAIN, true, true, true),
        ),
        menu(
            "can't check, stored: Retry now",
            |s| s.stored = Some(PollingOutcome::ConfigurationRequired),
            vec![sentence("Can't check"), entry(StateAction, RETRY_NOW, true)],
            tail(SHARE_MAIN, true, true, true),
        ),
        menu(
            "stored key outcome: Check now",
            |s| s.stored = Some(PollingOutcome::SelectedKeyUnavailable),
            vec![
                sentence("Choose a key"),
                entry(StateAction, CHECK_NOW, true),
            ],
            tail(SHARE_MAIN, true, true, true),
        ),
        menu(
            "stored host outcome: Check now",
            |s| s.stored = Some(PollingOutcome::HostApprovalRequired),
            vec![
                sentence("Needs host approval"),
                entry(StateAction, CHECK_NOW, true),
            ],
            tail(SHARE_MAIN, true, true, true),
        ),
        menu(
            "locked, live: Unlock… then Check now",
            |s| s.schedule().phase = attention(Reason::KeyLocked(key())),
            vec![
                sentence(LOCKED_SENTENCE),
                entry(StateAction, attention::UNLOCK, true),
                entry(CheckNow, CHECK_NOW, true),
            ],
            tail(SHARE_MAIN, true, true, true),
        ),
        menu(
            "locked, live, with no unlock target: Unlock… disabled",
            |s| {
                s.schedule().phase = attention(Reason::KeyLocked(key()));
                s.unlock_target = false;
            },
            vec![
                sentence(LOCKED_SENTENCE),
                entry(StateAction, attention::UNLOCK, false),
                entry(CheckNow, CHECK_NOW, true),
            ],
            tail(SHARE_MAIN, true, true, true),
        ),
        menu(
            "locked, stored: Check now",
            |s| s.stored = Some(PollingOutcome::UnlockRequired),
            vec![
                sentence(LOCKED_SENTENCE),
                entry(StateAction, CHECK_NOW, true),
            ],
            tail(SHARE_MAIN, true, true, true),
        ),
        menu(
            "needs attention, a reason: its action, then Check now",
            |s| s.schedule().phase = attention(Reason::KeyProblem(key())),
            vec![
                sentence("Choose a key"),
                entry(StateAction, attention::CHOOSE_A_KEY, true),
                entry(CheckNow, CHECK_NOW, true),
            ],
            tail(SHARE_MAIN, true, true, true),
        ),
        menu(
            "held: Resume checking, and Check now disabled",
            |s| s.schedule().policy.recovery_suspended = true,
            vec![
                sentence("Checking is held until recovery"),
                entry(StateAction, attention::RESUME_CHECKING, true),
                entry(CheckNow, CHECK_NOW, false),
            ],
            tail(SHARE_MAIN, true, true, true),
        ),
        menu(
            "unreadable: Retry rereads, Open folder stays enabled",
            |s| {
                s.schedule = None;
                s.unplaced = Some(Unplaced::Unreadable);
            },
            vec![
                sentence("Can't check alpha for updates. Retry."),
                entry(StateAction, RETRY, true),
                entry(CheckNow, CHECK_NOW, false),
            ],
            tail(SHARE_MAIN, true, true, true),
        ),
        menu(
            "can't open from a resolution: Share main and Open folder disabled, Check now by its shape",
            |s| s.unavailable(),
            vec![
                sentence(&format!(
                    "{} {unchanged}",
                    ResultCode::RepositoryNotRegistered.message()
                )),
                entry(StateAction, RETRY, true),
                entry(CheckNow, CHECK_NOW, true),
            ],
            tail(SHARE_MAIN, false, false, true),
        ),
        menu(
            "can't open, the kept row: the same",
            |s| s.kept = true,
            vec![
                sentence(&format!(
                    "alpha is no longer registered with Manyhands. {unchanged}"
                )),
                entry(StateAction, RETRY, true),
                entry(CheckNow, CHECK_NOW, true),
            ],
            tail(SHARE_MAIN, false, false, true),
        ),
        menu(
            "busy: Retry, and sharing and the folder stay enabled",
            |s| s.resolution(Resolution::Busy),
            vec![
                sentence("Manyhands was busy opening alpha. Its files are unchanged. Retry."),
                entry(StateAction, RETRY, true),
                entry(CheckNow, CHECK_NOW, true),
            ],
            tail(SHARE_MAIN, true, true, true),
        ),
        menu(
            "needs fixing: Settings… is its action, not repeated",
            |s| s.invalid(ConfigurationState::Missing, Vec::new()),
            vec![
                sentence("Its configuration needs fixing:"),
                entry(StateAction, SETTINGS, true),
                entry(CheckNow, CHECK_NOW, true),
            ],
            tail(SHARE_MAIN, true, true, false),
        ),
        menu(
            "this computer: Set up sharing…, and Share main… with no remote",
            |s| s.unscheduled(None),
            vec![
                sentence("No shared copy, so changes stay on this computer."),
                entry(StateAction, SET_UP_SHARING, true),
                entry(CheckNow, CHECK_NOW, false),
            ],
            tail(SHARE_MAIN_SET_UP, true, true, true),
        ),
        menu(
            "not ssh: Share main disabled",
            |s| s.unscheduled(Some("origin")),
            vec![
                sentence("origin isn't an SSH shared copy, so Manyhands can't check or share it."),
                entry(StateAction, SET_UP_SHARING, true),
                entry(CheckNow, CHECK_NOW, false),
            ],
            tail(SHARE_MAIN, false, true, true),
        ),
        menu(
            "sharing: Sharing… disabled",
            |s| {
                s.sharing_main = true;
                s.checked(Vec::new(), Vec::new());
            },
            vec![sentence(UP_TO_DATE_LINE), entry(CheckNow, CHECK_NOW, true)],
            tail(SHARING, false, true, true),
        ),
    ]
}
