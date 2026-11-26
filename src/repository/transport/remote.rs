//! A remote borrow cannot outlive the authenticated connection or secret borrow.
use super::{callbacks::CallbackAttempt, *};
use crate::repository::{
    ConfigurationInspection, RepositoryService, read_configuration,
    remote::{RemoteRefPlan, SynchronizationTarget},
};

struct PollTransfer<'a> {
    plan: &'a RemoteRefPlan,
    progress: &'a mut dyn FnMut(git2::Progress<'_>) -> bool,
    /// Set when the fetch failed because the repository's lease is held.
    lease_busy: &'a std::cell::Cell<bool>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TransferAction {
    Observe,
    Download,
    FetchTracking,
    PushObjects,
    Push,
    Delete(git2::Oid),
    /// The setup channel's four: its fetch of the primary into the base ref,
    /// its push of the setup's branch, and the guarded delete of that branch.
    /// The fourth, its observation, is `Observe`.
    FetchSetupBase,
    #[allow(dead_code)]
    PushSetup(git2::Oid),
    #[allow(dead_code)]
    DeleteSetup(git2::Oid),
}

impl TransferAction {
    /// A download whose tips are written by the call after it, with the
    /// compare-and-swap of `write_tracking_tip`.
    fn writes_tracking(self) -> bool {
        matches!(self, Self::FetchTracking | Self::FetchSetupBase)
    }
    /// A transfer that sends commands to the remote.
    fn sends_commands(self) -> bool {
        matches!(
            self,
            Self::Push | Self::Delete(_) | Self::PushSetup(_) | Self::DeleteSetup(_)
        )
    }
    /// The direction of the connection a setup transfer runs on.
    fn setup_direction(self) -> Option<SshDirection> {
        match self {
            Self::FetchSetupBase => Some(SshDirection::Fetch),
            Self::PushSetup(_) => Some(SshDirection::Push),
            Self::DeleteSetup(_) => Some(SshDirection::Delete),
            _ => None,
        }
    }
}

/// What a setup connection's advertisement holds of the two refs the setup
/// channel reads: the remote's primary, and the setup's branch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SetupAdvertisement {
    pub(crate) primary: Option<git2::Oid>,
    pub(crate) setup: Option<git2::Oid>,
}

/// What one call's push of the setup's branch did. Neither is proof of the
/// result: only `observe_setup_branch`, on a call of its own, is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SetupPush {
    /// The call's advertisement had the branch at the commit already, and
    /// nothing was sent.
    AlreadyThere,
    /// The guarded push was sent and returned.
    Sent,
}

/// What one call's guarded delete did. Neither is proof of the result: only
/// `observe_delete`'s advertisement, on a call of its own, is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DeleteAttempt {
    /// The call's advertisement had no such reference, and nothing was sent.
    Absent,
    /// The guarded delete was sent; the server may have refused or lost it.
    Sent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DeleteObservation {
    Deleted,
    StillPresent(git2::Oid),
    Changed(git2::Oid),
}

