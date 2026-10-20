//! The Hosts page with no network (design, Testing; ruling R11): bin
//! tests cannot initialize the transport, so no test here reaches a host.
//!
//! Covered: the pin list, empty and with a pin the library's own test
//! entry point stores; Check host unavailable without an SSH publication
//! remote; Check host without the transport, which shows the library's
//! typed failure and pins nothing; "Connecting" with Cancel unavailable
//! while it runs; a result of another repository never shown; the approve
//! and replace previews, which `prepare` builds with no connection
//! (`bind/host.rs`), the replace preview with both fingerprints, and
//! Escape pinning nothing; the `host.approve` / `host.replace` recovery;
//! and focus back on Check host after a prompt that came while it ran.
//!
//! Observation that succeeds, unlock during observation, approval and
//! replacement over a real host are the native checklist's step 9 and
//! F2's SSH tests.

use std::{fs, path::Path, sync::mpsc, thread, time::Duration};

use gpui_kit::{Entity, FocusHandle, TestAppContext, component::WindowExt};
use manyhands::{
    repository::{
        HostObservationDto, HostPinDto, SharedKeyId,
        keys::{
            KeySourceToken, PassphraseResponse, SessionCredentialProvider, SessionCredentials,
            UnlockReason, UnlockRequest,
        },
        transport::SshAuthority,
    },
    results::{Envelope, Outcome, ResultCode, Scope},
};

use super::{
    CANCEL_UNAVAILABLE, CONNECTING, DIFFERENT, HostsPage, MATCHES, NO_PINS, NO_REPOSITORY,
    NO_SSH_PUBLICATION, NOT_PINNED, PINNED, Trust, authority_line, chosen_line, key_line,
};
use crate::desktop::{
    services::{JobKind, requests::Intent, worker::WorkerEvent},
    test_support::fixtures::{Fixture, NoPassphrase, SSH_HOST, SSH_PORT},
    view::{
        confirm::{CONFIRM, PINNED_KEY, REQUESTED_KEY},
        harness::{Shell, wait_until},
    },
};

const ALGORITHM: &str = "ssh-ed25519";
/// Host key fingerprints in the one form a pin holds.
const OLD: &str = "SHA256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
const NEW: &str = "SHA256:EEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEE";
const WAIT: Duration = Duration::from_secs(10);

fn authority() -> SshAuthority {
    SshAuthority {
        host: SSH_HOST.to_owned(),
        port: SSH_PORT,
    }
}

/// The window over `fixture` with `root` selected, if any, and the Hosts
/// page shown.
fn hosts(fixture: Fixture, root: Option<&Path>, cx: &mut TestAppContext) -> Shell {
    let shell = Shell::open(fixture, cx);
    if let Some(root) = root {
        shell.select(root, cx);
    }
    shell.keys("secondary-,", cx);
    shell.click(("settings-page", 3usize), cx);
    assert!(shell.present("settings-hosts", cx));
    shell
}

fn page(shell: &Shell, cx: &mut TestAppContext) -> Entity<HostsPage> {
    let host = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.settings.clone());
    host.read_with(cx, |host, _| host.hosts.clone())
        .expect("the Hosts page is made")
}

fn check_focus(shell: &Shell, cx: &mut TestAppContext) -> FocusHandle {
    page(shell, cx).read_with(cx, |page, _| page.check_focus())
}

/// The pins the library lists: the oracle.
fn pins(shell: &Shell) -> Vec<HostPinDto> {
    shell.fixture.service().list_host_pins().unwrap().items
}

/// Waits until the page shows the answer to its latest Check host.
async fn until_observed(shell: &Shell, cx: &mut TestAppContext) {
    let page = page(shell, cx);
    wait_until(cx, &page, |page, _| page.observed_code().is_some()).await;
    shell.run(cx);
}

