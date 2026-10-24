//! The privacy sweep (Cycle contract 7, design Testing: Privacy): a
//! sentinel planted in a ticket draft's body is driven through restore, a
//! refresh, a save the library refuses as `invalid_input`, a stale save,
//! export (written and failed), the Discard and close dialogs, a failing
//! flush and the drafts-not-saved dialog on quit. Then every surface that
//! may show text is read back from the window's own models and frame: the
//! operation model's entries and their envelope messages, everything the
//! window draws outside the draft's own section, its notifications, each
//! dialog's message, and the `Debug` of every `DraftError` produced.
//!
//! Draft types derive no `Debug`, so `{:?}` applies to `DraftError` only.

use gpui_kit::{ElementId, TestAppContext, component::WindowExt};
use manyhands::{
    drafts::{DraftError, DraftErrorKind, DraftFields, FailureAction, FailurePoint},
    repository::ItemDtoKind,
    results::ResultCode,
};

use super::tests::{ONE, open, registry, ticket, with_ticket};
use crate::desktop::{
    model::tabs::TabKey,
    services::requests::{DesktopRequestId, Intent, RequestState},
    test_support::{drafts::store_draft, fixtures::Fixture},
    view::harness::Shell,
};

const SENTINEL: &str = "SENTINEL-D2-PRIVACY-5c7e";

/// The session's fields with `ticket_type` and `body`.
fn fields(current: &DraftFields, ticket_type: &str, body: &str) -> DraftFields {
    let mut fields = current.clone();
    if let DraftFields::Ticket {
        ticket_type: kind,
        body: text,
        ..
    } = &mut fields
    {
        *kind = ticket_type.to_owned();
        *text = body.to_owned();
    }
    fields
}

/// Every label the window draws outside the draft's own section of its
/// tab, where its text belongs.
fn drawn_outside_the_section(shell: &Shell, cx: &mut TestAppContext) -> Vec<String> {
    let section = ElementId::from("draft-section");
    shell.frame(cx, |window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .filter(|snapshot| !snapshot.path().contains(&section))
            .filter_map(|snapshot| snapshot.label().map(str::to_owned))
            .collect()
    })
}

/// Every operation entry: its command's label and, once finished, the
/// envelope message the operation area shows.
fn operation_entries(shell: &Shell, cx: &mut TestAppContext) -> Vec<String> {
    shell.operations(cx).read_with(cx, |operations, _| {
        operations
            .requests()
            .iter()
            .flat_map(|request| {
                let mut texts = vec![request.intent.label().to_owned()];
                if let RequestState::Finished(.., message) = &request.state {
                    texts.push(message.clone());
                }
                texts
            })
            .collect()
    })
}

fn code(shell: &Shell, id: DesktopRequestId, cx: &mut TestAppContext) -> Option<ResultCode> {
    shell
        .operations(cx)
        .read_with(cx, |operations, _| match &operations.request(id)?.state {
            RequestState::Finished(_, code, ..) => Some(*code),
            _ => None,
        })
}

/// The envelope message of the finished request `id`.
fn message(shell: &Shell, id: DesktopRequestId, cx: &mut TestAppContext) -> String {
    shell.operations(cx).read_with(cx, |operations, _| {
        match &operations.request(id).map(|request| &request.state) {
            Some(RequestState::Finished(.., message)) => message.clone(),
            _ => panic!("the request has finished"),
        }
    })
}

