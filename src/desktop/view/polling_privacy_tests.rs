//! D6's privacy sweep (acceptance A16, Task 13; design, Privacy), over a real
//! window, real repositories and the polling double. Distinct sentinels are
//! planted where D6's surfaces could leak them, and the D6 surfaces are
//! driven; after each step every surface is read back and no sentinel is on
//! any of them.
//!
//! The sentinels, and where each is planted:
//!
//! - `TITLE`: the titles of every item (committed tickets, a document), which
//!   the lists have read, so the desktop knows them;
//! - `BODY`: the bodies of the tickets and the document;
//! - `COMMENT`: a comment on two tickets, and a comment draft;
//! - `DRAFT`: the fields of the kept drafts of a ticket and a document;
//! - `EDIT`: text typed into an open ticket's editor, unkept;
//! - `PATH`: the document's path, which its tree row carries only in a
//!   tooltip and a description, neither of which the test snapshot reads
//!   (E-15), and which a retirement of the document could be tempted to name;
//! - `PASSPHRASE`: typed into the passphrase dialog an Unlock… raises;
//! - `PRIVATE`: the content of the private key file the unlock asks for;
//! - `PASSWORD`: the password in the URL of a second repository's publication
//!   remote (Cycle 03a Part I, U-3).
//!
//! The drive: an automatic poll of a repository that reports a closed ticket
//! (retired, its in-flight record read while the library holds it), two
//! tickets and a document the loop leaves alone (being edited, a kept draft,
//! a kept draft), a closed ticket the library refuses, one it leaves pending
//! and one it never reaches; the Retire… offers; the unkept ask (text typed
//! into a draft folder that cannot be written) and the kept-draft ask, for a
//! ticket and for a document; a Retire… the real library answers; a Check
//! now the real library answers; a document refused `unlock_required`, which
//! is attention with Unlock…; the Settings page, paused and needing its key;
//! Resume of the pending retirement; Unlock…, which resends it and asks for
//! the passphrase, typed and supplied; and a quit that waits for a job: its
//! ask about the unkept text, its words, and its words at the injected ten
//! seconds.
//!
//! Every surface is read from the window's own models and frame, as D5's
//! sweep does (`lifecycle/privacy_tests.rs`):
//!
//! - every operation entry: its command's label, its message and the
//!   `Debug` of each finished request's outcome, code, effects, recovery and
//!   message;
//! - the `Debug` of the polling schedule (every attempt, line, attention
//!   entry and retirement it holds) and of the in-flight record's
//!   retirement entries, and the bytes of the record file itself;
//! - every label and value the window draws outside the places the design
//!   puts item text (`ALLOWED`), by where it is drawn (`surface`): the
//!   attempt entries, their offers and retirement lines, the attention
//!   entries, the pending rows, the asks, the passphrase dialog, the Settings
//!   page and the quit's words; and no notification.
//!
//! The Repositories panel (rows, toolbar) and the footer are read like the
//! rest, `region-repositories` no longer being allowed, and `repositories`
//! reads what no snapshot does, through the functions that build it, for
//! every repository: tooltips, menus, More's menu and the footer's names,
//! against every sentinel but the title (unshared items are named by it).
//!
//! The sentinels' appearances inside `ALLOWED`, and in the data the desktop
//! read them from, are collected first, so the sweep is shown to read text
//! that holds them.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use gpui_kit::{ElementId, TestAppContext, component::WindowExt, test::TestWindowExt};
use manyhands::{
    drafts::{CommentTarget, DraftFields, DraftTarget, ItemKind},
    repository::{
        ContextRetirementState as State, ItemDtoKind, MutationDataDto, PollCompletion,
        PollDisposition, RequestId, RetirementRemoval as Removal, SharedKeyId,
        keys::{KeySourceToken, UnlockReason, UnlockRequest},
    },
    results::{Envelope, ResultCode as Code},
};

mod repositories;

use super::{harness::Shell, polling_window::PollingWindow, repositories::tests::support::not_due};
use crate::desktop::{
    model::{
        navigation::OpenTarget,
        polling::{Clock as _, TestClock, entry::short_code, retire_loop::Held},
    },
    services::{
        JobKind, ReadResult,
        drafts::Kept,
        poll_convert::{Library, ReportParts, TargetParts},
        requests::{Intent, Lane, RequestState},
    },
    test_support::{
        drafts::{FolderMode, superuser},
        fixtures::{Fixture, item_id, ticket_path, ticket_source},
        polling::Asked,
        retire::{OPERATION, answer, original_request, pending_row},
    },
};

