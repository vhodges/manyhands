//! The selected repository's detail in the content region, until Cycle 03a
//! Task 7 takes it out (Part F), and the constants other views share. The
//! Repositories panel itself is `view::repositories`.
//!
//! Text is fixed per state or `ResultCode::message()`; no error is ever
//! formatted into it.

use gpui_kit::{
    App, ClickEvent, Context, FocusHandle, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, StatefulInteractiveElement, Styled, TestSupportExt, ViewElement, WeakEntity,
    Window,
    component::{
        ActiveTheme, Disableable, Icon, Sizable,
        button::{Button, ButtonVariants},
        h_flex, v_flex,
    },
    div,
};
use manyhands::repository::{ConfigurationDto, ConfigurationState};

use super::{scroll::ContentScroll, workspace::Workspace};
use crate::desktop::{
    assets::icon_path,
    model::{
        list_state::index_label,
        navigation::{Navigation, RepositoryEntry, Resolution},
        stamp::RepoKey,
    },
};

pub(crate) const UNAVAILABLE: &str = "Unavailable";
pub(crate) const RESOLVING: &str = "Opening";
pub(crate) const NOTHING_SELECTED: &str = "Select a repository.";
pub(crate) const RETRY: &str = "Retry";
pub(crate) const REMOVE: &str = "Remove";
/// The chosen repository's row says so in text: its mark's label.
pub(crate) const SELECTED: &str = "Selected";
/// The repository detail's Share main: `repo sync` (design, Lane B, Share
/// controls).
pub(crate) const SHARE_MAIN: &str = "Share main";
/// Share main with no publication remote: it opens Settings › Remotes, as
/// the item header's Share… does.
pub(crate) const SHARE_MAIN_SET_UP: &str = "Share main…";
pub(crate) const SHARE_MAIN_BUTTON: &str = "selection-share-main";
/// Share or Share main while a share of the same target is queued or
/// running: disabled, so a second press queues nothing.
pub(crate) const SHARING: &str = "Sharing…";

/// A configuration state in fixed text.
pub(crate) fn configuration_label(state: ConfigurationState) -> &'static str {
    match state {
        ConfigurationState::Valid => "Configuration valid",
        ConfigurationState::Invalid => "Configuration invalid",
        ConfigurationState::Missing => "Configuration missing",
    }
}

/// The selected repository: its configuration and index, or why it is
/// unavailable with Retry and Remove. `remove_focus` is the focus Remove's
/// wrapper tracks, and `scroll` the content region's.
pub(crate) fn selection(
    navigation: &Navigation,
    remove_focus: &FocusHandle,
    retry_focus: &FocusHandle,
    scroll: &ContentScroll,
    cx: &mut Context<Workspace>,
) -> impl IntoElement + use<> {
    let Some(entry) = navigation.selected_entry() else {
        return v_flex()
            .gap_2()
            .child(div().text_sm().child(NOTHING_SELECTED));
    };
    let muted = cx.theme().muted_foreground;
    let key = entry.key().clone();
    let heading = v_flex()
        .child(
            div()
                .id("selection-name")
                .test_support()
                .aria_label(entry.name())
                .text_lg()
                .child(entry.name()),
        )
        .child(
            div()
                .text_xs()
                .text_color(muted)
                .child(entry.summary.root.clone()),
        );
    let body = match &entry.resolution {
        Resolution::Unknown => v_flex().child(
            div()
                .id("selection-resolving")
                .test_support()
                .aria_label(RESOLVING)
                .child(RESOLVING),
        ),
        Resolution::Resolved(_) => resolved(entry, cx),
        Resolution::Unavailable(code) => v_flex()
            .gap_2()
            .child(
                div()
                    .id("selection-unavailable-state")
                    .test_support()
                    .aria_label(UNAVAILABLE)
                    .font_weight(gpui_kit::FontWeight::BOLD)
                    .child(UNAVAILABLE),
            )
            .child(
                div()
                    .id("selection-unavailable")
                    .test_support()
                    .aria_label(code.message())
                    .child(code.message()),
            )
            .child(div().text_xs().text_color(muted).child(code.as_str()))
            .child(
                h_flex()
                    .gap_2()
                    .child(retry(key.clone(), retry_focus, cx))
                    .child(remove(key, remove_focus, scroll, cx)),
            ),
        Resolution::Busy => v_flex()
            .gap_2()
            .child(
                div()
                    .id("selection-busy")
                    .test_support()
                    .aria_label(crate::desktop::model::list_state::LABEL_BUSY)
                    .child(crate::desktop::model::list_state::LABEL_BUSY),
            )
            .child(retry(key, retry_focus, cx)),
        Resolution::Failed(failure) => v_flex()
            .gap_2()
            .child(
                div()
                    .id("selection-failed")
                    .test_support()
                    .aria_label(failure.code.message())
                    .child(failure.code.message()),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(muted)
                    .child(failure.code.as_str()),
            )
            .child(retry(key, retry_focus, cx)),
    };
    v_flex().gap_3().child(heading).child(body)
}

