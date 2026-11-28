//! Create a new repository (Wave 04 Cycle 05a design B8, B10): Name, then
//! Location with its Browse… and note, the line that says where the empty
//! folder will be made (`create-target`), the main branch as a text field
//! ("main" by default, required), the optional prefix, the optional
//! identity and the note.
//!
//! The folder is Location joined with Name by `Path::join` and shown as
//! typed: the sheet does not check that it exists, or is empty (R-13), and
//! the library's preview refuses what cannot be made. Create repository is
//! enabled when Name, Location and Main branch are filled (B10); nothing
//! says what is missing, since the button is disabled instead.

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use gpui_kit::{
    App, Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement,
    Styled, TestSupportExt, Window,
    component::{ActiveTheme, Icon, h_flex, input::Input, tooltip::Tooltip, v_flex},
    div, px,
};

use super::{
    AddRepositorySheet, CREATE_IDENTITY, FOLDER_NAME, Form, IDENTITY_HEADING, LOCATION,
    MAIN_BRANCH, MAIN_BRANCH_HELP, MAIN_BRANCH_TIP, NAME_FIELD, PREFIX, mode::Mode,
    render::remove_note, will_create,
};
use crate::desktop::{
    assets::icon_path,
    services::requests::Intent,
    view::{
        settings::{EMAIL, NAME, field_column, wrapped},
        shell::tokens::chrome_colors,
    },
};

impl AddRepositorySheet {
    /// Create's folder: Location joined with Name, as typed, when both are
    /// filled (R-13).
    fn create_target(&self, cx: &App) -> Option<PathBuf> {
        let (location, name) = (
            self.text(Form::Create.path(), cx),
            self.text(FOLDER_NAME, cx),
        );
        (!location.is_empty() && !name.is_empty()).then(|| PathBuf::from(location).join(name))
    }

    /// B10: Create repository is enabled when Name, Location and Main
    /// branch are filled, and no add of this open is pending (R-27).
    pub(super) fn create_enabled(&self, cx: &App) -> bool {
        !self.adding
            && self.create_target(cx).is_some()
            && !self.text(Form::Create.branch(), cx).is_empty()
    }

    /// Create's intent, when it is enabled: today's `RepoCreate`, its root
    /// Location joined with Name.
    pub(super) fn create_intent(&self, cx: &App) -> Option<Intent> {
        if !self.create_enabled(cx) {
            return None;
        }
        let form = Form::Create;
        let identity = Some((self.text(form.name(), cx), self.text(form.email(), cx)))
            .filter(|(name, email)| !(name.is_empty() && email.is_empty()));
        let prefix = Some(self.text(form.prefix(), cx)).filter(|prefix| !prefix.is_empty());
        Some(Intent::RepoCreate {
            root: self.create_target(cx)?,
            primary_branch: self.text(form.branch(), cx),
            identity,
            slug_prefix: prefix,
        })
    }

    /// Create one here (B6): the new mode, Location the checked folder's
    /// parent and Name its last part, and focus on Name (G20). The library's
    /// preview still refuses a folder that is not empty (R-11).
    pub(super) fn create_here(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let folder = self.inspection.path().unwrap_or_default().to_owned();
        let path = Path::new(&folder);
        let (location, name) = match (
            path.parent().and_then(Path::to_str),
            path.file_name().and_then(OsStr::to_str),
        ) {
            (Some(location), Some(name)) => (location.to_owned(), name.to_owned()),
            // A root has no last part: it is the location.
            _ => (folder.clone(), String::new()),
        };
        self.set_mode(Mode::New, cx);
        self.inputs[Form::Create.path()]
            .update(cx, |input, cx| input.set_value(location, window, cx));
        self.inputs[FOLDER_NAME].update(cx, |input, cx| {
            input.set_value(name, window, cx);
            input.focus(window, cx);
        });
    }

    /// The Main branch text field, with its tooltip and its help (B7, B8
    /// item 4).
    fn create_branch_field(&self, cx: &Context<Self>) -> impl IntoElement + use<> {
        let id = Form::Create.branch();
        let input = Input::new(&self.inputs[id]).aria_label(MAIN_BRANCH);
        field_column()
            .gap_0p5()
            .child(
                div()
                    .id("create-branch-label")
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(MAIN_BRANCH)
                    .tooltip(|window, cx| Tooltip::new(MAIN_BRANCH_TIP).build(window, cx)),
            )
            .child(wrapped(id, MAIN_BRANCH, &self.focus[id], &self.scroll, input).w_full())
            .child(
                div()
                    .text_xs()
                    .text_color(chrome_colors(cx).secondary)
                    .child(MAIN_BRANCH_HELP),
            )
    }

    /// `create-target`: where the empty folder will be made, when Name and
    /// Location are filled (B8 item 3).
    fn create_target_line(&self, cx: &App) -> Option<impl IntoElement + use<>> {
        let target = self.create_target(cx)?;
        let colors = chrome_colors(cx);
        let line = will_create(&target.display().to_string());
        Some(
            h_flex()
                .id("create-target")
                .test_support()
                .aria_label(line.clone())
                .items_start()
                .gap(px(6.))
                .text_size(px(12.))
                .child(
                    Icon::default()
                        .path(icon_path("circle-check"))
                        .text_color(colors.green)
                        .w(px(13.))
                        .h(px(13.)),
                )
                .child(div().flex_1().min_w_0().child(line)),
        )
    }

    /// Create a new repository (B8): Name, Location, where, Main branch, the
    /// prefix, the optional identity and the note.
    pub(super) fn create_form(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let muted = cx.theme().muted_foreground;
        let identity = v_flex()
            .id("create-identity")
            .test_support()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .child(IDENTITY_HEADING),
            )
            .child(div().text_xs().text_color(muted).child(CREATE_IDENTITY))
            .child(self.field(Form::Create.name(), NAME, cx))
            .child(self.field(Form::Create.email(), EMAIL, cx));
        v_flex()
            .gap_3()
            .child(self.field(FOLDER_NAME, NAME_FIELD, cx))
            .child(self.folder(Form::Create, LOCATION, None, cx))
            .children(self.create_target_line(cx))
            .child(self.create_branch_field(cx))
            .child(self.field(Form::Create.prefix(), PREFIX, cx))
            .child(identity)
            .child(remove_note("create-note", cx))
    }
}
