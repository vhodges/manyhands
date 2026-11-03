//! A composer's comment draft (design, Lane D, Placement): begun on the
//! first keystroke once the item's comment drafts are listed, edited as
//! the text changes, and the item's drafts adopted into their threads'
//! composers from the live listing. Nothing here derives `Debug`.

use std::path::Path;

use gpui_kit::{Context, EntityId, Window};
use manyhands::{
    canonical::ItemId,
    drafts::{CommentTarget, DraftErrorKind, DraftFields, DraftId},
};

use super::{SlotDraft, Thread, thread_of};
use crate::desktop::{
    services::{drafts::comments::AdoptedComment, submissions::SubmissionState},
    view::item::{ItemPane, composer::ComposerKind},
};

impl ItemPane {
    /// Adopts each comment draft of the item the listing shows that this
    /// tab has not met, into its thread's composer when that one holds no
    /// text and no draft. A comment being posted is not adopted, nor a
    /// reply whose comment is not listed.
    pub(super) fn adopt_comment_drafts(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self.item_key() else {
            return;
        };
        if !self.comment_drafts_listed(cx) {
            return;
        }
        let drafts = self.services.drafts().read(cx).comment_drafts(&item);
        let submissions = self.services.submissions().read(cx);
        let mut adopt = Vec::new();
        for draft in drafts {
            if self.composers.known.contains(&draft.id)
                || self.composers.begun.contains(&draft.target.comment_id)
            {
                continue;
            }
            let posting = submissions
                .get(Path::new(&item.repo), &draft.target.comment_id)
                .is_some_and(|post| matches!(post.state, SubmissionState::Posting));
            if posting {
                continue;
            }
            let Some(thread) = self.thread_for(&draft.target) else {
                continue;
            };
            let taken = self.composers.adopting.contains(&thread)
                || self.composers.of_thread(&thread).is_some_and(|slot| {
                    !matches!(slot.draft, SlotDraft::None)
                        || !slot.composer.read(cx).text(cx).is_empty()
                });
            if !taken {
                self.composers.adopting.insert(thread.clone());
                adopt.push((draft.id, thread));
            }
        }
        for (id, thread) in adopt {
            self.composers.known.insert(id);
            let adopting = self
                .services
                .drafts()
                .update(cx, |registry, cx| registry.adopt_comment(id, cx));
            cx.spawn_in(window, async move |pane, cx| {
                let adopted = adopting.await;
                let _ = pane.update_in(cx, |pane, window, cx| {
                    pane.composers.adopting.remove(&thread);
                    if let Ok(adopted) = adopted {
                        pane.comment_adopted(adopted, window, cx);
                    }
                });
            })
            .detach();
        }
    }

    /// The thread a comment of `target` goes in: none for a new comment,
    /// the root of its parent's thread for a reply, while its parent is
    /// listed.
    pub(super) fn thread_for(&self, target: &CommentTarget) -> Option<Thread> {
        match &target.parent_id {
            None => Some(None),
            Some(parent) => {
                let (root, _) = thread_of(self.listed_comments()?, &parent.to_string())?;
                Some(Some(root))
            }
        }
    }

