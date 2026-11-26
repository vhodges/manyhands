//! The owner's lifecycles of a pull-request repository, end to end in the
//! library (Wave 06 Cycle 03, P13; acceptance rows A1 to A9). A local bare
//! repository is the forge: plain `git2` merges, squashes, fast-forwards
//! and deletes on it, as the pull request's merge would. The ticket's
//! branch reaches it through the lifecycle's own publication (the fake
//! child records what a push leaves; the test pushes the same commits), and
//! a poll reads it through `synthetic_fetch`, fed by a real `git2` fetch.
//! Every test asserts that the local primary did not move through the
//! close, and that no synchronization targeted the primary (A11).
use super::*;
use crate::canonical::Integration;
use crate::repository::{
    CancellationToken, ClosureFilter, ItemDto, Mutation, MutationCall, NoProgress, PathChange,
    PollOnceInput, ReadinessFilter, ReadinessReasonCode as Reason, RequestId,
};

/// The forge: a bare repository that holds the primary, and the ticket's
/// branch once it is published.
struct Forge {
    dir: tempfile::TempDir,
    bare: git2::Repository,
}
impl Forge {
    /// A forge that holds the repository's primary.
    fn new(p: &PullRequest) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let bare = git2::Repository::init_bare(dir.path()).unwrap();
        bare.set_head("refs/heads/main").unwrap();
        let forge = Self { dir, bare };
        forge.push(&p.f.repo, "refs/heads/main");
        forge
    }
    fn url(&self) -> String {
        self.dir.path().to_str().unwrap().to_owned()
    }
    /// A push of the clone's `reference` to the forge, as the publication
    /// leaves it there.
    fn push(&self, repo: &git2::Repository, reference: &str) {
        repo.remote_anonymous(&self.url())
            .unwrap()
            .push(&[format!("{reference}:{reference}")], None)
            .unwrap();
    }
    /// The ticket's branch as the clone holds it.
    fn publish(&self, p: &PullRequest) {
        self.push(&p.f.repo, &context_ref());
    }
    /// A push of `commit` as the ticket's branch: another clone's
    /// publication, whose commits only the forge and that clone hold.
    fn publish_commit(&self, p: &PullRequest, commit: Oid) {
        p.f.repo
            .reference("refs/another-clone", commit, true, "another clone")
            .unwrap();
        p.f.repo
            .remote_anonymous(&self.url())
            .unwrap()
            .push(&[format!("refs/another-clone:{}", context_ref())], None)
            .unwrap();
        p.f.repo
            .find_reference("refs/another-clone")
            .unwrap()
            .delete()
            .unwrap();
    }
    fn tip(&self, reference: &str) -> Option<Oid> {
        self.bare.refname_to_id(reference).ok()
    }
    fn primary(&self) -> Oid {
        self.tip("refs/heads/main").unwrap()
    }
    /// The pull request's merge as a merge commit: both heads as parents,
    /// the branch's tree.
    fn merge_commit(&self, branch: Oid) -> Oid {
        let sig = git2::Signature::now("Forge", "forge@example.invalid").unwrap();
        let tree = self.bare.find_commit(branch).unwrap().tree().unwrap();
        self.bare
            .commit(
                Some("refs/heads/main"),
                &sig,
                &sig,
                "Merge pull request",
                &tree,
                &[
                    &self.bare.find_commit(self.primary()).unwrap(),
                    &self.bare.find_commit(branch).unwrap(),
                ],
            )
            .unwrap()
    }
    /// The squash: one new commit on the primary, with the branch's tree.
    fn squash(&self, branch: Oid) -> Oid {
        let sig = git2::Signature::now("Forge", "forge@example.invalid").unwrap();
        let tree = self.bare.find_commit(branch).unwrap().tree().unwrap();
        self.bare
            .commit(
                Some("refs/heads/main"),
                &sig,
                &sig,
                "Squash pull request",
                &tree,
                &[&self.bare.find_commit(self.primary()).unwrap()],
            )
            .unwrap()
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
    /// A later commit on the primary that edits the ticket's body: work the
    /// primary has and a branch made before it does not.
    fn moves_on(&self, body: &str) -> Oid {
        let sig = git2::Signature::now("Forge", "forge@example.invalid").unwrap();
        let parent = self.bare.find_commit(self.primary()).unwrap();
        let source = String::from_utf8(support::tree_bytes(&self.bare, parent.id(), &ticket()))
            .unwrap()
            .replace("context body", body);
        let mut index = git2::Index::new().unwrap();
        index.read_tree(&parent.tree().unwrap()).unwrap();
        index
            .add(&git2::IndexEntry {
                ctime: git2::IndexTime::new(0, 0),
                mtime: git2::IndexTime::new(0, 0),
                dev: 0,
                ino: 0,
                mode: 0o100644,
                uid: 0,
                gid: 0,
                file_size: source.len() as u32,
                id: self.bare.blob(source.as_bytes()).unwrap(),
                flags: 0,
                flags_extended: 0,
                path: ticket().into_bytes(),
            })
            .unwrap();
        let tree = self
            .bare
            .find_tree(index.write_tree_to(&self.bare).unwrap())
            .unwrap();
        self.bare
            .commit(
                Some("refs/heads/main"),
                &sig,
                &sig,
                "The primary moves on",
                &tree,
                &[&parent],
            )
            .unwrap()
    }
    /// The forge deletes the pull request's branch.
    fn delete_branch(&self) {
        self.bare
            .find_reference(&context_ref())
            .unwrap()
            .delete()
            .unwrap();
    }
    /// One fetch of `repo` from the forge, pruning what the forge no longer
    /// holds: the tracking refs are the forge's, and the forge's heads are
    /// the advertisement a poll reads.
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
    /// One retiring poll of a clone after a fetch: the envelope.
    fn poll_of(
        &self,
        service: &RepositoryService,
        root: &Path,
        repo: &git2::Repository,
    ) -> serde_json::Value {
        let advertised = self.fetch(repo);
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        let frozen = service.observation_configuration(root, &plan).unwrap();
        state::with_transaction(service, root, |tx, rid| {
            state::configure_endpoints(tx, rid, &plan, &frozen.endpoint_digest())
        })
        .unwrap();
        crate::repository::remote::synthetic_fetch(advertised);
        let envelope = service.execute(
            MutationCall {
                request_id: RequestId::new(),
                confirmation: None,
                mutation: Mutation::PollOnceRetiring(PollOnceInput { root: root.into() }),
            },
            &mut SessionCredentials::new(NoCredentials),
            &mut NoProgress,
            &CancellationToken::new(),
        );
        let value = serde_json::to_value(&envelope).unwrap();
        assert_eq!(value["code"], "ok", "{value}");
        value
    }
    /// One retiring poll after a fetch: the envelope.
    fn poll(&self, p: &PullRequest) -> serde_json::Value {
        self.poll_of(&p.f.service, p.f.root(), &p.f.repo)
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

/// The owner's repository: pull-request mode, the ticket's branch up to date
/// with the primary (a pull request is made from one) and published, and a
/// forge that holds both.
fn owner() -> (PullRequest, Forge) {
    prepared(PullRequest::new())
}

/// `owner`, over a repository the test has already set up.
fn prepared(p: PullRequest) -> (PullRequest, Forge) {
    let (tip, primary) = (p.f.context_oid(), p.f.primary());
    let uptodate = {
        let held = support::tree_bytes(&p.f.repo, tip, &ticket());
        let merged =
            p.f.repo
                .find_commit(support::tree_commit(
                    &p.f.repo,
                    primary,
                    &[(ticket().as_str(), Some((&held, 0o100644)))],
                ))
                .unwrap();
        let sig = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
        p.f.repo
            .commit(
                None,
                &sig,
                &sig,
                "Merge main",
                &merged.tree().unwrap(),
                &[
                    &p.f.repo.find_commit(tip).unwrap(),
                    &p.f.repo.find_commit(primary).unwrap(),
                ],
            )
            .unwrap()
    };
    p.hold(uptodate);
    p.refresh();
    let forge = Forge::new(&p);
    forge.publish(&p);
    p.f.repo
        .reference(&tracking_context_ref(), uptodate, true, "published")
        .unwrap();
    (p, forge)
}

/// The owner closes the ticket and publishes it, and a first poll sees the
/// branch: that is what makes its later absence a deletion.
fn closed_and_published(p: &PullRequest, forge: &Forge) -> (CloseTicketRequest, Oid) {
    let (req, closure) = p.close_first_request();
    p.assert_link();
    forge.publish(p);
    assert_eq!(forge.tip(&context_ref()), Some(closure));
    forge.poll(p);
    assert_eq!(p.listed_state(), Some(ClosureState::Closing));
    assert!(p.worktree().exists());
    (req, closure)
}

/// The ticket's closure as `commit` holds it, if it does.
fn closure_on(repo: &git2::Repository, commit: Oid) -> Option<(i64, String)> {
    let ticket = parse_ticket(&support::tree_bytes(repo, commit, &ticket()));
    ticket
        .closed_at
        .map(|at| (at.unix_timestamp(), ticket.closed_by.unwrap()))
}

/// What a poll that retires a merged ticket's context answers.
fn assert_retired_by(value: &serde_json::Value, case: &str) {
    let target = value["data"]["poll"]["targets"]
        .as_array()
        .and_then(|targets| targets.iter().find(|t| t["item_id"] == ID))
        .unwrap_or_else(|| panic!("{case}: the ticket context is a poll target: {value}"));
    assert_eq!(target["disposition"], "closed", "{case}: {value}");
    assert_eq!(
        value["data"]["retirements"][0]["state"], "retired",
        "{case}: {value}"
    );
}

impl PullRequest {
    /// The ticket as `ticket show` reads it.
    fn shown(&self) -> ItemDto {
        let repository = self.f.service.resolve_repository(self.f.root()).unwrap();
        self.f.service.show_item(&repository, &id()).unwrap()
    }
    /// Which tickets a listing of `filter` returns, by ID in ID order.
    fn listed(&self, filter: TicketFilter) -> Vec<String> {
        let repository = self.f.service.resolve_repository(self.f.root()).unwrap();
        let mut ids: Vec<String> = self
            .f
            .service
            .list_tickets(&repository, &filter)
            .unwrap()
            .items
            .iter()
            .filter_map(|item| item.id.clone())
            .collect();
        ids.sort();
        ids
    }
    /// One ticket as a listing reads it.
    fn listed_item(&self, id: &str) -> ItemDto {
        let repository = self.f.service.resolve_repository(self.f.root()).unwrap();
        self.f
            .service
            .list_tickets(&repository, &TicketFilter::default())
            .unwrap()
            .items
            .into_iter()
            .find(|item| item.id.as_deref() == Some(id))
            .expect("the ticket is listed")
    }
    fn nothing_retired(&self) {
        assert!(self.f.repo.find_reference(&context_ref()).is_ok());
        assert!(self.worktree().exists());
    }
}

/// A1 and A2, the owner's case: record the closure, publish it, the forge
/// merges the pull request (as a merge commit, a squash or a fast-forward)
/// and deletes its branch; then a poll finds the ticket closed, retires its
/// context, and the link record is gone.
#[test]
fn lifecycle_ends_closed_and_retired_for_each_merge_style() {
    for style in Merge::ALL {
        let case = format!("{style:?}");
        let (p, forge) = owner();
        assert_eq!(p.listed_state(), Some(ClosureState::Open), "{case}");
        let (req, closure) = closed_and_published(&p, &forge);
        // Recorded and published: the ticket is closing, with its link, and
        // the primary is where it was.
        assert_eq!(p.f.primary(), p.primary_before, "{case}");
        let merged = style.on(&forge, closure);
        forge.delete_branch();
        assert_eq!(
            closure_on(&forge.bare, merged),
            closure_on(&p.f.repo, closure),
            "{case}: the forge's primary holds the closure"
        );
        let value = forge.poll(&p);
        assert_retired_by(&value, &case);
        assert!(p.f.repo.find_reference(&context_ref()).is_err(), "{case}");
        assert!(!p.worktree().exists(), "{case}");
        assert_eq!(p.link(), None, "{case}");
        assert_eq!(p.link_given_at(), None, "{case}");
        assert_eq!(
            p.f.primary(),
            merged,
            "{case}: the poll moved it, nothing else"
        );
        assert_eq!(p.listed_state(), Some(ClosureState::Closed), "{case}");
        assert_eq!(p.stored(&req).step, ClosureStep::Complete, "{case}");
    }
}

/// A3, the owner's case: the clone that recorded the closure reads the
/// ticket `closing` until its own poll brings the forge's merge in, even
/// after a fetch has moved the tracking primary; then it reads `closed`.
#[test]
fn the_recording_clone_reads_closing_until_it_fetches() {
    let (p, forge) = owner();
    let (_, closure) = closed_and_published(&p, &forge);
    // `show` reads the closure, its publication and its link.
    let shown = p.shown();
    let closing = shown.closure.expect("a ticket's closure");
    assert_eq!(closing.state, ClosureState::Closing);
    assert_eq!(
        closing.closed_at.as_deref(),
        Some(record::closed_at_text(clock()).unwrap().as_str())
    );
    assert_eq!(closing.closed_by.as_deref(), Some(CLOSED_BY));
    assert_eq!(
        closing.publication,
        Some(crate::repository::ClosurePublication::Published)
    );
    assert_eq!(
        shown.pull_request.expect("its link").source,
        format!("manyhands/ticket/{ID}")
    );
    // The forge merges and deletes the branch; this clone has not looked.
    let merged = forge.squash(closure);
    forge.delete_branch();
    p.refresh();
    assert_eq!(p.listed_state(), Some(ClosureState::Closing));
    // A fetch moves the tracking refs, and not the local primary, which is
    // what reads: the ticket is still `closing`.
    forge.fetch(&p.f.repo);
    assert_eq!(p.tracking_primary(), merged);
    p.refresh();
    assert_eq!(p.f.primary(), p.primary_before);
    assert_eq!(p.listed_state(), Some(ClosureState::Closing));
    assert_eq!(p.shown().closure.unwrap().state, ClosureState::Closing);
    // The poll brings the merge into the local primary, and the ticket
    // reads `closed`.
    let value = forge.poll(&p);
    assert_retired_by(&value, "poll");
    assert_eq!(p.f.primary(), merged);
    assert_eq!(p.listed_state(), Some(ClosureState::Closed));
    let shown = p.shown();
    assert_eq!(shown.closure.unwrap().state, ClosureState::Closed);
    assert_eq!(shown.pull_request, None);
}

/// A4, the owner's case: the pull request is declined (its branch stays on
/// the forge, unmerged), the owner withdraws the closure, which reopens the
/// ticket on the forge's branch, and records the closure again: a new
/// `closed_at`, which the forge then merges.
#[test]
fn declined_withdrawn_then_recorded_again() {
    let (p, forge) = owner();
    let (_, first) = closed_and_published(&p, &forge);
    let mut io = PullRequestIo::default();
    // Declined: nothing happens on the forge's primary. The withdraw
    // reopens the ticket, publishes its commit, and keeps the link.
    let withdraw = p.withdraw_request();
    let outcome = PullRequest::branch(p.withdraw_at(withdraw.clone(), &mut io, later));
    assert_eq!(outcome.answer(), BranchAnswer::Withdrawn);
    assert_eq!(outcome.recording(), BranchRecording::Withdrawn);
    let withdrawn = p.f.context_oid();
    assert_eq!(outcome.checkpoint_oid(), Some(withdrawn));
    assert_eq!(closure_on(&p.f.repo, withdrawn), None);
    assert_eq!(p.listed_state(), Some(ClosureState::Open));
    assert_eq!(p.shown().closure.unwrap().publication, None);
    assert_eq!(p.link_given_at(), Some(CLOCK));
    forge.publish(&p);
    assert_eq!(forge.tip(&context_ref()), Some(withdrawn));
    assert!(p.f.repo.graph_descendant_of(withdrawn, first).unwrap());
    forge.poll(&p);
    assert_eq!(p.listed_state(), Some(ClosureState::Open));
    p.nothing_retired();
    // Recorded again, a day later: a new closure over the withdrawal.
    let again = p.request();
    let outcome = PullRequest::branch(p.run_at(again.clone(), &mut io, later));
    assert_eq!(outcome.answer(), BranchAnswer::Published);
    assert_eq!(outcome.recording(), BranchRecording::Recorded);
    let second = p.f.context_oid();
    assert_eq!(
        p.f.repo.find_commit(second).unwrap().parent_id(0).unwrap(),
        withdrawn
    );
    assert_eq!(p.stored(&again).closed_at, Some(later()));
    assert_eq!(
        closure_on(&p.f.repo, second),
        Some((later().unix_timestamp(), CLOSED_BY.to_owned()))
    );
    assert_ne!(closure_on(&p.f.repo, second), closure_on(&p.f.repo, first));
    assert_eq!(p.listed_state(), Some(ClosureState::Closing));
    assert_eq!(p.link_given_at(), Some(later().unix_timestamp()));
    p.assert_primary_untouched(&io);
    // The forge merges this one, and the ticket ends closed with it.
    forge.publish(&p);
    forge.poll(&p);
    let merged = forge.squash(second);
    forge.delete_branch();
    let value = forge.poll(&p);
    assert_retired_by(&value, "recorded again");
    assert_eq!(p.f.primary(), merged);
    assert_eq!(p.listed_state(), Some(ClosureState::Closed));
    assert_eq!(p.link(), None);
}

/// A5, the owner's case: while the closure is provisional a comment and a
/// save are allowed and leave it as recorded, and a save whose file changes
/// the closure is refused with `invalid_input`; the forge's merge still
/// ends the ticket closed with the closure recorded.
#[test]
fn comments_and_saves_while_provisional() {
    use crate::repository::{
        AuthoringTarget, ContextIntent, ExpectedPathObservation, LocalCommentCheckpointOutcome,
        SaveTicketRequest, SubmitCommentRequest, TicketDraft,
    };
    let (p, forge) = owner();
    let (_, closure) = closed_and_published(&p, &forge);
    let recorded = closure_on(&p.f.repo, closure).unwrap();
    let target = || AuthoringTarget {
        root: p.f.root().into(),
        kind: AuthoringKind::Ticket,
        item_id: id(),
        intent: ContextIntent::Edit,
        operation_id: OperationId::new(),
    };
    let file = p.worktree().join(ticket());
    // A comment on the closing ticket is checkpointed on its branch.
    let comment =
        p.f.service
            .checkpoint_comment_locally(
                SubmitCommentRequest {
                    target: target(),
                    comment_id: "01ARZ3NDEKTSV4RRFFQ69G5FB0".parse().unwrap(),
                    parent_id: None,
                    body: "A comment while closing.\n".into(),
                    expected_destination: ExpectedPathObservation::Missing,
                },
                None,
            )
            .unwrap();
    assert!(matches!(
        comment,
        LocalCommentCheckpointOutcome::Saved { .. }
    ));
    let commented = p.f.context_oid();
    assert_ne!(commented, closure);
    assert_eq!(closure_on(&p.f.repo, commented), Some(recorded.clone()));
    // A save of the title keeps the closure the branch holds.
    let save = |title: &str| {
        let source = fs::read(&file).unwrap();
        p.f.service.save_ticket(SaveTicketRequest {
            target: target(),
            draft: TicketDraft {
                title: title.into(),
                ticket_type: "task".into(),
                status: "open".into(),
                project: None,
                team: None,
                body: "context body\n".into(),
            },
            expected_path: ExpectedPathObservation::from_bytes(&source),
        })
    };
    save("Retitled while closing").unwrap();
    let saved = p.f.context_oid();
    assert_ne!(saved, commented);
    assert_eq!(closure_on(&p.f.repo, saved), Some(recorded.clone()));
    assert_eq!(p.listed_state(), Some(ClosureState::Closing));
    // The branch is ahead of the forge by the comment and the save, and
    // the closure on the forge is still the one it holds: `show` reads the
    // closure's publication, not the branch's.
    p.refresh();
    assert_ne!(forge.tip(&context_ref()), Some(saved));
    assert_eq!(
        p.shown().closure.unwrap().publication,
        Some(crate::repository::ClosurePublication::Published)
    );
    // A file that changes the closure is refused and nothing is committed.
    let source = fs::read_to_string(&file).unwrap();
    let edited = source.replace("closed_by: ", "closed_by: Someone Else ");
    assert_ne!(edited, source);
    fs::write(&file, &edited).unwrap();
    let error = save("Retitled again").err().expect("the save was refused");
    assert_eq!(
        crate::repository::mutation::repository_error_code(error.kind),
        crate::results::ResultCode::InvalidInput
    );
    assert_eq!(p.f.context_oid(), saved);
    fs::write(&file, &source).unwrap();
    // The branch, with the comment and the save, is published and merged.
    forge.publish(&p);
    forge.poll(&p);
    assert_eq!(forge.tip(&context_ref()), Some(saved));
    let merged = forge.merge_commit(saved);
    forge.delete_branch();
    assert_eq!(closure_on(&forge.bare, merged), Some(recorded));
    let value = forge.poll(&p);
    assert_retired_by(&value, "provisional");
    assert_eq!(p.listed_state(), Some(ClosureState::Closed));
    assert_eq!(p.f.primary(), merged);
}

/// A6, the owner's case: two clones record a closure; the other clone
/// publishes first. This clone's publication is rejected, and its retry
/// fetches, keeps the closure that is published, drops its own and
/// publishes: `kept_published`, which reads `existing`. The forge's merge
/// then closes the ticket with the other clone's closure.
#[test]
fn two_clones_record_and_the_later_keeps_the_published_closure() {
    let (p, forge) = owner();
    let req = p.request();
    let mut io = PullRequestIo::default();
    // This clone records its closure, and its publication stops.
    stopped_at_publication(&p, &req, &mut io, SynchronizationError::PushRejected);
    let ours = p.f.context_oid();
    let own = closure_on(&p.f.repo, ours).unwrap();
    assert_eq!(own, (CLOCK, CLOSED_BY.to_owned()));
    assert_eq!(p.listed_state(), Some(ClosureState::Closing));
    assert_eq!(
        p.shown().closure.unwrap().publication,
        Some(crate::repository::ClosurePublication::Pending)
    );
    // The other clone closed the ticket over the same branch first, and
    // published it.
    let tip = p.f.repo.find_commit(ours).unwrap().parent_id(0).unwrap();
    let theirs = p.closed_commit_on(tip, 1_700_000_000, "Remote <remote@example.invalid>");
    forge.publish_commit(&p, theirs);
    assert_eq!(forge.tip(&context_ref()), Some(theirs));
    // The retry fetches what is published, and its merge keeps it.
    forge.fetch(&p.f.repo);
    io.drive = Some((ChildRole::Publication, theirs));
    let outcome = PullRequest::branch(p.run(req.clone(), &mut io));
    assert_eq!(outcome.answer(), BranchAnswer::KeptPublished);
    assert_eq!(outcome.recording(), BranchRecording::Existing);
    assert_eq!(outcome.closed_at(), None);
    let kept = p.f.context_oid();
    let r = p.stored(&req);
    assert_eq!(r.step, ClosureStep::Complete);
    assert_eq!(r.answer_code, Some(BranchAnswer::KeptPublished));
    assert!(p.f.repo.graph_descendant_of(kept, theirs).unwrap());
    assert_eq!(
        closure_on(&p.f.repo, kept),
        Some((1_700_000_000, "Remote <remote@example.invalid>".to_owned()))
    );
    assert_eq!(p.listed_state(), Some(ClosureState::Closing));
    p.assert_link();
    p.assert_primary_untouched(&io);
    // It publishes the merge over what the other clone published: the
    // closure at the forge's branch is the other clone's, not its own.
    forge.publish(&p);
    assert_eq!(forge.tip(&context_ref()), Some(kept));
    assert_eq!(
        closure_on(&forge.bare, kept),
        closure_on(&forge.bare, theirs)
    );
    assert_ne!(closure_on(&forge.bare, kept), Some(own));
    assert_eq!(
        p.shown().closure.unwrap().publication,
        Some(crate::repository::ClosurePublication::Published)
    );
    forge.poll(&p);
    let merged = forge.squash(kept);
    forge.delete_branch();
    let value = forge.poll(&p);
    assert_retired_by(&value, "two clones");
    assert_eq!(
        closure_on(&p.f.repo, p.f.primary()),
        Some((1_700_000_000, "Remote <remote@example.invalid>".to_owned()))
    );
    assert_eq!(p.f.primary(), merged);
    assert_eq!(p.listed_state(), Some(ClosureState::Closed));
    assert_eq!(p.link(), None);
}

/// A7, the owner's case: a ticket that depends on the closing one stays
/// blocked (`closing_dependency`) until the primary holds the closure; the
/// closing ticket is open for it and for `--closure open`, and `ticket
/// ready`, `blocked`, `plan` and `critical-path` leave it out (decision 9).
#[test]
fn a_dependent_waits_for_the_primary() {
    let mut p = PullRequest::new();
    p.add_dependent();
    let (p, forge) = prepared(p);
    let repository = p.f.service.resolve_repository(p.f.root()).unwrap();
    let ready = ReadinessFilter::Ready;
    let blocked = ReadinessFilter::Blocked;
    let with = |readiness| TicketFilter {
        readiness: Some(readiness),
        ..TicketFilter::default()
    };
    let closure = |closure| TicketFilter {
        closure,
        ..TicketFilter::default()
    };
    let in_plan = |p: &PullRequest| -> Vec<(u64, String)> {
        let plan =
            p.f.service
                .ticket_plan(&repository, &TicketFilter::default())
                .unwrap();
        let mut found: Vec<_> = plan
            .batches
            .iter()
            .flat_map(|batch| {
                batch
                    .items
                    .iter()
                    .map(move |item| (batch.batch, item.id.clone().unwrap()))
            })
            .collect();
        found.extend(
            plan.unplannable
                .iter()
                .map(|u| (0, u.ticket.id.clone().unwrap())),
        );
        found
    };
    let on_path = |p: &PullRequest| -> Vec<String> {
        let mut ids: Vec<String> =
            p.f.service
                .ticket_critical_path(&repository)
                .unwrap()
                .items
                .iter()
                .filter_map(|item| item.id.clone())
                .collect();
        ids.sort();
        ids
    };
    // Open, the ticket holds its dependent back and is itself ready.
    assert_eq!(p.dependent_reasons(), [Reason::OpenDependency]);
    assert_eq!(p.listed(with(ready)), [ID]);
    assert_eq!(p.listed(with(blocked)), [DEPENDENT]);
    assert_eq!(in_plan(&p), [(1, ID.into()), (2, DEPENDENT.into())]);
    assert_eq!(on_path(&p), [ID, DEPENDENT]);
    // Recorded: the ticket is closing, which is open for its dependent
    // (blocked by `closing_dependency`) and for `--closure open`, and the
    // queries leave it out; each batch keeps its number.
    let (_, closure_commit) = closed_and_published(&p, &forge);
    assert_eq!(p.dependent_reasons(), [Reason::ClosingDependency]);
    assert_eq!(p.listed(closure(ClosureFilter::Open)), [ID, DEPENDENT]);
    assert_eq!(p.listed(closure(ClosureFilter::Closing)), [ID]);
    assert_eq!(
        p.listed(closure(ClosureFilter::Closed)),
        Vec::<String>::new()
    );
    assert_eq!(p.listed(with(ready)), Vec::<String>::new());
    assert_eq!(p.listed(with(blocked)), [DEPENDENT]);
    assert_eq!(in_plan(&p), [(2, DEPENDENT.into())]);
    assert!(!on_path(&p).contains(&ID.to_owned()), "{:?}", on_path(&p));
    let dependency = p
        .listed_item(DEPENDENT)
        .deps
        .first()
        .map(|dependency| dependency.state);
    assert_eq!(
        dependency,
        Some(crate::repository::DependencyState::Closing)
    );
    // The forge merges and deletes; a fetch moves the tracking primary and
    // the dependent still waits for the local one.
    let merged = forge.squash(closure_commit);
    forge.delete_branch();
    forge.fetch(&p.f.repo);
    p.refresh();
    assert_eq!(p.dependent_reasons(), [Reason::ClosingDependency]);
    assert_eq!(p.listed(with(ready)), Vec::<String>::new());
    // The poll brings the primary's closure in: the dependent is ready.
    let value = forge.poll(&p);
    assert_retired_by(&value, "dependent");
    assert_eq!(p.f.primary(), merged);
    assert_eq!(p.dependent_reasons(), []);
    assert_eq!(p.listed(with(ready)), [DEPENDENT]);
    assert_eq!(p.listed(with(blocked)), Vec::<String>::new());
    assert_eq!(p.listed(closure(ClosureFilter::Open)), [DEPENDENT]);
    assert_eq!(p.listed(closure(ClosureFilter::Closed)), [ID]);
    assert_eq!(in_plan(&p), [(1, DEPENDENT.into())]);
    assert_eq!(on_path(&p), [DEPENDENT]);
}

/// A9, the merged-first branch with its remote branch kept: the forge
/// merged the pull request (as a merge commit, a squash or a fast-forward)
/// and left the branch. Close takes the ordinary path on that branch: the
/// context child brings the primary in, and the closure is then the only
/// change against the primary. The second pull request merges it, and the
/// poll closes and retires the ticket.
#[test]
fn merged_first_with_the_remote_branch_kept_closes_in_place() {
    for style in Merge::ALL {
        let case = format!("{style:?}");
        let (p, forge) = owner();
        let branch = p.f.context_oid();
        forge.poll(&p);
        let first = style.on(&forge, branch);
        // A fetch (the poll's) brings the merge in: the branch survives.
        forge.fetch(&p.f.repo);
        assert_eq!(p.tracking_primary(), first, "{case}");
        assert_eq!(forge.tip(&context_ref()), Some(branch), "{case}");
        p.refresh();
        assert_eq!(p.listed_state(), Some(ClosureState::Open), "{case}");
        // The ordinary close is offered, on the branch.
        let preview = p.ready();
        assert_eq!(
            preview.path(),
            (Integration::PullRequest, ClosureAction::Close),
            "{case}"
        );
        let req = p.confirming(preview.expectation());
        let mut io = PullRequestIo {
            drive: Some((ChildRole::Context, branch)),
            decide_push: true,
            ..PullRequestIo::default()
        };
        let outcome = PullRequest::branch(p.run(req.clone(), &mut io));
        assert_eq!(outcome.answer(), BranchAnswer::Published, "{case}");
        assert_eq!(outcome.action(), ClosureAction::Close, "{case}");
        assert_eq!(outcome.recording(), BranchRecording::Recorded, "{case}");
        let closure = p.f.context_oid();
        // The only change against the primary is the closure: the branch's
        // ticket without it is the primary's.
        let diff =
            p.f.repo
                .diff_tree_to_tree(
                    Some(&p.f.repo.find_commit(first).unwrap().tree().unwrap()),
                    Some(&p.f.repo.find_commit(closure).unwrap().tree().unwrap()),
                    None,
                )
                .unwrap();
        let changed: Vec<_> = diff
            .deltas()
            .map(|delta| delta.new_file().path().unwrap().to_owned())
            .collect();
        assert_eq!(changed, [std::path::PathBuf::from(ticket())], "{case}");
        assert_eq!(
            observe::withdrawn_ticket_bytes(&support::tree_bytes(&p.f.repo, closure, &ticket()))
                .unwrap(),
            support::tree_bytes(&p.f.repo, first, &ticket()),
            "{case}"
        );
        assert!(
            p.f.repo.graph_descendant_of(closure, first).unwrap(),
            "{case}"
        );
        assert_eq!(p.listed_state(), Some(ClosureState::Closing), "{case}");
        p.assert_link();
        p.assert_primary_untouched(&io);
        // The second pull request: published, merged by fast-forward, and
        // the branch deleted.
        forge.publish(&p);
        forge.poll(&p);
        let merged = forge.fast_forward(closure);
        forge.delete_branch();
        let value = forge.poll(&p);
        assert_retired_by(&value, &case);
        assert_eq!(p.f.primary(), merged, "{case}");
        assert_eq!(p.listed_state(), Some(ClosureState::Closed), "{case}");
        assert_eq!(p.link(), None, "{case}");
    }
}

/// A8, the merged-first recovery without the remote branch: the forge
/// squashed the pull request, the primary moved on, and the branch is gone,
/// while the clone kept working on it. Until a poll has recorded the
/// deletion the close is blocked (the history is unproven); once it has,
/// close takes the new branch, whose preview lists the entries the primary
/// does not hold, and nothing on the forge is deleted. The new branch's
/// pull request then ends the ticket closed.
#[test]
fn merged_first_with_the_remote_branch_gone_closes_through_a_new_branch() {
    let (mut p, forge) = owner();
    forge.poll(&p);
    // Work done after the branch was published, which the squash cannot hold.
    let published = p.f.context_oid();
    write(&p.f.context, "notes.txt", "notes\n");
    let old = commit(&p.f.context);
    // The forge squashes what was published, the primary moves on (it
    // changes the ticket again), and the branch is deleted. The clone
    // fetches, and has not polled since: nothing says the branch is gone.
    forge.squash(published);
    let moved = forge.moves_on("primary changed it again");
    forge.delete_branch();
    forge.fetch(&p.f.repo);
    assert_eq!(p.tracking_primary(), moved);
    assert!(p.f.repo.find_reference(&tracking_context_ref()).is_err());
    assert_eq!(p.blockers(), [ClosureBlocker::HistoryUnknown]);
    // A poll records the deletion (and brings the primary in).
    let value = forge.poll(&p);
    let target = value["data"]["poll"]["targets"]
        .as_array()
        .and_then(|targets| targets.iter().find(|t| t["item_id"] == ID))
        .unwrap_or_else(|| panic!("the ticket context is a poll target: {value}"));
    assert_eq!(target["disposition"], "remotely_deleted", "{value}");
    assert_eq!(
        value["data"]["retirements"],
        serde_json::json!([]),
        "{value}"
    );
    p.nothing_retired();
    p.primary_before = p.f.primary();
    assert_eq!(p.primary_before, moved);
    // The new branch is offered, with what the primary does not hold.
    let preview = p.ready();
    assert_eq!(preview.path(), NEW_BRANCH);
    let facts = preview.new_branch.clone().expect("a new branch");
    assert_eq!((facts.old_tip, facts.retired_oid), (Some(old), Some(old)));
    let listed: Vec<_> = facts
        .unheld
        .iter()
        .map(|entry| (String::from_utf8(entry.path.clone()).unwrap(), entry.change))
        .collect();
    assert_eq!(
        listed,
        [
            (ticket(), PathChange::Modified),
            ("notes.txt".to_owned(), PathChange::Added),
        ]
    );
    let req = p.confirming(preview.expectation());
    let mut io = PullRequestIo::default();
    let outcome = PullRequest::branch(p.run(req.clone(), &mut io));
    assert_eq!(outcome.answer(), BranchAnswer::Published);
    assert_eq!(outcome.action(), ClosureAction::NewBranch);
    assert_eq!(outcome.retired_oid(), Some(old));
    let r = p.stored(&req);
    assert_eq!(r.step, ClosureStep::Complete);
    p.assert_new_branch(&r, moved);
    p.assert_retired(&r, old);
    p.assert_unreferenced(old, "A8");
    p.assert_link();
    assert_eq!(p.listed_state(), Some(ClosureState::Closing));
    p.assert_nothing_deleted_remotely(&io);
    p.assert_primary_untouched(&io);
    // The forge holds what it held, and the new branch once it is published.
    let closure = r.closure_oid.unwrap();
    forge.publish(&p);
    assert_eq!(forge.primary(), moved);
    assert_eq!(forge.tip(&context_ref()), Some(closure));
    forge.poll(&p);
    let merged = forge.squash(closure);
    forge.delete_branch();
    let value = forge.poll(&p);
    assert_retired_by(&value, "new branch");
    assert_eq!(p.f.primary(), merged);
    assert_eq!(p.listed_state(), Some(ClosureState::Closed));
    assert_eq!(p.link(), None);
}

/// A second clone of the repository, made from the forge and enabled with
/// its own registry, as another person's checkout would be.
struct Peer {
    root: tempfile::TempDir,
    _data: tempfile::TempDir,
    service: RepositoryService,
    repo: git2::Repository,
}
impl Peer {
    fn of(forge: &Forge) -> Self {
        use crate::repository::EnableRepositoryRequest;
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = tempfile::tempdir().unwrap();
        let repo = git2::Repository::clone(&forge.url(), root.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_bool("core.autocrlf", false).unwrap();
        config.set_str("user.name", "Peer").unwrap();
        config
            .set_str("user.email", "peer@example.invalid")
            .unwrap();
        // The address the configuration's remote is selected by; the fetch
        // is the test's, from the forge.
        repo.remote_set_url("origin", "ssh://fixture@127.0.0.1:1/repository")
            .unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        service
            .enable(EnableRepositoryRequest {
                root: root.path().into(),
                primary_branch: "main".into(),
                identity: None,
                operation_id: OperationId::new(),
            })
            .unwrap();
        Self {
            root,
            _data: data,
            service,
            repo,
        }
    }
    fn poll(&self, forge: &Forge) -> serde_json::Value {
        forge.poll_of(&self.service, self.root.path(), &self.repo)
    }
    fn shown(&self) -> ItemDto {
        let repository = self.service.resolve_repository(self.root.path()).unwrap();
        self.service.show_item(&repository, &id()).unwrap()
    }
    fn worktree(&self) -> std::path::PathBuf {
        self.root.path().join(".manyhands/worktrees").join(ID)
    }
}

/// Another clone that has never seen the ticket's branch reads the ticket
/// `closing` after its poll brings the branch in (the closure is on the
/// branch and not on the primary), reads it `closed` once the forge has
/// merged and deleted the branch, and retires the context its poll made.
#[test]
fn another_clone_reads_closing_after_its_poll() {
    let (p, forge) = owner();
    let (_, closure) = closed_and_published(&p, &forge);
    let peer = Peer::of(&forge);
    assert!(!peer.worktree().exists());
    let value = peer.poll(&forge);
    let target = value["data"]["poll"]["targets"]
        .as_array()
        .and_then(|targets| targets.iter().find(|t| t["item_id"] == ID))
        .unwrap_or_else(|| panic!("the ticket context is a poll target: {value}"));
    assert_ne!(target["disposition"], "closed", "{value}");
    assert_eq!(
        peer.repo.refname_to_id(&context_ref()).ok(),
        Some(closure),
        "{value}"
    );
    let shown = peer.shown();
    let closing = shown.closure.expect("a closure");
    assert_eq!(closing.state, ClosureState::Closing);
    assert_eq!(
        closing.closed_at.as_deref(),
        Some(record::closed_at_text(clock()).unwrap().as_str())
    );
    assert_eq!(
        closing.publication,
        Some(crate::repository::ClosurePublication::Published)
    );
    // This clone gave no link: it did not close.
    assert_eq!(shown.pull_request, None);
    let peer_primary = peer.repo.refname_to_id("refs/heads/main").unwrap();
    assert_eq!(peer_primary, p.primary_before);
    // The forge merges and deletes the branch, and both clones' polls
    // close the ticket and retire its context.
    forge.poll(&p);
    let merged = forge.merge_commit(closure);
    forge.delete_branch();
    let value = peer.poll(&forge);
    assert_retired_by(&value, "another clone");
    assert!(!peer.worktree().exists());
    assert_eq!(peer.repo.refname_to_id("refs/heads/main").unwrap(), merged);
    assert_eq!(peer.shown().closure.unwrap().state, ClosureState::Closed);
    let value = forge.poll(&p);
    assert_retired_by(&value, "the recording clone");
    assert_eq!(p.listed_state(), Some(ClosureState::Closed));
    assert_eq!(p.link(), None);
}
