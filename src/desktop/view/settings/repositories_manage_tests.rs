//! The Repositories page's registered repositories over real
//! repositories: setting a repository's identity and initials, removing a
//! registration from the page and from an Unavailable sidebar entry, each
//! command's typed failure, Escape and a stale preview, and the
//! `repo.identity_set` recovery.

use std::{fs, path::Path};

use gpui_kit::TestAppContext;
use manyhands::{
    canonical::CONFIG_PATH,
    repository::{
        CommitIdentity, IdentityDto, InitialsChange, ItemDtoKind, Mutation, RepoIdentitySetInput,
        RepoRemoveInput, RepositoryService,
    },
    results::{Outcome, ResultCode},
};

use super::{
    manage::{INITIALS_CLEAR, INITIALS_SET, INITIALS_UNCHANGED, NO_IDENTITY},
    tests::{control_focus, finished, page, registered, set, settings, value},
};
use crate::desktop::{
    model::navigation::{OpenTarget, Resolution},
    services::requests::{DesktopRequestId, Intent},
    test_support::fixtures::{
        Fixture, identity, item_id, remove_registration, set_identity_through,
    },
    view::{
        confirm::{CONFIRM, FILES_REMAIN, PREPARE_AGAIN},
        harness::{Shell, key, wait_until},
    },
};

const ONE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FJ0";

fn grace() -> CommitIdentity {
    CommitIdentity {
        name: "Grace Example".to_owned(),
        email: "grace@example.invalid".to_owned(),
    }
}

fn identities(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| {
        matches!(request.intent, Intent::RepoIdentitySet { .. })
    })
}

fn removals(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| {
        matches!(request.intent, Intent::RepoRemove { .. })
    })
}

/// What the library reads as `root`'s identity: the oracle the page is
/// held to.
fn read_identity(shell: &Shell, root: &Path) -> IdentityDto {
    let service = shell.fixture.service();
    let resolved = service.resolve_repository(root).unwrap();
    service.repository_identity(&resolved).unwrap()
}

/// The name, email and initials `root`'s own Git configuration holds.
fn local(root: &Path) -> (Option<String>, Option<String>, Option<String>) {
    let config = git2::Repository::open(root).unwrap().config().unwrap();
    let local = config.open_level(git2::ConfigLevel::Local).unwrap();
    let get = |name: &str| local.get_string(name).ok();
    (
        get("user.name"),
        get("user.email"),
        get("manyhands.initials"),
    )
}

/// A repository `a` holding the ticket One, indexed.
fn with_ticket(fixture: &Fixture) -> std::path::PathBuf {
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, ONE, "One", "");
    fixture.refresh(&root);
    root
}

fn open_ticket(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    let target = OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Ticket,
        id: item_id(ONE),
    };
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
}

fn tabs(shell: &Shell, cx: &mut TestAppContext) -> usize {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.tab_keys().len())
}

fn selected(shell: &Shell, cx: &mut TestAppContext) -> Option<String> {
    shell.workspace.read_with(cx, |workspace, _| {
        workspace.navigation().selected().cloned()
    })
}

/// Chooses `root` among the page's registered repositories.
fn manage(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    let rows = shell.rows("managed-row", cx);
    let ix = rows
        .iter()
        .position(|row| *row == key(root))
        .unwrap_or_else(|| panic!("{} listed in {rows:?}", key(root)));
    shell.click(("managed-choose", ix), cx);
    assert_eq!(shell.label("managed-root", cx), Some(key(root)));
}

async fn until_page_lists(shell: &Shell, roots: Vec<String>, cx: &mut TestAppContext) {
    let page = page(shell, cx);
    wait_until(cx, &page, move |page, _| page.registrations() == roots).await;
}

