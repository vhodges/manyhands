//! Local identity: `set_local_identity`, and `repo identity-set` through
//! the boundary.

use std::{fs, path::Path};

use manyhands::repository::{
    FailurePoint, InitialsChange, LocalIdentity, RepositoryErrorKind, RepositoryService,
    SetLocalIdentityOutcome, SetLocalIdentityRequest,
};

use super::support::{self, confirmation::Plain, git_home};

/// A credential planted in the repository's own configuration, which the
/// operation reads whole, and one planted outside it. Neither may reach
/// anything the command gives or keeps.
const LOCAL_SENTINEL: &str = "SENTINEL-local-credential-5e1b";
const GLOBAL_SENTINEL: &str = "SENTINEL-global-credential-9d42";

/// The repository's own configuration file, as it is on disk.
fn local_file(root: &Path) -> Vec<u8> {
    fs::read(root.join(".git/config")).unwrap()
}

fn local(root: &Path) -> LocalIdentity {
    let value = |key| support::confirmation::local_config(root, key);
    LocalIdentity {
        name: value("user.name"),
        email: value("user.email"),
        initials: value("manyhands.initials"),
    }
}

fn held(name: &str, email: &str, initials: Option<&str>) -> LocalIdentity {
    LocalIdentity {
        name: Some(name.to_owned()),
        email: Some(email.to_owned()),
        initials: initials.map(str::to_owned),
    }
}

fn request(root: &Path, name: &str, email: &str) -> SetLocalIdentityRequest {
    SetLocalIdentityRequest {
        root: root.to_owned(),
        name: name.to_owned(),
        email: email.to_owned(),
        initials: InitialsChange::Unchanged,
        expected: None,
    }
}

/// A service over a data directory of its own, with nothing registered.
fn service() -> (RepositoryService, tempfile::TempDir) {
    let data = tempfile::tempdir().unwrap();
    (RepositoryService::open_at(data.path()).unwrap(), data)
}

/// A Git repository with one commit and no identity of its own, which
/// Manyhands was never enabled on.
fn anonymous() -> (support::TestRepository, std::path::PathBuf) {
    let fixture = support::repository_without_local_identity();
    let root = fixture.root.canonicalize().unwrap();
    (fixture, root)
}

mod domain {
    use super::*;

    /// Runs in a process of its own: it writes configuration outside the
    /// repository, to see that it is left alone.
    #[test]
    fn an_identity_is_written_to_the_repository_and_to_no_other_configuration() {
        let Some(home) = git_home::own(
            "identity::domain::an_identity_is_written_to_the_repository_and_to_no_other_configuration",
        ) else {
            return;
        };
        fs::write(
            home.global_file(),
            format!(
                "[user]\n\tname = Global Person\n\temail = global@example.invalid\n\
                 [credential]\n\thelper = {GLOBAL_SENTINEL}\n"
            ),
        )
        .unwrap();
        fs::write(home.xdg_file(), "[manyhands]\n\tinitials = xd\n").unwrap();
        fs::write(home.system_file(), "[user]\n\tname = System Person\n").unwrap();
        let outside = home.files();
        assert!(outside.iter().all(Option::is_some));
        let (_fixture, root) = anonymous();
        assert_eq!(local(&root), LocalIdentity::default());
        let (service, _data) = service();

        let outcome = service
            .set_local_identity(SetLocalIdentityRequest {
                initials: InitialsChange::Set("AE".to_owned()),
                ..request(&root, "Ada Example", "ada@example.invalid")
            })
            .unwrap();

        let written = held("Ada Example", "ada@example.invalid", Some("ae"));
        assert_eq!(outcome, SetLocalIdentityOutcome::Changed(written.clone()));
        assert_eq!(local(&root), written);
        assert_eq!(
            home.files(),
            outside,
            "configuration outside the repository"
        );
        // Git itself now commits as the repository's identity.
        let signature = git2::Repository::open(&root).unwrap().signature().unwrap();
        assert_eq!(signature.name(), Some("Ada Example"));
        assert_eq!(signature.email(), Some("ada@example.invalid"));
        // Nothing was registered or enabled for it.
        assert!(service.list_repositories().unwrap().items.is_empty());
        assert!(!root.join(".manyhands").exists());
    }

    #[test]
    fn setting_what_is_already_held_writes_nothing() {
        git_home::isolated();
        let plain = Plain::new();
        let (service, _data) = service();
        let before = local_file(&plain.root);

        let outcome = service
            .set_local_identity(request(
                &plain.root,
                "Manyhands Test",
                "manyhands-test@example.invalid",
            ))
            .unwrap();

        assert_eq!(
            outcome,
            SetLocalIdentityOutcome::Unchanged(held(
                "Manyhands Test",
                "manyhands-test@example.invalid",
                None
            ))
        );
        assert_eq!(local_file(&plain.root), before);
        // Clearing initials that are not there changes nothing either.
        let cleared = service
            .set_local_identity(SetLocalIdentityRequest {
                initials: InitialsChange::Clear,
                ..request(
                    &plain.root,
                    "Manyhands Test",
                    "manyhands-test@example.invalid",
                )
            })
            .unwrap();
        assert!(matches!(cleared, SetLocalIdentityOutcome::Unchanged(_)));
        assert_eq!(local_file(&plain.root), before);
    }

    #[test]
    fn initials_are_set_left_alone_and_cleared() {
        git_home::isolated();
        let plain = Plain::new();
        let (service, _data) = service();
        let set = |name: &str, initials: InitialsChange| {
            service.set_local_identity(SetLocalIdentityRequest {
                initials,
                ..request(&plain.root, name, "ada@example.invalid")
            })
        };

        // Set, in lower case.
        set("Ada Example", InitialsChange::Set("Qa7".to_owned())).unwrap();
        assert_eq!(
            plain.local_config("manyhands.initials").as_deref(),
            Some("qa7")
        );

        // Absent from the request: left as they are, while the name
        // changes.
        let renamed = set("Ada Other", InitialsChange::Unchanged).unwrap();
        assert_eq!(
            renamed,
            SetLocalIdentityOutcome::Changed(held("Ada Other", "ada@example.invalid", Some("qa7")))
        );
        assert_eq!(
            plain.local_config("manyhands.initials").as_deref(),
            Some("qa7")
        );

        // The same initials again, in another case, are no change.
        let same = set("Ada Other", InitialsChange::Set("QA7".to_owned())).unwrap();
        assert!(matches!(same, SetLocalIdentityOutcome::Unchanged(_)));

        // Cleared.
        let cleared = set("Ada Other", InitialsChange::Clear).unwrap();
        assert_eq!(
            cleared,
            SetLocalIdentityOutcome::Changed(held("Ada Other", "ada@example.invalid", None))
        );
        assert_eq!(plain.local_config("manyhands.initials"), None);
        assert_eq!(
            plain.local_config("user.name").as_deref(),
            Some("Ada Other")
        );
    }

    #[test]
    fn an_identity_that_cannot_be_written_is_refused_before_anything_is_read() {
        git_home::isolated();
        let plain = Plain::new();
        let (service, _data) = service();
        let before = local_file(&plain.root);
        let good = ("Ada Example", "ada@example.invalid");
        for (name, email, initials) in [
            ("", good.1, InitialsChange::Unchanged),
            (good.0, "", InitialsChange::Unchanged),
            ("Ada\0Example", good.1, InitialsChange::Unchanged),
            (good.0, "ada\0@example.invalid", InitialsChange::Unchanged),
            (good.0, "ada.example.invalid", InitialsChange::Unchanged),
            (good.0, good.1, InitialsChange::Set("a".to_owned())),
            (good.0, good.1, InitialsChange::Set("abcd".to_owned())),
            (good.0, good.1, InitialsChange::Set("a-b".to_owned())),
            (good.0, good.1, InitialsChange::Set("éa".to_owned())),
            (good.0, good.1, InitialsChange::Set(String::new())),
        ] {
            let error = service
                .set_local_identity(SetLocalIdentityRequest {
                    initials: initials.clone(),
                    // The root is not looked at: none is there.
                    ..request(&plain.root.join("missing"), name, email)
                })
                .unwrap_err();
            assert_eq!(
                error.kind,
                RepositoryErrorKind::InvalidIdentity,
                "{name:?} {email:?} {initials:?}"
            );
        }
        assert_eq!(local_file(&plain.root), before);
    }

