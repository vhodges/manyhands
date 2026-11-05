//! The header's Share and the repository detail's Share main (design,
//! Lane B, Share controls), over real repositories: Share follows Edit and
//! sends `item sync`, with the remote in its tooltip; with no publication
//! remote it reads Share… and opens Settings › Remotes; while a share of
//! the same target is queued or running it reads Sharing… and queues
//! nothing more; Share main sends `repo sync`.

use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};

use gpui_kit::{TestAppContext, test::TestWindowExt};
use manyhands::{
    canonical::{CONFIG_PATH, ItemId},
    repository::{
        AuthoringKind, ConflictNextAction, ConflictNextDto, ItemDtoKind, MutationDataDto,
        ProgressEvent, ProgressStage, RequestId, SynchronizationConflictMutationDto,
        SynchronizationConflictType,
    },
    results::{Effects, Envelope, IntegrationEffect, PublicationEffect, ResultCode, Scope},
};

use super::{SHARE_BUTTON, SHARE_NOTE, SHARE_SET_UP, share_tooltip};
use crate::desktop::{
    model::{
        item::SHARE,
        navigation::OpenTarget,
        operations::{
            CONFLICT_BANNER, NEEDS_ATTENTION, SHARED, STOPPED_AFTER_PART, STOPPED_BEFORE_SENDING,
        },
        stamp::SettingsPage,
    },
    services::{
        JobKind,
        requests::{CommentBody, DesktopRequest, DesktopRequestId, Intent, Lane, RequestState},
        worker::WorkerEvent,
    },
    test_support::{
        drafts::store_draft,
        fixtures::{Fixture, item_id, ticket_source},
    },
    view::{
        harness::{Shell, WAIT, key},
        item::pill::{PillState, pill_words},
        operations::{STOP, STOPPING, request_id},
        settings::Shown,
        sidebar::{SHARE_MAIN, SHARE_MAIN_BUTTON, SHARE_MAIN_SET_UP, SHARING},
    },
};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FS0";

/// Repository `a` holding one ticket, with `origin` selected for
/// publication when `remote`, indexed, in an open window.
fn shell(remote: bool, cx: &mut TestAppContext) -> (Shell, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Shared", "");
    if remote {
        fixture.publish_over_ssh(&root);
    }
    fixture.refresh(&root);
    (Shell::open(fixture, cx), root)
}

fn open_ticket(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    let target = OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Ticket,
        id: item_id(TICKET),
    };
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
}

fn focused(shell: &Shell, id: &'static str, cx: &mut TestAppContext) -> bool {
    shell.frame(cx, |window, _| {
        window
            .try_find(id)
            .is_some_and(|element| element.focused() == Some(true))
    })
}

/// Whether `request` is `item sync` of the ticket in `root`.
fn item_sync_of(request: &DesktopRequest, root: &Path) -> bool {
    matches!(
        &request.intent,
        Intent::ItemSync { root: asked, kind: AuthoringKind::Ticket, id }
            if asked == root && *id == item_id(TICKET)
    )
}

fn item_syncs(shell: &Shell, root: &Path, cx: &mut TestAppContext) -> usize {
    let root = root.to_owned();
    shell
        .requests(cx, move |request| item_sync_of(request, &root))
        .len()
}

fn repo_syncs(shell: &Shell, root: &Path, cx: &mut TestAppContext) -> usize {
    let root = root.to_owned();
    shell
        .requests(cx, move |request| {
            matches!(&request.intent, Intent::RepoSync { root: asked } if *asked == root)
        })
        .len()
}

/// With a publication remote, Share comes after Edit, in the tab order
/// too, names the remote in its tooltip, and sends `item sync` of the
/// item.
#[gpui_kit::test]
async fn share_follows_edit_when_a_remote_exists(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(true, cx);
    open_ticket(&shell, &root, cx);
    assert_eq!(shell.label(SHARE_BUTTON, cx).as_deref(), Some(SHARE));
    assert_eq!(
        shell.label(SHARE_NOTE, cx),
        Some(share_tooltip("origin")),
        "the tooltip names the remote"
    );

    shell.keys("secondary-3", cx);
    let mut presses = 0;
    while !focused(&shell, "item-edit", cx) {
        presses += 1;
        assert!(presses < 6, "Tab never reached Edit");
        shell.keys("tab", cx);
    }
    shell.keys("tab", cx);
    assert!(focused(&shell, SHARE_BUTTON, cx), "Share after Edit");

    let held = shell.backend.hold_job(JobKind::SynchronizeItem);
    shell.click(SHARE_BUTTON, cx);
    held.wait_reached();
    let sent = shell.requests(cx, |request| item_sync_of(request, &root));
    assert_eq!(sent.len(), 1, "Share sent item sync of the item");
    held.release();
    shell.until_ended(sent[0], cx).await;
    assert_eq!(shell.label(SHARE_BUTTON, cx).as_deref(), Some(SHARE));
}

