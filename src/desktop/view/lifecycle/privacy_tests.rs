//! D5's privacy sweep (A16; design, Privacy), over real local-only
//! repositories. Unique sentinels are planted in an item's title, its body,
//! a comment on it, and the three sides of its conflict (the base's title,
//! mine and theirs), and the D5 surfaces are driven: a close confirmed,
//! running, broken after its checkpoint, refused at its merge, repaired
//! locally through the conflict sheet and its resolution, and finished.
//! An approve and merge stopped at a safe point, then refused at its tidy-up
//! with its banner, then finished, is `privacy_promote_tests.rs`; a shared
//! conflict and a republish, which come only from a remote, are
//! `privacy_remote_tests.rs`; both use this file's sentinels and `Seen`.
//!
//! After each step, every surface that may show text is read back from the
//! window's own models and frame (D2's and D3's sweeps,
//! `view/drafts_privacy_tests.rs`, `view/editing_privacy_tests.rs`):
//!
//! - every operation entry: its command's label and its envelope message,
//!   and the `Debug` of each finished request's outcome, code, effects,
//!   recovery and message;
//! - every label and value the window draws outside the places the design
//!   puts that text (`ALLOWED`), by where it is drawn: the operation area,
//!   the item's banners and its lifecycle lock (the pill and the notes
//!   beside its locked actions), the lifecycle dialog (its messages and
//!   outcome),
//!   the conflict sheet (its own words) and a confirmation; and no
//!   notification.
//!
//! The sentinels' appearances inside `ALLOWED` are collected too, so each
//! test shows the scan sees drawn text. The desktop logs nothing, so there
//! is no log to read.

use std::{
    collections::BTreeSet,
    path::Path,
    time::{Duration, Instant},
};

use gpui_kit::{ElementId, TestAppContext, component::WindowExt};
use manyhands::{
    repository::{FailurePoint, ProgressStage, RepositoryService},
    results::ResultCode,
};

use super::{
    outcome::{ENDING_DONE, ENDING_FAILED},
    outcome_tests::at_stage,
    tests::{TICKET, closes, code, open, pane, shell, ticket, until_present},
};
use crate::desktop::{
    services::{JobKind, requests::RequestState},
    test_support::fixtures::{Fixture, ticket_path, ticket_source},
    view::{harness::Shell, item::LIFECYCLE_ACTION},
};

pub(super) const TITLE: &str = "SENTINEL-D5-TITLE-3e9a";
pub(super) const BODY: &str = "SENTINEL-D5-BODY-71c4";
pub(super) const COMMENT_TEXT: &str = "SENTINEL-D5-COMMENT-a0f2";
pub(super) const BASE: &str = "SENTINEL-D5-BASE-5d18";
pub(super) const THEIRS: &str = "SENTINEL-D5-THEIRS-c6b7";
/// The title of a ticket that depends on the closed one: the close makes it
/// ready, and the dialog lists it (A3).
pub(super) const READY: &str = "SENTINEL-D5-READY-9b21";
pub(super) const SENTINELS: [&str; 6] = [TITLE, BODY, COMMENT_TEXT, BASE, THEIRS, READY];

const COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5KP0";
const DEPENDENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5KQ0";

/// Where the design puts an item's text, so a sentinel may show there: the
/// tab strip and the header's title; the read view and a comment's read
/// editor; the lifecycle dialog's subtitle ("{short} · {title} ·
/// {repository}", A2) and its made-ready rows, each a ticket's short code
/// and title (A3); and the conflict sheet's title ("Choose what to keep ·
/// {title}"), Before's lines, Yours' and Theirs' lines and the Result editor
/// (B3).
const ALLOWED: [&str; 10] = [
    "tab-strip",
    "item-title",
    "item-read",
    "comment-body",
    "lifecycle-subtitle",
    "lifecycle-made-ready",
    "conflict-title",
    "conflict-before",
    "conflict-line",
    "conflict-result",
];

/// Where a drawn text is read, by the container it is drawn in.
const SURFACES: [(&str, &str); 8] = [
    ("operation-area", "operation area"),
    ("item-banners", "banner"),
    ("item-pill", "lock"),
    ("item-share-locked", "lock"),
    ("item-locked-note", "lock"),
    ("lifecycle-dialog", "dialog message"),
    ("conflict-sheet", "sheet message"),
    ("confirm-dialog", "confirmation message"),
];

/// A long wait, for a real local synchronization or resolution under the
/// whole suite (as `view/conflict_tests.rs`'s `RESOLVING`).
const RESOLVING: Duration = Duration::from_secs(90);