    #[test]
    fn a_failure_after_the_first_key_leaves_the_configuration_file_as_it_was() {
        git_home::isolated();
        let plain = Plain::new();
        // What a rewrite by Git would not keep: a comment and its spacing.
        let mut source = local_file(&plain.root);
        source.extend_from_slice(
            format!(
                "# kept   as it is\n[remote \"origin\"]\n    url = https://user:{LOCAL_SENTINEL}@example.invalid/x.git\n"
            )
            .as_bytes(),
        );
        fs::write(plain.root.join(".git/config"), &source).unwrap();
        let data = tempfile::tempdir().unwrap();
        let service = RepositoryService::open_at_with_failure_point_for_testing(
            data.path(),
            FailurePoint::AfterLocalIdentityName,
        )
        .unwrap();
        let asked = SetLocalIdentityRequest {
            initials: InitialsChange::Set("ae".to_owned()),
            ..request(&plain.root, "Ada Example", "ada@example.invalid")
        };

        let error = service.set_local_identity(asked.clone()).unwrap_err();

        assert_eq!(error.kind, RepositoryErrorKind::InjectedFailure);
        assert_eq!(local_file(&plain.root), source, "as it was, byte for byte");
        // The same call, with nothing in its way, writes all three.
        assert!(matches!(
            service.set_local_identity(asked).unwrap(),
            SetLocalIdentityOutcome::Changed(_)
        ));
        assert_eq!(
            local(&plain.root),
            held("Ada Example", "ada@example.invalid", Some("ae"))
        );
    }

    #[test]
    fn a_root_that_is_no_repository_s_own_is_refused_as_what_it_is() {
        git_home::isolated();
        let (service, _data) = service();
        let kind = |root: &Path| {
            service
                .set_local_identity(request(root, "Ada Example", "ada@example.invalid"))
                .unwrap_err()
                .kind
        };

        let bare = support::bare_repository();
        let bare_before = fs::read(bare.root.join("config")).unwrap();
        assert_eq!(kind(&bare.root), RepositoryErrorKind::BareRepository);
        assert_eq!(fs::read(bare.root.join("config")).unwrap(), bare_before);

        let directory = tempfile::tempdir().unwrap();
        assert_eq!(kind(directory.path()), RepositoryErrorKind::NotRepository);
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
        assert_eq!(
            kind(&directory.path().join("missing")),
            RepositoryErrorKind::InvalidPath
        );

        // A directory below the root, and a linked worktree, which shares
        // the configuration of the repository that owns it.
        let plain = Plain::new();
        let before = local_file(&plain.root);
        fs::create_dir(plain.root.join("below")).unwrap();
        assert_eq!(
            kind(&plain.root.join("below")),
            RepositoryErrorKind::InvalidPath
        );
        let outside = tempfile::tempdir().unwrap();
        let linked = outside.path().join("linked");
        plain
            .fixture
            .repository
            .worktree("linked", &linked, None)
            .unwrap();
        assert_eq!(kind(&linked), RepositoryErrorKind::InvalidPath);
        assert_eq!(local_file(&plain.root), before);
    }

    #[test]
    fn an_identity_that_is_not_the_one_the_caller_saw_is_not_written_over() {
        git_home::isolated();
        let plain = Plain::new();
        let (service, _data) = service();
        let seen = local(&plain.root);
        git2::Config::open(&plain.root.join(".git/config"))
            .unwrap()
            .set_str("user.email", "later@example.invalid")
            .unwrap();
        let before = local_file(&plain.root);
        let asked = SetLocalIdentityRequest {
            expected: Some(seen),
            ..request(&plain.root, "Ada Example", "ada@example.invalid")
        };

        let error = service.set_local_identity(asked.clone()).unwrap_err();

        assert_eq!(error.kind, RepositoryErrorKind::ExternalChange);
        assert_eq!(local_file(&plain.root), before);
        // What the caller sees now is what it may write over.
        let outcome = service
            .set_local_identity(SetLocalIdentityRequest {
                expected: Some(local(&plain.root)),
                ..asked
            })
            .unwrap();
        assert!(matches!(outcome, SetLocalIdentityOutcome::Changed(_)));
    }
}

mod boundary {
    use manyhands::{
        repository::{
            IdentityPreviewDto, IdentitySource, InitialsSource, LocalIdentityDto, Mutation,
            MutationDataDto, PreviewDetailsDto, PreviewDto, PreviewEffect, RepoIdentitySetInput,
            RequestState, TicketSaveInput,
        },
        results::{Effects, Envelope, Outcome, ResultCode},
    };

    use super::{
        super::support::{
            confirmation::{Desk, IDENTITY_PREVIEW_SCHEMA, START},
            golden::{self, ContractCase},
            items::TICKET_A,
            mutation::{
                self, REQUEST_1, REQUEST_2, REQUEST_3, World, execute_with, identity_data,
                recovery_actions,
            },
        },
        *,
    };

    const IDENTITY_SCHEMA: &str = "identity_mutation.schema.json";

    pub(super) const NAME: &str = "Ada Example";
    pub(super) const EMAIL: &str = "ada@example.invalid";

    pub(super) fn input(root: &Path, initials: InitialsChange) -> RepoIdentitySetInput {
        RepoIdentitySetInput {
            root: root.to_owned(),
            name: NAME.to_owned(),
            email: EMAIL.to_owned(),
            initials,
        }
    }

    /// The request the scenarios make: a name and an email, with nothing
    /// said of initials.
    pub(super) fn set(root: &Path) -> Mutation {
        Mutation::RepoIdentitySet(input(root, InitialsChange::Unchanged))
    }

    pub(super) fn set_with(root: &Path, initials: InitialsChange) -> Mutation {
        Mutation::RepoIdentitySet(input(root, initials))
    }

    pub(super) fn details(preview: &PreviewDto) -> &IdentityPreviewDto {
        match &preview.details {
            PreviewDetailsDto::Identity(details) => details,
            other => panic!("not a preview of an identity: {other:?}"),
        }
    }

    /// Changes one value of the repository's own configuration by hand,
    /// as `git config` would.
    pub(super) fn by_hand(root: &Path, key: &str, value: &str) {
        git2::Config::open(&root.join(".git/config"))
            .unwrap()
            .set_str(key, value)
            .unwrap();
    }

    /// A request that set the identity: a success with no effect, whose
    /// data is what the repository's own configuration now holds.
    pub(super) fn assert_set(envelope: &Envelope<MutationDataDto>, initials: Option<&str>) {
        assert_eq!(envelope.code, ResultCode::Ok, "{}", envelope.message);
        assert_eq!(envelope.outcome, Outcome::Success);
        assert_eq!(envelope.effects, Effects::not_requested());
        assert_eq!(envelope.operation_id, None, "the command has no operation");
        assert!(envelope.recovery.is_empty());
        let data = identity_data(envelope);
        assert_eq!(
            (
                data.name.as_deref(),
                data.email.as_deref(),
                data.initials.as_deref()
            ),
            (Some(NAME), Some(EMAIL), initials)
        );
    }

    /// Runs in a process of its own: the identity it is shown comes from
    /// outside the repository.
    #[test]
    fn a_preview_shows_what_will_be_written_what_is_there_and_where_the_identity_comes_from() {
        let Some(home) = git_home::own(
            "identity::boundary::a_preview_shows_what_will_be_written_what_is_there_and_where_the_identity_comes_from",
        ) else {
            return;
        };
        fs::write(
            home.global_file(),
            "[user]\n\tname = Global Person\n\temail = global@example.invalid\n",
        )
        .unwrap();
        fs::write(home.xdg_file(), "[manyhands]\n\tinitials = xd\n").unwrap();
        fs::write(home.system_file(), "[user]\n\tname = System Person\n").unwrap();
        let outside = home.files();
        let (_fixture, root) = anonymous();
        let desk = Desk::new();
        let before = local_file(&root);
        let request = || set_with(&root, InitialsChange::Set("AE".to_owned()));

        let (confirmation, preview) = desk.preview(&request());

        assert_eq!(preview.command, "repo identity-set");
        assert_eq!(preview.scope.repository.as_deref(), root.to_str());
        assert_eq!(
            preview
                .effects
                .iter()
                .map(|entry| entry.effect)
                .collect::<Vec<_>>(),
            [PreviewEffect::IdentityWritten]
        );
        let shown = details(&preview);
        assert_eq!(shown.root, root.to_str().unwrap());
        // What will be written, the initials in lower case.
        assert_eq!(
            (
                shown.name.as_str(),
                shown.email.as_str(),
                shown.initials.as_deref()
            ),
            (NAME, EMAIL, Some("ae"))
        );
        // What the repository's own configuration holds: nothing.
        assert_eq!(
            shown.current,
            LocalIdentityDto {
                name: None,
                email: None,
                initials: None
            }
        );
        // The identity a commit would carry now, and where it is from.
        assert_eq!(shown.effective.source, IdentitySource::Global);
        assert_eq!(shown.effective.name.as_deref(), Some("Global Person"));
        assert_eq!(
            shown.effective.email.as_deref(),
            Some("global@example.invalid")
        );
        // Initials set outside the repository are not this repository's.
        assert_eq!(shown.effective.initials.as_deref(), Some("gp"));
        assert_eq!(shown.effective.initials_source, InitialsSource::Derived);
        // A preview writes its confirmation and nothing else.
        assert_eq!(local_file(&root), before);
        assert_eq!(home.files(), outside);
        assert_eq!(desk.request_rows(), 0);
        assert_eq!(desk.confirmation_rows(), 1);

        let done = desk.execute(REQUEST_1, &confirmation, request());

        assert_set(&done, Some("ae"));
        assert_eq!(local(&root), held(NAME, EMAIL, Some("ae")));
        assert_eq!(
            home.files(),
            outside,
            "configuration outside the repository"
        );
        let record = desk.record(REQUEST_1).unwrap();
        assert_eq!(record.state, RequestState::Finished);
        assert!(record.operations.is_empty());
        assert_eq!(record.expected_digest, None);
        // The repository is as little enabled as it was.
        assert!(!desk.registered(&root));
        assert!(!root.join(".manyhands").exists());
        assert_eq!(desk.pending_journal_rows(), 0);
    }

