//! Retirement end to end over real local repositories and the real service
//! (D6 Task 12b, design Part E, A21, A22, U-11). Nothing here is canned: the
//! library answers every call. A real retirement of a context that is still
//! there observes the remote over SSH first, which no desktop test reaches
//! (ledger, Task 11a), so these tests take an item whose context is already
//! gone, the library's `already_absent` path, which observes nothing.

use std::path::{Path, PathBuf};

use gpui_kit::TestAppContext;
use manyhands::{
    canonical::ItemId,
    drafts::{CommentTarget, DraftFields, DraftId, DraftTarget, ItemKind},
    repository::{ItemDtoKind, OperationId, RequestId},
};

use crate::desktop::{
    model::{
        navigation::OpenTarget,
        polling::{
            Answer, AttemptTarget, Clock, Disposition, Policy, PollAttempt, TargetKind,
            entry::RetireState,
        },
    },
    services::requests::{DesktopRequestId, Intent, Lane, RequestState},
    test_support::{
        drafts::store_draft,
        fixtures::{Fixture, item_id},
    },
    view::{
        draft_comparison::TICKET_CLOSED,
        drafts::{ACTIONS, row_id},
        harness::{Shell, key},
        settings::polling_support::{Rig, policy},
    },
};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC0";
const COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC1";
const OWN: &str = "Own copy of the item";
const PRIMARY: &str = "Closed on primary";

fn item() -> ItemId {
    item_id(TICKET)
}

/// Repository `a` whose primary branch holds the ticket closed, and whose
/// item still has its own context (a branch and a worktree) holding an
/// older copy: the state a poll finds after the ticket was closed and merged
/// elsewhere.
fn closed_on_primary() -> (Fixture, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    // Published over SSH, as a repository that polls is; paused by the rig.
    fixture.publish_over_ssh(&root);
    fixture.commit_ticket(&root, TICKET, "Open title", "");
    fixture.refresh(&root);
    // Saving a ticket that has only its primary copy provisions its own
    // context: a branch and a worktree.
    fixture.save_ticket(&root, TICKET, OWN);
    fixture.commit_closed_ticket(&root, TICKET, PRIMARY);
    fixture.refresh(&root);
    (fixture, root)
}

fn branch() -> String {
    format!("manyhands/ticket/{TICKET}")
}

fn worktree(root: &Path) -> PathBuf {
    root.join(".manyhands/worktrees").join(TICKET)
}

/// The item's branch and worktree, removed as a retirement removes them
/// (the library's own removal needs an SSH observation).
fn remove_context(root: &Path) {
    let git = git2::Repository::open(root).unwrap();
    for name in git.worktrees().unwrap().iter().flatten() {
        let found = git.find_worktree(name).unwrap();
        let mut options = git2::WorktreePruneOptions::new();
        options.valid(true).working_tree(true);
        found.prune(Some(&mut options)).unwrap();
    }
    git.find_branch(&branch(), git2::BranchType::Local)
        .unwrap()
        .delete()
        .unwrap();
}

/// A context for the item again, at primary's tip.
fn restore_context(root: &Path) {
    let git = git2::Repository::open(root).unwrap();
    let tip = git.head().unwrap().peel_to_commit().unwrap();
    let branch = git.branch(&branch(), &tip, false).unwrap();
    let reference = branch.into_reference();
    git.worktree(
        TICKET,
        &worktree(root),
        Some(git2::WorktreeAddOptions::new().reference(Some(&reference))),
    )
    .unwrap();
}

/// Whether the item has its own branch, and its own worktree.
fn has_context(root: &Path) -> (bool, bool) {
    let git = git2::Repository::open(root).unwrap();
    (
        git.find_branch(&branch(), git2::BranchType::Local).is_ok(),
        worktree(root).exists(),
    )
}

/// The item's open tab, which shows the copy the library reads.
fn open_tab(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    let target = OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Ticket,
        id: item(),
    };
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
}

