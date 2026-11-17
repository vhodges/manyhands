//! `01M5MK57`: promotion refuses an identity no commit can carry before its
//! record, and existing records and replays stay as they are.
use super::*;

const BRACKETED: &str = "Example <example@example.invalid>";
/// Git drops a trailing comma: the commit would carry another name.
const TRIMMED: &str = "Example Person,";
const IDENTITY_EMAIL: &str = "example@example.invalid";

fn set_identity(side: &Side, name: &str, email: &str) -> Result<(), FixtureError> {
    let mut config = fixed(side.repo()?.config())?;
    fixed(config.set_str("user.name", name))?;
    fixed(config.set_str("user.email", email))
}

fn remove_identity(side: &Side) -> Result<(), FixtureError> {
    let mut config = fixed(side.repo()?.config())?;
    fixed(config.remove("user.name"))?;
    fixed(config.remove("user.email"))
}

/// The configuration, every ref here and on the server, and every row of the
/// lifecycle's and the synchronization's journals.
fn image(p: &Pair, side: &Side) -> Result<Vec<String>, FixtureError> {
    let repo = side.repo()?;
    let mut image = vec![format!(
        "{:?}",
        fixed(std::fs::read(repo.path().join("config")))?
    )];
    for repository in [
        repo,
        fixed(git2::Repository::open_bare(p.server.repository_path()))?,
    ] {
        for reference in fixed(repository.references())? {
            let reference = fixed(reference)?;
            image.push(format!("{:?} {:?}", reference.name(), reference.target()));
        }
    }
    let db = side.db()?;
    for table in [
        "document_promotions",
        "remote_operation_records",
        "remote_identity_confirmations",
    ] {
        let mut rows = fixed(db.prepare(&format!("SELECT * FROM {table}")))?;
        let columns = rows.column_count();
        let mut found = fixed(rows.query([]))?;
        while let Some(row) = fixed(found.next())? {
            let mut line = table.to_owned();
            for column in 0..columns {
                let value: rusqlite::types::Value = fixed(row.get(column))?;
                line.push_str(&format!(" {value:?}"));
            }
            image.push(line);
        }
    }
    Ok(image)
}

/// A confirmed request meets an identity no commit can carry: it counts as
/// none, and nothing at all has happened.
pub(super) fn configured(name: &str) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.save(&p.a)?;
    let request = p.request()?;
    set_identity(&p.a, name, IDENTITY_EMAIL)?;
    let before = image(&p, &p.a)?;
    let connections = p.server.accepted_sockets();
    assert!(matches!(
        p.execute(&p.a.service, request),
        Err(DocumentPromotionError::IdentityRequired { .. })
    ));
    assert_eq!(image(&p, &p.a)?, before);
    assert_eq!(p.server.accepted_sockets(), connections);
    Ok(())
}

/// A confirmation of the same: `ExternalChange`, and nothing is persisted.
pub(super) fn confirmed() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.save(&p.a)?;
    let request = p.request()?;
    remove_identity(&p.a)?;
    let mut asked = request.clone();
    asked.operation_id = OperationId::new();
    let Err(DocumentPromotionError::IdentityRequired {
        expected_configuration,
    }) = p.execute(&p.a.service, asked)
    else {
        return Err(FixtureError);
    };
    let before = image(&p, &p.a)?;
    for name in [BRACKETED, TRIMMED] {
        let mut confirmed = request.clone();
        confirmed.operation_id = OperationId::new();
        confirmed.confirmed_identity = Some(ConfirmedCommitIdentity {
            confirmation_id: OperationId::new(),
            identity: CommitIdentity {
                name: name.into(),
                email: IDENTITY_EMAIL.into(),
            },
            expected_configuration,
        });
        assert!(matches!(
            p.execute(&p.a.service, confirmed),
            Err(DocumentPromotionError::ExternalChange)
        ));
        assert_eq!(image(&p, &p.a)?, before);
    }
    Ok(())
}

/// The control: a plain configured identity signs the merge.
pub(super) fn plain() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.save(&p.a)?;
    set_identity(&p.a, "Example Person", IDENTITY_EMAIL)?;
    p.promote()?;
    let repo = p.a.repo()?;
    let merge = fixed(repo.find_commit(fixed(repo.refname_to_id(MAIN))?))?;
    assert_eq!(merge.parent_count(), 2);
    assert_eq!(merge.author().name(), Some("Example Person"));
    Ok(())
}

/// A finished request keeps its stored outcome, and one with a record goes on
/// to the run's own stop, then continues once a usable identity is back.
pub(super) fn kept() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.save(&p.a)?;
    let request = p.request()?;
    let failing = fixed(RepositoryService::open_at_with_failure_point_for_testing(
        &p.a.data,
        FailurePoint::AfterContextSync,
    ))?;
    assert!(p.execute(&failing, request.clone()).is_err());
    set_identity(&p.a, BRACKETED, IDENTITY_EMAIL)?;
    assert!(matches!(
        p.execute(&p.a.service, request.clone()),
        Err(DocumentPromotionError::IdentityRequired { .. })
    ));
    let main = fixed(p.a.repo()?.refname_to_id(MAIN))?;
    set_identity(&p.a, "Example Person", IDENTITY_EMAIL)?;
    let DocumentPromotionResult::Complete(first) = p
        .execute(&p.a.service, request.clone())
        .map_err(|_| FixtureError)?
    else {
        return Err(FixtureError);
    };
    let merged = fixed(p.a.repo()?.refname_to_id(MAIN))?;
    assert_ne!(merged, main);
    set_identity(&p.a, BRACKETED, IDENTITY_EMAIL)?;
    let DocumentPromotionResult::Complete(replayed) =
        p.execute(&p.a.service, request).map_err(|_| FixtureError)?
    else {
        return Err(FixtureError);
    };
    assert_eq!(first.integration(), replayed.integration());
    assert_eq!(first.publication(), replayed.publication());
    assert_eq!(fixed(p.a.repo()?.refname_to_id(MAIN))?, merged);
    Ok(())
}

pub(super) const BRACKETED_NAME: &str = BRACKETED;
pub(super) const TRIMMED_NAME: &str = TRIMMED;
