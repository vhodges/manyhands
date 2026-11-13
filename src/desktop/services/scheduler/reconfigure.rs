//! Which requests reconfigure polling, and which repositories they reach
//! (D6 design R-11).
//!
//! `RemoteAdd`, `RemoteRemove`, `RemoteSelect`, `KeySelect`, `KeyClear`,
//! `KeyRemove`, `KeyDelete`, `HostApprove`, `HostReplace`, `RepoCreate`,
//! `RepoEnable` and `RepoRemove` change what a poll would find: its remote,
//! its key, the host it trusts, or whether the repository is registered.
//! The drain tells the scheduler when one is submitted and when it
//! finishes ([`heard`]); the scheduler then works out [`Scope`]'s
//! repositories.
//!
//! The rule for "every repository using that key, host or remote", the
//! simplest correct one:
//!
//! - a remote or registration change names its repository, and reaches
//!   only that one;
//! - a host approval or replacement reaches the repository it names and
//!   every scheduled repository whose publication remote is reached at that
//!   host (read from the remote's redacted location), those whose host
//!   cannot be told, and those held back for it (`HostUnapproved`);
//! - a key change names no repository: the selected key is the one key
//!   every repository's SSH poll uses, so it reaches every listed one.
//!
//! An intent that only changes the stored policy (Check now, the poll
//! verbs) reaches its repository's policy read and nothing else.

use crate::desktop::{
    model::{
        operations::Operations,
        polling::{Authority, PollAttempt, Root, TargetKind},
    },
    services::{
        inflight::RetirementEnd,
        poll_convert::check_now_attempt,
        requests::{DesktopRequestId, Intent},
        resends::named_request,
        worker::WorkerEvent,
    },
};
use manyhands::{
    canonical::ItemId,
    repository::{AuthoringKind, MutationDataDto, OperationId, RequestId},
    results::{
        CheckpointEffect, Envelope, IntegrationEffect, Outcome, PublicationEffect, ResultCode,
    },
};

/// When the request was heard.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Moment {
    /// Queued for the first time: queued retirements of the repositories
    /// it reaches are dropped (R-11).
    Submitted,
    /// Answered: their polls are cancelled, their attention cleared and
    /// their policies and remotes read again.
    Finished,
    /// Answered, and the change went through: all that `Finished` does,
    /// and the Check now the person was sent to make the change for is sent
    /// (C3).
    Succeeded,
}

/// What a reconfiguring request reaches.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Scope {
    /// One repository's remote or registration. `gone` when a removal
    /// finished: its schedule is dropped.
    Repository { root: Root, gone: bool },
    /// A host answer, for the repository it names and every one held back
    /// for the host.
    Host { root: Root, authority: Authority },
    /// A key change: every listed repository.
    Keys,
}

/// What the drain heard of a retirement request (R-31, E3).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Retiring {
    /// Its first progress event named the operation of its journal row.
    Operation {
        request: RequestId,
        operation: OperationId,
    },
    /// It finished. The request model has sorted its answer. `stopped`: the
    /// person pressed Stop on it, which ends the loop that sent it (RT-9).
    Answered { id: DesktopRequestId, stopped: bool },
    /// It was removed from the queue or never sent: its entry ends.
    Gone {
        id: DesktopRequestId,
        request: RequestId,
        end: RetirementEnd,
    },
}

