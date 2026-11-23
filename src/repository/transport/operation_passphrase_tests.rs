use super::*;

#[test]
fn a_proven_passphrase_survives_failures_that_are_not_about_it() {
    use SshTransportErrorKind::*;
    let cases = [
        // (kind, cached passphrase evicted, new passphrase evicted)
        (KeyRejected, false, true),
        (UnlockFailed, false, true),
        (KeyInvalidOrUnsupported, false, true),
        (TransportUnavailable, false, false),
        (RemoteUnavailable, false, false),
        (HostVerificationUnavailable, false, false),
        (SelectionChanged, true, true),
        (KeySourceChanged, true, true),
        (EndpointChanged, true, true),
        (HostTrustChanged, true, true),
    ];
    for (kind, cached_evicted, new_evicted) in cases {
        assert_eq!(invalidates_secret(true, &kind), cached_evicted, "{kind:?}");
        assert_eq!(invalidates_secret(false, &kind), new_evicted, "{kind:?}");
        // The connection path agrees: only a change under a cached passphrase,
        // or any failure of a new one, reports it as used up.
        assert_eq!(
            passphrase_use_failure(true, &kind) == PassphraseUseFailure::Unrelated,
            !cached_evicted,
            "{kind:?}"
        );
        assert_eq!(
            passphrase_use_failure(false, &kind),
            PassphraseUseFailure::Unavailable,
            "{kind:?}"
        );
    }
}

#[test]
fn a_rejection_of_a_proven_passphrase_reads_as_a_rejected_key() {
    let error = SshTransportError {
        root: std::path::PathBuf::new(),
        remote_name: String::new(),
        direction: SshDirection::Fetch,
        selected_key_id: None,
        authority: None,
        kind: SshTransportErrorKind::UnlockFailed,
    };
    assert_eq!(
        blame_for_cached(true, error.clone()).kind,
        SshTransportErrorKind::KeyRejected
    );
    assert_eq!(
        blame_for_cached(false, error).kind,
        SshTransportErrorKind::UnlockFailed
    );
}