/// One remote call's authenticated connection. Every listing and transfer
/// of the call runs on it and under its callback attempt, so the call
/// submits the selected key once, to the host it finalized; a reconnect
/// inside libgit2 asks that attempt again and fails closed.
pub(crate) struct AuthenticatedSshRemote<'repo, 'a> {
    remote: &'a mut git2::Remote<'repo>,
    attempt: &'a CallbackAttempt,
    service: &'a RepositoryService,
    prepared: &'a PreparedSshAttempt,
    context: &'a SshTransportError,
    passphrase: Option<&'a str>,
    agent: Option<&'a super::agent::AgentSigner>,
    observed: HostKeyIdentity,
    trust: super::trust::HostTrustSnapshot,
}
impl<'repo, 'a> AuthenticatedSshRemote<'repo, 'a> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        remote: &'a mut git2::Remote<'repo>,
        attempt: &'a CallbackAttempt,
        service: &'a RepositoryService,
        prepared: &'a PreparedSshAttempt,
        context: &'a SshTransportError,
        passphrase: Option<&'a str>,
        agent: Option<&'a super::agent::AgentSigner>,
        observed: HostKeyIdentity,
    ) -> Self {
        let mut trust = prepared.trust.clone();
        if super::trust::approval_matches(
            prepared.approval.as_ref(),
            &prepared.endpoint.authority,
            prepared.trust.pin.as_ref(),
            &observed,
        ) {
            trust.pin = Some(observed.clone());
        }
        Self {
            remote,
            attempt,
            service,
            prepared,
            context,
            passphrase,
            agent,
            observed,
            trust,
        }
    }
    /// Pin the actual authenticated handle to the action's original endpoint;
    /// the call's later transfers run on this handle, under the prepared policy.
    pub(in super::super) fn require_action_expectation(
        &self,
        expected: &SshScopeExpectation,
    ) -> Result<(), SshTransportError> {
        self.service
            .recheck_ssh(self.prepared, self.context)
            .map_err(|kind| self.context.with_kind(kind))?;
        self.recheck_trust()?;
        let effective = match self.context.direction {
            SshDirection::Fetch => self.remote.url(),
            SshDirection::Push | SshDirection::Delete => {
                self.remote.pushurl().or_else(|| self.remote.url())
            }
        };
        expected
            .check(self.prepared)
            .map_err(|kind| self.context.with_kind(kind))?;
        if effective
            .and_then(|url| super::endpoint::parse_ssh_endpoint(url).ok())
            .as_ref()
            != Some(&expected.endpoint)
        {
            return Err(self
                .context
                .with_kind(SshTransportErrorKind::EndpointChanged));
        }
        Ok(())
    }

    /// The call's connection can still carry the call. A push closes its
    /// connection as it returns. On a closed one, libgit2 would hand back the
    /// listing it cached at connect, which proves nothing about the remote now,
    /// and a transfer would reconnect inside libgit2, under whatever
    /// authentication the server grants (R-SDD-51). Both refuse here first.
    fn require_connected(&mut self) -> Result<(), SshTransportError> {
        if self.remote.connected() {
            Ok(())
        } else {
            Err(self
                .context
                .with_kind(SshTransportErrorKind::ProtocolFailure))
        }
    }
    /// This connection's advertisement; none once the connection has closed.
    pub(crate) fn advertisement(&mut self) -> Result<Vec<(String, git2::Oid)>, SshTransportError> {
        self.require_connected()?;
        self.remote
            .list()
            .map(|heads| {
                heads
                    .iter()
                    .map(|head| (head.name().to_owned(), head.oid()))
                    .collect()
            })
            .map_err(|_| {
                self.context
                    .with_kind(SshTransportErrorKind::ProtocolFailure)
            })
    }
    // Task 4 owns safe points between these calls and complete pre/post OID
    // comparison. A transfer return is never proof of publication or deletion.
    /// The call's connection's advertisement, after the rechecks a transfer
    /// makes. It is fresh evidence only before that connection's transfer.
    pub(crate) fn fresh_advertisement(
        &mut self,
    ) -> Result<Vec<(String, git2::Oid)>, SshTransportError> {
        let advertised = self.transfer(&[], TransferAction::Observe, None)?;
        #[cfg(test)]
        if self.context.direction == SshDirection::Push {
            super::operation_tests::checkpoint(
                super::operation_tests::Checkpoint::PushAdvertisementObserved,
            );
        }
        Ok(advertised)
    }

    pub(crate) fn fetch_exact(
        &mut self,
        plan: &RemoteRefPlan,
        target: &SynchronizationTarget,
    ) -> Result<(), SshTransportError> {
        self.fetch_exact_advertised(plan, target).map(|_| ())
    }

    /// `fetch_exact`, giving back the advertisement its download negotiated
    /// against: the call's connection's, the fetch endpoint's refs before
    /// the download.
    pub(crate) fn fetch_exact_advertised(
        &mut self,
        plan: &RemoteRefPlan,
        target: &SynchronizationTarget,
    ) -> Result<Vec<(String, git2::Oid)>, SshTransportError> {
        self.validate_plan(plan, SshDirection::Fetch)?;
        let advertised = self.fresh_advertisement()?;
        self.validate_plan(plan, SshDirection::Fetch)?;
        let mappings = exact_mappings(plan, target, SshDirection::Fetch);
        let present: Vec<_> = mappings
            .iter()
            .filter(|mapping| {
                let source = mapping
                    .strip_prefix('+')
                    .unwrap()
                    .split_once(':')
                    .unwrap()
                    .0;
                advertised.iter().any(|(name, _)| name == source)
            })
            .map(String::as_str)
            .collect();
        // Empty libgit2 refspec lists select configured/default mappings; never
        // call a transfer with one when the exact advertised scope is absent.
        if present.is_empty() {
            return Ok(advertised);
        }
        self.transfer(
            &present,
            TransferAction::FetchTracking,
            Some((plan, target)),
        )?;
        Ok(advertised)
    }

    /// Fetch only parsed advertised contexts and the advertised primary. The
    /// caller owns throttling and stop intent; this hook has no reservation
    /// knowledge and runs only during download, outside the repository lease.
    #[cfg(test)]
    pub(crate) fn fetch_poll_set(
        &mut self,
        plan: &RemoteRefPlan,
        progress: impl FnMut(git2::Progress<'_>) -> bool,
    ) -> Result<Vec<(String, git2::Oid)>, SshTransportError> {
        self.fetch_poll_set_noting_lease(plan, &std::cell::Cell::new(false), progress)
    }

    /// `lease_busy` tells a held lease from a protocol failure, which the
    /// error's kind doesn't: only a poll answers it differently.
    pub(crate) fn fetch_poll_set_noting_lease(
        &mut self,
        plan: &RemoteRefPlan,
        lease_busy: &std::cell::Cell<bool>,
        mut progress: impl FnMut(git2::Progress<'_>) -> bool,
    ) -> Result<Vec<(String, git2::Oid)>, SshTransportError> {
        self.validate_plan(plan, SshDirection::Fetch)?;
        self.transfer_inner(
            &[],
            TransferAction::FetchTracking,
            None,
            Some(PollTransfer {
                plan,
                progress: &mut progress,
                lease_busy,
            }),
        )
    }

    pub(crate) fn push_exact(
        &mut self,
        plan: &RemoteRefPlan,
        target: &SynchronizationTarget,
    ) -> Result<(), SshTransportError> {
        self.validate_plan(plan, SshDirection::Push)?;
        #[cfg(test)]
        super::operation_tests::checkpoint(super::operation_tests::Checkpoint::ExactPushStarted);
        let mappings = exact_mappings(plan, target, SshDirection::Push);
        self.transfer(
            &[mappings[0].as_str()],
            TransferAction::Push,
            Some((plan, target)),
        )
        .map(|_| ())
    }

    /// One guarded command at most, on the call's connection. A fresh
    /// authenticated advertisement is the only result evidence, including after
    /// a refused or ambiguously lost push; the push closes this connection as
    /// it returns, so that advertisement is `observe_delete`'s, on a call of its
    /// own.
    pub(crate) fn delete_exact(
        &mut self,
        plan: &RemoteRefPlan,
        target: &SynchronizationTarget,
        expected: git2::Oid,
    ) -> Result<DeleteAttempt, SshTransportError> {
        let (refs, reference) = self.delete_scope(plan, target, expected)?;
        // A retry must observe first: never blindly resend an ambiguous delete.
        if !self
            .fresh_advertisement()?
            .iter()
            .any(|(name, _)| *name == reference)
        {
            return Ok(DeleteAttempt::Absent);
        }
        let refs: Vec<_> = refs.iter().map(String::as_str).collect();
        match self.transfer(
            &refs,
            TransferAction::Delete(expected),
            Some((plan, target)),
        ) {
            Ok(_) => {}
            Err(error)
                if matches!(
                    error.kind,
                    SshTransportErrorKind::PushRejected
                        | SshTransportErrorKind::TransportUnavailable
                        | SshTransportErrorKind::RemoteUnavailable
                ) => {}
            Err(error) => return Err(error),
        }
        #[cfg(test)]
        super::operation_tests::checkpoint(
            super::operation_tests::Checkpoint::DeleteAttemptReturned,
        );
        Ok(DeleteAttempt::Sent)
    }

    /// What a sent `delete_exact` left, from this call's own advertisement.
    pub(crate) fn observe_delete(
        &mut self,
        plan: &RemoteRefPlan,
        target: &SynchronizationTarget,
        expected: git2::Oid,
    ) -> Result<DeleteObservation, SshTransportError> {
        let (_, reference) = self.delete_scope(plan, target, expected)?;
        let advertised = self.fresh_advertisement()?;
        Ok(
            match advertised.iter().find(|(name, _)| *name == reference) {
                None => DeleteObservation::Deleted,
                Some((_, oid)) if *oid == expected => DeleteObservation::StillPresent(*oid),
                Some((_, oid)) => DeleteObservation::Changed(*oid),
            },
        )
    }

    /// The exact delete mapping of `target` and the reference it deletes.
    fn delete_scope(
        &self,
        plan: &RemoteRefPlan,
        target: &SynchronizationTarget,
        expected: git2::Oid,
    ) -> Result<(Vec<String>, String), SshTransportError> {
        self.validate_plan(plan, SshDirection::Delete)?;
        let mappings = exact_mappings(plan, target, SshDirection::Delete);
        let refs: Vec<_> = mappings.iter().map(String::as_str).collect();
        if expected.is_zero() || !mappings_match_scope(plan, target, SshDirection::Delete, &refs) {
            return Err(self
                .context
                .with_kind(SshTransportErrorKind::ConfigurationInvalid));
        }
        let reference = mappings[0].strip_prefix(':').unwrap().to_owned();
        Ok((mappings, reference))
    }

    /// Download only the selected Push destination's commit graph. Upload-pack
    /// is explicitly bound to its policy-resolved endpoint, not the fetch URL.
    pub(crate) fn download_push_target(
        &mut self,
        plan: &RemoteRefPlan,
        target: &SynchronizationTarget,
    ) -> Result<Option<git2::Oid>, SshTransportError> {
        self.validate_plan(plan, SshDirection::Push)?;
        let repository = git2::Repository::open(&self.context.root).map_err(|_| {
            self.context
                .with_kind(SshTransportErrorKind::ProtocolFailure)
        })?;
        let mut remote = repository
            .remote_anonymous(&self.prepared.endpoint.connection_url)
            .map_err(|_| {
                self.context
                    .with_kind(SshTransportErrorKind::ProtocolFailure)
            })?;
        let source = match target {
            SynchronizationTarget::Primary => plan.primary().clone(),
            SynchronizationTarget::Context { kind, item_id } => plan.context(*kind, item_id),
        };
        // Upload-pack on the Push endpoint is a connection of its own, made
        // here with its own attempt under this call's policy: one key
        // submission, to the host this call finalized, with trust unchanged.
        self.service
            .recheck_ssh(self.prepared, self.context)
            .map_err(|kind| self.context.with_kind(kind))?;
        self.recheck_trust()?;
        let attempt = CallbackAttempt::for_finalized_host(self.observed.clone());
        let mut connection = remote
            .connect_auth(
                git2::Direction::Fetch,
                Some(super::callbacks::build_callbacks(
                    self.prepared,
                    self.passphrase,
                    self.agent,
                    &attempt,
                )),
                None,
            )
            .map_err(|error| {
                self.context
                    .with_kind(backend_failure(&attempt, &error, self.passphrase.is_some()))
            })?;
        let observed = attempt.observed_host().ok_or_else(|| {
            self.context
                .with_kind(SshTransportErrorKind::HostVerificationUnavailable)
        })?;
        if attempt.key_submissions() != 1 {
            return Err(self.context.with_kind(SshTransportErrorKind::KeyRejected));
        }
        self.service
            .recheck_ssh(self.prepared, self.context)
            .map_err(|kind| self.context.with_kind(kind))?;
        self.recheck_trust()?;
        if observed != self.observed {
            return Err(self
                .context
                .with_kind(SshTransportErrorKind::HostTrustChanged));
        }
        let mut scoped = AuthenticatedSshRemote {
            remote: connection.remote(),
            attempt: &attempt,
            service: self.service,
            prepared: self.prepared,
            context: self.context,
            passphrase: self.passphrase,
            agent: self.agent,
            observed: self.observed.clone(),
            trust: self.trust.clone(),
        };
        let advertised = scoped.transfer(
            &[source.remote_ref()],
            TransferAction::PushObjects,
            Some((plan, target)),
        )?;
        let oid = advertised
            .iter()
            .find(|(name, _)| name == source.remote_ref())
            .map(|(_, oid)| *oid);
        if oid.is_none_or(|oid| repository.find_commit(oid).is_err()) {
            return Err(self
                .context
                .with_kind(SshTransportErrorKind::ProtocolFailure));
        }
        Ok(oid)
    }

    /// The scope of a setup connection in `direction`. A connection whose
    /// source is not a setup's, or runs in another direction, is refused
    /// before anything is read or sent.
    fn setup_scope(&self, direction: SshDirection) -> Result<&'a SetupScope, SshTransportError> {
        match &self.prepared.remote_source {
            RemoteSource::Setup(scope) if self.context.direction == direction => Ok(scope),
            RemoteSource::Setup(_) | RemoteSource::Configured => Err(self
                .context
                .with_kind(SshTransportErrorKind::ConfigurationInvalid)),
        }
    }

    /// The one mapping of the setup channel in the connection's direction.
    fn setup_mapping(&self, direction: SshDirection) -> Result<String, SshTransportError> {
        let mut mappings = setup_mappings(&self.prepared.remote_source, direction);
        match (mappings.pop(), mappings.is_empty()) {
            (Some(mapping), true) => Ok(mapping),
            _ => Err(self
                .context
                .with_kind(SshTransportErrorKind::ConfigurationInvalid)),
        }
    }

    /// On a fetch connection of a setup: fetches the remote's primary into
    /// the setup's private base ref, never into the remote-tracking ref, so
    /// that it cannot race a poll. The ref is written with
    /// `write_tracking_tip`'s compare-and-swap. Gives back the connection's
    /// advertisement of the primary and of the setup's branch; with no
    /// primary advertised nothing is fetched.
    pub(crate) fn fetch_setup_base(&mut self) -> Result<SetupAdvertisement, SshTransportError> {
        let scope = self.setup_scope(SshDirection::Fetch)?;
        let mapping = self.setup_mapping(SshDirection::Fetch)?;
        let advertised = self.fresh_advertisement()?;
        let primary = setup_primary_ref(scope).ok_or_else(|| {
            self.context
                .with_kind(SshTransportErrorKind::ConfigurationInvalid)
        })?;
        let found = SetupAdvertisement {
            primary: advertised_oid(&advertised, &primary),
            setup: advertised_oid(&advertised, &scope.branch_ref()),
        };
        // An empty refspec list selects configured and default mappings in
        // libgit2: never call a transfer with one.
        if found.primary.is_some() {
            self.transfer(&[mapping.as_str()], TransferAction::FetchSetupBase, None)?;
        }
        Ok(found)
    }

    /// On a push connection of a setup: pushes the setup's branch, without
    /// force, as the one update from no ref to `commit`; `guard_create`
    /// refuses anything else before a command is sent. The call's own
    /// advertisement is read first: the branch already at `commit` sends
    /// nothing, and the branch at another OID is refused as `PushRejected`.
    /// A `PushRejected`, `TransportUnavailable` or `RemoteUnavailable` may
    /// follow a refused guard or a command that reached the server, so the
    /// outcome is read by `observe_setup_branch` on a call of its own.
    pub(crate) fn push_setup_branch(
        &mut self,
        commit: git2::Oid,
    ) -> Result<SetupPush, SshTransportError> {
        let scope = self.setup_scope(SshDirection::Push)?;
        let mapping = self.setup_mapping(SshDirection::Push)?;
        if commit.is_zero() {
            return Err(self
                .context
                .with_kind(SshTransportErrorKind::ConfigurationInvalid));
        }
        match advertised_oid(&self.fresh_advertisement()?, &scope.branch_ref()) {
            Some(oid) if oid == commit => return Ok(SetupPush::AlreadyThere),
            Some(_) => {
                return Err(self.context.with_kind(SshTransportErrorKind::PushRejected));
            }
            None => {}
        }
        self.transfer(&[mapping.as_str()], TransferAction::PushSetup(commit), None)?;
        Ok(SetupPush::Sent)
    }

    /// On a delete connection of a setup: one guarded command at most,
    /// deleting the setup's branch while it is at `expected`
    /// (`guard_delete`). As `delete_exact` does, it reads the call's
    /// advertisement first and sends nothing for a branch that is absent.
    /// The outcome of a sent delete, and of a refused or lost one, is read
    /// by `observe_setup_branch` on a call of its own.
    pub(crate) fn delete_setup_branch(
        &mut self,
        expected: git2::Oid,
    ) -> Result<DeleteAttempt, SshTransportError> {
        let scope = self.setup_scope(SshDirection::Delete)?;
        let mapping = self.setup_mapping(SshDirection::Delete)?;
        if expected.is_zero() {
            return Err(self
                .context
                .with_kind(SshTransportErrorKind::ConfigurationInvalid));
        }
        if advertised_oid(&self.fresh_advertisement()?, &scope.branch_ref()).is_none() {
            return Ok(DeleteAttempt::Absent);
        }
        self.transfer(
            &[mapping.as_str()],
            TransferAction::DeleteSetup(expected),
            None,
        )?;
        Ok(DeleteAttempt::Sent)
    }

    /// On a fetch connection of a setup, of its own: the setup's branch as
    /// this call's fresh advertisement has it, which is the only evidence of
    /// what a push or a delete left.
    pub(crate) fn observe_setup_branch(&mut self) -> Result<Option<git2::Oid>, SshTransportError> {
        let scope = self.setup_scope(SshDirection::Fetch)?;
        Ok(advertised_oid(
            &self.fresh_advertisement()?,
            &scope.branch_ref(),
        ))
    }

    fn validate_plan(
        &self,
        plan: &RemoteRefPlan,
        direction: SshDirection,
    ) -> Result<(), SshTransportError> {
        if !item_scope_allowed(&self.prepared.remote_source)
            || !plan_matches_configuration(plan, self.context)
            || direction != self.context.direction
        {
            return Err(self
                .context
                .with_kind(SshTransportErrorKind::ConfigurationInvalid));
        }
        Ok(())
    }

    // Legacy object-only primitives remain for the Cycle 03 fixture regression;
    // synchronization uses only the typed exact methods above.
    #[allow(dead_code)]
    pub(crate) fn download(&mut self, refspecs: &[&str]) -> Result<(), SshTransportError> {
        self.transfer(refspecs, TransferAction::Download, None)
            .map(|_| ())
    }
    #[allow(dead_code)]
    pub(crate) fn push(&mut self, refspecs: &[&str]) -> Result<(), SshTransportError> {
        self.transfer(refspecs, TransferAction::Push, None)
            .map(|_| ())
    }
    fn transfer(
        &mut self,
        refspecs: &[&str],
        action: TransferAction,
        scope: Option<(&RemoteRefPlan, &SynchronizationTarget)>,
    ) -> Result<Vec<(String, git2::Oid)>, SshTransportError> {
        self.transfer_inner(refspecs, action, scope, None)
    }

    fn transfer_inner(
        &mut self,
        refspecs: &[&str],
        action: TransferAction,
        scope: Option<(&RemoteRefPlan, &SynchronizationTarget)>,
        mut poll: Option<PollTransfer<'_>>,
    ) -> Result<Vec<(String, git2::Oid)>, SshTransportError> {
        // A setup source carries the setup channel's one mapping in its
        // direction and none of an item's; any other source carries none of
        // the channel's.
        let source = &self.prepared.remote_source;
        let sourced = match action.setup_direction() {
            Some(direction) => {
                scope.is_none()
                    && poll.is_none()
                    && direction == self.context.direction
                    && setup_mappings_match_scope(source, direction, refspecs)
            }
            None => action == TransferAction::Observe || item_scope_allowed(source),
        };
        if !sourced {
            return Err(self
                .context
                .with_kind(SshTransportErrorKind::ConfigurationInvalid));
        }
        if let Some(poll) = poll.as_ref() {
            self.validate_plan(poll.plan, SshDirection::Fetch)?;
        }
        if let Some((plan, target)) = scope
            && (!plan_matches_configuration(plan, self.context)
                || if action == TransferAction::PushObjects {
                    let selected = match target {
                        SynchronizationTarget::Primary => plan.primary().clone(),
                        SynchronizationTarget::Context { kind, item_id } => {
                            plan.context(*kind, item_id)
                        }
                    };
                    self.context.direction != SshDirection::Push
                        || refspecs != [selected.remote_ref()]
                } else {
                    !mappings_match_scope(plan, target, self.context.direction, refspecs)
                })
        {
            return Err(self
                .context
                .with_kind(SshTransportErrorKind::ConfigurationInvalid));
        }
        let push = matches!(
            self.context.direction,
            SshDirection::Push | SshDirection::Delete
        ) && action != TransferAction::PushObjects;
        if (matches!(action, TransferAction::Delete(_))
            && (self.context.direction != SshDirection::Delete || scope.is_none()))
            || (action == TransferAction::Push && self.context.direction != SshDirection::Push)
            || (action != TransferAction::Observe && action.sends_commands() != push)
        {
            return Err(self
                .context
                .with_kind(SshTransportErrorKind::ConfigurationInvalid));
        }
        self.service
            .recheck_ssh(self.prepared, self.context)
            .map_err(|kind| self.context.with_kind(kind))?;
        self.recheck_trust()?;
        // The transfer runs on the call's connection, under its callback
        // attempt: no reconnect, no second key submission, and the host is
        // the one this connection finalized. A closed connection refuses
        // before libgit2 could reconnect; the attempt's refusal of a second
        // credential request is the second line.
        self.require_connected()?;
        #[cfg(test)]
        super::operation_tests::checkpoint(super::operation_tests::Checkpoint::TransferStarted);
        if action == TransferAction::Observe {
            return self.advertisement();
        }
        if let Some((plan, _)) = scope
            && !plan_matches_configuration(plan, self.context)
        {
            return Err(self
                .context
                .with_kind(SshTransportErrorKind::ConfigurationInvalid));
        }
        // The poll's set and returned OIDs are this connection's advertisement,
        // the one its download negotiates against.
        let poll_advertisement = if poll.is_some() {
            self.advertisement()?
        } else {
            Vec::new()
        };
        let poll_mapping_set = poll
            .as_ref()
            .map(|poll| poll_mappings(poll.plan, &poll_advertisement));
        let poll_refspecs: Vec<_> = poll_mapping_set
            .as_ref()
            .into_iter()
            .flatten()
            .map(String::as_str)
            .collect();
        let refspecs = if poll.is_some() {
            poll_refspecs.as_slice()
        } else {
            refspecs
        };
        // Empty download mappings select configured/default mappings in libgit2.
        // An empty parsed poll set must stop here, without a download or prune.
        if poll.is_some() && refspecs.is_empty() {
            return Ok(poll_advertisement);
        }
        let tracking_repository = if action.writes_tracking() {
            Some(git2::Repository::open(&self.context.root).map_err(|_| {
                self.context
                    .with_kind(SshTransportErrorKind::ProtocolFailure)
            })?)
        } else {
            None
        };
        let object_advertisement = if action == TransferAction::PushObjects {
            let advertised = self.advertisement()?;
            if !advertised.iter().any(|(name, _)| name == refspecs[0]) {
                return Err(self
                    .context
                    .with_kind(SshTransportErrorKind::ProtocolFailure));
            }
            advertised
        } else {
            Vec::new()
        };
        let mut tracking_updates = Vec::new();
        if let Some(repository) = tracking_repository.as_ref() {
            let heads = self.advertisement()?;
            for mapping in refspecs {
                let (source, destination) = mapping
                    .strip_prefix('+')
                    .and_then(|mapping| mapping.split_once(':'))
                    .ok_or_else(|| {
                        self.context
                            .with_kind(SshTransportErrorKind::ConfigurationInvalid)
                    })?;
                let oid = heads
                    .iter()
                    .find(|(name, _)| name == source)
                    .map(|(_, oid)| *oid)
                    .ok_or_else(|| {
                        self.context
                            .with_kind(SshTransportErrorKind::ProtocolFailure)
                    })?;
                let old = tracking_oid(repository, destination).map_err(|_| {
                    self.context
                        .with_kind(SshTransportErrorKind::ProtocolFailure)
                })?;
                tracking_updates.push((destination.to_owned(), oid, old));
            }
        }
        let rejected = std::cell::Cell::new(false);
        let attempt = self.attempt;
        let mut callbacks =
            super::callbacks::build_callbacks(self.prepared, self.passphrase, self.agent, attempt);
        match action {
            TransferAction::Delete(expected) | TransferAction::DeleteSetup(expected) => {
                super::callbacks::guard_delete(
                    &mut callbacks,
                    refspecs[0].strip_prefix(':').unwrap(),
                    expected,
                );
            }
            TransferAction::PushSetup(commit) => {
                super::callbacks::guard_create(
                    &mut callbacks,
                    refspecs[0].split_once(':').unwrap().1,
                    commit,
                );
            }
            _ => {}
        }
        callbacks.push_update_reference(|_, status| {
            if status.is_some() {
                rejected.set(true);
            }
            Ok(())
        });
        if let Some(poll) = poll.as_mut() {
            callbacks.transfer_progress(|stats| attempt.poll_progress(|| (poll.progress)(stats)));
        }
        // libgit2 replaces prior options even on a connected remote. Every set
        // carries the connection's policy and attempt: a reconnect inside
        // libgit2 (a push closes its connection as it returns) asks that
        // attempt for credentials again and fails closed.
        let result = if push {
            let mut options = git2::PushOptions::new();
            options.remote_callbacks(callbacks);
            self.remote.push(refspecs, Some(&mut options))
        } else {
            let mut options = git2::FetchOptions::new();
            options.remote_callbacks(callbacks);
            options.update_fetchhead(false);
            if action.writes_tracking() || action == TransferAction::PushObjects {
                options.download_tags(git2::AutotagOption::None);
                options.prune(git2::FetchPrune::Off);
            }
            self.remote.download(refspecs, Some(&mut options))
        };
        // Locked libgit2 reports GenericError after callback=false, not User.
        // Preserve explicit stop intent and never enter the tracking-write stage.
        if attempt.poll_stopped() {
            return Err(self
                .context
                .with_kind(SshTransportErrorKind::TransportUnavailable));
        }
        if let Some(kind) = attempt.failure() {
            return Err(self.context.with_kind(
                if self.passphrase.is_some() && super::operation::authentication_failure(&kind) {
                    SshTransportErrorKind::UnlockFailed
                } else {
                    kind
                },
            ));
        }
        if rejected.get() {
            return Err(self.context.with_kind(SshTransportErrorKind::PushRejected));
        }
        result.map_err(|error| {
            self.context
                .with_kind(if push && error.code() == git2::ErrorCode::NotFastForward {
                    SshTransportErrorKind::PushRejected
                } else {
                    backend_failure(attempt, &error, self.passphrase.is_some())
                })
        })?;
        #[cfg(test)]
        if action == TransferAction::FetchTracking {
            super::operation_tests::checkpoint(
                super::operation_tests::Checkpoint::TrackingDownloaded,
            );
        }
        if let Some((plan, _)) = scope
            && !plan_matches_configuration(plan, self.context)
        {
            return Err(self
                .context
                .with_kind(SshTransportErrorKind::ConfigurationInvalid));
        }
        self.service
            .recheck_ssh(self.prepared, self.context)
            .map_err(|kind| self.context.with_kind(kind))?;
        if self
            .service
            .read_host_trust(&self.prepared.endpoint.authority)
            .map_err(|kind| self.context.with_kind(kind))?
            != self.trust
        {
            return Err(self
                .context
                .with_kind(SshTransportErrorKind::HostTrustChanged));
        }
        if let Some(repository) = tracking_repository.as_ref() {
            // Locked libgit2 update_tips truncates FETCH_HEAD even with its flag
            // disabled. Write only exact tracking tips after download instead.
            // No network call or prompt occurs while this short lease is held.
            let _lease = crate::repository::coordination::repository_lease(
                repository,
                &self.context.root,
                crate::repository::RepositoryOperation::Inspect,
            )
            .map_err(|error| {
                if let Some(poll) = poll.as_ref()
                    && error.kind == crate::repository::RepositoryErrorKind::RepositoryBusy
                {
                    poll.lease_busy.set(true);
                }
                self.context
                    .with_kind(SshTransportErrorKind::ProtocolFailure)
            })?;
            let valid_scope = match (scope, poll.as_ref()) {
                (Some((plan, target)), None) => {
                    plan_matches_configuration(plan, self.context)
                        && mappings_match_scope(plan, target, self.context.direction, refspecs)
                }
                (None, Some(poll)) => {
                    plan_matches_configuration(poll.plan, self.context)
                        && poll_mappings_match_scope(
                            poll.plan,
                            &poll_advertisement,
                            self.context.direction,
                            refspecs,
                        )
                }
                (None, None) if action == TransferAction::FetchSetupBase => {
                    setup_mappings_match_scope(
                        &self.prepared.remote_source,
                        self.context.direction,
                        refspecs,
                    )
                }
                _ => false,
            };
            if !valid_scope {
                return Err(self
                    .context
                    .with_kind(SshTransportErrorKind::ConfigurationInvalid));
            }
            self.service
                .recheck_ssh(self.prepared, self.context)
                .map_err(|kind| self.context.with_kind(kind))?;
            if self
                .service
                .read_host_trust(&self.prepared.endpoint.authority)
                .map_err(|kind| self.context.with_kind(kind))?
                != self.trust
            {
                return Err(self
                    .context
                    .with_kind(SshTransportErrorKind::HostTrustChanged));
            }
            // Validate every object and old tip before the first write. Failures
            // after a write retain inspectable metadata, never completed proof.
            for (destination, oid, old) in &tracking_updates {
                repository.find_commit(*oid).map_err(|_| {
                    self.context
                        .with_kind(SshTransportErrorKind::ProtocolFailure)
                })?;
                if tracking_oid(repository, destination).map_err(|_| {
                    self.context
                        .with_kind(SshTransportErrorKind::ProtocolFailure)
                })? != *old
                {
                    return Err(self
                        .context
                        .with_kind(SshTransportErrorKind::ProtocolFailure));
                }
            }
            for (destination, oid, old) in tracking_updates {
                #[cfg(test)]
                super::operation_tests::checkpoint(
                    super::operation_tests::Checkpoint::BeforeTrackingWrite,
                );
                write_tracking_tip(repository, &destination, oid, old).map_err(|_| {
                    self.context
                        .with_kind(SshTransportErrorKind::ProtocolFailure)
                })?;
                #[cfg(test)]
                super::operation_tests::checkpoint(
                    super::operation_tests::Checkpoint::TrackingWritten,
                );
            }
        }
        Ok(if poll.is_some() {
            poll_advertisement
        } else {
            object_advertisement
        })
    }
    fn recheck_trust(&self) -> Result<(), SshTransportError> {
        let trust = self
            .service
            .read_host_trust(&self.prepared.endpoint.authority)
            .map_err(|kind| self.context.with_kind(kind))?;
        if trust != self.trust {
            return Err(self
                .context
                .with_kind(SshTransportErrorKind::HostTrustChanged));
        }
        Ok(())
    }
    /// The registry connection this call's key reads go through.
    pub(in super::super) fn call_registry(&self) -> &crate::repository::CallRegistry {
        &self.prepared.registry
    }
    pub(super) fn verified(&self) -> SshTransportVerified {
        SshTransportVerified {
            root: self.context.root.clone(),
            remote_name: self.context.remote_name.clone(),
            selected_key_id: self.prepared.registration.id,
            direction: self.context.direction,
            authority: self.prepared.endpoint.authority.clone(),
            host_key: self.observed.clone(),
        }
    }
}

