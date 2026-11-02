//! The Remotes page over real repositories: the list with its redacted
//! locations, eligibility and selection; Add, Select and Select none as
//! user jobs, with their typed failures; Remove through the confirmation
//! dialog, its refusal of a remote the binding cannot remove
//! (`01M4PB7WZN9WNAQ3KXZ19J28KM`), Escape and stale previews; a partial
//! add that Retry completes under the same request; and a remote URL's
//! credential kept out of everything shown.

use std::path::{Path, PathBuf};

use gpui_kit::{
    Entity, FocusHandle, TestAppContext,
    component::{WindowExt, input::InputState},
};
use manyhands::{
    repository::{FailurePoint, Mutation, RemoteDto, RemoteRemoveInput, RepositoryService},
    results::{Outcome, RecoveryActionKind, ResultCode},
};

use super::{
    ELIGIBLE, NO_REMOTES, NOT_ELIGIBLE, RemotesPage, SELECTED, SELECTION_NONE, chosen_line,
    location_line, selection_line,
};
use crate::desktop::{
    services::{
        ReadKind,
        requests::{DesktopRequestId, Intent, RequestState},
    },
    test_support::fixtures::{Fixture, remote_names, remote_url, remove_remote_through},
    view::{
        confirm::{CONFIRM, PREPARE_AGAIN},
        harness::{Shell, key, wait_until},
        operations::{effects_text, recovery_id, request_id},
    },
};

const SENTINEL: &str = "sentinel-password-5c1e";
const SSH_URL: &str = "ssh://git@example.invalid/team/repo.git";
const HTTPS_URL: &str = "https://example.invalid/team/repo.git";

fn with_password() -> String {
    format!("https://ada:{SENTINEL}@example.invalid/team/secret.git")
}

/// The window over `fixture` with `root` selected and the Remotes page
/// shown.
fn remotes(fixture: Fixture, root: &Path, cx: &mut TestAppContext) -> Shell {
    let shell = Shell::open(fixture, cx);
    shell.select(root, cx);
    shell.keys("secondary-,", cx);
    shell.click(("settings-page", 1usize), cx);
    assert!(shell.present("settings-remotes", cx));
    shell
}

fn page(shell: &Shell, cx: &mut TestAppContext) -> Entity<RemotesPage> {
    let host = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.settings().clone());
    host.read_with(cx, |host, _| host.remotes.clone())
        .expect("the Remotes page is made")
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

fn control_focus(shell: &Shell, id: impl Into<String>, cx: &mut TestAppContext) -> FocusHandle {
    let page = page(shell, cx);
    let id = id.into();
    page.read_with(cx, |page, _| page.control_focus(&id))
}

/// The remotes the library lists for `root`: the oracle.
fn listed(shell: &Shell, root: &Path) -> Vec<RemoteDto> {
    let service = shell.fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    service.list_remotes_redacted(&repo).unwrap().items
}

/// The page's row of the remote `name`.
fn row(shell: &Shell, name: &str, cx: &mut TestAppContext) -> usize {
    shell
        .rows("remote-name", cx)
        .iter()
        .position(|shown| shown == name)
        .unwrap_or_else(|| panic!("{name} is not listed"))
}

/// Waits until the page lists the remote `name`.
async fn until_listed(shell: &Shell, name: &'static str, cx: &mut TestAppContext) {
    let page = page(shell, cx);
    wait_until(cx, &page, move |page, _| {
        page.remote_names().iter().any(|listed| listed == name)
    })
    .await;
}

fn of(shell: &Shell, cx: &mut TestAppContext, pick: fn(&Intent) -> bool) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| pick(&request.intent))
}

fn adds(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    of(shell, cx, |intent| {
        matches!(intent, Intent::RemoteAdd { .. })
    })
}

fn selects(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    of(shell, cx, |intent| {
        matches!(intent, Intent::RemoteSelect { .. })
    })
}

