use manyhands::repository::keys::{
    KeySourceToken, PassphraseResponse, PassphraseUseFailure, SecretPassphrase,
    SessionCredentialProvider, SessionCredentials, SessionUnlockFailure, SharedKeyId, UnlockReason,
    UnlockRequest,
};
use std::{cell::Cell, collections::VecDeque, fs, rc::Rc};

struct CountingProvider {
    calls: Rc<Cell<usize>>,
    responses: VecDeque<PassphraseResponse>,
}

impl CountingProvider {
    fn new(responses: impl IntoIterator<Item = PassphraseResponse>) -> (Self, Rc<Cell<usize>>) {
        let calls = Rc::new(Cell::new(0));
        (
            Self {
                calls: calls.clone(),
                responses: responses.into_iter().collect(),
            },
            calls,
        )
    }
}

impl SessionCredentialProvider for CountingProvider {
    fn request_passphrase(&mut self, _request: &UnlockRequest) -> PassphraseResponse {
        self.calls.set(self.calls.get() + 1);
        self.responses.pop_front().expect("queued response")
    }
}

fn supplied(value: &str) -> PassphraseResponse {
    PassphraseResponse::Supplied(SecretPassphrase::new(value.to_owned()).unwrap())
}

fn observed_request(label: &str) -> (tempfile::TempDir, UnlockRequest) {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("private-key");
    fs::write(&source, b"opaque fixture bytes").unwrap();
    let source = KeySourceToken::observe(&source).unwrap();
    (
        directory,
        UnlockRequest {
            key_id: SharedKeyId::new(),
            label: label.to_owned(),
            source,
            reason: UnlockReason::ProtectedKey,
        },
    )
}

#[test]
fn successful_unlock_prompts_once_per_session() {
    let (_source, request) = observed_request("primary");
    let (provider, calls) = CountingProvider::new([supplied("correct horse")]);
    let mut session = SessionCredentials::new(provider);
    assert!(!session.has_cached_passphrase(&request));

    for _ in 0..2 {
        assert_eq!(
            session.with_passphrase(request.clone(), |passphrase| {
                assert!(passphrase == "correct horse", "passphrase bytes changed");
                Ok(passphrase.len())
            }),
            Ok(13)
        );
    }
    assert_eq!(calls.get(), 1);
    assert!(session.has_cached_passphrase(&request));
    let mut different = request.clone();
    different.key_id = SharedKeyId::new();
    assert!(!session.has_cached_passphrase(&different));
    let (_other, other) = observed_request("other");
    different = request.clone();
    different.source = other.source;
    assert!(!session.has_cached_passphrase(&different));

    let (provider, second_session_calls) = CountingProvider::new([supplied("correct horse")]);
    let mut second_session = SessionCredentials::new(provider);
    assert!(second_session.with_passphrase(request, |_| Ok(())).is_ok());
    assert_eq!(calls.get() + second_session_calls.get(), 2);
}

#[test]
fn failed_validation_is_not_cached() {
    let (_source, request) = observed_request("primary");
    let (provider, calls) = CountingProvider::new([supplied("wrong"), supplied("correct")]);
    let mut session = SessionCredentials::new(provider);

    assert_eq!(
        session.with_passphrase(request.clone(), |_| {
            Err::<(), _>(PassphraseUseFailure::Rejected)
        }),
        Err(SessionUnlockFailure::Rejected)
    );
    assert_eq!(calls.get(), 1);
    assert!(session.with_passphrase(request, |_| Ok(())).is_ok());
    assert_eq!(calls.get(), 2);
}

#[test]
fn rejected_cached_material_is_evicted_without_prompting_again_in_the_same_call() {
    let (_source, request) = observed_request("primary");
    let (provider, calls) = CountingProvider::new([supplied("first"), supplied("replacement")]);
    let mut session = SessionCredentials::new(provider);
    assert!(session.with_passphrase(request.clone(), |_| Ok(())).is_ok());

    assert_eq!(
        session.with_passphrase(request.clone(), |_| {
            Err::<(), _>(PassphraseUseFailure::Rejected)
        }),
        Err(SessionUnlockFailure::Rejected)
    );
    assert_eq!(calls.get(), 1);
    assert!(session.with_passphrase(request, |_| Ok(())).is_ok());
    assert_eq!(calls.get(), 2);
}

