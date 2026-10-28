//! The header's banner slot (design, Banners): as many banners as apply,
//! most severe first. A banner has padding 10 12, radius 6, a tinted fill
//! and border, a 15 px icon, text at 12.5 px and its action.
//!
//! D1's status lines are banners here, with D1's words, IDs and actions:
//! loading, a failed or busy read with Retry, a read that kept what was
//! shown, a context that needs recovery, and an item not found with
//! Refresh. A nonconforming file, an entry with no ID, "Needs fixing":
//! D1's reason (each problem and its guidance), with Repair not available
//! yet. An Edit raises crash recovery off, a prepare still owed and a
//! refusal; its saves raise a base to review, a hand-off owed, a pending
//! checkpoint, the change banner under the review and a refusal (`edit`).
//! The other rows of the design's table are named here for the tasks that
//! raise them.
//!
//! A link clicked in one of the tab's editors is followed here (decision
//! 10; `services::links`). The link banner says when it was not found or
//! is a kind that is not opened, or names the path the nearest match
//! opened, the repository's own; it never names the link's text. The next
//! link clicked in the tab, of any kind, clears it.

use gpui_kit::{
    AnyElement, Context, Hsla, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt,
    component::{ActiveTheme, Icon, h_flex, v_flex},
    div, px,
};
use manyhands::{
    repository::{IndexState, ItemDto},
    results::ResultCode,
};

use super::{
    ItemPane, RECOVERY,
    header::{NoteAt, Weight, later_action},
    text,
};
use crate::desktop::{
    assets::icon_path,
    model::{
        item::{CONTEXT_PROBLEMS, REPAIR, context_problems},
        links::LinkTarget,
        list_state::ListState,
    },
    services::links::{self, LinkResolution},
    view::relationships::OpenRelated,
};

/// The "Needs fixing" banner's heading (`06-language-and-states.md`).
pub(crate) const NEEDS_FIXING: &str = "Needs fixing";
/// D3's link messages (design, Banners and Links).
pub(crate) const LINK_NOT_FOUND: &str = "Link not found";
pub(crate) const LINK_NOT_OPENED: &str = "This kind of link is not opened";
/// Followed by the path the nearest name opened.
pub(crate) const NEAREST_MATCH: &str = "Opened nearest match:";
const ICON_SIZE: f32 = 15.;
const TEXT_SIZE: f32 = 12.5;

/// A banner's tone, which picks its tint and icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Tone {
    Red,
    Amber,
    Info,
}

/// One row of the design's Banners table. The rows are declared in the
/// table's order, which their `Ord` follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Banner {
    /// Loading, refreshing, or not refreshed: info while a read runs,
    /// amber when the read failed or kept what was shown.
    Status { amber: bool },
    /// Context recovery, or the item not found.
    Recovery,
    /// A stored draft for this item (D2's banner, kept; E-8).
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 18.
    StoredDraft,
    /// Changes kept after Done.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 16.
    KeptAfterDone,
    /// A draft discarded elsewhere ended this tab's session (R-71).
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 16.
    DraftDiscarded,
    /// Crash recovery off.
    RecoveryOff,
    /// A save's checkpoint is pending, with Retry.
    CheckpointFailed,
    /// A hand-off to the index still owed: a prepare's, whose Resume sends
    /// it again (R-77), or a save's.
    DiscoveryPending,
    /// The item changed under a dirty or kept draft (R-68), or a save
    /// answered `external_change`.
    Changed,
    /// A base the save left unverified, with Review… (R-36).
    ReviewBeforeSaving,
    /// Another refusal, in the library's message.
    Refused,
    /// A nonconforming file, a ticket with no type included (owner,
    /// 2026-10-27): D1's reason, since the library names no missing field.
    NeedsFixing,
    /// An unmanaged document, with Adopt….
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 25.
    Unmanaged,
    /// A link not found, or opened as the nearest match.
    Link,
}

impl Banner {
    /// The row's tone.
    pub(crate) fn tone(self) -> Tone {
        match self {
            Self::Status { amber: false } | Self::DraftDiscarded | Self::Link => Tone::Info,
            Self::Recovery | Self::CheckpointFailed | Self::Refused => Tone::Red,
            Self::Status { amber: true }
            | Self::StoredDraft
            | Self::KeptAfterDone
            | Self::RecoveryOff
            | Self::DiscoveryPending
            | Self::Changed
            | Self::ReviewBeforeSaving
            | Self::NeedsFixing
            | Self::Unmanaged => Tone::Amber,
        }
    }
}

/// What the link banner says.
pub(super) enum LinkNote {
    NotFound,
    NotOpened,
    Nearest(String),
}

