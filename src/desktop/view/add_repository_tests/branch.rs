//! P10, `the_main_branch_is_the_checked_out_one` (Wave 04 Cycle 05a design
//! B7; R-12; G16): the Main branch select lists the checked-out branch
//! first, chosen and tagged Suggested, also when it has no commits yet, and
//! then the folder's other branches, refused. A Kit select's disabled item
//! is only drawn disabled, so the delegate refuses it on `on_will_change`,
//! and a `Confirm` naming it is ignored: the select goes back to the branch
//! that can be chosen. With none checked out, or a configuration on
//! another main branch, nothing is chosen. G16: an open list holds focus
//! until it closes, then its trigger does, and the sheet stays open.

use std::path::{Path, PathBuf};

use gpui_kit::{
    TestAppContext,
    component::{
        IndexPath,
        searchable_list::SearchableListChange,
        select::{SelectDelegate, SelectEvent},
    },
};

use super::{
    enter_in, settings,
    support::{Table, canned, holds_handle, land, set_up, sheet_open},
};
use crate::desktop::{
    services::ReadResult,
    test_support::fixtures::{Fixture, identity},
    view::{
        add_repository::branch::{Branch, BranchKind, Branches},
        harness::{Shell, key},
    },
};

fn suggested(name: &str) -> Branch {
    Branch {
        name: name.to_owned(),
        kind: BranchKind::Suggested,
    }
}

fn other(name: &str) -> Branch {
    Branch {
        name: name.to_owned(),
        kind: BranchKind::Other,
    }
}

fn held(name: &str) -> Branch {
    Branch {
        name: name.to_owned(),
        kind: BranchKind::Held,
    }
}

/// The branch the select has chosen, and the items the sheet gave it.
fn branches(shell: &Shell, cx: &mut TestAppContext) -> (Option<String>, Vec<Branch>) {
    shell.frame(cx, |_, _| ());
    super::sheet(shell, cx).read_with(cx, |sheet, cx| {
        (sheet.chosen_branch(cx), sheet.branch_items().to_vec())
    })
}

/// The row `row`: the select chose `chosen` and lists `items`.
fn expect(
    table: &mut Table,
    shell: &Shell,
    row: &str,
    chosen: Option<&str>,
    items: &[Branch],
    cx: &mut TestAppContext,
) {
    let (found, listed) = branches(shell, cx);
    table.fact(
        row,
        found.as_deref() == chosen && listed == items,
        &format!("{chosen:?} chosen of {items:?} (it is {found:?} of {listed:?})"),
    );
}

/// A branch `name` at the commit `root` has checked out, not checked out.
fn branch_off(root: &Path, name: &str) {
    let repository = git2::Repository::open(root).unwrap();
    let head = repository.head().unwrap().peel_to_commit().unwrap();
    repository.branch(name, &head, false).unwrap();
}

/// A Git repository `name` with `main` checked out and no commit yet
/// (fact 11a), Git's identity in its own configuration.
fn unborn(fixture: &Fixture, name: &str) -> PathBuf {
    let root = fixture.parent().join(name);
    let mut options = git2::RepositoryInitOptions::new();
    options.initial_head("main");
    let repository = git2::Repository::init_opts(&root, &options).unwrap();
    let mut config = git2::Config::open(&repository.path().join("config")).unwrap();
    config.set_str("user.name", &identity().name).unwrap();
    config.set_str("user.email", &identity().email).unwrap();
    root
}

fn check(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    super::set(shell, "add-path", &key(root), cx);
    enter_in(shell, "add-path", cx);
    shell.keys_landed(cx);
}

