//! The resolve conflict sheet drawn (handoff 2g): title, note and progress;
//! one card per part with Before, Yours and Theirs and its choices; each
//! body's Result editor; and Cancel · Save resolution. Text and buttons
//! carry their words as labels, for the tests to read.

use gpui_kit::{
    AnyElement, App, ClickEvent, Context, FontWeight, InteractiveElement, IntoElement,
    ParentElement, Render, StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{
        ActiveTheme, Disableable, Icon, Selectable, Sizable, WindowExt,
        button::{Button, ButtonVariants},
        h_flex, v_flex,
    },
    div, px,
};

use std::cell::Cell;

use manyhands::repository::ItemDtoKind;

use super::{
    BodyEditors, CANCEL, DESCRIPTION_LOCAL, DESCRIPTION_REMOTE, LOST, NOT_READ, READ_AGAIN,
    READING, RESULT_NOTE, RESUME, RESUME_LEAD, RETRY, ResolveSheet, SAVE_RESOLUTION, SETTLED,
    SheetSource,
};
use crate::desktop::{
    assets::icon_path,
    keys::{RESOLVE_SHEET_CONTEXT, SaveResolution},
    model::conflict::{
        BodyChoice, BodyPart, Choice, ConflictSet, PathConflict, PathResult, STANDARD_LAYOUT_NOTE,
    },
};

mod pieces;

use pieces::{body_state, column, field_state, state, taken_words, text};

/// What a part is called in its card: the body is the Description.
const DESCRIPTION: &str = "Description";
const KEEP_MINE: &str = "Keep mine";
const KEEP_THEIRS: &str = "Keep theirs";
const KEEP_BOTH: &str = "Keep both";
const BEFORE: &str = "Before";
const YOURS: &str = "Yours";
const THEIRS: &str = "Theirs";
const RESULT: &str = "Result";
/// An external set's card, until Task 12's guidance.
const OUTSIDE: &str = "This file needs fixing outside Manyhands";

impl ResolveSheet {
    fn on<F>(
        &self,
        cx: &Context<Self>,
        f: F,
    ) -> impl Fn(&ClickEvent, &mut Window, &mut App) + use<F>
    where
        F: Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    {
        let entity = cx.entity().downgrade();
        move |_, window, cx| {
            let _ = entity.update(cx, |sheet, cx| f(sheet, window, cx));
        }
    }

    fn header(&self, cx: &App) -> impl IntoElement + use<> {
        let muted = cx.theme().muted_foreground;
        let title = match &self.heading.title {
            Some(title) => format!("Choose what to keep \u{b7} {title}"),
            None => "Choose what to keep".to_owned(),
        };
        // Amber while parts wait for a choice, green once all are chosen.
        let progress = self.model.as_ref().and_then(|model| {
            matches!(model.set(), ConflictSet::Canonical(_)).then(|| {
                let (chosen, of) = model.chosen();
                let color = if chosen < of {
                    cx.theme().warning
                } else {
                    cx.theme().success
                };
                text("conflict-progress", format!("{chosen} of {of} chosen"))
                    .text_sm()
                    .text_color(color)
            })
        });
        let (icon, kind) = match self.heading.kind {
            ItemDtoKind::Ticket => ("ticket", "ticket"),
            ItemDtoKind::Document => ("file-text", "document"),
        };
        let description = match self.source {
            SheetSource::Local => DESCRIPTION_LOCAL,
            _ => DESCRIPTION_REMOTE,
        };
        let meta = h_flex()
            .gap_2()
            .items_center()
            .child(
                div()
                    .id("conflict-kind")
                    .test_support()
                    .aria_label(kind)
                    .child(Icon::default().path(icon_path(icon)).w(px(16.)).h(px(16.))),
            )
            .child(
                text("conflict-short", self.heading.short.clone())
                    .text_sm()
                    .font_family("monospace"),
            )
            .child(
                text("conflict-repository", self.heading.repository.clone())
                    .text_sm()
                    .text_color(muted),
            )
            .children(progress);
        v_flex()
            .gap_1()
            .child(text("conflict-title", title).font_weight(FontWeight::SEMIBOLD))
            .child(text("conflict-description", description).text_sm())
            .child(meta)
            .child(
                text("conflict-note", self.source.note())
                    .text_sm()
                    .text_color(muted),
            )
    }

    /// The footer's Git detail (handoff 2g): the conflicted paths and both
    /// sides' commits, in mono, cut short when long.
    fn git_line(&self, cx: &App) -> Option<impl IntoElement + use<>> {
        let model = self.model.as_ref()?;
        let paths: Vec<&str> = match model.set() {
            ConflictSet::Canonical(paths) => paths.iter().map(PathConflict::display_path).collect(),
            ConflictSet::External(paths) => paths
                .iter()
                .map(|path| path.display_path.as_str())
                .collect(),
            ConflictSet::Settled => return None,
        };
        let (ours, theirs) = model.parents();
        let short = |oid: &str| oid.chars().take(8).collect::<String>();
        Some(
            text(
                "conflict-git",
                format!(
                    "Git: conflict in {} \u{b7} ours {} \u{b7} theirs {}",
                    paths.join(", "),
                    short(ours),
                    short(theirs),
                ),
            )
            .text_xs()
            .font_family("monospace")
            .text_color(cx.theme().muted_foreground)
            .truncate(),
        )
    }

