//! `host observe`, `host approve` and `host replace`, against a real
//! authenticated server.
//!
//! Every envelope and every store is scanned for the world's private
//! material before anything is asserted of it: the passphrase, the key
//! file, the remote's location and, for these commands, the path of the
//! selected key, which the transport reads the key from.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use super::*;

pub(crate) const CASES: &[ssh_harness::Case] = &[
    (
        "observing_a_host_reports_what_it_presents_and_its_pin_and_pins_nothing",
        observing_a_host_reports_what_it_presents_and_its_pin_and_pins_nothing,
    ),
    (
        "approving_the_observed_fingerprint_stores_the_pin_and_publishes_nothing",
        approving_the_observed_fingerprint_stores_the_pin_and_publishes_nothing,
    ),
    (
        "a_synchronization_stopped_for_its_host_goes_on_once_the_offered_approval_is_made",
        a_synchronization_stopped_for_its_host_goes_on_once_the_offered_approval_is_made,
    ),
    (
        "a_fingerprint_the_host_does_not_present_is_host_mismatch_and_pins_nothing",
        a_fingerprint_the_host_does_not_present_is_host_mismatch_and_pins_nothing,
    ),
    (
        "a_rotated_host_key_is_replaced_by_naming_the_old_and_the_new_fingerprint",
        a_rotated_host_key_is_replaced_by_naming_the_old_and_the_new_fingerprint,
    ),
    (
        "a_host_the_publication_remote_does_not_name_is_invalid_input",
        a_host_the_publication_remote_does_not_name_is_invalid_input,
    ),
    (
        "the_fetch_host_and_the_push_host_are_each_approved_in_their_own_direction",
        the_fetch_host_and_the_push_host_are_each_approved_in_their_own_direction,
    ),
    (
        "a_pin_changed_after_the_preview_is_an_external_change_before_any_connection",
        a_pin_changed_after_the_preview_is_an_external_change_before_any_connection,
    ),
    (
        "a_pin_changed_between_the_observation_and_the_connection_is_not_written_over",
        a_pin_changed_between_the_observation_and_the_connection_is_not_written_over,
    ),
    (
        "a_remote_repointed_at_another_pinned_host_is_an_external_change_naming_no_key",
        a_remote_repointed_at_another_pinned_host_is_an_external_change_naming_no_key,
    ),
    (
        "a_call_that_fails_after_the_pin_was_stored_says_so_and_its_retry_is_a_no_op",
        a_call_that_fails_after_the_pin_was_stored_says_so_and_its_retry_is_a_no_op,
    ),
    (
        "an_approval_with_a_locked_key_and_no_passphrase_is_unlock_required_and_pins_nothing",
        an_approval_with_a_locked_key_and_no_passphrase_is_unlock_required_and_pins_nothing,
    ),
    (
        "approving_a_host_already_pinned_to_the_key_is_a_no_op",
        approving_a_host_already_pinned_to_the_key_is_a_no_op,
    ),
    (
        "an_approval_under_the_reapproval_marker_stores_the_pin_and_leaves_the_marker",
        an_approval_under_the_reapproval_marker_stores_the_pin_and_leaves_the_marker,
    ),
    (
        "a_connection_dropped_before_the_advertisement_pins_nothing_and_its_retry_pins_once",
        a_connection_dropped_before_the_advertisement_pins_nothing_and_its_retry_pins_once,
    ),
    (
        "a_finished_approval_is_replayed_without_a_connection_and_a_changed_input_is_a_mismatch",
        a_finished_approval_is_replayed_without_a_connection_and_a_changed_input_is_a_mismatch,
    ),
    (
        "an_approval_whose_result_was_lost_after_the_pin_was_stored_is_a_no_op_on_retry",
        an_approval_whose_result_was_lost_after_the_pin_was_stored_is_a_no_op_on_retry,
    ),
    (
        "the_output_privacy_scan_passes_what_a_host_approval_emits_and_fails_the_key_path",
        the_output_privacy_scan_passes_what_a_host_approval_emits_and_fails_the_key_path,
    ),
    (
        "every_command_that_approves_a_host_keeps_the_rules_of_the_boundary",
        every_command_that_approves_a_host_keeps_the_rules_of_the_boundary,
    ),
];

/// Cases run only as children of the last case above.
pub(crate) const OUTPUT_CONTROLS: &[ssh_harness::Case] = &[
    ("control_prints_what_a_host_approval_emits", || {
        control_prints(false)
    }),
    ("control_prints_the_path_of_the_selected_key", || {
        control_prints(true)
    }),
];

const HOST_OBSERVATION_SCHEMA: &str = "host_observation.schema.json";
const HOST_PREVIEW_SCHEMA: &str = "host_preview.schema.json";
const HOST_REPLACEMENT_SCHEMA: &str = "host_replacement.schema.json";
const HOST_MUTATION_SCHEMA: &str = "host_mutation.schema.json";

fn authority(server: &SshRemoteFixture) -> SshAuthority {
    SshAuthority {
        host: "127.0.0.1".into(),
        port: server.address().port(),
    }
}

fn approve_of(w: &World, server: &SshRemoteFixture, key: &HostKeyIdentity) -> Mutation {
    Mutation::HostApprove(HostApproveInput {
        root: w.root.clone(),
        authority: authority(server),
        algorithm: key.algorithm.clone(),
        fingerprint: key.sha256.clone(),
    })
}

/// The approval of the key the world's server presents.
fn approval(w: &World) -> Mutation {
    approve_of(w, &w.server, &w.server.host_identity())
}

fn replacement(w: &World, old: &str, key: &HostKeyIdentity) -> Mutation {
    Mutation::HostReplace(HostReplaceInput {
        root: w.root.clone(),
        authority: authority(&w.server),
        algorithm: key.algorithm.clone(),
        fingerprint: key.sha256.clone(),
        old_fingerprint: old.to_owned(),
    })
}

/// A key no server of the fixture presents.
fn another_key() -> Result<HostKeyIdentity, FixtureError> {
    let key = generate_key()?;
    Ok(HostKeyIdentity {
        algorithm: key.algorithm().to_string(),
        sha256: key
            .public_key()
            .fingerprint(russh::keys::HashAlg::Sha256)
            .to_string(),
    })
}

/// The path the selected key is read from: private, and read by every
/// connection these commands make.
fn key_path(w: &World) -> Vec<u8> {
    w.server
        .client_key_path()
        .to_string_lossy()
        .as_bytes()
        .to_vec()
}

/// The path of the selected key in every form a result could hold it
/// in: as it is, as JSON writes it and as `Debug` writes it. Where a
/// path holds backslashes the last two double them, and a scan for the
/// path as it is would miss it there.
fn key_path_forms(w: &World) -> Vec<Vec<u8>> {
    let raw = key_path(w);
    let text = String::from_utf8_lossy(&raw).into_owned();
    let json = serde_json::to_string(&text).expect("a path serializes");
    let debug = format!("{text:?}");
    let mut forms = vec![raw];
    for quoted in [json, debug] {
        let inner = quoted.as_bytes()[1..quoted.len() - 1].to_vec();
        if !forms.contains(&inner) {
            forms.push(inner);
        }
    }
    forms
}

/// How many connections the world's server has accepted, whatever
/// became of them: one the client gave up at the host's key is counted,
/// which the count of clients that authenticated does not see.
fn sockets(w: &World) -> usize {
    w.server.accepted_sockets()
}