    #[test]
    fn what_the_repository_already_holds_is_a_no_op_and_gets_no_confirmation() {
        git_home::isolated();
        let plain = Plain::new();
        by_hand(&plain.root, "user.name", NAME);
        by_hand(&plain.root, "user.email", EMAIL);
        let desk = Desk::new();
        let before = local_file(&plain.root);

        for request in [
            set(&plain.root),
            // Initials that are not there are cleared already.
            set_with(&plain.root, InitialsChange::Clear),
        ] {
            let prepared = desk.prepare(&request);
            assert_eq!(prepared.code, ResultCode::AlreadyApplied);
            assert_eq!(prepared.outcome, Outcome::Noop);
            assert!(prepared.data.is_none());
            assert!(prepared.recovery.is_empty());
        }
        assert_eq!(desk.confirmation_rows(), 0);
        assert_eq!(local_file(&plain.root), before);
    }

    #[test]
    fn initials_are_set_cleared_and_left_alone_through_the_boundary() {
        git_home::isolated();
        let plain = Plain::new();
        let desk = Desk::new();
        let run = |request_id: &str, initials: InitialsChange| {
            let (confirmation, preview) = desk.preview(&set_with(&plain.root, initials.clone()));
            let shown = details(&preview).initials.clone();
            let done = desk.execute(request_id, &confirmation, set_with(&plain.root, initials));
            (shown, done)
        };

        let (shown, done) = run(REQUEST_1, InitialsChange::Set("Qa7".to_owned()));
        assert_eq!(shown.as_deref(), Some("qa7"));
        assert_set(&done, Some("qa7"));
        assert_eq!(
            plain.local_config("manyhands.initials").as_deref(),
            Some("qa7")
        );

        // Nothing said of them, with the identity set already: they are
        // what the configuration holds, and the request is a no-op.
        let unchanged = desk.prepare(&set(&plain.root));
        assert_eq!(unchanged.code, ResultCode::AlreadyApplied);
        assert_eq!(
            plain.local_config("manyhands.initials").as_deref(),
            Some("qa7")
        );

        let (shown, done) = run(REQUEST_2, InitialsChange::Clear);
        assert_eq!(shown, None);
        assert_set(&done, None);
        assert_eq!(plain.local_config("manyhands.initials"), None);
        assert_eq!(plain.local_config("user.name").as_deref(), Some(NAME));
    }

    #[test]
    fn initials_the_request_does_not_name_are_shown_and_kept() {
        git_home::isolated();
        let plain = Plain::new();
        by_hand(&plain.root, "manyhands.initials", "zz");
        let desk = Desk::new();

        let (confirmation, preview) = desk.preview(&set(&plain.root));

        let shown = details(&preview);
        assert_eq!(shown.initials.as_deref(), Some("zz"));
        assert_eq!(
            shown.current,
            LocalIdentityDto {
                name: Some("Manyhands Test".to_owned()),
                email: Some("manyhands-test@example.invalid".to_owned()),
                initials: Some("zz".to_owned()),
            }
        );
        assert_eq!(shown.effective.source, IdentitySource::Repository);
        assert_eq!(shown.effective.initials_source, InitialsSource::Repository);
        let done = desk.execute(REQUEST_1, &confirmation, set(&plain.root));
        assert_set(&done, Some("zz"));
        assert_eq!(local(&plain.root), held(NAME, EMAIL, Some("zz")));
    }

    #[test]
    fn an_identity_that_cannot_be_written_is_invalid_input() {
        git_home::isolated();
        let plain = Plain::new();
        let desk = Desk::new();
        let before = local_file(&plain.root);
        let request = |name: &str, email: &str, initials: InitialsChange| {
            Mutation::RepoIdentitySet(RepoIdentitySetInput {
                root: plain.root.clone(),
                name: name.to_owned(),
                email: email.to_owned(),
                initials,
            })
        };
        for (what, request) in [
            (
                "an empty name",
                request("", EMAIL, InitialsChange::Unchanged),
            ),
            (
                "a NUL in the name",
                request("Ada\0Example", EMAIL, InitialsChange::Unchanged),
            ),
            (
                "a NUL in the email",
                request(NAME, "ada\0@example.invalid", InitialsChange::Unchanged),
            ),
            (
                "an email without @",
                request(NAME, "ada.example.invalid", InitialsChange::Unchanged),
            ),
            (
                "an empty email",
                request(NAME, "", InitialsChange::Unchanged),
            ),
            (
                "initials that are too long",
                request(NAME, EMAIL, InitialsChange::Set("abcd".to_owned())),
            ),
            (
                "initials that are not ASCII",
                request(NAME, EMAIL, InitialsChange::Set("éa".to_owned())),
            ),
        ] {
            let prepared = desk.prepare(&request);
            assert_eq!(prepared.code, ResultCode::InvalidInput, "{what}");
            assert_eq!(prepared.outcome, Outcome::Error, "{what}");
            assert!(prepared.data.is_none(), "{what}");
        }
        assert_eq!(desk.confirmation_rows(), 0);
        assert_eq!(local_file(&plain.root), before);
    }

    #[test]
    fn a_root_that_is_no_repository_s_own_is_refused_as_what_it_is() {
        git_home::isolated();
        let desk = Desk::new();
        let code = |root: &Path| desk.prepare(&set(root)).code;

        let bare = support::bare_repository();
        assert_eq!(code(&bare.root), ResultCode::BareRepository);
        let directory = tempfile::tempdir().unwrap();
        assert_eq!(code(directory.path()), ResultCode::NotRepository);
        assert_eq!(
            code(&directory.path().join("missing")),
            ResultCode::InvalidPath
        );
        let plain = Plain::new();
        let outside = tempfile::tempdir().unwrap();
        let linked = outside.path().join("linked");
        plain
            .fixture
            .repository
            .worktree("linked", &linked, None)
            .unwrap();
        assert_eq!(code(&linked), ResultCode::InvalidPath);
        assert_eq!(desk.confirmation_rows(), 0);

        // An execute of one presents no confirmation that could be its
        // own, and is refused for that before its root is looked at.
        let unconfirmed = execute_with(&desk.service, REQUEST_1, set(&bare.root));
        assert_eq!(unconfirmed.code, ResultCode::ConfirmationRequired);
        desk.assert_nothing_left(REQUEST_1);
    }

    #[test]
    fn a_failure_after_the_first_key_leaves_the_file_as_it_was_and_nothing_of_the_request() {
        git_home::isolated();
        let plain = Plain::new();
        let mut desk = Desk::new();
        let request = || set_with(&plain.root, InitialsChange::Set("ae".to_owned()));
        let (confirmation, _) = desk.preview(&request());
        let before = local_file(&plain.root);
        desk.fail_once_at(FailurePoint::AfterLocalIdentityName);

        let failed = desk.execute(REQUEST_1, &confirmation, request());

        assert_eq!(failed.code, ResultCode::InternalError);
        assert_eq!(failed.outcome, Outcome::Error);
        assert_eq!(failed.effects, Effects::not_requested());
        assert!(failed.data.is_none());
        assert!(failed.recovery.is_empty(), "{:?}", failed.recovery);
        assert_eq!(local_file(&plain.root), before, "as it was, byte for byte");
        desk.assert_nothing_left(REQUEST_1);

        // The confirmation was released with the record: the request,
        // sent again, is a new one and writes.
        let done = desk.execute(REQUEST_1, &confirmation, request());
        assert_set(&done, Some("ae"));
    }

