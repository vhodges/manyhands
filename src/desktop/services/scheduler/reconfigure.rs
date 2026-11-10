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
        polling::{Authority, Root},
    },
    services::{
        requests::{DesktopRequestId, Intent},
        worker::WorkerEvent,
    },
};
use manyhands::{
    repository::MutationDataDto,
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

/// What the drain heard that the scheduler acts on.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Heard {
    Reconfiguration(Moment, Scope),
    /// A request that only moved the stored policy of this repository
    /// finished: the policy is read again, nothing is cancelled.
    PolicyMoved(Root),
    /// A Check now finished with success: a completed poll, which clears
    /// the attention of its repository however the cause was fixed (C3),
    /// and proves the key it used unlocked.
    CheckedNow(Root),
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

/// What `event` means to the scheduler, if anything. Called by the drain
/// for each event before the request model applies it, so a finished
/// request is still there to read.
pub(crate) fn heard(event: &WorkerEvent, operations: &Operations) -> Option<Heard> {
    match event {
        // A retry or a Prepare again is not a new submission.
        WorkerEvent::Queued(request) if request.attempt <= 1 => scope_of(&request.intent, false)
            .map(|scope| Heard::Reconfiguration(Moment::Submitted, scope)),
        WorkerEvent::Finished(id, envelope) => {
            let intent = &operations.request(*id)?.intent;
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
                        Intent::PollOnce { .. } if envelope.outcome == Outcome::Success => {
                            Heard::CheckedNow(root)
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
    use manyhands::repository::{SharedKeyId, transport::SshAuthority};
    use std::path::PathBuf;

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
