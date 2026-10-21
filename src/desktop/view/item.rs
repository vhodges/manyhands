//! The read-only item view: what an activated row opened, read through
//! the library each time it loads.
//!
//! The header gives the kind, title, short code and ID, with separate
//! copy actions, then the lifecycle closure and the project status as
//! separate fields; a ticket's relationships panel follows them
//! (`relationships.rs`). Provenance names the context kind, branch, worktree
//! and head; nothing claims the copy equals or differs from the primary.
//! An item with a context or branch problem shows a recovery view in its
//! place: the problem codes, their guidance and paths, and no choice of
//! context. Unknown metadata is listed read-only; the source is shown as
//! the file's text, never rendered. Comments follow in list order,
//! indented by depth. The actions of later releases show, unavailable,
//! in fixed text.
//!
//! A tab loads in one read job: the repository is resolved once, then the
//! item, its comments and, for a ticket, its relationships are read one
//! after another, in the design's order. A load after the first keeps
//! what is shown until it lands. One that stays busy keeps it still, and
//! says "Could not refresh" with Busy's message and Retry, which loads
//! again in place; one that fails shows the failure as a first load does,
//! and the tab keeps its last title (ruling R-B5).
//!
//! An item the library no longer finds shows `item_not_found` with
//! Refresh, which refreshes the repository's index through the worker and
//! reads the item again. Every failure is shown by its code's message and
//! the code; no error is formatted.

use std::{collections::HashMap, path::PathBuf};

use gpui_kit::{
    AnyElement, App, AppContext, ClickEvent, ClipboardItem, Context, ElementId, Entity,
    FocusHandle, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Subscription, Task, TestSupportExt, Window,
    component::{
        ActiveTheme, Disableable,
        button::{Button, ButtonVariants},
        h_flex,
        input::{IndentInline, OutdentInline, Textarea, TextareaState},
        v_flex,
    },
    div, px,
};
use manyhands::{
    repository::{ClosureState, CommentListDto, DependencyDirection, ItemDto, ItemDtoKind},
    results::ResultCode,
};

use super::{
    relationships::RowKey,
    scroll::ContentScroll,
    sidebar::RETRY,
    tickets::{LIFECYCLE_CLOSED, LIFECYCLE_OPEN},
};
use crate::desktop::{
    model::{
        item::{
            CONTEXT_PROBLEMS, ItemReads, ItemView, LATER, LATER_NOTE, NOT_REPRESENTABLE,
            comment_author, context_kind_label, context_problems, metadata_value, provenance_label,
        },
        list_state::{ListState, Rows},
        navigation::{NotReloaded, OpenTarget},
        relationships::{RELATIONSHIP_PROBLEMS, parent_to_read},
        stamp::{Stamp, ViewScope},
    },
    services::{
        ReadKind, ReadResult, Reader, Services,
        requests::{DesktopRequestId, Intent, Lane, RequestState},
    },
};

pub(crate) const NONCONFORMING_FILE: &str = "Nonconforming file";
pub(crate) const SOURCE_NOT_UTF8: &str =
    "This file is not valid UTF-8, so its text cannot be shown.";
pub(crate) const REFRESH: &str = "Refresh";
pub(crate) const SOURCE: &str = "Source";
pub(crate) const REFRESHING: &str = "Refreshing the index";
pub(crate) const RECOVERY: &str = "This item's context needs repair before it can be read from it.";
pub(crate) const COPY_ID: &str = "Copy ID";
pub(crate) const COPY_SLUG: &str = "Copy short code";
/// What is shown is from an earlier load: the last one stayed busy.
pub(crate) const NOT_REFRESHED: &str = "Could not refresh";
/// How far each level of a reply is indented.
const COMMENT_INDENT: f32 = 16.;

