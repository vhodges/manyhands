//! Browse…, for both forms: the platform's chooser for one directory, and
//! the note under it (Wave 04 Cycle 05a design B6 item 1; Part G items 1
//! and 2). While the chooser is open Browse… is disabled and the note says
//! "Choosing a folder…"; a dismissal leaves the field as it was; after a
//! platform failure Browse… is disabled for the session, the note saying
//! why, and the field stays usable; a choice that is not text leaves
//! Browse… enabled and the note says so, until the next Browse….
//!
//! Choose another (B6) runs the same Browse… for the Add folder. Its button
//! goes with the check of the folder chosen, so focus that was still on it
//! goes to the folder field (Ruling 22): never when the sheet has closed,
//! a dialog is over it, or focus has moved on meanwhile.

use std::path::PathBuf;

use gpui_kit::{Context, IntoElement, PathPromptOptions, Window, component::WindowExt};

use super::{
    AddRepositorySheet, BROWSE_NOT_TEXT, BROWSE_PROMPT, BROWSE_UNAVAILABLE, CHOOSING, Form,
    render::note,
};

/// What the platform's chooser answered.
enum Answer {
    Chosen(Vec<PathBuf>),
    Dismissed,
    Failed,
}

impl AddRepositorySheet {
    /// Browse… is disabled: the chooser is open, or the platform failed.
    pub(super) fn browse_disabled(&self) -> bool {
        self.browsing.is_some() || self.browse_unavailable.is_some()
    }

    /// Browse: asks the platform for one directory.
    pub(super) fn browse(&mut self, form: Form, window: &mut Window, cx: &mut Context<Self>) {
        self.prompt(form, false, window, cx);
    }

    /// Choose another's Browse… for the Add folder (B6): the folder chosen
    /// takes focus from the action (Ruling 22).
    pub(super) fn choose_another(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.prompt(Form::Add, true, window, cx);
    }

    /// Asks the platform for one directory for `form`; `from_action` when
    /// Choose another asked.
    fn prompt(
        &mut self,
        form: Form,
        from_action: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.browse_disabled() {
            return;
        }
        self.browse_not_text = None;
        let answer = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(BROWSE_PROMPT.into()),
        });
        self.browsing = Some(cx.spawn_in(window, async move |this, cx| {
            let answer = match answer.await {
                Ok(Ok(Some(paths))) => Answer::Chosen(paths),
                Ok(Ok(None)) => Answer::Dismissed,
                Ok(Err(_)) | Err(_) => Answer::Failed,
            };
            let _ = this.update_in(cx, |this, window, cx| {
                this.answered(form, from_action, answer, window, cx)
            });
        }));
        cx.notify();
    }

    fn answered(
        &mut self,
        form: Form,
        from_action: bool,
        answer: Answer,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.browsing = None;
        match answer {
            Answer::Chosen(paths) => self.chosen(form, from_action, paths, window, cx),
            // Dismissed: the field stays as it was.
            Answer::Dismissed => {}
            Answer::Failed => self.browse_unavailable = Some(BROWSE_UNAVAILABLE),
        }
        cx.notify();
    }

    /// Browse chose `paths`: the first fills the form's folder, and the
    /// Add folder is checked. Asked by Choose another, focus still on it in
    /// the open sheet goes to the folder (Ruling 22).
    fn chosen(
        &mut self,
        form: Form,
        from_action: bool,
        paths: Vec<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(path) = paths.into_iter().next() else {
            return;
        };
        let Some(text) = path.to_str() else {
            self.browse_not_text = Some(form);
            return;
        };
        let text = text.to_owned();
        self.inputs[form.path()].update(cx, |input, cx| input.set_value(text, window, cx));
        if form == Form::Add {
            self.inspect(cx);
        }
        let on_action = self.choose_focus().is_focused(window);
        if from_action
            && on_action
            && self.open.is_open(window, cx)
            && !window.has_active_dialog(cx)
        {
            self.focus_path(window, cx);
        }
    }

    /// The note under `form`'s Browse…, when it has one to say.
    pub(super) fn browse_note(&self, form: Form) -> Option<impl IntoElement + use<>> {
        let why = if self.browsing.is_some() {
            CHOOSING
        } else if let Some(why) = self.browse_unavailable {
            why
        } else if self.browse_not_text == Some(form) {
            BROWSE_NOT_TEXT
        } else {
            return None;
        };
        Some(note(form.browse_note(), why))
    }

    /// The platform's chooser answered with a failure, which the test
    /// platform cannot give (P9, P21).
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn browse_failed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.answered(Form::Add, false, Answer::Failed, window, cx);
    }
}