/// With no publication remote, the header reads Share…, and pressing it
/// opens Settings › Remotes, sending nothing.
#[gpui_kit::test]
fn share_without_a_remote_reads_share_and_opens_remotes(cx: &mut TestAppContext) {
    let (shell, root) = shell(false, cx);
    open_ticket(&shell, &root, cx);
    assert_eq!(shell.label(SHARE_BUTTON, cx).as_deref(), Some(SHARE_SET_UP));
    assert!(!shell.present(SHARE_NOTE, cx), "no remote to name");

    shell.click(SHARE_BUTTON, cx);
    let shown = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.settings_shown(cx));
    assert!(matches!(shown, Shown::Settings(SettingsPage::Remotes)));
    assert_eq!(item_syncs(&shell, &root, cx), 0, "nothing was sent");
}

/// A remote selected for publication through the request path, as the
/// Remotes page selects one, reaches Share once the request finishes:
/// navigation reloads and gives the tab the remote, with no read of its
/// own.
#[gpui_kit::test]
async fn a_remote_selected_by_request_reaches_share(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Shared", "");
    fixture.add_remote(&root, "origin", "ssh://git@example.invalid/team/repo.git");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    open_ticket(&shell, &root, cx);
    assert_eq!(shell.label(SHARE_BUTTON, cx).as_deref(), Some(SHARE_SET_UP));

    let id = shell.workspace.update(cx, |workspace, _| {
        workspace.services().worker().submit(
            Intent::RemoteSelect {
                root: root.clone(),
                name: Some("origin".to_owned()),
            },
            Lane::User,
            None,
            None,
        )
    });
    shell.until_ended(id, cx).await;
    shell.run(cx);
    assert_eq!(shell.label(SHARE_BUTTON, cx).as_deref(), Some(SHARE));
    assert_eq!(shell.label(SHARE_NOTE, cx), Some(share_tooltip("origin")));
}

/// While a share of the item is queued or running, Share reads Sharing…
/// and is disabled: a second press, or the `ShareItem` action, queues
/// nothing.
#[gpui_kit::test]
async fn a_second_press_while_sharing_queues_nothing(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(true, cx);
    open_ticket(&shell, &root, cx);
    let held = shell.backend.hold_job(JobKind::SynchronizeItem);
    shell.click(SHARE_BUTTON, cx);
    held.wait_reached();
    assert_eq!(shell.label(SHARE_BUTTON, cx).as_deref(), Some(SHARING));

    shell.click(SHARE_BUTTON, cx);
    let pane = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.item_pane())
        .expect("the ticket's tab is shown");
    let taken = shell
        .visual(cx)
        .update(|window, cx| pane.update(cx, |pane, cx| pane.share_item(window, cx)));
    assert!(taken, "the action is Share's, though it queues nothing");
    shell.run(cx);
    let sent = shell.requests(cx, |request| item_sync_of(request, &root));
    assert_eq!(sent.len(), 1, "one share queued");

    held.release();
    shell.until_ended(sent[0], cx).await;
    assert_eq!(shell.label(SHARE_BUTTON, cx).as_deref(), Some(SHARE));
}

