//! The request lifecycle of a pull-request `remote select` (Wave 06 Cycle 02,
//! design Parts F and K, rules K1 to K7 and outcomes L1 to L14), driven
//! through the mutation boundary over the `Session` of the enable's tests,
//! whose `publish_setup` is scripted, so that no remote is reached and no
//! key is needed.

use std::panic::{AssertUnwindSafe, catch_unwind};

use serde_json::{Value, json};

use crate::{
    canonical::{ForgeKind, ForgeSetting, Integration},
    repository::{
        Mutation, MutationDataDto, OperationResumeInput, ReadError, RepositoryError,
        RepositoryErrorKind, RepositoryOperation, RepositoryService, RequestId, RequestState,
        mutation::{
            Accepted,
            command::Command,
            finish_setup_request,
            identity::{DigestSalt, FieldValue, IntentDigest, ScopeKey},
            progress::Session,
        },
        open_registry,
        setup::{
            change::{
                Change, ChangePath, Planned, path_of,
                tests::{
                    DIRECT_ORIGIN, DIRECT_ORIGIN_FORGE, PR_ORIGIN, World as Base, bytes, config,
                },
            },
            publish::SetupError,
            record::{self, SetupKind, SetupRecord},
        },
        transport::SshTransportErrorKind,
    },
    results::{CheckpointEffect, Envelope, Outcome, PublicationEffect, ResultCode, WriteEffect},
};

use super::{
    super::repo::tests::{R1, R2, Scripted, Step, branch_of, read_setup, transport},
    RemoteSelectBinding, RemoteSelectInput,
};
use crate::repository::mutation::bind::{Binding, CallContext, Entered, Standing};

/// An enabled repository whose primary holds a configuration, for a
/// `remote select`.
struct World {
    base: Base,
}

impl World {
    fn new(configuration: &str) -> Self {
        Self {
            base: Base::new(configuration),
        }
    }

    fn input(
        &self,
        name: Option<&str>,
        integration: Option<Integration>,
        forge: Option<Option<ForgeSetting>>,
    ) -> RemoteSelectInput {
        RemoteSelectInput {
            root: self.base.root.clone(),
            name: name.map(str::to_owned),
            integration,
            forge,
        }
    }

    /// The pull-request selection of `origin`.
    fn pull_request(&self) -> RemoteSelectInput {
        self.input(Some("origin"), Some(Integration::PullRequest), None)
    }

    fn run(
        &self,
        request: &str,
        input: RemoteSelectInput,
        session: &dyn Session,
    ) -> Envelope<MutationDataDto> {
        self.base.service.execute_binding(
            RequestId::parse(request).unwrap(),
            None,
            Mutation::RemoteSelect(input).into_binding(),
            session,
        )
    }

    fn record(&self, request: &str) -> crate::repository::request_store::RequestRecord {
        self.base
            .service
            .request_record(RequestId::parse(request).unwrap())
            .unwrap()
            .expect("a request record")
    }

    fn setup(&self) -> Option<SetupRecord> {
        read_setup(&self.base.service, &self.base.root)
    }

    fn count(&self, sql: &str) -> i64 {
        self.base.count(sql)
    }

    /// One registration, one record, one local row, one request: nothing
    /// was made twice. `setup` is the record as the first call made it.
    fn assert_one_setup(&self, setup: &SetupRecord, what: &str) {
        assert_eq!(self.count("SELECT COUNT(*) FROM repositories"), 1, "{what}");
        assert_eq!(
            self.count("SELECT COUNT(*) FROM repository_setups"),
            1,
            "{what}"
        );
        assert_eq!(
            self.count(
                "SELECT COUNT(*) FROM operation_records WHERE action = 'set_publication_remote'"
            ),
            1,
            "{what}"
        );
        assert_eq!(
            self.count("SELECT COUNT(*) FROM request_records"),
            1,
            "{what}"
        );
        let now = self.setup().expect("the record is kept");
        assert_eq!(now.setup_id, setup.setup_id, "{what}: a second setup");
        assert_eq!(now.request_id, setup.request_id, "{what}");
        assert_eq!(now.created_at, setup.created_at, "{what}");
    }

