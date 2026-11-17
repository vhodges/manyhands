//! The rows the Add folder's check is drawn as (Wave 04 Cycle 05a design
//! B6 item 2), each an icon and a sentence, and the one action it offers:
//! Open its settings, Create one here, Choose another, Retry or Check
//! again, a Kit button focus can be put on (B12; G22).

use std::path::PathBuf;

use gpui_kit::{
    ClickEvent, Context, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{Icon, Sizable, button::Button, h_flex, tooltip::Tooltip, v_flex},
    div,
    prelude::FluentBuilder,
    px,
};
use manyhands::{
    repository::{ConfigurationState, RepositoryInspectionDto},
    results::ResultCode,
};

use super::{
    ALREADY_ADDED, ALREADY_SET_UP, AddEvent, AddRepositorySheet, BUSY, CHECK_AGAIN, CHECKING,
    CHOOSE_ANOTHER, CREATE_HERE, IS_REPOSITORY, NEEDS_FIXING, NO_BRANCH, NOT_REPOSITORY,
    NOT_SET_UP, NOT_TEXT, OPEN_SETTINGS, RETRY, WILL_BE_ADDED,
    checks::{Inspection, allowed_branch, not_ssh},
    not_ssh_remote, other_main_branch,
};
use crate::desktop::{
    assets::icon_path,
    view::{settings::wrapped_kit, shell::tokens::chrome_colors},
};

/// What a check row's icon says.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Sign {
    Working,
    Passed,
    Note,
    Failed,
}

/// One row of the checks: its sign, its sentence, and the code a failure
/// names in its tooltip.
struct Row {
    sign: Sign,
    text: String,
    tip: Option<&'static str>,
}

fn row(sign: Sign, text: impl Into<String>) -> Row {
    Row {
        sign,
        text: text.into(),
        tip: None,
    }
}

/// The action a check offers (B6).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Action {
    OpenSettings,
    CreateHere,
    ChooseAnother,
    Retry,
    CheckAgain,
}

/// Each action's wrapper; its own Kit button is `(id, 0)`. Retry and Check
/// again share one.
pub(super) const ACTION_IDS: [&str; 4] = [
    "add-open-settings",
    "add-create-here",
    "add-choose",
    "add-retry",
];

impl Action {
    fn id(self) -> &'static str {
        match self {
            Self::OpenSettings => ACTION_IDS[0],
            Self::CreateHere => ACTION_IDS[1],
            Self::ChooseAnother => ACTION_IDS[2],
            Self::Retry | Self::CheckAgain => ACTION_IDS[3],
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::OpenSettings => OPEN_SETTINGS,
            Self::CreateHere => CREATE_HERE,
            Self::ChooseAnother => CHOOSE_ANOTHER,
            Self::Retry => RETRY,
            Self::CheckAgain => CHECK_AGAIN,
        }
    }
}

/// B6's table: the rows of a landed check, and its action. A registered
/// folder shows its configuration's state and problems, and Open its
/// settings (decision 18); one that is not offers Check again for a
/// configuration that needs fixing, fixed outside the app (owner Ruling
/// 25), as for the branch and remote rows.
fn checked_rows(check: &RepositoryInspectionDto) -> (Vec<Row>, Option<Action>) {
    let configuration = &check.configuration;
    let mut rows = vec![if check.registered {
        row(Sign::Passed, ALREADY_ADDED)
    } else {
        row(Sign::Passed, IS_REPOSITORY)
    }];
    match configuration.state {
        ConfigurationState::Missing => rows.push(row(Sign::Note, NOT_SET_UP)),
        ConfigurationState::Valid => {
            rows.push(row(Sign::Passed, ALREADY_SET_UP));
            if !check.registered {
                rows.push(row(Sign::Passed, WILL_BE_ADDED));
            }
        }
        ConfigurationState::Invalid => {
            rows.push(row(Sign::Failed, NEEDS_FIXING));
            rows.extend(
                configuration
                    .problems
                    .iter()
                    .map(|problem| row(Sign::Note, problem.guidance())),
            );
        }
    }
    if check.registered {
        return (rows, Some(Action::OpenSettings));
    }
    let mut again = configuration.state == ConfigurationState::Invalid;
    match (&check.head_branch, allowed_branch(check)) {
        (None, _) => {
            rows.push(row(Sign::Failed, NO_BRANCH));
            again = true;
        }
        (Some(_), None) => {
            let primary = configuration.primary_branch.as_deref().unwrap_or_default();
            rows.push(row(Sign::Failed, other_main_branch(primary)));
            again = true;
        }
        (Some(_), Some(_)) => {}
    }
    if let Some(remote) = not_ssh(check) {
        rows.push(row(Sign::Failed, not_ssh_remote(remote)));
        again = true;
    }
    (rows, again.then_some(Action::CheckAgain))
}

