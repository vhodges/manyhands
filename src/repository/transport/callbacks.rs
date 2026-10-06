//! Operation-local callback policy. No SQL, prompts, or inherited credentials.
use super::{
    HostKeyIdentity, PreparedSshAttempt, SshTransportErrorKind,
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
}

impl CallbackAttempt {
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
        let result = if allowed.contains(git2::CredentialType::SSH_KEY)
            && self.key_submissions.get() == 0
        {
            self.key_submissions.set(1);
            git2::Cred::ssh_key(
                expected_username,
                None,
                &prepared.registration.private_key_path,
                passphrase,
            )
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
                return Ok(git2::CertificateCheckStatus::CertificateOk);
            }
            return self.reject(SshTransportErrorKind::HostReplacementRequired {
                expected: pin.clone(),
                presented,
            });
        }
        if approved {
            return Ok(git2::CertificateCheckStatus::CertificateOk);
        }
        if prepared.trust.reapproval_required || prepared.approval.is_some() {
            return self.reject(SshTransportErrorKind::HostApprovalRequired { presented });
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
    attempt: &'a CallbackAttempt,
) -> git2::RemoteCallbacks<'a> {
    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.credentials(move |url, username, allowed| {
        attempt.credentials(prepared, passphrase, url, username, allowed)
    });
    callbacks.certificate_check(move |certificate, host| {
        attempt.check_host(prepared, host, certificate_identity(certificate))
    });
    callbacks
}
