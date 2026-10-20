//! The Repositories page over real directories and repositories: Create
//! and Add existing through the confirmation dialog, their typed
//! refusals, a partial create that Retry completes under the same
//! request, one repository for two presses of Confirm, Browse, a control
//! below the fold, and the `repo.inspect` recovery.

use std::{
    fs,
    path::{Path, PathBuf},
};

use gpui_kit::{
    Entity, FocusHandle, TestAppContext, component::input::InputState, px, size,
    test::TestWindowExt,
};
use manyhands::{
    canonical::CONFIG_PATH,
    repository::{
        CommitIdentity, FailurePoint, Mutation, RepoCreateInput, RepoEnableInput, RepositoryService,
    },
    results::{Outcome, RecoveryActionKind, ResultCode},
};

use super::{
    ALREADY_ADDED, BROWSE_PROMPT, IDENTITY_REQUIRED, NEED_FOLDER, NOT_ADDED, RepositoriesPage,
};
use crate::desktop::{
    services::{
        JobKind,
        requests::{DesktopRequestId, Intent, Lane, RequestState},
    },
    test_support::fixtures::{Fixture, enable_through, identity},
    view::{
        confirm::{CONFIRM, PREPARE_AGAIN},
        harness::{Shell, key},
        operations::recovery_id,
    },
};

/// The window over `fixture`, with the Repositories page shown.
fn settings(fixture: Fixture, cx: &mut TestAppContext) -> Shell {
    let shell = Shell::open(fixture, cx);
    shell.keys("secondary-,", cx);
    assert!(shell.present("settings-repositories", cx));
    shell
}

fn page(shell: &Shell, cx: &mut TestAppContext) -> Entity<RepositoriesPage> {
    let host = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.settings.clone());
    host.read_with(cx, |host, _| host.repositories.clone())
        .expect("the Repositories page is made")
}

fn input(shell: &Shell, id: &str, cx: &mut TestAppContext) -> Entity<InputState> {
    let page = page(shell, cx);
    page.read_with(cx, |page, _| page.input(id))
}

fn value(shell: &Shell, id: &str, cx: &mut TestAppContext) -> String {
    let input = input(shell, id, cx);
    input.read_with(cx, |input, _| input.value().to_string())
}

/// Sets the field `id` to `text`, as typing it would.
fn set(shell: &Shell, id: &str, text: &str, cx: &mut TestAppContext) {
    let input = input(shell, id, cx);
    shell.visual(cx).update(|window, cx| {
        input.update(cx, |input, cx| input.set_value(text.to_owned(), window, cx));
    });
    shell.run(cx);
}

/// Focuses the field `id`.
fn focus_in(shell: &Shell, id: &str, cx: &mut TestAppContext) {
    let input = input(shell, id, cx);
    shell.visual(cx).update(|window, cx| {
        input.update(cx, |input, cx| input.focus(window, cx));
    });
    shell.run(cx);
}

/// Focuses the field `id` and presses Enter in it.
fn enter_in(shell: &Shell, id: &str, cx: &mut TestAppContext) {
    focus_in(shell, id, cx);
    shell.keys("enter", cx);
}

fn text(path: &Path) -> &str {
    path.to_str().unwrap()
}

fn entries(path: &Path) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    entries.sort();
    entries
}

fn registered(shell: &Shell) -> Vec<String> {
    shell
        .fixture
        .service()
        .list_repositories()
        .unwrap()
        .items
        .into_iter()
        .map(|summary| summary.root)
        .collect()
}

fn listed(shell: &Shell, cx: &mut TestAppContext) -> Vec<String> {
    shell.workspace.read_with(cx, |workspace, _| {
        workspace
            .navigation()
            .repositories()
            .rows()
            .map(|rows| rows.iter().map(|entry| entry.key().clone()).collect())
            .unwrap_or_default()
    })
}

fn of(shell: &Shell, cx: &mut TestAppContext, pick: fn(&Intent) -> bool) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| pick(&request.intent))
}

fn creates(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    of(shell, cx, |intent| {
        matches!(intent, Intent::RepoCreate { .. })
    })
}

fn enables(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    of(shell, cx, |intent| {
        matches!(intent, Intent::RepoEnable { .. })
    })
}