    #[test]
    fn a_local_value_changed_after_the_preview_is_an_external_change() {
        git_home::isolated();
        let plain = Plain::new();
        let desk = Desk::new();
        let (stale, _) = desk.preview(&set(&plain.root));
        by_hand(&plain.root, "user.email", "later@example.invalid");
        let before = local_file(&plain.root);

        let changed = desk.execute(REQUEST_1, &stale, set(&plain.root));

        assert_eq!(changed.code, ResultCode::ExternalChange);
        assert_eq!(changed.outcome, Outcome::Blocked);
        assert_eq!(recovery_actions(&changed), ["request.prepare"]);
        assert!(changed.data.is_none());
        assert_eq!(local_file(&plain.root), before);
        desk.assert_nothing_left(REQUEST_1);

        // Initials count as much as the name and the email.
        by_hand(&plain.root, "user.email", "manyhands-test@example.invalid");
        by_hand(&plain.root, "manyhands.initials", "zz");
        let initials = desk.execute(REQUEST_1, &stale, set(&plain.root));
        assert_eq!(initials.code, ResultCode::ExternalChange);
        desk.assert_nothing_left(REQUEST_1);

        // A new preview shows what is there now, and its request writes.
        let (fresh, preview) = desk.preview(&set(&plain.root));
        assert_eq!(details(&preview).current.initials.as_deref(), Some("zz"));
        assert_set(
            &desk.execute(REQUEST_1, &fresh, set(&plain.root)),
            Some("zz"),
        );
    }

    /// The binding's own check on re-entry: the request was accepted,
    /// and died before it wrote.
    #[test]
    fn a_retry_that_finds_another_identity_than_the_one_previewed_is_an_external_change() {
        git_home::isolated();
        let plain = Plain::new();
        let mut desk = Desk::new();
        let (confirmation, _) = desk.preview(&set(&plain.root));
        desk.fail_once_at(FailurePoint::BeforeRequestDomainCall);
        let died = desk.execute(REQUEST_1, &confirmation, set(&plain.root));
        assert_eq!(died.code, ResultCode::InternalError);
        assert_eq!(
            desk.record(REQUEST_1).map(|record| record.state),
            Some(RequestState::Accepted)
        );
        // Someone sets another identity before the retry.
        by_hand(&plain.root, "user.name", "Later Person");
        let before = local_file(&plain.root);
        desk.reopen();

        let retry = execute_with(&desk.service, REQUEST_1, set(&plain.root));

        assert_eq!(retry.code, ResultCode::ExternalChange);
        assert_eq!(retry.outcome, Outcome::Blocked);
        assert_eq!(recovery_actions(&retry), ["request.prepare"]);
        assert_eq!(local_file(&plain.root), before, "the later identity stands");
        desk.assert_nothing_left(REQUEST_1);
        // Sent once more with the consent it had, it is a new request,
        // and is refused for the same reason.
        let again = desk.execute(REQUEST_1, &confirmation, set(&plain.root));
        assert_eq!(again.code, ResultCode::ExternalChange);
        assert_eq!(local_file(&plain.root), before);
        desk.assert_nothing_left(REQUEST_1);
    }

    #[test]
    fn an_identity_changed_between_the_observation_and_the_write_is_not_written_over() {
        git_home::isolated();
        let plain = Plain::new();
        let desk = Desk::new();
        let (confirmation, _) = desk.preview(&set(&plain.root));
        let root = plain.root.clone();
        // Runs once the request is accepted, before its domain call.
        desk.service
            .set_request_hook_for_testing(move || by_hand(&root, "user.name", "Later Person"));

        let changed = desk.execute(REQUEST_1, &confirmation, set(&plain.root));

        assert_eq!(changed.code, ResultCode::ExternalChange);
        assert_eq!(recovery_actions(&changed), ["request.prepare"]);
        assert_eq!(
            plain.local_config("user.name").as_deref(),
            Some("Later Person")
        );
        assert_eq!(
            plain.local_config("user.email").as_deref(),
            Some("manyhands-test@example.invalid")
        );
        desk.assert_nothing_left(REQUEST_1);
    }

    #[test]
    fn a_repository_another_operation_holds_is_busy_and_is_left_as_it_is() {
        git_home::isolated();
        let plain = Plain::new();
        let desk = Desk::new();
        let (confirmation, _) = desk.preview(&set(&plain.root));
        let before = local_file(&plain.root);
        let held = RepositoryService::hold_lease_for_testing(
            &plain.root,
            desk.data.path(),
            manyhands::repository::LeaseKind::Repository,
        )
        .unwrap();

        let busy = desk.execute(REQUEST_1, &confirmation, set(&plain.root));

        assert_eq!(busy.code, ResultCode::Busy);
        assert_eq!(busy.outcome, Outcome::Error);
        assert_eq!(recovery_actions(&busy), ["request.retry"]);
        assert_eq!(local_file(&plain.root), before);
        desk.assert_nothing_left(REQUEST_1);
        // Once the repository is let go, the same request writes.
        drop(held);
        assert_set(
            &desk.execute(REQUEST_1, &confirmation, set(&plain.root)),
            None,
        );
    }

    #[test]
    fn a_request_whose_result_was_lost_after_it_wrote_is_a_no_op_on_retry() {
        git_home::isolated();
        let plain = Plain::new();
        let mut desk = Desk::new();
        let request = || set_with(&plain.root, InitialsChange::Set("ae".to_owned()));
        let (confirmation, _) = desk.preview(&request());
        desk.fail_once_at(FailurePoint::BeforeRequestSettlement);
        let lost = desk.execute(REQUEST_1, &confirmation, request());
        assert_eq!(lost.code, ResultCode::InternalError);
        // The identity is written, and the record does not say so.
        assert_eq!(local(&plain.root), held(NAME, EMAIL, Some("ae")));
        assert_eq!(
            desk.record(REQUEST_1).map(|record| record.state),
            Some(RequestState::Accepted)
        );
        let written = local_file(&plain.root);
        desk.reopen();

        let retry = execute_with(&desk.service, REQUEST_1, request());

        assert_eq!(retry.code, ResultCode::AlreadyApplied, "{}", retry.message);
        assert_eq!(retry.outcome, Outcome::Noop);
        assert_eq!(retry.effects, Effects::not_requested());
        assert!(retry.recovery.is_empty());
        let data = identity_data(&retry);
        assert_eq!(
            (
                data.name.as_deref(),
                data.email.as_deref(),
                data.initials.as_deref()
            ),
            (Some(NAME), Some(EMAIL), Some("ae"))
        );
        assert_eq!(local_file(&plain.root), written, "nothing is written twice");
        assert_eq!(
            desk.record(REQUEST_1).map(|record| record.state),
            Some(RequestState::Finished)
        );
        // And that is the request's result from then on.
        let replay = execute_with(&desk.service, REQUEST_1, request());
        assert_eq!(
            serde_json::to_value(&replay).unwrap(),
            serde_json::to_value(&retry).unwrap()
        );
    }

    #[test]
    fn a_request_that_died_before_it_wrote_is_completed_by_its_retry_however_old() {
        git_home::isolated();
        let plain = Plain::new();
        let mut desk = Desk::new();
        let (confirmation, _) = desk.preview(&set(&plain.root));
        desk.fail_once_at(FailurePoint::BeforeRequestDomainCall);
        let died = desk.execute(REQUEST_1, &confirmation, set(&plain.root));
        // What a call that died returns is no result of the request.
        assert_eq!(died.code, ResultCode::InternalError);
        assert_eq!(
            plain.local_config("user.name").as_deref(),
            Some("Manyhands Test")
        );
        // Long after the confirmation would have expired for a new
        // request: it is this request's.
        desk.clock.set(START + 24 * 60 * 60);
        desk.reopen();

        let retry = execute_with(&desk.service, REQUEST_1, set(&plain.root));

        assert_set(&retry, None);
        assert_eq!(local(&plain.root), held(NAME, EMAIL, None));
        assert_eq!(
            desk.record(REQUEST_1).map(|record| record.state),
            Some(RequestState::Finished)
        );
    }

    #[test]
    fn a_retry_that_fails_keeps_its_record_and_is_what_goes_on() {
        git_home::isolated();
        let plain = Plain::new();
        let mut desk = Desk::new();
        let (confirmation, _) = desk.preview(&set(&plain.root));
        desk.fail_once_at(FailurePoint::BeforeRequestDomainCall);
        desk.execute(REQUEST_1, &confirmation, set(&plain.root));
        let before = local_file(&plain.root);
        desk.fail_once_at(FailurePoint::AfterLocalIdentityName);

        let failed = execute_with(&desk.service, REQUEST_1, set(&plain.root));

        assert_eq!(failed.code, ResultCode::InternalError);
        assert_eq!(failed.outcome, Outcome::Error);
        assert_eq!(recovery_actions(&failed), ["request.retry"]);
        assert_eq!(local_file(&plain.root), before, "as it was, byte for byte");
        assert_eq!(
            desk.record(REQUEST_1).map(|record| record.state),
            Some(RequestState::Accepted),
            "a retry does not give its record up"
        );
        desk.reopen();
        assert_set(
            &execute_with(&desk.service, REQUEST_1, set(&plain.root)),
            None,
        );
    }

