//! Comment drafts: a comment's text kept in the store while it is written,
//! by the item it is of and the thread it goes in.
//!
//! The caller allocates the comment's ID and names it, and its parent, in
//! the target, so a draft that could not be stored still leaves an ID to
//! post with. A comment draft has no base. Its session is D2's: `edit`
//! and flushing are unchanged, and `stored_for`, restore and save never
//! take one.
//!
//! The drafts of an item are known only once the first listing has
//! landed: until then `comment_drafts` lists none, stored or live, and a
//! caller begins a draft only after it has (source finding 17f). A draft
//! a process that is gone left stored is adopted: claimed, with its
//! comment ID, parent and newest text.
//!
//! A comment its post saved retires its draft through
//! `DraftClaim::retire`, at the revision the session held when asked, once
//! any write in flight lands; a store that holds another revision keeps the
//! draft, let go of, for the Drafts page. Re-targeting a draft to a new
//! comment ID (P13) is a retirement and a new `begin_comment`.
//!
//! `AdoptedComment` holds comment text and derives no `Debug`.

use gpui_kit::{Context, Task};
use manyhands::{
    canonical::ItemId,
    drafts::{
        CommentTarget, DraftClaim, DraftError, DraftErrorKind, DraftFields, DraftId, DraftTarget,
        Entry,
    },
    repository::ItemDtoKind,
};

use super::{DraftRegistry, Live, item_kind, off_thread, retire::take};
use crate::desktop::{
    model::{
        drafts::{DraftSession, RecoveryState},
        stamp::RepoKey,
    },
    services::ReadKind,
};

/// An item, by its repository's registration root, its kind and its ID:
/// whose discussion a comment draft is of.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 12.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct ItemKey {
    pub repo: RepoKey,
    pub kind: ItemDtoKind,
    pub id: ItemId,
}

/// A comment draft of an item, stored or live in this process. It holds
/// no text.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 12.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct CommentDraft {
    pub id: DraftId,
    /// The comment's ID, and its parent's when it is a reply.
    pub target: CommentTarget,
    /// This process has its session.
    pub live: bool,
}

/// A comment draft whose session this process holds: its newest text and
/// whether that is safe in the store. It derives no `Debug`.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 12.
pub(crate) struct AdoptedComment {
    pub id: DraftId,
    pub target: CommentTarget,
    pub body: String,
    pub state: RecoveryState,
}

impl ItemKey {
    /// Whether `target` is a comment draft's of this item.
    fn holds(&self, target: &DraftTarget) -> bool {
        target.comment.is_some()
            && target.repository == self.repo
            && target.item_kind == item_kind(self.kind)
            && target.item_id == self.id
    }
}

impl DraftRegistry {
    /// Creates a draft of the comment `target` of `item`, holding `body`,
    /// at revision 1, claimed, and its session. On failure nothing is
    /// stored; the caller keeps the text and the comment ID it chose, and
    /// may post with it.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 12.
    pub(crate) fn begin_comment(
        &mut self,
        item: ItemKey,
        target: CommentTarget,
        body: String,
        cx: &mut Context<Self>,
    ) -> Task<Result<DraftId, DraftErrorKind>> {
        let target = DraftTarget {
            repository: item.repo,
            item_kind: item_kind(item.kind),
            item_id: item.id,
            comment: Some(target),
        };
        self.begin(target, None, DraftFields::Comment { body }, cx)
    }

    /// The comment drafts of `item`, every thread's, live and stored, in
    /// ID order. None until the first listing has landed, nor while the
    /// store is unavailable. A draft another process holds is listed, not
    /// live; adopting it answers `OpenElsewhere`.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 12.
    pub(crate) fn comment_drafts(&self, item: &ItemKey) -> Vec<CommentDraft> {
        if self.unavailable.is_some() || self.listed == 0 {
            return Vec::new();
        }
        let mut drafts: Vec<CommentDraft> = self
            .sessions
            .values()
            .filter_map(|live| {
                let target = live.session.target();
                if !item.holds(target) {
                    return None;
                }
                Some(CommentDraft {
                    id: live.session.id(),
                    target: target.comment.clone()?,
                    live: true,
                })
            })
            .collect();
        let stored = self.listing.rows().into_iter().flatten();
        for listing in stored {
            let Entry::Draft(draft) = &listing.entry else {
                continue;
            };
            if self.current_id(listing.id).is_some() || !item.holds(&draft.target) {
                continue;
            }
            if let Some(target) = draft.target.comment.clone() {
                drafts.push(CommentDraft {
                    id: listing.id,
                    target,
                    live: false,
                });
            }
        }
        drafts.sort_by_key(|draft| draft.id);
        drafts
    }

