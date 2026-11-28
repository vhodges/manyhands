//! The owner's lifecycle of a pull-request `document promote`, end to end in
//! the library (Wave 06 Cycle 04, Task 7, design P5; acceptance rows A1 to
//! A3). A local bare repository is the forge: plain `git2` merges, squashes,
//! fast-forwards and deletes on it, as the pull request's merge would. The
//! document's branch reaches it through the shared fake publisher (which
//! pushes the context's ref and never the primary), and a poll reads it
//! through `synthetic_fetch`, fed by a real `git2` fetch. Every row asserts
//! that the local primary moved only by the poll's fast-forward, and that no
//! synchronization targeted the primary.
//!
//! The forge is a promotion twin of Cycle 03's test-local `Forge` rather than
//! a lift of it: that one is welded to the closure fixture's ticket (its
//! `PullRequest`, `ticket()` and the ticket's context ref), so lifting it
//! would be a rewrite of it, not a move.
use super::*;
use crate::repository::{
    CancellationToken, Mutation, MutationCall, NoProgress, PollOnceInput, PullRequestDto,
    RefreshOutcome, RefreshRepositoryRequest, RequestId,
};

/// The forge: a bare repository that holds the primary and, once it is
/// published, the document's branch.
struct Forge {
    dir: tempfile::TempDir,
    bare: git2::Repository,
}
impl Forge {
    /// A forge that holds the repository's primary and, when `published`, the
    /// document's branch.
    fn new(f: &Fixture, published: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let bare = git2::Repository::init_bare(dir.path()).unwrap();
        bare.set_head("refs/heads/main").unwrap();
        let forge = Self { dir, bare };
        let mut references = vec!["refs/heads/main".to_owned()];
        if published {
            references.push(context_ref());
        }
        for reference in references {
            f.repo
                .remote_anonymous(&forge.url())
                .unwrap()
                .push(&[format!("{reference}:{reference}")], None)
                .unwrap();
        }
        forge
    }
    fn url(&self) -> String {
        self.dir.path().to_str().unwrap().to_owned()
    }
    fn tip(&self, reference: &str) -> Option<Oid> {
        self.bare.refname_to_id(reference).ok()
    }
    fn primary(&self) -> Oid {
        self.tip("refs/heads/main").unwrap()
    }
    /// A new commit on the forge's primary with the branch's tree.
    fn commit_on_primary(&self, branch: Oid, message: &str, parents: &[Oid]) -> Oid {
        let sig = git2::Signature::now("Forge", "forge@example.invalid").unwrap();
        let tree = self.bare.find_commit(branch).unwrap().tree().unwrap();
        let parents = parents
            .iter()
            .map(|oid| self.bare.find_commit(*oid).unwrap())
            .collect::<Vec<_>>();
        self.bare
            .commit(
                Some("refs/heads/main"),
                &sig,
                &sig,
                message,
                &tree,
                &parents.iter().collect::<Vec<_>>(),
            )
            .unwrap()
    }
    /// The pull request's merge as a merge commit: both heads as parents,
    /// the branch's tree.
    fn merge_commit(&self, branch: Oid) -> Oid {
        self.commit_on_primary(branch, "Merge pull request", &[self.primary(), branch])
    }
    /// The squash: one new commit on the primary, with the branch's tree.
    fn squash(&self, branch: Oid) -> Oid {
        self.commit_on_primary(branch, "Squash pull request", &[self.primary()])
    }
    /// The fast-forward: the primary becomes the branch's tip.
    fn fast_forward(&self, branch: Oid) -> Oid {
        assert!(
            self.bare
                .graph_descendant_of(branch, self.primary())
                .unwrap(),
            "the branch contains the primary"
        );
        self.bare
            .reference("refs/heads/main", branch, true, "forge fast-forward")
            .unwrap();
        branch
    }
    /// The forge deletes the pull request's branch.
    fn delete_branch(&self) {
        self.bare
            .find_reference(&context_ref())
            .unwrap()
            .delete()
            .unwrap();
    }
    /// One fetch of the clone from the forge, pruning what the forge no
    /// longer holds: the tracking refs are the forge's, and the forge's heads
    /// are the advertisement a poll reads.
    fn fetch(&self, repo: &git2::Repository) -> Vec<(String, Oid)> {
        repo.remote_anonymous(&self.url())
            .unwrap()
            .fetch(&["+refs/heads/*:refs/remotes/origin/*"], None, None)
            .unwrap();
        let advertised: Vec<(String, Oid)> = self
            .bare
            .references_glob("refs/heads/*")
            .unwrap()
            .map(|r| {
                let r = r.unwrap();
                (r.name().unwrap().to_owned(), r.target().unwrap())
            })
            .collect();
        for tracking in repo.references_glob("refs/remotes/origin/*").unwrap() {
            let mut tracking = tracking.unwrap();
            let head = tracking
                .name()
                .unwrap()
                .replacen("refs/remotes/origin/", "refs/heads/", 1);
            if !advertised.iter().any(|(name, _)| *name == head) {
                tracking.delete().unwrap();
            }
        }
        advertised
    }
    /// One poll of the clone after a fetch, retiring or not: the envelope.
    fn poll(&self, f: &Fixture, retiring: bool) -> serde_json::Value {
        let advertised = self.fetch(&f.repo);
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        let frozen = f
            .service
            .observation_configuration(f.root.path(), &plan)
            .unwrap();
        state::with_transaction(&f.service, f.root.path(), |tx, rid| {
            state::configure_endpoints(tx, rid, &plan, &frozen.endpoint_digest())
        })
        .unwrap();
        crate::repository::remote::synthetic_fetch(advertised);
        let input = PollOnceInput {
            root: f.root.path().into(),
        };
        let envelope = f.service.execute(
            MutationCall {
                request_id: RequestId::new(),
                confirmation: None,
                mutation: if retiring {
                    Mutation::PollOnceRetiring(input)
                } else {
                    Mutation::PollOnce(input)
                },
            },
            &mut SessionCredentials::new(NoCredentials),
            &mut NoProgress,
            &CancellationToken::new(),
        );
        let value = serde_json::to_value(&envelope).unwrap();
        assert_eq!(value["code"], "ok", "{value}");
        value
    }
}

