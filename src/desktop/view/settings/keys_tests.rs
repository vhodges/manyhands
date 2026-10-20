//! The Keys page over a real key store in the fixture's home: the list
//! with label, fingerprint, kind and selection; Generate, unprotected and
//! protected, with the passphrase kept out of everything (Review Focus 5)
//! and both masked inputs emptied after every submit; a mismatched
//! confirmation refused before any job; Retry of a protected generation
//! asking for the passphrase again under the same request; Import, Copy
//! public key, Select, Clear selection and Unregister as user jobs; and
//! Delete of a generated key through the confirmation dialog, with Escape
//! and a stale preview.

use std::{fs, path::Path};

use gpui_kit::{
    Entity, Focusable, TestAppContext,
    component::{WindowExt, input::InputState},
};
use manyhands::{
    repository::{KeyDto, LeaseKind, RepositoryService, SharedKeyId},
    results::{Outcome, RecoveryActionKind, ResultCode},
};

use super::{
    COPIED, KIND_GENERATED, KIND_IMPORTED, KeysPage, MISMATCH, NEED_LABEL, NO_FINGERPRINT, NO_KEYS,
    REFUSED, SELECTED, SELECTION_NONE, retrying_line, selection_line,
};
use crate::desktop::{
    services::{
        KEY_STORE_UNAVAILABLE,
        requests::{DesktopRequestId, Intent, NotSent, RequestState},
    },
    test_support::fixtures::{
        Fixture, PRIVATE_KEY_SENTINEL, select_key_through, unregister_key_through,
    },
    view::{
        confirm::{CONFIRM, PREPARE_AGAIN},
        harness::{Shell, wait_until},
        operations::recovery_id,
    },
};

const SENTINEL: &str = "sentinel-passphrase-7f3a";

const LABEL: &str = "key-generate-label";
const PASSPHRASE: &str = "key-generate-passphrase";
const CONFIRMATION: &str = "key-generate-confirm";
const IMPORT_LABEL: &str = "key-import-label";
const IMPORT_PRIVATE: &str = "key-import-private";
const IMPORT_PUBLIC: &str = "key-import-public";

/// The window over `fixture` with the Keys page shown.
fn keys(fixture: Fixture, cx: &mut TestAppContext) -> Shell {
    let shell = Shell::open(fixture, cx);
    shell.keys("secondary-,", cx);
    shell.click(("settings-page", 2usize), cx);
    assert!(shell.present("settings-keys", cx));
    shell
}

fn page(shell: &Shell, cx: &mut TestAppContext) -> Entity<KeysPage> {
    let host = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.settings.clone());
    host.read_with(cx, |host, _| host.keys.clone())
        .expect("the Keys page is made")
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

/// Types `text` into the field `id`, which takes focus first.
fn type_into(shell: &Shell, id: &str, text: &str, cx: &mut TestAppContext) {
    let input = input(shell, id, cx);
    shell.visual(cx).update(|window, cx| {
        input.update(cx, |input, cx| input.focus(window, cx));
    });
    shell.visual(cx).simulate_input(text);
    shell.run(cx);
}

/// The keys the library lists: the oracle.
fn listed(shell: &Shell) -> Vec<KeyDto> {
    shell.fixture.service().list_keys().unwrap().items
}

fn id_of(key: &KeyDto) -> SharedKeyId {
    SharedKeyId::parse(&key.id).unwrap()
}

fn of(shell: &Shell, cx: &mut TestAppContext, pick: fn(&Intent) -> bool) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| pick(&request.intent))
}

