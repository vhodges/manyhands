//! The lifecycle dialog's confirm stage (D5 design, A2): the lead, the step
//! list, the whole branch's changed paths with their tags, the code note,
//! the Git line and the expiry; Confirm, by click or Enter; and a consent the
//! library refuses as stale.
//!
//! - **Steps** (handoff 2c, R-8): Save your latest edits; Share with the
//!   remote, or "Keep on this computer (sharing pending)" when publication
//!   is `pending_no_remote`; Mark as closed (closure only, "Closed by
//!   {closed_by}" from the preview, or "Already marked closed"); Merge into
//!   main, or "Already in main"; Share main (omitted when local-only); Tidy
//!   up, naming what `cleanup` removes.
//! - **Paths.** One row per `changes[]` entry: its change, the path and a
//!   tag from `item_path_kind`, else `code` (R-2). Any `code` row adds the
//!   code note. No line counts (R-3).
//! - **Stale (contract 3).** The first Confirm sends the run once with the
//!   preview's confirmation. When the library refuses that consent (an
//!   answer `offers_prepare_again` takes: a stale confirmation, or
//!   `external_change` offering `request.prepare`), nothing ran: the
//!   dialog says "Something changed since you confirmed" and offers Prepare
//!   again, which shows the new summary. Confirm then queues the same
//!   request again with the new confirmation (`Worker::reconfirm`) (P2, P3).
//!
//! The summary holds paths and `closed_by`; its types derive no `Debug`.

use std::path::Path;

use gpui_kit::{
    AnyElement, App, Context, FontWeight, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{ActiveTheme, Icon, h_flex, v_flex},
    div, px,
};
use manyhands::{
    canonical::{ItemKind, item_path_kind},
    repository::{
        ItemDtoKind, LifecyclePathChangeKind, LifecyclePlannedCleanupDto,
        LifecyclePlannedIntegration, LifecyclePlannedPublication, LifecyclePreviewDto,
        LifecycleRecording, PreviewDetailsDto, PreviewDto,
    },
    results::ResultCode,
};

use super::{
    LifecycleDialog, Phase,
    outcome::{StepKey, stage_step},
    text,
};
use crate::desktop::{
    assets::icon_path,
    services::requests::{Lane, RequestState},
    view::confirm::{NOT_QUEUED, local_time, offers_prepare_again},
};

/// A refused consent's words (A2, Stale).
pub(crate) const STALE: &str = "Something changed since you confirmed.";
pub(crate) const SAVE_STEP: &str = "Save your latest edits";
/// Step 2 for a repository with no publication remote.
pub(crate) const KEEP_LOCAL: &str = "Keep on this computer (sharing pending)";
pub(crate) const MARK_CLOSED: &str = "Mark as closed";
pub(crate) const ALREADY_MARKED: &str = "Already marked closed";
pub(crate) const TIDY_UP: &str = "Tidy up";
/// The changed paths' list scrolls past this height.
const PATHS_MAX_HEIGHT: f32 = 180.;

/// One step of the list: which it is, its title and what it does. It may
/// name who closes the ticket, and derives no `Debug`.
pub(crate) struct Step {
    pub key: StepKey,
    pub title: String,
    pub note: String,
}

impl Step {
    pub(super) fn new(key: StepKey, title: impl Into<String>, note: impl Into<String>) -> Self {
        Self {
            key,
            title: title.into(),
            note: note.into(),
        }
    }
}

fn item_word(kind: ItemDtoKind) -> &'static str {
    match kind {
        ItemDtoKind::Ticket => "ticket",
        ItemDtoKind::Document => "document",
    }
}