#[test]
fn cancelled_unlock_prompts_only_on_retry() {
    let (_source, request) = observed_request("primary");
    let (provider, calls) =
        CountingProvider::new([PassphraseResponse::Cancelled, supplied("retry")]);
    let mut session = SessionCredentials::new(provider);

    assert_eq!(
        session.with_passphrase(request.clone(), |_| Ok(())),
        Err(SessionUnlockFailure::Cancelled)
    );
    assert_eq!(calls.get(), 1);
    assert!(session.with_passphrase(request, |_| Ok(())).is_ok());
    assert_eq!(calls.get(), 2);
}

#[test]
fn unavailable_provider_has_no_cached_value() {
    let (_source, request) = observed_request("primary");
    let (provider, calls) =
        CountingProvider::new([PassphraseResponse::Unavailable, supplied("available now")]);
    let mut session = SessionCredentials::new(provider);

    assert_eq!(
        session.with_passphrase(request.clone(), |_| Ok(())),
        Err(SessionUnlockFailure::ProviderUnavailable)
    );
    assert_eq!(calls.get(), 1);
    assert!(session.with_passphrase(request, |_| Ok(())).is_ok());
    assert_eq!(calls.get(), 2);
}

#[test]
fn new_key_or_source_evicts_secret() {
    let (_source_a, request_a) = observed_request("A");
    let (_source_b, request_b) = observed_request("B");
    let (provider, calls) =
        CountingProvider::new([supplied("A first"), supplied("B"), supplied("A second")]);
    let mut session = SessionCredentials::new(provider);

    assert!(
        session
            .with_passphrase(request_a.clone(), |_| Ok(()))
            .is_ok()
    );
    assert!(session.with_passphrase(request_b, |_| Ok(())).is_ok());
    assert!(session.with_passphrase(request_a, |_| Ok(())).is_ok());
    assert_eq!(calls.get(), 3);
}

#[test]
fn clear_and_invalidate_drop_cached_secret() {
    let (_source, request) = observed_request("primary");
    let key_id = request.key_id;
    let (provider, calls) = CountingProvider::new([
        supplied("first"),
        supplied("after clear"),
        supplied("after invalidate"),
    ]);
    let mut session = SessionCredentials::new(provider);

    assert!(session.with_passphrase(request.clone(), |_| Ok(())).is_ok());
    session.clear();
    assert!(session.with_passphrase(request.clone(), |_| Ok(())).is_ok());
    session.invalidate(key_id);
    assert!(session.with_passphrase(request, |_| Ok(())).is_ok());
    assert_eq!(calls.get(), 3);
}

#[test]
fn validation_failures_remain_typed() {
    for (failure, expected) in [
        (
            PassphraseUseFailure::SourceChanged,
            SessionUnlockFailure::SourceChanged,
        ),
        (
            PassphraseUseFailure::Unavailable,
            SessionUnlockFailure::Unavailable,
        ),
    ] {
        let (_source, request) = observed_request("primary");
        let (provider, _) = CountingProvider::new([supplied("secret")]);
        let mut session = SessionCredentials::new(provider);
        assert_eq!(
            session.with_passphrase(request, |_| Err::<(), _>(failure)),
            Err(expected)
        );
    }
}

#[test]
fn source_metadata_change_requires_a_new_prompt() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("private-key");
    fs::write(&path, b"first").unwrap();
    let first = UnlockRequest {
        key_id: SharedKeyId::new(),
        label: "primary".to_owned(),
        source: KeySourceToken::observe(&path).unwrap(),
        reason: UnlockReason::ProtectedKey,
    };
    let (provider, calls) = CountingProvider::new([supplied("first"), supplied("second")]);
    let mut session = SessionCredentials::new(provider);
    assert!(session.with_passphrase(first.clone(), |_| Ok(())).is_ok());

    fs::write(&path, b"different length").unwrap();
    let changed = UnlockRequest {
        source: KeySourceToken::observe(&path).unwrap(),
        ..first
    };
    assert!(session.with_passphrase(changed, |_| Ok(())).is_ok());
    assert_eq!(calls.get(), 2);
}

