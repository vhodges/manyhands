//! The Add folder's check: `inspect_repository` of the folder's text, read
//! under `ViewScope::AddRepository` (Wave 04 Cycle 05a R-9), and what it
//! found, drawn under the folder.

use std::path::PathBuf;

use gpui_kit::{
    AppContext, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{ActiveTheme, v_flex},
    div,
};
use manyhands::{
    repository::{ConfigurationState, IdentityAvailability, RepositoryInspectionDto},
    results::ResultCode,
};

use super::{
    ALREADY_ADDED, AddRepositorySheet, CHECKED_OUT, CHECKING, Form, NOT_ADDED, render::text,
};
use crate::desktop::{
    model::{list_state::LABEL_BUSY, stamp::ViewScope},
    services::{ReadKind, ReadResult},
    view::sidebar::configuration_label,
};

/// What checking the Add folder found.
pub(super) enum Inspection {
    /// Not checked since its text last changed.
    None,
    Checking(String),
    Checked {
        /// The text that was checked.
        path: String,
        inspection: Box<RepositoryInspectionDto>,
    },
    Busy(String),
    Failed(ResultCode),
}

impl Inspection {
    /// The text this check is of, if it is of one.
    pub(super) fn path(&self) -> Option<&str> {
        match self {
            Self::Checking(path) | Self::Busy(path) | Self::Checked { path, .. } => Some(path),
            Self::None | Self::Failed(_) => None,
        }
    }

    /// Whether the check of `path` found that Git has no identity for it.
    pub(super) fn identity_required(&self, path: &str) -> bool {
        matches!(self, Self::Checked { path: checked, inspection }
            if checked == path && inspection.identity_state == IdentityAvailability::Required)
    }
}

impl AddRepositorySheet {
    pub(super) fn forget_inspection(&mut self, cx: &mut Context<Self>) {
        self.inspection = Inspection::None;
        self.submit_pending = false;
        self.failed_path = None;
        self.inspect_read = None;
        self.generations.next(ViewScope::AddRepository);
        cx.notify();
    }

    /// Checks the Add folder with `inspect_repository`. An empty field is
    /// not checked.
    #[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
    pub(super) fn inspect(&mut self, cx: &mut Context<Self>) {
        let path = self.text(Form::Add.path(), cx);
        if path.is_empty() {
            self.forget_inspection(cx);
            return;
        }
        let stamp = self.generations.next(ViewScope::AddRepository);
        self.inspection = Inspection::Checking(path.clone());
        self.failed_path = None;
        let root = PathBuf::from(&path);
        self.inspect_read = Some(self.services.read(
            ReadKind::Inspect,
            stamp,
            move |service| service.inspect_repository(&root),
            cx,
            move |this: &mut Self, stamp, result, cx| {
                if !this.generations.is_current(&stamp) {
                    return;
                }
                this.inspection = match result {
                    ReadResult::Ok(inspection) => {
                        if this.text(Form::Add.branch(), cx).is_empty() {
                            this.pending_branch = inspection.head_branch.clone();
                        }
                        Inspection::Checked {
                            path,
                            inspection: Box::new(inspection),
                        }
                    }
                    ReadResult::Busy => Inspection::Busy(path),
                    ReadResult::Failed(failure) => {
                        this.failed_path = Some(path);
                        Inspection::Failed(failure.code)
                    }
                };
                if this.submit_pending {
                    let window = this.window;
                    cx.spawn(async move |this, cx| {
                        let _ = cx.update_window(window, |_, window, cx| {
                            let _ = this.update(cx, |this, cx| this.resume_submit(window, cx));
                        });
                    })
                    .detach();
                }
                cx.notify();
            },
        ));
        cx.notify();
    }

    /// The `repo.inspect` recovery: `root` in the Add folder, checked.
    pub(crate) fn inspect_root(
        &mut self,
        root: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(text) = root.to_str() else {
            return;
        };
        let text = text.to_owned();
        // No Add was pressed for this folder (R-48).
        self.submit_pending = false;
        self.inputs[Form::Add.path()].update(cx, |input, cx| input.set_value(text, window, cx));
        self.inspect(cx);
    }

    /// Whether the Add folder's current text has been checked: what was
    /// found, or that the check failed.
    pub(super) fn add_checked(&self, path: &str) -> bool {
        matches!(&self.inspection, Inspection::Checked { path: checked, .. } if checked == path)
            || self.failed_path.as_deref() == Some(path)
    }

    /// What the check of the Add folder found.
    pub(super) fn inspection(&self, cx: &Context<Self>) -> Option<impl IntoElement + use<>> {
        let muted = cx.theme().muted_foreground;
        let body = v_flex().gap_1();
        let body = match &self.inspection {
            Inspection::None => return None,
            Inspection::Checking(_) => body.child(text("add-inspection", CHECKING)),
            Inspection::Busy(_) => body.child(text("add-inspection", LABEL_BUSY)),
            Inspection::Failed(code) => body.child(text("add-inspection", code.message())).child(
                div()
                    .id("add-inspection-code")
                    .test_support()
                    .aria_label(code.as_str())
                    .text_xs()
                    .text_color(muted)
                    .child(code.as_str()),
            ),
            Inspection::Checked { inspection, .. } => {
                let state = if inspection.registered {
                    ALREADY_ADDED
                } else {
                    NOT_ADDED
                };
                let configuration = &inspection.configuration;
                let body = body.child(text("add-inspection", state)).children(
                    inspection
                        .head_branch
                        .clone()
                        .map(|branch| text("add-head-branch", format!("{CHECKED_OUT}: {branch}"))),
                );
                let body = if inspection.registered
                    || configuration.state != ConfigurationState::Missing
                {
                    body.child(text(
                        "add-configuration",
                        configuration_label(configuration.state),
                    ))
                } else {
                    body
                };
                body.children(
                    configuration
                        .problems
                        .iter()
                        .enumerate()
                        .map(|(ix, problem)| text(("add-problem", ix), problem.code.guidance())),
                )
            }
        };
        Some(body)
    }
}
