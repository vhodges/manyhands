//! P8's rows (Wave 04 Cycle 05b design B5, Part C): each part's card names
//! it in words, not by its front matter key, with the kind of detail it
//! holds and its state; a taken row names its field the same way; and a set
//! that goes outside draws one card per path, with one row of actions below
//! them all (R-14).
//!
//! The conflicts are canned (D5 decision 9), as `support` cans them: a read
//! stands in place of the library's, and nothing is asserted of the
//! repository.

use gpui_kit::{ElementId, TestAppContext};
use manyhands::repository::ConflictDto;

use super::support::{Rig, Table, close_everything, external, into_the_editor, synchronization};
use crate::desktop::{
    test_support::fixtures::ticket_path,
    view::{
        conflict::{
            CHOOSE_ONE, CHOSEN, EDITED, TAKEN_THEIRS, TICKET_DETAIL,
            tests::{TICKET, canned, canned_path},
        },
        harness::Shell,
    },
};

/// `TICKET`'s source: its `status`, `team` when given, the key Manyhands
/// does not define `estimate`, and `body`.
fn ticket(status: &str, team: Option<&str>, estimate: u32, body: &str) -> String {
    let team = team.map_or(String::new(), |team| format!("team: {team}\n"));
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {TICKET}\ntitle: Same\n\
         type: task\nstatus: {status}\n{team}estimate: {estimate}\n---\n{body}\n"
    )
}

/// The share's conflict on `TICKET`: `status`, `estimate` and the body
/// changed by both sides, differently; `team` set by theirs alone, so it
/// is taken from theirs (R-5).
fn named_parts() -> ConflictDto {
    let mut path = canned_path(TICKET, '1');
    path.base = Some(ticket("open", None, 1, "Base body."));
    path.local = Some(ticket("doing", None, 2, "Local body."));
    path.incoming = Some(ticket("blocked", Some("core"), 3, "Incoming body."));
    canned(synchronization(), vec![path])
}

/// How many drawn elements are `name` itself, or `(name, n)` for any `n`.
fn drawn(shell: &Shell, name: &str, cx: &mut TestAppContext) -> usize {
    shell.frame(cx, |window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .filter_map(|element| element.path().last())
            .filter(|id| match id {
                ElementId::Name(named) | ElementId::NamedInteger(named, _) => {
                    named.as_ref() == name
                }
                _ => false,
            })
            .count()
    })
}

pub(super) async fn rows(table: &mut Table, cx: &mut TestAppContext) {
    let rig = Rig::new(cx);
    let shell = &rig.shell;
    let sheet = rig
        .by_banner(Some(named_parts()), "conflict-save", cx)
        .await;

    let label = |id: (&'static str, usize), cx: &mut TestAppContext| shell.label(id, cx);
    let status = label(("conflict-field", 0), cx);
    table.fact(
        "P8 status is named Status",
        status.as_deref() == Some("Status"),
        &format!("the first field reads \"Status\" (it reads {status:?})"),
    );
    let unknown = label(("conflict-field", 1), cx);
    table.fact(
        "P8 an unknown key is named as written",
        unknown.as_deref() == Some("estimate"),
        &format!("the second field reads \"estimate\" (it reads {unknown:?})"),
    );
    let kinds = shell.rows("conflict-field-kind", cx);
    table.fact(
        "P8 each field is a Ticket detail, the body has no kind",
        kinds == [TICKET_DETAIL, TICKET_DETAIL] && shell.present(("conflict-body", 0usize), cx),
        &format!("two kinds, each {TICKET_DETAIL:?}, beside a body (they read {kinds:?})"),
    );
    let taken = label(("conflict-taken", 0), cx);
    table.fact(
        "P8 a taken field is named in words",
        taken.as_deref() == Some(format!("Team: {TAKEN_THEIRS}").as_str()),
        &format!("the taken row reads \"Team: …\" (it reads {taken:?})"),
    );

    let waiting = label(("conflict-choice", 0), cx);
    table.fact(
        "P8 a part not chosen reads Choose one",
        waiting.as_deref() == Some(CHOOSE_ONE),
        &format!("Status reads {CHOOSE_ONE:?} (it reads {waiting:?})"),
    );
    shell.click(("conflict-keep-mine", 0usize), cx);
    let chosen = label(("conflict-choice", 0), cx);
    table.fact(
        "P8 a choice reads Chosen",
        chosen.as_deref() == Some(CHOSEN),
        &format!("Status reads {CHOSEN:?} once kept (it reads {chosen:?})"),
    );
    into_the_editor(shell, &sheet, cx);
    shell.visual(cx).simulate_input(" typed");
    shell.run(cx);
    let edited = label(("conflict-body-choice", 0), cx);
    table.fact(
        "P8 a typed Result reads Edited",
        edited.as_deref() == Some(EDITED),
        &format!("the body reads {EDITED:?} once typed in (it reads {edited:?})"),
    );
    close_everything(shell, cx);

    rig.by_banner(Some(external()), "conflict-open-folder", cx)
        .await;
    let cards = shell.rows("conflict-external", cx);
    let reasons = shell.rows("conflict-external-reason", cx);
    let paths = [ticket_path(TICKET), "blob.bin".to_owned()];
    let per_path = cards.len() == paths.len()
        && reasons.len() == paths.len()
        && cards
            .iter()
            .zip(&paths)
            .all(|(card, path)| card.starts_with(&format!("{path}: ")));
    table.fact(
        "P8 an external set draws one card per path",
        per_path,
        &format!("a card with its reason for each of {paths:?} (cards {cards:?})"),
    );
    let actions = (
        drawn(shell, "conflict-open-folder", cx),
        drawn(shell, "conflict-check-again", cx),
    );
    table.fact(
        "P8 an external set has one row of actions",
        actions == (1, 1),
        &format!("one Open folder and one Check again (drawn {actions:?})"),
    );
    close_everything(shell, cx);
}
