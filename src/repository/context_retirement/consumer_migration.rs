//! Whole known incoming-FK closure replacement. No live parent is renamed or
//! dropped under a referencing table. Temporary copies retain original rowids;
//! all schema, evidence, marker and rollback changes share one immediate TX.
use super::*;
use std::collections::BTreeSet;

pub(super) fn consumer_marked(db: &Connection) -> Result<bool, RepositoryError> {
    db.query_row(
        "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name=?1)",
        [CONSUMERS_MARKER],
        |r| r.get(0),
    )
    .map_err(|_| recovery_required())
}

pub(super) fn validate_consumer_closure(
    db: &Connection,
    definitions: &[(&str, &str, String)],
) -> Result<(), RepositoryError> {
    validate_closure(db, definitions, false)
}
fn validate_closure(
    db: &Connection,
    definitions: &[(&str, &str, String)],
    whole_rebuild: bool,
) -> Result<(), RepositoryError> {
    let tables: BTreeSet<_> = definitions
        .iter()
        .filter(|(k, n, _)| {
            *k == "table"
                && (whole_rebuild
                    || n.starts_with("poll_retirement_")
                    || matches!(
                        *n,
                        "retirement_observation_receipts"
                            | "retirement_observation_pins"
                            | "retirement_observation_pin_releases"
                            | "context_retirement_effect_intents"
                            | "context_retirement_absence_receipts"
                            | "context_retirement_worktree_resumptions"
                    ))
        })
        .map(|(_, n, _)| *n)
        .collect();
    let mut q = db
        .prepare("SELECT type,name,tbl_name,sql FROM sqlite_master WHERE sql IS NOT NULL")
        .map_err(|_| recovery_required())?;
    let rows = q
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })
        .map_err(|_| recovery_required())?;
    for row in rows {
        let (kind, name, table, sql) = row.map_err(|_| recovery_required())?;
        let known = definitions
            .iter()
            .any(|(k, n, _)| *k == kind && name.eq_ignore_ascii_case(n));
        if !known && name.to_ascii_lowercase().starts_with("poll_retirement_") {
            return Err(recovery_required());
        }
        if !known
            && ((matches!(kind.as_str(), "index" | "trigger")
                && tables.iter().any(|t| table.eq_ignore_ascii_case(t)))
                || (matches!(kind.as_str(), "trigger" | "view")
                    && tables.iter().any(|t| sql.to_ascii_lowercase().contains(t))))
        {
            return Err(recovery_required());
        }
        if kind == "table" && !known {
            let mut fk = db
                .prepare("SELECT \"table\" FROM pragma_foreign_key_list(?1)")
                .map_err(|_| recovery_required())?;
            for parent in fk
                .query_map([name], |r| r.get::<_, String>(0))
                .map_err(|_| recovery_required())?
            {
                if tables
                    .iter()
                    .any(|t| parent.as_ref().is_ok_and(|p| p.eq_ignore_ascii_case(t)))
                {
                    return Err(recovery_required());
                }
                parent.map_err(|_| recovery_required())?;
            }
        }
    }
    Ok(())
}

pub(super) fn replace_consumer_closure(
    tx: &rusqlite::Transaction<'_>,
    old: &[(&str, &str, String)],
    new: &[(&str, &str, String)],
) -> Result<(), RepositoryError> {
    validate_closure(tx, old, true)?;
    let mut remaining: BTreeSet<&str> = old
        .iter()
        .filter(|(k, _, _)| *k == "table")
        .map(|(_, n, _)| *n)
        .collect();
    let mut order = Vec::new();
    while !remaining.is_empty() {
        let mut next = None;
        for &table in &remaining {
            let incoming: bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master m JOIN pragma_foreign_key_list(m.name) fk WHERE m.type='table' AND fk.\"table\"=?1 COLLATE NOCASE AND m.name!=?1 AND m.name IN (SELECT name FROM sqlite_master WHERE type='table'))",[table],|r|r.get(0)).map_err(|_|recovery_required())?;
            // Resolve against the remaining graph, not already ordered tables.
            let children=tx.prepare("SELECT m.name FROM sqlite_master m JOIN pragma_foreign_key_list(m.name) fk WHERE m.type='table' AND fk.\"table\"=?1 COLLATE NOCASE AND m.name!=?1").and_then(|mut q|q.query_map([table], |r|r.get::<_,String>(0))?.collect::<Result<Vec<_>,_>>()).map_err(|_|recovery_required())?;
            if !incoming || !children.iter().any(|c| remaining.contains(c.as_str())) {
                next = Some(table);
                break;
            }
        }
        let table = next.ok_or_else(recovery_required)?;
        remaining.remove(table);
        order.push(table);
    }
    for table in &order {
        tx.execute_batch(&format!("CREATE TEMP TABLE consumer_copy_{table} AS SELECT rowid AS saved_rowid,* FROM main.{table}" )).map_err(|_|recovery_required())?;
    }
    #[cfg(test)]
    migration_checkpoint(20)?;
    for (kind, name, _) in old.iter().filter(|(k, _, _)| *k != "table") {
        tx.execute_batch(&format!("DROP {kind} {name}"))
            .map_err(|_| recovery_required())?;
    }
    for table in &order {
        tx.execute_batch(&format!("DROP TABLE {table}"))
            .map_err(|_| recovery_required())?;
    }
    #[cfg(test)]
    migration_checkpoint(21)?;
    for (_, _, sql) in new.iter().filter(|(k, _, _)| *k == "table") {
        tx.execute_batch(sql).map_err(|_| recovery_required())?;
    }
    for table in order.iter().rev() {
        tx.execute_batch(&format!("INSERT INTO main.{table}(rowid,{}) SELECT saved_rowid,{} FROM temp.consumer_copy_{table}",columns(tx,table)?,columns(tx,table)?)).map_err(|_|recovery_required())?;
        let equal:bool=tx.query_row(&format!("SELECT NOT EXISTS(SELECT rowid,* FROM main.{table} EXCEPT SELECT * FROM temp.consumer_copy_{table}) AND NOT EXISTS(SELECT * FROM temp.consumer_copy_{table} EXCEPT SELECT rowid,* FROM main.{table})"),[],|r|r.get(0)).map_err(|_|recovery_required())?;
        if !equal {
            return Err(recovery_required());
        }
    }
    #[cfg(test)]
    migration_checkpoint(22)?;
    for (_, _, sql) in new.iter().filter(|(k, _, _)| *k != "table") {
        tx.execute_batch(sql).map_err(|_| recovery_required())?;
    }
    audit_objects(tx, new)?;
    for table in order {
        tx.execute_batch(&format!("DROP TABLE temp.consumer_copy_{table}"))
            .map_err(|_| recovery_required())?;
    }
    #[cfg(test)]
    migration_checkpoint(23)?;
    Ok(())
}
fn columns(db: &Connection, table: &str) -> Result<String, RepositoryError> {
    db.prepare("SELECT name FROM pragma_table_info(?1) ORDER BY cid")
        .and_then(|mut q| {
            q.query_map([table], |r| r.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map(|c| c.join(","))
        .map_err(|_| recovery_required())
}