/// The page lists each registration; choosing one shows the identity the
/// library reads for it.
#[gpui_kit::test]
fn the_page_lists_registrations_and_shows_the_chosen_ones_identity(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    let b = fixture.create_repository("b");
    let shell = settings(fixture, cx);
    assert_eq!(shell.rows("managed-row", cx), [key(&a), key(&b)]);
    assert!(!shell.present("identity-submit", cx));

    manage(&shell, &b, cx);
    let expected = read_identity(&shell, &b);
    assert_eq!(
        shell.label("identity-current-name", cx),
        Some(format!("Name: {}", expected.name.unwrap()))
    );
    assert_eq!(
        shell.label("identity-current-email", cx),
        Some(format!("Email: {}", expected.email.unwrap()))
    );
    assert_eq!(
        shell.label("identity-current-source", cx),
        Some(format!("From: {}", expected.source.as_str()))
    );
    // The form starts from what is there, its initials left unchanged.
    assert_eq!(value(&shell, "identity-name", cx), identity().name);
    assert_eq!(value(&shell, "identity-email", cx), identity().email);
    assert_eq!(
        shell.label("identity-initials-chosen", cx).as_deref(),
        Some(INITIALS_UNCHANGED)
    );
    assert!(!shell.present("identity-initials", cx));
}

