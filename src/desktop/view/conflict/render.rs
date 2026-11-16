//! The resolve conflict sheet drawn (handoff 2g; Wave 04 Cycle 05b B4 to
//! B6): its title and description on the common frame, in the Kit sheet's
//! title slot; the meta line, note and status, then one card per part,
//! named in words, with Before, Yours and Theirs and its choices, and each
//! body's Result editor; and the Git line, Cancel and the primary action on
//! the common frame, in its footer slot. Text and buttons carry their words
//! as labels, for the tests to read.

use gpui_kit::{
    AnyElement, App, ClickEvent, Context, FontWeight, InteractiveElement, IntoElement,
    ParentElement, Render, StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{
        ActiveTheme, Disableable, Icon, Sizable,
        button::{Button, ButtonVariants},
        h_flex, v_flex,
    },
    div, px,
};

use manyhands::repository::ItemDtoKind;

use super::{
    BodyEditors, CANCEL, DESCRIPTION_REMOTE, KEEP_MINE, LOST, NOT_READ, READ_AGAIN, READING,
    RESULT_NOTE, RESUME, RESUME_LEAD, RETRY, ResolveSheet, SAVE_RESOLUTION, SETTLED, SheetSource,
    description_local, external::REPAIRED,
};
use crate::desktop::{
    assets::icon_path,
    keys::{LeaveResult, RESOLVE_SHEET_CONTEXT, SaveResolution},
    model::{
        conflict::{
            BodyChoice, BodyPart, Choice, ConflictSet, PathConflict, PathResult,
            STANDARD_LAYOUT_NOTE, names::field_name,
        },
        operations::primary_name,
    },
    view::frame::{self, Footer, Header},
};

mod pieces;

use pieces::{
    Lines, body_state, choice_button, columns, field_state, header_row, kind_words, state,
    taken_words,
};
pub(super) use pieces::{card_frame, text};

/// What a part is called in its card: the body is the Description.
const DESCRIPTION: &str = "Description";
const KEEP_THEIRS: &str = "Keep theirs";
const KEEP_BOTH: &str = "Keep both";
const RESULT: &str = "Result";