/// How the forge merges the pull request.
#[derive(Clone, Copy, Debug)]
enum Merge {
    Commit,
    Squash,
    FastForward,
}
impl Merge {
    const ALL: [Merge; 3] = [Merge::Commit, Merge::Squash, Merge::FastForward];
    fn on(self, forge: &Forge, branch: Oid) -> Oid {
        match self {
            Merge::Commit => forge.merge_commit(branch),
            Merge::Squash => forge.squash(branch),
            Merge::FastForward => forge.fast_forward(branch),
        }
    }
}

/// The owner's repository: pull-request mode, the document's branch up to
/// date with the primary (a pull request is made from one), and a forge. A new
/// document is not published: the forge holds the primary alone. For `edit`,
/// the primary already holds the document as it was first saved, and the
/// branch is published with it, with an edit of it that the forge does not
/// have.
fn owner(edit: bool) -> (Fixture, Forge) {
    let f = Fixture::new();
    f.remote();
    f.set_config(PULL_REQUEST_CONFIG);
    let tip = f.tip();
    let held = f.blob_at(tip, DOC);
    if edit {
        write(&f.repo, DOC, &held);
    }
    let primary = commit(&f.repo);
    f.set_tracking_main(primary);
    let uptodate = f.commit_tree(primary, &[(DOC, &held)], &[tip, primary]);
    checkout(&f.context, uptodate);
    if edit {
        f.repo
            .reference(&tracking_context_ref(), uptodate, true, "published")
            .unwrap();
    } else {
        f.unpublish();
    }
    let forge = Forge::new(&f, edit);
    if edit {
        f.edit("an edit after the merge");
    }
    refresh(&f);
    (f, forge)
}