    /// The lines that say what happened: a notice, a store's error, an
    /// answer, a failed read.
    fn status(&self, cx: &Context<Self>) -> impl IntoElement + use<> {
        let muted = cx.theme().muted_foreground;
        let answer = self.answer.as_ref().map(|(code, message)| {
            v_flex()
                .child(text("conflict-message", message.clone()).text_sm())
                .child(
                    text("conflict-code", code.as_str())
                        .text_xs()
                        .text_color(muted),
                )
        });
        let not_read = self.not_read.map(|code| {
            h_flex()
                .gap_2()
                .items_center()
                .child(text("conflict-read-failed", NOT_READ).text_sm())
                .child(
                    text("conflict-read-code", code.as_str())
                        .text_xs()
                        .text_color(muted),
                )
                .child(
                    Button::new("conflict-read-again")
                        .label(READ_AGAIN)
                        .small()
                        .on_click(self.on(cx, |sheet, _, cx| sheet.read_conflict(cx))),
                )
        });
        v_flex()
            .gap_1()
            .children(
                self.notice
                    .map(|notice| text("conflict-notice", notice).text_sm()),
            )
            .children(
                self.error
                    .clone()
                    .map(|error| text("conflict-error", error).text_sm()),
            )
            .children(answer)
            .children(not_read)
    }

    /// The model's cards.
    fn cards(&self, cx: &Context<Self>) -> AnyElement {
        let Some(model) = &self.model else {
            let words = if self.reading { READING } else { "" };
            return text("conflict-state", words).text_sm().into_any_element();
        };
        match model.set() {
            ConflictSet::Canonical(paths) => {
                let mut field = 0;
                let mut taken = 0;
                let line = Cell::new(0);
                let mut cards = v_flex().gap_3();
                for (index, path) in paths.iter().enumerate() {
                    cards =
                        cards.child(self.path_card(index, path, &mut field, &mut taken, &line, cx));
                }
                cards.into_any_element()
            }
            ConflictSet::External(paths) => v_flex()
                .gap_2()
                .children(paths.iter().enumerate().map(|(index, path)| {
                    text(
                        ("conflict-external", index),
                        format!("{}: {OUTSIDE}", path.display_path),
                    )
                    .text_sm()
                }))
                .into_any_element(),
            ConflictSet::Settled => {
                let words = if self.resumable() {
                    RESUME_LEAD
                } else {
                    SETTLED
                };
                text("conflict-state", words).text_sm().into_any_element()
            }
        }
    }