    /// The comment draft `id` with its session: the live one, or the
    /// stored draft claimed and made a session. A draft that is not a
    /// comment's answers `NotFound`, and its claim is let go of; one gone
    /// since the listing answers `NotFound` and the store is listed again.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 12.
    pub(crate) fn adopt_comment(
        &mut self,
        id: DraftId,
        cx: &mut Context<Self>,
    ) -> Task<Result<AdoptedComment, DraftErrorKind>> {
        if let Some(current) = self.current_id(id) {
            return Task::ready(self.adopted_live(current));
        }
        // A claim is a listing's kin: it waits behind `Drafts`.
        let claim = off_thread(
            &self.backend,
            ReadKind::Drafts,
            move |store| store.claim(id),
            cx,
        );
        cx.spawn(async move |registry, cx| {
            let claimed = claim.await;
            registry
                .update(cx, |registry, cx| registry.adopted(claimed, cx))
                // Gone with the services: the claim is let go of.
                .unwrap_or(Err(DraftErrorKind::Io))
        })
    }

    fn adopted(
        &mut self,
        claimed: Result<DraftClaim, DraftError>,
        cx: &mut Context<Self>,
    ) -> Result<AdoptedComment, DraftErrorKind> {
        let claim = match claimed {
            Ok(claim) => claim,
            Err(error) => {
                if matches!(
                    error.kind,
                    DraftErrorKind::NotFound | DraftErrorKind::OpenElsewhere
                ) {
                    self.reload_listing(cx);
                }
                return Err(error.kind);
            }
        };
        // A claim refused here is dropped, which lets it go.
        let draft = claim.draft();
        if draft.target.comment.is_none() || comment_body(&draft.current).is_none() {
            return Err(DraftErrorKind::NotFound);
        }
        let session = DraftSession::new(claim, cx.background_executor().now());
        let id = session.id();
        self.sessions.insert(
            id,
            Live {
                session,
                timer: None,
                write: None,
                suspended: false,
            },
        );
        self.reload_listing(cx);
        cx.notify();
        self.adopted_live(id)
    }

    /// The live session `id` as an adopted comment.
    fn adopted_live(&self, id: DraftId) -> Result<AdoptedComment, DraftErrorKind> {
        let session = &self
            .sessions
            .get(&id)
            .ok_or(DraftErrorKind::NotFound)?
            .session;
        let target = session
            .target()
            .comment
            .clone()
            .ok_or(DraftErrorKind::NotFound)?;
        let body = comment_body(session.current()).ok_or(DraftErrorKind::NotFound)?;
        Ok(AdoptedComment {
            id,
            target,
            body: body.to_owned(),
            state: session.recovery_state(),
        })
    }

    /// The comment of the draft `id` is saved: its draft retires. A live
    /// session ends now and, once any write in flight lands, its claim
    /// retires the revision the session held; a draft only stored is
    /// claimed and retires the revision it was listed at. A store holding
    /// another revision keeps the draft, let go of. The store is listed
    /// again once it lands, whatever it answered.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 13.
    pub(crate) fn retire_comment(&mut self, id: DraftId, cx: &mut Context<Self>) {
        let live = self
            .current_id(id)
            .and_then(|current| self.sessions.remove(&current));
        let (id, slot, write, saved) = match live {
            Some(Live { session, write, .. }) => (
                session.id(),
                Some(session.claim_slot()),
                write,
                Some(session.revision()),
            ),
            None => (
                id,
                None,
                None,
                self.listed(id).map(|(draft, _)| draft.revision),
            ),
        };
        self.detach_ended();
        cx.notify();
        cx.spawn(async move |registry, cx| {
            if let Some(write) = write {
                write.await;
            }
            let Ok(retire) = registry.update(cx, |registry, cx| {
                off_thread(
                    &registry.backend,
                    ReadKind::DraftWrite,
                    move |store| {
                        let claim = match &slot {
                            Some(slot) => take(slot)?,
                            None => store.claim(id)?,
                        };
                        let saved = saved.unwrap_or(claim.draft().revision);
                        // A kept claim is dropped here, which lets it go.
                        claim.retire(saved).map(|_| ())
                    },
                    cx,
                )
            }) else {
                return;
            };
            // Retired, kept or refused, the draft is listed as it now is.
            let _ = retire.await;
            let _ = registry.update(cx, |registry, cx| {
                registry.reload_listing(cx);
                cx.notify();
            });
        })
        .detach();
    }

    /// Discards the comment draft `id`, whatever its text: through its
    /// session when this process holds one, otherwise from the store.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 12.
    pub(crate) fn discard_comment(&mut self, id: DraftId, cx: &mut Context<Self>) {
        if self.current_id(id).is_some() {
            self.discard(id, cx).detach();
        } else {
            self.discard_stored(id, true, cx).detach();
        }
    }
}

/// A comment's text among its fields.
fn comment_body(fields: &DraftFields) -> Option<&str> {
    match fields {
        DraftFields::Comment { body } => Some(body),
        _ => None,
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "comments_tests.rs"]
mod tests;
