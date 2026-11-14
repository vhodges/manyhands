//! `focus_on_open_survives_the_sheet_keeping_focus` (design A6 F1 and F2;
//! Ruling 3): Settings opened on a route naming a field the page draws
//! keeps focus on that field, and on a route naming none on the page's
//! entry, read after two draws, so a rule that keeps focus in the sheet
//! (Task 2's Step 5; A7 item 1) cannot undo a route's focus. The routes are
//! those whose target Task 6 does not change; Task 6 folds this test into
//! P4.

use gpui_kit::{ElementId, TestAppContext};
use manyhands::{repository::transport::SshAuthority, results::ResultCode};

use super::{
    returns::{escape, settings_open},
    support::{holds, host, route},
    table::Table,
};
use crate::desktop::{
    model::stamp::SettingsPage,
    services::requests::Intent,
    test_support::fixtures::Fixture,
    view::{
        activity::tests::cases::{answered, fabricate},
        harness::Shell,
        settings::SettingsRoute,
    },
};

pub(super) async fn rows(table: &mut Table, cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.publish_over_ssh(&root);
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    shell.select(&root, cx);
    // A protected generation that a retry asks the passphrase of again.
    let retry = fabricate(
        &shell,
        1,
        Intent::KeyGenerate {
            label: "locked".to_owned(),
            protected: true,
        },
        answered("key generate", ResultCode::Busy),
        cx,
    );
    let authority = SshAuthority {
        host: "example.invalid".to_owned(),
        port: 22,
    };
    let rows: [(&str, SettingsRoute, ElementId); 6] = [
        (
            "F1 Identity(root)",
            SettingsRoute::Identity(root.clone()),
            "identity-name".into(),
        ),
        (
            "F1 CheckHost(root, authority)",
            SettingsRoute::CheckHost(root.clone(), authority),
            "hosts-check".into(),
        ),
        (
            "F1 RetryGeneration(id)",
            SettingsRoute::RetryGeneration(retry),
            "key-generate-passphrase".into(),
        ),
        (
            "F2 Page(Repositories)",
            SettingsRoute::Page(SettingsPage::Repositories),
            ("settings-page", 0usize).into(),
        ),
        (
            "F2 Page(Polling)",
            SettingsRoute::Page(SettingsPage::Polling),
            ("settings-page", 2usize).into(),
        ),
        (
            "F2 Manage(root)",
            SettingsRoute::Manage(root.clone()),
            ("settings-page", 0usize).into(),
        ),
    ];
    for (row, settings_route, target) in rows {
        route(&shell, cx, move |workspace, window, cx| {
            workspace.show_settings(settings_route, None, window, cx)
        });
        assert!(settings_open(&shell, cx), "{row}: Settings opened");
        let held = holds(&shell, target.clone(), cx);
        table.row(&shell, row, held, &format!("{target:?}"), cx);
        escape(&shell, row, cx);
    }

    // The worker must not outlive the test: the check Check host started
    // ends first.
    let host = host(&shell, cx);
    let page = shell
        .visual(cx)
        .update(|window, cx| host.update(cx, |host, cx| host.hosts(window, cx)));
    shell
        .until_drawn(cx, "Check host's answer", |_, cx| {
            page.read_with(cx, |page, _| page.observed_code().is_some())
        })
        .await;
}