pub(crate) struct ItemPane {
    pub(super) model: ItemView,
    services: Services,
    /// The tab's load: one read job, its calls one after another under one
    /// stamp. Replacing it drops the last.
    read: Option<Task<()>>,
    /// The focus of each relationship row that can be opened, by its
    /// group's row name, its ticket's ID and which row of that ID in the
    /// group it is, so it follows its ticket when a reload reorders the
    /// group. Only the rows drawn in the last frame keep theirs.
    pub(super) row_focus: HashMap<RowKey, FocusHandle>,
    /// Around Copy short code and Copy ID, whose own focus is the Kit's.
    copy_focus: [FocusHandle; 2],
    /// The index refresh Refresh asked for; the item is read again when it
    /// ends.
    refreshing: Option<DesktopRequestId>,
    /// The read-only text the source is shown in, and the source it was
    /// last given. Made at the next frame, which has the window it needs.
    source: Option<(String, Entity<TextareaState>)>,
    /// The content region's scrolling, which brings a control of this
    /// view into view when it takes focus.
    pub(super) scroll: ContentScroll,
    _operations: Subscription,
}

/// One load of a tab, in the design's order: the repository is resolved
/// once, then `show_item` (or `show_path` for a nonconforming entry),
/// `list_comments`, and for a ticket `ticket_dependencies`,
/// `ticket_children` and the parent's `show_item`. A load whose
/// resolution or item read gives nothing stops there: the reads after it
/// give the same answer, and the parent is not read.
///
/// The tree is asked for in both directions to depth 1: `up` gives the
/// tickets that depend on the ticket, and `down` the short code and title
/// of each it depends on, from the same snapshot.
#[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
async fn read_tab(reader: Reader, target: OpenTarget) -> ItemReads {
    let root = match &target {
        OpenTarget::Item { repo, .. } | OpenTarget::Inspect { repo, .. } => PathBuf::from(repo),
    };
    let repo = match reader
        .call(ReadKind::Resolve, |service| {
            service.resolve_repository(&root)
        })
        .await
    {
        ReadResult::Ok(repo) => repo,
        ReadResult::Busy => return ItemReads::stopped(&target, ReadResult::Busy),
        ReadResult::Failed(failure) => {
            return ItemReads::stopped(&target, ReadResult::Failed(failure));
        }
    };
    let (kind, id) = match &target {
        OpenTarget::Item { kind, id, .. } => (*kind, id),
        OpenTarget::Inspect { worktree, path, .. } => {
            let (worktree, path) = (PathBuf::from(worktree), PathBuf::from(path));
            let item = reader
                .call(ReadKind::Item, |service| {
                    service.show_path(&repo, Some(&worktree), &path)
                })
                .await;
            return ItemReads::stopped(&target, item);
        }
    };
    let item = reader
        .call(ReadKind::Item, |service| service.show_item(&repo, id))
        .await;
    let parent = match &item {
        ReadResult::Ok(item) => parent_to_read(item.parent.as_ref()),
        _ => return ItemReads::stopped(&target, item),
    };
    let comments = reader
        .call(ReadKind::Comments, |service| {
            service.list_comments(&repo, id)
        })
        .await;
    if kind != ItemDtoKind::Ticket {
        return ItemReads {
            item,
            comments: Some(comments),
            dependents: None,
            children: None,
            parent: None,
        };
    }
    let dependents = reader
        .call(ReadKind::Dependencies, |service| {
            service.ticket_dependencies(&repo, id, DependencyDirection::Both, Some(1))
        })
        .await;
    let children = reader
        .call(ReadKind::Children, |service| {
            service.ticket_children(&repo, id)
        })
        .await;
    let parent = match parent {
        Some(parent) => Some(
            reader
                .call(ReadKind::Item, |service| service.show_item(&repo, &parent))
                .await,
        ),
        None => None,
    };
    ItemReads {
        item,
        comments: Some(comments),
        dependents: Some(dependents),
        children: Some(children),
        parent,
    }
}

impl ItemPane {
    /// The view of `target`, loading at once, its reads stamped under
    /// `scope`.
    pub(crate) fn new(
        target: OpenTarget,
        scope: ViewScope,
        services: Services,
        scroll: ContentScroll,
        cx: &mut Context<Self>,
    ) -> Self {
        let operations = cx.observe(services.operations(), |this, _, cx| {
            this.operations_changed(cx)
        });
        let mut pane = Self {
            model: ItemView::new(target, scope),
            services,
            read: None,
            row_focus: HashMap::new(),
            copy_focus: [cx.focus_handle(), cx.focus_handle()],
            refreshing: None,
            source: None,
            scroll,
            _operations: operations,
        };
        let stamp = pane.model.load();
        pane.read(stamp, cx);
        pane
    }

