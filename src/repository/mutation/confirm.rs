//! Two-phase confirmation: a command whose caller must first be shown
//! what it will do is prepared, and then executed with the confirmation
//! the preview gave.
//!
//! `prepare` validates the input, observes and returns a preview. It
//! writes one confirmation record and nothing else: no directory, no
//! repository, no registration and no Git configuration, and it takes no
//! lease, so nothing waits on a preview that is outstanding.
//!
//! `execute` runs such a command only for a request that presents a
//! confirmation, and `consent` is what it asks of the one presented. The
//! last of the conditions, that a fresh observation gives the digest the
//! preview stored, is asked in `execute` itself once the binding has
//! observed; and the confirmation is marked as accepted in the
//! transaction that writes the request's record.
//!
//! A confirmation is no part of what identifies a request. It is checked
//! once, when a request no record holds presents it. From then on it
//! belongs to that request: the record holds it, a retry of the request is
//! honored however old the confirmation has become, and any other request
//! that presents it is `confirmation_used`. A request that is rejected
//! with nothing left behind has its record deleted, which releases its
//! confirmation in the same transaction for as long as it has to run.

use crate::results::{Effects, Envelope, ResultCode, Scope, absolute_path_string};

use super::{
    super::{RepositoryService, recovery::JournalRow},
    Mutation,
    bind::{
        Accepted, Answer, Binding, CallContext, Confirmed, Entered, Place, Previewed, Ran, Standing,
    },
    command::Command,
    dto::{MutationDataDto, PreviewDto, PreviewEffectDto},
    identity::{ConfirmationId, DigestSalt, IntentDigest, ScopeKey},
    observe::ObservationDigest,
    outcome,
    progress::NoSession,
    records::{ConfirmationRecord, NewConfirmation},
    reply::Reply,
    stored_time,
};

/// How long a confirmation is there for a request to accept: ten minutes
/// from its creation. Once a request has accepted it, it is that
/// request's for as long as the request's record stands.
pub const CONFIRMATION_LIFETIME_SECONDS: i64 = 600;

/// Whether a confirmation made at `created_at`, to expire at
/// `expires_at`, can no longer be accepted at `now`. It lasts from its
/// creation up to its expiry and not at it. A clock that reads earlier
/// than the creation has been set back, and the confirmation is expired
/// then too, so that setting a clock back cannot extend one.
const fn expired(created_at: i64, expires_at: i64, now: i64) -> bool {
    now < created_at || now >= expires_at
}

/// The digest a confirmation `confirmation_id` holds for the request
/// `binding` makes in `scope`: the request's command, repository, target
/// and input, salted with the confirmation's ID.
fn confirmed_digest(
    binding: &dyn Binding,
    scope: &ScopeKey,
    confirmation_id: ConfirmationId,
) -> IntentDigest {
    binding
        .fields(IntentDigest::builder(
            DigestSalt::Confirmation(confirmation_id),
            binding.command().name(),
            scope,
            &binding.target(),
        ))
        .finish()
}

/// The scope key of the place a binding works in. `None` when the place
/// names a root that cannot be resolved.
pub(super) fn scope_key(place: Place<'_>) -> Option<ScopeKey> {
    match place {
        Place::Repository(root) => ScopeKey::for_repository(root),
        Place::NewRepository(root) => ScopeKey::for_new_repository(root),
        Place::Application => Some(ScopeKey::application()),
    }
}

/// The envelope of a `prepare` that gave no preview: a rejection, or a
/// no-op for a command whose end state already holds. It names no
/// request, since `prepare` is of none.
fn without_preview(command: Command, scope: Scope, answer: Answer) -> Envelope<PreviewDto> {
    let Answer {
        code,
        effects,
        // A preview is the only data `prepare` gives.
        data: _,
        // What the binding or a read suggested is kept, as `execute`
        // keeps it; `outcome::offered` decides.
        recovery,
        local_change,
        offers_nothing,
    } = answer;
    let recovery = outcome::offered(
        &outcome::Named {
            code,
            command,
            root: scope.repository.as_deref(),
            request_id: None,
            operation_id: None,
            retry: false,
            unsettled: false,
        },
        recovery,
        offers_nothing,
        None,
    );
    Envelope::classified_mutation(command.name(), scope, false, code, effects, local_change)
        .with_recovery(recovery)
}

