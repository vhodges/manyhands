//! S1: the rows of this session's requests (design A2, B1-B3, B5), one for
//! each request that is not dismissed. A retirement is a request like any
//! other (R-51): nothing hides it. A source leaves `attention` to the log.

mod offers;

use std::path::Path;

use manyhands::{
    canonical::ItemId,
    repository::{AuthoringKind, ProgressStage},
    results::Outcome,
};

use super::{
    copy::{
        DISMISS, NOTHING_TO_CHANGE, failure_description, item_name, lines_description, lower_first,
        scope_name, share_words,
    },
    intent_words::{Names, Words, intent_words, item_of},
    lifecycle_lines::{Reach, outcome_lines},
    rows::{ActivityAction, Derived, Family, Group, ItemRef, Kind, Offer, RowKey, Subject},
};
use crate::desktop::{
    model::{
        operations::{Operations, Progressing, StopOffer, stage_text, stage_words},
        polling::retire::Verdict,
        publication::{ShareTarget, SyncOffer, SyncOutcome},
        stamp::RepoKey,
    },
    services::requests::{DesktopRequest, DesktopRequestId, Intent, NotSent, RequestState},
};

/// Precomputed by the Workspace from Services and the worker (A2).
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct RequestFacts {
    /// `stop_offer(id, remote.is_some())`.
    pub stop_offer: StopOffer,
    /// Today's shown share offers.
    pub shown_offers: Vec<SyncOffer>,
    /// What the Workspace worked out at the refresh and the model cannot
    /// (R-9): the Busy holder's words ("May be waiting for {label}"), or, for
    /// a deleted branch, that its republish is queued or running.
    pub share_words: Option<String>,
    pub already_finished: bool,
    /// The answer stopped on a conflict whose sheet Review changes opens.
    pub review: bool,
    /// Today's shown recovery actions: their place among the envelope's
    /// actions, their label and whether they can run.
    pub recoveries: Vec<(usize, String, bool)>,
    /// A resend waits on it (R-47).
    pub resend_waits: bool,
    /// A resend follows from it (N23).
    pub followed: bool,
    /// Its follow-up ended Removed or NotSent (N23 undone).
    pub follow_void: bool,
    /// A failed PollOnce whose answer `check_failed` applied (N4, R-58).
    pub check_applied: bool,
    /// A failed PollOnce whose repository applied a completed attempt since
    /// (N22 clause 4).
    pub superseded: bool,
}

/// The request half of `Inputs`, which `request_rows` reads: the model, the
/// facts of a request, and the lookups the Workspace supplies.
#[allow(dead_code)] // Used from Task 5
pub(crate) struct RequestInputs<'a> {
    pub operations: &'a Operations,
    pub requests: &'a dyn Fn(DesktopRequestId) -> RequestFacts,
    /// An item's title as read, never a draft's own (B1).
    pub title: &'a dyn Fn(&RepoKey, AuthoringKind, &ItemId) -> Option<String>,
    /// A repository's main branch, when known.
    pub primary: &'a dyn Fn(&RepoKey) -> Option<String>,
}

/// One row for each request this session holds that is not dismissed,
/// newest first, as the model lists them.
#[allow(dead_code)] // Used from Task 5
pub(crate) fn request_rows(inputs: &RequestInputs<'_>) -> Vec<Derived> {
    inputs
        .operations
        .requests()
        .iter()
        .filter(|request| !matches!(request.state, RequestState::Dismissed))
        .map(|request| Row::of(inputs, request).derive())
        .collect()
}

/// What a row's words say, before its offers.
struct Said {
    kind: Kind,
    title: String,
    description: Option<String>,
}

/// A request and everything its row reads of it.
struct Row<'a> {
    request: &'a DesktopRequest,
    operations: &'a Operations,
    facts: RequestFacts,
    root: Option<RepoKey>,
    primary: Option<String>,
    item: Option<ItemRef>,
    names: Names,
}

impl<'a> Row<'a> {
    fn of(inputs: &'a RequestInputs<'a>, request: &'a DesktopRequest) -> Self {
        let root = request
            .intent
            .root()
            .map(|root| root.to_string_lossy().into_owned());
        let primary = root.as_ref().and_then(|root| (inputs.primary)(root));
        let item = item_of(&request.intent).and_then(|(kind, id)| {
            Some(ItemRef {
                root: root.clone()?,
                kind,
                id: id.clone(),
            })
        });
        let name = item.as_ref().map(|item| {
            let title = (inputs.title)(&item.root, item.kind, &item.id);
            item_name(title.as_deref(), item.kind, &item.id)
        });
        Self {
            request,
            operations: inputs.operations,
            facts: (inputs.requests)(request.id),
            names: Names {
                repo: scope_name(request.intent.root()),
                primary: primary.clone().unwrap_or_else(|| "main".to_owned()),
                item: name,
            },
            root,
            primary,
            item,
        }
    }