    /// What this view shows.
    #[cfg(test)]
    pub(crate) fn target(&self) -> &OpenTarget {
        self.model.target()
    }

    /// Loads the tab again in place, after its repository may have
    /// changed or on Retry. What is shown, and the source's text field
    /// with its selection and focus, stay until the load lands; a load
    /// that gives nothing keeps what is shown and says it could not
    /// refresh.
    pub(crate) fn reload(&mut self, cx: &mut Context<Self>) -> Stamp {
        let stamp = self.model.reload();
        self.read(stamp.clone(), cx);
        stamp
    }

    /// Starts the tab's load under `stamp`, as one read job.
    fn read(&mut self, stamp: Stamp, cx: &mut Context<Self>) {
        let target = self.model.target().clone();
        self.read = Some(self.services.read_job(
            stamp,
            move |reader| read_tab(reader, target),
            cx,
            |this: &mut Self, stamp, reads, cx| {
                if this.model.apply_reads(stamp, reads) {
                    cx.notify();
                }
            },
        ));
        cx.notify();
    }

    pub(super) fn repo(&self) -> &str {
        match self.model.target() {
            OpenTarget::Item { repo, .. } | OpenTarget::Inspect { repo, .. } => repo,
        }
    }

    /// Refresh: refreshes the repository's index through the worker; the
    /// item is read again when that ends.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        if self.refreshing.is_some() {
            return;
        }
        let root = PathBuf::from(self.repo());
        let id =
            self.services
                .worker()
                .submit(Intent::IndexRefresh { root }, Lane::User, None, None);
        self.refreshing = Some(id);
        cx.notify();
    }

    /// A worker event arrived: once the refresh Refresh asked for has
    /// ended, however it ended, the item is read again. A finished one
    /// marked the repository's items changed, so the workspace reads every
    /// open tab of it again, this one with them; one not sent, or removed
    /// from the queue by a cancel or on quit, changed nothing, so this
    /// view reads itself again, in place. A request the model does not
    /// hold yet has not ended: its `Queued` event comes through the
    /// worker's channel after `submit` returns, and another event can
    /// reach the model first.
    fn operations_changed(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.refreshing else {
            return;
        };
        let state = self
            .services
            .operations()
            .read(cx)
            .request(id)
            .map(|request| request.state.clone());
        match state {
            None | Some(RequestState::Queued | RequestState::Running { .. }) => {}
            Some(RequestState::Finished(..)) => {
                self.refreshing = None;
                cx.notify();
            }
            Some(_) => {
                self.refreshing = None;
                self.reload(cx);
            }
        }
    }

    /// The tab's label: the title the item was last read with, kept when
    /// a later load fails, else its ID, or the path of a nonconforming
    /// entry.
    pub(crate) fn tab_title(&self) -> String {
        if let Some(title) = self.model.last_title() {
            return title.to_owned();
        }
        match self.model.target() {
            OpenTarget::Item { id, .. } => id.to_string(),
            OpenTarget::Inspect { path, .. } => path.clone(),
        }
    }
}

/// A piece of text a test reads by `id`.
pub(super) fn text(
    id: impl Into<ElementId>,
    value: impl Into<String>,
) -> impl IntoElement + Styled + InteractiveElement {
    let value = value.into();
    div()
        .id(id)
        .test_support()
        .aria_label(value.clone())
        .child(value)
}

/// A named field: its name, then its value.
fn field(
    id: &'static str,
    name: &'static str,
    value: String,
    cx: &App,
) -> impl IntoElement + use<> {
    h_flex()
        .gap_2()
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .w(px(120.))
                .child(name),
        )
        .child(text(id, value).text_sm())
}

fn kind_label(kind: ItemDtoKind) -> &'static str {
    match kind {
        ItemDtoKind::Document => "Document",
        ItemDtoKind::Ticket => "Ticket",
    }
}

