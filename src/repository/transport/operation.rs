//! Prepare, authenticate, recheck, use, and drop one selected-key operation.
use super::{
    callbacks::{CallbackAttempt, build_callbacks},
    endpoint::configured_remote_endpoint,
    remote::{AuthenticatedSshRemote, backend_failure},
    *,
};
use crate::repository::{
    ConfigurationInspection, RepositoryOperation, RepositoryService, canonical_repository_root,
    keys::*, read_configuration,
};

impl RepositoryService {
    pub fn verify_ssh_transport<P: SessionCredentialProvider>(
        &self,
        request: VerifySshTransportRequest,
        session: &mut SessionCredentials<P>,
    ) -> Result<SshTransportVerified, SshTransportError> {
        self.with_authenticated_remote(request, session, |remote| {
            remote.advertisement()?;
            Ok(remote.verified())
        })
    }

    pub(crate) fn with_authenticated_remote<P: SessionCredentialProvider, T>(
        &self,
        request: VerifySshTransportRequest,
        session: &mut SessionCredentials<P>,
        use_remote: impl FnOnce(&mut AuthenticatedSshRemote<'_, '_>) -> Result<T, SshTransportError>,
    ) -> Result<T, SshTransportError> {
        self.with_authenticated_remote_policy(request, session, false, use_remote)
    }

    /// Synchronization supplies its frozen action endpoint. Reject a newly
    /// resolved destination before authentication/prompts, not only before effects.
    pub(in super::super) fn with_authenticated_remote_expected<P: SessionCredentialProvider, T>(
        &self,
        request: VerifySshTransportRequest,
        session: &mut SessionCredentials<P>,
        expectation: &SshScopeExpectation,
        use_remote: impl FnOnce(&mut AuthenticatedSshRemote<'_, '_>) -> Result<T, SshTransportError>,
    ) -> Result<T, SshTransportError> {
        self.with_authenticated_remote_policy_expected(
            request,
            session,
            false,
            Some(expectation),
            use_remote,
        )
    }

    pub(crate) fn with_authenticated_remote_policy<P: SessionCredentialProvider, T>(
        &self,
        request: VerifySshTransportRequest,
        session: &mut SessionCredentials<P>,
        automatic: bool,
        use_remote: impl FnOnce(&mut AuthenticatedSshRemote<'_, '_>) -> Result<T, SshTransportError>,
    ) -> Result<T, SshTransportError> {
        self.with_authenticated_remote_policy_expected(
            request, session, automatic, None, use_remote,
        )
    }

    fn with_authenticated_remote_policy_expected<P: SessionCredentialProvider, T>(
        &self,
        request: VerifySshTransportRequest,
        session: &mut SessionCredentials<P>,
        automatic: bool,
        expectation: Option<&SshScopeExpectation>,
        use_remote: impl FnOnce(&mut AuthenticatedSshRemote<'_, '_>) -> Result<T, SshTransportError>,
    ) -> Result<T, SshTransportError> {
        let mut context = SshTransportError {
            root: request.root.clone(),
            remote_name: String::new(),
            direction: request.direction,
            selected_key_id: None,
            authority: None,
            kind: SshTransportErrorKind::ConfigurationInvalid,
        };
        if !crate::runtime::git_transport_initialized() {
            return Err(context.with_kind(SshTransportErrorKind::RuntimeUninitialized));
        }
        let prepared = match self.prepare_ssh(request, &mut context) {
            Ok(prepared) => prepared,
            Err(kind) => {
                invalidate_preflight_session(session, &kind);
                return Err(context.with_kind(kind));
            }
        };
        if let Some(expectation) = expectation
            && let Err(kind) = expectation.check(&prepared)
        {
            invalidate_preflight_session(session, &kind);
            return Err(context.with_kind(kind));
        }
        let unlock = UnlockRequest {
            key_id: prepared.registration.id,
            label: prepared.registration.label.clone(),
            source: prepared.source.clone(),
            reason: UnlockReason::AuthenticationAmbiguous,
        };
        #[cfg(test)]
        super::operation_tests::checkpoint(super::operation_tests::Checkpoint::Prepared);
        let cached = session.has_cached_passphrase(&unlock);
        session.reconcile_source(&unlock);
        let mut use_remote = Some(use_remote);
        let mut may_prompt = false;
        if !cached {
            match self.connect_ssh(&prepared, &context, None, &mut use_remote, &mut may_prompt) {
                Ok(result) => return result,
                Err(_) if may_prompt => {}
                Err(error) => return Err(error),
            }
        }
        // The first connection and all callback/Git/registry handles have dropped.
        // An Err here prevents newly supplied material from entering the cache.
        if automatic && let Some(failure) = session.blocked_unlock(&unlock) {
            return Err(context.with_kind(match failure {
                SessionUnlockFailure::ProviderUnavailable => {
                    SshTransportErrorKind::ProviderUnavailable
                }
                _ => SshTransportErrorKind::UnlockCancelled,
            }));
        }
        let mut connection_error = None;
        let result = session.with_passphrase(unlock, |passphrase| {
            #[cfg(test)]
            if !cached {
                super::operation_tests::checkpoint(
                    super::operation_tests::Checkpoint::ProviderReturned,
                );
            }
            self.connect_ssh(
                &prepared,
                &context,
                Some(passphrase),
                &mut use_remote,
                &mut may_prompt,
            )
            .map_err(|error| {
                connection_error = Some(error);
                PassphraseUseFailure::Unavailable
            })
        });
        // Cancellation/unavailability can also race state changes in the provider.
        if let Err(kind) = self.recheck_ssh(&prepared, &context) {
            invalidate_preflight_session(session, &kind);
            return Err(context.with_kind(kind));
        }
        if let Some(error) = connection_error {
            return Err(error);
        }
        match result {
            Ok(result) => {
                if result
                    .as_ref()
                    .is_err_and(|error| invalidates_secret(&error.kind))
                {
                    session.invalidate(prepared.registration.id);
                }
                result
            }
            Err(failure) => Err(context.with_kind(match failure {
                SessionUnlockFailure::Cancelled => SshTransportErrorKind::UnlockCancelled,
                SessionUnlockFailure::ProviderUnavailable => {
                    SshTransportErrorKind::ProviderUnavailable
                }
                _ => SshTransportErrorKind::UnlockFailed,
            })),
        }
    }