/// What the drain heard that the scheduler acts on.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Heard {
    /// A retirement request's progress, answer or end.
    Retirement(Retiring),
    Reconfiguration(Moment, Scope),
    /// A request that only moved the stored policy of this repository
    /// finished: the policy is read again, nothing is cancelled.
    PolicyMoved(Root),
    /// A share finished: the drain's own "item shared" (an `ItemSync` or
    /// `ItemRepublish`, a close or an approval), or a `RepoSync`, whose
    /// publication is `Published` or `Current`. The item leaves the unshared
    /// list, and main with it when `main`.
    Shared {
        root: Root,
        item: Option<ItemId>,
        main: bool,
    },
    /// A request that made a checkpoint and did not publish finished, and
    /// its envelope names its item: the item joins the unshared list, as
    /// `Unpublished` when `new`.
    Saved {
        root: Root,
        item: ItemId,
        kind: Option<TargetKind>,
        new: bool,
    },
    /// A close or an approval merged into main and did not publish: main is
    /// ahead of the shared copy.
    MainAhead {
        root: Root,
    },
    /// A Check now finished with success, or with `discovery_pending`: a
    /// completed poll, which clears the attention of its repository however
    /// the cause was fixed (C3), and proves the key it used unlocked. Its
    /// attempt, when it completed and said what it saw, is the repository's
    /// latest and may start or replace a retirement loop (E1).
    CheckedNow(Root, Option<PollAttempt>),
    /// The Resume of a pending retirement's row was answered
    /// `original_request_required`, naming `request` (Fact 26): the
    /// retirement of the item the in-flight record's entry names is sent
    /// again under it, on the user lane (E4, R-31).
    ResendRetirement {
        /// The Resume request that was answered: it has done its work, and
        /// is dismissed once the resend is sent.
        resume: DesktopRequestId,
        request: RequestId,
        root: Root,
        kind: TargetKind,
        item: ItemId,
    },
    /// An Abandon, or a Resume that finished the row itself (the index
    /// hand-off), settled the pending retirement whose journal row is
    /// `operation`: its entry in the in-flight record ends, since no request
    /// of it is left to end it. `finished`: a Resume completed the row (the
    /// attempt's entry says so); otherwise nothing is claimed.
    PendingEnded {
        root: Root,
        operation: OperationId,
        finished: bool,
    },
    /// A Check now finished with a failure: the request's answer, read as a
    /// poll's would be (C3, A4). The request names the keys its attempt
    /// asked for.
    CheckFailed {
        root: Root,
        request: DesktopRequestId,
        code: ResultCode,
        authority: Option<Authority>,
    },
}

fn root_of(intent: &Intent) -> Option<Root> {
    intent.root()?.to_str().map(str::to_owned)
}

/// The scope of a reconfiguring intent, if it is one.
pub(in crate::desktop::services) fn scope_of(intent: &Intent, gone: bool) -> Option<Scope> {
    Some(match intent {
        Intent::RepoCreate { .. }
        | Intent::RepoEnable { .. }
        | Intent::RepoRemove { .. }
        | Intent::RemoteAdd { .. }
        | Intent::RemoteRemove { .. }
        | Intent::RemoteSelect { .. } => Scope::Repository {
            root: root_of(intent)?,
            gone: gone && matches!(intent, Intent::RepoRemove { .. }),
        },
        Intent::HostApprove {
            root, authority, ..
        }
        | Intent::HostReplace {
            root, authority, ..
        } => Scope::Host {
            root: root.to_str()?.to_owned(),
            authority: Authority {
                host: authority.host.clone(),
                port: authority.port,
            },
        },
        Intent::KeySelect { .. }
        | Intent::KeyClear
        | Intent::KeyRemove { .. }
        | Intent::KeyDelete { .. } => Scope::Keys,
        _ => return None,
    })
}

/// A request that moves a repository's stored polling state without
/// reconfiguring it: Check now (a deadline and a row) and the three policy
/// verbs, whose answer is the policy itself (D6 C1).
fn policy_of(intent: &Intent) -> Option<Root> {
    match intent {
        Intent::PollOnce { .. }
        | Intent::PollConfigure { .. }
        | Intent::PollPause { .. }
        | Intent::PollResume { .. } => root_of(intent),
        _ => None,
    }
}

/// The host a host answer names, from the data it carries.
fn host_of(envelope: &Envelope<manyhands::repository::MutationDataDto>) -> Option<Authority> {
    match &envelope.data {
        Some(MutationDataDto::Synchronization(data)) => Some(Authority {
            host: data.host.clone(),
            port: data.port,
        }),
        _ => None,
    }
}

/// What `event` means to the scheduler as a retirement request's, if it is
/// one's.
fn retiring(event: &WorkerEvent, operations: &Operations) -> Option<Retiring> {
    let id = match event {
        WorkerEvent::Progress(id, _)
        | WorkerEvent::Finished(id, _)
        | WorkerEvent::Removed(id)
        | WorkerEvent::NotSent(id, _) => *id,
        _ => return None,
    };
    let request = operations.request(id)?;
    if !matches!(request.intent, Intent::ContextRetire { .. }) {
        return None;
    }
    match event {
        WorkerEvent::Progress(_, progress) => Some(Retiring::Operation {
            request: request.request_id,
            operation: OperationId::parse(progress.operation_id.as_deref()?).ok()?,
        }),
        WorkerEvent::Finished(..) => Some(Retiring::Answered {
            id,
            stopped: operations.is_stopping(id),
        }),
        WorkerEvent::Removed(_) => Some(Retiring::Gone {
            id,
            request: request.request_id,
            end: RetirementEnd::Removed,
        }),
        WorkerEvent::NotSent(..) => Some(Retiring::Gone {
            id,
            request: request.request_id,
            end: RetirementEnd::NotSent,
        }),
        _ => None,
    }
}