    fn id(&self) -> DesktopRequestId {
        self.request.id
    }

    fn derive(self) -> Derived {
        let words = intent_words(&self.request.intent, &self.names);
        let owed = self.owed();
        let said = self.said(&words, owed);
        let (button, mut menu) = self.offers(owed.is_some());
        let ended = !matches!(
            self.request.state,
            RequestState::Queued | RequestState::Running { .. }
        );
        if ended {
            menu.push(offer(
                ActivityAction::Dismiss(RowKey::Request(self.id())),
                DISMISS,
                true,
            ));
        }
        let quiet = match &self.request.state {
            RequestState::Finished(Outcome::Success | Outcome::Noop | Outcome::Cancelled, ..) => {
                owed.is_none()
            }
            RequestState::Removed => true,
            _ => false,
        };
        let clearable = quiet
            && !self.facts.resend_waits
            && menu.iter().all(|offer| {
                !offer.enabled
                    || matches!(
                        offer.action,
                        ActivityAction::Dismiss(_) | ActivityAction::Open(_)
                    )
            });
        Derived {
            key: RowKey::Request(self.id()),
            kind: said.kind,
            title: said.title,
            description: said.description,
            repo: self
                .request
                .intent
                .root()
                .map(|root| scope_name(Some(root))),
            button,
            menu,
            detail: self.detail(owed),
            attention: false,
            item: self.item.clone(),
            clearable,
            at: None,
            group: self.group(),
        }
    }

    /// An Owed retirement's verdict: its words and offers differ, whatever
    /// its envelope says (A2).
    fn owed(&self) -> Option<bool> {
        match self
            .operations
            .retirement(self.id())
            .map(|answer| &answer.verdict)
        {
            Some(Verdict::Owed { catching_up, .. })
                if matches!(self.request.state, RequestState::Finished(..)) =>
            {
                Some(*catching_up)
            }
            _ => None,
        }
    }

    /// The stage words of a running request, by its intent's kind (F3).
    fn stage_words(&self, stage: ProgressStage) -> Option<String> {
        let primary = Some(self.names.primary.as_str());
        match Progressing::of(&self.request.intent) {
            Some(kind) => stage_text(kind, stage, None, primary),
            None => Some(stage_words(stage, primary)),
        }
    }

    fn said(&self, words: &Words, owed: Option<bool>) -> Said {
        let said = |kind, title: String, description| Said {
            kind,
            title,
            description,
        };
        let (doing, verb) = (&words.doing, &words.verb);
        match &self.request.state {
            RequestState::Queued => said(Kind::Working, format!("Waiting to {verb}"), None),
            RequestState::Running { stage, .. } => {
                let staged = stage.and_then(|stage| self.stage_words(stage));
                let title = match staged {
                    Some(staged) => format!("{doing}… · {staged}"),
                    None => format!("{doing}…"),
                };
                said(Kind::Working, title, None)
            }
            RequestState::Removed => said(Kind::Info, format!("Cancelled: {verb}"), None),
            RequestState::NotSent(why) => {
                let reason = match why {
                    NotSent::SecretRequired => "your key's passphrase is needed again",
                    NotSent::KeyStoreUnavailable => "Manyhands could not find your SSH key store",
                };
                said(Kind::Failure, format!("Didn't {verb}: {reason}"), None)
            }
            RequestState::Dismissed => said(Kind::Info, String::new(), None),
            RequestState::Finished(outcome, _, _, _, message) => {
                let failed = matches!(
                    outcome,
                    Outcome::Partial | Outcome::Blocked | Outcome::Error
                );
                let failure = failure_description(message);
                if owed.is_some() {
                    let subject = self.names.item.as_deref().unwrap_or("it");
                    return said(
                        Kind::Failure,
                        format!("Retiring {subject} didn't finish"),
                        failed.then_some(failure),
                    );
                }
                if let Some(result) = self.operations.share_result(self.id()) {
                    return self.share_said(&result.outcome, result.attempt.target, &failure);
                }
                match outcome {
                    Outcome::Success | Outcome::Noop => {
                        let lines = self.lifecycle_lines();
                        let description = lines.or(words.note.map(str::to_owned)).or_else(|| {
                            (*outcome == Outcome::Noop).then(|| NOTHING_TO_CHANGE.to_owned())
                        });
                        let title = words.done.clone().unwrap_or_else(|| words.doing.clone());
                        said(Kind::Success, title, description)
                    }
                    Outcome::Cancelled => {
                        said(Kind::Info, format!("Stopped {}", lower_first(doing)), None)
                    }
                    Outcome::Partial | Outcome::Blocked | Outcome::Error => {
                        said(Kind::Failure, format!("Couldn't {verb}"), Some(failure))
                    }
                }
            }
        }
    }