pub(super) fn backend_failure(
    attempt: &CallbackAttempt,
    error: &git2::Error,
    supplied: bool,
) -> SshTransportErrorKind {
    if let Some(kind) = attempt.failure() {
        return if supplied && super::operation::authentication_failure(&kind) {
            SshTransportErrorKind::UnlockFailed
        } else {
            kind
        };
    }
    if attempt.passthrough() && error.code() == git2::ErrorCode::Certificate {
        return attempt
            .observed_host()
            .map(|presented| SshTransportErrorKind::HostApprovalRequired { presented })
            .unwrap_or(SshTransportErrorKind::HostVerificationUnavailable);
    }
    match error.code() {
        git2::ErrorCode::Auth => {
            if supplied {
                SshTransportErrorKind::UnlockFailed
            } else {
                SshTransportErrorKind::KeyRejected
            }
        }
        git2::ErrorCode::Certificate => SshTransportErrorKind::HostVerificationUnavailable,
        git2::ErrorCode::NotFound => SshTransportErrorKind::RemoteUnavailable,
        _ => SshTransportErrorKind::TransportUnavailable,
    }
}

fn exact_mappings(
    plan: &RemoteRefPlan,
    target: &SynchronizationTarget,
    direction: SshDirection,
) -> Vec<String> {
    match (direction, target) {
        (SshDirection::Fetch, SynchronizationTarget::Primary) => vec![plan.primary_fetch_refspec()],
        (SshDirection::Fetch, SynchronizationTarget::Context { kind, item_id }) => {
            plan.context_fetch_refspec(*kind, item_id).into()
        }
        (SshDirection::Push, SynchronizationTarget::Primary) => vec![plan.primary_push_refspec()],
        (SshDirection::Push, SynchronizationTarget::Context { kind, item_id }) => {
            vec![plan.context_push_refspec(*kind, item_id)]
        }
        // One exact delete of the target's own context ref, for a document
        // or a ticket context (named edit 5); never the primary or a pattern.
        (SshDirection::Delete, SynchronizationTarget::Context { kind, item_id }) => {
            vec![format!(":{}", plan.context(*kind, item_id).remote_ref())]
        }
        (SshDirection::Delete, SynchronizationTarget::Primary) => Vec::new(),
    }
}

