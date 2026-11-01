//! `01M5MK57`: closure treats an identity no commit can carry as none. A
//! preview says so; an execution past that check (R-21) ends in one terminal
//! refused row, and existing records and replays stay as they are.
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

fn unfinished_rows(side: &Side) -> Result<i64, FixtureError> {
    fixed(side.db()?.query_row(
        "SELECT count(*) FROM ticket_closures WHERE step NOT IN ('abandoned','complete','superseded')",
        [],
        |row| row.get(0),
    ))
}

fn operations(side: &Side) -> Result<i64, FixtureError> {
    fixed(
        side.db()?
            .query_row("SELECT count(*) FROM remote_operation_records", [], |row| {
                row.get(0)
            }),
    )
}

/// Configured: the preview's one blocker, and the race past it refused with
/// one terminal row and no other effect.
pub(super) fn configured(name: &str) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.edit(&p.a, BODY)?;
    let request = p.request(&p.a)?;
    set_identity(&p.a, name, IDENTITY_EMAIL)?;
    p.server.wait_for_helper_audits()?;
    let before = all_images(&p)?;
    let operations_before = operations(&p.a)?;
    let connections = p.server.accepted_sockets();
    let TicketClosurePreflight::Blocked {
        blockers,
        identity_configuration,
    } = p.preflight(&p.a)?
    else {
        return Err(FixtureError);
    };
    assert_eq!(blockers, [ClosureBlocker::IdentityRequired]);
    assert!(identity_configuration.is_some());
    assert!(matches!(
        p.execute(&p.a.service, request.clone()),
        Err(TicketClosureError::ExternalChange)
    ));
    assert_eq!(step(&p.a, request.operation_id)?, "abandoned");
    assert_eq!(unfinished_rows(&p.a)?, 0);
    assert_eq!(operations(&p.a)?, operations_before);
    assert_eq!(p.server.accepted_sockets(), connections);
    assert_eq!(all_images(&p)?, before);
    Ok(())
}

/// Confirmed: blocked, and refused with nothing persisted.
pub(super) fn confirmed() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.edit(&p.a, BODY)?;
    let request = p.request(&p.a)?;
    {
        let mut config = fixed(p.a.repo()?.config())?;
        fixed(config.remove("user.name"))?;
        fixed(config.remove("user.email"))?;
    }
    let TicketClosurePreflight::Blocked {
        identity_configuration: Some(expected_configuration),
        ..
    } = p.preflight(&p.a)?
    else {
        return Err(FixtureError);
    };
    p.server.wait_for_helper_audits()?;
    let before = all_images(&p)?;
    let operations_before = operations(&p.a)?;
    for name in [BRACKETED, TRIMMED] {
        let confirmation = ConfirmedCommitIdentity {
            confirmation_id: OperationId::new(),
            identity: CommitIdentity {
                name: name.into(),
                email: IDENTITY_EMAIL.into(),
            },
            expected_configuration,
        };
        let preflight = fixed(
            p.a.service
                .preview_ticket_closure(PreviewTicketClosureRequest {
                    root: p.a.root.clone(),
                    item_id: item(),
                    confirmed_identity: Some(confirmation.clone()),
                }),
        )?;
        assert!(matches!(
            preflight,
            TicketClosurePreflight::Blocked { ref blockers, .. }
                if blockers == &[ClosureBlocker::IdentityRequired]
        ));
        let mut confirmed = request.clone();
        confirmed.operation_id = OperationId::new();
        confirmed.confirmed_identity = Some(confirmation);
        assert!(matches!(
            p.execute(&p.a.service, confirmed.clone()),
            Err(TicketClosureError::ExternalChange)
        ));
        assert_eq!(step(&p.a, confirmed.operation_id)?, "abandoned");
        assert_eq!(unfinished_rows(&p.a)?, 0);
        assert_eq!(operations(&p.a)?, operations_before);
        assert_eq!(all_images(&p)?, before);
    }
    Ok(())
}

/// The control: a plain configured identity closes and signs.
pub(super) fn plain() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.edit(&p.a, BODY)?;
    set_identity(&p.a, "Example Person", IDENTITY_EMAIL)?;
    p.close(&p.a)?;
    let repo = p.a.repo()?;
    let merge = fixed(repo.find_commit(fixed(repo.refname_to_id(MAIN))?))?;
    assert_eq!(merge.parent_count(), 2);
    assert_eq!(merge.author().name(), Some("Example Person"));
    Ok(())
}

/// A checkpoint already written carries its own `closed_by`, so a request
/// that holds one finishes, and its replay is what it stored.
pub(super) fn kept() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.edit(&p.a, BODY)?;
    let request = p.request(&p.a)?;
    stopped(&p, &p.a, &request, FailurePoint::AfterClosureCheckpoint)?;
    set_identity(&p.a, BRACKETED, IDENTITY_EMAIL)?;
    let first = authoritative(
        p.execute(&p.a.service, request.clone())
            .map_err(|_| FixtureError)?,
    );
    assert_eq!(first.cleanup(), PromotionCleanup::Complete);
    let merged = fixed(p.a.repo()?.refname_to_id(MAIN))?;
    let replayed = authoritative(p.execute(&p.a.service, request).map_err(|_| FixtureError)?);
    assert_eq!(first.integration(), replayed.integration());
    assert_eq!(first.publication(), replayed.publication());
    assert_eq!(fixed(p.a.repo()?.refname_to_id(MAIN))?, merged);
    Ok(())
}

pub(super) const BRACKETED_NAME: &str = BRACKETED;
pub(super) const TRIMMED_NAME: &str = TRIMMED;
