//! The Repositories page's registered repositories (design, Settings /
//! Repositories): each registration `list_repositories` gives, and for
//! the one chosen, its identity and initials, and Remove.
//!
//! **Identity and initials.** The identity shown is the one the library
//! reads for the repository, `repository_identity`: the name and email a
//! commit would carry, the configuration level they come from, and the
//! initials a short code would be composed with. (`inspect_repository`
//! says only whether one is available.) The form starts from that name
//! and email; its initials are left as they are, cleared, or set, which
//! is the library's `InitialsChange`. Set identity builds
//! `Intent::RepoIdentitySet` and opens the confirmation dialog; once a
//! request of it finishes, the identity is read again and shown.
//!
//! **Remove** builds `Intent::RepoRemove` for the chosen root and opens
//! the confirmation dialog, which says local files remain. What a
//! successful removal does to the tabs and the selection, the workspace
//! does, for a removal started from anywhere (`Workspace::
//! operations_changed`). Here the list is read again, and the removed
//! root is no longer the chosen one.
//!
//! The list is read when the page is made and again whenever a request
//! that registers, removes or sets an identity finishes, whoever started
//! it. Text is fixed strings, the library's own values, and
//! `ResultCode::message()` with its code.

use std::{collections::HashSet, path::PathBuf};

use gpui_kit::{
    ClickEvent, Context, FocusHandle, IntoElement, ParentElement, Styled, Window,
    component::{
        ActiveTheme, WindowExt,
        button::{Button, ButtonVariants},
        h_flex, v_flex,
    },
    div,
};
use manyhands::{
    repository::{IdentityDto, InitialsChange},
    results::{Outcome, ResultCode},
};

use super::{RepositoriesPage, text};
use crate::desktop::{
    model::{
        list_state::LABEL_BUSY,
        stamp::{Generations, SettingsPage, ViewScope},
    },
    services::{
        ReadKind, ReadResult, Services,
        requests::{DesktopRequestId, Intent, RequestState},
    },
    view::{confirm::confirm, settings::wrapped},
};

pub(crate) const MANAGED_HEADING: &str = "Registered repositories";
pub(crate) const NO_REGISTRATIONS: &str = "No repository is registered yet.";
pub(crate) const READING_REGISTRATIONS: &str = "Reading the registered repositories";
pub(crate) const MANAGE: &str = "Manage";
pub(crate) const MANAGING: &str = "Managing";
pub(crate) const IDENTITY_HEADING: &str = "Identity";
pub(crate) const READING_IDENTITY: &str = "Reading the identity";
pub(crate) const NO_IDENTITY: &str = "Git finds no name and email for this repository.";
pub(crate) const NO_INITIALS: &str = "No initials to compose a short code with";
pub(crate) const INITIALS: &str = "Initials";
pub(crate) const INITIALS_FIELD: &str = "Initials (two or three letters or digits)";
pub(crate) const INITIALS_UNCHANGED: &str = "Leave initials as they are";
pub(crate) const INITIALS_CLEAR: &str = "Clear initials";
pub(crate) const INITIALS_SET: &str = "Set initials";
pub(crate) const SET_IDENTITY: &str = "Set identity";
pub(crate) const REMOVE: &str = "Remove repository";
pub(crate) const REMOVE_NOTE: &str =
    "Removing takes the repository out of Manyhands. Its files stay where they are.";

const NAME_FIELD: &str = "identity-name";
const EMAIL_FIELD: &str = "identity-email";
const INITIALS_INPUT: &str = "identity-initials";
const SUBMIT: &str = "identity-submit";
const REMOVE_SUBMIT: &str = "remove-submit";

/// The identity form's inputs.
pub(super) const FIELDS: [&str; 3] = [NAME_FIELD, EMAIL_FIELD, INITIALS_INPUT];
/// The section's other controls, each in a focus-tracking wrapper.
pub(super) const CONTROLS: [&str; 5] = [
    SUBMIT,
    REMOVE_SUBMIT,
    "identity-initials-unchanged",
    "identity-initials-clear",
    "identity-initials-set",
];

/// What becomes of the repository's initials.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Initials {
    Unchanged,
    Clear,
    Set,
}

impl Initials {
    const ALL: [Self; 3] = [Self::Unchanged, Self::Clear, Self::Set];

    fn id(self) -> &'static str {
        match self {
            Self::Unchanged => "identity-initials-unchanged",
            Self::Clear => "identity-initials-clear",
            Self::Set => "identity-initials-set",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Unchanged => INITIALS_UNCHANGED,
            Self::Clear => INITIALS_CLEAR,
            Self::Set => INITIALS_SET,
        }
    }
}

/// A read's state, as the section shows it.
enum Read<T> {
    None,
    Loading,
    Ready(T),
    Busy,
    Failed(ResultCode),
}

