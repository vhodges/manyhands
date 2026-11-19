//! What the Add an existing folder tables share (Wave 04 Cycle 05a design
//! B5 to B7, B10): a table of rows that fails once, naming every row that
//! did not hold; canned checks, built where a real repository cannot give
//! the case (design Testing); a check landed as the read would land it; and
//! the reads of the sheet the rows make.

use std::path::Path;

use gpui_kit::{ElementId, FocusHandle, TestAppContext, test::TestWindowExt};
use manyhands::{
    repository::{
        ConfigurationDto, ConfigurationState, IdentityAvailability, ProblemDto, RemoteDto,
        RepositoryInspectionDto,
    },
    results::ProblemCode,
};

use super::sheet;
use crate::desktop::{
    services::ReadResult,
    view::harness::{Shell, key},
};

/// The rows of one test, as they are read. A row that does not hold is
/// kept with what was found, and the table fails once, at its end.
pub(super) struct Table {
    name: &'static str,
    held: Vec<String>,
    failed: Vec<String>,
}

impl Table {
    pub(super) fn new(name: &'static str) -> Self {
        Self {
            name,
            held: Vec::new(),
            failed: Vec::new(),
        }
    }

    /// The row `row`: `held` says whether `what` is so.
    pub(super) fn fact(&mut self, row: &str, held: bool, what: &str) {
        if held {
            self.held.push(row.to_owned());
        } else {
            self.failed.push(format!("{row}: {what} does not hold"));
        }
    }

    /// Fails, naming every row that did not hold, unless none did.
    pub(super) fn done(self) {
        eprintln!("{}: rows that held: {:?}", self.name, self.held);
        assert!(
            self.failed.is_empty(),
            "{}: {} of {} rows did not hold:\n  {}",
            self.name,
            self.failed.len(),
            self.failed.len() + self.held.len(),
            self.failed.join("\n  ")
        );
    }
}

/// A check of `root` as the read gives it for a plain repository with
/// `main` checked out, its only branch, not registered, not set up, with an
/// identity and no remote. Each canned row changes what it is about.
pub(super) fn canned(root: &Path) -> RepositoryInspectionDto {
    RepositoryInspectionDto {
        selected_path: key(root),
        root: key(root),
        registered: false,
        head_branch: Some("main".to_owned()),
        local_branches: vec!["main".to_owned()],
        configuration: ConfigurationDto {
            state: ConfigurationState::Missing,
            primary_branch: None,
            publication_remote: None,
            problems: Vec::new(),
        },
        identity_state: IdentityAvailability::Available,
        remotes: Vec::new(),
        operations: Vec::new(),
    }
}

/// `canned`, set up: a valid configuration on `primary`.
pub(super) fn set_up(root: &Path, primary: &str) -> RepositoryInspectionDto {
    let mut check = canned(root);
    check.configuration.state = ConfigurationState::Valid;
    check.configuration.primary_branch = Some(primary.to_owned());
    check
}

/// `canned`, its configuration invalid with one problem.
pub(super) fn needs_fixing(root: &Path) -> RepositoryInspectionDto {
    let mut check = canned(root);
    check.configuration.state = ConfigurationState::Invalid;
    check.configuration.problems = vec![ProblemDto {
        code: ProblemCode::MalformedConfiguration,
        path: None,
        target_id: None,
    }];
    check
}

/// The guidance of `needs_fixing`'s problem, as its row reads it.
pub(super) fn fixing_guidance() -> String {
    ProblemCode::MalformedConfiguration.guidance().to_owned()
}

/// A remote `name` whose redacted fetch location is `location`.
pub(super) fn remote(name: &str, location: &str, eligible: bool) -> RemoteDto {
    RemoteDto {
        name: name.to_owned(),
        fetch_location: location.to_owned(),
        push_location: location.to_owned(),
        publication_eligible: eligible,
        selected_for_publication: false,
    }
}

/// `root`'s text typed in the folder field, and the check `result` landed
/// for it, as the read would land it: any check still in flight is then
/// stale (`generations`).
pub(super) fn land(
    shell: &Shell,
    root: &Path,
    result: ReadResult<RepositoryInspectionDto>,
    cx: &mut TestAppContext,
) {
    super::set(shell, "add-path", &key(root), cx);
    land_only(shell, &key(root), result, cx);
}