/// The background refreshes of `root` this session submitted.
fn background_refreshes(shell: &Shell, root: &Path, cx: &mut TestAppContext) -> usize {
    let root = root.to_owned();
    let operations = shell.operations(cx);
    operations.read_with(cx, |operations, _| {
        operations
            .requests()
            .iter()
            .filter(|request| {
                request.lane == Lane::Background
                    && matches!(&request.intent, Intent::IndexRefresh { root: r } if *r == root)
            })
            .count()
    })
}

fn finished(shell: &Shell, id: DesktopRequestId, cx: &mut TestAppContext) -> (Outcome, ResultCode) {
    let operations = shell.operations(cx);
    operations.read_with(cx, |operations, _| {
        match &operations.request(id).unwrap().state {
            RequestState::Finished(outcome, code, ..) => (*outcome, *code),
            _ => panic!("not finished"),
        }
    })
}

/// Waits until a background refresh of `root` has been submitted and has
/// ended, then until the list shows `root`.
async fn until_onboarded(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    let wanted = root.to_owned();
    shell
        .until(cx, move |operations| {
            operations.requests().iter().any(|request| {
                request.lane == Lane::Background
                    && matches!(&request.intent, Intent::IndexRefresh { root } if *root == wanted)
                    && matches!(request.state, RequestState::Finished(..))
            })
        })
        .await;
    let key = key(root);
    until_listed(shell, key, cx).await;
}

async fn until_listed(shell: &Shell, key: String, cx: &mut TestAppContext) {
    crate::desktop::view::harness::wait_until(cx, &shell.workspace, move |workspace, _| {
        workspace
            .navigation()
            .repositories()
            .rows()
            .is_some_and(|rows| rows.iter().any(|entry| *entry.key() == key))
    })
    .await;
}

/// The preview's detail lines.
fn details(shell: &Shell, cx: &mut TestAppContext) -> Vec<String> {
    shell.rows("confirm-detail", cx)
}

fn ada() -> CommitIdentity {
    identity()
}

