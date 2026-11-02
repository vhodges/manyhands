//! A remote borrow cannot outlive the authenticated connection or secret borrow.
use super::{callbacks::CallbackAttempt, *};
use crate::repository::{
    ConfigurationInspection, RepositoryService, read_configuration,
    remote::{RemoteRefPlan, SynchronizationTarget},
};

struct PollTransfer<'a> {
    plan: &'a RemoteRefPlan,
    progress: &'a mut dyn FnMut(git2::Progress<'_>) -> bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TransferAction {
    Observe,
    Download,
    FetchTracking,
    PushObjects,
    Push,
    Delete(git2::Oid),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // Consumed by the later promotion orchestrator.
pub(crate) enum DeleteObservation {
    Deleted,
    StillPresent(git2::Oid),
    Changed(git2::Oid),
}

pub(crate) struct AuthenticatedSshRemote<'repo, 'a> {
    remote: &'a mut git2::Remote<'repo>,
    service: &'a RepositoryService,
    prepared: &'a PreparedSshAttempt,
    context: &'a SshTransportError,
    passphrase: Option<&'a str>,
    observed: HostKeyIdentity,
    trust: super::trust::HostTrustSnapshot,
}
impl<'repo, 'a> AuthenticatedSshRemote<'repo, 'a> {
    pub(super) fn new(
        remote: &'a mut git2::Remote<'repo>,
        service: &'a RepositoryService,
        prepared: &'a PreparedSshAttempt,
        context: &'a SshTransportError,
        passphrase: Option<&'a str>,
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
            service,
            prepared,
            context,
            passphrase,
            observed,
            trust,
        }
    }
    /// Pin the actual authenticated handle to the action's original endpoint;
    /// subsequent typed reconnects continue using this handle and prepared policy.
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

    pub(crate) fn advertisement(&self) -> Result<Vec<(String, git2::Oid)>, SshTransportError> {
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
            return Ok(());
        }
        self.transfer(
            &present,
            TransferAction::FetchTracking,
            Some((plan, target)),
        )
        .map(|_| ())
    }

    /// Fetch only parsed advertised contexts and the advertised primary. The
    /// caller owns throttling and stop intent; this hook has no reservation
    /// knowledge and runs only during download, outside the repository lease.
    #[allow(dead_code)] // Consumed by the later one-shot poll orchestrator.
    pub(crate) fn fetch_poll_set(
        &mut self,
        plan: &RemoteRefPlan,
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

    /// One guarded command at most. A fresh authenticated advertisement is the
    /// only result evidence, including after a refused or ambiguously lost push.
    #[allow(dead_code)] // Consumed by the later promotion orchestrator.
    pub(crate) fn delete_exact(
        &mut self,
        plan: &RemoteRefPlan,
        target: &SynchronizationTarget,
        expected: git2::Oid,
    ) -> Result<DeleteObservation, SshTransportError> {
        self.validate_plan(plan, SshDirection::Delete)?;
        let mappings = exact_mappings(plan, target, SshDirection::Delete);
        let refs: Vec<_> = mappings.iter().map(String::as_str).collect();
        if expected.is_zero() || !mappings_match_scope(plan, target, SshDirection::Delete, &refs) {
            return Err(self
                .context
                .with_kind(SshTransportErrorKind::ConfigurationInvalid));
        }
        let reference = mappings[0].strip_prefix(':').unwrap();
        // A retry must observe first: never blindly resend an ambiguous delete.
        if !self
            .fresh_advertisement()?
            .iter()
            .any(|(name, _)| name == reference)
        {
            return Ok(DeleteObservation::Deleted);
        }
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
        let advertised = self.fresh_advertisement()?;
        Ok(
            match advertised.iter().find(|(name, _)| name == reference) {
                None => DeleteObservation::Deleted,
                Some((_, oid)) if *oid == expected => DeleteObservation::StillPresent(*oid),
                Some((_, oid)) => DeleteObservation::Changed(*oid),
            },
        )
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
        let mut scoped = AuthenticatedSshRemote {
            remote: &mut remote,
            service: self.service,
            prepared: self.prepared,
            context: self.context,
            passphrase: self.passphrase,
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

    fn validate_plan(
        &self,
        plan: &RemoteRefPlan,
        direction: SshDirection,
    ) -> Result<(), SshTransportError> {
        if !plan_matches_configuration(plan, self.context) || direction != self.context.direction {
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
            || (action != TransferAction::Observe
                && matches!(action, TransferAction::Push | TransferAction::Delete(_)) != push)
        {
            return Err(self
                .context
                .with_kind(SshTransportErrorKind::ConfigurationInvalid));
        }
        self.service
            .recheck_ssh(self.prepared, self.context)
            .map_err(|kind| self.context.with_kind(kind))?;
        self.recheck_trust()?;
        // Renew positive selected-key evidence before any transfer. A disconnected
        // remote never inherits a prior connection's authentication evidence.
        self.remote.disconnect().map_err(|_| {
            self.context
                .with_kind(SshTransportErrorKind::TransportUnavailable)
        })?;
        let attempt = CallbackAttempt::default();
        let mut connection = self
            .remote
            .connect_auth(
                if push {
                    git2::Direction::Push
                } else {
                    git2::Direction::Fetch
                },
                Some(super::callbacks::build_callbacks(
                    self.prepared,
                    self.passphrase,
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
        #[cfg(test)]
        super::operation_tests::checkpoint(super::operation_tests::Checkpoint::Reconnected);
        self.service
            .recheck_ssh(self.prepared, self.context)
            .map_err(|kind| self.context.with_kind(kind))?;
        let current_trust = self
            .service
            .read_host_trust(&self.prepared.endpoint.authority)
            .map_err(|kind| self.context.with_kind(kind))?;
        if current_trust != self.trust || observed != self.observed {
            return Err(self
                .context
                .with_kind(SshTransportErrorKind::HostTrustChanged));
        }
        if action == TransferAction::Observe {
            return connection
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
                });
        }
        if let Some((plan, _)) = scope
            && !plan_matches_configuration(plan, self.context)
        {
            return Err(self
                .context
                .with_kind(SshTransportErrorKind::ConfigurationInvalid));
        }
        // The poll's set and returned OIDs belong to this authenticated download
        // connection, never an earlier advertisement on a different connection.
        let poll_advertisement = if poll.is_some() {
            connection
                .list()
                .map_err(|_| {
                    self.context
                        .with_kind(SshTransportErrorKind::ProtocolFailure)
                })?
                .iter()
                .map(|head| (head.name().to_owned(), head.oid()))
                .collect::<Vec<_>>()
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
        let tracking_repository = if action == TransferAction::FetchTracking {
            Some(git2::Repository::open(&self.context.root).map_err(|_| {
                self.context
                    .with_kind(SshTransportErrorKind::ProtocolFailure)
            })?)
        } else {
            None
        };
        let object_advertisement = if action == TransferAction::PushObjects {
            let advertised = connection
                .list()
                .map_err(|_| {
                    self.context
                        .with_kind(SshTransportErrorKind::ProtocolFailure)
                })?
                .iter()
                .map(|head| (head.name().to_owned(), head.oid()))
                .collect::<Vec<_>>();
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
            let heads = connection.list().map_err(|_| {
                self.context
                    .with_kind(SshTransportErrorKind::ProtocolFailure)
            })?;
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
                    .find(|head| head.name() == source)
                    .map(|head| head.oid())
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
        let mut callbacks =
            super::callbacks::build_callbacks(self.prepared, self.passphrase, &attempt);
        if let TransferAction::Delete(expected) = action {
            super::callbacks::guard_delete(
                &mut callbacks,
                refspecs[0].strip_prefix(':').unwrap(),
                expected,
            );
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
        // carries policy; an unexpected further reconnect fails closed through
        // the one-submission bound instead of silently changing credentials.
        let result = if push {
            let mut options = git2::PushOptions::new();
            options.remote_callbacks(callbacks);
            connection.remote().push(refspecs, Some(&mut options))
        } else {
            let mut options = git2::FetchOptions::new();
            options.remote_callbacks(callbacks);
            options.update_fetchhead(false);
            if matches!(
                action,
                TransferAction::FetchTracking | TransferAction::PushObjects
            ) {
                options.download_tags(git2::AutotagOption::None);
                options.prune(git2::FetchPrune::Off);
            }
            connection.remote().download(refspecs, Some(&mut options))
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
                    backend_failure(&attempt, &error, self.passphrase.is_some())
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
            .map_err(|_| {
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
