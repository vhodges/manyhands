//! The column-scoped registry row scanner, moved here from
//! `tests/document_promotion.rs` `scan_rows` (Cycle 10 named edit 4).
//!
//! An exempt value is accepted only in exactly its own discovery column of
//! `discovered_items`, and only when the cell holds exactly that value; it is
//! never a blanket permission for other tables or other discovery columns.
//!
//! One workflow column has an exemption of its own: `author`, of the two
//! operation journals `operation_records` and `remote_operation_records`.
//! The design records there the configured commit identity of the clone an
//! operation ran in, `Name <email>`, as the operation's author. A cell of
//! that column that holds exactly one of `RowPolicy::authors` is accepted
//! against the probes that value contains (the identity and its name and
//! email), and scanned against every other probe. Any other value of the
//! column, and an author value in any other column or table, is scanned as
//! every cell is.
#![allow(dead_code)]
use super::ssh_privacy;
use super::ssh_remote::{FixtureError, fixed};

/// Which tables a scan covers and which accepted discovery values it allows.
pub struct RowPolicy<'a> {
    /// Workflow tables besides every `remote_%` table.
    pub workflow: &'a [&'a str],
    /// A table that must exist, proving the scan reached the real schema.
    pub required: &'a str,
    /// `(discovered_items column, exact accepted value)`.
    pub exempt: &'a [(&'a str, &'a [u8])],
    /// The exact values the `author` column of the operation journals may
    /// hold: each clone's configured commit identity.
    pub authors: &'a [&'a [u8]],
}

/// The operation journals whose `author` column `RowPolicy::authors`
/// covers.
pub const AUTHORED_TABLES: [&str; 2] = ["operation_records", "remote_operation_records"];

/// The column of an operation journal that names its operation's author.
pub const AUTHOR_COLUMN: &str = "author";

/// The accepted author value `bytes` is, of `authors`, when `table`'s
/// column `column` may hold one; see `RowPolicy::authors`.
pub fn accepted_author<'a>(
    authors: &[&'a [u8]],
    table: &str,
    column: &str,
    bytes: &[u8],
) -> Option<&'a [u8]> {
    (AUTHORED_TABLES.contains(&table) && column == AUTHOR_COLUMN)
        .then(|| authors.iter().copied().find(|author| *author == bytes))
        .flatten()
}

/// Whether `value` holds `probe`.
pub fn contains(value: &[u8], probe: &[u8]) -> bool {
    probe.is_empty() || value.windows(probe.len()).any(|window| window == probe)
}

/// Workflow rows only: no exemption applies.
pub fn scan_workflow_rows(
    db: &rusqlite::Connection,
    probes: &[Vec<u8>],
    policy: &RowPolicy<'_>,
) -> Result<(), FixtureError> {
    scan_rows(db, probes, false, policy)
}

/// With `include_cache`, every table; otherwise the workflow tables only.
pub fn scan_rows(
    db: &rusqlite::Connection,
    probes: &[Vec<u8>],
    include_cache: bool,
    policy: &RowPolicy<'_>,
) -> Result<(), FixtureError> {
    ssh_privacy::clean(b"", probes)?;
    let sql = if include_cache {
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name"
            .to_owned()
    } else {
        let names = policy
            .workflow
            .iter()
            .map(|name| format!("'{name}'"))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "SELECT name FROM sqlite_master WHERE type='table' AND (name LIKE 'remote_%' OR name IN ({names})) ORDER BY name"
        )
    };
    let mut names = fixed(db.prepare(&sql))?;
    let tables = fixed(
        fixed(names.query_map([], |r| r.get::<_, String>(0)))?.collect::<Result<Vec<_>, _>>(),
    )?;
    assert!(tables.iter().any(|n| n == policy.required));
    for table in tables {
        let mut q = fixed(db.prepare(&format!("SELECT * FROM {table}")))?;
        let cols = q.column_count();
        let exempt_columns: Vec<(usize, &[u8])> = if include_cache && table == "discovered_items" {
            policy
                .exempt
                .iter()
                .filter_map(|(column, value)| {
                    (0..cols)
                        .find(|col| q.column_name(*col).ok() == Some(*column))
                        .map(|col| (col, *value))
                })
                .collect()
        } else {
            Vec::new()
        };
        let columns: Vec<String> = (0..cols)
            .map(|col| q.column_name(col).map(str::to_owned))
            .collect::<Result<_, _>>()
            .map_err(|_| FixtureError)?;
        let mut rows = fixed(q.query([]))?;
        while let Some(row) = fixed(rows.next())? {
            for (col, column) in columns.iter().enumerate() {
                if let rusqlite::types::ValueRef::Text(bytes)
                | rusqlite::types::ValueRef::Blob(bytes) = fixed(row.get_ref(col))?
                {
                    let accepted = exempt_columns
                        .iter()
                        .find(|(column, value)| *column == col && bytes == *value)
                        .map(|(_, value)| *value);
                    if let Some(value) = accepted {
                        for probe in probes.iter().filter(|p| p.as_slice() != value) {
                            ssh_privacy::clean(bytes, std::slice::from_ref(probe))?;
                        }
                    } else if let Some(author) =
                        accepted_author(policy.authors, &table, column, bytes)
                    {
                        for probe in probes.iter().filter(|p| !contains(author, p)) {
                            ssh_privacy::clean(bytes, std::slice::from_ref(probe))?;
                        }
                    } else {
                        ssh_privacy::clean(bytes, probes)?;
                    }
                }
            }
        }
    }
    Ok(())
}