    #[test]
    fn a_finished_request_is_replayed_and_a_changed_input_is_a_mismatch() {
        git_home::isolated();
        let plain = Plain::new();
        let desk = Desk::new();
        let (confirmation, _) = desk.preview(&set(&plain.root));
        let done = desk.execute(REQUEST_1, &confirmation, set(&plain.root));
        assert_set(&done, None);
        // The identity changes afterwards: a replay neither looks nor
        // writes.
        by_hand(&plain.root, "user.name", "Later Person");
        let later = local_file(&plain.root);
        let record = desk.record(REQUEST_1);

        let replay = execute_with(&desk.service, REQUEST_1, set(&plain.root));

        assert_eq!(
            serde_json::to_value(&replay).unwrap(),
            serde_json::to_value(&done).unwrap()
        );
        assert_eq!(local_file(&plain.root), later);
        for changed in [
            Mutation::RepoIdentitySet(RepoIdentitySetInput {
                name: "Ada Other".to_owned(),
                ..input(&plain.root, InitialsChange::Unchanged)
            }),
            Mutation::RepoIdentitySet(RepoIdentitySetInput {
                email: "other@example.invalid".to_owned(),
                ..input(&plain.root, InitialsChange::Unchanged)
            }),
            // Initials that are cleared are not initials left alone.
            set_with(&plain.root, InitialsChange::Clear),
            set_with(&plain.root, InitialsChange::Set("ae".to_owned())),
        ] {
            let mismatch = desk.execute(REQUEST_1, &confirmation, changed);
            assert_eq!(mismatch.code, ResultCode::RequestMismatch);
            assert_eq!(mismatch.outcome, Outcome::Error);
        }
        assert_eq!(desk.record(REQUEST_1), record);
        assert_eq!(local_file(&plain.root), later);
        assert_eq!(desk.request_rows(), 1);
    }

    #[test]
    fn a_save_that_stopped_for_an_identity_succeeds_once_one_is_set() {
        git_home::isolated();
        let world = World::new();
        let mut own = git2::Config::open(&world.root.join(".git/config")).unwrap();
        own.remove("user.name").unwrap();
        own.remove("user.email").unwrap();
        drop(own);
        let save = |world: &World| {
            Mutation::TicketSave(TicketSaveInput {
                draft: World::draft(mutation::TITLE, mutation::BODY),
                ..world.save_input(TICKET_A, &world.token(TICKET_A))
            })
        };

        let stopped = world.execute(REQUEST_1, save(&world));

        assert_eq!(stopped.code, ResultCode::IdentityRequired);
        assert_eq!(recovery_actions(&stopped), ["repo.identity_set"]);
        assert_eq!(
            stopped.recovery[0].arguments.get("root"),
            Some(&serde_json::Value::from(world.root.to_str().unwrap()))
        );
        assert_eq!(world.branch_commits(TICKET_A), 0);

        // The action the result offers, on a repository that is enabled.
        let preview = world.service.prepare(&set(&world.root));
        assert_eq!(preview.code, ResultCode::Ok, "{}", preview.message);
        let preview = preview.data.unwrap();
        assert_eq!(details(&preview).effective.source, IdentitySource::None);
        let done = mutation::execute_confirmed(
            &world.service,
            REQUEST_2,
            &preview.confirmation_id,
            set(&world.root),
        );
        assert_set(&done, None);

        let saved = world.execute(REQUEST_3, save(&world));

        assert_eq!(saved.outcome, Outcome::Success, "{}", saved.message);
        assert_eq!(world.branch_commits(TICKET_A), 1);
        let repository = git2::Repository::open(&world.root).unwrap();
        let commit = repository
            .find_commit(world.branch_tip(TICKET_A).unwrap())
            .unwrap();
        assert_eq!(commit.author().name(), Some(NAME));
        assert_eq!(commit.author().email(), Some(EMAIL));
        let identity = world
            .service
            .repository_identity(&world.resolved())
            .unwrap();
        assert_eq!(identity.source, IdentitySource::Repository);
        assert_eq!(identity.initials.as_deref(), Some("ae"));
        assert_eq!(identity.initials_source, InitialsSource::Derived);
    }

    /// Runs in a process of its own: it plants a credential outside the
    /// repository as well as in it.
    #[test]
    fn no_credential_of_any_configuration_reaches_an_envelope_or_a_record() {
        let Some(home) = git_home::own(
            "identity::boundary::no_credential_of_any_configuration_reaches_an_envelope_or_a_record",
        ) else {
            return;
        };
        fs::write(
            home.global_file(),
            format!(
                "[user]\n\tname = Global Person\n\temail = global@example.invalid\n\
                 [credential]\n\thelper = {GLOBAL_SENTINEL}\n"
            ),
        )
        .unwrap();
        let (_fixture, root) = anonymous();
        let mut source = local_file(&root);
        source.extend_from_slice(
            format!(
                "[remote \"origin\"]\n\turl = https://user:{LOCAL_SENTINEL}@example.invalid/x.git\n"
            )
            .as_bytes(),
        );
        fs::write(root.join(".git/config"), &source).unwrap();
        let sentinels = [LOCAL_SENTINEL, GLOBAL_SENTINEL];
        let scan = |envelope: serde_json::Value| {
            assert_eq!(golden::find_sentinel(&envelope, &sentinels), None);
        };
        let mut desk = Desk::new();

        let preview = desk.prepare(&set(&root));
        let confirmation = preview.data.as_ref().unwrap().confirmation_id.clone();
        scan(serde_json::to_value(&preview).unwrap());
        // A call reads the file whole, to edit a copy of it.
        desk.fail_once_at(FailurePoint::AfterLocalIdentityName);
        let failed = desk.execute(REQUEST_1, &confirmation, set(&root));
        assert_eq!(failed.code, ResultCode::InternalError);
        assert_eq!(local_file(&root), source);
        scan(serde_json::to_value(&failed).unwrap());
        let done = desk.execute(REQUEST_1, &confirmation, set(&root));
        assert_set(&done, None);
        scan(serde_json::to_value(&done).unwrap());
        scan(serde_json::to_value(execute_with(&desk.service, REQUEST_1, set(&root))).unwrap());
        scan(serde_json::to_value(desk.prepare(&set(&root))).unwrap());
        scan(
            serde_json::to_value(
                desk.service
                    .show_request(manyhands::repository::RequestId::parse(REQUEST_1).unwrap()),
            )
            .unwrap(),
        );

        mutation::assert_request_stores_exclude(desk.data.path(), &sentinels);
        // The scan can fail: both credentials are in files the command
        // read, and the stores are where it keeps what it may keep.
        assert!(String::from_utf8_lossy(&local_file(&root)).contains(LOCAL_SENTINEL));
        assert!(
            String::from_utf8_lossy(&fs::read(home.global_file()).unwrap())
                .contains(GLOBAL_SENTINEL)
        );
        assert!(mutation::find_stored_sentinel(desk.data.path(), &[NAME]).is_some());
    }

    /// Checks an envelope of `repo identity-set`, or of its preview,
    /// against its fixture. What differs from run to run is replaced.
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
                sentinels: &[],
            },
            envelope,
        );
    }

    #[test]
    fn a_repo_identity_set_and_its_preview_match_their_schemas_and_goldens() {
        git_home::isolated();
        let plain = Plain::new();
        by_hand(&plain.root, "manyhands.initials", "mt");
        // The clock stands still, so the preview's expiry is fixed.
        let desk = Desk::new();
        let request = || set_with(&plain.root, InitialsChange::Set("AE".to_owned()));

        let stale = desk.service.prepare(&request());
        assert_eq!(stale.outcome, Outcome::Success);
        assert_golden(
            "repo_identity_set_preview",
            &plain.root,
            IDENTITY_PREVIEW_SCHEMA,
            &stale,
        );
        let stale = stale.data.unwrap().confirmation_id;
        by_hand(&plain.root, "user.email", "later@example.invalid");
        let changed = desk.execute(REQUEST_1, &stale, request());
        assert_eq!(changed.code, ResultCode::ExternalChange);
        assert_golden(
            "repo_identity_set_external_change",
            &plain.root,
            IDENTITY_SCHEMA,
            &changed,
        );
        by_hand(&plain.root, "user.email", "manyhands-test@example.invalid");

        let done = desk.execute(REQUEST_1, &stale, request());
        assert_set(&done, Some("ae"));
        assert_golden("repo_identity_set", &plain.root, IDENTITY_SCHEMA, &done);

        let noop = desk.service.prepare(&request());
        assert_eq!(noop.outcome, Outcome::Noop);
        assert_golden(
            "repo_identity_set_preview_noop",
            &plain.root,
            IDENTITY_PREVIEW_SCHEMA,
            &noop,
        );
    }
}

