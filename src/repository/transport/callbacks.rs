//! Operation-local callback policy. No SQL, prompts, or inherited credentials.
use super::{
    HostKeyIdentity, PreparedSshAttempt, SshTransportErrorKind,
    agent::AgentSigner,
    agent_credential,
    endpoint::parse_ssh_endpoint,
    trust::{approval_matches, valid_identity},
};
use std::cell::{Cell, RefCell};

#[derive(Default)]
pub(super) struct CallbackAttempt {
    failure: RefCell<Option<SshTransportErrorKind>>,
    observed_host: RefCell<Option<HostKeyIdentity>>,
    passthrough: Cell<bool>,
    key_submissions: Cell<u8>,
    username_submissions: Cell<u8>,
    poll_stopped: Cell<bool>,
    /// The host a call already finalized, for a further connection it makes.
    finalized_host: Option<HostKeyIdentity>,
}

impl CallbackAttempt {
    /// The attempt of a further connection of a call that finalized `host`:
    /// a host that is not `host` is refused at the host check, before a key
    /// is offered to it, whether the prepared policy accepts it or would
    /// pass it to libgit2's known_hosts check.
    pub(super) fn for_finalized_host(host: HostKeyIdentity) -> Self {
        Self {
            finalized_host: Some(host),
            ..Self::default()
        }
    }
    fn accept(
        &self,
        presented: &HostKeyIdentity,
    ) -> Result<git2::CertificateCheckStatus, git2::Error> {
        if self.not_finalized(presented) {
            return self.reject(SshTransportErrorKind::HostTrustChanged);
        }
        Ok(git2::CertificateCheckStatus::CertificateOk)
    }
    /// Whether this is a further connection of a call that finalized a host
    /// other than `presented`.
    fn not_finalized(&self, presented: &HostKeyIdentity) -> bool {
        self.finalized_host
            .as_ref()
            .is_some_and(|finalized| finalized != presented)
    }
    pub(super) fn poll_progress(&self, keep_going: impl FnOnce() -> bool) -> bool {
        if self.poll_stopped.get() {
            return false;
        }
        let continuing = keep_going();
        self.poll_stopped.set(!continuing);
        continuing
    }
    pub(super) fn poll_stopped(&self) -> bool {
        self.poll_stopped.get()
    }
    pub(super) fn failure(&self) -> Option<SshTransportErrorKind> {
        self.failure.borrow().clone()
    }
    pub(super) fn observed_host(&self) -> Option<HostKeyIdentity> {
        self.observed_host.borrow().clone()
    }
    pub(super) fn passthrough(&self) -> bool {
        self.passthrough.get()
    }
    pub(super) fn key_submissions(&self) -> u8 {
        self.key_submissions.get()
    }
    fn reject<T>(&self, kind: SshTransportErrorKind) -> Result<T, git2::Error> {
        self.failure.borrow_mut().get_or_insert(kind);
        Err(git2::Error::from_str("SSH policy rejected"))
    }
    pub(super) fn credentials(
        &self,
        prepared: &PreparedSshAttempt,
        passphrase: Option<&str>,
        agent: Option<&AgentSigner>,
        url: &str,
        username: Option<&str>,
        allowed: git2::CredentialType,
    ) -> Result<git2::Cred, git2::Error> {
        if self.failure().is_some() {
            return Err(git2::Error::from_str("SSH policy rejected"));
        }
        if parse_ssh_endpoint(url).as_ref() != Ok(&prepared.endpoint) {
            return self.reject(SshTransportErrorKind::EndpointChanged);
        }
        let Some(expected_username) = prepared.endpoint.username.as_deref() else {
            return self.reject(SshTransportErrorKind::UsernameRequired);
        };
        if username.is_some_and(|name| name != expected_username) {
            return self.reject(SshTransportErrorKind::EndpointChanged);
        }
        // A connection submits the selected key once. A request after that,
        // for the key or any other method, is libgit2 reconnecting on the
        // call's connection: it fails closed, as a rejected key.
        if self.key_submissions.get() != 0 {
            return self.reject(SshTransportErrorKind::KeyRejected);
        }
        let result = if allowed.contains(git2::CredentialType::SSH_KEY) {
            self.key_submissions.set(1);
            match agent {
                // Offers exactly the selected key; the agent's others are never
                // listed to the server or asked to sign.
                Some(signer) => agent_credential::credential(expected_username, signer),
                None => git2::Cred::ssh_key(
                    expected_username,
                    None,
                    &prepared.registration.private_key_path,
                    passphrase,
                ),
            }
        } else if allowed == git2::CredentialType::USERNAME && self.username_submissions.get() == 0
        {
            self.username_submissions.set(1);
            git2::Cred::username(expected_username)
        } else {
            return self.reject(SshTransportErrorKind::KeyRejected);
        };
        result.or_else(|_| self.reject(SshTransportErrorKind::KeyInvalidOrUnsupported))
    }
    pub(super) fn check_host(
        &self,
        prepared: &PreparedSshAttempt,
        host: &str,
        presented: Option<HostKeyIdentity>,
    ) -> Result<git2::CertificateCheckStatus, git2::Error> {
        if self.failure().is_some() {
            return Err(git2::Error::from_str("SSH policy rejected"));
        }
        let host = if host.contains(':') && !host.starts_with('[') {
            format!("[{host}]")
        } else {
            host.to_owned()
        };
        let endpoint = parse_ssh_endpoint(&format!(
            "ssh://fixture@{host}:{}/fixture",
            prepared.endpoint.authority.port
        ));
        if !endpoint.is_ok_and(|endpoint| endpoint.authority == prepared.endpoint.authority) {
            return self.reject(SshTransportErrorKind::EndpointChanged);
        }
        let Some(presented) = presented.filter(valid_identity) else {
            return self.reject(SshTransportErrorKind::HostVerificationUnavailable);
        };
        if self
            .observed_host
            .borrow()
            .as_ref()
            .is_some_and(|prior| prior != &presented)
        {
            return self.reject(SshTransportErrorKind::HostTrustChanged);
        }
        *self.observed_host.borrow_mut() = Some(presented.clone());
        let approved = approval_matches(
            prepared.approval.as_ref(),
            &prepared.endpoint.authority,
            prepared.trust.pin.as_ref(),
            &presented,
        );
        if let Some(pin) = prepared.trust.pin.as_ref() {
            if pin == &presented || approved {
                return self.accept(&presented);
            }
            return self.reject(SshTransportErrorKind::HostReplacementRequired {
                expected: pin.clone(),
                presented,
            });
        }
        if approved {
            return self.accept(&presented);
        }
        if prepared.trust.reapproval_required || prepared.approval.is_some() {
            return self.reject(SshTransportErrorKind::HostApprovalRequired { presented });
        }
        // With no pin and no approval, libgit2's known_hosts check decides,
        // and the call compares the host with the one it finalized only once
        // the connection is made and the key submitted. A further connection
        // refuses any other host here, before a key is offered, as the pinned
        // and approved branches do (the final review's F-4).
        if self.not_finalized(&presented) {
            return self.reject(SshTransportErrorKind::HostTrustChanged);
        }
        self.passthrough.set(true);
        Ok(git2::CertificateCheckStatus::CertificatePassthrough)
    }
}

