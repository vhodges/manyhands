//! `closed_by` privacy: present only in canonical Markdown, Git objects and
//! `discovered_items.closed_by` (live, WAL, backup), never in a workflow row,
//! output, error or diagnostic.
use super::*;

pub(super) const CASES: &[ssh_harness::Case] = &[
    ("privacy_closed_by_checkpointed", || closed_by_policy(false)),
    ("privacy_closed_by_pinned_wal", || closed_by_policy(true)),
    ("capture_privacy_controls_fail_closed", capture_controls),
];
pub(super) const OUTPUT_CONTROLS: &[ssh_harness::Case] = &[
    ("control_closed_by_output", || {
        output_control(&closed_by(NAME, EMAIL))
    }),
    ("control_title_output", || output_control(TITLE)),
    ("control_body_output", || output_control(BODY)),
    ("control_passphrase_output", || {
        output_control(remote_world::PASSWORD)
    }),
    ("control_endpoint_output", endpoint_output_control),
];

fn operation_rows_snapshot(db: &rusqlite::Connection) -> Result<[u8; 32], FixtureError> {
    let mut query = fixed(db.prepare("SELECT * FROM operation_records ORDER BY id"))?;
    let columns = query.column_count();
    let rows = fixed(
        fixed(query.query_map([], |r| {
            (0..columns)
                .map(|c| r.get::<_, rusqlite::types::Value>(c))
                .collect::<Result<Vec<_>, _>>()
        }))?
        .collect::<Result<Vec<_>, _>>(),
    )?;
    Ok(*blake3::hash(format!("{rows:?}").as_bytes()).as_bytes())
}

