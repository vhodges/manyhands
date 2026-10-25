//! `repo create` and `repo enable` with an identity Git cannot commit as:
//! refused at the preview, before anything is written or confirmed. A
//! supplied one is `invalid_input`; one from Git's configuration is no
//! identity, so `identity_required`.

use super::*;

const EMAIL: &str = "example@example.invalid";

fn bracketed() -> [(&'static str, CommitIdentity); 2] {
    [
        (
            "a bracketed name",
            CommitIdentity {
                name: format!("Example Person <{EMAIL}>"),
                email: EMAIL.to_owned(),
            },
        ),
        (
            "a bracketed email",
            CommitIdentity {
                name: "Example Person".to_owned(),
                email: format!("<{EMAIL}>"),
            },
        ),
    ]
}

fn write_bracketed_global_identity(home: &Path) -> Vec<u8> {
    let source = "[user]\n\tname = \"Example Person <example@example.invalid>\"\n\
                  \temail = example@example.invalid\n";
    fs::write(home.join(".gitconfig"), source).unwrap();
    assert_eq!(global_configuration().as_deref(), Some(source.as_bytes()));
    source.as_bytes().to_vec()
}

/// The owner's case: a supplied name holding an email in angle brackets.
/// The plain name beside it previews and creates.
#[test]
fn repo_create_with_an_identity_git_cannot_commit_is_invalid_input_at_the_preview() {
    let desk = Desk::new();
    let site = Site::new();
    let before = desk.surroundings(site.parent.path());

    for (what, identity) in bracketed() {
        let refused = desk.prepare(&Mutation::RepoCreate(RepoCreateInput {
            identity: Some(identity.clone()),
            ..site.input()
        }));

        assert_eq!(refused.code, ResultCode::InvalidInput, "{what}");
        assert_eq!(refused.outcome, Outcome::Error, "{what}");
        assert_eq!(refused.data, None, "{what}");
        assert_eq!(desk.confirmation_rows(), 0, "{what}");
        assert_eq!(desk.surroundings(site.parent.path()), before, "{what}");
        assert!(!site.root.exists(), "{what}");
    }

    let plain = || {
        Mutation::RepoCreate(RepoCreateInput {
            identity: Some(CommitIdentity {
                name: "Example Person".to_owned(),
                email: EMAIL.to_owned(),
            }),
            ..site.input()
        })
    };
    let (confirmation, _) = desk.preview(&plain());
    let created = desk.execute(REQUEST_1, &confirmation, plain());
    assert_enabled(&created, &site.root);
    assert_eq!(
        local_config(&site.root, "user.name").as_deref(),
        Some("Example Person")
    );
    assert_eq!(desk.pending_journal_rows(), 0);
}

#[test]
fn repo_enable_with_a_supplied_identity_git_cannot_commit_is_invalid_input_at_the_preview() {
    let desk = Desk::new();
    let plain = Plain::new();
    let before = desk.surroundings(&plain.root);

    for (what, identity) in bracketed() {
        let refused = desk.prepare(&Mutation::RepoEnable(RepoEnableInput {
            identity: Some(identity.clone()),
            ..plain.input()
        }));

        assert_eq!(refused.code, ResultCode::InvalidInput, "{what}");
        assert_eq!(refused.data, None, "{what}");
        assert_eq!(desk.confirmation_rows(), 0, "{what}");
        assert_eq!(desk.surroundings(&plain.root), before, "{what}");
    }
}

#[test]
fn repo_create_with_a_global_identity_git_cannot_commit_is_identity_required() {
    let Some(home) = in_a_process_with_its_own_git_home(
        "unusable_identity::repo_create_with_a_global_identity_git_cannot_commit_is_identity_required",
    ) else {
        return;
    };
    let global = write_bracketed_global_identity(&home);
    let desk = Desk::new();
    let site = Site::new();
    let before = desk.surroundings(site.parent.path());

    let refused = desk.prepare(&Mutation::RepoCreate(RepoCreateInput {
        identity: None,
        ..site.input()
    }));

    assert_eq!(
        refused.code,
        ResultCode::IdentityRequired,
        "{}",
        refused.message
    );
    assert_eq!(refused.outcome, Outcome::Blocked);
    assert_eq!(refused.data, None);
    assert_eq!(desk.confirmation_rows(), 0);
    assert_eq!(desk.surroundings(site.parent.path()), before);
    assert!(!site.root.exists());
    assert_eq!(global_configuration(), Some(global));
}

#[test]
fn repo_enable_with_a_global_identity_git_cannot_commit_is_identity_required() {
    let Some(home) = in_a_process_with_its_own_git_home(
        "unusable_identity::repo_enable_with_a_global_identity_git_cannot_commit_is_identity_required",
    ) else {
        return;
    };
    let global = write_bracketed_global_identity(&home);
    let desk = Desk::new();
    let fixture = support::repository_without_local_identity();
    let plain = Plain {
        root: fixture.root.canonicalize().unwrap(),
        fixture,
    };
    let before = desk.surroundings(&plain.root);

    let refused = desk.prepare(&plain.enable());

    assert_eq!(
        refused.code,
        ResultCode::IdentityRequired,
        "{}",
        refused.message
    );
    assert_eq!(refused.outcome, Outcome::Blocked);
    assert_eq!(refused.data, None);
    assert_eq!(recovery_actions(&refused), ["repo.identity_set"]);
    assert_eq!(desk.confirmation_rows(), 0);
    assert_eq!(desk.surroundings(&plain.root), before);
    assert_eq!(global_configuration(), Some(global));
}
