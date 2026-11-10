//! Retire… and a pending retirement's Resume and Abandon in the operation
//! area (D6 Task 12a, design E4, E3, decisions 15 and 18, A19, A20, W-6).
//! The library's answers are canned (decision 9); what is checked is what
//! the person sees and what the service receives.

use gpui_kit::{TestAppContext, px, size, test::TestWindowExt};
use manyhands::{
    canonical::ItemId,
    drafts::{CommentTarget, DraftFields, DraftTarget, ItemKind},
    repository::{
        ContextRetirementState as State, OperationId, RequestId, RetirementRemoval as Removal,
    },
    results::ResultCode as Code,
};

use crate::desktop::{
    model::{
        operations::Entered,
        polling::{
            Answer, AttemptTarget, Clock, Disposition, PollAttempt, TargetKind,
            entry::{RetireOffer, offer_text, short_code},
            retire::Verdict,
        },
        tabs::TabKey,
    },
    services::{
        JobKind, ReadResult,
        edit_tabs::TabEditing,
        requests::{DesktopRequest, Intent, Lane},
    },
    test_support::{
        drafts::{FolderMode, superuser},
        fixtures::Fixture,
        retire::{answer, original_request, pending_row},
    },
    view::{
        confirm::CANCEL,
        retire_asks::{RETIRE_CHOICE, SHOW_TAB, kept_message, unkept_message},
        settings::polling_support::Rig,
    },
};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC0";
const COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC1";
const TABS: usize = 120;

fn id(n: usize) -> ItemId {
    format!("01ARZ3NDEKTSV4RRFFQ69G5FD{n}").parse().unwrap()
}

fn retired() -> manyhands::results::Envelope<manyhands::repository::MutationDataDto> {
    answer(State::Retired, Code::Ok, Removal::Observed)
}

/// Repository `a`, published over SSH, with the ticket `TICKET`, in a window
/// of the default size, polling on and `a` scheduled.
fn rig(cx: &mut TestAppContext) -> Rig {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.publish_over_ssh(&root);
    fixture.commit_ticket(&root, TICKET, "Closed elsewhere", "");
    fixture.refresh(&root);
    let rig = Rig::over(fixture, vec![root], cx);
    let _ = rig.services.clone().with_polling();
    rig.schedule(0, cx);
    rig.shell
        .visual(cx)
        .simulate_resize(size(px(1280.), px(840.)));
    rig
}

/// A completed attempt that reported `targets`, as the schedule's own
/// transitions take it.
fn report(rig: &Rig, targets: Vec<(ItemId, TargetKind)>, cx: &mut TestAppContext) {
    let now = rig.clock.now();
    let attempt = PollAttempt {
        root: rig.key(0),
        operation: OperationId::new(),
        answer: Answer::Completed,
        completed_at: Some(now.utc),
        next_eligible_at: Some(now.utc + time::Duration::seconds(300)),
        targets: targets
            .into_iter()
            .map(|(item, kind)| AttemptTarget {
                item_id: Some(item),
                kind: Some(kind),
                disposition: match kind {
                    TargetKind::Ticket => Disposition::Closed,
                    TargetKind::Document => Disposition::Promoted,
                },
            })
            .collect(),
        index_pending: false,
    };
    rig.services.scheduler().update(cx, |scheduler, cx| {
        scheduler.schedule_mut().after(&now, &attempt);
        cx.notify();
    });
    rig.shell.run(cx);
}

/// The element of the action Retire… of `item` in repository `a`'s entry.
fn action(rig: &Rig, item: &ItemId) -> String {
    format!("attempt-{}-retire-{}", rig.key(0), short_code(item))
}

fn words(rig: &Rig, item: &ItemId) -> String {
    format!("attempt-{}-offer-{}", rig.key(0), short_code(item))
}

/// The retirements the person's actions sent, oldest first.
fn retirements(rig: &Rig, cx: &mut TestAppContext) -> Vec<DesktopRequest> {
    rig.sent(cx, |intent| matches!(intent, Intent::ContextRetire { .. }))
}