/// Negative controls on a real store: `closed_by` placed transactionally in
/// an existing workflow row, and in another discovery column, is detected;
/// both are rolled back before any service call.
fn workflow_control(
    p: &Pair,
    db: &rusqlite::Connection,
    closed_by: &str,
) -> Result<(), FixtureError> {
    let exempt = p.exempt();
    let refs = exempt_refs(&exempt);
    let policy = row_policy(&refs);
    let cache = cache_probes(&p.probes);
    privacy_rows::scan_workflow_rows(db, &p.probes, &policy)?;
    privacy_rows::scan_rows(db, &cache, true, &policy)?;
    let before = operation_rows_snapshot(db)?;
    let (id, original): (i64, Option<String>) = fixed(db.query_row(
        "SELECT id,target FROM operation_records ORDER BY id LIMIT 1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ))?;
    let transaction = fixed(db.unchecked_transaction())?;
    // A real journal row's free-text target; its other constraints intact.
    assert_eq!(
        fixed(transaction.execute(
            "UPDATE operation_records SET target=?1 WHERE id=?2",
            rusqlite::params![format!("fixture/{closed_by}"), id],
        ))?,
        1
    );
    assert!(fixed(transaction.query_row(
        "SELECT instr(target,?1)>0 FROM operation_records WHERE id=?2",
        rusqlite::params![closed_by, id],
        |r| r.get::<_, bool>(0),
    ))?);
    assert_ne!(operation_rows_snapshot(&transaction)?, before);
    assert!(privacy_rows::scan_workflow_rows(&transaction, &p.probes, &policy).is_err());
    assert!(privacy_rows::scan_rows(&transaction, &cache, true, &policy).is_err());
    fixed(transaction.rollback())?;
    assert_eq!(operation_rows_snapshot(db)?, before);
    assert_eq!(
        fixed(db.query_row(
            "SELECT target FROM operation_records WHERE id=?1",
            [id],
            |r| r.get::<_, Option<String>>(0),
        ))?,
        original
    );
    // The exemption is column-scoped: the same value in `title` is a leak.
    let transaction = fixed(db.unchecked_transaction())?;
    assert_eq!(
        fixed(transaction.execute(
            "UPDATE discovered_items SET title=?1 WHERE closed_by=?1",
            [closed_by],
        ))?,
        1
    );
    assert!(privacy_rows::scan_rows(&transaction, &cache, true, &policy).is_err());
    fixed(transaction.rollback())?;
    // The bare email outside its exempt `closed_by` cell is a leak too.
    scan_identity_parts(db, &exempt)?;
    let transaction = fixed(db.unchecked_transaction())?;
    assert_eq!(
        fixed(transaction.execute(
            "UPDATE discovered_items SET title=?1 WHERE closed_by=?2",
            [EMAIL, closed_by],
        ))?,
        1
    );
    assert!(scan_identity_parts(&transaction, &exempt).is_err());
    fixed(transaction.rollback())?;
    scan_identity_parts(db, &exempt)?;
    privacy_rows::scan_workflow_rows(db, &p.probes, &policy)?;
    privacy_rows::scan_rows(db, &cache, true, &policy)
}

fn closed_by_policy(pinned: bool) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let readers = [p.a.db()?, p.b.db()?];
    if pinned {
        for reader in &readers {
            fixed(reader.execute_batch("BEGIN; SELECT COUNT(*) FROM ticket_closures;"))?;
        }
    }
    p.edit(&p.a, BODY)?;
    let outcome = p.close(&p.a)?;
    let ClosureRecording::Recorded { checkpoint_oid } = outcome.closure() else {
        return Err(FixtureError);
    };
    // B independently discovers the closure through the public poll.
    poll(&p, &p.b)?;
    if !pinned {
        for reader in &readers {
            let busy: i64 =
                fixed(reader.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| r.get(0)))?;
            assert_eq!(busy, 0);
        }
    }
    p.privacy()?;
    let by = p.a.closed_by();
    // Positive controls: Git objects and canonical Markdown hold it.
    let repo = p.a.repo()?;
    let merged = oid(&p.a, MAIN)?;
    let markdown = blob(&repo, merged, TICKET_PATH)?;
    assert!(ssh_privacy::clean(&markdown, &[by.clone().into_bytes()]).is_err());
    let checkpoint = fixed(repo.find_commit(checkpoint_oid))?;
    assert_eq!(checkpoint.author().name(), Some(NAME));
    assert_eq!(checkpoint.committer().email(), Some(EMAIL));
    assert!(
        ssh_privacy::clean(
            &fixed(std::fs::read(p.b.root.join(TICKET_PATH)))?,
            &[by.clone().into_bytes()]
        )
        .is_err()
    );
    for (side_index, side) in [&p.a, &p.b].into_iter().enumerate() {
        assert_eq!(
            fixed(side.db()?.query_row(
                "SELECT COUNT(*) FROM discovered_items WHERE closed_by=?1",
                [&by],
                |r| r.get::<_, i64>(0),
            ))?,
            1,
            "each live clone indexes the closure"
        );
        let backup = fixed(rusqlite::Connection::open(side.data.join("v")))?;
        assert_eq!(
            fixed(backup.query_row(
                "SELECT COUNT(*) FROM discovered_items WHERE closed_by=?1",
                [&by],
                |r| r.get::<_, i64>(0),
            ))?,
            1,
            "each backup holds the accepted discovery value"
        );
        // The closure record exists and holds `closed_at`, never `closed_by`
        // (every one of its columns is in the workflow scan above).
        assert_eq!(
            fixed(side.db()?.query_row(
                "SELECT COUNT(*) FROM ticket_closures WHERE closed_at IS NOT NULL",
                [],
                |r| r.get::<_, i64>(0),
            ))?,
            i64::from(side_index == 0)
        );
        let inventory = [
            REGISTRY_FILE.to_owned(),
            format!("{REGISTRY_FILE}-wal"),
            "v".into(),
        ];
        let mut copies = Vec::new();
        for (index, name) in inventory.into_iter().enumerate() {
            let path = side.data.join(name);
            let bytes = if index == 1 && !pinned && !path.exists() {
                Vec::new()
            } else {
                fixed(std::fs::read(path))?
            };
            copies.push(ssh_privacy::clean(&bytes, &[by.clone().into_bytes()]).is_err());
            ssh_harness::observation(&[
                31,
                side_index as u128,
                index as u128,
                copies[index] as u128,
            ]);
        }
        assert!(copies[2]);
        assert!(copies[usize::from(pinned)]);
    }
    // Record the positive copies before any negative control could place
    // the value into WAL or backup pages.
    drop(readers);
    let mut negative = 0;
    for side in [&p.a, &p.b] {
        workflow_control(&p, &side.db()?, &by)?;
        negative += 1;
        let backup = fixed(rusqlite::Connection::open(side.data.join("v")))?;
        workflow_control(&p, &backup, &by)?;
        negative += 1;
    }
    assert_eq!(negative, 4);
    // The raw-store part scan fails on a bare email outside a `closed_by`
    // value, and accepts the same email inside one.
    let exempt = p.exempt();
    let control = p.a.data.join("raw-control");
    fixed(std::fs::write(&control, format!("x{EMAIL}x")))?;
    assert!(scan_store_identity_parts(&p.a.data, &exempt).is_err());
    fixed(std::fs::write(&control, format!("x{by}x")))?;
    scan_store_identity_parts(&p.a.data, &exempt)?;
    fixed(std::fs::remove_file(&control))?;
    assert!(p.diagnostics(&by).is_err());
    assert!(p.diagnostics(NAME).is_err());
    assert!(p.diagnostics(EMAIL).is_err());
    p.privacy()
}

fn output_control(secret: &str) -> Result<(), FixtureError> {
    ssh_privacy::save(&[secret.as_bytes().to_vec()])?;
    // Only child-only controls print canaries; the parent scans first and
    // never forwards these bytes.
    eprintln!("{secret}");
    Ok(())
}
fn endpoint_output_control() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.privacy()?;
    eprintln!("{}", p.server.url());
    Ok(())
}
fn capture_controls() -> Result<(), FixtureError> {
    ssh_privacy::save(&[b"closure-control-private".to_vec()])?;
    for (case, _) in OUTPUT_CONTROLS {
        assert_eq!(
            ssh_harness::run_isolated_with_output_privacy(case),
            Err(ssh_harness::IsolationFailure::OutputPrivacy)
        );
    }
    ssh_harness::raw_output_regression()?;
    ssh_harness::capture_limit_regression()
}