/// Opens the confirmation dialog for `trust` from the page, as Approve and
/// Replace do with what Check host observed.
fn confirm_trust(shell: &Shell, trust: Trust, cx: &mut TestAppContext) {
    let page = page(shell, cx);
    shell
        .visual(cx)
        .update(|window, cx| page.update(cx, |page, cx| page.confirm_trust(trust, window, cx)));
    shell.run(cx);
}

fn approve(root: &Path, fingerprint: &str) -> Trust {
    Trust {
        root: root.to_owned(),
        authority: authority(),
        algorithm: ALGORITHM.to_owned(),
        fingerprint: fingerprint.to_owned(),
        replaces: None,
    }
}

fn replace(root: &Path, fingerprint: &str, old: &str) -> Trust {
    Trust {
        replaces: Some(old.to_owned()),
        ..approve(root, fingerprint)
    }
}

/// With no pin stored, the page says so, as the library lists none.
#[gpui_kit::test]
fn with_no_pin_the_page_says_so(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let shell = hosts(fixture, Some(&root), cx);
    assert!(pins(&shell).is_empty());
    assert_eq!(
        shell.label("hosts-pins-state", cx).as_deref(),
        Some(NO_PINS)
    );
    assert!(shell.rows("host-pin", cx).is_empty());
}

/// A pin the library's own test entry point stores is listed by its host,
/// port, algorithm and fingerprint, as `list_host_pins` gives it.
#[gpui_kit::test]
fn a_stored_pin_is_listed_as_the_library_lists_it(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    fixture.pin_host(ALGORITHM, OLD);
    let shell = hosts(fixture, None, cx);
    let oracle = pins(&shell);
    assert_eq!(oracle.len(), 1);
    assert_eq!(
        shell.rows("host-pin", cx),
        [authority_line(&oracle[0].host, oracle[0].port)]
    );
    assert_eq!(
        shell.label(("host-pin-key", 0usize), cx),
        Some(key_line(
            &oracle[0].pinned_algorithm,
            &oracle[0].pinned_fingerprint
        ))
    );
    assert_eq!(authority_line(SSH_HOST, SSH_PORT), "example.invalid:22");
    assert!(!shell.present("hosts-pins-state", cx));
}

/// With no repository selected Check host is not offered; with one that
/// has no SSH publication remote it is unavailable, said in fixed text,
/// and runs nothing.
#[gpui_kit::test]
fn check_host_needs_an_ssh_publication_remote(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.add_remote(&root, "origin", "https://example.invalid/team/repo.git");
    let shell = hosts(fixture, None, cx);
    assert_eq!(
        shell.label("hosts-check-state", cx).as_deref(),
        Some(NO_REPOSITORY)
    );
    assert!(!shell.present("hosts-check", cx));

    let shell = hosts(shell.fixture, Some(&root), cx);
    assert_eq!(
        shell.label("hosts-root", cx),
        Some(chosen_line(root.to_str().unwrap()))
    );
    assert_eq!(
        shell.label("hosts-check-state", cx).as_deref(),
        Some(NO_SSH_PUBLICATION)
    );
    // Drawn, unavailable: pressing it asks nothing.
    assert!(shell.present("hosts-check", cx));
    shell.click("hosts-check", cx);
    let page = page(&shell, cx);
    shell.visual(cx).update(|window, cx| {
        page.update(cx, |page, cx| page.check(window, cx));
    });
    shell.run(cx);
    assert!(!shell.backend.jobs_seen().contains(&JobKind::ObserveHost));
}