/// E4, E3, A19, W-6: Retire… shows on every reported `Closed` or `Promoted`
/// item the loop did not retire (skipped, refused, not reached), with the
/// item's short code and the words of its kind, never a title; not on one
/// retired, retiring or pending. Pressed where a click reaches it at the
/// default window, it submits `Intent::ContextRetire` on the user lane under
/// a new request ID, recorded in the in-flight record first, and takes the
/// item out of the refused set.
#[gpui_kit::test]
async fn retire_left_offer(cx: &mut TestAppContext) {
    let rig = rig(cx);
    let root = rig.key(0);
    let (refused, skipped, document, pending, done, running, marked) =
        (id(1), id(2), id(3), id(4), id(5), id(6), id(7));
    report(
        &rig,
        vec![
            (done.clone(), TargetKind::Ticket),
            (refused.clone(), TargetKind::Ticket),
            (skipped.clone(), TargetKind::Ticket),
            (document.clone(), TargetKind::Document),
            (pending.clone(), TargetKind::Ticket),
            (running.clone(), TargetKind::Ticket),
            (marked.clone(), TargetKind::Ticket),
        ],
        cx,
    );
    // A mark holds an item "Retiring…" before its retirement is sent (RT-6).
    rig.services.operations().update(cx, |operations, cx| {
        operations.mark_retiring((
            root.clone(),
            manyhands::repository::ItemDtoKind::Ticket,
            marked.to_string(),
        ));
        cx.notify();
    });
    let owed = Verdict::Owed {
        attention: None,
        catching_up: false,
    };
    rig.services.scheduler().update(cx, |scheduler, cx| {
        let schedule = scheduler.schedule_mut();
        for item in [&done, &pending, &running] {
            schedule.retire_started(&root, TargetKind::Ticket, item);
        }
        schedule.retire_settled(&root, TargetKind::Ticket, &done, &Verdict::Retired);
        schedule.retire_settled(&root, TargetKind::Ticket, &pending, &owed);
        schedule.retirement_answered(&root, TargetKind::Ticket, &refused, &Verdict::Refused);
        cx.notify();
    });
    rig.shell.run(cx);
    rig.shell.frame(cx, |_, _| ());

    // Offered for the three the loop left, in the poll's order, in words
    // that name the short code and no title.
    for (item, kind) in [
        (&refused, TargetKind::Ticket),
        (&skipped, TargetKind::Ticket),
        (&document, TargetKind::Document),
    ] {
        assert!(rig.shell.present(action(&rig, item), cx), "{item}");
        assert_eq!(
            rig.shell.label(words(&rig, item), cx),
            Some(
                RetireOffer {
                    kind,
                    item: item.clone()
                }
                .text()
            ),
        );
    }
    assert_eq!(
        rig.shell.label(words(&rig, &skipped), cx),
        Some(offer_text(TargetKind::Ticket, &short_code(&skipped)))
    );
    for item in [&done, &pending, &running, &marked] {
        assert!(!rig.shell.present(action(&rig, item), cx), "{item}");
    }

    // The action is inside the area, and a click reaches it.
    let area = rig
        .shell
        .frame(cx, |window, _| window.find("operation-area").bounds());
    let button = action(&rig, &refused);
    assert!(rig.shell.tab_reaches(button.clone(), TABS, cx));
    let bounds = rig
        .shell
        .frame(cx, |window, _| window.find(button.clone()).bounds());
    assert!(
        bounds.left() >= area.left() && bounds.right() <= area.right(),
        "Retire… wraps inside the area: {bounds:?} in {area:?}"
    );
    rig.shell
        .backend
        .answer_job(JobKind::ContextRetire, retired());
    rig.shell.click(button.clone(), cx);
    rig.shell
        .until(cx, |operations| {
            operations
                .requests()
                .iter()
                .any(|request| matches!(request.intent, Intent::ContextRetire { .. }))
        })
        .await;
    let sent = retirements(&rig, cx);
    assert_eq!(sent.len(), 1, "one retirement");
    assert_eq!(sent[0].lane, Lane::User);
    match &sent[0].intent {
        Intent::ContextRetire {
            root: sent,
            kind,
            id,
        } => {
            assert_eq!(sent.to_str(), Some(root.as_str()));
            assert_eq!(*kind, manyhands::repository::AuthoringKind::Ticket);
            assert_eq!(*id, refused);
        }
        _ => panic!("not a retirement"),
    }
    rig.shell.until_ended(sent[0].id, cx).await;
    rig.services.scheduler().read_with(cx, |scheduler, _| {
        assert!(!scheduler.schedule().retirement_refused(&root, &refused));
        assert!(
            scheduler.store().retirements().is_empty(),
            "retired: its entry ended"
        );
    });
    // Retired, it is no longer offered; the others still are.
    rig.shell.run(cx);
    rig.shell.frame(cx, |_, _| ());
    assert!(!rig.shell.present(action(&rig, &refused), cx));
    assert!(rig.shell.present(action(&rig, &skipped), cx));
}