/// The check `result` landed for `text`, the field left as it is.
pub(super) fn land_only(
    shell: &Shell,
    text: &str,
    result: ReadResult<RepositoryInspectionDto>,
    cx: &mut TestAppContext,
) {
    let sheet = sheet(shell, cx);
    let text = text.to_owned();
    sheet.update(cx, |sheet, cx| sheet.land_check(text, result, cx));
    shell.run(cx);
}

/// The sentences of the folder's check rows, as drawn.
pub(super) fn rows(shell: &Shell, cx: &mut TestAppContext) -> Vec<String> {
    shell.rows("add-check", cx)
}

/// The action buttons a check can offer, each in its wrapper: Open its
/// settings, Create one here, Choose another, and Retry or Check again,
/// which share one.
pub(super) const ACTIONS: [&str; 4] = [
    "add-open-settings",
    "add-create-here",
    "add-choose",
    "add-retry",
];

/// The action a check offers: its wrapper and its label; `None` when none
/// is drawn. More than one drawn is reported as found.
pub(super) fn action(shell: &Shell, cx: &mut TestAppContext) -> Option<(&'static str, String)> {
    let drawn: Vec<(&'static str, String)> = ACTIONS
        .into_iter()
        .filter_map(|id| shell.label(id, cx).map(|label| (id, label)))
        .collect();
    match drawn.as_slice() {
        [] => None,
        [one] => Some(one.clone()),
        _ => Some(("several", format!("{drawn:?}"))),
    }
}

/// Clicks `id` when it is drawn; when it is not, records that and gives
/// false, so the table goes on to its other rows.
pub(super) fn press(
    table: &mut Table,
    shell: &Shell,
    row: &str,
    id: impl Into<ElementId>,
    cx: &mut TestAppContext,
) -> bool {
    let id = id.into();
    let drawn = shell.present(id.clone(), cx);
    table.fact(
        &format!("{row} (its control drawn)"),
        drawn,
        &format!("{id:?} drawn"),
    );
    if drawn {
        shell.click(id, cx);
    }
    drawn
}

/// The platform's chooser failed (Browse… is then unavailable).
pub(super) fn browse_failed(shell: &Shell, cx: &mut TestAppContext) {
    let sheet = sheet(shell, cx);
    shell
        .visual(cx)
        .update(|window, cx| sheet.update(cx, |sheet, cx| sheet.browse_failed(window, cx)));
    shell.run(cx);
}

/// Whether Add repository is enabled, as the footer draws it (B10).
pub(super) fn add_enabled(shell: &Shell, cx: &mut TestAppContext) -> bool {
    shell.frame(cx, |_, _| ());
    sheet(shell, cx).read_with(cx, |sheet, cx| sheet.add_enabled(cx))
}

/// Two draws, with what each starts run after it: a focus is read after
/// them (Cycle 04's A6 rule).
pub(super) fn settle(shell: &Shell, cx: &mut TestAppContext) {
    for _ in 0..2 {
        shell.frame(cx, |_, _| ());
        shell.run(cx);
    }
}

/// Whether `handle` holds focus, after two draws.
pub(super) fn holds_handle(shell: &Shell, handle: &FocusHandle, cx: &mut TestAppContext) -> bool {
    settle(shell, cx);
    shell.frame(cx, |window, _| handle.is_focused(window))
}

/// Whether the element `id`, or something in it, holds focus, after two
/// draws.
pub(super) fn holds(shell: &Shell, id: impl Into<ElementId>, cx: &mut TestAppContext) -> bool {
    settle(shell, cx);
    let id = id.into();
    shell.frame(cx, |window, _| {
        window
            .try_find(id)
            .is_some_and(|element| element.focused() == Some(true))
    })
}

/// Whether a confirmation (or any dialog) is open.
pub(super) fn has_dialog(shell: &Shell, cx: &mut TestAppContext) -> bool {
    use gpui_kit::component::WindowExt;
    shell.frame(cx, |window, cx| window.has_active_dialog(cx))
}

/// Whether the Add sheet is still open.
pub(super) fn sheet_open(shell: &Shell, cx: &mut TestAppContext) -> bool {
    shell.present("add-sheet", cx)
}
