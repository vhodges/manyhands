//! The polling, repository and drafts rows of `each_action_runs_its_handler` (P18).

use gpui_kit::TestAppContext;
use manyhands::{drafts::DraftFields, repository::ItemDtoKind};

use super::cases::{TICKET, label_of, settings};
use crate::desktop::{
    model::{
        activity::{
            copy::{DISMISS, OPEN, REVIEW, SHARE_MAIN},
            rows::{ActivityAction, RowKey, Target},
            states::hold_key,
        },
        polling::{
            Answer, AttemptTarget, Clock, Disposition, PollAttempt, Reason, TargetKind,
            TransportFailure,
            attention::OPEN_REMOTE_SETTINGS,
            entry::{RETIRE, SHARE},
        },
        repository_state::RowAction,
        stamp::SettingsPage,
        tabs::TabKey,
    },
    services::{ReadKind, requests::Intent},
    test_support::{
        drafts::store_draft,
        fixtures::{Fixture, item_id},
    },
    view::{
        activity_support::{act, refresh, row},
        harness::{Shell, key},
        settings::{Shown, polling_support::Rig},
    },
};

fn target(item: Option<&manyhands::canonical::ItemId>, disposition: Disposition) -> AttemptTarget {
    AttemptTarget {
        item_id: item.cloned(),
        kind: item.map(|_| TargetKind::Ticket),
        disposition,
        main: item.is_none(),
    }
}

/// Share…, Share main, Open, Retire…, a hold's action and a brought-in
/// row's Dismiss, from the polling rows.
pub(super) async fn states(cx: &mut TestAppContext) {
    use manyhands::canonical::ItemId;
    let rig = Rig::new(&[("a", true)], cx);
    let root = rig.key(0);
    rig.schedule(0, cx);
    let (unshared, problem, brought, retired) = (
        ItemId::generate(),
        ItemId::generate(),
        ItemId::generate(),
        ItemId::generate(),
    );
    let now = rig.clock.now();
    let attempt = PollAttempt {
        root: root.clone(),
        operation: manyhands::repository::OperationId::new(),
        answer: Answer::Completed,
        completed_at: Some(now.utc),
        next_eligible_at: Some(now.utc + time::Duration::seconds(300)),
        targets: vec![
            target(Some(&unshared), Disposition::Unpublished),
            target(None, Disposition::Unpublished),
            target(Some(&problem), Disposition::Dirty),
            target(Some(&brought), Disposition::FastForwarded),
            target(Some(&retired), Disposition::Closed),
        ],
        index_pending: false,
    };
    rig.services.scheduler().update(cx, |scheduler, cx| {
        scheduler.schedule_mut().after(&now, &attempt);
        cx.notify();
    });
    rig.shell.run(cx);
    let shell = &rig.shell;

    // Share…, and Share main: each sends its own share.
    let share_of = |item: &ItemId| Target::Item(item.clone());
    act(
        shell,
        &RowKey::Unshared {
            root: root.clone(),
            target: share_of(&unshared),
        },
        SHARE,
        cx,
    );
    let items = rig.sent(
        cx,
        |intent| matches!(intent, Intent::ItemSync { id, .. } if id == &unshared),
    );
    assert_eq!(items.len(), 1, "Share… sent the item's share");
    act(
        shell,
        &RowKey::Unshared {
            root: root.clone(),
            target: Target::Main,
        },
        SHARE_MAIN,
        cx,
    );
    assert_eq!(
        rig.sent(cx, |intent| matches!(intent, Intent::RepoSync { .. }))
            .len(),
        1,
        "Share main sent the repository's share"
    );

    // Open takes the person to the item.
    act(
        shell,
        &RowKey::Problem {
            root: root.clone(),
            target: share_of(&problem),
        },
        OPEN,
        cx,
    );
    let tab = TabKey::Item {
        repo: root.clone(),
        kind: ItemDtoKind::Ticket,
        id: problem.to_string(),
    };
    let tabs = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.tab_keys());
    assert!(tabs.contains(&tab), "its tab opened");

    // Retire… goes through its asks and sends the retirement.
    act(
        shell,
        &RowKey::RetireOffer {
            root: root.clone(),
            item: retired.clone(),
        },
        RETIRE,
        cx,
    );
    shell
        .until_drawn(cx, "the retirement", |_, cx| {
            !rig.sent(cx, |intent| matches!(intent, Intent::ContextRetire { .. }))
                .is_empty()
        })
        .await;

    // Dismiss takes a brought-in row for the session.
    let brought_row = RowKey::Brought {
        root: root.clone(),
        seq: 0,
    };
    assert!(row(shell, &brought_row, cx).is_some());
    act(shell, &brought_row, DISMISS, cx);
    assert!(row(shell, &brought_row, cx).is_none(), "its row went");

    // A hold's action opens the place that fixes it.
    let reason = Reason::ConfigurationRequired(root.clone());
    rig.services.scheduler().update(cx, |scheduler, cx| {
        assert!(scheduler.schedule_mut().raise(&root, reason.clone()));
        cx.notify();
    });
    shell.run(cx);
    act(
        shell,
        &RowKey::Hold(hold_key(&reason)),
        OPEN_REMOTE_SETTINGS,
        cx,
    );
    assert_eq!(settings(shell, cx), Shown::Settings(SettingsPage::Remotes));
}

