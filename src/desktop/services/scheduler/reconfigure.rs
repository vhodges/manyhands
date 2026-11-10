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
    repository::{MutationDataDto, OperationId, RequestId},
    results::{Envelope, Outcome, ResultCode},
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
        request: RequestId,
        root: Root,
        kind: TargetKind,
        item: ItemId,
    },
    /// An Abandon, or a Resume that finished the row itself (the index
    /// hand-off), settled the pending retirement whose journal row is
    /// `operation`: its entry in the in-flight record ends, since no request
    /// of it is left to end it. `retired`: the copy is gone (a Resume), not
    /// abandoned.
    PendingEnded {
        root: Root,
        operation: OperationId,
        retired: bool,
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
        retired: resume,
    })
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
            if let Intent::OperationResume { root, operation_id }
            | Intent::OperationAbandon { root, operation_id } = intent
            {
                let resume = matches!(intent, Intent::OperationResume { .. });
                return settled_retirement(root, *operation_id, resume, envelope, operations);
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
        let ended = |retired| {
            Some(Heard::PendingEnded {
                root: "/r".to_owned(),
                operation,
                retired,
            })
        };

        assert_eq!(
            finished(resume(operation), original_request(named)),
            Some(Heard::ResendRetirement {
                request: named,
                root: "/r".to_owned(),
                kind: TargetKind::Ticket,
                item,
            })
        );
        assert_eq!(finished(resume(operation), done()), ended(true));
        assert_eq!(finished(abandon(operation), done()), ended(false));
        // Not a retirement's row, another code, a failed abandon, and an
        // abandon answered with a request to send again.
        assert_eq!(finished(resume(other), original_request(named)), None);
        assert_eq!(finished(resume(other), done()), None);
        assert_eq!(finished(resume(operation), refused()), None);
        assert_eq!(finished(abandon(operation), refused()), None);
        assert_eq!(finished(abandon(operation), original_request(named)), None);
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
