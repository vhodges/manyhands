//! A draft in an item's tab: the banner for a stored one, and the section
//! of one restored here.
//!
//! The banner shows when the last listing holds a draft of this item that
//! this process has not claimed. It sits in the header's banner slot
//! (`Banner::StoredDraft`; design, Banners). Nothing is restored on its
//! own; Restore asks the registry to claim the draft and attach it to this
//! tab, then opens the tab through its item locator (ruling R25). A
//! refusal is shown in words; a draft another process holds may be tried
//! again. Review in Drafts asks the workspace to open the Drafts page on
//! the draft's row. Continue editing, last, takes the restored-draft route
//! (`edit`'s `restored`).
//!
//! The section shows the draft's own fields read-only, whether its newest
//! text is saved for recovery (Retry on a failed write), where the item
//! stands against it (Retry on a failed read), Continue editing and Save,
//! which go on through the restored-draft route, then, when the item
//! changed, Compare, which expands the comparison inline (R-17), and
//! Export… and Discard…, the Drafts page's own (`draft_actions`). A reload
//! of the tab recomputes where the item stands; it never touches the
//! draft's text or base. The section is the tab's `draft-section` (R-38).

use gpui_kit::{
    AnyElement, App, ClickEvent, Context, ElementId, EventEmitter, InteractiveElement, IntoElement,
    ParentElement, StatefulInteractiveElement, Styled, TestSupportExt,
    component::{ActiveTheme, button::Button, h_flex, v_flex},
    div,
};
use manyhands::{
    drafts::{
        DraftError, DraftErrorKind, DraftId, RefusalReason, Restore, TargetState, classify,
        restore_policy,
    },
    repository::ItemDto,
};
use time::format_description::well_known::Rfc3339;

use super::{ItemPane, text};
use crate::desktop::{
    model::{
        drafts::RecoveryState,
        editor_text::normalized,
        navigation::{ItemLocator, OpenTarget},
        tabs::TabKey,
    },
    services::drafts::RestoreRefused,
    view::{
        confirm::local_time,
        draft_comparison::{DraftComparisonView, FieldIds, draft_fields, target_label},
        drafts::actions::OPEN_ELSEWHERE_RETRY,
        relationships::OpenRelated,
        sidebar::RETRY,
    },
};

pub(crate) const DRAFT_KEPT: &str = "A recovered draft of this item is kept";
pub(crate) const RESTORE: &str = "Restore";
pub(crate) const REVIEW_IN_DRAFTS: &str = "Review in Drafts";
pub(crate) const RESTORED_DRAFT: &str = "Restored draft";
pub(crate) const SAVED_FOR_RECOVERY: &str = "Saved for recovery at";
pub(crate) const NOT_YET_SAVED: &str = "Not yet saved for recovery";
pub(crate) const NOT_SAVED: &str = "Recovery copy not saved";
pub(crate) const COMPARE: &str = "Compare";
pub(crate) const ANOTHER_OPEN: &str = "Another draft of this item is open";
pub(crate) const NO_LONGER_KEPT: &str = "This draft is no longer kept";
pub(crate) const NOT_THIS_ITEM: &str = "This draft is not of this item";
pub(crate) const NOT_DISCARDED: &str = "Draft not discarded";

/// The restored draft section's own fields.
const SECTION_FIELDS: FieldIds = FieldIds {
    title: "draft-title",
    field: "draft-field",
    body: "draft-body",
};

/// The banner's Review in Drafts: the Drafts page is to open on this
/// draft's row.
pub(crate) struct ReviewInDrafts(pub DraftId);

impl EventEmitter<ReviewInDrafts> for ItemPane {}

/// A pane's draft state: what Restore answered, a failed discard, what
/// the last export came to, Discard's focus, and where the item stands
/// against the attached session, kept until the tab reads again or the
/// session changes.
#[derive(Default)]
pub(super) struct TabDraft {
    restoring: bool,
    refused: Option<RestoreRefused>,
    discard_failed: Option<DraftErrorKind>,
    comparing: bool,
    /// What the last export came to, in words.
    pub(super) exported: Option<String>,
    /// The section's Discard…, where the dialog gives the focus back.
    pub(super) discard_focus: Option<gpui_kit::FocusHandle>,
    /// The section's actions row, scrolled into view when the focus is
    /// in it.
    pub(super) actions_focus: Option<gpui_kit::FocusHandle>,
    /// The session and revision it was computed for, and the state.
    target: Option<(DraftId, u64, TargetState)>,
}

impl TabDraft {
    /// The tab read again: where the item stands is computed afresh.
    pub(super) fn reads_landed(&mut self) {
        self.target = None;
    }

    /// Whether a claim of a stored draft is in flight.
    pub(super) fn restoring(&self) -> bool {
        self.restoring
    }

    /// What the last claim was refused for.
    pub(super) fn refused(&self) -> Option<RestoreRefused> {
        self.refused
    }
}