/// What Git would not hand back as it was given, and what no commit can
/// carry: each is refused before anything is written.
mod validation {
    use manyhands::{
        repository::{
            ConfirmationId, Mutation, RepoIdentitySetInput,
            request_store::{DigestSalt, FieldValue, IntentDigest, NewConfirmation, ScopeKey},
        },
        results::{Outcome, ResultCode},
    };

    use super::{
        super::support::{
            confirmation::{Desk, START},
            items::TICKET_A,
            mutation::{self, REQUEST_1, REQUEST_2, World},
        },
        boundary::{EMAIL, NAME, details},
        *,
    };

    /// The names and emails that are refused, each with why.
    fn refused() -> Vec<(&'static str, String, String)> {
        let name = |what, name: &str| (what, name.to_owned(), EMAIL.to_owned());
        let email = |what, email: &str| (what, NAME.to_owned(), email.to_owned());
        vec![
            name("a name with a space before it", " Ada Example"),
            name("a name with a space after it", "Ada Example "),
            name("a name that ends in a carriage return", "Ada Example\r"),
            name("a name that ends in a form feed", "Ada Example\u{c}"),
            name("a name that ends in a no-break space", "Ada Example\u{a0}"),
            name("a name that holds a newline", "Ada\nExample"),
            name("a name that holds a tab", "Ada\tExample"),
            name("a name that is a space", " "),
            name("a name no commit can carry", "Ada <ops>"),
            name("a name that is only punctuation Git drops", ";"),
            name("a name that ends in a quote, which Git drops", "Ada \"Q\""),
            name(
                "a name that ends in a comma, which Git drops",
                "Ada Example,",
            ),
            email("an email with a space after it", "ada@example.invalid "),
            email("an email that holds a newline", "ada@example.invalid\nx"),
            email("an email in angle brackets", "<ada@example.invalid>"),
        ]
    }

    fn set(root: &Path, name: &str, email: &str) -> Mutation {
        Mutation::RepoIdentitySet(RepoIdentitySetInput {
            root: root.to_owned(),
            name: name.to_owned(),
            email: email.to_owned(),
            initials: InitialsChange::Unchanged,
        })
    }

    #[test]
    fn a_value_git_would_not_hand_back_or_no_commit_can_carry_is_refused_by_the_operation() {
        git_home::isolated();
        let plain = Plain::new();
        let (service, _data) = service();
        let before = local_file(&plain.root);
        for (what, name, email) in refused() {
            let error = service
                .set_local_identity(request(&plain.root, &name, &email))
                .unwrap_err();
            assert_eq!(error.kind, RepositoryErrorKind::InvalidIdentity, "{what}");
        }
        assert_eq!(local_file(&plain.root), before);
    }

    #[test]
    fn a_value_git_would_not_hand_back_or_no_commit_can_carry_is_invalid_input_at_prepare() {
        git_home::isolated();
        let plain = Plain::new();
        let desk = Desk::new();
        let before = local_file(&plain.root);
        for (what, name, email) in refused() {
            let prepared = desk.prepare(&set(&plain.root, &name, &email));
            assert_eq!(prepared.code, ResultCode::InvalidInput, "{what}");
            assert_eq!(prepared.outcome, Outcome::Error, "{what}");
            assert!(prepared.data.is_none(), "{what}");
        }
        assert_eq!(desk.confirmation_rows(), 0);
        assert_eq!(local_file(&plain.root), before);
    }

    /// No preview is given of such a request, so no confirmation is its
    /// own. One is recorded here by hand, as a preview of the request
    /// would have recorded it, so that `execute` gets as far as the
    /// binding's own check.
    #[test]
    fn a_value_git_would_not_hand_back_or_no_commit_can_carry_is_invalid_input_at_execute() {
        git_home::isolated();
        let plain = Plain::new();
        let desk = Desk::new();
        let before = local_file(&plain.root);
        let scope = ScopeKey::for_repository(&plain.root).unwrap();
        for (what, name, email) in refused() {
            let confirmation_id = ConfirmationId::new();
            let recorded = desk
                .service
                .insert_confirmation(&NewConfirmation {
                    confirmation_id,
                    scope: scope.clone(),
                    command: "repo identity-set".to_owned(),
                    target: String::new(),
                    intent_digest: IntentDigest::builder(
                        DigestSalt::Confirmation(confirmation_id),
                        "repo identity-set",
                        &scope,
                        "",
                    )
                    .field("name", FieldValue::Text(&name))
                    .field("email", FieldValue::Text(&email))
                    .field("initials", FieldValue::Absent)
                    .finish(),
                    observation_digest: Some("0".repeat(64)),
                    expires_at: START + 600,
                })
                .unwrap();
            assert!(recorded);

            let executed = desk.execute(
                REQUEST_1,
                &confirmation_id.to_string(),
                set(&plain.root, &name, &email),
            );

            assert_eq!(executed.code, ResultCode::InvalidInput, "{what}");
            assert_eq!(executed.outcome, Outcome::Error, "{what}");
            desk.assert_nothing_left(REQUEST_1);
        }
        assert_eq!(local_file(&plain.root), before);
    }

    /// Unusual, and valid: Git hands each back as it was given, and a
    /// commit carries it.
    #[test]
    fn an_unusual_name_that_git_hands_back_unchanged_is_written_and_commits() {
        git_home::isolated();
        let world = World::new();
        let name = "Åsa \"Q\" O'Neil  #1; née Ödman Jr.";
        let email = "åsa+q#1@example.invalid";
        let request = || set(&world.root, name, email);

        let preview = world.service.prepare(&request());
        assert_eq!(preview.code, ResultCode::Ok, "{}", preview.message);
        let preview = preview.data.unwrap();
        assert_eq!(details(&preview).name, name);
        let done = mutation::execute_confirmed(
            &world.service,
            REQUEST_1,
            &preview.confirmation_id,
            request(),
        );

        assert_eq!(done.code, ResultCode::Ok, "{}", done.message);
        assert_eq!(local(&world.root), held(name, email, None));
        // What was written is what is asked: the same request is a no-op.
        assert_eq!(
            world.service.prepare(&request()).code,
            ResultCode::AlreadyApplied
        );
        // And a save commits as it.
        let saved = world.execute(
            REQUEST_2,
            Mutation::TicketSave(manyhands::repository::TicketSaveInput {
                draft: World::draft(mutation::TITLE, mutation::BODY),
                ..world.save_input(TICKET_A, &world.token(TICKET_A))
            }),
        );
        assert_eq!(saved.outcome, Outcome::Success, "{}", saved.message);
        let repository = git2::Repository::open(&world.root).unwrap();
        let commit = repository
            .find_commit(world.branch_tip(TICKET_A).unwrap())
            .unwrap();
        assert_eq!(commit.author().name(), Some(name));
        assert_eq!(commit.author().email(), Some(email));
    }
}

/// A repository whose own configuration does not say one thing of a key
/// the command sets: the key is there twice, or comes from a file the
/// configuration includes. Git refuses to set such a key, so no preview
/// is given of it.
mod ambiguous {
    use manyhands::results::{Outcome, ResultCode};

    use super::{
        super::support::confirmation::Desk,
        boundary::{EMAIL, NAME, set, set_with},
        *,
    };

    fn append(root: &Path, source: &str) {
        let mut config = local_file(root);
        config.extend_from_slice(source.as_bytes());
        fs::write(root.join(".git/config"), config).unwrap();
    }

    /// What a scenario does to a repository before its request.
    type Arrange = fn(&Path);

    /// Each way a key can be said twice, or elsewhere.
    fn arrangements() -> Vec<(&'static str, Arrange)> {
        vec![
            ("a name that is there twice", |root| {
                append(root, "[user]\n\tname = Second Name\n");
            }),
            ("an email that is there twice", |root| {
                append(root, "[user]\n\temail = second@example.invalid\n");
            }),
            ("initials that are there twice", |root| {
                append(root, "[manyhands]\n\tinitials = aa\n\tinitials = bb\n");
            }),
            ("an email from an included file", |root| {
                let mut own = git2::Config::open(&root.join(".git/config")).unwrap();
                own.remove("user.email").unwrap();
                drop(own);
                fs::write(
                    root.join(".git/identity.inc"),
                    "[user]\n\temail = included@example.invalid\n",
                )
                .unwrap();
                append(root, "[include]\n\tpath = identity.inc\n");
            }),
            (
                "a name the file holds and an included file holds too",
                |root| {
                    fs::write(
                        root.join(".git/identity.inc"),
                        format!("[user]\n\tname = {NAME}\n"),
                    )
                    .unwrap();
                    append(root, "[include]\n\tpath = identity.inc\n");
                },
            ),
        ]
    }