/// The index, refreshed from what the contexts and the primary hold now.
fn refresh(f: &Fixture) {
    let refreshed = f.service.refresh_repository(RefreshRepositoryRequest {
        root: f.root.path().into(),
        operation_id: OperationId::new(),
    });
    assert!(
        matches!(refreshed, Ok(RefreshOutcome::Refreshed { .. })),
        "{refreshed:?}"
    );
}

/// The `pull_request` of `document show`, as the library reads it.
fn shown_link(f: &Fixture) -> Option<PullRequestDto> {
    refresh(f);
    let repository = f.service.resolve_repository(f.root.path()).unwrap();
    f.service
        .show_item(&repository, &ITEM.parse().unwrap())
        .unwrap()
        .pull_request
}

/// What a poll reports of the document's context (a retiring poll's answer
/// holds the poll's own under `poll`).
fn disposition(value: &serde_json::Value) -> String {
    let poll = value["data"].get("poll").unwrap_or(&value["data"]);
    poll["targets"]
        .as_array()
        .and_then(|targets| targets.iter().find(|t| t["item_id"] == ITEM))
        .unwrap_or_else(|| panic!("the document context is a poll target: {value}"))["disposition"]
        .as_str()
        .unwrap()
        .to_owned()
}

/// What no pull-request promote does, with the local primary where the case
/// says it is.
fn assert_never_pushed_or_merged(f: &Fixture, io: &PullRequestIo, primary: Oid, case: &str) {
    assert_eq!(f.primary(), primary, "{case}: the local primary");
    assert_eq!(io.primary_calls, 0, "{case}: the primary was synchronized");
    assert_eq!(io.deletes, 0, "{case}: a branch was deleted");
    assert_eq!(promotion_commits(&f.repo), 0, "{case}: a merge was made");
}