/// The plan whose primary a setup fetches: the scope's names, valid as any
/// configuration's are.
fn setup_plan(scope: &SetupScope) -> Option<RemoteRefPlan> {
    RemoteRefPlan::from_configuration(&scope.remote_name, &scope.primary_branch).ok()
}

/// The remote's primary ref a setup fetches.
fn setup_primary_ref(scope: &SetupScope) -> Option<String> {
    Some(setup_plan(scope)?.primary().remote_ref().to_owned())
}

/// The OID `name` has in an advertisement, if it is in it.
fn advertised_oid(advertised: &[(String, git2::Oid)], name: &str) -> Option<git2::Oid> {
    advertised
        .iter()
        .find(|(advertised, _)| advertised == name)
        .map(|(_, oid)| *oid)
}

/// The one mapping the setup channel makes in `direction`, for a setup
/// source: the fetch of the remote's primary into the setup's private base
/// ref, the push of the setup's branch to itself, or the delete of that
/// branch. A configured source, or a scope whose names make no valid plan,
/// has none.
fn setup_mappings(source: &RemoteSource, direction: SshDirection) -> Vec<String> {
    let RemoteSource::Setup(scope) = source else {
        return Vec::new();
    };
    let Some(primary) = setup_primary_ref(scope) else {
        return Vec::new();
    };
    let branch = scope.branch_ref();
    vec![match direction {
        SshDirection::Fetch => format!("+{primary}:{}", scope.base_ref()),
        SshDirection::Push => format!("{branch}:{branch}"),
        SshDirection::Delete => format!(":{branch}"),
    }]
}