/// A repository's own action, from its state's row: a repository that
/// cannot be reached offers Retry now, which sends a check.
pub(super) async fn repository(cx: &mut TestAppContext) {
    let rig = Rig::new(&[("a", true)], cx);
    rig.schedule(0, cx);
    rig.answer(0, Answer::Transport(TransportFailure::Unreachable), cx);
    let shell = &rig.shell;
    refresh(shell, cx);
    let at = RowKey::Repository(rig.key(0));
    let label = label_of(
        shell,
        &at,
        |action| matches!(action, ActivityAction::Repository(_, RowAction::RetryNow)),
        cx,
    );
    act(shell, &at, &label, cx);
    assert_eq!(
        rig.sent(cx, |intent| matches!(intent, Intent::PollOnce { .. }))
            .len(),
        1,
        "Retry now sent a check"
    );
}

// --- Drafts ---

/// Review and Dismiss of the recovered drafts, a note's Dismiss, and a
/// failing session's Retry.
pub(super) async fn drafts(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "One", "");
    fixture.refresh(&root);
    store_draft(&fixture, &root, ItemDtoKind::Ticket, TICKET, "Mine");
    let shell = Shell::open(fixture, cx);

    let recovered = RowKey::Recovered;
    assert!(!shell.workspace.read_with(cx, |w, cx| w.drafts_shown(cx)));
    act(&shell, &recovered, REVIEW, cx);
    assert!(shell.workspace.read_with(cx, |w, cx| w.drafts_shown(cx)));
    act(&shell, &recovered, DISMISS, cx);
    assert!(
        row(&shell, &recovered, cx).is_none(),
        "dismissed for the session"
    );

    // A record the store could not read is set aside and told of once.
    let directory = shell.fixture.data_dir().join("inflight");
    std::fs::write(directory.join("record.json"), b"not a record").unwrap();
    let services = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().clone())
        .with_polling();
    cx.update(|cx| services.window_activated(cx));
    shell.run(cx);
    let note = RowKey::Note(crate::desktop::services::inflight::Report::Unreadable);
    assert!(row(&shell, &note, cx).is_some());
    act(&shell, &note, DISMISS, cx);
    assert!(row(&shell, &note, cx).is_none());

    // A failing session offers Retry, which writes its newest text now.
    let target = crate::desktop::model::navigation::OpenTarget::Item {
        repo: key(&root),
        kind: ItemDtoKind::Ticket,
        id: item_id(TICKET),
    };
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
    shell.click("draft-restore-button", cx);
    let drafts = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().drafts().clone());
    let tab = TabKey::Item {
        repo: key(&root),
        kind: ItemDtoKind::Ticket,
        id: TICKET.to_owned(),
    };
    let id = drafts
        .read_with(cx, |registry, _| registry.attached(&tab))
        .expect("the draft is restored");
    let mut fields = drafts.read_with(cx, |registry, _| {
        registry.session(id).expect("its session").current().clone()
    });
    if let DraftFields::Ticket { body, .. } = &mut fields {
        body.push_str(" more");
    }
    drafts.update(cx, |registry, cx| registry.edit(id, fields, cx));
    let failing = shell.fixture.draft_store().with_failure_point_for_testing(
        manyhands::drafts::FailurePoint::CreateTemp,
        manyhands::drafts::FailureAction::Fail(manyhands::drafts::DraftErrorKind::NoSpace),
    );
    let flushed = drafts.update(cx, |registry, cx| registry.flush_now(id, cx));
    shell.run(cx);
    flushed.await.expect_err("the flush fails");
    shell.run(cx);
    let flush = RowKey::Flush(id);
    assert!(row(&shell, &flush, cx).is_some(), "the failure is listed");
    drop(failing);
    let writes = shell.backend.reads_seen(ReadKind::DraftWrite);
    act(&shell, &flush, "Retry", cx);
    assert!(
        shell.backend.reads_seen(ReadKind::DraftWrite) > writes,
        "Retry wrote the newest text now"
    );
}