/// The steps the preview plans, in the library's order (R-8): a closure is
/// recorded before the merge.
pub(crate) fn steps(details: &LifecyclePreviewDto) -> Vec<Step> {
    let primary = details.primary_branch.as_str();
    // The remote, where the work is shared: the library names one with
    // every `publish` publication, and none with `pending_no_remote`.
    let sharing = details
        .remote
        .as_deref()
        .filter(|_| details.publication == LifecyclePlannedPublication::Publish);
    let mut steps = vec![Step::new(
        StepKey::Save,
        SAVE_STEP,
        "Anything unsaved in this tab is saved first.",
    )];
    steps.push(match sharing {
        Some(remote) => Step::new(
            StepKey::ShareContext,
            format!("Share with {remote}"),
            "Bring in other people's changes, then send yours.",
        ),
        None => Step::new(
            StepKey::ShareContext,
            KEEP_LOCAL,
            "This repository has no remote, so nothing is sent.",
        ),
    });
    if let Some(closure) = &details.closure {
        let title = match closure.recording {
            LifecycleRecording::Record => MARK_CLOSED,
            LifecycleRecording::Existing => ALREADY_MARKED,
        };
        steps.push(Step::new(
            StepKey::MarkClosed,
            title,
            format!("Closed by {}", closure.closed_by),
        ));
    }
    steps.push(match details.integration {
        LifecyclePlannedIntegration::MergeCommit => Step::new(
            StepKey::Merge,
            format!("Merge into {primary}"),
            format!("The files below become part of {primary}."),
        ),
        LifecyclePlannedIntegration::AlreadyContained => Step::new(
            StepKey::Merge,
            format!("Already in {primary}"),
            format!("{primary} already holds every change below."),
        ),
    });
    if let Some(remote) = sharing {
        steps.push(Step::new(
            StepKey::SharePrimary,
            format!("Share {primary} with {remote}"),
            format!("Send {primary} to {remote}."),
        ));
    }
    steps.push(Step::new(
        StepKey::TidyUp,
        TIDY_UP,
        tidy_note(&details.cleanup, details.remote.as_deref()),
    ));
    steps
}

/// What the tidy-up removes, in words: the remote branch by its remote's
/// name, when the preview names one.
pub(super) fn tidy_note(cleanup: &LifecyclePlannedCleanupDto, remote: Option<&str>) -> String {
    let on_remote = match remote {
        Some(remote) => format!("the branch on {remote}"),
        None => "the remote branch".to_owned(),
    };
    let parts: Vec<&str> = [
        (cleanup.worktree, "the working copy"),
        (cleanup.local_branch, "the local branch"),
        (cleanup.remote_branch, on_remote.as_str()),
    ]
    .into_iter()
    .filter_map(|(planned, words)| planned.then_some(words))
    .collect();
    match parts.as_slice() {
        [] => "Nothing is left to remove.".to_owned(),
        [one] => format!("Remove {one}."),
        [rest @ .., last] => format!("Remove {} and {last}.", rest.join(", ")),
    }
}

/// The Git line: what Git is asked to do, in its own terms.
pub(crate) fn git_line(details: &LifecyclePreviewDto) -> String {
    let (branch, primary) = (&details.branch, &details.primary_branch);
    let mut parts = vec![match details.integration {
        LifecyclePlannedIntegration::MergeCommit => format!("merge {branch} into {primary}"),
        LifecyclePlannedIntegration::AlreadyContained => {
            format!("{branch} already in {primary}")
        }
    }];
    if details.publication == LifecyclePlannedPublication::Publish
        && let Some(remote) = &details.remote
    {
        parts.push(format!("push {remote}"));
    }
    let cleanup = &details.cleanup;
    let deleted: Vec<&str> = [
        (cleanup.worktree, "worktree"),
        (cleanup.local_branch, "local branch"),
        (cleanup.remote_branch, "remote branch"),
    ]
    .into_iter()
    .filter_map(|(planned, name)| planned.then_some(name))
    .collect();
    if !deleted.is_empty() {
        parts.push(format!("delete {}", deleted.join(", ")));
    }
    format!("Git: {}", parts.join(" \u{b7} "))
}

/// A changed path's tag, by its path alone (R-2).
pub(crate) fn path_tag(path: &str) -> &'static str {
    match item_path_kind(Path::new(path)) {
        Ok(ItemKind::Ticket) => "ticket",
        Ok(ItemKind::Document) => "document",
        Ok(ItemKind::Comment) => "comment",
        Err(_) => "code",
    }
}