    fn commit_configuration(&self, source: &str) {
        let git = self.base.git();
        std::fs::write(
            self.base.root.join(crate::canonical::CONFIG_PATH),
            bytes(&config(source)),
        )
        .unwrap();
        let mut index = git.index().unwrap();
        index
            .add_path(std::path::Path::new(crate::canonical::CONFIG_PATH))
            .unwrap();
        index.write().unwrap();
        let tree = git.find_tree(index.write_tree().unwrap()).unwrap();
        let signature =
            git2::Signature::new("Fixture", "fixture@example.invalid", &git2::Time::new(2, 0))
                .unwrap();
        let parent = git.head().unwrap().peel_to_commit().unwrap();
        git.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Another change",
            &tree,
            &[&parent],
        )
        .unwrap();
    }
}

fn data_of(envelope: &Envelope<MutationDataDto>) -> Value {
    serde_json::to_value(&envelope.data).unwrap()
}

/// The `setup` data of design Part I for a change, in `state`, as the pull
/// request link of an unrecognised remote and the primary `main` has it.
fn setup_data(record: &SetupRecord, state: &str) -> Value {
    let branch = branch_of(record);
    json!({
        "type": "setup",
        "id": record.setup_id.to_string(),
        "kind": "change",
        "state": state,
        "request_id": record.request_id.to_string(),
        "branch": branch,
        "remote": "origin",
        "integration": "pull_request",
        "pull_request": {"source": branch, "target": "main", "forge": null, "url": null},
    })
}

// L1: a first call that publishes.
fn l1_a_first_call_that_publishes() {
    let w = World::new(DIRECT_ORIGIN);
    let configuration = w.base.configuration();
    let blob = w.base.blob();
    let session = Scripted::new(vec![Step::Published]);

    let answer = w.run(R1, w.pull_request(), &session);

    assert_eq!(answer.code, ResultCode::Ok, "{}", answer.message);
    assert_eq!(answer.outcome, Outcome::Success);
    assert_eq!(session.calls(), 1);
    let setup = w.setup().expect("the record is kept until the merge");
    assert_eq!(setup.kind, SetupKind::Change);
    assert_eq!(setup.base_configuration_oid, Some(blob));
    assert_eq!(data_of(&answer), setup_data(&setup, "published"));
    // Part F step 4 and K7: written, no checkpoint, published, and no commit.
    assert_eq!(answer.effects.write, WriteEffect::Written);
    assert_eq!(answer.effects.checkpoint, CheckpointEffect::NotRequested);
    assert_eq!(answer.effects.publication, PublicationEffect::Published);
    assert_eq!(answer.effects.commit_oid, None);
    assert_eq!(w.record(R1).state, RequestState::Finished);
    assert_eq!(
        w.base.row(),
        Some(("completed".to_owned(), Some("setup_recorded".to_owned())))
    );
    w.assert_one_setup(&setup, "L1");
    w.base.assert_primary_untouched("L1", &configuration);
    // The intended file is the record's: the mode and the remote, the rest
    // of the configuration kept.
    let intended = config(std::str::from_utf8(&setup.configuration).unwrap());
    assert_eq!(intended.integration(), Integration::PullRequest);
    assert_eq!(intended.publication_remote.as_deref(), Some("origin"));
    assert_eq!(intended.unknown["extra"], toml::Value::Boolean(true));
}

