//! `host approve`, `host replace` and `host inspect`: what of them needs
//! no server, and the fixtures of their results.
//!
//! A preview is made without a connection, and so are the answers to a
//! pin that moved and to a host already pinned to the key. Everything
//! that connects is in `tests/mutation_remote/host.rs`, against a real
//! server, where each envelope is checked against the same schemas: no
//! fixture can be kept of an envelope that names a server's port and the
//! key it made for itself.

use std::path::Path;

use manyhands::{
    repository::{
        HostApproveInput, HostObservationDto, HostReplaceInput, Mutation, PreviewDetailsDto,
        PreviewEffect,
        transport::{HostKeyIdentity, SshAuthority},
    },
    results::{Envelope, Outcome, ResultCode, Scope},
};

use super::support::{
    confirmation::{HOST_PREVIEW_SCHEMA, Home},
    git_home,
    golden::ContractCase,
    mutation::{self, REQUEST_1, REQUEST_2, REQUEST_3, recovery_actions},
};

const HOST_SCHEMA: &str = "host_mutation.schema.json";
const OBSERVATION_SCHEMA: &str = "host_observation.schema.json";

/// A location that is never connected to. Its user and its path are
/// the remote's own business: neither may reach a result.
const LOCATION: &str = "ssh://SENTINEL-user-41c7@Git.Example.Invalid:2222/SENTINEL-path-8be2.git";

fn authority() -> SshAuthority {
    SshAuthority {
        host: "git.example.invalid".to_owned(),
        port: 2222,
    }
}

/// A key a pin can hold: the fingerprint is `letter` forty-two times and
/// the one last character every digest of its length can end with.
fn key(letter: &str) -> HostKeyIdentity {
    HostKeyIdentity {
        algorithm: "ssh-ed25519".to_owned(),
        sha256: format!("SHA256:{}A", letter.repeat(42)),
    }
}

fn approve(root: &Path, key: &HostKeyIdentity) -> Mutation {
    Mutation::HostApprove(HostApproveInput {
        root: root.to_owned(),
        authority: authority(),
        algorithm: key.algorithm.clone(),
        fingerprint: key.sha256.clone(),
    })
}

fn replace(root: &Path, old: &HostKeyIdentity, new: &HostKeyIdentity) -> Mutation {
    Mutation::HostReplace(HostReplaceInput {
        root: root.to_owned(),
        authority: authority(),
        algorithm: new.algorithm.clone(),
        fingerprint: new.sha256.clone(),
        old_fingerprint: old.sha256.clone(),
    })
}

/// A repository whose publication remote names a host nothing answers
/// at.
fn home() -> Home {
    let home = Home::new();
    home.add_remote("origin", LOCATION);
    home.select(REQUEST_3, Some("origin"));
    home
}

/// Pins `key` as another actor's approval would, with no connection.
fn pin(home: &Home, key: &HostKeyIdentity) {
    home.service()
        .approve_host_pin_for_testing(&authority(), key)
        .unwrap();
}

/// Checks an envelope of a host command against its fixture. What
/// differs from run to run is replaced.
fn assert_golden<T: serde::Serialize>(
    name: &str,
    root: &Path,
    data_schema: &str,
    envelope: &Envelope<T>,
) {
    let value = serde_json::to_value(envelope).unwrap();
    let mut placeholders = vec![(root.to_str().unwrap(), "<root>")];
    if let Some(confirmation) = value
        .pointer("/data/confirmation_id")
        .and_then(|found| found.as_str())
    {
        placeholders.push((confirmation, "<confirmation>"));
    }
    mutation::assert_contract(
        &ContractCase {
            name,
            data_schema: (!value["data"].is_null()).then_some(data_schema),
            placeholders: &placeholders,
            sentinels: &["SENTINEL-user-41c7", "SENTINEL-path-8be2"],
        },
        envelope,
    );
}