/// The repository detail offers Share main, which sends `repo sync` of
/// the repository; while it is running it reads Sharing… and queues
/// nothing more.
#[gpui_kit::test]
async fn share_main_in_the_repository_detail_submits_repo_sync(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(true, cx);
    shell.select(&root, cx);
    assert_eq!(
        shell.label(SHARE_MAIN_BUTTON, cx).as_deref(),
        Some(SHARE_MAIN)
    );

    let held = shell.backend.hold_job(JobKind::SynchronizeRepo);
    shell.click(SHARE_MAIN_BUTTON, cx);
    held.wait_reached();
    assert_eq!(
        repo_syncs(&shell, &root, cx),
        1,
        "Share main sent repo sync"
    );
    assert_eq!(shell.label(SHARE_MAIN_BUTTON, cx).as_deref(), Some(SHARING));
    // The button is disabled, so a press reaches nothing: Share main's own
    // guard is asked directly, as a press made before the frame is.
    let workspace = shell.workspace.clone();
    let repo = key(&root);
    shell.visual(cx).update(|window, cx| {
        workspace.update(cx, |workspace, cx| {
            workspace.share_main(repo, true, window, cx)
        })
    });
    shell.run(cx);
    assert_eq!(
        repo_syncs(&shell, &root, cx),
        1,
        "a second press queued nothing"
    );

    let sent = shell.requests(cx, |request| {
        matches!(request.intent, Intent::RepoSync { .. })
    });
    held.release();
    shell.until_ended(sent[0], cx).await;
    assert_eq!(
        shell.label(SHARE_MAIN_BUTTON, cx).as_deref(),
        Some(SHARE_MAIN)
    );
}

/// With no publication remote, the repository detail reads Share main…,
/// and pressing it opens Settings › Remotes, sending nothing.
#[gpui_kit::test]
fn share_main_without_a_remote_reads_share_main_and_opens_remotes(cx: &mut TestAppContext) {
    let (shell, root) = shell(false, cx);
    shell.select(&root, cx);
    assert_eq!(
        shell.label(SHARE_MAIN_BUTTON, cx).as_deref(),
        Some(SHARE_MAIN_SET_UP)
    );

    shell.click(SHARE_MAIN_BUTTON, cx);
    let shown = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.settings_shown(cx));
    assert!(matches!(shown, Shown::Settings(SettingsPage::Remotes)));
    assert_eq!(repo_syncs(&shell, &root, cx), 0, "nothing was sent");
}

/// While an item tab is shown, the repository detail at the end of its
/// scroll region offers Share main too, and it sends `repo sync`.
#[gpui_kit::test]
async fn share_main_at_the_end_of_an_item_tab_submits_repo_sync(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(true, cx);
    shell.select(&root, cx);
    open_ticket(&shell, &root, cx);
    assert!(
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.item_pane())
            .is_some(),
        "the ticket's tab is shown"
    );
    // The detail ends the item's scroll region: End brings it on screen.
    shell.keys("secondary-3", cx);
    shell.keys("end", cx);
    assert_eq!(
        shell.label(SHARE_MAIN_BUTTON, cx).as_deref(),
        Some(SHARE_MAIN)
    );

    let held = shell.backend.hold_job(JobKind::SynchronizeRepo);
    shell.click(SHARE_MAIN_BUTTON, cx);
    held.wait_reached();
    assert_eq!(
        repo_syncs(&shell, &root, cx),
        1,
        "Share main sent repo sync"
    );
    assert_eq!(shell.label(SHARE_MAIN_BUTTON, cx).as_deref(), Some(SHARING));
    assert_eq!(item_syncs(&shell, &root, cx), 0, "not the item's share");

    let sent = shell.requests(cx, |request| {
        matches!(request.intent, Intent::RepoSync { .. })
    });
    held.release();
    shell.until_ended(sent[0], cx).await;
}

// Task 9: stages and Stop (design, Lane B, Stage text and Stop at a safe
// point).

/// Fabricated requests stand far above the worker's own numbering.
const FABRICATED: u64 = 1_000_000;

fn ticket_id() -> ItemId {
    item_id(TICKET)
}

fn item_sync(root: &Path) -> Intent {
    Intent::ItemSync {
        root: root.to_owned(),
        kind: AuthoringKind::Ticket,
        id: ticket_id(),
    }
}

fn comment_add(root: &Path) -> Intent {
    Intent::CommentAdd {
        root: root.to_owned(),
        kind: AuthoringKind::Ticket,
        id: ticket_id(),
        comment_id: item_id("01ARZ3NDEKTSV4RRFFQ69G5FS1"),
        parent_id: None,
        body: CommentBody::new("A comment.".to_owned()),
    }
}