// L2: a first call that stops after it recorded, once for each answer K2
// leaves owed.
fn l2_each_owed_answer_keeps_the_request_and_the_record() {
    let rows: Vec<(&str, Step, ResultCode)> = vec![
        ("busy", Step::Fail(|| SetupError::Busy), ResultCode::Busy),
        (
            "recovery_required",
            Step::Fail(|| SetupError::RecoveryRequired),
            ResultCode::RecoveryRequired,
        ),
        (
            "the setup remote changed (S11)",
            Step::Fail(|| transport(SshTransportErrorKind::EndpointChanged)),
            ResultCode::RecoveryRequired,
        ),
        (
            "identity_required",
            Step::Fail(|| SetupError::IdentityRequired),
            ResultCode::IdentityRequired,
        ),
        (
            "no_selected_key",
            Step::Fail(|| transport(SshTransportErrorKind::NoSelectedKey)),
            ResultCode::NoSelectedKey,
        ),
        (
            "unlock_required",
            Step::Fail(|| transport(SshTransportErrorKind::UnlockCancelled)),
            ResultCode::UnlockRequired,
        ),
        (
            "push_rejected",
            Step::Fail(|| transport(SshTransportErrorKind::PushRejected)),
            ResultCode::PushRejected,
        ),
        (
            "a Git error",
            Step::Fail(|| {
                SetupError::Repository(RepositoryError::new(
                    RepositoryOperation::SetPublicationRemote,
                    None,
                    RepositoryErrorKind::Io,
                    "a read failed",
                ))
            }),
            crate::repository::mutation::repository_error_code(RepositoryErrorKind::Io),
        ),
        (
            "a registry error",
            Step::Fail(|| SetupError::Registry(ReadError::new(ResultCode::IndexUnavailable))),
            ResultCode::IndexUnavailable,
        ),
        (
            "a session with no credentials",
            Step::NoCredentials,
            ResultCode::InternalError,
        ),
    ];
    for (what, step, code) in rows {
        let w = World::new(DIRECT_ORIGIN);
        let configuration = w.base.configuration();
        let session = Scripted::new(vec![step]);

        let answer = w.run(R1, w.pull_request(), &session);

        assert_eq!(answer.code, code, "{what}");
        assert_eq!(answer.outcome, Outcome::Partial, "{what}");
        assert_eq!(session.calls(), 1, "{what}");
        let setup = w
            .setup()
            .unwrap_or_else(|| panic!("{what}: the record is kept"));
        assert_eq!(data_of(&answer), setup_data(&setup, "recorded"), "{what}");
        // K2: owed. A settled or deleted request would free its ID for a
        // second setup.
        let record = w.record(R1);
        assert_eq!(record.state, RequestState::Accepted, "{what}");
        assert_eq!(record.result, None, "{what}");
        assert_eq!(
            w.base.row(),
            Some(("completed".to_owned(), Some("setup_recorded".to_owned()))),
            "{what}"
        );
        w.assert_one_setup(&setup, what);
        w.base.assert_primary_untouched(what, &configuration);
    }
}