/// Check host with no transport initialized: the worker's observation
/// answers the library's typed failure, which the page shows by its
/// message and code, and nothing is pinned. While it runs the page says
/// "Connecting" and that Cancel is unavailable, in words, and a second
/// Check host asks nothing.
#[gpui_kit::test]
async fn check_host_without_the_transport_shows_the_typed_failure(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.publish_over_ssh(&root);
    let shell = hosts(fixture, Some(&root), cx);
    // The oracle: the library's own answer, asked here as the worker asks
    // it. With no transport it connects to nothing.
    let oracle = shell.fixture.service().observe_host(
        &root,
        None,
        &mut SessionCredentials::new(NoPassphrase),
    );
    assert_ne!(oracle.code, ResultCode::Ok);

    let held = shell.backend.hold_job(JobKind::ObserveHost);
    shell.click("hosts-check", cx);
    held.wait_reached();
    assert_eq!(
        shell.label("hosts-observation-state", cx).as_deref(),
        Some(CONNECTING)
    );
    assert_eq!(
        shell.label("hosts-cancel-unavailable", cx).as_deref(),
        Some(CANCEL_UNAVAILABLE)
    );
    let page = page(&shell, cx);
    shell.visual(cx).update(|window, cx| {
        page.update(cx, |page, cx| page.check(window, cx));
    });
    held.release();
    until_observed(&shell, cx).await;

    assert_eq!(
        shell.label("hosts-observation-message", cx),
        Some(oracle.message.clone())
    );
    assert_eq!(
        shell.label("hosts-observation-code", cx).as_deref(),
        Some(oracle.code.as_str())
    );
    assert!(!shell.present("hosts-observation-state", cx));
    assert!(!shell.present("hosts-approve", cx));
    assert!(!shell.present("hosts-replace", cx));
    assert!(pins(&shell).is_empty());
    assert_eq!(
        shell
            .backend
            .jobs_seen()
            .iter()
            .filter(|kind| **kind == JobKind::ObserveHost)
            .count(),
        1
    );
    // An observation is no request of the operation area.
    assert!(shell.requests(cx, |_| true).is_empty());
}

/// The Task 14 lesson: a Check host of one repository, answered after the
/// page moved to another, is never shown under the other.
#[gpui_kit::test]
async fn a_check_of_another_repository_is_never_shown(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    let b = fixture.create_repository("b");
    fixture.publish_over_ssh(&a);
    fixture.publish_over_ssh(&b);
    let shell = hosts(fixture, Some(&a), cx);
    let held = shell.backend.hold_job(JobKind::ObserveHost);
    shell.click("hosts-check", cx);
    held.wait_reached();

    shell.click("settings-close", cx);
    shell.select(&b, cx);
    shell.keys("secondary-,", cx);
    shell.click(("settings-page", 3usize), cx);
    assert_eq!(
        shell.label("hosts-root", cx),
        Some(chosen_line(b.to_str().unwrap()))
    );
    held.release();
    let operations = shell.operations(cx);
    wait_until(cx, &operations, |operations, _| operations.observed_any()).await;
    shell.run(cx);
    assert!(!shell.present("hosts-observation-state", cx));
    assert!(!shell.present("hosts-observation-message", cx));
    let page = page(&shell, cx);
    assert!(page.read_with(cx, |page, _| page.observed_code().is_none()));
}

/// The approval's preview: `prepare` with a known host, algorithm and
/// fingerprint shows it with no connection. Escape closes it; nothing is
/// pinned, nothing is submitted, and focus goes back to Check host.
#[gpui_kit::test]
fn the_approval_preview_shows_the_key_and_escape_pins_nothing(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.publish_over_ssh(&root);
    let shell = hosts(fixture, Some(&root), cx);
    let check = check_focus(&shell, cx);
    shell
        .visual(cx)
        .update(|window, cx| window.focus(&check, cx));
    confirm_trust(&shell, approve(&root, NEW), cx);
    assert_eq!(shell.label("confirm-ok", cx).as_deref(), Some(CONFIRM));
    let details = shell.rows("confirm-detail", cx);
    assert!(
        details.contains(&format!("{REQUESTED_KEY}: {}", key_line(ALGORITHM, NEW))),
        "{details:?}"
    );
    assert!(
        details.contains(&format!("Host: {}", authority_line(SSH_HOST, SSH_PORT))),
        "{details:?}"
    );
    assert!(
        !details.iter().any(|line| line.starts_with(PINNED_KEY)),
        "{details:?}"
    );

    shell.keys("escape", cx);
    assert!(!shell.present("confirm-dialog", cx));
    assert!(pins(&shell).is_empty());
    assert!(shell.requests(cx, |_| true).is_empty());
    assert!(shell.frame(cx, |window, _| check.is_focused(window)));
}