impl<T> Read<T> {
    fn from_result<U>(result: ReadResult<U>, f: impl FnOnce(U) -> T) -> Self {
        match result {
            ReadResult::Ok(value) => Self::Ready(f(value)),
            ReadResult::Busy => Self::Busy,
            ReadResult::Failed(failure) => Self::Failed(failure.code),
        }
    }
}

/// The section's state.
pub(super) struct Managed {
    /// The roots `list_repositories` last gave.
    registrations: Read<Vec<String>>,
    list_generations: Generations,
    list_read: Option<gpui_kit::Task<()>>,
    /// The root the identity form and Remove act on.
    root: Option<PathBuf>,
    identity: Read<Box<IdentityDto>>,
    identity_generations: Generations,
    identity_read: Option<gpui_kit::Task<()>>,
    initials: Initials,
    /// The first identity read for a newly chosen root fills the form.
    prefill: bool,
    /// The name and email it fills it with, at the next frame, which has
    /// the window a field needs.
    pending: Option<(String, String)>,
    /// The focus of each row's Manage, by row.
    rows: Vec<FocusHandle>,
    /// The finished requests already answered, by attempt.
    seen: HashSet<(DesktopRequestId, u32)>,
}

/// The requests whose finishing can change what the section lists.
fn concerns(intent: &Intent) -> bool {
    matches!(
        intent,
        Intent::RepoCreate { .. }
            | Intent::RepoEnable { .. }
            | Intent::RepoRemove { .. }
            | Intent::RepoIdentitySet { .. }
    )
}

impl Managed {
    pub(super) fn new(services: &Services, cx: &mut Context<RepositoriesPage>) -> Self {
        // What finished before the page was made is already in the list
        // it reads now.
        let seen = services
            .operations()
            .read(cx)
            .requests()
            .iter()
            .filter(|request| matches!(request.state, RequestState::Finished(..)))
            .map(|request| (request.id, request.attempt))
            .collect();
        Self {
            registrations: Read::None,
            list_generations: Generations::default(),
            list_read: None,
            root: None,
            identity: Read::None,
            identity_generations: Generations::default(),
            identity_read: None,
            initials: Initials::Unchanged,
            prefill: false,
            pending: None,
            rows: Vec::new(),
            seen,
        }
    }
}

const SCOPE: ViewScope = ViewScope::Settings(SettingsPage::Repositories);

impl RepositoriesPage {
    /// The roots the section lists.
    #[cfg(test)]
    pub(crate) fn registrations(&self) -> Vec<String> {
        match &self.managed.registrations {
            Read::Ready(roots) => roots.clone(),
            _ => Vec::new(),
        }
    }

    /// The shown identity's name line.
    #[cfg(test)]
    pub(crate) fn identity_shown(&self) -> Option<String> {
        match &self.managed.identity {
            Read::Ready(identity) => Some(name_line(identity)),
            _ => None,
        }
    }

    /// Reads the registered repositories.
    #[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
    pub(super) fn load_registrations(&mut self, cx: &mut Context<Self>) {
        let stamp = self.managed.list_generations.next(SCOPE);
        if !matches!(self.managed.registrations, Read::Ready(_)) {
            self.managed.registrations = Read::Loading;
        }
        self.managed.list_read = Some(self.services.read(
            ReadKind::Repositories,
            stamp,
            |service| service.list_repositories(),
            cx,
            |this: &mut Self, stamp, result, cx| {
                if !this.managed.list_generations.is_current(&stamp) {
                    return;
                }
                this.managed.registrations = Read::from_result(result, |list| {
                    list.items.into_iter().map(|summary| summary.root).collect()
                });
                cx.notify();
            },
        ));
        cx.notify();
    }

    /// Reads the chosen root's identity.
    #[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
    fn load_identity(&mut self, cx: &mut Context<Self>) {
        let Some(root) = self.managed.root.clone() else {
            return;
        };
        let stamp = self.managed.identity_generations.next(SCOPE);
        self.managed.identity = Read::Loading;
        self.managed.identity_read = Some(self.services.read(
            ReadKind::Identity,
            stamp,
            move |service| {
                let repo = service.resolve_repository(&root)?;
                service.repository_identity(&repo)
            },
            cx,
            |this: &mut Self, stamp, result, cx| {
                if !this.managed.identity_generations.is_current(&stamp) {
                    return;
                }
                if let ReadResult::Ok(identity) = &result
                    && std::mem::take(&mut this.managed.prefill)
                {
                    this.managed.pending = Some((
                        identity.name.clone().unwrap_or_default(),
                        identity.email.clone().unwrap_or_default(),
                    ));
                }
                this.managed.identity = Read::from_result(result, Box::new);
                cx.notify();
            },
        ));
        cx.notify();
    }