/// Set identity with initials: the preview shows what will be written and
/// writes nothing; Confirm writes the name, email and initials to the
/// repository's own configuration, and the page shows the identity a
/// fresh read gives.
#[gpui_kit::test]
async fn setting_the_identity_with_initials_writes_it_and_shows_the_new_one(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let shell = settings(fixture, cx);
    manage(&shell, &root, cx);
    set(&shell, "identity-name", &grace().name, cx);
    set(&shell, "identity-email", &grace().email, cx);
    shell.click("identity-initials-set", cx);
    assert_eq!(
        shell.label("identity-initials-chosen", cx).as_deref(),
        Some(INITIALS_SET)
    );
    set(&shell, "identity-initials", "GE", cx);
    let before = local(&root);

    shell.click("identity-submit", cx);
    assert_eq!(shell.label("confirm-ok", cx).as_deref(), Some(CONFIRM));
    let details = shell.rows("confirm-detail", cx);
    for wanted in [
        format!("Name: {}", grace().name),
        format!("Email: {}", grace().email),
        "Initials: ge".to_owned(),
    ] {
        assert!(details.contains(&wanted), "{wanted} in {details:?}");
    }
    assert_eq!(local(&root), before, "the preview writes nothing");

    shell.click("confirm-ok", cx);
    let id = identities(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    assert_eq!(finished(&shell, id, cx), (Outcome::Success, ResultCode::Ok));
    assert_eq!(
        local(&root),
        (
            Some(grace().name),
            Some(grace().email),
            Some("ge".to_owned())
        )
    );
    let expected = read_identity(&shell, &root);
    let page = page(&shell, cx);
    let wanted = format!("Name: {}", grace().name);
    wait_until(cx, &page, move |page, _| {
        page.identity_shown().as_deref() == Some(wanted.as_str())
    })
    .await;
    assert_eq!(
        shell.label("identity-current-name", cx),
        Some(format!("Name: {}", grace().name))
    );
    assert_eq!(
        shell.label("identity-current-initials", cx),
        Some(format!(
            "Initials: {} ({})",
            expected.initials.unwrap(),
            expected.initials_source.as_str()
        ))
    );
    assert!(!shell.present("confirm-dialog", cx));
}

/// Clear initials maps to the library's `Clear`: the preview of a new
/// name says the repository will hold no initials.
#[gpui_kit::test]
fn clearing_initials_previews_none(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let shell = settings(fixture, cx);
    manage(&shell, &root, cx);
    set(&shell, "identity-name", &grace().name, cx);
    shell.click("identity-initials-clear", cx);
    assert_eq!(
        shell.label("identity-initials-chosen", cx).as_deref(),
        Some(INITIALS_CLEAR)
    );
    shell.click("identity-submit", cx);
    assert_eq!(shell.label("confirm-ok", cx).as_deref(), Some(CONFIRM));
    let details = shell.rows("confirm-detail", cx);
    assert!(
        !details.iter().any(|detail| detail.starts_with("Initials")),
        "{details:?}"
    );
}

/// Initials that do not fit are refused by the preview with the
/// library's typed answer and no Confirm. Nothing is written and no
/// request is made.
#[gpui_kit::test]
fn initials_that_do_not_fit_show_the_typed_failure(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let expected = fixture
        .service()
        .prepare(&Mutation::RepoIdentitySet(RepoIdentitySetInput {
            root: root.clone(),
            name: identity().name,
            email: identity().email,
            initials: InitialsChange::Set("x".to_owned()),
        }))
        .code;
    assert_ne!(expected, ResultCode::Ok);
    let shell = settings(fixture, cx);
    manage(&shell, &root, cx);
    shell.click("identity-initials-set", cx);
    set(&shell, "identity-initials", "x", cx);
    let before = local(&root);

    shell.click("identity-submit", cx);
    assert_eq!(
        shell.label("confirm-code", cx).as_deref(),
        Some(expected.as_str())
    );
    assert_eq!(
        shell.label("confirm-message", cx).as_deref(),
        Some(expected.message())
    );
    assert!(!shell.present("confirm-ok", cx));
    assert_eq!(local(&root), before);
    assert!(identities(&shell, cx).is_empty());
}

/// Escape on the identity preview: nothing is written, no request is
/// made, and focus goes back to Set identity.
#[gpui_kit::test]
fn escape_on_the_identity_preview_changes_nothing(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let shell = settings(fixture, cx);
    manage(&shell, &root, cx);
    set(&shell, "identity-name", &grace().name, cx);
    let before = local(&root);
    let submit = control_focus(&shell, "identity-submit", cx);
    shell
        .visual(cx)
        .update(|window, cx| window.focus(&submit, cx));
    shell.click("identity-submit", cx);
    assert!(shell.present("confirm-ok", cx));

    shell.keys("escape", cx);
    assert!(!shell.present("confirm-dialog", cx));
    assert_eq!(local(&root), before);
    assert!(shell.requests(cx, |_| true).is_empty());
    assert!(shell.frame(cx, |window, _| submit.is_focused(window)));
}

/// A stale identity preview: a second actor sets another identity
/// between the preview and Confirm. The answer is shown with Prepare
/// again, and the second actor's identity stands.
#[gpui_kit::test]
async fn an_identity_changed_since_its_preview_offers_prepare_again(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let shell = settings(fixture, cx);
    manage(&shell, &root, cx);
    set(&shell, "identity-name", &grace().name, cx);
    set(&shell, "identity-email", &grace().email, cx);
    shell.click("identity-submit", cx);
    assert!(shell.present("confirm-ok", cx));
    let other = RepositoryService::open_at(shell.fixture.data_dir()).unwrap();
    let late = CommitIdentity {
        name: "Late Example".to_owned(),
        email: "late@example.invalid".to_owned(),
    };
    set_identity_through(&other, &root, &late);

    shell.click("confirm-ok", cx);
    let id = identities(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    let (_, code) = finished(&shell, id, cx);
    assert_eq!(code, ResultCode::ExternalChange);
    assert_eq!(
        shell.label("confirm-prepare-again", cx).as_deref(),
        Some(PREPARE_AGAIN)
    );
    let (name, email, _) = local(&root);
    assert_eq!((name, email), (Some(late.name), Some(late.email)));
}

/// Remove an accessible repository from the page: the dialog says local
/// files remain; Confirm takes the registration out, leaves every file,
/// reloads the lists, lets the selection go and closes its tabs.
#[gpui_kit::test]
async fn removing_an_accessible_repository_keeps_its_files_and_closes_its_tabs(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = with_ticket(&fixture);
    let shell = Shell::open(fixture, cx);
    shell.select(&root, cx);
    open_ticket(&shell, &root, cx);
    assert_eq!(tabs(&shell, cx), 1);
    shell.keys("secondary-,", cx);
    manage(&shell, &root, cx);

    shell.click("remove-submit", cx);
    assert_eq!(shell.label("confirm-ok", cx).as_deref(), Some(CONFIRM));
    assert_eq!(
        shell.label("confirm-note", cx).as_deref(),
        Some(FILES_REMAIN)
    );
    let details = shell.rows("confirm-detail", cx);
    assert!(
        details.contains(&"Files changed: no".to_owned()),
        "{details:?}"
    );
    assert!(!shell.rows("confirm-effect", cx).is_empty());
    assert_eq!(
        registered(&shell),
        [key(&root)],
        "the preview removes nothing"
    );

    shell.click("confirm-ok", cx);
    let id = removals(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    assert_eq!(finished(&shell, id, cx), (Outcome::Success, ResultCode::Ok));
    assert!(registered(&shell).is_empty());
    assert!(root.join(CONFIG_PATH).is_file());
    assert!(root.join(".git").is_dir());
    assert!(root.join(".manyhands").is_dir());
    until_page_lists(&shell, Vec::new(), cx).await;
    wait_until(cx, &shell.workspace, |workspace, _| {
        workspace
            .navigation()
            .repositories()
            .rows()
            .is_some_and(|rows| rows.is_empty())
    })
    .await;
    assert_eq!(tabs(&shell, cx), 0);
    assert_eq!(selected(&shell, cx), None);
    assert!(!shell.present("managed-root", cx));
    assert!(!shell.present("confirm-dialog", cx));
}

/// A registered root that is now a file, not a folder, is refused by the
/// preview with the library's typed answer and no Confirm. The
/// registration stays, and so do its tabs.
#[gpui_kit::test]
fn removing_a_root_that_is_now_a_file_shows_the_typed_failure(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = with_ticket(&fixture);
    let shell = Shell::open(fixture, cx);
    open_ticket(&shell, &root, cx);
    fs::rename(&root, root.with_file_name("a-away")).unwrap();
    fs::write(&root, "not a folder\n").unwrap();
    let expected = shell
        .fixture
        .service()
        .prepare(&Mutation::RepoRemove(RepoRemoveInput {
            root: root.clone(),
        }))
        .code;
    assert_ne!(expected, ResultCode::Ok);
    shell.keys("secondary-,", cx);
    manage(&shell, &root, cx);

    shell.click("remove-submit", cx);
    assert_eq!(
        shell.label("confirm-code", cx).as_deref(),
        Some(expected.as_str())
    );
    assert_eq!(
        shell.label("confirm-message", cx).as_deref(),
        Some(expected.message())
    );
    assert!(!shell.present("confirm-ok", cx));
    assert_eq!(registered(&shell), [key(&root)]);
    assert!(removals(&shell, cx).is_empty());
    assert_eq!(tabs(&shell, cx), 1);
}

/// Escape on the removal preview: the registration and the tabs stay, no
/// request is made, and focus goes back to Remove.
#[gpui_kit::test]
fn escape_on_the_removal_preview_changes_nothing(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = with_ticket(&fixture);
    let shell = Shell::open(fixture, cx);
    open_ticket(&shell, &root, cx);
    shell.keys("secondary-,", cx);
    manage(&shell, &root, cx);
    let remove = control_focus(&shell, "remove-submit", cx);
    shell
        .visual(cx)
        .update(|window, cx| window.focus(&remove, cx));
    shell.click("remove-submit", cx);
    assert!(shell.present("confirm-ok", cx));

    shell.keys("escape", cx);
    assert!(!shell.present("confirm-dialog", cx));
    assert_eq!(registered(&shell), [key(&root)]);
    assert!(shell.requests(cx, |_| true).is_empty());
    assert_eq!(tabs(&shell, cx), 1);
    assert!(shell.frame(cx, |window, _| remove.is_focused(window)));
}

/// A stale removal preview: a second actor removes the registration
/// between the preview and Confirm. The answer, `external_change`, is
/// shown with Prepare again; the fresh preview finds the command's end
/// state already holds. The removal was not this request's, so its tabs
/// are left, as for any registration a second actor removes (Review
/// Focus 2), and the files stay.
#[gpui_kit::test]
async fn a_registration_removed_since_its_preview_offers_prepare_again(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = with_ticket(&fixture);
    let shell = Shell::open(fixture, cx);
    open_ticket(&shell, &root, cx);
    shell.keys("secondary-,", cx);
    manage(&shell, &root, cx);
    shell.click("remove-submit", cx);
    assert!(shell.present("confirm-ok", cx));
    let other = RepositoryService::open_at(shell.fixture.data_dir()).unwrap();
    remove_registration(&other, &root);

    shell.click("confirm-ok", cx);
    let id = removals(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    let (_, code) = finished(&shell, id, cx);
    assert_eq!(code, ResultCode::ExternalChange);
    assert_eq!(
        shell.label("confirm-prepare-again", cx).as_deref(),
        Some(PREPARE_AGAIN)
    );
    assert!(registered(&shell).is_empty());
    assert!(root.join(CONFIG_PATH).is_file());
    assert_eq!(tabs(&shell, cx), 1);

    // The fresh preview: nothing is left to remove.
    shell.click("confirm-prepare-again", cx);
    assert_eq!(
        shell.label("confirm-code", cx).as_deref(),
        Some(ResultCode::AlreadyApplied.as_str())
    );
    assert!(!shell.present("confirm-ok", cx));
    assert_eq!(tabs(&shell, cx), 1);
}

/// The Part C gate (`01M4R19P94`): a registration whose folder was
/// renamed away shows Unavailable; its Remove, reached by keyboard,
/// prepares the removal, and Confirm takes the registration out. Its
/// tabs close and the selection goes.
#[gpui_kit::test]
async fn removing_a_registration_whose_folder_was_renamed_away_succeeds(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = with_ticket(&fixture);
    let shell = Shell::open(fixture, cx);
    shell.select(&root, cx);
    open_ticket(&shell, &root, cx);
    assert_eq!(tabs(&shell, cx), 1);
    let away = root.with_file_name("a-away");
    fs::rename(&root, &away).unwrap();
    shell.select(&root, cx);
    let resolution = shell.workspace.read_with(cx, |workspace, _| {
        workspace
            .navigation()
            .selected_entry()
            .unwrap()
            .resolution
            .clone()
    });
    assert!(
        matches!(resolution, Resolution::Unavailable(_)),
        "{resolution:?}"
    );

    // By keyboard: the content region, then Tab until Remove holds focus.
    shell.keys("secondary-3", cx);
    let remove = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.remove_focus());
    let mut presses = 0;
    while !shell.frame(cx, |window, cx| remove.contains_focused(window, cx)) {
        presses += 1;
        assert!(presses < 60, "Tab never reached Remove");
        shell.keys("tab", cx);
    }
    shell.press("enter", cx);
    assert_eq!(shell.label("confirm-ok", cx).as_deref(), Some(CONFIRM));
    assert_eq!(
        shell.label("confirm-note", cx).as_deref(),
        Some(FILES_REMAIN)
    );
    shell.keys("enter", cx);
    let id = removals(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    assert_eq!(finished(&shell, id, cx), (Outcome::Success, ResultCode::Ok));
    assert!(registered(&shell).is_empty());
    assert!(away.join(CONFIG_PATH).is_file(), "the files are kept");
    wait_until(cx, &shell.workspace, |workspace, _| {
        workspace.tab_keys().is_empty() && workspace.navigation().selected().is_none()
    })
    .await;
    assert!(!shell.present("selection-unavailable", cx));
}

/// The `repo.identity_set` recovery's entry point opens the identity form
/// for its root, with focus in the name field. The model maps the action
/// to it (`operations_tests`).
#[gpui_kit::test]
fn the_identity_recovery_opens_the_form_for_its_root(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let shell = Shell::open(fixture, cx);
    let target = root.clone();
    shell.visual(cx).update(|window, cx| {
        shell.workspace.update(cx, |workspace, cx| {
            workspace.identity_in_settings(target, window, cx)
        })
    });
    shell.run(cx);
    assert!(shell.present("settings-repositories", cx));
    assert_eq!(shell.label("managed-root", cx), Some(key(&root)));
    assert!(shell.present("identity-submit", cx));
    let name = control_focus(&shell, "identity-name", cx);
    assert!(shell.frame(cx, |window, cx| name.contains_focused(window, cx)));
}

/// A repository Git finds no identity for says so.
#[gpui_kit::test]
fn a_repository_without_an_identity_says_so(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let config = git2::Repository::open(&root).unwrap().config().unwrap();
    let mut local = config.open_level(git2::ConfigLevel::Local).unwrap();
    local.remove("user.name").unwrap();
    local.remove("user.email").unwrap();
    let shell = settings(fixture, cx);
    manage(&shell, &root, cx);
    assert_eq!(
        shell.label("identity-current-name", cx).as_deref(),
        Some(NO_IDENTITY)
    );
}