/// The replacement's preview shows the pin it replaces and the key that
/// replaces it, both fingerprints. Escape leaves the old pin.
#[gpui_kit::test]
fn the_replacement_preview_shows_both_fingerprints_and_escape_keeps_the_pin(
    cx: &mut TestAppContext,
) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.publish_over_ssh(&root);
    fixture.pin_host(ALGORITHM, OLD);
    let shell = hosts(fixture, Some(&root), cx);
    confirm_trust(&shell, replace(&root, NEW, OLD), cx);
    assert_eq!(shell.label("confirm-ok", cx).as_deref(), Some(CONFIRM));
    let details = shell.rows("confirm-detail", cx);
    assert!(
        details.contains(&format!("{REQUESTED_KEY}: {}", key_line(ALGORITHM, NEW))),
        "{details:?}"
    );
    assert!(
        details.contains(&format!("{PINNED_KEY}: {}", key_line(ALGORITHM, OLD))),
        "{details:?}"
    );

    shell.keys("escape", cx);
    assert!(!shell.present("confirm-dialog", cx));
    let pinned = pins(&shell);
    assert_eq!(pinned.len(), 1);
    assert_eq!(pinned[0].pinned_fingerprint, OLD);
    assert!(shell.requests(cx, |_| true).is_empty());
}

/// Approving a host that has a pin is refused at its preview as
/// `host_replacement_required`, and the dialog shows both pins, with no
/// Confirm.
#[gpui_kit::test]
fn approving_a_pinned_host_shows_both_pins(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.publish_over_ssh(&root);
    fixture.pin_host(ALGORITHM, OLD);
    let shell = hosts(fixture, Some(&root), cx);
    confirm_trust(&shell, approve(&root, NEW), cx);
    assert_eq!(
        shell.label("confirm-code", cx).as_deref(),
        Some(ResultCode::HostReplacementRequired.as_str())
    );
    assert_eq!(
        shell.label("confirm-requested-key", cx),
        Some(format!("{REQUESTED_KEY}: {}", key_line(ALGORITHM, NEW)))
    );
    assert_eq!(
        shell.label("confirm-pinned-key", cx),
        Some(format!("{PINNED_KEY}: {}", key_line(ALGORITHM, OLD)))
    );
    assert!(!shell.present("confirm-ok", cx));
    shell.keys("escape", cx);
    assert_eq!(pins(&shell)[0].pinned_fingerprint, OLD);
}

/// While a dialog is open the page's handlers do nothing.
#[gpui_kit::test]
fn the_page_acts_on_nothing_while_a_dialog_is_open(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.publish_over_ssh(&root);
    let shell = hosts(fixture, Some(&root), cx);
    confirm_trust(&shell, approve(&root, NEW), cx);
    assert!(shell.present("confirm-dialog", cx));
    let page = page(&shell, cx);
    shell.visual(cx).update(|window, cx| {
        page.update(cx, |page, cx| {
            page.check(window, cx);
            page.confirm_trust(approve(&root, OLD), window, cx);
        })
    });
    shell.run(cx);
    assert!(!shell.backend.jobs_seen().contains(&JobKind::ObserveHost));
    assert!(shell.frame(cx, |window, cx| window.has_active_dialog(cx)));
    shell.keys("escape", cx);
    assert!(!shell.present("confirm-dialog", cx));
}