/// The text of a refusal.
pub(crate) fn refused_label(refused: RestoreRefused) -> String {
    match refused {
        RestoreRefused::AnotherOpen => ANOTHER_OPEN.to_owned(),
        RestoreRefused::NoLongerKept => NO_LONGER_KEPT.to_owned(),
        RestoreRefused::NotThisItem => NOT_THIS_ITEM.to_owned(),
        // Perhaps only a passing probe of its lock: Restore may be tried
        // again.
        RestoreRefused::OpenElsewhere => OPEN_ELSEWHERE_RETRY.to_owned(),
        RestoreRefused::Failed(kind) => error_text(kind),
    }
}

/// The store's fixed sentence for `kind`.
pub(super) fn error_text(kind: DraftErrorKind) -> String {
    DraftError { kind }.to_string()
}

/// A recovery state in words.
pub(crate) fn recovery_label(state: RecoveryState) -> String {
    match state {
        RecoveryState::Saved(at) => match at.format(&Rfc3339) {
            Ok(at) => format!("{SAVED_FOR_RECOVERY} {}", local_time(&at)),
            Err(_) => SAVED_FOR_RECOVERY.to_owned(),
        },
        RecoveryState::Pending => NOT_YET_SAVED.to_owned(),
        RecoveryState::Failed(kind) => format!("{NOT_SAVED}: {}", error_text(kind)),
    }
}

pub(super) fn button(
    id: &'static str,
    label: &'static str,
    on_click: impl Fn(&ClickEvent, &mut gpui_kit::Window, &mut App) + 'static,
) -> impl IntoElement {
    div().id(id).test_support().aria_label(label).child(
        Button::new(ElementId::from(format!("{id}-button")))
            .label(label)
            .on_click(on_click),
    )
}

impl ItemPane {
    pub(in crate::desktop::view) fn tab_key(&self) -> TabKey {
        TabKey::for_target(self.model.target())
    }

    /// Restores the stored draft `id` into this tab, once at a time, and
    /// shows the tab, as an item's tab is opened anywhere (D2's Restore).
    pub(super) fn restore(&mut self, id: DraftId, cx: &mut Context<Self>) {
        self.claim(
            id,
            |pane, _, cx| {
                let OpenTarget::Item {
                    repo,
                    kind,
                    id: item,
                } = pane.model.target().clone()
                else {
                    return;
                };
                let locator = ItemLocator::Identified { kind, id: item };
                cx.emit(OpenRelated(locator.in_repository(&repo)));
            },
            cx,
        );
    }

