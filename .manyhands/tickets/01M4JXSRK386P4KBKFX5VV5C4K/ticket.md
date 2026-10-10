---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4JXSRK386P4KBKFX5VV5C4K"
title: "Split the local database so settings survive index and journal failures"
type: "techdebt"
status: "open"
project: "manyhands"
team: "core"
---

All application-local state lives in one SQLite file, `manyhands.sqlite3`.
When that file is structurally corrupt it is renamed aside and replaced with an
empty one. That loses every repository registration, SSH key registration, host
pin, pending operation and all conflict evidence, although only the discovery
index can be rebuilt from Git and Markdown.

Split it into three files so that the settings a user entered survive a failure
of the index or the journal.

Requested by the product owner on 2026-10-10. Expected after the MVP. Not
planned or authorized; this ticket records the research so planning can start
from it.

## Evidence

Read from the source at main `f87ce81` on 2026-10-10. Nothing was run and no
spike was written.

- One file with 29 tables. The persistence RFC requires that single file
  (`docs/RFC/repository-index-persistence-and-refresh.md`, "Local Database").
- `replace_corrupt_registry` in `src/repository.rs` renames the whole file to
  `manyhands.sqlite3.corrupt-<timestamp>` and a fresh one is migrated in its
  place. A rebuild then restores the index for the one root it was given.
- Two marker files exist only to remember what that replacement lost:
  `ssh-host-trust-reapproval-required` (`src/repository/transport/trust.rs`)
  and `remote-history-recovery-required` (`src/repository/recovery.rs`).
- Index schema changes are migrated column by column
  (`ITEM_READ_COLUMNS`, `ITEM_READ_OBJECTS` in `src/repository/discovery.rs`),
  although the index could be dropped and rebuilt.

## Proposed direction

Owner direction, 2026-10-10: do steps 1 and 2. The purpose is mostly to
preserve settings. Some loss of atomicity is acceptable for operations that
touch more than one file.

| File | Tables | Rebuildable |
| --- | --- | --- |
| `index.sqlite3` | `contexts`, `discovered_items`, `discovered_comments`, `problems`, `configuration_observations`, `item_edges`, `item_problems` | Yes |
| `settings.sqlite3` | `repositories`, `shared_ssh_keys`, `owned_generated_keys`, `key_material_operations`, `ssh_host_pins`, polling policy | No |
| Journal (the remaining file) | `operation_records`, `operation_record_contexts`, `remote_operation_records` and the remote integration, resolution, publication and identity-confirmation tables; remote observations | No |

1. **Move the index into its own file.** Index corruption or an index schema
   change becomes "delete and rebuild" with nothing else lost. Sized at about
   half of Wave 02 Cycle 06.
2. **Move settings into their own file.** A journal failure no longer costs
   registrations, keys or host pins. Sized at about a quarter of Cycle 06. This
   step carries the most uncertainty, because the remote state module
   (`src/repository/remote/state.rs`) was surveyed, not read in full.

Kept out:

- Remote observations (`remote_observation_batches`, `remote_ref_observations`,
  `remote_context_states`) stay with the journal. A conflict window references
  the observation batch it was built on, and observations are published in the
  same transaction that acknowledges the operation. Moving them was sized at
  another half of Cycle 06 for little recovery gain.
- `key_material_operations` stays beside the key tables, so key generation and
  deletion remain one transaction.
- The journal is not made recoverable. The split contains the damage of a
  corrupt journal; the evidence in it is still lost.

## What the split has to solve

- **No atomic commit across files.** In WAL mode SQLite does not commit
  attached databases atomically across a crash. Each operation that spans
  files needs a fixed write order and a reconcile rule. The existing rule that
  Git and Markdown win and a pending index write is retried covers the index.
- **Transactions that span areas today.** Rebuild and refresh write the
  registration, the journal row and the index in one transaction
  (`src/repository.rs`, the rebuild and refresh persistence). The repository
  columns that record observation (`accessibility`, `config_blob_oid`,
  `refresh_required`, `refreshed_at`) belong with the index.
- **`repositories.id` is a foreign key throughout.** Eight tables cascade from
  it and `operation_records` references it. Across files this becomes a stable
  key held in each file, a multi-step journaled registration removal, and a
  sweep for orphaned rows.
- **Reads lose a single snapshot.** The read boundary uses one read-only
  connection with `ATTACH` disabled (`src/repository/read/mod.rs`). The status
  read touches index and journal tables and would accept slight skew or read
  twice. Item reads resolve the root through `repositories`.
- **Availability and locking.** `IndexAvailability`, the corruption check and
  the application-data recovery guard are per file today and become per area.
  The host-trust marker can go once settings are separate; the remote-history
  marker stays for the journal.
- **Tests.** About 290 direct references to the file across about 30 test
  files, and about 50 production open sites.

## Open points for planning

- An RFC amendment is needed first; the single-file rule is a MUST.
- Upgrade path. The sizing assumes none, as there are no installs yet. If this
  lands after a first release, add a one-time splitter and its failure
  handling, sized at about another quarter of Cycle 06.
- Timing. The change touches every open site and most test files, so it
  conflicts with any Cycle running beside it. A Wave boundary is the cheapest
  slot.
- Whether the index is rebuilt automatically after loss, now that nothing else
  is lost with it, or still only on an explicit rebuild.
- Whether settings get a backup copy on change, since the file is small and
  rarely written.
- Which stable repository key to use across files: the canonical root path or
  a new identifier.
- Whether polling backoff and latest result stay with polling policy in
  settings or move to the journal.
- No identity table was found; commit identity appears to come from Git
  configuration. Confirm whether "identities" in settings means anything
  beyond SSH keys and host trust.