    /// The form is filled from the first identity read for its root.
    pub(super) fn apply_prefill(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some((name, email)) = self.managed.pending.take() {
            self.inputs[NAME_FIELD].update(cx, |input, cx| input.set_value(name, window, cx));
            self.inputs[EMAIL_FIELD].update(cx, |input, cx| input.set_value(email, window, cx));
        }
    }

    /// Chooses `root` for the identity form and Remove; with `focus`, the
    /// name field takes focus (the `repo.identity_set` recovery).
    pub(crate) fn manage_root(
        &mut self,
        root: PathBuf,
        focus: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.managed.root = Some(root);
        self.managed.initials = Initials::Unchanged;
        self.managed.prefill = true;
        self.managed.pending = None;
        for id in FIELDS {
            self.inputs[id].update(cx, |input, cx| input.set_value("", window, cx));
        }
        self.load_identity(cx);
        if focus {
            self.inputs[NAME_FIELD].update(cx, |input, cx| input.focus(window, cx));
        }
        cx.notify();
    }

    fn choose_initials(&mut self, initials: Initials, cx: &mut Context<Self>) {
        self.managed.initials = initials;
        cx.notify();
    }

    /// Set identity: the form's intent, in the confirmation dialog.
    pub(super) fn submit_identity(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) {
            return;
        }
        let Some(root) = self.managed.root.clone() else {
            return;
        };
        let initials = match self.managed.initials {
            Initials::Unchanged => InitialsChange::Unchanged,
            Initials::Clear => InitialsChange::Clear,
            Initials::Set => InitialsChange::Set(self.text(INITIALS_INPUT, cx)),
        };
        let intent = Intent::RepoIdentitySet {
            root,
            name: self.text(NAME_FIELD, cx),
            email: self.text(EMAIL_FIELD, cx),
            initials,
        };
        debug_assert!(intent.confirmed());
        let origin = window
            .focused(cx)
            .unwrap_or_else(|| self.focus[SUBMIT].clone());
        confirm(intent, origin, &self.services, window, cx);
    }

    /// Remove: `repo remove` of the chosen root, in the confirmation
    /// dialog.
    fn submit_remove(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) {
            return;
        }
        let Some(root) = self.managed.root.clone() else {
            return;
        };
        let origin = window
            .focused(cx)
            .unwrap_or_else(|| self.focus[REMOVE_SUBMIT].clone());
        confirm(
            Intent::RepoRemove { root },
            origin,
            &self.services,
            window,
            cx,
        );
    }

    /// A request finished: one that registers, removes or sets an identity
    /// reads the list again; one that set the chosen root's identity reads
    /// it again; a removal of the chosen root that succeeded lets it go.
    pub(super) fn operations_changed(&mut self, cx: &mut Context<Self>) {
        let mut list = false;
        let mut identity = false;
        let mut removed = false;
        let operations = self.services.operations().clone();
        for request in operations.read(cx).requests() {
            let RequestState::Finished(outcome, ..) = &request.state else {
                continue;
            };
            if !concerns(&request.intent)
                || !self.managed.seen.insert((request.id, request.attempt))
            {
                continue;
            }
            list = true;
            let chosen = request.intent.root() == self.managed.root.as_deref();
            match request.intent {
                Intent::RepoIdentitySet { .. } if chosen => identity = true,
                Intent::RepoRemove { .. } if chosen && *outcome == Outcome::Success => {
                    removed = true
                }
                _ => {}
            }
        }
        if removed {
            self.managed.root = None;
            self.managed.identity = Read::None;
            self.managed.identity_read = None;
            self.managed.identity_generations.next(SCOPE);
        } else if identity {
            self.load_identity(cx);
        }
        if list {
            self.load_registrations(cx);
        }
    }
}

/// The identity's name line, or that there is none.
fn name_line(identity: &IdentityDto) -> String {
    match &identity.name {
        Some(name) => format!("Name: {name}"),
        None => NO_IDENTITY.to_owned(),
    }
}