/// Everything the sweep read, by where it was read.
#[derive(Default)]
struct Seen(Vec<(&'static str, String)>);

impl Seen {
    fn add(&mut self, surface: &'static str, texts: impl IntoIterator<Item = String>) {
        self.0.extend(texts.into_iter().map(|text| (surface, text)));
    }

    /// The window's surfaces as they stand now.
    fn window(&mut self, shell: &Shell, cx: &mut TestAppContext) {
        self.add("operation entry", operation_entries(shell, cx));
        self.add("drawn", drawn_outside_the_section(shell, cx));
        let notifications = shell.frame(cx, |window, cx| window.notifications(cx).len());
        assert_eq!(notifications, 0, "the desktop shows no notification");
    }

    fn surfaces(&self, surface: &str) -> usize {
        self.0.iter().filter(|(at, _)| *at == surface).count()
    }
}

#[gpui_kit::test]
async fn a_draft_body_is_in_no_entry_message_notification_dialog_or_error(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let a = with_ticket(&fixture, "a", ONE, "One");
    let body = format!("{SENTINEL} body");
    store_draft(&fixture, &a, ItemDtoKind::Ticket, ONE, &body);
    let shell = Shell::open(fixture, cx);
    let mut seen = Seen::default();
    let mut errors: Vec<DraftError> = Vec::new();

    // Restore: the draft's text is in its section, and only there.
    open(&shell, ticket(&a, ONE), cx);
    shell.click("draft-restore-button", cx);
    let drafts = registry(&shell, cx);
    let tab = TabKey::for_target(&ticket(&a, ONE));
    let id = drafts
        .read_with(cx, |registry, _| registry.attached(&tab))
        .expect("the draft is restored");
    assert_eq!(
        shell.label("draft-body", cx).as_deref(),
        Some(body.as_str())
    );
    // The scan sees drawn text: the section is all that is left out.
    let everything: Vec<String> = shell.frame(cx, |window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .filter_map(|snapshot| snapshot.label().map(str::to_owned))
            .collect()
    });
    assert!(everything.iter().any(|text| text.contains(SENTINEL)));
    seen.window(&shell, cx);

    // Refresh.
    shell.select(&a, cx);
    shell.keys("secondary-r", cx);
    let refresh = shell.requests(cx, |request| {
        matches!(request.intent, Intent::IndexRefresh { .. })
    })[0];
    shell.until_ended(refresh, cx).await;
    assert_eq!(code(&shell, refresh, cx), Some(ResultCode::Ok));
    seen.window(&shell, cx);

    // A save the library refuses as `invalid_input`: an empty type.
    let current = drafts.read_with(cx, |registry, _| {
        registry.session(id).unwrap().current().clone()
    });
    let invalid = fields(&current, "", &format!("{SENTINEL} invalid"));
    drafts.update(cx, |registry, cx| registry.edit(id, invalid, cx));
    let saved = drafts.update(cx, |registry, cx| registry.save(id, cx));
    shell.run(cx);
    let request = saved.await.expect("the save is sent");
    shell.until_ended(request, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::InvalidInput));
    let refused = message(&shell, request, cx);
    assert!(!refused.is_empty(), "the refusal says why");
    seen.add("envelope message", [refused]);
    seen.window(&shell, cx);

    // A stale save: the item was saved since the draft's base.
    let valid = fields(&current, "task", &format!("{SENTINEL} stale"));
    drafts.update(cx, |registry, cx| registry.edit(id, valid, cx));
    shell.fixture.save_ticket(&a, ONE, "Theirs");
    let saved = drafts.update(cx, |registry, cx| registry.save(id, cx));
    shell.run(cx);
    let request = saved.await.expect("the save is sent");
    shell.until_ended(request, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::ExternalChange));
    seen.add("envelope message", [message(&shell, request, cx)]);
    seen.window(&shell, cx);

    // Export…, written, then to a folder that does not exist.
    let folder = tempfile::tempdir().unwrap();
    for path in [
        folder.path().join("One.md"),
        folder.path().join("missing").join("One.md"),
    ] {
        shell.click("draft-export-button", cx);
        cx.simulate_new_path_selection(move |_| Some(path));
        shell.run(cx);
        let result = shell.label("draft-export-result", cx).expect("said");
        seen.add("export result", [result]);
    }
    let written = std::fs::read_to_string(folder.path().join("One.md")).unwrap();
    assert!(written.contains(SENTINEL), "the export holds the text");
    assert!(
        seen.0
            .iter()
            .any(|(at, text)| *at == "export result"
                && text.starts_with(super::actions::NOT_EXPORTED))
    );
    seen.window(&shell, cx);

    // The Discard and close dialogs, each closed by Escape.
    shell.click("draft-discard-button", cx);
    let message = shell.label("discard-draft-message", cx).expect("asked");
    seen.add("dialog message", [message]);
    seen.window(&shell, cx);
    shell.keys("escape", cx);
    shell.keys("secondary-3 secondary-w", cx);
    let message = shell.label("close-draft-message", cx).expect("asked");
    seen.add("dialog message", [message]);
    seen.window(&shell, cx);
    shell.keys("escape", cx);
    assert!(
        !shell
            .visual(cx)
            .update(|window, cx| window.has_active_dialog(cx))
    );

    // A failing flush: the store's own failure point.
    let failing = fields(&current, "task", &format!("{SENTINEL} unflushed"));
    drafts.update(cx, |registry, cx| registry.edit(id, failing, cx));
    let _failing = shell.fixture.draft_store().with_failure_point_for_testing(
        FailurePoint::CreateTemp,
        FailureAction::Fail(DraftErrorKind::NoSpace),
    );
    let flushed = drafts.update(cx, |registry, cx| registry.flush_now(id, cx));
    shell.run(cx);
    let kind = flushed.await.expect_err("the flush fails");
    errors.push(DraftError { kind });
    let failures = drafts.read_with(cx, |registry, _| registry.flush_failures());
    assert!(!failures.is_empty(), "the failure is listed");
    for (_, title, kind) in failures {
        seen.add("flush failure title", [title]);
        errors.push(DraftError { kind });
    }
    seen.window(&shell, cx);

    // Quit: the flush fails again and the dialog says how many.
    shell.keys("secondary-q", cx);
    let message = shell
        .label("drafts-not-saved-message", cx)
        .expect("the dialog asks");
    seen.add("dialog message", [message]);
    seen.window(&shell, cx);
    shell.click("drafts-not-saved-cancel-button", cx);

    // Every surface was reached, and none holds the sentinel.
    for surface in [
        "operation entry",
        "envelope message",
        "drawn",
        "export result",
        "dialog message",
        "flush failure title",
    ] {
        assert!(seen.surfaces(surface) > 0, "{surface} was read");
    }
    assert!(
        seen.0
            .iter()
            .any(|(at, text)| *at == "drawn" && text.starts_with("Recovery copy of")),
        "the flush failure's entry in the operation area was read"
    );
    for (surface, text) in &seen.0 {
        assert!(!text.contains(SENTINEL), "{surface}: {text}");
    }
    assert!(!errors.is_empty());
    for error in &errors {
        let shown = [format!("{error:?}"), error.to_string()];
        for text in shown {
            assert!(!text.contains(SENTINEL), "DraftError: {text}");
        }
    }
}