/// Create in an empty directory, with identity and prefix: the preview
/// lists its effects and details and creates nothing, neither in the
/// directory nor in the registry. Confirm creates and registers it, the
/// list shows it, and one background refresh of it is submitted.
#[gpui_kit::test]
async fn create_in_an_empty_directory_previews_nothing_then_creates_and_refreshes(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.empty_directory("new");
    let shell = settings(fixture, cx);
    set(&shell, "create-path", text(&root), cx);
    set(&shell, "create-branch", "main", cx);
    set(&shell, "create-name", &ada().name, cx);
    set(&shell, "create-email", &ada().email, cx);
    set(&shell, "create-prefix", "nw", cx);

    shell.click("create-submit", cx);
    assert_eq!(shell.label("confirm-ok", cx).as_deref(), Some(CONFIRM));
    assert!(!shell.rows("confirm-effect", cx).is_empty());
    let details = details(&shell, cx);
    for wanted in [
        format!("Root: {}", text(&root)),
        "Primary branch: main".to_owned(),
        format!("Name: {}", ada().name),
        format!("Email: {}", ada().email),
        "Short code prefix: nw".to_owned(),
    ] {
        assert!(details.contains(&wanted), "{wanted} in {details:?}");
    }
    // The preview made nothing.
    assert!(entries(&root).is_empty(), "{:?}", entries(&root));
    assert!(registered(&shell).is_empty());
    assert!(creates(&shell, cx).is_empty());

    shell.click("confirm-ok", cx);
    let id = creates(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    assert_eq!(finished(&shell, id, cx), (Outcome::Success, ResultCode::Ok));
    assert!(root.join(CONFIG_PATH).is_file());
    assert_eq!(registered(&shell), [key(&root)]);
    until_onboarded(&shell, &root, cx).await;
    assert_eq!(listed(&shell, cx), [key(&root)]);
    assert_eq!(background_refreshes(&shell, &root, cx), 1);
    assert!(!shell.present("confirm-dialog", cx));
}

/// Add an existing repository Git has no identity for: the identity
/// fields appear once the folder is checked, the primary branch is the
/// one checked out, and success writes the identity to the repository's
/// own configuration.
#[gpui_kit::test]
async fn adding_a_repository_without_identity_asks_for_one_and_keeps_it(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.plain_repository("plain", None);
    let shell = settings(fixture, cx);
    assert!(!shell.present("add-name", cx));
    assert!(!shell.present("add-email", cx));

    set(&shell, "add-path", text(&root), cx);
    enter_in(&shell, "add-path", cx);
    assert_eq!(
        shell.label("add-inspection", cx).as_deref(),
        Some(NOT_ADDED)
    );
    assert_eq!(
        shell.label("add-identity-required", cx).as_deref(),
        Some(IDENTITY_REQUIRED)
    );
    assert!(shell.present("add-name", cx));
    assert!(shell.present("add-email", cx));
    assert_eq!(value(&shell, "add-branch", cx), "main");

    set(&shell, "add-name", &ada().name, cx);
    set(&shell, "add-email", &ada().email, cx);
    shell.click("add-submit", cx);
    assert_eq!(shell.label("confirm-ok", cx).as_deref(), Some(CONFIRM));
    assert!(registered(&shell).is_empty());
    shell.click("confirm-ok", cx);
    let id = enables(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    assert_eq!(finished(&shell, id, cx), (Outcome::Success, ResultCode::Ok));

    let config = git2::Repository::open(&root).unwrap().config().unwrap();
    let local = config.open_level(git2::ConfigLevel::Local).unwrap();
    assert_eq!(local.get_string("user.name").unwrap(), ada().name);
    assert_eq!(local.get_string("user.email").unwrap(), ada().email);
    assert_eq!(registered(&shell), [key(&root)]);
    until_onboarded(&shell, &root, cx).await;
    assert_eq!(background_refreshes(&shell, &root, cx), 1);

    // Checked again, it is registered.
    enter_in(&shell, "add-path", cx);
    assert_eq!(
        shell.label("add-inspection", cx).as_deref(),
        Some(ALREADY_ADDED)
    );
}

/// A repository Git finds an identity for shows no identity fields.
#[gpui_kit::test]
fn a_repository_with_an_identity_shows_no_identity_fields(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.plain_repository("plain", Some(ada()));
    let shell = settings(fixture, cx);
    set(&shell, "add-path", text(&root), cx);
    enter_in(&shell, "add-path", cx);
    assert_eq!(
        shell.label("add-inspection", cx).as_deref(),
        Some(NOT_ADDED)
    );
    assert!(!shell.present("add-identity-required", cx));
    assert!(!shell.present("add-name", cx));
}

/// Create in a directory that is not empty: the preview is refused with
/// the library's own typed answer, shown by its message and code, with
/// no Confirm. Nothing changed in the directory or the registry, and no
/// request was made.
#[gpui_kit::test]
fn create_in_a_non_empty_directory_shows_its_typed_failure(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.empty_directory("full");
    fs::write(root.join("notes.txt"), "kept\n").unwrap();
    let expected = fixture
        .service()
        .prepare(&Mutation::RepoCreate(RepoCreateInput {
            root: root.clone(),
            primary_branch: "main".to_owned(),
            identity: Some(ada()),
            slug_prefix: None,
        }))
        .code;
    assert_ne!(expected, ResultCode::Ok);
    let shell = settings(fixture, cx);
    set(&shell, "create-path", text(&root), cx);
    set(&shell, "create-name", &ada().name, cx);
    set(&shell, "create-email", &ada().email, cx);

    shell.click("create-submit", cx);
    assert_eq!(
        shell.label("confirm-code", cx).as_deref(),
        Some(expected.as_str())
    );
    assert_eq!(
        shell.label("confirm-message", cx).as_deref(),
        Some(expected.message())
    );
    assert!(!shell.present("confirm-ok", cx));
    assert_eq!(entries(&root), [root.join("notes.txt")]);
    assert!(registered(&shell).is_empty());
    assert!(shell.requests(cx, |_| true).is_empty());
}

/// Add a folder that is not a Git repository: checking it shows the
/// read's typed failure, and submitting it the preview's. Nothing is
/// made in the folder and nothing is registered.
#[gpui_kit::test]
fn adding_a_folder_that_is_not_a_repository_shows_its_typed_failure(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.empty_directory("loose");
    fs::write(root.join("notes.txt"), "kept\n").unwrap();
    let inspected = fixture
        .service()
        .inspect_repository(&root)
        .expect_err("not a repository")
        .code();
    let prepared = fixture
        .service()
        .prepare(&Mutation::RepoEnable(RepoEnableInput {
            root: root.clone(),
            primary_branch: "main".to_owned(),
            identity: None,
            slug_prefix: None,
        }))
        .code;
    assert_ne!(prepared, ResultCode::Ok);
    let shell = settings(fixture, cx);
    set(&shell, "add-path", text(&root), cx);
    enter_in(&shell, "add-path", cx);
    assert_eq!(
        shell.label("add-inspection", cx).as_deref(),
        Some(inspected.message())
    );
    assert_eq!(
        shell.label("add-inspection-code", cx).as_deref(),
        Some(inspected.as_str())
    );
    assert!(!shell.present("add-name", cx));

    set(&shell, "add-branch", "main", cx);
    shell.click("add-submit", cx);
    assert_eq!(
        shell.label("confirm-code", cx).as_deref(),
        Some(prepared.as_str())
    );
    assert_eq!(
        shell.label("confirm-message", cx).as_deref(),
        Some(prepared.message())
    );
    assert!(!shell.present("confirm-ok", cx));
    assert_eq!(entries(&root), [root.join("notes.txt")]);
    assert!(registered(&shell).is_empty());
    assert!(shell.requests(cx, |_| true).is_empty());
}

/// Submit with no folder says so and opens no dialog.
#[gpui_kit::test]
fn submitting_with_no_folder_says_so(cx: &mut TestAppContext) {
    let shell = settings(Fixture::new(), cx);
    shell.click("create-submit", cx);
    assert_eq!(
        shell.label("create-message", cx).as_deref(),
        Some(NEED_FOLDER)
    );
    assert!(!shell.present("confirm-dialog", cx));
    shell.click("add-submit", cx);
    assert_eq!(shell.label("add-message", cx).as_deref(), Some(NEED_FOLDER));
    assert!(!shell.present("confirm-dialog", cx));
}

/// The fields of a create in an empty `name`.
fn fill_create(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    set(shell, "create-path", text(root), cx);
    set(shell, "create-name", &ada().name, cx);
    set(shell, "create-email", &ada().email, cx);
}

/// Escape closes the preview with nothing made and no request, and focus
/// goes back to where it was.
#[gpui_kit::test]
fn escape_on_the_preview_makes_nothing(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.empty_directory("new");
    let shell = settings(fixture, cx);
    fill_create(&shell, &root, cx);
    let submit = control_focus(&shell, "create-submit", cx);
    shell
        .visual(cx)
        .update(|window, cx| window.focus(&submit, cx));
    shell.click("create-submit", cx);
    assert!(shell.present("confirm-ok", cx));

    shell.keys("escape", cx);
    assert!(!shell.present("confirm-dialog", cx));
    assert!(entries(&root).is_empty());
    assert!(registered(&shell).is_empty());
    assert!(shell.requests(cx, |_| true).is_empty());
    assert!(shell.frame(cx, |window, _| submit.is_focused(window)));
}

/// A stale preview: the directory gains a file between the preview and
/// Confirm. The answer is shown with Prepare again, and nothing was
/// created.
#[gpui_kit::test]
async fn a_create_whose_directory_changed_since_its_preview_offers_prepare_again(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.empty_directory("new");
    let shell = settings(fixture, cx);
    fill_create(&shell, &root, cx);
    shell.click("create-submit", cx);
    assert!(shell.present("confirm-ok", cx));
    fs::write(root.join("late.txt"), "late\n").unwrap();

    shell.click("confirm-ok", cx);
    let id = creates(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    let (_, code) = finished(&shell, id, cx);
    assert_ne!(code, ResultCode::Ok);
    assert_eq!(
        shell.label("confirm-prepare-again", cx).as_deref(),
        Some(PREPARE_AGAIN),
        "{}",
        code.as_str()
    );
    assert_eq!(entries(&root), [root.join("late.txt")]);
    assert!(registered(&shell).is_empty());
}

/// Review Focus 4: Enter twice on Confirm makes one request and one
/// repository.
#[gpui_kit::test]
async fn pressing_confirm_twice_creates_one_repository(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.empty_directory("new");
    let shell = settings(fixture, cx);
    fill_create(&shell, &root, cx);
    shell.click("create-submit", cx);
    assert!(shell.present("confirm-ok", cx));

    let held = shell.backend.hold_job(JobKind::RepoCreate);
    shell.keys("enter", cx);
    held.wait_reached();
    shell.keys("enter", cx);
    assert_eq!(creates(&shell, cx).len(), 1);
    held.release();
    let id = creates(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    assert_eq!(finished(&shell, id, cx), (Outcome::Success, ResultCode::Ok));
    assert_eq!(creates(&shell, cx), [id]);
    let ran = shell
        .backend
        .jobs_seen()
        .into_iter()
        .filter(|kind| *kind == JobKind::RepoCreate)
        .count();
    assert_eq!(ran, 1);
    assert_eq!(registered(&shell), [key(&root)]);
    until_onboarded(&shell, &root, cx).await;
    assert_eq!(background_refreshes(&shell, &root, cx), 1);
}

/// Partial: a create that fails once the repository is initialized ends
/// `partial`, with its recovery in the operation area. Retry, under the
/// same request, completes it; the list shows it, and one refresh of it
/// is submitted.
#[gpui_kit::test]
async fn a_partial_create_is_completed_by_retry_under_the_same_request(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.empty_directory("new");
    let shell = settings(fixture, cx);
    fill_create(&shell, &root, cx);
    shell.click("create-submit", cx);
    assert!(shell.present("confirm-ok", cx));
    shell.backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            shell.fixture.data_dir(),
            FailurePoint::AfterRepositoryInitialization,
        )
        .unwrap(),
    );

    shell.click("confirm-ok", cx);
    let id = creates(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    let (outcome, code) = finished(&shell, id, cx);
    assert_eq!(outcome, Outcome::Partial, "{}", code.as_str());
    assert!(!shell.present("confirm-dialog", cx));
    let (request_id, retry) = {
        let operations = shell.operations(cx);
        operations.read_with(cx, |operations, _| {
            let request = operations.request(id).unwrap();
            let RequestState::Finished(_, _, _, recovery, _) = &request.state else {
                unreachable!()
            };
            let retry = recovery
                .iter()
                .position(|action| action.action == RecoveryActionKind::RequestRetry)
                .expect("the partial create offers its retry");
            (request.request_id, retry)
        })
    };
    assert!(shell.present(recovery_id(id, retry), cx));
    assert_eq!(
        shell.label(format!("request-{}-code", id.0), cx).as_deref(),
        Some(code.as_str())
    );
    assert_eq!(background_refreshes(&shell, &root, cx), 0);

    shell
        .backend
        .replace_service(RepositoryService::open_at(shell.fixture.data_dir()).unwrap());
    shell.click(recovery_id(id, retry), cx);
    shell
        .until(cx, move |operations| {
            operations.request(id).is_some_and(|request| {
                request.attempt == 2 && matches!(request.state, RequestState::Finished(..))
            })
        })
        .await;
    assert_eq!(finished(&shell, id, cx), (Outcome::Success, ResultCode::Ok));
    assert_eq!(creates(&shell, cx), [id]);
    let same = shell.operations(cx).read_with(cx, |operations, _| {
        operations.request(id).unwrap().request_id == request_id
    });
    assert!(same, "Retry is the same request to the library");
    assert_eq!(registered(&shell), [key(&root)]);
    until_onboarded(&shell, &root, cx).await;
    assert_eq!(background_refreshes(&shell, &root, cx), 1);
}

/// Browse asks for one directory. A selection fills the field and checks
/// the folder; a dismissal leaves the field as it was, and usable.
#[gpui_kit::test]
fn browse_fills_the_field_and_a_dismissal_leaves_it_unchanged(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.plain_repository("plain", Some(ada()));
    let shell = settings(fixture, cx);

    shell.click("add-browse", cx);
    let chosen = root.clone();
    cx.simulate_path_prompt_response(move |options| {
        assert!(options.directories && !options.files && !options.multiple);
        assert_eq!(
            options.prompt.as_ref().map(|p| p.as_ref()),
            Some(BROWSE_PROMPT)
        );
        Some(vec![chosen])
    });
    shell.run(cx);
    assert_eq!(value(&shell, "add-path", cx), text(&root));
    assert_eq!(
        shell.label("add-inspection", cx).as_deref(),
        Some(NOT_ADDED)
    );

    set(&shell, "create-path", "/typed/by/hand", cx);
    shell.click("create-browse", cx);
    cx.simulate_path_prompt_response(|_| None);
    shell.run(cx);
    assert_eq!(value(&shell, "create-path", cx), "/typed/by/hand");
    assert!(!shell.present("browse-unavailable", cx));
    // Still usable: typing changes it, and Browse asks again.
    focus_in(&shell, "create-path", cx);
    shell.visual(cx).simulate_input("/more");
    shell.run(cx);
    assert_eq!(value(&shell, "create-path", cx), "/typed/by/hand/more");
    shell.click("create-browse", cx);
    cx.simulate_path_prompt_response(|_| None);
    shell.run(cx);
}

fn control_focus(shell: &Shell, id: &str, cx: &mut TestAppContext) -> FocusHandle {
    let page = page(shell, cx);
    page.read_with(cx, |page, _| page.control_focus(id))
}

/// In a window too short for the page, Tab reaches Create repository,
/// below the fold, and the content region scrolls it into view: the
/// control is a Kit button, whose own focus is private, so its wrapper
/// is revealed.
#[gpui_kit::test]
fn a_form_control_below_the_fold_is_scrolled_into_view(cx: &mut TestAppContext) {
    let shell = settings(Fixture::new(), cx);
    shell.visual(cx).simulate_resize(size(px(1440.), px(640.)));
    shell.run(cx);
    let bounds = |shell: &Shell, cx: &mut TestAppContext| {
        shell.frame(cx, |window, _| {
            (
                window.find("content").bounds(),
                window.find("create-submit").bounds(),
            )
        })
    };
    let (viewport, submit) = bounds(&shell, cx);
    assert!(
        submit.top() > viewport.bottom(),
        "Create repository starts below the fold: {submit:?} {viewport:?}"
    );

    let focus = control_focus(&shell, "create-submit", cx);
    let mut presses = 0;
    while !shell.frame(cx, |window, cx| focus.contains_focused(window, cx)) {
        presses += 1;
        assert!(presses < 60, "Tab never reached Create repository");
        shell.keys("tab", cx);
    }
    let (viewport, submit) = bounds(&shell, cx);
    assert!(
        submit.intersects(&viewport),
        "the focused button is in view: {submit:?} {viewport:?}"
    );
    let offset = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.content_offset());
    assert!(offset < px(0.));
}

/// The `repo.inspect` recovery's entry point opens the Repositories page
/// with the root checked, and focuses it. The model maps the action to it
/// (`operations_tests`); no local command offers it, so it is called here
/// as `run_recovery` calls it.
#[gpui_kit::test]
fn the_inspect_recovery_opens_the_page_with_its_root_checked(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let shell = Shell::open(fixture, cx);
    let target = root.clone();
    shell.visual(cx).update(|window, cx| {
        shell.workspace.update(cx, |workspace, cx| {
            workspace.inspect_in_settings(target, window, cx)
        })
    });
    shell.run(cx);
    assert!(shell.present("settings-repositories", cx));
    assert_eq!(value(&shell, "add-path", cx), text(&root));
    assert_eq!(
        shell.label("add-inspection", cx).as_deref(),
        Some(ALREADY_ADDED)
    );
    assert!(shell.present("add-configuration", cx));
    assert!(shell.frame(cx, |window, _| {
        window.find(("settings-page", 0usize)).focused() == Some(true)
    }));
}

/// A plain repository with an identity, its folder checked in Add.
fn checked_add(cx: &mut TestAppContext) -> (Shell, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.plain_repository("plain", Some(ada()));
    let shell = settings(fixture, cx);
    set(&shell, "add-path", text(&root), cx);
    enter_in(&shell, "add-path", cx);
    assert_eq!(
        shell.label("add-inspection", cx).as_deref(),
        Some(NOT_ADDED)
    );
    (shell, root)
}

/// Escape on an Add preview: nothing is registered, nothing is written
/// to the repository, no request is made, and focus goes back.
#[gpui_kit::test]
fn escape_on_an_add_preview_makes_nothing(cx: &mut TestAppContext) {
    let (shell, root) = checked_add(cx);
    let submit = control_focus(&shell, "add-submit", cx);
    shell
        .visual(cx)
        .update(|window, cx| window.focus(&submit, cx));
    shell.click("add-submit", cx);
    assert_eq!(shell.label("confirm-ok", cx).as_deref(), Some(CONFIRM));

    shell.keys("escape", cx);
    assert!(!shell.present("confirm-dialog", cx));
    assert!(registered(&shell).is_empty());
    assert!(!root.join(".manyhands").exists());
    assert!(shell.requests(cx, |_| true).is_empty());
    assert!(shell.frame(cx, |window, _| submit.is_focused(window)));
}

/// A stale Add preview: a second actor enables the same root after the
/// preview. Confirm is answered with Prepare again, and the root is
/// registered once.
#[gpui_kit::test]
async fn an_add_whose_repository_was_enabled_since_its_preview_offers_prepare_again(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root) = checked_add(cx);
    shell.click("add-submit", cx);
    assert!(shell.present("confirm-ok", cx));
    let other = RepositoryService::open_at(shell.fixture.data_dir()).unwrap();
    enable_through(&other, &root);

    shell.click("confirm-ok", cx);
    let id = enables(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    let (_, code) = finished(&shell, id, cx);
    assert_ne!(code, ResultCode::Ok);
    assert_eq!(
        shell.label("confirm-prepare-again", cx).as_deref(),
        Some(PREPARE_AGAIN),
        "{}",
        code.as_str()
    );
    assert_eq!(registered(&shell), [key(&root)]);
    assert_eq!(enables(&shell, cx), [id]);
}

/// Ruling R-T12b: Add pressed at once after typing the folder of a
/// repository Git has no identity for checks it first. The identity
/// fields appear with focus in the first, and no preview opens; filled,
/// Add opens the preview.
#[gpui_kit::test]
fn add_pressed_at_once_checks_first_and_asks_for_the_identity(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.plain_repository("plain", None);
    let shell = settings(fixture, cx);
    set(&shell, "add-path", text(&root), cx);
    assert!(!shell.present("add-inspection", cx));

    shell.click("add-submit", cx);
    assert_eq!(
        shell.label("add-identity-required", cx).as_deref(),
        Some(IDENTITY_REQUIRED)
    );
    assert!(!shell.present("confirm-dialog", cx));
    let name = control_focus(&shell, "add-name", cx);
    assert!(shell.frame(cx, |window, cx| name.contains_focused(window, cx)));
    assert_eq!(value(&shell, "add-branch", cx), "main");
    assert!(shell.requests(cx, |_| true).is_empty());

    set(&shell, "add-name", &ada().name, cx);
    set(&shell, "add-email", &ada().email, cx);
    shell.click("add-submit", cx);
    assert_eq!(shell.label("confirm-ok", cx).as_deref(), Some(CONFIRM));
    let details = details(&shell, cx);
    assert!(
        details.contains(&format!("Name: {}", ada().name)),
        "{details:?}"
    );
}

/// The same immediate Add on a repository Git has an identity for goes
/// straight on to the preview, the primary branch filled from the check.
#[gpui_kit::test]
fn add_pressed_at_once_on_a_repository_with_an_identity_opens_the_preview(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.plain_repository("plain", Some(ada()));
    let shell = settings(fixture, cx);
    set(&shell, "add-path", text(&root), cx);
    shell.click("add-submit", cx);
    assert_eq!(shell.label("confirm-ok", cx).as_deref(), Some(CONFIRM));
    assert!(!shell.present("add-identity-required", cx));
    assert!(details(&shell, cx).contains(&"Primary branch: main".to_owned()));
}