impl RepositoriesPage {
    /// The registrations, each with Manage.
    fn registration_rows(&mut self, cx: &mut Context<Self>) -> gpui_kit::Div {
        let muted = cx.theme().muted_foreground;
        let body = v_flex().gap_1();
        let roots = match &self.managed.registrations {
            Read::None | Read::Loading => {
                return body.child(text("managed-state", READING_REGISTRATIONS));
            }
            Read::Busy => return body.child(text("managed-state", LABEL_BUSY)),
            Read::Failed(code) => {
                return body
                    .child(text("managed-state", code.message()))
                    .child(div().text_xs().text_color(muted).child(code.as_str()));
            }
            Read::Ready(roots) if roots.is_empty() => {
                return body.child(text("managed-state", NO_REGISTRATIONS));
            }
            Read::Ready(roots) => roots.clone(),
        };
        while self.managed.rows.len() < roots.len() {
            self.managed.rows.push(cx.focus_handle());
        }
        let chosen = self.managed.root.as_deref().and_then(|root| root.to_str());
        let mut body = body;
        for (ix, root) in roots.into_iter().enumerate() {
            let managing = chosen == Some(root.as_str());
            let target = PathBuf::from(&root);
            let button = Button::new(("managed-choose-button", ix))
                .label(MANAGE)
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    if !window.has_active_dialog(cx) {
                        this.manage_root(target.clone(), false, window, cx)
                    }
                }));
            body = body.child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(text(("managed-row", ix), root))
                    .children(managing.then(|| text(("managed-managing", ix), MANAGING)))
                    .child(wrapped(
                        ("managed-choose", ix),
                        MANAGE,
                        &self.managed.rows[ix],
                        &self.scroll,
                        button,
                    )),
            );
        }
        body
    }

    /// The identity the library reads for the chosen root.
    fn current_identity(&self, cx: &Context<Self>) -> gpui_kit::Div {
        let muted = cx.theme().muted_foreground;
        let body = v_flex().gap_0p5();
        match &self.managed.identity {
            Read::None | Read::Loading => body.child(text("identity-state", READING_IDENTITY)),
            Read::Busy => body.child(text("identity-state", LABEL_BUSY)),
            Read::Failed(code) => body.child(text("identity-failure", code.message())).child(
                div()
                    .text_xs()
                    .text_color(muted)
                    .child(text("identity-failure-code", code.as_str())),
            ),
            Read::Ready(identity) => {
                let body = body.child(text("identity-current-name", name_line(identity)));
                let body = match &identity.email {
                    Some(email) => body
                        .child(text("identity-current-email", format!("Email: {email}")))
                        .child(text(
                            "identity-current-source",
                            format!("From: {}", identity.source.as_str()),
                        )),
                    None => body,
                };
                body.child(text(
                    "identity-current-initials",
                    match &identity.initials {
                        Some(initials) => format!(
                            "{INITIALS}: {initials} ({})",
                            identity.initials_source.as_str()
                        ),
                        None => NO_INITIALS.to_owned(),
                    },
                ))
            }
        }
    }

    /// The initials choice: Unchanged, Clear or Set, the chosen one said in
    /// text.
    fn initials_choice(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let mut choice = h_flex().gap_2().flex_wrap();
        for initials in Initials::ALL {
            let chosen = self.managed.initials == initials;
            choice = choice.child(self.button(
                initials.id(),
                initials.label(),
                chosen,
                false,
                move |this, _, cx| this.choose_initials(initials, cx),
                cx,
            ));
        }
        v_flex().gap_1().child(choice).child(text(
            "identity-initials-chosen",
            self.managed.initials.label(),
        ))
    }

    /// The section: the registrations, then the chosen one's identity form
    /// and Remove.
    pub(super) fn managed_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let muted = cx.theme().muted_foreground;
        let rows = self.registration_rows(cx);
        let section = v_flex()
            .gap_2()
            .child(Self::heading("managed-heading", MANAGED_HEADING))
            .child(rows);
        let Some(root) = self.managed.root.clone() else {
            return section;
        };
        let root = root.to_string_lossy().into_owned();
        let choice = self.initials_choice(cx);
        let initials = (self.managed.initials == Initials::Set)
            .then(|| self.field(INITIALS_INPUT, INITIALS_FIELD, cx));
        let remove = wrapped(
            REMOVE_SUBMIT,
            REMOVE,
            &self.focus[REMOVE_SUBMIT],
            &self.scroll,
            Button::new((REMOVE_SUBMIT, 0usize))
                .danger()
                .label(REMOVE)
                .on_click(
                    cx.listener(|this, _: &ClickEvent, window, cx| this.submit_remove(window, cx)),
                ),
        );
        section.child(
            v_flex()
                .gap_2()
                .child(text("managed-root", root))
                .child(Self::heading("identity-heading", IDENTITY_HEADING))
                .child(self.current_identity(cx))
                .child(self.field(NAME_FIELD, super::NAME, cx))
                .child(self.field(EMAIL_FIELD, super::EMAIL, cx))
                .child(choice)
                .children(initials)
                .child(self.button(
                    SUBMIT,
                    SET_IDENTITY,
                    true,
                    false,
                    |this, window, cx| this.submit_identity(window, cx),
                    cx,
                ))
                .child(div().text_xs().text_color(muted).child(REMOVE_NOTE))
                .child(remove),
        )
    }
}
