//! P13, `share_changes_through_lists_the_folders_remotes` (Wave 04 Cycle
//! 05a design C1; decision 17; R-12's refusal, which C1 applies to a
//! remote sharing cannot use).
//!
//! The select lists the folder's remotes as the check found them, each
//! with its redacted fetch location, then "Keep on this computer". A
//! remote that is not an SSH address is greyed, and refused as B7's
//! branches are: the delegate's `on_will_change` leaves the selection as
//! it was, and a `Confirm` naming it is ignored, the select going back to
//! the choice before it. The check chooses the configuration's sharing
//! remote when it names one sharing can use, else Keep on this computer
//! (decision 17). A real check where it gives the case (a folder with an
//! SSH remote and an HTTPS one); canned checks where it cannot (a
//! configured remote; design Testing). Not drawn in the new mode, nor for a
//! registered folder.

use std::path::Path;

use gpui_kit::{
    TestAppContext,
    component::{
        IndexPath,
        searchable_list::SearchableListChange,
        select::{SelectDelegate, SelectEvent, SelectItem},
    },
};

use super::{
    enter_in, new_mode, settings, sheet,
    support::{Table, canned, land, remote, set_up},
};
use crate::desktop::{
    services::ReadResult,
    test_support::fixtures::{Fixture, identity},
    view::{
        add_repository::{
            SHARE_HELP, SHARE_THROUGH,
            share::{Share, Shares},
        },
        harness::{Shell, key},
    },
};

const SSH: &str = "ssh://git@example.invalid/team/repo.git";
const HTTPS: &str = "https://example.invalid/team/repo.git";

/// The item of the remote `name`, fetching from `location`.
fn to(name: &str, location: &str, eligible: bool) -> Share {
    Share {
        remote: Some(name.to_owned()),
        location: Some(location.to_owned()),
        eligible,
    }
}

/// The last item, Keep on this computer.
fn keep() -> Share {
    Share {
        remote: None,
        location: None,
        eligible: true,
    }
}

/// The share the select has chosen, and the items the sheet gave it.
fn shares(shell: &Shell, cx: &mut TestAppContext) -> (Option<Option<String>>, Vec<Share>) {
    shell.frame(cx, |_, _| ());
    sheet(shell, cx).read_with(cx, |sheet, cx| {
        (sheet.chosen_share(cx), sheet.share_items().to_vec())
    })
}

/// The row `row`: the select chose `chosen` (a remote's name, or `None`
/// for Keep on this computer) and lists `items`.
fn expect(
    table: &mut Table,
    shell: &Shell,
    row: &str,
    chosen: Option<&str>,
    items: &[Share],
    cx: &mut TestAppContext,
) {
    let (found, listed) = shares(shell, cx);
    let wanted = Some(chosen.map(str::to_owned));
    table.fact(
        row,
        found == wanted && listed == items,
        &format!("{wanted:?} chosen of {items:?} (it is {found:?} of {listed:?})"),
    );
}

/// The row `row`: the select is drawn, under its label, or it is not.
fn drawn(table: &mut Table, shell: &Shell, row: &str, wanted: bool, cx: &mut TestAppContext) {
    let label = shell.label("add-share", cx);
    let is = label.is_some();
    table.fact(
        &format!("{row} (drawn {wanted})"),
        is == wanted && (!wanted || label.as_deref() == Some(SHARE_THROUGH)),
        &format!("the select drawn {wanted} (its wrapper reads {label:?})"),
    );
}

/// `root` typed in the folder field, checked by Enter, and its check
/// landed.
fn check(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    super::set(shell, "add-path", &key(root), cx);
    enter_in(shell, "add-path", cx);
    shell.keys_landed(cx);
}

/// A `Confirm` of the select naming `named`, as the Kit's list emits one,
/// the select set to it first.
fn confirm(shell: &Shell, named: Option<Option<&str>>, cx: &mut TestAppContext) {
    let select = sheet(shell, cx).read_with(cx, |sheet, _| sheet.share_select());
    let named = named.map(|to| to.map(str::to_owned));
    shell.visual(cx).update(|window, cx| {
        select.update(cx, |select, cx| {
            if let Some(value) = &named {
                select.set_selected_value(value, window, cx);
            }
            cx.emit(SelectEvent::Confirm(named.clone()));
        })
    });
    shell.run(cx);
}

/// What the add would send once it succeeds, read from the sheet (C2).
fn to_send(shell: &Shell, cx: &mut TestAppContext) -> Option<Option<String>> {
    sheet(shell, cx).read_with(cx, |sheet, _| sheet.share_to_send())
}