/// The note a branch that holds code adds.
pub(crate) fn code_note(kind: ItemDtoKind) -> String {
    format!(
        "This working copy includes code as well as the {}. All of it is merged.",
        item_word(kind)
    )
}

pub(super) fn change_words(change: LifecyclePathChangeKind) -> (&'static str, &'static str) {
    match change {
        LifecyclePathChangeKind::Added => ("added", "plus"),
        LifecyclePathChangeKind::Modified => ("modified", "pencil"),
        LifecyclePathChangeKind::Deleted => ("deleted", "minus"),
    }
}

impl LifecycleDialog {
    /// Confirm: the first run, sent once with the preview's confirmation;
    /// after Prepare again, the same request queued again with the new one.
    /// Anything but a held preview does nothing.
    pub(crate) fn confirm(&mut self, cx: &mut Context<Self>) {
        let Phase::Confirm {
            confirmation,
            preview,
        } = &self.phase
        else {
            return;
        };
        let confirmation = *confirmation;
        self.plan = match &preview.details {
            PreviewDetailsDto::Lifecycle(details) => Some(details.clone()),
            _ => None,
        };
        self.following_anew();
        let worker = self.services.worker();
        let (request, attempt) = match self.confirmed {
            None => (
                worker.submit(self.intent.clone(), Lane::User, Some(confirmation), None),
                1,
            ),
            Some((request, _)) => match worker.reconfirm(request, confirmation) {
                Some(attempt) => (request, attempt),
                None => {
                    self.phase = Phase::Message(NOT_QUEUED);
                    cx.notify();
                    return;
                }
            },
        };
        self.confirmed = Some((request, attempt));
        self.phase = Phase::Submitted(request);
        cx.notify();
    }