impl AddRepositorySheet {
    /// The rows of the folder's check, and the action it offers.
    fn rows(&self) -> (Vec<Row>, Option<Action>) {
        match &self.inspection {
            Inspection::None => (Vec::new(), None),
            Inspection::Checking(_) => (vec![row(Sign::Working, CHECKING)], None),
            Inspection::Busy(_) => (vec![row(Sign::Failed, BUSY)], Some(Action::Retry)),
            Inspection::NotText(_) => (
                vec![row(Sign::Failed, NOT_TEXT)],
                Some(Action::ChooseAnother),
            ),
            Inspection::Failed {
                code: ResultCode::NotRepository,
                ..
            } => (
                vec![row(Sign::Failed, NOT_REPOSITORY)],
                Some(Action::CreateHere),
            ),
            Inspection::Failed { code, .. } => {
                let failed = Row {
                    tip: Some(code.as_str()),
                    ..row(Sign::Failed, code.message())
                };
                (vec![failed], Some(Action::ChooseAnother))
            }
            Inspection::Checked { inspection, .. } => checked_rows(inspection),
        }
    }

    /// The focus of the action the check offers, if any (G22).
    pub(super) fn action_focus(&self) -> Option<gpui_kit::FocusHandle> {
        self.rows().1.map(|action| self.focus[action.id()].clone())
    }

    /// The checks (`add-checks`): its rows, then its action, a Kit button
    /// focus can be put on (B12).
    pub(super) fn checks(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        let (rows, action) = self.rows();
        if rows.is_empty() {
            return None;
        }
        let colors = chrome_colors(cx);
        let rows = rows.into_iter().enumerate().map(|(ix, row)| {
            let (icon, color) = match row.sign {
                Sign::Working => ("loader-circle", colors.secondary),
                Sign::Passed => ("circle-check", colors.green),
                Sign::Note => ("info", colors.secondary),
                Sign::Failed => ("circle-x", colors.red),
            };
            let text = SharedString::from(row.text);
            h_flex()
                .id(("add-check", ix))
                .test_support()
                .aria_label(text.clone())
                .items_start()
                .gap(px(6.))
                .text_size(px(12.))
                .child(
                    Icon::default()
                        .path(icon_path(icon))
                        .text_color(color)
                        .w(px(13.))
                        .h(px(13.)),
                )
                .child(div().flex_1().min_w_0().child(text))
                .when_some(row.tip, |row, tip| {
                    row.tooltip(move |window, cx| Tooltip::new(tip).build(window, cx))
                })
        });
        let action = action.map(|action| {
            let id = action.id();
            let button = Button::new((id, 0usize))
                .small()
                .label(action.label())
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.run_action(action, window, cx)
                }));
            wrapped_kit(
                id,
                action.label(),
                &self.focus[id],
                &self.scroll,
                (id, 0usize),
                button,
            )
        });
        Some(
            v_flex()
                .id("add-checks")
                .test_support()
                .gap(px(4.))
                .children(rows)
                .children(action.map(|action| div().pt(px(4.)).child(action))),
        )
    }

    /// What a check's action does (B6). Retry and Check again go with the
    /// check they start, so focus is put on the folder first (Ruling 22);
    /// Choose another's Browse… puts it there once its folder is chosen.
    fn run_action(&mut self, action: Action, window: &mut Window, cx: &mut Context<Self>) {
        match action {
            Action::OpenSettings => {
                if let Inspection::Checked { inspection, .. } = &self.inspection {
                    cx.emit(AddEvent::OpenSettings(PathBuf::from(&inspection.root)));
                }
            }
            Action::CreateHere => self.create_here(window, cx),
            Action::ChooseAnother if !self.browse_disabled() => self.choose_another(window, cx),
            Action::ChooseAnother => self.focus_path(window, cx),
            Action::Retry | Action::CheckAgain => {
                self.focus_path(window, cx);
                self.inspect(cx);
            }
        }
    }

    /// Choose another's own handle, which focus is on when it is pressed by
    /// keyboard (B6, B12).
    pub(super) fn choose_focus(&self) -> &gpui_kit::FocusHandle {
        &self.focus[Action::ChooseAnother.id()]
    }

    /// Focus on the Add folder field (B6; Ruling 22).
    pub(super) fn focus_path(&self, window: &mut Window, cx: &mut Context<Self>) {
        let path = self.add_path_focus(cx);
        window.focus(&path, cx);
    }

    /// The code each check row names in its tooltip (P9).
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn check_tips(&self) -> Vec<Option<&'static str>> {
        self.rows().0.into_iter().map(|row| row.tip).collect()
    }
}
