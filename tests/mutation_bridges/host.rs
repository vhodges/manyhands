//! `host approve`, `host replace` and `host observe`: what of them needs
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
        HostApproveInput, HostObservationDto, HostReplaceInput, HostReplacementDto, Mutation,
        PrepareDataDto, PreviewDetailsDto, PreviewEffect,
        transport::{HostKeyIdentity, SshAuthority},
    },
    results::{Envelope, Outcome, ResultCode, Scope},
};

use super::support::{
    confirmation::{HOST_PREVIEW_SCHEMA, HOST_REPLACEMENT_SCHEMA, Home, prepare_whole_with},
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
    assert_eq!(details.pinned_fingerprint, None);

    // Another actor pins the host: the preview is stale, and the request
    // is refused before anything is connected to. Nothing answers at the
    // host, so a connection would be a failure of the transport.
    pin(&home, &second);
    let stale = home.execute(REQUEST_1, &preview.confirmation_id, approve(&root, &first));
    assert_eq!(stale.code, ResultCode::ExternalChange);
    assert_eq!(recovery_actions(&stale), ["request.prepare"]);
    assert_golden("host_approve_external_change", &root, HOST_SCHEMA, &stale);

    // A host that has a pin is not approved: its pin is replaced.
    // The refusal says what the replacement is formed from, as the same
    // code from a synchronization does: the host, its pin, the key this
    // request named, and the action. Nothing was connected to, so it
    // says nothing of a key the host presents.
    let pinned = prepare_whole_with(home.service(), &approve(&root, &first));
    assert_eq!(pinned.code, ResultCode::HostReplacementRequired);
    assert_eq!(
        pinned.data,
        Some(PrepareDataDto::HostReplacement(HostReplacementDto {
            host: "git.example.invalid".to_owned(),
            port: 2222,
            algorithm: first.algorithm.clone(),
            fingerprint: first.sha256.clone(),
            pinned_algorithm: second.algorithm.clone(),
            pinned_fingerprint: second.sha256.clone(),
        }))
    );
    assert_eq!(recovery_actions(&pinned), ["host.replace"]);
    assert_eq!(
        pinned.recovery[0].arguments["authority"],
        "git.example.invalid:2222"
    );
    assert_eq!(
        pinned.recovery[0].arguments["root"].as_str(),
        pinned.scope.repository.as_deref()
    );
    assert_golden(
        "host_approve_host_replacement_required",
        &root,
        HOST_REPLACEMENT_SCHEMA,
        &pinned,
    );
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
        "host observe",
        Scope {
            repository: Some("<root>".to_owned()),
            ..Scope::default()
        },
        HostObservationDto {
            host: authority().host,
            port: authority().port,
            fetch: true,
            push: false,
            presented_algorithm: presented.algorithm,
            presented_fingerprint: presented.sha256,
            pinned_algorithm: Some(pinned.algorithm),
            pinned_fingerprint: Some(pinned.sha256),
            reapproval_required: false,
        },
    );
    mutation::assert_contract(
        &ContractCase {
            name: "host_observe",
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

/// An observation that fails offers what every read and every mutation
/// offers for the same code on the same repository, and is told from the
/// pin read of `host inspect` by its own command name.
#[test]
fn an_observation_that_fails_keeps_the_recovery_its_read_suggested() {
    git_home::isolated();
    let home = home();
    let mut session = manyhands::repository::keys::SessionCredentials::new(NoPassphrase);
    // The repository was enabled and is no longer registered.
    home.remove_registration_by_hand();

    let lost = home.service().observe_host(home.root(), None, &mut session);

    assert_eq!(lost.command, "host observe");
    assert_eq!(lost.code, ResultCode::RepositoryNotRegistered);
    assert!(lost.data.is_none());
    assert_eq!(recovery_actions(&lost), ["index.rebuild"]);
    assert_eq!(
        lost.recovery[0].arguments["root"].as_str(),
        home.root().to_str()
    );
    mutation::assert_contract(
        &ContractCase {
            name: "host_observe_repository_not_registered",
            data_schema: None,
            placeholders: &[(home.root().to_str().unwrap(), "<root>")],
            sentinels: &["SENTINEL-user-41c7", "SENTINEL-path-8be2"],
        },
        &lost,
    );
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

/// A host written another way than the remote writes it either names
/// the remote's host, as a pin would store it, or is no host: it is
/// never a second host beside the one the remote names. Each is
/// answered by `prepare`, which connects to nothing; this target never
/// starts the transport.
#[test]
fn a_host_written_another_way_names_the_remote_s_host_or_is_invalid_input() {
    git_home::isolated();
    let home = Home::new();
    home.add_remote("origin", LOCATION);
    home.select(REQUEST_3, Some("origin"));
    let root = home.root().to_owned();
    let approving = |host: &str, port: u16| {
        Mutation::HostApprove(HostApproveInput {
            root: root.clone(),
            authority: SshAuthority {
                host: host.to_owned(),
                port,
            },
            algorithm: key("a").algorithm,
            fingerprint: key("a").sha256,
        })
    };
    // `Some` is the host the preview shows; `None` is `invalid_input`.
    let cases: [(&str, &str, u16, Option<&str>); 17] = [
        // The remote names a host in mixed case and a port.
        (
            LOCATION,
            "git.example.invalid",
            2222,
            Some("git.example.invalid"),
        ),
        (
            LOCATION,
            "GIT.Example.INVALID",
            2222,
            Some("git.example.invalid"),
        ),
        (LOCATION, "git.example.invalid.", 2222, None),
        (LOCATION, "git.example.invalid", 22, None),
        (LOCATION, "git.example.invalid:2222", 2222, None),
        (LOCATION, "git.example.invalid:2222", 22, None),
        (
            LOCATION,
            "SENTINEL-user-41c7@git.example.invalid",
            2222,
            None,
        ),
        (LOCATION, "user@git.example.invalid", 2222, None),
        (LOCATION, "git.example.invalid/path", 2222, None),
        (LOCATION, "[git.example.invalid]", 2222, None),
        (LOCATION, " git.example.invalid", 2222, None),
        // The remote names no port: that is port 22, and no other.
        (
            "ssh://git@git.example.invalid/team/project.git",
            "git.example.invalid",
            22,
            Some("git.example.invalid"),
        ),
        (
            "ssh://git@git.example.invalid/team/project.git",
            "git.example.invalid",
            2222,
            None,
        ),
        (
            "ssh://git@git.example.invalid:22/team/project.git",
            "git.example.invalid",
            22,
            Some("git.example.invalid"),
        ),
        // The remote names an IPv6 address.
        (
            "ssh://git@[2001:db8::1]:2222/team/project.git",
            "2001:db8::1",
            2222,
            Some("2001:db8::1"),
        ),
        (
            "ssh://git@[2001:db8::1]:2222/team/project.git",
            "2001:0DB8:0:0:0:0:0:1",
            2222,
            Some("2001:db8::1"),
        ),
        (
            "ssh://git@[2001:db8::1]:2222/team/project.git",
            "[2001:db8::1]",
            2222,
            None,
        ),
    ];
    for (location, host, port, expected) in cases {
        home.git().remote_set_url("origin", location).unwrap();
        let envelope = home.prepare(&approving(host, port));
        let shown = envelope
            .data
            .as_ref()
            .map(|preview| match &preview.details {
                PreviewDetailsDto::Host(details) => {
                    assert_eq!(details.port, port, "{location} {host}:{port}");
                    details.host.clone()
                }
                _ => panic!("the details of a host preview"),
            });
        assert_eq!(shown.as_deref(), expected, "{location} {host}:{port}");
        if expected.is_none() {
            assert_eq!(
                envelope.code,
                ResultCode::InvalidInput,
                "{location} {host}:{port}"
            );
        }
        let rendered = serde_json::to_string(&envelope).unwrap();
        assert!(!rendered.contains("SENTINEL"), "{location} {host}:{port}");
    }
    // No pin came of any of it.
    assert!(home.service().list_host_pins().unwrap().items.is_empty());
}