    /// Enter in the dialog: Confirm on the confirm stage, Prepare again on
    /// a refused consent, and on a sent run what its outcome takes
    /// (`enter_submitted`); nothing elsewhere.
    pub(crate) fn enter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.phase {
            Phase::Confirm { .. } => self.confirm(cx),
            Phase::Stale { .. } => self.prepare(cx),
            Phase::Submitted(_) => self.enter_submitted(window, cx),
            _ => {}
        }
    }

    /// The shown run moved on: its stage lights its step (L3), and the
    /// dialog draws it again. An answer to the confirmed request that
    /// refuses the consent shows Stale, with Prepare again.
    pub(super) fn operations_changed(&mut self, cx: &mut Context<Self>) {
        let Phase::Submitted(shown) = self.phase else {
            return;
        };
        let stage = match self
            .services
            .operations()
            .read(cx)
            .request(shown)
            .map(|request| &request.state)
        {
            Some(RequestState::Running {
                stage: Some(stage), ..
            }) => stage_step(*stage),
            _ => None,
        };
        // Stages come in order: the latest seen is the furthest.
        self.reached = self.reached.max(stage);
        cx.notify();
        let Some((request, attempt)) = self.confirmed else {
            return;
        };
        if shown != request {
            return;
        }
        let stale = {
            let operations = self.services.operations().read(cx);
            match operations.request(request) {
                // The model holds an earlier attempt until this one's
                // `Queued` event arrives.
                Some(asked) if asked.attempt == attempt => match &asked.state {
                    RequestState::Finished(_, code, _, recovery, message)
                        if offers_prepare_again(*code, recovery) =>
                    {
                        Some((*code, message.clone()))
                    }
                    _ => None,
                },
                _ => None,
            }
        };
        if let Some((code, message)) = stale {
            self.phase = Phase::Stale { code, message };
            cx.notify();
        }
    }

    /// The confirm stage: lead, steps, changed paths, code note, Git line
    /// and expiry.
    pub(super) fn confirm_body(&self, preview: &PreviewDto, cx: &App) -> AnyElement {
        let theme = cx.theme();
        let (muted, border, mono, info) = (
            theme.muted_foreground,
            theme.border,
            theme.mono_font_family.clone(),
            theme.info,
        );
        let lead = match self.heading.kind {
            ItemDtoKind::Ticket => {
                "Closing runs these steps in order. If one fails, it stops there and nothing after it happens."
            }
            ItemDtoKind::Document => {
                "Merging runs these steps in order. If one fails, it stops there and nothing after it happens."
            }
        };
        let mut body = v_flex()
            .gap_2()
            .child(text("lifecycle-lead", lead).text_sm());
        if let PreviewDetailsDto::Lifecycle(details) = &preview.details {
            let list = v_flex()
                .gap_2()
                .p_2()
                .border_1()
                .border_color(border)
                .rounded_md()
                .children(steps(details).into_iter().enumerate().map(|(ix, step)| {
                    h_flex()
                        .gap_2()
                        .items_start()
                        .child(
                            div()
                                .flex_none()
                                .w(px(20.))
                                .h(px(20.))
                                .rounded_full()
                                .border_1()
                                .border_color(border)
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_xs()
                                .child((ix + 1).to_string()),
                        )
                        .child(
                            v_flex()
                                .min_w_0()
                                .child(
                                    text(("lifecycle-step", ix), step.title)
                                        .text_sm()
                                        .font_weight(FontWeight::SEMIBOLD),
                                )
                                .child(
                                    text(("lifecycle-step-note", ix), step.note)
                                        .text_xs()
                                        .text_color(muted),
                                ),
                        )
                }));
            let code = details
                .changes
                .iter()
                .any(|change| path_tag(&change.path) == "code");
            let paths = v_flex()
                .gap_1()
                .child(
                    text(
                        "lifecycle-paths-heading",
                        format!(
                            "Files that will be merged into {} ({})",
                            details.primary_branch,
                            details.changes.len()
                        ),
                    )
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD),
                )
                .child(
                    // Its own scroll, so Confirm and Cancel stay in view on
                    // a long branch.
                    v_flex()
                        .id("lifecycle-paths")
                        .gap_1()
                        .max_h(px(PATHS_MAX_HEIGHT))
                        .overflow_y_scroll()
                        .children(details.changes.iter().enumerate().map(|(ix, change)| {
                            let (words, icon) = change_words(change.change);
                            let tag = path_tag(&change.path);
                            h_flex()
                                .id(("lifecycle-path-row", ix))
                                .test_support()
                                .gap_2()
                                .items_center()
                                .child(
                                    div()
                                        .id(("lifecycle-path-change", ix))
                                        .test_support()
                                        .aria_label(words)
                                        .flex_none()
                                        .child(
                                            Icon::default()
                                                .path(icon_path(icon))
                                                .w(px(14.))
                                                .h(px(14.)),
                                        ),
                                )
                                .child(
                                    text(("lifecycle-path", ix), change.path.clone())
                                        .min_w_0()
                                        .text_xs()
                                        .font_family(mono.clone()),
                                )
                                .child(
                                    text(("lifecycle-path-tag", ix), tag)
                                        .flex_none()
                                        .text_xs()
                                        .text_color(if tag == "code" { info } else { muted }),
                                )
                        })),
                );
            body =
                body.child(list)
                    .child(paths)
                    .children(code.then(|| {
                        text("lifecycle-code-note", code_note(self.heading.kind)).text_sm()
                    }))
                    .child(
                        text("lifecycle-git", git_line(details))
                            .text_xs()
                            .font_family(mono)
                            .text_color(muted),
                    );
        }
        body.child(
            text(
                "lifecycle-expires",
                format!("Expires {}", local_time(&preview.expires_at)),
            )
            .text_xs()
            .text_color(muted),
        )
        .into_any_element()
    }

    /// A refused consent: what changed, its message and code.
    pub(super) fn stale_body(code: ResultCode, message: &str, cx: &App) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        v_flex()
            .gap_1()
            .child(text("lifecycle-stale", STALE).text_sm())
            .child(
                text("lifecycle-message", message.to_owned())
                    .text_sm()
                    .text_color(muted),
            )
            .child(
                text("lifecycle-code", code.as_str())
                    .text_xs()
                    .text_color(muted),
            )
            .into_any_element()
    }
}