/// How the row of a confirmed request's operation reads when the request
/// is entered again: what `Confirmed::row` answers.
pub(crate) enum Row {
    /// No row of the command's operation reads so. Nothing is run, and
    /// the answer is `internal_error`.
    Impossible,
    /// The row answers the request, or what the binding read of it
    /// does: an operation that has ended, or one that cannot go on as
    /// things are. The domain is not called.
    Answers(Box<Ran>),
    /// The operation has recorded that it began its work. The request's
    /// own work has changed what would be observed, so nothing is
    /// compared, and the confirmation is the request's however old it
    /// is: the domain is called. A row there is, then: this answer for
    /// a request with no row is `internal_error`, and nothing is run.
    Recorded,
    /// Nothing of the request's is known to be done: the state is
    /// observed again and compared with what was previewed.
    Unrecorded,
}

impl Row {
    /// A row that answers the request with `ran`.
    pub(crate) fn answers(ran: Ran) -> Self {
        Self::Answers(Box::new(ran))
    }
}

/// What a confirmed binding observed on re-entry under a row that reads
/// `Row::Unrecorded`: what `Confirmed::observe_again` answers.
pub(crate) enum Seen {
    /// The command's end state holds, by the request's work or by
    /// another's. With no row the request is finished with this answer,
    /// a no-op. A row that is pending is the domain's to end, and it is
    /// called; the answer is not used then.
    EndState(Answer),
    /// The state there is now. `found` is its digest. `own` is the
    /// digest of a state the request's own work turned into the one
    /// found before it recorded anything, when there is such a state: a
    /// consent to either is a consent to what is there.
    State {
        found: ObservationDigest,
        own: Option<ObservationDigest>,
    },
}

/// The re-entry of a command that needs confirmation: every such
/// binding is entered again through here, with its three probes.
///
/// In order:
///
/// 1. The row is read; see `Row`. A row that answers the request, or
///    that no operation of the command has, ends the call here. A row
///    that has recorded the request's work is taken up by the domain
///    call with nothing compared. That is never said of a request with
///    no row: a binding that says it is answered `internal_error`.
/// 2. Otherwise the binding observes; see `Seen`. When the command's
///    end state holds and no row stands, the request is a no-op and is
///    finished; with a pending row the domain is called to end it.
/// 3. Otherwise what was observed is compared with what the request's
///    consent was to, `CallContext::previewed`. This is the only thing
///    that keeps a destructive command from running on a stale consent
///    after lost output, and it is made here for every binding: a
///    difference, or a consent that cannot be read, is answered by
///    `Confirmed::differs` and the domain is not called.
pub(crate) fn reenter<B: Confirmed + ?Sized>(
    binding: &mut B,
    call: &CallContext<'_>,
    accepted: &Accepted,
    journal: JournalRow,
) -> Result<Entered, Ran> {
    match binding.row(call, accepted, &journal) {
        Row::Impossible => {
            return Err(Ran {
                answer: Answer::stopped(ResultCode::InternalError),
                standing: Standing::NotRun { journal },
            });
        }
        Row::Answers(ran) => return Err(*ran),
        Row::Recorded => {
            // A request that has no row has recorded nothing. A binding
            // that says otherwise is wrong, and it is the one answer on
            // which nothing would be compared: the domain is not called
            // and the record is left as it is.
            if journal == JournalRow::Absent {
                return Err(Ran {
                    answer: Answer::stopped(ResultCode::InternalError),
                    standing: Standing::Stopped { journal: None },
                });
            }
            return Ok(Entered::Unfinished);
        }
        Row::Unrecorded => {}
    }
    match binding.observe_again(call, accepted, &journal)? {
        Seen::EndState(answer) => {
            if journal == JournalRow::Absent {
                Err(Ran {
                    answer,
                    standing: Standing::Final,
                })
            } else {
                Ok(Entered::Unfinished)
            }
        }
        Seen::State { found, own } => {
            let previewed = call.previewed();
            let consented_to = previewed
                .as_ref()
                .is_some_and(|previewed| *previewed == found || Some(previewed) == own.as_ref());
            if consented_to {
                Ok(Entered::Unfinished)
            } else {
                Err(binding.differs(journal, previewed.is_some()))
            }
        }
    }
}