fn removes(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    of(shell, cx, |intent| {
        matches!(intent, Intent::RemoteRemove { .. })
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

/// Fills the Add form and presses Add remote.
fn add(shell: &Shell, name: &str, url: &str, cx: &mut TestAppContext) {
    set(shell, "remote-add-name", name, cx);
    set(shell, "remote-add-url", url, cx);
    shell.click("remote-add-submit", cx);
}

/// Every piece of text a test can read off the window: each observed
/// element's label and accessibility value.
fn window_text(shell: &Shell, cx: &mut TestAppContext) -> Vec<String> {
    shell.frame(cx, |window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .flat_map(|snapshot| [snapshot.label(), snapshot.value()])
            .flatten()
            .map(str::to_owned)
            .collect()
    })
}

/// Every field of every request the operation area could show: what the
/// request record shows, its outcome and its recovery's arguments.
fn request_fields(shell: &Shell, cx: &mut TestAppContext) -> Vec<String> {
    let operations = shell.operations(cx);
    operations.read_with(cx, |operations, _| {
        let mut fields = Vec::new();
        for request in operations.requests() {
            fields.push(request.request_id.to_string());
            fields.push(request.intent.label().to_owned());
            fields.extend(request.intent.root().map(|root| root.display().to_string()));
            fields.extend(request.confirmation.map(|id| id.to_string()));
            if let RequestState::Finished(outcome, code, effects, recovery, message) =
                &request.state
            {
                fields.push(format!("{outcome:?} {code:?} {effects:?}"));
                for action in recovery {
                    fields.push(serde_json::to_string(&action.arguments).unwrap());
                    fields.extend(action.operation_id.clone());
                }
                fields.push(message.clone());
            }
        }
        fields
    })
}

/// A remote whose URL holds a password is listed by its redacted
/// locations, the library's own, and the password is nowhere in the
/// window. An HTTPS remote is listed, labeled not eligible.
#[gpui_kit::test]
fn a_remote_url_with_a_password_is_shown_redacted(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.add_remote(&root, "origin", &with_password());
    let shell = remotes(fixture, &root, cx);
    let oracle = listed(&shell, &root);
    assert_eq!(oracle.len(), 1);
    assert!(!oracle[0].fetch_location.contains(SENTINEL));

    assert_eq!(shell.rows("remote-name", cx), ["origin"]);
    assert_eq!(
        shell.label(("remote-fetch", 0usize), cx),
        Some(location_line(
            super::FETCHES_FROM,
            &oracle[0].fetch_location
        ))
    );
    assert_eq!(
        shell.label(("remote-push", 0usize), cx),
        Some(location_line(super::PUSHES_TO, &oracle[0].push_location))
    );
    assert_eq!(
        shell.label(("remote-eligibility", 0usize), cx).as_deref(),
        Some(NOT_ELIGIBLE)
    );
    assert!(!shell.present(("remote-selected", 0usize), cx));
    let shown = window_text(&shell, cx);
    assert!(
        shown.iter().all(|text| !text.contains(SENTINEL)),
        "{shown:?}"
    );
}

/// Add with a URL that holds a password: the remote is added as typed,
/// the form lets the URL go, and the password is in no request field, no
/// operation-area text and nothing the window shows; the list shows the
/// remote redacted.
#[gpui_kit::test]
async fn an_added_url_with_a_password_is_kept_out_of_everything_shown(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let shell = remotes(fixture, &root, cx);
    assert_eq!(
        shell.label("remotes-state", cx).as_deref(),
        Some(NO_REMOTES)
    );

    add(&shell, "backup", &with_password(), cx);
    let id = adds(&shell, cx)[0];
    // The form no longer holds what was typed.
    assert_eq!(value(&shell, "remote-add-url", cx), "");
    assert_eq!(value(&shell, "remote-add-name", cx), "");
    shell.until_ended(id, cx).await;
    assert_eq!(finished(&shell, id, cx), (Outcome::Success, ResultCode::Ok));
    assert_eq!(remote_url(&root, "backup"), Some(with_password()));
    until_listed(&shell, "backup", cx).await;
    let oracle = listed(&shell, &root);
    let ix = row(&shell, "backup", cx);
    assert_eq!(
        shell.label(("remote-fetch", ix), cx),
        Some(location_line(
            super::FETCHES_FROM,
            &oracle[0].fetch_location
        ))
    );

    let fields = request_fields(&shell, cx);
    assert!(!fields.is_empty());
    assert!(
        fields.iter().all(|field| !field.contains(SENTINEL)),
        "{fields:?}"
    );
    shell.keys("secondary-4", cx);
    assert_eq!(
        shell.label(request_id(id, "command"), cx).as_deref(),
        Some(
            Intent::RemoteAdd {
                root: PathBuf::new(),
                name: String::new(),
                url: String::new()
            }
            .label()
        )
    );
    let shown = window_text(&shell, cx);
    assert!(!shown.is_empty());
    assert!(
        shown.iter().all(|text| !text.contains(SENTINEL)),
        "{shown:?}"
    );
    let notifications = shell.frame(cx, |window, cx| window.notifications(cx).len());
    assert_eq!(notifications, 0);
}

/// Add an SSH remote, then Select it: it is listed eligible, then shown
/// selected for publication, as the library lists it. Select none clears
/// the selection.
#[gpui_kit::test]
async fn an_added_ssh_remote_selected_shows_as_selected_for_publication(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let shell = remotes(fixture, &root, cx);
    assert_eq!(
        shell.label("remotes-selection", cx).as_deref(),
        Some(SELECTION_NONE)
    );

    add(&shell, "publish", SSH_URL, cx);
    let id = adds(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    assert_eq!(finished(&shell, id, cx), (Outcome::Success, ResultCode::Ok));
    until_listed(&shell, "publish", cx).await;
    let ix = row(&shell, "publish", cx);
    assert_eq!(
        shell.label(("remote-eligibility", ix), cx).as_deref(),
        Some(ELIGIBLE)
    );
    assert!(!shell.present(("remote-selected", ix), cx));

    shell.click(("remote-select", ix), cx);
    let id = selects(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    assert_eq!(finished(&shell, id, cx), (Outcome::Success, ResultCode::Ok));
    let page = page(&shell, cx);
    wait_until(cx, &page, |page, _| page.selected() == Some("publish")).await;
    assert!(listed(&shell, &root)[0].selected_for_publication);
    assert_eq!(
        shell.label(("remote-selected", ix), cx).as_deref(),
        Some(SELECTED)
    );
    assert_eq!(
        shell.label("remotes-selection", cx),
        Some(selection_line("publish"))
    );

    shell.click("remotes-select-none", cx);
    let none = selects(&shell, cx)[0];
    assert_ne!(none, id);
    shell.until_ended(none, cx).await;
    assert_eq!(
        finished(&shell, none, cx),
        (Outcome::Success, ResultCode::Ok)
    );
    wait_until(cx, &page, |page, _| page.selected().is_none()).await;
    assert!(!listed(&shell, &root)[0].selected_for_publication);
    assert_eq!(
        shell.label("remotes-selection", cx).as_deref(),
        Some(SELECTION_NONE)
    );
    assert!(!shell.present(("remote-selected", ix), cx));
}

/// A remote that fetches over HTTPS is listed and labeled not eligible;
/// selecting it is answered with the library's typed failure, shown on
/// the page and in the operation area, and nothing is selected.
#[gpui_kit::test]
async fn selecting_a_remote_that_is_not_eligible_shows_its_typed_failure(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.add_remote(&root, "origin", HTTPS_URL);
    let shell = remotes(fixture, &root, cx);
    assert!(!listed(&shell, &root)[0].publication_eligible);
    assert_eq!(
        shell.label(("remote-eligibility", 0usize), cx).as_deref(),
        Some(NOT_ELIGIBLE)
    );

    shell.click(("remote-select", 0usize), cx);
    let id = selects(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    let (outcome, code) = finished(&shell, id, cx);
    assert_ne!(outcome, Outcome::Success);
    assert_eq!(code, ResultCode::InvalidRemote);
    assert_eq!(
        shell.label("remotes-result-code", cx).as_deref(),
        Some(code.as_str())
    );
    assert_eq!(
        shell.label(request_id(id, "code"), cx).as_deref(),
        Some(code.as_str())
    );
    assert!(!listed(&shell, &root)[0].selected_for_publication);
    assert_eq!(
        shell.label("remotes-selection", cx).as_deref(),
        Some(SELECTION_NONE)
    );
}

/// Add under a name already taken with another location is answered
/// `remote_name_conflict`; the remote keeps its location.
#[gpui_kit::test]
async fn adding_a_remote_whose_name_is_taken_shows_its_typed_failure(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.add_remote(&root, "origin", HTTPS_URL);
    let shell = remotes(fixture, &root, cx);

    add(&shell, "origin", SSH_URL, cx);
    let id = adds(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    let (outcome, code) = finished(&shell, id, cx);
    assert_ne!(outcome, Outcome::Success);
    assert_eq!(code, ResultCode::RemoteNameConflict);
    assert_eq!(
        shell.label("remotes-result-code", cx).as_deref(),
        Some(code.as_str())
    );
    assert_eq!(remote_url(&root, "origin").as_deref(), Some(HTTPS_URL));
}

/// A refused add of a URL with a password: the library answers
/// `remote_name_conflict`, and the password is in no request field, no
/// operation-area text and nothing the window shows.
#[gpui_kit::test]
async fn a_refused_add_of_a_url_with_a_password_keeps_it_out_of_everything_shown(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.add_remote(&root, "origin", HTTPS_URL);
    let shell = remotes(fixture, &root, cx);

    add(&shell, "origin", &with_password(), cx);
    let id = adds(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    let (outcome, code) = finished(&shell, id, cx);
    assert_ne!(outcome, Outcome::Success);
    assert_eq!(code, ResultCode::RemoteNameConflict);
    assert_eq!(remote_url(&root, "origin").as_deref(), Some(HTTPS_URL));

    let fields = request_fields(&shell, cx);
    assert!(!fields.is_empty());
    assert!(
        fields.iter().all(|field| !field.contains(SENTINEL)),
        "{fields:?}"
    );
    shell.keys("secondary-4", cx);
    let shown = window_text(&shell, cx);
    assert!(!shown.is_empty());
    assert!(
        shown.iter().all(|text| !text.contains(SENTINEL)),
        "{shown:?}"
    );
    let notifications = shell.frame(cx, |window, cx| window.notifications(cx).len());
    assert_eq!(notifications, 0);
}

/// Add with no name, or no location, says so and asks nothing.
#[gpui_kit::test]
fn adding_with_an_empty_field_says_so(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let shell = remotes(fixture, &root, cx);
    shell.click("remote-add-submit", cx);
    assert_eq!(
        shell.label("remote-add-message", cx).as_deref(),
        Some(super::NEED_NAME)
    );
    set(&shell, "remote-add-name", "backup", cx);
    shell.click("remote-add-submit", cx);
    assert_eq!(
        shell.label("remote-add-message", cx).as_deref(),
        Some(super::NEED_URL)
    );
    assert!(shell.requests(cx, |_| true).is_empty());
}

/// Remove: the preview shows the remote and its redacted locations and
/// removes nothing; Confirm removes it, and the list no longer gives it.
#[gpui_kit::test]
async fn removing_a_remote_previews_then_removes_it(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.add_remote(&root, "backup", &with_password());
    let shell = remotes(fixture, &root, cx);
    let oracle = listed(&shell, &root);

    shell.click(("remote-remove", 0usize), cx);
    assert_eq!(shell.label("confirm-ok", cx).as_deref(), Some(CONFIRM));
    let details = shell.rows("confirm-detail", cx);
    for wanted in [
        "Remote: backup".to_owned(),
        format!("Fetches from: {}", oracle[0].fetch_location),
    ] {
        assert!(details.contains(&wanted), "{wanted} in {details:?}");
    }
    let shown = window_text(&shell, cx);
    assert!(
        shown.iter().all(|text| !text.contains(SENTINEL)),
        "{shown:?}"
    );
    assert_eq!(remote_names(&root), ["backup"]);
    assert!(removes(&shell, cx).is_empty());

    shell.click("confirm-ok", cx);
    let id = removes(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    assert_eq!(finished(&shell, id, cx), (Outcome::Success, ResultCode::Ok));
    assert!(remote_names(&root).is_empty());
    let page = page(&shell, cx);
    wait_until(cx, &page, |page, _| page.remote_names().is_empty()).await;
    assert_eq!(
        shell.label("remotes-state", cx).as_deref(),
        Some(NO_REMOTES)
    );
    assert!(!shell.present("confirm-dialog", cx));
}

/// `01M4PB7WZN9WNAQ3KXZ19J28KM`: removing `origin` beside `origin.bak`
/// is refused by the binding at its preview. The dialog shows the
/// refusal's code and message, with no Confirm; both remotes stay and no
/// request is made.
#[gpui_kit::test]
fn removing_origin_beside_origin_bak_shows_the_refusal_and_keeps_both(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.add_sibling_remotes(&root);
    let expected = fixture
        .service()
        .prepare(&Mutation::RemoteRemove(RemoteRemoveInput {
            root: root.clone(),
            name: "origin".to_owned(),
        }));
    assert_ne!(expected.code, ResultCode::Ok);
    let shell = remotes(fixture, &root, cx);
    assert_eq!(shell.rows("remote-name", cx), ["origin", "origin.bak"]);

    shell.click(("remote-remove", 0usize), cx);
    assert_eq!(
        shell.label("confirm-code", cx).as_deref(),
        Some(expected.code.as_str())
    );
    assert_eq!(
        shell.label("confirm-message", cx).as_deref(),
        Some(expected.message.as_str())
    );
    assert!(!shell.present("confirm-ok", cx));
    assert_eq!(remote_names(&root), ["origin", "origin.bak"]);
    assert!(shell.requests(cx, |_| true).is_empty());

    shell.keys("escape", cx);
    assert!(!shell.present("confirm-dialog", cx));
    assert_eq!(shell.rows("remote-name", cx), ["origin", "origin.bak"]);
}

/// Escape closes the removal's preview with nothing removed and no
/// request, and focus goes back to Remove.
#[gpui_kit::test]
fn escape_on_a_remove_preview_removes_nothing(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.add_remote(&root, "backup", HTTPS_URL);
    let shell = remotes(fixture, &root, cx);
    let remove = control_focus(&shell, "remote-remove-0", cx);
    shell
        .visual(cx)
        .update(|window, cx| window.focus(&remove, cx));
    shell.click(("remote-remove", 0usize), cx);
    assert!(shell.present("confirm-ok", cx));

    shell.keys("escape", cx);
    assert!(!shell.present("confirm-dialog", cx));
    assert_eq!(remote_names(&root), ["backup"]);
    assert!(shell.requests(cx, |_| true).is_empty());
    assert!(shell.frame(cx, |window, _| remove.is_focused(window)));
}

/// A stale preview: a second actor points the remote elsewhere between
/// the preview and Confirm. The answer is shown with Prepare again, and
/// the remote stays, as the second actor left it.
#[gpui_kit::test]
async fn a_remote_changed_since_its_remove_preview_offers_prepare_again(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.add_remote(&root, "backup", HTTPS_URL);
    let shell = remotes(fixture, &root, cx);
    shell.click(("remote-remove", 0usize), cx);
    assert!(shell.present("confirm-ok", cx));
    shell.fixture.set_remote_url(&root, "backup", SSH_URL);

    shell.click("confirm-ok", cx);
    let id = removes(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    let (_, code) = finished(&shell, id, cx);
    assert_ne!(code, ResultCode::Ok);
    assert_eq!(
        shell.label("confirm-prepare-again", cx).as_deref(),
        Some(PREPARE_AGAIN),
        "{}",
        code.as_str()
    );
    assert_eq!(remote_url(&root, "backup").as_deref(), Some(SSH_URL));
}

/// A stale preview: a second actor removes the remote between the
/// preview and Confirm. The answer offers Prepare again, whose fresh
/// preview finds nothing left to remove; the list no longer gives it.
#[gpui_kit::test]
async fn a_remote_removed_since_its_preview_offers_prepare_again(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.add_remote(&root, "backup", HTTPS_URL);
    let shell = remotes(fixture, &root, cx);
    shell.click(("remote-remove", 0usize), cx);
    assert!(shell.present("confirm-ok", cx));
    let other = RepositoryService::open_at(shell.fixture.data_dir()).unwrap();
    remove_remote_through(&other, &root, "backup");

    shell.click("confirm-ok", cx);
    let id = removes(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    let (_, code) = finished(&shell, id, cx);
    assert_ne!(code, ResultCode::Ok);
    assert_eq!(
        shell.label("confirm-prepare-again", cx).as_deref(),
        Some(PREPARE_AGAIN),
        "{}",
        code.as_str()
    );
    shell.click("confirm-prepare-again", cx);
    assert_eq!(
        shell.label("confirm-code", cx).as_deref(),
        Some(ResultCode::AlreadyApplied.as_str())
    );
    assert!(!shell.present("confirm-ok", cx));
    shell.keys("escape", cx);
    let page = page(&shell, cx);
    wait_until(cx, &page, |page, _| page.remote_names().is_empty()).await;
}

/// Partial: an add that fails once the remote is written ends `partial`,
/// and the operation area shows its code, its completed effects and its
/// Retry. Retry, under the same request, completes it.
#[gpui_kit::test]
async fn a_partial_add_is_completed_by_retry_under_the_same_request(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let shell = remotes(fixture, &root, cx);
    shell.backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            shell.fixture.data_dir(),
            FailurePoint::AfterRemoteMutation,
        )
        .unwrap(),
    );

    add(&shell, "publish", SSH_URL, cx);
    let id = adds(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    let (outcome, code) = finished(&shell, id, cx);
    assert_eq!(outcome, Outcome::Partial, "{}", code.as_str());
    let (request_id_before, retry, effects) = {
        let operations = shell.operations(cx);
        operations.read_with(cx, |operations, _| {
            let request = operations.request(id).unwrap();
            let RequestState::Finished(_, _, effects, recovery, _) = &request.state else {
                unreachable!()
            };
            let retry = recovery
                .iter()
                .position(|action| action.action == RecoveryActionKind::RequestRetry)
                .expect("the partial add offers its retry");
            (request.request_id, retry, effects_text(effects))
        })
    };
    assert_eq!(
        shell.label(request_id(id, "code"), cx).as_deref(),
        Some(code.as_str())
    );
    assert_eq!(
        shell.label(request_id(id, "effects"), cx).as_deref(),
        Some(effects.as_str())
    );
    assert!(shell.present(recovery_id(id, retry), cx));
    assert_eq!(remote_url(&root, "publish").as_deref(), Some(SSH_URL));

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
    assert_eq!(adds(&shell, cx), [id]);
    let same = shell.operations(cx).read_with(cx, |operations, _| {
        operations.request(id).unwrap().request_id == request_id_before
    });
    assert!(same, "Retry is the same request to the library");
    until_listed(&shell, "publish", cx).await;
    assert_eq!(remote_names(&root), ["publish"]);
}

/// The page works on the selected repository; another registration can
/// be chosen, and its remotes are listed.
#[gpui_kit::test]
fn the_page_lists_the_selected_repository_or_the_one_chosen(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    let b = fixture.create_repository("b");
    fixture.add_remote(&b, "origin", HTTPS_URL);
    let shell = remotes(fixture, &a, cx);
    assert_eq!(shell.label("remotes-root", cx), Some(chosen_line(&key(&a))));
    assert_eq!(
        shell.label("remotes-state", cx).as_deref(),
        Some(NO_REMOTES)
    );

    let rows = shell.rows("remotes-registration", cx);
    let ix = rows
        .iter()
        .position(|root| *root == key(&b))
        .expect("b is listed");
    shell.click(("remotes-choose", ix), cx);
    assert_eq!(shell.label("remotes-root", cx), Some(chosen_line(&key(&b))));
    assert_eq!(shell.rows("remote-name", cx), ["origin"]);
}

/// While a dialog is open, the page's handlers do nothing: no add,
/// selection, choice or second dialog.
#[gpui_kit::test]
fn the_page_acts_on_nothing_while_a_dialog_is_open(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let b = fixture.create_repository("b");
    fixture.add_remote(&root, "backup", HTTPS_URL);
    let shell = remotes(fixture, &root, cx);
    set(&shell, "remote-add-name", "publish", cx);
    set(&shell, "remote-add-url", SSH_URL, cx);
    shell.click(("remote-remove", 0usize), cx);
    assert!(shell.present("confirm-ok", cx));

    let page = page(&shell, cx);
    shell.visual(cx).update(|window, cx| {
        page.update(cx, |page, cx| {
            page.submit_add(window, cx);
            page.submit_select(root.clone(), Some("backup".to_owned()), window, cx);
            page.submit_select(root.clone(), None, window, cx);
            page.submit_remove(root.clone(), "backup".to_owned(), 0, window, cx);
            page.choose_root(b.clone(), window, cx);
        })
    });
    shell.run(cx);
    assert!(shell.requests(cx, |_| true).is_empty());
    assert_eq!(value(&shell, "remote-add-url", cx), SSH_URL);
    assert_eq!(
        shell.label("remotes-root", cx),
        Some(chosen_line(&key(&root)))
    );
    let dialogs = shell.frame(cx, |window, cx| window.has_active_dialog(cx));
    assert!(dialogs);
    shell.keys("escape", cx);
    assert!(!shell.present("confirm-dialog", cx));
}

/// Review fix: Choose another repository while its remotes are still
/// read shows none of the last one's rows under it, and its own once the
/// read lands.
#[gpui_kit::test]
fn choosing_another_repository_never_shows_the_last_ones_remotes(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    let b = fixture.create_repository("b");
    fixture.add_remote(&a, "origin", HTTPS_URL);
    fixture.add_remote(&b, "upstream", SSH_URL);
    let shell = remotes(fixture, &a, cx);
    assert_eq!(shell.rows("remote-name", cx), ["origin"]);

    let held = shell.backend.hold_read(ReadKind::Remotes);
    let ix = shell
        .rows("remotes-registration", cx)
        .iter()
        .position(|root| *root == key(&b))
        .expect("b is listed");
    shell.click(("remotes-choose", ix), cx);
    assert_eq!(shell.label("remotes-root", cx), Some(chosen_line(&key(&b))));
    assert!(!shell.present(("remote-select", 0usize), cx));
    assert!(!shell.present(("remote-remove", 0usize), cx));
    assert!(!shell.present("remotes-select-none", cx));
    assert!(!shell.present("remotes-selection", cx));
    assert!(shell.rows("remote-name", cx).is_empty());
    assert_eq!(
        shell.label("remotes-state", cx).as_deref(),
        Some(super::READING_REMOTES)
    );

    let _ = held.send(());
    shell.run(cx);
    assert_eq!(shell.rows("remote-name", cx), ["upstream"]);
}

/// Review fix: a row's Select and Remove act only on the repository the
/// row was read from. Once another is chosen, a row of the last one asks
/// nothing: no selection, and no preview.
#[gpui_kit::test]
fn a_row_of_another_repository_acts_on_nothing(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    let b = fixture.create_repository("b");
    fixture.add_remote(&a, "origin", HTTPS_URL);
    fixture.add_remote(&b, "origin", HTTPS_URL);
    let shell = remotes(fixture, &a, cx);
    let page = page(&shell, cx);
    shell
        .visual(cx)
        .update(|window, cx| page.update(cx, |page, cx| page.choose_root(b.clone(), window, cx)));
    shell.run(cx);
    assert_eq!(shell.label("remotes-root", cx), Some(chosen_line(&key(&b))));

    shell.visual(cx).update(|window, cx| {
        page.update(cx, |page, cx| {
            page.submit_select(a.clone(), Some("origin".to_owned()), window, cx);
            page.submit_select(a.clone(), None, window, cx);
            page.submit_remove(a.clone(), "origin".to_owned(), 0, window, cx);
        })
    });
    shell.run(cx);
    assert!(shell.requests(cx, |_| true).is_empty());
    assert!(!shell.present("confirm-dialog", cx));
}