    fn path_card(
        &self,
        index: usize,
        path: &PathConflict,
        field: &mut usize,
        taken: &mut usize,
        line: &Cell<usize>,
        cx: &Context<Self>,
    ) -> impl IntoElement + use<> {
        let muted = cx.theme().muted_foreground;
        let mut card = v_flex().gap_2().child(
            text(("conflict-path", index), path.display_path())
                .text_xs()
                .font_family("monospace")
                .text_color(muted),
        );
        for row in path.taken() {
            card = card.child(
                text(
                    ("conflict-taken", *taken),
                    format!("{}: {}", row.key, taken_words(row.from)),
                )
                .text_sm()
                .text_color(muted),
            );
            *taken += 1;
        }
        if let Some(from) = path.body_taken() {
            card = card.child(
                text(
                    ("conflict-body-taken", index),
                    format!("{DESCRIPTION}: {}", taken_words(from)),
                )
                .text_sm()
                .text_color(muted),
            );
        }
        for (at, part) in path.fields().iter().enumerate() {
            let k = *field;
            *field += 1;
            let chooser = |id: &'static str, label: &'static str, choice: Choice| {
                Button::new((id, k))
                    .label(label)
                    .small()
                    .selected(part.choice == Some(choice))
                    .on_click(self.on(cx, move |sheet, _, cx| {
                        sheet.choose_field(index, at, choice, cx)
                    }))
            };
            card = card.child(
                v_flex()
                    .gap_1()
                    .p_2()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(px(6.))
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                text(("conflict-field", k), part.key.clone())
                                    .font_weight(FontWeight::SEMIBOLD),
                            )
                            .child(chooser("conflict-keep-mine", KEEP_MINE, Choice::Mine))
                            .child(chooser("conflict-keep-theirs", KEEP_THEIRS, Choice::Theirs))
                            .child(state(("conflict-choice", k), field_state(part.choice), cx)),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(column(BEFORE, &part.base, None, line, cx))
                            .child(column(YOURS, &part.mine, Some(&part.base), line, cx))
                            .child(column(THEIRS, &part.theirs, Some(&part.base), line, cx)),
                    ),
            );
        }
        if let (Some(body), Some(editors)) = (
            path.body(),
            self.editors.iter().find(|editors| editors.path == index),
        ) {
            card = card.child(self.body_card(index, body, editors, line, cx));
        }
        if let PathResult::Ready {
            normalized: true, ..
        } = path.result()
        {
            card = card.child(
                text(("conflict-layout", index), STANDARD_LAYOUT_NOTE)
                    .text_xs()
                    .text_color(muted),
            );
        }
        card
    }

    fn body_card(
        &self,
        index: usize,
        body: &BodyPart,
        editors: &BodyEditors,
        line: &Cell<usize>,
        cx: &Context<Self>,
    ) -> impl IntoElement + use<> {
        let border = cx.theme().border;
        let choice = body.choice;
        let chooser = |id: &'static str, label: &'static str, kept: BodyChoice| {
            Button::new((id, index))
                .label(label)
                .small()
                .selected(choice == Some(kept))
                .on_click(self.on(cx, move |sheet, _, cx| sheet.choose_body(index, kept, cx)))
        };
        let base = Some(body.base.clone());
        v_flex()
            .gap_1()
            .p_2()
            .border_1()
            .border_color(border)
            .rounded(px(6.))
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        text(("conflict-body", index), DESCRIPTION)
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .child(chooser("conflict-body-mine", KEEP_MINE, BodyChoice::Mine))
                    .child(chooser(
                        "conflict-body-theirs",
                        KEEP_THEIRS,
                        BodyChoice::Theirs,
                    ))
                    .child(chooser("conflict-body-both", KEEP_BOTH, BodyChoice::Both))
                    .child(state(
                        ("conflict-body-choice", index),
                        body_state(choice),
                        cx,
                    )),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(column(BEFORE, &base, None, line, cx))
                    .child(column(
                        YOURS,
                        &Some(body.mine.clone()),
                        Some(&base),
                        line,
                        cx,
                    ))
                    .child(column(
                        THEIRS,
                        &Some(body.theirs.clone()),
                        Some(&base),
                        line,
                        cx,
                    )),
            )
            .child(div().text_xs().child(RESULT))
            .child(
                div()
                    .id(("conflict-result", index))
                    .test_support()
                    .p_2()
                    .border_1()
                    .border_color(border)
                    .rounded(px(4.))
                    .child(editors.result.clone()),
            )
            .child(
                text(("conflict-result-note", index), RESULT_NOTE)
                    .text_xs()
                    .text_color(cx.theme().muted_foreground),
            )
    }

    /// Resume, Retry or Save resolution, and Cancel.
    fn buttons(&self, cx: &Context<Self>) -> impl IntoElement + use<> {
        let origin = self.origin.clone();
        let mut row = h_flex().gap_2().justify_end();
        if self.resumable() {
            row = row.child(
                Button::new("conflict-resume")
                    .label(RESUME)
                    .primary()
                    .small()
                    .on_click(self.on(cx, |sheet, window, cx| sheet.resume(window, cx))),
            );
        } else if self.kept.is_some() {
            row = row.child(
                Button::new("conflict-retry")
                    .label(RETRY)
                    .primary()
                    .small()
                    .disabled(self.waiting())
                    .on_click(self.on(cx, |sheet, _, cx| sheet.save(cx))),
            );
        } else if self.lost() {
            row = row.child(text("conflict-lost", LOST).text_sm());
        } else if matches!(
            self.model.as_ref().map(|model| model.set()),
            Some(ConflictSet::Canonical(_))
        ) {
            let ready = !self.waiting()
                && self
                    .model
                    .as_ref()
                    .is_some_and(|model| model.resolutions().is_some());
            row = row.child(
                Button::new("conflict-save")
                    .label(SAVE_RESOLUTION)
                    .primary()
                    .small()
                    .disabled(!ready)
                    .on_click(self.on(cx, |sheet, _, cx| sheet.save(cx))),
            );
        }
        row.child(
            Button::new("conflict-cancel")
                .label(CANCEL)
                .small()
                .on_click(move |_, window, cx| {
                    window.close_dialog(cx);
                    window.focus(&origin, cx);
                }),
        )
    }
}

impl Render for ResolveSheet {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.make_editors(window, cx);
        v_flex()
            .id("conflict-sheet")
            .test_support()
            .key_context(RESOLVE_SHEET_CONTEXT)
            .on_action(cx.listener(|sheet, _: &SaveResolution, _, cx| sheet.save(cx)))
            .gap_3()
            .max_h(px(720.))
            .overflow_y_scroll()
            .child(self.header(cx))
            .child(self.status(cx))
            .child(self.cards(cx))
            .children(self.git_line(cx))
            .child(self.buttons(cx))
    }
}