/// What `consent` found for a request no record holds.
pub(super) enum Consent {
    /// The command needs no confirmation, and the request presents none.
    NotNeeded,
    /// The confirmation the request presents is its own to use.
    Given(Box<ConfirmationRecord>),
    /// The command needs a confirmation, and the request names a root
    /// that cannot be resolved: it has no scope key, so no confirmation
    /// can be of it and none was looked for. This is no consent. The
    /// binding's own check of the root is what answers such a request,
    /// and a request that passed that check all the same is not run.
    Unplaced,
}

impl Consent {
    /// The confirmation, when one was given.
    pub(super) fn given(&self) -> Option<&ConfirmationRecord> {
        match self {
            Self::Given(record) => Some(record),
            Self::NotNeeded | Self::Unplaced => None,
        }
    }
}

impl RepositoryService {
    /// Previews a mutation that needs confirmation, and returns the
    /// confirmation an `execute` of the same command, repository, target
    /// and input presents to run it. Every handled outcome is an envelope.
    ///
    /// It validates the input and observes, and makes the checks the
    /// command itself would be refused for, so a preview is given only of
    /// something that can be done as things stand. It writes one
    /// confirmation record and nothing else, and takes no lease. A command
    /// whose end state already holds is a no-op, and gets no confirmation.
    /// A command that needs no confirmation is `not_confirmable`.
    ///
    /// The envelope names no request: `prepare` changes no canonical or
    /// Git state, and the confirmation ID is how a caller refers to it.
    pub fn prepare(&self, mutation: &Mutation) -> Envelope<PreviewDto> {
        let envelope = self.prepare_call(mutation);
        self.offering(envelope, outcome::CallKind::Prepare)
    }

    /// One call of `prepare`, before the journal is asked what its
    /// envelope can offer.
    fn prepare_call(&self, mutation: &Mutation) -> Envelope<PreviewDto> {
        let command = mutation.kind();
        let Some(mut binding) = mutation.confirmed() else {
            return without_preview(
                command,
                Scope::default(),
                Answer::stopped(ResultCode::NotConfirmable),
            );
        };
        let scope_key = scope_key(binding.place());
        let repository = match (&scope_key, binding.place()) {
            (Some(key), _) => key.repository().map(str::to_owned),
            (None, Place::Repository(root) | Place::NewRepository(root)) => {
                absolute_path_string(root)
            }
            (None, Place::Application) => None,
        };
        let scope = binding.scope(repository, None);
        let call = CallContext {
            service: self,
            session: &NoSession,
            confirmation: None,
        };
        let Previewed {
            observation,
            effects,
            details,
        } = match binding.preview(&call) {
            Ok(previewed) => previewed,
            Err(answer) => return without_preview(command, scope, answer),
        };
        let failed = |code| without_preview(command, scope.clone(), Answer::stopped(code));
        // `preview` resolved the root, so the place has a scope key.
        let Some(scope_key) = scope_key else {
            return failed(ResultCode::InternalError);
        };
        let confirmation_id = ConfirmationId::new();
        let expires_at = self.record_time() + CONFIRMATION_LIFETIME_SECONDS;
        let Ok(expires) = stored_time(expires_at) else {
            return failed(ResultCode::InternalError);
        };
        let inserted = self.insert_confirmation(&NewConfirmation {
            confirmation_id,
            intent_digest: confirmed_digest(binding.as_ref(), &scope_key, confirmation_id),
            scope: scope_key,
            command: command.name().to_owned(),
            target: binding.target(),
            observation_digest: Some(observation),
            expires_at,
        });
        match inserted {
            Ok(true) => {}
            // A new ID that a record already holds: no preview is given
            // under another's confirmation.
            Ok(false) => return failed(ResultCode::InternalError),
            Err(error) => return failed(error.code()),
        }
        Envelope::classified_mutation(
            command.name(),
            scope.clone(),
            false,
            ResultCode::Ok,
            Effects::not_requested(),
            false,
        )
        .with_data(PreviewDto {
            confirmation_id: confirmation_id.to_string(),
            expires_at: expires,
            command: command.name().to_owned(),
            scope,
            effects: effects
                .into_iter()
                .map(|effect| PreviewEffectDto { effect })
                .collect(),
            details,
        })
    }