    /// B3: a finished share, by its classified outcome.
    fn share_said(&self, outcome: &SyncOutcome, target: ShareTarget, failure: &str) -> Said {
        let subject = match target {
            ShareTarget::Item => self.names.item.clone().unwrap_or_default(),
            ShareTarget::Main => self.names.primary.clone(),
        };
        let holder = match outcome {
            SyncOutcome::Busy => self.facts.share_words.as_deref(),
            _ => None,
        };
        let said = share_words(outcome, &subject, &self.names.repo, holder, failure);
        Said {
            kind: said.kind,
            title: said.title,
            description: said.description,
        }
    }

    /// A finished lifecycle's outcome lines, from its answer, as one
    /// description (B2): the merge, the sharing and the tidy-up.
    fn lifecycle_lines(&self) -> Option<String> {
        if !matches!(
            self.request.intent,
            Intent::TicketClose { .. } | Intent::DocumentPromote { .. }
        ) {
            return None;
        }
        let envelope = self.operations.lifecycle_answer(self.id())?;
        let reach = Reach::of(envelope, None);
        let lines = outcome_lines(envelope, &reach, &self.names.primary);
        Some(lines_description(&lines))
    }

    /// The operation the request is about: its answer's, else an
    /// OperationResume's or OperationAbandon's own.
    fn operation(&self) -> Option<String> {
        self.operations
            .operation_of(self.request)
            .or_else(|| match &self.request.intent {
                Intent::OperationResume { operation_id, .. }
                | Intent::OperationAbandon { operation_id, .. } => Some(operation_id.to_string()),
                _ => None,
            })
    }

    /// What the log's attention pass reads (A1, A3).
    fn group(&self) -> Group {
        let intent = &self.request.intent;
        let operation = self.operation();
        let running_operation = self
            .operations
            .running_operation(self.id())
            .map(str::to_owned)
            .or_else(|| {
                operation
                    .clone()
                    .filter(|operation| self.operations.poll_runs(operation))
            });
        Group {
            candidate: matches!(
                self.request.state,
                RequestState::Finished(Outcome::Partial | Outcome::Blocked | Outcome::Error, ..)
            ),
            operation,
            roots: self.root.iter().cloned().collect(),
            check: matches!(intent, Intent::PollOnce { .. }),
            check_applied: self.facts.check_applied,
            superseded: self.facts.superseded,
            subject: Some(Subject::Intent {
                family: family_of(intent),
                root: self.root.clone(),
                item: item_of(intent).map(|(_, id)| id.clone()),
                target: target_of(intent),
            }),
            sent: self.operations.sent(self.id()),
            outcome: match &self.request.state {
                RequestState::Finished(outcome, ..) => Some(*outcome),
                _ => None,
            },
            running_operation,
            followed: self.facts.followed,
            follow_void: self.facts.follow_void,
            resend_waits: self.facts.resend_waits,
        }
    }
}

fn offer(action: ActivityAction, label: &str, enabled: bool) -> Offer {
    Offer::new(action, label, enabled)
}

/// An intent's family: it groups the intent with its retries and resends, and
/// a share with any later share of the same item (A3).
fn family_of(intent: &Intent) -> Family {
    match intent {
        Intent::ItemSync { .. } | Intent::ItemRepublish { .. } => Family::ItemShare,
        Intent::RepoSync { .. } => Family::MainShare,
        Intent::PollOnce { .. } => Family::Check,
        Intent::ContextRetire { .. } => Family::Retire,
        Intent::DocumentSave { .. } | Intent::TicketSave { .. } => Family::Save,
        other => Family::Own(other.job_kind()),
    }
}

/// An intent's own target (A3), for identity only and never drawn: a
/// comment, a key, a host, a path, a remote or an operation.
fn target_of(intent: &Intent) -> Option<String> {
    let path = |path: &Path| path.to_string_lossy().into_owned();
    match intent {
        Intent::CommentAdd { comment_id, .. } => Some(comment_id.to_string()),
        Intent::KeySelect { id } | Intent::KeyRemove { id } | Intent::KeyDelete { id } => {
            Some(id.to_string())
        }
        Intent::HostApprove { authority, .. } | Intent::HostReplace { authority, .. } => {
            Some(format!("{}:{}", authority.host, authority.port))
        }
        Intent::DocumentRepair { path: at, .. } | Intent::TicketRepair { path: at, .. } => {
            Some(path(at))
        }
        Intent::FolderCreate { path, .. } => Some(path.clone()),
        Intent::RemoteAdd { name, .. } | Intent::RemoteRemove { name, .. } => Some(name.clone()),
        Intent::RemoteSelect { name, .. } => name.clone(),
        Intent::OperationResume { operation_id, .. }
        | Intent::OperationAbandon { operation_id, .. }
        | Intent::ConflictResolve { operation_id, .. }
        | Intent::LocalContextResolve { operation_id, .. } => Some(operation_id.to_string()),
        _ => None,
    }
}