/// Scans what a call returned, and what the stores hold after it, for
/// the world's private material and the path of the selected key.
fn scanned<T: serde::Serialize + std::fmt::Debug>(w: &World, envelope: &T) -> serde_json::Value {
    let mut private = w.probes.clone();
    private.extend(key_path_forms(w));
    // The case's output is scanned for the path as well.
    ssh_privacy::save(&private).expect("the probes");
    let rendered = serde_json::to_vec(envelope).expect("an envelope serializes");
    ssh_privacy::clean(&rendered, &private).expect("redacted envelope");
    ssh_privacy::clean(format!("{envelope:?}").as_bytes(), &private).expect("redacted envelope");
    w.privacy().expect("redacted durable state");
    // The records of requests and of confirmations hold no path of a
    // key either. The key registry does, and is not scanned for it.
    let db = w.db().expect("the database");
    for table in ["request_records", "confirmation_records"] {
        let mut statement = db
            .prepare(&format!("SELECT * FROM {table}"))
            .expect("the table");
        let columns = statement.column_count();
        let mut rows = statement.query([]).expect("the rows");
        while let Some(row) = rows.next().expect("a row") {
            for column in 0..columns {
                if let rusqlite::types::ValueRef::Text(bytes)
                | rusqlite::types::ValueRef::Blob(bytes) = row.get_ref(column).expect("a column")
                {
                    ssh_privacy::clean(bytes, &private).expect("a redacted record");
                }
            }
        }
    }
    let value = serde_json::to_value(envelope).expect("an envelope serializes");
    schema::check_published("envelope.schema.json", &value).expect("the envelope schema");
    value
}

fn observe_as(
    w: &World,
    authority: Option<&SshAuthority>,
    provider: impl SessionCredentialProvider,
) -> Envelope<HostObservationDto> {
    let envelope =
        w.service
            .observe_host(&w.root, authority, &mut SessionCredentials::new(provider));
    let value = scanned(w, &envelope);
    assert_eq!(envelope.command, "host observe");
    // A read of the network is of no request.
    assert_eq!(envelope.request_id, None);
    assert_eq!(envelope.operation_id, None);
    assert_eq!(
        value["data"].is_null(),
        envelope.outcome != Outcome::Success
    );
    if !value["data"].is_null() {
        schema::check_published(HOST_OBSERVATION_SCHEMA, &value["data"])
            .expect("the observation schema");
    }
    envelope
}

fn observe(w: &World) -> Result<HostObservationDto, FixtureError> {
    observe_as(w, None, Provider).data.ok_or(FixtureError)
}

fn prepare(w: &World, mutation: &Mutation) -> Envelope<PrepareDataDto> {
    let envelope = w.service.prepare(mutation);
    let value = scanned(w, &envelope);
    assert_eq!(envelope.request_id, None);
    match &envelope.data {
        Some(PrepareDataDto::Preview(_)) => {
            schema::check_published(HOST_PREVIEW_SCHEMA, &value["data"])
                .expect("the preview schema");
        }
        Some(PrepareDataDto::HostReplacement(_)) => {
            schema::check_published(HOST_REPLACEMENT_SCHEMA, &value["data"])
                .expect("the replacement schema");
        }
        // Of a promotion or a closure, which no case here prepares.
        Some(PrepareDataDto::LifecycleBlocked(_)) => panic!("a host command's prepare"),
        None => {}
    }
    envelope
}

/// The confirmation of a preview of `mutation`, and what it showed.
fn preview(w: &World, mutation: &Mutation) -> Result<(String, HostPreviewDto), FixtureError> {
    let envelope = prepare(w, mutation);
    assert_eq!(envelope.code, ResultCode::Ok, "{}", envelope.message);
    let data = envelope
        .data
        .and_then(PrepareDataDto::into_preview)
        .ok_or(FixtureError)?;
    match data.details {
        PreviewDetailsDto::Host(details) => {
            // The root is the one the result is of, as the library
            // spells it: a platform may spell the fixture's another way.
            assert_eq!(Some(&details.root), envelope.scope.repository.as_ref());
            assert_eq!(data.scope, envelope.scope);
            Ok((data.confirmation_id, details))
        }
        _ => Err(FixtureError),
    }
}

