//! Browse…, for both forms: the platform's chooser for one directory. A
//! dismissal leaves the field as it was; a platform failure disables
//! Browse, with why, and the field stays usable; a choice that is not text
//! says so (fact 14).

use std::path::PathBuf;

use gpui_kit::{Context, PathPromptOptions, Window};

use super::{AddRepositorySheet, BROWSE_NOT_TEXT, BROWSE_PROMPT, BROWSE_UNAVAILABLE, Form};

impl AddRepositorySheet {
    /// Browse: asks the platform for one directory.
    pub(super) fn browse(&mut self, form: Form, window: &mut Window, cx: &mut Context<Self>) {
        if self.browsing.is_some() || self.browse_unavailable.is_some() {
            return;
        }
        let answer = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(BROWSE_PROMPT.into()),
        });
        self.browsing = Some(cx.spawn_in(window, async move |this, cx| {
            let answer = answer.await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.browsing = None;
                match answer {
                    Ok(Ok(Some(paths))) => this.chosen(form, paths, window, cx),
                    // Dismissed: the field stays as it was.
                    Ok(Ok(None)) => {}
                    Ok(Err(_)) | Err(_) => this.browse_unavailable = Some(BROWSE_UNAVAILABLE),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    /// Browse chose `paths`: the first fills the form's folder, and the
    /// Add folder is checked.
    fn chosen(
        &mut self,
        form: Form,
        paths: Vec<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(path) = paths.into_iter().next() else {
            return;
        };
        let Some(text) = path.to_str() else {
            self.messages.insert(form, BROWSE_NOT_TEXT);
            return;
        };
        let text = text.to_owned();
        self.messages.remove(&form);
        self.inputs[form.path()].update(cx, |input, cx| input.set_value(text, window, cx));
        if form == Form::Add {
            self.inspect(cx);
        }
    }
}