#[test]
fn credential_formatting_is_redacted() {
    let secret_text = "never format this";
    let (_source, request) = observed_request("public label");
    let (provider, _) = CountingProvider::new([supplied(secret_text)]);
    let session = SessionCredentials::new(provider);

    for formatted in [
        format!(
            "{:?}",
            SecretPassphrase::new(secret_text.to_owned()).unwrap()
        ),
        format!("{:?}", supplied(secret_text)),
        format!("{:?}", PassphraseResponse::Cancelled),
        format!("{:?}", PassphraseResponse::Unavailable),
        format!("{:?}", request.source),
        format!("{request:?}"),
        format!("{session:?}"),
        format!("{:?}", SecretPassphrase::new(String::new()).unwrap_err()),
        format!("{:?}", PassphraseUseFailure::Rejected),
        format!("{:?}", SessionUnlockFailure::Rejected),
    ] {
        assert!(!formatted.contains(secret_text));
    }
}

#[test]
fn credential_wrappers_have_exact_redacted_or_opaque_formatting() {
    use manyhands::repository::{
        OperationId,
        keys::{GenerateSharedKeyRequest, KeyProtection},
    };
    let secret = format!("formatting-{}", OperationId::new());
    let (_source, request) = observed_request("public label");
    let (provider, _) = CountingProvider::new([supplied(&secret)]);
    let mut session = SessionCredentials::new(provider);
    let operation_id = OperationId::new();
    let generation = GenerateSharedKeyRequest {
        operation_id,
        label: "public label".into(),
        protection: KeyProtection::Passphrase(SecretPassphrase::new(secret.clone()).unwrap()),
    };
    for (actual, expected) in [
        (
            format!("{:?}", SecretPassphrase::new(secret.clone()).unwrap()),
            "SecretPassphrase([REDACTED])".to_owned(),
        ),
        (
            format!("{:?}", supplied(&secret)),
            "Supplied([REDACTED])".to_owned(),
        ),
        (
            format!("{:?}", PassphraseResponse::Cancelled),
            "Cancelled".to_owned(),
        ),
        (
            format!("{:?}", PassphraseResponse::Unavailable),
            "Unavailable".to_owned(),
        ),
        (
            format!("{:?}", request.source),
            "KeySourceToken([OPAQUE])".to_owned(),
        ),
        (
            format!("{request:?}"),
            format!(
                "UnlockRequest {{ key_id: {:?}, label: \"public label\", source: KeySourceToken([OPAQUE]), reason: ProtectedKey }}",
                request.key_id
            ),
        ),
        (
            format!("{session:?}"),
            "SessionCredentials { provider: \"[REDACTED]\", cached_passphrase: false }".to_owned(),
        ),
        (
            format!("{:?}", generation.protection),
            "Passphrase(SecretPassphrase([REDACTED]))".to_owned(),
        ),
        (
            format!("{:?}", KeyProtection::Unencrypted),
            "Unencrypted".to_owned(),
        ),
        (
            format!("{generation:?}"),
            format!(
                "GenerateSharedKeyRequest {{ operation_id: {operation_id:?}, label: \"public label\", protection: Passphrase(SecretPassphrase([REDACTED])) }}"
            ),
        ),
    ] {
        assert!(
            actual == expected,
            "credential formatting must remain opaque/redacted"
        );
    }
    assert!(session.with_passphrase(request, |_| Ok(())).is_ok());
    assert!(
        format!("{session:?}")
            == "SessionCredentials { provider: \"[REDACTED]\", cached_passphrase: true }",
        "cached credential formatting must remain redacted"
    );
}