const TITLE: &str = "SENTINEL-D6-TITLE-5b2e";
const BODY: &str = "SENTINEL-D6-BODY-9d41";
const COMMENT: &str = "SENTINEL-D6-COMMENT-c07a";
const DRAFT: &str = "SENTINEL-D6-DRAFT-18fe";
const EDIT: &str = "SENTINEL-D6-EDIT-a3b6";
const PATH: &str = "sentinel-d6-path-3c91";
const PASSPHRASE: &str = "SENTINEL-D6-PASSPHRASE-e27d";
const PRIVATE: &str = "SENTINEL-D6-PRIVATE-4f08";
const PASSWORD: &str = "sentinel-password-03a";
const SENTINELS: [&str; 9] = [
    TITLE, BODY, COMMENT, DRAFT, EDIT, PATH, PASSPHRASE, PRIVATE, PASSWORD,
];

/// A closed ticket that is free, one being edited, one with kept drafts, a
/// promoted document with a kept draft, a closed ticket the library
/// refuses, one it leaves pending and one the loop never reaches.
const FREE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FA1";
const EDITED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FA2";
const KEPT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FA3";
const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FA4";
const REFUSED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FA5";
const PENDING: &str = "01ARZ3NDEKTSV4RRFFQ69G5FA6";
const UNREACHED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FA7";
const COMMENT_ON_EDITED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FB1";
const COMMENT_ON_KEPT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FB2";
const DRAFTED_COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FB3";

/// Where the design puts item text, so a sentinel may show there: the tab
/// strip and the item's own page (title, read view, edit editor, comments,
/// source) and the lists, which name items by title (a row's path is its
/// tooltip, which no snapshot reads, E-15). Everything
/// else the window draws is read.
///
/// `item-banners` and `item-pill` are allowed too, and are out of scope for
/// this sweep: the item's own banners and lock pill belong to D5's and D3's
/// sweeps (`lifecycle/privacy_tests.rs`, `editing_privacy_tests.rs`), which
/// read them with the item's text in hand. D6 puts nothing there, and this
/// drive opens an item tab only to hold an edit session.
const ALLOWED: [&str; 11] = [
    "tab-strip",
    "item-title",
    "item-read",
    "item-edit-view",
    "item-source",
    "comment-body",
    "region-tickets",
    "region-documents",
    "toolbar-link-popover",
    "item-banners",
    "item-pill",
];

/// Where a text is read, by the id of the element that draws it, for the
/// surfaces the test asserts it read: `(prefix, contains, surface)`.
const SURFACES: [(&str, &str, &str); 17] = [
    ("attempt-", "-offer-", "retire offer"),
    ("attempt-", "-retire-", "retire action"),
    ("attempt-", "-retirement-", "retirement line"),
    ("attempt-", "-line-", "attempt line"),
    ("attempt-", "-repository", "attempt entry"),
    ("attention", "", "attention"),
    ("pending-", "", "pending row"),
    ("retire-unkept", "", "unkept ask"),
    ("retire-kept", "", "kept ask"),
    ("passphrase-", "", "passphrase dialog"),
    ("polling-", "", "settings row"),
    ("quit-", "", "quit"),
    ("edits-not-kept", "", "quit ask"),
    ("confirm-", "", "confirmation"),
    ("request-", "", "operation entry"),
    ("repo-", "", "repository row"),
    ("footer-", "", "footer"),
];

/// The name of an element ID, its index aside.
fn name(id: &ElementId) -> Option<&str> {
    match id {
        ElementId::Name(name) | ElementId::NamedInteger(name, _) => Some(name),
        _ => None,
    }
}

