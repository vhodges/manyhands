//! Where the composers are and what their text is kept as (design, Lane D,
//! Placement).
//!
//! - **The new comment's** composer follows the threads, once the item's
//!   comments are read, collapsed to one Write a comment… tab stop until
//!   it is opened or holds text or a draft (decision 22, `start`).
//! - **The reply composer** is one, in the thread whose Reply was pressed,
//!   after its last reply, with focus in it. Its comment answers the
//!   thread's root (decision 5); "Replying to" names the comment Reply was
//!   pressed on. Moved to another thread, it leaves its composer, text and
//!   draft kept in the thread it leaves, and takes the one kept in the
//!   thread it reaches, if any. While it is posting it does not move.
//! - **Drafts.** A composer's comment ID is allocated on the first
//!   keystroke once the item's comment drafts are listed, and its draft
//!   begun with it (`begin_comment`, `drafts`); later text edits that draft. Before
//!   the listing nothing is begun: the text stays in the composer.
//! - **Adoption.** Each comment draft of the item the listing shows that
//!   this tab has not met is adopted into its thread's composer, if that
//!   one holds no text and no draft: as the tab opens, and whenever the
//!   listing changes while it is open, as the Drafts page's Open makes it
//!   (`adopt_comment`). A comment being posted (`Submissions`) is never
//!   adopted. A reply draft whose comment is not listed waits.
//! - **Keeping the caret in view** (R-67): each composer shown has its
//!   `CaretInView` in the item's region, so a composer that grows at the
//!   foot of the page keeps its caret in view.
//!
//! What a composer posts and how its outcome applies are `post`'s, where
//! a post stands `check`'s, and moving a comment to a new ID
//! `retarget`'s (design, Posting). Nothing here derives `Debug`: a slot
//! holds text.

mod actions;
mod check;
mod drafts;
mod held;
mod overtaken;
mod post;
mod retarget;
mod settle;
mod start;
#[cfg(all(test, feature = "desktop-test-support"))]
mod test_access;

#[cfg(all(test, feature = "desktop-test-support"))]
pub(crate) use post::{MAYBE_SAVED, NOT_SAVED_FINAL};
#[cfg(all(test, feature = "desktop-test-support"))]
pub(crate) use retarget::NOT_MOVED;

use std::collections::{BTreeSet, HashMap, HashSet};

use gpui_kit::{
    AnyElement, AppContext, Context, Entity, EntityId, FocusHandle, InteractiveElement,
    IntoElement, ParentElement, Styled, Subscription, TestSupportExt, Window, div, px,
};
use manyhands::{
    canonical::ItemId,
    drafts::{CommentTarget, DraftErrorKind, DraftId, ItemKind},
    repository::{CommentListDto, ItemDtoKind},
};

use super::{Composer, ComposerEvent, ComposerKind, ComposerState};
use crate::desktop::{
    model::{
        discussion::{display_name, threads},
        drafts::RecoveryState,
        list_state::ListState,
        navigation::OpenTarget,
    },
    services::drafts::comments::ItemKey,
    view::{
        editor::CaretInView,
        item::{
            ItemPane, PublicationRemote,
            discussion::{REPLY_INDENT, REPLY_POSTING},
            posting::pending_row,
            text,
            tokens::item_colors,
        },
    },
};

/// The collapsed new-comment composer's words (decision 22).
pub(crate) const WRITE_A_COMMENT: &str = "Write a comment\u{2026}";

/// The tab's composers.
#[derive(Default)]
pub(in crate::desktop::view::item) struct Composers {
    /// The new comment's, once the comments are read.
    new: Option<Slot>,
    /// The reply composer and the ID of the root of the thread it is in.
    reply: Option<(String, Slot)>,
    /// The reply composers of other threads, with their text and drafts,
    /// by their roots' IDs.
    kept: HashMap<String, Slot>,
    /// The comment drafts this tab began or adopted, or tried to adopt:
    /// never adopted again.
    known: HashSet<DraftId>,
    /// The threads a draft is being adopted into: one at a time.
    adopting: HashSet<Thread>,
    /// The comments whose drafts this tab began: a draft of one is its
    /// own, even before its begin lands.
    begun: BTreeSet<ItemId>,
    /// The comment whose Reply opened the reply composer: where focus goes
    /// back when it closes.
    reply_from: Option<String>,
    /// The collapsed new-comment composer's one tab stop (decision 22),
    /// made with the composer.
    start: Option<FocusHandle>,
    /// Write a comment… opened the new comment's composer.
    open: bool,
    /// Focus goes into the new comment's editor once it is drawn.
    focus_new: bool,
    /// Focus goes to Write a comment… as it is drawn.
    focus_start: bool,
    /// A composer that took the place of a focused one: focus goes into
    /// its editor as it is drawn.
    focus: Option<Entity<Composer>>,
    /// Drafts the Drafts page discarded: their composers start again
    /// (`held`).
    discarded: Vec<Vec<DraftId>>,
}