/// A1 to A3, the owner's case, for a new document and for an edit of a merged
/// one, each merged by a merge commit, a squash and a fast-forward: promote
/// gives the link, a poll leaves the document unpromoted, promote again
/// gives the same link, a save is published by the next promote; then the
/// forge merges and deletes the branch, the poll fast-forwards the primary
/// and reports the document promoted, promote's prepare answers
/// `already_applied`, `document show` gives no link, and the retiring poll
/// retires the context and the link row is gone. The local primary moves only
/// by the poll's fast-forward, and the forge's primary only by the forge.
#[test]
fn pull_request_promote_lifecycles() {
    use record::PromotionAnswer as Answer;
    for edit in [false, true] {
        for style in Merge::ALL {
            let case = format!(
                "{} merged by {style:?}",
                if edit {
                    "an edit of a merged document"
                } else {
                    "a new document"
                }
            );
            let case = case.as_str();
            let (f, forge) = owner(edit);
            let (primary_before, forge_primary) = (f.primary(), forge.primary());
            assert_eq!(f.tracking_main(), primary_before, "{case}");
            let mut io = PullRequestIo {
                forge: Some(forge.url()),
                ..PullRequestIo::default()
            };
            let link = |given_at| {
                Some((
                    format!("manyhands/document/{ITEM}"),
                    "main".to_owned(),
                    None,
                    None,
                    given_at,
                ))
            };
            let dto = PullRequestDto {
                source: format!("manyhands/document/{ITEM}"),
                target: "main".to_owned(),
                forge: None,
                url: None,
            };

            // Promote gives the link, and the forge sees the branch.
            let first = branch(run_at(&f, f.request(), &mut io, clock));
            assert_eq!(first.answer(), Answer::Promoted, "{case}");
            let tip = f.tip();
            assert_eq!(
                first.publication(),
                Some(PromotionPublication::Published { oid: tip }),
                "{case}"
            );
            assert_eq!(forge.tip(&context_ref()), Some(tip), "{case}");
            assert_eq!(f.link_row(), link(CLOCK), "{case}");
            assert_eq!(shown_link(&f), Some(dto.clone()), "{case}");

            // A poll leaves the document unpromoted: the context stands.
            let polled = forge.poll(&f, false);
            assert_eq!(disposition(&polled), "current", "{case}: {polled}");
            assert_eq!(f.tip(), tip, "{case}");
            assert!(
                f.root
                    .path()
                    .join(".manyhands/worktrees")
                    .join(ITEM)
                    .exists()
            );
            assert_eq!(f.link_row(), link(CLOCK), "{case}");

            // Promote again gives the same link, renewed.
            let second = branch(run_at(&f, f.request(), &mut io, later));
            assert_eq!(second.answer(), Answer::AlreadyPromoted, "{case}");
            assert_eq!(
                second.publication(),
                Some(PromotionPublication::AlreadyCurrent { oid: tip }),
                "{case}"
            );
            assert_eq!(f.link_row(), link(CLOCK + 86_400), "{case}");
            assert_eq!(f.link_rows(), 1, "{case}");
            assert_eq!(shown_link(&f), Some(dto.clone()), "{case}");

            // A save, and promote again publishes the save.
            let saved = f.edit("a save before the merge");
            let third = branch(run_at(&f, f.request(), &mut io, later));
            assert_eq!(third.answer(), Answer::AlreadyPromoted, "{case}");
            assert_eq!(
                third.publication(),
                Some(PromotionPublication::Published { oid: saved }),
                "{case}"
            );
            assert_eq!(forge.tip(&context_ref()), Some(saved), "{case}");
            assert_eq!(f.link_rows(), 1, "{case}");
            assert_eq!(shown_link(&f), Some(dto.clone()), "{case}");
            // Nothing so far moved either primary.
            assert_never_pushed_or_merged(&f, &io, primary_before, case);
            assert_eq!(
                forge.primary(),
                forge_primary,
                "{case}: only the forge merges"
            );

            // The forge merges by the row's style and deletes the branch. The
            // clone has not looked: it still offers the link.
            let merged = style.on(&forge, saved);
            forge.delete_branch();
            assert_ne!(merged, forge_primary, "{case}");
            assert_eq!(f.primary(), primary_before, "{case}");
            assert_eq!(shown_link(&f), Some(dto.clone()), "{case}");

            // The poll fast-forwards the primary, and reports the document
            // promoted; nothing is retired by it.
            let polled = forge.poll(&f, false);
            assert_eq!(disposition(&polled), "promoted", "{case}: {polled}");
            assert_never_pushed_or_merged(&f, &io, merged, case);
            assert_eq!(f.tracking_main(), merged, "{case}");
            assert!(polled["data"]["retirements"].is_null(), "{case}: {polled}");
            assert_eq!(f.link_rows(), 1, "{case}");

            // Promote's prepare answers `already_applied`, and `document
            // show` gives no link.
            let held = f
                .service
                .preview_document_promotion(PreviewDocumentPromotionRequest {
                    root: f.root.path().into(),
                    item_id: ITEM.parse().unwrap(),
                })
                .unwrap();
            assert!(
                matches!(held, DocumentPromotionPreflight::PrimaryHolds),
                "{case}: {held:?}"
            );
            assert_eq!(shown_link(&f), None, "{case}");
            assert_eq!(f.promotion_rows(), 3, "{case}: prepare wrote no record");

            // The retiring poll retires the context, and the link row is gone.
            let retired = forge.poll(&f, true);
            assert_eq!(disposition(&retired), "promoted", "{case}: {retired}");
            assert_eq!(
                retired["data"]["retirements"][0]["state"], "retired",
                "{case}: {retired}"
            );
            assert!(f.repo.find_reference(&context_ref()).is_err(), "{case}");
            assert!(
                !f.root
                    .path()
                    .join(".manyhands/worktrees")
                    .join(ITEM)
                    .exists(),
                "{case}"
            );
            assert_eq!(f.link_row(), None, "{case}");
            assert_eq!(f.link_rows(), 0, "{case}");
            assert_eq!(shown_link(&f), None, "{case}");
            assert_never_pushed_or_merged(&f, &io, merged, case);
            assert_eq!(forge.primary(), merged, "{case}: the forge's only move");
        }
    }
}