    #[test]
    fn a_key_said_twice_or_in_an_included_file_gets_no_preview() {
        git_home::isolated();
        for (what, arrange) in arrangements() {
            let plain = Plain::new();
            arrange(&plain.root);
            let desk = Desk::new();
            let before = local_file(&plain.root);

            for request in [
                set(&plain.root),
                set_with(&plain.root, InitialsChange::Set("ae".to_owned())),
            ] {
                let prepared = desk.prepare(&request);
                assert_eq!(prepared.code, ResultCode::RecoveryRequired, "{what}");
                assert_eq!(prepared.outcome, Outcome::Blocked, "{what}");
                assert!(prepared.data.is_none(), "{what}");
                assert_eq!(recovery_actions_of(&prepared), ["repo.inspect"], "{what}");
            }
            assert_eq!(desk.confirmation_rows(), 0, "{what}");
            assert_eq!(local_file(&plain.root), before, "{what}");
        }
    }

    fn recovery_actions_of<T>(envelope: &manyhands::results::Envelope<T>) -> Vec<&'static str> {
        envelope
            .recovery
            .iter()
            .map(|action| action.action.as_str())
            .collect()
    }

    #[test]
    fn a_key_said_twice_or_in_an_included_file_is_refused_by_the_operation() {
        git_home::isolated();
        for (what, arrange) in arrangements() {
            let plain = Plain::new();
            arrange(&plain.root);
            let (service, _data) = service();
            let before = local_file(&plain.root);

            let error = service
                .set_local_identity(SetLocalIdentityRequest {
                    initials: InitialsChange::Set("ae".to_owned()),
                    ..request(&plain.root, NAME, EMAIL)
                })
                .unwrap_err();

            assert_eq!(error.kind, RepositoryErrorKind::RecoveryRequired, "{what}");
            assert_eq!(local_file(&plain.root), before, "{what}");
        }
    }

    /// A key that is said once, in the file itself, beside an include
    /// that says other things: the command writes as it does anywhere.
    #[test]
    fn an_included_file_that_says_nothing_of_the_identity_is_no_obstacle() {
        git_home::isolated();
        let plain = Plain::new();
        fs::write(plain.root.join(".git/other.inc"), "[core]\n\tabbrev = 12\n").unwrap();
        append(&plain.root, "[include]\n\tpath = other.inc\n");
        let (service, _data) = service();

        let outcome = service
            .set_local_identity(request(&plain.root, NAME, EMAIL))
            .unwrap();

        assert!(matches!(outcome, SetLocalIdentityOutcome::Changed(_)));
        assert_eq!(local(&plain.root), held(NAME, EMAIL, None));
        let written = String::from_utf8(local_file(&plain.root)).unwrap();
        assert!(written.contains("path = other.inc"), "{written}");
    }
}

/// The write is one replacement of the configuration file under Git's
/// own lock on it, `config.lock`: nothing another program wrote is ever
/// put back over, and an identity is never half set.
mod atomic {
    use std::sync::{Arc, Mutex};

    use manyhands::results::{Outcome, ResultCode};

    use super::{
        super::support::{
            confirmation::Desk,
            mutation::{REQUEST_1, recovery_actions},
        },
        boundary::{EMAIL, NAME, assert_set, by_hand, set},
        *,
    };

    const URL: &str = "https://example.invalid/team/project.git";

    fn lock_file(root: &Path) -> std::path::PathBuf {
        root.join(".git/config.lock")
    }

    /// The names of what the repository's Git directory holds, but for
    /// the file Manyhands' own lease is taken on.
    fn git_directory(root: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(root.join(".git"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .filter(|name| name != "manyhands-operation.lock")
            .collect();
        names.sort();
        names
    }

    #[test]
    fn a_configuration_lock_someone_left_is_busy_and_is_not_removed() {
        git_home::isolated();
        let plain = Plain::new();
        let desk = Desk::new();
        let (confirmation, _) = desk.preview(&set(&plain.root));
        let before = local_file(&plain.root);
        fs::write(lock_file(&plain.root), "left behind").unwrap();

        // The operation itself.
        let error = desk
            .service
            .set_local_identity(request(&plain.root, NAME, EMAIL))
            .unwrap_err();
        assert_eq!(error.kind, RepositoryErrorKind::RepositoryBusy);
        // And through the boundary.
        let busy = desk.execute(REQUEST_1, &confirmation, set(&plain.root));

        assert_eq!(busy.code, ResultCode::Busy);
        assert_eq!(busy.outcome, Outcome::Error);
        assert_eq!(recovery_actions(&busy), ["request.retry"]);
        assert_eq!(local_file(&plain.root), before);
        assert_eq!(
            fs::read(lock_file(&plain.root)).unwrap(),
            b"left behind",
            "the lock is its owner's to remove"
        );
        desk.assert_nothing_left(REQUEST_1);
        // Once it is gone, the same request writes.
        fs::remove_file(lock_file(&plain.root)).unwrap();
        assert_set(
            &desk.execute(REQUEST_1, &confirmation, set(&plain.root)),
            None,
        );
        assert!(!lock_file(&plain.root).exists());
    }

    /// A mode is a Unix thing: the test is Unix only.
    #[cfg(unix)]
    #[test]
    fn the_configuration_file_keeps_its_mode() {
        use std::os::unix::fs::PermissionsExt;

        git_home::isolated();
        for mode in [0o640, 0o664, 0o600] {
            let plain = Plain::new();
            let path = plain.root.join(".git/config");
            fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
            let (service, _data) = service();

            let outcome = service
                .set_local_identity(SetLocalIdentityRequest {
                    initials: InitialsChange::Set("ae".to_owned()),
                    ..request(&plain.root, NAME, EMAIL)
                })
                .unwrap();

            assert!(matches!(outcome, SetLocalIdentityOutcome::Changed(_)));
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o7777,
                mode,
                "{mode:o}"
            );
        }
    }

    #[test]
    fn a_change_git_makes_while_the_identity_is_being_written_is_refused_and_not_lost() {
        git_home::isolated();
        let plain = Plain::new();
        let (service, _data) = service();
        let other = {
            let root = plain.root.clone();
            move || {
                git2::Config::open(&root.join(".git/config"))
                    .and_then(|mut config| config.set_str("remote.origin.url", URL))
            }
        };
        // Another program writes the configuration while this one holds
        // its lock, as `git remote add` would.
        let attempt = Arc::new(Mutex::new(None));
        let held = {
            let (attempt, other, root) = (attempt.clone(), other.clone(), plain.root.clone());
            move || {
                let locked = lock_file(&root).exists();
                *attempt.lock().unwrap() = Some((locked, other().map_err(|error| error.class())));
            }
        };
        service.set_local_identity_lock_hook_for_testing(held);

        let outcome = service
            .set_local_identity(request(&plain.root, NAME, EMAIL))
            .unwrap();

        assert!(matches!(outcome, SetLocalIdentityOutcome::Changed(_)));
        let (locked, refused) = attempt.lock().unwrap().take().expect("the hook ran");
        assert!(locked, "the configuration is locked as Git locks it");
        // Git refuses the other program: its change is not made, and so
        // not made and then lost.
        assert!(refused.is_err(), "{refused:?}");
        assert_eq!(local(&plain.root), held_identity());
        assert_eq!(plain.local_config("remote.origin.url"), None);
        assert!(!lock_file(&plain.root).exists());
        // It tries again, as it would, and both are there.
        other().unwrap();
        assert_eq!(
            plain.local_config("remote.origin.url").as_deref(),
            Some(URL)
        );
        assert_eq!(local(&plain.root), held_identity());
    }

    fn held_identity() -> LocalIdentity {
        held(NAME, EMAIL, None)
    }

    #[test]
    fn a_change_to_another_key_after_the_preview_is_kept() {
        git_home::isolated();
        let plain = Plain::new();
        let desk = Desk::new();
        let (confirmation, _) = desk.preview(&set(&plain.root));
        let root = plain.root.clone();
        // Runs once the request is accepted, before its domain call.
        desk.service
            .set_request_hook_for_testing(move || by_hand(&root, "remote.origin.url", URL));

        let done = desk.execute(REQUEST_1, &confirmation, set(&plain.root));

        assert_set(&done, None);
        assert_eq!(
            plain.local_config("remote.origin.url").as_deref(),
            Some(URL)
        );
        assert_eq!(local(&plain.root), held_identity());
    }

    #[test]
    fn a_write_that_fails_part_way_never_touched_the_configuration_file() {
        git_home::isolated();
        let plain = Plain::new();
        let data = tempfile::tempdir().unwrap();
        let service = RepositoryService::open_at_with_failure_point_for_testing(
            data.path(),
            FailurePoint::AfterLocalIdentityName,
        )
        .unwrap();
        // The lease's file is there before, so that the listing compares.
        drop(
            RepositoryService::hold_lease_for_testing(
                &plain.root,
                data.path(),
                manyhands::repository::LeaseKind::Repository,
            )
            .unwrap(),
        );
        let path = plain.root.join(".git/config");
        let before = (local_file(&plain.root), git_directory(&plain.root));
        let seen = Arc::new(Mutex::new(None));
        {
            // At the moment of the failure: the name is set in what will
            // be written, and the file itself holds none of it.
            let (seen, path) = (seen.clone(), path.clone());
            service.set_local_identity_lock_hook_for_testing(move || {
                *seen.lock().unwrap() = Some(fs::read(path).unwrap());
            });
        }

        let error = service
            .set_local_identity(request(&plain.root, NAME, EMAIL))
            .unwrap_err();

        assert_eq!(error.kind, RepositoryErrorKind::InjectedFailure);
        assert_eq!(seen.lock().unwrap().as_ref(), Some(&before.0));
        assert_eq!(local_file(&plain.root), before.0);
        assert_eq!(
            git_directory(&plain.root),
            before.1,
            "no lock and no scratch file is left"
        );
        // The call again finds the file as it was, and writes.
        assert!(matches!(
            service
                .set_local_identity(request(&plain.root, NAME, EMAIL))
                .unwrap(),
            SetLocalIdentityOutcome::Changed(_)
        ));
        assert_eq!(git_directory(&plain.root), before.1);
    }
}

/// Where the lock is taken and what the write leaves beside the
/// configuration file.
mod placement {
    use std::sync::{Arc, Mutex};