/// One composer and what its text is kept as.
struct Slot {
    composer: Entity<Composer>,
    /// The comment it writes, once its ID is allocated.
    target: Option<CommentTarget>,
    draft: SlotDraft,
    /// Where its posting stands (`post`).
    post: post::SlotPost,
    reveal: CaretInView,
    _events: Subscription,
}

/// Where a slot's draft stands.
enum SlotDraft {
    None,
    /// `begin_comment` is out.
    Beginning,
    Begun(DraftId),
    /// The draft could not be begun: the next keystroke tries again.
    NotKept(DraftErrorKind),
}

/// The thread a composer is in, by its root's ID: none for the new
/// comment's.
type Thread = Option<String>;

impl Composers {
    fn slots(&self) -> impl Iterator<Item = &Slot> {
        self.new
            .iter()
            .chain(self.reply.iter().map(|(_, slot)| slot))
            .chain(self.kept.values())
    }

    /// The slot of `composer`, and its thread.
    fn find(&mut self, composer: EntityId) -> Option<(Thread, &mut Slot)> {
        if let Some(slot) = self.new.as_mut()
            && slot.composer.entity_id() == composer
        {
            return Some((None, slot));
        }
        if let Some((root, slot)) = self.reply.as_mut()
            && slot.composer.entity_id() == composer
        {
            return Some((Some(root.clone()), slot));
        }
        self.kept
            .iter_mut()
            .find(|(_, slot)| slot.composer.entity_id() == composer)
            .map(|(root, slot)| (Some(root.clone()), slot))
    }

    /// The slot of `thread`'s composer, shown or kept.
    fn of_thread(&self, thread: &Thread) -> Option<&Slot> {
        match thread {
            None => self.new.as_ref(),
            Some(root) => match &self.reply {
                Some((at, slot)) if at == root => Some(slot),
                _ => self.kept.get(root),
            },
        }
    }

    /// Whether the reply composer is posting: it does not move.
    pub(in crate::desktop::view::item) fn reply_posting(&self, cx: &gpui_kit::App) -> bool {
        self.reply
            .as_ref()
            .is_some_and(|(_, slot)| slot.composer.read(cx).is_posting())
    }
}

/// The root of the thread holding the comment `id`, and the comment's
/// author's name.
fn thread_of(list: &CommentListDto, id: &str) -> Option<(String, String)> {
    let ix = list
        .items
        .iter()
        .position(|comment| comment.id.as_deref() == Some(id))?;
    let thread = threads(list)
        .into_iter()
        .find(|thread| thread.root == ix || thread.replies.iter().any(|reply| reply.ix == ix))?;
    let root = list.items[thread.root].id.clone()?;
    Some((root, display_name(list.items[ix].author.as_deref())))
}

fn drafts_kind(kind: ItemDtoKind) -> ItemKind {
    match kind {
        ItemDtoKind::Document => ItemKind::Document,
        ItemDtoKind::Ticket => ItemKind::Ticket,
    }
}

impl ItemPane {
    /// The item whose discussion this tab shows; none for a file read by
    /// its path.
    fn item_key(&self) -> Option<ItemKey> {
        match self.model.target() {
            OpenTarget::Item { repo, kind, id } => Some(ItemKey {
                repo: repo.clone(),
                kind: *kind,
                id: id.clone(),
            }),
            OpenTarget::Inspect { .. } => None,
        }
    }

    fn has_remote(&self) -> bool {
        matches!(self.publication_remote, PublicationRemote::Named(_))
    }

    /// The comments as last read, once read.
    fn listed_comments(&self) -> Option<&CommentListDto> {
        self.model.comments().and_then(ListState::rows)
    }