fn generations(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    of(shell, cx, |intent| {
        matches!(intent, Intent::KeyGenerate { .. })
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

/// Waits until the page lists `count` keys.
async fn until_count(shell: &Shell, count: usize, cx: &mut TestAppContext) {
    let page = page(shell, cx);
    wait_until(cx, &page, move |page, _| page.key_labels().len() == count).await;
}

/// The page's row of the key labeled `label`.
fn row(shell: &Shell, label: &str, cx: &mut TestAppContext) -> usize {
    shell
        .rows("key-label", cx)
        .iter()
        .position(|shown| shown == label)
        .unwrap_or_else(|| panic!("{label} is not listed"))
}

/// Fills the Generate form and presses Generate key.
fn generate(
    shell: &Shell,
    label: &str,
    passphrase: &str,
    confirmation: &str,
    cx: &mut TestAppContext,
) {
    set(shell, LABEL, label, cx);
    if !passphrase.is_empty() {
        type_into(shell, PASSPHRASE, passphrase, cx);
    }
    if !confirmation.is_empty() {
        type_into(shell, CONFIRMATION, confirmation, cx);
    }
    shell.click("key-generate-submit", cx);
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

/// Every field of every request the session holds.
fn request_fields(shell: &Shell, cx: &mut TestAppContext) -> Vec<String> {
    let operations = shell.operations(cx);
    operations.read_with(cx, |operations, _| {
        let mut fields = Vec::new();
        for request in operations.requests() {
            fields.push(format!("{:?}", request.id));
            fields.push(request.request_id.to_string());
            fields.push(request.intent.label().to_owned());
            fields.extend(request.intent.root().map(|root| root.display().to_string()));
            fields.extend(request.confirmation.map(|id| id.to_string()));
            fields.push(format!("{:?} {}", request.lane, request.attempt));
            if let Intent::KeyGenerate { label, protected } = &request.intent {
                fields.push(format!("{label} {protected}"));
            }
            match &request.state {
                RequestState::Finished(outcome, code, effects, recovery, message) => {
                    fields.push(format!("{outcome:?} {code:?} {effects:?} {recovery:?}"));
                    fields.push(message.clone());
                }
                RequestState::NotSent(why) => fields.push(format!("{why:?}")),
                _ => {}
            }
        }
        fields
    })
}

fn is_encrypted(path: &Path) -> bool {
    let text = fs::read_to_string(path).unwrap();
    ssh_key::PrivateKey::from_openssh(text)
        .unwrap()
        .is_encrypted()
}

/// Review Focus 5: an unprotected and a protected key are generated and
/// both listed, as the library lists them. The protected one's passphrase
/// reaches no request field, no operation-area text, no notification,
/// nothing the window shows and nothing the page holds, and both masked
/// inputs are empty after each submit.
#[gpui_kit::test]
async fn generated_keys_are_listed_and_the_passphrase_is_kept_nowhere(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let shell = keys(fixture, cx);
    assert_eq!(shell.label("keys-state", cx).as_deref(), Some(NO_KEYS));

    generate(&shell, "plain", "", "", cx);
    let plain = generations(&shell, cx)[0];
    shell.until_ended(plain, cx).await;
    assert_eq!(
        finished(&shell, plain, cx),
        (Outcome::Success, ResultCode::Ok)
    );
    until_count(&shell, 1, cx).await;

    generate(&shell, "locked", SENTINEL, SENTINEL, cx);
    // Masked and emptied: not even the inputs hold what was typed.
    assert_eq!(value(&shell, PASSPHRASE, cx), "");
    assert_eq!(value(&shell, CONFIRMATION, cx), "");
    assert_eq!(value(&shell, LABEL, cx), "");
    let locked = generations(&shell, cx)[0];
    assert_ne!(locked, plain);
    shell.until_ended(locked, cx).await;
    assert_eq!(
        finished(&shell, locked, cx),
        (Outcome::Success, ResultCode::Ok)
    );
    until_count(&shell, 2, cx).await;

    let oracle = listed(&shell);
    let labels: Vec<String> = oracle.iter().map(|key| key.label.clone()).collect();
    assert_eq!(labels, ["plain", "locked"]);
    assert_eq!(shell.rows("key-label", cx), labels);
    for (ix, key) in oracle.iter().enumerate() {
        assert_eq!(
            shell.label(("key-fingerprint", ix), cx),
            Some(
                key.fingerprint
                    .clone()
                    .unwrap_or_else(|| NO_FINGERPRINT.to_owned())
            )
        );
        assert_eq!(
            shell.label(("key-kind", ix), cx).as_deref(),
            Some(KIND_GENERATED)
        );
    }
    assert!(!is_encrypted(Path::new(&oracle[0].private_key_path)));
    assert!(is_encrypted(Path::new(&oracle[1].private_key_path)));
    let protected = shell.operations(cx).read_with(cx, |operations, _| {
        matches!(
            operations.request(locked).unwrap().intent,
            Intent::KeyGenerate {
                protected: true,
                ..
            }
        )
    });
    assert!(protected);

    let fields = request_fields(&shell, cx);
    assert!(!fields.is_empty());
    assert!(
        fields.iter().all(|field| !field.contains(SENTINEL)),
        "{fields:?}"
    );
    let page = page(&shell, cx);
    let held = page.read_with(cx, |page, cx| page.held_text(cx));
    assert!(!held.is_empty());
    assert!(held.iter().all(|text| !text.contains(SENTINEL)), "{held:?}");
    let shown = window_text(&shell, cx);
    assert!(
        shown.iter().all(|text| !text.contains(SENTINEL)),
        "{shown:?}"
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

/// A confirmation that differs from the passphrase is refused before any
/// job, and both masked inputs are emptied; so is a confirmation with no
/// passphrase, and a passphrase holding NUL. The label stays.
#[gpui_kit::test]
fn a_mismatched_confirmation_is_refused_before_any_job(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let shell = keys(fixture, cx);

    generate(&shell, "locked", SENTINEL, "sentinel-passphrase-7f3b", cx);
    assert_eq!(
        shell.label("key-generate-message", cx).as_deref(),
        Some(MISMATCH)
    );
    assert_eq!(value(&shell, PASSPHRASE, cx), "");
    assert_eq!(value(&shell, CONFIRMATION, cx), "");
    assert_eq!(value(&shell, LABEL, cx), "locked");
    assert!(shell.requests(cx, |_| true).is_empty());

    type_into(&shell, CONFIRMATION, SENTINEL, cx);
    shell.click("key-generate-submit", cx);
    assert_eq!(
        shell.label("key-generate-message", cx).as_deref(),
        Some(MISMATCH)
    );
    assert_eq!(value(&shell, CONFIRMATION, cx), "");
    assert!(shell.requests(cx, |_| true).is_empty());

    set(&shell, PASSPHRASE, "a\0b", cx);
    set(&shell, CONFIRMATION, "a\0b", cx);
    shell.click("key-generate-submit", cx);
    assert_eq!(
        shell.label("key-generate-message", cx).as_deref(),
        Some(REFUSED)
    );
    assert_eq!(value(&shell, PASSPHRASE, cx), "");
    assert_eq!(value(&shell, CONFIRMATION, cx), "");
    assert!(shell.requests(cx, |_| true).is_empty());

    set(&shell, LABEL, "", cx);
    shell.click("key-generate-submit", cx);
    assert_eq!(
        shell.label("key-generate-message", cx).as_deref(),
        Some(NEED_LABEL)
    );
    assert!(shell.requests(cx, |_| true).is_empty());
    assert!(listed(&shell).is_empty());
}

/// Holds the key registry's write lease, as another call would: a key
/// generation answers `busy`.
fn hold_registry(shell: &Shell) -> manyhands::repository::LeaseHolderForTesting {
    RepositoryService::hold_lease_for_testing(
        shell.fixture.home(),
        shell.fixture.data_dir(),
        LeaseKind::CacheWrite,
    )
    .unwrap()
}

fn request_id_of(
    shell: &Shell,
    id: DesktopRequestId,
    cx: &mut TestAppContext,
) -> (manyhands::repository::RequestId, u32) {
    shell.operations(cx).read_with(cx, |operations, _| {
        let request = operations.request(id).unwrap();
        (request.request_id, request.attempt)
    })
}

/// A protected generation answered `busy` is not retried by itself: the
/// passphrase went with its attempt. The page's Retry asks for the
/// passphrase again, refuses a mismatch, and then retries under the same
/// request ID; the key is made, protected, and nothing holds the
/// passphrase.
#[gpui_kit::test]
async fn retry_of_a_protected_generation_asks_for_the_passphrase_again(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let shell = keys(fixture, cx);
    let lease = hold_registry(&shell);

    generate(&shell, "locked", SENTINEL, SENTINEL, cx);
    let id = generations(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    assert_eq!(finished(&shell, id, cx).1, ResultCode::Busy);
    let (request_id, attempt) = request_id_of(&shell, id, cx);
    assert_eq!(attempt, 1);
    drop(lease);

    shell.click("keys-retry", cx);
    let page = page(&shell, cx);
    assert_eq!(page.read_with(cx, |page, _| page.retrying()), Some(id));
    assert_eq!(
        shell.label("key-generate-retrying", cx),
        Some(retrying_line("locked"))
    );
    // A mismatch is refused here too, and nothing is retried.
    type_into(&shell, PASSPHRASE, SENTINEL, cx);
    shell.click("key-generate-submit", cx);
    assert_eq!(
        shell.label("key-generate-message", cx).as_deref(),
        Some(MISMATCH)
    );
    assert_eq!(request_id_of(&shell, id, cx).1, 1);

    type_into(&shell, PASSPHRASE, SENTINEL, cx);
    type_into(&shell, CONFIRMATION, SENTINEL, cx);
    shell.click("key-generate-submit", cx);
    assert_eq!(value(&shell, PASSPHRASE, cx), "");
    assert_eq!(value(&shell, CONFIRMATION, cx), "");
    shell
        .until(cx, move |operations| {
            operations.request(id).is_some_and(|request| {
                request.attempt == 2 && matches!(request.state, RequestState::Finished(..))
            })
        })
        .await;
    assert_eq!(finished(&shell, id, cx), (Outcome::Success, ResultCode::Ok));
    assert_eq!(request_id_of(&shell, id, cx), (request_id, 2));
    assert_eq!(generations(&shell, cx), [id]);
    assert_eq!(page.read_with(cx, |page, _| page.retrying()), None);
    until_count(&shell, 1, cx).await;
    assert!(is_encrypted(Path::new(&listed(&shell)[0].private_key_path)));

    let fields = request_fields(&shell, cx);
    assert!(
        fields.iter().all(|field| !field.contains(SENTINEL)),
        "{fields:?}"
    );
    let held = page.read_with(cx, |page, cx| page.held_text(cx));
    assert!(held.iter().all(|text| !text.contains(SENTINEL)), "{held:?}");
}

/// A retry that reaches a protected generation without its passphrase,
/// as the operation area's Retry does, ends `NotSent(SecretRequired)`;
/// the page then shows the passphrase fields again, and its submit
/// retries the same request.
#[gpui_kit::test]
async fn a_generation_not_sent_for_its_passphrase_shows_the_fields_again(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let shell = keys(fixture, cx);
    let lease = hold_registry(&shell);
    generate(&shell, "locked", SENTINEL, SENTINEL, cx);
    let id = generations(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    drop(lease);
    let (request_id, _) = request_id_of(&shell, id, cx);

    let worker = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().worker().clone());
    worker.retry(id, None);
    shell
        .until(cx, move |operations| {
            operations.request(id).is_some_and(|request| {
                matches!(
                    request.state,
                    RequestState::NotSent(NotSent::SecretRequired)
                )
            })
        })
        .await;
    let page = page(&shell, cx);
    wait_until(cx, &page, move |page, _| page.retrying() == Some(id)).await;
    assert_eq!(
        shell.label("key-generate-retrying", cx),
        Some(retrying_line("locked"))
    );

    type_into(&shell, PASSPHRASE, SENTINEL, cx);
    type_into(&shell, CONFIRMATION, SENTINEL, cx);
    shell.press("enter", cx);
    shell
        .until(cx, move |operations| {
            operations.request(id).is_some_and(|request| {
                request.attempt == 3 && matches!(request.state, RequestState::Finished(..))
            })
        })
        .await;
    assert_eq!(finished(&shell, id, cx), (Outcome::Success, ResultCode::Ok));
    assert_eq!(request_id_of(&shell, id, cx).0, request_id);
    assert_eq!(value(&shell, PASSPHRASE, cx), "");
    until_count(&shell, 1, cx).await;
}

/// Copy public key puts exactly the key's public text on the clipboard,
/// for a generated key and for an imported one. No control shows or
/// copies private material.
#[gpui_kit::test]
async fn copy_public_key_copies_exactly_the_public_key_text(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    fixture.generate_key("made");
    let shell = keys(fixture, cx);
    let (private, public) = shell.fixture.external_key_files("mine");
    set(&shell, IMPORT_LABEL, "mine", cx);
    set(&shell, IMPORT_PRIVATE, &private.to_string_lossy(), cx);
    set(&shell, IMPORT_PUBLIC, &public.to_string_lossy(), cx);
    shell.click("key-import-submit", cx);
    let import = of(&shell, cx, |intent| {
        matches!(intent, Intent::KeyImport { .. })
    })[0];
    shell.until_ended(import, cx).await;
    assert_eq!(
        finished(&shell, import, cx),
        (Outcome::Success, ResultCode::Ok)
    );
    until_count(&shell, 2, cx).await;

    for key in listed(&shell) {
        let ix = row(&shell, &key.label, cx);
        let wanted = shell
            .fixture
            .service()
            .public_key_text(id_of(&key))
            .unwrap()
            .public_key;
        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("before".to_owned()));
        shell.click(("key-copy", ix), cx);
        let copied = cx.read_from_clipboard().and_then(|item| item.text());
        assert_eq!(copied.as_deref(), Some(wanted.as_str()), "{}", key.label);
        assert_eq!(shell.label("keys-copied", cx).as_deref(), Some(COPIED));
    }
    let shown = window_text(&shell, cx);
    for text in &shown {
        assert!(!text.contains(PRIVATE_KEY_SENTINEL), "{text}");
        assert!(!text.contains("PRIVATE KEY"), "{text}");
    }
    let held = page(&shell, cx).read_with(cx, |page, cx| page.held_text(cx));
    assert!(held.iter().all(|text| !text.contains(PRIVATE_KEY_SENTINEL)));
}

/// An imported key is listed as imported and offers no Delete.
/// Unregister removes its registration and leaves its files.
#[gpui_kit::test]
async fn an_imported_key_offers_no_delete_and_unregister_leaves_its_files(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let shell = keys(fixture, cx);
    let (private, public) = shell.fixture.external_key_files("mine");
    set(&shell, IMPORT_LABEL, "mine", cx);
    set(&shell, IMPORT_PRIVATE, &private.to_string_lossy(), cx);
    type_into(&shell, IMPORT_PUBLIC, &public.to_string_lossy(), cx);
    shell.press("enter", cx);
    let import = of(&shell, cx, |intent| {
        matches!(intent, Intent::KeyImport { .. })
    })[0];
    assert_eq!(value(&shell, IMPORT_PRIVATE, cx), "");
    shell.until_ended(import, cx).await;
    until_count(&shell, 1, cx).await;
    let oracle = listed(&shell);
    assert_eq!(
        shell.label(("key-kind", 0usize), cx).as_deref(),
        Some(KIND_IMPORTED)
    );
    assert_eq!(
        shell.label(("key-fingerprint", 0usize), cx),
        oracle[0].fingerprint.clone()
    );
    assert!(shell.present(("key-unregister", 0usize), cx));
    assert!(!shell.present(("key-delete", 0usize), cx));

    shell.click(("key-unregister", 0usize), cx);
    assert!(!shell.present("confirm-dialog", cx));
    let id = of(&shell, cx, |intent| {
        matches!(intent, Intent::KeyRemove { .. })
    })[0];
    shell.until_ended(id, cx).await;
    assert_eq!(finished(&shell, id, cx), (Outcome::Success, ResultCode::Ok));
    until_count(&shell, 0, cx).await;
    assert!(listed(&shell).is_empty());
    assert!(private.exists() && public.exists());
}

/// Import with a relative private key path is answered with the library's
/// typed failure; with no label or no private key path it asks nothing.
#[gpui_kit::test]
async fn an_import_the_library_refuses_shows_its_typed_failure(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let shell = keys(fixture, cx);
    shell.click("key-import-submit", cx);
    assert!(shell.present("key-import-message", cx));
    assert!(shell.requests(cx, |_| true).is_empty());

    set(&shell, IMPORT_LABEL, "mine", cx);
    set(&shell, IMPORT_PRIVATE, "relative/key", cx);
    shell.click("key-import-submit", cx);
    let id = of(&shell, cx, |intent| {
        matches!(intent, Intent::KeyImport { .. })
    })[0];
    shell.until_ended(id, cx).await;
    let (outcome, code) = finished(&shell, id, cx);
    assert_ne!(outcome, Outcome::Success);
    assert_eq!(
        shell.label("keys-result-code", cx).as_deref(),
        Some(code.as_str())
    );
    assert!(listed(&shell).is_empty());
}

/// Select shows the key selected, as the library lists it; Clear
/// selection shows none selected.
#[gpui_kit::test]
async fn select_and_clear_selection(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    fixture.generate_key("one");
    fixture.generate_key("two");
    let shell = keys(fixture, cx);
    assert_eq!(
        shell.label("keys-selection", cx).as_deref(),
        Some(SELECTION_NONE)
    );
    let ix = row(&shell, "two", cx);
    shell.click(("key-select", ix), cx);
    let id = of(&shell, cx, |intent| {
        matches!(intent, Intent::KeySelect { .. })
    })[0];
    shell.until_ended(id, cx).await;
    assert_eq!(finished(&shell, id, cx), (Outcome::Success, ResultCode::Ok));
    let page = page(&shell, cx);
    wait_until(cx, &page, |page, _| page.selected_label() == Some("two")).await;
    assert!(listed(&shell)[ix].selected);
    assert_eq!(
        shell.label(("key-selected", ix), cx).as_deref(),
        Some(SELECTED)
    );
    assert_eq!(
        shell.label("keys-selection", cx),
        Some(selection_line("two"))
    );

    shell.click("keys-clear", cx);
    let id = of(&shell, cx, |intent| matches!(intent, Intent::KeyClear))[0];
    shell.until_ended(id, cx).await;
    assert_eq!(finished(&shell, id, cx), (Outcome::Success, ResultCode::Ok));
    wait_until(cx, &page, |page, _| page.selected_label().is_none()).await;
    assert!(listed(&shell).iter().all(|key| !key.selected));
    assert_eq!(
        shell.label("keys-selection", cx).as_deref(),
        Some(SELECTION_NONE)
    );
}

/// Delete of a generated key: the preview deletes nothing; Confirm
/// removes its files and its registration.
#[gpui_kit::test]
async fn deleting_a_generated_key_removes_its_files_after_confirmation(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    fixture.generate_key("made");
    let shell = keys(fixture, cx);
    let key = listed(&shell).remove(0);
    let private = Path::new(&key.private_key_path).to_owned();
    let public = Path::new(key.public_key_path.as_deref().unwrap()).to_owned();

    shell.click(("key-delete", 0usize), cx);
    assert_eq!(shell.label("confirm-ok", cx).as_deref(), Some(CONFIRM));
    assert!(private.exists() && public.exists());
    let deletes = |shell: &Shell, cx: &mut TestAppContext| {
        of(shell, cx, |intent| {
            matches!(intent, Intent::KeyDelete { .. })
        })
    };
    assert!(deletes(&shell, cx).is_empty());

    shell.click("confirm-ok", cx);
    let id = deletes(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    assert_eq!(finished(&shell, id, cx), (Outcome::Success, ResultCode::Ok));
    assert!(!private.exists() && !public.exists());
    until_count(&shell, 0, cx).await;
    assert!(listed(&shell).is_empty());
    assert_eq!(shell.label("keys-state", cx).as_deref(), Some(NO_KEYS));
}

/// Escape closes the deletion's preview: the files stay, no request is
/// made, and focus goes back to Delete.
#[gpui_kit::test]
fn escape_on_a_delete_preview_leaves_the_files(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    fixture.generate_key("made");
    let shell = keys(fixture, cx);
    let key = listed(&shell).remove(0);
    let delete = page(&shell, cx).read_with(cx, |page, _| page.control_focus("key-delete-0"));
    shell
        .visual(cx)
        .update(|window, cx| window.focus(&delete, cx));
    shell.click(("key-delete", 0usize), cx);
    assert!(shell.present("confirm-ok", cx));

    shell.keys("escape", cx);
    assert!(!shell.present("confirm-dialog", cx));
    assert!(Path::new(&key.private_key_path).exists());
    assert!(Path::new(key.public_key_path.as_deref().unwrap()).exists());
    assert!(shell.requests(cx, |_| true).is_empty());
    assert_eq!(listed(&shell).len(), 1);
    assert!(shell.frame(cx, |window, _| delete.is_focused(window)));
}

/// A stale preview: a second actor selects the key between the preview
/// and Confirm. The answer offers Prepare again, and the files stay.
#[gpui_kit::test]
async fn a_key_changed_since_its_delete_preview_offers_prepare_again(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let id = fixture.generate_key("made");
    let shell = keys(fixture, cx);
    let key = listed(&shell).remove(0);
    shell.click(("key-delete", 0usize), cx);
    assert!(shell.present("confirm-ok", cx));
    let other = RepositoryService::open_at(shell.fixture.data_dir()).unwrap();
    select_key_through(&other, id);

    shell.click("confirm-ok", cx);
    let request = of(&shell, cx, |intent| {
        matches!(intent, Intent::KeyDelete { .. })
    })[0];
    shell.until_ended(request, cx).await;
    let (_, code) = finished(&shell, request, cx);
    assert_ne!(code, ResultCode::Ok);
    assert_eq!(
        shell.label("confirm-prepare-again", cx).as_deref(),
        Some(PREPARE_AGAIN),
        "{}",
        code.as_str()
    );
    assert!(Path::new(&key.private_key_path).exists());
    assert_eq!(listed(&shell).len(), 1);
}

/// A row of a key the list no longer gives acts on nothing: no copy, no
/// selection, no unregistration and no preview.
#[gpui_kit::test]
fn a_row_of_a_key_no_longer_listed_acts_on_nothing(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let id = fixture.generate_key("made");
    let shell = keys(fixture, cx);
    assert_eq!(shell.rows("key-label", cx), ["made"]);
    unregister_key_through(shell.fixture.service(), id);
    let page = page(&shell, cx);
    page.update(cx, |page, cx| page.opened(cx));
    shell.run(cx);
    assert!(shell.rows("key-label", cx).is_empty());

    cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("before".to_owned()));
    shell.visual(cx).update(|window, cx| {
        page.update(cx, |page, cx| {
            page.copy(id, window, cx);
            page.submit_select(id, window, cx);
            page.submit_unregister(id, window, cx);
            page.submit_delete(id, 0, window, cx);
        })
    });
    shell.run(cx);
    assert!(shell.requests(cx, |_| true).is_empty());
    assert!(!shell.present("confirm-dialog", cx));
    let copied = cx.read_from_clipboard().and_then(|item| item.text());
    assert_eq!(copied.as_deref(), Some("before"));
}

/// While a dialog is open, the page's handlers do nothing.
#[gpui_kit::test]
fn the_page_acts_on_nothing_while_a_dialog_is_open(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let id = fixture.generate_key("made");
    let shell = keys(fixture, cx);
    set(&shell, LABEL, "other", cx);
    set(&shell, IMPORT_LABEL, "mine", cx);
    set(&shell, IMPORT_PRIVATE, "/nowhere/key", cx);
    shell.click(("key-delete", 0usize), cx);
    assert!(shell.present("confirm-ok", cx));

    let page = page(&shell, cx);
    shell.visual(cx).update(|window, cx| {
        page.update(cx, |page, cx| {
            page.submit_generate(window, cx);
            page.submit_import(window, cx);
            page.submit_select(id, window, cx);
            page.submit_clear(window, cx);
            page.submit_unregister(id, window, cx);
            page.submit_delete(id, 0, window, cx);
        })
    });
    shell.run(cx);
    assert!(shell.requests(cx, |_| true).is_empty());
    assert_eq!(value(&shell, LABEL, cx), "other");
    shell.keys("escape", cx);
    assert!(!shell.present("confirm-dialog", cx));
}

/// With no key store to be found, the page says so in fixed text.
#[gpui_kit::test]
fn an_unavailable_key_store_is_said_in_fixed_text(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    fs::remove_dir_all(fixture.home()).unwrap();
    let shell = keys(fixture, cx);
    assert_eq!(
        shell.label("keys-store-unavailable", cx).as_deref(),
        Some(KEY_STORE_UNAVAILABLE)
    );
}

/// The operation area's Retry of a protected generation sends nothing
/// without the passphrase: it shows the Keys page asking for it, with
/// focus in the passphrase field, and the page's submit retries the same
/// request.
#[gpui_kit::test]
async fn the_operation_area_retry_of_a_protected_generation_asks_on_the_page(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let shell = keys(fixture, cx);
    let lease = hold_registry(&shell);
    generate(&shell, "locked", SENTINEL, SENTINEL, cx);
    let id = generations(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    drop(lease);
    let (request_id, _) = request_id_of(&shell, id, cx);
    let retry = shell.operations(cx).read_with(cx, |operations, _| {
        let RequestState::Finished(_, _, _, recovery, _) = &operations.request(id).unwrap().state
        else {
            unreachable!()
        };
        recovery
            .iter()
            .position(|action| action.action == RecoveryActionKind::RequestRetry)
            .expect("a busy generation offers its retry")
    });
    shell.click("settings-close", cx);
    shell.keys("secondary-4", cx);
    shell.click(recovery_id(id, retry), cx);

    assert!(shell.present("settings-keys", cx));
    let page = page(&shell, cx);
    assert_eq!(page.read_with(cx, |page, _| page.retrying()), Some(id));
    // Nothing was sent without the passphrase.
    assert_eq!(request_id_of(&shell, id, cx).1, 1);
    let field = input(&shell, PASSPHRASE, cx);
    let focused = shell.frame(cx, |window, cx| {
        field.read(cx).focus_handle(cx).is_focused(window)
    });
    assert!(focused);

    shell.visual(cx).simulate_input(SENTINEL);
    type_into(&shell, CONFIRMATION, SENTINEL, cx);
    shell.press("enter", cx);
    shell
        .until(cx, move |operations| {
            operations.request(id).is_some_and(|request| {
                request.attempt == 2 && matches!(request.state, RequestState::Finished(..))
            })
        })
        .await;
    assert_eq!(finished(&shell, id, cx), (Outcome::Success, ResultCode::Ok));
    assert_eq!(request_id_of(&shell, id, cx).0, request_id);
}