/// The `host.approve` and `host.replace` recovery: the Hosts page on the
/// root the action names, checking the host it names, with focus on
/// Check host. No transport, so the check answers its typed failure.
#[gpui_kit::test]
async fn the_host_recovery_checks_the_named_host_on_the_hosts_page(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    let b = fixture.create_repository("b");
    fixture.publish_over_ssh(&b);
    let shell = Shell::open(fixture, cx);
    shell.select(&a, cx);
    let oracle = shell.fixture.service().observe_host(
        &b,
        Some(&authority()),
        &mut SessionCredentials::new(NoPassphrase),
    );
    let target = b.clone();
    shell.visual(cx).update(|window, cx| {
        shell.workspace.update(cx, |workspace, cx| {
            workspace.check_host_in_settings(target, authority(), window, cx)
        })
    });
    shell.run(cx);
    assert!(shell.present("settings-hosts", cx));
    assert_eq!(
        shell.label("hosts-root", cx),
        Some(chosen_line(b.to_str().unwrap()))
    );
    until_observed(&shell, cx).await;
    assert_eq!(
        shell.label("hosts-observation-code", cx).as_deref(),
        Some(oracle.code.as_str())
    );
    let check = check_focus(&shell, cx);
    assert!(shell.frame(cx, |window, _| check.is_focused(window)));
}

/// A provider call in flight on a plain thread, as the worker makes one.
fn ask(shell: &Shell, label: &str, cx: &mut TestAppContext) -> mpsc::Receiver<&'static str> {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("key");
    fs::write(&path, b"opaque fixture bytes").unwrap();
    let request = UnlockRequest {
        key_id: SharedKeyId::new(),
        label: label.to_owned(),
        source: KeySourceToken::observe(&path).unwrap(),
        reason: UnlockReason::ProtectedKey,
    };
    let mut provider = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().worker().provider());
    let (done, answer) = mpsc::channel();
    thread::spawn(move || {
        let _directory = directory;
        let answer = match provider.request_passphrase(&request) {
            PassphraseResponse::Supplied(_) => "supplied",
            PassphraseResponse::Cancelled => "cancelled",
            PassphraseResponse::Unavailable => "unavailable",
        };
        let _ = done.send(answer);
    });
    answer
}

/// Carried from Task 11: Check host is started from the page, not from a
/// dialog, so a prompt that comes while it runs gives focus back to Check
/// host when it closes. The prompt comes through the provider bridge from
/// a plain thread while the observation is held on the worker.
#[gpui_kit::test]
async fn focus_returns_to_check_host_after_a_prompt(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.publish_over_ssh(&root);
    let shell = hosts(fixture, Some(&root), cx);
    let held = shell.backend.hold_job(JobKind::ObserveHost);
    shell.click("hosts-check", cx);
    held.wait_reached();
    let check = check_focus(&shell, cx);
    assert!(shell.frame(cx, |window, _| check.is_focused(window)));

    let answer = ask(&shell, "primary", cx);
    wait_until(cx, &shell.workspace, |workspace, _| {
        workspace.passphrase().is_some()
    })
    .await;
    shell.run(cx);
    assert!(shell.present("passphrase-dialog", cx));
    assert!(!shell.frame(cx, |window, _| check.is_focused(window)));

    shell.keys("escape", cx);
    assert_eq!(answer.recv_timeout(WAIT).unwrap(), "cancelled");
    assert!(!shell.present("passphrase-dialog", cx));
    assert!(shell.frame(cx, |window, _| check.is_focused(window)));
    held.release();
    until_observed(&shell, cx).await;
}

/// What a host observation answers when it connected: built here as test
/// input to the desktop's own model (ruling R-T16b), since no desktop test
/// can connect. Production code never builds one.
fn observed(pinned: Option<&str>, presented: &str) -> HostObservationDto {
    HostObservationDto {
        host: SSH_HOST.to_owned(),
        port: SSH_PORT,
        fetch: true,
        push: true,
        presented_algorithm: ALGORITHM.to_owned(),
        presented_fingerprint: presented.to_owned(),
        pinned_algorithm: pinned.map(|_| ALGORITHM.to_owned()),
        pinned_fingerprint: pinned.map(str::to_owned),
        reapproval_required: false,
    }
}

