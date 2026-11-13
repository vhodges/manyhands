//! The titles of what a repository has not shared yet (Cycle 03a, R-9), so
//! a row and its tooltip name an item by its title, not its ID.
//!
//! A title is learned from the Tickets and Documents lists whenever they
//! load. For a named item whose title no list gave, the scheduler's notify
//! starts one read job for its repository: it resolves the repository, then
//! shows each unknown item. A repository whose task is still in `reads` is
//! never given another, since dropping a task cancels its read, so a notify
//! while a read runs starts nothing and costs no try. An item is done once
//! its read finds it or finds it gone; only a busy or failed answer counts a
//! try, at most `TRIES` an item a session, after which it is named by its
//! ID (K-3). Everything goes when a repository does.

use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
};

use gpui_kit::{Context, Task};
use manyhands::{canonical::ItemId, repository::ItemListDto, results::ResultCode};

use super::Workspace;
use crate::desktop::{
    model::{
        repository_state::NAMED_AT_MOST,
        stamp::{Generations, RepoKey, ViewScope},
    },
    services::{ReadKind, ReadResult},
};

/// How many busy or failed reads an item has in a session.
const TRIES: u8 = 3;

/// Where an item's read stands.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tries {
    /// This many busy or failed reads so far.
    Pending(u8),
    /// Found, or found gone: never read again.
    Done,
}

/// What one item's read answered.
enum Answer {
    Found(Option<String>),
    Gone,
    /// Busy or failed: counts one try.
    Retry,
}

type Key = (RepoKey, ItemId);

/// The session's titles, and what is read for more. Keyed by `ItemId`'s
/// order, which it has and a hash it has not.
#[derive(Default)]
pub(super) struct ShareTitles {
    titles: BTreeMap<Key, String>,
    tries: BTreeMap<Key, Tries>,
    /// One running job per repository.
    reads: HashMap<RepoKey, Task<()>>,
    generations: Generations,
}

impl ShareTitles {
    /// Whether a read of `key` is worth making.
    fn unknown(&self, key: &Key) -> bool {
        !self.titles.contains_key(key)
            && match self.tries.get(key) {
                None => true,
                Some(Tries::Pending(tried)) => *tried < TRIES,
                Some(Tries::Done) => false,
            }
    }
}

impl Workspace {
    /// The title of the item `id` of `root`, when it is known.
    pub(in crate::desktop::view) fn share_title(&self, root: &str, id: &ItemId) -> Option<String> {
        self.share_titles
            .titles
            .get(&(root.to_owned(), id.clone()))
            .cloned()
    }

    /// Remembers the titles of the items a list read of `root` gave. A
    /// title is a fact, so a list that is no longer wanted teaches it too.
    pub(super) fn learn_titles(&mut self, root: &RepoKey, list: &ItemListDto) {
        for item in &list.items {
            let (Some(id), Some(title)) = (item.id.as_deref(), &item.title) else {
                continue;
            };
            if let Ok(id) = id.parse::<ItemId>() {
                self.share_titles
                    .titles
                    .insert((root.clone(), id), title.clone());
            }
        }
    }

    /// Forgets every repository not in `roots`: its titles and tries, and
    /// its read, which dropping its task cancels.
    pub(super) fn retain_titles(&mut self, roots: &[RepoKey]) {
        let titles = &mut self.share_titles;
        titles.titles.retain(|(root, _), _| roots.contains(root));
        titles.tries.retain(|(root, _), _| roots.contains(root));
        titles.reads.retain(|root, _| roots.contains(root));
    }

    /// Starts one read job for each listed repository that names an item
    /// with a kind whose title is neither known nor given up, and has no
    /// job running. Main and an item with no kind, which a local conflict
    /// resolution adds, are not read (K-3).
    pub(super) fn read_share_titles(&mut self, cx: &mut Context<Self>) {
        let asks: Vec<(RepoKey, Vec<ItemId>)> = {
            let scheduler = self.services.scheduler().read(cx);
            self.listed()
                .unwrap_or_default()
                .into_iter()
                .filter(|root| !self.share_titles.reads.contains_key(root))
                .filter_map(|root| {
                    let ids: Vec<ItemId> = scheduler
                        .schedule()
                        .repo(&root)?
                        .unshared
                        .items
                        .iter()
                        .take(NAMED_AT_MOST)
                        .filter(|target| !target.main && target.kind.is_some())
                        .filter_map(|target| target.item_id.clone())
                        .filter(|id| self.share_titles.unknown(&(root.clone(), id.clone())))
                        .collect();
                    (!ids.is_empty()).then_some((root, ids))
                })
                .collect()
        };
        for (root, ids) in asks {
            self.read_titles_of(root, ids, cx);
        }
    }

    /// The job of `root`: resolve it, then show each of `ids`, copying the
    /// made-ready reads (D5 A3).
    #[allow(clippy::result_large_err)]
    fn read_titles_of(&mut self, root: RepoKey, ids: Vec<ItemId>, cx: &mut Context<Self>) {
        let stamp = self
            .share_titles
            .generations
            .next(ViewScope::ShareTitles(root.clone()));
        let path = PathBuf::from(&root);
        let landed = root.clone();
        let task = self.services.read_job(
            stamp,
            move |reader| async move {
                let repo = match reader
                    .call(ReadKind::ShareTitles, |service| {
                        service.resolve_repository(&path)
                    })
                    .await
                {
                    ReadResult::Ok(repo) => repo,
                    _ => return ids.into_iter().map(|id| (id, Answer::Retry)).collect(),
                };
                let mut answers = Vec::with_capacity(ids.len());
                for id in ids {
                    let answer = match reader
                        .call(ReadKind::ShareTitles, |service| {
                            service.show_item(&repo, &id)
                        })
                        .await
                    {
                        ReadResult::Ok(item) => Answer::Found(item.title),
                        ReadResult::Failed(failure) if failure.code == ResultCode::ItemNotFound => {
                            Answer::Gone
                        }
                        ReadResult::Busy | ReadResult::Failed(_) => Answer::Retry,
                    };
                    answers.push((id, answer));
                }
                answers
            },
            cx,
            // Whatever the stamp says: a title is a fact (R-9).
            move |this: &mut Self, _, answers, cx| this.apply_titles(landed, answers, cx),
        );
        self.share_titles.reads.insert(root, task);
    }

    /// Merges the answers of the job of `root` by (root, id), and frees the
    /// repository for another job at the next notify.
    fn apply_titles(
        &mut self,
        root: RepoKey,
        answers: Vec<(ItemId, Answer)>,
        cx: &mut Context<Self>,
    ) {
        let titles = &mut self.share_titles;
        titles.reads.remove(&root);
        for (id, answer) in answers {
            let key = (root.clone(), id);
            let tries = match answer {
                Answer::Found(title) => {
                    titles
                        .titles
                        .extend(title.map(|title| (key.clone(), title)));
                    Tries::Done
                }
                Answer::Gone => Tries::Done,
                Answer::Retry => match titles.tries.get(&key) {
                    Some(Tries::Pending(tried)) => Tries::Pending(tried + 1),
                    _ => Tries::Pending(1),
                },
            };
            titles.tries.insert(key, tries);
        }
        cx.notify();
    }
}
