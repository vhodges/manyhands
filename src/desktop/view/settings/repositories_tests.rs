//! The Repositories page's test helpers, which the managed section's tests
//! import (`repositories_manage_tests.rs`). The Add and Create tests moved
//! with their forms to `view/add_repository_tests.rs` (Wave 04 Cycle 05a
//! design B4, E-1).

use gpui_kit::{Entity, FocusHandle, TestAppContext, component::input::InputState};
use manyhands::results::{Outcome, ResultCode};

use super::RepositoriesPage;
use crate::desktop::{
    services::requests::{DesktopRequestId, RequestState},
    test_support::fixtures::Fixture,
    view::harness::Shell,
};

/// The window over `fixture`, with the Repositories page shown.
pub(super) fn settings(fixture: Fixture, cx: &mut TestAppContext) -> Shell {
    let shell = Shell::open(fixture, cx);
    shell.keys("secondary-,", cx);
    assert!(shell.present("settings-repositories", cx));
    shell
}

pub(super) fn page(shell: &Shell, cx: &mut TestAppContext) -> Entity<RepositoriesPage> {
    let host = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.settings().clone());
    host.read_with(cx, |host, _| host.repositories.clone())
        .expect("the Repositories page is made")
}

fn input(shell: &Shell, id: &str, cx: &mut TestAppContext) -> Entity<InputState> {
    let page = page(shell, cx);
    page.read_with(cx, |page, _| page.input(id))
}

pub(super) fn value(shell: &Shell, id: &str, cx: &mut TestAppContext) -> String {
    let input = input(shell, id, cx);
    input.read_with(cx, |input, _| input.value().to_string())
}

/// Sets the field `id` to `text`, as typing it would.
pub(super) fn set(shell: &Shell, id: &str, text: &str, cx: &mut TestAppContext) {
    let input = input(shell, id, cx);
    shell.visual(cx).update(|window, cx| {
        input.update(cx, |input, cx| input.set_value(text.to_owned(), window, cx));
    });
    shell.run(cx);
}

pub(super) fn registered(shell: &Shell) -> Vec<String> {
    shell
        .fixture
        .service()
        .list_repositories()
        .unwrap()
        .items
        .into_iter()
        .map(|summary| summary.root)
        .collect()
}

pub(super) fn finished(
    shell: &Shell,
    id: DesktopRequestId,
    cx: &mut TestAppContext,
) -> (Outcome, ResultCode) {
    let operations = shell.operations(cx);
    operations.read_with(cx, |operations, _| {
        match &operations.request(id).unwrap().state {
            RequestState::Finished(outcome, code, ..) => (*outcome, *code),
            _ => panic!("not finished"),
        }
    })
}

pub(super) fn control_focus(shell: &Shell, id: &str, cx: &mut TestAppContext) -> FocusHandle {
    let page = page(shell, cx);
    page.read_with(cx, |page, _| page.control_focus(id))
}