/// The page's verdict on what a host presented, beside its pin.
#[test]
fn the_verdict_compares_the_presented_key_with_the_pin() {
    assert!(matches!(
        super::verdict(&observed(None, NEW)),
        super::Verdict::NotPinned
    ));
    assert!(matches!(
        super::verdict(&observed(Some(NEW), NEW)),
        super::Verdict::Matches
    ));
    assert!(matches!(
        super::verdict(&observed(Some(OLD), NEW)),
        super::Verdict::Different {
            algorithm: ALGORITHM,
            fingerprint: OLD
        }
    ));
    let mut other_algorithm = observed(Some(NEW), NEW);
    other_algorithm.pinned_algorithm = Some("ssh-rsa".to_owned());
    assert!(matches!(
        super::verdict(&other_algorithm),
        super::Verdict::Different { .. }
    ));
}

/// Starts Check host with the worker's observation held, and answers it
/// with `answer` through the request model, as the worker's event would.
/// Gives the hold, which the test keeps until it ends.
fn answer_check(
    shell: &Shell,
    answer: HostObservationDto,
    cx: &mut TestAppContext,
) -> crate::desktop::test_support::delaying::JobGate {
    let held = shell.backend.hold_job(JobKind::ObserveHost);
    shell.click("hosts-check", cx);
    held.wait_reached();
    let id = page(shell, cx)
        .read_with(cx, |page, _| page.observing_id())
        .expect("Check host started");
    let envelope = Envelope::read_success("host observe", Scope::default(), answer);
    shell.operations(cx).update(cx, |operations, cx| {
        operations.apply(WorkerEvent::HostObserved(id, envelope));
        cx.notify();
    });
    shell.run(cx);
    held
}

/// An unpinned host shows its key and Approve, whose preview is of
/// `host approve` with the key presented.
#[gpui_kit::test]
async fn an_unpinned_host_offers_approve_of_the_presented_key(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.publish_over_ssh(&root);
    let shell = hosts(fixture, Some(&root), cx);
    let _held = answer_check(&shell, observed(None, NEW), cx);
    assert_eq!(
        shell.label("hosts-observed-verdict", cx).as_deref(),
        Some(NOT_PINNED)
    );
    assert_eq!(
        shell.label("hosts-observed-fingerprint", cx),
        Some(format!("Fingerprint: {NEW}"))
    );
    assert!(!shell.present("hosts-replace", cx));
    shell.click("hosts-approve", cx);
    assert_eq!(
        shell.label("confirm-command", cx).as_deref(),
        Some(
            Intent::HostApprove {
                root: root.clone(),
                authority: authority(),
                algorithm: String::new(),
                fingerprint: String::new(),
            }
            .label()
        )
    );
    assert_eq!(shell.label("confirm-ok", cx).as_deref(), Some(CONFIRM));
    let details = shell.rows("confirm-detail", cx);
    assert!(
        details.contains(&format!("{REQUESTED_KEY}: {}", key_line(ALGORITHM, NEW))),
        "{details:?}"
    );
    shell.keys("escape", cx);
    assert!(pins(&shell).is_empty());
}