    /// Whether a comment is being written in this tab: a composer holds
    /// text that is not blank or is posting or failed, or its draft is
    /// still being begun (RT-3, RT-6). Apart from the unsaved text the quit
    /// asks count: a composer's text is kept by its own draft.
    pub(in crate::desktop::view::item) fn composing(&self, cx: &gpui_kit::App) -> bool {
        self.composers.slots().any(|slot| {
            matches!(slot.draft, SlotDraft::Beginning) || slot.composer.read(cx).is_writing(cx)
        })
    }

    /// Whether the item's comment drafts are listed: the store listed
    /// once, or unavailable, when none ever will be.
    fn comment_drafts_listed(&self, cx: &gpui_kit::App) -> bool {
        let registry = self.services.drafts().read(cx);
        registry.listings_landed() > 0 || registry.unavailable().is_some()
    }

    /// A composer of `kind` holding `restored`, its events heard here and
    /// its caret kept in the item's region.
    fn slot(
        &mut self,
        kind: ComposerKind,
        restored: &str,
        target: Option<CommentTarget>,
        draft: SlotDraft,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Slot {
        let item = match self.model.target() {
            OpenTarget::Item { kind, .. } => drafts_kind(*kind),
            OpenTarget::Inspect { .. } => ItemKind::Ticket,
        };
        let remote = self.has_remote();
        let composer = cx.new(|cx| Composer::new(kind, item, remote, restored, window, cx));
        let editor = composer.read(cx).editor().clone();
        let reveal = CaretInView::new(&editor, &self.scroll, window, cx);
        let events = cx.subscribe_in(&composer, window, |pane, composer, event, window, cx| {
            pane.composer_event(composer.entity_id(), event, window, cx)
        });
        Slot {
            composer,
            target,
            draft,
            post: post::SlotPost::default(),
            reveal,
            _events: events,
        }
    }

    /// As each frame begins, after the discussion's: the new comment's
    /// composer once the comments are read, the remote and each draft's
    /// state in every composer, the item's comment drafts adopted, and the
    /// caret of a composer that took focus revealed.
    pub(in crate::desktop::view::item) fn sync_composers(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.item_key().is_none() || self.listed_comments().is_none() {
            return;
        }
        if self.composers.new.is_none() {
            let slot = self.slot(ComposerKind::New, "", None, SlotDraft::None, window, cx);
            self.composers.new = Some(slot);
        }
        if self.composers.start.is_none() {
            self.composers.start = Some(cx.focus_handle().tab_stop(true));
        }
        self.follow_new_focus(window, cx);
        if let Some(composer) = self.composers.focus.take() {
            let editor = composer.read(cx).editor().clone();
            editor.update(cx, |editor, cx| editor.focus(window, cx));
        }
        let remote = self.has_remote();
        let retiring = self.retiring(cx);
        let registry = self.services.drafts().read(cx);
        let states: Vec<(Entity<Composer>, Option<RecoveryState>)> = self
            .composers
            .slots()
            .map(|slot| {
                let state = match slot.draft {
                    SlotDraft::Begun(id) => registry.recovery_state(id),
                    SlotDraft::NotKept(kind) => Some(RecoveryState::Failed(kind)),
                    SlotDraft::None | SlotDraft::Beginning => None,
                };
                (slot.composer.clone(), state)
            })
            .collect();
        for (composer, state) in states {
            composer.update(cx, |composer, cx| {
                composer.set_remote(remote, cx);
                composer.set_recovery(state, cx);
                composer.set_locked(retiring, cx);
            });
        }
        self.reset_discarded(window, cx);
        self.adopt_comment_drafts(window, cx);
        self.apply_posts(window, cx);
        self.hold_comment_drafts(cx);
        let composers = &mut self.composers;
        for slot in composers
            .new
            .iter_mut()
            .chain(composers.reply.iter_mut().map(|(_, slot)| slot))
        {
            slot.reveal.note_focus(window, cx);
        }
    }

    /// Reply on the comment `id`: the reply composer in its thread, naming
    /// it, with focus in it. Nothing while the reply composer is posting.
    pub(in crate::desktop::view::item) fn reply_pressed(
        &mut self,
        id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.composers.reply_posting(cx) {
            return;
        }
        let Some((root, name)) = self.listed_comments().and_then(|list| thread_of(list, id)) else {
            return;
        };
        let here = matches!(&self.composers.reply, Some((at, _)) if *at == root);
        if !here {
            // A reply left with no text is not kept, nor its draft.
            if let Some((left, slot)) = self.composers.reply.take() {
                if slot.composer.read(cx).text(cx).trim().is_empty() {
                    self.discard_slot_draft(&slot, cx);
                } else {
                    self.composers.kept.insert(left, slot);
                }
            }
            let slot = match self.composers.kept.remove(&root) {
                Some(slot) => slot,
                None => self.slot(
                    ComposerKind::Reply {
                        to: name.clone(),
                        me: None,
                    },
                    "",
                    None,
                    SlotDraft::None,
                    window,
                    cx,
                ),
            };
            self.composers.reply = Some((root, slot));
        }
        self.composers.reply_from = Some(id.to_owned());
        if let Some((_, slot)) = &self.composers.reply {
            let composer = slot.composer.clone();
            composer.update(cx, |composer, cx| {
                composer.set_replying_to(name, cx);
                composer
                    .editor()
                    .clone()
                    .update(cx, |editor, cx| editor.focus(window, cx));
            });
        }
        cx.notify();
    }

    fn composer_event(
        &mut self,
        composer: EntityId,
        event: &ComposerEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            ComposerEvent::Changed(text) => self.composer_changed(composer, text, cx),
            ComposerEvent::Submit => self.composer_submitted(composer, window, cx),
            ComposerEvent::Cancel => self.composer_closed(composer, false, window, cx),
            ComposerEvent::Discard => self.composer_closed(composer, true, window, cx),
            ComposerEvent::Failed(action) => {
                self.composer_failed_action(composer, *action, window, cx)
            }
        }
    }