impl ItemPane {
    /// A state that is not simply ready, in text; a failure by its code's
    /// message and its code, with Refresh for an item that is gone and
    /// Retry otherwise.
    pub(super) fn status<T: Rows>(
        &self,
        name: &'static str,
        state: &ListState<T>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<T> {
        let muted = cx.theme().muted_foreground;
        let label = match state {
            ListState::Ready { index, .. }
                if index.as_ref().is_none_or(|index| {
                    index.state == manyhands::repository::IndexState::Current
                }) =>
            {
                None
            }
            _ => Some(
                text(format!("{name}-state"), state.label())
                    .text_xs()
                    .text_color(muted),
            ),
        };
        let failure = match state {
            ListState::Failed(failure) => Some(failure.code),
            _ => None,
        };
        let failed = failure.map(|code| {
            v_flex()
                .gap_1()
                .child(text(format!("{name}-failure"), code.message()).text_sm())
                .child(
                    text(format!("{name}-failure-code"), code.as_str())
                        .text_xs()
                        .text_color(muted),
                )
        });
        let action = match (state, failure) {
            (_, Some(ResultCode::ItemNotFound)) => Some(match self.refreshing {
                Some(_) => div()
                    .id(format!("{name}-refreshing"))
                    .test_support()
                    .aria_label(REFRESHING)
                    .text_sm()
                    .child(REFRESHING),
                None => div().id(format!("{name}-refresh")).test_support().child(
                    Button::new(format!("{name}-refresh-button"))
                        .label(REFRESH)
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.refresh(cx))),
                ),
            }),
            (ListState::Busy | ListState::Failed(_), _) => Some(
                div().id(format!("{name}-retry")).test_support().child(
                    Button::new(format!("{name}-retry-button"))
                        .label(RETRY)
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.reload(cx);
                        })),
                ),
            ),
            _ => None,
        };
        v_flex()
            .gap_1()
            .children(label)
            .children(failed)
            .children(action)
    }

    /// A load that gave nothing while rows were shown: they stay, and
    /// this says so, in fixed text, with the answer's message and code,
    /// and Retry, which loads again in place.
    fn not_refreshed(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        let code = match self.model.not_refreshed()? {
            NotReloaded::Busy => ResultCode::Busy,
            NotReloaded::Failed(failure) => failure.code,
        };
        let muted = cx.theme().muted_foreground;
        Some(
            v_flex()
                .id("item-not-refreshed")
                .test_support()
                .aria_label(NOT_REFRESHED)
                .gap_1()
                .p_2()
                .border_1()
                .border_color(cx.theme().border)
                .child(div().text_sm().child(NOT_REFRESHED))
                .child(text("item-not-refreshed-failure", code.message()).text_sm())
                .child(
                    text("item-not-refreshed-code", code.as_str())
                        .text_xs()
                        .text_color(muted),
                )
                .child(
                    div().id("item-not-refreshed-retry").test_support().child(
                        Button::new("item-not-refreshed-retry-button")
                            .label(RETRY)
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.reload(cx);
                            })),
                    ),
                ),
        )
    }

    /// What was opened, from the target until the read gives more: the
    /// kind and ID, or the path of a nonconforming entry.
    fn identity(&self, item: Option<&ItemDto>, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let muted = cx.theme().muted_foreground;
        let mut header = v_flex().gap_1();
        let (kind, id, path) = match self.model.target() {
            OpenTarget::Item { kind, id, .. } => (
                Some(kind_label(*kind)),
                Some(id.to_string()),
                item.map(|item| item.path.clone()),
            ),
            OpenTarget::Inspect { path, .. } => (None, None, Some(path.clone())),
        };
        let mut line = h_flex().gap_2();
        line = match kind {
            Some(kind) => line.child(text("item-kind", kind).text_xs().text_color(muted)),
            None => line.child(
                text("item-nonconforming", NONCONFORMING_FILE)
                    .text_xs()
                    .text_color(muted),
            ),
        };
        if let Some(title) = item.and_then(|item| item.title.clone()) {
            line = line.child(text("item-title", title).text_lg());
        }
        header = header.child(line);
        let slug = item.and_then(|item| item.slug.clone());
        let mut codes = h_flex().gap_2();
        if let Some(slug) = slug.clone() {
            codes = codes
                .child(text("item-slug", slug.clone()).text_sm())
                .child(
                    div()
                        .id("copy-slug")
                        .test_support()
                        .aria_label(COPY_SLUG)
                        .track_focus(&self.copy_focus[0])
                        .child(
                            Button::new("copy-slug-button")
                                .ghost()
                                .label(COPY_SLUG)
                                .on_click(move |_, _, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(slug.clone()))
                                }),
                        ),
                );
        }
        if let Some(id) = id {
            let copied = id.clone();
            codes = codes
                .child(text("item-id", id).text_sm().text_color(muted))
                .child(
                    div()
                        .id("copy-id")
                        .test_support()
                        .aria_label(COPY_ID)
                        .track_focus(&self.copy_focus[1])
                        .child(
                            Button::new("copy-id-button")
                                .ghost()
                                .label(COPY_ID)
                                .on_click(move |_, _, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(copied.clone()))
                                }),
                        ),
                );
        }
        header = header.child(codes);
        if let Some(path) = path {
            header = header.child(text("item-path", path).text_xs().text_color(muted));
        }
        header
    }

    /// Provenance, or in its place the recovery view: no context is
    /// offered when the item's own needs repair.
    fn provenance(&self, item: &ItemDto, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let problems = context_problems(item);
        if !problems.is_empty() {
            return v_flex()
                .id("item-recovery")
                .test_support()
                .aria_label(RECOVERY)
                .gap_1()
                .p_2()
                .border_1()
                .border_color(cx.theme().danger)
                .child(div().text_sm().child(RECOVERY))
                .children(problems.into_iter().enumerate().map(|(ix, problem)| {
                    v_flex()
                        .child(text(("recovery-problem", ix), problem.code.as_str()).text_sm())
                        .child(
                            text(("recovery-guidance", ix), problem.guidance())
                                .text_xs()
                                .text_color(muted),
                        )
                        .children(problem.path.clone().map(|path| {
                            text(("recovery-path", ix), path)
                                .text_xs()
                                .text_color(muted)
                        }))
                }))
                .into_any_element();
        }
        let context = &item.context;
        v_flex()
            .id("provenance-section")
            .gap_1()
            .child(
                text("provenance", provenance_label(context))
                    .text_xs()
                    .text_color(muted),
            )
            .child(field(
                "provenance-kind",
                "Context",
                context_kind_label(context.kind).to_owned(),
                cx,
            ))
            .children(
                context
                    .branch
                    .clone()
                    .map(|branch| field("provenance-branch", "Branch", branch, cx)),
            )
            .child(field(
                "provenance-worktree",
                "Worktree",
                context.worktree.clone(),
                cx,
            ))
            .children(
                context
                    .head_oid
                    .clone()
                    .map(|head| field("provenance-head", "Head commit", head, cx)),
            )
            .into_any_element()
    }

    /// The fields Manyhands defines: lifecycle closure, then project
    /// status and the rest, each its own.
    fn fields(&self, item: &ItemDto, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let mut fields = v_flex().gap_1();
        if let Some(closure) = &item.closure {
            let lifecycle = match closure.state {
                ClosureState::Closed => LIFECYCLE_CLOSED,
                _ => LIFECYCLE_OPEN,
            };
            fields = fields.child(field(
                "item-lifecycle",
                "Lifecycle",
                lifecycle.to_owned(),
                cx,
            ));
            if let Some(at) = closure.closed_at.clone() {
                fields = fields.child(field("item-closed-at", "Closed at", at, cx));
            }
            if let Some(by) = closure.closed_by.clone() {
                fields = fields.child(field("item-closed-by", "Closed by", by, cx));
            }
        }
        for (id, name, value) in [
            ("item-status", "Status", &item.status),
            ("item-type", "Type", &item.ticket_type),
            ("item-project", "Project", &item.project),
            ("item-team", "Team", &item.team),
            ("item-changed", "Changed", &item.changed_at),
        ] {
            if let Some(value) = value.clone() {
                fields = fields.child(field(id, name, value, cx));
            }
        }
        fields
    }

    /// Every problem but the context ones the recovery view shows, with
    /// its guidance and path, and for a ticket but its relationship
    /// problems, which its panel lists.
    fn problems(&self, item: &ItemDto, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let muted = cx.theme().muted_foreground;
        v_flex().gap_1().children(
            item.problems
                .iter()
                .filter(|problem| !CONTEXT_PROBLEMS.contains(&problem.code))
                // A ticket's panel lists these under its groups.
                .filter(|problem| {
                    self.model.relationships().is_none()
                        || !RELATIONSHIP_PROBLEMS.contains(&problem.code)
                })
                .enumerate()
                .map(|(ix, problem)| {
                    v_flex()
                        .child(text(("item-problem", ix), problem.code.as_str()).text_sm())
                        .child(
                            text(("item-problem-guidance", ix), problem.guidance())
                                .text_xs()
                                .text_color(muted),
                        )
                        .children(problem.path.clone().map(|path| {
                            text(("item-problem-path", ix), path)
                                .text_xs()
                                .text_color(muted)
                        }))
                })
                .collect::<Vec<_>>(),
        )
    }

    /// Unknown metadata as read-only key and value rows.
    fn metadata(&self, item: &ItemDto, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let muted = cx.theme().muted_foreground;
        v_flex().gap_1().children(
            item.unknown_metadata
                .iter()
                .enumerate()
                .map(|(ix, (key, value))| {
                    let value = metadata_value(value);
                    let shown = value
                        .clone()
                        .unwrap_or_else(|| NOT_REPRESENTABLE.to_owned());
                    let value = text(("meta-value", ix), shown).text_sm();
                    h_flex()
                        .gap_2()
                        .child(
                            text(("meta-key", ix), key.clone())
                                .text_xs()
                                .text_color(muted)
                                .w(px(120.)),
                        )
                        .child(value)
                })
                .collect::<Vec<_>>(),
        )
    }

    /// Gives the read-only text the source the last read gave, making it
    /// when there is none yet. Nothing the user does there reaches the
    /// item: the text rejects every edit, and the view never writes.
    ///
    /// Only a read that gave the item changes it: while a read is in
    /// flight, or after one that failed, the text field and its selection
    /// stay, for the read that next gives the item. One whose file is not
    /// UTF-8 has no text to keep.
    fn sync_source(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self.model.item().rows() else {
            return;
        };
        let Some(source) = item.source.clone() else {
            self.source = None;
            return;
        };
        match &mut self.source {
            Some((shown, _)) if *shown == source => {}
            Some((shown, state)) => {
                state.update(cx, |state, cx| state.set_value(source.clone(), window, cx));
                *shown = source;
            }
            None => {
                let value = source.clone();
                let state = cx.new(|cx| {
                    let mut state = TextareaState::new(window, cx).default_value(value);
                    state.set_readonly(true, cx);
                    state
                });
                self.source = Some((source, state));
            }
        }
    }

    /// The text the source is shown in, for tests.
    #[cfg(test)]
    pub(crate) fn source_state(&self) -> Option<&Entity<TextareaState>> {
        self.source.as_ref().map(|(_, state)| state)
    }

    /// The file's text in a read-only, selectable, monospaced plain-text
    /// field: never rendered, so nothing is loaded and no link is
    /// followed. Or why there is none.
    fn source(&self, item: &ItemDto, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        match (&item.source, &self.source) {
            (Some(_), Some((_, state))) => div()
                .id("item-source")
                .test_support()
                // Tab and shift-Tab move focus on, as from any other
                // control: the text takes no indent, being read-only.
                .capture_action(|_: &IndentInline, window, cx| {
                    window.focus_next(cx);
                    cx.stop_propagation();
                })
                .capture_action(|_: &OutdentInline, window, cx| {
                    window.focus_prev(cx);
                    cx.stop_propagation();
                })
                .child(
                    Textarea::new(state)
                        .readonly(true)
                        .aria_label(SOURCE)
                        .font_family(theme.mono_font_family.clone())
                        .text_sm()
                        .h(px(320.)),
                )
                .relative()
                .child(
                    self.scroll
                        .marker(&gpui_kit::Focusable::focus_handle(state.read(cx), cx)),
                )
                .into_any_element(),
            // The text is made at the next frame.
            (Some(_), None) => div().into_any_element(),
            (None, _) => text("item-source-missing", SOURCE_NOT_UTF8.to_owned())
                .text_sm()
                .text_color(theme.muted_foreground)
                .into_any_element(),
        }
    }

    /// The comments in list order, each indented by its depth.
    fn discussion(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        let comments = self.model.comments()?;
        let muted = cx.theme().muted_foreground;
        let status = self.status("comments", comments, cx);
        let rows = comments
            .rows()
            .map(|list: &CommentListDto| {
                list.items
                    .iter()
                    .enumerate()
                    .map(|(ix, comment)| {
                        let mut entry = v_flex().id(("comment", ix)).test_support().gap_1().child(
                            h_flex()
                                .gap_2()
                                .child(
                                    text(("comment-author", ix), comment_author(comment)).text_sm(),
                                )
                                .children(comment.created_at.clone().map(|at| {
                                    text(("comment-time", ix), at).text_xs().text_color(muted)
                                })),
                        );
                        if let Some(body) = comment.body.clone() {
                            entry = entry.child(text(("comment-body", ix), body).text_sm());
                        }
                        if comment.id.is_none() {
                            entry = entry
                                .child(text(("comment-path", ix), comment.path.clone()).text_xs())
                                .child(
                                    text(
                                        ("comment-problem", ix),
                                        comment
                                            .problems
                                            .iter()
                                            .map(|problem| problem.code.as_str())
                                            .collect::<Vec<_>>()
                                            .join(", "),
                                    )
                                    .text_xs(),
                                );
                        }
                        div()
                            .pl(px(COMMENT_INDENT * comment.depth as f32))
                            .child(entry)
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        Some(
            v_flex()
                .id("discussion")
                .gap_2()
                .child(div().text_sm().child("Discussion"))
                .child(status)
                .children(rows),
        )
    }

    /// The actions of later releases, each shown and unavailable.
    fn later(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let muted = cx.theme().muted_foreground;
        h_flex()
            .flex_wrap()
            .gap_2()
            .children(LATER.iter().enumerate().map(|(ix, action)| {
                v_flex()
                    .child(
                        div()
                            .id(("later-action", ix))
                            .test_support()
                            .aria_label(*action)
                            .child(
                                Button::new(("later-button", ix))
                                    .label(*action)
                                    .disabled(true),
                            ),
                    )
                    .child(
                        text(("later-note", ix), LATER_NOTE)
                            .text_xs()
                            .text_color(muted),
                    )
            }))
    }
}

impl Render for ItemPane {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_source(window, cx);
        let item = self.model.item().rows().cloned();
        let identity = self.identity(item.as_ref(), cx);
        let status = self.status("item", self.model.item(), cx);
        let not_refreshed = self.not_refreshed(cx);
        // A ticket's relationships, beside its fields once it is read.
        let relationships = match &item {
            Some(_) => self.relationships_panel(window, cx),
            None => None,
        };
        if relationships.is_none() {
            // No row is drawn, so none keeps its focus.
            self.row_focus.clear();
        }
        let body = item.as_ref().map(|item| {
            v_flex()
                .gap_3()
                .child(self.provenance(item, cx))
                .child(self.fields(item, cx))
                .children(relationships)
                .child(self.problems(item, cx))
                .child(self.metadata(item, cx))
                .child(self.source(item, cx))
        });
        let discussion = self.discussion(cx);
        v_flex()
            .id("item-view")
            .test_support()
            .gap_3()
            .child(identity)
            .child(status)
            .children(not_refreshed)
            .children(body)
            .children(discussion)
            .child(self.later(cx))
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "item_tests.rs"]
mod tests;