/// Puts a request of `intent`, numbered `n` above the worker's, in the
/// request model as running, through `Operations::apply`.
fn running(shell: &Shell, n: u64, intent: Intent, cx: &mut TestAppContext) -> DesktopRequest {
    let request = DesktopRequest {
        id: DesktopRequestId(FABRICATED + n),
        request_id: RequestId::new(),
        intent,
        confirmation: None,
        lane: Lane::User,
        attempt: 1,
        state: RequestState::Queued,
    };
    let operations = shell.operations(cx);
    let queued = request.clone();
    operations.update(cx, |operations, cx| {
        operations.apply(WorkerEvent::Queued(queued));
        operations.apply(WorkerEvent::Started(request.id));
        cx.notify();
    });
    shell.run(cx);
    request
}

/// Applies a constructed progress event of `stage` to `request`.
fn reach(shell: &Shell, request: &DesktopRequest, stage: ProgressStage, cx: &mut TestAppContext) {
    let event = ProgressEvent {
        request_id: request.request_id.to_string(),
        operation_id: None,
        command: "item sync".to_owned(),
        scope: Scope::default(),
        stage,
        effects: Effects::not_requested(),
    };
    let operations = shell.operations(cx);
    let id = request.id;
    operations.update(cx, |operations, cx| {
        operations.apply(WorkerEvent::Progress(id, event));
        cx.notify();
    });
    shell.run(cx);
}

/// Each stage's text, after the pill's "Sharing · " or "Posting · ".
const SHARE_STAGES: [(ProgressStage, &str); 9] = [
    (ProgressStage::Accepted, "Getting ready…"),
    (ProgressStage::Observing, "Getting ready…"),
    (ProgressStage::Connecting, "Connecting to origin…"),
    (ProgressStage::Fetching, "Bringing in changes…"),
    (ProgressStage::UpdatingLocal, "Merging changes…"),
    (ProgressStage::Pushing, "Sending your saved changes…"),
    (ProgressStage::Verifying, "Checking it arrived…"),
    (ProgressStage::Indexing, "Updating the list…"),
    (ProgressStage::Stopping, "Stopping at a safe point…"),
];

const POST_STAGES: [(ProgressStage, &str); 10] = [
    (ProgressStage::Accepted, "Getting a private copy ready…"),
    (ProgressStage::Observing, "Getting a private copy ready…"),
    (ProgressStage::Writing, "Saving…"),
    (ProgressStage::Connecting, "Connecting to origin…"),
    (ProgressStage::Fetching, "Bringing in changes…"),
    (ProgressStage::UpdatingLocal, "Merging changes…"),
    (ProgressStage::Pushing, "Sending your saved changes…"),
    (ProgressStage::Verifying, "Checking it arrived…"),
    (ProgressStage::Indexing, "Updating the list…"),
    (ProgressStage::Stopping, "Stopping at a safe point…"),
];

/// Each stage a share or a post reports reads as the design's text, in
/// the item's pill and in its operation area entry.
#[gpui_kit::test]
fn each_stage_shows_its_text_for_share_and_post(cx: &mut TestAppContext) {
    let (shell, root) = shell(true, cx);
    open_ticket(&shell, &root, cx);

    let share = running(&shell, 1, item_sync(&root), cx);
    for (stage, text) in SHARE_STAGES {
        reach(&shell, &share, stage, cx);
        let words = format!("Sharing · {text}");
        assert_eq!(
            shell.label("item-pill", cx).as_deref(),
            Some(words.as_str()),
            "{stage:?}"
        );
        assert_eq!(
            shell.label(request_id(share.id, "state"), cx).as_deref(),
            Some(words.as_str()),
            "{stage:?}"
        );
    }
    finish(&shell, share.id, cx);

    let post = running(&shell, 2, comment_add(&root), cx);
    for (stage, text) in POST_STAGES {
        reach(&shell, &post, stage, cx);
        let words = format!("Posting · {text}");
        assert_eq!(
            shell.label("item-pill", cx).as_deref(),
            Some(words.as_str()),
            "{stage:?}"
        );
        assert_eq!(
            shell.label(request_id(post.id, "state"), cx).as_deref(),
            Some(words.as_str()),
            "{stage:?}"
        );
    }
}