async fn title_becomes(shell: &Shell, title: &'static str, cx: &mut TestAppContext) {
    shell
        .until_drawn(cx, title, move |shell, cx| {
            shell.label("item-title", cx).as_deref() == Some(title)
        })
        .await;
}

/// The retirement of the ticket, sent as Retire… and Resume send it: under
/// `request`, on the user lane, through the scheduler.
fn retire_under(rig: &Rig, request: RequestId, cx: &mut TestAppContext) -> DesktopRequestId {
    let root = rig.key(0);
    rig.services.scheduler().update(cx, |scheduler, _| {
        scheduler.resend_retirement(request, &root, TargetKind::Ticket, &item(), Lane::User)
    })
}

/// A completed attempt that reported the ticket `Closed`, so the entry has
/// the item to say things about.
fn reported_closed(rig: &Rig, cx: &mut TestAppContext) {
    // Paused, so that no poll replaces the attempt reported here.
    rig.policy(
        0,
        Policy {
            paused: true,
            ..policy()
        },
        cx,
    );
    let now = rig.clock.now();
    let attempt = PollAttempt {
        root: rig.key(0),
        operation: OperationId::new(),
        answer: Answer::Completed,
        completed_at: Some(now.utc),
        next_eligible_at: Some(now.utc + time::Duration::seconds(300)),
        targets: vec![AttemptTarget {
            item_id: Some(item()),
            kind: Some(TargetKind::Ticket),
            disposition: Disposition::Closed,
        }],
        index_pending: false,
    };
    rig.services.scheduler().update(cx, |scheduler, cx| {
        scheduler.schedule_mut().after(&now, &attempt);
        cx.notify();
    });
    rig.shell.run(cx);
}

/// A rig over `fixture`'s repository, paused so that no poll runs.
fn rig_over(fixture: Fixture, root: &Path, cx: &mut TestAppContext) -> Rig {
    cx.executor().allow_parking();
    fixture.pause_polling(root);
    let rig = Rig::over(fixture, vec![root.to_owned()], cx);
    // Polling on, as in the application: the scheduler hears the answers.
    let _ = rig.services.clone().with_polling();
    rig
}

fn store_comment_draft(fixture: &Fixture, root: &Path, body: &str) -> DraftId {
    let target = DraftTarget {
        repository: key(root),
        item_kind: ItemKind::Ticket,
        item_id: item(),
        comment: Some(CommentTarget {
            comment_id: item_id(COMMENT),
            parent_id: None,
        }),
    };
    match fixture.draft_store().create(
        target,
        None,
        DraftFields::Comment {
            body: body.to_owned(),
        },
    ) {
        Ok(claim) => claim.draft().id,
        Err(error) => panic!("the draft must be stored: {error}"),
    }
}

/// The actions row `id` of the Drafts page offers.
fn actions(shell: &Shell, id: DraftId, cx: &mut TestAppContext) -> Vec<&'static str> {
    ACTIONS
        .iter()
        .map(|(part, _)| *part)
        .filter(|part| shell.present(row_id(id, part), cx))
        .collect()
}