/// Whether `mappings` are exactly the setup channel's one mapping in
/// `direction`: no other ref, no force, no pattern and no second mapping.
fn setup_mappings_match_scope(
    source: &RemoteSource,
    direction: SshDirection,
    mappings: &[&str],
) -> bool {
    let expected = setup_mappings(source, direction);
    !expected.is_empty()
        && mappings.len() == expected.len()
        && mappings
            .iter()
            .zip(&expected)
            .all(|(mapping, expected)| mapping == expected)
}

/// Whether a call's source may carry an item's refs: a configured one only,
/// so a setup call maps no item ref.
fn item_scope_allowed(source: &RemoteSource) -> bool {
    matches!(source, RemoteSource::Configured)
}

fn plan_matches_configuration(plan: &RemoteRefPlan, context: &SshTransportError) -> bool {
    plan.remote_name() == context.remote_name
        && matches!(read_configuration(&context.root), Ok(ConfigurationInspection::Valid(config))
        if config.publication_remote.as_deref() == Some(plan.remote_name())
            && config.primary_branch == plan.primary_branch())
}

fn mappings_match_scope(
    plan: &RemoteRefPlan,
    target: &SynchronizationTarget,
    direction: SshDirection,
    mappings: &[&str],
) -> bool {
    let expected = exact_mappings(plan, target, direction);
    !mappings.is_empty()
        && mappings.len() <= expected.len()
        && mappings
            .iter()
            .all(|mapping| expected.iter().any(|expected| expected == mapping))
        && mappings
            .iter()
            .enumerate()
            .all(|(index, mapping)| !mappings[..index].contains(mapping))
}

