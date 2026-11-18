---
id: 01M6X9B9ESX7BTGC7VR441DDN7
item_id: 01M66HFV44YXNQ7ANJVA16TQ1H
created_at: 2026-11-08T09:55:32.607186727Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
**Review-ready, 2026-11-08.** Branch head `b32e56a1` merges cleanly with
main `d70142b6`.

All 14 plan tasks are done. Each was implemented by Sonnet 5.5 and approved
by an independent reviewer, and a final whole-branch review answered READY,
with every acceptance criterion met. Details are in
`docs/plans/2026-11-07-remote-history-repair-execution.md`.

**Gates.** Every command ran through Devenv with `--locked`:
- check, fmt and clippy `-D warnings` clean;
- the fast tier with `--no-fail-fast`: 5,503 passed, plus 1 marker-list
  expectation, fixed and rerun (`recovery_foundation_gate` 52/52);
- SSH tier, once each:
  - `ssh_transport` 123, `remote_observation` 15, `remote_polling` 84;
  - `remote_synchronization` 69, `remote_merge_recovery` 29;
  - `cli_remote` 162, which includes the new journey
    `remote_history_repair_journey`;
  - `document_promotion` 79, `ticket_closure` 78, `mutation_remote` 703;
- from round 1, unchanged since: `local_sync` 160 and
  `comment_publication` 41.

**Round 1 (at `22f72aab`) found 18 failures, now fixed:**
- three code bugs:
  - a late endpoint edit during observation didn't forget;
  - a forgetting fence broke R1 retirement;
  - the conformance homes were missing the new verbs;
- one should-fix from the final review: ghost unknown rows;
- stale test expectations.

**Owner notes:**
- **The schema change is one-way.** This branch rebuilds three registry
  tables. Run the owner check (`remote history` on the real registry, then
  `key select` and a new ticket's `item sync`) after the merge and a rebuild
  of main, or on a copy of the registry.
- **Known limits:**
  - a replayed final sync shows `repo.inspect`;
  - release is desktop-only (sheet: vh-ez4y9);
  - `busy` doesn't name its holder (vh-vsfbp).

Closure is by `ticket close` after review approval.