/// Registers an item tab that holds unsaved text no draft keeps.
fn unkept_tab(rig: &Rig, cx: &mut TestAppContext) {
    let tab = TabKey::Item {
        repo: rig.key(0),
        kind: manyhands::repository::ItemDtoKind::Ticket,
        id: TICKET.to_owned(),
    };
    rig.services.edit_tabs().update(cx, |tabs, _| {
        tabs.watch(
            tab,
            Box::new(|_| {
                Some(TabEditing {
                    unkept: true,
                    ..TabEditing::default()
                })
            }),
        )
    });
}

fn ticket() -> ItemId {
    TICKET.parse().unwrap()
}

fn tab_open(rig: &Rig, cx: &mut TestAppContext) -> bool {
    let wanted = TabKey::Item {
        repo: rig.key(0),
        kind: manyhands::repository::ItemDtoKind::Ticket,
        id: TICKET.to_owned(),
    };
    rig.shell
        .workspace
        .read_with(cx, |workspace, _| workspace.tab_keys().contains(&wanted))
}

/// E4 step 2, decision 15: with a tab of the item holding text that no draft
/// keeps, nothing is submitted. The dialog gives the count, names no title
/// and no text, and offers Show tab, the default (Enter), which shows the
/// tab, and Cancel (Escape); there is no "Retire anyway". Either way the
/// press is over, and Retire… can be pressed again.
#[gpui_kit::test]
async fn retire_left_unkept_ask(cx: &mut TestAppContext) {
    let rig = rig(cx);
    report(&rig, vec![(ticket(), TargetKind::Ticket)], cx);
    unkept_tab(&rig, cx);
    let button = action(&rig, &ticket());
    rig.shell.frame(cx, |_, _| ());

    rig.shell.click(button.clone(), cx);
    assert!(rig.shell.present("retire-unkept-dialog", cx), "it asks");
    assert_eq!(
        rig.shell.label("retire-unkept-message", cx),
        Some(unkept_message(1))
    );
    assert_eq!(
        rig.shell.label("retire-unkept-show", cx).as_deref(),
        Some(SHOW_TAB)
    );
    assert_eq!(
        rig.shell.label("retire-unkept-cancel", cx).as_deref(),
        Some(CANCEL)
    );
    for anyway in [
        "retire-kept-retire",
        "retire-unkept-retire",
        "retire-unkept-anyway",
    ] {
        assert!(!rig.shell.present(anyway, cx), "{anyway}: no Retire anyway");
    }
    assert!(retirements(&rig, cx).is_empty(), "nothing is submitted");

    // Escape stays as it was; the tab is not shown.
    rig.shell.keys("escape", cx);
    assert!(!rig.shell.present("retire-unkept-dialog", cx));
    assert!(!tab_open(&rig, cx));

    // Again: Enter takes the default, Show tab.
    rig.shell.click(button, cx);
    assert!(
        rig.shell.present("retire-unkept-dialog", cx),
        "pressed again"
    );
    rig.shell.press("enter", cx);
    assert!(!rig.shell.present("retire-unkept-dialog", cx));
    assert!(tab_open(&rig, cx), "Show tab shows the tab");
    assert!(retirements(&rig, cx).is_empty(), "still nothing submitted");
}

