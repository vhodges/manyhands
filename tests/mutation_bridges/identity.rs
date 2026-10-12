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
    fn a_failure_after_the_first_key_puts_the_configuration_file_back() {
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
        assert_eq!(local_file(&plain.root), source, "restored byte for byte");
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