/// Ends the fabricated request `id` with a success.
fn finish(shell: &Shell, id: DesktopRequestId, cx: &mut TestAppContext) {
    let operations = shell.operations(cx);
    let envelope = Envelope::classified_mutation(
        "item sync",
        Scope::default(),
        false,
        ResultCode::Ok,
        Effects::not_requested(),
        false,
    );
    operations.update(cx, |operations, cx| {
        operations.apply(WorkerEvent::Finished(id, Box::new(envelope)));
        cx.notify();
    });
    shell.run(cx);
}

/// With no publication remote, a post reads "Posting · …" at every stage
/// it can reach, never "Sharing", and offers no Stop (R-15): it has no
/// safe point.
#[gpui_kit::test]
fn a_post_without_a_remote_never_says_sharing(cx: &mut TestAppContext) {
    let (shell, root) = shell(false, cx);
    open_ticket(&shell, &root, cx);
    let post = running(&shell, 1, comment_add(&root), cx);
    for stage in [
        ProgressStage::Accepted,
        ProgressStage::Observing,
        ProgressStage::Writing,
        ProgressStage::Indexing,
    ] {
        reach(&shell, &post, stage, cx);
        for shown in [
            shell.label("item-pill", cx),
            shell.label(request_id(post.id, "state"), cx),
        ] {
            let shown = shown.expect("the post's progress is shown");
            assert!(shown.starts_with("Posting · "), "{stage:?}: {shown}");
            assert!(!shown.contains("Sharing"), "{stage:?}: {shown}");
        }
        assert!(!shell.present(request_id(post.id, "stop"), cx), "{stage:?}");
    }
}

/// Repository `a` holding the ticket, published over SSH with the
/// selection committed, indexed, in an open window.
fn shared_shell(cx: &mut TestAppContext) -> (Shell, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Shared", "");
    fixture.publish_over_ssh(&root);
    let config = std::fs::read(root.join(CONFIG_PATH)).unwrap();
    fixture.commit_file(&root, CONFIG_PATH, &config);
    fixture.refresh(&root);
    (Shell::open(fixture, cx), root)
}

/// R-1: an accepted share offers Stop at a safe point in place of
/// Finishing; pressed, it cancels the running job's token and reads
/// Stopping…, and the share then stops at a safe point and says so.
#[gpui_kit::test]
async fn an_accepted_share_offers_stop_and_stopping_signals_it(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shared_shell(cx);
    // Held after its acceptance, before its domain call (fact 16).
    let (reached_tx, reached) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();
    shell
        .fixture
        .service()
        .set_request_hook_for_testing(move || {
            let _ = reached_tx.send(());
            let _ = released.recv_timeout(WAIT);
        });
    shell.select(&root, cx);
    shell.click(SHARE_MAIN_BUTTON, cx);
    reached.recv_timeout(WAIT).expect("the share was accepted");
    shell
        .until(cx, |operations| {
            operations.requests().iter().any(|request| {
                matches!(request.intent, Intent::RepoSync { .. })
                    && matches!(request.state, RequestState::Running { accepted: true, .. })
            })
        })
        .await;
    let id = shell.requests(cx, |request| {
        matches!(request.intent, Intent::RepoSync { .. })
    })[0];
    assert_eq!(
        shell.label(request_id(id, "stop"), cx).as_deref(),
        Some(STOP)
    );
    assert!(!shell.present(request_id(id, "cancel"), cx));

    shell.click(request_id(id, "stop"), cx);
    let worker = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().worker().clone());
    let job = worker.running_job().expect("the share still runs");
    assert_eq!(job.id, id);
    assert!(job.cancelled, "Stop cancelled its token");
    assert_eq!(
        shell.label(request_id(id, "stop"), cx).as_deref(),
        Some(STOPPING)
    );

    drop(release);
    shell.until_ended(id, cx).await;
    assert!(!shell.present(request_id(id, "stop"), cx));
    // It stopped at its first safe point, before anything was sent.
    assert_eq!(
        shell.label(request_id(id, "code"), cx).as_deref(),
        Some("cancelled")
    );
    assert_eq!(
        shell.label(request_id(id, "outcome"), cx).as_deref(),
        Some(STOPPED_BEFORE_SENDING)
    );
}