    fn prepare_ssh(
        &self,
        request: VerifySshTransportRequest,
        context: &mut SshTransportError,
    ) -> Result<PreparedSshAttempt, SshTransportErrorKind> {
        let (repository, root) =
            canonical_repository_root(&request.root, RepositoryOperation::Inspect)
                .map_err(|_| SshTransportErrorKind::ConfigurationInvalid)?;
        context.root = root;
        context.remote_name = publication_remote(&context.root)?;
        let registration = self
            .list_shared_keys()
            .map_err(|_| SshTransportErrorKind::RegistryUnavailable)?
            .into_iter()
            .find(|registration| registration.selected)
            .ok_or(SshTransportErrorKind::NoSelectedKey)?;
        context.selected_key_id = Some(registration.id);
        repository
            .find_remote(&context.remote_name)
            .map_err(|_| SshTransportErrorKind::PublicationRemoteMissing)?;
        let endpoint =
            configured_remote_endpoint(&repository, &context.remote_name, context.direction)?;
        context.authority = Some(endpoint.authority.clone());
        if endpoint.username.is_none() {
            return Err(SshTransportErrorKind::UsernameRequired);
        }
        let source =
            KeySourceToken::observe(&registration.private_key_path).map_err(source_failure)?;
        let trust = self.read_host_trust(&endpoint.authority)?;
        Ok(PreparedSshAttempt {
            registration,
            source,
            endpoint,
            trust,
            approval: request.approval,
        })
    }

    pub(super) fn recheck_ssh(
        &self,
        prepared: &PreparedSshAttempt,
        context: &SshTransportError,
    ) -> Result<(), SshTransportErrorKind> {
        let selected = self
            .list_shared_keys()
            .map_err(|_| SshTransportErrorKind::RegistryUnavailable)?
            .into_iter()
            .find(|registration| registration.selected);
        if selected.as_ref().map(|registration| registration.id) != Some(prepared.registration.id) {
            return Err(SshTransportErrorKind::SelectionChanged);
        }
        if selected
            .as_ref()
            .map(|registration| &registration.private_key_path)
            != Some(&prepared.registration.private_key_path)
            || KeySourceToken::observe(&prepared.registration.private_key_path).as_ref()
                != Ok(&prepared.source)
        {
            return Err(SshTransportErrorKind::KeySourceChanged);
        }
        if publication_remote(&context.root).as_ref() != Ok(&context.remote_name) {
            return Err(SshTransportErrorKind::EndpointChanged);
        }
        let repository = git2::Repository::open(&context.root)
            .map_err(|_| SshTransportErrorKind::EndpointChanged)?;
        if configured_remote_endpoint(&repository, &context.remote_name, context.direction).as_ref()
            != Ok(&prepared.endpoint)
        {
            return Err(SshTransportErrorKind::EndpointChanged);
        }
        Ok(())
    }

