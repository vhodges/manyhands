use std::{
    fmt,
    path::{Path, PathBuf},
};
use zeroize::Zeroizing;

use super::{
    KeyMaterialAction, KeyMaterialError, KeyMaterialErrorKind, SharedKeyId,
    storage::observe_regular_source,
};

/// An owned passphrase whose application-controlled storage is erased on drop.
pub struct SecretPassphrase {
    value: Zeroizing<String>,
    #[cfg(test)]
    _drop_witness: Option<DropWitness>,
}

#[cfg(test)]
struct DropWitness(std::sync::Arc<std::sync::atomic::AtomicUsize>);

#[cfg(test)]
impl Drop for DropWitness {
    fn drop(&mut self) {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidPassphrase;

impl SecretPassphrase {
    pub fn new(value: String) -> Result<Self, InvalidPassphrase> {
        let value = Zeroizing::new(value);
        if value.is_empty() || value.contains('\0') {
            return Err(InvalidPassphrase);
        }
        Ok(Self {
            value,
            #[cfg(test)]
            _drop_witness: None,
        })
    }

    pub(super) fn expose(&self) -> &str {
        &self.value
    }

    #[cfg(test)]
    fn with_drop_witness(
        value: String,
        drops: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    ) -> Self {
        Self {
            value: Zeroizing::new(value),
            _drop_witness: Some(DropWitness(drops)),
        }
    }
}
impl fmt::Debug for SecretPassphrase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretPassphrase([REDACTED])")
    }
}
impl fmt::Display for InvalidPassphrase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("passphrase must be nonempty and contain no NUL")
    }
}
impl std::error::Error for InvalidPassphrase {}

/// Opaque, non-persisted metadata identifying an observed regular key source.
#[derive(Clone, PartialEq, Eq)]
pub struct KeySourceToken {
    path: PathBuf,
    encoded_identity: String,
}

impl KeySourceToken {
    /// Observes a readable regular source without reading its contents.
    pub fn observe(path: &Path) -> Result<Self, KeyMaterialError> {
        let path = path.canonicalize().map_err(|error| KeyMaterialError {
            operation: KeyMaterialAction::Inspect,
            key_id: None,
            operation_id: None,
            kind: if error.kind() == std::io::ErrorKind::NotFound {
                KeyMaterialErrorKind::SourceMissing
            } else {
                KeyMaterialErrorKind::SourceUnreadable
            },
        })?;
        let encoded_identity = observe_regular_source(&path)?.encode();
        Ok(Self {
            path,
            encoded_identity,
        })
    }
}

impl fmt::Debug for KeySourceToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("KeySourceToken([OPAQUE])")
    }
}

/// Non-secret context supplied to a credential provider for one unlock attempt.
#[derive(Clone, PartialEq, Eq)]
pub struct UnlockRequest {
    pub key_id: SharedKeyId,
    pub label: String,
    pub source: KeySourceToken,
}

impl fmt::Debug for UnlockRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("UnlockRequest")
            .field("key_id", &self.key_id)
            .field("label", &self.label)
            .field("source", &self.source)
            .finish()
    }
}

/// The result of asking the caller for a passphrase.
///
/// Credential responses deliberately do not implement serialization:
///
/// ```compile_fail
/// # use manyhands::repository::keys::{PassphraseResponse, SecretPassphrase};
/// let response = PassphraseResponse::Supplied(SecretPassphrase::new("secret".into()).unwrap());
/// let _ = serde_yaml::to_string(&response);
/// ```
pub enum PassphraseResponse {
    Supplied(SecretPassphrase),
    Cancelled,
    Unavailable,
}

impl fmt::Debug for PassphraseResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Supplied(_) => formatter.write_str("Supplied([REDACTED])"),
            Self::Cancelled => formatter.write_str("Cancelled"),
            Self::Unavailable => formatter.write_str("Unavailable"),
        }
    }
}

pub trait SessionCredentialProvider {
    fn request_passphrase(&mut self, request: &UnlockRequest) -> PassphraseResponse;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassphraseUseFailure {
    Rejected,
    SourceChanged,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionUnlockFailure {
    Cancelled,
    ProviderUnavailable,
    InvalidPassphrase,
    Rejected,
    SourceChanged,
    Unavailable,
}

impl fmt::Display for SessionUnlockFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Cancelled => "passphrase request was cancelled",
            Self::ProviderUnavailable => "passphrase provider is unavailable",
            Self::InvalidPassphrase => "the supplied passphrase is invalid",
            Self::Rejected => "the supplied passphrase was rejected",
            Self::SourceChanged => "the key source changed during unlock",
            Self::Unavailable => "the key could not be used",
        })
    }
}

impl std::error::Error for SessionUnlockFailure {}

struct CachedPassphrase {
    key_id: SharedKeyId,
    source: KeySourceToken,
    passphrase: SecretPassphrase,
}