    /// The draft of `slot`, if begun, is discarded. One still beginning is
    /// discarded as it lands, since its composer is gone.
    fn discard_slot_draft(&self, slot: &Slot, cx: &mut Context<Self>) {
        if let SlotDraft::Begun(id) = slot.draft {
            self.services
                .drafts()
                .update(cx, |registry, cx| registry.discard_comment(id, cx));
        }
    }

    /// Cancel with no text, or Discard: the draft goes, and the reply
    /// composer closes, focus back on the Reply that opened it. The new
    /// comment's starts again, empty: after Discard open, with focus; after
    /// an empty Cancel collapsed, with focus on Write a comment….
    fn composer_closed(
        &mut self,
        composer: EntityId,
        discard: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((thread, slot)) = self.composers.find(composer) else {
            return;
        };
        if let SlotDraft::Begun(id) = slot.draft {
            self.services
                .drafts()
                .update(cx, |registry, cx| registry.discard_comment(id, cx));
        }
        match thread {
            None => {
                let slot = self.slot(ComposerKind::New, "", None, SlotDraft::None, window, cx);
                self.composers.new = Some(slot);
                self.composers.open = discard;
                self.composers.focus_new = discard;
                self.composers.focus_start = !discard;
            }
            Some(root) => {
                if matches!(&self.composers.reply, Some((at, _)) if *at == root) {
                    self.composers.reply = None;
                    // Focus goes back to the Reply that opened it.
                    let from = self.composers.reply_from.take();
                    if let Some(handle) = from.and_then(|id| self.discussion.reply_focus(&id)) {
                        window.focus(&handle, cx);
                    }
                } else {
                    self.composers.kept.remove(&root);
                }
            }
        }
        cx.notify();
    }

    /// The reply composer, when it is in the thread of the root `root`:
    /// after its last reply, at the reply indent.
    pub(in crate::desktop::view::item) fn reply_composer_in(
        &self,
        root: &str,
        cx: &gpui_kit::App,
    ) -> Option<AnyElement> {
        let (at, slot) = self.composers.reply.as_ref()?;
        if at != root {
            return None;
        }
        let colors = item_colors(cx);
        let pending: Vec<AnyElement> = self
            .pending_posts(Some(root), cx)
            .into_iter()
            .map(|(id, body)| pending_row(&id, body, cx))
            .collect();
        let note = slot.composer.read(cx).is_posting().then(|| {
            text("reply-posting-note", REPLY_POSTING)
                .text_size(px(12.))
                .text_color(colors.page_muted)
        });
        Some(
            div()
                .id("reply-composer")
                .test_support()
                .w_full()
                .border_t_1()
                .border_color(colors.page_border_soft)
                .py(px(12.))
                .pr(px(14.))
                .pl(px(14. + REPLY_INDENT))
                .children(pending)
                .children(note)
                .child(slot.composer.clone())
                .into_any_element(),
        )
    }
}