    fn connect_ssh<T>(
        &self,
        prepared: &PreparedSshAttempt,
        context: &SshTransportError,
        passphrase: Option<&str>,
        use_remote: &mut Option<
            impl FnOnce(&mut AuthenticatedSshRemote<'_, '_>) -> Result<T, SshTransportError>,
        >,
        may_prompt: &mut bool,
    ) -> Result<Result<T, SshTransportError>, SshTransportError> {
        self.recheck_ssh(prepared, context)
            .map_err(|kind| context.with_kind(kind))?;
        let repository = git2::Repository::open(&context.root)
            .map_err(|_| context.with_kind(SshTransportErrorKind::ConfigurationInvalid))?;
        let mut remote = repository
            .remote_anonymous(&prepared.endpoint.connection_url)
            .map_err(|_| context.with_kind(SshTransportErrorKind::ConfigurationInvalid))?;
        // Normalization can itself make a URL match a rewrite that did not
        // match the raw configured spelling. Validate this actual handle too.
        let effective = match context.direction {
            SshDirection::Fetch => remote.url(),
            SshDirection::Push => remote.pushurl().or_else(|| remote.url()),
        };
        if effective
            .and_then(|url| super::endpoint::parse_ssh_endpoint(url).ok())
            .as_ref()
            != Some(&prepared.endpoint)
        {
            return Err(context.with_kind(SshTransportErrorKind::ConfigurationInvalid));
        }
        let attempt = CallbackAttempt::default();
        let mut connection = remote
            .connect_auth(
                match context.direction {
                    SshDirection::Fetch => git2::Direction::Fetch,
                    SshDirection::Push => git2::Direction::Push,
                },
                Some(build_callbacks(prepared, passphrase, &attempt)),
                None,
            )
            .map_err(|error| {
                let kind = backend_failure(&attempt, &error, passphrase.is_some());
                *may_prompt = passphrase.is_none()
                    && attempt.key_submissions() == 1
                    && attempt.observed_host().is_some()
                    && (authentication_failure(&kind)
                        || (attempt.failure().is_none()
                            && error.code() == git2::ErrorCode::GenericError
                            && error.class() == git2::ErrorClass::Ssh));
                context.with_kind(kind)
            })?;
        let observed = attempt
            .observed_host()
            .ok_or_else(|| context.with_kind(SshTransportErrorKind::HostVerificationUnavailable))?;
        if attempt.key_submissions() != 1 {
            return Err(context.with_kind(SshTransportErrorKind::KeyRejected));
        }
        #[cfg(test)]
        super::operation_tests::checkpoint(super::operation_tests::Checkpoint::Authenticated);
        self.recheck_ssh(prepared, context)
            .map_err(|kind| context.with_kind(kind))?;
        self.finalize_host_trust(
            &prepared.endpoint.authority,
            &prepared.trust,
            &observed,
            prepared.approval.as_ref(),
        )
        .map_err(|kind| context.with_kind(kind))?;
        let mut adapter = AuthenticatedSshRemote::new(
            connection.remote(),
            self,
            prepared,
            context,
            passphrase,
            observed,
        );
        Ok(use_remote.take().expect("one authenticated operation")(
            &mut adapter,
        ))
    }
}

fn publication_remote(root: &std::path::Path) -> Result<String, SshTransportErrorKind> {
    match read_configuration(root).map_err(|_| SshTransportErrorKind::ConfigurationInvalid)? {
        ConfigurationInspection::Valid(config) => config
            .publication_remote
            .ok_or(SshTransportErrorKind::PublicationRemoteMissing),
        _ => Err(SshTransportErrorKind::ConfigurationInvalid),
    }
}
fn source_failure(error: KeyMaterialError) -> SshTransportErrorKind {
    match error.kind {
        KeyMaterialErrorKind::SourceMissing => SshTransportErrorKind::KeyMissing,
        _ => SshTransportErrorKind::KeyUnreadable,
    }
}

fn invalidate_preflight_session<P: SessionCredentialProvider>(
    session: &mut SessionCredentials<P>,
    kind: &SshTransportErrorKind,
) {
    if matches!(
        kind,
        SshTransportErrorKind::NoSelectedKey
            | SshTransportErrorKind::SelectionChanged
            | SshTransportErrorKind::KeyMissing
            | SshTransportErrorKind::KeyUnreadable
            | SshTransportErrorKind::KeySourceChanged
    ) {
        session.clear();
    } else {
        session.clear_cached_passphrase();
    }
}
pub(super) fn authentication_failure(kind: &SshTransportErrorKind) -> bool {
    matches!(
        kind,
        SshTransportErrorKind::KeyRejected
            | SshTransportErrorKind::KeyInvalidOrUnsupported
            | SshTransportErrorKind::UnlockFailed
    )
}
fn invalidates_secret(kind: &SshTransportErrorKind) -> bool {
    authentication_failure(kind)
        || matches!(
            kind,
            SshTransportErrorKind::SelectionChanged
                | SshTransportErrorKind::KeySourceChanged
                | SshTransportErrorKind::EndpointChanged
                | SshTransportErrorKind::HostTrustChanged
        )
}
impl SshTransportError {
    pub(super) fn with_kind(&self, kind: SshTransportErrorKind) -> Self {
        Self {
            kind,
            ..self.clone()
        }
    }
}