/// Stores a draft of the ticket, and one of a comment on it.
fn keep_drafts(rig: &Rig) {
    let store = rig.shell.fixture.draft_store();
    let item = TICKET.parse::<ItemId>().unwrap();
    let ticket = DraftTarget {
        repository: rig.key(0),
        item_kind: ItemKind::Ticket,
        item_id: item.clone(),
        comment: None,
    };
    let comment = DraftTarget {
        comment: Some(CommentTarget {
            comment_id: COMMENT.parse().unwrap(),
            parent_id: None,
        }),
        ..ticket.clone()
    };
    let fields = DraftFields::Ticket {
        title: "T".to_owned(),
        ticket_type: "task".to_owned(),
        status: "open".to_owned(),
        project: None,
        team: None,
        deps: Vec::new(),
        parent: None,
        body: "Words".to_owned(),
    };
    let body = DraftFields::Comment {
        body: "Words".to_owned(),
    };
    drop(
        store
            .create(ticket, None, fields)
            .expect("the ticket's draft"),
    );
    drop(
        store
            .create(comment, None, body)
            .expect("the comment's draft"),
    );
}

/// E4 step 3, E5, decision 18: with a kept draft, the dialog warns that a
/// closed ticket's drafts can only be exported afterwards, and for a promoted
/// document that they stay restorable but comment drafts can only be
/// exported. It offers Retire and Cancel, the default: Enter retires
/// nothing; Retire, reached by Tab and pressed by Enter, submits one
/// retirement on the user lane.
#[gpui_kit::test]
async fn retire_left_kept_draft_ask(cx: &mut TestAppContext) {
    let rig = rig(cx);
    report(&rig, vec![(ticket(), TargetKind::Ticket)], cx);
    keep_drafts(&rig);
    // A comment being written, whose newest text is not yet stored.
    let third = DraftTarget {
        repository: rig.key(0),
        item_kind: ItemKind::Ticket,
        item_id: ticket(),
        comment: Some(CommentTarget {
            comment_id: "01ARZ3NDEKTSV4RRFFQ69G5FC2".parse().unwrap(),
            parent_id: None,
        }),
    };
    let drafts = rig.services.drafts().clone();
    let begun = drafts.update(cx, |registry, cx| {
        registry.begin(
            third,
            None,
            DraftFields::Comment {
                body: "Begun".to_owned(),
            },
            cx,
        )
    });
    let live = begun.await.expect("a session");
    drafts.update(cx, |registry, cx| {
        registry.edit(
            live,
            DraftFields::Comment {
                body: "Newer".to_owned(),
            },
            cx,
        )
    });
    assert!(drafts.read_with(cx, |registry, _| registry.unsaved()));
    let button = action(&rig, &ticket());
    rig.shell.frame(cx, |_, _| ());
    let asked = |rig: &Rig, cx: &mut TestAppContext| {
        rig.shell
            .workspace
            .read_with(cx, |workspace, _| workspace.retire_asked.len())
    };

    // The words, per kind.
    let ticket_words = kept_message(TargetKind::Ticket, Some(3));
    assert_eq!(
        ticket_words,
        "This ticket has 3 kept drafts. After it is retired you can still export them, but \
         not restore them into it."
    );
    assert_eq!(
        kept_message(TargetKind::Document, Some(2)),
        "You can still restore or export the document's drafts. Drafts of comments can only \
         be exported."
    );

    rig.shell.click(button.clone(), cx);
    rig.shell
        .until_drawn(cx, "the question", |shell, cx| {
            shell.present("retire-kept-dialog", cx)
        })
        .await;
    assert_eq!(
        rig.shell.label("retire-kept-message", cx),
        Some(ticket_words),
        "the item's draft and its comments', counted once each"
    );
    assert_eq!(asked(&rig, cx), 1, "a second press waits");
    assert_eq!(
        rig.shell.label("retire-kept-cancel", cx).as_deref(),
        Some(CANCEL)
    );
    assert_eq!(
        rig.shell.label("retire-kept-retire", cx).as_deref(),
        Some(RETIRE_CHOICE)
    );

    // Enter is Cancel: nothing is submitted, and the press is over.
    rig.shell.press("enter", cx);
    assert!(!rig.shell.present("retire-kept-dialog", cx));
    assert!(retirements(&rig, cx).is_empty());
    assert_eq!(asked(&rig, cx), 0, "the press is over");

    // Retire, by keyboard.
    rig.shell.click(button, cx);
    rig.shell
        .until_drawn(cx, "the question again", |shell, cx| {
            shell.present("retire-kept-dialog", cx)
        })
        .await;
    rig.shell
        .backend
        .answer_job(JobKind::ContextRetire, retired());
    assert!(rig.shell.tab_reaches("retire-kept-retire-button", 6, cx));
    rig.shell.press("enter", cx);
    rig.shell
        .until(cx, |operations| {
            operations
                .requests()
                .iter()
                .any(|request| matches!(request.intent, Intent::ContextRetire { .. }))
        })
        .await;
    let sent = retirements(&rig, cx);
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].lane, Lane::User);
    rig.shell.until_ended(sent[0].id, cx).await;
    // The item's sessions were written before it was sent (E4 step 4), and
    // the drafts are still in the store (E5): retirement touched none.
    assert!(!drafts.read_with(cx, |registry, _| registry.unsaved()));
    assert_eq!(rig.shell.fixture.draft_store().list().unwrap().len(), 3);
    assert_eq!(asked(&rig, cx), 0);
}