/// A resume or abandon of the operation `operation_id` of `root`. A resume
/// answered `original_request_required`: when the in-flight record's entry names the
/// operation, the retirement is to be sent again under the request the
/// answer names. Any other resume is not the scheduler's.
///
/// A resume or an abandon that settled the row itself ends the entry: the
/// retirement is over, and no request is left to end it.
fn settled_retirement(
    id: DesktopRequestId,
    root: &std::path::Path,
    operation_id: OperationId,
    resume: bool,
    envelope: &Envelope<MutationDataDto>,
    operations: &Operations,
) -> Option<Heard> {
    let root = root.to_str()?;
    let entry = operations.retirement_of(root, operation_id)?;
    if envelope.code == ResultCode::OriginalRequestRequired {
        if !resume {
            return None;
        }
        return Some(Heard::ResendRetirement {
            resume: id,
            request: named_request(envelope)?,
            root: entry.root.clone(),
            kind: entry.kind,
            item: entry.item.clone(),
        });
    }
    let settled = matches!(envelope.outcome, Outcome::Success | Outcome::Noop);
    settled.then(|| Heard::PendingEnded {
        root: entry.root.clone(),
        operation: operation_id,
        // A row settled elsewhere (a no-op) is not this Resume's doing.
        finished: resume && envelope.outcome == Outcome::Success,
    })
}

/// What a finished request's effects say of sharing: a share, a save or a
/// merge that did not publish (Part A). Before `scope_of`, since none of
/// these reconfigures polling.
fn sharing(intent: &Intent, envelope: &Envelope<MutationDataDto>) -> Option<Heard> {
    let root = root_of(intent)?;
    let effects = &envelope.effects;
    let published = matches!(
        effects.publication,
        PublicationEffect::Published | PublicationEffect::Current
    );
    let shared = |item: Option<&ItemId>, main| Heard::Shared {
        root: root.clone(),
        item: item.cloned(),
        main,
    };
    match intent {
        Intent::ItemSync { id, .. } | Intent::ItemRepublish { id, .. } if published => {
            return Some(shared(Some(id), false));
        }
        Intent::TicketClose { id, .. } | Intent::DocumentPromote { id, .. } if published => {
            return Some(shared(Some(id), true));
        }
        Intent::RepoSync { .. } if published => return Some(shared(None, true)),
        Intent::TicketClose { .. } | Intent::DocumentPromote { .. } => {
            let merged = effects.integration == IntegrationEffect::Complete
                && effects.publication == PublicationEffect::Pending;
            return merged.then_some(Heard::MainAhead { root });
        }
        _ => {}
    }
    if published || effects.checkpoint != CheckpointEffect::Committed {
        return None;
    }
    let (kind, new) = match intent {
        Intent::DocumentSave { .. } | Intent::DocumentMove { .. } => {
            (Some(TargetKind::Document), false)
        }
        Intent::TicketSave { .. } => (Some(TargetKind::Ticket), false),
        Intent::DocumentCreate { .. } => (Some(TargetKind::Document), true),
        Intent::TicketCreate { .. } => (Some(TargetKind::Ticket), true),
        Intent::CommentAdd { kind, .. } | Intent::LocalContextSync { kind, .. } => {
            (Some(kind_of(*kind)), false)
        }
        // Its intent names no item and no kind (U-7).
        Intent::LocalContextResolve { .. } => (None, false),
        _ => return None,
    };
    let item = envelope.scope.item_id.as_deref()?.parse().ok()?;
    Some(Heard::Saved {
        root,
        item,
        kind,
        new,
    })
}

fn kind_of(kind: AuthoringKind) -> TargetKind {
    match kind {
        AuthoringKind::Ticket => TargetKind::Ticket,
        AuthoringKind::Document => TargetKind::Document,
    }
}