    /// The consent a request no record holds runs with: the confirmation
    /// it presents, when that is this request's to use. An `Err` answers
    /// the request, with nothing recorded and nothing done.
    ///
    /// In order: a command that needs no confirmation presents none, or
    /// is `not_confirmable`; one that needs it and presents none is
    /// `confirmation_required`; the record must exist, or it is
    /// `confirmation_not_found`; its command, scope, target and input must
    /// be the request's, compared through the digest salted with the
    /// confirmation's ID, or it is `confirmation_mismatch`; no request may
    /// have accepted it, or it is `confirmation_used`; and it must not
    /// have expired, or it is `confirmation_expired`.
    ///
    /// The state the preview observed is compared afterwards, by
    /// `execute`, once the binding has observed again.
    pub(super) fn consent(
        &self,
        reply: &Reply<'_>,
        presented: Option<ConfirmationId>,
    ) -> Result<Consent, Envelope<MutationDataDto>> {
        let binding = reply.binding;
        let confirmation_id = match (binding.command().needs_confirmation(), presented) {
            (false, None) => return Ok(Consent::NotNeeded),
            (false, Some(_)) => return Err(reply.stopped(ResultCode::NotConfirmable)),
            (true, None) => return Err(reply.stopped(ResultCode::ConfirmationRequired)),
            (true, Some(confirmation_id)) => confirmation_id,
        };
        let Some(scope) = &reply.identity.scope else {
            return Ok(Consent::Unplaced);
        };
        let record = match self.confirmation_record(confirmation_id) {
            Ok(Some(record)) => record,
            Ok(None) => return Err(reply.stopped(ResultCode::ConfirmationNotFound)),
            Err(error) => return Err(reply.read_failure(&error)),
        };
        let of_this_request = record.scope == *scope
            && record.command == binding.command().name()
            && record.target == binding.target()
            && record.intent_digest == confirmed_digest(binding, scope, confirmation_id);
        if !of_this_request {
            return Err(reply.stopped(ResultCode::ConfirmationMismatch));
        }
        // A request that has accepted it has a record, and this request
        // has none: it is another's.
        if record.accepted_by.is_some() {
            return Err(reply.stopped(ResultCode::ConfirmationUsed));
        }
        if expired(record.created_at, record.expires_at, self.record_time()) {
            return Err(reply.stopped(ResultCode::ConfirmationExpired));
        }
        Ok(Consent::Given(Box::new(record)))
    }
}

#[cfg(test)]
#[path = "confirm_tests.rs"]
mod reenter_tests;

#[cfg(test)]
mod tests {
    use super::{CONFIRMATION_LIFETIME_SECONDS, expired};

    #[test]
    fn a_confirmation_lasts_ten_minutes_from_its_creation() {
        let created = 1_700_000_000;
        let expires = created + CONFIRMATION_LIFETIME_SECONDS;
        assert_eq!(CONFIRMATION_LIFETIME_SECONDS, 600);
        assert!(!expired(created, expires, created));
        assert!(!expired(created, expires, expires - 1));
        assert!(expired(created, expires, expires));
        assert!(expired(created, expires, expires + 3600));
    }

    #[test]
    fn a_clock_set_back_before_its_creation_does_not_extend_a_confirmation() {
        let created = 1_700_000_000;
        let expires = created + CONFIRMATION_LIFETIME_SECONDS;
        assert!(expired(created, expires, created - 1));
        assert!(expired(created, expires, 0));
    }
}