// The path chosen from the intent (design Part F's table), through the whole
// binding: what `prepare` refuses, what is already applied, what commits to
// the primary as today, and what goes on a setup branch.
fn the_path_is_chosen_from_the_intent() {
    // An equal configuration: `already_applied`, nothing written.
    let w = World::new(PR_ORIGIN);
    let none = Scripted::none();
    let answer = w.run(R1, w.input(Some("origin"), None, None), &none);
    assert_eq!(
        answer.code,
        ResultCode::AlreadyApplied,
        "{}",
        answer.message
    );
    assert_eq!(answer.outcome, Outcome::Noop);
    assert_eq!(none.calls(), 0);
    assert_eq!(w.count("SELECT COUNT(*) FROM repository_setups"), 0);
    assert_eq!(w.base.primary(), w.base.head);

    // Direct to direct, today's commit; a forge change in the same commit.
    let w = World::new(DIRECT_ORIGIN);
    let none = Scripted::none();
    let forge = ForgeSetting {
        kind: Some(ForgeKind::Gitlab),
        web_url: None,
        pull_request_url: None,
    };
    let answer = w.run(
        R1,
        w.input(
            Some("backup"),
            Some(Integration::Direct),
            Some(Some(forge.clone())),
        ),
        &none,
    );
    assert_eq!(answer.code, ResultCode::Ok, "{}", answer.message);
    assert_eq!(answer.effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(none.calls(), 0);
    assert_eq!(w.count("SELECT COUNT(*) FROM repository_setups"), 0);
    let git = w.base.git();
    let head = git.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(head.parent_id(0).unwrap(), w.base.head, "one commit");
    let written = config(std::str::from_utf8(&w.base.configuration()).unwrap());
    assert_eq!(written.publication_remote.as_deref(), Some("backup"));
    assert_eq!(written.forge(), Some(forge));

    // Another remote of a pull-request configuration, and clearing its remote:
    // refused before the request is accepted.
    let w = World::new(PR_ORIGIN);
    for (what, input, code) in [
        (
            "another remote",
            w.input(Some("backup"), None, None),
            ResultCode::InvalidInput,
        ),
        (
            "clearing the remote",
            w.input(None, None, None),
            ResultCode::InvalidConfiguration,
        ),
    ] {
        let none = Scripted::none();
        let answer = w.run(R1, input, &none);
        assert_eq!(answer.code, code, "{what}: {}", answer.message);
        assert_eq!(none.calls(), 0, "{what}");
        assert_eq!(w.count("SELECT COUNT(*) FROM request_records"), 0, "{what}");
        assert_eq!(
            w.count("SELECT COUNT(*) FROM repository_setups"),
            0,
            "{what}"
        );
        assert_eq!(w.base.primary(), w.base.head, "{what}");
    }

    // Pull request to direct, on its remote: a setup branch, and the
    // primary is not touched.
    let w = World::new(PR_ORIGIN);
    let configuration = w.base.configuration();
    let session = Scripted::new(vec![Step::Published]);
    let answer = w.run(
        R1,
        w.input(Some("origin"), Some(Integration::Direct), None),
        &session,
    );
    assert_eq!(answer.code, ResultCode::Ok, "{}", answer.message);
    let setup = w.setup().expect("a setup");
    assert_eq!(setup.kind, SetupKind::Change);
    let intended = config(std::str::from_utf8(&setup.configuration).unwrap());
    assert_eq!(intended.integration(), Integration::Direct);
    assert_eq!(data_of(&answer)["integration"], "direct");
    w.base
        .assert_primary_untouched("pull request to direct", &configuration);

    // A selection met by a pending setup is refused by the gate, before it
    // is accepted.
    let w = World::new(DIRECT_ORIGIN);
    let stopped = Scripted::new(vec![Step::Fail(|| SetupError::Busy)]);
    assert_eq!(w.run(R1, w.pull_request(), &stopped).code, ResultCode::Busy);
    let none = Scripted::none();
    let refused = w.run(R2, w.input(Some("backup"), None, None), &none);
    assert_eq!(refused.code, ResultCode::SetupPending);
    assert_eq!(w.count("SELECT COUNT(*) FROM request_records"), 1);
}

// L3, K1, K3: the same request again, record present: the setup continues.
// It is taken up as recorded and goes through its own re-entry, never
// `moved`, and compares no digest: the primary's configuration has changed
// since the request was accepted, and `moved` would have answered
// `external_change`.
fn l3_the_same_request_again_continues_the_recorded_setup() {
    let w = World::new(DIRECT_ORIGIN);
    let stopped = Scripted::new(vec![Step::Fail(|| SetupError::Busy)]);
    assert_eq!(w.run(R1, w.pull_request(), &stopped).code, ResultCode::Busy);
    let setup = w.setup().unwrap();
    w.commit_configuration(DIRECT_ORIGIN_FORGE);
    let configuration = w.base.configuration();
    let head = w.base.primary();

    let again = Scripted::new(vec![Step::Published]);
    let answer = w.run(R1, w.pull_request(), &again);

    assert_eq!(answer.code, ResultCode::Ok, "{}", answer.message);
    assert_eq!(again.calls(), 1, "published once, by this call");
    assert_eq!(data_of(&answer), setup_data(&setup, "published"));
    assert_eq!(answer.effects.commit_oid, None);
    assert_eq!(w.record(R1).state, RequestState::Finished);
    w.assert_one_setup(&setup, "L3");
    assert_eq!(w.base.primary(), head, "L3: nothing is committed");
    assert_eq!(w.base.configuration(), configuration, "L3");

    // L7: the request again after it finished, or after its answer was
    // lost: the stored envelope, through `from_stored`, and nothing is
    // published, nothing checked against the repository.
    let lost = Scripted::none();
    let replayed = w.run(R1, w.pull_request(), &lost);
    assert_eq!(lost.calls(), 0);
    assert_eq!(replayed.code, answer.code);
    assert_eq!(replayed.outcome, answer.outcome);
    assert_eq!(replayed.effects, answer.effects);
    assert_eq!(data_of(&replayed), data_of(&answer));
    w.assert_one_setup(&setup, "L7");
}

// L5, L4: a crash between the removal of the record and the request's
// settle: the removal finished the request in its own transaction.
fn l5_a_removal_finishes_the_request_in_its_own_transaction() {
    // What the ending of a refused setup (R-7) does for a change: the
    // request, then the record; the registration stays.
    fn end(service: &RepositoryService, root: &std::path::Path, request: RequestId) {
        let record = read_setup(service, root).unwrap();
        let data =
            MutationDataDto::from_stored(Command::RemoteSelect, &setup_data(&record, "recorded"))
                .expect("the setup data of a stored selection is read back");
        let mut db = open_registry(&service.registry_path, &mut |_| {}).unwrap();
        let id: i64 = db
            .query_row("SELECT id FROM repositories", [], |row| row.get(0))
            .unwrap();
        let tx = db.transaction().unwrap();
        assert!(
            finish_setup_request(
                &tx,
                id,
                request,
                ResultCode::RemoteAlreadySetUp,
                Some(data),
                1_700_000_000,
            )
            .unwrap()
        );
        record::delete(&tx, id).unwrap();
        tx.commit().unwrap();
    }

    // (a) The call answers what ended the setup.
    let w = World::new(DIRECT_ORIGIN);
    let session = Scripted::new(vec![Step::Then(Box::new(|service, root, request| {
        end(service, root, request);
        Err(SetupError::RemoteAlreadySetUp)
    }))]);
    let answer = w.run(R1, w.pull_request(), &session);
    assert_eq!(answer.code, ResultCode::RemoteAlreadySetUp);
    assert_eq!(data_of(&answer)["type"], "setup");
    assert_eq!(data_of(&answer)["kind"], "change");
    assert_eq!(w.record(R1).state, RequestState::Finished);
    assert_eq!(w.setup().map(|_| ()), None);
    // The registration is the repository's, and stays.
    assert_eq!(w.count("SELECT COUNT(*) FROM repositories"), 1);
    let none = Scripted::none();
    let replayed = w.run(R1, w.pull_request(), &none);
    assert_eq!(none.calls(), 0);
    assert_eq!(replayed.code, answer.code);
    assert_eq!(replayed.effects, answer.effects);
    assert_eq!(data_of(&replayed), data_of(&answer));

    // (b) The process dies after the removal and before the settle (L5).
    let w = World::new(DIRECT_ORIGIN);
    let session = Scripted::new(vec![Step::Then(Box::new(|service, root, request| {
        end(service, root, request);
        panic!("the process died after the removal")
    }))]);
    let crashed = catch_unwind(AssertUnwindSafe(|| w.run(R1, w.pull_request(), &session)));
    assert!(crashed.is_err());
    assert_eq!(w.record(R1).state, RequestState::Finished);
    let none = Scripted::none();
    let answer = w.run(R1, w.pull_request(), &none);
    assert_eq!(none.calls(), 0);
    assert_eq!(answer.code, ResultCode::RemoteAlreadySetUp);
    assert_eq!(data_of(&answer)["type"], "setup");
    assert_eq!(answer.effects.commit_oid, None);
}

// L6: a crash before the recording transaction, then the same request: the
// pending row has no step, and the request runs afresh as a setup request, by
// its intent, and never as a direct selection.
fn l6_a_crash_before_recording_runs_afresh_as_a_setup_request() {
    let w = World::new(DIRECT_ORIGIN);
    let configuration = w.base.configuration();
    w.base
        .service
        .set_registration_git_hook_for_testing(|| panic!("the process died before recording"));
    let session = Scripted::none();
    let crashed = catch_unwind(AssertUnwindSafe(|| w.run(R1, w.pull_request(), &session)));
    assert!(crashed.is_err());
    assert_eq!(session.calls(), 0);
    assert_eq!(w.record(R1).state, RequestState::Accepted);
    assert_eq!(w.count("SELECT COUNT(*) FROM repository_setups"), 0);
    assert_eq!(
        w.count(
            "SELECT COUNT(*) FROM operation_records
              WHERE action = 'set_publication_remote' AND state != 'completed'"
        ),
        1
    );

    let again = Scripted::new(vec![Step::Published]);
    let answer = w.run(R1, w.pull_request(), &again);

    assert_eq!(answer.code, ResultCode::Ok, "{}", answer.message);
    assert_eq!(again.calls(), 1);
    let setup = w.setup().expect("recorded by the second call");
    assert_eq!(data_of(&answer), setup_data(&setup, "published"));
    assert_eq!(
        w.base.row(),
        Some(("completed".to_owned(), Some("setup_recorded".to_owned())))
    );
    w.assert_one_setup(&setup, "L6");
    // Not the direct path: nothing is committed on the primary.
    w.base.assert_primary_untouched("L6", &configuration);
}

// L6, the other side: a configuration that changed under the request, before
// anything was recorded, is an external change. Nothing is committed and no
// setup is recorded.
fn a_configuration_changed_before_the_recording_is_an_external_change() {
    let w = World::new(DIRECT_ORIGIN);
    w.base
        .service
        .set_registration_git_hook_for_testing(|| panic!("the process died before recording"));
    let session = Scripted::none();
    let crashed = catch_unwind(AssertUnwindSafe(|| w.run(R1, w.pull_request(), &session)));
    assert!(crashed.is_err());
    w.commit_configuration(DIRECT_ORIGIN_FORGE);
    let head = w.base.primary();

    let again = Scripted::none();
    let answer = w.run(R1, w.pull_request(), &again);

    assert_eq!(
        answer.code,
        ResultCode::ExternalChange,
        "{}",
        answer.message
    );
    assert_eq!(again.calls(), 0);
    assert_eq!(w.base.primary(), head);
    assert_eq!(w.count("SELECT COUNT(*) FROM repository_setups"), 0);
}

// L11, K2, R-20: a call of a request that read its row pending, and answers
// after another call of the same request recorded the setup. Its domain call
// is refused by a check, but the request is recorded by now: it is owed, with
// the setup, and the same request goes on. Settled as a refusal before the
// recording, from the row it reads after the call, it would be deleted, and its
// ID freed for a second setup.
fn l11_a_call_refused_after_another_recorded_the_setup_leaves_the_request_owed() {
    let w = World::new(DIRECT_ORIGIN);
    let stopped = Scripted::new(vec![Step::Fail(|| SetupError::Busy)]);
    assert_eq!(w.run(R1, w.pull_request(), &stopped).code, ResultCode::Busy);
    let setup = w.setup().unwrap();
    // The configuration path is not clean by the time the late call makes
    // its checks.
    let clean = w.base.configuration();
    let mut edited = config(std::str::from_utf8(&clean).unwrap());
    edited
        .unknown
        .insert("extra".into(), toml::Value::Boolean(false));
    std::fs::write(
        w.base.root.join(crate::canonical::CONFIG_PATH),
        bytes(&edited),
    )
    .unwrap();
    let record = w.record(R1);
    let accepted = Accepted::of_record(&record, record.attempt).unwrap();
    let binding = RemoteSelectBinding::from(w.pull_request());
    let session = Scripted::none();
    let call = CallContext {
        service: &w.base.service,
        session: &session,
        confirmation: None,
    };
    let planned: Planned = {
        let current = config(std::str::from_utf8(&clean).unwrap());
        match path_of(
            &current,
            &Change {
                name: Some("origin"),
                integration: Some(Integration::PullRequest),
                forge: None,
            },
        ) {
            Ok(ChangePath::Setup(planned)) => *planned,
            other => panic!("{other:?}"),
        }
    };

    // It made its domain call as a first one: `recorded` was false.
    let ran = binding.call_setup(&call, &accepted, Some(&planned), false);

    assert_eq!(ran.answer.code, ResultCode::WorktreeNotClean);
    assert!(matches!(ran.standing, Standing::Owed));
    // Recorded, so durable though no effect shows it: partial, never a
    // plain refusal (ruling Q2).
    assert_eq!(
        Outcome::of_mutation(
            false,
            ran.answer.code,
            &ran.answer.effects,
            ran.answer.local_change
        ),
        Outcome::Partial
    );
    assert_eq!(
        serde_json::to_value(&ran.answer.data).unwrap(),
        setup_data(&setup, "recorded")
    );
    assert_eq!(w.record(R1).state, RequestState::Accepted);
    // The same request goes on from its record, once the file is as it was.
    std::fs::write(w.base.root.join(crate::canonical::CONFIG_PATH), clean).unwrap();
    let again = Scripted::new(vec![Step::Published]);
    let answer = w.run(R1, w.pull_request(), &again);
    assert_eq!(answer.code, ResultCode::Ok, "{}", answer.message);
    assert_eq!(w.record(R1).state, RequestState::Finished);
    w.assert_one_setup(&setup, "L11 refused");
}

// A binding that cannot tell its path is never run as the direct one: a
// request whose row is gone or has no step of its own, entered again by
// `rerun` over a configuration that cannot be read, would otherwise commit.
fn a_binding_that_cannot_tell_its_path_is_never_run_as_a_direct_selection() {
    let w = World::new(DIRECT_ORIGIN);
    w.base
        .service
        .set_registration_git_hook_for_testing(|| panic!("the process died before recording"));
    let session = Scripted::none();
    let crashed = catch_unwind(AssertUnwindSafe(|| w.run(R1, w.pull_request(), &session)));
    assert!(crashed.is_err());
    // The configuration is no longer a valid one.
    std::fs::write(
        w.base.root.join(crate::canonical::CONFIG_PATH),
        b"not a configuration\n",
    )
    .unwrap();
    let record = w.record(R1);
    let accepted = Accepted::of_record(&record, record.attempt).unwrap();
    let binding = RemoteSelectBinding::from(w.pull_request());
    let call = CallContext {
        service: &w.base.service,
        session: &session,
        confirmation: None,
    };
    let head = w.base.primary();

    let ran = binding.rerun(&call, &accepted, &Entered::Unfinished);

    assert_eq!(ran.answer.code, ResultCode::InvalidConfiguration);
    assert_eq!(w.base.primary(), head, "nothing was committed");
    assert_eq!(w.count("SELECT COUNT(*) FROM repository_setups"), 0);
}

// Part B's race rule, through the binding: an accepted `ticket close` of the
// repository is in a change's way. The change answers `recovery_required`,
// finally: its request is finished, and nothing is recorded or published.
fn an_accepted_close_is_in_the_way_of_a_change() {
    let w = World::new(DIRECT_ORIGIN);
    let configuration = w.base.configuration();
    w.base.accept(RequestId::parse(R2).unwrap(), "ticket close");
    let none = Scripted::none();

    let answer = w.run(R1, w.pull_request(), &none);

    assert_eq!(
        answer.code,
        ResultCode::RecoveryRequired,
        "{}",
        answer.message
    );
    assert_eq!(none.calls(), 0);
    assert_eq!(w.record(R1).state, RequestState::Finished);
    assert_eq!(
        w.record(R1).result.as_ref().map(|result| result.code),
        Some(ResultCode::RecoveryRequired)
    );
    assert_eq!(w.count("SELECT COUNT(*) FROM repository_setups"), 0);
    assert_eq!(
        w.count("SELECT COUNT(*) FROM operation_records WHERE state != 'completed'"),
        0
    );
    w.base
        .assert_primary_untouched("in the way", &configuration);
    // The same request again is the stored answer, not a second look.
    let again = w.run(R1, w.pull_request(), &none);
    assert_eq!(again.code, ResultCode::RecoveryRequired);
    assert_eq!(none.calls(), 0);
}

// L13: `operation resume` of a setup request's operation.
fn l13_resume_names_the_original_request() {
    let w = World::new(DIRECT_ORIGIN);
    let stopped = Scripted::new(vec![Step::Fail(|| SetupError::Busy)]);
    let first = w.run(R1, w.pull_request(), &stopped);
    let operation = first.operation_id.clone().expect("the request's operation");

    let resumed = w.base.service.execute_binding(
        RequestId::parse(R2).unwrap(),
        None,
        Mutation::OperationResume(OperationResumeInput {
            root: w.base.root.clone(),
            operation_id: crate::repository::OperationId::parse(&operation).unwrap(),
        })
        .into_binding(),
        &Scripted::none(),
    );

    assert_eq!(resumed.code, ResultCode::OriginalRequestRequired);
    assert!(
        resumed
            .recovery
            .iter()
            .any(|action| serde_json::to_string(action).unwrap().contains(R1)),
        "the recovery names the request: {:?}",
        resumed.recovery
    );
    assert_eq!(w.record(R1).state, RequestState::Accepted);
    assert_eq!(w.count("SELECT COUNT(*) FROM repository_setups"), 1);
}

// L14: a registry read that fails on re-entry is owed: it never falls
// through to the direct path, and no second setup is made.
fn l14_an_unreadable_record_is_owed_not_the_direct_path() {
    let w = World::new(DIRECT_ORIGIN);
    let configuration = w.base.configuration();
    let stopped = Scripted::new(vec![Step::Fail(|| SetupError::Busy)]);
    assert_eq!(w.run(R1, w.pull_request(), &stopped).code, ResultCode::Busy);
    let setup = w.setup().unwrap();
    // A row this build cannot read: the table's checks are what keep one out.
    {
        let db = open_registry(&w.base.service.registry_path, &mut |_| {}).unwrap();
        db.execute_batch(
            "PRAGMA ignore_check_constraints = ON;
             UPDATE repository_setups SET kind = 'unreadable';
             PRAGMA ignore_check_constraints = OFF;",
        )
        .unwrap();
    }

    let session = Scripted::none();
    let answer = w.run(R1, w.pull_request(), &session);

    assert_ne!(answer.code, ResultCode::Ok);
    assert_eq!(session.calls(), 0);
    let record = w.record(R1);
    assert_eq!(record.state, RequestState::Accepted, "owed, not settled");
    assert_eq!(record.result, None);
    assert_eq!(w.count("SELECT COUNT(*) FROM repository_setups"), 1);
    assert_eq!(
        w.base.row(),
        Some(("completed".to_owned(), Some("setup_recorded".to_owned())))
    );
    w.base.assert_primary_untouched("L14", &configuration);

    // Read again, it continues.
    open_registry(&w.base.service.registry_path, &mut |_| {})
        .unwrap()
        .execute("UPDATE repository_setups SET kind = 'change'", [])
        .unwrap();
    let again = Scripted::new(vec![Step::Published]);
    let answer = w.run(R1, w.pull_request(), &again);
    assert_eq!(answer.code, ResultCode::Ok, "{}", answer.message);
    w.assert_one_setup(&setup, "L14");
}

// The intent digest adds `integration` and `forge` only when they are
// supplied, and tells a null `forge` (removed) from an absent one.
fn the_digest_adds_the_new_fields_only_when_they_are_supplied() {
    let w = World::new(DIRECT_ORIGIN);
    let scope = ScopeKey::for_repository(&w.base.root).unwrap();
    let request = RequestId::parse(R1).unwrap();
    let digest = |input: RemoteSelectInput| {
        let binding = RemoteSelectBinding::from(input);
        binding
            .fields(IntentDigest::builder(
                DigestSalt::Request(request),
                "remote select",
                &scope,
                &binding.target(),
            ))
            .finish()
    };
    // Today's digest: the name alone.
    let today = IntentDigest::builder(
        DigestSalt::Request(request),
        "remote select",
        &scope,
        "origin",
    )
    .field("name", FieldValue::Text("origin"))
    .finish();
    assert_eq!(digest(w.input(Some("origin"), None, None)), today);
    let github = ForgeSetting {
        kind: Some(ForgeKind::Github),
        web_url: None,
        pull_request_url: None,
    };
    let rows = [
        digest(w.input(Some("origin"), None, None)),
        digest(w.input(Some("origin"), Some(Integration::Direct), None)),
        digest(w.input(Some("origin"), Some(Integration::PullRequest), None)),
        digest(w.input(Some("origin"), None, Some(None))),
        digest(w.input(Some("origin"), None, Some(Some(github.clone())))),
        digest(w.input(
            Some("origin"),
            None,
            Some(Some(ForgeSetting {
                kind: Some(ForgeKind::Gitlab),
                web_url: None,
                pull_request_url: None,
            })),
        )),
        digest(w.input(
            Some("origin"),
            Some(Integration::PullRequest),
            Some(Some(github)),
        )),
    ];
    for (i, a) in rows.iter().enumerate() {
        for (j, b) in rows.iter().enumerate() {
            assert_eq!(a == b, i == j, "rows {i} and {j}");
        }
    }
}

/// Design Parts F and K for a pull-request `remote select`: the path chosen
/// from the intent, the standing of each answer, the route a re-entered
/// request takes, what it answers, and that no second record or setup branch
/// is made.
#[test]
fn the_select_request_lifecycle_of_design_part_k() {
    the_digest_adds_the_new_fields_only_when_they_are_supplied();
    the_path_is_chosen_from_the_intent();
    l1_a_first_call_that_publishes();
    l2_each_owed_answer_keeps_the_request_and_the_record();
    l3_the_same_request_again_continues_the_recorded_setup();
    l5_a_removal_finishes_the_request_in_its_own_transaction();
    l6_a_crash_before_recording_runs_afresh_as_a_setup_request();
    a_configuration_changed_before_the_recording_is_an_external_change();
    l11_a_call_refused_after_another_recorded_the_setup_leaves_the_request_owed();
    a_binding_that_cannot_tell_its_path_is_never_run_as_a_direct_selection();
    an_accepted_close_is_in_the_way_of_a_change();
    l13_resume_names_the_original_request();
    l14_an_unreadable_record_is_owed_not_the_direct_path();
}