#[test]
fn the_previews_and_the_local_answers_of_a_host_approval_match_their_schemas_and_goldens() {
    git_home::isolated();
    let home = home();
    let root = home.root().to_owned();
    let (first, second) = (key("a"), key("b"));

    // A host with no pin: the preview of its approval.
    let shown = home.prepare(&approve(&root, &first));
    assert_eq!(shown.outcome, Outcome::Success, "{}", shown.message);
    assert_golden("host_approve_preview", &root, HOST_PREVIEW_SCHEMA, &shown);
    let preview = shown.data.unwrap();
    assert_eq!(
        preview
            .effects
            .iter()
            .map(|entry| entry.effect)
            .collect::<Vec<_>>(),
        [PreviewEffect::HostPinned]
    );
    let PreviewDetailsDto::Host(details) = &preview.details else {
        panic!("the details of a host preview");
    };
    assert_eq!(details.host, "git.example.invalid");
    assert!(details.fetch && details.push);
    assert_eq!(details.current_fingerprint, None);

    // Another actor pins the host: the preview is stale, and the request
    // is refused before anything is connected to. Nothing answers at the
    // host, so a connection would be a failure of the transport.
    pin(&home, &second);
    let stale = home.execute(REQUEST_1, &preview.confirmation_id, approve(&root, &first));
    assert_eq!(stale.code, ResultCode::ExternalChange);
    assert_eq!(recovery_actions(&stale), ["request.prepare"]);
    assert_golden("host_approve_external_change", &root, HOST_SCHEMA, &stale);

    // A host that has a pin is not approved: its pin is replaced.
    let pinned = home.prepare(&approve(&root, &first));
    assert_eq!(pinned.code, ResultCode::HostReplacementRequired);
    assert!(pinned.data.is_none());
    let replacing = home.prepare(&replace(&root, &second, &first));
    assert_eq!(replacing.outcome, Outcome::Success, "{}", replacing.message);
    assert_golden(
        "host_replace_preview",
        &root,
        HOST_PREVIEW_SCHEMA,
        &replacing,
    );
    let replacing = replacing.data.unwrap();
    assert_eq!(
        replacing
            .effects
            .iter()
            .map(|entry| entry.effect)
            .collect::<Vec<_>>(),
        [PreviewEffect::HostPinReplaced]
    );

    // The host is pinned to the key meanwhile: the request's end state
    // holds, and it is a no-op with no connection.
    pin(&home, &first);
    let done = home.execute(
        REQUEST_2,
        &replacing.confirmation_id,
        replace(&root, &second, &first),
    );
    assert_eq!(done.code, ResultCode::AlreadyApplied);
    assert_eq!(done.outcome, Outcome::Noop);
    assert_golden("host_replace_noop", &root, HOST_SCHEMA, &done);
    // And no preview is given of it.
    let noop = home.prepare(&approve(&root, &first));
    assert_eq!(noop.outcome, Outcome::Noop);
    assert_golden(
        "host_approve_preview_noop",
        &root,
        HOST_PREVIEW_SCHEMA,
        &noop,
    );
}

/// The shape of an observation. One that was made is of a real server,
/// and is checked against the same schema where it is made.
#[test]
fn a_host_observation_matches_its_schema_and_golden() {
    git_home::isolated();
    let (presented, pinned) = (key("a"), key("b"));
    let envelope = Envelope::read_success(
        "host inspect",
        Scope {
            repository: Some("<root>".to_owned()),
            ..Scope::default()
        },
        HostObservationDto {
            host: authority().host,
            port: authority().port,
            fetch: true,
            push: false,
            algorithm: presented.algorithm,
            fingerprint: presented.sha256,
            pinned_algorithm: Some(pinned.algorithm),
            pinned_fingerprint: Some(pinned.sha256),
            reapproval_required: false,
        },
    );
    mutation::assert_contract(
        &ContractCase {
            name: "host_inspect_observation",
            data_schema: Some(OBSERVATION_SCHEMA),
            placeholders: &[],
            sentinels: &[],
        },
        &envelope,
    );
}

/// An observation of a host the remote does not name, or of no
/// repository, is refused before any connection: this target never
/// starts the transport, so a connection would be `internal_error`.
#[test]
fn an_observation_that_names_no_host_of_the_remote_is_refused_before_any_connection() {
    git_home::isolated();
    let home = home();
    let mut session = manyhands::repository::keys::SessionCredentials::new(NoPassphrase);
    // A host the publication remote does not name.
    let elsewhere = SshAuthority {
        host: "elsewhere.example.invalid".to_owned(),
        port: 2222,
    };
    let unnamed = home
        .service()
        .observe_host(home.root(), Some(&elsewhere), &mut session);
    assert_eq!(unnamed.code, ResultCode::InvalidInput);
    assert!(unnamed.data.is_none());
    assert_eq!(unnamed.request_id, None);
    assert_eq!(unnamed.scope.repository.as_deref(), home.root().to_str());
    // A directory that is no repository.
    let nowhere = tempfile::tempdir().unwrap();
    let unresolved = home
        .service()
        .observe_host(nowhere.path(), None, &mut session);
    assert_eq!(unresolved.code, ResultCode::NotRepository);
    for envelope in [&unnamed, &unresolved] {
        let value = serde_json::to_value(envelope).unwrap();
        super::support::schema::check_published("envelope.schema.json", &value).unwrap();
        assert!(
            !value.to_string().contains("SENTINEL"),
            "{}",
            envelope.message
        );
    }
}

struct NoPassphrase;

impl manyhands::repository::keys::SessionCredentialProvider for NoPassphrase {
    fn request_passphrase(
        &mut self,
        _: &manyhands::repository::keys::UnlockRequest,
    ) -> manyhands::repository::keys::PassphraseResponse {
        manyhands::repository::keys::PassphraseResponse::Unavailable
    }
}