/// A ticket's source titled `title`, with `body` for its body.
pub(super) fn ticket_with_body(id: &str, title: &str, body: &str) -> String {
    ticket_source(id, title, "").replace(&format!("Body of {title}.\n"), body)
}

/// Commits a comment on `item` whose body holds `COMMENT_TEXT`.
pub(super) fn commit_sentinel_comment(fixture: &Fixture, root: &Path, item: &str) {
    let source = format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: {COMMENT}\n\
         item_id: {item}\ncreated_at: 2026-09-30T12:00:00Z\n\
         created_by: Ada Lovelace <ada@example.invalid>\n---\n{COMMENT_TEXT} is the comment.\n"
    );
    fixture.commit_file(
        root,
        &format!(".manyhands/comments/{item}/{COMMENT}.md"),
        source.as_bytes(),
    );
}

/// The name of an element ID, its index aside.
fn name(id: &ElementId) -> Option<&str> {
    match id {
        ElementId::Name(name) | ElementId::NamedInteger(name, _) => Some(name),
        _ => None,
    }
}

/// The first of `names` that `path` is inside, if any.
fn inside(
    path: &[ElementId],
    names: impl IntoIterator<Item = &'static str> + Clone,
) -> Option<&'static str> {
    path.iter()
        .filter_map(name)
        .find_map(|at| names.clone().into_iter().find(|allowed| *allowed == at))
}

/// Everything the sweep read, by where it was read, and which sentinel
/// showed where inside `ALLOWED`. Sets, since a long wait sweeps often.
#[derive(Default)]
pub(super) struct Seen {
    read: BTreeSet<(&'static str, String)>,
    allowed: BTreeSet<(&'static str, &'static str)>,
    answers: BTreeSet<String>,
}

impl Seen {
    fn add(&mut self, surface: &'static str, texts: impl IntoIterator<Item = String>) {
        self.read
            .extend(texts.into_iter().map(|text| (surface, text)));
    }

    /// The window's surfaces as they stand now: the operation entries and
    /// their answers, every label and value drawn outside `ALLOWED` by
    /// where it is drawn, and inside it; no notification.
    pub(super) fn window(&mut self, shell: &Shell, cx: &mut TestAppContext) {
        let (entries, answers) = shell.operations(cx).read_with(cx, |operations, _| {
            let mut entries = Vec::new();
            let mut answers = Vec::new();
            for request in operations.requests() {
                entries.push(request.intent.label().to_owned());
                match &request.state {
                    RequestState::Finished(outcome, code, effects, recovery, message) => {
                        entries.push(message.clone());
                        answers.push(format!(
                            "{outcome:?} {code:?} {} {effects:?} {recovery:?} {message:?}",
                            code.as_str()
                        ));
                    }
                    RequestState::NotSent(why) => answers.push(format!("{why:?}")),
                    _ => {}
                }
            }
            (entries, answers)
        });
        self.add("operation entry", entries);
        self.answers.extend(answers);
        let drawn: Vec<(Option<&'static str>, &'static str, String)> =
            shell.frame(cx, |window, _| {
                gpui_kit::base::test_support::snapshots(window)
                    .iter()
                    .flat_map(|snapshot| {
                        let path = snapshot.path();
                        let at = inside(path, ALLOWED);
                        let surface = SURFACES
                            .iter()
                            .find(|(container, _)| inside(path, [*container]).is_some())
                            .map_or("drawn", |(_, surface)| *surface);
                        [snapshot.label(), snapshot.value()]
                            .into_iter()
                            .flatten()
                            .map(move |text| (at, surface, text.to_owned()))
                            .collect::<Vec<_>>()
                    })
                    .collect()
            });
        for (at, surface, text) in drawn {
            match at {
                Some(at) => self.allowed.extend(
                    SENTINELS
                        .into_iter()
                        .filter(|sentinel| text.contains(sentinel))
                        .map(|sentinel| (at, sentinel)),
                ),
                None => {
                    self.read.insert((surface, text));
                }
            }
        }
        let notifications = shell.frame(cx, |window, cx| window.notifications(cx).len());
        assert_eq!(notifications, 0, "the desktop shows no notification");
    }

    /// Whether `surface` was read, holding `text`.
    pub(super) fn read_at(&self, surface: &str, text: &str) -> bool {
        self.read
            .iter()
            .any(|(at, read)| *at == surface && read.contains(text))
    }

    /// Whether a sentinel showed inside the allowed place `at`.
    pub(super) fn allowed_at(&self, at: &str) -> bool {
        self.allowed.iter().any(|(allowed, _)| *allowed == at)
    }