fn certificate_identity(certificate: &git2::cert::Cert<'_>) -> Option<HostKeyIdentity> {
    let hostkey = certificate.as_hostkey()?;
    // Decode the SSH wire public key, deriving both algorithm and SHA-256
    // from the same bytes. Never substitute the MD5 or SHA-1 fields.
    let key = ssh_key::PublicKey::from_bytes(hostkey.hostkey()?).ok()?;
    Some(HostKeyIdentity {
        algorithm: key.algorithm().to_string(),
        sha256: key.fingerprint(ssh_key::HashAlg::Sha256).to_string(),
    })
}

pub(super) fn build_callbacks<'a>(
    prepared: &'a PreparedSshAttempt,
    passphrase: Option<&'a str>,
    agent: Option<&'a AgentSigner>,
    attempt: &'a CallbackAttempt,
) -> git2::RemoteCallbacks<'a> {
    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.credentials(move |url, username, allowed| {
        attempt.credentials(prepared, passphrase, agent, url, username, allowed)
    });
    callbacks.certificate_check(move |certificate, host| {
        attempt.check_host(prepared, host, certificate_identity(certificate))
    });
    callbacks
}

/// Locked libgit2 uses src=current remote OID and dst=new OID, even though
/// a deletion has an empty source refname. Refuse before sending any command.
pub(super) fn guard_delete<'a>(
    callbacks: &mut git2::RemoteCallbacks<'a>,
    reference: &'a str,
    expected: git2::Oid,
) {
    callbacks.push_negotiation(move |updates| {
        if updates.len() != 1
            || expected.is_zero()
            || updates[0].src() != expected
            || !updates[0].dst().is_zero()
            || updates[0].dst_refname() != Some(reference)
        {
            return Err(git2::Error::from_str("SSH deletion precondition rejected"));
        }
        Ok(())
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One connection submits the selected key once. A later credential
    /// request on it is libgit2 reconnecting: whatever it asks for, the
    /// attempt refuses it as a rejected key, submits nothing more and falls
    /// back to no other key or method, and the call answers as a failed
    /// authentication of that kind does today.
    #[test]
    fn a_second_key_submission_on_one_connection_fails_closed() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let prepared = super::super::tests::prepared(file.path());
        let url = prepared.endpoint.connection_url.clone();
        for second in [
            git2::CredentialType::SSH_KEY,
            git2::CredentialType::SSH_KEY | git2::CredentialType::USER_PASS_PLAINTEXT,
            git2::CredentialType::USERNAME,
            git2::CredentialType::USER_PASS_PLAINTEXT,
            git2::CredentialType::SSH_INTERACTIVE,
            git2::CredentialType::SSH_CUSTOM,
            git2::CredentialType::DEFAULT,
        ] {
            let attempt = CallbackAttempt::default();
            attempt
                .credentials(
                    &prepared,
                    None,
                    None,
                    &url,
                    Some("fixture"),
                    git2::CredentialType::SSH_KEY,
                )
                .unwrap();
            assert_eq!(attempt.key_submissions(), 1);
            let refused = attempt.credentials(&prepared, None, None, &url, Some("fixture"), second);
            assert_eq!(
                refused.err().map(|error| error.message().to_owned()),
                Some("SSH policy rejected".to_owned()),
                "{second:?}"
            );
            assert_eq!(attempt.key_submissions(), 1, "{second:?}");
            assert_eq!(
                attempt.failure(),
                Some(SshTransportErrorKind::KeyRejected),
                "{second:?}"
            );
            let error = git2::Error::from_str("SSH policy rejected");
            assert_eq!(
                super::super::remote::backend_failure(&attempt, &error, false),
                SshTransportErrorKind::KeyRejected
            );
            assert_eq!(
                super::super::remote::backend_failure(&attempt, &error, true),
                SshTransportErrorKind::UnlockFailed
            );
            // Nothing more is offered on that connection.
            for allowed in [
                git2::CredentialType::SSH_KEY,
                git2::CredentialType::USERNAME,
            ] {
                assert!(
                    attempt
                        .credentials(&prepared, None, None, &url, Some("fixture"), allowed)
                        .is_err()
                );
            }
            assert!(
                attempt
                    .check_host(&prepared, "example.invalid", None)
                    .is_err()
            );
            assert_eq!(attempt.key_submissions(), 1);
        }
    }
}