/// A20, K-27, E4: a pending retirement's row offers Resume when the
/// in-flight record's entry names it, and Abandon whatever. Resume is
/// reached and pressed by keyboard; its `original_request_required` answer
/// sends the retirement again on the user lane under the request the answer
/// names. A row with no entry offers Abandon alone, which opens the existing
/// confirmation, by keyboard too.
#[gpui_kit::test]
async fn retire_pending_row_by_keyboard(cx: &mut TestAppContext) {
    let rig = rig(cx);
    let root = rig.key(0);
    let (request, operation) = (RequestId::new(), OperationId::new());
    let other = OperationId::new();
    rig.services.scheduler().update(cx, |scheduler, cx| {
        scheduler.hold_retirement(request, &root, TargetKind::Ticket, &ticket(), operation, cx)
    });
    let seed = |cx: &mut TestAppContext| {
        rig.services.operations().update(cx, |operations, cx| {
            let stamp = operations.load_pending(root.clone());
            operations.apply_pending(
                stamp,
                ReadResult::Ok(manyhands::repository::OperationListDto {
                    items: vec![
                        pending_row(&operation.to_string(), "created", None),
                        pending_row(&other.to_string(), "created", None),
                    ],
                    complete: true,
                }),
            );
            cx.notify();
        });
    };
    seed(cx);
    rig.shell.run(cx);
    rig.shell.frame(cx, |_, _| ());
    let held = rig.services.operations().read_with(cx, |operations, _| {
        operations.retirement_of(&root, operation).cloned()
    });
    assert_eq!(
        held,
        Some(Entered {
            root: root.clone(),
            kind: TargetKind::Ticket,
            item: ticket(),
            request: Some(request),
            operation: Some(operation),
        })
    );

    // Row 0 (the entry's) offers Resume and Abandon; row 1, Abandon alone.
    assert!(rig.shell.present(("pending-resume", 0usize), cx));
    assert!(rig.shell.present(("pending-abandon", 0usize), cx));
    assert!(!rig.shell.present(("pending-resume", 1usize), cx));
    assert!(rig.shell.present(("pending-abandon", 1usize), cx));

    // Abandon, by keyboard, on the row no entry names.
    rig.shell.keys("secondary-4", cx);
    assert!(rig.shell.tab_reaches(("pending-abandon", 1usize), TABS, cx));
    rig.shell.press("enter", cx);
    assert!(
        rig.shell.present("confirm-dialog", cx),
        "the confirmation opens"
    );
    rig.shell.keys("escape", cx);
    assert!(!rig.shell.present("confirm-dialog", cx));
    // The abandon's preview read the repository's rows again: list them as
    // the library would still.
    seed(cx);
    rig.shell.run(cx);

    // Resume, by keyboard.
    rig.shell
        .backend
        .answer_job(JobKind::OperationResume, original_request(request));
    rig.shell
        .backend
        .answer_job(JobKind::ContextRetire, retired());
    rig.shell.keys("secondary-4", cx);
    assert!(rig.shell.tab_reaches(("pending-resume", 0usize), TABS, cx));
    rig.shell.press("enter", cx);
    rig.shell
        .until(cx, |operations| {
            operations
                .requests()
                .iter()
                .any(|request| matches!(request.intent, Intent::ContextRetire { .. }))
        })
        .await;
    let sent = retirements(&rig, cx);
    assert_eq!(sent.len(), 1);
    assert_eq!((sent[0].lane, sent[0].request_id), (Lane::User, request));
}