    /// Whether `sentinel` showed inside the allowed place `at`.
    pub(super) fn allowed_holding(&self, at: &str, sentinel: &str) -> bool {
        self.allowed
            .iter()
            .any(|(allowed, held)| *allowed == at && *held == sentinel)
    }

    /// Each of `surfaces` was read, and no surface read holds a sentinel,
    /// nor any answer's rendering.
    pub(super) fn holds_none(&self, surfaces: &[&str]) {
        for surface in surfaces {
            assert!(
                self.read.iter().any(|(at, _)| at == surface),
                "{surface} was read"
            );
        }
        for (surface, text) in &self.read {
            for sentinel in SENTINELS {
                assert!(!text.contains(sentinel), "{surface}: {text}");
            }
        }
        assert!(!self.answers.is_empty(), "answers were read");
        for answer in &self.answers {
            for sentinel in SENTINELS {
                assert!(!answer.contains(sentinel), "an answer: {answer}");
            }
        }
    }
}

/// The shown tab's read view holds `BODY` and its comment's editor
/// `COMMENT_TEXT`: they are on screen, where the design puts them.
pub(super) fn shows_body_and_comment(shell: &Shell, cx: &mut TestAppContext) {
    shell.frame(cx, |_, _| ());
    let (read, comment) = pane(shell, cx).read_with(cx, |pane, cx| {
        let read = pane.read_editor(cx).expect("the read view");
        let comment = pane.comment_view(COMMENT).expect("the comment's editor");
        (read, comment.read(cx).editor().clone())
    });
    for (editor, sentinel) in [(read, BODY), (comment, COMMENT_TEXT)] {
        let text = editor.read_with(cx, |editor, _| editor.text().to_owned());
        assert!(text.contains(sentinel), "{sentinel} is shown");
    }
}

/// Waits, past the harness's `WAIT`, until `done` holds of the window,
/// sweeping it into `seen` each time it looks, so the progress shown
/// meanwhile is read too.
pub(super) async fn until_long(
    shell: &Shell,
    what: &str,
    seen: &mut Seen,
    cx: &mut TestAppContext,
    done: impl Fn(&Shell, &mut TestAppContext) -> bool,
) {
    let deadline = Instant::now() + RESOLVING;
    loop {
        shell.frame(cx, |_, _| ());
        shell.run(cx);
        seen.window(shell, cx);
        if done(shell, cx) {
            shell.frame(cx, |_, _| ());
            return;
        }
        assert!(Instant::now() < deadline, "{what} did not happen");
        cx.executor().timer(Duration::from_millis(10)).await;
    }
}

/// A16, real: a ticket whose title, body, comment and conflict sides hold
/// sentinels is closed. The close is confirmed, held running at
/// `recording`, and broken after its closure checkpoint
/// (`FailurePoint::AfterClosureCheckpoint`); main then moves on the ticket
/// (theirs), so Retry is refused at the merge and offers Repair locally…
/// (P9, as `conflict/repair_tests.rs` `to_the_repair` drives it); the
/// repair's local synchronization stops on a real conflict of the ticket's
/// file, whose sheet shows the three sides (Before holds `BASE`, Yours
/// `TITLE` and Theirs `THEIRS`); mine is kept and saved; the dialog comes
/// back and Retry finishes the close, which makes a dependent ticket titled
/// `READY` ready, listed in the dialog. No sentinel reaches an entry, an
/// answer, the operation area, a banner, the dialog's messages and outcome
/// or the sheet's own words.
#[gpui_kit::test]
async fn a16_a_close_broken_refused_repaired_and_done_shows_no_item_text(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, BASE, "");
        commit_sentinel_comment(fixture, root, TICKET);
        fixture.commit_ticket(root, DEPENDENT, READY, &format!("deps:\n  - {TICKET}\n"));
        fixture.refresh(root);
        fixture.save_ticket_with_body(root, TICKET, TITLE, &format!("{BODY} is the body.\n"));
    });
    let mut seen = Seen::default();

    // Viewing, then the confirm stage.
    open(&shell, ticket(&root), cx);
    shows_body_and_comment(&shell, cx);
    seen.window(&shell, cx);
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-confirm", cx).await;
    seen.window(&shell, cx);

    // Running, held at `recording`, then broken after its closure
    // checkpoint (`FailurePoint::AfterClosureCheckpoint`, as
    // `conflict/repair_tests.rs` `to_the_repair` breaks one).
    shell.backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            shell.fixture.data_dir(),
            FailurePoint::AfterClosureCheckpoint,
        )
        .unwrap(),
    );
    let gate = shell
        .backend
        .hold_at_stage(JobKind::TicketClose, ProgressStage::Recording);
    shell.click("lifecycle-confirm", cx);
    gate.wait_reached();
    let first = closes(&shell, cx)[0];
    at_stage(&shell, first, ProgressStage::Recording, cx).await;
    until_present(&shell, "lifecycle-stop", cx).await;
    seen.window(&shell, cx);
    gate.release();
    shell.until_ended(first, cx).await;
    assert_ne!(code(&shell, first, cx), Some(ResultCode::Ok), "broken");
    until_present(&shell, "lifecycle-ending", cx).await;
    seen.window(&shell, cx);
    assert!(
        seen.read_at("lock", super::lock_tests::CLOSING),
        "the lock is shown"
    );
    shell
        .backend
        .replace_service(RepositoryService::open_at(shell.fixture.data_dir()).unwrap());

    // Main moves on the ticket (theirs): Retry is refused at the merge.
    shell.fixture.commit_file(
        &root,
        &ticket_path(TICKET),
        ticket_with_body(TICKET, THEIRS, &format!("Body of {THEIRS}.\n")).as_bytes(),
    );
    until_present(&shell, "lifecycle-retry", cx).await;
    shell.click("lifecycle-retry", cx);
    let refused = closes(&shell, cx)[0];
    shell.until_ended(refused, cx).await;
    assert_eq!(
        code(&shell, refused, cx),
        Some(ResultCode::RecoveryRequired)
    );
    until_present(&shell, "lifecycle-repair-locally", cx).await;
    assert_eq!(
        shell.label("lifecycle-ending", cx).as_deref(),
        Some(ENDING_FAILED)
    );
    seen.window(&shell, cx);

    // Repair locally…: the sheet on the real conflict, its sides shown.
    shell.click("lifecycle-repair-locally", cx);
    until_long(&shell, "the sheet's opening", &mut seen, cx, |shell, cx| {
        shell.present("conflict-save", cx)
    })
    .await;
    seen.window(&shell, cx);
    assert!(seen.allowed_holding("conflict-title", TITLE));
    for sentinel in [TITLE, THEIRS] {
        assert!(
            seen.allowed_holding("conflict-line", sentinel),
            "{sentinel}"
        );
    }
    assert!(
        seen.allowed_holding("conflict-before", BASE),
        "Before's lines"
    );
    let mut part = 0usize;
    while shell.present(("conflict-keep-mine", part), cx) {
        shell.click(("conflict-keep-mine", part), cx);
        part += 1;
    }
    let mut body = 0usize;
    while shell.present(("conflict-body-mine", body), cx) {
        shell.click(("conflict-body-mine", body), cx);
        body += 1;
    }
    assert!(part + body > 0, "a part to choose");
    seen.window(&shell, cx);
    shell.click("conflict-save", cx);
    seen.window(&shell, cx);
    until_long(
        &shell,
        "the resolution's answer",
        &mut seen,
        cx,
        |shell, cx| !shell.present("conflict-sheet", cx),
    )
    .await;
    seen.window(&shell, cx);

    // The dialog again: Retry finishes the same close.
    until_present(&shell, "lifecycle-retry", cx).await;
    seen.window(&shell, cx);
    shell.click("lifecycle-retry", cx);
    let retried = closes(&shell, cx)[0];
    until_long(&shell, "the close's answer", &mut seen, cx, |shell, cx| {
        code(shell, retried, cx).is_some()
    })
    .await;
    assert_eq!(code(&shell, retried, cx), Some(ResultCode::Ok));
    until_present(&shell, "lifecycle-ending", cx).await;
    assert_eq!(
        shell.label("lifecycle-ending", cx).as_deref(),
        Some(ENDING_DONE)
    );
    shell
        .until_drawn(cx, "the made-ready list", |shell, cx| {
            shell.present(("lifecycle-made-ready", 0usize), cx)
        })
        .await;
    seen.window(&shell, cx);
    assert!(
        seen.allowed_holding("lifecycle-made-ready", READY),
        "the made-ready list names the dependent"
    );
    shell.keys("escape", cx);
    seen.window(&shell, cx);

    seen.holds_none(&[
        "operation entry",
        "operation area",
        "lock",
        "banner",
        "dialog message",
        "sheet message",
        "drawn",
    ]);
    assert!(seen.read_at("dialog message", ENDING_DONE));
    for at in ["item-title", "lifecycle-subtitle"] {
        assert!(seen.allowed_at(at), "a sentinel is drawn in {at}");
    }
}