/// What `event` means to the scheduler, if anything. Called by the drain
/// for each event before the request model applies it, so a finished
/// request is still there to read.
pub(crate) fn heard(event: &WorkerEvent, operations: &Operations) -> Option<Heard> {
    if let Some(retiring) = retiring(event, operations) {
        return Some(Heard::Retirement(retiring));
    }
    match event {
        // A retry or a Prepare again is not a new submission.
        WorkerEvent::Queued(request) if request.attempt <= 1 => scope_of(&request.intent, false)
            .map(|scope| Heard::Reconfiguration(Moment::Submitted, scope)),
        WorkerEvent::Finished(id, envelope) => {
            let intent = &operations.request(*id)?.intent;
            if let Some(sharing) = sharing(intent, envelope) {
                return Some(sharing);
            }
            if let Intent::OperationResume { root, operation_id }
            | Intent::OperationAbandon { root, operation_id } = intent
            {
                let resume = matches!(intent, Intent::OperationResume { .. });
                return settled_retirement(*id, root, *operation_id, resume, envelope, operations);
            }
            let gone =
                envelope.outcome == Outcome::Success || envelope.code == ResultCode::AlreadyApplied;
            match scope_of(intent, gone) {
                Some(scope) => {
                    let moment = if gone {
                        Moment::Succeeded
                    } else {
                        Moment::Finished
                    };
                    Some(Heard::Reconfiguration(moment, scope))
                }
                None => {
                    let root = policy_of(intent)?;
                    Some(match intent {
                        Intent::PollOnce { .. }
                            if envelope.outcome == Outcome::Success
                                || envelope.code == ResultCode::DiscoveryPending =>
                        {
                            // The poll completed; with `discovery_pending`
                            // its index refresh did not (R-10).
                            let pending = envelope.code == ResultCode::DiscoveryPending;
                            let attempt = match &envelope.data {
                                Some(MutationDataDto::Poll(data)) => {
                                    check_now_attempt(&root, data, pending)
                                }
                                _ => None,
                            };
                            Heard::CheckedNow(root, attempt)
                        }
                        Intent::PollOnce { .. } => Heard::CheckFailed {
                            root,
                            request: *id,
                            code: envelope.code,
                            authority: host_of(envelope),
                        },
                        _ => Heard::PolicyMoved(root),
                    })
                }
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::services::requests::DesktopRequestId;
    use manyhands::repository::{SharedKeyId, transport::SshAuthority};
    use std::path::PathBuf;

    /// Fact 26, E4, R-31: a resume of a pending retirement's row, answered
    /// `original_request_required`, resends the retirement of the item the
    /// in-flight record's entry names, under the request ID the answer
    /// names. A resume or an abandon that settled the row ends the entry. A
    /// row no entry names, another code and a failed abandon are not the
    /// scheduler's.
    #[test]
    fn resumed_and_abandoned_retirements_are_heard() {
        use crate::desktop::{
            model::operations::Entered,
            services::requests::{DesktopRequest, Lane, RequestState},
            test_support::retire::original_request,
        };
        use manyhands::results::Scope;

        let item: ItemId = "01ARZ3NDEKTSV4RRFFQ69G5FG0".parse().unwrap();
        let (operation, other) = (OperationId::new(), OperationId::new());
        let named = RequestId::new();
        let mut operations = Operations::default();
        operations.set_retirement_entries(vec![Entered {
            root: "/r".to_owned(),
            kind: TargetKind::Ticket,
            item: item.clone(),
            request: Some(named),
            operation: Some(operation),
        }]);
        let mut n = 0;
        let mut finished = |intent: Intent, answer: Envelope<MutationDataDto>| {
            n += 1;
            let id = DesktopRequestId(n);
            operations.apply(WorkerEvent::Queued(DesktopRequest {
                id,
                request_id: RequestId::new(),
                intent,
                confirmation: None,
                lane: Lane::User,
                attempt: 1,
                state: RequestState::Queued,
            }));
            heard(&WorkerEvent::Finished(id, Box::new(answer)), &operations)
        };
        let resume = |operation_id| Intent::OperationResume {
            root: "/r".into(),
            operation_id,
        };
        let abandon = |operation_id| Intent::OperationAbandon {
            root: "/r".into(),
            operation_id,
        };
        let done = || {
            Envelope::mutation(
                "operation",
                Scope::default(),
                Outcome::Success,
                ResultCode::Ok,
            )
        };
        let refused = || {
            Envelope::<MutationDataDto>::failure(
                "operation",
                Scope::default(),
                ResultCode::RecoveryRequired,
                vec![],
            )
        };
        let ended = |finished| {
            Some(Heard::PendingEnded {
                root: "/r".to_owned(),
                operation,
                finished,
            })
        };

        assert_eq!(
            finished(resume(operation), original_request(named)),
            Some(Heard::ResendRetirement {
                resume: DesktopRequestId(1),
                request: named,
                root: "/r".to_owned(),
                kind: TargetKind::Ticket,
                item,
            })
        );
        assert_eq!(finished(resume(operation), done()), ended(true));
        // Settled elsewhere (a no-op): the entry ends, and nothing is claimed.
        let noop = Envelope::mutation(
            "operation",
            Scope::default(),
            Outcome::Noop,
            ResultCode::AlreadyApplied,
        );
        assert_eq!(finished(resume(operation), noop), ended(false));
        assert_eq!(finished(abandon(operation), done()), ended(false));
        // Not a retirement's row, another code, a failed abandon, and an
        // abandon answered with a request to send again.
        assert_eq!(finished(resume(other), original_request(named)), None);
        assert_eq!(finished(resume(other), done()), None);
        assert_eq!(finished(resume(operation), refused()), None);
        assert_eq!(finished(abandon(operation), refused()), None);
        assert_eq!(finished(abandon(operation), original_request(named)), None);
    }

    /// Part A: a finished request that shared, saved or merged is heard, in
    /// the order the drain hands the events over, with what the scheduler
    /// needs to change its unshared list: a share of an item, an item and
    /// main, or main; a save, a create or a repair that made a checkpoint
    /// and did not publish; a close or an approval that merged and did not
    /// publish. By the drain's own predicate (decision 24; R-3, R-30, R-45).
    #[test]
    fn finished_shares_and_saves_are_heard() {
        use crate::desktop::services::requests::{CommentBody, DesktopRequest, Lane, RequestState};
        use manyhands::{
            repository::{AuthoringKind, RelationshipWrite},
            results::{CheckpointEffect, Effects, IntegrationEffect, PublicationEffect},
        };

        const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG0";
        const OTHER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG1";
        let item: ItemId = ITEM.parse().unwrap();
        let root = PathBuf::from("/r");
        let id = || -> ItemId { ITEM.parse().unwrap() };
        let effects = |checkpoint, publication, integration| Effects {
            checkpoint,
            publication,
            integration,
            ..Effects::not_requested()
        };
        let share = |item: Option<ItemId>, main| {
            Some(Heard::Shared {
                root: "/r".to_owned(),
                item,
                main,
            })
        };
        let saved = |kind, new| {
            Some(Heard::Saved {
                root: "/r".to_owned(),
                item: item.clone(),
                kind,
                new,
            })
        };

        let mut operations = Operations::default();
        let mut n = 0;
        // `named`: the envelope's scope names the item.
        let mut finished = |intent: Intent,
                            (outcome, code): (Outcome, ResultCode),
                            effects: Effects,
                            named: bool| {
            n += 1;
            let request = DesktopRequestId(n);
            operations.apply(WorkerEvent::Queued(DesktopRequest {
                id: request,
                request_id: RequestId::new(),
                intent,
                confirmation: None,
                lane: Lane::User,
                attempt: 1,
                state: RequestState::Queued,
            }));
            let scope = manyhands::results::Scope {
                item_id: named.then(|| ITEM.to_owned()),
                ..Default::default()
            };
            let mut answer = Envelope::mutation("command", scope, outcome, code);
            answer.effects = effects;
            heard(
                &WorkerEvent::Finished(request, Box::new(answer)),
                &operations,
            )
        };
        let done = (Outcome::Success, ResultCode::Ok);
        let partial = (Outcome::Partial, ResultCode::DiscoveryPending);
        let refused = (Outcome::Error, ResultCode::RecoveryRequired);
        use CheckpointEffect::{Committed, Unchanged};
        use PublicationEffect::{Current, NotRequested, Pending, Published};
        let none = IntegrationEffect::NotRequested;
        let sync = |kind| Intent::ItemSync {
            root: root.clone(),
            kind,
            id: id(),
        };

        // Shares: by the drain's predicate, `Published` or `Current`.
        let shares = [
            (
                "ItemSync, success",
                sync(AuthoringKind::Ticket),
                done,
                Published,
                share(Some(id()), false),
            ),
            (
                "ItemSync, discovery pending",
                sync(AuthoringKind::Document),
                partial,
                Published,
                share(Some(id()), false),
            ),
            (
                "ItemSync, an error that did not publish",
                sync(AuthoringKind::Ticket),
                refused,
                NotRequested,
                None,
            ),
            (
                "ItemSync, nothing to publish",
                sync(AuthoringKind::Ticket),
                done,
                NotRequested,
                None,
            ),
            (
                "ItemRepublish, current",
                Intent::ItemRepublish {
                    root: root.clone(),
                    kind: AuthoringKind::Ticket,
                    id: id(),
                },
                done,
                Current,
                share(Some(id()), false),
            ),
            (
                "TicketClose, published",
                Intent::TicketClose {
                    root: root.clone(),
                    id: id(),
                },
                done,
                Published,
                share(Some(id()), true),
            ),
            (
                "DocumentPromote, published",
                Intent::DocumentPromote {
                    root: root.clone(),
                    id: id(),
                },
                done,
                Published,
                share(Some(id()), true),
            ),
            (
                "RepoSync, published",
                Intent::RepoSync { root: root.clone() },
                done,
                Published,
                share(None, true),
            ),
            (
                "RepoSync, current",
                Intent::RepoSync { root: root.clone() },
                done,
                Current,
                share(None, true),
            ),
            (
                "RepoSync, not requested",
                Intent::RepoSync { root: root.clone() },
                done,
                NotRequested,
                None,
            ),
        ];
        for (name, intent, answer, publication, expected) in shares {
            let effects = effects(Committed, publication, none);
            assert_eq!(finished(intent, answer, effects, true), expected, "{name}");
        }

        // Saves: a checkpoint that did not publish, an item the envelope names.
        let kinds = |kind| Some(kind);
        let authoring = vec![
            (
                "DocumentSave",
                Intent::DocumentSave {
                    root: root.clone(),
                    id: id(),
                    title: "T".to_owned(),
                    body: "B".to_owned(),
                    observation: None,
                },
                kinds(TargetKind::Document),
                false,
            ),
            (
                "TicketSave",
                Intent::TicketSave {
                    root: root.clone(),
                    id: id(),
                    title: "T".to_owned(),
                    ticket_type: "task".to_owned(),
                    status: "open".to_owned(),
                    project: None,
                    team: None,
                    body: "B".to_owned(),
                    deps: RelationshipWrite::Unchanged,
                    parent: RelationshipWrite::Unchanged,
                    observation: "o".to_owned(),
                },
                kinds(TargetKind::Ticket),
                false,
            ),
            (
                "DocumentCreate",
                Intent::DocumentCreate {
                    root: root.clone(),
                    id: id(),
                    path: PathBuf::from("docs/a.md"),
                    title: "T".to_owned(),
                    body: "B".to_owned(),
                },
                kinds(TargetKind::Document),
                true,
            ),
            (
                "TicketCreate",
                Intent::TicketCreate {
                    root: root.clone(),
                    id: id(),
                    title: "T".to_owned(),
                    ticket_type: "task".to_owned(),
                    status: "open".to_owned(),
                    project: None,
                    team: None,
                    body: "B".to_owned(),
                    deps: Vec::new(),
                    parent: None,
                },
                kinds(TargetKind::Ticket),
                true,
            ),
            (
                "DocumentMove",
                Intent::DocumentMove {
                    root: root.clone(),
                    id: id(),
                    observation: None,
                    destination_path: PathBuf::from("docs/b.md"),
                    destination_observation: "absent".to_owned(),
                },
                kinds(TargetKind::Document),
                false,
            ),
            (
                "CommentAdd",
                Intent::CommentAdd {
                    root: root.clone(),
                    kind: AuthoringKind::Document,
                    id: id(),
                    comment_id: OTHER.parse().unwrap(),
                    parent_id: None,
                    body: CommentBody::new("C".to_owned()),
                },
                kinds(TargetKind::Document),
                false,
            ),
            (
                "LocalContextSync",
                Intent::LocalContextSync {
                    root: root.clone(),
                    kind: AuthoringKind::Ticket,
                    id: id(),
                    observation: "o".to_owned(),
                },
                kinds(TargetKind::Ticket),
                false,
            ),
        ];
        for (name, intent, kind, new) in &authoring {
            for publication in [NotRequested, Pending] {
                let effects = effects(Committed, publication, none);
                assert_eq!(
                    finished(intent.clone(), done, effects.clone(), true),
                    saved(*kind, *new),
                    "{name}, {publication:?}"
                );
            }
            // One that published is shared already; one that changed nothing,
            // or whose envelope names no item, saved nothing to count.
            for (why, effects, named) in [
                ("published", effects(Committed, Published, none), true),
                ("current", effects(Committed, Current, none), true),
                ("unchanged", effects(Unchanged, NotRequested, none), true),
                (
                    "no item named",
                    effects(Committed, NotRequested, none),
                    false,
                ),
            ] {
                assert_eq!(
                    finished(intent.clone(), done, effects, named),
                    None,
                    "{name}, {why}"
                );
            }
        }

        // A local conflict's resolution names no item in the library's
        // envelope (U-7, K-1), so it counts nothing; one that named its item
        // would join with no kind.
        let resolve = Intent::LocalContextResolve {
            root: root.clone(),
            operation_id: OperationId::new(),
            observation: "o".to_owned(),
            resolutions: Vec::new(),
        };
        let committed = effects(Committed, NotRequested, none);
        assert_eq!(
            finished(resolve.clone(), done, committed.clone(), false),
            None
        );
        assert_eq!(finished(resolve, done, committed, true), saved(None, false));

        // A close or an approval that merged and did not publish leaves main
        // ahead of the shared copy.
        let merged = |publication| effects(Committed, publication, IntegrationEffect::Complete);
        for intent in [
            Intent::TicketClose {
                root: root.clone(),
                id: id(),
            },
            Intent::DocumentPromote {
                root: root.clone(),
                id: id(),
            },
        ] {
            let ahead = Some(Heard::MainAhead {
                root: "/r".to_owned(),
            });
            assert_eq!(finished(intent.clone(), done, merged(Pending), true), ahead);
            assert_eq!(
                finished(intent.clone(), done, merged(NotRequested), true),
                None
            );
            let incomplete = effects(Committed, Pending, IntegrationEffect::Pending);
            assert_eq!(finished(intent, done, incomplete, true), None);
        }

        // A Check now is unchanged: a completed poll, with no data here.
        let poll = Intent::PollOnce { root: root.clone() };
        assert_eq!(
            finished(
                poll,
                done,
                effects(CheckpointEffect::NotRequested, NotRequested, none),
                false
            ),
            Some(Heard::CheckedNow("/r".to_owned(), None))
        );
    }

    /// R-11's list, and what each reaches.
    #[test]
    fn scopes_of_intents() {
        let root = PathBuf::from("/r");
        let repo = |gone| Scope::Repository {
            root: "/r".to_owned(),
            gone,
        };
        let id = SharedKeyId::new();
        let cases: Vec<(Intent, Option<Scope>)> = vec![
            (
                Intent::RemoteAdd {
                    root: root.clone(),
                    name: "o".to_owned(),
                    url: "u".to_owned(),
                },
                Some(repo(false)),
            ),
            (
                Intent::RemoteRemove {
                    root: root.clone(),
                    name: "o".to_owned(),
                },
                Some(repo(false)),
            ),
            (
                Intent::RemoteSelect {
                    root: root.clone(),
                    name: None,
                },
                Some(repo(false)),
            ),
            (Intent::RepoRemove { root: root.clone() }, Some(repo(false))),
            (Intent::KeyClear, Some(Scope::Keys)),
            (Intent::KeySelect { id }, Some(Scope::Keys)),
            (Intent::KeyRemove { id }, Some(Scope::Keys)),
            (Intent::KeyDelete { id }, Some(Scope::Keys)),
            (
                Intent::HostApprove {
                    root: root.clone(),
                    authority: SshAuthority {
                        host: "h".to_owned(),
                        port: 22,
                    },
                    algorithm: String::new(),
                    fingerprint: String::new(),
                },
                Some(Scope::Host {
                    root: "/r".to_owned(),
                    authority: Authority {
                        host: "h".to_owned(),
                        port: 22,
                    },
                }),
            ),
            (Intent::IndexRefresh { root: root.clone() }, None),
            (Intent::PollOnce { root }, None),
        ];
        for (intent, expected) in cases {
            assert_eq!(scope_of(&intent, false), expected);
        }
        // Only a removal that went through drops the schedule.
        assert_eq!(
            scope_of(&Intent::RepoRemove { root: "/r".into() }, true),
            Some(repo(true))
        );
        assert_eq!(
            scope_of(
                &Intent::RemoteAdd {
                    root: "/r".into(),
                    name: "o".into(),
                    url: "u".into()
                },
                true
            ),
            Some(repo(false))
        );
    }
}