    use manyhands::results::ResultCode;

    use super::{
        super::support::{confirmation::Desk, mutation::REQUEST_1},
        boundary::{EMAIL, NAME, assert_set, set},
        *,
    };

    /// A symbolic link is a Unix thing here: the test is Unix only.
    ///
    /// Git locks a configuration file that is a link beside what the
    /// link names, and replaces that: the link stays a link.
    #[cfg(unix)]
    #[test]
    fn a_configuration_file_that_is_a_link_is_locked_and_written_where_it_points() {
        git_home::isolated();
        let plain = Plain::new();
        let shared = tempfile::tempdir().unwrap();
        let target = shared.path().canonicalize().unwrap().join("config");
        let link = plain.root.join(".git/config");
        fs::rename(&link, &target).unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();
        let desk = Desk::new();
        let (confirmation, _) = desk.preview(&set(&plain.root));
        let before = fs::read(&target).unwrap();

        // A lock beside the target is the one Git takes: busy.
        let beside_target = shared.path().join("config.lock");
        fs::write(&beside_target, "held").unwrap();
        let busy = desk.execute(REQUEST_1, &confirmation, set(&plain.root));
        assert_eq!(busy.code, ResultCode::Busy);
        assert_eq!(fs::read(&target).unwrap(), before);
        assert_eq!(fs::read(&beside_target).unwrap(), b"held");
        fs::remove_file(&beside_target).unwrap();

        // A file of that name beside the link locks nothing.
        let beside_link = plain.root.join(".git/config.lock");
        fs::write(&beside_link, "nobody's").unwrap();
        let done = desk.execute(REQUEST_1, &confirmation, set(&plain.root));

        assert_set(&done, None);
        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink(),
            "the link is still a link"
        );
        assert_eq!(fs::read_link(&link).unwrap(), target);
        let written = git2::Config::open(&target).unwrap();
        assert_eq!(written.get_string("user.name").unwrap(), NAME);
        assert_eq!(written.get_string("user.email").unwrap(), EMAIL);
        assert_eq!(fs::read(&beside_link).unwrap(), b"nobody's");
        assert!(!beside_target.exists());
    }

    /// A link that names nothing is refused: the file is the user's to
    /// put right. Unix only, as above.
    #[cfg(unix)]
    #[test]
    fn a_configuration_link_that_names_nothing_is_refused() {
        git_home::isolated();
        let plain = Plain::new();
        let link = plain.root.join(".git/config");
        fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink(plain.root.join("nowhere"), &link).unwrap();
        let (service, _data) = service();

        let error = service
            .set_local_identity(request(&plain.root, NAME, EMAIL))
            .unwrap_err();

        assert_eq!(error.kind, RepositoryErrorKind::RecoveryRequired);
        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert!(!plain.root.join("nowhere").exists());
    }

    #[test]
    fn a_repository_with_no_configuration_file_gets_one() {
        git_home::isolated();
        let plain = Plain::new();
        let path = plain.root.join(".git/config");
        fs::remove_file(&path).unwrap();
        let (service, _data) = service();

        let outcome = service
            .set_local_identity(request(&plain.root, NAME, EMAIL))
            .unwrap();

        assert_eq!(
            outcome,
            SetLocalIdentityOutcome::Changed(held(NAME, EMAIL, None))
        );
        assert_eq!(local(&plain.root), held(NAME, EMAIL, None));
        assert!(!plain.root.join(".git/config.lock").exists());
    }

    /// The entries of the repository's Git directory, each with whether
    /// it can be read or entered by anyone but its owner.
    #[cfg(unix)]
    fn entries(root: &Path) -> std::collections::BTreeMap<String, bool> {
        use std::os::unix::fs::PermissionsExt;
        fs::read_dir(root.join(".git"))
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                let mode = entry.metadata().unwrap().permissions().mode();
                (entry.file_name().into_string().unwrap(), mode & 0o077 != 0)
            })
            .collect()
    }

    /// Modes are a Unix thing: the test is Unix only.
    ///
    /// A configuration file only its owner can read holds what only its
    /// owner may read. Nothing the write makes beside it is open to
    /// anyone else, while it runs or after.
    #[cfg(unix)]
    #[test]
    fn nothing_the_write_makes_is_open_to_anyone_but_the_owner() {
        use std::os::unix::fs::PermissionsExt;

        git_home::isolated();
        for fail in [false, true] {
            let plain = Plain::new();
            let path = plain.root.join(".git/config");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
            let data = tempfile::tempdir().unwrap();
            let service = if fail {
                RepositoryService::open_at_with_failure_point_for_testing(
                    data.path(),
                    FailurePoint::AfterLocalIdentityName,
                )
                .unwrap()
            } else {
                RepositoryService::open_at(data.path()).unwrap()
            };
            // The lease's file is there before, so that the listings compare.
            drop(
                RepositoryService::hold_lease_for_testing(
                    &plain.root,
                    data.path(),
                    manyhands::repository::LeaseKind::Repository,
                )
                .unwrap(),
            );
            let before = entries(&plain.root);
            let during = Arc::new(Mutex::new(None));
            {
                let (during, root) = (during.clone(), plain.root.clone());
                // With the lock held, and the name set in the copy.
                service.set_local_identity_lock_hook_for_testing(move || {
                    *during.lock().unwrap() = Some(entries(&root));
                });
            }

            let result = service.set_local_identity(request(&plain.root, NAME, EMAIL));

            assert_eq!(result.is_err(), fail);
            let during = during.lock().unwrap().take().expect("the hook ran");
            let made: Vec<_> = during
                .iter()
                .filter(|(name, _)| !before.contains_key(*name))
                .collect();
            // The lock, and what the copy is made in.
            assert_eq!(made.len(), 2, "{made:?}");
            assert!(made.iter().all(|(_, open)| !**open), "{made:?}");
            assert_eq!(entries(&plain.root), before, "nothing is left");
        }
    }

    /// The compare of the file's bytes before it is replaced: a program
    /// that does not keep to Git's lock wrote the file while this one
    /// held it.
    #[test]
    fn a_file_rewritten_by_a_program_that_ignores_the_lock_is_not_written_over() {
        git_home::isolated();
        let plain = Plain::new();
        let (service, _data) = service();
        let path = plain.root.join(".git/config");
        let mut rewritten = local_file(&plain.root);
        rewritten
            .extend_from_slice(b"[remote \"origin\"]\n\turl = https://example.invalid/x.git\n");
        {
            let (path, rewritten) = (path.clone(), rewritten.clone());
            service.set_local_identity_lock_hook_for_testing(move || {
                fs::write(path, rewritten).unwrap();
            });
        }

        let error = service
            .set_local_identity(request(&plain.root, NAME, EMAIL))
            .unwrap_err();

        assert_eq!(error.kind, RepositoryErrorKind::ExternalChange);
        assert_eq!(local_file(&plain.root), rewritten, "the other write stands");
        assert!(!plain.root.join(".git/config.lock").exists());
    }
}