    /// Claims the stored draft `id` into this tab, once at a time, and
    /// attaches it; then `claimed` goes on with the session's ID. A refusal
    /// is said in the banner.
    pub(super) fn claim(
        &mut self,
        id: DraftId,
        claimed: impl FnOnce(&mut Self, DraftId, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) {
        if !matches!(self.model.target(), OpenTarget::Item { .. }) || self.draft.restoring {
            return;
        }
        self.draft.restoring = true;
        self.draft.refused = None;
        self.draft.discard_failed = None;
        let tab = self.tab_key();
        let restore = self
            .services
            .drafts()
            .update(cx, |registry, cx| registry.restore(id, tab, cx));
        let drafts = self.services.drafts().clone();
        cx.spawn(async move |pane, cx| {
            let restored = restore.await;
            let shown = pane.update(cx, |pane, cx| {
                pane.draft.restoring = false;
                match restored {
                    Ok(restored) => {
                        pane.draft.comparing = false;
                        pane.draft.target = None;
                        claimed(pane, restored, cx);
                    }
                    Err(refused) => pane.draft.refused = Some(refused),
                }
                cx.notify();
            });
            // The tab closed, or its repository went, while the claim was
            // taken: nothing shows the session, so it is let go of and the
            // draft offered again.
            if let (Err(_), Ok(restored)) = (shown, restored) {
                drafts.update(cx, |registry, cx| {
                    registry.release_when_stored(restored, cx)
                });
            }
        })
        .detach();
    }

    /// The draft's discard failed: the store has let go of it, and it is
    /// listed again.
    pub(in crate::desktop::view) fn discard_failed(
        &mut self,
        kind: DraftErrorKind,
        cx: &mut Context<Self>,
    ) {
        self.draft.discard_failed = Some(kind);
        cx.notify();
    }

    /// The banner, when the last listing holds a draft of this item this
    /// process has not claimed, and what the last Restore answered.
    pub(super) fn draft_banner(
        &self,
        item: Option<&ItemDto>,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let muted = cx.theme().muted_foreground;
        let refused = self.draft.refused.map(|refused| {
            text("draft-refused", refused_label(refused))
                .text_sm()
                .into_any_element()
        });
        let discard_failed = self.draft.discard_failed.map(|kind| {
            text(
                "draft-discard-failed",
                format!("{NOT_DISCARDED}: {}", error_text(kind)),
            )
            .text_sm()
            .into_any_element()
        });
        let offered = item.and_then(|item| {
            let TabKey::Item { repo, kind, id } = self.tab_key() else {
                return None;
            };
            let registry = self.services.drafts().read(cx);
            let stored = registry.stored_for(&repo, kind, &id)?;
            let (draft, held) = registry.listed(stored)?;
            // Normalized as the base is (R-11, R-72).
            let state = classify(draft, Ok(&normalized(item)));
            let policy = restore_policy(draft, &state, held);
            Some((stored, state, policy))
        });
        let banner = offered.map(|(stored, state, policy)| {
            // Held elsewhere may pass: Restore stays offered.
            let restorable = matches!(
                policy,
                Restore::Allowed
                    | Restore::AllowedFlagged
                    | Restore::Refused(RefusalReason::OpenElsewhere)
            );
            let action = if restorable {
                button(
                    "draft-restore",
                    RESTORE,
                    cx.listener(move |this, _: &ClickEvent, _, cx| this.restore(stored, cx)),
                )
                .into_any_element()
            } else {
                text("draft-banner-state", target_label(&state))
                    .text_xs()
                    .text_color(muted)
                    .into_any_element()
            };
            let review = button(
                "draft-review",
                REVIEW_IN_DRAFTS,
                cx.listener(move |_, _: &ClickEvent, _, cx| cx.emit(ReviewInDrafts(stored))),
            );
            // D2's actions stay; Continue editing comes last (design,
            // Banners and Design Deviations).
            let continue_editing =
                restorable.then(|| self.continue_button("draft-banner-continue", cx));
            h_flex()
                .flex_wrap()
                .gap_2()
                .items_center()
                .child(text("draft-banner", DRAFT_KEPT).text_sm())
                .child(action)
                .child(review)
                .children(continue_editing)
                .into_any_element()
        });
        if banner.is_none() && refused.is_none() && discard_failed.is_none() {
            return None;
        }
        // In the header's banner slot, which frames it.
        Some(
            v_flex()
                .gap_1()
                .children(banner)
                .children(refused)
                .children(discard_failed)
                .into_any_element(),
        )
    }

    /// The section of the draft restored in this tab.
    pub(super) fn draft_section(
        &mut self,
        window: &gpui_kit::Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let tab = self.tab_key();
        let registry = self.services.drafts().read(cx);
        let id = registry.attached(&tab)?;
        let session = registry.session(id)?;
        let revision = session.revision();
        let fresh =
            matches!(&self.draft.target, Some((at, seen, _)) if *at == id && *seen == revision);
        if !fresh && let Some(item) = self.model.item_read() {
            self.draft.target = registry
                .classify(id, item)
                .map(|state| (id, revision, state));
        }
        let registry = self.services.drafts().read(cx);
        let session = registry.session(id)?;
        let mine = session.current().clone();
        let base = session.base().map(|base| base.fields.clone());
        let recovery = registry.recovery_state(id)?;
        let state = self.draft.target.as_ref().map(|(_, _, state)| state);
        let comparison = match state {
            Some(TargetState::Changed {
                theirs, comparison, ..
            })
            | Some(TargetState::Unverified { theirs, comparison }) => {
                // With no base, the comparison was made from the item.
                let base = base.as_ref().unwrap_or(theirs);
                Some(DraftComparisonView::new(comparison, base, theirs, &mine))
            }
            _ => None,
        };
        let target = state.map(|state| self.target_line(state, cx));
        let compare = comparison.map(|comparison| {
            let expanded = self.draft.comparing;
            v_flex()
                .gap_2()
                .child(button(
                    "draft-compare",
                    COMPARE,
                    cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.draft.comparing = !this.draft.comparing;
                        cx.notify();
                    }),
                ))
                .children(expanded.then(|| comparison.render(cx)))
        });
        let continuing = h_flex()
            .flex_wrap()
            .gap_2()
            .child(self.continue_button("draft-continue", cx))
            .child(self.restored_save_button(cx));
        Some(
            v_flex()
                .id("draft-section")
                .test_support()
                .aria_label(RESTORED_DRAFT)
                .gap_2()
                .p_2()
                .border_1()
                .border_color(cx.theme().border)
                .child(div().text_xs().child(RESTORED_DRAFT))
                .child(draft_fields(SECTION_FIELDS, &mine, cx))
                .child(self.recovery_line(id, recovery, cx))
                .children(target)
                // Continue editing before Compare (R-17), then D2's order.
                .child(continuing)
                .children(compare)
                .child(self.draft_actions(id, window, cx))
                .into_any_element(),
        )
    }

    fn recovery_line(
        &self,
        id: DraftId,
        state: RecoveryState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let retry = matches!(state, RecoveryState::Failed(_)).then(|| {
            button(
                "draft-recovery-retry",
                RETRY,
                cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.services
                        .drafts()
                        .update(cx, |registry, cx| registry.flush_now(id, cx))
                        .detach();
                }),
            )
        });
        h_flex()
            .gap_2()
            .child(text("draft-recovery", recovery_label(state)).text_sm())
            .children(retry)
    }

    fn target_line(&self, state: &TargetState, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let retry = matches!(state, TargetState::ReadFailed(_)).then(|| {
            button(
                "draft-target-retry",
                RETRY,
                cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.reload(cx);
                }),
            )
        });
        h_flex()
            .gap_2()
            .child(text("draft-target", target_label(state)).text_sm())
            .children(retry)
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "draft_tests.rs"]
mod tests;