    /// An adopted draft goes in its thread's composer. If that one took
    /// text or a draft meanwhile, or its thread is no longer listed, it
    /// stays held here and is met again: a later frame adopts it, at once
    /// since its session is live, once the thread is free. A reply opens
    /// the reply composer in its thread when none is open, without taking
    /// focus; else it is kept in its thread. A composer it replaces that
    /// had focus hands it to the new one.
    pub(super) fn comment_adopted(
        &mut self,
        adopted: AdoptedComment,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(thread) = self.thread_for(&adopted.target) else {
            self.composers.known.remove(&adopted.id);
            return;
        };
        let replaced = self.composers.of_thread(&thread);
        let taken = replaced.is_some_and(|slot| {
            !matches!(slot.draft, SlotDraft::None) || !slot.composer.read(cx).text(cx).is_empty()
        });
        if taken {
            self.composers.known.remove(&adopted.id);
            return;
        }
        let shown = match (&thread, &self.composers.reply) {
            (None, _) => true,
            (Some(root), Some((at, _))) => at == root,
            (Some(_), None) => false,
        };
        let focused =
            shown && replaced.is_some_and(|slot| slot.composer.read(cx).contains_focus(window, cx));
        let kind = match &adopted.target.parent_id {
            None => ComposerKind::New,
            Some(parent) => {
                let to = self
                    .listed_comments()
                    .and_then(|list| thread_of(list, &parent.to_string()))
                    .map(|(_, name)| name)
                    .unwrap_or_default();
                ComposerKind::Reply { to, me: None }
            }
        };
        let slot = self.slot(
            kind,
            &adopted.body,
            Some(adopted.target),
            SlotDraft::Begun(adopted.id),
            window,
            cx,
        );
        if focused {
            self.composers.focus = Some(slot.composer.clone());
        }
        match thread {
            None => self.composers.new = Some(slot),
            Some(root) => match &self.composers.reply {
                Some((at, _)) if *at != root => {
                    self.composers.kept.insert(root, slot);
                }
                _ => self.composers.reply = Some((root, slot)),
            },
        }
        cx.notify();
    }

    /// The text of `composer` changed: its draft follows. The first change
    /// once the item's comment drafts are listed allocates the comment's
    /// ID and begins its draft.
    pub(super) fn composer_changed(
        &mut self,
        composer: EntityId,
        text: &str,
        cx: &mut Context<Self>,
    ) {
        let listed = self.comment_drafts_listed(cx);
        let Some(item) = self.item_key() else {
            return;
        };
        let drafts = self.services.drafts().clone();
        let Some((thread, slot)) = self.composers.find(composer) else {
            return;
        };
        match slot.draft {
            SlotDraft::Begun(id) => {
                drafts.update(cx, |registry, cx| {
                    registry.edit(
                        id,
                        DraftFields::Comment {
                            body: text.to_owned(),
                        },
                        cx,
                    )
                });
                return;
            }
            SlotDraft::Beginning => return,
            SlotDraft::None | SlotDraft::NotKept(_) if listed => {}
            SlotDraft::None | SlotDraft::NotKept(_) => return,
        }
        if slot.target.is_none() {
            // A reply answers its root: one whose ID cannot be read gets no
            // comment ID, and no draft, rather than becoming a new comment.
            let parent_id = match thread {
                None => None,
                Some(root) => match root.parse() {
                    Ok(root) => Some(root),
                    Err(_) => return,
                },
            };
            slot.target = Some(CommentTarget {
                comment_id: ItemId::generate(),
                parent_id,
            });
        }
        let Some(target) = slot.target.clone() else {
            return;
        };
        slot.draft = SlotDraft::Beginning;
        self.composers.begun.insert(target.comment_id.clone());
        let body = text.to_owned();
        let begin = drafts.update(cx, |registry, cx| {
            registry.begin_comment(item, target, body.clone(), cx)
        });
        cx.spawn(async move |pane, cx| {
            let begun = begin.await;
            let _ = pane.update(cx, |pane, cx| {
                pane.comment_begun(composer, &body, begun, cx)
            });
        })
        .detach();
    }

    /// A composer's draft has begun, holding `body`, or could not be. One
    /// whose composer is gone meanwhile is discarded; newer text is kept.
    pub(super) fn comment_begun(
        &mut self,
        composer: EntityId,
        body: &str,
        begun: Result<DraftId, DraftErrorKind>,
        cx: &mut Context<Self>,
    ) {
        let drafts = self.services.drafts().clone();
        if let Ok(id) = begun {
            self.composers.known.insert(id);
        }
        let Some((_, slot)) = self.composers.find(composer) else {
            if let Ok(id) = begun {
                drafts.update(cx, |registry, cx| registry.discard_comment(id, cx));
            }
            return;
        };
        match begun {
            Ok(id) => {
                slot.draft = SlotDraft::Begun(id);
                let text = slot.composer.read(cx).text(cx);
                if text != body {
                    drafts.update(cx, |registry, cx| {
                        registry.edit(id, DraftFields::Comment { body: text }, cx)
                    });
                }
            }
            Err(kind) => slot.draft = SlotDraft::NotKept(kind),
        }
        cx.notify();
    }
}