/// Which banner D1's status line of `state` is, if it shows one: none for
/// rows from a current index, red for an item not found, amber for a busy
/// or failed read, and info while loading or from an index that is not
/// current.
pub(crate) fn status_banner<T>(state: &ListState<T>) -> Option<Banner> {
    match state {
        ListState::Ready { index, .. }
            if index
                .as_ref()
                .is_none_or(|index| index.state == IndexState::Current) =>
        {
            None
        }
        ListState::Failed(failure) if failure.code == ResultCode::ItemNotFound => {
            Some(Banner::Recovery)
        }
        ListState::Busy | ListState::Failed(_) => Some(Banner::Status { amber: true }),
        ListState::Loading | ListState::Ready { .. } => Some(Banner::Status { amber: false }),
    }
}

/// Whether `item` is a nonconforming file: the library gives such an entry
/// no ID. A conforming item read by its path is not one.
pub(super) fn nonconforming(item: &ItemDto) -> bool {
    item.id.is_none()
}

/// The banners, most severe first, and in the table's order within a tone,
/// whatever order they were raised in.
pub(crate) fn most_severe_first<E>(mut banners: Vec<(Banner, E)>) -> Vec<(Banner, E)> {
    banners.sort_by_key(|(banner, _)| (banner.tone(), *banner));
    banners
}

impl ItemPane {
    /// The banners that apply to `item`, framed and most severe first,
    /// and how many later actions they drew from `later_from` on.
    pub(super) fn banners(
        &self,
        item: Option<&ItemDto>,
        later_from: usize,
        cx: &mut Context<Self>,
    ) -> (Vec<AnyElement>, usize) {
        let mut banners: Vec<(Banner, AnyElement)> = Vec::new();
        let state = self.model.item();
        if let Some(banner) = status_banner(state) {
            let status = self.status("item", state, cx).into_any_element();
            banners.push((banner, status));
        }
        if let Some(kept) = self.not_refreshed(cx) {
            banners.push((Banner::Status { amber: true }, kept.into_any_element()));
        }
        if let Some(recovery) = item.and_then(|item| self.recovery(item, cx)) {
            banners.push((Banner::Recovery, recovery));
        }
        let mut used = 0;
        if let Some(item) = item.filter(|item| nonconforming(item)) {
            banners.push((Banner::NeedsFixing, self.needs_fixing(item, later_from, cx)));
            used += 1;
        }
        if let Some(note) = &self.link_note {
            banners.push((Banner::Link, link_banner(note, cx)));
        }
        banners.extend(self.edit_banners(cx));
        let framed = most_severe_first(banners)
            .into_iter()
            .enumerate()
            .map(|(ix, (banner, content))| frame(ix, banner, content, cx))
            .collect();
        (framed, used)
    }

    /// D1's recovery guidance, for a context or branch problem: it says
    /// the context needs repair and lists each problem's guidance.
    fn recovery(&self, item: &ItemDto, cx: &mut Context<Self>) -> Option<AnyElement> {
        let problems = context_problems(item);
        if problems.is_empty() {
            return None;
        }
        let muted = cx.theme().muted_foreground;
        Some(
            v_flex()
                .id("item-recovery")
                .test_support()
                .aria_label(RECOVERY)
                .gap_1()
                .child(div().child(RECOVERY))
                .children(problems.into_iter().enumerate().map(|(ix, problem)| {
                    v_flex()
                        .child(text(("recovery-problem", ix), problem.code.as_str()))
                        .child(
                            text(("recovery-guidance", ix), problem.guidance()).text_color(muted),
                        )
                        .children(
                            problem
                                .path
                                .clone()
                                .map(|path| text(("recovery-path", ix), path).text_color(muted)),
                        )
                }))
                .into_any_element(),
        )
    }

    /// "Needs fixing", D1's reason (each problem with its guidance) and
    /// Repair, which is not available yet.
    fn needs_fixing(&self, item: &ItemDto, later: usize, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let problems: Vec<_> = item
            .problems
            .iter()
            .filter(|problem| !CONTEXT_PROBLEMS.contains(&problem.code))
            .enumerate()
            .map(|(ix, problem)| {
                v_flex()
                    .child(text(("item-problem", ix), problem.code.as_str()))
                    .child(
                        text(("item-problem-guidance", ix), problem.guidance()).text_color(muted),
                    )
                    .children(
                        problem
                            .path
                            .clone()
                            .map(|path| text(("item-problem-path", ix), path).text_color(muted)),
                    )
            })
            .collect();
        h_flex()
            .flex_wrap()
            .gap_3()
            .items_start()
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_1()
                    .child(
                        text("item-needs-fixing", NEEDS_FIXING)
                            .font_weight(gpui_kit::FontWeight::SEMIBOLD),
                    )
                    .children(problems),
            )
            .child(later_action(
                later,
                REPAIR,
                None,
                Weight::Plain,
                NoteAt::Below,
                cx,
            ))
            .into_any_element()
    }
}