fn confirmed_as(
    w: &World,
    service: &RepositoryService,
    id: &str,
    confirmation: Option<&str>,
    mutation: Mutation,
    provider: impl SessionCredentialProvider,
) -> Envelope<MutationDataDto> {
    let envelope = service.execute(
        MutationCall {
            request_id: request(id),
            confirmation: confirmation.map(|id| ConfirmationId::parse(id).unwrap()),
            mutation,
        },
        &mut SessionCredentials::new(provider),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    let value = scanned(w, &envelope);
    let host = matches!(envelope.command.as_str(), "host approve" | "host replace");
    if host && !value["data"].is_null() {
        schema::check_published(HOST_MUTATION_SCHEMA, &value["data"]).expect("the data schema");
    }
    // No effect shows a pin.
    if host {
        assert_eq!(envelope.effects, Effects::not_requested());
        assert_eq!(envelope.operation_id, None);
    }
    envelope
}

fn confirmed(
    w: &World,
    id: &str,
    confirmation: &str,
    mutation: Mutation,
) -> Envelope<MutationDataDto> {
    confirmed_as(w, &w.service, id, Some(confirmation), mutation, Provider)
}

fn host_data(envelope: &Envelope<MutationDataDto>) -> Result<&HostMutationDto, FixtureError> {
    match envelope.data.as_ref() {
        Some(MutationDataDto::Host(data)) => Ok(data),
        _ => Err(FixtureError),
    }
}

/// Every pin there is, as `(port, fingerprint)`.
fn pins(w: &World) -> Vec<(u16, String)> {
    w.service
        .list_host_pins()
        .expect("the pins")
        .items
        .into_iter()
        .map(|pin| (pin.port, pin.pinned_fingerprint))
        .collect()
}

fn pinned_to(server: &SshRemoteFixture, key: &HostKeyIdentity) -> (u16, String) {
    (server.address().port(), key.sha256.clone())
}

/// Every reference of the repository, tracking references among them,
/// as Git holds it: the files below `refs`, and `HEAD`, `FETCH_HEAD` and
/// `packed-refs` where they are there, byte for byte.
fn references(w: &World) -> Result<BTreeMap<PathBuf, Vec<u8>>, FixtureError> {
    fn walk(directory: &Path, found: &mut BTreeMap<PathBuf, Vec<u8>>) -> Result<(), FixtureError> {
        for entry in fixed(std::fs::read_dir(directory))? {
            let path = fixed(entry)?.path();
            if path.is_dir() {
                walk(&path, found)?;
            } else {
                found.insert(path.clone(), fixed(std::fs::read(&path))?);
            }
        }
        Ok(())
    }
    let git = w.root.join(".git");
    let mut found = BTreeMap::new();
    walk(&git.join("refs"), &mut found)?;
    for name in ["HEAD", "FETCH_HEAD", "packed-refs", "ORIG_HEAD"] {
        let path = git.join(name);
        if path.exists() {
            found.insert(path.clone(), fixed(std::fs::read(&path))?);
        }
    }
    // And as Git reads them, so that nothing is missed that is kept
    // some other way.
    let repo = w.repo()?;
    for reference in fixed(repo.references())? {
        let reference = fixed(reference)?;
        let name = reference.name().ok_or(FixtureError)?.to_owned();
        let target = match reference.target() {
            Some(oid) => oid.to_string(),
            None => reference.symbolic_target().ok_or(FixtureError)?.to_owned(),
        };
        found.insert(PathBuf::from(format!("read:{name}")), target.into_bytes());
    }
    assert!(
        found
            .keys()
            .any(|path| path.to_string_lossy().contains("remotes")),
        "the snapshot holds a tracking reference"
    );
    Ok(found)
}

/// A service on the world's stores whose next request dies at `point`.
fn dying_at(w: &World, point: FailurePoint) -> Result<RepositoryService, FixtureError> {
    fixed(RepositoryService::open_at_with_failure_point_for_testing(
        &w.data(),
        point,
    ))
}

fn observing_a_host_reports_what_it_presents_and_its_pin_and_pins_nothing()
-> Result<(), FixtureError> {
    let w = World::new()?;
    let presented = w.server.host_identity();

    // A host that was never approved: what it presents, and no pin.
    // The observation does connect, and the count of sockets sees it
    // although the client gave the connection up at the host's key and
    // never authenticated.
    let (opened, authenticated) = (sockets(&w), connections(&w));
    let unknown = observe(&w)?;
    assert!(sockets(&w) > opened);
    assert_eq!(connections(&w), authenticated);
    assert_eq!(unknown.host, "127.0.0.1");
    assert_eq!(unknown.port, w.server.address().port());
    assert!(unknown.fetch && unknown.push);
    assert_eq!(unknown.presented_algorithm, presented.algorithm);
    assert_eq!(unknown.presented_fingerprint, presented.sha256);
    assert_eq!(unknown.pinned_algorithm, None);
    assert_eq!(unknown.pinned_fingerprint, None);
    assert!(!unknown.reapproval_required);
    assert_eq!(pins(&w), []);
    // Named by its authority, it is the same host.
    let named = observe_as(&w, Some(&authority(&w.server)), Provider);
    assert_eq!(named.data.as_ref(), Some(&unknown));
    assert!(named.scope.repository.is_some());
    assert_eq!(pins(&w), []);

    // A pinned host: the pin is reported with what it presents.
    approve(&w)?;
    let trusted = observe(&w)?;
    assert_eq!(trusted.presented_fingerprint, presented.sha256);
    assert_eq!(trusted.pinned_algorithm, Some(presented.algorithm.clone()));
    assert_eq!(trusted.pinned_fingerprint, Some(presented.sha256.clone()));
    assert_eq!(pins(&w), [pinned_to(&w.server, &presented)]);

    // A host whose key changed: both, and the pin stays as it was.
    w.server.rotate_host_key()?;
    let rotated = w.server.host_identity();
    let changed = observe(&w)?;
    assert_eq!(changed.presented_fingerprint, rotated.sha256);
    assert_eq!(changed.pinned_fingerprint, Some(presented.sha256.clone()));
    assert_eq!(pins(&w), [pinned_to(&w.server, &presented)]);

    // A host the publication remote does not name is not connected to.
    let elsewhere = SshAuthority {
        host: "127.0.0.1".into(),
        port: 9,
    };
    let opened = sockets(&w);
    let refused = observe_as(&w, Some(&elsewhere), Provider);
    assert_eq!(refused.code, ResultCode::InvalidInput);
    assert!(refused.data.is_none());
    assert_eq!(sockets(&w), opened);

    // With no key selected nothing can authenticate.
    fixed(w.service.clear_shared_key_selection())?;
    let keyless = observe_as(&w, None, Provider);
    assert_eq!(keyless.code, ResultCode::NoSelectedKey);
    assert_eq!(keyless.outcome, Outcome::Blocked);
    assert!(keyless.data.is_none());
    assert_eq!(pins(&w), [pinned_to(&w.server, &presented)]);
    Ok(())
}

fn approving_the_observed_fingerprint_stores_the_pin_and_publishes_nothing()
-> Result<(), FixtureError> {
    let w = World::new()?;
    // Something a publication would push, were anything published.
    advance(&w.repo()?, "refs/heads/main", "local-advance")?;
    let observed = observe(&w)?;
    let key = HostKeyIdentity {
        algorithm: observed.presented_algorithm.clone(),
        sha256: observed.presented_fingerprint.clone(),
    };
    let approve = approve_of(&w, &w.server, &key);
    let before = references(&w)?;
    let served = local_branches(&w.bare()?)?;
    let pushes = accepted(&w.server);
    let updates = w.server.receive_updates().len();

    // The preview is made without a connection.
    let connected = connections(&w);
    let opened = sockets(&w);
    let (confirmation, shown) = preview(&w, &approve)?;
    assert_eq!(connections(&w), connected);
    assert_eq!(sockets(&w), opened);
    assert_eq!(shown.host, "127.0.0.1");
    assert_eq!(shown.port, w.server.address().port());
    assert!(shown.fetch && shown.push);
    assert_eq!(shown.algorithm, key.algorithm);
    assert_eq!(shown.fingerprint, key.sha256);
    assert_eq!(shown.pinned_fingerprint, None);
    assert_eq!(shown.pinned_algorithm, None);
    assert_eq!(pins(&w), []);

    let envelope = confirmed(&w, REQUEST_1, &confirmation, approve);
    assert_eq!(envelope.code, ResultCode::Ok, "{}", envelope.message);
    assert_eq!(envelope.outcome, Outcome::Success);
    assert_eq!(actions(&envelope), []);
    let data = host_data(&envelope)?;
    assert_eq!(data.host, "127.0.0.1");
    assert_eq!(data.port, w.server.address().port());
    assert_eq!(data.fingerprint, key.sha256);
    assert_eq!(data.presented_fingerprint, None);
    // It connected once, to see the key, and stored the pin.
    assert_eq!(connections(&w), connected + 1);
    assert_eq!(pins(&w), [pinned_to(&w.server, &key)]);
    // Nothing was published, and no reference moved, here or there.
    assert_eq!(accepted(&w.server), pushes);
    assert_eq!(w.server.receive_updates().len(), updates);
    assert_eq!(references(&w)?, before);
    assert_eq!(local_branches(&w.bare()?)?, served);
    assert_eq!(
        record(&w, REQUEST_1).map(|record| record.state),
        Some(RequestState::Finished)
    );

    // A preview of a host that is trusted connects no more than one of
    // a host that is not: a replacement by a key the host does not
    // present is previewed all the same.
    let connected = connections(&w);
    let opened = sockets(&w);
    preview(&w, &replacement(&w, &key.sha256, &another_key()?))?;
    assert_eq!(connections(&w), connected);
    assert_eq!(sockets(&w), opened);
    Ok(())
}

fn a_synchronization_stopped_for_its_host_goes_on_once_the_offered_approval_is_made()
-> Result<(), FixtureError> {
    let w = World::new()?;
    let local = advance(&w.repo()?, "refs/heads/main", "local-advance")?;
    let stopped = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(stopped.code, ResultCode::HostApprovalRequired);
    assert_host_action(&w, &stopped, RecoveryActionKind::HostApprove)?;
    // What the action and the data give is what the command takes.
    let offered = &stopped.recovery[0];
    let named = offered.arguments["authority"]
        .as_str()
        .ok_or(FixtureError)?;
    let (host, port) = named.rsplit_once(':').ok_or(FixtureError)?;
    let data = synchronization_data(&stopped)?;
    let approve = Mutation::HostApprove(HostApproveInput {
        root: PathBuf::from(offered.arguments["root"].as_str().ok_or(FixtureError)?),
        authority: SshAuthority {
            host: host.to_owned(),
            port: fixed(port.parse())?,
        },
        algorithm: data.presented_algorithm.clone(),
        fingerprint: data.presented_fingerprint.clone(),
    });
    let (confirmation, _) = preview(&w, &approve)?;
    let pushes = accepted(&w.server);
    let approved = confirmed(&w, REQUEST_2, &confirmation, approve);
    assert_eq!(approved.outcome, Outcome::Success, "{}", approved.message);
    // The approval pushed nothing; the same synchronization then does.
    assert_eq!(accepted(&w.server), pushes);
    let retried = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(retried.effects, published(local));
    assert_eq!(retried.operation_id, stopped.operation_id);
    assert_eq!(accepted(&w.server), pushes + 1);

    // And a replacement, from a synchronization stopped for a host
    // whose key changed.
    let pinned = w.server.host_identity();
    w.server.rotate_host_key()?;
    advance(&w.repo()?, "refs/heads/main", "local-advance-2")?;
    let stopped = execute(&w, REQUEST_3, repo_sync(&w));
    assert_eq!(stopped.code, ResultCode::HostReplacementRequired);
    assert_host_action(&w, &stopped, RecoveryActionKind::HostReplace)?;
    let data = synchronization_data(&stopped)?;
    let replace = replacement(
        &w,
        data.pinned_fingerprint.as_deref().ok_or(FixtureError)?,
        &HostKeyIdentity {
            algorithm: data.presented_algorithm.clone(),
            sha256: data.presented_fingerprint.clone(),
        },
    );
    assert_eq!(data.pinned_fingerprint, Some(pinned.sha256));
    let (confirmation, _) = preview(&w, &replace)?;
    let replaced = confirmed(&w, REQUEST_4, &confirmation, replace);
    assert_eq!(replaced.outcome, Outcome::Success, "{}", replaced.message);
    let retried = execute(&w, REQUEST_3, repo_sync(&w));
    assert_eq!(retried.outcome, Outcome::Success, "{}", retried.message);
    Ok(())
}

fn a_fingerprint_the_host_does_not_present_is_host_mismatch_and_pins_nothing()
-> Result<(), FixtureError> {
    let w = World::new()?;
    let presented = w.server.host_identity();
    let wrong = another_key()?;
    let approve = approve_of(&w, &w.server, &wrong);
    // The preview is local: it cannot know what the host presents.
    let (confirmation, _) = preview(&w, &approve)?;
    let before = references(&w)?;
    let pushes = accepted(&w.server);

    let envelope = confirmed(&w, REQUEST_1, &confirmation, approve);
    assert_eq!(envelope.code, ResultCode::HostMismatch);
    assert_golden(
        &w,
        "host_approve_host_mismatch",
        Some(HOST_SCHEMA),
        &envelope,
    );
    assert_eq!(envelope.outcome, Outcome::Blocked);
    let data = host_data(&envelope)?;
    assert_eq!(data.fingerprint, wrong.sha256);
    assert_eq!(data.presented_algorithm, Some(presented.algorithm.clone()));
    assert_eq!(data.presented_fingerprint, Some(presented.sha256.clone()));
    // Nothing was pinned, nothing is left of the request, and nothing
    // is offered: the same request never gets past it.
    assert_eq!(pins(&w), []);
    assert_eq!(actions(&envelope), []);
    assert!(record(&w, REQUEST_1).is_none());
    assert_eq!(references(&w)?, before);
    assert_eq!(accepted(&w.server), pushes);

    // So it is for a replacement by a key the host does not present.
    approve_by_hand(&w)?;
    let replace = replacement(&w, &presented.sha256, &wrong);
    let (confirmation, _) = preview(&w, &replace)?;
    let envelope = confirmed(&w, REQUEST_2, &confirmation, replace);
    assert_eq!(envelope.code, ResultCode::HostMismatch);
    assert_eq!(
        host_data(&envelope)?.presented_fingerprint,
        Some(presented.sha256.clone())
    );
    assert_eq!(pins(&w), [pinned_to(&w.server, &presented)]);
    Ok(())
}

/// Pins the key the world's server presents through the pin store's
/// own code, with no connection: what another actor's approval leaves.
fn approve_by_hand(w: &World) -> Result<(), FixtureError> {
    fixed(
        w.service
            .approve_host_pin_for_testing(&authority(&w.server), &w.server.host_identity()),
    )
}

fn a_rotated_host_key_is_replaced_by_naming_the_old_and_the_new_fingerprint()
-> Result<(), FixtureError> {
    let w = World::new()?;
    let pushes = accepted(&w.server);
    let old = w.server.host_identity();
    let (confirmation, _) = preview(&w, &approval(&w))?;
    assert_eq!(
        confirmed(&w, REQUEST_1, &confirmation, approval(&w)).outcome,
        Outcome::Success
    );
    w.server.rotate_host_key()?;
    let new = w.server.host_identity();
    assert!(old != new);

    // An approval is of a host that has no pin: this one is replaced.
    // The refusal says what the replacement is formed from: the pin
    // there is and the key the request named. Nothing was connected to,
    // so it says nothing of what the host presents.
    let connected = connections(&w);
    let approve = prepare(&w, &approval(&w));
    assert_eq!(approve.code, ResultCode::HostReplacementRequired);
    assert_eq!(connections(&w), connected);
    assert_eq!(
        approve.data,
        Some(PrepareDataDto::HostReplacement(HostReplacementDto {
            host: authority(&w.server).host,
            port: authority(&w.server).port,
            algorithm: new.algorithm.clone(),
            fingerprint: new.sha256.clone(),
            pinned_algorithm: old.algorithm.clone(),
            pinned_fingerprint: old.sha256.clone(),
        }))
    );
    assert_eq!(approve.recovery.len(), 1);
    assert_eq!(approve.recovery[0].action.as_str(), "host.replace");
    assert_eq!(
        approve.recovery[0].arguments["authority"],
        format!("127.0.0.1:{}", authority(&w.server).port)
    );
    assert_eq!(
        approve.recovery[0].arguments["root"].as_str(),
        approve.scope.repository.as_deref()
    );
    // The old fingerprint is to be the pin's.
    for wrong in [new.sha256.clone(), another_key()?.sha256] {
        let refused = prepare(&w, &replacement(&w, &wrong, &new));
        assert_eq!(refused.code, ResultCode::InvalidInput, "{wrong}");
        assert!(refused.data.is_none());
    }
    // And a fingerprint, where one is asked for.
    let malformed = prepare(&w, &replacement(&w, "SHA256:not-a-digest", &new));
    assert_eq!(malformed.code, ResultCode::InvalidInput);
    assert_eq!(pins(&w), [pinned_to(&w.server, &old)]);

    let replace = replacement(&w, &old.sha256, &new);
    let connected = connections(&w);
    let opened = sockets(&w);
    let (confirmation, shown) = preview(&w, &replace)?;
    assert_eq!(connections(&w), connected);
    assert_eq!(sockets(&w), opened);
    assert_eq!(shown.pinned_fingerprint, Some(old.sha256.clone()));
    assert_eq!(shown.pinned_algorithm, Some(old.algorithm.clone()));
    assert_eq!(shown.fingerprint, new.sha256);
    let before = references(&w)?;
    let envelope = confirmed(&w, REQUEST_2, &confirmation, replace);
    assert_eq!(envelope.code, ResultCode::Ok, "{}", envelope.message);
    assert_eq!(envelope.command, "host replace");
    assert_eq!(host_data(&envelope)?.fingerprint, new.sha256);
    // The pin changed: there is one, and it is the new key.
    assert_eq!(pins(&w), [pinned_to(&w.server, &new)]);
    assert_eq!(references(&w)?, before);
    assert_eq!(accepted(&w.server), pushes);
    Ok(())
}

fn a_host_the_publication_remote_does_not_name_is_invalid_input() -> Result<(), FixtureError> {
    let w = World::new()?;
    let key = w.server.host_identity();
    let elsewhere = |host: &str, port: u16| {
        Mutation::HostApprove(HostApproveInput {
            root: w.root.clone(),
            authority: SshAuthority {
                host: host.to_owned(),
                port,
            },
            algorithm: key.algorithm.clone(),
            fingerprint: key.sha256.clone(),
        })
    };
    let connected = connections(&w);
    for (host, port) in [
        ("127.0.0.1", 9),
        ("example.invalid", w.server.address().port()),
        ("not a host", 22),
    ] {
        let refused = prepare(&w, &elsewhere(host, port));
        assert_eq!(refused.code, ResultCode::InvalidInput, "{host}:{port}");
        assert_eq!(refused.outcome, Outcome::Error);
        assert!(refused.data.is_none());
    }
    // Nor is a key that is none, or a replacement of a pin that is not
    // there.
    let unkeyed = Mutation::HostApprove(HostApproveInput {
        root: w.root.clone(),
        authority: authority(&w.server),
        algorithm: "ssh-unheard-of".into(),
        fingerprint: key.sha256.clone(),
    });
    assert_eq!(prepare(&w, &unkeyed).code, ResultCode::InvalidInput);
    let unpinned = prepare(&w, &replacement(&w, &another_key()?.sha256, &key));
    assert_eq!(unpinned.code, ResultCode::InvalidInput);
    assert_eq!(connections(&w), connected);
    assert_eq!(pins(&w), []);
    // The host's name is taken as a pin stores it: in lower case.
    fixed(
        w.repo()?
            .remote_set_url("origin", &w.server.url().replace("127.0.0.1", "LOCALHOST")),
    )?;
    let shouted = Mutation::HostApprove(HostApproveInput {
        root: w.root.clone(),
        authority: SshAuthority {
            host: "LocalHost".into(),
            port: w.server.address().port(),
        },
        algorithm: key.algorithm.clone(),
        fingerprint: key.sha256.clone(),
    });
    let (_, shown) = preview(&w, &shouted)?;
    assert_eq!(shown.host, "localhost");
    Ok(())
}

fn the_fetch_host_and_the_push_host_are_each_approved_in_their_own_direction()
-> Result<(), FixtureError> {
    let mut w = World::new()?;
    // A second server, which the remote is pushed to. Neither is
    // trusted.
    let destination = SshRemoteFixture::start()?;
    destination.allow_client_public_key(fixed(russh::keys::PublicKey::from_bytes(
        &w.server.allowed_client_public_key(),
    ))?);
    w.probes.push(destination.url().into_bytes());
    ssh_privacy::save(&w.probes)?;
    fixed(
        w.repo()?
            .remote_set_pushurl("origin", Some(&destination.url())),
    )?;
    let before = references(&w)?;
    let pushes = accepted(&w.server);
    let fetch_side = connections(&w);

    // The host that is pushed to: the connection goes to it, and to it
    // only.
    let pushing = observe_as(&w, Some(&authority(&destination)), Provider)
        .data
        .ok_or(FixtureError)?;
    assert!(pushing.push && !pushing.fetch);
    assert_eq!(
        pushing.presented_fingerprint,
        destination.host_identity().sha256
    );
    let approve = approve_of(&w, &destination, &destination.host_identity());
    let (confirmation, shown) = preview(&w, &approve)?;
    assert!(shown.push && !shown.fetch);
    let pushed = confirmed(&w, REQUEST_1, &confirmation, approve);
    assert_eq!(pushed.outcome, Outcome::Success, "{}", pushed.message);
    assert_eq!(destination.accepted_keys().len(), 1);
    assert_eq!(connections(&w), fetch_side);
    assert_eq!(
        pins(&w),
        [pinned_to(&destination, &destination.host_identity())]
    );
    // It asked for what a push asks for, and pushed nothing.
    assert!(!destination.commands().is_empty());
    assert!(
        destination
            .commands()
            .iter()
            .all(|command| command.starts_with(b"git-receive-pack"))
    );
    assert!(destination.receive_updates().is_empty());

    // The host that is fetched from, which is what is observed when no
    // host is named.
    let fetching = observe(&w)?;
    assert!(fetching.fetch && !fetching.push);
    assert_eq!(fetching.port, w.server.address().port());
    let commands = w.server.commands().len();
    let (confirmation, shown) = preview(&w, &approval(&w))?;
    assert!(shown.fetch && !shown.push);
    let fetched = confirmed(&w, REQUEST_2, &confirmation, approval(&w));
    assert_eq!(fetched.outcome, Outcome::Success, "{}", fetched.message);
    assert_eq!(destination.accepted_keys().len(), 1);
    assert_eq!(connections(&w), fetch_side + 1);
    assert!(w.server.commands().len() > commands);
    assert!(
        w.server.commands()[commands..]
            .iter()
            .all(|command| command.starts_with(b"git-upload-pack"))
    );
    let mut both = vec![
        pinned_to(&w.server, &w.server.host_identity()),
        pinned_to(&destination, &destination.host_identity()),
    ];
    both.sort();
    let mut held = pins(&w);
    held.sort();
    assert_eq!(held, both);
    assert_eq!(references(&w)?, before);
    assert_eq!(accepted(&w.server), pushes);
    Ok(())
}

fn a_pin_changed_after_the_preview_is_an_external_change_before_any_connection()
-> Result<(), FixtureError> {
    let w = World::new()?;
    let other = another_key()?;
    let connected = connections(&w);
    let commands = w.server.commands().len();

    // Between the preview and the request: the boundary's comparison.
    let (confirmation, _) = preview(&w, &approval(&w))?;
    fixed(
        w.service
            .approve_host_pin_for_testing(&authority(&w.server), &other),
    )?;
    let stale = confirmed(&w, REQUEST_1, &confirmation, approval(&w));
    assert_eq!(stale.code, ResultCode::ExternalChange);
    assert_eq!(stale.outcome, Outcome::Blocked);
    assert_eq!(actions(&stale), [RecoveryActionKind::RequestPrepare]);
    assert!(record(&w, REQUEST_1).is_none());
    assert_eq!(pins(&w), [pinned_to(&w.server, &other)]);

    // Between a request's acceptance and its retry: the binding's own.
    // The request was accepted on a preview of the pin as it now is,
    // and died before it connected.
    let replace = replacement(&w, &other.sha256, &w.server.host_identity());
    let (confirmation, _) = preview(&w, &replace)?;
    let dying = dying_at(&w, FailurePoint::BeforeRequestDomainCall)?;
    let died = confirmed_as(
        &w,
        &dying,
        REQUEST_2,
        Some(&confirmation),
        replacement(&w, &other.sha256, &w.server.host_identity()),
        Provider,
    );
    assert_eq!(died.code, ResultCode::InternalError);
    assert_eq!(
        record(&w, REQUEST_2).map(|record| record.state),
        Some(RequestState::Accepted)
    );
    let third = another_key()?;
    fixed(
        w.service
            .approve_host_pin_for_testing(&authority(&w.server), &third),
    )?;
    let retried = confirmed(&w, REQUEST_2, &confirmation, replace);
    assert_eq!(retried.code, ResultCode::ExternalChange);
    assert_eq!(actions(&retried), [RecoveryActionKind::RequestPrepare]);
    // The record is released, and the other actor's pin stands.
    assert!(record(&w, REQUEST_2).is_none());
    assert_eq!(pins(&w), [pinned_to(&w.server, &third)]);

    // Neither opened a connection: no client authenticated, and the
    // server was asked for nothing.
    assert_eq!(connections(&w), connected);
    assert_eq!(w.server.commands().len(), commands);
    Ok(())
}

fn a_pin_changed_between_the_observation_and_the_connection_is_not_written_over()
-> Result<(), FixtureError> {
    let w = World::new()?;
    let other = another_key()?;
    let (confirmation, _) = preview(&w, &approval(&w))?;
    // After the request observed the pin and was accepted, and before
    // the verification reads it.
    let elsewhere = fixed(RepositoryService::open_at(&w.data()))?;
    let (host, key) = (authority(&w.server), other.clone());
    w.service.set_request_hook_for_testing(move || {
        elsewhere
            .approve_host_pin_for_testing(&host, &key)
            .expect("another actor's pin");
    });
    let envelope = confirmed(&w, REQUEST_1, &confirmation, approval(&w));
    assert_eq!(envelope.code, ResultCode::ExternalChange);
    assert_eq!(actions(&envelope), [RecoveryActionKind::RequestPrepare]);
    assert!(record(&w, REQUEST_1).is_none());
    assert_eq!(pins(&w), [pinned_to(&w.server, &other)]);
    Ok(())
}

/// The remote is given another location after the request observed it
/// and before the verification reads it: a host that is pinned and
/// presents its pin. The verification authenticates there and pins
/// nothing. The host the request names was never connected to, so
/// nothing is said of what it presented.
fn a_remote_repointed_at_another_pinned_host_is_an_external_change_naming_no_key()
-> Result<(), FixtureError> {
    let mut w = World::new()?;
    let other = SshRemoteFixture::start()?;
    other.allow_client_public_key(fixed(russh::keys::PublicKey::from_bytes(
        &w.server.allowed_client_public_key(),
    ))?);
    w.probes.push(other.url().into_bytes());
    ssh_privacy::save(&w.probes)?;
    let pinned = w.server.host_identity();
    approve_by_hand(&w)?;
    fixed(
        w.service
            .approve_host_pin_for_testing(&authority(&other), &other.host_identity()),
    )?;
    let mut held = pins(&w);
    held.sort();
    let replace = replacement(&w, &pinned.sha256, &another_key()?);
    let (confirmation, _) = preview(&w, &replace)?;

    let (root, location) = (w.root.clone(), other.url());
    w.service.set_request_hook_for_testing(move || {
        git2::Repository::open(&root)
            .and_then(|repository| repository.remote_set_url("origin", &location))
            .expect("the remote, pointed elsewhere");
    });
    let (here, there) = (sockets(&w), other.accepted_sockets());
    let envelope = confirmed(&w, REQUEST_1, &confirmation, replace);
    // It went to the other host, and not to the one the request names.
    assert!(other.accepted_sockets() > there);
    assert_eq!(sockets(&w), here);
    assert_eq!(envelope.code, ResultCode::ExternalChange);
    assert_eq!(envelope.outcome, Outcome::Blocked);
    assert!(envelope.data.is_none());
    assert_eq!(actions(&envelope), [RecoveryActionKind::RequestPrepare]);
    assert!(record(&w, REQUEST_1).is_none());
    // Neither host's pin was written or changed.
    let mut after = pins(&w);
    after.sort();
    assert_eq!(after, held);
    Ok(())
}

/// The pin is stored, and the call fails all the same: the remote is
/// given another location once the host's trust is finalized, which
/// the verification finds before it returns. The answer does not say
/// that nothing changed.
fn a_call_that_fails_after_the_pin_was_stored_says_so_and_its_retry_is_a_no_op()
-> Result<(), FixtureError> {
    use repository::transport::operation_tests::{Checkpoint, install_hook};
    let w = World::new()?;
    let key = w.server.host_identity();
    let (confirmation, _) = preview(&w, &approval(&w))?;
    let root = w.root.clone();
    let mut once = true;
    let hook = install_hook(move |point| {
        if point == Checkpoint::HostTrustFinalized && std::mem::take(&mut once) {
            git2::Repository::open(&root)
                .and_then(|repository| {
                    repository.remote_set_url("origin", "ssh://fixture@127.0.0.1:9/elsewhere.git")
                })
                .expect("the remote, pointed elsewhere");
        }
    });
    let failed = confirmed(&w, REQUEST_1, &confirmation, approval(&w));
    drop(hook);
    assert_eq!(pins(&w), [pinned_to(&w.server, &key)]);
    // What stopped it is its code; that it left a pin is its outcome
    // and its data.
    assert_eq!(failed.code, ResultCode::ExternalChange);
    assert_eq!(failed.outcome, Outcome::Partial);
    let data = host_data(&failed)?;
    assert_eq!(data.fingerprint, key.sha256);
    assert_eq!(data.presented_fingerprint, None);
    // The request is not over: the same request is what ends it.
    assert_eq!(actions(&failed), [RecoveryActionKind::RequestRetry]);
    assert_eq!(
        record(&w, REQUEST_1).map(|record| record.state),
        Some(RequestState::Accepted)
    );

    // The request ID with another input, while its record is accepted
    // and the pin is stored: a mismatch that does not say nothing was
    // changed. The request has no operation, so no row says an attempt
    // ran; the pin store, which no longer shows what was previewed, does.
    let changed = confirmed(
        &w,
        REQUEST_1,
        &confirmation,
        approve_of(&w, &w.server, &another_key()?),
    );
    assert_eq!(changed.code, ResultCode::RequestMismatch);
    assert_eq!(changed.outcome, Outcome::Partial);
    assert_eq!(pins(&w), [pinned_to(&w.server, &key)]);

    let (connected, opened) = (connections(&w), sockets(&w));
    let retried = confirmed(&w, REQUEST_1, &confirmation, approval(&w));
    assert_eq!(retried.code, ResultCode::AlreadyApplied);
    assert_eq!(retried.outcome, Outcome::Noop);
    assert_eq!(host_data(&retried)?.fingerprint, key.sha256);
    assert_eq!(
        record(&w, REQUEST_1).map(|record| record.state),
        Some(RequestState::Finished)
    );
    assert_eq!((connections(&w), sockets(&w)), (connected, opened));
    assert_eq!(pins(&w), [pinned_to(&w.server, &key)]);
    Ok(())
}

fn an_approval_with_a_locked_key_and_no_passphrase_is_unlock_required_and_pins_nothing()
-> Result<(), FixtureError> {
    let w = World::new()?;
    let (confirmation, _) = preview(&w, &approval(&w))?;
    let locked = confirmed_as(
        &w,
        &w.service,
        REQUEST_1,
        Some(&confirmation),
        approval(&w),
        NoPassphrase,
    );
    assert_eq!(locked.code, ResultCode::UnlockRequired);
    assert_eq!(locked.outcome, Outcome::Blocked);
    assert!(locked.data.is_none());
    assert_eq!(pins(&w), []);
    // Nothing is left of the request: its confirmation is free, and
    // the same request goes on once a passphrase is to be had.
    assert!(record(&w, REQUEST_1).is_none());
    let unlocked = confirmed(&w, REQUEST_1, &confirmation, approval(&w));
    assert_eq!(unlocked.outcome, Outcome::Success, "{}", unlocked.message);
    assert_eq!(pins(&w), [pinned_to(&w.server, &w.server.host_identity())]);

    // The observation of a trusted host needs the key as well.
    let unobserved = observe_as(&w, None, NoPassphrase);
    assert_eq!(unobserved.code, ResultCode::UnlockRequired);
    assert!(unobserved.data.is_none());
    Ok(())
}

fn approving_a_host_already_pinned_to_the_key_is_a_no_op() -> Result<(), FixtureError> {
    let w = World::new()?;
    let key = w.server.host_identity();
    // A preview taken while the host has no pin.
    let (confirmation, _) = preview(&w, &approval(&w))?;
    approve_by_hand(&w)?;
    let connected = connections(&w);

    // No preview is given of a host that is pinned to the key.
    let pinned = prepare(&w, &approval(&w));
    assert_eq!(pinned.code, ResultCode::AlreadyApplied);
    assert_eq!(pinned.outcome, Outcome::Noop);
    assert!(pinned.data.is_none());
    // And a request that finds it so is a no-op, not a stale preview.
    let envelope = confirmed(&w, REQUEST_1, &confirmation, approval(&w));
    assert_eq!(envelope.code, ResultCode::AlreadyApplied);
    assert_eq!(envelope.outcome, Outcome::Noop);
    assert_eq!(host_data(&envelope)?.fingerprint, key.sha256);
    assert!(record(&w, REQUEST_1).is_none());
    assert_eq!(connections(&w), connected);
    assert_eq!(pins(&w), [pinned_to(&w.server, &key)]);
    Ok(())
}

fn an_approval_under_the_reapproval_marker_stores_the_pin_and_leaves_the_marker()
-> Result<(), FixtureError> {
    let w = World::new()?;
    fixed(w.service.require_host_reapproval_for_testing())?;
    let marker = w.data().join("ssh-host-trust-reapproval-required");
    let bytes = fixed(std::fs::read(&marker))?;
    assert!(fixed(w.service.list_host_pins())?.reapproval_required);
    assert!(observe(&w)?.reapproval_required);

    let (confirmation, shown) = preview(&w, &approval(&w))?;
    assert!(shown.reapproval_required);
    let envelope = confirmed(&w, REQUEST_1, &confirmation, approval(&w));
    assert_eq!(envelope.outcome, Outcome::Success, "{}", envelope.message);
    assert_eq!(pins(&w), [pinned_to(&w.server, &w.server.host_identity())]);
    // The marker is as it was.
    assert_eq!(fixed(std::fs::read(&marker))?, bytes);
    assert!(fixed(w.service.list_host_pins())?.reapproval_required);
    Ok(())
}

fn a_connection_dropped_before_the_advertisement_pins_nothing_and_its_retry_pins_once()
-> Result<(), FixtureError> {
    let w = World::new()?;
    let key = w.server.host_identity();
    let (confirmation, _) = preview(&w, &approval(&w))?;
    let before = references(&w)?;
    let pushes = accepted(&w.server);

    // The client authenticated, and the connection went before the
    // server said what it holds.
    w.server.disconnect_at(FixtureBoundary::Advertisement);
    let connected = connections(&w);
    let dropped = confirmed(&w, REQUEST_1, &confirmation, approval(&w));
    assert_eq!(connections(&w), connected + 1);
    // Pinned: the verification pins only once the host has answered
    // for its repository, so nothing was stored and nothing is left of
    // the request.
    assert_eq!(dropped.code, ResultCode::TransportUnavailable);
    assert_eq!(dropped.outcome, Outcome::Error);
    assert_eq!(actions(&dropped), [RecoveryActionKind::RequestRetry]);
    assert!(dropped.data.is_none());
    assert_eq!(pins(&w), []);
    assert!(record(&w, REQUEST_1).is_none());

    // The same request, once the host can be reached.
    w.server.clear_fault();
    let retried = confirmed(&w, REQUEST_1, &confirmation, approval(&w));
    assert_eq!(retried.outcome, Outcome::Success, "{}", retried.message);
    assert_eq!(pins(&w), [pinned_to(&w.server, &key)]);
    // And once more: a replay, and still one pin.
    let again = confirmed(&w, REQUEST_1, &confirmation, approval(&w));
    assert_eq!(again.outcome, Outcome::Success);
    assert_eq!(pins(&w), [pinned_to(&w.server, &key)]);
    assert_eq!(references(&w)?, before);
    assert_eq!(accepted(&w.server), pushes);

    // A retry that fails keeps its record: it is the request's, and
    // the same request is what goes on.
    w.server.rotate_host_key()?;
    let new = w.server.host_identity();
    let replace = || replacement(&w, &key.sha256, &new);
    let (confirmation, _) = preview(&w, &replace())?;
    let dying = dying_at(&w, FailurePoint::BeforeRequestDomainCall)?;
    confirmed_as(
        &w,
        &dying,
        REQUEST_2,
        Some(&confirmation),
        replace(),
        Provider,
    );
    w.server.disconnect_at(FixtureBoundary::Advertisement);
    let failed = confirmed(&w, REQUEST_2, &confirmation, replace());
    assert_eq!(failed.code, ResultCode::TransportUnavailable);
    assert_eq!(actions(&failed), [RecoveryActionKind::RequestRetry]);
    assert_eq!(
        record(&w, REQUEST_2).map(|record| record.state),
        Some(RequestState::Accepted)
    );
    assert_eq!(pins(&w), [pinned_to(&w.server, &key)]);
    w.server.clear_fault();
    let finished = confirmed(&w, REQUEST_2, &confirmation, replace());
    assert_eq!(finished.outcome, Outcome::Success, "{}", finished.message);
    assert_eq!(pins(&w), [pinned_to(&w.server, &new)]);
    Ok(())
}

fn a_finished_approval_is_replayed_without_a_connection_and_a_changed_input_is_a_mismatch()
-> Result<(), FixtureError> {
    let w = World::new()?;
    let key = w.server.host_identity();
    let (confirmation, _) = preview(&w, &approval(&w))?;
    let first = confirmed(&w, REQUEST_1, &confirmation, approval(&w));
    assert_eq!(first.outcome, Outcome::Success, "{}", first.message);
    let stored = record(&w, REQUEST_1).map(|record| format!("{record:?}"));

    // Whatever has happened to the host since: the stored result, with
    // no connection and no pin touched.
    w.server.rotate_host_key()?;
    w.server.disconnect_at(FixtureBoundary::Handshake);
    let connected = connections(&w);
    let commands = w.server.commands().len();
    for presented in [Some(confirmation.as_str()), None] {
        let again = confirmed_as(
            &w,
            &w.service,
            REQUEST_1,
            presented,
            approve_of(&w, &w.server, &key),
            Provider,
        );
        assert_eq!(
            fixed(serde_json::to_string(&again))?,
            fixed(serde_json::to_string(&first))?
        );
    }
    assert_eq!(connections(&w), connected);
    assert_eq!(w.server.commands().len(), commands);

    // Another key, or another host, under the same request ID.
    let other = another_key()?;
    let changed = confirmed(
        &w,
        REQUEST_1,
        &confirmation,
        approve_of(&w, &w.server, &other),
    );
    assert_eq!(changed.code, ResultCode::RequestMismatch);
    assert_eq!(changed.outcome, Outcome::Error);
    let elsewhere = Mutation::HostApprove(HostApproveInput {
        root: w.root.clone(),
        authority: SshAuthority {
            host: "127.0.0.1".into(),
            port: 9,
        },
        algorithm: key.algorithm.clone(),
        fingerprint: key.sha256.clone(),
    });
    assert_eq!(
        confirmed(&w, REQUEST_1, &confirmation, elsewhere).code,
        ResultCode::RequestMismatch
    );
    // And a replacement is another command.
    assert_eq!(
        confirmed(
            &w,
            REQUEST_1,
            &confirmation,
            replacement(&w, &other.sha256, &key)
        )
        .code,
        ResultCode::RequestMismatch
    );
    assert_eq!(
        record(&w, REQUEST_1).map(|record| format!("{record:?}")),
        stored
    );
    assert_eq!(pins(&w), [pinned_to(&w.server, &key)]);
    assert_eq!(connections(&w), connected);
    Ok(())
}

fn an_approval_whose_result_was_lost_after_the_pin_was_stored_is_a_no_op_on_retry()
-> Result<(), FixtureError> {
    let w = World::new()?;
    let key = w.server.host_identity();
    let (confirmation, _) = preview(&w, &approval(&w))?;
    // The call stored the pin and died before it recorded its end.
    let dying = dying_at(&w, FailurePoint::BeforeRequestSettlement)?;
    let lost = confirmed_as(
        &w,
        &dying,
        REQUEST_1,
        Some(&confirmation),
        approval(&w),
        Provider,
    );
    assert_eq!(lost.code, ResultCode::InternalError);
    assert_eq!(pins(&w), [pinned_to(&w.server, &key)]);
    assert_eq!(
        record(&w, REQUEST_1).map(|record| record.state),
        Some(RequestState::Accepted)
    );

    // The retry finds its end state: a no-op, not a stale preview, with
    // no connection and still one pin.
    let connected = connections(&w);
    let retried = confirmed(&w, REQUEST_1, &confirmation, approval(&w));
    assert_eq!(retried.code, ResultCode::AlreadyApplied);
    assert_eq!(retried.outcome, Outcome::Noop);
    assert_eq!(host_data(&retried)?.fingerprint, key.sha256);
    assert_eq!(connections(&w), connected);
    assert_eq!(pins(&w), [pinned_to(&w.server, &key)]);
    assert_eq!(
        record(&w, REQUEST_1).map(|record| record.state),
        Some(RequestState::Finished)
    );
    // And that is what the request is answered with from then on.
    let replayed = confirmed(&w, REQUEST_1, &confirmation, approval(&w));
    assert_eq!(replayed.code, ResultCode::AlreadyApplied);
    assert_eq!(connections(&w), connected);
    Ok(())
}

/// An observation, a preview and an approval, with everything each
/// returned written to the case's output, and then, when `private`, the
/// path the selected key was read from.
fn control_prints(private: bool) -> Result<(), FixtureError> {
    let w = World::new()?;
    let observed = observe_as(&w, None, Provider);
    let (confirmation, _) = preview(&w, &approval(&w))?;
    let shown = prepare(&w, &approval(&w));
    let approved = confirmed(&w, REQUEST_1, &confirmation, approval(&w));
    assert_eq!(approved.outcome, Outcome::Success);
    let wrong = another_key()?;
    let replace = replacement(&w, &w.server.host_identity().sha256, &wrong);
    let (confirmation, _) = preview(&w, &replace)?;
    let mismatched = confirmed(&w, REQUEST_2, &confirmation, replace);
    assert_eq!(mismatched.code, ResultCode::HostMismatch);
    println!("{}", fixed(serde_json::to_string(&observed))?);
    eprintln!("{observed:?}");
    println!("{}", fixed(serde_json::to_string(&shown))?);
    eprintln!("{shown:?}");
    for envelope in [&approved, &mismatched] {
        println!("{}", fixed(serde_json::to_string(envelope))?);
        eprintln!("{envelope:?}");
    }
    for id in [REQUEST_1, REQUEST_2] {
        let shown = w.service.show_request(request(id));
        println!("{}", fixed(serde_json::to_string(&shown))?);
    }
    if private {
        println!("{}", String::from_utf8_lossy(&key_path(&w)));
    }
    Ok(())
}

fn the_output_privacy_scan_passes_what_a_host_approval_emits_and_fails_the_key_path()
-> Result<(), FixtureError> {
    ssh_privacy::save(&[b"runner-private-capture-control".to_vec()])?;
    assert_eq!(
        ssh_harness::run_isolated_with_output_privacy("control_prints_what_a_host_approval_emits"),
        Ok(())
    );
    assert_eq!(
        ssh_harness::run_isolated_with_output_privacy(
            "control_prints_the_path_of_the_selected_key"
        ),
        Err(ssh_harness::IsolationFailure::OutputPrivacy)
    );
    Ok(())
}

// The conformance table, for the two commands that approve a host.

/// A world whose server is not trusted, with `rule` driven through the
/// requests `request` gives of it. `Ask::Same` and `Ask::Changed`
/// present the confirmation of one preview of `Ask::Same`; the request
/// that is refused presents a confirmation no preview gave, and the one
/// with the wrong consent presents none.
///
/// The preview is made stale by another actor's pin: the host is pinned
/// to a third key, with no connection.
fn conforming(
    arrange: fn(&World) -> Result<(), FixtureError>,
    request: fn(&World, conformance::Ask) -> Mutation,
    rule: &mut dyn FnMut(&conformance::Subject<'_>),
) -> Result<(), String> {
    use conformance::{Ask, NO_SUCH_CONFIRMATION, Recorded, Subject};
    let built = || -> Result<World, FixtureError> {
        let w = World::new()?;
        arrange(&w)?;
        Ok(w)
    };
    let w = built().map_err(|_| "a world with a server".to_owned())?;
    let confirmation = std::cell::OnceCell::new();
    let previewed = || -> String {
        confirmation
            .get_or_init(|| preview(&w, &request(&w, Ask::Same)).expect("a preview").0)
            .clone()
    };
    rule(&Subject {
        outdate: Some(&|| {
            previewed();
            w.service
                .approve_host_pin_for_testing(&authority(&w.server), &third_key())
                .expect("another actor's pin");
        }),
        call: &|id, ask| {
            let presented = match ask {
                Ask::Same | Ask::Changed => Some(previewed()),
                Ask::Refused => Some(NO_SUCH_CONFIRMATION.to_owned()),
                Ask::Unconsented => None,
            };
            let mutation = match ask {
                Ask::Same | Ask::Refused | Ask::Unconsented => request(&w, Ask::Same),
                Ask::Changed => request(&w, Ask::Changed),
            };
            let envelope =
                confirmed_as(&w, &w.service, id, presented.as_deref(), mutation, Provider);
            serde_json::to_vec(&envelope).expect("an envelope serializes")
        },
        record: &|id| {
            record(&w, id).map(|record| Recorded {
                finished: record.state == RequestState::Finished,
                image: format!("{record:?}"),
            })
        },
        request_rows: &|| {
            w.db()
                .expect("the database")
                .query_row("SELECT COUNT(*) FROM request_records", [], |row| row.get(0))
                .expect("the request records")
        },
    });
    Ok(())
}

/// A key that is not the server's, the same on every call.
fn fixed_other_key() -> HostKeyIdentity {
    HostKeyIdentity {
        algorithm: "ssh-ed25519".into(),
        sha256: format!("SHA256:{}", "A".repeat(43)),
    }
}

/// A key that is neither the server's nor `fixed_other_key`.
fn third_key() -> HostKeyIdentity {
    HostKeyIdentity {
        algorithm: "ssh-ed25519".into(),
        sha256: format!("SHA256:{}A", "C".repeat(42)),
    }
}

pub(crate) const CONFORMANCE: [conformance::Row; 2] = [
    conformance::Row::new("host approve", true, "confirmation_not_found", |rule| {
        conforming(
            |_| Ok(()),
            |w, ask| match ask {
                conformance::Ask::Same
                | conformance::Ask::Refused
                | conformance::Ask::Unconsented => approval(w),
                conformance::Ask::Changed => approve_of(w, &w.server, &fixed_other_key()),
            },
            rule,
        )
    }),
    conformance::Row::new("host replace", true, "confirmation_not_found", |rule| {
        conforming(
            // The host is pinned to a key it no longer presents.
            |w| {
                fixed(
                    w.service
                        .approve_host_pin_for_testing(&authority(&w.server), &fixed_other_key()),
                )
            },
            |w, ask| match ask {
                conformance::Ask::Same
                | conformance::Ask::Refused
                | conformance::Ask::Unconsented => {
                    replacement(w, &fixed_other_key().sha256, &w.server.host_identity())
                }
                // Another old fingerprint.
                conformance::Ask::Changed => replacement(
                    w,
                    &format!("SHA256:{}A", "B".repeat(42)),
                    &w.server.host_identity(),
                ),
            },
            rule,
        )
    }),
];

/// The rows above. Those of the two synchronization commands are
/// driven by the target's other conformance case.
fn every_command_that_approves_a_host_keeps_the_rules_of_the_boundary() -> Result<(), FixtureError>
{
    conformance::run_part(
        conformance::Home::Remote,
        &request_store::bound_commands(),
        &CONFORMANCE,
        &super::CONFORMANCE.map(|row| row.command),
    );
    Ok(())
}