/// A stopped share says whether part of the work finished, and offers
/// Share, which sends a new request.
#[gpui_kit::test]
async fn a_stopped_share_says_what_completed_and_offers_share(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(true, cx);
    shell.select(&root, cx);
    let completed = Effects {
        integration: IntegrationEffect::Complete,
        ..Effects::not_requested()
    };
    let mut seen = Vec::new();
    for (effects, text) in [
        (Effects::not_requested(), STOPPED_BEFORE_SENDING),
        (completed, STOPPED_AFTER_PART),
    ] {
        let stopped = Envelope::classified_mutation(
            "repo sync",
            Scope::default(),
            true,
            ResultCode::Cancelled,
            effects,
            false,
        );
        let held = shell
            .backend
            .hold_job_return_answering(JobKind::SynchronizeRepo, stopped);
        match seen.last() {
            None => shell.click(SHARE_MAIN_BUTTON, cx),
            Some(&previous) => shell.click(request_id(previous, "share"), cx),
        }
        held.wait_reached();
        let sent = shell.requests(cx, |request| {
            matches!(request.intent, Intent::RepoSync { .. })
        });
        assert_eq!(sent.len(), seen.len() + 1, "Share sent a new request");
        let id = sent[0];
        assert!(!seen.contains(&id));
        held.release();
        shell.until_ended(id, cx).await;
        assert_eq!(
            shell.label(request_id(id, "outcome"), cx).as_deref(),
            Some(text)
        );
        assert_eq!(
            shell.label(request_id(id, "share"), cx).as_deref(),
            Some(SHARE)
        );
        seen.push(id);
    }
}

// Task 10a: outcomes in the item tab, and Share… on its own repository.

/// Ends the fabricated request `id` with `envelope`.
fn finish_with(
    shell: &Shell,
    id: DesktopRequestId,
    envelope: Envelope<MutationDataDto>,
    cx: &mut TestAppContext,
) {
    let operations = shell.operations(cx);
    operations.update(cx, |operations, cx| {
        operations.apply(WorkerEvent::Finished(id, Box::new(envelope)));
        cx.notify();
    });
    shell.run(cx);
}

fn conflict() -> Envelope<MutationDataDto> {
    Envelope::classified_mutation(
        "item sync",
        Scope::default(),
        false,
        ResultCode::WorktreeConflicted,
        Effects::not_requested(),
        false,
    )
    .with_data(MutationDataDto::SynchronizationConflict(
        SynchronizationConflictMutationDto {
            r#type: SynchronizationConflictType::SynchronizationConflict,
            conflict: ConflictNextDto {
                operation_id: "01M4CC0VN1KTQFSVR99378TDHV".to_owned(),
                next_action: ConflictNextAction::ALL[0],
            },
        },
    ))
}

/// A share of the item that stopped on a conflict raises the conflict
/// banner, which says the work is safe and offers Review changes (decision
/// 13; D5 B5, E-7), and the pill needs attention; dismissing its entry
/// ends both.
#[gpui_kit::test]
fn the_conflict_banner_says_work_is_safe_and_offers_review_changes(cx: &mut TestAppContext) {
    let (shell, root) = shell(true, cx);
    open_ticket(&shell, &root, cx);
    assert!(!shell.present("item-conflict", cx));
    let share = running(&shell, 1, item_sync(&root), cx);
    finish_with(&shell, share.id, conflict(), cx);

    assert_eq!(
        shell.label("item-conflict", cx).as_deref(),
        Some(CONFLICT_BANNER)
    );
    assert_eq!(
        CONFLICT_BANNER,
        "You and someone else both changed this since it was last shared. Your work is \
         safe."
    );
    assert_eq!(
        shell.label("item-pill", cx).as_deref(),
        Some(NEEDS_ATTENTION)
    );
    // The banner's frame holds its words and one control: Review changes.
    assert_eq!(
        shell.label("item-review-changes", cx).as_deref(),
        Some(crate::desktop::view::conflict::REVIEW_CHANGES)
    );
    let others = shell.frame(cx, |window, _| {
        let words = window
            .try_find("item-conflict")
            .expect("the banner")
            .path()
            .to_vec();
        let review = window
            .try_find("item-review-changes")
            .expect("Review changes")
            .path()
            .to_vec();
        let frame = &words[..words.len() - 1];
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .filter(|element| {
                let path = element.path();
                path.starts_with(frame)
                    && path != frame
                    && path != words.as_slice()
                    && !path.starts_with(&review)
            })
            .count()
    });
    assert_eq!(others, 0, "the banner offers Review changes only");

    shell.click(request_id(share.id, "dismiss"), cx);
    assert!(!shell.present("item-conflict", cx));
    assert_ne!(
        shell.label("item-pill", cx).as_deref(),
        Some(NEEDS_ATTENTION)
    );
}