impl ItemPane {
    /// Decision 10: a link clicked in one of this tab's editors, the read
    /// view, a comment's or the edit editor. It is the tab's next
    /// navigation, so the last link's banner goes, and so does a
    /// resolution still running. A browser link, which the editor opened,
    /// and a link to nothing, such as a heading here, do no more. A scheme
    /// that is not opened says so; a path or a title is resolved from the
    /// item's folder, a comment's too, and opens its tab, or says it was
    /// not found. The banner never names the target.
    pub(crate) fn follow_link(&mut self, target: &LinkTarget, cx: &mut Context<Self>) {
        self.link_note = None;
        self.link_read = None;
        cx.notify();
        match target {
            LinkTarget::Path { .. } | LinkTarget::Title(_) => {}
            LinkTarget::Unsafe => {
                self.link_note = Some(LinkNote::NotOpened);
                return;
            }
            LinkTarget::External(_) | LinkTarget::Ignored => return,
        }
        let Some(item) = self.model.item().rows() else {
            self.link_note = Some(LinkNote::NotFound);
            return;
        };
        let resolution = links::resolve(target.clone(), item, self.repo(), &self.services, cx);
        self.link_read = Some(cx.spawn(async move |pane, cx| {
            let resolution = resolution.await;
            // The tab may be gone; then nobody follows the link.
            let _ = pane.update(cx, |pane, cx| pane.link_resolved(resolution, cx));
        }));
    }

    /// A resolved item, or a file with no ID, opens its tab in this
    /// repository; the nearest match says, in this tab, which path opened;
    /// anything else is not found.
    fn link_resolved(&mut self, resolution: LinkResolution, cx: &mut Context<Self>) {
        self.link_read = None;
        let repo = self.repo().to_owned();
        match resolution {
            LinkResolution::Item(locator) => cx.emit(OpenRelated(locator.in_repository(&repo))),
            LinkResolution::Nearest { locator, path } => {
                self.link_note = Some(LinkNote::Nearest(path));
                cx.emit(OpenRelated(locator.in_repository(&repo)));
            }
            LinkResolution::NotManaged | LinkResolution::NotFound => {
                self.link_note = Some(LinkNote::NotFound);
            }
        }
        cx.notify();
    }
}

/// The link banner's words: "not found", "not opened", or the nearest
/// match's path, the repository's own, in the monospace font.
fn link_banner(note: &LinkNote, cx: &gpui_kit::App) -> AnyElement {
    match note {
        LinkNote::NotFound => text("item-link", LINK_NOT_FOUND).into_any_element(),
        LinkNote::NotOpened => text("item-link", LINK_NOT_OPENED).into_any_element(),
        LinkNote::Nearest(path) => h_flex()
            .id("item-link")
            .test_support()
            .aria_label(format!("{NEAREST_MATCH} {path}"))
            .flex_wrap()
            .gap_1()
            .child(NEAREST_MATCH)
            .child(
                div()
                    .font_family(cx.theme().mono_font_family.clone())
                    .child(path.clone()),
            )
            .into_any_element(),
    }
}

/// The tint and icon of `tone`.
fn look(tone: Tone, cx: &gpui_kit::App) -> (Hsla, &'static str) {
    let theme = cx.theme();
    match tone {
        Tone::Info => (theme.info, "info"),
        Tone::Amber => (theme.warning, "circle-alert"),
        Tone::Red => (theme.danger, "circle-alert"),
    }
}

/// `content` in `banner`'s frame, the `ix`th of the slot.
fn frame(ix: usize, banner: Banner, content: AnyElement, cx: &gpui_kit::App) -> AnyElement {
    let (tint, icon) = look(banner.tone(), cx);
    let foreground = cx.theme().foreground;
    h_flex()
        .id(("item-banner", ix))
        .test_support()
        .w_full()
        .items_start()
        .gap(px(10.))
        .px(px(12.))
        .py(px(10.))
        .rounded(px(6.))
        .border_1()
        .border_color(tint.opacity(0.45))
        .bg(tint.opacity(0.13))
        .text_size(px(TEXT_SIZE))
        .text_color(foreground)
        .child(
            Icon::default()
                .path(icon_path(icon))
                .flex_none()
                .w(px(ICON_SIZE))
                .h(px(ICON_SIZE))
                .text_color(tint),
        )
        .child(div().flex_1().min_w_0().child(content))
        .into_any_element()
}