/// Retry on the selection. Its wrapper tracks `focus`, around the Kit
/// button's own, so a test sees it hold focus.
fn retry(
    key: String,
    focus: &FocusHandle,
    cx: &mut Context<Workspace>,
) -> impl IntoElement + use<> {
    div()
        .id("selection-retry")
        .test_support()
        .aria_label(RETRY)
        .track_focus(focus)
        .child(Button::new("selection-retry-button").label(RETRY).on_click(
            cx.listener(move |this, _: &ClickEvent, _, cx| this.retry_resolution(key.clone(), cx)),
        ))
}

/// Remove on an Unavailable entry: prepares `repo remove` of its root in
/// the confirmation dialog. A Kit button, so Tab reaches it and Enter
/// presses it; its wrapper tracks a focus of the workspace's, which the
/// dialog gives back, and carries the scroll marker.
fn remove(
    key: String,
    focus: &FocusHandle,
    scroll: &ContentScroll,
    cx: &mut Context<Workspace>,
) -> impl IntoElement + use<> {
    super::settings::wrapped(
        "selection-remove",
        REMOVE,
        focus,
        scroll,
        Button::new("selection-remove-button")
            .danger()
            .label(REMOVE)
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.remove_repository(key.clone(), window, cx)
            })),
    )
}

/// The configuration and index state the list read stored for a
/// resolved entry, with an invalid configuration's problems.
fn resolved(entry: &RepositoryEntry, cx: &mut Context<Workspace>) -> gpui_kit::Div {
    let muted = cx.theme().muted_foreground;
    let ConfigurationDto {
        state,
        primary_branch,
        problems,
        ..
    } = &entry.summary.configuration;
    let index = index_label(&entry.summary.index);
    v_flex()
        .gap_1()
        .child(
            div()
                .id("selection-configuration")
                .test_support()
                .aria_label(configuration_label(*state))
                .child(configuration_label(*state)),
        )
        .children(primary_branch.clone().map(|branch| {
            h_flex()
                .gap_1()
                .text_sm()
                .child(div().text_color(muted).child("Primary branch"))
                .child(branch)
        }))
        .children(problems.iter().enumerate().map(|(index, problem)| {
            div()
                .id(("selection-problem", index))
                .test_support()
                .aria_label(problem.code.guidance())
                .text_sm()
                .child(problem.code.guidance())
        }))
        .child(
            div()
                .id("selection-index")
                .test_support()
                .aria_label(index)
                .child(index),
        )
        .child(div().flex().pt_2().child(ShareMain {
            workspace: cx.entity().downgrade(),
            root: entry.key().clone(),
            remote: entry.summary.configuration.publication_remote.is_some(),
        }))
}

/// Share main on a resolved repository: `repo sync` of `root` when it has
/// a publication remote (`remote`), Share main… opening Settings › Remotes
/// when it has none, and Sharing…, disabled, while a share of the
/// repository is queued or running.
///
/// It is drawn as the frame lays it out, after the workspace has drawn
/// itself, so it reads from the workspace's worker whether a share is
/// running; `selection` is drawn while the workspace is being updated,
/// when it cannot be read.
struct ShareMain {
    workspace: WeakEntity<Workspace>,
    root: RepoKey,
    remote: bool,
}

impl RenderOnce for ShareMain {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let sharing = self
            .workspace
            .upgrade()
            .is_some_and(|workspace| workspace.read(cx).sharing_main(&self.root));
        let label = match (sharing, self.remote) {
            (true, _) => SHARING,
            (false, true) => SHARE_MAIN,
            (false, false) => SHARE_MAIN_SET_UP,
        };
        let Self {
            workspace,
            root,
            remote,
        } = self;
        Button::new(SHARE_MAIN_BUTTON)
            .label(label)
            .small()
            .icon(Icon::default().path(icon_path("cloud-upload")))
            .disabled(sharing)
            .on_click(move |_: &ClickEvent, window, cx| {
                let root = root.clone();
                let _ = workspace.update(cx, |workspace, cx| {
                    workspace.share_main(root, remote, window, cx)
                });
            })
    }
}

impl IntoElement for ShareMain {
    type Element = ViewElement<Self>;

    fn into_element(self) -> Self::Element {
        ViewElement::new(self)
    }
}