pub(super) fn rows(table: &mut Table, cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let plain = fixture.plain_repository("plain", Some(identity()));
    branch_off(&plain, "feature");
    let fresh = unborn(&fixture, "fresh");
    let shell = settings(fixture, cx);

    expect(table, &shell, "Before a check lands", None, &[], cx);

    check(&shell, &plain, cx);
    let listed = [suggested("main"), other("feature")];
    let row = "The checked-out branch chosen and tagged, the other refused";
    expect(table, &shell, row, Some("main"), &listed, cx);

    refused_by_the_delegate(table);
    by_keyboard(table, &shell, cx);

    // A Confirm naming a refused branch is ignored: the select goes back.
    let select = super::sheet(&shell, cx).read_with(cx, |sheet, _| sheet.branch_select());
    shell.visual(cx).update(|window, cx| {
        select.update(cx, |select, cx| {
            select.set_selected_value(&"feature".to_owned(), window, cx);
            cx.emit(SelectEvent::Confirm(Some("feature".to_owned())));
        })
    });
    shell.run(cx);
    let row = "A Confirm naming the refused branch";
    expect(table, &shell, row, Some("main"), &listed, cx);

    check(&shell, &fresh, cx);
    let row = "The checked-out branch with no commits yet (fact 11a)";
    expect(table, &shell, row, Some("main"), &[suggested("main")], cx);

    let mut detached = canned(&plain);
    detached.head_branch = None;
    detached.local_branches = vec!["feature".to_owned(), "main".to_owned()];
    land(&shell, &plain, ReadResult::Ok(detached), cx);
    let row = "No branch checked out";
    expect(
        table,
        &shell,
        row,
        None,
        &[other("feature"), other("main")],
        cx,
    );

    let mut same = set_up(&plain, "main");
    same.local_branches = vec!["main".to_owned(), "feature".to_owned()];
    land(&shell, &plain, ReadResult::Ok(same), cx);
    let row = "Configuration Valid on the checked-out branch";
    expect(table, &shell, row, Some("main"), &listed, cx);

    let mut trunk = set_up(&plain, "trunk");
    trunk.local_branches = vec!["main".to_owned(), "trunk".to_owned()];
    land(&shell, &plain, ReadResult::Ok(trunk), cx);
    let row = "Configuration Valid on another main branch";
    expect(
        table,
        &shell,
        row,
        None,
        &[held("main"), other("trunk")],
        cx,
    );
}

/// R-12: the delegate's `on_will_change` leaves the selection as it was
/// for a branch that cannot be chosen, and takes one that can.
fn refused_by_the_delegate(table: &mut Table) {
    let (first, second) = (IndexPath::default().row(0), IndexPath::default().row(1));
    let mut delegate = Branches::new(vec![suggested("main"), other("feature")]);
    let mut selection = vec![(first, suggested("main"))];
    delegate.on_will_change(
        &mut selection,
        &[
            SearchableListChange::Deselect { index: first },
            SearchableListChange::Select { index: second },
        ],
    );
    table.fact(
        "on_will_change refuses the other branch",
        selection == [(first, suggested("main"))],
        &format!("the selection kept (it is {selection:?})"),
    );
    let mut selection = Vec::new();
    delegate.on_will_change(
        &mut selection,
        &[SearchableListChange::Select { index: first }],
    );
    table.fact(
        "on_will_change takes the checked-out branch",
        selection == [(first, suggested("main"))],
        &format!("main selected (it is {selection:?})"),
    );
}

/// G16, by keyboard: Enter on the select opens its list, which holds focus;
/// Down and Enter on the refused branch close it on its trigger, the
/// checked-out branch still chosen; Escape on an open list closes it on
/// its trigger, and the sheet stays open.
fn by_keyboard(table: &mut Table, shell: &Shell, cx: &mut TestAppContext) {
    let select = super::sheet(shell, cx).read_with(cx, |sheet, _| sheet.branch_select());
    let trigger = select.read_with(cx, |select, cx| {
        gpui_kit::Focusable::focus_handle(select, cx)
    });
    shell.frame(cx, |window, cx| window.focus(&trigger, cx));
    shell.run(cx);
    for close in ["enter", "escape"] {
        shell.keys("enter", cx);
        let list = select.read_with(cx, |select, cx| {
            gpui_kit::Focusable::focus_handle(select, cx)
        });
        table.fact(
            &format!("G16 the list open, before {close}"),
            list != trigger && holds_handle(shell, &list, cx) && sheet_open(shell, cx),
            "focus in the open list, the sheet open",
        );
        if close == "enter" {
            shell.keys("down", cx);
        }
        shell.keys(close, cx);
        let (chosen, _) = branches(shell, cx);
        table.fact(
            &format!("G16 the list closed by {close}"),
            holds_handle(shell, &trigger, cx)
                && sheet_open(shell, cx)
                && chosen.as_deref() == Some("main"),
            &format!("focus on the select, the sheet open, main chosen (it is {chosen:?})"),
        );
    }
}