impl ResolveSheet {
    pub(super) fn on<F>(
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

    /// A footer button's press: as `on`, after focusing the sheet's own
    /// handle, as `LifecycleDialog::on` does, so a footer control that goes
    /// with its press leaves focus in the sheet (Wave 04 Cycle 05b B3).
    fn on_footer<F>(
        &self,
        cx: &Context<Self>,
        f: F,
    ) -> impl Fn(&ClickEvent, &mut Window, &mut App) + use<F>
    where
        F: Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    {
        let entity = cx.entity().downgrade();
        move |_, window, cx| {
            let _ = entity.update(cx, |sheet, cx| {
                window.focus(&sheet.focus, cx);
                f(sheet, window, cx)
            });
        }
    }

    /// The body's first lines (B4): the meta line, with the item's kind,
    /// short code, repository and progress, and the note of what Saving
    /// does.
    fn lead(&self, cx: &App) -> impl IntoElement + use<> {
        let muted = cx.theme().muted_foreground;
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
        v_flex().gap_1().child(meta).child(
            text("conflict-note", self.source.note(&self.primary_branch()))
                .text_sm()
                .text_color(muted),
        )
    }

    /// The primary branch, as the sheet's heading knows it (R-35).
    fn primary_branch(&self) -> String {
        primary_name(self.heading.primary.as_deref()).to_owned()
    }

    /// The footer's Git detail (handoff 2g): the conflicted paths and both
    /// sides' commits, in mono, filling the footer's left and cut short when
    /// long (B6, R-15).
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
            .flex_1()
            .min_w_0()
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
                        // Not while a Save's lookup or request is out (M-1).
                        .disabled(self.waiting())
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
                let line = Lines::default();
                let mut cards = v_flex().gap_3();
                for (index, path) in paths.iter().enumerate() {
                    cards =
                        cards.child(self.path_card(index, path, &mut field, &mut taken, &line, cx));
                }
                cards.into_any_element()
            }
            ConflictSet::External(paths) => self.external_cards(paths, cx),
            // B4: Check again found the repair not yet one the library
            // takes up.
            ConflictSet::Settled if model.external_repair() && !self.resumable() => {
                self.repair_not_done(cx)
            }
            ConflictSet::Settled => {
                let words = match (self.resumable(), self.outside) {
                    (true, true) => REPAIRED,
                    (true, false) => RESUME_LEAD,
                    (false, _) => SETTLED,
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
        line: &Lines,
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
                    format!("{}: {}", field_name(&row.key), taken_words(row.from)),
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
        let kind = kind_words(path.item_kind());
        for (at, part) in path.fields().iter().enumerate() {
            let k = *field;
            *field += 1;
            let chooser = |id: &'static str, label: &'static str, choice: Choice| {
                choice_button((id, k), label, part.choice == Some(choice))
                    .disabled(self.saving())
                    .on_click(self.on(cx, move |sheet, _, cx| {
                        sheet.choose_field(index, at, choice, cx)
                    }))
            };
            // The field's name in words (Part C), its kind and its state.
            let words = [
                text(("conflict-field", k), field_name(&part.key))
                    .font_weight(FontWeight::SEMIBOLD)
                    .into_any_element(),
                text(("conflict-field-kind", k), kind)
                    .text_xs()
                    .text_color(muted)
                    .into_any_element(),
                state(("conflict-choice", k), field_state(part.choice), cx).into_any_element(),
            ];
            card = card.child(
                card_frame(cx)
                    .child(header_row(
                        words,
                        [
                            chooser("conflict-keep-mine", KEEP_MINE, Choice::Mine),
                            chooser("conflict-keep-theirs", KEEP_THEIRS, Choice::Theirs),
                        ],
                    ))
                    .child(columns(
                        part.base.as_deref(),
                        part.lines[0].as_ref(),
                        part.lines[1].as_ref(),
                        line,
                        cx,
                    )),
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

    /// The Result editor in its tab stop (R-31): Tab reaches the stop, which
    /// shows a ring; Enter on it puts focus in the editor, where Tab indents,
    /// Enter is a newline and Ctrl+Enter saves; Escape there comes back to
    /// the stop, and Escape on the stop closes the sheet.
    fn result_stop(
        &self,
        index: usize,
        editors: &BodyEditors,
        cx: &Context<Self>,
    ) -> impl IntoElement + use<> {
        let (border, ring) = (cx.theme().border, cx.theme().ring);
        let (enter, leave) = (editors.result.clone(), editors.stop.clone());
        let stop = editors.stop.clone();
        div()
            .id(("conflict-result", index))
            .test_support()
            .track_focus(&editors.stop)
            .p_2()
            .border_1()
            .border_color(border)
            .rounded(px(4.))
            // The ring changes the colour only, so the editor never moves.
            .focus(move |style| style.border_color(ring))
            .on_click(move |event, window, cx| {
                if event.is_keyboard() && stop.is_focused(window) {
                    enter.update(cx, |editor, cx| editor.focus(window, cx));
                }
            })
            .on_action(move |_: &LeaveResult, window, cx| window.focus(&leave, cx))
            .child(editors.result.clone())
    }

    fn body_card(
        &self,
        index: usize,
        body: &BodyPart,
        editors: &BodyEditors,
        line: &Lines,
        cx: &Context<Self>,
    ) -> impl IntoElement + use<> {
        let choice = body.choice;
        let chooser = |id: &'static str, label: &'static str, kept: BodyChoice| {
            choice_button((id, index), label, choice == Some(kept))
                .disabled(self.saving())
                .on_click(self.on(cx, move |sheet, _, cx| sheet.choose_body(index, kept, cx)))
        };
        // The body is one part, already named Description: no kind (K-14).
        let words = [
            text(("conflict-body", index), DESCRIPTION)
                .font_weight(FontWeight::SEMIBOLD)
                .into_any_element(),
            state(("conflict-body-choice", index), body_state(choice), cx).into_any_element(),
        ];
        card_frame(cx)
            .child(header_row(
                words,
                [
                    chooser("conflict-body-mine", KEEP_MINE, BodyChoice::Mine),
                    chooser("conflict-body-theirs", KEEP_THEIRS, BodyChoice::Theirs),
                    chooser("conflict-body-both", KEEP_BOTH, BodyChoice::Both),
                ],
            ))
            .child(columns(
                Some(&body.base),
                Some(&body.lines[0]),
                Some(&body.lines[1]),
                line,
                cx,
            ))
            .child(div().text_xs().child(RESULT))
            .child(self.result_stop(index, editors, cx))
            .child(
                text(("conflict-result-note", index), RESULT_NOTE)
                    .text_xs()
                    .text_color(cx.theme().muted_foreground),
            )
    }

    /// The footer's last element (B6): exactly one of Resume, Retry, the
    /// lost line, or Save resolution, enabled once every part is chosen;
    /// none for a set with nothing to save here.
    fn primary_action(&self, cx: &Context<Self>) -> Option<AnyElement> {
        if self.resumable() {
            return Some(
                Button::new("conflict-resume")
                    .label(RESUME)
                    .primary()
                    .small()
                    .on_click(self.on_footer(cx, |sheet, window, cx| sheet.resume(window, cx)))
                    .into_any_element(),
            );
        }
        if self.kept.is_some() {
            return Some(
                Button::new("conflict-retry")
                    .label(RETRY)
                    .primary()
                    .small()
                    .disabled(self.waiting())
                    .on_click(self.on_footer(cx, |sheet, window, cx| sheet.save(window, cx)))
                    .into_any_element(),
            );
        }
        if self.lost() {
            return Some(text("conflict-lost", LOST).text_sm().into_any_element());
        }
        let model = self.model.as_ref()?;
        let ConflictSet::Canonical(_) = model.set() else {
            return None;
        };
        let ready = !self.waiting() && model.resolutions().is_some();
        Some(
            Button::new("conflict-save")
                .label(SAVE_RESOLUTION)
                .primary()
                .small()
                .disabled(!ready)
                .on_click(self.on_footer(cx, |sheet, window, cx| sheet.save(window, cx)))
                .into_any_element(),
        )
    }

    /// The Kit sheet's title slot (B4): the title and description on the
    /// common frame, with no tile, inside `conflict-header`, which takes the
    /// row's width beside the Kit's close button (B2).
    pub(super) fn header_slot(&self, cx: &App) -> impl IntoElement + use<> {
        let title = match &self.heading.title {
            Some(title) => format!("Choose what to keep \u{b7} {title}"),
            None => "Choose what to keep".to_owned(),
        };
        let description = match self.source {
            SheetSource::Local => description_local(&self.primary_branch()),
            _ => DESCRIPTION_REMOTE.to_owned(),
        };
        let header = Header {
            icon: None,
            title: title.into(),
            description: Some(description.into()),
            ids: ("conflict-title", Some("conflict-description")),
        };
        div()
            .id("conflict-header")
            .test_support()
            .flex()
            .flex_1()
            .min_w_0()
            .child(frame::sheet_header(header, cx))
    }

    /// The Kit sheet's footer slot (B6, R-15): the Git line filling the
    /// left, Cancel, then the primary action, on the common frame, inside
    /// `conflict-footer`, which tracks the footer's own focus and saves on
    /// Ctrl+Enter from its buttons, as the body does (B2, R-8).
    pub(super) fn footer_slot(&self, cx: &Context<Self>) -> impl IntoElement + use<> {
        let cancel = Button::new("conflict-cancel")
            .label(CANCEL)
            .small()
            .on_click(self.on_footer(cx, |sheet, window, cx| {
                sheet.close(window, cx);
            }));
        let mut buttons: Vec<AnyElement> = self
            .git_line(cx)
            .map(IntoElement::into_any_element)
            .into_iter()
            .collect();
        buttons.push(cancel.into_any_element());
        buttons.extend(self.primary_action(cx));
        v_flex()
            .id("conflict-footer")
            .test_support()
            .track_focus(&self.footer_focus)
            .key_context(RESOLVE_SHEET_CONTEXT)
            .on_action(cx.listener(|sheet, _: &SaveResolution, window, cx| sheet.save(window, cx)))
            .w_full()
            .child(frame::sheet_footer(
                Footer {
                    hint: None,
                    buttons,
                },
                cx,
            ))
    }
}

impl Render for ResolveSheet {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.make_editors(window, cx);
        v_flex()
            .id("conflict-sheet")
            .test_support()
            .track_focus(&self.focus)
            .key_context(RESOLVE_SHEET_CONTEXT)
            .on_action(cx.listener(|sheet, _: &SaveResolution, window, cx| sheet.save(window, cx)))
            .gap_3()
            .size_full()
            .px_4()
            .pb_4()
            .overflow_y_scroll()
            .child(self.lead(cx))
            .child(self.status(cx))
            .child(self.cards(cx))
    }
}