/// A finished share of the item says its outcome in the pill until its
/// entry is dismissed; then the pill reads the item again.
#[gpui_kit::test]
fn a_finished_share_says_its_outcome_in_the_pill(cx: &mut TestAppContext) {
    let (shell, root) = shell(true, cx);
    open_ticket(&shell, &root, cx);
    let viewing = shell.label("item-pill", cx);
    let share = running(&shell, 1, item_sync(&root), cx);
    let published = Envelope::classified_mutation(
        "item sync",
        Scope::default(),
        false,
        ResultCode::Ok,
        Effects {
            publication: PublicationEffect::Published,
            ..Effects::not_requested()
        },
        false,
    );
    finish_with(&shell, share.id, published, cx);
    assert_eq!(shell.label("item-pill", cx).as_deref(), Some(SHARED));
    assert_eq!(
        shell.label(request_id(share.id, "outcome"), cx).as_deref(),
        Some(SHARED)
    );

    shell.click(request_id(share.id, "dismiss"), cx);
    assert_eq!(shell.label("item-pill", cx), viewing);
}

/// O-1: Share… opens Settings › Remotes on the item's own repository,
/// though the page last worked on another; Share main… on the
/// repository detail does too. The selection is the repository's.
#[gpui_kit::test]
fn share_opens_remotes_on_its_own_repository_after_another(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    let b = fixture.create_repository("b");
    fixture.commit_ticket(&b, TICKET, "Shared", "");
    fixture.refresh(&a);
    fixture.refresh(&b);
    let shell = Shell::open(fixture, cx);

    shell.select(&a, cx);
    shell.click(SHARE_MAIN_BUTTON, cx);
    assert!(shell.present("settings-remotes", cx));
    assert_eq!(
        shell.label("remotes-root", cx),
        Some(format!("Repository: {}", key(&a)))
    );
    shell.click("settings-close", cx);

    open_ticket(&shell, &b, cx);
    assert_eq!(shell.label(SHARE_BUTTON, cx).as_deref(), Some(SHARE_SET_UP));
    shell.click(SHARE_BUTTON, cx);
    assert!(shell.present("settings-remotes", cx));
    assert_eq!(
        shell.label("remotes-root", cx),
        Some(format!("Repository: {}", key(&b))),
        "the Remotes page works on the item's repository"
    );
}

fn published() -> Envelope<MutationDataDto> {
    Envelope::classified_mutation(
        "item sync",
        Scope::default(),
        false,
        ResultCode::Ok,
        Effects {
            publication: PublicationEffect::Published,
            ..Effects::not_requested()
        },
        false,
    )
}

/// A kept draft of the item outranks a Shared outcome in the pill: the
/// pill says the draft is kept, and the entry still says Shared.
#[gpui_kit::test]
fn a_kept_draft_after_a_shared_outcome_reads_draft_kept(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Shared", "");
    fixture.publish_over_ssh(&root);
    fixture.refresh(&root);
    store_draft(&fixture, &root, ItemDtoKind::Ticket, TICKET, "Draft body");
    let shell = Shell::open(fixture, cx);
    open_ticket(&shell, &root, cx);
    let kept = pill_words(PillState::ViewingDraftKept);
    assert_eq!(shell.label("item-pill", cx).as_deref(), Some(kept));

    let share = running(&shell, 1, item_sync(&root), cx);
    finish_with(&shell, share.id, published(), cx);
    assert_eq!(
        shell.label(request_id(share.id, "outcome"), cx).as_deref(),
        Some(SHARED)
    );
    assert_eq!(shell.label("item-pill", cx).as_deref(), Some(kept));
}