/// The first of `places` that `path` is inside, if any.
fn inside(path: &[ElementId], places: &[&'static str]) -> Option<&'static str> {
    path.iter()
        .filter_map(name)
        .find_map(|at| places.iter().copied().find(|place| *place == at))
}

/// The surface the element at `path` belongs to, by its own id.
fn surface(path: &[ElementId]) -> &'static str {
    let last = path.last().and_then(name).unwrap_or("");
    SURFACES
        .iter()
        .find(|(prefix, contains, _)| last.starts_with(prefix) && last.contains(contains))
        .map_or("drawn", |(_, _, surface)| surface)
}

/// Everything the sweeps read, by where it was read.
#[derive(Default)]
struct Seen {
    read: BTreeSet<(&'static str, String)>,
    /// Which sentinel showed where inside `ALLOWED`.
    allowed: BTreeSet<(&'static str, &'static str)>,
    /// How many times the in-flight record had bytes to read.
    records: usize,
    /// The Repositories panel's and the footer's functions' texts.
    repositories: repositories::Texts,
}

impl Seen {
    fn add(&mut self, surface: &'static str, texts: impl IntoIterator<Item = String>) {
        self.read
            .extend(texts.into_iter().map(|text| (surface, text)));
    }

    /// The window's surfaces as they stand now, read back.
    fn sweep(&mut self, window: &PollingWindow, cx: &mut TestAppContext) {
        let shell = &window.shell;
        // The operation entries and answers.
        let (entries, answers) = shell.operations(cx).read_with(cx, |operations, _| {
            let (mut entries, mut answers) = (Vec::new(), Vec::new());
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
        self.add("answer", answers);
        // The schedule's and the record's own state, as `Debug` renders it.
        let (schedule, store) = window.services.scheduler().read_with(cx, |scheduler, _| {
            (
                format!("{:?}", scheduler.schedule()),
                format!("{:?}", scheduler.store().retirements()),
            )
        });
        self.add("schedule", [schedule]);
        self.add("retirement entries", [store]);
        // The record's file.
        let record = window.shell.fixture.data_dir().join("inflight/record.json");
        if let Ok(bytes) = fs::read(&record) {
            self.records += 1;
            self.add(
                "record file",
                [String::from_utf8_lossy(&bytes).into_owned()],
            );
        }
        // What is drawn.
        let drawn: Vec<(Option<&'static str>, &'static str, String)> =
            shell.frame(cx, |window, _| {
                gpui_kit::base::test_support::snapshots(window)
                    .iter()
                    .flat_map(|snapshot| {
                        let path = snapshot.path();
                        let at = inside(path, &ALLOWED);
                        let surface = surface(path);
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
        self.repositories.extend(repositories::read(window, cx));
        let notifications = shell.frame(cx, |window, cx| window.notifications(cx).len());
        assert_eq!(notifications, 0, "the desktop shows no notification");
        self.holds_none();
    }

    /// No surface read holds a sentinel.
    fn holds_none(&self) {
        repositories::holds_none(&self.repositories);
        for (surface, text) in &self.read {
            for sentinel in SENTINELS {
                assert!(
                    !text.contains(sentinel),
                    "{sentinel} reached {surface}: {text}"
                );
            }
        }
    }

    /// `surface` was read, holding `text`.
    fn read_at(&self, surface: &str, text: &str) -> bool {
        self.read
            .iter()
            .any(|(at, read)| *at == surface && read.contains(text))
    }

    fn surfaces(&self, surface: &str) -> usize {
        self.read.iter().filter(|(at, _)| *at == surface).count()
    }
}

/// A ticket's source titled `{TITLE}-{n}` with `BODY` for its body.
fn ticket_with_sentinels(id: &str, n: usize) -> String {
    let title = format!("{TITLE}-{n}");
    ticket_source(id, &title, "").replace(
        &format!("Body of {title}.\n"),
        &format!("{BODY} is the body.\n"),
    )
}

fn comment_on(item: &str, id: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: {id}\nitem_id: {item}\n\
         created_at: 2026-09-30T12:00:00Z\ncreated_by: Ada Lovelace <ada@example.invalid>\n\
         ---\n{COMMENT} is the comment.\n"
    )
}

/// A report of `items`, closed tickets and the promoted document.
fn report(clock: &TestClock, items: &[(&str, bool)]) -> Library {
    let now = clock.utc();
    Library::Polled {
        parts: ReportParts {
            completion: PollCompletion::Completed,
            completed_at: now,
            next_eligible_at: Some(now + time::Duration::seconds(300)),
            targets: items
                .iter()
                .map(|(id, document)| TargetParts {
                    item_id: Some(item_id(id)),
                    kind: Some(if *document {
                        manyhands::repository::AuthoringKind::Document
                    } else {
                        manyhands::repository::AuthoringKind::Ticket
                    }),
                    disposition: if *document {
                        PollDisposition::Promoted
                    } else {
                        PollDisposition::Closed
                    },
                    main: false,
                })
                .collect(),
            outcome: None,
        },
        index_pending: false,
    }
}

/// The element of Retire… of `item` in repository `root`'s entry.
fn retire_button(root: &str, item: &str) -> String {
    format!("attempt-{root}-retire-{}", short_code(&item_id(item)))
}

/// Retire… of `item`, reached by Tab (which scrolls it into view in
/// Activity) and clicked where it is.
fn press_retire(window: &PollingWindow, root: &str, item: &str, cx: &mut TestAppContext) {
    let button = retire_button(root, item);
    let shown = window.shell.frame(cx, |win, _| {
        win.try_find(button.clone())
            .is_some_and(|element| element.visible())
    });
    if !shown {
        window.shell.keys("secondary-4", cx);
        assert!(
            window.shell.tab_reaches(button.clone(), 160, cx),
            "{button}"
        );
    }
    window.shell.click(button, cx);
}

fn target(root: &str, id: &str, comment: Option<&str>, kind: ItemKind) -> DraftTarget {
    DraftTarget {
        repository: root.to_owned(),
        item_kind: kind,
        item_id: item_id(id),
        comment: comment.map(|comment| CommentTarget {
            comment_id: comment.parse().unwrap(),
            parent_id: None,
        }),
    }
}

/// The bytes of every file under `dir`, as text.
fn files_under(dir: &Path) -> Vec<(PathBuf, String)> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_owned()];
    while let Some(next) = pending.pop() {
        let Ok(entries) = fs::read_dir(&next) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if let Ok(bytes) = fs::read(&path) {
                found.push((path, String::from_utf8_lossy(&bytes).into_owned()));
            }
        }
    }
    found
}

#[gpui_kit::test]
async fn polling_privacy(cx: &mut TestAppContext) {
    // A read-only folder stops nothing for the superuser, so the one step
    // that needs unkept text (and the quit's ask about it) is skipped there;
    // every other step runs.
    let rooted = superuser();
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    for (n, id) in [FREE, EDITED, KEPT, REFUSED, PENDING, UNREACHED]
        .into_iter()
        .enumerate()
    {
        fixture.commit_file(
            &a,
            &ticket_path(id),
            ticket_with_sentinels(id, n).as_bytes(),
        );
    }
    for (item, comment) in [(EDITED, COMMENT_ON_EDITED), (KEPT, COMMENT_ON_KEPT)] {
        fixture.commit_file(
            &a,
            &format!(".manyhands/comments/{item}/{comment}.md"),
            comment_on(item, comment).as_bytes(),
        );
    }
    fixture.create_document(
        &a,
        DOCUMENT,
        &format!("docs/{PATH}.md"),
        &format!("{TITLE}-document"),
    );
    fixture.publish_over_ssh(&a);
    fixture.refresh(&a);
    // A second repository whose publication remote's URL gains a password
    // (U-3, `repositories::password_is_not_eligible`).
    let b = fixture.create_repository("b");
    fixture.publish_over_ssh(&b);
    fixture.set_remote_url(
        &b,
        "origin",
        &format!("ssh://git:{PASSWORD}@example.invalid/r.git"),
    );
    let root = crate::desktop::view::harness::key(&a);

    // Kept drafts: a ticket's and a comment's on it, a comment being
    // written, and the document's.
    let store = fixture.draft_store();
    let ticket_fields = DraftFields::Ticket {
        title: format!("{DRAFT} title"),
        ticket_type: "task".to_owned(),
        status: "open".to_owned(),
        project: None,
        team: None,
        deps: Vec::new(),
        parent: None,
        body: format!("{DRAFT} body"),
    };
    for (draft, fields) in [
        (target(&root, KEPT, None, ItemKind::Ticket), ticket_fields),
        (
            target(&root, KEPT, Some(DRAFTED_COMMENT), ItemKind::Ticket),
            DraftFields::Comment {
                body: format!("{COMMENT} in a draft"),
            },
        ),
        (
            target(&root, DOCUMENT, None, ItemKind::Document),
            DraftFields::Document {
                title: format!("{DRAFT} document"),
                body: format!("{DRAFT} document body"),
            },
        ),
    ] {
        drop(store.create(draft, None, fields).expect("a kept draft"));
    }

    // The private key file the unlock asks about holds a sentinel.
    let keys = tempfile::tempdir().unwrap();
    let key_id = SharedKeyId::new();
    let private = keys.path().join(key_id.to_string());
    fs::write(&private, format!("{PRIVATE} is the private key\n")).unwrap();
    let unlock = UnlockRequest {
        key_id,
        label: "primary".to_owned(),
        source: KeySourceToken::observe(&private).unwrap(),
        reason: UnlockReason::ProtectedKey,
    };

    // The positive controls, part one: every sentinel is in the data the
    // desktop reads or is given.
    let on_disk = files_under(&a);
    let holds = |needle: &str, files: &[(PathBuf, String)]| {
        files.iter().any(|(_, text)| text.contains(needle))
    };
    for sentinel in [TITLE, BODY, COMMENT] {
        assert!(holds(sentinel, &on_disk), "{sentinel} is in the repository");
    }
    assert!(
        holds(PASSWORD, &files_under(&b)),
        "{PASSWORD} is in the second repository's remote"
    );
    assert!(
        a.join(format!("docs/{PATH}.md")).exists() || holds(PATH, &on_disk),
        "{PATH} is the document's path"
    );
    let drafts = store.list().unwrap();
    assert!(!drafts.is_empty());
    let kept: Vec<String> = drafts
        .iter()
        .map(|listing| String::from_utf8_lossy(&store.raw_bytes(listing.id).unwrap()).into_owned())
        .collect();
    assert!(
        kept.iter().any(|text| text.contains(DRAFT))
            && kept.iter().any(|text| text.contains(COMMENT)),
        "the drafts hold {DRAFT} and a comment's {COMMENT}"
    );
    assert!(fs::read_to_string(&private).unwrap().contains(PRIVATE));

    // The launch poll the workspace's listing starts reports the seven
    // items. The loop retires the free one (held in the library, so the
    // in-flight record can be read with its entry in it), leaves the edited
    // one and the two with kept drafts, meets a refusal at the next free one
    // (and goes on), a pending retirement at the next (and stops), and never
    // reaches the last. Polling starts once the window is set up (the draft
    // listing has landed and the edited tab is open).
    let (window, (held, poll)) = PollingWindow::over(
        fixture,
        &[a.clone(), b.clone()],
        |backend, clock| {
            // Every poll the drive does not script (the second repository's,
            // if it is polled at all) gets a "not due" and never the network.
            backend.answer_unanswered_polls(not_due(clock));
            let inner = backend.inner();
            let held = inner.hold_job(JobKind::ContextRetire);
            inner.answer_job(
                JobKind::ContextRetire,
                answer(State::Retired, Code::Ok, Removal::Observed),
            );
            inner.answer_job(
                JobKind::ContextRetire,
                answer(State::Refused, Code::WorktreeNotClean, Removal::None),
            );
            inner.answer_job(
                JobKind::ContextRetire,
                answer(State::Pending, Code::DiscoveryPending, Removal::None),
            );
            backend.answer_poll(report(
                clock,
                &[
                    (FREE, false),
                    (EDITED, false),
                    (KEPT, false),
                    (DOCUMENT, true),
                    (REFUSED, false),
                    (PENDING, false),
                    (UNREACHED, false),
                ],
            ));
            (held, backend.hold_poll())
        },
        cx,
    );
    let mut seen = Seen::default();
    window.shell.select(&a, cx);
    // The retirement requests so far, and whether every request has ended.
    let asked = |window: &PollingWindow, cx: &mut TestAppContext| {
        window.shell.requests(cx, |request| {
            matches!(request.intent, Intent::ContextRetire { .. })
        })
    };
    let ended = |shell: &Shell, cx: &mut TestAppContext| {
        shell.operations(cx).read_with(cx, |operations, _| {
            operations.requests().iter().all(|request| {
                !matches!(
                    request.state,
                    RequestState::Queued | RequestState::Running { .. }
                )
            })
        })
    };

    // The edited ticket is open in a tab and being edited (its text is typed
    // after the loop has run: the loop needs the draft listing, which a
    // read-only folder would not give).
    window.shell.workspace.update(cx, |workspace, cx| {
        workspace.open_item(
            OpenTarget::Item {
                repo: root.clone(),
                kind: ItemDtoKind::Ticket,
                id: item_id(EDITED),
            },
            cx,
        )
    });
    window.shell.run(cx);
    window.shell.click("item-edit", cx);
    let host = window.shell.until_editing(cx).await;
    window.shell.run(cx);
    seen.sweep(&window, cx);
    assert!(
        seen.allowed.contains(&("tab-strip", TITLE)),
        "{TITLE} is drawn in the tab strip, where the design puts it"
    );

    // The window is set up, and the draft listing has landed (an unknown
    // listing counts as a draft, RT-5): polling starts, and the poll goes.
    let drafts = window.services.drafts().clone();
    window
        .shell
        .until_drawn(cx, "the draft listing", |_, cx| {
            drafts.read_with(cx, |registry, _| {
                registry.held_by(&root, ItemDtoKind::Ticket, &item_id(FREE)) != Held::Unknown
            })
        })
        .await;
    window.start(cx);
    drop(poll);
    window
        .shell
        .until_drawn(cx, "the first retirement", |_, _| {
            window
                .backend
                .inner()
                .jobs_seen()
                .contains(&JobKind::ContextRetire)
        })
        .await;
    held.wait_reached();
    // Paused while the loop runs, so that the policy read after it makes the
    // repository no more due: another poll would replace the attempt, and
    // with it the offers (the stored deadline of a constructed answer is
    // none, as in `loop_tests`).
    window.shell.fixture.pause_polling(&a);
    seen.sweep(&window, cx);
    assert!(
        seen.records > 0,
        "the record has bytes while a retirement is held"
    );
    assert!(
        seen.read_at("record file", FREE),
        "the record names the item it holds"
    );
    held.release();
    window
        .shell
        .until_drawn(
            cx,
            "the loop to stop at the pending retirement",
            |shell, cx| {
                asked(&window, cx).len() == 3
                    && ended(shell, cx)
                    && shell.present(retire_button(&root, UNREACHED), cx)
            },
        )
        .await;
    seen.sweep(&window, cx);
    // The pending retirement's row in Activity, as the library lists it:
    // Resume (the record's entry names its operation) and Abandon.
    seed_pending(&window, &root, cx);
    window.shell.run(cx);
    window.shell.frame(cx, |_, _| ());
    seen.sweep(&window, cx);
    assert!(window.shell.present(("pending-resume", 0usize), cx));

    // Text is typed into the edited ticket after its draft folder became
    // read-only: no draft can keep it. Retire… of that ticket asks, with a
    // count, no title and no text, and offers Show tab, which shows the tab.
    let read_only =
        (!rooted).then(|| FolderMode::set(&window.shell.fixture.data_dir().join("drafts"), 0o500));
    window.shell.type_at_end(&host, EDIT, cx);
    window.shell.run(cx);
    let typed = host.read_with(cx, |host, cx| host.text(cx));
    assert!(typed.contains(EDIT), "{EDIT} is in the editor");
    assert!(typed.contains(BODY), "{BODY} is in the editor");
    let comment_text = window
        .shell
        .workspace
        .read_with(cx, |workspace, _| workspace.item_pane())
        .and_then(|pane| pane.read_with(cx, |pane, _| pane.comment_view(COMMENT_ON_EDITED)))
        .map(|view| view.read_with(cx, |view, cx| view.editor().read(cx).text().to_owned()));
    assert!(
        comment_text.is_some_and(|text| text.contains(COMMENT)),
        "{COMMENT} is in the comment's editor"
    );
    assert_eq!(
        drafts.read_with(cx, |registry, _| registry.kept_of(
            &root,
            ItemDtoKind::Ticket,
            &item_id(KEPT)
        )),
        Kept::Counted {
            item: 1,
            comments: 1
        },
        "the desktop holds the ticket's draft and its comment's ({DRAFT}, {COMMENT})"
    );
    for (at, sentinel) in [
        ("tab-strip", TITLE),
        ("region-tickets", TITLE),
        ("region-documents", TITLE),
    ] {
        assert!(
            seen.allowed.contains(&(at, sentinel)),
            "{sentinel} is drawn in {at}, where the design puts it"
        );
    }
    seen.sweep(&window, cx);
    let sent = asked(&window, cx).len();
    if !rooted {
        press_retire(&window, &root, EDITED, cx);
        window.shell.run(cx);
        assert!(window.shell.present("retire-unkept-dialog", cx), "it asks");
        seen.sweep(&window, cx);
        assert!(seen.read_at("unkept ask", "1"), "the ask counts the tab");
        window.shell.press("enter", cx);
        seen.sweep(&window, cx);
        assert_eq!(asked(&window, cx).len(), sent, "nothing was submitted");
    }

    // Retire… of a ticket with kept drafts (its draft and a comment's):
    // the ask counts them and is Cancelled by its default.
    press_retire(&window, &root, KEPT, cx);
    window
        .shell
        .until_drawn(cx, "the kept-draft ask", |shell, cx| {
            shell.present("retire-kept-dialog", cx)
        })
        .await;
    seen.sweep(&window, cx);
    assert!(seen.read_at("kept ask", "2 kept drafts"), "two drafts");
    window.shell.press("enter", cx);
    assert!(!window.shell.present("retire-kept-dialog", cx));
    assert_eq!(asked(&window, cx).len(), sent, "Cancel submitted nothing");
    seen.sweep(&window, cx);

    // Retire… of the item the loop never reached, which the real library
    // answers (no canned answer is queued): its own words are read.
    press_retire(&window, &root, UNREACHED, cx);
    window
        .shell
        .until_drawn(cx, "the real answer", |shell, cx| {
            let sent = asked(&window, cx);
            sent.len() == 4 && ended(shell, cx)
        })
        .await;
    seen.sweep(&window, cx);

    // A Check now that nothing stands in for: the library answers it as it
    // does with no transport, and its own words are read.
    let before_check = window.shell.requests(cx, |request| {
        matches!(request.intent, Intent::PollOnce { .. })
    });
    window
        .services
        .worker()
        .submit(Intent::PollOnce { root: a.clone() }, Lane::User, None, None);
    window
        .shell
        .until_drawn(cx, "the real Check now", |shell, cx| {
            shell
                .requests(cx, |request| {
                    matches!(request.intent, Intent::PollOnce { .. })
                })
                .len()
                == before_check.len() + 1
                && ended(shell, cx)
        })
        .await;
    seen.sweep(&window, cx);

    // Retire… of the document, whose kept draft is asked about in the
    // document's words; the library refuses `unlock_required`, which is
    // attention with Unlock… for the key.
    inner_answer(
        &window,
        answer(State::Refused, Code::UnlockRequired, Removal::None),
    );
    window.backend.select_key(Some(key_id));
    press_retire(&window, &root, DOCUMENT, cx);
    window
        .shell
        .until_drawn(cx, "the document's ask", |shell, cx| {
            shell.present("retire-kept-dialog", cx)
        })
        .await;
    seen.sweep(&window, cx);
    assert!(seen.read_at("kept ask", "document's drafts"));
    window.shell.click("retire-kept-retire", cx);
    window
        .shell
        .until_drawn(cx, "the attention entry", |shell, cx| {
            shell.present(("attention-action", 0usize), cx)
        })
        .await;
    seen.sweep(&window, cx);
    assert!(seen.surfaces("attention") > 0);

    // The Settings page, with the repository's status line (it is
    // paused and needs its key unlocked) and the controls.
    window.shell.keys("secondary-,", cx);
    window.shell.click(("settings-page", 4usize), cx);
    window
        .shell
        .until_drawn(cx, "the page's rows", |shell, cx| {
            shell.present(("polling-name", 0usize), cx)
        })
        .await;
    seen.sweep(&window, cx);
    assert!(seen.surfaces("settings row") > 0);

    // Resume of the pending retirement: the library names the original
    // request, which is sent again and is pending again.
    let request: RequestId = window
        .services
        .scheduler()
        .read_with(cx, |scheduler, _| {
            scheduler
                .store()
                .retirements()
                .iter()
                .find(|entry| entry.item().is_some_and(|(_, id)| id == item_id(PENDING)))
                .and_then(|entry| entry.request())
        })
        .expect("the pending retirement's entry");
    inner_resume(&window, request);
    inner_answer(
        &window,
        answer(State::Pending, Code::DiscoveryPending, Removal::None),
    );
    // (What the retirements since read of the repository's rows is as the
    // library would list them: the one pending row.)
    seed_pending(&window, &root, cx);
    window.shell.click(("pending-resume", 0usize), cx);
    window
        .shell
        .until_drawn(cx, "the resend", |shell, cx| {
            asked(&window, cx).len() == 6 && ended(shell, cx)
        })
        .await;
    seen.sweep(&window, cx);

    // Unlock…: it clears the key's declined mark and resends the pending
    // retirement on the user lane, which asks for the passphrase. The
    // passphrase is typed and supplied.
    inner_answer(&window, answer(State::Retired, Code::Ok, Removal::Observed));
    let provider = window.services.worker().provider();
    window.backend.ask_in_check_now(provider, unlock);
    window.shell.click(("attention-action", 0usize), cx);
    window
        .shell
        .until_drawn(cx, "the passphrase dialog", |shell, cx| {
            shell.present("passphrase-dialog", cx)
        })
        .await;
    seen.sweep(&window, cx);
    window.shell.visual(cx).simulate_input(PASSPHRASE);
    window.shell.run(cx);
    let typed = window
        .shell
        .workspace
        .read_with(cx, |workspace, _| workspace.passphrase())
        .and_then(|dialog| dialog.read_with(cx, |dialog, _| dialog.input()))
        .map(|input| input.read_with(cx, |input, _| input.value().to_string()));
    assert_eq!(
        typed.as_deref(),
        Some(PASSPHRASE),
        "the dialog holds {PASSPHRASE}, masked on screen"
    );
    seen.sweep(&window, cx);
    window.shell.press("enter", cx);
    window
        .shell
        .until_drawn(cx, "the unlock's retirement", |shell, cx| {
            !shell.present("passphrase-dialog", cx)
                && asked(&window, cx).len() == 7
                && ended(shell, cx)
        })
        .await;
    assert_eq!(
        window.backend.asked(),
        [Asked::Supplied],
        "the passphrase was supplied"
    );
    seen.sweep(&window, cx);
    seen.sweep(&window, cx);

    // A quit that waits for a job: the words, and the ten seconds' words.
    // (The drafts folder is writable again, so quitting saves the typed text
    // first and then waits.)
    drop(read_only);
    let gate = window.backend.inner().hold_job(JobKind::IndexRefresh);
    window.services.worker().submit(
        Intent::IndexRefresh { root: a.clone() },
        Lane::User,
        None,
        None,
    );
    gate.wait_reached();
    let quit = window.shell.frame(cx, |win, cx| {
        window
            .shell
            .workspace
            .update(cx, |workspace, cx| workspace.request_quit(win, cx))
    });
    assert!(!quit, "a job is running");
    window.shell.run(cx);
    // The typed text is not kept (unless rooted): quitting asks first (its
    // words are fixed), and Quit anyway goes on to wait for the job.
    if !rooted {
        assert!(window.shell.present("edits-not-kept-dialog", cx));
        seen.sweep(&window, cx);
        window.shell.click("edits-not-kept-quit", cx);
        window.shell.run(cx);
    }
    seen.sweep(&window, cx);
    cx.executor().advance_clock(Duration::from_secs(10));
    window.shell.run(cx);
    seen.sweep(&window, cx);
    assert!(seen.surfaces("quit") > 0, "the quit's words were read");
    drop(gate);

    // Every surface of the drive was read, and a sentinel reached none.
    for surface in [
        "retire offer",
        "retire action",
        "retirement line",
        "attempt entry",
        "attention",
        "pending row",
        "unkept ask",
        "kept ask",
        "passphrase dialog",
        "settings row",
        "quit",
        "quit ask",
        "operation entry",
        "repository row",
        "footer",
        "answer",
        "schedule",
        "retirement entries",
        "record file",
    ] {
        if rooted && matches!(surface, "unkept ask" | "quit ask") {
            continue;
        }
        assert!(seen.surfaces(surface) > 0, "{surface} was read");
    }
    seen.holds_none();
    repositories::was_read(&seen.repositories);
    repositories::password_is_not_eligible(&window, &b, cx);
    for (surface, words) in [
        ("retire offer", "is closed"),
        ("retire offer", "was promoted"),
        ("retirement line", "Retired the copy of"),
        (
            "retirement line",
            "didn't finish. The lists are still catching up.",
        ),
        ("attention", "Unlock\u{2026}"),
        ("attention", "Holding back a"),
        ("kept ask", "kept drafts"),
        ("kept ask", "document's drafts"),
        ("unkept ask", "not kept as a draft"),
        ("pending row", "Resume"),
        ("pending row", "Abandon"),
        ("passphrase dialog", "Passphrase for primary"),
        ("quit", "Finishing"),
        ("quit", "Still stopping"),
        ("quit ask", "Quit anyway"),
        ("settings row", "Paused"),
        ("settings row", "Needs unlocking"),
        ("operation entry", "worktree_not_clean"),
    ] {
        if rooted && matches!(surface, "unkept ask" | "quit ask") {
            continue;
        }
        assert!(seen.read_at(surface, words), "{surface} read {words:?}");
    }
}

/// The library's list of the repository's pending operations, as it stands
/// with the one pending retirement: the row Resume and Abandon are on.
fn seed_pending(window: &PollingWindow, root: &str, cx: &mut TestAppContext) {
    window.services.operations().update(cx, |operations, cx| {
        let stamp = operations.load_pending(root.to_owned());
        operations.apply_pending(
            stamp,
            ReadResult::Ok(manyhands::repository::OperationListDto {
                items: vec![pending_row(OPERATION, "created", None)],
                complete: true,
            }),
        );
        cx.notify();
    });
    window.shell.run(cx);
    window.shell.frame(cx, |_, _| ());
}

/// A canned answer for the next retirement the library is asked for.
fn inner_answer(window: &PollingWindow, envelope: Envelope<MutationDataDto>) {
    window
        .backend
        .inner()
        .answer_job(JobKind::ContextRetire, envelope);
}

/// A canned answer for the next Resume: the library names the request that
/// holds the operation.
fn inner_resume(window: &PollingWindow, request: RequestId) {
    window
        .backend
        .inner()
        .answer_job(JobKind::OperationResume, original_request(request));
}