/// A21, U-11, E5: with primary holding the closed ticket and the item's
/// branch and worktree gone, the item's open tab reads primary's copy once
/// the retirement has answered, and a kept item draft and a kept comment
/// draft are listed on the Drafts page and export, not restore.
#[gpui_kit::test]
async fn retire_end_state_real(cx: &mut TestAppContext) {
    let (fixture, root) = closed_on_primary();
    let kept = store_draft(
        &fixture,
        &root,
        ItemDtoKind::Ticket,
        TICKET,
        "Kept item words",
    );
    let comment = store_comment_draft(&fixture, &root, "Kept comment words");
    let rig = rig_over(fixture, &root, cx);
    let shell = &rig.shell;

    // Before: the tab reads the item's own copy.
    open_tab(shell, &root, cx);
    title_becomes(shell, OWN, cx).await;
    assert_eq!(has_context(&root), (true, true));

    remove_context(&root);
    assert_eq!(has_context(&root), (false, false));
    let sent = retire_under(&rig, RequestId::new(), cx);
    shell.until_ended(sent, cx).await;
    // The library answered `already_absent`, a retirement (E3), and the
    // lists and tabs read again: the tab reads primary's copy (U-11).
    title_becomes(shell, PRIMARY, cx).await;

    // The drafts outlive it, listed, with Export… and without Restore.
    shell.keys("secondary-shift-d", cx);
    shell
        .until_drawn(cx, "the drafts page", |shell, cx| {
            shell.present(row_id(comment, "export"), cx)
                && shell.present(row_id(kept, "export"), cx)
        })
        .await;
    assert_eq!(
        shell.label(row_id(kept, "state"), cx).as_deref(),
        Some(TICKET_CLOSED)
    );
    for id in [kept, comment] {
        let offered = actions(shell, id, cx);
        assert!(offered.contains(&"export"), "{offered:?}");
        assert!(!offered.contains(&"restore"), "{offered:?}");
    }
    // Export… writes each one's text.
    for (id, words) in [(kept, "Kept item words"), (comment, "Kept comment words")] {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("exported.md");
        shell.click(row_id(id, "export"), cx);
        assert!(cx.did_prompt_for_new_path());
        let chosen = path.clone();
        cx.simulate_new_path_selection(move |_| Some(chosen));
        shell
            .until_drawn(cx, "the export", |_, _| path.exists())
            .await;
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains(words), "{words}: {text}");
    }
}

/// The states of the attempt's entry's lines about the ticket, the entries
/// of the in-flight record, and the retirements Activity still lists.
fn says(rig: &Rig, cx: &mut TestAppContext) -> (Vec<RetireState>, usize, usize) {
    let (lines, entries) = rig.services.scheduler().read_with(cx, |scheduler, _| {
        let lines = scheduler
            .schedule()
            .attempts()
            .flat_map(|(_, _, lines)| lines.to_vec())
            .filter(|line| line.item == item())
            .map(|line| line.state)
            .collect::<Vec<_>>();
        (lines, scheduler.store().retirements().len())
    });
    let listed = rig.services.operations().read_with(cx, |operations, _| {
        operations
            .requests()
            .iter()
            .filter(|request| matches!(request.intent, Intent::ContextRetire { .. }))
            .filter(|request| !matches!(request.state, RequestState::Dismissed))
            .count()
    });
    (lines, entries, listed)
}

/// Lets the scheduler hear the answer that ended `id`, then checks what it
/// made of it: one line, Retired, no entry in the in-flight record and no
/// row in Activity.
async fn settled(rig: &Rig, id: DesktopRequestId, cx: &mut TestAppContext) {
    rig.shell.until_ended(id, cx).await;
    for _ in 0..20 {
        rig.shell.run(cx);
        cx.executor()
            .timer(std::time::Duration::from_millis(10))
            .await;
    }
    assert_eq!(says(rig, cx), (vec![RetireState::Retired], 0, 0));
}

/// A22, O-14, Fact 24: a Final answer replayed under the same request ID is
/// the library's stored answer, not a new run: with a context back in place
/// for the item, nothing is removed, and the desktop adds no second entry to
/// the in-flight record, no second line to the attempt's entry and no
/// second row to Activity.
#[gpui_kit::test]
async fn retire_replay_final(cx: &mut TestAppContext) {
    let (fixture, root) = closed_on_primary();
    remove_context(&root);
    let rig = rig_over(fixture, &root, cx);
    reported_closed(&rig, cx);

    let request = RequestId::new();
    let first = retire_under(&rig, request, cx);
    // Retired: one line, no entry, and no row (an answered retirement is
    // dismissed, E3).
    settled(&rig, first, cx).await;

    // The item has a context again; a replay of the same request must
    // leave it alone.
    restore_context(&root);
    assert_eq!(has_context(&root), (true, true));
    let again = retire_under(&rig, request, cx);
    settled(&rig, again, cx).await;
    assert_eq!(
        has_context(&root),
        (true, true),
        "nothing removed a second time"
    );
}