/// Caller-owned passphrase state for one application session.
///
/// The validation callback receives a temporary borrow. Callback consumers are
/// trusted not to copy or log the passphrase.
///
/// Session credentials deliberately do not implement serialization:
///
/// ```compile_fail
/// # use manyhands::repository::keys::{PassphraseResponse, SessionCredentialProvider, SessionCredentials, UnlockRequest};
/// # struct Provider;
/// # impl SessionCredentialProvider for Provider {
/// #   fn request_passphrase(&mut self, _: &UnlockRequest) -> PassphraseResponse { PassphraseResponse::Cancelled }
/// # }
/// let session = SessionCredentials::new(Provider);
/// let _ = serde_yaml::to_string(&session);
/// ```
pub struct SessionCredentials<P> {
    provider: P,
    cached: Option<CachedPassphrase>,
}

impl<P> fmt::Debug for SessionCredentials<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SessionCredentials")
            .field("provider", &"[REDACTED]")
            .field("cached_passphrase", &self.cached.is_some())
            .finish()
    }
}

impl<P: SessionCredentialProvider> SessionCredentials<P> {
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            cached: None,
        }
    }

    pub fn clear(&mut self) {
        self.cached = None;
    }

    pub fn invalidate(&mut self, key_id: SharedKeyId) {
        if self.cached.as_ref().map(|cached| cached.key_id) == Some(key_id) {
            self.clear();
        }
    }

    pub fn with_passphrase<T>(
        &mut self,
        request: UnlockRequest,
        use_passphrase: impl FnOnce(&str) -> Result<T, PassphraseUseFailure>,
    ) -> Result<T, SessionUnlockFailure> {
        let cache_matches = self.cached.as_ref().is_some_and(|cached| {
            cached.key_id == request.key_id && cached.source == request.source
        });

        if cache_matches {
            let result = use_passphrase(
                self.cached
                    .as_ref()
                    .expect("cache match requires an entry")
                    .passphrase
                    .expose(),
            );
            if result.is_err() {
                self.clear();
            }
            return result.map_err(SessionUnlockFailure::from);
        }
        self.clear();

        match self.provider.request_passphrase(&request) {
            PassphraseResponse::Supplied(passphrase) => {
                let result = use_passphrase(passphrase.expose());
                match result {
                    Ok(value) => {
                        self.cached = Some(CachedPassphrase {
                            key_id: request.key_id,
                            source: request.source,
                            passphrase,
                        });
                        Ok(value)
                    }
                    Err(failure) => Err(failure.into()),
                }
            }
            PassphraseResponse::Cancelled => Err(SessionUnlockFailure::Cancelled),
            PassphraseResponse::Unavailable => Err(SessionUnlockFailure::ProviderUnavailable),
        }
    }
}

impl From<PassphraseUseFailure> for SessionUnlockFailure {
    fn from(failure: PassphraseUseFailure) -> Self {
        match failure {
            PassphraseUseFailure::Rejected => Self::Rejected,
            PassphraseUseFailure::SourceChanged => Self::SourceChanged,
            PassphraseUseFailure::Unavailable => Self::Unavailable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::Cell,
        collections::VecDeque,
        fs,
        rc::Rc,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };

    struct Provider {
        responses: VecDeque<PassphraseResponse>,
    }

    impl SessionCredentialProvider for Provider {
        fn request_passphrase(&mut self, _: &UnlockRequest) -> PassphraseResponse {
            self.responses.pop_front().unwrap()
        }
    }

    #[test]
    fn clear_invalidate_and_session_drop_destroy_cached_secret() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("private-key");
        fs::write(&path, b"opaque fixture bytes").unwrap();
        let request = UnlockRequest {
            key_id: SharedKeyId::new(),
            label: "primary".to_owned(),
            source: KeySourceToken::observe(&path).unwrap(),
        };
        let drops = Arc::new(AtomicUsize::new(0));
        let response = || {
            PassphraseResponse::Supplied(SecretPassphrase::with_drop_witness(
                "secret".to_owned(),
                drops.clone(),
            ))
        };
        let provider = Provider {
            responses: [response(), response(), response()].into(),
        };
        let mut session = SessionCredentials::new(provider);
        let validations = Rc::new(Cell::new(0));
        let validate = || {
            let validations = validations.clone();
            move |_: &str| {
                validations.set(validations.get() + 1);
                Ok(())
            }
        };

        session
            .with_passphrase(request.clone(), validate())
            .unwrap();
        session.clear();
        assert_eq!(drops.load(Ordering::SeqCst), 1);

        session
            .with_passphrase(request.clone(), validate())
            .unwrap();
        session.invalidate(SharedKeyId::new());
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        session.invalidate(request.key_id);
        assert_eq!(drops.load(Ordering::SeqCst), 2);

        session.with_passphrase(request, validate()).unwrap();
        drop(session);
        assert_eq!(drops.load(Ordering::SeqCst), 3);
        assert_eq!(validations.get(), 3);
    }
}