/// A host pinned to another key shows the pin and Replace, whose preview
/// is of `host replace` with the pin as the old key.
#[gpui_kit::test]
async fn a_host_pinned_to_another_key_offers_replace_of_that_pin(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.publish_over_ssh(&root);
    fixture.pin_host(ALGORITHM, OLD);
    let shell = hosts(fixture, Some(&root), cx);
    let _held = answer_check(&shell, observed(Some(OLD), NEW), cx);
    assert_eq!(
        shell.label("hosts-observed-verdict", cx).as_deref(),
        Some(DIFFERENT)
    );
    assert_eq!(
        shell.label("hosts-observed-pinned", cx),
        Some(format!("{PINNED}: {}", key_line(ALGORITHM, OLD)))
    );
    assert!(!shell.present("hosts-approve", cx));
    shell.click("hosts-replace", cx);
    assert_eq!(shell.label("confirm-ok", cx).as_deref(), Some(CONFIRM));
    let details = shell.rows("confirm-detail", cx);
    assert!(
        details.contains(&format!("{REQUESTED_KEY}: {}", key_line(ALGORITHM, NEW))),
        "{details:?}"
    );
    assert!(
        details.contains(&format!("{PINNED_KEY}: {}", key_line(ALGORITHM, OLD))),
        "{details:?}"
    );
    shell.keys("escape", cx);
    assert_eq!(pins(&shell)[0].pinned_fingerprint, OLD);
}

/// A host that presents its pin says so and offers neither control.
#[gpui_kit::test]
async fn a_host_that_presents_its_pin_offers_nothing(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.publish_over_ssh(&root);
    fixture.pin_host(ALGORITHM, OLD);
    let shell = hosts(fixture, Some(&root), cx);
    let _held = answer_check(&shell, observed(Some(OLD), OLD), cx);
    assert_eq!(
        shell.label("hosts-observed-verdict", cx).as_deref(),
        Some(MATCHES)
    );
    assert!(!shell.present("hosts-approve", cx));
    assert!(!shell.present("hosts-replace", cx));
}

/// After a successful approval the answer that offered it goes, with its
/// Approve, and focus lands on Check host, which is drawn. The approval's
/// success is delivered to the request model as its worker event would
/// be (ruling R-T16b): a real one needs a host.
#[gpui_kit::test]
async fn a_successful_approval_clears_the_answer_and_focuses_check_host(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.publish_over_ssh(&root);
    let shell = hosts(fixture, Some(&root), cx);
    let observing = answer_check(&shell, observed(None, NEW), cx);
    // The worker's own answer, which comes after, is not taken: the page
    // keeps the first answer to its Check host.
    observing.release();
    let operations = shell.operations(cx);
    wait_until(cx, &operations, |operations, _| {
        operations.observed_any() && !shell_observed_is_success(operations)
    })
    .await;
    assert_eq!(
        shell.label("hosts-observed-verdict", cx).as_deref(),
        Some(NOT_PINNED)
    );
    let approving = shell.backend.hold_job(JobKind::HostApprove);
    // As a keyboard user has it: focus on Approve when it is pressed.
    let approve = page(&shell, cx).read_with(cx, |page, _| page.control_focus("hosts-approve"));
    shell
        .visual(cx)
        .update(|window, cx| window.focus(&approve, cx));
    shell.click("hosts-approve", cx);
    shell.click("confirm-ok", cx);
    approving.wait_reached();
    shell.run(cx);
    let id = shell.requests(cx, |request| {
        matches!(request.intent, Intent::HostApprove { .. })
    })[0];
    shell.operations(cx).update(cx, |operations, cx| {
        operations.apply(WorkerEvent::Finished(
            id,
            Box::new(Envelope::mutation(
                "host approve",
                Scope::default(),
                Outcome::Success,
                ResultCode::Ok,
            )),
        ));
        cx.notify();
    });
    shell.run(cx);

    assert!(!shell.present("confirm-dialog", cx));
    assert!(!shell.present("hosts-approve", cx));
    assert!(!shell.present("hosts-observed-verdict", cx));
    let check = check_focus(&shell, cx);
    assert!(shell.present("hosts-check", cx));
    assert!(shell.frame(cx, |window, _| check.is_focused(window)));
}

/// Whether the last observation the model holds is a success: false once
/// the worker's own answer, which cannot connect, has replaced the one the
/// test delivered.
fn shell_observed_is_success(operations: &crate::desktop::model::operations::Operations) -> bool {
    operations.last_observation_succeeded()
}