/// E4 step 4: the item's draft sessions are written while it is read-only,
/// and a write that fails means text no draft keeps: nothing is submitted,
/// the item is not left "Retiring…", and the person is told, as for unkept
/// text.
#[cfg(unix)]
#[gpui_kit::test]
async fn retire_flush_failure_submits_nothing(cx: &mut TestAppContext) {
    if superuser() {
        return;
    }
    let rig = rig(cx);
    report(&rig, vec![(ticket(), TargetKind::Ticket)], cx);
    let drafts = rig.services.drafts().clone();
    let target = DraftTarget {
        repository: rig.key(0),
        item_kind: ItemKind::Ticket,
        item_id: ticket(),
        comment: Some(CommentTarget {
            comment_id: COMMENT.parse().unwrap(),
            parent_id: None,
        }),
    };
    let begun = drafts.update(cx, |registry, cx| {
        registry.begin(
            target,
            None,
            DraftFields::Comment {
                body: "Begun".to_owned(),
            },
            cx,
        )
    });
    let live = begun.await.expect("a session");
    drafts.update(cx, |registry, cx| {
        registry.edit(
            live,
            DraftFields::Comment {
                body: "Newer".to_owned(),
            },
            cx,
        )
    });
    // The drafts folder becomes read-only, so the newest text cannot be
    // written (the listing still reads).
    let _read_only = FolderMode::set(&rig.shell.fixture.data_dir().join("drafts"), 0o500);

    rig.shell.frame(cx, |_, _| ());
    rig.shell.click(action(&rig, &ticket()), cx);
    rig.shell
        .until_drawn(cx, "the kept drafts question", |shell, cx| {
            shell.present("retire-kept-dialog", cx)
        })
        .await;
    rig.shell.click("retire-kept-retire", cx);
    rig.shell
        .until_drawn(cx, "the unkept text question", |shell, cx| {
            shell.present("retire-unkept-dialog", cx)
        })
        .await;
    assert!(retirements(&rig, cx).is_empty(), "nothing is submitted");
    let retiring = rig.services.operations().read_with(cx, |operations, _| {
        operations.is_retiring(
            &rig.key(0),
            manyhands::repository::ItemDtoKind::Ticket,
            TICKET,
        )
    });
    assert!(!retiring, "the mark is cleared");
}

/// E4: a second press of Retire… while the first waits for the fresh draft
/// listing starts nothing: one question, and once it is answered no other
/// follows.
#[gpui_kit::test]
async fn retire_second_press_waits(cx: &mut TestAppContext) {
    use crate::desktop::services::ReadKind;
    let rig = rig(cx);
    report(&rig, vec![(ticket(), TargetKind::Ticket)], cx);
    keep_drafts(&rig);
    rig.shell.frame(cx, |_, _| ());
    let release = rig.shell.backend.hold_read(ReadKind::Drafts);
    let button = action(&rig, &ticket());
    rig.shell.click(button.clone(), cx);
    rig.shell.click(button, cx);
    assert!(
        !rig.shell.present("retire-kept-dialog", cx),
        "still listing"
    );
    release.send(()).unwrap();
    rig.shell
        .until_drawn(cx, "the question", |shell, cx| {
            shell.present("retire-kept-dialog", cx)
        })
        .await;
    rig.shell.press("enter", cx);
    for _ in 0..20 {
        rig.shell.run(cx);
        cx.executor()
            .timer(std::time::Duration::from_millis(10))
            .await;
    }
    assert!(
        !rig.shell.present("retire-kept-dialog", cx),
        "one question, not two"
    );
    assert!(retirements(&rig, cx).is_empty());
}