/// The pill says a Shared outcome only of the read it was first seen
/// with: once the item reads otherwise, the pill reads the item again.
#[gpui_kit::test]
async fn a_shared_outcome_leaves_the_pill_once_the_item_reads_otherwise(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(true, cx);
    open_ticket(&shell, &root, cx);
    let share = running(&shell, 1, item_sync(&root), cx);
    finish_with(&shell, share.id, published(), cx);
    // The share's own reload of the tab leaves the file as it was.
    shell.run(cx);
    assert_eq!(shell.label("item-pill", cx).as_deref(), Some(SHARED));

    shell
        .fixture
        .commit_ticket(&root, TICKET, "Changed since", "");
    let refresh = shell.workspace.update(cx, |workspace, _| {
        workspace.services().worker().submit(
            Intent::IndexRefresh { root: root.clone() },
            Lane::User,
            None,
            None,
        )
    });
    shell.until_ended(refresh, cx).await;
    shell
        .until_drawn(cx, "the tab read the changed item", |shell, cx| {
            shell.label("item-title", cx).as_deref() == Some("Changed since")
        })
        .await;
    assert_eq!(
        shell.label("item-pill", cx).as_deref(),
        Some(pill_words(PillState::ViewingShared))
    );
    assert_eq!(
        shell.label(request_id(share.id, "outcome"), cx).as_deref(),
        Some(SHARED),
        "the entry still says it"
    );
}

/// Orchestrator ruling (Task 10b): sharing private work says Shared in
/// the pill. The outcome speaks of the item as it is first read after the
/// share finished, not as it read before: here the item changed before
/// the share finished and after the tab last read it, as a share that
/// brings in others' changes leaves it. The
/// pill says Shared over the private copy until the item reads otherwise,
/// then reads the item again. The share runs for real; its answer is
/// constructed, as the test binary publishes nothing.
#[gpui_kit::test]
async fn sharing_private_work_shows_shared_until_the_item_reads_differently(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Shared", "");
    fixture.publish_over_ssh(&root);
    fixture.refresh(&root);
    fixture.save_ticket(&root, TICKET, "Private");
    let shell = Shell::open(fixture, cx);
    open_ticket(&shell, &root, cx);
    let private = pill_words(PillState::ViewingPrivate);
    assert_eq!(shell.label("item-pill", cx).as_deref(), Some(private));

    // Changed where the tab has not read it, as a share's merge leaves it.
    shell.fixture.save_ticket(&root, TICKET, "Shared since");
    let held = shell
        .backend
        .hold_job_return_answering(JobKind::SynchronizeItem, published());
    shell.click(SHARE_BUTTON, cx);
    held.wait_reached();
    held.release();
    let share = shell.requests(cx, |request| item_sync_of(request, &root))[0];
    shell.until_ended(share, cx).await;
    shell
        .until_drawn(cx, "the tab read the item after the share", |shell, cx| {
            shell.label("item-title", cx).as_deref() == Some("Shared since")
        })
        .await;
    shell
        .until_drawn(cx, "the pill said the outcome", |shell, cx| {
            shell.label("item-pill", cx).as_deref() == Some(SHARED)
        })
        .await;
    shell.run(cx);
    assert_eq!(shell.label("item-pill", cx).as_deref(), Some(SHARED));

    // The share left the item's synchronization pending, and a save is
    // refused (`recovery_required`): the file is changed in its working
    // copy, which the item reads.
    let service = shell.fixture.service();
    let repo = service.resolve_repository(&root).unwrap();
    let item = service.show_item(&repo, &item_id(TICKET)).unwrap();
    std::fs::write(
        Path::new(&item.context.worktree).join(&item.path),
        ticket_source(TICKET, "Changed since", ""),
    )
    .unwrap();
    let refresh = shell.workspace.update(cx, |workspace, _| {
        workspace.services().worker().submit(
            Intent::IndexRefresh { root: root.clone() },
            Lane::User,
            None,
            None,
        )
    });
    shell.until_ended(refresh, cx).await;
    shell
        .until_drawn(cx, "the tab read the changed item", |shell, cx| {
            shell.label("item-title", cx).as_deref() == Some("Changed since")
        })
        .await;
    assert_eq!(shell.label("item-pill", cx).as_deref(), Some(private));
    assert_eq!(
        shell.label(request_id(share, "outcome"), cx).as_deref(),
        Some(SHARED),
        "the entry still says it"
    );
}
