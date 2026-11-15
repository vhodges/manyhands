//! The Add folder's check (Wave 04 Cycle 05a design B6 item 2):
//! `inspect_repository` of the folder's text, read under
//! `ViewScope::AddRepository` (R-9), and whether Add repository can go
//! ahead on what it found (B10). Its rows and action are `rows`.

use std::path::{Path, PathBuf};

use gpui_kit::{App, AppContext, Context, Window};
use manyhands::{
    repository::{ConfigurationState, IdentityAvailability, RepositoryInspectionDto},
    results::ResultCode,
};

use super::{AddRepositorySheet, Form};
use crate::desktop::{
    model::stamp::ViewScope,
    services::{ReadKind, ReadResult},
};

/// What checking the Add folder found, each of the text it was of.
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
    Failed {
        path: String,
        code: ResultCode,
    },
    /// A route's root that is not text (Part G item 4), named over the
    /// field it emptied.
    NotText(String),
}

impl Inspection {
    /// The text this check is of.
    pub(super) fn path(&self) -> Option<&str> {
        match self {
            Self::Checking(path)
            | Self::Busy(path)
            | Self::NotText(path)
            | Self::Checked { path, .. }
            | Self::Failed { path, .. } => Some(path),
            Self::None => None,
        }
    }

    /// The check of `path` has landed: what it found, or that it could not.
    pub(super) fn landed_for(&self, path: &str) -> bool {
        !matches!(self, Self::None | Self::Checking(_)) && self.path() == Some(path)
    }

    /// Whether the check of `path` found that Git has no identity for it.
    pub(super) fn identity_required(&self, path: &str) -> bool {
        matches!(self, Self::Checked { path: checked, inspection }
            if checked == path && inspection.identity_state == IdentityAvailability::Required)
    }

    /// The configuration a check found is Valid: it holds the prefix
    /// (R-31).
    pub(super) fn set_up(&self) -> bool {
        matches!(self, Self::Checked { inspection, .. }
            if inspection.configuration.state == ConfigurationState::Valid)
    }
}

/// The branch an add can start from (B7): the checked-out one, unless the
/// configuration names another main branch.
pub(super) fn allowed_branch(check: &RepositoryInspectionDto) -> Option<&str> {
    let head = check.head_branch.as_deref()?;
    let configuration = &check.configuration;
    let other = configuration.state == ConfigurationState::Valid
        && configuration
            .primary_branch
            .as_deref()
            .is_some_and(|primary| primary != head);
    (!other).then_some(head)
}

/// The remote the configuration shares through, when it is among the
/// folder's remotes and is not an SSH address (B6).
pub(super) fn not_ssh(check: &RepositoryInspectionDto) -> Option<&str> {
    let remote = check.configuration.publication_remote.as_deref()?;
    let set_up = check.configuration.state == ConfigurationState::Valid;
    (set_up
        && check
            .remotes
            .iter()
            .any(|found| found.name == remote && !found.publication_eligible))
    .then_some(remote)
}

impl AddRepositorySheet {
    pub(super) fn forget_inspection(&mut self, cx: &mut Context<Self>) {
        self.inspection = Inspection::None;
        self.submit_pending = false;
        self.inspect_read = None;
        self.branches_stale = true;
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
        self.branches_stale = true;
        let root = PathBuf::from(&path);
        self.inspect_read = Some(self.services.read(
            ReadKind::Inspect,
            stamp,
            move |service| service.inspect_repository(&root),
            cx,
            move |this: &mut Self, stamp, result, cx| {
                // A check of text since changed is dropped, and the wait
                // with it.
                if this.generations.is_current(&stamp) {
                    this.landed(path, result, cx);
                }
            },
        ));
        cx.notify();
    }

    /// The check of `path` landed with `result`; an Add waiting for it goes
    /// on, once the window can be reached (R-T12b).
    fn landed(
        &mut self,
        path: String,
        result: ReadResult<RepositoryInspectionDto>,
        cx: &mut Context<Self>,
    ) {
        self.inspection = match result {
            ReadResult::Ok(inspection) => Inspection::Checked {
                path,
                inspection: Box::new(inspection),
            },
            ReadResult::Busy => Inspection::Busy(path),
            ReadResult::Failed(failure) => Inspection::Failed {
                path,
                code: failure.code,
            },
        };
        self.branches_stale = true;
        if self.submit_pending {
            let window = self.window;
            cx.spawn(async move |this, cx| {
                let _ = cx.update_window(window, |_, window, cx| {
                    let _ = this.update(cx, |this, cx| this.resume_submit(window, cx));
                });
            })
            .detach();
        }
        cx.notify();
    }

    /// The `repo.inspect` recovery: `root` in the Add folder, checked. A
    /// root that is not text empties the field and is named (P22).
    pub(crate) fn inspect_root(
        &mut self,
        root: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text = root.to_str().map(str::to_owned);
        // No Add was pressed for this folder (R-48).
        self.submit_pending = false;
        let shown = text.clone().unwrap_or_default();
        self.inputs[Form::Add.path()].update(cx, |input, cx| input.set_value(shown, window, cx));
        if text.is_some() {
            self.inspect(cx);
        } else {
            self.forget_inspection(cx);
            self.inspection = Inspection::NotText(String::new());
        }
    }

    /// A success for an earlier open, landing while a later open shows
    /// (B10, B2 step 7): its check of the folder just added, which said it
    /// was not, is forgotten, so it is checked again. Its text and focus
    /// stay (Ruling 19 C-5).
    pub(crate) fn forget_check_of(&mut self, root: &Path, cx: &mut Context<Self>) {
        let stale = matches!(&self.inspection, Inspection::Checked { path, inspection }
            if !inspection.registered
                && (Path::new(path) == root || Path::new(&inspection.root) == root));
        if stale {
            self.forget_inspection(cx);
        }
    }

    /// B10: Add repository can go ahead on the folder's text and its check.
    /// Not checked yet, or checking, it can: pressing it checks first.
    pub(super) fn add_enabled(&self, cx: &App) -> bool {
        let path = self.text(Form::Add.path(), cx);
        if self.adding || path.is_empty() {
            return false;
        }
        if !self.inspection.landed_for(&path) {
            return true;
        }
        match &self.inspection {
            Inspection::Checked { inspection, .. } => {
                !inspection.registered
                    && inspection.configuration.state != ConfigurationState::Invalid
                    && allowed_branch(inspection).is_some()
                    && not_ssh(inspection).is_none()
            }
            _ => false,
        }
    }

    /// The folder text whose check has landed with what it found (P8).
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn checked(&self) -> Option<&str> {
        match &self.inspection {
            Inspection::Checked { path, .. } => Some(path),
            _ => None,
        }
    }

    /// The folder text whose check has started or landed (P4).
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn inspected(&self) -> Option<&str> {
        match &self.inspection {
            Inspection::NotText(_) => None,
            inspection => inspection.path(),
        }
    }

    /// The check `result` landed for `path`, as the read lands it, a canned
    /// check or `Busy` included (P9 to P11): a check still in flight is
    /// then stale.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn land_check(
        &mut self,
        path: String,
        result: ReadResult<RepositoryInspectionDto>,
        cx: &mut Context<Self>,
    ) {
        self.generations.next(ViewScope::AddRepository);
        self.inspect_read = None;
        self.landed(path, result, cx);
    }
}