/// P13 (C1; decision 17): the folder's remotes and Keep on this computer,
/// greyed and refused where sharing cannot use them, the configured remote
/// or Keep chosen when the check lands; not drawn in the new mode or for a
/// registered folder.
#[gpui_kit::test]
fn share_changes_through_lists_the_folders_remotes(cx: &mut TestAppContext) {
    let mut table = Table::new("P13");
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    let plain = fixture.plain_repository("plain", Some(identity()));
    fixture.add_remote(&plain, "origin", SSH);
    fixture.add_remote(&plain, "backup", HTTPS);
    // The oracle: the remotes as the library's check lists them.
    let found = fixture
        .service()
        .inspect_repository(&plain)
        .unwrap()
        .remotes;
    let shell = settings(fixture, cx);

    let row = "Before a check lands";
    drawn(&mut table, &shell, row, true, cx);
    expect(&mut table, &shell, row, None, &[keep()], cx);

    // A real check: the remotes in the library's order, each with its
    // redacted fetch location, then Keep; nothing configured, so Keep.
    check(&shell, &plain, cx);
    let mut listed: Vec<Share> = found
        .iter()
        .map(|found| {
            to(
                &found.name,
                &found.fetch_location,
                found.publication_eligible,
            )
        })
        .collect();
    listed.push(keep());
    let eligible: Vec<(&str, bool)> = found
        .iter()
        .map(|found| (found.name.as_str(), found.publication_eligible))
        .collect();
    table.fact(
        "The oracle: origin over SSH, backup over HTTPS",
        eligible.contains(&("origin", true)) && eligible.contains(&("backup", false)),
        &format!("origin eligible and backup not (they are {eligible:?})"),
    );
    let row = "A real check: the folder's remotes, then Keep, Keep chosen";
    drawn(&mut table, &shell, row, true, cx);
    expect(&mut table, &shell, row, None, &listed, cx);
    let help = shell.label("add-share-help", cx);
    table.fact(
        "The help under the select",
        help.as_deref() == Some(SHARE_HELP),
        &format!("the help {SHARE_HELP:?} (it is {help:?})"),
    );

    // Greyed where sharing cannot use the remote (C1).
    let canned_remotes = vec![remote("origin", SSH, true), remote("backup", HTTPS, false)];
    let items = [to("origin", SSH, true), to("backup", HTTPS, false), keep()];
    let greyed: Vec<bool> = items.iter().map(SelectItem::disabled).collect();
    table.fact(
        "Only the remote sharing cannot use is greyed",
        greyed == [false, true, false],
        &format!("greyed [false, true, false] (it is {greyed:?})"),
    );

    // Not set up: nothing is set up to share, so Keep (decision 17).
    let mut check_of = canned(&plain);
    check_of.remotes = canned_remotes.clone();
    land(&shell, &plain, ReadResult::Ok(check_of), cx);
    let row = "Not set up, an SSH and an HTTPS remote: Keep chosen";
    expect(&mut table, &shell, row, None, &items, cx);

    // Set up, sharing through each kind of remote (canned).
    for (configured, chosen, why) in [
        ("origin", Some("origin"), "an SSH remote: it is chosen"),
        ("backup", None, "a remote that is not SSH: Keep chosen"),
        ("gone", None, "a remote the folder lacks: Keep chosen"),
    ] {
        let mut check_of = set_up(&plain, "main");
        check_of.configuration.publication_remote = Some(configured.to_owned());
        check_of.remotes = canned_remotes.clone();
        land(&shell, &plain, ReadResult::Ok(check_of), cx);
        let row = format!("Set up, sharing through {why}");
        expect(&mut table, &shell, &row, chosen, &items, cx);
    }

    refused_by_the_delegate(&mut table, &items);

    // Chosen, then a `Confirm` naming the remote that cannot be used, or
    // nothing: each is ignored, the choice before it kept (as R-12).
    let mut check_of = canned(&plain);
    check_of.remotes = canned_remotes.clone();
    land(&shell, &plain, ReadResult::Ok(check_of), cx);
    confirm(&shell, Some(Some("origin")), cx);
    let row = "A Confirm naming origin chooses it";
    expect(&mut table, &shell, row, Some("origin"), &items, cx);
    for (named, what) in [(Some(Some("backup")), "backup"), (None, "nothing")] {
        confirm(&shell, named, cx);
        let row = format!("A Confirm naming {what} is ignored");
        expect(&mut table, &shell, &row, Some("origin"), &items, cx);
        let sends = to_send(&shell, cx);
        table.fact(
            &format!("{row} (the add sends origin)"),
            sends == Some(Some("origin".to_owned())),
            &format!("origin kept for the add (it is {sends:?})"),
        );
    }
    confirm(&shell, Some(None), cx);
    let row = "A Confirm naming Keep chooses it";
    expect(&mut table, &shell, row, None, &items, cx);

    // A check that lands again chooses its own default.
    confirm(&shell, Some(Some("origin")), cx);
    let mut check_of = canned(&plain);
    check_of.remotes = canned_remotes.clone();
    land(&shell, &plain, ReadResult::Ok(check_of), cx);
    let row = "A check landing again chooses its default";
    expect(&mut table, &shell, row, None, &items, cx);

    // Not drawn in the new mode, and drawn again back in the folder's.
    new_mode(&shell, cx);
    drawn(&mut table, &shell, "In the new mode", false, cx);
    shell.click("add-mode-existing", cx);
    drawn(
        &mut table,
        &shell,
        "Back in the existing-folder mode",
        true,
        cx,
    );

    // Not drawn for a registered folder.
    check(&shell, &a, cx);
    drawn(&mut table, &shell, "A registered folder", false, cx);

    table.done();
}

/// C1, as R-12: the delegate's `on_will_change` leaves the selection as it
/// was for the remote sharing cannot use, and takes one it can.
fn refused_by_the_delegate(table: &mut Table, items: &[Share]) {
    let row = |ix| IndexPath::default().row(ix);
    let mut delegate = Shares::new(items.to_vec());
    let mut selection = vec![(row(2), keep())];
    delegate.on_will_change(
        &mut selection,
        &[
            SearchableListChange::Deselect { index: row(2) },
            SearchableListChange::Select { index: row(1) },
        ],
    );
    table.fact(
        "on_will_change refuses the remote that is not SSH",
        selection == [(row(2), keep())],
        &format!("the selection kept (it is {selection:?})"),
    );
    delegate.on_will_change(
        &mut selection,
        &[
            SearchableListChange::Deselect { index: row(2) },
            SearchableListChange::Select { index: row(0) },
        ],
    );
    table.fact(
        "on_will_change takes the SSH remote",
        selection == [(row(0), items[0].clone())],
        &format!("origin selected (it is {selection:?})"),
    );
}