fn tracking_oid(
    repository: &git2::Repository,
    name: &str,
) -> Result<Option<git2::Oid>, git2::Error> {
    match repository.find_reference(name) {
        Ok(reference) => reference
            .target()
            .map(Some)
            .ok_or_else(|| git2::Error::from_str("tracking reference is not direct")),
        Err(error) if error.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

// Object/all-destination validation stays in the caller. In locked libgit2
// 1.9.7, force=false checks existence BEFORE the ref lock. A non-NULL zero OID
// instead requires absence in cmp_old_ref UNDER that lock. force=true skips
// only the preliminary existence check; the expected-value guard still applies.
fn write_tracking_tip(
    repository: &git2::Repository,
    destination: &str,
    oid: git2::Oid,
    old: Option<git2::Oid>,
) -> Result<(), git2::Error> {
    repository
        .reference_matching(
            destination,
            oid,
            true,
            old.unwrap_or_else(git2::Oid::zero),
            "manyhands exact fetch",
        )
        .map(|_| ())
}

fn poll_mappings(plan: &RemoteRefPlan, advertised: &[(String, git2::Oid)]) -> Vec<String> {
    use crate::repository::remote::RemoteRefClassification;
    advertised
        .iter()
        .filter_map(|(name, _)| match plan.classify_advertised_ref(name)? {
            RemoteRefClassification::Primary => Some(plan.primary_fetch_refspec()),
            RemoteRefClassification::RecognizedContext { kind, item_id } => {
                let target = plan.context(kind, &item_id);
                Some(format!(
                    "+{}:{}",
                    target.remote_ref(),
                    target.tracking_ref()
                ))
            }
            RemoteRefClassification::MalformedContext => None,
        })
        .collect()
}

fn poll_mappings_match_scope(
    plan: &RemoteRefPlan,
    advertised: &[(String, git2::Oid)],
    direction: SshDirection,
    mappings: &[&str],
) -> bool {
    let expected = poll_mappings(plan, advertised);
    direction == SshDirection::Fetch
        && mappings.len() == expected.len()
        && expected
            .iter()
            .all(|mapping| mappings.contains(&mapping.as_str()))
        && mappings
            .iter()
            .enumerate()
            .all(|(index, mapping)| !mappings[..index].contains(mapping))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    #[test]
    fn transport_delete_scope_is_exact_context_only() {
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        let target = SynchronizationTarget::Context {
            kind: crate::repository::AuthoringKind::Document,
            item_id: crate::canonical::ItemId::from_str("01ARZ3NDEKTSV4RRFFQ69G5FAX").unwrap(),
        };
        let mapping = ":refs/heads/manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAX";
        assert_eq!(
            exact_mappings(&plan, &target, SshDirection::Delete),
            [mapping]
        );
        assert!(mappings_match_scope(
            &plan,
            &target,
            SshDirection::Delete,
            &[mapping]
        ));
        for invalid in [
            "",
            ":refs/heads/main",
            ":refs/heads/unrelated",
            ":refs/heads/manyhands/document/bad",
            ":refs/heads/*",
            "+:refs/heads/manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAX",
            "refs/heads/main:refs/heads/main",
        ] {
            assert!(!mappings_match_scope(
                &plan,
                &target,
                SshDirection::Delete,
                &[invalid]
            ));
        }
        assert!(!mappings_match_scope(
            &plan,
            &SynchronizationTarget::Primary,
            SshDirection::Delete,
            &[":refs/heads/main"]
        ));
        assert!(!mappings_match_scope(
            &plan,
            &target,
            SshDirection::Delete,
            &[]
        ));
        assert!(!mappings_match_scope(
            &plan,
            &target,
            SshDirection::Delete,
            &[mapping, mapping]
        ));
        assert!(!mappings_match_scope(
            &plan,
            &target,
            SshDirection::Push,
            &[mapping]
        ));
        let ticket = SynchronizationTarget::Context {
            kind: crate::repository::AuthoringKind::Ticket,
            item_id: crate::canonical::ItemId::from_str("01ARZ3NDEKTSV4RRFFQ69G5FAX").unwrap(),
        };
        // Named edit 5 (owner, 2026-10-17): a ticket context delete is the
        // same exact `:<context remote ref>` shape as a document's.
        assert_eq!(
            exact_mappings(&plan, &ticket, SshDirection::Delete),
            [":refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAX"]
        );
        for invalid in [
            "",
            "+:refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAX",
        ] {
            assert!(!mappings_match_scope(
                &plan,
                &ticket,
                SshDirection::Delete,
                &[invalid]
            ));
        }
        assert!(mappings_match_scope(
            &plan,
            &ticket,
            SshDirection::Delete,
            &[":refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAX"]
        ));
        // The kind must match the target: neither kind's ref deletes the other.
        assert!(!mappings_match_scope(
            &plan,
            &ticket,
            SshDirection::Delete,
            &[mapping]
        ));
        assert!(!mappings_match_scope(
            &plan,
            &target,
            SshDirection::Delete,
            &[":refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAX"]
        ));
        // A ticket ref with a bad or another ticket ID is refused.
        for invalid in [
            ":refs/heads/manyhands/ticket/bad",
            ":refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAV",
            ":refs/heads/manyhands/ticket/*",
        ] {
            assert!(!mappings_match_scope(
                &plan,
                &ticket,
                SshDirection::Delete,
                &[invalid]
            ));
        }
    }
    /// The setup channel maps one ref in each direction and nothing else: the
    /// remote's primary into the setup's private base ref on a fetch, the
    /// setup's branch on a push, and the same branch's delete. Each mapping
    /// is accepted in its own direction only; a configured source maps no
    /// setup ref, and a setup source no item ref.
    #[test]
    fn setup_mappings_name_only_the_setup_ref() {
        const ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
        const OTHER: &str = "01BX5ZZKBKACTAV9WEVGEMMVRZ";
        let scope = |remote: &str, primary: &str| SetupScope {
            setup_id: ItemId::from_str(ID).unwrap(),
            remote_name: remote.into(),
            primary_branch: primary.into(),
            endpoint_digest: [7; 32],
        };
        let source = RemoteSource::Setup(scope("origin", "main"));
        let fetch = format!("+refs/heads/main:refs/manyhands/setups/{ID}/base");
        let push = format!("refs/heads/manyhands/setup/{ID}:refs/heads/manyhands/setup/{ID}");
        let delete = format!(":refs/heads/manyhands/setup/{ID}");
        let own = [
            (SshDirection::Fetch, &fetch),
            (SshDirection::Push, &push),
            (SshDirection::Delete, &delete),
        ];
        for (direction, mapping) in own {
            assert_eq!(
                setup_mappings(&source, direction),
                [mapping.as_str()],
                "{direction:?}"
            );
        }
        // Each mapping is accepted in its own direction and in no other.
        for (direction, _) in own {
            for (mapped, mapping) in own {
                assert_eq!(
                    setup_mappings_match_scope(&source, direction, &[mapping.as_str()]),
                    direction == mapped,
                    "{direction:?} {mapped:?}"
                );
            }
        }
        // Nothing else is accepted in any direction: another ref, a force, a
        // pattern, another setup's ref, the primary, an item's ref, an empty
        // or a doubled mapping, a second mapping beside the right one.
        let other_branch = format!("refs/heads/manyhands/setup/{OTHER}");
        let others = [
            String::new(),
            "+refs/heads/main:refs/remotes/origin/main".into(),
            "refs/heads/main:refs/heads/main".into(),
            "+refs/heads/main:refs/heads/main".into(),
            ":refs/heads/main".into(),
            format!("refs/heads/manyhands/setup/{ID}:refs/heads/main"),
            format!("refs/heads/main:refs/heads/manyhands/setup/{ID}"),
            format!("+{push}"),
            format!("+:refs/heads/manyhands/setup/{ID}"),
            format!("{other_branch}:{other_branch}"),
            format!(":{other_branch}"),
            format!("+refs/heads/main:refs/manyhands/setups/{OTHER}/base"),
            format!("+refs/heads/other:refs/manyhands/setups/{ID}/base"),
            format!("+refs/heads/main:refs/remotes/origin/manyhands/setup/{ID}"),
            "refs/heads/manyhands/setup/*:refs/heads/manyhands/setup/*".into(),
            ":refs/heads/manyhands/setup/*".into(),
            format!(":refs/heads/manyhands/ticket/{ID}"),
            format!(":refs/heads/manyhands/document/{ID}"),
            format!("refs/heads/manyhands/ticket/{ID}:refs/heads/manyhands/ticket/{ID}"),
        ];
        for direction in [
            SshDirection::Fetch,
            SshDirection::Push,
            SshDirection::Delete,
        ] {
            for mapping in &others {
                assert!(
                    !setup_mappings_match_scope(&source, direction, &[mapping.as_str()]),
                    "{direction:?} {mapping}"
                );
            }
            assert!(!setup_mappings_match_scope(&source, direction, &[]));
        }
        for (direction, mapping) in own {
            let mapping = mapping.as_str();
            assert!(!setup_mappings_match_scope(
                &source,
                direction,
                &[mapping, mapping]
            ));
            assert!(!setup_mappings_match_scope(
                &source,
                direction,
                &[mapping, "refs/heads/main:refs/heads/main"]
            ));
        }
        // A configured source maps no setup ref, in any direction.
        for (direction, mapping) in own {
            assert!(setup_mappings(&RemoteSource::Configured, direction).is_empty());
            assert!(!setup_mappings_match_scope(
                &RemoteSource::Configured,
                direction,
                &[mapping.as_str()]
            ));
        }
        // A setup source carries no item's refs, and a configured one does.
        assert!(item_scope_allowed(&RemoteSource::Configured));
        assert!(!item_scope_allowed(&source));
        // A scope whose names are no valid plan maps nothing.
        for invalid in [scope("bad name", "main"), scope("origin", "HEAD")] {
            let source = RemoteSource::Setup(invalid);
            for (direction, mapping) in own {
                assert!(setup_mappings(&source, direction).is_empty());
                assert!(!setup_mappings_match_scope(
                    &source,
                    direction,
                    &[mapping.as_str()]
                ));
            }
        }
        // Another primary moves the fetch mapping and nothing else.
        let moved = RemoteSource::Setup(scope("origin", "trunk"));
        assert_eq!(
            setup_mappings(&moved, SshDirection::Fetch),
            [format!("+refs/heads/trunk:refs/manyhands/setups/{ID}/base")]
        );
        assert_eq!(setup_mappings(&moved, SshDirection::Push), [push.as_str()]);
        assert_eq!(
            setup_mappings(&moved, SshDirection::Delete),
            [delete.as_str()]
        );
    }
    fn tracking_commit(repository: &git2::Repository, message: &str) -> git2::Oid {
        let tree = repository.treebuilder(None).unwrap().write().unwrap();
        let tree = repository.find_tree(tree).unwrap();
        let signature =
            git2::Signature::new("Fixture", "fixture@example.invalid", &git2::Time::new(1, 0))
                .unwrap();
        repository
            .commit(None, &signature, &signature, message, &tree, &[])
            .unwrap()
    }
    fn external_git(root: &std::path::Path, args: &[&str]) {
        assert!(
            std::process::Command::new("git")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", root.join("absent-config"))
                .args([
                    "-c",
                    "user.name=Fixture",
                    "-c",
                    "user.email=fixture@example.invalid",
                    "-C"
                ])
                .arg(root)
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    }

    #[test]
    fn transport_absent_tracking_guard_locks_before_comparing_concurrent_creation() {
        let directory = tempfile::tempdir().unwrap();
        let repository = git2::Repository::init(directory.path()).unwrap();
        repository
            .config()
            .unwrap()
            .set_bool("core.logallrefupdates", true)
            .unwrap();
        let incoming = tracking_commit(&repository, "incoming");
        let competing = tracking_commit(&repository, "competing");
        let name = "refs/remotes/origin/concurrent";
        let captured = tracking_oid(&repository, name).unwrap();
        assert_eq!(captured, None);
        // External Git commits the competing creation; an independent git2
        // handle then holds the public ref lock. This distinguishes lock+CAS
        // from a pre-lock check without timing the backend's instructions.
        external_git(
            directory.path(),
            &["update-ref", name, &competing.to_string()],
        );
        let external = git2::Repository::open(directory.path()).unwrap();
        let mut lock = external.transaction().unwrap();
        lock.lock_ref(name).unwrap();
        let bytes = std::fs::read(repository.path().join(name)).unwrap();
        let log = std::fs::read(repository.path().join("logs").join(name)).unwrap();
        assert_eq!(
            write_tracking_tip(&repository, name, incoming, captured)
                .unwrap_err()
                .code(),
            git2::ErrorCode::Locked
        );
        assert_eq!(std::fs::read(repository.path().join(name)).unwrap(), bytes);
        assert_eq!(
            std::fs::read(repository.path().join("logs").join(name)).unwrap(),
            log
        );
        drop(lock);
        // After release it must reach the under-lock old-value comparison,
        // reject the captured absence as Modified, and retain the competitor.
        assert_eq!(
            write_tracking_tip(&repository, name, incoming, captured)
                .unwrap_err()
                .code(),
            git2::ErrorCode::Modified
        );
        assert_eq!(repository.refname_to_id(name).unwrap(), competing);
        assert_eq!(std::fs::read(repository.path().join(name)).unwrap(), bytes);
        assert_eq!(
            std::fs::read(repository.path().join("logs").join(name)).unwrap(),
            log
        );
        assert!(!repository.path().join(format!("{name}.lock")).exists());
        assert!(!repository.path().join("FETCH_HEAD").exists());
        // True absence remains a successful guarded create.
        write_tracking_tip(&repository, "refs/remotes/origin/new", incoming, None).unwrap();
        assert_eq!(
            repository.refname_to_id("refs/remotes/origin/new").unwrap(),
            incoming
        );
    }

    #[test]
    fn transport_absent_tracking_guard_rejects_direct_packed_and_symbolic_competitors() {
        for mode in 0..4 {
            let directory = tempfile::tempdir().unwrap();
            let repository = git2::Repository::init(directory.path()).unwrap();
            let incoming = tracking_commit(&repository, "incoming");
            let competing = tracking_commit(&repository, "competing");
            let name = "refs/remotes/origin/concurrent";
            let captured = tracking_oid(&repository, name).unwrap();
            assert_eq!(captured, None);
            match mode {
                0..=2 => {
                    let target = if mode == 1 { incoming } else { competing };
                    repository
                        .reference(name, target, false, "external creation")
                        .unwrap();
                    if mode == 2 {
                        external_git(directory.path(), &["pack-refs", "--all"]);
                        assert!(!repository.path().join(name).exists());
                    }
                }
                _ => {
                    repository
                        .reference("refs/remotes/origin/other", competing, false, "fixture")
                        .unwrap();
                    repository
                        .reference_symbolic(
                            name,
                            "refs/remotes/origin/other",
                            false,
                            "external creation",
                        )
                        .unwrap();
                }
            }
            let reference = repository.find_reference(name).unwrap();
            let target = reference.target();
            let symbolic = reference.symbolic_target().map(str::to_owned);
            let reflog_len = repository.reflog(name).map(|log| log.len()).ok();
            assert_eq!(
                write_tracking_tip(&repository, name, incoming, captured)
                    .unwrap_err()
                    .code(),
                git2::ErrorCode::Modified
            );
            let reference = repository.find_reference(name).unwrap();
            assert_eq!(reference.target(), target);
            assert_eq!(reference.symbolic_target(), symbolic.as_deref());
            assert_eq!(
                repository.reflog(name).map(|log| log.len()).ok(),
                reflog_len
            );
            assert!(!repository.path().join(format!("{name}.lock")).exists());
        }
    }
    #[test]
    fn transport_poll_scope_rejects_unadvertised_or_incomplete_mappings() {
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        let oid = git2::Oid::from_bytes(&[1; 20]).unwrap();
        let name = "refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAV";
        let advertised = vec![("refs/heads/main".into(), oid), (name.into(), oid)];
        let mappings = poll_mappings(&plan, &advertised);
        let refs: Vec<_> = mappings.iter().map(String::as_str).collect();
        assert!(poll_mappings_match_scope(
            &plan,
            &advertised,
            SshDirection::Fetch,
            &refs
        ));
        assert!(!poll_mappings_match_scope(
            &plan,
            &advertised,
            SshDirection::Push,
            &refs
        ));
        for invalid in [
            vec![],
            vec![refs[0]],
            vec![refs[0], refs[0]],
            vec![refs[0], refs[1], refs[1]],
            vec![refs[0], "+refs/heads/*:refs/remotes/origin/*"],
            vec![
                refs[0],
                "+refs/heads/unrelated:refs/remotes/origin/unrelated",
            ],
            vec![
                refs[0],
                "+refs/heads/manyhands/ticket/bad:refs/remotes/origin/manyhands/ticket/bad",
            ],
            vec![refs[0], "+refs/heads/main:refs/heads/main"],
        ] {
            assert!(!poll_mappings_match_scope(
                &plan,
                &advertised,
                SshDirection::Fetch,
                &invalid
            ));
        }
    }
    #[test]
    fn transport_poll_mappings_are_exact_parsed_advertised_set() {
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        let oid = git2::Oid::from_bytes(&[1; 20]).unwrap();
        let ticket = "refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAV";
        let advertised: Vec<_> = [
            "HEAD",
            "refs/heads/main",
            ticket,
            "refs/heads/manyhands/ticket/bad",
            "refs/heads/unrelated",
            "refs/tags/tag",
        ]
        .into_iter()
        .map(|name| (name.to_owned(), oid))
        .collect();
        assert_eq!(
            poll_mappings(&plan, &advertised),
            vec![
                plan.primary_fetch_refspec(),
                format!(
                    "+{ticket}:refs/remotes/origin/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAV"
                )
            ]
        );
        assert!(poll_mappings(&plan, &advertised[3..]).is_empty());
    }
    #[test]
    fn exact_mapping_scope_rejects_unchecked_syntax_and_direction() {
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        let target = SynchronizationTarget::Context {
            kind: crate::repository::AuthoringKind::Ticket,
            item_id: crate::canonical::ItemId::from_str("01ARZ3NDEKTSV4RRFFQ69G5FAV").unwrap(),
        };
        let fetch = exact_mappings(&plan, &target, SshDirection::Fetch);
        let push = exact_mappings(&plan, &target, SshDirection::Push);
        assert!(mappings_match_scope(
            &plan,
            &target,
            SshDirection::Fetch,
            &[&fetch[0], &fetch[1]]
        ));
        assert!(mappings_match_scope(
            &plan,
            &target,
            SshDirection::Push,
            &[&push[0]]
        ));
        for invalid in [
            format!("+{}", push[0]),
            ":refs/heads/main".into(),
            "refs/heads/*:refs/heads/*".into(),
            plan.primary_push_refspec(),
            fetch[1].clone(),
            "refs/heads/main:refs/heads/other".into(),
        ] {
            assert!(!mappings_match_scope(
                &plan,
                &target,
                SshDirection::Push,
                &[&invalid]
            ));
        }
        for invalid in [
            "+refs/heads/main:refs/heads/main",
            "refs/heads/main:refs/remotes/origin/main",
            "+refs/heads/*:refs/remotes/origin/*",
            "+refs/heads/main:refs/remotes/other/main",
            "+refs/heads/other:refs/remotes/origin/other",
        ] {
            assert!(!mappings_match_scope(
                &plan,
                &target,
                SshDirection::Fetch,
                &[invalid]
            ));
        }
        assert!(!mappings_match_scope(
            &plan,
            &SynchronizationTarget::Primary,
            SshDirection::Fetch,
            &[&fetch[1]]
        ));
        assert!(!mappings_match_scope(
            &plan,
            &target,
            SshDirection::Fetch,
            &[&fetch[0], &fetch[0]]
        ));
        assert!(!mappings_match_scope(
            &plan,
            &target,
            SshDirection::Fetch,
            &[]
        ));
    }
}
