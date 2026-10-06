//! A remote borrow cannot outlive the authenticated connection or secret borrow.
use super::{callbacks::CallbackAttempt, *};
use crate::repository::{
    ConfigurationInspection, RepositoryService, read_configuration,
    remote::{RemoteRefPlan, SynchronizationTarget},
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum TransferAction {
    Observe,
    Download,
    FetchTracking,
    PushObjects,
    Push,
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
        let push =
            self.context.direction == SshDirection::Push && action != TransferAction::PushObjects;
        if action != TransferAction::Observe && (action == TransferAction::Push) != push {
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
        callbacks.push_update_reference(|_, status| {
            if status.is_some() {
                rejected.set(true);
            }
            Ok(())
        });
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
            let (plan, target) = scope.expect("tracking transfer has typed scope");
            if !plan_matches_configuration(plan, self.context)
                || !mappings_match_scope(plan, target, self.context.direction, refspecs)
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
                match old {
                    Some(old) => repository.reference_matching(
                        &destination,
                        oid,
                        true,
                        old,
                        "manyhands exact fetch",
                    ),
                    None => repository.reference(&destination, oid, false, "manyhands exact fetch"),
                }
                .map_err(|_| {
                    self.context
                        .with_kind(SshTransportErrorKind::ProtocolFailure)
                })?;
                #[cfg(test)]
                super::operation_tests::checkpoint(
                    super::operation_tests::Checkpoint::TrackingWritten,
                );
            }
        }
        Ok(object_advertisement)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
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
