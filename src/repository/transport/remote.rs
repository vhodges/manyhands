//! A remote borrow cannot outlive the authenticated connection or secret borrow.
use super::{callbacks::CallbackAttempt, *};
use crate::repository::RepositoryService;

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
    // Later lifecycle callers supply coordinated refspec policy; only fixtures
    // consume these crate-private transfer primitives in this Cycle.
    #[allow(dead_code)]
    pub(crate) fn download(&mut self, refspecs: &[&str]) -> Result<(), SshTransportError> {
        self.transfer(refspecs, false)
    }
    #[allow(dead_code)]
    pub(crate) fn push(&mut self, refspecs: &[&str]) -> Result<(), SshTransportError> {
        self.transfer(refspecs, true)
    }
    fn transfer(&mut self, refspecs: &[&str], push: bool) -> Result<(), SshTransportError> {
        if push != (self.context.direction == SshDirection::Push) {
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
                .with_kind(backend_failure(&attempt, &error, self.passphrase.is_some()))
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
