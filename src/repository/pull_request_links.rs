//! The pull request link this clone gave an item: where to open the pull
//! request of a ticket's branch. The registry keeps one row per item, which
//! a read presents and a retirement deletes.
use rusqlite::{Connection, OptionalExtension};

use crate::{canonical::ForgeKind, forge::PullRequestLink};

/// The link the registry holds for the item `item_id` of `kind` (`ticket`
/// or `document`) in the registration `repository_id`. A stored forge this
/// build does not know reads as none, as the configuration's does.
pub(super) fn link_of(
    connection: &Connection,
    repository_id: i64,
    kind: &str,
    item_id: &str,
) -> rusqlite::Result<Option<PullRequestLink>> {
    connection
        .query_row(
            "SELECT source, target, forge, url FROM pull_request_links
              WHERE repository_id = ?1 AND kind = ?2 AND item_ulid = ?3",
            rusqlite::params![repository_id, kind, item_id],
            |row| {
                let forge: Option<String> = row.get(2)?;
                Ok(PullRequestLink {
                    source: row.get(0)?,
                    target: row.get(1)?,
                    forge: forge.and_then(|forge| ForgeKind::parse(&forge)),
                    url: row.get(3)?,
                })
            },
        )
        .optional()
}

/// Deletes the link of an item, in the caller's transaction. An item with
/// no link has nothing to delete.
pub(super) fn delete_link(
    connection: &Connection,
    repository_id: i64,
    kind: &str,
    item_id: &str,
) -> rusqlite::Result<()> {
    connection.execute(
        "DELETE FROM pull_request_links
          WHERE repository_id = ?1 AND kind = ?2 AND item_ulid = ?3",
        rusqlite::params![repository_id, kind, item_id],
    )?;
    Ok(())
}

/// Writes the link row of an item as a build that gives one would, for the
/// tests that need a row before the lifecycle writes them.
#[cfg(test)]
pub(super) fn insert_link_for_test(
    connection: &Connection,
    repository_id: i64,
    kind: &str,
    item_id: &str,
) {
    connection
        .execute(
            "INSERT INTO pull_request_links
                 (repository_id, kind, item_ulid, source, target, forge, url, given_at)
             VALUES (?1, ?2, ?3, ?4, 'main', 'github', NULL, 1)",
            rusqlite::params![
                repository_id,
                kind,
                item_id,
                format!("manyhands/{kind}/{item_id}")
            ],
        )
        .unwrap();
}

/// The `(kind, item)` of every link row, in order.
#[cfg(test)]
pub(super) fn linked_items_for_test(connection: &Connection) -> Vec<(String, String)> {
    connection
        .prepare("SELECT kind, item_ulid FROM pull_request_links ORDER BY kind, item_ulid")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}
